use super::tests::{
    budget_input, cashflow_input, client_input, configured, debt_input, savings_input,
};
use crate::scenarios::*;
use crate::{budget, planning};
use serde::Serialize;
fn audit_count(w: &super::Workspace) -> i64 {
    w.session
        .as_ref()
        .unwrap()
        .connection
        .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
        .unwrap()
}
pub(super) fn fixture() -> (tempfile::TempDir, crate::storage::Workspace, String) {
    let (dir, mut w, _) = configured();
    let client = w.save_client(client_input()).unwrap();
    let mut budget = budget_input(0);
    budget.lines.push(budget::BudgetLine {
        label: "Essential bills".into(),
        kind: budget::Kind::Necessary,
        amount: Some("1000".into()),
        frequency: budget::Frequency::Monthly,
        estimated: false,
    });
    budget.lines.push(budget::BudgetLine {
        label: "Optional spending".into(),
        kind: budget::Kind::Discretionary,
        amount: Some("200".into()),
        frequency: budget::Frequency::Monthly,
        estimated: false,
    });
    w.save_client_budget(&client.id, budget).unwrap();
    w.save_client_debt_plan(&client.id, debt_input(0)).unwrap();
    w.save_client_savings_plan(&client.id, savings_input(0))
        .unwrap();
    w.save_client_cashflow_plan(&client.id, cashflow_input(0))
        .unwrap();
    (dir, w, client.id)
}
pub(super) fn input() -> Input {
    Input {
        expected_revisions: planning::Revisions {
            budget: 1,
            debt: 1,
            savings: 1,
            cashflow: 1,
        },
        label: "Temporary proposal".into(),
        assumptions: "Synthetic only".into(),
        baseline_strategy: planning::Strategy::Avalanche,
        proposed_strategy: planning::Strategy::Snowball,
        budget_changes: vec![],
        debt_payment: None,
        emergency_contribution: None,
        cash_changes: vec![],
        debt_extras: vec![],
        goal_contributions: vec![],
        recurring_expenses: vec![],
        income_reductions: vec![],
        new_goals: vec![],
        targeted_extras: vec![],
        savings_funding: None,
    }
}
fn json<T: Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap()
}
pub(super) fn income_reduction() -> crate::income_periods::Reduction {
    crate::income_periods::Reduction {
        budget_index: 0,
        from_month: "2026-11".into(),
        through_month: "2026-11".into(),
        amount: Some("800".into()),
        cash_schedule: Some(1),
    }
}
#[test]
fn income_periods_preserve_normal_models_and_paycheck_cadence_then_restore_normal_receipts() {
    let (_dir, mut w, id) = fixture();
    let mut cash = cashflow_input(1);
    cash.data.end_date = "2026-12-31".into();
    w.save_client_cashflow_plan(&id, cash).unwrap();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let audits = audit_count(&w);
    let mut i = input();
    i.expected_revisions.cashflow = 2;
    i.income_reductions.push(income_reduction());
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    let m = &r.income_periods;
    assert_eq!(m.len(), 2);
    assert!(m[0].reduced);
    assert!(!m[1].reduced);
    assert_eq!(m[0].normal_income.as_deref(), Some("3250.00"));
    assert_eq!(m[0].income.as_deref(), Some("1733.33"));
    assert_eq!(m[0].normal_buffer.as_deref(), Some("1700.00"));
    assert_eq!(m[0].buffer.as_deref(), Some("183.33"));
    assert_eq!(m[1].month, "2026-12");
    assert_eq!(m[1].income.as_deref(), Some("3250.00"));
    assert_eq!(m[1].buffer.as_deref(), Some("1700.00"));
    assert_eq!(r.proposed.plans.budget.summary.monthly_income, "3250.00");
    assert_eq!(json(&r.proposed.plans.debt), before["debt"]);
    assert_eq!(json(&r.proposed.plans.savings), before["savings"]);
    let c = r.proposed.plans.cashflow.summary.as_ref().unwrap();
    assert_eq!(c.combined_closing.as_deref(), Some("4900.00"));
    let pay: Vec<_> = c
        .ledger
        .iter()
        .filter(|e| e.kind == crate::cashflow::Kind::Paycheck)
        .map(|e| (e.date.as_str(), e.amount.as_deref()))
        .collect();
    assert_eq!(
        pay,
        vec![
            ("2026-11-06", Some("800.00")),
            ("2026-11-20", Some("800.00")),
            ("2026-12-04", Some("1500.00")),
            ("2026-12-18", Some("1500.00"))
        ]
    );
    assert_eq!(
        r.input.income_reductions[0].amount.as_deref(),
        Some("800.00")
    );
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    assert_eq!(audit_count(&w), audits);
}
#[test]
fn income_periods_keep_zero_unknown_deficits_and_completed_goal_commitments_explicit() {
    let (_dir, mut w, id) = fixture();
    for (amount, income, buffer) in [
        (None, None, None),
        (Some("0"), Some("0.00"), Some("-1550.00")),
        (Some("400"), Some("866.67"), Some("-683.33")),
    ] {
        let mut i = input();
        let mut reduction = income_reduction();
        reduction.amount = amount.map(str::to_owned);
        i.income_reductions.push(reduction);
        let r = w.compare_financial_scenarios(&id, i).unwrap();
        assert_eq!(r.income_periods[0].income.as_deref(), income);
        assert_eq!(r.income_periods[0].buffer.as_deref(), buffer);
        if amount.is_none() {
            assert!(r
                .proposed
                .plans
                .cashflow
                .summary
                .unwrap()
                .combined_closing
                .is_none());
            assert!(r
                .warnings
                .iter()
                .any(|w| w.contains("check is unavailable")));
        } else {
            assert!(r.warnings.iter().any(|w| w.contains("does not fund")));
        }
    }
    let mut i = input();
    let mut reduction = income_reduction();
    reduction.cash_schedule = None;
    reduction.from_month = "2027-09".into();
    reduction.through_month = "2027-09".into();
    i.income_reductions.push(reduction);
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert_eq!(
        r.income_periods[0].savings_reserved.as_deref(),
        Some("0.00")
    );
    assert_eq!(r.income_periods[0].buffer.as_deref(), Some("433.33"));
    assert_eq!(
        r.proposed
            .plans
            .cashflow
            .summary
            .unwrap()
            .combined_closing
            .as_deref(),
        Some("3300.00")
    );
    let mut i = input();
    let mut reduction = income_reduction();
    reduction.cash_schedule = None;
    reduction.from_month = "2076-11".into();
    reduction.through_month = "2076-11".into();
    i.income_reductions.push(reduction);
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert!(r.income_periods[0].buffer.is_none());
}
#[test]
fn income_periods_reject_overlap_forgery_invalid_ranges_and_conflicting_or_excessive_dated_changes()
{
    let (_dir, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let audits = audit_count(&w);
    for key in [
        "line",
        "range",
        "order",
        "length",
        "amount",
        "increase",
        "precision",
        "negative",
        "schedule",
    ] {
        let mut i = input();
        let mut r = income_reduction();
        match key {
            "line" => r.budget_index = 1,
            "range" => r.from_month = "2026-13".into(),
            "order" => r.through_month = "2026-10".into(),
            "length" => r.through_month = "2028-11".into(),
            "amount" => r.amount = Some("1500".into()),
            "increase" => r.amount = Some("1600".into()),
            "precision" => r.amount = Some("12.345".into()),
            "negative" => r.amount = Some("-1".into()),
            _ => r.cash_schedule = Some(0),
        }
        i.income_reductions.push(r);
        assert!(w.compare_financial_scenarios(&id, i).is_err());
    }
    let mut i = input();
    i.income_reductions = vec![income_reduction(), income_reduction()];
    assert!(w
        .compare_financial_scenarios(&id, i)
        .unwrap_err()
        .contains("overlap"));
    let mut i = input();
    i.income_reductions = (0..7).map(|_| income_reduction()).collect();
    assert!(w.compare_financial_scenarios(&id, i).is_err());
    let mut i = input();
    i.income_reductions.push(income_reduction());
    i.cash_changes.push(CashChange {
        settlement_change: None,
        index: 1,
        amount: Some("1".into()),
        first_date: "2026-11-06".into(),
    });
    assert!(w
        .compare_financial_scenarios(&id, i)
        .unwrap_err()
        .contains("also edit"));
    let mut raw = json(&input());
    raw["incomeReductions"] = serde_json::json!([{"budgetIndex":0,"fromMonth":"2026-11","throughMonth":"2026-11","amount":"800","cashSchedule":null,"buffer":"9999"}]);
    assert!(serde_json::from_value::<Input>(raw).is_err());
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    assert_eq!(audit_count(&w), audits);
    let mut cash = cashflow_input(1);
    cash.data.schedules[1].frequency = crate::cashflow::Frequency::Monthly;
    w.save_client_cashflow_plan(&id, cash).unwrap();
    let mut i = input();
    i.expected_revisions.cashflow = 2;
    i.income_reductions.push(income_reduction());
    assert!(w
        .compare_financial_scenarios(&id, i)
        .unwrap_err()
        .contains("frequency"));
    let mut cash = cashflow_input(2);
    cash.data.end_date = "2027-10-31".into();
    cash.data
        .schedules
        .resize(99, cash.data.schedules[0].clone());
    w.save_client_cashflow_plan(&id, cash).unwrap();
    let mut i = input();
    i.expected_revisions.cashflow = 3;
    i.income_reductions.push(income_reduction());
    assert!(w
        .compare_financial_scenarios(&id, i)
        .unwrap_err()
        .contains("100 dated schedules"));
}
pub(super) fn expense() -> RecurringExpense {
    RecurringExpense {
        label: " Proposed care ".into(),
        kind: budget::Kind::Necessary,
        amount: Some("75".into()),
        frequency: budget::Frequency::Monthly,
        dated: None,
    }
}
pub(super) fn expense_dates() -> ExpenseDates {
    ExpenseDates {
        account_id: "checking".into(),
        first_date: "2026-11-15".into(),
        second_day: None,
        until_date: None,
    }
}
#[test]
fn recurring_expenses_recalculate_normalized_contexts_and_only_explicit_estimated_bills_without_writes(
) {
    let (_dir, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let audits = audit_count(&w);
    for (frequency, monthly) in [
        (budget::Frequency::Weekly, "52.04"),
        (budget::Frequency::Biweekly, "26.02"),
        (budget::Frequency::Semimonthly, "24.02"),
        (budget::Frequency::Monthly, "12.01"),
        (budget::Frequency::Annual, "1.00"),
    ] {
        let mut i = input();
        let mut e = expense();
        e.amount = Some(" 12.01 ".into());
        e.frequency = frequency;
        i.recurring_expenses.push(e);
        let r = w.compare_financial_scenarios(&id, i).unwrap();
        assert_eq!(
            r.proposed.plans.budget.summary.monthly_amounts[3].as_deref(),
            Some(monthly)
        );
        assert_eq!(r.input.recurring_expenses[0].label, "Proposed care");
        assert_eq!(
            r.input.recurring_expenses[0].amount.as_deref(),
            Some("12.01")
        );
        assert!(r.proposed.plans.budget.lines[3].estimated);
        assert_eq!(
            json(&r.proposed.plans.cashflow.data),
            before["cashflow"]["data"]
        );
    }
    let mut i = input();
    let mut e = expense();
    e.amount = Some("12.01".into());
    e.frequency = budget::Frequency::Weekly;
    let mut dates = expense_dates();
    dates.first_date = "2026-11-05".into();
    e.dated = Some(dates);
    i.recurring_expenses.push(e);
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    let p = &r.proposed.plans;
    // Weekly 12.01 * 52/12 rounds to 52.04; four actual November bills total 48.04.
    assert_eq!(p.budget.summary.necessary_expenses, "1052.04");
    assert_eq!(p.debt.budget_context.monthly_capacity, "1997.96");
    assert_eq!(
        p.savings
            .summary
            .as_ref()
            .unwrap()
            .emergency_target
            .as_deref(),
        Some("3156.12")
    );
    assert_eq!(r.proposed.first_month_buffer.as_deref(), Some("1647.96"));
    assert_eq!(
        p.cashflow
            .summary
            .as_ref()
            .unwrap()
            .combined_closing
            .as_deref(),
        Some("3251.96")
    );
    let rows: Vec<_> = p
        .cashflow
        .summary
        .as_ref()
        .unwrap()
        .ledger
        .iter()
        .filter(|e| e.schedule_index == 3)
        .collect();
    assert_eq!(
        rows.iter().map(|e| e.date.as_str()).collect::<Vec<_>>(),
        vec!["2026-11-05", "2026-11-12", "2026-11-19", "2026-11-26"]
    );
    assert!(rows.iter().all(|e| e.kind == crate::cashflow::Kind::Bill
        && e.certainty == crate::cashflow::Certainty::Estimated
        && e.account_id == "checking"
        && e.destination_id.is_none()));
    assert_eq!(
        p.cashflow.summary.as_ref().unwrap().closing_balances[1]
            .amount
            .as_deref(),
        Some("1200.00")
    );
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    assert_eq!(audit_count(&w), audits);
}
#[test]
fn recurring_expenses_keep_unknown_zero_deficits_and_separate_forecast_windows_explicit() {
    let (_dir, mut w, id) = fixture();
    for (amount, buffer) in [
        (None, None),
        (Some("0"), Some("1700.00")),
        (Some("3000"), Some("-1300.00")),
    ] {
        let mut i = input();
        let mut e = expense();
        e.amount = amount.map(str::to_owned);
        i.recurring_expenses.push(e);
        let r = w.compare_financial_scenarios(&id, i).unwrap();
        assert_eq!(r.proposed.first_month_buffer.as_deref(), buffer);
        if amount.is_none() {
            assert!(r
                .proposed
                .plans
                .savings
                .summary
                .as_ref()
                .unwrap()
                .emergency_target
                .is_none());
        }
    }
    let mut i = input();
    let mut e = expense();
    e.amount = None;
    e.dated = Some(expense_dates());
    i.recurring_expenses.push(e);
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    let c = r.proposed.plans.cashflow.summary.unwrap();
    assert!(c.combined_closing.is_none());
    assert!(c.closing_balances[0].amount.is_none());
    assert_eq!(c.closing_balances[1].amount.as_deref(), Some("1200.00"));
    let mut i = input();
    let mut e = expense();
    let mut d = expense_dates();
    d.first_date = "2026-12-15".into();
    e.dated = Some(d);
    i.recurring_expenses.push(e);
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert_eq!(r.proposed.first_month_buffer.as_deref(), Some("1625.00"));
    assert_eq!(
        r.proposed
            .plans
            .cashflow
            .summary
            .unwrap()
            .combined_closing
            .as_deref(),
        Some("3300.00")
    );
    assert!(r.warnings.iter().any(|w| w.contains("no dated occurrence")));
    let mut i = input();
    let mut e = expense();
    e.amount = Some("12.01".into());
    e.frequency = budget::Frequency::Weekly;
    let mut d = expense_dates();
    d.first_date = "2026-11-05".into();
    d.until_date = Some("2026-11-15".into());
    e.dated = Some(d);
    i.recurring_expenses.push(e);
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert_eq!(r.proposed.first_month_buffer.as_deref(), Some("1647.96"));
    assert_eq!(
        r.proposed
            .plans
            .cashflow
            .summary
            .unwrap()
            .combined_closing
            .as_deref(),
        Some("3275.98")
    );
    let mut i = input();
    let mut e = expense();
    e.frequency = budget::Frequency::Semimonthly;
    e.amount = Some("10".into());
    let mut d = expense_dates();
    d.first_date = "2026-11-01".into();
    d.second_day = Some(15);
    e.dated = Some(d);
    i.recurring_expenses.push(e);
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert_eq!(
        r.proposed
            .plans
            .cashflow
            .summary
            .unwrap()
            .combined_closing
            .as_deref(),
        Some("3280.00")
    );
}
#[test]
fn recurring_expenses_reject_invalid_purposes_dates_accounts_precision_and_limits_without_partial_writes(
) {
    let (_dir, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let audits = audit_count(&w);
    let mut invalid = vec![];
    for kind in [
        budget::Kind::Income,
        budget::Kind::Savings,
        budget::Kind::DebtPayment,
    ] {
        let mut e = expense();
        e.kind = kind;
        invalid.push(e);
    }
    for value in ["12.345", "-1"] {
        let mut e = expense();
        e.amount = Some(value.into());
        invalid.push(e);
    }
    for label in ["".to_owned(), "x".repeat(121), "bad\nname".into()] {
        let mut e = expense();
        e.label = label;
        invalid.push(e);
    }
    let mut e = expense();
    e.frequency = budget::Frequency::Irregular;
    invalid.push(e);
    for key in ["account", "date", "until", "second", "annual"] {
        let mut e = expense();
        let mut d = expense_dates();
        match key {
            "account" => d.account_id = "other-client-account".into(),
            "date" => d.first_date = "2026-02-30".into(),
            "until" => d.until_date = Some("2026-11-01".into()),
            "second" => e.frequency = budget::Frequency::Semimonthly,
            _ => e.frequency = budget::Frequency::Annual,
        }
        e.dated = Some(d);
        invalid.push(e);
    }
    for e in invalid {
        let mut i = input();
        i.recurring_expenses.push(e);
        assert!(w.compare_financial_scenarios(&id, i).is_err());
    }
    let mut i = input();
    i.recurring_expenses = (0..21).map(|_| expense()).collect();
    assert!(w.compare_financial_scenarios(&id, i).is_err());
    let mut raw = json(&input());
    raw["recurringExpenses"] = serde_json::json!([{"label":"forged","kind":"necessary","amount":"1","frequency":"monthly","dated":null,"monthlyTotal":"0"}]);
    assert!(serde_json::from_value::<Input>(raw).is_err());
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    assert_eq!(audit_count(&w), audits);
    let mut budget = budget_input(1);
    budget.lines.resize(100, budget.lines[0].clone());
    w.save_client_budget(&id, budget).unwrap();
    let mut i = input();
    i.expected_revisions.budget = 2;
    i.recurring_expenses.push(expense());
    assert!(w
        .compare_financial_scenarios(&id, i)
        .unwrap_err()
        .contains("100 lines"));
    let mut cash = cashflow_input(1);
    cash.data
        .schedules
        .resize(100, cash.data.schedules[0].clone());
    w.save_client_cashflow_plan(&id, cash).unwrap();
    let mut i = input();
    i.expected_revisions.budget = 2;
    i.expected_revisions.cashflow = 2;
    let mut e = expense();
    e.dated = Some(expense_dates());
    i.recurring_expenses.push(e);
    // Restore room in the budget to reach the independent cash-flow capacity guard.
    w.save_client_budget(&id, budget_input(2)).unwrap();
    i.expected_revisions.budget = 3;
    assert!(w
        .compare_financial_scenarios(&id, i)
        .unwrap_err()
        .contains("100 activity"));
    let other = w.save_client(client_input()).unwrap();
    w.save_client_budget(&other.id, budget_input(0)).unwrap();
    let mut i = input();
    i.expected_revisions = w.get_assessment_planning(&other.id).unwrap().revisions();
    let mut e = expense();
    e.dated = Some(expense_dates());
    i.recurring_expenses.push(e);
    assert!(w
        .compare_financial_scenarios(&other.id, i)
        .unwrap_err()
        .contains("Save a cash-flow plan"));
}
#[test]
fn extra_payments_and_goal_contributions_recalculate_exactly_without_replacing_saved_commitments() {
    let (_dir, mut w, id) = fixture();
    let mut debt = debt_input(1);
    debt.data.accounts.truncate(1);
    debt.data.accounts[0].balance = Some("1000".into());
    debt.data.extras = vec![crate::debt::DatedAmount {
        month: "2026-12".into(),
        amount: "25".into(),
    }];
    w.save_client_debt_plan(&id, debt).unwrap();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let audits = audit_count(&w);
    let mut i = input();
    i.expected_revisions.debt = 2;
    i.debt_extras = vec![crate::debt::DatedAmount {
        month: "2026-11".into(),
        amount: " 150 ".into(),
    }];
    i.goal_contributions = vec![GoalContributionChange {
        index: 0,
        amount: Some("100".into()),
    }];
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    let p = &r.proposed.plans;
    let debt = p
        .debt
        .comparison
        .as_ref()
        .unwrap()
        .snowball
        .as_ref()
        .unwrap();
    // Zero-interest 1000: Nov 250, Dec 125, Jan-Jun 100 each, Jul 25.
    assert_eq!(debt.total_paid, "1000.00");
    assert_eq!(debt.total_interest, "0.00");
    assert_eq!(debt.payoff_month.as_deref(), Some("2027-07"));
    assert_eq!(debt.schedule[0].remaining_balance, "750.00");
    assert_eq!(debt.schedule[1].available_payment, "125.00");
    assert_eq!(p.debt.data.as_ref().unwrap().extras.len(), 2);
    assert_eq!(r.input.debt_extras[0].amount, "150.00");
    assert_eq!(
        r.input.goal_contributions[0].amount.as_deref(),
        Some("100.00")
    );
    // Remaining goal 1200 - reserved 1000: 4 * 50 versus 2 * 100.
    assert_eq!(
        r.baseline.plans.savings.summary.as_ref().unwrap().goals[0]
            .forecast
            .completion_date
            .as_deref(),
        Some("2027-02-28")
    );
    let s = p.savings.summary.as_ref().unwrap();
    assert_eq!(
        s.goals[0].forecast.completion_date.as_deref(),
        Some("2026-12-31")
    );
    assert_eq!(
        p.savings.data.as_ref().unwrap().goals[0]
            .reserved_amount
            .as_deref(),
        Some("1000.00")
    );
    assert_eq!(s.requested_monthly_savings.as_deref(), Some("300.00"));
    assert_eq!(r.baseline.first_month_buffer.as_deref(), Some("1700.00"));
    // 3250 - 1000 - 200 - (100 + 150 extra) - (200 + 100 goal).
    assert_eq!(r.proposed.first_month_buffer.as_deref(), Some("1500.00"));
    assert_eq!(
        p.cashflow
            .summary
            .as_ref()
            .unwrap()
            .combined_closing
            .as_deref(),
        Some("3300.00")
    );
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    assert_eq!(audit_count(&w), audits);
}

#[test]
fn new_proposal_controls_preserve_unknown_zero_and_overallocated_funding() {
    let (_dir, mut w, id) = fixture();
    for (value, status, buffer) in [
        (None, crate::savings::ForecastStatus::Unknown, None),
        (
            Some("0"),
            crate::savings::ForecastStatus::NoContribution,
            Some("1750.00"),
        ),
        // Actual first-month goal funding is capped at the remaining 200.
        (
            Some("2000"),
            crate::savings::ForecastStatus::Projected,
            Some("1550.00"),
        ),
    ] {
        let mut i = input();
        i.goal_contributions = vec![GoalContributionChange {
            index: 0,
            amount: value.map(str::to_owned),
        }];
        let r = w.compare_financial_scenarios(&id, i).unwrap();
        assert_eq!(
            r.proposed.plans.savings.summary.as_ref().unwrap().goals[0]
                .forecast
                .status,
            status
        );
        assert_eq!(r.proposed.first_month_buffer.as_deref(), buffer);
    }
    let mut i = input();
    i.debt_extras = vec![crate::debt::DatedAmount {
        month: "2026-11".into(),
        amount: "3000".into(),
    }];
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert_eq!(r.proposed.first_month_buffer.as_deref(), Some("-1300.00"));
    assert!(r
        .proposed
        .plans
        .debt
        .comparison
        .as_ref()
        .unwrap()
        .warnings
        .iter()
        .any(|w| w.contains("capacity")));
    assert_eq!(
        r.proposed.plans.savings.summary.as_ref().unwrap().schedule[0]
            .debt_reserved
            .as_deref(),
        Some("3100.00")
    );
}

#[test]
fn new_proposal_controls_reject_conflicts_bounds_and_forged_values_without_writing() {
    let (_dir, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let audits = audit_count(&w);
    let extra = |month: &str, amount: &str| crate::debt::DatedAmount {
        month: month.into(),
        amount: amount.into(),
    };
    for extras in [
        vec![extra("2026-11", "1"), extra("2026-11", "2")],
        vec![extra("2026-10", "1")],
        vec![extra("2076-11", "1")],
        vec![extra("2026-13", "1")],
        vec![extra("2026-11", "12.345")],
        vec![extra("2026-11", "-1")],
        vec![extra("2026-11", "")],
        (0..25).map(|_| extra("2026-11", "1")).collect(),
    ] {
        let mut i = input();
        i.debt_extras = extras;
        assert!(w.compare_financial_scenarios(&id, i).is_err());
    }
    for (index, amount) in [(1, Some("1")), (0, Some("12.345")), (0, Some("-1"))] {
        let mut i = input();
        i.goal_contributions = vec![GoalContributionChange {
            index,
            amount: amount.map(str::to_owned),
        }];
        assert!(w.compare_financial_scenarios(&id, i).is_err());
    }
    for count in [2, 51] {
        let mut i = input();
        i.goal_contributions = (0..count)
            .map(|_| GoalContributionChange {
                index: 0,
                amount: None,
            })
            .collect();
        assert!(w.compare_financial_scenarios(&id, i).is_err());
    }
    let mut raw = json(&input());
    raw["debtExtras"] = serde_json::json!([{"month":"2026-11","amount":"1","totalInterest":"0"}]);
    assert!(serde_json::from_value::<Input>(raw).is_err());
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    assert_eq!(audit_count(&w), audits);
    let mut d = debt_input(1);
    d.data.extras = vec![extra("2026-11", "25")];
    w.save_client_debt_plan(&id, d).unwrap();
    let mut i = input();
    i.expected_revisions.debt = 2;
    i.debt_extras = vec![extra("2026-11", "50")];
    assert!(w
        .compare_financial_scenarios(&id, i)
        .unwrap_err()
        .contains("one event"));
    let other = w.save_client(client_input()).unwrap();
    w.save_client_budget(&other.id, budget_input(0)).unwrap();
    let empty = w.get_assessment_planning(&other.id).unwrap();
    let mut i = input();
    i.expected_revisions = empty.revisions();
    i.debt_extras = vec![extra("2026-11", "1")];
    assert!(w
        .compare_financial_scenarios(&other.id, i)
        .unwrap_err()
        .contains("Save a debt plan"));
    let mut i = input();
    i.expected_revisions = empty.revisions();
    i.goal_contributions = vec![GoalContributionChange {
        index: 0,
        amount: None,
    }];
    assert!(w
        .compare_financial_scenarios(&other.id, i)
        .unwrap_err()
        .contains("Save a savings plan"));
}
#[test]
fn combines_exact_budget_debt_savings_and_explicit_dated_changes_without_writes() {
    let (_dir, mut w, id) = fixture();
    let baseline = w.get_assessment_planning(&id).unwrap();
    let mut i = input();
    i.budget_changes = vec![
        BudgetChange {
            index: 0,
            amount: Some("1600".into()),
        },
        BudgetChange {
            index: 2,
            amount: Some("100".into()),
        },
    ];
    i.debt_payment = Some(AmountChange {
        amount: Some("200".into()),
    });
    i.emergency_contribution = Some(AmountChange {
        amount: Some("300".into()),
    });
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    // 1600 * 26/12 = 3466.67; expenses 1000 + 100 => surplus 2366.67.
    assert_eq!(r.proposed.plans.budget.summary.monthly_income, "3466.67");
    assert_eq!(
        r.proposed.plans.budget.summary.surplus_before_allocations,
        "2366.67"
    );
    assert_eq!(
        r.proposed.plans.savings.context.monthly_surplus.as_deref(),
        Some("2366.67")
    );
    assert_eq!(
        r.proposed
            .plans
            .savings
            .context
            .debt_monthly_payment
            .as_deref(),
        Some("200.00")
    );
    assert_eq!(
        r.proposed
            .plans
            .savings
            .summary
            .as_ref()
            .unwrap()
            .requested_monthly_savings
            .as_deref(),
        Some("350.00")
    );
    assert_eq!(r.proposed.first_month_buffer.as_deref(), Some("1816.67"));
    let debt = r
        .proposed
        .plans
        .debt
        .comparison
        .as_ref()
        .unwrap()
        .snowball
        .as_ref()
        .unwrap();
    assert_eq!(debt.total_interest, "23.01");
    assert_eq!(debt.payoff_month.as_deref(), Some("2027-02"));
    assert_eq!(
        r.baseline
            .plans
            .debt
            .comparison
            .as_ref()
            .unwrap()
            .avalanche
            .as_ref()
            .unwrap()
            .total_interest,
        "35.35"
    );
    assert_eq!(
        r.proposed
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
    assert_eq!(json(&r.baseline.plans), json(&baseline));
    assert_eq!(
        r.proposed
            .plans
            .cashflow
            .summary
            .as_ref()
            .unwrap()
            .combined_closing
            .as_deref(),
        Some("3300.00")
    );
    // Explicitly delay the bill; amount stays the same. Closing unchanged, shortfall resolved.
    let mut i = input();
    let row = &baseline.cashflow.data.as_ref().unwrap().schedules[0];
    i.cash_changes.push(CashChange {
        settlement_change: None,
        index: 0,
        amount: row.amount.clone(),
        first_date: "2026-11-08".into(),
    });
    let dated = w.compare_financial_scenarios(&id, i).unwrap();
    assert!(dated
        .baseline
        .plans
        .cashflow
        .summary
        .as_ref()
        .unwrap()
        .days
        .iter()
        .any(|d| d.warnings.iter().any(|w| w.contains("shortfall"))));
    assert!(dated
        .proposed
        .plans
        .cashflow
        .summary
        .as_ref()
        .unwrap()
        .days
        .iter()
        .all(|d| d.warnings.iter().all(|w| !w.contains("shortfall"))));
    assert_eq!(
        json(&w.get_assessment_planning(&id).unwrap()),
        json(&baseline)
    );
}
#[test]
fn unknowns_deficits_and_unsupported_debt_remain_explicit() {
    let (_dir, mut w, id) = fixture();
    let mut i = input();
    i.budget_changes.push(BudgetChange {
        index: 0,
        amount: None,
    });
    i.debt_payment = Some(AmountChange { amount: None });
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert!(r
        .proposed
        .plans
        .debt
        .comparison
        .as_ref()
        .unwrap()
        .avalanche
        .is_none());
    assert!(r.proposed.plans.savings.context.monthly_surplus.is_none());
    assert!(r.proposed.first_month_buffer.is_none());
    let mut i = input();
    i.budget_changes.push(BudgetChange {
        index: 0,
        amount: Some("0".into()),
    });
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert_eq!(
        r.proposed.plans.budget.summary.surplus_before_allocations,
        "-1200.00"
    );
    assert!(r
        .proposed
        .first_month_buffer
        .as_ref()
        .unwrap()
        .starts_with('-'));
    let mut baseline = w.get_assessment_planning(&id).unwrap();
    baseline.budget.summary.monthly_income = "9999.99".into();
    assert!(compare(baseline, input()).is_err());
}
#[test]
fn rejects_stale_scoped_invalid_and_excessive_assumptions_without_audit_or_plan_writes() {
    let (_dir, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let audits = audit_count(&w);
    for revision in 0..4 {
        let mut i = input();
        match revision {
            0 => i.expected_revisions.budget = 0,
            1 => i.expected_revisions.debt = 0,
            2 => i.expected_revisions.savings = 0,
            _ => i.expected_revisions.cashflow = 0,
        };
        assert!(w.compare_financial_scenarios(&id, i).is_err());
    }
    for (index, amount) in [(100, "10"), (0, "12.345"), (0, "-1")] {
        let mut i = input();
        i.budget_changes.push(BudgetChange {
            index,
            amount: Some(amount.into()),
        });
        assert!(w.compare_financial_scenarios(&id, i).is_err());
    }
    let mut i = input();
    i.budget_changes = vec![
        BudgetChange {
            index: 0,
            amount: None,
        },
        BudgetChange {
            index: 0,
            amount: None,
        },
    ];
    assert!(w.compare_financial_scenarios(&id, i).is_err());
    let mut i = input();
    i.cash_changes.push(CashChange {
        settlement_change: None,
        index: 0,
        amount: Some("1".into()),
        first_date: "2026-02-30".into(),
    });
    assert!(w.compare_financial_scenarios(&id, i).is_err());
    let mut i = input();
    i.budget_changes = (0..101)
        .map(|index| BudgetChange {
            index,
            amount: None,
        })
        .collect();
    assert!(w.compare_financial_scenarios(&id, i).is_err());
    let mut i = input();
    i.cash_changes = vec![
        CashChange {
            settlement_change: None,
            index: 0,
            amount: None,
            first_date: "2026-11-08".into(),
        },
        CashChange {
            settlement_change: None,
            index: 0,
            amount: None,
            first_date: "2026-11-09".into(),
        },
    ];
    assert!(w.compare_financial_scenarios(&id, i).is_err());
    let mut i = input();
    i.label = "x".repeat(121);
    assert!(w.compare_financial_scenarios(&id, i).is_err());
    let mut raw = json(&input());
    raw["projectedOutcomes"] = serde_json::json!({"balance":"9999"});
    assert!(serde_json::from_value::<Input>(raw).is_err());
    let other = w.save_client(client_input()).unwrap();
    assert!(w.compare_financial_scenarios(&other.id, input()).is_err());
    assert!(w
        .compare_financial_scenarios("foreign-client", input())
        .is_err());
    // A successful preview is read-only too; exclude the deliberate extra client creation.
    let audits_after_creation = audit_count(&w);
    w.compare_financial_scenarios(&id, input()).unwrap();
    assert_eq!(audit_count(&w), audits_after_creation);
    assert!(audits_after_creation > audits);
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
    w.lock_workspace();
    assert!(w.compare_financial_scenarios(&id, input()).is_err());
}

#[test]
fn retained_future_debt_changes_and_extra_reserves_are_not_replaced_by_base_payment() {
    let (_dir, mut w, id) = fixture();
    let mut debt = debt_input(1);
    debt.data.payment_changes = vec![crate::debt::DatedAmount {
        month: "2026-12".into(),
        amount: "2000".into(),
    }];
    debt.data.extras = vec![crate::debt::DatedAmount {
        month: "2026-11".into(),
        amount: "50".into(),
    }];
    w.save_client_debt_plan(&id, debt).unwrap();
    let mut i = input();
    i.expected_revisions.debt = 2;
    i.debt_payment = Some(AmountChange {
        amount: Some("200".into()),
    });
    i.emergency_contribution = Some(AmountChange {
        amount: Some("300".into()),
    });
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert_eq!(r.proposed.first_month_buffer.as_deref(), Some("1450.00"));
    let savings = r.proposed.plans.savings.summary.as_ref().unwrap();
    assert_eq!(savings.schedule[0].debt_reserved.as_deref(), Some("250.00"));
    assert_eq!(
        savings.schedule[1].debt_reserved.as_deref(),
        Some("2000.00")
    );
    assert!(savings.schedule[1]
        .unallocated
        .as_ref()
        .unwrap()
        .starts_with('-'));
    let data = r.proposed.plans.debt.data.as_ref().unwrap();
    assert_eq!(data.payment_changes[0].amount, "2000.00");
    assert_eq!(data.extras[0].amount, "50.00");
}
#[test]
fn optional_absent_models_are_unavailable_and_cannot_receive_unbound_changes() {
    let (_dir, mut w, _) = configured();
    let client = w.save_client(client_input()).unwrap();
    w.save_client_budget(&client.id, budget_input(0)).unwrap();
    let mut i = input();
    i.expected_revisions = planning::Revisions {
        budget: 1,
        debt: 0,
        savings: 0,
        cashflow: 0,
    };
    let r = w.compare_financial_scenarios(&client.id, i).unwrap();
    assert!(r.proposed.first_month_buffer.is_none());
    assert!(r.proposed.plans.debt.comparison.is_none());
    assert!(r.proposed.plans.cashflow.summary.is_none());
    for kind in 0..3 {
        let mut i = input();
        i.expected_revisions = planning::Revisions {
            budget: 1,
            debt: 0,
            savings: 0,
            cashflow: 0,
        };
        match kind {
            0 => {
                i.debt_payment = Some(AmountChange {
                    amount: Some("10".into()),
                })
            }
            1 => {
                i.emergency_contribution = Some(AmountChange {
                    amount: Some("10".into()),
                })
            }
            _ => i.cash_changes.push(CashChange {
                settlement_change: None,
                index: 0,
                amount: Some("10".into()),
                first_date: "2026-11-08".into(),
            }),
        };
        assert!(w.compare_financial_scenarios(&client.id, i).is_err());
    }
}

#[test]
fn combined_buffer_keeps_the_higher_budget_debt_reserve_until_both_inputs_change() {
    let (_dir, mut w, id) = fixture();
    let mut budget = budget_input(1);
    budget.lines = w.get_client_budget(&id).unwrap().lines;
    budget.lines.push(budget::BudgetLine {
        label: "Debt allocation".into(),
        kind: budget::Kind::DebtPayment,
        amount: Some("400".into()),
        frequency: budget::Frequency::Monthly,
        estimated: false,
    });
    w.save_client_budget(&id, budget).unwrap();
    for (allocation, expected, reserved) in [
        (None, "1616.67", "400.00"),
        (Some("100"), "1816.67", "200.00"),
    ] {
        let mut i = input();
        i.expected_revisions.budget = 2;
        i.budget_changes = vec![
            BudgetChange {
                index: 0,
                amount: Some("1600".into()),
            },
            BudgetChange {
                index: 2,
                amount: Some("100".into()),
            },
        ];
        if let Some(amount) = allocation {
            i.budget_changes.push(BudgetChange {
                index: 3,
                amount: Some(amount.into()),
            });
        }
        i.debt_payment = Some(AmountChange {
            amount: Some("200".into()),
        });
        i.emergency_contribution = Some(AmountChange {
            amount: Some("300".into()),
        });
        let r = w.compare_financial_scenarios(&id, i).unwrap();
        assert_eq!(r.proposed.first_month_buffer.as_deref(), Some(expected));
        assert_eq!(
            r.proposed.plans.savings.summary.as_ref().unwrap().schedule[0]
                .debt_reserved
                .as_deref(),
            Some(reserved)
        );
        assert_eq!(
            w.get_client_budget(&id)
                .unwrap()
                .summary
                .planned_debt_payments,
            "400.00"
        );
    }
}

#[test]
fn paycheck_earmarks_use_saved_monthly_priorities_exactly_and_never_write() {
    let (_dir, mut w, client) = fixture();
    let mut p = w.get_assessment_planning(&client).unwrap();
    p.budget.lines[1].amount = Some("1000.01".into());
    let (lines, summary) = budget::calculate(p.budget.lines).unwrap();
    p.budget.lines = lines;
    p.budget.summary = summary;
    let original = serde_json::to_vec(&p).unwrap();
    let a = crate::paycheck::allocate(&p).unwrap();
    assert_eq!(a.rows.len(), 2);
    assert_eq!(a.rows[0].allocations[0].as_deref(), Some("500.01"));
    assert_eq!(a.rows[1].allocations[0].as_deref(), Some("500.00"));
    assert_eq!(a.rows[0].dated_commitments.as_deref(), Some("200.00"));
    assert_eq!(serde_json::to_vec(&p).unwrap(), original);
    let mut weekly = crate::saved_scenarios::copy(&p).unwrap();
    weekly.budget.lines[1].amount = Some("100.01".into());
    weekly.budget.lines[1].frequency = budget::Frequency::Weekly;
    let (lines, summary) = budget::calculate(weekly.budget.lines).unwrap();
    weekly.budget.lines = lines;
    weekly.budget.summary = summary;
    let cash = weekly.cashflow.data.as_mut().unwrap();
    cash.schedules[1].frequency = crate::cashflow::Frequency::Weekly;
    weekly.cashflow.summary = Some(crate::cashflow::calculate(cash.clone()).unwrap().1);
    let a = crate::paycheck::allocate(&weekly).unwrap();
    assert_eq!(a.rows.len(), 4);
    assert_eq!(a.rows[0].allocations[0].as_deref(), Some("108.35"));
    assert_eq!(a.rows[1].allocations[0].as_deref(), Some("108.35"));
    assert_eq!(a.rows[2].allocations[0].as_deref(), Some("108.34"));
    assert_eq!(a.rows[3].allocations[0].as_deref(), Some("108.34"));
    let data = p.cashflow.data.as_mut().unwrap();
    data.start_date = "2026-11-02".into();
    p.cashflow.summary = Some(crate::cashflow::calculate(data.clone()).unwrap().1);
    let a = crate::paycheck::allocate(&p).unwrap();
    assert!(a.rows[0].total.is_none());
    assert!(a.rows[0]
        .warnings
        .iter()
        .any(|w| w.contains("Partial month")));
}

#[test]
fn new_goal_and_funding_control_validation_never_writes_or_invents_opening_reserves() {
    let (_dir, mut w, client) = fixture();
    let original = json(&w.get_assessment_planning(&client).unwrap());
    let count = audit_count(&w);
    let make = || crate::scenarios::NewGoal {
        id: "new-goal".into(),
        label: "Fictional repairs".into(),
        target_amount: None,
        target_date: None,
        monthly_contribution: None,
    };
    let mut proposal = input();
    proposal.new_goals = vec![make(), make()];
    assert!(w.compare_financial_scenarios(&client, proposal).is_err());
    let mut proposal = input();
    proposal.new_goals = vec![make()];
    let r = w.compare_financial_scenarios(&client, proposal).unwrap();
    assert_eq!(r.calculator, "combined-preview-v2");
    let savings = r.proposed.plans.savings;
    assert_eq!(
        savings.data.unwrap().goals[1].reserved_amount.as_deref(),
        Some("0.00")
    );
    assert_eq!(
        savings.summary.unwrap().goals[1].forecast.status,
        crate::savings::ForecastStatus::Unknown
    );
    let mut proposal = input();
    proposal.savings_funding = Some(crate::savings::funding::Plan {
        sources: vec![],
        events: vec![crate::savings::funding::Event {
            date: "2026-11-10".into(),
            kind: crate::savings::funding::Kind::Fund,
            bucket: None,
            destination: None,
            source_id: Some("missing-source".into()),
            amount: Some("100".into()),
            note: "Fictional amount is not funded".into(),
        }],
    });
    assert!(w.compare_financial_scenarios(&client, proposal).is_err());
    assert_eq!(json(&w.get_assessment_planning(&client).unwrap()), original);
    assert_eq!(audit_count(&w), count);
}

#[test]
fn scenario_transfer_availability_is_explicit_and_never_borrows_in_transit_funds() {
    let (_dir, mut w, id) = fixture();
    let before = json(&w.get_assessment_planning(&id).unwrap());
    let change = |date: Option<&str>| CashChange {
        index: 2,
        amount: Some("200".into()),
        first_date: "2026-11-07".into(),
        settlement_change: Some(crate::scenarios::SettlementChange {
            delayed: true,
            first_available_date: date.map(Into::into),
        }),
    };
    let mut i = input();
    i.cash_changes = vec![change(Some("2026-11-10"))];
    let r = w.compare_financial_scenarios(&id, i).unwrap();
    assert_eq!(r.calculator, "combined-preview-v2");
    let c = r.proposed.plans.cashflow.summary.unwrap();
    assert_eq!(c.ledger[2].combined_balance.as_deref(), Some("1600.00"));
    assert_eq!(c.ledger[2].destination_balance.as_deref(), Some("1000.00"));
    assert_eq!(c.ledger[3].date, "2026-11-10");
    assert_eq!(c.combined_closing.as_deref(), Some("3300.00"));
    let mut i = input();
    i.cash_changes = vec![change(None)];
    assert!(w
        .compare_financial_scenarios(&id, i)
        .unwrap()
        .proposed
        .plans
        .cashflow
        .summary
        .unwrap()
        .combined_closing
        .is_none());
    let mut i = input();
    i.cash_changes = vec![change(Some("2026-12-05"))];
    let c = w
        .compare_financial_scenarios(&id, i)
        .unwrap()
        .proposed
        .plans
        .cashflow
        .summary
        .unwrap();
    assert_eq!(c.combined_closing.as_deref(), Some("3100.00"));
    assert_eq!(c.in_transit.as_deref(), Some("200.00"));
    let mut i = input();
    let mut wrong = change(Some("2026-11-10"));
    wrong.index = 0;
    i.cash_changes = vec![wrong];
    assert!(w.compare_financial_scenarios(&id, i).is_err());
    assert_eq!(json(&w.get_assessment_planning(&id).unwrap()), before);
}
