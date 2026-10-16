//! Scoped read-only presentation context and no-overwrite local PDF export.
use super::*;
use crate::{assessment_pdf, report_presentation as presentation};
use sha2::{Digest, Sha256};
fn read(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    id: &str,
) -> Result<
    (
        presentation::Context,
        crate::assessments::Record,
        Option<crate::final_reports::Record>,
    ),
    String,
> {
    scoped_client(c, w, client)?;
    let a = assessments_store::record(c, w, client, id)?;
    let approval:Option<String>=c.query_row("SELECT id FROM final_report_approvals WHERE assessment_id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![id,client,w.organization_id,w.counselor_id],|r|r.get(0)).optional().map_err(storage_error)?;
    let f = approval
        .map(|id| final_reports_store::record(c, w, client, &id))
        .transpose()?;
    let mut context = presentation::Context {
        client_id: client.into(),
        assessment_id: id.into(),
        presentation: presentation::VERSION.into(),
        receipt: String::new(),
        internal: presentation::summary(&a, None)?,
        final_report: f
            .as_ref()
            .map(|f| presentation::summary(&a, Some(f)))
            .transpose()?,
    };
    context.receipt = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&context).map_err(|_| STORAGE_ERROR)?)
    );
    Ok((context, a, f))
}
impl Workspace {
    pub fn get_report_presentation(
        &mut self,
        client: &str,
        id: &str,
    ) -> Result<presentation::Context, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let (context, _, _) = read(&tx, &s.info, client, id)?;
        tx.commit().map_err(storage_error)?;
        Ok(context)
    }
    pub fn export_report_presentation(
        &mut self,
        client: &str,
        input: presentation::ExportInput,
    ) -> Result<assessment_pdf::Export, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let (context, a, f) = read(&tx, &s.info, client, &input.assessment_id)?;
        if context.receipt != input.expected_receipt {
            return Err("Report context changed. Refresh the presentation preview before exporting; no PDF was created.".into());
        }
        let selected = context.selected(input.kind)?;
        let at = timestamp();
        let (bytes, pages) = match (input.kind, input.layout) {
            (presentation::Kind::Internal, presentation::Layout::SummaryAppendix) => {
                assessment_pdf::render_presented_assessment(&a, &at)?
            }
            (presentation::Kind::Internal, presentation::Layout::Detailed) => {
                assessment_pdf::render(&a, &at)?
            }
            (presentation::Kind::Final, presentation::Layout::SummaryAppendix) => {
                assessment_pdf::render_presented_final_report(
                    f.as_ref().ok_or(STORAGE_ERROR)?,
                    &at,
                )?
            }
            (presentation::Kind::Final, presentation::Layout::Detailed) => {
                assessment_pdf::render_final_report(f.as_ref().ok_or(STORAGE_ERROR)?, &at)?
            }
        };
        let record_id = selected.record_id.clone();
        tx.commit().map_err(storage_error)?;
        assessment_pdf::save_bytes(&record_id, bytes, pages, &input.path, &self.data_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        card_final_policy_tests as card_final,
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    use presentation::{ExportInput, Kind, Layout};
    fn json(v: &impl Serialize) -> Vec<u8> {
        serde_json::to_vec(v).unwrap()
    }
    fn request(c: &presentation::Context, kind: Kind, layout: Layout, path: &Path) -> ExportInput {
        ExportInput {
            assessment_id: c.assessment_id.clone(),
            kind,
            layout,
            expected_receipt: c.receipt.clone(),
            path: path.to_string_lossy().into_owned(),
        }
    }
    fn pages(bytes: &[u8]) -> Vec<String> {
        pdf_extract::extract_text_from_mem_by_pages(bytes).unwrap()
    }
    fn compact(s: &str) -> String {
        s.split_whitespace().collect()
    }
    fn first_page(bytes: &[u8], name: &str, start: &str, amount: &str) {
        let p = pages(bytes);
        assert!(p.len() > 1);
        let first = compact(&p[0]);
        for expected in [
            name,
            start,
            "Financial summary / USD",
            amount,
            presentation::VERSION,
        ] {
            assert!(
                first.contains(&compact(expected)),
                "Missing {expected}: {}",
                p[0]
            );
        }
        assert!(!p[0].contains("Approval SHA-256"));
        assert!(!p[0].contains("Approved check /"));
        assert!(!p[0].contains("Evidence appendix / original detailed report"));
        assert!(p[1].contains("Evidence appendix / original detailed report"));
    }
    #[test]
    fn presentation_exports_v1_scope_receipts_readonly_originals_and_recovery() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let a = bank::final_candidate(&mut w, &c, root.path());
        let original = json(&a.payload);
        let outputs = tempfile::tempdir().unwrap();
        let no_final = outputs.path().join("no-final.pdf");
        let before = w.get_report_presentation(&c.id, &a.payload.id).unwrap();
        assert!(before.final_report.is_none());
        assert!(w
            .export_report_presentation(
                &c.id,
                request(&before, Kind::Final, Layout::SummaryAppendix, &no_final)
            )
            .is_err());
        assert!(!no_final.exists());
        let review = w.review_final_report(&c.id, &a.payload.id).unwrap();
        let final_report = w
            .approve_final_report(&c.id, bank::final_input(&review))
            .unwrap();
        let original_approval = json(&final_report.payload);
        let context = w.get_report_presentation(&c.id, &a.payload.id).unwrap();
        let v1 = context.final_report.as_ref().unwrap();
        assert!(v1.basis.starts_with("Bank-only"));
        assert!(!v1.needs_review);
        let stale = outputs.path().join("unreviewed.pdf");
        assert!(w
            .export_report_presentation(
                &c.id,
                request(&before, Kind::Internal, Layout::SummaryAppendix, &stale)
            )
            .err()
            .unwrap()
            .contains("context changed"));
        assert!(!stale.exists());
        let changes = w.session().unwrap().connection.total_changes();
        for (kind, layout, name) in [
            (Kind::Internal, Layout::SummaryAppendix, "internal.pdf"),
            (Kind::Final, Layout::SummaryAppendix, "final.pdf"),
            (Kind::Final, Layout::Detailed, "detailed.pdf"),
        ] {
            let path = outputs.path().join(name);
            let export = w
                .export_report_presentation(&c.id, request(&context, kind, layout, &path))
                .unwrap();
            let bytes = fs::read(&path).unwrap();
            assert!(export.pages > 1);
            if layout == Layout::SummaryAppendix {
                first_page(
                    &bytes,
                    &format!("{} {}", c.first_name, c.last_name),
                    &a.payload.assessment.period_start,
                    "799.99",
                );
            } else {
                let p = pages(&bytes);
                assert!(p[0].contains("Final approval ID"));
                assert!(!p[0].contains(presentation::VERSION));
            }
            assert!(w
                .export_report_presentation(&c.id, request(&context, kind, layout, &path))
                .is_err());
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert_eq!(w.session().unwrap().connection.total_changes(), changes);
        assert_eq!(
            json(
                &w.get_assessment_record(&c.id, &a.payload.id)
                    .unwrap()
                    .payload
            ),
            original
        );
        assert_eq!(
            json(
                &w.get_final_report(&c.id, &final_report.payload.id)
                    .unwrap()
                    .payload
            ),
            original_approval
        );
        let foreign = outputs.path().join("foreign.pdf");
        assert!(w.get_report_presentation(&other.id, &a.payload.id).is_err());
        assert!(w
            .export_report_presentation(
                &other.id,
                request(&context, Kind::Final, Layout::SummaryAppendix, &foreign)
            )
            .is_err());
        assert!(!foreign.exists());
        let protected = root.path().join("protected.pdf");
        assert!(w
            .export_report_presentation(
                &c.id,
                request(
                    &context,
                    Kind::Internal,
                    Layout::SummaryAppendix,
                    &protected
                )
            )
            .is_err());
        assert!(!protected.exists());
        let mut changed = client_input();
        changed.id = Some(c.id.clone());
        changed.first_name = "Changed current identity".into();
        w.save_client(changed).unwrap();
        let stale = outputs.path().join("stale.pdf");
        assert!(w
            .export_report_presentation(
                &c.id,
                request(&context, Kind::Final, Layout::SummaryAppendix, &stale)
            )
            .is_err());
        assert!(!stale.exists());
        let current = w.get_report_presentation(&c.id, &a.payload.id).unwrap();
        assert!(current.final_report.as_ref().unwrap().needs_review);
        assert_eq!(
            current.internal.client_name,
            format!("{} {}", c.first_name, c.last_name)
        );
        let backups = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(backups.path().to_str().unwrap().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let locked = outputs.path().join("locked.pdf");
        assert!(w.get_report_presentation(&c.id, &a.payload.id).is_err());
        assert!(w
            .export_report_presentation(
                &c.id,
                request(&current, Kind::Final, Layout::SummaryAppendix, &locked)
            )
            .is_err());
        assert!(!locked.exists());
        drop(w);
        let mut w = Workspace::new(root.path().into());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            json(&w.get_report_presentation(&c.id, &a.payload.id).unwrap()),
            json(&current)
        );
        w.lock_workspace();
        let r = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(r.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = w.get_report_presentation(&c.id, &a.payload.id).unwrap();
        assert_eq!(json(&restored), json(&current));
        let path = outputs.path().join("restored.pdf");
        w.export_report_presentation(
            &c.id,
            request(&restored, Kind::Final, Layout::SummaryAppendix, &path),
        )
        .unwrap();
        let p = pages(&fs::read(path).unwrap());
        assert!(p[0].contains("CURRENT CONTEXT NEEDS REVIEW"));
        assert!(compact(&p[0]).contains("799.99"));
        assert_eq!(
            json(
                &w.get_final_report(&c.id, &final_report.payload.id)
                    .unwrap()
                    .payload
            ),
            original_approval
        );
    }
    #[test]
    fn presentation_card_summary_keeps_exact_original_spending_and_debt_after_reversal() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let (a, legacy) = card_final::fixture(&mut w, &c, root.path(), "complete");
        let review = w.review_final_report(&c.id, &a.payload.id).unwrap();
        let approved = w
            .approve_final_report(&c.id, card_final::input(&review))
            .unwrap();
        let original = json(&approved.payload);
        let context = w.get_report_presentation(&c.id, &a.payload.id).unwrap();
        let figure = context.final_report.as_ref().unwrap();
        assert!(!figure.needs_review);
        assert!(figure.basis.starts_with("Approved bank/card"));
        let at = "2026-10-15T12:00:00Z";
        let (bytes, _) = assessment_pdf::render_presented_final_report(&approved, at).unwrap();
        first_page(
            &bytes,
            &format!("{} {}", c.first_name, c.last_name),
            &a.payload.assessment.period_start,
            "1287.10",
        );
        let first = compact(&pages(&bytes)[0]);
        for value in [
            "2000.01", "1187.09", "100.01", "1287.10", "712.91", "749.96", "50.03", "1037.05",
        ] {
            assert!(first.contains(value), "missing {value}");
        }
        assert_eq!(first.matches("1037.05").count(), 1);
        assert!(!first.contains("2074.10"));
        let (legacy_bytes, _) = assessment_pdf::render_presented_final_report(&legacy, at).unwrap();
        assert!(compact(&pages(&legacy_bytes)[0]).contains("Bank-onlyapprovedactivity"));
        assert!(!pages(&legacy_bytes)[0].contains("Combined recorded spending"));
        let cards = w.get_card_statements(&c.id).unwrap();
        let decision = crate::storage::cards_store::tests::decision(
            &cards,
            0,
            crate::cards::Kind::Purchase,
            crate::imports::Status::Rejected,
        );
        w.review_card_transaction(&c.id, decision).unwrap();
        let historical = w.get_final_report(&c.id, &approved.payload.id).unwrap();
        assert!(historical.needs_review);
        assert_eq!(json(&historical.payload), original);
        let (bytes, _) = assessment_pdf::render_presented_final_report(&historical, at).unwrap();
        assert!(pages(&bytes)[0].contains("CURRENT CONTEXT NEEDS REVIEW"));
        assert!(compact(&pages(&bytes)[0]).contains("1287.10"));
    }
    #[test]
    fn presentation_long_branding_unknowns_and_legacy_format_stay_on_first_page() {
        let (root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let mut a = bank::final_candidate(&mut w, &c, root.path());
        // A validated synthetic capture with maximal identity lengths and long
        // firm disclosure must keep the summary's key figures on page one.
        a.payload.title = "W".repeat(120);
        a.payload.client.first_name = "W".repeat(100);
        a.payload.client.last_name = "W".repeat(100);
        a.payload.workspace.organization_name = "W".repeat(120);
        let mut data = crate::branding::defaults(&a.payload.workspace.organization_name);
        data.disclosure = "W".repeat(2000);
        data.address = "W".repeat(300);
        data.phone = "5".repeat(60);
        let mut raw = vec![];
        {
            let mut e = png::Encoder::new(&mut raw, 1, 1);
            e.set_color(png::ColorType::Rgb);
            e.set_depth(png::BitDepth::Eight);
            e.write_header()
                .unwrap()
                .write_image_data(&[20, 50, 70])
                .unwrap();
        }
        data.logo_png = Some(crate::branding::inspect_logo(&raw).unwrap());
        a.needs_review = true;
        data.email = "a@example.invalid".into();
        a.payload.branding = Some(crate::branding::Snapshot {
            organization_id: a.payload.workspace.organization_id.clone(),
            revision: 1,
            updated_at: "2026-10-15T12:00:00Z".into(),
            data: crate::branding::canonical(data).unwrap(),
        });
        let (bytes, _) =
            assessment_pdf::render_presented_assessment(&a, "2026-10-15T12:00:00Z").unwrap();
        let p = pages(&bytes);
        assert!(compact(&p[0]).contains("799.99"));
        assert!(p[1].contains("Evidence appendix"));
        assert!(!p[0].contains("Firm disclosure"));
        assert!(p.concat().contains("Firm disclosure"));
        a.payload.branding = None;
        a.payload.completeness = None;
        a.payload.format = 1;
        let (bytes, _) =
            assessment_pdf::render_presented_assessment(&a, "2026-10-15T12:00:00Z").unwrap();
        assert!(compact(&pages(&bytes)[0]).contains("799.99"));
        // This fixture has no verified savings/debt balances: Unknown is retained.
        let source = w.get_client_imports(&c.id).unwrap();
        let mut no_facts = source;
        no_facts.financial_facts = crate::financial_facts::Facts::default();
        let date = chrono::DateTime::parse_from_rfc3339(&a.payload.created_at)
            .unwrap()
            .date_naive();
        a.payload.basis = crate::assessments::capture(no_facts);
        a.payload.assessment = crate::dashboard::assess(
            &a.payload.basis,
            crate::dashboard::Input {
                from_month: a.payload.assessment.period_start[..7].into(),
                through_month: a.payload.assessment.period_start[..7].into(),
            },
            date,
        )
        .unwrap();
        let summary = presentation::summary(&a, None).unwrap();
        assert!(summary
            .figures
            .iter()
            .any(|f| f.label == "Estimated net worth at balance date" && f.value.is_none()));
        let (bytes, _) =
            assessment_pdf::render_presented_assessment(&a, "2026-10-15T12:00:00Z").unwrap();
        assert!(pages(&bytes)[0].contains("Unknown"));
    }
}
