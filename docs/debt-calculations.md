> Current 0.52.0 extensions: [financial planning subpages](financial-planning-subpages.md).
> The milestone below describes the original unextended model and retained historical behavior.
> Explicit optional funding, advanced debt, targeted payment and settlement assumptions use the existing engines with v2 capture.

# Debt repayment planning — 0.3.0 development milestone

This second slice of specification Stage 3 saves a debt plan for each client and
compares monthly snowball and avalanche projections. Use fictional accounts.
Projections are planning estimates; they do not establish contractual payment
amounts or exact payoff dates. There is no connection to a lender or cloud service.

## Accounts and required knowledge

Record a debt nickname, type, outstanding balance, APR, monthly minimum, known
monthly fee, interest model, and optional source/terms notes. Credit card, auto,
student, personal, mortgage, other installment, and custom labels are supported.
**Terms checked by counselor** is a manual review marker, not independent source
verification or an imported-statement approval.

New amounts are blank/unknown, including APR and fees. Enter `0` for a known
zero, rather than leaving it blank. Incomplete records can be saved. A projection
requires a known total monthly payment budget and all active accounts to have
known balances, APRs, minimums, fees, and the supported model. It never ignores an
unknown debt in order to produce a deceptively complete comparison. Known-zero
balances need no further terms and accrue no interest or fees.

Select **Fixed monthly APR estimate** to use this milestone's model. Unknown,
daily, variable, and promotional interest models are stored but block comparison
for active accounts. Daily accrual, rate changes, introductory-rate expiration,
percentage-based/declining minimums, and nonmonthly contractual payments are not
modeled. Debt type alone does not select or establish a lender's interest method.
Due dates, account numbers, remaining contractual terms, and richer statement
provenance are future account-detail work; they are not invented here.

## Exact calculations and timing

Money uses integer cents. APR uses exact integer units of 0.0001 percentage point;
`12.3456` means 12.3456%, not a decimal rate. Enter nonnegative amounts with up to
two decimal places, and APR percentages with up to four decimal places. No binary
floating-point money or interest calculations occur in React or Rust.

For each modeled month, on each positive opening balance:

1. Calculate interest as opening cents × APR units ÷ 12,000,000.
2. Round this account's interest to the nearest cent, with exact half cents upward.
3. Add the entered monthly fee. A paid-off balance incurs no further fees.
4. Reserve each account's entered fixed monthly minimum, capped at its complete
   amount owed (opening balance + interest + fee).
5. If the total payment budget cannot cover these capped minimums, stop before
   recording an unaffordable month. Do not assign a debt-free month.
6. Allocate the remaining money according to the selected strategy. Cap every
   payment at that account's amount owed and roll any excess to the next account
   in the same month. Payments are made at month end in this model.
7. Carry each closing balance forward, including unpaid interest/fees. Flag any
   account whose balance grows in one or more simulated months.

The monthly fee is added after interest; it becomes part of the following month's
opening balance if unpaid. The total monthly payment budget stays constant unless
an explicit change is entered, so freed minimum payments remain available to the
other debts. Unused money after all debts are paid is not charged or carried into
new borrowing. No future purchases or additional borrowing are assumed.

**Snowball** orders active accounts by their current opening balances ascending,
then APR descending, then saved entry order. **Avalanche** orders APR descending,
then current opening balances ascending, then saved entry order. Priorities are
recomputed each month. All initial balances, rates, fees, minimums, budgets,
changes, and extra payments are identical in the comparison. Fees and account
terms can affect which strategy costs less; the interface makes no blanket claim
that one strategy always wins.

## Payment events, limits, and incomplete projections

An extra payment adds a one-time amount to the pool in its specified month.
A monthly payment change replaces the total base budget from that month onward.
Both can occur in the same month. Combine same-month events of the same type into
one entry; duplicates are rejected. Future events after payoff are unused.

