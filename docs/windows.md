# Windows build and installation guide

## What is available now

The verified **0.1.0 Windows x64 development installer is available** from [build 38450281912](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38450281912). Its 11 frontend and 12 backend tests, installation, responsive native-window launch, and uninstall checks passed on a disposable Windows Server 2025 runner. See [development-status.md](development-status.md) for the exact evidence and remaining acceptance work.

[Download windows-installer](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/runs/38450281912/artifacts/11842130980), sign in to GitHub if prompted, and choose **Extract All** for the ZIP. Read `README.txt` and run `Eisenbart Counseling Suite_0.1.0_x64-setup.exe`. Open **Eisenbart Counseling Suite** from the Start menu afterward. The download expires October 27, 2026. The [evaluation guide](user-guide.md) explains workspace setup, fictional clients, notes, locking, and troubleshooting.

The first target is Windows 11 x64. This is a development application for fictional data, not a production financial-record system. There is no password recovery or tested backup/restore yet.

## Get an installer from GitHub

The source foundation has been uploaded to the repository. To download a successful Windows development build:

1. Open [Windows desktop build on GitHub](https://github.com/Eisenbartl/eisenbart-co-desktop/actions/workflows/windows.yml). Sign in to GitHub with access to the repository if prompted.
2. Open an existing successful run for the desired checkpoint. If a fresh build is needed, choose **Run workflow** and the branch containing that milestone, then start it. A push to the configured branch may also start the workflow automatically.
3. Wait for the run to finish. A green result means frontend/backend tests and automated installation, native-window launch, and uninstall checks passed on a disposable GitHub-hosted Windows runner. It does not mean a person tested the complete client workflow or used the application offline on a clean Windows 11 computer.
4. Open that run and download the **windows-installer** artifact near the bottom of the page.
5. Extract the whole downloaded ZIP file into a folder. Read `README.txt`, then open the `.exe` inside to install the development application. `SHA256SUMS.txt` records its download checksum. The workflow retains artifacts for 14 days; an expired download requires a new successful build.

If no artifact appears, open the failed step's output and the **windows-validation-logs** artifact when present. Do not treat source files or a failed workflow as an installer. GitHub Actions access and usage limits depend on the repository's account.

## What the Windows package check proves

The workflow uses the GitHub-hosted `windows-2025` image, Node 24, stable MSVC Rust, x64 Visual Studio build tools, and Strawberry Perl when available. NASM is optional; the bundled OpenSSL source supports a build without assembly when it is unavailable. Locked dependency installation and Rust tests preserve the declared dependency versions. Tauri's default static Visual C++ runtime setting avoids requiring a separate runtime installer for the application itself.

`scripts/windows-package-smoke.ps1` runs only on disposable GitHub-hosted Windows runners. It refuses to proceed if the application's local data, installation, or process already exists. It validates the installer's Windows executable header and signature status, installs to a unique runner temporary directory, verifies the installed application is an x64 executable, waits for a responsive native window with the expected title, stops only its own application process, uninstalls, and verifies the installed files were removed. It cleans only application data created by that isolated run. It creates no counselor account, password, or client record. Its log is uploaded separately.

These checks exercise the actual NSIS package and installed executable. They do not verify the full client workflow, database recovery, or fresh-machine offline WebView2 installation. GitHub's Windows image already supplies development software and a browser runtime. The Windows 11 manual acceptance checks below remain required.

## Install and evaluate

The verified 0.1.0 package includes Microsoft's WebView2 runtime installer: archive inspection found the 214,090,448-byte `MicrosoftEdgeWebView2RuntimeInstaller.exe` payload and passed the installer archive integrity check. A build downloads that redistributable in advance. A fresh-machine offline test must still confirm it installs and works without preinstalled WebView2.

On a Windows 11 x64 evaluation computer:

1. Run the extracted installer and follow its steps. Administrator rights may be requested by Windows or the runtime installer.
2. Open **Eisenbart Counseling Suite** from the Start menu.
3. Create a fictional organization and counselor, then choose a long password you can retain securely.
4. Add a fictional client, edit the profile, add a note, and change the lifecycle status.
5. Lock the app and unlock with the password. Verify a wrong password does not unlock it.
6. Close and reopen the program, sign in, and confirm the client and note persist.
7. Disconnect internet access and repeat sign-in, creating/editing clients, notes, and reopening.

The development installer is unsigned until a signing certificate is obtained and configured. Windows may display an unknown-publisher or SmartScreen prompt. Verify the download came from the expected repository build; signing and clean-machine testing are prerequisites for public distribution.

Do not import real client information for this evaluation. Future financial calculators, statement imports, reports, and recovery features are not part of this milestone.

## Where local records live

The app uses Tauri's per-user local application-data directory, ordinarily `%LOCALAPPDATA%\com.eisenbart.counseling` on Windows. Paste that location into File Explorer's address bar to inspect the folder. Data belongs to the Windows user running the program and is not stored in the source repository. The files are `workspace.db` (encrypted records) and `workspace.json` (non-secret setup metadata).

The encrypted database and its setup metadata must stay together. Never manually edit them, send them to another person, or delete them as a troubleshooting shortcut. Reinstalling the program does not supply a forgotten database password. This milestone has no supported recovery path if the password is lost.

## Build locally on Windows

This section is for a developer or technical helper. Install:

- Node.js 24 with npm.
- A current stable Rust toolchain for `x86_64-pc-windows-msvc`.
- Microsoft Visual Studio Build Tools with **Desktop development with C++**, the MSVC compiler, and Windows SDK.
- The WebView2 runtime for development. The distributable installer is bundled separately during packaging.
- Strawberry Perl for the bundled SQLCipher/OpenSSL build. MSVC's `cl`, `nmake`, and `link` must be available in the developer terminal. NASM is optional; a source-build failure must be diagnosed rather than bypassing encryption.

Open a developer PowerShell terminal in the repository and run:

```powershell
npm ci
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

To package after the checks pass:

```powershell
npm run tauri -- build --ci --bundles nsis -- --locked
```

The expected installer output directory is `src-tauri\target\release\bundle\nsis`. Building uses the internet to download dependencies and the WebView2 offline redistributable. The installed core workflow is designed to work without those development services.

## Windows release acceptance still required

Before sharing a release, install on a clean Windows 11 x64 machine, including one without WebView2 and without internet access. Test setup, wrong-password handling, client isolation, lock/unlock, persistence after a restart, and uninstall/reinstall data behavior. Record actual results and failures. Complete recovery, security, accessibility, licensing, signing, and broader product criteria in [roadmap.md](roadmap.md) before calling the application production-ready.
