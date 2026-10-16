# Bank and card review

Version 0.35.0 adds Financial Review → Bank & card activity and an optional
**reviewed bank/card supplement** in saved counselor assessments. It is a working,
provisional internal review. It does not authorize final card-inclusive reports,
change bank dashboard figures or rewrite planning assumptions.

## Workflow

1. Import and individually approve signed card CSV rows in Documents → Credit-card
   statements. Approve bank transactions separately in Transactions.
2. Create durable card accounts in Bank & card activity. Assign each original
   statement explicitly; matching names never establish ownership or identity.
3. Match each approved card payment to its reviewed bank counterpart. The bank row
   must be an approved **balance movement**, with the same negative USD amount and
   a date within seven days. Review/reclassify it in Transactions first if needed.
   A linked bank transfer cannot be reused. One bank row can match one card row;
   split/partial payments and automatic suggestions are not implemented.
4. Choose complete calendar months and calculate. Bank cash movement, bank spending,
   card spending, card payments and liability movement are displayed separately.
5. In Reports → Working tools → Saved counselor assessments, select the same months,
   include the reviewed bank/card supplement, load it explicitly and acknowledge
   the basis. Unresolved issues may be saved honestly in an internal assessment.
   They never turn unavailable combined spending into zero.

Rename/archive/reopen accounts without changing their durable IDs or original
sources. Archived accounts retain financial evidence; reopen before assigning a
new statement. Unassigning a source removes its current repayment matches.
Removing a match, correcting a row or reapproving unchanged amounts preserves
history. A newer bank or card decision invalidates the original match permanently
until the counselor explicitly matches the current approvals again.

## Accounting rules and checked example

All arithmetic uses integer USD cents in Rust. There is no currency conversion.

| Measure | Independent example |
|---|---|
| Bank wages | 2000.01 |
| Bank rent spending | 1200.02 |
| Bank card repayment | 50.03 outflow, classified as balance movement |
| Bank net cash movement | 2000.01 − 1200.02 − 50.03 = **749.96** |
| Card charges/refund | 100.01 purchase − 20.02 refund + 5.04 fee + 2.05 interest |
| Net card spending | **87.08** |
| Card payment | **50.03**, excluded from spending and income |
| Change in card debt | 87.08 − 50.03 = **37.05** |
| Provisional combined observed spending | 1200.02 + 87.08 = **1287.10** |

Combined observed spending is unavailable if bank rows are undecided/undated,
card sources are unassigned, statement arithmetic fails, declared account periods
have gaps/overlaps, adjacent reported balances disagree, an approved card
adjustment needs spending treatment, or an in-period card payment lacks a current
in-period bank counterpart. Individual approved ledger subtotals remain visible
with their issues; overlapping card sources can duplicate that provisional card
subtotal and therefore always block the combined figure. Cross-period payments
remain explicit unresolved items; no guessed timing adjustment occurs.

These checks cannot establish a complete household inventory or prove that an
unmatched bank expense is not a repayment. Verify originals and classifications.
No card balance is added to manually verified debt/net worth: those facts may
already include the same card. No new amount is passed to budgeting, repayment,
savings, scenarios or client action-plan calculators.

## Evidence, bounds and compatibility

Schema 24 adds scoped append-only card-review snapshots and atomic audit writes.
At most 20 durable accounts, 20 assigned statements, 200 repayment pairs and 200
history versions are supported per client, with a 128 KiB snapshot limit.
Source/decision limits from [credit-card-review.md](credit-card-review.md) remain.
Each change requires an expected link version and shared financial-source revision,
review reason and fresh acknowledgement. A failed audit, stale save or invalid
counterpart rolls back the source revision and all new history.

Format-8 assessments retain the exact card sources, all their decisions, reviewed
account/source links, repayment decision IDs, calculated figures and limitations.
Original sources are checked against their scoped immutable records on read and
recovery; calculations are recomputed against the report's captured bank basis.
Older repayment bank approvals are retained when later approvals make them stale.
The internal PDF includes the supplement, source hashes and approval bindings.
The existing 8 MiB/report and 32 MiB/client report limits still apply; large
captures fail without altering existing records.

Optional JSON omission preserves formats 1–7 and their original bytes/digests.
Later changes flag the saved report without replacing its evidence. Historical
card snapshots are reconstructed at their captured source revision. Version-1
final policy adds a blocking gate only for the new card supplement; old-format
readiness receipts retain their existing meaning. Known cards continue to block
bank-only final approval. Client-plan rendering does not include the new internal
supplement, and a parent needing review cannot authorize a new plan.

Encrypted restore validates every card-link history version, scoped source and
original approval before activation. This is an application integrity check,
not an independent security audit or digital signature.

## Next dependencies

Implement a versioned card-inclusive household inventory/completeness policy,
reviewed balance/fact mappings and explicitly supported timing/exclusion cases
before enabling final approval. Preserve v1 bank policy and format-8 calculation
behavior for historical records. Add statement-account selection to future card
import adapters without mutating existing format-1 sources. Card PDFs, other debt
and paystub adapters, OCR, direct printing and installed Windows acceptance remain
separate work.

The next policy needs dated applicability for newly opened/closed cards, a recorded
household inventory and source-bound necessary/discretionary spending roles for
card approvals. Map each card's verified closing debt to the existing balance
inventory explicitly, preserving unknown/unmapped balances and reconciliation
differences. Do not add card debt a second time or infer spending roles from a
category name. Final approval must require all applicable bank/card evidence,
current review bindings, resolved exceptions and fresh professional confirmation.
Keep unsupported cross-period/split repayments blocked until their semantics are
defined and independently checked.


## Subsequent approval policy (0.37.0)

The original `bank-card-review-v1` calculator and provisional capture above remain
unchanged. A separately implemented [card-inclusive final policy](card-inclusive-final-approval.md)
can now approve complete supported format-9 evidence with saved bank and card
completeness, current repayment bindings and fresh professional decisions. Old
calculator disclosures remain historical; its supplement alone cannot authorize
approval. Unsupported cases remain blocked under the new policy.
