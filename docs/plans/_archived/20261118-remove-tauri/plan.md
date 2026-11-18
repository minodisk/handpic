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

# Remove the Tauri app and finish the Handpic rename

## Purpose

The GPUI port (`crates/native`) is now the only app. The frozen Tauri app
(`crates/app`), its frontend (`crates/app/ui`) and every JS / pnpm / Vite+
resource leave the repository, the remaining "Riffle" names (crates, binary,
env vars, types, docs) become Handpic, and the app's identifier is
`dev.minodisk.handpic` everywhere. Once done, the repository builds, tests,
packages and documents one app, and no contributor or agent has to work around
a crate that is never built locally.

This plan takes over three pieces of in-flight work and must annotate them
when the matching step lands:

- [the gpui-port plan](../../20261109-gpui-port/plan.md), Step 10: sub-milestone
  5's removal of `crates/app` (without the workspace merge, which stays
  there) and sub-milestone 6's docs.
- [the rename-to-handpic plan](../../20261109-rename-to-handpic/plan.md), Step 3
  (the crate rename).
- `todo.md`'s item "Native: drop the `.native` suffix from the native app's
  index and log folders at the switch-over".

Not in this plan: merging `crates/native` into the root workspace, the native
updater, a native release flow (`release.yml` building and attaching the
native packages), deleting backend code only the Tauri app used, and redrawing
the app icon. `crates/native`'s behavior does not change except where a step
says so (the binary's name in Step 2, the data folders in Step 3).

## Decisions

- (2026-11-18) The user chose the interim `release.yml`: keep only the
  `release-please` job and mark `/release` suspended until the native release
  flow (gpui-port Step 10, sub-milestone 4) lands.
- (2026-11-18) The native updater and release flow, when their sub-milestones
  come, follow gpui-kit's official guide (https://gpui-kit.com/docs/auto-update/)
  as closely as possible. Not part of this plan.
- (2026-11-18) The user asked that every `todo.md` item this work makes
  obsolete be deleted: each step deletes the items its own change closes or
  makes moot (an item about a removed file, task or tool), and Step 4 sweeps
  the rest.
- (2026-11-18) The user chose `handpic` as the native app's binary name
  (`[[bin]].name`); the package stays `handpic-native`.
- Step 2's binary name and Step 3's index option are asked of the user before
  each of those steps starts.
- (2026-11-18) The user chose Option 3 for Step 3: the `.native` index is
  moved into the plain folder and an index already there is renamed aside.
- (2026-11-18) The user chose to delete the Tauri measurements from
  `performance.md` / `.ja.md` (Step 5).
- (2026-11-18) The user chose to move the three remaining measurements of code
  the native app still runs from `performance.md` / `.ja.md` into
  `docs/agents/shared-crates.md`.

## Steps

- [x] Step 1: Remove `crates/app`, the frontend and the JS tooling; move the icons; fix CI, mise, release-please and the agent tooling
  - Done when:
    - `crates/app` (the crate, `ui/`, `icons/`, `nsis/`, `capabilities/`,
      `tauri.conf.json`), `package.json`, `pnpm-lock.yaml`, `tsconfig.json`
      and `vite.config.ts` are gone, and no `.ts`, `.css` or `.html` file is
      tracked. The only JavaScript left is
      `.claude/skills/merge-settings/scripts/merge.js`.
    - The icons the native app uses live in `crates/native/icons/`
      (`icon.ico`, `icon.icns`, `64x64.png`, `128x128@2x.png`, `source.png`)
      and every reader points there: `crates/native/build.rs`,
      `crates/native/src/icons.rs`, `crates/native/packaging/windows/handpic.iss`,
      the `native:bundle:macos` and `native:install:linux` tasks,
      `tools/macos/app-icon/gen.py`, and the `<img>` of `README.md` and
      `README.ja.md`.
    - The root `Cargo.toml` lists `crates/backend`, `crates/cli` and
      `crates/core` only and has no `[patch.crates-io]`; `Cargo.lock` holds no
      Tauri crate.
    - `mise.toml` has no `tauri:*` task and no `pnpm` tool; `fmt`, `lint` and
      `test` run no `pnpm` or `vp` command.
    - `.github/workflows/ci.yml` installs no WebView packages and no pnpm
      store cache; its `lint`, `test`, `native` and `ci` jobs still exist
      under those names (branch protection requires `ci`).
    - `.github/workflows/release.yml` has no Tauri build (see the approach for
      its shape) and `release-please-config.json` names no deleted file.
    - `tools/ci/local.sh` has no `vp-check` and no `riffle-app` /
      frontend-test "skipped" lines; `tools/ci/local.test.sh` matches.
    - `.claude/**` names no `crates/app` path, no `pnpm` / `vp` command and no
      "Tauri crate" exemption.
    - `CLAUDE.md`'s Layout section and `CONTRIBUTING.md` describe no
      `crates/app`, frontend or `tauri:*` task.
    - `git grep -n 'crates/app' -- ':!docs/plans' ':!CHANGELOG.md' ':!docs/agents' ':!docs/humans' ':!todo.md'`
      prints nothing (the three excluded doc areas are Steps 4 and 5).
    - `mise run ci:local` passes locally (the `mise.toml` change selects every
      check, the native clippy and tests included) and the PR's GitHub checks
      pass on all three platforms.
    - A `Package` run dispatched on the branch
      (`gh workflow run package.yml --ref <branch>`) is green: the macOS DMGs
      carry `icon.icns`, the Windows setup builds with its icon.
  - Implementation approach (as far as it is known; omit if unknown):
    - **Icons.** `crates/native/icons/` is inside the native crate, so a
      change to an icon now selects the native checks in `ci:local` (today's
      rule in `docs/agents/native-app.md` says a change to
      `crates/app/icons/icon.ico` does not). Move with `git mv`. Paths after
      the move: `build.rs` joins `icons/icon.ico` to `CARGO_MANIFEST_DIR`;
      `icons.rs` uses `include_bytes!("../icons/64x64.png")`; `handpic.iss`
      is in `crates/native/packaging/windows/`, so `SetupIconFile` is
      `..\..\icons\icon.ico`; the two mise tasks run with
      `dir = "crates/native"`, so `icons/icon.icns` and
      `icons/128x128@2x.png`. Not moved, deleted with the crate: the
      `Square*Logo.png`, `StoreLogo.png`, `32x32.png`, `128x128.png`,
      `icon.png` and `icons/menu/` (only the Tauri bundle and its muda menu
      read them). `tools/macos/export-menu-icons.swift` only writes
      `icons/menu/`, so it goes too.
    - `tools/macos/app-icon/gen.py` writes `source.png` and its header tells
      the reader to derive the other sizes with `pnpm exec tauri icon`. Point
      `PNG_PATH` at the new folder and replace the derivation note with a
      statement that no checked-in command derives `icon.ico` / `icon.icns` /
      the PNG sizes any more. Do not invent a replacement command here.
    - **Root workspace.** Remove the member and the whole `muda` patch block
      with its comment (an unused patch warns on every cargo run). Refresh
      `Cargo.lock` by building (`cargo check --workspace`), not
      `cargo update`, so only the removed crates leave it.
      `crates/native/Cargo.lock` does not change in this step.
    - **Source comments that name a frontend file as a live counterpart**
      become false and contain `crates/app`; reword each to the native file:
      `crates/backend/src/shortcuts.rs` (`MODIFIER_KEYS` "must stay in sync"
      with `keys.ts`), `crates/core/src/fullsharp.rs` (two constants "in
      `focus.ts`"; the native side is `crates/native/src/mark.rs`),
      `crates/core/src/eyes.rs` and `crates/core/models/LICENSE-mediapipe`
      (`facemesh.ts`; the native side is `crates/native/src/facemesh.rs`).
      Also the comments in `crates/backend` that say "both apps" or "the
      Tauri app keeps ..." as a present fact (`text.rs`, `mcp.rs`,
      `rename.rs`, `datefolder.rs`, `foldersidecars.rs`) and
      `crates/core/src/scan.rs`'s one mention. Comment-only edits; no code.
    - **"A port of `x.ts`" notes** in `crates/native/src/**` (about 100) and
      in `CLAUDE.md` stay as historical notes: they name a bare file, not a
      path, and the source is in git history. `CLAUDE.md` says once that
      those names are files of the removed Tauri frontend. Comments in
      `crates/native/src/**` that describe the Tauri app in the present tense
      are left for Step 4's pass, not this step.
    - **`mise.toml`.** Remove `tauri:dev`, `tauri:release:devtools`,
      `tauri:release` and the `pnpm` tool. `fmt`: drop the `vp fmt` line.
      `lint`: drop `pnpm install` and `pnpm exec vp check`. `test`: drop
      `pnpm install` and `vp test` with the comment. Reword `ci:local`'s
      description. Keep `node`: `merge.js`, `merge.test.sh` (run by `lint`)
      and `.claude/skills/develop/scripts/commit-settings.sh` call it.
    - **`ci.yml`.** `lint`: drop the pnpm store steps. `test`: drop the
      "Install the Linux WebView dependencies" step and the pnpm store steps;
      keep the cargo cache. Verify on the PR's checks that the root
      workspace builds on `ubuntu-latest` with no extra apt package.
    - **`release.yml` and release-please** (decided 2026-11-18): keep the
      `release-please` job so the release PR and `CHANGELOG.md` keep
      accumulating, delete the `build` and `publish` jobs, and drop the job
      outputs nothing reads. Remove the three `extra-files` entries
      `crates/app/Cargo.toml`, `package.json` and `crates/app/tauri.conf.json`;
      do not add `crates/native/Cargo.toml` (the native release flow owns
      that). `.claude/skills/release/SKILL.md` gets a first section saying
      releases are suspended until the native release flow lands and that
      `/release` stops there; `verify-release.sh` is left as is and named
      there as stale.
    - **`tools/ci/local.sh`.** Remove `vp_extensions`, the `vp` flag and its
      path rules, the `vp-check` check, `crates/app/*` from the `fmt root`
      rule, and the two fixed "skipped" entries; update the header and the
      `root_packages` comment. `local.test.sh` drops the `vp-check`
      expectations and the `crates/app` cases.
    - **`.claude/**`.**
      `skills/merge/scripts/check-merge-approval.sh` and its test: remove
      `crates/app/tauri.conf.json` from the release-shaping list and the test
      line that expects the rest of `crates/app` to be safe. The npm rule
      (`package.json | pnpm-lock.yaml | pnpm-workspace.yaml | .npmrc`) is
      removed with its test line, and the texts that list those files
      (`skills/develop/references/pr-merge-lifecycle.md`,
      `skills/develop/SKILL.md`, `skills/pr/SKILL.md`) follow. Reword the
      "never builds the Tauri crate / frontend tests" passages in
      `agents/implementer.md`, `agents/planner.md`, `agents/pr-check-fixer.md`
      (also its Vitest example) and `skills/develop/README.md`: what
      `ci:local` still leaves to the PR's checks is the other platforms and
      the tests of crates that depend on a changed one.
      `skills/delegate/SKILL.md` cites `docs/agents/tauri-app.md` for the
      Windows doubled-backslash rule; leave the citation until Step 4 moves
      that rule.
    - **`.gitignore`.** Drop `node_modules` and the three `crates/app` lines.
    - **`CLAUDE.md`.** Delete the `crates/app` and `crates/app/ui` runs of the
      Layout paragraph and the "frontend lives under" paragraph; fix the
      phrases that lean on them (`riffle-backend` "the Tauri-free backend
      both apps depend on", "must not depend on `crates/app`", the icon
      paths, "the Tauri app's line format", `mise run ci` formatting the
      frontend). `CONTRIBUTING.md`: "Building and running" becomes the
      native app's (`native:dev`, `native:release`, NASM), "Checks" loses
      `vp`, the stats note about Tauri assets is checked.
    - **Lychee.** `docs/agents/tauri-app.md` and `ui-styling.md` stay until
      Step 4, so every Markdown link still resolves. Archived plans link to
      neither `crates/app` nor the root JS files (checked with `git grep`).
    - **In-flight plans.** Add a note to the gpui-port plan's Step 10 (items
      5 and 6 are taken by this plan, the workspace merge stays) and to the
      gpui-packaging plan's Step 3 (the Linux icon now comes from
      `crates/native/icons/128x128@2x.png`).
    - This PR changes workflows, `release-please-config.json` and a
      lockfile, so the merge approval check will ask the user. Pushing a
      workflow change needs the `workflow` scope
      (`docs/agents/github-actions-workflows.md`).
