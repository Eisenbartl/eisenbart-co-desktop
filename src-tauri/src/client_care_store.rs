//! Authenticated append-only intake snapshots and optimistic edits.
use super::*;
use crate::client_care::{self, Data, Input, Record};
use std::collections::HashSet;

fn today() -> String {
    Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .format("%Y-%m-%d")
        .to_string()
}

fn decode(json: &str) -> Result<Data, String> {
    if json.len() > 131072 {
        return Err(STORAGE_ERROR.into());
    }
    let data: Data = serde_json::from_str(json).map_err(|_| STORAGE_ERROR.to_owned())?;
    if client_care::canonical(data.clone()).map_err(|_| STORAGE_ERROR.to_owned())? != data {
        return Err(STORAGE_ERROR.into());
    }
    Ok(data)
}

fn read(c: &Connection, info: &WorkspaceInfo, client: &str) -> Result<Record, String> {
    let row: Option<(i64,String,String)> = c.query_row(
        "SELECT revision,data_json,updated_at FROM client_care_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision DESC LIMIT 1",
        params![client,info.organization_id,info.counselor_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))
    ).optional().map_err(storage_error)?;
    match row {
        None => Ok(client_care::record(
            client,
            0,
            None,
            Data::default(),
            &today(),
        )),
        Some((revision, json, updated)) => {
            if !(1..=500).contains(&revision)
                || chrono::DateTime::parse_from_rfc3339(&updated).is_err()
            {
                return Err(STORAGE_ERROR.into());
            }
            Ok(client_care::record(
                client,
                revision,
                Some(updated),
                decode(&json)?,
                &today(),
            ))
        }
    }
}

