//! Immutable entered planning assumptions, separate from approved financial facts.
use crate::{budget, cashflow, debt, savings};
use serde::{Deserialize, Serialize};
const CALCULATOR: &str = "entered-plans-v1";
pub fn calculator(plans: &Plans) -> &'static str {
    if plans
        .debt
        .data
        .as_ref()
        .is_some_and(|d| d.advanced.is_some() || !d.targeted_extras.is_empty())
        || plans
            .savings
            .data
            .as_ref()
            .is_some_and(|d| d.funding.is_some())
        || plans.cashflow.data.as_ref().is_some_and(|d| {
            d.accounts.iter().any(|a| a.reserve.is_some())
                || d.schedules.iter().any(|s| s.settlement.is_some())
        })
    {
        "entered-plans-v2"
    } else {
        CALCULATOR
    }
}
const MAX_BYTES: usize = 4 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Revisions {
    pub budget: i64,
    pub debt: i64,
    pub savings: i64,
    pub cashflow: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Strategy {
    None,
    Snowball,
    Avalanche,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub expected_revisions: Revisions,
    pub label: String,
    pub assumptions: String,
    pub debt_strategy: Strategy,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plans {
    pub client_id: String,
    pub budget: budget::ClientBudget,
    pub debt: debt::ClientDebtPlan,
    pub savings: savings::ClientSavingsPlan,
    pub cashflow: cashflow::ClientCashflowPlan,
}
impl Plans {
    pub fn revisions(&self) -> Revisions {
        Revisions {
            budget: self.budget.revision,
            debt: self.debt.revision,
            savings: self.savings.revision,
            cashflow: self.cashflow.revision,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub calculator: String,
    pub label: String,
    pub assumptions: String,
    pub debt_strategy: Strategy,
    pub revisions: Revisions,
    pub plans: Plans,
}
pub fn projection(s: &Snapshot) -> Option<&debt::Projection> {
    let c = s.plans.debt.comparison.as_ref()?;
    match s.debt_strategy {
        Strategy::None => None,
        Strategy::Snowball => c.snowball.as_ref(),
        Strategy::Avalanche => c.avalanche.as_ref(),
    }
}
pub fn capture(plans: Plans, input: Selection) -> Result<Snapshot, String> {
    if plans.revisions() != input.expected_revisions {
        return Err("Saved planning data changed elsewhere. Review the current saved plans and select the scenario again; nothing was recorded.".into());
    }
    let s = Snapshot {
        calculator: calculator(&plans).into(),
        label: input.label,
        assumptions: input.assumptions,
        debt_strategy: input.debt_strategy,
        revisions: plans.revisions(),
        plans,
    };
    validate(&s, &s.plans.client_id)?;
    Ok(s)
}
fn equal<T: Serialize>(a: &T, b: &T) -> Result<(), String> {
    if serde_json::to_value(a).map_err(|_| "Invalid planning snapshot.")?
        != serde_json::to_value(b).map_err(|_| "Invalid planning snapshot.")?
    {
        return Err("Captured planning assumptions or calculations are inconsistent.".into());
    }
    Ok(())
}
fn metadata(
    client: &str,
    expected: &str,
    revision: i64,
    time: &Option<String>,
    present: bool,
) -> Result<(), String> {
    if client != expected
        || !(0..i64::from(i32::MAX)).contains(&revision)
        || (revision > 0) != present
        || time.is_some() != present
    {
        return Err("Invalid captured planning identity or version.".into());
    }
    if let Some(t) = time {
        chrono::DateTime::parse_from_rfc3339(t).map_err(|_| "Invalid captured planning date.")?;
    }
    Ok(())
}
pub fn validate(s: &Snapshot, client: &str) -> Result<(), String> {
    if s.calculator != calculator(&s.plans)
        || s.plans.client_id != client
        || s.revisions != s.plans.revisions()
        || s.label.trim() != s.label
        || s.label.is_empty()
        || s.label.chars().count() > 120
        || s.label.chars().any(char::is_control)
        || s.assumptions.trim() != s.assumptions
        || s.assumptions.chars().count() > 4000
        || s.assumptions
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err("Invalid captured scenario description or calculation version.".into());
    }
    if serde_json::to_vec(s)
        .map_err(|_| "Invalid captured scenario.")?
        .len()
        > MAX_BYTES
    {
        return Err("The captured planning scenario exceeds its 4 MiB safety limit.".into());
    }
    let p = &s.plans;
    let b = &p.budget;
    metadata(
        &b.client_id,
        client,
        b.revision,
        &b.updated_at,
        b.revision > 0,
    )?;
    if b.revision == 0 || b.lines.is_empty() {
        return Err("Save a nonempty client budget before including a planning scenario.".into());
    }
    let (lines, summary) = budget::calculate(b.lines.clone())?;
    equal(&lines, &b.lines)?;
    equal(&summary, &b.summary)?;
    let d = &p.debt;
    metadata(
        &d.client_id,
        client,
        d.revision,
        &d.updated_at,
        d.data.is_some(),
    )?;
    let dc = debt_context(b);
    equal(&dc, &d.budget_context)?;
    if let Some(data) = &d.data {
        let (data, mut cmp) = debt::calculate(data.clone())?;
        debt::apply_budget_context(&data, &mut cmp, &dc)?;
        equal(&data, d.data.as_ref().unwrap())?;
        equal(&Some(cmp), &d.comparison)?;
    } else if d.comparison.is_some() {
        return Err("A missing debt plan cannot supply projections.".into());
    }
    if s.debt_strategy != Strategy::None && projection(s).is_none() {
        return Err("The selected debt strategy has no supported projection. Review missing or unsupported repayment terms before selecting it.".into());
    }
    let savings = &p.savings;
    metadata(
        &savings.client_id,
        client,
        savings.revision,
        &savings.updated_at,
        savings.data.is_some(),
    )?;
    let sc = savings_context(b, d);
    equal(&sc, &savings.context)?;
    if let Some(data) = &savings.data {
        let (data, summary) = savings::calculate(data.clone(), &sc)?;
        equal(&data, savings.data.as_ref().unwrap())?;
        equal(&Some(summary), &savings.summary)?;
    } else if savings.summary.is_some() {
        return Err("A missing savings plan cannot supply forecasts.".into());
    }
    let cash = &p.cashflow;
    metadata(
        &cash.client_id,
        client,
        cash.revision,
        &cash.updated_at,
        cash.data.is_some(),
    )?;
    equal(&cashflow_context(savings), &cash.context)?;
    if let Some(data) = &cash.data {
        let (data, summary) = cashflow::calculate(data.clone())?;
        equal(&data, cash.data.as_ref().unwrap())?;
        equal(&Some(summary), &cash.summary)?;
    } else if cash.summary.is_some() {
        return Err("A missing dated cash-flow plan cannot supply balances.".into());
    }
    Ok(())
}

pub fn debt_context(budget: &budget::ClientBudget) -> debt::BudgetContext {
    debt::BudgetContext {
        revision: budget.revision,
        monthly_capacity: budget.summary.available_for_debt.clone(),
        planned_debt_payments: budget.summary.planned_debt_payments.clone(),
        warnings: budget.summary.warnings.clone(),
    }
}

pub fn savings_context(
    budget: &budget::ClientBudget,
    debt: &debt::ClientDebtPlan,
) -> savings::Context {
    let has_necessary = budget
        .lines
        .iter()
        .any(|line| line.kind == budget::Kind::Necessary && line.amount.is_some());
    let necessary_missing = budget
        .lines
        .iter()
        .any(|line| line.kind == budget::Kind::Necessary && line.amount.is_none());
    let cash_missing = budget.lines.iter().any(|line| {
        matches!(
            line.kind,
            budget::Kind::Income | budget::Kind::Necessary | budget::Kind::Discretionary
        ) && line.amount.is_none()
    });
    let has_income = budget
        .lines
        .iter()
        .any(|line| line.kind == budget::Kind::Income && line.amount.is_some());
    let savings_missing = budget
        .lines
        .iter()
        .any(|line| line.kind == budget::Kind::Savings && line.amount.is_none());
    let debt_missing = budget
        .lines
        .iter()
        .any(|line| line.kind == budget::Kind::DebtPayment && line.amount.is_none());
    let mut warnings = Vec::new();
    if budget.revision == 0 {
        warnings.push(
            "No saved budget is available for emergency expenses or allocation checks.".to_owned(),
        );
    }
    if !budget.summary.warnings.is_empty() {
        warnings.push("Review missing, estimated, or deficit warnings in the saved budget; these projections use only entered values.".to_owned());
    }
    if debt.revision == 0 {
        warnings.push("No saved debt plan: using the budget's debt allocation. Confirm that all obligations are included.".to_owned());
    }
    if debt
        .comparison
        .as_ref()
        .is_some_and(|c| c.snowball.is_none() || c.avalanche.is_none())
    {
        warnings.push("The saved debt plan has incomplete or unmodeled terms; its entered payment budget does not verify required lender payments.".to_owned());
    }
    if debt.comparison.as_ref().is_some_and(|c| {
        [c.snowball.as_ref(), c.avalanche.as_ref()]
            .into_iter()
            .flatten()
            .any(|p| {
                p.status != debt::Status::PaidOff || !p.negative_amortization_accounts.is_empty()
            })
    }) {
        warnings.push("Review the debt plan's payment shortfall, growth, or horizon warnings; savings projections do not resolve those issues.".to_owned());
    }
    let debt_data = debt.data.as_ref();
    savings::Context {
        budget_revision: budget.revision,
        debt_revision: debt.revision,
        necessary_expenses: if has_necessary && !necessary_missing {
            Some(budget.summary.necessary_expenses.clone())
        } else {
            None
        },
        monthly_surplus: if budget.revision > 0 && has_income && has_necessary && !cash_missing {
            Some(budget.summary.surplus_before_allocations.clone())
        } else {
            None
        },
        budget_savings: if budget.revision > 0 && !savings_missing {
            Some(budget.summary.proposed_savings.clone())
        } else {
            None
        },
        budget_debt_payments: if budget.revision > 0 && !debt_missing {
            Some(budget.summary.planned_debt_payments.clone())
        } else {
            None
        },
        debt_start_month: debt_data.as_ref().map(|d| d.start_month.clone()),
        debt_monthly_payment: debt_data.as_ref().and_then(|d| d.monthly_payment.clone()),
        debt_changes: debt_data
            .as_ref()
            .map(|d| d.payment_changes.clone())
            .unwrap_or_default(),
        debt_extras: debt_data.map(debt::all_extras).unwrap_or_default(),
        warnings,
    }
}

pub fn cashflow_context(savings: &savings::ClientSavingsPlan) -> cashflow::Context {
    cashflow::Context {
        budget_revision: savings.context.budget_revision,
        debt_revision: savings.context.debt_revision,
        savings_revision: savings.revision,
        monthly_surplus: savings.context.monthly_surplus.clone(),
        budget_debt: savings.context.budget_debt_payments.clone(),
        budget_savings: savings.context.budget_savings.clone(),
        debt_monthly_payment: savings.context.debt_monthly_payment.clone(),
        savings_monthly_contribution: savings
            .summary
            .as_ref()
            .and_then(|s| s.requested_monthly_savings.clone()),
    }
}
