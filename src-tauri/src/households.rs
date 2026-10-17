//! Shared agreements and settlement math. Money stays in integer cents; no private client reads.
use crate::budget::{money, parse_money};
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! model { ($name:ident {$($f:ident : $t:ty),* $(,)?}) => {
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all="camelCase",deny_unknown_fields)]
    pub struct $name {$(pub $f:$t),*}
}; }
model!(Consent {
    income: bool,
    balances: bool,
    transactions: bool,
    planning: bool,
    evidence: String
});
model!(Member { id:String, client_id:Option<String>, name:String, relationship:String, start:String, end:Option<String>, consent:Consent, monthly_income:Option<String> });
model!(Share {
    member_id: String,
    fixed: String,
    weight: String
});
model!(Allocation { method:String, shares:Vec<Share> });
model!(Rule { id:String, category:String, effective:String, until:Option<String>, allocation:Allocation, note:String });
model!(Expense { id:String, agreement_id:String, name:String, category:String, amount:String, frequency:String, due:String, until:Option<String>, effective:String, status:String, purpose:String, payer:String, account_id:Option<String>, reimbursement_date:Option<String>, allocation:Option<Allocation>, note:String });
model!(Account { id:String, label:String, owners:Vec<String>, opening_balance:Option<String>, balance_date:String, verified:bool });
model!(Source {
    client_id: String,
    row_id: String,
    approval_id: String
});
model!(Payment { id:String, expense_id:Option<String>, period:String, from_member:String, to_member:Option<String>, account_id:Option<String>, destination_account:Option<String>, date:String, amount:String, kind:String, state:String, source:Option<Source>, note:String });
model!(Goal { id:String, label:String, target:String, reserved:String, monthly:String, target_date:Option<String>, allocation:Allocation, note:String });
model!(Proposal { id:String, name:String, members:Vec<Member>, expenses:Vec<Expense>, note:String });
model!(Report {
    id: String,
    month: String,
    revision: i64,
    saved_at: String,
    permissions: String,
    body: String
});
model!(History {
    revision: i64,
    at: String,
    action: String,
    note: String
});
model!(Data { name:String, archived:bool, members:Vec<Member>, expenses:Vec<Expense>, rules:Vec<Rule>, accounts:Vec<Account>, payments:Vec<Payment>, voids:Vec<String>, goals:Vec<Goal>, proposals:Vec<Proposal>, accepted:Vec<String>, reports:Vec<Report> });
impl Data {
    pub fn new(name: String) -> Self {
        Self {
            name,
            archived: false,
            members: vec![],
            expenses: vec![],
            rules: vec![],
            accounts: vec![],
            payments: vec![],
            voids: vec![],
            goals: vec![],
            proposals: vec![],
            accepted: vec![],
            reports: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "action",
    content = "data",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    Create(String),
    Rename(String),
    Archive(bool),
    Member(Member),
    Expense(Expense),
    Rule(Rule),
    Account(Account),
    Payment(Payment),
    VoidPayment(String),
    Goal(Goal),
    Proposal(Proposal),
    AcceptProposal(String),
    Report(Report),
    BudgetReconciled {
        client_id: String,
        before: String,
        after: String,
        budget_revision: i64,
        month: String,
    },
}
model!(Input {
    expected_revision: i64,
    action: Action
});
model!(MemberTotal {
    member_id: String,
    name: String,
    obligation: String,
    bill_payments: String,
    settlements_sent: String,
    settlements_received: String,
    net_contribution: String,
    remaining: String,
    percent: u16
});
model!(Portion {
    member_id: String,
    amount: String,
    percent: u16
});
model!(Bill { agreement_id:String, version_id:String, name:String, category:String, purpose:String, date:String, amount:String, status:String, payer:String, account_id:Option<String>, reimbursement_date:Option<String>, shares:Vec<Portion> });
model!(Category {
    label: String,
    amount: String,
    percent: u16
});
model!(Settlement {
    from_member: String,
    to_member: String,
    amount: String
});
model!(Summary { month:String, total:String, necessary:String, discretionary:String, confirmed_paid:String, known_income:Option<String>, income_complete:bool, unpaid_bills:String, member_totals:Vec<MemberTotal>, categories:Vec<Category>, bills:Vec<Bill>, settlements:Vec<Settlement>, warnings:Vec<String> });

pub fn text(s: &str, label: &str, max: usize) -> Result<String, String> {
    let v = s.trim();
    if v.is_empty() || v.chars().count() > max || v.chars().any(char::is_control) {
        Err(format!(
            "{label} needs 1–{max} characters without control characters."
        ))
    } else {
        Ok(v.into())
    }
}
pub fn date(v: &str) -> Result<NaiveDate, String> {
    let d =
        NaiveDate::parse_from_str(v, "%Y-%m-%d").map_err(|_| "Enter a real YYYY-MM-DD date.")?;
    if !(1900..=2100).contains(&d.year()) || d.format("%Y-%m-%d").to_string() != v {
        return Err("Dates must be YYYY-MM-DD between 1900 and 2100.".into());
    }
    Ok(d)
}
pub fn bounds(month: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let start = date(&format!("{month}-01"))?;
    let end = if start.month() == 12 {
        NaiveDate::from_ymd_opt(start.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1)
    }
    .ok_or("Invalid month.")?
    .pred_opt()
    .ok_or("Invalid month.")?;
    Ok((start, end))
}
fn id(v: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(v)
        .map(|_| ())
        .map_err(|_| "A stable record identifier is required.".into())
}
fn member<'a>(d: &'a Data, id: &str) -> Result<&'a Member, String> {
    d.members
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| "Choose a household member.".into())
}
fn active(m: &Member, on: &str) -> bool {
    m.start.as_str() <= on && m.end.as_ref().is_none_or(|end| end.as_str() >= on)
}
fn range(start: &str, end: &Option<String>) -> Result<(), String> {
    date(start)?;
    if let Some(end) = end {
        date(end)?;
        if end.as_str() < start {
            return Err("End date precedes the start date.".into());
        }
    }
    Ok(())
}
fn amount(s: &mut String) -> Result<i64, String> {
    let c = parse_money(s.trim())?;
    *s = money(c);
    Ok(c)
}
fn unique<'a>(values: impl Iterator<Item = &'a String>) -> bool {
    let mut seen = BTreeSet::new();
    values.into_iter().all(|v| seen.insert(v))
}
fn weight(s: &str) -> Result<i64, String> {
    let v = parse_money(s)?;
    if v > 1_000_000_000 {
        return Err("Allocation weight is too large.".into());
    }
    Ok(v)
}
pub fn split(total: i64, a: &Allocation, d: &Data, on: &str) -> Result<Vec<Portion>, String> {
    if !(0..=100_000_000_000).contains(&total)
        || a.shares.is_empty()
        || a.shares.len() > 100
        || !unique(a.shares.iter().map(|s| &s.member_id))
    {
        return Err("Choose 1–100 distinct participating members.".into());
    }
    if !["equal", "percentage", "income", "individual", "fixed"].contains(&a.method.as_str()) {
        return Err("Choose an allocation method.".into());
    }
    if a.method == "individual" && a.shares.len() != 1 {
        return Err("Individual responsibility requires one member.".into());
    }
    let mut parts = vec![];
    let mut fixed = 0_i64;
    let mut weights = 0_i64;
    for s in &a.shares {
        let m = member(d, &s.member_id)?;
        if !active(m, on) {
            return Err(format!("{} is not a member on {on}.", m.name));
        }
        let f = parse_money(&s.fixed)?;
        fixed = fixed.checked_add(f).ok_or("Allocation is too large.")?;
        let w = match a.method.as_str() {
            "equal" | "individual" => {
                if s.weight == "0" || s.weight == "0.00" {
                    0
                } else {
                    1
                }
            }
            "percentage" => weight(&s.weight)?,
            "fixed" => 0,
            "income" => {
                if s.weight == "0" || s.weight == "0.00" {
                    0
                } else {
                    if !m.consent.income {
                        return Err(format!("{} has not authorized income sharing.", m.name));
                    }
                    m.monthly_income
                        .as_ref()
                        .ok_or_else(|| format!("Income is unknown for {}.", m.name))
                        .and_then(|v| parse_money(v))?
                }
            }
            _ => unreachable!(),
        };
        weights = weights.checked_add(w).ok_or("Allocation is too large.")?;
        parts.push((s.member_id.clone(), f, w, 0_i64));
    }
    if fixed > total {
        return Err(format!(
            "Fixed contributions exceed the expense by ${}.",
            money(fixed - total)
        ));
    }
    let rest = total - fixed;
    if rest > 0 && weights == 0 {
        return Err(format!("${} remains unallocated. Choose remainder participants or complete fixed contributions.",money(rest)));
    }
    if a.method == "percentage" && rest > 0 && weights != 10000 {
        return Err("Remainder percentages must total exactly 100%.".into());
    }
    if rest > 0 {
        let mut distributed = 0;
        for p in &mut parts {
            let n = i128::from(rest) * i128::from(p.2);
            let whole = (n / i128::from(weights)) as i64;
            p.1 += whole;
            p.3 = (n % i128::from(weights)) as i64;
            distributed += whole
        }
        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_by(|&a, &b| {
            parts[b]
                .3
                .cmp(&parts[a].3)
                .then(parts[a].0.cmp(&parts[b].0))
        });
        for &i in order.iter().take((rest - distributed) as usize) {
            parts[i].1 += 1
        }
    }
    Ok(parts
        .into_iter()
        .map(|(member_id, c, _, _)| Portion {
            member_id,
            amount: money(c),
            percent: if total == 0 {
                0
            } else {
                ((i128::from(c) * 10000) / i128::from(total)) as u16
            },
        })
        .collect())
}
fn validate_allocation(a: &Allocation, d: &Data, on: &str, total: i64) -> Result<(), String> {
    split(total, a, d, on).map(|_| ())
}
fn rule_for<'a>(d: &'a Data, category: &str, on: &str) -> Option<&'a Rule> {
    d.rules
        .iter()
        .filter(|r| {
            r.category == category
                && r.effective.as_str() <= on
                && r.until.as_ref().is_none_or(|end| end.as_str() >= on)
        })
        .max_by_key(|r| r.effective.as_str())
}
fn allocation_for<'a>(d: &'a Data, e: &'a Expense, on: &str) -> Result<&'a Allocation, String> {
    e.allocation
        .as_ref()
        .or_else(|| rule_for(d, &e.category, on).map(|r| &r.allocation))
        .ok_or_else(|| {
            format!(
                "{} needs an allocation or an applicable category rule.",
                e.name
            )
        })
}
fn replace<T>(v: &mut Vec<T>, value: T, key: impl Fn(&T) -> &String) {
    if let Some(i) = v.iter().position(|x| key(x) == key(&value)) {
        v[i] = value
    } else {
        v.push(value)
    }
}
pub fn apply(d: &mut Data, action: &mut Action) -> Result<(), String> {
    if d.archived && !matches!(action, Action::Archive(false) | Action::Report(_)) {
        return Err("Reopen this archived household before changing it.".into());
    }
    match action {
        Action::Create(name) | Action::Rename(name) => {
            *name = text(name, "Household name", 100)?;
            d.name = name.clone()
        }
        Action::Archive(v) => d.archived = *v,
        Action::Member(m) => {
            id(&m.id)?;
            m.name = text(&m.name, "Member name", 100)?;
            m.relationship = text(&m.relationship, "Relationship", 60)?;
            range(&m.start, &m.end)?;
            if let Some(c) = &m.client_id {
                id(c)?;
                if d.members
                    .iter()
                    .any(|x| x.id != m.id && x.client_id.as_ref() == Some(c))
                {
                    return Err("This client is already linked to this household; edit their membership dates.".into());
                }
            }
            if d.members.len() >= 100 && !d.members.iter().any(|x| x.id == m.id) {
                return Err("Households support up to 100 members.".into());
            }
            if let Some(old) = d.members.iter().find(|x| x.id == m.id) {
                if old.client_id != m.client_id || old.start != m.start {
                    return Err("Linked client and original membership start cannot be rewritten. End the membership instead.".into());
                }
            }
            if m.consent.income
                || m.consent.balances
                || m.consent.transactions
                || m.consent.planning
            {
                m.consent.evidence = text(&m.consent.evidence, "Sharing authorization", 1000)?
            }
            if !m.consent.income {
                m.monthly_income = None
            }
            if let Some(v) = &mut m.monthly_income {
                amount(v)?;
            }
            replace(&mut d.members, m.clone(), |m| &m.id);
        }
        Action::Expense(e) => {
            id(&e.id)?;
            id(&e.agreement_id)?;
            if d.expenses.iter().any(|x| x.id == e.id) {
                return Err("Agreement revisions are immutable.".into());
            }
            if d.expenses.len() >= 1000 {
                return Err(
                    "This household has reached its 1,000 agreement revision limit.".into(),
                );
            }
            e.name = text(&e.name, "Expense name", 120)?;
            e.category = text(&e.category, "Category", 80)?;
            let total = amount(&mut e.amount)?;
            range(&e.effective, &e.until)?;
            date(&e.due)?;
            if ![
                "once",
                "monthly",
                "weekly",
                "biweekly",
                "annual",
                "irregular",
            ]
            .contains(&e.frequency.as_str())
                || !["confirmed", "estimated", "pending"].contains(&e.status.as_str())
                || !["necessary", "discretionary", "sharedDebt"].contains(&e.purpose.as_str())
            {
                return Err("Choose a supported frequency, status and spending type.".into());
            }
            member(d, &e.payer)?;
            if let Some(a) = &e.account_id {
                if !d.accounts.iter().any(|x| &x.id == a) {
                    return Err("Choose an authorized household payment account.".into());
                }
            }
            if let Some(v) = &e.reimbursement_date {
                date(v)?;
            }
            if e.note.len() > 2000 {
                return Err("Expense notes exceed 2,000 characters.".into());
            }
            validate_allocation(allocation_for(d, e, &e.effective)?, d, &e.effective, total)?;
            d.expenses.push(e.clone());
        }
        Action::Rule(r) => {
            id(&r.id)?;
            if d.rules.len() >= 500 || d.rules.iter().any(|x| x.id == r.id) {
                return Err("Rule revisions are immutable; add a new effective-dated rule.".into());
            }
            r.category = text(&r.category, "Category", 80)?;
            range(&r.effective, &r.until)?;
            if r.note.len() > 2000 {
                return Err("Rule notes exceed 2,000 characters.".into());
            } // validate percentage/fixed rules against the actual expense during preview
            if r.allocation.shares.is_empty() {
                return Err("Select rule participants.".into());
            }
            for s in &r.allocation.shares {
                member(d, &s.member_id)?;
            }
            d.rules.push(r.clone());
        }
        Action::Account(a) => {
            id(&a.id)?;
            a.label = text(&a.label, "Account label", 100)?;
            date(&a.balance_date)?;
            if a.owners.is_empty() || !unique(a.owners.iter()) {
                return Err("Choose distinct account owners.".into());
            }
            for owner in &a.owners {
                let m = member(d, owner)?;
                if !m.consent.balances {
                    return Err(format!(
                        "{} has not authorized shared account balances.",
                        m.name
                    ));
                }
            }
            if let Some(v) = &mut a.opening_balance {
                let negative = v.trim().starts_with('-');
                let c = parse_money(v.trim().trim_start_matches('-'))?;
                *v = money(if negative { -c } else { c });
            }
            if d.accounts.len() >= 100 && !d.accounts.iter().any(|x| x.id == a.id) {
                return Err("Account limit reached.".into());
            }
            replace(&mut d.accounts, a.clone(), |a| &a.id);
        }
        Action::Payment(p) => {
            id(&p.id)?;
            if d.payments.len() >= 5000 || d.payments.iter().any(|x| x.id == p.id) {
                return Err(
                    "Payments are immutable; void an incorrect entry and record its replacement."
                        .into(),
                );
            }
            let c = amount(&mut p.amount)?;
            if c == 0 {
                return Err("Payment must be greater than zero.".into());
            }
            date(&p.date)?;
            bounds(&p.period)?;
            let payer = member(d, &p.from_member)?;
            if p.kind == "income" && !payer.consent.income {
                return Err("Income sharing has not been authorized.".into());
            }
            if let Some(to) = &p.to_member {
                member(d, to)?;
                if to == &p.from_member {
                    return Err("Choose a different receiving member.".into());
                }
            }
            if ![
                "bill",
                "refund",
                "settlement",
                "contribution",
                "transfer",
                "income",
            ]
            .contains(&p.kind.as_str())
                || !["planned", "recorded", "confirmed"].contains(&p.state.as_str())
            {
                return Err("Choose a supported payment kind and confirmation status.".into());
            }
            if ["bill", "refund"].contains(&p.kind.as_str()) {
                if !p
                    .expense_id
                    .as_ref()
                    .is_some_and(|id| d.expenses.iter().any(|x| &x.agreement_id == id))
                {
                    return Err("Link this payment to a household expense.".into());
                }
            } else if p.expense_id.is_some() {
                return Err(
                    "Internal movements must not be recorded as household expenses.".into(),
                );
            }
            if ["settlement", "contribution", "transfer"].contains(&p.kind.as_str())
                && p.to_member.is_none()
            {
                return Err("Choose the receiving member for an internal movement.".into());
            }
            for a in [&p.account_id, &p.destination_account]
                .into_iter()
                .flatten()
            {
                if !d.accounts.iter().any(|x| &x.id == a) {
                    return Err("Choose a household account.".into());
                }
            }
            if p.note.len() > 2000 {
                return Err("Payment notes exceed 2,000 characters.".into());
            }
            d.payments.push(p.clone());
        }
        Action::VoidPayment(p) => {
            if !d.payments.iter().any(|x| &x.id == p) || d.voids.contains(p) {
                return Err("Choose an active payment to void.".into());
            }
            d.voids.push(p.clone());
        }
        Action::Goal(g) => {
            id(&g.id)?;
            g.label = text(&g.label, "Goal name", 120)?;
            amount(&mut g.target)?;
            amount(&mut g.reserved)?;
            let total = amount(&mut g.monthly)?;
            if let Some(v) = &g.target_date {
                date(v)?;
            }
            if g.note.len() > 2000 {
                return Err("Goal notes exceed 2,000 characters.".into());
            }
            let on = d
                .members
                .iter()
                .map(|m| m.start.as_str())
                .max()
                .ok_or("Add members first.")?;
            validate_allocation(&g.allocation, d, on, total)?;
            if d.goals.len() >= 50 && !d.goals.iter().any(|x| x.id == g.id) {
                return Err("Goal limit reached.".into());
            }
            replace(&mut d.goals, g.clone(), |x| &x.id);
        }
        Action::Proposal(p) => {
            id(&p.id)?;
            p.name = text(&p.name, "Scenario name", 120)?;
            if d.proposals.len() >= 100 || d.proposals.iter().any(|x| x.id == p.id) {
                return Err("Scenario limit reached or duplicate identifier.".into());
            }
            let mut proposed = d.clone();
            for m in &p.members {
                apply(&mut proposed, &mut Action::Member(m.clone()))?
            }
            for e in &p.expenses {
                apply(&mut proposed, &mut Action::Expense(e.clone()))?
            }
            if p.note.len() > 2000 {
                return Err("Scenario notes exceed 2,000 characters.".into());
            }
            d.proposals.push(p.clone());
        }
        Action::AcceptProposal(id) => {
            if d.accepted.contains(id) {
                return Err("Scenario is already accepted.".into());
            }
            let p = d
                .proposals
                .iter()
                .find(|p| &p.id == id)
                .ok_or("Choose a saved scenario.")?
                .clone();
            for m in p.members {
                apply(d, &mut Action::Member(m))?
            }
            for e in p.expenses {
                apply(d, &mut Action::Expense(e))?
            }
            d.accepted.push(id.clone());
        }
        Action::Report(r) => {
            id(&r.id)?;
            bounds(&r.month)?;
            if d.reports.len() >= 100
                || d.reports.iter().any(|x| x.id == r.id)
                || r.body.len() > 1_000_000
            {
                return Err("Report limit reached.".into());
            }
            d.reports.push(r.clone());
        }
        Action::BudgetReconciled {
            client_id,
            before,
            after,
            ..
        } => {
            id(client_id)?;
            if before.len() > 65536 || after.len() > 65536 {
                return Err("Budget history is too large.".into());
            }
        }
    }
    Ok(())
}
fn occurrence(e: &Expense, on: NaiveDate) -> Result<bool, String> {
    let first = date(&e.due)?;
    if on < first {
        return Ok(false);
    }
    let delta = (on - first).num_days();
    Ok(match e.frequency.as_str() {
        "once" | "irregular" => on == first,
        "weekly" => delta % 7 == 0,
        "biweekly" => delta % 14 == 0,
        "monthly" => {
            let next = if on.month() == 12 {
                NaiveDate::from_ymd_opt(on.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(on.year(), on.month() + 1, 1)
            }
            .ok_or("Invalid date.")?;
            on.day()
                == first
                    .day()
                    .min(next.pred_opt().ok_or("Invalid date.")?.day())
        }
        "annual" => {
            on.month() == first.month()
                && on.day()
                    == first.day().min(if on.month() == 2 && first.day() == 29 {
                        NaiveDate::from_ymd_opt(on.year(), 3, 1)
                            .ok_or("Invalid date.")?
                            .pred_opt()
                            .ok_or("Invalid date.")?
                            .day()
                    } else {
                        first.day()
                    })
        }
        _ => false,
    })
}
/// Recurrence and effective dates use the agreement in force on each due date.
/// Midmonth changes affect later due dates; no implicit daily proration of an already due bill.
pub fn summary(d: &Data, month: &str) -> Result<Summary, String> {
    let (start, end) = bounds(month)?;
    let mut bills = vec![];
    let mut warnings = vec![];
    let ids: BTreeSet<_> = d.expenses.iter().map(|e| e.agreement_id.as_str()).collect();
    for n in 0..=(end - start).num_days() {
        let on = start + Duration::days(n);
        let day = on.to_string();
        for id in &ids {
            let e = d
                .expenses
                .iter()
                .filter(|e| {
                    e.agreement_id == *id
                        && e.effective <= day
                        && e.until.as_ref().is_none_or(|u| u >= &day)
                })
                .max_by_key(|e| e.effective.as_str());
            let Some(e) = e else { continue };
            if !occurrence(e, on)? {
                continue;
            }
            let a = allocation_for(d, e, &day);
            let shares = a.and_then(|a| split(parse_money(&e.amount)?, a, d, &day));
            match shares {
                Ok(shares) => bills.push(Bill {
                    agreement_id: e.agreement_id.clone(),
                    version_id: e.id.clone(),
                    name: e.name.clone(),
                    category: e.category.clone(),
                    purpose: e.purpose.clone(),
                    date: day.clone(),
                    amount: e.amount.clone(),
                    status: e.status.clone(),
                    payer: e.payer.clone(),
                    account_id: e.account_id.clone(),
                    reimbursement_date: e.reimbursement_date.clone(),
                    shares,
                }),
                Err(err) => warnings.push(format!("{} on {day}: {err}", e.name)),
            }
        }
    }
    let mut cats: BTreeMap<String, i64> = BTreeMap::new();
    let mut obligations: BTreeMap<String, i64> = BTreeMap::new();
    let mut total = 0;
    let mut necessary = 0;
    let mut discretionary = 0;
    for b in &bills {
        let c = parse_money(&b.amount)?;
        total += c;
        *cats.entry(b.category.clone()).or_default() += c;
        if b.purpose == "necessary" {
            necessary += c
        } else if b.purpose == "discretionary" {
            discretionary += c
        }
        for p in &b.shares {
            *obligations.entry(p.member_id.clone()).or_default() += parse_money(&p.amount)?
        }
        if b.status != "confirmed" {
            warnings.push(format!("{} is {} planning information.", b.name, b.status));
        }
    }
    let mut paid: BTreeMap<String, i64> = BTreeMap::new();
    let mut sent: BTreeMap<String, i64> = BTreeMap::new();
    let mut received: BTreeMap<String, i64> = BTreeMap::new();
    let mut bybill: BTreeMap<String, i64> = BTreeMap::new();
    for p in d
        .payments
        .iter()
        .filter(|p| p.period == month && p.state == "confirmed" && !d.voids.contains(&p.id))
    {
        let c = parse_money(&p.amount)?;
        match p.kind.as_str() {
            "bill" | "refund" => {
                let c = if p.kind == "refund" { -c } else { c };
                *paid.entry(p.from_member.clone()).or_default() += c;
                *bybill
                    .entry(p.expense_id.clone().ok_or("Missing payment expense.")?)
                    .or_default() += c
            }
            "settlement" | "contribution" => {
                *sent.entry(p.from_member.clone()).or_default() += c;
                *received
                    .entry(p.to_member.clone().ok_or("Missing receiving member.")?)
                    .or_default() += c
            }
            _ => {}
        }
    }
    if bybill.values().any(|&x| x < 0) {
        return Err("Refunds exceed confirmed payments for an expense in this period.".into());
    }
    let confirmed_paid: i64 = paid.values().sum();
    let mut totals = vec![];
    let mut debtors = vec![];
    let mut creditors = vec![];
    let mut known_income = 0;
    let mut income_complete = true;
    for m in &d.members {
        if active(m, &end.to_string()) && m.start <= end.to_string() {
            if m.consent.income {
                if let Some(v) = &m.monthly_income {
                    known_income += parse_money(v)?
                } else {
                    income_complete = false
                }
            } else {
                income_complete = false
            }
        }
        let o = *obligations.get(&m.id).unwrap_or(&0);
        let p = *paid.get(&m.id).unwrap_or(&0);
        let s = *sent.get(&m.id).unwrap_or(&0);
        let r = *received.get(&m.id).unwrap_or(&0);
        let net = p + s - r;
        let balance = o - net;
        if balance > 0 {
            debtors.push((m.id.clone(), balance))
        } else if balance < 0 {
            creditors.push((m.id.clone(), -balance))
        }
        totals.push(MemberTotal {
            member_id: m.id.clone(),
            name: m.name.clone(),
            obligation: money(o),
            bill_payments: money(p),
            settlements_sent: money(s),
            settlements_received: money(r),
            net_contribution: money(net),
            remaining: money(balance),
            percent: if total == 0 {
                0
            } else {
                ((i128::from(o) * 10000) / i128::from(total)) as u16
            },
        });
    }
    let mut settlements = vec![];
    for (debtor, due) in &mut debtors {
        for (creditor, credit) in &mut creditors {
            let c = (*due).min(*credit);
            if c > 0 {
                settlements.push(Settlement {
                    from_member: debtor.clone(),
                    to_member: creditor.clone(),
                    amount: money(c),
                });
                *due -= c;
                *credit -= c
            }
        }
    }
    if !income_complete {
        warnings
            .push("Income is incomplete or restricted; affordability is not established.".into())
    }
    if d.members.len() < 2 {
        warnings.push("Add at least two members to complete this household.".into())
    }
    if d.payments
        .iter()
        .any(|p| p.period == month && p.state != "confirmed" && !d.voids.contains(&p.id))
    {
        warnings.push(
            "Planned and unverified payments are excluded from confirmed contributions.".into(),
        )
    }
    let mut expected: BTreeMap<String, i64> = BTreeMap::new();
    for b in &bills {
        *expected.entry(b.agreement_id.clone()).or_default() += parse_money(&b.amount)?;
    }
    let unpaid: i64 = expected
        .iter()
        .map(|(id, c)| (c - bybill.get(id).unwrap_or(&0)).max(0))
        .sum();
    Ok(Summary {
        month: month.into(),
        total: money(total),
        necessary: money(necessary),
        discretionary: money(discretionary),
        confirmed_paid: money(confirmed_paid),
        known_income: if known_income == 0 && !income_complete {
            None
        } else {
            Some(money(known_income))
        },
        income_complete,
        unpaid_bills: money(unpaid),
        member_totals: totals,
        categories: cats
            .into_iter()
            .map(|(label, c)| Category {
                label,
                amount: money(c),
                percent: if total == 0 {
                    0
                } else {
                    ((i128::from(c) * 10000) / i128::from(total)) as u16
                },
            })
            .collect(),
        bills,
        settlements,
        warnings,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn member(n: u32) -> Member {
        Member {
            id: format!("00000000-0000-4000-8000-{n:012}"),
            client_id: None,
            name: format!("Member {n}"),
            relationship: "roommate".into(),
            start: "2026-01-01".into(),
            end: None,
            consent: Consent {
                income: false,
                balances: false,
                transactions: false,
                planning: false,
                evidence: String::new(),
            },
            monthly_income: None,
        }
    }
    pub fn data() -> Data {
        let mut d = Data::new("Synthetic Oak Street".into());
        d.members = (1..=3).map(member).collect();
        d
    }
    pub fn allocation(d: &Data, method: &str, weights: &[&str]) -> Allocation {
        Allocation {
            method: method.into(),
            shares: d
                .members
                .iter()
                .zip(weights)
                .map(|(m, w)| Share {
                    member_id: m.id.clone(),
                    fixed: "0.00".into(),
                    weight: (*w).into(),
                })
                .collect(),
        }
    }
    pub fn expense(d: &Data) -> Expense {
        Expense {
            id: uuid::Uuid::new_v4().to_string(),
            agreement_id: uuid::Uuid::new_v4().to_string(),
            name: "Rent".into(),
            category: "Housing".into(),
            amount: "2400.00".into(),
            frequency: "monthly".into(),
            due: "2026-10-01".into(),
            until: None,
            effective: "2026-10-01".into(),
            status: "confirmed".into(),
            purpose: "necessary".into(),
            payer: d.members[0].id.clone(),
            account_id: None,
            reimbursement_date: Some("2026-10-05".into()),
            allocation: Some(allocation(d, "percentage", &["25", "37.5", "37.5"])),
            note: String::new(),
        }
    }
    pub fn payment(
        d: &Data,
        e: &Expense,
        n: usize,
        amount: &str,
        kind: &str,
        to: Option<usize>,
    ) -> Payment {
        Payment {
            id: uuid::Uuid::new_v4().to_string(),
            expense_id: if ["bill", "refund"].contains(&kind) {
                Some(e.agreement_id.clone())
            } else {
                None
            },
            period: "2026-10".into(),
            from_member: d.members[n].id.clone(),
            to_member: to.map(|i| d.members[i].id.clone()),
            account_id: None,
            destination_account: None,
            date: "2026-10-01".into(),
            amount: amount.into(),
            kind: kind.into(),
            state: "confirmed".into(),
            source: None,
            note: String::new(),
        }
    }
    #[test]
    fn roommates_exact_and_payer_fronting_is_not_double_spending() {
        let mut d = data();
        let e = expense(&d);
        apply(&mut d, &mut Action::Expense(e.clone())).unwrap();
        let p = payment(&d, &e, 0, "2400", "bill", None);
        apply(&mut d, &mut Action::Payment(p)).unwrap();
        let s = summary(&d, "2026-10").unwrap();
        assert_eq!(s.total, "2400.00");
        assert_eq!(
            s.member_totals
                .iter()
                .map(|x| x.obligation.as_str())
                .collect::<Vec<_>>(),
            ["600.00", "900.00", "900.00"]
        );
        assert_eq!(s.settlements.len(), 2);
        assert!(s.settlements.iter().all(|s| s.amount == "900.00"));
        let p = payment(&d, &e, 1, "400", "settlement", Some(0));
        apply(&mut d, &mut Action::Payment(p)).unwrap();
        let s = summary(&d, "2026-10").unwrap();
        assert_eq!(s.total, "2400.00");
        assert_eq!(s.member_totals[1].remaining, "500.00");
        assert_eq!(s.confirmed_paid, "2400.00");
    }
    #[test]
    fn equal_rounding_is_stable_not_vector_order() {
        let d = data();
        let a = allocation(&d, "equal", &["1", "1", "1"]);
        let p = split(100, &a, &d, "2026-10-01").unwrap();
        assert_eq!(
            p.iter().map(|p| p.amount.as_str()).collect::<Vec<_>>(),
            ["0.34", "0.33", "0.33"]
        );
        let mut reverse = a.clone();
        reverse.shares.reverse();
        let q = split(100, &reverse, &d, "2026-10-01").unwrap();
        assert_eq!(p[0], q[2]);
    }
    #[test]
    fn two_members_fixed_and_hybrid() {
        let mut d = data();
        d.members.pop();
        let mut a = allocation(&d, "equal", &["0", "1"]);
        a.shares[0].fixed = "500".into();
        let p = split(240000, &a, &d, "2026-10-01").unwrap();
        assert_eq!(p[0].amount, "500.00");
        assert_eq!(p[1].amount, "1900.00");
        a.method = "fixed".into();
        assert!(split(240000, &a, &d, "2026-10-01").is_err());
        a.shares[1].fixed = "1900".into();
        assert!(split(240000, &a, &d, "2026-10-01").is_ok());
        a.shares[1].fixed = "1900.01".into();
        assert!(split(240000, &a, &d, "2026-10-01").is_err());
    }
    #[test]
    fn income_requires_consent_known_nonzero_basis() {
        let mut d = data();
        let a = allocation(&d, "income", &["1", "1", "1"]);
        assert!(split(10000, &a, &d, "2026-10-01").is_err());
        for (i, m) in d.members.iter_mut().enumerate() {
            m.consent.income = true;
            m.monthly_income = Some(if i == 0 { "0" } else { "1000" }.into());
        }
        let p = split(10000, &a, &d, "2026-10-01").unwrap();
        assert_eq!(
            p.iter().map(|x| x.amount.as_str()).collect::<Vec<_>>(),
            ["0.00", "50.00", "50.00"]
        );
        for m in &mut d.members {
            m.monthly_income = Some("0".into())
        }
        assert!(split(10000, &a, &d, "2026-10-01").is_err());
        d.members[0].monthly_income = None;
        assert!(split(10000, &a, &d, "2026-10-01").is_err());
    }
    #[test]
    fn invalid_percent_duplicate_members_and_overallocation_rejected() {
        let d = data();
        let mut a = allocation(&d, "percentage", &["25", "37.5", "37.49"]);
        assert!(split(240000, &a, &d, "2026-10-01").is_err());
        a.shares[2].weight = "37.5".into();
        a.shares[2].member_id = a.shares[0].member_id.clone();
        assert!(split(240000, &a, &d, "2026-10-01").is_err());
    }
    #[test]
    fn effective_midmonth_changes_preserve_due_obligations_and_temporary_rules() {
        let mut d = data();
        let e = expense(&d);
        apply(&mut d, &mut Action::Expense(e.clone())).unwrap();
        let mut newer = e.clone();
        newer.id = uuid::Uuid::new_v4().to_string();
        newer.amount = "3000".into();
        newer.effective = "2026-10-15".into();
        newer.until = Some("2026-11-30".into());
        apply(&mut d, &mut Action::Expense(newer)).unwrap();
        assert_eq!(summary(&d, "2026-10").unwrap().total, "2400.00");
        assert_eq!(summary(&d, "2026-11").unwrap().total, "3000.00");
        assert_eq!(summary(&d, "2026-12").unwrap().total, "2400.00");
    }
    #[test]
    fn missed_pending_refunds_voids_and_multiple_payers() {
        let mut d = data();
        let e = expense(&d);
        apply(&mut d, &mut Action::Expense(e.clone())).unwrap();
        let mut planned = payment(&d, &e, 0, "2400", "bill", None);
        planned.state = "planned".into();
        apply(&mut d, &mut Action::Payment(planned)).unwrap();
        assert_eq!(summary(&d, "2026-10").unwrap().confirmed_paid, "0.00");
        for (n, c) in [(0, "1000"), (1, "1400")] {
            let p = payment(&d, &e, n, c, "bill", None);
            apply(&mut d, &mut Action::Payment(p)).unwrap();
        }
        let p = payment(&d, &e, 1, "100", "refund", None);
        let id = p.id.clone();
        apply(&mut d, &mut Action::Payment(p)).unwrap();
        assert_eq!(summary(&d, "2026-10").unwrap().confirmed_paid, "2300.00");
        apply(&mut d, &mut Action::VoidPayment(id)).unwrap();
        assert_eq!(summary(&d, "2026-10").unwrap().confirmed_paid, "2400.00");
        let p = payment(&d, &e, 0, "2500", "refund", None);
        apply(&mut d, &mut Action::Payment(p)).unwrap();
        assert!(summary(&d, "2026-10").is_err());
    }
    #[test]
    fn membership_ends_flag_new_obligations_without_losing_previous_bills() {
        let mut d = data();
        let e = expense(&d);
        apply(&mut d, &mut Action::Expense(e)).unwrap();
        d.members[2].end = Some("2026-10-15".into());
        assert_eq!(summary(&d, "2026-10").unwrap().total, "2400.00");
        let s = summary(&d, "2026-11").unwrap();
        assert_eq!(s.total, "0.00");
        assert!(s.warnings.iter().any(|w| w.contains("not a member")));
    }
    #[test]
    fn scenario_preview_does_not_change_active_agreement() {
        let mut d = data();
        let e = expense(&d);
        apply(&mut d, &mut Action::Expense(e.clone())).unwrap();
        let mut future = e;
        future.id = uuid::Uuid::new_v4().to_string();
        future.amount = "2600".into();
        let p = Proposal {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Rent increase".into(),
            members: vec![],
            expenses: vec![future],
            note: String::new(),
        };
        let pid = p.id.clone();
        apply(&mut d, &mut Action::Proposal(p)).unwrap();
        assert_eq!(summary(&d, "2026-10").unwrap().total, "2400.00");
        apply(&mut d, &mut Action::AcceptProposal(pid.clone())).unwrap();
        assert_eq!(summary(&d, "2026-10").unwrap().total, "2600.00");
        assert!(apply(&mut d, &mut Action::AcceptProposal(pid)).is_err());
    }
}
