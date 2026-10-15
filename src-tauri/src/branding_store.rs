//! Authenticated firm-scoped append-only presentation and report bindings.
use super::*;
use crate::branding::{self, Input, Record, Snapshot, Version};
fn snapshots(c: &Connection, w: &WorkspaceInfo) -> Result<Vec<Snapshot>, String> {
    let mut q=c.prepare("SELECT revision,updated_at,data_json FROM firm_branding_history WHERE organization_id=?1 ORDER BY revision LIMIT 101").map_err(storage_error)?;
    let rows = q
        .query_map([&w.organization_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(storage_error)?;
    let mut all = Vec::new();
    for row in rows {
        let (revision, updated_at, json) = row.map_err(storage_error)?;
        if json.len() > 131072 || revision != all.len() as i64 + 1 || revision > 100 {
            return Err(STORAGE_ERROR.into());
        }
        let data = serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
        let s = Snapshot {
            organization_id: w.organization_id.clone(),
            revision,
            updated_at,
            data,
        };
        branding::validate(&s).map_err(|_| STORAGE_ERROR.to_owned())?;
        all.push(s);
    }
    Ok(all)
}
pub(super) fn read(c: &Connection, w: &WorkspaceInfo) -> Result<Record, String> {
    let name: String = c
        .query_row(
            "SELECT name FROM organizations WHERE id=?1",
            [&w.organization_id],
            |r| r.get(0),
        )
        .map_err(storage_error)?;
    let all = snapshots(c, w)?;
    let latest = all.last();
    let data = latest.map_or_else(|| branding::defaults(&name), |s| s.data.clone());
    if data.name != name || branding::canonical(data.clone())? != data {
        return Err(STORAGE_ERROR.into());
    }
    Ok(Record {
        organization_id: w.organization_id.clone(),
        revision: all.len() as i64,
        updated_at: latest.map(|s| s.updated_at.clone()),
        data,
        history: all
            .into_iter()
            .map(|s| Version {
                revision: s.revision,
                name: s.data.name,
                updated_at: s.updated_at,
            })
            .collect(),
    })
}
pub(super) fn validate_source(
    c: &Connection,
    w: &WorkspaceInfo,
    s: &Snapshot,
) -> Result<(), String> {
    branding::validate(s)?;
    if s.organization_id != w.organization_id {
        return Err(STORAGE_ERROR.into());
    }
    let all = snapshots(c, w)?;
    if all.get(s.revision as usize - 1) != Some(s) {
        return Err(STORAGE_ERROR.into());
    }
    Ok(())
}
pub(super) fn revision(c: &Connection, w: &WorkspaceInfo) -> Result<i64, String> {
    c.query_row(
        "SELECT COALESCE(MAX(revision),0) FROM firm_branding_history WHERE organization_id=?1",
        [&w.organization_id],
        |r| r.get(0),
    )
    .map_err(storage_error)
}
pub(super) fn capture(
    c: &Connection,
    w: &WorkspaceInfo,
    expected: Option<i64>,
) -> Result<Option<Snapshot>, String> {
    let current = read(c, w)?;
    if current.revision == 0 && expected.is_none() {
        return Ok(None);
    }
    if expected != Some(current.revision) || current.revision == 0 {
        return Err("Firm branding changed or has not been reviewed. Refresh the report's firm branding preview before recording; nothing was saved.".into());
    }
    Ok(Some(Snapshot {
        organization_id: current.organization_id,
        revision: current.revision,
        updated_at: current.updated_at.ok_or(STORAGE_ERROR)?,
        data: current.data,
    }))
}
impl Workspace {
    pub fn get_firm_branding(&mut self) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let r = read(&tx, &s.info)?;
        tx.commit().map_err(storage_error)?;
        Ok(r)
    }
    pub fn save_firm_branding(&mut self, input: Input) -> Result<Record, String> {
        let data = branding::canonical(input.data)?;
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let current = read(&tx, &s.info)?;
        if input.expected_revision != current.revision {
            return Err("Firm branding changed in another window. Your draft is retained; discard and reload before saving.".into());
        }
        if current.revision >= 100 {
            return Err("Firm branding reached the current 100-version history limit.".into());
        }
        let json = serde_json::to_string(&data).map_err(|_| STORAGE_ERROR.to_owned())?;
        if json.len() > 131072 {
            return Err("Firm branding exceeds its storage limit.".into());
        }
        tx.execute("INSERT INTO firm_branding_history(organization_id,counselor_id,revision,data_json,updated_at) VALUES(?1,?2,?3,?4,?5)",params![s.info.organization_id,s.info.counselor_id,current.revision+1,json,timestamp()]).map_err(storage_error)?;
        tx.execute(
            "UPDATE organizations SET name=?1 WHERE id=?2",
            params![data.name, s.info.organization_id],
        )
        .map_err(storage_error)?;
        audit(
            &tx,
            &s.info,
            "firm_branding_saved",
            Some(&s.info.organization_id),
        )?;
        let result = read(&tx, &s.info)?;
        tx.commit().map_err(storage_error)?;
        s.info.organization_name = data.name;
        Ok(result)
    }
    pub fn inspect_branding_logo(&mut self, path: String) -> Result<String, String> {
        self.session()?;
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err("Choose an absolute path to a local PNG logo.".into());
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| "Unable to read the local PNG logo.")?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|_| "Unable to read the local PNG logo.")?;
        branding::inspect_logo(&bytes)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::{configured, PASSWORD};
    #[test]
    fn branding_revision_scope_atomic_audit_session_refresh_and_encrypted_recovery() {
        let (dir, mut w, _) = configured();
        let original = w.workspace_info().unwrap();
        let mut data = w.get_firm_branding().unwrap().data;
        data.name = "Fictional advisory practice".into();
        data.email = "contact@example.invalid".into();
        data.logo_png =
            Some(crate::branding::inspect_logo(&crate::branding::tests::logo()).unwrap());
        let mut other = Workspace::new(dir.path().to_path_buf());
        other.unlock_workspace(PASSWORD.into()).unwrap();
        let first = w
            .save_firm_branding(Input {
                expected_revision: 0,
                data: data.clone(),
            })
            .unwrap();
        assert_eq!(first.revision, 1);
        assert_eq!(other.workspace_info().unwrap().organization_name, data.name);
        assert!(other
            .save_firm_branding(Input {
                expected_revision: 0,
                data: data.clone()
            })
            .is_err());
        let mut foreign = original.clone();
        foreign.organization_id = Uuid::new_v4().to_string();
        let s = w.session().unwrap();
        assert!(read(&s.connection, &foreign).is_err());
        let captured = capture(&s.connection, &s.info, Some(1)).unwrap().unwrap();
        let mut forged = captured.clone();
        forged.data.name = "Forged".into();
        assert!(validate_source(&s.connection, &s.info, &forged).is_err());
        s.connection.execute_batch("CREATE TRIGGER reject_branding_audit BEFORE INSERT ON audit_events WHEN NEW.event='firm_branding_saved' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
        let mut changed = data.clone();
        changed.name = "Uncommitted name".into();
        assert!(w
            .save_firm_branding(Input {
                expected_revision: 1,
                data: changed
            })
            .is_err());
        assert_eq!(w.workspace_info().unwrap().organization_name, data.name);
        assert_eq!(w.get_firm_branding().unwrap().revision, 1);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_branding_audit;")
            .unwrap();
        let mut changed = data;
        changed.phone = "555-0100".into();
        w.save_firm_branding(Input {
            expected_revision: 1,
            data: changed.clone(),
        })
        .unwrap();
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.get_firm_branding().is_err());
        assert!(w.inspect_branding_logo("/tmp/no-logo.png".into()).is_err());
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let current = w.get_firm_branding().unwrap();
        assert_eq!(current.revision, 2);
        assert_eq!(current.history.len(), 2);
        assert_eq!(current.data, changed);
        let s = w.session().unwrap();
        validate_source(&s.connection, &s.info, &captured).unwrap();
    }
    #[test]
    fn branding_migration_and_corrupt_history_fail_before_recovery_activation() {
        let (_dir, mut w, _) = configured();
        let s = w.session().unwrap();
        s.connection.execute_batch("DROP TABLE IF EXISTS card_review_history; DROP TABLE IF EXISTS card_decisions; DROP TABLE IF EXISTS card_statements; DROP TABLE IF EXISTS final_report_approvals; DROP TABLE IF EXISTS coverage_review_history; DROP TABLE firm_branding_history; PRAGMA user_version=19; CREATE TABLE audit_events_v20(collision TEXT);").unwrap();
        assert!(migrate(&mut s.connection, false).is_err());
        assert_eq!(
            s.connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            19
        );
        assert_eq!(
            s.connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name='firm_branding_history'",
                    [],
                    |r| r.get::<_, i32>(0)
                )
                .unwrap(),
            0
        );
        s.connection
            .execute_batch("DROP TABLE audit_events_v20;")
            .unwrap();
        migrate(&mut s.connection, false).unwrap();
        let mut data = w.get_firm_branding().unwrap().data;
        data.phone = "555-0100".into();
        w.save_firm_branding(Input {
            expected_revision: 0,
            data: data.clone(),
        })
        .unwrap();
        data.phone = "555-0101".into();
        w.save_firm_branding(Input {
            expected_revision: 1,
            data,
        })
        .unwrap();
        w.session().unwrap().connection.execute_batch("DROP TRIGGER firm_branding_no_delete; DELETE FROM firm_branding_history WHERE revision=1;").unwrap();
        assert!(w.get_firm_branding().is_err());
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        assert!(!w.data_dir.join("workspace-active.json").exists());
    }
}
