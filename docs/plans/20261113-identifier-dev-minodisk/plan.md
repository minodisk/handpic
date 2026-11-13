<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Change the app identifier to dev.minodisk.handpic

## Purpose

The app identifier is `com.minodisk.handpic`, but the domain the author owns is
`minodisk.dev`, so the correct reverse-DNS identifier is `dev.minodisk.handpic`.
After this work the Tauri app, the native app and the documentation use the new
identifier, and data under either earlier identifier (`com.minodisk.handpic`,
`com.minodisk.riffle`) is moved to the new folders on the first launch.

No Handpic release has shipped yet (the newest tag, `v1.10.0`, is Riffle), so
`com.minodisk.handpic` folders exist only on machines that ran a build from
`main`. This should merge before the first Handpic release
(`../20261109-rename-to-handpic/plan.md`, Step 5).

The in-flight `gpui-settings` plan (another session) adds
`<config dir>/<datadir::IDENTIFIER>/settings.sqlite`, derived from
`datadir::IDENTIFIER` and seeded from the Tauri `settings.json` under
`IDENTIFIER` then each `LEGACY_IDENTIFIERS` folder. It needs no migration of
its own and will follow this change; this plan merges first (agreed with that
session).

## Steps

- [x] Step 1: Switch the identifier to `dev.minodisk.handpic` and migrate `com.minodisk.handpic` data
  - Done when:
    - `crates/app/tauri.conf.json` has `"identifier": "dev.minodisk.handpic"`; `bundle.windows.wix.upgradeCode`, `productName`, the updater `pubkey` and `endpoints` are unchanged.
    - `LEGACY_IDENTIFIERS` is `["com.minodisk.handpic", "com.minodisk.riffle"]` (newest first) in both `crates/app/src/migrate.rs` and `crates/native/src/datadir.rs`, and `datadir.rs`'s `IDENTIFIER` is `dev.minodisk.handpic`.
    - A test in `migrate.rs` shows, on temp directories only, that with both legacy folders present under one base the data of both ends up in the new folder, both old folders are gone, and on a file present in both the `com.minodisk.handpic` copy wins.
    - No test in `migrate.rs` calls `run` (or anything else that resolves the real `dirs::*` folders) with an identifier that leaves a legacy identifier to migrate.
    - `crates/native/src/settings.rs` tests cover: the current identifier's file wins; `com.minodisk.handpic`'s file is used when the current one is absent and wins over `com.minodisk.riffle`'s; the Riffle file is still used when only it exists.
    - `rg "com\.minodisk\.handpic"` finds matches only in the two `LEGACY_IDENTIFIERS` lists, tests, `docs/plans/_archived/**`, `docs/plans/review-history/**`, the sentence in `docs/agents/native-app.md` that names the earlier `.native` folder, this plan folder, and the historical records of `docs/plans/20261109-rename-to-handpic/` (see Trade-offs).
    - `mise run ci` passes (it covers the root workspace, the separate `crates/native` workspace and the frontend).
  - Implementation approach (as far as it is known; omit if unknown):
    - `crates/app/src/migrate.rs`:
      - `run` iterates `LEGACY_IDENTIFIERS` in list order and merges each legacy folder into the current one; an existing destination file wins (`move_file`), so the first-listed identifier wins a conflict between two legacy folders. Put `com.minodisk.handpic` first and state the order in a comment, as `datadir.rs` does ("newest first").
      - The existing test `the_current_identifier_is_not_a_migration` calls `run(LEGACY_IDENTIFIERS[0])` on the real OS directories. With two entries, that call would migrate a real `com.minodisk.riffle` folder of the developer into `com.minodisk.handpic`. Rewrite it so it touches no real directory. One way: split the base-dir-independent part out of `run` (the filter of the current identifier plus the loop over `pairs_under` / `migrate_pairs` for given bases) and test that part with temp bases; the multi-legacy order test uses the same seam. Keep the macOS leftover removal behind the same filter and the same `all_moved` condition as today.
      - The macOS leftovers (`WebKit`, `HTTPStorages`, `Preferences/<id>.plist`, `Saved Application State`) are then also removed for `com.minodisk.handpic`; that is the existing behavior applied to the new entry and needs no code change.
    - `crates/native/src/datadir.rs`: change `IDENTIFIER`, extend `LEGACY_IDENTIFIERS`, update the `.native` path in the test to `dev.minodisk.handpic.native`. Do not add a migration of `<cache dir>/com.minodisk.handpic.native`: the native app never shipped, and `docs/agents/native-app.md` already records the same decision for `com.minodisk.riffle.native` (pass 1 rebuilds the cache). The native `src/logger.rs` writes `Handpic.log` under its own folder; check whether that path derives from `IDENTIFIER` and update any literal that names the old identifier.
    - `crates/native/src/settings.rs`: `candidates` already chains `IDENTIFIER` and `LEGACY_IDENTIFIERS` in order, so only the tests change (literal folder names, plus the new legacy case).
    - `crates/native` is a separate Cargo workspace with its own `Cargo.lock`; no dependency changes are expected in either workspace.
    - Documentation and templates, replacing the identifier where it denotes the current product:
      - `.github/ISSUE_TEMPLATE/os.yml` (three log paths)
      - `CLAUDE.md` (the `<cache dir>/com.minodisk.handpic.native/index.sqlite` mention)
      - `CONTRIBUTING.md` (the index paths)
      - `docs/agents/app-log.md` (the path table; keep the column alignment the formatter expects)
      - `docs/agents/native-app.md` ("The native app keeps its own index": new folder name, and name `com.minodisk.handpic.native` next to `com.minodisk.riffle.native` as an earlier developer cache that is not migrated)
      - `docs/humans/usage.md` and `docs/humans/usage.ja.md` (the Open Log Folder paths; change both in this PR to keep the pair in sync)
      - Any other current-product occurrence `rg` finds (outside the exemptions above).
    - Leave as they are: the "Upgrading from Riffle" text in `usage.md` / `usage.ja.md` (it names `com.minodisk.riffle`, which is still what a 1.10.0 user has), `todo.md` (its matches are `com.minodisk.riffle`), `docs/plans/_archived/**`, `docs/plans/review-history/**`, other in-flight plans of other sessions, and `crates/app/nsis/hooks.nsh` (keyed on the product name, not the identifier). `README.md` and `README.ja.md` contain no occurrence.
    - `docs/plans/20261109-rename-to-handpic/` is an in-flight plan whose pending manual checks expect data under `com.minodisk.handpic`. Update only those forward-looking expectations to `dev.minodisk.handpic` (in `learnings.md`, the "Pending manual check, Step 2, first launch on real Riffle data" item; in `plan.md`, whatever Step 5 still has to verify) and leave the records of what was built and run.
    - No installer change is needed (see Trade-offs for the basis). Do not touch `bundle.windows`.
    - Verify during implementation if feasible: on the development machine, after one launch of a build with the new identifier, the data is under `dev.minodisk.handpic` in `%APPDATA%` and `%LOCALAPPDATA%`, no `com.minodisk.handpic` folder remains there, and the open folder, settings and ranking are intact. Record the result in `learnings.md`, or record it as a pending manual check if it cannot be run. Close any running Handpic first; a file held open makes the migration stop with its error dialog and resume on the next launch.

