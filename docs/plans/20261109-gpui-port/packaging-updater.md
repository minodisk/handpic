# Packaging and the updater for the switch-over

The decision record for Step 10 of [the gpui-port plan](./plan.md): how the
native app is packaged, how an install of it is updated, and what happens to
the Tauri installs already in the field. First written on 2026-11-14 by the
gpui-ci-packaging plan (`../_archived/20261114-gpui-ci-packaging/plan.md`),
and re-decided on 2026-11-16 by the gpui-packaging-redecision plan
(`../20261116-gpui-packaging-redecision/plan.md`). The earlier text
(`cargo-packager` and a self-written updater keeping the full `latest.json`
contract) is in git history only.

Each claim is tagged **Source** (read from the named source on 2026-11-16) or
**Unverified** (to be checked in the sub-milestone that needs it). Nothing
here comes from running a packaging tool, `self_update` or an update; no
packaging or updater code was written.

## The user's decision (2026-11-16)

- Auto-update for Tauri installs stops at the last Tauri release. Tauri users
  reinstall the native app by hand.
- Dropped with it: the `latest.json` manifest, the minisign key pair
  (`TAURI_SIGNING_PRIVATE_KEY`, `plugins.updater.pubkey`), NSIS
  `/P /UPDATE` compatibility, the MSI upgrade code, an rpm for shipped rpm
  installs, and the "real-device update from the last Tauri release" gate.
- The native app's packaging and updating follow gpui-kit's standard guides,
  https://gpui-kit.com/docs/packaging/ and
  https://gpui-kit.com/docs/auto-update/.

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

The gpui-kit-aligned default per platform. A line that depends on an open
question names it.

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
- **macOS, arm64 and x86_64 separately.** `Handpic.app` in a DMG. An update
  replaces the whole `.app` (who performs it: open question 1; signing and
  notarization: open question 3).
- **Windows x64.** An Inno Setup setup `.exe` and a portable ZIP (whether the
  ZIP ships: open question 6). The installer owns an installed copy's update
  (open question 1); `self_update` may update the portable copy. No MSI and
  no NSIS.
- **Linux x64.** A `.tar.gz` with `install.sh`, and a `.deb`. `apt` / `dpkg`
  owns a deb install's update; `self_update` may update the tarball install.
  rpm and AppImage: open question 4.
- **Every release** carries `SHA256SUMS` and no `latest.json` (authenticity
  beyond it: open question 2).
- **The update flow in the app.** A launch check and a manual check; the
  install's owner does the install (open question 1); restart to finish; the
  install or the relaunch only after the sidecar writer is drained on quit,
  as `crates/app/src/update.rs` does today.
- **Not taken.**
  - `cargo-packager` plus the self-written `latest.json` updater: its
    deciding fact, Windows NSIS `/P` compatibility with shipped installs, is
    gone with the user's decision.
  - `cargo-packager-updater`: a manifest dialect of its own for a contract
    nobody needs to keep any more.
  - `cargo-bundle`: no NSIS or Inno Setup, Windows support called
    experimental (**Source**: `cargo-bundle` 0.12.0's readme, as read on
    2026-11-14 by the earlier record), and nothing the platform-native tools above do not cover.

## Unverified

To be checked in the sub-milestone that needs each one.

- **Unverified.** How the running app knows its install type (portable, Inno
  Setup, DMG, deb), which "do not switch owners" requires.
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
  deb's, so that `apt` upgrades in place; a leftover `Riffle.app`.
- **Unverified.** That the native app picks up the Tauri install's data.
  **Source** (`CLAUDE.md`): `settings.rs` seeds its store from the Tauri
  `settings.json`, and the index is the native app's own. Not checked on a
  real install.

## Open questions

None is decided. Each is settled before the sub-milestone that needs it.

1. **Who performs the update of installer-owned installs** (Inno Setup, DMG,
   deb).
   - (a) Notify only: the app says a newer version exists and opens the
     release page; the user runs the installer, drags the new `.app` or
     installs the deb. Least code, no elevation or file-lock handling, and no
     authenticity question for these installs beyond the OS signature. A
     regression from today's silent updates on Windows and macOS.
   - (b) The app drives the owner: it downloads the setup `.exe` and runs it
     at quit; stages and swaps the whole `.app`; a deb stays with `apt` or a
     hand install, as today. Close to today's behavior, but it is
     per-platform install code, and it needs its own verification of what it
     downloads.
   - gpui-kit allows both and forbids only replacing the binary inside such an
     install.
2. **Publisher authenticity beyond `SHA256SUMS`.** Today's updates are
   minisign-verified; `SHA256SUMS` from the same GitHub release only proves
   integrity.
   - OS signatures (Developer ID plus notarization, Authenticode) checked on
     the staged artifact; nothing on Linux.
   - `self_update`'s `signatures` feature (`zipsign-api`): a verifying key
     compiled into the app and a new signing secret in CI; covers the
     portable archives only; unverified in detail.
   - Accept checksum-only over GitHub HTTPS, recorded as a step down.
   - With "notify only" in question 1, the question shrinks to the portable
     installs.
3. **OS code signing.** gpui-kit's standard is a Developer ID signed and
   notarized DMG and a signed exe, setup and uninstaller. Today nothing is
   OS-signed (`README.md` tells users how to pass Gatekeeper and
   SmartScreen).
   - Follow the guide: paid certificates and CI secrets, the user's call.
   - Stay unsigned (ad hoc on macOS): a deviation from the guide, and OS
     signatures are then no answer to question 2.
4. **rpm and AppImage.**
   - rpm: drop it (rpm-distribution users take the `.tar.gz` with
     `install.sh`), or add a separate recipe (`cargo-generate-rpm`,
     `rpmbuild`).
   - AppImage: the Linux download `README.md` leads with, and today's only
     self-updating Linux format. Either the `.tar.gz` + `install.sh` install
     with `self_update` replaces it, or it is kept with a tool outside
     gpui-kit's guide.
   - MSI is written above as dropped, since the guide offers Inno Setup or a
     ZIP.
5. **What the last Tauri release tells users, and which release that is.**
   - (a) v1.10.0, still named Riffle, stays the last one. No install is told
     anything in the app; the notice lives in the first native release's
     notes, the README and the usage guide; a menu check on a Tauri install
     shows an update error from then on.
   - (b) One final Tauri release from `main` (it would also ship the Handpic
     rename) whose release notes announce the reinstall. Installs update to
     it silently, so its notes are seen only on GitHub.
   - (c) As (b), with a one-time notice in the app. It needs a change in the
     frozen `crates/app`, an exception only the user can grant.
   - Independent of these: attach to later native releases a frozen copy of
     the last Tauri release's `latest.json`, pointing at that release's own
     assets. Installs older than it could still reach it, and a Tauri check
     would report "up to date" instead of an error. The cost is one legacy
     asset on every release and a rule that it never names a native
     artifact.
6. **Whether the portable Windows ZIP ships at all.** The guide says to
   choose only the formats users need. Without it, `self_update` has no
   Windows target and Windows updates rest wholly on question 1.
