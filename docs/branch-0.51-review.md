# Version 0.51.0 development-branch review

## Scope and branch policy

Review branch: `client-workspace-redesign-0.46`, targeting `main`.
Original validated-development tip: `af546e008469d66e5084b11dfc04625f2b7e79b8`.
Main/base: `2c104af9a907f10c0f8fc3981d6cada8c2120c30` (0.43.0, schema 27).
Initial comparison is **37 ahead, 0 behind**, 135 changed files. Consolidation
adds CI, migration regression and documentation commits without rewriting those
37 commits, adding features, merging or enabling automatic merge. Application
version stays **0.51.0**, schema **32**. No installer is authorized.

## Suggested review order

| Change group | Existing milestones | Reviewer focus |
| --- | --- | --- |
| Password/recovery and inactivity policy | `4e29c96` through `d25f3a7` | Durable encrypted generation activation, old-password backup behavior, session invalidation, backend idle enforcement, rollback and crash recovery |
| Client directory and guarded workspaces | `3cba5da` through `8504f37` | Stable client IDs, scoped tabs, dirty/busy guards, lock privacy; old installer evidence remains historical |
| Spending and extraction | `333ecf4` through `0e59b05` | Approved-only exact totals, editable purpose history, conservative transaction candidates, original source retention and no fine-print purchases |
| Existing household planning | `bb519a2` through `d83b653` | Exact allocations, explicit source/member mappings, encrypted journal revisions, individual handoffs and no inferred relationships |
| Unified transaction review | `c56d014` through `b6bd43d` | Canonical bank/card rows, optional retained PDF, exact purchase amount/count shares, unknown methods, duplicate/reversal behavior, scoped source metadata |
| Health assessment and insights | `79434c7` through `af546e0` | Versioned explainable scoring, essential-data gates, approved observations, refund/transfer boundaries, evidence and non-writing existing-planner previews |
| Consolidation | Subsequent commits on this branch | Version/format/lint/embedded-build CI gates, schema-31 late-failure rollback/idempotence regression, current docs and recorded final checks |

The aggregate original diff is roughly 20,456 additions / 922 deletions. Review
financial/storage changes before presentation. No production financial formula,
transaction store, authorization policy or schema is changed by consolidation.

## Schema and compatibility review

| Upgrade | Existing implementation | Required verification |
| --- | --- | --- |
| 27 → 28 | `password-change-schema.sql` | Audit extension preserves old values; failed extension leaves schema/history intact |
| 28 → 29 | `security-policy-schema.sql` | Encrypted policy history, legacy 15-minute default, atomic rollback and reopen |
| 29 → 30 | Compatibility gate only | Older apps cannot write versioned PDF evidence; original sources/reports unchanged |
| 30 → 31 | `households-schema.sql` | Household journal and audit extension, failure rollback and retained notes |
| 31 → 32 | `transaction-inspection-schema.sql` | Scoped append-only inspection metadata; late trigger-collision failure rolls back table/DDL/version; successful retry and repeated migration preserve all rows and audit values |
| 32 → 32 | No health migration | Idempotent reopen; recovery verifies supported journal/source/report histories |

The new schema-31 regression includes populated clients, notes, retained CSV BLOB,
saved assessment and existing household history. It compares all preexisting table
contents, full audit rows and failed-migration schema SQL; successful migration
adds an empty inspection table only, repeated migration changes nothing, and a full
lock/reopen retains the same source/report/household records. Older-schema migration,
rollback, unsupported-newer-version and staged encrypted recovery tests remain in
the complete Rust suite. This does not authorize a data downgrade. Preserve the
whole workspace and supported backups; older binaries cannot safely open schema 32.

## Fresh validation for this consolidation

- [x] Fetch branch/base and verify the 37-commit comparison with no divergence.
- [x] Locked cloud setup and all six build version inputs agree on 0.51.0.
- [x] Production frontend build and all 322 UI tests pass.
The initial complete Rust run passed **316 library tests and two supervised
PDF-worker integration tests** (427.05s). The new late-failure schema-31 regression
then passed independently; the final 317-test matrix is pending. Vite emitted its
existing advisory for chunks larger than 500 kB; type checking and bundling passed.
No bundle-size or test assertion was weakened.

- [ ] Final complete Rust matrix, PDF-worker integrations, formatting and Clippy.
- [ ] Current embedded Linux no-bundle build and complete native smoke workflow.
- [ ] Fresh Windows source CI, tests/migrations, lint and no-bundle build.
- [ ] Reconcile docs, commit results and verify the final remote review head.

Current working artifacts/log index:
`/workspace/shared/branch-051-validation`. Exact outcomes and CI links will replace
the pending items after execution. Historical focused 0.51.0 health acceptance and
0.50.0 native results remain recorded in their feature guides and are not counted
as a fresh complete native run here.

## Documentation reconciliation

Current README, prototype map and user guide identify 0.51.0/schema 32 and the
implemented household, unified-review and health workflows. The original bank
financial-dashboard arithmetic is labeled as its historical appendix; saved
reports are implemented, rather than described as the next stage. Windows docs
distinguish current source validation from the historical 0.47.0/schema-29 download,
remove packaging from ordinary build commands and warn against opening upgraded
records in the old application. Earlier milestone outcomes remain historical.

## Merge and release limits

Keep this branch unmerged until all validation is resolved and the review is
approved. A green CI run must match the reviewed source head; stale old Windows
runs are not evidence for 0.51.0. No merge or auto-merge is performed by this task.
Windows source builds do not establish a Windows 11 installed/offline counseling
workflow, security audit, accessibility review or commercial readiness. Unsupported
OCR/institution/loan adapters and advanced planning terms remain existing limits.
The health index is a counseling policy and withholds a score on missing essential
evidence. No installer or new household/business functionality is introduced.
