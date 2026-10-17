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
fn notes_count(c: &Connection, w: &WorkspaceInfo, client: &str) -> Result<u32, String> {
    let count:i64=c.query_row("SELECT COUNT(*) FROM client_notes WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).map_err(storage_error)?;
    if !(0..=10_000).contains(&count) {
        return Err("Session notes exceed the report evidence safety limit.".into());
    }
    Ok(count as u32)
}
fn note(c: &Connection, w: &WorkspaceInfo, client: &str, id: &str) -> Result<ClientNote, String> {
    c.query_row("SELECT id,client_id,counselor_id,counselor_name,body,created_at FROM client_notes WHERE id=?1 AND client_id=?2 AND organization_id=?3 AND counselor_id=?4",params![id,client,w.organization_id,w.counselor_id],|r|Ok(ClientNote{id:r.get(0)?,client_id:r.get(1)?,counselor_id:r.get(2)?,counselor_name:r.get(3)?,body:r.get(4)?,created_at:r.get(5)?})).optional().map_err(storage_error)?.ok_or("A selected session note is unavailable for this client.".into())
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
    scenarios_store::validate_link(c, w, client, &p)?;
    if let Some(i) = &p.intake {
        report_intake_store::validate_source(c, w, client, i)?;
    }
    if let Some(review) = &p.completeness {
        coverage_review_store::validate_source(c, w, client, review)?;
    }
    if let Some(branding) = &p.branding {
        branding_store::validate_source(c, w, branding)?;
    }
    if let Some(cards) = &p.cards {
        card_review_store::validate_capture(c, w, client, cards)?;
    }
    if let Some(s) = &p.card_facts {
        card_facts_store::validate_capture(c, w, client, s)?;
    }
    texts(&p)?;
    assessments::validate(&p).map_err(|_| STORAGE_ERROR.to_owned())?;
    if let Some(e) = &p.supporting {
        if e.notes_count > notes_count(c, w, client)? {
            return Err(STORAGE_ERROR.into());
        }
        for n in &e.notes {
            if note(c, w, client, &n.id)? != *n {
                return Err(STORAGE_ERROR.into());
            }
        }
    }
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
pub(super) fn plan_revisions(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<crate::planning::Revisions, String> {
    let get = |table: &str| -> Result<i64, String> {
        c.query_row(&format!("SELECT revision FROM {table} WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3"),params![client,w.organization_id,w.counselor_id],|r|r.get(0)).optional().map_err(storage_error).map(|v|v.unwrap_or(0))
    };
    Ok(crate::planning::Revisions {
        budget: get("client_budgets")?,
        debt: get("client_debt_plans")?,
        savings: get("client_savings_plans")?,
        cashflow: get("client_cashflow_plans")?,
    })
}
pub(super) fn read_plans(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
) -> Result<crate::planning::Plans, String> {
    Ok(crate::planning::Plans {
        client_id: client.into(),
        budget: read_budget(c, w, client)?,
        debt: read_debt_plan(c, w, client)?,
        savings: read_savings_plan(c, w, client)?,
        cashflow: read_cashflow_plan(c, w, client)?,
    })
}
pub(super) fn record(
    c: &Connection,
    w: &WorkspaceInfo,
    client: &str,
    id: &str,
) -> Result<Record, String> {
    let (payload, digest) = decode(c, w, client, id)?;
    let current_client = scoped_client(c, w, client)?;
    let revision:i64=c.query_row("SELECT revision FROM client_import_state WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",params![client,w.organization_id,w.counselor_id],|r|r.get(0)).optional().map_err(storage_error)?.unwrap_or(0);
    let mut review_reasons = Vec::new();
    if revision != payload.assessment.revision {
        review_reasons.push("Imported source decisions or verified financial facts changed after this assessment was saved. Its original figures remain historical; record a new assessment for current conclusions.".into());
    }
    if let Some(review) = &payload.completeness {
        if coverage_review_store::revision(c, w, client)? != review.revision {
            review_reasons.push("A newer completeness review was recorded. This assessment retains its original reviewed inventory, source selections, missing information and exceptions.".into());
        }
    }
    if current_client != payload.client {
        review_reasons.push("The saved client profile changed after this assessment. The record retains its original identity and goals.".into());
    }
    if branding_store::revision(c, w)? != payload.branding.as_ref().map_or(0, |b| b.revision) {
        review_reasons.push("Firm branding changed after this assessment was saved. Its captured firm presentation is preserved, including after names or contact details are reverted.".into());
    }
    if w != &payload.workspace {
        review_reasons
            .push("Workspace identity details changed after this assessment was saved.".into());
    }
    if let Some(i) = &payload.intake {
        if report_intake_store::changed_since(c, w, client, i)? {
            review_reasons.push("Selected intake information changed after this report was saved. The original selected context remains historical, including after values are reverted.".into());
        }
    }
    if let Some(e) = &payload.supporting {
        if notes_count(c, w, client)? != e.notes_count {
            review_reasons.push("Session notes were added after this report. Its selected original notes remain historical; review the later counseling context before relying on it.".into());
        }
    }
    if let Some(p) = &payload.planning {
        if plan_revisions(c, w, client)? != p.revisions {
            review_reasons.push("Saved budget, debt, savings or dated cash-flow plans changed after this scenario was captured. Its original assumptions and projections remain historical.".into());
        }
    }
    if payload.format == 9 {
        let current_links = card_review_store::history(c, w, client)?.pop();
        if payload.cards.as_ref().is_none_or(|capture| {
            current_links
                .as_ref()
                .is_none_or(|links| links.revision != capture.links.revision)
        }) {
            review_reasons.push("Card accounts, assignments or repayment links changed after this assessment. Its original evidence remains historical.".into());
        }
    } else if payload.cards.is_some() || cards_store::has_cards(c, w, client)? {
        if payload.cards.is_some() {
            review_reasons.push("A bank/card review supplement is captured below. This older supplement remains provisional. For supported card-inclusive final approval, record a new assessment with reviewed card completeness. Bank dashboard and planning figures retain their original basis.".into());
        } else {
            review_reasons.push("Credit-card evidence is stored separately and is not included in this bank-based assessment. Review and capture the bank/card supplement and card completeness in a new assessment before requesting card-inclusive final approval.".into());
        }
    }
    if let Some(s) = &payload.card_facts {
        if card_facts_store::revision(c, w, client)? != s.revision {
            review_reasons.push("A later card completeness review exists. The captured inventory, spending roles and balance comparisons remain historical.".into());
        }
    }
    let scenario = scenarios_store::validate_link(c, w, client, &payload)?;
    if let Some(r) = &scenario {
        review_reasons.extend(r.review_reasons.clone());
    }
    Ok(Record {
        validation: crate::completeness::report(&payload, !review_reasons.is_empty())?,
        scenario,
        client_id: client.into(),
        needs_review: !review_reasons.is_empty(),
        review_reasons,
        payload,
        digest,
    })
}
impl Workspace {
    pub fn get_assessment_evidence(
        &mut self,
        client: &str,
    ) -> Result<crate::report_evidence::Preview, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let notes_count = notes_count(&tx, &s.info, client)?;
        let source = imports_store::read_imports(&tx, &s.info, client)?;
        let notes = client_history_store::current_notes(&tx, &s.info, client)?
            .into_iter()
            .take(100)
            .collect();
        let review_count = source
            .recurring_history
            .iter()
            .map(|r| &r.group_id)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let result = crate::report_evidence::Preview {
            client_id: client.into(),
            source_revision: source.revision,
            notes_count,
            notes,
            group_count: source.merchant_groups.len(),
            review_count,
        };
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn get_assessment_planning(
        &mut self,
        client: &str,
    ) -> Result<crate::planning::Plans, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .map_err(storage_error)?;
        scoped_client(&tx, &s.info, client)?;
        let plans = read_plans(&tx, &s.info, client)?;
        tx.commit().map_err(storage_error)?;
        Ok(plans)
    }

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
        let current_plans = plan_revisions(&s.connection, &s.info, client)?;
        let current_notes = notes_count(&s.connection, &s.info, client)?;
        let card_evidence = cards_store::has_cards(&s.connection, &s.info, client)?;
        let mut records = Vec::new();
        for (id, title, created_at, period_start, period_end, source_revision) in rows {
            required_text(&title, "Assessment title", 120)?;
            chrono::DateTime::parse_from_rfc3339(&created_at)
                .map_err(|_| STORAGE_ERROR.to_owned())?;
            crate::imports::iso_date(&period_start)?;
            crate::imports::iso_date(&period_end)?;
            // Status derives from current context, never mutates the archived JSON.
            let profile_at_save:String=s.connection.query_row("SELECT json_extract(payload_json,'$.client.updatedAt') FROM client_assessment_records WHERE id=?1",[&id],|r|r.get(0)).map_err(storage_error)?;
            let plan_json:Option<String>=s.connection.query_row("SELECT json_extract(payload_json,'$.planning.revisions') FROM client_assessment_records WHERE id=?1",[&id],|r|r.get(0)).map_err(storage_error)?;
            let plans_changed = plan_json
                .map(|json| {
                    serde_json::from_str::<crate::planning::Revisions>(&json)
                        .map(|r| r != current_plans)
                        .map_err(|_| STORAGE_ERROR.to_owned())
                })
                .transpose()?
                .unwrap_or(false);
            let recorded_notes:Option<u32>=s.connection.query_row("SELECT json_extract(payload_json,'$.supporting.notesCount') FROM client_assessment_records WHERE id=?1",[&id],|r|r.get(0)).map_err(storage_error)?;
            let brand_revision:Option<i64>=s.connection.query_row("SELECT json_extract(payload_json,'$.branding.revision') FROM client_assessment_records WHERE id=?1",[&id],|r|r.get(0)).map_err(storage_error)?;
            let coverage_revision:Option<i64>=s.connection.query_row("SELECT json_extract(payload_json,'$.completeness.revision') FROM client_assessment_records WHERE id=?1",[&id],|r|r.get(0)).map_err(storage_error)?;
            let coverage_changed = match coverage_revision {
                Some(r) => coverage_review_store::revision(&s.connection, &s.info, client)? != r,
                None => false,
            };
            let intake_json: Option<String> = s.connection.query_row(
                "SELECT json_extract(payload_json,'$.intake') FROM client_assessment_records WHERE id=?1",
                [&id], |r| r.get(0),
            ).map_err(storage_error)?;
            let intake_changed = intake_json
                .map(|json| {
                    let intake: crate::report_intake::Snapshot =
                        serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
                    report_intake_store::changed_since(&s.connection, &s.info, client, &intake)
                })
                .transpose()?
                .unwrap_or(false);
            let branding_changed =
                branding_store::revision(&s.connection, &s.info)? != brand_revision.unwrap_or(0);
            let card_capture:bool=s.connection.query_row("SELECT COALESCE(json_type(payload_json,'$.cards')='object',0) FROM client_assessment_records WHERE id=?1",[&id],|r|r.get(0)).map_err(storage_error)?;
            let integrated:Option<(i64,i64)>=s.connection.query_row("SELECT json_extract(payload_json,'$.cards.links.revision'),json_extract(payload_json,'$.cardFacts.revision') FROM client_assessment_records WHERE id=?1 AND json_extract(payload_json,'$.format')=9",[&id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage_error)?;
            let cards_changed = if let Some((links, facts)) = integrated {
                card_review_store::history(&s.connection, &s.info, client)?
                    .last()
                    .is_none_or(|v| v.revision != links)
                    || card_facts_store::revision(&s.connection, &s.info, client)? != facts
            } else {
                card_evidence || card_capture
            };
            records.push(Summary {
                id,
                title,
                created_at,
                period_start,
                period_end,
                source_revision,
                needs_review: cards_changed
                    || intake_changed
                    || coverage_changed
                    || branding_changed
                    || source_revision != source.revision
                    || profile_at_save != current.updated_at
                    || plans_changed
                    || recorded_notes.is_some_and(|n| n != current_notes),
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
                from_month: input.from_month.clone(),
                through_month: input.through_month.clone(),
            },
            calculation_date,
        )?;
        if input.selected_scenario.is_some() && input.planning.is_some() {
            return Err("Select either a saved combined scenario or the current saved planning models, not both.".into());
        }
        let selected_scenario = input.selected_scenario;
        let chosen = selected_scenario
            .as_ref()
            .map(|selection| scenarios_store::select(&tx, &s.info, client, selection))
            .transpose()?;
        let planning = input
            .planning
            .map(|mut selection| {
                selection.label = required_text(&selection.label, "Scenario label", 120)?;
                selection.assumptions =
                    optional_text(&selection.assumptions, "Scenario assumptions", 4000, true)?;
                crate::planning::capture(read_plans(&tx, &s.info, client)?, selection)
            })
            .transpose()?;
        let planning = if let Some(r) = &chosen {
            Some(crate::saved_scenarios::planning(&r.payload)?)
        } else {
            planning
        };
        let supporting=input.supporting.map(|selection| -> Result<crate::report_evidence::Snapshot,String>{
            let count=notes_count(&tx,&s.info,client)?;
            if count!=selection.expected_notes_count {return Err("Session notes changed since evidence review. Reload report evidence and review your selection; nothing was saved.".into());}
            if selection.note_ids.len()>crate::report_evidence::MAX_NOTES || selection.note_ids.iter().collect::<std::collections::HashSet<_>>().len()!=selection.note_ids.len() {return Err("Select at most ten distinct session notes.".into());}
            client_history_store::require_current(&tx,&s.info,client,&selection.note_ids)?;
            let notes=selection.note_ids.iter().map(|id|note(&tx,&s.info,client,id)).collect::<Result<Vec<_>,_>>()?;
            Ok(crate::report_evidence::Snapshot{calculator:"counselor-evidence-v1".into(),notes_count:count,notes})
        }).transpose()?;
        let intake = input
            .intake
            .map(|selection| {
                crate::report_intake::capture(
                    report_intake_store::current(&tx, &s.info, client)?,
                    selection,
                )
            })
            .transpose()?;
        let completeness = input
            .completeness
            .map(|selection| {
                coverage_review_store::capture(
                    &tx,
                    &s.info,
                    client,
                    selection,
                    &source,
                    &profile,
                    (&input.from_month, &input.through_month),
                )
            })
            .transpose()?;
        if input.card_facts.is_some() && input.cards.is_none() {
            return Err(
                "Include the reviewed bank/card supplement when capturing card completeness."
                    .into(),
            );
        }
        let card_facts = input
            .card_facts
            .map(|selection| {
                card_facts_store::capture(
                    &tx,
                    &s.info,
                    client,
                    selection,
                    &source,
                    &profile,
                    (&input.from_month, &input.through_month),
                )
            })
            .transpose()?;
        let cards = input
            .cards
            .map(|selection| {
                card_review_store::capture(
                    &tx,
                    &s.info,
                    client,
                    selection,
                    &source,
                    crate::dashboard::Input {
                        from_month: input.from_month.clone(),
                        through_month: input.through_month.clone(),
                    },
                )
            })
            .transpose()?;
        let required_bank_decisions = cards
            .as_ref()
            .map(|c| {
                c.links
                    .repayments
                    .iter()
                    .map(|p| p.bank_decision_id.as_str())
                    .collect::<std::collections::HashSet<_>>()
            })
            .unwrap_or_default();
        let card_bank_history = source
            .history
            .iter()
            .filter(|h| required_bank_decisions.contains(h.id.as_str()))
            .map(|h| serde_json::to_string(h).map_err(|_| STORAGE_ERROR))
            .collect::<Result<Vec<_>, _>>()?;
        let mut basis = if supporting.is_some() {
            assessments::capture_report(source, true)
        } else {
            assessments::capture(source)
        };
        for h in card_bank_history {
            let h: crate::imports::History = serde_json::from_str(&h).map_err(|_| STORAGE_ERROR)?;
            if !basis.history.iter().any(|b| b.id == h.id) {
                basis.history.insert(0, h);
            }
        }
        let branding = branding_store::capture(&tx, &s.info, input.expected_branding_revision)?;
        let mut recorded_workspace = s.info.clone();
        recorded_workspace.organization_name = tx
            .query_row(
                "SELECT name FROM organizations WHERE id=?1",
                [&s.info.organization_id],
                |r| r.get(0),
            )
            .map_err(storage_error)?;
        let has_card_capture = cards.is_some();
        let has_card_facts = card_facts.is_some();
        let payload = Payload {
            card_facts,
            cards,
            completeness: completeness.clone(),
            branding: branding.clone(),
            selected_scenario,
            planning,
            format: if has_card_facts {
                9
            } else if has_card_capture {
                8
            } else if completeness.is_some() {
                7
            } else if branding.is_some() {
                6
            } else if intake.is_some() {
                assessments::INTAKE_FORMAT
            } else if chosen.is_some() {
                4
            } else if supporting.is_some() {
                assessments::FORMAT
            } else {
                2
            },
            supporting,
            intake,
            calculator: assessments::CALCULATOR.into(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            id: Uuid::new_v4().to_string(),
            created_at,
            workspace: recorded_workspace,
            client: profile,
            title: required_text(&input.title, "Assessment title", 120)?,
            findings: optional_text(&input.findings, "Counselor findings", 4000, true)?,
            recommendations: optional_text(&input.recommendations, "Recommendations", 4000, true)?,
            follow_up: optional_text(&input.follow_up, "Follow-up", 4000, true)?,
            assessment,
            basis,
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
        if let Some(selection) = &payload.selected_scenario {
            tx.execute("INSERT INTO assessment_scenario_links(assessment_id,scenario_id,client_id,organization_id,counselor_id,digest) VALUES(?1,?2,?3,?4,?5,?6)",params![payload.id,selection.id,client,s.info.organization_id,s.info.counselor_id,selection.digest]).map_err(storage_error)?;
        }
        audit(&tx, &s.info, "assessment_recorded", Some(&payload.id))?;
        let result = record(&tx, &s.info, client, &payload.id)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}
