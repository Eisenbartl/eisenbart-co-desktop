//! Read-only presentation of existing saved plan results. No new persistence.
use super::*;
use crate::budget::{money, parse_money, Kind};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub client_id: String,
    pub monthly_income: Option<String>,
    pub necessary_expenses: Option<String>,
    pub unallocated: Option<String>,
    pub entered_debt: Option<String>,
    pub debt_monthly_payment: Option<String>,
    pub budget_debt_payment: Option<String>,
    pub emergency_target: Option<String>,
    pub accessible_savings: Option<String>,
    pub restricted_savings: Option<String>,
    pub payoff_month: Option<String>,
    pub warnings: Vec<String>,
}

fn summarize(p: crate::planning::Plans) -> Result<Overview, String> {
    let known = |kind: Kind| {
        p.budget.revision > 0
            && p.budget.lines.iter().any(|l| l.kind == kind)
            && p.budget
                .lines
                .iter()
                .filter(|l| l.kind == kind)
                .all(|l| l.amount.is_some())
    };
    let mut warnings = Vec::new();
    let entered_debt = if let Some(data) = &p.debt.data {
        if data.accounts.is_empty() || data.accounts.iter().any(|a| a.balance.is_none()) {
            None
        } else {
            let total = data.accounts.iter().try_fold(0i64, |sum, a| {
                sum.checked_add(parse_money(a.balance.as_deref().unwrap())?)
                    .ok_or_else(|| "Entered debt total exceeds the supported range.".to_owned())
            })?;
            Some(money(total))
        }
    } else {
        None
    };
    let debt_payment = p.debt.data.as_ref().and_then(|d| d.monthly_payment.clone());
    let budget_payment = (p.budget.revision > 0
        && p.budget
            .lines
            .iter()
            .filter(|l| l.kind == Kind::DebtPayment)
            .all(|l| l.amount.is_some()))
    .then(|| p.budget.summary.planned_debt_payments.clone());
    if let (Some(debt), Some(budget)) = (&debt_payment, &budget_payment) {
        if parse_money(debt)? != parse_money(budget)? {
            warnings.push(format!("Debt payment mismatch: the saved simulation uses ${debt} per month, while the saved budget allocates ${budget}. Review and save consistent inputs before relying on a combined scenario."));
        }
    }
    if !known(Kind::Income) {
        warnings.push(
            "Monthly take-home income is unknown or incomplete. Review the saved budget.".into(),
        );
    }
    if p.budget.summary.missing_count > 0 {
        warnings
            .push("Some saved budget amounts are unknown; unallocated cash is unavailable.".into());
    }
    if entered_debt.is_none() {
        warnings.push("Entered debt balances are unknown or incomplete. Statement balances are reviewed separately.".into());
    }
    Ok(Overview {
        client_id: p.client_id,
        monthly_income: known(Kind::Income).then(|| p.budget.summary.monthly_income.clone()),
        necessary_expenses: known(Kind::Necessary)
            .then(|| p.budget.summary.necessary_expenses.clone()),
        unallocated: (known(Kind::Income) && p.budget.summary.missing_count == 0)
            .then(|| p.budget.summary.unallocated.clone()),
        entered_debt,
        debt_monthly_payment: debt_payment,
        budget_debt_payment: budget_payment,
        emergency_target: p
            .savings
            .summary
            .as_ref()
            .and_then(|s| s.emergency_target.clone()),
        accessible_savings: p
            .savings
            .summary
            .as_ref()
            .and_then(|s| s.accessible_emergency.clone()),
        restricted_savings: p
            .savings
            .data
            .as_ref()
            .and_then(|s| s.restricted_savings.clone()),
        payoff_month: p
            .debt
            .comparison
            .as_ref()
            .and_then(|c| c.snowball.as_ref())
            .and_then(|s| s.payoff_month.clone()),
        warnings,
    })
}

