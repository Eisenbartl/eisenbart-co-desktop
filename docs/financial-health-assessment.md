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

Implemented in 0.51.0 without a schema change (schema 32). Open the existing
client **Financial dashboard**, choose months, and use the integrated health view.
The earlier bank dashboard remains available in its disclosure.

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
balances and required-payment facts are complete. Current reciprocal transfer
legs are also required; cross-period transfers leave allocation metrics unavailable. Coverage must be reviewed;
empty imports do not prove zero income or absence of cards. Verified zero values
are distinguishable from unknown values. Incomplete observed activity can still
be displayed with an explicit limitation and without a definitive score.

## Arithmetic and presentation boundaries

Use Rust integer cents and wider integer intermediates. Ratios truncate to two
decimal places; monthly averages and reduction amounts round half a cent upward. Freshness and completed-month checks use America/Chicago
calendar dates, consistent with the application’s counseling workflow.
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
- [x] Show score/components/explanations and withhold overall score for essential unknowns.
- [x] Show approved monthly patterns, costs, recurring expenses and current purposes.
- [x] Show income-relative expenses, obligations, savings and transfers with distinct bases.
- [x] Support existing classification editing and explicit monthly/annual reduction previews.
- [x] Explain every recommendation with actual evidence, impact and uncertainty.
- [x] Delegate saved-plan projections to existing engines with no automatic plan writes.
- [x] Preserve scope, original evidence, approval reversal behavior and stale-result guards.
- [x] Cover missing/zero/irregular income, deficits, incomplete/overlapping histories,
  refunds, repayments/transfers, threshold boundaries and source corrections in tests.
- [x] Pass frontend/Rust/build/native acceptance, including lock/restart/recovery and isolation.
- [x] Record executed outcomes, outstanding limits and next actions; commit working progress.

Implementation and executed checks are recorded here and in development status.

## Completed workflow and executed checks

The Financial dashboard retains its explicit assessment-period form. Health
components, evidence gates, income-relative rows, month-by-month behavior and
conditional recommendations appear above the existing bank-detail disclosure.
Balance facts retain their exact review ID, as-of date, counselor and evidence;
transaction evidence retains source hash, approval ID and actual page/line.
Existing classification editing opens Spending Analysis for the same months.
A discussion-bound purpose change recalculates current metrics immediately and
withholds the definitive score until its inventory review is renewed.

The optional reduction preview uses net category activity after refunds, explicit
percentage, counselor acknowledgment and optional saved monthly-line mappings.
Changing an assumption requires acknowledgment again. It can reuse the existing
combined scenario view for debt or emergency funding and an explicitly mapped
dated expense. It never writes transactions, classifications, saved plans or old
reports. Unmapped calendar obligations remain unchanged and are clearly disclosed.
Saved scenarios and final reports continue through their existing reviewed flows.

Ten initial health tests and the additional cross-period transfer test passed,
including independent score thresholds, genuinely zero income, negative cash
flow, irregular receipts, mixed bank/card repayments and refunds, classification
changes, overlaps/gaps, stale contexts, client scope and encrypted recovery.
Independent planner examples verify a $50/month, $600/year reduction after a $50
refund, a zero-interest loan payoff changing from five months to four, and an
emergency goal changing from 17 months to 14, with unchanged saved inputs/audit.
The frontend production build passed. Initial complete UI verification reached
322 tests; one assertion needed to distinguish unavailable component points from
an unavailable balance payment, after which all 322 passed. The additional final
UI pass also passed all 322 tests, including period handoff and renewed acknowledgment.
The full Rust matrix passed all 316 library tests and both actual supervised
PDF-worker integration tests in one sequential run. Final Clippy and the embedded
standalone no-bundle build passed. After a final clarification of two dashboard
scope descriptions, all 91 affected UI tests and the embedded standalone build
passed again; the calculators, transaction review and persistence were unchanged.

The current 0.51.0 unified-review native workflow passed all three checkpoints in
`/workspace/shared/desktop-smoke-financial-health-first`: actual PDF pages/zoom,
combined filters and payment shares, existing approval/purpose/duplicate handling,
downstream updates, narrow layout, isolation, restart and encrypted recovery.
That combined run later stopped at a health-fixture locator error and is not an
overall passing run. The first two health attempts used PDF-style record numbering
for a headered CSV; the third saved coverage successfully but waited for the wrong
status element. Harness-only fixes retained the real financial forms and avoided
replaying uncertain writes; every rerun used fresh isolated synthetic data.

Focused health acceptance then passed, and the final run after the descriptive
copy and narrow-capture refinement passed again with exit 0 and its completion
marker: `/workspace/shared/desktop-smoke-financial-health-final`. Its three
checkpoints verify missing-data withholding followed by 95/100; exact spending,
refunds, ratios, costs and source/balance evidence; a $50/month, $600/year reduction
through existing plans without saved-input mutation; an existing purpose edit,
renewed coverage and 85/100; an 800px one-column score layout without page overflow;
and lock, full restart, encrypted recovery and strict client isolation. The score,
reduction and narrow screenshots were visually inspected. Source and standalone
binary hashes remained unchanged throughout acceptance; native processes and
driver ports closed before any later Cargo command. Final evidence is retained in
`/workspace/shared/financial-health-validation/completed.json`.

A subsequent test-only refinement uses the same fixed fixture date for assessment
and reduction, preventing calendar-day rollover from breaking tests, and explicitly
asserts that the following day invalidates an old preview context. An initial helper
name shadowed a local value and failed compilation; renaming it corrected the test
helper. All 11 affected health tests passed again, with Clippy warnings denied.
The runtime source prefix and accepted standalone binary remained byte-identical.
The full 316-test/2-worker run preceded this test-only refinement; it was not repeated.
The historical 117-checkpoint complete native regression was not repeated for 0.51.0;
current native evidence is the focused unified-review and health workflows above.

## Remaining limits and next required actions

All acceptance criteria for this assignment are satisfied for the existing supported
bank/card ledgers. This is a development build, with these explicit limits:

- The score is a transparent product-policy counseling index, not an independently
  validated credit rating or financial prediction. Counselors should review its
  thresholds with the client and inspect the component evidence.
- Missing, unreconciled, overlapping, stale or incomplete essential evidence withholds
  a definitive score. Supported periods are 1–24 months; existing bounded import,
  classification and review limits still apply. Cross-period owned transfers remain
  unavailable until their allocation evidence can be established.
- Existing unsupported OCR/institution/loan/mortgage/credit-line adapters were not
  added. Explicitly verified balances and required payments support current measures;
  unknown planning terms retain existing unavailable-result warnings.
- The assessment is a live projection. Historical reports and saved plans are
  preserved; it neither silently writes proposed reductions nor adds a health score
  to previously issued reports. Counselors use the existing reviewed scenario and
  reporting workflows to commit agreed planning decisions.
- Native acceptance was on Linux. Windows installation/offline distribution and
  commercial readiness require separate verification. No installer, household
  linking or business counseling functionality was introduced.

Next action: counselor evaluation of this completed dashboard and its policy using
fictional records. Broader adapters and Windows acceptance remain existing separately
scoped priorities; create no installer unless explicitly requested.
