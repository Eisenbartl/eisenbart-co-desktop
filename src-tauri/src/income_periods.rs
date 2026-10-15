//! Bounded temporary-income funding checks; continuing commitments are never silently reduced.
use crate::{budget, cashflow, debt, planning, savings, scenarios::CashChange};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reduction {
    pub budget_index: usize,
    pub from_month: String,
    pub through_month: String,
    pub amount: Option<String>,
    pub cash_schedule: Option<usize>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Month {
    pub month: String,
    pub reduced: bool,
    pub normal_income: Option<String>,
    pub income: Option<String>,
    pub debt_reserved: Option<String>,
    pub savings_reserved: Option<String>,
    pub normal_buffer: Option<String>,
    pub buffer: Option<String>,
}
fn income(b: &budget::ClientBudget) -> Option<String> {
    let lines: Vec<_> = b
        .lines
        .iter()
        .filter(|l| l.kind == budget::Kind::Income)
        .collect();
    (!lines.is_empty() && lines.iter().all(|l| l.amount.is_some()))
        .then(|| b.summary.monthly_income.clone())
}
fn surplus(b: &budget::ClientBudget, context: &savings::Context) -> Result<Option<i64>, String> {
    if context.monthly_surplus.is_none() {
        return Ok(None);
    }
    Ok(Some(
        budget::parse_money(&b.summary.monthly_income)?
            - budget::parse_money(&b.summary.total_spending)?,
    ))
}
fn scheduled_savings(p: &planning::Plans, month: i32) -> Result<Option<i64>, String> {
    let Some(data) = &p.savings.data else {
        return p
            .savings
            .context
            .budget_savings
            .as_deref()
            .map(budget::parse_money)
            .transpose();
    };
    let start = debt::month_number(&data.start_month)?;
    if !(start..start + 600).contains(&month) {
        return Ok(None);
    }
    let Some(summary) = &p.savings.summary else {
        return Ok(None);
    };
    if let Some(row) = summary
        .schedule
        .iter()
        .find(|r| debt::month_number(&r.month).ok() == Some(month))
    {
        return budget::parse_money(&row.total_savings).map(Some);
    }
    // A nonempty forecast stops only when targets are funded or all further contributions are zero.
    Ok(summary
        .schedule
        .last()
        .filter(|r| debt::month_number(&r.month).is_ok_and(|m| m < month))
        .map(|_| 0))
}
fn frequency(f: budget::Frequency) -> Result<cashflow::Frequency, String> {
    match f {
        budget::Frequency::Weekly => Ok(cashflow::Frequency::Weekly),
        budget::Frequency::Biweekly => Ok(cashflow::Frequency::Biweekly),
        budget::Frequency::Semimonthly => Ok(cashflow::Frequency::Semimonthly),
        budget::Frequency::Monthly => Ok(cashflow::Frequency::Monthly),
        _ => Err(
            "Temporary reductions support regular weekly, biweekly, semimonthly or monthly income."
                .into(),
        ),
    }
}
pub fn apply(
    p: &mut planning::Plans,
    inputs: &mut [Reduction],
    edits: &[CashChange],
) -> Result<Vec<Month>, String> {
    if inputs.is_empty() {
        return Ok(vec![]);
    }
    if inputs.len() > 6 {
        return Err("Use at most six temporary income reductions.".into());
    }
    let mut occupied = HashSet::new();
    let mut cash_months = HashSet::new();
    let mut bounds = vec![];
    for input in inputs.iter_mut() {
        let from = debt::month_number(&input.from_month)?;
        let through = debt::month_number(&input.through_month)?;
        if through < from || through - from >= 24 {
            return Err("Choose an ordered temporary income period of at most 24 months.".into());
        }
        let line = p
            .budget
            .lines
            .get(input.budget_index)
            .filter(|l| l.kind == budget::Kind::Income)
            .ok_or("Choose an existing income budget line for each temporary reduction.")?;
        let f = frequency(line.frequency)?;
        let normal = line
            .amount
            .as_deref()
            .ok_or("Enter a known normal income amount before modeling a temporary reduction.")?;
        let normal = budget::parse_money(normal)?;
        if let Some(amount) = &mut input.amount {
            let cents = budget::parse_money(amount.trim())?;
            if cents >= normal {
                return Err("A temporary reduction must be below the normal proposed income amount; blank means unknown.".into());
            }
            *amount = budget::money(cents);
        }
        for month in from..=through {
            if !occupied.insert((input.budget_index, month)) {
                return Err("Temporary reductions for the same income line cannot overlap.".into());
            }
        }
        if let Some(index) = input.cash_schedule {
            if edits.iter().any(|e| e.index == index) {
                return Err("Do not also edit a paycheck schedule selected for a temporary income reduction.".into());
            }
            let schedule = p
                .cashflow
                .data
                .as_ref()
                .and_then(|d| d.schedules.get(index))
                .filter(|s| {
                    matches!(
                        s.kind,
                        cashflow::Kind::Paycheck | cashflow::Kind::OtherInflow
                    )
                })
                .ok_or(
                    "Choose an existing income schedule in this client's saved cash-flow model.",
                )?;
            if schedule.frequency != f {
                return Err(
                    "The selected paycheck frequency must match the income budget line.".into(),
                );
            }
            if let Some(amount) = &input.amount {
                let paycheck = schedule
                    .amount
                    .as_deref()
                    .map(budget::parse_money)
                    .transpose()?;
                let reduced = budget::parse_money(amount)?;
                if paycheck.is_none_or(|v| reduced >= v) {
                    return Err(
                        "The temporary amount must also be below a known selected paycheck amount."
                            .into(),
                    );
                }
            }
            for month in from..=through {
                if !cash_months.insert((index, month)) {
                    return Err(
                        "Temporary reductions cannot overlap on the same paycheck schedule.".into(),
                    );
                }
            }
        }
        bounds.push((from, through));
    }
    let start = bounds
        .iter()
        .map(|b| b.0)
        .min()
        .ok_or("Missing income period.")?;
    let last = bounds
        .iter()
        .map(|b| b.1)
        .max()
        .ok_or("Missing income period.")?;
    if last - start >= 24 {
        return Err("All temporary income periods must fit within a 24-month span.".into());
    }
    // Include one normal recovery month when the supported date range permits it.
    let end = (last + 1).min(2100 * 12 + 11);
    let mut months = vec![];
    for month in start..=end {
        let mut lines = p.budget.lines.clone();
        let mut reduced = false;
        for (input, (from, through)) in inputs.iter().zip(&bounds) {
            if (*from..=*through).contains(&month) {
                lines[input.budget_index].amount = input.amount.clone();
                lines[input.budget_index].estimated = true;
                reduced = true;
            }
        }
        let (lines, summary) = budget::calculate(lines)?;
        let monthly = budget::ClientBudget {
            client_id: p.client_id.clone(),
            revision: p.budget.revision,
            updated_at: p.budget.updated_at.clone(),
            lines,
            summary,
        };
        let context = planning::savings_context(&monthly, &p.debt);
        let reserved = savings::debt_reserved(&p.savings.context, month)?;
        let saved = scheduled_savings(p, month)?;
        let normal = surplus(&p.budget, &p.savings.context)?
            .zip(reserved)
            .zip(saved)
            .map(|((a, d), s)| budget::money(a - d - s));
        let buffer = surplus(&monthly, &context)?
            .zip(reserved)
            .zip(saved)
            .map(|((a, d), s)| budget::money(a - d - s));
        months.push(Month {
            month: format!("{:04}-{:02}", month / 12, month % 12 + 1),
            reduced,
            normal_income: income(&p.budget),
            income: income(&monthly),
            debt_reserved: reserved.map(budget::money),
            savings_reserved: saved.map(budget::money),
            normal_buffer: normal,
            buffer,
        });
    }
    if inputs.iter().any(|i| i.cash_schedule.is_some()) {
        let mut data = p
            .cashflow
            .data
            .take()
            .ok_or("Save a cash-flow model before selecting paycheck schedules.")?;
        let summary = p
            .cashflow
            .summary
            .as_ref()
            .ok_or("The dated baseline is unavailable.")?;
        let mut schedules = vec![];
        for (index, schedule) in data.schedules.into_iter().enumerate() {
            if !inputs.iter().any(|i| i.cash_schedule == Some(index)) {
                schedules.push(schedule);
                continue;
            }
            let occurrences: Vec<_> = summary
                .ledger
                .iter()
                .filter(|e| e.schedule_index == index)
                .collect();
            if !occurrences.iter().any(|row| {
                inputs.iter().any(|i| {
                    i.cash_schedule == Some(index)
                        && i.from_month.as_str() <= &row.date[..7]
                        && i.through_month.as_str() >= &row.date[..7]
                })
            }) {
                schedules.push(schedule);
                continue;
            }
            for row in occurrences {
                let mut s = schedule.clone();
                s.first_date = row.date.clone();
                s.frequency = cashflow::Frequency::Once;
                s.second_day = None;
                s.until_date = None;
                if let Some(input) = inputs.iter().find(|i| {
                    i.cash_schedule == Some(index)
                        && i.from_month.as_str() <= &row.date[..7]
                        && i.through_month.as_str() >= &row.date[..7]
                }) {
                    s.amount = input.amount.clone();
                    s.certainty = cashflow::Certainty::Estimated;
                }
                schedules.push(s);
            }
        }
        if schedules.len() > 100 {
            return Err("Temporary paycheck adjustments exceed 100 dated schedules. Shorten the saved forecast or reduce selected schedules.".into());
        }
        data.schedules = schedules;
        let (data, summary) = cashflow::calculate(data)?;
        p.cashflow.data = Some(data);
        p.cashflow.summary = Some(summary);
    }
    Ok(months)
}
