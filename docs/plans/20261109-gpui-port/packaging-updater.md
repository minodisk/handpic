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
(`../20261117-gpui-packaging/plan.md`) from its `Package` runs, and by the
native-updater plan (`../20261118-native-updater/plan.md`) from its tests
and its rehearsal on a test feed (the section below).

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
- **Source** (the native-updater plan, `../20261118-native-updater/plan.md`,
  Steps 2 and 3). zipsign 0.2.2 signs a `.zip` (the signature before its
  data) and a `.tar.gz` (a gzip member after it) with an ed25519 key, bound
  to the file's name; `self_update`'s `verifying_keys` checks it after the
  `SHA256SUMS` digest and before extraction. The verifying key is built into
  the app (`crates/app/packaging/update.pub`, or the test key of a
  `HANDPIC_UPDATE_TEST_REPO` build); the private key stays with the user.

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

## The update flow on a test feed (2026-11-21)

The rehearsal of the rc.1 to rc.2 update, Step 7 of the native-updater plan,
with the test key instead of the production one. `Package` runs 44769330982
(`version=2.0.0-rc.1`) and 44769335894 (`version=2.0.0-rc.2`), both with
`update_test_repo=minodisk/handpic-update-test` and green on the first try,
were published by `tools/package/test-release.sh` as GitHub prereleases of
the public scratch repository `minodisk/handpic-update-test`, the feed both
builds read. A stable build for the same feed (`1.99.0`: the branch's source
with `set-version.sh 1.99.0` and `HANDPIC_UPDATE_TEST_REPO` set, built
locally with `cargo build --release --locked`) was not published.

Machine: the only one at hand, a Windows 11 Home 10.0.26200 x64 PC, the
account a member of Administrators running with its UAC-filtered
(non-elevated) token, UAC on. Every copy ran with its own `--settings`,
`--index`, `--log` and `--mcp-port` (the MCP companion seeded on through
`--import-settings`), so each launch checked, and its state was read from
the log, the executable's `ProductVersion`, the `HKCU` uninstall entry and
the MCP `get_status` `update` object. A quit was the window's close
(`CloseMainWindow`), which runs the same quit handlers as `q`.

