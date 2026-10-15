# Saved completeness reviews — 0.32.0, schema 21

Financial Review → Completeness & final validation → Saved completeness reviews
now records counselor coverage context in encrypted storage. It is separate from
the live date-coverage diagnostics, approved financial facts and final approval.

## Working workflow

1. Open a client and the saved reviews section. Enter the same review months you
   intend to use in a report (1–24 complete calendar months).
2. Describe the household account/obligation inventory reviewed. Marking it
   reviewed requires notes; leaving it unchecked retains explicit uncertainty.
3. Every known original statement account remains listed, including archived
   accounts. Enter an applicable date range inside the review period, or leave
   both dates unknown. Document opening/closure or other exceptions in notes.
4. Select coverage sources for the matching account. Add unprovided accounts or
   obligations such as loans/cards as unlinked expected items. Their balances
   remain unknown; this does not create an approved financial account or parser.
5. Record missing information and a conclusion, then save. Each save appends a
   new version with original choices, source revision, client-profile context and
   timestamp. Numbered history is read-only, even the latest numbered version.
6. In Reports → Working tools, choose Include a saved completeness review, load
   and inspect it, and use its exact months and current client/source context.
   The report captures the original saved review, not editable browser fields.

Known accounts cannot be removed from a review; manual expected items can be
removed from the next draft while previous saved versions retain them. Bounds:
40 expected items, 20 known statement accounts, 50 source choices, 100 versions,
128 KiB/version, 2,000 characters each for inventory/conclusion, 4,000 for missing
information and 1,000 per exception. Unknown/partial source periods never provide
coverage. The saved unresolved list counts uncovered selected expected days,
unknown applicability, manual missing evidence and exceptions. Live financial
review still displays arithmetic issues, undecided rows and source-wide checks.

Coverage source selection is **review context only**: it never excludes approved
transactions from report amounts. Overlapping sources and duplicates still need
the original review controls. A documented exception does not automatically
waive a final-report requirement. No missing figure becomes zero.

## Consistency and history

Rust derives ownership from the authenticated workspace and validates all
account/source links. Immediate transactions enforce review/source/profile
versions and atomic audit; stale failures retain drafts and never retry writes.
Review latest context keeps the draft, adds newly known accounts with unknown
dates and clears the inventory acknowledgment for explicit reconsideration.
A saved-reference view makes the latest stored values available for comparison.
Navigation, client changes and lock retain existing draft/privacy guards; a dirty
or busy saved-review section cannot be collapsed to discard its data silently.

Schema 21 adds scoped append-only `coverage_review_history` and its audit event.
Restore validates every bounded, canonical historical snapshot and original
account/source binding before activation. Review changes do not bump financial
import revisions or alter plans, statement metadata, receipt statuses or totals.

Format-7 internal assessments optionally attach the exact reviewed snapshot.
Capture requires the latest saved version, matching original source choices,
import revision, client-profile timestamp and exact period. Reads/export/recovery
compare the attachment to its scoped original history; a newer review flags the
assessment and linked client plan, even if values were reverted. Old snapshots,
JSON bytes and digests remain unchanged. Optional omission preserves formats
1–6. Current source/client changes keep the earlier stale-context checks.

Internal report views, the report checklist and internal PDFs show the captured
inventory, missing information, expected dates, source selections and exceptions.
Client-plan PDFs retain their existing disclosures and omit this private internal
review context. All presentation remains offline.

## Remaining work / next priority

Final-report approval remains unavailable. This stage records review context,
including unresolved information; it does not certify complete finances. Next
define and implement a final-report readiness/approval contract: explicit required
checks, applicability and exception decisions, current professional review,
scenario/assumption agreement, auditable finalization and immutable final exports.
A counselor checkbox or full declared calendar coverage must never bypass those
requirements. Additional source parsers, OCR, advanced financial terms, printing
and commercial release acceptance remain in the prototype development map.

## Verification

Synthetic tests exercise valid/invalid date and source selections, mandatory known
accounts, limits, stale versions/profile/source context, ownership, audit rollback,
read-only history, migration rollback, lock/restart and encrypted recovery. Report
tests preserve old payload bytes, bind original review versions, reject forged
rehashed attachments and keep private review notes out of client-plan PDFs. UI
checks cover exact inputs, retained failures, explicit refresh, historical modes,
late replies, report selection and global navigation/lock guards. Native checks
create/amend/reopen a review, attach it to an assessment, restart, restore encrypted
history, and export the original review after a newer version exists. Actual
completed results are recorded in development-status.md.