- [x] Step 2: Rename the crates, the binaries and the remaining Riffle names to Handpic
  - Done when:
    - The packages are `handpic-core`, `handpic-backend`, `handpic-cli` and
      `handpic-native`; the binaries are `handpic-cli` and the native app's
      (`handpic-native` or `handpic`, asked of the user before this step
      starts); both `Cargo.lock`s are refreshed.
    - The `RIFFLE_*` environment variables are `HANDPIC_*`, the test temp
      folder prefixes are `handpic-…`, and the native view type `Riffle` is
      `Handpic`.
    - Packaging follows the binary: `Info.plist`'s `CFBundleExecutable`, the
      `.desktop` file's `Exec`, `handpic.iss`'s comment, `tools/package/macos.sh`
      and `windows.sh` (`binary=`), the `native:bundle:macos` and
      `native:install:linux` tasks, `ci.yml`'s bundle check and
      `package.yml`'s checks.
    - `git grep -n -i riffle -- ':!docs/plans' ':!CHANGELOG.md' ':!todo.md' ':!docs/humans' ':!*.lock'`
      prints only the exceptions listed below.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass; a
      `Package` run dispatched on the branch is green.
    - The rename-to-handpic plan's Step 3 carries a note that this step did
      it.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged, so no `riffle-app` and no `package.json`
      `name` remain to rename.
    - Mechanical: `[package].name` and the path dependency keys in the four
      `Cargo.toml`s, `[[bin]].name` in `crates/native/Cargo.toml`,
      `use riffle_core::` / `riffle_backend::` across `crates/backend`,
      `crates/cli`, `crates/native` and `crates/core/tests`, the
      `cargo test -p riffle-…` snippets in doc comments, `root_packages` in
      `tools/ci/local.sh` with `local.test.sh`, `.claude/agents/pr-check-fixer.md`,
      `CLAUDE.md`, `CONTRIBUTING.md`, `docs/agents/*.md` (log targets
      `riffle_native::…` and `riffle_backend::…` in `app-log.md`,
      `Riffle::…` in `native-app.md`, the `tract-onnx-inference.md`
      snippets). The crate directories keep their names.
    - Env vars: `RIFFLE_NATIVE_ALLOW_NO_SIMD` (`crates/native/build.rs`, its
      `rerun-if-env-changed`, `CONTRIBUTING.md`, `native-app.md`),
      `RIFFLE_EYES_FILES`, `RIFFLE_FACE_JPEG`, `RIFFLE_HEVC_CR3`,
      `RIFFLE_DETECTOR_*` (ignored tests). No workflow sets any of them.
    - `datadir.rs`'s packaging tests compare against `env!("CARGO_BIN_NAME")`,
      so they fail until `Info.plist` and the `.desktop` file are renamed
      with the binary; change them together.
    - `CLAUDE.md` says the diagnostics write to `Riffle.log`; the file is
      `Handpic.log`. Fix it here.
    - Kept on purpose, to be listed in the PR body:
      - `crates/native/src/datadir.rs` `LEGACY_IDENTIFIERS`
        (`com.minodisk.handpic`, `com.minodisk.riffle`) and the
        `settings.rs` tests over it (`the_riffle_file_is_used_when_only_it_exists`):
        a new settings store is seeded from an earlier install's
        `settings.json`.
      - `tools/macos/app-icon/gen.py` and `icon.svg`, which draw
        `RIFFLE 60FPS CULLER` into the icon's edge print, with the rendered
        icons: redrawing needs macOS and a derivation step that no longer
        exists in the repository (follow-up).
      - `CHANGELOG.md`, `docs/plans/**` and the history lines of `todo.md`.
      - `docs/humans/usage.md` / `.ja.md` "Upgrading from Riffle", rewritten
        in Step 5.
    - A developer's `~/.local/bin/riffle-native` from an earlier
      `native:install:linux` and a test install's `riffle-native.exe` are
      left behind by the rename; nothing was released under either name.
