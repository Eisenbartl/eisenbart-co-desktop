//! Explicit local counselor rules; never automatic financial decisions.
use crate::imports::{Fields, Kind};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Merchant,
    Description,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Data {
    pub label: String,
    pub account_id: String,
    pub kind: Kind,
    pub mode: Mode,
    pub pattern: String,
    pub category: String,
    pub archived: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Version {
    pub id: String,
    pub revision: i64,
    pub data: Data,
    pub reason: String,
    pub updated_at: String,
    pub counselor_name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub id: Option<String>,
    pub expected_revision: i64,
    pub expected_fingerprint: String,
    pub data: Data,
    pub reason: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub rule_id: String,
    pub revision: i64,
    pub expected_snapshot: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Decision {
    pub selection: Selection,
    pub decision: crate::imports::ReviewInput,
}
pub fn text(value: &str, label: &str, max: usize) -> Result<String, String> {
    let v = value.trim();
    if v.is_empty() || v.chars().count() > max || v.chars().any(char::is_control) {
        return Err(format!(
            "{label} is required, must fit {max} characters and cannot contain controls."
        ));
    }
    Ok(v.into())
}
pub fn canonical(mut d: Data) -> Result<Data, String> {
    d.label = text(&d.label, "Rule label", 120)?;
    d.pattern = text(&d.pattern, "Exact rule match text", 120)?;
    d.category = text(&d.category, "Rule category", 80)?;
    if !matches!(d.kind, Kind::Expense | Kind::Fee) || uuid::Uuid::parse_str(&d.account_id).is_err()
    {
        return Err("Choose a scoped bank account and an expense or fee type.".into());
    }
    Ok(d)
}
pub fn key(d: &Data) -> Result<String, String> {
    let f = Fields {
        date: None,
        amount: None,
        description: if d.mode == Mode::Description {
            d.pattern.clone()
        } else {
            String::new()
        },
        merchant: if d.mode == Mode::Merchant {
            d.pattern.clone()
        } else {
            String::new()
        },
        category: String::new(),
        kind: d.kind.clone(),
    };
    Ok(crate::recurring::merchant_identity(&d.account_id, &f)?.0)
}
pub fn matches(d: &Data, account: &str, f: &Fields) -> Result<bool, String> {
    Ok(!d.archived && key(d)? == crate::recurring::merchant_identity(account, f)?.0)
}
