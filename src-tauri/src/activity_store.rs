//! Read-only client-wide transaction timeline, derived from bounded existing evidence.
use super::*;
use crate::{
    cards,
    imports::{self, Raw, Status},
};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
const PAGE_SIZE: usize = 25;
#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Query {
    pub text: String,
    pub from: Option<String>,
    pub through: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub account_id: Option<String>,
    pub status: Option<Status>,
    pub offset: usize,
    pub snapshot: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub kind: String,
    pub file_name: String,
    pub sha256: String,
    pub original_account: String,
    pub current_account: String,
    pub account_id: Option<String>,
    pub archived: bool,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub retained: bool,
}
#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Fields {
    pub date: Option<String>,
    pub amount: Option<String>,
    pub description: String,
    pub merchant: String,
    pub category: String,
    pub kind: String,
}
fn bank_fields(f: &imports::Fields) -> Fields {
    Fields {
        date: f.date.clone(),
        amount: f.amount.clone(),
        description: f.description.clone(),
        merchant: f.merchant.clone(),
        category: f.category.clone(),
        kind: serde_json::to_value(&f.kind)
            .expect("enum serialization")
            .as_str()
            .unwrap()
            .into(),
    }
}
fn card_fields(f: &cards::Fields) -> Fields {
    Fields {
        date: f.date.clone(),
        amount: f.amount.clone(),
        description: f.description.clone(),
        merchant: String::new(),
        category: f.category.clone(),
        kind: serde_json::to_value(&f.kind)
            .expect("enum serialization")
            .as_str()
            .unwrap()
            .into(),
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub kind: String,
    pub source_id: String,
    pub record_id: String,
    pub decision_id: Option<String>,
    pub current: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub source_id: String,
    pub record_id: String,
    pub row: Option<usize>,
    pub decision_id: Option<String>,
    pub occurred_at: String,
    pub counselor_name: Option<String>,
    pub from_status: Option<Status>,
    pub status: Status,
    pub current_status: Status,
    pub latest: bool,
    pub fields: Fields,
    pub reason: String,
    pub raw: Raw,
    pub links: Vec<Link>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub client_id: String,
    pub snapshot: String,
    pub query: Query,
    pub total: usize,
    pub page_size: usize,
    pub sources: Vec<Source>,
    pub events: Vec<Event>,
}
// Keep references while filtering and sorting; copy original evidence only for the returned page.
struct Candidate<'a> {
    id: String,
    source: &'a str,
    record: String,
    row: Option<usize>,
    decision: Option<&'a str>,
    at: &'a str,
    counselor: Option<&'a str>,
    from: Option<Status>,
    status: Status,
    current: Status,
    latest: bool,
    fields: Fields,
    reason: &'a str,
    raw: &'a Raw,
}
impl Workspace {
    pub fn query_transaction_activity(
        &mut self,
        client: &str,
        mut q: Query,
    ) -> Result<Page, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        q.text = cards::text(&q.text, 200, false)?;
        for d in [&q.from, &q.through].into_iter().flatten() {
            imports::iso_date(d)?;
        }
        if q.from
            .as_ref()
            .zip(q.through.as_ref())
            .is_some_and(|(a, b)| a > b)
            || q.offset > 23000
            || !q.offset.is_multiple_of(PAGE_SIZE)
            || q.source_kind
                .as_deref()
                .is_some_and(|k| !matches!(k, "bank" | "card"))
            || q.account_id
                .as_ref()
                .is_some_and(|v| Uuid::parse_str(v).is_err())
            || q.source_id
                .as_ref()
                .is_some_and(|v| Uuid::parse_str(v).is_err())
            || q.snapshot
                .as_ref()
                .is_some_and(|v| v.len() != 64 || !v.bytes().all(|b| b.is_ascii_hexdigit()))
            || (q.offset > 0 && q.snapshot.is_none())
        {
            return Err("Use valid activity filters and restart the search before paging.".into());
        }
        let bank = imports_store::read_imports(&tx, &s.info, client)?;
        let cards = cards_store::read(&tx, &s.info, client)?;
        let account_labels = statement_accounts_store::read(&tx, &s.info, client)?;
        let links_history = card_review_store::history(&tx, &s.info, client)?;
        let links = links_history.last();
        let mut sources = Vec::new();
        for d in &bank.documents {
            let a = account_labels
                .accounts
                .iter()
                .find(|a| a.id == d.account_id)
                .ok_or(STORAGE_ERROR)?;
            sources.push(Source {
                id: d.id.clone(),
                kind: "bank".into(),
                file_name: d.file_name.clone(),
                sha256: d.sha256.clone(),
                original_account: d.input.account_label.clone(),
                current_account: a.label.clone(),
                account_id: Some(a.id.clone()),
                archived: a.archived,
                period_start: d.input.period_start.clone(),
                period_end: d.input.period_end.clone(),
                retained: d.source_retained,
            });
        }
        for c in &cards.statements {
            let a = links.and_then(|l| {
                l.assignments
                    .iter()
                    .find(|a| a.statement_id == c.source.id)
                    .and_then(|a| l.accounts.iter().find(|b| b.id == a.account_id))
            });
            sources.push(Source {
                id: c.source.id.clone(),
                kind: "card".into(),
                file_name: c.source.file_name.clone(),
                sha256: c.source.sha256.clone(),
                original_account: c.source.input.account_label.clone(),
                current_account: a
                    .map_or_else(|| c.source.input.account_label.clone(), |a| a.label.clone()),
                account_id: a.map(|a| a.id.clone()),
                archived: a.is_some_and(|a| a.archived),
                period_start: Some(c.source.input.period_start.clone()),
                period_end: Some(c.source.input.period_end.clone()),
                retained: c.source.input.retain_source,
            });
        }
        let token = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(client, bank.revision, links.map(|l| l.revision), &sources))
                    .map_err(|_| STORAGE_ERROR)?
            )
        );
        if q.snapshot.as_ref().is_some_and(|v| v != &token) {
            return Err("Activity changed since this search. Refresh activity to restart at the first page.".into());
        }
        // Index histories once, including an orphan check: partial history must never look complete.
        let records = bank
            .records
            .iter()
            .map(|r| (r.id.as_str(), r))
            .collect::<HashMap<_, _>>();
        let mut history: HashMap<&str, Vec<&imports::History>> = HashMap::new();
        let mut ids = HashSet::new();
        for h in &bank.history {
            if !records.contains_key(h.record_id.as_str()) || !ids.insert(h.id.as_str()) {
                return Err(STORAGE_ERROR.into());
            }
            history.entry(&h.record_id).or_default().push(h);
        }
        let mut all = Vec::new();
        for r in &bank.records {
            let hs = history.get(r.id.as_str()).map(Vec::as_slice).unwrap_or(&[]);
            let d = bank
                .documents
                .iter()
                .find(|d| d.id == r.document_id)
                .ok_or(STORAGE_ERROR)?;
            let initial = hs
                .first()
                .map_or_else(|| r.status.clone(), |h| h.from_status.clone());
            all.push(Candidate {
                id: format!("bank-import:{}", r.id),
                source: &d.id,
                record: r.id.clone(),
                row: None,
                decision: None,
                at: &d.imported_at,
                counselor: None,
                from: None,
                status: initial.clone(),
                current: r.status.clone(),
                latest: hs.is_empty(),
                fields: bank_fields(&r.raw.extracted),
                reason: "Imported source evidence",
                raw: &r.raw,
            });
            let mut previous = initial;
            for h in hs {
                if h.from_status != previous {
                    return Err(STORAGE_ERROR.into());
                }
                previous = h.to_status.clone();
                all.push(Candidate {
                    id: format!("bank-decision:{}", h.id),
                    source: &d.id,
                    record: r.id.clone(),
                    row: None,
                    decision: Some(&h.id),
                    at: &h.reviewed_at,
                    counselor: Some(&h.counselor_name),
                    from: Some(h.from_status.clone()),
                    status: h.to_status.clone(),
                    current: r.status.clone(),
                    latest: hs.last().is_some_and(|last| last.id == h.id),
                    fields: bank_fields(&h.fields),
                    reason: &h.reason,
                    raw: &r.raw,
                });
            }
            if let Some(h) = hs.last() {
                if h.to_status != r.status || h.fields != r.fields {
                    return Err(STORAGE_ERROR.into());
                }
            }
        }
        for c in &cards.statements {
            for r in &c.rows {
                let hs = c
                    .history
                    .iter()
                    .filter(|h| h.row == r.row)
                    .collect::<Vec<_>>();
                all.push(Candidate {
                    id: format!("card-import:{}:{}", c.source.id, r.row),
                    source: &c.source.id,
                    record: format!("{}:{}", c.source.id, r.row),
                    row: Some(r.row),
                    decision: None,
                    at: &c.source.imported_at,
                    counselor: None,
                    from: None,
                    status: Status::Pending,
                    current: r.status.clone(),
                    latest: hs.is_empty(),
                    fields: card_fields(&cards::initial(&r.raw)),
                    reason: "Imported source evidence",
                    raw: &r.raw,
                });
                let mut previous = Status::Pending;
                for h in hs.iter() {
                    all.push(Candidate {
                        id: format!("card-decision:{}", h.id),
                        source: &c.source.id,
                        record: format!("{}:{}", c.source.id, r.row),
                        row: Some(r.row),
                        decision: Some(&h.id),
                        at: &h.reviewed_at,
                        counselor: Some(&h.counselor_name),
                        from: Some(previous),
                        status: h.status.clone(),
                        current: r.status.clone(),
                        latest: hs.last().is_some_and(|last| last.id == h.id),
                        fields: card_fields(&h.fields),
                        reason: &h.reason,
                        raw: &r.raw,
                    });
                    previous = h.status.clone();
                }
            }
        }
        let source_map = sources
            .iter()
            .map(|s| (s.id.as_str(), s))
            .collect::<HashMap<_, _>>();
        let needle = q.text.to_lowercase();
        let mut matched = Vec::new();
        for e in all {
            let time = chrono::DateTime::parse_from_rfc3339(e.at).map_err(|_| STORAGE_ERROR)?;
            let source = source_map.get(e.source).ok_or(STORAGE_ERROR)?;
            if q.account_id
                .as_ref()
                .is_some_and(|a| source.account_id.as_ref() != Some(a))
                || q.source_kind.as_ref().is_some_and(|k| k != &source.kind)
                || q.source_id.as_ref().is_some_and(|id| id != e.source)
                || q.status.as_ref().is_some_and(|s| s != &e.status)
                || q.from
                    .as_ref()
                    .is_some_and(|d| e.fields.date.as_ref().is_none_or(|v| v < d))
                || q.through
                    .as_ref()
                    .is_some_and(|d| e.fields.date.as_ref().is_none_or(|v| v > d))
            {
                continue;
            }
            let text = format!(
                "{} {} {} {} {} {} {} {} {} {} {}",
                e.fields.description,
                e.fields.merchant,
                e.fields.category,
                e.fields.kind,
                e.reason,
                source.file_name,
                source.original_account,
                source.current_account,
                e.record,
                e.decision.unwrap_or(""),
                e.raw.reported_description
            );
            if !needle.is_empty() && !text.to_lowercase().contains(&needle) {
                continue;
            }
            matched.push((time, e));
        }
        matched.sort_by(|(a, x), (b, y)| b.cmp(a).then_with(|| x.id.cmp(&y.id)));
        let total = matched.len();
        if q.offset > total {
            return Err("This activity page is unavailable. Refresh activity to restart.".into());
        }
        let mut events = Vec::new();
        for (_, e) in matched.into_iter().skip(q.offset).take(PAGE_SIZE) {
            let mut context = Vec::new();
            if let Some(r) = records.get(e.record.as_str()) {
                if let Some(id) = &r.linked_id {
                    let other = records.get(id.as_str()).ok_or(STORAGE_ERROR)?;
                    if other.linked_id.as_deref() != Some(r.id.as_str()) {
                        return Err(STORAGE_ERROR.into());
                    }
                    context.push(Link {
                        kind: "Current bank transfer counterpart".into(),
                        source_id: other.document_id.clone(),
                        record_id: other.id.clone(),
                        decision_id: history
                            .get(other.id.as_str())
                            .and_then(|h| h.last())
                            .map(|h| h.id.clone()),
                        current: true,
                    });
                }
            }
            if let Some(l) = links {
                for p in &l.repayments {
                    let is_bank = p.bank_record_id == e.record;
                    let is_card = p.statement_id == e.source && Some(p.row) == e.row;
                    if !is_bank && !is_card {
                        continue;
                    }
                    let bank_record = records
                        .get(p.bank_record_id.as_str())
                        .ok_or(STORAGE_ERROR)?;
                    context.push(Link {
                        kind: "Latest recorded repayment link".into(),
                        source_id: if is_bank {
                            p.statement_id.clone()
                        } else {
                            bank_record.document_id.clone()
                        },
                        record_id: if is_bank {
                            format!("{}:{}", p.statement_id, p.row)
                        } else {
                            p.bank_record_id.clone()
                        },
                        decision_id: Some(if is_bank {
                            p.card_decision_id.clone()
                        } else {
                            p.bank_decision_id.clone()
                        }),
                        current: crate::card_review::current(p, &bank, &cards),
                    });
                }
            }
            events.push(Event {
                id: e.id,
                source_id: e.source.into(),
                record_id: e.record,
                row: e.row,
                decision_id: e.decision.map(str::to_owned),
                occurred_at: e.at.into(),
                counselor_name: e.counselor.map(str::to_owned),
                from_status: e.from,
                status: e.status,
                current_status: e.current,
                latest: e.latest,
                fields: e.fields,
                reason: e.reason.into(),
                raw: e.raw.clone(),
                links: context,
            });
        }
        tx.commit().map_err(storage_error)?;
        q.snapshot = Some(token.clone());
        Ok(Page {
            client_id: client.into(),
            snapshot: token,
            query: q,
            total,
            page_size: PAGE_SIZE,
            sources,
            events,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        cards_store::tests as card,
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    fn encoded(v: &impl Serialize) -> String {
        serde_json::to_string(v).unwrap()
    }
    fn review(w: &mut Workspace, id: &str, record: &str, status: Status, amount: &str) {
        let s = w.get_client_imports(id).unwrap();
        let mut fields = s
            .records
            .iter()
            .find(|r| r.id == record)
            .unwrap()
            .fields
            .clone();
        fields.amount = Some(amount.into());
        fields.description = "Corrected fictional purchase".into();
        w.review_transaction(
            id,
            imports::ReviewInput {
                expected_revision: s.revision,
                record_id: record.into(),
                status,
                fields,
                reason: "Inspected original fictional evidence".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
    }
    #[test]
    fn mixed_history_filters_pagination_corrections_scope_and_recovery_are_read_only() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let mut csv = "Date,Description,Amount,Category,Type\n".to_owned();
        for i in 0..31 {
            csv.push_str(&format!(
                "2026-09-02,Fictional original {i},-1.01,Food,expense\n"
            ));
        }
        let b = bank::stage(
            &mut w,
            &c.id,
            dir.path(),
            "history.csv",
            &csv,
            bank::doc("Original checking"),
        );
        let id = b.records[0].id.clone();
        review(&mut w, &c.id, &id, Status::Approved, "-1.02");
        review(&mut w, &c.id, &id, Status::Rejected, "-1.02");
        let mut cs = card::stage(&mut w, &c.id, dir.path());
        cs = w
            .review_card_transaction(
                &c.id,
                card::decision(&cs, 0, cards::Kind::Purchase, Status::Approved),
            )
            .unwrap();
        let mut correction = card::decision(&cs, 0, cards::Kind::Purchase, Status::Approved);
        correction.fields.amount = Some("100.02".into());
        cs = w.review_card_transaction(&c.id, correction).unwrap();
        w.review_card_transaction(
            &c.id,
            card::decision(&cs, 0, cards::Kind::Purchase, Status::Rejected),
        )
        .unwrap();
        let revision = w.get_client_imports(&c.id).unwrap().revision;
        let changes = w.session().unwrap().connection.total_changes();
        let first = w
            .query_transaction_activity(&c.id, Query::default())
            .unwrap();
        assert_eq!(first.total, 41);
        assert_eq!(first.events.len(), 25);
        assert_eq!(
            encoded(&first),
            encoded(
                &w.query_transaction_activity(&c.id, Query::default())
                    .unwrap()
            )
        );
        let mut next = first.query.clone();
        next.offset = 25;
        let second = w.query_transaction_activity(&c.id, next).unwrap();
        assert_eq!(second.events.len(), 16);
        let ids = first
            .events
            .iter()
            .chain(&second.events)
            .map(|e| &e.id)
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), 41);
        // All 31 imports share the same timestamp. Tie ordering is stable across the page boundary.
        let imports = first
            .events
            .iter()
            .chain(&second.events)
            .filter(|e| e.id.starts_with("bank-import"))
            .collect::<Vec<_>>();
        assert_eq!(imports.len(), 31);
        assert!(imports.windows(2).all(|p| p[0].id < p[1].id));
        let filtered = w
            .query_transaction_activity(
                &c.id,
                Query {
                    source_kind: Some("bank".into()),
                    status: Some(Status::Approved),
                    from: Some("2026-09-02".into()),
                    through: Some("2026-09-02".into()),
                    text: "FICTIONAL ORIGINAL 0".into(),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(filtered.total, 1);
        let e = &filtered.events[0];
        assert!(!e.latest);
        assert_eq!(e.current_status, Status::Rejected);
        assert_eq!(e.fields.amount.as_deref(), Some("-1.02"));
        assert_eq!(e.raw.reported_amount.as_deref(), Some("-1.01"));
        assert!(e.decision_id.is_some());
        let cq = w
            .query_transaction_activity(
                &c.id,
                Query {
                    source_kind: Some("card".into()),
                    status: Some(Status::Approved),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(cq.total, 2);
        assert_eq!(cq.events[0].fields.amount.as_deref(), Some("100.02"));
        assert_eq!(cq.events[0].raw.reported_amount.as_deref(), Some("100.01"));
        assert_eq!(
            w.query_transaction_activity(
                &c.id,
                Query {
                    account_id: Some(b.accounts[0].id.clone()),
                    ..Query::default()
                }
            )
            .unwrap()
            .total,
            33
        );
        assert_eq!(
            w.query_transaction_activity(
                &c.id,
                Query {
                    text: "%_".into(),
                    ..Query::default()
                }
            )
            .unwrap()
            .total,
            0
        );
        assert_eq!(
            w.query_transaction_activity(
                &other.id,
                Query {
                    source_id: Some(b.documents[0].id.clone()),
                    ..Query::default()
                }
            )
            .unwrap()
            .total,
            0
        );
        assert!(w
            .query_transaction_activity(&Uuid::new_v4().to_string(), Query::default())
            .is_err());
        assert_eq!(w.session().unwrap().connection.total_changes(), changes);
        assert_eq!(w.get_client_imports(&c.id).unwrap().revision, revision);
        let original = encoded(&first);
        drop(w);
        let mut w = Workspace::new(dir.path().into());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            encoded(
                &w.query_transaction_activity(&c.id, Query::default())
                    .unwrap()
            ),
            original
        );
        let backups = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w
            .query_transaction_activity(&c.id, Query::default())
            .is_err());
        let r = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(r.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            encoded(
                &w.query_transaction_activity(&c.id, Query::default())
                    .unwrap()
            ),
            original
        );
        review(&mut w, &c.id, &id, Status::Approved, "-1.01");
        assert!(w
            .query_transaction_activity(&c.id, first.query)
            .err()
            .unwrap()
            .contains("Activity changed"));
    }
    #[test]
    fn invalid_queries_and_unreadable_history_fail_closed() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        for q in [
            Query {
                from: Some("2026-02-30".into()),
                ..Query::default()
            },
            Query {
                from: Some("2026-09-02".into()),
                through: Some("2026-09-01".into()),
                ..Query::default()
            },
            Query {
                source_kind: Some("loan".into()),
                ..Query::default()
            },
            Query {
                offset: 25,
                ..Query::default()
            },
            Query {
                offset: 1,
                ..Query::default()
            },
            Query {
                text: "a".repeat(201),
                ..Query::default()
            },
            Query {
                snapshot: Some("bad".into()),
                ..Query::default()
            },
        ] {
            assert!(w.query_transaction_activity(&c.id, q).is_err());
        }
        let b=bank::stage(&mut w,&c.id,dir.path(),"invalid-date.csv","Date,Description,Amount,Category,Type\nunknown,Fictional unknown date,-1,Food,expense\n",bank::doc("Unknown date checking"));
        assert_eq!(
            w.query_transaction_activity(&c.id, Query::default())
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            w.query_transaction_activity(
                &c.id,
                Query {
                    from: Some("2026-01-01".into()),
                    ..Query::default()
                }
            )
            .unwrap()
            .total,
            0
        );
        let connection = &w.session().unwrap().connection;
        // Corruption is injected only into this disposable synthetic database.
        connection
            .execute_batch("DROP TRIGGER transaction_choices_no_update")
            .ok();
        connection
            .execute(
                "UPDATE transaction_choices SET fields_json='unreadable' WHERE record_id=?1",
                [&b.records[0].id],
            )
            .unwrap();
        assert!(w
            .query_transaction_activity(&c.id, Query::default())
            .is_err());
    }
    #[test]
    fn transfer_legs_repayment_bindings_and_account_renames_remain_explicit() {
        use crate::card_review::{Action, Input};
        fn change(w: &mut Workspace, id: &str, action: Action) {
            let d = w.get_card_review(id).unwrap();
            w.save_card_review(
                id,
                Input {
                    expected_revision: d.snapshot.as_ref().map_or(0, |s| s.revision),
                    expected_source_revision: d.source_revision,
                    reason: "Inspected fictional relationship".into(),
                    acknowledge: true,
                    action,
                },
            )
            .unwrap();
        }
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let b=bank::stage(&mut w,&c.id,dir.path(),"outgoing.csv","Date,Description,Amount,Category,Type\n2026-09-03,Fictional transfer out,-10,Savings,transfer\n2026-09-03,Fictional repayment,-50.03,Card,balanceMovement\n",bank::doc("Checking"));
        let left = b.records[0].id.clone();
        let repayment = b.records[1].id.clone();
        let b=bank::stage(&mut w,&c.id,dir.path(),"incoming.csv","Date,Description,Amount,Category,Type\n2026-09-03,Fictional transfer in,10,Savings,transfer\n",bank::doc("Savings"));
        let right = b.records[2].id.clone();
        w.pair_transfer_transactions(
            &c.id,
            imports::PairInput {
                expected_revision: b.revision,
                left_id: left.clone(),
                right_id: right.clone(),
                reason: "Inspected fictional equal opposite legs".into(),
            },
        )
        .unwrap();
        let b = w.get_client_imports(&c.id).unwrap();
        let row = b.records.iter().find(|r| r.id == repayment).unwrap();
        w.review_transaction(
            &c.id,
            imports::ReviewInput {
                expected_revision: b.revision,
                record_id: repayment.clone(),
                status: Status::Approved,
                fields: row.fields.clone(),
                reason: "Verified fictional repayment".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
        let cs = card::stage(&mut w, &c.id, dir.path());
        let cs = w
            .review_card_transaction(
                &c.id,
                card::decision(&cs, 2, cards::Kind::Payment, Status::Approved),
            )
            .unwrap();
        let statement_id = cs.statements[0].source.id.clone();
        change(
            &mut w,
            &c.id,
            Action::CreateAccount {
                label: "Original durable card".into(),
            },
        );
        let account = w.get_card_review(&c.id).unwrap().snapshot.unwrap().accounts[0]
            .id
            .clone();
        change(
            &mut w,
            &c.id,
            Action::AssignStatement {
                statement_id: statement_id.clone(),
                account_id: Some(account.clone()),
            },
        );
        change(
            &mut w,
            &c.id,
            Action::LinkRepayment {
                statement_id: statement_id.clone(),
                row: 2,
                bank_record_id: repayment.clone(),
            },
        );
        let all = w
            .query_transaction_activity(&c.id, Query::default())
            .unwrap();
        let l = all
            .events
            .iter()
            .find(|e| e.record_id == left && e.latest)
            .unwrap();
        assert_eq!(l.links[0].record_id, right);
        let r = all
            .events
            .iter()
            .find(|e| e.record_id == right && e.latest)
            .unwrap();
        assert_eq!(r.links[0].record_id, left);
        for e in all
            .events
            .iter()
            .filter(|e| e.record_id == repayment || e.row == Some(2))
        {
            assert_eq!(e.links.len(), 1);
            assert!(e.links[0].current);
            assert!(e.links[0].decision_id.is_some());
        }
        change(
            &mut w,
            &c.id,
            Action::UpdateAccount {
                id: account.clone(),
                label: "Renamed archived card".into(),
                archived: true,
            },
        );
        assert!(w
            .query_transaction_activity(&c.id, all.query)
            .err()
            .unwrap()
            .contains("Activity changed"));
        let p = w
            .query_transaction_activity(
                &c.id,
                Query {
                    account_id: Some(account),
                    ..Query::default()
                },
            )
            .unwrap();
        assert_eq!(p.total, 6);
        assert!(p
            .sources
            .iter()
            .any(|s| s.current_account == "Renamed archived card" && s.archived));
        let cs = w.get_card_statements(&c.id).unwrap();
        w.review_card_transaction(
            &c.id,
            card::decision(&cs, 2, cards::Kind::Payment, Status::Rejected),
        )
        .unwrap();
        let stale = w
            .query_transaction_activity(
                &c.id,
                Query {
                    source_id: Some(statement_id),
                    ..Query::default()
                },
            )
            .unwrap();
        assert!(stale
            .events
            .iter()
            .filter(|e| e.row == Some(2))
            .all(|e| !e.links[0].current));
        // Authenticated ownership still gates a valid client identifier.
        let counselor = w.session().unwrap().info.counselor_id.clone();
        w.session().unwrap().info.counselor_id = Uuid::new_v4().to_string();
        assert!(w
            .query_transaction_activity(&c.id, Query::default())
            .is_err());
        w.session().unwrap().info.counselor_id = counselor;
    }
}