- [x] Step 3: Put the native app's index and log under `dev.minodisk.handpic`
  - Done when:
    - The default index is `<cache dir>/dev.minodisk.handpic/index.sqlite`
      and the default log `Handpic.log` under `dev.minodisk.handpic`
      (`~/Library/Logs/<id>/` on macOS, `<local data dir>/<id>/logs/`
      elsewhere); `--index` and `--log` still override them.
    - What happens to an existing `dev.minodisk.handpic.native` index, and to
      an index a Tauri build left in the plain folder, is the option the
      user chose (see Trade-offs), covered by unit tests over temp folders.
    - `datadir.rs`'s and `logger.rs`'s tests name the new paths; `CLAUDE.md`,
      `docs/agents/native-app.md` ("The native app keeps its own index") and
      `docs/agents/app-log.md` (one path table) say the same;
      `CONTRIBUTING.md`'s "The index" and
      `.github/ISSUE_TEMPLATE/os.yml`'s log paths, which already name the
      plain folder, are checked to be true now.
    - The `todo.md` item on the `.native` suffix is removed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - The identifier audit found `IDENTIFIER` (`dev.minodisk.handpic`) used
      consistently by `Info.plist`'s `CFBundleIdentifier`, the `.desktop`
      file's name, `Icon` and `StartupWMClass`, Inno Setup's `AppId` and the
      uninstall keys `package.yml` checks, `set_app_identity`, the window's
      `app_id`, the settings store's folder and the MCP server name
      (`handpic`). The only deviations are the two `{IDENTIFIER}.native`
      folders in `datadir.rs` (`index_path`) and `logger.rs` (`log_path`),
      and the docs that describe them.
    - No released build ever wrote `dev.minodisk.handpic` or
      `dev.minodisk.handpic.native`: 1.10.0 shipped as Riffle under
      `com.minodisk.riffle`. Only machines that ran builds from `main` hold
      either folder.
    - The backend's `index.rs` is the code both apps opened their index
      with, so a Tauri-built file in the plain folder has the same schema.
      Verify during implementation that the native app opens such a file and
      that rows of an older `EXTRACTOR_VERSION` are re-extracted, not
      dropped.
    - The log needs no move: the app starts a `Handpic.log` in the new
      folder and the old one stays where it was.
    - Any move of the index happens before `Index::open`, takes
      `index.sqlite` with its `-wal` and `-shm`, and is skipped under
      `--index`.
