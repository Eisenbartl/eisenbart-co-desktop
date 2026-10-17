//! Current transaction projection and supplemental inspection details; original ledgers own money.
use super::*;
use crate::{
    budget::money,
    cards,
    imports::{self, Status},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
const PAGE: usize = 25;
const MAX_DETAILS: usize = 10000;
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Query {
    pub search: String,
    pub from: Option<String>,
    pub through: Option<String>,
    pub account_type: Option<String>,
    pub institution: Option<String>,
    pub account_id: Option<String>,
    pub source_id: Option<String>,
    pub kind: Option<String>,
    pub method: Option<String>,
    pub category: Option<String>,
    pub direction: Option<String>,
    pub minimum: Option<String>,
    pub maximum: Option<String>,
    pub status: Option<Status>,
    pub reconciliation: Option<String>,
    pub oldest_first: bool,
    pub posting_date: bool,
    pub page: usize,
    pub context: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "target", rename_all = "camelCase", deny_unknown_fields)]
pub enum Data {
    Account {
        #[serde(rename = "accountId")]
        account_id: String,
        institution: String,
        #[serde(rename = "lastFour")]
        last_four: String,
        subtype: String,
    },
    Transaction {
        #[serde(rename = "rowId")]
        row_id: String,
        binding: String,
        #[serde(rename = "postingDate")]
        posting_date: Option<String>,
        method: String,
        #[serde(rename = "confirmedDuplicate")]
        confirmed_duplicate: bool,
    },
}
impl Data {
    fn key(&self) -> String {
        match self {
            Self::Account { account_id, .. } => format!("account:{account_id}"),
            Self::Transaction { row_id, .. } => row_id.clone(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DetailEvent {
    pub revision: usize,
    pub data: Data,
    pub reason: String,
    pub at: String,
    pub counselor: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Save {
    pub context: String,
    pub expected_revision: i64,
    pub data: Data,
    pub reason: String,
    pub acknowledge: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub kind: String,
    pub file_name: String,
    pub account_id: String,
    pub account_label: String,
    pub account_type: String,
    pub institution: String,
    pub masked_identifier: Option<String>,
    pub retained: bool,
    pub pdf: bool,
    pub sha256: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub opening_balance: Option<String>,
    pub closing_balance: Option<String>,
    pub reconciliation: String,
    pub warnings: Vec<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub record_id: String,
    pub source_id: String,
    pub card_row: Option<usize>,
    pub binding: String,
    pub date: Option<String>,
    pub posting_date: Option<String>,
    pub description: String,
    pub merchant: String,
    pub category: String,
    pub kind: String,
    pub method: String,
    pub amount: Option<String>,
    pub original_amount: Option<String>,
    pub currency: String,
    pub direction: String,
    pub status: Status,
    pub page: Option<u32>,
    pub line: u64,
    pub warnings: Vec<String>,
    pub duplicate: bool,
    pub confirmed_duplicate: bool,
    pub purpose: String,
    #[serde(skip)]
    purchase: bool,
    #[serde(skip)]
    eligible: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodTotal {
    pub method: String,
    pub amount: String,
    pub count: usize,
    pub average: Option<String>,
    pub share: Option<String>,
    pub basis_points: u16,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub methods: Vec<MethodTotal>,
    pub purchase_total: String,
    pub classified: String,
    pub unclassified: String,
    pub unclassified_share: Option<String>,
    pub excluded: usize,
    pub observations: Vec<String>,
    pub categories: Vec<(String, String)>,
    pub prior_methods: Vec<MethodTotal>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub client_id: String,
    pub context: String,
    pub revision: i64,
    pub query: Query,
    pub total: usize,
    pub page_size: usize,
    pub sources: Vec<Source>,
    pub categories: Vec<String>,
    pub kinds: Vec<String>,
    pub rows: Vec<Row>,
    pub analysis: Analysis,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub client_id: String,
    pub context: String,
    pub revision: i64,
    pub row: Row,
    pub source: Source,
    pub original: imports::Raw,
    pub metadata_history: Vec<DetailEvent>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Original {
    pub client_id: String,
    pub source_id: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}
struct State {
    bank: imports::Snapshot,
    cards: cards::Snapshot,
    sources: Vec<Source>,
    rows: Vec<Row>,
    history: Vec<DetailEvent>,
    revision: i64,
    token: String,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn json<T: Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|_| STORAGE_ERROR.into())
}
fn label<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .expect("enum")
        .as_str()
        .expect("enum string")
        .into()
}
fn canonical(mut d: Data) -> Result<Data, String> {
    match &mut d {
        Data::Account {
            account_id,
            institution,
            last_four,
            subtype,
        } => {
            *account_id = cards::text(account_id, 100, true)?;
            *institution = cards::text(institution, 100, false)?;
            if !last_four.is_empty()
                && (last_four.len() != 4 || !last_four.bytes().all(|b| b.is_ascii_digit()))
            {
                return Err("Enter only the last four account digits, or leave unknown.".into());
            }
            if !matches!(
                subtype.as_str(),
                "checking" | "savings" | "moneyMarket" | "otherDeposit" | "creditCard"
            ) {
                return Err("Use a supported deposit or credit-card account type. Loan ledgers require their own statement adapter.".into());
            }
        }
        Data::Transaction {
            row_id,
            binding,
            posting_date,
            method,
            ..
        } => {
            *row_id = cards::text(row_id, 100, true)?;
            *binding = cards::text(binding, 100, true)?;
            if let Some(d) = posting_date {
                imports::iso_date(d)?;
            }
            if !METHODS.contains(&method.as_str()) {
                return Err("Choose a supported transaction method, or leave unclassified.".into());
            }
        }
    }
    Ok(d)
}
const METHODS: &[&str] = &[
    "unclassified",
    "debitCard",
    "creditCard",
    "check",
    "achDebit",
    "achCredit",
    "directDeposit",
    "cashWithdrawal",
    "cashDeposit",
    "internalTransfer",
    "externalTransfer",
    "cardPayment",
    "loanPayment",
    "interestEarned",
    "interestCharged",
    "fee",
    "overdraftFee",
    "nsfFee",
    "refund",
    "reversal",
    "otherDeposit",
    "otherWithdrawal",
];
fn history(c: &Connection, w: &WorkspaceInfo, id: &str) -> Result<Vec<DetailEvent>, String> {
    let mut q=c.prepare("SELECT revision,payload_json,digest FROM transaction_inspection_details WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision LIMIT 10001").map_err(storage_error)?;
    let mut out = Vec::new();
    let mut previous = String::new();
    for item in q
        .query_map(params![id, w.organization_id, w.counselor_id], |r| {
            Ok((
                r.get::<_, i64>(0)? as usize,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(storage_error)?
    {
        let (rev, body, digest) = item.map_err(storage_error)?;
        let event: DetailEvent = serde_json::from_str(&body).map_err(|_| STORAGE_ERROR)?;
        if out.len() >= MAX_DETAILS
            || body.len() > 8192
            || rev != out.len() + 1
            || event.revision != rev
            || canonical(event.data.clone())? != event.data
            || cards::text(&event.reason, 500, true)? != event.reason
            || cards::text(&event.counselor, 200, true)? != event.counselor
            || chrono::DateTime::parse_from_rfc3339(&event.at).is_err()
            || json(&event)? != body
            || hash(format!("{previous}\n{body}").as_bytes()) != digest
        {
            return Err(STORAGE_ERROR.into());
        }
        previous = digest;
        out.push(event);
    }
    Ok(out)
}
fn source_state(d: &imports::Document, bank: &imports::Snapshot) -> String {
    if d.balance_reconciliation
        .as_ref()
        .is_some_and(crate::reconciliation::Report::conflicts)
    {
        return "difference".into();
    }
    if d.source_difference.as_deref().is_some_and(|v| v != "0.00")
        || d.approved_difference
            .as_deref()
            .is_some_and(|v| v != "0.00")
    {
        return "difference".into();
    }
    if d.approved_difference.as_deref() == Some("0.00")
        && bank
            .records
            .iter()
            .filter(|r| r.document_id == d.id)
            .all(|r| r.status == Status::Approved)
    {
        "matched"
    } else {
        "unknown"
    }
    .into()
}
fn state(c: &Connection, w: &WorkspaceInfo, id: &str) -> Result<State, String> {
    scoped_client(c, w, id)?;
    let bank = imports_store::read_imports(c, w, id)?;
    let cards = cards_store::read(c, w, id)?;
    let accounts = statement_accounts_store::read(c, w, id)?;
    let links = card_review_store::history(c, w, id)?.pop();
    let history = history(c, w, id)?;
    let latest = history
        .iter()
        .map(|h| (h.data.key(), &h.data))
        .collect::<BTreeMap<_, _>>();
    let mut sources = Vec::new();
    let mut rows = Vec::new();
    for d in &bank.documents {
        let a = accounts
            .accounts
            .iter()
            .find(|a| a.id == d.account_id)
            .ok_or(STORAGE_ERROR)?;
        sources.push(Source {
            id: d.id.clone(),
            kind: "bank".into(),
            file_name: d.file_name.clone(),
            account_id: format!("bank:{}", a.id),
            account_label: a.label.clone(),
            account_type: a.kind.clone(),
            institution: String::new(),
            masked_identifier: None,
            retained: d.source_retained,
            pdf: d.input.pdf_selection.is_some(),
            sha256: d.sha256.clone(),
            period_start: d.input.period_start.clone(),
            period_end: d.input.period_end.clone(),
            opening_balance: d.input.opening_balance.clone(),
            closing_balance: d.input.closing_balance.clone(),
            reconciliation: source_state(d, &bank),
            warnings: d.warnings.clone(),
        });
    }
    for s in &cards.statements {
        let a = links.as_ref().and_then(|l| {
            l.assignments
                .iter()
                .find(|a| a.statement_id == s.source.id)
                .and_then(|a| l.accounts.iter().find(|b| b.id == a.account_id))
        });
        sources.push(Source {
            id: s.source.id.clone(),
            kind: "card".into(),
            file_name: s.source.file_name.clone(),
            account_id: a.map_or_else(
                || format!("card-source:{}", s.source.id),
                |a| format!("card:{}", a.id),
            ),
            account_label: a.map_or_else(
                || format!("{} · unassigned source", s.source.input.account_label),
                |a| a.label.clone(),
            ),
            account_type: "creditCard".into(),
            institution: String::new(),
            masked_identifier: None,
            retained: s.source.input.retain_source,
            pdf: s.source.input.pdf.is_some(),
            sha256: s.source.sha256.clone(),
            period_start: Some(s.source.input.period_start.clone()),
            period_end: Some(s.source.input.period_end.clone()),
            opening_balance: Some(s.source.input.opening_owed.clone()),
            closing_balance: Some(s.source.input.closing_owed.clone()),
            reconciliation: if s.summary.reconciled {
                "matched"
            } else if s
                .summary
                .source_difference
                .as_deref()
                .is_some_and(|v| v != "0.00")
            {
                "difference"
            } else {
                "unknown"
            }
            .into(),
            warnings: s.warnings.clone(),
        });
    }
    for s in &mut sources {
        if let Some(Data::Account {
            institution,
            last_four,
            subtype,
            ..
        }) = latest.get(&format!("account:{}", s.account_id))
        {
            s.institution = institution.clone();
            s.masked_identifier = (!last_four.is_empty()).then(|| format!("•••• {last_four}"));
            s.account_type = subtype.clone();
        }
    }
    let eligible = spending_analysis::inspection_rows(c, w, id)?;
    for r in &bank.records {
        let binding = bank
            .history
            .iter()
            .rev()
            .find(|h| h.record_id == r.id)
            .map_or_else(|| format!("import:{}", r.id), |h| h.id.clone());
        let f = &r.fields;
        rows.push(make_row(
            format!("bank:{}", r.id),
            r.id.clone(),
            r.document_id.clone(),
            None,
            binding,
            f.date.clone(),
            f.amount.clone(),
            f.description.clone(),
            f.merchant.clone(),
            f.category.clone(),
            label(&f.kind),
            r.status.clone(),
            &r.raw,
            r.warnings.clone(),
            !r.duplicate_ids.is_empty(),
        )?);
    }
    for s in &cards.statements {
        for r in &s.rows {
            let binding = s.history.iter().rev().find(|h| h.row == r.row).map_or_else(
                || format!("import:{}:{}", s.source.id, r.row),
                |h| h.id.clone(),
            );
            let f = &r.fields;
            let amount = f
                .amount
                .as_deref()
                .map(imports::signed_money)
                .transpose()?
                .map(|n| money(-n));
            rows.push(make_row(
                format!("card:{}:{}", s.source.id, r.row),
                format!("{}:{}", s.source.id, r.row),
                s.source.id.clone(),
                Some(r.row),
                binding,
                f.date.clone(),
                amount,
                f.description.clone(),
                String::new(),
                f.category.clone(),
                label(&f.kind),
                r.status.clone(),
                &r.raw,
                r.raw.issues.clone(),
                false,
            )?);
        }
    }
    for r in &mut rows {
        if let Some((purchase, role)) = eligible.get(&r.id) {
            r.purchase = *purchase;
            r.eligible = true;
            r.purpose = role.clone();
        }
        if r.card_row.is_some()
            && r.status == Status::Approved
            && !r.category.is_empty()
            && !r.eligible
        {
            r.duplicate = true;
        }
        if let Some(Data::Transaction {
            binding,
            posting_date,
            method,
            confirmed_duplicate,
            ..
        }) = latest.get(&r.id)
        {
            if binding == &r.binding {
                r.posting_date = posting_date.clone();
                r.method = method.clone();
                r.confirmed_duplicate = *confirmed_duplicate;
            } else {
                r.warnings.push(
                    "Earlier inspection details refer to an older decision; verify them again."
                        .into(),
                );
            }
        }
    }
    let revision = bank.revision;
    let token = hash(
        json(&(
            id,
            revision,
            links.as_ref().map(|l| l.revision),
            &sources,
            history.len(),
        ))?
        .as_bytes(),
    );
    Ok(State {
        bank,
        cards,
        sources,
        rows,
        history,
        revision,
        token,
    })
}
#[allow(clippy::too_many_arguments)]
fn make_row(
    id: String,
    record_id: String,
    source_id: String,
    card_row: Option<usize>,
    binding: String,
    date: Option<String>,
    amount: Option<String>,
    description: String,
    merchant: String,
    category: String,
    kind: String,
    status: Status,
    raw: &imports::Raw,
    warnings: Vec<String>,
    duplicate: bool,
) -> Result<Row, String> {
    let n = amount.as_deref().map(imports::signed_money).transpose()?;
    let method = if card_row.is_some() && status == Status::Approved {
        match kind.as_str() {
            "purchase" => "creditCard",
            "payment" => "cardPayment",
            "refund" => "refund",
            "interest" => "interestCharged",
            "fee" => "fee",
            _ => "unclassified",
        }
    } else {
        "unclassified"
    }
    .into();
    Ok(Row {
        id,
        record_id,
        source_id,
        card_row,
        binding,
        date,
        posting_date: None,
        description,
        merchant,
        category,
        kind,
        method,
        amount,
        original_amount: raw.extracted.amount.clone(),
        currency: "USD".into(),
        direction: n
            .map_or("unknown", |n| {
                if n < 0 {
                    "debit"
                } else if n > 0 {
                    "credit"
                } else {
                    "zero"
                }
            })
            .into(),
        status,
        page: raw.page,
        line: raw.line,
        warnings,
        duplicate,
        confirmed_duplicate: false,
        purpose: "unclassified".into(),
        purchase: false,
        eligible: false,
    })
}
fn validate_query(q: &mut Query) -> Result<(), String> {
    q.search = cards::text(&q.search, 200, false)?;
    for d in [&q.from, &q.through].into_iter().flatten() {
        imports::iso_date(d)?;
    }
    for s in [
        &q.institution,
        &q.account_id,
        &q.source_id,
        &q.kind,
        &q.method,
        &q.category,
    ]
    .into_iter()
    .flatten()
    {
        cards::text(s, 120, true)?;
    }
    if q.from
        .as_ref()
        .zip(q.through.as_ref())
        .is_some_and(|(a, b)| a > b)
        || q.page > 400
        || q.page > 0 && q.context.is_none()
        || q.context
            .as_ref()
            .is_some_and(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        || q.direction
            .as_deref()
            .is_some_and(|v| !matches!(v, "debit" | "credit" | "zero" | "unknown"))
        || q.reconciliation.as_deref().is_some_and(|v| {
            !matches!(
                v,
                "matched" | "difference" | "unknown" | "suspectedDuplicate" | "confirmedDuplicate"
            )
        })
        || q.account_type.as_deref().is_some_and(|v| {
            !matches!(
                v,
                "checking"
                    | "savings"
                    | "moneyMarket"
                    | "creditCard"
                    | "loanCreditLine"
                    | "otherDeposit"
            )
        })
    {
        return Err("Use valid combined filters and restart the search before paging.".into());
    }
    let lo = q
        .minimum
        .as_deref()
        .map(imports::signed_money)
        .transpose()?;
    let hi = q
        .maximum
        .as_deref()
        .map(imports::signed_money)
        .transpose()?;
    if lo.is_some_and(|v| v < 0)
        || hi.is_some_and(|v| v < 0)
        || lo.zip(hi).is_some_and(|(a, b)| a > b)
    {
        return Err("Use a nonnegative, ordered absolute amount range.".into());
    }
    Ok(())
}
fn matches(r: &Row, s: &Source, q: &Query) -> Result<bool, String> {
    let date = if q.posting_date {
        &r.posting_date
    } else {
        &r.date
    };
    let n = r
        .amount
        .as_deref()
        .map(imports::signed_money)
        .transpose()?
        .map(i64::abs);
    let hay = format!(
        "{} {} {} {} {} {} {} {} {}",
        r.description,
        r.merchant,
        r.id,
        s.file_name,
        s.account_label,
        s.institution,
        r.kind,
        r.method,
        s.masked_identifier.as_deref().unwrap_or("")
    )
    .to_lowercase();
    Ok(!(q
        .from
        .as_ref()
        .is_some_and(|d| date.as_ref().is_none_or(|v| v < d))
        || q.through
            .as_ref()
            .is_some_and(|d| date.as_ref().is_none_or(|v| v > d))
        || q.account_type
            .as_ref()
            .is_some_and(|v| v != &s.account_type)
        || q.institution.as_ref().is_some_and(|v| v != &s.institution)
        || q.account_id.as_ref().is_some_and(|v| v != &s.account_id)
        || q.source_id.as_ref().is_some_and(|v| v != &s.id)
        || q.kind.as_ref().is_some_and(|v| v != &r.kind)
        || q.method.as_ref().is_some_and(|v| v != &r.method)
        || q.category.as_ref().is_some_and(|v| v != &r.category)
        || q.direction.as_ref().is_some_and(|v| v != &r.direction)
        || q.status.as_ref().is_some_and(|v| v != &r.status)
        || q.reconciliation.as_ref().is_some_and(|v| {
            if v == "confirmedDuplicate" {
                !r.confirmed_duplicate
            } else if v == "suspectedDuplicate" {
                !r.duplicate || r.confirmed_duplicate
            } else {
                v != &s.reconciliation
            }
        })
        || q.minimum
            .as_deref()
            .map(imports::signed_money)
            .transpose()?
            .is_some_and(|v| n.is_none_or(|n| n < v))
        || q.maximum
            .as_deref()
            .map(imports::signed_money)
            .transpose()?
            .is_some_and(|v| n.is_none_or(|n| n > v))
        || !hay.contains(&q.search.to_lowercase())))
}
fn percent(n: i64, d: i64) -> Option<String> {
    (d > 0).then(|| {
        format!(
            "{}.{:02}%",
            (i128::from(n) * 10000 / i128::from(d)) / 100,
            (i128::from(n) * 10000 / i128::from(d)) % 100
        )
    })
}
fn method_totals(rows: &[&Row]) -> Result<(Vec<MethodTotal>, i64, i64), String> {
    let mut amounts = BTreeMap::<&str, (i64, usize)>::new();
    let mut total = 0i64;
    let mut unknown = 0i64;
    for r in rows.iter().filter(|r| r.purchase) {
        let n = imports::signed_money(r.amount.as_deref().ok_or(STORAGE_ERROR)?)?.abs();
        total = total.checked_add(n).ok_or(STORAGE_ERROR)?;
        let m = match r.method.as_str() {
            "creditCard" => "creditCard",
            "debitCard" => "debitCard",
            "unclassified" => "unclassified",
            _ => "otherMethod",
        };
        let entry = amounts.entry(m).or_default();
        entry.0 = entry.0.checked_add(n).ok_or(STORAGE_ERROR)?;
        entry.1 += 1;
        if m == "unclassified" {
            unknown = unknown.checked_add(n).ok_or(STORAGE_ERROR)?;
        }
    }
    let classified = total - unknown;
    let methods = ["creditCard", "debitCard", "unclassified", "otherMethod"]
        .iter()
        .map(|m| {
            let (n, count) = amounts.get(m).copied().unwrap_or_default();
            MethodTotal {
                method: (*m).into(),
                amount: money(n),
                count,
                average: (count > 0).then(|| money((n + count as i64 / 2) / count as i64)),
                share: percent(n, total),
                basis_points: if total == 0 {
                    0
                } else {
                    (i128::from(n) * 10000 / i128::from(total)) as u16
                },
            }
        })
        .collect();
    Ok((methods, classified, total))
}
fn analysis(st: &State, selected: &[&Row], q: &Query) -> Result<Analysis, String> {
    let eligible = |r: &&Row| {
        r.eligible
            && !r.duplicate
            && st
                .sources
                .iter()
                .find(|s| s.id == r.source_id)
                .is_some_and(|s| s.reconciliation == "matched")
    };
    let rows = selected
        .iter()
        .copied()
        .filter(eligible)
        .collect::<Vec<_>>();
    let (methods, classified, total) = method_totals(&rows)?;
    let mut observations=vec!["Purchase shares include only approved, categorized, reconciled, nonduplicate rows in the applied filters. Unknown payment methods remain in the denominator. Credit use alone is not a financial judgment.".into()];
    if total > 0 {
        let card = imports::signed_money(&methods[0].amount)?;
        let debit = imports::signed_money(&methods[1].amount)?;
        if classified == total && card + debit == total {
            let observation = if i128::from(card) * 100 >= i128::from(total) * 60 {
                "Credit cards account for at least 60% of purchase value in these reviewed records."
            } else if i128::from(debit) * 100 >= i128::from(total) * 60 {
                "Checking-linked debit cards account for at least 60% of purchase value in these reviewed records."
            } else {
                "Credit and checking-linked debit purchase values fall within a 40–60% split in these reviewed records."
            };
            observations.push(observation.into());
        } else {
            observations.push("Other or unclassified methods limit the credit-versus-debit comparison; no overall payment preference is inferred.".into());
        }
    }
    let mut categories = BTreeMap::<String, i64>::new();
    let mut frequency = BTreeMap::<&str, usize>::new();
    for r in &rows {
        let observed = if r.kind == "interest" && r.direction == "debit" {
            "interestCharged"
        } else {
            r.method.as_str()
        };
        *frequency.entry(observed).or_default() += 1;
        if r.purchase {
            let n = imports::signed_money(r.amount.as_deref().ok_or(STORAGE_ERROR)?)?.abs();
            let amount = categories.entry(r.category.clone()).or_default();
            *amount = amount.checked_add(n).ok_or(STORAGE_ERROR)?;
        }
    }
    for (key, text, threshold) in [
        ("overdraftFee", "reviewed overdraft fees", 2),
        ("nsfFee", "reviewed insufficient-funds fees", 2),
        ("interestCharged", "reviewed interest charges", 2),
        ("cashWithdrawal", "reviewed cash withdrawals", 1),
    ] {
        let count = frequency.get(key).copied().unwrap_or_default();
        if count >= threshold {
            observations.push(format!("{count} {text} in the selected records."));
        }
    }
    let subscribed = rows
        .iter()
        .filter(|r| {
            st.bank.merchant_groups.iter().any(|g| {
                g.state == "Confirmed"
                    && g.review
                        .as_ref()
                        .is_some_and(|review| review.subscription && !review.invalidated)
                    && g.evidence.iter().any(|e| e.record_id == r.record_id)
            })
        })
        .count();
    if subscribed > 0 {
        observations.push(format!("{subscribed} selected charges have current counselor-confirmed subscription evidence. Missing subscription reviews remain unknown."));
    }
    let small = rows
        .iter()
        .filter(|r| {
            r.purchase
                && r.purpose == "discretionary"
                && r.amount
                    .as_deref()
                    .and_then(|n| imports::signed_money(n).ok())
                    .is_some_and(|n| n.abs() < 1500)
        })
        .count();
    if small >= 5 {
        observations.push(format!("{small} explicitly discretionary purchases under $15; discuss their combined effect with the client."));
    }
    let mut prior_methods = vec![];
    if let Some((from, through)) = q.from.as_deref().zip(q.through.as_deref()) {
        let from = imports::iso_date(from)?;
        let through = imports::iso_date(through)?;
        let span = (through - from).num_days() + 1;
        let prev_start = from
            .checked_sub_signed(chrono::Duration::days(span))
            .ok_or("Date range is too early.")?;
        let prev_end = from.pred_opt().ok_or("Date range is too early.")?;
        let mut prev = q.clone();
        prev.from = Some(prev_start.to_string());
        prev.through = Some(prev_end.to_string());
        let previous = st
            .rows
            .iter()
            .filter(|r| {
                st.sources
                    .iter()
                    .find(|s| s.id == r.source_id)
                    .is_some_and(|s| matches(r, s, &prev).unwrap_or(false))
            })
            .filter(eligible)
            .collect::<Vec<_>>();
        prior_methods = method_totals(&previous)?.0;
        observations.push(format!("Comparison uses the preceding equal-length period {prev_start} through {prev_end}. Missing account coverage can change apparent patterns."));
    }
    let mut categories = categories
        .into_iter()
        .map(|(k, v)| (k, money(v)))
        .collect::<Vec<_>>();
    categories.sort_by(|a, b| {
        imports::signed_money(&b.1)
            .unwrap()
            .cmp(&imports::signed_money(&a.1).unwrap())
            .then_with(|| a.0.cmp(&b.0))
    });
    categories.truncate(5);
    Ok(Analysis {
        methods,
        purchase_total: money(total),
        classified: money(classified),
        unclassified: money(total - classified),
        unclassified_share: percent(total - classified, total),
        excluded: selected.len() - rows.len(),
        observations,
        categories,
        prior_methods,
    })
}
impl Workspace {
    pub fn query_unified_transactions(&mut self, id: &str, mut q: Query) -> Result<Page, String> {
        validate_query(&mut q)?;
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let st = state(&tx, &s.info, id)?;
        if q.context.as_ref().is_some_and(|v| v != &st.token) {
            return Err("Transactions changed. Refresh transactions before paging; no records were changed.".into());
        }
        let mut matched = Vec::new();
        for r in &st.rows {
            let source = st
                .sources
                .iter()
                .find(|s| s.id == r.source_id)
                .ok_or(STORAGE_ERROR)?;
            if matches(r, source, &q)? {
                matched.push(r);
            }
        }
        matched.sort_by(|a, b| {
            let da = if q.posting_date {
                &a.posting_date
            } else {
                &a.date
            };
            let db = if q.posting_date {
                &b.posting_date
            } else {
                &b.date
            };
            match (da, db) {
                (Some(a), Some(b)) => {
                    if q.oldest_first {
                        a.cmp(b)
                    } else {
                        b.cmp(a)
                    }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
            .then_with(|| a.id.cmp(&b.id))
        });
        let total = matched.len();
        if q.page * PAGE > total {
            return Err("This transaction page is unavailable. Refresh transactions.".into());
        }
        let analysis = analysis(&st, &matched, &q)?;
        let rows = matched
            .into_iter()
            .skip(q.page * PAGE)
            .take(PAGE)
            .cloned()
            .collect();
        let categories = st
            .rows
            .iter()
            .map(|r| r.category.clone())
            .filter(|s| !s.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let kinds = st
            .rows
            .iter()
            .map(|r| r.kind.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let warnings=vec!["Supported transaction ledgers: checking, savings (including explicitly identified money-market deposits), and credit cards. Loan/mortgage/credit-line transaction adapters and non-USD currencies are unavailable; no missing principal/interest allocations are inferred.".into(),"Sorting uses the selected recorded date with stable internal-ID ties; no transaction times are invented. Unknown dates sort last and are excluded by date-range filters. Card charges are negative here; payments/refunds are positive liability reductions, not bank deposits. Original signed liability amounts remain in evidence.".into(),"Statement match is an arithmetic check, not proof of complete coverage. Coverage, overlaps and missing statements remain in Completeness & final validation. Suspected duplicate rows stay visible and are excluded from method analysis; rejecting a confirmed duplicate preserves its source and reason.".into()];
        let page = Page {
            client_id: id.into(),
            context: st.token.clone(),
            revision: st.revision,
            query: q,
            total,
            page_size: PAGE,
            sources: st.sources,
            categories,
            kinds,
            rows,
            analysis,
            warnings,
        };
        tx.commit().map_err(storage_error)?;
        Ok(page)
    }
    pub fn get_transaction_inspection(
        &mut self,
        id: &str,
        row_id: &str,
    ) -> Result<Evidence, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let st = state(&tx, &s.info, id)?;
        let row = st
            .rows
            .iter()
            .find(|r| r.id == row_id)
            .ok_or("This transaction is unavailable for the selected client.")?
            .clone();
        let source = st
            .sources
            .iter()
            .find(|s| s.id == row.source_id)
            .ok_or(STORAGE_ERROR)?
            .clone();
        let original = if let Some(n) = row.card_row {
            st.cards
                .statements
                .iter()
                .find(|s| s.source.id == source.id)
                .and_then(|s| s.rows.iter().find(|r| r.row == n))
                .map(|r| r.raw.clone())
        } else {
            st.bank
                .records
                .iter()
                .find(|r| r.id == row.record_id)
                .map(|r| r.raw.clone())
        }
        .ok_or(STORAGE_ERROR)?;
        let metadata_history = st
            .history
            .into_iter()
            .filter(|h| {
                h.data.key() == row.id
                    || h.data.key() == format!("account:{}", source.account_id)
                    || source.kind == "card"
                        && h.data.key() == format!("account:card-source:{}", source.id)
            })
            .collect();
        let evidence = Evidence {
            client_id: id.into(),
            context: st.token,
            revision: st.revision,
            row,
            source,
            original,
            metadata_history,
        };
        tx.commit().map_err(storage_error)?;
        Ok(evidence)
    }
    pub fn save_transaction_inspection(&mut self, id: &str, input: Save) -> Result<(), String> {
        let data = canonical(input.data)?;
        let reason = cards::text(&input.reason, 500, true)?;
        if !input.acknowledge {
            return Err("Confirm the details against the original source before saving.".into());
        }
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let st = state(&tx, &s.info, id)?;
        if input.context != st.token || input.expected_revision != st.revision {
            return Err("Transactions changed. Refresh before saving inspection details; no changes were applied.".into());
        }
        match &data {
            Data::Account {
                account_id,
                subtype,
                ..
            } => {
                let sources = st
                    .sources
                    .iter()
                    .filter(|s| &s.account_id == account_id)
                    .collect::<Vec<_>>();
                if sources.is_empty()
                    || sources.iter().any(|s| {
                        if s.kind == "card" {
                            subtype != "creditCard"
                        } else {
                            st.bank
                                .accounts
                                .iter()
                                .find(|a| format!("bank:{}", a.id) == s.account_id)
                                .is_none_or(|a| {
                                    if a.kind == "checking" {
                                        subtype != "checking"
                                    } else {
                                        !matches!(
                                            subtype.as_str(),
                                            "savings" | "moneyMarket" | "otherDeposit"
                                        )
                                    }
                                })
                        }
                    })
                {
                    return Err("Choose a compatible reviewed deposit/card subtype; original account classification stays unchanged.".into());
                }
            }
            Data::Transaction {
                row_id,
                binding,
                method,
                confirmed_duplicate,
                ..
            } => {
                let r = st
                    .rows
                    .iter()
                    .find(|r| &r.id == row_id && &r.binding == binding)
                    .ok_or("The source decision changed; refresh and verify the details again.")?;
                if *confirmed_duplicate && r.status != Status::Rejected {
                    return Err("Reject a verified duplicate through the existing review workflow before confirming its exclusion.".into());
                }
                if !compatible(r, method)
                    || method == "debitCard"
                        && !st
                            .sources
                            .iter()
                            .any(|s| s.id == r.source_id && s.account_type == "checking")
                {
                    return Err("This method conflicts with the reviewed kind, direction or account. Correct the financial transaction first, or choose unclassified.".into());
                }
            }
        }
        if st.history.len() >= MAX_DETAILS {
            return Err("The inspection detail history reached its 10,000-entry limit.".into());
        }
        let event = DetailEvent {
            revision: st.history.len() + 1,
            data,
            reason,
            at: Utc::now().to_rfc3339(),
            counselor: s.info.counselor_name.clone(),
        };
        let body = json(&event)?;
        let previous=tx.query_row("SELECT digest FROM transaction_inspection_details WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision DESC LIMIT 1",params![id,s.info.organization_id,s.info.counselor_id],|r|r.get::<_,String>(0)).optional().map_err(storage_error)?.unwrap_or_default();
        imports_store::bump(&tx, &s.info, id, input.expected_revision)?;
        tx.execute("INSERT INTO transaction_inspection_details(client_id,organization_id,counselor_id,revision,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6)",params![id,s.info.organization_id,s.info.counselor_id,event.revision as i64,body,hash(format!("{previous}\n{body}").as_bytes())]).map_err(storage_error)?;
        audit(&tx, &s.info, "transaction_reviewed", Some(id))?;
        tx.commit().map_err(storage_error)?;
        Ok(())
    }
    pub fn get_original_statement(
        &mut self,
        id: &str,
        source_id: &str,
    ) -> Result<Original, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let st = state(&tx, &s.info, id)?;
        let source = st
            .sources
            .iter()
            .find(|s| s.id == source_id)
            .ok_or("Source unavailable for this client.")?;
        if !source.pdf || !source.retained {
            return Err("Original PDF bytes were not retained. Captured transaction evidence is available; no original page can be displayed.".into());
        }
        let sql = if source.kind == "bank" {
            "SELECT source_blob FROM import_documents WHERE id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4"
        } else {
            "SELECT source_blob FROM card_statements WHERE id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4"
        };
        let bytes: Vec<u8> = tx
            .query_row(
                sql,
                params![source_id, id, s.info.organization_id, s.info.counselor_id],
                |r| r.get(0),
            )
            .map_err(storage_error)?;
        if bytes.len() > imports::MAX_BYTES
            || !bytes.starts_with(b"%PDF-")
            || hash(&bytes) != source.sha256
        {
            return Err("The retained original failed integrity checks.".into());
        }
        let out = Original {
            client_id: id.into(),
            source_id: source_id.into(),
            sha256: source.sha256.clone(),
            bytes,
        };
        tx.commit().map_err(storage_error)?;
        Ok(out)
    }
}
fn compatible(r: &Row, m: &str) -> bool {
    if m == "unclassified" {
        return true;
    }
    if r.card_row.is_some() {
        return matches!(
            (r.kind.as_str(), m),
            ("purchase", "creditCard")
                | ("payment", "cardPayment")
                | ("refund", "refund" | "reversal")
                | ("adjustment", "reversal")
                | ("interest", "interestCharged")
                | ("fee", "fee" | "overdraftFee" | "nsfFee")
        );
    }
    match m {
        "debitCard" => r.direction == "debit" && r.kind == "expense",
        "check" | "achDebit" => {
            r.direction == "debit"
                && matches!(
                    r.kind.as_str(),
                    "expense" | "fee" | "interest" | "balanceMovement" | "transfer"
                )
        }
        "achCredit" | "directDeposit" | "cashDeposit" | "otherDeposit" => {
            r.direction == "credit"
                && matches!(
                    r.kind.as_str(),
                    "income" | "refund" | "interest" | "balanceMovement" | "transfer"
                )
        }
        "cashWithdrawal" | "otherWithdrawal" => {
            r.direction == "debit" && r.kind == "balanceMovement"
        }
        "internalTransfer" => r.kind == "transfer",
        "externalTransfer" => r.kind == "balanceMovement",
        "cardPayment" | "loanPayment" => r.direction == "debit" && r.kind == "balanceMovement",
        "interestEarned" => r.direction == "credit" && r.kind == "interest",
        "interestCharged" => r.direction == "debit" && r.kind == "interest",
        "fee" | "overdraftFee" | "nsfFee" => r.direction == "debit" && r.kind == "fee",
        "refund" => r.direction == "credit" && r.kind == "refund",
        "reversal" => matches!(r.kind.as_str(), "balanceMovement" | "refund"),
        _ => false,
    }
}
pub(super) fn validate_all(c: &Connection, w: &WorkspaceInfo) -> Result<(), String> {
    let mut q=c.prepare("SELECT DISTINCT client_id FROM transaction_inspection_details WHERE organization_id=?1 AND counselor_id=?2").map_err(storage_error)?;
    let ids = q
        .query_map(params![w.organization_id, w.counselor_id], |r| {
            r.get::<_, String>(0)
        })
        .map_err(storage_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(storage_error)?;
    for id in ids {
        let st = state(c, w, &id)?;
        for event in &st.history {
            match &event.data {
                Data::Account { account_id, .. } => {
                    if !st.sources.iter().any(|s| &s.account_id == account_id)
                        && !st
                            .cards
                            .statements
                            .iter()
                            .any(|s| account_id == &format!("card-source:{}", s.source.id))
                        && !card_review_store::history(c, w, &id)?.iter().any(|h| {
                            h.accounts
                                .iter()
                                .any(|a| account_id == &format!("card:{}", a.id))
                        })
                    {
                        return Err(STORAGE_ERROR.into());
                    }
                }
                Data::Transaction {
                    row_id, binding, ..
                } => {
                    let r = st
                        .rows
                        .iter()
                        .find(|r| &r.id == row_id)
                        .ok_or(STORAGE_ERROR)?;
                    let valid = r.binding == *binding
                        || binding == &format!("import:{}", r.record_id)
                        || st
                            .bank
                            .history
                            .iter()
                            .any(|h| h.record_id == r.record_id && &h.id == binding)
                        || st
                            .cards
                            .statements
                            .iter()
                            .filter(|s| s.source.id == r.source_id)
                            .any(|s| {
                                s.history
                                    .iter()
                                    .any(|h| Some(h.row) == r.card_row && &h.id == binding)
                            });
                    if !valid {
                        return Err(STORAGE_ERROR.into());
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        cards_store::tests as card,
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    fn approve(w: &mut Workspace, id: &str, record: &str, status: Status) {
        let s = w.get_client_imports(id).unwrap();
        let r = s.records.iter().find(|r| r.id == record).unwrap();
        w.review_transaction(
            id,
            imports::ReviewInput {
                expected_revision: s.revision,
                record_id: record.into(),
                fields: r.fields.clone(),
                status,
                reason: "Verified original synthetic transaction and duplicate context".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
    }
    fn fixture(w: &mut Workspace, id: &str, dir: &Path) {
        let mut d = bank::doc("Synthetic checking");
        d.closing_balance = Some("819.99".into());
        let s=bank::stage(w,id,dir,"inspection-checking.csv","Date,Description,Amount,Category,Type\n2026-09-01,Debit market,-100.00,Food,expense\n2026-09-01,Unknown method,-50.00,Food,expense\n2026-09-03,Card repayment,-50.03,Debt,balanceMovement\n2026-09-02,Purchase refund,20.02,Food,refund\n",d);
        for r in &s.records {
            approve(w, id, &r.id, Status::Approved);
        }
        card::stage(w, id, dir);
        for (i, k) in [
            cards::Kind::Purchase,
            cards::Kind::Refund,
            cards::Kind::Payment,
            cards::Kind::Fee,
            cards::Kind::Interest,
        ]
        .into_iter()
        .enumerate()
        {
            let s = w.get_card_statements(id).unwrap();
            w.review_card_transaction(id, card::decision(&s, i, k, Status::Approved))
                .unwrap();
        }
    }
    fn save(w: &mut Workspace, id: &str, data: Data) -> Result<(), String> {
        let p = w.query_unified_transactions(id, Query::default())?;
        w.save_transaction_inspection(
            id,
            Save {
                context: p.context,
                expected_revision: p.revision,
                data,
                reason: "Counselor inspected the original fictional statement".into(),
                acknowledge: true,
            },
        )
    }
    #[test]
    fn chronological_current_rows_unify_accounts_and_preserve_original_signs() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        fixture(&mut w, &c.id, dir.path());
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        assert_eq!(p.total, 9);
        assert_eq!(p.rows[0].date.as_deref(), Some("2026-09-05"));
        let same = p
            .rows
            .iter()
            .filter(|r| r.date.as_deref() == Some("2026-09-01"))
            .collect::<Vec<_>>();
        assert_eq!(same.len(), 3);
        assert!(same.windows(2).all(|r| r[0].id < r[1].id));
        let card = same.iter().find(|r| r.card_row == Some(0)).unwrap();
        assert_eq!(card.amount.as_deref(), Some("-100.01"));
        assert_eq!(card.original_amount.as_deref(), Some("100.01"));
        assert_eq!(card.method, "creditCard");
        let old = w
            .query_unified_transactions(
                &c.id,
                Query {
                    oldest_first: true,
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(old.rows[0].date.as_deref(), Some("2026-09-01"));
        assert!(p.sources.iter().all(|s| s.reconciliation == "matched"));
        let repayment = p.rows.iter().find(|r| r.kind == "payment").unwrap();
        assert_eq!(repayment.amount.as_deref(), Some("50.03"));
        assert!(!repayment.purchase);
    }
    #[test]
    fn exact_purchase_methods_exclude_repayments_refunds_fees_and_show_unknown_coverage() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        fixture(&mut w, &c.id, dir.path());
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        let r = p
            .rows
            .iter()
            .find(|r| r.description == "Debit market")
            .unwrap();
        save(
            &mut w,
            &c.id,
            Data::Transaction {
                row_id: r.id.clone(),
                binding: r.binding.clone(),
                posting_date: Some("2026-09-02".into()),
                method: "debitCard".into(),
                confirmed_duplicate: false,
            },
        )
        .unwrap();
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        assert_eq!(p.analysis.purchase_total, "250.01");
        assert_eq!(p.analysis.classified, "200.01");
        assert_eq!(p.analysis.unclassified, "50.00");
        assert_eq!(p.analysis.unclassified_share.as_deref(), Some("19.99%"));
        let totals = &p.analysis.methods;
        assert_eq!(totals[0].amount, "100.01");
        assert_eq!(totals[0].count, 1);
        assert_eq!(totals[0].average.as_deref(), Some("100.01"));
        assert_eq!(totals[0].share.as_deref(), Some("40.00%"));
        assert_eq!(totals[1].amount, "100.00");
        assert_eq!(totals[1].share.as_deref(), Some("39.99%"));
        let posted = w
            .query_unified_transactions(
                &c.id,
                Query {
                    posting_date: true,
                    from: Some("2026-09-02".into()),
                    through: Some("2026-09-02".into()),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(posted.total, 1);
        assert_eq!(posted.rows[0].date.as_deref(), Some("2026-09-01"));
        assert_eq!(posted.rows[0].posting_date.as_deref(), Some("2026-09-02"));
    }
    #[test]
    fn filters_compose_literal_search_amount_direction_institution_and_review_state() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        fixture(&mut w, &c.id, dir.path());
        let other = w.save_client(client_input()).unwrap();
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        let a = p
            .sources
            .iter()
            .find(|s| s.kind == "bank")
            .unwrap()
            .account_id
            .clone();
        save(
            &mut w,
            &c.id,
            Data::Account {
                account_id: a.clone(),
                institution: "Synthetic % _ Bank".into(),
                last_four: "1234".into(),
                subtype: "checking".into(),
            },
        )
        .unwrap();
        let q = Query {
            search: "% _".into(),
            institution: Some("Synthetic % _ Bank".into()),
            account_type: Some("checking".into()),
            account_id: Some(a),
            direction: Some("debit".into()),
            minimum: Some("70".into()),
            maximum: Some("100.00".into()),
            category: Some("Food".into()),
            status: Some(Status::Approved),
            reconciliation: Some("matched".into()),
            ..Query::default()
        };
        let p = w.query_unified_transactions(&c.id, q).unwrap();
        assert_eq!(p.total, 1);
        assert_eq!(p.rows[0].description, "Debit market");
        assert_eq!(p.sources[0].masked_identifier.as_deref(), Some("•••• 1234"));
        assert_eq!(
            w.query_unified_transactions(&other.id, Query::default())
                .unwrap()
                .total,
            0
        );
        assert!(w
            .get_transaction_inspection(&other.id, &p.rows[0].id)
            .is_err());
        assert!(w
            .query_unified_transactions(
                &c.id,
                Query {
                    minimum: Some("200".into()),
                    maximum: Some("1".into()),
                    ..Query::default()
                }
            )
            .is_err());
        assert_eq!(
            w.query_unified_transactions(
                &c.id,
                Query {
                    account_type: Some("loanCreditLine".into()),
                    ..Query::default()
                }
            )
            .unwrap()
            .total,
            0
        );
    }
    #[test]
    fn pagination_tokens_and_posting_dates_are_explicit_unknowns() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let mut csv = "Date,Description,Amount,Category,Type\n".to_owned();
        for i in 0..28 {
            csv += &format!("2026-09-01,Synthetic row {i},-1.00,Food,expense\n");
        }
        csv += "unknown,Undated,-1.00,Food,expense\n";
        bank::stage(
            &mut w,
            &c.id,
            dir.path(),
            "pages.csv",
            &csv,
            bank::doc("Paged checking"),
        );
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        assert_eq!(p.total, 29);
        assert_eq!(p.rows.len(), 25);
        let page = w
            .query_unified_transactions(
                &c.id,
                Query {
                    page: 1,
                    context: Some(p.context.clone()),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(page.rows.len(), 4);
        assert!(page.rows.last().unwrap().date.is_none());
        assert!(!page
            .rows
            .iter()
            .any(|r| p.rows.iter().any(|a| a.id == r.id)));
        assert_eq!(
            w.query_unified_transactions(
                &c.id,
                Query {
                    posting_date: true,
                    from: Some("2026-09-01".into()),
                    ..Query::default()
                }
            )
            .unwrap()
            .total,
            0
        );
        save(
            &mut w,
            &c.id,
            Data::Account {
                account_id: p.sources[0].account_id.clone(),
                institution: "Updated bank".into(),
                last_four: String::new(),
                subtype: "checking".into(),
            },
        )
        .unwrap();
        assert!(w
            .query_unified_transactions(
                &c.id,
                Query {
                    page: 1,
                    context: Some(p.context),
                    ..Query::default()
                }
            )
            .is_err());
    }
    #[test]
    fn metadata_is_atomic_source_bound_scoped_and_invalidated_by_financial_corrections() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        fixture(&mut w, &c.id, dir.path());
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        let r = p
            .rows
            .iter()
            .find(|r| r.description == "Debit market")
            .unwrap();
        assert!(!compatible(r, "cashWithdrawal"));
        assert!(!compatible(r, "otherWithdrawal"));
        let id = r.id.clone();
        let record = r.record_id.clone();
        let data = Data::Transaction {
            row_id: id.clone(),
            binding: r.binding.clone(),
            posting_date: None,
            method: "debitCard".into(),
            confirmed_duplicate: false,
        };
        let original = json(&w.get_transaction_inspection(&c.id, &id).unwrap().original).unwrap();
        w.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER fail_inspection_audit BEFORE INSERT ON audit_events WHEN NEW.event='transaction_reviewed' BEGIN SELECT RAISE(ABORT,'Synthetic rollback'); END;").unwrap();
        assert!(save(&mut w, &c.id, data.clone()).is_err());
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER fail_inspection_audit;")
            .unwrap();
        assert_eq!(
            w.query_unified_transactions(&c.id, Query::default())
                .unwrap()
                .revision,
            p.revision
        );
        save(&mut w, &c.id, data).unwrap();
        assert_eq!(
            w.get_transaction_inspection(&c.id, &id).unwrap().row.method,
            "debitCard"
        );
        approve(&mut w, &c.id, &record, Status::NeedsClarification);
        let evidence = w.get_transaction_inspection(&c.id, &id).unwrap();
        assert_eq!(evidence.row.method, "unclassified");
        assert_eq!(evidence.metadata_history.len(), 1);
        assert_eq!(json(&evidence.original).unwrap(), original);
        assert!(evidence
            .row
            .warnings
            .iter()
            .any(|s| s.contains("older decision")));
        let card = p.rows.iter().find(|r| r.card_row == Some(0)).unwrap();
        assert!(save(
            &mut w,
            &c.id,
            Data::Transaction {
                row_id: card.id.clone(),
                binding: card.binding.clone(),
                posting_date: None,
                method: "debitCard".into(),
                confirmed_duplicate: false,
            }
        )
        .is_err());
    }
    #[test]
    fn duplicate_statement_purchase_evidence_is_visible_and_excluded() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        fixture(&mut w, &c.id, dir.path());
        let s = w.get_card_statements(&c.id).unwrap();
        let source = &s.statements[0].source;
        let path = dir.path().join("card-overlap.csv");
        fs::write(
            &path,
            "Date,Description,Amount,Category\n2026-09-01,Fictional groceries,100.01,Food\n",
        )
        .unwrap();
        let p = w
            .inspect_client_csv(c.id.clone(), path.to_str().unwrap().into())
            .unwrap();
        let mut input = source.input.clone();
        input.closing_owed = "1100.01".into();
        let staged = w
            .stage_card_statement(
                &c.id,
                cards::Stage {
                    expected_revision: s.revision,
                    token: p.token,
                    input,
                },
            )
            .unwrap();
        let statement = staged.statements.last().unwrap();
        w.review_card_transaction(
            &c.id,
            cards::ReviewInput {
                expected_revision: staged.revision,
                statement_id: statement.source.id.clone(),
                row: 0,
                fields: cards::Fields {
                    kind: cards::Kind::Purchase,
                    ..statement.rows[0].fields.clone()
                },
                status: Status::Approved,
                reason: "Inspected overlapping synthetic evidence".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        let p = w
            .query_unified_transactions(
                &c.id,
                Query {
                    reconciliation: Some("suspectedDuplicate".into()),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(p.total, 2);
        assert!(p.rows.iter().all(|r| r.duplicate));
        assert_eq!(p.analysis.purchase_total, "0.00");
    }
    #[test]
    fn supplemental_history_survives_restart_and_encrypted_recovery() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        fixture(&mut w, &c.id, dir.path());
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        let r = &p.rows[0];
        let row = r.id.clone();
        save(
            &mut w,
            &c.id,
            Data::Transaction {
                row_id: row.clone(),
                binding: r.binding.clone(),
                posting_date: Some("2026-09-06".into()),
                method: "interestCharged".into(),
                confirmed_duplicate: false,
            },
        )
        .unwrap();
        let destination = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(destination.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.get_transaction_inspection(&c.id, &row).is_err());
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let evidence = w.get_transaction_inspection(&c.id, &row).unwrap();
        assert_eq!(evidence.row.posting_date.as_deref(), Some("2026-09-06"));
        assert_eq!(evidence.metadata_history.len(), 1);
        let s = w.session.as_ref().unwrap();
        validate_all(&s.connection, &s.info).unwrap();
    }
    #[test]
    fn retained_original_pdf_is_scoped_and_no_retention_does_not_fabricate_a_document() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let bytes = crate::pdf_fixture::statement();
        let path = dir.path().join("original.pdf");
        fs::write(&path, &bytes).unwrap();
        // Exercise retained-byte publication, not the separately supervised executable worker.
        let p = bank::captured_test_pdf(&mut w, &c.id, &path, bytes.clone());
        let mut d = bank::doc("Original PDF checking");
        d.retain_source = true;
        let source_line = p.pdf_pages.as_ref().unwrap()[0]
            .text
            .lines()
            .position(|line| line.contains("Fictional payroll"))
            .unwrap() as u32
            + 1;
        d.pdf_selection = Some(vec![crate::pdf::Locator {
            page: 1,
            line: source_line,
        }]);
        let imported = w
            .stage_client_csv(
                &c.id,
                imports::StageInput {
                    expected_revision: 0,
                    token: p.token,
                    document: d,
                },
            )
            .unwrap();
        let source = &imported.documents[0].id;
        let original = w.get_original_statement(&c.id, source).unwrap();
        assert_eq!(original.bytes, bytes);
        assert_eq!(original.sha256, hash(&bytes));
        assert!(w.get_original_statement(&other.id, source).is_err());
        let row = w
            .get_transaction_inspection(&c.id, &format!("bank:{}", imported.records[0].id))
            .unwrap();
        assert_eq!(row.row.page, Some(1));
        assert_eq!(row.original.line, u64::from(source_line));
        let csv = bank::stage(
            &mut w,
            &c.id,
            dir.path(),
            "not-retained.csv",
            "Date,Description,Amount,Category,Type\n2026-09-01,Unretained source,-1,Food,expense\n",
            bank::doc("Other source"),
        );
        assert!(w
            .get_original_statement(&c.id, &csv.documents.last().unwrap().id)
            .is_err());
        w.lock_workspace();
        assert!(w.get_original_statement(&c.id, source).is_err());
    }
    #[test]
    fn savings_money_market_subtype_does_not_invent_debit_card_purchases() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let mut d = bank::doc("Money market savings");
        d.account_kind = "savings".into();
        d.closing_balance = Some("999.00".into());
        let staged = bank::stage(
            &mut w,
            &c.id,
            dir.path(),
            "market.csv",
            "Date,Description,Amount,Category,Type\n2026-09-01,Actual withdrawal,-1,Food,expense\n",
            d,
        );
        approve(&mut w, &c.id, &staged.records[0].id, Status::Approved);
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        save(
            &mut w,
            &c.id,
            Data::Account {
                account_id: p.sources[0].account_id.clone(),
                institution: "Synthetic institution".into(),
                last_four: "5678".into(),
                subtype: "moneyMarket".into(),
            },
        )
        .unwrap();
        let p = w
            .query_unified_transactions(
                &c.id,
                Query {
                    account_type: Some("moneyMarket".into()),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(p.total, 1);
        let r = &p.rows[0];
        assert!(save(
            &mut w,
            &c.id,
            Data::Transaction {
                row_id: r.id.clone(),
                binding: r.binding.clone(),
                posting_date: None,
                method: "debitCard".into(),
                confirmed_duplicate: false,
            }
        )
        .is_err());
        assert_eq!(p.analysis.unclassified, "1.00");
    }
    #[test]
    fn discrepancies_are_visible_but_excluded_from_purchase_metrics() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let s=bank::stage(&mut w,&c.id,dir.path(),"mismatch.csv","Date,Description,Amount,Category,Type\n2026-09-01,Unbalanced purchase,-10,Food,expense\n",bank::doc("Discrepant source"));
        approve(&mut w, &c.id, &s.records[0].id, Status::Approved);
        let p = w
            .query_unified_transactions(
                &c.id,
                Query {
                    reconciliation: Some("difference".into()),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(p.total, 1);
        assert_eq!(p.analysis.purchase_total, "0.00");
        assert_eq!(p.analysis.excluded, 1);
        assert!(p.analysis.methods.iter().all(|m| m.share.is_none()));
    }
    #[test]
    fn confirmed_duplicate_requires_an_existing_rejection_and_preserves_financial_history() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        fixture(&mut w, &c.id, dir.path());
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        let r = p
            .rows
            .iter()
            .find(|r| r.description == "Debit market")
            .unwrap();
        let row = r.id.clone();
        let record = r.record_id.clone();
        assert!(save(
            &mut w,
            &c.id,
            Data::Transaction {
                row_id: row.clone(),
                binding: r.binding.clone(),
                posting_date: None,
                method: "unclassified".into(),
                confirmed_duplicate: true
            }
        )
        .is_err());
        approve(&mut w, &c.id, &record, Status::Rejected);
        let e = w.get_transaction_inspection(&c.id, &row).unwrap();
        save(
            &mut w,
            &c.id,
            Data::Transaction {
                row_id: row.clone(),
                binding: e.row.binding,
                posting_date: None,
                method: "unclassified".into(),
                confirmed_duplicate: true,
            },
        )
        .unwrap();
        let p = w
            .query_unified_transactions(
                &c.id,
                Query {
                    reconciliation: Some("confirmedDuplicate".into()),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(p.total, 1);
        assert_eq!(p.rows[0].status, Status::Rejected);
        assert!(p.rows[0].confirmed_duplicate);
        assert_eq!(p.analysis.purchase_total, "0.00");
        approve(&mut w, &c.id, &record, Status::Approved);
        let e = w.get_transaction_inspection(&c.id, &row).unwrap();
        assert!(!e.row.confirmed_duplicate);
        assert_eq!(e.metadata_history.len(), 1);
        assert_eq!(e.original.reported_amount.as_deref(), Some("-100.00"));
    }
    #[test]
    fn payment_observations_use_exact_values_instead_of_rounded_percent_thresholds() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        fixture(&mut w, &c.id, dir.path());
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        for r in p
            .rows
            .iter()
            .filter(|r| r.source_id == p.sources[0].id && r.kind == "expense")
        {
            save(
                &mut w,
                &c.id,
                Data::Transaction {
                    row_id: r.id.clone(),
                    binding: r.binding.clone(),
                    posting_date: None,
                    method: "debitCard".into(),
                    confirmed_duplicate: false,
                },
            )
            .unwrap();
        }
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        assert_eq!(p.analysis.purchase_total, "250.01");
        assert_eq!(p.analysis.methods[1].amount, "150.00");
        assert!(p
            .analysis
            .observations
            .iter()
            .any(|s| s.contains("40–60% split")));
        let card = w
            .query_unified_transactions(
                &c.id,
                Query {
                    account_type: Some("creditCard".into()),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(card.analysis.purchase_total, "100.01");
        assert!(card
            .analysis
            .observations
            .iter()
            .any(|s| s.contains("Credit cards account for at least 60%")));
    }
    #[test]
    fn reviewed_channels_preserve_movements_refunds_and_interest_instead_of_forcing_income_or_purchases(
    ) {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let mut d = bank::doc("Channel checking");
        d.opening_balance = Some("0".into());
        d.closing_balance = Some("2370".into());
        let staged=bank::stage(&mut w,&c.id,dir.path(),"channels.csv","Date,Description,Amount,Category,Type\n2026-09-01,Loan proceeds,500,Debt,balanceMovement\n2026-09-02,Loan repayment,-100,Debt,balanceMovement\n2026-09-03,Actual wages,2000,Wages,income\n2026-09-04,Reversed purchase,25,Food,refund\n2026-09-05,External movement,-50,Transfers,balanceMovement\n2026-09-06,Interest one,-2,Interest,interest\n2026-09-07,Interest two,-3,Interest,interest\n",d);
        for r in &staged.records {
            approve(&mut w, &c.id, &r.id, Status::Approved);
        }
        let before = w.get_client_imports(&c.id).unwrap();
        let originals = json(
            &before
                .records
                .iter()
                .map(|r| (&r.raw, &r.fields))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        for (name, method) in [
            ("Loan proceeds", "achCredit"),
            ("Loan proceeds", "directDeposit"),
            ("Loan proceeds", "externalTransfer"),
            ("Loan repayment", "achDebit"),
            ("Loan repayment", "check"),
            ("Actual wages", "directDeposit"),
            ("Reversed purchase", "achCredit"),
            ("Reversed purchase", "reversal"),
            ("External movement", "externalTransfer"),
            ("Interest one", "achDebit"),
        ] {
            let p = w
                .query_unified_transactions(&c.id, Query::default())
                .unwrap();
            let r = p.rows.iter().find(|r| r.description == name).unwrap();
            save(
                &mut w,
                &c.id,
                Data::Transaction {
                    row_id: r.id.clone(),
                    binding: r.binding.clone(),
                    posting_date: None,
                    method: method.into(),
                    confirmed_duplicate: false,
                },
            )
            .unwrap();
        }
        let after = w.get_client_imports(&c.id).unwrap();
        assert_eq!(after.summary.income, "2000.00");
        assert_eq!(after.summary.spending, "5.00");
        assert_eq!(after.summary.refunds, "25.00");
        assert_eq!(after.summary.net_cash, "2370.00");
        assert_eq!(after.history.len(), before.history.len());
        assert_eq!(
            json(
                &after
                    .records
                    .iter()
                    .map(|r| (&r.raw, &r.fields))
                    .collect::<Vec<_>>()
            )
            .unwrap(),
            originals
        );
        let p = w
            .query_unified_transactions(&c.id, Query::default())
            .unwrap();
        assert_eq!(p.total, 7);
        assert_eq!(p.analysis.purchase_total, "0.00");
        assert!(p
            .analysis
            .observations
            .iter()
            .any(|s| s.contains("2 reviewed interest charges")));
        assert_eq!(
            p.rows
                .iter()
                .find(|r| r.description == "Loan proceeds")
                .unwrap()
                .kind,
            "balanceMovement"
        );
        assert_eq!(
            p.rows
                .iter()
                .find(|r| r.description == "Reversed purchase")
                .unwrap()
                .kind,
            "refund"
        );
        let session = w.session.as_ref().unwrap();
        validate_all(&session.connection, &session.info).unwrap();
    }
}
