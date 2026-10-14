//! Advisory historical patterns: approved outflows only, never planning writes.
use crate::budget::money;
use crate::imports::{iso_date, signed_money, Fields, History, Kind, Record, Status};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Frequency {
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Annual,
    Irregular,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Confirmed,
    Dismissed,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub record_id: String,
    pub approval_id: String,
    pub document_id: String,
    pub account_id: String,
    pub page: Option<u32>,
    pub line: u64,
    pub fields: Fields,
    pub duplicate_ids: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Review {
    #[serde(default)]
    pub invalidated: bool,
    pub id: String,
    pub group_id: String,
    pub evidence_hash: String,
    pub label: String,
    pub decision: Decision,
    pub frequency: Option<Frequency>,
    pub subscription: bool,
    pub reason: String,
    pub reviewed_at: String,
    pub counselor_name: String,
    pub evidence: Vec<Evidence>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewInput {
    pub expected_revision: i64,
    pub group_id: String,
    pub evidence_hash: String,
    pub label: String,
    pub decision: Decision,
    pub frequency: Option<Frequency>,
    pub subscription: bool,
    pub reason: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub merchant: String,
    pub uses_description: bool,
    pub account_id: String,
    pub kind: Kind,
    pub evidence_hash: String,
    pub evidence: Vec<Evidence>,
    pub total_paid: String,
    pub smallest_payment: String,
    pub largest_payment: String,
    pub suggested_frequency: Option<Frequency>,
    pub can_confirm: bool,
    pub warnings: Vec<String>,
    pub state: String,
    pub review: Option<Review>,
}
fn normalized(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn identity(e: &Evidence) -> Result<(String, String, bool), String> {
    let fallback = e.fields.merchant.trim().is_empty();
    let name = if fallback {
        &e.fields.description
    } else {
        &e.fields.merchant
    };
    let key = serde_json::to_vec(&(&e.account_id, &e.fields.kind, fallback, normalized(name)))
        .map_err(|_| "Cannot group payments.")?;
    Ok((
        format!("{:x}", Sha256::digest(key)),
        name.trim().into(),
        fallback,
    ))
}
pub fn evidence_hash(e: &[Evidence]) -> Result<String, String> {
    let bytes = serde_json::to_vec(e).map_err(|_| "Cannot verify payment evidence.")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn frequency(dates: &[NaiveDate]) -> Option<Frequency> {
    if dates.len() < 3 || dates.windows(2).any(|p| p[0] == p[1]) {
        return None;
    }
    let gaps: Vec<i64> = dates.windows(2).map(|p| (p[1] - p[0]).num_days()).collect();
    // Monthly schedules are tested by calendar months, never 30-day arithmetic.
    for (months, f) in [
        (1, Frequency::Monthly),
        (3, Frequency::Quarterly),
        (12, Frequency::Annual),
    ] {
        if dates.windows(2).all(|p| {
            let difference =
                (p[1].year() - p[0].year()) * 12 + p[1].month() as i32 - p[0].month() as i32;
            difference == months && (p[1].day() as i32 - p[0].day() as i32).abs() <= 3
        }) {
            return Some(f);
        }
    }
    for (days, tolerance, f) in [(7, 1, Frequency::Weekly), (14, 2, Frequency::Fortnightly)] {
        if gaps.iter().all(|v| (v - days).abs() <= tolerance) {
            return Some(f);
        }
    }
    None
}
fn group(mut evidence: Vec<Evidence>) -> Result<Group, String> {
    evidence.sort_by(|a, b| (&a.fields.date, &a.record_id).cmp(&(&b.fields.date, &b.record_id)));
    let first = evidence.first().ok_or("No payment evidence.")?;
    let (id, merchant, uses_description) = identity(first)?;
    let mut dates = Vec::new();
    let mut payments = Vec::new();
    for e in &evidence {
        if identity(e)?.0 != id || !matches!(e.fields.kind, Kind::Expense | Kind::Fee) {
            return Err("Invalid merchant evidence.".into());
        }
        let amount = signed_money(
            e.fields
                .amount
                .as_deref()
                .ok_or("Missing payment amount.")?,
        )?;
        if amount >= 0 {
            return Err("Payment evidence must be an outflow.".into());
        }
        payments.push(-amount);
        dates.push(iso_date(
            e.fields.date.as_deref().ok_or("Missing payment date.")?,
        )?);
    }
    let duplicate = evidence.iter().any(|e| !e.duplicate_ids.is_empty());
    let distinct_dates = dates.windows(2).all(|p| p[0] != p[1]);
    let can_confirm = dates.len() >= 3 && distinct_dates && !duplicate && evidence.len() <= 500;
    let mut warnings = vec!["Historical evidence only. Missing statements, merchant identity, and whether payments will continue require counselor verification.".into()];
    if uses_description {
        warnings.push("No merchant was entered; grouping uses the exact normalized description. Correct the verified Merchant field to combine or separate payments.".into());
    }
    if duplicate || !distinct_dates {
        warnings.push("Possible duplicate or multiple same-day payments: resolve or separate them before confirming recurrence.".into());
    }
    if dates.len() < 3 {
        warnings.push("At least three approved payments on distinct dates are required to confirm recurrence.".into());
    }
    if evidence.len() > 500 {
        warnings.push("This group exceeds the 500-payment review limit; separate merchants before confirming.".into());
    }
    let smallest = *payments.iter().min().unwrap();
    let largest = *payments.iter().max().unwrap();
    if smallest != largest {
        warnings.push("Observed amounts vary; no fixed future charge is inferred.".into());
    }
    let total = payments
        .iter()
        .try_fold(0i64, |total, v| total.checked_add(*v))
        .ok_or("Payment total is too large.")?;
    Ok(Group {
        id,
        merchant,
        uses_description,
        account_id: first.account_id.clone(),
        kind: first.fields.kind.clone(),
        evidence_hash: evidence_hash(&evidence)?,
        total_paid: money(total),
        smallest_payment: money(smallest),
        largest_payment: money(largest),
        suggested_frequency: if can_confirm { frequency(&dates) } else { None },
        can_confirm,
        warnings,
        state: "Unreviewed".into(),
        review: None,
        evidence,
    })
}
pub fn analyze(
    records: &[Record],
    history: &[Review],
    decisions: &[History],
) -> Result<Vec<Group>, String> {
    let mut buckets: BTreeMap<String, Vec<Evidence>> = BTreeMap::new();
    for r in records.iter().filter(|r| {
        r.status == Status::Approved && matches!(r.fields.kind, Kind::Expense | Kind::Fee)
    }) {
        let mut duplicates = r.duplicate_ids.clone();
        duplicates.sort();
        let e = Evidence {
            record_id: r.id.clone(),
            approval_id: decisions
                .iter()
                .rev()
                .find(|h| h.record_id == r.id && h.to_status == Status::Approved)
                .ok_or("Approved payment has no review provenance.")?
                .id
                .clone(),
            document_id: r.document_id.clone(),
            account_id: r.account_id.clone(),
            page: r.raw.page,
            line: r.raw.line,
            fields: r.fields.clone(),
            duplicate_ids: duplicates,
        };
        buckets.entry(identity(&e)?.0).or_default().push(e);
    }
    let mut groups = Vec::new();
    for evidence in buckets.into_values() {
        let mut g = group(evidence)?;
        if let Some(r) = history.iter().rev().find(|r| r.group_id == g.id) {
            g.state = if r.invalidated || r.evidence_hash != g.evidence_hash {
                "Needs review"
            } else if r.decision == Decision::Confirmed {
                "Confirmed"
            } else {
                "Dismissed"
            }
            .into();
            g.review = Some(r.clone());
        }
        groups.push(g);
    }
    groups.sort_by(|a, b| {
        (&a.merchant, &a.account_id, &a.id).cmp(&(&b.merchant, &b.account_id, &b.id))
    });
    Ok(groups)
}
pub fn validate_review(
    r: &Review,
    records: &[Record],
    decisions: &[History],
) -> Result<(), String> {
    if r.evidence.is_empty()
        || r.evidence.len() > 500
        || r.label.trim().is_empty()
        || r.label.chars().count() > 120
        || r.reason.trim().is_empty()
        || r.reason.chars().count() > 500
        || r.id.is_empty()
        || r.counselor_name.is_empty()
        || chrono::DateTime::parse_from_rfc3339(&r.reviewed_at).is_err()
    {
        return Err("Invalid payment review history.".into());
    }
    let mut ids = std::collections::HashSet::new();
    for e in &r.evidence {
        if !decisions.iter().any(|h| {
            h.id == e.approval_id
                && h.record_id == e.record_id
                && h.to_status == Status::Approved
                && h.fields == e.fields
        }) || !ids.insert(&e.record_id)
            || !records.iter().any(|v| {
                v.id == e.record_id
                    && v.document_id == e.document_id
                    && v.account_id == e.account_id
                    && v.raw.page == e.page
                    && v.raw.line == e.line
            })
            || crate::imports::approval_error(&e.fields).is_some()
        {
            return Err("Invalid payment evidence provenance.".into());
        }
    }
    let g = group(r.evidence.clone())?;
    if g.id != r.group_id
        || g.evidence_hash != r.evidence_hash
        || (r.decision == Decision::Confirmed && (!g.can_confirm || r.frequency.is_none()))
        || (r.decision == Decision::Dismissed && (r.frequency.is_some() || r.subscription))
    {
        return Err("Invalid recurring decision.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imports::Raw;
    fn records(dates: &[&str]) -> (Vec<Record>, Vec<History>) {
        let mut records = Vec::new();
        let mut history = Vec::new();
        for (i, date) in dates.iter().enumerate() {
            let id = format!("record-{i}");
            let fields = Fields {
                date: Some((*date).into()),
                amount: Some("-10.01".into()),
                description: format!("Fictional invoice {i}"),
                merchant: if i % 2 == 0 {
                    "  FICTIONAL   SERVICE "
                } else {
                    "fictional service"
                }
                .into(),
                category: "Services".into(),
                kind: Kind::Expense,
            };
            let raw = Raw {
                page: None,
                reported_line: None,
                extraction_method: "csv-utf8-v1".into(),
                record_number: i as u32 + 2,
                line: i as u64 + 2,
                reported_date: (*date).into(),
                reported_amount: Some("-10.01".into()),
                reported_debit: None,
                reported_credit: None,
                reported_description: fields.description.clone(),
                reported_merchant: Some(fields.merchant.clone()),
                reported_category: Some(fields.category.clone()),
                reported_kind: Some("expense".into()),
                reported_reference: None,
                reported_balance: None,
                extracted: fields.clone(),
                reference: None,
                balance: None,
                issues: vec![],
            };
            records.push(Record {
                id: id.clone(),
                document_id: "doc".into(),
                account_id: "account".into(),
                raw,
                fields: fields.clone(),
                status: Status::Approved,
                linked_id: None,
                reason: "Reviewed".into(),
                warnings: vec![],
                duplicate_ids: vec![],
                bulk_eligible: false,
            });
            history.push(History {
                id: format!("approval-{i}"),
                record_id: id,
                from_status: Status::Pending,
                to_status: Status::Approved,
                fields,
                reason: "Reviewed".into(),
                reviewed_at: "2026-10-14T00:00:00Z".into(),
                counselor_name: "Fictional counselor".into(),
            });
        }
        (records, history)
    }
    #[test]
    fn exact_month_end_variable_amounts_and_advisory_exclusions() {
        let (mut rows, mut history) = records(&["2024-01-31", "2024-02-29", "2024-03-31"]);
        rows[1].fields.amount = Some("-12.03".into());
        history[1].fields = rows[1].fields.clone();
        let g = analyze(&rows, &[], &history).unwrap().remove(0);
        assert_eq!(
            (
                g.total_paid.as_str(),
                g.smallest_payment.as_str(),
                g.largest_payment.as_str()
            ),
            ("32.05", "10.01", "12.03")
        );
        assert_eq!(g.suggested_frequency, Some(Frequency::Monthly));
        assert_eq!(g.state, "Unreviewed");
        rows[1].status = Status::Pending;
        assert!(!analyze(&rows, &[], &history).unwrap()[0].can_confirm);
        for kind in [
            Kind::Refund,
            Kind::Transfer,
            Kind::BalanceMovement,
            Kind::Income,
            Kind::Interest,
        ] {
            rows.iter_mut().for_each(|r| r.fields.kind = kind.clone());
            assert!(analyze(&rows, &[], &history).unwrap().is_empty());
        }
    }
    #[test]
    fn conservative_cadence_duplicate_same_day_account_and_identity_boundaries() {
        for (dates, expected) in [
            (
                ["2026-01-01", "2026-01-08", "2026-01-15"],
                Some(Frequency::Weekly),
            ),
            (
                ["2026-01-01", "2026-01-15", "2026-01-29"],
                Some(Frequency::Fortnightly),
            ),
            (
                ["2026-01-01", "2026-04-01", "2026-07-01"],
                Some(Frequency::Quarterly),
            ),
            (
                ["2024-02-29", "2025-02-28", "2026-02-28"],
                Some(Frequency::Annual),
            ),
            (["2026-01-01", "2026-02-15", "2026-03-17"], None),
        ] {
            let (r, h) = records(&dates);
            assert_eq!(
                analyze(&r, &[], &h).unwrap()[0].suggested_frequency,
                expected
            );
        }
        let (mut r, h) = records(&["2026-01-01", "2026-02-01", "2026-03-01"]);
        r[0].duplicate_ids.push("possible-duplicate".into());
        assert!(!analyze(&r, &[], &h).unwrap()[0].can_confirm);
        r[0].duplicate_ids.clear();
        r[1].fields.date = r[0].fields.date.clone();
        assert!(!analyze(&r, &[], &h).unwrap()[0].can_confirm);
        r[1].account_id = "other-account".into();
        assert_eq!(analyze(&r, &[], &h).unwrap().len(), 2);
        r.iter_mut().for_each(|r| r.fields.merchant.clear());
        assert_eq!(analyze(&r, &[], &h).unwrap().len(), 3);
    }
    #[test]
    fn evidence_change_and_reapproval_never_silently_reactivate_confirmation() {
        let (mut rows, mut h) = records(&["2026-01-01", "2026-02-01", "2026-03-01"]);
        let g = analyze(&rows, &[], &h).unwrap().remove(0);
        let review = Review {
            invalidated: false,
            id: "review".into(),
            group_id: g.id,
            evidence_hash: g.evidence_hash,
            label: "Fictional service".into(),
            decision: Decision::Confirmed,
            frequency: Some(Frequency::Monthly),
            subscription: true,
            reason: "Verified statement and contract".into(),
            reviewed_at: "2026-10-14T00:00:00Z".into(),
            counselor_name: "Fictional counselor".into(),
            evidence: g.evidence,
        };
        validate_review(&review, &rows, &h).unwrap();
        assert_eq!(
            analyze(&rows, std::slice::from_ref(&review), &h).unwrap()[0].state,
            "Confirmed"
        );
        rows[1].status = Status::Rejected;
        assert_eq!(
            analyze(&rows, std::slice::from_ref(&review), &h).unwrap()[0].state,
            "Needs review"
        );
        rows[1].status = Status::Approved;
        h.push(History {
            id: "new-approval".into(),
            record_id: rows[1].id.clone(),
            from_status: Status::Rejected,
            to_status: Status::Approved,
            fields: rows[1].fields.clone(),
            reason: "Reapproved".into(),
            reviewed_at: "2026-10-14T01:00:00Z".into(),
            counselor_name: "Fictional counselor".into(),
        });
        assert_eq!(
            analyze(&rows, std::slice::from_ref(&review), &h).unwrap()[0].state,
            "Needs review"
        );
        let mut corrupt = review.clone();
        corrupt.evidence[0].fields.amount = Some("-99.99".into());
        assert!(validate_review(&corrupt, &rows, &h).is_err());
    }
}
