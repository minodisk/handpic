# Learnings: remove-tauri

## Step 1

- The plan's list of icon readers missed one: `tools/package/linux.sh` also
  read `crates/app/icons/128x128@2x.png` for the tarball and the `.deb`. It
  now reads `crates/native/icons/128x128@2x.png`. A `git grep 'app/icons'`
  over the whole tree, not only the listed files, is what found it.
- `cargo check --workspace` refreshed `Cargo.lock` with only removals: a
  package-set diff of the old and new lock (parsed with `tomllib`) showed 299
  packages gone and none added or bumped. The `git diff --stat` count of
  "insertions" comes from the dependency lists of the surviving packages being
  rewritten (a `"syn 3.0.6"` style disambiguation flips when one of two
  versions leaves), not from new packages.
- On this host, the Bash tool's heredocs and `sed` replacements lose
  backslashes (`..\..\icons\icon.ico` became `....iconsicon.ico`, and a
  Python string with `\\` became one with a line continuation). Edits whose
  text holds a backslash went through the Edit tool or a Python script
  written with the Write tool.
- `tools/ci/local.sh`'s "skipped" list was never empty before (it always held
  the two fixed Tauri / frontend entries). With them gone, a run that selects
  every check would print an empty list, so it now prints `(none)` then.
- `todo.md` items deleted under the plan's 2026-11-18 decision (each about a
  file, task or tool this step removed): "App: the muda template-icon fork is
  a temporary bridge" (the `[patch.crates-io]` block and
  `tools/macos/export-menu-icons.swift` are gone), "Tooling: format JSON and
  YAML without node once the Tauri app is gone" (vp is gone and the plan
  accepts having no JSON / YAML / TOML formatter), and "App: the silent update
  path is unverified end-to-end" (the Tauri updater artifacts left with the
  release build). The many items that run `mise run tauri:dev` to check Tauri
  app behavior were left for Step 4's sweep.
- `docs/agents/native-app.md` still says a change to
  `crates/app/icons/icon.ico` does not select the native checks; that is
  false now (the icons are inside `crates/native`), and Step 4 rewrites that
  guide.
- The release skill's new first section stops `/release` and names
  `verify-release.sh` as stale; `release.yml` keeps its `workflow_dispatch`
  trigger, which now dispatches no job (the `release-please` job runs only on
  a push).

## Step 2

- The native crate is bin-only, so its crate name, and with it its `log`
  target and `CARGO_CRATE_NAME` (the logger's own-lines filter), comes from
  `[[bin]].name`, not from the package: with the package `handpic-native`
  and the binary `handpic`, the target is `handpic::scan`, not
  `handpic_native::scan`. `logger.rs`'s test line and `app-log.md` say
  `handpic`.
- The new crate name `handpic` is a string prefix of `handpic_backend` and
  `handpic_core`, so the old `starts_with(CARGO_CRATE_NAME)` filter let the
  backend's debug lines through under `RUST_LOG=debug` (review round 1).
  `logger.rs`'s `is_own` now matches the target exactly or as a `handpic::`
  module path, and a unit test keeps `handpic::scan` and drops
  `handpic_backend::photolab`.
- `datadir.rs`'s packaging tests compare `Info.plist` and the `.desktop`
  file against `env!("CARGO_BIN_NAME")`, so they needed no edit; only the two
  packaging files did.
- `cargo metadata --offline` in the root and in `crates/native` was enough to
  rewrite both `Cargo.lock`s for the renamed path packages; no dependency
  version moved.
- Kept on purpose beyond the plan's list: the `riffle_app` / `riffle-app`
  names in `docs/agents/app-log.md`, `native-app.md`, `tauri-app.md` and
  `tract-onnx-inference.md`. They name the removed Tauri crate as history or
  in Tauri-only rules that Step 4 rewrites or deletes; renaming them would
  invent a `handpic-app` that never existed. Also `native-app.md`'s
  `com.minodisk.riffle.native` and the v1.10.0 deb's `Package: riffle`
  (history).
- `todo.md` had an item on the same `hdiutil` failure ("sometimes fails",
  run 44081604694); it was rewritten in place with the warm-cache evidence of
  run 44126142482 and the new Done-when rather than added as a duplicate. No
  `todo.md` item was closed by this step alone.

## Step 3

- The index move (`datadir::move_native_index`, run by `open_index` only
  without `--index`) renames the plain folder's `-wal` / `-shm` aside with its
  `index.sqlite`. Leaving a stale `-wal` beside the moved-in file would let
  SQLite replay another database's pages into it.
- A Tauri-built index the native app opens as is (no `.native` index on the
  machine) is read through the same `Index::open` both apps used: its
  `PRAGMA user_version` migrations run, and rows whose `files.extractor` is
  not the current `EXTRACTOR_VERSION` are re-extracted by `reconcile`, not
  dropped (`crates/backend/src/index.rs`). Checked by reading the code and its
  migration tests, not by opening a real Tauri file.
- `CONTRIBUTING.md`'s "The index" paths (macOS `~/Library/Caches/...`,
  Windows `%LOCALAPPDATA%\...`) and `.github/ISSUE_TEMPLATE/os.yml`'s log
  paths already named `dev.minodisk.handpic` and are now true of the native
  app. `docs/agents/app-log.md`'s native section now points at the top table
  instead of keeping a second one; Step 4 rewrites the rest of that file.

