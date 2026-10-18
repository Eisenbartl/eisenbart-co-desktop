//! Dated cash flow in integer cents. No inferred dates or automatic plan imports.
use crate::budget::{money, parse_money};
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub id: String,
    pub label: String,
    pub opening_balance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserve: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Kind {
    Paycheck,
    Bill,
    Loan,
    Savings,
    OtherInflow,
    OtherOutflow,
    Transfer,
}
impl Kind {
    fn inflow(&self) -> bool {
        matches!(self, Self::Paycheck | Self::OtherInflow)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Frequency {
    Once,
    Weekly,
    Biweekly,
    Semimonthly,
    Monthly,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Certainty {
    Confirmed,
    Estimated,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settlement {
    /// First receipt date; null explicitly means settlement is unknown.
    pub first_available_date: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Schedule {
    pub label: String,
    pub account_id: String,
    pub destination_id: Option<String>,
    pub kind: Kind,
    pub amount: Option<String>,
    pub first_date: String,
    pub frequency: Frequency,
    pub second_day: Option<u32>,
    pub until_date: Option<String>,
    pub certainty: Certainty,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<Settlement>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanData {
    pub start_date: String,
    pub end_date: String,
    pub accounts: Vec<Account>,
    pub schedules: Vec<Schedule>,
    pub notes: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanInput {
    pub expected_revision: i64,
    pub data: PlanData,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    pub budget_revision: i64,
    pub debt_revision: i64,
    pub savings_revision: i64,
    pub monthly_surplus: Option<String>,
    pub budget_debt: Option<String>,
    pub budget_savings: Option<String>,
    pub debt_monthly_payment: Option<String>,
    pub savings_monthly_contribution: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Balance {
    pub account_id: String,
    pub amount: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub schedule_index: usize,
    pub date: String,
    pub label: String,
    pub kind: Kind,
    pub certainty: Certainty,
    pub account_id: String,
    pub destination_id: Option<String>,
    pub amount: Option<String>,
    pub source_balance: Option<String>,
    pub destination_balance: Option<String>,
    pub combined_balance: Option<String>,
    pub next_paycheck: Option<String>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_leg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_transit: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Day {
    pub date: String,
    pub balances: Vec<Balance>,
    pub combined_balance: Option<String>,
    pub entry_indices: Vec<usize>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_transit: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub ledger: Vec<Entry>,
    pub days: Vec<Day>,
    pub closing_balances: Vec<Balance>,
    pub combined_closing: Option<String>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_transit: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientCashflowPlan {
    pub client_id: String,
    pub revision: i64,
    pub updated_at: Option<String>,
    pub data: Option<PlanData>,
    pub summary: Option<Summary>,
    pub context: Context,
}

pub(crate) fn date(value: &str) -> Result<NaiveDate, String> {
    if value.len() != 10
        || !value.is_ascii()
        || value.as_bytes()[4] != b'-'
        || value.as_bytes()[7] != b'-'
        || !value
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Err("Enter real dates as YYYY-MM-DD, between 1900 and 2100.".to_owned());
    }
    let result = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| "Enter a real calendar date.".to_owned())?;
    if !(1900..=2100).contains(&result.year()) {
        return Err("Dates must be between 1900 and 2100.".to_owned());
    }
    Ok(result)
}
fn label(value: &mut String) -> Result<(), String> {
    *value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > 120 || value.chars().any(char::is_control) {
        return Err(
            "Account and activity names need 1–120 characters without control characters."
                .to_owned(),
        );
    }
    Ok(())
}
fn cents(value: &mut Option<String>, signed: bool) -> Result<Option<i64>, String> {
    value
        .as_mut()
        .map(|v| {
            let text = v.trim();
            let negative = signed && text.starts_with('-');
            let magnitude = parse_money(if negative { &text[1..] } else { text })?;
            let amount = if negative { -magnitude } else { magnitude };
            *v = money(amount);
            Ok(amount)
        })
        .transpose()
}
fn clamped_day(year: i32, month: u32, day: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("bounded year");
    day.min(next.pred_opt().expect("bounded date").day())
}
pub(crate) fn occurs(schedule: &Schedule, first: NaiveDate, day: NaiveDate) -> bool {
    if day < first {
        return false;
    }
    let delta = (day - first).num_days();
    match schedule.frequency {
        Frequency::Once => day == first,
        Frequency::Weekly => delta % 7 == 0,
        Frequency::Biweekly => delta % 14 == 0,
        Frequency::Monthly => day.day() == clamped_day(day.year(), day.month(), first.day()),
        Frequency::Semimonthly => {
            day.day() == clamped_day(day.year(), day.month(), first.day())
                || day.day()
                    == clamped_day(
                        day.year(),
                        day.month(),
                        schedule.second_day.expect("validated second day"),
                    )
        }
    }
}
fn combined(balances: &[Option<i64>]) -> Option<String> {
    balances
        .iter()
        .copied()
        .collect::<Option<Vec<_>>>()
        .map(|v| money(v.iter().sum()))
}
fn balances(data: &PlanData, values: &[Option<i64>]) -> Vec<Balance> {
    data.accounts
        .iter()
        .zip(values)
        .map(|(a, v)| Balance {
            account_id: a.id.clone(),
            amount: v.map(money),
        })
        .collect()
}

pub fn calculate(mut data: PlanData) -> Result<(PlanData, Summary), String> {
    let start = date(&data.start_date)?;
    let end = date(&data.end_date)?;
    if end < start || (end - start).num_days() > 365 {
        return Err("Choose an inclusive planning period of 1–366 days.".to_owned());
    }
    if data.accounts.is_empty() || data.accounts.len() > 20 || data.schedules.len() > 100 {
        return Err("Use 1–20 accounts and at most 100 activity schedules.".to_owned());
    }
    data.notes = data.notes.trim().to_owned();
    if data.notes.chars().count() > 1000
        || data
            .notes
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(
            "Cash-flow notes must be at most 1,000 characters without unsupported controls."
                .to_owned(),
        );
    }
    let mut ids = HashSet::new();
    let mut values = Vec::new();
    for account in &mut data.accounts {
        if account.id.is_empty()
            || account.id.len() > 64
            || !account
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !ids.insert(account.id.clone())
        {
            return Err("Each account needs a unique valid identifier.".to_owned());
        }
        label(&mut account.label)?;
        values.push(cents(&mut account.opening_balance, true)?);
        cents(&mut account.reserve, false)?;
    }
    let mut terms = Vec::new();
    for s in &mut data.schedules {
        label(&mut s.label)?;
        let source = data
            .accounts
            .iter()
            .position(|a| a.id == s.account_id)
            .ok_or("Choose an existing source account for every activity.")?;
        let dest = if s.kind == Kind::Transfer {
            let index = data
                .accounts
                .iter()
                .position(|a| Some(&a.id) == s.destination_id.as_ref())
                .ok_or("Transfers need an existing destination account.")?;
            if source == index {
                return Err("A transfer needs two different accounts.".to_owned());
            }
            Some(index)
        } else {
            if s.destination_id.is_some() {
                return Err("Only account transfers have a destination account.".to_owned());
            }
            None
        };
        let first = date(&s.first_date)?;
        let until = s.until_date.as_deref().map(date).transpose()?;
        if until.is_some_and(|v| v < first) {
            return Err("A schedule's last date cannot precede its first date.".to_owned());
        }
        if s.frequency == Frequency::Semimonthly {
            if !s
                .second_day
                .is_some_and(|v| (1..=31).contains(&v) && v != first.day())
            {
                return Err(
                    "Semimonthly schedules need a different second day from 1–31.".to_owned(),
                );
            }
        } else if s.second_day.is_some() {
            return Err("A second day applies only to semimonthly schedules.".to_owned());
        }
        if let Some(settlement) = &s.settlement {
            if s.kind != Kind::Transfer {
                return Err("Settlement dates apply only to internal transfers.".into());
            }
            if let Some(receipt) = &settlement.first_available_date {
                let lag = (date(receipt)? - first).num_days();
                if !(0..=366).contains(&lag) {
                    return Err("Transfer availability must be on or within 366 calendar days after departure.".into());
                }
            }
        }
        let amount = cents(&mut s.amount, false)?;
        terms.push((source, dest, first, until, amount));
    }
    let delayed = data.schedules.iter().any(|s| s.settlement.is_some());
    let mut transit = Some(0_i64);
    let mut events = Vec::new();
    let mut day = start - Duration::days(366);
    loop {
        for (index, s) in data.schedules.iter().enumerate() {
            let (_, _, first, until, amount) = terms[index];
            if until.is_some_and(|v| day > v) || !occurs(s, first, day) {
                continue;
            }
            if day >= start {
                events.push((day, index, false));
            }
            if let Some(settlement) = &s.settlement {
                if let Some(receipt) = &settlement.first_available_date {
                    let available = day + (date(receipt)? - first);
                    if day < start && available >= start {
                        transit = transit.zip(amount).map(|(t, a)| t + a);
                    }
                    if (start..=end).contains(&available) {
                        events.push((available, index, true));
                    }
                } else if day < start {
                    // Opening account balances do not establish whether an earlier unknown transfer settled.
                    transit = None;
                    if let Some(dest) = terms[index].1 {
                        values[dest] = None;
                    }
                }
            }
        }
        if events.len() > 5000 {
            return Err(
                "The period generates more than 5,000 entries. Shorten it or reduce schedules."
                    .into(),
            );
        }
        if day == end {
            break;
        }
        day += Duration::days(1);
    }
    // Separate receipts are credits; every debit remains before same-day credits.
    events.sort_by_key(|(date, i, receipt)| {
        (*date, *receipt || data.schedules[*i].kind.inflow(), *i)
    });
    let mut warnings=vec!["Only entered activity is included. Monthly budgets and debt/savings forecasts supply references, never invented dates or automatic transactions. Confirm that every bill, payment, transfer, and account is included.".to_owned(),"Balances are before the first day's activity. Same-day debits precede credits; transfers settle immediately as one paired event. Bank processing, holds, holidays, interest, overdraft fees, and credit limits are not modeled.".to_owned()];
    if delayed {
        warnings[1] = "Same-day debits precede credits. Delayed transfers leave spendable cash at departure and enter the destination only on the entered availability date. Recurring transfers use the same entered calendar-day lag; no bank/holiday adjustment is inferred. In-transit funds are excluded from combined spendable cash. Unknown settlement prevents a complete destination forecast.".into();
    }
    if values.iter().any(Option::is_none) || terms.iter().any(|t| t.4.is_none()) {
        warnings.push("Unknown balances or amounts stay unknown; an unknown activity makes both affected account balances unknown for the rest of the period. Known accounts can still have shortfall warnings.".to_owned());
    }
    if data
        .schedules
        .iter()
        .any(|s| s.certainty == Certainty::Estimated)
    {
        warnings.push("Estimated activity is included in projections. Confirmed means a entered schedule, not a posted or bank-verified transaction.".to_owned());
    }
    for (a, v) in data.accounts.iter().zip(&values) {
        if v.is_some_and(|v| v < 0) {
            warnings.push(format!(
                "{} starts below zero; review immediate funding.",
                a.label
            ));
        }
    }
    let mut ledger = Vec::new();
    let mut days = Vec::new();
    let mut cursor = 0;
    day = start;
    loop {
        let mut indices = Vec::new();
        let mut day_warnings = Vec::new();
        while cursor < events.len() && events[cursor].0 == day {
            let index = events[cursor].1;
            let s = &data.schedules[index];
            let (source, dest, _, _, amount) = terms[index];
            let receipt = events[cursor].2;
            if receipt {
                let dest = dest.expect("validated transfer");
                values[dest] = values[dest].zip(amount).map(|(v, a)| v + a);
                transit = transit.zip(amount).map(|(v, a)| v - a);
            } else {
                let signed_amount = amount.map(|v| if s.kind.inflow() { v } else { -v });
                values[source] = values[source].zip(signed_amount).map(|(v, a)| v + a);
                if let Some(dest) = dest {
                    if s.settlement.is_none() {
                        values[dest] = values[dest].zip(amount).map(|(v, a)| v + a);
                    } else {
                        transit = transit.zip(amount).map(|(v, a)| v + a);
                        if s.settlement
                            .as_ref()
                            .is_some_and(|v| v.first_available_date.is_none())
                        {
                            values[dest] = None;
                            transit = None;
                        }
                    }
                }
            }
            let mut entry_warnings = Vec::new();
            if values[source].is_some_and(|v| v < 0) {
                entry_warnings.push("Potential insufficient funds / overdraft in this account, including conservative same-day ordering.".to_owned());
            }
            if dest.is_some_and(|d| values[d].is_some_and(|v| v < 0)) {
                entry_warnings.push("Destination account remains below zero.".to_owned());
            }
            let next = events
                .iter()
                .skip(cursor + 1)
                .find(|(_, i, receipt)| {
                    !receipt
                        && data.schedules[*i].kind == Kind::Paycheck
                        && data.schedules[*i].account_id == s.account_id
                })
                .map(|(d, _, _)| d.to_string());
            if matches!(s.kind, Kind::Bill | Kind::Loan) {
                entry_warnings.push(match &next {Some(d)=>format!("Payment occurs before the next entered paycheck ({d}); review available funds and ask the payee whether a date adjustment is allowed."),None=>"No later paycheck is entered for this account in the period; confirm funding for this payment.".to_owned()});
            }
            if amount.is_none() {
                entry_warnings.push(
                    "Activity amount is unknown; affected balances cannot be projected.".to_owned(),
                );
            }
            if entry_warnings
                .iter()
                .any(|v| v.contains("below zero") || v.contains("overdraft"))
            {
                day_warnings.push(format!(
                    "{}: potential shortfall during the day.",
                    data.accounts[source].label
                ));
            }
            indices.push(ledger.len());
            ledger.push(Entry {
                schedule_index: index,
                date: day.to_string(),
                label: s.label.clone(),
                kind: s.kind.clone(),
                certainty: s.certainty.clone(),
                account_id: s.account_id.clone(),
                destination_id: s.destination_id.clone(),
                amount: s.amount.clone(),
                source_balance: values[source].map(money),
                destination_balance: dest.and_then(|d| values[d].map(money)),
                combined_balance: combined(&values),
                next_paycheck: next,
                warnings: entry_warnings,
                transfer_leg: s
                    .settlement
                    .as_ref()
                    .map(|_| if receipt { "receipt" } else { "departure" }.into()),
                in_transit: if delayed { transit.map(money) } else { None },
            });
            cursor += 1;
        }
        for (a, v) in data.accounts.iter().zip(&values) {
            if a.reserve
                .as_ref()
                .is_some_and(|r| v.is_some_and(|v| v < parse_money(r).expect("validated reserve")))
            {
                day_warnings.push(format!(
                    "{}: available balance is below the entered reserve.",
                    a.label
                ));
            }
            if v.is_some_and(|v| v < 0) {
                day_warnings.push(format!("{}: closing balance below zero.", a.label));
            }
        }
        for a in &data.accounts {
            if indices
                .iter()
                .filter(|i| {
                    ledger[**i].account_id == a.id
                        && matches!(
                            ledger[**i].kind,
                            Kind::Bill | Kind::Loan | Kind::Savings | Kind::OtherOutflow
                        )
                })
                .count()
                >= 3
            {
                day_warnings.push(format!("{}: three or more payments on one day. Review permitted payment dates and preview alternatives before saving.",a.label));
            }
        }
        day_warnings.sort();
        day_warnings.dedup();
        days.push(Day {
            date: day.to_string(),
            balances: balances(&data, &values),
            combined_balance: combined(&values),
            entry_indices: indices,
            warnings: day_warnings,
            in_transit: if delayed { transit.map(money) } else { None },
        });
        if day == end {
            break;
        }
        day += Duration::days(1);
    }
    if ledger.is_empty() {
        warnings.push("No entered activity falls in this period; opening balances alone do not establish a complete cash-flow forecast.".to_owned());
    }
    let summary = Summary {
        ledger,
        days,
        closing_balances: balances(&data, &values),
        combined_closing: combined(&values),
        in_transit: if delayed { transit.map(money) } else { None },
        warnings,
    };
    Ok((data, summary))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn fixture() -> PlanData {
        PlanData {
            start_date: "2026-11-01".into(),
            end_date: "2026-11-30".into(),
            accounts: vec![
                Account {
                    reserve: None,
                    id: "checking".into(),
                    label: "Synthetic checking".into(),
                    opening_balance: Some("500".into()),
                },
                Account {
                    reserve: None,
                    id: "reserve".into(),
                    label: "Synthetic reserve".into(),
                    opening_balance: Some("1000".into()),
                },
            ],
            schedules: vec![
                schedule(
                    "Rent",
                    Kind::Bill,
                    "checking",
                    "1200",
                    "2026-11-03",
                    Frequency::Once,
                ),
                schedule(
                    "Wages",
                    Kind::Paycheck,
                    "checking",
                    "1500",
                    "2026-11-06",
                    Frequency::Biweekly,
                ),
                Schedule {
                    settlement: None,
                    destination_id: Some("reserve".into()),
                    ..schedule(
                        "Reserve transfer",
                        Kind::Transfer,
                        "checking",
                        "200",
                        "2026-11-07",
                        Frequency::Once,
                    )
                },
            ],
            notes: "Fictional dates".into(),
        }
    }
    fn schedule(
        name: &str,
        kind: Kind,
        account: &str,
        amount: &str,
        date: &str,
        frequency: Frequency,
    ) -> Schedule {
        Schedule {
            settlement: None,
            label: name.into(),
            kind,
            account_id: account.into(),
            destination_id: None,
            amount: Some(amount.into()),
            first_date: date.into(),
            frequency,
            second_day: None,
            until_date: None,
            certainty: Certainty::Estimated,
        }
    }
    #[test]
    fn independent_positive_month_but_shortfall_and_paired_transfer() {
        let (data, s) = calculate(fixture()).unwrap();
        assert_eq!(data.accounts[0].opening_balance.as_deref(), Some("500.00"));
        assert_eq!(s.ledger.len(), 4);
        assert_eq!(s.ledger[0].source_balance.as_deref(), Some("-700.00"));
        assert_eq!(s.ledger[0].next_paycheck.as_deref(), Some("2026-11-06"));
        assert!(s.days[2].warnings.iter().any(|w| w.contains("shortfall")));
        assert_eq!(s.ledger[2].source_balance.as_deref(), Some("600.00"));
        assert_eq!(s.ledger[2].destination_balance.as_deref(), Some("1200.00"));
        assert_eq!(s.ledger[2].combined_balance, s.ledger[1].combined_balance);
        assert_eq!(s.closing_balances[0].amount.as_deref(), Some("2100.00"));
        assert_eq!(s.combined_closing.as_deref(), Some("3300.00"));
    }
    #[test]
    fn same_day_debits_first_even_when_credit_entered_first() {
        let mut p = fixture();
        p.schedules = vec![
            schedule(
                "Pay",
                Kind::Paycheck,
                "checking",
                "2000",
                "2026-11-01",
                Frequency::Once,
            ),
            schedule(
                "Bill",
                Kind::Bill,
                "checking",
                "600",
                "2026-11-01",
                Frequency::Once,
            ),
        ];
        let (_, s) = calculate(p).unwrap();
        assert_eq!(s.ledger[0].label, "Bill");
        assert_eq!(s.ledger[0].source_balance.as_deref(), Some("-100.00"));
        assert_eq!(s.days[0].balances[0].amount.as_deref(), Some("1900.00"));
        assert!(!s.days[0].warnings.is_empty());
    }
    #[test]
    fn monthly_anchor_leap_semimonthly_clamps_without_double_events() {
        let mut p = fixture();
        p.start_date = "2028-01-31".into();
        p.end_date = "2028-03-31".into();
        p.schedules = vec![schedule(
            "Pay",
            Kind::Paycheck,
            "checking",
            "0.01",
            "2028-01-31",
            Frequency::Monthly,
        )];
        let (_, s) = calculate(p.clone()).unwrap();
        assert_eq!(
            s.ledger.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(),
            vec!["2028-01-31", "2028-02-29", "2028-03-31"]
        );
        p.schedules[0].frequency = Frequency::Semimonthly;
        p.schedules[0].second_day = Some(30);
        let (_, s) = calculate(p).unwrap();
        assert_eq!(
            s.ledger.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(),
            vec!["2028-01-31", "2028-02-29", "2028-03-30", "2028-03-31"]
        );
        assert_eq!(s.closing_balances[0].amount.as_deref(), Some("500.04"));
    }
    #[test]
    fn weekly_biweekly_prior_anchor_until_and_irregular_oneoffs() {
        let mut p = fixture();
        p.schedules = vec![
            schedule(
                "Weekly",
                Kind::Paycheck,
                "checking",
                "1",
                "2026-10-30",
                Frequency::Weekly,
            ),
            schedule(
                "Irregular",
                Kind::OtherInflow,
                "reserve",
                "0.01",
                "2026-11-09",
                Frequency::Once,
            ),
        ];
        p.schedules[0].until_date = Some("2026-11-13".into());
        let (_, s) = calculate(p).unwrap();
        assert_eq!(
            s.ledger.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(),
            vec!["2026-11-06", "2026-11-09", "2026-11-13"]
        );
        assert_eq!(s.combined_closing.as_deref(), Some("1502.01"));
    }
    #[test]
    fn unknowns_propagate_per_account_and_transfer_without_inventing_zero() {
        let mut p = fixture();
        p.schedules[0].amount = None;
        let (_, s) = calculate(p.clone()).unwrap();
        assert_eq!(s.ledger[1].source_balance, None);
        assert_eq!(s.combined_closing, None);
        assert_eq!(s.days[5].balances[1].amount.as_deref(), Some("1000.00"));
        assert_eq!(s.closing_balances[1].amount.as_deref(), Some("1200.00"));
        p.schedules[2].amount = None;
        let (_, s) = calculate(p).unwrap();
        assert_eq!(s.closing_balances[1].amount, None);
        let mut p = fixture();
        p.accounts[1].opening_balance = None;
        let (_, s) = calculate(p).unwrap();
        assert_eq!(s.closing_balances[0].amount.as_deref(), Some("2100.00"));
        assert_eq!(s.combined_closing, None);
    }
    #[test]
    fn concentration_and_unknown_next_paycheck_are_explicit() {
        let mut p = fixture();
        p.schedules = vec![
            schedule(
                "Bill",
                Kind::Bill,
                "checking",
                "100",
                "2026-11-02",
                Frequency::Once,
            ),
            schedule(
                "Loan",
                Kind::Loan,
                "checking",
                "100",
                "2026-11-02",
                Frequency::Once,
            ),
            schedule(
                "Savings",
                Kind::Savings,
                "checking",
                "100",
                "2026-11-02",
                Frequency::Once,
            ),
        ];
        let (_, s) = calculate(p).unwrap();
        assert!(s.days[1]
            .warnings
            .iter()
            .any(|w| w.contains("three or more")));
        assert!(s.ledger[0]
            .warnings
            .iter()
            .any(|w| w.contains("No later paycheck")));
    }
    #[test]
    fn invalid_dates_precision_accounts_ranges_and_event_limit_rejected() {
        for d in ["2027-02-29", "2028-2-29", "2101-01-01", "2028-+2-29"] {
            let mut p = fixture();
            p.schedules[0].first_date = d.into();
            assert!(calculate(p).is_err());
        }
        for a in ["1.001", "-1", "1e3", "1000000000.01"] {
            let mut p = fixture();
            p.schedules[0].amount = Some(a.into());
            assert!(calculate(p).is_err());
        }
        let mut p = fixture();
        p.accounts[1].id = "checking".into();
        assert!(calculate(p).is_err());
        let mut p = fixture();
        p.schedules[2].destination_id = Some("checking".into());
        assert!(calculate(p).is_err());
        let mut p = fixture();
        p.end_date = "2027-11-02".into();
        assert!(calculate(p).is_err());
        let mut p = fixture();
        p.start_date = "2026-01-01".into();
        p.end_date = "2026-12-31".into();
        p.schedules = vec![
            schedule(
                "Weekly",
                Kind::Paycheck,
                "checking",
                "1",
                "2026-01-01",
                Frequency::Weekly
            );
            100
        ];
        assert!(calculate(p).unwrap_err().contains("5,000"));
    }
    #[test]
    fn empty_activity_negative_opening_and_preview_do_not_change_original() {
        let mut p = fixture();
        p.accounts[0].opening_balance = Some("-0.01".into());
        p.schedules.clear();
        let (_, s) = calculate(p).unwrap();
        assert_eq!(s.combined_closing.as_deref(), Some("999.99"));
        assert!(s.warnings.iter().any(|w| w.contains("starts below")));
        assert!(s.warnings.iter().any(|w| w.contains("No entered activity")));
        let original = fixture();
        let mut adjusted = original.clone();
        adjusted.schedules[0].first_date = "2026-11-08".into();
        let (_, s) = calculate(adjusted).unwrap();
        assert!(s
            .days
            .iter()
            .all(|d| !d.warnings.iter().any(|w| w.contains("shortfall"))));
        assert_eq!(original.schedules[0].first_date, "2026-11-03");
    }
}

#[cfg(test)]
mod settlement_tests {
    use super::*;
    #[test]
    fn delayed_transfer_preserves_funds_but_not_premature_liquidity_and_reserves() {
        let mut d = tests::fixture();
        d.schedules.retain(|s| s.kind == Kind::Transfer);
        d.accounts[0].reserve = Some("400".into());
        d.schedules[0].settlement = Some(Settlement {
            first_available_date: Some("2026-11-10".into()),
        });
        let (_, s) = calculate(d.clone()).unwrap();
        assert_eq!(s.ledger.len(), 2);
        assert_eq!(s.ledger[0].source_balance.as_deref(), Some("300.00"));
        assert_eq!(s.ledger[0].destination_balance.as_deref(), Some("1000.00"));
        assert_eq!(s.ledger[0].combined_balance.as_deref(), Some("1300.00"));
        assert_eq!(s.ledger[0].in_transit.as_deref(), Some("200.00"));
        assert_eq!(
            s.days[6].warnings,
            vec!["Synthetic checking: available balance is below the entered reserve."]
        );
        assert_eq!(s.ledger[1].date, "2026-11-10");
        assert_eq!(s.ledger[1].transfer_leg.as_deref(), Some("receipt"));
        assert_eq!(s.ledger[1].source_balance.as_deref(), Some("300.00"));
        assert_eq!(s.combined_closing.as_deref(), Some("1500.00"));
        assert_eq!(s.in_transit.as_deref(), Some("0.00"));
        d.end_date = "2026-11-08".into();
        assert_eq!(
            calculate(d.clone()).unwrap().1.in_transit.as_deref(),
            Some("200.00")
        );
        d.start_date = "2026-11-08".into();
        d.end_date = "2026-11-12".into();
        d.accounts[0].opening_balance = Some("300".into());
        let s = calculate(d.clone()).unwrap().1;
        assert_eq!(s.ledger.len(), 1);
        assert_eq!(s.combined_closing.as_deref(), Some("1500.00"));
        d.schedules[0]
            .settlement
            .as_mut()
            .unwrap()
            .first_available_date = None;
        let s = calculate(d.clone()).unwrap().1;
        assert!(s.combined_closing.is_none());
        d.schedules[0]
            .settlement
            .as_mut()
            .unwrap()
            .first_available_date = Some("2026-11-06".into());
        assert!(calculate(d).is_err());
    }
    #[test]
    fn recurring_receipts_remain_distinct_and_same_day_debits_precede_them() {
        let mut d = tests::fixture();
        d.schedules.retain(|s| s.kind == Kind::Transfer);
        d.schedules[0].frequency = Frequency::Weekly;
        d.schedules[0].settlement = Some(Settlement {
            first_available_date: Some("2026-11-09".into()),
        });
        let s = calculate(d.clone()).unwrap().1;
        assert_eq!(
            s.ledger
                .iter()
                .filter(|e| e.transfer_leg.as_deref() == Some("departure"))
                .count(),
            4
        );
        assert_eq!(
            s.ledger
                .iter()
                .filter(|e| e.transfer_leg.as_deref() == Some("receipt"))
                .count(),
            4
        );
        assert_eq!(s.combined_closing.as_deref(), Some("1500.00"));
        d.schedules[0].amount = None;
        assert!(calculate(d).unwrap().1.combined_closing.is_none());
    }
}
