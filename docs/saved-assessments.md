# Saved counselor assessments — 0.13.0

In a saved client's statement-review panel, expand **Saved counselor assessments**.
Choose 1–24 complete calendar months, enter a title, and optionally add findings,
recommendations and follow-up arrangements. Review the approved basis, exclusions,
estimates and missing information, then confirm and select **Save assessment
record**. This saves an internal record inside the encrypted workspace. It is
not a finalized client action plan or a PDF export.

## What is preserved

Rust captures the saved client profile/goals, firm/counselor identity, creation
instant, app and calculation versions, exact period assessment, source revision,
all imported transaction fields and original mapped evidence, account assignments,
source hashes/metadata/statement checks, current transaction decisions, historical
approval bindings needed by classifications, and the latest verified balance facts.
Counselor-authored findings/recommendations/follow-up are captured separately from
the approved financial figures. The saved record is read-only; revisions require
saving a new record. Original encrypted source files remain in the workspace when
retention was authorized; the record does not duplicate complete original BLOBs.

Only Approved transactions inside the period contribute to financial amounts.
Other captured dates/statuses remain available as labeled evidence and exclusions.
The report viewer shows categories, monthly activity, dated balances, conditional
metrics, source discrepancies and paged original/decision evidence. Currency
formatting and charts reuse the locally bundled dashboard components, using exact
Rust-calculated strings and integer chart scales. Timestamps display in
America/Chicago; calculation dates retain their explicit UTC label.

Recurring-pattern decisions, saved plan/scenario inputs, debt APR terms and saved
session notes are not included yet. Existing planning panels are separate entered
estimates. No financial scenario is selected in this checkpoint. This record
therefore supplies the assessment/reporting foundation, not the complete specified
Counselor Report or Client Action Plan. PDF/print/local export and scenario-backed
client plans are subsequent work.

## Later changes

Changing import decisions or verified facts marks an older record **Needs review**.
Changing the client profile or workspace identity also flags its recorded context.
The original figures, profile, recommendations, supporting decisions and checksum
never change. Returning to the original transaction fields does not reset the
monotonic source revision. Saving a report itself does not change import revision,
transaction approval, financial facts, recurring reviews or plans.

A newer record recalculates from current approved decisions. Both records remain
available. A complete balance inventory does not establish complete statement
coverage; incomplete/estimated/outdated labels remain captured. These are
historical internal assessments and cannot be mistaken for current final reports.

If source/profile context changes while a draft is edited, the interface keeps
the notes and blocks saving until **Use current source context** is explicitly
selected; this clears the acknowledgment for renewed review. Backend revision
and saved-profile timestamp checks still reject simultaneous stale writes. Drafts
join navigation/lock/conflicting-operation guards and survive failed saves. Client
switches cancel late responses and clear the old identity/data.

## Storage, integrity and recovery

Schema 14 adds append-only `client_assessment_records` and an `assessment_recorded`
audit event. SQLCipher encrypts the complete payload. An immediate transaction
captures profile, sources, figures and atomic audit under the authenticated
organization/counselor/client. IPC accepts period/context and notes, not financial
outputs or arbitrary report bodies. Reads verify scope, headers, payload version,
SHA-256 bytes, evidence bindings, approved transfer reciprocity, captured totals
and recomputation at the original calculation date. SHA-256 is an integrity check,
not a digital signature, certification, or independent verification of evidence.

Recovery validates every saved record before activating a new workspace. Corrupt
records fail safely while preserving the current files. Older schema-13 workspaces
migrate transactionally without changing financial sources or decisions. Future
calculator changes must preserve the `approved-assessment-v1` reader/validator or
add explicit compatible version handling; never silently regenerate older figures.

Limits: 50 records/client, 8 MiB per encoded payload, 32 MiB total payloads/client,
120-character title and 4,000 characters each for findings/recommendations/follow-up.
Multiline notes are allowed; invalid control characters are rejected. Supporting
history is bounded to 5,000 current decisions plus up to 500 older fact bindings;
all existing import/account/document limits remain. Oversized saves write nothing.
Full originals, repeated recurring history and prior balance-fact snapshots are
not copied into each record. Encrypted recovery retains its existing overall
256 MiB database safety limit. Maximum-size performance acceptance remains part
of final release testing.

## Independent synthetic workflow

The native fixture records operating cash flow **199.98** and estimated net worth
**525.05**. A one-cent source correction updates the live view to **199.99** and
invalidates the old expense purpose until explicit re-verification. Withdrawal
updates the live view/new record to **500.00**, while the original saved record
still shows **199.98** and Needs review. Both retain their captured evidence and
recommendations after full-process restart and both encrypted recovery paths.

Backend examples independently verify **799.99** before withdrawing rent,
**2,000.01** after withdrawal, and captured net worth **4,000.06**. They also test
stale context, foreign client IDs, missing acknowledgment, failed atomic audit,
append-only history, malformed figures/dates, migration rollback and corrupt
snapshot rejection before recovery activation.

## Validation checkpoint

Version 0.13.0 implementation d2b0ab5 passed 78 interface tests, 112 Linux library
plus 2 executable worker tests, formatting/Clippy, production frontend and the
compiled native workflow. Windows source CI [38600293809](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38600293809)
passed 113 library plus 2 worker tests and all 78 interface tests. All installer,
installation-acceptance and package/upload steps were skipped. Native validation
verified original/new records and source-change flags through full restart and
both encrypted recovery paths. The first transport interruption and successful
fresh run are recorded in development-status.md. No financial write was replayed.
Schema 14 requires a compatible executable; older applications reject newer
workspaces. Complete Windows offline installation/release acceptance and maximum
data-volume performance remain open.
