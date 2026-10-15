# Combined financial scenarios — 0.22.0

The client panel compares authenticated saved budget/debt/savings/cash-flow
models with hypothetical proposed inputs. Temporary comparisons are read-only;
leaving the client, locking, restarting, editing a live plan or reloading the
baseline clears their inputs/results. Explicitly saving a reviewed proposal
creates an immutable scenario, with original assumptions and both model sets.
Selecting it in a counselor report captures the proposed models and original
comparison for offline PDF export. Later baseline revisions flag history without
rewriting it. See [saved-combined-scenarios.md](saved-combined-scenarios.md).

## Supported proposal inputs

- Amount changes to existing budget lines, retaining their purpose and frequency.
  This supports recurring income increases/reductions, discretionary reductions
  and expense/allocation changes. Changed lines are estimates.
- Independently selected baseline/proposed snowball or avalanche references, or
  no debt projection; a change to the debt model's base monthly payment.
- A change to the primary emergency-savings or an existing goal’s monthly
  contribution. Goal targets, dates and reserved funds stay in effect.
- Additive dated extras to the shared debt repayment budget; existing debt
  events remain effective. See [scenario-payment-goal-controls.md](scenario-payment-goal-controls.md).
- New recurring necessary/discretionary expenses, optionally including explicitly
  selected estimated bill schedules in the proposed cash-flow model.
- Explicit amount/first-date changes to existing dated cash-flow schedules.
  Account bindings, frequency, second payday, end dates and forecast period stay
  in effect. Changed entries are estimates; paired transfers remain paired.

Blank changed amounts mean unknown, not zero. Zero is explicit. Missing optional
models remain unavailable and cannot receive edits. Existing debt payment changes
and extras remain in effect; a base payment proposal never replaces a later dated
payment change. Savings reserve checks retain these commitments, including after
a projected debt payoff. Budgets, model allocations and dated schedules are
independent entered assumptions: debt/savings changes do not create dated payments,
change cash-flow openings or synchronize budget allocation lines.

## Results and validity

`combined-preview-v1` returns both canonical model sets, all four baseline
versions, input assumptions/changes, creation time and limitations. The interface
shows income/spending/budget unallocated, selected debt cost/payoff/status,
emergency target/contribution/completion/status, total requested savings,
first savings-forecast-month allocation buffer, dated closing and warning-day
counts side by side. It lists canonical proposed changes and retained assumptions
with both sets of model warnings. Times display in America/Chicago; UTC is stored
in the temporary response. Unavailable/unsupported outcomes are labeled explicitly.

Rust reads all four saved models in a scoped deferred transaction, validates the
baseline's canonical calculations and revisions, bounds and validates indexed
changes, then recalculates budget → debt → savings → cash-flow context using the
existing independently tested integer-cent calculators. Each budget line is
normalized/rounded half-up before totaling. Debt interest remains the supported
fixed monthly APR/12 estimate. Savings has no yield. Dated debits precede credits
on the same day; transfers preserve combined funds. There is no floating-point
money arithmetic. The proposed set is revalidated as a complete model set.

IPC accepts only versions, text, strategies and changed strings/indices/dates,
never frontend financial outcomes or caller-supplied model sets. Duplicate and
out-of-range indices, invalid precision/dates, stale revisions, oversized inputs,
wrong clients and locked access fail without writes. Canonical model captures
retain their existing 4 MiB safety limit. The interface rejects late/wrong-client,
wrong-calculator or wrong-version results. Competing writes and unfinished edits
block comparison; edits hide old results and failed proposal inputs remain for
correction. Reload only reads the baseline.

## Independently expected examples

Backend fixture: income 1,500 biweekly = 3,250.00; necessary spending 1,000.00,
discretionary 200.00. Proposed income 1,600 biweekly = 3,466.67 and discretionary
100.00 leave 2,366.67 before allocations. Base debt 200.00 plus emergency 300.00
and retained goal 50.00 leave first-month buffer 1,816.67. With 100.00 at 0% and
500.00 at 24%, proposed snowball interest is 23.01 with February 2027 payoff;
baseline avalanche at 100.00 costs 35.35. Emergency target 3,000.00 minus
1,000.00 accessible requires seven 300.00 contributions through May 31, 2027.

Native fixture uses 1,633.33 necessary spending. The same income/discretionary
proposal yields 1,733.34 before allocations, then the higher budget debt reserve 400.00 (versus proposed model payment
200.00) and savings 350.00 leave 983.34. Emergency target remains 4,899.99 and completion advances to
November 30, 2027. The saved 3,250.00 income, debt 100.00, savings 200.00 and
original bill date remain unchanged. Delaying the November 3 bill until November 8
explicitly removes the shortfall while combined cash-flow closing stays 3,300.00.

A separate retained-commitment check keeps a November extra of 50.00 and a December
payment change of 2,000.00: a proposed 200.00 base payment reserves 250.00 in
November and 2,000.00 in December; the later deficit stays visible.

## New recurring expenses

Each expense has a name, necessary/discretionary purpose, optional known amount
and weekly/biweekly/semimonthly/monthly/annual frequency. Up to 20 additions are
supported, within the existing 100-line budget limit. Rust marks them estimated,
normalizes each line half-up to cents and recalculates debt capacity, emergency
targets and savings affordability. Blank means unknown; zero is known zero.

Optional dated details require a saved cash-flow model, an explicitly selected
owned account and real first date. An optional last date and semimonthly second
day use the existing calendar rules; the same amount/frequency supplies an
estimated Bill, with no invented transfer or balance change. Annual expenses are
budget-only because annual calendar recurrence is not implemented. Existing and
new schedules together retain the 100-schedule limit. A selected new schedule
with no occurrence in the forecast adds a visible warning.

Payment dates bound only the dated forecast. The normalized recurring budget
assumption applies throughout the other models, including emergency targets.
These are additions: existing bills remain effective, and counselors must check
for duplicate obligations. No live plan or approved transaction is written.
The optional `recurringExpenses` array defaults empty and is omitted when empty;
schema 16 and existing calculator/payload formats stay unchanged. Earlier captured
input order and warnings stay unchanged without new expenses. Reads/recovery
validate new expense assumptions and generated bills against the original capture.

Independent example: 12.01 weekly normalizes to 52.04 monthly, while four November
payments total 48.04. Necessary spending 1,000.00 becomes 1,052.04; a three-month
emergency target becomes 3,156.12; a 1,700.00 buffer becomes 1,647.96. A separately
entered dated closing of 3,300.00 becomes 3,251.96 only when those bills are
explicitly selected. Unaffected accounts remain independently known.

The integrated native proposal also retains the earlier income, debt extra and
goal controls. Adding 75.00 monthly necessary spending changes its 833.34 buffer
to 758.34, emergency target to 5,124.99 and completion to December 31, 2027. An
explicit November 15 bill changes dated closing to 3,225.00. Debt interest stays
12.20 and goal completion December 31, 2026. Approved operating activity remains
separately 1,820.02. Saved reports name the new expense, classification, frequency,
account and date assumptions; the same facts survive restart and both recovery
paths. [Development status](development-status.md) records actual checks.

## Remaining scope

Temporary date-range income shocks, new-goal proposals, account-targeted extra
payments, daily/variable/promotional debt terms and broader original reporting
requirements remain open. This is a development build. No new dependency,
migration or installer is introduced in this checkpoint.
