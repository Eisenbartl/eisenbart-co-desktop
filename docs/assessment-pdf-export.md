# Offline internal assessment PDF export — 0.14.0

Open a saved client, expand **Saved counselor assessments**, and select a saved
record. Use **Choose PDF destination**, or enter a full local path ending in
`.pdf`, then select **Export saved assessment PDF**. Choose a new filename in a
private folder outside the application data directory. Save or discard an
assessment draft first. The completion message gives the file location and page
count. The PDF can be opened and printed in a local PDF viewer; direct printing
inside the app is not implemented.

The file is readable client information outside encrypted workspace storage.
Export never overwrites an existing file, including a symbolic link. Canceling
the chooser preserves the destination. A failure preserves the record and entered
path; there is no automatic write retry.

## Contents and financial basis

The PDF includes the captured firm name, client/counselor identity, record and
export times (explicit UTC), assessment period, calculation version, goals,
approved income/spending/refunds/cash flow, monthly/category charts with text
figures, classifications and dated inventory metrics, counselor findings,
recommendations and follow-up, document IDs/hashes/discrepancies, transaction
source locators and captured decisions, inventory evidence and record checksum.
Pages have margins, wrapping and page numbers. Money strings and integer chart
scales come from the saved Rust assessment, never live UI amounts or browser
calculations. The stored record digest refers to the archived record, not the PDF.

Every page is labeled **INTERNAL / INCOMPLETE**. All saved source rows remain
labeled evidence; only approved rows within the period supplied the financial
amounts. Missing balances stay unknown, estimates are labeled, and unverified
coverage, outdated periods and other warnings remain visible. Original statement
files are referenced, not attached. No statement files, credentials or encrypted
workspace keys are passed to a PDF subprocess.

At export, Rust authenticates the session, scopes the client and record to the
counselor/firm, validates the stored hash, identity, evidence and original
calculation, and obtains current review flags. Later source/profile changes print
**NEEDS REVIEW** and the reasons while preserving the historical figures. Export
neither changes source revisions nor creates a new assessment, financial decision
or audit entry; assessment creation remains audited. Export-specific audit events
and organization export permissions remain future security work.

This is a printable internal assessment checkpoint. Selected planning scenarios,
debt APR/repayment terms, recurring-review decisions and saved session notes are
not captured yet. A complete specified Counselor Report, final report acceptance,
branded Client Action Plan with prioritized actions, allocations and milestones,
and direct application printing remain open.

## Implementation and limits

`assessment_pdf.rs` uses the already locked `lopdf` library to create PDF 1.7 with
local vector bars, an embedded Open Sans font, glyph advance widths and a Unicode
text map. There are no external fonts, renderers, web resources, JavaScript,
network requests or new PDF generation service. `ttf-parser` reads bundled font
metrics; `tempfile` creates a private temporary file in the destination folder.
Both were already in the locked dependency tree and are now direct dependencies.
The Apache-2.0 font license and attribution are preserved with the bundled asset;
see [licenses.md](licenses.md).

Long text and hashes wrap by measured width. Export is limited to 1,000 pages and
32 MiB and fails rather than producing a truncated report. The bundled font
supports common Latin, Greek and Cyrillic characters. Unsupported glyphs,
combining diacritics requiring shaping and directional control characters fail
with their Unicode code point; use supported text/precomposed accents in a new
record. Full complex-script shaping, multilingual font fallback, custom logos,
PDF accessibility tags and PDF/A archival certification are not implemented.

Generation finishes before opening a destination file. The temporary file is
flushed/synchronized, then published without replacement in the same folder;
ordinary failures clean up temporary bytes. Destination folders must exist and
be writable; filesystems must support the no-clobber publication operation.
A crash/power failure can leave a temporary file; directory durability and external
folder protection still depend on the OS/filesystem. Export rejects paths under
the canonical workspace root and never replaces a workspace or unrelated file.

## Verification

Four new backend tests cover independent original cash flow 799.99/net worth
4,000.06, changed live cash flow 2,000.01 with historical export retained,
Unicode extraction/font embedding, multipage notes/evidence and measured margins,
page limit, unsupported characters/no partial output, scope/lock and destination
failures. Corrupted records also fail export before a file is created. Two new
interface tests exercise cancellation, selected-record arguments, failed-path
retention, conflicting operations, unsaved-draft blocking and late client replies.
The native workflow exports fictional Taylor's original 199.98/net worth 525.05
after withdrawal changes current activity to 500.00, checks extracted PDF text and
refused overwrites, and repeats after restart and both encrypted recovery paths.
Native PDF inspection uses development-only Poppler `pdftotext`; it is not a
shipped runtime dependency. Actual results are in [development-status.md](development-status.md).
