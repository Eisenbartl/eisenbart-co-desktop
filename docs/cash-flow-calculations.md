# Dated cash-flow model

The Rust `cashflow.rs` module is authoritative. React collects input and displays
native results. Cash-flow forecasts are independent planning inputs: no statement
facts, budget, debt, or savings plan is changed by forecasting or previewing.

## Money, accounts, and completeness

Amounts are exact USD integer cents; no floating-point arithmetic or rounding is
used. An input accepts at most two decimal places and at most 1,000,000,000 USD.
Opening balances can be negative. Activity amounts are nonnegative magnitudes;
activity type determines direction. Zero is known, blank is unknown. Unknown
opening balances persist; an unknown activity amount makes its affected account
balances unknown afterward. Other accounts continue projecting independently.
A combined balance is available only when every modeled account balance is known.

Enter each account's balance immediately before the selected first day's activity.
There is no automatic link between these opening balances and the savings module's
reserve buckets, which would risk counting the same funds twice. Accounts have
local identifiers scoped within this client's cash-flow plan. A forecast includes
only entered accounts and activity; absence of a warning does not prove completeness.

Paychecks and other inflows add funds. Bills, loan payments, external savings
transfers, and other payments remove funds. An internal transfer subtracts from
one account and adds the identical amount to a different modeled account in a
single event. It neither increases household income nor reduces combined cash.
A destination is required for an internal transfer and forbidden for other activity.
Do not enter both an external savings payment and an internal transfer for the
same transaction. Refunds/other inflows do not count as paychecks for payday review.

## Entered dates and recurrence

All activity starts from an entered valid date (1900–2100). New activities have
blank dates and unknown amounts. Default period dates define a viewing period,
not manufactured transaction dates. The inclusive forecast is at most 366 days.

| Schedule | Expansion |
|---|---|
| One time / irregular | Only the entered date; add additional entries for irregular pay |
| Weekly | Every 7 calendar days from the entered first date |
| Every two weeks | Every 14 calendar days from that date, not two times per month |
| Monthly | First date's day in each month; clamp to the last day of shorter months and return to the original day in later months |
| Twice each month | First date's day and a different entered second day (1–31), with month-end clamping; never emit two events when both days clamp to the same date |

No event occurs before the first date. An optional last date is inclusive and
cannot precede the first date. An anchor can precede the forecast; expansion keeps
its original cadence. There are no weekend, holiday, payroll, or bank-processing
adjustments. Monthly day 31 must originate from a date with day 31; entering a
February 28 date anchors day 28. Preview and inspect generated dates before saving.

Confirmed and estimated schedules are visibly distinguished, both included in
projections. Confirmed describes the entered schedule; it does not assert a
transaction has posted or that bank funds are available.

## Running balances and review

Entries sort by date, then debits before credits, then stable input order. An
internal transfer is a debit-class paired event. Conservative debit-first order
can flag a same-day shortfall even if a later deposit restores a positive closing
balance. No time-of-day or bank settlement guarantee is inferred. Account balances
are calculated before the combined financial view, so savings in another account
cannot silently cover an overdrawn checking account.

The ledger shows the affected source/destination balances and combined balance
after each event. The calendar shows each day's closing account/combined balances,
activity, certainty, and warnings. Negative opening balances, negative intermediate
balances, and negative daily closes are flagged. An unknown balance cannot establish
a shortfall or its absence. Three or more external outflows from one account on a
day trigger a concentration review (a heuristic, not a financial/legal threshold).

Bills and loan payments show the next entered paycheck for the same account within
the selected period, including a later same-day credit under conservative ordering.
If none exists, funding is explicitly unverified. A payday in another account,
future pay beyond the selected period, or an unknown paycheck amount cannot be
assumed to fund a bill. Date-adjustment messages ask the counselor to verify payee
permission; they do not claim payment dates can legally or contractually move.
Edit a date and choose **Preview cash flow / refresh references** to examine an
alternative without saving. Saving the draft is a separate explicit action.

Budget/debt/savings revisions and reference amounts are refreshed on preview and
save. No dated payments are imported automatically, because those plans contain
monthly amounts, not verified due dates. References do not prove all obligations
were entered. Review future debt extras/payment changes and capped savings
contributions in their source plans and enter their actual intended dates once.

No overdraft fees, credit limits, interest, holds, bank synchronization, actual
transaction reconciliation, business-day shifting, or probabilistic income models
are implemented. Transfers settle immediately in this model; delayed transfers
need a future settlement model. Bounds: 20 accounts, 100 schedules, 5,000 generated
entries, 1,000 note characters, and 64 KiB saved JSON. Forecasts are estimates.

## Independently checked fictional examples

Opening checking 500.00; reserve account 1,000.00; rent 1,200.00 on November 3,
2026; pay 1,500.00 every two weeks from November 6; internal transfer 200.00 on
November 7. Period November 1–30.

| Event | Checking afterward | Reserve afterward | Combined |
|---|---:|---:|---:|
| Rent, November 3 | -700.00 | 1,000.00 | 300.00 |
| Pay, November 6 | 800.00 | 1,000.00 | 1,800.00 |
| Internal transfer, November 7 | 600.00 | 1,200.00 | 1,800.00 |
| Pay, November 20 | 2,100.00 | 1,200.00 | 3,300.00 |

The month has positive net activity while rent still causes a checking-account
shortfall. Python Decimal independently checked all balances and conservation.
Previewing rent on November 8 removes this fixture's shortfall while retaining
the same month-end totals; this does not verify that the payee permits that date.
The original saved date remains November 3 until a changed plan is saved.

Monthly pay of 0.01 anchored January 31, 2028 occurs January 31, February 29,
March 31. Semimonthly days 31 and 30 from January 31 produce January 31,
February 29 once, March 30, March 31: four events totaling 0.04. Known opening
500 with a 600 same-day bill and 2,000 deposit has a conservative intermediate
balance -100 and closes at 1,900. All examples have asserted native Rust tests.

## Storage and UI boundaries

Schema 6 stores inputs per authenticated client/organization/counselor with
optimistic revisions and an atomic audit event. Competing saves/previews reject a
stale revision. Preview does not write inputs, revisions, or audit events. Plan
reads rederive forecasts without changing inputs. Migration failures roll back
new tables and audit changes; previous records and immutable audit protections
are preserved. Older applications cannot open schema 6.

The interface hides forecasts after edits until a new preview or save succeeds.
Preview preserves the unsaved draft and saved baseline. Failed loads cannot expose
an empty replacement form. Load/write/preview replies from an earlier client or
unmounted locked workspace are discarded. Cash-flow dirty and busy states share
the client navigation and save guards with notes/budget/debt/savings/profile forms.
