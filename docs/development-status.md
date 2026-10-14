# Development status

## Recovery hardening: encrypted backup and restore — 0.6.0 source

Following the original staged specification, implemented manual encrypted local
backups and verified whole-workspace restore. The user selects a local/external
folder; unique, exclusively created `.ebcbackup` files never replace existing
backups. SQLCipher transaction snapshots include committed WAL records. The
container authenticates its header and ciphertext with domain-separated
HMAC-SHA256 derived from the workspace key; no password or raw key is persisted.

Restore is available on locked, first-setup, and storage-error screens. It verifies
the password/container, SQLCipher and SQLite integrity, relationships, schema,
and saved plan inputs, then displays the organization/counselor/client count.
An explicit RESTORE confirmation activates a new private generation through
one atomically replaced pointer. Previous files remain intact; records are not
merged. Candidate cancellation/expiry, conflicting operations, source changes,
invalid files, and migration/audit/activation failures preserve the active source.
Existing sessions lock if another instance changes the active generation.

Schema 7 preserves all earlier records and immutable audit history, adding
backup/restore events. Older backups migrate only in the recovery copy. Limits:
256 MiB database, 4 KiB header, and 1,000 clients. Backups require the original
password; there is no password reset. See [backup-recovery.md](backup-recovery.md)
for user steps, exact format, file layout, retained-copy limits, and durability.

| Check | Result |
|---|---|
| Locked setup/dependency fetch and production frontend build | Passed |
| Interface tests | 47 passed: backup folder selection, password clearing, warnings/errors, verified review/confirmation/cancellation, stale responses, and storage-error recovery to a clean locked screen |
| Linux Rust tests | Full suite 63 passed; final recovery-only rerun 10 passed after exclusive-staging/publication safeguards were tightened |
| Rust format and Clippy all targets with warnings denied | Passed |
| Compiled native desktop workflow | Passed: full existing client/financial workflows; encrypted backup; wrong backup password; process stop during verified review leaves original active; explicit restore retains original database bytes and all saved plans; fresh process detects missing metadata and recovers through the error screen |
| Native visual inspection | Passed for backup controls and authenticated restore review; native OS picker dialogs themselves were not exercised by WebDriver |
| Deferred Windows download metadata | Parsed successfully with PowerShell; packaging not invoked |
| Windows source checks | Run [38496639835](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38496639835) passed from 85f0c13: production frontend build, 47 interface tests and 64 Rust tests, including Windows file locks, pointer replacement, backup/restore failures, and SQLCipher memory-lock regression |
| New Windows installer | Intentionally not built, as requested |
| Clean Windows 11 installed/offline recovery and physical removable-drive/power-loss drill | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-recovery`; logs:
`/workspace/shared/recovery-native-smoke.log`, `recovery-backend-final.log`,
`recovery-backend-final-recovery.log`, and `recovery-frontend-final.log`.
Negative fixtures deliberately cause SQLCipher page-authentication diagnostics;
those checks passed by rejecting damaged data. No verification was disabled.
During implementation, compile errors in type inference/audit transaction usage
and fixture error formatting were corrected before tests passed. Final review
also tightened exclusive staging and ensured a failed temporary-pointer claim
cannot delete a pre-existing file. Snapshot sync handles have write access for
Windows FlushFileBuffers compatibility; Windows source validation is recorded separately.

Source commit `85f0c134adcc536bc0d4b2328f1c7cf119a38895` is uploaded to
main. Windows source CI passed, and installer build, install/launch/uninstall,
metadata preparation, installer upload, and validation-log upload were explicitly
skipped. No manual package run was requested. The full Windows source test log is
retained at `/workspace/shared/recovery-windows-validation.log`. This verifies
Windows core compilation and tests, not installed Windows GUI/picker/offline
recovery or real removable-drive durability.

Recoverable tracked-source and full-history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-recovery-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-recovery.bundle`, refreshed and verified
after the final validation notes commit. These are development source archives;
application data backups use the separate encrypted `.ebcbackup` format.

A durable backup survives a subsequent audit-write failure and reports that
warning; its snapshot contains prior audit history. A post-activation directory
sync failure reports an applied restore with a durability warning. Interruption
can leave incomplete backup files or unreferenced encrypted staging folders,
which are never selected automatically. Previous generations are retained;
supported retained-copy selection/pruning is future work. Future document/report
attachments will need inclusion in an extended format; none are stored yet.