## Step 4

- `tauri-app.md`'s kept sections now live in `docs/agents/shared-crates.md`
  ("Working on the shared crates and the tooling"), with the tag definitions.
  The Done-when asks for no "Tauri" in the new guide's text, so it calls the
  removed app "the earlier app" / "the earlier webview app" where a section
  needs it as the place a lesson was learned. The Sony MakerNote section
  moved into `raw-metadata-parsing.md` itself (same heading, so its anchor is
  unchanged and the link there is now an in-page one).
- Several kept sections described Tauri-only machinery around a still-true
  backend rule (`Scans`, `ScansState`, `spawn_eviction`, `set_scan_focus`,
  `AppReadSlots`, `AppListing`, the `ranking.json` store, the
  `preview_during_pass_1_on_a_real_folder` and `embed_pass_on_a_real_folder`
  harnesses). None of those exists in the repository any more (checked with
  `git grep`); the sections now state the backend rule and point at the
  native counterpart (`idle::IdleGate`, `cache::ClearFlow`, `native-app.md`)
  or say the harness left with the Tauri crate.
- `objc2-exception-helper` (the reason `cargo check --target
  aarch64-apple-darwin` failed here) is in neither `Cargo.lock` now; whether
  the backend alone checks for that target here was not tried, and the guide
  says so rather than repeating the old claim.
- `todo.md`: 236 items before, 161 after. 77 deleted as Tauri-only (checks of
  the Tauri UI, menus, updater, webview, `migrate.rs`, Tauri commands and
  frontend modules; the native app has its own check items for the same
  features). 29 kept items were retitled from `App:` to `Core:`, `Backend:`
  or `Native:` and re-pointed at the backend / native files, four of them
  rewritten (the macOS / Linux trash checks, the closed-eyes label review
  split from its Tauri GUI checks, the scan priority's macOS branch, the
  NVMe read cap). Two items were added: the backend code only the Tauri app
  reached, and the icon that still reads RIFFLE.
- Per Step 2's deferred note, the stale binary, CLI and code names were
  renamed in `todo.md`'s actionable lines only (unchecked TODO lines and the
  "Build in ..." instructions: `riffle-native.exe` -> `handpic.exe`,
  `riffle-cli` -> `handpic-cli`, `[riffle_native][DEBUG]` ->
  `[handpic][DEBUG]`), plus `RIFFLE_DETECTOR_*` and `Riffle::` everywhere
  (current env and type names). History lines keep `riffle-cli`.
- The plan's Step 1 note that archived plans link to neither removed guide
  was wrong: `docs/plans/_archived/20261001-photolab-guide/plan.md` linked
  `tauri-app.md` and `docs/plans/_archived/20261006-pending-rename-display/plan.md`
  `ui-styling.md`, and the first `ci:local` run's lychee failed on both. The
  links became plain names marked "removed since". `git grep` for
  `](...tauri-app.md` under `docs/plans` before deleting a guide finds them.
- The bulk edits went through Python scripts written with the Write tool and
  JSON replacement pairs that each had to match exactly once; a pair that
  matched zero or several times stopped the run, which caught nothing here
  but would have caught a stale anchor text.

## Deferred issues (todo candidates)

