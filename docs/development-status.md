# Development status

## Windows packaging — in progress

The source foundation was uploaded to GitHub as commit `8a5430b` on `main`.
The first actual Windows run is
[38445920011](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38445920011).
Its frontend build and 11 frontend tests passed. Rust compilation succeeded, but
the test process aborted with a Windows stack overflow before installer creation;
no installer artifact was produced by that run.

The error log and locked SQLCipher 4.14.0 source identified recursive native
logging: the Windows stderr formatter allocates through SQLite; a failed
`VirtualLock` allocation logs a memory warning, causing another allocation and
the same warning. The Windows-only fix configures native log sources once before
memory security activates, excluding MEMORY while retaining CORE, PROVIDER, and
MUTEX diagnostics. Encryption, Argon2id, memory wiping, memory-lock attempts,
integrity checks, and application errors are preserved. Memory locking remains
best effort; native memory-lock failure warnings are omitted on Windows.

A Windows-only subprocess regression forces a real lock-quota failure and checks
encrypted allocation, integrity, persistence, incorrect-password rejection,
unchanged files, and successful reopening. Its full Windows execution is pending.
The existing 11 backend tests passed again on Linux after this change.
The standalone Linux desktop was rebuilt and its full native setup/client/note/
lock/restart smoke test passed again. Artifacts are retained at
`/workspace/shared/desktop-smoke-windows-fix`.

The packaging workflow has been prepared with explicit x64 MSVC prerequisites,
locked builds, and install/launch/uninstall checks. New PowerShell scripts verify
the executable architecture, guard existing installations/data, test a package
only on a disposable Windows runner, and generate a download README and SHA-256
checksum. PowerShell 7.6.6 parsing and synthetic safety/header/checksum fixture
checks passed on Linux; that is not Windows installer execution.

GitHub's API is now reachable using the existing injected authentication. The
environment draft includes `api.github.com`,
`results-receiver.actions.githubusercontent.com`, and `*.blob.core.windows.net`,
preserving the package-manager preset. The latter two are GitHub's documented
log/artifact download destinations. API and log downloads now work using the
existing injected authentication; no additional credentials were requested.

No Windows download is claimed while installer creation and validation remain
incomplete. Clean Windows 11 offline acceptance and full product functionality
remain separate requirements.

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
| Windows executable/NSIS installer build | Initial Windows CI reached a failed Rust test step; installer build was skipped |
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
secret bindings or network domains were required.

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

Build the configured installer on Windows, install it on a clean Windows 11 x64
machine, disconnect internet access, and repeat setup/client/note/lock/restart
checks. Preserve this foundation before beginning the next application milestone:
exact money types, documented income-frequency normalization, and a tested budget
calculation workflow.

Encrypted backup/restore and a tested recovery procedure must be implemented
before using actual client records. Financial calculations, document import and
review, debt/savings/cash-flow planning, scenarios, PDFs, multiple accounts/roles,
password change/recovery, exports, licensing, installer signing, and commercial
readiness remain incomplete. See [roadmap.md](roadmap.md) for staged criteria and
[windows.md](windows.md) for Windows build and manual acceptance instructions.
