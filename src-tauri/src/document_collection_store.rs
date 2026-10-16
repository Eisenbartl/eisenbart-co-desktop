//! Scoped append-only collection checklists and authenticated source links.
use super::*;
use crate::document_collection::{self as collection, Input, Item, Record, Source, Version};
use std::collections::HashSet;
fn choices(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(Vec<crate::imports::Account>, Vec<Source>), String> {
    let mut q=c.prepare("SELECT id,label,kind FROM statement_accounts WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY label,id LIMIT 1001").map_err(storage_error)?;
    let accounts = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok(crate::imports::Account {
                id: r.get(0)?,
                label: r.get(1)?,
                kind: r.get(2)?,
            })
        })
        .map_err(storage_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(storage_error)?;
    let mut q=c.prepare("SELECT id,account_id,file_name FROM import_documents WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY imported_at DESC,id LIMIT 1001").map_err(storage_error)?;
    let sources = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok(Source {
                id: r.get(0)?,
                account_id: r.get(1)?,
                file_name: r.get(2)?,
            })
        })
        .map_err(storage_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(storage_error)?;
    if accounts.len() > 1000 || sources.len() > 1000 {
        return Err("Checklist source selection supports up to 1,000 statement accounts and 1,000 imported documents per client.".into());
    }
    Ok((accounts, sources))
}
fn links(
    items: &[Item],
    accounts: &[crate::imports::Account],
    sources: &[Source],
) -> Result<(), String> {
    for i in items {
        if i.account_id
            .as_ref()
            .is_some_and(|id| !accounts.iter().any(|a| &a.id == id))
        {
            return Err("A checklist account does not belong to this client.".into());
        }
        if let Some(id) = &i.source_document_id {
            let source = sources
                .iter()
                .find(|s| &s.id == id)
                .ok_or("A linked source document does not belong to this client.")?;
            if i.account_id
                .as_ref()
                .is_some_and(|a| a != &source.account_id)
            {
                return Err("The linked source belongs to a different account. Choose its account or leave the optional account blank.".into());
            }
        }
    }
    Ok(())
}
fn decode(json: &str) -> Result<Vec<Item>, String> {
    if json.len() > 131072 {
        return Err(STORAGE_ERROR.into());
    }
    let data: Vec<Item> = serde_json::from_str(json).map_err(|_| STORAGE_ERROR.to_owned())?;
    if collection::canonical(data.clone()).map_err(|_| STORAGE_ERROR.to_owned())? != data {
        return Err(STORAGE_ERROR.into());
    }
    Ok(data)
}
fn history(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<Vec<Version>, String> {
    let mut q=c.prepare("SELECT revision,updated_at FROM document_collection_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision LIMIT 501").map_err(storage_error)?;
    let rows = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok(Version {
                revision: r.get(0)?,
                updated_at: r.get(1)?,
            })
        })
        .map_err(storage_error)?;
    let mut result = Vec::new();
    for row in rows {
        let v = row.map_err(storage_error)?;
        if v.revision != result.len() as i64 + 1
            || v.revision > 500
            || chrono::DateTime::parse_from_rfc3339(&v.updated_at).is_err()
        {
            return Err(STORAGE_ERROR.into());
        }
        result.push(v);
    }
    Ok(result)
}
fn read(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    requested: Option<i64>,
) -> Result<Record, String> {
    let history = history(c, w, client)?;
    let latest_revision = history.len() as i64;
    let revision = requested.unwrap_or(latest_revision);
    if revision < 0 || revision > latest_revision || (revision == 0 && latest_revision > 0) {
        return Err("The selected checklist version is unavailable.".into());
    }
    let (accounts, sources) = choices(c, w, client)?;
    let (items, updated_at) = if revision == 0 {
        (vec![], None)
    } else {
        let json:String=c.query_row("SELECT data_json FROM document_collection_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 AND revision=?4",params![client,w.organization_id,w.counselor_id,revision],|r|r.get(0)).map_err(storage_error)?;
        (
            decode(&json)?,
            Some(history[revision as usize - 1].updated_at.clone()),
        )
    };
    links(&items, &accounts, &sources).map_err(|_| STORAGE_ERROR.to_owned())?;
    let missing = items
        .iter()
        .filter(|i| !i.archived && i.status == "missing")
        .count();
    let received = items
        .iter()
        .filter(|i| !i.archived && i.status == "received")
        .count();
    let archived = items.iter().filter(|i| i.archived).count();
    Ok(Record {
        client_id: client.into(),
        revision,
        latest_revision,
        updated_at,
        review_date: Utc::now()
            .with_timezone(&chrono_tz::America::Chicago)
            .format("%Y-%m-%d")
            .to_string(),
        items,
        history,
        accounts,
        sources,
        missing,
        received,
        archived,
    })
}
pub(super) fn validate_records(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(), String> {
    let versions = history(c, w, client)?;
    let (accounts, sources) = choices(c, w, client)?;
    let mut q=c.prepare("SELECT data_json FROM document_collection_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision LIMIT 501").map_err(storage_error)?;
    let rows = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            r.get::<_, String>(0)
        })
        .map_err(storage_error)?;
    let mut old = HashSet::new();
    let mut count = 0;
    for row in rows {
        let items = decode(&row.map_err(storage_error)?)?;
        links(&items, &accounts, &sources).map_err(|_| STORAGE_ERROR.to_owned())?;
        let ids: HashSet<String> = items.into_iter().map(|i| i.id).collect();
        if !old.is_subset(&ids) {
            return Err(STORAGE_ERROR.into());
        }
        old = ids;
        count += 1;
    }
    if count != versions.len() {
        return Err(STORAGE_ERROR.into());
    }
    Ok(())
}
impl Workspace {
    pub fn get_document_collection(
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
    pub fn save_document_collection(
        &mut self,
        client: &str,
        mut input: Input,
    ) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let current = read(&tx, &s.info, client, None)?;
        if input.expected_revision != current.revision {
            return Err("The checklist changed in another window. Your draft is retained; discard and reload before saving.".into());
        }
        if current.revision >= 500 {
            return Err(
                "This client reached the current 500-version checklist history limit.".into(),
            );
        }
        let old: HashSet<&str> = current.items.iter().map(|i| i.id.as_str()).collect();
        for i in &mut input.items {
            if i.id.is_empty() {
                i.id = Uuid::new_v4().to_string();
            } else if !old.contains(i.id.as_str()) {
                return Err("A request does not belong to this saved client checklist.".into());
            }
        }
        let items = collection::canonical(input.items)?;
        let ids: HashSet<&str> = items.iter().map(|i| i.id.as_str()).collect();
        if !old.is_subset(&ids) {
            return Err("Archive saved requests instead of deleting their history.".into());
        }
        links(&items, &current.accounts, &current.sources)?;
        let json = serde_json::to_string(&items).map_err(|_| STORAGE_ERROR.to_owned())?;
        tx.execute("INSERT INTO document_collection_snapshots(client_id,organization_id,counselor_id,revision,data_json,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",params![client,s.info.organization_id,s.info.counselor_id,current.revision+1,json,timestamp()]).map_err(storage_error)?;
        audit(&tx, &s.info, "document_collection_saved", Some(client))?;
        let result = read(&tx, &s.info, client, None)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_collection::tests::item;
    use crate::storage::tests::{client_input, configured, PASSWORD};
    fn input(rev: i64, items: Vec<Item>) -> Input {
        Input {
            expected_revision: rev,
            items,
        }
    }
    #[test]
    fn collection_crud_scope_stale_history_audit_rollback_and_recovery() {
        let (dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        let b = w.save_client(client_input()).unwrap();
        assert_eq!(w.get_document_collection(&a.id, None).unwrap().revision, 0);
        assert!(w.get_document_collection("foreign", None).is_err());
        assert!(w
            .save_document_collection("foreign", input(0, vec![]))
            .is_err());
        let mut first = item();
        first.id.clear();
        let r = w
            .save_document_collection(&a.id, input(0, vec![first]))
            .unwrap();
        let original = r.items.clone();
        assert_eq!((r.missing, r.received, r.archived), (1, 0, 0));
        assert!(w
            .save_document_collection(&b.id, input(0, r.items.clone()))
            .is_err());
        assert!(w
            .save_document_collection(&a.id, input(0, r.items.clone()))
            .is_err());
        assert!(w.save_document_collection(&a.id, input(1, vec![])).is_err());
        let mut items = r.items;
        items[0].status = "received".into();
        items[0].received_date = Some("2026-10-14".into());
        items[0].notes = "Verified receipt only; not financial approval".into();
        let r = w.save_document_collection(&a.id, input(1, items)).unwrap();
        assert_eq!((r.missing, r.received), (0, 1));
        let mut archived = r.items;
        archived[0].archived = true;
        let r = w
            .save_document_collection(&a.id, input(2, archived))
            .unwrap();
        assert_eq!((r.received, r.archived), (0, 1));
        assert_eq!(
            w.get_document_collection(&a.id, Some(1)).unwrap().items,
            original
        );
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER reject_collection_audit BEFORE INSERT ON audit_events WHEN NEW.event='document_collection_saved' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
        let mut active = r.items;
        active[0].archived = false;
        assert!(w
            .save_document_collection(&a.id, input(3, active.clone()))
            .is_err());
        assert_eq!(w.get_document_collection(&a.id, None).unwrap().revision, 3);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_collection_audit;")
            .unwrap();
        w.save_document_collection(&a.id, input(3, active)).unwrap();
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.get_document_collection(&a.id, None).is_err());
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        drop(w);
        let mut w = Workspace::new(dir.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let r = w.get_document_collection(&a.id, None).unwrap();
        assert_eq!(r.revision, 4);
        assert_eq!((r.received, r.archived), (1, 0));
        assert_eq!(r.history.len(), 4);
        assert_eq!(
            w.get_document_collection(&a.id, Some(1)).unwrap().items,
            original
        );
    }
    #[test]
    fn collection_source_links_are_scoped_and_do_not_approve_transactions() {
        use crate::imports::{DocumentInput, StageInput};
        let (dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        let b = w.save_client(client_input()).unwrap();
        let mut stage = |client: &str, label: &str| {
            let path = dir.path().join(format!("{label}.csv"));
            fs::write(&path,"Date,Description,Amount,Category,Type\n2026-09-01,Fictional wages,2000,Wages,income\n").unwrap();
            let preview = w
                .inspect_client_csv(client.into(), path.to_string_lossy().into())
                .unwrap();
            let rev = w.get_client_imports(client).unwrap().revision;
            w.stage_client_csv(
                client,
                StageInput {
                    expected_revision: rev,
                    token: preview.token,
                    document: DocumentInput {
                        balance_order: None,
                        pdf_selection: None,
                        pdf: None,
                        account_id: None,
                        account_label: label.into(),
                        account_kind: "checking".into(),
                        period_start: None,
                        period_end: None,
                        opening_balance: None,
                        closing_balance: None,
                        retain_source: false,
                        mapping: crate::imports::tests::mapping(),
                    },
                },
            )
            .unwrap()
        };
        let source = stage(&a.id, "first");
        let other = stage(&a.id, "second");
        let foreign = stage(&b.id, "foreign");
        let mut i = item();
        i.id.clear();
        i.status = "received".into();
        i.received_date = Some("2026-10-14".into());
        i.account_id = Some(source.accounts[0].id.clone());
        i.source_document_id = Some(source.documents[0].id.clone());
        let saved = w
            .save_document_collection(&a.id, input(0, vec![i]))
            .unwrap();
        assert_eq!(saved.sources.len(), 2);
        let before = w.get_client_imports(&a.id).unwrap();
        assert_eq!(before.summary.approved, 0);
        let mut mismatch = saved.items.clone();
        mismatch[0].account_id = Some(
            other
                .accounts
                .iter()
                .find(|a| a.id != source.accounts[0].id)
                .unwrap()
                .id
                .clone(),
        );
        assert!(w
            .save_document_collection(&a.id, input(1, mismatch))
            .is_err());
        let mut bad = saved.items.clone();
        bad[0].account_id = None;
        bad[0].source_document_id = Some(foreign.documents[0].id.clone());
        assert!(w.save_document_collection(&a.id, input(1, bad)).is_err());
        let mut bad = saved.items;
        bad[0].source_document_id = None;
        bad[0].account_id = Some(foreign.accounts[0].id.clone());
        assert!(w.save_document_collection(&a.id, input(1, bad)).is_err());
        let after = w.get_client_imports(&a.id).unwrap();
        assert_eq!(before.revision, after.revision);
        assert_eq!(after.summary.approved, 0);
        assert_eq!(w.get_document_collection(&a.id, None).unwrap().revision, 1);
    }
    #[test]
    fn collection_migration_rolls_back_and_corrupt_history_blocks_restore() {
        let (_dir, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        {
            let s = w.session().unwrap();
            s.connection.execute_batch("DROP TABLE IF EXISTS workspace_security_history; DROP TABLE IF EXISTS note_amendments; DROP INDEX IF EXISTS note_amendment_scope; DROP TABLE IF EXISTS category_rule_applications; DROP TABLE IF EXISTS category_rule_versions; DROP INDEX IF EXISTS review_history_rule_scope; DROP TABLE IF EXISTS card_facts_history; DROP TABLE IF EXISTS card_review_history; DROP TABLE IF EXISTS card_decisions; DROP TABLE IF EXISTS card_statements; DROP TABLE IF EXISTS final_report_approvals; DROP TABLE IF EXISTS coverage_review_history; DROP TABLE IF EXISTS firm_branding_history; DROP TABLE statement_account_history; DROP TRIGGER statement_accounts_no_update; DROP TRIGGER statement_accounts_no_delete; DROP TABLE document_collection_snapshots; PRAGMA user_version=17; CREATE TABLE document_collection_snapshots(blocked TEXT);").unwrap();
            assert!(migrate(&mut s.connection, false).is_err());
            assert_eq!(
                s.connection
                    .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                    .unwrap(),
                17
            );
            s.connection
                .execute_batch("DROP TABLE document_collection_snapshots;")
                .unwrap();
            migrate(&mut s.connection, false).unwrap();
            assert_eq!(
                s.connection
                    .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                    .unwrap(),
                SCHEMA_VERSION
            );
        }
        let mut i = item();
        i.id.clear();
        let first = w
            .save_document_collection(&a.id, input(0, vec![i]))
            .unwrap();
        let mut items = first.items;
        items[0].archived = true;
        w.save_document_collection(&a.id, input(1, items)).unwrap();
        w.session().unwrap().connection.execute_batch("DROP TRIGGER document_collection_no_update; UPDATE document_collection_snapshots SET data_json='[]' WHERE revision=2;").unwrap();
        {
            let session = w.session().unwrap();
            assert!(validate_records(&session.connection, &session.info, &a.id).is_err());
        }
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        assert!(!w.data_dir.join("workspace-active.json").exists());
    }
}
