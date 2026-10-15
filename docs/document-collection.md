# Document collection checklist — 0.28.0 source

Open **Documents → Collection checklist** and choose a client. Add a request,
enter its title and document type, optionally select an existing statement
account, requested period, due date and notes, then choose **Save document
checklist**. Missing dates and accounts stay unknown.

Edit a saved request to mark it **Received**, entering an explicit received date.
Optionally link a source already imported for the same client. If an account is
selected it must match that source. Receipt can also be tracked without importing
unsupported document formats or retaining any file. Receipt does not approve,
extract or apply financial data; continue that work in the separate import and
review pages.

Use **Archive this request** and save to remove it from active requests. Select
**Archived**, edit and uncheck Archive to reactivate it. Only an unsaved new
request can be removed. Saved requests retain their identifiers and history.
Filters change the view, not saved data. Counts describe the saved version.

**Checklist version** opens a read-only snapshot, including when the selected
number happens to be the latest version. Choose **Current checklist** to edit.
History preserves original titles, receipt status, dates, notes and linked IDs.
Account/source names come from the currently stored source inventory. Imported
sources remain immutable; the checklist does not attach a second copy of files.

Unsaved drafts are guarded when switching page/client or locking. Writes are
blocked during loading/saving. Failed saves retain the draft and are never
retried automatically. A stale revision requires explicit discard/reload before
saving again. Refresh rereads the current checklist. The application remains
offline, and stored requests/history are included in encrypted backups.

## Persistence and limits

Rust authorizes the client, validates dates/text/status/source ownership and
assigns new request IDs. Schema 18 adds append-only checklist snapshots and an
audit event. Each save checks the expected revision and commits data and audit
atomically. Saved IDs cannot be removed or imported from another client.
Read and restore validate canonical data, references, version continuity and
retained IDs; corrupt older history blocks recovery before activation. Migration
is transactional, including migration-failure rollback.

The current bounds are 100 requests including archived records, 500 saved
versions per client, 128 KiB per snapshot and 1,000 available statement accounts
or imported sources per client. Title: 200 characters; notes: 2,000 characters;
dates: 1900–2100. Dates are date-only; timestamps display in America/Chicago.
No dependencies or financial models changed. Older schema-17 applications cannot
open schema 18: retain encrypted backups and use the newer application.

This stage does not add OCR, document uploads/email requests, automatic receipt
matching, standalone account editing, financial approval, final report eligibility
or permanent deletion. Those features retain their entries in the
[development map](prototype-development-map.md). No new installer is created.
