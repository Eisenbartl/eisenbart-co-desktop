//! Monthly planning estimates. Never infer missing contractual terms.
use crate::budget::{money, parse_money};
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MONTHS: i32 = 600;
const BALANCE_LIMIT: i64 = 1_000_000_000_000_000;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum DebtType {
    CreditCard,
    AutoLoan,
    StudentLoan,
    PersonalLoan,
    Mortgage,
    Other,
    Custom,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum InterestModel {
    Unknown,
    FixedMonthly,
    Daily,
    Variable,
    Promotional,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub label: String,
    pub debt_type: DebtType,
    pub balance: Option<String>,
    pub apr: Option<String>,
    pub minimum_payment: Option<String>,
    pub monthly_fee: Option<String>,
    pub interest_model: InterestModel,
    pub verified: bool,
    pub terms_note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatedAmount {
    pub month: String,
    pub amount: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RateChange {
    pub month: String,
    pub apr: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdvancedAccount {
    pub account_index: usize,
    /// Required for daily accrual; 360 or 365 calendar-day basis.
    pub day_basis: Option<u32>,
    pub rate_changes: Vec<RateChange>,
    pub fees: Vec<DatedAmount>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaymentSchedule {
    pub first_date: String,
    pub frequency: crate::cashflow::Frequency,
    pub second_day: Option<u32>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Advanced {
    pub accounts: Vec<AdvancedAccount>,
    pub payments: Option<PaymentSchedule>,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payment {
    pub date: String,
    pub payments: Vec<String>,
    pub interest: Vec<String>,
    pub fees: Vec<String>,
    pub balances: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetExtra {
    pub account_index: usize,
    pub month: String,
    pub amount: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanData {
    pub start_month: String,
    pub monthly_payment: Option<String>,
    pub accounts: Vec<Account>,
    pub extras: Vec<DatedAmount>,
    pub payment_changes: Vec<DatedAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced: Option<Advanced>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targeted_extras: Vec<TargetExtra>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanInput {
    pub expected_revision: i64,
    pub data: PlanData,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Status {
    PaidOff,
    InsufficientBudget,
    HorizonReached,
    ModelLimit,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Month {
    pub month: String,
    pub available_payment: String,
    pub payments: Vec<String>,
    pub interest: Vec<String>,
    pub fees: Vec<String>,
    pub balances: Vec<String>,
    pub total_payment: String,
    pub remaining_balance: String,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Projection {
    pub status: Status,
    pub payoff_month: Option<String>,
    pub account_payoff_months: Vec<Option<String>>,
    pub total_interest: String,
    pub total_fees: String,
    pub total_paid: String,
    pub remaining_balance: String,
    pub negative_amortization_accounts: Vec<usize>,
    pub warnings: Vec<String>,
    pub schedule: Vec<Month>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dated_payments: Vec<Payment>,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Comparison {
    pub warnings: Vec<String>,
    pub snowball: Option<Projection>,
    pub avalanche: Option<Projection>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetContext {
    pub revision: i64,
    pub monthly_capacity: String,
    pub planned_debt_payments: String,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientDebtPlan {
    pub client_id: String,
    pub revision: i64,
    pub updated_at: Option<String>,
    pub data: Option<PlanData>,
    pub comparison: Option<Comparison>,
    pub budget_context: BudgetContext,
}

pub fn month_number(value: &str) -> Result<i32, String> {
    if value.len() != 7 || !value.is_ascii() || value.as_bytes()[4] != b'-' {
        return Err("Enter a planning month as YYYY-MM.".to_owned());
    }
    if !value[..4].bytes().all(|b| b.is_ascii_digit())
        || !value[5..].bytes().all(|b| b.is_ascii_digit())
    {
        return Err("Enter a planning month as YYYY-MM.".to_owned());
    }
    let year: i32 = value[..4]
        .parse()
        .map_err(|_| "Enter a planning month as YYYY-MM.")?;
    let month: i32 = value[5..]
        .parse()
        .map_err(|_| "Enter a planning month as YYYY-MM.")?;
    if !(1900..=2100).contains(&year) || !(1..=12).contains(&month) {
        return Err("Planning months must be valid months from 1900 through 2100.".to_owned());
    }
    Ok(year * 12 + month - 1)
}
fn month_label(number: i32) -> String {
    format!("{:04}-{:02}", number / 12, number % 12 + 1)
}
fn apr_units(value: &str) -> Result<i64, String> {
    let invalid = || {
        "APR must be a nonnegative percentage up to 1000, with at most four decimal places."
            .to_owned()
    };
    let parts: Vec<&str> = value.split('.').collect();
    if value.len() > 12
        || parts.len() > 2
        || parts[0].is_empty()
        || !parts[0].bytes().all(|b| b.is_ascii_digit())
        || parts
            .get(1)
            .is_some_and(|s| s.is_empty() || s.len() > 4 || !s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(invalid());
    }
    let whole: i64 = parts[0].parse().map_err(|_| invalid())?;
    let fraction = parts
        .get(1)
        .map(|s| format!("{s:0<4}"))
        .unwrap_or_else(|| "0000".to_owned());
    let units = whole
        .checked_mul(10000)
        .and_then(|n| n.checked_add(fraction.parse().ok()?))
        .ok_or_else(invalid)?;
    if units > 10_000_000 {
        return Err(invalid());
    }
    Ok(units)
}
fn optional_money(value: &mut Option<String>) -> Result<Option<i64>, String> {
    match value {
        Some(text) => {
            let cents = parse_money(text.trim())?;
            *text = money(cents);
            Ok(Some(cents))
        }
        None => Ok(None),
    }
}
fn dated_amounts(values: &mut [DatedAmount], start: i32) -> Result<BTreeMap<i32, i64>, String> {
    if values.len() > 24 {
        return Err("Use at most 24 extra payments and 24 payment changes.".to_owned());
    }
    let mut parsed = BTreeMap::new();
    for value in values.iter_mut() {
        let month = month_number(&value.month)?;
        if !(start..start + MONTHS).contains(&month) {
            return Err(
                "Payment events must fall within the 600-month planning horizon.".to_owned(),
            );
        }
        let cents = parse_money(value.amount.trim())?;
        value.amount = money(cents);
        if parsed.insert(month, cents).is_some() {
            return Err("Use only one event of each type per month; combine extra payments for the same month.".to_owned());
        }
    }
    values.sort_by(|a, b| a.month.cmp(&b.month));
    Ok(parsed)
}

struct Terms {
    balance: i64,
    apr: i64,
    minimum: i64,
    fee: i64,
    day_basis: Option<u32>,
    rates: BTreeMap<i32, i64>,
    fees: BTreeMap<i32, i64>,
}

pub fn calculate(mut data: PlanData) -> Result<(PlanData, Comparison), String> {
    let start = month_number(&data.start_month)?;
    if data.accounts.len() > 50 {
        return Err("Use at most 50 debt accounts.".to_owned());
    }
    let payment = optional_money(&mut data.monthly_payment)?;
    let extras = dated_amounts(&mut data.extras, start)?;
    let changes = dated_amounts(&mut data.payment_changes, start)?;
    if data.targeted_extras.len() > 24 {
        return Err("Use at most 24 account-targeted extra payments.".into());
    }
    let mut targets: BTreeMap<i32, Vec<(usize, i64)>> = BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    for t in &mut data.targeted_extras {
        let month = month_number(&t.month)?;
        if t.account_index >= data.accounts.len()
            || !(start..start + MONTHS).contains(&month)
            || !seen.insert((month, t.account_index))
        {
            return Err("Targeted payments need a unique existing account/month within the planning horizon.".into());
        }
        let amount = parse_money(t.amount.trim())?;
        t.amount = money(amount);
        targets
            .entry(month)
            .or_default()
            .push((t.account_index, amount));
    }
    data.targeted_extras.sort_by(|a, b| {
        a.month
            .cmp(&b.month)
            .then(a.account_index.cmp(&b.account_index))
    });

    let mut warnings = vec!["Planning estimate: fixed APR ÷ 12, interest on the opening balance rounded to cents before monthly fees and end-of-month payments. No new borrowing.".to_owned()];
    let mut advanced_accounts = BTreeMap::new();
    if let Some(advanced) = &mut data.advanced {
        if advanced.accounts.len() > 50 {
            return Err("Use at most 50 advanced debt accounts.".into());
        }
        for a in &mut advanced.accounts {
            if a.account_index >= data.accounts.len()
                || advanced_accounts.contains_key(&a.account_index)
            {
                return Err(
                    "Advanced terms must identify each existing account at most once.".into(),
                );
            }
            if a.day_basis.is_some_and(|v| v != 360 && v != 365) {
                return Err("Daily interest needs an explicit 360- or 365-day basis.".into());
            }
            if a.rate_changes.len() > 24 {
                return Err("Use at most 24 known rate changes per account.".into());
            }
            let mut rates = BTreeMap::new();
            for r in &mut a.rate_changes {
                let month = month_number(&r.month)?;
                if !(start..start + MONTHS).contains(&month) || rates.contains_key(&month) {
                    return Err(
                        "Rate changes need unique months within the planning horizon.".into(),
                    );
                }
                let apr = apr_units(r.apr.trim())?;
                r.apr = format!("{}.{:04}", apr / 10000, apr % 10000);
                rates.insert(month, apr);
            }
            a.rate_changes.sort_by(|a, b| a.month.cmp(&b.month));
            advanced_accounts.insert(
                a.account_index,
                (a.day_basis, rates, dated_amounts(&mut a.fees, start)?),
            );
        }
        advanced.accounts.sort_by_key(|a| a.account_index);
        if let Some(payments) = &advanced.payments {
            let first = crate::cashflow::date(&payments.first_date)?;
            if month_number(&payments.first_date[..7])? != start
                || payments.frequency == crate::cashflow::Frequency::Once
            {
                return Err(
                    "Recurring debt payments must begin in the first planning month.".into(),
                );
            }
            if payments.frequency == crate::cashflow::Frequency::Semimonthly {
                if !payments
                    .second_day
                    .is_some_and(|d| (1..=31).contains(&d) && d != first.day())
                {
                    return Err(
                        "Twice-monthly debt payments require a distinct second day from 1–31."
                            .into(),
                    );
                }
            } else if payments.second_day.is_some() {
                return Err("Only twice-monthly payments use a second day.".into());
            }
        }
    }
    let periodic = data.advanced.as_ref().is_some_and(|a| a.payments.is_some());
    let mut terms = Vec::new();
    let mut incomplete = payment.is_none();
    if payment.is_none() {
        warnings
            .push("Enter a known monthly payment budget before comparing strategies.".to_owned());
    }
    if data.accounts.is_empty() {
        incomplete = true;
        warnings.push("No debt accounts entered.".to_owned());
    }
    for (index, account) in data.accounts.iter_mut().enumerate() {
        account.label = account.label.trim().to_owned();
        account.terms_note = account.terms_note.trim().to_owned();
        if account.label.is_empty()
            || account.label.chars().count() > 120
            || account.label.chars().any(char::is_control)
        {
            return Err(
                "Each debt needs a name of 1–120 characters without control characters.".to_owned(),
            );
        }
        if account.terms_note.chars().count() > 500
            || account
                .terms_note
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t')
        {
            return Err("Debt terms notes must be at most 500 characters without unsupported control characters.".to_owned());
        }
        let balance = optional_money(&mut account.balance)?;
        let minimum = optional_money(&mut account.minimum_payment)?;
        let fee = optional_money(&mut account.monthly_fee)?;
        let apr = account
            .apr
            .as_deref()
            .map(|text| apr_units(text.trim()))
            .transpose()?;
        if let Some(apr) = apr {
            account.apr = Some(format!("{}.{:04}", apr / 10000, apr % 10000));
        }
        let (day_basis, rates, fees) =
            advanced_accounts
                .remove(&index)
                .unwrap_or((None, BTreeMap::new(), BTreeMap::new()));
        let supported = match account.interest_model {
            InterestModel::FixedMonthly => !periodic && day_basis.is_none() && rates.is_empty(),
            InterestModel::Daily => day_basis.is_some() && rates.is_empty(),
            InterestModel::Variable => !rates.is_empty(),
            InterestModel::Promotional => !rates.is_empty(),
            InterestModel::Unknown => false,
        } && (!periodic || day_basis.is_some());
        let active = balance != Some(0);
        if active
            && (balance.is_none()
                || minimum.is_none()
                || fee.is_none()
                || apr.is_none()
                || !supported)
        {
            incomplete = true;
            warnings.push(if data.advanced.is_none() {format!("Account {} has missing or unmodeled terms. No payoff comparison is produced until all active accounts have a balance, APR, minimum, monthly fee, and fixed monthly model.", index + 1)}else{format!("Account {} has missing or unsupported interest/payment terms. Daily or frequent payments require an entered day basis; variable/promotional terms need explicit known rate changes.",index+1)});
        }
        if active && !account.verified {
            warnings.push(format!("Account {} terms are unverified; check the lender's terms before relying on this estimate.", index + 1));
        }
        terms.push(Terms {
            balance: balance.unwrap_or(0),
            apr: apr.unwrap_or(0),
            minimum: minimum.unwrap_or(0),
            fee: fee.unwrap_or(0),
            day_basis,
            rates,
            fees,
        });
    }
    if !extras.is_empty() {
        warnings.push("Extra payments require separately verified funding; check cash availability in each event month.".to_owned());
    }
    if !data.targeted_extras.is_empty() {
        warnings.push("Account-targeted extras are additional commitments after required minimums. Unused targeted money is not reassigned, and dated cash-flow is unchanged. Verify funding and explicitly reconcile any agreed live plan.".into());
    }
    if data.advanced.is_some() {
        warnings[0]="Advanced planning estimate with explicit entered rate, fee and payment assumptions; no new borrowing. Review the detailed accrual/rounding rules below.".into();
        warnings.push("Advanced planning estimate: entered future APR changes take effect at the start of their month; the last known rate continues. Promotional reversion must be entered explicitly; no retroactive deferred interest is modeled. Monthly fees and dated month-level charges apply at the first payment/accrual event while active. Daily simple interest accrues on the interval opening balance using actual calendar days / entered 360 or 365 basis, rounds half up per interval, and capitalizes at payments and month end. Payment budget and minimums are split equally across entered occurrences, with earlier remainder cents; extras apply at the last payment. No holidays, new borrowing or automatic cash-flow writes.".into());
    }
    let (snowball, avalanche) = if incomplete {
        (None, None)
    } else {
        let payment = payment.unwrap_or(0);
        (
            Some(project(
                &terms,
                start,
                payment,
                PaymentEvents {
                    extras: &extras,
                    changes: &changes,
                    targets: &targets,
                },
                false,
                data.advanced.as_ref(),
            )),
            Some(project(
                &terms,
                start,
                payment,
                PaymentEvents {
                    extras: &extras,
                    changes: &changes,
                    targets: &targets,
                },
                true,
                data.advanced.as_ref(),
            )),
        )
    };
    Ok((
        data,
        Comparison {
            warnings,
            snowball,
            avalanche,
        },
    ))
}

struct PaymentEvents<'a> {
    extras: &'a BTreeMap<i32, i64>,
    changes: &'a BTreeMap<i32, i64>,
    targets: &'a BTreeMap<i32, Vec<(usize, i64)>>,
}
fn project(
    terms: &[Terms],
    start: i32,
    mut payment: i64,
    events: PaymentEvents<'_>,
    avalanche: bool,
    advanced: Option<&Advanced>,
) -> Projection {
    let PaymentEvents {
        extras,
        changes,
        targets,
    } = events;
    let mut balances: Vec<i64> = terms.iter().map(|t| t.balance).collect();
    let mut payoff_months = vec![None; terms.len()];
    let mut schedule = Vec::new();
    let mut paid = 0;
    let mut interest_total = 0;
    let mut fees_total = 0;
    let mut negative = vec![false; terms.len()];
    let mut warnings = Vec::new();
    let mut status = Status::HorizonReached;
    let mut dated_payments = Vec::new();
    'months: for number in start..start + MONTHS {
        if balances.iter().all(|b| *b == 0) {
            status = Status::PaidOff;
            break;
        }
        if let Some(changed) = changes.get(&number) {
            payment = *changed;
        }
        let month = month_label(number);
        let first = NaiveDate::from_ymd_opt(number / 12, (number % 12 + 1) as u32, 1)
            .expect("bounded month");
        let next = if first.month() == 12 {
            NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
        }
        .expect("next month");
        let last = next - Duration::days(1);
        let payment_dates = if let Some(cadence) = advanced.and_then(|a| a.payments.as_ref()) {
            let anchor =
                NaiveDate::parse_from_str(&cadence.first_date, "%Y-%m-%d").expect("validated date");
            let schedule = crate::cashflow::Schedule {
                label: String::new(),
                account_id: String::new(),
                destination_id: None,
                kind: crate::cashflow::Kind::Loan,
                amount: None,
                first_date: cadence.first_date.clone(),
                frequency: cadence.frequency.clone(),
                second_day: cadence.second_day,
                until_date: None,
                certainty: crate::cashflow::Certainty::Estimated,
                settlement: None,
            };
            (0..(next - first).num_days())
                .map(|d| first + Duration::days(d))
                .filter(|d| crate::cashflow::occurs(&schedule, anchor, *d))
                .collect::<Vec<_>>()
        } else {
            vec![last]
        };
        let count = payment_dates.len();
        let mut dates = payment_dates.clone();
        if dates.last() != Some(&last) {
            dates.push(last);
        }
        let rates: Vec<_> = terms
            .iter()
            .map(|t| {
                t.rates
                    .range(..=number)
                    .next_back()
                    .map_or(t.apr, |(_, r)| *r)
            })
            .collect();
        let opening_month = balances.clone();
        let mut monthly_payments = vec![0; terms.len()];
        let mut monthly_interest = vec![0; terms.len()];
        let mut monthly_fees = vec![0; terms.len()];
        let mut previous = first - Duration::days(1);
        for (period, day) in dates.iter().enumerate() {
            let payment_index = payment_dates.iter().position(|d| d == day);
            let split = |value: i64, index: usize| {
                value / count.max(1) as i64
                    + i64::from((index as i64) < value % count.max(1) as i64)
            };
            let target_list = if payment_index.is_some_and(|i| i + 1 == count) {
                targets.get(&number).map(Vec::as_slice).unwrap_or(&[])
            } else {
                &[]
            };
            let targeted: i64 = target_list.iter().map(|(_, v)| v).sum();
            let available = targeted
                + payment_index.map_or(0, |i| {
                    split(payment, i)
                        + if i + 1 == count {
                            extras.get(&number).copied().unwrap_or(0)
                        } else {
                            0
                        }
                });
            let interest: Vec<i64> = balances
                .iter()
                .zip(terms)
                .zip(&rates)
                .map(|((b, t), r)| {
                    let (numerator, denominator) = if let Some(basis) = t.day_basis {
                        (
                            i128::from(*b)
                                * i128::from(*r)
                                * i128::from((*day - previous).num_days()),
                            i128::from(basis) * 1_000_000,
                        )
                    } else {
                        (i128::from(*b) * i128::from(*r), 12_000_000)
                    };
                    ((numerator + denominator / 2) / denominator) as i64
                })
                .collect();
            let fees: Vec<_> = balances
                .iter()
                .zip(terms)
                .map(|(b, t)| {
                    if *b > 0 && period == 0 {
                        t.fee + t.fees.get(&number).copied().unwrap_or(0)
                    } else {
                        0
                    }
                })
                .collect();
            let owed: Vec<_> = balances
                .iter()
                .zip(&interest)
                .zip(&fees)
                .map(|((b, i), f)| b + i + f)
                .collect();
            let mut payments: Vec<i64> = owed
                .iter()
                .zip(terms)
                .map(|(o, t)| (*o).min(payment_index.map_or(0, |i| split(t.minimum, i))))
                .collect();
            let required: i64 = payments.iter().sum();
            if available - targeted < required {
                status = Status::InsufficientBudget;
                warnings.push(format!("{month}: available payment {} is below required minimum payments {}. Projection stops before an unaffordable payment; no debt-free date is assigned.",money(available-targeted),money(required)));
                // Monthly results are atomic; do not expose partial payments as a complete month.
                if period > 0 {
                    warnings.push("The final simulated month contains only the earlier affordable payment events; review dated payments for the partial month.".into());
                }
                break 'months;
            }
            let mut extra = available - required;
            for (i, amount) in target_list {
                let applied = (*amount).min(owed[*i] - payments[*i]);
                payments[*i] += applied;
                // Unused account-specific money is not silently sent to a different lender.
                extra -= amount;
            }
            let mut order: Vec<_> = (0..terms.len()).filter(|i| balances[*i] > 0).collect();
            order.sort_by(|a, b| {
                if avalanche {
                    rates[*b]
                        .cmp(&rates[*a])
                        .then(balances[*a].cmp(&balances[*b]))
                        .then(a.cmp(b))
                } else {
                    balances[*a]
                        .cmp(&balances[*b])
                        .then(rates[*b].cmp(&rates[*a]))
                        .then(a.cmp(b))
                }
            });
            for i in order {
                let added = extra.min(owed[i] - payments[i]);
                payments[i] += added;
                extra -= added;
            }
            let closing: Vec<_> = owed.iter().zip(&payments).map(|(o, p)| o - p).collect();
            if closing.iter().any(|b| *b > BALANCE_LIMIT) {
                status = Status::ModelLimit;
                warnings.push("Growing balances exceeded the model's safety limit. No debt-free date is assigned.".into());
                break 'months;
            }
            for i in 0..terms.len() {
                if balances[i] > 0 && closing[i] == 0 {
                    payoff_months[i] = Some(month.clone());
                }
                monthly_payments[i] += payments[i];
                monthly_interest[i] += interest[i];
                monthly_fees[i] += fees[i];
            }
            paid += payments.iter().sum::<i64>();
            interest_total += interest.iter().sum::<i64>();
            fees_total += fees.iter().sum::<i64>();
            balances = closing;
            if advanced.is_some() {
                dated_payments.push(Payment {
                    date: day.to_string(),
                    payments: payments.iter().copied().map(money).collect(),
                    interest: interest.iter().copied().map(money).collect(),
                    fees: fees.iter().copied().map(money).collect(),
                    balances: balances.iter().copied().map(money).collect(),
                });
            }
            previous = *day;
        }
        for (i, balance) in balances.iter().enumerate() {
            if *balance > opening_month[i] {
                negative[i] = true;
            }
        }
        schedule.push(Month {
            month,
            available_payment: money(
                payment
                    + extras.get(&number).copied().unwrap_or(0)
                    + targets
                        .get(&number)
                        .map_or(0, |v| v.iter().map(|(_, a)| a).sum::<i64>()),
            ),
            payments: monthly_payments.iter().copied().map(money).collect(),
            interest: monthly_interest.iter().copied().map(money).collect(),
            fees: monthly_fees.iter().copied().map(money).collect(),
            balances: balances.iter().copied().map(money).collect(),
            total_payment: money(monthly_payments.iter().sum()),
            remaining_balance: money(balances.iter().sum()),
        });
    }
    if balances.iter().all(|b| *b == 0) {
        status = Status::PaidOff;
    }
    if status == Status::HorizonReached {
        warnings.push(
            "Balances remain after the 600-month horizon; no debt-free date is assigned."
                .to_owned(),
        );
    }
    let negative_accounts: Vec<usize> = negative
        .iter()
        .enumerate()
        .filter_map(|(i, grew)| if *grew { Some(i) } else { None })
        .collect();
    if !negative_accounts.is_empty() {
        warnings.push("Some monthly payments did not cover interest and fees; those balances grew. Review the marked accounts.".to_owned());
    }
    let payoff_month = if status == Status::PaidOff {
        schedule.last().map(|m| m.month.clone())
    } else {
        None
    };
    Projection {
        status,
        payoff_month,
        account_payoff_months: payoff_months,
        total_interest: money(interest_total),
        total_fees: money(fees_total),
        total_paid: money(paid),
        remaining_balance: money(balances.iter().sum()),
        negative_amortization_accounts: negative_accounts,
        warnings,
        schedule,
        dated_payments,
    }
}

pub(crate) fn all_extras(data: &PlanData) -> Vec<DatedAmount> {
    if data.targeted_extras.is_empty() {
        return data.extras.clone();
    }
    let mut totals: BTreeMap<String, i64> = BTreeMap::new();
    for extra in &data.extras {
        *totals.entry(extra.month.clone()).or_default() +=
            parse_money(&extra.amount).expect("validated extra");
    }
    for extra in &data.targeted_extras {
        *totals.entry(extra.month.clone()).or_default() +=
            parse_money(&extra.amount).expect("validated targeted extra");
    }
    totals
        .into_iter()
        .map(|(month, amount)| DatedAmount {
            month,
            amount: money(amount),
        })
        .collect()
}
pub fn apply_budget_context(
    data: &PlanData,
    comparison: &mut Comparison,
    context: &BudgetContext,
) -> Result<(), String> {
    if context.revision == 0 {
        comparison
            .warnings
            .push("No saved client budget is available to check payment capacity.".to_owned());
        return Ok(());
    }
    if !context.warnings.is_empty() {
        comparison.warnings.push("The saved budget has missing, estimated, or deficit warnings; payment capacity needs review.".to_owned());
    }
    // Context comes from the server's exact signed budget formatter; unlike an
    // entered single line, its aggregate may exceed the per-input $1bn limit.
    let capacity: i64 = context
        .monthly_capacity
        .replace('.', "")
        .parse()
        .map_err(|_| "Invalid budget context.")?;
    let planned: i64 = context
        .planned_debt_payments
        .replace('.', "")
        .parse()
        .map_err(|_| "Invalid budget context.")?;
    if data
        .monthly_payment
        .as_deref()
        .map(parse_money)
        .transpose()?
        .is_some_and(|payment| payment != planned)
    {
        comparison.warnings.push("The initial monthly payment differs from the saved budget's debt allocation. Reconcile these separate plans before relying on them.".to_owned());
    }
    let mut exceeded = data
        .monthly_payment
        .as_deref()
        .map(parse_money)
        .transpose()?
        .is_some_and(|payment| payment > capacity);
    for change in &data.payment_changes {
        exceeded |= parse_money(&change.amount)? > capacity;
    }
    for extra in &data.extras {
        let base = data
            .payment_changes
            .iter()
            .rfind(|c| c.month <= extra.month)
            .map(|c| c.amount.as_str())
            .or(data.monthly_payment.as_deref());
        if let Some(base) = base {
            exceeded |= parse_money(base)? + parse_money(&extra.amount)? > capacity;
        }
    }
    if !data.targeted_extras.is_empty() {
        for extra in all_extras(data) {
            let base = data
                .payment_changes
                .iter()
                .rfind(|c| c.month <= extra.month)
                .map(|c| c.amount.as_str())
                .or(data.monthly_payment.as_deref());
            if let Some(base) = base {
                exceeded |=
                    parse_money(base)? + crate::budget::parse_total(&extra.amount)? > capacity;
            }
        }
    }
    if exceeded {
        comparison.warnings.push("Proposed payments exceed the current saved budget's monthly capacity after savings in one or more months. Confirm additional funding or adjust the plan.".to_owned());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn fixture() -> PlanData {
        PlanData {
            start_month: "2026-11".to_owned(),
            monthly_payment: Some("100".to_owned()),
            accounts: vec![
                account("Small", "100", "0", "10"),
                account("Large", "500", "24", "20"),
            ],
            extras: vec![],
            payment_changes: vec![],
            advanced: None,
            targeted_extras: vec![],
        }
    }
    fn account(label: &str, balance: &str, apr: &str, minimum: &str) -> Account {
        Account {
            label: label.to_owned(),
            debt_type: DebtType::CreditCard,
            balance: Some(balance.to_owned()),
            apr: Some(apr.to_owned()),
            minimum_payment: Some(minimum.to_owned()),
            monthly_fee: Some("0".to_owned()),
            interest_model: InterestModel::FixedMonthly,
            verified: true,
            terms_note: String::new(),
        }
    }
    #[test]
    fn independent_monthly_amortization_and_small_final_payment() {
        let mut data = fixture();
        data.accounts = vec![account("Loan", "1000", "12", "100")];
        let (_, c) = calculate(data).unwrap();
        for p in [c.snowball.unwrap(), c.avalanche.unwrap()] {
            assert_eq!(p.status, Status::PaidOff);
            assert_eq!(p.schedule.len(), 11);
            assert_eq!(p.schedule[0].interest, vec!["10.00"]);
            assert_eq!(p.schedule[0].balances, vec!["910.00"]);
            assert_eq!(p.schedule[1].balances, vec!["819.10"]);
            assert_eq!(p.schedule[10].payments, vec!["58.98"]);
            assert_eq!(p.total_interest, "58.98");
            assert_eq!(p.total_paid, "1058.98");
            assert_eq!(p.payoff_month.as_deref(), Some("2027-09"));
        }
    }
    #[test]
    fn independent_two_debt_comparison_uses_identical_payment_budgets() {
        let (_, c) = calculate(fixture()).unwrap();
        let snow = c.snowball.unwrap();
        let ava = c.avalanche.unwrap();
        assert_eq!(snow.schedule[0].payments, vec!["80.00", "20.00"]);
        assert_eq!(ava.schedule[0].payments, vec!["10.00", "90.00"]);
        assert_eq!(snow.total_interest, "43.10");
        assert_eq!(ava.total_interest, "35.35");
        assert_eq!(snow.total_paid, "643.10");
        assert_eq!(ava.total_paid, "635.35");
        assert_eq!(snow.payoff_month, Some("2027-05".to_owned()));
        assert_eq!(snow.payoff_month, ava.payoff_month);
        assert_eq!(
            snow.schedule
                .iter()
                .map(|m| &m.available_payment)
                .collect::<Vec<_>>(),
            ava.schedule
                .iter()
                .map(|m| &m.available_payment)
                .collect::<Vec<_>>()
        );
        // The last month caps each payment at the remaining debt, not at $100.
        assert_eq!(snow.schedule.last().unwrap().total_payment, "43.10");
        assert_eq!(ava.schedule.last().unwrap().total_payment, "35.35");
    }
    #[test]
    fn extra_rollover_fees_and_future_payment_changes_are_conserved() {
        let mut data = fixture();
        data.accounts = vec![
            account("First", "10", "0", "1"),
            account("Second", "15", "0", "1"),
        ];
        data.monthly_payment = Some("20".to_owned());
        data.accounts[0].monthly_fee = Some("1".to_owned());
        data.extras = vec![DatedAmount {
            month: "2026-11".to_owned(),
            amount: "5".to_owned(),
        }];
        data.payment_changes = vec![DatedAmount {
            month: "2026-12".to_owned(),
            amount: "2".to_owned(),
        }];
        let (_, c) = calculate(data).unwrap();
        let p = c.snowball.unwrap();
        assert_eq!(p.schedule[0].payments, vec!["11.00", "14.00"]);
        assert_eq!(p.schedule[1].payments, vec!["0.00", "1.00"]);
        assert_eq!(p.total_fees, "1.00");
        assert_eq!(p.total_interest, "0.00");
        assert_eq!(p.total_paid, "26.00");
        assert_eq!(p.schedule[1].available_payment, "2.00");
        assert_eq!(
            p.account_payoff_months,
            vec![Some("2026-11".to_owned()), Some("2026-12".to_owned())]
        );
    }
    #[test]
    fn minimum_shortfall_stops_before_unaffordable_payment() {
        let mut data = fixture();
        data.monthly_payment = Some("29.99".to_owned());
        let (_, c) = calculate(data).unwrap();
        for p in [c.snowball.unwrap(), c.avalanche.unwrap()] {
            assert_eq!(p.status, Status::InsufficientBudget);
            assert!(p.schedule.is_empty());
            assert_eq!(p.payoff_month, None);
            assert_eq!(p.total_paid, "0.00");
            assert_eq!(p.remaining_balance, "600.00");
        }
        let mut data = fixture();
        data.payment_changes.push(DatedAmount {
            month: "2026-12".to_owned(),
            amount: "0".to_owned(),
        });
        let (_, c) = calculate(data).unwrap();
        let p = c.snowball.unwrap();
        assert_eq!(p.status, Status::InsufficientBudget);
        assert_eq!(p.schedule.len(), 1);
        assert!(p.payoff_month.is_none());
    }
    #[test]
    fn missing_or_unmodeled_active_terms_prevent_fabricated_projections() {
        for model in [
            InterestModel::Unknown,
            InterestModel::Daily,
            InterestModel::Variable,
            InterestModel::Promotional,
        ] {
            let mut data = fixture();
            data.accounts[0].interest_model = model;
            let (_, c) = calculate(data).unwrap();
            assert!(c.snowball.is_none() && c.avalanche.is_none());
        }
        for field in ["balance", "apr", "minimum", "fee", "payment"] {
            let mut data = fixture();
            match field {
                "balance" => data.accounts[0].balance = None,
                "apr" => data.accounts[0].apr = None,
                "minimum" => data.accounts[0].minimum_payment = None,
                "fee" => data.accounts[0].monthly_fee = None,
                _ => data.monthly_payment = None,
            };
            assert!(calculate(data).unwrap().1.snowball.is_none());
        }
        let mut data = fixture();
        data.accounts = vec![account("Already zero", "0", "0", "0")];
        data.accounts[0].apr = None;
        data.accounts[0].interest_model = InterestModel::Unknown;
        let (_, c) = calculate(data).unwrap();
        let p = c.snowball.unwrap();
        assert_eq!(p.status, Status::PaidOff);
        assert!(p.schedule.is_empty() && p.payoff_month.is_none());
    }
    #[test]
    fn negative_amortization_horizon_and_model_limit_do_not_claim_payoff() {
        let mut data = fixture();
        data.accounts = vec![account("Growing", "100", "12", "0")];
        data.monthly_payment = Some("0.50".to_owned());
        let (_, c) = calculate(data).unwrap();
        let p = c.snowball.unwrap();
        assert_eq!(p.status, Status::HorizonReached);
        assert_eq!(p.schedule.len(), 600);
        assert_eq!(p.schedule[0].balances, vec!["100.50"]);
        assert_eq!(p.negative_amortization_accounts, vec![0]);
        assert!(p.payoff_month.is_none());
        let mut data = fixture();
        data.accounts = vec![account("High rate", "1000000000", "1000", "0")];
        data.monthly_payment = Some("0".to_owned());
        let (_, c) = calculate(data).unwrap();
        let p = c.snowball.unwrap();
        assert_eq!(p.status, Status::ModelLimit);
        assert!(p.payoff_month.is_none());
    }
    #[test]
    fn rate_precision_half_cent_validation_and_deterministic_ties() {
        let mut data = fixture();
        data.accounts = vec![account("Half cent", "0.50", "12", "0")];
        data.monthly_payment = Some("1".to_owned());
        assert_eq!(
            calculate(data).unwrap().1.snowball.unwrap().total_interest,
            "0.01"
        );
        for apr in ["-1", "1e2", "NaN", "12.12345", "1000.0001", ".5", "1."] {
            let mut data = fixture();
            data.accounts[0].apr = Some(apr.to_owned());
            assert!(calculate(data).is_err());
        }
        for month in [
            "2026-13",
            "2026-00",
            "2026-+1",
            "2026-1",
            "2026-11junk",
            "2101-01",
        ] {
            let mut data = fixture();
            data.start_month = month.to_owned();
            assert!(calculate(data).is_err());
        }
        let mut data = fixture();
        data.accounts = vec![
            account("First", "100", "12", "1"),
            account("Second", "100", "12", "1"),
        ];
        data.monthly_payment = Some("10".to_owned());
        let (_, c) = calculate(data).unwrap();
        assert_eq!(
            c.snowball.unwrap().schedule[0].payments,
            vec!["9.00", "1.00"]
        );
        assert_eq!(
            c.avalanche.unwrap().schedule[0].payments,
            vec!["9.00", "1.00"]
        );
        let mut data = fixture();
        data.accounts = vec![account("A", "1", "0", "0"); 51];
        assert!(calculate(data).is_err());
        let mut data = fixture();
        data.extras = vec![
            DatedAmount {
                month: "2026-11".to_owned(),
                amount: "10".to_owned()
            };
            2
        ];
        assert!(calculate(data).is_err());
    }
    #[test]
    fn capacity_flags_use_saved_budget_and_include_changed_and_extra_payments() {
        let (data, mut c) = calculate(fixture()).unwrap();
        let context = BudgetContext {
            revision: 1,
            monthly_capacity: "99.99".to_owned(),
            planned_debt_payments: "50.00".to_owned(),
            warnings: vec![],
        };
        apply_budget_context(&data, &mut c, &context).unwrap();
        assert!(c.warnings.iter().any(|w| w.contains("exceed")));
        let mut data = fixture();
        data.payment_changes = vec![DatedAmount {
            month: "2026-12".to_owned(),
            amount: "200".to_owned(),
        }];
        let (data, mut c) = calculate(data).unwrap();
        let context = BudgetContext {
            monthly_capacity: "150.00".to_owned(),
            ..context
        };
        apply_budget_context(&data, &mut c, &context).unwrap();
        assert!(c.warnings.iter().any(|w| w.contains("exceed")));
        let context = BudgetContext {
            monthly_capacity: "433333333333.00".to_owned(),
            ..context
        };
        let (data, mut c) = calculate(fixture()).unwrap();
        apply_budget_context(&data, &mut c, &context).unwrap();
        assert!(!c.warnings.iter().any(|w| w.contains("exceed")));
    }
}

#[cfg(test)]
mod advanced_tests {
    use super::*;
    fn daily() -> PlanData {
        let mut d = tests::fixture();
        d.accounts.truncate(1);
        let a = &mut d.accounts[0];
        a.balance = Some("1000".into());
        a.apr = Some("12".into());
        a.minimum_payment = Some("10".into());
        a.interest_model = InterestModel::Daily;
        d.advanced = Some(Advanced {
            accounts: vec![AdvancedAccount {
                account_index: 0,
                day_basis: Some(365),
                rate_changes: vec![],
                fees: vec![],
            }],
            payments: None,
        });
        d
    }
    #[test]
    fn actual_days_daily_interest_leap_year_and_explicit_unknowns() {
        let d = daily();
        let c = calculate(d.clone()).unwrap().1;
        let p = c.snowball.unwrap();
        assert_eq!(p.schedule[0].interest, vec!["9.86"]);
        assert_eq!(p.schedule[0].balances, vec!["909.86"]);
        let mut leap = d.clone();
        leap.start_month = "2028-02".into();
        assert_eq!(
            calculate(leap).unwrap().1.snowball.unwrap().schedule[0].interest,
            vec!["9.53"]
        );
        let mut unknown = d;
        unknown.advanced.as_mut().unwrap().accounts[0].day_basis = None;
        assert!(calculate(unknown).unwrap().1.snowball.is_none());
    }
    #[test]
    fn weekly_payments_split_monthly_budget_and_accrue_between_actual_dates() {
        let mut d = daily();
        d.accounts[0].apr = Some("36.5".into());
        d.monthly_payment = Some("400".into());
        d.advanced.as_mut().unwrap().payments = Some(PaymentSchedule {
            first_date: "2026-11-07".into(),
            frequency: crate::cashflow::Frequency::Weekly,
            second_day: None,
        });
        let p = calculate(d).unwrap().1.snowball.unwrap();
        assert_eq!(p.schedule[0].total_payment, "400.00");
        assert_eq!(p.schedule[0].interest, vec!["25.32"]);
        assert_eq!(p.schedule[0].balances, vec!["625.32"]);
        assert_eq!(p.dated_payments[0].date, "2026-11-07");
        assert_eq!(p.dated_payments[0].interest, vec!["7.00"]);
        assert_eq!(p.dated_payments[4].payments, vec!["0.00"]);
    }
    #[test]
    fn promotional_reversion_known_fees_and_current_rate_priority_are_explicit() {
        let mut d = daily();
        d.accounts[0].interest_model = InterestModel::Promotional;
        d.accounts[0].apr = Some("0".into());
        let a = &mut d.advanced.as_mut().unwrap().accounts[0];
        a.day_basis = None;
        a.rate_changes = vec![RateChange {
            month: "2026-12".into(),
            apr: "24".into(),
        }];
        a.fees = vec![DatedAmount {
            month: "2026-12".into(),
            amount: "20".into(),
        }];
        let p = calculate(d.clone()).unwrap().1.avalanche.unwrap();
        assert_eq!(p.schedule[0].balances, vec!["900.00"]);
        assert_eq!(p.schedule[1].interest, vec!["18.00"]);
        assert_eq!(p.schedule[1].fees, vec!["20.00"]);
        assert_eq!(p.schedule[1].balances, vec!["838.00"]);
        d.advanced.as_mut().unwrap().accounts[0]
            .rate_changes
            .clear();
        assert!(calculate(d).unwrap().1.avalanche.is_none());
    }
    #[test]
    fn duplicate_out_of_range_and_incompatible_terms_are_rejected_or_withheld() {
        let mut d = daily();
        d.advanced.as_mut().unwrap().accounts[0].day_basis = Some(364);
        assert!(calculate(d).is_err());
        let mut d = daily();
        let a = d.advanced.as_ref().unwrap().accounts[0].clone();
        d.advanced.as_mut().unwrap().accounts.push(a);
        assert!(calculate(d).is_err());
        let mut d = daily();
        d.advanced.as_mut().unwrap().payments = Some(PaymentSchedule {
            first_date: "2026-12-01".into(),
            frequency: crate::cashflow::Frequency::Monthly,
            second_day: None,
        });
        assert!(calculate(d).is_err());
    }
}

#[cfg(test)]
mod targeted_tests {
    use super::*;
    #[test]
    fn targeted_extras_keep_unused_money_separate_and_reserve_whole_commitment() {
        let mut d = tests::fixture();
        d.targeted_extras = vec![TargetExtra {
            account_index: 0,
            month: "2026-11".into(),
            amount: "150".into(),
        }];
        let (d, c) = calculate(d).unwrap();
        let p = c.avalanche.unwrap();
        assert_eq!(p.schedule[0].payments, vec!["100.00", "90.00"]);
        assert_eq!(p.schedule[0].available_payment, "250.00");
        assert_eq!(p.schedule[0].total_payment, "190.00");
        assert_eq!(all_extras(&d)[0].amount, "150.00");
        let mut duplicate = d.clone();
        duplicate.targeted_extras.push(d.targeted_extras[0].clone());
        assert!(calculate(duplicate).is_err());
        let mut wrong = d.clone();
        wrong.targeted_extras[0].account_index = 2;
        assert!(calculate(wrong).is_err());
        let mut short = d;
        short.monthly_payment = Some("20".into());
        assert_eq!(
            calculate(short).unwrap().1.avalanche.unwrap().status,
            Status::InsufficientBudget
        );
    }
}

#[cfg(test)]
mod changing_priority_tests {
    use super::*;
    #[test]
    fn avalanche_uses_the_effective_rate_in_each_month_for_both_minima_and_extra_priority() {
        let mut d = tests::fixture();
        d.accounts[0].interest_model = InterestModel::Variable;
        d.advanced = Some(Advanced {
            accounts: vec![AdvancedAccount {
                account_index: 0,
                day_basis: None,
                rate_changes: vec![RateChange {
                    month: "2026-12".into(),
                    apr: "60".into(),
                }],
                fees: vec![],
            }],
            payments: None,
        });
        let c = calculate(d).unwrap().1;
        let p = c.avalanche.unwrap();
        assert_eq!(p.schedule[0].payments, vec!["10.00", "90.00"]);
        assert_eq!(p.schedule[1].interest, vec!["4.50", "8.40"]);
        assert_eq!(p.schedule[1].payments, vec!["80.00", "20.00"]);
        assert_eq!(p.schedule[1].balances, vec!["14.50", "408.40"]);
    }
}
