# Version 0.51.0 development-branch review

This is the retained 0.51.0 consolidation review. Current development also includes
the user-requested [0.52.0 financial planning subpages](financial-planning-subpages.md).
Use [development status](development-status.md) and current PR checks for current
validation; the original 37 commits and this historical boundary remain preserved.

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
| Password/recovery and inactivity policy (5 commits) | `4e29c96` through `d25f3a7` | Durable encrypted generation activation, old-password backup behavior, session invalidation, backend idle enforcement, rollback and crash recovery |
| Client directory and guarded workspaces (4 commits) | `3cba5da` through `8504f37` | Stable client IDs, scoped tabs, dirty/busy guards, lock privacy; old installer evidence remains historical |
| Spending and extraction (6 commits) | `333ecf4` through `0e59b05` | Approved-only exact totals, editable purpose history, conservative transaction candidates, original source retention and no fine-print purchases |
| Existing household planning (7 commits) | `bb519a2` through `d83b653` | Exact allocations, explicit source/member mappings, encrypted journal revisions, individual handoffs and no inferred relationships |
| Unified transaction review (9 commits) | `c56d014` through `b6bd43d` | Canonical bank/card rows, optional retained PDF, exact purchase amount/count shares, unknown methods, duplicate/reversal behavior, scoped source metadata |
| Health assessment and insights (6 commits) | `79434c7` through `af546e0` | Versioned explainable scoring, essential-data gates, approved observations, refund/transfer boundaries, evidence and non-writing existing-planner previews |
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

## Completed validation and evidence

| Check | Executed result |
| --- | --- |
| Source identity | 0.51.0 agrees across all six package/Cargo/Tauri inputs; mismatch rejection also verified with a temporary synthetic copy |
| Frontend | Production typecheck/build; all 322 UI tests in 44 files pass, including the final guard-readiness correction |
| Linux Rust | Complete 317-library-test matrix and both real PDF-worker integrations pass; no failures or ignored cases. Nineteen affected backup/password cases also pass after the platform correction |
| Windows source CI | [Successful corrected run](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/39059990292), code `e28d6e2`: all 322 UI / 318 library / 2 PDF-worker tests, version/format/Clippy/frontend/embedded debug build pass |
| Native Linux | Final no-bundle application: complete 119-checkpoint workflow, completion marker and exit 0; includes all earlier and current feature sections |
| Migration/recovery | Full matrix includes upgrades, rollback, unsupported newer versions and staged recovery; new populated schema-31 late-DDL failure, retry, idempotence and reopen case passes |
| Installer policy | All five gated packaging steps skipped; corrected Windows run produced zero installer artifacts |
| Review state | Original 37 commits preserved, main unchanged, no merge or auto-merge; final documentation commit is subject to current PR checks |

Windows has one additional memory-lock-quota test. Zero-test main/doc targets are
not counted as PDF-worker verification. The initial 316-test baseline also passed;
the new migration regression raised Linux coverage to 317. Later current-code full
matrix runs, exact commands and log paths are retained in the validation index,
rather than treating targeted reruns as a single full-matrix execution.

[PR #1](https://github.com/Eisenbartl/eisenbart-co-desktop/pull/1) and its
[current-head checks](https://github.com/Eisenbartl/eisenbart-co-desktop/pull/1/checks)
are the review entry points. The successful source run above binds the app/test/CI
changes; the final documentation head is checked again without an installer.
Evidence index: `/workspace/shared/branch-051-validation/validation.json`.

The complete native run is
`/workspace/shared/desktop-smoke-051-consolidation-fourth`. It retains the full
planning/report/recovery regression plus current extraction, unified review,
health, household, real one-minute inactivity and password-change/recovery flows.
No native section was skipped. The score and 800px screenshots were visually
inspected. Runtime/build inputs and binary stayed unchanged; the only source delta
during that run was the separately recorded UI acceptance-test readiness edit.
Driver ports closed before later Cargo commands. Accepted binary SHA-256:

```text
11f62461691d20c0bf5888c518f76bbe678b003eefb4392796bcea223d960f34
```

## Diagnosed failures and bounded corrections

- The [first Windows run](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/39046706118)
  passed 322 UI, 318 library and two worker tests, then the new Clippy gate rejected
  `unused_mut` in `backup::staging`. Platform-conditional bindings retain Unix mode
  0700 and the original Windows behavior; no warning suppression was added.
- The [second Windows run](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/39059470204)
  passed 321 of 322 UI tests and exposed a save-test race between updated markup and
  React's guard effects. The same dirty=false assertion is awaited, together with
  busy=false and an enabled saved Type selector. Single-write and exact recalculation
  assertions remain. All nine affected tests and the full 322-test suite pass.
- Initial native runs stopped after 9, 13 and 105 checkpoints at legacy bank-queue
  message/navigation/row-readiness assumptions. The harness explicitly selects the
  old queue where intended and checks the new unified default in navigation.
  Original financial/isolation/history assertions remain. Each rerun used a fresh
  fictional workspace, with no uncertain write replayed. Failed artifacts are
  retained separately; the complete fourth run passed on the final runtime code.

Vite still emits its existing >500 kB chunk advisory. Windows logs also retain
vendored OpenSSL debug-symbol and existing action-runtime deprecation advisories.
These are distinct from the corrected Rust warning; no build threshold, checksum,
TLS, encryption or test safeguard was weakened to obtain passing results.

## Documentation reconciliation

Current README, roadmap, prototype map and user guide identify 0.51.0/schema 32 and the
implemented household, unified-review and health workflows. The original bank
financial-dashboard arithmetic is labeled as its historical appendix; saved
reports are implemented, rather than described as the next stage. Windows docs
distinguish current source validation from the historical 0.47.0/schema-29 download,
remove packaging from ordinary build commands and warn against opening upgraded
records in the old application. Earlier milestone outcomes remain historical. Completed-stage design notes now
identify implemented password, category-rule and client-history follow-ups rather
than scheduling them again.

## Merge and release limits

Keep this branch unmerged until all validation is resolved and the review is
approved. A green CI run must match the reviewed source head; stale old Windows
runs are not evidence for 0.51.0. No merge or auto-merge is performed by this task.
Windows source builds do not establish a Windows 11 installed/offline counseling
workflow, security audit, accessibility review or commercial readiness. Unsupported
OCR/institution/loan adapters and advanced planning terms remain existing limits.
The health index is a counseling policy and withholds a score on missing essential
evidence. No installer or new household/business functionality is introduced.
