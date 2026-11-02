#!/bin/sh
# Headquarter installer (add-release-installer D2-D4, split-host-and-app-install D2-D4).
#
#   curl -fsSL https://github.com/alfonsusenrico/headquarter/releases/latest/download/install.sh | sh
#   sh install.sh --host-only      the host for a server: hq, kept running as a user service
#   sh install.sh --app-only       the app only, a client of hosts elsewhere
#   sh install.sh --full           both again, after one of the above
#   sh install.sh --uninstall
#
# By default it installs both parts: the host (`hq` and `headquarter` in ~/.local/bin) and
# the app (Headquarter.app in ~/Applications on macOS; `headquarter-app` and a menu entry on
# Linux). Run again without an option, it keeps the parts it installed before. Every
# archive is checked against its SHA-256 before anything is installed. No administrator
# rights.
#
#   HQ_INSTALL_DIR   where the commands go (default ~/.local/bin)
#   HQ_VERSION       a release such as 0.1.0 (default: the latest)
#   HQ_INSTALL_FROM  a folder holding the release archives, instead of downloading
#   HQ_PROGRESS      1 adds, for the app's updater, a "progress <step>" line per step and a
#                    "note [ok] <text>" line per file it fetches, checks or installs
#   HQ_SERVICE       files: write the host-only service file without starting it (tests)
set -eu

REPO=alfonsusenrico/headquarter
BIN_DIR=${HQ_INSTALL_DIR:-$HOME/.local/bin}
SHARE_DIR=$HOME/.local/share/headquarter
MANIFEST=$SHARE_DIR/install-manifest
BUNDLE=$HOME/Applications/Headquarter.app
BUNDLE_ID=io.github.alfonsusenrico.headquarter
SIGNING_KEYCHAIN=$SHARE_DIR/signing.keychain-db
SIGNING_NAME="Headquarter Local Signing"
LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
SERVICE_LABEL=io.github.alfonsusenrico.headquarter.host
LAUNCH_AGENT=$HOME/Library/LaunchAgents/$SERVICE_LABEL.plist
SYSTEMD_UNIT=$HOME/.config/systemd/user/headquarter-host.service