impl Workspace {
    pub fn get_client_workspace_overview(&mut self, client: &str) -> Result<Overview, String> {
        summarize(self.get_assessment_planning(client)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::{client_input, configured};
    #[test]
    fn overview_is_scoped_non_writing_and_preserves_unknowns_and_explicit_zero() {
        let (_root, mut w, _) = configured();
        let a = w.save_client(client_input()).unwrap();
        let b = w.save_client(client_input()).unwrap();
        let before: i64 = w
            .session()
            .unwrap()
            .connection
            .query_row("SELECT count(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        let empty = w.get_client_workspace_overview(&a.id).unwrap();
        assert!(
            empty.monthly_income.is_none()
                && empty.entered_debt.is_none()
                && empty.restricted_savings.is_none()
        );
        assert!(empty.unallocated.is_none());
        w.save_client_budget(
            &a.id,
            crate::budget::BudgetInput {
                expected_revision: 0,
                lines: vec![crate::budget::BudgetLine {
                    label: "Fictional explicit zero".into(),
                    kind: Kind::Income,
                    amount: Some("0.00".into()),
                    frequency: crate::budget::Frequency::Monthly,
                    estimated: false,
                }],
            },
        )
        .unwrap();
        let saved_audit: i64 = w
            .session()
            .unwrap()
            .connection
            .query_row("SELECT count(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(saved_audit, before + 1);
        assert_eq!(
            w.get_client_workspace_overview(&a.id)
                .unwrap()
                .monthly_income
                .as_deref(),
            Some("0.00")
        );
        assert!(w
            .get_client_workspace_overview(&b.id)
            .unwrap()
            .monthly_income
            .is_none());
        assert!(w
            .get_client_workspace_overview("unavailable-client")
            .is_err());
        let after: i64 = w
            .session()
            .unwrap()
            .connection
            .query_row("SELECT count(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after, saved_audit);
        w.lock_workspace();
        assert!(w.get_client_workspace_overview(&a.id).is_err());
    }
    #[test]
    fn exact_saved_figures_and_payment_mismatch_refresh_without_changing_plans() {
        let (_root, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let line = |label: &str, kind, amount: &str| crate::budget::BudgetLine {
            label: label.into(),
            kind,
            amount: Some(amount.into()),
            frequency: crate::budget::Frequency::Monthly,
            estimated: false,
        };
        let mut lines = vec![
            line("Fictional take-home", Kind::Income, "3000.00"),
            line("Fictional necessities", Kind::Necessary, "1175.00"),
        ];
        w.save_client_budget(
            &client.id,
            crate::budget::BudgetInput {
                expected_revision: 0,
                lines: lines.clone(),
            },
        )
        .unwrap();
        let mut debt = crate::storage::tests::debt_input(0);
        debt.data.accounts.truncate(1);
        debt.data.accounts[0].balance = Some("5000.00".into());
        debt.data.monthly_payment = Some("200.00".into());
        w.save_client_debt_plan(&client.id, debt).unwrap();
        let mut savings = crate::storage::tests::savings_input(0);
        savings.data.target_months = 3;
        savings.data.restricted_savings = None;
        w.save_client_savings_plan(&client.id, savings).unwrap();
        let overview = w.get_client_workspace_overview(&client.id).unwrap();
        assert_eq!(overview.monthly_income.as_deref(), Some("3000.00"));
        assert_eq!(overview.necessary_expenses.as_deref(), Some("1175.00"));
        assert_eq!(overview.unallocated.as_deref(), Some("1825.00"));
        assert_eq!(overview.entered_debt.as_deref(), Some("5000.00"));
        assert_eq!(overview.emergency_target.as_deref(), Some("3525.00"));
        assert!(overview.restricted_savings.is_none());
        assert!(overview
            .warnings
            .iter()
            .any(|w| w.contains("$200.00") && w.contains("$0.00")));
        assert_eq!(w.get_client_debt_plan(&client.id).unwrap().revision, 1);
        lines.push(line(
            "Fictional debt allocation",
            Kind::DebtPayment,
            "200.00",
        ));
        w.save_client_budget(
            &client.id,
            crate::budget::BudgetInput {
                expected_revision: 1,
                lines: lines.clone(),
            },
        )
        .unwrap();
        let reconciled = w.get_client_workspace_overview(&client.id).unwrap();
        assert_eq!(reconciled.unallocated.as_deref(), Some("1625.00"));
        assert!(!reconciled.warnings.iter().any(|w| w.contains("mismatch")));
        lines[1].amount = None;
        w.save_client_budget(
            &client.id,
            crate::budget::BudgetInput {
                expected_revision: 2,
                lines,
            },
        )
        .unwrap();
        let incomplete = w.get_client_workspace_overview(&client.id).unwrap();
        assert!(incomplete.necessary_expenses.is_none() && incomplete.unallocated.is_none());
        let (_other_root, mut other, _) = configured();
        let other_client = other.save_client(client_input()).unwrap();
        assert!(w.get_client_workspace_overview(&other_client.id).is_err());
    }
}
