#!/usr/bin/env bash
# Relocate release WebKit subprocess paths without writing to system directories.
# For trusted cloud smoke tests only, callers may supply the sandbox override.
set -euo pipefail
source /workspace/shared/desktop-env.sh
exec proot \
  -b "$DESKTOP_SYSROOT/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1:/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1" \
  "$@"
