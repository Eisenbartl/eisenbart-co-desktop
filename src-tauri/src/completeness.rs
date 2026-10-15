//! Conservative diagnostics over known immutable sources; never certifies household completeness.
use crate::{
    dashboard,
    imports::{self, Snapshot, Status},
};
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DateRange {
    pub start: String,
    pub end: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountCoverage {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub document_ids: Vec<String>,
    pub covered_days: usize,
    pub total_days: usize,
    pub gaps: Vec<DateRange>,
    pub overlaps: Vec<DateRange>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCheck {
    pub id: String,
    pub file_name: String,
    pub account_id: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub pending: usize,
    pub clarification: usize,
    pub rejected: usize,
    pub undated: usize,
    pub outside_declared_period: usize,
    pub issues: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub client_id: String,
    pub revision: i64,
    pub period_start: String,
    pub period_end: String,
    pub approved: usize,
    pub rejected: usize,
    pub pending: usize,
    pub clarification: usize,
    pub undated: usize,
    pub out_of_period: usize,
    pub assignment_issues: usize,
    pub accounts: Vec<AccountCoverage>,
    pub sources: Vec<SourceCheck>,
    pub blocking_reasons: Vec<String>,
    pub final_eligible: bool,
}
fn declared(d: &imports::Document) -> Option<(NaiveDate, NaiveDate)> {
    let start = imports::iso_date(d.input.period_start.as_deref()?).ok()?;
    let end = imports::iso_date(d.input.period_end.as_deref()?).ok()?;
    (start <= end).then_some((start, end))
}
fn ranges(days: &[usize], start: NaiveDate, matches: impl Fn(usize) -> bool) -> Vec<DateRange> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < days.len() {
        if !matches(days[i]) {
            i += 1;
            continue;
        }
        let first = i;
        while i + 1 < days.len() && matches(days[i + 1]) {
            i += 1;
        }
        result.push(DateRange {
            start: (start + chrono::Duration::days(first as i64)).to_string(),
            end: (start + chrono::Duration::days(i as i64)).to_string(),
        });
        i += 1;
    }
    result
}
pub fn review(
    source: &Snapshot,
    input: dashboard::Input,
    today: NaiveDate,
) -> Result<Review, String> {
    let a = dashboard::assess(source, input, today)?;
    from_assessment(source, &a)
}
fn from_assessment(source: &Snapshot, a: &dashboard::Assessment) -> Result<Review, String> {
    let start = imports::iso_date(&a.period_start)?;
    let end = imports::iso_date(&a.period_end)?;
    let total_days = (end - start).num_days() as usize + 1;
    if total_days > 732 {
        return Err("Coverage period exceeds the supported limit.".into());
    }
    let mut accounts = Vec::new();
    for account in &source.accounts {
        let mut days = vec![0; total_days];
        let mut document_ids = Vec::new();
        for d in source
            .documents
            .iter()
            .filter(|d| d.account_id == account.id)
        {
            if let Some((s, e)) = declared(d) {
                let (s, e) = (s.max(start), e.min(end));
                if s <= e {
                    document_ids.push(d.id.clone());
                    for day in
                        &mut days[(s - start).num_days() as usize..=(e - start).num_days() as usize]
                    {
                        *day += 1;
                    }
                }
            }
        }
        accounts.push(AccountCoverage {
            id: account.id.clone(),
            label: account.label.clone(),
            kind: account.kind.clone(),
            document_ids,
            covered_days: days.iter().filter(|n| **n > 0).count(),
            total_days,
            gaps: ranges(&days, start, |n| n == 0),
            overlaps: ranges(&days, start, |n| n > 1),
        });
    }
    let assignment_issues = source
        .documents
        .iter()
        .filter(|d| !source.accounts.iter().any(|a| a.id == d.account_id))
        .count()
        + source
            .records
            .iter()
            .filter(|r| {
                !source
                    .documents
                    .iter()
                    .any(|d| d.id == r.document_id && d.account_id == r.account_id)
                    || !source.accounts.iter().any(|a| a.id == r.account_id)
            })
            .count();
    let mut sources = Vec::new();
    for d in &source.documents {
        let rows = source
            .records
            .iter()
            .filter(|r| r.document_id == d.id)
            .collect::<Vec<_>>();
        let period = declared(d);
        let applicable = period.is_none_or(|(s, e)| s <= end && e >= start)
            || rows.iter().any(|r| {
                r.fields.date.as_deref().is_none_or(|date| {
                    date >= a.period_start.as_str() && date <= a.period_end.as_str()
                })
            });
        if !applicable {
            continue;
        }
        let mut issues = Vec::new();
        if period.is_none() {
            issues.push("Statement start/end dates are unknown; no coverage is assumed.".into());
        }
        match d.source_difference.as_deref() {
            None=>issues.push("Whole-source closing comparison is unavailable; check entered balances and source rows.".into()),
            Some("0.00")=>{}, Some(_)=>issues.push("Whole-source closing arithmetic has a discrepancy.".into()),
        }
        match d.approved_difference.as_deref() {
            None=>issues.push("Approved closing comparison is unavailable; resolve row decisions and missing balances.".into()),
            Some("0.00")=>{}, Some(_)=>issues.push("Approved closing arithmetic has a discrepancy.".into()),
        }
        if d.balance_reconciliation
            .as_ref()
            .is_some_and(|r| r.conflicts())
        {
            issues.push(
                "Running-balance reconciliation has differences or sequence conflicts.".into(),
            );
        }
        let outside_declared_period =
            rows.iter()
                .filter(|r| {
                    period.is_some_and(|(s, e)| {
                        r.fields.date.as_deref().is_some_and(|v| {
                            v < s.to_string().as_str() || v > e.to_string().as_str()
                        })
                    })
                })
                .count();
        if outside_declared_period > 0 {
            issues.push("Some source rows fall outside its declared statement period.".into());
        }
        if !d.warnings.is_empty() {
            issues.push("Source warnings remain available in Documents / Working tools; review them individually.".into());
        }
        sources.push(SourceCheck {
            id: d.id.clone(),
            file_name: d.file_name.clone(),
            account_id: d.account_id.clone(),
            period_start: d.input.period_start.clone(),
            period_end: d.input.period_end.clone(),
            pending: rows.iter().filter(|r| r.status == Status::Pending).count(),
            clarification: rows
                .iter()
                .filter(|r| r.status == Status::NeedsClarification)
                .count(),
            rejected: rows.iter().filter(|r| r.status == Status::Rejected).count(),
            undated: rows.iter().filter(|r| r.fields.date.is_none()).count(),
            outside_declared_period,
            issues,
        });
    }
    let mut blocking_reasons=vec!["The complete household account and obligation inventory has not been verified. Known checking/savings accounts and entered statement dates cannot establish complete financial coverage.".into()];
    if accounts.is_empty() {
        blocking_reasons.push("No statement accounts are recorded.".into());
    }
    if accounts.iter().any(|a| !a.gaps.is_empty()) {
        blocking_reasons.push("Known accounts have dates without declared statement coverage. Review applicability and obtain missing statements.".into());
    }
    if accounts.iter().any(|a| !a.overlaps.is_empty()) {
        blocking_reasons.push("Declared statement periods overlap; verify source selection and possible duplicated activity.".into());
    }
    if assignment_issues > 0 {
        blocking_reasons.push("Source/account assignments are inconsistent.".into());
    }
    if a.totals.pending + a.totals.needs_clarification + a.undated_records > 0 {
        blocking_reasons.push("Pending, clarification or undated records need review; they do not supply approved period totals.".into());
    }
    if sources
        .iter()
        .any(|s| !s.issues.is_empty() || s.pending + s.clarification + s.undated > 0)
    {
        blocking_reasons.push("Applicable sources have unresolved review or arithmetic checks, including source-wide exclusions.".into());
    }
    if a.outdated {
        blocking_reasons.push(
            "The captured assessment period was already outdated at calculation time.".into(),
        );
    }
    if a.period_end > a.calculated_on_utc {
        blocking_reasons.push(
            "The assessment period includes future dates relative to its calculation date.".into(),
        );
    }
    Ok(Review {
        client_id: source.client_id.clone(),
        revision: source.revision,
        period_start: a.period_start.clone(),
        period_end: a.period_end.clone(),
        approved: a.totals.approved,
        rejected: a.totals.rejected,
        pending: a.totals.pending,
        clarification: a.totals.needs_clarification,
        undated: a.undated_records,
        out_of_period: source.records.len()
            - a.totals.approved
            - a.totals.rejected
            - a.totals.pending
            - a.totals.needs_clarification
            - a.undated_records,
        assignment_issues,
        accounts,
        sources,
        blocking_reasons,
        final_eligible: false,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub label: String,
    pub state: String,
    pub detail: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportValidation {
    pub version: String,
    pub final_eligible: bool,
    pub coverage: Review,
    pub checks: Vec<Check>,
}
pub fn report(
    p: &crate::assessments::Payload,
    needs_review: bool,
) -> Result<ReportValidation, String> {
    let coverage = from_assessment(&p.basis, &p.assessment)?;
    let item = |label: &str, state: &str, detail: &str| Check {
        label: label.into(),
        state: state.into(),
        detail: detail.into(),
    };
    let checks=vec![
        item("Client and source identity","Checked","Stored client, organization, counselor and source bindings passed authenticated validation. Confirm the intended person before sharing."),
        item("Calculation version","Checked",&p.calculator),
        item("Current supporting context",if needs_review{"Needs review"}else{"Checked"},if needs_review{"Source, profile, firm or selected planning context changed; original figures are preserved."}else{"Recorded context matches the last authenticated check. This does not establish completeness."}),
        item("Assessment period",if p.assessment.period_end>p.assessment.calculated_on_utc {"Blocked"}else if p.assessment.outdated {"Needs review"}else{"Checked"},"The exact captured period is shown below. Future dates block historical use; age is evaluated as of the original calculation, not the current viewing date."),
        item("Verified balances and classifications",if p.assessment.facts_metrics.stale_classifications>0 {"Blocked"}else if !p.assessment.facts_metrics.balance_complete || !p.assessment.facts_metrics.balance_aligned || p.assessment.facts_metrics.balance_outdated || p.assessment.facts_metrics.unclassified_spending!="0.00" || p.assessment.facts_metrics.unclassified_income!="0.00" {"Unknown"}else{"Needs review"},"Inspect the captured dated balance inventory and income/spending classifications. Missing, stale, unaligned or unclassified evidence cannot establish a complete financial position."),
        item("Approved data",if coverage.pending+coverage.clarification+coverage.undated>0{"Blocked"}else{"Checked"},"Only approved, dated in-period rows supply financial totals. Rejected and out-of-period rows remain excluded; rejecting a row does not verify a complete statement."),
        item("Account assignments",if coverage.assignment_issues>0{"Blocked"}else{"Checked"},"Imported source/account links are checked within this client; missing real-world accounts remain unknown."),
        item("Document review and coverage",if coverage.accounts.is_empty()||coverage.accounts.iter().any(|a|!a.gaps.is_empty()||!a.overlaps.is_empty())||coverage.sources.iter().any(|s|!s.issues.is_empty()||s.pending+s.clarification+s.undated>0){"Blocked"}else{"Unknown"},"Declared date coverage and arithmetic agreement do not verify the institution or prove every row was imported."),
        item("Planning assumptions and scenario",if p.planning.is_some(){"Needs review"}else{"Unknown"},if p.selected_scenario.is_some(){"A named saved combined scenario and its assumptions were captured. Confirm suitability and agreement; projections remain estimates."}else if p.planning.is_some(){"Saved entered plans and explicit assumptions were captured; no named combined scenario was selected. Confirm suitability and agreement."}else{"No planning scenario was selected. Its applicability and assumptions remain unconfirmed."}),
        item("Financial completeness","Blocked","Complete household inventory, source completeness and final professional review are not yet recorded. This assessment cannot be finalized."),
    ];
    Ok(ReportValidation {
        version: "coverage-review-v1".into(),
        final_eligible: false,
        coverage,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document(id: &str, start: Option<&str>, end: Option<&str>) -> imports::Document {
        imports::Document {
            id: id.into(),
            account_id: "checking".into(),
            file_name: format!("{id}.csv"),
            sha256: "a".repeat(64),
            byte_count: 100,
            imported_at: "2026-10-14T00:00:00Z".into(),
            source_retained: false,
            input: imports::DocumentInput {
                balance_order: None,
                pdf_selection: None,
                account_id: Some("checking".into()),
                account_label: "Synthetic checking".into(),
                account_kind: "checking".into(),
                period_start: start.map(str::to_owned),
                period_end: end.map(str::to_owned),
                opening_balance: None,
                closing_balance: None,
                retain_source: false,
                mapping: imports::tests::mapping(),
            },
            row_count: 0,
            source_expected_closing: None,
            source_difference: Some("0.00".into()),
            approved_difference: Some("0.00".into()),
            warnings: vec![],
            balance_history: vec![],
            balance_reconciliation: None,
        }
    }
    fn check(s: &Snapshot, from: &str, to: &str) -> Review {
        review(
            s,
            dashboard::Input {
                from_month: from.into(),
                through_month: to.into(),
            },
            NaiveDate::from_ymd_opt(2026, 10, 14).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn unions_clipped_dates_and_reports_interior_gaps_overlaps_and_empty_accounts() {
        let mut s = dashboard::tests::fixture();
        s.records.clear();
        s.documents = vec![
            document("a", Some("2026-08-01"), Some("2026-09-10")),
            document("b", Some("2026-09-10"), Some("2026-09-20")),
            document("c", Some("2026-09-25"), Some("2026-10-31")),
            document("unknown", None, Some("2026-09-30")),
        ];
        s.accounts.push(imports::Account {
            id: "no-source".into(),
            label: "Archived or standalone account".into(),
            kind: "savings".into(),
        });
        let r = check(&s, "2026-09", "2026-09");
        let a = &r.accounts[0];
        assert_eq!((a.covered_days, a.total_days), (26, 30));
        assert_eq!(a.document_ids, vec!["a", "b", "c"]);
        assert_eq!(
            (&*a.gaps[0].start, &*a.gaps[0].end),
            ("2026-09-21", "2026-09-24")
        );
        assert_eq!(
            (&*a.overlaps[0].start, &*a.overlaps[0].end),
            ("2026-09-10", "2026-09-10")
        );
        assert_eq!(r.accounts[1].covered_days, 0);
        assert_eq!(r.accounts[1].gaps[0].end, "2026-09-30");
        assert!(
            r.sources.iter().find(|s| s.id == "unknown").unwrap().issues[0].contains("unknown")
        );
        assert!(!r.final_eligible);
        let leap = check(&s, "2024-02", "2024-02");
        assert_eq!(leap.accounts[0].total_days, 29);
    }
    #[test]
    fn decisions_do_not_imply_coverage_and_unknowns_discrepancies_and_assignments_remain_explicit()
    {
        let mut s = dashboard::tests::fixture();
        s.documents = vec![document("doc", Some("2026-09-01"), Some("2026-09-30"))];
        let r = check(&s, "2026-09", "2026-09");
        assert_eq!(
            (
                r.approved,
                r.rejected,
                r.pending,
                r.clarification,
                r.undated,
                r.out_of_period
            ),
            (9, 1, 1, 1, 1, 3)
        );
        assert_eq!(r.accounts[0].covered_days, 30);
        assert!(!r.final_eligible);
        assert_eq!(r.assignment_issues, 0);
        assert_eq!(r.sources[0].outside_declared_period, 3);
        assert_eq!(r.sources[0].rejected, 1);
        let before = serde_json::to_string(&s).unwrap();
        check(&s, "2026-09", "2026-09");
        assert_eq!(serde_json::to_string(&s).unwrap(), before);
        s.documents[0].source_difference = None;
        s.documents[0].approved_difference = Some("0.01".into());
        s.records[0].account_id = "foreign".into();
        let r = check(&s, "2026-09", "2026-09");
        assert_eq!(r.assignment_issues, 1);
        assert!(r.sources[0]
            .issues
            .iter()
            .any(|i| i.contains("unavailable")));
        assert!(r.sources[0]
            .issues
            .iter()
            .any(|i| i.contains("discrepancy")));
        s.records.clear();
        let r = check(&s, "2026-09", "2026-09");
        assert_eq!(r.approved, 0);
        assert!(!r.final_eligible);
        for (a, b) in [
            ("2026-00", "2026-09"),
            ("2026-09", "2026-08"),
            ("2024-01", "2026-01"),
        ] {
            assert!(review(
                &s,
                dashboard::Input {
                    from_month: a.into(),
                    through_month: b.into()
                },
                NaiveDate::from_ymd_opt(2026, 10, 14).unwrap()
            )
            .is_err());
        }
    }
}
