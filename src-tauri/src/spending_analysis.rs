//! Live spending analysis. Existing approved sources and immutable purpose reviews only.
use super::*;
use crate::budget::money;
use crate::{
    card_facts as cf, card_review as cr, cards, dashboard, financial_facts as ff, imports,
};
use chrono::{Datelike, Months};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub from_month: String,
    pub through_month: String,
    pub account_id: Option<String>,
    pub search: String,
    pub category: Option<String>,
    pub role: Option<String>,
    pub page: usize,
    pub page_size: usize,
    pub compare_previous: bool,
    pub opportunity: Option<String>,
}
#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Necessary,
    Discretionary,
}
impl Role {
    fn label(self) -> &'static str {
        match self {
            Self::Necessary => "necessary",
            Self::Discretionary => "discretionary",
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    pub context: String,
    pub query: Input,
    pub row_id: String,
    pub approval_id: String,
    pub role: Role,
    pub reason: String,
    pub acknowledge: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub approval_id: String,
    pub date: String,
    pub merchant: String,
    pub amount: String,
    pub category: String,
    pub role: String,
    pub account_id: String,
    pub account_label: String,
    pub source_label: String,
    pub editable: bool,
    #[serde(skip)]
    cents: i64,
    #[serde(skip)]
    purchase: bool,
    #[serde(skip)]
    record_id: String,
    #[serde(skip)]
    card: Option<(String, usize)>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub label: String,
    pub amount: String,
    pub share: String,
    pub basis_points: u16,
    pub scale: u16,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub spending: String,
    pub necessary: String,
    pub discretionary: String,
    pub unclassified: String,
    pub refunds: String,
    pub net_spending: String,
    pub debt_payments: String,
    pub transfers: String,
    pub discretionary_share: Option<String>,
    pub necessary_share: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opportunity {
    pub kind: String,
    pub title: String,
    pub amount: String,
    pub count: usize,
    pub detail: String,
    pub row_ids: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub client_id: String,
    pub context: String,
    pub period_start: String,
    pub period_end: String,
    pub accounts: Vec<(String, String)>,
    pub categories: Vec<String>,
    pub totals: Totals,
    pub previous: Option<Totals>,
    pub spending_change: Option<String>,
    pub mix: Vec<Category>,
    pub discretionary_categories: Vec<Category>,
    pub necessary_categories: Vec<Category>,
    pub rows: Vec<Row>,
    pub matching_rows: usize,
    pub page: usize,
    pub page_size: usize,
    pub opportunities: Vec<Opportunity>,
    pub excluded_unapproved: usize,
    pub excluded_uncategorized: usize,
    pub warnings: Vec<String>,
    pub discretionary_budget: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub reviewed_at: String,
    pub counselor_name: String,
    pub role: String,
    pub approval_id: Option<String>,
    pub reason: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub client_id: String,
    pub row_id: String,
    pub original: imports::Raw,
    pub events: Vec<Event>,
}
struct Context {
    profile: Client,
    bank: imports::Snapshot,
    cards: cards::Snapshot,
    links: Option<cr::Snapshot>,
    facts: Vec<cf::Snapshot>,
    budget: ClientBudget,
}
fn context(c: &Connection, w: &WorkspaceInfo, id: &str) -> Result<Context, String> {
    Ok(Context {
        profile: scoped_client(c, w, id)?,
        bank: imports_store::read_imports(c, w, id)?,
        cards: cards_store::read(c, w, id)?,
        links: card_review_store::history(c, w, id)?.pop(),
        facts: card_facts_store::versions(c, w, id)?,
        budget: read_budget(c, w, id)?,
    })
}
fn receipt(c: &Context) -> Result<String, String> {
    let body = serde_json::to_vec(&(
        &c.profile.id,
        &c.profile.updated_at,
        c.bank.revision,
        c.cards.revision,
        c.links.as_ref().map(|s| s.revision),
        c.facts.last().map(|s| s.revision),
        c.budget.revision,
    ))
    .map_err(|_| STORAGE_ERROR)?;
    Ok(format!("{:x}", Sha256::digest(body)))
}
fn add(a: i64, b: i64) -> Result<i64, String> {
    a.checked_add(b)
        .ok_or_else(|| "Spending totals exceed the supported range.".into())
}
fn percent(n: i64, d: i64) -> Option<String> {
    if d <= 0 {
        return None;
    }
    let b = (i128::from(n) * 10000 + i128::from(d) / 2) / i128::from(d);
    Some(format!("{}.{:02}%", b / 100, b % 100))
}
fn number(s: &str) -> Result<i64, String> {
    s.replace('.', "")
        .parse()
        .map_err(|_| "Invalid saved total.".into())
}
fn rows(c: &Context) -> Result<(Vec<Row>, usize, usize), String> {
    let mut out = vec![];
    let (mut unapproved, mut uncategorized) = (0, 0);
    for r in &c.bank.records {
        if r.status != imports::Status::Approved {
            unapproved += 1;
            continue;
        }
        if r.fields.category.trim().is_empty() {
            uncategorized += 1;
            continue;
        }
        let Some(date) = r.fields.date.as_ref() else {
            continue;
        };
        let n = imports::signed_money(
            r.fields
                .amount
                .as_deref()
                .ok_or("Approved amount missing.")?,
        )?;
        let spending = matches!(r.fields.kind, imports::Kind::Expense | imports::Kind::Fee)
            || (r.fields.kind == imports::Kind::Interest && n < 0);
        let purpose = c.bank.financial_facts.data.as_ref().and_then(|d| {
            d.classifications
                .iter()
                .find(|x| x.record_id == r.id && ff::active(x, r, &c.bank.history))
        });
        let role = if spending {
            match purpose.map(|p| &p.role) {
                Some(ff::Role::Necessary) => "necessary",
                Some(ff::Role::Discretionary) => "discretionary",
                _ => "unclassified",
            }
        } else {
            match r.fields.kind {
                imports::Kind::Income | imports::Kind::Interest => "income",
                imports::Kind::Refund => "reimbursement",
                imports::Kind::Transfer => "transfer",
                _ => "balanceMovement",
            }
        };
        let approval = c
            .bank
            .history
            .iter()
            .rev()
            .find(|h| h.record_id == r.id && h.to_status == imports::Status::Approved)
            .ok_or("Approved decision unavailable.")?;
        let label = c
            .bank
            .accounts
            .iter()
            .find(|a| a.id == r.account_id)
            .map(|a| a.label.clone())
            .unwrap_or_else(|| "Bank account".into());
        out.push(Row {
            id: format!("bank:{}", r.id),
            approval_id: approval.id.clone(),
            date: date.clone(),
            merchant: if r.fields.merchant.trim().is_empty() {
                r.fields.description.clone()
            } else {
                r.fields.merchant.clone()
            },
            amount: money(n),
            category: r.fields.category.clone(),
            role: role.into(),
            account_id: format!("bank:{}", r.account_id),
            account_label: label,
            source_label: "Bank statement".into(),
            editable: spending,
            cents: if spending { -n } else { n },
            purchase: r.fields.kind == imports::Kind::Expense,
            record_id: r.id.clone(),
            card: None,
        });
    }
    for s in &c.cards.statements {
        let assignment = c
            .links
            .as_ref()
            .and_then(|l| l.assignments.iter().find(|a| a.statement_id == s.source.id));
        let (account_id, label) = if let Some(a) = assignment {
            let name = c
                .links
                .as_ref()
                .and_then(|l| l.accounts.iter().find(|x| x.id == a.account_id))
                .map(|a| a.label.clone())
                .ok_or("Card identity unavailable.")?;
            (format!("card:{}", a.account_id), name)
        } else {
            (
                format!("card-source:{}", s.source.id),
                format!("{} · unassigned source", s.source.input.account_label),
            )
        };
        for r in &s.rows {
            if r.status != imports::Status::Approved {
                unapproved += 1;
                continue;
            }
            if r.fields.category.trim().is_empty() {
                uncategorized += 1;
                continue;
            }
            let Some(date) = r.fields.date.as_ref() else {
                continue;
            };
            let n = imports::signed_money(
                r.fields
                    .amount
                    .as_deref()
                    .ok_or("Approved card amount missing.")?,
            )?;
            let approval = s
                .history
                .iter()
                .rev()
                .find(|h| h.row == r.row)
                .ok_or("Card approval unavailable.")?;
            let spending = matches!(
                r.fields.kind,
                cards::Kind::Purchase | cards::Kind::Fee | cards::Kind::Interest
            );
            let purpose = c.facts.last().and_then(|f| {
                f.data.classifications.iter().find(|x| {
                    x.statement_id == s.source.id && x.row == r.row && x.approval_id == approval.id
                })
            });
            let role = if spending {
                match purpose.map(|p| &p.role) {
                    Some(cf::Role::Necessary) => "necessary",
                    Some(cf::Role::Discretionary) => "discretionary",
                    None => "unclassified",
                }
            } else {
                match r.fields.kind {
                    cards::Kind::Refund => "reimbursement",
                    cards::Kind::Payment => "debtPayment",
                    _ => "balanceMovement",
                }
            };
            out.push(Row {
                id: format!("card:{}:{}", s.source.id, r.row),
                approval_id: approval.id.clone(),
                date: date.clone(),
                merchant: r.fields.description.clone(),
                amount: money(if r.fields.kind == cards::Kind::Payment {
                    n
                } else {
                    -n
                }),
                category: r.fields.category.clone(),
                role: role.into(),
                account_id: account_id.clone(),
                account_label: label.clone(),
                source_label: format!("Card · {}", s.source.file_name),
                editable: spending,
                cents: if spending || r.fields.kind == cards::Kind::Payment {
                    n
                } else {
                    -n
                },
                purchase: r.fields.kind == cards::Kind::Purchase,
                record_id: String::new(),
                card: Some((s.source.id.clone(), r.row)),
            });
        }
    }
    if out.len() > 20000 {
        return Err(
            "Analysis exceeds its 20,000-row safety limit. Review a smaller retained source set."
                .into(),
        );
    }
    // Identical rows across overlapping card sources cannot be safely counted twice.
    let duplicate_key = |r: &Row| {
        (
            if r.account_id.starts_with("card-source:") {
                format!("unassigned:{}", r.account_label.to_lowercase())
            } else {
                r.account_id.clone()
            },
            r.date.clone(),
            r.merchant.to_lowercase(),
            r.amount.clone(),
        )
    };
    let mut duplicates: BTreeMap<_, BTreeSet<String>> = BTreeMap::new();
    for r in &out {
        if let Some((source, _)) = &r.card {
            duplicates
                .entry(duplicate_key(r))
                .or_default()
                .insert(source.clone());
        }
    }
    let duplicate_ids = out
        .iter()
        .filter(|r| r.card.is_some() && duplicates[&duplicate_key(r)].len() > 1)
        .map(|r| r.id.clone())
        .collect::<BTreeSet<_>>();
    if !duplicate_ids.is_empty() {
        out.retain(|r| !duplicate_ids.contains(&r.id));
        uncategorized += duplicate_ids.len();
    }
    Ok((out, unapproved, uncategorized))
}
fn totals(rows: &[Row]) -> Result<Totals, String> {
    let (
        mut spending,
        mut necessary,
        mut discretionary,
        mut unknown,
        mut refunds,
        mut debt,
        mut transfers,
    ) = (0, 0, 0, 0, 0, 0, 0);
    for r in rows {
        match r.role.as_str() {
            "necessary" | "discretionary" | "unclassified" => {
                spending = add(spending, r.cents)?;
                match r.role.as_str() {
                    "necessary" => necessary = add(necessary, r.cents)?,
                    "discretionary" => discretionary = add(discretionary, r.cents)?,
                    _ => unknown = add(unknown, r.cents)?,
                }
            }
            "reimbursement" => refunds = add(refunds, r.cents)?,
            "debtPayment" => debt = add(debt, -r.cents)?,
            "transfer" if r.cents < 0 => transfers = add(transfers, -r.cents)?,
            _ => (),
        }
    }
    Ok(Totals {
        spending: money(spending),
        necessary: money(necessary),
        discretionary: money(discretionary),
        unclassified: money(unknown),
        refunds: money(refunds),
        net_spending: money(add(spending, -refunds)?),
        debt_payments: money(debt),
        transfers: money(transfers),
        necessary_share: percent(necessary, spending),
        discretionary_share: percent(discretionary, spending),
    })
}
fn categories(rows: &[Row], discretionary: bool) -> Result<Vec<Category>, String> {
    let mut amounts: BTreeMap<String, i64> = BTreeMap::new();
    for r in rows.iter().filter(|r| {
        if discretionary {
            r.role == "discretionary"
        } else {
            r.editable
        }
    }) {
        let v = amounts.entry(r.category.clone()).or_default();
        *v = add(*v, r.cents)?;
    }
    let mut amounts = amounts
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .collect::<Vec<_>>();
    amounts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let sum = amounts.iter().try_fold(0, |a, (_, n)| add(a, *n))?;
    let maximum = amounts.first().map_or(0, |(_, n)| *n);
    let mut shares = amounts
        .iter()
        .map(|(_, n)| ((i128::from(*n) * 10000) / i128::from(sum)) as u16)
        .collect::<Vec<_>>();
    let remainder = 10000 - shares.iter().map(|n| usize::from(*n)).sum::<usize>();
    let mut order = (0..amounts.len()).collect::<Vec<_>>();
    if sum > 0 {
        order.sort_by_key(|i| {
            std::cmp::Reverse((i128::from(amounts[*i].1) * 10000) % i128::from(sum))
        });
        for i in order.into_iter().take(remainder) {
            shares[i] += 1;
        }
    }
    Ok(amounts
        .into_iter()
        .enumerate()
        .map(|(i, (label, n))| Category {
            label,
            amount: money(n),
            share: format!("{}.{:02}%", shares[i] / 100, shares[i] % 100),
            basis_points: shares[i],
            scale: ((i128::from(n) * 1000) / i128::from(maximum)) as u16,
        })
        .collect())
}
fn period(q: &Input) -> Result<(String, String, i32), String> {
    if ![10, 25, 50].contains(&q.page_size)
        || q.page > 2000
        || q.search.chars().count() > 120
        || q.search.chars().any(char::is_control)
        || q.category.as_ref().is_some_and(|s| s.chars().count() > 160)
    {
        return Err("Use bounded analysis filters and 10, 25 or 50 rows per page.".into());
    }
    if q.role.as_deref().is_some_and(|v| {
        ![
            "necessary",
            "discretionary",
            "unclassified",
            "income",
            "reimbursement",
            "debtPayment",
            "transfer",
            "balanceMovement",
        ]
        .contains(&v)
    }) {
        return Err("Choose a supported analysis type.".into());
    }
    if q.opportunity.as_deref().is_some_and(|v| {
        ![
            "smallPurchases",
            "subscriptions",
            "overspending",
            "recurringCandidates",
        ]
        .contains(&v)
    }) {
        return Err("Choose a supported opportunity filter.".into());
    }
    let (a, b, n) = dashboard::period(&dashboard::Input {
        from_month: q.from_month.clone(),
        through_month: q.through_month.clone(),
    })?;
    Ok((a.to_string(), b.to_string(), n))
}
fn scoped(rows: &[Row], q: &Input, start: &str, end: &str) -> Vec<Row> {
    rows.iter()
        .filter(|r| {
            r.date.as_str() >= start
                && r.date.as_str() <= end
                && q.account_id.as_ref().is_none_or(|a| a == &r.account_id)
        })
        .cloned()
        .collect()
}
fn table_match(r: &Row, q: &Input) -> bool {
    q.category.as_ref().is_none_or(|s| s == &r.category)
        && q.role.as_ref().is_none_or(|s| s == &r.role)
        && (q.search.trim().is_empty()
            || format!(
                "{} {} {} {}",
                r.merchant, r.category, r.amount, r.account_label
            )
            .to_lowercase()
            .contains(&q.search.trim().to_lowercase()))
}
fn analyze(c: &Context, q: &Input) -> Result<Analysis, String> {
    let (start, end, months) = period(q)?;
    let (all, unapproved, uncategorized) = rows(c)?;
    let accounts = all
        .iter()
        .map(|r| (r.account_id.clone(), r.account_label.clone()))
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .collect::<Vec<_>>();
    if q.account_id
        .as_ref()
        .is_some_and(|id| !accounts.iter().any(|a| &a.0 == id))
    {
        return Err(
            "This account has no eligible approved, categorized transactions. Choose All accounts."
                .into(),
        );
    }
    let selected = scoped(&all, q, &start, &end);
    let total = totals(&selected)?;
    let previous = if q.compare_previous {
        imports::iso_date(&start)?
            .checked_sub_months(Months::new(months as u32))
            .filter(|d| d.year() >= 1900)
            .map(|a| {
                totals(&scoped(
                    &all,
                    q,
                    &a.to_string(),
                    &imports::iso_date(&start)
                        .unwrap()
                        .pred_opt()
                        .unwrap()
                        .to_string(),
                ))
            })
            .transpose()?
    } else {
        None
    };
    let change = previous.as_ref().and_then(|p| {
        let old = number(&p.spending).ok()?;
        let current = number(&total.spending).ok()?;
        if old == 0 {
            None
        } else {
            let delta = current - old;
            let p = percent(delta.abs(), old)?;
            Some(format!("{}{}", if delta < 0 { "−" } else { "+" }, p))
        }
    });
    let budget = if q.account_id.is_none()
        && c.budget.revision > 0
        && c.budget
            .lines
            .iter()
            .any(|l| l.kind == crate::budget::Kind::Discretionary)
        && c.budget
            .lines
            .iter()
            .filter(|l| l.kind == crate::budget::Kind::Discretionary)
            .all(|l| l.amount.is_some())
    {
        Some(
            number(&c.budget.summary.discretionary_expenses)?
                .checked_mul(i64::from(months))
                .ok_or("Budget target exceeds range.")?,
        )
    } else {
        None
    };
    let discretionary = selected
        .iter()
        .filter(|r| r.role == "discretionary")
        .collect::<Vec<_>>();
    let mut opportunities = vec![];
    let small = discretionary
        .iter()
        .filter(|r| r.purchase && r.cents > 0 && r.cents < 1500)
        .copied()
        .collect::<Vec<_>>();
    if small.len() >= 5 {
        let sum = small.iter().try_fold(0, |a, r| add(a, r.cents))?;
        opportunities.push(Opportunity{kind:"smallPurchases".into(),title:"Frequent small purchases".into(),amount:money(sum),count:small.len(),detail:"At least 5 discretionary purchases under $15 in this period. This is a discussion prompt, not an estimate of savings.".into(),row_ids:small.iter().map(|r|r.id.clone()).collect()});
    }
    let subscriptions = c
        .bank
        .merchant_groups
        .iter()
        .filter(|g| {
            g.state == "Confirmed"
                && g.review
                    .as_ref()
                    .is_some_and(|r| r.subscription && !r.invalidated)
        })
        .collect::<Vec<_>>();
    let subscribed = discretionary
        .iter()
        .filter(|r| {
            subscriptions
                .iter()
                .any(|g| g.evidence.iter().any(|e| e.record_id == r.record_id))
        })
        .copied()
        .collect::<Vec<_>>();
    if !subscribed.is_empty() {
        let sum = subscribed.iter().try_fold(0, |a, r| add(a, r.cents))?;
        opportunities.push(Opportunity{kind:"subscriptions".into(),title:"Reviewed discretionary subscriptions".into(),amount:money(sum),count:subscribed.len(),detail:"Actual charges in this period backed by current counselor-confirmed subscription decisions. No recurring obligation is inferred from a merchant name.".into(),row_ids:subscribed.iter().map(|r|r.id.clone()).collect()});
    }
    if let Some(target) = budget {
        let actual = number(&total.discretionary)?;
        if actual > target {
            opportunities.push(Opportunity{kind:"overspending".into(),title:"Discretionary spending above saved budget".into(),amount:money(actual-target),count:discretionary.len(),detail:format!("Compared with {} months of the saved counselor-entered discretionary budget ({}). Statement activity and planning targets remain separate.",months,money(target)),row_ids:discretionary.iter().map(|r|r.id.clone()).collect()});
        }
    }
    let mut candidates: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for r in all
        .iter()
        .filter(|r| r.role == "discretionary" && r.date.as_str() <= end.as_str())
    {
        candidates
            .entry((
                r.account_id.clone(),
                r.merchant
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase(),
            ))
            .or_default()
            .insert(r.date[..7].into());
    }
    let recurring = discretionary
        .iter()
        .filter(|r| {
            candidates
                .get(&(
                    r.account_id.clone(),
                    r.merchant
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_lowercase(),
                ))
                .is_some_and(|m| m.len() >= 3)
                && !subscribed.iter().any(|s| s.id == r.id)
        })
        .copied()
        .collect::<Vec<_>>();
    if !recurring.is_empty() {
        opportunities.push(Opportunity{kind:"recurringCandidates".into(),title:"Possible recurring discretionary merchants".into(),amount:money(recurring.iter().try_fold(0,|a,r|add(a,r.cents))?),count:recurring.len(),detail:"Merchant activity appears in at least 3 distinct months on the same account. These are candidates to discuss, not confirmed subscriptions.".into(),row_ids:recurring.iter().map(|r|r.id.clone()).collect()});
    }
    let mut filtered = selected
        .iter()
        .filter(|r| {
            table_match(r, q)
                && q.opportunity.as_ref().is_none_or(|kind| {
                    opportunities
                        .iter()
                        .find(|o| &o.kind == kind)
                        .is_some_and(|o| o.row_ids.contains(&r.id))
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|a, b| b.date.cmp(&a.date).then(a.id.cmp(&b.id)));
    let matching = filtered.len();
    let page = q.page.min(matching.saturating_sub(1) / q.page_size);
    let visible = filtered
        .into_iter()
        .skip(page * q.page_size)
        .take(q.page_size)
        .collect();
    let warnings=vec!["Approved and categorized source rows only. Missing or stale spending purposes remain Unclassified; category names do not determine necessary/discretionary type.".into(),"Total spending is gross expenses, card purchases, fees and paid interest. Refunds are separate; debt payments and outgoing transfers are excluded from spending. Debt payments show card payments only; other bank principal movements have no inferred purpose.".into(),"Bank cash signs and card liability signs are normalized for the table: an outflow is negative. Account and date filters affect the snapshot; table filters only narrow transaction details.".into(),"Scope is not evidence of complete statement coverage. Potential identical card rows are excluded until source review; their count is included with excluded incomplete rows. Reconcile overlapping sources before relying on this live view.".into()];
    Ok(Analysis {
        client_id: c.profile.id.clone(),
        context: receipt(c)?,
        period_start: start,
        period_end: end,
        accounts,
        categories: selected
            .iter()
            .map(|r| r.category.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        totals: total,
        previous,
        spending_change: change,
        mix: categories(&selected, false)?,
        discretionary_categories: categories(&selected, true)?,
        necessary_categories: categories(
            &selected
                .iter()
                .filter(|r| r.role == "necessary")
                .cloned()
                .collect::<Vec<_>>(),
            false,
        )?,
        rows: visible,
        matching_rows: matching,
        page,
        page_size: q.page_size,
        opportunities,
        excluded_unapproved: unapproved,
        excluded_uncategorized: uncategorized,
        warnings,
        discretionary_budget: budget.map(money),
    })
}
impl Workspace {
    pub fn get_spending_analysis(&mut self, id: &str, q: Input) -> Result<Analysis, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let result = analyze(&context(&tx, &s.info, id)?, &q)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn change_spending_classification(
        &mut self,
        id: &str,
        input: Change,
    ) -> Result<(), String> {
        if !input.acknowledge {
            return Err("Confirm that the spending type was discussed with the client.".into());
        }
        let reason = required_text(&input.reason, "Classification discussion reason", 500)?;
        let c = {
            let s = self.session()?;
            let tx = s.connection.transaction().map_err(storage_error)?;
            let c = context(&tx, &s.info, id)?;
            tx.commit().map_err(storage_error)?;
            c
        };
        if receipt(&c)? != input.context {
            return Err("Source or classification context changed. Reload analysis before saving; nothing was changed.".into());
        }
        let (start, end, _) = period(&input.query)?;
        let (all, _, _) = rows(&c)?;
        let r = scoped(&all, &input.query, &start, &end)
            .into_iter()
            .find(|r| r.id == input.row_id && table_match(r, &input.query))
            .ok_or("The approved, categorized transaction is unavailable in this analysis.")?;
        if !r.editable || r.approval_id != input.approval_id {
            return Err("The transaction no longer has the reviewed spending approval. Reload and review its source.".into());
        }
        if r.role == input.role.label() {
            return Err("Choose a different spending type before saving.".into());
        }
        if let Some((statement_id, row)) = r.card {
            let current = self.get_card_facts(id)?;
            if current.latest_revision != c.facts.last().map_or(0, |f| f.revision)
                || current.current_profile_updated_at != c.profile.updated_at
                || current.choices.link_revision != c.links.as_ref().map_or(0, |l| l.revision)
                || current.current_source_revision != c.bank.revision
                || current
                    .choices
                    .sources
                    .iter()
                    .find(|s| s.id == statement_id)
                    .and_then(|s| s.rows.iter().find(|r| r.row == row))
                    .and_then(|r| r.approval_id.as_ref())
                    != Some(&input.approval_id)
            {
                return Err("Card source changed. Reload before changing its type.".into());
            }
            let data = crate::storage::card_facts_store::purpose_data(
                &current,
                statement_id,
                row,
                input.approval_id,
                match input.role {
                    Role::Necessary => cf::Role::Necessary,
                    Role::Discretionary => cf::Role::Discretionary,
                },
                &reason,
                (&input.query.from_month, &input.query.through_month),
            );
            self.save_card_facts(
                id,
                cf::Input {
                    expected_revision: current.latest_revision,
                    expected_source_revision: current.current_source_revision,
                    expected_profile_updated_at: current.current_profile_updated_at,
                    data,
                    acknowledge: true,
                },
            )?;
        } else {
            let mut data = c.bank.financial_facts.data.clone().unwrap_or_default();
            data.classifications.retain(|x| x.record_id != r.record_id);
            data.classifications.push(ff::Classification {
                record_id: r.record_id,
                approval_id: input.approval_id,
                role: match input.role {
                    Role::Necessary => ff::Role::Necessary,
                    Role::Discretionary => ff::Role::Discretionary,
                },
            });
            self.review_financial_facts(
                id,
                ff::Input {
                    expected_revision: c.bank.revision,
                    data,
                    reason,
                    acknowledge: true,
                },
            )?;
        }
        Ok(())
    }
    pub fn get_spending_classification_history(
        &mut self,
        id: &str,
        row_id: &str,
    ) -> Result<History, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let c = context(&tx, &s.info, id)?;
        let mut events = vec![];
        let original = if let Some(bank_id) = row_id.strip_prefix("bank:") {
            let row = c
                .bank
                .records
                .iter()
                .find(|r| r.id == bank_id)
                .ok_or("Transaction unavailable for this client.")?;
            let mut previous = None;
            for review in &c.bank.financial_facts.history {
                let item = review
                    .data
                    .classifications
                    .iter()
                    .find(|x| x.record_id == bank_id);
                let next = item.map(|x| (&x.role, &x.approval_id));
                if next != previous {
                    events.push(Event {
                        reviewed_at: review.reviewed_at.clone(),
                        counselor_name: review.counselor_name.clone(),
                        role: item
                            .map(|x| match x.role {
                                ff::Role::Necessary => "necessary",
                                ff::Role::Discretionary => "discretionary",
                                _ => "other",
                            })
                            .unwrap_or("unclassified")
                            .into(),
                        approval_id: item.map(|x| x.approval_id.clone()),
                        reason: review.reason.clone(),
                    });
                }
                previous = next;
            }
            row.raw.clone()
        } else {
            let all = c
                .cards
                .statements
                .iter()
                .flat_map(|s| s.rows.iter().map(move |r| (s, r)))
                .find(|(s, r)| format!("card:{}:{}", s.source.id, r.row) == row_id)
                .ok_or("Card transaction unavailable for this client.")?;
            let mut previous = None;
            for review in &c.facts {
                let item = review
                    .data
                    .classifications
                    .iter()
                    .find(|x| x.statement_id == all.0.source.id && x.row == all.1.row);
                let next = item.map(|x| (&x.role, &x.approval_id));
                if next != previous {
                    events.push(Event {
                        reviewed_at: review.updated_at.clone(),
                        counselor_name: review.counselor_name.clone(),
                        role: item
                            .map(|x| match x.role {
                                cf::Role::Necessary => "necessary",
                                cf::Role::Discretionary => "discretionary",
                            })
                            .unwrap_or("unclassified")
                            .into(),
                        approval_id: item.map(|x| x.approval_id.clone()),
                        reason: review.data.conclusion.clone(),
                    });
                }
                previous = next;
            }
            all.1.raw.clone()
        };
        tx.commit().map_err(storage_error)?;
        Ok(History {
            client_id: id.into(),
            row_id: row_id.into(),
            original,
            events,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        cards_store::tests as card,
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    use tempfile::tempdir;
    fn query() -> Input {
        Input {
            from_month: "2026-09".into(),
            through_month: "2026-09".into(),
            account_id: None,
            search: String::new(),
            category: None,
            role: None,
            page: 0,
            page_size: 50,
            compare_previous: true,
            opportunity: None,
        }
    }
    fn approve(w: &mut Workspace, id: &str, row: &str, status: imports::Status) {
        let s = w.get_client_imports(id).unwrap();
        let r = s.records.iter().find(|r| r.id == row).unwrap();
        w.review_transaction(
            id,
            imports::ReviewInput {
                expected_revision: s.revision,
                record_id: row.into(),
                status,
                fields: r.fields.clone(),
                reason: "Verified original fictional source".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
    }
    fn paired_transfer(w: &mut Workspace, id: &str, dir: &Path, amount: &str, date: &str) {
        let source = w.get_client_imports(id).unwrap();
        let left = source
            .records
            .iter()
            .find(|r| {
                r.fields.kind == imports::Kind::Transfer
                    && r.linked_id.is_none()
                    && r.fields.amount.as_ref().is_some_and(|n| n.starts_with('-'))
            })
            .unwrap()
            .id
            .clone();
        let incoming=bank::stage(w,id,dir,"reserve-in.csv",&format!("Date,Description,Amount,Category,Type\n{date},Reserve received,{amount},Savings,transfer\n"),bank::doc("Separate reserve account"));
        let right = incoming
            .records
            .iter()
            .find(|r| r.fields.description == "Reserve received")
            .unwrap()
            .id
            .clone();
        w.pair_transfer_transactions(
            id,
            imports::PairInput {
                expected_revision: incoming.revision,
                left_id: left,
                right_id: right,
                reason: "Verified both fictional transfer legs on distinct accounts".into(),
            },
        )
        .unwrap();
    }
    fn classify(w: &mut Workspace, id: &str, needle: &str, role: Role) {
        let view = w.get_spending_analysis(id, query()).unwrap();
        let row = view
            .rows
            .iter()
            .find(|r| r.merchant == needle)
            .unwrap_or_else(|| panic!("missing {needle}"));
        w.change_spending_classification(
            id,
            Change {
                context: view.context,
                query: query(),
                row_id: row.id.clone(),
                approval_id: row.approval_id.clone(),
                role,
                reason: "Discussed fictional household purpose with the client".into(),
                acknowledge: true,
            },
        )
        .unwrap();
    }
    #[test]
    fn exact_bank_and_card_purposes_preserve_sources_history_reports_restart_and_scope() {
        let (root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let other = w.save_client(client_input()).unwrap();
        let mut document = bank::doc("Analysis checking");
        document.retain_source = true;
        let source=bank::stage(&mut w,&client.id,dir.path(),"analysis.csv","Date,Description,Amount,Category,Type\n2026-09-01,Rent,-1000,Housing,expense\n2026-09-02,Dining,-200,Dining,expense\n2026-09-03,Refund,20,Dining,refund\n2026-09-04,Reserve,-150,Savings,transfer\n2026-09-05,Card repayment,-50.03,Card payment,balanceMovement\n2026-09-06,Unprovided category,-30,,expense\n2026-09-07,Pending,-999,Dining,expense\n",document);
        for r in source.records.iter().filter(|r| {
            r.fields.description != "Pending"
                && r.fields.description != "Unprovided category"
                && r.fields.kind != imports::Kind::Transfer
        }) {
            approve(&mut w, &client.id, &r.id, imports::Status::Approved);
        }
        paired_transfer(&mut w, &client.id, dir.path(), "150", "2026-09-04");
        let mut cards = card::stage(&mut w, &client.id, dir.path());
        for (row, kind) in [
            cards::Kind::Purchase,
            cards::Kind::Refund,
            cards::Kind::Payment,
            cards::Kind::Fee,
            cards::Kind::Interest,
        ]
        .into_iter()
        .enumerate()
        {
            cards = w
                .review_card_transaction(
                    &client.id,
                    card::decision(&cards, row, kind, imports::Status::Approved),
                )
                .unwrap();
        }
        classify(&mut w, &client.id, "Rent", Role::Necessary);
        classify(&mut w, &client.id, "Dining", Role::Discretionary);
        classify(&mut w, &client.id, "Fictional groceries", Role::Necessary);
        let view = w.get_spending_analysis(&client.id, query()).unwrap();
        assert_eq!(view.totals.spending, "1307.10");
        assert_eq!(view.totals.necessary, "1100.01");
        assert_eq!(view.totals.discretionary, "200.00");
        assert_eq!(view.totals.unclassified, "7.09");
        assert_eq!(view.totals.refunds, "40.02");
        assert_eq!(view.totals.net_spending, "1267.08");
        assert_eq!(view.totals.debt_payments, "50.03");
        assert_eq!(view.totals.transfers, "150.00");
        assert_eq!(
            view.mix
                .iter()
                .map(|c| u32::from(c.basis_points))
                .sum::<u32>(),
            10000
        );
        assert_eq!(view.previous.unwrap().spending, "0.00");
        assert!(view.spending_change.is_none());
        assert_eq!(view.excluded_uncategorized, 0);
        assert_eq!(view.excluded_unapproved, 2);
        let before = w.get_client_imports(&client.id).unwrap();
        let original =
            serde_json::to_vec(&(&before.documents, &before.records, &before.history)).unwrap();
        let saved = w
            .save_assessment_record(&client.id, bank::assessment_input(&before, &client))
            .unwrap();
        let digest = saved.digest.clone();
        classify(&mut w, &client.id, "Dining", Role::Necessary);
        let now = w.get_spending_analysis(&client.id, query()).unwrap();
        assert_eq!(now.totals.spending, "1307.10");
        assert_eq!(now.totals.necessary, "1300.01");
        assert_eq!(now.totals.discretionary, "0.00");
        assert!(now.discretionary_categories.is_empty());
        let current = w.get_client_imports(&client.id).unwrap();
        assert_eq!(
            serde_json::to_vec(&(&current.documents, &current.records, &current.history)).unwrap(),
            original
        );
        let old = w
            .get_assessment_record(&client.id, &saved.payload.id)
            .unwrap();
        assert_eq!(old.digest, digest);
        assert!(old.needs_review);
        assert_eq!(
            old.payload.assessment.facts_metrics.discretionary_spending,
            "200.00"
        );
        let row = now.rows.iter().find(|r| r.merchant == "Dining").unwrap();
        let h = w
            .get_spending_classification_history(&client.id, &row.id)
            .unwrap();
        assert_eq!(h.events.len(), 2);
        assert_eq!(h.events[0].role, "discretionary");
        assert_eq!(h.events[1].role, "necessary");
        assert_eq!(h.original.reported_amount.as_deref(), Some("-200"));
        assert!(w
            .get_spending_classification_history(&other.id, &row.id)
            .is_err());
        assert_eq!(
            w.get_spending_analysis(&other.id, query())
                .unwrap()
                .totals
                .spending,
            "0.00"
        );
        let live = w
            .get_client_dashboard(
                &client.id,
                dashboard::Input {
                    from_month: "2026-09".into(),
                    through_month: "2026-09".into(),
                },
            )
            .unwrap();
        assert_eq!(live.facts_metrics.necessary_spending, "1200.00");
        let ledger = w.get_card_statements(&client.id).unwrap();
        let original_card = serde_json::to_vec(&ledger).unwrap();
        classify(
            &mut w,
            &client.id,
            "Fictional groceries",
            Role::Discretionary,
        );
        assert_eq!(
            serde_json::to_vec(&w.get_card_statements(&client.id).unwrap()).unwrap(),
            original_card
        );
        assert!(
            !w.get_card_facts(&client.id)
                .unwrap()
                .snapshot
                .unwrap()
                .data
                .inventory_reviewed
        );
        // Repeated discussion changes keep all snapshots without growing the current
        // conclusion with obsolete annotations or exhausting its text bound.
        for _ in 0..3 {
            classify(&mut w, &client.id, "Fictional groceries", Role::Necessary);
            classify(
                &mut w,
                &client.id,
                "Fictional groceries",
                Role::Discretionary,
            );
        }
        let card_history = w.get_card_facts(&client.id).unwrap();
        assert_eq!(card_history.history.len(), 8);
        assert_eq!(
            card_history
                .snapshot
                .unwrap()
                .data
                .conclusion
                .matches("[Spending Analysis discussion]")
                .count(),
            1
        );
        assert_eq!(
            serde_json::to_vec(&w.get_card_statements(&client.id).unwrap()).unwrap(),
            original_card
        );
        w.lock_workspace();
        assert!(w.get_spending_analysis(&client.id, query()).is_err());
        drop(w);
        let mut reopened = Workspace::new(root.path().into());
        reopened.unlock_workspace(PASSWORD.into()).unwrap();
        let after = reopened.get_spending_analysis(&client.id, query()).unwrap();
        assert_eq!(after.totals.discretionary, "100.01");
        assert_eq!(after.totals.necessary, "1200.00");
    }
    #[test]
    fn stale_approval_audit_failure_and_filters_cannot_mutate_or_relabel_transfers() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let source=bank::stage(&mut w,&client.id,dir.path(),"scope.csv","Date,Description,Amount,Category,Type\n2026-09-01,Coffee,-10,Food,expense\n2026-09-02,Reserve,-30,Savings,transfer\n",bank::doc("Scope checking"));
        for row in source
            .records
            .iter()
            .filter(|r| r.fields.kind != imports::Kind::Transfer)
        {
            approve(&mut w, &client.id, &row.id, imports::Status::Approved);
        }
        paired_transfer(&mut w, &client.id, dir.path(), "30", "2026-09-02");
        let v = w.get_spending_analysis(&client.id, query()).unwrap();
        let coffee = v.rows.iter().find(|r| r.merchant == "Coffee").unwrap();
        let change = || Change {
            context: v.context.clone(),
            query: query(),
            row_id: coffee.id.clone(),
            approval_id: coffee.approval_id.clone(),
            role: Role::Discretionary,
            reason: "Discussed fictional coffee purpose".into(),
            acknowledge: true,
        };
        let mut i = change();
        i.acknowledge = false;
        assert!(w.change_spending_classification(&client.id, i).is_err());
        let mut i = change();
        i.reason = " ".into();
        assert!(w.change_spending_classification(&client.id, i).is_err());
        let mut i = change();
        i.row_id = v
            .rows
            .iter()
            .find(|r| r.role == "transfer")
            .unwrap()
            .id
            .clone();
        assert!(w.change_spending_classification(&client.id, i).is_err());
        w.session.as_mut().unwrap().connection.execute_batch("CREATE TRIGGER fail_analysis_audit BEFORE INSERT ON audit_events WHEN NEW.event='financial_facts_reviewed' BEGIN SELECT RAISE(ABORT,'Synthetic failure'); END;").unwrap();
        assert!(w
            .change_spending_classification(&client.id, change())
            .is_err());
        assert!(w
            .get_client_imports(&client.id)
            .unwrap()
            .financial_facts
            .history
            .is_empty());
        w.session
            .as_mut()
            .unwrap()
            .connection
            .execute_batch("DROP TRIGGER fail_analysis_audit;")
            .unwrap();
        w.change_spending_classification(&client.id, change())
            .unwrap();
        assert!(w
            .change_spending_classification(&client.id, change())
            .is_err());
        let mut q = query();
        q.account_id = Some("bank:other-firm-account".into());
        assert!(w.get_spending_analysis(&client.id, q).is_err());
        let mut q = query();
        q.from_month = "2026-11".into();
        assert!(w.get_spending_analysis(&client.id, q).is_err());
        let mut q = query();
        q.search = "Food".into();
        q.role = Some("necessary".into());
        assert_eq!(
            w.get_spending_analysis(&client.id, q)
                .unwrap()
                .matching_rows,
            0
        );
        approve(
            &mut w,
            &client.id,
            &source.records[0].id,
            imports::Status::Rejected,
        );
        assert_eq!(
            w.get_spending_analysis(&client.id, query())
                .unwrap()
                .totals
                .spending,
            "0.00"
        );
        approve(
            &mut w,
            &client.id,
            &source.records[0].id,
            imports::Status::Approved,
        );
        let current = w.get_spending_analysis(&client.id, query()).unwrap();
        assert_eq!(current.totals.unclassified, "10.00");
        assert_eq!(current.totals.discretionary, "0.00");
    }
    #[test]
    fn opportunities_use_reviewed_roles_real_budget_and_exact_periods() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let csv="Date,Description,Amount,Category,Type\n2026-08-01,Coffee,-10,Food,expense\n2026-09-01,Coffee,-8,Food,expense\n2026-09-02,Coffee,-9,Food,expense\n2026-09-03,Coffee,-10,Food,expense\n2026-09-04,Coffee,-11,Food,expense\n2026-09-05,Coffee,-12,Food,expense\n2026-09-06,Exactly fifteen,-15,Food,expense\n";
        let source = bank::stage(
            &mut w,
            &client.id,
            dir.path(),
            "small.csv",
            csv,
            bank::doc("Coffee checking"),
        );
        for r in &source.records {
            approve(&mut w, &client.id, &r.id, imports::Status::Approved);
        }
        let bank = w.get_client_imports(&client.id).unwrap();
        let classifications = bank
            .records
            .iter()
            .map(|r| ff::Classification {
                record_id: r.id.clone(),
                approval_id: bank
                    .history
                    .iter()
                    .rev()
                    .find(|h| h.record_id == r.id)
                    .unwrap()
                    .id
                    .clone(),
                role: ff::Role::Discretionary,
            })
            .collect();
        w.review_financial_facts(
            &client.id,
            ff::Input {
                expected_revision: bank.revision,
                data: ff::Data {
                    classifications,
                    ..Default::default()
                },
                reason: "Verified discretionary coffee discussions".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        w.save_client_budget(
            &client.id,
            crate::budget::BudgetInput {
                expected_revision: 0,
                lines: vec![crate::budget::BudgetLine {
                    label: "Optional purchases".into(),
                    kind: crate::budget::Kind::Discretionary,
                    amount: Some("40".into()),
                    frequency: crate::budget::Frequency::Monthly,
                    estimated: false,
                }],
            },
        )
        .unwrap();
        let v = w.get_spending_analysis(&client.id, query()).unwrap();
        assert_eq!(v.totals.spending, "65.00");
        assert_eq!(v.spending_change.as_deref(), Some("+550.00%"));
        assert_eq!(v.discretionary_budget.as_deref(), Some("40.00"));
        let small = v
            .opportunities
            .iter()
            .find(|o| o.kind == "smallPurchases")
            .unwrap();
        assert_eq!(small.count, 5);
        assert_eq!(small.amount, "50.00");
        assert_eq!(
            v.opportunities
                .iter()
                .find(|o| o.kind == "overspending")
                .unwrap()
                .amount,
            "25.00"
        );
        assert!(!v.opportunities.iter().any(|o| o.kind == "subscriptions"));
        let mut q = query();
        q.opportunity = Some("smallPurchases".into());
        assert_eq!(
            w.get_spending_analysis(&client.id, q)
                .unwrap()
                .matching_rows,
            5
        );
        let mut q = query();
        q.search = "Exactly".into();
        assert_eq!(
            w.get_spending_analysis(&client.id, q)
                .unwrap()
                .matching_rows,
            1
        );
        // Exact row ID, not merchant name: reclassify just one of five small purchases.
        let row = v.rows.iter().find(|r| r.amount == "-8.00").unwrap();
        w.change_spending_classification(
            &client.id,
            Change {
                context: v.context,
                query: query(),
                row_id: row.id.clone(),
                approval_id: row.approval_id.clone(),
                role: Role::Necessary,
                reason: "Client needs this purchase for work".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        let after = w.get_spending_analysis(&client.id, query()).unwrap();
        assert!(!after
            .opportunities
            .iter()
            .any(|o| o.kind == "smallPurchases"));
        assert_eq!(after.totals.discretionary, "57.00");
        assert_eq!(after.totals.necessary, "8.00");
        assert_eq!(
            after
                .opportunities
                .iter()
                .find(|o| o.kind == "overspending")
                .unwrap()
                .amount,
            "17.00"
        );
    }
    #[test]
    fn card_duplicates_and_legacy_uncategorized_rows_are_excluded_without_losing_same_source_purchases(
    ) {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let source = card::stage(&mut w, &client.id, dir.path());
        w.review_card_transaction(
            &client.id,
            card::decision(&source, 0, cards::Kind::Purchase, imports::Status::Approved),
        )
        .unwrap();
        let session = w.session().unwrap();
        let mut c = context(&session.connection, &session.info, &client.id).unwrap();
        let mut repeated = c.cards.statements[0].rows[0].clone();
        repeated.row = 5;
        let mut decision = c.cards.statements[0].history[0].clone();
        decision.row = 5;
        decision.id = "second-legitimate-purchase".into();
        c.cards.statements[0].rows.push(repeated);
        c.cards.statements[0].history.push(decision);
        let (same, _, _) = rows(&c).unwrap();
        assert_eq!(same.len(), 2);
        assert_eq!(totals(&same).unwrap().spending, "200.02");
        let mut overlap = c.cards.statements[0].clone();
        overlap.source.id = "overlapping-source".into();
        // A corrected category never makes an overlapping copy safe to count.
        overlap.rows[0].fields.category = "Corrected Food".into();
        c.cards.statements.push(overlap);
        let (excluded, _, n) = rows(&c).unwrap();
        assert!(excluded.is_empty());
        assert_eq!(n, 4);
        c.cards.statements.truncate(1);
        c.cards.statements[0].rows[0].fields.category = "  ".into();
        let (eligible, _, n) = rows(&c).unwrap();
        assert_eq!(eligible.len(), 1);
        assert_eq!(n, 1);
        // Largest-remainder allocation is deterministic and reconciles to exactly 100%.
        let mut pennies = eligible[0].clone();
        pennies.cents = 1;
        let mut a = pennies.clone();
        a.category = "A".into();
        let mut b = pennies.clone();
        b.category = "B".into();
        let mut d = pennies;
        d.category = "C".into();
        let shares = categories(&[a, b, d], false).unwrap();
        assert_eq!(
            shares.iter().map(|r| r.basis_points).collect::<Vec<_>>(),
            vec![3334, 3333, 3333]
        );
        assert!(categories(&[], false).unwrap().is_empty());
        assert_eq!(percent(1, 6).unwrap(), "16.67%");
    }
    #[test]
    fn subscriptions_need_current_confirmed_evidence_and_follow_purpose_changes() {
        let (_root, mut w, _) = configured();
        let dir = tempdir().unwrap();
        let client = w.save_client(client_input()).unwrap();
        let source=bank::stage(&mut w,&client.id,dir.path(),"subscription.csv","Date,Description,Amount,Category,Type\n2026-07-05,Fictional service,-10,Memberships,expense\n2026-08-05,Fictional service,-10,Memberships,expense\n2026-09-05,Fictional service,-10,Memberships,expense\n",bank::doc("Checking"));
        for r in &source.records {
            approve(&mut w, &client.id, &r.id, imports::Status::Approved);
        }
        let source = w.get_client_imports(&client.id).unwrap();
        let classifications = source
            .records
            .iter()
            .map(|r| ff::Classification {
                record_id: r.id.clone(),
                approval_id: source
                    .history
                    .iter()
                    .rev()
                    .find(|h| h.record_id == r.id)
                    .unwrap()
                    .id
                    .clone(),
                role: ff::Role::Discretionary,
            })
            .collect();
        w.review_financial_facts(
            &client.id,
            ff::Input {
                expected_revision: source.revision,
                data: ff::Data {
                    classifications,
                    ..Default::default()
                },
                reason: "Discussed each fictional service payment".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        let initial = w.get_spending_analysis(&client.id, query()).unwrap();
        assert!(!initial
            .opportunities
            .iter()
            .any(|o| o.kind == "subscriptions"));
        assert_eq!(
            initial
                .opportunities
                .iter()
                .find(|o| o.kind == "recurringCandidates")
                .unwrap()
                .amount,
            "10.00"
        );
        let source = w.get_client_imports(&client.id).unwrap();
        let group = &source.merchant_groups[0];
        assert!(group.can_confirm);
        w.review_recurring_payment(
            &client.id,
            crate::recurring::ReviewInput {
                expected_revision: source.revision,
                group_id: group.id.clone(),
                evidence_hash: group.evidence_hash.clone(),
                label: "Fictional monthly service".into(),
                decision: crate::recurring::Decision::Confirmed,
                frequency: Some(crate::recurring::Frequency::Monthly),
                subscription: true,
                reason: "Client verified the current subscription".into(),
            },
        )
        .unwrap();
        let confirmed = w.get_spending_analysis(&client.id, query()).unwrap();
        assert_eq!(
            confirmed
                .opportunities
                .iter()
                .find(|o| o.kind == "subscriptions")
                .unwrap()
                .amount,
            "10.00"
        );
        classify(&mut w, &client.id, "Fictional service", Role::Necessary);
        let changed = w.get_spending_analysis(&client.id, query()).unwrap();
        assert!(!changed
            .opportunities
            .iter()
            .any(|o| o.kind == "subscriptions"));
        assert_eq!(changed.totals.necessary, "10.00");
        let mut q = query();
        q.from_month = "1900-01".into();
        q.through_month = "1900-01".into();
        assert!(w
            .get_spending_analysis(&client.id, q)
            .unwrap()
            .previous
            .is_none());
    }
}
