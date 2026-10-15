# Windows development installer — 0.23.0

Current development source is **0.26.0**, with schema 17 intake/follow-ups, a practice-wide
dashboard overview and selected intake in immutable internal reports (format 5). No new installer is created until requested.
The downloadable **0.23.0** preview below predates those changes. Do not open a
schema-17 workspace or format-5 reports with the older application.

## Download and install

[Download the verified 0.23.0 Windows x64 preview](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38668750314/artifacts/11943645145)
(about 213 MiB). The ZIP contains `Eisenbart Counseling Suite_0.23.0_x64-setup.exe`,
`README.txt` and `SHA256SUMS.txt`. This artifact expires October 29, 2026 (UTC).
[Build and Windows checks](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38668750314)
passed for source `bd2d1a88544ea941e69373f6312cf275d3767186`.

1. Download the **windows-installer** ZIP from the verified build link. Sign in to
   GitHub with access to the repository if prompted.
2. Right-click the ZIP and choose **Extract All**. Read the included `README.txt`.
3. Run the `.exe` in the extracted folder, then open **Eisenbart Counseling Suite**
   from the Windows Start menu.
4. Use fictional records and retain the password you choose. See the
   [evaluation guide](user-guide.md) for the client, scenario, report and backup steps.

This is an unsigned development installer; Windows may show an unknown-publisher
or SmartScreen warning. Confirm the build/version and repository before running
it. No certificate or signing claim is supplied. The artifact is retained for
14 days; an expired download requires a new build.

The current package includes budgets, debt/savings/cash-flow planning, encrypted
recovery, reviewed CSV/selectable-text PDF imports, recurring and balance review,
approved-data charts, saved scenarios and counselor/client-plan PDF export.
Temporary income periods show funding deficits/recovery and explicitly selected
paycheck changes. Scanned OCR, advanced terms, broader intake/follow-up and other
commercial requirements remain open. This is not a production release.

## Updating an earlier preview

Close the app before installing. Keep the existing workspace, its password and a
backup. If the old preview has no backup controls, copy the entire closed-app data
folder to a safe location first. Do not delete files to make setup appear again.
Version 0.23.0 uses schema 16 and migrates supported older workspaces; earlier
applications may not read upgraded or newly extended records. Keep using the new
version afterward. Reinstalling cannot recover a forgotten encryption password.

The normal per-user data directory is `%LOCALAPPDATA%\com.eisenbart.counseling`.
After recovery, `workspace-active.json` selects records under recovery generations.
Keep that entire directory intact; do not edit or move individual database,
metadata, pointer or recovery files. Use supported encrypted backup and restore.

## What validation establishes

The packaging workflow runs on a disposable Windows Server 2025 x64 runner. It
installs locked dependencies, builds/tests the frontend, tests Rust persistence
and PDF workers, builds the actual NSIS installer, installs it into a private
location, checks a responsive installed x64 native window, uninstalls it and
cleans only its own data. SHA-256 download metadata is produced after those steps.
The script refuses existing installations, processes or workspaces.

This does not establish a complete installed Windows counseling workflow, a clean
Windows 11 offline installation without preinstalled WebView2, or a security
review. The installer is configured to carry Microsoft's WebView2 offline
redistributable. Its inclusion and actual package results are recorded in
[development status](development-status.md). Linux native workflow checks and
Windows package checks are separate evidence.

## Clean Windows 11 evaluation still needed

Use a disposable fictional workspace to check setup, wrong passwords, client
isolation, budgets/debt/savings/cash flow, CSV/PDF approvals and original evidence,
recurring/balance review, approved dashboards, scenario periods, reports/client
plans, lock/restart and encrypted recovery. Repeat offline. Test native file/folder
dialogs, wrong/damaged backups, retained originals, error-screen recovery and an
upgrade from a copied fictional older workspace. Check uninstall/reinstall data
behavior separately. Preserve failures and their steps. Signing, accessibility,
security, realistic data volume and commercial acceptance remain release work.

## Build instructions for a developer

Install Node 24, stable MSVC Rust, Visual Studio C++ tools/Windows SDK, WebView2
for development and Strawberry Perl for bundled SQLCipher/OpenSSL. NASM is optional.
Use the repository checkout and locked commands:

```powershell
npm ci
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri -- build --ci --bundles nsis -- --locked
```

The output is under `src-tauri\target\release\bundle\nsis`. Building downloads
dependencies and the offline WebView2 redistributable; essential installed
operation is offline. Do not disable TLS, signatures or encryption to fix a build.
A manual GitHub workflow run with `package_installer=true` performs packaging;
ordinary source pushes only test. A green source-only run has no installer.
