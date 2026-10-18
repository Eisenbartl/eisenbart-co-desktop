# Client history and reviewed note amendments — implementation reference

Implemented in 0.43.0/schema 27. See [usage and limits](client-history.md) and [executed validation](development-status.md). These original boundaries are retained for context. The former follow-up, [local workspace password change](workspace-password-change.md), is also implemented in 0.44.0. Follow [current development status](development-status.md) for new priorities.

Complete the existing **Clients → History & records** area with a bounded,
searchable client timeline and reviewed corrections to session notes. Reuse
original note storage, intake/follow-up history and existing report/source records;
do not duplicate transaction activity, financial calculators or report approval.

Preserve each original note's exact content, author, time and ID. Corrections
must be explicit append-only amendments with a reason and immutable versions,
not silent update/delete. Define how current note presentation, amendment history
and newly captured reports refer to versions before adding storage. Old reports
must retain their original selected note bytes, digests and source references;
changed notes should require fresh review without rewriting historical claims.

Scope every read/write to the authenticated counselor/organization/client in Rust.
Use stable bounded pages, literal/date/type filters, stale-snapshot rejection,
optimistic versions and atomic audit. Timeline navigation must open the existing
source workflow with guarded client/draft context. Do not expose other clients
through late replies, identifiers, URLs or browser storage. Archive/removal should
be added only with explicit retention semantics; no destructive source deletion.

Test independently expected timeline ordering and original/amended content,
malformed/foreign/stale writes, failed-audit rollback, legacy reports, lock/restart,
client isolation and encrypted recovery. Extend the full native suite through
visible controls and run Linux/Windows source checks. Keep one tested committed
stage; do not create an installer unless the user explicitly asks.
