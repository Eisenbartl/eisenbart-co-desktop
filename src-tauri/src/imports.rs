//! Bounded explicit CSV extraction and exact approved-data arithmetic.
use crate::budget::{money, parse_money};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_ROWS: usize = 2000;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Unclassified,
    Income,
    Expense,
    Transfer,
    Refund,
    Fee,
    Interest,
    BalanceMovement,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Status {
    Pending,
    #[serde(rename = "Needs Clarification")]
    NeedsClarification,
    Approved,
    Rejected,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DateFormat {
    Iso,
    Mdy,
    Dmy,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AmountMode {
    Signed,
    DebitCredit,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mapping {
    pub date: usize,
    pub description: usize,
    pub amount_mode: AmountMode,
    pub amount: Option<usize>,
    pub debit: Option<usize>,
    pub credit: Option<usize>,
    pub signed_is_debit: bool,
    pub date_format: DateFormat,
    pub merchant: Option<usize>,
    pub category: Option<usize>,
    pub kind: Option<usize>,
    pub reference: Option<usize>,
    pub balance: Option<usize>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fields {
    pub date: Option<String>,
    pub amount: Option<String>,
    pub description: String,
    pub merchant: String,
    pub category: String,
    pub kind: Kind,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Raw {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub reported_line: Option<String>,
    pub extraction_method: String,
    pub record_number: u32,
    pub line: u64,
    pub reported_date: String,
    pub reported_amount: Option<String>,
    pub reported_debit: Option<String>,
    pub reported_credit: Option<String>,
    pub reported_description: String,
    pub reported_merchant: Option<String>,
    pub reported_category: Option<String>,
    pub reported_kind: Option<String>,
    pub reported_reference: Option<String>,
    pub reported_balance: Option<String>,
    pub extracted: Fields,
    pub reference: Option<String>,
    pub balance: Option<String>,
    pub issues: Vec<String>,
}
#[derive(Debug)]
pub struct CsvData {
    pub headers: Vec<String>,
    pub rows: Vec<(u64, Vec<String>)>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentInput {
    #[serde(default)]
    pub balance_order: Option<crate::reconciliation::Order>,
    #[serde(default)]
    pub pdf_selection: Option<Vec<crate::pdf::Locator>>,
    pub account_id: Option<String>,
    pub account_label: String,
    pub account_kind: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub opening_balance: Option<String>,
    pub closing_balance: Option<String>,
    pub retain_source: bool,
    pub mapping: Mapping,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StageInput {
    pub expected_revision: i64,
    pub token: String,
    pub document: DocumentInput,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewInput {
    pub expected_revision: i64,
    pub record_id: String,
    pub status: Status,
    pub fields: Fields,
    pub reason: String,
    pub acknowledge_warnings: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BulkInput {
    pub expected_revision: i64,
    pub record_ids: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairInput {
    pub expected_revision: i64,
    pub left_id: String,
    pub right_id: String,
    pub reason: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub pdf_pages: Option<Vec<crate::pdf::Page>>,
    pub client_id: String,
    pub token: String,
    pub file_name: String,
    pub sha256: String,
    pub byte_count: usize,
    pub headers: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub row_count: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub label: String,
    pub kind: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub balance_history: Vec<crate::reconciliation::Review>,
    pub balance_reconciliation: Option<crate::reconciliation::Report>,
    pub id: String,
    pub account_id: String,
    pub file_name: String,
    pub sha256: String,
    pub byte_count: usize,
    pub imported_at: String,
    pub source_retained: bool,
    pub input: DocumentInput,
    pub row_count: usize,
    pub source_expected_closing: Option<String>,
    pub source_difference: Option<String>,
    pub approved_difference: Option<String>,
    pub warnings: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub document_id: String,
    pub account_id: String,
    pub raw: Raw,
    pub fields: Fields,
    pub status: Status,
    pub linked_id: Option<String>,
    pub reason: String,
    pub warnings: Vec<String>,
    pub duplicate_ids: Vec<String>,
    pub bulk_eligible: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub id: String,
    pub record_id: String,
    pub from_status: Status,
    pub to_status: Status,
    pub fields: Fields,
    pub reason: String,
    pub reviewed_at: String,
    pub counselor_name: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub pending: usize,
    pub needs_clarification: usize,
    pub approved: usize,
    pub rejected: usize,
    pub income: String,
    pub spending: String,
    pub refunds: String,
    pub net_spending: String,
    pub net_cash: String,
    pub account_net: Vec<(String, String)>,
    pub warnings: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub financial_facts: crate::financial_facts::Facts,
    pub merchant_groups: Vec<crate::recurring::Group>,
    pub recurring_history: Vec<crate::recurring::Review>,
    pub client_id: String,
    pub revision: i64,
    pub accounts: Vec<Account>,
    pub documents: Vec<Document>,
    pub records: Vec<Record>,
    pub history: Vec<History>,
    pub summary: Summary,
}

pub fn iso_date(value: &str) -> Result<NaiveDate, String> {
    if value.len() != 10
        || !value.is_ascii()
        || value.as_bytes()[4] != b'-'
        || value.as_bytes()[7] != b'-'
        || !value
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Err("Enter a real date as YYYY-MM-DD.".into());
    }
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| "Enter a real calendar date.".to_owned())?;
    if !(1900..=2100).contains(&date.year()) {
        return Err("Dates must be between 1900 and 2100.".into());
    }
    Ok(date)
}
pub fn signed_money(value: &str) -> Result<i64, String> {
    let text = value.trim();
    let negative = text.starts_with('-');
    let n = parse_money(if negative { &text[1..] } else { text })?;
    Ok(if negative { -n } else { n })
}
pub(crate) fn reported_money(value: &str) -> Result<i64, String> {
    let mut text = value.trim();
    let mut negative = false;
    if text.starts_with('(') && text.ends_with(')') {
        negative = true;
        text = &text[1..text.len() - 1];
    } else if text.starts_with('-') {
        negative = true;
        text = &text[1..];
    } else if text.starts_with('+') {
        text = &text[1..];
    }
    if let Some(rest) = text.strip_prefix('$') {
        text = rest;
    }
    if text.contains(',') {
        if text
            .split_once('.')
            .is_some_and(|(_, fraction)| fraction.contains(','))
        {
            return Err("Invalid comma in decimal fraction.".into());
        }
        let integer = text.split('.').next().unwrap_or("");
        let groups: Vec<_> = integer.split(',').collect();
        if groups[0].is_empty()
            || groups[0].len() > 3
            || !groups
                .iter()
                .enumerate()
                .all(|(i, g)| g.bytes().all(|b| b.is_ascii_digit()) && (i == 0 || g.len() == 3))
        {
            return Err("Invalid grouped USD amount.".into());
        }
    }
    let plain = text.replace(',', "");
    let n = parse_money(&plain)?;
    Ok(if negative { -n } else { n })
}
fn strict_quotes(text: &str) -> Result<(), String> {
    // Start, bare field, quoted field, after closing quote. Escaped quotes reenter quoted state.
    let mut state = 0;
    for b in text.bytes() {
        state = match (state, b) {
            (0, b'"') => 2,
            (0, b',' | b'\r' | b'\n') => 0,
            (0, _) => 1,
            (1, b'"') => {
                return Err("Malformed CSV quoting. Export a standard comma-separated CSV.".into())
            }
            (1, b',' | b'\r' | b'\n') => 0,
            (1, _) => 1,
            (2, b'"') => 3,
            (2, _) => 2,
            (3, b'"') => 2,
            (3, b',' | b'\r' | b'\n') => 0,
            (3, _) => {
                return Err("Malformed CSV quoting. Export a standard comma-separated CSV.".into())
            }
            _ => unreachable!(),
        };
    }
    if state == 2 {
        return Err("The CSV ends inside a quoted field.".into());
    }
    Ok(())
}
pub fn read_csv(bytes: &[u8]) -> Result<CsvData, String> {
    if bytes.len() > MAX_BYTES || bytes.is_empty() {
        return Err("Select a nonempty UTF-8 CSV no larger than 2 MiB.".into());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "The file is not UTF-8. Export a UTF-8 CSV first.".to_owned())?
        .strip_prefix('\u{feff}')
        .unwrap_or(std::str::from_utf8(bytes).expect("checked UTF-8"));
    strict_quotes(text)?;
    let mut reader = csv::ReaderBuilder::new()
        .flexible(false)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| "Unable to read CSV column headings.".to_owned())?
        .iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if headers.is_empty()
        || headers.len() > 64
        || headers
            .iter()
            .any(|h| h.len() > 120 || h.chars().any(char::is_control))
    {
        return Err("Use at most 64 short column headings without control characters.".into());
    }
    // csv-core positions can precede the LF of a CRLF record separator.
    // Locate the actual row start from byte offsets; count physical newlines,
    // including those inside quoted fields, independently of that position.
    let data = text.as_bytes();
    let mut line_starts = vec![0_usize];
    for (i, b) in data.iter().enumerate() {
        if *b == b'\n' || (*b == b'\r' && data.get(i + 1) != Some(&b'\n')) {
            line_starts.push(i + 1);
        }
    }
    let mut rows = Vec::new();
    for result in reader.records() {
        let r = result.map_err(|_| {
            "CSV rows have inconsistent columns or malformed encoding. No rows were imported."
                .to_owned()
        })?;
        if rows.len() >= MAX_ROWS
            || r.iter().any(|v| {
                v.len() > 2048
                    || v.chars()
                        .any(|c| c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
            })
        {
            return Err("Use at most 2,000 rows, with fields no longer than 2,048 bytes and no unsupported controls.".into());
        }
        let mut offset = r.position().map(|p| p.byte() as usize).unwrap_or(0);
        while data.get(offset).is_some_and(|b| matches!(b, b'\r' | b'\n')) {
            offset += 1;
        }
        let physical_line = line_starts.partition_point(|start| *start <= offset) as u64;
        rows.push((physical_line, r.iter().map(str::to_owned).collect()));
    }
    if rows.is_empty() {
        return Err("The CSV contains no transaction rows.".into());
    }
    Ok(CsvData { headers, rows })
}
pub fn validate_mapping(m: &Mapping, columns: usize) -> Result<(), String> {
    let mut indices = vec![m.date, m.description];
    indices.extend(
        [m.merchant, m.category, m.kind, m.reference, m.balance]
            .into_iter()
            .flatten(),
    );
    match m.amount_mode {
        AmountMode::Signed => {
            indices.push(m.amount.ok_or("Select an amount column.")?);
            if m.debit.is_some() || m.credit.is_some() {
                return Err("Signed amounts cannot also map debit/credit columns.".into());
            }
        }
        AmountMode::DebitCredit => {
            indices.push(m.debit.ok_or("Select the debit column.")?);
            indices.push(m.credit.ok_or("Select the credit column.")?);
            if m.amount.is_some() || m.signed_is_debit {
                return Err("Debit/credit mode cannot map a signed amount or invert signs.".into());
            }
        }
    }
    let mut seen = HashSet::new();
    if indices.iter().any(|i| *i >= columns || !seen.insert(*i)) {
        return Err("Choose different existing columns for each mapped field.".into());
    }
    Ok(())
}
pub(crate) fn source_date(value: &str, format: &DateFormat) -> Option<String> {
    let text = value.trim();
    if *format == DateFormat::Iso {
        return iso_date(text).ok().map(|d| d.to_string());
    }
    if !text.is_ascii() {
        return None;
    }
    let parts = text.split('/').collect::<Vec<_>>();
    if parts.len() != 3
        || !(1..=2).contains(&parts[0].len())
        || !(1..=2).contains(&parts[1].len())
        || parts[2].len() != 4
        || parts.iter().any(|p| !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let first = parts[0].parse::<u32>().ok()?;
    let second = parts[1].parse::<u32>().ok()?;
    let year = parts[2].parse::<i32>().ok()?;
    if !(1900..=2100).contains(&year) {
        return None;
    }
    let (month, day) = if *format == DateFormat::Mdy {
        (first, second)
    } else {
        (second, first)
    };
    NaiveDate::from_ymd_opt(year, month, day).map(|d| d.to_string())
}
fn source_kind(value: &str) -> Kind {
    match value.trim().to_ascii_lowercase().as_str() {
        "income" => Kind::Income,
        "expense" => Kind::Expense,
        "transfer" => Kind::Transfer,
        "refund" => Kind::Refund,
        "fee" => Kind::Fee,
        "interest" => Kind::Interest,
        "balance movement" | "balancemovement" => Kind::BalanceMovement,
        _ => Kind::Unclassified,
    }
}
pub fn extract(csv: &CsvData, m: &Mapping) -> Result<Vec<Raw>, String> {
    validate_mapping(m, csv.headers.len())?;
    let mut extracted = Vec::with_capacity(csv.rows.len());
    for (index, (line, row)) in csv.rows.iter().enumerate() {
        let get = |i: Option<usize>| i.map(|i| row[i].clone());
        let date = source_date(&row[m.date], &m.date_format);
        let mut issues = Vec::new();
        if date.is_none() {
            issues.push("Missing or invalid date under the selected date format.".into());
        }
        let amount = match m.amount_mode {
            AmountMode::Signed => reported_money(&row[m.amount.expect("validated mapping")])
                .ok()
                .map(|v| if m.signed_is_debit { -v } else { v }),
            AmountMode::DebitCredit => {
                let debit = row[m.debit.expect("validated mapping")].trim();
                let credit = row[m.credit.expect("validated mapping")].trim();
                let parse = |value: &str| {
                    if value.is_empty() {
                        None
                    } else {
                        reported_money(value).ok()
                    }
                };
                match (parse(debit), parse(credit)) {
                    (Some(d), Some(c)) if d >= 0 && c >= 0 && (d == 0 || c == 0) => Some(c - d),
                    (Some(d), None) if d >= 0 && credit.is_empty() => Some(-d),
                    (None, Some(c)) if c >= 0 && debit.is_empty() => Some(c),
                    _ => None,
                }
            }
        };
        if amount.is_none() {
            issues.push("Missing, ambiguous, or invalid USD amount. Both debit and credit cannot be nonzero.".into());
        }
        if amount == Some(0) {
            issues.push("Zero-amount activity needs individual review.".into());
        }
        let normalized = |value: &str, limit: usize| {
            let text = value.split_whitespace().collect::<Vec<_>>().join(" ");
            if text.chars().count() > limit {
                String::new()
            } else {
                text
            }
        };
        let description = normalized(&row[m.description], 500);
        if description.is_empty() {
            issues.push("Missing or overlong description.".into());
        }
        let merchant = get(m.merchant)
            .map(|v| normalized(&v, 120))
            .unwrap_or_default();
        let category = get(m.category)
            .map(|v| normalized(&v, 80))
            .unwrap_or_default();
        let kind = get(m.kind)
            .map(|v| source_kind(&v))
            .unwrap_or(Kind::Unclassified);
        if kind == Kind::Unclassified {
            issues.push("Transaction type has not been classified.".into());
        }
        if category.is_empty() {
            issues.push("Category has not been assigned.".into());
        }
        for i in [
            Some(m.description),
            m.merchant,
            m.category,
            m.kind,
            m.reference,
        ]
        .into_iter()
        .flatten()
        {
            if row[i].trim_start().starts_with(['=', '+', '@']) {
                issues.push(
                    "Spreadsheet-like active content is treated as text; review individually."
                        .into(),
                );
                break;
            }
        }
        let reference = get(m.reference)
            .filter(|v| !v.trim().is_empty() && v.chars().count() <= 120)
            .map(|v| v.trim().to_owned());
        let balance = get(m.balance)
            .and_then(|v| reported_money(&v).ok())
            .map(money);
        if get(m.balance).is_some_and(|v| !v.trim().is_empty()) && balance.is_none() {
            issues.push(
                "The reported running balance is invalid; it is not used as a verified figure."
                    .into(),
            );
        }
        extracted.push(Raw {
            page: None,
            reported_line: None,
            extraction_method: "csv-utf8-v1".into(),
            record_number: index as u32 + 2,
            line: *line,
            reported_date: row[m.date].clone(),
            reported_amount: get(m.amount),
            reported_debit: get(m.debit),
            reported_credit: get(m.credit),
            reported_description: row[m.description].clone(),
            reported_merchant: get(m.merchant),
            reported_category: get(m.category),
            reported_kind: get(m.kind),
            reported_reference: get(m.reference),
            reported_balance: get(m.balance),
            extracted: Fields {
                date,
                amount: amount.map(money),
                description,
                merchant,
                category,
                kind,
            },
            reference,
            balance,
            issues,
        });
    }
    Ok(extracted)
}
pub fn validate_fields(mut fields: Fields) -> Result<Fields, String> {
    if let Some(date) = &fields.date {
        iso_date(date)?;
    }
    if let Some(amount) = &fields.amount {
        fields.amount = Some(money(signed_money(amount)?));
    }
    for (value, max) in [
        (&mut fields.description, 500),
        (&mut fields.merchant, 120),
        (&mut fields.category, 80),
    ] {
        *value = value.trim().to_owned();
        if value.chars().count() > max || value.chars().any(char::is_control) {
            return Err("Review text exceeds its limit or contains unsupported controls.".into());
        }
    }
    Ok(fields)
}
pub fn approval_error(fields: &Fields) -> Option<String> {
    let Some(date) = &fields.date else {
        return Some("A verified transaction date is required.".into());
    };
    if iso_date(date).is_err() {
        return Some("A real transaction date is required.".into());
    }
    let Some(amount) = fields.amount.as_ref().and_then(|v| signed_money(v).ok()) else {
        return Some("A verified USD amount is required.".into());
    };
    if fields.description.is_empty() || fields.category.is_empty() {
        return Some("Assign a description and category before approval.".into());
    }
    let compatible = match fields.kind {
        Kind::Unclassified => false,
        Kind::Income | Kind::Refund => amount > 0,
        Kind::Expense | Kind::Fee => amount < 0,
        Kind::Transfer => amount != 0,
        Kind::Interest | Kind::BalanceMovement => true,
    };
    if !compatible {
        return Some(
            "Classify the transaction consistently with its deposit/withdrawal sign.".into(),
        );
    }
    None
}
pub fn fingerprint(fields: &Fields) -> Option<String> {
    let (Some(date), Some(amount)) = (&fields.date, &fields.amount) else {
        return None;
    };
    if fields.description.is_empty() {
        return None;
    }
    Some(
        serde_json::to_string(&(
            date,
            amount,
            fields
                .description
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase(),
        ))
        .expect("string fingerprint"),
    )
}
pub fn summarize(records: &[Record], accounts: &[Account]) -> Result<Summary, String> {
    summarize_selected(&records.iter().collect::<Vec<_>>(), accounts)
}

pub(crate) fn summarize_selected(
    records: &[&Record],
    accounts: &[Account],
) -> Result<Summary, String> {
    let mut s = Summary {
        pending: 0,
        needs_clarification: 0,
        approved: 0,
        rejected: 0,
        income: String::new(),
        spending: String::new(),
        refunds: String::new(),
        net_spending: String::new(),
        net_cash: String::new(),
        account_net: vec![],
        warnings: vec![],
    };
    let (mut income, mut spending, mut refunds, mut net) = (0_i64, 0_i64, 0_i64, 0_i64);
    for r in records {
        match r.status {
            Status::Pending => s.pending += 1,
            Status::NeedsClarification => s.needs_clarification += 1,
            Status::Rejected => s.rejected += 1,
            Status::Approved => {
                s.approved += 1;
                if approval_error(&r.fields).is_some() {
                    return Err("Invalid approved transaction.".into());
                }
                let n = signed_money(r.fields.amount.as_deref().expect("approved amount"))?;
                net += n;
                match r.fields.kind {
                    Kind::Income => income += n,
                    Kind::Expense | Kind::Fee => spending -= n,
                    Kind::Refund => refunds += n,
                    Kind::Interest if n >= 0 => income += n,
                    Kind::Interest => spending -= n,
                    _ => {}
                }
            }
        }
    }
    for a in accounts {
        let amount = records
            .iter()
            .filter(|r| r.account_id == a.id && r.status == Status::Approved)
            .map(|r| signed_money(r.fields.amount.as_deref().expect("approved amount")))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .sum();
        s.account_net.push((a.id.clone(), money(amount)));
    }
    s.income = money(income);
    s.spending = money(spending);
    s.refunds = money(refunds);
    s.net_spending = money(spending - refunds);
    s.net_cash = money(net);
    s.warnings.push("These totals cover approved imported records across all imported dates, not monthly budget estimates. Missing statements, pending/clarification rows, and unverified classifications can make the assessment incomplete.".into());
    if refunds > spending {
        s.warnings.push("Refunds exceed approved spending in these records. Net spending is negative; the difference is not treated as income.".into());
    }
    Ok(s)
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn mapping() -> Mapping {
        Mapping {
            date: 0,
            description: 1,
            amount_mode: AmountMode::Signed,
            amount: Some(2),
            debit: None,
            credit: None,
            signed_is_debit: false,
            date_format: DateFormat::Iso,
            merchant: None,
            category: Some(3),
            kind: Some(4),
            reference: None,
            balance: None,
        }
    }
    #[test]
    fn exact_money_grouping_and_signs_without_float_or_guessing() {
        for (input, expected) in [
            ("$1,234.56", 123456),
            ("($50.01)", -5001),
            ("-0.01", -1),
            ("+12.3", 1230),
        ] {
            assert_eq!(reported_money(input).unwrap(), expected);
        }
        for v in [
            "1,23.45",
            "1,234.5,6",
            "1e3",
            "1.001",
            "--1",
            "-($2)",
            "1000000000.01",
        ] {
            assert!(reported_money(v).is_err());
        }
    }
    #[test]
    fn quoted_bom_multiline_provenance_and_active_text_are_preserved() {
        let csv=read_csv(b"\xef\xbb\xbfDate,Description,Amount,Category,Type\r\n2028-02-29,\"Grocery, first\r\nsecond\",-12.01,Food,expense\r\n2028-03-01,=SUM(A1),10,Other,income\r\n").unwrap();
        let rows = extract(&csv, &mapping()).unwrap();
        assert_eq!(rows[0].line, 2);
        assert_eq!(rows[1].line, 4);
        assert_eq!(rows[0].record_number, 2);
        assert_eq!(rows[0].extracted.description, "Grocery, first second");
        assert_eq!(rows[0].reported_description, "Grocery, first\r\nsecond");
        assert_eq!(rows[0].extracted.amount.as_deref(), Some("-12.01"));
        assert!(rows[1].issues.iter().any(|v| v.contains("active content")));
    }
    #[test]
    fn malformed_encoding_structure_resource_limits_and_mappings_fail() {
        for v in [
            &b"A,B\n1\n"[..],
            &b"A,B\n1,\"unterminated"[..],
            &b"A,B\n1,a\"b\n"[..],
            &b"A,B\n1,\"a\"x\n"[..],
            &b"\xff\xfe\n"[..],
        ] {
            assert!(read_csv(v).is_err());
        }
        assert!(read_csv(&vec![b'a'; MAX_BYTES + 1]).is_err());
        let csv = read_csv(b"D,T,A,C,K\n2026-01-01,Test,1,C,income\n").unwrap();
        let mut m = mapping();
        m.date = m.description;
        assert!(extract(&csv, &m).is_err());
        let mut m = mapping();
        m.amount = Some(99);
        assert!(extract(&csv, &m).is_err());
    }
    #[test]
    fn explicit_dates_missing_values_and_debit_credit_conflicts_stay_uncertain() {
        let csv = read_csv(b"D,T,A,C,K\n03/04/2026,Test,,C,income\n2027-02-29,Test,1,C,income\n")
            .unwrap();
        let r = extract(&csv, &mapping()).unwrap();
        assert!(r[0].extracted.date.is_none());
        assert!(r[0].extracted.amount.is_none());
        let mut m = mapping();
        m.date_format = DateFormat::Dmy;
        assert_eq!(
            extract(&csv, &m).unwrap()[0].extracted.date.as_deref(),
            Some("2026-04-03")
        );
        m.date_format = DateFormat::Mdy;
        assert_eq!(
            extract(&csv, &m).unwrap()[0].extracted.date.as_deref(),
            Some("2026-03-04")
        );
        let csv=read_csv(b"D,T,Debit,Credit\n2026-01-01,A,12.01,\n2026-01-02,B,,2.02\n2026-01-03,C,,\n2026-01-04,D,1,2\n").unwrap();
        m = mapping();
        m.category = None;
        m.kind = None;
        m.amount_mode = AmountMode::DebitCredit;
        m.amount = None;
        m.debit = Some(2);
        m.credit = Some(3);
        let r = extract(&csv, &m).unwrap();
        assert_eq!(r[0].extracted.amount.as_deref(), Some("-12.01"));
        assert_eq!(r[1].extracted.amount.as_deref(), Some("2.02"));
        assert!(r[2].extracted.amount.is_none());
        assert!(r[3].extracted.amount.is_none());
    }
    #[test]
    fn signed_polarity_and_classification_are_explicit() {
        let csv = read_csv(b"D,T,A,C,K\n2026-01-01,Pay,100,Wages,income\n").unwrap();
        let mut m = mapping();
        m.signed_is_debit = true;
        let r = extract(&csv, &m).unwrap();
        assert_eq!(r[0].extracted.amount.as_deref(), Some("-100.00"));
        assert!(approval_error(&r[0].extracted).is_some());
        let mut f = r[0].extracted.clone();
        f.kind = Kind::Expense;
        assert!(approval_error(&f).is_none());
        f.amount = None;
        assert!(approval_error(&f).is_some());
    }
}

#[cfg(test)]
mod totals_tests {
    use super::*;
    #[test]
    fn independent_approved_arithmetic_excludes_pending_rejected_movements_and_offsets_refunds() {
        let source="D,T,A,C,K\n2026-09-01,Wages,1000,Test,income\n2026-09-02,Rent,-120,Test,expense\n2026-09-03,Refund,20,Test,refund\n2026-09-04,Fee,-0.01,Test,fee\n2026-09-05,Interest charged,-3.33,Test,interest\n2026-09-06,Interest earned,2.22,Test,interest\n2026-09-07,Principal,-200,Test,balance movement\n2026-09-08,Transfer out,-50,Test,transfer\n2026-09-09,Transfer in,50,Test,transfer\n2026-09-10,Pending income,9999,Test,income\n2026-09-11,Rejected income,8888,Test,income\n2026-09-12,Clarification expense,-7777,Test,expense\n";
        let rows = extract(&read_csv(source.as_bytes()).unwrap(), &tests::mapping()).unwrap();
        let records = rows
            .into_iter()
            .enumerate()
            .map(|(i, raw)| Record {
                id: format!("row-{i}"),
                document_id: "doc".into(),
                account_id: if i == 8 { "savings" } else { "checking" }.into(),
                fields: raw.extracted.clone(),
                raw,
                status: match i {
                    9 => Status::Pending,
                    10 => Status::Rejected,
                    11 => Status::NeedsClarification,
                    _ => Status::Approved,
                },
                linked_id: match i {
                    7 => Some("row-8".into()),
                    8 => Some("row-7".into()),
                    _ => None,
                },
                reason: String::new(),
                warnings: vec![],
                duplicate_ids: vec![],
                bulk_eligible: false,
            })
            .collect::<Vec<_>>();
        let accounts = vec![
            Account {
                id: "checking".into(),
                label: "Checking".into(),
                kind: "checking".into(),
            },
            Account {
                id: "savings".into(),
                label: "Savings".into(),
                kind: "savings".into(),
            },
        ];
        let s = summarize(&records, &accounts).unwrap();
        assert_eq!(s.income, "1002.22");
        assert_eq!(s.spending, "123.34");
        assert_eq!(s.refunds, "20.00");
        assert_eq!(s.net_spending, "103.34");
        assert_eq!(s.net_cash, "698.88");
        assert_eq!(s.account_net[0].1, "648.88");
        assert_eq!(s.account_net[1].1, "50.00");
        assert_eq!(
            (s.approved, s.pending, s.rejected, s.needs_clarification),
            (9, 1, 1, 1)
        );
    }
}
