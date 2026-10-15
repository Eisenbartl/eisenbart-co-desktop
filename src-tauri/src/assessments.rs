//! Immutable counselor assessment evidence; later source changes never rewrite it.
use crate::{
    dashboard, imports,
    models::{Client, WorkspaceInfo},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
pub const MAX_PAYLOAD: usize = 8 * 1024 * 1024;
pub const FORMAT: u32 = 3;
pub const INTAKE_FORMAT: u32 = 5;
pub const CALCULATOR: &str = "approved-assessment-v1";
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    #[serde(default)]
    pub expected_branding_revision: Option<i64>,
    #[serde(default)]
    pub intake: Option<crate::report_intake::Selection>,
    #[serde(default)]
    pub supporting: Option<crate::report_evidence::Selection>,
    #[serde(default)]
    pub selected_scenario: Option<crate::saved_scenarios::Selection>,
    #[serde(default)]
    pub planning: Option<crate::planning::Selection>,
    pub expected_revision: i64,
    pub expected_profile_updated_at: String,
    pub from_month: String,
    pub through_month: String,
    pub title: String,
    pub findings: String,
    pub recommendations: String,
    pub follow_up: String,
    pub acknowledge: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branding: Option<crate::branding::Snapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intake: Option<crate::report_intake::Snapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supporting: Option<crate::report_evidence::Snapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_scenario: Option<crate::saved_scenarios::Selection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planning: Option<crate::planning::Snapshot>,
    pub format: u32,
    pub calculator: String,
    pub app_version: String,
    pub id: String,
    pub created_at: String,
    pub workspace: WorkspaceInfo,
    pub client: Client,
    pub title: String,
    pub findings: String,
    pub recommendations: String,
    pub follow_up: String,
    pub assessment: dashboard::Assessment,
    pub basis: imports::Snapshot,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub period_start: String,
    pub period_end: String,
    pub source_revision: i64,
    pub needs_review: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub client_id: String,
    pub records: Vec<Summary>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub validation: crate::completeness::ReportValidation,
    pub client_id: String,
    pub payload: Payload,
    pub digest: String,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<crate::saved_scenarios::Record>,
}

