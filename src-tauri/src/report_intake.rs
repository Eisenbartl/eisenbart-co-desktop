//! Explicitly selected, counselor-entered context; never financial evidence.
use crate::client_care::{self, Data, Intake};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub expected_revision: i64,
    pub sections: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub client_id: String,
    pub revision: i64,
    pub updated_at: Option<String>,
    pub values: Intake,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub client_id: String,
    pub revision: i64,
    pub updated_at: String,
    pub sections: Vec<String>,
    pub values: Intake,
}

pub fn sections(v: Vec<String>) -> Result<Vec<String>, String> {
    if v.is_empty()
        || v.len() > 3
        || v.iter()
            .any(|x| !matches!(x.as_str(), "address" | "household" | "employment"))
        || v.iter().collect::<std::collections::HashSet<_>>().len() != v.len()
    {
        return Err("Select one or more distinct intake sections to include.".into());
    }
    Ok(["address", "household", "employment"]
        .into_iter()
        .filter(|s| v.iter().any(|v| v == s))
        .map(str::to_owned)
        .collect())
}
pub fn selected(v: &Intake, s: &[String]) -> Intake {
    let mut out = Intake::default();
    if s.iter().any(|s| s == "address") {
        out.address = v.address.clone();
    }
    if s.iter().any(|s| s == "household") {
        out.household = v.household.clone();
    }
    if s.iter().any(|s| s == "employment") {
        out.employment = v.employment.clone();
        out.employer = v.employer.clone();
        out.occupation = v.occupation.clone();
        out.income_notes = v.income_notes.clone();
    }
    out
}
pub fn capture(p: Preview, s: Selection) -> Result<Snapshot, String> {
    if p.revision < 1 || p.updated_at.is_none() {
        return Err("Save this client's intake before including it in a report.".into());
    }
    if s.expected_revision != p.revision {
        return Err("Intake changed since report review. Review intake again and confirm your selection; nothing was saved.".into());
    }
    let sections = sections(s.sections)?;
    let r = Snapshot {
        client_id: p.client_id,
        revision: p.revision,
        updated_at: p.updated_at.unwrap(),
        values: selected(&p.values, &sections),
        sections,
    };
    validate(&r, &r.client_id)?;
    Ok(r)
}
pub fn validate(s: &Snapshot, client: &str) -> Result<(), String> {
    let bad = || "The saved intake context is invalid or unsupported.".to_owned();
    let values = client_care::canonical(Data {
        intake: s.values.clone(),
        follow_ups: vec![],
    })
    .map_err(|_| bad())?
    .intake;
    if s.client_id != client
        || !(1..=500).contains(&s.revision)
        || chrono::DateTime::parse_from_rfc3339(&s.updated_at).is_err()
        || sections(s.sections.clone())? != s.sections
        || values != s.values
        || selected(&s.values, &s.sections) != s.values
    {
        return Err(bad());
    }
    Ok(())
}
pub fn employment_label(v: &str) -> &str {
    match v {
        "employed" => "Employed",
        "selfEmployed" => "Self-employed",
        "unemployed" => "Unemployed",
        "retired" => "Retired",
        "student" => "Student",
        "other" => "Other",
        _ => "Not recorded",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_context_omits_unselected_fields_and_rejects_invalid_bindings() {
        let intake = Intake {
            address: "Private address excluded".into(),
            household: "Two fictional adults".into(),
            ..Intake::default()
        };
        let preview = || Preview {
            client_id: "client".into(),
            revision: 2,
            updated_at: Some("2026-10-15T12:00:00Z".into()),
            values: intake.clone(),
        };
        let s = capture(
            preview(),
            Selection {
                expected_revision: 2,
                sections: vec!["household".into()],
            },
        )
        .unwrap();
        assert!(s.values.address.is_empty());
        assert_eq!(s.values.household, intake.household);
        assert!(validate(&s, "another client").is_err());
        let mut forged = s.clone();
        forged.values.address = "Unselected leak".into();
        assert!(validate(&forged, "client").is_err());
        assert!(capture(
            preview(),
            Selection {
                expected_revision: 1,
                sections: vec!["household".into()]
            }
        )
        .is_err());
        assert!(sections(vec!["household".into(), "household".into()]).is_err());
        assert!(sections(vec!["followUps".into()]).is_err());
    }
}
