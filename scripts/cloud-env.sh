#!/usr/bin/env bash
# Source in the managed Debian cloud environment; never changes HOME.
export CARGO_HOME=/workspace/shared/cargo
export RUSTUP_HOME=/workspace/shared/rustup
export npm_config_cache=/workspace/shared/npm-cache
export PATH="$CARGO_HOME/bin:$PATH"
source /workspace/shared/desktop-env.sh
