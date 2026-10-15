//! Versioned card identity and repayment evidence. Bank cash and recorded spending are distinct.
use crate::{budget::money, cards, dashboard, imports};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub id: String,
    pub label: String,
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Assignment {
    pub statement_id: String,
    pub account_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Repayment {
    pub statement_id: String,
    pub row: usize,
    pub card_decision_id: String,
    pub bank_record_id: String,
    pub bank_decision_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub format: u32,
    pub client_id: String,
    pub revision: i64,
    pub source_revision: i64,
    pub updated_at: String,
    pub counselor_name: String,
    pub reason: String,
    pub accounts: Vec<Account>,
    pub assignments: Vec<Assignment>,
    pub repayments: Vec<Repayment>,
}
#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    CreateAccount {
        label: String,
    },
    UpdateAccount {
        id: String,
        label: String,
        archived: bool,
    },
    AssignStatement {
        statement_id: String,
        account_id: Option<String>,
    },
    LinkRepayment {
        statement_id: String,
        row: usize,
        bank_record_id: String,
    },
    UnlinkRepayment {
        statement_id: String,
        row: usize,
    },
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub expected_source_revision: i64,
    pub reason: String,
    pub acknowledge: bool,
    pub action: Action,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub client_id: String,
    pub source_revision: i64,
    pub snapshot: Option<Snapshot>,
    pub history: Vec<Snapshot>,
    pub cards: cards::Snapshot,
    pub bank: imports::Snapshot,
    pub stale_repayments: Vec<Repayment>,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Review {
    pub client_id: String,
    pub calculator: String,
    pub source_revision: i64,
    pub link_revision: i64,
    pub period_start: String,
    pub period_end: String,
    pub bank_cash_movement: String,
    pub bank_net_spending: String,
    pub card_net_spending: String,
    pub observed_spending: Option<String>,
    pub card_payments: String,
    pub matched_payments: String,
    pub card_liability_movement: String,
    pub issues: Vec<String>,
    pub disclosures: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capture {
    pub ledger: cards::Snapshot,
    pub links: Snapshot,
    pub review: Review,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub revision: i64,
}
fn uuid(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok()
}
pub fn validate(s: &Snapshot) -> Result<(), String> {
    if s.format != 1
        || !uuid(&s.client_id)
        || !(1..=200).contains(&s.revision)
        || s.source_revision < 1
        || chrono::DateTime::parse_from_rfc3339(&s.updated_at).is_err()
        || cards::text(&s.counselor_name, 120, true)? != s.counselor_name
        || cards::text(&s.reason, 500, true)? != s.reason
        || s.accounts.len() > 20
        || s.assignments.len() > 20
        || s.repayments.len() > 200
    {
        return Err("Invalid or excessive card review history.".into());
    }
    let mut ids = HashSet::new();
    let mut labels = HashSet::new();
    for a in &s.accounts {
        if !uuid(&a.id)
            || !ids.insert(&a.id)
            || cards::text(&a.label, 120, true)? != a.label
            || !labels.insert(a.label.to_lowercase())
        {
            return Err("Choose distinct card account names and identities.".into());
        }
    }
    let mut sources = HashSet::new();
    for a in &s.assignments {
        if !uuid(&a.statement_id)
            || !ids.contains(&a.account_id)
            || !sources.insert(&a.statement_id)
        {
            return Err("Choose one owned card account per statement.".into());
        }
    }
    let mut card_rows = HashSet::new();
    let mut bank_rows = HashSet::new();
    for p in &s.repayments {
        if !sources.contains(&p.statement_id)
            || p.row >= cards::MAX_ROWS
            || !uuid(&p.card_decision_id)
            || !uuid(&p.bank_record_id)
            || !uuid(&p.bank_decision_id)
            || !card_rows.insert((&p.statement_id, p.row))
            || !bank_rows.insert(&p.bank_record_id)
        {
            return Err(
                "Each repayment must link one distinct card row and one distinct bank row.".into(),
            );
        }
    }
    Ok(())
}
/// Historical links bind immutable decisions, not whichever correction happens to be current.
pub fn evidence(
    p: &Repayment,
    bank: &imports::Snapshot,
    ledger: &cards::Snapshot,
) -> Result<(), String> {
    let statement = ledger
        .statements
        .iter()
        .find(|s| s.source.id == p.statement_id)
        .ok_or("Card source is unavailable for this client.")?;
    let card = statement
        .history
        .iter()
        .find(|h| h.id == p.card_decision_id && h.row == p.row)
        .ok_or("Card approval is unavailable.")?;
    let b = bank
        .records
        .iter()
        .find(|r| r.id == p.bank_record_id)
        .ok_or("Bank record is unavailable for this client.")?;
    let h = bank
        .history
        .iter()
        .find(|h| h.id == p.bank_decision_id && h.record_id == b.id)
        .ok_or("Bank approval is unavailable.")?;
    if card.status != imports::Status::Approved
        || card.fields.kind != cards::Kind::Payment
        || h.to_status != imports::Status::Approved
        || h.fields.kind != imports::Kind::BalanceMovement
    {
        return Err("Match an approved card payment to an approved bank balance movement. Review the bank classification first; linked bank transfers cannot be reused.".into());
    }
    let n = imports::signed_money(
        card.fields
            .amount
            .as_deref()
            .ok_or("Missing card amount.")?,
    )?;
    let m = imports::signed_money(h.fields.amount.as_deref().ok_or("Missing bank amount.")?)?;
    let d = imports::iso_date(card.fields.date.as_deref().ok_or("Missing card date.")?)?;
    let e = imports::iso_date(h.fields.date.as_deref().ok_or("Missing bank date.")?)?;
    if n >= 0 || n != m || (d - e).num_days().abs() > 7 {
        return Err("Repayments require equal negative USD amounts and dates within seven days. Split or partial payments need a future workflow.".into());
    }
    Ok(())
}
pub fn current(p: &Repayment, bank: &imports::Snapshot, ledger: &cards::Snapshot) -> bool {
    evidence(p, bank, ledger).is_ok()
        && bank
            .records
            .iter()
            .find(|b| b.id == p.bank_record_id)
            .is_some_and(|b| b.linked_id.is_none())
        && bank
            .history
            .iter()
            .rev()
            .find(|h| h.record_id == p.bank_record_id)
            .is_some_and(|h| h.id == p.bank_decision_id)
        && ledger
            .statements
            .iter()
            .find(|s| s.source.id == p.statement_id)
            .and_then(|s| s.history.iter().rev().find(|h| h.row == p.row))
            .is_some_and(|h| h.id == p.card_decision_id)
}
// Unlike entered amounts, calculated totals may exceed the per-row USD limit.
fn cents(s: &str) -> Result<i64, String> {
    s.replace('.', "")
        .parse()
        .map_err(|_| "Invalid calculated money total.".into())
}
pub fn calculate(
    bank: &imports::Snapshot,
    ledger: &cards::Snapshot,
    links: &Snapshot,
    input: dashboard::Input,
) -> Result<Review, String> {
    validate(links)?;
    if bank.client_id != ledger.client_id
        || bank.client_id != links.client_id
        || bank.revision != ledger.revision
        || links.source_revision > bank.revision
    {
        return Err("Card review context does not match this client and source revision.".into());
    }
    let (start, end, _) = dashboard::period(&input)?;
    let start = start.to_string();
    let end = end.to_string();
    let inside =
        |date: Option<&str>| date.is_some_and(|d| d >= start.as_str() && d <= end.as_str());
    let mut issues = vec![];
    let mut add = |v: String| {
        if !issues.contains(&v) {
            issues.push(v)
        }
    };
    let bank_rows = bank
        .records
        .iter()
        .filter(|r| inside(r.fields.date.as_deref()))
        .collect::<Vec<_>>();
    let totals = imports::summarize_selected(&bank_rows, &bank.accounts)?;
    if totals.pending + totals.needs_clarification > 0
        || bank.records.iter().any(|r| r.fields.date.is_none())
    {
        add("Bank activity has undecided or undated rows. Combined observed spending is unavailable.".into());
    }
    let mut spending = 0;
    let mut payments = 0;
    let mut matched = 0;
    let mut movement = 0;
    let sources = ledger
        .statements
        .iter()
        .filter(|s| s.source.input.period_start <= end && s.source.input.period_end >= start)
        .collect::<Vec<_>>();
    if sources.is_empty() {
        add("No card statement covers this assessment period.".into());
    }
    for s in &sources {
        if !links
            .assignments
            .iter()
            .any(|a| a.statement_id == s.source.id)
        {
            add(format!(
                "Assign {} to a reviewed card account before combining it.",
                s.source.file_name
            ));
        }
        if !s.summary.reconciled {
            add(format!(
                "{} has undecided rows or a statement reconciliation difference.",
                s.source.file_name
            ));
        }
        for row in &s.rows {
            if row.status != imports::Status::Approved || !inside(row.fields.date.as_deref()) {
                continue;
            }
            let n = imports::signed_money(
                row.fields
                    .amount
                    .as_deref()
                    .ok_or("Missing approved card amount.")?,
            )?;
            movement += n;
            match row.fields.kind {
                cards::Kind::Purchase
                | cards::Kind::Fee
                | cards::Kind::Interest
                | cards::Kind::Refund => spending += n,
                cards::Kind::Payment => {
                    payments -= n;
                    if links
                        .repayments
                        .iter()
                        .find(|p| p.statement_id == s.source.id && p.row == row.row)
                        .is_some_and(|p| {
                            current(p, bank, ledger)
                                && bank
                                    .records
                                    .iter()
                                    .find(|b| b.id == p.bank_record_id)
                                    .is_some_and(|b| inside(b.fields.date.as_deref()))
                        })
                    {
                        matched -= n;
                    } else {
                        add(format!(
                            "{} row {} has an unmatched, stale or out-of-period bank repayment.",
                            s.source.file_name, row.raw.record_number
                        ));
                    }
                }
                cards::Kind::Adjustment => add(format!(
                    "{} row {} has an adjustment requiring future spending treatment.",
                    s.source.file_name, row.raw.record_number
                )),
                cards::Kind::Unclassified => {
                    return Err("Invalid approved card classification.".into())
                }
            }
        }
    }
    for p in &links.repayments {
        if sources.iter().any(|s| s.source.id == p.statement_id) && !current(p, bank, ledger) {
            add("A repayment link refers to a superseded approval. Unlink or explicitly review and match it again.".into());
        }
    }
    for a in &links.accounts {
        let mut periods = sources
            .iter()
            .filter(|s| {
                links
                    .assignments
                    .iter()
                    .any(|x| x.statement_id == s.source.id && x.account_id == a.id)
            })
            .copied()
            .collect::<Vec<_>>();
        if a.archived && periods.is_empty() {
            continue;
        }
        periods.sort_by_key(|s| &s.source.input.period_start);
        let mut next = imports::iso_date(&start)?;
        let mut previous: Option<&cards::Statement> = None;
        for s in periods {
            let from = imports::iso_date(&s.source.input.period_start)?;
            let through = imports::iso_date(&s.source.input.period_end)?;
            if from > next {
                add(format!(
                    "{} has a gap in declared card statement coverage.",
                    a.label
                ));
            }
            if previous.is_some_and(|p| s.source.input.period_start <= p.source.input.period_end) {
                add(format!(
                    "{} has overlapping card statements; duplicate spending is possible.",
                    a.label
                ));
            }
            if previous.is_some_and(|p| {
                imports::iso_date(&p.source.input.period_end)
                    .ok()
                    .and_then(|d| d.succ_opt())
                    == Some(from)
                    && p.source.input.closing_owed != s.source.input.opening_owed
            }) {
                add(format!(
                    "{} has inconsistent consecutive statement balances.",
                    a.label
                ));
            }
            if through >= next {
                next = through.succ_opt().ok_or("Invalid statement end.")?;
            }
            previous = Some(s);
        }
        if next <= imports::iso_date(&end)? {
            add(format!(
                "{} does not cover the full assessment period.",
                a.label
            ));
        }
    }
    let bank_spending = cents(&totals.net_spending)?;
    Ok(Review{client_id:bank.client_id.clone(),calculator:"bank-card-review-v1".into(),source_revision:bank.revision,link_revision:links.revision,period_start:start,period_end:end,
 bank_cash_movement:totals.net_cash,bank_net_spending:totals.net_spending,card_net_spending:money(spending),observed_spending:issues.is_empty().then(||money(bank_spending+spending)),card_payments:money(payments),matched_payments:money(matched),card_liability_movement:money(movement),issues,
 disclosures:vec!["Approved imported activity only. Bank cash movement includes repayment outflows; card spending includes purchases, fees and interest less refunds. Payments are excluded from spending and never treated as income.".into(),"Combined observed spending is a provisional subtotal, not household completeness, a budget or a finalized assessment. Verify bank payment classifications and missing accounts against originals. Card coverage checks use declared dates only.".into(),"Card liability changes are not total household debt or net worth. They are not added to manually verified balances, which may already include these cards. Planning models and client action-plan calculations remain unchanged.".into(),"Final report approval remains blocked until a versioned card-inclusive completeness policy is implemented. No automatic matching or classification occurs.".into()]})
}
