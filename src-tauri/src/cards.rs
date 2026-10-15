//! Supervised card liability ledger. Positive amounts increase debt; no cash projections.
use crate::budget::money;
use crate::imports::{self, Mapping, Raw, Status};
use serde::{Deserialize, Serialize};
pub const MAX_BYTES: usize = 256 * 1024;
pub const MAX_ROWS: usize = 200;
pub const MAX_JSON: usize = 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Unclassified,
    Purchase,
    Refund,
    Payment,
    Fee,
    Interest,
    Adjustment,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fields {
    pub date: Option<String>,
    pub amount: Option<String>,
    pub description: String,
    pub category: String,
    pub kind: Kind,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub account_label: String,
    pub period_start: String,
    pub period_end: String,
    pub opening_owed: String,
    pub closing_owed: String,
    pub retain_source: bool,
    pub mapping: Mapping,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stage {
    pub expected_revision: i64,
    pub token: String,
    pub input: Input,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewInput {
    pub expected_revision: i64,
    pub statement_id: String,
    pub row: usize,
    pub fields: Fields,
    pub status: Status,
    pub reason: String,
    pub acknowledge: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub format: u32,
    pub id: String,
    pub client_id: String,
    pub imported_at: String,
    pub import_revision: i64,
    pub file_name: String,
    pub sha256: String,
    pub byte_count: usize,
    pub input: Input,
    pub rows: Vec<Raw>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub row: usize,
    pub raw: Raw,
    pub fields: Fields,
    pub status: Status,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub row: usize,
    pub fields: Fields,
    pub status: Status,
    pub reason: String,
    pub reviewed_at: String,
    pub counselor_name: String,
    pub revision: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub approved: usize,
    pub rejected: usize,
    pub pending: usize,
    pub clarification: usize,
    pub purchases: String,
    pub refunds: String,
    pub payments: String,
    pub fees: String,
    pub interest: String,
    pub adjustments: String,
    pub net_spending: String,
    pub liability_movement: String,
    pub source_closing: Option<String>,
    pub source_difference: Option<String>,
    pub approved_closing: Option<String>,
    pub approved_difference: Option<String>,
    pub reconciled: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Statement {
    pub source: Source,
    pub rows: Vec<Row>,
    pub history: Vec<Review>,
    pub summary: Summary,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub client_id: String,
    pub revision: i64,
    pub statements: Vec<Statement>,
}
pub fn text(s: &str, max: usize, required: bool) -> Result<String, String> {
    let v = s.trim();
    if (required && v.is_empty()) || v.chars().count() > max || v.chars().any(char::is_control) {
        return Err("Enter bounded plain text without control characters.".into());
    }
    Ok(v.into())
}
pub fn canonical(mut i: Input) -> Result<Input, String> {
    i.account_label = text(&i.account_label, 120, true)?;
    let start = imports::iso_date(&i.period_start)?;
    let end = imports::iso_date(&i.period_end)?;
    if start > end || (end - start).num_days() > 366 {
        return Err("Use an ordered card statement period of at most 367 days.".into());
    }
    i.opening_owed = money(imports::signed_money(&i.opening_owed)?);
    i.closing_owed = money(imports::signed_money(&i.closing_owed)?);
    let m = &i.mapping;
    if m.amount_mode != imports::AmountMode::Signed
        || m.debit.is_some()
        || m.credit.is_some()
        || m.balance.is_some()
        || m.kind.is_some()
        || m.merchant.is_some()
        || m.reference.is_some()
    {
        return Err("Card CSV review supports a single signed amount, date, description and optional category. Classify every row individually.".into());
    }
    imports::validate_mapping(&i.mapping, 64)?;
    Ok(i)
}
pub fn initial(raw: &Raw) -> Fields {
    Fields {
        date: raw.extracted.date.clone(),
        amount: raw.extracted.amount.clone(),
        description: raw.extracted.description.clone(),
        category: raw.extracted.category.clone(),
        kind: Kind::Unclassified,
    }
}
pub fn decision(
    mut f: Fields,
    status: &Status,
    i: &Input,
    reason: &str,
    ack: bool,
) -> Result<(Fields, String), String> {
    f.description = text(&f.description, 500, false)?;
    f.category = text(&f.category, 80, false)?;
    if let Some(d) = &f.date {
        imports::iso_date(d)?;
    }
    if let Some(a) = &f.amount {
        f.amount = Some(money(imports::signed_money(a)?));
    }
    let reason = text(reason, 500, true)?;
    if !ack {
        return Err("Inspect the original card row, sign, possible duplicates and classification, then confirm explicitly.".into());
    }
    if *status == Status::Approved {
        let d = f
            .date
            .as_deref()
            .ok_or("A verified card transaction date is required.")?;
        let n = imports::signed_money(
            f.amount
                .as_deref()
                .ok_or("A verified card amount is required.")?,
        )?;
        if d < i.period_start.as_str() || d > i.period_end.as_str() {
            return Err(
                "Approved card rows must fall inside the declared statement period.".into(),
            );
        }
        let sign = match f.kind {
            Kind::Purchase | Kind::Fee | Kind::Interest => n > 0,
            Kind::Refund | Kind::Payment => n < 0,
            Kind::Adjustment => n != 0,
            Kind::Unclassified => false,
        };
        if !sign || f.description.is_empty() || f.category.is_empty() {
            return Err("Verify the description/category and card type: charges increase debt; refunds/payments reduce debt. Zero and unclassified rows cannot be approved.".into());
        }
    }
    Ok((f, reason))
}
pub fn summary(s: &Source, rows: &[Row]) -> Result<Summary, String> {
    let (mut approved, mut rejected, mut pending, mut clarification) = (0, 0, 0, 0);
    let (
        mut purchases,
        mut refunds,
        mut payments,
        mut fees,
        mut interest,
        mut adjustments,
        mut movement,
    ) = (0, 0, 0, 0, 0, 0, 0);
    for r in rows {
        match r.status {
            Status::Pending => pending += 1,
            Status::Rejected => rejected += 1,
            Status::NeedsClarification => clarification += 1,
            Status::Approved => {
                approved += 1;
                let n = imports::signed_money(
                    r.fields
                        .amount
                        .as_deref()
                        .ok_or("Missing approved amount.")?,
                )?;
                movement += n;
                match r.fields.kind {
                    Kind::Purchase => purchases += n,
                    Kind::Refund => refunds -= n,
                    Kind::Payment => payments -= n,
                    Kind::Fee => fees += n,
                    Kind::Interest => interest += n,
                    Kind::Adjustment => adjustments += n,
                    Kind::Unclassified => return Err("Invalid approved card row.".into()),
                }
            }
        }
    }
    let opening = imports::signed_money(&s.input.opening_owed)?;
    let closing = imports::signed_money(&s.input.closing_owed)?;
    let original = s
        .rows
        .iter()
        .map(|r| {
            r.extracted
                .amount
                .as_deref()
                .and_then(|v| imports::signed_money(v).ok())
        })
        .sum::<Option<i64>>();
    let source_closing = original.map(|n| opening + n);
    let source_difference = source_closing.map(|n| n - closing);
    let approved_closing = if pending + clarification == 0 {
        Some(opening + movement)
    } else {
        None
    };
    let approved_difference = approved_closing.map(|n| n - closing);
    Ok(Summary {
        approved,
        rejected,
        pending,
        clarification,
        purchases: money(purchases),
        refunds: money(refunds),
        payments: money(payments),
        fees: money(fees),
        interest: money(interest),
        adjustments: money(adjustments),
        net_spending: money(purchases + fees + interest - refunds),
        liability_movement: money(movement),
        source_closing: source_closing.map(money),
        source_difference: source_difference.map(money),
        approved_closing: approved_closing.map(money),
        approved_difference: approved_difference.map(money),
        reconciled: source_difference == Some(0) && approved_difference == Some(0),
    })
}
