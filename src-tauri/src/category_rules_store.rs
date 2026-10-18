use super::*;
use crate::category_rules::{self as rules, Input, Version};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub label: String,
    pub archived: bool,
    pub revision: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub current: Version,
    pub history: Vec<Version>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub decision_id: String,
    pub record_id: String,
    pub rule: Version,
    pub category: String,
    pub status: String,
    pub reviewed_at: String,
    pub source_snapshot: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub client_id: String,
    pub fingerprint: String,
    pub accounts: Vec<Account>,
    pub rules: Vec<Rule>,
    pub application_count: usize,
    pub applications: Vec<Application>,
}
fn json(v: &impl Serialize) -> Result<String, String> {
    serde_json::to_string(v).map_err(|_| STORAGE_ERROR.into())
}
pub(super) fn read(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<Directory, String> {
    scoped_client(c, w, client)?;
    let accounts = statement_accounts_store::read(c, w, client)?
        .accounts
        .into_iter()
        .map(|a| Account {
            id: a.id,
            label: a.label,
            archived: a.archived,
            revision: a.revision,
        })
        .collect::<Vec<_>>();
    let mut groups: BTreeMap<String, Vec<Version>> = BTreeMap::new();
    let mut q=c.prepare("SELECT rule_id,revision,account_id,payload_json,digest FROM category_rule_versions WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rule_id,revision LIMIT 5001").map_err(storage_error)?;
    let rows = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(storage_error)?;
    let mut count = 0;
    for row in rows {
        let (id, rev, account, payload, digest) = row.map_err(storage_error)?;
        count += 1;
        let v: Version = serde_json::from_str(&payload).map_err(|_| STORAGE_ERROR)?;
        let history = groups.entry(id.clone()).or_default();
        if count > 5000
            || v.id != id
            || Uuid::parse_str(&id).is_err()
            || v.revision != rev
            || rev != history.len() as i64 + 1
            || rev > 100
            || v.data.account_id != account
            || !accounts.iter().any(|a| a.id == account)
            || rules::canonical(v.data.clone())? != v.data
            || rules::text(&v.reason, "Rule reason", 500)? != v.reason
            || rules::text(&v.counselor_name, "Counselor", 120)? != v.counselor_name
            || chrono::DateTime::parse_from_rfc3339(&v.updated_at).is_err()
            || format!("{:x}", Sha256::digest(payload.as_bytes())) != digest
            || json(&v)? != payload
        {
            return Err(STORAGE_ERROR.into());
        }
        history.push(v);
    }
    if groups.len() > 50 {
        return Err(STORAGE_ERROR.into());
    }
    let rules = groups
        .into_values()
        .map(|history| Rule {
            current: history.last().unwrap().clone(),
            history,
        })
        .collect::<Vec<_>>();
    let mut keys = std::collections::BTreeSet::new();
    for r in &rules {
        if !r.current.data.archived && !keys.insert(rules::key(&r.current.data)?) {
            return Err(STORAGE_ERROR.into());
        }
    }
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(json(&(
            "local-category-rules-v1",
            client,
            &accounts,
            &rules
        ))?)
    );
    let mut q=c.prepare("SELECT a.decision_id,a.record_id,a.rule_id,a.rule_revision,a.source_snapshot,h.from_fields_json,h.to_fields_json,h.from_status,h.to_status,h.reviewed_at,d.account_id FROM category_rule_applications a JOIN transaction_review_history h ON h.id=a.decision_id AND h.record_id=a.record_id AND h.client_id=a.client_id AND h.organization_id=a.organization_id AND h.counselor_id=a.counselor_id JOIN import_records r ON r.id=a.record_id AND r.client_id=a.client_id AND r.organization_id=a.organization_id AND r.counselor_id=a.counselor_id JOIN import_documents d ON d.id=r.document_id AND d.client_id=r.client_id AND d.organization_id=r.organization_id AND d.counselor_id=r.counselor_id WHERE a.client_id=?1 AND a.organization_id=?2 AND a.counselor_id=?3 AND h.from_linked_id IS NULL AND h.to_status IN ('Pending','Needs Clarification','Approved','Rejected') ORDER BY h.rowid DESC LIMIT 5001").map_err(storage_error)?;
    let rows = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, String>(10)?,
            ))
        })
        .map_err(storage_error)?;
    let mut applications = Vec::new();
    let mut application_count = 0;
    for row in rows {
        let (
            decision_id,
            record_id,
            id,
            revision,
            snapshot,
            before,
            after,
            from_status,
            status,
            reviewed_at,
            account,
        ) = row.map_err(storage_error)?;
        application_count += 1;
        let rule = rules
            .iter()
            .flat_map(|r| &r.history)
            .find(|v| v.id == id && v.revision == revision)
            .ok_or(STORAGE_ERROR)?;
        let before: crate::imports::Fields =
            serde_json::from_str(&before).map_err(|_| STORAGE_ERROR)?;
        let after: crate::imports::Fields =
            serde_json::from_str(&after).map_err(|_| STORAGE_ERROR)?;
        let mut expected = before.clone();
        expected.category = rule.data.category.clone();
        if before.date.is_none()
            || before.description.is_empty()
            || before
                .amount
                .as_ref()
                .and_then(|a| crate::imports::signed_money(a).ok())
                .is_none_or(|a| a >= 0)
            || chrono::DateTime::parse_from_rfc3339(&reviewed_at).is_err()
            || Uuid::parse_str(&decision_id).is_err()
            || Uuid::parse_str(&record_id).is_err()
            || application_count > 5000
            || !matches!(from_status.as_str(), "Pending" | "Needs Clarification")
            || !before.category.is_empty()
            || !rules::matches(&rule.data, &account, &before)?
            || after != expected
            || crate::imports::validate_fields(before.clone())? != before
            || crate::imports::validate_fields(after.clone())? != after
            || snapshot.len() != 64
            || !snapshot
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(STORAGE_ERROR.into());
        }
        if applications.len() < 25 {
            applications.push(Application {
                decision_id,
                record_id,
                rule: rule.clone(),
                category: after.category,
                status,
                reviewed_at,
                source_snapshot: snapshot,
            });
        }
    }
    let stored:i64=c.query_row("SELECT COUNT(*) FROM category_rule_applications WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?;
    if stored != application_count as i64 {
        return Err(STORAGE_ERROR.into());
    }
    Ok(Directory {
        client_id: client.into(),
        fingerprint,
        accounts,
        rules,
        application_count,
        applications,
    })
}
impl Workspace {
    pub fn get_category_rules(&mut self, client: &str) -> Result<Directory, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let d = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(d)
    }
    pub fn save_category_rule(&mut self, client: &str, input: Input) -> Result<Directory, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let d = read(&tx, &s.info, client)?;
        if input.expected_fingerprint != d.fingerprint {
            return Err("Category rules or accounts changed. Reload rules before saving.".into());
        }
        let data = rules::canonical(input.data)?;
        let reason = rules::text(&input.reason, "Rule change reason", 500)?;
        if !d
            .accounts
            .iter()
            .any(|a| a.id == data.account_id && (data.archived || !a.archived))
        {
            return Err("Choose an active bank account before activating a rule.".into());
        }
        let (id, revision) = match input.id {
            Some(id) => {
                let old = d
                    .rules
                    .iter()
                    .find(|r| r.current.id == id)
                    .ok_or("This rule is unavailable for this client.")?;
                if old.current.revision != input.expected_revision || old.current.revision >= 100 {
                    return Err("Rule revision changed or reached its 100-version limit.".into());
                }
                if old.current.data == data {
                    return Err("The rule is unchanged.".into());
                }
                (id, old.current.revision + 1)
            }
            None => {
                if input.expected_revision != 0 || d.rules.len() >= 50 {
                    return Err(
                        "Use a new rule revision and at most 50 retained rules per client.".into(),
                    );
                }
                (Uuid::new_v4().to_string(), 1)
            }
        };
        let key = rules::key(&data)?;
        if !data.archived
            && d.rules.iter().any(|r| {
                r.current.id != id
                    && !r.current.data.archived
                    && rules::key(&r.current.data).ok().as_ref() == Some(&key)
            })
        {
            return Err("An active rule already matches this account, type and exact text. Edit or archive it first.".into());
        }
        let v = Version {
            id,
            revision,
            data,
            reason,
            updated_at: timestamp(),
            counselor_name: s.info.counselor_name.clone(),
        };
        let payload = json(&v)?;
        tx.execute("INSERT INTO category_rule_versions(rule_id,client_id,organization_id,counselor_id,revision,account_id,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![v.id,client,s.info.organization_id,s.info.counselor_id,v.revision,v.data.account_id,payload,format!("{:x}",Sha256::digest(payload.as_bytes()))]).map_err(storage_error)?;
        audit(&tx, &s.info, "category_rule_saved", Some(&v.id))?;
        let result = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}
