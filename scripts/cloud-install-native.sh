#!/usr/bin/env bash
# Install signed Debian desktop packages into a writable, isolated sysroot.
# No privileged package installation or writes to system directories are used.
set -euo pipefail
native_apt=/workspace/shared/desktop-apt
native_root=/workspace/shared/desktop-sysroot
mkdir -p "$native_apt/etc/apt.conf.d" "$native_apt/etc/sources.list.d" \
  "$native_apt/etc/preferences.d" "$native_apt/state/lists/partial" \
  "$native_apt/cache/archives/partial" "$native_root"
touch "$native_apt/state/status"
cat > "$native_apt/apt.conf" <<'APTCONF'
Dir::Etc "/workspace/shared/desktop-apt/etc";
Dir::Etc::Parts "apt.conf.d";
Dir::Etc::Main "apt.conf";
Dir::Etc::sourcelist "sources.list";
Dir::Etc::sourceparts "sources.list.d";
Dir::State "/workspace/shared/desktop-apt/state";
Dir::State::status "/workspace/shared/desktop-apt/state/status";
Dir::Cache "/workspace/shared/desktop-apt/cache";
Dir::Log "/workspace/shared/desktop-apt/log";
APT::Architecture "amd64";
APT::Sandbox::User "agent";
APT::Install-Recommends "false";
APT::Install-Suggests "false";
Acquire::Languages "none";
Acquire::Retries "2";
Acquire::https::Proxy "http://proxy:8080";
Acquire::https::CaInfo "/etc/ssl/certs/ca-certificates.crt";
APTCONF
cat > "$native_apt/etc/sources.list" <<'APTSOURCES'
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.pgp] https://deb.debian.org/debian trixie main
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.pgp] https://deb.debian.org/debian trixie-updates main
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.pgp] https://security.debian.org/debian-security trixie-security main
APTSOURCES
APT_CONFIG="$native_apt/apt.conf" /usr/bin/apt-get update
APT_CONFIG="$native_apt/apt.conf" /usr/bin/apt-get --yes --download-only install \
  --no-install-recommends libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev xvfb xauth libssl-dev libc6-dev \
  webkitgtk-webdriver proot
for archive in "$native_apt"/cache/archives/*.deb; do
  dpkg-deb --extract "$archive" "$native_root"
done
source /workspace/shared/desktop-env.sh
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1 ayatana-appindicator3-0.1 \
  librsvg-2.0 openssl
WebKitWebDriver --help >/dev/null
printf 'Desktop native dependencies are ready in %s\n' "$native_root"
