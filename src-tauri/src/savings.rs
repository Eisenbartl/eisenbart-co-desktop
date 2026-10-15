//! Savings planning in cents. Reserve buckets partition one liquid pool.
use crate::budget::{money, parse_money};
use crate::debt::{month_number, DatedAmount};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

const HORIZON: i32 = 600;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Goal {
    pub label: String,
    pub target_amount: Option<String>,
    pub target_date: Option<String>,
    pub reserved_amount: Option<String>,
    pub monthly_contribution: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanData {
    pub start_month: String,
    pub target_months: u32,
    pub liquid_savings: Option<String>,
    pub restricted_savings: Option<String>,
    pub obligations_reserved: Option<String>,
    pub emergency_contribution: Option<String>,
    pub alternative_contributions: Vec<String>,
    pub goals: Vec<Goal>,
    pub priorities_note: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanInput {
    pub expected_revision: i64,
    pub data: PlanData,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    pub budget_revision: i64,
    pub debt_revision: i64,
    pub necessary_expenses: Option<String>,
    pub monthly_surplus: Option<String>,
    pub budget_savings: Option<String>,
    pub budget_debt_payments: Option<String>,
    pub debt_start_month: Option<String>,
    pub debt_monthly_payment: Option<String>,
    pub debt_changes: Vec<DatedAmount>,
    pub debt_extras: Vec<DatedAmount>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ForecastStatus {
    Funded,
    Projected,
    NoContribution,
    Unknown,
    HorizonReached,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Forecast {
    pub status: ForecastStatus,
    pub remaining: Option<String>,
    pub contribution: Option<String>,
    pub months_needed: Option<u32>,
    pub completion_date: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoalResult {
    pub suggested_contribution: Option<String>,
    pub contribution_months_before_due: Option<u32>,
    pub on_track: Option<bool>,
    pub forecast: Forecast,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Month {
    pub month: String,
    pub emergency_contribution: String,
    pub goal_contributions: Vec<String>,
    pub total_savings: String,
    pub debt_reserved: Option<String>,
    pub unallocated: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scenario {
    pub forecast: Forecast,
    pub allocation_warning: Option<bool>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub accessible_emergency: Option<String>,
    pub obligations_reserved: Option<String>,
    pub sinking_funds_reserved: Option<String>,
    pub restricted_savings: Option<String>,
    pub coverage_months: Option<String>,
    pub emergency_target: Option<String>,
    pub emergency: Forecast,
    pub alternatives: Vec<Scenario>,
    pub goals: Vec<GoalResult>,
    pub requested_monthly_savings: Option<String>,
    pub warnings: Vec<String>,
    pub schedule: Vec<Month>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientSavingsPlan {
    pub client_id: String,
    pub revision: i64,
    pub updated_at: Option<String>,
    pub data: Option<PlanData>,
    pub summary: Option<Summary>,
    pub context: Context,
}

fn optional_money(value: &mut Option<String>) -> Result<Option<i64>, String> {
    value
        .as_mut()
        .map(|v| {
            let cents = parse_money(v.trim())?;
            *v = money(cents);
            Ok(cents)
        })
        .transpose()
}
fn aggregate(value: &Option<String>) -> Result<Option<i64>, String> {
    value
        .as_ref()
        .map(|v| {
            v.replace('.', "")
                .parse()
                .map_err(|_| "Invalid savings context.".to_owned())
        })
        .transpose()
}
fn end_date(number: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt((number + 1) / 12, ((number + 1) % 12 + 1) as u32, 1)
        .expect("bounded model month")
        .pred_opt()
        .expect("bounded model date")
}
fn parse_date(value: &str) -> Result<NaiveDate, String> {
    let bad = || "Enter a real goal date as YYYY-MM-DD, between 1900 and 2100.".to_owned();
    if value.len() != 10
        || !value.is_ascii()
        || value.as_bytes()[4] != b'-'
        || value.as_bytes()[7] != b'-'
    {
        return Err(bad());
    }
    month_number(&value[..7]).map_err(|_| bad())?;
    if !value[8..].bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| bad())
}
fn forecast(remaining: Option<i64>, contribution: Option<i64>, start: i32) -> Forecast {
    let (status, months, completion) = match (remaining, contribution) {
        (Some(0), _) => (ForecastStatus::Funded, Some(0), None),
        (Some(_), Some(0)) => (ForecastStatus::NoContribution, None, None),
        (Some(left), Some(amount)) => {
            let months = (left + amount - 1) / amount;
            if months > i64::from(HORIZON) {
                (ForecastStatus::HorizonReached, None, None)
            } else {
                (
                    ForecastStatus::Projected,
                    Some(months as u32),
                    Some(end_date(start + months as i32 - 1).to_string()),
                )
            }
        }
        _ => (ForecastStatus::Unknown, None, None),
    };
    Forecast {
        status,
        remaining: remaining.map(money),
        contribution: contribution.map(money),
        months_needed: months,
        completion_date: completion,
    }
}

struct GoalTerms {
    remaining: Option<i64>,
    contribution: Option<i64>,
}

pub(crate) fn debt_reserved(context: &Context, number: i32) -> Result<Option<i64>, String> {
    let Some(budget_debt) = aggregate(&context.budget_debt_payments)? else {
        return Ok(None);
    };
    if context.debt_revision == 0 {
        return Ok(Some(budget_debt));
    }
    let Some(start) = context.debt_start_month.as_ref() else {
        return Ok(None);
    };
    if number < month_number(start)? {
        return Ok(Some(budget_debt));
    }
    let base = context
        .debt_changes
        .iter()
        .rfind(|v| month_number(&v.month).is_ok_and(|m| m <= number))
        .map(|v| v.amount.as_str())
        .or(context.debt_monthly_payment.as_deref());
    let Some(base) = base else {
        return Ok(None);
    };
    let mut planned = parse_money(base)?;
    for extra in &context.debt_extras {
        if month_number(&extra.month)? == number {
            planned += parse_money(&extra.amount)?;
        }
    }
    // Reserve the higher of the budget allocation and the saved debt plan,
    // rather than silently freeing money from conflicting plans or payoff estimates.
    Ok(Some(budget_debt.max(planned)))
}

fn schedule(
    remaining: Option<i64>,
    contribution: Option<i64>,
    goals: &[GoalTerms],
    start: i32,
    context: &Context,
) -> Result<Vec<Month>, String> {
    let (Some(mut emergency), Some(contribution)) = (remaining, contribution) else {
        return Ok(vec![]);
    };
    if goals
        .iter()
        .any(|g| g.remaining.is_none() || g.contribution.is_none())
    {
        return Ok(vec![]);
    }
    let mut left: Vec<i64> = goals.iter().map(|g| g.remaining.unwrap_or(0)).collect();
    let surplus = aggregate(&context.monthly_surplus)?;
    let mut rows = Vec::new();
    for number in start..start + HORIZON {
        let transferred = emergency.min(contribution);
        emergency -= transferred;
        let goal_transfers: Vec<i64> = goals
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let amount = left[i].min(g.contribution.unwrap_or(0));
                left[i] -= amount;
                amount
            })
            .collect();
        let total = transferred + goal_transfers.iter().sum::<i64>();
        let debt = debt_reserved(context, number)?;
        let unallocated = surplus
            .zip(debt)
            .map(|(income, debt)| income - debt - total);
        rows.push(Month {
            month: format!("{:04}-{:02}", number / 12, number % 12 + 1),
            emergency_contribution: money(transferred),
            goal_contributions: goal_transfers.iter().copied().map(money).collect(),
            total_savings: money(total),
            debt_reserved: debt.map(money),
            unallocated: unallocated.map(money),
        });
        let future = context
            .debt_changes
            .iter()
            .chain(context.debt_extras.iter())
            .any(|e| month_number(&e.month).is_ok_and(|m| m > number && m < start + HORIZON));
        if emergency == 0 && left.iter().all(|v| *v == 0) && !future {
            break;
        }
        // If zero contributions keep some goals unfunded, continue only until
        // known future debt events have been checked; no completion date is invented.
        if transferred == 0 && goal_transfers.iter().all(|v| *v == 0) && !future {
            break;
        }
    }
    Ok(rows)
}

pub fn calculate(mut data: PlanData, context: &Context) -> Result<(PlanData, Summary), String> {
    let start = month_number(&data.start_month)?;
    if !(1..=60).contains(&data.target_months) {
        return Err("Emergency coverage targets must be 1–60 whole months.".to_owned());
    }
    if data.goals.len() > 50 || data.alternative_contributions.len() > 5 {
        return Err("Use at most 50 sinking funds and five alternative contributions.".to_owned());
    }
    data.priorities_note = data.priorities_note.trim().to_owned();
    if data.priorities_note.chars().count() > 1000
        || data
            .priorities_note
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err("Funding priorities must be at most 1000 characters without unsupported control characters.".to_owned());
    }
    let liquid = optional_money(&mut data.liquid_savings)?;
    let restricted = optional_money(&mut data.restricted_savings)?;
    let obligations = optional_money(&mut data.obligations_reserved)?;
    let contribution = optional_money(&mut data.emergency_contribution)?;
    let mut warnings = context.warnings.clone();
    warnings.push("Estimates assume month-end contributions, no interest/returns, withdrawals, inflation, or changes in expenses. Dates are not guarantees of affordability.".to_owned());
    if restricted.is_none() {
        warnings.push(
            "Restricted savings are unknown and are excluded from emergency coverage.".to_owned(),
        );
    }
    let mut reserved_total = Some(0_i64);
    let mut terms = Vec::new();
    let mut results = Vec::new();
    for (index, goal) in data.goals.iter_mut().enumerate() {
        goal.label = goal.label.trim().to_owned();
        if goal.label.is_empty()
            || goal.label.chars().count() > 120
            || goal.label.chars().any(char::is_control)
        {
            return Err(
                "Each sinking fund needs a name of 1–120 characters without control characters."
                    .to_owned(),
            );
        }
        let target = optional_money(&mut goal.target_amount)?;
        let reserve = optional_money(&mut goal.reserved_amount)?;
        let contribution = optional_money(&mut goal.monthly_contribution)?;
        reserved_total = reserved_total
            .zip(reserve)
            .map(|(total, value)| total + value);
        let remaining = target
            .zip(reserve)
            .map(|(target, reserved)| (target - reserved).max(0));
        let deadline = goal.target_date.as_deref().map(parse_date).transpose()?;
        if deadline.is_some_and(|date| date > end_date(start + HORIZON - 1)) {
            return Err(
                "Goal target dates must be within the 600-month planning horizon.".to_owned(),
            );
        }
        let mut goal_warnings = Vec::new();
        let first_date = NaiveDate::from_ymd_opt(start / 12, (start % 12 + 1) as u32, 1)
            .expect("bounded start month");
        if deadline.is_some_and(|date| date < first_date) {
            goal_warnings.push("The target date precedes the plan; historical funding by that date is not verified.".to_owned());
        }
        if target.is_none() || reserve.is_none() || contribution.is_none() {
            goal_warnings.push(
                "Goal amounts are incomplete; no missing amount is treated as zero.".to_owned(),
            );
        }
        let slots = deadline.map(|date| {
            let month = date.year() * 12 + date.month() as i32 - 1;
            let last = if end_date(month) <= date {
                month
            } else {
                month - 1
            };
            (last - start + 1).max(0) as u32
        });
        let suggested = match (remaining, slots) {
            (Some(0), _) => Some(0),
            (Some(left), Some(months)) if months > 0 => {
                Some((left + i64::from(months) - 1) / i64::from(months))
            }
            _ => None,
        };
        let prediction = forecast(remaining, contribution, start);
        let on_track = match (remaining, deadline, slots, contribution) {
            (Some(0), Some(date), _, _) => {
                if date < first_date {
                    None
                } else {
                    Some(true)
                }
            }
            (Some(_), Some(_), Some(0), _) => Some(false),
            (Some(_), Some(_), Some(_), Some(amount)) => suggested.map(|needed| amount >= needed),
            _ => None,
        };
        if deadline.is_none() {
            goal_warnings.push(
                "No target date entered; no deadline-based suggested contribution is calculated."
                    .to_owned(),
            );
        }
        if slots == Some(0) && remaining.is_some_and(|v| v > 0) {
            goal_warnings.push("No month-end contribution fits before this deadline; review immediate funding or the target date.".to_owned());
        }
        if on_track == Some(false) {
            goal_warnings.push(
                "The entered contribution does not fund this goal by its target date.".to_owned(),
            );
        }
        if !goal_warnings.is_empty() {
            warnings.push(format!(
                "Review sinking fund {} for missing inputs or deadline warnings.",
                index + 1
            ));
        }
        terms.push(GoalTerms {
            remaining,
            contribution,
        });
        results.push(GoalResult {
            suggested_contribution: suggested.map(money),
            contribution_months_before_due: slots,
            on_track,
            forecast: prediction,
            warnings: goal_warnings,
        });
    }
    let known_reserves = obligations.unwrap_or(0)
        + data
            .goals
            .iter()
            .filter_map(|g| g.reserved_amount.as_deref())
            .map(parse_money)
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .sum::<i64>();
    if liquid.is_some_and(|total| known_reserves > total) {
        return Err("Bill reserves and sinking-fund reserves exceed total accessible savings. These buckets must not double-count the same money.".to_owned());
    }
    let accessible = liquid
        .zip(obligations)
        .zip(reserved_total)
        .map(|((total, bills), funds)| total - bills - funds);
    if accessible.is_none() {
        warnings.push("Accessible savings or reserve amounts are missing; emergency money cannot be verified.".to_owned());
    }
    let expense = aggregate(&context.necessary_expenses)?.filter(|v| *v > 0);
    if expense.is_none() {
        warnings.push("A positive, complete necessary-expense base is required from the saved budget for emergency coverage and targets.".to_owned());
    }
    let target = expense.map(|expense| expense * i64::from(data.target_months));
    let remaining = target
        .zip(accessible)
        .map(|(target, available)| (target - available).max(0));
    let coverage = accessible.zip(expense).map(|(available, expense)| {
        money(((i128::from(available) * 100) / i128::from(expense)) as i64)
    });
    let emergency = forecast(remaining, contribution, start);
    let rows = schedule(remaining, contribution, &terms, start, context)?;
    let mut requested = contribution;
    for goal in &terms {
        requested = requested
            .zip(goal.contribution)
            .map(|(total, amount)| total + amount);
    }
    if requested.is_none() {
        warnings.push(
            "Monthly contributions are incomplete; a complete allocation check is unavailable."
                .to_owned(),
        );
    }
    if requested
        .zip(aggregate(&context.budget_savings)?)
        .is_some_and(|(selected, budget)| selected != budget)
    {
        warnings.push("Entered savings contributions differ from the saved budget's savings allocation. Reconcile the plans; no budget is changed automatically.".to_owned());
    }
    if rows
        .iter()
        .any(|r| r.unallocated.as_ref().is_some_and(|v| v.starts_with('-')))
    {
        warnings.push("Savings and reserved debt payments exceed the saved budget's surplus in one or more projected months. Reduce allocations or confirm additional funding.".to_owned());
    }
    if context.monthly_surplus.is_none()
        || rows.is_empty()
        || rows.iter().any(|r| r.debt_reserved.is_none())
    {
        warnings.push("A complete monthly affordability check is unavailable. Review missing savings, budget, or debt inputs.".to_owned());
    }
    let mut alternatives = Vec::new();
    for amount in &mut data.alternative_contributions {
        let cents = parse_money(amount.trim())?;
        *amount = money(cents);
        let alternative_rows = schedule(remaining, Some(cents), &terms, start, context)?;
        alternatives.push(Scenario {
            forecast: forecast(remaining, Some(cents), start),
            allocation_warning: if alternative_rows.is_empty()
                || alternative_rows.iter().any(|r| r.unallocated.is_none())
            {
                None
            } else {
                Some(
                    alternative_rows
                        .iter()
                        .any(|r| r.unallocated.as_ref().is_some_and(|v| v.starts_with('-'))),
                )
            },
        });
    }
    Ok((
        data,
        Summary {
            accessible_emergency: accessible.map(money),
            obligations_reserved: obligations.map(money),
            sinking_funds_reserved: reserved_total.map(money),
            restricted_savings: restricted.map(money),
            coverage_months: coverage,
            emergency_target: target.map(money),
            emergency,
            alternatives,
            goals: results,
            requested_monthly_savings: requested.map(money),
            warnings,
            schedule: rows,
        },
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn fixture() -> PlanData {
        PlanData {
            start_month: "2026-11".to_owned(),
            target_months: 3,
            liquid_savings: Some("2600".to_owned()),
            restricted_savings: Some("5000".to_owned()),
            obligations_reserved: Some("600".to_owned()),
            emergency_contribution: Some("200".to_owned()),
            alternative_contributions: vec!["100".to_owned(), "300".to_owned(), "1500".to_owned()],
            goals: vec![Goal {
                label: "Synthetic insurance".to_owned(),
                target_amount: Some("1200".to_owned()),
                target_date: Some("2027-02-28".to_owned()),
                reserved_amount: Some("1000".to_owned()),
                monthly_contribution: Some("50".to_owned()),
            }],
            priorities_note: "Protect bills and debt payments while building emergency coverage."
                .to_owned(),
        }
    }
    fn context() -> Context {
        Context {
            budget_revision: 1,
            debt_revision: 1,
            necessary_expenses: Some("1633.33".to_owned()),
            monthly_surplus: Some("1416.67".to_owned()),
            budget_savings: Some("300.00".to_owned()),
            budget_debt_payments: Some("400.00".to_owned()),
            debt_start_month: Some("2026-11".to_owned()),
            debt_monthly_payment: Some("100.00".to_owned()),
            debt_changes: vec![],
            debt_extras: vec![],
            warnings: vec![],
        }
    }
    #[test]
    fn independent_emergency_buckets_targets_dates_and_last_cent() {
        let (_, s) = calculate(fixture(), &context()).unwrap();
        assert_eq!(s.accessible_emergency.as_deref(), Some("1000.00"));
        assert_eq!(s.coverage_months.as_deref(), Some("0.61"));
        assert_eq!(s.emergency_target.as_deref(), Some("4899.99"));
        assert_eq!(s.emergency.remaining.as_deref(), Some("3899.99"));
        assert_eq!(s.emergency.months_needed, Some(20));
        assert_eq!(s.emergency.completion_date.as_deref(), Some("2028-06-30"));
        assert_eq!(s.schedule[0].total_savings, "250.00");
        assert_eq!(s.schedule[0].debt_reserved.as_deref(), Some("400.00"));
        assert_eq!(s.schedule[0].unallocated.as_deref(), Some("766.67"));
        assert_eq!(s.schedule.last().unwrap().emergency_contribution, "99.99");
        assert_eq!(s.schedule[4].goal_contributions, vec!["0.00"]);
        let mut changed = fixture();
        changed.restricted_savings = Some("999999999".to_owned());
        assert_eq!(
            calculate(changed, &context())
                .unwrap()
                .1
                .accessible_emergency,
            s.accessible_emergency
        );
    }
    #[test]
    fn independent_goal_deadline_and_upward_cent_rounding() {
        let (_, s) = calculate(fixture(), &context()).unwrap();
        let goal = &s.goals[0];
        assert_eq!(goal.suggested_contribution.as_deref(), Some("50.00"));
        assert_eq!(goal.contribution_months_before_due, Some(4));
        assert_eq!(goal.forecast.completion_date.as_deref(), Some("2027-02-28"));
        assert_eq!(goal.on_track, Some(true));
        let mut data = fixture();
        data.goals[0].target_date = Some("2027-02-15".to_owned());
        let (_, s) = calculate(data, &context()).unwrap();
        assert_eq!(s.goals[0].suggested_contribution.as_deref(), Some("66.67"));
        assert_eq!(s.goals[0].contribution_months_before_due, Some(3));
        assert_eq!(s.goals[0].on_track, Some(false));
        assert_eq!(
            s.goals[0].forecast.completion_date.as_deref(),
            Some("2027-02-28")
        );
    }
    #[test]
    fn alternative_contributions_keep_same_funds_and_warn_overallocation() {
        let (canonical, s) = calculate(fixture(), &context()).unwrap();
        assert_eq!(
            canonical.alternative_contributions,
            vec!["100.00", "300.00", "1500.00"]
        );
        assert_eq!(s.alternatives[0].forecast.months_needed, Some(39));
        assert_eq!(
            s.alternatives[0].forecast.completion_date.as_deref(),
            Some("2030-01-31")
        );
        assert_eq!(s.alternatives[1].forecast.months_needed, Some(13));
        assert_eq!(
            s.alternatives[1].forecast.completion_date.as_deref(),
            Some("2027-11-30")
        );
        assert_eq!(s.alternatives[2].forecast.months_needed, Some(3));
        assert_eq!(s.alternatives[2].allocation_warning, Some(true));
        assert_eq!(s.alternatives[0].allocation_warning, Some(false));
        assert_eq!(s.emergency.contribution.as_deref(), Some("200.00"));
    }
    #[test]
    fn unknown_zero_and_funded_dates_are_not_fabricated() {
        let mut data = fixture();
        data.liquid_savings = None;
        let (_, s) = calculate(data, &context()).unwrap();
        assert!(s.accessible_emergency.is_none());
        assert_eq!(s.emergency.status, ForecastStatus::Unknown);
        assert_eq!(s.alternatives[0].allocation_warning, None);
        let mut data = fixture();
        data.emergency_contribution = Some("0".to_owned());
        let (_, s) = calculate(data, &context()).unwrap();
        assert_eq!(s.emergency.status, ForecastStatus::NoContribution);
        assert!(s.emergency.completion_date.is_none());
        let mut data = fixture();
        data.liquid_savings = Some("6500".to_owned());
        let (_, s) = calculate(data, &context()).unwrap();
        assert_eq!(s.emergency.status, ForecastStatus::Funded);
        assert_eq!(s.emergency.months_needed, Some(0));
        assert!(s.emergency.completion_date.is_none());
        assert_eq!(s.schedule[0].emergency_contribution, "0.00");
        let mut c = context();
        c.necessary_expenses = Some("0.00".to_owned());
        assert!(calculate(fixture(), &c)
            .unwrap()
            .1
            .emergency_target
            .is_none());
        let mut c = context();
        c.monthly_surplus = None;
        assert_eq!(
            calculate(fixture(), &c).unwrap().1.alternatives[0].allocation_warning,
            None
        );
    }
    #[test]
    fn reserves_cannot_double_count_the_liquid_pool_including_partial_inputs() {
        let mut data = fixture();
        data.liquid_savings = Some("1599.99".to_owned());
        assert!(calculate(data, &context())
            .unwrap_err()
            .contains("double-count"));
        let mut data = fixture();
        data.liquid_savings = Some("1600".to_owned());
        assert_eq!(
            calculate(data, &context())
                .unwrap()
                .1
                .accessible_emergency
                .as_deref(),
            Some("0.00")
        );
        let mut data = fixture();
        data.goals[0].reserved_amount = None;
        let (_, s) = calculate(data, &context()).unwrap();
        assert!(s.accessible_emergency.is_none());
        assert_eq!(s.goals[0].forecast.status, ForecastStatus::Unknown);
    }
    #[test]
    fn future_debt_commitments_remain_reserved_even_after_savings_finish() {
        let mut c = context();
        c.debt_extras.push(DatedAmount {
            month: "2026-12".to_owned(),
            amount: "1500.00".to_owned(),
        });
        let (_, s) = calculate(fixture(), &c).unwrap();
        assert_eq!(s.schedule[1].debt_reserved.as_deref(), Some("1600.00"));
        assert_eq!(s.schedule[1].unallocated.as_deref(), Some("-433.33"));
        assert!(s.warnings.iter().any(|w| w.contains("exceed")));
        let mut data = fixture();
        data.liquid_savings = Some("10000".to_owned());
        data.goals[0].target_amount = Some("1000".to_owned());
        let (_, s) = calculate(data, &c).unwrap();
        assert_eq!(s.schedule.len(), 2);
        assert_eq!(s.schedule[1].total_savings, "0.00");
        assert_eq!(s.schedule[1].unallocated.as_deref(), Some("-183.33"));
    }
    #[test]
    fn leap_days_early_deadlines_and_past_funding_history_are_explicit() {
        let mut data = fixture();
        data.start_month = "2028-02".to_owned();
        data.goals[0].target_date = Some("2028-02-29".to_owned());
        let (_, s) = calculate(data.clone(), &context()).unwrap();
        assert_eq!(s.goals[0].contribution_months_before_due, Some(1));
        data.goals[0].target_date = Some("2028-02-28".to_owned());
        let (_, s) = calculate(data, &context()).unwrap();
        assert_eq!(s.goals[0].contribution_months_before_due, Some(0));
        assert_eq!(s.goals[0].suggested_contribution, None);
        assert_eq!(s.goals[0].on_track, Some(false));
        let mut data = fixture();
        data.goals[0].target_amount = Some("1000".to_owned());
        data.goals[0].target_date = Some("2025-01-01".to_owned());
        let (_, s) = calculate(data, &context()).unwrap();
        assert_eq!(s.goals[0].forecast.status, ForecastStatus::Funded);
        assert_eq!(s.goals[0].on_track, None);
        assert!(s.goals[0].forecast.completion_date.is_none());
    }
    #[test]
    fn precision_dates_limits_and_long_goals_are_validated() {
        for date in [
            "2027-02-29",
            "2028-02-30",
            "2027-2-28",
            "2027-+2-28",
            "2027-02-28x",
            "2101-01-01",
        ] {
            let mut data = fixture();
            data.goals[0].target_date = Some(date.to_owned());
            assert!(calculate(data, &context()).is_err());
        }
        for amount in ["-1", "1.001", "1e3", "1000000000.01"] {
            let mut data = fixture();
            data.emergency_contribution = Some(amount.to_owned());
            assert!(calculate(data, &context()).is_err());
        }
        for months in [0, 61] {
            let mut data = fixture();
            data.target_months = months;
            assert!(calculate(data, &context()).is_err());
        }
        let mut data = fixture();
        data.emergency_contribution = Some("0.01".to_owned());
        let (_, s) = calculate(data, &context()).unwrap();
        assert_eq!(s.emergency.status, ForecastStatus::HorizonReached);
        assert!(s.emergency.completion_date.is_none());
        assert_eq!(s.schedule.len(), 600);
        let mut data = fixture();
        data.target_months = 60;
        let c = Context {
            necessary_expenses: Some("433333333333.00".to_owned()),
            ..context()
        };
        assert_eq!(
            calculate(data, &c).unwrap().1.emergency_target.as_deref(),
            Some("25999999999980.00")
        );
    }
}
