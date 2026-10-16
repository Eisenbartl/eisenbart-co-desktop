//! Local advisory categories over current, scoped bank approval evidence.
//! Suggestions only prepare drafts. Existing transaction review owns all writes.
use crate::{
    imports::{self, Fields, Kind, Snapshot, Status},
    recurring::{self, Evidence},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
pub const VERSION: &str = "approved-merchant-category-v1";
pub const PAGE_SIZE: usize = 25;
const STALE: &str =
    "Classification evidence changed. Reload imported records and suggestions before continuing.";
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Query {
    pub text: String,
    pub offset: usize,
    pub expected_snapshot: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<crate::category_rules::Version>,
    pub record_id: String,
    pub document_id: String,
    pub account_label: String,
    pub file_name: String,
    pub page: Option<u32>,
    pub line: u64,
    pub fields: Fields,
    pub status: Status,
    pub category: Option<String>,
    pub reason: String,
    pub evidence_count: usize,
    pub evidence: Vec<Evidence>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub client_id: String,
    pub revision: i64,
    pub snapshot: String,
    pub policy: String,
    pub query: Query,
    pub total: usize,
    pub page_size: usize,
    pub candidates: Vec<Candidate>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftInput {
    pub expected_revision: i64,
    pub expected_snapshot: String,
    pub record_id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<crate::category_rules::Selection>,
    pub client_id: String,
    pub revision: i64,
    pub record_id: String,
    pub category: String,
}
fn fingerprint(s: &Snapshot) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                VERSION,
                &s.client_id,
                s.revision,
                &s.accounts,
                &s.documents,
                &s.records,
                &s.history
            ))
            .map_err(|_| "Classification evidence could not be checked.")?
        )
    ))
}
fn eligible(r: &imports::Record) -> bool {
    matches!(r.status, Status::Pending | Status::NeedsClarification)
        && matches!(r.fields.kind, Kind::Expense | Kind::Fee)
        && r.fields.category.is_empty()
        && r.linked_id.is_none()
}
fn candidate(s: &Snapshot, r: &imports::Record) -> Result<Candidate, String> {
    let key = recurring::merchant_identity(&r.account_id, &r.fields)?.0;
    let group = s.merchant_groups.iter().find(|g| g.id == key);
    let count = group.map_or(0, |g| g.evidence.len());
    let evidence = group
        .map(|g| g.evidence.iter().take(5).cloned().collect())
        .unwrap_or_default();
    let mut result = Candidate {
        rule: None,
        record_id: r.id.clone(),
        document_id: r.document_id.clone(),
        account_label: s
            .accounts
            .iter()
            .find(|a| a.id == r.account_id)
            .map(|a| a.label.clone())
            .unwrap_or_default(),
        file_name: s
            .documents
            .iter()
            .find(|d| d.id == r.document_id)
            .map(|d| d.file_name.clone())
            .unwrap_or_default(),
        page: r.raw.page,
        line: r.raw.line,
        fields: r.fields.clone(),
        status: r.status.clone(),
        category: None,
        reason: String::new(),
        evidence_count: count,
        evidence,
    };
    result.reason=if r.fields.date.is_none() || r.fields.amount.as_ref().and_then(|a|imports::signed_money(a).ok()).is_none_or(|a|a>=0) || r.fields.description.is_empty() {
        "Verify the date, withdrawal amount and description in individual review first."
    } else if !r.duplicate_ids.is_empty() {
        "Possible duplicate source records need individual review."
    } else if count<2 {
        "At least two matching current approvals on distinct dates are required."
    } else if count>5 {
        "More than five matching approvals: this bounded workflow cannot suggest a category. Inspect the full history in Working tools."
    } else {
        let g=group.ok_or("Classification evidence is unavailable.")?;
        let categories:BTreeSet<_>=g.evidence.iter().map(|e|e.fields.category.as_str()).collect();
        let dates:BTreeSet<_>=g.evidence.iter().filter_map(|e|e.fields.date.as_ref()).collect();
        if categories.len()!=1 || categories.contains("") {
            "Matching current approvals have conflicting or missing categories; choose manually."
        } else if dates.len()<2 || g.evidence.iter().any(|e|!e.duplicate_ids.is_empty()) {
            "Independent evidence is insufficient or may contain duplicate records."
        } else {
            result.category=categories.first().map(|v|(*v).to_owned());
            "All matching current approvals agree. Same account and transaction type; merchant or fallback description matches after case/space normalization only."
        }
    }.into();
    Ok(result)
}
#[cfg(test)]
pub fn query(s: &Snapshot, q: Query) -> Result<Page, String> {
    query_rules(s, q, &[], "")
}
pub fn context_fingerprint(s: &Snapshot, rules_fingerprint: &str) -> Result<String, String> {
    let base = fingerprint(s)?;
    if rules_fingerprint.is_empty() {
        return Ok(base);
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(format!("{base}:{rules_fingerprint}"))
    ))
}
fn candidate_rules(
    s: &Snapshot,
    r: &imports::Record,
    rules: &[crate::category_rules::Version],
) -> Result<Candidate, String> {
    let mut c = candidate(s, r)?;
    let matching = rules
        .iter()
        .filter_map(
            |v| match crate::category_rules::matches(&v.data, &r.account_id, &r.fields) {
                Ok(true) => Some(Ok(v)),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            },
        )
        .collect::<Result<Vec<_>, String>>()?;
    if matching.len() > 1 {
        c.category = None;
        c.reason = "Conflicting active rules; review manually.".into();
        return Ok(c);
    }
    if let Some(v) = matching.first() {
        c.rule = Some((*v).clone());
        c.category = None;
        let group = s
            .merchant_groups
            .iter()
            .find(|g| crate::category_rules::key(&v.data).ok().as_ref() == Some(&g.id));
        c.reason=if r.fields.date.is_none()||r.fields.description.is_empty()||r.fields.amount.as_ref().and_then(|a|imports::signed_money(a).ok()).is_none_or(|a|a>=0)||!r.duplicate_ids.is_empty(){"Verify unknown source facts or possible duplicates before using this rule."}
   else if group.is_some_and(|g|g.evidence.len()>5||g.evidence.iter().any(|e|!e.duplicate_ids.is_empty())){"The matching evidence exceeds this bounded workflow or contains possible duplicates; review manually."}
   else if group.is_some_and(|g|g.evidence.iter().any(|e|e.fields.category!=v.data.category)){"The rule conflicts with current approved categories. Resolve the evidence or rule explicitly; no category is chosen."}
   else {c.category=Some(v.data.category.clone());"Explicit counselor rule, with no conflicting current approved category. This fills an unsaved review only."}.into();
    }
    Ok(c)
}
pub fn query_rules(
    s: &Snapshot,
    mut q: Query,
    rules: &[crate::category_rules::Version],
    rules_fingerprint: &str,
) -> Result<Page, String> {
    q.text = q.text.trim().into();
    if q.text.chars().count() > 100
        || q.text.chars().any(char::is_control)
        || !q.offset.is_multiple_of(PAGE_SIZE)
    {
        return Err(
            "Use a literal search of up to 100 characters and a valid classification page.".into(),
        );
    }
    let snapshot = context_fingerprint(s, rules_fingerprint)?;
    if q.expected_snapshot.as_ref().is_some_and(|v| v != &snapshot)
        || (q.offset > 0 && q.expected_snapshot.is_none())
    {
        return Err(STALE.into());
    }
    let text = q.text.to_lowercase();
    let mut rows: Vec<_> = s
        .records
        .iter()
        .filter(|r| {
            eligible(r)
                && (text.is_empty()
                    || format!("{} {}", r.fields.description, r.fields.merchant)
                        .to_lowercase()
                        .contains(&text))
        })
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    if q.offset > rows.len() {
        return Err("This classification page is unavailable. Reload suggestions.".into());
    }
    let candidates = rows
        .iter()
        .skip(q.offset)
        .take(PAGE_SIZE)
        .map(|r| candidate_rules(s, r, rules))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page {
        client_id: s.client_id.clone(),
        revision: s.revision,
        snapshot,
        policy: VERSION.into(),
        query: q,
        total: rows.len(),
        page_size: PAGE_SIZE,
        candidates,
    })
}
#[cfg(test)]
pub fn prepare(s: &Snapshot, input: DraftInput) -> Result<Draft, String> {
    prepare_rules(s, input, &[], "")
}
pub fn prepare_rules(
    s: &Snapshot,
    input: DraftInput,
    rules: &[crate::category_rules::Version],
    rules_fingerprint: &str,
) -> Result<Draft, String> {
    if input.expected_revision != s.revision
        || input.expected_snapshot != context_fingerprint(s, rules_fingerprint)?
    {
        return Err(STALE.into());
    }
    let r = s
        .records
        .iter()
        .find(|r| r.id == input.record_id && eligible(r))
        .ok_or("This record is not eligible for a category suggestion.")?;
    let c = candidate_rules(s, r, rules)?;
    Ok(Draft {
        rule: c.rule.map(|v| crate::category_rules::Selection {
            rule_id: v.id,
            revision: v.revision,
            expected_snapshot: input.expected_snapshot,
        }),
        client_id: s.client_id.clone(),
        revision: s.revision,
        record_id: r.id.clone(),
        category: c.category.ok_or(c.reason)?,
    })
}
