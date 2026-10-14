//! Scoped append-only client guidance, with immutable supporting assessment links.
use super::*;
use crate::action_plans::{self, Directory, Input, Payload, Record, Review, Summary};
use sha2::{Digest, Sha256};
fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn limits(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<(), String> {
    let (n,size):(i64,i64)=c.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(payload_json AS BLOB))),0) FROM client_action_plans WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(storage_error)?;
    if !(0..=50).contains(&n) || size > 4 * 1024 * 1024 {
        return Err(STORAGE_ERROR.into());
    }
    Ok(())
}
fn record(c: &Connection, w: &WorkspaceInfo, client: &str, id: &str) -> Result<Record, String> {
    limits(c, w, client)?;
    let row:Option<(String,String,String,String,String,String)>=c.query_row("SELECT payload_json,digest,assessment_id,title,created_at,report_date FROM client_action_plans WHERE id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![id,client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(storage_error)?;
    let (json, digest, basis, title, created, date) =
        row.ok_or("This client action plan is unavailable for this client.")?;
    if json.len() > action_plans::MAX_BYTES || hash(&json) != digest {
        return Err(STORAGE_ERROR.into());
    }
    let payload: Payload = serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
    if payload.id != id
        || payload.client_id != client
        || payload.assessment_id != basis
        || payload.title != title
        || payload.created_at != created
        || payload.report_date != date
    {
        return Err(STORAGE_ERROR.into());
    }
    let assessment = assessments_store::record(c, w, client, &basis)?;
    action_plans::validate(&payload, &assessment).map_err(|_| STORAGE_ERROR.to_owned())?;
    Ok(Record {
        client_id: client.into(),
        needs_review: assessment.needs_review,
        review_reasons: assessment.review_reasons.clone(),
        payload,
        assessment,
        digest,
    })
}
pub(super) fn validate_records(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(), String> {
    limits(c, w, client)?;
    let ids=c.prepare("SELECT id FROM client_action_plans WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|r.get::<_,String>(0)).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
    for id in ids {
        record(c, w, client, &id)?;
    }
    Ok(())
}
impl Workspace {
    pub fn review_client_action_plan(
        &mut self,
        client: &str,
        assessment_id: &str,
    ) -> Result<Review, String> {
        let a = self.get_assessment_record(client, assessment_id)?;
        action_plans::review(&a, &timestamp())
    }
    pub fn get_client_action_plan(&mut self, client: &str, id: &str) -> Result<Record, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        record(&s.connection, &s.info, client, id)
    }
    pub fn list_client_action_plans(&mut self, client: &str) -> Result<Directory, String> {
        let s = self.session()?;
        scoped_client(&s.connection, &s.info, client)?;
        limits(&s.connection, &s.info, client)?;
        let ids=s.connection.prepare("SELECT id FROM client_action_plans WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid DESC").map_err(storage_error)?.query_map(params![client,s.info.organization_id,s.info.counselor_id],|r|r.get::<_,String>(0)).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)?;
        let mut records = vec![];
        for id in ids {
            let r = record(&s.connection, &s.info, client, &id)?;
            records.push(Summary {
                id: r.payload.id,
                title: r.payload.title,
                created_at: r.payload.created_at,
                report_date: r.payload.report_date,
                assessment_id: r.payload.assessment_id,
                needs_review: r.needs_review,
            });
        }
        Ok(Directory {
            client_id: client.into(),
            records,
        })
    }
    pub fn save_client_action_plan(
        &mut self,
        client: &str,
        input: Input,
    ) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        limits(&tx, &s.info, client)?;
        let a = assessments_store::record(&tx, &s.info, client, &input.assessment_id)?;
        let p = action_plans::capture(&a, input, Uuid::new_v4().to_string(), timestamp())?;
        let json = serde_json::to_string(&p).map_err(|_| STORAGE_ERROR.to_owned())?;
        let count:i64=tx.query_row("SELECT COUNT(*) FROM client_action_plans WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,s.info.organization_id,s.info.counselor_id],|r|r.get(0)).map_err(storage_error)?;
        if json.len() > action_plans::MAX_BYTES || count >= 50 {
            return Err("Client action plans reached their safety limit (50 records, 64 KiB per plan). Nothing was recorded.".into());
        }
        let digest = hash(&json);
        tx.execute("INSERT INTO client_action_plans(id,client_id,organization_id,counselor_id,assessment_id,title,created_at,report_date,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![p.id,client,s.info.organization_id,s.info.counselor_id,p.assessment_id,p.title,p.created_at,p.report_date,json,digest]).map_err(storage_error)?;
        audit(&tx, &s.info, "action_plan_recorded", Some(&p.id))?;
        let result = record(&tx, &s.info, client, &p.id)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn export_client_action_plan(
        &mut self,
        client: &str,
        id: &str,
        path: &str,
    ) -> Result<crate::assessment_pdf::Export, String> {
        let r = self.get_client_action_plan(client, id)?;
        let (bytes, pages) = crate::assessment_pdf::render_action_plan(&r)?;
        crate::assessment_pdf::save_bytes(&r.payload.id, bytes, pages, path, &self.data_dir)
    }
}
