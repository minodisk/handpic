# Learnings

## Step 1

- The cause read from source was right. With the two UI tests written and the
  fix absent, both failed at the first check after the right press
  (`the right press kept the keyboard`: the view's handle had the focus, not
  the menu's), on the Windows host's test platform, the strip's on the first
  run and the tree's on a second run with the fix taken out again. With
  `window.prevent_default()` in `open_context` both pass.
- The opened temporary folder is never a tree row: its name starts with
  `.tmp` and it sits under `AppData`, both hidden from the tree (on Linux and
  macOS the leading dot alone hides it). The tree test presses the first
  root's row instead (a `tree-first-row` selector on row 0, the list scrolled
  to the top first) and compares the clipboard with that row's path and name.
  The strip's cells `div` carries `strip-cells`; the second cell is hit at
  its origin plus one pitch.
- `Strip::geometry` is private; the test reads `geometry()`.
- `PopupMenu`'s `right` / `left` bindings propagate in a menu without
  submenus, so with the menu focused `right` still reaches `key_down`, which
  closes the menu and pages. GPUI dispatches a bound action before the
  key-down listeners, so Up / Down / Enter stop at the menu.
- The other `default_prevented` reader on the same press is the
  `active` (pressed) style of an ancestor `div`; none in the view's ancestors
  of a tree row or a strip cell draws one for a right press, so nothing else
  changes. The old menu's `on_mouse_down_out` runs in the capture phase,
  before the new open, so a second right-click still replaces the menu.
- The app crate's test build takes 2 to 3 minutes incrementally here; a
  `timeout 590` wrapper killed one cold compile (exit 143).
- `mise run ci:local` (one run): fmt, lychee and both clippy runs (with and
  without `screenshot`) passed; its `test app` stage compiled from cold under
  the 1.98.1 toolchain (the earlier `cargo test` runs used 1.97.1, so nothing
  was reused) and was killed with the host critically low on memory, so the
  app's full test suite was left to the PR's GitHub checks. The two new UI
  tests had passed locally under `cargo test --bin handpic menu_takes`.

## Deferred issues (todo candidates)

- Pending manual check (Windows 11, this host): the right-click menus driven
  by the keyboard in a real window, as the plan's "Trade-offs and risks"
  describes. Needs a build with the `screenshot` feature, its own `--index` /
  `--settings` / `--log` in a temp folder, `--mcp-port 47881` and
  `--import-settings` with `{"mcpEnabled": true}`, a folder of test photos,
  synthetic key presses with the window checked foreground by PID, and MCP
  `get_view` / the clipboard to read the result. Steps: on the tree,
  right-click a row, then Down, Down, Enter: `Copy Path` runs (the path on
  the clipboard); Down, Down, Down, Up, Enter: the same. On the strip,
  right-click a cell, then Down, Down, Down, Enter: `Copy Path` (the reveal
  item is the second, so never Down, Down, Enter there), and the current
  file's position unchanged after the Downs. Do not move the pointer after
  the right-click (hover sets the highlight). Record the run in `results.md`.
  The step's checkbox was ticked on the automated criteria (the UI tests).

## Manual check (2026-11-28, wrap-up)

- The pending check above was run on this host and passed on both menus
  (`results.md`). It needed no foreground window: messages posted to the
  app's own `hwnd` (found by PID) drove the right-click and the arrow keys
  while the user's terminal stayed in front, so the shared keyboard and
  pointer were never touched.
- Found on the way: the strip's `Copy Path` copies the verbatim `\?\`
  prefix the tree's leaves out; filed in `todo.md`, not fixed here.
