# Local category suggestions — introduced in 0.41.0

The original approved-evidence policy below remains available. Version 0.42.0 adds [explicit reusable category rules](local-category-rules.md), combined bank/rule snapshot checks and original-version application history in schema 26. A rule can supply an explicitly entered category with fewer than two approvals; conflicting or unsafe evidence still blocks it.

Open **Transactions → Classification assistance** for the selected client.
The page lists unreviewed bank expenses and fees with a missing category, in
stable pages of 25. Search description or merchant with literal text; punctuation
such as `%`, `_` and apostrophes has no special search meaning. Working tools
remains the place for all other types and manual corrections.

Suggestions reuse the existing approved merchant groups. Matching requires the
same source account and transaction type. Entered merchants match entered
merchants; when neither record has a merchant, the description is the fallback.
Only case and repeated whitespace are normalized. No numbers, merchant suffixes,
punctuation or Unicode characters are discarded to force a match.

A suggestion requires two to five current matching approvals, at least two
distinct dates, one identical nonempty category and no possible duplicate
evidence. The target also needs a known withdrawal amount/date/description and
no possible duplicate. Conflicting, missing or insufficient evidence stays
unresolved with a reason. Groups above five remain unresolved in this first
bounded workflow: all evidence used for an offered suggestion is visible.
Unknown income/expense types, income, refunds, interest, transfers, balance/debt
movements, linked transfers, already categorized/approved/rejected records and
card records are outside this policy. No confidence percentage is invented.

Expand **Current approved evidence** to inspect dates, exact USD strings,
descriptions, categories, source document/page/line and original approval IDs.
Only current approved records support a suggestion; prior, rejected, reversed
and undecided decisions do not supply supporting approval evidence.

## Using and saving a suggestion

**Use suggested category in unsaved review** rechecks the source snapshot and
fills only the category in the existing transaction review form below. It does
not write or approve anything, change the amount/date/type, or select Approved.
The original unreviewed status is preserved. **Discard category review draft**
clears it without saving; ordinary navigation also uses the existing draft guard.

Review the original source, enter a reason and explicitly save a decision. You
may retain Pending/Needs Clarification; their amounts remain excluded from
approved spending. Choosing Approved uses the same warning acknowledgement,
correction, sign, date, provenance, duplicate and reason checks as Working tools.
Existing immutable decision history, audit, reversal and financial-fact/report
invalidation continue to own the result. There is no bulk suggestion approval.

Rust reads authenticated client/counselor scope in one SQLite snapshot. Pagination
and draft preparation use a fingerprint bound to policy version, client, import
revision, accounts/documents, current records and exact decision history.
Preparation also checks the expected revision and recomputes the category; the
renderer cannot supply an arbitrary proposed category to that read command.
The final save uses existing optimistic import-revision checks. A changed bank
evidence set requires an explicit **Reload imported records**, not silent retry.

Querying and preparing drafts add no tables, financial facts, saved rules or audit
rows. Saved decisions already persist in encrypted storage, restart and recovery.
Client changes/lock clear suggestions and drafts, ignore late replies, and hold
navigation while a request is active. No client data goes to a remote service,
browser local storage or URL. The policy is `approved-merchant-category-v1`;
schema 25 and all financial calculators/approval policies remain unchanged.

## Verification and remaining work

Backend fixtures cover independently expected Food categories, conflict/reversal,
duplicate/unknown/large/same-date evidence, literal merchant matching, paging,
client/counselor scope, stale fingerprints/revisions, read-only counts, audited
rollback, explicit unapproved saves, restart and encrypted recovery. UI fixtures
cover evidence display, escaped text, paging, draft/discard/save through the old
review API, stale/foreign replies, busy/dirty navigation and client/lock cleanup.
The native suite retains every prior assertion and exercises actual CSV import,
two approvals, 31 candidates, real paging, an unsaved category draft, unapproved
save with unchanged approved totals, history and restart/recovery/client isolation.
Actual executed checks are recorded in [development status](development-status.md).

Reusable counselor rules are implemented in 0.42.0; see [rule usage and limits](local-category-rules.md). Larger paged evidence, cross-account matching, card/other transaction roles and bulk workflows remain open. The policy description above records the original 0.41.0 behavior; schema 26 now persists rules and original applications.
No installer is created unless the user asks for one. Installed Windows offline
acceptance and commercial/security work remain separate release requirements.
