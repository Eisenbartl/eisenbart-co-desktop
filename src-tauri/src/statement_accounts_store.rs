//! Current workflow labels/lifecycle are separate from immutable financial source identities.
use super::*;
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub revision: i64,
    pub label: String,
    pub archived: bool,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub label: String,
    pub original_label: String,
    pub kind: String,
    pub archived: bool,
    pub revision: i64,
    pub source_count: i64,
    pub history: Vec<Version>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub client_id: String,
    pub import_revision: i64,
    pub accounts: Vec<Account>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub id: Option<String>,
    pub expected_revision: i64,
    pub expected_import_revision: i64,
    pub label: String,
    pub kind: String,
    pub archived: bool,
}
pub(super) fn read(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<Directory, String> {
    let mut q=c.prepare("SELECT a.id,a.label,a.kind,(SELECT COUNT(*) FROM import_documents d WHERE d.account_id=a.id AND d.client_id=a.client_id AND d.organization_id=a.organization_id AND d.counselor_id=a.counselor_id) FROM statement_accounts a WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid LIMIT 21").map_err(storage_error)?;
    let rows = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .map_err(storage_error)?;
    let mut accounts = Vec::new();
    for row in rows {
        let (id, original_label, kind, source_count) = row.map_err(storage_error)?;
        if Uuid::parse_str(&id).is_err()
            || required_text(&original_label, "Account label", 120)? != original_label
            || !matches!(kind.as_str(), "checking" | "savings")
        {
            return Err(STORAGE_ERROR.into());
        }
        let mut q=c.prepare("SELECT revision,label,archived,updated_at FROM statement_account_history WHERE account_id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4 ORDER BY revision LIMIT 501").map_err(storage_error)?;
        let versions = q
            .query_map(
                params![id, client, w.organization_id, w.counselor_id],
                |r| {
                    Ok(Version {
                        revision: r.get(0)?,
                        label: r.get(1)?,
                        archived: r.get(2)?,
                        updated_at: r.get(3)?,
                    })
                },
            )
            .map_err(storage_error)?;
        let mut history = Vec::new();
        for v in versions {
            let v = v.map_err(storage_error)?;
            if v.revision != history.len() as i64 + 1
                || v.revision > 500
                || required_text(&v.label, "Account label", 120)? != v.label
                || chrono::DateTime::parse_from_rfc3339(&v.updated_at).is_err()
            {
                return Err(STORAGE_ERROR.into());
            }
            history.push(v);
        }
        let last = history.last();
        accounts.push(Account {
            id,
            label: last.map_or_else(|| original_label.clone(), |v| v.label.clone()),
            original_label,
            kind,
            archived: last.is_some_and(|v| v.archived),
            revision: history.len() as i64,
            source_count,
            history,
        });
    }
    if accounts.len() > 20 {
        return Err(STORAGE_ERROR.into());
    }
    // Reserve both current and original labels, including archived accounts.
    for (i, a) in accounts.iter().enumerate() {
        if accounts.iter().skip(i + 1).any(|b| {
            [a.label.to_lowercase(), a.original_label.to_lowercase()]
                .iter()
                .any(|name| {
                    name == &b.label.to_lowercase() || name == &b.original_label.to_lowercase()
                })
        }) {
            return Err(STORAGE_ERROR.into());
        }
    }
    let import_revision=c.query_row("SELECT revision FROM client_import_state WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).optional().map_err(storage_error)?.unwrap_or(0);
    Ok(Directory {
        client_id: client.into(),
        import_revision,
        accounts,
    })
}
impl Workspace {
    pub fn get_statement_accounts(&mut self, client: &str) -> Result<Directory, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let result = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn save_statement_account(
        &mut self,
        client: &str,
        input: Input,
    ) -> Result<Directory, String> {
        let label = required_text(&input.label, "Account label", 120)?;
        if !matches!(input.kind.as_str(), "checking" | "savings") {
            return Err(
                "Choose checking or savings. Other account types need future document parsers."
                    .into(),
            );
        }
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let current = read(&tx, &s.info, client)?;
        if current
            .accounts
            .iter()
            .filter(|a| Some(&a.id) != input.id.as_ref())
            .any(|a| {
                a.label.to_lowercase() == label.to_lowercase()
                    || a.original_label.to_lowercase() == label.to_lowercase()
            })
        {
            return Err("This account label is already reserved by a current or historical account. Choose a distinct label.".into());
        }
        let (id, revision) = if let Some(id) = input.id {
            let old = current
                .accounts
                .iter()
                .find(|a| a.id == id)
                .ok_or("Choose an account belonging to this client.")?;
            if old.revision != input.expected_revision {
                return Err("The account changed in another window. Your draft is retained; discard and reload before saving.".into());
            }
            if old.kind != input.kind {
                return Err("The original account type cannot be changed. Create a separate account for a different type.".into());
            }
            if old.revision >= 500 {
                return Err("This account reached the current 500-version history limit.".into());
            }
            (id, old.revision + 1)
        } else {
            if input.expected_revision != 0 || input.archived {
                return Err("New accounts must start active at version zero.".into());
            }
            if current.accounts.len() >= 20 {
                return Err(
                    "Use at most 20 statement accounts per client, including archived accounts."
                        .into(),
                );
            }
            imports_store::bump(&tx, &s.info, client, input.expected_import_revision)?;
            let id = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO statement_accounts(id,client_id,organization_id,counselor_id,label,kind) VALUES(?1,?2,?3,?4,?5,?6)",params![id,client,s.info.organization_id,s.info.counselor_id,label,input.kind]).map_err(storage_error)?;
            (id, 1)
        };
        tx.execute("INSERT INTO statement_account_history(account_id,client_id,organization_id,counselor_id,revision,label,archived,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![id,client,s.info.organization_id,s.info.counselor_id,revision,label,input.archived,timestamp()]).map_err(storage_error)?;
        audit(&tx, &s.info, "statement_account_saved", Some(&id))?;
        let result = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::{client_input, configured, PASSWORD};
    fn input(id: Option<&str>, rev: i64, imports: i64, label: &str, archived: bool) -> Input {
        Input {
            id: id.map(str::to_owned),
            expected_revision: rev,
            expected_import_revision: imports,
            label: label.into(),
            kind: "checking".into(),
            archived,
        }
    }
    #[test]
    fn account_lifecycle_scope_revision_atomic_audit_and_encrypted_recovery() {
        let (dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        let b = w.save_client(client_input()).unwrap();
        assert!(w.get_statement_accounts("foreign").is_err());
        assert!(w
            .save_statement_account("foreign", input(None, 0, 0, "Private", false))
            .is_err());
        let first = w
            .save_statement_account(&a.id, input(None, 0, 0, "Fictional checking", false))
            .unwrap();
        let id = first.accounts[0].id.clone();
        assert_eq!(first.import_revision, 1);
        assert!(w
            .save_statement_account(&a.id, input(None, 0, 0, "Other", false))
            .is_err());
        assert!(w
            .save_statement_account(&b.id, input(Some(&id), 1, 0, "Foreign", false))
            .is_err());
        assert!(w.get_statement_accounts(&b.id).unwrap().accounts.is_empty());
        assert!(w
            .save_statement_account(&a.id, input(None, 0, 1, "fictional CHECKING", false))
            .is_err());
        let r = w
            .save_statement_account(&a.id, input(Some(&id), 1, 1, "Renamed checking", true))
            .unwrap();
        assert_eq!(r.import_revision, 1);
        assert!(r.accounts[0].archived);
        assert_eq!(r.accounts[0].original_label, "Fictional checking");
        assert!(w
            .save_statement_account(&a.id, input(Some(&id), 1, 1, "Stale", false))
            .is_err());
        let mut changed_kind = input(Some(&id), 2, 1, "Renamed checking", false);
        changed_kind.kind = "savings".into();
        assert!(w.save_statement_account(&a.id, changed_kind).is_err());
        assert!(w
            .save_statement_account(&a.id, input(None, 0, 1, "Fictional checking", false))
            .is_err());
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER reject_account_audit BEFORE INSERT ON audit_events WHEN NEW.event='statement_account_saved' BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
        assert!(w
            .save_statement_account(&a.id, input(Some(&id), 2, 1, "No commit", false))
            .is_err());
        assert!(w
            .save_statement_account(&a.id, input(None, 0, 1, "No new account", false))
            .is_err());
        let unchanged = w.get_statement_accounts(&a.id).unwrap();
        assert_eq!(unchanged.accounts.len(), 1);
        assert_eq!(unchanged.accounts[0].revision, 2);
        assert_eq!(unchanged.import_revision, 1);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_account_audit;")
            .unwrap();
        let r = w
            .save_statement_account(&a.id, input(Some(&id), 2, 1, "Renamed checking", false))
            .unwrap();
        assert!(!r.accounts[0].archived);
        let s = w.session().unwrap();
        assert!(s
            .connection
            .execute("DELETE FROM statement_accounts WHERE id=?1", [&id])
            .is_err());
        assert!(s
            .connection
            .execute(
                "UPDATE statement_account_history SET label='changed' WHERE account_id=?1",
                [&id]
            )
            .is_err());
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.get_statement_accounts(&a.id).is_err());
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        drop(w);
        let mut w = Workspace::new(dir.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let saved = w.get_statement_accounts(&a.id).unwrap();
        assert_eq!(saved.accounts[0].revision, 3);
        assert_eq!(saved.accounts[0].history[0].label, "Fictional checking");
        assert!(!saved.accounts[0].archived);
    }
    #[test]
    fn imported_source_and_snapshot_bytes_survive_metadata_changes_and_archived_import_is_blocked()
    {
        use crate::imports::{DocumentInput, StageInput};
        let (dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        let path = dir.path().join("fictional.csv");
        fs::write(
            &path,
            "Date,Description,Amount,Category,Type\n2026-09-01,Fictional wages,2000,Wages,income\n",
        )
        .unwrap();
        let preview = w
            .inspect_client_csv(a.id.clone(), path.to_string_lossy().into())
            .unwrap();
        let doc = DocumentInput {
            balance_order: None,
            pdf_selection: None,
            account_id: None,
            account_label: "Original checking".into(),
            account_kind: "checking".into(),
            period_start: None,
            period_end: None,
            opening_balance: None,
            closing_balance: None,
            retain_source: true,
            mapping: crate::imports::tests::mapping(),
        };
        let original = w
            .stage_client_csv(
                &a.id,
                StageInput {
                    expected_revision: 0,
                    token: preview.token,
                    document: doc.clone(),
                },
            )
            .unwrap();
        let bytes = serde_json::to_vec(&original).unwrap();
        let id = original.accounts[0].id.clone();
        let r = w
            .save_statement_account(&a.id, input(Some(&id), 0, 1, "Current checking", true))
            .unwrap();
        assert_eq!(r.accounts[0].source_count, 1);
        assert_eq!(
            bytes,
            serde_json::to_vec(&w.get_client_imports(&a.id).unwrap()).unwrap()
        );
        // Collection source/account references continue resolving the unchanged original identity.
        let choices = w.get_document_collection(&a.id, None).unwrap();
        assert_eq!(choices.accounts[0].label, "Original checking");
        assert_eq!(choices.sources[0].account_id, id);
        let second = dir.path().join("next.csv");
        fs::write(
            &second,
            "Date,Description,Amount,Category,Type\n2026-09-02,Fictional wages,500,Wages,income\n",
        )
        .unwrap();
        let preview = w
            .inspect_client_csv(a.id.clone(), second.to_string_lossy().into())
            .unwrap();
        let mut doc = doc;
        doc.account_id = Some(id.clone());
        assert!(w
            .stage_client_csv(
                &a.id,
                StageInput {
                    expected_revision: 1,
                    token: preview.token.clone(),
                    document: doc.clone()
                }
            )
            .err()
            .unwrap()
            .contains("archived"));
        assert_eq!(w.get_client_imports(&a.id).unwrap().revision, 1);
        w.save_statement_account(&a.id, input(Some(&id), 1, 1, "Current checking", false))
            .unwrap();
        let r = w
            .stage_client_csv(
                &a.id,
                StageInput {
                    expected_revision: 1,
                    token: preview.token,
                    document: doc,
                },
            )
            .unwrap();
        assert_eq!(r.documents.len(), 2);
        assert_eq!(r.documents[1].input.account_label, "Original checking");
        assert_eq!(r.accounts[0].id, id);
    }
    #[test]
    fn account_migration_rollback_and_corrupt_history_prevent_restore() {
        let (_dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        let s = w.session().unwrap();
        s.connection.execute_batch("DROP TABLE IF EXISTS category_rule_applications; DROP TABLE IF EXISTS category_rule_versions; DROP INDEX IF EXISTS review_history_rule_scope; DROP TABLE IF EXISTS card_facts_history; DROP TABLE IF EXISTS card_review_history; DROP TABLE IF EXISTS card_decisions; DROP TABLE IF EXISTS card_statements; DROP TABLE IF EXISTS final_report_approvals; DROP TABLE IF EXISTS coverage_review_history; DROP TABLE IF EXISTS firm_branding_history; DROP TABLE statement_account_history; DROP TRIGGER statement_accounts_no_update; DROP TRIGGER statement_accounts_no_delete; PRAGMA user_version=18; CREATE TABLE audit_events_v19(collision TEXT);").unwrap();
        assert!(migrate(&mut s.connection, false).is_err());
        assert_eq!(
            s.connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            18
        );
        assert_eq!(
            s.connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name='statement_account_history'",
                    [],
                    |r| r.get::<_, i32>(0)
                )
                .unwrap(),
            0
        );
        s.connection
            .execute_batch("DROP TABLE audit_events_v19;")
            .unwrap();
        migrate(&mut s.connection, false).unwrap();
        let first = w
            .save_statement_account(&a.id, input(None, 0, 0, "Fictional", false))
            .unwrap();
        let id = &first.accounts[0].id;
        w.save_statement_account(&a.id, input(Some(id), 1, 1, "Renamed", false))
            .unwrap();
        w.session().unwrap().connection.execute_batch("DROP TRIGGER statement_account_history_no_delete; DELETE FROM statement_account_history WHERE revision=1;").unwrap();
        assert!(w.get_statement_accounts(&a.id).is_err());
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        assert!(!w.data_dir.join("workspace-active.json").exists());
    }
}
