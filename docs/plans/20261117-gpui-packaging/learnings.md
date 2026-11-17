# Learnings

## Step 1

- The GPUI crates carry two copyright holders, not one: `gpui-component`,
  `gpui-component-macros`, `gpui-base`, `gpui-kit` and `gpui-kit-assets`
  0.9.1 are Longbridge's (`Copyright 2024 - 2026 Longbridge`), while the
  `gpui-pre-*` 0.3.12 crates' `LICENSE-APACHE` reads `Copyright 2022 - 2025 Zed
  Industries, Inc.`. `crates/native/packaging/NOTICE-gpui` names both lines
  and then carries the Apache-2.0 text of `gpui-component`'s
  `LICENSE-APACHE` (from "Apache License, Version 2.0, January 2004" on).
  `gpui-kit` itself ships no `LICENSE-APACHE` file in its crate, only the
  `license` field. `gpui-pre-reqwest` is `MIT OR Apache-2.0`; it is covered by
  the Apache text.
- `crates/core/models/LICENSE` is YuNet's MIT license; `notices.sh` ships it
  as `LICENSE-yunet` so the names in `licenses/` do not collide with the
  themes' `LICENSE` (shipped as `LICENSE-themes`).
- The layout stays in `native:bundle:macos`, as the plan says. So the
  workflow runs `cargo build --release --locked` through `mise exec`, then
  `mise run native:bundle:macos` (whose own release `cargo build` finds the
  binary up to date), then `tools/package/macos.sh`, which only signs and
  makes the DMG. `macos.sh` stages the app with `ditto` rather than
  `cp -R`, so the ad hoc signature's extended attributes survive the copy.
- The DMG's `<arch>` comes from `uname -m` (`arm64` / `x86_64`), which is
  what the plan's file names use; the workflow's check step asserts the
  executable's `lipo -archs` equals the matrix's `arch`.
- A change to `mise.toml` selects every check in `mise run ci:local`, so
  this step's local run includes the root crates' and the native crate's
  clippy and tests.
- Nothing macOS-specific could run here (Windows host): `actionlint`,
  `shellcheck` and `local.test.sh` pass, `version.sh` prints `0.0.0` and
  `notices.sh` copies the six files, but `plutil`, `codesign`, `hdiutil` and
  the bundle and package tasks are only exercised by a GitHub run.

## Deferred issues (todo candidates)

- Pending check (Step 1, not done by the implementation agent; the PR phase
  does it): the `Package` run on the PR branch. The committed
  `.github/workflows/package.yml` is `workflow_dispatch` only, and a dispatch
  needs the file on the default branch. Steps: push a temporary commit adding
  `pull_request:` to its `on:`, wait for the `Package` run, check both
  `macos` legs and the `checksums` job are green, download the two DMGs and
  `SHA256SUMS` (`gh run download <run-id>`), check `SHA256SUMS` lists both
  DMGs with matching digests (`sha256sum -c`), then remove the trigger in
  another commit before the merge. Expected: `Handpic-0.0.0-macos-arm64.dmg`
  and `Handpic-0.0.0-macos-x86_64.dmg`, each holding an ad hoc signed
  `Handpic.app` with `0.0.0` in both version fields and
  `Contents/Resources/licenses/` with the six notices (the workflow's
  "Check the DMG" step asserts these on the runner). Step 1's checkbox stays
  unchecked until this is done. Source: plan Step 1, "Done when" (the
  `Package` run item) and the approach's temporary-trigger item; file
  `.github/workflows/package.yml`.
- Pending manual check (Step 1, macOS only, the user's): `mise run
  native:package:macos` on a Mac writes
  `crates/native/target/release/Handpic-0.0.0-macos-<arch>.dmg`; open it, drag
  `Handpic.app` to `Applications` and start it (Gatekeeper asks for the ad
  hoc signed app's first launch to be allowed under System Settings, Privacy
  & Security). Not runnable on the Windows host. Source: plan Step 1, "Done
  when" (`mise run native:package:macos`); files `mise.toml`,
  `tools/package/macos.sh`.
