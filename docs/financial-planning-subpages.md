# Financial planning subpages — 0.52.0 development source

These subpages extend the existing client budget, debt, savings, cash-flow and
scenario services. They do not execute payments, change approved transactions,
replace an agreed plan on preview, or merge different clients' funds.

## Paycheck allocation

Budget Planner → Paycheck allocation reads the saved monthly budget and the saved
calendar in one scoped snapshot. It earmarks each non-income priority equally
across all entered paycheck occurrences in each **complete calendar month**.
Each line's existing frequency normalization is reused. Integer cents divide by
the number of paychecks; the earliest occurrences receive remainder cents. Thus
1,000.01 over two paychecks is 500.01 and 500.00, with no lost cent. Weekly,
biweekly, twice-monthly, monthly and irregular dates come from the calendar;
monthly averages never manufacture dates.

Partial months, unknown amounts, estimated pay and insufficient receipts stay
visible. Dated commitments until the next paycheck in the same account are a
separate timing comparison, never added again to budget earmarks. Opening cash
and other deposits are not paycheck income. These equal earmarks are read-only
suggestions: edit the existing saved budget priorities and dated schedules,
then refresh. No second allocation ledger or automatic payment schedule is made.

## Transfer settlement and reserves

The existing cash-flow editor has an optional reserve floor per account and an
optional availability date per internal transfer. Unextended records retain the
original immediate paired-transfer calculation. Explicit delayed transfers have
a departure debit and a destination receipt credit. In-transit money is excluded
from combined settled cash until receipt. Incoming/outgoing account shortages
and reserve-floor violations are checked independently; another account's money
does not automatically cover a deficit.

An entered first availability date supplies an explicit calendar-day lag for
recurrences (0–366 days). No bank/holiday/business-day adjustment is inferred.
Blank availability means unknown; affected forecasts remain unavailable. Receipts
from pre-period departures are included using opening account balances that
already exclude the departure. Same-day debits precede receipts. Transfers leaving
the period unsettled remain in transit. Reserves are portions of existing account
balances, not new money or additional expenses.

## Advanced debt terms

The existing debt engine supports explicit daily simple accrual on actual days
using an entered 360- or 365-day basis; known monthly APR changes, including
promotional reversion; and additional month-level fees. Unknown contractual terms
continue to withhold payoff comparisons. APR changes take effect at month start;
the last entered rate continues. Uncertain future rates are estimates, not lender
promises. Deferred/retroactive promotional interest is not modeled.

Optional weekly, biweekly, semimonthly or monthly payment dates split the **same
monthly payment budget and monthly minima** across that month's occurrences.
Earlier occurrences receive remainder cents. Daily accrual terms are required
for frequent/dated payments. Interest rounds half up per interval, capitalizes at
payments and month end, and accrues through month end after the last payment.
Fees apply once at the first monthly event while a balance remains. Extras go
to the last payment date in their month. Both strategies share the same terms;
avalanche uses that month's effective APR. The UI shows dated estimates and the
complete monthly account schedule. Lender-specific dynamic minimums, holidays,
new borrowing and deferred interest are not inferred.

Independent examples: 1,000 at 12%, actual/365, November 2026 gives 9.86 interest
and 909.86 after a 100 payment. February 2028 gives 9.53. At 36.5%, weekly November
7/14/21/28 payments totaling 400 give 25.32 interest and 625.32 month-end debt.
A 0% 1,000 balance paid 100 in November, reverting to 24% in December with a 20 fee,
closes those months at 900 and 838.

## Savings funding and drawdowns

Optional sources/events live inside the existing saved savings plan. Opening
liquid savings still partitions into bill, goal and emergency reserves. Separate
one-time sources must be explicitly named and funded; recurring income and money
already in opening savings must not be entered again. Source IDs are unique and
source availability depletes as used. Events select existing emergency/goal
buckets and real dates within the bounded projection. Transfers between buckets
conserve the pool. Restricted savings and bill reserves are not automatically
spent or released.

Events precede month-end contributions, including events on that same day. The
existing contribution schedule refills only the remaining target after each event.
Requests above available funds show applied amounts and explicit unfunded
shortfalls; they never create negative fictitious reserves. Unknown sources or
amounts invalidate affected balances/milestones. The existing affordability check
still reserves the higher budget/model debt commitment and monthly savings once.
External funding/drawdown events do not silently become income, expenses or dated
bank activity. Counselors reconcile actual spending/timing explicitly.

Dated balance paths update completion dates and goal-deadline status. Constant
contribution suggestions are withheld when event-driven funding changes their
meaning. Alternative contributions run through the same event-aware schedule.
The UI shows source balances, shortfalls and projected reserve paths separately
from verified savings history.

## Verified savings progress

Financial dashboard → Savings progress reads existing counselor-verified balance
reviews for an explicit 1–24 month period. It shows only real dated snapshots;
latest recorded corrections at a given balance date supersede that date's older
review. No missing dates are interpolated. Unknowns and partial inventories remain
visible, accessible and reserved/restricted balances stay distinct, and each point
retains its review ID, evidence and optional document link. A chart scale comes
from exact Rust amounts. Planning contributions never masquerade as observations.

## Expanded scenario controls

Financial Scenarios → New goals & funding sources uses the existing combined
comparison and immutable scenario service. New goals carry explicit proposal IDs
and zero opening reserves; any funding must be specified. Separate sources and
events extend only proposed savings. Account-targeted debt extras identify the
captured debt account/revision, apply after required minimums, and reserve their
entire commitment even when the account needs less. Unused targeted funds are
not sent to another lender. The same targeted-payment editor also works in the
saved debt plan, through its existing explicit Save action. Savings funding
uses the shared source/event editor. Existing calendar transfer availability can
be explicitly changed in a scenario; unknown dates remain unknown.

Comparisons show all newly added goals against **Not in saved baseline**, preserve
original plans and include source availability, exact affordability and milestones.
Immutable records can be selected in internal reports/PDFs and validated across
recovery. V2 captures preserve advanced assumptions; original v1 records retain
their calculations and bytes. Client action plans that express a single recurring
allocation cannot represent these dated reserve/targeted commitments: review and
explicitly save a separate compatible monthly agreement before creating one.
No preview or scenario save silently adopts any plan.

## Compatibility and acceptance

Schema 33 is a compatibility gate over existing plan JSON storage, not another
financial store. Old writers must not discard new assumptions. Optional fields
are omitted when absent so historical inputs/results and report/scenario digests
remain reproducible. Existing authenticated scope, optimistic revisions, atomic
audit, stale-reference checks and encrypted recovery remain authoritative.

The source milestone is in progress. Focused backend tests pass for settlement,
paycheck cents, advanced debt and savings source/depletion/refill/unknowns; legacy
calculation tests remain in place. Full source/native verification, expanded
scenario controls and final review evidence will be recorded in development
status before marking this assignment complete. No installer is authorized.