The first planning month and event months use `YYYY-MM`, with years 1900–2100.
Events must fall within the 600-month horizon beginning at the chosen first month.
There are at most 50 accounts, 24 extras, and 24 payment changes. Each entered
money amount is at most $1 billion; APR is at most 1000%. Names are limited to
120 characters, notes to 500, and stored plan JSON to 128 KiB. Internally, opening
balances are capped at 1,000,000,000,000,000 cents; growing balances reaching the
model limit stop the projection safely. Bounded inputs and horizon keep all sums
within integer range; intermediate interest multiplication uses 128-bit integers.

A shortfall, the horizon, or the model limit produces a partial result and clear
warnings, with no projected global debt-free month. Its interest/fee/payment
totals and remaining balance describe only completed simulated months, not an
entire payoff. An individual account paid during those months may retain its
estimated payoff month. All-zero starting balances produce no schedule or newly
invented payoff date. Unverified terms remain marked uncertain even when known
numeric values permit a simulation. All reported payoff months are estimates.

## Coordination with the client budget

Rust loads the latest saved client budget within the authenticated workspace.
The reference shows its revision, monthly capacity after savings, and planned
debt payments. The total debt payment is entered explicitly and never silently
updates the budget. Plans flag missing/estimated budget information and proposed
payments above its current monthly capacity, including changes and extra-payment
months. A different initial payment from the budget's debt allocation is also
flagged for reconciliation. Extra payments require separately verified funding.

The reference in the editor is the snapshot checked when the debt plan was loaded
or saved. **Save debt plan and compare** checks the latest saved budget again.
Saving a budget afterward does not automatically replace a debt draft or recheck
its on-screen reference. Monthly budget averages do not prove that funds will be
available on a particular bill date. Future dated cash-flow and savings modules
will add that coordination.

## Independently checked examples

| Fixture | Expected result |
|---|---|
| $1,000 balance, 12% APR, $100 minimum/total monthly payment, zero fee, first month 2026-11 | First interest $10.00 and closing balance $910.00; second closing balance $819.10; 11 months; final payment $58.98; total interest $58.98; total paid $1,058.98; estimated final month 2027-09 |
| $100 at 0% with $10 minimum, plus $500 at 24% with $20 minimum; $100 total monthly payment; zero fees; first month 2026-11 | Snowball first payments $80/$20, total interest $43.10, total paid $643.10; avalanche first payments $10/$90, total interest $35.35, total paid $635.35; both finish in month 7 (2027-05) |
| $10 and $15 at 0%, minimums $1 each; first debt monthly fee $1; base $20 plus one-time $5 in 2026-11; new base $2 in 2026-12 | First payments $11/$14; next payment $0/$1; total fees $1; total paid $26; fee stops on the paid first account |
| First two-debt fixture with $29.99 monthly budget | Required first minimums $30; no affordable schedule, no payoff month |
| $0.50 balance at 12%, sufficient payment budget | First monthly interest is half a cent, rounded to $0.01 |

Fixed expected values were independently checked with Python Decimal and direct
zero-interest accounting. Rust tests also cover missing/unmodeled terms, tied
priorities, negative amortization, horizon/model limits, malformed dates/APRs,
duplicate events, large inputs, and budget capacity. The native desktop smoke
enters the two-debt fixture through visible controls, checks both strategies and
account allocations, then locks/restarts to verify encrypted persistence and
client isolation.

## Local storage and version changes

Inputs live in encrypted schema 4; calculated schedules are recomputed rather
than stored as purported historical facts. Firm/counselor scope is enforced in
Rust. Revision checks reject stale windows. Plan writes and content-free audit
events are atomic; failure preserves the prior plan. Failed loads cannot become
empty editable replacements. Locking clears private UI data and old asynchronous
responses cannot populate another client. Unsaved edits participate in leave
warnings; save regularly because automatic locking clears them.

The schema upgrade preserves clients, notes, budgets, and earlier audit history.
Older 0.1.0/0.2.0 applications cannot open schema 4 afterward. Keep using the newer
version and preserve the data folder. Backup/restore, password recovery, clean
Windows 11 offline acceptance, and independent security review remain unfinished.
