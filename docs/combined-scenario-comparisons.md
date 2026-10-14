# Combined scenario comparisons — 0.19.0

The client panel can compare an authenticated saved budget/debt/savings/cash-flow
baseline with one temporary proposal. Load the baseline explicitly, enter a label
and assumptions, then compare. This is an offline read-only preview. Inputs and
outcomes are not saved; leaving the client, locking, restarting, editing a live
plan or reloading the baseline clears temporary context. No plan, source decision,
audit entry, counselor assessment or client agreement is created or changed.
Persistent scenario selection and report capture are the next checkpoint.

## Supported proposal inputs

- Amount changes to existing budget lines, retaining their purpose and frequency.
  This supports recurring income increases/reductions, discretionary reductions
  and expense/allocation changes. Changed lines are estimates.
- Independently selected baseline/proposed snowball or avalanche references, or
  no debt projection; a change to the debt model's base monthly payment.
- A change to the primary emergency-savings monthly contribution. Saved goal
  contributions, targets and reserved funds stay in effect.
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

## Limits of this checkpoint

This does not save named alternative scenarios, select them for reports or client
agreements, add/remove budget lines, create extra-loan entries, change goal
contributions, implement temporary date-range income shocks, automatically
recommend an allocation, model daily/variable/promotional debt, infer missing
statement coverage or establish a commercial release. Future controls and saved
records must preserve the same separation and provenance. No migration, new
dependency or installer is introduced. See [development-status.md](development-status.md)
for the checks actually completed.
