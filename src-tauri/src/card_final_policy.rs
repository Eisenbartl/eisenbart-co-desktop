//! Frozen v2 approval rules over original format-9 evidence. No bank/card v1 overrides.
use crate::{
    assessments,
    budget::money,
    card_facts::Applicability,
    final_reports::{self, AccountScope, Gate, Readiness},
    financial_facts,
    imports::{Kind, Status},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

pub const POLICY: &str = "counselor-final-report-v2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub calculator: String,
    pub bank_cash_movement: String,
    pub bank_net_spending: String,
    pub card_net_spending: String,
    pub necessary_spending: String,
    pub discretionary_spending: String,
    pub combined_spending: String,
    pub income_less_spending: String,
    pub matched_repayments: String,
    pub existing_household_debt: String,
}

// Calculated totals can exceed the per-row input limit. Check the canonical
// fixed-two-decimal representation rather than parsing a float or input amount.
fn cents(s: &str) -> Result<i64, String> {
    let n = s
        .replace('.', "")
        .parse::<i64>()
        .map_err(|_| "Invalid calculated money total.")?;
    if money(n) != s {
        return Err("Noncanonical calculated money total.".into());
    }
    Ok(n)
}
fn sum(a: &str, b: &str) -> Result<String, String> {
    Ok(money(
        cents(a)?
            .checked_add(cents(b)?)
            .ok_or("Calculated money total overflow.")?,
    ))
}

