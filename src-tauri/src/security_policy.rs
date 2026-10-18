//! Encrypted workspace inactivity policy. Reading it never extends activity.
use super::*;
const POLICY_ERROR: &str = "The workspace security policy could not be verified. Lock and reopen the workspace, or restore a verified backup.";
const MAX_CHANGES: usize = 1000;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub revision: i64,
    pub idle_minutes: u32,
    pub saved_at: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    pub revision: i64,
    pub idle_minutes: u32,
    pub total_changes: usize,
    pub history: Vec<Change>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub idle_minutes: String,
}

pub(super) fn read(connection: &Connection, identity: &WorkspaceInfo) -> Result<Policy, String> {
    let mut statement = connection.prepare("SELECT revision,organization_id,counselor_id,idle_minutes,saved_at FROM workspace_security_history ORDER BY revision LIMIT 1001").map_err(|_|POLICY_ERROR.to_owned())?;
    let mut rows = statement.query([]).map_err(|_| POLICY_ERROR.to_owned())?;
    let mut history = Vec::new();
    while let Some(row) = rows.next().map_err(|_| POLICY_ERROR.to_owned())? {
        let revision: i64 = row.get(0).map_err(|_| POLICY_ERROR.to_owned())?;
        let organization: String = row.get(1).map_err(|_| POLICY_ERROR.to_owned())?;
        let counselor: String = row.get(2).map_err(|_| POLICY_ERROR.to_owned())?;
        let idle_minutes: u32 = row.get(3).map_err(|_| POLICY_ERROR.to_owned())?;
        let saved_at: String = row.get(4).map_err(|_| POLICY_ERROR.to_owned())?;
        if history.len() >= MAX_CHANGES
            || revision != history.len() as i64 + 1
            || organization != identity.organization_id
            || counselor != identity.counselor_id
            || !(1..=60).contains(&idle_minutes)
            || chrono::DateTime::parse_from_rfc3339(&saved_at).is_err()
        {
            return Err(POLICY_ERROR.into());
        }
        history.push(Change {
            revision,
            idle_minutes,
            saved_at,
        });
    }
    let (revision, idle_minutes) = history
        .last()
        .map(|r| (r.revision, r.idle_minutes))
        .unwrap_or((0, 15));
    let total_changes = history.len();
    let history = history.into_iter().rev().take(10).collect();
    Ok(Policy {
        revision,
        idle_minutes,
        total_changes,
        history,
    })
}

