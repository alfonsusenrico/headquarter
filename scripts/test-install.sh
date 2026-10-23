#!/bin/sh
# test-install.sh: runs scripts/install.sh against made-up release archives in a
# temporary home folder: install, a checksum mismatch, and uninstall. Needs only sh,
# tar with gzip, and (macOS) codesign. Run on macOS directly; on Linux, for example:
#   docker run --rm -v "$PWD:/src:ro" debian:12-slim sh /src/scripts/test-install.sh
# The checks are strings run by `check` with eval, so they expand there, not here.
# shellcheck disable=SC2016,SC2034
set -eu

repo=$(cd "$(dirname "$0")/.." && pwd)
root=$(mktemp -d)
trap '[ -n "${KEEP:-}" ] || rm -rf "$root"' EXIT INT TERM
failures=0

check() {
  if eval "$2"; then
    printf 'ok   %s\n' "$1"
  else
    printf 'FAIL %s\n' "$1"
    failures=$((failures + 1))
  fi
}

case $(uname -m) in arm64 | aarch64) arch=aarch64 ;; *) arch=x86_64 ;; esac
case $(uname -s) in Darwin) target=$arch-apple-darwin ;; *) target=$arch-unknown-linux-gnu ;; esac

sha256() {
  if command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1"; else sha256sum "$1"; fi
}

# A release folder like the workflow publishes, with stand-in programs.
release=$root/release
mkdir -p "$release" "$root/pack/hq-cli-$target" "$root/pack/hq-app-$target/png"
cat >"$root/pack/hq-cli-$target/hq" <<'EOF'
#!/bin/sh
case ${1:-} in
  --version) echo "hq 9.8.7" ;;
  stop) echo stopped >>"$HOME/hq-stopped" ;;
esac
EOF
cp "$root/pack/hq-cli-$target/hq" "$root/pack/hq-cli-$target/headquarter"
printf '#!/bin/sh\necho app\n' >"$root/pack/hq-app-$target/headquarter-app"
chmod 755 "$root/pack/hq-cli-$target/hq" "$root/pack/hq-cli-$target/headquarter" \
  "$root/pack/hq-app-$target/headquarter-app"
cp "$repo/assets/icon/Headquarter.icns" "$root/pack/hq-app-$target/"
cp "$repo"/assets/icon/png/*.png "$root/pack/hq-app-$target/png/"
for package in hq-cli hq-app; do
  archive=$package-$target.tar.gz
  tar -czf "$release/$archive" -C "$root/pack" "$package-$target"
  (cd "$release" && sha256 "$archive" >"$archive.sha256")
done

run() {
  HOME=$1 HQ_INSTALL_FROM=$2 PATH=/usr/bin:/bin:/usr/sbin:/sbin \
    sh "$repo/scripts/install.sh" "${3:-}" >"$1/out" 2>&1
}

# Install.
if [ "$(uname -s)" = Darwin ]; then searched=$(security list-keychains -d user); fi
home=$root/home
mkdir -p "$home/.config/headquarter"
echo kept >"$home/.config/headquarter/config.toml"
check "install succeeds" 'run "$home" "$release"'
check "hq and headquarter installed" '[ -x "$home/.local/bin/hq" ] && [ -x "$home/.local/bin/headquarter" ]'
check "PATH advice printed" 'grep -q "is not on your PATH" "$home/out"'
check "version read from the release" 'grep -q "Installed Headquarter 9.8.7" "$home/out"'
if [ "$(uname -s)" = Darwin ]; then
  app=$home/Applications/Headquarter.app
  check "bundle assembled" '[ -x "$app/Contents/MacOS/headquarter-app" ] && [ -x "$app/Contents/MacOS/hq" ] && [ -f "$app/Contents/Resources/Headquarter.icns" ]'
  check "bundle version" 'grep -q "<string>9.8.7</string>" "$app/Contents/Info.plist"'
  check "bundle signed" 'codesign --verify --deep --strict "$app" 2>/dev/null'
  keychain=$home/.local/share/headquarter/signing.keychain-db
  check "signed with this Mac's certificate" 'codesign -dvv "$app" 2>&1 | grep -q "Authority=Headquarter Local Signing"'
  check "signing keychain private" '[ "$(stat -f %Lp "$keychain")" = 600 ]'
  check "keychain search list untouched" '[ "$(security list-keychains -d user)" = "$searched" ]'
  certificate() { security find-certificate -Z -c "Headquarter Local Signing" "$keychain" | grep SHA-1; }
  first_certificate=$(certificate)
else
  check "app installed" '[ -x "$home/.local/bin/headquarter-app" ]'
  check "menu entry" 'grep -q "Exec=$home/.local/bin/headquarter-app" "$home/.local/share/applications/headquarter.desktop"'
  check "icons" '[ -f "$home/.local/share/icons/hicolor/256x256/apps/headquarter.png" ]'
  check "menu entry names the window class" 'grep -q "^StartupWMClass=headquarter$" "$home/.local/share/applications/headquarter.desktop"'
fi
check "manifest lists only home paths" '! grep -v "^$home/" "$home/.local/share/headquarter/install-manifest"'

# A second run updates in place.
check "reinstall succeeds" 'run "$home" "$release"'
if [ "$(uname -s)" = Darwin ]; then
  check "reinstall keeps the certificate" '[ "$(certificate)" = "$first_certificate" ]'
fi

# Uninstall.
echo session-data >"$home/.local/share/headquarter/state.db"
manifest=$(cat "$home/.local/share/headquarter/install-manifest")
check "uninstall succeeds" 'run "$home" "$release" --uninstall'
check "host stopped first" '[ -f "$home/hq-stopped" ]'
for path in $manifest; do
  check "removed $(basename "$path")" '[ ! -e "$path" ]'
done
check "settings kept" '[ -f "$home/.config/headquarter/config.toml" ]'
check "data kept" '[ -f "$home/.local/share/headquarter/state.db" ]'
check "folders named" 'grep -q "settings and sessions stay in" "$home/out"'

# A checksum mismatch installs nothing.
bad=$root/bad
cp -R "$release" "$bad"
printf '0000000000000000000000000000000000000000000000000000000000000000  hq-app-%s.tar.gz\n' \
  "$target" >"$bad/hq-app-$target.tar.gz.sha256"
fresh=$root/fresh
mkdir -p "$fresh"
check "mismatch fails" '! run "$fresh" "$bad"'
check "mismatch named" 'grep -q "does not match its checksum" "$fresh/out"'
check "nothing installed" '[ ! -e "$fresh/.local" ] && [ ! -e "$fresh/Applications" ]'

if [ "$failures" -gt 0 ]; then
  printf '%s check(s) failed\n' "$failures"
  exit 1
fi
printf 'all installer checks passed (%s)\n' "$target"
