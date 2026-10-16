//! Scoped immutable card completeness snapshots, exact source validation and audit.
use super::*;
use crate::card_facts::{self as review, Choices, Input, Record, Snapshot};
use sha2::{Digest, Sha256};
fn json<T: Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|_| STORAGE_ERROR.into())
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
pub(super) fn revision(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<i64, String> {
    c.query_row("SELECT COALESCE(MAX(revision),0) FROM card_facts_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)
}
fn current_choices(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(Choices, i64), String> {
    let links = card_review_store::history(c, w, client)?.pop();
    let bank = imports_store::read_imports(c, w, client)?;
    let ledger = cards_store::read(c, w, client)?;
    Ok((
        review::choices(
            links.as_ref(),
            &ledger,
            bank.financial_facts.history.last().cloned(),
        ),
        bank.revision,
    ))
}
fn validate_source(c: &Connection, w: &WorkspaceInfo, s: &Snapshot) -> Result<(), String> {
    let links = card_review_store::history(c, w, &s.client_id)?
        .into_iter()
        .rfind(|l| l.source_revision <= s.source_revision);
    let facts=s.choices.facts.as_ref().map(|f|->Result<crate::financial_facts::Review,String>{
  let (body,reason,at,counselor):(String,String,String,String)=c.query_row("SELECT data_json,reason,reviewed_at,counselor_name FROM financial_fact_reviews WHERE id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![f.id,s.client_id,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(storage_error)?;
  if at>s.updated_at{return Err(STORAGE_ERROR.into());}
  Ok(crate::financial_facts::Review{id:f.id.clone(),data:serde_json::from_str(&body).map_err(|_|STORAGE_ERROR)?,reason,reviewed_at:at,counselor_name:counselor})
 }).transpose()?;
    let ledger = cards_store::read_at(c, w, &s.client_id, s.source_revision)?;
    if review::choices(links.as_ref(), &ledger, facts) != s.choices {
        return Err(
            "The card completeness evidence differs from its original scoped sources.".into(),
        );
    }
    Ok(())
}
pub(super) fn versions(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<Vec<Snapshot>, String> {
    let (count,bytes):(i64,i64)=c.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(payload_json AS BLOB))),0) FROM card_facts_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(storage_error)?;
    if count > 50 || bytes > 16 * 1024 * 1024 {
        return Err("Card completeness history exceeds its safety limit.".into());
    }
    let rows=c.prepare("SELECT revision,payload_json,digest FROM card_facts_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision LIMIT 51").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    let mut result: Vec<Snapshot> = vec![];
    for (rev, body, digest) in rows {
        if body.len() > review::MAX_JSON || hash(&body) != digest {
            return Err(STORAGE_ERROR.into());
        }
        let s: Snapshot = serde_json::from_str(&body).map_err(|_| STORAGE_ERROR)?;
        review::validate(&s)?;
        if s.client_id != client
            || s.revision != rev
            || rev != result.len() as i64 + 1
            || json(&s)? != body
            || result.last().is_some_and(|p| {
                p.source_revision > s.source_revision || p.updated_at > s.updated_at
            })
        {
            return Err(STORAGE_ERROR.into());
        }
        validate_source(c, w, &s)?;
        result.push(s);
    }
    Ok(result)
}
pub(super) fn validate_capture(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    s: &Snapshot,
) -> Result<(), String> {
    if s.client_id != client || !versions(c, w, client)?.contains(s) {
        return Err(
            "Captured card completeness does not match the client's original history.".into(),
        );
    }
    Ok(())
}
pub(super) fn capture(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    selection: review::Selection,
    bank: &crate::imports::Snapshot,
    profile: &Client,
    period: (&str, &str),
) -> Result<Snapshot, String> {
    let s = versions(c, w, client)?
        .pop()
        .ok_or("Save and review card completeness first.")?;
    if s.revision != selection.revision
        || s.source_revision != bank.revision
        || s.profile_updated_at != profile.updated_at
        || s.data.from_month != period.0
        || s.data.through_month != period.1
        || s.choices != current_choices(c, w, client)?.0
    {
        return Err("Card completeness has a different version, source, client or period. Review the latest evidence before capturing it.".into());
    }
    Ok(s)
}
fn read(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<Record, String> {
    let profile = scoped_client(c, w, client)?;
    let history = versions(c, w, client)?;
    let (choices, source) = current_choices(c, w, client)?;
    let mut reasons = vec![];
    if history.last().is_some_and(|s| {
        s.source_revision != source
            || s.profile_updated_at != profile.updated_at
            || s.choices != choices
    }) {
        reasons.push("Card accounts, source decisions, verified balances or the client profile changed. The original review is preserved; compare the latest evidence and save a new reviewed version.".into());
    }
    Ok(Record {
        client_id: client.into(),
        current_source_revision: source,
        current_profile_updated_at: profile.updated_at,
        latest_revision: history.len() as i64,
        snapshot: history.last().cloned(),
        history,
        choices,
        needs_review: !reasons.is_empty(),
        review_reasons: reasons,
    })
}
impl Workspace {
    pub fn get_card_facts(&mut self, client: &str) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let result = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn save_card_facts(&mut self, client: &str, input: Input) -> Result<Record, String> {
        if !input.acknowledge {
            return Err("Confirm card inventory, source-bound roles, balance mappings and unresolved findings before saving.".into());
        }
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let profile = scoped_client(&tx, &s.info, client)?;
        let previous = versions(&tx, &s.info, client)?;
        let (choices, source) = current_choices(&tx, &s.info, client)?;
        if previous.len() as i64 != input.expected_revision
            || source != input.expected_source_revision
            || profile.updated_at != input.expected_profile_updated_at
        {
            return Err("Card review, source or client context changed elsewhere. Review latest evidence; your draft was not saved.".into());
        }
        if previous.len() >= 50 {
            return Err("This client reached the 50-version card completeness limit.".into());
        }
        let data = review::canonical(input.data, &choices)?;
        let evaluation = review::evaluate(&data, &choices)?;
        let snapshot = Snapshot {
            format: 1,
            client_id: client.into(),
            revision: input.expected_revision + 1,
            source_revision: source,
            profile_updated_at: profile.updated_at,
            updated_at: timestamp(),
            counselor_name: s.info.counselor_name.clone(),
            choices,
            data,
            evaluation,
        };
        review::validate(&snapshot)?;
        let body = json(&snapshot)?;
        let total = previous
            .iter()
            .map(|s| json(s).map(|b| b.len()))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .sum::<usize>();
        if body.len() > review::MAX_JSON || total + body.len() > 16 * 1024 * 1024 {
            return Err(
                "Card completeness exceeds the 4 MiB review or 16 MiB client-history limit.".into(),
            );
        }
        tx.execute("INSERT INTO card_facts_history(client_id,organization_id,counselor_id,revision,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6)",params![client,s.info.organization_id,s.info.counselor_id,snapshot.revision,body,hash(&body)]).map_err(storage_error)?;
        audit(&tx, &s.info, "card_facts_saved", Some(client))?;
        let result = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        cards_store::tests as cards,
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    use crate::{
        card_facts::{Applicability, Classification, Data, Expected, Role},
        card_review,
        cards::Kind,
        financial_facts::{Balance, BalanceKind},
        imports::Status,
    };
    fn input(r: &Record, data: Data) -> Input {
        Input {
            expected_revision: r.latest_revision,
            expected_source_revision: r.current_source_revision,
            expected_profile_updated_at: r.current_profile_updated_at.clone(),
            data,
            acknowledge: true,
        }
    }
    fn fixture(w: &mut Workspace, id: &str, dir: &Path) -> Record {
        let mut s = cards::stage(w, id, dir);
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
                .review_card_transaction(id, cards::decision(&s, row, kind, Status::Approved))
                .unwrap();
        }
        let d = w.get_card_review(id).unwrap();
        let d = w
            .save_card_review(
                id,
                card_review::Input {
                    expected_revision: 0,
                    expected_source_revision: d.source_revision,
                    reason: "Verified original fictional account".into(),
                    acknowledge: true,
                    action: card_review::Action::CreateAccount {
                        label: "Fictional card".into(),
                    },
                },
            )
            .unwrap();
        w.save_card_review(
            id,
            card_review::Input {
                expected_revision: 1,
                expected_source_revision: d.source_revision,
                reason: "Explicit original source assignment".into(),
                acknowledge: true,
                action: card_review::Action::AssignStatement {
                    statement_id: s.statements[0].source.id.clone(),
                    account_id: Some(d.snapshot.unwrap().accounts[0].id.clone()),
                },
            },
        )
        .unwrap();
        let r = w.get_client_imports(id).unwrap();
        w.review_financial_facts(
            id,
            crate::financial_facts::Input {
                expected_revision: r.revision,
                data: crate::financial_facts::Data {
                    as_of: Some("2026-09-30".into()),
                    full_inventory: true,
                    balances: vec![Balance {
                        label: "Fictional card debt".into(),
                        kind: BalanceKind::Debt,
                        amount: Some("1037.05".into()),
                        monthly_payment: None,
                        evidence:
                            "Reviewed synthetic card statement, entered once in debt inventory"
                                .into(),
                        document_id: None,
                    }],
                    classifications: vec![],
                },
                reason: "Verified fictional full debt inventory".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        w.get_card_facts(id).unwrap()
    }
    fn complete(r: &Record) -> Data {
        let source = &r.choices.sources[0];
        Data {
            from_month: "2026-09".into(),
            through_month: "2026-09".into(),
            inventory_reviewed: true,
            inventory_notes: "Reviewed all fictional household cards, including archived accounts"
                .into(),
            missing_information: String::new(),
            conclusion:
                "Reviewed card evidence only; bank completeness and final approval remain separate"
                    .into(),
            accounts: vec![Expected {
                account_id: Some(r.choices.accounts[0].id.clone()),
                label: r.choices.accounts[0].label.clone(),
                applicability: Applicability::Applicable,
                start: Some("2026-09-01".into()),
                end: Some("2026-09-30".into()),
                notes: "Known applicable card throughout period".into(),
                source_ids: vec![source.id.clone()],
                closing_source_id: Some(source.id.clone()),
                balance_label: Some("Fictional card debt".into()),
            }],
            classifications: source
                .rows
                .iter()
                .filter(|r| r.fields.kind != Kind::Payment)
                .map(|r| Classification {
                    statement_id: source.id.clone(),
                    row: r.row,
                    approval_id: r.approval_id.clone().unwrap(),
                    role: if r.row == 0 {
                        Role::Discretionary
                    } else {
                        Role::Necessary
                    },
                })
                .collect(),
        }
    }
    #[test]
    fn exact_roles_debt_mapping_report9_history_recovery_and_legacy_bytes() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let r = fixture(&mut w, &c.id, root.path());
        let before = w.get_client_imports(&c.id).unwrap();
        let old = w
            .save_assessment_record(&c.id, bank::assessment_input(&before, &c))
            .unwrap();
        let old_json = json(&old.payload).unwrap();
        assert!(!old_json.contains("\"cardFacts\""));
        let r = w.save_card_facts(&c.id, input(&r, complete(&r))).unwrap();
        let s = r.snapshot.as_ref().unwrap();
        assert!(s.evaluation.evidence_complete);
        // Independent role arithmetic: necessary = -20.02 + 5.04 + 2.05 = -12.93; discretionary = 100.01.
        assert_eq!(s.evaluation.necessary_spending, "-12.93");
        assert_eq!(s.evaluation.discretionary_spending, "100.01");
        assert_eq!(s.evaluation.unclassified_spending, "0.00");
        assert_eq!(s.evaluation.findings[0].difference.as_deref(), Some("0.00"));
        assert_eq!(
            json(&w.get_client_imports(&c.id).unwrap().financial_facts).unwrap(),
            json(&before.financial_facts).unwrap()
        );
        assert_eq!(
            w.get_client_imports(&c.id).unwrap().revision,
            before.revision
        );
        let mut i = bank::assessment_input(&before, &c);
        i.cards = Some(card_review::Selection {
            revision: r.choices.link_revision,
        });
        i.card_facts = Some(review::Selection { revision: 1 });
        let report = w.save_assessment_record(&c.id, i).unwrap();
        assert_eq!(report.payload.format, 9);
        let original = json(&report.payload).unwrap();
        assert!(
            !w.review_final_report(&c.id, &report.payload.id)
                .unwrap()
                .readiness
                .ready
        );
        let pdf = crate::assessment_pdf::render(&report, &Utc::now().to_rfc3339())
            .unwrap()
            .0;
        let text = pdf_extract::extract_text_from_mem(&pdf).unwrap();
        assert!(text.contains("Captured card completeness evidence"));
        assert!(text.contains("-12.93"));
        assert!(text.contains("1037.05"));
        assert!(!text.contains("2074.10"));
        let mut data = complete(&r);
        data.missing_information = "Awaiting independent original confirmation".into();
        let updated = w.save_card_facts(&c.id, input(&r, data)).unwrap();
        assert!(
            !updated
                .snapshot
                .as_ref()
                .unwrap()
                .evaluation
                .evidence_complete
        );
        assert!(w
            .get_assessment_record(&c.id, &report.payload.id)
            .unwrap()
            .review_reasons
            .iter()
            .any(|s| s.contains("later card completeness")));
        let ledger = w.get_card_statements(&c.id).unwrap();
        w.review_card_transaction(
            &c.id,
            cards::decision(&ledger, 0, Kind::Purchase, Status::Approved),
        )
        .unwrap();
        let current = w.get_card_facts(&c.id).unwrap();
        assert!(current.needs_review);
        assert!(w
            .save_card_facts(&c.id, input(&current, complete(&r)))
            .is_err());
        assert_eq!(
            json(
                &w.get_assessment_record(&c.id, &report.payload.id)
                    .unwrap()
                    .payload
            )
            .unwrap(),
            original
        );
        assert_eq!(
            json(
                &w.get_assessment_record(&c.id, &old.payload.id)
                    .unwrap()
                    .payload
            )
            .unwrap(),
            old_json
        );
        let saved = json(&current.history).unwrap();
        w.lock_workspace();
        assert!(w.get_card_facts(&c.id).is_err());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            json(&w.get_card_facts(&c.id).unwrap().history).unwrap(),
            saved
        );
        let backups = tempfile::tempdir().unwrap();
        let b = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let pending = w.prepare_restore(b.path, PASSWORD.into()).unwrap();
        w.confirm_restore(pending.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            json(
                &w.get_assessment_record(&c.id, &report.payload.id)
                    .unwrap()
                    .payload
            )
            .unwrap(),
            original
        );
        assert_eq!(
            json(&w.get_card_facts(&c.id).unwrap().history).unwrap(),
            saved
        );
    }
    #[test]
    fn incomplete_unknown_duplicates_gaps_offsets_and_applicability_remain_visible() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let r = fixture(&mut w, &c.id, root.path());
        let mut d = complete(&r);
        d.classifications.clear();
        let v = review::evaluate(&d, &r.choices).unwrap();
        assert!(!v.evidence_complete);
        assert_eq!(v.stale_classifications, 4);
        assert_eq!(v.unclassified_spending, "87.08");
        let mut choices = r.choices.clone();
        choices.sources[0].rows[0].fields.amount = Some("20.02".into());
        choices.sources[0].rows[3].fields.amount = Some("0.00".into());
        choices.sources[0].rows[4].fields.amount = Some("0.00".into());
        let v = review::evaluate(&d, &choices).unwrap();
        assert_eq!(v.unclassified_spending, "0.00");
        assert_eq!(v.stale_classifications, 4);
        assert!(!v.evidence_complete);
        let d = complete(&r);
        for mutate in [0, 1, 2, 3, 4, 5] {
            let mut choices = r.choices.clone();
            match mutate {
                0 => choices.sources[0].reconciled = false,
                1 => choices.sources[0].start = "2026-09-02".into(),
                2 => choices.facts.as_mut().unwrap().data.balances[0].amount = None,
                3 => {
                    choices.facts.as_mut().unwrap().data.balances[0].amount = Some("1037.06".into())
                }
                4 => choices.facts.as_mut().unwrap().data.as_of = Some("2026-09-29".into()),
                _ => choices.sources[0].closing = "-1.00".into(),
            };
            assert!(!review::evaluate(&d, &choices).unwrap().evidence_complete);
        }
        let mut missing_closing = d.clone();
        missing_closing.accounts[0].closing_source_id = None;
        let unknown = review::evaluate(&missing_closing, &r.choices).unwrap();
        assert_eq!(unknown.findings[0].closing_owed, None);
        assert_eq!(
            unknown.findings[0].mapped_balance.as_deref(),
            Some("1037.05")
        );
        assert_eq!(unknown.findings[0].difference, None);
        let mut not = d.clone();
        let e = &mut not.accounts[0];
        e.applicability = Applicability::NotApplicable;
        e.start = None;
        e.end = None;
        e.source_ids.clear();
        e.closing_source_id = None;
        e.balance_label = None;
        assert!(review::canonical(not.clone(), &r.choices).is_ok());
        assert!(
            !review::evaluate(&not, &r.choices)
                .unwrap()
                .evidence_complete
        );
        let mut missing = d.clone();
        missing.accounts.push(Expected {
            account_id: None,
            label: "Unprovided card".into(),
            applicability: Applicability::Unknown,
            start: None,
            end: None,
            notes: "Client reports another card; no evidence yet".into(),
            source_ids: vec![],
            closing_source_id: None,
            balance_label: None,
        });
        assert!(
            !review::evaluate(&missing, &r.choices)
                .unwrap()
                .evidence_complete
        );
        let mut double_choices = r.choices.clone();
        let another = crate::card_review::Account {
            id: Uuid::new_v4().to_string(),
            label: "Second fictional card".into(),
            archived: false,
        };
        double_choices.accounts.push(another.clone());
        let mut double = d.clone();
        let mut expected = double.accounts[0].clone();
        expected.account_id = Some(another.id);
        expected.label = another.label;
        expected.source_ids.clear();
        expected.closing_source_id = None;
        double.accounts.push(expected);
        assert!(review::canonical(double, &double_choices)
            .unwrap_err()
            .contains("distinct existing debt balance"));
        let mut duplicate = d.clone();
        duplicate
            .classifications
            .push(duplicate.classifications[0].clone());
        assert!(review::canonical(duplicate, &r.choices).is_err());
        let mut payment = d.clone();
        payment.classifications.push(Classification {
            statement_id: r.choices.sources[0].id.clone(),
            row: 2,
            approval_id: r.choices.sources[0].rows[2].approval_id.clone().unwrap(),
            role: Role::Necessary,
        });
        assert!(review::canonical(payment, &r.choices).is_err());
        let mut choices = r.choices.clone();
        choices.sources.push(choices.sources[0].clone());
        choices.sources[1].id = Uuid::new_v4().to_string();
        let mut duplicate = d.clone();
        duplicate.accounts[0]
            .source_ids
            .push(choices.sources[1].id.clone());
        assert!(
            !review::evaluate(&duplicate, &choices)
                .unwrap()
                .evidence_complete
        );
        let mut short = d.clone();
        short.accounts[0].start = Some("2026-09-03".into());
        assert!(review::evaluate(&short, &r.choices)
            .unwrap()
            .issues
            .iter()
            .any(|s| s.contains("outside its recorded applicability")));
    }
    #[test]
    fn scope_stale_audit_rollback_append_only_and_corrupt_restore() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let r = fixture(&mut w, &c.id, root.path());
        let foreign = w.get_card_facts(&other.id).unwrap();
        assert!(w
            .save_card_facts(&other.id, input(&foreign, complete(&r)))
            .is_err());
        let mut stale = input(&r, complete(&r));
        stale.expected_source_revision -= 1;
        assert!(w.save_card_facts(&c.id, stale).is_err());
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER deny_card_facts_audit BEFORE INSERT ON audit_events WHEN NEW.event='card_facts_saved' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
        assert!(w.save_card_facts(&c.id, input(&r, complete(&r))).is_err());
        assert_eq!(w.get_card_facts(&c.id).unwrap().latest_revision, 0);
        assert_eq!(
            w.get_client_imports(&c.id).unwrap().revision,
            r.current_source_revision
        );
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER deny_card_facts_audit;")
            .unwrap();
        let saved = w.save_card_facts(&c.id, input(&r, complete(&r))).unwrap();
        let s = saved.snapshot.unwrap();
        let conn = &w.session().unwrap().connection;
        assert!(conn.execute("DELETE FROM card_facts_history", []).is_err());
        assert!(conn
            .execute("UPDATE card_facts_history SET digest='rewrite'", [])
            .is_err());
        conn.execute_batch("DROP TRIGGER card_facts_no_update;")
            .unwrap();
        let mut corrupt = s;
        corrupt.choices.sources[0].sha256 = "a".repeat(64);
        let body = json(&corrupt).unwrap();
        conn.execute(
            "UPDATE card_facts_history SET payload_json=?1,digest=?2",
            params![body, hash(&body)],
        )
        .unwrap();
        assert!(w.get_card_facts(&c.id).is_err());
        let backups = tempfile::tempdir().unwrap();
        let b = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(b.path, PASSWORD.into()).is_err());
    }
    #[test]
    fn schema24_migration_failure_rolls_back_without_changing_card_evidence() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let r = fixture(&mut w, &c.id, root.path());
        let before = json(&r.choices).unwrap();
        let conn = &mut w.session().unwrap().connection;
        conn.execute_batch("DROP TABLE IF EXISTS note_amendments; DROP INDEX IF EXISTS note_amendment_scope; DROP TABLE IF EXISTS category_rule_applications; DROP TABLE IF EXISTS category_rule_versions; DROP INDEX IF EXISTS review_history_rule_scope; DROP TABLE card_facts_history; PRAGMA user_version=24; CREATE TABLE audit_events_v25(collision TEXT);").unwrap();
        assert!(migrate(conn, false).is_err());
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            24
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='card_facts_history'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        conn.execute_batch("DROP TABLE audit_events_v25;").unwrap();
        migrate(conn, false).unwrap();
        assert_eq!(
            json(&w.get_card_facts(&c.id).unwrap().choices).unwrap(),
            before
        );
    }
}
