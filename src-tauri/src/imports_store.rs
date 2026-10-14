//! Authenticated client CSV sources, immutable provenance, and explicit decisions.
use super::*;
use crate::budget::money;
use crate::imports::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
const MAX_RECORDS: usize = 5000;
const MAX_HISTORY: usize = 10000;
pub(super) struct PendingCsv {
    client_id: String,
    token: String,
    bytes: Zeroizing<Vec<u8>>,
    file_name: String,
    sha256: String,
    created: Instant,
    pdf_pages: Option<Vec<crate::pdf::Page>>,
}
impl PendingCsv {
    pub(super) fn expired(&self) -> bool {
        self.created.elapsed() >= AUTO_LOCK_AFTER
    }
}
fn status(value: &Status) -> &'static str {
    match value {
        Status::Pending => "Pending",
        Status::NeedsClarification => "Needs Clarification",
        Status::Approved => "Approved",
        Status::Rejected => "Rejected",
    }
}
fn decode_status(value: &str) -> Result<Status, String> {
    match value {
        "Pending" => Ok(Status::Pending),
        "Needs Clarification" => Ok(Status::NeedsClarification),
        "Approved" => Ok(Status::Approved),
        "Rejected" => Ok(Status::Rejected),
        _ => Err(STORAGE_ERROR.into()),
    }
}
fn json<T: Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|_| STORAGE_ERROR.to_owned())
}
fn decode<T: serde::de::DeserializeOwned>(v: &str) -> Result<T, String> {
    serde_json::from_str(v).map_err(|_| STORAGE_ERROR.to_owned())
}
fn revision(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<i64, String> {
    c.query_row("SELECT revision FROM client_import_state WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).optional().map_err(storage_error).map(|v|v.unwrap_or(0))
}
fn bump(c: &Transaction, w: &WorkspaceInfo, client: &str, expected: i64) -> Result<(), String> {
    if !(0..i64::from(i32::MAX)).contains(&expected) || revision(c, w, client)? != expected {
        return Err("This client's import/review data changed elsewhere. Refresh before trying again; unsaved edits have not been applied.".into());
    }
    c.execute("INSERT INTO client_import_state(client_id,organization_id,counselor_id,revision) VALUES(?1,?2,?3,?4) ON CONFLICT(client_id) DO UPDATE SET revision=excluded.revision WHERE client_import_state.organization_id=excluded.organization_id AND client_import_state.counselor_id=excluded.counselor_id",params![client,w.organization_id,w.counselor_id,expected+1]).map_err(storage_error)?;
    Ok(())
}
fn reason(value: &str) -> Result<String, String> {
    optional_text(value, "Review reason", 500, false)
}
fn period_warning(d: &Document, f: &Fields) -> Option<String> {
    f.date
        .as_ref()
        .filter(|date| {
            d.input
                .period_start
                .as_ref()
                .is_some_and(|start| *date < start)
                || d.input.period_end.as_ref().is_some_and(|end| *date > end)
        })
        .map(|_| {
            "Transaction date lies outside the entered statement period; verify it individually."
                .into()
        })
}
fn overlaps(snapshot: &Snapshot, record: &Record, fields: &Fields) -> bool {
    let keys = [fingerprint(&record.raw.extracted), fingerprint(fields)]
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();
    snapshot
        .records
        .iter()
        .filter(|r| {
            r.id != record.id && r.account_id == record.account_id && r.status != Status::Rejected
        })
        .any(|r| {
            [fingerprint(&r.raw.extracted), fingerprint(&r.fields)]
                .into_iter()
                .flatten()
                .any(|k| keys.contains(&k))
                || record
                    .raw
                    .reference
                    .as_ref()
                    .is_some_and(|reference| r.raw.reference.as_ref() == Some(reference))
        })
}
fn normalize_document(mut d: DocumentInput) -> Result<DocumentInput, String> {
    if d.balance_order.is_some() && (d.pdf_selection.is_some() || d.mapping.balance.is_none()) {
        return Err("Running-balance order requires a mapped CSV balance column; text-PDF balance columns are not supported yet.".into());
    }
    if !matches!(d.account_kind.as_str(), "checking" | "savings") {
        return Err("Choose a checking or savings account for this CSV stage.".into());
    }
    if d.account_id.is_none() {
        d.account_label = required_text(&d.account_label, "Account label", 120)?;
    }
    for date in [&d.period_start, &d.period_end].into_iter().flatten() {
        iso_date(date)?;
    }
    if d.period_start
        .as_ref()
        .zip(d.period_end.as_ref())
        .is_some_and(|(a, b)| a > b)
    {
        return Err("The statement period ends before it starts.".into());
    }
    for v in [&mut d.opening_balance, &mut d.closing_balance]
        .into_iter()
        .flatten()
    {
        *v = money(signed_money(v)?);
    }
    Ok(d)
}
fn read_financial_facts(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    records: &[Record],
    history: &[History],
    documents: &[Document],
) -> Result<crate::financial_facts::Facts, String> {
    use crate::financial_facts::{self, Data, Facts, Review};
    let size:i64=c.query_row("SELECT COALESCE(SUM(length(CAST(data_json AS BLOB))),0) FROM financial_fact_reviews WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?;
    if size > 8 * 1024 * 1024 {
        return Err(STORAGE_ERROR.into());
    }
    let rows=c.prepare("SELECT id,data_json,reason,reviewed_at,counselor_name FROM financial_fact_reviews WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid LIMIT 101").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    if rows.len() > 100 {
        return Err(STORAGE_ERROR.into());
    }
    let ids = documents.iter().map(|d| d.id.as_str()).collect();
    let mut reviews = Vec::new();
    for (id, json, reason, reviewed_at, counselor_name) in rows {
        if json.len() > financial_facts::MAX_JSON {
            return Err(STORAGE_ERROR.into());
        }
        let data: Data = decode(&json)?;
        if financial_facts::normalize(data.clone())? != data {
            return Err(STORAGE_ERROR.into());
        }
        financial_facts::validate_bindings(&data, records, history, &ids)
            .map_err(|_| STORAGE_ERROR.to_owned())?;
        required_text(&reason, "Facts review reason", 500)?;
        required_text(&counselor_name, "Counselor name", 120)?;
        chrono::DateTime::parse_from_rfc3339(&reviewed_at).map_err(|_| STORAGE_ERROR.to_owned())?;
        reviews.push(Review {
            id,
            data,
            reason,
            reviewed_at,
            counselor_name,
        });
    }
    Ok(Facts {
        data: reviews.last().map(|r| r.data.clone()),
        history: reviews,
    })
}
pub(super) fn read_imports(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<Snapshot, String> {
    let accounts=c.prepare("SELECT id,label,kind FROM statement_accounts WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid LIMIT 21").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok(Account{id:r.get(0)?,label:r.get(1)?,kind:r.get(2)?})).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    if accounts.len() > 20 {
        return Err(STORAGE_ERROR.into());
    }
    let doc_rows=c.prepare("SELECT id,account_id,file_name,source_hash,byte_count,imported_at,input_json,source_blob IS NOT NULL FROM import_documents WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid LIMIT 51").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,u32>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,bool>(7)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    if doc_rows.len() > 50 {
        return Err(STORAGE_ERROR.into());
    }
    let mut documents = Vec::new();
    for (id, account_id, file_name, sha256, byte_count, imported_at, input, source_retained) in
        doc_rows
    {
        let input: DocumentInput = decode(&input)?;
        if input.balance_order.is_some()
            && (input.pdf_selection.is_some() || input.mapping.balance.is_none())
        {
            return Err(STORAGE_ERROR.into());
        }
        documents.push(Document {
            balance_history: vec![],
            balance_reconciliation: None,
            id,
            account_id,
            file_name,
            sha256,
            byte_count: byte_count as usize,
            imported_at,
            source_retained,
            input,
            row_count: 0,
            source_expected_closing: None,
            source_difference: None,
            approved_difference: None,
            warnings: vec![],
        });
    }
    let balance_reviews=c.prepare("SELECT id,document_id,row_order,reason,reviewed_at,counselor_name FROM statement_balance_reviews WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid LIMIT 1001").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    if balance_reviews.len() > 1000 {
        return Err(STORAGE_ERROR.into());
    }
    for (id, document_id, order, reason, reviewed_at, counselor_name) in balance_reviews {
        let order = match order.as_deref() {
            Some("oldestFirst") => Some(crate::reconciliation::Order::OldestFirst),
            Some("newestFirst") => Some(crate::reconciliation::Order::NewestFirst),
            None => None,
            _ => return Err(STORAGE_ERROR.into()),
        };
        let d = documents
            .iter_mut()
            .find(|d| d.id == document_id)
            .ok_or(STORAGE_ERROR)?;
        if required_text(&reason, "Review reason", 500)? != reason
            || required_text(&counselor_name, "Counselor name", 120)? != counselor_name
            || chrono::DateTime::parse_from_rfc3339(&reviewed_at).is_err()
            || (order.is_some()
                && (d.input.mapping.balance.is_none() || d.input.pdf_selection.is_some()))
        {
            return Err(STORAGE_ERROR.into());
        }
        d.balance_history.push(crate::reconciliation::Review {
            id,
            document_id,
            order,
            reason,
            reviewed_at,
            counselor_name,
        });
    }
    let rows=c.prepare("SELECT r.id,r.document_id,r.raw_json,t.fields_json,t.status,t.linked_id,t.reason FROM import_records r JOIN transaction_choices t ON t.record_id=r.id AND t.client_id=r.client_id AND t.organization_id=r.organization_id AND t.counselor_id=r.counselor_id WHERE r.client_id=?1 AND r.organization_id=?2 AND r.counselor_id=?3 ORDER BY r.rowid LIMIT 5001").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,Option<String>>(5)?,r.get::<_,String>(6)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    if rows.len() > MAX_RECORDS {
        return Err(STORAGE_ERROR.into());
    }
    let mut records = Vec::new();
    for (id, document_id, raw, fields, st, linked_id, reason) in rows {
        let d = documents
            .iter()
            .find(|d| d.id == document_id)
            .ok_or(STORAGE_ERROR)?;
        let raw: Raw = decode(&raw)?;
        if let Some(selected) = &d.input.pdf_selection {
            if raw.extraction_method != "pdf-text-lines-v1"
                || !selected
                    .iter()
                    .any(|v| Some(v.page) == raw.page && u64::from(v.line) == raw.line)
                || raw.reported_line.is_none()
            {
                return Err(STORAGE_ERROR.into());
            }
        } else if raw.page.is_some()
            || raw.reported_line.is_some()
            || raw.extraction_method != "csv-utf8-v1"
        {
            return Err(STORAGE_ERROR.into());
        }
        records.push(Record {
            id,
            document_id,
            account_id: d.account_id.clone(),
            raw,
            fields: decode(&fields)?,
            status: decode_status(&st)?,
            linked_id,
            reason,
            warnings: vec![],
            duplicate_ids: vec![],
            bulk_eligible: false,
        });
    }
    let count:i64=c.query_row("SELECT COUNT(*) FROM import_records WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?;
    if count != records.len() as i64 {
        return Err(STORAGE_ERROR.into());
    }
    for d in &mut documents {
        if d.input.pdf_selection.is_some() {
            d.warnings.push("PDF statement checks include only the explicitly selected lines. Verify the original statement for completeness; headers and unselected lines were not imported.".into());
        }
        let rs = records
            .iter()
            .filter(|r| r.document_id == d.id)
            .collect::<Vec<_>>();
        d.row_count = rs.len();
        let order = d
            .balance_history
            .last()
            .map(|h| h.order.clone())
            .unwrap_or_else(|| d.input.balance_order.clone());
        let report = crate::reconciliation::check(&d.input, &rs, order)?;
        if report.conflicts() {
            d.warnings.push("Running-balance checks have an unresolved discrepancy or date-order conflict; review the statement individually.".into());
        }
        d.balance_reconciliation = Some(report);
        let original = rs
            .iter()
            .map(|r| {
                r.raw
                    .extracted
                    .amount
                    .as_ref()
                    .map(|v| signed_money(v))
                    .transpose()
            })
            .collect::<Result<Option<Vec<_>>, _>>()?;
        if let (Some(open), Some(close)) = (&d.input.opening_balance, &d.input.closing_balance) {
            if let Some(values) = original {
                let expected = signed_money(open)? + values.iter().sum::<i64>();
                d.source_expected_closing = Some(money(expected));
                let diff = expected - signed_money(close)?;
                d.source_difference = Some(money(diff));
                if diff != 0 {
                    d.warnings.push("Source transactions do not reconcile with the entered opening/closing balances. Verify statement completeness and mapping; uncertain rows need individual review.".into());
                }
            } else {
                d.warnings.push("Source reconciliation is unavailable because transaction amounts are incomplete.".into());
            }
            if rs
                .iter()
                .all(|r| matches!(r.status, Status::Approved | Status::Rejected))
            {
                let sum = rs
                    .iter()
                    .filter(|r| r.status == Status::Approved)
                    .map(|r| signed_money(r.fields.amount.as_deref().ok_or(STORAGE_ERROR)?))
                    .collect::<Result<Vec<_>, _>>()?
                    .iter()
                    .sum::<i64>();
                d.approved_difference =
                    Some(money(signed_money(open)? + sum - signed_money(close)?));
            }
        } else {
            d.warnings.push("Opening or closing balance is unknown; reconciliation is unavailable, not a zero difference.".into());
        }
        if d.input.period_start.is_none() || d.input.period_end.is_none() {
            d.warnings.push("Statement period is incomplete; row dates do not establish the full statement period.".into());
        }
    }
    let mut matches: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (i, r) in records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.status != Status::Rejected)
    {
        let mut keys = HashSet::new();
        for f in [&r.raw.extracted, &r.fields] {
            if let Some(k) = fingerprint(f) {
                keys.insert(format!("f{k}"));
            }
        }
        if let Some(reference) = &r.raw.reference {
            keys.insert(format!("r{reference}"));
        }
        for key in keys {
            matches
                .entry((r.account_id.clone(), key))
                .or_default()
                .push(i);
        }
    }
    let mut dup_sets = vec![HashSet::new(); records.len()];
    for indices in matches.values().filter(|v| v.len() > 1) {
        for i in indices {
            // Keep diagnostics bounded even if every source row overlaps.
            // Any match still blocks bulk approval; at most 20 IDs are shown.
            for j in indices.iter().take(21) {
                if i != j && dup_sets[*i].len() < 20 {
                    dup_sets[*i].insert(records[*j].id.clone());
                }
            }
        }
    }
    for (i, r) in records.iter_mut().enumerate() {
        r.duplicate_ids = dup_sets[i].iter().cloned().collect();
        r.duplicate_ids.sort();
        r.warnings = r.raw.issues.clone();
        if !r.duplicate_ids.is_empty() {
            r.warnings.push("Possible duplicate on this account (original/corrected date, amount and description, or source reference). It may be legitimate; review individually before including it. Up to 20 related IDs are shown.".into());
        }
        let d = documents
            .iter()
            .find(|d| d.id == r.document_id)
            .expect("document checked");
        if d.source_difference.as_ref().is_some_and(|v| v != "0.00") {
            r.warnings
                .push("Source statement has a reconciliation discrepancy.".into());
        }
        if d.input.mapping.balance.is_some() && d.input.pdf_selection.is_none() {
            let report = d.balance_reconciliation.as_ref().ok_or(STORAGE_ERROR)?;
            if report.conflicts() {
                r.warnings.push("Running-balance discrepancy or source-order conflict requires individual review.".into());
            } else if let Some(row) = report.rows.iter().find(|row| row.record_id == r.id) {
                if row.source.state != crate::reconciliation::State::Matched {
                    r.warnings.push(format!(
                        "Running-balance source check is incomplete: {}",
                        row.source.note
                    ));
                }
            }
        }
        if let Some(warning) = period_warning(d, &r.fields) {
            r.warnings.push(warning);
        }
        if let Some(error) = approval_error(&r.fields) {
            r.warnings.push(error);
        }
        if r.fields.kind == Kind::Transfer && r.linked_id.is_none() {
            r.warnings.push("An internal transfer needs a reviewed counterpart on another account before either leg is approved.".into());
        }
        r.bulk_eligible = r.status == Status::Pending
            && r.warnings.is_empty()
            && !matches!(
                r.fields.kind,
                Kind::Transfer | Kind::Refund | Kind::BalanceMovement
            )
            && r.fields == r.raw.extracted;
    }
    for r in records
        .iter()
        .filter(|r| r.status == Status::Approved && r.fields.kind == Kind::Transfer)
    {
        let linked = r
            .linked_id
            .as_ref()
            .and_then(|id| records.iter().find(|x| x.id == *id))
            .ok_or(STORAGE_ERROR)?;
        if linked.status != Status::Approved
            || linked.fields.kind != Kind::Transfer
            || linked.linked_id.as_deref() != Some(&r.id)
            || linked.account_id == r.account_id
            || signed_money(linked.fields.amount.as_deref().ok_or(STORAGE_ERROR)?)?
                != -signed_money(r.fields.amount.as_deref().ok_or(STORAGE_ERROR)?)?
        {
            return Err(STORAGE_ERROR.into());
        }
    }
    for r in records.iter().filter(|r| r.linked_id.is_some()) {
        if r.status != Status::Approved || r.fields.kind != Kind::Transfer {
            return Err(STORAGE_ERROR.into());
        }
    }
    let hs=c.prepare("SELECT id,record_id,from_status,to_status,to_fields_json,reason,reviewed_at,counselor_name FROM transaction_review_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid LIMIT 10001").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,String>(7)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    if hs.len() > MAX_HISTORY {
        return Err(STORAGE_ERROR.into());
    }
    let mut history = Vec::new();
    for (id, record_id, from, to, fields, reason, reviewed_at, counselor_name) in hs {
        history.push(History {
            id,
            record_id,
            from_status: decode_status(&from)?,
            to_status: decode_status(&to)?,
            fields: decode(&fields)?,
            reason,
            reviewed_at,
            counselor_name,
        });
    }
    let size: i64 = c.query_row("SELECT COALESCE(SUM(length(CAST(review_json AS BLOB))),0) FROM recurring_review_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?;
    if size > 8 * 1024 * 1024 {
        return Err(STORAGE_ERROR.into());
    }
    let recurring_rows = c.prepare("SELECT h.id,h.review_json,i.review_id IS NOT NULL FROM recurring_review_history h LEFT JOIN recurring_review_invalidations i ON i.review_id=h.id WHERE h.client_id=?1 AND h.organization_id=?2 AND h.counselor_id=?3 ORDER BY h.rowid LIMIT 1001").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,bool>(2)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    if recurring_rows.len() > 1000 {
        return Err(STORAGE_ERROR.into());
    }
    let mut recurring_history = Vec::new();
    for (id, value, invalidated) in recurring_rows {
        let mut review: crate::recurring::Review = decode(&value)?;
        review.invalidated = invalidated;
        if review.id != id {
            return Err(STORAGE_ERROR.into());
        }
        crate::recurring::validate_review(&review, &records, &history)
            .map_err(|_| STORAGE_ERROR.to_owned())?;
        recurring_history.push(review);
    }
    let merchant_groups = crate::recurring::analyze(&records, &recurring_history, &history)?;
    let summary = summarize(&records, &accounts)?;
    let financial_facts = read_financial_facts(c, w, client, &records, &history, &documents)?;
    Ok(Snapshot {
        financial_facts,
        merchant_groups,
        recurring_history,
        client_id: client.into(),
        revision: revision(c, w, client)?,
        accounts,
        documents,
        records,
        history,
        summary,
    })
}
// Persist invalidation in the same transaction as the source change. A return to
// old facts must never reactivate a prior counselor decision without a new review.
fn invalidate_recurring_changes(
    c: &Transaction,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(), String> {
    let snapshot = read_imports(c, w, client)?;
    let mut seen = HashSet::new();
    for review in snapshot.recurring_history.iter().rev() {
        if !seen.insert(&review.group_id) || review.invalidated {
            continue;
        }
        if !snapshot
            .merchant_groups
            .iter()
            .any(|g| g.id == review.group_id && g.evidence_hash == review.evidence_hash)
        {
            c.execute("INSERT INTO recurring_review_invalidations(review_id,changed_revision) VALUES(?1,?2)",params![review.id,snapshot.revision]).map_err(storage_error)?;
        }
    }
    Ok(())
}
struct ReviewUpdate<'a> {
    fields: &'a Fields,
    status: &'a Status,
    linked: Option<&'a str>,
    note: &'a str,
}
fn save_decision(
    c: &Transaction,
    w: &WorkspaceInfo,
    client: &str,
    r: &Record,
    update: ReviewUpdate<'_>,
) -> Result<(), String> {
    let ReviewUpdate {
        fields,
        status: st,
        linked,
        note,
    } = update;
    let count:i64=c.query_row("SELECT COUNT(*) FROM transaction_review_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?;
    if count >= MAX_HISTORY as i64 {
        return Err("This client reached the 10,000-review-history limit. Existing decisions are preserved.".into());
    }
    let changed=c.execute("UPDATE transaction_choices SET fields_json=?1,status=?2,linked_id=?3,reason=?4 WHERE record_id=?5 AND client_id=?6 AND organization_id=?7 AND counselor_id=?8",params![json(fields)?,status(st),linked,note,r.id,client,w.organization_id,w.counselor_id]).map_err(storage_error)?;
    if changed != 1 {
        return Err(STORAGE_ERROR.into());
    }
    c.execute("INSERT INTO transaction_review_history(id,record_id,client_id,organization_id,counselor_id,counselor_name,from_status,to_status,from_fields_json,to_fields_json,from_linked_id,to_linked_id,reason,reviewed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",params![Uuid::new_v4().to_string(),r.id,client,w.organization_id,w.counselor_id,w.counselor_name,status(&r.status),status(st),json(&r.fields)?,json(fields)?,r.linked_id,linked,note,timestamp()]).map_err(storage_error)?;
    Ok(())
}
impl Workspace {
    pub fn inspect_client_csv(&mut self, client: String, path: String) -> Result<Preview, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, &client)?;
        if Path::new(&path)
            .extension()
            .and_then(|v| v.to_str())
            .is_none_or(|v| !v.eq_ignore_ascii_case("csv"))
            || !regular_file_exists(Path::new(&path))?
        {
            return Err("Choose an existing local .csv file.".into());
        }
        let mut bytes = Zeroizing::new(Vec::new());
        File::open(&path)
            .map_err(|_| "The CSV could not be opened.".to_owned())?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "The CSV could not be read.".to_owned())?;
        let csv = read_csv(&bytes)?;
        let token = Uuid::new_v4().to_string();
        let sha256 = format!("{:x}", Sha256::digest(&*bytes));
        let file_name = Path::new(&path)
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or("Use a UTF-8 file name.")?
            .to_owned();
        if file_name.len() > 255 || file_name.chars().any(char::is_control) {
            return Err("Use a shorter file name.".into());
        }
        let result = Preview {
            pdf_pages: None,
            client_id: client.clone(),
            token: token.clone(),
            file_name: file_name.clone(),
            sha256: sha256.clone(),
            byte_count: bytes.len(),
            headers: csv.headers,
            sample_rows: csv.rows.iter().take(3).map(|(_, r)| r.clone()).collect(),
            row_count: csv.rows.len(),
        };
        self.pending_csv = Some(PendingCsv {
            client_id: client,
            token,
            bytes,
            file_name,
            sha256,
            created: Instant::now(),
            pdf_pages: None,
        });
        Ok(result)
    }
    pub fn inspect_client_pdf(&mut self, client: String, path: String) -> Result<Preview, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, &client)?;
        if Path::new(&path)
            .extension()
            .and_then(|v| v.to_str())
            .is_none_or(|v| !v.eq_ignore_ascii_case("pdf"))
            || !regular_file_exists(Path::new(&path))?
        {
            return Err("Choose an existing local .pdf file.".into());
        }
        let mut bytes = Zeroizing::new(Vec::new());
        File::open(&path)
            .map_err(|_| "The PDF could not be opened.")?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "The PDF could not be read.")?;
        let pages = crate::pdf_process::extract(&bytes)?;
        self.capture_pdf(client, Path::new(&path), bytes, pages)
    }
    fn capture_pdf(
        &mut self,
        client: String,
        path: &Path,
        bytes: Zeroizing<Vec<u8>>,
        pages: Vec<crate::pdf::Page>,
    ) -> Result<Preview, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, &client)?;
        crate::pdf::validate_pages(&pages)?;
        let file_name = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or("Use a UTF-8 file name.")?
            .to_owned();
        if file_name.len() > 255 || file_name.chars().any(char::is_control) {
            return Err("Use a shorter file name.".into());
        }
        let token = Uuid::new_v4().to_string();
        let sha256 = format!("{:x}", Sha256::digest(&*bytes));
        let result = Preview {
            client_id: client.clone(),
            token: token.clone(),
            file_name: file_name.clone(),
            sha256: sha256.clone(),
            byte_count: bytes.len(),
            headers: vec![],
            sample_rows: vec![],
            row_count: pages
                .iter()
                .map(|p| p.text.lines().filter(|l| !l.trim().is_empty()).count())
                .sum(),
            pdf_pages: Some(pages.clone()),
        };
        self.pending_csv = Some(PendingCsv {
            client_id: client,
            token,
            bytes,
            file_name,
            sha256,
            created: Instant::now(),
            pdf_pages: Some(pages),
        });
        Ok(result)
    }
    pub fn cancel_client_csv(&mut self, token: &str) {
        if self.pending_csv.as_ref().is_some_and(|p| p.token == token) {
            self.pending_csv = None;
        }
    }
    pub fn get_client_imports(&mut self, client: &str) -> Result<Snapshot, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        read_imports(&s.connection, &s.info, client)
    }
    pub fn stage_client_csv(
        &mut self,
        client: &str,
        input: StageInput,
    ) -> Result<Snapshot, String> {
        {
            let s = self.session()?;
            scoped_client(&s.connection, &s.info, client)?;
        }
        let pending=self.pending_csv.as_ref().filter(|p|p.token==input.token&&p.client_id==client&&!p.expired()).ok_or("Select and inspect this client's CSV again; the preview is unavailable or expired.")?;
        let mut document = normalize_document(input.document)?;
        let is_pdf = pending.pdf_pages.is_some();
        let rows = if let Some(pages) = &pending.pdf_pages {
            if document.mapping.amount_mode != AmountMode::Signed {
                return Err("PDF extraction currently supports one signed amount at the end of each selected line.".into());
            }
            let selection = document
                .pdf_selection
                .as_ref()
                .ok_or("Select the PDF transaction lines explicitly.")?;
            crate::pdf::records(
                pages,
                selection,
                &document.mapping.date_format,
                document.mapping.signed_is_debit,
            )?
        } else {
            if document.pdf_selection.is_some() {
                return Err("A CSV preview cannot be staged as a PDF.".into());
            }
            let csv = read_csv(&pending.bytes)?;
            extract(&csv, &document.mapping)?
        };
        let bytes = if document.retain_source {
            Some(Zeroizing::new(pending.bytes.to_vec()))
        } else {
            None
        };
        let file_name = pending.file_name.clone();
        let hash = pending.sha256.clone();
        let size = pending.bytes.len();
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        bump(&tx, &s.info, client, input.expected_revision)?;
        let current = read_imports(&tx, &s.info, client)?;
        if current.documents.len() >= 50 || current.records.len() + rows.len() > MAX_RECORDS {
            return Err(
                "Use at most 50 source documents and 5,000 imported rows per client in this stage."
                    .into(),
            );
        }
        let account = if let Some(id) = &document.account_id {
            current
                .accounts
                .iter()
                .find(|a| a.id == *id)
                .ok_or("Choose an account belonging to this client.")?
                .id
                .clone()
        } else {
            if current
                .accounts
                .iter()
                .any(|a| a.label.to_lowercase() == document.account_label.to_lowercase())
            {
                return Err("An account with this label already exists. Select it or enter a distinct label for a different account.".into());
            }
            if current.accounts.len() >= 20 {
                return Err("Use at most 20 statement accounts per client.".into());
            }
            let id = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO statement_accounts(id,client_id,organization_id,counselor_id,label,kind) VALUES(?1,?2,?3,?4,?5,?6)",params![id,client,s.info.organization_id,s.info.counselor_id,document.account_label,document.account_kind]).map_err(storage_error)?;
            id
        };
        let saved_account = current.accounts.iter().find(|a| a.id == account);
        if let Some(a) = saved_account {
            document.account_label = a.label.clone();
            document.account_kind = a.kind.clone();
        }
        let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM import_documents WHERE client_id=?1 AND account_id=?2 AND source_hash=?3)",params![client,account,hash],|r|r.get(0)).map_err(storage_error)?;
        if duplicate {
            return Err("This exact source file was already imported for this account. Review its existing rows instead of importing it again.".into());
        }
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO import_documents(id,client_id,organization_id,counselor_id,account_id,file_name,source_hash,byte_count,imported_at,input_json,source_blob) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![id,client,s.info.organization_id,s.info.counselor_id,account,file_name,hash,size as i64,timestamp(),json(&document)?,bytes.as_ref().map(|v|v.as_slice())]).map_err(storage_error)?;
        let mut added = HashSet::new();
        for raw in rows {
            let rid = Uuid::new_v4().to_string();
            added.insert(rid.clone());
            tx.execute("INSERT INTO import_records(id,client_id,organization_id,counselor_id,document_id,raw_json) VALUES(?1,?2,?3,?4,?5,?6)",params![rid,client,s.info.organization_id,s.info.counselor_id,id,json(&raw)?]).map_err(storage_error)?;
            tx.execute("INSERT INTO transaction_choices(record_id,client_id,organization_id,counselor_id,status,fields_json,reason) VALUES(?1,?2,?3,?4,?5,?6,'')",params![rid,client,s.info.organization_id,s.info.counselor_id,if raw.issues.is_empty(){"Pending"}else{"Needs Clarification"},json(&raw.extracted)?]).map_err(storage_error)?;
        }
        let checked = read_imports(&tx, &s.info, client)?;
        for r in checked.records.iter().filter(|r| {
            added.contains(&r.id) && r.status == Status::Pending && !r.warnings.is_empty()
        }) {
            tx.execute(
                "UPDATE transaction_choices SET status='Needs Clarification' WHERE record_id=?1",
                [&r.id],
            )
            .map_err(storage_error)?;
        }
        audit(
            &tx,
            &s.info,
            if is_pdf {
                "pdf_imported"
            } else {
                "csv_imported"
            },
            Some(&id),
        )?;
        invalidate_recurring_changes(&tx, &s.info, client)?;
        let result = read_imports(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        self.pending_csv = None;
        Ok(result)
    }
    pub fn review_transaction(
        &mut self,
        client: &str,
        input: ReviewInput,
    ) -> Result<Snapshot, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        bump(&tx, &s.info, client, input.expected_revision)?;
        let snapshot = read_imports(&tx, &s.info, client)?;
        let r = snapshot
            .records
            .iter()
            .find(|r| r.id == input.record_id)
            .ok_or("This record is unavailable for this client.")?;
        let fields = validate_fields(input.fields)?;
        let note = reason(&input.reason)?;
        if r.status == Status::Approved
            && (input.status != Status::Approved || fields != r.fields)
            && note.is_empty()
        {
            return Err("Explain the correction or reversal of the prior approval.".into());
        }
        if input.status == Status::Approved {
            if let Some(error) = approval_error(&fields) {
                return Err(error);
            }
            if fields.kind == Kind::Transfer {
                return Err("Approve internal transfers through the paired-transfer control, never as one isolated leg.".into());
            }
            let uncertain = overlaps(&snapshot, r, &fields)
                || !r.warnings.is_empty()
                || fields != r.raw.extracted
                || matches!(fields.kind, Kind::Refund | Kind::BalanceMovement);
            if uncertain && (!input.acknowledge_warnings || note.is_empty()) {
                return Err("Individually acknowledge the source warnings/corrections and explain this approval.".into());
            }
            if r.linked_id.is_some() {
                return Err("Reverse the linked transfer before reclassifying one leg.".into());
            }
        }
        if let Some(linked) = &r.linked_id {
            let other = snapshot
                .records
                .iter()
                .find(|x| x.id == *linked)
                .ok_or(STORAGE_ERROR)?;
            save_decision(
                &tx,
                &s.info,
                client,
                other,
                ReviewUpdate {
                    fields: &other.fields,
                    status: &Status::NeedsClarification,
                    linked: None,
                    note: &format!("Transfer approval reversed: {note}"),
                },
            )?;
        }
        save_decision(
            &tx,
            &s.info,
            client,
            r,
            ReviewUpdate {
                fields: &fields,
                status: &input.status,
                linked: None,
                note: &note,
            },
        )?;
        audit(&tx, &s.info, "transaction_reviewed", Some(&r.id))?;
        invalidate_recurring_changes(&tx, &s.info, client)?;
        let result = read_imports(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn review_financial_facts(
        &mut self,
        client: &str,
        input: crate::financial_facts::Input,
    ) -> Result<Snapshot, String> {
        use crate::financial_facts;
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        if !input.acknowledge {
            return Err("Confirm that these are verified facts, not planning estimates.".into());
        }
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let current = read_imports(&tx, &s.info, client)?;
        let data = financial_facts::normalize(input.data)?;
        if data
            .as_of
            .as_deref()
            .map(iso_date)
            .transpose()?
            .is_some_and(|d| d > chrono::Utc::now().date_naive())
        {
            return Err("Balance facts cannot have a future as-of date.".into());
        }
        let ids = current.documents.iter().map(|d| d.id.as_str()).collect();
        financial_facts::validate_bindings(&data, &current.records, &current.history, &ids)?;
        for c in &data.classifications {
            let unchanged = current
                .financial_facts
                .data
                .as_ref()
                .is_some_and(|old| old.classifications.contains(c));
            if !unchanged
                && !current
                    .records
                    .iter()
                    .any(|r| r.id == c.record_id && financial_facts::active(c, r, &current.history))
            {
                return Err("A new or changed classification requires the current approved source decision. Reverify the transaction first.".into());
            }
        }
        let json = serde_json::to_string(&data).map_err(|_| STORAGE_ERROR.to_owned())?;
        if current.financial_facts.history.len() >= 100
            || current
                .financial_facts
                .history
                .iter()
                .map(|r| {
                    serde_json::to_vec(&r.data)
                        .map(|v| v.len())
                        .unwrap_or(financial_facts::MAX_JSON)
                })
                .sum::<usize>()
                + json.len()
                > 8 * 1024 * 1024
        {
            return Err(
                "Financial fact history reached its local safety limit; nothing was saved.".into(),
            );
        }
        let reason = required_text(&input.reason, "Facts review reason", 500)?;
        bump(&tx, &s.info, client, input.expected_revision)?;
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO financial_fact_reviews(id,client_id,organization_id,counselor_id,data_json,reason,reviewed_at,counselor_name) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![id,client,s.info.organization_id,s.info.counselor_id,json,reason,timestamp(),s.info.counselor_name]).map_err(storage_error)?;
        audit(&tx, &s.info, "financial_facts_reviewed", Some(&id))?;
        let result = read_imports(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn review_statement_balances(
        &mut self,
        client: &str,
        input: crate::reconciliation::ReviewInput,
    ) -> Result<Snapshot, String> {
        use crate::reconciliation::Order;
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        bump(&tx, &s.info, client, input.expected_revision)?;
        let snapshot = read_imports(&tx, &s.info, client)?;
        let d = snapshot
            .documents
            .iter()
            .find(|d| d.id == input.document_id)
            .ok_or("This source document is unavailable for this client.")?;
        if input.order.is_some()
            && (d.input.mapping.balance.is_none() || d.input.pdf_selection.is_some())
        {
            return Err(
                "Choose a CSV source with a mapped after-transaction balance column.".into(),
            );
        }
        if snapshot
            .documents
            .iter()
            .map(|d| d.balance_history.len())
            .sum::<usize>()
            >= 1000
        {
            return Err("Statement balance review history reached its local safety limit; this choice was not saved.".into());
        }
        let note = required_text(&input.reason, "Source order review reason", 500)?;
        let order = input.order.map(|v| match v {
            Order::OldestFirst => "oldestFirst",
            Order::NewestFirst => "newestFirst",
        });
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO statement_balance_reviews(id,client_id,organization_id,counselor_id,document_id,row_order,reason,reviewed_at,counselor_name) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,client,s.info.organization_id,s.info.counselor_id,d.id,order,note,timestamp(),s.info.counselor_name]).map_err(storage_error)?;
        audit(&tx, &s.info, "statement_balance_reviewed", Some(&id))?;
        invalidate_recurring_changes(&tx, &s.info, client)?;
        let result = read_imports(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn review_recurring_payment(
        &mut self,
        client: &str,
        input: crate::recurring::ReviewInput,
    ) -> Result<Snapshot, String> {
        use crate::recurring::{Decision, Review};
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        bump(&tx, &s.info, client, input.expected_revision)?;
        let snapshot = read_imports(&tx, &s.info, client)?;
        let g = snapshot
            .merchant_groups
            .iter()
            .find(|g| g.id == input.group_id)
            .ok_or("This payment group is unavailable; reopen the client.")?;
        if g.evidence_hash != input.evidence_hash {
            return Err(
                "Payment evidence changed. Reopen the client and review the current payments."
                    .into(),
            );
        }
        if g.evidence.len() > 500 {
            return Err("Separate this group before reviewing more than 500 payments.".into());
        }
        if input.decision == Decision::Confirmed && (!g.can_confirm || input.frequency.is_none()) {
            return Err("Confirm at least three approved payments on distinct dates, resolve possible duplicates, and choose a frequency.".into());
        }
        if input.decision == Decision::Dismissed
            && (input.frequency.is_some() || input.subscription)
        {
            return Err(
                "A dismissed pattern cannot have a frequency or subscription classification."
                    .into(),
            );
        }
        let label = required_text(&input.label, "Payment label", 120)?;
        let reason = required_text(&input.reason, "Payment review reason", 500)?;
        let r = Review {
            invalidated: false,
            id: Uuid::new_v4().to_string(),
            group_id: g.id.clone(),
            evidence_hash: g.evidence_hash.clone(),
            label,
            decision: input.decision,
            frequency: input.frequency,
            subscription: input.subscription,
            reason,
            reviewed_at: timestamp(),
            counselor_name: s.info.counselor_name.clone(),
            evidence: g.evidence.clone(),
        };
        let value = json(&r)?;
        let size:i64=tx.query_row("SELECT COALESCE(SUM(length(CAST(review_json AS BLOB))),0) FROM recurring_review_history WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,s.info.organization_id,s.info.counselor_id],|r|r.get(0)).map_err(storage_error)?;
        if snapshot.recurring_history.len() >= 1000
            || value.len() > 262144
            || size + value.len() as i64 > 8 * 1024 * 1024
        {
            return Err("Payment review history reached its local safety limit; this decision was not saved.".into());
        }
        tx.execute("INSERT INTO recurring_review_history(id,client_id,organization_id,counselor_id,review_json) VALUES(?1,?2,?3,?4,?5)",params![r.id,client,s.info.organization_id,s.info.counselor_id,value]).map_err(storage_error)?;
        audit(&tx, &s.info, "recurring_reviewed", Some(&r.id))?;
        invalidate_recurring_changes(&tx, &s.info, client)?;
        let result = read_imports(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn bulk_approve_transactions(
        &mut self,
        client: &str,
        input: BulkInput,
    ) -> Result<Snapshot, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        bump(&tx, &s.info, client, input.expected_revision)?;
        let current = read_imports(&tx, &s.info, client)?;
        if input.record_ids.is_empty()
            || input.record_ids.len() > 100
            || input.record_ids.iter().collect::<HashSet<_>>().len() != input.record_ids.len()
        {
            return Err("Select 1–100 distinct validated pending rows for bulk approval.".into());
        }
        for id in &input.record_ids {
            let r=current.records.iter().find(|r|r.id==*id).filter(|r|r.bulk_eligible).ok_or("Bulk approval includes an uncertain, corrected, duplicate, or unavailable row. Review it individually.")?;
            save_decision(
                &tx,
                &s.info,
                client,
                r,
                ReviewUpdate {
                    fields: &r.fields,
                    status: &Status::Approved,
                    linked: None,
                    note: "Explicit bulk approval of validated pending source row",
                },
            )?;
        }
        audit(&tx, &s.info, "transaction_reviewed", Some(client))?;
        invalidate_recurring_changes(&tx, &s.info, client)?;
        let result = read_imports(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn pair_transfer_transactions(
        &mut self,
        client: &str,
        input: PairInput,
    ) -> Result<Snapshot, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        bump(&tx, &s.info, client, input.expected_revision)?;
        let current = read_imports(&tx, &s.info, client)?;
        let note = required_text(&input.reason, "Transfer review reason", 500)?;
        let left = current
            .records
            .iter()
            .find(|r| r.id == input.left_id)
            .ok_or("Select two records belonging to this client.")?;
        let right = current
            .records
            .iter()
            .find(|r| r.id == input.right_id)
            .ok_or("Select two records belonging to this client.")?;
        if left.id == right.id
            || left.account_id == right.account_id
            || left.status == Status::Rejected
            || right.status == Status::Rejected
            || left.linked_id.is_some()
            || right.linked_id.is_some()
        {
            return Err(
                "Select two unlinked, unrejected records from different client accounts.".into(),
            );
        }
        let l = signed_money(
            left.fields
                .amount
                .as_deref()
                .ok_or("Correct both transfer amounts first.")?,
        )?;
        let r = signed_money(
            right
                .fields
                .amount
                .as_deref()
                .ok_or("Correct both transfer amounts first.")?,
        )?;
        if l == 0 || l != -r {
            return Err(
                "Internal transfer legs need equal nonzero amounts with opposite signs.".into(),
            );
        }
        for (record, other) in [(left, right), (right, left)] {
            let mut fields = record.fields.clone();
            fields.kind = Kind::Transfer;
            if fields.category.is_empty() {
                fields.category = "Transfer".into();
            }
            if let Some(error) = approval_error(&fields) {
                return Err(error);
            }
            save_decision(
                &tx,
                &s.info,
                client,
                record,
                ReviewUpdate {
                    fields: &fields,
                    status: &Status::Approved,
                    linked: Some(&other.id),
                    note: &note,
                },
            )?;
        }
        audit(&tx, &s.info, "transfers_linked", Some(&left.id))?;
        invalidate_recurring_changes(&tx, &s.info, client)?;
        let result = read_imports(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::{
        budget_input, cashflow_input, client_input, configured, debt_input, savings_input, PASSWORD,
    };
    use tempfile::tempdir;
    pub(super) const CHECKING:&str="Date,Description,Amount,Category,Type\n2026-09-01,Synthetic wages,2000,Wages,income\n2026-09-02,Synthetic rent,-1200,Housing,expense\n2026-09-03,Synthetic fee,-10.01,Fees,fee\n2026-09-04,Synthetic refund,20.01,Housing,refund\n2026-09-05,Synthetic reserve transfer,-200,Savings,transfer\n";
    fn doc(label: &str) -> DocumentInput {
        DocumentInput {
            balance_order: None,
            pdf_selection: None,
            account_id: None,
            account_label: label.into(),
            account_kind: "checking".into(),
            period_start: Some("2026-09-01".into()),
            period_end: Some("2026-09-30".into()),
            opening_balance: Some("1000".into()),
            closing_balance: Some("1610".into()),
            retain_source: false,
            mapping: crate::imports::tests::mapping(),
        }
    }
    fn stage(
        w: &mut Workspace,
        client: &str,
        dir: &Path,
        name: &str,
        content: &str,
        d: DocumentInput,
    ) -> Snapshot {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        let revision = w.get_client_imports(client).unwrap().revision;
        let p = w
            .inspect_client_csv(client.into(), path.to_string_lossy().into())
            .unwrap();
        w.stage_client_csv(
            client,
            StageInput {
                expected_revision: revision,
                token: p.token,
                document: d,
            },
        )
        .unwrap()
    }
    fn review(
        w: &mut Workspace,
        client: &str,
        id: &str,
        st: Status,
        note: &str,
        ack: bool,
    ) -> Result<Snapshot, String> {
        let s = w.get_client_imports(client)?;
        let r = s.records.iter().find(|r| r.id == id).unwrap();
        w.review_transaction(
            client,
            ReviewInput {
                expected_revision: s.revision,
                record_id: id.into(),
                status: st,
                fields: r.fields.clone(),
                reason: note.into(),
                acknowledge_warnings: ack,
            },
        )
    }
    fn facts_fixture(w: &mut Workspace, client: &str, dir: &Path) -> Snapshot {
        let s=stage(w,client,dir,"facts.csv","Date,Description,Amount,Category,Type\n2026-09-01,Fictional payroll,2000.01,Wages,income\n2026-09-02,Fictional rent,-1200.02,Housing,expense\n",doc("Facts checking"));
        let ids = s.records.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
        for id in ids {
            review(
                w,
                client,
                &id,
                Status::Approved,
                "Verified fictional source",
                true,
            )
            .unwrap();
        }
        w.get_client_imports(client).unwrap()
    }
    fn facts_data(s: &Snapshot) -> crate::financial_facts::Data {
        use crate::financial_facts::{Balance, BalanceKind, Classification, Data, Role};
        Data {
            as_of: Some("2026-09-30".into()),
            full_inventory: true,
            balances: vec![
                ("Checking", BalanceKind::Cash, "800.01", None),
                ("Emergency", BalanceKind::AccessibleSavings, "2400.03", None),
                ("Reserved", BalanceKind::RestrictedSavings, "300.03", None),
                ("Property", BalanceKind::OtherAsset, "1000.04", None),
                ("Loan", BalanceKind::Debt, "500.05", Some("100.01")),
            ]
            .into_iter()
            .map(|(label, kind, amount, payment)| Balance {
                label: label.into(),
                kind,
                amount: Some(amount.into()),
                monthly_payment: payment.map(str::to_owned),
                evidence: "Verified fictional dated account evidence".into(),
                document_id: Some(s.documents[0].id.clone()),
            })
            .collect(),
            classifications: s
                .records
                .iter()
                .map(|r| Classification {
                    record_id: r.id.clone(),
                    approval_id: s
                        .history
                        .iter()
                        .rev()
                        .find(|h| h.record_id == r.id)
                        .unwrap()
                        .id
                        .clone(),
                    role: if r.fields.kind == Kind::Income {
                        Role::TakeHomePay
                    } else {
                        Role::Necessary
                    },
                })
                .collect(),
        }
    }
    fn assessment_input(s: &Snapshot, c: &Client) -> crate::assessments::Input {
        crate::assessments::Input {
            selected_scenario: None,
            supporting: None,
            planning: None,
            expected_revision: s.revision,
            expected_profile_updated_at: c.updated_at.clone(),
            from_month: "2026-09".into(),
            through_month: "2026-09".into(),
            title: "Fictional September assessment".into(),
            findings: "Approved source review; missing statement coverage remains disclosed."
                .into(),
            recommendations: "Verify the remaining records before making commitments.".into(),
            follow_up: "Review the fictional sources at the next appointment.".into(),
            acknowledge: true,
        }
    }
    fn saved_planning_fixture(w: &mut Workspace, client: &str) {
        let mut budget = budget_input(0);
        for (label, kind, amount) in [
            ("Necessary", crate::budget::Kind::Necessary, "500"),
            ("Discretionary", crate::budget::Kind::Discretionary, "200"),
            ("Savings", crate::budget::Kind::Savings, "250"),
            ("Debt", crate::budget::Kind::DebtPayment, "150"),
        ] {
            budget.lines.push(crate::budget::BudgetLine {
                label: label.into(),
                kind,
                amount: Some(amount.into()),
                frequency: crate::budget::Frequency::Monthly,
                estimated: false,
            });
        }
        w.save_client_budget(client, budget).unwrap();
        w.save_client_debt_plan(client, debt_input(0)).unwrap();
        w.save_client_savings_plan(client, savings_input(0))
            .unwrap();
        w.save_client_cashflow_plan(client, cashflow_input(0))
            .unwrap();
    }
    fn scenario_input(w: &mut Workspace, s: &Snapshot, c: &Client) -> crate::assessments::Input {
        let mut input = assessment_input(s, c);
        let plans = w.get_assessment_planning(&c.id).unwrap();
        input.planning = Some(crate::planning::Selection {
            expected_revisions: plans.revisions(),
            label: "Fictional primary plan".into(),
            assumptions: "Entered planning estimates; lender dates remain to be verified.".into(),
            debt_strategy: crate::planning::Strategy::Snowball,
        });
        input
    }
    #[test]
    fn planning_snapshot_keeps_independent_historical_figures_and_original_entered_models() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let source = facts_fixture(&mut w, &c.id, dir.path());
        saved_planning_fixture(&mut w, &c.id);
        let input = scenario_input(&mut w, &source, &c);
        let saved = w.save_assessment_record(&c.id, input).unwrap();
        let id = saved.payload.id.clone();
        let planning = saved.payload.planning.as_ref().unwrap();
        assert_eq!(saved.payload.format, 2);
        assert_eq!(saved.payload.assessment.operating_cashflow, "799.99");
        assert_eq!(planning.plans.budget.summary.monthly_income, "3250.00");
        assert_eq!(planning.plans.budget.summary.unallocated, "2150.00");
        assert_eq!(
            crate::planning::projection(planning)
                .unwrap()
                .total_interest,
            "43.10"
        );
        assert_eq!(
            planning
                .plans
                .savings
                .summary
                .as_ref()
                .unwrap()
                .emergency_target
                .as_deref(),
            Some("1500.00")
        );
        assert_eq!(
            planning
                .plans
                .savings
                .summary
                .as_ref()
                .unwrap()
                .emergency
                .completion_date
                .as_deref(),
            Some("2027-01-31")
        );
        let original = serde_json::to_value(planning).unwrap();
        let digest = saved.digest.clone();
        let rev = source.revision;
        let mut edited = budget_input(1);
        edited.lines[0].amount = Some("2000.01".into());
        w.save_client_budget(&c.id, edited).unwrap();
        let old = w.get_assessment_record(&c.id, &id).unwrap();
        assert!(old.needs_review);
        assert!(old
            .review_reasons
            .iter()
            .any(|r| r.contains("Saved budget")));
        assert_eq!(old.digest, digest);
        assert_eq!(
            serde_json::to_value(old.payload.planning.as_ref().unwrap()).unwrap(),
            original
        );
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, rev);
        assert!(w.list_assessment_records(&c.id).unwrap().records[0].needs_review);
        let exported = w
            .export_assessment_pdf(
                &c.id,
                &id,
                dir.path().join("scenario.pdf").to_str().unwrap(),
            )
            .unwrap();
        let doc = lopdf::Document::load(exported.path).unwrap();
        let text = doc
            .extract_text(&doc.get_pages().keys().copied().collect::<Vec<_>>())
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for expected in [
            "Captured planning scenario / hypothetical",
            "Fictional primary plan",
            "Monthly income: USD 3250.00",
            "Projected total interest: USD 43.10",
            "Primary emergency completion: 2027-01-31",
            "Operating cash flow: USD 799.99",
            "NEEDS REVIEW",
            "Source / destination account",
        ] {
            assert!(text.contains(expected), "Missing {expected}");
        }
        let rejected = review(
            &mut w,
            &c.id,
            &source.records[1].id,
            Status::Rejected,
            "Withdraw historical rent",
            true,
        )
        .unwrap();
        assert_eq!(
            w.get_assessment_record(&c.id, &id)
                .unwrap()
                .payload
                .assessment
                .operating_cashflow,
            "799.99"
        );
        let mut next = scenario_input(&mut w, &rejected, &c);
        next.planning.as_mut().unwrap().debt_strategy = crate::planning::Strategy::Avalanche;
        let new = w.save_assessment_record(&c.id, next).unwrap();
        assert_eq!(new.payload.assessment.operating_cashflow, "2000.01");
        assert_eq!(new.payload.planning.as_ref().unwrap().revisions.budget, 2);
        assert_eq!(
            new.payload.planning.as_ref().unwrap().debt_strategy,
            crate::planning::Strategy::Avalanche
        );
        assert_eq!(
            crate::planning::projection(new.payload.planning.as_ref().unwrap())
                .unwrap()
                .total_interest,
            "35.35"
        );
        assert!(!new.needs_review);
    }
    #[test]
    fn planning_snapshot_rejects_stale_choices_missing_budget_scope_and_unsupported_projection_atomically(
    ) {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let source = facts_fixture(&mut w, &c.id, dir.path());
        let mut input = scenario_input(&mut w, &source, &c);
        input.planning.as_mut().unwrap().debt_strategy = crate::planning::Strategy::None;
        assert!(w.save_assessment_record(&c.id, input).is_err());
        saved_planning_fixture(&mut w, &c.id);
        let stale = scenario_input(&mut w, &source, &c);
        w.save_client_cashflow_plan(&c.id, cashflow_input(1))
            .unwrap();
        assert!(w
            .save_assessment_record(&c.id, stale)
            .err()
            .unwrap()
            .contains("changed elsewhere"));
        assert!(w.list_assessment_records(&c.id).unwrap().records.is_empty());
        let mut debt = debt_input(1);
        debt.data.accounts[0].interest_model = crate::debt::InterestModel::Daily;
        w.save_client_debt_plan(&c.id, debt).unwrap();
        let input = scenario_input(&mut w, &source, &c);
        assert!(w
            .save_assessment_record(&c.id, input)
            .err()
            .unwrap()
            .contains("no supported projection"));
        let mut input = scenario_input(&mut w, &source, &c);
        input.planning.as_mut().unwrap().debt_strategy = crate::planning::Strategy::None;
        let saved = w.save_assessment_record(&c.id, input).unwrap();
        assert!(crate::planning::projection(saved.payload.planning.as_ref().unwrap()).is_none());
        let other = w
            .save_client(ClientInput {
                first_name: "Other".into(),
                ..client_input()
            })
            .unwrap();
        assert!(w
            .get_assessment_record(&other.id, &saved.payload.id)
            .is_err());
        assert_eq!(
            w.get_assessment_planning(&other.id)
                .unwrap()
                .revisions()
                .budget,
            0
        );
        assert!(w.get_assessment_planning("foreign-client").is_err());
        w.lock_workspace();
        assert!(w
            .get_assessment_planning(&c.id)
            .err()
            .unwrap()
            .contains("locked"));
    }
    #[test]
    fn planning_snapshot_validation_rejects_forged_outcomes_context_identity_and_legacy_attachments(
    ) {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let source = facts_fixture(&mut w, &c.id, dir.path());
        saved_planning_fixture(&mut w, &c.id);
        let input = scenario_input(&mut w, &source, &c);
        let saved = w.save_assessment_record(&c.id, input).unwrap();
        let value = serde_json::to_value(&saved.payload).unwrap();
        for (pointer, bad) in [
            (
                "/planning/plans/budget/summary/monthlyIncome",
                serde_json::json!("9999.99"),
            ),
            (
                "/planning/plans/debt/comparison/snowball/totalInterest",
                serde_json::json!("0.00"),
            ),
            (
                "/planning/plans/savings/summary/emergency/completionDate",
                serde_json::json!("2026-01-01"),
            ),
            (
                "/planning/plans/cashflow/summary/combinedClosing",
                serde_json::json!("9999.99"),
            ),
            (
                "/planning/plans/savings/context/budgetRevision",
                serde_json::json!(9),
            ),
            (
                "/planning/plans/debt/clientId",
                serde_json::json!("other-client"),
            ),
            ("/planning/revisions/budget", serde_json::json!(2)),
            ("/planning/calculator", serde_json::json!("unsupported-v2")),
            ("/format", serde_json::json!(1)),
        ] {
            let mut changed = value.clone();
            *changed.pointer_mut(pointer).unwrap() = bad;
            let p: crate::assessments::Payload = serde_json::from_value(changed).unwrap();
            assert!(
                crate::assessments::validate(&p).is_err(),
                "Accepted {pointer}"
            );
        }
        let mut extra = value.clone();
        extra["planning"]["plans"]["budget"]["unrecognized"] = serde_json::json!(true);
        assert!(serde_json::from_value::<crate::assessments::Payload>(extra).is_err());
        let mut changed = value.clone();
        changed["planning"]["plans"]["savings"]["summary"]["emergencyTarget"] =
            serde_json::json!("999.99");
        let json = serde_json::to_string(&changed).unwrap();
        let hash = format!("{:x}", Sha256::digest(json.as_bytes()));
        let conn = &w.session.as_ref().unwrap().connection;
        conn.execute_batch("DROP TRIGGER assessment_records_no_update;")
            .unwrap();
        conn.execute(
            "UPDATE client_assessment_records SET payload_json=?1,digest=?2 WHERE id=?3",
            params![json, hash, saved.payload.id],
        )
        .unwrap();
        assert!(w.get_assessment_record(&c.id, &saved.payload.id).is_err());
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            w.get_client_imports(&c.id).unwrap().revision,
            source.revision
        );
    }
    #[test]
    fn planning_snapshot_restart_and_recovery_keep_historical_models_and_read_format_one() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let source = facts_fixture(&mut w, &c.id, dir.path());
        let old = w
            .save_assessment_record(&c.id, assessment_input(&source, &c))
            .unwrap();
        let mut legacy = serde_json::to_value(&old.payload).unwrap();
        legacy["format"] = serde_json::json!(1);
        legacy["appVersion"] = serde_json::json!("0.14.0");
        assert!(!legacy.as_object().unwrap().contains_key("planning"));
        let json = serde_json::to_string(&legacy).unwrap();
        let hash = format!("{:x}", Sha256::digest(json.as_bytes()));
        let conn = &w.session.as_ref().unwrap().connection;
        conn.execute_batch("DROP TRIGGER assessment_records_no_update;")
            .unwrap();
        conn.execute(
            "UPDATE client_assessment_records SET payload_json=?1,digest=?2 WHERE id=?3",
            params![json, hash, old.payload.id],
        )
        .unwrap();
        conn.execute_batch(
            include_str!("assessments-schema.sql")
                .lines()
                .find(|l| l.starts_with("CREATE TRIGGER assessment_records_no_update "))
                .unwrap(),
        )
        .unwrap();
        saved_planning_fixture(&mut w, &c.id);
        let input = scenario_input(&mut w, &source, &c);
        let saved = w.save_assessment_record(&c.id, input).unwrap();
        let digest = saved.digest.clone();
        w.save_client_budget(&c.id, budget_input(1)).unwrap();
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        drop(w);
        let mut w = Workspace::new(root.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let legacy = w.get_assessment_record(&c.id, &old.payload.id).unwrap();
        assert_eq!(legacy.payload.format, 1);
        assert!(legacy.payload.planning.is_none());
        assert!(!legacy.needs_review);
        let current = w.get_assessment_record(&c.id, &saved.payload.id).unwrap();
        assert!(current.needs_review);
        assert_eq!(current.digest, digest);
        assert_eq!(
            crate::planning::projection(current.payload.planning.as_ref().unwrap())
                .unwrap()
                .total_interest,
            "43.10"
        );
        w.lock_workspace();
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_assessment_record(&c.id, &saved.payload.id).unwrap();
        assert_eq!(restored.digest, digest);
        assert!(restored.needs_review);
        assert_eq!(
            restored
                .payload
                .planning
                .as_ref()
                .unwrap()
                .plans
                .budget
                .summary
                .unallocated,
            "2150.00"
        );
        assert_eq!(
            w.get_assessment_record(&c.id, &old.payload.id)
                .unwrap()
                .payload
                .format,
            1
        );
    }
    #[test]
    fn assessment_pdf_exports_original_figures_unicode_evidence_and_review_status_without_writes() {
        let (_root, mut w, _) = configured();
        let out = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, out.path());
        let data = facts_data(&s);
        let s = w
            .review_financial_facts(&c.id, facts_input(&s, data))
            .unwrap();
        let mut input = assessment_input(&s, &c);
        input.findings = format!(
            "José’s fictional café — Ελληνικά Кириллица\n{}\nFINAL NOTE",
            "VeryLongUnbrokenFictionalEvidence".repeat(100)
        );
        let saved = w.save_assessment_record(&c.id, input).unwrap();
        let id = &saved.payload.id;
        let current = w
            .export_assessment_pdf(&c.id, id, out.path().join("current.pdf").to_str().unwrap())
            .unwrap();
        assert!(current.pages > 3);
        let bytes = fs::read(&current.path).unwrap();
        let doc = lopdf::Document::load_mem(&bytes).unwrap();
        let text = doc
            .extract_text(&doc.get_pages().keys().copied().collect::<Vec<_>>())
            .unwrap();
        for expected in [
            "Operating cash flow: USD 799.99",
            "Estimated net worth at balance date: USD 4000.06",
            "José’s fictional café — Ελληνικά Кириллица",
            "FINAL NOTE",
            "INTERNAL / INCOMPLETE",
            "Page 1 of",
            "Captured transaction evidence",
            &saved.digest,
        ] {
            assert!(text.contains(expected), "Missing {expected}");
        }
        assert!(!text.contains("NEEDS REVIEW:"));
        assert!(!text.contains("This live view is recalculated"));
        assert!(text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("Later decisions do not rewrite them"));
        assert!(doc
            .objects
            .values()
            .any(|v| v.as_dict().ok().is_some_and(|d| d.has(b"FontFile2"))));
        assert!(!doc.objects.values().any(|v| v
            .as_dict()
            .ok()
            .is_some_and(|d| d.has(b"JavaScript") || d.has(b"OpenAction"))));
        let rent = &s.records[1].id;
        let changed = review(
            &mut w,
            &c.id,
            rent,
            Status::Rejected,
            "Withdraw source rent",
            true,
        )
        .unwrap();
        let oldpath = out.path().join("historical.pdf");
        w.export_assessment_pdf(&c.id, id, oldpath.to_str().unwrap())
            .unwrap();
        let doc = lopdf::Document::load(&oldpath).unwrap();
        let text = doc
            .extract_text(&doc.get_pages().keys().copied().collect::<Vec<_>>())
            .unwrap();
        assert!(text.contains("NEEDS REVIEW:"));
        assert!(text.contains("Operating cash flow: USD 799.99"));
        assert!(!text.contains("Operating cash flow: USD 2000.01"));
        assert_eq!(
            w.get_assessment_record(&c.id, id).unwrap().digest,
            saved.digest
        );
        assert_eq!(
            w.get_client_imports(&c.id).unwrap().revision,
            changed.revision
        );
        assert_eq!(w.list_assessment_records(&c.id).unwrap().records.len(), 1);
    }
    #[test]
    fn assessment_pdf_rejects_overwrites_wrong_scope_locked_session_and_workspace_paths() {
        let (_root, mut w, _) = configured();
        let out = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, out.path());
        let saved = w
            .save_assessment_record(&c.id, assessment_input(&s, &c))
            .unwrap();
        let id = &saved.payload.id;
        let path = out.path().join("private.pdf");
        fs::write(&path, b"Existing file").unwrap();
        assert!(w
            .export_assessment_pdf(&c.id, id, path.to_str().unwrap())
            .unwrap_err()
            .contains("already exists"));
        assert_eq!(fs::read(&path).unwrap(), b"Existing file");
        assert!(w
            .export_assessment_pdf(&c.id, id, w.data_dir.join("report.pdf").to_str().unwrap())
            .is_err());
        assert!(w.export_assessment_pdf(&c.id, id, "relative.pdf").is_err());
        assert!(w
            .export_assessment_pdf(&c.id, id, out.path().join("bad.txt").to_str().unwrap())
            .is_err());
        let nonexistent = out.path().join("missing").join("report.pdf");
        assert!(w
            .export_assessment_pdf(&c.id, id, nonexistent.to_str().unwrap())
            .is_err());
        let other = w
            .save_client(ClientInput {
                first_name: "Robin".into(),
                ..client_input()
            })
            .unwrap();
        let fresh = out.path().join("wrong.pdf");
        assert!(w
            .export_assessment_pdf(&other.id, id, fresh.to_str().unwrap())
            .is_err());
        assert!(!fresh.exists());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&path, &fresh).unwrap();
            assert!(w
                .export_assessment_pdf(&c.id, id, fresh.to_str().unwrap())
                .is_err());
            assert_eq!(fs::read(&path).unwrap(), b"Existing file");
            fs::remove_file(&fresh).unwrap();
        }
        w.lock_workspace();
        assert!(w
            .export_assessment_pdf(&c.id, id, fresh.to_str().unwrap())
            .unwrap_err()
            .contains("locked"));
        assert!(!fresh.exists());
    }
    #[test]
    fn assessment_pdf_fails_unsupported_characters_without_losing_record_or_leaving_partial_file() {
        let (_root, mut w, _) = configured();
        let out = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, out.path());
        let mut input = assessment_input(&s, &c);
        input.findings = "Fictional unsupported character: 🏦".into();
        let saved = w.save_assessment_record(&c.id, input).unwrap();
        let path = out.path().join("unsupported.pdf");
        assert!(w
            .export_assessment_pdf(&c.id, &saved.payload.id, path.to_str().unwrap())
            .unwrap_err()
            .contains("does not support"));
        assert!(!path.exists());
        assert_eq!(
            w.get_assessment_record(&c.id, &saved.payload.id)
                .unwrap()
                .digest,
            saved.digest
        );
        assert!(!fs::read_dir(out.path()).unwrap().any(|f| f
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".tmp")));
    }
    #[test]
    fn assessment_records_keep_original_approved_figures_after_reversal_and_profile_changes() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        let data = facts_data(&s);
        let s = w
            .review_financial_facts(&c.id, facts_input(&s, data))
            .unwrap();
        let saved = w
            .save_assessment_record(&c.id, assessment_input(&s, &c))
            .unwrap();
        let id = saved.payload.id.clone();
        let digest = saved.digest.clone();
        assert_eq!(saved.payload.assessment.operating_cashflow, "799.99");
        assert_eq!(
            saved.payload.assessment.facts_metrics.net_worth.as_deref(),
            Some("4000.06")
        );
        assert!(!saved.needs_review);
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, s.revision);
        let rent = s.records[1].id.clone();
        let rejected = review(
            &mut w,
            &c.id,
            &rent,
            Status::Rejected,
            "Withdraw source rent",
            true,
        )
        .unwrap();
        let historical = w.get_assessment_record(&c.id, &id).unwrap();
        assert!(historical.needs_review);
        assert_eq!(historical.digest, digest);
        assert_eq!(historical.payload.assessment.operating_cashflow, "799.99");
        let new = w
            .save_assessment_record(&c.id, assessment_input(&rejected, &c))
            .unwrap();
        assert_eq!(new.payload.assessment.operating_cashflow, "2000.01");
        assert_eq!(new.payload.assessment.totals.rejected, 1);
        assert!(!new.needs_review);
        assert_eq!(
            new.payload.assessment.facts_metrics.necessary_spending,
            "0.00"
        );
        assert!(
            w.list_assessment_records(&c.id)
                .unwrap()
                .records
                .iter()
                .find(|r| r.id == id)
                .unwrap()
                .needs_review
        );
        let mut changed = client_input();
        changed.id = Some(c.id.clone());
        changed.first_name = "Renamed fictional client".into();
        w.save_client(changed).unwrap();
        let older = w.get_assessment_record(&c.id, &new.payload.id).unwrap();
        assert!(older.needs_review);
        assert_eq!(older.payload.client.first_name, "Synthetic");
        let conn = &w.session.as_ref().unwrap().connection;
        assert!(conn
            .execute("UPDATE client_assessment_records SET title='changed'", [])
            .is_err());
        assert!(conn
            .execute("DELETE FROM client_assessment_records", [])
            .is_err());
    }
    #[test]
    fn assessment_records_scope_stale_acknowledgment_audit_and_payload_validation_are_safe() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        let mut input = assessment_input(&s, &c);
        input.acknowledge = false;
        assert!(w.save_assessment_record(&c.id, input).is_err());
        let mut input = assessment_input(&s, &c);
        input.expected_revision -= 1;
        assert!(w.save_assessment_record(&c.id, input).is_err());
        let mut input = assessment_input(&s, &c);
        input.expected_profile_updated_at = "old".into();
        assert!(w.save_assessment_record(&c.id, input).is_err());
        let conn = &w.session.as_ref().unwrap().connection;
        conn.execute_batch("CREATE TRIGGER reject_assessment_audit BEFORE INSERT ON audit_events WHEN NEW.event='assessment_recorded' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
        assert!(w
            .save_assessment_record(&c.id, assessment_input(&s, &c))
            .is_err());
        assert!(w.list_assessment_records(&c.id).unwrap().records.is_empty());
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, s.revision);
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_assessment_audit;")
            .unwrap();
        let mut saved = w
            .save_assessment_record(&c.id, assessment_input(&s, &c))
            .unwrap();
        assert!(w
            .get_assessment_record(&other.id, &saved.payload.id)
            .is_err());
        assert!(w
            .list_assessment_records(&other.id)
            .unwrap()
            .records
            .is_empty());
        saved.payload.assessment.totals.income = "999999.99".into();
        assert!(crate::assessments::validate(&saved.payload).is_err());
        saved.payload.assessment.period_start = "x".into();
        assert!(crate::assessments::validate(&saved.payload).is_err());
        w.lock_workspace();
        assert!(w.get_assessment_record(&c.id, &saved.payload.id).is_err());
        assert!(w
            .export_assessment_pdf(
                &c.id,
                &saved.payload.id,
                dir.path().join("corrupt.pdf").to_str().unwrap()
            )
            .is_err());
        assert!(!dir.path().join("corrupt.pdf").exists());
    }
    #[test]
    fn assessment_records_schema_thirteen_rollback_restart_and_backup_recovery() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        let data = facts_data(&s);
        let s = w
            .review_financial_facts(&c.id, facts_input(&s, data))
            .unwrap();
        let conn = &mut w.session.as_mut().unwrap().connection;
        conn.execute_batch("DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE client_action_plans; DROP TABLE client_assessment_records; PRAGMA user_version=13; CREATE TABLE audit_events_v14(collision TEXT);").unwrap();
        assert!(migrate(conn, false).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            13
        );
        assert!(conn
            .prepare("SELECT * FROM client_assessment_records")
            .is_err());
        conn.execute_batch("DROP TABLE audit_events_v14;").unwrap();
        migrate(conn, false).unwrap();
        let saved = w
            .save_assessment_record(&c.id, assessment_input(&s, &c))
            .unwrap();
        let id = saved.payload.id.clone();
        let digest = saved.digest.clone();
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        drop(w);
        let mut w = Workspace::new(root.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(w.get_assessment_record(&c.id, &id).unwrap().digest, digest);
        let s = w.get_client_imports(&c.id).unwrap();
        review(
            &mut w,
            &c.id,
            &s.records[1].id,
            Status::Rejected,
            "Change after backup",
            true,
        )
        .unwrap();
        assert!(w.get_assessment_record(&c.id, &id).unwrap().needs_review);
        w.lock_workspace();
        let r = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(r.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_assessment_record(&c.id, &id).unwrap();
        assert_eq!(restored.digest, digest);
        assert!(!restored.needs_review);
        assert_eq!(restored.payload.basis.financial_facts.history.len(), 1);
    }
    #[test]
    fn assessment_records_corrupt_evidence_is_rejected_before_backup_activation() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        let saved = w
            .save_assessment_record(&c.id, assessment_input(&s, &c))
            .unwrap();
        let conn = &w.session.as_ref().unwrap().connection;
        conn.execute_batch("DROP TRIGGER assessment_records_no_update;")
            .unwrap();
        conn.execute(
            "UPDATE client_assessment_records SET payload_json='{}' WHERE id=?1",
            [&saved.payload.id],
        )
        .unwrap();
        assert!(w.get_assessment_record(&c.id, &saved.payload.id).is_err());
        assert!(w
            .export_assessment_pdf(
                &c.id,
                &saved.payload.id,
                dir.path().join("corrupt.pdf").to_str().unwrap()
            )
            .is_err());
        assert!(!dir.path().join("corrupt.pdf").exists());
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            w.get_client_imports(&c.id).unwrap().summary.income,
            "2000.01"
        );
    }
    fn facts_input(
        s: &Snapshot,
        data: crate::financial_facts::Data,
    ) -> crate::financial_facts::Input {
        crate::financial_facts::Input {
            expected_revision: s.revision,
            data,
            reason: "Verified common date, balances and transaction purposes".into(),
            acknowledge: true,
        }
    }
    fn assessed(w: &mut Workspace, client: &str) -> crate::dashboard::Assessment {
        w.get_client_dashboard(
            client,
            crate::dashboard::Input {
                from_month: "2026-09".into(),
                through_month: "2026-09".into(),
            },
        )
        .unwrap()
    }
    #[test]
    fn financial_facts_exact_metrics_reversals_require_new_approval_and_unknowns_block_ratios() {
        use crate::financial_facts::Role;
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &client.id, dir.path());
        let data = facts_data(&s);
        let s = w
            .review_financial_facts(&client.id, facts_input(&s, data))
            .unwrap();
        let a = assessed(&mut w, &client.id);
        let m = a.facts_metrics;
        assert_eq!(m.necessary_spending, "1200.02");
        assert_eq!(m.take_home_pay, "2000.01");
        assert_eq!(m.net_worth.as_deref(), Some("4000.06"));
        assert_eq!(m.emergency_months_estimate.as_deref(), Some("1.99"));
        assert_eq!(
            m.take_home_payment_percent_estimate.as_deref(),
            Some("5.00")
        );
        assert_eq!(m.debt_balances[0].scale, 1000);
        assert_eq!(m.stale_classifications, 0);
        let rent = s.records[1].id.clone();
        review(
            &mut w,
            &client.id,
            &rent,
            Status::Rejected,
            "Withdraw rent approval",
            true,
        )
        .unwrap();
        assert_eq!(
            assessed(&mut w, &client.id)
                .facts_metrics
                .stale_classifications,
            1
        );
        let s = review(
            &mut w,
            &client.id,
            &rent,
            Status::Approved,
            "Reverify original rent after withdrawal",
            true,
        )
        .unwrap();
        let m = assessed(&mut w, &client.id).facts_metrics;
        assert_eq!(m.unclassified_spending, "1200.02");
        assert!(m.emergency_months_estimate.is_none());
        let mut data = s.financial_facts.data.clone().unwrap();
        data.classifications[1].role = Role::Discretionary;
        assert!(w
            .review_financial_facts(&client.id, facts_input(&s, data))
            .is_err());
        assert_eq!(
            w.get_client_imports(&client.id).unwrap().revision,
            s.revision
        );
        let mut data = s.financial_facts.data.clone().unwrap();
        data.classifications[1].approval_id = s.history.last().unwrap().id.clone();
        let s = w
            .review_financial_facts(&client.id, facts_input(&s, data))
            .unwrap();
        assert_eq!(
            assessed(&mut w, &client.id)
                .facts_metrics
                .necessary_spending,
            "1200.02"
        );
        let mut data = s.financial_facts.data.clone().unwrap();
        data.balances[1].amount = None;
        let s = w
            .review_financial_facts(&client.id, facts_input(&s, data))
            .unwrap();
        let m = assessed(&mut w, &client.id).facts_metrics;
        assert!(m.net_worth.is_none());
        assert!(m.accessible_savings.is_none());
        assert!(m.emergency_months_estimate.is_none());
        let mut data = s.financial_facts.data.clone().unwrap();
        data.balances[1].amount = Some("2400.03".into());
        data.as_of = Some("2026-09-29".into());
        w.review_financial_facts(&client.id, facts_input(&s, data))
            .unwrap();
        let m = assessed(&mut w, &client.id).facts_metrics;
        assert!(!m.balance_aligned);
        assert!(m.net_worth.is_some());
        assert!(m.emergency_months_estimate.is_none());
        assert!(m.take_home_payment_percent_estimate.is_none());
    }
    #[test]
    fn financial_facts_scope_stale_revisions_audit_rollback_and_append_only_history() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &client.id, dir.path());
        let data = facts_data(&s);
        let other_snapshot = w.get_client_imports(&other.id).unwrap();
        assert!(w
            .review_financial_facts(&other.id, facts_input(&other_snapshot, data.clone()))
            .is_err());
        let mut i = facts_input(&s, data.clone());
        i.acknowledge = false;
        assert!(w.review_financial_facts(&client.id, i).is_err());
        let mut i = facts_input(&s, data.clone());
        i.expected_revision -= 1;
        assert!(w.review_financial_facts(&client.id, i).is_err());
        w.session.as_mut().unwrap().connection.execute_batch("CREATE TRIGGER reject_facts_audit BEFORE INSERT ON audit_events WHEN NEW.event='financial_facts_reviewed' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
        assert!(w
            .review_financial_facts(&client.id, facts_input(&s, data.clone()))
            .is_err());
        assert_eq!(
            w.get_client_imports(&client.id).unwrap().revision,
            s.revision
        );
        assert!(w
            .get_client_imports(&client.id)
            .unwrap()
            .financial_facts
            .history
            .is_empty());
        w.session
            .as_mut()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_facts_audit;")
            .unwrap();
        let saved = w
            .review_financial_facts(&client.id, facts_input(&s, data))
            .unwrap();
        assert_eq!(saved.financial_facts.history.len(), 1);
        let c = &w.session.as_ref().unwrap().connection;
        assert!(c
            .execute("UPDATE financial_fact_reviews SET reason='changed'", [])
            .is_err());
        assert!(c.execute("DELETE FROM financial_fact_reviews", []).is_err());
        let i = facts_input(&saved, crate::financial_facts::Data::default());
        w.review_financial_facts(&client.id, i).unwrap();
        let m = assessed(&mut w, &client.id).facts_metrics;
        assert!(m.cash.is_none());
        assert!(m.net_worth.is_none());
        assert_eq!(m.unclassified_spending, "1200.02");
        assert_eq!(
            w.get_client_imports(&client.id)
                .unwrap()
                .financial_facts
                .history
                .len(),
            2
        );
    }
    #[test]
    fn financial_facts_restart_backup_and_schema_twelve_migration_rollback_preserve_records() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &client.id, dir.path());
        let before = s.records[0].raw.clone();
        let c = &mut w.session.as_mut().unwrap().connection;
        c.execute_batch("DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE IF EXISTS client_action_plans; DROP TABLE IF EXISTS client_assessment_records; DROP TABLE financial_fact_reviews; PRAGMA user_version=12; CREATE TABLE audit_events_v13(collision TEXT);").unwrap();
        assert!(migrate(c, false).is_err());
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            12
        );
        assert!(c.prepare("SELECT * FROM financial_fact_reviews").is_err());
        c.execute_batch("DROP TABLE audit_events_v13;").unwrap();
        migrate(c, false).unwrap();
        let s = w.get_client_imports(&client.id).unwrap();
        assert_eq!(s.records[0].raw, before);
        assert!(s.financial_facts.history.is_empty());
        let data = facts_data(&s);
        w.review_financial_facts(&client.id, facts_input(&s, data))
            .unwrap();
        let backup = w
            .create_backup(dir.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        drop(w);
        let mut w = Workspace::new(root.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            assessed(&mut w, &client.id)
                .facts_metrics
                .net_worth
                .as_deref(),
            Some("4000.06")
        );
        w.lock_workspace();
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            w.get_client_imports(&client.id)
                .unwrap()
                .financial_facts
                .history
                .len(),
            1
        );
        assert_eq!(
            assessed(&mut w, &client.id)
                .facts_metrics
                .emergency_months_estimate
                .as_deref(),
            Some("1.99")
        );
    }
    const RUNNING:&str="Date,Description,Amount,Category,Type,Balance\n2026-09-01,Fictional wages,500.01,Wages,income,1500.01\n2026-09-02,Fictional rent,-300.02,Housing,expense,1199.99\n2026-09-03,Fictional fee,-0.01,Fees,fee,1199.98\n";
    fn balance_input(
        s: &Snapshot,
        order: Option<crate::reconciliation::Order>,
    ) -> crate::reconciliation::ReviewInput {
        crate::reconciliation::ReviewInput {
            expected_revision: s.revision,
            document_id: s.documents[0].id.clone(),
            order,
            reason: "Verified original after-transaction balance column and source sequence".into(),
        }
    }
    fn balance_fixture(w: &mut Workspace, client: &str, dir: &Path) -> Snapshot {
        let (d, _) = crate::reconciliation::tests::fixture();
        stage(w, client, dir, "running.csv", RUNNING, d)
    }
    #[test]
    fn running_balance_scope_settings_corrections_reversals_restart_and_recovery() {
        use crate::reconciliation::{Order, State};
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let s = balance_fixture(&mut w, &c.id, dir.path());
        assert_eq!(s.summary.approved, 0);
        assert!(s.records.iter().all(|r| !r.bulk_eligible));
        assert!(w
            .bulk_approve_transactions(
                &c.id,
                BulkInput {
                    expected_revision: s.revision,
                    record_ids: s.records.iter().map(|r| r.id.clone()).collect()
                }
            )
            .is_err());
        let mut cross = balance_input(&s, Some(Order::OldestFirst));
        cross.expected_revision = 0;
        assert!(w.review_statement_balances(&other.id, cross).is_err());
        let selected = w
            .review_statement_balances(&c.id, balance_input(&s, Some(Order::OldestFirst)))
            .unwrap();
        assert_eq!(
            selected.documents[0]
                .balance_reconciliation
                .as_ref()
                .unwrap()
                .source_matched,
            3
        );
        assert!(selected
            .records
            .iter()
            .all(|r| r.status == Status::NeedsClarification && !r.bulk_eligible));
        assert!(selected.documents[0].input.balance_order.is_none());
        for id in selected
            .records
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
        {
            review(
                &mut w,
                &c.id,
                &id,
                Status::Approved,
                "Individually verified the clarified source row",
                true,
            )
            .unwrap();
        }
        let approved = w.get_client_imports(&c.id).unwrap();
        assert_eq!(
            approved.documents[0]
                .balance_reconciliation
                .as_ref()
                .unwrap()
                .approved_matched,
            3
        );
        assert_eq!(approved.summary.net_cash, "199.98");
        let mut f = approved.records[1].fields.clone();
        f.amount = Some("-300.01".into());
        let corrected = w
            .review_transaction(
                &c.id,
                ReviewInput {
                    expected_revision: approved.revision,
                    record_id: approved.records[1].id.clone(),
                    fields: f,
                    status: Status::Approved,
                    reason: "Verified fictional corrected payment amount".into(),
                    acknowledge_warnings: true,
                },
            )
            .unwrap();
        let check = corrected.documents[0]
            .balance_reconciliation
            .as_ref()
            .unwrap();
        assert_eq!((check.source_matched, check.approved_differences), (3, 2));
        assert_eq!(check.rows[2].approved.difference.as_deref(), Some("0.01"));
        assert_eq!(corrected.summary.net_cash, "199.99");
        assert_eq!(corrected.records[1].raw.balance.as_deref(), Some("1199.99"));
        assert_eq!(
            corrected.records[1].raw.extracted.amount.as_deref(),
            Some("-300.02")
        );
        let reversed = review(
            &mut w,
            &c.id,
            &corrected.records[1].id,
            Status::Rejected,
            "Withdraw source payment from approved movement",
            true,
        )
        .unwrap();
        let check = reversed.documents[0]
            .balance_reconciliation
            .as_ref()
            .unwrap();
        assert_eq!(check.rows[1].approved.state, State::Excluded);
        assert_eq!(check.rows[2].approved.difference.as_deref(), Some("300.02"));
        assert_eq!(reversed.summary.net_cash, "500.00");
        assert!(w
            .get_client_imports(&other.id)
            .unwrap()
            .documents
            .is_empty());
        w.lock_workspace();
        assert!(w
            .review_statement_balances(&c.id, balance_input(&reversed, None))
            .is_err());
        drop(w);
        let mut w = Workspace::new(root.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let reopened = w.get_client_imports(&c.id).unwrap();
        assert_eq!(
            reopened.documents[0].balance_history,
            reversed.documents[0].balance_history
        );
        assert_eq!(
            reopened.documents[0].balance_reconciliation,
            reversed.documents[0].balance_reconciliation
        );
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let cleared = w
            .review_statement_balances(&c.id, balance_input(&reopened, None))
            .unwrap();
        assert!(cleared.documents[0]
            .balance_reconciliation
            .as_ref()
            .unwrap()
            .order
            .is_none());
        assert_eq!(cleared.summary.net_cash, "500.00");
        assert_eq!(cleared.documents[0].balance_history.len(), 2);
        w.lock_workspace();
        let candidate = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(candidate.token, "RESTORE".into())
            .unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_client_imports(&c.id).unwrap();
        assert_eq!(
            restored.documents[0].balance_history,
            reversed.documents[0].balance_history
        );
        assert_eq!(
            restored.documents[0].balance_reconciliation,
            reversed.documents[0].balance_reconciliation
        );
        assert_eq!(restored.summary.net_cash, "500.00");
    }
    #[test]
    fn balance_setting_failure_stale_revision_and_bad_source_are_atomic() {
        use crate::reconciliation::Order;
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = balance_fixture(&mut w, &c.id, dir.path());
        let mut empty = balance_input(&s, Some(Order::OldestFirst));
        empty.reason = "".into();
        assert!(w.review_statement_balances(&c.id, empty).is_err());
        w.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER reject_balance_audit BEFORE INSERT ON audit_events WHEN NEW.event='statement_balance_reviewed' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
        assert!(w
            .review_statement_balances(&c.id, balance_input(&s, Some(Order::OldestFirst)))
            .is_err());
        let after = w.get_client_imports(&c.id).unwrap();
        assert_eq!(after.revision, s.revision);
        assert!(after.documents[0].balance_history.is_empty());
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_balance_audit;")
            .unwrap();
        let mut unicode = balance_input(&s, Some(Order::OldestFirst));
        unicode.reason = "é".repeat(500);
        let saved = w.review_statement_balances(&c.id, unicode).unwrap();
        assert_eq!(
            saved.documents[0].balance_history[0].reason.chars().count(),
            500
        );
        assert!(w
            .review_statement_balances(&c.id, balance_input(&s, None))
            .is_err());
        let connection = &w.session.as_ref().unwrap().connection;
        assert!(connection
            .execute("UPDATE statement_balance_reviews SET row_order=NULL", [])
            .is_err());
        assert!(connection
            .execute("DELETE FROM statement_balance_reviews", [])
            .is_err());
        let wrong = w
            .review_statement_balances(&c.id, balance_input(&saved, Some(Order::NewestFirst)))
            .unwrap();
        assert_eq!(
            wrong.documents[0]
                .balance_reconciliation
                .as_ref()
                .unwrap()
                .source_sequence,
            crate::reconciliation::Sequence::Conflict
        );
        assert!(wrong.records.iter().all(|r| !r.bulk_eligible));
        assert_eq!(wrong.summary.approved, 0);
        let mut no_column = doc("No balance column");
        no_column.balance_order = Some(Order::OldestFirst);
        let path = dir.path().join("no-column.csv");
        fs::write(&path, CHECKING).unwrap();
        let preview = w
            .inspect_client_csv(c.id.clone(), path.to_string_lossy().into())
            .unwrap();
        assert!(w
            .stage_client_csv(
                &c.id,
                StageInput {
                    expected_revision: wrong.revision,
                    token: preview.token,
                    document: no_column
                }
            )
            .is_err());
        let (mut valid_doc, _) = crate::reconciliation::tests::fixture();
        valid_doc.balance_order = Some(Order::OldestFirst);
        valid_doc.account_label = "Validated balance checking".into();
        let valid = stage(
            &mut w,
            &c.id,
            dir.path(),
            "validated.csv",
            RUNNING,
            valid_doc,
        );
        let valid_id = valid.documents.last().unwrap().id.clone();
        let valid_rows: Vec<_> = valid
            .records
            .iter()
            .filter(|r| r.document_id == valid_id)
            .collect();
        assert!(valid_rows.iter().all(|r| r.bulk_eligible));
        let validated = w
            .bulk_approve_transactions(
                &c.id,
                BulkInput {
                    expected_revision: valid.revision,
                    record_ids: valid_rows.iter().map(|r| r.id.clone()).collect(),
                },
            )
            .unwrap();
        assert_eq!(
            validated
                .documents
                .last()
                .unwrap()
                .balance_reconciliation
                .as_ref()
                .unwrap()
                .approved_matched,
            3
        );
        let mismatched = RUNNING.replace("1199.99\n", "1200.00\n");
        let (mut d, _) = crate::reconciliation::tests::fixture();
        d.balance_order = Some(Order::OldestFirst);
        d.account_label = "Fictional discrepancy checking".into();
        let discrepancy = stage(&mut w, &c.id, dir.path(), "discrepancy.csv", &mismatched, d);
        let check = discrepancy
            .documents
            .last()
            .unwrap()
            .balance_reconciliation
            .as_ref()
            .unwrap();
        assert_eq!(check.source_differences, 2);
        assert_eq!(
            discrepancy
                .documents
                .last()
                .unwrap()
                .source_difference
                .as_deref(),
            Some("0.00")
        );
        assert!(discrepancy
            .records
            .iter()
            .filter(|r| r.document_id == discrepancy.documents.last().unwrap().id)
            .all(|r| !r.bulk_eligible));
    }
    #[test]
    fn balance_schema_eleven_migration_failure_preserves_legacy_records() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let before = stage(
            &mut w,
            &c.id,
            dir.path(),
            "legacy.csv",
            CHECKING,
            doc("Legacy checking"),
        );
        let connection = &mut w.session.as_mut().unwrap().connection;
        connection.execute_batch("DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE IF EXISTS client_action_plans; DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE statement_balance_reviews; PRAGMA user_version=11; CREATE TABLE audit_events_v12(collision TEXT);").unwrap();
        assert!(migrate(connection, false).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            11
        );
        assert!(connection
            .prepare("SELECT * FROM statement_balance_reviews")
            .is_err());
        connection
            .execute_batch("DROP TABLE audit_events_v12;")
            .unwrap();
        migrate(connection, false).unwrap();
        let after = w.get_client_imports(&c.id).unwrap();
        assert_eq!(after.revision, before.revision);
        assert_eq!(after.records[0].raw, before.records[0].raw);
        assert!(after.documents[0].balance_history.is_empty());
        assert_eq!(
            after.documents[0]
                .balance_reconciliation
                .as_ref()
                .unwrap()
                .source_sequence,
            crate::reconciliation::Sequence::NotConfigured
        );
    }
    fn recurring_fixture(w: &mut Workspace, client: &str, dir: &Path) -> Snapshot {
        let mut d = doc("Fictional service checking");
        d.period_start = None;
        d.period_end = None;
        d.opening_balance = None;
        d.closing_balance = None;
        let s=stage(w,client,dir,"service.csv","Date,Description,Amount,Category,Type\n2026-01-31,Fictional service,-10.01,Services,expense\n2026-02-28,Fictional service,-12.03,Services,expense\n2026-03-31,Fictional service,-10.01,Services,expense\n",d);
        assert!(s.merchant_groups.is_empty());
        w.bulk_approve_transactions(
            client,
            BulkInput {
                expected_revision: s.revision,
                record_ids: s.records.iter().map(|r| r.id.clone()).collect(),
            },
        )
        .unwrap()
    }
    fn recurring_input(s: &Snapshot) -> crate::recurring::ReviewInput {
        let g = &s.merchant_groups[0];
        crate::recurring::ReviewInput {
            expected_revision: s.revision,
            group_id: g.id.clone(),
            evidence_hash: g.evidence_hash.clone(),
            label: "Fictional service contract".into(),
            decision: crate::recurring::Decision::Confirmed,
            frequency: Some(crate::recurring::Frequency::Monthly),
            subscription: true,
            reason: "Verified three original statements and fictional subscription contract".into(),
        }
    }
    #[test]
    fn recurring_decisions_are_scoped_atomic_persistent_and_backed_up() {
        let (root, mut w, _) = configured();
        let target = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let s = recurring_fixture(&mut w, &c.id, target.path());
        assert_eq!(s.merchant_groups[0].total_paid, "32.05");
        assert_eq!(
            s.merchant_groups[0].suggested_frequency,
            Some(crate::recurring::Frequency::Monthly)
        );
        let mut other_input = recurring_input(&s);
        other_input.expected_revision = 0;
        assert!(w.review_recurring_payment(&other.id, other_input).is_err());
        let mut bad = recurring_input(&s);
        bad.reason = "".into();
        assert!(w.review_recurring_payment(&c.id, bad).is_err());
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, s.revision);
        let connection = &w.session.as_ref().unwrap().connection;
        connection.execute_batch("CREATE TRIGGER reject_payment_audit BEFORE INSERT ON audit_events WHEN NEW.event='recurring_reviewed' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
        assert!(w
            .review_recurring_payment(&c.id, recurring_input(&s))
            .is_err());
        let untouched = w.get_client_imports(&c.id).unwrap();
        assert_eq!(untouched.revision, s.revision);
        assert!(untouched.recurring_history.is_empty());
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_payment_audit;")
            .unwrap();
        let mut unicode = recurring_input(&s);
        unicode.label = "ü".repeat(120);
        unicode.reason = "é".repeat(500);
        let saved = w.review_recurring_payment(&c.id, unicode).unwrap();
        assert_eq!(saved.recurring_history[0].label.chars().count(), 120);
        assert_eq!(saved.recurring_history[0].reason.chars().count(), 500);
        let mut too_long = recurring_input(&saved);
        too_long.label = "ü".repeat(121);
        assert!(w.review_recurring_payment(&c.id, too_long).is_err());
        let mut too_long = recurring_input(&saved);
        too_long.reason = "é".repeat(501);
        assert!(w.review_recurring_payment(&c.id, too_long).is_err());
        assert_eq!(saved.merchant_groups[0].state, "Confirmed");
        assert_eq!(saved.summary.spending, s.summary.spending);
        assert_eq!(saved.summary.net_cash, s.summary.net_cash);
        assert!(w
            .get_client_imports(&other.id)
            .unwrap()
            .recurring_history
            .is_empty());
        assert!(w
            .review_recurring_payment(&c.id, recurring_input(&s))
            .is_err());
        let connection = &w.session.as_ref().unwrap().connection;
        assert!(connection
            .execute("UPDATE recurring_review_history SET review_json='{}'", [])
            .is_err());
        assert!(connection
            .execute("DELETE FROM recurring_review_history", [])
            .is_err());
        w.lock_workspace();
        assert!(w
            .review_recurring_payment(&c.id, recurring_input(&saved))
            .is_err());
        drop(w);
        let mut w = Workspace::new(root.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            w.get_client_imports(&c.id).unwrap().recurring_history,
            saved.recurring_history
        );
        let backup = w
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let mut input = recurring_input(&saved);
        input.decision = crate::recurring::Decision::Dismissed;
        input.frequency = None;
        input.subscription = false;
        input.reason = "Not an ongoing contract after counselor review".into();
        assert_eq!(
            w.review_recurring_payment(&c.id, input)
                .unwrap()
                .merchant_groups[0]
                .state,
            "Dismissed"
        );
        w.lock_workspace();
        let candidate = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(candidate.token, "RESTORE".into())
            .unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_client_imports(&c.id).unwrap();
        assert_eq!(restored.recurring_history, saved.recurring_history);
        assert_eq!(restored.merchant_groups[0].state, "Confirmed");
    }
    #[test]
    fn recurring_reversals_duplicates_corrections_and_stale_evidence_require_review() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = recurring_fixture(&mut w, &c.id, dir.path());
        let saved = w
            .review_recurring_payment(&c.id, recurring_input(&s))
            .unwrap();
        let first = saved.records[0].id.clone();
        let reversed = review(
            &mut w,
            &c.id,
            &first,
            Status::Rejected,
            "Withdraw inaccurate source approval",
            true,
        )
        .unwrap();
        assert_eq!(reversed.merchant_groups[0].state, "Needs review");
        assert!(!reversed.merchant_groups[0].can_confirm);
        assert!(w
            .review_recurring_payment(&c.id, recurring_input(&reversed))
            .is_err());
        let again = review(
            &mut w,
            &c.id,
            &first,
            Status::Approved,
            "Verified source again",
            true,
        )
        .unwrap();
        assert_eq!(again.merchant_groups[0].state, "Needs review");
        let mut stale = recurring_input(&again);
        stale.evidence_hash = saved.merchant_groups[0].evidence_hash.clone();
        assert!(w.review_recurring_payment(&c.id, stale).is_err());
        let confirmed = w
            .review_recurring_payment(&c.id, recurring_input(&again))
            .unwrap();
        assert_eq!(confirmed.recurring_history.len(), 2);
        let mut additional_doc = doc("");
        additional_doc.account_id = Some(confirmed.accounts[0].id.clone());
        additional_doc.period_start = None;
        additional_doc.period_end = None;
        additional_doc.opening_balance = None;
        additional_doc.closing_balance = None;
        let added=stage(&mut w,&c.id,dir.path(),"later.csv","Date,Description,Amount,Category,Type\n2026-04-30,Fictional service,-10.01,Services,expense\n",additional_doc);
        let additional_id = added.records.last().unwrap().id.clone();
        w.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER fail_invalidation BEFORE INSERT ON recurring_review_invalidations BEGIN SELECT RAISE(ABORT,'Synthetic invalidation write failure'); END;").unwrap();
        assert!(review(
            &mut w,
            &c.id,
            &additional_id,
            Status::Approved,
            "New charge cannot commit without invalidation",
            true
        )
        .is_err());
        let unchanged = w.get_client_imports(&c.id).unwrap();
        assert_eq!(unchanged.revision, added.revision);
        assert_eq!(unchanged.summary.spending, added.summary.spending);
        assert_eq!(unchanged.records.last().unwrap().status, Status::Pending);
        assert_eq!(unchanged.merchant_groups[0].state, "Confirmed");
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER fail_invalidation;")
            .unwrap();
        let with_extra = review(
            &mut w,
            &c.id,
            &additional_id,
            Status::Approved,
            "Verified later charge",
            true,
        )
        .unwrap();
        assert_eq!(with_extra.merchant_groups[0].state, "Needs review");
        let back_to_old_facts = review(
            &mut w,
            &c.id,
            &additional_id,
            Status::Rejected,
            "Later charge was unrelated",
            true,
        )
        .unwrap();
        assert_eq!(
            back_to_old_facts.merchant_groups[0].evidence_hash,
            confirmed.merchant_groups[0].evidence_hash
        );
        assert_eq!(back_to_old_facts.merchant_groups[0].state, "Needs review");
        assert!(
            back_to_old_facts
                .recurring_history
                .last()
                .unwrap()
                .invalidated
        );
        let connection = &w.session.as_ref().unwrap().connection;
        assert!(connection
            .execute("DELETE FROM recurring_review_invalidations", [])
            .is_err());
        assert!(connection
            .execute(
                "UPDATE recurring_review_invalidations SET changed_revision=1",
                []
            )
            .is_err());
        let confirmed = w
            .review_recurring_payment(&c.id, recurring_input(&back_to_old_facts))
            .unwrap();
        assert_eq!(confirmed.merchant_groups[0].state, "Confirmed");
        assert!(!confirmed.recurring_history.last().unwrap().invalidated);
        let r = &confirmed.records[0];
        let mut f = r.fields.clone();
        f.merchant = "Another fictional merchant".into();
        let changed = w
            .review_transaction(
                &c.id,
                ReviewInput {
                    expected_revision: confirmed.revision,
                    record_id: r.id.clone(),
                    fields: f,
                    status: Status::Approved,
                    reason: "Verified different merchant identity".into(),
                    acknowledge_warnings: true,
                },
            )
            .unwrap();
        assert_eq!(changed.merchant_groups.len(), 2);
        assert_eq!(
            changed
                .merchant_groups
                .iter()
                .find(|g| g.id == confirmed.merchant_groups[0].id)
                .unwrap()
                .state,
            "Needs review"
        );
        let mut d = doc("");
        d.account_id = Some(changed.accounts[0].id.clone());
        d.period_start = None;
        d.period_end = None;
        d.opening_balance = None;
        d.closing_balance = None;
        let duplicate=stage(&mut w,&c.id,dir.path(),"overlap.csv","Date,Description,Amount,Category,Type\n2026-02-28,Fictional service,-12.03,Services,expense\n",d);
        assert!(duplicate
            .merchant_groups
            .iter()
            .find(|g| g.id == confirmed.merchant_groups[0].id)
            .unwrap()
            .warnings
            .iter()
            .any(|w| w.contains("duplicate")));
        for id in duplicate
            .records
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
        {
            review(
                &mut w,
                &c.id,
                &id,
                Status::Rejected,
                "Remove remaining payment evidence",
                true,
            )
            .unwrap();
        }
        let none = w.get_client_imports(&c.id).unwrap();
        assert!(none.merchant_groups.is_empty());
        assert_eq!(none.recurring_history.len(), 3);
    }
    #[test]
    fn recurring_schema_ten_upgrade_preserves_decisions_and_requires_reconfirmation() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = recurring_fixture(&mut w, &c.id, dir.path());
        let before = w
            .review_recurring_payment(&c.id, recurring_input(&s))
            .unwrap();
        let connection = &mut w.session.as_mut().unwrap().connection;
        connection.execute_batch("DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE IF EXISTS client_action_plans; DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; PRAGMA user_version=10; CREATE TABLE recurring_review_invalidations(collision TEXT);").unwrap();
        assert!(migrate(connection, false).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            10
        );
        connection
            .execute_batch("DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE IF EXISTS client_action_plans; DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations;")
            .unwrap();
        migrate(connection, false).unwrap();
        let after = w.get_client_imports(&c.id).unwrap();
        assert_eq!(after.revision, before.revision);
        assert_eq!(after.summary.spending, before.summary.spending);
        assert_eq!(
            after.recurring_history[0].evidence,
            before.recurring_history[0].evidence
        );
        assert_eq!(
            after.recurring_history[0].id,
            before.recurring_history[0].id
        );
        assert_eq!(after.merchant_groups[0].state, "Needs review");
        assert_eq!(
            w.review_recurring_payment(&c.id, recurring_input(&after))
                .unwrap()
                .merchant_groups[0]
                .state,
            "Confirmed"
        );
    }
    #[test]
    fn recurring_schema_nine_migration_rolls_back_and_preserves_imports() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let before = recurring_fixture(&mut w, &c.id, dir.path());
        let connection = &mut w.session.as_mut().unwrap().connection;
        connection
            .execute_batch(
                "DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE IF EXISTS client_action_plans; DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history;",
            )
            .unwrap();
        connection
            .execute_batch(include_str!("pdf-schema.sql"))
            .unwrap();
        connection
            .execute_batch("CREATE TABLE audit_events_v10(collision TEXT);")
            .unwrap();
        assert!(migrate(connection, false).is_err());
        assert!(connection
            .prepare("SELECT * FROM recurring_review_history")
            .is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            9
        );
        connection
            .execute_batch("DROP TABLE audit_events_v10;")
            .unwrap();
        migrate(connection, false).unwrap();
        let after = w.get_client_imports(&c.id).unwrap();
        assert_eq!(after.revision, before.revision);
        assert_eq!(after.records.len(), before.records.len());
        assert_eq!(after.summary.spending, "32.05");
        assert!(after.recurring_history.is_empty());
    }
    #[test]
    fn independent_approved_totals_refunds_reversals_and_paired_transfers() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let budget = w.save_client_budget(&client.id, budget_input(0)).unwrap();
        let s = stage(
            &mut w,
            &client.id,
            dir.path(),
            "checking.csv",
            CHECKING,
            doc("Checking"),
        );
        assert_eq!(s.summary.income, "0.00");
        assert_eq!(s.summary.spending, "0.00");
        assert_eq!(s.documents[0].source_difference.as_deref(), Some("0.00"));
        assert!(s.documents[0].approved_difference.is_none());
        let ids = s
            .records
            .iter()
            .filter(|r| r.bulk_eligible)
            .map(|r| r.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 3);
        let s = w
            .bulk_approve_transactions(
                &client.id,
                BulkInput {
                    expected_revision: s.revision,
                    record_ids: ids,
                },
            )
            .unwrap();
        assert_eq!(s.summary.income, "2000.00");
        assert_eq!(s.summary.spending, "1210.01");
        assert_eq!(s.summary.net_cash, "789.99");
        let refund = s
            .records
            .iter()
            .find(|r| r.fields.kind == Kind::Refund)
            .unwrap()
            .id
            .clone();
        assert!(review(&mut w, &client.id, &refund, Status::Approved, "", false).is_err());
        let s = review(
            &mut w,
            &client.id,
            &refund,
            Status::Approved,
            "Verified refund, not a second income deposit",
            true,
        )
        .unwrap();
        assert_eq!(s.summary.refunds, "20.01");
        assert_eq!(s.summary.net_spending, "1190.00");
        assert_eq!(s.summary.net_cash, "810.00");
        let fee = s
            .records
            .iter()
            .find(|r| r.fields.kind == Kind::Fee)
            .unwrap()
            .id
            .clone();
        assert!(review(&mut w, &client.id, &fee, Status::Rejected, "", false).is_err());
        let s = review(
            &mut w,
            &client.id,
            &fee,
            Status::Rejected,
            "Fee disputed; exclude it from the approved assessment",
            false,
        )
        .unwrap();
        assert_eq!(s.summary.spending, "1200.00");
        assert_eq!(s.summary.net_spending, "1179.99");
        assert_eq!(s.summary.net_cash, "820.01");
        let debit = s
            .records
            .iter()
            .find(|r| r.fields.kind == Kind::Transfer)
            .unwrap()
            .id
            .clone();
        let mut d = doc("Savings");
        d.account_kind = "savings".into();
        d.opening_balance = Some("300".into());
        d.closing_balance = Some("500".into());
        let s=stage(&mut w,&client.id,dir.path(),"savings.csv","Date,Description,Amount,Category,Type\n2026-09-06,Synthetic reserve arrival,200,Savings,transfer\n",d);
        let credit = s
            .records
            .iter()
            .find(|r| r.fields.description == "Synthetic reserve arrival")
            .unwrap()
            .id
            .clone();
        let s = w
            .pair_transfer_transactions(
                &client.id,
                PairInput {
                    expected_revision: s.revision,
                    left_id: debit.clone(),
                    right_id: credit.clone(),
                    reason: "Verified the two source accounts and equal settlement amounts".into(),
                },
            )
            .unwrap();
        assert_eq!(s.summary.income, "2000.00");
        assert_eq!(s.summary.net_spending, "1179.99");
        assert_eq!(s.summary.net_cash, "820.01");
        assert_eq!(
            s.summary
                .account_net
                .iter()
                .map(|(_, v)| v.as_str())
                .collect::<Vec<_>>(),
            vec!["620.01", "200.00"]
        );
        assert_eq!(s.documents[0].approved_difference.as_deref(), Some("10.01"));
        let s = review(
            &mut w,
            &client.id,
            &debit,
            Status::NeedsClarification,
            "Withdraw transfer approval pending source verification",
            false,
        )
        .unwrap();
        assert_eq!(
            s.records.iter().find(|r| r.id == credit).unwrap().status,
            Status::NeedsClarification
        );
        assert!(s.records.iter().all(|r| r.linked_id.is_none()));
        assert_eq!(s.summary.net_cash, "820.01");
        assert_eq!(
            s.summary
                .account_net
                .iter()
                .map(|(_, v)| v.as_str())
                .collect::<Vec<_>>(),
            vec!["820.01", "0.00"]
        );
        assert!(s.history.iter().any(|h| h.record_id == fee
            && h.from_status == Status::Approved
            && h.to_status == Status::Rejected));
        assert_eq!(w.get_client_budget(&client.id).unwrap(), budget);
    }
    #[test]
    fn duplicate_source_reference_overlap_and_rejected_reapproval_need_review() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let mut d = doc("Checking");
        d.opening_balance = None;
        d.closing_balance = None;
        let content="Date,Description,Amount,Category,Type\n2026-09-01,Same synthetic pay,100,Wages,income\n2026-09-01,Same synthetic pay,100,Wages,income\n";
        let s = stage(
            &mut w,
            &client.id,
            dir.path(),
            "duplicates.csv",
            content,
            d.clone(),
        );
        assert!(s
            .records
            .iter()
            .all(|r| r.status == Status::NeedsClarification
                && !r.bulk_eligible
                && !r.duplicate_ids.is_empty()));
        let first = s.records[0].id.clone();
        let second = s.records[1].id.clone();
        review(
            &mut w,
            &client.id,
            &second,
            Status::Rejected,
            "Duplicate source row",
            false,
        )
        .unwrap();
        let s = review(
            &mut w,
            &client.id,
            &first,
            Status::Approved,
            "Only one deposit verified",
            true,
        )
        .unwrap();
        assert_eq!(s.summary.income, "100.00");
        assert!(review(&mut w, &client.id, &second, Status::Approved, "", false).is_err());
        assert_eq!(
            w.get_client_imports(&client.id).unwrap().summary.income,
            "100.00"
        );
        let s = review(
            &mut w,
            &client.id,
            &second,
            Status::Approved,
            "Two separate deposits verified despite identical dates/amounts",
            true,
        )
        .unwrap();
        assert_eq!(s.summary.income, "200.00");
        let p = w
            .inspect_client_csv(
                client.id.clone(),
                dir.path().join("duplicates.csv").to_string_lossy().into(),
            )
            .unwrap();
        d.account_id = Some(s.accounts[0].id.clone());
        assert!(w
            .stage_client_csv(
                &client.id,
                StageInput {
                    expected_revision: s.revision,
                    token: p.token,
                    document: d.clone()
                }
            )
            .err()
            .unwrap()
            .contains("already imported"));
        let mut d = doc("Reference checking");
        d.opening_balance = None;
        d.closing_balance = None;
        d.mapping.reference = Some(5);
        let s=stage(&mut w,&client.id,dir.path(),"reference.csv","Date,Description,Amount,Category,Type,Reference\n2026-09-01,First source,10,A,income,REF-1\n2026-09-02,Conflicting source,20,B,income,REF-1\n",d);
        let rows = s
            .records
            .iter()
            .filter(|r| r.raw.reference.as_deref() == Some("REF-1"))
            .collect::<Vec<_>>();
        assert!(rows
            .iter()
            .all(|r| !r.duplicate_ids.is_empty() && !r.bulk_eligible));
    }
    #[test]
    fn interrupted_import_audit_failure_source_capture_scope_and_stale_writes_are_atomic() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let other = w
            .save_client(ClientInput {
                first_name: "Other".into(),
                ..client_input()
            })
            .unwrap();
        let path = dir.path().join("source.csv");
        fs::write(&path, CHECKING).unwrap();
        let p = w
            .inspect_client_csv(client.id.clone(), path.to_string_lossy().into())
            .unwrap();
        assert_eq!(w.get_client_imports(&client.id).unwrap().revision, 0);
        assert!(w
            .get_client_imports(&client.id)
            .unwrap()
            .documents
            .is_empty());
        assert!(w
            .stage_client_csv(
                &other.id,
                StageInput {
                    expected_revision: 0,
                    token: p.token.clone(),
                    document: doc("Other")
                }
            )
            .is_err());
        fs::write(&path, "Malformed replacement after inspection").unwrap();
        w.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER fail_import BEFORE INSERT ON audit_events WHEN NEW.event='csv_imported' BEGIN SELECT RAISE(ABORT,'Synthetic disk failure'); END;").unwrap();
        assert!(w
            .stage_client_csv(
                &client.id,
                StageInput {
                    expected_revision: 0,
                    token: p.token.clone(),
                    document: doc("Checking")
                }
            )
            .is_err());
        let empty = w.get_client_imports(&client.id).unwrap();
        assert!(
            empty.accounts.is_empty() && empty.documents.is_empty() && empty.records.is_empty()
        );
        assert_eq!(empty.revision, 0);
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER fail_import;")
            .unwrap();
        let s = w
            .stage_client_csv(
                &client.id,
                StageInput {
                    expected_revision: 0,
                    token: p.token,
                    document: doc("Checking"),
                },
            )
            .unwrap();
        assert_eq!(s.records[0].raw.reported_description, "Synthetic wages");
        assert_eq!(s.records.len(), 5);
        let mut competing = Workspace::new(root.path().to_path_buf());
        competing.unlock_workspace(PASSWORD.into()).unwrap();
        let eligible = s.records.iter().find(|r| r.bulk_eligible).unwrap();
        let id = eligible.id.clone();
        let fields = eligible.fields.clone();
        let saved = review(
            &mut w,
            &client.id,
            &id,
            Status::Approved,
            "Verified income",
            false,
        )
        .unwrap();
        assert!(competing
            .review_transaction(
                &client.id,
                ReviewInput {
                    expected_revision: s.revision,
                    record_id: id.clone(),
                    status: Status::Rejected,
                    fields,
                    reason: "Stale window".into(),
                    acknowledge_warnings: false
                }
            )
            .err()
            .unwrap()
            .contains("changed"));
        assert_eq!(
            competing
                .get_client_imports(&client.id)
                .unwrap()
                .summary
                .income,
            saved.summary.income
        );
        let other_row = stage(
            &mut w,
            &other.id,
            dir.path(),
            "other.csv",
            CHECKING,
            doc("Other checking"),
        )
        .records[0]
            .id
            .clone();
        let s = w.get_client_imports(&client.id).unwrap();
        let local = s
            .records
            .iter()
            .find(|r| r.bulk_eligible)
            .unwrap()
            .id
            .clone();
        assert!(w
            .bulk_approve_transactions(
                &client.id,
                BulkInput {
                    expected_revision: s.revision,
                    record_ids: vec![local, other_row]
                }
            )
            .is_err());
        assert_eq!(
            w.get_client_imports(&client.id).unwrap().revision,
            s.revision
        );
        w.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER fail_review BEFORE INSERT ON audit_events WHEN NEW.event='transaction_reviewed' BEGIN SELECT RAISE(ABORT,'Synthetic disk failure'); END;").unwrap();
        assert!(review(
            &mut w,
            &client.id,
            &id,
            Status::Rejected,
            "Simulated rejected write",
            false
        )
        .is_err());
        assert_eq!(
            w.get_client_imports(&client.id).unwrap().summary.income,
            "2000.00"
        );
        w.lock_workspace();
        assert!(w.pending_csv.is_none());
        assert_eq!(w.get_client_imports(&client.id).err().unwrap(), LOCKED);
        assert_eq!(
            w.inspect_client_csv(client.id, path.to_string_lossy().into())
                .err()
                .unwrap(),
            LOCKED
        );
    }
    #[test]
    fn missing_amounts_mismatches_corrections_and_active_text_never_bulk_approve() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let mut d = doc("Checking");
        d.closing_balance = Some("1100".into());
        let s=stage(&mut w,&client.id,dir.path(),"uncertain.csv","Date,Description,Amount,Category,Type\n2026-09-01,Missing source amount,,Other,income\n2026-09-02,=SUM(A1),10,Other,income\n",d);
        assert!(s.records.iter().all(|r| !r.bulk_eligible));
        assert_eq!(s.summary.income, "0.00");
        assert!(s.documents[0].source_difference.is_none());
        let r = &s.records[0];
        let mut fields = r.fields.clone();
        fields.amount = Some("100.01".into());
        assert!(w
            .review_transaction(
                &client.id,
                ReviewInput {
                    expected_revision: s.revision,
                    record_id: r.id.clone(),
                    status: Status::Approved,
                    fields: fields.clone(),
                    reason: String::new(),
                    acknowledge_warnings: false
                }
            )
            .is_err());
        let s = w
            .review_transaction(
                &client.id,
                ReviewInput {
                    expected_revision: s.revision,
                    record_id: r.id.clone(),
                    status: Status::Approved,
                    fields,
                    reason: "Counselor verified the missing source figure separately".into(),
                    acknowledge_warnings: true,
                },
            )
            .unwrap();
        assert_eq!(s.summary.income, "100.01");
        assert!(s.records[0].raw.reported_amount.as_deref() == Some(""));
        assert!(s.records[0].raw.extracted.amount.is_none());
        let mut d = doc("Mismatch");
        d.closing_balance = Some("1001".into());
        let s = stage(
            &mut w,
            &client.id,
            dir.path(),
            "mismatch.csv",
            "Date,Description,Amount,Category,Type\n2026-09-03,Known value,100,Other,income\n",
            d,
        );
        assert_eq!(
            s.documents.last().unwrap().source_difference.as_deref(),
            Some("99.00")
        );
        let r = s.records.last().unwrap();
        assert!(!r.bulk_eligible);
        assert!(w
            .bulk_approve_transactions(
                &client.id,
                BulkInput {
                    expected_revision: s.revision,
                    record_ids: vec![r.id.clone()]
                }
            )
            .is_err());
    }
    #[test]
    fn source_retention_is_authorized_encrypted_and_backup_restores_decision_history() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let mut d = doc("Checking");
        d.retain_source = true;
        let s = stage(&mut w, &client.id, dir.path(), "retained.csv", CHECKING, d);
        assert!(s.documents[0].source_retained);
        let r = s
            .records
            .iter()
            .find(|r| r.bulk_eligible)
            .unwrap()
            .id
            .clone();
        let s = review(
            &mut w,
            &client.id,
            &r,
            Status::Approved,
            "Verified source income",
            false,
        )
        .unwrap();
        let history = s.history.len();
        let bytes: Vec<u8> = w
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row(
                "SELECT source_blob FROM import_documents WHERE id=?1",
                [&s.documents[0].id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bytes, CHECKING.as_bytes());
        let c = &w.session.as_ref().unwrap().connection;
        for sql in [
            "DELETE FROM import_documents",
            "UPDATE import_records SET raw_json='{}'",
            "DELETE FROM transaction_review_history",
        ] {
            assert!(c.execute(sql, []).is_err());
        }
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let encrypted = fs::read(root.path().join(DATABASE_NAME)).unwrap();
        assert!(!encrypted
            .windows("Synthetic wages".len())
            .any(|v| v == b"Synthetic wages"));
        let r = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(r.token, "RESTORE".into()).unwrap();
        let mut restarted = Workspace::new(root.path().to_path_buf());
        restarted.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = restarted.get_client_imports(&client.id).unwrap();
        assert_eq!(restored.summary.income, "2000.00");
        assert_eq!(restored.history.len(), history);
        assert_eq!(restored.records[0].raw, s.records[0].raw);
        assert!(restored.documents[0].source_retained);
    }
    #[test]
    fn large_overlap_groups_keep_diagnostics_bounded_and_every_row_requires_review() {
        let (_root, mut workspace, _) = configured();
        let directory = tempdir().unwrap();
        let client = workspace.save_client(client_input()).unwrap();
        let source = format!(
            "Date,Description,Amount,Category,Type\n{}",
            "2026-09-01,Identical synthetic wages,1,Wages,income\n".repeat(2000)
        );
        let mut input = doc("Checking");
        input.closing_balance = Some("3000".into());
        let snapshot = stage(
            &mut workspace,
            &client.id,
            directory.path(),
            "overlap.csv",
            &source,
            input,
        );
        assert_eq!(snapshot.records.len(), 2000);
        assert_eq!(snapshot.summary.needs_clarification, 2000);
        assert_eq!(snapshot.summary.income, "0.00");
        assert!(snapshot
            .records
            .iter()
            .all(|r| !r.bulk_eligible && r.duplicate_ids.len() == 20));
        assert!(serde_json::to_vec(&snapshot).unwrap().len() < 6 * 1024 * 1024);
    }
    #[test]
    fn pdf_sources_need_individual_decisions_are_atomic_and_recover_with_original_locators() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let bytes = crate::pdf_fixture::statement();
        let pages = crate::pdf::extract_pages(&bytes).unwrap();
        let selections = pages
            .iter()
            .flat_map(|p| {
                p.text
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.starts_with("2026-"))
                    .map(|(i, _)| crate::pdf::Locator {
                        page: p.number,
                        line: i as u32 + 1,
                    })
            })
            .collect::<Vec<_>>();
        let preview = w
            .capture_pdf(
                client.id.clone(),
                Path::new("synthetic.pdf"),
                Zeroizing::new(bytes.clone()),
                pages,
            )
            .unwrap();
        assert!(w.get_client_imports(&client.id).unwrap().records.is_empty());
        let mut input = doc("PDF checking");
        input.pdf_selection = Some(selections);
        input.retain_source = true;
        input.opening_balance = None;
        input.closing_balance = None;
        assert!(w
            .stage_client_csv(
                &other.id,
                StageInput {
                    expected_revision: 0,
                    token: preview.token.clone(),
                    document: input.clone()
                }
            )
            .is_err());
        w.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER fail_pdf_audit BEFORE INSERT ON audit_events WHEN NEW.event='pdf_imported' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(w
            .stage_client_csv(
                &client.id,
                StageInput {
                    expected_revision: 0,
                    token: preview.token.clone(),
                    document: input.clone()
                }
            )
            .is_err());
        assert!(w
            .get_client_imports(&client.id)
            .unwrap()
            .accounts
            .is_empty());
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute("DROP TRIGGER fail_pdf_audit", [])
            .unwrap();
        let saved = w
            .stage_client_csv(
                &client.id,
                StageInput {
                    expected_revision: 0,
                    token: preview.token,
                    document: input,
                },
            )
            .unwrap();
        assert_eq!(saved.summary.needs_clarification, 4);
        assert!(saved.records.iter().all(|r| !r.bulk_eligible));
        assert!(w
            .bulk_approve_transactions(
                &client.id,
                BulkInput {
                    expected_revision: saved.revision,
                    record_ids: vec![saved.records[0].id.clone()]
                }
            )
            .is_err());
        let mut fields = saved.records[0].fields.clone();
        fields.kind = Kind::Income;
        fields.category = "Wages".into();
        let decide = |ack| ReviewInput {
            expected_revision: saved.revision,
            record_id: saved.records[0].id.clone(),
            fields: fields.clone(),
            status: Status::Approved,
            reason: "Verified PDF page and currency against original synthetic source".into(),
            acknowledge_warnings: ack,
        };
        assert!(w.review_transaction(&client.id, decide(false)).is_err());
        let accepted = w.review_transaction(&client.id, decide(true)).unwrap();
        assert_eq!(accepted.summary.income, "1000.01");
        assert_eq!(accepted.history.len(), 1);
        assert!(w.get_client_imports(&other.id).unwrap().records.is_empty());
        let source: Vec<u8> = w
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row(
                "SELECT source_blob FROM import_documents WHERE id=?1",
                [&accepted.documents[0].id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(source, bytes);
        let mut overlap_input = doc("unused");
        overlap_input.account_id = Some(accepted.accounts[0].id.clone());
        overlap_input.opening_balance = None;
        overlap_input.closing_balance = None;
        let overlaps=stage(&mut w,&client.id,dir.path(),"overlap.csv","Date,Description,Amount,Category,Type\n2026-10-01,Fictional payroll,1000.01,Wages,income\n",overlap_input);
        let overlapping = overlaps.records.last().unwrap();
        assert!(!overlapping.bulk_eligible);
        assert!(overlapping.duplicate_ids.contains(&accepted.records[0].id));
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let encrypted = fs::read(root.path().join(DATABASE_NAME)).unwrap();
        assert!(!encrypted.windows(17).any(|v| v == b"Fictional payroll"));
        let prepared = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(prepared.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_client_imports(&client.id).unwrap();
        assert_eq!(restored.summary.income, "1000.01");
        assert_eq!(restored.records[0].raw, accepted.records[0].raw);
        assert_eq!(restored.history.len(), 1);
        assert!(restored.documents[0].source_retained);
        let withdrawn = review(
            &mut w,
            &client.id,
            &restored.records[0].id,
            Status::Rejected,
            "Withdraw synthetic PDF approval",
            false,
        )
        .unwrap();
        assert_eq!(withdrawn.summary.income, "0.00");
        assert_eq!(withdrawn.history.len(), 2);
    }
    #[test]
    fn pdf_schema_eight_upgrade_failure_preserves_csv_sources_and_approval_history() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let saved = stage(
            &mut w,
            &client.id,
            dir.path(),
            "before.csv",
            CHECKING,
            doc("Checking"),
        );
        let before = review(
            &mut w,
            &client.id,
            &saved.records[0].id,
            Status::Approved,
            "CSV verified before PDF upgrade",
            false,
        )
        .unwrap();
        let connection = &mut w.session.as_mut().unwrap().connection;
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        connection
            .execute_batch("DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE IF EXISTS client_action_plans; DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; PRAGMA user_version=8; CREATE TABLE audit_events_v9(collision TEXT);")
            .unwrap();
        assert!(migrate(connection, false).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            8
        );
        assert!(connection.execute("DELETE FROM audit_events", []).is_err());
        connection
            .execute("DROP TABLE audit_events_v9", [])
            .unwrap();
        migrate(connection, false).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            count
        );
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
        let after = w.get_client_imports(&client.id).unwrap();
        assert_eq!(after.summary.income, before.summary.income);
        assert_eq!(after.records[0].raw, before.records[0].raw);
        assert_eq!(after.history.len(), before.history.len());
    }
    #[test]
    fn expiry_wrong_tokens_and_client_scope_never_stage_uninspected_data() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let path = dir.path().join("source.csv");
        fs::write(&path, CHECKING).unwrap();
        let p = w
            .inspect_client_csv(client.id.clone(), path.to_string_lossy().into())
            .unwrap();
        w.cancel_client_csv("unrelated-old-token");
        assert!(w.pending_csv.is_some());
        w.cancel_client_csv(&p.token);
        assert!(w.pending_csv.is_none());
        assert!(w
            .stage_client_csv(
                &client.id,
                StageInput {
                    expected_revision: 0,
                    token: p.token,
                    document: doc("Checking")
                }
            )
            .is_err());
        let p = w
            .inspect_client_csv(client.id.clone(), path.to_string_lossy().into())
            .unwrap();
        w.pending_csv.as_mut().unwrap().created = Instant::now() - AUTO_LOCK_AFTER;
        assert!(w
            .stage_client_csv(
                &client.id,
                StageInput {
                    expected_revision: 0,
                    token: p.token,
                    document: doc("Checking")
                }
            )
            .is_err());
        assert!(w.get_client_imports(&client.id).unwrap().records.is_empty());
    }

    fn action_input(a: &crate::assessments::Record) -> crate::action_plans::Input {
        use crate::action_plans::{Action, Data, Input, Owner};
        Input {
            assessment_id: a.payload.id.clone(),
            expected_digest: a.digest.clone(),
            title: "Fictional agreed plan – José".into(),
            acknowledge: true,
            data: Data {
                budget_recommendations:
                    "Agree on the selected model contributions; verify actual lender dates.".into(),
                actions: [
                    "Verify missing statements",
                    "Confirm lender payment dates",
                    "Arrange primary savings contributions",
                ]
                .into_iter()
                .map(|s| Action {
                    step: s.into(),
                    owner: Owner::Client,
                    due_date: None,
                })
                .collect(),
                follow_up: "Arrange the next appointment after reviewing the records.".into(),
                follow_up_date: None,
                limitations: "Entered assumptions; statement coverage remains unverified.".into(),
            },
        }
    }
    #[test]
    fn client_action_plan_exact_agreement_immutable_basis_and_client_pdf() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        saved_planning_fixture(&mut w, &c.id);
        let input = scenario_input(&mut w, &s, &c);
        let a = w.save_assessment_record(&c.id, input).unwrap();
        let review = w.review_client_action_plan(&c.id, &a.payload.id).unwrap();
        assert_eq!(review.allocations.monthly_income, "3250.00");
        assert_eq!(review.allocations.surplus, "2550.00");
        assert_eq!(review.allocations.debt, "100.00");
        assert_eq!(review.allocations.savings, "250.00");
        assert_eq!(review.allocations.unallocated, "2200.00");
        assert!(review
            .disclosures
            .iter()
            .any(|x| x.contains("debt USD 150.00")));
        let r = w.save_client_action_plan(&c.id, action_input(&a)).unwrap();
        let id = r.payload.id.clone();
        let digest = r.digest.clone();
        assert_eq!(r.payload.allocations, review.allocations);
        assert_eq!(r.assessment.payload.assessment.operating_cashflow, "799.99");
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, s.revision);
        assert_eq!(w.get_client_budget(&c.id).unwrap().revision, 1);
        let conn = &w.session.as_ref().unwrap().connection;
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event='action_plan_recorded'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert!(conn
            .execute("UPDATE client_action_plans SET title='changed'", [])
            .is_err());
        assert!(conn.execute("DELETE FROM client_action_plans", []).is_err());
        let mut changed = budget_input(1);
        changed.lines[0].amount = Some("2000.01".into());
        w.save_client_budget(&c.id, changed).unwrap();
        let old = w.get_client_action_plan(&c.id, &id).unwrap();
        assert!(old.needs_review);
        assert_eq!(old.digest, digest);
        assert_eq!(old.payload.allocations.unallocated, "2200.00");
        assert!(w.list_client_action_plans(&c.id).unwrap().records[0].needs_review);
        assert!(w.review_client_action_plan(&c.id, &a.payload.id).is_err());
        assert!(w.save_client_action_plan(&c.id, action_input(&a)).is_err());
        let path = dir.path().join("client.pdf");
        w.export_client_action_plan(&c.id, &id, path.to_str().unwrap())
            .unwrap();
        let doc = lopdf::Document::load(&path).unwrap();
        let text = doc
            .extract_text(&doc.get_pages().keys().copied().collect::<Vec<_>>())
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for expected in [
            "Your financial action plan",
            "José",
            "NEEDS REVIEW",
            "Agreed debt allocation: USD 100.00",
            "Agreed primary savings allocation: USD 250.00",
            "Unallocated / buffer: USD 2200.00",
            "Operating cash flow: USD 799.99",
            "2027-01-31",
            "USD 43.10",
            "Follow-up",
            "incomplete",
        ] {
            assert!(text.contains(expected), "Missing {expected}: {text}");
        }
        assert!(
            text.find("Verify missing statements").unwrap()
                < text.find("Confirm lender payment dates").unwrap()
        );
        assert!(!text.contains("INTERNAL / INCOMPLETE"));
        let bytes = fs::read(&path).unwrap();
        assert!(w
            .export_client_action_plan(&c.id, &id, path.to_str().unwrap())
            .is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn client_action_plan_rejects_invalid_steps_dates_scope_and_atomic_audit_failure() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        saved_planning_fixture(&mut w, &c.id);
        let input = scenario_input(&mut w, &s, &c);
        let a = w.save_assessment_record(&c.id, input).unwrap();
        for case in 0..8 {
            let mut input = action_input(&a);
            match case {
                0 => input.data.actions.truncate(2),
                1 => input.data.actions = vec![input.data.actions[0].clone(); 6],
                2 => input.data.actions[1].step = input.data.actions[0].step.to_uppercase(),
                3 => input.data.actions[0].due_date = Some("2026-02-30".into()),
                4 => input.data.follow_up_date = Some("2020-01-01".into()),
                5 => input.acknowledge = false,
                6 => input.expected_digest = "wrong".into(),
                _ => input.data.limitations = " ".into(),
            };
            assert!(
                w.save_client_action_plan(&c.id, input).is_err(),
                "case {case}"
            );
        }
        assert!(w
            .save_client_action_plan(&other.id, action_input(&a))
            .is_err());
        let conn = &w.session.as_ref().unwrap().connection;
        conn.execute_batch("CREATE TRIGGER reject_action_audit BEFORE INSERT ON audit_events WHEN NEW.event='action_plan_recorded' BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
        assert!(w.save_client_action_plan(&c.id, action_input(&a)).is_err());
        assert!(w
            .list_client_action_plans(&c.id)
            .unwrap()
            .records
            .is_empty());
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_action_audit;")
            .unwrap();
        let saved = w.save_client_action_plan(&c.id, action_input(&a)).unwrap();
        let id = &saved.payload.id;
        assert!(w.get_client_action_plan(&other.id, id).is_err());
        assert!(w
            .export_client_action_plan(
                &other.id,
                id,
                dir.path().join("wrong.pdf").to_str().unwrap()
            )
            .is_err());
        assert!(w
            .export_client_action_plan(&c.id, id, w.data_dir.join("bad.pdf").to_str().unwrap())
            .is_err());
        w.lock_workspace();
        assert!(w.get_client_action_plan(&c.id, id).is_err());
        assert!(w
            .export_client_action_plan(&c.id, id, dir.path().join("locked.pdf").to_str().unwrap())
            .is_err());
    }
    #[test]
    fn client_action_plan_cannot_invent_unknown_or_excess_allocations() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        saved_planning_fixture(&mut w, &c.id);
        let mut b = budget_input(1);
        b.lines[0].amount = None;
        w.save_client_budget(&c.id, b).unwrap();
        let input = scenario_input(&mut w, &s, &c);
        let a = w.save_assessment_record(&c.id, input).unwrap();
        assert!(w.review_client_action_plan(&c.id, &a.payload.id).is_err());
        let mut b = budget_input(2);
        b.lines[0].amount = Some("100".into());
        b.lines[0].frequency = crate::budget::Frequency::Monthly;
        b.lines.push(crate::budget::BudgetLine {
            label: "Necessary".into(),
            kind: crate::budget::Kind::Necessary,
            amount: Some("1".into()),
            frequency: crate::budget::Frequency::Monthly,
            estimated: true,
        });
        w.save_client_budget(&c.id, b).unwrap();
        let input = scenario_input(&mut w, &s, &c);
        let a = w.save_assessment_record(&c.id, input).unwrap();
        assert!(w
            .review_client_action_plan(&c.id, &a.payload.id)
            .err()
            .unwrap()
            .contains("exceed"));
        assert!(w
            .list_client_action_plans(&c.id)
            .unwrap()
            .records
            .is_empty());
    }
    #[test]
    fn client_action_plan_schema_fourteen_upgrade_restart_and_encrypted_recovery() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        saved_planning_fixture(&mut w, &c.id);
        let input = scenario_input(&mut w, &s, &c);
        let a = w.save_assessment_record(&c.id, input).unwrap();
        let conn = &mut w.session.as_mut().unwrap().connection;
        conn.execute_batch("DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE client_action_plans; DROP INDEX assessment_records_identity; PRAGMA user_version=14; CREATE TABLE audit_events_v15(collision TEXT);").unwrap();
        assert!(migrate(conn, false).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            14
        );
        assert!(conn.prepare("SELECT * FROM client_action_plans").is_err());
        conn.execute_batch("DROP TABLE audit_events_v15;").unwrap();
        migrate(conn, false).unwrap();
        let r = w.save_client_action_plan(&c.id, action_input(&a)).unwrap();
        let id = r.payload.id;
        let digest = r.digest;
        let mut b = budget_input(1);
        b.lines[0].amount = Some("2000".into());
        w.save_client_budget(&c.id, b).unwrap();
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        drop(w);
        let mut w = Workspace::new(root.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let r = w.get_client_action_plan(&c.id, &id).unwrap();
        assert_eq!(r.digest, digest);
        assert!(r.needs_review);
        w.lock_workspace();
        let ready = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(ready.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let r = w.get_client_action_plan(&c.id, &id).unwrap();
        assert_eq!(r.digest, digest);
        assert!(r.needs_review);
        assert_eq!(r.payload.allocations.unallocated, "2200.00");
    }
    #[test]
    fn client_action_plan_forged_financial_output_is_rejected_even_with_rehashed_json() {
        use sha2::{Digest, Sha256};
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = facts_fixture(&mut w, &c.id, dir.path());
        saved_planning_fixture(&mut w, &c.id);
        let input = scenario_input(&mut w, &s, &c);
        let a = w.save_assessment_record(&c.id, input).unwrap();
        let mut r = w.save_client_action_plan(&c.id, action_input(&a)).unwrap();
        r.payload.allocations.unallocated = "999999.99".into();
        assert!(crate::action_plans::validate(&r.payload, &r.assessment).is_err());
        let json = serde_json::to_string(&r.payload).unwrap();
        let digest = format!("{:x}", Sha256::digest(json.as_bytes()));
        let conn = &w.session.as_ref().unwrap().connection;
        conn.execute_batch("DROP TRIGGER action_plans_no_update;")
            .unwrap();
        conn.execute(
            "UPDATE client_action_plans SET payload_json=?1,digest=?2 WHERE id=?3",
            params![json, digest, r.payload.id],
        )
        .unwrap();
        assert!(w.get_client_action_plan(&c.id, &r.payload.id).is_err());
        let path = dir.path().join("forged.pdf");
        assert!(w
            .export_client_action_plan(&c.id, &r.payload.id, path.to_str().unwrap())
            .is_err());
        assert!(!path.exists());
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
    }

    fn report_input(
        w: &mut Workspace,
        s: &Snapshot,
        c: &Client,
        ids: Vec<String>,
    ) -> crate::assessments::Input {
        let mut input = assessment_input(s, c);
        input.from_month = "2026-01".into();
        input.through_month = "2026-03".into();
        input.supporting = Some(crate::report_evidence::Selection {
            expected_notes_count: w.get_assessment_evidence(&c.id).unwrap().notes_count,
            note_ids: ids,
        });
        input
    }
    #[test]
    fn counselor_report_keeps_selected_notes_reviewed_payments_and_independent_financial_totals() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = recurring_fixture(&mut w, &c.id, dir.path());
        let s = w
            .review_recurring_payment(&c.id, recurring_input(&s))
            .unwrap();
        let n = w
            .add_client_note(
                &c.id,
                "Fictional José discussion USD 9999.99; this narrative is not approved income.",
            )
            .unwrap();
        let input = report_input(&mut w, &s, &c, vec![n.id.clone()]);
        let a = w.save_assessment_record(&c.id, input).unwrap();
        let id = a.payload.id.clone();
        let digest = a.digest.clone();
        assert_eq!(a.payload.format, 3);
        assert_eq!(
            a.payload.supporting.as_ref().unwrap().notes,
            vec![n.clone()]
        );
        assert_eq!(a.payload.assessment.totals.income, "0.00");
        assert_eq!(a.payload.assessment.totals.spending, "32.05");
        assert_eq!(a.payload.assessment.operating_cashflow, "-32.05");
        assert_eq!(a.payload.basis.merchant_groups[0].total_paid, "32.05");
        assert_eq!(a.payload.basis.merchant_groups[0].state, "Confirmed");
        assert_eq!(
            a.payload.basis.recurring_history[0].evidence,
            s.recurring_history[0].evidence
        );
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, s.revision);
        w.add_client_note(&c.id, "Later fictional note, not part of the old report.")
            .unwrap();
        let old = w.get_assessment_record(&c.id, &id).unwrap();
        assert!(old.needs_review);
        assert_eq!(old.digest, digest);
        assert_eq!(old.payload.supporting.as_ref().unwrap().notes, vec![n]);
        assert!(w.list_assessment_records(&c.id).unwrap().records[0].needs_review);
        let path = dir.path().join("internal.pdf");
        w.export_assessment_pdf(&c.id, &id, path.to_str().unwrap())
            .unwrap();
        let doc = lopdf::Document::load(path).unwrap();
        let text = doc
            .extract_text(&doc.get_pages().keys().copied().collect::<Vec<_>>())
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for word in [
            "Selected counseling session notes",
            "José discussion USD 9999.99",
            "Historical total / smallest / largest: USD 32.05 / USD 10.01 / USD 12.03",
            "Confirmed",
            "Original reviewed payment",
            "Operating cash flow: USD -32.05",
            "NEEDS REVIEW",
        ] {
            assert!(text.contains(word), "Missing {word}");
        }
        assert!(!text.contains("Later fictional note, not part"));
        assert!(text.contains(&s.recurring_history[0].evidence[0].approval_id));
    }
    #[test]
    fn counselor_report_preserves_older_approval_bindings_after_payment_correction() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = recurring_fixture(&mut w, &c.id, dir.path());
        let s = w
            .review_recurring_payment(&c.id, recurring_input(&s))
            .unwrap();
        let original = s.recurring_history[0].evidence[0].approval_id.clone();
        let mut input = crate::imports::ReviewInput {
            expected_revision: s.revision,
            record_id: s.records[0].id.clone(),
            status: Status::Approved,
            fields: s.records[0].fields.clone(),
            reason: "Correct fictional payment".into(),
            acknowledge_warnings: true,
        };
        input.fields.amount = Some("-9.99".into());
        let s = w.review_transaction(&c.id, input).unwrap();
        let input = report_input(&mut w, &s, &c, vec![]);
        let a = w.save_assessment_record(&c.id, input).unwrap();
        assert_eq!(a.payload.basis.merchant_groups[0].state, "Needs review");
        assert_eq!(a.payload.basis.merchant_groups[0].total_paid, "32.03");
        assert_eq!(a.payload.assessment.totals.spending, "32.03");
        assert!(a.payload.basis.history.iter().any(|h| h.id == original));
        assert_eq!(
            a.payload.basis.recurring_history[0].evidence[0]
                .fields
                .amount
                .as_deref(),
            Some("-10.01")
        );
        crate::assessments::validate(&a.payload).unwrap();
        let s = review(
            &mut w,
            &c.id,
            &s.records[0].id,
            Status::Rejected,
            "Withdraw fictional payment",
            true,
        )
        .unwrap();
        let input = report_input(&mut w, &s, &c, vec![]);
        let latest = w.save_assessment_record(&c.id, input).unwrap();
        assert_eq!(latest.payload.basis.merchant_groups[0].total_paid, "22.04");
        assert_eq!(
            latest.payload.basis.recurring_history[0].evidence[0]
                .fields
                .amount
                .as_deref(),
            Some("-10.01")
        );
        assert!(
            w.get_assessment_record(&c.id, &a.payload.id)
                .unwrap()
                .needs_review
        );
    }
    #[test]
    fn counselor_report_rejects_stale_notes_foreign_selection_and_forged_evidence_before_restore() {
        use sha2::{Digest, Sha256};
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let s = recurring_fixture(&mut w, &c.id, dir.path());
        let n = w
            .add_client_note(&c.id, "Original fictional counseling context.")
            .unwrap();
        let foreign = w
            .add_client_note(&other.id, "Other fictional client's context.")
            .unwrap();
        let mut input = report_input(&mut w, &s, &c, vec![n.id.clone()]);
        input.supporting.as_mut().unwrap().expected_notes_count = 0;
        assert!(w.save_assessment_record(&c.id, input).is_err());
        for ids in [
            vec![foreign.id],
            vec![n.id.clone(), n.id.clone()],
            vec![n.id.clone(); 11],
        ] {
            let input = report_input(&mut w, &s, &c, ids);
            assert!(w.save_assessment_record(&c.id, input).is_err());
        }
        assert!(w.list_assessment_records(&c.id).unwrap().records.is_empty());
        let input = report_input(&mut w, &s, &c, vec![n.id]);
        let mut a = w.save_assessment_record(&c.id, input).unwrap();
        let id = a.payload.id.clone();
        a.payload.basis.merchant_groups[0].total_paid = "9999.99".into();
        assert!(crate::assessments::validate(&a.payload).is_err());
        a.payload.basis.merchant_groups[0].total_paid = "32.05".into();
        a.payload.supporting.as_mut().unwrap().notes[0].body =
            "Forged counselor note with matching hash.".into();
        crate::assessments::validate(&a.payload).unwrap();
        let json = serde_json::to_string(&a.payload).unwrap();
        let digest = format!("{:x}", Sha256::digest(json.as_bytes()));
        let conn = &w.session.as_ref().unwrap().connection;
        conn.execute_batch("DROP TRIGGER assessment_records_no_update;")
            .unwrap();
        conn.execute(
            "UPDATE client_assessment_records SET payload_json=?1,digest=?2 WHERE id=?3",
            params![json, digest, id],
        )
        .unwrap();
        assert!(w.get_assessment_record(&c.id, &id).is_err());
        let path = dir.path().join("forged.pdf");
        assert!(w
            .export_assessment_pdf(&c.id, &id, path.to_str().unwrap())
            .is_err());
        assert!(!path.exists());
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        assert!(w.get_assessment_evidence(&c.id).is_err());
    }
    #[test]
    fn counselor_report_restart_and_recovery_preserve_format_three_and_legacy_records() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let c = w.save_client(client_input()).unwrap();
        let s = recurring_fixture(&mut w, &c.id, dir.path());
        let n = w
            .add_client_note(
                &c.id,
                "Fictional selected follow-up, unchanged after recovery.",
            )
            .unwrap();
        let legacy = w
            .save_assessment_record(&c.id, assessment_input(&s, &c))
            .unwrap();
        assert_eq!(legacy.payload.format, 2);
        assert!(legacy.payload.supporting.is_none());
        let input = report_input(&mut w, &s, &c, vec![n.id]);
        let report = w.save_assessment_record(&c.id, input).unwrap();
        let id = report.payload.id.clone();
        let digest = report.digest;
        let backup = w
            .create_backup(dir.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        drop(w);
        let mut w = Workspace::new(root.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(w.get_assessment_record(&c.id, &id).unwrap().digest, digest);
        w.add_client_note(
            &c.id,
            "Later context that recovery will remove from active workspace.",
        )
        .unwrap();
        assert!(w.get_assessment_record(&c.id, &id).unwrap().needs_review);
        assert!(
            !w.get_assessment_record(&c.id, &legacy.payload.id)
                .unwrap()
                .needs_review
        );
        w.lock_workspace();
        let ready = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(ready.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_assessment_record(&c.id, &id).unwrap();
        assert_eq!(restored.digest, digest);
        assert!(!restored.needs_review);
        assert_eq!(
            restored.payload.supporting.as_ref().unwrap().notes[0].body,
            "Fictional selected follow-up, unchanged after recovery."
        );
        assert_eq!(
            w.get_assessment_record(&c.id, &legacy.payload.id)
                .unwrap()
                .digest,
            legacy.digest
        );
    }
}
