//! Reviewed card inventory and evidence, separate from bank facts and final approval.
use crate::{budget::money, card_review, cards, dashboard, financial_facts, imports};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
pub const MAX_JSON: usize = 4 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Row {
    pub row: usize,
    pub approval_id: Option<String>,
    pub status: imports::Status,
    pub fields: cards::Fields,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub account_id: Option<String>,
    pub file_name: String,
    pub sha256: String,
    pub start: String,
    pub end: String,
    pub opening: String,
    pub closing: String,
    pub reconciled: bool,
    pub rows: Vec<Row>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Choices {
    pub link_revision: i64,
    pub accounts: Vec<card_review::Account>,
    pub sources: Vec<Source>,
    pub facts: Option<financial_facts::Review>,
}
pub fn choices(
    links: Option<&card_review::Snapshot>,
    ledger: &cards::Snapshot,
    facts: Option<financial_facts::Review>,
) -> Choices {
    Choices {
        link_revision: links.map_or(0, |s| s.revision),
        accounts: links.map(|s| s.accounts.clone()).unwrap_or_default(),
        sources: ledger
            .statements
            .iter()
            .map(|s| Source {
                id: s.source.id.clone(),
                account_id: links
                    .and_then(|l| l.assignments.iter().find(|a| a.statement_id == s.source.id))
                    .map(|a| a.account_id.clone()),
                file_name: s.source.file_name.clone(),
                sha256: s.source.sha256.clone(),
                start: s.source.input.period_start.clone(),
                end: s.source.input.period_end.clone(),
                opening: s.source.input.opening_owed.clone(),
                closing: s.source.input.closing_owed.clone(),
                reconciled: s.summary.reconciled,
                rows: s
                    .rows
                    .iter()
                    .map(|r| Row {
                        row: r.row,
                        approval_id: s
                            .history
                            .iter()
                            .rev()
                            .find(|h| {
                                h.row == r.row
                                    && h.status == imports::Status::Approved
                                    && r.status == imports::Status::Approved
                            })
                            .map(|h| h.id.clone()),
                        status: r.status.clone(),
                        fields: r.fields.clone(),
                    })
                    .collect(),
            })
            .collect(),
        facts,
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Applicability {
    Unknown,
    Applicable,
    NotApplicable,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Necessary,
    Discretionary,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Expected {
    pub account_id: Option<String>,
    pub label: String,
    pub applicability: Applicability,
    pub start: Option<String>,
    pub end: Option<String>,
    pub notes: String,
    pub source_ids: Vec<String>,
    pub closing_source_id: Option<String>,
    pub balance_label: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Classification {
    pub statement_id: String,
    pub row: usize,
    pub approval_id: String,
    pub role: Role,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Data {
    pub from_month: String,
    pub through_month: String,
    pub inventory_reviewed: bool,
    pub inventory_notes: String,
    pub missing_information: String,
    pub accounts: Vec<Expected>,
    pub classifications: Vec<Classification>,
    pub conclusion: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Finding {
    pub account_id: Option<String>,
    pub label: String,
    pub closing_owed: Option<String>,
    pub mapped_balance: Option<String>,
    pub difference: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evaluation {
    pub calculator: String,
    pub period_start: String,
    pub period_end: String,
    pub necessary_spending: String,
    pub discretionary_spending: String,
    pub unclassified_spending: String,
    pub stale_classifications: usize,
    pub findings: Vec<Finding>,
    pub evidence_complete: bool,
    pub issues: Vec<String>,
    pub disclosures: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub format: u32,
    pub client_id: String,
    pub revision: i64,
    pub source_revision: i64,
    pub profile_updated_at: String,
    pub updated_at: String,
    pub counselor_name: String,
    pub choices: Choices,
    pub data: Data,
    pub evaluation: Evaluation,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub expected_source_revision: i64,
    pub expected_profile_updated_at: String,
    pub data: Data,
    pub acknowledge: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub revision: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub client_id: String,
    pub current_source_revision: i64,
    pub current_profile_updated_at: String,
    pub latest_revision: i64,
    pub snapshot: Option<Snapshot>,
    pub history: Vec<Snapshot>,
    pub choices: Choices,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,
}
pub fn canonical(mut d: Data, c: &Choices) -> Result<Data, String> {
    let (start, end, _) = dashboard::period(&dashboard::Input {
        from_month: d.from_month.clone(),
        through_month: d.through_month.clone(),
    })?;
    d.inventory_notes = cards::text(&d.inventory_notes, 2000, false)?;
    d.missing_information = cards::text(&d.missing_information, 2000, false)?;
    d.conclusion = cards::text(&d.conclusion, 2000, false)?;
    if d.inventory_reviewed && d.inventory_notes.is_empty() {
        return Err("Describe the card inventory reviewed before confirming it.".into());
    }
    if d.accounts.len() > 40
        || d.classifications.len() > 4000
        || c.accounts.len() > 20
        || c.sources.len() > 20
    {
        return Err("Card evidence exceeds its account, statement or classification limit.".into());
    }
    let mut ids = HashSet::new();
    let mut labels = HashSet::new();
    let mut balances = HashSet::new();
    for e in &mut d.accounts {
        e.label = cards::text(&e.label, 120, true)?;
        e.notes = cards::text(&e.notes, 1000, false)?;
        if !labels.insert(e.label.to_lowercase()) {
            return Err("Use distinct expected card labels.".into());
        }
        if let Some(id) = &e.account_id {
            let a = c
                .accounts
                .iter()
                .find(|a| &a.id == id)
                .ok_or("This card account is unavailable for this client.")?;
            if !ids.insert(id) || e.label != a.label {
                return Err("Include each known card account once with its reviewed name.".into());
            }
        }
        match (&e.start, &e.end) {
            (Some(s), Some(t)) => {
                let s = imports::iso_date(s)?;
                let t = imports::iso_date(t)?;
                if s > t || s < start || t > end {
                    return Err(
                        "Card applicability dates must be ordered inside the review period.".into(),
                    );
                }
            }
            (None, None) => {}
            _ => return Err("Enter both applicability dates or leave both unknown.".into()),
        }
        if e.applicability != Applicability::Applicable
            && (e.start.is_some()
                || !e.source_ids.is_empty()
                || e.closing_source_id.is_some()
                || e.balance_label.is_some())
        {
            return Err("Only applicable cards can select dates, statement sources and a balance mapping. Clear those selections explicitly.".into());
        }
        if e.source_ids.len() > 20
            || e.source_ids.iter().collect::<HashSet<_>>().len() != e.source_ids.len()
        {
            return Err("Choose distinct card statement sources.".into());
        }
        for id in &e.source_ids {
            let s = c
                .sources
                .iter()
                .find(|s| &s.id == id)
                .ok_or("The card source is unavailable for this client.")?;
            if e.account_id.is_none() || s.account_id != e.account_id {
                return Err(
                    "Each selected source must belong to its reviewed card account.".into(),
                );
            }
        }
        if let Some(id) = &e.closing_source_id {
            if !e.source_ids.contains(id) {
                return Err(
                    "The closing balance must use a selected statement for this card.".into(),
                );
            }
        }
        if let Some(label) = &e.balance_label {
            let b = c
                .facts
                .as_ref()
                .and_then(|f| f.data.balances.iter().find(|b| &b.label == label))
                .ok_or(
                    "The selected verified balance is unavailable. Review financial facts first.",
                )?;
            if b.kind != financial_facts::BalanceKind::Debt || !balances.insert(label) {
                return Err("Map each card to one distinct existing debt balance. Combined debt entries must be split explicitly in verified facts first.".into());
            }
        }
    }
    if ids.len() != c.accounts.len() {
        return Err("Include every known card account, including archived cards; unknown applicability remains explicit.".into());
    }
    let mut rows = HashSet::new();
    for x in &d.classifications {
        let row = c
            .sources
            .iter()
            .find(|s| s.id == x.statement_id)
            .and_then(|s| s.rows.iter().find(|r| r.row == x.row))
            .ok_or("The classified card row is unavailable for this client.")?;
        if !rows.insert((&x.statement_id, x.row))
            || row.status != imports::Status::Approved
            || row.approval_id.as_deref() != Some(x.approval_id.as_str())
            || !spending(&row.fields.kind)
        {
            return Err("Classify each currently approved purchase, refund, fee or interest row once, using its exact approval. Payments and adjustments cannot receive spending roles.".into());
        }
    }
    Ok(d)
}
fn spending(k: &cards::Kind) -> bool {
    matches!(
        k,
        cards::Kind::Purchase | cards::Kind::Refund | cards::Kind::Fee | cards::Kind::Interest
    )
}
pub fn evaluate(d: &Data, c: &Choices) -> Result<Evaluation, String> {
    let (start, end, _) = dashboard::period(&dashboard::Input {
        from_month: d.from_month.clone(),
        through_month: d.through_month.clone(),
    })?;
    let start = start.to_string();
    let end = end.to_string();
    let mut issues = vec![];
    let mut add = |s: String| {
        if !issues.contains(&s) {
            issues.push(s)
        }
    };
    if !d.inventory_reviewed || d.inventory_notes.is_empty() {
        add("The household card inventory has not been explicitly reviewed.".into());
    }
    if !d.missing_information.is_empty() {
        add("Missing card information remains recorded.".into());
    }
    if d.conclusion.is_empty() {
        add("Record a professional review conclusion.".into());
    }
    if c.accounts.is_empty() {
        add("No durable card accounts have been reviewed.".into());
    }
    let mut necessary = 0;
    let mut discretionary = 0;
    let mut unclassified = 0;
    let mut stale = 0;
    for s in &c.sources {
        if s.start > end || s.end < start {
            continue;
        }
        if s.account_id.is_none() {
            add(format!(
                "Assign {} to a reviewed card account.",
                s.file_name
            ));
        }
        for r in &s.rows {
            if r.status != imports::Status::Approved
                || !r
                    .fields
                    .date
                    .as_deref()
                    .is_some_and(|t| t >= start.as_str() && t <= end.as_str())
            {
                continue;
            }
            if spending(&r.fields.kind) {
                let n = imports::signed_money(
                    r.fields
                        .amount
                        .as_deref()
                        .ok_or("Missing approved card amount.")?,
                )?;
                if let Some(x) = d.classifications.iter().find(|x| {
                    x.statement_id == s.id
                        && x.row == r.row
                        && Some(x.approval_id.as_str()) == r.approval_id.as_deref()
                }) {
                    match x.role {
                        Role::Necessary => necessary += n,
                        Role::Discretionary => discretionary += n,
                    }
                } else {
                    unclassified += n;
                    stale += 1;
                }
            }
            if r.fields.kind == cards::Kind::Adjustment {
                add(format!(
                    "{} row {} is an adjustment needing supported spending treatment.",
                    s.file_name,
                    r.row + 2
                ));
            }
        }
    }
    if stale > 0 {
        add(format!("{stale} approved card spending rows lack current source-bound classifications. Offsetting amounts do not clear this gate."));
    }
    let mut findings = vec![];
    for e in &d.accounts {
        let mut finding = Finding {
            account_id: e.account_id.clone(),
            label: e.label.clone(),
            closing_owed: None,
            mapped_balance: e.balance_label.as_ref().and_then(|label| {
                c.facts
                    .as_ref()
                    .and_then(|f| f.data.balances.iter().find(|b| &b.label == label))
                    .and_then(|b| b.amount.clone())
            }),
            difference: None,
        };
        if e.account_id.is_none() {
            add(format!(
                "{} is an unprovided or unlinked expected card.",
                e.label
            ));
        }
        let sources = c
            .sources
            .iter()
            .filter(|s| {
                s.account_id.is_some()
                    && s.account_id == e.account_id
                    && s.start <= end
                    && s.end >= start
            })
            .collect::<Vec<_>>();
        match e.applicability {
            Applicability::Unknown => add(format!("{} has unknown applicability.", e.label)),
            Applicability::NotApplicable => {
                if e.notes.is_empty() {
                    add(format!(
                        "{} requires a reason for non-applicability.",
                        e.label
                    ));
                }
                if !sources.is_empty() {
                    add(format!("{} has an in-period declared statement and cannot be marked not applicable.",e.label));
                }
            }
            Applicability::Applicable => {
                match (&e.start, &e.end) {
                    (Some(from), Some(through)) => {
                        let mut selected = sources
                            .iter()
                            .copied()
                            .filter(|s| e.source_ids.contains(&s.id))
                            .collect::<Vec<_>>();
                        selected.sort_by_key(|s| &s.start);
                        if sources.iter().any(|s| !e.source_ids.contains(&s.id)) {
                            add(format!(
                                "{} has an in-period card statement omitted from coverage review.",
                                e.label
                            ));
                        }
                        let mut next = imports::iso_date(from)?;
                        let mut previous: Option<&Source> = None;
                        let mut fingerprints = HashSet::new();
                        for s in selected {
                            if !s.reconciled {
                                add(format!(
                                    "{} has undecided rows or a reconciliation difference.",
                                    s.file_name
                                ));
                            }
                            if s.start > *through || s.end < *from {
                                continue;
                            }
                            let a = imports::iso_date(&s.start)?;
                            let b = imports::iso_date(&s.end)?;
                            if a > next {
                                add(format!(
                                    "{} has a gap in expected statement coverage.",
                                    e.label
                                ));
                            }
                            if previous.is_some_and(|p| p.end >= s.start) {
                                add(format!("{} has overlapping card statements.", e.label));
                            }
                            if previous.is_some_and(|p| {
                                imports::iso_date(&p.end).ok().and_then(|t| t.succ_opt()) == Some(a)
                                    && p.closing != s.opening
                            }) {
                                add(format!(
                                    "{} has inconsistent consecutive statement balances.",
                                    e.label
                                ));
                            }
                            if b >= next {
                                next = b.succ_opt().ok_or("Invalid card period.")?;
                            }
                            previous = Some(s);
                            for r in &s.rows {
                                if r.status == imports::Status::Approved {
                                    if !r.fields.date.as_deref().is_some_and(|t| {
                                        t >= from.as_str() && t <= through.as_str()
                                    }) {
                                        add(format!("{} has approved activity outside its recorded applicability.",e.label));
                                    }
                                    let key = (
                                        &r.fields.date,
                                        &r.fields.amount,
                                        r.fields.description.to_lowercase(),
                                    );
                                    if !fingerprints.insert(key) {
                                        add(format!("{} has possible duplicate approved card rows requiring resolution.",e.label));
                                    }
                                }
                            }
                        }
                        if next <= imports::iso_date(through)? {
                            add(format!("{} has uncovered expected dates.", e.label));
                        }
                    }
                    _ => add(format!(
                        "{} requires known expected start and end dates.",
                        e.label
                    )),
                }
                if let Some(s) = e
                    .closing_source_id
                    .as_ref()
                    .and_then(|id| c.sources.iter().find(|s| &s.id == id))
                {
                    finding.closing_owed = Some(s.closing.clone());
                    if s.end != end || !s.reconciled {
                        add(format!(
                            "{} needs a reconciled closing statement at the assessment end date.",
                            e.label
                        ));
                    }
                    if let Some(f) = &c.facts {
                        if f.data.as_of.as_deref() != Some(end.as_str()) || !f.data.full_inventory {
                            add("Verified household balances must be a full inventory dated at assessment end.".into());
                        }
                        if let Some(b) = e
                            .balance_label
                            .as_ref()
                            .and_then(|label| f.data.balances.iter().find(|b| &b.label == label))
                        {
                            finding.mapped_balance = b.amount.clone();
                            if let Some(n) = &b.amount {
                                let diff =
                                    imports::signed_money(&s.closing)? - imports::signed_money(n)?;
                                finding.difference = Some(money(diff));
                                if diff != 0 {
                                    add(format!(
                                        "{} differs from its mapped verified debt balance.",
                                        e.label
                                    ));
                                }
                            } else {
                                add(format!(
                                    "{} maps to an unknown debt balance; unknown is not zero.",
                                    e.label
                                ));
                            }
                        } else {
                            add(format!(
                                "{} has no reviewed verified-debt mapping.",
                                e.label
                            ));
                        }
                    } else {
                        add(
                            "No counselor-verified household balance inventory is available."
                                .into(),
                        );
                    }
                } else {
                    add(format!("{} has no reviewed closing statement.", e.label));
                }
            }
        }
        findings.push(finding);
    }
    Ok(Evaluation{calculator:"card-completeness-evidence-v1".into(),period_start:start,period_end:end,necessary_spending:money(necessary),discretionary_spending:money(discretionary),unclassified_spending:money(unclassified),stale_classifications:stale,findings,evidence_complete:issues.is_empty(),issues,disclosures:vec!["These findings check recorded card inventory, declared coverage, spending roles and links to existing verified debt. Mapped balances are compared, never added to household debt or net worth.".into(),"Card refunds reduce their explicitly reviewed spending role. Payments are not spending or income. Unknown, pending, rejected and unsupported records never become verified zero.".into(),"This is internal evidence review, not final approval. Bank completeness, repayment timing/matching, other obligations and professional approval require their own checks. The original bank/card calculator and all planning models remain unchanged.".into(),"Negative card balances are credits and cannot map to nonnegative debt without a future explicit asset treatment. Non-applicability cannot hide a declared in-period statement.".into()]})
}
pub fn validate(s: &Snapshot) -> Result<(), String> {
    if s.format != 1
        || uuid::Uuid::parse_str(&s.client_id).is_err()
        || !(1..=50).contains(&s.revision)
        || s.source_revision < 0
        || chrono::DateTime::parse_from_rfc3339(&s.updated_at).is_err()
        || chrono::DateTime::parse_from_rfc3339(&s.profile_updated_at).is_err()
        || cards::text(&s.counselor_name, 120, true)? != s.counselor_name
        || canonical(s.data.clone(), &s.choices)? != s.data
        || evaluate(&s.data, &s.choices)? != s.evaluation
    {
        return Err("The recorded card completeness evidence is invalid.".into());
    }
    Ok(())
}