## Trade-offs and risks

- **Windows installers: no side-by-side install expected, read from the templates, not run.** The WiX and NSIS templates embedded in `@tauri-apps/cli-win32-x64-msvc` 2.11.5 key the install on the product name and the manufacturer: `UpgradeCode` is pinned in `tauri.conf.json` (its default would derive from `<productName>.exe.app.x64`, not the identifier), WiX's registry keys are `Software\{{manufacturer}}\{{product_name}}`, and NSIS's `UNINSTKEY` is `...\Uninstall\${PRODUCTNAME}`. The manufacturer defaults to the identifier's second element, `minodisk` in both the old and the new identifier. The identifier (`BUNDLEID`) appears only as the shortcuts' AppUserModelID, the URL protocol label, and the folders the uninstaller's "delete app data" checkbox removes. An install check on Windows belongs to the rename plan's Step 5, which already verifies the upgrade from Riffle 1.10.0.
- **AppUserModelID changes with the identifier.** A taskbar pin or jump list made by a `com.minodisk.handpic` build may not attach to the new build's window. Only machines that installed an unreleased build are affected.
- **macOS.** The bundle identifier changes, so the system asks again for folder access (TCC grants are tied to it) on a machine that ran a `com.minodisk.handpic` build. For a Riffle 1.10.0 user nothing changes against the rename plan: the identifier changes once either way. The release workflow has no Apple signing or notarization, so no certificate or provisioning profile refers to the identifier.
- **Single instance across identifiers.** `tauri_plugin_single_instance` names its mutex / socket / D-Bus name after the identifier, so a `com.minodisk.handpic` build and a `dev.minodisk.handpic` build running together do not see each other. Developer machines only; not handled.
- **The in-flight rename plan still names the old identifier.** Only its forward-looking expectations are updated; its historical records keep the old identifier and are exempt from the acceptance grep. Rewriting every occurrence would falsify records such as the build command that was actually run.
- **Whether `com.minodisk.handpic` needs to be a legacy identifier at all.** No release carries it, so the entry serves only machines that ran `main` builds (and the `gpui-settings` seeding). It is kept; the cost is one more list entry and, on macOS, the leftover cleanup for that identifier.
- **Merge order against the first Handpic release.** If a Handpic release shipped with `com.minodisk.handpic` before this merges, the legacy entry would become load-bearing for real users. Merging this first avoids that.
- **The native app's index is not migrated.** `<cache dir>/com.minodisk.handpic.native/index.sqlite` is left behind and the next native open runs a cold pass 1 into `dev.minodisk.handpic.native`. The orphaned folder can be deleted by hand.

## Progress

- (2026-11-13) Step 1 complete; the first-launch manual check is pending (see `learnings.md`)
