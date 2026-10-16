//! Presentation over validated immutable records. No financial calculation or approval.
use crate::{assessments, final_reports};
use serde::{Deserialize, Serialize};
pub const VERSION: &str = "counselor-summary-v1";
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Export {
    #[serde(flatten)]
    pub pdf: crate::assessment_pdf::Export,
    pub viewer_ticket: String,
    pub viewer_supported: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Internal,
    Final,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Layout {
    SummaryAppendix,
    Detailed,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Figure {
    pub label: String,
    pub value: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub kind: Kind,
    pub record_id: String,
    pub digest: String,
    pub title: String,
    pub firm_name: String,
    pub client_name: String,
    pub period_start: String,
    pub period_end: String,
    pub report_date: String,
    pub balance_as_of: Option<String>,
    pub basis: String,
    pub status: String,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
    pub figures: Vec<Figure>,
    pub disclosure: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub client_id: String,
    pub assessment_id: String,
    pub presentation: String,
    pub receipt: String,
    pub internal: Summary,
    pub final_report: Option<Summary>,
}
impl Context {
    pub fn selected(&self, kind: Kind) -> Result<&Summary, String> {
        match kind {
            Kind::Internal => Ok(&self.internal),
            Kind::Final => self.final_report.as_ref().ok_or("This assessment has no final counselor approval. Use the internal report or complete the Final report checklist.".into()),
        }
    }
}
pub fn summary(
    r: &assessments::Record,
    approved: Option<&final_reports::Record>,
) -> Result<Summary, String> {
    assessments::validate(&r.payload)?;
    if let Some(f) = approved {
        final_reports::validate(&f.payload, r)?;
    }
    let p = &r.payload;
    let a = &p.assessment;
    let m = &a.facts_metrics;
    let mut figures = vec![];
    let mut add = |label: &str, value: Option<&str>| {
        figures.push(Figure {
            label: label.into(),
            value: value.map(str::to_owned),
        })
    };
    add("Approved bank income", Some(&a.totals.income));
    let card = approved.and_then(|f| f.payload.readiness.card_summary.as_ref());
    let basis = if let Some(s) = card {
        add("Combined necessary spending", Some(&s.necessary_spending));
        add(
            "Combined discretionary spending",
            Some(&s.discretionary_spending),
        );
        add("Combined recorded spending", Some(&s.combined_spending));
        add(
            "Income less combined spending",
            Some(&s.income_less_spending),
        );
        add("Bank cash movement", Some(&s.bank_cash_movement));
        add(
            "Matched repayments (excluded from spending)",
            Some(&s.matched_repayments),
        );
        add(
            "Existing household debt (counted once)",
            Some(&s.existing_household_debt),
        );
        "Approved bank/card spending; matched card repayments are excluded from spending and existing debt is counted once."
    } else {
        add("Bank net spending", Some(&a.totals.net_spending));
        add("Bank income less net spending", Some(&a.operating_cashflow));
        add("Bank cash movement", Some(&a.totals.net_cash));
        add("Recorded cash at balance date", m.cash.as_deref());
        add(
            "Accessible savings at balance date",
            m.accessible_savings.as_deref(),
        );
        add("Recorded debt at balance date", m.debt.as_deref());
        add(
            "Estimated net worth at balance date",
            m.net_worth.as_deref(),
        );
        "Bank-only approved activity. Separate credit-card activity is excluded from these spending figures; see captured card evidence in the appendix when present."
    };
    let kind = if approved.is_some() {
        Kind::Final
    } else {
        Kind::Internal
    };
    let at = approved.map_or(p.created_at.as_str(), |f| f.payload.approved_at.as_str());
    let date = chrono::DateTime::parse_from_rfc3339(at)
        .map_err(|_| "Report date is unreadable.")?
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive()
        .to_string();
    let needs_review = approved.map_or(r.needs_review, |f| f.needs_review);
    Ok(Summary {
        kind,record_id:approved.map_or_else(||p.id.clone(),|f|f.payload.id.clone()),
        digest:approved.map_or_else(||r.digest.clone(),|f|f.digest.clone()),title:p.title.clone(),
        firm_name:p.branding.as_ref().map_or_else(||p.workspace.organization_name.clone(),|b|b.data.name.clone()),
        client_name:format!("{} {}",p.client.first_name,p.client.last_name),
        period_start:a.period_start.clone(),period_end:a.period_end.clone(),report_date:date,
        balance_as_of:m.as_of.clone(),basis:basis.into(),
        status:if approved.is_some(){if needs_review{"Historical counselor approval / current context needs review"}else{"Final counselor report / counselor approved"}}else{"Internal assessment / incomplete information"}.into(),
        needs_review,review_reasons:approved.map_or_else(||r.review_reasons.clone(),|f|f.review_reasons.clone()),figures,
        disclosure:if approved.is_some(){"Original approved figures and assumptions are retained. This summary does not update them. Read the evidence appendix and current-review flags before reuse."}else{"This internal assessment is not a final approval. Missing information, pending decisions and assumptions remain unresolved where indicated in the evidence appendix."}.into(),
    })
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportInput {
    pub assessment_id: String,
    pub kind: Kind,
    pub layout: Layout,
    pub expected_receipt: String,
    pub path: String,
}
