//! Client-facing reviewed guidance tied to an immutable assessment/scenario.
//! No client-supplied financial output enters the plan; all money uses exact cents.
use crate::{assessments, budget, imports, planning};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
pub const MAX_BYTES: usize = 64 * 1024;
const CALCULATOR: &str = "client-action-plan-v1";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Owner {
    Client,
    Counselor,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Action {
    pub step: String,
    pub owner: Owner,
    pub due_date: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Data {
    pub budget_recommendations: String,
    pub actions: Vec<Action>,
    pub follow_up: String,
    pub follow_up_date: Option<String>,
    pub limitations: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub assessment_id: String,
    pub expected_digest: String,
    pub title: String,
    pub data: Data,
    pub acknowledge: bool,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Allocations {
    pub monthly_income: String,
    pub necessary: String,
    pub discretionary: String,
    pub surplus: String,
    pub debt: String,
    pub savings: String,
    pub unallocated: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payload {
    pub format: u32,
    pub calculator: String,
    pub id: String,
    pub client_id: String,
    pub assessment_id: String,
    pub assessment_digest: String,
    pub created_at: String,
    pub report_date: String,
    pub app_version: String,
    pub title: String,
    pub data: Data,
    pub allocations: Allocations,
    pub disclosures: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub client_id: String,
    pub payload: Payload,
    pub assessment: assessments::Record,
    pub digest: String,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub report_date: String,
    pub assessment_id: String,
    pub needs_review: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub client_id: String,
    pub records: Vec<Summary>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub client_id: String,
    pub assessment_id: String,
    pub assessment_digest: String,
    pub report_date: String,
    pub allocations: Allocations,
    pub disclosures: Vec<String>,
}
pub fn local_date(value: &str) -> Result<NaiveDate, String> {
    Ok(chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| "Invalid action plan timestamp.")?
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive())
}
pub fn text(value: &str, field: &str, max: usize, multiline: bool) -> Result<String, String> {
    let s = value.trim();
    if s.is_empty()
        || s.chars().count() > max
        || s.chars()
            .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t')))
    {
        return Err(format!("{field} must contain 1–{max} characters."));
    }
    Ok(s.into())
}
pub fn normalize(mut d: Data, date: NaiveDate) -> Result<Data, String> {
    d.budget_recommendations = text(
        &d.budget_recommendations,
        "Agreed budget recommendations",
        2000,
        true,
    )?;
    d.follow_up = text(&d.follow_up, "Follow-up arrangements", 1000, true)?;
    d.limitations = text(&d.limitations, "Reviewed limitations", 2000, true)?;
    if !(3..=5).contains(&d.actions.len()) {
        return Err("Include three to five prioritized action steps.".into());
    }
    let mut seen = std::collections::HashSet::new();
    for a in &mut d.actions {
        a.step = text(&a.step, "Action step", 600, true)?;
        if !seen.insert(a.step.to_lowercase()) {
            return Err("Each prioritized step must be distinct.".into());
        }
        if let Some(value) = &a.due_date {
            if imports::iso_date(value)? < date {
                return Err("Action due dates cannot precede the action plan report date.".into());
            }
        }
    }
    if let Some(value) = &d.follow_up_date {
        if imports::iso_date(value)? < date {
            return Err("Follow-up dates cannot precede the action plan report date.".into());
        }
    }
    Ok(d)
}
fn cents(v: &str) -> Result<i64, String> {
    let n = v
        .replace('.', "")
        .parse::<i64>()
        .map_err(|_| "Invalid captured allocation amount.")?;
    if budget::money(n) != v {
        return Err("Invalid captured allocation precision.".into());
    }
    Ok(n)
}
pub fn allocations(a: &assessments::Payload) -> Result<Allocations, String> {
    assessments::validate(a)?;
    let s = a
        .planning
        .as_ref()
        .ok_or("Choose a saved assessment with a captured planning scenario.")?;
    let b = &s.plans.budget;
    if b.summary.missing_count > 0
        || !b
            .lines
            .iter()
            .any(|l| l.kind == budget::Kind::Income && l.amount.is_some())
        || !b
            .lines
            .iter()
            .any(|l| l.kind == budget::Kind::Necessary && l.amount.is_some())
    {
        return Err("Review a complete entered budget with known income and necessary expenses before recording agreed allocations.".into());
    }
    let debt = match &s.plans.debt.data {
        Some(d) => d
            .monthly_payment
            .as_ref()
            .ok_or("The captured monthly debt payment is unknown.")?,
        None => &b.summary.planned_debt_payments,
    };
    let savings = match &s.plans.savings.summary {
        Some(s) => s
            .requested_monthly_savings
            .as_ref()
            .ok_or("The captured primary savings contributions are incomplete.")?,
        None => &b.summary.proposed_savings,
    };
    let savings_cents = cents(savings)?;
    let remaining = cents(&b.summary.surplus_before_allocations)?
        .checked_sub(cents(debt)?)
        .and_then(|v| v.checked_sub(savings_cents))
        .ok_or("Allocation calculation exceeds its supported limit.")?;
    if remaining < 0 {
        return Err("The selected debt and primary savings allocations exceed the captured monthly surplus. Revise and save a new scenario before recording an action plan.".into());
    }
    Ok(Allocations {
        monthly_income: b.summary.monthly_income.clone(),
        necessary: b.summary.necessary_expenses.clone(),
        discretionary: b.summary.discretionary_expenses.clone(),
        surplus: b.summary.surplus_before_allocations.clone(),
        debt: debt.clone(),
        savings: savings.clone(),
        unallocated: budget::money(remaining),
    })
}
pub fn disclosures(a: &assessments::Payload, date: NaiveDate) -> Result<Vec<String>, String> {
    let s = a
        .planning
        .as_ref()
        .ok_or("A captured planning scenario is required.")?;
    let agreement = allocations(a)?;
    let mut warnings=vec!["This is counselor-reviewed guidance based on incomplete financial information. Statement/account coverage remains unverified; forecasts are conditional estimates, not guarantees or a certification of financial completeness.".into(),"Historical figures use only approved imported activity in the assessment period. Entered planning income, expenses, balances and allocations are separate assumptions; missing information is not treated as verified zero.".into(),"Agreement is recorded by the counselor; no electronic client signature, legal approval or independent financial verification is implied.".into(),"Cash-flow dates and account schedules are entered independently. Budget, repayment and savings amounts are not automatically synchronized to that schedule.".into()];
    if agreement.debt != s.plans.budget.summary.planned_debt_payments
        || agreement.savings != s.plans.budget.summary.proposed_savings
    {
        warnings.push(format!("Agreed allocations use the selected model payments (debt USD {}, primary savings USD {}), which differ from the captured budget allocations (debt USD {}, savings USD {}). The original budget is unchanged; review these differences and update the active plans before implementation.",agreement.debt,agreement.savings,s.plans.budget.summary.planned_debt_payments,s.plans.budget.summary.proposed_savings));
    }
    if (date - imports::iso_date(&a.assessment.period_end)?).num_days() > 90 {
        warnings.push("The historical assessment period is over 90 days old; review current statements before relying on it.".into());
    }
    warnings.push(format!("Period source review: {} approved, {} rejected, {} pending, {} needing clarification; {} undated records excluded. Review decisions do not establish complete statement coverage.",a.assessment.totals.approved,a.assessment.totals.rejected,a.assessment.totals.pending,a.assessment.totals.needs_clarification,a.assessment.undated_records));
    warnings.extend(
        a.assessment
            .warnings
            .iter()
            .filter(|w| w.as_str() != crate::dashboard::LIVE_VIEW_WARNING)
            .cloned(),
    );
    warnings.extend(s.plans.budget.summary.warnings.iter().cloned());
    if let Some(c) = &s.plans.debt.comparison {
        warnings.extend(c.warnings.iter().cloned());
    } else {
        warnings.push("No saved debt plan verifies actual debt obligations. The debt allocation, if any, comes from the captured budget.".into());
    }
    if let Some(p) = planning::projection(s) {
        warnings.extend(p.warnings.iter().cloned());
    } else {
        warnings.push(
            "No supported repayment projection is selected; no payoff milestone is promised."
                .into(),
        );
    }
    if let Some(v) = &s.plans.savings.summary {
        warnings.extend(v.warnings.iter().cloned());
    } else {
        warnings.push("No saved savings forecast verifies goals or deadlines; the savings allocation comes from the captured budget.".into());
    }
    if let Some(c) = &s.plans.cashflow.summary {
        warnings.extend(c.warnings.iter().cloned());
        if c.ledger.iter().any(|x| !x.warnings.is_empty()) {
            warnings.push("The independent dated cash-flow ledger contains forecast concerns; review the full saved scenario before acting.".into());
        }
    } else {
        warnings.push(
            "No dated cash-flow schedule is captured; pay and due dates remain to be verified."
                .into(),
        );
    }
    for d in &a.basis.documents {
        if d.source_difference.as_deref().is_some_and(|v| v != "0.00")
            || d.approved_difference
                .as_deref()
                .is_some_and(|v| v != "0.00")
        {
            warnings.push(format!("Captured source {} has a closing-balance discrepancy. Resolve/review the original statement; these comparisons do not verify assets.",d.file_name));
        }
    }
    let mut seen = std::collections::HashSet::new();
    warnings.retain(|w| seen.insert(w.clone()));
    Ok(warnings)
}
pub fn review(a: &assessments::Record, created: &str) -> Result<Review, String> {
    if a.needs_review {
        return Err("This assessment needs review. Record a new current assessment/scenario before creating a client action plan.".into());
    }
    let date = local_date(created)?;
    Ok(Review {
        client_id: a.client_id.clone(),
        assessment_id: a.payload.id.clone(),
        assessment_digest: a.digest.clone(),
        report_date: date.to_string(),
        allocations: allocations(&a.payload)?,
        disclosures: disclosures(&a.payload, date)?,
    })
}
pub fn validate(p: &Payload, a: &assessments::Record) -> Result<(), String> {
    if p.format != 1
        || p.calculator != CALCULATOR
        || p.id.is_empty()
        || p.app_version.is_empty()
        || p.client_id != a.client_id
        || p.assessment_id != a.payload.id
        || p.assessment_digest != a.digest
    {
        return Err("Invalid or unsupported client action plan identity/basis.".into());
    }
    let date = local_date(&p.created_at)?;
    if p.report_date != date.to_string()
        || text(&p.title, "Action plan title", 120, false)? != p.title
        || normalize(p.data.clone(), date)? != p.data
    {
        return Err("Invalid client action plan dates or text.".into());
    }
    if p.allocations != allocations(&a.payload)? || p.disclosures != disclosures(&a.payload, date)?
    {
        return Err(
            "Client action plan allocations or disclosures do not match its captured basis.".into(),
        );
    }
    Ok(())
}
pub fn capture(
    a: &assessments::Record,
    input: Input,
    id: String,
    created_at: String,
) -> Result<Payload, String> {
    if !input.acknowledge {
        return Err("Confirm the client, approved basis, scenario, proposed agreement and reviewed limitations before recording the action plan.".into());
    }
    if input.assessment_id != a.payload.id || input.expected_digest != a.digest {
        return Err(
            "The supporting assessment did not match the reviewed record. Nothing was recorded."
                .into(),
        );
    }
    let r = review(a, &created_at)?;
    let date = local_date(&created_at)?;
    let p = Payload {
        format: 1,
        calculator: CALCULATOR.into(),
        id,
        client_id: a.client_id.clone(),
        assessment_id: a.payload.id.clone(),
        assessment_digest: a.digest.clone(),
        created_at,
        report_date: r.report_date,
        app_version: env!("CARGO_PKG_VERSION").into(),
        title: text(&input.title, "Action plan title", 120, false)?,
        data: normalize(input.data, date)?,
        allocations: r.allocations,
        disclosures: r.disclosures,
    };
    validate(&p, a)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_plan_report_date_uses_chicago_daylight_saving_and_local_midnight() {
        for (utc, date) in [
            ("2026-01-02T04:00:00Z", "2026-01-01"),
            ("2026-07-02T04:30:00Z", "2026-07-01"),
            ("2026-07-02T05:00:00Z", "2026-07-02"),
            ("2026-01-02T06:00:00Z", "2026-01-02"),
        ] {
            assert_eq!(local_date(utc).unwrap().to_string(), date);
        }
        assert!(local_date("invalid").is_err());
        let mut value = serde_json::json!({"assessmentId":"x","expectedDigest":"x","title":"x","data":{"budgetRecommendations":"Verify", "actions":[{"step":"Review statements","owner":"client","dueDate":null},{"step":"Confirm payments","owner":"counselor","dueDate":null},{"step":"Arrange savings","owner":"client","dueDate":null}],"followUp":"Arrange meeting","followUpDate":null,"limitations":"Missing coverage"},"acknowledge":true});
        assert!(serde_json::from_value::<Input>(value.clone()).is_ok());
        value["allocations"] = serde_json::json!({"debt":"100"});
        assert!(serde_json::from_value::<Input>(value).is_err());
    }
}
