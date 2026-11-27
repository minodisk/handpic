# Learnings: title bar usable while a confirmation dialog is open

## Step 1

- The bar's wrapper (`div().relative().flex_shrink_0()` holding `TitleBar`
  and the folder-name overlay) is built before `let main` in `render`, so
  the `#[cfg(test)]` probe (`on_mouse_down` bumping `Handpic::bar_presses`)
  can be added with a `#[cfg(test)] let` without a helper or a
  `#[cfg(not(test))]` arm; the wrapper is then passed whole to `deferred(..)`
  at the default priority 0.
- `open_settings` did not refuse on `clear.asking()`. The cache
  confirmation can be open with the settings closed: an MCP `clear_cache`
  without `mcpAllowDestructive` calls `press_clear`, which opens it
  directly. With the gear now reachable under a dialog, that refusal was
  added and covered by `the_settings_do_not_open_under_the_cache_confirmation`.
- `modal::backdrop` is positioned against `main` (absolute in taffy is
  relative to the parent), which starts at the window's top under the
  client-side padding, so `top(TITLE_BAR_HEIGHT)` lines it up with the
  settings layer's top edge.
- On this Windows host a bare `python3` in Git Bash hung until the tool's
  timeout (as `CLAUDE.md` warns for `python`); edits went through `perl` and
  `sed` instead.
- The local check was skipped under the host's standing rule (no `mise run
  ci:local` or local cargo builds or tests; GitHub Actions on the PR
  verifies). One `ci:local` run started before the rule reached this step
  was killed by the harness for low memory while still building third-party
  crates. So the change is not compiled here, and the new UI tests
  (`the_title_bar_lies_above_the_trash_dialog_and_keeps_its_focus`,
  `the_settings_do_not_open_under_the_cache_confirmation`) are unverified
  until GHA runs, as is the first one failing on the code before the change.

## Deferred issues (todo candidates)

- Pending manual check (the plan's Step 1 "Manual check", owned by the main
  agent's wrap-up): on Windows 11, real device, a build from
  `mise run app:dev:screenshot` (cargo `-j 2`) run with its own `--index`,
  `--settings`, `--log` and `--mcp-port 47880` on photos copied from
  `D:\Photos\tests\20261127-c5\step7\photos` into a folder of its own, with
  synthesized input sent only under an exclusive lock on
  `D:\Photos\tests\.ui-lock`. With the trash dialog open (strip right-click >
  `Move 3 Files to Trash…`) and again with the `Delete Sidecars…` dialog
  open: minimize gives `IsIconic`, maximize gives `IsZoomed`, a drag on the
  bar moves the window, close (last, on an asking dialog) closes the app; a
  screenshot shows the bar undimmed and the rest dimmed; after each bar
  press that keeps the window the dialog is still open and Enter / Escape
  still act on it; a press on the dimmed area below the bar does nothing.
  Needs: the Windows machine, MCP tools and screenshots, synthesized mouse
  input, the log, a build and the test photos. Record it in `results.md` and
  `learnings.md` and update `todo.md` and the Inferred labels in
  `docs/agents/app.md`. The step's checkbox was ticked on the automated
  criteria only. Basis: the plan's Step 1; files `crates/app/src/main.rs`,
  `crates/app/src/modal.rs`, `docs/agents/app.md`, `todo.md`.

## Wrap-up real-device check

- The check passed on Windows 11 under both the trash dialog and the
  `Delete Sidecars…` dialog: minimize, maximize, the bar drag and close all
  worked, the bar undimmed, and the dialog kept its focus (Tab still moved
  the outline, Enter on `Cancel` and Escape still closed it). Results in
  `results.md`.
- Real-pointer `mouse_event` on the window controls (`SetCursorPos`, three
  relative moves, press, release) reached the non-client hit test every
  time; the drag worked with 40 relative moves 20 ms apart between press and
  release. Posted client clicks were used only for the menus and the
  presses below the bar.
- To test Enter without confirming a destructive run: Tab to `Cancel`,
  then Enter. The dialog opens with the destructive button focused.
- Holding the `.ui-lock` across several tool calls worked through a separate
  hidden PowerShell process that opens it with `FileShare.None` and waits for
  a release file; each tool call is its own process, so a handle opened in
  one call would close at its end.
- `select_photos` wants the open folder's spelling of a path, here the
  verbatim `\?\D:\...` form `get_view` reports; plain `D:\...` was refused
  as not shown.
- Comparing two MCP screenshots (with and without the dialog) is the
  cleanest "undimmed" test: zero differing pixels in the bar's rows. A
  desktop capture adds the window's 1 px DWM top border, which differs.
- The cold debug build with `-j 2` took about 30 minutes and was not killed
  for memory.
