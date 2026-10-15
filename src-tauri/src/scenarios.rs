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
pub struct GoalContributionChange {
    pub index: usize,
    pub amount: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpenseDates {
    pub account_id: String,
    pub first_date: String,
    pub second_day: Option<u32>,
    pub until_date: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecurringExpense {
    pub label: String,
    pub kind: budget::Kind,
    pub amount: Option<String>,
    pub frequency: budget::Frequency,
    pub dated: Option<ExpenseDates>,
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
    // Empty extensions are omitted to preserve canonical historical v1 payloads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub debt_extras: Vec<debt::DatedAmount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goal_contributions: Vec<GoalContributionChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recurring_expenses: Vec<RecurringExpense>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub income_reductions: Vec<crate::income_periods::Reduction>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub income_periods: Vec<crate::income_periods::Month>,
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
    if input.debt_extras.len() > 24 || input.goal_contributions.len() > 50 {
        return Err("Use at most 24 extra payments and 50 goal contribution changes.".into());
    }
    if input.recurring_expenses.len() > 20 {
        return Err("Use at most 20 new recurring expenses per proposal.".into());
    }
    if input.income_reductions.len() > 6 {
        return Err("Use at most six temporary income reductions.".into());
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
    let original_budget_count = proposed.budget.lines.len();
    for expense in &input.recurring_expenses {
        if !matches!(
            expense.kind,
            budget::Kind::Necessary | budget::Kind::Discretionary
        ) || expense.frequency == budget::Frequency::Irregular
        {
            return Err("New recurring expenses must be necessary or discretionary spending with a regular frequency.".into());
        }
        proposed.budget.lines.push(budget::BudgetLine {
            label: expense.label.clone(),
            kind: expense.kind,
            amount: expense.amount.clone(),
            frequency: expense.frequency,
            estimated: true,
        });
    }
    let (lines, summary) = budget::calculate(proposed.budget.lines)?;
    proposed.budget.lines = lines;
    proposed.budget.summary = summary;
    for change in &mut input.budget_changes {
        change.amount = proposed.budget.lines[change.index].amount.clone();
    }
    for (index, expense) in input.recurring_expenses.iter_mut().enumerate() {
        let line = &proposed.budget.lines[original_budget_count + index];
        expense.label = line.label.clone();
        expense.amount = line.amount.clone();
    }
    if let Some(change) = &input.debt_payment {
        proposed
            .debt
            .data
            .as_mut()
            .ok_or("Save a debt plan before changing its monthly payment.")?
            .monthly_payment = change.amount.clone();
    }
    if !input.debt_extras.is_empty() {
        proposed
            .debt
            .data
            .as_mut()
            .ok_or("Save a debt plan before proposing extra payments.")?
            .extras
            .extend(input.debt_extras.iter().cloned());
    }
    proposed.debt.budget_context = planning::debt_context(&proposed.budget);
    if let Some(data) = proposed.debt.data.take() {
        let (data, mut cmp) = debt::calculate(data)?;
        debt::apply_budget_context(&data, &mut cmp, &proposed.debt.budget_context)?;
        if let Some(change) = &mut input.debt_payment {
            change.amount = data.monthly_payment.clone();
        }
        for extra in &mut input.debt_extras {
            extra.amount = data
                .extras
                .iter()
                .find(|e| e.month == extra.month)
                .ok_or("Cannot verify the proposed extra payment.")?
                .amount
                .clone();
        }
        input.debt_extras.sort_by(|a, b| a.month.cmp(&b.month));
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
    seen.clear();
    for change in &input.goal_contributions {
        if !seen.insert(change.index) {
            return Err(
                "A savings goal contribution can only be changed once per comparison.".into(),
            );
        }
        proposed
            .savings
            .data
            .as_mut()
            .ok_or("Save a savings plan before changing goal contributions.")?
            .goals
            .get_mut(change.index)
            .ok_or("The changed savings goal is outside the saved baseline.")?
            .monthly_contribution = change.amount.clone();
    }
    proposed.savings.context = planning::savings_context(&proposed.budget, &proposed.debt);
    if let Some(data) = proposed.savings.data.take() {
        let (data, summary) = savings::calculate(data, &proposed.savings.context)?;
        if let Some(change) = &mut input.emergency_contribution {
            change.amount = data.emergency_contribution.clone();
        }
        for change in &mut input.goal_contributions {
            change.amount = data.goals[change.index].monthly_contribution.clone();
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
    for expense in &input.recurring_expenses {
        if let Some(dates) = &expense.dated {
            let frequency = match expense.frequency {
                budget::Frequency::Weekly => cashflow::Frequency::Weekly,
                budget::Frequency::Biweekly => cashflow::Frequency::Biweekly,
                budget::Frequency::Semimonthly => cashflow::Frequency::Semimonthly,
                budget::Frequency::Monthly => cashflow::Frequency::Monthly,
                _ => return Err("Annual expenses support budget estimates only; remove their dated forecast selection.".into()),
            };
            proposed
                .cashflow
                .data
                .as_mut()
                .ok_or("Save a cash-flow plan before adding an explicitly dated expense.")?
                .schedules
                .push(cashflow::Schedule {
                    label: expense.label.clone(),
                    account_id: dates.account_id.clone(),
                    destination_id: None,
                    kind: cashflow::Kind::Bill,
                    amount: expense.amount.clone(),
                    first_date: dates.first_date.clone(),
                    frequency,
                    second_day: dates.second_day,
                    until_date: dates.until_date.clone(),
                    certainty: cashflow::Certainty::Estimated,
                });
        }
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
    let expense_start = baseline
        .cashflow
        .data
        .as_ref()
        .map_or(0, |d| d.schedules.len());
    let expense_count = input
        .recurring_expenses
        .iter()
        .filter(|e| e.dated.is_some())
        .count();
    let expense_without_occurrence = proposed.cashflow.summary.as_ref().is_some_and(|s| {
        (expense_start..expense_start + expense_count)
            .any(|index| s.ledger.iter().all(|e| e.schedule_index != index))
    });
    let income_periods = crate::income_periods::apply(
        &mut proposed,
        &mut input.income_reductions,
        &input.cash_changes,
    )?;
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
    let mut warnings = vec![
        "Hypothetical entered values only. This preview is temporary and does not save a scenario, change approved history, live plans, agreements, reports or audit events.".into(),
        "Baseline versions identify the saved inputs; proposed results are recalculations, not newly saved versions. Changed budget lines and dated entries are estimates.".into(),
        "Budget allocations, debt payments and savings contributions remain distinct. Compare the savings model's first-month buffer as well as the budget's unallocated amount; neither verifies all obligations.".into(),
        "Dated cash-flow schedules change only through explicit dated-entry edits. Budget, debt and savings changes do not automatically change dates, account balances or scheduled payments.".into(),
        "Existing debt payment changes and extras, goal contributions, opening balances, reserve commitments and forecast periods remain in effect. A base payment change does not replace later dated payment changes.".into(),
        "Debt uses supported fixed monthly terms; savings assumes no investment yield. Unknown or unsupported terms remain unavailable; no outcome is guaranteed or automatically recommended.".into(),
    ];
    if !input.debt_extras.is_empty() || !input.goal_contributions.is_empty() {
        let prefix = "Proposed extra payments are added to the shared repayment budget and follow the selected strategy, not a named account, with one extra per month and at most 24 in total. Only explicitly changed goal contributions are replaced; targets, reserves, future base payment changes and other goals remain in effect. Extras reserve funding even if repayment finishes early. Funding sources are not verified";
        let ending = if input
            .income_reductions
            .iter()
            .any(|i| i.cash_schedule.is_some())
        {
            "; only explicitly selected expense dates and temporary paycheck periods change dated activity."
        } else if input.recurring_expenses.iter().any(|e| e.dated.is_some()) {
            "; only explicitly dated new expenses add cash-flow bills."
        } else {
            " and dated cash-flow schedules are unchanged."
        };
        warnings.push(format!("{prefix}{ending}"));
    }
    if !input.recurring_expenses.is_empty() {
        warnings.push("New recurring expenses are estimated additions, not replacements for existing bills. Monthly budget normalization applies throughout the model; first/last payment dates limit only the explicitly selected dated forecast, not the budget, debt capacity or emergency target. Check for duplicate obligations. Annual amounts are budget-only. Missing amounts stay unknown; dated payments use the same entered amount/frequency and only counselor-selected accounts/dates. No live plan or approved transaction is created.".into());
        if expense_without_occurrence {
            warnings.push("At least one new expense has no dated occurrence within the saved forecast period. Its monthly budget assumption still applies; review payment dates and the forecast horizon.".into());
        }
    }
    if !income_periods.is_empty() {
        warnings.push("Temporary income periods are monthly funding checks against continuing debt and savings commitments, including capped goal contributions. The main model table retains normal recurring income; payoff and savings milestones remain conditional on continuing the entered commitments. No drawdown or automatic payment/contribution reduction is inferred. One following normal month is shown when supported. Only an explicitly selected matching paycheck schedule changes dated receipts; those occurrences retain their original dates and return to normal amounts outside the period. Unknown receipts keep affected balances unknown. Review monthly funding before recording a separate client agreement.".into());
        if income_periods
            .iter()
            .any(|m| m.buffer.as_ref().is_some_and(|v| v.starts_with('-')))
        {
            warnings.push("Reduced income does not fund the continuing debt and savings commitments in at least one month. Review the monthly deficits and confirm funding or revise the proposed commitments.".into());
        }
        if income_periods.iter().any(|m| m.buffer.is_none()) {
            warnings.push("At least one temporary-income monthly funding check is unavailable. Review missing amounts, model start months and forecast horizons; unknown is not zero.".into());
        }
    }
    Ok(Comparison {
        calculator: "combined-preview-v1".into(),
        client_id: baseline.client_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        revisions: baseline.revisions(),
        baseline: outcome(baseline, input.baseline_strategy.clone()),
        proposed: outcome(proposed, input.proposed_strategy.clone()),
        input,
        warnings,
        income_periods,
    })
}