pub fn capture(source: imports::Snapshot) -> imports::Snapshot {
    capture_report(source, false)
}
pub fn capture_report(mut source: imports::Snapshot, supporting: bool) -> imports::Snapshot {
    if supporting {
        let mut seen = HashSet::new();
        source.recurring_history.reverse();
        source
            .recurring_history
            .retain(|r| seen.insert(r.group_id.clone()));
        source.recurring_history.reverse();
    }
    // Keep exact current decisions plus older approved bindings needed by facts.
    let mut latest = HashMap::new();
    for h in &source.history {
        latest.insert(h.record_id.as_str(), h.id.as_str());
    }
    let mut ids = latest
        .values()
        .map(|id| (*id).to_owned())
        .collect::<HashSet<_>>();
    if let Some(data) = &source.financial_facts.data {
        ids.extend(data.classifications.iter().map(|c| c.approval_id.clone()));
    }
    if supporting {
        ids.extend(
            source
                .recurring_history
                .iter()
                .flat_map(|r| r.evidence.iter().map(|e| e.approval_id.clone())),
        );
    }
    source.history.retain(|h| ids.contains(&h.id));
    source.financial_facts.history = source.financial_facts.history.pop().into_iter().collect();
    for d in &mut source.documents {
        d.balance_history = d.balance_history.pop().into_iter().collect();
    }
    if !supporting {
        source.merchant_groups.clear();
        source.recurring_history.clear();
    }
    source
}
pub fn validate(payload: &Payload) -> Result<(), String> {
    let bad = || "The saved assessment evidence is invalid or unsupported.".to_owned();
    if !matches!(payload.format, 1 | 2 | FORMAT | 4 | INTAKE_FORMAT | 6)
        || (payload.format == 1 && payload.planning.is_some())
        || (payload.supporting.is_some() && payload.format < FORMAT)
        || (payload.format == FORMAT && payload.supporting.is_none())
        || ((payload.intake.is_some() && !matches!(payload.format, INTAKE_FORMAT | 6))
            || (payload.format == INTAKE_FORMAT && payload.intake.is_none()))
        || (payload.format == 4 && payload.selected_scenario.is_none())
        || (payload.selected_scenario.is_some() && !matches!(payload.format, 4 | INTAKE_FORMAT | 6))
        || (payload.selected_scenario.is_some() && payload.planning.is_none())
        || payload.calculator != CALCULATOR
        || payload.app_version.is_empty()
        || payload.id.is_empty()
        || payload.client.id != payload.basis.client_id
        || payload.client.id != payload.assessment.client_id
        || payload.client.organization_id != payload.workspace.organization_id
        || payload.basis.revision != payload.assessment.revision
    {
        return Err(bad());
    }
    if payload.branding.is_some() != (payload.format == 6) {
        return Err(bad());
    }
    if let Some(branding) = &payload.branding {
        crate::branding::validate(branding)?;
        if branding.organization_id != payload.workspace.organization_id
            || branding.data.name != payload.workspace.organization_name
        {
            return Err(bad());
        }
    }
    if let Some(intake) = &payload.intake {
        crate::report_intake::validate(intake, &payload.client.id)?;
    }
    if let Some(selection) = &payload.selected_scenario {
        if uuid::Uuid::parse_str(&selection.id).is_err()
            || selection.digest.len() != 64
            || !selection.digest.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(bad());
        }
    }
    if let Some(p) = &payload.planning {
        crate::planning::validate(p, &payload.client.id)?;
    }
    let date = chrono::DateTime::parse_from_rfc3339(&payload.created_at)
        .map_err(|_| bad())?
        .date_naive();
    if date.to_string() != payload.assessment.calculated_on_utc {
        return Err(bad());
    }
    let s = &payload.basis;
    if s.records.len() > 5000
        || s.documents.len() > 50
        || s.accounts.len() > 20
        || s.history.len()
            > if payload.supporting.is_some() {
                15500
            } else {
                5500
            }
        || s.financial_facts.history.len() > 1
        || (payload.supporting.is_none()
            && (!s.merchant_groups.is_empty() || !s.recurring_history.is_empty()))
    {
        return Err(bad());
    }
    let records = s
        .records
        .iter()
        .map(|r| r.id.as_str())
        .collect::<HashSet<_>>();
    let docs = s
        .documents
        .iter()
        .map(|d| d.id.as_str())
        .collect::<HashSet<_>>();
    let accounts = s
        .accounts
        .iter()
        .map(|a| a.id.as_str())
        .collect::<HashSet<_>>();
    let decisions = s
        .history
        .iter()
        .map(|h| h.id.as_str())
        .collect::<HashSet<_>>();
    if records.len() != s.records.len()
        || docs.len() != s.documents.len()
        || accounts.len() != s.accounts.len()
        || decisions.len() != s.history.len()
    {
        return Err(bad());
    }
    for d in &s.documents {
        if !accounts.contains(d.account_id.as_str())
            || d.sha256.len() != 64
            || !d.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || d.balance_history.len() > 1
        {
            return Err(bad());
        }
    }
    for h in &s.history {
        if !records.contains(h.record_id.as_str()) {
            return Err(bad());
        }
    }
    for r in &s.records {
        if !docs.contains(r.document_id.as_str())
            || !accounts.contains(r.account_id.as_str())
            || !s
                .documents
                .iter()
                .any(|d| d.id == r.document_id && d.account_id == r.account_id)
        {
            return Err(bad());
        }
        let latest = s.history.iter().rev().find(|h| h.record_id == r.id);
        if let Some(h) = latest {
            if h.to_status != r.status || h.fields != r.fields {
                return Err(bad());
            }
        } else if matches!(
            r.status,
            imports::Status::Approved | imports::Status::Rejected
        ) {
            return Err(bad());
        }
        if r.status == imports::Status::Approved && imports::approval_error(&r.fields).is_some() {
            return Err(bad());
        }
        if r.status == imports::Status::Approved && r.fields.kind == imports::Kind::Transfer {
            let other = r
                .linked_id
                .as_deref()
                .and_then(|id| s.records.iter().find(|x| x.id == id))
                .ok_or_else(bad)?;
            if other.status != imports::Status::Approved
                || other.fields.kind != imports::Kind::Transfer
                || other.linked_id.as_deref() != Some(&r.id)
                || other.account_id == r.account_id
                || imports::signed_money(other.fields.amount.as_deref().ok_or_else(bad)?)?
                    != -imports::signed_money(r.fields.amount.as_deref().ok_or_else(bad)?)?
            {
                return Err(bad());
            }
        }
    }
    if let Some(data) = &s.financial_facts.data {
        if crate::financial_facts::normalize(data.clone())? != *data
            || s.financial_facts.history.last().map(|h| &h.data) != Some(data)
        {
            return Err(bad());
        }
        crate::financial_facts::validate_bindings(data, &s.records, &s.history, &docs)?;
    } else if !s.financial_facts.history.is_empty() {
        return Err(bad());
    }
    if payload.supporting.is_some() {
        crate::report_evidence::validate(payload)?;
    }
    imports::iso_date(&payload.assessment.period_start)?;
    imports::iso_date(&payload.assessment.period_end)?;
    let summary = imports::summarize(&s.records, &s.accounts)?;
    if serde_json::to_value(summary).map_err(|_| bad())?
        != serde_json::to_value(&s.summary).map_err(|_| bad())?
    {
        return Err(bad());
    }
    let expected = dashboard::assess(
        s,
        dashboard::Input {
            from_month: payload
                .assessment
                .period_start
                .get(..7)
                .ok_or_else(bad)?
                .to_owned(),
            through_month: payload
                .assessment
                .period_end
                .get(..7)
                .ok_or_else(bad)?
                .to_owned(),
        },
        date,
    )?;
    if serde_json::to_value(expected).map_err(|_| bad())?
        != serde_json::to_value(&payload.assessment).map_err(|_| bad())?
    {
        return Err(bad());
    }
    Ok(())
}
