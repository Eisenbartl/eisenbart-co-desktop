# Development status

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
| Windows 0.2.0 installer | Pending CI build and installer verification; the existing 0.1.0 download does not include budgets |
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

This checkpoint combines specification stages 1 and 2: an established repository
and a functioning desktop foundation. Advanced financial modules have not been
started. This is a development build for fictional records only.

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

Validate and deliver the 0.2.0 Windows package for the budget checkpoint. The
next application milestone is exact debt payoff schedules, followed by comparable
snowball/avalanche plans with documented interest/rounding/payment assumptions and
independently checked examples. Keep development scoped to fictional records.
Clean Windows 11 offline acceptance and encrypted backup/restore remain
prerequisites for real client use.

Encrypted backup/restore and a tested recovery procedure must be implemented
before using actual client records. Broader financial calculations, document import and
review, debt/savings/cash-flow planning, scenarios, PDFs, multiple accounts/roles,
password change/recovery, exports, licensing, installer signing, and commercial
readiness remain incomplete. See [roadmap.md](roadmap.md) for staged criteria and
[windows.md](windows.md) for Windows build and manual acceptance instructions.

## Distance to the complete product

Stages 1–2 provide the implemented architecture and desktop foundation; the
unavailable original workbook remains unverified. A tested Windows development
package also completes part of Stage 7. Stages 3–8 are still open:

| Specification stage | Remaining work |
|---|---|
| 3: Financial core | Exact client budget checkpoint implemented; optional budget methods, debt, savings goals, paycheck cash flow, and scenario engines remain |
| 4: Document processing | Local CSV/text-PDF imports, extraction, provenance, reconciliation, review and approval |
| 5: Dashboard and reports | Financial charts, counselor reports, client action plans, reproducible offline PDFs |
| 6: Security/commercial | Backup/restore, password safeguards, accounts/roles, branding, licensing and independent review |
| 7: Production packaging | Signed distribution, upgrade behavior, remaining administrator/recovery/support guides |
| 8: Final verification | Complete installed workflows, clean Windows 11 offline tests, usability and release acceptance |

These stages differ greatly in size and will be split into smaller tested
checkpoints. A stage count is not a percentage or a reliable calendar estimate.
The complete specified application remains substantial work; the development
installer is suitable for evaluating the implemented foundation.
