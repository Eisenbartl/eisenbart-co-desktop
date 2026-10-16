# Windows development installer — 0.46.0

The current preview includes the redesigned client directory, in-app client tabs,
twelve focused workspace sections and review subtabs. Existing financial tools,
imports, immutable report evidence, encrypted backup/recovery, password changes
and configurable inactivity locking are retained. Schema is 29.

## Download and install

[Download the verified 0.46.0 Windows x64 preview](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38950497634/artifacts/12069289452)
(about 213.4 MiB). Extract the ZIP to find
`Eisenbart Counseling Suite_0.46.0_x64-setup.exe`, `README.txt` and `SHA256SUMS.txt`.
GitHub sign-in with repository access may be required. The artifact expires
October 30, 2026 at 19:20 (UTC); expired artifacts require another build.

[Build and Windows validation](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38950497634)
passed for source `cc29e7764b2a6bfd9ca6cad181438dbfc5deab49`. That commit differs
from the natively tested application source `3cba5da` only in bundled download
instructions. SHA-256 was checked again after downloading the package:

```text
c398a20641ba396466f7c13473f6c5df7d0b2935a9107c0954fc9f0ac22a922c  Eisenbart Counseling Suite_0.46.0_x64-setup.exe
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

The packaging workflow ran locked frontend builds, all 271 UI tests, all 247
Windows library tests and both PDF-worker tests on Windows Server 2025 x64.
It built the NSIS installer, installed it in a private test directory, checked a
responsive installed x64 native window, stopped its test process, uninstalled,
and removed only its own data. Its log explicitly records that no setup/password/
client workflow was entered and offline installation was not tested. Download
metadata and checksums were generated after successful package checks.

Linux validation separately covers the complete native counseling, statement,
report, privacy, restart and encrypted recovery workflows, plus focused redesign
checks and desktop/narrow visual inspection. See [executed evidence and source
scope](development-status.md). Automated Windows launch checks do not establish a
complete installed counseling workflow, clean Windows 11 offline installation,
independent security/accessibility review or commercial readiness.

## Clean Windows 11 evaluation still needed

Use a disposable fictional workspace to check an offline installation and upgrade,
setup/wrong passwords, client tabs/isolation, planning and source approvals,
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
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri -- build --ci --bundles nsis -- --locked
```

The output is under `src-tauri\target\release\bundle\nsis`. Building downloads
dependencies and the offline WebView2 redistributable; essential installed
operation is offline. Do not disable TLS, signatures or encryption to fix a build.
A manual GitHub workflow run with `package_installer=true` performs packaging;
ordinary source pushes only test. A green source-only run has no installer.