Fictional data remains required while independent security review and clean
Windows acceptance are unfinished. The earlier 0.3.0 download does not include
this feature. Installer creation remains manual opt-in. Next: scoped synthetic
CSV import and explicit review/approval, preserving planning inputs separately.

## Stage 3D: paycheck-based cash-flow calendar — 0.5.0 source

Implemented persistent cash-flow plans with up to 20 separate accounts; exact
opening/running/closing balances; entered weekly, biweekly, semimonthly, monthly,
and irregular one-time schedules; dated bills/loan payments/external savings;
paired internal transfers; confirmed/estimated schedules; a calendar with daily
account review; and a chronological ledger. Dated entries are never manufactured
from monthly budgets or forecasts. Current budget/debt/savings revisions and
amounts are references for reviewing completeness, not automatic transactions.

The engine identifies negative opening, intermediate, and daily balances, bills
before the next entered paycheck, missing funding dates, and concentrated payment
days. Same-day debits precede credits conservatively, keeping shortfalls visible
even when daily/monthly totals are positive. Transfers preserve combined cash;
a reserve account never silently covers checking. Unknown amounts remain unknown
and propagate only to affected accounts. Dates respect leap years, original
monthly anchors, inclusive end dates, and semimonthly collision deduplication.

The counselor can edit a date and preview an alternative without saving. Preview
refreshes references, preserves the draft/baseline, and does not write records,
revisions, or audit events. Adjustments require actual payee permission and do
not guarantee bank processing. Encrypted schema 6 preserves all earlier plans
and immutable audit records; atomic saves and optimistic revisions reject stale
windows, failures roll back, and locking clears private state. Older applications
cannot reopen schema 6. See [cash-flow-calculations.md](cash-flow-calculations.md).

| Check | Result |
|---|---|
| Locked setup, production frontend build, compiled debug native desktop | Passed |
| Interface tests | 40 passed, including 7 cash-flow tests and client navigation/lock guards |
| Linux Rust tests | 53 passed, including 8 independent cash-flow calculation tests and 3 persistence/migration/atomicity tests; foreign firm/counselor scope also covers get/preview/save |
| Independent Python Decimal fixture | Checking -700 with combined +300 before payday; transfer conserves 1,800; closes checking 2,100 + reserve 1,200 = 3,300 |
| Rust format and Clippy all targets with warnings denied | Passed |
| Final native GUI workflow | Passed: account/activity entry, calendar/ledger, independently checked balances and next payday, preview without writing, client isolation, lock/wrong password/unlock, full process restart and original saved date |
| Final calendar visual inspection | Passed after widening cells and adding horizontal scrolling to keep amounts readable in narrow client panels |
| Deferred Windows download metadata | Parsed with PowerShell; packaging not invoked |
| New Windows installer | Intentionally not built, as requested |
| Windows source checks | Run [38492199152](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38492199152) succeeded from 75d3934: production frontend build, 40 interface tests and 54 Rust tests passed, including the Windows-only SQLCipher memory-lock regression |
| Clean Windows 11 offline installed cash-flow workflow | Not run |

Final native artifacts: `/workspace/shared/desktop-smoke-cashflow-verified`.
The initial native run stopped on a transient read-only text GET connection
closure after setup. Driver logs confirmed the connection closed before the
response completed. The harness now retries one GET observation, matching its
existing one-retry locator handling; typing, clicks, saves, and JavaScript are
never replayed. The rerun and final layout run passed the full required workflow.
An initially incomplete TypeScript test fixture was corrected before the successful
production build; one ambiguous test locator was narrowed to the account-name field.

Source commit `75d393425d5db79b10913aa14abf70e5dd619048` is uploaded to main.
Windows CI explicitly skipped installer build, install/launch/uninstall checks,
metadata preparation, installer upload, and validation-log artifact upload.
No manual package run was requested. The full Windows test log is preserved at
`/workspace/shared/cashflow-windows-validation.log`. This confirms Windows source
build/tests, not a newly installed Windows GUI or clean-machine offline acceptance.

