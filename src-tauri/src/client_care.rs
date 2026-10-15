//! Optional counselor-entered intake and date-only follow-up tracking.
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Intake {
    pub address: String,
    pub household: String,
    pub employment: String,
    pub employer: String,
    pub occupation: String,
    pub income_notes: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FollowUp {
    /// Empty only for a new draft; assigned by Rust on save.
    pub id: String,
    pub kind: String,
    pub title: String,
    pub due_date: String,
    pub time: String,
    pub status: String,
    pub notes: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Data {
    pub intake: Intake,
    pub follow_ups: Vec<FollowUp>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub data: Data,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub client_id: String,
    pub revision: i64,
    pub updated_at: Option<String>,
    pub review_date: String,
    pub overdue: usize,
    pub due_today: usize,
    pub data: Data,
}

fn text(s: &str, label: &str, max: usize, multiline: bool) -> Result<String, String> {
    let s = s.trim();
    if s.chars().count() > max
        || s.chars()
            .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t')))
    {
        return Err(format!(
            "{label} must be at most {max} characters without unsupported control characters."
        ));
    }
    Ok(s.into())
}

pub fn canonical(mut d: Data) -> Result<Data, String> {
    d.intake.address = text(&d.intake.address, "Mailing address", 1000, true)?;
    d.intake.household = text(&d.intake.household, "Household composition", 2000, true)?;
    d.intake.employer = text(&d.intake.employer, "Employer", 200, false)?;
    d.intake.occupation = text(&d.intake.occupation, "Occupation", 200, false)?;
    d.intake.income_notes = text(
        &d.intake.income_notes,
        "Employment and income details",
        4000,
        true,
    )?;
    if !matches!(
        d.intake.employment.as_str(),
        "" | "employed" | "selfEmployed" | "unemployed" | "retired" | "student" | "other"
    ) {
        return Err("Choose an employment status, or leave it unspecified.".into());
    }
    if d.follow_ups.len() > 100 {
        return Err("This version supports up to 100 follow-ups per client, including completed and cancelled items.".into());
    }
    let mut ids = HashSet::new();
    for f in &mut d.follow_ups {
        if uuid::Uuid::parse_str(&f.id).is_err() || !ids.insert(&f.id) {
            return Err("Follow-up identifiers are invalid or repeated.".into());
        }
        f.title = text(&f.title, "Follow-up title", 200, false)?;
        f.notes = text(&f.notes, "Follow-up notes", 2000, true)?;
        if f.title.is_empty() {
            return Err("Enter a title for each follow-up.".into());
        }
        if !matches!(f.kind.as_str(), "task" | "appointment")
            || !matches!(f.status.as_str(), "open" | "completed" | "cancelled")
        {
            return Err("Choose a valid follow-up kind and status.".into());
        }
        let date = NaiveDate::parse_from_str(&f.due_date, "%Y-%m-%d")
            .map_err(|_| "Enter a valid follow-up date in YYYY-MM-DD format.".to_owned())?;
        if date.format("%Y-%m-%d").to_string() != f.due_date
            || !("1900-01-01"..="2100-12-31").contains(&f.due_date.as_str())
        {
            return Err("Follow-up dates must be from 1900-01-01 through 2100-12-31.".into());
        }
        if !f.time.is_empty() {
            let t = chrono::NaiveTime::parse_from_str(&f.time, "%H:%M")
                .map_err(|_| "Enter an appointment time in HH:MM format.".to_owned())?;
            if f.kind != "appointment" || t.format("%H:%M").to_string() != f.time {
                return Err("Only appointments can have a time, in HH:MM format.".into());
            }
        }
    }
    if serde_json::to_vec(&d)
        .map_err(|_| "The intake could not be encoded.".to_owned())?
        .len()
        > 131072
    {
        return Err("This intake and follow-up record exceeds the 128 KiB limit.".into());
    }
    Ok(d)
}

pub fn record(
    client: &str,
    revision: i64,
    updated_at: Option<String>,
    data: Data,
    today: &str,
) -> Record {
    let open = |f: &&FollowUp| f.status == "open";
    let overdue = data
        .follow_ups
        .iter()
        .filter(open)
        .filter(|f| f.due_date.as_str() < today)
        .count();
    let due_today = data
        .follow_ups
        .iter()
        .filter(open)
        .filter(|f| f.due_date == today)
        .count();
    Record {
        client_id: client.into(),
        revision,
        updated_at,
        review_date: today.into(),
        overdue,
        due_today,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn follow(date: &str, status: &str) -> FollowUp {
        FollowUp {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "task".into(),
            title: "Verify fictional statement".into(),
            due_date: date.into(),
            time: "".into(),
            status: status.into(),
            notes: "".into(),
        }
    }
    #[test]
    fn due_dates_are_exact_and_closed_items_do_not_generate_reminders() {
        let data = Data {
            follow_ups: vec![
                follow("2026-10-14", "open"),
                follow("2026-10-15", "open"),
                follow("2026-10-16", "open"),
                follow("2026-10-01", "completed"),
                follow("2026-10-01", "cancelled"),
            ],
            ..Default::default()
        };
        let r = record("client", 1, None, canonical(data).unwrap(), "2026-10-15");
        assert_eq!((r.overdue, r.due_today), (1, 1));
    }
    #[test]
    fn validates_optional_intake_dates_times_identifiers_and_bounds() {
        assert_eq!(canonical(Data::default()).unwrap(), Data::default());
        let mut d = Data {
            follow_ups: vec![follow("2026-02-30", "open")],
            ..Default::default()
        };
        assert!(canonical(d.clone()).is_err());
        d.follow_ups[0].due_date = "2028-02-29".into();
        d.follow_ups[0].kind = "appointment".into();
        d.follow_ups[0].time = "09:30".into();
        assert!(canonical(d.clone()).is_ok());
        d.follow_ups[0].kind = "task".into();
        assert!(canonical(d.clone()).is_err());
        d.follow_ups[0].kind = "appointment".into();
        d.follow_ups[0].time = "24:00".into();
        assert!(canonical(d.clone()).is_err());
        d.follow_ups[0].time.clear();
        d.follow_ups.push(d.follow_ups[0].clone());
        assert!(canonical(d.clone()).is_err());
        d.follow_ups.pop();
        d.intake.employment = "invented".into();
        assert!(canonical(d).is_err());
        let mut d = Data::default();
        d.intake.income_notes = "x".repeat(4001);
        assert!(canonical(d).is_err());
        let d = Data {
            follow_ups: (0..101).map(|_| follow("2026-10-15", "open")).collect(),
            ..Default::default()
        };
        assert!(canonical(d).is_err());
    }
}
