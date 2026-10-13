# Client budget planning — 0.2.0 development milestone

This first slice of specification Stage 3 provides one editable USD spending
plan per client. Open a saved client and scroll to **Monthly budget**. It is a
planning worksheet, not a ledger, financial recommendation, or debt payoff
simulation. Use fictional records while recovery and security work remain open.

## Exact money and normalization

Rust owns validation and all calculations. Enter nonnegative decimal dollar
amounts with up to two decimal places, without symbols, commas, or exponential
notation. The backend converts them to integer cents; IPC and stored original
amounts use canonical decimal strings. React formats those strings without
floating-point arithmetic. A line is limited to $1 billion and a plan to 100
lines. Category/source names are customizable and limited to 120 characters.
Only USD is supported in this milestone.

| Entered frequency | Monthly equivalent |
|---|---|
| Weekly | Amount × 52 ÷ 12 |
| Every two weeks | Amount × 26 ÷ 12 |
| Twice a month | Amount × 2 |
| Monthly | Amount |
| Annual | Amount ÷ 12 |
| Irregular | Counselor-entered estimated monthly total; always marked estimated |

Normalize each line independently and round to the nearest cent, with an exact
half cent rounding upward. Add the rounded lines to form totals. For example,
$0.03 every two weeks becomes $0.065 per month, rounded to $0.07. Two such lines
total $0.14. Rounding the combined unrounded amounts would produce $0.13; that
is deliberately not this worksheet's convention. Original entered amounts and
frequencies remain available separately from monthly equivalents.

These are annualized monthly averages. A year with 53 weekly or 27 biweekly
paychecks is not automatically detected; the future paycheck calendar will model
actual dates. Monthly averages do not establish when cash is available for bills.
Irregular income requires an explicit monthly estimate, rather than guessing from
an unknown history.

## Totals and allocations

| Result | Formula |
|---|---|
| Take-home income | Sum of normalized income lines |
| Necessary/discretionary expenses | Separate sums of the respective expense lines |
| Total spending before allocations | Necessary + discretionary expenses |
| Surplus / deficit before allocations | Income − spending |
| Proposed savings | Sum of savings allocation lines |
| Capacity for debt payments after savings | Surplus − proposed savings |
| Planned debt payments | Sum of debt payment allocation lines |
| Unallocated / deficit after allocations | Capacity for debt payments − planned debt payments |

Negative results stay negative and appear as deficits. Debt capacity is a planning
amount, not a guarantee that a payment is affordable. Put debt payments in **Debt
payment allocation**; entering the same payment as an expense as well would
count it twice. Savings also belongs in allocations. Optional budgeting methods,
percentage targets, minimum-payment validation, interest, and repayment schedules
are future checkpoints.

Unknown amounts are blank/null and excluded from numeric totals, with an
incomplete-plan warning. A known zero is entered as `0`, contributes zero, and is
not missing. Estimates retain their marker; irregular known amounts are always
estimates. Plans without known income or necessary-expense amounts are flagged.
Other warnings identify excluded missing amounts, included estimates, and a
negative result after allocations. No automatic rule decides whether all of a
household's costs have been entered. Counselors must review completeness.

## Independently checked household example

Enter $1,500 take-home pay every two weeks, $1,200 monthly rent, $100 weekly
groceries, $200 monthly discretionary spending, $300 monthly savings, and $400
monthly debt payments:

| Result | Expected USD |
|---|---|
| Monthly income | 3,250.00 |
| Necessary expenses | 1,633.33 |
| Total spending | 1,833.33 |
| Surplus before allocations | 1,416.67 |
| Debt capacity after savings | 1,116.67 |
| Unallocated after planned debt payments | 716.67 |

The backend tests use these fixed expected results and additional independently
checked frequency, half-cent, zero, missing/estimated, deficit, malformed-input,
and maximum-size examples. The native desktop smoke uses this same household
through visible controls, encrypted saving, locking, separate clients, and full
process restart.

## Persistence and editing

**Save budget and calculate** validates and saves the entire plan atomically with
a content-free `budget_saved` audit event. A validation or audit-write failure
preserves the prior saved plan. Budget data remains inside the encrypted workspace
and follows the authenticated organization/counselor scope. No historical
transactions exist or are modified in this milestone.

Every save increments a revision. A stale window cannot overwrite a newer plan;
close and reopen that client to load the current version before trying again.
A failed load offers a retry, never an empty writable replacement. Unsaved edits
participate in the existing leave-record warning. Editing hides old totals until
saving recalculates them. Locking clears private budget state, and late responses
from another client or session do not restore it. Automatic locking still clears
unsaved edits, so save regularly.

Database schema 3 adds client budgets and expands the immutable audit event
constraint. Its transactional migration preserves clients, notes, and existing
audit entries; tests also exercise rollback of a failed migration. Version 0.1.0
cannot open schema 3. Do not switch back to the old application after upgrading.
There is still no supported backup/restore or password recovery.