impl Workspace {
    /// UI reloads and timer observations must not manufacture user activity.
    pub fn get_security_policy(&mut self) -> Result<Policy, String> {
        self.expire_if_idle();
        let session = self.session.as_ref().ok_or_else(|| LOCKED.to_owned())?;
        read(&session.connection, &session.info)
    }
    pub fn save_security_policy(&mut self, input: Input) -> Result<Policy, String> {
        let session = self.session()?;
        let minutes = input.idle_minutes.trim();
        if minutes.is_empty() || !minutes.bytes().all(|c| c.is_ascii_digit()) {
            return Err("Enter a whole number from 1 to 60 minutes.".into());
        }
        let minutes: u32 = minutes
            .parse()
            .map_err(|_| "Enter a whole number from 1 to 60 minutes.".to_owned())?;
        if !(1..=60).contains(&minutes) {
            return Err("Enter a whole number from 1 to 60 minutes.".into());
        }
        let transaction = session
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let current = read(&transaction, &session.info)?;
        if current.revision != input.expected_revision {
            return Err(
                "Security settings changed in another instance. Reload before saving.".into(),
            );
        }
        if current.idle_minutes == minutes {
            return Err("Choose a different inactivity timeout before saving.".into());
        }
        if current.total_changes >= MAX_CHANGES {
            return Err("The retained security-history limit has been reached.".into());
        }
        let revision = current.revision + 1;
        transaction.execute("INSERT INTO workspace_security_history(revision,organization_id,counselor_id,idle_minutes,saved_at) VALUES(?1,?2,?3,?4,?5)",params![revision,session.info.organization_id,session.info.counselor_id,minutes,timestamp()]).map_err(storage_error)?;
        audit(
            &transaction,
            &session.info,
            "security_policy_saved",
            Some(&revision.to_string()),
        )?;
        let result = read(&transaction, &session.info)?;
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::{client_input, configured, PASSWORD};
    fn input(revision: i64, minutes: &str) -> Input {
        Input {
            expected_revision: revision,
            idle_minutes: minutes.into(),
        }
    }
    #[test]
    fn default_boundary_non_activity_reads_and_other_instance_shortening_are_enforced() {
        let (root, mut w, _) = configured();
        assert_eq!(w.get_security_policy().unwrap().idle_minutes, 15);
        w.session.as_mut().unwrap().last_activity = Instant::now() - Duration::from_secs(14 * 60);
        let activity = w.session.as_ref().unwrap().last_activity;
        assert_eq!(w.get_security_policy().unwrap().revision, 0);
        assert_eq!(w.session.as_ref().unwrap().last_activity, activity);
        assert!(!w.expire_if_idle());
        w.session.as_mut().unwrap().last_activity = Instant::now() - AUTO_LOCK_AFTER;
        assert!(w.expire_if_idle());
        assert!(w.get_security_policy().is_err());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let mut other = Workspace::new(root.path().into());
        other.unlock_workspace(PASSWORD.into()).unwrap();
        other.session.as_mut().unwrap().last_activity = Instant::now() - Duration::from_secs(120);
        w.save_security_policy(input(0, "1")).unwrap();
        assert!(other.expire_if_idle());
        assert!(other.session.is_none());
        assert!(other.list_clients().is_err());
        w.session.as_mut().unwrap().last_activity = Instant::now() - Duration::from_secs(61);
        assert!(w.workspace_info().is_err());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let policy = w.save_security_policy(input(1, "60")).unwrap();
        assert_eq!(policy.idle_minutes, 60);
        w.session.as_mut().unwrap().last_activity = Instant::now() - Duration::from_secs(59 * 60);
        assert!(!w.expire_if_idle());
        w.workspace_info().unwrap();
        assert!(w.session.as_ref().unwrap().last_activity.elapsed() < Duration::from_secs(5));
        w.session.as_mut().unwrap().last_activity = Instant::now() - Duration::from_secs(60 * 60);
        assert!(w.expire_if_idle());
    }
    #[test]
    fn bounds_revisions_immutable_history_and_failed_audit_never_change_policy() {
        let (_root, mut w, _) = configured();
        for minutes in [
            "0",
            "61",
            "-1",
            "1.5",
            "1e1",
            "+1",
            "",
            "999999999999999999999999",
        ] {
            assert!(w.save_security_policy(input(0, minutes)).is_err());
            assert_eq!(w.get_security_policy().unwrap().revision, 0);
        }
        let saved = w.save_security_policy(input(0, " 5 ")).unwrap();
        assert_eq!(saved.idle_minutes, 5);
        assert_eq!(saved.history[0].revision, 1);
        assert!(w
            .save_security_policy(input(0, "10"))
            .unwrap_err()
            .contains("Reload"));
        assert!(w.save_security_policy(input(1, "5")).is_err());
        let c = &w.session.as_ref().unwrap().connection;
        assert!(c
            .execute("UPDATE workspace_security_history SET idle_minutes=10", [])
            .is_err());
        assert!(c
            .execute("DELETE FROM workspace_security_history", [])
            .is_err());
        c.execute_batch("CREATE TEMP TRIGGER fail_security_audit BEFORE INSERT ON audit_events WHEN NEW.event='security_policy_saved' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
        assert!(w.save_security_policy(input(1, "10")).is_err());
        assert_eq!(w.get_security_policy().unwrap(), saved);
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER fail_security_audit;")
            .unwrap();
        let updated = w.save_security_policy(input(1, "10")).unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.history[1], saved.history[0]);
        assert_eq!(
            w.session
                .as_ref()
                .unwrap()
                .connection
                .query_row(
                    "SELECT count(*) FROM audit_events WHERE event='security_policy_saved'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
        w.lock_workspace();
        assert!(w.save_security_policy(input(2, "30")).is_err());
    }
    #[test]
    fn policy_survives_restart_password_rotation_and_both_old_and_new_backups() {
        let (root, mut w, _) = configured();
        let destination = tempfile::tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let note = w
            .add_client_note(&client.id, "Synthetic timeout-history note")
            .unwrap();
        let old = w
            .create_backup(destination.path().display().to_string(), PASSWORD.into())
            .unwrap();
        let saved = w.save_security_policy(input(0, "5")).unwrap();
        let new = w
            .create_backup(destination.path().display().to_string(), PASSWORD.into())
            .unwrap();
        let next = "Synthetic-timeout-password-83";
        w.change_workspace_password(super::super::password_change::PasswordChangeInput {
            current_password: PASSWORD.into(),
            new_password: next.into(),
            confirmation: next.into(),
            acknowledge_retained_copies: true,
        })
        .unwrap();
        let mut restarted = Workspace::new(root.path().into());
        restarted.unlock_workspace(next.into()).unwrap();
        assert_eq!(restarted.get_security_policy().unwrap(), saved);
        assert_eq!(
            restarted.list_client_notes(&client.id).unwrap(),
            vec![note.clone()]
        );
        for (backup, minutes) in [(old, 15), (new, 5)] {
            restarted.lock_workspace();
            let review = restarted
                .prepare_restore(backup.path, PASSWORD.into())
                .unwrap();
            restarted
                .confirm_restore(review.token, "RESTORE".into())
                .unwrap();
            restarted.unlock_workspace(PASSWORD.into()).unwrap();
            assert_eq!(
                restarted.get_security_policy().unwrap().idle_minutes,
                minutes
            );
            assert_eq!(
                restarted.list_client_notes(&client.id).unwrap(),
                vec![note.clone()]
            );
        }
    }
    #[test]
    fn corrupt_policy_locks_active_sessions_and_rejects_unlock() {
        let (root, mut w, _) = configured();
        w.save_security_policy(input(0, "5")).unwrap();
        // Corrupt an encrypted logical record deliberately. Foreign keys and
        // SQLCipher integrity remain intact, so policy validation must catch it.
        w.session.as_ref().unwrap().connection.execute_batch("DROP TRIGGER workspace_security_no_update; UPDATE workspace_security_history SET saved_at='not-a-timestamp';").unwrap();
        assert!(w.expire_if_idle());
        assert!(w.session.is_none());
        assert!(w
            .unlock_workspace(PASSWORD.into())
            .unwrap_err()
            .contains("policy"));
        let mut fresh = Workspace::new(root.path().into());
        assert!(fresh.unlock_workspace(PASSWORD.into()).is_err());
    }
    #[test]
    fn schema28_migration_rolls_back_then_preserves_notes_and_default_timeout() {
        let (root, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let note = w
            .add_client_note(&client.id, "Synthetic legacy timeout note")
            .unwrap();
        let c = &mut w.session.as_mut().unwrap().connection;
        c.execute_batch("DROP TABLE IF EXISTS transaction_inspection_details; DROP TABLE household_events; DROP TABLE workspace_security_history; PRAGMA user_version=28; CREATE TABLE audit_events_v29(collision TEXT);").unwrap();
        assert!(migrate(c, false).is_err());
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            28
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='workspace_security_history'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        c.execute_batch("DROP TABLE audit_events_v29;").unwrap();
        w.lock_workspace();
        let mut restarted = Workspace::new(root.path().into());
        restarted.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(restarted.get_security_policy().unwrap().idle_minutes, 15);
        assert_eq!(restarted.list_client_notes(&client.id).unwrap(), vec![note]);
    }
    #[test]
    fn history_is_bounded_complete_and_rejects_revision_gaps() {
        let (_root, mut w, info) = configured();
        w.session.as_ref().unwrap().connection.execute("WITH RECURSIVE versions(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM versions WHERE n<1000) INSERT INTO workspace_security_history SELECT n,?1,?2,5,?3 FROM versions",params![info.organization_id,info.counselor_id,timestamp()]).unwrap();
        let policy = w.get_security_policy().unwrap();
        assert_eq!(policy.total_changes, 1000);
        assert_eq!(policy.history.len(), 10);
        assert_eq!(policy.history[0].revision, 1000);
        assert_eq!(policy.history[9].revision, 991);
        assert!(w
            .save_security_policy(input(1000, "10"))
            .unwrap_err()
            .contains("limit"));
        w.session.as_ref().unwrap().connection.execute_batch("DROP TRIGGER workspace_security_no_delete; DELETE FROM workspace_security_history WHERE revision=500;").unwrap();
        assert!(w.expire_if_idle());
        assert!(w.session.is_none());
    }
}
