# release-install Specification

## Purpose
How Headquarter is released and installed: CI builds archives with checksums on version tags, and one installer line puts the command line and the app on macOS or Linux without administrator rights, verifies every download, signs the macOS app with a certificate made on the owner's Mac, and removes exactly what it installed.

## Requirements

### Requirement: One-line install
The system SHALL install, with one command and without administrator rights, the host and the app (default), the host only (`--host-only`), or the app only (`--app-only`), for the owner's system and architecture, and SHALL say how to put the install folder on `PATH` when it is missing.

#### Scenario: macOS
- **WHEN** the owner runs the install line on an Apple silicon Mac
- **THEN** `hq` and `headquarter` are in `~/.local/bin` and `Headquarter.app` is in `~/Applications`, opening without a security prompt

#### Scenario: Linux
- **WHEN** the owner runs the install line on x86-64 Linux
- **THEN** `hq`, `headquarter` and `headquarter-app` are in `~/.local/bin`, Headquarter appears in the desktop's application menu, and any missing system library is named with the package that provides it

#### Scenario: Folder not on PATH
- **WHEN** `~/.local/bin` is not on `PATH`
- **THEN** the installer prints the line to add to the shell's start-up file

#### Scenario: Linux window matched to its entry
- **WHEN** Headquarter runs on a Linux desktop after the install line
- **THEN** its window carries the app id `headquarter`, the desktop entry names the same window class, and the dock and window switcher show the Headquarter icon

#### Scenario: Host only on a server
- **WHEN** the owner runs the install line with `--host-only` on a headless Linux server
- **THEN** `hq` and `headquarter` are installed, no app or menu entry is, and the host runs as the owner's service

#### Scenario: App only
- **WHEN** the owner runs the install line with `--app-only` on a Mac
- **THEN** only `Headquarter.app` is installed, and it opens at Connect to a Host

### Requirement: Verified downloads
The system SHALL verify every downloaded archive against its published SHA-256 before installing anything.

#### Scenario: Checksum mismatch
- **WHEN** a downloaded archive does not match its checksum
- **THEN** the installer stops with an error and nothing is installed or changed

### Requirement: Clean removal
The system SHALL remove exactly what the installer created when asked, after stopping the host, and SHALL keep the owner's settings and data.

#### Scenario: Uninstall
- **WHEN** the owner runs the installer with `--uninstall`
- **THEN** the binaries, app bundle or desktop entry it installed are removed, the host is stopped, and the settings and data folders remain and are named

### Requirement: Releases from CI
The system SHALL build and publish releases only from CI on version tags: the host and command line for macOS and Linux on both architectures, the app for those, and for Windows x86-64 a setup program, with checksums and the installer.

#### Scenario: Tag pushed
- **WHEN** the owner pushes the tag `v0.1.0`
- **THEN** CI builds the archives for the four Unix targets and the Windows setup, and publishes them with checksums and `install.sh` on the `v0.1.0` release

### Requirement: Ready to publish
The system SHALL carry a licence, a changelog and install instructions, and SHALL track nothing that should stay private, before the repository is made public.

#### Scenario: Licence
- **WHEN** someone reads the repository
- **THEN** the licence is MIT OR Apache-2.0, stated in license files and every crate

### Requirement: macOS menu bar
On macOS the app SHALL show its name, Headquarter, in the menu bar, the Dock and the app switcher when installed, and SHALL offer a Headquarter, an Edit and a Window menu whose items run the same actions as the app's shortcuts, listing only shortcuts the app already keeps for itself, so no key an agent uses is taken.

#### Scenario: Installed app
- **WHEN** the owner opens Headquarter.app
- **THEN** the menu bar reads "Headquarter" with About Headquarter, Check for Updates…, Themes (⌘G), Hide (⌘H), Hide Others and Quit (⌘Q), an Edit menu with Copy and Paste, and a Window menu with Minimize (⌘M), New Session (⌘N) and Open Sessions (⌘O)

#### Scenario: Menu item
- **WHEN** the owner chooses Window > New Session
- **THEN** the New session window opens, as with ⌘N

#### Scenario: Agent keys
- **WHEN** an agent is focused and the owner presses a plain Ctrl key, such as Ctrl-C
- **THEN** the key reaches the agent and no menu item runs

#### Scenario: Linux
- **WHEN** the app runs on Linux
- **THEN** there is no app menu bar and the Ctrl-Shift shortcuts work unchanged

### Requirement: Local signature on macOS
On macOS the installer SHALL sign the app bundle with a self-signed code-signing certificate it creates once on the owner's Mac and reuses on every update. It SHALL keep that certificate in a keychain file of its own, outside the keychain search list, SHALL remove it on uninstall, and SHALL fall back to an ad-hoc signature with a message when it cannot create or use the certificate.

#### Scenario: First install
- **WHEN** the owner installs Headquarter on a Mac for the first time
- **THEN** `Headquarter.app` is signed by "Headquarter Local Signing", the keychain file is readable only by the owner, and the owner's keychain list is unchanged

#### Scenario: Update
- **WHEN** the owner runs the install line again
- **THEN** the app is signed with the same certificate as before

