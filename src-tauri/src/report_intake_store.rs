//! Intake provenance in immutable reports and permanent selected-context review.
use super::*;
use crate::client_care::{self, Data, Intake};
use crate::report_intake::{self, Preview, Snapshot};

fn version(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    revision: i64,
) -> Result<Preview, String> {
    let row:Option<(String,String)>=c.query_row("SELECT json_extract(data_json,'$.intake'),updated_at FROM client_care_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 AND revision=?4",params![client,w.organization_id,w.counselor_id,revision],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage_error)?;
    let (json, updated) = row.ok_or_else(|| STORAGE_ERROR.to_owned())?;
    let values: Intake = serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
    if client_care::canonical(Data {
        intake: values.clone(),
        follow_ups: vec![],
    })
    .map_err(|_| STORAGE_ERROR.to_owned())?
    .intake
        != values
        || chrono::DateTime::parse_from_rfc3339(&updated).is_err()
    {
        return Err(STORAGE_ERROR.into());
    }
    Ok(Preview {
        client_id: client.into(),
        revision,
        updated_at: Some(updated),
        values,
    })
}
pub(super) fn current(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<Preview, String> {
    let revision:i64=c.query_row("SELECT COALESCE(MAX(revision),0) FROM client_care_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?;
    if revision == 0 {
        return Ok(Preview {
            client_id: client.into(),
            revision: 0,
            updated_at: None,
            values: Intake::default(),
        });
    }
    if !(1..=500).contains(&revision) {
        return Err(STORAGE_ERROR.into());
    }
    version(c, w, client, revision)
}
pub(super) fn validate_source(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    s: &Snapshot,
) -> Result<(), String> {
    report_intake::validate(s, client).map_err(|_| STORAGE_ERROR.to_owned())?;
    let original = version(c, w, client, s.revision)?;
    if original.updated_at.as_deref() != Some(s.updated_at.as_str())
        || report_intake::selected(&original.values, &s.sections) != s.values
    {
        return Err(STORAGE_ERROR.into());
    }
    Ok(())
}
pub(super) fn changed_since(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    s: &Snapshot,
) -> Result<bool, String> {
    let mut q=c.prepare("SELECT revision FROM client_care_snapshots WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 AND revision>?4 ORDER BY revision LIMIT 501").map_err(storage_error)?;
    let rows = q
        .query_map(
            params![client, w.organization_id, w.counselor_id, s.revision],
            |r| r.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    for (i, row) in rows.enumerate() {
        let revision = row.map_err(storage_error)?;
        if revision != s.revision + i as i64 + 1 || revision > 500 {
            return Err(STORAGE_ERROR.into());
        }
        let p = version(c, w, client, revision)?;
        if report_intake::selected(&p.values, &s.sections) != s.values {
            return Ok(true);
        }
    }
    Ok(false)
}
impl Workspace {
    pub fn get_assessment_intake(&mut self, client: &str) -> Result<Preview, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        current(&s.connection, &s.info, client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report_intake::Selection;
    use crate::storage::tests::{client_input, configured, PASSWORD};
    fn care() -> Data {
        let mut d = Data::default();
        d.intake.address = "PRIVATE_MAILING_ADDRESS_789".into();
        d.intake.household = "Two fictional adults".into();
        d.intake.employment = "employed".into();
        d.intake.employer = "Original fictional employer".into();
        d.intake.income_notes = "Unverified freelance context; confirm statements".into();
        d.follow_ups.push(crate::client_care::FollowUp {
            id: "".into(),
            kind: "task".into(),
            title: "Fictional follow-up".into(),
            due_date: "2026-11-01".into(),
            time: "".into(),
            status: "open".into(),
            notes: "PRIVATE_FOLLOW_UP_OUTCOME_789".into(),
        });
        d
    }
    fn report(c: &Client, intake: Option<Selection>) -> crate::assessments::Input {
        crate::assessments::Input {
            expected_branding_revision: None,
            intake,
            supporting: None,
            selected_scenario: None,
            planning: None,
            expected_revision: 0,
            expected_profile_updated_at: c.updated_at.clone(),
            from_month: "2026-09".into(),
            through_month: "2026-09".into(),
            title: "Internal intake context".into(),
            findings: "Fictional assessment".into(),
            recommendations: "Verify financial evidence separately".into(),
            follow_up: "Arrange a fictional review".into(),
            acknowledge: true,
        }
    }
    fn selection(revision: i64) -> Option<Selection> {
        Some(Selection {
            expected_revision: revision,
            sections: vec!["employment".into(), "household".into()],
        })
    }
    fn save(w: &mut Workspace, c: &str, revision: i64, data: Data) -> Data {
        w.save_client_care(
            c,
            crate::client_care::Input {
                expected_revision: revision,
                data,
            },
        )
        .unwrap()
        .data
    }
    #[test]
    fn report_intake_preserves_selected_sources_legacy_bytes_pdf_and_recovery() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        assert_eq!(w.get_assessment_intake(&c.id).unwrap().revision, 0);
        assert!(w.get_assessment_intake("foreign").is_err());
        assert!(w
            .save_assessment_record(&c.id, report(&c, selection(0)))
            .is_err());
        let legacy = w.save_assessment_record(&c.id, report(&c, None)).unwrap();
        let legacy_bytes = serde_json::to_vec(&legacy.payload).unwrap();
        assert!(!String::from_utf8_lossy(&legacy_bytes).contains("\"intake\""));
        let original = save(&mut w, &c.id, 0, care());
        assert!(w
            .save_assessment_record(&c.id, report(&c, selection(0)))
            .err()
            .unwrap()
            .contains("Intake changed"));
        let r = w
            .save_assessment_record(&c.id, report(&c, selection(1)))
            .unwrap();
        let id = r.payload.id.clone();
        let digest = r.digest.clone();
        assert_eq!(r.payload.format, 5);
        let captured = r.payload.intake.as_ref().unwrap();
        assert_eq!(captured.sections, vec!["household", "employment"]);
        assert!(captured.values.address.is_empty());
        let bytes = serde_json::to_vec(&r.payload).unwrap();
        let (pdf, _) = crate::assessment_pdf::render(&r, &timestamp()).unwrap();
        let doc = lopdf::Document::load_mem(&pdf).unwrap();
        let text = doc
            .extract_text(&doc.get_pages().keys().copied().collect::<Vec<_>>())
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("Selected intake context"));
        assert!(text.contains("Original fictional employer"));
        assert!(text.contains("Two fictional adults"));
        assert!(text.contains("not verified financial facts"));
        assert!(!text.contains("PRIVATE_MAILING_ADDRESS_789"));
        assert!(!text.contains("PRIVATE_FOLLOW_UP_OUTCOME_789"));
        let mut unrelated = original.clone();
        unrelated.intake.address = "Later unselected address".into();
        unrelated.follow_ups[0].notes = "Later unrelated task outcome".into();
        let unchanged = save(&mut w, &c.id, 1, unrelated);
        assert!(!w.get_assessment_record(&c.id, &id).unwrap().needs_review);
        let mut changed = unchanged.clone();
        changed.intake.employer = "Later fictional employer".into();
        save(&mut w, &c.id, 2, changed);
        assert!(w.get_assessment_record(&c.id, &id).unwrap().needs_review);
        save(&mut w, &c.id, 3, unchanged.clone());
        let historical = w.get_assessment_record(&c.id, &id).unwrap();
        assert!(historical.needs_review);
        assert_eq!(historical.digest, digest);
        assert_eq!(serde_json::to_vec(&historical.payload).unwrap(), bytes);
        assert_eq!(
            serde_json::to_vec(
                &w.get_assessment_record(&c.id, &legacy.payload.id)
                    .unwrap()
                    .payload
            )
            .unwrap(),
            legacy_bytes
        );
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let mut later = unchanged;
        later.intake.household = "Later household".into();
        save(&mut w, &c.id, 4, later);
        w.lock_workspace();
        assert!(w
            .get_assessment_intake(&c.id)
            .err()
            .unwrap()
            .contains("locked"));
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(w.get_client_care(&c.id).unwrap().revision, 4);
        drop(w);
        let mut w = Workspace::new(dir.path().to_path_buf());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_assessment_record(&c.id, &id).unwrap();
        assert!(restored.needs_review);
        assert_eq!(serde_json::to_vec(&restored.payload).unwrap(), bytes);
    }
    #[test]
    fn report_intake_rejects_forged_rehashed_context_and_audit_failure_before_restore() {
        use sha2::{Digest, Sha256};
        let (_dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        save(&mut w, &c.id, 0, care());
        w.session().unwrap().connection.execute_batch("CREATE TRIGGER reject_report_audit BEFORE INSERT ON audit_events WHEN NEW.event='assessment_recorded' BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;").unwrap();
        assert!(w
            .save_assessment_record(&c.id, report(&c, selection(1)))
            .is_err());
        assert!(w.list_assessment_records(&c.id).unwrap().records.is_empty());
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER reject_report_audit;")
            .unwrap();
        let mut r = w
            .save_assessment_record(&c.id, report(&c, selection(1)))
            .unwrap();
        r.payload.intake.as_mut().unwrap().values.employer = "Forged employer".into();
        let json = serde_json::to_string(&r.payload).unwrap();
        let hash = format!("{:x}", Sha256::digest(json.as_bytes()));
        w.session()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER assessment_records_no_update;")
            .unwrap();
        w.session()
            .unwrap()
            .connection
            .execute(
                "UPDATE client_assessment_records SET payload_json=?1,digest=?2 WHERE id=?3",
                params![json, hash, r.payload.id],
            )
            .unwrap();
        assert!(w.get_assessment_record(&c.id, &r.payload.id).is_err());
        let out = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(out.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        assert!(!w.data_dir.join("workspace-active.json").exists());
    }
}
