# Reusable local category-rule CRUD — implementation reference

Implemented in 0.42.0/schema 26. See [usage, persistence and limits](local-category-rules.md) and [executed validation](development-status.md). The original stage boundaries below are retained for context. The former follow-up, [client history and reviewed note amendments](client-history.md), is implemented in 0.43.0. See [current development status](development-status.md) for new priorities.

0.41.0 implements read-only bank category suggestions and unsaved review drafts.
Reuse `classification.rs`, the existing merchant identity and audited transaction
review; do not rebuild classification, approval or financial calculations.

Add explicit counselor-owned rules with create, read, edit, archive/reactivate
and immutable history. Decide the first scope conservatively (client + account
+ known expense/fee type + exact normalized entered merchant/fallback text).
Categories are explicit counselor input, not guessed from another client or
remote model. Define conflict handling between rules and current approved
evidence; conflicting rules/evidence must not silently choose a winner. Rules
never auto-approve, bulk-write or change transactions/reports when saved.

Use authenticated Rust scope, optimistic versions, atomic audit and a tested
migration/recovery validator if persistent tables are needed. Bind each offered
rule suggestion to the exact rule revision and source/import context. Applying
it should still fill an unsaved review, with final explicit save through existing
transaction decision controls. Rule edit/archive must invalidate stale drafts
without rewriting prior decisions. Preserve historical rule provenance if a
saved decision references a rule; define the capture format before implementing.

Keep read-only approved-evidence suggestions working, including conflict/unknown
reasons and the current two-to-five evidence bound. Larger evidence paging is
separate unless completed in this stage with meaningful coverage. Do not turn
normalization into fuzzy numeric/punctuation stripping or infer income, transfer,
refund/card/debt roles. Keep new sources/counselors isolated and essential work
offline. Test CRUD, conflicts, malformed/duplicate rules, stale use, audit rollback,
archive/reactivation, no automatic financial writes, restart, legacy data and
encrypted recovery. Retain the complete desktop/source checks and do not create
an installer without an explicit user request.
