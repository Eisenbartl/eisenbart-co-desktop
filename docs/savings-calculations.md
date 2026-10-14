# Savings planning — 0.4.0 source milestone

The first savings milestone adds a persistent client plan for emergency reserves,
sinking funds, alternative contributions, and budget/debt allocation review.
This is a development feature for fictional records. No new Windows installer
was requested or produced; the earlier 0.3.0 download does not include savings.

## Savings buckets and emergency coverage

**Total accessible savings before earmarks** is the entire liquid pool available
to the household, before setting aside bills and sinking funds. **Restricted
savings** is separate, unavailable money and is never added to that pool.
**Reserved for upcoming bills** and each sinking fund's **Already reserved**
amount are portions of the liquid pool, not extra accounts of money.

Available emergency money = liquid savings − bill reserves − all sinking-fund
reserves. Known earmarks exceeding the pool are rejected, including a known
partial sum that already exceeds it. Unknown reserves block complete emergency
coverage; they are never silently treated as zero. A known zero must be entered
as `0`. Restricted savings can remain unknown without being counted as available.
Excess reserves above a goal's target remain earmarked until the counselor changes
them; the application does not automatically release someone else's funds.

The emergency expense base comes only from the saved client's necessary-expense
budget lines. All necessary amounts must be known, and the sum positive. It never
substitutes gross income, take-home income, or discretionary spending. Estimated
budget values retain review warnings. Missing income can still permit a target
from known necessary expenses, while preventing complete affordability checks.

| Result | Formula |
|---|---|
| Emergency target | Necessary monthly expenses × selected whole months |
| Current coverage | Available emergency money ÷ necessary expenses, floored to 2 decimal places |
| Remaining to target | max(target − emergency money, 0) |
| Contribution months needed | ceil(remaining cents ÷ monthly contribution cents) |
| Estimated completion | End of the Nth contribution month, beginning with the selected first month |

Choose 1, 3, 6, or a custom 1–60 whole-month target. The initial 3-month selection
is editable and is not a financial recommendation. A zero or missing expense base
has no computed emergency target/coverage. A positive remaining amount with a
known zero contribution has no payoff/completion date. Already-funded targets
need zero further contributions and have no invented historical completion date.

## Sinking funds and deadline rounding

Each goal has a custom name, target, target date, currently earmarked amount, and
selected monthly contribution. Common examples are repairs, annual bills,
insurance, medical needs, holidays, and maintenance. Save incomplete goals when
terms are unknown; results show which calculations are unavailable.

Money uses exact integer cents. Goal dates use strict calendar-valid
`YYYY-MM-DD`. Contributions arrive at **month end** in this model. Count only the
month ends on or before the goal's deadline. A bill due before the last day cannot
use that month's contribution. Suggested contribution = remaining ÷ eligible
months, **rounded upward to the cent** so the target is met. Unknown dates or no
remaining contribution dates do not produce an invented suggested payment.

A fully funded current reserve does not prove that a deadline before the plan
start was met; historic deadline verification is unavailable. Insufficient
selected contributions produce a deadline warning even when they would eventually
fund the goal later. This is a monthly savings model, not a dated bill-payment
or bank-interest model.

All contributions are capped at their remaining goals. The final transfer can be
smaller than the entered monthly amount. Transfers stop once a goal is funded,
and unused money is not silently reassigned. No interest/returns, withdrawals,
inflation, expense changes, or bank movements are assumed. The application never
makes payments or transfers.

## Scenarios and coordination

Compare up to five alternative emergency contributions against the same liquid
pool, earmarks, goals, and debt commitments. Scenarios do not replace the selected
contribution or rewrite the client's budget/debt plan. Their funding check is
**unavailable**, rather than successful, when inputs do not permit verification.

Allocation rows calculate saved budget income minus necessary/discretionary
spending, minus modeled savings transfers, minus reserved debt payments. Initial
reserves are not counted as new spending. The debt reserve uses the **higher** of
the budget's debt allocation and the saved debt plan's applicable base payment
plus that month's extra payments. Future debt changes take effect in their month.
The model does not automatically free funds after estimated debt payoff or when
separate plans disagree. It continues far enough to check relevant future debt
events even when savings have already been funded.

A missing debt payment amount prevents a complete funding check. Missing budget
cash amounts do likewise. Debt shortfall/growth/horizon warnings remain visible
for review. Entered savings totals differing from the budget's savings allocation
are flagged; neither plan is automatically changed. Negative monthly remainders
flag overallocated funds. These checks use entered monthly averages and do not
prove cash is available on an actual due date.

Use **Funding priorities and notes** to record the counselor/client's priorities,
then choose contributions and debt allocations accordingly. This version does not
redistribute funding priorities automatically. Reconcile annual bills already
included as budget expenses so the same need is not counted again through a
sinking-fund allocation. Richer bill linking and dated cash flow are future work.

**Refresh budget and debt references** rechecks current references while retaining
unsaved savings edits. A concurrently changed savings revision requires reopening
the client and is never silently adopted. Saving also rechecks current references
within the authenticated transaction.

## Independently checked example

Start contributions in November 2026. Liquid savings are 2,600.00 USD; 600.00 is
reserved for bills and 1,000.00 for an insurance goal. Restricted savings of
5,000.00 remain separate. Necessary monthly expenses are 1,633.33. Choose a
3-month emergency target, 200.00/month emergency contribution, and an insurance
target of 1,200.00 due February 28, 2027 with 50.00/month contributions.

| Result | Expected |
|---|---|
| Available emergency money | 1,000.00 USD |
| Current coverage | 0.61 months |
| Emergency target / remaining | 4,899.99 / 3,899.99 USD |
| Baseline emergency completion | 20 months, June 30, 2028; final transfer 99.99 USD |
| Insurance contribution / completion | 50.00/month; February 28, 2027 |
| Insurance if due February 15 instead | 3 eligible month ends; suggested 66.67/month; selected 50.00 misses the deadline |
| Emergency alternative 100.00/month | 39 months; January 31, 2030 |
| Emergency alternative 300.00/month | 13 months; November 30, 2027 |
| Emergency alternative 1,500.00/month | 3 months; flagged overallocated in the example budget |

With budget surplus 1,416.67, budget debt allocation 400.00, and debt plan base
100.00, reserve 400.00 for debt. The first savings transfers total 250.00,
leaving 766.67. Adding a 1,500.00 debt extra payment in December reserves 1,600.00
then and leaves −433.33, with a warning. Python Decimal independently checks the
rounding, remaining amounts, required months, and capped final transfers.

## Validation, storage, and limits

Per entered amount: nonnegative USD with at most two decimals, up to $1 billion.
At most 50 goals, five scenarios, 120 characters per name, and 1,000 characters
for priority notes. Plan dates have years 1900–2100; goal deadlines cannot extend
beyond the 600-month projection horizon. Already-past dates are retained with
warnings. Long/unfunded goals receive no invented completion date beyond the
horizon. Aggregate emergency targets can exceed the per-input limit; integer
bounds and 128-bit intermediate coverage arithmetic preserve exact results.

Only validated inputs are stored in encrypted schema 5, not forecasts pretending
to be historical facts. Rust enforces firm/counselor/client scope. Revision checks
reject stale windows; savings and content-free audit writes are atomic. Failed
loads/writes preserve prior records. The upgrade preserves clients, notes, budgets,
debts, and prior audit entries. Older applications cannot open schema 5.

Locking clears private savings state and delayed responses cannot populate
another client or session. Save regularly; automatic lock clears unfinished edits.
Backup/restore, password recovery, clean Windows 11 installed/offline savings
acceptance, and independent security review remain incomplete.
