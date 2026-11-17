# Packaging and the updater for the switch-over

The decision record for Step 10 of [the gpui-port plan](./plan.md): how the
native app is packaged, how an install of it is updated, and what happens to
the Tauri installs already in the field. First written on 2026-11-14 by the
gpui-ci-packaging plan (`../_archived/20261114-gpui-ci-packaging/plan.md`),
and re-decided on 2026-11-16 by the gpui-packaging-redecision plan
(`../_archived/20261116-gpui-packaging-redecision/plan.md`). The open
questions that re-decision left were answered by the user on 2026-11-16 and
recorded by the gpui-packaging-answers plan
(`../_archived/20261116-gpui-packaging-answers/plan.md`). The earlier text
(`cargo-packager` and a self-written updater keeping the full `latest.json`
contract) is in git history only.

Each claim is tagged **Source** (read from the named source on 2026-11-16) or
**Unverified** (to be checked in the sub-milestone that needs it). Nothing
here comes from running a packaging tool, `self_update` or an update; no
packaging or updater code was written. The exceptions are the items tagged
**Measured**, rewritten by the gpui-packaging plan
(`../20261117-gpui-packaging/plan.md`) from its `Package` runs.

## The user's decision (2026-11-16)

- Auto-update for Tauri installs stops at the last Tauri release. Tauri users
  reinstall the native app by hand.
