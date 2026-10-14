//! Scoped immutable scenarios, atomic audit, and report/recovery provenance.
use super::*;
use crate::saved_scenarios::{self, Directory, Input, Payload, Record, Selection, Summary};
use sha2::{Digest, Sha256};
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn limits(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<(u32, i64), String> {
    let (n,size):(i64,i64)=c.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(payload_json AS BLOB))),0) FROM client_scenarios WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(storage_error)?;
    if !(0..=50).contains(&n) || size > 32 * 1024 * 1024 {
        return Err(STORAGE_ERROR.into());
    }
    Ok((n as u32, size))
}
pub(super) fn record(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    id: &str,
) -> Result<Record, String> {
    limits(c, w, client)?;
    let row:Option<(String,String,String,String)>=c.query_row("SELECT payload_json,digest,label,created_at FROM client_scenarios WHERE id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![id,client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(storage_error)?;
    let (json, digest, label, created) =
        row.ok_or("This saved scenario is unavailable for this client.")?;
    if json.len() > saved_scenarios::MAX_PAYLOAD || hash(&json) != digest {
        return Err(STORAGE_ERROR.into());
    }
    let p: Payload = serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
    if p.id != id
        || p.workspace.organization_id != w.organization_id
        || p.workspace.counselor_id != w.counselor_id
        || p.comparison.input.label != label
        || p.comparison.created_at != created
    {
        return Err(STORAGE_ERROR.into());
    }
    saved_scenarios::validate(&p, client).map_err(|_| STORAGE_ERROR.to_owned())?;
    let mut reasons = vec![];
    if p.comparison.revisions != assessments_store::plan_revisions(c, w, client)? {
        reasons.push("Saved plan versions changed after this scenario. Its original baseline and proposal remain historical; load a fresh baseline for a new scenario.".into());
    }
    if &p.workspace != w {
        reasons.push("Workspace identity details changed since this scenario was saved.".into());
    }
    Ok(Record {
        client_id: client.into(),
        payload: p,
        digest,
        needs_review: !reasons.is_empty(),
        review_reasons: reasons,
    })
}
pub(super) fn validate_records(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<(), String> {
    limits(c, w, client)?;
    let ids = ids(c, w, client)?;
    for id in ids {
        record(c, w, client, &id)?;
    }
    Ok(())
}
fn ids(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<Vec<String>, String> {
    c.prepare("SELECT id FROM client_scenarios WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3 ORDER BY rowid DESC").map_err(storage_error)?.query_map(params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)
}
pub(super) fn select(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    selection: &Selection,
) -> Result<Record, String> {
    let r = record(c, w, client, &selection.id)?;
    if r.digest != selection.digest {
        return Err("The saved scenario changed or does not match the reviewed selection.".into());
    }
    if r.needs_review {
        return Err("This scenario's baseline needs review. Load the current saved plans and record a fresh scenario before selecting it for a new report.".into());
    }
    Ok(r)
}
pub(super) fn validate_link(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    p: &crate::assessments::Payload,
) -> Result<Option<Record>, String> {
    let link:Option<(String,String)>=c.query_row("SELECT scenario_id,digest FROM assessment_scenario_links WHERE assessment_id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![p.id,client,w.organization_id,w.counselor_id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage_error)?;
    match (&p.selected_scenario, link) {
        (None, None) => Ok(None),
        (Some(selection), Some((id, digest)))
            if selection.id == id && selection.digest == digest =>
        {
            let r = record(c, w, client, &id)?;
            if r.digest != digest
                || serde_json::to_value(saved_scenarios::planning(&r.payload)?)
                    .map_err(|_| STORAGE_ERROR.to_owned())?
                    != serde_json::to_value(&p.planning).map_err(|_| STORAGE_ERROR.to_owned())?
            {
                return Err(STORAGE_ERROR.into());
            }
            Ok(Some(r))
        }
        _ => Err(STORAGE_ERROR.into()),
    }
}
impl Workspace {
    pub fn list_client_scenarios(&mut self, client: &str) -> Result<Directory, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        limits(&tx, &s.info, client)?;
        let mut records = vec![];
        for id in ids(&tx, &s.info, client)? {
            let r = record(&tx, &s.info, client, &id)?;
            records.push(Summary {
                id: r.payload.id,
                label: r.payload.comparison.input.label,
                created_at: r.payload.comparison.created_at,
                digest: r.digest,
                needs_review: r.needs_review,
            });
        }
        tx.commit().map_err(storage_error)?;
        Ok(Directory {
            client_id: client.into(),
            records,
        })
    }
    pub fn get_client_scenario(&mut self, client: &str, id: &str) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let r = record(&tx, &s.info, client, id)?;
        tx.commit().map_err(storage_error)?;
        Ok(r)
    }
    pub fn save_client_scenario(&mut self, client: &str, input: Input) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let (count, size) = limits(&tx, &s.info, client)?;
        if count != input.expected_count {
            return Err("Scenario history changed since review. Reload saved scenario history and check for an existing save before recording again; nothing was saved.".into());
        }
        let p = saved_scenarios::capture(
            assessments_store::read_plans(&tx, &s.info, client)?,
            input.proposal,
            s.info.clone(),
            Uuid::new_v4().to_string(),
            timestamp(),
        )?;
        let json = serde_json::to_string(&p).map_err(|_| STORAGE_ERROR.to_owned())?;
        if count >= 50
            || json.len() > saved_scenarios::MAX_PAYLOAD
            || size + json.len() as i64 > 32 * 1024 * 1024
        {
            return Err("Saved scenarios reached their safety limit (50 records, 8 MiB per record, 32 MiB per client). Nothing was saved.".into());
        }
        let digest = hash(&json);
        tx.execute("INSERT INTO client_scenarios(id,client_id,organization_id,counselor_id,label,created_at,payload_json,digest) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![p.id,client,s.info.organization_id,s.info.counselor_id,p.comparison.input.label,p.comparison.created_at,json,digest]).map_err(storage_error)?;
        audit(&tx, &s.info, "scenario_recorded", Some(&p.id))?;
        let result = record(&tx, &s.info, client, &p.id)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
#[path = "saved_scenarios_tests.rs"]
mod tests;
