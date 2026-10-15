//! Live, read-only approved transaction assessments. No planning data or asset inference.
use crate::{
    budget::money,
    imports::{self, Kind, Snapshot, Status, Summary},
};
use chrono::{Datelike, Months, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub from_month: String,
    pub through_month: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Month {
    pub month: String,
    pub totals: Summary,
    pub operating_cashflow: String,
    pub income_scale: u16,
    pub spending_scale: u16,
    pub cashflow_scale: u16,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub label: String,
    pub spending: String,
    pub scale: u16,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub facts_metrics: crate::financial_facts::Metrics,
    pub client_id: String,
    pub revision: i64,
    pub period_start: String,
    pub period_end: String,
    pub calculated_on_utc: String,
    pub latest_approved_date: Option<String>,
    pub outdated: bool,
    pub totals: Summary,
    pub operating_cashflow: String,
    pub months: Vec<Month>,
    pub categories: Vec<Category>,
    pub undated_records: usize,
    pub reviewed_percent: Option<u16>,
    pub document_ids: Vec<String>,
    pub warnings: Vec<String>,
}
pub const LIVE_VIEW_WARNING: &str = "This live view is recalculated from current decisions; it is not a saved final report. Period choices are not saved. Review percentages measure decisions on dated imported rows, not statement coverage.";
fn month(value: &str) -> Result<NaiveDate, String> {
    if value.len() != 7 {
        return Err("Choose assessment months as YYYY-MM.".into());
    }
    imports::iso_date(&format!("{value}-01"))
}
// Generated canonical totals may exceed the per-transaction input limit.
fn total_cents(value: &str) -> Result<i64, String> {
    value
        .replace('.', "")
        .parse::<i64>()
        .map_err(|_| "Assessment total is invalid.".into())
}
fn scale(amount: i64, maximum: i64) -> u16 {
    if maximum == 0 {
        0
    } else {
        ((i128::from(amount.abs()) * 1000) / i128::from(maximum)) as u16
    }
}
pub fn period(input: &Input) -> Result<(NaiveDate, NaiveDate, i32), String> {
    let start = month(&input.from_month)?;
    let end_month = month(&input.through_month)?;
    let count = (end_month.year() - start.year()) * 12 + end_month.month() as i32
        - start.month() as i32
        + 1;
    if !(1..=24).contains(&count) {
        return Err("Choose an ordered assessment period of 1–24 complete calendar months.".into());
    }
    let end = end_month
        .checked_add_months(Months::new(1))
        .and_then(|d| d.pred_opt())
        .ok_or("Assessment period is invalid.")?;
    Ok((start, end, count))
}
pub fn assess(source: &Snapshot, input: Input, today: NaiveDate) -> Result<Assessment, String> {
    let (start, end, count) = period(&input)?;
    let mut dated = Vec::new();
    let mut undated_records = 0;
    for record in &source.records {
        match record.fields.date.as_deref() {
            Some(date) => {
                let date = imports::iso_date(date)?;
                if date >= start && date <= end {
                    dated.push(record);
                }
            }
            None => {
                undated_records += 1;
            }
        }
    }
    let mut totals = imports::summarize_selected(&dated, &source.accounts)?;
    totals.warnings[0] = "Approved imported activity within the displayed assessment period; statement coverage remains unverified.".into();
    let operating_cashflow =
        money(total_cents(&totals.income)? - total_cents(&totals.net_spending)?);
    let total_records =
        totals.approved + totals.rejected + totals.pending + totals.needs_clarification;
    let reviewed_percent = (total_records > 0)
        .then(|| ((totals.approved + totals.rejected) * 100 / total_records) as u16);
    let mut months = Vec::new();
    let (mut comparison_max, mut cashflow_max) = (0, 0);
    for offset in 0..count {
        let day = start
            .checked_add_months(Months::new(offset as u32))
            .ok_or("Assessment month is invalid.")?;
        let key = day.format("%Y-%m").to_string();
        let rows = dated
            .iter()
            .copied()
            .filter(|r| {
                r.fields
                    .date
                    .as_deref()
                    .is_some_and(|d| d.starts_with(&key))
            })
            .collect::<Vec<_>>();
        let mut summary = imports::summarize_selected(&rows, &source.accounts)?;
        summary.warnings[0] = "Approved imported activity for this calendar month; statement coverage remains unverified.".into();
        let income = total_cents(&summary.income)?;
        let spending = total_cents(&summary.spending)?;
        let cashflow = income - total_cents(&summary.net_spending)?;
        comparison_max = comparison_max.max(income).max(spending);
        cashflow_max = cashflow_max.max(cashflow.abs());
        months.push(Month {
            month: key,
            totals: summary,
            operating_cashflow: money(cashflow),
            income_scale: 0,
            spending_scale: 0,
            cashflow_scale: 0,
        });
    }
    for m in &mut months {
        m.income_scale = scale(total_cents(&m.totals.income)?, comparison_max);
        m.spending_scale = scale(total_cents(&m.totals.spending)?, comparison_max);
        m.cashflow_scale = scale(total_cents(&m.operating_cashflow)?, cashflow_max);
    }
    let mut categories = BTreeMap::<String, i64>::new();
    for row in dated.iter().filter(|r| r.status == Status::Approved) {
        let n = imports::signed_money(
            row.fields
                .amount
                .as_deref()
                .ok_or("Approved amount missing.")?,
        )?;
        if matches!(row.fields.kind, Kind::Expense | Kind::Fee)
            || (row.fields.kind == Kind::Interest && n < 0)
        {
            let label = if row.fields.category.is_empty() {
                "Unclassified category"
            } else {
                &row.fields.category
            };
            *categories.entry(label.to_owned()).or_default() -= n;
        }
    }
    let maximum = categories.values().copied().max().unwrap_or(0);
    let categories = categories
        .into_iter()
        .map(|(label, n)| Category {
            label,
            spending: money(n),
            scale: scale(n, maximum),
        })
        .collect();
    let latest_approved_date = dated
        .iter()
        .filter(|r| r.status == Status::Approved)
        .filter_map(|r| r.fields.date.clone())
        .max();
    let outdated = (today - end).num_days() > 90;
    let document_ids = dated
        .iter()
        .map(|r| r.document_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut warnings = vec![
        "Incomplete assessment: reviewed imported records do not establish that every account or statement is present. Zero means no approved activity in the imported records, not proof of no activity.".into(),
        "Income receipts include positive interest. Take-home pay and necessary/discretionary spending use separate, explicit classifications bound to exact approved source decisions.".into(),
        "Refunds reduce net spending and are shown separately from gross spending categories. Transfers and balance movements are excluded from operating cash flow, but included in account movement.".into(),
        "Dated counselor-verified balance facts and source classifications supply additional metrics only when sufficient. Missing facts stay unavailable. Manual plans remain separate estimates.".into(),
        LIVE_VIEW_WARNING.into(),
    ];
    if totals.pending + totals.needs_clarification > 0 {
        warnings.push("Undecided records within this period are excluded from the amounts.".into());
    }
    if undated_records > 0 {
        warnings.push("Undated records cannot be assigned to a period and are excluded; review them in the queue before treating this assessment as complete.".into());
    }
    if outdated {
        warnings.push("Outdated period: it ended more than 90 days before the calculation date (UTC). Historical amounts are retained; current finances may differ.".into());
    }
    if dated.iter().any(|r| {
        r.fields
            .date
            .as_deref()
            .is_some_and(|d| d > today.to_string().as_str())
    }) {
        warnings.push("The selected records include future dates; verify the dates before using them as historical facts.".into());
    }
    if source.documents.iter().any(|d| {
        document_ids.contains(&d.id)
            && (!d.warnings.is_empty()
                || d.source_difference.as_deref().is_some_and(|v| v != "0.00")
                || d.approved_difference
                    .as_deref()
                    .is_some_and(|v| v != "0.00")
                || d.balance_reconciliation
                    .as_ref()
                    .is_some_and(|r| r.conflicts()))
    }) {
        warnings.push("Selected source documents have statement warnings or unresolved checks. Review source documents and running-balance checks; approved amounts alone do not resolve these issues.".into());
    }
    Ok(Assessment {
        facts_metrics: crate::financial_facts::metrics(
            source,
            start,
            end,
            today,
            i64::from(count),
        )?,
        client_id: source.client_id.clone(),
        revision: source.revision,
        period_start: start.to_string(),
        period_end: end.to_string(),
        calculated_on_utc: today.to_string(),
        latest_approved_date,
        outdated,
        totals,
        operating_cashflow,
        months,
        categories,
        undated_records,
        reviewed_percent,
        document_ids,
        warnings,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use imports::{Account, Record};
    pub(crate) fn fixture() -> Snapshot {
        let csv = "Date,Description,Amount,Category,Type\n2026-09-01,Wages,1000.01,Pay,income\n2026-09-02,Rent,-300.02,Housing,expense\n2026-09-03,Refund,20.03,Housing,refund\n2026-09-04,Fee,-0.01,Fees,fee\n2026-09-05,Interest,-3.33,Fees,interest\n2026-09-06,Interest,2.22,Interest,interest\n2026-09-07,Principal,-200,Loan,balance movement\n2026-09-08,Transfer,-50,Internal,transfer\n2026-09-08,Transfer,50,Internal,transfer\n2026-09-09,Pending,9999,Pay,income\n2026-09-10,Rejected,8888,Pay,income\n2026-09-11,Clarify,-7777,Housing,expense\n2026-10-01,Wages,200.01,Pay,income\n2026-10-02,Food,-0.02,Food,expense\n2026-08-01,Outside period,777,Pay,income\n2026-11-01,Outside period,-888,Housing,expense\n";
        let raw = imports::extract(
            &imports::read_csv(csv.as_bytes()).unwrap(),
            &imports::tests::mapping(),
        )
        .unwrap();
        let mut records = raw
            .into_iter()
            .enumerate()
            .map(|(i, raw)| Record {
                id: format!("r{i}"),
                document_id: "doc".into(),
                account_id: "checking".into(),
                fields: raw.extracted.clone(),
                raw,
                status: match i {
                    9 => Status::Pending,
                    10 => Status::Rejected,
                    11 => Status::NeedsClarification,
                    _ => Status::Approved,
                },
                linked_id: None,
                reason: String::new(),
                warnings: vec![],
                duplicate_ids: vec![],
                bulk_eligible: false,
            })
            .collect::<Vec<_>>();
        let mut undated = records.pop().unwrap();
        undated.fields.date = None;
        undated.status = Status::NeedsClarification;
        records.push(undated);
        let accounts = vec![Account {
            id: "checking".into(),
            label: "Synthetic checking".into(),
            kind: "checking".into(),
        }];
        let summary = imports::summarize(&records, &accounts).unwrap();
        Snapshot {
            financial_facts: crate::financial_facts::Facts::default(),
            client_id: "synthetic".into(),
            revision: 42,
            accounts,
            records,
            documents: vec![],
            history: vec![],
            merchant_groups: vec![],
            recurring_history: vec![],
            summary,
        }
    }
    fn input(a: &str, b: &str) -> Input {
        Input {
            from_month: a.into(),
            through_month: b.into(),
        }
    }
    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 14).unwrap()
    }
    #[test]
    fn exact_period_categories_movements_review_and_independent_months() {
        let s = assess(&fixture(), input("2026-09", "2026-10"), today()).unwrap();
        assert_eq!(
            (s.period_start.as_str(), s.period_end.as_str()),
            ("2026-09-01", "2026-10-31")
        );
        assert_eq!(s.totals.income, "1202.24");
        assert_eq!(s.totals.spending, "303.38");
        assert_eq!(s.totals.refunds, "20.03");
        assert_eq!(s.totals.net_spending, "283.35");
        assert_eq!(s.operating_cashflow, "918.89");
        assert_eq!(s.totals.net_cash, "718.89");
        assert_eq!(
            (
                s.totals.approved,
                s.totals.rejected,
                s.totals.pending,
                s.totals.needs_clarification
            ),
            (11, 1, 1, 1)
        );
        assert_eq!(s.reviewed_percent, Some(85));
        assert_eq!(s.undated_records, 1);
        assert_eq!(s.months[0].totals.income, "1002.23");
        assert_eq!(s.months[0].totals.spending, "303.36");
        assert_eq!(s.months[0].operating_cashflow, "718.90");
        assert_eq!(s.months[1].operating_cashflow, "199.99");
        assert_eq!(
            s.categories
                .iter()
                .map(|c| (c.label.as_str(), c.spending.as_str()))
                .collect::<Vec<_>>(),
            vec![("Fees", "3.34"), ("Food", "0.02"), ("Housing", "300.02")]
        );
        assert_eq!(s.categories[2].scale, 1000);
        assert_eq!(s.months[0].income_scale, 1000);
        assert_eq!(s.latest_approved_date.as_deref(), Some("2026-10-02"));
        assert!(!s.outdated);
        assert_eq!(s.document_ids, vec!["doc"]);
    }
    #[test]
    fn reversals_and_corrected_dates_recompute_without_cached_facts() {
        let mut f = fixture();
        f.records[1].status = Status::Rejected;
        f.records[12].fields.date = Some("2026-08-31".into());
        f.revision += 1;
        let s = assess(&f, input("2026-09", "2026-10"), today()).unwrap();
        assert_eq!(s.totals.income, "1002.23");
        assert_eq!(s.totals.spending, "3.36");
        assert_eq!(s.operating_cashflow, "1018.90");
        assert_eq!(s.totals.net_spending, "-16.67");
        assert!(s.categories.iter().all(|c| c.label != "Housing"));
        assert_eq!(s.revision, 43);
    }
    #[test]
    fn empty_months_negative_cashflow_and_exact_zero_scales() {
        let s = assess(&fixture(), input("2026-06", "2026-06"), today()).unwrap();
        assert_eq!(s.reviewed_percent, None);
        assert!(s.categories.is_empty());
        assert!(s.outdated);
        assert_eq!(s.months[0].totals.income, "0.00");
        assert_eq!(s.months[0].cashflow_scale, 0);
        let mut f = fixture();
        f.records
            .retain(|r| r.fields.kind == Kind::Expense && r.status == Status::Approved);
        let s = assess(&f, input("2026-09", "2026-10"), today()).unwrap();
        assert_eq!(s.months[0].operating_cashflow, "-300.02");
        assert_eq!(s.months[0].cashflow_scale, 1000);
        assert!(s.warnings.iter().any(|w| w.contains("Incomplete")));
    }
    #[test]
    fn bounds_leap_year_inclusive_dates_future_records_and_invalid_approved_data() {
        let f = fixture();
        for (a, b) in [
            ("2026-9", "2026-10"),
            ("2026-13", "2026-13"),
            ("2026-11", "2026-10"),
            ("2024-01", "2026-01"),
            ("1899-12", "1900-01"),
        ] {
            assert!(assess(&f, input(a, b), today()).is_err());
        }
        assert_eq!(
            assess(&f, input("2024-02", "2024-02"), today())
                .unwrap()
                .period_end,
            "2024-02-29"
        );
        assert_eq!(
            assess(&f, input("2025-01", "2026-12"), today())
                .unwrap()
                .months
                .len(),
            24
        );
        let mut f = fixture();
        f.records[0].fields.date = Some("2026-10-31".into());
        assert!(assess(&f, input("2026-10", "2026-10"), today())
            .unwrap()
            .warnings
            .iter()
            .any(|w| w.contains("future dates")));
        let mut large = fixture();
        large
            .records
            .retain(|r| r.status == Status::Approved && r.fields.kind == Kind::Income);
        for r in &mut large.records {
            r.fields.amount = Some("1000000000.00".into());
        }
        let large = assess(&large, input("2026-09", "2026-10"), today()).unwrap();
        assert_eq!(large.totals.income, "2000000000.00");
        assert_eq!(large.operating_cashflow, "2000000000.00");
        assert_eq!(large.months[0].income_scale, 1000);
        f.records[0].fields.amount = None;
        assert!(assess(&f, input("2026-10", "2026-10"), today()).is_err());
    }
}
