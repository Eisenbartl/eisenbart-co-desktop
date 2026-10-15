# Financial completeness diagnostics — 0.31.0

Financial Review → Completeness & final validation reads current saved source
evidence for 1–24 calendar months. Reports → Final report checklist opens a
saved assessment, derives coverage from its original evidence and displays its
current authenticated context flags. The same checklist appears beside saved
assessments and in their internal PDF exports. Neither screen changes decisions,
amounts, workflow receipt statuses or historical report payloads.

## Meaning of coverage

Rust unions inclusive entered statement date ranges per original account ID,
clips them to the selected period, and lists uncovered and overlapping dates.
Unknown/partial/invalid periods supply no declared coverage. A statement with no
selected transaction rows can still have a declared period; this is explicitly
not verified row completeness. Overlaps suggest a duplicate/source-selection
review, not automatic deletion or adjustment. Leap days and month boundaries use
calendar dates, never estimated month lengths.

Every known account is shown, including archived and standalone accounts. Source
names remain historical; renaming/archiving cannot remove financial evidence or
silently erase a coverage gap. Full-period comparison is a conservative diagnostic:
this stage does **not** record whether an account existed throughout that period,
was legitimately inapplicable, or whether unrecorded accounts exist.

Applicable sources have an overlapping entered period, an in-period or undated
row, or unknown dates. Fully known out-of-period sources without applicable rows
are excluded from source details. Source counts cover their entire source, while
headline counts separate in-period statuses, undated rows across the client and
dated out-of-period rows. Rejected, pending, clarification, undated and
out-of-period rows never enter approved period amounts. Rejecting activity does
not verify a complete statement. Missing arithmetic comparisons stay unavailable;
differences, sequence conflicts, rows outside declared dates and source warnings
remain explicit. Original source warning details remain in Documents.

## Report validation and compatibility

`completeness.rs` owns `coverage-review-v1`. It uses the existing exact approved
assessment calculator and authenticated `get_client_imports` / report reads.
It introduces no money arithmetic, schema migration, remote asset or new library.
Client/organization/counselor bindings and immutable source/planning validation
remain enforced before the checklist is produced.

The checklist covers identity, calculation version, current context, period,
approved-row decisions, account assignments, document coverage, dated financial
facts/classifications and captured planning assumptions/scenario. Checked means
that particular technical check passed, not that a counselor confirmed the
whole financial picture. Unknown and Needs review are separate from Checked.
Current source/profile/branding/selected-context changes retain existing review
flags. Age/future-date diagnostics use the original calculation date for a saved
report; they are not a new assessment of the household today.

Derived validation lives outside the immutable payload and digest. Formats 1–6
remain byte compatible; no historical JSON is rewritten. PDFs label the derived
validation version, use original captured coverage and recheck current context
at export. Client plans keep their existing incomplete-basis disclosures and
inherit the original parent presentation; private internal validation details
are not added to the client-plan PDF.

## Explicit remaining work / next priority

**Finalization remains unavailable.** `finalEligible` is always false in this
stage. A complete household inventory and final professional review have not yet
been recorded. A fully decided queue, zero arithmetic differences, full declared
calendar coverage, a collection item marked received, or a saved scenario cannot
clear that block.

Next implement a versioned completeness review: explicit expected accounts and
applicable date ranges, missing accounts/obligations/information, documented
exceptions and source selection, scoped optimistic saves with atomic audit,
retained history and encrypted recovery. Bind that reviewed version to report
capture and invalidate it when its source/client/period context changes. Do not
add a finalization button until a specific final-report contract and all required
professional-review gates are implemented and tested. Broader statement parsers,
OCR, advanced debt terms, printing and commercial release acceptance remain in
the development map.

## Verification

Synthetic tests cover date union/clipping/gaps/overlaps/leap days, unknown periods,
empty accounts, row exclusions, assignment inconsistency, unavailable arithmetic,
source discrepancies, invalid periods, read-only behavior, authenticated scope,
lock/reopen and encrypted recovery. Historical report tests prove live source
changes update current flags without changing captured coverage, JSON or digest.
UI tests cover explicit period selection, mismatch rejection, refresh, late-reply
privacy, client changes and navigation. Native automation exercises both working
pages after the full financial/restart/recovery workflows and checks the internal
PDF disclosures. See development-status.md for checks actually completed.
