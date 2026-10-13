# Managed cloud development

These instructions target the selected Debian 13 managed cloud machine. The Windows user does not need to perform them to run a finished installer. Native Linux validation helps development; it does not replace Windows installation, packaging, or clean-machine offline acceptance.

## Reproduce setup

Use the existing isolated checkout. Do not create a Git worktree unless explicitly requested.

```sh
cd /workspace/eisenbart-co-desktop
bash scripts/cloud-setup.sh
source scripts/cloud-env.sh
```

`cloud-setup.sh` installs or reuses a stable Rust toolchain and native desktop dependencies, installs locked npm packages, fetches locked Cargo dependencies, and prepares the native WebDriver test tools. It requires development network access to obtain packages. Signed Debian packages are downloaded and extracted into a writable sysroot under `/workspace/shared/desktop-sysroot`; privileged installation into system directories is unnecessary. Existing usable dependencies are reused.

The helper files installed under `/workspace/shared` and dependency/build caches can be retained between tasks. Shell activation and running processes are not retained. Source `scripts/cloud-env.sh` in each new shell before native builds or tests. The script selects the shared Rust/cache locations, native headers/libraries, and a default of four concurrent Cargo build jobs without changing `HOME`.

The scripts are specific to the managed machine's Debian packages and configured proxy. Preserve TLS, repository signatures, and package checksums. Do not reuse them unchanged on a different Linux distribution or a Windows machine.

## Build and test

After setup and activation, run:

```sh
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

For local desktop development with a display, `npm run tauri dev` starts the React development server and native host. This cloud machine has no ordinary interactive Windows desktop. The native smoke script uses a virtual Linux display and drives the actual Tauri interface through WebDriver; it does not substitute a browser-only preview or call application commands directly.

`scripts/desktop-smoke.py` exercises fictional organization/counselor setup, client creation and edits, isolated notes, lock/unlock, incorrect-password rejection, and reopening the native process to check persistence. It creates isolated temporary data directories and keeps screenshots/logs outside the checkout. Use it only with synthetic test records. Its final exit status must be checked; starting a driver or creating an artifact directory is not a passing result.

## Native runtime and test exception

`scripts/cloud-native-run.sh` uses `proot` to expose WebKit subprocesses from the writable native sysroot at the paths expected by the packaged Debian libraries. The wrapper preserves the WebKit sandbox by default. Normal Windows application code and installer configuration do not include a sandbox bypass.

The managed Linux container may prevent WebKit's additional sandbox from starting. A trusted, synthetic-data-only smoke run can require the explicit `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1` environment variable for that test process, together with Xvfb and a private D-Bus session. This weakens isolation for that Linux test process and must never become a product default, a general startup instruction, or a way to test untrusted imported documents. Actual executed commands and results belong in [development-status.md](development-status.md).

Environment setup configuration should save the installation script and new-task activation/startup steps. Saving that draft does not execute it or prove that a new cloud task has reproduced the environment. Check the final status report for which configuration was saved and which validation actually ran.
