//! Scoped immutable completeness review versions and report source bindings.
use super::*;
use crate::coverage_review::{self as review, Input, Record, Snapshot, Version};
pub(super) fn versions(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<Vec<Snapshot>, String> {
    let mut q=c.prepare("SELECT revision,data_json,updated_at FROM coverage_review_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision LIMIT 101").map_err(storage_error)?;
    let rows = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(storage_error)?;
    let source = imports_store::read_imports(c, w, client)?;
    let choices = review::choices(&source);
    let mut result = Vec::new();
    for row in rows {
        let (rev, json, at) = row.map_err(storage_error)?;
        if json.len() > review::MAX_JSON || rev != result.len() as i64 + 1 || rev > 100 {
            return Err(STORAGE_ERROR.into());
        }
        let s: Snapshot = serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
        review::validate(&s).map_err(|_| STORAGE_ERROR.to_owned())?;
        if serde_json::to_string(&s).map_err(|_| STORAGE_ERROR.to_owned())? != json
            || s.client_id != client
            || s.revision != rev
            || s.updated_at != at
            || s.source_revision > source.revision
            || s.choices
                .accounts
                .iter()
                .any(|a| !choices.accounts.contains(a))
            || s.choices
                .sources
                .iter()
                .any(|d| !choices.sources.contains(d))
        {
            return Err(STORAGE_ERROR.into());
        }
        result.push(s);
    }
    Ok(result)
}
pub(super) fn validate_records(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(), String> {
    versions(c, w, client).map(|_| ())
}
pub(super) fn revision(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<i64, String> {
    c.query_row("SELECT COALESCE(MAX(revision),0) FROM coverage_review_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)
}
pub(super) fn validate_source(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    s: &Snapshot,
) -> Result<(), String> {
    let versions = versions(c, w, client)?;
    if versions.iter().any(|v| v == s) {
        Ok(())
    } else {
        Err("The captured completeness review differs from its original client history.".into())
    }
}
pub(super) fn capture(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    selection: review::Selection,
    source: &crate::imports::Snapshot,
    profile: &Client,
    period: (&str, &str),
) -> Result<Snapshot, String> {
    let latest = versions(c, w, client)?
        .pop()
        .ok_or("Save and review a completeness review first.")?;
    if selection.revision != latest.revision
        || latest.source_revision != source.revision
        || latest.profile_updated_at != profile.updated_at
        || latest.data.from_month != period.0
        || latest.data.through_month != period.1
        || latest.choices != review::choices(source)
    {
        return Err("The completeness review changed or has a different client, source or period context. Review the current saved version before recording; nothing was saved.".into());
    }
    Ok(latest)
}
fn read(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    requested: Option<i64>,
) -> Result<Record, String> {
    let profile = scoped_client(c, w, client)?;
    let all = versions(c, w, client)?;
    let latest = all.len() as i64;
    let snapshot = match requested {
        Some(r) => Some(
            all.iter()
                .find(|s| s.revision == r)
                .cloned()
                .ok_or("This review version is unavailable for this client.")?,
        ),
        None => all.last().cloned(),
    };
    let source = imports_store::read_imports(c, w, client)?;
    let mut reasons = vec![];
    if let Some(s) = &snapshot {
        if s.source_revision != source.revision || s.profile_updated_at != profile.updated_at {
            reasons.push("Client or source context changed after this review. Review the latest saved sources and record a new version; the historical review remains unchanged.".into());
        }
        if s.revision != latest {
            reasons.push("This is a historical review; a later version exists.".into());
        }
    }
    let unresolved = snapshot
        .as_ref()
        .map(review::unresolved)
        .unwrap_or_default();
    let history = all
        .iter()
        .map(|s| Version {
            revision: s.revision,
            updated_at: s.updated_at.clone(),
            from_month: s.data.from_month.clone(),
            through_month: s.data.through_month.clone(),
        })
        .collect();
    Ok(Record {
        client_id: client.into(),
        latest_revision: latest,
        snapshot,
        history,
        choices: review::choices(&source),
        current_source_revision: source.revision,
        current_profile_updated_at: profile.updated_at,
        needs_review: !reasons.is_empty(),
        review_reasons: reasons,
        unresolved,
    })
}
impl Workspace {
    pub fn get_coverage_review(
        &mut self,
        client: &str,
        revision: Option<i64>,
    ) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let result = read(&tx, &s.info, client, revision)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn save_coverage_review(&mut self, client: &str, input: Input) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let profile = scoped_client(&tx, &s.info, client)?;
        let latest = versions(&tx, &s.info, client)?.len() as i64;
        let source = imports_store::read_imports(&tx, &s.info, client)?;
        if input.expected_revision != latest
            || input.expected_source_revision != source.revision
            || input.expected_profile_updated_at != profile.updated_at
        {
            return Err("The review, client or source context changed in another window. Your draft is retained; review the latest context before saving.".into());
        }
        if latest >= 100 {
            return Err("This client reached the 100-version completeness review limit.".into());
        }
        let choices = review::choices(&source);
        let data = review::canonical(input.data, &choices)?;
        let snapshot = Snapshot {
            client_id: client.into(),
            revision: latest + 1,
            updated_at: timestamp(),
            source_revision: source.revision,
            profile_updated_at: profile.updated_at,
            choices,
            data,
        };
        review::validate(&snapshot)?;
        let json = serde_json::to_string(&snapshot).map_err(|_| STORAGE_ERROR.to_owned())?;
        if json.len() > review::MAX_JSON {
            return Err("Completeness review exceeds the storage size limit.".into());
        }
        tx.execute("INSERT INTO coverage_review_history(client_id,organization_id,counselor_id,revision,data_json,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",params![client,s.info.organization_id,s.info.counselor_id,snapshot.revision,json,snapshot.updated_at]).map_err(storage_error)?;
        audit(&tx, &s.info, "coverage_review_saved", Some(client))?;
        let result = read(&tx, &s.info, client, None)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage_review::{Data, Expectation};
    use crate::storage::tests::{client_input, configured, PASSWORD};
    fn data() -> Data {
        Data {
            from_month: "2026-09".into(),
            through_month: "2026-09".into(),
            inventory_reviewed: false,
            inventory_notes: "Fictional household inventory discussion".into(),
            accounts: vec![Expectation {
                account_id: None,
                label: "Unprovided fictional loan".into(),
                kind: "loan".into(),
                expected_start: None,
                expected_end: None,
                exception: "Loan statement still requested".into(),
                source_ids: vec![],
            }],
            missing_information: "Fictional loan balance and terms unknown".into(),
            conclusion: "Collect remaining evidence".into(),
        }
    }
    fn input(r: &Record, d: Data) -> Input {
        Input {
            expected_revision: r.latest_revision,
            expected_source_revision: r.current_source_revision,
            expected_profile_updated_at: r.current_profile_updated_at.clone(),
            data: d,
        }
    }
    #[test]
    fn coverage_review_crud_stale_scope_atomic_audit_history_and_recovery() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        assert!(w.get_coverage_review("wrong-client", None).is_err());
        let initial = w.get_coverage_review(&c.id, None).unwrap();
        assert!(initial.snapshot.is_none());
        let before = serde_json::to_vec(&w.get_client_imports(&c.id).unwrap()).unwrap();
        let first = w
            .save_coverage_review(&c.id, input(&initial, data()))
            .unwrap();
        assert_eq!(first.latest_revision, 1);
        assert!(!first.unresolved.is_empty());
        assert!(!first.needs_review);
        let original = first.snapshot.as_ref().unwrap().clone();
        assert!(w.get_coverage_review(&other.id, Some(1)).is_err());
        assert!(w
            .save_coverage_review(&c.id, input(&initial, data()))
            .is_err());
        let mut changed = data();
        changed.missing_information = "Fictional loan statement and rate pending".into();
        let second = w
            .save_coverage_review(&c.id, input(&first, changed.clone()))
            .unwrap();
        assert_eq!(second.latest_revision, 2);
        assert_eq!(
            w.get_coverage_review(&c.id, Some(1))
                .unwrap()
                .snapshot
                .unwrap(),
            original
        );
        assert_eq!(
            serde_json::to_vec(&w.get_client_imports(&c.id).unwrap()).unwrap(),
            before
        );
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER fail_coverage_audit BEFORE INSERT ON audit_events WHEN NEW.event='coverage_review_saved' BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;").unwrap();
        assert!(w
            .save_coverage_review(&c.id, input(&second, changed.clone()))
            .is_err());
        assert_eq!(
            w.get_coverage_review(&c.id, None).unwrap().latest_revision,
            2
        );
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER fail_coverage_audit;")
            .unwrap();
        let mut profile = client_input();
        profile.id = Some(c.id.clone());
        profile.goals = "Fictional changed goals".into();
        w.save_client(profile).unwrap();
        assert!(w
            .save_coverage_review(&c.id, input(&second, changed.clone()))
            .is_err());
        assert!(w.get_coverage_review(&c.id, None).unwrap().needs_review);
        w.lock_workspace();
        assert!(w.get_coverage_review(&c.id, None).is_err());
        drop(w);
        let mut w = Workspace::new(root.path().into());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            w.get_coverage_review(&c.id, Some(1))
                .unwrap()
                .snapshot
                .unwrap(),
            original
        );
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let prepared = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(prepared.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            w.get_coverage_review(&c.id, Some(1))
                .unwrap()
                .snapshot
                .unwrap(),
            original
        );
        assert!(w
            .session()
            .unwrap()
            .connection
            .execute(
                "DELETE FROM coverage_review_history WHERE client_id=?1",
                [&c.id]
            )
            .is_err());
        // Restore rejects noncanonical saved history even if the SQLite file itself is valid.
        w.session().unwrap().connection.execute_batch("DROP TRIGGER coverage_review_no_update; UPDATE coverage_review_history SET data_json='{}';").unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
    }
    #[test]
    fn coverage_review_migration_is_atomic_and_preserves_prior_records() {
        let (_root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let s = w.session().unwrap();
        let before: i64 = s
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        s.connection.execute_batch("DROP TABLE IF EXISTS card_facts_history; DROP TABLE IF EXISTS card_review_history; DROP TABLE IF EXISTS card_decisions; DROP TABLE IF EXISTS card_statements; DROP TABLE IF EXISTS final_report_approvals; DROP TABLE coverage_review_history; PRAGMA user_version=20; CREATE TABLE audit_events_v21(collision TEXT);").unwrap();
        assert!(migrate(&mut s.connection, false).is_err());
        let version: i32 = s
            .connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 20);
        assert_eq!(
            s.connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name='coverage_review_history'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        s.connection
            .execute_batch("DROP TABLE audit_events_v21;")
            .unwrap();
        migrate(&mut s.connection, false).unwrap();
        let after: i64 = s
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after);
        assert_eq!(
            scoped_client(&s.connection, &s.info, &c.id).unwrap().id,
            c.id
        );
        assert!(read(&s.connection, &s.info, &c.id, None)
            .unwrap()
            .snapshot
            .is_none());
    }
    #[test]
    fn coverage_review_rejects_invalid_dates_controls_types_and_forged_sources() {
        let (_root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let r = w.get_coverage_review(&c.id, None).unwrap();
        let mut d = data();
        d.accounts[0].expected_start = Some("2026-09-01".into());
        assert!(w.save_coverage_review(&c.id, input(&r, d.clone())).is_err());
        d.accounts[0].expected_end = Some("2026-10-01".into());
        assert!(w.save_coverage_review(&c.id, input(&r, d.clone())).is_err());
        d.accounts[0].expected_end = Some("2026-09-30".into());
        d.accounts[0].source_ids = vec![Uuid::new_v4().to_string()];
        assert!(w.save_coverage_review(&c.id, input(&r, d)).is_err());
        let mut d = data();
        d.inventory_reviewed = true;
        d.inventory_notes.clear();
        assert!(w.save_coverage_review(&c.id, input(&r, d)).is_err());
        let mut d = data();
        d.accounts[0].label = "Invalid\0control".into();
        assert!(w.save_coverage_review(&c.id, input(&r, d)).is_err());
        assert_eq!(
            w.get_coverage_review(&c.id, None).unwrap().latest_revision,
            0
        );
    }
}
