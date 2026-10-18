//! Verified savings inventory history, separate from forecast contributions.
use crate::{
    budget::{money, parse_money},
    dashboard,
    financial_facts::{BalanceKind, Facts},
    imports,
};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub client_id: String,
    pub from_date: String,
    pub through_date: String,
    pub points: Vec<Point>,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub date: String,
    pub review_id: String,
    pub reviewed_at: String,
    pub complete: bool,
    pub accessible: Option<String>,
    pub restricted: Option<String>,
    pub scale: Option<u32>,
    pub sources: Vec<Source>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub label: String,
    pub amount: Option<String>,
    pub evidence: String,
    pub document_id: Option<String>,
    pub document_name: Option<String>,
}
pub fn project(
    client: &str,
    facts: &Facts,
    documents: &[imports::Document],
    input: &dashboard::Input,
) -> Result<Progress, String> {
    let (from, through, _) = dashboard::period(input)?;
    let mut dates = BTreeMap::new();
    for review in &facts.history {
        if let Some(date) = &review.data.as_of {
            if date >= &from.to_string() && date <= &through.to_string() {
                dates.insert(date.clone(), review);
            }
        }
    }
    let mut points = vec![];
    for (date, review) in dates {
        let rows: Vec<_> = review
            .data
            .balances
            .iter()
            .filter(|b| {
                matches!(
                    b.kind,
                    BalanceKind::AccessibleSavings | BalanceKind::RestrictedSavings
                )
            })
            .collect();
        // Absence from an incomplete inventory is missing evidence, not a zero balance.
        let sum = |kind: BalanceKind| -> Result<Option<String>, String> {
            let selected: Vec<_> = rows.iter().filter(|b| b.kind == kind).collect();
            if selected.is_empty() && !review.data.full_inventory {
                return Ok(None);
            }
            Ok(selected
                .iter()
                .map(|b| b.amount.as_deref().map(parse_money).transpose())
                .collect::<Result<Option<Vec<_>>, String>>()?
                .map(|v| money(v.iter().sum())))
        };
        points.push(Point {
            date,
            review_id: review.id.clone(),
            reviewed_at: review.reviewed_at.clone(),
            complete: review.data.full_inventory
                && review.data.balances.iter().all(|b| b.amount.is_some()),
            accessible: sum(BalanceKind::AccessibleSavings)?,
            restricted: sum(BalanceKind::RestrictedSavings)?,
            scale: None,
            sources: rows
                .iter()
                .map(|b| Source {
                    label: b.label.clone(),
                    amount: b.amount.clone(),
                    evidence: b.evidence.clone(),
                    document_id: b.document_id.clone(),
                    document_name: b.document_id.as_ref().and_then(|id| {
                        documents
                            .iter()
                            .find(|d| &d.id == id)
                            .map(|d| d.file_name.clone())
                    }),
                })
                .collect(),
        });
    }
    let max = points
        .iter()
        .filter_map(|p| p.accessible.as_deref())
        .map(crate::budget::parse_total)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    for p in &mut points {
        p.scale = p
            .accessible
            .as_deref()
            .map(crate::budget::parse_total)
            .transpose()?
            .map(|v| {
                if max == 0 {
                    0
                } else {
                    (i128::from(v) * 1000 / i128::from(max)) as u32
                }
            });
    }
    Ok(Progress{client_id:client.into(),from_date:from.to_string(),through_date:through.to_string(),points,warnings:vec!["Counselor-verified balance snapshots only, not inferred from transfers, statement totals or saved savings plans. The latest recorded review for each balance date supersedes earlier corrections at that date. Partial inventories and unknown balances remain labeled. Missing dates are not interpolated; changing account inventories can make totals incomparable. Restricted/reserved funds remain separate.".into()]})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::financial_facts::{Balance, Data, Review};
    fn review(id: &str, date: &str, amount: Option<&str>, complete: bool) -> Review {
        Review {
            id: id.into(),
            data: Data {
                as_of: Some(date.into()),
                full_inventory: complete,
                balances: vec![Balance {
                    label: "Fictional reserve".into(),
                    kind: BalanceKind::AccessibleSavings,
                    amount: amount.map(Into::into),
                    monthly_payment: None,
                    evidence: "Counselor verified synthetic balance".into(),
                    document_id: None,
                }],
                classifications: vec![],
            },
            reason: "Synthetic review".into(),
            reviewed_at: "2026-10-17T12:00:00Z".into(),
            counselor_name: "Fictional counselor".into(),
        }
    }
    #[test]
    fn latest_review_per_balance_date_wins_without_invented_months_or_unknown_zero() {
        let f = Facts {
            data: None,
            history: vec![
                review("old", "2026-08-31", Some("100"), true),
                review("corrected", "2026-08-31", Some("120"), true),
                review("missing", "2026-10-17", None, false),
            ],
        };
        let p = project(
            "client-a",
            &f,
            &[],
            &dashboard::Input {
                from_month: "2026-08".into(),
                through_month: "2026-10".into(),
            },
        )
        .unwrap();
        assert_eq!(p.points.len(), 2);
        assert_eq!(p.points[0].review_id, "corrected");
        assert_eq!(p.points[0].accessible.as_deref(), Some("120.00"));
        assert_eq!(p.points[0].restricted.as_deref(), Some("0.00"));
        assert_eq!(p.points[0].scale, Some(1000));
        assert!(
            p.points[1].accessible.is_none()
                && p.points[1].restricted.is_none()
                && p.points[1].scale.is_none()
        );
        assert!(!p.points[1].complete);
        assert_eq!(f.history[0].data.balances[0].amount.as_deref(), Some("100"));
    }
}
