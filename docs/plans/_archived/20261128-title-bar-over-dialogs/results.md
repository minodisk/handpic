# Results: Windows real-device check of the title bar under a dialog (2026-11-28)

The plan's Step 1 "Manual check", run on the Windows 11 host by an agent
session through the MCP companion, the `screenshot` tool, posted keys and
clicks, and real-pointer `mouse_event` input on the title bar.

## Setup

- Machine: Windows 11 Home 10.0.26200, one 5120 x 1440 monitor at 100 %
  (96 dpi); window 1600 x 1000 client, Neutral dark.
- Build: `crates/app` at `b34463f3` (branch `fix/title-bar-over-dialogs`),
  debug, `cargo build --features screenshot -j 2` (what
  `mise run app:dev:screenshot` runs), 30 min 18 s from cold.
- Launch: `target/debug/handpic.exe` with `--index`, `--settings` and `--log`
  under `D:\handpic-titlebar-checks\run\a\`, `--mcp-port 47880`, and
  `--import-settings` of `{"mcpEnabled": true, "sidecarFormat": "both"}`
  (`mcpAllowDestructive` off, so each operation opened its dialog).
- Test folder: `D:\handpic-titlebar-checks\photos\`, the 45 files of
  `D:\Photos\tests\20261127-c5\step7\photos` (ARWs hardlinked, its seven
  `.xmp` copied; its `sub\` folder left out).
- Input: all synthesized input under an exclusive lock on
  `D:\Photos\tests\.ui-lock`, held by a separate PowerShell process for the
  whole session. Right-clicks, menu clicks, clicks below the bar and keys
  (Tab, Enter, Escape) posted to the window; the title bar's minimize,
  maximize, close and drag pressed with real-pointer `mouse_event` at screen
  coordinates (`SetCursorPos`, three relative moves, press), each point
  first checked with `WindowFromPoint` to belong to this run's PID and a
  desktop capture showing nothing over the window. State read with
  `IsIconic`, `IsZoomed` and `GetWindowRect`; minimize and maximize undone
  with `ShowWindow(SW_RESTORE)`.

## Bar undimmed, the rest dimmed: PASS

With the trash dialog and with the `Delete Sidecars…` dialog open, the
captures show the bar (app icon, `Handpic`, the folder name, gear and window
controls) at full brightness and the panes, strip and status row dimmed. In
the MCP screenshots the bar's 34 rows are pixel-identical between no dialog
and the trash dialog open, while below them the tree went `#171717` →
`#090909` and the photo `#875d61` → `#362527`.

Not done: other themes.

## Trash dialog (`Move 3 Files to Trash…`): PASS

- Three files selected through `select_photos`, the dialog opened from the
  strip's right-click menu (`get_status`: `Move to Trash`,
  `awaiting_confirmation`).
- Minimize: `IsIconic` true; restored. Maximize: `IsZoomed` true (rect
  54,-8,5128,1448); restored. Drag on the bar's empty area: the window moved
  from 1783,216 to 1604,288, and a second drag moved it back.
- After those presses and two clicks on the dimmed area below the bar, the
  dialog was still `awaiting_confirmation`, the outline still on
  `Move to Trash`. Tab moved the outline to `Cancel`; Enter on it closed the
  dialog (`operation` null) with the three files still on disk.
- Reopened: a drag and a minimize / restore, then Escape closed it, the
  files still on disk.

Not done: Enter on `Move to Trash` (it would move the files), and
`Delete Permanently…`.

## `Delete Sidecars…` dialog: PASS

- Opened from the tree's right-click on `photos` (`7 XMP sidecars (2.5 KB)`).
- Minimize: `IsIconic` true; maximize: `IsZoomed` true; each restored. Drag:
  1784,217 → 1605,289 and back. Two clicks below the bar: still
  `awaiting_confirmation`.
- Tab moved the outline to `Cancel`; Enter on it closed the dialog, the seven
  `.xmp` still there. Reopened, maximize / restore, then Escape closed it.

Not done: Enter on `Move to Trash`.

## Close under an asking dialog: PASS

Reopened `Delete Sidecars…` (`awaiting_confirmation`), a real click on the
bar's close button: the window and the process (PID 34096) were gone within
4 s, the log ending in `quit: MCP server stopped=true`, no panic or
invariant line, the seven `.xmp` untouched.

Not done: close under the trash dialog (one close per launch), and the cache
confirmation dialog.
