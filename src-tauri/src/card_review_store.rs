//! Authenticated append-only card account, source assignment and repayment history.
use super::*;
use crate::card_review::{self as review, Action, Directory, Snapshot};
use sha2::{Digest, Sha256};
fn json<T: Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|_| STORAGE_ERROR.into())
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
pub(super) fn history(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<Vec<Snapshot>, String> {
    let bank = imports_store::read_imports(c, w, client)?;
    let cards = cards_store::read(c, w, client)?;
    let rows=c.prepare("SELECT revision,source_revision,payload_json,digest FROM card_review_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision LIMIT 201").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    let mut result: Vec<Snapshot> = vec![];
    for (revision, source_revision, body, digest) in rows {
        if body.len() > 131072 || hash(&body) != digest {
            return Err(STORAGE_ERROR.into());
        }
        let s: Snapshot = serde_json::from_str(&body).map_err(|_| STORAGE_ERROR)?;
        review::validate(&s)?;
        if s.client_id != client
            || s.revision != revision
            || revision != result.len() as i64 + 1
            || s.source_revision != source_revision
            || source_revision > bank.revision
            || result.last().is_some_and(|p| {
                p.source_revision >= source_revision
                    || p.updated_at > s.updated_at
                    || p.accounts
                        .iter()
                        .any(|a| !s.accounts.iter().any(|b| b.id == a.id))
            })
        {
            return Err(STORAGE_ERROR.into());
        }
        for a in &s.assignments {
            if !cards.statements.iter().any(|c| {
                c.source.id == a.statement_id && c.source.import_revision < source_revision
            }) {
                return Err(STORAGE_ERROR.into());
            }
        }
        for p in &s.repayments {
            review::evidence(p, &bank, &cards).map_err(|_| STORAGE_ERROR)?;
            let h = bank
                .history
                .iter()
                .find(|h| h.id == p.bank_decision_id)
                .ok_or(STORAGE_ERROR)?;
            let card = cards
                .statements
                .iter()
                .find(|c| c.source.id == p.statement_id)
                .and_then(|c| c.history.iter().find(|h| h.id == p.card_decision_id))
                .ok_or(STORAGE_ERROR)?;
            if h.reviewed_at > s.updated_at || card.revision >= source_revision {
                return Err(STORAGE_ERROR.into());
            }
        }
        result.push(s);
    }
    Ok(result)
}
pub(super) fn capture(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    selection: review::Selection,
    bank: &crate::imports::Snapshot,
    period: crate::dashboard::Input,
) -> Result<review::Capture, String> {
    let all = history(c, w, client)?;
    let links = all
        .last()
        .ok_or("Review card accounts and source assignments before capturing card evidence.")?;
    if links.revision != selection.revision {
        return Err(
            "Card account links changed. Review the latest card evidence before saving.".into(),
        );
    }
    let ledger = cards_store::read(c, w, client)?;
    let calculated = review::calculate(bank, &ledger, links, period)?;
    Ok(review::Capture {
        ledger,
        links: links.clone(),
        review: calculated,
    })
}
pub(super) fn validate_capture(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    capture: &review::Capture,
) -> Result<(), String> {
    let all = history(c, w, client)?;
    let original = all
        .iter()
        .find(|s| s.revision == capture.links.revision)
        .ok_or(STORAGE_ERROR)?;
    if original != &capture.links
        || all
            .iter()
            .any(|s| s.revision > original.revision && s.source_revision <= capture.ledger.revision)
        || json(&cards_store::read_at(
            c,
            w,
            client,
            capture.ledger.revision,
        )?)? != json(&capture.ledger)?
    {
        return Err(STORAGE_ERROR.into());
    }
    Ok(())
}
impl Workspace {
    pub fn get_card_review(&mut self, client: &str) -> Result<Directory, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let history = history(&tx, &s.info, client)?;
        let bank = imports_store::read_imports(&tx, &s.info, client)?;
        let cards = cards_store::read(&tx, &s.info, client)?;
        let stale_repayments = history
            .last()
            .map(|h| {
                h.repayments
                    .iter()
                    .filter(|p| !review::current(p, &bank, &cards))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let result = Directory {
            stale_repayments,
            client_id: client.into(),
            source_revision: bank.revision,
            snapshot: history.last().cloned(),
            history,
            cards,
            bank,
        };
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn preview_card_review(
        &mut self,
        client: &str,
        input: crate::dashboard::Input,
    ) -> Result<review::Review, String> {
        let d = self.get_card_review(client)?;
        review::calculate(
            &d.bank,
            &d.cards,
            d.snapshot
                .as_ref()
                .ok_or("Create a card account and review source assignments first.")?,
            input,
        )
    }
    pub fn save_card_review(
        &mut self,
        client: &str,
        input: review::Input,
    ) -> Result<Directory, String> {
        if !input.acknowledge {
            return Err("Confirm the card identity, source assignment and repayment evidence before saving.".into());
        }
        let reason = crate::cards::text(&input.reason, 500, true)?;
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let all = history(&tx, &s.info, client)?;
        if all.len() as i64 != input.expected_revision || all.len() >= 200 {
            return Err("Card review changed or reached its 200-version limit. Refresh before saving; your draft was not saved.".into());
        }
        imports_store::bump(&tx, &s.info, client, input.expected_source_revision)?;
        let mut next = all.last().cloned().unwrap_or(Snapshot {
            format: 1,
            client_id: client.into(),
            revision: 0,
            source_revision: 0,
            updated_at: String::new(),
            counselor_name: String::new(),
            reason: String::new(),
            accounts: vec![],
            assignments: vec![],
            repayments: vec![],
        });
        let bank = imports_store::read_imports(&tx, &s.info, client)?;
        let ledger = cards_store::read(&tx, &s.info, client)?;
        match input.action {
            Action::CreateAccount { label } => {
                if next.accounts.len() >= 20 {
                    return Err("Use at most 20 durable card accounts per client.".into());
                }
                next.accounts.push(review::Account {
                    id: Uuid::new_v4().to_string(),
                    label: crate::cards::text(&label, 120, true)?,
                    archived: false,
                });
            }
            Action::UpdateAccount {
                id,
                label,
                archived,
            } => {
                let a = next
                    .accounts
                    .iter_mut()
                    .find(|a| a.id == id)
                    .ok_or("This card account is unavailable for this client.")?;
                a.label = crate::cards::text(&label, 120, true)?;
                a.archived = archived;
            }
            Action::AssignStatement {
                statement_id,
                account_id,
            } => {
                if !ledger
                    .statements
                    .iter()
                    .any(|s| s.source.id == statement_id)
                {
                    return Err("Choose this client's card statement.".into());
                }
                if account_id
                    .as_ref()
                    .is_some_and(|id| !next.accounts.iter().any(|a| &a.id == id && !a.archived))
                {
                    return Err("Choose an active card account for this client.".into());
                }
                next.assignments.retain(|a| a.statement_id != statement_id);
                if let Some(account_id) = account_id {
                    next.assignments.push(review::Assignment {
                        statement_id,
                        account_id,
                    });
                } else {
                    next.repayments.retain(|p| p.statement_id != statement_id);
                }
            }
            Action::LinkRepayment {
                statement_id,
                row,
                bank_record_id,
            } => {
                let statement = ledger
                    .statements
                    .iter()
                    .find(|s| s.source.id == statement_id)
                    .ok_or("Choose this client's card statement.")?;
                let card = statement
                    .history
                    .iter()
                    .rev()
                    .find(|h| h.row == row)
                    .ok_or("Approve the card payment first.")?;
                let bank_decision = bank
                    .history
                    .iter()
                    .rev()
                    .find(|h| h.record_id == bank_record_id)
                    .ok_or("Approve the bank balance movement first.")?;
                let p = review::Repayment {
                    statement_id,
                    row,
                    bank_record_id,
                    bank_decision_id: bank_decision.id.clone(),
                    card_decision_id: card.id.clone(),
                };
                review::evidence(&p, &bank, &ledger)?;
                if !review::current(&p, &bank, &ledger) {
                    return Err(
                        "Review the current bank and card approvals before matching.".into(),
                    );
                }
                if next.repayments.iter().any(|x| {
                    x.bank_record_id == p.bank_record_id
                        && (x.statement_id != p.statement_id || x.row != p.row)
                }) {
                    return Err("This bank row is already matched to another card payment. Unlink it explicitly first.".into());
                }
                next.repayments
                    .retain(|x| x.statement_id != p.statement_id || x.row != p.row);
                next.repayments.push(p);
            }
            Action::UnlinkRepayment { statement_id, row } => {
                if !next
                    .repayments
                    .iter()
                    .any(|p| p.statement_id == statement_id && p.row == row)
                {
                    return Err("This repayment link is unavailable for this client.".into());
                }
                next.repayments
                    .retain(|p| p.statement_id != statement_id || p.row != row);
            }
        }
        next.revision = input.expected_revision + 1;
        next.source_revision = input.expected_source_revision + 1;
        next.updated_at = timestamp();
        next.counselor_name = s.info.counselor_name.clone();
        next.reason = reason;
        review::validate(&next)?;
        if all.last().is_some_and(|p| {
            p.accounts == next.accounts
                && p.assignments == next.assignments
                && p.repayments == next.repayments
        }) {
            return Err("No card account, assignment or repayment change was made.".into());
        }
        let body = json(&next)?;
        if body.len() > 131072 {
            return Err("Card review exceeds the 128 KiB limit.".into());
        }
        tx.execute("INSERT INTO card_review_history(client_id,organization_id,counselor_id,revision,source_revision,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![client,s.info.organization_id,s.info.counselor_id,next.revision,next.source_revision,body,hash(&body)]).map_err(storage_error)?;
        audit(&tx, &s.info, "card_review_saved", Some(client))?;
        let history = history(&tx, &s.info, client)?;
        let stale_repayments = next
            .repayments
            .iter()
            .filter(|p| !review::current(p, &bank, &ledger))
            .cloned()
            .collect();
        let result = Directory {
            client_id: client.into(),
            source_revision: bank.revision,
            snapshot: Some(next),
            history,
            cards: ledger,
            bank,
            stale_repayments,
        };
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        cards_store::tests as card,
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    use crate::{cards::Kind, imports::Status};
    fn input(d: &Directory, action: Action) -> review::Input {
        review::Input {
            expected_revision: d.snapshot.as_ref().map_or(0, |s| s.revision),
            expected_source_revision: d.source_revision,
            reason: "Verified synthetic original account and counterpart".into(),
            acknowledge: true,
            action,
        }
    }
    fn change(w: &mut Workspace, id: &str, action: Action) -> Directory {
        let d = w.get_card_review(id).unwrap();
        w.save_card_review(id, input(&d, action)).unwrap()
    }
    fn period() -> crate::dashboard::Input {
        crate::dashboard::Input {
            from_month: "2026-09".into(),
            through_month: "2026-09".into(),
        }
    }
    fn fixture(w: &mut Workspace, id: &str, dir: &Path) -> Directory {
        let s=bank::stage(w,id,dir,"repayment.csv","Date,Description,Amount,Category,Type\n2026-09-01,Fictional wages,2000.01,Wages,income\n2026-09-02,Fictional rent,-1200.02,Housing,expense\n2026-09-04,Fictional card repayment,-50.03,Card repayment,balanceMovement\n",bank::doc("Repayment checking"));
        for row in s.records {
            let revision = w.get_client_imports(id).unwrap().revision;
            w.review_transaction(
                id,
                crate::imports::ReviewInput {
                    expected_revision: revision,
                    record_id: row.id,
                    status: Status::Approved,
                    fields: row.fields,
                    reason: "Verified synthetic source".into(),
                    acknowledge_warnings: true,
                },
            )
            .unwrap();
        }
        let mut s = card::stage(w, id, dir);
        for (row, kind) in [
            Kind::Purchase,
            Kind::Refund,
            Kind::Payment,
            Kind::Fee,
            Kind::Interest,
        ]
        .into_iter()
        .enumerate()
        {
            s = w
                .review_card_transaction(id, card::decision(&s, row, kind, Status::Approved))
                .unwrap();
        }
        let d = change(
            w,
            id,
            Action::CreateAccount {
                label: "Fictional durable card".into(),
            },
        );
        let account = d.snapshot.unwrap().accounts[0].id.clone();
        change(
            w,
            id,
            Action::AssignStatement {
                statement_id: s.statements[0].source.id.clone(),
                account_id: Some(account),
            },
        )
    }
    #[test]
    fn combined_example_report8_legacy_bytes_stale_links_and_encrypted_recovery() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let mut d = fixture(&mut w, &c.id, root.path());
        let source = w.get_client_imports(&c.id).unwrap();
        let old = w
            .save_assessment_record(&c.id, bank::assessment_input(&source, &c))
            .unwrap();
        let old_bytes = json(&old.payload).unwrap();
        assert!(!old_bytes.contains("\"cards\""));
        assert_eq!(old.payload.format, 2);
        assert!(w
            .preview_card_review(&c.id, period())
            .unwrap()
            .observed_spending
            .is_none());
        let source_id = d.cards.statements[0].source.id.clone();
        let bank_id = d.bank.records[2].id.clone();
        d = change(
            &mut w,
            &c.id,
            Action::LinkRepayment {
                statement_id: source_id.clone(),
                row: 2,
                bank_record_id: bank_id.clone(),
            },
        );
        let v = w.preview_card_review(&c.id, period()).unwrap();
        // Independently: bank cash = 2000.01 - 1200.02 - 50.03 = 749.96.
        // Spending = 1200.02 + (100.01 - 20.02 + 5.04 + 2.05) = 1287.10.
        assert_eq!(v.bank_cash_movement, "749.96");
        assert_eq!(v.bank_net_spending, "1200.02");
        assert_eq!(v.card_net_spending, "87.08");
        assert_eq!(v.observed_spending.as_deref(), Some("1287.10"));
        assert_eq!(v.card_payments, "50.03");
        assert_eq!(v.matched_payments, "50.03");
        assert_eq!(v.card_liability_movement, "37.05");
        assert!(v.issues.is_empty());
        let source = w.get_client_imports(&c.id).unwrap();
        let mut report = bank::assessment_input(&source, &c);
        report.cards = Some(review::Selection {
            revision: d.snapshot.as_ref().unwrap().revision,
        });
        let saved = w.save_assessment_record(&c.id, report).unwrap();
        assert_eq!(saved.payload.format, 8);
        assert_eq!(saved.payload.cards.as_ref().unwrap().review, v);
        let bytes = json(&saved.payload).unwrap();
        let digest = saved.digest.clone();
        let readiness = w.review_final_report(&c.id, &saved.payload.id).unwrap();
        assert!(!readiness.readiness.ready);
        assert!(readiness
            .readiness
            .gates
            .iter()
            .any(|g| g.key == "cardPolicy" && !g.passed));
        let pdf = crate::assessment_pdf::render(&saved, &Utc::now().to_rfc3339())
            .unwrap()
            .0;
        let text = pdf_extract::extract_text_from_mem(&pdf).unwrap();
        assert!(text.contains("Captured bank/card review supplement"));
        assert!(text.contains("1287.10"));
        let bank_source = w.get_client_imports(&c.id).unwrap();
        let row = bank_source
            .records
            .iter()
            .find(|r| r.id == bank_id)
            .unwrap();
        w.review_transaction(
            &c.id,
            crate::imports::ReviewInput {
                expected_revision: bank_source.revision,
                record_id: bank_id.clone(),
                status: Status::Approved,
                fields: row.fields.clone(),
                reason: "Fresh explicit review of same amount".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
        let d = w.get_card_review(&c.id).unwrap();
        assert_eq!(d.stale_repayments.len(), 1);
        assert!(w
            .preview_card_review(&c.id, period())
            .unwrap()
            .observed_spending
            .is_none());
        let historical = w.get_assessment_record(&c.id, &saved.payload.id).unwrap();
        assert_eq!(json(&historical.payload).unwrap(), bytes);
        assert_eq!(historical.digest, digest);
        assert_eq!(
            json(
                &w.get_assessment_record(&c.id, &old.payload.id)
                    .unwrap()
                    .payload
            )
            .unwrap(),
            old_bytes
        );
        // A stale supplement can still be captured honestly. It retains both old and latest bank approvals.
        let source = w.get_client_imports(&c.id).unwrap();
        let mut report = bank::assessment_input(&source, &c);
        report.cards = Some(review::Selection {
            revision: d.snapshot.as_ref().unwrap().revision,
        });
        let stale = w.save_assessment_record(&c.id, report).unwrap();
        assert!(stale
            .payload
            .cards
            .unwrap()
            .review
            .observed_spending
            .is_none());
        change(
            &mut w,
            &c.id,
            Action::LinkRepayment {
                statement_id: source_id.clone(),
                row: 2,
                bank_record_id: bank_id,
            },
        );
        let s = w.get_card_statements(&c.id).unwrap();
        w.review_card_transaction(
            &c.id,
            card::decision(&s, 2, Kind::Payment, Status::Rejected),
        )
        .unwrap();
        assert_eq!(w.get_card_review(&c.id).unwrap().stale_repayments.len(), 1);
        change(
            &mut w,
            &c.id,
            Action::UnlinkRepayment {
                statement_id: source_id,
                row: 2,
            },
        );
        let original = json(&w.get_card_review(&c.id).unwrap()).unwrap();
        w.lock_workspace();
        assert!(w.get_card_review(&c.id).is_err());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(json(&w.get_card_review(&c.id).unwrap()).unwrap(), original);
        let backups = tempfile::tempdir().unwrap();
        let b = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let r = w.prepare_restore(b.path, PASSWORD.into()).unwrap();
        w.confirm_restore(r.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(json(&w.get_card_review(&c.id).unwrap()).unwrap(), original);
        assert_eq!(
            json(
                &w.get_assessment_record(&c.id, &saved.payload.id)
                    .unwrap()
                    .payload
            )
            .unwrap(),
            bytes
        );
    }
    #[test]
    fn card_identity_scope_history_conflicts_and_audit_failure_are_atomic() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let d = fixture(&mut w, &c.id, root.path());
        let original = json(&d).unwrap();
        let s = d.snapshot.as_ref().unwrap();
        let id = s.accounts[0].id.clone();
        let source = d.cards.statements[0].source.id.clone();
        for action in [
            Action::CreateAccount {
                label: "FICTIONAL DURABLE CARD".into(),
            },
            Action::UpdateAccount {
                id: other.id.clone(),
                label: "Wrong".into(),
                archived: false,
            },
            Action::AssignStatement {
                statement_id: other.id.clone(),
                account_id: Some(id.clone()),
            },
            Action::LinkRepayment {
                statement_id: source.clone(),
                row: 0,
                bank_record_id: d.bank.records[2].id.clone(),
            },
            Action::LinkRepayment {
                statement_id: source.clone(),
                row: 2,
                bank_record_id: d.bank.records[1].id.clone(),
            },
        ] {
            assert!(w.save_card_review(&c.id, input(&d, action)).is_err());
            assert_eq!(json(&w.get_card_review(&c.id).unwrap()).unwrap(), original);
        }
        let cross = input(
            &w.get_card_review(&other.id).unwrap(),
            Action::AssignStatement {
                statement_id: source.clone(),
                account_id: Some(id.clone()),
            },
        );
        assert!(w.save_card_review(&other.id, cross).is_err());
        assert!(w.get_card_review(&other.id).unwrap().snapshot.is_none());
        let mut stale = input(
            &d,
            Action::CreateAccount {
                label: "Another".into(),
            },
        );
        stale.expected_source_revision -= 1;
        assert!(w.save_card_review(&c.id, stale).is_err());
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER deny_card_link_audit BEFORE INSERT ON audit_events WHEN NEW.event='card_review_saved' BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;").unwrap();
        assert!(w
            .save_card_review(
                &c.id,
                input(
                    &d,
                    Action::CreateAccount {
                        label: "Rollback".into()
                    }
                )
            )
            .is_err());
        assert_eq!(json(&w.get_card_review(&c.id).unwrap()).unwrap(), original);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER deny_card_link_audit;")
            .unwrap();
        let archived = change(
            &mut w,
            &c.id,
            Action::UpdateAccount {
                id: id.clone(),
                label: "Renamed card".into(),
                archived: true,
            },
        );
        assert_eq!(archived.snapshot.as_ref().unwrap().accounts[0].id, id);
        assert!(w
            .save_card_review(
                &c.id,
                input(
                    &archived,
                    Action::AssignStatement {
                        statement_id: source.clone(),
                        account_id: Some(id.clone())
                    }
                )
            )
            .is_err());
        let reopened = change(
            &mut w,
            &c.id,
            Action::UpdateAccount {
                id: id.clone(),
                label: "Renamed card".into(),
                archived: false,
            },
        );
        assert_eq!(
            reopened.history[0].accounts[0].label,
            "Fictional durable card"
        );
        let conn = &w.session().unwrap().connection;
        assert!(conn.execute("DELETE FROM card_review_history", []).is_err());
        assert!(conn
            .execute("UPDATE card_review_history SET digest='rewrite'", [])
            .is_err());
        conn.execute_batch("DROP TRIGGER card_review_no_update;")
            .unwrap();
        let mut corrupt = reopened.snapshot.unwrap();
        corrupt.assignments[0].statement_id = other.id.clone();
        let body = json(&corrupt).unwrap();
        conn.execute(
            "UPDATE card_review_history SET payload_json=?1,digest=?2 WHERE revision=?3",
            params![body, hash(&body), corrupt.revision],
        )
        .unwrap();
        assert!(w.get_card_review(&c.id).is_err());
        let backups = tempfile::tempdir().unwrap();
        let b = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(b.path, PASSWORD.into()).is_err());
    }
    #[test]
    fn coverage_overlap_gaps_continuity_unknown_rows_and_invalid_capture_fail_closed() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let d = fixture(&mut w, &c.id, root.path());
        let source = d.cards.statements[0].source.id.clone();
        let id = d.snapshot.as_ref().unwrap().accounts[0].id.clone();
        let d = change(
            &mut w,
            &c.id,
            Action::LinkRepayment {
                statement_id: source.clone(),
                row: 2,
                bank_record_id: d.bank.records[2].id.clone(),
            },
        );
        let links = d.snapshot.as_ref().unwrap();
        let mut ledger = d.cards.clone();
        ledger.statements[0].source.input.period_start = "2026-09-02".into();
        let v = review::calculate(&d.bank, &ledger, links, period()).unwrap();
        assert!(v.observed_spending.is_none());
        assert!(v.issues.iter().any(|s| s.contains("gap")));
        let mut ledger = d.cards.clone();
        ledger.statements.push(ledger.statements[0].clone());
        ledger.statements[1].source.id = Uuid::new_v4().to_string();
        let mut linked = links.clone();
        linked.assignments.push(review::Assignment {
            statement_id: ledger.statements[1].source.id.clone(),
            account_id: id.clone(),
        });
        let v = review::calculate(&d.bank, &ledger, &linked, period()).unwrap();
        assert!(v.observed_spending.is_none());
        assert!(v.issues.iter().any(|s| s.contains("overlapping")));
        let mut ledger = d.cards.clone();
        ledger.statements[0].source.input.period_end = "2026-09-15".into();
        let mut next = ledger.statements[0].clone();
        next.source.id = Uuid::new_v4().to_string();
        next.source.input.period_start = "2026-09-16".into();
        next.source.input.period_end = "2026-09-30".into();
        next.source.input.opening_owed = "999.99".into();
        next.rows.clear();
        let mut linked = links.clone();
        linked.assignments.push(review::Assignment {
            statement_id: next.source.id.clone(),
            account_id: id,
        });
        ledger.statements.push(next);
        let v = review::calculate(&d.bank, &ledger, &linked, period()).unwrap();
        assert!(v
            .issues
            .iter()
            .any(|s| s.contains("inconsistent consecutive")));
        assert!(v.observed_spending.is_none());
        let mut ledger = d.cards.clone();
        ledger.statements[0].summary.reconciled = false;
        let v = review::calculate(&d.bank, &ledger, links, period()).unwrap();
        assert!(v.observed_spending.is_none());
        let mut bank: crate::imports::Snapshot =
            serde_json::from_str(&json(&d.bank).unwrap()).unwrap();
        bank.records[0].status = Status::Pending;
        let v = review::calculate(&bank, &d.cards, links, period()).unwrap();
        assert!(v.observed_spending.is_none());
        bank.client_id = Uuid::new_v4().to_string();
        assert!(review::calculate(&bank, &d.cards, links, period()).is_err());
        let mut p = links.repayments[0].clone();
        p.row = 0;
        assert!(review::evidence(&p, &d.bank, &d.cards).is_err());
        let session = w.session().unwrap();
        let mut capture = capture(
            &session.connection,
            &session.info,
            &c.id,
            review::Selection {
                revision: links.revision,
            },
            &d.bank,
            period(),
        )
        .unwrap();
        capture.ledger.statements[0].source.file_name = "forged.csv".into();
        let session = w.session().unwrap();
        assert!(validate_capture(&session.connection, &session.info, &c.id, &capture).is_err());
        let mut duplicate = links.clone();
        duplicate.repayments.push(duplicate.repayments[0].clone());
        assert!(review::validate(&duplicate).is_err());
    }
    #[test]
    fn empty_card_inventory_remains_provisional_in_report_directory_and_final_policy() {
        let (_root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let d = change(
            &mut w,
            &c.id,
            Action::CreateAccount {
                label: "Awaiting fictional card evidence".into(),
            },
        );
        let source = w.get_client_imports(&c.id).unwrap();
        let mut input = bank::assessment_input(&source, &c);
        input.cards = Some(review::Selection {
            revision: d.snapshot.unwrap().revision,
        });
        let r = w.save_assessment_record(&c.id, input).unwrap();
        assert!(r.needs_review);
        assert!(r
            .payload
            .cards
            .as_ref()
            .unwrap()
            .review
            .observed_spending
            .is_none());
        assert!(w.list_assessment_records(&c.id).unwrap().records[0].needs_review);
        assert!(w
            .review_final_report(&c.id, &r.payload.id)
            .unwrap()
            .readiness
            .gates
            .iter()
            .any(|g| g.key == "cardPolicy" && !g.passed));
    }
    #[test]
    fn migration23_rolls_back_and_retains_previous_card_sources() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let s = card::stage(&mut w, &c.id, root.path());
        let original = json(&s).unwrap();
        let conn = &mut w.session().unwrap().connection;
        conn.execute_batch("DROP TABLE IF EXISTS household_events; DROP TABLE IF EXISTS workspace_security_history; DROP TABLE IF EXISTS note_amendments; DROP INDEX IF EXISTS note_amendment_scope; DROP TABLE IF EXISTS category_rule_applications; DROP TABLE IF EXISTS category_rule_versions; DROP INDEX IF EXISTS review_history_rule_scope; DROP TABLE IF EXISTS card_facts_history; DROP TABLE card_review_history; PRAGMA user_version=23; CREATE TABLE audit_events_v24(collision TEXT);").unwrap();
        assert!(migrate(conn, false).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            23
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='card_review_history'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        conn.execute_batch("DROP TABLE audit_events_v24;").unwrap();
        migrate(conn, false).unwrap();
        assert_eq!(
            json(&w.get_card_statements(&c.id).unwrap()).unwrap(),
            original
        );
    }
}
