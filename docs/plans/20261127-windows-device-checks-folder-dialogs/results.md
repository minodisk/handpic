# Results: Windows real-device checks of the status row, the folder drop and the folder dialogs (2026-11-27)

Checks of `todo.md` items from "App: real-device checks of the status row's
StatusBar and progress circle" up to (not including) "App: real-device checks
of the tree's inline folder rename (gpui-folder-dialogs Step 7)", run on the
Windows 11 host by an agent session through the MCP companion, the
`screenshot` tool and posted input.

## Setup

- Build: `crates/app` at `f5cab4bf` (main, 2.2.0), debug, `--features screenshot`.
- Display: one 5120 x 1440 monitor at 100 % (96 dpi); window 1600 x 1000 client, Neutral dark.
- Material: copies (and, for the read-only sets, hardlinks) under `D:\handpic-c4-checks\`, outside the photo folders:
  `canvas\` (125 ARWs of `20261123-mcp-visual\big`, shot 2026-08-22 and 2026-09-19),
  `sidecars\` (the twelve files of `20261123-mcp-visual\marks`), `jpeg-only\`,
  `sequence\` (twelve JPEGs, six sharing a second), `seq500\` (the 500 JPEGs of `jpeg500`),
  `import-src\` (four ARWs of 2026-10-17 and 2026-09-19, two `.xmp`, one `.dop`,
  two JPEGs dated to those days, `undated.jpg` stripped of Exif), and `step6\`
  (`one-date\` three ARWs of 2026-10-17 with two `.xmp`, `three-dates\`,
  `two-dates\`, `collide\` one ARW of 2024-11-19 next to an empty `2024-11-19\`).
- Each launch: a fresh `--index`, `--settings`, `--log` under `run\<name>\`,
  `--mcp-port 47814`, and `{"mcpEnabled": true, "sidecarFormat": "both",
  "importRoot": "D:/handpic-c4-checks/import-dst"}` through `--import-settings`
  (`mcpAllowDestructive` off, so every folder operation opened its dialog).
- Input: keys posted to the window as `WM_KEYDOWN` / `WM_KEYUP` with the
  modifier state set through `AttachThreadInput` + `SetKeyboardState` (the
  method `docs/agents/app.md` records for `tab`); right and left clicks
  posted as `WM_RBUTTONDOWN` / `WM_LBUTTONDOWN` at client coordinates. No
  real keyboard, no real mouse except the drop attempt below.
- Canvas measured on 1600 x 1000 screenshots by the runs of preview-surround
  pixels; "shown" and "gone" compared pixel for pixel.

## Seen (PASS)

### StatusBar: the canvas in the other pane layouts (box 1)

Each layout on a fresh index of `canvas\`, captured during pass 2
(`analyzing n/125`, the circle drawn at the status row's right end) and again
once idle (no circle):

| Layout | Canvas (x, y, size) | Pixels differing between shown and gone |
| --- | --- | --- |
| right pane hidden | x 221-1599, y 34-800, 1379 x 767 | only y 922-992 (the strip's cells and the status row) |
| both side panes hidden (`set_panes` left and right off) | x 0-1599, y 34-800, 1600 x 767 | only y 922-992 |
| strip alone hidden | x 221-1378, y 34-969, 1158 x 936 | only y 979-992 (the status row) |

The canvas did not resize when the circle appeared or went in any layout.

### Rewrite Sidecars from Index… / Delete Sidecars… (Step 3)

- Rewrite on `sidecars\` (4 judged, 4 skipped): the dialog opened with
  `Rewrite` outlined; Tab moved the outline to `Cancel` and a second Tab back
  to `Rewrite` (the frame pixel-identical to the first); Escape closed it.
- Rewrite on `canvas\` with 125 files judged (1 star): Enter on the focused
  `Rewrite` ran it; the captured frame shows `Rewrite` and `Cancel` dimmed,
  the note `Writing the sidecars; this closes once every file is written.`
  and the status row's progress circle; `get_status` went `running` 0 / 0,
  64 / 125, then `Rewrote the sidecars of 125 files in canvas`.
- Delete on `sidecars\` (4 `.xmp`, 3 `.dop`): Enter on `Move to Trash` moved
  the 7 sidecars (`Moved 7 sidecars to the Trash`), the strip's stars, flag
  dots and labels went; a posted `Ctrl+Z` restored them (`Restored 7
  sidecars from the Trash`), the files back on disk and the marks back in the
  strip.
- `Ctrl+,` posted while an Import preview of 500 JPEGs was reading (pane note
  `reading D:\Photos\tests\2026-09-28-gui-check-jpeg-500…`, `get_status`
  `previewing` before and after) did not open the settings. (A first try
  against the Delete preview of `canvas\` ended its preview before the key
  arrived; with nothing under way the same posted key did open the settings,
  so the key itself reached the view.)

### Sequence JPEG Timestamps… (Step 4)

- On `sequence\`: Tab moved the outline from `Run` to `Cancel`, a second Tab
  back (identical frame); `Ctrl+,` with the dialog up changed nothing (frame
  identical); Escape closed the dialog.
- On `seq500\`: the item picked from the tree's right-click menu started the
  preview; the menu opened again and the item picked a second time during
  `previewing` did nothing (no second preview, no error, one dialog:
  `500 files will be written to seq500-sequenced`, `4 of them get a new
  capture time`).
- Enter ran it; `Ctrl+,` posted during the run (`sequencing 21 / 500`,
  `Run` dimmed, `Cancel` focused) did not open the settings; Escape ended
  it with `canceled, 71 of 500 written`.
- Through MCP, a second `sequence_timestamps` during `previewing` was refused
  with `a folder operation is under way in Handpic; try again once it ends`.

### Import… (Step 5)

- `Import…` from the menu on `import-src\`: the dialog listed `9 files
  (206.7 MB) to copy into 2 folders`, the two date folders and
  `undated.jpg` under `Without a capture date, not copied:`; Tab moved
  `Run` -> `Cancel` -> `Run`; Escape closed it. The `reading …` note before
  the dialog was seen on the 500-JPEG preview above (the 9-file preview ended
  before a capture).
- `Run` on `canvas\` (375 files, 6.31 GB, not in the file cache: 187 MB/s,
  about 34 s): the dialog stepped aside, `2026-08-22` opened in the tree and
  the title bar, and the pane counted `importing 3 / 375` ... `importing 45 /
  375`.
- `Import…` picked again mid-run (`importing 255 / 375`): the dialog came
  back with `Run` dimmed and `Cancel` focused and the note `Cancel stops at
  once and drops the file in flight; …`; Escape hid it and the run went on
  (285, then the end: `Imported 375 files (6.31 GB)`).
- A second run into `import-dst2\`: `Import…` picked again at about 39 / 375
  and Enter on the focused `Cancel` stopped it with `Import canceled, 78
  files copied`; 78 files on disk, no `.handpic-tmp` file left.

### Rename by Capture Date… (Step 6)

- On `one-date\`: the dialog (`Rename one-date to 2026-10-17?`, `2026-10-17
  3 files`) had `Run` outlined; Tab -> `Cancel`, Tab -> `Run` (identical
  frame); Escape closed it. Opened again and Enter: the captured frame shows
  `Run` and `Cancel` dimmed with `Moving the files; this closes once every
  file is moved.`; it ended `renamed to D:\handpic-c4-checks\step6\2026-10-17`
  with the folder reopened under its new name and the strip's two stars kept.
- On `collide\` next to an empty `2024-11-19\`: `Run` disabled, the reason
  `2024-11-19 already exists next to collide; nothing is merged.` and the
  outline on `Cancel`.

## Not as expected

- Import, box 3: the strip did not fill during the run. At `importing 45 /
  375` the open `2026-08-22` showed `This folder has no RAW or JPEG files.`
  and the status line `no RAW files in …` while 51 ARWs were already on disk.
  The log shows the folder listed with `files=0` at 14:23:42 and the next
  `resync` only at 14:24:10 with `files=100`, after that date folder's copies
  had all landed. Cause: the open folder's watcher (`crates/engine/src/watch.rs`)
  reports a burst `DEBOUNCE` (500 ms) after its last event, and a copy that
  never pauses that long never settles. Delegated as a fix (a maximum wait on
  the debounce) to the session `dlg-c4-import-watch`.

## Not run

- Folder drop (gpui-folder-drop): no real OLE drag reached the window. An
  `IDropSource` of the agent's own passed to `DoDragDrop` failed with
  `E_FAIL` (0x80004005) with no button down, and a WinForms `DoDragDrop`
  started from an injected left press with the cursor moved onto the window
  (`WindowFromPoint` named the Handpic window) drew no outline and returned
  `None`; one attempt left the left button down until released by hand. The
  drop checks need a hand drag from Explorer.
- Filter menu icons on a HiDPI or fractional-scale display: the host has one
  display at 100 %.
- The macOS and Linux boxes (status row, bursts): not this host.