- [x] Step 4: Rewrite the agent docs and `todo.md` for one app
  - Done when:
    - `docs/agents/tauri-app.md` and `docs/agents/ui-styling.md` are gone.
      The sections of `tauri-app.md` that are still true of `crates/core`,
      `crates/backend`, CI or the tooling live in a guide that has no Tauri
      in its name or text, and the Hit / Measured / Inferred tag definitions
      the other guides link to live there too.
    - `docs/agents/app-log.md` describes one `Handpic.log` (no
      `tauri_plugin_log`, no `uncaught-js:` section, no `riffle_app` target,
      no data-migration row), and `docs/agents/native-app.md`,
      `github-actions-workflows.md`, `photolab.md`,
      `raw-metadata-parsing.md` and `tract-onnx-inference.md` name no Tauri
      app as a present thing and link to nothing removed.
    - `todo.md` holds no item whose work was in `crates/app` or
      `crates/app/ui` only.
    - `git grep -n 'crates/app' -- ':!docs/plans' ':!CHANGELOG.md' ':!docs/humans'`
      prints nothing.
    - `mise run ci:local` passes locally (lychee with `--include-fragments`
      included) and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 to 3 are merged, so the docs are written against the
      final crate names and data paths.
    - `tauri-app.md` is 3059 lines in four parts: "Rust side" (lines 17 to
      1751), "Frontend" (1752 to 2877), "CI" (2878 to 2955) and
      "Verification" (2956 on). Sort each `###` section into keep or drop.
      Drop: Tauri commands, events, menus, the updater, `tauri.conf.json`,
      `tauri icon`, the window lifecycle, every DOM / CSS / TypeScript /
      Vite+ / pnpm section, `tauri-action`, the `riffle-app` test notes and
      the `migrate` test note. Keep, reworded where they say "command" or
      "frontend": the index and scan rules (`SCHEMA_VERSION`,
      `EXTRACTOR_VERSION`, the `Scans` lock, eviction, the third pass), the
      sidecar and XMP / `.dop` rules, trash and restore, the JPEG decode
      measurements, `notify` on Windows, chrono's `format`, case-insensitive
      paths, the release-please and CI rules that still apply, and the
      Windows shell rules (the Python heredoc and the doubled backslash).
    - Links that must keep resolving: `raw-metadata-parsing.md` links to the
      Sony MakerNote section by anchor; five guides link to the tag
      definitions; `native-app.md` cites the `Writer::set` `#[allow]` section
      and the `configure` behind `Unwatch` section by name;
      `.claude/skills/delegate/SKILL.md` cites the doubled-backslash rule.
      The Sony section may fit `raw-metadata-parsing.md` itself.
    - `native-app.md` mentions Tauri on about 170 lines. Most compare the
      port with the original; reword those that state a present fact ("the
      Tauri app keeps", "shared with the Tauri app", "never point a run at
      the Tauri app's index", the rule that a signature change in the shared
      crates can break the frozen Tauri crate) and leave past-tense port
      history.
    - `todo.md`: 227 items, 107 titled `App: …`, 79 `Native…`. An `App:` item
      is not automatically dead: one about behavior the native app also has
      (a filter rule, a sidecar rule) is retitled and re-pointed at the
      native files; one about the webview, the Tauri menu, the updater or
      the frontend is deleted. Use the `todo-curator` agent's conventions.
    - Add one `todo.md` item for the backend code only the Tauri app
      reached (candidates under Trade-offs), and one for the icon redraw.
    - A light pass over `crates/native/src/**` comments that speak of the
      Tauri app in the present tense (for example `datadir.rs`'s "apart from
      the Tauri app's", `logger.rs`'s "Tauri's `app_log_dir`") belongs here.
      Comment-only.
