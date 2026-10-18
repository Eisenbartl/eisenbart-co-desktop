> Current 0.52.0 extensions: [financial planning subpages](financial-planning-subpages.md).
> The milestone below describes the original unextended model and retained historical behavior.
> Explicit optional funding, advanced debt, targeted payment and settlement assumptions use the existing engines with v2 capture.

# Extra debt payments and savings-goal contribution proposals

Version 0.21.0 extends the combined scenario workflow with two explicit controls:

- Add dated extra payments to the debt model’s shared repayment budget. Both
  strategies apply that budget using their existing allocation rules; this is not
  an instruction to send a payment to a named account.
- Change the monthly contribution to an existing sinking-fund goal. Target amount,
  target date, opening reserve and other goals stay unchanged.

These proposals can be compared temporarily, saved as immutable scenarios and
selected in internal counselor reports. No live plan, approved transaction,
balance, agreement or dated cash-flow entry is changed. Funding sources are not
inferred. New recurring expenses, temporary income shocks, new goals and targeted
account-specific extra payments remain future controls.

## Input and calculations

React sends `debtExtras: [{month, amount}]` and
`goalContributions: [{index, amount}]` alongside the existing expected baseline
versions. It sends strings, not calculated outcomes. Rust reads all four scoped
models, validates their canonical baseline and versions, applies the proposal and
recalculates through the unchanged debt/savings/cash-flow calculators.

Extras are additions to existing debt events. The existing debt rules allow one
extra per month, at most 24 extras in total, nonnegative exact-cent amounts and
months inside the debt plan’s 600-month horizon. A conflicting month, including
one in the saved baseline, is rejected rather than silently combined or replaced.
Future base payment changes remain effective. The interface displays saved extras
and lets the counselor remove only the extra rows in the current proposal.

Goal contributions use indices in the saved baseline, at most 50 unique changes.
Negative, excessive-precision or out-of-range inputs are rejected by the
existing model validation. A blank goal contribution explicitly means unknown;
zero means no contribution. Actual contributions stop at the remaining target,
while requested recurring allocations and unknowns retain their original model
meaning. Forecasts assume no yield and month-end contributions.

The savings model reserves the higher applicable budget/model debt commitment,
including added extras. It conservatively retains model commitments independently
of the projected debt payoff; an early payoff does not automatically release
funding. Dated cash-flow schedules remain independent and must be edited explicitly.
Warnings and unavailable forecasts stay visible. No financing recommendation is
automatically accepted.

## Independent examples

With monthly income 3,250.00, spending 1,200.00, base debt payment 100.00,
emergency contribution 200.00 and goal contribution 50.00, the first-month buffer
is 1,700.00. A 150.00 November extra and 100.00 goal contribution make it
`3250 - 1200 - 250 - 300 = 1500.00`. A zero-interest 1,000.00 debt with an existing
25.00 December extra is repaid with November 250.00, December 125.00, January–June
100.00 and July 25.00. The baseline finishes in August. A goal with target 1,200.00
and existing reserve 1,000.00 finishes in February with 50.00 per month or December
with 100.00 per month. Reserves and targets never change.

The complete native fixture also retains its separate 400.00 budget debt
allocation. A proposed 200.00 base payment plus 300.00 November extra reserves
500.00 that month. With the prior income/spending/emergency assumptions and a
100.00 goal contribution, its original 983.34 proposal buffer becomes 833.34,
debt interest becomes 12.20, payoff is December 2026 and goal completion is
December 31, 2026. Approved report operating activity remains separately 1,820.02;
dated cash-flow closing remains 3,300.00. These are synthetic planning examples.

## Historical compatibility and reporting

Database schema 16, saved payload format 1, `combined-preview-v1`,
`combined-scenario-v1`, `entered-plans-v1` and report formats remain compatible.
The two new optional arrays default empty and are omitted when empty. Existing
input field order, calculations and the six original comparison warnings remain
unchanged for scenarios without these controls. Extended proposals append a
specific commitment/funding warning. Existing record JSON/digests are never
rewritten. Reopening/recovery recomputes only from each record’s captured baseline.

The comparison includes each goal’s monthly contribution, completion date,
forecast status and target applicability. Canonical changes distinguish newly
added extras from retained baseline events. The selected report PDF names added
payments and changed goal contributions, then includes the complete proposed
terms, milestones and warnings in its existing planning section.

Atomic audit/count/version guards, client/organization ownership, report links,
payload limits, encryption and recovery validation remain in force. Edits clear
the temporary result before another save; failed inputs stay available for
correction. Client changes, locks, parent edits and explicit baseline reloads
discard temporary controls. Financial mutations are never automatically retried.

See [development-status.md](development-status.md) for actual validation results
and [remaining-requirements.md](remaining-requirements.md) for the open roadmap.
