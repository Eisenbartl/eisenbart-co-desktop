# Verified classifications and dated balance facts — 0.12.0

Open a saved client and expand **Verified classifications and dated balance
facts** in the statement review panel. These are explicitly counselor-verified
facts. They are independent of hypothetical budget/debt/savings/cash-flow plans.
A statement arithmetic match never supplies a verified balance automatically.

## Source-bound classifications

For each currently Approved expense, fee or negative interest transaction, choose
Necessary or Discretionary. For Approved Income, choose Verified take-home pay
or Other income receipt. Positive interest remains a receipt, not payroll.
Unclassified amounts remain explicit. Refunds are shown separately and are not
silently allocated to either expense class.

A classification captures the exact client-scoped transaction approval ID.
Corrections, withdrawals or reapproval create new source decisions and make the
old classification inactive. Returning to the same date/amount does not reactivate
it. Use **Reverify prior classification** to explicitly confirm the corrected source,
choose a different classification, or withdraw it.
An unrelated balance edit can preserve an inactive binding, but cannot change its
role or silently rebind it to a new approval. Source changes never erase history.

## Dated balance inventory

Record up to 50 distinct accounts/assets at one common **as-of date**, with an
amount, type, verification evidence and optional imported source document.
Other reviewed local evidence can be described without importing a document.
Evidence is an explicit counselor attestation, not an automatic institution
confirmation or external lookup. Dates cannot be in the future (UTC). Amounts
are nonnegative USD; debt is a positive amount owed, including overdrafts.

Types: Cash/checking, Accessible unreserved savings, Reserved/restricted savings,
Other assets and Debt owed. Enter each balance once; labels must be unique ignoring
case. This checkpoint does not allocate partial reserves within an account;
classify partially reserved savings conservatively as reserved, or wait for more
precise account allocation support. Other assets may include an evidenced
valuation, making net worth an estimate. Unknown amounts stay blank; zero is a
known zero. Debt's required monthly payment is separate and may remain unknown.

Declare a complete inventory only after checking all assets and debts at that
date; omitted types then mean verified absence. Adding/removing a balance clears
this declaration. A reason and explicit facts/evidence confirmation are required
for each saved snapshot. Unknown amounts still prevent a complete calculation
even when the inventory declaration is checked. Removing all balances and clearing
the date/declaration withdraws current balance facts without deleting history.

## Dashboard interpretation

- Classified spending/payroll totals use only current approved source decisions
  inside the selected transaction period. Known classified totals are partial
  when unclassified amounts remain; they never stand in for all household data.
- Recorded cash, accessible/reserved savings and debt subtotals show their balance
  date. A group with unknown amounts remains unavailable; absent groups are
  unavailable unless a complete inventory explicitly declares no such balances.
- Estimated net worth = cash + accessible savings + reserved savings + other
  assets − debt, only with a complete inventory and no unknown amounts. It uses
  the balance snapshot date, even if that differs from the transaction period.
- A local SVG chart shows itemized recorded debt balances with exact text values.
- Monthly necessary spending is an estimate from classified gross necessary
  spending divided by selected calendar-month count, rounded half up to cents.
  Unknown expense classes or undecided/undated non-rejected records prevent it.
- Estimated savings coverage = accessible, unreserved savings / estimated monthly
  necessary spending. Cash and reserved savings are excluded. Zero/unknown
  denominators do not produce a ratio.
- Estimated debt-payment/take-home ratio = verified required debt payments /
  average classified take-home pay × 100. Other income/interest are excluded;
  unknown income classifications or unknown required payments prevent the ratio.
  This is **not** a lender's gross-income debt-to-income ratio.

Combined coverage/payment ratios require complete balance facts dated at the
transaction period end, no undecided/undated non-rejected records, and balance
facts no more than 90 days old. Ratios truncate to two decimals. Missing statement
coverage remains unverified, so these are estimates from the recorded evidence;
a complete balance declaration does not certify complete transaction coverage.
Changing transaction approvals updates classifications and estimates but does
not rewrite independently verified balances. Savings-goal progress and gross
income underwriting metrics remain future work.

## Persistence and safeguards

Schema 13 adds scoped append-only `financial_fact_reviews` and an audit event.
Each review stores the entire facts snapshot, source approval bindings, reason,
time and counselor. The latest snapshot supplies current facts. Historical
snapshots remain inspectable and cannot be updated/deleted through SQL. Existing
source fields, approvals, recurring reviews and planning inputs are preserved.

Writes use the existing optimistic import revision, immediate transaction,
authenticated organization/counselor/client scope and atomic audit. UI drafts join
navigation/lock/conflicting-operation guards and survive failed saves. Save
requires a reason and confirmation. No financial operation is automatically
retried. Limits: 500 classifications, 50 balances, 256 KiB/snapshot, 100 reviews and
8 MiB total snapshot JSON/client. Each amount uses the existing 1,000,000,000.00
input cap; derived totals may exceed it. Labels/evidence/reasons use character
limits and reject control characters. SQLCipher backup/recovery validates all
snapshots and historical approval/document bindings before activation.

## Independent examples

Cash 800.01 + accessible savings 2,400.03 + reserved savings 300.03 + other assets
1,000.04 − debt 500.05 = **4,000.06** estimated net worth. Accessible savings /
necessary spending 1,200.02 truncates to **1.99 months**. Required payment 100.01 /
take-home pay 2,000.01 × 100 truncates to **5.00%**.

The native fixture independently verifies 100.01 + 600.05 + 25.02 − 200.03 =
**525.05**; 600.05 / 300.02 gives **2.00 months** and 25.01 / 500.01 × 100 gives
**5.00%**. A one-cent expense correction invalidates necessity until explicit
re-verification. Rejecting that expense removes it and retains the inactive
classification/history through restart and both encrypted recovery paths.

## Validation checkpoint

Final implementation 96cd863 passed 73 interface tests, 108 Linux library plus
2 executable worker tests, formatting/Clippy and the compiled native workflow.
Windows source CI [38593850120](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38593850120)
passed 109 library plus 2 worker tests and all 73 interface tests. Installer,
installation-acceptance and upload steps were skipped. The native workflow
verified explicit facts, exact metrics, correction/re-verification, rejection
exclusion, full-process restart and both encrypted recovery paths with unchanged
history/bindings. Earlier failed dropdown re-verification runs and their safe
inactive result are documented in development-status.md. No financial mutation
was replayed. Schema 13 requires an application that supports that schema; older
executables reject newer workspaces. Keep encrypted backups before future upgrades.
