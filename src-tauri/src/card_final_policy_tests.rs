use super::*;
use super::{
    cards_store::tests as card,
    imports_store::tests as bank,
    tests::{client_input, configured, PASSWORD},
};
use crate::{
    card_facts as cf, card_review as cr, cards, coverage_review as cv, final_reports as fr,
    financial_facts as ff, imports,
};
use chrono::Datelike;

fn change(w: &mut Workspace, id: &str, action: cr::Action) -> cr::Directory {
    let d = w.get_card_review(id).unwrap();
    w.save_card_review(
        id,
        cr::Input {
            expected_revision: d.snapshot.as_ref().map_or(0, |s| s.revision),
            expected_source_revision: d.source_revision,
            reason: "Inspected fictional original and repayment evidence".into(),
            acknowledge: true,
            action,
        },
    )
    .unwrap()
}
fn fixture(
    w: &mut Workspace,
    c: &Client,
    dir: &Path,
    case: &str,
) -> (crate::assessments::Record, fr::Record) {
    let old = bank::final_candidate(w, c, dir);
    let review = w.review_final_report(&c.id, &old.payload.id).unwrap();
    let legacy = w
        .approve_final_report(&c.id, bank::final_input(&review))
        .unwrap();
    let (start, end, month) = (
        old.payload.assessment.period_start.clone(),
        old.payload.assessment.period_end.clone(),
        old.payload.assessment.period_start[..7].to_owned(),
    );
    let date = crate::imports::iso_date(&start)
        .unwrap()
        .with_day(3)
        .unwrap();
    let mut d = bank::doc("Fictional repayment checking");
    d.period_start = Some(start.clone());
    d.period_end = Some(end.clone());
    d.opening_balance = Some("100.00".into());
    let closing = match case {
        "unmatchedBank" => "49.96",
        "bankRefund" => "54.97",
        _ => "49.97",
    };
    d.closing_balance = Some(closing.into());
    d.retain_source = true;
    let extra = match case {
        "unmatchedBank" => {
            format!("{date},Unmatched fictional principal,-0.01,Other,balanceMovement\n")
        }
        "bankRefund" => format!("{date},Fictional bank refund,5.00,Refund,refund\n"),
        _ => String::new(),
    };
    let source=bank::stage(w,&c.id,dir,"repayment.csv",&format!("Date,Description,Amount,Category,Type\n{date},Fictional card repayment,-50.03,Card repayment,balanceMovement\n{extra}"),d);
    let bank_row = source
        .records
        .iter()
        .find(|r| r.fields.kind == imports::Kind::BalanceMovement)
        .unwrap();
    for row in source
        .records
        .iter()
        .filter(|r| r.status == imports::Status::Pending)
    {
        let revision = w.get_client_imports(&c.id).unwrap().revision;
        w.review_transaction(
            &c.id,
            imports::ReviewInput {
                expected_revision: revision,
                record_id: row.id.clone(),
                fields: row.fields.clone(),
                status: imports::Status::Approved,
                reason: "Original fictional bank source row inspected".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
    }
    let path = dir.join("final-card.csv");
    fs::write(&path,format!("Date,Description,Amount,Category\n{month}-01,Fictional purchase,100.01,Food\n{month}-02,Fictional refund,-20.02,Food\n{month}-03,Fictional repayment,-50.03,Card payment\n{month}-04,Fictional fee,5.04,Fees\n{month}-05,Fictional interest,2.05,Interest\n")).unwrap();
    let p = w
        .inspect_client_csv(c.id.clone(), path.to_string_lossy().into())
        .unwrap();
    let mut input = card::input();
    input.period_start = if case == "outsideSource" {
        crate::imports::iso_date(&start)
            .unwrap()
            .pred_opt()
            .unwrap()
            .to_string()
    } else {
        start.clone()
    };
    input.period_end = end.clone();
    let revision = w.get_card_statements(&c.id).unwrap().revision;
    let mut ledger = w
        .stage_card_statement(
            &c.id,
            cards::Stage {
                expected_revision: revision,
                token: p.token,
                input,
            },
        )
        .unwrap();
    for (i, kind) in [
        cards::Kind::Purchase,
        cards::Kind::Refund,
        cards::Kind::Payment,
        cards::Kind::Fee,
        cards::Kind::Interest,
    ]
    .into_iter()
    .enumerate()
    {
        ledger = w
            .review_card_transaction(
                &c.id,
                card::decision(&ledger, i, kind, imports::Status::Approved),
            )
            .unwrap();
    }
    let d = change(
        w,
        &c.id,
        cr::Action::CreateAccount {
            label: "Fictional reviewed card".into(),
        },
    );
    let account = d.snapshot.unwrap().accounts[0].id.clone();
    let statement = ledger.statements[0].source.id.clone();
    change(
        w,
        &c.id,
        cr::Action::AssignStatement {
            statement_id: statement.clone(),
            account_id: Some(account.clone()),
        },
    );
    if case != "unmatched" {
        change(
            w,
            &c.id,
            cr::Action::LinkRepayment {
                statement_id: statement.clone(),
                row: 2,
                bank_record_id: bank_row.id.clone(),
            },
        );
    }
    let source = w.get_client_imports(&c.id).unwrap();
    let mut facts = source.financial_facts.data.clone().unwrap();
    facts.balances.push(ff::Balance {
        label: "Fictional repayment checking".into(),
        kind: ff::BalanceKind::Cash,
        amount: Some(closing.into()),
        monthly_payment: None,
        evidence: "Verified original synthetic checking closing balance".into(),
        document_id: None,
    });
    facts.balances.push(ff::Balance {
        label: "Fictional mapped card debt".into(),
        kind: ff::BalanceKind::Debt,
        amount: Some(
            if case == "mismatch" {
                "1037.06"
            } else {
                "1037.05"
            }
            .into(),
        ),
        monthly_payment: (case != "unknownPayment").then(|| "50.03".into()),
        evidence: "Original synthetic card closing debt recorded once".into(),
        document_id: None,
    });
    if case == "otherDebt" {
        facts.balances.push(ff::Balance {
            label: "Unmodeled fictional loan".into(),
            kind: ff::BalanceKind::Debt,
            amount: Some("10.01".into()),
            monthly_payment: Some("1.00".into()),
            evidence: "Other loan has no supported obligation evidence".into(),
            document_id: None,
        });
    }
    if case == "missingBankRole" {
        facts.classifications.clear();
    }
    facts.full_inventory = true;
    let source = w
        .review_financial_facts(
            &c.id,
            ff::Input {
                expected_revision: source.revision,
                data: facts,
                reason: "Reviewed complete fictional inventory and required payments".into(),
                acknowledge: true,
            },
        )
        .unwrap();
    let coverage = w.get_coverage_review(&c.id, None).unwrap();
    let choices = cv::choices(&source);
    let coverage=w.save_coverage_review(&c.id,cv::Input {expected_revision:coverage.latest_revision,expected_source_revision:source.revision,expected_profile_updated_at:c.updated_at.clone(),data:cv::Data {from_month:month.clone(),through_month:month.clone(),inventory_reviewed:true,inventory_notes:"Both bank accounts and the separately recorded card inventory reviewed; no other obligations omitted.".into(),missing_information:String::new(),conclusion:"Complete fictional bank context for explicit combined review".into(),accounts:choices.accounts.iter().map(|a|cv::Expectation {account_id:Some(a.id.clone()),label:a.label.clone(),kind:a.kind.clone(),expected_start:Some(start.clone()),expected_end:Some(end.clone()),exception:String::new(),source_ids:choices.sources.iter().filter(|s|s.account_id==a.id).map(|s|s.id.clone()).collect()}).collect()}}).unwrap();
    let cf = w.get_card_facts(&c.id).unwrap();
    let data = cf::Data {
        from_month: month.clone(),
        through_month: month.clone(),
        inventory_reviewed: true,
        inventory_notes: "All fictional cards and applicability verified".into(),
        missing_information: String::new(),
        conclusion: "Original role, repayment and debt evidence ready for separate approval".into(),
        accounts: vec![cf::Expected {
            account_id: Some(account),
            label: "Fictional reviewed card".into(),
            applicability: cf::Applicability::Applicable,
            start: Some(if case == "limitedCard" {
                crate::imports::iso_date(&start)
                    .unwrap()
                    .succ_opt()
                    .unwrap()
                    .to_string()
            } else {
                start
            }),
            end: Some(end),
            notes: "Full period applicability confirmed from fictional originals".into(),
            source_ids: vec![statement.clone()],
            closing_source_id: Some(statement.clone()),
            balance_label: Some("Fictional mapped card debt".into()),
        }],
        classifications: cf.choices.sources[0]
            .rows
            .iter()
            .filter(|r| {
                r.fields.kind != cards::Kind::Payment && !(case == "missingRole" && r.row == 0)
            })
            .map(|r| cf::Classification {
                statement_id: statement.clone(),
                row: r.row,
                approval_id: r.approval_id.clone().unwrap(),
                role: if r.row == 0 {
                    cf::Role::Discretionary
                } else {
                    cf::Role::Necessary
                },
            })
            .collect(),
    };
    let saved = w
        .save_card_facts(
            &c.id,
            cf::Input {
                expected_revision: 0,
                expected_source_revision: cf.current_source_revision,
                expected_profile_updated_at: c.updated_at.clone(),
                data,
                acknowledge: true,
            },
        )
        .unwrap();
    let mut input = bank::assessment_input(&source, c);
    input.from_month = month.clone();
    input.through_month = month;
    input.title = "Fictional card-inclusive final candidate".into();
    input.completeness = Some(cv::Selection {
        revision: coverage.latest_revision,
    });
    input.cards = Some(cr::Selection {
        revision: saved.choices.link_revision,
    });
    input.card_facts = Some(cf::Selection { revision: 1 });
    (w.save_assessment_record(&c.id, input).unwrap(), legacy)
}
fn input(p: &fr::Preview) -> fr::Input {
    let mut i = bank::final_input(p);
    i.decision.card_review=Some(fr::CardDecision {notes:"Reviewed card originals, each role and repayment, required payments and original household debt without duplication.".into(),confirmed:true});
    i
}

#[test]
fn card_final_exact_figures_scope_receipts_atomic_approval_pdf_and_historical_recovery() {
    let (root, mut w, _) = configured();
    let c = w.save_client(client_input()).unwrap();
    let (a, legacy) = fixture(&mut w, &c, root.path(), "complete");
    let original = serde_json::to_vec(&a.payload).unwrap();
    let legacy_bytes = serde_json::to_vec(&legacy.payload).unwrap();
    assert!(!String::from_utf8(legacy_bytes.clone())
        .unwrap()
        .contains("cardReview"));
    assert!(!String::from_utf8(legacy_bytes.clone())
        .unwrap()
        .contains("cardSummary"));
    assert_eq!(
        serde_json::to_vec(
            &w.get_final_report(&c.id, &legacy.payload.id)
                .unwrap()
                .payload
        )
        .unwrap(),
        legacy_bytes
    );
    let p = w.review_final_report(&c.id, &a.payload.id).unwrap();
    assert!(p.readiness.ready, "{:?}", p.readiness.gates);
    assert_eq!(p.readiness.policy, crate::card_final_policy::POLICY);
    let s = p.readiness.card_summary.as_ref().unwrap();
    assert_eq!(
        (
            s.necessary_spending.as_str(),
            s.discretionary_spending.as_str()
        ),
        ("1187.09", "100.01")
    );
    assert_eq!(
        (
            s.combined_spending.as_str(),
            s.income_less_spending.as_str()
        ),
        ("1287.10", "712.91")
    );
    assert_eq!(
        (
            s.bank_cash_movement.as_str(),
            s.matched_repayments.as_str(),
            s.existing_household_debt.as_str()
        ),
        ("749.96", "50.03", "1037.05")
    );
    assert_eq!(p.readiness.accounts.len(), 3);
    assert!(p
        .readiness
        .accounts
        .last()
        .unwrap()
        .account_id
        .starts_with("card:"));
    let mut bad = input(&p);
    bad.decision.card_review = None;
    assert!(w.approve_final_report(&c.id, bad).is_err());
    let mut bad = input(&p);
    bad.decision.card_review.as_mut().unwrap().confirmed = false;
    assert!(w.approve_final_report(&c.id, bad).is_err());
    let mut bad = input(&p);
    bad.decision.card_review.as_mut().unwrap().notes.clear();
    assert!(w.approve_final_report(&c.id, bad).is_err());
    let mut bad = input(&p);
    bad.decision.accounts.pop();
    assert!(w.approve_final_report(&c.id, bad).is_err());
    let mut bad = input(&p);
    bad.expected_readiness = "0".repeat(64);
    assert!(w.approve_final_report(&c.id, bad).is_err());
    w.session().unwrap().connection.execute_batch("CREATE TRIGGER fail_v2_audit BEFORE INSERT ON audit_events WHEN NEW.event='final_report_approved' BEGIN SELECT RAISE(ABORT,'synthetic v2 audit failure'); END;").unwrap();
    assert!(w.approve_final_report(&c.id, input(&p)).is_err());
    assert!(w
        .review_final_report(&c.id, &a.payload.id)
        .unwrap()
        .approval_id
        .is_none());
    w.session()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER fail_v2_audit;")
        .unwrap();
    let approved = w.approve_final_report(&c.id, input(&p)).unwrap();
    assert_eq!(approved.payload.format, 2);
    assert!(!approved.needs_review);
    let bytes = serde_json::to_vec(&approved.payload).unwrap();
    let end = crate::imports::iso_date(&a.payload.assessment.period_end).unwrap();
    let future = crate::card_final_policy::readiness(&a.payload, &a.digest, false, end).unwrap();
    assert!(!future.ready);
    assert!(future.gates.iter().any(|g| g.key == "period" && !g.passed));
    let expired = crate::card_final_policy::readiness(
        &a.payload,
        &a.digest,
        false,
        end + chrono::Duration::days(91),
    )
    .unwrap();
    assert!(!expired.ready);
    let mut wrong_policy: fr::Payload = serde_json::from_slice(&bytes).unwrap();
    wrong_policy.policy = fr::POLICY.into();
    assert!(fr::validate(&wrong_policy, &a).is_err());
    wrong_policy.policy = crate::card_final_policy::POLICY.into();
    wrong_policy.format = 1;
    assert!(fr::validate(&wrong_policy, &a).is_err());
    assert_eq!(
        serde_json::to_vec(&approved.assessment.payload).unwrap(),
        original
    );
    let other = w.save_client(client_input()).unwrap();
    assert!(w.get_final_report(&other.id, &approved.payload.id).is_err());
    assert!(w.approve_final_report(&c.id, input(&p)).is_err());
    let out = tempfile::tempdir().unwrap();
    let pdf = out.path().join("approved-card.pdf");
    w.export_final_report_pdf(&c.id, &approved.payload.id, pdf.to_str().unwrap())
        .unwrap();
    assert!(w
        .export_final_report_pdf(&c.id, &approved.payload.id, pdf.to_str().unwrap())
        .is_err());
    let text = pdf_extract::extract_text_from_mem(&fs::read(&pdf).unwrap()).unwrap();
    for v in [
        "Card-inclusive approved figures",
        "Combined recorded spending: USD 1287.10",
        "Income less combined spending: USD 712.91",
        "Bank cash movement: USD 749.96",
        "Existing household debt / counted once: USD 1037.05",
        "counselor-final-report-v2",
    ] {
        assert!(text.contains(v), "Missing {v}");
    }
    let cf = w.get_card_facts(&c.id).unwrap();
    let mut data = cf.snapshot.unwrap().data;
    data.conclusion = "Later reviewed conclusion; original approval stays historical".into();
    w.save_card_facts(
        &c.id,
        cf::Input {
            expected_revision: 1,
            expected_source_revision: cf.current_source_revision,
            expected_profile_updated_at: c.updated_at.clone(),
            data,
            acknowledge: true,
        },
    )
    .unwrap();
    let changed = w.get_final_report(&c.id, &approved.payload.id).unwrap();
    assert!(changed.needs_review);
    assert_eq!(serde_json::to_vec(&changed.payload).unwrap(), bytes);
    assert!(
        !w.review_final_report(&c.id, &a.payload.id)
            .unwrap()
            .readiness
            .ready
    );
    assert!(
        w.list_assessment_records(&c.id)
            .unwrap()
            .records
            .iter()
            .find(|r| r.id == a.payload.id)
            .unwrap()
            .needs_review
    );
    w.lock_workspace();
    drop(w);
    let mut w = Workspace::new(root.path().into());
    w.unlock_workspace(PASSWORD.into()).unwrap();
    assert_eq!(
        serde_json::to_vec(
            &w.get_final_report(&c.id, &approved.payload.id)
                .unwrap()
                .payload
        )
        .unwrap(),
        bytes
    );
    let backup = w
        .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
        .unwrap();
    w.lock_workspace();
    let restore = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
    w.confirm_restore(restore.token, "RESTORE".into()).unwrap();
    w.unlock_workspace(PASSWORD.into()).unwrap();
    assert_eq!(
        serde_json::to_vec(
            &w.get_final_report(&c.id, &approved.payload.id)
                .unwrap()
                .payload
        )
        .unwrap(),
        bytes
    );
    assert_eq!(
        serde_json::to_vec(
            &w.get_final_report(&c.id, &legacy.payload.id)
                .unwrap()
                .payload
        )
        .unwrap(),
        legacy_bytes
    );
}

#[test]
fn card_final_requires_complete_roles_debt_mappings_payments_and_supported_obligations() {
    for (case, key) in [
        ("unmatched", "cardCalculator"),
        ("missingRole", "cardEvidence"),
        ("mismatch", "cardEvidence"),
        ("unknownPayment", "obligations"),
        ("otherDebt", "obligations"),
        ("missingBankRole", "bankRoles"),
        ("limitedCard", "cardScope"),
        ("outsideSource", "cardSources"),
        ("unmatchedBank", "bankMovements"),
        ("bankRefund", "combinedFigures"),
    ] {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let (a, _) = fixture(&mut w, &c, root.path(), case);
        let p = w.review_final_report(&c.id, &a.payload.id).unwrap();
        assert!(!p.readiness.ready, "{case}");
        assert!(p.readiness.card_summary.is_none(), "{case}");
        assert!(
            p.readiness.gates.iter().any(|g| g.key == key && !g.passed),
            "{case}: {:?}",
            p.readiness.gates
        );
        assert!(w.approve_final_report(&c.id, input(&p)).is_err());
    }
}

#[test]
fn card_final_rehashed_combined_summary_or_policy_cannot_rewrite_original_approval() {
    use sha2::{Digest, Sha256};
    let (root, mut w, _) = configured();
    let c = w.save_client(client_input()).unwrap();
    let (a, _) = fixture(&mut w, &c, root.path(), "complete");
    let p = w.review_final_report(&c.id, &a.payload.id).unwrap();
    let approved = w.approve_final_report(&c.id, input(&p)).unwrap();
    let mut forged = approved.payload;
    forged
        .readiness
        .card_summary
        .as_mut()
        .unwrap()
        .combined_spending = "0.00".into();
    let json = serde_json::to_string(&forged).unwrap();
    let hash = format!("{:x}", Sha256::digest(json.as_bytes()));
    w.session()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER final_reports_no_update;")
        .unwrap();
    w.session()
        .unwrap()
        .connection
        .execute(
            "UPDATE final_report_approvals SET payload_json=?1,digest=?2 WHERE id=?3",
            params![json, hash, forged.id],
        )
        .unwrap();
    assert!(w.get_final_report(&c.id, &forged.id).is_err());
    let out = tempfile::tempdir().unwrap();
    let backup = w
        .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
        .unwrap();
    w.lock_workspace();
    assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
}