- **Pending check: a `Package` run dispatched on the branch.** Step 1's
  Done-when asks for `gh workflow run package.yml --ref remove-tauri-step-1`
  to be green: the macOS DMGs carry `icon.icns` (now
  `crates/native/icons/icon.icns`, copied by `native:bundle:macos`) and the
  Windows setup builds with its icon (`handpic.iss`'s
  `SetupIconFile=..\..\icons\icon.ico`), and the Linux tarball / `.deb` carry
  the icon `tools/package/linux.sh` now reads from `crates/native/icons/`.
  The implementer does not push, so this is for the PR stage: dispatch it
  after the push and check all legs pass. The step's checkbox was ticked on
  the locally checkable criteria. Files: `.github/workflows/package.yml`,
  `crates/native/packaging/windows/handpic.iss`, `mise.toml`,
  `tools/package/linux.sh`.
- **Pending check: the root workspace on `ubuntu-latest` with no WebView
  packages.** `ci.yml`'s `test` job no longer installs `libwebkit2gtk-4.1-dev`
  and friends; the PR's `test (ubuntu-latest)` check confirms that
  `riffle-core`, `riffle-backend` and `riffle-cli` build without them. Basis:
  the plan's `ci.yml` approach. File: `.github/workflows/ci.yml`.
- **`todo.md` still names the old binary, CLI and env vars in its
  actionable steps.** Step 2 renamed the native binary to `handpic`
  (`handpic.exe`), the CLI to `handpic-cli`, the view type to `Handpic` and
  the `RIFFLE_*` env vars to `HANDPIC_*`, but `todo.md` is outside Step 2's
  grep and was left alone: its TODO lines still say `riffle-native.exe`,
  `target\debug\riffle-native.exe`, `riffle-cli`, `[riffle_native][DEBUG]`
  (now `[handpic][DEBUG]`), `RIFFLE_DETECTOR_*` and `Riffle::`. Step 4's
  `todo.md` sweep should rename them in the steps a reader would run, keeping
  history lines. Basis: Step 2's rename. File: `todo.md`.
- **Pending check: a `Package` run dispatched on the Step 2 branch.** Step
  2's Done-when asks for it to be green with the binary renamed to `handpic`
  (`Contents/MacOS/handpic`, `handpic.exe` in the setup and the ZIP,
  `bin/handpic` and `/usr/lib/handpic/handpic` on Linux). The macOS legs may
  hit the `hdiutil` no-space failure the rewritten `todo.md` item tracks,
  which is not caused by the rename. The step's checkbox was ticked on the
  locally checkable criteria. Files: `.github/workflows/package.yml`,
  `tools/package/*.sh`, `crates/native/packaging/**`, `mise.toml`.
- `.github/ISSUE_TEMPLATE/os.yml` tells the reporter that `Help > Open Log
  Folder` opens the log folder, but the native app has no such menu item
  (nothing under `crates/native/src` names it; only the Tauri app had it).
  Found while checking the template's log paths in Step 3, which are
  correct. Either add an equivalent to the native app or drop the sentence.
  Files: `.github/ISSUE_TEMPLATE/os.yml`, `docs/agents/app-log.md` (its "Where
  the file is" bullets say the same).
- Pending manual check (Step 3): on Windows, with an index under
  `%LOCALAPPDATA%\dev.minodisk.handpic.native\` (from a `main` build) and
  optionally a Tauri build's under `%LOCALAPPDATA%\dev.minodisk.handpic\`,
  launch the native app without `--index`. Expect `Handpic.log` (now under
  `%LOCALAPPDATA%\dev.minodisk.handpic\logs\`) to show
  `index: moved in from the .native folder` (with the aside name when a
  Tauri index was there), `index.sqlite` (and its `-wal` / `-shm`) gone from
  `.native` with the folder kept because the old `logs\` is still in it, the
  Tauri index kept as
  `index-aside-1.sqlite`, and the open folder's thumbnails shown without a
  cold pass 1. The step was ticked on the unit tests over temp folders.
  Files: `crates/native/src/datadir.rs`, `crates/native/src/main.rs`
  (`open_index`).
- `todo.md`'s "Backend: re-check that notify 8.2.2 still queues `configure`
  behind `Unwatch` when the native app starts using `watch`" may already be
  met: `docs/agents/native-app.md` (the `crates/native/Cargo.lock` rule under
  "The backend is compiled twice", and the tree-watch section) records that
  the tree watcher's `unwatch` + `settle` and the folder rename's release
  path return promptly on 8.2.2. Step 4 only re-pointed the item at
  `shared-crates.md`, since it is not made moot by the Tauri removal. Check
  the item against those measurements and close it or narrow it. Files:
  `todo.md`, `docs/agents/native-app.md`, `docs/agents/shared-crates.md`.