Recoverable source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-cashflow-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-cashflow.bundle`; the archive contents
and full-history bundle are verified and refreshed after the final notes commit.
These are development source checkpoints, not backups of client workspace data.

Source-only checkpoint; no installer, bank integration, actual transaction
reconciliation, overdraft fees, holds, holiday shifts, or delayed transfer
settlement. Only entered accounts/activity are projected. Fictional records
remain required while supported encrypted recovery and Windows acceptance are
unfinished. The earlier 0.3.0 download contains neither savings nor cash flow.

## Stage 3C: emergency savings and sinking funds — 0.4.0 source

Implemented persistent client savings plans: emergency targets for 1/3/6/custom
whole months of necessary expenses; liquid/restricted/bill/sinking-fund reserve
separation; current emergency coverage; remaining amounts; capped monthly
contributions and estimated completion dates; dated sinking funds with suggested
contributions rounded upward; alternative emergency contributions; and monthly
budget/debt funding review. The counselor can record priorities and adjust
allocations; the application never moves money or silently rewrites other plans.

Reserve buckets partition one liquid pool, preventing double counting. Unknown
inputs remain unknown; missing expense bases or zero contributions do not invent
goal dates. Month-end contributions respect actual deadline days and leap years.
The last transfer is capped and stops at the target. Joint funding checks reserve
the higher of budget/debt payments and include future debt changes/extras, even
after savings finish. Overallocations and conflicting/missing references are
flagged. Scenario affordability is unavailable when it cannot be checked.
See [savings-calculations.md](savings-calculations.md) for assumptions and examples.

Encrypted schema 5 preserves clients, notes, budgets, debts, and immutable audit
history. Writes/audit are atomic, optimistic revisions reject competing windows,
failed loads cannot replace a plan, and locking removes private UI state.
Refreshing budget/debt references keeps unsaved savings drafts and cannot adopt
another window's changed revision. The older versions cannot open an upgraded
schema 5 workspace.

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 32 passed: exact inputs, native forecasts, target choices, goals/scenarios, safe reference refresh, failed loads/saves, missing affordability, stale responses, and lock/leave guards |
| Linux Rust tests | 42 passed: independent reserve/coverage/rounding/deadline/last-cent/scenario examples, zero/missing/funded states, leap days, past dates, caps/horizon, debt events, scope, competing writes, audit rollback, and schema 1/2/3/4 migrations |
| Independent Python Decimal examples | Confirmed remaining 3,899.99; 200/month requires 20 months with final 99.99; 100/month needs 39 months; 300/month needs 13; 200 over 3 eligible months requires 66.67/month; coverage floors to 0.61 |
| Rust format and Clippy, all targets with warnings denied | Passed |
| Final compiled native desktop workflow | Passed: visible savings/reserve/goal/scenario editing, known independent results, cap at final transfer, client isolation, lock/wrong password, and full process restart with saved savings |
| Windows source checks | Run 38489872140 succeeded from e3e06dc; conclusion confirmed during the cash-flow stage after GitHub access returned |
| New Windows installer/package | Intentionally not built, per request to save packaging time |
| Clean Windows 11 installed/offline savings acceptance | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-savings-final`. The initial
run passed financial checks but lost one read-only native-driver locator connection
at relaunch. The harness now retries that read once; typing, clicks, and saves
are never replayed and all workflow assertions remain required. The final run
passed fully. An incorrect expected last-transfer value in one fixture was
corrected to 99.99 after independent Decimal verification; the calculator already
returned that correct value.

The Windows workflow now always runs frontend/backend checks on normal pushes
and pull requests. Installer creation, installer smoke checks, metadata, and
artifact upload require a manual dispatch with `package_installer: true`.
The default is false. This preserves source validation without creating downloads
for each milestone. The 0.3.0 installer remains the earlier download and does
not contain the savings feature. This milestone remains for fictional records,
with no interest/returns, inflation, withdrawals, changing savings expenses,
automatic funding redistribution, recovery, or independent security certification.

### Preserved checkpoint and remaining CI confirmation

Source commit `e3e06dc84a031fdc8e98605ee3d410e323db408e` was successfully pushed
before authentication stopped working. Windows source-check run
[38489872140](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38489872140)
was observed with successful frontend checks and the backend step in progress.
Later authenticated API reads returned `HTTP 401: Bad credentials`; a fresh
uncached request also failed, and normal Git remote access could no longer
authenticate. Runtime status exposed no configured credential refresh capability.
No credential values were inspected or printed, no alternate identity was used,
and no interactive login or replacement token was requested.

At the end of this savings stage, the final Windows result and skipped-package
step records were unverified; this was not a reported build/test failure.
During the cash-flow stage, normal injected Git/CLI access returned and run
38489872140 was confirmed successful. The local notes are included in the next
source push. The workflow definition
makes installer/package upload steps conditional on manual opt-in, and no such
run was requested. Local checks passed and the complete source was already
uploaded. Final validation notes were committed locally for later synchronization.

