# Windows validation — current 0.51.0 source; historical 0.47.0 installer

Current source is **0.51.0, schema 32**. No current installer is requested or
provided. The 0.47.0 download below is a historical artifact with schema 29; it
does not contain newer extraction, household, unified-review or health features.
Do not open a schema-32 workspace in that older application. Preserve backups and
use the current source build after upgrading.

The [0.51.0 review record](branch-0.51-review.md) tracks fresh Windows source CI,
migrations, full tests and Linux native acceptance separately from historical
packaging. Windows CI now checks matching version inputs, formatting, frontend,
Rust tests, Clippy and an embedded debug application with **no bundle**. Packaging
steps remain gated by an explicit manual `package_installer=true`; leave it false.
A green source run does not establish Windows 11 installed/offline acceptance.

## Historical 0.47.0 preview and executed packaging checks


The historical 0.47.0 preview adds client **Spending Analysis**: approved/categorized
spending cards, category charts, transaction filters, discussed Type changes and
supported opportunity prompts. Original imports and classification history are
retained; all affected live results refresh after saving. The directory, client
tabs and thirteen focused sections keep import/review separate. Existing planning,
reports, encryption and security remain available. Schema is 29.
[Feature usage and calculation rules](spending-analysis.md).

## Download and install

[Download the verified 0.47.0 Windows x64 preview](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38974473672/artifacts/12078199826)
(about 213.4 MiB). Extract the ZIP to find
`Eisenbart Counseling Suite_0.47.0_x64-setup.exe`, `README.txt` and `SHA256SUMS.txt`.
GitHub sign-in with repository access may be required. The artifact expires
October 30, 2026 at 22:38 (UTC); expired artifacts require another build.

[Build and Windows validation](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38974473672)
passed for source `25e9dafd6f16523ef4832f424ae8d1ff6f8dbd80`, the same application
source used by the successful focused and complete native checks. The downloaded
EXE, source/version instructions and checksum were verified again locally:

```text
e2d28f7a5265bf47f0028c8ddf312abcc42b46ff1a74b88e08d036ddae2d7da4  Eisenbart Counseling Suite_0.47.0_x64-setup.exe
```

1. Download the **windows-installer** ZIP and choose **Extract All** in File Explorer.
2. Read its `README.txt`, then run the extracted `.exe`.
3. Open **Eisenbart Counseling Suite** from the Windows Start menu.
4. Use fictional records and retain the password you choose. Follow the
   [evaluation guide](user-guide.md) for clients, planning, reports and recovery.

This is an unsigned development installer. Windows may show an unknown-publisher
or SmartScreen warning. No publisher certificate or commercial-readiness claim is
supplied. Essential operation is offline; the installer is configured to include
Microsoft's WebView2 offline redistributable.

## Updating an earlier preview

Close all application instances before installing. Preserve the existing
workspace, its password and a supported encrypted backup. If an old preview has
no backup controls, copy its entire closed-app data folder to a safe location
first. Never delete files to force setup to appear. This version migrates
supported older workspaces to schema 29; older apps may not read upgraded records.
Continue using the new version afterward.

The normal per-user data directory is `%LOCALAPPDATA%\com.eisenbart.counseling`.
`workspace-active.json` selects records under recovery generations. Keep the
entire directory intact; do not edit or move individual databases, metadata,
pointers or recovery files. Use supported encrypted backup and restore.

Changing the current workspace password creates a verified encrypted generation
and locks the app. Existing backups and retained originals keep their original
passwords. Restoring an older backup restores its password and captured settings.
There is no forgotten-password reset; reinstalling cannot recover a lost password.
Close older app versions before changing passwords or inactivity policy.

## What validation establishes

The packaging workflow ran locked frontend builds, all 281 UI tests, all 252
Windows library tests and both PDF-worker tests on Windows Server 2025 x64.
It built the NSIS installer, installed it in a private test directory, checked a
responsive installed x64 native window, stopped its test process, uninstalled,
and removed only its own data. Its log explicitly records that no setup/password/
client workflow was entered and offline installation was not tested. Download
metadata and checksums were generated after successful package checks.

Linux validation separately covers the complete native counseling, statement,
report, privacy, restart and encrypted recovery workflows, plus focused Spending Analysis
checks and desktop/narrow visual inspection. See [executed evidence and source
scope](development-status.md). Automated Windows launch checks do not establish a
complete installed counseling workflow, clean Windows 11 offline installation,
independent security/accessibility review or commercial readiness.

## Clean Windows 11 evaluation still needed

Use a disposable fictional workspace to check an offline installation and upgrade,
setup/wrong passwords, client tabs/isolation, Spending Analysis Type edits and
cancellation, exact recalculation/history, planning and source approvals,
original report evidence, PDF export/viewer handoff, lock/restart, password changes,
inactivity policy and encrypted recovery. Test native file/folder dialogs,
wrong/damaged backups, retained originals, error-screen recovery and uninstall/
reinstall data behavior. Preserve failures and their steps. Scanned PDF OCR,
advanced financial terms and other planned requirements remain open.

## Build instructions for a developer

Install Node 24, stable MSVC Rust, Visual Studio C++ tools/Windows SDK, WebView2
for development and Strawberry Perl for bundled SQLCipher/OpenSSL. NASM is optional.
Use the repository checkout and locked commands:

```powershell
npm ci
npm run build
npm run check:version
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run tauri -- build --debug --no-bundle --ci -- --locked
```

The debug executable is under `src-tauri\target\debug`. No installer is created.
Building requires dependency downloads and development WebView2 prerequisites.
Do not disable TLS, signatures or encryption to fix a build. Ordinary pushes to
main and PRs targeting main run source validation; manual dispatch also defaults
to no packaging. Only an explicitly requested run with `package_installer=true`
may build/install-test an NSIS package and the offline WebView2 redistributable.
