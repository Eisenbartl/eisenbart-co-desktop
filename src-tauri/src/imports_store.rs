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
        documents.push(Document {
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
    Ok(Snapshot {
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
    use crate::storage::tests::{budget_input, client_input, configured, PASSWORD};
    use tempfile::tempdir;
    pub(super) const CHECKING:&str="Date,Description,Amount,Category,Type\n2026-09-01,Synthetic wages,2000,Wages,income\n2026-09-02,Synthetic rent,-1200,Housing,expense\n2026-09-03,Synthetic fee,-10.01,Fees,fee\n2026-09-04,Synthetic refund,20.01,Housing,refund\n2026-09-05,Synthetic reserve transfer,-200,Savings,transfer\n";
    fn doc(label: &str) -> DocumentInput {
        DocumentInput {
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
        connection.execute_batch("DROP TABLE recurring_review_invalidations; PRAGMA user_version=10; CREATE TABLE recurring_review_invalidations(collision TEXT);").unwrap();
        assert!(migrate(connection, false).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            10
        );
        connection
            .execute_batch("DROP TABLE recurring_review_invalidations;")
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
                "DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history;",
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
            .execute_batch("DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; PRAGMA user_version=8; CREATE TABLE audit_events_v9(collision TEXT);")
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
}
