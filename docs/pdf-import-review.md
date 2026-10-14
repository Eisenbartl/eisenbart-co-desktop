# Selectable-text PDF import — 0.8.0 source

This is the second document-processing checkpoint. It imports selected text lines
from synthetic USD checking/savings PDF statements into the existing review
queue. No new installer was requested; the earlier 0.3.0 download lacks this
feature. Use fictional records while Windows acceptance/security review remain open.

## Import steps

1. Open a saved client. Under **Statement import and transaction review**, choose
   **Selectable-text PDF** as the file format. Choose the local PDF, or enter its
   full path, then click **Inspect PDF**.
2. Expand each page and compare its extracted text against the original PDF in
   your trusted offline PDF reader. The application displays text, not a PDF
   renderer. Page numbers refer to the source PDF; line numbers refer to extracted
   text and can differ from printed rows. Nothing has been saved or selected yet.
3. Select the actual transaction lines explicitly. Select 1–500 lines per source.
   Supported starting layout: a full date first, description in the middle, one
   signed USD amount last. For example:

   ```text
   2026-10-01 Fictional payroll 1000.01
   2026-10-02 Fictional groceries -25.11
   ```

4. Select an existing checking/savings account, or name a distinct new one.
   Choose the actual date style (YYYY-MM-DD, MM/DD/YYYY, or DD/MM/YYYY). Normally
   deposits are positive and withdrawals negative; sign inversion requires its
   explicit checkbox. PDF extraction currently supports one signed amount,
   not separate debit/credit or running-balance table columns.
5. Enter statement period/opening/closing balances only if known. Blank stays
   unknown. Decide whether to authorize retaining the complete original PDF
   inside the encrypted workspace; the checkbox starts unchecked.
6. Click **Import into review queue**. Every selected PDF record starts as
   **Needs Clarification** and is ineligible for bulk approval. Nothing is
   approved automatically.
7. Review each record against its original page/line. Correct dates, descriptions,
   and amounts if necessary; assign its category and transaction type; choose a
   decision, enter a verification reason, and acknowledge source warnings.
   Unknown/invalid amounts or dates and unclassified records cannot be approved.
   PDF font decoding and text ordering do not establish bank verification.

Malformed, protected, fully scanned/empty, or unsupported files fail visibly and
write no accounts/documents/transactions/audit event. Image-only pages in a mixed
PDF can have no extracted text; check the whole original for missing transactions.
Wrapped descriptions, split table cells, or uncertain glyphs need individual
correction. Multiple numeric values make the amount unknown rather than choosing
between transaction and running balance. Headers/totals are not automatically
identified or selected. Dates, account numbers, categories, merchants, types,
statement balances, and missing amounts are not inferred.

## Review and exact totals

The existing CSV review workflow also applies: four exact states, client filters,
immutable reported values, duplicate advisories across CSV/PDF sources on the same
account, paired transfers, approval reversal history, and approved-only totals.
PDF source text stays unchanged when verified values are corrected. Source page,
extracted line, hash, source ID, account, selected-line metadata, entered period,
extraction method `pdf-text-lines-v1`, uncertainty flags, and status are retained.

Source opening/closing checks include only selected lines and rely on entered
balances. They do not prove completeness. Approved reconciliation is unavailable
until every imported row is Approved or Rejected. A pending PDF expense contributes
zero to official totals, even if extraction appears numerically clear.

Independent fixture: PDF payroll 1,000.01 and groceries -25.11 yield original net
974.90. Approving only payroll adds 1,000.01 to income/activity; the unapproved
expense adds zero. Alongside the prior CSV fixture, income becomes 3,000.01 and
combined approved activity 1,820.02. Reversing the approval withdraws its effect
and preserves the prior decision. Planning inputs remain separate.

## Parser isolation, storage, and limits

The desktop starts a supervised child of its own executable in a private parser
mode before Tauri/workspace initialization. Only the bounded source bytes travel
through stdin; no workspace key, password, file path, source filename, HOME,
authentication, or proxy variables are passed. Only required OS/library loader
selectors are preserved. The child uses pure-Rust `lopdf`/`pdf-extract` to decode
text, without opening a renderer, evaluating PDF JavaScript, extracting attachments,
or doing OCR. Library errors/panic detail and document contents are not logged.

The supervisor rejects failure, oversized/malformed output, and expired processing;
it kills and reaps the child. Wall time is 15 seconds. Linux also limits CPU to
10 seconds and heap/data allocations to 512 MiB, and disables core dumps; Windows uses a 512 MiB process
memory Job Object with kill on close. Input is at most 2 MiB, output text 1 MiB,
PDF pages 100, nonempty lines 2,000, all extracted lines 4,000, and each line
2,048 bytes. Pipe output is capped at 2 MiB. These are resource/process boundaries,
not a complete operating-system filesystem/network sandbox or a security certification.
The parser's production path has no workspace/file/network service capability.

Inspection captures exact source bytes/hash and decoded pages for up to 15 minutes;
lock/cancel discards the preview. Only selected immutable text and metadata are
persisted unless complete-source retention is authorized. Original PDFs are optional
SQLCipher BLOBs, covered by the existing authenticated encrypted backup. The source
outside the application stays where it was selected. Imports, revisions, and
`pdf_imported` audit writes are atomic; failed operations preserve previous data.

Schema 9 preserves earlier clients/plans/CSV provenance/approval history and adds
PDF audit events. Missing PDF fields in older CSV JSON remain readable. Restore
validates PDF source locators as well as imported decisions and transfer links.
Use 0.8.0 or newer for upgraded workspaces. Existing per-client limits still apply:
20 accounts, 50 documents, 5,000 imported records, 10,000 review-history entries;
whole-workspace backup limit 256 MiB. Split/large statements may require CSV export.

Scanned OCR, institution-specific adapters, automatic table reconstruction,
merchant/recurring-payment grouping, PDF export/reports, native original-document
viewing, and clean installed Windows 11 acceptance remain separate work. Dependency
notices and independent security review must be completed before commercial use.
