#!/usr/bin/env bash
# Reproduce development dependencies in the managed Debian 13 cloud machine.
set -euo pipefail
cloud_checkout="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$cloud_checkout"
mkdir -p /workspace/shared/tooling /workspace/shared/npm-cache
export CARGO_HOME=/workspace/shared/cargo
export RUSTUP_HOME=/workspace/shared/rustup
export npm_config_cache=/workspace/shared/npm-cache
export PATH="$CARGO_HOME/bin:$PATH"
if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
  curl --fail --show-error --silent --location https://sh.rustup.rs \
    -o /workspace/shared/tooling/rustup-init.sh
  sh /workspace/shared/tooling/rustup-init.sh -y --profile minimal \
    --default-toolchain stable --no-modify-path
fi
rustup component add rustfmt clippy
install -m 644 scripts/cloud-native-env.sh /workspace/shared/desktop-env.sh
install -m 755 scripts/cloud-native-run.sh /workspace/shared/desktop-run.sh
source scripts/cloud-env.sh
if ! pkg-config --exists gtk+-3.0 webkit2gtk-4.1 ayatana-appindicator3-0.1; then
  bash scripts/cloud-install-native.sh
fi
if [[ ! -x "$CARGO_HOME/bin/tauri-driver" ]]; then
  cargo install tauri-driver --version 2.1.0 --locked
fi
npm ci --no-fund --no-audit
cargo fetch --manifest-path src-tauri/Cargo.toml --locked
rustc --version
node --version
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1
