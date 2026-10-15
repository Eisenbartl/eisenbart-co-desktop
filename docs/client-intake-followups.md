# Client intake and follow-ups

Introduced in 0.24.0 source. Version 0.25.0 adds the [practice dashboard overview](follow-up-overview.md), which opens a selected item directly in this panel.

Open a saved client and expand **Intake and follow-ups**. Mailing address,
household composition, employment status, employer, occupation and income
context are optional. These are counselor-entered notes, separate from verified
statement facts, financial model inputs and PDF report content. Existing contact
information, household size, goals, lifecycle and session notes remain available.

Add a task or appointment with an explicit title and due date. Appointments may
also have a time. Dates and times use America/Chicago; the saved reminder counts
compare open items with the current date in that zone. Past open items are
overdue, open items dated today are due today, and completed/cancelled items
generate neither count. Refresh the record to update the review date. Dates are
accepted from 1900-01-01 through 2100-12-31. Times use exact HH:MM values.

Save to persist edits. Record an outcome in the item's notes, then mark it
completed or cancelled as appropriate. Saved items cannot be removed; unsaved
draft items can be removed. A rejected save keeps the draft visible. **Discard
changes and reload intake** explicitly replaces it with the latest stored version.
Client switching respects the existing unsaved-change guard; locking clears
private intake and tasks. Reads and writes reject stale responses for another
client, and other saves temporarily disable conflicting editing controls.

Reminders are available within the opened client record and, from 0.25.0, on the
dashboard across clients. The application does not run Windows/background
notifications, create recurring appointments, contact clients, sync calendars
or configure time zones.
Those are separate workflow work. Earlier intake versions are retained for
integrity/recovery; this stage does not add a history browser or export them in
financial PDFs. No financial or client action-plan history is rewritten.

## Storage and recovery

Schema 17 adds `client_care_snapshots`. Each canonical JSON version belongs to an
authenticated organization, counselor and client. Rust assigns new task IDs,
validates input and dates, requires the current revision and preserves existing
task identities. Saving appends a version and its audit event in one transaction.
Database triggers reject snapshot updates/deletion. A failed audit or stale edit
leaves the earlier version intact. Current limits are 100 tasks/appointments,
500 saved versions per client and 128 KiB per version. Optional text fields and
individual titles/notes have additional documented input limits in the form.

Migration from schema 16 is transactional and leaves original clients, notes,
models and reports intact. Older snapshots/backups migrate without inventing
intake data. Encrypted recovery validates every version's canonical data,
contiguous revisions, timestamp and retained task identities before activation;
a corrupted older version cannot hide behind a valid latest record. Preserve an
encrypted backup and password before upgrading; the 0.23.0 application must not
be used against a schema 17 workspace. The existing downloadable 0.23.0 installer
does not contain this source stage.

## Validation

Focused Rust checks cover optional fields, leap dates/time bounds, exact due
counts, authenticated client scope, real second-window conflicts, append-only
history, audit rollback, schema migration rollback, reopening, encrypted restore
and rejection of corrupted earlier history. Interface checks cover exact save
payloads, errors and retained drafts, explicit discard/reload, delayed reads and
saves, disabled controls and lock/navigation privacy. The native workflow saves
fictional intake, an appointment with explicit time, an overdue task, completion
and cancellation; checks a second client's empty record; and repeats assertions
after process restart, encrypted restore and missing-metadata recovery. Actual
run results and platform limits are recorded in [development status](development-status.md).