- [x] Step 5: Rewrite the user docs for the GPUI app
  - Done when:
    - `README.md` and `docs/humans/usage.md` describe what the native app
      does and nothing it lacks, and `README.ja.md` and
      `docs/humans/usage.ja.md` say the same in Japanese, in the same PR.
    - The install and update text matches the release state (no native
      release yet; see Trade-offs).
    - `docs/humans/performance.md` / `.ja.md` name no `tauri:*` task and
      present no Tauri measurement as the current app's.
    - `git grep -n -i -E 'crates/app|tauri' -- README.md README.ja.md docs/humans`
      prints only lines that name the earlier app as history.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 to 4 are merged.
    - The native app has no menu bar, so every `File > …`, `Edit > …`,
      `Help > Open Log Folder` and `Handpic > Settings...` reference in
      `usage.md` needs its native equivalent (a key, the title bar's gear,
      a right-click item) or removal.
    - What the native app does not have is listed in
      `crates/native/src/companion.rs`'s coverage table as `not_in_native`:
      compare, the `recommended` sort, the Genre filter, the file rename,
      the ranking refit and the AI switch. The in-app updater and the
      `Timing logs` setting are also absent. Check each `usage.md` feature
      against the native source rather than against this list alone.
    - `usage.md`'s "Installing", "Updating" and "Upgrading from Riffle"
      describe the Tauri installers, updater and data migration, all
      removed. The native app only seeds a new settings store from an
      earlier install's `settings.json` (`LEGACY_IDENTIFIERS`).
    - The Lucide notice path `crates/app/ui/LICENSE-lucide` (twice in
      `usage.md`) becomes `crates/native/LICENSE-lucide`.
    - `performance.md` mixes measurements of `crates/core` and
      `crates/backend` (still valid), of the native pager, and of the Tauri
      app's page latency. Keep every number with its conditions; the
      decision is whether the Tauri sections are deleted or kept under a
      heading that says they measure the retired app.

