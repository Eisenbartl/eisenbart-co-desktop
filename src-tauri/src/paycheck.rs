//! Read-only earmarks from saved monthly budget priorities and actual entered payday occurrences.
use crate::{budget, cashflow, planning};
use chrono::{Datelike, NaiveDate};
use serde::Serialize;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Allocation {
    pub client_id: String,
    pub revisions: planning::Revisions,
    pub labels: Vec<String>,
    pub rows: Vec<Row>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub date: String,
    pub label: String,
    pub account_id: String,
    pub certainty: cashflow::Certainty,
    pub paycheck: Option<String>,
    pub allocations: Vec<Option<String>>,
    pub total: Option<String>,
    pub remainder: Option<String>,
    pub next_paycheck: Option<String>,
    pub dated_commitments: Option<String>,
    pub warnings: Vec<String>,
}
pub fn allocate(plans: &planning::Plans) -> Result<Allocation, String> {
    let lines: Vec<_> = plans
        .budget
        .lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.kind != budget::Kind::Income)
        .collect();
    let mut result = Allocation { client_id:plans.client_id.clone(), revisions:plans.revisions(), labels:lines.iter().map(|(_,l)|l.label.clone()).collect(), rows:vec![], warnings:vec!["Read-only equal earmarks across every entered paycheck in each complete calendar month. Earlier paychecks receive remainder cents. No schedule, payment, transaction or saved plan is created. Dated commitments are shown separately, not added to budget earmarks. Other deposits/opening balances are not allocated. Review account availability and priorities before agreeing to payments.".into()] };
    let (Some(data), Some(summary)) = (&plans.cashflow.data, &plans.cashflow.summary) else {
        result.warnings.push("Save dated paychecks in the cash-flow calendar first; pay dates are never inferred from monthly averages.".into());
        return Ok(result);
    };
    let checks: Vec<_> = summary
        .ledger
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == cashflow::Kind::Paycheck)
        .collect();
    for (position, (ledger_index, e)) in checks.iter().enumerate() {
        let date = NaiveDate::parse_from_str(&e.date, "%Y-%m-%d")
            .map_err(|_| "Invalid payday context.")?;
        let month = &e.date[..7];
        let members: Vec<_> = checks
            .iter()
            .enumerate()
            .filter(|(_, (_, v))| v.date.starts_with(month))
            .map(|(i, _)| i)
            .collect();
        let rank = members
            .iter()
            .position(|i| *i == position)
            .expect("payday member");
        let following = if date.month() == 12 {
            NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)
        }
        .expect("valid month");
        let complete = data.start_date <= format!("{month}-01")
            && data.end_date >= following.pred_opt().expect("month end").to_string();
        let allocations: Vec<_> = lines
            .iter()
            .map(|(i, l)| {
                if !complete || plans.budget.revision == 0 || l.amount.is_none() {
                    return Ok(None);
                }
                let Some(normalized) = plans.budget.summary.monthly_amounts[*i].as_deref() else {
                    return Ok(None);
                };
                let value = budget::parse_total(normalized)?;
                Ok(Some(budget::money(
                    value / members.len() as i64
                        + i64::from((rank as i64) < value % members.len() as i64),
                )))
            })
            .collect::<Result<_, String>>()?;
        let total = if allocations.is_empty() && plans.budget.revision == 0 {
            None
        } else {
            allocations
                .iter()
                .map(|v| v.as_deref().map(budget::parse_total).transpose())
                .collect::<Result<Option<Vec<_>>, String>>()?
                .map(|v| v.iter().sum::<i64>())
        };
        let paycheck = e.amount.as_deref().map(budget::parse_money).transpose()?;
        let next = checks
            .iter()
            .skip(position + 1)
            .find(|(_, n)| n.account_id == e.account_id);
        let through = next.map(|(i, _)| *i).unwrap_or(summary.ledger.len());
        let commitments = summary
            .ledger
            .iter()
            .take(through)
            .skip(ledger_index + 1)
            .filter(|v| {
                v.account_id == e.account_id
                    && v.transfer_leg.as_deref() != Some("receipt")
                    && matches!(
                        v.kind,
                        cashflow::Kind::Bill
                            | cashflow::Kind::Loan
                            | cashflow::Kind::Savings
                            | cashflow::Kind::OtherOutflow
                            | cashflow::Kind::Transfer
                    )
            })
            .map(|v| v.amount.as_deref().map(budget::parse_money).transpose())
            .collect::<Result<Option<Vec<_>>, String>>()?
            .map(|v| v.iter().sum::<i64>());
        let mut warnings = e.warnings.clone();
        if !complete {
            warnings.push("Partial month: monthly budget earmarks unavailable. Expand the calendar to the full month.".into());
        }
        if e.certainty == cashflow::Certainty::Estimated {
            warnings.push("Estimated paycheck; verify availability before allocating.".into());
        }
        if paycheck.zip(total).is_some_and(|(p, t)| p < t) {
            warnings.push("Budget earmarks exceed this paycheck. Review priorities, other funding and the dated account forecast.".into());
        }
        if next.is_none() {
            warnings.push("No following paycheck entered for this account; commitments stop at the forecast end.".into());
        }
        result.rows.push(Row {
            date: e.date.clone(),
            label: e.label.clone(),
            account_id: e.account_id.clone(),
            certainty: e.certainty.clone(),
            paycheck: e.amount.clone(),
            allocations,
            total: total.map(budget::money),
            remainder: paycheck.zip(total).map(|(p, t)| budget::money(p - t)),
            next_paycheck: next.map(|(_, n)| n.date.clone()),
            dated_commitments: commitments.map(budget::money),
            warnings,
        });
    }
    if checks.is_empty() {
        result.warnings.push("No paycheck occurs in the entered period. Add actual expected dates, including separate one-time entries for irregular income.".into());
    }
    Ok(result)
}