pub fn readiness(
    p: &assessments::Payload,
    digest: &str,
    changed: bool,
    today: NaiveDate,
) -> Result<Readiness, String> {
    // Reuse the frozen bank-only gates, preserving their original wording and
    // interpretation for old receipts. Only v2 removes the unconditional card gate.
    let mut r = final_reports::readiness(p, digest, changed, today)?;
    r.policy = POLICY.into();
    r.gates.retain(|g| g.key != "cardPolicy");
    let mut add = |key: &str, label: &str, passed: bool, detail: &str| {
        r.gates.push(Gate {
            key: key.into(),
            label: label.into(),
            passed,
            detail: detail.into(),
        })
    };
    add("cardCapture", "Captured bank and card evidence", p.format == 9 && p.cards.is_some() && p.card_facts.is_some(), "Use a format-9 assessment with the exact reviewed bank/card supplement and saved card completeness version. Older internal reports keep their original interpretation.");
    let (Some(cards), Some(facts)) = (&p.cards, &p.card_facts) else {
        r.ready = false;
        return Ok(r);
    };
    let a = &p.assessment;
    let inside = |date: Option<&str>| {
        date.is_some_and(|d| d >= a.period_start.as_str() && d <= a.period_end.as_str())
    };
    add("cardEvidence", "Complete reviewed card inventory and balances", facts.evaluation.evidence_complete && facts.evaluation.issues.is_empty(), "Resolve every recorded card inventory, source, applicability, role and closing-debt finding. Unknowns, missing cards, credits, adjustments and discrepancies cannot be waived.");
    add("cardCalculator", "Reconciled card activity and repayments", cards.review.calculator == "bank-card-review-v1" && cards.review.issues.is_empty() && cards.review.observed_spending.is_some(), "The original bank/card calculator must have no unresolved issue. Each card repayment needs its exact current, equal bank counterpart in the same assessment period; split and cross-period cases stay blocked.");
    let whole_period = facts.data.accounts.iter().all(|e| match e.applicability {
        Applicability::Applicable => {
            e.start.as_deref() == Some(&a.period_start) && e.end.as_deref() == Some(&a.period_end)
        }
        Applicability::NotApplicable => true,
        Applicability::Unknown => false,
    });
    add("cardScope", "Supported card applicability", whole_period, "Policy v2 supports full-period applicable cards and explicitly explained nonapplicable cards. Opening/closure balance semantics require a future policy; notes cannot override this check.");
    let selected_sources = facts
        .data
        .accounts
        .iter()
        .flat_map(|e| e.source_ids.iter())
        .collect::<std::collections::HashSet<_>>();
    let card_sources = cards.ledger.statements.iter().all(|s| {
        let relevant = s.source.input.period_start <= a.period_end
            && s.source.input.period_end >= a.period_start
            || s.rows
                .iter()
                .any(|row| row.status == Status::Approved && inside(row.fields.date.as_deref()));
        !relevant
            || selected_sources.contains(&s.source.id)
                && s.source.input.period_start >= a.period_start
                && s.source.input.period_end <= a.period_end
                && s.rows.iter().all(|row| {
                    !matches!(row.status, Status::Pending | Status::NeedsClarification)
                        && row.fields.date.as_deref().is_some_and(|d| {
                            d >= s.source.input.period_start.as_str()
                                && d <= s.source.input.period_end.as_str()
                        })
                })
    });
    add("cardSources", "All applicable card sources and row decisions", card_sources, "Every relevant card statement must be selected and lie inside the assessment period, with source-wide completed decisions and dated rows inside its declared period. Approved activity outside declared coverage cannot disappear.");
    let data = p.basis.financial_facts.data.as_ref();
    let roles = p
        .basis
        .records
        .iter()
        .filter(|b| b.status == Status::Approved && inside(b.fields.date.as_deref()))
        .all(|b| {
            let needs_role = matches!(b.fields.kind, Kind::Income | Kind::Expense | Kind::Fee)
                || b.fields.kind == Kind::Interest
                    && b.fields
                        .amount
                        .as_deref()
                        .is_some_and(|n| n.starts_with('-'));
            !needs_role
                || data.is_some_and(|d| {
                    d.classifications.iter().any(|c| {
                        c.record_id == b.id && financial_facts::active(c, b, &p.basis.history)
                    })
                })
        });
    add("bankRoles", "Every bank income and spending role reviewed", roles, "Each eligible approved bank row needs its exact current role binding, even when unclassified positive and negative amounts offset to zero. Categories cannot supply these roles.");
    let movements = p
        .basis
        .records
        .iter()
        .filter(|b| b.status == Status::Approved && inside(b.fields.date.as_deref()))
        .all(|b| match b.fields.kind {
            Kind::BalanceMovement => cards.links.repayments.iter().any(|link| {
                link.bank_record_id == b.id
                    && crate::card_review::current(link, &p.basis, &cards.ledger)
                    && cards
                        .ledger
                        .statements
                        .iter()
                        .find(|s| s.source.id == link.statement_id)
                        .and_then(|s| s.rows.iter().find(|r| r.row == link.row))
                        .is_some_and(|row| inside(row.fields.date.as_deref()))
            }),
            Kind::Transfer => b.linked_id.as_ref().is_some_and(|id| {
                p.basis.records.iter().any(|other| {
                    &other.id == id
                        && other.status == Status::Approved
                        && inside(other.fields.date.as_deref())
                        && other.linked_id.as_ref() == Some(&b.id)
                })
            }),
            _ => true,
        });
    add("bankMovements", "Bank movements have supported counterpart evidence", movements, "Every approved balance movement must be a current matched card repayment; transfers need their reciprocal approved in-period leg. Other borrowing, principal, split or timing treatments require explicit future semantics, never invented income or expenses.");
    let obligations = data.is_some_and(|d| {
        d.balances
            .iter()
            .filter(|b| b.kind == financial_facts::BalanceKind::Debt)
            .all(|b| {
                b.monthly_payment.is_some()
                    && facts.data.accounts.iter().any(|e| {
                        e.applicability == Applicability::Applicable
                            && e.balance_label.as_ref() == Some(&b.label)
                    })
            })
    });
    add("obligations", "Recorded debt obligations within supported scope", obligations, "Every recorded debt must have a known required monthly payment and a distinct reviewed card mapping. Other debt/loan obligations need their own supported evidence workflow before card-inclusive approval. The counselor must confirm the declared full household inventory.");
    let m = &a.facts_metrics;
    let bank_roles = sum(&m.necessary_spending, &m.discretionary_spending)?;
    let necessary = sum(&m.necessary_spending, &facts.evaluation.necessary_spending)?;
    let discretionary = sum(
        &m.discretionary_spending,
        &facts.evaluation.discretionary_spending,
    )?;
    let combined = sum(&necessary, &discretionary)?;
    let card_roles = sum(
        &facts.evaluation.necessary_spending,
        &facts.evaluation.discretionary_spending,
    )?;
    let totals_match = bank_roles == cards.review.bank_net_spending
        && card_roles == cards.review.card_net_spending
        && cards.review.observed_spending.as_ref() == Some(&combined);
    add("combinedFigures", "Exact combined spending agrees with original evidence", totals_match, "Necessary and discretionary bank/card spending must reconcile to the captured subtotal. Unsupported bank refund-role treatments stay blocked. Card refunds offset their reviewed role; repayments are excluded and existing debt is never added again.");
    r.ready = r.gates.iter().all(|g| g.passed);
    // Withhold the official combined summary whenever its financial basis fails.
    if r.gates
        .iter()
        .all(|g| g.passed || matches!(g.key.as_str(), "context" | "period"))
    {
        r.card_summary = Some(Summary {
            calculator: "card-inclusive-final-v1".into(),
            bank_cash_movement: cards.review.bank_cash_movement.clone(),
            bank_net_spending: cards.review.bank_net_spending.clone(),
            card_net_spending: card_roles,
            necessary_spending: necessary,
            discretionary_spending: discretionary,
            combined_spending: combined.clone(),
            income_less_spending: money(
                cents(&a.totals.income)?
                    .checked_sub(cents(&combined)?)
                    .ok_or("Calculated money total overflow.")?,
            ),
            matched_repayments: cards.review.matched_payments.clone(),
            existing_household_debt: m
                .debt
                .clone()
                .ok_or("Verified household debt is unknown.")?,
        });
    }
    for e in &facts.data.accounts {
        if let Some(id) = &e.account_id {
            r.accounts.push(AccountScope {
                account_id: format!("card:{id}"),
                label: format!("Card / {}", e.label),
                applicability: if e.applicability == Applicability::NotApplicable {
                    "notApplicable"
                } else if e.start.as_deref() == Some(&a.period_start)
                    && e.end.as_deref() == Some(&a.period_end)
                {
                    "fullPeriod"
                } else {
                    "unconfirmed"
                }
                .into(),
                expected_start: e.start.clone(),
                expected_end: e.end.clone(),
                exception: e.notes.clone(),
                reason_required: true,
            });
        }
    }
    r.disclosures.push("Policy v2 approves only the bounded recorded bank/card evidence. Combined income less spending differs from bank cash movement: card purchases and repayments have different timing. Matched repayments are excluded from spending; closing card debt is compared to the existing inventory once. No planning model, bank-only metric or old policy receipt is rewritten.".into());
    r.disclosures.push("Original v1 supplement/evidence disclosures below remain historical statements about those calculators. This separate v2 approval and its combined summary establish the recorded approval scope; they do not upgrade or rewrite those original calculations. Other loan obligations, credits, early closure and unsupported repayment/refund treatments remain outside this policy.".into());
    Ok(r)
}
