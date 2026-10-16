//! Counselor history and explicit note corrections; no financial calculations.
use crate::models::ClientNote;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Amendment {
    pub revision: i64,
    pub previous_id: String,
    pub note: ClientNote,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub client_id: String,
    pub fingerprint: String,
    pub original: ClientNote,
    pub amendments: Vec<Amendment>,
}
impl Thread {
    pub fn current(&self) -> &ClientNote {
        self.amendments
            .last()
            .map(|a| &a.note)
            .unwrap_or(&self.original)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Correction {
    pub note_id: String,
    pub expected_fingerprint: String,
    pub body: String,
    pub reason: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Profile,
    Note,
    Intake,
    Document,
    Scenario,
    Report,
    ActionPlan,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub source_id: String,
    pub kind: Kind,
    pub title: String,
    pub detail: String,
    pub occurred_at: String,
    pub day: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Query {
    pub text: String,
    pub kind: Option<Kind>,
    pub from_date: String,
    pub through_date: String,
    pub offset: usize,
    pub expected_snapshot: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub client_id: String,
    pub snapshot: String,
    pub offset: usize,
    pub total: usize,
    pub events: Vec<Event>,
}
