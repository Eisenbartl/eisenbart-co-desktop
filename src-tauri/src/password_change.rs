//! Copy-before-switch password rotation. The active database is never rekeyed
//! in place. Metadata and encrypted pages are published as one generation.
use super::*;
use zeroize::Zeroize;

const CHANGE_ERROR: &str = "Password change could not finish. The previous workspace remains active. Check available disk space and try again.";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasswordChangeInput {
    pub current_password: String,
    pub new_password: String,
    pub confirmation: String,
    pub acknowledge_retained_copies: bool,
}
impl Drop for PasswordChangeInput {
    fn drop(&mut self) {
        self.current_password.zeroize();
        self.new_password.zeroize();
        self.confirmation.zeroize();
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChangeResult {
    pub durability_warning: bool,
}

impl Workspace {
    pub fn change_workspace_password(
        &mut self,
        input: PasswordChangeInput,
    ) -> Result<PasswordChangeResult, String> {
        self.change_password_checked(input, |_, _| Ok(()))
    }

    fn change_password_checked(
        &mut self,
        mut input: PasswordChangeInput,
        mut checkpoint: impl FnMut(&str, &Path) -> Result<(), String>,
    ) -> Result<PasswordChangeResult, String> {
        // Wrap every secret immediately, including rejected confirmations.
        let current = Zeroizing::new(std::mem::take(&mut input.current_password));
        let new = Zeroizing::new(std::mem::take(&mut input.new_password));
        let confirmation = Zeroizing::new(std::mem::take(&mut input.confirmation));
        let _activation = self.activation_lock()?;
        let identity = self.session()?.info.clone();
        validate_password(&current)?;
        validate_password(&new)?;
        if *new != *confirmation {
            return Err("The new passwords do not match.".into());
        }
        if *current == *new {
            return Err("Choose a new password different from the current password.".into());
        }
        if !input.acknowledge_retained_copies {
            return Err("Confirm that older backups and retained workspace copies keep their original passwords.".into());
        }
        let old_metadata = self.read_metadata()?;
        let old_key = derive_key(&current, &old_metadata.salt)?;
        let source = open_encrypted(&self.database_path(), &old_key).map_err(|_| {
            "The current password is incorrect, or the workspace cannot be verified.".to_owned()
        })?;
        backup::verify_database(&source)?;
        if backup::info(&source)? != identity {
            return Err(CHANGE_ERROR.into());
        }
        let mut salt = [0_u8; 32];
        OsRng
            .try_fill_bytes(&mut salt)
            .map_err(|_| CHANGE_ERROR.to_owned())?;
        let metadata = Metadata {
            version: METADATA_VERSION,
            salt,
        };
        let key = derive_key(&new, &salt)?;
        let mut candidate = backup::staging(&self.data_dir)?;
        let database = candidate.path.join(DATABASE_NAME);
        backup::export_snapshot(&source, &database, &key)?;
        drop(source);
        let mut file = create_private_file(&candidate.path.join(METADATA_NAME))?;
        serde_json::to_writer(&mut file, &metadata).map_err(|_| CHANGE_ERROR.to_owned())?;
        file.sync_all().map_err(|_| CHANGE_ERROR.to_owned())?;
        drop(file);
        checkpoint("snapshot", &candidate.path)?;
        let mut connection = open_encrypted(&database, &key)?;
        backup::verify_database(&connection)?;
        if backup::info(&connection)? != identity {
            return Err(CHANGE_ERROR.into());
        }
        let transaction = connection.transaction().map_err(storage_error)?;
        audit(&transaction, &identity, "workspace_password_changed", None)?;
        transaction.commit().map_err(storage_error)?;
        backup::verify_database(&connection)?;
        drop(connection);
        checkpoint("verified", &candidate.path)?;
        // Reopen using the actual persisted metadata, not only the in-memory key.
        let persisted: Metadata = serde_json::from_slice(
            &fs::read(candidate.path.join(METADATA_NAME)).map_err(|_| CHANGE_ERROR.to_owned())?,
        )
        .map_err(|_| CHANGE_ERROR.to_owned())?;
        if persisted.version != METADATA_VERSION || persisted.salt != salt {
            return Err(CHANGE_ERROR.into());
        }
        let verified_key = derive_key(&new, &persisted.salt)?;
        let reopened = open_encrypted(&database, &verified_key)?;
        backup::verify_database(&reopened)?;
        drop(reopened);
        checkpoint("activation", &candidate.path)?;
        let warning = backup::activate_generation(
            &self.data_dir,
            &mut candidate,
            &Uuid::new_v4().to_string(),
        )?;
        // Nothing fallible after activation: report the committed result and
        // discard all session-bound previews, export receipts and drafts.
        self.lock_workspace();
        self.active_dir = candidate.path.clone();
        Ok(PasswordChangeResult {
            durability_warning: warning,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    use sha2::{Digest, Sha256};
    const NEW: &str = "Synthetic-new-workspace-passphrase-84";
    fn input() -> PasswordChangeInput {
        PasswordChangeInput {
            current_password: PASSWORD.into(),
            new_password: NEW.into(),
            confirmation: NEW.into(),
            acknowledge_retained_copies: true,
        }
    }
    // Compare every persisted value, including source BLOBs and exact report
    // payload/digest bytes. Only the new append-only audit event may differ.
    fn contents(c: &Connection) -> Vec<(String, Vec<u8>)> {
        let names = c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name!='audit_events' ORDER BY name").unwrap()
            .query_map([], |r|r.get::<_,String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        names
            .into_iter()
            .map(|name| {
                let mut q = c
                    .prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")))
                    .unwrap();
                let columns = q.column_count();
                let mut rows = q.query([]).unwrap();
                let mut encoded = Vec::new();
                while let Some(row) = rows.next().unwrap() {
                    let mut bytes = Vec::new();
                    for col in 0..columns {
                        let value = row.get::<_, rusqlite::types::Value>(col).unwrap();
                        let part = format!("{value:?}");
                        bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
                        bytes.extend_from_slice(part.as_bytes());
                    }
                    encoded.push(bytes);
                }
                encoded.sort();
                let mut hash = Sha256::new();
                for row in encoded {
                    hash.update((row.len() as u64).to_le_bytes());
                    hash.update(row);
                }
                (name, hash.finalize().to_vec())
            })
            .collect()
    }
    #[test]
    fn rotation_preserves_all_values_old_files_and_backup_passwords_and_locks_stale_instances() {
        let (root, mut w, _) = configured();
        let target = tempfile::tempdir().unwrap();
        w.session()
            .unwrap()
            .connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .unwrap();
        let client = w.save_client(client_input()).unwrap();
        w.add_client_note(&client.id, "Synthetic retained note")
            .unwrap();
        let mut doc = bank::doc("Synthetic password checking");
        doc.retain_source = true;
        let records = bank::stage(
            &mut w,
            &client.id,
            root.path(),
            "synthetic.csv",
            "Date,Description,Amount,Category,Type\n2026-09-01,Synthetic wages,610,Wages,income\n",
            doc,
        );
        let report = w
            .save_assessment_record(&client.id, bank::assessment_input(&records, &client))
            .unwrap();
        let old = w
            .create_backup(target.path().display().to_string(), PASSWORD.into())
            .unwrap();
        w.inspect_client_csv(
            client.id.clone(),
            root.path().join("synthetic.csv").display().to_string(),
        )
        .unwrap();
        assert!(w.pending_csv.is_some());
        let before = contents(&w.session().unwrap().connection);
        let old_audit: Vec<String> = w
            .session()
            .unwrap()
            .connection
            .prepare("SELECT id FROM audit_events ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let old_meta = fs::read(w.metadata_path()).unwrap();
        let old_db = fs::read(w.database_path()).unwrap();
        let mut stale = Workspace::new(root.path().into());
        stale.unlock_workspace(PASSWORD.into()).unwrap();
        assert!(
            !w.with_operation(|w| w.change_workspace_password(input()))
                .unwrap()
                .durability_warning
        );
        assert!(w.session.is_none());
        assert!(w.pending_csv.is_none());
        assert!(w.pending_restore.is_none());
        assert_eq!(fs::read(root.path().join(METADATA_NAME)).unwrap(), old_meta);
        assert_eq!(fs::read(root.path().join(DATABASE_NAME)).unwrap(), old_db);
        assert!(stale
            .with_operation(|w| w.save_client(client_input()))
            .unwrap_err()
            .contains("locked"));
        assert!(stale.session.is_none());
        let mut restarted = Workspace::new(root.path().into());
        assert!(restarted.unlock_workspace(PASSWORD.into()).is_err());
        restarted.unlock_workspace(NEW.into()).unwrap();
        assert_ne!(fs::read(restarted.metadata_path()).unwrap(), old_meta);
        assert_eq!(contents(&restarted.session().unwrap().connection), before);
        let c = &restarted.session().unwrap().connection;
        for id in old_audit {
            assert_eq!(
                c.query_row("SELECT count(*) FROM audit_events WHERE id=?1", [id], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM audit_events WHERE event='workspace_password_changed'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert!(c
            .execute(
                "DELETE FROM audit_events WHERE event='workspace_password_changed'",
                []
            )
            .is_err());
        assert_eq!(
            restarted
                .get_assessment_record(&client.id, &report.payload.id)
                .unwrap()
                .digest,
            report.digest
        );
        let newer = restarted
            .create_backup(target.path().display().to_string(), NEW.into())
            .unwrap();
        for (backup, password, wrong) in [(old, PASSWORD, NEW), (newer, NEW, PASSWORD)] {
            let destination = tempfile::tempdir().unwrap();
            let mut restored = Workspace::new(destination.path().into());
            assert!(restored
                .prepare_restore(backup.path.clone(), wrong.into())
                .is_err());
            let review = restored
                .prepare_restore(backup.path, password.into())
                .unwrap();
            restored
                .confirm_restore(review.token, "RESTORE".into())
                .unwrap();
            restored.unlock_workspace(password.into()).unwrap();
            assert_eq!(contents(&restored.session().unwrap().connection), before);
        }
    }
    #[test]
    fn rejected_passwords_confirmation_and_acknowledgment_never_publish_or_disclose_secrets() {
        let (root, mut w, _) = configured();
        for case in 0..6 {
            let mut request = input();
            match case {
                0 => request.current_password = "Synthetic-wrong-password".into(),
                1 => request.confirmation = "Synthetic-mismatch".into(),
                2 => request.new_password = "short".into(),
                3 => {
                    request.new_password = PASSWORD.into();
                    request.confirmation = PASSWORD.into();
                }
                4 => request.acknowledge_retained_copies = false,
                _ => {
                    w.lock_workspace();
                }
            }
            let error = w.change_workspace_password(request).err().unwrap();
            assert!(!error.contains(PASSWORD));
            assert!(!error.contains(NEW));
            assert!(!root.path().join("workspace-active.json").exists());
        }
        w.unlock_workspace(PASSWORD.into()).unwrap();
    }
    #[test]
    fn snapshot_audit_metadata_and_activation_failures_leave_old_generation_usable() {
        for point in ["snapshot", "audit", "metadata", "activation"] {
            let (root, mut w, _) = configured();
            let client = w.save_client(client_input()).unwrap();
            let result=w.change_password_checked(input(),|stage,path| {
                if stage=="snapshot" && point=="snapshot" {return Err("Synthetic full-disk snapshot failure".into());}
                if stage=="snapshot" && point=="audit" {
                    let metadata:Metadata=serde_json::from_slice(&fs::read(path.join(METADATA_NAME)).unwrap()).unwrap();
                    let key=derive_key(NEW,&metadata.salt).unwrap();
                    open_encrypted(&path.join(DATABASE_NAME),&key).unwrap().execute_batch("CREATE TRIGGER fail_password_audit BEFORE INSERT ON audit_events WHEN NEW.event='workspace_password_changed' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
                }
                if stage=="verified" && point=="metadata" {fs::write(path.join(METADATA_NAME),b"{}").unwrap();}
                if stage=="activation" && point=="activation" {fs::create_dir(root.path().join("workspace-active.json")).unwrap();}
                Ok(())
            });
            assert!(result.is_err(), "{point}");
            if point == "activation" {
                fs::remove_dir(root.path().join("workspace-active.json")).unwrap();
            }
            let mut restarted = Workspace::new(root.path().into());
            restarted.unlock_workspace(PASSWORD.into()).unwrap();
            assert_eq!(restarted.get_client(&client.id).unwrap(), client);
            assert!(restarted.unlock_workspace(NEW.into()).is_err());
        }
    }
    #[test]
    fn serialized_commands_fail_closed_while_another_process_owns_activation_lock() {
        let (root, mut w, _) = configured();
        let guard = backup::activation_lock(root.path()).unwrap();
        assert!(w.with_operation(|w| w.save_client(client_input())).is_err());
        assert!(w.change_workspace_password(input()).is_err());
        drop(guard);
        assert!(w.with_operation(|w| w.list_clients()).unwrap().is_empty());
        w.with_operation(|w| w.change_workspace_password(input()))
            .unwrap();
        w.with_operation(|w| w.unlock_workspace(NEW.into()))
            .unwrap();
    }
    #[test]
    fn interrupted_change_child() {
        let Ok(directory) = std::env::var("EISENBART_PASSWORD_CRASH_FIXTURE") else {
            return;
        };
        let phase = std::env::var("EISENBART_PASSWORD_CRASH_PHASE").unwrap();
        let mut w = Workspace::new(PathBuf::from(directory));
        w.unlock_workspace(PASSWORD.into()).unwrap();
        w.change_password_checked(input(), |stage, _| {
            if stage == phase {
                std::process::exit(73);
            }
            Ok(())
        })
        .unwrap();
        std::process::exit(74);
    }
    #[test]
    fn abrupt_process_exit_before_and_after_activation_keeps_a_complete_usable_pair() {
        for phase in ["snapshot", "verified", "activation", "committed"] {
            let (root, mut w, _) = configured();
            let client = w.save_client(client_input()).unwrap();
            w.lock_workspace();
            let outcome = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "storage::password_change::tests::interrupted_change_child",
                    "--nocapture",
                ])
                .env("EISENBART_PASSWORD_CRASH_FIXTURE", root.path())
                .env("EISENBART_PASSWORD_CRASH_PHASE", phase)
                .status()
                .unwrap();
            assert_eq!(
                outcome.code(),
                Some(if phase == "committed" { 74 } else { 73 })
            );
            let password = if phase == "committed" { NEW } else { PASSWORD };
            let mut restarted = Workspace::new(root.path().into());
            restarted.unlock_workspace(password.into()).unwrap();
            assert_eq!(restarted.get_client(&client.id).unwrap(), client);
        }
    }
    #[test]
    fn schema31_inspection_upgrade_rolls_back_preserves_all_evidence_and_is_idempotent() {
        let (root, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let note = w
            .add_client_note(&client.id, "Synthetic schema31 retained note")
            .unwrap();
        w.create_household("Synthetic existing household".into(), "2026-10".into())
            .unwrap();
        let mut doc = bank::doc("Synthetic migration checking");
        doc.retain_source = true;
        let records = bank::stage(
            &mut w,
            &client.id,
            root.path(),
            "migration.csv",
            "Date,Description,Amount,Category,Type\n2026-09-01,Synthetic wages,610,Wages,income\n",
            doc,
        );
        let report = w
            .save_assessment_record(&client.id, bank::assessment_input(&records, &client))
            .unwrap();
        fn audits(c: &Connection) -> Vec<String> {
            c.prepare("SELECT * FROM audit_events ORDER BY id")
                .unwrap()
                .query_map([], |r| {
                    let values = (0..6)
                        .map(|i| r.get::<_, rusqlite::types::Value>(i))
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    Ok(format!("{values:?}"))
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        }
        let c = &mut w.session.as_mut().unwrap().connection;
        c.execute_batch("DROP TABLE transaction_inspection_details; PRAGMA user_version=31; CREATE TRIGGER inspection_details_no_update BEFORE UPDATE ON clients BEGIN SELECT RAISE(ABORT,'Synthetic migration collision'); END;").unwrap();
        let before = contents(c);
        let audit_before = audits(c);
        let schema_before: String = c.query_row("SELECT group_concat(sql,';') FROM (SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name)", [], |r| r.get(0)).unwrap();
        // Fail after the new table was created: the entire DDL transaction must roll back.
        assert!(migrate(c, false).is_err());
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            31
        );
        let schema_after: String = c.query_row("SELECT group_concat(sql,';') FROM (SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name)", [], |r| r.get(0)).unwrap();
        assert_eq!(schema_after, schema_before);
        assert_eq!(contents(c), before);
        assert_eq!(audits(c), audit_before);
        c.execute_batch("DROP TRIGGER inspection_details_no_update;")
            .unwrap();
        migrate(c, false).unwrap();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
        let upgraded = contents(c);
        assert_eq!(
            upgraded
                .iter()
                .filter(|(name, _)| name != "transaction_inspection_details")
                .cloned()
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(audits(c), audit_before);
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM transaction_inspection_details",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        migrate(c, false).unwrap();
        assert_eq!(contents(c), upgraded);
        assert_eq!(audits(c), audit_before);
        w.lock_workspace();
        let mut reopened = Workspace::new(root.path().into());
        reopened.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(reopened.list_client_notes(&client.id).unwrap(), vec![note]);
        assert_eq!(
            serde_json::to_value(reopened.get_client_imports(&client.id).unwrap()).unwrap(),
            serde_json::to_value(records).unwrap()
        );
        assert_eq!(
            serde_json::to_value(
                reopened
                    .get_assessment_record(&client.id, &report.payload.id)
                    .unwrap()
                    .payload
            )
            .unwrap(),
            serde_json::to_value(report.payload).unwrap()
        );
        assert_eq!(reopened.list_households().unwrap().len(), 1);
        assert_eq!(contents(&reopened.session().unwrap().connection), upgraded);
    }
    #[test]
    fn schema27_audit_extension_is_atomic_and_preserves_populated_history() {
        let (_root, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let note = w
            .add_client_note(&client.id, "Synthetic migration note")
            .unwrap();
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TABLE IF EXISTS transaction_inspection_details; DROP TABLE household_events; DROP TABLE workspace_security_history;")
            .unwrap();
        let before = contents(&w.session.as_ref().unwrap().connection);
        let c = &mut w.session.as_mut().unwrap().connection;
        c.execute_batch("PRAGMA user_version=27; CREATE TABLE audit_events_v28(collision TEXT);")
            .unwrap();
        assert!(migrate(c, false).is_err());
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            27
        );
        c.execute_batch("DROP TABLE audit_events_v28;").unwrap();
        assert_eq!(contents(c), before);
        migrate(c, false).unwrap();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
        assert_eq!(
            contents(c)
                .into_iter()
                .filter(|(name, _)| name != "workspace_security_history"
                    && name != "household_events"
                    && name != "transaction_inspection_details")
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM transaction_inspection_details",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(w.list_client_notes(&client.id).unwrap(), vec![note]);
    }
}
