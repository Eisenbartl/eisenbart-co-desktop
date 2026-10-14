//! Optional, non-writing planning frameworks. Money and percentages use integers.
use crate::budget::{self, ClientBudget, Kind};
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub required_debt_minimum: Option<String>,
    pub needs_percent: String,
    pub wants_percent: String,
    pub future_percent: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub label: String,
    pub percent: String,
    pub actual: Option<String>,
    pub target: Option<String>,
    pub difference: Option<String>,
    pub position: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Method {
    pub label: String,
    pub buckets: Vec<Bucket>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZeroBased {
    pub income: Option<String>,
    pub assigned: Option<String>,
    pub remaining: Option<String>,
    pub status: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub client_id: String,
    pub revision: i64,
    pub calculator: String,
    pub required_debt_minimum: Option<String>,
    pub zero_based: ZeroBased,
    pub standard: Method,
    pub custom: Method,
    pub warnings: Vec<String>,
}
fn percent(value: &str) -> Result<i64, String> {
    let v = value.trim();
    let mut parts = v.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    if v.len() > 6
        || whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || parts.next().is_some()
        || fraction
            .is_some_and(|s| s.is_empty() || s.len() > 2 || !s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("Enter percentages from 0 to 100 with at most two decimal places.".into());
    }
    let n = whole.parse::<i64>().map_err(|_| "Invalid percentage.")? * 100
        + match fraction {
            None => 0,
            Some(s) if s.len() == 1 => s.parse::<i64>().map_err(|_| "Invalid percentage.")? * 10,
            Some(s) => s.parse::<i64>().map_err(|_| "Invalid percentage.")?,
        };
    if n > 10000 {
        return Err("Each percentage must be between 0 and 100.".into());
    }
    Ok(n)
}
fn cents(v: &str) -> Result<i64, String> {
    let n = v
        .replace('.', "")
        .parse::<i64>()
        .map_err(|_| "Invalid saved budget amount.")?;
    if budget::money(n) != v {
        return Err("Invalid saved budget precision.".into());
    }
    Ok(n)
}
fn known(b: &ClientBudget, kind: Kind, required: bool) -> bool {
    let rows = b
        .lines
        .iter()
        .filter(|l| l.kind == kind)
        .collect::<Vec<_>>();
    (!required || !rows.is_empty()) && rows.iter().all(|l| l.amount.is_some())
}
fn targets(income: i64, p: [i64; 3]) -> [i64; 3] {
    // Floor each share, then distribute remaining cents by largest fractional
    // remainder. Ties go needs, wants, future; the shares always sum to income.
    let products = p.map(|v| i128::from(income) * i128::from(v));
    let mut values = products.map(|v| (v / 10000) as i64);
    let mut order = [0, 1, 2];
    order.sort_by_key(|&i| (-(products[i] % 10000), i));
    let remainder = income - values.iter().sum::<i64>();
    for &i in order.iter().take(remainder as usize) {
        values[i] += 1;
    }
    values
}
fn method(label: &str, p: [i64; 3], income: Option<i64>, actual: [Option<i64>; 3]) -> Method {
    let targets = income.map(|v| targets(v, p));
    Method {
        label: label.into(),
        buckets: [
            "Needs (including required debt minimums)",
            "Wants / discretionary",
            "Savings and extra debt",
        ]
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let target = targets.map(|v| v[i]);
            let difference = actual[i].zip(target).map(|(a, t)| a - t);
            Bucket {
                label: (*label).into(),
                percent: budget::money(p[i]),
                actual: actual[i].map(budget::money),
                target: target.map(budget::money),
                difference: difference.map(budget::money),
                position: match difference {
                    None => "unavailable",
                    Some(0) => "at",
                    Some(v) if v > 0 => "above",
                    _ => "below",
                }
                .into(),
            }
        })
        .collect(),
    }
}
pub fn compare(b: &ClientBudget, input: Input) -> Result<Comparison, String> {
    if input.expected_revision != b.revision || input.expected_revision < 0 {
        return Err("The saved budget changed. Reload the saved budget and compare again; nothing was changed.".into());
    }
    let (lines, summary) = budget::calculate(b.lines.clone())?;
    if lines != b.lines || summary != b.summary {
        return Err("The saved budget is invalid.".into());
    }
    let percentages = [
        percent(&input.needs_percent)?,
        percent(&input.wants_percent)?,
        percent(&input.future_percent)?,
    ];
    if percentages.iter().sum::<i64>() != 10000 {
        return Err(
            "Custom needs, wants and future percentages must total exactly 100.00%.".into(),
        );
    }
    let income = known(b, Kind::Income, true)
        .then(|| cents(&b.summary.monthly_income))
        .transpose()?;
    let necessary = known(b, Kind::Necessary, true)
        .then(|| cents(&b.summary.necessary_expenses))
        .transpose()?;
    let wants = known(b, Kind::Discretionary, false)
        .then(|| cents(&b.summary.discretionary_expenses))
        .transpose()?;
    let savings = known(b, Kind::Savings, false)
        .then(|| cents(&b.summary.proposed_savings))
        .transpose()?;
    let debt = known(b, Kind::DebtPayment, false)
        .then(|| cents(&b.summary.planned_debt_payments))
        .transpose()?;
    let minimum = match input.required_debt_minimum {
        Some(v) => {
            let amount = budget::parse_money(v.trim())?;
            let debt = debt.ok_or(
                "Review the unknown saved debt allocation before classifying its minimum payments.",
            )?;
            if amount > debt {
                return Err("Required debt minimums cannot exceed the saved planned debt allocation. This input classifies existing debt payments; it does not add new payments.".into());
            }
            Some(amount)
        }
        None if debt == Some(0) => Some(0),
        None => None,
    };
    let actual = [
        necessary.zip(minimum).map(|(n, m)| n + m),
        wants,
        savings.zip(debt).zip(minimum).map(|((s, d), m)| s + d - m),
    ];
    let assigned = necessary
        .zip(wants)
        .zip(savings)
        .zip(debt)
        .map(|(((n, w), s), d)| n + w + s + d);
    let remaining = income.zip(assigned).map(|(i, a)| i - a);
    let status = match remaining {
        None => "incomplete",
        Some(0) => "fullyAssigned",
        Some(v) if v > 0 => "unassigned",
        _ => "overAssigned",
    };
    let mut warnings = b.summary.warnings.clone();
    warnings.push("These optional planning frameworks use only the saved entered budget, not approved historical transactions or independently synchronized debt/savings/cash-flow models. They are references, not mandatory rules or proof of complete finances.".into());
    warnings.push("Required debt minimums are reclassified from the existing debt allocation into needs; only extra debt and proposed savings enter the future bucket. Never enter the same debt payment in both necessary expenses and debt allocations.".into());
    warnings.push("Needs/wants percentages are reference spending ceilings; the future percentage is a savings/extra-debt reference. Below a spending ceiling does not mean that money should be spent. Unknowns cannot establish compliance.".into());
    warnings.push("Percentage targets use largest-remainder cent allocation, with ties assigned to needs, then wants, then future, so rounded targets sum exactly to entered monthly income. Comparison inputs/results are temporary and not part of saved reports or agreements.".into());
    if minimum.is_none() {
        warnings.push("Required debt minimums are unknown: needs and savings/extra-debt comparisons remain unavailable. Enter the known minimum portion of the saved debt allocation, including explicit 0 when appropriate.".into());
    }
    if income == Some(0) {
        warnings.push("Entered income is zero; targets are zero reference amounts, not verified absence of income or proof of affordability.".into());
    }
    if b.lines.iter().all(|l| l.kind != Kind::DebtPayment) {
        warnings.push("No debt allocation is entered. Zero here describes this budget only, not verification that no debt or required payments exist.".into());
    }
    Ok(Comparison {
        client_id: b.client_id.clone(),
        revision: b.revision,
        calculator: "budget-framework-v1".into(),
        required_debt_minimum: minimum.map(budget::money),
        zero_based: ZeroBased {
            income: income.map(budget::money),
            assigned: assigned.map(budget::money),
            remaining: remaining.map(budget::money),
            status: status.into(),
        },
        standard: method("50/30/20 reference", [5000, 3000, 2000], income, actual),
        custom: method("Custom percentage reference", percentages, income, actual),
        warnings,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::{BudgetLine, Frequency};
    fn input() -> Input {
        Input {
            expected_revision: 1,
            required_debt_minimum: Some("100".into()),
            needs_percent: "60".into(),
            wants_percent: "20".into(),
            future_percent: "20".into(),
        }
    }
    fn fixture() -> ClientBudget {
        let (lines, summary) = budget::calculate(
            [
                (Kind::Income, "1500", Frequency::Biweekly),
                (Kind::Necessary, "1200", Frequency::Monthly),
                (Kind::Necessary, "100", Frequency::Weekly),
                (Kind::Discretionary, "200", Frequency::Monthly),
                (Kind::Savings, "300", Frequency::Monthly),
                (Kind::DebtPayment, "400", Frequency::Monthly),
            ]
            .into_iter()
            .map(|(kind, amount, frequency)| BudgetLine {
                label: "Fictional budget entry".into(),
                kind,
                amount: Some(amount.into()),
                frequency,
                estimated: false,
            })
            .collect(),
        )
        .unwrap();
        ClientBudget {
            client_id: "fictional-client".into(),
            revision: 1,
            updated_at: None,
            lines,
            summary,
        }
    }
    #[test]
    fn independent_allocation_and_minimum_reclassification() {
        let r = compare(&fixture(), input()).unwrap();
        assert_eq!(r.zero_based.remaining.as_deref(), Some("716.67"));
        assert_eq!(r.zero_based.assigned.as_deref(), Some("2533.33"));
        let s = &r.standard.buckets;
        assert_eq!(s[0].actual.as_deref(), Some("1733.33"));
        assert_eq!(s[0].target.as_deref(), Some("1625.00"));
        assert_eq!(s[0].difference.as_deref(), Some("108.33"));
        assert_eq!(s[1].difference.as_deref(), Some("-775.00"));
        assert_eq!(s[2].actual.as_deref(), Some("600.00"));
        assert_eq!(s[2].difference.as_deref(), Some("-50.00"));
        assert_eq!(r.custom.buckets[0].target.as_deref(), Some("1950.00"));
        assert_eq!(r.custom.buckets[0].difference.as_deref(), Some("-216.67"));
    }
    #[test]
    fn independent_last_cent_ties_and_fractional_percentages() {
        assert_eq!(targets(1, [5000, 3000, 2000]), [1, 0, 0]);
        assert_eq!(targets(2, [5000, 3000, 2000]), [1, 1, 0]);
        assert_eq!(targets(3, [5000, 3000, 2000]), [1, 1, 1]);
        assert_eq!(targets(101, [3333, 3333, 3334]), [34, 33, 34]);
        assert_eq!(targets(1, [0, 0, 10000]), [0, 0, 1]);
        assert_eq!(percent("33.34").unwrap(), 3334);
        let big = targets(43_333_333_333_300, [5000, 3000, 2000]);
        assert_eq!(
            big,
            [21_666_666_666_650, 12_999_999_999_990, 8_666_666_666_660]
        );
    }
    #[test]
    fn unknowns_zero_deficits_and_invalid_framework_inputs() {
        let b = fixture();
        let mut i = input();
        i.required_debt_minimum = None;
        let r = compare(&b, i).unwrap();
        assert!(r.standard.buckets[0].actual.is_none());
        assert!(r.standard.buckets[2].difference.is_none());
        assert_eq!(r.zero_based.remaining.as_deref(), Some("716.67"));
        let mut lines = b.lines.clone();
        lines[0].amount = None;
        let (lines, summary) = budget::calculate(lines).unwrap();
        let r = compare(
            &ClientBudget {
                lines,
                summary,
                ..b
            },
            input(),
        )
        .unwrap();
        assert_eq!(r.zero_based.status, "incomplete");
        assert!(r.standard.buckets[0].target.is_none());
        for v in ["-1", "100.01", "1e2", "33.333", "NaN", ""] {
            let mut i = input();
            i.needs_percent = v.into();
            assert!(compare(&fixture(), i).is_err());
        }
        let mut i = input();
        i.future_percent = "19.99".into();
        assert!(compare(&fixture(), i).is_err());
        let mut i = input();
        i.required_debt_minimum = Some("400.01".into());
        assert!(compare(&fixture(), i).is_err());
        let mut i = input();
        i.expected_revision = 0;
        assert!(compare(&fixture(), i).is_err());
        let mut lines = fixture().lines;
        for l in &mut lines {
            l.amount = Some("0".into());
        }
        let (lines, summary) = budget::calculate(lines).unwrap();
        let r = compare(
            &ClientBudget {
                client_id: "zero".into(),
                revision: 1,
                updated_at: None,
                lines,
                summary,
            },
            Input {
                required_debt_minimum: None,
                ..input()
            },
        )
        .unwrap();
        assert_eq!(r.zero_based.status, "fullyAssigned");
        assert_eq!(r.standard.buckets[0].target.as_deref(), Some("0.00"));
        let mut b = fixture();
        b.summary.monthly_income = "9999.99".into();
        assert!(compare(&b, input()).is_err());
    }
    #[test]
    fn deficit_balanced_missing_expenses_and_cent_conservation() {
        let make = |entries: Vec<(Kind, Option<&str>)>| {
            let (lines, summary) = budget::calculate(
                entries
                    .into_iter()
                    .map(|(kind, amount)| BudgetLine {
                        label: "Fictional reference".into(),
                        kind,
                        amount: amount.map(String::from),
                        frequency: Frequency::Monthly,
                        estimated: false,
                    })
                    .collect(),
            )
            .unwrap();
            ClientBudget {
                client_id: "reference".into(),
                revision: 1,
                updated_at: None,
                lines,
                summary,
            }
        };
        let deficit = make(vec![
            (Kind::Income, Some("100")),
            (Kind::Necessary, Some("150")),
            (Kind::Savings, Some("10")),
            (Kind::DebtPayment, Some("20")),
        ]);
        let r = compare(
            &deficit,
            Input {
                required_debt_minimum: Some("0".into()),
                ..input()
            },
        )
        .unwrap();
        assert_eq!(r.zero_based.remaining.as_deref(), Some("-80.00"));
        assert_eq!(r.zero_based.status, "overAssigned");
        let balanced = make(vec![
            (Kind::Income, Some("100")),
            (Kind::Necessary, Some("50")),
            (Kind::Discretionary, Some("30")),
            (Kind::Savings, Some("20")),
        ]);
        let r = compare(
            &balanced,
            Input {
                required_debt_minimum: None,
                ..input()
            },
        )
        .unwrap();
        assert_eq!(r.zero_based.status, "fullyAssigned");
        assert!(r
            .standard
            .buckets
            .iter()
            .all(|b| b.difference.as_deref() == Some("0.00")));
        let missing = make(vec![(Kind::Income, Some("100")), (Kind::Necessary, None)]);
        let r = compare(
            &missing,
            Input {
                required_debt_minimum: None,
                ..input()
            },
        )
        .unwrap();
        assert!(r.zero_based.remaining.is_none());
        assert!(r.standard.buckets[0].actual.is_none());
        assert_eq!(r.standard.buckets[0].target.as_deref(), Some("50.00"));
        for income in 0..1000 {
            for p in [
                [5000, 3000, 2000],
                [3333, 3333, 3334],
                [10000, 0, 0],
                [0, 5000, 5000],
            ] {
                let shares = targets(income, p);
                assert_eq!(shares.iter().sum::<i64>(), income);
                assert!(shares.iter().all(|v| *v >= 0));
            }
        }
    }
}