pub(super) fn validate_records(
    c: &Connection,
    info: &WorkspaceInfo,
    client: &str,
) -> Result<(), String> {
    let mut q = c.prepare("SELECT revision,data_json,updated_at FROM client_care_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision LIMIT 501").map_err(storage_error)?;
    let rows = q
        .query_map(
            params![client, info.organization_id, info.counselor_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(storage_error)?;
    let mut old_ids = HashSet::new();
    for (i, row) in rows.enumerate() {
        let (revision, json, updated) = row.map_err(storage_error)?;
        if revision != (i + 1) as i64
            || revision > 500
            || chrono::DateTime::parse_from_rfc3339(&updated).is_err()
        {
            return Err(STORAGE_ERROR.into());
        }
        let data = decode(&json)?;
        let ids: HashSet<String> = data.follow_ups.into_iter().map(|f| f.id).collect();
        if !old_ids.is_subset(&ids) {
            return Err(STORAGE_ERROR.into());
        }
        old_ids = ids;
    }
    Ok(())
}

impl Workspace {
    pub fn get_follow_up_overview(
        &mut self,
        input: crate::follow_up_overview::Input,
    ) -> Result<crate::follow_up_overview::Overview, String> {
        use crate::follow_up_overview::{self, Item};
        let s = self.session()?;
        let input = follow_up_overview::clean(input)?;
        // One SELECT holds a consistent SQLite read snapshot for client identity
        // and latest care data, including concurrent writes from other windows.
        let mut q = s.connection.prepare(
            "SELECT c.id,c.first_name,c.last_name,c.status,h.revision,h.data_json,h.updated_at
             FROM clients c JOIN client_care_snapshots h ON h.client_id=c.id
                AND h.organization_id=c.organization_id AND h.counselor_id=c.counselor_id
             WHERE c.organization_id=?1 AND c.counselor_id=?2 AND (?3 OR c.status!='Archived')
                AND h.revision=(SELECT MAX(v.revision) FROM client_care_snapshots v
                    WHERE v.client_id=c.id AND v.organization_id=c.organization_id AND v.counselor_id=c.counselor_id)
             ORDER BY c.id LIMIT 1001"
        ).map_err(storage_error)?;
        let mut rows = q
            .query(params![
                s.info.organization_id,
                s.info.counselor_id,
                input.include_archived
            ])
            .map_err(storage_error)?;
        let mut items = Vec::new();
        let mut clients = 0;
        while let Some(r) = rows.next().map_err(storage_error)? {
            clients += 1;
            if clients > 1000 {
                return Err("The follow-up overview supports up to 1,000 included clients with saved intake records. Review follow-ups in individual client records.".into());
            }
            let id: String = r.get(0).map_err(storage_error)?;
            let first: String = r.get(1).map_err(storage_error)?;
            let last: String = r.get(2).map_err(storage_error)?;
            let status: String = r.get(3).map_err(storage_error)?;
            let revision: i64 = r.get(4).map_err(storage_error)?;
            let json: String = r.get(5).map_err(storage_error)?;
            let updated: String = r.get(6).map_err(storage_error)?;
            if !(1..=500).contains(&revision)
                || chrono::DateTime::parse_from_rfc3339(&updated).is_err()
            {
                return Err(STORAGE_ERROR.into());
            }
            for f in decode(&json)?.follow_ups {
                items.push(Item {
                    client_id: id.clone(),
                    client_name: format!("{first} {last}"),
                    client_status: status.clone(),
                    follow_up_id: f.id,
                    kind: f.kind,
                    title: f.title,
                    due_date: f.due_date,
                    time: f.time,
                    status: f.status,
                    care_revision: revision,
                });
            }
        }
        Ok(follow_up_overview::build(input, items, &today(), &s.info))
    }

    pub fn get_client_care(&mut self, client: &str) -> Result<Record, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        read(&s.connection, &s.info, client)
    }
    pub fn save_client_care(&mut self, client: &str, mut input: Input) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let current = read(&tx, &s.info, client)?;
        if input.expected_revision != current.revision {
            return Err("Intake or follow-ups changed in another window. Your draft is retained; discard and reload before editing again.".into());
        }
        if current.revision >= 500 {
            return Err("This client reached the current 500-version intake history limit.".into());
        }
        let old: HashSet<&str> = current
            .data
            .follow_ups
            .iter()
            .map(|f| f.id.as_str())
            .collect();
        for f in &mut input.data.follow_ups {
            if f.id.is_empty() {
                f.id = Uuid::new_v4().to_string();
            } else if !old.contains(f.id.as_str()) {
                return Err("A follow-up does not belong to this saved client record.".into());
            }
        }
        let data = client_care::canonical(input.data)?;
        let ids: HashSet<&str> = data.follow_ups.iter().map(|f| f.id.as_str()).collect();
        if !old.is_subset(&ids) {
            return Err(
                "Keep saved follow-ups in history; mark an item cancelled instead of deleting it."
                    .into(),
            );
        }
        let revision = current.revision + 1;
        let updated = timestamp();
        let json = serde_json::to_string(&data).map_err(|_| STORAGE_ERROR.to_owned())?;
        tx.execute("INSERT INTO client_care_snapshots(client_id,organization_id,counselor_id,revision,data_json,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",params![client,s.info.organization_id,s.info.counselor_id,revision,json,updated]).map_err(storage_error)?;
        audit(&tx, &s.info, "client_care_saved", Some(client))?;
        tx.commit().map_err(storage_error)?;
        Ok(client_care::record(
            client,
            revision,
            Some(updated),
            data,
            &today(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_care::FollowUp;
    use crate::storage::tests::{client_input, configured, PASSWORD};

    fn draft() -> Data {
        let mut d = Data::default();
        d.intake.household = "Two fictional adults; one dependent".into();
        d.intake.employment = "employed".into();
        d.intake.employer = " Fictional Example Employer ".into();
        d.follow_ups.push(FollowUp {
            id: "".into(),
            kind: "appointment".into(),
            title: "Review fictional statements".into(),
            due_date: "2026-11-01".into(),
            time: "09:30".into(),
            status: "open".into(),
            notes: "Bring the synthetic checking statement".into(),
        });
        d
    }
    fn input(revision: i64, data: Data) -> Input {
        Input {
            expected_revision: revision,
            data,
        }
    }

    #[test]
    fn overview_uses_latest_scoped_records_excludes_archived_and_never_writes() {
        use crate::follow_up_overview::Input as Query;
        let query = || Query {
            view: "open".into(),
            kind: "all".into(),
            include_archived: false,
            search: "".into(),
            page: 0,
        };
        let (_dir, mut w, info) = configured();
        let a = w.save_client(client_input()).unwrap();
        let mut archived = client_input();
        archived.status = "Archived".into();
        let b = w.save_client(archived).unwrap();
        let first = w.save_client_care(&a.id, input(0, draft())).unwrap();
        let mut next = first.data.clone();
        next.follow_ups[0].status = "completed".into();
        next.follow_ups.push(crate::client_care::FollowUp {
            id: "".into(),
            kind: "task".into(),
            title: "Overdue fictional review".into(),
            due_date: "1900-01-01".into(),
            time: "".into(),
            status: "open".into(),
            notes: "Private notes never enter overview rows".into(),
        });
        w.save_client_care(&a.id, input(1, next)).unwrap();
        w.save_client_care(&b.id, input(0, draft())).unwrap();
        {
            let c = &w.session().unwrap().connection;
            c.execute(
                "INSERT INTO organizations(id,name) VALUES('foreign-org','Foreign organization')",
                [],
            )
            .unwrap();
            c.execute("INSERT INTO counselors(id,organization_id,name) VALUES('foreign-counselor','foreign-org','Other counselor'),('same-org-counselor',?1,'Same organization other counselor')",[&info.organization_id]).unwrap();
            let json = serde_json::to_string(&first.data).unwrap();
            for (id, org, counselor) in [
                ("foreign-client", "foreign-org", "foreign-counselor"),
                (
                    "other-counselor-client",
                    info.organization_id.as_str(),
                    "same-org-counselor",
                ),
            ] {
                c.execute("INSERT INTO clients(id,organization_id,counselor_id,first_name,last_name,household_size,status,created_at,updated_at) VALUES(?1,?2,?3,'Foreign','Private Client',1,'Planning',?4,?4)",params![id,org,counselor,timestamp()]).unwrap();
                c.execute("INSERT INTO client_care_snapshots(client_id,organization_id,counselor_id,revision,data_json,updated_at) VALUES(?1,?2,?3,1,?4,?5)",params![id,org,counselor,json,timestamp()]).unwrap();
            }
        }
        let before: i64 = w
            .session()
            .unwrap()
            .connection
            .query_row("SELECT count(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        let r = w.get_follow_up_overview(query()).unwrap();
        assert_eq!((r.overdue, r.upcoming, r.closed, r.total), (1, 0, 1, 1));
        assert_eq!(r.items[0].client_id, a.id);
        assert_eq!(r.items[0].care_revision, 2);
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("Private notes never"));
        let r = w
            .get_follow_up_overview(Query {
                include_archived: true,
                ..query()
            })
            .unwrap();
        assert_eq!((r.total, r.upcoming), (2, 1));
        assert!(r
            .items
            .iter()
            .any(|f| f.client_id == b.id && f.client_status == "Archived"));
        assert_eq!(
            w.get_follow_up_overview(Query {
                search: "Foreign".into(),
                include_archived: true,
                ..query()
            })
            .unwrap()
            .total,
            0
        );
        let r = w
            .get_follow_up_overview(Query {
                view: "closed".into(),
                ..query()
            })
            .unwrap();
        assert_eq!(r.items[0].status, "completed");
        let after: i64 = w
            .session()
            .unwrap()
            .connection
            .query_row("SELECT count(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER client_care_no_update;")
            .unwrap();
        w.session()
            .unwrap()
            .connection
            .execute(
                "UPDATE client_care_snapshots SET data_json='{}' WHERE client_id=?1 AND revision=2",
                [&a.id],
            )
            .unwrap();
        assert!(w.get_follow_up_overview(query()).is_err());
        w.lock_workspace();
        assert!(w
            .get_follow_up_overview(query())
            .err()
            .unwrap()
            .contains("locked"));
    }

    #[test]
    fn care_scope_conflicts_history_reopen_and_encrypted_recovery() {
        let (dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        let b = w.save_client(client_input()).unwrap();
        assert_eq!(w.get_client_care(&a.id).unwrap().revision, 0);
        assert!(w.get_client_care("foreign").is_err());
        assert!(w.save_client_care("foreign", input(0, draft())).is_err());
        let first = w.save_client_care(&a.id, input(0, draft())).unwrap();
        assert_eq!(first.data.intake.employer, "Fictional Example Employer");
        assert!(Uuid::parse_str(&first.data.follow_ups[0].id).is_ok());
        assert!(w
            .save_client_care(&b.id, input(0, first.data.clone()))
            .is_err());
        assert!(w
            .save_client_care(&a.id, input(0, draft()))
            .err()
            .unwrap()
            .contains("another window"));
        assert!(w
            .save_client_care(&a.id, input(1, Data::default()))
            .err()
            .unwrap()
            .contains("cancelled"));
        let mut other_window = Workspace::new(dir.path().to_path_buf());
        other_window.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(other_window.get_client_care(&a.id).unwrap().revision, 1);
        let mut next = first.data.clone();
        next.follow_ups[0].status = "completed".into();
        next.follow_ups[0].notes = "Fictional documents reviewed".into();
        let saved = w.save_client_care(&a.id, input(1, next)).unwrap();
        assert_eq!(saved.revision, 2);
        assert!(other_window
            .save_client_care(&a.id, input(1, first.data.clone()))
            .err()
            .unwrap()
            .contains("another window"));
        drop(other_window);
        assert!(w.get_client_care(&b.id).unwrap().data.follow_ups.is_empty());
        {
            let s = w.session().unwrap();
            validate_records(&s.connection, &s.info, &a.id).unwrap();
            let old: String = s
                .connection
                .query_row(
                    "SELECT data_json FROM client_care_snapshots WHERE client_id=?1 AND revision=1",
                    [&a.id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(decode(&old).unwrap(), first.data);
            assert!(s
                .connection
                .execute(
                    "DELETE FROM client_care_snapshots WHERE client_id=?1",
                    [&a.id]
                )
                .is_err());
            let n: i64 = s
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM audit_events WHERE event='client_care_saved'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 2);
        }
        w.lock_workspace();
        assert!(w.get_client_care(&a.id).err().unwrap().contains("locked"));
        assert!(w
            .save_client_care(&a.id, input(2, draft()))
            .err()
            .unwrap()
            .contains("locked"));
        drop(w);
        let mut w = Workspace::new(dir.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(w.get_client_care(&a.id).unwrap().data, saved.data);
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let mut changed = saved.data.clone();
        changed.intake.address = "Later fictional address".into();
        w.save_client_care(&a.id, input(2, changed)).unwrap();
        w.lock_workspace();
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_client_care(&a.id).unwrap();
        assert_eq!(restored.revision, 2);
        assert_eq!(restored.data, saved.data);
    }

    #[test]
    fn migration_and_audit_failures_roll_back_without_losing_records() {
        let (_dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        w.add_client_note(&a.id, "Original fictional note").unwrap();
        let s = w.session().unwrap();
        s.connection.execute_batch("DROP TABLE IF EXISTS firm_branding_history; DROP TABLE statement_account_history; DROP TRIGGER statement_accounts_no_update; DROP TRIGGER statement_accounts_no_delete; DROP TABLE document_collection_snapshots; DROP TABLE client_care_snapshots; PRAGMA user_version=16; CREATE TABLE audit_events_v17(collision TEXT);").unwrap();
        assert!(migrate(&mut s.connection, false).is_err());
        let version: i64 = s
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, 16);
        let tables: i64 = s
            .connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='client_care_snapshots'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
        s.connection
            .execute_batch("DROP TABLE audit_events_v17;")
            .unwrap();
        migrate(&mut s.connection, false).unwrap();
        assert_eq!(
            w.list_client_notes(&a.id).unwrap()[0].body,
            "Original fictional note"
        );
        assert_eq!(w.get_client_care(&a.id).unwrap().revision, 0);
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER reject_care_audit BEFORE INSERT ON audit_events WHEN NEW.event='client_care_saved' BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;").unwrap();
        assert!(w.save_client_care(&a.id, input(0, draft())).is_err());
        assert_eq!(w.get_client_care(&a.id).unwrap().revision, 0);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_care_audit;")
            .unwrap();
        assert_eq!(
            w.save_client_care(&a.id, input(0, draft()))
                .unwrap()
                .revision,
            1
        );
    }

    #[test]
    fn recovery_rejects_corrupt_older_intake_history_before_activation() {
        let (_dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        let first = w.save_client_care(&a.id, input(0, draft())).unwrap();
        w.save_client_care(&a.id, input(1, first.data)).unwrap();
        w.session().unwrap().connection.execute_batch("DROP TRIGGER client_care_no_update; UPDATE client_care_snapshots SET data_json='{}' WHERE revision=1;").unwrap();
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        assert!(!w.data_dir.join("workspace-active.json").exists());
    }
}
