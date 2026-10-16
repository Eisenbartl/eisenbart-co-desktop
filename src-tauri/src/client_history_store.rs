use super::*;
use crate::client_history::{Amendment, Correction, Event, Kind, Page, Query, Thread};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
fn hash(value: &impl Serialize) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|_| STORAGE_ERROR)?)
    ))
}
pub(super) fn threads(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<Vec<Thread>, String> {
    scoped_client(c, w, client)?;
    let mut q=c.prepare("SELECT id,client_id,counselor_id,counselor_name,body,created_at FROM client_notes WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY created_at,id LIMIT 10001").map_err(storage_error)?;
    let notes = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok(ClientNote {
                id: r.get(0)?,
                client_id: r.get(1)?,
                counselor_id: r.get(2)?,
                counselor_name: r.get(3)?,
                body: r.get(4)?,
                created_at: r.get(5)?,
            })
        })
        .map_err(storage_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(storage_error)?;
    if notes.len() > 10000 {
        return Err("Session notes exceed the retained history limit.".into());
    }
    for n in &notes {
        if Uuid::parse_str(&n.id).is_err()
            || required_multiline_text(&n.body, "Note", 8000)? != n.body
            || required_text(&n.counselor_name, "Counselor", 120)? != n.counselor_name
            || chrono::DateTime::parse_from_rfc3339(&n.created_at).is_err()
        {
            return Err(STORAGE_ERROR.into());
        }
    }
    let by_id: HashMap<_, _> = notes.iter().map(|n| (n.id.clone(), n)).collect();
    let mut groups: BTreeMap<String, Vec<Amendment>> = BTreeMap::new();
    let mut children = HashSet::new();
    let mut q=c.prepare("SELECT root_id,revision,previous_id,note_id,reason FROM note_amendments WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY root_id,revision LIMIT 10001").map_err(storage_error)?;
    let rows = q
        .query_map(params![client, w.organization_id, w.counselor_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(storage_error)?;
    for row in rows {
        let (root, revision, previous, id, reason) = row.map_err(storage_error)?;
        let original = by_id.get(&root).ok_or(STORAGE_ERROR)?;
        let n = (*by_id.get(&id).ok_or(STORAGE_ERROR)?).clone();
        let amendments = groups.entry(root.clone()).or_default();
        let old = amendments.last().map(|a| &a.note).unwrap_or(original);
        if !(1..=100).contains(&revision)
            || revision != amendments.len() as i64 + 1
            || old.id != previous
            || root == id
            || n.body == old.body
            || chrono::DateTime::parse_from_rfc3339(&n.created_at).map_err(|_| STORAGE_ERROR)?
                < chrono::DateTime::parse_from_rfc3339(&old.created_at)
                    .map_err(|_| STORAGE_ERROR)?
            || required_multiline_text(&reason, "Correction reason", 500)? != reason
            || !children.insert(id)
        {
            return Err(STORAGE_ERROR.into());
        }
        amendments.push(Amendment {
            revision,
            previous_id: previous,
            note: n,
            reason,
        });
    }
    if groups.keys().any(|id| children.contains(id)) {
        return Err(STORAGE_ERROR.into());
    }
    let mut result = Vec::new();
    for n in notes.into_iter().filter(|n| !children.contains(&n.id)) {
        let amendments = groups.remove(&n.id).unwrap_or_default();
        result.push(Thread {
            client_id: client.into(),
            fingerprint: hash(&("note-amendment-v1", client, &n, &amendments))?,
            original: n,
            amendments,
        });
    }
    if !groups.is_empty() {
        return Err(STORAGE_ERROR.into());
    }
    result.sort_by(|a, b| {
        b.current()
            .created_at
            .cmp(&a.current().created_at)
            .then_with(|| b.current().id.cmp(&a.current().id))
    });
    Ok(result)
}
pub(super) fn current_notes(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<Vec<ClientNote>, String> {
    Ok(threads(c, w, client)?
        .iter()
        .map(|t| t.current().clone())
        .collect())
}
pub(super) fn require_current(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    ids: &[String],
) -> Result<(), String> {
    let current = current_notes(c, w, client)?;
    if ids.iter().any(|id| !current.iter().any(|n| &n.id == id)) {
        return Err("A selected note was corrected or is unavailable. Reload report evidence and select its current version.".into());
    }
    Ok(())
}
fn event(
    id: String,
    source_id: String,
    kind: Kind,
    title: String,
    detail: String,
    at: String,
) -> Result<Event, String> {
    let date = chrono::DateTime::parse_from_rfc3339(&at).map_err(|_| STORAGE_ERROR)?;
    if title.is_empty() || title.chars().count() > 255 || detail.chars().count() > 9000 {
        return Err(STORAGE_ERROR.into());
    }
    Ok(Event {
        id,
        source_id,
        kind,
        title,
        detail,
        occurred_at: date
            .with_timezone(&Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
        day: date
            .with_timezone(&chrono_tz::America::Chicago)
            .format("%Y-%m-%d")
            .to_string(),
    })
}
fn timeline(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<Vec<Event>, String> {
    let profile = scoped_client(c, w, client)?;
    let mut events = vec![event(
        "profile-created".into(),
        client.into(),
        Kind::Profile,
        "Client created".into(),
        "Original creation time; profile history before this timeline is not reconstructed.".into(),
        profile.created_at.clone(),
    )?];
    events.push(event(
        "profile-current".into(),
        client.into(),
        Kind::Profile,
        "Current profile last updated".into(),
        format!(
            "{} {} · {}. This is the latest profile timestamp, not a retained profile version.",
            profile.first_name, profile.last_name, profile.status
        ),
        profile.updated_at,
    )?);
    for t in threads(c, w, client)? {
        let original = t.original;
        events.push(event(
            format!("note:{}", original.id),
            original.id.clone(),
            Kind::Note,
            "Original session note".into(),
            format!("{}\n{}", original.counselor_name, original.body),
            original.created_at,
        )?);
        for a in t.amendments {
            events.push(event(
                format!("note:{}", a.note.id),
                a.note.id.clone(),
                Kind::Note,
                format!("Session note correction {}", a.revision),
                format!(
                    "{}\n{}\nReason: {}\nOriginal note: {}",
                    a.note.counselor_name, a.note.body, a.reason, original.id
                ),
                a.note.created_at,
            )?);
        }
    }
    // Only trusted static statements select scoped metadata, never source blobs or arbitrary SQL.
    let sources=[
        (Kind::Intake,"SELECT CAST(revision AS TEXT),'Intake and follow-ups version '||revision,updated_at FROM client_care_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY revision LIMIT 20001"),
        (Kind::Document,"SELECT id,file_name,imported_at FROM import_documents WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY id LIMIT 20001"),
        (Kind::Document,"SELECT id,json_extract(payload_json,'$.fileName'),json_extract(payload_json,'$.importedAt') FROM card_statements WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY id LIMIT 20001"),
        (Kind::Scenario,"SELECT id,label,created_at FROM client_scenarios WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY id LIMIT 20001"),
        (Kind::Report,"SELECT id,title,created_at FROM client_assessment_records WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY id LIMIT 20001"),
        (Kind::ActionPlan,"SELECT id,title,created_at FROM client_action_plans WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY id LIMIT 20001"),
    ];
    for (index, (kind, sql)) in sources.into_iter().enumerate() {
        let mut q = c.prepare(sql).map_err(storage_error)?;
        let rows = q
            .query_map(params![client, w.organization_id, w.counselor_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(storage_error)?;
        for row in rows {
            let (id, title, at) = row.map_err(storage_error)?;
            if title.chars().any(char::is_control) {
                return Err(STORAGE_ERROR.into());
            }
            events.push(event(format!("source:{index}:{id}"),id,kind,title,"Open its existing source section to inspect full evidence and history. Individual transaction decisions remain in Transactions → Activity history.".into(),at)?);
            if events.len() > 20000 {
                return Err("Client timeline exceeds the bounded 20,000-event workflow.".into());
            }
        }
    }
    events.sort_by(|a, b| {
        b.occurred_at
            .cmp(&a.occurred_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(events)
}
fn page(client: &str, events: Vec<Event>, q: Query) -> Result<Page, String> {
    let text = q.text.trim().to_lowercase();
    if text.chars().count() > 100
        || text.chars().any(char::is_control)
        || q.offset > 20000
        || !q.offset.is_multiple_of(25)
    {
        return Err("Use a literal search up to 100 characters and a valid history page.".into());
    }
    for d in [&q.from_date, &q.through_date] {
        if !d.is_empty()
            && chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map(|v| v.to_string() != *d)
                .unwrap_or(true)
        {
            return Err("Use valid history dates (YYYY-MM-DD).".into());
        }
    }
    if !q.from_date.is_empty() && !q.through_date.is_empty() && q.from_date > q.through_date {
        return Err("History start date must not follow its end date.".into());
    }
    let snapshot = hash(&("client-timeline-v1", client, &events))?;
    if q.expected_snapshot.as_ref().is_some_and(|v| v != &snapshot)
        || (q.offset > 0 && q.expected_snapshot.is_none())
    {
        return Err("Client history changed. Search again from the first page.".into());
    }
    let filtered = events
        .into_iter()
        .filter(|e| {
            q.kind.is_none_or(|k| k == e.kind)
                && (q.from_date.is_empty() || e.day >= q.from_date)
                && (q.through_date.is_empty() || e.day <= q.through_date)
                && (text.is_empty()
                    || format!("{} {} {}", e.title, e.detail, e.source_id)
                        .to_lowercase()
                        .contains(&text))
        })
        .collect::<Vec<_>>();
    Ok(Page {
        client_id: client.into(),
        snapshot,
        offset: q.offset,
        total: filtered.len(),
        events: filtered.into_iter().skip(q.offset).take(25).collect(),
    })
}
impl Workspace {
    pub fn get_note_history(&mut self, client: &str, note: &str) -> Result<Thread, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let t = threads(&tx, &s.info, client)?
            .into_iter()
            .find(|t| t.original.id == note || t.amendments.iter().any(|a| a.note.id == note))
            .ok_or("This note is unavailable for this client.")?;
        tx.commit().map_err(storage_error)?;
        Ok(t)
    }
    pub fn correct_client_note(
        &mut self,
        client: &str,
        input: Correction,
    ) -> Result<Thread, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let all = threads(&tx, &s.info, client)?;
        let total = all.iter().map(|t| 1 + t.amendments.len()).sum::<usize>();
        let old = all
            .into_iter()
            .find(|t| t.current().id == input.note_id)
            .ok_or("This note was corrected or is unavailable. Reload note history.")?;
        if old.fingerprint != input.expected_fingerprint {
            return Err(
                "Note history changed. Reload and review the current version; nothing was saved."
                    .into(),
            );
        }
        if old.amendments.len() >= 100 || total >= 10000 {
            return Err("This correction exceeds the retained note history limit.".into());
        }
        let body = required_multiline_text(&input.body, "Corrected note", 8000)?;
        let reason = required_multiline_text(&input.reason, "Correction reason", 500)?;
        if body == old.current().body {
            return Err("Change the note text before recording a correction.".into());
        }
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO client_notes(id,client_id,organization_id,counselor_id,counselor_name,body,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,client,s.info.organization_id,s.info.counselor_id,s.info.counselor_name,body,timestamp()]).map_err(storage_error)?;
        tx.execute("INSERT INTO note_amendments(root_id,revision,previous_id,note_id,client_id,organization_id,counselor_id,reason) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![old.original.id,old.amendments.len() as i64+1,old.current().id,id,client,s.info.organization_id,s.info.counselor_id,reason]).map_err(storage_error)?;
        audit(&tx, &s.info, "note_amended", Some(&old.original.id))?;
        let updated = threads(&tx, &s.info, client)?
            .into_iter()
            .find(|t| t.original.id == old.original.id)
            .ok_or(STORAGE_ERROR)?;
        tx.commit().map_err(storage_error)?;
        Ok(updated)
    }
    pub fn get_client_history(&mut self, client: &str, input: Query) -> Result<Page, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let result = page(client, timeline(&tx, &s.info, client)?, input)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    fn query() -> Query {
        Query {
            text: String::new(),
            kind: None,
            from_date: String::new(),
            through_date: String::new(),
            offset: 0,
            expected_snapshot: None,
        }
    }
    fn correction(t: &Thread, body: &str) -> Correction {
        Correction {
            note_id: t.current().id.clone(),
            expected_fingerprint: t.fingerprint.clone(),
            body: body.into(),
            reason: "Fictional original record checked and correction reviewed".into(),
        }
    }
    #[test]
    fn reviewed_note_corrections_preserve_original_reports_scope_atomicity_and_recovery() {
        let (root, mut w, info) = configured();
        let client = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let note = w
            .add_client_note(&client.id, "Fictional original session: meeting Tuesday")
            .unwrap();
        let original = w.get_note_history(&client.id, &note.id).unwrap();
        let source = bank::stage(
            &mut w,
            &client.id,
            root.path(),
            "fictional-note-source.csv",
            "Date,Description,Amount,Category,Type\n2026-09-01,Fictional wages,1200,Wages,income\n",
            bank::doc("Fictional history checking"),
        );
        let row = &source.records[0];
        let source = w
            .review_transaction(
                &client.id,
                crate::imports::ReviewInput {
                    expected_revision: source.revision,
                    record_id: row.id.clone(),
                    fields: row.fields.clone(),
                    status: crate::imports::Status::Approved,
                    reason: "Fictional source verified".into(),
                    acknowledge_warnings: true,
                },
            )
            .unwrap();
        let make_report = |count, id: String| {
            let mut input = bank::assessment_input(&source, &client);
            input.supporting = Some(crate::report_evidence::Selection {
                expected_notes_count: count,
                note_ids: vec![id],
            });
            input
        };
        let saved = w
            .save_assessment_record(&client.id, make_report(1, note.id.clone()))
            .unwrap();
        let bytes = serde_json::to_vec(&saved.payload).unwrap();
        let digest = saved.digest.clone();
        assert!(w.get_note_history(&other.id, &note.id).is_err());
        assert!(w
            .correct_client_note(&other.id, correction(&original, "Foreign correction"))
            .is_err());
        for (body, reason) in [
            ("", "Reason"),
            ("New body", ""),
            ("New body", "\u{1}"),
            (&"x".repeat(8001), "Reason"),
        ] {
            let mut input = correction(&original, body);
            input.reason = reason.into();
            assert!(w.correct_client_note(&client.id, input).is_err());
        }
        assert!(w
            .correct_client_note(&client.id, correction(&original, &note.body))
            .is_err());
        w.session().unwrap().connection.execute_batch("CREATE TEMP TRIGGER synthetic_note_audit BEFORE INSERT ON audit_events WHEN NEW.event='note_amended' BEGIN SELECT RAISE(ABORT,'Synthetic audit failure'); END;").unwrap();
        assert!(w
            .correct_client_note(
                &client.id,
                correction(&original, "Fictional corrected session: meeting Wednesday")
            )
            .is_err());
        assert_eq!(w.get_note_history(&client.id, &note.id).unwrap(), original);
        assert_eq!(w.list_client_notes(&client.id).unwrap(), vec![note.clone()]);
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER synthetic_note_audit;")
            .unwrap();
        let revised = w
            .correct_client_note(
                &client.id,
                correction(&original, "Fictional corrected session: meeting Wednesday"),
            )
            .unwrap();
        assert_eq!(revised.original, note);
        assert_eq!(revised.amendments.len(), 1);
        assert_eq!(revised.amendments[0].previous_id, note.id);
        assert_eq!(
            w.list_client_notes(&client.id).unwrap(),
            vec![revised.current().clone()]
        );
        assert!(w
            .correct_client_note(&client.id, correction(&original, "Stale correction"))
            .is_err());
        let old = w
            .get_assessment_record(&client.id, &saved.payload.id)
            .unwrap();
        assert_eq!(serde_json::to_vec(&old.payload).unwrap(), bytes);
        assert_eq!(old.digest, digest);
        assert!(old.needs_review);
        assert!(w.list_assessment_records(&client.id).unwrap().records[0].needs_review);
        let preview = w.get_assessment_evidence(&client.id).unwrap();
        assert_eq!(preview.notes_count, 2);
        assert_eq!(preview.notes, vec![revised.current().clone()]);
        assert!(w
            .save_assessment_record(&client.id, make_report(1, note.id.clone()))
            .is_err());
        assert!(w
            .save_assessment_record(&client.id, make_report(2, note.id.clone()))
            .is_err());
        let new = w
            .save_assessment_record(&client.id, make_report(2, revised.current().id.clone()))
            .unwrap();
        assert_eq!(
            new.payload.supporting.as_ref().unwrap().notes[0],
            *revised.current()
        );
        let reverted = w
            .correct_client_note(&client.id, correction(&revised, &note.body))
            .unwrap();
        assert_eq!(reverted.amendments.len(), 2);
        assert_eq!(reverted.current().body, note.body);
        assert_ne!(reverted.current().id, note.id);
        assert!(
            w.get_assessment_record(&client.id, &saved.payload.id)
                .unwrap()
                .needs_review
        );
        assert!(
            w.get_assessment_record(&client.id, &new.payload.id)
                .unwrap()
                .needs_review
        );
        assert_eq!(
            serde_json::to_vec(&w.get_client_imports(&client.id).unwrap()).unwrap(),
            serde_json::to_vec(&source).unwrap()
        );
        let c = &w.session().unwrap().connection;
        assert!(c
            .execute("UPDATE note_amendments SET reason='Silent change'", [])
            .is_err());
        assert!(c.execute("DELETE FROM note_amendments", []).is_err());
        assert!(c.execute("DELETE FROM client_notes", []).is_err());
        let backups = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(backups.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        drop(w);
        let mut w = Workspace::new(root.path().to_path_buf());
        assert!(w.get_note_history(&client.id, &note.id).is_err());
        assert_eq!(w.unlock_workspace(PASSWORD.into()).unwrap(), info);
        assert_eq!(w.get_note_history(&client.id, &note.id).unwrap(), reverted);
        w.lock_workspace();
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(w.get_note_history(&client.id, &note.id).unwrap(), reverted);
        assert_eq!(
            w.get_assessment_record(&client.id, &saved.payload.id)
                .unwrap()
                .digest,
            digest
        );
        w.session().unwrap().connection.execute_batch("DROP TRIGGER note_amendments_no_update; UPDATE note_amendments SET reason='' WHERE revision=1;").unwrap();
        let backup = w
            .create_backup(backups.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
    }
    #[test]
    fn timeline_pages_literal_dates_stale_scope_and_reads_are_non_writing() {
        let (_root, mut w, info) = configured();
        let client = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let mut ids = Vec::new();
        for i in 0..31 {
            let id = Uuid::from_u128(i + 1).to_string();
            ids.push(id.clone());
            w.session().unwrap().connection.execute("INSERT INTO client_notes(id,client_id,organization_id,counselor_id,counselor_name,body,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,client.id,info.organization_id,info.counselor_id,info.counselor_name,format!("Fictional timeline {i:02} 50%_O'Brien"),if i==0{"2026-10-16T04:30:00Z"}else{"2026-10-16T05:30:00Z"}]).unwrap();
        }
        let before: i64 = w
            .session()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        let mut q = query();
        q.kind = Some(Kind::Note);
        q.text = "50%_O'Brien".into();
        let first = w.get_client_history(&client.id, q.clone()).unwrap();
        assert_eq!(first.total, 31);
        assert_eq!(first.events.len(), 25);
        assert_eq!(first.events[0].source_id, ids[30]);
        assert_eq!(first.events[0].day, "2026-10-16");
        q.offset = 25;
        q.expected_snapshot = Some(first.snapshot.clone());
        let second = w.get_client_history(&client.id, q.clone()).unwrap();
        assert_eq!(second.events.len(), 6);
        assert_eq!(second.events.last().unwrap().source_id, ids[0]);
        assert_eq!(second.events.last().unwrap().day, "2026-10-15");
        assert!(!first
            .events
            .iter()
            .any(|e| second.events.iter().any(|x| x.id == e.id)));
        q.offset = 0;
        q.expected_snapshot = None;
        q.from_date = "2026-10-15".into();
        q.through_date = q.from_date.clone();
        assert_eq!(
            w.get_client_history(&client.id, q.clone()).unwrap().total,
            1
        );
        q.from_date = "2026-02-30".into();
        assert!(w.get_client_history(&client.id, q.clone()).is_err());
        q.from_date = "2026-10-16".into();
        assert!(w.get_client_history(&client.id, q.clone()).is_err());
        assert_eq!(w.get_client_history(&other.id, query()).unwrap().total, 2);
        assert!(w
            .get_client_history(&Uuid::new_v4().to_string(), query())
            .is_err());
        let after: i64 = w
            .session()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after);
        let mut stale = query();
        stale.offset = 25;
        assert!(w.get_client_history(&client.id, stale.clone()).is_err());
        stale.expected_snapshot = Some(first.snapshot);
        w.add_client_note(&client.id, "Fictional later note")
            .unwrap();
        assert!(w.get_client_history(&client.id, stale).is_err());
        w.session.as_mut().unwrap().info.counselor_id = Uuid::new_v4().to_string();
        assert!(w.get_client_history(&client.id, query()).is_err());
        w.session.as_mut().unwrap().info = info;
        w.lock_workspace();
        assert!(w.get_client_history(&client.id, query()).is_err());
    }
    #[test]
    fn schema_26_migration_rollback_preserves_notes_and_existing_rules() {
        let (root, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let source = bank::stage(&mut w, &client.id, root.path(), "fictional-migration-rule.csv", "Date,Description,Amount,Category,Type\n2026-09-01,Fictional legacy market,-12.03,,expense\n", bank::doc("Fictional legacy rule account"));
        let directory = w.get_category_rules(&client.id).unwrap();
        w.save_category_rule(
            &client.id,
            crate::category_rules::Input {
                id: None,
                expected_revision: 0,
                expected_fingerprint: directory.fingerprint,
                data: crate::category_rules::Data {
                    label: "Fictional retained rule".into(),
                    account_id: source.accounts[0].id.clone(),
                    kind: crate::imports::Kind::Expense,
                    mode: crate::category_rules::Mode::Description,
                    pattern: "Fictional legacy market".into(),
                    category: "Groceries".into(),
                    archived: false,
                },
                reason: "Fictional rule before note migration".into(),
            },
        )
        .unwrap();
        let n = w
            .add_client_note(&client.id, "Fictional note before migration")
            .unwrap();
        let rules = serde_json::to_vec(&w.get_category_rules(&client.id).unwrap()).unwrap();
        let c = &mut w.session.as_mut().unwrap().connection;
        c.execute_batch("DROP TABLE IF EXISTS workspace_security_history; DROP TABLE note_amendments; DROP INDEX note_amendment_scope; PRAGMA user_version=26; CREATE TABLE audit_events_v27(collision TEXT);").unwrap();
        assert!(migrate(c, false).is_err());
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            26
        );
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='note_amendments'",
                [],
                |r| r.get::<_, i32>(0)
            )
            .unwrap(),
            0
        );
        c.execute_batch("DROP TABLE audit_events_v27;").unwrap();
        migrate(c, false).unwrap();
        assert_eq!(w.list_client_notes(&client.id).unwrap(), vec![n]);
        assert_eq!(
            serde_json::to_vec(&w.get_category_rules(&client.id).unwrap()).unwrap(),
            rules
        );
    }
}
