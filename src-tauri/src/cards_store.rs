//! Scoped immutable card sources and append-only individual decisions.
use super::*;
use crate::cards::{self, Review, Row, Snapshot, Source, Statement};
use sha2::{Digest, Sha256};
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn json<T: Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|_| STORAGE_ERROR.into())
}
fn decode<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    serde_json::from_str(s).map_err(|_| STORAGE_ERROR.into())
}
pub(super) fn has_cards(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<bool, String> {
    c.query_row("SELECT EXISTS(SELECT 1 FROM card_statements WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3)",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)
}
fn bounds(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<(), String> {
    for (table, max, size) in [
        ("card_statements", 20, 20 * 1024 * 1024),
        ("card_decisions", 4000, 8 * 1024 * 1024),
    ] {
        let column = if table == "card_statements" {
            "payload_json"
        } else {
            "fields_json"
        };
        let(n,bytes):(i64,i64)=c.query_row(&format!("SELECT COUNT(*),COALESCE(SUM(length(CAST({column} AS BLOB))),0) FROM {table} WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3"),params![client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(storage_error)?;
        if n > max || bytes > size {
            return Err("Card history exceeds this stage's safety limit.".into());
        }
    }
    Ok(())
}
fn status(s: &crate::imports::Status) -> &'static str {
    match s {
        crate::imports::Status::Pending => "Pending",
        crate::imports::Status::Approved => "Approved",
        crate::imports::Status::Rejected => "Rejected",
        crate::imports::Status::NeedsClarification => "Needs Clarification",
    }
}
fn parse_status(s: &str) -> Result<crate::imports::Status, String> {
    match s {
        "Pending" => Ok(crate::imports::Status::Pending),
        "Approved" => Ok(crate::imports::Status::Approved),
        "Rejected" => Ok(crate::imports::Status::Rejected),
        "Needs Clarification" => Ok(crate::imports::Status::NeedsClarification),
        _ => Err(STORAGE_ERROR.into()),
    }
}
pub(super) fn read(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<Snapshot, String> {
    read_at(c, w, client, imports_store::revision(c, w, client)?)
}
pub(super) fn read_at(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    revision: i64,
) -> Result<Snapshot, String> {
    bounds(c, w, client)?;
    let current_revision = imports_store::revision(c, w, client)?;
    if revision < 0 || revision > current_revision {
        return Err(STORAGE_ERROR.into());
    }
    let saved=c.prepare("SELECT id,payload_json,digest,source_hash,source_blob FROM card_statements WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid LIMIT 21").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<Vec<u8>>>(4)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    let mut statements = vec![];
    for (id, body, digest, source_hash, blob) in saved {
        let blob = blob.map(Zeroizing::new);
        if body.len() > cards::MAX_JSON || hash(body.as_bytes()) != digest {
            return Err(STORAGE_ERROR.into());
        }
        let source: Source = decode(&body)?;
        if source.import_revision > revision {
            continue;
        }
        if source.format != 1
            || source.id != id
            || source.client_id != client
            || Uuid::parse_str(&id).is_err()
            || chrono::DateTime::parse_from_rfc3339(&source.imported_at).is_err()
            || source.sha256 != source_hash
            || source.sha256.len() != 64
            || !source.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || source.import_revision < 1
            || source.import_revision > revision
            || source.byte_count == 0
            || source.byte_count > cards::MAX_BYTES
            || source.rows.is_empty()
            || source.rows.len() > cards::MAX_ROWS
            || source.file_name.is_empty()
            || source.file_name.len() > 255
            || source.file_name.chars().any(char::is_control)
            || source.input.retain_source != blob.is_some()
            || json(&cards::canonical(source.input.clone())?)? != json(&source.input)?
        {
            return Err(STORAGE_ERROR.into());
        }
        if let Some(bytes) = blob {
            if bytes.len() != source.byte_count || hash(&bytes) != source.sha256 {
                return Err(STORAGE_ERROR.into());
            }
            let csv = crate::imports::read_csv(&bytes)?;
            let raw = crate::imports::extract(&csv, &source.input.mapping)?;
            if json(&raw)? != json(&source.rows)? {
                return Err(STORAGE_ERROR.into());
            }
        }
        let mut rows = source
            .rows
            .iter()
            .enumerate()
            .map(|(row, raw)| {
                Ok(Row {
                    row,
                    raw: decode(&json(raw)?)?,
                    fields: cards::initial(raw),
                    status: crate::imports::Status::Pending,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        // Even without retained bytes, validate the declared mapping and captured raw cells.
        let mut columns = vec![
            source.input.mapping.date,
            source.input.mapping.description,
            source.input.mapping.amount.ok_or(STORAGE_ERROR)?,
        ];
        if let Some(n) = source.input.mapping.category {
            columns.push(n)
        }
        let width = columns.into_iter().max().ok_or(STORAGE_ERROR)? + 1;
        if width > 64 {
            return Err(STORAGE_ERROR.into());
        }
        let mut reconstructed = crate::imports::CsvData {
            headers: vec![String::new(); width],
            rows: vec![],
        };
        let mut line = 1;
        for raw in &source.rows {
            if raw.line <= line {
                return Err(STORAGE_ERROR.into());
            }
            line = raw.line;
            let mut cells = vec![String::new(); width];
            cells[source.input.mapping.date] = raw.reported_date.clone();
            cells[source.input.mapping.description] = raw.reported_description.clone();
            cells[source.input.mapping.amount.ok_or(STORAGE_ERROR)?] =
                raw.reported_amount.clone().ok_or(STORAGE_ERROR)?;
            if let Some(n) = source.input.mapping.category {
                cells[n] = raw.reported_category.clone().ok_or(STORAGE_ERROR)?;
            }
            reconstructed.rows.push((raw.line, cells));
        }
        if json(&crate::imports::extract(
            &reconstructed,
            &source.input.mapping,
        )?)? != json(&source.rows)?
        {
            return Err(STORAGE_ERROR.into());
        }

        crate::imports::validate_mapping(&source.input.mapping, width)?;
        for (n, raw) in source.rows.iter().enumerate() {
            if raw.record_number != n as u32 + 2
                || raw.extraction_method != "csv-utf8-v1"
                || raw.page.is_some()
            {
                return Err(STORAGE_ERROR.into());
            }
        }
        let saved=c.prepare("SELECT id,row_index,fields_json,status,reason,reviewed_at,counselor_name,revision FROM card_decisions WHERE statement_id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4 ORDER BY rowid LIMIT 4001").map_err(storage_error)?.query_map(params![id,client,w.organization_id,w.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)? as usize,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,i64>(7)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
        let mut history = vec![];
        let mut previous = source.import_revision;
        for (id, row, fields, state, reason, at, counselor, rev) in saved {
            if rev > revision {
                continue;
            }
            if fields.len() > 8192
                || row >= rows.len()
                || rev <= previous
                || rev > revision
                || Uuid::parse_str(&id).is_err()
                || chrono::DateTime::parse_from_rfc3339(&at).is_err()
                || at < source.imported_at
                || cards::text(&counselor, 120, true)? != counselor
            {
                return Err(STORAGE_ERROR.into());
            }
            let fields: cards::Fields = decode(&fields)?;
            let state = parse_status(&state)?;
            let (canonical, reason2) =
                cards::decision(fields.clone(), &state, &source.input, &reason, true)?;
            if canonical != fields || reason2 != reason {
                return Err(STORAGE_ERROR.into());
            }
            rows[row].fields = fields.clone();
            rows[row].status = state.clone();
            previous = rev;
            history.push(Review {
                id,
                row,
                fields,
                status: state,
                reason,
                reviewed_at: at,
                counselor_name: counselor,
                revision: rev,
            });
        }
        let summary = cards::summary(&source, &rows)?;
        let mut warnings=vec!["Card statements are a separate liability review. Their amounts do not alter bank dashboards or planning figures. Financial Review offers an explicit bank/card supplement for internal reports; card-inclusive final approval remains blocked until complete-basis validation is implemented.".into(),"Positive normalized amounts increase debt; negative amounts reduce it. Payments reduce debt and are not expenses or income. Purchases, fees and interest less refunds supply card spending only. Adjustments are excluded from spending; verify their meaning and cash counterpart.".into(),"Arithmetic reconciliation does not establish complete real-world evidence or verify a bank. Inspect original rows, possible duplicates and rejected activity individually.".into()];
        let mut fingerprints = std::collections::HashSet::new();
        if source
            .rows
            .iter()
            .filter_map(|r| crate::imports::fingerprint(&r.extracted))
            .any(|key| !fingerprints.insert(key))
        {
            warnings.push("Possible duplicate original date, amount and description within this statement. Inspect the source; repeated activity can also be legitimate. Nothing is automatically rejected.".into());
        }
        if source.rows.iter().any(|r| !r.issues.is_empty()) {
            warnings.push("The captured source has extraction or classification warnings. Inspect each original row; no rows are automatically approved.".into())
        }
        if summary.source_difference.is_none() {
            warnings.push("Original source movement is unknown because at least one amount could not be parsed.".into())
        }
        statements.push(Statement {
            source,
            rows,
            history,
            summary,
            warnings,
        });
    }
    for index in 0..statements.len() {
        let overlapping = statements.iter().enumerate().any(|(n, s)| {
            n != index
                && s.source.input.account_label.to_lowercase()
                    == statements[index].source.input.account_label.to_lowercase()
                && s.source.input.period_start <= statements[index].source.input.period_end
                && s.source.input.period_end >= statements[index].source.input.period_start
        });
        if overlapping {
            statements[index].warnings.push("Another recorded statement for this card name overlaps this period. Verify duplicates; statement summaries are not combined automatically.".into())
        }
    }
    Ok(Snapshot {
        client_id: client.into(),
        revision,
        statements,
    })
}
impl Workspace {
    pub fn get_card_statements(&mut self, client: &str) -> Result<Snapshot, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let result = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn stage_card_statement(
        &mut self,
        client: &str,
        input: cards::Stage,
    ) -> Result<Snapshot, String> {
        {
            let s = self.session()?;
            scoped_client(&s.connection, &s.info, client)?;
        }
        if !(0..i64::from(i32::MAX)).contains(&input.expected_revision) {
            return Err("Invalid source revision. Refresh card data before saving.".into());
        }
        let p = self
            .pending_csv
            .as_ref()
            .filter(|p| {
                p.client_id == client
                    && p.token == input.token
                    && !p.expired()
                    && p.pdf_pages.is_none()
            })
            .ok_or("Inspect this client's CSV again; the card preview is missing or expired.")?;
        if p.bytes.len() > cards::MAX_BYTES {
            return Err("Use a card CSV of at most 256 KiB.".into());
        }
        let settings = cards::canonical(input.input)?;
        let csv = crate::imports::read_csv(&p.bytes)?;
        let rows = crate::imports::extract(&csv, &settings.mapping)?;
        if rows.is_empty() || rows.len() > cards::MAX_ROWS {
            return Err("Use 1–200 card transaction rows per statement in this stage.".into());
        }
        let source = Source {
            format: 1,
            id: Uuid::new_v4().to_string(),
            client_id: client.into(),
            imported_at: timestamp(),
            import_revision: input.expected_revision + 1,
            file_name: p.file_name.clone(),
            sha256: p.sha256.clone(),
            byte_count: p.bytes.len(),
            input: settings,
            rows,
        };
        let bytes = source
            .input
            .retain_source
            .then(|| Zeroizing::new(p.bytes.to_vec()));
        let body = json(&source)?;
        if body.len() > cards::MAX_JSON {
            return Err("Card source evidence exceeds 1 MiB.".into());
        }
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        imports_store::bump(&tx, &s.info, client, input.expected_revision)?;
        let current = read(&tx, &s.info, client)?;
        if current.statements.len() >= 20 {
            return Err("Use at most 20 card statements per client in this stage.".into());
        }
        if current
            .statements
            .iter()
            .any(|s| s.source.sha256 == source.sha256)
        {
            return Err("This card CSV is already recorded for this client.".into());
        }
        tx.execute("INSERT INTO card_statements(id,client_id,organization_id,counselor_id,payload_json,digest,source_hash,source_blob) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![source.id,client,s.info.organization_id,s.info.counselor_id,body,hash(body.as_bytes()),source.sha256,bytes.as_ref().map(|b|b.as_slice())]).map_err(storage_error)?;
        audit(&tx, &s.info, "card_statement_imported", Some(&source.id))?;
        let result = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        self.pending_csv = None;
        Ok(result)
    }
    pub fn review_card_transaction(
        &mut self,
        client: &str,
        input: cards::ReviewInput,
    ) -> Result<Snapshot, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        imports_store::bump(&tx, &s.info, client, input.expected_revision)?;
        let current = read(&tx, &s.info, client)?;
        let statement = current
            .statements
            .iter()
            .find(|s| s.source.id == input.statement_id)
            .ok_or("This card statement is unavailable for this client.")?;
        if input.row >= statement.rows.len() {
            return Err("The selected card row is unavailable.".into());
        }
        let (fields, reason) = cards::decision(
            input.fields,
            &input.status,
            &statement.source.input,
            &input.reason,
            input.acknowledge,
        )?;
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO card_decisions(id,statement_id,client_id,organization_id,counselor_id,row_index,fields_json,status,reason,reviewed_at,counselor_name,revision) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![id,input.statement_id,client,s.info.organization_id,s.info.counselor_id,input.row as i64,json(&fields)?,status(&input.status),reason,timestamp(),s.info.counselor_name,input.expected_revision+1]).map_err(storage_error)?;
        audit(&tx, &s.info, "card_transaction_reviewed", Some(&id))?;
        let result = read(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::cards::{Fields, Kind};
    use crate::imports::{AmountMode, DateFormat, Mapping, Status};
    use crate::storage::tests::{client_input, configured, PASSWORD};
    const CSV:&str="Date,Description,Amount,Category\n2026-09-01,Fictional groceries,100.01,Food\n2026-09-02,Fictional refund,-20.02,Food\n2026-09-03,Fictional card payment,-50.03,Card payment\n2026-09-04,Fictional fee,5.04,Fees\n2026-09-05,Fictional interest,2.05,Interest\n";
    pub(crate) fn input() -> cards::Input {
        cards::Input {
            account_label: "Fictional card".into(),
            period_start: "2026-09-01".into(),
            period_end: "2026-09-30".into(),
            opening_owed: "1000.00".into(),
            closing_owed: "1037.05".into(),
            retain_source: true,
            mapping: Mapping {
                date: 0,
                description: 1,
                amount: Some(2),
                category: Some(3),
                amount_mode: AmountMode::Signed,
                signed_is_debit: false,
                date_format: DateFormat::Iso,
                debit: None,
                credit: None,
                merchant: None,
                kind: None,
                reference: None,
                balance: None,
            },
        }
    }
    pub(crate) fn stage(w: &mut Workspace, client: &str, dir: &Path) -> Snapshot {
        let path = dir.join("card.csv");
        fs::write(&path, CSV).unwrap();
        let p = w
            .inspect_client_csv(client.into(), path.to_str().unwrap().into())
            .unwrap();
        let revision = w.get_card_statements(client).unwrap().revision;
        w.stage_card_statement(
            client,
            cards::Stage {
                expected_revision: revision,
                token: p.token,
                input: input(),
            },
        )
        .unwrap()
    }
    pub(crate) fn decision(
        s: &Snapshot,
        row: usize,
        kind: Kind,
        status: Status,
    ) -> cards::ReviewInput {
        let statement = &s.statements[0];
        cards::ReviewInput {
            expected_revision: s.revision,
            statement_id: statement.source.id.clone(),
            row,
            fields: Fields {
                kind,
                ..statement.rows[row].fields.clone()
            },
            status,
            reason: "Inspected original fictional card evidence and possible duplicates".into(),
            acknowledge: true,
        }
    }
    #[test]
    fn card_liability_review_exact_spending_payments_reversal_scope_and_recovery() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w
            .save_client(ClientInput {
                first_name: "Other".into(),
                ..client_input()
            })
            .unwrap();
        let mut s = stage(&mut w, &c.id, root.path());
        assert_eq!(
            s.statements[0].summary.source_difference.as_deref(),
            Some("0.00")
        );
        assert_eq!(s.statements[0].summary.pending, 5);
        assert!(s.statements[0].summary.approved_difference.is_none());
        let mut wrong = decision(&s, 2, Kind::Purchase, Status::Approved);
        assert!(w.review_card_transaction(&c.id, wrong).is_err());
        wrong = decision(&s, 0, Kind::Purchase, Status::Approved);
        wrong.acknowledge = false;
        assert!(w.review_card_transaction(&c.id, wrong).is_err());
        assert!(w
            .review_card_transaction(&other.id, decision(&s, 0, Kind::Purchase, Status::Approved))
            .is_err());
        let stale = decision(&s, 0, Kind::Purchase, Status::Approved);
        for (row, kind) in [
            Kind::Purchase,
            Kind::Refund,
            Kind::Payment,
            Kind::Fee,
            Kind::Interest,
        ]
        .into_iter()
        .enumerate()
        {
            s = w
                .review_card_transaction(&c.id, decision(&s, row, kind, Status::Approved))
                .unwrap();
        }
        let a = &s.statements[0].summary;
        assert!(a.reconciled);
        assert_eq!(a.net_spending, "87.08");
        assert_eq!(a.payments, "50.03");
        assert_eq!(a.liability_movement, "37.05");
        assert_eq!(a.approved_closing.as_deref(), Some("1037.05"));
        assert!(w.review_card_transaction(&c.id, stale).is_err());
        let bank = w.get_client_imports(&c.id).unwrap();
        assert!(bank.records.is_empty());
        assert_eq!(bank.summary.income, "0.00");
        assert_eq!(bank.summary.spending, "0.00");
        assert_eq!(bank.summary.net_cash, "0.00");
        let mut correction = decision(&s, 0, Kind::Purchase, Status::Approved);
        correction.fields.amount = Some("100.02".into());
        s = w.review_card_transaction(&c.id, correction).unwrap();
        assert_eq!(
            s.statements[0].summary.source_difference.as_deref(),
            Some("0.00")
        );
        assert_eq!(
            s.statements[0].summary.approved_difference.as_deref(),
            Some("0.01")
        );
        assert_eq!(
            s.statements[0].rows[0].raw.reported_amount.as_deref(),
            Some("100.01")
        );
        s = w
            .review_card_transaction(&c.id, decision(&s, 0, Kind::Purchase, Status::Rejected))
            .unwrap();
        assert_eq!(s.statements[0].summary.net_spending, "-12.93");
        assert_eq!(s.statements[0].history.len(), 7);
        let original = json(&s).unwrap();
        let bank_revision = s.revision;
        w.lock_workspace();
        assert!(w.get_card_statements(&c.id).is_err());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            json(&w.get_card_statements(&c.id).unwrap()).unwrap(),
            original
        );
        let backup_root = tempfile::tempdir().unwrap();
        let destination = backup_root.path().join("backups");
        fs::create_dir(&destination).unwrap();
        let backup = w
            .create_backup(destination.to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let recovered = w.get_card_statements(&c.id).unwrap();
        assert_eq!(json(&recovered).unwrap(), original);
        assert_eq!(recovered.revision, bank_revision);
        assert!(w
            .get_card_statements(&other.id)
            .unwrap()
            .statements
            .is_empty());
        let s = w.session().unwrap();
        assert!(s
            .connection
            .execute("DELETE FROM card_statements", [])
            .is_err());
        assert!(s
            .connection
            .execute("UPDATE card_decisions SET reason='silent rewrite'", [])
            .is_err());
    }
    #[test]
    fn card_source_and_decision_audit_failures_duplicate_and_corrupt_history_fail_closed() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let path = root.path().join("card.csv");
        fs::write(&path, CSV).unwrap();
        let preview = w
            .inspect_client_csv(c.id.clone(), path.to_str().unwrap().into())
            .unwrap();
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER deny_card_audit BEFORE INSERT ON audit_events WHEN NEW.event IN ('card_statement_imported','card_transaction_reviewed') BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;").unwrap();
        assert!(w
            .stage_card_statement(
                &c.id,
                cards::Stage {
                    expected_revision: 0,
                    token: preview.token.clone(),
                    input: input()
                }
            )
            .is_err());
        assert!(w.get_card_statements(&c.id).unwrap().statements.is_empty());
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER deny_card_audit;")
            .unwrap();
        let s = w
            .stage_card_statement(
                &c.id,
                cards::Stage {
                    expected_revision: 0,
                    token: preview.token,
                    input: input(),
                },
            )
            .unwrap();
        let previous = json(&s).unwrap();
        let preview = w
            .inspect_client_csv(c.id.clone(), path.to_str().unwrap().into())
            .unwrap();
        assert!(w
            .stage_card_statement(
                &c.id,
                cards::Stage {
                    expected_revision: s.revision,
                    token: preview.token,
                    input: input()
                }
            )
            .err()
            .unwrap()
            .contains("already recorded"));
        assert_eq!(
            json(&w.get_card_statements(&c.id).unwrap()).unwrap(),
            previous
        );
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER deny_card_audit BEFORE INSERT ON audit_events WHEN NEW.event='card_transaction_reviewed' BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;").unwrap();
        assert!(w
            .review_card_transaction(&c.id, decision(&s, 0, Kind::Purchase, Status::Approved))
            .is_err());
        assert_eq!(
            json(&w.get_card_statements(&c.id).unwrap()).unwrap(),
            previous
        );
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER deny_card_audit;")
            .unwrap();
        w.review_card_transaction(&c.id, decision(&s, 0, Kind::Purchase, Status::Approved))
            .unwrap();
        w.session().unwrap().connection.execute_batch("DROP TRIGGER card_decisions_no_update; UPDATE card_decisions SET fields_json=json_set(fields_json,'$.amount','-100.01');").unwrap();
        assert!(w.get_card_statements(&c.id).is_err());
        let backup_root = tempfile::tempdir().unwrap();
        let destination = backup_root.path().join("corrupt-backups");
        fs::create_dir(&destination).unwrap();
        let backup = w
            .create_backup(destination.to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
    }
    #[test]
    fn card_schema_migration_rolls_back_and_retries_without_losing_prior_audit() {
        let (_root, mut w, _) = configured();
        w.save_client(client_input()).unwrap();
        let c = &mut w.session().unwrap().connection;
        let before: i64 = c
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        c.execute_batch("DROP TABLE IF EXISTS category_rule_applications; DROP TABLE IF EXISTS category_rule_versions; DROP INDEX IF EXISTS review_history_rule_scope; DROP TABLE IF EXISTS card_facts_history; DROP TABLE IF EXISTS card_review_history; DROP TABLE card_decisions; DROP TABLE card_statements; PRAGMA user_version=22; CREATE TABLE audit_events_v23(collision TEXT);").unwrap();
        assert!(migrate(c, false).is_err());
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            22
        );
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='card_statements'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        c.execute_batch("DROP TABLE audit_events_v23;").unwrap();
        migrate(c, false).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            before
        );
    }
    #[test]
    fn card_sign_mapping_unknowns_dates_adjustments_and_limits_remain_explicit() {
        let mut i = input();
        i.mapping.signed_is_debit = true;
        let csv = crate::imports::read_csv(
            b"Date,Description,Amount,Category\n2026-09-01,Card charge,-12.34,Food\n",
        )
        .unwrap();
        let raw = crate::imports::extract(&csv, &i.mapping).unwrap();
        assert_eq!(raw[0].extracted.amount.as_deref(), Some("12.34"));
        let f = Fields {
            kind: Kind::Purchase,
            ..cards::initial(&raw[0])
        };
        assert!(cards::decision(
            f.clone(),
            &Status::Approved,
            &i,
            "Verified original sign",
            true
        )
        .is_ok());
        let mut invalid = f.clone();
        invalid.date = Some("2026-10-01".into());
        assert!(cards::decision(invalid, &Status::Approved, &i, "Wrong period", true).is_err());
        let mut invalid = f;
        invalid.amount = Some("0.00".into());
        assert!(
            cards::decision(invalid, &Status::Approved, &i, "Zero is not movement", true).is_err()
        );
        i.mapping.amount = Some(usize::MAX);
        assert!(cards::canonical(i).is_err());
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let path = root.path().join("too-many.csv");
        fs::write(
            &path,
            format!(
                "Date,Description,Amount,Category\n{}",
                "2026-09-01,Charge,1.00,Food\n".repeat(201)
            ),
        )
        .unwrap();
        let p = w
            .inspect_client_csv(c.id.clone(), path.to_str().unwrap().into())
            .unwrap();
        assert!(w
            .stage_card_statement(
                &c.id,
                cards::Stage {
                    expected_revision: i64::MAX,
                    token: p.token.clone(),
                    input: input()
                }
            )
            .err()
            .unwrap()
            .contains("revision"));
        assert!(w
            .stage_card_statement(
                &c.id,
                cards::Stage {
                    expected_revision: 0,
                    token: p.token,
                    input: input()
                }
            )
            .is_err());
        assert!(w.get_card_statements(&c.id).unwrap().statements.is_empty());
    }
}
