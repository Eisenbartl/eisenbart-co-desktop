# Credit-card statement review — 0.48.0, schema 30

Import statements → Credit-card statements records signed USD CSV or selectable-text PDF evidence,
individual counselor decisions and statement arithmetic in encrypted storage.
This is a working liability review, separate from the existing bank cash engine.

## Use the page

1. Select the client and open Import statements → Credit-card statements → Import a card statement. Choose/inspect a UTF-8
   CSV, then explicitly map date, description, signed amount and optional category.
   Choose the source date format. For PDFs use the [transaction-focused inspection](pdf-import-review.md) with editable preselection and exact draft totals.
2. Enter the card name, statement dates and opening/closing balances owed. Positive
   balances mean debt; negative balances mean a credit held on the card. If the
   source uses negative charges and positive payments, explicitly invert its signs.
   Optional authorization retains the complete CSV inside workspace encryption.
3. Save the statement. CSV rows start Pending; PDF rows start Needs Clarification. All remain unclassified. Original source
   cells, line/record references, hash, mapping and balances remain immutable.
4. Open Review & reconciliation → Card transactions, then each row. Inspect the original evidence and choose purchase, refund,
   payment, fee, interest or other adjustment; verify date, signed amount,
   description and category. Choose its review state, give a reason, confirm
   original-source/duplicate review and save. For purchases, fees and interest choose a discussed necessary/discretionary spending purpose; it saves atomically with approval and updates Spending Analysis/card review. Card completeness needs renewed confirmation. Nothing approves rows automatically.
5. Inspect the separate original and approved closing comparisons. Corrections
   append decisions, retain original amounts and can reveal a difference.
   Rejecting an earlier approval withdraws it from current totals while preserving
   the full history. Refresh explicitly after stale-save errors; drafts remain and
   the source acknowledgment clears for renewed review.

## Exact liability rules

| Activity | Normalized change in debt | Spending treatment |
|---|---|---|
| Purchase, fee, card interest | Positive | Adds to card spending |
| Merchant refund | Negative | Reduces card spending; never income |
| Card repayment | Negative | Reduces debt; never another expense or income |
| Other balance adjustment | Either nonzero sign | Excluded from spending; verify its nature and cash counterpart |

Closing owed = opening owed + signed movements. Unknown original amounts make the
original comparison unknown. Approved closing/difference remain unknown until all
rows are approved or rejected. Rejected rows contribute no movement. Approval
requires a real in-period date, nonzero amount, consistent role/sign, description,
category, reason and explicit source confirmation. Adjustments are not a cash
advance model or automatic treatment of rewards, balance transfers or disputed
charges; these require professional evidence and later specialized rules.

Independent example: opening debt USD 1,000.00; purchase +100.01, refund -20.02,
payment -50.03, fee +5.04 and interest +2.05 produce closing debt **1,037.05**.
Card spending is **87.08**, repayment **50.03**, debt change **37.05**. A one-cent
purchase correction makes only the approved closing difference **0.01**; original
source arithmetic stays unchanged. Rejecting that purchase leaves card spending
**-12.93**, reflecting refunds exceeding the remaining charges, not income.

## Downstream figures and current limits

Approved and categorized card rows feed Spending Analysis. Card account/repayment
links, reviewed combined cash-versus-spending figures, card completeness and
card-inclusive final reports use the existing Bank & card activity, Card
completeness and final-report workflows. A bank-only dashboard retains its bank
scope. Importing or classifying card activity never rewrites saved budget, debt,
savings or cash-flow estimates, verified balance facts, or earlier report bytes.
A card payment reduces debt and must not become another purchase.

Source changes bump the shared source revision and mark affected captured reports
for renewed review. Purpose changes also clear the previous card-inventory review.
Final card-inclusive approval still requires current source, repayment and
completeness evidence; a reconciled statement alone does not prove completeness.
Original report formats, approvals and digests stay immutable.

Limits: 256 KiB/CSV or 2 MiB / 100 pages per selectable-text PDF, 1–200 selected
transactions/statement, 20 statements/client, 1 MiB captured source JSON,
4,000 individual source decisions/client and 8 MiB decision fields. Source periods
span at most 367 days. USD and documented numeric date/sign/column layouts are
supported. PDF running balances are reference evidence. Blank flattened debit/
credit cells, OCR, month-name dates, arbitrary institution layouts, cash advances,
installment terms and planning-account mappings need later work. Follow the
[PDF inspection rules](pdf-import-review.md) for ambiguous candidates.

Exact repeated source hashes are rejected per client. Repeated row fingerprints
and overlapping periods warn without automatically deleting potentially legitimate
rows. Durable card identities and explicit repayment links belong to Bank & card
activity; individual statement arithmetic remains separately visible.

## Persistence and recovery

Rust derives firm/counselor/client scope from the authenticated workspace. Scoped
immutable sources, individual decisions and purpose histories use immediate
revision/audit transactions. A purpose saved during source review binds to that
exact new approval in the same transaction. Failure rolls back both writes.
CSV/PDF inspection tokens are short-lived and client-bound; lock/unmount clears
private previews.

Reads and encrypted recovery validate original hashes/retained bytes, captured CSV
cells or canonical versioned PDF evidence, decisions, source ownership, revision
history and links. Corrupt evidence cannot activate through recovery. Schema 30
protects the new PDF format from older writers without rewriting legacy sources
or report payloads. Calculations use exact Rust integer cents. This stage adds no
external service, dependency or installer.

Verification covers exact amounts and signs, mandatory decisions, source mapping,
unknown/date/zero/limit failures, scope, stale revisions, duplicate files, atomic
audit rollback, immutable corrections/rejections, migration rollback, corrupt
recovery rejection and preservation/blocking of bank reports. UI tests cover
explicit inputs, history, retained failures, refresh confirmation, client/unmount
privacy and navigation/lock guards. Native verification exercises the full existing
application plus card import/review, reconciliation, correction/history, unchanged
bank amounts, report flags, restart, encrypted restore and client isolation.
Executed results are recorded in development-status.md.


## Integration available in 0.35.0

Durable reviewed card-account/source links, explicit bank repayment matching and a
provisional internal report supplement are now implemented. Use Financial Review
→ Bank & card activity and see [bank-card-review.md](bank-card-review.md). Original
bank dashboard/planning figures remain unchanged. Current [card completeness](card-completeness-evidence.md) and
[card-inclusive final approval](card-inclusive-final-approval.md) add the required
reviewed inventory, balance mappings, source and repayment evidence.