#### Scenario: Certificate cannot be made
- **WHEN** creating or using the certificate fails
- **THEN** the installer says that macOS notifications will not show, signs the app ad hoc, and finishes the install

#### Scenario: Uninstall
- **WHEN** the owner uninstalls Headquarter
- **THEN** the keychain file with the certificate is removed with the rest

### Requirement: Installer progress
The installer SHALL, when `HQ_PROGRESS=1` is set, print one line per step (`progress download`, `progress verify`, `progress install`, `progress done <version>`) and one `note` line per file it fetches, checks or installs (`note ok` once done, with the home folder written as `~`), besides its usual output, and SHALL print nothing new without it.

#### Scenario: Progress asked for
- **WHEN** the installer runs with `HQ_PROGRESS=1`
- **THEN** its output holds `progress download`, `progress verify`, `progress install` and `progress done 0.1.9` in that order, and notes such as `note ok Installed ~/.local/bin/hq and ~/.local/bin/headquarter`

#### Scenario: Not asked for
- **WHEN** the installer runs without `HQ_PROGRESS`
- **THEN** its output has no progress lines and no notes

### Requirement: Android APK in releases
The system SHALL publish with every version tag a signed Android APK named `headquarter-android.apk`, for arm64-v8a, with its SHA-256 in `headquarter-android.apk.sha256`, built and signed by the same release run as the desktop archives, and SHALL publish nothing when the APK cannot be built or signed.

#### Scenario: A release
- **WHEN** the owner pushes a version tag
- **THEN** the release carries `headquarter-android.apk` and its `.sha256` beside the desktop archives, `apksigner verify` accepts the APK, and its native libraries are for `arm64-v8a` only

#### Scenario: Installing over the previous release
- **WHEN** a phone with the previous release's APK installs the new one
- **THEN** it installs as an update and keeps the paired computers

#### Scenario: The APK fails
- **WHEN** the Android build or its signing fails in a release run
- **THEN** no GitHub release is published for that tag

#### Scenario: Debug builds for an emulator
- **WHEN** the owner builds the debug app
- **THEN** it carries the core for `arm64-v8a`, `armeabi-v7a` and `x86_64`

### Requirement: The signing key stays with the release run
The system SHALL sign release APKs only in release runs from version tags, with a key that the owner created and keeps a backup of, and SHALL never print the key or its passwords.

#### Scenario: A run that is not from a version tag
- **WHEN** the Android job runs from anything but a version tag
- **THEN** it cannot read the signing secrets and fails before building

#### Scenario: Local builds
- **WHEN** the release APK is built without the signing values, as `./scripts/check` does
- **THEN** it builds unsigned and shrunk, and nothing asks for the key

### Requirement: Debug builds beside the release
Debug builds of the app SHALL install beside the release build, under their own id and the label "Headquarter (debug)".

#### Scenario: A development phone
- **WHEN** the release APK and a debug build are installed on one phone
- **THEN** both appear, as Headquarter and Headquarter (debug), each with its own paired computers

### Requirement: Windows checks on every pull request
The system SHALL build, lint with warnings as errors, and test the client crates for Windows on every pull request, on a free GitHub-hosted runner.

#### Scenario: A change breaks the Windows build
- **WHEN** a pull request adds code that does not compile for Windows
- **THEN** the Windows check fails on that pull request

### Requirement: Install parts are remembered
The installer SHALL record which parts it installed, keep them when run again without an option, change them when run with another option, and remove only recorded parts on uninstall.

#### Scenario: Rerun on the server
- **WHEN** the owner reruns the plain install line on a host-only server
- **THEN** the host is updated and no app is installed

#### Scenario: From full to app only
- **WHEN** the owner runs the install line with `--app-only` on a full install
- **THEN** the app stays, `hq` and the host service are removed, and the data folders remain and are named

### Requirement: App archives carry no host
The app archives SHALL hold the desktop app only, and the macOS app bundle SHALL not contain `hq`.

#### Scenario: Release assets
- **WHEN** a release is published
- **THEN** the app archives contain no `hq` executable, and the host archives contain `hq` and `headquarter`

### Requirement: Windows setup
The release SHALL offer the Windows app as `headquarter-setup-x86_64.exe`: a setup wizard that installs for the current user into `%LOCALAPPDATA%\Programs\Headquarter` without an administrator prompt, adds a Start menu entry and optionally a desktop shortcut, registers an uninstaller, and updates an installed copy in place. Uninstalling SHALL keep the app's data.

#### Scenario: First install
- **WHEN** the owner runs the setup on a PC without Headquarter and accepts the defaults
- **THEN** no administrator prompt appears, the app is in `%LOCALAPPDATA%\Programs\Headquarter`, the Start menu lists Headquarter with its icon, and Finish can open it

#### Scenario: Updating
- **WHEN** the app is running and the owner runs a newer setup
- **THEN** the setup closes the app, replaces it in place, and the paired hosts and settings are kept

#### Scenario: Uninstalling
- **WHEN** the owner removes Headquarter in Settings ▸ Apps
- **THEN** the program and its shortcuts are removed and `%APPDATA%\Headquarter` stays

#### Scenario: Checksums
- **WHEN** the release is published
- **THEN** `headquarter-setup-x86_64.exe.sha256` matches the setup, and the release carries no Windows zip
