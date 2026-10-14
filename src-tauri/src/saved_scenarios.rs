//! Immutable combined proposals, recomputed from their own captured baseline.
use crate::{models::WorkspaceInfo, planning, scenarios};
use serde::{Deserialize, Serialize};
pub const MAX_PAYLOAD: usize = 8 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_count: u32,
    pub proposal: scenarios::Input,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub id: String,
    pub digest: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payload {
    pub id: String,
    pub format: u32,
    pub app_version: String,
    pub workspace: WorkspaceInfo,
    pub comparison: scenarios::Comparison,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub client_id: String,
    pub payload: Payload,
    pub digest: String,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub label: String,
    pub created_at: String,
    pub digest: String,
    pub needs_review: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub client_id: String,
    pub records: Vec<Summary>,
}
pub fn copy<T: Serialize + serde::de::DeserializeOwned>(v: &T) -> Result<T, String> {
    serde_json::from_value(serde_json::to_value(v).map_err(|_| "Invalid captured scenario.")?)
        .map_err(|_| "Invalid captured scenario.".into())
}
fn stored(mut c: scenarios::Comparison, created: &str) -> scenarios::Comparison {
    c.calculator = "combined-scenario-v1".into();
    c.created_at = created.into();
    c.warnings[0]="Saved hypothetical proposal only. Saving or selecting this record does not change live plans, approved history or client agreements. Later model edits flag this baseline for review without rewriting it.".into();
    c
}
pub fn capture(
    plans: planning::Plans,
    input: scenarios::Input,
    workspace: WorkspaceInfo,
    id: String,
    created: String,
) -> Result<Payload, String> {
    let p = Payload {
        id,
        format: 1,
        app_version: env!("CARGO_PKG_VERSION").into(),
        workspace,
        comparison: stored(scenarios::compare(plans, input)?, &created),
    };
    validate(&p, &p.comparison.client_id)?;
    Ok(p)
}
pub fn validate(p: &Payload, client: &str) -> Result<(), String> {
    let bad = || "The saved combined scenario is invalid or unsupported.".to_owned();
    if p.format != 1
        || p.app_version.is_empty()
        || uuid::Uuid::parse_str(&p.id).is_err()
        || p.comparison.client_id != client
        || p.comparison.calculator != "combined-scenario-v1"
        || serde_json::to_vec(p).map_err(|_| bad())?.len() > MAX_PAYLOAD
    {
        return Err(bad());
    }
    chrono::DateTime::parse_from_rfc3339(&p.comparison.created_at).map_err(|_| bad())?;
    let expected = stored(
        scenarios::compare(
            copy(&p.comparison.baseline.plans)?,
            copy(&p.comparison.input)?,
        )?,
        &p.comparison.created_at,
    );
    if serde_json::to_value(expected).map_err(|_| bad())?
        != serde_json::to_value(&p.comparison).map_err(|_| bad())?
    {
        return Err(bad());
    }
    Ok(())
}
pub fn planning(p: &Payload) -> Result<planning::Snapshot, String> {
    planning::capture(
        copy(&p.comparison.proposed.plans)?,
        planning::Selection {
            expected_revisions: p.comparison.revisions.clone(),
            label: p.comparison.input.label.clone(),
            assumptions: p.comparison.input.assumptions.clone(),
            debt_strategy: p.comparison.proposed.strategy.clone(),
        },
    )
}
