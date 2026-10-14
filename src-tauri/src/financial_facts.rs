//! Explicit counselor-verified facts, separate from planning estimates.
use crate::{
    budget::{money, parse_money},
    imports::{self, History, Kind, Record, Status},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
pub const MAX_JSON: usize = 256 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Necessary,
    Discretionary,
    TakeHomePay,
    OtherIncome,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Classification {
    pub record_id: String,
    pub approval_id: String,
    pub role: Role,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BalanceKind {
    Cash,
    AccessibleSavings,
    RestrictedSavings,
    OtherAsset,
    Debt,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Balance {
    pub label: String,
    pub kind: BalanceKind,
    pub amount: Option<String>,
    pub monthly_payment: Option<String>,
    pub evidence: String,
    pub document_id: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Data {
    pub as_of: Option<String>,
    pub full_inventory: bool,
    pub balances: Vec<Balance>,
    pub classifications: Vec<Classification>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub data: Data,
    pub reason: String,
    pub acknowledge: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Review {
    pub id: String,
    pub data: Data,
    pub reason: String,
    pub reviewed_at: String,
    pub counselor_name: String,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    pub data: Option<Data>,
    pub history: Vec<Review>,
}
fn text(value: &str, label: &str, max: usize) -> Result<String, String> {
    let v = value.trim();
    if v.is_empty() || v.chars().count() > max || v.chars().any(char::is_control) {
        return Err(format!(
            "{label} must contain 1–{max} characters without control characters."
        ));
    }
    Ok(v.into())
}
pub fn normalize(mut data: Data) -> Result<Data, String> {
    if data.balances.len() > 50 || data.classifications.len() > 500 {
        return Err(
            "Use at most 50 balances and 500 transaction classifications per facts snapshot."
                .into(),
        );
    }
    if (!data.balances.is_empty() || data.full_inventory) && data.as_of.is_none() {
        return Err("An as-of date is required for balance facts.".into());
    }
    if let Some(date) = &data.as_of {
        imports::iso_date(date)?;
    }
    let mut labels = HashSet::new();
    for b in &mut data.balances {
        b.label = text(&b.label, "Balance label", 120)?;
        b.evidence = text(&b.evidence, "Balance evidence", 500)?;
        if !labels.insert(b.label.to_lowercase()) {
            return Err(
                "Use a unique balance label for each account or asset; do not list it twice."
                    .into(),
            );
        }
        if b.monthly_payment.is_some() && b.kind != BalanceKind::Debt {
            return Err("Monthly required payments belong only to debt balances.".into());
        }
        for amount in [&mut b.amount, &mut b.monthly_payment]
            .into_iter()
            .flatten()
        {
            *amount = money(parse_money(amount)?);
        }
    }
    let mut ids = HashSet::new();
    for c in &data.classifications {
        if !ids.insert(&c.record_id) {
            return Err("Classify each transaction once.".into());
        }
    }
    if serde_json::to_vec(&data)
        .map_err(|_| "Facts could not be encoded.")?
        .len()
        > MAX_JSON
    {
        return Err("Financial facts exceed their local size limit.".into());
    }
    Ok(data)
}
fn compatible(role: &Role, fields: &imports::Fields) -> bool {
    match role {
        Role::TakeHomePay | Role::OtherIncome => fields.kind == Kind::Income,
        Role::Necessary | Role::Discretionary => {
            matches!(fields.kind, Kind::Expense | Kind::Fee)
                || (fields.kind == Kind::Interest
                    && fields
                        .amount
                        .as_deref()
                        .and_then(|v| imports::signed_money(v).ok())
                        .is_some_and(|v| v < 0))
        }
    }
}
pub fn validate_bindings(
    data: &Data,
    records: &[Record],
    history: &[History],
    document_ids: &HashSet<&str>,
) -> Result<(), String> {
    for c in &data.classifications {
        if !records.iter().any(|r| r.id == c.record_id) {
            return Err("The classified transaction is unavailable for this client.".into());
        }
        let approval = history
            .iter()
            .find(|h| {
                h.id == c.approval_id
                    && h.record_id == c.record_id
                    && h.to_status == Status::Approved
            })
            .ok_or("The classification requires this client's exact approved source decision.")?;
        if !compatible(&c.role, &approval.fields) {
            return Err(
                "The classification is incompatible with its approved transaction type.".into(),
            );
        }
    }
    for b in &data.balances {
        if b.document_id
            .as_deref()
            .is_some_and(|id| !document_ids.contains(id))
        {
            return Err("Balance evidence document is unavailable for this client.".into());
        }
    }
    Ok(())
}
pub fn active(c: &Classification, record: &Record, history: &[History]) -> bool {
    record.status == Status::Approved
        && compatible(&c.role, &record.fields)
        && history
            .iter()
            .rev()
            .find(|h| h.record_id == record.id)
            .is_some_and(|h| {
                h.id == c.approval_id
                    && h.to_status == Status::Approved
                    && h.fields == record.fields
            })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceBar {
    pub label: String,
    pub amount: Option<String>,
    pub scale: u16,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub necessary_spending: String,
    pub discretionary_spending: String,
    pub unclassified_spending: String,
    pub take_home_pay: String,
    pub unclassified_income: String,
    pub stale_classifications: usize,
    pub as_of: Option<String>,
    pub balance_complete: bool,
    pub balance_aligned: bool,
    pub balance_outdated: bool,
    pub cash: Option<String>,
    pub accessible_savings: Option<String>,
    pub restricted_savings: Option<String>,
    pub debt: Option<String>,
    pub net_worth: Option<String>,
    pub monthly_necessary_estimate: Option<String>,
    pub emergency_months_estimate: Option<String>,
    pub take_home_payment_percent_estimate: Option<String>,
    pub debt_balances: Vec<BalanceBar>,
}
fn ratio(n: i64, d: i64, mult: i64) -> Option<String> {
    if d <= 0 {
        return None;
    }
    let v = i128::from(n) * i128::from(mult) * 100 / i128::from(d);
    Some(format!("{}.{:02}", v / 100, v % 100))
}
pub fn metrics(
    source: &imports::Snapshot,
    start: NaiveDate,
    end: NaiveDate,
    today: NaiveDate,
    months: i64,
) -> Result<Metrics, String> {
    let empty = Data::default();
    let data = source.financial_facts.data.as_ref().unwrap_or(&empty);
    let (mut necessary, mut discretionary, mut unknown, mut pay, mut unknown_income) =
        (0, 0, 0, 0, 0);
    let mut undecided = false;
    for r in &source.records {
        let Some(date) = r.fields.date.as_deref() else {
            if r.status != Status::Rejected {
                undecided = true;
            }
            continue;
        };
        let date = imports::iso_date(date)?;
        if date < start || date > end {
            continue;
        }
        if matches!(r.status, Status::Pending | Status::NeedsClarification) {
            undecided = true;
        }
        if r.status != Status::Approved {
            continue;
        }
        let role = data
            .classifications
            .iter()
            .find(|c| c.record_id == r.id && active(c, r, &source.history))
            .map(|c| &c.role);
        let n = imports::signed_money(
            r.fields
                .amount
                .as_deref()
                .ok_or("Approved amount missing.")?,
        )?;
        if matches!(r.fields.kind, Kind::Expense | Kind::Fee)
            || (r.fields.kind == Kind::Interest && n < 0)
        {
            match role {
                Some(Role::Necessary) => necessary -= n,
                Some(Role::Discretionary) => discretionary -= n,
                _ => unknown -= n,
            }
        }
        if r.fields.kind == Kind::Income {
            match role {
                Some(Role::TakeHomePay) => pay += n,
                Some(Role::OtherIncome) => {}
                _ => unknown_income += n,
            }
        }
    }
    let stale_classifications = data
        .classifications
        .iter()
        .filter(|c| {
            !source
                .records
                .iter()
                .any(|r| r.id == c.record_id && active(c, r, &source.history))
        })
        .count();
    let as_of = data.as_of.as_deref().map(imports::iso_date).transpose()?;
    let complete =
        data.full_inventory && as_of.is_some() && data.balances.iter().all(|b| b.amount.is_some());
    let aligned = as_of == Some(end);
    let outdated = as_of.is_some_and(|d| (today - d).num_days() > 90);
    let subtotal = |kind: BalanceKind| -> Result<Option<i64>, String> {
        let rows = data
            .balances
            .iter()
            .filter(|b| b.kind == kind)
            .collect::<Vec<_>>();
        if rows.is_empty() && !data.full_inventory {
            return Ok(None);
        }
        if rows.iter().any(|b| b.amount.is_none()) {
            return Ok(None);
        }
        Ok(Some(
            rows.iter()
                .map(|b| parse_money(b.amount.as_deref().unwrap()))
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .sum(),
        ))
    };
    let cash = subtotal(BalanceKind::Cash)?;
    let accessible = subtotal(BalanceKind::AccessibleSavings)?;
    let restricted = subtotal(BalanceKind::RestrictedSavings)?;
    let debt = subtotal(BalanceKind::Debt)?;
    let other = subtotal(BalanceKind::OtherAsset)?;
    let net_worth = if complete {
        Some(money(
            cash.unwrap() + accessible.unwrap() + restricted.unwrap() + other.unwrap()
                - debt.unwrap(),
        ))
    } else {
        None
    };
    let monthly_necessary = if unknown == 0 && !undecided {
        Some((necessary + months / 2) / months)
    } else {
        None
    };
    let emergency = if complete && aligned && !outdated {
        monthly_necessary.and_then(|n| accessible.and_then(|a| ratio(a, n, 1)))
    } else {
        None
    };
    let debt_rows = data
        .balances
        .iter()
        .filter(|b| b.kind == BalanceKind::Debt)
        .collect::<Vec<_>>();
    let required = if debt_rows.iter().all(|b| b.monthly_payment.is_some()) {
        Some(
            debt_rows
                .iter()
                .map(|b| parse_money(b.monthly_payment.as_deref().unwrap()))
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .sum(),
        )
    } else {
        None
    };
    let payment_ratio = if complete && aligned && !outdated && unknown_income == 0 && !undecided {
        required.and_then(|p| ratio(p, (pay + months / 2) / months, 100))
    } else {
        None
    };
    let max = debt_rows
        .iter()
        .filter_map(|b| b.amount.as_deref().and_then(|n| parse_money(n).ok()))
        .max()
        .unwrap_or(0);
    let debt_balances = debt_rows
        .into_iter()
        .map(|b| BalanceBar {
            label: b.label.clone(),
            amount: b.amount.clone(),
            scale: b
                .amount
                .as_deref()
                .and_then(|n| parse_money(n).ok())
                .filter(|_| max > 0)
                .map(|n| (i128::from(n) * 1000 / i128::from(max)) as u16)
                .unwrap_or(0),
        })
        .collect();
    Ok(Metrics {
        necessary_spending: money(necessary),
        discretionary_spending: money(discretionary),
        unclassified_spending: money(unknown),
        take_home_pay: money(pay),
        unclassified_income: money(unknown_income),
        stale_classifications,
        as_of: data.as_of.clone(),
        balance_complete: complete,
        balance_aligned: aligned,
        balance_outdated: outdated,
        cash: cash.map(money),
        accessible_savings: accessible.map(money),
        restricted_savings: restricted.map(money),
        debt: debt.map(money),
        net_worth,
        monthly_necessary_estimate: monthly_necessary.map(money),
        emergency_months_estimate: emergency,
        take_home_payment_percent_estimate: payment_ratio,
        debt_balances,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalization_keeps_unknown_zero_and_evidence_distinct_and_bounds_labels() {
        let mut d = Data {
            as_of: Some("2026-09-30".into()),
            full_inventory: false,
            balances: vec![Balance {
                label: " Checking ".into(),
                kind: BalanceKind::Cash,
                amount: Some("0".into()),
                monthly_payment: None,
                evidence: " Verified dated source ".into(),
                document_id: None,
            }],
            classifications: vec![],
        };
        let n = normalize(d.clone()).unwrap();
        assert_eq!(n.balances[0].amount.as_deref(), Some("0.00"));
        assert_eq!(n.balances[0].label, "Checking");
        d.balances[0].amount = None;
        assert_eq!(normalize(d.clone()).unwrap().balances[0].amount, None);
        d.balances[0].monthly_payment = Some("10".into());
        assert!(normalize(d.clone()).is_err());
        d.balances[0].monthly_payment = None;
        d.balances.push(d.balances[0].clone());
        d.balances[1].label = "checking".into();
        assert!(normalize(d.clone()).is_err());
        d.balances.pop();
        d.balances[0].label = "🔒".repeat(120);
        assert!(normalize(d.clone()).is_ok());
        d.balances[0].label.push('x');
        assert!(normalize(d.clone()).is_err());
        d.balances[0].label = "Checking".into();
        d.as_of = None;
        assert!(normalize(d.clone()).is_err());
        d.as_of = Some("2026-02-30".into());
        assert!(normalize(d.clone()).is_err());
        d.as_of = Some("2026-09-30".into());
        d.balances[0].evidence.clear();
        assert!(normalize(d).is_err());
    }
    #[test]
    fn independent_conservative_ratios_and_size_limits() {
        assert_eq!(ratio(240003, 120002, 1).as_deref(), Some("1.99"));
        assert_eq!(ratio(10001, 200001, 100).as_deref(), Some("5.00"));
        assert_eq!(ratio(0, 100, 1).as_deref(), Some("0.00"));
        assert_eq!(ratio(100, 0, 1), None);
        let d = Data {
            as_of: Some("2026-09-30".into()),
            full_inventory: false,
            balances: (0..51)
                .map(|i| Balance {
                    label: format!("{i}"),
                    kind: BalanceKind::Cash,
                    amount: None,
                    monthly_payment: None,
                    evidence: "Synthetic evidence".into(),
                    document_id: None,
                })
                .collect(),
            classifications: vec![],
        };
        assert!(normalize(d).is_err());
        let d = Data {
            classifications: (0..501)
                .map(|i| Classification {
                    record_id: format!("{i}"),
                    approval_id: "a".into(),
                    role: Role::Necessary,
                })
                .collect(),
            ..Data::default()
        };
        assert!(normalize(d).is_err());
    }
}