## Trade-offs and risks

- **What `release.yml` becomes (Step 1; decided 2026-11-18).** The `build`
  job is `tauri-apps/tauri-action` and cannot outlive `crates/app`.
  - Taken: keep the `release-please` job only, and mark `/release` suspended.
    The release PR and the changelog keep accumulating. If the release PR is
    merged by mistake, release-please tags and creates a draft release with
    no assets; nothing publishes it, and `/releases/latest` stays at 1.10.0.
    This matches the gpui-port plan: "no release is cut from `main` before"
    the native release flow.
  - Rejected A: build the native release flow now. It depends on the updater
    and on the Linux packages, and it is a plan of its own.
  - Rejected B: delete `release.yml` or leave it `workflow_dispatch` only.
- **Tauri installs get no further update.** The updater public key and the
  `latest.json` flow leave with `crates/app`. The user already dropped the
  update path from Tauri installs on 2026-11-16 (gpui-port plan, Progress).
  The `TAURI_SIGNING_*` repository secrets become unused; removing them is
  the user's.
- **Where the icons go.** Taken: `crates/native/icons/`, because every
  reader is the native crate or its packaging and a change there then runs
  the native checks. A top-level `assets/icons/` keeps `README.md`'s image
  out of a crate folder but leaves the icon outside every crate's change
  detection.
