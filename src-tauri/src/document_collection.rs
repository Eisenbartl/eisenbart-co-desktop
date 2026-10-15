//! Collection progress is counselor-entered workflow context, never approval.
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub account_id: Option<String>,
    pub source_document_id: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub due_date: Option<String>,
    pub received_date: Option<String>,
    pub notes: String,
    pub archived: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub items: Vec<Item>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub revision: i64,
    pub updated_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub account_id: String,
    pub file_name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub client_id: String,
    pub revision: i64,
    pub latest_revision: i64,
    pub updated_at: Option<String>,
    pub review_date: String,
    pub items: Vec<Item>,
    pub history: Vec<Version>,
    pub accounts: Vec<crate::imports::Account>,
    pub sources: Vec<Source>,
    pub missing: usize,
    pub received: usize,
    pub archived: usize,
}
fn date(s: &Option<String>) -> Result<(), String> {
    if let Some(s) = s {
        let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| "Enter dates in YYYY-MM-DD format.".to_owned())?;
        if d.format("%Y-%m-%d").to_string() != *s
            || !("1900-01-01"..="2100-12-31").contains(&s.as_str())
        {
            return Err("Checklist dates must be from 1900-01-01 through 2100-12-31.".into());
        }
    }
    Ok(())
}
pub fn canonical(mut items: Vec<Item>) -> Result<Vec<Item>, String> {
    if items.len() > 100 {
        return Err(
            "A client checklist supports up to 100 requests, including archived requests.".into(),
        );
    }
    let mut ids = HashSet::new();
    for i in &mut items {
        if uuid::Uuid::parse_str(&i.id).is_err() || !ids.insert(i.id.clone()) {
            return Err("Checklist identifiers are invalid or repeated.".into());
        }
        i.title = i.title.trim().to_owned();
        i.notes = i.notes.trim().to_owned();
        if i.title.is_empty()
            || i.title.chars().count() > 200
            || i.title.chars().any(char::is_control)
            || i.notes.chars().count() > 2000
            || i.notes
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err("Enter a request title up to 200 characters and notes up to 2,000 characters, without unsupported control characters.".into());
        }
        if !matches!(
            i.kind.as_str(),
            "statement" | "creditCard" | "loan" | "paystub" | "tax" | "other"
        ) || !matches!(i.status.as_str(), "missing" | "received")
        {
            return Err("Choose a valid document type and missing/received status.".into());
        }
        for s in [&i.account_id, &i.source_document_id].into_iter().flatten() {
            if uuid::Uuid::parse_str(s).is_err() {
                return Err("Choose a saved account or source document.".into());
            }
        }
        for d in [
            &i.period_start,
            &i.period_end,
            &i.due_date,
            &i.received_date,
        ] {
            date(d)?;
        }
        if matches!((&i.period_start,&i.period_end),(Some(a),Some(b)) if a>b) {
            return Err("The requested period end must not precede its start.".into());
        }
        if (i.status == "received") != i.received_date.is_some()
            || (i.source_document_id.is_some() && i.status != "received")
        {
            return Err("Received requests need an explicit received date. Missing requests cannot have a received date or linked source.".into());
        }
    }
    if serde_json::to_vec(&items)
        .map_err(|_| "Checklist could not be encoded.".to_owned())?
        .len()
        > 131072
    {
        return Err("This checklist exceeds the 128 KiB limit.".into());
    }
    Ok(items)
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn item() -> Item {
        Item {
            id: uuid::Uuid::new_v4().to_string(),
            title: " Fictional September statement ".into(),
            kind: "statement".into(),
            status: "missing".into(),
            account_id: None,
            source_document_id: None,
            period_start: Some("2026-09-01".into()),
            period_end: Some("2026-09-30".into()),
            due_date: Some("2026-10-30".into()),
            received_date: None,
            notes: " Verify complete period ".into(),
            archived: false,
        }
    }
    #[test]
    fn collection_validation_keeps_unknowns_and_rejects_invalid_states_dates_ids_and_limits() {
        let a = item();
        let c = canonical(vec![a.clone()]).unwrap();
        assert_eq!(c[0].title, "Fictional September statement");
        assert!(c[0].account_id.is_none());
        assert!(canonical(vec![a.clone(), a.clone()]).is_err());
        for mutate in [
            |i: &mut Item| i.received_date = Some("2026-10-14".into()),
            |i: &mut Item| i.status = "received".into(),
            |i: &mut Item| i.due_date = Some("2026-02-30".into()),
            |i: &mut Item| i.period_end = Some("2026-08-31".into()),
            |i: &mut Item| i.title = "bad\0title".into(),
            |i: &mut Item| i.account_id = Some("foreign".into()),
        ] {
            let mut b = a.clone();
            mutate(&mut b);
            assert!(canonical(vec![b]).is_err());
        }
        assert!(canonical((0..101).map(|_| item()).collect()).is_err());
        let mut b = a;
        b.status = "received".into();
        b.received_date = Some("2026-10-14".into());
        assert!(canonical(vec![b]).is_ok());
    }
}