pub(super) fn capture(
    c: &Transaction,
    w: &WorkspaceInfo,
    client: &str,
    decision: &str,
    record: &str,
    selection: &rules::Selection,
) -> Result<(), String> {
    let count:i64=c.query_row("SELECT COUNT(*) FROM category_rule_applications WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?;
    if count >= 5000 {
        return Err("This client reached the 5,000 rule-application limit.".into());
    }
    c.execute("INSERT INTO category_rule_applications(decision_id,record_id,client_id,organization_id,counselor_id,rule_id,rule_revision,source_snapshot) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![decision,record,client,w.organization_id,w.counselor_id,selection.rule_id,selection.revision,selection.expected_snapshot]).map_err(storage_error)?;
    audit(c, w, "category_rule_applied", Some(decision))?;
    Ok(())
}

pub(super) fn active(d: &Directory) -> Vec<Version> {
    d.rules
        .iter()
        .filter(|r| {
            !r.current.data.archived
                && d.accounts
                    .iter()
                    .any(|a| a.id == r.current.data.account_id && !a.archived)
        })
        .map(|r| r.current.clone())
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    use crate::{
        category_rules::{Data, Decision, Mode},
        classification::{DraftInput, Query},
        imports::{ReviewInput, Status},
    };
    fn input(d: &Directory, v: Option<&Version>, data: Data) -> Input {
        Input {
            id: v.map(|v| v.id.clone()),
            expected_revision: v.map_or(0, |v| v.revision),
            expected_fingerprint: d.fingerprint.clone(),
            data,
            reason: "Fictional counselor rule decision".into(),
        }
    }
    fn query() -> Query {
        Query {
            text: String::new(),
            offset: 0,
            expected_snapshot: None,
        }
    }
    fn prepared(w: &mut Workspace, client: &str, id: &str) -> crate::classification::Draft {
        let p = w.query_category_suggestions(client, query()).unwrap();
        w.prepare_category_draft(
            client,
            DraftInput {
                expected_revision: p.revision,
                expected_snapshot: p.snapshot,
                record_id: id.into(),
            },
        )
        .unwrap()
    }
    #[test]
    fn rule_crud_conflicts_stale_use_atomic_application_scope_and_recovery() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let s=bank::stage(&mut w,&c.id,root.path(),"fictional-rule.csv","Date,Description,Amount,Category,Type\n2026-09-01,Fictional rule market,-11.01,,expense\n2026-09-02,Fictional rule market,-12.02,,expense\n2026-09-03,Fictional rule market,-13.03,,expense\n",bank::doc("Fictional rule checking"));
        let id = s.records[0].id.clone();
        let account = s.accounts[0].id.clone();
        let initial = serde_json::to_vec(&s).unwrap();
        let d = w.get_category_rules(&c.id).unwrap();
        let data = Data {
            label: "Fictional grocery rule".into(),
            account_id: account.clone(),
            kind: crate::imports::Kind::Expense,
            mode: Mode::Description,
            pattern: "FICTIONAL   RULE MARKET".into(),
            category: "Food".into(),
            archived: false,
        };
        for mut bad in [data.clone(), data.clone(), data.clone(), data.clone()]
            .into_iter()
            .enumerate()
        {
            match bad.0 {
                0 => bad.1.category = "".into(),
                1 => bad.1.kind = crate::imports::Kind::Income,
                2 => bad.1.pattern = "x".repeat(121),
                _ => bad.1.account_id = Uuid::new_v4().to_string(),
            };
            assert!(w.save_category_rule(&c.id, input(&d, None, bad.1)).is_err());
        }
        w.session().unwrap().connection.execute_batch("CREATE TEMP TRIGGER synthetic_rule_audit BEFORE INSERT ON audit_events WHEN NEW.event='category_rule_saved' BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
        assert!(w
            .save_category_rule(&c.id, input(&d, None, data.clone()))
            .is_err());
        assert!(w.get_category_rules(&c.id).unwrap().rules.is_empty());
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER synthetic_rule_audit;")
            .unwrap();
        let d = w
            .save_category_rule(&c.id, input(&d, None, data.clone()))
            .unwrap();
        let original = d.rules[0].current.clone();
        assert_eq!(original.revision, 1);
        assert_eq!(
            serde_json::to_vec(&w.get_client_imports(&c.id).unwrap()).unwrap(),
            initial
        );
        assert!(w
            .save_category_rule(&c.id, input(&d, None, data.clone()))
            .is_err());
        let foreign = w.get_category_rules(&other.id).unwrap();
        assert!(w
            .save_category_rule(&other.id, input(&foreign, None, data.clone()))
            .is_err());
        assert!(w
            .save_category_rule(&other.id, input(&foreign, Some(&original), data.clone()))
            .is_err());
        let draft = prepared(&mut w, &c.id, &id);
        assert_eq!(draft.category, "Food");
        let selection = draft.rule.unwrap();
        let mut changed = data.clone();
        changed.category = "Groceries".into();
        w.save_category_rule(&c.id, input(&d, Some(&original), changed.clone()))
            .unwrap();
        let row = s.records.iter().find(|r| r.id == id).unwrap();
        let mut fields = row.fields.clone();
        fields.category = "Food".into();
        let request = |selection: crate::category_rules::Selection,
                       fields: crate::imports::Fields,
                       revision: i64| Decision {
            selection,
            decision: ReviewInput {
                expected_revision: revision,
                record_id: id.clone(),
                fields,
                status: Status::NeedsClarification,
                reason: "Fictional original source and rule verified".into(),
                acknowledge_warnings: false,
            },
        };
        assert!(w
            .review_rule_category(&c.id, request(selection, fields, s.revision))
            .is_err());
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, s.revision);
        let draft = prepared(&mut w, &c.id, &id);
        let mut fields = row.fields.clone();
        fields.category = draft.category.clone();
        let selection = draft.rule.unwrap();
        let mut forged = fields.clone();
        forged.amount = Some("-99.99".into());
        assert!(w
            .review_rule_category(&c.id, request(selection.clone(), forged, s.revision))
            .is_err());
        w.session().unwrap().connection.execute_batch("CREATE TEMP TRIGGER synthetic_application_audit BEFORE INSERT ON audit_events WHEN NEW.event='category_rule_applied' BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
        assert!(w
            .review_rule_category(
                &c.id,
                request(selection.clone(), fields.clone(), s.revision)
            )
            .is_err());
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, s.revision);
        assert_eq!(w.get_category_rules(&c.id).unwrap().application_count, 0);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER synthetic_application_audit;")
            .unwrap();
        let saved = w
            .review_rule_category(&c.id, request(selection, fields, s.revision))
            .unwrap();
        assert_eq!(saved.summary.spending, "0.00");
        assert_eq!(
            saved.records.iter().find(|r| r.id == id).unwrap().status,
            Status::NeedsClarification
        );
        let mut d = w.get_category_rules(&c.id).unwrap();
        assert_eq!(d.application_count, 1);
        assert_eq!(d.applications[0].rule.revision, 2);
        assert_eq!(d.applications[0].category, "Groceries");
        let application = json(&d.applications[0]).unwrap();
        let mut inactive = changed.clone();
        inactive.archived = true;
        let v = d.rules[0].current.clone();
        d = w
            .save_category_rule(&c.id, input(&d, Some(&v), inactive))
            .unwrap();
        assert!(w
            .query_category_suggestions(&c.id, query())
            .unwrap()
            .candidates
            .iter()
            .all(|c| c.category.is_none()));
        let v = d.rules[0].current.clone();
        d = w
            .save_category_rule(&c.id, input(&d, Some(&v), changed.clone()))
            .unwrap();
        assert_eq!(d.rules[0].history.len(), 4);
        assert_eq!(json(&d.applications[0]).unwrap(), application);
        let source = w.get_client_imports(&c.id).unwrap();
        let proof = source.records.iter().find(|r| r.id != id).unwrap();
        let mut conflict = proof.fields.clone();
        conflict.category = "Leisure".into();
        w.review_transaction(
            &c.id,
            ReviewInput {
                expected_revision: source.revision,
                record_id: proof.id.clone(),
                fields: conflict,
                status: Status::Approved,
                reason: "Fictional conflicting source category verified".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
        let p = w.query_category_suggestions(&c.id, query()).unwrap();
        assert!(p
            .candidates
            .iter()
            .all(|c| c.category.is_none() && c.reason.contains("conflicts")));
        let stale = d.fingerprint.clone();
        let mut fixed = changed.clone();
        fixed.category = "Leisure".into();
        let v = d.rules[0].current.clone();
        d = w
            .save_category_rule(&c.id, input(&d, Some(&v), fixed))
            .unwrap();
        assert_ne!(d.fingerprint, stale);
        assert!(w
            .save_category_rule(
                &c.id,
                Input {
                    expected_fingerprint: stale,
                    ..input(&d, Some(&v), changed.clone())
                }
            )
            .is_err());
        assert!(!w
            .get_category_rules(&other.id)
            .unwrap()
            .rules
            .iter()
            .any(|r| r.current.id == original.id));
        assert!(w
            .session()
            .unwrap()
            .connection
            .execute(
                "UPDATE category_rule_versions SET digest='forged' WHERE rule_id=?1",
                [&original.id]
            )
            .is_err());
        assert!(w
            .session()
            .unwrap()
            .connection
            .execute("DELETE FROM category_rule_applications", [])
            .is_err());
        let backups = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        let history = json(&w.get_category_rules(&c.id).unwrap()).unwrap();
        w.lock_workspace();
        assert!(w.get_category_rules(&c.id).is_err());
        assert!(w.save_category_rule(&c.id, input(&d, None, data)).is_err());
        drop(w);
        let mut w = Workspace::new(root.path().into());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            json(&w.get_category_rules(&c.id).unwrap()).unwrap(),
            history
        );
        w.lock_workspace();
        let restored = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(restored.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            json(&w.get_category_rules(&c.id).unwrap()).unwrap(),
            history
        );
        // Corrupt retained history fails recovery before it can replace active files.
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER category_rule_versions_no_update;")
            .unwrap();
        w.session()
            .unwrap()
            .connection
            .execute(
                "UPDATE category_rule_versions SET digest=?1 WHERE revision=1",
                ["0".repeat(64)],
            )
            .unwrap();
        assert!(w.get_category_rules(&c.id).is_err());
        let corrupt = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(corrupt.path, PASSWORD.into()).is_err());
    }
    #[test]
    fn schema_25_migration_is_atomic_and_preserves_original_imports() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let s = bank::stage(
            &mut w,
            &c.id,
            root.path(),
            "legacy-rules.csv",
            "Date,Description,Amount,Category,Type\n2026-09-03,Fictional legacy,-13.03,,expense\n",
            bank::doc("Legacy rule checking"),
        );
        let original = serde_json::to_vec(&s).unwrap();
        let conn = &mut w.session().unwrap().connection;
        conn.execute_batch("DROP TABLE IF EXISTS transaction_inspection_details; DROP TABLE IF EXISTS household_events; DROP TABLE IF EXISTS workspace_security_history; DROP TABLE IF EXISTS note_amendments; DROP INDEX IF EXISTS note_amendment_scope; DROP TABLE category_rule_applications; DROP TABLE category_rule_versions; DROP INDEX review_history_rule_scope; PRAGMA user_version=25; CREATE TABLE audit_events_v26(collision TEXT);").unwrap();
        assert!(migrate(conn, false).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            25
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='category_rule_versions'",
                [],
                |r| r.get::<_, i32>(0)
            )
            .unwrap(),
            0
        );
        conn.execute_batch("DROP TABLE audit_events_v26;").unwrap();
        migrate(conn, false).unwrap();
        assert_eq!(
            w.session()
                .unwrap()
                .connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
        assert_eq!(
            serde_json::to_vec(&w.get_client_imports(&c.id).unwrap()).unwrap(),
            original
        );
        assert!(w.get_category_rules(&c.id).unwrap().rules.is_empty());
    }
    #[test]
    fn archived_accounts_disable_rules_and_invalidate_prepared_rule_context() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let s=bank::stage(&mut w,&c.id,root.path(),"archive-rule.csv","Date,Description,Amount,Category,Type\n2026-09-03,Fictional archived merchant,-13.03,,expense\n",bank::doc("Fictional archive checking"));
        let account = &s.accounts[0];
        let d = w.get_category_rules(&c.id).unwrap();
        let data = Data {
            label: "Archive fixture rule".into(),
            account_id: account.id.clone(),
            kind: crate::imports::Kind::Expense,
            mode: Mode::Description,
            pattern: "Fictional archived merchant".into(),
            category: "Food".into(),
            archived: false,
        };
        let d = w
            .save_category_rule(&c.id, input(&d, None, data.clone()))
            .unwrap();
        let v = d.rules[0].current.clone();
        let draft = prepared(&mut w, &c.id, &s.records[0].id);
        let accounts = w.get_statement_accounts(&c.id).unwrap();
        let a = &accounts.accounts[0];
        let edit = |a: &statement_accounts_store::Account, revision: i64, archived: bool| {
            statement_accounts_store::Input {
                id: Some(a.id.clone()),
                expected_revision: a.revision,
                expected_import_revision: revision,
                label: a.label.clone(),
                kind: a.kind.clone(),
                archived,
            }
        };
        w.save_statement_account(&c.id, edit(a, accounts.import_revision, true))
            .unwrap();
        let d = w.get_category_rules(&c.id).unwrap();
        assert!(w
            .query_category_suggestions(&c.id, query())
            .unwrap()
            .candidates[0]
            .category
            .is_none());
        let selection = draft.rule.unwrap();
        assert!(w
            .prepare_category_draft(
                &c.id,
                DraftInput {
                    expected_revision: s.revision,
                    expected_snapshot: selection.expected_snapshot,
                    record_id: s.records[0].id.clone()
                }
            )
            .is_err());
        let mut changed = data.clone();
        changed.label = "Cannot activate against an archived account".into();
        assert!(w
            .save_category_rule(&c.id, input(&d, Some(&v), changed))
            .is_err());
        let mut archived = data.clone();
        archived.archived = true;
        let d = w
            .save_category_rule(&c.id, input(&d, Some(&v), archived))
            .unwrap();
        let accounts = w.get_statement_accounts(&c.id).unwrap();
        w.save_statement_account(
            &c.id,
            edit(&accounts.accounts[0], accounts.import_revision, false),
        )
        .unwrap();
        let latest = w.get_category_rules(&c.id).unwrap();
        w.save_category_rule(&c.id, input(&latest, Some(&d.rules[0].current), data))
            .unwrap();
        assert_eq!(prepared(&mut w, &c.id, &s.records[0].id).category, "Food");
        assert_eq!(w.get_category_rules(&c.id).unwrap().application_count, 0);
    }
}