- Dropped with it: the `latest.json` manifest, the Tauri updater's use of
  the minisign key pair (`plugins.updater.pubkey`, and
  `TAURI_SIGNING_PRIVATE_KEY` as the Tauri bundler's input), NSIS
  `/P /UPDATE` compatibility, the MSI upgrade code, an rpm for shipped rpm
  installs, and the "real-device update from the last Tauri release" gate. A
  project-owned signing key itself is kept (answer 2 below).
- The native app's packaging and updating follow gpui-kit's standard guides,
  https://gpui-kit.com/docs/packaging/ and
  https://gpui-kit.com/docs/auto-update/.

The user's answers to the questions the re-decision left, also on
2026-11-16, each spelled out in "The decision" below:

1. Installer-owned installs: the app drives the owner (the setup `.exe` run
   at quit, the whole `.app` swapped); a deb stays with the package manager.
2. Publisher authenticity: every update the app installs is verified against
   a project-owned signing key.
3. OS code signing: none.
4. Linux formats: gpui-kit's only, a `.tar.gz` with `install.sh` and a
   `.deb`; no AppImage, no rpm.
5. The last Tauri release is v1.10.0, with no in-app notice and no frozen
   legacy `latest.json`.
6. The Windows portable ZIP ships.
7. The update check on a deb install notifies and installs nothing.

## How Tauri installs stop updating

- **Source** (`crates/app/tauri.conf.json`). The updater endpoint is
  `https://github.com/minodisk/handpic/releases/latest/download/latest.json`.
- **Source** (`tauri-plugin-updater` 2.12.0's `updater.rs`, the version in
  the root `Cargo.lock`). A response without a success status leaves no
  release, and `check` returns `Error::ReleaseNotFound`.
- **Source** (`crates/app/src/update.rs`). The startup check logs
  `update failed: …`; only the menu check shows an error dialog.
- **Source** (the three above). So the first published release without a
  `latest.json` asset ends auto-update for every Tauri install, with no code
  change.
- Rule: no native release attaches an asset named `latest.json` in the Tauri
  shape that names native artifacts, since shipped installs would try to
  install it.
- **Source** (`gh release list`, 2026-11-16). v1.10.0 is the latest published
  release. The earlier record read its assets as named `Riffle`. `main`
  already carries the Handpic rename (`tauri.conf.json` `productName`),
  unreleased.

## What gpui-kit's guides say

**Source**: the two pages above, read on 2026-11-16 through a summarizing
fetch. The exact scripts (`install.sh`, the `.iss`, the deb `control`) were
not read line by line; the packaging sub-milestone reads them.

### Packaging

- No single packager. `cargo build --release --locked` per target and
  architecture, on that platform.
- macOS: a `.app` (`Info.plist`, `.icns`); Developer ID with hardened runtime
  (`codesign --options runtime --timestamp`), nested code first, not
  `--deep`; a DMG by `hdiutil create … -format UDZO`, signed;
  `xcrun notarytool submit --wait` with the result checked as Accepted;
  `xcrun stapler staple` / `validate`. A ZIP or tarball of the `.app` is the
  portable form.
- Windows: a portable ZIP, or an Inno Setup `.exe` installer (`ISCC.exe`, a
  stable `AppId`). The exe is signed before packaging, then the setup; the
  uninstaller needs Inno's signed-uninstaller workflow.
- Linux: a `.tar.gz` with an `install.sh` that verifies a SHA-256 before a
  per-user install under `~/.local`, and a `.deb` by
  `dpkg-deb --root-owner-group --build` with `Depends` derived from the built
  binary. rpm needs a separate recipe. AppImage and MSI are not mentioned.
- SHA-256 checksums are published; install, upgrade from N to N+1 and
  uninstall are tested on clean machines.

### Auto update

- The install's owner updates it.
- `self_update` 1.3 is an optional dependency: the GitHub backend,
  `checksum_from_asset("SHA256SUMS")`, `is_update_available()` / `update()`.
  It blocks, so it runs on a background worker. It is only for a writable
  single-binary portable install.
- A macOS `.app` from a DMG: a bundle-aware updater, or a new signed and
  notarized app replacing the whole `.app`, never the executable alone.
- Inno Setup: the installer owns the files, the shortcuts and the uninstall
  record.
- deb / rpm: the package manager.
- A multi-file portable archive is updated as a whole.
- Restart to finish; report the new version only after the relaunch.
- Stage outside the live install.
- Windows file locks and elevation are handled by a helper or the installer.
- Do not silently switch an install from one owner to another.
- `SHA256SUMS` proves integrity, not who published; use a signing key or
  signed metadata where that matters.
- Pin the selected version between the check and the install.

### `self_update`'s signatures

- **Source** (docs.rs feature list, 2026-11-16). `self_update` is at 1.3.3
  and has a `signatures` feature depending on `zipsign-api` 0.2.2.
- **Unverified.** How it signs and verifies, and where the key lives.

## The decision

The gpui-kit-aligned layout per platform, with the user's answers of
2026-11-16 in place.

- **Tooling.** Platform-native tools driven from mise tasks and the release
  workflow; no `cargo-packager`, `cargo-bundle` or Tauri bundler. It grows
  from what exists (**Source**: `mise.toml`, `crates/native/packaging/`,
  `crates/native/build.rs`, `docs/agents/native-app.md`):
  - `native:bundle:macos` lays out an unsigned `Handpic.app`, with no DMG.
  - `native:install:linux` installs into the same `~/.local` layout
    gpui-kit's `install.sh` uses.
  - `build.rs` already embeds the Windows icon.
  - The license notices (`gpui-component`, the `gpui-pre-*` crates, Lucide;
    the models' licenses in `crates/core/models/`) are due at packaging.
- **macOS, arm64 and x86_64 separately.** `Handpic.app` in a DMG, ad hoc
  signed. The app updates a DMG install itself (answer 1): it stages the new
  version and swaps the whole `.app`, never the executable alone.
- **Windows x64.** A per-user Inno Setup setup `.exe`, installed under
  `%LOCALAPPDATA%` so that there is no UAC prompt, and a portable ZIP next
  to it, as gpui-kit's guide has it (answer 6). For an Inno Setup install
  the app downloads the new setup `.exe` in the background and runs it at
  quit, silently (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`); the next
  launch is the new version (answer 1). `self_update` updates the portable
  copy. No MSI and no NSIS.
- **Linux x64.** A `.tar.gz` with `install.sh`, updated by `self_update`,
  and a `.deb`, left to the package manager with no in-app install
  (answer 1). There is no apt repository, so in effect a deb install is
  reinstalled by hand after the app's notice: the launch check and the menu
  item run on a deb install too, and when a newer version exists the app
  says so and offers to open the release page; it installs nothing
  (answer 7). No rpm and no AppImage (answer 4).
  - **Source** (`tauri-plugin-updater` 2.12.0's `updater.rs`,
    `install_deb`). Today's updater runs `dpkg -i` through `pkexec` for a
    deb install.
  - **Source** (`docs/humans/usage.md`, "Updating"). The usage guide
    nevertheless says a deb install is updated by installing the newer
    package.
  - **Unverified.** Whether a shipped deb install really updates itself, and
    so whether leaving the deb to the package manager is a regression.
  - **Source** (`README.md` line 32). The AppImage is the Linux download the
    README leads with today; sub-milestone 6 of Step 10 rewrites that.
- **OS code signing: none** (answer 3). No Apple Developer ID and no
  notarization (macOS stays ad hoc); no Authenticode. This deviates from
  gpui-kit's guide. **Source** (`README.md`, "Install"): the README tells
  users how to pass Gatekeeper and SmartScreen, and keeps doing so.
- **Publisher authenticity: a project-owned signing key** (answer 2). Every
  update the app installs is verified against a key the project owns, as
  minisign does for the Tauri app today; `SHA256SUMS` alone is not enough.
  For the portable archives the candidate is `self_update`'s `signatures`
  feature (`zipsign-api`). The mechanism for the setup `.exe` and the `.app`
  download is chosen in the updater sub-milestone and must use the project's
  own key; whether that is the existing minisign key pair or a new one is
  chosen there too.
- **Every release** carries `SHA256SUMS` and the signatures of the project's
  key, and no `latest.json` of any kind: neither one naming native artifacts
  nor a frozen copy of the last Tauri release's.
- **The update flow in the app.** A launch check and a manual check, on
  every install type. The app drives the install's owner: the setup `.exe`
  at quit, the `.app` swap, `self_update` for the portable ZIP and the
  `.tar.gz`; a deb install only gets the notice. Every download is verified
  with the project's key before it is installed. Restart to finish; the
  install or the relaunch only after the sidecar writer is drained on quit,
  as `crates/app/src/update.rs` does today. gpui-kit's rule "do not silently
  switch an install from one owner to another" still holds.
- **Tauri installs** (answer 5). The last Tauri release is v1.10.0, still
  named Riffle. No further Tauri release and no in-app notice. The reinstall
  notice lives in the first native release's notes, in `README.md` /
  `README.ja.md` and in the usage guide under `docs/humans/`. No frozen
  legacy `latest.json` is attached to native releases, so a menu check on a
  Tauri install shows an update error from the first native release on.
  Consequences:
  - The Tauri-side changes on `main` since v1.10.0, the Handpic rename among
    them, never ship.
  - No release is published from `main` until sub-milestone 4 of Step 10 has
    switched `release.yml` to the native artifacts.
- **Not taken.**
  - `cargo-packager` plus the self-written `latest.json` updater: its
    deciding fact, Windows NSIS `/P` compatibility with shipped installs, is
    gone with the user's decision.
  - `cargo-packager-updater`: a manifest dialect of its own for a contract
    nobody needs to keep any more.
  - `cargo-bundle`: no NSIS or Inno Setup, Windows support called
    experimental (**Source**: `cargo-bundle` 0.12.0's readme, as read on
    2026-11-14 by the earlier record), and nothing the platform-native tools above do not cover.
  - Notify only for the installer, DMG and deb installs: a regression from
    today's silent updates on Windows and macOS.
  - OS signatures as the authenticity answer: there is no OS code signing.
  - Checksum-only over GitHub HTTPS: `SHA256SUMS` proves integrity, not who
    published.
  - Paid certificates (Developer ID, Authenticode): the user chose no OS
    code signing.
  - Free signing for open source, such as SignPath Foundation: left out with
    the rest of OS code signing. **Unverified**: nothing was read about it.
  - rpm and AppImage: outside gpui-kit's guide; the `.tar.gz` with
    `install.sh` covers rpm distributions and replaces the AppImage.
  - A final Tauri release from `main`, with or without an in-app notice: its
    notes would be seen only on GitHub, and an in-app notice needs a change
    in the frozen `crates/app`.
  - A frozen copy of the last Tauri release's `latest.json` on native
    releases: one legacy asset on every release, and a rule that it never
    names a native artifact, only to turn an update error into "up to date".
  - No check, or a manual-only check, on a deb install: a deb user would not
    learn of a new version.

## Unverified

To be checked in the sub-milestone that needs each one.

- **Unverified.** How the running app knows its install type (portable, Inno
  Setup, DMG, deb), which "do not switch owners" requires, and, for answer 7,
  whether a deb install only notifies.
- **Unverified.** `self_update`'s asset-name and target matching, the
  binary's path in the archive, and whether the portable archives count as
  single-binary once the license notices ship beside the binary.
- **Unverified.** Whether replacing the running exe works on Windows for the
  portable copy.
- **Unverified.** The executable's name in a release (`riffle-native` today
  in `Info.plist`, the `.desktop` file and `datadir.rs`'s tests).
- **Unverified.** The version source: `crates/native/Cargo.toml` and
  `Info.plist` are `0.0.0`, and `release-please-config.json` bumps neither.
- **Unverified.** The `.desktop` file's `Exec` (relative today; gpui-kit's
  `install.sh` writes an absolute path).
- **Unverified.** The deb's `Depends` and the Vulkan driver requirement at
  run time.
- **Unverified.** What a hand reinstall over a Tauri install does on each
  platform: the Inno Setup install next to the NSIS / MSI one, with its
  uninstall entry and shortcuts; the deb's `Package` name against the Tauri
  deb's, so that `apt` upgrades in place; a leftover `Riffle.app`; an
  AppImage file and an rpm package of the Tauri app, formats now dropped,
  left for the user to remove.
- **Unverified.** That the native app picks up the Tauri install's data.
  **Source** (`CLAUDE.md`): `settings.rs` seeds its store from the Tauri
  `settings.json`, and the index is the native app's own. Not checked on a
  real install.
- **Measured** (2026-11-17, the gpui-packaging plan's Step 2, `Package` run
  44077941693 on `windows-latest`). `PrivilegesRequired=lowest` with
  `DefaultDirName={autopf}\Handpic` installs to
  `%LOCALAPPDATA%\Programs\Handpic` (`C:\Users\runneradmin\AppData\Local\Programs\Handpic\`
  on the runner), with the uninstall entry under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\dev.minodisk.handpic_is1`
  and none under `HKLM`, and a Start menu shortcut under `%APPDATA%`.
  The install under `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`, a second run
  over it (the upgrade) and the silent uninstall each exit 0, the uninstall
  removing the exe, the entry and the shortcut. Directives:
  `AppId=dev.minodisk.handpic` (never to change), `ArchitecturesAllowed` and
  `ArchitecturesInstallIn64BitMode` `x64compatible`, `SetupIconFile`
  `crates/app/icons/icon.ico`, the version given as `ISCC
  /DAppVersion=<version>`. The runner image ships Inno Setup 6.7.1, its
  `ISCC.exe` on `PATH` as Chocolatey's `iscc` shim, so the workflow
  installs nothing. **Unverified**: the hosted runner runs elevated, so the
  absence of a UAC prompt for a standard user, at the install and at a
  silent upgrade, is for a real machine (sub-milestone 4's gate).
- **Unverified.** Whether the setup started at quit outlives the app's
  process and replaces the exe once it has exited (file locks, Inno's
  close-applications handling).
- **Unverified.** How `Restart Now` relaunches the app after a silent setup
  run.
- **Unverified.** Whether SmartScreen warns on a setup `.exe` the app
  downloaded itself.
- **Unverified.** Where the staged `.app` comes from on macOS: the DMG
  mounted, or an archive of the `.app` published beside it. The updater
  sub-milestone chooses, and the packaging adds the asset it needs.
- **Unverified.** Whether the `.app` swap works where the app is installed:
  a standard user's `/Applications`, or an app run from the DMG or from
  Downloads under App Translocation.
- **Unverified.** Whether Gatekeeper quarantines the swapped ad hoc bundle,
  and whether the folder-access grants survive a swap to a bundle with
  another ad hoc signature.
- **Unverified.** Which archive formats `zipsign` signs, and what verifies
  the setup `.exe` and the `.app` download with the project's key.