Recoverable source/history checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-savings-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-savings.bundle`. They are development
source archives, not backups of client workspace data. Recheck CI and synchronize
any local documentation commits when GitHub authentication is available again.

## Stage 3B: monthly debt payoff planning — 0.3.0

Implemented saved client debt accounts and exact fixed-monthly APR payoff
projections, with comparable snowball/avalanche strategies. Each month reserves
entered minimums, allocates extra money by strategy, caps payments at the amount
owed, and rolls excess to the next account. Plans support known monthly fees,
one-time extra payments, and future changes to the total monthly payment budget.
The interface exposes estimated payoff months, interest/fee/payment totals,
remaining balances, and an interactive per-account monthly schedule.

Money uses exact cents; APR uses exact 0.0001-percentage-point units and 128-bit
intermediate multiplication. No money calculation occurs in React. Missing
active terms or unmodeled interest methods block payoff projections while still
allowing incomplete accounts to be saved. Shortfalls stop before unaffordable
payments; growing balances, horizon/safety limits, unverified terms, and saved
budget-capacity/allocation conflicts are flagged. Partial totals cover only
simulated months, with no invented debt-free date. Both strategies share all
payment events and assumptions. See [debt-calculations.md](debt-calculations.md).

Debt inputs are encrypted in schema 4 with firm/counselor/client scope, optimistic
revisions, atomic writes/audit, and rollback on failure. The migration preserves
clients, notes, budgets, and immutable audit history. Calculated projections are
recomputed and do not overwrite budgets or historical transactions. Private
editor state clears on lock; failed loads and delayed client/session responses
cannot become replacement plans. Unsaved debt edits participate in leave prompts.

Executed checkpoint checks:

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 25 passed: input strings/revisions, supported/unknown terms, schedule inspection, events, failed loads/saves, stale responses, unsaved prompts, and lock privacy |
| Linux Rust tests | 31 passed: independent amortization/comparison/final-payment examples, zero interest, fees/events, minimum shortfall, negative amortization, horizon/model limits, rate/date validation, ties, budget context, scope, competing windows, failed writes, and schema 1/2/3 upgrades |
| Independent Python Decimal fixtures | Single loan: 58.98 interest, 1,058.98 paid, 11 months; two debts: snowball 43.10 interest/643.10 paid, avalanche 35.35 interest/635.35 paid, both 7 months |
| Native desktop build and workflow | Passed on the initial complete compiled bundle: visible two-debt editing, independent strategy totals/allocations, budget coexistence, client isolation, lock/wrong password, and full process restart |
| Final Rust format, Clippy (all targets, warnings denied), backend and native checks | Passed on final source after the allocation-warning improvement |
| Windows 0.3.0 build/installer | Passed run 38485910521: 25 frontend/32 Windows backend tests, release build, silent install, responsive x64 native window, uninstall, and isolated cleanup |
| Clean Windows 11 offline/full installed debt workflow | Not run |

Final native artifacts are at `/workspace/shared/desktop-smoke-debt-final`; the
initial run is retained at `/workspace/shared/desktop-smoke-debt`.
This is a development milestone for fictional accounts. It implements fixed
monthly estimates, not daily accrual, variable/promotional APRs, changing lender
minimum formulas, or nonmonthly contractual payments. Source/account-detail
expansion, actual due-date cash flow, savings goals, recovery, and independent
review remain open. The 0.3.0 upgrade prevents older 0.1.0/0.2.0 apps from opening
schema 4; keep using the newer version and preserve the local data folder.

### Verified 0.3.0 Windows download

[Download the debt-planning development installer](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38485910521/artifacts/11859772481).
[Run 38485910521](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38485910521)
succeeded from source commit `631719ba5811c869b4190603c8eddcef53d82b41`.
The package is `Eisenbart Counseling Suite_0.3.0_x64-setup.exe`; its artifact
expires October 27, 2026 and may require GitHub sign-in with repository access.

The actual downloaded ZIP matched GitHub's digest and passed ZIP integrity. The
installer matched `SHA256SUMS.txt`; its README matched the source commit, version,
debt features, and schema-upgrade notice. Downloaded logs confirmed 25 frontend
and 32 backend tests passed, including the real Windows lock-quota regression.
The separate package log confirmed installation, responsive x64 native window,
uninstall, file removal, and removal of only test-created application data.
Archive inspection also verified the bundled WebView2 offline runtime payload.

Installer SHA-256:
`3965d9f9af6a425b1afa0b0ba39cb93ec776da95fd520e194755184bc8745914`.
Installer size: 220,951,701 bytes. ZIP size: 221,001,487 bytes.
Installer artifact: `11859772481`; validation artifact: `11859682545`.
Verified local downloads/logs: `/workspace/shared/windows-downloads/run-38485910521`.
Recoverable source checkpoints are retained at
`/workspace/shared/checkpoints/eisenbart-debt-source.tar.gz` and
`/workspace/shared/checkpoints/eisenbart-debt.bundle`; these are source/history
archives, not backups of client workspaces.

The installer is unsigned and for fictional accounts only. Windows CI launched
the installed native window but entered no client/budget/debt workflow and used a
runner with preinstalled WebView2. Linux native workflow evidence and Windows
backend/package checks do not establish clean Windows 11 offline or full
installed debt-workflow acceptance. Those checks remain open.

## Stage 3A: exact client budgets — 0.2.0

Implemented the first financial-core checkpoint: one persistent USD budget per
client, custom category/source lines, take-home income frequency normalization,
necessary/discretionary spending, proposed savings, planned debt payment
allocations, and signed surplus/deficit results. Amounts and calculations use
integer cents in Rust; the interface receives exact decimal strings. Weekly,
biweekly, semimonthly, monthly, annual, and irregular monthly-estimate inputs are
supported. Missing amounts and estimates are visibly flagged. Original planning
amounts/frequencies remain stored, and no historical transactions are changed.
See [budget-calculations.md](budget-calculations.md) for rounding, formulas,
limits, and independently checked examples.

Budget saving is authenticated and scoped to the current firm/counselor's client.
Encrypted schema 3 adds budgets and preserves earlier clients, notes, and audit
history. Saves and content-free audit events are atomic, with rollback on failure.
Revision checking prevents another desktop window's older plan from overwriting
a newer one. Failed loads cannot become empty writable replacements. Unsaved
edits participate in leave-record prompts; editing hides outdated totals; locking
clears budget state; delayed responses cannot populate another client's editor.

Executed validation for this checkpoint:

| Check | Result |
|---|---|
| Locked setup and production frontend build | Passed |
| Frontend tests | 18 passed; includes exact input payloads, native result display, missing/zero distinction, estimated irregular income, failed loads, private-state clearing, and stale responses |
| Linux Rust tests | 20 passed; includes independent rounding/totals/deficit examples, invalid/large inputs, encrypted persistence, scope, competing connections, atomic rollback, and schema 1/2 upgrades |
| Independent Python Decimal calculation | Matched the documented household example: 3,250.00 income, 1,633.33 necessary expenses, 716.67 unallocated |
| Rust format and Clippy (`--all-targets`, warnings denied) | Passed |
| Native desktop build | Passed with bundled production frontend |
| Native desktop workflow | Passed: visible budget editing, exact six-line example, encrypted saving, client isolation, locking, wrong-password rejection, and complete process restart with persisted budget |
| Windows 0.2.0 installer | Passed run 38482971562: 18 frontend/21 Windows backend tests, release build, silent install, responsive x64 native window, uninstall, and isolated test-data cleanup |
| Clean Windows 11 offline/full installed budgeting workflow | Not run |

Native artifacts: `/workspace/shared/desktop-smoke-budget-final`. An initial
cloud native-driver attempt lost a connection while typing intake; the app
remained responsive for a failure screenshot. The harness now identifies the
failed method/endpoint without printing input payloads. A fresh final build/run
completed all workflow assertions without retrying commands or weakening checks.

Version 0.2.0 upgrades the local database to schema 3. Version 0.1.0 will refuse
that newer schema; switching back is unsupported. The development build remains
for fictional records only, with no supported backup/restore or password recovery.
This is not the full Stage 3 financial engine: optional budget methods, debt
payoff schedules, savings targets, cash-flow dates, and scenarios remain open.

Windows run [38482182798](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482182798)
finished application validation but failed test-data cleanup when WebView2 still
held `EBWebView/lockfile`. No installer download was uploaded from that run.
Cleanup retains both known-folder ownership checks and the disposable-runner
restriction, waits at most 45 seconds for I/O failures, fails immediately for
other errors, and still requires the test-created directory to be removed.
It does not kill unrelated browser processes or skip the cleanup check.

### Verified 0.2.0 Windows download

[Download the budget development installer](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482971562/artifacts/11857823874).
[Run 38482971562](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38482971562)
completed successfully from commit `ff5d2e2402f8e7561adfb96f710d5f2ec62fd579`.
The installer is `Eisenbart Counseling Suite_0.2.0_x64-setup.exe`.
GitHub sign-in may be required; artifact retention expires October 27, 2026.

The final downloaded ZIP matched GitHub's SHA-256 digest and passed ZIP
integrity. The installer matched the bundled checksum; its README matched the
exact source commit, version, budget features, and schema-upgrade notice.
Downloaded Windows logs independently confirmed all 18 frontend and 21 backend
tests passed, including the real memory-lock quota regression. The validation
log confirmed native installation, x64 executable/window, successful uninstall,
and removal of only test-created application data. The bounded cleanup
correction passed the complete rerun. NSIS archive inspection passed and found
the 214,090,448-byte WebView2 offline runtime installer payload.

Installer SHA-256:
`663f1c65e8c1919fffa91917590318a703e31f32541c151f29edb86874cb2cc5`.
Installer size: 220,903,103 bytes. ZIP size: 220,952,850 bytes.
Installer artifact: `11857823874`; validation artifact: `11858447750`.
Local verified files/logs: `/workspace/shared/windows-downloads/run-38482971562`.
The native budget implementation checkpoint is `84f474b`; the subsequent
`ff5d2e2` commit changes only Windows test cleanup and documentation.

This package remains unsigned and for fictional records. Windows CI entered
no client/password/budget workflow, and had preinstalled WebView2. Linux native
workflow evidence and Windows backend/package checks do not establish clean
Windows 11 offline or full installed budgeting acceptance. Those checks and
backup/restore remain open.

## Windows development installer milestone — October 13, 2026

**Download available:** [windows-installer ZIP](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38450281912/artifacts/11842130980).
The verified installer is `Eisenbart Counseling Suite_0.1.0_x64-setup.exe`, built
from source commit `370d4414db381edfe3686e1106d15ea6471e1238` on `main`.
[Windows run 38450281912](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38450281912)
completed successfully. The download expires October 27, 2026 and may require
GitHub sign-in with repository access. Extract the ZIP, read `README.txt`, and
run the installer. See [user-guide.md](user-guide.md) for first use.

Executed on the disposable GitHub-hosted `windows-2025` runner:

- Locked frontend production build and **11 frontend tests passed**.
- Locked MSVC Rust build and **12 backend tests passed**, including the real
  Windows memory-lock quota regression, encrypted persistence, integrity,
  incorrect-password rejection, and scope protections.
- Release NSIS installer built successfully with bundled WebView2 offline runtime.
- Silent installation returned exit code 0; the installed program's PE machine
  was AMD64 (`0x8664`). It displayed a responsive native window with the expected
  title. The isolated test stopped its own process, silently uninstalled with
  exit code 0, and verified removal of the installed files.
- Download instructions/checksums and separate validation logs uploaded.

The installer is **unsigned** and remains a development build for fictional
records. The Windows package test entered no password, counselor account, or
client workflow; it tested installation and native-window launch. The runner
already had a browser runtime. A clean Windows 11 offline install without
preinstalled WebView2, full installed client workflows, and upgrade/data-retention
acceptance remain unrun. No security certification or commercial readiness is
claimed.

Independent checks after downloading the actual artifacts:

| Artifact check | Result |
|---|---|
| GitHub installer ZIP SHA-256 | Matched GitHub's digest; ZIP integrity passed |
| Installer SHA-256 | Matched bundled `SHA256SUMS.txt` |
| Download README source commit | Matched the successful run's full source commit |
| Windows validation log | Confirmed installation, responsive x64 app, uninstall, and isolated cleanup passed |
| NSIS archive integrity | Passed 7-Zip archive test |
| Offline runtime payload | Found 214,090,448-byte `MicrosoftEdgeWebView2RuntimeInstaller.exe` inside the installer |

Installer SHA-256:
`d6faff55f9b5d9db6c8f6783ed64eae1ea5e17e587d50e15e30446f11f831408`.
The installer is 220,875,970 bytes; the download ZIP is 220,925,577 bytes.
Artifact ID: `11842130980`. Validation-log artifact ID: `11841143510`.
Local verified downloads and run logs are retained under
`/workspace/shared/windows-downloads/run-38450281912`.

### Windows crash correction preserved in this checkpoint

The initial Windows run
[38445920011](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38445920011)
built the frontend and Rust code but aborted its tests with a stack overflow;
that run produced no installer. Locked SQLCipher 4.14.0 source and its error log
identified recursive native logging: failed `VirtualLock` allocations log a
memory warning through a formatter that itself allocates through SQLite.

The Windows-only correction configures native log sources once before memory
security activates, excluding MEMORY while retaining CORE, PROVIDER, and MUTEX
diagnostics. Encryption, Argon2id, memory wiping, memory-lock attempts, integrity
checks, and application errors are preserved. Memory locking remains best effort;
native memory-lock failure warnings are omitted on Windows. The Windows-only
subprocess regression forces an actual lock-quota failure and now passes on
Windows. See [security.md](security.md) for this limitation and review needs.

After the correction, the existing 11 Linux backend tests passed again and the
compiled Linux desktop passed the complete synthetic setup/client/note/lock/
restart workflow again. Those artifacts are retained at
`/workspace/shared/desktop-smoke-windows-fix`. The Windows packaging scripts also
passed PowerShell parsing and synthetic guard/header/checksum checks before CI.

GitHub API, logs, and artifact downloads worked through the existing injected
authentication after the environment network settings were updated. No new
credentials were requested. Runtime configuration metadata still reports unknown
network enforcement; successful commands establish access to these endpoints,
not a general network-readiness claim.

## Foundation milestone — October 13, 2026

This earlier checkpoint combined specification stages 1 and 2: an established
repository and a functioning desktop foundation. Advanced financial modules had
not been started at that checkpoint. It was a development build for fictional
records only; the later budget milestone is recorded above.

Implemented:

- Tauri 2 desktop host, React/TypeScript interface, original application icons,
  bundled local assets, and Windows NSIS packaging configuration.
- First-run organization and counselor setup with editable Eisenbart & Co.
  default branding; local password unlock; manual and 15-minute inactivity lock.
- SQLCipher-encrypted SQLite storage, Argon2id key derivation, key/password/KDF
  memory cleanup, schema migrations, integrity checks, and guarded setup.
- Persistent client intake, contact details, household size, goals, lifecycle
  status, searching, filtering, and editing.
- Append-only counselor notes with authorship/timestamps, operational audit
  records without note/contact contents, and server-enforced firm/counselor scope.
- Real client counts and lifecycle summaries; no invented financial results.
- Session and client-switch guards that discard stale responses and clear private
  interface data when locked.
- Reusable cloud setup/activation, backend/frontend tests, native desktop smoke
  harness, Windows build workflow, security notes, and staged roadmap.

The original spreadsheet was unavailable and has not been verified or changed.
No real client information was imported.

## Executed checks

Machine: managed Debian 13 Linux x64, Node 24.19.0, Rust 1.99.0, GTK 3.24.49,
WebKitGTK 2.54.0. Native packages were verified against signed Debian metadata.
The writable native sysroot avoids privileged installation.

| Check | Result |
|---|---|
| `bash scripts/cloud-setup.sh` | Passed; locked npm installation and Cargo fetch, tool activation and native dependency checks |
| `npm run build` | Passed; TypeScript and bundled production frontend |
| `npm test` | Passed: 11 tests, including setup, editing, client-note isolation, lock clearing, and stale-session failures |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -- --test-threads=2` | Passed: 11 backend tests; persistence, encryption, wrong keys, validation, scope, append-only history, migrations, corruption and existing-file preservation |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `npm run tauri -- build --debug --no-bundle` | Passed; standalone Linux desktop binary with compiled frontend assets |
| Native GUI workflow | Passed with exit 0 on the final compiled bundle, including a complete app/driver process restart and record persistence |
| Windows executable/NSIS installer build | Passed on run 38450281912; automated install/native-window launch/uninstall passed |
| Clean Windows 11 offline install/use | Not run |
| Independent security review and commercial release review | Not performed |

