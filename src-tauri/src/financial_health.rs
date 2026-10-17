//! Integrated live health projection. Existing ledgers and planners remain authoritative.
use super::*;
use crate::{
    budget::{self, money, parse_money},
    card_review, dashboard, financial_facts as ff, imports, planning, scenarios,
};
use chrono::{Datelike, Months, NaiveDate};
use sha2::{Digest, Sha256};
use spending_analysis as sa;
use std::collections::{BTreeMap, BTreeSet};
const POLICY: &str = "counseling-health-v1";
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub from_month: String,
    pub through_month: String,
}
impl Input {
    fn dashboard(&self) -> dashboard::Input {
        dashboard::Input {
            from_month: self.from_month.clone(),
            through_month: self.through_month.clone(),
        }
    }
    fn spending(&self) -> sa::Input {
        sa::Input {
            from_month: self.from_month.clone(),
            through_month: self.through_month.clone(),
            account_id: None,
            search: String::new(),
            category: None,
            role: None,
            page: 0,
            page_size: 25,
            compare_previous: false,
            opportunity: None,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gate {
    pub key: String,
    pub label: String,
    pub passed: bool,
    pub detail: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub key: String,
    pub label: String,
    pub points: Option<u16>,
    pub maximum: u16,
    pub value: String,
    pub explanation: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub key: String,
    pub label: String,
    pub amount: Option<String>,
    pub monthly: Option<String>,
    pub income_percent: Option<String>,
    pub basis: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Month {
    pub month: String,
    pub income: String,
    pub necessary: String,
    pub discretionary: String,
    pub net_spending: String,
    pub operating_surplus: String,
    pub fees: String,
    pub interest: String,
    pub approved_rows: usize,
    pub complete: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub row_id: String,
    pub approval_id: String,
    pub source_id: String,
    pub file_name: String,
    pub sha256: String,
    pub date: String,
    pub description: String,
    pub amount: String,
    pub category: String,
    pub purpose: String,
    pub account: String,
    pub page: Option<u32>,
    pub line: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub label: String,
    pub net_spending: String,
    pub count: usize,
    pub row_ids: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Insight {
    pub key: String,
    pub title: String,
    pub basis: String,
    pub monthly_impact: Option<String>,
    pub annual_impact: Option<String>,
    pub impact_explanation: String,
    pub row_ids: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanLine {
    pub index: usize,
    pub label: String,
    pub amount: String,
    pub kind: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningReference {
    pub revisions: planning::Revisions,
    pub expense_lines: Vec<PlanLine>,
    pub allocation_lines: Vec<PlanLine>,
    pub cash_lines: Vec<PlanLine>,
    pub debt_payment: Option<String>,
    pub emergency_contribution: Option<String>,
    pub emergency_target: Option<String>,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceBasis {
    pub review_id: String,
    pub reviewed_at: String,
    pub counselor: String,
    pub as_of: Option<String>,
    pub balances: Vec<ff::Balance>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub client_id: String,
    pub revision: i64,
    pub context: String,
    pub policy: String,
    pub period_start: String,
    pub period_end: String,
    pub score: Option<u16>,
    pub balance_basis: Option<BalanceBasis>,
    pub components: Vec<Component>,
    pub gates: Vec<Gate>,
    pub measures: Vec<Measure>,
    pub months: Vec<Month>,
    pub categories: Vec<Category>,
    pub insights: Vec<Insight>,
    pub evidence: Vec<Evidence>,
    pub excluded_rows: usize,
    pub planning: PlanningReference,
    pub warnings: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReductionInput {
    pub query: Input,
    pub context: String,
    pub category: String,
    pub reduction_percent: u16,
    pub expense_line: Option<usize>,
    pub destination: String,
    pub allocation_line: Option<usize>,
    pub cash_line: Option<usize>,
    pub acknowledge: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reduction {
    pub client_id: String,
    pub context: String,
    pub category: String,
    pub basis: String,
    pub monthly_impact: String,
    pub annual_impact: String,
    pub row_ids: Vec<String>,
    pub scenario: Option<scenarios::Comparison>,
    pub warnings: Vec<String>,
}
fn cents(s: &str) -> Result<i64, String> {
    let n = s
        .replace('.', "")
        .parse::<i64>()
        .map_err(|_| STORAGE_ERROR)?;
    if money(n) != s {
        return Err(STORAGE_ERROR.into());
    }
    Ok(n)
}
fn add(a: i64, b: i64) -> Result<i64, String> {
    a.checked_add(b)
        .ok_or_else(|| "Financial amount exceeds the supported range.".into())
}
fn monthly(n: i64, months: i64) -> i64 {
    if n < 0 {
        -((-n + months / 2) / months)
    } else {
        (n + months / 2) / months
    }
}
fn percent(n: i64, d: i64) -> Option<String> {
    ff::ratio(n.abs(), d, 100).map(|s| format!("{}{s}%", if n < 0 { "−" } else { "" }))
}
fn gate(key: &str, label: &str, passed: bool, detail: &str) -> Gate {
    Gate {
        key: key.into(),
        label: label.into(),
        passed,
        detail: detail.into(),
    }
}
fn component(
    key: &str,
    label: &str,
    points: Option<u16>,
    maximum: u16,
    value: String,
    explanation: &str,
) -> Component {
    Component {
        key: key.into(),
        label: label.into(),
        points,
        maximum,
        value,
        explanation: explanation.into(),
    }
}
struct ScoreBasis {
    income: i64,
    net: i64,
    necessary: i64,
    savings: Option<i64>,
    required: Option<i64>,
    months: i64,
}
fn components(transaction_ready: bool, balances_ready: bool, basis: ScoreBasis) -> Vec<Component> {
    let ScoreBasis {
        income,
        net,
        necessary,
        savings,
        required,
        months,
    } = basis;
    let margin = income - net;
    let operating = transaction_ready.then(|| {
        if income <= 0 || margin < 0 {
            0
        } else if i128::from(margin) * 100 < i128::from(income) * 5 {
            10
        } else if i128::from(margin) * 100 < i128::from(income) * 10 {
            20
        } else if i128::from(margin) * 100 < i128::from(income) * 20 {
            30
        } else {
            40
        }
    });
    let debt = (transaction_ready && balances_ready)
        .then_some(required)
        .flatten()
        .map(|p| {
            let period = i128::from(p) * i128::from(months);
            if p == 0 {
                30
            } else if income <= 0 {
                0
            } else if period * 100 <= i128::from(income) * 20 {
                30
            } else if period * 100 <= i128::from(income) * 35 {
                20
            } else if period * 100 <= i128::from(income) * 50 {
                10
            } else {
                0
            }
        });
    let reserve = (transaction_ready && balances_ready && necessary > 0)
        .then_some(savings)
        .flatten()
        .map(|s| {
            let n = i128::from(s) * i128::from(months);
            let d = i128::from(necessary);
            if s == 0 {
                0
            } else if n < d {
                5
            } else if n < d * 3 {
                15
            } else if n < d * 6 {
                25
            } else {
                30
            }
        });
    vec![component("operatingMargin","Operating margin",operating,40,format!("${} surplus before debt principal and transfers; {} of income",money(monthly(margin,months)),percent(margin,income).unwrap_or_else(||"ratio unavailable with zero income".into())),"40 points at a margin ≥20%; 30 at ≥10%; 20 at ≥5%; 10 at ≥0%; 0 for a deficit or verified zero income. Essential transaction evidence and current income/expense roles are required."),component("debtObligations","Required debt payments",debt,30,required.map(|p|format!("${} per month; {} of income",money(p),percent(p*months,income).unwrap_or_else(||"ratio unavailable with zero income".into()))).unwrap_or_else(||"Required monthly payments unknown".into()),"30 points at ≤20% of income (including verified no debt); 20 at ≤35%; 10 at ≤50%; 0 above 50%. Verified zero income with positive payments earns 0; missing payments are unavailable."),component("emergencyReserves","Emergency reserves",reserve,30,savings.and_then(|s|ff::ratio(s*months,necessary,1)).map(|s|format!("{s} months of necessary gross expenses")).unwrap_or_else(||"Coverage unavailable".into()),"30 points at ≥6 months; 25 at ≥3; 15 at ≥1; 5 for positive coverage below one month; 0 for verified zero savings. Requires current dated accessible savings and a positive, fully classified necessary-expense basis. Restricted savings is excluded.")]
}
fn planning_reference(p: &planning::Plans) -> PlanningReference {
    let mut expense_lines = vec![];
    let mut allocation_lines = vec![];
    let mut cash_lines = vec![];
    for (index, line) in p.budget.lines.iter().enumerate() {
        if line.frequency != budget::Frequency::Monthly {
            continue;
        }
        let Some(amount) = &line.amount else { continue };
        let row = PlanLine {
            index,
            label: line.label.clone(),
            amount: amount.clone(),
            kind: format!("{:?}", line.kind),
        };
        if matches!(
            line.kind,
            budget::Kind::Necessary | budget::Kind::Discretionary
        ) {
            expense_lines.push(row)
        } else if matches!(line.kind, budget::Kind::DebtPayment | budget::Kind::Savings) {
            allocation_lines.push(row)
        }
    }
    if let Some(d) = &p.cashflow.data {
        for (index, s) in d.schedules.iter().enumerate() {
            if s.frequency == crate::cashflow::Frequency::Monthly
                && matches!(
                    s.kind,
                    crate::cashflow::Kind::Bill | crate::cashflow::Kind::OtherOutflow
                )
            {
                if let Some(amount) = &s.amount {
                    cash_lines.push(PlanLine {
                        index,
                        label: s.label.clone(),
                        amount: amount.clone(),
                        kind: "Monthly dated expense".into(),
                    })
                }
            }
        }
    }
    PlanningReference{revisions:p.revisions(),expense_lines,allocation_lines,cash_lines,debt_payment:p.debt.data.as_ref().and_then(|d|d.monthly_payment.clone()),emergency_contribution:p.savings.data.as_ref().and_then(|d|d.emergency_contribution.clone()),emergency_target:p.savings.summary.as_ref().and_then(|s|s.emergency_target.clone()),warnings:vec!["Saved plans are separate counselor-entered assumptions; historical transactions never overwrite them. Choose explicit line mappings before generating a temporary scenario.".into()]}
}
struct Computed {
    assessment: Assessment,
    plans: planning::Plans,
}
fn assess(
    c: &Connection,
    w: &WorkspaceInfo,
    id: &str,
    q: Input,
    today: NaiveDate,
) -> Result<Computed, String> {
    let profile = scoped_client(c, w, id)?;
    let (start, end, count) = dashboard::period(&q.dashboard())?;
    let months = i64::from(count);
    let start_text = start.to_string();
    let end_text = end.to_string();
    let bank = imports_store::read_imports(c, w, id)?;
    let ledger = cards_store::read(c, w, id)?;
    let plans = assessments_store::read_plans(c, w, id)?;
    let coverage = coverage_review_store::versions(c, w, id)?.pop();
    let card = card_facts_store::read(c, w, id)?;
    let links = card_review_store::history(c, w, id)?.pop();
    let (current, sources) = unified_review::health_evidence(c, w, id)?;
    let accepted = current
        .iter()
        .filter(|r| {
            r.eligible
                && sources
                    .iter()
                    .any(|s| s.id == r.source_id && s.reconciliation == "matched")
        })
        .map(|r| r.id.clone())
        .collect::<BTreeSet<_>>();
    let (activity, all) = sa::health_activity(c, w, id, &q.spending(), &accepted)?;
    let period_rows = all
        .iter()
        .filter(|r| r.date >= start_text && r.date <= end_text)
        .cloned()
        .collect::<Vec<_>>();
    let inside =
        |d: Option<&str>| d.is_some_and(|d| d >= start_text.as_str() && d <= end_text.as_str());
    let selected = current
        .iter()
        .filter(|r| accepted.contains(&r.id) && inside(r.date.as_deref()))
        .collect::<Vec<_>>();
    let bank_rows = bank
        .records
        .iter()
        .filter(|r| {
            accepted.contains(&format!("bank:{}", r.id)) && inside(r.fields.date.as_deref())
        })
        .collect::<Vec<_>>();
    let imported = imports::summarize_selected(&bank_rows, &bank.accounts)?;
    let income = cents(&imported.income)?;
    let net = cents(&activity.totals.net_spending)?;
    let necessary = cents(&activity.totals.necessary)?;
    let base = dashboard::assess(&bank, q.dashboard(), today)?;
    let diagnostic = crate::completeness::review(&bank, q.dashboard(), today)?;
    let coverage_current = coverage.as_ref().is_some_and(|v| {
        v.source_revision == bank.revision
            && v.profile_updated_at == profile.updated_at
            && v.data.from_month == q.from_month
            && v.data.through_month == q.through_month
            && v.choices == crate::coverage_review::choices(&bank)
    });
    let inventory = coverage.as_ref().is_some_and(|v| {
        coverage_current
            && v.data.inventory_reviewed
            && !v.data.inventory_notes.is_empty()
            && v.data.missing_information.is_empty()
            && !v.data.accounts.is_empty()
    });
    let covered = inventory
        && !diagnostic.accounts.is_empty()
        && diagnostic.assignment_issues == 0
        && diagnostic.accounts.iter().all(|a| {
            a.gaps.is_empty()
                && a.overlaps.is_empty()
                && coverage.as_ref().unwrap().data.accounts.iter().any(|e| {
                    e.account_id.as_deref() == Some(&a.id)
                        && e.expected_start.as_deref() == Some(&start_text)
                        && e.expected_end.as_deref() == Some(&end_text)
                        && e.exception.is_empty()
                        && a.document_ids.iter().all(|d| e.source_ids.contains(d))
                })
        });
    let excluded_rows = current
        .iter()
        .filter(|r| {
            r.status != imports::Status::Rejected
                && (inside(r.date.as_deref()) || r.date.is_none())
                && !accepted.contains(&r.id)
        })
        .count();
    let sources_ready = excluded_rows == 0
        && selected.iter().any(|r| r.card_row.is_none())
        && diagnostic.pending + diagnostic.clarification + diagnostic.undated == 0
        && diagnostic.sources.iter().all(|s| {
            s.outside_declared_period == 0
                && sources
                    .iter()
                    .any(|source| source.id == s.id && source.reconciliation == "matched")
        });
    let roles_ready = selected.iter().all(|r| r.purpose != "unclassified")
        && bank_rows.iter().all(|r| {
            r.fields.kind != imports::Kind::Income
                || bank.financial_facts.data.as_ref().is_some_and(|d| {
                    d.classifications
                        .iter()
                        .any(|f| f.record_id == r.id && ff::active(f, r, &bank.history))
                })
        });
    let needs_card = !ledger.statements.is_empty()
        || links.as_ref().is_some_and(|l| !l.accounts.is_empty())
        || coverage
            .as_ref()
            .is_some_and(|v| v.data.accounts.iter().any(|a| a.kind == "creditCard"));
    let card_ready = if !needs_card {
        inventory
    } else {
        !card.needs_review
            && card.snapshot.as_ref().is_some_and(|s| {
                s.data.from_month == q.from_month
                    && s.data.through_month == q.through_month
                    && s.evaluation.evidence_complete
                    && s.evaluation.issues.is_empty()
            })
            && links.as_ref().is_some_and(|l| {
                card_review::calculate(&bank, &ledger, l, q.dashboard())
                    .is_ok_and(|r| r.issues.is_empty() && r.observed_spending.is_some())
            })
    };
    let m = &base.facts_metrics;
    let balances_ready = m.balance_complete && m.balance_aligned && !m.balance_outdated;
    let required = bank
        .financial_facts
        .data
        .as_ref()
        .filter(|_| balances_ready)
        .and_then(|d| {
            let rows = d
                .balances
                .iter()
                .filter(|b| b.kind == ff::BalanceKind::Debt)
                .collect::<Vec<_>>();
            if rows.iter().any(|b| b.monthly_payment.is_none()) {
                None
            } else {
                rows.iter().try_fold(0i64, |sum, b| {
                    add(
                        sum,
                        parse_money(b.monthly_payment.as_deref().unwrap()).ok()?,
                    )
                    .ok()
                })
            }
        });
    let savings = m.accessible_savings.as_deref().map(cents).transpose()?;
    let transfers_ready = bank_rows
        .iter()
        .filter(|r| r.fields.kind == imports::Kind::Transfer)
        .all(|r| {
            r.linked_id.as_ref().is_some_and(|linked| {
                bank_rows
                    .iter()
                    .any(|other| &other.id == linked && other.linked_id.as_deref() == Some(&r.id))
            })
        });
    let gates=vec![gate("period","Completed, recent period",end<today&&(today-end).num_days()<=90,"Choose complete calendar months ending before today and within the last 90 days. Historical observations remain visible without a current score."),
        gate("inventory","Current account inventory review",inventory,"Save a current Completeness & final validation inventory for these exact months and latest source decisions, with no missing information."),
        gate("coverage","Complete bank statement coverage",covered,"All recorded checking/savings accounts need exactly one selected statement for every day, full-period applicability and no overlaps or unexplained exceptions."),
        gate("sources","Approved reconciled financial evidence",sources_ready,"Resolve undecided, undated, uncategorized, duplicated or unreconciled evidence. At least one approved bank row is required; an empty import is not verified zero income."),
        gate("roles","Current income and expense classifications",roles_ready,"Review income roles in Verified figures and each expense purpose in Spending Analysis. Categories and merchants cannot determine necessity."),
        gate("cards","Card scope and repayment reconciliation",card_ready,"Use current Card completeness and Bank & card activity checks when cards are recorded. Missing cards are not inferred from an empty upload; a current full account inventory is essential."),
        gate("balances","Verified dated balance inventory",balances_ready,"Record the complete financial inventory in Verified figures, dated at period end and no older than 90 days. Missing balances are unknown, not zero."),
        gate("payments","Verified required debt payments",required.is_some(),"Every verified debt balance needs a known required monthly payment. A reviewed complete inventory can establish no debt."),
        gate("transfers","Current reciprocal transfer evidence",transfers_ready,"Owned transfer legs must both have current reciprocal approvals in this period. Cross-period or unresolved legs leave allocation figures unavailable rather than inferred."),
        gate("necessary","Positive necessary-expense basis",necessary>0,"Emergency coverage needs positive approved necessary expenses; zero or unclassified necessary costs cannot imply perfect reserves.")];
    let transaction_ready = gates
        .iter()
        .filter(|g| {
            matches!(
                g.key.as_str(),
                "period" | "inventory" | "coverage" | "sources" | "roles" | "cards" | "transfers"
            )
        })
        .all(|g| g.passed);
    let components = components(
        transaction_ready,
        balances_ready,
        ScoreBasis {
            income,
            net,
            necessary,
            savings,
            required,
            months,
        },
    );
    let score = if gates.iter().all(|g| g.passed) {
        components
            .iter()
            .try_fold(0u16, |sum, c| c.points.map(|n| sum + n))
    } else {
        None
    };
    let mut savings_in = 0;
    let mut savings_out = 0;
    let mut other_transfers = 0;
    let mut loan_payments = 0;
    let mut other_movements = 0;
    for r in &bank_rows {
        let n = imports::signed_money(r.fields.amount.as_deref().ok_or(STORAGE_ERROR)?)?;
        if r.fields.kind == imports::Kind::Transfer && n < 0 {
            let pair = r.linked_id.as_ref().and_then(|id| {
                bank_rows
                    .iter()
                    .find(|p| &p.id == id && p.linked_id.as_deref() == Some(&r.id))
            });
            let from = bank
                .accounts
                .iter()
                .find(|a| a.id == r.account_id)
                .map(|a| a.kind.as_str());
            let to = pair
                .and_then(|p| bank.accounts.iter().find(|a| a.id == p.account_id))
                .map(|a| a.kind.as_str());
            match (from, to) {
                (Some("checking"), Some("savings")) => savings_in = add(savings_in, -n)?,
                (Some("savings"), Some("checking")) => savings_out = add(savings_out, -n)?,
                _ => other_transfers = add(other_transfers, -n)?,
            }
        } else if r.fields.kind == imports::Kind::BalanceMovement && n < 0 {
            let method = selected
                .iter()
                .find(|p| p.record_id == r.id)
                .map(|p| p.method.as_str());
            if method == Some("loanPayment") {
                loan_payments = add(loan_payments, -n)?
            } else if method != Some("cardPayment") {
                let linked = links.as_ref().is_some_and(|l| {
                    l.repayments.iter().any(|p| {
                        p.bank_record_id == r.id && card_review::current(p, &bank, &ledger)
                    })
                });
                if !linked {
                    other_movements = add(other_movements, -n)?
                }
            }
        }
    }
    let observed_debt = add(cents(&activity.totals.debt_payments)?, loan_payments)?;
    let make = |key: &str, label: &str, n: Option<i64>, basis: &str| Measure {
        key: key.into(),
        label: label.into(),
        amount: n.map(money),
        monthly: n.map(|n| money(monthly(n, months))),
        income_percent: n.and_then(|n| percent(n, income)),
        basis: basis.into(),
    };
    let measures=vec![make("income","Approved income receipts",Some(income),"Approved bank income and earned interest only; borrowing, refunds and transfers excluded. Includes irregular receipts actually recorded."),
        make("necessary","Necessary gross expenses",Some(necessary),"Current counselor-reviewed purposes; gross expenses include their approved fees/interest. Refunds are separate."),
        make("discretionary","Discretionary gross expenses",Some(cents(&activity.totals.discretionary)?),"Current explicit discretionary purposes; no merchant-based judgment."),
        make("unclassified","Unclassified expense purpose",Some(cents(&activity.totals.unclassified)?),"Approved expenses still needing a current necessary/discretionary decision."),
        make("refunds","Refunds and reimbursements",Some(cents(&activity.totals.refunds)?),"Credits shown separately and subtracted exactly once to reach net spending; not income."),
        make("netSpending","Net spending after refunds",Some(net),"Gross expenses less refunds; card repayments and transfers are excluded."),
        make("operatingSurplus","Operating surplus / deficit",Some(income-net),"Income less net spending; before debt principal and savings/transfers. This is not the dated bank cash-flow forecast."),
        make("debtObserved","Observed card / identified loan repayments",Some(observed_debt),"Card liability payments once, plus explicitly reviewed loan-payment bank movements. Bank card counterparts are not added again. Other principal is not guessed."),
        make("debtRequired","Required debt obligations",required.map(|n|n*months),"Verified monthly required payments × selected months, not an additional historical spending category. Do not add to observed repayments."),
        make("savingsIn","Owned checking → savings transfers",transfers_ready.then_some(savings_in),"Both current reciprocal approved legs must be present in the period. Allocation, not a purchase or expense."),
        make("savingsOut","Owned savings → checking transfers",transfers_ready.then_some(savings_out),"Withdrawals from savings shown separately; not earned income."),
        make("otherTransfers","Other owned transfers (outgoing once)",transfers_ready.then_some(other_transfers),"Excludes the checking/savings allocations above; reciprocal incoming legs are not counted twice."),
        make("otherMovements","Other outgoing bank movements",Some(other_movements),"Known approved balance movements without inferred payment purpose; not included in purchases or income.")];
    let mut month_rows = vec![];
    let mut cost_rows = vec![];
    for offset in 0..count {
        let d = start
            .checked_add_months(Months::new(offset as u32))
            .ok_or(STORAGE_ERROR)?;
        let label = format!("{:04}-{:02}", d.year(), d.month());
        let rows = period_rows
            .iter()
            .filter(|r| r.date.starts_with(&label))
            .cloned()
            .collect::<Vec<_>>();
        let totals = sa::totals(&rows)?;
        let br = bank_rows
            .iter()
            .filter(|r| {
                r.fields
                    .date
                    .as_ref()
                    .is_some_and(|s| s.starts_with(&label))
            })
            .copied()
            .collect::<Vec<_>>();
        let inc = cents(&imports::summarize_selected(&br, &bank.accounts)?.income)?;
        let mut fees = 0;
        let mut interest = 0;
        for r in selected.iter().filter(|r| {
            r.date.as_ref().is_some_and(|s| s.starts_with(&label)) && r.direction == "debit"
        }) {
            let amount = cents(r.amount.as_deref().ok_or(STORAGE_ERROR)?)?.abs();
            if r.kind == "fee" {
                fees = add(fees, amount)?;
                cost_rows.push(r.id.clone())
            } else if r.kind == "interest" {
                interest = add(interest, amount)?;
                cost_rows.push(r.id.clone())
            }
        }
        month_rows.push(Month {
            month: label,
            income: money(inc),
            necessary: totals.necessary,
            discretionary: totals.discretionary,
            net_spending: totals.net_spending.clone(),
            operating_surplus: money(inc - cents(&totals.net_spending)?),
            fees: money(fees),
            interest: money(interest),
            approved_rows: rows.len(),
            complete: transaction_ready,
        })
    }
    let mut category_map = BTreeMap::<String, (i64, usize, Vec<String>)>::new();
    for r in &period_rows {
        let n = match r.role.as_str() {
            "necessary" | "discretionary" | "unclassified" => r.cents,
            "reimbursement" => -r.cents,
            _ => continue,
        };
        let e = category_map.entry(r.category.clone()).or_default();
        e.0 = add(e.0, n)?;
        e.1 += usize::from(r.editable);
        e.2.push(r.id.clone())
    }
    let categories = category_map
        .into_iter()
        .filter(|(_, v)| v.0 > 0)
        .map(|(label, (n, count, row_ids))| Category {
            label,
            net_spending: money(n),
            count,
            row_ids,
        })
        .collect::<Vec<_>>();
    let evidence = selected
        .iter()
        .map(|r| {
            let source = sources.iter().find(|s| s.id == r.source_id).unwrap();
            Evidence {
                row_id: r.id.clone(),
                approval_id: r.binding.clone(),
                source_id: r.source_id.clone(),
                file_name: source.file_name.clone(),
                sha256: source.sha256.clone(),
                date: r.date.clone().unwrap(),
                description: r.description.clone(),
                amount: r.amount.clone().unwrap(),
                category: r.category.clone(),
                purpose: r.purpose.clone(),
                account: source.account_label.clone(),
                page: r.page,
                line: r.line,
            }
        })
        .collect::<Vec<_>>();
    let impact = |key: &str,
                  title: &str,
                  basis: String,
                  n: Option<i64>,
                  explanation: &str,
                  row_ids: Vec<String>|
     -> Result<Insight, String> {
        let monthly_impact = n.map(|n| monthly(n, months));
        Ok(Insight {
            key: key.into(),
            title: title.into(),
            basis,
            monthly_impact: monthly_impact.map(money),
            annual_impact: monthly_impact
                .map(|n| n.checked_mul(12).map(money).ok_or(STORAGE_ERROR))
                .transpose()?,
            impact_explanation: explanation.into(),
            row_ids,
        })
    };
    let mut insights = vec![];
    if score.is_none() {
        insights.push(impact("evidence","Complete the evidence before relying on a score",gates.iter().filter(|g|!g.passed).map(|g|g.label.clone()).collect::<Vec<_>>().join("; "),None,"Financial impact is unavailable until essential evidence is complete. Available figures describe observed records only, not complete finances.",vec![])?)
    }
    if income < net {
        insights.push(impact("deficit","Review the operating deficit",format!("Approved income {} is below net expenses {} over {count} month(s).",money(income),money(net)),Some(net-income),"Closing this observed gap would require this much additional income or lower expenses each month, before debt principal and transfers. Annual impact assumes the same monthly gap continues; it is not a forecast.",evidence.iter().filter(|r|matches!(r.purpose.as_str(),"necessary"|"discretionary"|"unclassified"|"income"|"reimbursement")).map(|r|r.row_id.clone()).collect())?)
    }
    if transaction_ready && balances_ready {
        if let (Some(target), Some(available)) = (
            crate::savings::reserve_target(Some(monthly(necessary, months)), 6),
            savings,
        ) {
            if target > available {
                insights.push(impact("reserveGap","Discuss the emergency-reserve gap",format!("Six months of observed necessary expenses correspond to ${}; verified accessible savings are ${}, leaving ${} to discuss. This uses the existing savings target arithmetic.",money(target),money(available),money(target-available)),None,"No monthly contribution or future income is assumed. The six-month benchmark is an illustration, not a mandatory target. Review earmarked funds and use the existing savings/scenario engines to compare an affordable contribution and funding dates.",evidence.iter().filter(|e|e.purpose=="necessary").map(|e|e.row_id.clone()).collect())?)
            }
        }
    }
    let costs = month_rows.iter().try_fold(0, |sum, m| {
        add(sum, add(cents(&m.fees)?, cents(&m.interest)?)?)
    })?;
    if costs > 0 {
        insights.push(impact("costs","Discuss interest and institution charges",format!("{} in approved interest and fees over {count} month(s); these costs are already included in expenses.",money(costs)),Some(costs),"If equivalent future charges could be avoided entirely, the displayed monthly/annual amount is the upper-bound cash cost avoided. Eligibility and contract terms need review; no avoidance is assumed.",cost_rows)?)
    }
    for o in activity.opportunities {
        insights.push(impact(&o.kind,&o.title,format!("{} {} · {}",o.count,o.amount,o.detail),Some(cents(&o.amount)?),"This is observed activity to discuss. The monthly amount averages the selected period; annual impact assumes eliminating equivalent net future costs for twelve months. Opportunities can overlap and are not additive or guaranteed savings. Use the refund-adjusted reduction preview before planning.",o.row_ids)?)
    }
    if month_rows.len() > 1 {
        let incomes = month_rows
            .iter()
            .map(|m| cents(&m.income))
            .collect::<Result<Vec<_>, _>>()?;
        let low = *incomes.iter().min().unwrap();
        let high = *incomes.iter().max().unwrap();
        if low != high {
            insights.push(impact("irregularIncome","Plan around the observed income variation",format!("Monthly approved receipts range from {} to {}; the selected-period average is {}. Every selected calendar month is included.",money(low),money(high),money(monthly(income,months))),None,"The observed monthly range is not a savings opportunity or guaranteed future income. Review payment dates and a conservative scenario in the existing cash-flow calendar.",evidence.iter().filter(|r|r.purpose=="income").map(|r|r.row_id.clone()).collect())?)
        }
    }
    let token_data = serde_json::to_vec(&(
        POLICY,
        &profile.id,
        &profile.updated_at,
        &q,
        bank.revision,
        coverage.as_ref().map(|s| s.revision),
        card.latest_revision,
        plans.revisions(),
        today.to_string(),
    ))
    .map_err(|_| STORAGE_ERROR.to_owned())?;
    let context = format!("{:x}", Sha256::digest(token_data));
    Ok(Computed{assessment:Assessment{client_id:id.into(),revision:bank.revision,context,policy:POLICY.into(),period_start:start_text,period_end:end_text,score,balance_basis:bank.financial_facts.history.last().map(|r|BalanceBasis{review_id:r.id.clone(),reviewed_at:r.reviewed_at.clone(),counselor:r.counselor_name.clone(),as_of:r.data.as_of.clone(),balances:r.data.balances.clone()}),components,gates,measures,months:month_rows,categories,insights,evidence,excluded_rows,planning:planning_reference(&plans),warnings:vec!["This live 0–100 counseling index is an explainable product policy, not a credit score, validated prediction or final report. Missing information is never scored as poor performance.".into(),"Income-relative rows have different bases and must not be added together. Gross purpose amounts include their fees/interest; refunds are separate; net expenses subtract refunds once. Repayments, savings allocations and transfers are not additional purchases.".into(),"Only approved, categorized, nonduplicate rows from matched sources supply observations. Empty or incomplete months are not proof of no activity. Statement arithmetic and recorded inventories cannot establish undisclosed real-world accounts.".into(),"Recommendations are discussion prompts grounded in recorded evidence. Their conditional annual amounts are monthly assumptions × 12, can overlap, and are not guaranteed savings. Saved plans and issued reports keep their original assumptions and history.".into()]},plans})
}
fn reduce(current: Computed, input: ReductionInput) -> Result<Reduction, String> {
    let a = current.assessment;
    let plans = current.plans;
    if input.context != a.context {
        return Err("Source decisions, balances, coverage or saved plans changed. Refresh the assessment and review the new evidence before previewing a reduction.".into());
    }
    if input.destination == "unallocated" && input.allocation_line.is_some() {
        return Err("An unallocated preview cannot also increase an allocation line.".into());
    }
    if !input.acknowledge
        || input.reduction_percent > 100
        || input.reduction_percent == 0
        || !matches!(
            input.destination.as_str(),
            "unallocated" | "debt" | "emergency"
        )
    {
        return Err("Choose a reduction from 1–100%, a supported destination and confirm the discussion assumptions.".into());
    }
    let category = a
        .categories
        .iter()
        .find(|r| r.label == input.category)
        .ok_or("Choose a current category with positive net spending.")?;
    let (_, _, months) = dashboard::period(&input.query.dashboard())?;
    let n = i128::from(cents(&category.net_spending)?) * i128::from(input.reduction_percent);
    let denom = i128::from(months) * 100;
    let amount = i64::try_from((n + denom / 2) / denom).map_err(|_| STORAGE_ERROR.to_owned())?;
    let annual = amount.checked_mul(12).ok_or(STORAGE_ERROR)?;
    let mut warnings=vec!["Temporary counselor-selected assumption; no transaction, classification, saved budget or issued report is changed. Net category spending includes refunds before the reduction. Annual amount assumes twelve identical future months.".into()];
    if a.score.is_none() {
        warnings.push("The financial evidence is incomplete. This preview uses observed eligible records only and must not be interpreted as an overall affordable allocation.".into())
    }
    let scenario = if let Some(index) = input.expense_line {
        let line = plans
            .budget
            .lines
            .get(index)
            .ok_or("Choose an existing saved monthly expense line.")?;
        if line.frequency != budget::Frequency::Monthly
            || !matches!(
                line.kind,
                budget::Kind::Necessary | budget::Kind::Discretionary
            )
        {
            return Err(
                "Map the reduction explicitly to a monthly necessary or discretionary expense."
                    .into(),
            );
        }
        let original = line
            .amount
            .as_deref()
            .map(parse_money)
            .transpose()?
            .ok_or("The mapped expense amount is unknown.")?;
        if amount > original {
            return Err("The reduction exceeds the mapped saved expense. Choose a lower percentage or a different explicit mapping.".into());
        }
        let mut changes = vec![scenarios::BudgetChange {
            index,
            amount: Some(money(original - amount)),
        }];
        let mut debt_payment = None;
        let mut emergency_contribution = None;
        if input.destination != "unallocated" {
            let ai = input
                .allocation_line
                .ok_or("Choose the saved monthly allocation line to increase.")?;
            let allocation = plans
                .budget
                .lines
                .get(ai)
                .ok_or("The allocation line is unavailable.")?;
            let expected = if input.destination == "debt" {
                budget::Kind::DebtPayment
            } else {
                budget::Kind::Savings
            };
            if allocation.kind != expected || allocation.frequency != budget::Frequency::Monthly {
                return Err("The allocation line must match the selected debt or savings destination and use a monthly frequency.".into());
            }
            let original = allocation
                .amount
                .as_deref()
                .map(parse_money)
                .transpose()?
                .ok_or("The allocation amount is unknown.")?;
            changes.push(scenarios::BudgetChange {
                index: ai,
                amount: Some(money(add(original, amount)?)),
            });
            if input.destination == "debt" {
                let payment = plans
                    .debt
                    .data
                    .as_ref()
                    .and_then(|d| d.monthly_payment.as_deref())
                    .ok_or("Save a known debt-payment baseline first.")?;
                debt_payment = Some(scenarios::AmountChange {
                    amount: Some(money(add(parse_money(payment)?, amount)?)),
                })
            } else {
                let contribution = plans
                    .savings
                    .data
                    .as_ref()
                    .and_then(|d| d.emergency_contribution.as_deref())
                    .ok_or("Save a known emergency-contribution baseline first.")?;
                emergency_contribution = Some(scenarios::AmountChange {
                    amount: Some(money(add(parse_money(contribution)?, amount)?)),
                })
            }
        }
        let mut cash_changes = vec![];
        if let Some(index) = input.cash_line {
            let schedule = plans
                .cashflow
                .data
                .as_ref()
                .and_then(|d| d.schedules.get(index))
                .ok_or("Choose an existing dated monthly expense.")?;
            if schedule.frequency != crate::cashflow::Frequency::Monthly
                || !matches!(
                    schedule.kind,
                    crate::cashflow::Kind::Bill | crate::cashflow::Kind::OtherOutflow
                )
            {
                return Err("The calendar mapping must be a monthly bill or outflow.".into());
            }
            let original = schedule
                .amount
                .as_deref()
                .map(parse_money)
                .transpose()?
                .ok_or("The dated expense amount is unknown.")?;
            if amount > original {
                return Err("The reduction exceeds the mapped dated expense.".into());
            }
            cash_changes.push(scenarios::CashChange {
                index,
                amount: Some(money(original - amount)),
                first_date: schedule.first_date.clone(),
            })
        } else {
            warnings.push("Dated cash-flow schedules are unchanged because no dated monthly expense was mapped. Do not treat an unchanged calendar as confirmation of the budget reduction.".into())
        }
        if input.destination != "unallocated" {
            warnings.push("The proposed budget and debt/savings allocation change is delegated to the existing engines. Dated allocation schedules remain unchanged; review their timing in the cash-flow calendar before acting.".into())
        }
        let strategy = if plans
            .debt
            .comparison
            .as_ref()
            .is_some_and(|c| c.avalanche.is_some())
        {
            planning::Strategy::Avalanche
        } else {
            planning::Strategy::None
        };
        let proposal=scenarios::Input{expected_revisions:plans.revisions(),label:format!("Discussed reduction: {}",category.label),assumptions:format!("Health source context {}; {}% of observed net category {} over {} months. Monthly {} / annual {} are conditional assumptions. Explicit budget line {}. Originals preserved.",a.context,input.reduction_percent,category.net_spending,months,money(amount),money(annual),line.label),baseline_strategy:strategy.clone(),proposed_strategy:strategy,budget_changes:changes,debt_payment,emergency_contribution,cash_changes,debt_extras:vec![],goal_contributions:vec![],recurring_expenses:vec![],income_reductions:vec![]};
        Some(scenarios::compare(plans, proposal)?)
    } else {
        if input.destination != "unallocated"
            || input.allocation_line.is_some()
            || input.cash_line.is_some()
        {
            return Err(
                "Map a saved monthly expense before projecting a debt, savings or calendar change."
                    .into(),
            );
        }
        warnings.push("No saved expense is mapped. Only the observed monthly/annual impact is shown; no debt, savings or calendar projection is inferred.".into());
        None
    };
    Ok(Reduction {
        client_id: a.client_id,
        context: a.context,
        category: category.label.clone(),
        basis: format!(
            "{}% of {} net category spending over {} calendar months; refunds deducted once.",
            input.reduction_percent, category.net_spending, months
        ),
        monthly_impact: money(amount),
        annual_impact: money(annual),
        row_ids: category.row_ids.clone(),
        scenario,
        warnings,
    })
}
impl Workspace {
    pub fn get_financial_health(&mut self, id: &str, input: Input) -> Result<Assessment, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let result = assess(
            &tx,
            &s.info,
            id,
            input,
            chrono::Utc::now()
                .with_timezone(&chrono_tz::America::Chicago)
                .date_naive(),
        )?
        .assessment;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
    pub fn preview_financial_health_reduction(
        &mut self,
        id: &str,
        input: ReductionInput,
    ) -> Result<Reduction, String> {
        let s = self.session()?;
        let tx = s.connection.transaction().map_err(storage_error)?;
        let current = assess(
            &tx,
            &s.info,
            id,
            input.query.clone(),
            chrono::Utc::now()
                .with_timezone(&chrono_tz::America::Chicago)
                .date_naive(),
        )?;
        let result = reduce(current, input)?;
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        imports_store::tests as bank,
        tests::{client_input, configured, PASSWORD},
    };
    use crate::{
        coverage_review,
        financial_facts::{Balance, BalanceKind, Classification, Data, Role},
        imports::{Kind, Status},
    };
    fn query() -> Input {
        Input {
            from_month: "2026-09".into(),
            through_month: "2026-09".into(),
        }
    }
    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 17).unwrap()
    }
    fn view(w: &mut Workspace, id: &str, q: Input) -> Assessment {
        let s = w.session().unwrap();
        assess(&s.connection, &s.info, id, q, today())
            .unwrap()
            .assessment
    }
    fn decide(w: &mut Workspace, id: &str, record: &str, status: Status) {
        let s = w.get_client_imports(id).unwrap();
        let r = s.records.iter().find(|r| r.id == record).unwrap();
        w.review_transaction(
            id,
            imports::ReviewInput {
                expected_revision: s.revision,
                record_id: record.into(),
                status,
                fields: r.fields.clone(),
                reason: "Verified synthetic evidence and financial classification".into(),
                acknowledge_warnings: true,
            },
        )
        .unwrap();
    }
    fn facts(w: &mut Workspace, id: &str) {
        let s = w.get_client_imports(id).unwrap();
        let classifications = s
            .records
            .iter()
            .filter(|r| r.status == Status::Approved)
            .filter_map(|r| {
                let role = match r.fields.kind {
                    Kind::Income => Role::TakeHomePay,
                    Kind::Expense if r.fields.description.contains("Dining") => Role::Discretionary,
                    Kind::Expense | Kind::Fee | Kind::Interest => Role::Necessary,
                    _ => return None,
                };
                let approval = s
                    .history
                    .iter()
                    .rev()
                    .find(|h| h.record_id == r.id)
                    .unwrap();
                Some(Classification {
                    record_id: r.id.clone(),
                    approval_id: approval.id.clone(),
                    role,
                })
            })
            .collect();
        w.review_financial_facts(
            id,
            ff::Input {
                expected_revision: s.revision,
                data: Data {
                    as_of: Some("2026-09-30".into()),
                    full_inventory: true,
                    balances: vec![
                        Balance {
                            label: "Checking".into(),
                            kind: BalanceKind::Cash,
                            amount: Some("2000".into()),
                            monthly_payment: None,
                            evidence:
                                "Verified fictional ending cash; complete inventory discussed"
                                    .into(),
                            document_id: None,
                        },
                        Balance {
                            label: "Available savings".into(),
                            kind: BalanceKind::AccessibleSavings,
                            amount: Some("6000".into()),
                            monthly_payment: None,
                            evidence: "Verified available funds; no restricted reserves included"
                                .into(),
                            document_id: None,
                        },
                        Balance {
                            label: "Fictional loan".into(),
                            kind: BalanceKind::Debt,
                            amount: Some("1000".into()),
                            monthly_payment: Some("200".into()),
                            evidence:
                                "Verified synthetic principal and required monthly obligation"
                                    .into(),
                            document_id: None,
                        },
                    ],
                    classifications,
                },
                reason: "Reviewed the complete synthetic financial inventory and current roles"
                    .into(),
                acknowledge: true,
            },
        )
        .unwrap();
    }
    fn coverage(w: &mut Workspace, id: &str, q: &Input) {
        let r = w.get_coverage_review(id, None).unwrap();
        let accounts = r
            .choices
            .accounts
            .iter()
            .map(|a| coverage_review::Expectation {
                account_id: Some(a.id.clone()),
                label: a.label.clone(),
                kind: a.kind.clone(),
                expected_start: Some(format!("{}-01", q.from_month)),
                expected_end: Some("2026-09-30".into()),
                exception: String::new(),
                source_ids: r
                    .choices
                    .sources
                    .iter()
                    .filter(|s| s.account_id == a.id)
                    .map(|s| s.id.clone())
                    .collect(),
            })
            .collect();
        w.save_coverage_review(id,coverage_review::Input{expected_revision:r.latest_revision,expected_source_revision:r.current_source_revision,expected_profile_updated_at:r.current_profile_updated_at,data:coverage_review::Data{from_month:q.from_month.clone(),through_month:q.through_month.clone(),inventory_reviewed:true,inventory_notes:"All client checking/savings, income, obligations and absence of credit cards reviewed; synthetic client".into(),accounts,missing_information:String::new(),conclusion:"Complete fictional period and financial inventory reviewed".into()}}).unwrap();
    }
    fn fixture() -> (tempfile::TempDir, Workspace, String) {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let mut d = bank::doc("Fictional health checking");
        d.closing_balance = Some("2000".into());
        d.retain_source = true;
        let s=bank::stage(&mut w,&c.id,dir.path(),"health.csv","Date,Description,Amount,Category,Type\n2026-09-01,Salary,3000,Wages,income\n2026-09-02,Rent,-1500,Housing,expense\n2026-09-03,Dining discussion,-300,Dining,expense\n2026-09-04,Service charge,-20,Fees,fee\n2026-09-05,Interest cost,-30,Interest,interest\n2026-09-06,Returned purchase,50,Dining,refund\n2026-09-07,Loan payment,-200,Debt,balanceMovement\n",d);
        for r in &s.records {
            decide(&mut w, &c.id, &r.id, Status::Approved)
        }
        facts(&mut w, &c.id);
        coverage(&mut w, &c.id, &query());
        (dir, w, c.id)
    }
    fn points(
        income: i64,
        net: i64,
        necessary: i64,
        savings: Option<i64>,
        required: Option<i64>,
        months: i64,
    ) -> Vec<Option<u16>> {
        components(
            true,
            true,
            ScoreBasis {
                income,
                net,
                necessary,
                savings,
                required,
                months,
            },
        )
        .into_iter()
        .map(|c| c.points)
        .collect()
    }
    fn audit(w: &Workspace) -> i64 {
        w.session
            .as_ref()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap()
    }
    fn reduction(a: &Assessment) -> ReductionInput {
        ReductionInput {
            query: query(),
            context: a.context.clone(),
            category: "Dining".into(),
            reduction_percent: 20,
            expense_line: None,
            destination: "unallocated".into(),
            allocation_line: None,
            cash_line: None,
            acknowledge: true,
        }
    }
    fn measure<'a>(a: &'a Assessment, key: &str) -> &'a Measure {
        a.measures.iter().find(|m| m.key == key).unwrap()
    }
    #[test]
    fn exact_score_thresholds_use_unrounded_values_and_distinguish_zero_from_missing() {
        assert_eq!(
            points(10000, 8000, 1000, Some(6000), Some(2000), 1),
            vec![Some(40), Some(30), Some(30)]
        );
        assert_eq!(
            points(10000, 8001, 1000, Some(5999), Some(2001), 1),
            vec![Some(30), Some(20), Some(25)]
        );
        assert_eq!(
            points(10000, 9500, 1000, Some(3000), Some(3500), 1),
            vec![Some(20), Some(20), Some(25)]
        );
        assert_eq!(
            points(10000, 9501, 1000, Some(2999), Some(3501), 1),
            vec![Some(10), Some(10), Some(15)]
        );
        assert_eq!(
            points(10000, 10001, 1000, Some(0), Some(5001), 1),
            vec![Some(0), Some(0), Some(0)]
        );
        assert_eq!(
            points(0, 10000, 1000, Some(0), Some(1000), 1),
            vec![Some(0), Some(0), Some(0)]
        );
        assert_eq!(
            points(0, 0, 1000, Some(0), Some(0), 1),
            vec![Some(0), Some(30), Some(0)]
        );
        assert_eq!(
            points(10000, 10000, 0, None, None, 1),
            vec![Some(10), None, None]
        );
        assert_eq!(
            points(20000, 16000, 2000, Some(6000), Some(2000), 2),
            vec![Some(40), Some(30), Some(30)]
        );
        assert!(components(
            false,
            true,
            ScoreBasis {
                income: 10000,
                net: 0,
                necessary: 1000,
                savings: Some(6000),
                required: Some(0),
                months: 1
            }
        )
        .iter()
        .all(|c| c.points.is_none()));
        assert_eq!(percent(-25, 10000).as_deref(), Some("−0.25%"));
        assert_eq!(percent(1, 0), None);
    }
    #[test]
    fn verified_healthy_projection_reuses_totals_refunds_costs_and_source_history_without_writes() {
        let (_dir, mut w, id) = fixture();
        let before = audit(&w);
        let originals = serde_json::to_value(
            w.get_client_imports(&id)
                .unwrap()
                .records
                .iter()
                .map(|r| &r.raw)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let a = view(&mut w, &id, query());
        assert!(
            a.gates.iter().all(|g| g.passed),
            "{:?}",
            a.gates
                .iter()
                .filter(|g| !g.passed)
                .map(|g| &g.label)
                .collect::<Vec<_>>()
        );
        assert_eq!(a.score, Some(95));
        assert_eq!(measure(&a, "income").amount.as_deref(), Some("3000.00"));
        assert_eq!(
            measure(&a, "necessary").income_percent.as_deref(),
            Some("51.66%")
        );
        assert_eq!(
            measure(&a, "netSpending").amount.as_deref(),
            Some("1800.00")
        );
        assert_eq!(
            measure(&a, "operatingSurplus").monthly.as_deref(),
            Some("1200.00")
        );
        assert_eq!(
            a.categories
                .iter()
                .find(|c| c.label == "Dining")
                .unwrap()
                .net_spending,
            "250.00"
        );
        assert_eq!(a.months[0].fees, "20.00");
        assert_eq!(a.months[0].interest, "30.00");
        let cost = a.insights.iter().find(|i| i.key == "costs").unwrap();
        assert_eq!(cost.monthly_impact.as_deref(), Some("50.00"));
        assert_eq!(cost.annual_impact.as_deref(), Some("600.00"));
        assert_eq!(cost.row_ids.len(), 2);
        assert!(a
            .evidence
            .iter()
            .all(|e| !e.approval_id.is_empty() && e.sha256.len() == 64));
        assert_eq!(audit(&w), before);
        assert_eq!(
            serde_json::to_value(
                w.get_client_imports(&id)
                    .unwrap()
                    .records
                    .iter()
                    .map(|r| &r.raw)
                    .collect::<Vec<_>>()
            )
            .unwrap(),
            originals
        );
        let s = w.get_client_imports(&id).unwrap();
        let record = s
            .records
            .iter()
            .find(|r| r.fields.description == "Loan payment")
            .unwrap();
        let p = w
            .query_unified_transactions(&id, unified_review::Query::default())
            .unwrap();
        let r = p.rows.iter().find(|r| r.record_id == record.id).unwrap();
        w.save_transaction_inspection(
            &id,
            unified_review::Save {
                context: p.context,
                expected_revision: p.revision,
                data: unified_review::Data::Transaction {
                    row_id: r.id.clone(),
                    binding: r.binding.clone(),
                    posting_date: None,
                    method: "loanPayment".into(),
                    confirmed_duplicate: false,
                },
                reason: "Verified this financial movement as loan repayment".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        let a = view(&mut w, &id, query());
        assert_eq!(
            measure(&a, "debtObserved").amount.as_deref(),
            Some("200.00")
        );
        assert_eq!(
            measure(&a, "netSpending").amount.as_deref(),
            Some("1800.00")
        );
    }
    #[test]
    fn missing_inventory_unknown_payments_zero_income_and_negative_cashflow_do_not_become_good_scores(
    ) {
        let (dir, mut w, id) = fixture();
        let another = w.save_client(client_input()).unwrap();
        let missing = view(&mut w, &another.id, query());
        assert_eq!(missing.score, None);
        assert!(missing.components.iter().all(|c| c.points.is_none()));
        assert!(missing
            .gates
            .iter()
            .any(|g| g.key == "balances" && !g.passed));
        let s = w.get_client_imports(&id).unwrap();
        let mut data = s.financial_facts.data.unwrap();
        data.balances
            .iter_mut()
            .find(|b| b.kind == BalanceKind::Debt)
            .unwrap()
            .monthly_payment = None;
        w.review_financial_facts(
            &id,
            ff::Input {
                expected_revision: s.revision,
                data,
                reason: "Required payment is not yet verified".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        coverage(&mut w, &id, &query());
        let a = view(&mut w, &id, query());
        assert!(a.score.is_none());
        assert!(a
            .components
            .iter()
            .find(|c| c.key == "debtObligations")
            .unwrap()
            .points
            .is_none());
        let zero = w.save_client(client_input()).unwrap();
        let mut d = bank::doc("Zero-income checking");
        d.closing_balance = Some("900".into());
        let s = bank::stage(
            &mut w,
            &zero.id,
            dir.path(),
            "zero.csv",
            "Date,Description,Amount,Category,Type\n2026-09-01,Rent,-100,Housing,expense\n",
            d,
        );
        decide(&mut w, &zero.id, &s.records[0].id, Status::Approved);
        facts(&mut w, &zero.id);
        let s = w.get_client_imports(&zero.id).unwrap();
        let mut data = s.financial_facts.data.unwrap();
        data.balances
            .iter_mut()
            .find(|b| b.kind == BalanceKind::AccessibleSavings)
            .unwrap()
            .amount = Some("0".into());
        w.review_financial_facts(
            &zero.id,
            ff::Input {
                expected_revision: s.revision,
                data,
                reason: "Verified zero income and zero accessible savings".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        coverage(&mut w, &zero.id, &query());
        let a = view(&mut w, &zero.id, query());
        assert_eq!(a.score, Some(0));
        assert_eq!(measure(&a, "income").amount.as_deref(), Some("0.00"));
        assert!(measure(&a, "netSpending").income_percent.is_none());
        assert_eq!(
            a.insights
                .iter()
                .find(|i| i.key == "deficit")
                .unwrap()
                .monthly_impact
                .as_deref(),
            Some("100.00")
        );
    }
    #[test]
    fn reversal_correction_overlap_and_date_gaps_withhold_scores_and_refresh_impacts() {
        let (dir, mut w, id) = fixture();
        let a = view(&mut w, &id, query());
        let old_token = a.context.clone();
        let r = w
            .get_client_imports(&id)
            .unwrap()
            .records
            .into_iter()
            .find(|r| r.fields.description == "Dining discussion")
            .unwrap();
        decide(&mut w, &id, &r.id, Status::Rejected);
        let rejected = view(&mut w, &id, query());
        assert!(rejected.score.is_none());
        assert!(rejected.excluded_rows > 0);
        assert!(rejected.evidence.is_empty());
        assert!(w
            .preview_financial_health_reduction(&id, reduction(&a))
            .is_err());
        decide(&mut w, &id, &r.id, Status::Approved);
        let pending = view(&mut w, &id, query());
        assert_eq!(pending.score, None);
        facts(&mut w, &id);
        coverage(&mut w, &id, &query());
        let restored = view(&mut w, &id, query());
        assert_eq!(restored.score, Some(95));
        assert_ne!(restored.context, old_token);
        let mut d = bank::doc("Fictional health checking");
        d.account_id = Some(w.get_client_imports(&id).unwrap().accounts[0].id.clone());
        d.opening_balance = Some("0".into());
        d.closing_balance = Some("1".into());
        let s = bank::stage(
            &mut w,
            &id,
            dir.path(),
            "overlap.csv",
            "Date,Description,Amount,Category,Type\n2026-09-08,Extra income,1,Other,income\n",
            d,
        );
        let row = s
            .records
            .iter()
            .find(|r| r.fields.description == "Extra income")
            .unwrap();
        decide(&mut w, &id, &row.id, Status::Approved);
        facts(&mut w, &id);
        coverage(&mut w, &id, &query());
        let overlap = view(&mut w, &id, query());
        assert_eq!(overlap.score, None);
        assert!(overlap
            .gates
            .iter()
            .any(|g| g.key == "coverage" && !g.passed));
        let q = Input {
            from_month: "2026-08".into(),
            through_month: "2026-09".into(),
        };
        coverage(&mut w, &id, &q);
        assert!(view(&mut w, &id, q)
            .gates
            .iter()
            .any(|g| g.key == "coverage" && !g.passed));
    }
    #[test]
    fn irregular_income_uses_all_calendar_months_and_keeps_negative_months_visible() {
        let (dir, mut w, _) = configured();
        let c = w.save_client(client_input()).unwrap();
        let mut d = bank::doc("Irregular receipts checking");
        d.period_start = Some("2026-08-01".into());
        d.closing_balance = Some("2000".into());
        let s=bank::stage(&mut w,&c.id,dir.path(),"irregular.csv","Date,Description,Amount,Category,Type\n2026-08-01,Contract receipt,3000,Wages,income\n2026-08-02,Rent,-1000,Housing,expense\n2026-08-03,Dining,-500,Dining,expense\n2026-09-01,Contract receipt,1000,Wages,income\n2026-09-02,Rent,-1000,Housing,expense\n2026-09-03,Dining,-500,Dining,expense\n",d);
        for r in &s.records {
            decide(&mut w, &c.id, &r.id, Status::Approved)
        }
        facts(&mut w, &c.id);
        let q = Input {
            from_month: "2026-08".into(),
            through_month: "2026-09".into(),
        };
        coverage(&mut w, &c.id, &q);
        let a = view(&mut w, &c.id, q);
        assert_eq!(a.score, Some(100));
        assert_eq!(measure(&a, "income").monthly.as_deref(), Some("2000.00"));
        assert_eq!(a.months[0].operating_surplus, "1500.00");
        assert_eq!(a.months[1].operating_surplus, "-500.00");
        assert!(a.insights.iter().any(|i| i.key == "irregularIncome"
            && i.basis.contains("1000.00")
            && i.basis.contains("3000.00")));
    }
    #[test]
    fn owned_savings_transfers_are_one_allocation_and_never_income_or_expense() {
        let (dir, mut w, id) = fixture();
        let mut d = bank::doc("Separate transfer checking");
        d.opening_balance = Some("1000".into());
        d.closing_balance = Some("700".into());
        let s=bank::stage(&mut w,&id,dir.path(),"transfer-out.csv","Date,Description,Amount,Category,Type\n2026-09-10,Move to own savings,-300,Reserve,transfer\n",d);
        let left = s
            .records
            .iter()
            .find(|r| r.fields.description == "Move to own savings")
            .unwrap()
            .id
            .clone();
        let mut d = bank::doc("Owned savings");
        d.account_kind = "savings".into();
        d.opening_balance = Some("1000".into());
        d.closing_balance = Some("1300".into());
        let s=bank::stage(&mut w,&id,dir.path(),"transfer-in.csv","Date,Description,Amount,Category,Type\n2026-09-10,Move from own checking,300,Reserve,transfer\n",d);
        let right = s
            .records
            .iter()
            .find(|r| r.fields.description == "Move from own checking")
            .unwrap()
            .id
            .clone();
        w.pair_transfer_transactions(
            &id,
            imports::PairInput {
                expected_revision: s.revision,
                left_id: left,
                right_id: right,
                reason: "Verified current reciprocal owned checking to savings transfer".into(),
            },
        )
        .unwrap();
        facts(&mut w, &id);
        coverage(&mut w, &id, &query());
        let a = view(&mut w, &id, query());
        assert_eq!(measure(&a, "savingsIn").amount.as_deref(), Some("300.00"));
        assert_eq!(
            measure(&a, "savingsIn").income_percent.as_deref(),
            Some("10.00%")
        );
        assert_eq!(measure(&a, "income").amount.as_deref(), Some("3000.00"));
        assert_eq!(
            measure(&a, "netSpending").amount.as_deref(),
            Some("1800.00")
        );
        assert_eq!(
            measure(&a, "otherTransfers").amount.as_deref(),
            Some("0.00")
        );
    }
    #[test]
    fn cross_period_transfer_legs_remain_unavailable_instead_of_becoming_savings_or_income() {
        let (dir, mut w, id) = fixture();
        let mut d = bank::doc("Cross-period checking");
        d.closing_balance = Some("700".into());
        let s=bank::stage(&mut w,&id,dir.path(),"cross-out.csv","Date,Description,Amount,Category,Type\n2026-09-30,Across month end,-300,Reserve,transfer\n",d);
        let left = s
            .records
            .iter()
            .find(|r| r.fields.description == "Across month end")
            .unwrap()
            .id
            .clone();
        let mut d = bank::doc("Cross-period savings");
        d.account_kind = "savings".into();
        d.period_start = Some("2026-10-01".into());
        d.period_end = Some("2026-10-31".into());
        d.closing_balance = Some("1300".into());
        let s=bank::stage(&mut w,&id,dir.path(),"cross-in.csv","Date,Description,Amount,Category,Type\n2026-10-01,Arrived next month,300,Reserve,transfer\n",d);
        let right = s
            .records
            .iter()
            .find(|r| r.fields.description == "Arrived next month")
            .unwrap()
            .id
            .clone();
        w.pair_transfer_transactions(
            &id,
            imports::PairInput {
                expected_revision: s.revision,
                left_id: left,
                right_id: right,
                reason: "Explicitly reviewed a transfer across calendar months".into(),
            },
        )
        .unwrap();
        facts(&mut w, &id);
        coverage(&mut w, &id, &query());
        let a = view(&mut w, &id, query());
        assert_eq!(a.score, None);
        assert!(a.gates.iter().any(|g| g.key == "transfers" && !g.passed));
        assert_eq!(measure(&a, "savingsIn").amount, None);
        assert_eq!(measure(&a, "income").amount.as_deref(), Some("3000.00"));
        assert_eq!(
            measure(&a, "netSpending").amount.as_deref(),
            Some("1800.00")
        );
    }
    fn plans(w: &mut Workspace, id: &str) {
        w.save_client_budget(
            id,
            budget::BudgetInput {
                expected_revision: 0,
                lines: vec![
                    ("Take-home", budget::Kind::Income, "3000"),
                    ("Necessary", budget::Kind::Necessary, "1550"),
                    ("Dining", budget::Kind::Discretionary, "300"),
                    ("Debt", budget::Kind::DebtPayment, "200"),
                    ("Emergency", budget::Kind::Savings, "200"),
                ]
                .into_iter()
                .map(|(label, kind, n)| budget::BudgetLine {
                    label: label.into(),
                    kind,
                    amount: Some(n.into()),
                    frequency: budget::Frequency::Monthly,
                    estimated: false,
                })
                .collect(),
            },
        )
        .unwrap();
        w.save_client_debt_plan(
            id,
            crate::debt::PlanInput {
                expected_revision: 0,
                data: crate::debt::PlanData {
                    start_month: "2026-10".into(),
                    monthly_payment: Some("200".into()),
                    accounts: vec![crate::debt::Account {
                        label: "Loan".into(),
                        debt_type: crate::debt::DebtType::PersonalLoan,
                        balance: Some("1000".into()),
                        apr: Some("0".into()),
                        minimum_payment: Some("200".into()),
                        monthly_fee: Some("0".into()),
                        interest_model: crate::debt::InterestModel::FixedMonthly,
                        verified: true,
                        terms_note: "Verified zero-interest fictional loan".into(),
                    }],
                    extras: vec![],
                    payment_changes: vec![],
                },
            },
        )
        .unwrap();
        w.save_client_savings_plan(
            id,
            crate::savings::PlanInput {
                expected_revision: 0,
                data: crate::savings::PlanData {
                    start_month: "2026-10".into(),
                    target_months: 6,
                    liquid_savings: Some("6000".into()),
                    restricted_savings: Some("0".into()),
                    obligations_reserved: Some("0".into()),
                    emergency_contribution: Some("200".into()),
                    alternative_contributions: vec![],
                    goals: vec![],
                    priorities_note: "Discussed explicit synthetic reserve goal".into(),
                },
            },
        )
        .unwrap();
        w.save_client_cashflow_plan(
            id,
            crate::cashflow::PlanInput {
                expected_revision: 0,
                data: crate::cashflow::PlanData {
                    start_date: "2026-10-01".into(),
                    end_date: "2026-12-31".into(),
                    accounts: vec![crate::cashflow::Account {
                        id: "checking".into(),
                        label: "Plan checking".into(),
                        opening_balance: Some("2000".into()),
                    }],
                    schedules: vec![crate::cashflow::Schedule {
                        label: "Dining dates".into(),
                        account_id: "checking".into(),
                        destination_id: None,
                        kind: crate::cashflow::Kind::Bill,
                        amount: Some("300".into()),
                        first_date: "2026-10-02".into(),
                        frequency: crate::cashflow::Frequency::Monthly,
                        second_day: None,
                        until_date: None,
                        certainty: crate::cashflow::Certainty::Estimated,
                    }],
                    notes: "Synthetic future dates explicitly discussed".into(),
                },
            },
        )
        .unwrap();
    }
    #[test]
    fn refund_adjusted_reductions_delegate_to_existing_debt_savings_and_calendar_without_mutation()
    {
        let (_dir, mut w, id) = fixture();
        plans(&mut w, &id);
        let a = view(&mut w, &id, query());
        let before = serde_json::to_value(w.get_assessment_planning(&id).unwrap()).unwrap();
        let events = audit(&w);
        let preview = w
            .preview_financial_health_reduction(&id, reduction(&a))
            .unwrap();
        assert_eq!(preview.monthly_impact, "50.00");
        assert_eq!(preview.annual_impact, "600.00");
        assert!(preview.scenario.is_none());
        let mut input = reduction(&a);
        input.expense_line = Some(2);
        input.destination = "debt".into();
        input.allocation_line = Some(3);
        input.cash_line = Some(0);
        let preview = w.preview_financial_health_reduction(&id, input).unwrap();
        let c = preview.scenario.unwrap();
        assert_eq!(
            c.proposed.plans.budget.lines[2].amount.as_deref(),
            Some("250.00")
        );
        assert_eq!(
            c.proposed
                .plans
                .debt
                .data
                .as_ref()
                .unwrap()
                .monthly_payment
                .as_deref(),
            Some("250.00")
        );
        let old = c
            .baseline
            .plans
            .debt
            .comparison
            .as_ref()
            .unwrap()
            .avalanche
            .as_ref()
            .unwrap();
        let new = c
            .proposed
            .plans
            .debt
            .comparison
            .as_ref()
            .unwrap()
            .avalanche
            .as_ref()
            .unwrap();
        assert_eq!(old.payoff_month.as_deref(), Some("2027-02"));
        assert_eq!(new.payoff_month.as_deref(), Some("2027-01"));
        assert_eq!(
            c.proposed
                .plans
                .cashflow
                .summary
                .as_ref()
                .unwrap()
                .combined_closing
                .as_deref(),
            Some("1250.00")
        );
        let mut input = reduction(&a);
        input.expense_line = Some(2);
        input.destination = "emergency".into();
        input.allocation_line = Some(4);
        let c = w
            .preview_financial_health_reduction(&id, input)
            .unwrap()
            .scenario
            .unwrap();
        assert_eq!(
            c.baseline
                .plans
                .savings
                .summary
                .as_ref()
                .unwrap()
                .emergency
                .months_needed,
            Some(17)
        );
        assert_eq!(
            c.proposed
                .plans
                .savings
                .summary
                .as_ref()
                .unwrap()
                .emergency
                .months_needed,
            Some(14)
        );
        assert_eq!(
            serde_json::to_value(w.get_assessment_planning(&id).unwrap()).unwrap(),
            before
        );
        assert_eq!(audit(&w), events);
        assert_eq!(view(&mut w, &id, query()).score, Some(95));
        let mut bad = reduction(&a);
        bad.reduction_percent = 101;
        assert!(w.preview_financial_health_reduction(&id, bad).is_err());
        let mut bad = reduction(&a);
        bad.category = "Nonexistent".into();
        assert!(w.preview_financial_health_reduction(&id, bad).is_err());
        let mut bad = reduction(&a);
        bad.expense_line = Some(0);
        assert!(w.preview_financial_health_reduction(&id, bad).is_err());
    }
    #[test]
    fn counselor_reclassification_recalculates_actual_metrics_and_reserve_score_with_preserved_history(
    ) {
        let (_dir, mut w, id) = fixture();
        let s = w.get_client_imports(&id).unwrap();
        let mut data = s.financial_facts.data.unwrap();
        data.balances
            .iter_mut()
            .find(|b| b.kind == BalanceKind::AccessibleSavings)
            .unwrap()
            .amount = Some("5000".into());
        w.review_financial_facts(
            &id,
            ff::Input {
                expected_revision: s.revision,
                data,
                reason: "Verified the accessible reserve separately".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        coverage(&mut w, &id, &query());
        assert_eq!(view(&mut w, &id, query()).score, Some(95));
        let q = query().spending();
        let analysis = w.get_spending_analysis(&id, q.clone()).unwrap();
        let row = analysis
            .rows
            .iter()
            .find(|r| r.category == "Dining" && r.editable)
            .unwrap();
        let row_id = row.id.clone();
        w.change_spending_classification(
            &id,
            sa::Change {
                context: analysis.context,
                query: q,
                row_id: row_id.clone(),
                approval_id: row.approval_id.clone(),
                role: sa::Role::Necessary,
                reason: "Counselor discussed this as necessary with the fictional client".into(),
                acknowledge: true,
            },
        )
        .unwrap();
        let current = view(&mut w, &id, query());
        assert_eq!(
            measure(&current, "necessary").amount.as_deref(),
            Some("1850.00")
        );
        assert_eq!(
            measure(&current, "discretionary").amount.as_deref(),
            Some("0.00")
        );
        assert!(current.score.is_none());
        coverage(&mut w, &id, &query());
        assert_eq!(view(&mut w, &id, query()).score, Some(85));
        let h = w.get_spending_classification_history(&id, &row_id).unwrap();
        assert!(h
            .events
            .iter()
            .any(|e| e.reason.contains("discussed this as necessary")));
        assert_eq!(h.original.reported_amount.as_deref(), Some("-300"));
    }
    #[test]
    fn combined_card_purchases_refunds_and_matched_repayments_use_the_same_approved_ledgers() {
        let (dir, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let (record, _) = crate::storage::card_final_policy_tests::fixture(
            &mut w,
            &client,
            dir.path(),
            "complete",
        );
        let q = Input {
            from_month: record.payload.assessment.period_start[..7].into(),
            through_month: record.payload.assessment.period_end[..7].into(),
        };
        let before = serde_json::to_value(&record.payload).unwrap();
        let a = w.get_financial_health(&client.id, q.clone()).unwrap();
        assert_eq!(
            a.score,
            Some(70),
            "{:?}",
            a.gates
                .iter()
                .filter(|g| !g.passed)
                .map(|g| &g.label)
                .collect::<Vec<_>>()
        );
        assert_eq!(measure(&a, "income").amount.as_deref(), Some("2000.01"));
        assert_eq!(
            measure(&a, "netSpending").amount.as_deref(),
            Some("1287.10")
        );
        assert_eq!(measure(&a, "refunds").amount.as_deref(), Some("20.02"));
        assert_eq!(measure(&a, "debtObserved").amount.as_deref(), Some("50.03"));
        assert_eq!(a.months[0].fees, "5.04");
        assert_eq!(a.months[0].interest, "2.05");
        let ledger = w.get_card_statements(&client.id).unwrap();
        w.review_card_transaction(
            &client.id,
            crate::storage::cards_store::tests::decision(
                &ledger,
                0,
                crate::cards::Kind::Purchase,
                Status::Rejected,
            ),
        )
        .unwrap();
        let after = w.get_financial_health(&client.id, q).unwrap();
        assert_eq!(after.score, None);
        assert_eq!(
            measure(&after, "netSpending").amount.as_deref(),
            Some("1200.02")
        );
        assert!(after
            .evidence
            .iter()
            .all(|e| !e.row_id.starts_with("card:")));
        let preserved = w
            .get_assessment_record(&client.id, &record.payload.id)
            .unwrap();
        assert_eq!(serde_json::to_value(preserved.payload).unwrap(), before);
    }
    #[test]
    fn authenticated_client_scope_context_revisions_lock_and_encrypted_recovery_preserve_results() {
        let (dir, mut w, id) = fixture();
        let other = w.save_client(client_input()).unwrap();
        let a = view(&mut w, &id, query());
        assert!(w.get_financial_health("not-a-client", query()).is_err());
        assert!(w
            .preview_financial_health_reduction(&other.id, reduction(&a))
            .is_err());
        let mut b = client_input();
        b.id = Some(id.clone());
        b.goals = "Changed profile context".into();
        w.save_client(b).unwrap();
        assert!(w
            .preview_financial_health_reduction(&id, reduction(&a))
            .is_err());
        coverage(&mut w, &id, &query());
        let a = view(&mut w, &id, query());
        let destination = tempfile::tempdir().unwrap();
        let backup = w
            .create_backup(destination.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        assert!(w.get_financial_health(&id, query()).is_err());
        drop(w);
        let mut w = Workspace::new(dir.path().into());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(view(&mut w, &id, query()).score, a.score);
        w.lock_workspace();
        let restore = w.prepare_restore(backup.path, PASSWORD.into()).unwrap();
        w.confirm_restore(restore.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        let restored = view(&mut w, &id, query());
        assert_eq!(restored.score, a.score);
        assert_eq!(restored.evidence.len(), 7);
        assert!(view(&mut w, &other.id, query()).evidence.is_empty());
    }
}
