# Credit-card CSV review — 0.34.0, schema 23

Documents → Credit-card statements now records signed USD card CSV evidence,
individual counselor decisions and statement arithmetic in encrypted storage.
This is a working liability review, separate from the existing bank cash engine.

## Use the page

1. Select the client and open Import a card CSV statement. Choose/inspect a UTF-8
   CSV, then explicitly map date, description, signed amount and optional category.
   Choose the source date format. No column meanings are guessed.
2. Enter the card name, statement dates and opening/closing balances owed. Positive
   balances mean debt; negative balances mean a credit held on the card. If the
   source uses negative charges and positive payments, explicitly invert its signs.
   Optional authorization retains the complete CSV inside workspace encryption.
3. Save the statement. Every row starts pending and unclassified. Original source
   cells, line/record references, hash, mapping and balances remain immutable.
4. Open each row. Inspect the original evidence and choose purchase, refund,
   payment, fee, interest or other adjustment; verify date, signed amount,
   description and category. Choose its review state, give a reason, confirm
   original-source/duplicate review and save. Nothing approves rows automatically.
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

## Bank reports and current limits

Card statements do **not** feed bank dashboards, budgets, debt forecasts, cash-flow
calendars, report amounts or balance facts in this stage. The page and bank tools
state this boundary. Do not import card purchases as bank cash movements or label
repayment as spending to work around it.

Recorded card evidence bumps the shared source revision and flags every existing
or newly recorded bank assessment and linked client plan. Final bank-report
approval is blocked for any client with known card statements, even a completely
reconciled card statement: integrated card capture/reporting is not yet supported.
Original assessment/approval payloads and digests remain unchanged; historical
approvals remain readable with explicit current review flags. Other clients are
unaffected. No final-report policy v1 meaning is rewritten.

Limits: 256 KiB/CSV, 1–200 rows/statement, 20 statements/client, 1 MiB captured
source JSON, 4,000 individual decisions/client and 8 MiB decision fields. Source
periods span at most 367 days. USD, signed amount columns and ISO/US/EU explicit
source dates are supported. Separate debit/credit card columns, card PDFs, OCR,
per-row reported card running balances, automated bank-specific parsing, cash
advances, installment terms and card account metadata CRUD are future work.
Card names identify source groups for overlap warnings; they are not durable
bank-account IDs or planning-account links. Exact repeated source hashes are
rejected per client. Repeated row fingerprints and overlapping periods under the
same card name warn without automatically removing potentially legitimate rows.
Statement summaries are not combined, preventing silent duplicate aggregation.

## Persistence, recovery and future integration

Rust derives firm/counselor/client scope from the authenticated workspace. Schema
23 adds immutable card sources and append-only individual decisions, compound
ownership links and audit events. Immediate transactions recheck the shared
source revision and save source/decision, revision and audit together. Failed
writes preserve state and drafts. Existing bounded CSV inspection supplies
short-lived, client-bound tokens; lock/unmount clears previews.

Reads and encrypted recovery validate bounded sources, original hashes/retained
bytes, captured CSV cell extraction, canonical decisions, source row ownership,
chronological revision history and scoped links. Corrupt decisions cannot be
activated through recovery. No financial calculations use JavaScript or binary
floating point. No dependencies, network services or installer were added.

Next priority: integrate card spending/liability evidence and explicitly linked
bank repayments into financial review and versioned report snapshots. Define the
cash-versus-spending views first, verify independent combined examples, retain
legacy report bytes, and require complete evidence before a new final policy can
approve card-inclusive reports. Do not remove the conservative bank-report block
until that acceptance passes. Card PDF extraction and true card-account identity,
renaming/archive and planning mappings follow as separate bounded work.

Verification covers exact amounts and signs, mandatory decisions, source mapping,
unknown/date/zero/limit failures, scope, stale revisions, duplicate files, atomic
audit rollback, immutable corrections/rejections, migration rollback, corrupt
recovery rejection and preservation/blocking of bank reports. UI tests cover
explicit inputs, history, retained failures, refresh confirmation, client/unmount
privacy and navigation/lock guards. Native verification exercises the full existing
application plus card import/review, reconciliation, correction/history, unchanged
bank amounts, report flags, restart, encrypted restore and client isolation.
Executed results are recorded in development-status.md.
