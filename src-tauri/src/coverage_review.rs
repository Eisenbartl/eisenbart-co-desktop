//! Counselor-entered coverage context, separate from financial facts and final approval.
use crate::{dashboard, imports};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
pub const MAX_JSON: usize = 131072;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub id: String,
    pub label: String,
    pub kind: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub account_id: String,
    pub file_name: String,
    pub sha256: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Choices {
    pub accounts: Vec<Account>,
    pub sources: Vec<Source>,
}
pub fn choices(s: &imports::Snapshot) -> Choices {
    Choices {
        accounts: s
            .accounts
            .iter()
            .map(|a| Account {
                id: a.id.clone(),
                label: a.label.clone(),
                kind: a.kind.clone(),
            })
            .collect(),
        sources: s
            .documents
            .iter()
            .map(|d| Source {
                id: d.id.clone(),
                account_id: d.account_id.clone(),
                file_name: d.file_name.clone(),
                sha256: d.sha256.clone(),
                period_start: d.input.period_start.clone(),
                period_end: d.input.period_end.clone(),
            })
            .collect(),
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Expectation {
    pub account_id: Option<String>,
    pub label: String,
    pub kind: String,
    pub expected_start: Option<String>,
    pub expected_end: Option<String>,
    pub exception: String,
    pub source_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Data {
    pub from_month: String,
    pub through_month: String,
    pub inventory_reviewed: bool,
    pub inventory_notes: String,
    pub accounts: Vec<Expectation>,
    pub missing_information: String,
    pub conclusion: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub client_id: String,
    pub revision: i64,
    pub updated_at: String,
    pub source_revision: i64,
    pub profile_updated_at: String,
    pub choices: Choices,
    pub data: Data,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub expected_source_revision: i64,
    pub expected_profile_updated_at: String,
    pub data: Data,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub revision: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub revision: i64,
    pub updated_at: String,
    pub from_month: String,
    pub through_month: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub client_id: String,
    pub latest_revision: i64,
    pub snapshot: Option<Snapshot>,
    pub history: Vec<Version>,
    pub choices: Choices,
    pub current_source_revision: i64,
    pub current_profile_updated_at: String,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
    pub unresolved: Vec<String>,
}
fn text(value: &str, label: &str, max: usize) -> Result<String, String> {
    let v = value.trim();
    if v.chars().count() > max || v.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        return Err(format!(
            "{label} exceeds its text limit or contains unsupported controls."
        ));
    }
    Ok(v.into())
}
pub fn canonical(mut d: Data, c: &Choices) -> Result<Data, String> {
    d.inventory_notes = text(&d.inventory_notes, "Inventory review notes", 2000)?;
    d.missing_information = text(&d.missing_information, "Missing information", 4000)?;
    d.conclusion = text(&d.conclusion, "Review conclusion", 2000)?;
    if d.inventory_reviewed && d.inventory_notes.is_empty() {
        return Err(
            "Describe the account/obligation inventory reviewed before marking it reviewed.".into(),
        );
    }
    if d.accounts.len() > 40 || c.accounts.len() > 20 || c.sources.len() > 50 {
        return Err(
            "Coverage review supports 40 expected items, 20 known accounts and 50 sources.".into(),
        );
    }
    let (start, end) = period(&d.from_month, &d.through_month)?;
    let mut known = HashSet::new();
    let mut names = HashSet::new();
    for e in &mut d.accounts {
        e.label = text(&e.label, "Expected item label", 120)?;
        e.exception = text(&e.exception, "Account exception", 1000)?;
        if e.label.is_empty()
            || !names.insert(e.label.to_lowercase())
            || !matches!(
                e.kind.as_str(),
                "checking" | "savings" | "creditCard" | "loan" | "other"
            )
        {
            return Err(
                "Use distinct nonempty expected item labels and supported account kinds.".into(),
            );
        }
        if let Some(id) = &e.account_id {
            let a = c
                .accounts
                .iter()
                .find(|a| &a.id == id)
                .ok_or("An expected account is unavailable for this client.")?;
            if !known.insert(id.clone()) || e.label != a.label || e.kind != a.kind {
                return Err(
                    "Keep original known account names/types and include each account once.".into(),
                );
            }
        }
        match (&e.expected_start,&e.expected_end){
            (Some(s),Some(t))=>{let s=imports::iso_date(s)?;let t=imports::iso_date(t)?;if s>t || s<start || t>end{return Err("Expected dates must be ordered and inside the review period.".into());}},
            (None,None)=>{},_=>return Err("Provide both expected dates or leave both unknown; document exceptions separately.".into())
        }
        if e.source_ids.len() > 50
            || e.source_ids.iter().collect::<HashSet<_>>().len() != e.source_ids.len()
        {
            return Err("Select distinct coverage sources.".into());
        }
        for id in &e.source_ids {
            let s = c
                .sources
                .iter()
                .find(|s| &s.id == id)
                .ok_or("A selected source is unavailable for this review.")?;
            if e.account_id.as_deref() != Some(&s.account_id) {
                return Err(
                    "A coverage source must belong to its expected account and this client.".into(),
                );
            }
        }
    }
    if known.len() != c.accounts.len() {
        return Err("Include every known account, including archived accounts. Unknown applicability can remain explicit.".into());
    }
    Ok(d)
}
pub fn period(from: &str, through: &str) -> Result<(chrono::NaiveDate, chrono::NaiveDate), String> {
    let (start, end, _) = dashboard::period(&dashboard::Input {
        from_month: from.into(),
        through_month: through.into(),
    })?;
    Ok((start, end))
}
pub fn validate(s: &Snapshot) -> Result<(), String> {
    if s.choices
        .accounts
        .iter()
        .map(|a| &a.id)
        .collect::<HashSet<_>>()
        .len()
        != s.choices.accounts.len()
        || s.choices
            .sources
            .iter()
            .map(|d| &d.id)
            .collect::<HashSet<_>>()
            .len()
            != s.choices.sources.len()
        || s.choices
            .sources
            .iter()
            .any(|d| !s.choices.accounts.iter().any(|a| a.id == d.account_id))
    {
        return Err("Saved completeness source inventory is invalid.".into());
    }

    if !(1..=100).contains(&s.revision)
        || s.source_revision < 0
        || uuid::Uuid::parse_str(&s.client_id).is_err()
        || chrono::DateTime::parse_from_rfc3339(&s.updated_at).is_err()
        || chrono::DateTime::parse_from_rfc3339(&s.profile_updated_at).is_err()
        || canonical(s.data.clone(), &s.choices)? != s.data
    {
        return Err("Saved completeness review is invalid.".into());
    }
    Ok(())
}
pub fn unresolved(s: &Snapshot) -> Vec<String> {
    let mut result = vec![];
    if !s.data.inventory_reviewed {
        result.push(
            "Household account and obligation inventory has not been marked reviewed.".into(),
        );
    }
    if !s.data.missing_information.is_empty() {
        result.push("Missing information remains recorded in this review.".into());
    }
    if s.data.accounts.is_empty() {
        result.push("No expected financial accounts or obligations are recorded.".into());
    }
    for e in &s.data.accounts {
        if !e.exception.is_empty() {
            result.push(format!(
                "{}: a documented exception still requires professional review.",
                e.label
            ));
        }
        match (&e.expected_start, &e.expected_end) {
            (Some(start), Some(end)) => {
                let (Ok(mut day), Ok(end)) = (imports::iso_date(start), imports::iso_date(end))
                else {
                    continue;
                };
                let mut missing = 0;
                while day <= end {
                    if !s.choices.sources.iter().any(|d| {
                        e.source_ids.contains(&d.id)
                            && d.period_start
                                .as_deref()
                                .is_some_and(|v| v <= day.to_string().as_str())
                            && d.period_end
                                .as_deref()
                                .is_some_and(|v| v >= day.to_string().as_str())
                    }) {
                        missing += 1;
                    }
                    let Some(next) = day.succ_opt() else { break };
                    day = next;
                }
                if missing > 0 {
                    result.push(format!(
                        "{}: {missing} expected days lack selected declared source coverage.",
                        e.label
                    ));
                }
            }
            _ => result.push(format!(
                "{}: expected dates or applicability remain unconfirmed.",
                e.label
            )),
        }
    }
    result
}