- **`vp check` was also the formatter gate for JSON, YAML and TOML.**
  Removing Vite+ means nothing checks the formatting of the workflows,
  `mise.toml` or `crates/core/i18n/*.json` any more. Taken: accept it, as the
  request is to remove every frontend tool.
- **`node` stays in `mise.toml`.** It is not frontend tooling:
  `.claude/skills/merge-settings/scripts/merge.js`, its test (run by
  `mise run lint` and CI) and `commit-settings.sh` need it.
- **The npm rule of the merge approval check.** Taken: remove it with the
  files it guards.
- **The binary's name (Step 2; ask before it starts).** `handpic-native`, the
  name the rename-to-handpic and gpui-packaging plans already use, or plain
  `handpic`, more natural with one app left. Either way it should be settled
  before the first native release.
- **The index when the `.native` suffix goes (Step 3; ask before it
  starts).** Only machines that ran builds from `main` are affected. Such a
  machine can hold two indexes: the native app's under
  `dev.minodisk.handpic.native` (recent) and a Tauri build's under
  `dev.minodisk.handpic` (older, but with `embeddings` and the ranking's
  `views`). Judgments are in the sidecars either way.
  - Option 1: no move. The app opens whatever is in the plain folder, else
    starts empty.
  - Option 2: move the `.native` index into the plain folder only when the
    plain folder has no index.
  - Option 3 (recommended): the `.native` index wins. It is moved into the
    plain folder, and an index already there is renamed aside rather than
    deleted.
  - Option 4: keep the suffix.
- **`tauri-app.md`: salvage or delete (Step 4).** Taken: keep the sections
  that are about the shared crates and the tooling under a new name.
- **The Tauri measurements in `performance.md` (Step 5).** Delete them, or
  keep them under a heading that names the retired app.
- **README install text before a native release exists (Step 5).** Taken:
  say the packaged builds are not released yet and point to building from
  source.
- **References the acceptance criteria cannot reach.** Non-archived
  `docs/plans/**` and `CHANGELOG.md` keep their `crates/app` and Riffle text
  as history; the plan only annotates the in-flight plans it supersedes.
- **Backend code only the Tauri app reached is left in place.** Candidates
  for the follow-up: `riffle_backend::ai`, `index::run_embed_scan` with the
  `embeddings` table and `Index::rank_rows` / `rank_dirs`,
  `riffle_core::rank`, `genre` and `embed`, and the seven-tool default mode of
  `mcp::Mcp`. The AI features are more likely to be wired up than deleted.
- **The icon still reads RIFFLE.** The artwork's edge print and its
  generator keep the old name; a redraw needs macOS.
- **Parallel work.** Other plans in flight touch `mise.toml`, `ci.yml`,
  `package.yml` or `CLAUDE.md` (mise-rust-analyzer, gpui-packaging,
  native-scrollbar-gutter). Steps 1 and 2 rewrite large parts of those
  files; expect rebases.
- **`CLAUDE.md` is edited by Steps 1, 2 and 3.** A long-lived branch of any
  of them will conflict in the Layout paragraph.

## Progress

- (2026-11-18) Step 1 complete
- (2026-11-18) Step 2 complete
- (2026-11-18) Step 3 complete
- (2026-11-18) Step 4 complete
- (2026-11-18) Step 5 complete
