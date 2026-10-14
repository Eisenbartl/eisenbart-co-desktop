//! Monthly planning estimates. Never infer missing contractual terms.
use crate::budget::{money, parse_money};
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
pub struct PlanData {
    pub start_month: String,
    pub monthly_payment: Option<String>,
    pub accounts: Vec<Account>,
    pub extras: Vec<DatedAmount>,
    pub payment_changes: Vec<DatedAmount>,
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
}

pub fn calculate(mut data: PlanData) -> Result<(PlanData, Comparison), String> {
    let start = month_number(&data.start_month)?;
    if data.accounts.len() > 50 {
        return Err("Use at most 50 debt accounts.".to_owned());
    }
    let payment = optional_money(&mut data.monthly_payment)?;
    let extras = dated_amounts(&mut data.extras, start)?;
    let changes = dated_amounts(&mut data.payment_changes, start)?;
    let mut warnings = vec!["Planning estimate: fixed APR ÷ 12, interest on the opening balance rounded to cents before monthly fees and end-of-month payments. No new borrowing.".to_owned()];
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
        let active = balance != Some(0);
        if active
            && (balance.is_none()
                || minimum.is_none()
                || fee.is_none()
                || apr.is_none()
                || account.interest_model != InterestModel::FixedMonthly)
        {
            incomplete = true;
            warnings.push(format!("Account {} has missing or unmodeled terms. No payoff comparison is produced until all active accounts have a balance, APR, minimum, monthly fee, and fixed monthly model.", index + 1));
        }
        if active && !account.verified {
            warnings.push(format!("Account {} terms are unverified; check the lender's terms before relying on this estimate.", index + 1));
        }
        terms.push(Terms {
            balance: balance.unwrap_or(0),
            apr: apr.unwrap_or(0),
            minimum: minimum.unwrap_or(0),
            fee: fee.unwrap_or(0),
        });
    }
    if !extras.is_empty() {
        warnings.push("Extra payments require separately verified funding; check cash availability in each event month.".to_owned());
    }
    let (snowball, avalanche) = if incomplete {
        (None, None)
    } else {
        let payment = payment.unwrap_or(0);
        (
            Some(project(&terms, start, payment, &extras, &changes, false)),
            Some(project(&terms, start, payment, &extras, &changes, true)),
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

fn project(
    terms: &[Terms],
    start: i32,
    mut payment: i64,
    extras: &BTreeMap<i32, i64>,
    changes: &BTreeMap<i32, i64>,
    avalanche: bool,
) -> Projection {
    let mut balances: Vec<i64> = terms.iter().map(|t| t.balance).collect();
    let mut payoff_months = vec![None; terms.len()];
    let mut schedule = Vec::new();
    let mut paid = 0;
    let mut interest_total = 0;
    let mut fees_total = 0;
    let mut negative = vec![false; terms.len()];
    let mut warnings = Vec::new();
    let mut status = Status::HorizonReached;
    for number in start..start + MONTHS {
        if balances.iter().all(|b| *b == 0) {
            status = Status::PaidOff;
            break;
        }
        if let Some(changed) = changes.get(&number) {
            payment = *changed;
        }
        let available = payment + extras.get(&number).copied().unwrap_or(0);
        let month = month_label(number);
        // Validated balances <= 1e15 cents and APR <= 1e7 units bound each
        // charge below 8.34e14 cents. i128 multiplication avoids intermediate
        // overflow. Across all months, total payments <= 600 * 2e11 cents;
        // total charges are bounded by those payments plus <= 50 * 1e15
        // ending cents. All subsequent sums therefore fit comfortably in i64.
        let interest: Vec<i64> = balances
            .iter()
            .zip(terms)
            .map(|(balance, t)| {
                ((i128::from(*balance) * i128::from(t.apr) + 6_000_000) / 12_000_000) as i64
            })
            .collect();
        let fees: Vec<i64> = balances
            .iter()
            .zip(terms)
            .map(|(balance, t)| if *balance > 0 { t.fee } else { 0 })
            .collect();
        let owed: Vec<i64> = balances
            .iter()
            .zip(&interest)
            .zip(&fees)
            .map(|((b, i), f)| b + i + f)
            .collect();
        let mut payments: Vec<i64> = owed
            .iter()
            .zip(terms)
            .map(|(owed, t)| (*owed).min(t.minimum))
            .collect();
        let required: i64 = payments.iter().sum();
        if available < required {
            status = Status::InsufficientBudget;
            warnings.push(format!("{month}: available payment {} is below required minimum payments {}. Projection stops before an unaffordable payment; no debt-free date is assigned.", money(available), money(required)));
            break;
        }
        let mut extra = available - required;
        let mut order: Vec<usize> = (0..terms.len()).filter(|i| balances[*i] > 0).collect();
        order.sort_by(|a, b| {
            if avalanche {
                terms[*b]
                    .apr
                    .cmp(&terms[*a].apr)
                    .then(balances[*a].cmp(&balances[*b]))
                    .then(a.cmp(b))
            } else {
                balances[*a]
                    .cmp(&balances[*b])
                    .then(terms[*b].apr.cmp(&terms[*a].apr))
                    .then(a.cmp(b))
            }
        });
        for i in order {
            let added = extra.min(owed[i] - payments[i]);
            payments[i] += added;
            extra -= added;
        }
        let closing: Vec<i64> = owed.iter().zip(&payments).map(|(o, p)| o - p).collect();
        if closing.iter().any(|b| *b > BALANCE_LIMIT) {
            status = Status::ModelLimit;
            warnings.push("Growing balances exceeded the model's safety limit. No debt-free date is assigned.".to_owned());
            break;
        }
        for i in 0..terms.len() {
            if closing[i] > balances[i] {
                negative[i] = true;
            }
            if balances[i] > 0 && closing[i] == 0 {
                payoff_months[i] = Some(month.clone());
            }
        }
        let monthly_paid: i64 = payments.iter().sum();
        paid += monthly_paid;
        interest_total += interest.iter().sum::<i64>();
        fees_total += fees.iter().sum::<i64>();
        schedule.push(Month {
            month,
            available_payment: money(available),
            payments: payments.iter().map(|v| money(*v)).collect(),
            interest: interest.iter().map(|v| money(*v)).collect(),
            fees: fees.iter().map(|v| money(*v)).collect(),
            balances: closing.iter().map(|v| money(*v)).collect(),
            total_payment: money(monthly_paid),
            remaining_balance: money(closing.iter().sum()),
        });
        balances = closing;
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
    }
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