say() { printf '%s\n' "$*"; }
fail() { printf 'headquarter install: %s\n' "$*" >&2; exit 1; }
progress() {
  if [ "${HQ_PROGRESS:-}" = 1 ]; then say "progress $*"; fi
}
# A line for the updater's log; "ok" first marks something done.
note() {
  if [ "${HQ_PROGRESS:-}" = 1 ]; then say "note $*"; fi
}
# $1 with the home folder written as ~.
home_path() {
  case $1 in
    "$HOME"/*) say "~${1#"$HOME"}" ;;
    *) say "$1" ;;
  esac
}

target() {
  case $(uname -m) in
    arm64 | aarch64) arch=aarch64 ;;
    x86_64 | amd64) arch=x86_64 ;;
    *) fail "unsupported processor $(uname -m)" ;;
  esac
  case $(uname -s) in
    Darwin) say "$arch-apple-darwin" ;;
    Linux) say "$arch-unknown-linux-gnu" ;;
    *) fail "unsupported system $(uname -s); Headquarter runs on macOS and Linux" ;;
  esac
}

sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d ' ' -f 1
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  else
    fail "neither shasum nor sha256sum is available to check the download"
  fi
}

# Puts archive $1 and its checksum file into folder $2.
fetch() {
  note "Downloading $1"
  if [ -n "${HQ_INSTALL_FROM:-}" ]; then
    cp "$HQ_INSTALL_FROM/$1" "$HQ_INSTALL_FROM/$1.sha256" "$2/" ||
      fail "$1 or its checksum is missing from $HQ_INSTALL_FROM"
    return
  fi
  if [ -n "${HQ_VERSION:-}" ]; then
    base=https://github.com/$REPO/releases/download/v${HQ_VERSION#v}
  else
    base=https://github.com/$REPO/releases/latest/download
  fi
  for file in "$1" "$1.sha256"; do
    curl -fsSL --retry 2 -o "$2/$file" "$base/$file" || fail "cannot download $base/$file"
  done
}

verify() {
  expected=$(cut -d ' ' -f 1 <"$1.sha256")
  actual=$(sha256 "$1")
  [ "$expected" = "$actual" ] ||
    fail "$(basename "$1") does not match its checksum; nothing was installed"
  note ok "$(basename "$1") matches its checksum"
}

# The path of the file named $2 inside the unpacked archive folder $1.
find_in() {
  found=$(find "$1" -type f -name "$2" | head -n 1)
  [ -n "$found" ] || fail "the release has no $2"
  say "$found"
}

record() { say "$1" >>"$MANIFEST.new"; }

put() {
  mkdir -p "$(dirname "$2")"
  cp "$1" "$2.new"
  chmod "${3:-755}" "$2.new"
  mv -f "$2.new" "$2"
  record "$2"
}

# Makes this Mac's own self-signed code-signing identity, once, in a keychain file of its
# own (sign-macos-bundle). macOS lets an app post notifications only when it is signed
# with a certificate, trusted or not; an ad-hoc signature is refused. Reusing the same
# identity on every update keeps the owner's notification permission.
ensure_signing_identity() {
  [ -f "$SIGNING_KEYCHAIN" ] && return 0
  scratch=$(mktemp -d) || return 1
  cat >"$scratch/cert.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $SIGNING_NAME
[ext]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
EOF
  # The system's LibreSSL writes a PKCS#12 file that `security import` reads.
  /usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -config "$scratch/cert.cnf" \
    -keyout "$scratch/key.pem" -out "$scratch/cert.pem" >/dev/null 2>&1 &&
    /usr/bin/openssl pkcs12 -export -inkey "$scratch/key.pem" -in "$scratch/cert.pem" \
      -out "$scratch/identity.p12" -passout pass:headquarter >/dev/null 2>&1 &&
    make_signing_keychain "$scratch/identity.p12"
  made=$?
  rm -rf "$scratch"
  return $made
}

# A keychain with an empty password, readable only by its owner, holding only this
# identity. It never joins the keychain search list, so nothing else sees it.
make_signing_keychain() {
  searched=$(security list-keychains -d user)
  mkdir -p "$SHARE_DIR"
  security create-keychain -p "" "$SIGNING_KEYCHAIN" >/dev/null 2>&1 || return 1
  if [ "$(security list-keychains -d user)" != "$searched" ]; then
    eval "security list-keychains -d user -s $searched"
  fi
  chmod 600 "$SIGNING_KEYCHAIN"
  if ! {
    security unlock-keychain -p "" "$SIGNING_KEYCHAIN" &&
      security import "$1" -k "$SIGNING_KEYCHAIN" -P headquarter -T /usr/bin/codesign >/dev/null 2>&1 &&
      security set-key-partition-list -S apple-tool:,apple: -s -k "" "$SIGNING_KEYCHAIN" >/dev/null 2>&1
  }; then
    rm -f "$SIGNING_KEYCHAIN"
    return 1
  fi
}

# Apple silicon runs only signed code. Falls back to an ad-hoc signature, which runs
# but cannot post notifications.
sign_bundle() {
  if ensure_signing_identity &&
    security unlock-keychain -p "" "$SIGNING_KEYCHAIN" &&
    codesign --force --deep --keychain "$SIGNING_KEYCHAIN" --sign "$SIGNING_NAME" "$1" >/dev/null 2>&1; then
    record "$SIGNING_KEYCHAIN"
    signed="signed by this Mac"
    return
  fi
  say "Could not make a signing certificate; the app is signed ad hoc and macOS will not show its notifications."
  signed="signed ad hoc, so macOS will not show its notifications"
  codesign --force --deep --sign - "$1" >/dev/null 2>&1 || fail "cannot sign the app bundle (codesign)"
}

# Builds Headquarter.app from the app archive unpacked under folder $2. The app finds
# this computer's host through the install record, not inside the bundle (D3).
install_mac_app() {
  version=$1 unpacked=$2
  rm -rf "$BUNDLE.new"
  mkdir -p "$BUNDLE.new/Contents/MacOS" "$BUNDLE.new/Contents/Resources"
  cp "$(find_in "$unpacked/hq-app" headquarter-app)" "$BUNDLE.new/Contents/MacOS/headquarter-app"
  chmod 755 "$BUNDLE.new/Contents/MacOS/headquarter-app"
  cp "$(find_in "$unpacked/hq-app" Headquarter.icns)" "$BUNDLE.new/Contents/Resources/Headquarter.icns"
  cat >"$BUNDLE.new/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleName</key><string>Headquarter</string>
  <key>CFBundleDisplayName</key><string>Headquarter</string>
  <key>CFBundleExecutable</key><string>headquarter-app</string>
  <key>CFBundleIconFile</key><string>Headquarter</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
  sign_bundle "$BUNDLE.new"
  rm -rf "$BUNDLE"
  mv "$BUNDLE.new" "$BUNDLE"
  record "$BUNDLE"
  note ok "Installed $(home_path "$BUNDLE"), $signed"
  # Tells Launch Services about this copy now, so Spotlight and the notification
  # centre find its name and icon here. Best effort: macOS also finds the app later.
  "$LSREGISTER" -f "$BUNDLE" >/dev/null 2>&1 ||
    say "Launch Services did not register the app yet; macOS will find it on its own."
}

install_linux_app() {
  unpacked=$1
  put "$(find_in "$unpacked" headquarter-app)" "$BIN_DIR/headquarter-app"
  for size in 16 32 64 128 256 512; do
    put "$(find_in "$unpacked" "$size.png")" \
      "$HOME/.local/share/icons/hicolor/${size}x${size}/apps/headquarter.png" 644
  done
  entry=$HOME/.local/share/applications/headquarter.desktop
  mkdir -p "$(dirname "$entry")"
  cat >"$entry" <<EOF
[Desktop Entry]
Type=Application
Name=Headquarter
Comment=A terminal for coding agents
Exec=$BIN_DIR/headquarter-app
Icon=headquarter
StartupWMClass=headquarter
Categories=Development;TerminalEmulator;
Terminal=false
EOF
  record "$entry"
  note ok "Installed $(home_path "$BIN_DIR/headquarter-app") with its icons and menu entry"
}

# Names the package for each runtime library the app needs and the system lacks.
check_linux_libraries() {
  command -v ldconfig >/dev/null 2>&1 || return 0
  libraries=$(ldconfig -p 2>/dev/null) || return 0
  missing=
  for pair in libxkbcommon.so:libxkbcommon0/libxkbcommon \
    libwayland-client.so:libwayland-client0/libwayland-client \
    libxcb.so:libxcb1/libxcb \
    libfontconfig.so:libfontconfig1/fontconfig \
    libfreetype.so:libfreetype6/freetype \
    libvulkan.so:libvulkan1/vulkan-loader; do
    case $libraries in
      *"${pair%%:*}"*) ;;
      *) missing="$missing ${pair#*:}" ;;
    esac
  done
  [ -z "$missing" ] && return 0
  say "The app needs these system libraries (Debian/Fedora package names):"
  for package in $missing; do say "  $package"; done
}

advise_path() {
  case :$PATH: in
    *:"$BIN_DIR":*) ;;
    *)
      say ""
      say "$BIN_DIR is not on your PATH. Add this line to your shell's start-up file"
      say "(~/.zshrc, ~/.bashrc or ~/.profile), then open a new terminal:"
      say "  export PATH=\"$BIN_DIR:\$PATH\""
      ;;
  esac
}

# Environment lines for the host service: the home folder, and the data and settings
# folders when the installer was run with them, so the service uses the same ones.
service_environment() {
  format=$1
  for name in HOME HQ_DATA_DIR HQ_CONFIG_DIR; do
    eval "value=\${$name:-}"
    [ -n "$value" ] || continue
    case $format in
      systemd) say "Environment=$name=$value" ;;
      launchd) say "    <key>$name</key><string>$value</string>" ;;
    esac
  done
}

# Keeps the host running for a host-only install (D4): a systemd user unit on Linux, a
# LaunchAgent on macOS. Agents run as the owner, with the owner's logins, so never as root.
install_service() {
  case $target in
    *apple-darwin) install_launch_agent ;;
    *) install_systemd_unit ;;
  esac
}

install_launch_agent() {
  mkdir -p "$(dirname "$LAUNCH_AGENT")"
  cat >"$LAUNCH_AGENT.new" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$SERVICE_LABEL</string>
  <key>ProgramArguments</key>
  <array><string>$BIN_DIR/hq</string><string>daemon</string></array>
  <key>EnvironmentVariables</key>
  <dict>
$(service_environment launchd)
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict>
</plist>
EOF
  mv -f "$LAUNCH_AGENT.new" "$LAUNCH_AGENT"
  record "$LAUNCH_AGENT"
  if [ "${HQ_SERVICE:-}" = files ]; then
    service_note="Wrote $LAUNCH_AGENT; not started."
    return
  fi
  domain=gui/$(id -u)
  if launchctl print "$domain/$SERVICE_LABEL" >/dev/null 2>&1; then
    service_note="The host runs as a LaunchAgent while you are logged in."
  elif launchctl bootstrap "$domain" "$LAUNCH_AGENT" >/dev/null 2>&1; then
    service_note="The host runs now, and starts again whenever you log in."
  else
    service_note="Could not start the host's LaunchAgent; start the host with: $BIN_DIR/hq daemon &"
  fi
  note ok "$service_note"
}

install_systemd_unit() {
  if [ "${HQ_SERVICE:-}" != files ] && {
    ! command -v systemctl >/dev/null 2>&1 || ! systemctl --user show-environment >/dev/null 2>&1
  }; then
    service_note="This system has no systemd user session, so nothing keeps the host running. Start it with:
  nohup $BIN_DIR/hq daemon >/dev/null 2>&1 &"
    return
  fi
  mkdir -p "$(dirname "$SYSTEMD_UNIT")"
  cat >"$SYSTEMD_UNIT.new" <<EOF
[Unit]
Description=Headquarter host

[Service]
ExecStart=$BIN_DIR/hq daemon
$(service_environment systemd)
Restart=always
RestartSec=2

[Install]
WantedBy=default.target
EOF
  mv -f "$SYSTEMD_UNIT.new" "$SYSTEMD_UNIT"
  record "$SYSTEMD_UNIT"
  if [ "${HQ_SERVICE:-}" = files ]; then
    service_note="Wrote $SYSTEMD_UNIT; not started."
    return
  fi
  systemctl --user daemon-reload >/dev/null 2>&1 || true
  if systemctl --user enable --now headquarter-host.service >/dev/null 2>&1; then
    service_note="The host runs as the systemd user service headquarter-host."
  else
    service_note="Could not start headquarter-host; see: systemctl --user status headquarter-host"
  fi
  user=$(id -un)
  # Without lingering, the user's services stop at logout.
  if [ "$(loginctl show-user "$user" -p Linger --value 2>/dev/null)" != yes ] &&
    ! loginctl enable-linger "$user" >/dev/null 2>&1; then
    service_note="$service_note
To keep it running while nobody is logged in, run once: sudo loginctl enable-linger $user"
  fi
  note ok "Installed the headquarter-host service"
}

# Removes one path an install recorded: the app bundle, a service (stopped first), or a file.
remove_path() {
  path=$1
  case $path in
    parts=*) return ;;
  esac
  if [ "$path" = "$BUNDLE" ]; then
    "$LSREGISTER" -u "$BUNDLE" >/dev/null 2>&1 || true
    rm -rf "$BUNDLE"
  elif [ "$path" = "$LAUNCH_AGENT" ]; then
    if [ "${HQ_SERVICE:-}" != files ]; then
      launchctl bootout "gui/$(id -u)/$SERVICE_LABEL" >/dev/null 2>&1 || true
    fi
    rm -f "$path"
  elif [ "$path" = "$SYSTEMD_UNIT" ]; then
    if [ "${HQ_SERVICE:-}" != files ]; then
      systemctl --user disable --now headquarter-host.service >/dev/null 2>&1 || true
    fi
    rm -f "$path"
    if [ "${HQ_SERVICE:-}" != files ]; then
      systemctl --user daemon-reload >/dev/null 2>&1 || true
    fi
  elif [ -d "$path" ]; then
    say "skipped $path: a folder the installer did not create"
  else
    rm -f "$path"
  fi
}

# The parts the last install recorded; an install older than parts had both.
recorded_parts() {
  [ -f "$MANIFEST" ] || return 0
  parts_line=$(sed -n 's/^parts=//p' "$MANIFEST" | head -n 1)
  say "${parts_line:-host,app}"
}

has_part() {
  case ",$parts," in
    *",$1,"*) return 0 ;;
  esac
  return 1
}

install() {
  target=$(target)
  previous=$(recorded_parts)
  parts=${1:-${previous:-host,app}}
  packages=
  if has_part host; then packages="hq-cli"; fi
  if has_part app; then packages="$packages hq-app"; fi
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT INT TERM
  progress download
  for package in $packages; do
    fetch "$package-$target.tar.gz" "$work"
  done
  progress verify
  for package in $packages; do
    verify "$work/$package-$target.tar.gz"
  done
  # Everything is verified; only now is anything unpacked or installed.
  progress install
  for package in $packages; do
    mkdir -p "$work/$package"
    tar -xzf "$work/$package-$target.tar.gz" -C "$work/$package"
  done
  if has_part host; then
    version=$("$(find_in "$work/hq-cli" hq)" --version 2>/dev/null | awk '{print $NF}')
  else
    version=$("$(find_in "$work/hq-app" headquarter-app)" --version 2>/dev/null | awk '{print $NF}')
  fi
  [ -n "$version" ] || fail "cannot read the version of the download"

  mkdir -p "$SHARE_DIR"
  say "parts=$parts" >"$MANIFEST.new"
  service_note=
  if has_part host; then
    put "$(find_in "$work/hq-cli" hq)" "$BIN_DIR/hq"
    put "$(find_in "$work/hq-cli" headquarter)" "$BIN_DIR/headquarter"
    note ok "Installed $(home_path "$BIN_DIR/hq") and $(home_path "$BIN_DIR/headquarter")"
    # Host only: nothing else starts the host, so a service keeps it running.
    if ! has_part app; then install_service; fi
  fi
  if has_part app; then
    case $target in
      *apple-darwin) install_mac_app "$version" "$work" ;;
      *) install_linux_app "$work/hq-app" ;;
    esac
  fi
  # What the previous install had and this one does not: a part given up, or a service
  # no longer needed. The host stops before its files go.
  if [ -f "$MANIFEST" ]; then
    if [ -x "$BIN_DIR/hq" ] && ! has_part host; then
      "$BIN_DIR/hq" stop --yes >/dev/null 2>&1 || true
    fi
    while IFS= read -r path; do
      grep -qxF "$path" "$MANIFEST.new" || remove_path "$path"
    done <"$MANIFEST"
  fi
  mv -f "$MANIFEST.new" "$MANIFEST"
  progress done "$version"

  case $parts in
    host) say "Installed the Headquarter $version host." ;;
    app) say "Installed the Headquarter $version app." ;;
    *) say "Installed Headquarter $version." ;;
  esac
  if has_part app; then
    case $target in
      *apple-darwin) say "Open Headquarter from Spotlight or $BUNDLE." ;;
      *)
        say "Open Headquarter from your application menu or run headquarter-app."
        check_linux_libraries
        ;;
    esac
  fi
  if [ -n "$service_note" ]; then say "$service_note"; fi
  if has_part host && has_part app; then
    say "If an older Headquarter is still running, quit it and run: hq stop"
  elif has_part host; then
    say "Pair the app on another computer with: hq remote pair"
    if [ -n "$previous" ]; then
      say "A host already running keeps its version until it restarts. To switch now, run: hq stop"
      say "Running sessions end and resume on the new version."
    fi
  else
    say "Connect it to a host with Connect to a Host, using the code \`hq remote pair\` shows there."
  fi
  # The commands, and on Linux the app, go into the bin folder.
  case $target in
    *linux-gnu) advise_path ;;
    *) if has_part host; then advise_path; fi ;;
  esac
}

uninstall() {
  [ -f "$MANIFEST" ] || fail "nothing to remove: $MANIFEST is missing"
  if [ -x "$BIN_DIR/hq" ]; then
    "$BIN_DIR/hq" stop --yes >/dev/null 2>&1 || true
  fi
  while IFS= read -r path; do
    remove_path "$path"
  done <"$MANIFEST"
  rm -f "$MANIFEST"
  say "Removed Headquarter. Your settings and sessions stay in:"
  say "  ${XDG_CONFIG_HOME:-$HOME/.config}/headquarter"
  say "  $SHARE_DIR"
}

case ${1:-} in
  "") install ;;
  --full) install host,app ;;
  --host-only) install host ;;
  --app-only) install app ;;
  --uninstall) uninstall ;;
  *) fail "unknown option $1 (use --host-only, --app-only, --full or --uninstall)" ;;
esac
