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