| Install      | Check                                                                                           | Result |
| ------------ | ----------------------------------------------------------------------------------------------- | ------ |
| Portable ZIP | rc.1 updates to rc.2 while running                                                              | **Measured** (2026-11-21, the PC above). rc.1 unpacked with `Expand-Archive` logged `install Portable`, was offered `2.0.0-rc.2` and installed it 4 s after launch without a click. `handpic.exe` read `2.0.0-rc.2` while rc.1 still ran (`self_replace` left `.handpic.<random>.__relocated__.exe` and `.__selfdelete__.exe` beside it, the latter running as a helper), `get_status` said `ready_to_restart` with target `2.0.0-rc.2`, and the pane's note named `Restart Now`. |
| Portable ZIP | `Restart Now` relaunches rc.2; the judgments made just before the restart are in the sidecars   | **Measured** (2026-11-21, the PC above, the user's click). The quit handlers drained (scan stopped, judgment stores and sidecar writer drained, settings drained, MCP stopped) and the relaunched process started in the same second, `handpic.exe` reading `2.0.0-rc.2`. A rating pressed just before `Restart Now` reached the `.xmp` (`xmp:Rating="4"`). No `__relocated__` / `__selfdelete__` file was left in the install folder. |
| Inno Setup   | rc.1 downloads in the background                                                                | **Measured** (2026-11-21, the PC above). rc.1 installed by its setup (`/VERYSILENT`, exit 0 in 2 s) logged `install Inno`, staged `Handpic-2.0.0-rc.2-windows-x64-setup.exe` in `%LOCALAPPDATA%\dev.minodisk.handpic\update` 2 s after the offer, and `get_status` said `ready_to_restart` ("installs when Handpic quits"). |
| Inno Setup   | a plain quit installs rc.2 without a prompt                                                     | **Measured** (2026-11-21, the PC above). After the three drain lines the quit logged `update setup … relaunch=false started=Ok(())`; the installed exe and the uninstall entry's `DisplayVersion` read `2.0.0-rc.2` 2 s later, unattended, and no Handpic process ran 15 s after that. The staging folder was gone at the next launch. |
| Inno Setup   | `Restart Now` relaunches rc.2                                                                   | **Measured** (2026-11-21, the PC above, the user's click). The last quit handler logged `quit: update setup …\update\Handpic-2.0.0-rc.2-windows-x64-setup.exe relaunch=true started=Ok(())`; the installed exe read `2.0.0-rc.2` and the app came back about 2 s after the quit, with no UAC and no SmartScreen prompt. |
| Inno Setup   | no SmartScreen or UAC prompt for a standard user                                                | **Measured** for this account's non-elevated token: both silent installs and the upgrade at quit finished unattended in about 2 s (a prompt would have held them), and the staged setup carries no `Zone.Identifier` stream (no Mark of the Web, which SmartScreen keys on). The user watched the screen during `Restart Now`: no prompt appeared. **Unverified**: a standard (non-administrator) account; left to the rc dry run. |
| `.app`       | rc.1 in `/Applications` swaps to rc.2 and reopens; from the DMG or Downloads it fails with the move message; Gatekeeper and the folder-access grants after the swap | **Unverified**: no macOS machine at hand. Deferred to the rc dry run. The swap and its refusals are the engine's tests, the real `codesign` one on the `test` job's macOS leg. |
| Tarball      | rc.1 from `install.sh` updates to rc.2 and relaunches                                           | **Unverified**: no Linux machine at hand. Deferred to the rc dry run. |
| deb          | rc.1 shows the notice and opens the release page; nothing is installed                          | **Unverified**: no Linux machine at hand. Deferred to the rc dry run. |
| Any          | a stable build on the same feed is not offered an rc                                            | **Measured** (2026-11-21, the PC above). The local `1.99.0` build as a portable copy, with `v2.0.0-rc.2` on the feed, logged `offered None`, and `get_status` said `idle`, "Handpic is up to date." |
| Any          | offline at launch leaves the app usable                                                         | **Measured** (2026-11-21, the PC above), offline simulated by `HTTPS_PROXY` / `HTTP_PROXY` naming a closed loopback port (`ureq` reads them). rc.1 portable logged `check failed: TransportError … (os error 10061)` 2 s after launch, `get_status` said `failed` with that message, a pick set over MCP was written to the sidecar, the exe stayed rc.1, and the quit drained as usual. |
| Any          | a tampered asset installs nothing                                                               | **Measured** (2026-11-21, the PC above). A `v2.0.0-rc.3` prerelease of rc.2's portable ZIP and setup ZIP, renamed and one byte flipped in each, with a recomputed `SHA256SUMS` (so only the signature could catch it), was published for the test and deleted after it. rc.1 portable failed with `SignatureError: no matching key/signature pair found` and its exe stayed byte for byte the same; the Inno install (rc.2 by then) failed with the same error on the setup ZIP, the staging folder was removed, and its quit started no setup. A wrong digest, a missing asset and an unsigned archive are the engine's tests. |

Two more things the `Restart Now` runs showed:

- **Measured** (the same runs). The relaunch does not carry the original
  command-line arguments: both relaunched processes' command lines held only
  the exe path, so a test run started with `--log`, `--index` or
  `--settings` comes back on the default data folders. Users pass none, so
  this only matters to a test run.
- **Hit** (the same runs). Both relaunches opened a console window streaming
  the log: the Windows release `handpic.exe` is a console-subsystem binary
  (PE Subsystem 3; `crates/app/src/main.rs` sets no `windows_subsystem`).
  The updater does not cause it: any launch from Explorer or the Start menu
  shows it, and the earlier runs missed it because they were started from a
  shell. Fixed in a separate PR, which must land before the release PR
  #928.

## Unverified

To be checked in the sub-milestone that needs each one.

- **Source** (the native-updater plan, Step 3; `crates/engine/src/update.rs`,
  `detect`). The install type is read from the executable's path: an
  `unins000.exe` beside it is Inno Setup, else a `licenses/` folder beside it
  is the portable ZIP; `<x>.app/Contents/MacOS/<exe>` is a `.app`;
  `/usr/lib/handpic/<exe>` the deb; `<…>/.local/bin/<exe>` the tarball;
  anything else (a `cargo run` or `target/` binary) unknown. A deb or unknown
  install stops at `Available` with `Open Release Page` and installs nothing
  (unit-tested). **Unverified**: on a real deb install (the deb row of the
  plan's Step 7, deferred to the rc dry run).
- **Source** (the native-updater plan, Step 3). The asset is matched by its
  exact name through `asset_matcher` (`Handpic-<version>-windows-x64.zip`,
  `Handpic-<version>-linux-x64.tar.gz`), which replaces `self_update`'s
  target matching, with the release pinned by `release_tag("v<version>")`;
  the executable is `bin_path_in_archive` `Handpic/<exe>` in the ZIP and
  `Handpic/bin/<exe>` in the tarball. Only the executable is replaced: the
  ZIP's `licenses/` and the tarball's icon and desktop entry stay as first
  installed (the plan's deviation 6 from gpui-kit's guide).
- **Measured** (2026-11-21, the test feed above). Replacing the running exe
  works on Windows for the portable copy: `self_update` (`self_replace`)
  installed rc.2 over the running rc.1, and `Restart Now` relaunched it as
  rc.2 with the `self_replace` leftovers gone.
- **Unverified.** The executable's name in a release (`riffle-native` today
  in `Info.plist`, the `.desktop` file and `datadir.rs`'s tests).
- **Measured** (2026-11-17, the gpui-packaging plan). The version source is
  `[package].version` of `crates/native/Cargo.toml`, which
  `tools/package/version.sh` prints and every package and file name carries
  (the copied `Info.plist`'s two version fields, the setup's `AppVersion`,
  the deb's `Version`); the checked-in `Info.plist` stays `0.0.0`. It is
  `0.0.0` in every `Package` run. **Unverified**: `release-please-config.json`
  bumps neither `crates/native/Cargo.toml` nor `crates/native/Cargo.lock`
  yet; wiring them is sub-milestone 4's.
- **Measured** (2026-11-17, the gpui-packaging plan's Step 3, `Package` run
  44096260804 on `ubuntu-latest`). The `.desktop` file's `Exec`: the
  checked-in file keeps the bare `Exec=riffle-native` (`datadir.rs` tests
  it; `native:install:linux` installs it as is), and the packaged copies
  are absolute: `install.sh` writes the quoted
  `Exec="<home>/.local/bin/riffle-native"` and the deb carries
  `Exec=/usr/lib/handpic/riffle-native`. **Unverified**: a launch from the
  desktop menu on a real session.
- **Measured** (2026-11-17, the gpui-packaging plan's Step 3, `Package` runs
  44096260804 and 44099086987 on `ubuntu-latest`, Ubuntu 24.04.5, glibc
  2.39). The deb's `Depends` is `dpkg-shlibdeps`'s over the built binary,
  `libc6 (>= 2.39), libgcc-s1 (>= 4.2), libxcb1 (>= 1.12),
  libxkbcommon-x11-0 (>= 0.5.0), libxkbcommon0 (>= 0.5.0)`, plus
  `libvulkan1` and `libwayland-client0` added by hand: the binary's
  `NEEDED` names neither, because wgpu loads `libvulkan.so.1` (or
  `libEGL.so.1` for its GL fallback) and GPUI's `wayland-backend` loads
  `libwayland-client.so.0` at run time. So the packages need glibc 2.39 or
  later, the `ubuntu-latest` baseline. **Unverified**: the run time on a
  real machine, with and without a Vulkan driver (the GL fallback), and on
  X11 and Wayland.
- **Unverified.** What a hand reinstall over a Tauri install does on each
  platform: the Inno Setup install next to the NSIS / MSI one, with its
  uninstall entry and shortcuts; a leftover `Riffle.app`; an
  AppImage file and an rpm package of the Tauri app, formats now dropped,
  left for the user to remove.
- **Measured** (2026-11-17, the gpui-packaging plan's Step 3). The deb's
  `Package` name against the Tauri deb's: the native deb is `handpic`, while
  the shipped Tauri v1.10.0 deb is `Package: riffle` (its control file, read
  from the release's `Riffle_1.10.0_amd64.deb`; `main`'s unreleased
  `tauri.conf.json` would name it `handpic` with `Conflicts` / `Replaces`
  `riffle`). So `apt` does not upgrade a Riffle install in place; a
  hand-installed native deb sits beside it. **Unverified**: whether the
  native deb should declare `Conflicts` / `Replaces: riffle`, left to
  sub-milestone 4's real-device check.
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
- **Measured** (the native-updater plan, Step 5, `Package` run 44547625174
  on `windows-latest`). The setup started at quit gets the exe unlocked by a
  `[Code]` wait, not the Restart Manager: the app passes `/WAITPID=<its pid>`
  and `InitializeSetup` waits for that process (at most 60 s) before
  installing; the upgrade over a stand-in holding the exe open waited for it
  and exited 0. The app starts the setup detached from an `on_app_quit`
  handler registered after the judgment and settings drains.
- **Measured** (the same run). The relaunch after a silent setup run belongs
  to the setup: `Restart Now` adds `/RELAUNCH`, which turns on a `[Run]`
  entry starting the installed app; without it (a plain quit, or a setup
  run by hand) nothing starts.
- **Source** (`crates/app/src/main.rs`'s `Restart Now`,
  `crates/engine/src/update.rs`). For a portable, tarball or `.app` install
  `Restart Now` is GPUI's `cx.restart()` after the drains, towards the path
  read at launch (the bundle for a `.app`). Measured for the portable copy
  on 2026-11-21 (the test-feed table); unverified for the tarball and the
  `.app`.
- **Measured** (2026-11-21, the test feed above). The setup `.exe` the app
  downloads carries no Mark of the Web and ran at quit unattended; a
  standard account is left to the rc dry run.
- **Source** (the native-updater plan, Steps 2 and 6). The staged `.app`
  comes from `Handpic-<version>-macos-<arch>.tar.gz`, the ad hoc signed
  `Handpic.app` alone, zipsign-signed, published beside the DMG; the DMG is
  for people only. `self_update`'s bundle mode extracts it beside the
  installed bundle, `codesign --verify --strict` checks it, and the two are
  swapped by renames.
- **Unverified.** Whether the `.app` swap works where the app is installed:
  a standard user's `/Applications`, or an app run from the DMG or from
  Downloads under App Translocation (where the engine refuses with the move
  message, unit-tested). No macOS machine was at hand for the test feed;
  deferred to the rc dry run.
- **Unverified.** Whether Gatekeeper quarantines the swapped ad hoc bundle,
  and whether the folder-access grants survive a swap to a bundle with
  another ad hoc signature. Deferred to the rc dry run with the row above.
- **Source** (the native-updater plan, Steps 2, 5 and 6). zipsign signs only
  `.zip` and `.tar.gz`, so the setup `.exe` ships inside a signed
  `Handpic-<version>-windows-x64-setup.exe.zip`, checked by
  `self_update::verify_signature`, and the `.app` inside the signed macOS
  tarball, checked by `self_update`'s bundle mode. The DMG, the setup `.exe`,
  `install.sh` and the deb are for people and covered by `SHA256SUMS`
  only.
