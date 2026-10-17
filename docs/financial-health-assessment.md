# Financial health assessment and behavioral insights

## Prerequisite and reuse inspection

The preceding extraction/current-transaction review is complete through `b6bd43d`.
Its accepted source and standalone binary hashes match this checkout, and the
focused native acceptance exit is 0, including actual PDF rendering, approval
changes, downstream analysis, client isolation, restart and encrypted recovery.
Evidence: `/workspace/shared/unified-review-refinements-validation/completed.json`.
This assignment does not restart or replace those systems. No installer is authorized.

| Requirement | Reused implementation | New work |
| --- | --- | --- |
| Approved activity and exact money | `imports_store`, `cards_store`, `spending_analysis`, `dashboard`, `financial_facts` | One scoped read-only health projection and versioned explainable scoring policy |
| Roles, refunds and repayment boundaries | Existing approval-bound bank/card purposes, `card_review` and unified rows | Shared evidence IDs and separate spending, obligation and transfer measures |
| Missing/overlapping/incomplete evidence | `completeness`, saved `coverage_review`, `card_facts`, statement reconciliation | Explicit score gates; missing values never count as poor performance |
| Recurring payments, costs and patterns | Current approved rows, existing confirmed subscription reviews and method analysis | Monthly observations and source-linked recommendations with stated amounts |
| Counselor classification | Existing Spending Analysis / bank/card review forms and immutable history | Navigate to the working editing workflow; no second approval service |
| Potential reductions | Actual net category spending and explicit counselor assumptions | Exact non-writing monthly/annual impact, with no merchant-based moral judgment |
| Debt/savings/cash flow/scenarios | `planning`, `scenarios`, `budget`, `debt`, `savings`, `cashflow` | Explicit mapping of a reduction to a saved monthly expense; delegate projections |
| Privacy, history and reports | Authenticated client scope, SQLCipher, existing immutable reports | Source-context freshness and read-only projections; no duplicate transaction store |

## Scoring policy — counseling-health-v1

The 0–100 index is an explainable counseling aid, not a credit score, diagnosis,
validated prediction or guarantee. No merchant, identity or payment-card preference
changes a score. Three components sum to 100 possible points:

- Operating margin: 40 points. Approved income less combined net expenses,
  divided by approved income. Below 0% earns 0; 0–<5% earns 10; 5–<10% earns 20;
  10–<20% earns 30; at least 20% earns 40. This is operating surplus before debt
  principal and transfers, not a promise of available checking cash.
- Required debt payments: 30 points. Verified required monthly payments divided
  by average monthly approved income. No debt earns 30; >0–20% earns 30;
  >20–35% earns 20; >35–50% earns 10; >50% earns 0. Verified zero income with
  positive payments earns 0; missing payments remain unavailable.
- Emergency reserves: 30 points. Dated verified accessible savings divided by
  average monthly combined necessary gross expenses. Zero coverage earns 0;
  >0–<1 month earns 5; 1–<3 earns 15; 3–<6 earns 25; at least 6 earns 30.
  Zero necessary expenses has no meaningful reserve denominator, so it is
  unavailable rather than perfect. Restricted savings is excluded.

Thresholds are product policy, not mandatory client targets. Components explain
their figures, thresholds, points and missing evidence separately. The overall
score is withheld unless essential transaction coverage, period, current roles,
balances and required-payment facts are complete. Coverage must be reviewed;
empty imports do not prove zero income or absence of cards. Verified zero values
are distinguishable from unknown values. Incomplete observed activity can still
be displayed with an explicit limitation and without a definitive score.

## Arithmetic and presentation boundaries

Use Rust integer cents and wider integer intermediates. Ratios truncate to two
decimal places; monthly averages and reduction amounts round half a cent upward.
Compare score thresholds against exact underlying values, not rounded ratios.
Monthly figures use every calendar month selected, including verified zero-income
months; irregular income is shown by month and never projected as guaranteed.
Gross necessary/discretionary expenses and refunds are separate; net spending
subtracts refunds exactly once. Card repayments and loan principal are movements,
not additional purchases. Savings contributions require actual reviewed owned
checking/savings transfer evidence; other movements remain explicitly separate.

Reduction choices are counselor-selected assumptions based on actual net category
activity, not automatic declarations that merchants or purchases are unhealthy.
Annual impacts are monthly assumptions × 12, not promises. Debt repayment,
emergency funding and dated cash projections must run through existing planning
services with explicit saved-line mapping and source-context checks. Historical
transactions, saved plans and previously issued reports are never silently edited.

## Acceptance checklist

- [x] Verify preceding transaction-review evidence and inspect canonical services.
- [ ] Show score/components/explanations and withhold overall score for essential unknowns.
- [ ] Show approved monthly patterns, costs, recurring expenses and current purposes.
- [ ] Show income-relative expenses, obligations, savings and transfers with distinct bases.
- [ ] Support existing classification editing and explicit monthly/annual reduction previews.
- [ ] Explain every recommendation with actual evidence, impact and uncertainty.
- [ ] Delegate saved-plan projections to existing engines with no automatic plan writes.
- [ ] Preserve scope, original evidence, approval reversal behavior and stale-result guards.
- [ ] Cover missing/zero/irregular income, deficits, incomplete/overlapping histories,
  refunds, repayments/transfers, threshold boundaries and source corrections in tests.
- [ ] Pass frontend/Rust/build/native acceptance, including lock/restart/recovery and isolation.
- [ ] Record executed outcomes, outstanding limits and next actions; commit working progress.

Implementation and executed checks will be recorded here and in development status.
