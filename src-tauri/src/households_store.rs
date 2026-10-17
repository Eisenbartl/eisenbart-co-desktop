//! One encrypted, counselor-scoped append-only journal per household. Never copies source rows.
use super::*;
use crate::households::{self as h, Action, Data};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryItem {
    pub id: String,
    pub name: String,
    pub archived: bool,
    pub members: usize,
    pub revision: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub revision: i64,
    pub data: Data,
    pub summary: h::Summary,
    pub history: Vec<h::History>,
    pub cashflow: Option<cashflow::Summary>,
    pub cashflow_warnings: Vec<String>,
    pub goal_results: Vec<savings::GoalResult>,
    pub goal_shares: Vec<Vec<h::Portion>>,
    pub source_rows: Vec<LinkedRow>,
    pub actual_mix: Vec<h::Category>,
    pub actual_total: String,
    pub actual_necessary: String,
    pub actual_discretionary: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedRow {
    pub member_id: String,
    pub payment_id: String,
    pub client_id: String,
    pub row: spending_analysis::Row,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetPreview {
    pub client_id: String,
    pub household_revision: i64,
    pub budget: ClientBudget,
    pub proposed_lines: Vec<BudgetLine>,
    pub suggested_replacements: Vec<usize>,
    pub warnings: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetChange {
    pub expected_revision: i64,
    pub budget_revision: i64,
    pub member_id: String,
    pub month: String,
    pub replace_indices: Vec<usize>,
    pub acknowledge: bool,
}
fn digest(previous: &str, json: &str, at: &str) -> String {
    format!("{:x}", Sha256::digest(format!("{previous}\n{at}\n{json}")))
}
fn ids(c: &Connection, w: &WorkspaceInfo) -> Result<Vec<String>, String> {
    let mut q=c.prepare("SELECT DISTINCT household_id FROM household_events WHERE organization_id=?1 AND counselor_id=?2 ORDER BY household_id LIMIT 501").map_err(storage_error)?;
    let ids = q
        .query_map(params![w.organization_id, w.counselor_id], |r| r.get(0))
        .map_err(storage_error)?
        .collect::<rusqlite::Result<Vec<String>>>()
        .map_err(storage_error)?;
    if ids.len() > 500 {
        return Err("This workspace exceeds the 500-household recovery limit.".into());
    }
    Ok(ids)
}
fn read(
    c: &Connection,
    w: &WorkspaceInfo,
    id: &str,
    through: Option<i64>,
) -> Result<(Data, i64, Vec<h::History>, String), String> {
    Uuid::parse_str(id).map_err(|_| "Household unavailable.")?;
    let mut q=c.prepare("SELECT revision,action_json,digest,saved_at FROM household_events WHERE household_id=?1 AND organization_id=?2 AND counselor_id=?3 AND revision<=?4 ORDER BY revision LIMIT 10001").map_err(storage_error)?;
    let rows = q
        .query_map(
            params![
                id,
                w.organization_id,
                w.counselor_id,
                through.unwrap_or(10000)
            ],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        )
        .map_err(storage_error)?;
    let mut data = Data::new(String::new());
    let mut revision = 0;
    let mut previous = String::new();
    let mut history = vec![];
    let mut bytes = 0;
    for row in rows {
        let (rev, json, hash, at) = row.map_err(storage_error)?;
        bytes += json.len();
        if rev != revision + 1
            || rev > 10000
            || bytes > 32 * 1024 * 1024
            || json.len() > 1048576
            || chrono::DateTime::parse_from_rfc3339(&at).is_err()
            || digest(&previous, &json, &at) != hash
        {
            return Err(STORAGE_ERROR.into());
        }
        let mut action: Action = serde_json::from_str(&json).map_err(|_| STORAGE_ERROR)?;
        if (rev == 1) != matches!(action, Action::Create(_)) {
            return Err(STORAGE_ERROR.into());
        }
        h::apply(&mut data, &mut action).map_err(|_| STORAGE_ERROR)?;
        if serde_json::to_string(&action).map_err(|_| STORAGE_ERROR)? != json {
            return Err(STORAGE_ERROR.into());
        }
        let value = serde_json::to_value(&action).map_err(|_| STORAGE_ERROR)?;
        history.push(h::History {
            revision: rev,
            at,
            action: value["action"].as_str().unwrap_or("change").into(),
            note: match &action {
                Action::Expense(e) => format!("{} · effective {}", e.name, e.effective),
                Action::Rule(r) => format!("{} · effective {}", r.category, r.effective),
                Action::Member(m) => m.name.clone(),
                Action::Payment(p) => format!("{} · {} · {}", p.kind, p.date, p.state),
                Action::BudgetReconciled {
                    budget_revision,
                    month,
                    ..
                } => format!("Budget revision {budget_revision} · {month}"),
                _ => String::new(),
            },
        });
        previous = hash;
        revision = rev;
    }
    if revision == 0 {
        return Err("Household unavailable.".into());
    }
    if through.is_some_and(|r| r != revision) {
        return Err("Household revision unavailable.".into());
    }
    Ok((data, revision, history, previous))
}
fn append(
    c: &Transaction<'_>,
    w: &WorkspaceInfo,
    id: &str,
    revision: i64,
    previous: &str,
    action: &Action,
) -> Result<(), String> {
    let json = serde_json::to_string(action).map_err(|_| STORAGE_ERROR)?;
    if json.len() > 1048576 {
        return Err("This household change exceeds the 1 MiB record limit.".into());
    }
    let at = timestamp();
    c.execute("INSERT INTO household_events(household_id,organization_id,counselor_id,revision,action_json,digest,saved_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,w.organization_id,w.counselor_id,revision,json,digest(previous,&json,&at),at]).map_err(storage_error)?;
    audit(c, w, "household_changed", Some(id))
}
pub(super) fn validate_all(c: &Connection, w: &WorkspaceInfo) -> Result<(), String> {
    for id in ids(c, w)? {
        let (d, _, _, _) = read(c, w, &id, None)?;
        for m in &d.members {
            if let Some(client) = &m.client_id {
                scoped_client(c, w, client)?;
            }
        }
    }
    Ok(())
}
fn permissions(d: &Data) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(
                &d.members
                    .iter()
                    .map(|m| (&m.id, &m.consent))
                    .collect::<Vec<_>>()
            )
            .unwrap_or_default()
        )
    )
}
fn consent_member<'a>(d: &'a Data, member_id: &str, kind: &str) -> Result<&'a h::Member, String> {
    let m = d
        .members
        .iter()
        .find(|m| m.id == member_id)
        .ok_or("Member unavailable.")?;
    let allowed = match kind {
        "transactions" => m.consent.transactions,
        "planning" => m.consent.planning,
        _ => false,
    };
    if !allowed {
        return Err(format!("{} has not authorized {kind} sharing.", m.name));
    }
    Ok(m)
}
fn authorized_source(
    c: &Connection,
    w: &WorkspaceInfo,
    d: &Data,
    p: &h::Payment,
) -> Result<spending_analysis::Row, String> {
    let source = p.source.as_ref().ok_or("Source unavailable.")?;
    let m = consent_member(d, &p.from_member, "transactions")?;
    if m.client_id.as_ref() != Some(&source.client_id) {
        return Err("The source belongs to a different client.".into());
    }
    let month = p.date.get(..7).ok_or("Invalid payment date.")?;
    let row = spending_analysis::household_rows(c, w, &source.client_id, month)?
        .into_iter()
        .find(|r| r.id == source.row_id && r.approval_id == source.approval_id)
        .ok_or("The source approval changed or is no longer eligible.")?;
    let signed = crate::imports::signed_money(&row.amount)?;
    let expected = crate::budget::parse_money(&p.amount)?;
    let kind_ok = match p.kind.as_str() {
        "bill" => {
            ["necessary", "discretionary", "unclassified", "debtPayment"]
                .contains(&row.role.as_str())
                && signed < 0
        }
        "refund" => row.role == "reimbursement" && signed > 0,
        _ => row.role == "transfer" && signed < 0,
    };
    if !kind_ok || signed.abs() != expected || row.date != p.date {
        return Err(
            "The approved source amount, date or transaction type does not match this payment."
                .into(),
        );
    }
    Ok(row)
}
fn live(
    c: &Connection,
    w: &WorkspaceInfo,
    d: &Data,
) -> Result<(Data, Vec<LinkedRow>, Vec<String>), String> {
    let mut live = d.clone();
    let mut rows = vec![];
    let mut warnings = vec![];
    for p in &mut live.payments {
        if p.source.is_some() && !d.voids.contains(&p.id) {
            match authorized_source(c, w, d, p) {
                Ok(row) => {
                    if p.state == "confirmed" {
                        rows.push(LinkedRow {
                            member_id: p.from_member.clone(),
                            payment_id: p.id.clone(),
                            client_id: p
                                .source
                                .as_ref()
                                .ok_or("Source unavailable.")?
                                .client_id
                                .clone(),
                            row,
                        })
                    }
                }
                Err(_) => {
                    p.state = "recorded".into();
                    warnings.push("A linked payment has restricted or changed source evidence. Reconcile it before counting it as confirmed.".into())
                }
            }
        }
    }
    for a in &mut live.accounts {
        if a.owners.iter().any(|id| {
            d.members
                .iter()
                .find(|m| &m.id == id)
                .is_none_or(|m| !m.consent.balances)
        }) {
            a.opening_balance = None;
            a.verified = false;
        }
    }
    Ok((live, rows, warnings))
}
fn cashflow(d: &Data, s: &h::Summary) -> Result<(Option<cashflow::Summary>, Vec<String>), String> {
    let (start, end) = h::bounds(&s.month)?;
    let mut warnings=vec!["Projected balances use explicitly shared opening balances and dated plans. No private paydays are inferred. Account attribution does not change expense responsibility.".into()];
    if d.accounts.is_empty() {
        warnings.push(
            "Add authorized accounts and dated balances to model payer timing and shortfalls."
                .into(),
        );
        return Ok((None, warnings));
    }
    let mut schedules = vec![];
    for b in &s.bills {
        let already: i64 = d
            .payments
            .iter()
            .filter(|p| {
                p.expense_id.as_ref() == Some(&b.agreement_id)
                    && p.period == s.month
                    && !d.voids.contains(&p.id)
            })
            .map(|p| {
                crate::budget::parse_money(&p.amount).unwrap_or(0)
                    * if p.kind == "refund" { -1 } else { 1 }
            })
            .sum(); // one payment pool per agreement; consume sequentially below
        let prior: i64 = s
            .bills
            .iter()
            .take_while(|x| !std::ptr::eq(*x, b))
            .filter(|x| x.agreement_id == b.agreement_id)
            .map(|x| crate::budget::parse_money(&x.amount).unwrap_or(0))
            .sum();
        let remaining = (crate::budget::parse_money(&b.amount)? - (already - prior).max(0)).max(0);
        if remaining > 0 {
            if let Some(account) = &b.account_id {
                schedules.push(cashflow::Schedule {
                    label: format!("{} · planned remainder", b.name),
                    account_id: account.clone(),
                    destination_id: None,
                    kind: cashflow::Kind::Bill,
                    amount: Some(budget::money(remaining)),
                    first_date: b.date.clone(),
                    frequency: cashflow::Frequency::Once,
                    second_day: None,
                    until_date: None,
                    certainty: cashflow::Certainty::Estimated,
                });
            } else {
                warnings.push(format!(
                    "{} has no payment account; payer timing is incomplete.",
                    b.name
                ))
            }
        }
    }
    for p in d.payments.iter().filter(|p| {
        !d.voids.contains(&p.id) && p.date >= start.to_string() && p.date <= end.to_string()
    }) {
        if let Some(account) = &p.account_id {
            let transfer = ["settlement", "contribution", "transfer"].contains(&p.kind.as_str())
                && p.destination_account.is_some();
            schedules.push(cashflow::Schedule {
                label: format!("{} · {}", p.kind, p.state),
                account_id: account.clone(),
                destination_id: if transfer {
                    p.destination_account.clone()
                } else {
                    None
                },
                kind: if transfer {
                    cashflow::Kind::Transfer
                } else if p.kind == "income" {
                    cashflow::Kind::Paycheck
                } else if p.kind == "refund" {
                    cashflow::Kind::OtherInflow
                } else {
                    cashflow::Kind::OtherOutflow
                },
                amount: Some(p.amount.clone()),
                first_date: p.date.clone(),
                frequency: cashflow::Frequency::Once,
                second_day: None,
                until_date: None,
                certainty: if p.state == "confirmed" {
                    cashflow::Certainty::Confirmed
                } else {
                    cashflow::Certainty::Estimated
                },
            })
        } else {
            warnings.push(format!("A {} on {} has no source account.", p.kind, p.date))
        }
    }
    let accounts=d.accounts.iter().map(|a|{if a.balance_date!=start.to_string(){warnings.push(format!("{} needs an opening balance dated {}. Its balance is unknown in this projection.",a.label,start))}cashflow::Account{id:a.id.clone(),label:a.label.clone(),opening_balance:if a.balance_date==start.to_string(){a.opening_balance.clone()}else{None}}}).collect();
    let plan = cashflow::PlanData {
        start_date: start.to_string(),
        end_date: end.to_string(),
        accounts,
        schedules,
        notes: String::new(),
    };
    match cashflow::calculate(plan) {
        Ok((_, result)) => Ok((Some(result), warnings)),
        Err(e) => {
            warnings.push(e);
            Ok((None, warnings))
        }
    }
}
fn record(c: &Connection, w: &WorkspaceInfo, id: &str, month: &str) -> Result<Record, String> {
    let (d, revision, history, _) = read(c, w, id, None)?;
    let (mut display, rows, warnings) = live(c, w, &d)?;
    let mut summary = h::summary(&display, month)?;
    summary.warnings.extend(warnings);
    let (cf, cfw) = cashflow(&display, &summary)?;
    let goal_data = savings::PlanData {
        start_month: month.into(),
        target_months: 3,
        liquid_savings: Some(budget::money(
            display
                .goals
                .iter()
                .map(|g| budget::parse_money(&g.reserved).unwrap_or(0))
                .sum(),
        )),
        restricted_savings: None,
        obligations_reserved: None,
        emergency_contribution: None,
        alternative_contributions: vec![],
        goals: display
            .goals
            .iter()
            .map(|g| savings::Goal {
                label: g.label.clone(),
                target_amount: Some(g.target.clone()),
                target_date: g.target_date.clone(),
                reserved_amount: Some(g.reserved.clone()),
                monthly_contribution: Some(g.monthly.clone()),
            })
            .collect(),
        priorities_note: String::new(),
    };
    let context=savings::Context{budget_revision:revision,debt_revision:0,necessary_expenses:Some(summary.necessary.clone()),monthly_surplus:None,budget_savings:None,budget_debt_payments:None,debt_start_month:None,debt_monthly_payment:None,debt_changes:vec![],debt_extras:vec![],warnings:vec!["Only explicitly shared household goals are included. Personal assets and debts remain separate.".into()]};
    let (_, goals) = savings::calculate(goal_data, &context)?;
    let mut goal_shares = vec![];
    for g in &display.goals {
        match h::split(
            budget::parse_money(&g.monthly)?,
            &g.allocation,
            &display,
            &format!("{month}-01"),
        ) {
            Ok(shares) => goal_shares.push(shares),
            Err(e) => {
                summary.warnings.push(format!("{}: {e}", g.label));
                goal_shares.push(vec![])
            }
        }
    }
    let permitted = permissions(&d);
    for report in &mut display.reports {
        if report.permissions != permitted {
            report.body = String::new();
        }
    }
    let mut categories = std::collections::BTreeMap::<String, i64>::new();
    let (mut total, mut necessary, mut discretionary) = (0, 0, 0);
    for linked in rows.iter().filter(|r| r.row.date.starts_with(month)) {
        let p = d
            .payments
            .iter()
            .find(|p| p.id == linked.payment_id)
            .ok_or("Payment unavailable.")?;
        if !["bill", "refund"].contains(&p.kind.as_str()) {
            continue;
        }
        let c = budget::parse_money(&p.amount)? * if p.kind == "refund" { -1 } else { 1 };
        total += c;
        *categories.entry(linked.row.category.clone()).or_default() += c;
        if linked.row.role == "necessary" {
            necessary += c
        } else if linked.row.role == "discretionary" {
            discretionary += c
        }
    }
    let actual_mix = categories
        .into_iter()
        .map(|(label, c)| h::Category {
            label,
            amount: budget::money(c),
            percent: if total > 0 && c > 0 {
                ((i128::from(c) * 10000) / i128::from(total)).min(10000) as u16
            } else {
                0
            },
        })
        .collect();
    Ok(Record {
        id: id.into(),
        revision,
        data: display,
        summary,
        history,
        cashflow: cf,
        cashflow_warnings: cfw,
        goal_results: goals.goals,
        goal_shares,
        source_rows: rows
            .into_iter()
            .filter(|r| r.row.date.starts_with(month))
            .collect(),
        actual_mix,
        actual_total: budget::money(total),
        actual_necessary: budget::money(necessary),
        actual_discretionary: budget::money(discretionary),
    })
}
impl Workspace {
    pub fn list_households(&mut self) -> Result<Vec<DirectoryItem>, String> {
        let s = self.session()?;
        ids(&s.connection, &s.info)?
            .into_iter()
            .map(|id| {
                let (d, revision, _, _) = read(&s.connection, &s.info, &id, None)?;
                Ok(DirectoryItem {
                    id,
                    name: d.name,
                    archived: d.archived,
                    members: d.members.len(),
                    revision,
                })
            })
            .collect()
    }
    pub fn create_household(&mut self, name: String, month: String) -> Result<Record, String> {
        h::bounds(&month)?;
        let s = self.session()?;
        if ids(&s.connection, &s.info)?.len() >= 500 {
            return Err("Household limit reached.".into());
        }
        let id = Uuid::new_v4().to_string();
        let mut action = Action::Create(name);
        h::apply(&mut Data::new(String::new()), &mut action)?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        append(&tx, &s.info, &id, 1, "", &action)?;
        let result = record(&tx, &s.info, &id, &month)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn get_household(&mut self, id: &str, month: &str) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let r = record(&tx, &s.info, id, month)?;
        tx.commit().map_err(storage_error)?;
        Ok(r)
    }
    pub fn save_household(
        &mut self,
        id: &str,
        month: &str,
        mut input: h::Input,
    ) -> Result<Record, String> {
        h::bounds(month)?;
        if matches!(
            input.action,
            Action::Create(_) | Action::Report(_) | Action::BudgetReconciled { .. }
        ) {
            return Err("Use the dedicated creation, report or budget workflow.".into());
        }
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let (mut d, revision, _, previous) = read(&tx, &s.info, id, None)?;
        if revision != input.expected_revision {
            return Err("Household changed. Reload before saving; nothing was changed.".into());
        }
        if let Action::Member(m) = &input.action {
            if let Some(c) = &m.client_id {
                scoped_client(&tx, &s.info, c)?;
            }
        }
        if let Action::Payment(p) = &input.action {
            if p.source.is_some() {
                authorized_source(&tx, &s.info, &d, p)?;
                for other in ids(&tx, &s.info)? {
                    let (other, _, _, _) = read(&tx, &s.info, &other, None)?;
                    if other
                        .payments
                        .iter()
                        .filter(|p| !other.voids.contains(&p.id))
                        .any(|x| {
                            x.source.as_ref().is_some_and(|x| {
                                p.source.as_ref().is_some_and(|y| {
                                    x.client_id == y.client_id && x.row_id == y.row_id
                                })
                            })
                        })
                    {
                        return Err("This source transaction is already linked to an active household payment. Void or reconcile the existing link first.".into());
                    }
                }
            }
        }
        h::apply(&mut d, &mut input.action)?;
        for m in &d.members {
            if let Some(c) = &m.client_id {
                scoped_client(&tx, &s.info, c)?;
            }
        }
        h::summary(&d, month)?;
        append(&tx, &s.info, id, revision + 1, &previous, &input.action)?;
        let result = record(&tx, &s.info, id, month)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn preview_household(
        &mut self,
        id: &str,
        month: &str,
        mut input: h::Input,
    ) -> Result<h::Summary, String> {
        let s = self.session()?;
        let (mut d, revision, _, _) = read(&s.connection, &s.info, id, None)?;
        if revision != input.expected_revision {
            return Err("Household changed. Reload before previewing.".into());
        }
        if !matches!(
            input.action,
            Action::Expense(_)
                | Action::Rule(_)
                | Action::Proposal(_)
                | Action::AcceptProposal(_)
                | Action::Member(_)
        ) {
            return Err("Choose an agreement or scenario to preview.".into());
        }
        h::apply(&mut d, &mut input.action)?;
        if let Action::Proposal(p) = input.action {
            h::apply(&mut d, &mut Action::AcceptProposal(p.id))?
        }
        h::summary(&d, month)
    }
    pub fn household_sources(
        &mut self,
        id: &str,
        member_id: &str,
        month: &str,
    ) -> Result<Vec<spending_analysis::Row>, String> {
        let s = self.session()?;
        let (d, _, _, _) = read(&s.connection, &s.info, id, None)?;
        let m = consent_member(&d, member_id, "transactions")?;
        spending_analysis::household_rows(
            &s.connection,
            &s.info,
            m.client_id
                .as_deref()
                .ok_or("Link an individual client to browse approved sources.")?,
            month,
        )
    }
    pub fn household_classify(
        &mut self,
        id: &str,
        member_id: &str,
        input: spending_analysis::Change,
    ) -> Result<(), String> {
        let client = {
            let s = self.session()?;
            let (d, _, _, _) = read(&s.connection, &s.info, id, None)?;
            let m = consent_member(&d, member_id, "transactions")?;
            if !d.payments.iter().any(|p| {
                !d.voids.contains(&p.id)
                    && p.from_member == member_id
                    && p.source.as_ref().is_some_and(|r| {
                        r.row_id == input.row_id && r.approval_id == input.approval_id
                    })
            }) {
                return Err("Choose a current linked household source.".into());
            }
            m.client_id.clone().ok_or("Client unavailable.")?
        };
        self.change_spending_classification(&client, input)
    }
    pub fn record_household_report(
        &mut self,
        id: &str,
        month: &str,
        expected: i64,
    ) -> Result<Record, String> {
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let (d, revision, _, previous) = read(&tx, &s.info, id, None)?;
        if revision != expected {
            return Err("Household changed. Reload before recording a report.".into());
        }
        let current = record(&tx, &s.info, id, month)?;
        let body=serde_json::to_string(&serde_json::json!({"name":d.name,"summary":current.summary,"actualTotal":current.actual_total,"actualNecessary":current.actual_necessary,"actualDiscretionary":current.actual_discretionary,"sourceRows":current.source_rows,"cashflow":current.cashflow,"warnings":current.cashflow_warnings})).map_err(|_|STORAGE_ERROR)?;
        let action = Action::Report(h::Report {
            id: Uuid::new_v4().to_string(),
            month: month.into(),
            revision,
            saved_at: timestamp(),
            permissions: permissions(&d),
            body,
        });
        append(&tx, &s.info, id, revision + 1, &previous, &action)?;
        let result = record(&tx, &s.info, id, month)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}
fn budget_preview(
    c: &Connection,
    w: &WorkspaceInfo,
    id: &str,
    member: &str,
    month: &str,
) -> Result<BudgetPreview, String> {
    let (d, revision, _, _) = read(c, w, id, None)?;
    let m = consent_member(&d, member, "planning")?;
    let client = m
        .client_id
        .as_deref()
        .ok_or("Link an individual client before reconciling a budget.")?;
    scoped_client(c, w, client)?;
    let budget = read_budget(c, w, client)?;
    let summary = h::summary(&d, month)?;
    let prefix = format!("HH:{}:", &id[..8]);
    let mut grouped = std::collections::BTreeMap::<(String, String, String), i64>::new();
    for b in &summary.bills {
        if b.status != "confirmed" {
            continue;
        }
        if let Some(p) = b.shares.iter().find(|p| p.member_id == member) {
            *grouped
                .entry((b.agreement_id.clone(), b.name.clone(), b.purpose.clone()))
                .or_default() += budget::parse_money(&p.amount)?;
        }
    }
    let proposed_lines: Vec<BudgetLine> = grouped
        .into_iter()
        .map(|((agreement, name, purpose), c)| BudgetLine {
            label: format!(
                "{prefix}{} {}",
                &agreement[..8],
                name.chars().take(80).collect::<String>()
            ),
            kind: match purpose.as_str() {
                "necessary" => budget::Kind::Necessary,
                "sharedDebt" => budget::Kind::DebtPayment,
                _ => budget::Kind::Discretionary,
            },
            amount: Some(budget::money(c)),
            frequency: budget::Frequency::Monthly,
            estimated: false,
        })
        .collect();
    let mut warnings=vec!["Only confirmed agreements in the selected month are included. Review existing personal lines and other households to prevent duplication. Full bill payments and reimbursements are not additional budget expenses.".into()];
    let suggested_replacements = budget
        .lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.label.starts_with(&prefix))
        .map(|(i, _)| i)
        .collect();
    if proposed_lines.is_empty() {
        warnings.push("There are no confirmed household shares to reconcile in this month.".into())
    }
    Ok(BudgetPreview {
        client_id: client.into(),
        household_revision: revision,
        budget,
        proposed_lines,
        suggested_replacements,
        warnings,
    })
}
impl Workspace {
    pub fn household_budget_preview(
        &mut self,
        id: &str,
        member: &str,
        month: &str,
    ) -> Result<BudgetPreview, String> {
        let s = self.session()?;
        budget_preview(&s.connection, &s.info, id, member, month)
    }
    pub fn household_budget_reconcile(
        &mut self,
        id: &str,
        input: BudgetChange,
    ) -> Result<Record, String> {
        if !input.acknowledge {
            return Err("Confirm that existing budget entries have been checked for overlapping obligations.".into());
        }
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let (d, revision, _, previous) = read(&tx, &s.info, id, None)?;
        if d.archived {
            return Err("Reopen the household before reconciling a budget.".into());
        }
        let preview = budget_preview(&tx, &s.info, id, &input.member_id, &input.month)?;
        if revision != input.expected_revision || preview.budget.revision != input.budget_revision {
            return Err(
                "Household or individual budget changed. Preview again; nothing was changed."
                    .into(),
            );
        }
        let selected: BTreeSet<_> = input.replace_indices.iter().copied().collect();
        if selected.len() != input.replace_indices.len()
            || selected.iter().any(|i| {
                *i >= preview.budget.lines.len()
                    || !matches!(
                        preview.budget.lines[*i].kind,
                        budget::Kind::Necessary
                            | budget::Kind::Discretionary
                            | budget::Kind::DebtPayment
                    )
            })
        {
            return Err("Choose distinct existing expense rows to replace; income and savings cannot be removed here.".into());
        }
        let prefix = format!("HH:{}:", &id[..8]);
        if preview
            .budget
            .lines
            .iter()
            .enumerate()
            .any(|(i, l)| l.label.starts_with(&prefix) && !selected.contains(&i))
        {
            return Err("Select all prior shares from this household for replacement to prevent double counting.".into());
        }
        let before = serde_json::to_string(&preview.budget).map_err(|_| STORAGE_ERROR)?;
        let mut lines: Vec<_> = preview
            .budget
            .lines
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !selected.contains(i))
            .map(|(_, l)| l)
            .collect();
        lines.extend(preview.proposed_lines);
        let (lines, _) = budget::calculate(lines)?;
        let after = serde_json::to_string(&lines).map_err(|_| STORAGE_ERROR)?;
        let next = input.budget_revision + 1;
        tx.execute("INSERT INTO client_budgets(client_id,organization_id,counselor_id,revision,lines_json,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(client_id) DO UPDATE SET revision=excluded.revision,lines_json=excluded.lines_json,updated_at=excluded.updated_at WHERE client_budgets.organization_id=excluded.organization_id AND client_budgets.counselor_id=excluded.counselor_id",params![preview.client_id,s.info.organization_id,s.info.counselor_id,next,after,timestamp()]).map_err(storage_error)?;
        audit(&tx, &s.info, "budget_saved", Some(&preview.client_id))?;
        append(
            &tx,
            &s.info,
            id,
            revision + 1,
            &previous,
            &Action::BudgetReconciled {
                client_id: preview.client_id,
                before,
                after,
                budget_revision: next,
                month: input.month.clone(),
            },
        )?;
        let result = record(&tx, &s.info, id, &input.month)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::super::tests::{client_input, configured, PASSWORD};
    use super::*;
    use crate::households::tests::{data, expense, member, payment};
    fn save(w: &mut Workspace, r: Record, action: Action) -> Record {
        w.save_household(
            &r.id,
            "2026-10",
            h::Input {
                expected_revision: r.revision,
                action,
            },
        )
        .unwrap()
    }
    #[test]
    fn journal_scope_budget_conflicts_history_reopen_and_multiple_households() {
        let (dir, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let mut r = w
            .create_household("Oak Street".into(), "2026-10".into())
            .unwrap();
        let mut m = member(1);
        m.client_id = Some(client.id.clone());
        m.consent.planning = true;
        m.consent.evidence = "Synthetic consent".into();
        r = save(&mut w, r, Action::Member(m));
        for n in 2..=3 {
            r = save(&mut w, r, Action::Member(member(n)))
        }
        let e = expense(&r.data);
        r = save(&mut w, r, Action::Expense(e.clone()));
        let preview = w
            .household_budget_preview(&r.id, &r.data.members[0].id, "2026-10")
            .unwrap();
        assert_eq!(preview.proposed_lines[0].amount.as_deref(), Some("600.00"));
        let changes = || BudgetChange {
            expected_revision: r.revision,
            budget_revision: 0,
            member_id: r.data.members[0].id.clone(),
            month: "2026-10".into(),
            replace_indices: vec![],
            acknowledge: true,
        };
        w.household_budget_reconcile(&r.id, changes()).unwrap();
        assert!(w.household_budget_reconcile(&r.id, changes()).is_err());
        let budget = w.get_client_budget(&client.id).unwrap();
        assert_eq!(budget.summary.necessary_expenses, "600.00");
        let id = r.id.clone();
        r = w.get_household(&id, "2026-10").unwrap();
        let p = payment(&r.data, &e, 0, "2400", "bill", None);
        r = save(&mut w, r, Action::Payment(p));
        assert_eq!(r.summary.total, "2400.00");
        assert_eq!(r.summary.settlements.len(), 2);
        let r = w
            .record_household_report(&id, "2026-10", r.revision)
            .unwrap();
        assert!(!r.data.reports[0].body.is_empty());
        let mut changed = r.data.members[0].clone();
        changed.consent.planning = false;
        let r = save(&mut w, r, Action::Member(changed));
        assert!(r.data.reports[0].body.is_empty());
        assert!(w
            .household_budget_preview(&id, &r.data.members[0].id, "2026-10")
            .is_err());
        let other = w
            .create_household("Second household".into(), "2026-10".into())
            .unwrap();
        let _other = save(&mut w, other, Action::Member(r.data.members[0].clone()));
        assert!(w
            .get_household(&Uuid::new_v4().to_string(), "2026-10")
            .is_err());
        w.lock_workspace();
        let mut reopened = Workspace::new(dir.path().to_path_buf());
        reopened.unlock_workspace(PASSWORD.into()).unwrap();
        let saved = reopened.get_household(&id, "2026-10").unwrap();
        assert_eq!(saved.summary.total, "2400.00");
        assert_eq!(saved.revision, r.revision);
        assert_eq!(reopened.list_households().unwrap().len(), 2);
    }
    #[test]
    fn journal_is_append_only_and_hash_checked() {
        let (_dir, mut w, info) = configured();
        let r = w
            .create_household("Synthetic".into(), "2026-10".into())
            .unwrap();
        let s = w.session().unwrap();
        assert!(s
            .connection
            .execute("UPDATE household_events SET action_json='{}'", [])
            .is_err());
        assert!(s
            .connection
            .execute("DELETE FROM household_events", [])
            .is_err());
        validate_all(&s.connection, &info).unwrap();
        s.connection
            .execute_batch("DROP TRIGGER household_no_update")
            .unwrap();
        s.connection
            .execute("UPDATE household_events SET digest='bad'", [])
            .unwrap();
        assert!(w.get_household(&r.id, "2026-10").is_err());
    }
    #[test]
    fn foreign_links_and_stale_revisions_do_not_write() {
        let (_dir, mut w, _) = configured();
        let r = w
            .create_household("Synthetic".into(), "2026-10".into())
            .unwrap();
        let mut m = member(1);
        m.client_id = Some(Uuid::new_v4().to_string());
        assert!(w
            .save_household(
                &r.id,
                "2026-10",
                h::Input {
                    expected_revision: 1,
                    action: Action::Member(m)
                }
            )
            .is_err());
        assert!(w
            .save_household(
                &r.id,
                "2026-10",
                h::Input {
                    expected_revision: 0,
                    action: Action::Rename("Lost update".into())
                }
            )
            .is_err());
        assert_eq!(w.get_household(&r.id, "2026-10").unwrap().revision, 1);
    }
    #[test]
    fn timing_reuses_cashflow_engine_and_shared_balances_require_permission() {
        let (_dir, mut w, _) = configured();
        let mut r = w
            .create_household("Synthetic".into(), "2026-10".into())
            .unwrap();
        for n in 1..=3 {
            let mut m = member(n);
            m.consent.balances = true;
            m.consent.evidence = "Synthetic permission".into();
            r = save(&mut w, r, Action::Member(m))
        }
        let account = h::Account {
            id: Uuid::new_v4().to_string(),
            label: "Joint checking".into(),
            owners: vec![r.data.members[0].id.clone(), r.data.members[1].id.clone()],
            opening_balance: Some("1000".into()),
            balance_date: "2026-10-01".into(),
            verified: true,
        };
        let aid = account.id.clone();
        r = save(&mut w, r, Action::Account(account));
        let mut e = expense(&r.data);
        e.account_id = Some(aid);
        r = save(&mut w, r, Action::Expense(e));
        assert_eq!(
            r.cashflow.as_ref().unwrap().combined_closing.as_deref(),
            Some("-1400.00")
        );
        assert!(r
            .cashflow
            .as_ref()
            .unwrap()
            .ledger
            .iter()
            .any(|e| !e.warnings.is_empty()));
        let mut m = r.data.members[1].clone();
        m.consent.balances = false;
        r = save(&mut w, r, Action::Member(m));
        assert!(r.data.accounts[0].opening_balance.is_none());
        assert!(r.cashflow.unwrap().combined_closing.is_none());
    }
    #[test]
    fn source_link_requires_explicit_permission_and_cannot_fabricate_approval() {
        let (_dir, mut w, _) = configured();
        let mut r = w
            .create_household("Synthetic".into(), "2026-10".into())
            .unwrap();
        for n in 1..=3 {
            r = save(&mut w, r, Action::Member(member(n)))
        }
        let e = expense(&data());
        let e = h::Expense {
            payer: r.data.members[0].id.clone(),
            ..e
        };
        r = save(&mut w, r, Action::Expense(e.clone()));
        let mut p = payment(&r.data, &e, 0, "2400", "bill", None);
        p.source = Some(h::Source {
            client_id: Uuid::new_v4().to_string(),
            row_id: "bank:fiction".into(),
            approval_id: Uuid::new_v4().to_string(),
        });
        assert!(w
            .save_household(
                &r.id,
                "2026-10",
                h::Input {
                    expected_revision: r.revision,
                    action: Action::Payment(p)
                }
            )
            .is_err());
        assert!(w
            .household_sources(&r.id, &r.data.members[0].id, "2026-10")
            .is_err());
    }
}
