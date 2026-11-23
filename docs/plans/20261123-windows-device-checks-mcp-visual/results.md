# Results: Windows real-device checks through MCP and screenshots (2026-11-23)

Checks of `todo.md` items driven through the MCP companion (`open_folder`,
`navigate`, `set_overlays`, `set_judgment`, `set_filter`, `set_settings`,
the folder-operation tools, `undo`, `wait_for_idle`, `get_status`) and the
`screenshot` tool, with a few synthetic left clicks (no key presses, no IME,
no drag, no right-click, no HiDPI). Visual judgments were made by the user
from the running window on 2026-11-23.

## Setup

- Build: `crates/app` at `3918064c` (main), debug, `--features screenshot`.
- Material: copies and hardlinks under `D:\Photos\tests\20261123-mcp-visual\`:
  `marks\` (the eight files of `20261110-gpui-pass2\marks`), `longnames\`
  (`20261113-filter-sort\sort\raw`), `big\` (hardlinks of
  `2026-09-28-gui-check`, 125 ARWs plus sidecars), `jpeg500\` (hardlinks of
  `2026-09-28-gui-check-jpeg-500`), `onedate\` (`_DSC1616`, `_DSC1758`,
  `_DSC1760`, all shot 2026-10-17), `raf\`.
- Each launch had a fresh `--index`, `--settings`, `--log` under `run\<name>\`,
  `--mcp-port 41973`, and `{"mcpEnabled": true}` (plus
  `"mcpAllowDestructive": true` for r4) through `--import-settings`. Window
  1600 x 1000 client, Neutral dark unless stated.
- Synthetic left clicks: `mouse_event` after a few relative moves, the window
  made foreground (an Alt tap before `SetForegroundWindow`), under
  `D:\Photos\tests\.ui-lock`.
- MCP client gotcha: Git Bash mangles `\\` in JSON arguments even through
  stdin here-strings; the client converted `D:/...` paths to backslashes.

## Seen (PASS)

1. Sort / Filter with no folder (todo-three-decisions, box 1): with no folder
   both buttons are dimmed and a left click on either opens no menu; with a
   folder open `Filter ▾` opens on a click; in the JPEG-only `jpeg500` the
   Sort button is dimmed.
2. Reserved slot (todo-three-decisions, box 2): `1 / 8` stayed at the same x
   with the progress circle shown (pass 2) and gone (idle).
3. Middle ellipsis (todo-three-decisions, box 3, default strip height only):
   `ILCE-6300_Free_ra…3068.arw`, `L1001880-DxO_D…ME 3.jpg` fit inside their
   cells; short names centered and unchanged. Not run: after dragging the
   strip shorter and taller.
4. Title bar folder name (native-title-bar-folder-name): no name with no
   folder; `marks` centered after an open; the name followed another folder's
   open, `Rename by Capture Date…` (`onedate` -> `2026-10-17`) and a folder
   rename (`rename_folder` to a 72-character name); at a 784 px window the long
   name still fitted, at 604 px it ended in `…` before the gear with room to
   spare left of it. Not judged: "Handpic" at 14 px by eye.
5. Meta pane Analysis (native-meta-analysis, box 1): on `big` (fresh index)
   the shown file's `Analysis` group appeared at `analyzing 27/125` with no
   input; `Eye focus` `Sharp · 0.281`, `Eyes` `Open · 76`, `Eyes closed`,
   `Eye aspect ratio`, `Yaw` / `Pitch` / `Roll`, `Mesh eye offset`, `Edge gap`,
   `Sharpness` rows; `AF point` (`5913, 2964 of 8640 × 5760`) and `AF frame`
   directly after `AF tracking` in `Maker note`, neither in `Analysis`. In
   the 8-file `marks` folder the group (and the strip's face icons) appeared
   only when pass 2 ended: pass 2 commits rows in batches of 10 (`BATCH`,
   `crates/engine/src/index.rs`), so a folder under 10 files reports every
   file at once. Expected behaviour.
6. Meta pane (box 2, partly): a JPEG (`jpeg500`) shows no `Analysis` group;
   with a filter hiding every file (`stars: [2]`) the pane shows only
   `No files match the current filter.`; the `Analysis` rows were the same
   with `f` on and off. Not run: a failed preview.
7. f overlay (gpui-focus-marks, boxes 1-3, judged by the user as OK; no Tauri
   reference build was at hand, so not side by side): AF crosshair and frame
   bright green on `_DSC1758`, gray on `_DSC1616`, white on the rest, no AF
   mark on `ILCE-6400_DSC00405`; nine face boxes on `_DSC1758` with cyan dots
   and labels; the magenta mesh on `DSC03068` (closed eyes, no iris rings),
   `DSC00405`, `DSC01340` (profile, orientation 6) and `_DSC1758`. Labels of
   small adjacent faces overlap on `_DSC1616` / `_DSC1758`; the user accepted
   that. Not run: holding `→` with `f` on and the `eyes` log lines.
8. Pass 2 status (gpui-focus-marks box 4, partly): `scanning N/8` then
   `analyzing N/8`, ending with an empty status line. Not run: a folder switch
   from the tree mid-pass 2.
9. Strip face icon (native-strip-face-icon-tooltips box 1, partly): icons
   appeared on cells during pass 2 with no input, none on neutral frames or
   JPEGs. Not run: 150 % scale.
10. Catalog icons (native-catalog-icons, Windows box): the filter menu's star
    rows (filled and outlined), the orientation rectangles and the `Face focus`
    icon, and the strip's good / low face icons, judged by the user as OK in
    Neutral dark.
11. Filter menu headings (gpui-filter-menu-icons box 1): hovering `Flag` drew
    no highlight; a click on `Rating` changed nothing and the menu stayed
    open (`get_view` filter unchanged).
12. Rewrite / Delete Sidecars held for a scan (gpui-folder-dialogs Step 3,
    box 2): `rewrite_sidecars` on `marks` during pass 1 of a fresh index noted
    `Rewrite Sidecars from Index: waiting for the scan to finish`, then the
    dialog opened once pass 2 ended; same for `delete_sidecars` on `longnames`
    (`Move 3 sidecars (844 B) of longnames to the Trash?`).
13. Delete Sidecars and undo (Step 3, box 4, through MCP): with
    `mcpAllowDestructive`, the confirmed run moved 4 `.xmp` to the Trash and
    the strip's stars, flags and labels went; MCP `undo` restored the files
    and the marks. Not run: `Ctrl+Z` itself (a key press).
14. Sequence progress (Step 4, box 3): a confirmed run on 500 JPEGs showed
    `sequencing 109 / 500` in the pane's notes and ended `Wrote 500 of 500
    files to …\jpeg500-sequenced`.
15. Rename by Capture Date (Step 6, box 2, partly): with two files judged
    (4 stars; pick + Green), the confirmed run renamed `onedate` to
    `2026-10-17`, reopened it, and the strip kept both judgments and the
    current file. Not run: Tab / Escape in the dialog, the dimmed buttons.
16. Appearance (native-themes, Windows boxes, partly): Neutral Light,
    Catppuccin Latte, Tokyo Night and Neutral dark set through `set_settings`
    applied at once to the panes, strip, status row, title bar and menus.
    Not run: System following the OS, restart, the pickers' keys.

## New findings (user, 2026-11-23)

A. Light themes: a rejected strip cell is dimmed so faintly that it melts into
   the light background (seen in Neutral Light and Catppuccin Latte).
B. Light themes: the `Filter ▾` menu's `Face focus` `Good` icon green is too
   bright to read on the light surface.
C. The strip's stars are drawn with a dark outer outline while the
   face focus icons (Good / Low) are not; the user wants the strip's icon
   drawing unified.
D. On `jpeg500` (500 JPEGs, hardlinks of cached files, fresh index) the strip
   cells in view at positions 3-9 stayed gray placeholders until pass 1 ended
   (`scan extract … in 89963ms`), while cells 1, 2, 10 and 11 had thumbnails
   early (seen at `scanning 492/500` and `498/500`). The hot list should have
   taken the cells in view first.
E. Reproduced `### App: Rewrite Sidecars from Index reported "4 files not in
   the index (skipped)" in a fully scanned folder`: on `marks` after both
   passes, the preview read `4 files with a judgment` and `4 files not in the
   index (skipped)` (the other four are indexed, unjudged).
