//! Scoped immutable records, atomic audit and authenticated recovery validation.
use super::*;
use crate::assessments::{self, Directory, Input, Payload, Record, Summary};
use sha2::{Digest, Sha256};
const MAX_RECORDS: usize = 50;
const MAX_TOTAL: i64 = 32 * 1024 * 1024;
fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn limits(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<(usize, i64), String> {
    let (n,size):(i64,i64)=c.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(payload_json AS BLOB))),0) FROM client_assessment_records WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(storage_error)?;
    if !(0..=MAX_RECORDS as i64).contains(&n) || size > MAX_TOTAL {
        return Err(STORAGE_ERROR.into());
    }
    Ok((n as usize, size))
}
fn texts(p: &Payload) -> Result<(), String> {
    if required_text(&p.title, "Assessment title", 120)? != p.title
        || optional_text(&p.findings, "Counselor findings", 4000, true)? != p.findings
        || optional_text(&p.recommendations, "Recommendations", 4000, true)? != p.recommendations
        || optional_text(&p.follow_up, "Follow-up", 4000, true)? != p.follow_up
    {
        return Err(STORAGE_ERROR.into());
    }
    Ok(())
}
fn decode(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    id: &str,
) -> Result<(Payload, String), String> {
    let row:Option<(String,String,String,String,String,i64)>=c.query_row("SELECT payload_json,digest,title,created_at,period_start,source_revision FROM client_assessment_records WHERE id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![id,client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(storage_error)?;
    let (json, digest, title, created, start, revision) =
        row.ok_or("This saved assessment is unavailable for this client.")?;
    if json.len() > assessments::MAX_PAYLOAD || hash(&json) != digest {
        return Err(STORAGE_ERROR.into());
    }
    let p: Payload = serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
    if p.id != id
        || p.workspace.organization_id != w.organization_id
        || p.workspace.counselor_id != w.counselor_id
        || p.client.id != client
        || p.title != title
        || p.created_at != created
        || p.assessment.period_start != start
        || p.assessment.revision != revision
    {
        return Err(STORAGE_ERROR.into());
    }
    let end: String = c
        .query_row(
            "SELECT period_end FROM client_assessment_records WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(storage_error)?;
    if end != p.assessment.period_end {
        return Err(STORAGE_ERROR.into());
    }
    texts(&p)?;
    assessments::validate(&p).map_err(|_| STORAGE_ERROR.to_owned())?;
    Ok((p, digest))
}
pub(super) fn validate_records(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(), String> {
    limits(c, w, client)?;
    let ids=c.prepare("SELECT id FROM client_assessment_records WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|r.get::<_,String>(0)).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    for id in ids {
        decode(c, w, client, &id)?;
    }
    Ok(())
}
fn record(c: &Connection, w: &WorkspaceInfo, client: &str, id: &str) -> Result<Record, String> {
    let (payload, digest) = decode(c, w, client, id)?;
    let current_client = scoped_client(c, w, client)?;
    let revision:i64=c.query_row("SELECT revision FROM client_import_state WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).optional().map_err(storage_error)?.unwrap_or(0);
    let mut review_reasons = Vec::new();
    if revision != payload.assessment.revision {
        review_reasons.push("Imported source decisions or verified financial facts changed after this assessment was saved. Its original figures remain historical; record a new assessment for current conclusions.".into());
    }
    if current_client != payload.client {
        review_reasons.push("The saved client profile changed after this assessment. The record retains its original identity and goals.".into());
    }
    if w != &payload.workspace {
        review_reasons
            .push("Workspace identity details changed after this assessment was saved.".into());
    }
    Ok(Record {
        client_id: client.into(),
        needs_review: !review_reasons.is_empty(),
        review_reasons,
        payload,
        digest,
    })
}
impl Workspace {
    pub fn export_assessment_pdf(
        &mut self,
        client: &str,
        id: &str,
        path: &str,
    ) -> Result<crate::assessment_pdf::Export, String> {
        let r = self.get_assessment_record(client, id)?;
        crate::assessment_pdf::save(&r, path, &self.data_dir)
    }

    pub fn list_assessment_records(&mut self, client: &str) -> Result<Directory, String> {
        let s = self.session()?;
        let current = scoped_client(&s.connection, &s.info, client)?;
        limits(&s.connection, &s.info, client)?;
        // Full records are read and verified only when opened, or during backup recovery.
        let source = imports_store::read_imports(&s.connection, &s.info, client)?;
        let rows=s.connection.prepare("SELECT id,title,created_at,period_start,period_end,source_revision FROM client_assessment_records WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid DESC").map_err(storage_error)?.query_map(params![client,s.info.organization_id,s.info.counselor_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?))).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
        let mut records = Vec::new();
        for (id, title, created_at, period_start, period_end, source_revision) in rows {
            required_text(&title, "Assessment title", 120)?;
            chrono::DateTime::parse_from_rfc3339(&created_at)
                .map_err(|_| STORAGE_ERROR.to_owned())?;
            crate::imports::iso_date(&period_start)?;
            crate::imports::iso_date(&period_end)?;
            // Status derives from current context, never mutates the archived JSON.
            let profile_at_save:String=s.connection.query_row("SELECT json_extract(payload_json,'$.client.updatedAt') FROM client_assessment_records WHERE id=?1",[&id],|r|r.get(0)).map_err(storage_error)?;
            records.push(Summary {
                id,
                title,
                created_at,
                period_start,
                period_end,
                source_revision,
                needs_review: source_revision != source.revision
                    || profile_at_save != current.updated_at,
            });
        }
        Ok(Directory {
            client_id: client.into(),
            records,
        })
    }
    pub fn get_assessment_record(&mut self, client: &str, id: &str) -> Result<Record, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        limits(&s.connection, &s.info, client)?;
        record(&s.connection, &s.info, client, id)
    }
    pub fn save_assessment_record(&mut self, client: &str, input: Input) -> Result<Record, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        if !input.acknowledge {
            return Err("Confirm the approved basis, exclusions and incomplete information before saving this assessment record.".into());
        }
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let profile = scoped_client(&tx, &s.info, client)?;
        let source = imports_store::read_imports(&tx, &s.info, client)?;
        if source.revision != input.expected_revision
            || profile.updated_at != input.expected_profile_updated_at
        {
            return Err("Client or financial source data changed elsewhere. Refresh before saving; this assessment was not recorded.".into());
        }
        let created_at = timestamp();
        let calculation_date = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_err(|_| STORAGE_ERROR.to_owned())?
            .date_naive();
        let assessment = crate::dashboard::assess(
            &source,
            crate::dashboard::Input {
                from_month: input.from_month,
                through_month: input.through_month,
            },
            calculation_date,
        )?;
        let payload = Payload {
            format: assessments::FORMAT,
            calculator: assessments::CALCULATOR.into(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            id: Uuid::new_v4().to_string(),
            created_at,
            workspace: s.info.clone(),
            client: profile,
            title: required_text(&input.title, "Assessment title", 120)?,
            findings: optional_text(&input.findings, "Counselor findings", 4000, true)?,
            recommendations: optional_text(&input.recommendations, "Recommendations", 4000, true)?,
            follow_up: optional_text(&input.follow_up, "Follow-up", 4000, true)?,
            assessment,
            basis: assessments::capture(source),
        };
        assessments::validate(&payload)?;
        let json = serde_json::to_string(&payload).map_err(|_| STORAGE_ERROR.to_owned())?;
        let digest = hash(&json);
        let (n, size) = limits(&tx, &s.info, client)?;
        if json.len() > assessments::MAX_PAYLOAD
            || n >= MAX_RECORDS
            || size + json.len() as i64 > MAX_TOTAL
        {
            return Err("Saved assessments reached their local safety limit (50 records, 8 MiB per record, 32 MiB per client). Nothing was saved.".into());
        }
        tx.execute("INSERT INTO client_assessment_records(id,client_id,organization_id,counselor_id,title,created_at,period_start,period_end,source_revision,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![payload.id,client,s.info.organization_id,s.info.counselor_id,payload.title,payload.created_at,payload.assessment.period_start,payload.assessment.period_end,payload.assessment.revision,json,digest]).map_err(storage_error)?;
        audit(&tx, &s.info, "assessment_recorded", Some(&payload.id))?;
        let result = record(&tx, &s.info, client, &payload.id)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}
