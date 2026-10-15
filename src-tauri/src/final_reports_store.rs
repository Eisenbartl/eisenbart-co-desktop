//! Atomic, scoped counselor approvals bound to immutable assessment digests.
use super::*;
use crate::final_reports::{self as final_report, Input, Payload, Preview, Record};
use sha2::{Digest, Sha256};
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn limits(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<(), String> {
    let(n,bytes):(i64,i64)=c.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(payload_json AS BLOB))),0) FROM final_report_approvals WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(storage_error)?;
    if n > 50 || bytes > 50 * final_report::MAX_BYTES as i64 {
        return Err("Final reports exceed the current safety limit.".into());
    }
    Ok(())
}
fn preview(c: &Connection, w: &WorkspaceInfo, client: &str, id: &str) -> Result<Preview, String> {
    let a = assessments_store::record(c, w, client, id)?;
    let today = Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive();
    let readiness = final_report::readiness(&a.payload, &a.digest, a.needs_review, today)?;
    let receipt = hash(&serde_json::to_string(&readiness).map_err(|_| STORAGE_ERROR.to_owned())?);
    let approval_id=c.query_row("SELECT id FROM final_report_approvals WHERE assessment_id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![id,client,w.organization_id,w.counselor_id],|r|r.get(0)).optional().map_err(storage_error)?;
    Ok(Preview {
        readiness,
        receipt,
        approval_id,
    })
}
fn record(c: &Connection, w: &WorkspaceInfo, client: &str, id: &str) -> Result<Record, String> {
    limits(c, w, client)?;
    let row:Option<(String,String,String,String)>=c.query_row("SELECT payload_json,digest,assessment_id,approved_at FROM final_report_approvals WHERE id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![id,client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(storage_error)?;
    let (json, digest, parent, at) =
        row.ok_or("This final report is unavailable for this client.")?;
    if json.len() > final_report::MAX_BYTES || hash(&json) != digest {
        return Err(STORAGE_ERROR.into());
    }
    let payload: Payload = serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
    let assessment = assessments_store::record(c, w, client, &parent)?;
    final_report::validate(&payload, &assessment).map_err(|_| STORAGE_ERROR.to_owned())?;
    if payload.id != id || payload.client_id != client || payload.approved_at != at {
        return Err(STORAGE_ERROR.into());
    }
    let mut reasons = assessment.review_reasons.clone();
    let today = Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive();
    if (today - crate::imports::iso_date(&assessment.payload.assessment.period_end)?).num_days()
        > 90
    {
        reasons.push("The approved period is now more than 90 days old. Original approval remains historical; review current finances before reuse.".into());
    }
    Ok(Record {
        client_id: client.into(),
        needs_review: !reasons.is_empty(),
        review_reasons: reasons,
        payload,
        digest,
        assessment,
    })
}
pub(super) fn validate_records(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(), String> {
    limits(c, w, client)?;
    let ids=c.prepare("SELECT id FROM final_report_approvals WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid LIMIT 51").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|r.get::<_,String>(0)).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    for id in ids {
        record(c, w, client, &id)?;
    }
    Ok(())
}
impl Workspace {
    pub fn review_final_report(&mut self, client: &str, id: &str) -> Result<Preview, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let result = preview(&tx, &s.info, client, id)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn get_final_report(&mut self, client: &str, id: &str) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let result = record(&tx, &s.info, client, id)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn approve_final_report(&mut self, client: &str, input: Input) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        limits(&tx, &s.info, client)?;
        let current = preview(&tx, &s.info, client, &input.assessment_id)?;
        if current.approval_id.is_some() {
            return Err("This assessment already has a final approval. Open its original report; record a new assessment for a revised approval.".into());
        }
        if current.readiness.assessment_digest != input.expected_digest
            || current.receipt != input.expected_readiness
        {
            return Err("Readiness changed after review. Your notes are retained; refresh and review all confirmations before approving.".into());
        }
        let decision = final_report::canonical(input.decision, &current.readiness)?;
        let assessment = assessments_store::record(&tx, &s.info, client, &input.assessment_id)?;
        let at = timestamp();
        let date = chrono::DateTime::parse_from_rfc3339(&at)
            .map_err(|_| STORAGE_ERROR.to_owned())?
            .with_timezone(&chrono_tz::America::Chicago)
            .date_naive()
            .to_string();
        if date != current.readiness.reviewed_on {
            return Err("The local report date changed during approval. Refresh readiness and confirm again; nothing was saved.".into());
        }
        let payload = Payload {
            format: 1,
            policy: final_report::POLICY.into(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            id: Uuid::new_v4().to_string(),
            client_id: client.into(),
            assessment_id: input.assessment_id,
            assessment_digest: input.expected_digest,
            approved_at: at,
            report_date: date,
            approved_by: s.info.clone(),
            readiness: current.readiness,
            decision,
        };
        final_report::validate(&payload, &assessment)?;
        let json = serde_json::to_string(&payload).map_err(|_| STORAGE_ERROR.to_owned())?;
        let count:i64=tx.query_row("SELECT COUNT(*) FROM final_report_approvals WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,s.info.organization_id,s.info.counselor_id],|r|r.get(0)).map_err(storage_error)?;
        if count >= 50 || json.len() > final_report::MAX_BYTES {
            return Err("Final approval exceeds the 50-record/64-KiB safety limit.".into());
        }
        let digest = hash(&json);
        let candidate = Record {
            client_id: client.into(),
            payload,
            digest: digest.clone(),
            assessment,
            needs_review: false,
            review_reasons: vec![],
        };
        crate::assessment_pdf::render_final_report(&candidate, &candidate.payload.approved_at)?;
        let payload = &candidate.payload;
        tx.execute("INSERT INTO final_report_approvals(id,client_id,organization_id,counselor_id,assessment_id,approved_at,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![payload.id,client,s.info.organization_id,s.info.counselor_id,payload.assessment_id,payload.approved_at,json,digest]).map_err(storage_error)?;
        audit(&tx, &s.info, "final_report_approved", Some(&payload.id))?;
        let result = record(&tx, &s.info, client, &payload.id)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn export_final_report_pdf(
        &mut self,
        client: &str,
        id: &str,
        path: &str,
    ) -> Result<crate::assessment_pdf::Export, String> {
        let r = self.get_final_report(client, id)?;
        let (bytes, pages) = crate::assessment_pdf::render_final_report(&r, &timestamp())?;
        crate::assessment_pdf::save_bytes(&r.payload.id, bytes, pages, path, &self.data_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::{client_input, configured};
    #[test]
    fn final_report_migration_failure_rolls_back_without_changing_prior_history() {
        let (_root, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let s = w.session().unwrap();
        let before: i64 = s
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        s.connection.execute_batch("DROP TABLE final_report_approvals; DROP INDEX assessment_final_identity; PRAGMA user_version=21; CREATE TABLE audit_events_v22(collision TEXT);").unwrap();
        assert!(migrate(&mut s.connection, false).is_err());
        assert_eq!(
            s.connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                .unwrap(),
            21
        );
        assert_eq!(
            s.connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name='final_report_approvals'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        s.connection
            .execute_batch("DROP TABLE audit_events_v22;")
            .unwrap();
        migrate(&mut s.connection, false).unwrap();
        assert_eq!(
            s.connection
                .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            before
        );
        assert_eq!(
            scoped_client(&s.connection, &s.info, &c.id).unwrap().id,
            c.id
        );
        assert!(preview(&s.connection, &s.info, &c.id, "wrong-assessment").is_err());
    }
}