Binary: `src-tauri/target/debug/eisenbart-counseling-suite`. This Linux development
binary is not a Windows executable or installer. `dist/` is generated frontend
output; opening it alone is not the desktop application.

The native test uses real Tauri IPC and compiled assets, not a browser mock. It
creates a fictional firm/counselor and two clients, saves an authored note, checks
the second client has no such note, edits status/goals, verifies locking clears
private DOM values, rejects a wrong password, unlocks, and checks records after
relaunch. The final test terminates all its private native app/driver processes
and starts a fresh driver against the same temporary data. Temporary application
data is removed afterward. Early test-harness issues with CSS-clipped identity
text and native WebKit session teardown were corrected using full identity values
and a full process restart. All final workflow assertions passed. No live test
processes or driver listeners remained.

Final native command, from the checkout after `source scripts/cloud-env.sh`:

```bash
WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
WEBKIT_DISABLE_DMABUF_RENDERER=1 LIBGL_ALWAYS_SOFTWARE=1 \
dbus-run-session -- xvfb-run -a -s '-screen 0 1280x900x24 -nolisten tcp' \
  bash scripts/cloud-native-run.sh python3 scripts/desktop-smoke.py \
  --artifacts /workspace/shared/desktop-smoke-foundation-final-process-restart
```

The native screenshot and driver diagnostics are retained at that artifacts
location. No development server was running or required for the tested app.

