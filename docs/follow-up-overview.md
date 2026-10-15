# Practice appointments and follow-ups — 0.25.0 source

The dashboard now lists saved appointments and follow-up tasks across the
authenticated counselor's clients. Open items are shown by default. Choose
**Overdue**, **Due today**, **Upcoming (after today)** or **Completed / cancelled**,
optionally choose a kind or search a client name/title, then select **Apply
follow-up filters**. A changed filter does not silently change the displayed list;
the form tells you to apply it. Search is case-insensitive and limited to 100
characters. Archived clients are excluded unless **Include archived clients**
is checked and applied.

Counts cover all included clients, independently of the list's view, kind and
search filters. Only open items generate overdue/today/upcoming counts. Completed
and cancelled items remain available in the closed view. Dates use the current
America/Chicago date; an appointment earlier today remains due today, because
these are date-only reminders. A missing appointment time is explicitly shown as
**Time not set**, not interpreted as midnight. Refresh on a new day or after
changes made in another window.

The list sorts by due date, entered time, case-insensitive client name and internal
client/item IDs. Items without a time sort before timed items on the same day.
Duplicate client names remain separate records. Pages contain up to 25 items;
filter changes start at the first page, and a page beyond the current result set
is moved to the last available page. No results are silently truncated. The
overview supports up to 1,000 included clients with saved care records and the
existing 100-item per-client care limit. An exceeded bound or unreadable current
record produces an error instead of incomplete counts or a misleading empty list.

Choose **Open client** to open the exact saved item in **Intake and follow-ups**.
The panel opens, scrolls to that item and places keyboard focus in its title.
Review the current record, enter an outcome and save a completion/cancellation
there. The existing unsaved-change, scoped authorization and revision checks
still apply. A missing target is explained without selecting another item.
Returning to the dashboard loads current saved follow-ups with the default view.

This is an offline in-app overview. It does not send messages, schedule Windows
notifications, sync external calendars or change client lifecycle stages. Full
intake notes and follow-up outcome notes are not included in overview responses.

## Implementation and checks

Schema 17 and dependencies are unchanged. Rust uses one read query for matching
client identities and their latest care versions, so another window's writes
cannot mix identities and versions within one result. Organization and counselor
ownership are enforced in SQL. Current records reuse care validation; list reads
do not change snapshots, financial models or audit history. Date grouping,
filtering, stable ordering and pagination are performed in Rust.

The interface clears old rows during loading/failure, validates workspace and
filter identity, and ignores superseded or unmounted requests. Locking removes
the overview and any opened client draft. Core tests exercise exact day boundaries,
closed items, stable pagination, missing times, duplicate names, latest-version
selection, archived clients, foreign organizations/counselors, corrupt data and
unchanged audit counts. Interface and native checks cover explicit filtering,
empty/error states, selected-item navigation, draft guards and privacy. Native
checks repeat overview assertions through restart and both encrypted recovery
paths; actual results are recorded in [development status](development-status.md).

No installer is produced for this stage. The existing 0.23.0 Windows preview
predates both intake and this overview.
