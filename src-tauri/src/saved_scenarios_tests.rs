use super::super::{
    scenario_tests::{fixture, input},
    tests::{budget_input, client_input, PASSWORD},
};
use super::*;
use crate::scenarios;
fn proposal() -> scenarios::Input {
    let mut i = input();
    i.budget_changes = vec![
        scenarios::BudgetChange {
            index: 0,
            amount: Some("1600".into()),
        },
        scenarios::BudgetChange {
            index: 2,
            amount: Some("100".into()),
        },
    ];
    i.debt_payment = Some(scenarios::AmountChange {
        amount: Some("200".into()),
    });
    i.emergency_contribution = Some(scenarios::AmountChange {
        amount: Some("300".into()),
    });
    i
}
fn save(w: &mut Workspace, id: &str) -> Record {
    w.save_client_scenario(
        id,
        Input {
            expected_count: 0,
            proposal: proposal(),
        },
    )
    .unwrap()
}
fn report(w: &mut Workspace, id: &str, r: &Record) -> crate::assessments::Input {
    let client = w.get_client(id).unwrap();
    crate::assessments::Input {
        selected_scenario: Some(Selection {
            id: r.payload.id.clone(),
            digest: r.digest.clone(),
        }),
        supporting: None,
        planning: None,
        expected_revision: 0,
        expected_profile_updated_at: client.updated_at,
        from_month: "2026-09".into(),
        through_month: "2026-09".into(),
        title: "Selected proposal report".into(),
        findings: "Synthetic basis; coverage incomplete".into(),
        recommendations: "Review assumptions".into(),
        follow_up: "Discuss model differences".into(),
        acknowledge: true,
    }
}
fn json<T: Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap()
}
#[test]
fn recurring_expense_scenarios_preserve_legacy_bytes_report_dates_and_recovery_provenance() {
    use super::super::scenario_tests::{expense, expense_dates};
    let (root, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let legacy = save(&mut w, &id);
    let legacy_bytes = serde_json::to_vec(&legacy.payload).unwrap();
    assert!(json(&legacy.payload)["comparison"]["input"]
        .get("recurringExpenses")
        .is_none());
    let decoded: Payload = serde_json::from_slice(&legacy_bytes).unwrap();
    saved_scenarios::validate(&decoded, &id).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), legacy_bytes);
    let mut proposal = input();
    proposal.label = "Recurring care expense".into();
    let mut e = expense();
    e.dated = Some(expense_dates());
    proposal.recurring_expenses.push(e);
    let record = w
        .save_client_scenario(
            &id,
            Input {
                expected_count: 1,
                proposal,
            },
        )
        .unwrap();
    assert_eq!(
        record
            .payload
            .comparison
            .proposed
            .first_month_buffer
            .as_deref(),
        Some("1625.00")
    );
    let i = report(&mut w, &id, &record);
    let a = w.save_assessment_record(&id, i).unwrap();
    assert_eq!(a.payload.assessment.operating_cashflow, "0.00");
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    let out = tempfile::tempdir().unwrap();
    let pdf = w
        .export_assessment_pdf(
            &id,
            &a.payload.id,
            out.path().join("expense.pdf").to_str().unwrap(),
        )
        .unwrap();
    let d = lopdf::Document::load(pdf.path).unwrap();
    let text = d
        .extract_text(&d.get_pages().keys().copied().collect::<Vec<_>>())
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for value in ["New proposed recurring expense: Proposed care / Necessary / USD 75.00 per Monthly / estimated", "Synthetic checking / first 2026-11-15", "Baseline USD 1700.00 / proposed USD 1625.00", "Baseline USD 3300.00 / proposed USD 3225.00"]{
        assert!(text.contains(value),"Missing {value}");
    }
    for part in ["amount", "account", "date", "outcome"] {
        let mut forged = json(&record.payload);
        let e = &mut forged["comparison"]["input"]["recurringExpenses"][0];
        match part {
            "amount" => e["amount"] = "76.00".into(),
            "account" => e["dated"]["accountId"] = "foreign".into(),
            "date" => e["dated"]["firstDate"] = "2026-11-16".into(),
            _ => forged["comparison"]["proposed"]["firstMonthBuffer"] = "9999.99".into(),
        }
        let forged: Payload = serde_json::from_value(forged).unwrap();
        assert!(saved_scenarios::validate(&forged, &id).is_err());
    }
    let mut budget = budget_input(1);
    budget.lines = w.get_client_budget(&id).unwrap().lines;
    budget.lines[0].amount = Some("1600".into());
    w.save_client_budget(&id, budget).unwrap();
    assert!(
        w.get_client_scenario(&id, &record.payload.id)
            .unwrap()
            .needs_review
    );
    let backup = w
        .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
        .unwrap();
    w.lock_workspace();
    let mut reopened = Workspace::new(root.path().into());
    reopened.unlock_workspace(PASSWORD.into()).unwrap();
    assert_eq!(
        reopened
            .get_assessment_record(&id, &a.payload.id)
            .unwrap()
            .digest,
        a.digest
    );
    reopened.lock_workspace();
    let review = reopened
        .prepare_restore(backup.path, PASSWORD.into())
        .unwrap();
    reopened
        .confirm_restore(review.token, "RESTORE".into())
        .unwrap();
    reopened.unlock_workspace(PASSWORD.into()).unwrap();
    for original in [&legacy, &record] {
        let restored = reopened
            .get_client_scenario(&id, &original.payload.id)
            .unwrap();
        assert_eq!(restored.digest, original.digest);
        assert_eq!(json(&restored.payload), json(&original.payload));
        assert!(restored.needs_review);
    }
    let restored = reopened.get_assessment_record(&id, &a.payload.id).unwrap();
    assert_eq!(restored.scenario.unwrap().digest, record.digest);
    assert!(restored.needs_review);
}
#[test]
fn extended_and_legacy_scenarios_keep_canonical_history_reports_and_encrypted_recovery() {
    let (root, mut w, id) = fixture();
    let legacy = save(&mut w, &id);
    let old_report = report(&mut w, &id, &legacy);
    let old_report = w.save_assessment_record(&id, old_report).unwrap();
    let legacy_bytes = serde_json::to_vec(&legacy.payload).unwrap();
    let legacy_json = json(&legacy.payload);
    assert!(legacy_json["comparison"]["input"]
        .get("debtExtras")
        .is_none());
    assert!(legacy_json["comparison"]["input"]
        .get("goalContributions")
        .is_none());
    assert_eq!(legacy.payload.comparison.warnings.len(), 6);
    let decoded: Payload = serde_json::from_slice(&legacy_bytes).unwrap();
    saved_scenarios::validate(&decoded, &id).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), legacy_bytes);
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let mut p = input();
    p.label = "Extra loan and goal proposal".into();
    p.debt_extras = vec![crate::debt::DatedAmount {
        month: "2026-11".into(),
        amount: "100".into(),
    }];
    p.goal_contributions = vec![scenarios::GoalContributionChange {
        index: 0,
        amount: Some("100".into()),
    }];
    let new = w
        .save_client_scenario(
            &id,
            Input {
                expected_count: 1,
                proposal: p,
            },
        )
        .unwrap();
    assert_eq!(
        new.payload
            .comparison
            .proposed
            .first_month_buffer
            .as_deref(),
        Some("1550.00")
    );
    let i = report(&mut w, &id, &new);
    let a = w.save_assessment_record(&id, i).unwrap();
    assert_eq!(a.payload.assessment.operating_cashflow, "0.00");
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    for changed in ["extra", "goal", "index"] {
        let mut forged = json(&new.payload);
        match changed {
            "extra" => forged["comparison"]["input"]["debtExtras"][0]["amount"] = "101.00".into(),
            "goal" => {
                forged["comparison"]["proposed"]["plans"]["savings"]["data"]["goals"][0]
                    ["monthlyContribution"] = "999.00".into()
            }
            _ => forged["comparison"]["input"]["goalContributions"][0]["index"] = 99.into(),
        }
        let forged: Payload = serde_json::from_value(forged).unwrap();
        assert!(saved_scenarios::validate(&forged, &id).is_err());
    }
    let out = tempfile::tempdir().unwrap();
    let pdf = w
        .export_assessment_pdf(
            &id,
            &a.payload.id,
            out.path().join("extended.pdf").to_str().unwrap(),
        )
        .unwrap();
    let d = lopdf::Document::load(pdf.path).unwrap();
    let text = d
        .extract_text(&d.get_pages().keys().copied().collect::<Vec<_>>())
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for value in [
        "Additional proposed debt payment: 2026-11 / USD 100.00",
        "Proposed goal monthly contribution: Synthetic insurance / USD 100.00",
        "Baseline USD 1700.00 / proposed USD 1550.00",
        "2026-12-31",
    ] {
        assert!(text.contains(value), "Missing {value}");
    }
    let mut budget = budget_input(1);
    budget.lines = w.get_client_budget(&id).unwrap().lines;
    budget.lines[0].amount = Some("1600".into());
    w.save_client_budget(&id, budget).unwrap();
    let backup = w
        .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
        .unwrap();
    w.lock_workspace();
    let mut reopened = Workspace::new(root.path().into());
    reopened.unlock_workspace(PASSWORD.into()).unwrap();
    assert!(
        reopened
            .get_client_scenario(&id, &new.payload.id)
            .unwrap()
            .needs_review
    );
    reopened.lock_workspace();
    let review = reopened
        .prepare_restore(backup.path, PASSWORD.into())
        .unwrap();
    reopened
        .confirm_restore(review.token, "RESTORE".into())
        .unwrap();
    reopened.unlock_workspace(PASSWORD.into()).unwrap();
    for (record, report) in [(&legacy, &old_report), (&new, &a)] {
        let restored = reopened
            .get_client_scenario(&id, &record.payload.id)
            .unwrap();
        assert_eq!(restored.digest, record.digest);
        assert_eq!(json(&restored.payload), json(&record.payload));
        assert!(restored.needs_review);
        let linked = reopened
            .get_assessment_record(&id, &report.payload.id)
            .unwrap();
        assert_eq!(linked.digest, report.digest);
        assert_eq!(linked.scenario.unwrap().digest, record.digest);
        assert!(linked.needs_review);
    }
}
#[test]
fn saved_scenario_and_report_keep_exact_baseline_proposal_and_live_data_separate() {
    let (_dir, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let r = save(&mut w, &id);
    assert_eq!(r.payload.comparison.calculator, "combined-scenario-v1");
    assert_eq!(
        r.payload.comparison.proposed.first_month_buffer.as_deref(),
        Some("1816.67")
    );
    assert_eq!(
        r.payload
            .comparison
            .proposed
            .plans
            .debt
            .comparison
            .as_ref()
            .unwrap()
            .snowball
            .as_ref()
            .unwrap()
            .total_interest,
        "23.01"
    );
    assert_eq!(
        r.payload
            .comparison
            .proposed
            .plans
            .savings
            .summary
            .as_ref()
            .unwrap()
            .emergency
            .completion_date
            .as_deref(),
        Some("2027-05-31")
    );
    let note = w
        .add_client_note(
            &id,
            "Synthetic counseling note USD 9999.99 is narrative only",
        )
        .unwrap();
    let mut report = report(&mut w, &id, &r);
    report.supporting = Some(crate::report_evidence::Selection {
        expected_notes_count: 1,
        note_ids: vec![note.id],
    });
    let a = w.save_assessment_record(&id, report).unwrap();
    assert!(a.payload.supporting.is_some());
    assert_eq!(a.payload.format, 4);
    assert!(a.scenario.is_some());
    assert_eq!(
        a.payload
            .planning
            .as_ref()
            .unwrap()
            .plans
            .budget
            .summary
            .monthly_income,
        "3466.67"
    );
    assert_eq!(a.payload.assessment.operating_cashflow, "0.00");
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    let out = tempfile::tempdir().unwrap();
    let pdf = w
        .export_assessment_pdf(
            &id,
            &a.payload.id,
            out.path().join("scenario.pdf").to_str().unwrap(),
        )
        .unwrap();
    let d = lopdf::Document::load(pdf.path).unwrap();
    let text = d
        .extract_text(&d.get_pages().keys().copied().collect::<Vec<_>>())
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for value in [
        "Selected saved combined scenario / hypothetical",
        "Baseline USD 3250.00 / proposed USD 3466.67",
        "Baseline USD 1700.00 / proposed USD 1816.67",
        "Projected total interest: USD 23.01",
        "Primary emergency completion: 2027-05-31",
    ] {
        assert!(text.contains(value), "Missing {value}: {text}");
    }
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    let count: i64 = w
        .session
        .as_ref()
        .unwrap()
        .connection
        .query_row(
            "SELECT COUNT(*) FROM audit_events WHERE event='scenario_recorded'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
#[test]
fn scenario_history_versions_count_scope_and_failed_audit_prevent_replay_and_writes() {
    let (_dir, mut w, id) = fixture();
    let c = &w.session.as_ref().unwrap().connection;
    c.execute_batch("CREATE TRIGGER synthetic_scenario_audit BEFORE INSERT ON audit_events WHEN NEW.event='scenario_recorded' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
    assert!(w
        .save_client_scenario(
            &id,
            Input {
                expected_count: 0,
                proposal: proposal()
            }
        )
        .is_err());
    assert!(w.list_client_scenarios(&id).unwrap().records.is_empty());
    w.session
        .as_ref()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER synthetic_scenario_audit;")
        .unwrap();
    let r = save(&mut w, &id);
    assert!(w
        .save_client_scenario(
            &id,
            Input {
                expected_count: 0,
                proposal: proposal()
            }
        )
        .err()
        .unwrap()
        .contains("history changed"));
    for index in 0..4 {
        let mut p = proposal();
        match index {
            0 => p.expected_revisions.budget = 0,
            1 => p.expected_revisions.debt = 0,
            2 => p.expected_revisions.savings = 0,
            _ => p.expected_revisions.cashflow = 0,
        };
        assert!(w
            .save_client_scenario(
                &id,
                Input {
                    expected_count: 1,
                    proposal: p
                }
            )
            .is_err());
    }
    let other = w.save_client(client_input()).unwrap();
    assert!(w.get_client_scenario(&other.id, &r.payload.id).is_err());
    assert!(w
        .get_client_scenario("foreign-client", &r.payload.id)
        .is_err());
    assert!(w
        .list_client_scenarios(&other.id)
        .unwrap()
        .records
        .is_empty());
    assert!(w
        .session
        .as_ref()
        .unwrap()
        .connection
        .execute("DELETE FROM client_scenarios WHERE id=?1", [&r.payload.id])
        .is_err());
    assert!(w
        .session
        .as_ref()
        .unwrap()
        .connection
        .execute(
            "UPDATE client_scenarios SET label='changed' WHERE id=?1",
            [&r.payload.id]
        )
        .is_err());
    assert_eq!(w.list_client_scenarios(&id).unwrap().records.len(), 1);
    w.lock_workspace();
    assert!(w.get_client_scenario(&id, &r.payload.id).is_err());
}
#[test]
fn later_edits_flag_history_and_prevent_new_report_selection_without_rewriting_records() {
    let (_dir, mut w, id) = fixture();
    let r = save(&mut w, &id);
    let i = report(&mut w, &id, &r);
    let a = w.save_assessment_record(&id, i).unwrap();
    let frozen = json(&r.payload);
    let captured = json(&a.payload);
    let mut b = budget_input(1);
    b.lines = w.get_client_budget(&id).unwrap().lines;
    b.lines[0].amount = Some("1601".into());
    w.save_client_budget(&id, b).unwrap();
    let old = w.get_client_scenario(&id, &r.payload.id).unwrap();
    assert!(old.needs_review);
    assert_eq!(old.digest, r.digest);
    assert_eq!(json(&old.payload), frozen);
    let old = w.get_assessment_record(&id, &a.payload.id).unwrap();
    assert!(old.needs_review);
    assert_eq!(json(&old.payload), captured);
    let i = report(&mut w, &id, &r);
    assert!(w
        .save_assessment_record(&id, i)
        .err()
        .unwrap()
        .contains("baseline needs review"));
    let mut i = report(&mut w, &id, &r);
    i.selected_scenario.as_mut().unwrap().digest = "0".repeat(64);
    assert!(w.save_assessment_record(&id, i).is_err());
}
#[test]
fn restart_and_encrypted_recovery_validate_and_restore_scenarios_and_report_links() {
    let (root, mut w, id) = fixture();
    let r = save(&mut w, &id);
    let i = report(&mut w, &id, &r);
    let a = w.save_assessment_record(&id, i).unwrap();
    let out = tempfile::tempdir().unwrap();
    let backup = w
        .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
        .unwrap();
    w.lock_workspace();
    let mut reopened = Workspace::new(root.path().into());
    reopened.unlock_workspace(PASSWORD.into()).unwrap();
    assert_eq!(
        reopened
            .get_client_scenario(&id, &r.payload.id)
            .unwrap()
            .digest,
        r.digest
    );
    reopened
        .save_client_scenario(
            &id,
            Input {
                expected_count: 1,
                proposal: proposal(),
            },
        )
        .unwrap();
    reopened.lock_workspace();
    let review = reopened
        .prepare_restore(backup.path, PASSWORD.into())
        .unwrap();
    reopened
        .confirm_restore(review.token, "RESTORE".into())
        .unwrap();
    reopened.unlock_workspace(PASSWORD.into()).unwrap();
    assert_eq!(
        reopened.list_client_scenarios(&id).unwrap().records.len(),
        1
    );
    let restored = reopened.get_assessment_record(&id, &a.payload.id).unwrap();
    assert_eq!(restored.scenario.unwrap().digest, r.digest);
    assert_eq!(restored.digest, a.digest);
}
#[test]
fn forged_outcomes_or_broken_report_links_are_rejected_before_recovery_activation() {
    let (_root, mut w, id) = fixture();
    let r = save(&mut w, &id);
    let i = report(&mut w, &id, &r);
    let a = w.save_assessment_record(&id, i).unwrap();
    let mut fake = json(&r.payload);
    fake["comparison"]["proposed"]["firstMonthBuffer"] = serde_json::json!("999999.99");
    let text = serde_json::to_string(&fake).unwrap();
    let digest = hash(&text);
    w.session
        .as_ref()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER scenarios_no_update;")
        .unwrap();
    w.session
        .as_ref()
        .unwrap()
        .connection
        .execute(
            "UPDATE client_scenarios SET payload_json=?1,digest=?2 WHERE id=?3",
            params![text, digest, r.payload.id],
        )
        .unwrap();
    assert!(w.get_client_scenario(&id, &r.payload.id).is_err());
    assert!(w.get_assessment_record(&id, &a.payload.id).is_err());
    let out = tempfile::tempdir().unwrap();
    let backup = w
        .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
        .unwrap();
    w.lock_workspace();
    assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
}
#[test]
fn schema_fifteen_upgrade_and_failed_upgrade_preserve_original_models_and_audits() {
    let (_dir, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let client = w.get_client(&id).unwrap();
    let legacy = w
        .save_assessment_record(
            &id,
            crate::assessments::Input {
                selected_scenario: None,
                supporting: None,
                planning: None,
                expected_revision: 0,
                expected_profile_updated_at: client.updated_at,
                from_month: "2026-09".into(),
                through_month: "2026-09".into(),
                title: "Legacy report before upgrade".into(),
                findings: "".into(),
                recommendations: "".into(),
                follow_up: "".into(),
                acknowledge: true,
            },
        )
        .unwrap();
    let conn = &mut w.session.as_mut().unwrap().connection;
    conn.execute_batch(
        "DROP TABLE assessment_scenario_links;DROP TABLE client_scenarios;PRAGMA user_version=15;",
    )
    .unwrap();
    migrate(conn, false).unwrap();
    assert_eq!(
        conn.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        16
    );
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    assert_eq!(
        w.get_assessment_record(&id, &legacy.payload.id)
            .unwrap()
            .digest,
        legacy.digest
    );
    let conn = &mut w.session.as_mut().unwrap().connection;
    conn.execute_batch("DROP TABLE assessment_scenario_links;DROP TABLE client_scenarios;CREATE TABLE client_scenarios(broken TEXT);PRAGMA user_version=15;").unwrap();
    assert!(migrate(conn, false).is_err());
    assert_eq!(
        conn.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        15
    );
    let links: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='assessment_scenario_links'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(links, 0);
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
}

#[test]
fn failed_report_audit_rolls_back_its_scenario_link_and_record() {
    let (_dir, mut w, id) = fixture();
    let r = save(&mut w, &id);
    let i = report(&mut w, &id, &r);
    w.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER synthetic_report_audit BEFORE INSERT ON audit_events WHEN NEW.event='assessment_recorded' BEGIN SELECT RAISE(ABORT,'Synthetic report audit failure'); END;").unwrap();
    assert!(w.save_assessment_record(&id, i).is_err());
    let c = &w.session.as_ref().unwrap().connection;
    for table in ["client_assessment_records", "assessment_scenario_links"] {
        let n: i64 = c
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
    assert_eq!(w.list_client_scenarios(&id).unwrap().records.len(), 1);
}
#[test]
fn history_limits_and_missing_projection_terms_do_not_create_false_report_outcomes() {
    let (_dir, mut w, id) = fixture();
    let mut p = proposal();
    p.debt_payment = Some(scenarios::AmountChange { amount: None });
    let r = w
        .save_client_scenario(
            &id,
            Input {
                expected_count: 0,
                proposal: p,
            },
        )
        .unwrap();
    let i = report(&mut w, &id, &r);
    assert!(w.save_assessment_record(&id, i).is_err());
    assert!(r.payload.comparison.proposed.first_month_buffer.is_none());
    let mut forged =
        serde_json::json!({"expectedCount":1,"proposal":serde_json::to_value(proposal()).unwrap()});
    forged["projectedOutcomes"] = serde_json::json!({"income":"999999"});
    assert!(serde_json::from_value::<Input>(forged).is_err());
    let c = &w.session.as_ref().unwrap().connection;
    for _ in 1..50 {
        let mut payload: saved_scenarios::Payload = saved_scenarios::copy(&r.payload).unwrap();
        payload.id = Uuid::new_v4().to_string();
        let json = serde_json::to_string(&payload).unwrap();
        c.execute("INSERT INTO client_scenarios(id,client_id,organization_id,counselor_id,label,created_at,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![payload.id,id,payload.workspace.organization_id,payload.workspace.counselor_id,payload.comparison.input.label,payload.comparison.created_at,json,hash(&json)]).unwrap();
    }
    assert!(w
        .save_client_scenario(
            &id,
            Input {
                expected_count: 50,
                proposal: proposal()
            }
        )
        .err()
        .unwrap()
        .contains("safety limit"));
    let count: i64 = w
        .session
        .as_ref()
        .unwrap()
        .connection
        .query_row("SELECT COUNT(*) FROM client_scenarios", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 50);
}
