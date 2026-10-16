use super::*;
use crate::classification;
impl Workspace {
    fn classification_snapshot(
        &mut self,
        client: &str,
    ) -> Result<crate::imports::Snapshot, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let snapshot = imports_store::read_imports(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(snapshot)
    }
    pub fn query_category_suggestions(
        &mut self,
        client: &str,
        input: classification::Query,
    ) -> Result<classification::Page, String> {
        classification::query(&self.classification_snapshot(client)?, input)
    }
    pub fn prepare_category_draft(
        &mut self,
        client: &str,
        input: classification::DraftInput,
    ) -> Result<classification::Draft, String> {
        classification::prepare(&self.classification_snapshot(client)?, input)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    use crate::{
        classification::{self, DraftInput, Query},
        imports::{ReviewInput, Status},
    };
    fn query(offset: usize, snapshot: Option<String>) -> Query {
        Query {
            text: String::new(),
            offset,
            expected_snapshot: snapshot,
        }
    }
    fn input(page: &classification::Page, id: &str) -> DraftInput {
        DraftInput {
            expected_revision: page.revision,
            expected_snapshot: page.snapshot.clone(),
            record_id: id.into(),
        }
    }
    fn copy(s: &crate::imports::Snapshot) -> crate::imports::Snapshot {
        serde_json::from_value(serde_json::to_value(s).unwrap()).unwrap()
    }
    #[test]
    fn categories_are_advisory_scoped_stale_bounded_and_use_existing_atomic_decisions() {
        let (root, mut w, _info) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let mut csv="Date,Description,Amount,Category,Type\n2026-09-01,Fictional Market,-11.01,Food,expense\n2026-09-02,Fictional Market,-12.02,Food,expense\n2026-09-03,Fictional Market,-13.03,,expense\n".to_owned();
        for n in 0..30 {
            csv.push_str(&format!(
                "2026-09-04,Fictional Market,-{}.00,,expense\n",
                20 + n
            ));
        }
        csv.push_str("2026-09-05,<script>literal % _ apostrophe',-1.23,,expense\n");
        let staged = bank::stage(
            &mut w,
            &c.id,
            root.path(),
            "category-fictional.csv",
            &csv,
            bank::doc("Fictional category checking"),
        );
        let approved_ids: Vec<_> = staged
            .records
            .iter()
            .take(2)
            .map(|r| r.id.clone())
            .collect();
        for id in &approved_ids {
            let s = w.get_client_imports(&c.id).unwrap();
            let r = s.records.iter().find(|r| &r.id == id).unwrap();
            w.review_transaction(
                &c.id,
                ReviewInput {
                    expected_revision: s.revision,
                    record_id: id.clone(),
                    status: Status::Approved,
                    fields: r.fields.clone(),
                    reason: "Synthetic verified source".into(),
                    acknowledge_warnings: true,
                },
            )
            .unwrap();
        }
        let s = w.get_client_imports(&c.id).unwrap();
        let id = s
            .records
            .iter()
            .find(|r| r.fields.amount.as_deref() == Some("-13.03"))
            .unwrap()
            .id
            .clone();
        let before = serde_json::to_vec(&s).unwrap();
        let changes = w.session().unwrap().connection.total_changes();
        let p = w.query_category_suggestions(&c.id, query(0, None)).unwrap();
        assert_eq!(p.total, 32);
        assert_eq!(p.candidates.len(), 25);
        let second = w
            .query_category_suggestions(&c.id, query(25, Some(p.snapshot.clone())))
            .unwrap();
        assert_eq!(second.candidates.len(), 7);
        assert!(!p
            .candidates
            .iter()
            .any(|a| second.candidates.iter().any(|b| a.record_id == b.record_id)));
        assert!(w
            .query_category_suggestions(&c.id, query(25, None))
            .is_err());
        let literal = w
            .query_category_suggestions(
                &c.id,
                Query {
                    text: "% _ apostrophe'".into(),
                    ..query(0, None)
                },
            )
            .unwrap();
        assert_eq!(literal.total, 1);
        assert!(literal.candidates[0].category.is_none());
        assert!(w
            .query_category_suggestions(
                &c.id,
                Query {
                    text: "x".repeat(101),
                    ..query(0, None)
                }
            )
            .is_err());
        let d = w.prepare_category_draft(&c.id, input(&p, &id)).unwrap();
        assert_eq!(d.category, "Food");
        assert_eq!(d.revision, s.revision);
        let all = classification::query(
            &s,
            Query {
                text: "Fictional Market".into(),
                ..query(0, None)
            },
        )
        .unwrap();
        assert!(all
            .candidates
            .iter()
            .all(|c| c.category.as_deref() == Some("Food") && c.evidence.len() == 2));
        assert!(all.candidates[0]
            .evidence
            .iter()
            .all(|e| !e.approval_id.is_empty()));
        assert_eq!(w.session().unwrap().connection.total_changes(), changes);
        assert_eq!(
            serde_json::to_vec(&w.get_client_imports(&c.id).unwrap()).unwrap(),
            before
        );
        assert!(w.prepare_category_draft(&other.id, input(&p, &id)).is_err());
        assert!(w
            .query_category_suggestions(&other.id, query(25, Some(p.snapshot.clone())))
            .is_err());
        assert!(w
            .prepare_category_draft(&c.id, input(&p, "foreign-record"))
            .is_err());
        // Conflicts, missing facts, duplicate evidence, insufficient history and
        // large groups stay unresolved. No merchant suffixes are stripped.
        for mode in [
            "conflict",
            "reversal",
            "duplicate",
            "same-date",
            "large",
            "unknown",
            "suffix",
            "different-account",
            "unclassified",
            "provided",
        ] {
            let mut v = copy(&s);
            let target = v.records.iter_mut().find(|r| r.id == id).unwrap();
            match mode {
                "unknown" => target.fields.amount = None,
                "suffix" => target.fields.description.push_str(" #123"),
                "different-account" => target.account_id = "other".into(),
                "unclassified" => target.fields.kind = crate::imports::Kind::Unclassified,
                "provided" => target.fields.category = "Manual category".into(),
                _ => (),
            }
            let g = &mut v.merchant_groups[0];
            match mode {
                "conflict" => g.evidence[0].fields.category = "Leisure".into(),
                "reversal" => {
                    g.evidence.pop();
                }
                "duplicate" => g.evidence[0]
                    .duplicate_ids
                    .push("possible-duplicate".into()),
                "same-date" => g.evidence[0].fields.date = g.evidence[1].fields.date.clone(),
                "large" => {
                    let e = g.evidence[0].clone();
                    g.evidence.extend(vec![e; 4]);
                }
                _ => (),
            }
            assert!(
                classification::prepare(
                    &v,
                    input(&classification::query(&v, query(0, None)).unwrap(), &id)
                )
                .is_err(),
                "{mode}"
            );
        }
        let mut normalized = copy(&s);
        normalized
            .records
            .iter_mut()
            .find(|r| r.id == id)
            .unwrap()
            .fields
            .description = "FICTIONAL   MARKET".into();
        let p2 = classification::query(&normalized, query(0, None)).unwrap();
        assert_eq!(
            classification::prepare(&normalized, input(&p2, &id))
                .unwrap()
                .category,
            "Food"
        );
        // The prepared value only fills a draft. Saving remains the old audited,
        // explicit decision API and may keep the row Pending.
        let mut fields = s
            .records
            .iter()
            .find(|r| r.id == id)
            .unwrap()
            .fields
            .clone();
        fields.category = d.category;
        w.session().unwrap().connection.execute_batch("CREATE TEMP TRIGGER synthetic_category_audit_failure BEFORE INSERT ON audit_events WHEN NEW.event='transaction_reviewed' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
        let save = |fields: crate::imports::Fields| ReviewInput {
            expected_revision: s.revision,
            record_id: id.clone(),
            status: Status::Pending,
            fields,
            reason: "Synthetic category checked against original source".into(),
            acknowledge_warnings: false,
        };
        assert!(w.review_transaction(&c.id, save(fields.clone())).is_err());
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, s.revision);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER synthetic_category_audit_failure;")
            .unwrap();
        let saved = w.review_transaction(&c.id, save(fields)).unwrap();
        assert_eq!(saved.summary.spending, s.summary.spending);
        assert_eq!(
            saved.records.iter().find(|r| r.id == id).unwrap().status,
            Status::Pending
        );
        assert_eq!(saved.history.len(), s.history.len() + 1);
        assert!(w
            .prepare_category_draft(&c.id, input(&p, &id))
            .unwrap_err()
            .contains("changed"));
        assert!(w
            .query_category_suggestions(&c.id, query(25, Some(p.snapshot.clone())))
            .is_err());
        // Real reversal removes current approval evidence; old history remains.
        let r = saved
            .records
            .iter()
            .find(|r| r.id == approved_ids[0])
            .unwrap();
        let reversed = w
            .review_transaction(
                &c.id,
                ReviewInput {
                    expected_revision: saved.revision,
                    record_id: r.id.clone(),
                    status: Status::Rejected,
                    fields: r.fields.clone(),
                    reason: "Synthetic reversed source".into(),
                    acknowledge_warnings: true,
                },
            )
            .unwrap();
        assert!(w
            .query_category_suggestions(&c.id, query(0, None))
            .unwrap()
            .candidates
            .iter()
            .all(|r| r.category.is_none()));
        let backups = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.query_category_suggestions(&c.id, query(0, None)).is_err());
        assert!(w.prepare_category_draft(&c.id, input(&p, &id)).is_err());
        drop(w);
        let mut w = Workspace::new(root.path().into());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let current = w.query_category_suggestions(&c.id, query(0, None)).unwrap();
        assert!(current.candidates.iter().all(|r| r.category.is_none()));
        w.lock_workspace();
        let restore = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(restore.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            w.query_category_suggestions(&c.id, query(0, None))
                .unwrap()
                .snapshot,
            current.snapshot
        );
        assert_eq!(
            w.get_client_imports(&c.id).unwrap().history.len(),
            reversed.history.len()
        );
        // Scope derives from the session, never a supplied firm ID.
        w.session().unwrap().info.counselor_id = "fictional-foreign-counselor".into();
        assert!(w.query_category_suggestions(&c.id, query(0, None)).is_err());
    }
}