SQLCipher prints expected HMAC/decryption diagnostics in intentionally wrong-key
and corrupted-file checks. They contain no test passwords or client contents.
The tests exercise actual encrypted-page authentication; these diagnostics are
not failed test outcomes.

## Reusable environment configuration

The environment draft has saved `install_script` and `start_skill`. Installation
calls the repository's tested `scripts/cloud-setup.sh`; startup instructions cover
tool activation, builds, checks, and new-process native GUI validation. No new
secret bindings were required. Later Windows artifact validation used the
user-reviewed GitHub API, Actions log receiver, and Azure Blob download domains.

The trusted synthetic Linux GUI test needs an explicit WebKit sandbox exception
because this cloud container denies WebKit's nested sandbox. It remains confined
to test processes; product configuration and default helpers preserve the sandbox.
This does not validate Windows WebView2 isolation or certify security.

Draft saving does not publish the environment or demonstrate restoration in a
new task. Review and save the draft in environment settings, then publish the
environment through the product. The foundation source checkpoint is now
uploaded to GitHub. Retained cloud files, uploaded Git history, and restoration
in a newly created cloud task are separate outcomes; a fresh-task restore has
not been verified.

Recoverable source checkpoints are retained outside the checkout at
`/workspace/shared/checkpoints/eisenbart-foundation-source.tar.gz` (tracked
source) and `eisenbart-foundation.bundle` (Git history). These are development
source checkpoints, not backups of application client data. A fresh-task restore
has not been verified; retain or upload the source before relying on a new remote
clone.

