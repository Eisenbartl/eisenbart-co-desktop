//! Canonical counselor-authored context, separate from approved financial amounts.
use crate::{assessments, models::ClientNote};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
pub const MAX_NOTES: usize = 10;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub expected_notes_count: u32,
    pub note_ids: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub calculator: String,
    pub notes_count: u32,
    pub notes: Vec<ClientNote>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub client_id: String,
    pub source_revision: i64,
    pub notes_count: u32,
    pub notes: Vec<ClientNote>,
    pub group_count: usize,
    pub review_count: usize,
}
pub fn validate(p: &assessments::Payload) -> Result<(), String> {
    let e = p
        .supporting
        .as_ref()
        .ok_or("Missing counselor report evidence.")?;
    if e.calculator != "counselor-evidence-v1"
        || e.notes.len() > MAX_NOTES
        || e.notes_count < e.notes.len() as u32
        || e.notes_count > 10_000
    {
        return Err("Invalid counselor report evidence limits/version.".into());
    }
    let recorded =
        chrono::DateTime::parse_from_rfc3339(&p.created_at).map_err(|_| "Invalid report time.")?;
    let mut ids = HashSet::new();
    for n in &e.notes {
        if n.id.is_empty()
            || !ids.insert(&n.id)
            || n.client_id != p.client.id
            || n.counselor_id != p.workspace.counselor_id
            || n.counselor_name.trim().is_empty()
            || n.counselor_name.chars().count() > 120
            || crate::action_plans::text(&n.body, "Session note", 8000, true)? != n.body
            || chrono::DateTime::parse_from_rfc3339(&n.created_at)
                .map_err(|_| "Invalid session note time.")?
                > recorded
        {
            return Err("Invalid captured session note identity/text/time.".into());
        }
    }
    let s = &p.basis;
    if s.recurring_history.len() > 1000 || s.merchant_groups.len() > 5000 {
        return Err("Invalid captured payment evidence limits.".into());
    }
    let mut groups = HashSet::new();
    let mut review_ids = HashSet::new();
    for r in &s.recurring_history {
        if !groups.insert(&r.group_id)
            || !review_ids.insert(&r.id)
            || chrono::DateTime::parse_from_rfc3339(&r.reviewed_at)
                .map_err(|_| "Invalid payment review time.")?
                > recorded
        {
            return Err("Invalid captured payment review identity/time.".into());
        }
        crate::recurring::validate_review(r, &s.records, &s.history)?;
    }
    let expected = crate::recurring::analyze(&s.records, &s.recurring_history, &s.history)?;
    if serde_json::to_value(expected).map_err(|_| "Cannot validate captured groups.")?
        != serde_json::to_value(&s.merchant_groups)
            .map_err(|_| "Cannot validate captured groups.")?
    {
        return Err("Captured payment groups differ from approved evidence.".into());
    }
    Ok(())
}
