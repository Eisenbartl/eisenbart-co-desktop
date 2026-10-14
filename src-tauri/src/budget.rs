//! USD planning values, independent of transactions. All arithmetic uses cents.
use serde::{Deserialize, Serialize};

const MAX_CENTS: i64 = 100_000_000_000; // $1 billion per entered amount

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Kind {
    Income,
    Necessary,
    Discretionary,
    Savings,
    DebtPayment,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Frequency {
    Weekly,
    Biweekly,
    Semimonthly,
    Monthly,
    Annual,
    Irregular,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetLine {
    pub label: String,
    pub kind: Kind,
    pub amount: Option<String>,
    pub frequency: Frequency,
    pub estimated: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetInput {
    pub expected_revision: i64,
    pub lines: Vec<BudgetLine>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub monthly_income: String,
    pub necessary_expenses: String,
    pub discretionary_expenses: String,
    pub total_spending: String,
    pub surplus_before_allocations: String,
    pub proposed_savings: String,
    pub available_for_debt: String,
    pub planned_debt_payments: String,
    pub unallocated: String,
    pub monthly_amounts: Vec<Option<String>>,
    pub missing_count: usize,
    pub estimated_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientBudget {
    pub client_id: String,
    pub revision: i64,
    pub updated_at: Option<String>,
    pub lines: Vec<BudgetLine>,
    pub summary: Summary,
}

pub fn money(cents: i64) -> String {
    let value = i128::from(cents).abs();
    format!(
        "{}{}.{:02}",
        if cents < 0 { "-" } else { "" },
        value / 100,
        value % 100
    )
}

pub(crate) fn parse_money(value: &str) -> Result<i64, String> {
    let invalid = || {
        "Enter a nonnegative USD amount with at most two decimal places (for example, 1250.50); no commas or currency symbols.".to_owned()
    };
    if value.is_empty() || value.len() > 16 {
        return Err(invalid());
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction
            .is_some_and(|s| s.is_empty() || s.len() > 2 || !s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(invalid());
    }
    let major: i64 = whole.parse().map_err(|_| invalid())?;
    let minor: i64 = match fraction {
        Some(s) if s.len() == 1 => s.parse::<i64>().map_err(|_| invalid())? * 10,
        Some(s) => s.parse().map_err(|_| invalid())?,
        None => 0,
    };
    let cents = major
        .checked_mul(100)
        .and_then(|n| n.checked_add(minor))
        .ok_or_else(invalid)?;
    if cents > MAX_CENTS {
        return Err("Each amount must be at most 1000000000.00 USD.".to_owned());
    }
    Ok(cents)
}

fn normalize(cents: i64, frequency: Frequency) -> Result<i64, String> {
    let (numerator, denominator) = match frequency {
        Frequency::Weekly => (52, 12),
        Frequency::Biweekly => (26, 12),
        Frequency::Semimonthly => (2, 1),
        Frequency::Annual => (1, 12),
        Frequency::Monthly | Frequency::Irregular => (1, 1),
    };
    // Round each nonnegative line to cents, half upward, then sum rounded lines.
    cents
        .checked_mul(numerator)
        .and_then(|n| n.checked_add(denominator / 2))
        .map(|n| n / denominator)
        .ok_or_else(|| "The budget amount is too large.".to_owned())
}

pub fn calculate(lines: Vec<BudgetLine>) -> Result<(Vec<BudgetLine>, Summary), String> {
    if lines.len() > 100 {
        return Err("A budget can contain at most 100 lines.".to_owned());
    }
    let mut canonical = Vec::new();
    let mut monthly_amounts = Vec::new();
    let mut totals = [0_i64; 5];
    let mut missing = 0;
    let mut estimated = 0;
    for mut line in lines {
        line.label = line.label.trim().to_owned();
        if line.label.is_empty()
            || line.label.chars().count() > 120
            || line.label.chars().any(char::is_control)
        {
            return Err(
                "Each budget line needs a label of 1–120 characters without control characters."
                    .to_owned(),
            );
        }
        line.estimated |= line.frequency == Frequency::Irregular;
        let amount = match line.amount.as_deref() {
            None => {
                missing += 1;
                None
            }
            Some(value) => {
                let cents = parse_money(value.trim())?;
                line.amount = Some(money(cents));
                if line.estimated {
                    estimated += 1;
                }
                let monthly = normalize(cents, line.frequency)?;
                let index = match line.kind {
                    Kind::Income => 0,
                    Kind::Necessary => 1,
                    Kind::Discretionary => 2,
                    Kind::Savings => 3,
                    Kind::DebtPayment => 4,
                };
                totals[index] = totals[index]
                    .checked_add(monthly)
                    .ok_or("The budget total is too large.")?;
                Some(money(monthly))
            }
        };
        canonical.push(line);
        monthly_amounts.push(amount);
    }
    let [income, necessary, discretionary, savings, debt] = totals;
    let spending = necessary + discretionary;
    let surplus = income - spending;
    let available = surplus - savings;
    let unallocated = available - debt;
    let mut warnings = Vec::new();
    if !canonical
        .iter()
        .any(|line| line.kind == Kind::Income && line.amount.is_some())
    {
        warnings.push("No income amount entered; this plan is incomplete.".to_owned());
    }
    if !canonical
        .iter()
        .any(|line| line.kind == Kind::Necessary && line.amount.is_some())
    {
        warnings.push(
            "No necessary-expense amount entered; check that essential costs are included."
                .to_owned(),
        );
    }
    if missing > 0 {
        warnings
            .push("Missing amounts are excluded from totals; the plan is incomplete.".to_owned());
    }
    if estimated > 0 {
        warnings.push(
            "Totals include estimates; irregular amounts are counselor-entered monthly estimates."
                .to_owned(),
        );
    }
    if unallocated < 0 {
        warnings
            .push("Planned spending and allocations exceed income; review the deficit.".to_owned());
    }
    Ok((
        canonical,
        Summary {
            monthly_income: money(income),
            necessary_expenses: money(necessary),
            discretionary_expenses: money(discretionary),
            total_spending: money(spending),
            surplus_before_allocations: money(surplus),
            proposed_savings: money(savings),
            available_for_debt: money(available),
            planned_debt_payments: money(debt),
            unallocated: money(unallocated),
            monthly_amounts,
            missing_count: missing,
            estimated_count: estimated,
            warnings,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(kind: Kind, value: &str, frequency: Frequency) -> BudgetLine {
        BudgetLine {
            label: "Fictional planning amount".to_owned(),
            kind,
            amount: Some(value.to_owned()),
            frequency,
            estimated: false,
        }
    }
    #[test]
    fn independently_checked_frequency_and_cent_rounding_examples() {
        // $600 weekly = $31,200/year = $2,600/month; $1,500 every two
        // weeks = $39,000/year = $3,250/month, unlike twice-monthly $3,000.
        for (input, frequency, expected) in [
            ("600", Frequency::Weekly, "2600.00"),
            ("1500", Frequency::Biweekly, "3250.00"),
            ("1500", Frequency::Semimonthly, "3000.00"),
            ("123.45", Frequency::Monthly, "123.45"),
            ("1200", Frequency::Annual, "100.00"),
            ("0.06", Frequency::Annual, "0.01"),
            ("0.03", Frequency::Biweekly, "0.07"),
            ("0.01", Frequency::Weekly, "0.04"),
            ("999.99", Frequency::Weekly, "4333.29"),
            ("0", Frequency::Monthly, "0.00"),
        ] {
            let (_, summary) = calculate(vec![line(Kind::Income, input, frequency)]).unwrap();
            assert_eq!(summary.monthly_income, expected);
        }
        let (_, summary) =
            calculate(vec![line(Kind::Income, "0.03", Frequency::Biweekly); 2]).unwrap();
        assert_eq!(summary.monthly_income, "0.14"); // sum rounded lines, not rounded aggregate
    }
    #[test]
    fn independent_household_example_preserves_signed_deficits_and_allocations() {
        let (_, s) = calculate(vec![
            line(Kind::Income, "1500", Frequency::Biweekly),
            line(Kind::Necessary, "1200", Frequency::Monthly),
            line(Kind::Necessary, "100", Frequency::Weekly),
            line(Kind::Discretionary, "200", Frequency::Monthly),
            line(Kind::Savings, "300", Frequency::Monthly),
            line(Kind::DebtPayment, "400", Frequency::Monthly),
        ])
        .unwrap();
        assert_eq!(s.monthly_income, "3250.00");
        assert_eq!(s.necessary_expenses, "1633.33");
        assert_eq!(s.total_spending, "1833.33");
        assert_eq!(s.surplus_before_allocations, "1416.67");
        assert_eq!(s.available_for_debt, "1116.67");
        assert_eq!(s.unallocated, "716.67");
        let (_, deficit) = calculate(vec![
            line(Kind::Income, "100", Frequency::Monthly),
            line(Kind::Necessary, "150", Frequency::Monthly),
            line(Kind::Savings, "10", Frequency::Monthly),
            line(Kind::DebtPayment, "20", Frequency::Monthly),
        ])
        .unwrap();
        assert_eq!(deficit.surplus_before_allocations, "-50.00");
        assert_eq!(deficit.available_for_debt, "-60.00");
        assert_eq!(deficit.unallocated, "-80.00");
    }
    #[test]
    fn missing_estimated_and_zero_are_distinct() {
        let mut missing = line(Kind::Necessary, "1", Frequency::Monthly);
        missing.amount = None;
        let (lines, s) = calculate(vec![
            line(Kind::Income, "100", Frequency::Irregular),
            missing,
            line(Kind::Discretionary, "0", Frequency::Monthly),
        ])
        .unwrap();
        assert!(lines[0].estimated);
        assert_eq!(
            s.monthly_amounts,
            vec![Some("100.00".to_owned()), None, Some("0.00".to_owned())]
        );
        assert_eq!((s.missing_count, s.estimated_count), (1, 1));
        assert!(s.warnings.iter().any(|w| w.contains("incomplete")));
        assert!(calculate(vec![])
            .unwrap()
            .1
            .warnings
            .iter()
            .any(|w| w.contains("No income")));
    }
    #[test]
    fn malformed_precision_negative_and_overlarge_input_are_rejected() {
        for value in [
            "",
            "-1",
            "+1",
            "1e3",
            "NaN",
            "1,000",
            "$12",
            "1.001",
            ".1",
            "1.",
            "1.2.3",
            "١٢",
            "1000000000.01",
            "99999999999999999",
        ] {
            assert!(
                calculate(vec![line(Kind::Income, value, Frequency::Monthly)]).is_err(),
                "{value}"
            );
        }
        assert!(calculate(vec![line(Kind::Income, "1", Frequency::Monthly); 101]).is_err());
        let (_, s) = calculate(vec![
            line(Kind::Income, "1000000000", Frequency::Weekly);
            100
        ])
        .unwrap();
        assert_eq!(s.monthly_income, "433333333333.00");
        assert_eq!(money(i64::MIN), "-92233720368547758.08");
    }
}