## Next development priority

Encrypted backup/restore is implemented with authenticated files, retained
previous workspaces, and tested interrupted/failure recovery. Clean installed
Windows acceptance and independent security review remain prerequisites for
actual client use. The next application priority is **local CSV statement import
with provenance, duplicate checks, and explicit review/approval**. Approved
historical facts must remain distinct from planning estimates and rejected data.
Begin with synthetic CSVs, bounded parsing, exact amounts, a review queue, and
client-scoped encrypted records; add PDF extraction in a later checkpoint.

Advanced financial models, broader scenarios, PDFs, multiple counselor roles,
password change/reset, exports, licensing, signing, retained-copy selection,
scheduled backups, and commercial readiness remain incomplete. See
[roadmap.md](roadmap.md) and [windows.md](windows.md) for staged criteria.

## Distance to the complete product

Stages 1–2 provide the implemented architecture and desktop foundation; the
unavailable original workbook remains unverified. A tested Windows development
package also completes part of Stage 7. Stages 3–8 are still open:

| Specification stage | Remaining work |
|---|---|
| 3: Financial core | Exact budgets, fixed-monthly debt schedules, savings goals, and dated cash-flow calendars implemented; optional budget methods, advanced debt terms, delayed transfer settlement, and broader scenario engines remain |
| 4: Document processing | Local CSV/text-PDF imports, extraction, provenance, reconciliation, review and approval |
| 5: Dashboard and reports | Financial charts, counselor reports, client action plans, reproducible offline PDFs |
| 6: Security/commercial | Encrypted backup/restore implemented; clean Windows recovery, password safeguards, accounts/roles, branding, licensing and independent review remain |
| 7: Production packaging | Signed distribution, upgrade behavior, remaining administrator/recovery/support guides |
| 8: Final verification | Complete installed workflows, clean Windows 11 offline tests, usability and release acceptance |

These stages differ greatly in size and will be split into smaller tested
checkpoints. A stage count is not a percentage or a reliable calendar estimate.
The complete specified application remains substantial work; the development
installer is suitable for evaluating the implemented foundation.
