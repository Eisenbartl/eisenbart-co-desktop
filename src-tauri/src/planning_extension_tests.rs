use super::tests::{client_input, PASSWORD};
use super::*;
#[test]
fn schema32_upgrade_preserves_old_canonical_plans_and_scenarios_and_is_idempotent() {
    let (root, mut w, client) = super::scenario_tests::fixture();
    let record = w
        .save_client_scenario(
            &client,
            crate::saved_scenarios::Input {
                expected_count: 0,
                proposal: super::scenario_tests::input(),
            },
        )
        .unwrap();
    let plans = serde_json::to_vec(&w.get_assessment_planning(&client).unwrap()).unwrap();
    let payload = serde_json::to_vec(&record.payload).unwrap();
    assert_eq!(record.payload.comparison.calculator, "combined-scenario-v1");
    let session = w.session.as_ref().unwrap();
    let audit: i64 = session
        .connection
        .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
        .unwrap();
    session
        .connection
        .pragma_update(None, "user_version", 32)
        .unwrap();
    w.lock_workspace();
    for _ in 0..2 {
        let mut reopened = Workspace::new(root.path().into());
        reopened.unlock_workspace(PASSWORD.into()).unwrap();
        let session = reopened.session.as_ref().unwrap();
        assert_eq!(
            session
                .connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            33
        );
        assert_eq!(
            session
                .connection
                .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            audit
        );
        assert_eq!(
            serde_json::to_vec(&reopened.get_assessment_planning(&client).unwrap()).unwrap(),
            plans
        );
        assert_eq!(
            serde_json::to_vec(
                &reopened
                    .get_client_scenario(&client, &record.payload.id)
                    .unwrap()
                    .payload
            )
            .unwrap(),
            payload
        );
        reopened.lock_workspace();
    }
}
#[test]
fn verified_progress_reads_are_client_scoped_and_do_not_change_any_financial_record() {
    let (_root, mut w, client) = super::scenario_tests::fixture();
    let other = w.save_client(client_input()).unwrap();
    let query = crate::dashboard::Input {
        from_month: "2026-09".into(),
        through_month: "2026-11".into(),
    };
    let before = serde_json::to_value(w.get_assessment_planning(&client).unwrap()).unwrap();
    assert!(w.get_savings_progress("missing-client", &query).is_err());
    assert!(w
        .get_savings_progress(&client, &query)
        .unwrap()
        .points
        .is_empty());
    assert!(w
        .get_savings_progress(&other.id, &query)
        .unwrap()
        .points
        .is_empty());
    assert!(w
        .get_savings_progress(
            &client,
            &crate::dashboard::Input {
                from_month: "2026-11".into(),
                through_month: "2026-10".into()
            }
        )
        .is_err());
    assert_eq!(
        serde_json::to_value(w.get_assessment_planning(&client).unwrap()).unwrap(),
        before
    );
    w.lock_workspace();
    assert!(w.get_savings_progress(&client, &query).is_err());
}
