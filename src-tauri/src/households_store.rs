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
    pub upcoming: Vec<h::Bill>,
    pub upcoming_as_of: String,
    pub history: Vec<h::History>,
    pub cashflow: Option<cashflow::Summary>,
    pub cashflow_warnings: Vec<String>,
    pub goal_results: Vec<savings::GoalResult>,
    pub goal_shares: Vec<Vec<h::Portion>>,
    pub source_rows: Vec<LinkedRow>,
    pub actual_mix: Vec<h::Category>,
    pub discretionary_mix: Vec<h::Category>,
    pub member_mix: Vec<h::Category>,
    pub actual_total: String,
    pub actual_necessary: String,
    pub actual_discretionary: String,
    pub actual_refunds: String,
    pub actual_net: String,
    pub actual_debt_payments: String,
    pub trends: Vec<h::Category>,
    pub opportunities: Vec<HouseholdOpportunity>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdOpportunity {
    pub title: String,
    pub amount: String,
    pub count: usize,
    pub detail: String,
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
    pub budget: PreviewBudget,
    pub proposed_lines: Vec<BudgetLine>,
    pub suggested_replacements: Vec<usize>,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewBudget {
    pub client_id: String,
    pub revision: i64,
    pub lines: Vec<BudgetLine>,
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
    let mut linked = BTreeSet::new();
    for id in ids(c, w)? {
        let (d, _, _, _) = read(c, w, &id, None)?;
        for p in d.payments.iter().filter(|p| !d.voids.contains(&p.id)) {
            if let Some(source) = &p.source {
                if !linked.insert((source.client_id.clone(), source.row_id.clone())) {
                    return Err("Duplicate household source links cannot be recovered as independent payments.".into());
                }
            }
        }
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
        "income" => m.consent.income && row.role == "income" && signed > 0,
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
    live.payments.retain(|p| {
        p.kind != "income"
            || d.members
                .iter()
                .find(|m| m.id == p.from_member)
                .is_some_and(|m| m.consent.income)
    });
    for p in &mut live.proposals {
        for m in &mut p.members {
            if !d
                .members
                .iter()
                .find(|existing| existing.id == m.id)
                .is_some_and(|m| m.consent.income)
            {
                m.monthly_income = None;
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
    // Explicit reimbursement dates create estimates, never completed settlements.
    // Use a single explicitly owned account; multiple possible accounts remain unknown.
    let mut consumed = std::collections::BTreeMap::<(String, String), i64>::new();
    for bill in &s.bills {
        if let (Some(due), Some(destination)) = (&bill.reimbursement_date, &bill.account_id) {
            for share in bill
                .shares
                .iter()
                .filter(|share| share.member_id != bill.payer)
            {
                let key = (share.member_id.clone(), bill.payer.clone());
                let prior = consumed.entry(key.clone()).or_default();
                let recorded: i64 = d
                    .payments
                    .iter()
                    .filter(|p| {
                        !d.voids.contains(&p.id)
                            && p.period == s.month
                            && p.from_member == key.0
                            && p.to_member.as_ref() == Some(&key.1)
                            && ["settlement", "contribution"].contains(&p.kind.as_str())
                    })
                    .map(|p| budget::parse_money(&p.amount).unwrap_or(0))
                    .sum();
                let obligation = budget::parse_money(&share.amount)?;
                let remaining = (obligation - (recorded - *prior).max(0)).max(0);
                *prior += obligation;
                if remaining == 0 {
                    continue;
                }
                let accounts: Vec<_> = d
                    .accounts
                    .iter()
                    .filter(|a| a.owners == [share.member_id.clone()])
                    .collect();
                if accounts.len() == 1 && accounts[0].id != *destination {
                    schedules.push(cashflow::Schedule {
                        label: format!("{} · expected reimbursement", bill.name),
                        account_id: accounts[0].id.clone(),
                        destination_id: Some(destination.clone()),
                        kind: cashflow::Kind::Transfer,
                        amount: Some(budget::money(remaining)),
                        first_date: due.clone(),
                        frequency: cashflow::Frequency::Once,
                        second_day: None,
                        until_date: None,
                        certainty: cashflow::Certainty::Estimated,
                    });
                } else {
                    warnings.push(format!("{}: expected reimbursement from {} on {} needs an explicit source account. Record a planned settlement to model its timing.",bill.name,share.member_id,due))
                }
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
    let today = Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive();
    let (start, end) = h::bounds(month)?;
    let as_of = if today >= start && today <= end {
        today
    } else {
        start
    };
    let through = as_of + chrono::Duration::days(30);
    let mut upcoming = summary
        .bills
        .iter()
        .filter(|b| b.date >= as_of.to_string() && b.date <= through.to_string())
        .cloned()
        .collect::<Vec<_>>();
    if through > end {
        if let Some(next) = start.checked_add_months(chrono::Months::new(1)) {
            if let Ok(next) = h::summary(&display, &next.format("%Y-%m").to_string()) {
                if next.invalid_allocations > 0 {
                    summary.warnings.push("Some upcoming bills have unresolved allocations; upcoming totals are incomplete.".into())
                }
                upcoming.extend(
                    next.bills
                        .into_iter()
                        .filter(|b| b.date <= through.to_string()),
                );
            }
        }
    }
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
    let mut discretionary_categories = std::collections::BTreeMap::<String, i64>::new();
    let mut member_spending = std::collections::BTreeMap::<String, i64>::new();
    let mut categories = std::collections::BTreeMap::<String, i64>::new();
    let mut trends = std::collections::BTreeMap::<String, i64>::new();
    let (mut total, mut necessary, mut discretionary, mut refunds, mut debt) = (0, 0, 0, 0, 0);
    let mut small = vec![];
    let mut subscriptions = vec![];
    let mut subscription_keys = BTreeSet::new();
    for client in d
        .members
        .iter()
        .filter(|m| m.consent.transactions)
        .filter_map(|m| m.client_id.as_ref())
    {
        for row in spending_analysis::household_subscription_rows(c, w, client, month)? {
            subscription_keys.insert((client.clone(), row));
        }
    }
    for linked in &rows {
        let p = d
            .payments
            .iter()
            .find(|p| p.id == linked.payment_id)
            .ok_or("Payment unavailable.")?;
        let amount = budget::parse_money(&p.amount)?;
        if p.kind == "bill"
            && ["necessary", "discretionary", "unclassified"].contains(&linked.row.role.as_str())
        {
            *trends.entry(linked.row.date[..7].into()).or_default() += amount;
        }
        if !linked.row.date.starts_with(month) {
            continue;
        }
        if p.kind == "refund" {
            refunds += amount;
            continue;
        }
        if p.kind != "bill" {
            continue;
        }
        if linked.row.role == "debtPayment" {
            debt += amount;
            continue;
        }
        total += amount;
        *member_spending.entry(linked.member_id.clone()).or_default() += amount;
        *categories.entry(linked.row.category.clone()).or_default() += amount;
        if linked.row.role == "necessary" {
            necessary += amount
        } else if linked.row.role == "discretionary" {
            discretionary += amount;
            *discretionary_categories
                .entry(linked.row.category.clone())
                .or_default() += amount;
            if amount < 1500 {
                small.push(amount)
            }
            if subscription_keys.contains(&(linked.client_id.clone(), linked.row.id.clone())) {
                subscriptions.push(amount)
            }
        }
    }
    let mut opportunities = vec![];
    if small.len() >= 5 {
        opportunities.push(HouseholdOpportunity {
            title: "Frequent small discretionary purchases".into(),
            amount: budget::money(small.iter().sum()),
            count: small.len(),
            detail:
                "Reviewed linked purchases under $15. Discuss whether any routine can be adjusted."
                    .into(),
        })
    }
    if !subscriptions.is_empty() {
        opportunities.push(HouseholdOpportunity{title:"Reviewed shared subscriptions".into(),amount:budget::money(subscriptions.iter().sum()),count:subscriptions.len(),detail:"Supported by current counselor-confirmed subscription reviews in the original client workspace.".into()})
    }
    let target = budget::parse_money(&summary.discretionary)?;
    if discretionary > target {
        opportunities.push(HouseholdOpportunity{title:"Discretionary spending above the household plan".into(),amount:budget::money(discretionary-target),count:1,detail:"Approved linked transactions compared with this month's discretionary agreements. Coverage may be incomplete.".into()})
    }
    if rows
        .iter()
        .any(|r| r.row.date.starts_with(month) && r.row.role == "unclassified")
    {
        summary.warnings.push("Some approved linked spending has no reviewed necessary/discretionary purpose. Those type totals are incomplete.".into())
    }
    let discretionary_mix = discretionary_categories
        .into_iter()
        .map(|(label, c)| h::Category {
            label,
            amount: budget::money(c),
            percent: if discretionary > 0 {
                ((i128::from(c) * 10000) / i128::from(discretionary)) as u16
            } else {
                0
            },
        })
        .collect();
    let member_mix = member_spending
        .into_iter()
        .map(|(member, c)| h::Category {
            label: format!(
                "{} · {}",
                d.members
                    .iter()
                    .find(|m| m.id == member)
                    .map(|m| m.name.as_str())
                    .unwrap_or("Unknown"),
                &member[..8]
            ),
            amount: budget::money(c),
            percent: if total > 0 {
                ((i128::from(c) * 10000) / i128::from(total)) as u16
            } else {
                0
            },
        })
        .collect();
    let trend_max = trends.values().copied().max().unwrap_or(0);
    let actual_mix = categories
        .into_iter()
        .map(|(label, c)| h::Category {
            label,
            amount: budget::money(c),
            percent: if total > 0 {
                ((i128::from(c) * 10000) / i128::from(total)) as u16
            } else {
                0
            },
        })
        .collect();
    let trends = trends
        .into_iter()
        .map(|(label, c)| h::Category {
            label,
            amount: budget::money(c),
            percent: if trend_max > 0 {
                ((i128::from(c) * 10000) / i128::from(trend_max)) as u16
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
        upcoming,
        upcoming_as_of: as_of.to_string(),
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
        discretionary_mix,
        member_mix,
        actual_total: budget::money(total),
        actual_necessary: budget::money(necessary),
        actual_discretionary: budget::money(discretionary),
        actual_refunds: budget::money(refunds),
        actual_net: budget::money(total - refunds),
        actual_debt_payments: budget::money(debt),
        trends,
        opportunities,
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
            Action::Create(_)
                | Action::Report(_)
                | Action::BudgetReconciled { .. }
                | Action::CashflowReconciled { .. }
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
        let checked = h::summary(&d, month)?;
        if checked.invalid_allocations > 0
            && matches!(
                input.action,
                Action::Expense(_) | Action::Rule(_) | Action::AcceptProposal(_)
            )
        {
            return Err("This agreement would leave invalid or unallocated household bills. Correct the allocation before finalizing.".into());
        }
        if let Action::Rule(rule) = &input.action {
            let (start, _) = h::bounds(&rule.effective[..7])?;
            for n in 0..=2 {
                let on = start
                    .checked_add_months(chrono::Months::new(n))
                    .ok_or("Invalid effective month.")?;
                if h::summary(&d, &on.format("%Y-%m").to_string())?.invalid_allocations > 0 {
                    return Err("This rule would leave an upcoming bill unallocated. Revise the participants or fixed contributions first.".into());
                }
            }
        }
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
        let rows = spending_analysis::household_rows(
            &s.connection,
            &s.info,
            m.client_id
                .as_deref()
                .ok_or("Link an individual client to browse approved sources.")?,
            month,
        )?;
        Ok(rows
            .into_iter()
            .filter(|row| row.role != "income" || m.consent.income)
            .collect())
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
        let body=serde_json::to_string(&serde_json::json!({"name":d.name,"summary":current.summary,"actualTotal":current.actual_total,"actualNecessary":current.actual_necessary,"actualDiscretionary":current.actual_discretionary,"sourceRows":current.source_rows,"cashflow":current.cashflow,"warnings":current.cashflow_warnings,"goals":current.data.goals,"goalResults":current.goal_results})).map_err(|_|STORAGE_ERROR)?;
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
struct PlanTag {
    prefix: String,
    legacy: Option<String>,
    short: String,
}
impl PlanTag {
    fn owns(&self, label: &str) -> bool {
        label.starts_with(&self.prefix)
            || self.legacy.as_ref().is_some_and(|s| label.starts_with(s))
    }
    fn owns_schedule(&self, label: &str, month: &str) -> bool {
        label.starts_with(&format!("{}{month} ", self.prefix))
            || self
                .legacy
                .as_ref()
                .is_some_and(|s| label.starts_with(&format!("{s}{month} ")))
    }
    fn ambiguous(&self, label: &str) -> bool {
        self.legacy.is_none() && label.starts_with(&self.short)
    }
}
fn plan_tag(c: &Connection, w: &WorkspaceInfo, id: &str) -> Result<PlanTag, String> {
    let short = format!("HH:{}:", &id[..8]);
    let unique = ids(c, w)?
        .iter()
        .filter(|other| other.get(..8) == id.get(..8))
        .count()
        == 1;
    Ok(PlanTag {
        prefix: format!("HH:{id}:"),
        legacy: if unique { Some(short.clone()) } else { None },
        short,
    })
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
    if summary.invalid_allocations > 0 {
        return Err(
            "Resolve unallocated household bills before reconciling an individual budget.".into(),
        );
    }
    let tag = plan_tag(c, w, id)?;
    let prefix = &tag.prefix;
    if budget.lines.iter().any(|line| tag.ambiguous(&line.label)) {
        return Err("Older shortened household labels are ambiguous. Review and rename their entries in the individual budget before reconciling these households.".into());
    }
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
    let mut proposed_lines: Vec<BudgetLine> = grouped
        .into_iter()
        .map(|((agreement, name, purpose), c)| BudgetLine {
            label: format!(
                "{prefix}{} {}",
                &agreement[..8],
                name.chars().take(64).collect::<String>()
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
    for goal in &d.goals {
        if let Ok(shares) = h::split(
            budget::parse_money(&goal.monthly)?,
            &goal.allocation,
            &d,
            &format!("{month}-01"),
        ) {
            if let Some(share) = shares.into_iter().find(|s| s.member_id == member) {
                proposed_lines.push(BudgetLine {
                    label: format!(
                        "{prefix}{} {}",
                        &goal.id[..8],
                        goal.label.chars().take(64).collect::<String>()
                    ),
                    kind: budget::Kind::Savings,
                    amount: Some(share.amount),
                    frequency: budget::Frequency::Monthly,
                    estimated: false,
                });
            }
        }
    }
    let mut warnings=vec!["Only confirmed agreements in the selected month are included. Review existing personal lines and other households to prevent duplication. Full bill payments and reimbursements are not additional budget expenses.".into()];
    let suggested_replacements = budget
        .lines
        .iter()
        .enumerate()
        .filter(|(_, line)| tag.owns(&line.label))
        .map(|(i, _)| i)
        .collect();
    if proposed_lines.is_empty() {
        warnings.push("There are no confirmed household shares to reconcile in this month.".into())
    }
    Ok(BudgetPreview {
        client_id: client.into(),
        household_revision: revision,
        budget: PreviewBudget {
            client_id: budget.client_id,
            revision: budget.revision,
            lines: budget
                .lines
                .into_iter()
                .map(|mut line| {
                    if line.kind == budget::Kind::Income && !m.consent.income {
                        line.label = "Restricted income line".into();
                        line.amount = None;
                    }
                    line
                })
                .collect(),
        },
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
        let tag = plan_tag(&tx, &s.info, id)?;
        let selected: BTreeSet<_> = input.replace_indices.iter().copied().collect();
        if selected.len() != input.replace_indices.len()
            || selected.iter().any(|i| {
                *i >= preview.budget.lines.len()
                    || !matches!(
                        preview.budget.lines[*i].kind,
                        budget::Kind::Necessary
                            | budget::Kind::Discretionary
                            | budget::Kind::DebtPayment
                    ) && !(preview.budget.lines[*i].kind == budget::Kind::Savings
                        && tag.owns(&preview.budget.lines[*i].label))
            })
        {
            return Err("Choose distinct existing expense rows to replace; income and savings cannot be removed here.".into());
        }
        if preview
            .budget
            .lines
            .iter()
            .enumerate()
            .any(|(i, l)| tag.owns(&l.label) && !selected.contains(&i))
        {
            return Err("Select all prior shares from this household for replacement to prevent double counting.".into());
        }
        let original = read_budget(&tx, &s.info, &preview.client_id)?;
        let before = serde_json::to_string(&original).map_err(|_| STORAGE_ERROR)?;
        let mut lines: Vec<_> = original
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
impl Workspace {
    pub fn household_report(
        &mut self,
        id: &str,
        report_id: &str,
    ) -> Result<(String, h::Report, h::Summary, String), String> {
        let s = self.session()?;
        let (d, _, _, _) = read(&s.connection, &s.info, id, None)?;
        let report = d
            .reports
            .iter()
            .find(|r| r.id == report_id)
            .ok_or("Household report unavailable.")?
            .clone();
        if report.permissions != permissions(&d) {
            return Err(
                "Sharing authorization changed; this historical report is restricted.".into(),
            );
        }
        let body: serde_json::Value =
            serde_json::from_str(&report.body).map_err(|_| STORAGE_ERROR)?;
        let summary = serde_json::from_value(body["summary"].clone()).map_err(|_| STORAGE_ERROR)?;
        Ok((
            body["name"].as_str().ok_or(STORAGE_ERROR)?.into(),
            report,
            summary,
            body["actualTotal"].as_str().ok_or(STORAGE_ERROR)?.into(),
        ))
    }
    pub fn export_household_report_pdf(
        &mut self,
        id: &str,
        report_id: &str,
        path: &str,
    ) -> Result<crate::assessment_pdf::Export, String> {
        let (name, report, summary, actual) = self.household_report(id, report_id)?;
        let (bytes, pages) =
            crate::assessment_pdf::render_household(&name, &report, &summary, &actual)?;
        crate::assessment_pdf::save_bytes(report_id, bytes, pages, path, &self.data_dir)
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashflowPreview {
    pub client_id: String,
    pub household_revision: i64,
    pub revision: i64,
    pub accounts: Vec<(String, String)>,
    pub existing: Vec<cashflow::Schedule>,
    pub proposed: Vec<cashflow::Schedule>,
    pub suggested_replacements: Vec<usize>,
    pub warnings: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CashflowChange {
    pub expected_revision: i64,
    pub revision: i64,
    pub member_id: String,
    pub month: String,
    pub account_id: String,
    pub replace_indices: Vec<usize>,
    pub acknowledge: bool,
}
fn cashflow_preview(
    c: &Connection,
    w: &WorkspaceInfo,
    id: &str,
    member_id: &str,
    month: &str,
    account_id: Option<&str>,
) -> Result<CashflowPreview, String> {
    let (d, revision, _, _) = read(c, w, id, None)?;
    let m = consent_member(&d, member_id, "planning")?;
    let client = m
        .client_id
        .as_deref()
        .ok_or("Link an individual client before cash-flow reconciliation.")?;
    let current = read_cashflow_plan(c, w, client)?;
    let plan=current.data.ok_or("Save an individual cash-flow plan with accounts and a date range before reconciling household movements.")?;
    let (start, end) = h::bounds(month)?;
    if plan.start_date > start.to_string() || plan.end_date < end.to_string() {
        return Err("The individual cash-flow date range must cover the selected household month. Review its dates in the individual workspace first.".into());
    }
    let (live, _, mut warnings) = live(c, w, &d)?;
    let summary = h::summary(&live, month)?;
    if summary.invalid_allocations > 0 {
        return Err(
            "Resolve unallocated household bills before reconciling individual cash flow.".into(),
        );
    }
    let tag = plan_tag(c, w, id)?;
    let prefix = format!("{}{month} ", tag.prefix);
    if plan.schedules.iter().any(|s| tag.ambiguous(&s.label)) {
        return Err("Older shortened household schedule labels are ambiguous. Review and rename their individual schedules before reconciling these households.".into());
    }
    let mut proposed = vec![];
    let mut add = |label: String, date: String, amount: String, inflow: bool, confirmed: bool| {
        proposed.push(cashflow::Schedule {
            label: format!("{prefix}{}", label.chars().take(64).collect::<String>()),
            account_id: account_id.unwrap_or("").into(),
            destination_id: None,
            kind: if inflow {
                cashflow::Kind::OtherInflow
            } else {
                cashflow::Kind::OtherOutflow
            },
            amount: Some(amount),
            first_date: date,
            frequency: cashflow::Frequency::Once,
            second_day: None,
            until_date: None,
            certainty: if confirmed {
                cashflow::Certainty::Confirmed
            } else {
                cashflow::Certainty::Estimated
            },
        });
    };
    if let Some(account) = account_id {
        if !plan.accounts.iter().any(|a| a.id == account) {
            return Err("Choose an existing individual cash-flow account.".into());
        }
        for bill in &summary.bills {
            if bill.payer == member_id {
                let paid: i64 = live
                    .payments
                    .iter()
                    .filter(|p| {
                        !live.voids.contains(&p.id)
                            && p.period == month
                            && p.expense_id.as_ref() == Some(&bill.agreement_id)
                            && p.kind == "bill"
                    })
                    .map(|p| budget::parse_money(&p.amount).unwrap_or(0))
                    .sum();
                let prior: i64 = summary
                    .bills
                    .iter()
                    .take_while(|x| !std::ptr::eq(*x, bill))
                    .filter(|x| x.agreement_id == bill.agreement_id)
                    .map(|x| budget::parse_money(&x.amount).unwrap_or(0))
                    .sum();
                let remaining = (budget::parse_money(&bill.amount)? - (paid - prior).max(0)).max(0);
                if remaining > 0 {
                    add(
                        format!("{} external bill", bill.name),
                        bill.date.clone(),
                        budget::money(remaining),
                        false,
                        false,
                    );
                }
            }
        }
        for p in live.payments.iter().filter(|p| {
            !live.voids.contains(&p.id) && p.date >= start.to_string() && p.date <= end.to_string()
        }) {
            if p.from_member == member_id {
                add(
                    p.kind.clone(),
                    p.date.clone(),
                    p.amount.clone(),
                    ["refund", "income"].contains(&p.kind.as_str()),
                    p.state == "confirmed",
                );
            }
            if p.to_member.as_deref() == Some(member_id) {
                add(
                    format!("{} received", p.kind),
                    p.date.clone(),
                    p.amount.clone(),
                    true,
                    p.state == "confirmed",
                );
            }
        }
        let mut consumed = std::collections::BTreeMap::<(String, String), i64>::new();
        for bill in &summary.bills {
            for share in bill.shares.iter().filter(|s| s.member_id != bill.payer) {
                if share.member_id != member_id && bill.payer != member_id {
                    continue;
                }
                let key = (share.member_id.clone(), bill.payer.clone());
                let prior = consumed.entry(key.clone()).or_default();
                let recorded: i64 = live
                    .payments
                    .iter()
                    .filter(|p| {
                        !live.voids.contains(&p.id)
                            && p.period == month
                            && p.from_member == key.0
                            && p.to_member.as_ref() == Some(&key.1)
                            && ["settlement", "contribution"].contains(&p.kind.as_str())
                    })
                    .map(|p| budget::parse_money(&p.amount).unwrap_or(0))
                    .sum();
                let obligation = budget::parse_money(&share.amount)?;
                let remaining = (obligation - (recorded - *prior).max(0)).max(0);
                *prior += obligation;
                if remaining > 0 {
                    if let Some(date) = &bill.reimbursement_date {
                        add(
                            format!("{} expected reimbursement", bill.name),
                            date.clone(),
                            budget::money(remaining),
                            bill.payer == member_id,
                            false,
                        )
                    } else {
                        warnings.push(format!(
                            "{} needs an agreed reimbursement date. No date was invented.",
                            bill.name
                        ));
                    }
                }
            }
        }
    }
    let suggested_replacements = plan
        .schedules
        .iter()
        .enumerate()
        .filter(|(_, s)| tag.owns_schedule(&s.label, month))
        .map(|(i, _)| i)
        .collect();
    let mut existing = plan.schedules;
    for s in &mut existing {
        if matches!(
            s.kind,
            cashflow::Kind::Paycheck | cashflow::Kind::OtherInflow
        ) && !tag.owns(&s.label)
            && !m.consent.income
        {
            s.amount = None;
            s.label = "Restricted income schedule".into();
            s.first_date = start.to_string();
            s.until_date = None;
            s.frequency = cashflow::Frequency::Once;
            s.second_day = None;
        }
    }
    warnings.push("Review existing schedules for overlap. The payer's full external bill and dated reimbursements are separate; cash flow is not the same as their budget share. Goal deposits require an explicitly dated savings schedule in the individual plan.".into());
    Ok(CashflowPreview {
        client_id: client.into(),
        household_revision: revision,
        revision: current.revision,
        accounts: plan.accounts.into_iter().map(|a| (a.id, a.label)).collect(),
        existing,
        proposed,
        suggested_replacements,
        warnings,
    })
}
impl Workspace {
    pub fn household_cashflow_preview(
        &mut self,
        id: &str,
        member: &str,
        month: &str,
        account: Option<&str>,
    ) -> Result<CashflowPreview, String> {
        let s = self.session()?;
        cashflow_preview(&s.connection, &s.info, id, member, month, account)
    }
    pub fn household_cashflow_reconcile(
        &mut self,
        id: &str,
        input: CashflowChange,
    ) -> Result<Record, String> {
        if !input.acknowledge {
            return Err("Review and approve the dated cash-flow reconciliation first.".into());
        }
        let s = self.session()?;
        let tx = s
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let (d, revision, _, previous) = read(&tx, &s.info, id, None)?;
        if d.archived {
            return Err("Reopen the household before changing individual plans.".into());
        }
        let preview = cashflow_preview(
            &tx,
            &s.info,
            id,
            &input.member_id,
            &input.month,
            Some(&input.account_id),
        )?;
        if preview.household_revision != input.expected_revision
            || preview.revision != input.revision
        {
            return Err("Household or individual cash-flow plan changed. Preview again; nothing was changed.".into());
        }
        let selected: BTreeSet<_> = input.replace_indices.iter().copied().collect();
        if selected.len() != input.replace_indices.len()
            || selected.iter().any(|i| *i >= preview.existing.len())
            || preview
                .suggested_replacements
                .iter()
                .any(|i| !selected.contains(i))
        {
            return Err("Select valid existing schedules, including all prior household schedules for this month, to prevent duplicates.".into());
        }
        let current = read_cashflow_plan(&tx, &s.info, &preview.client_id)?;
        let mut plan = current
            .data
            .ok_or("Individual cash-flow plan unavailable.")?;
        if selected
            .iter()
            .any(|i| matches!(plan.schedules[*i].kind, cashflow::Kind::Paycheck))
        {
            return Err("Income schedules cannot be removed by household reconciliation.".into());
        }
        let before = serde_json::to_string(&plan).map_err(|_| STORAGE_ERROR)?;
        plan.schedules = plan
            .schedules
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !selected.contains(i))
            .map(|(_, s)| s)
            .collect();
        plan.schedules.extend(preview.proposed);
        let (plan, _) = cashflow::calculate(plan)?;
        let after = serde_json::to_string(&plan).map_err(|_| STORAGE_ERROR)?;
        let next = input.revision + 1;
        tx.execute("INSERT INTO client_cashflow_plans(client_id,organization_id,counselor_id,revision,plan_json,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(client_id) DO UPDATE SET revision=excluded.revision,plan_json=excluded.plan_json,updated_at=excluded.updated_at WHERE client_cashflow_plans.organization_id=excluded.organization_id AND client_cashflow_plans.counselor_id=excluded.counselor_id",params![preview.client_id,s.info.organization_id,s.info.counselor_id,next,after,timestamp()]).map_err(storage_error)?;
        audit(
            &tx,
            &s.info,
            "cashflow_plan_saved",
            Some(&preview.client_id),
        )?;
        append(
            &tx,
            &s.info,
            id,
            revision + 1,
            &previous,
            &Action::CashflowReconciled {
                client_id: preview.client_id,
                before,
                after,
                revision: next,
                month: input.month.clone(),
            },
        )?;
        let result = record(&tx, &s.info, id, &input.month)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}
#[cfg(test)]
mod integration_tests {
    use super::super::tests::{client_input, configured, PASSWORD};
    use super::*;
    use crate::households::tests::{expense, member, payment};
    fn save(w: &mut Workspace, r: Record, action: Action, month: &str) -> Record {
        w.save_household(
            &r.id,
            month,
            h::Input {
                expected_revision: r.revision,
                action,
            },
        )
        .unwrap()
    }
    fn family(w: &mut Workspace, client: &str, month: &str) -> Record {
        let mut r = w
            .create_household("Synthetic Household".into(), month.into())
            .unwrap();
        for n in 1..=3 {
            let mut m = member(n);
            if n == 1 {
                m.client_id = Some(client.into());
                m.consent.transactions = true;
                m.consent.planning = true;
                m.consent.evidence =
                    "Synthetic authorization for household transactions and budget expenses.".into()
            }
            r = save(w, r, Action::Member(m), month)
        }
        r
    }
    #[test]
    fn approved_source_live_classification_revocation_duplicate_link_and_original_preservation() {
        let (dir, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let mut r = family(&mut w, &client.id, "2026-09");
        let mut e = expense(&r.data);
        e.amount = "1200.00".into();
        e.effective = "2026-09-01".into();
        e.due = e.effective.clone();
        r = save(&mut w, r, Action::Expense(e.clone()), "2026-09");
        let s=imports_store::tests::stage(&mut w,&client.id,dir.path(),"household.csv","Date,Description,Amount,Category,Type\n2026-09-02,Synthetic rent,-1200,Housing,expense\n2026-09-03,Synthetic salary,2000,Wages,income\n",imports_store::tests::doc("Synthetic checking"));
        let source = s
            .records
            .iter()
            .find(|r| r.fields.kind == crate::imports::Kind::Expense)
            .unwrap();
        assert!(w
            .household_sources(&r.id, &r.data.members[0].id, "2026-09")
            .unwrap()
            .is_empty());
        w.review_transaction(
            &client.id,
            crate::imports::ReviewInput {
                expected_revision: s.revision,
                record_id: source.id.clone(),
                status: crate::imports::Status::Approved,
                fields: source.fields.clone(),
                reason: "Verified synthetic source".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
        let rows = w
            .household_sources(&r.id, &r.data.members[0].id, "2026-09")
            .unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        let mut p = payment(&r.data, &e, 0, "1200.00", "bill", None);
        p.period = "2026-09".into();
        p.date = "2026-09-02".into();
        p.source = Some(h::Source {
            client_id: client.id.clone(),
            row_id: row.id.clone(),
            approval_id: row.approval_id.clone(),
        });
        r = save(&mut w, r, Action::Payment(p.clone()), "2026-09");
        assert_eq!(r.actual_total, "1200.00");
        assert_eq!(r.summary.confirmed_paid, "1200.00");
        let mut duplicate = p;
        duplicate.id = Uuid::new_v4().to_string();
        assert!(w
            .save_household(
                &r.id,
                "2026-09",
                h::Input {
                    expected_revision: r.revision,
                    action: Action::Payment(duplicate)
                }
            )
            .is_err());
        let q = spending_analysis::Input {
            from_month: "2026-09".into(),
            through_month: "2026-09".into(),
            account_id: None,
            search: String::new(),
            category: None,
            role: None,
            page: 0,
            page_size: 50,
            compare_previous: false,
            opportunity: None,
        };
        let view = w.get_spending_analysis(&client.id, q.clone()).unwrap();
        let original = serde_json::to_string(
            &w.get_client_imports(&client.id)
                .unwrap()
                .records
                .iter()
                .find(|r| r.id == source.id)
                .unwrap()
                .raw,
        )
        .unwrap();
        w.household_classify(
            &r.id,
            &r.data.members[0].id,
            spending_analysis::Change {
                context: view.context,
                query: q.clone(),
                row_id: row.id.clone(),
                approval_id: row.approval_id.clone(),
                role: spending_analysis::Role::Necessary,
                reason: "Discussed necessary shared rent".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        let now = w.get_household(&r.id, "2026-09").unwrap();
        assert_eq!(now.actual_necessary, "1200.00");
        assert_eq!(
            w.get_spending_analysis(&client.id, q)
                .unwrap()
                .totals
                .necessary,
            "1200.00"
        );
        assert_eq!(
            serde_json::to_string(
                &w.get_client_imports(&client.id)
                    .unwrap()
                    .records
                    .iter()
                    .find(|r| r.id == source.id)
                    .unwrap()
                    .raw
            )
            .unwrap(),
            original
        );
        let mut m = now.data.members[0].clone();
        m.consent.transactions = false;
        let restricted = save(&mut w, now, Action::Member(m), "2026-09");
        assert_eq!(restricted.actual_total, "0.00");
        assert_eq!(restricted.summary.confirmed_paid, "0.00");
        assert!(restricted.source_rows.is_empty());
        assert!(restricted
            .summary
            .warnings
            .iter()
            .any(|w| w.contains("restricted")));
    }
    #[test]
    fn restricted_income_is_not_in_budget_preview_or_overwritten_by_reconciliation() {
        let (_dir, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        w.save_client_budget(
            &client.id,
            budget::BudgetInput {
                expected_revision: 0,
                lines: vec![BudgetLine {
                    label: "Private salary".into(),
                    kind: budget::Kind::Income,
                    amount: Some("5678.90".into()),
                    frequency: budget::Frequency::Monthly,
                    estimated: false,
                }],
            },
        )
        .unwrap();
        let mut r = family(&mut w, &client.id, "2026-10");
        let e = expense(&r.data);
        r = save(&mut w, r, Action::Expense(e), "2026-10");
        let preview = w
            .household_budget_preview(&r.id, &r.data.members[0].id, "2026-10")
            .unwrap();
        let json = serde_json::to_string(&preview).unwrap();
        assert!(!json.contains("5678.90"));
        assert!(!json.contains("Private salary"));
        w.household_budget_reconcile(
            &r.id,
            BudgetChange {
                expected_revision: r.revision,
                budget_revision: 1,
                member_id: r.data.members[0].id.clone(),
                month: "2026-10".into(),
                replace_indices: vec![],
                acknowledge: true,
            },
        )
        .unwrap();
        assert_eq!(
            w.get_client_budget(&client.id).unwrap().lines[0]
                .amount
                .as_deref(),
            Some("5678.90")
        );
    }
    #[test]
    fn frozen_income_agreements_survive_income_changes_and_journal_replay() {
        let (dir, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let mut r = family(&mut w, &client.id, "2026-10");
        for i in 0..3 {
            let mut m = r.data.members[i].clone();
            m.consent.income = true;
            m.monthly_income = Some(if i == 0 { "1000" } else { "2000" }.into());
            m.consent.evidence = "Synthetic income-sharing consent".into();
            r = save(&mut w, r, Action::Member(m), "2026-10")
        }
        let mut e = expense(&r.data);
        e.allocation = Some(crate::households::tests::allocation(
            &r.data,
            "income",
            &["1", "1", "1"],
        ));
        r = save(&mut w, r, Action::Expense(e), "2026-10");
        assert_eq!(r.summary.member_totals[0].obligation, "480.00");
        let mut m = r.data.members[0].clone();
        m.monthly_income = Some("5000".into());
        r = save(&mut w, r, Action::Member(m), "2026-10");
        assert_eq!(r.summary.member_totals[0].obligation, "480.00");
        let id = r.id.clone();
        drop(w);
        let mut reopened = Workspace::new(dir.path().to_path_buf());
        reopened.unlock_workspace(PASSWORD.into()).unwrap();
        let r = reopened.get_household(&id, "2026-10").unwrap();
        assert_eq!(r.summary.member_totals[0].obligation, "480.00");
    }
    #[test]
    fn individual_cashflow_reconciles_full_payer_and_later_reimbursements_transactionally() {
        let (_dir, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let mut r = family(&mut w, &client.id, "2026-10");
        let e = expense(&r.data);
        r = save(&mut w, r, Action::Expense(e), "2026-10");
        let account = Uuid::new_v4().to_string();
        w.save_client_cashflow_plan(
            &client.id,
            cashflow::PlanInput {
                expected_revision: 0,
                data: cashflow::PlanData {
                    start_date: "2026-10-01".into(),
                    end_date: "2026-10-31".into(),
                    accounts: vec![cashflow::Account {
                        id: account.clone(),
                        label: "Personal checking".into(),
                        opening_balance: Some("1000".into()),
                    }],
                    schedules: vec![],
                    notes: String::new(),
                },
            },
        )
        .unwrap();
        let preview = w
            .household_cashflow_preview(&r.id, &r.data.members[0].id, "2026-10", Some(&account))
            .unwrap();
        assert_eq!(preview.proposed.len(), 3);
        assert_eq!(preview.proposed[0].amount.as_deref(), Some("2400.00"));
        assert_eq!(preview.proposed[1].first_date, "2026-10-05");
        let change = || CashflowChange {
            expected_revision: r.revision,
            revision: 1,
            member_id: r.data.members[0].id.clone(),
            month: "2026-10".into(),
            account_id: account.clone(),
            replace_indices: vec![],
            acknowledge: true,
        };
        w.household_cashflow_reconcile(&r.id, change()).unwrap();
        assert!(w.household_cashflow_reconcile(&r.id, change()).is_err());
        let plan = w.get_client_cashflow_plan(&client.id).unwrap();
        assert_eq!(plan.revision, 2);
        let s = plan.summary.unwrap();
        assert_eq!(s.ledger[0].source_balance.as_deref(), Some("-1400.00"));
        assert_eq!(s.combined_closing.as_deref(), Some("400.00"));
    }
    #[test]
    fn report_pdf_and_backup_preserve_household_history_without_overwriting() {
        let (dir, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let r = family(&mut w, &client.id, "2026-10");
        let r = save(
            &mut w,
            r,
            Action::Expense(expense(&crate::households::tests::data())),
            "2026-10",
        );
        let r = w
            .record_household_report(&r.id, "2026-10", r.revision)
            .unwrap();
        let output = tempfile::tempdir().unwrap();
        let path = output
            .path()
            .join("household.pdf")
            .to_string_lossy()
            .into_owned();
        let pdf = w
            .export_household_report_pdf(&r.id, &r.data.reports[0].id, &path)
            .unwrap();
        assert!(pdf.pages >= 1);
        assert!(fs::read(&path).unwrap().starts_with(b"%PDF-"));
        assert!(w
            .export_household_report_pdf(&r.id, &r.data.reports[0].id, &path)
            .is_err());
        let backup = w
            .create_backup(output.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let review = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(review.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            w.get_household(&r.id, "2026-10").unwrap().revision,
            r.revision
        );
        assert!(dir.path().exists());
    }
}
#[cfg(test)]
mod migration_tests {
    use super::super::tests::{client_input, configured, PASSWORD};
    use super::*;
    #[test]
    fn schema30_upgrade_rolls_back_atomically_and_retains_notes() {
        let (_root, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let note = w
            .add_client_note(&client.id, "Synthetic legacy household upgrade note")
            .unwrap();
        let c = &mut w.session.as_mut().unwrap().connection;
        c.execute_batch("DROP TABLE household_events; PRAGMA user_version=30; CREATE TABLE audit_events_v31(collision TEXT);").unwrap();
        assert!(migrate(c, false).is_err());
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            30
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='household_events'",
                [],
                |r| r.get::<_, i32>(0)
            )
            .unwrap(),
            0
        );
        c.execute_batch("DROP TABLE audit_events_v31;").unwrap();
        migrate(c, false).unwrap();
        assert_eq!(w.list_client_notes(&client.id).unwrap(), vec![note]);
        assert!(w.list_households().unwrap().is_empty());
    }
    #[test]
    fn corrupt_household_history_blocks_recovery_activation() {
        let (root, mut w, _) = configured();
        let r = w
            .create_household("Synthetic recovery household".into(), "2026-10".into())
            .unwrap();
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch(
                "DROP TRIGGER household_no_update;UPDATE household_events SET digest='corrupt';",
            )
            .unwrap();
        let output = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(output.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let pointer = fs::read(root.path().join("workspace-active.json")).ok();
        w.lock_workspace();
        assert!(w.prepare_restore(backup.path, PASSWORD.into()).is_err());
        assert_eq!(
            fs::read(root.path().join("workspace-active.json")).ok(),
            pointer
        );
        assert!(w.get_household(&r.id, "2026-10").is_err());
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
#[cfg(test)]
mod household_identity_tests {
    use super::super::tests::{client_input, configured};
    use super::*;
    use crate::households::tests::{expense, member};
    #[test]
    fn matching_short_ids_never_replace_another_households_budget_shares() {
        let (_dir, mut w, info) = configured();
        let client = w.save_client(client_input()).unwrap();
        let a = w
            .create_household("First household".into(), "2026-10".into())
            .unwrap();
        let b = format!("{}{}", &a.id[..8], &Uuid::new_v4().to_string()[8..]);
        {
            let s = w.session().unwrap();
            let tx = s.connection.transaction().unwrap();
            append(
                &tx,
                &info,
                &b,
                1,
                "",
                &Action::Create("Second household".into()),
            )
            .unwrap();
            tx.commit().unwrap();
        }
        for id in [&a.id, &b] {
            let mut r = w.get_household(id, "2026-10").unwrap();
            for n in 1..=3 {
                let mut m = member(n);
                if n == 1 {
                    m.client_id = Some(client.id.clone());
                    m.consent.planning = true;
                    m.consent.evidence = "Synthetic shared planning authorization".into()
                }
                r = w
                    .save_household(
                        id,
                        "2026-10",
                        h::Input {
                            expected_revision: r.revision,
                            action: Action::Member(m),
                        },
                    )
                    .unwrap();
            }
            let mut e = expense(&r.data);
            if id == &b {
                e.amount = "300.00".into();
                e.name = "Utilities".into()
            }
            r = w
                .save_household(
                    id,
                    "2026-10",
                    h::Input {
                        expected_revision: r.revision,
                        action: Action::Expense(e),
                    },
                )
                .unwrap();
            let budget_revision = w.get_client_budget(&client.id).unwrap().revision;
            let preview = w
                .household_budget_preview(id, &r.data.members[0].id, "2026-10")
                .unwrap();
            assert!(preview.suggested_replacements.is_empty());
            w.household_budget_reconcile(
                id,
                BudgetChange {
                    expected_revision: r.revision,
                    budget_revision,
                    member_id: r.data.members[0].id.clone(),
                    month: "2026-10".into(),
                    replace_indices: vec![],
                    acknowledge: true,
                },
            )
            .unwrap();
        }
        let budget = w.get_client_budget(&client.id).unwrap();
        assert_eq!(budget.lines.len(), 2);
        assert_eq!(budget.summary.necessary_expenses, "675.00");
        assert!(budget.lines[0].label.contains(&a.id));
        assert!(budget.lines[1].label.contains(&b));
        let r = w.get_household(&a.id, "2026-10").unwrap();
        let preview = w
            .household_budget_preview(&a.id, &r.data.members[0].id, "2026-10")
            .unwrap();
        assert_eq!(preview.suggested_replacements, vec![0]);
        let s = w.session().unwrap();
        let tag = plan_tag(&s.connection, &s.info, &a.id).unwrap();
        assert!(tag.owns_schedule(&format!("HH:{}:2026-10 rent", a.id), "2026-10"));
        assert!(!tag.owns_schedule(&format!("HH:{b}:2026-10 utilities"), "2026-10"));
        assert!(tag.ambiguous(&format!("HH:{}:old rent", &a.id[..8])));
    }
    #[test]
    fn backup_validation_rejects_duplicate_source_links_across_households() {
        let (_dir, mut w, info) = configured();
        let mut data = crate::households::tests::data();
        let e = expense(&data);
        h::apply(&mut data, &mut Action::Expense(e.clone())).unwrap();
        let mut p = crate::households::tests::payment(&data, &e, 0, "2400.00", "bill", None);
        p.source = Some(h::Source {
            client_id: Uuid::new_v4().to_string(),
            row_id: "bank:synthetic-duplicate".into(),
            approval_id: Uuid::new_v4().to_string(),
        });
        let s = w.session().unwrap();
        for _ in 0..2 {
            let id = Uuid::new_v4().to_string();
            let tx = s.connection.transaction().unwrap();
            let mut previous = String::new();
            let mut actions = vec![Action::Create("Synthetic duplicate fixture".into())];
            actions.extend(data.members.iter().cloned().map(Action::Member));
            actions.push(Action::Expense(e.clone()));
            actions.push(Action::Payment(p.clone()));
            for (i, action) in actions.into_iter().enumerate() {
                append(&tx, &info, &id, (i + 1) as i64, &previous, &action).unwrap();
                previous = read(&tx, &info, &id, None).unwrap().3;
            }
            tx.commit().unwrap();
        }
        assert!(validate_all(&s.connection, &info)
            .unwrap_err()
            .contains("Duplicate household source"));
    }
}
