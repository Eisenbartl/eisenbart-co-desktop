# Client transaction activity history — 0.38.0

Open **Transactions → Activity history** and select a client. The page reads the
existing encrypted bank/card sources and immutable review decisions. It does not
create transactions, change decisions, calculate totals or replace the existing
review queue. No migration, dependency or installer is needed.

Search covers event descriptions, merchants, categories, types, reasons, source
filenames, current/original account names, original descriptions and exact
record/decision IDs. Matching is literal, case-insensitive substring matching;
percent/underscore are ordinary characters. Separate filters select bank/card,
current account identity, exact source, transaction dates and historical review
state. Apply filters with Search; edited inputs do not relabel an existing page.
Clear filters returns to the complete timeline.

Each import and subsequent decision is a separate event, ordered by actual event
time descending, with a stable event ID tie-breaker. Date filters use the
transaction date recorded at that event, inclusively; unknown dates are excluded
when either date bound is set. Status describes that event, while details
separately label the current row status and whether the event is latest. An old
approval can therefore appear under Approved even after a later rejection.

Expand an event to inspect its counselor, reason, old/new state, corrected values,
original raw reported values, extraction warnings, source SHA-256, original and
current account identity, archive state, statement period, retention choice,
record/physical line/PDF-page locator and exact decision ID. Original captured
cells remain readable even when original file bytes were not retained. An
unassigned card's entered source name is not a durable reviewed account identity.

Show all events for this record searches its source and exact record locator.
Current bank transfers expose the other leg; repayment context exposes the exact
captured counterpart approval and whether its binding remains current. Open
counterpart history and Open linked decision perform another scoped read. Links
are explicitly **latest stored relationship context**, not a claim that this
relationship existed at every historical event. Removed relationships are still
available in their existing account/link review history; this page does not add
new historical relationship events or invent missing old transfer-link fields.

Bank signs describe cash movement. Positive card amounts increase liability;
negative card amounts reduce it. Repeated events must never be summed as spending.
Financial figures remain owned by the existing approved-data calculators.

## Bounds, consistency and privacy

Rust reauthorizes the selected client against the authenticated organization and
counselor, and reads bank/card/account/link data in one read transaction. It uses
existing validated store bounds: 5,000 bank rows, 10,000 bank decisions, 20 card
sources of up to 200 rows, and 4,000 card decisions. Sorting/filtering operate on
bounded references and compact event fields; raw evidence is copied only for the
25 returned events. Existing source readers also verify bounded history and card
bytes/digests. Invalid filters, unavailable clients and unreadable evidence fail
without writes or a misleading partial timeline.

Pages carry a fingerprint of client, shared financial revision, card-link version
and visible source/account metadata. Later financial decisions or account/link
changes reject the old page token; Refresh activity starts at page one. Tokens
are consistency markers, not authorization credentials. Search replies are
ignored after navigation/client changes; lock unmounts private history. No
confidential data or filters are stored in a URL or browser local storage.

This is transaction history, not the future combined client timeline, note
amendments, general audit browser, deletion workflow, classification rules, or a
financial export. Essential operation remains offline.

## Verification

Backend fixtures independently check 41 mixed events and a 25/16 page split,
shared import timestamps, original/corrected cents, historical/current states,
literal search, inclusive dates/unknown exclusion, account/source/status filters,
reciprocal transfer legs, current/obsolete repayment approvals, renamed archived
cards, stale tokens, read-only database change counts, ownership/lock, restart,
encrypted recovery and unreadable-source failure. UI tests cover applied filters,
paging errors/retry, exact evidence navigation, unknowns/escaping and late replies.
The complete native workflow exercises this page over the earlier recovered
synthetic bank/card records. Actual validation outcomes are recorded separately
in [development status](development-status.md).
