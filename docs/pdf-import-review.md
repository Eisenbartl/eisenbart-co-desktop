# Transaction-focused statement extraction — 0.48.0, schema 30

Open a client’s **Import statements** section. Bank imports accept one checking
or savings account; **Credit-card statements** accepts card liability statements.
Use CSV with explicit column mappings, or choose **Selectable-text PDF** and
inspect the local file. Extraction runs offline in the bounded PDF worker.

## Inspect and select transactions

The PDF checklist shows transaction candidates, rather than every source page and
line. Institution contact text, fine print, opening/closing balances, statement
summaries and totals stay outside the checklist. Full extracted text remains in a
collapsed, read-only source-context panel for comparison against the original PDF.
There is no automatic institution verification or OCR.

Clear dated transaction candidates start checked. Uncertain candidates start
unchecked. Check/uncheck any candidate, restore the suggested selection, or uncheck
all. Search and confidence filters change visibility only; checked hidden rows
still belong to the selection. Every selection change recomputes the count,
known credits/debits and net movement in Rust using integer cents. Unknown amounts
are counted separately and excluded from known subtotals. These are unsaved source
totals; they do not become approved financial facts simply by checking a box.

Supported evidence includes:

- ISO dates and explicit month/day or day/month formats; ambiguous dates require
  a format choice. Yearless dates require one uniquely matching date in an actual
  statement period, including year rollover. Detected numeric statement dates can
  be adopted explicitly; account numbers and balances are never guessed.
- Signed USD amounts, parentheses/trailing minus, CR/DR markers, debit/credit
  section headings, transaction amount plus running balance, and separate
  debit/credit columns with explicit zero cells. Choose the amount layout or sign
  inversion when needed. Running balances are reference evidence, not spending.
- Up to three consecutive lines of a dated wrapped transaction. Original selected
  text, page/extracted-line location and recognized values are preserved.

Blank flattened debit/credit cells, multiple unmatched monetary columns, dates
outside the period, ambiguous/invalid dates and missing amounts remain uncertain.
Standalone undated text is excluded; unsupported transaction layouts need CSV export.
Compare against the complete original. A transaction can be corrected in review;
unsupported files should use the institution’s CSV export. Scanned/image-only pages,
month-name dates and arbitrary institution layouts need later parser/OCR work.
The detector is conservative and cannot establish statement completeness.

Limits remain 2 MiB / 100 pages per PDF; 500 selected bank transactions or 200 card
transactions per source. Sources over the selection cap show a truncation warning;
never treat that selection as the full statement. Complete-original retention is
optional and unchecked by default; when authorized the original bytes remain in
encrypted storage. Otherwise only selected transaction blocks and the minimal
financial headings needed to reproduce their interpretation are saved, together
with the hash, settings, source locator and immutable imported values.

## Review and downstream figures

Importing updates the client’s source inventory and review counts. Every PDF row
starts **Needs Clarification**, ineligible for bulk approval. Open **Review &
reconciliation → Review queue** for bank rows or **Card transactions** for cards.
Verify dates, amounts, descriptions, category and actual financial kind. Purchases,
withdrawals, deposits, payments, refunds and transfers retain their different
financial treatment; a card payment is not another purchase.

For an approved bank expense/fee/interest withdrawal or card purchase/fee/interest,
choose **Necessary spending** or **Discretionary spending**, enter the discussion
reason and confirm individual review. Source approval and purpose save atomically.
Errors or stale context leave both unchanged and retain the draft. Purpose decisions
bind to the exact current approval; an unrelated correction/reversal invalidates
the old binding unless a new purpose is explicitly saved. Every earlier decision
and original imported field remains available in history.

Saved decisions automatically feed Spending Analysis totals, percentages, charts
and opportunities; bank decisions also update current approved bank figures and
verified-purpose metrics, while card decisions update card review/completeness
figures. Bank-only dashboards keep their documented account scope. Current views
recompute on reopening; no manual copying is required. Saved planning inputs remain
counselor estimates, and saved reports retain their captured values with renewed
review required when their supporting evidence changes. Card-purpose edits withdraw
any previous inventory/completeness confirmation until re-reviewed.

## Compatibility and verification

New sources use `pdf-statement-rows-v2` and card source format 2. Existing
`pdf-text-lines-v1`, CSV records, approvals, report payloads and source hashes are
preserved. Schema 30 is a compatibility gate preventing an older application from
writing the new source format. Reopening and encrypted backup recovery replay the
minimal versioned evidence and validate that it reproduces the original rows.

Synthetic parser/storage/UI/native tests cover fine-print exclusion, wrapped rows,
column/sign/date ambiguity, editable defaults, exact draft totals, no automatic
approval, atomic purpose rollback, history, restart/recovery and client isolation.
No installer is produced for this stage unless explicitly requested.
