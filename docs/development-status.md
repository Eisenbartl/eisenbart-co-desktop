# Development status

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

The debt checkpoint and its verified 0.3.0 Windows development package are
complete. The next application milestone is emergency savings coverage and sinking-fund
goals, with exact contributions/targets and explicit budget/debt allocation
coordination. Keep development scoped to fictional records. Clean Windows 11
offline acceptance and encrypted backup/restore remain prerequisites for real
client use.

Encrypted backup/restore and a tested recovery procedure must be implemented
before using actual client records. Broader financial calculations, document import and
review, advanced debt models, savings/cash-flow planning, scenarios, PDFs, multiple accounts/roles,
password change/recovery, exports, licensing, installer signing, and commercial
readiness remain incomplete. See [roadmap.md](roadmap.md) for staged criteria and
[windows.md](windows.md) for Windows build and manual acceptance instructions.

## Distance to the complete product

Stages 1–2 provide the implemented architecture and desktop foundation; the
unavailable original workbook remains unverified. A tested Windows development
package also completes part of Stage 7. Stages 3–8 are still open:

| Specification stage | Remaining work |
|---|---|
| 3: Financial core | Exact budgets and fixed-monthly debt schedules implemented; optional budget methods, advanced debt terms, savings goals, paycheck cash flow, and scenario engines remain |
| 4: Document processing | Local CSV/text-PDF imports, extraction, provenance, reconciliation, review and approval |
| 5: Dashboard and reports | Financial charts, counselor reports, client action plans, reproducible offline PDFs |
| 6: Security/commercial | Backup/restore, password safeguards, accounts/roles, branding, licensing and independent review |
| 7: Production packaging | Signed distribution, upgrade behavior, remaining administrator/recovery/support guides |
| 8: Final verification | Complete installed workflows, clean Windows 11 offline tests, usability and release acceptance |

These stages differ greatly in size and will be split into smaller tested
checkpoints. A stage count is not a percentage or a reliable calendar estimate.
The complete specified application remains substantial work; the development
installer is suitable for evaluating the implemented foundation.
