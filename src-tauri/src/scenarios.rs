//! Non-writing combined what-if comparisons. Input carries assumptions, never outcomes.
use crate::{budget, cashflow, debt, planning, savings};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AmountChange {
    pub amount: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetChange {
    pub index: usize,
    pub amount: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CashChange {
    pub index: usize,
    pub amount: Option<String>,
    pub first_date: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revisions: planning::Revisions,
    pub label: String,
    pub assumptions: String,
    pub baseline_strategy: planning::Strategy,
    pub proposed_strategy: planning::Strategy,
    pub budget_changes: Vec<BudgetChange>,
    pub debt_payment: Option<AmountChange>,
    pub emergency_contribution: Option<AmountChange>,
    pub cash_changes: Vec<CashChange>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Outcome {
    pub strategy: planning::Strategy,
    pub first_month_buffer: Option<String>,
    pub plans: planning::Plans,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Comparison {
    pub calculator: String,
    pub client_id: String,
    pub created_at: String,
    pub revisions: planning::Revisions,
    pub input: Input,
    pub baseline: Outcome,
    pub proposed: Outcome,
    pub warnings: Vec<String>,
}
fn outcome(plans: planning::Plans, strategy: planning::Strategy) -> Outcome {
    let first_month_buffer = plans
        .savings
        .summary
        .as_ref()
        .and_then(|s| s.schedule.first())
        .and_then(|m| m.unallocated.clone());
    Outcome {
        strategy,
        first_month_buffer,
        plans,
    }
}
pub fn compare(plans: planning::Plans, mut input: Input) -> Result<Comparison, String> {
    if input.budget_changes.len() > 100 || input.cash_changes.len() > 100 {
        return Err(
            "A comparison supports at most 100 budget and 100 dated schedule changes.".into(),
        );
    }
    input.label = input.label.trim().into();
    input.assumptions = input.assumptions.trim().into();
    // Validate the original identity, revision, canonical calculations and shared contexts.
    let baseline = planning::capture(
        plans,
        planning::Selection {
            expected_revisions: input.expected_revisions.clone(),
            label: input.label.clone(),
            assumptions: input.assumptions.clone(),
            debt_strategy: planning::Strategy::None,
        },
    )?
    .plans;
    let mut proposed: planning::Plans = serde_json::from_slice(
        &serde_json::to_vec(&baseline).map_err(|_| "Cannot prepare the baseline comparison.")?,
    )
    .map_err(|_| "Cannot prepare the baseline comparison.")?;
    let mut seen = HashSet::new();
    for change in &mut input.budget_changes {
        if !seen.insert(change.index) {
            return Err("A budget line can only be changed once per comparison.".into());
        }
        let line = proposed
            .budget
            .lines
            .get_mut(change.index)
            .ok_or("The changed budget line is outside the saved baseline.")?;
        line.amount = change.amount.clone();
        line.estimated = true;
    }
    let (lines, summary) = budget::calculate(proposed.budget.lines)?;
    proposed.budget.lines = lines;
    proposed.budget.summary = summary;
    for change in &mut input.budget_changes {
        change.amount = proposed.budget.lines[change.index].amount.clone();
    }
    if let Some(change) = &input.debt_payment {
        proposed
            .debt
            .data
            .as_mut()
            .ok_or("Save a debt plan before changing its monthly payment.")?
            .monthly_payment = change.amount.clone();
    }
    proposed.debt.budget_context = planning::debt_context(&proposed.budget);
    if let Some(data) = proposed.debt.data.take() {
        let (data, mut cmp) = debt::calculate(data)?;
        debt::apply_budget_context(&data, &mut cmp, &proposed.debt.budget_context)?;
        if let Some(change) = &mut input.debt_payment {
            change.amount = data.monthly_payment.clone();
        }
        proposed.debt.data = Some(data);
        proposed.debt.comparison = Some(cmp);
    }
    if let Some(change) = &input.emergency_contribution {
        proposed
            .savings
            .data
            .as_mut()
            .ok_or("Save a savings plan before changing its emergency contribution.")?
            .emergency_contribution = change.amount.clone();
    }
    proposed.savings.context = planning::savings_context(&proposed.budget, &proposed.debt);
    if let Some(data) = proposed.savings.data.take() {
        let (data, summary) = savings::calculate(data, &proposed.savings.context)?;
        if let Some(change) = &mut input.emergency_contribution {
            change.amount = data.emergency_contribution.clone();
        }
        proposed.savings.data = Some(data);
        proposed.savings.summary = Some(summary);
    }
    seen.clear();
    for change in &input.cash_changes {
        if !seen.insert(change.index) {
            return Err("A dated schedule can only be changed once per comparison.".into());
        }
        let row = proposed
            .cashflow
            .data
            .as_mut()
            .ok_or("Save a dated cash-flow plan before changing its schedule.")?
            .schedules
            .get_mut(change.index)
            .ok_or("The changed schedule is outside the saved baseline.")?;
        row.amount = change.amount.clone();
        row.first_date = change.first_date.clone();
        row.certainty = cashflow::Certainty::Estimated;
    }
    proposed.cashflow.context = planning::cashflow_context(&proposed.savings);
    if let Some(data) = proposed.cashflow.data.take() {
        let (data, summary) = cashflow::calculate(data)?;
        for change in &mut input.cash_changes {
            change.amount = data.schedules[change.index].amount.clone();
        }
        proposed.cashflow.data = Some(data);
        proposed.cashflow.summary = Some(summary);
    }
    // Recheck recalculated models as a complete set without requiring a supported debt result.
    let proposed = planning::capture(
        proposed,
        planning::Selection {
            expected_revisions: input.expected_revisions.clone(),
            label: input.label.clone(),
            assumptions: input.assumptions.clone(),
            debt_strategy: planning::Strategy::None,
        },
    )?
    .plans;
    let warnings = vec![
        "Hypothetical entered values only. This preview is temporary and does not save a scenario, change approved history, live plans, agreements, reports or audit events.".into(),
        "Baseline versions identify the saved inputs; proposed results are recalculations, not newly saved versions. Changed budget lines and dated entries are estimates.".into(),
        "Budget allocations, debt payments and savings contributions remain distinct. Compare the savings model's first-month buffer as well as the budget's unallocated amount; neither verifies all obligations.".into(),
        "Dated cash-flow schedules change only through explicit dated-entry edits. Budget, debt and savings changes do not automatically change dates, account balances or scheduled payments.".into(),
        "Existing debt payment changes and extras, goal contributions, opening balances, reserve commitments and forecast periods remain in effect. A base payment change does not replace later dated payment changes.".into(),
        "Debt uses supported fixed monthly terms; savings assumes no investment yield. Unknown or unsupported terms remain unavailable; no outcome is guaranteed or automatically recommended.".into(),
    ];
    Ok(Comparison {
        calculator: "combined-preview-v1".into(),
        client_id: baseline.client_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        revisions: baseline.revisions(),
        baseline: outcome(baseline, input.baseline_strategy.clone()),
        proposed: outcome(proposed, input.proposed_strategy.clone()),
        input,
        warnings,
    })
}
