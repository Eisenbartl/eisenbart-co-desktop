# Merchant grouping and recurring-payment review — 0.9.0 source

Open a saved client and scroll to **Merchant groups and recurring payments** in
statement review. Import and approve the actual statement transactions first.
Pending, Needs Clarification, and Rejected records never enter this analysis.
Only approved expenses and fees are included in this checkpoint: refunds,
income, interest, transfers, and balance movements are excluded.

Payments are grouped within one account and transaction type using the verified
merchant name. Capitalization and repeated whitespace are ignored; punctuation,
digits, bank suffixes and merchant identities are not guessed or stripped.
Without a merchant, the exact normalized description is used and marked as a
fallback. Different accounts/types stay separate. To combine related statement
names or separate unrelated payments, use **Review source transaction** and
correct the verified **Merchant** field with the required reason and source
acknowledgment. The immutable original statement values remain available.

Click **Review payments for …** to see verified dates, signed amounts,
descriptions/categories, source filename and CSV line or PDF page/line. The
regular review queue provides the complete original provenance and corrections.
All amounts and observed totals are computed in Rust using integer cents. A
three-payment example of $10.01, $12.03 and $10.01 has total $32.05, a $10.01–$12.03
range, and $22.04 left if the first approval is withdrawn.

At least three approved payments on distinct dates are required to confirm
recurrence. Possible duplicates and multiple same-day payments block confirmation
until reviewed/rejected or separated with corrected merchants. Groups with more
than 500 payments cannot be decided in this checkpoint. No automatic approval
or subscription classification occurs.

Suggestions consider every adjacent date in the group:

| Pattern | Conservative date test |
|---|---|
| Weekly | Every interval 6–8 days |
| Every two weeks | Every interval 12–16 days |
| Monthly | Consecutive calendar months, posting day varies by at most 3 |
| Quarterly | Every three calendar months, posting day varies by at most 3 |
| Annual | Every twelve calendar months, posting day varies by at most 3 |

Calendar tests handle January 31 → February 28/29 → March 31. Two charges,
skipped periods, same-day charges, conflicting duplicates or an irregular
sequence do not produce a regular suggestion. Variable amounts are displayed
as a range and warning; no fixed future charge or monthly average is invented.
These are historical date patterns, not proof of a continuing obligation,
merchant identity, complete statements, or a subscription. Semimonthly schedules,
multiple plans at one merchant and incomplete histories need manual separation
or counselor-defined review rather than an invented pattern.

Choose or correct a payment label and frequency, optionally check **I verified
this is a subscription**, and provide a review reason. Frequency can be corrected
to any supported cadence or **Irregular / counselor-defined** after verifying the
evidence. Alternatively dismiss the pattern or withdraw an earlier confirmation.
**Save payment pattern decision** records the explicit decision; it never
rewrites historical transactions or budget/debt/savings/cash-flow plans, predicts
a guaranteed next charge, cancels a contract, or moves money.

Every decision captures the approved facts and their exact approval-history IDs,
with counselor, time and reason. Changes to evidence, approvals, dates, amounts,
merchant membership or duplicate warnings mark the previous decision **Needs
review**. Reapproving an identical transaction still requires reconfirmation;
old confirmations cannot silently revive. Evidence changes also append a permanent invalidation marker in the same source transaction: approving a new payment and later rejecting it cannot revive an earlier decision when the remaining evidence matches again. Groups without approved payments
leave the current list while their decisions remain in history. Expand **Payment
pattern decision history** for captured evidence and previous reasons.

Unsaved payment edits participate in the existing navigation/lock safeguards.
Conflicting source/review operations are disabled while editing, and failed
saves preserve the draft. Writes enforce authenticated client ownership,
optimistic import revisions, matching evidence hashes, and atomic audit/history
transactions in Rust. No caller-supplied evidence or organization is trusted.

Schema 11 stores append-only history in the encrypted local workspace; encrypted
backups include it, and restore validates each captured approval and original
source locator. Migration failures roll back. The earlier schema-10 source checkpoint is supported: its saved decisions are retained and flagged for one explicit reconfirmation because transient evidence changes were not tracked there. Older application versions cannot
open schema 11. Local bounds are 1,000 decisions/client, 256 KiB/decision and
8 MiB total captured decision JSON/client; exceeding a limit fails without
saving or removing history. Existing import/workspace/backup limits still apply.
This source checkpoint creates no new Windows installer. Scanned OCR,
institution adapters, row-level running-balance reconciliation, reports and
commercial readiness remain separate work.
