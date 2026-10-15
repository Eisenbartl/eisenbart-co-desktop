//! Explicit counselor approval for a bounded, current assessment. No financial overrides.
use crate::{
    assessments,
    imports::{self, Status},
    models::WorkspaceInfo,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
pub const POLICY: &str = "counselor-final-report-v1";
pub const MAX_BYTES: usize = 65536;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gate {
    pub key: String,
    pub label: String,
    pub passed: bool,
    pub detail: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountScope {
    pub account_id: String,
    pub label: String,
    pub applicability: String,
    pub expected_start: Option<String>,
    pub expected_end: Option<String>,
    pub exception: String,
    pub reason_required: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Readiness {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_summary: Option<crate::card_final_policy::Summary>,
    pub policy: String,
    pub client_id: String,
    pub assessment_id: String,
    pub assessment_digest: String,
    pub reviewed_on: String,
    pub ready: bool,
    pub gates: Vec<Gate>,
    pub accounts: Vec<AccountScope>,
    pub planning: String,
    pub disclosures: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountDecision {
    pub account_id: String,
    pub reason: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Decision {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_review: Option<CardDecision>,
    pub accounts: Vec<AccountDecision>,
    pub planning: String,
    pub planning_notes: String,
    pub professional_notes: String,
    pub confirm_client: bool,
    pub confirm_sources: bool,
    pub confirm_inventory: bool,
    pub confirm_assumptions: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardDecision {
    pub notes: String,
    pub confirmed: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub assessment_id: String,
    pub expected_digest: String,
    pub expected_readiness: String,
    pub decision: Decision,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payload {
    pub format: u32,
    pub policy: String,
    pub app_version: String,
    pub id: String,
    pub client_id: String,
    pub assessment_id: String,
    pub assessment_digest: String,
    pub approved_at: String,
    pub report_date: String,
    pub approved_by: WorkspaceInfo,
    pub readiness: Readiness,
    pub decision: Decision,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub readiness: Readiness,
    pub receipt: String,
    pub approval_id: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub client_id: String,
    pub payload: Payload,
    pub digest: String,
    pub assessment: assessments::Record,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
}
pub fn readiness(
    p: &assessments::Payload,
    digest: &str,
    changed: bool,
    today: NaiveDate,
) -> Result<Readiness, String> {
    assessments::validate(p)?;
    let a = &p.assessment;
    let start = imports::iso_date(&a.period_start)?;
    let end = imports::iso_date(&a.period_end)?;
    let mut gates = vec![];
    let mut add = |key: &str, label: &str, passed: bool, detail: &str| {
        gates.push(Gate {
            key: key.into(),
            label: label.into(),
            passed,
            detail: detail.into(),
        })
    };
    if p.cards.is_some() {
        add("cardPolicy","Card-inclusive final approval policy",false,"The bank/card supplement is provisional. Policy v1 cannot approve card-inclusive completeness; retain this as an internal assessment.");
    }
    add("context","Current client and supporting context",!changed,"Client, source decisions, firm, selected review and planning context must match the saved assessment. Record a new assessment after changes.");
    add("period","Completed, recent assessment period",end<today && (today-end).num_days()<=90,"Policy v1 supports completed periods ending within the last 90 days. Older internal assessments remain available for historical export.");
    add("approved","Approved dated financial activity",a.totals.approved>0 && a.totals.pending+a.totals.needs_clarification+a.undated_records==0,"At least one approved in-period transaction is required. Pending, clarification and undated records cannot be waived; rejected and out-of-period rows remain excluded.");
    let m = &a.facts_metrics;
    add("facts","Dated balances and financial classifications",m.balance_complete && m.balance_aligned && !m.balance_outdated && m.stale_classifications==0 && m.unclassified_spending=="0.00" && m.unclassified_income=="0.00","Record a complete counselor-verified balance inventory at period end and current income/spending classifications. Unknown balances or classifications cannot become zero.");
    let mut accounts = vec![];
    let mut scope_ok = true;
    let mut selected_ok = true;
    if let Some(c) = &p.completeness {
        add("inventory","Recorded household inventory and missing information",c.data.inventory_reviewed && !c.data.inventory_notes.is_empty() && c.data.missing_information.is_empty() && !c.data.accounts.is_empty() && c.data.accounts.iter().all(|e|e.account_id.is_some()),"Attach a reviewed household inventory. Missing information and unprovided/unlinked accounts or obligations block approval.");
        for e in &c.data.accounts {
            let Some(id) = &e.account_id else {
                scope_ok = false;
                continue;
            };
            let rows = p
                .basis
                .records
                .iter()
                .filter(|r| {
                    &r.account_id == id
                        && r.status == Status::Approved
                        && r.fields.date.as_deref().is_some_and(|d| {
                            d >= a.period_start.as_str() && d <= a.period_end.as_str()
                        })
                })
                .collect::<Vec<_>>();
            let applicability = match (&e.expected_start, &e.expected_end) {
                (Some(s), Some(t)) => {
                    if rows.iter().any(|r| {
                        r.fields
                            .date
                            .as_deref()
                            .is_some_and(|d| d < s.as_str() || d > t.as_str())
                    }) {
                        scope_ok = false;
                    }
                    let (mut day, last) = (imports::iso_date(s)?, imports::iso_date(t)?);
                    while day <= last {
                        let text = day.to_string();
                        let count = c
                            .choices
                            .sources
                            .iter()
                            .filter(|d| {
                                e.source_ids.contains(&d.id)
                                    && d.period_start
                                        .as_deref()
                                        .is_some_and(|s| s <= text.as_str())
                                    && d.period_end.as_deref().is_some_and(|t| t >= text.as_str())
                            })
                            .count();
                        if count != 1 {
                            scope_ok = false;
                        }
                        let Some(next) = day.succ_opt() else { break };
                        day = next;
                    }
                    if rows.iter().any(|r| !e.source_ids.contains(&r.document_id)) {
                        selected_ok = false;
                    }
                    if s == &a.period_start && t == &a.period_end {
                        "fullPeriod"
                    } else {
                        if e.exception.is_empty() {
                            scope_ok = false;
                        }
                        "limitedPeriod"
                    }
                }
                (None, None) if !e.exception.is_empty() && rows.is_empty() => "notApplicable",
                _ => {
                    scope_ok = false;
                    "unconfirmed"
                }
            };
            accounts.push(AccountScope {
                account_id: id.clone(),
                label: e.label.clone(),
                applicability: applicability.into(),
                expected_start: e.expected_start.clone(),
                expected_end: e.expected_end.clone(),
                exception: e.exception.clone(),
                reason_required: applicability != "fullPeriod" || !e.exception.is_empty(),
            });
        }
    } else {
        add("inventory","Recorded household inventory and missing information",false,"Record a current completeness review and attach its exact saved version to a new assessment.");
        scope_ok = false;
    }
    add("coverage","Expected periods, applicability and selected coverage",scope_ok && !accounts.is_empty(),"Every applicable expected day requires exactly one selected declared source period. Partial/nonapplicable periods need recorded explanations; approved activity cannot be excluded by an exception.");
    add("selection","All approved activity represented by coverage sources",selected_ok,"Every approved in-period transaction must belong to a selected source for its expected account. Coverage selection never filters report amounts.");
    let mut source_ok = true;
    let mut source_count = 0;
    for d in &p.basis.documents {
        let rows = p
            .basis
            .records
            .iter()
            .filter(|r| r.document_id == d.id)
            .collect::<Vec<_>>();
        let applicable = rows.iter().any(|r| {
            r.fields
                .date
                .as_deref()
                .is_none_or(|v| v >= a.period_start.as_str() && v <= a.period_end.as_str())
        }) || d
            .input
            .period_start
            .as_deref()
            .zip(d.input.period_end.as_deref())
            .is_some_and(|(s, t)| s <= a.period_end.as_str() && t >= a.period_start.as_str());
        if !applicable {
            continue;
        }
        source_count += 1;
        let dates = d
            .input
            .period_start
            .as_deref()
            .zip(d.input.period_end.as_deref());
        if !d.source_retained
            || d.source_difference.as_deref() != Some("0.00")
            || d.approved_difference.as_deref() != Some("0.00")
            || dates.is_none()
            || rows.iter().any(|r| {
                matches!(r.status, Status::Pending | Status::NeedsClarification)
                    || r.fields.date.is_none()
                    || dates.is_some_and(|(s, t)| {
                        r.fields.date.as_deref().is_some_and(|v| v < s || v > t)
                    })
            })
            || d.balance_reconciliation
                .as_ref()
                .is_some_and(|r| r.conflicts())
        {
            source_ok = false;
        }
    }
    add("sources","Retained sources and reconciled statement arithmetic",source_ok && source_count>0,"Applicable original sources must be retained with authorization, have declared dates and zero whole-source/approved closing differences. Resolve source-wide decisions and running-balance conflicts. Parsing/arithmetic cannot verify a bank.");
    let planning = if p.planning.is_some() {
        "captured"
    } else {
        "notApplicable"
    }
    .to_owned();
    let ready = gates.iter().all(|g| g.passed);
    let mut disclosures=vec!["Counselor approval applies only to the stated period and recorded scope. It is not independent bank verification, a digital signature, or certification of the application for commercial use.".into(),"Approved imported activity supplies the historical amounts. Rejected, undecided, undated and out-of-period rows never supply those amounts. Estimates, unknown ratios, source warnings and dated evidence remain disclosed in the supporting assessment.".into(),"The counselor must inspect original source completeness, possible duplicates, exclusions, classifications, balances, assumptions and recommendations before approval. Automated checks cannot establish unrecorded real-world accounts.".into()];
    if p.planning.is_some() {
        disclosures.push("Captured planning inputs, repayment terms and milestones are hypothetical projections. They do not become historical financial facts through approval.".into());
    } else {
        disclosures.push("No planning scenario is included. The counselor recorded why scenario-based recommendations are not applicable to this assessment.".into());
    }
    if start > end {
        return Err("Invalid assessment period.".into());
    }
    Ok(Readiness {
        card_summary: None,
        policy: POLICY.into(),
        client_id: p.client.id.clone(),
        assessment_id: p.id.clone(),
        assessment_digest: digest.into(),
        reviewed_on: today.to_string(),
        ready,
        gates,
        accounts,
        planning,
        disclosures,
    })
}
pub fn canonical(mut d: Decision, r: &Readiness) -> Result<Decision, String> {
    fn text(s: &str, label: &str, max: usize, required: bool) -> Result<String, String> {
        let s = s.trim();
        if s.chars().count() > max
            || s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
            || required && s.is_empty()
        {
            return Err(format!(
                "{label} is required and must fit its text limit without unsupported controls."
            ));
        }
        Ok(s.into())
    }
    if !r.ready {
        return Err("Final approval is blocked. Resolve every required check and review a new assessment first.".into());
    }
    if !(d.confirm_client && d.confirm_sources && d.confirm_inventory && d.confirm_assumptions) {
        return Err("Confirm the client, original sources, inventory/scope and assumptions explicitly before approval.".into());
    }
    if r.policy == crate::card_final_policy::POLICY {
        let cards = d
            .card_review
            .as_mut()
            .ok_or("Record the card and repayment review explicitly before approval.")?;
        if !cards.confirmed || r.card_summary.is_none() {
            return Err("Confirm the reviewed card sources, spending, repayments and existing debt mappings.".into());
        }
        cards.notes = text(
            &cards.notes,
            "Card evidence and repayment review",
            2000,
            true,
        )?;
    } else if d.card_review.is_some() {
        return Err("Card review decisions require the card-inclusive approval policy.".into());
    }
    if d.planning != r.planning || d.accounts.len() != r.accounts.len() {
        return Err(
            "The account or scenario decisions do not match this assessment's required scope."
                .into(),
        );
    }
    let mut seen = std::collections::HashSet::new();
    for a in &mut d.accounts {
        let required = r
            .accounts
            .iter()
            .find(|e| e.account_id == a.account_id)
            .ok_or("An approval account does not belong to this assessment.")?;
        if !seen.insert(&a.account_id) {
            return Err("Review each expected account once.".into());
        }
        a.reason = text(
            &a.reason,
            "Account applicability/exception decision",
            1000,
            required.reason_required,
        )?;
    }
    d.planning_notes = text(
        &d.planning_notes,
        "Scenario applicability and assumptions agreement",
        2000,
        true,
    )?;
    d.professional_notes = text(
        &d.professional_notes,
        "Professional approval notes",
        4000,
        true,
    )?;
    Ok(d)
}
pub fn validate(p: &Payload, a: &assessments::Record) -> Result<(), String> {
    let at = chrono::DateTime::parse_from_rfc3339(&p.approved_at)
        .map_err(|_| "Invalid approval date.")?;
    let today = at.with_timezone(&chrono_tz::America::Chicago).date_naive();
    let original = match (p.format, p.policy.as_str()) {
        (1, POLICY) => readiness(&a.payload, &a.digest, false, today)?,
        (2, crate::card_final_policy::POLICY) => {
            crate::card_final_policy::readiness(&a.payload, &a.digest, false, today)?
        }
        _ => return Err("Unsupported final approval policy or format.".into()),
    };
    if p.policy != original.policy
        || p.app_version.is_empty()
        || uuid::Uuid::parse_str(&p.id).is_err()
        || p.client_id != a.client_id
        || p.assessment_id != a.payload.id
        || p.assessment_digest != a.digest
        || p.approved_by != a.payload.workspace
        || p.readiness != original
        || p.report_date != today.to_string()
        || at
            < chrono::DateTime::parse_from_rfc3339(&a.payload.created_at)
                .map_err(|_| "Invalid assessment date.")?
        || canonical(p.decision.clone(), &original)? != p.decision
    {
        return Err("The saved final approval or its original evidence is invalid.".into());
    }
    Ok(())
}
