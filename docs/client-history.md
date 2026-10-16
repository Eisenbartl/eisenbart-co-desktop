# Client history and reviewed note corrections — 0.43.0

## Correcting a session note

Open a client and choose **Session notes**. Current note versions appear with
their author and date. **Review correction for note…** opens the original note
and every saved correction. Enter the corrected text and a reason, check the
original counseling record, then **Save reviewed note correction**. A correction
creates a new dated, authored note linked to the original; it never changes or
deletes the original text. Unchanged text, empty/control-character reasons,
foreign clients and stale versions are rejected. Failed saves keep your draft;
reload or close asks before discarding edits. Navigation checks unsaved changes
and active requests, and lock/client changes clear private page state.

A further correction targets the current version. Even a correction restoring
the original wording gets a new ID and time; prior versions remain historical.
The first bounded workflow allows 100 corrections per original note and 10,000
retained note versions per client, including originals and corrections. Notes
allow 8,000 characters; reasons allow 500. There is no silent edit, permanent
deletion or note archiving. The current-note list retains its existing unpaged
presentation; realistic-volume usability needs separate acceptance.

## Reports keep their original evidence

Saved reports retain the exact selected note ID, text, author and date, original
payload bytes and digest. A correction increases retained note count and flags
reports that captured note context for renewed review. Reverting wording does
not clear that historical flag. New report evidence offers the latest 100 current
note versions, with up to ten explicit selections. Superseded IDs cannot be
selected in a newly saved report, even if their original text is restored later.
The report's saved-note count includes retained correction versions, while the
selection list shows current heads. A stale evidence count requires explicit
reload. Report and client-action-plan financial calculations remain unchanged.

## Searchable history and records

**Clients → History & records** combines original notes/corrections, intake and
follow-up snapshot versions, bank/card document import metadata, saved scenarios,
counselor assessments and client action plans. Profile entries show creation and
the latest update only, clearly labeled; earlier profile values are not invented.
Individual transaction decisions remain in **Transactions → Activity history**.
The timeline is a read-only index, not a replacement for the full source record.

Search literal title, note text/reason, record ID or metadata; punctuation such
as `%`, `_` and apostrophes has no wildcard meaning. Filter by record type or
inclusive dates in America/Chicago. Records sort newest first, with a stable
identifier tie-break; pages contain at most 25 entries. Paging uses the last
applied filters and an exact source snapshot, even while you type another search.
If history changes, search explicitly from the first page. Open an event to read
its timestamp, original record ID and detail. **Open … section** opens the selected
client's existing workflow; use the displayed ID/history controls to locate the
specific original there. It does not automatically select a historical record.

The timeline is bounded to 20,000 events; it does not write, approve, recalculate,
export or delete records. Search state and private details are not stored in a
URL, browser storage or remote service. Delayed client replies are ignored and
mismatched replies rejected.

## Storage and acceptance

Schema 27 adds an immutable correction-link table with scoped original/previous/
new-note foreign keys and a new audit event, preserving existing note and report
formats. Correction save rechecks the current thread fingerprint under a write
lock, appends the note/link/audit and validates the resulting chain atomically.
Recovery validates scope, sequential versions, unique linear links, reason/text,
chronology and all original reports before activation. Migration from schema 26
is atomic. Older applications cannot reopen the upgraded workspace.

Backend fixtures cover independently expected correction/report bytes, stale and
foreign requests, rollback, reversion, read-only timeline paging/literal dates,
restart, valid encrypted recovery and rejection of damaged history. UI/native
coverage and actual executed results are in [development status](development-status.md).
No installer was requested. Windows source checks are separate from clean
installed Windows offline acceptance and commercial/security release work.
