#!/usr/bin/env bash
# Source this file before building or running the Tauri desktop application.
# Native Debian packages live in a writable sysroot; system directories are untouched.
export DESKTOP_SYSROOT="${DESKTOP_SYSROOT:-/workspace/shared/desktop-sysroot}"
export PATH="$DESKTOP_SYSROOT/usr/bin:${PATH}"
export PKG_CONFIG_PATH="$DESKTOP_SYSROOT/usr/lib/x86_64-linux-gnu/pkgconfig:$DESKTOP_SYSROOT/usr/lib/pkgconfig:$DESKTOP_SYSROOT/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export PKG_CONFIG_SYSROOT_DIR="$DESKTOP_SYSROOT"
export LIBRARY_PATH="$DESKTOP_SYSROOT/usr/lib/x86_64-linux-gnu:$DESKTOP_SYSROOT/lib/x86_64-linux-gnu${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LD_LIBRARY_PATH="$DESKTOP_SYSROOT/usr/lib/x86_64-linux-gnu:$DESKTOP_SYSROOT/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export CPATH="$DESKTOP_SYSROOT/usr/include:$DESKTOP_SYSROOT/usr/include/x86_64-linux-gnu${CPATH:+:$CPATH}"
export XDG_DATA_DIRS="$DESKTOP_SYSROOT/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export XDG_CACHE_HOME="${XDG_CACHE_HOME:-/workspace/shared/desktop-cache}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/workspace/shared/desktop-runtime}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
mkdir -p "$XDG_CACHE_HOME"
if [[ ! -d "$XDG_RUNTIME_DIR" ]]; then
  mkdir -p -m 700 "$XDG_RUNTIME_DIR"
fi
