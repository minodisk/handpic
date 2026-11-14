# Learnings: gpui-component-dep

## Step 1

- `cargo update -w` re-resolved only the changed dependencies: the lock moved
  from Zed's git GPUI at `b5cd310b` to the crates.io `gpui-pre-*` `0.3.12`
  crates plus gpui-component's graph (`gpui-base`, `gpui-component-macros`,
  `gpui-kit-assets` at 0.9.1, `gpui-pre-reqwest` 0.12.15, a second `windows`
  line at 0.58, `zed-font-kit` and `zed-scap` now from crates.io). The lock
  diff is about +1800 / -335 lines.
- The lock's only git sources after the change are the two kept patches,
  `zed-industries/async-task` and `zed-industries/async-process`; the async-task
  patch is applied (the lock resolves `async-task` 4.7.1 to the git rev), and
  there is no `[[patch.unused]]`. Every `gpui-pre*` package is at 0.3.12, one
  version each.
- `cargo tree -e features -i gpui-pre-platform` shows only `default` /
  `multithreaded`, from riffle-native itself: nothing turns on
  `runtime_shaders`.
- No source file changed: the snapshot compiled with every `use gpui::...` as
  it was, and `mise run native:check` (clippy `-D warnings`, 697 tests) passed
  on Windows. No lint rejected the unused `gpui-component` dependency, so no
  `allow` was needed. The first build of the larger graph took several minutes
  locally; the CI legs' times are still to be seen against
  `timeout-minutes: 30`.
- Writing a file with Python's text mode on Windows turned `Cargo.toml`'s line
  endings into CRLF; rewrite in binary mode (or `newline='\n'`).

## Deferred issues (todo candidates)

- Pending manual check (Step 1, "The app runs as before on the developer's
  machine"): on the developer's Windows machine, run `mise run native:dev` on
  this branch and check against `main` that a folder opens, paging, the
  judgment keys, the sort / filter menus, the right-click menus, the settings
  modal and the tree's keyboard behave as before. Expected: no difference,
  since no source file changed and `gpui_component::init` is not called. The
  step's checkbox was ticked on the automated criteria (`cargo tree`, the
  lock, `mise run native:check`); the `native` CI job on Windows, Linux and
  macOS runs on the PR. Related: `crates/native/Cargo.toml`,
  `crates/native/Cargo.lock`, `docs/agents/native-app.md`.
- CI timing check (from the learnings extraction): read the build time of
  each leg of the `native` CI job (Windows, Linux, macOS) on this PR against
  `timeout-minutes: 30`; the lock change makes each leg's first run a nearly
  cold build of a larger graph. Done when every leg finishes inside the
  timeout with margin; otherwise raise the timeout and record it in the
  "gpui-pre snapshot" entry of `docs/agents/native-app.md`.
