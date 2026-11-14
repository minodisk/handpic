# Packaging and the updater for the switch-over

The decision record for Step 10 of [the gpui-port plan](./plan.md): which tool
packages the native app, how it updates itself, and what the first native
release must give the Tauri installs already in the field. Written by
the gpui-ci-packaging plan (`../20261114-gpui-ci-packaging/plan.md`) on
2026-11-14.

Each claim is tagged **Source** (read from the named source on 2026-11-14) or
**Unverified** (to be checked in the sub-milestone that needs it). Nothing
here comes from running `cargo-packager`, `cargo-bundle` or an update; no
packaging or updater code was written.

## The `latest.json` contract today

Sources: `crates/app/tauri.conf.json`, `.github/workflows/release.yml`, the
`latest.json` attached to v1.10.0, and `tauri-plugin-updater` 2.12.0's
`updater.rs` (the version in the root `Cargo.lock`).

- **Source.** The endpoint is
  `https://github.com/minodisk/handpic/releases/latest/download/latest.json`
  (`plugins.updater.endpoints`).
- **Source.** `tauri-action` writes `latest.json` and attaches it to the draft
  release; `updaterJsonPreferNsis: true` makes the plain `windows-x86_64` key
  point at the NSIS installer.
- **Source.** The shape is `version`, `notes`, `pub_date` and `platforms`, a
  map of key to `{ signature, url }`. The keys in v1.10.0 are
  `windows-x86_64`, `windows-x86_64-nsis`, `windows-x86_64-msi`,
  `darwin-aarch64`, `darwin-aarch64-app`, `darwin-x86_64`,
  `darwin-x86_64-app`, `linux-x86_64`, `linux-x86_64-appimage`,
  `linux-x86_64-deb`, `linux-x86_64-rpm`.
- **Source.** `signature` is the base64 of a minisign `.sig` file's text, made
  over the artifact's bytes with the key in `TAURI_SIGNING_PRIVATE_KEY`. The
  public key in `plugins.updater.pubkey` is the base64 of a minisign public
  key file. The trusted comment carries `timestamp`, `file` and `version`; the
  client checks `version` against the manifest only when a signature carries
  one (`require_signed_version` is off).

## What a shipped Tauri install does with it

Source: `tauri-plugin-updater` 2.12.0's `updater.rs`, and
`crates/app/src/update.rs` for when the install runs. This is what the first
native release must satisfy, since the installs in the field run this code
unchanged.

- **Source.** It looks up `{os}-{arch}-{installer}` (the installer type is
  baked into the binary by the bundler), then `{os}-{arch}`; updates when
  `version` is greater; downloads `url`; verifies the signature; then installs:
  - Windows, NSIS install: writes the bytes to a temporary `.exe` (a raw exe
    or a zip holding one) and launches it with `/P /UPDATE`, plus
    `/R /ARGS ...` on `Restart Now`.
  - Windows, MSI install: `msiexec /i <msi> /passive /promptrestart`. A major
    upgrade needs the same upgrade code
    (`15df1a23-08b6-56be-b9df-b38d92f7dd75` in `tauri.conf.json`).
  - macOS: untars the `.app.tar.gz`, drops the first path component and swaps
    the result into the existing bundle's path.
  - Linux, AppImage: writes the bytes (a raw AppImage, or a `.tar.gz` holding
    one) over the running AppImage's path.
  - Linux, deb / rpm: checks the bytes are a deb / rpm, then
    `pkexec dpkg -i` / `rpm -U`. An rpm install handed anything else fails
    with `InvalidUpdaterFormat`.
- **Source.** Nothing in the contract is private to Tauri: plain JSON,
  minisign, and per-platform artifact shapes. A native updater can read the
  same file with `reqwest` (already in `crates/backend`, rustls) and
  `minisign-verify` (already in the root lock, the crate the plugin uses),
  plus `tar` and `flate2` for macOS.

## The candidates

- **Source** (crates.io and the repository). `cargo-packager` 0.11.8
  (2025-11; commits on 2026-11-12): NSIS, WiX MSI, `.app`, DMG, deb,
  AppImage, pacman. No rpm. Its NSIS template descends from Tauri's: it reads
  `/P` and `/R` and uses the same `Uninstall\${PRODUCTNAME}` and
  `Software\${MANUFACTURER}\${PRODUCTNAME}` registry keys. It does not read
  `/UPDATE`. It ships a minisign signer.
- **Source.** `cargo-packager-updater` 0.2.3 (2025-07): the same manifest
  idea, but its macOS keys are `macos-<arch>`, not `darwin-<arch>`, and each
  platform entry needs a `format` field. The Tauri plugin's entry struct does
  not reject unknown fields, so one file could carry both sets.
- **Source.** `cargo-bundle` 0.12.0 (2026-09): `osx`, `deb`, `rpm`,
  `appimage`, and `msi` / `wxsmsi`, with Windows support called experimental
  in its readme. No NSIS, no updater signatures.

## Recommendation

- **Packager: `cargo-packager`.** The deciding fact is Windows: shipped
  installs launch the downloaded file as an NSIS installer with `/P`, and
  `latest.json` prefers NSIS. `cargo-bundle` cannot produce one.
  `cargo-packager` also covers every other format shipped today except rpm.
- **Updater: self-written, in `crates/backend`,** reading the unchanged
  `latest.json` keys with the same key pair. The same file then serves the
  shipped Tauri installs and the native installs with one set of keys, no
  second HTTP stack comes in, and the Windows install stays deferred to quit
  after the sidecar flush as `crates/app/src/update.rs` does it.
- **Not taken: `cargo-packager-updater`.** It would mean a second manifest
  dialect in the same file (`macos-*` keys, `format`), its own HTTP stack, a
  release from 2025-07, and still the quit-time deferral to write around it.
  The self-written updater costs the per-platform install code (about what
  `update.rs` plus the plugin's install functions do today) and its security
  review.
- So: keep the contract; no new one is written.

## Unverified

To be checked in the sub-milestone that needs each one.

- **Unverified.** Whether `cargo-packager`'s NSIS installer, launched with
  `/P /UPDATE` by a shipped install, upgrades in place. `/UPDATE` is ignored,
  so it takes its own reinstall path, which runs the old (Tauri) uninstaller
  with `/P`. Needs a real Windows install. Also the main binary's name
  (`riffle-app.exe` today, `riffle-native` in the port) and the shortcuts that
  point at it.
- **Unverified.** Whether `crates/app/nsis/hooks.nsh` (the legacy `Riffle`
  uninstall) can be carried into a `cargo-packager` NSIS template.
- **Unverified.** Whether `cargo-packager`'s WiX config can pin the upgrade
  code.
- **Unverified.** Whether `cargo-packager`'s signer accepts the existing
  `TAURI_SIGNING_PRIVATE_KEY` and produces a signature the public key in
  `tauri.conf.json` verifies. Whatever signs, the release job must verify
  each signature with that public key before uploading.
- **Unverified.** rpm. `cargo-packager` has no rpm target, and a shipped rpm
  install looks up `linux-x86_64-rpm` and rejects anything but an rpm. Either
  keep an rpm in the release with a second tool (for example
  `cargo-generate-rpm`), or drop rpm and tell those users to reinstall by
  hand. Decided in the packaging sub-milestone.
- **Unverified.** Whether the last shipped release is still named `Riffle`
  (v1.10.0's assets are) when the switch-over ships, which decides how many
  legacy names the installers must handle.
