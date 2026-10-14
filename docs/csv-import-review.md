# CSV import and transaction review — 0.7.0 source

This checkpoint implements the first document-processing slice from the master
specification. Work offline with synthetic UTF-8 comma-separated statements for
one USD checking or savings account per file. This is a source checkpoint; no
new installer was requested. The earlier 0.3.0 download lacks this feature.

## Import a fictional statement

1. Open a saved client and find **CSV import and transaction review**.
2. Choose a local `.csv` file, or enter its full path, then click **Inspect CSV**.
   A bounded preview captures the file's bytes and SHA-256 checksum; inspection
   writes no document, transaction, account, revision, or audit record.
3. Select the existing statement account, or give a new account a distinct label
   and select Checking or Savings. Account numbers are never inferred.
4. Enter a statement period and opening/closing balances only when known.
   Leave missing values blank. These are counselor-entered metadata, not verified
   institution figures. Use plain signed decimal USD amounts with at most two
   decimal places for edited amounts and balances.
5. Select the actual Date, Description, and Amount columns explicitly. Choose
   either signed amounts or separate Debit/Credit columns, and ISO, MDY, or DMY
   dates. For signed amounts, deposits are normally positive and withdrawals
   negative; inversion requires the visible checkbox. Optional merchant,
   category, type, reference, and running-balance columns can also be mapped.
6. Decide whether to authorize retaining the complete original CSV. The checkbox
   starts unchecked. Mapped originals and provenance are retained in either case;
   the complete source is retained only with this authorization, inside SQLCipher.
7. Click **Import into review queue**. No records are automatically approved.
   Canceling a preview discards transient bytes and does not create an import.
   Captured bytes expire after 15 minutes or locking; inspect again when needed.

Example synthetic input:

```csv
Date,Description,Amount,Category,Type
2026-09-01,Synthetic wages,2000,Wages,income
2026-09-02,Synthetic rent,-1200,Housing,expense
2026-09-03,Synthetic fee,-10.01,Fees,fee
2026-09-04,Synthetic refund,20.01,Housing,refund
2026-09-05,Synthetic reserve transfer,-200,Savings,transfer
```

The parser supports an optional UTF-8 BOM, quoted commas, quoted multiline fields,
CRLF/LF, and doubled quotes. Money accepts strict thousands groups, an optional
leading sign and dollar symbol, or parentheses for a negative value. Dates use
an explicitly selected format with a four-digit year, bounded to 1900–2100.
Malformed encoding/quoting, inconsistent columns, controls, or oversize files
are rejected. Unknown or invalid values remain unknown and need clarification.
A single blank Debit/Credit means no opposite movement only when the other cell
is numeric; both blank is unknown and both nonzero is conflicting.

## Make decisions

The four states are **Pending**, **Needs Clarification**, **Approved**, and
**Rejected**. Search descriptions/merchants/categories and filter by account,
document, date, type, or state. Unknown dates are hidden by date filters.

Review an individual row to see immutable reported values, extraction method,
source ID, logical CSV record (header is 1), physical line, uncertainty flags,
possible duplicate IDs, and decision history. Edit the verified fields, classify
its type/category, and select a decision. Unknown amounts/dates, incomplete
fields, unclassified types, or invalid income/expense signs cannot be approved.
Corrections and uncertain approvals need acknowledgment and a reason; refunds
and external balance movements need individual review. Reversing an approval
needs a reason, withdraws its effect immediately, and preserves history.

Select checkboxes to bulk-approve 1–100 unchanged, validated Pending rows.
Transfers, refunds, balance movements, corrected rows, duplicates, invalid fields,
and statement discrepancies are ineligible. Every row is checked again in Rust;
a failed batch changes none of its rows. Stale windows cannot overwrite a newer
revision. If another window changes this client, preserve any notes about your
edits, leave/reopen the client to load its current data, then review again.

Exact same-file imports for the same account are rejected. Date/amount/normalized
name overlaps and reused references across that account's unrejected records
produce advisory duplicate warnings, comparing both original and corrected
fields. Legitimate repeated payments can be approved individually with a reason;
the application never decides that matching rows must be duplicates. At most 20 related record IDs are shown per row to keep large overlapping imports bounded.

## Transfers, totals, and statement checks

Use **Reconcile an internal transfer** to select two equal, opposite, nonzero
amounts from different client accounts. Verify both sources and enter a reason.
Posted dates may differ. Both legs are approved together and excluded from
household income/spending while contributing to their own account activity.
Reversing either leg clears both links and withdraws both approvals atomically.
External transfers and loan/card principal payments require individual balance
movement classification so purchases are not counted again as expenses.

Totals use exact integer cents and only Approved rows across **all imported
dates**; table filters do not change totals. Income and positive interest add to
income; expenses, fees, and negative interest add to spending. Refunds reduce
net spending and never become income. Transfers and other balance movements
change account activity without becoming income/spending. Pending, clarification,
and rejected rows contribute zero. These facts do not rewrite planning inputs.

Statement checks compare entered opening plus all known source amounts against
entered closing. Unknown amounts/balances make the result unavailable. A gap
flags rows for individual review. After every row is Approved or Rejected, a
second check uses approved values; excluding a fee can leave a visible difference.
Running-balance source cells are retained for review; row-by-row reconciliation
is not implemented. These checks cannot establish bank verification.

Independent example: approve wages 2,000, rent -1,200, fee -10.01, and refund 20.01
then reject the fee with a reason. Income is 2,000; spending 1,200; refunds 20.01;
net spending 1,179.99; net activity 820.01. Pair a -200 checking movement with +200
savings: checking activity is 620.01, savings 200, combined still 820.01.
Checking opening 1,000 originally closes 1,610 with all source movements; after
rejecting the fee, approved closing is 1,620.01, a disclosed 10.01 difference.

## Storage, recovery, and limits

Schema 8 scopes documents, accounts, rows, choices, and history to the
client/organization/counselor. Immutable source records preserve mapped original
strings, extracted fields/issues, file hash/name, mapping, account, entered
period/balances, and source locators. Dedicated immutable encrypted history keeps
before/after decisions, fields, transfer links, reviewer, timestamp, and reason;
generic audit events contain opaque identifiers rather than financial details.
Imports/decisions/revisions/audit writes commit together or roll back together.

Authorized complete CSV sources are encrypted database BLOBs and are included
in existing encrypted backups alongside provenance and decision history. Restore
validates approved totals and reciprocal transfer links. Lock clears transient
source previews and private interface state. No network service or active source
content is invoked. Source strings, including markup, are rendered as text. Spreadsheet-like prefixes (=, +, @) in mapped text fields are flagged for individual review.
The source file itself remains where the counselor selected it; the application
cannot encrypt or remove an original outside its private workspace.

Limits: 2 MiB/file, 2,000 rows/file, 64 columns, 120 bytes/header, 2,048 bytes/cell;
20 accounts, 50 documents, 5,000 rows, and 10,000 history entries/client. Limits
fail visibly, preserving prior records. Whole-workspace backup limits also apply.
Earlier applications cannot open upgraded schema 8; use 0.7.0 or newer.

Text-PDF extraction is the next priority. Institution adapters, card-specific
parsers, inferred recurring-payment groups, scanned-document OCR, multi-currency,
source deletion/export, row-level running-balance checks, finalized reports,
clean installed Windows acceptance, and independent security review remain open.
