# Learnings: windows-device-checks-c5

## Step 1: record the 2026-11-27 run

- Setup worth keeping for the next run: each launch with its own `--index`,
  `--settings` and `--log`, a fixed `--mcp-port`, and
  `{"mcpEnabled": true}` (plus `"mcpAllowDestructive": true` where a run must
  be confirmed) through `--import-settings`; the material as hardlinked ARWs
  (cheap, so many fresh 264-file folders for the held rename were at hand);
  and a preview made to fail after its scan by zeroing the JPEG `FF D8 FF`
  markers with the file's size and mtime kept, so the index rows stay and
  only the preview read fails.
- Synthetic input (`mouse_event` / `keybd_event` after a few relative moves,
  the window made foreground) added what MCP alone cannot close: right-clicks,
  thumb and window-edge drags, and keys inside the settings (Tab, Space,
  arrows in the language `Select`, `+` captures). Still the user's: the
  Japanese IME, a HiDPI display, a drop from Explorer and the look (colors
  were judged only as "visible").
- A Python `subprocess` capture of a launcher that starts the app with
  redirected std handles hung until the app exited: the app inherits the
  handles, so the pipe never closes. Writing the launcher's output to files
  and reading them avoids it. This fact is not in `results.md`; the
  `docs/agents/app.md` rule cites this file for it.
- Two failures were handed to their own sessions: the title bar's controls
  and drag dead under the trash and folder operation dialogs
  (`dlg-titlebar-dialog`), and the no-folder message overflowing a 240 px
  preview (`dlg-empty-wrap`). Each is a failed box in its `todo.md` item.
- While applying the edits: `main` was at the planned base (`0b9d3300`), so
  the rebase was a no-op. The item the plan called
  `### App: the strip's stars have a dark outer outline and its face focus icons do not`
  is on `main` as
  `### App: real-device check of the strip marks' one dark outline (strip-marks-outline)`
  (reworded by another session); it was left untouched either way. Every
  other anchor of lists B and C was found as written.
- The `todo.md` edits were applied with a script that locates each item by
  its exact `###` line and each box by its opening words, asserting one match
  each, so a moved or reworded anchor would have failed loudly rather than
  edit the wrong place.

## Deferred issues (todo candidates)

- `docs/agents/app.md`'s keymap section ("The keymap is edited in the modal
  and replaced whole") says a refused capture key shows its refusal on the
  status line, but on Windows 11 on 2026-11-27 `+` then `home` / `end`
  showed `"home" is taken by this app` in red in the settings body, the
  full-window settings layer covering the status line (results item 14 in
  this folder's `results.md`). Basis: the plan's trade-off note on this run;
  left as is in this step. Re-check which line it is and correct the rule.
  Files: `docs/agents/app.md`, `crates/app/src/settingsview.rs`.
