# Reusable local category rules — 0.42.0

Open **Transactions → Classification assistance** for a selected client. Under
**Reusable category rules**, create a labeled rule, choose an active bank account,
Expense or Fee, and either an explicit merchant or the description fallback.
Enter the exact matching text, category and reason, then save. Only case and
repeated whitespace are normalized; numbers and punctuation remain significant.
Description rules apply only when the transaction has no explicit merchant.
Rules belong to the authenticated client and counselor, and one source account
and transaction type. They never use another client's data or a remote service.

**Edit rule** appends a new version. Select Archived to stop offering it; clear
Archived to reactivate it on an active account. A reason is required for each
change. Expand version history to read the original versions and authorship.
There is no permanent deletion or silent overwrite. Duplicate active matching
rules are rejected. Archiving the account disables its rules until both account
and rule are active; history remains available.

## Review each transaction

A matching rule can offer its explicitly entered category even without two prior
approvals. Current approved evidence must agree with it. Conflicting evidence,
unknown source fields, possible duplicates and evidence groups above five stay
unresolved. This does not infer income, transfers, card/debt roles or spending
necessity. The existing approved-evidence suggestion policy remains available
when no active rule matches; see [category suggestions](classification-assistance.md).

**Use suggested category in unsaved review** prepares a draft without saving or
approving anything. A rule draft shows the exact rule version. Source and category
fields are protected in that draft; discard it to make a manual correction.
Review the original document, choose a review status, enter a reason and save.
Pending/Needs Clarification remain excluded from approved spending. Approved
still requires all existing date, sign, warning and source-review checks.

The final save rechecks the exact bank and rule snapshot in the same transaction
as the original decision, rule-application record and audit. An edit, archive,
account change or changed bank evidence makes a prepared draft stale. Reload
and review it again; the application does not retry a stale write automatically.
Rule creation/editing itself never changes transactions, bank import revisions,
financial totals, reports or approvals. There is no bulk application.

**Saved rule applications** shows the most recent 25 applications and the retained
total, including original category, decision status, review date and exact rule
version. Later rule edits/archive do not rewrite that record or the original
transaction decision. A later manual decision can change the transaction's current
state; this application list remains historical evidence of the original save.

## Persistence, limits and verification

Schema 26 adds immutable rule versions and original-decision application links
inside the encrypted workspace. Existing reports and transaction-history JSON
remain unchanged. Migration from schema 25 is atomic; backup/recovery validates
all rule versions, digests, account scope and original decision/source bindings
before activating recovered data. Older application versions cannot open the
upgraded workspace. Lock/client changes clear drafts and ignore late responses.

The initial bounded workflow retains at most 50 rules per client (including
archived rules), 100 versions per rule and 5,000 applications. Labels/patterns
are at most 120 characters and categories 80; control characters and empty values
are rejected. Larger evidence paging, cross-account/firm-wide rules, other source
roles and bulk application remain separate work. No installer is created unless
requested. [Development status](development-status.md) records executed checks;
installed Windows offline and commercial acceptance remain open.
