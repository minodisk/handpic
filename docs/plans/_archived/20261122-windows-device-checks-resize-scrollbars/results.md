# Windows real-device checks, batch 8 (2026-11-22): drag-resizable panes, strip height and scrollbars

## Setup

- Windows 11, one 5120 x 1440 display at 96 DPI (`GetDpiForWindow` 96, 100 % scale). No HiDPI display was at hand.
- Debug build of `crates/app` with `--features screenshot` (`cargo build --features screenshot`), Neutral dark theme.
- Material under `D:\Photos\tests\20261122-calm-meadow\`: `long\` (299 hardlinks to `D:\Photos\samples\ARW`, 295 listed), `short\` (3 hardlinks), `tree\` (60 subfolders of one hardlink each; renamed `tree-r` by the rename check). Nothing else under `D:\Photos` was opened.
- Every launch: `--index`, `--settings` and `--log` under the test folder (`run1\` for the release and restart cycle, which reuses them by design; `run2\` fresh for the empty-state check), `--import-settings` with `{"mcpEnabled": true}`, `--mcp-port 41961`. `mcpAllowDestructive` stayed off.
- Input: synthetic `SetCursorPos` / `mouse_event` / `keybd_event`, each block after checking `GetForegroundWindow` is the instance's window (by PID). Evidence: client-area captures, pixel runs along rows and columns, `GetCursorInfo` against `LoadCursor(IDC_*)`, MCP `get_view` / `get_status` / `set_filter` / `set_judgment` / `open_folder`, and the `settings` table of the test store read with Python's `sqlite3` in read-only mode.
- The default window's client area is 1600 x 1000: tree pane x 0-219, its 1 px boundary line at x 220, the meta pane's line at x 1379 (preview 221-1378), the strip pane's top line at y 801, the status row's border at y 970.
- Every instance was quit with `q` (the quit handlers ran; the store read after each quit matched the one read before it).

## Incidents

- The first cursor probe (pointer moved away and back with `SetCursorPos`, 450 ms wait) read `ARROW` all along the pane boundaries and `SIZENS` on one row of the strip's band. A probe that ends with a few relative `mouse_event` moves before the read gave the bands below; the reads still shifted by about 1 px between runs.
- The first drag aimed at the meta pane's boundary pressed at x 1380 (1 px right of its line) and moved nothing: that handle's band lies on the preview side of the line (x about 1374-1379). Pressing at x 1376 worked.
- `--auto` takes a value (`--auto N`, N automatic pages, then quit); a bare `--auto` exits with the usage error. The `--auto` check used `--auto 40` on `long\`.

## `### App: real-device checks of the drag-resizable tree and meta panes`

### Box 1: hover the preview's left edge

- Cursor (`GetCursorInfo`): `SIZEWE` on x 221-226 at the tree's boundary (line at x 220) and on x about 1376-1380 at the meta pane's (line at x 1379), `ARROW` either side. Read twice, the edges moved by 1 px. PASS.
- The pill: while hovered, a 3 x 20 px pill (`#474747` / `#494949` / `#383838`) shows centered on the boundary at y 407-426, the preview's vertical middle (34-801); it is not drawn at rest. Seen on both boundaries. PASS.
- The 1 px line is `#1a1a1a` between the tree pane's `#171717` and the preview's black surround: it reads as the preview's edge against the black, but is barely different from the pane's own fill (3 levels). Judged acceptable in Neutral dark; no other theme was looked at.

### Box 2: a jump at the first move

- Pressed at x 223 on the tree's boundary (line at 220), the line did not move for pointer moves to 224, 225 and 227, then sat under the pointer: 230 at a pointer of 230, 240 at 240. Released at 223, the line stayed at 223.
- Pressed at x 1379 on the meta pane's (line at 1379): no move at 1378, 1377, 1375, then line 1369 at a pointer of 1370, 1359 at 1360; released at 1379, the line was at 1378.
- So the boundary does not follow the first 3-5 px of a drag, then jumps to the pointer: the press's offset from the line inside the band (here up to 3 px) is lost. FINDING (minor): a drag pressed off the line's own column moves the boundary by that offset.

### Box 3: release positions and restarts

Each case released the drag, read the store before any other input, quit with `q`, read the store again, relaunched on the same `--settings` and measured the boundary in a capture.

| Release | Store right after release | After quit | After relaunch |
| --- | --- | --- | --- |
| Over the preview (tree's edge to x 400) | `left` 400 | 400 | line at 400 |
| Over a pane (tree's edge to x 1450, over the meta pane) | `left` 480 (clamped) | 480 | line at 480 |
| Over the strip (tree's edge to (300, 880)) | `left` 300 | 300 | line at 300 |
| Outside the window, left (tree's edge to client x -150) | `left` 160 (clamped) | 160 | line at 160 |
| Outside the window, right (meta pane's edge to client x 1700) | `right` 160 (clamped) | 160 | line at 1439 |

- A release outside the window wrote the width at once: the store held it before any click inside, and the window kept the foreground. PASS for every case.

### Box 4: `z`, then drag a boundary

- With the tree at 400, `z` on `DSC-HX95_DSC00018.ARW`, then the tree's boundary dragged to 162 (the canvas grows). During the drag and right after the release the held crop sat centered with black bands either side; about 3 s later the crop filled the wider canvas edge to edge. PASS (judged from captures; a re-cut prints no `zoom` line).

### Box 5: the 5 px bands and the layout at 160 px

- Empty-state message: on a fresh store with no folder, both panes dragged to 480 (preview x 481-1118), the message's text spans x 561-1035. A click 3 px inside its left end opened the folder picker (`#32770` `フォルダーの選択` of the instance's PID), closed afterwards with `WM_CLOSE`; the status line stayed `no folder`. PASS, though the message stayed 80 px from each band, so the bands could not reach it in this window.
- Preview context menu: there is none. A right press on the preview at x 170 (5 px right of the tree's band at width 160) and on the meta pane opened nothing; `context.rs`'s `Target` has only `Folder` and `Files`. Not applicable.
- Panes' scrollbars: at a tree width of 160, the tree's thumb (x about 152-157) sits 4 px left of the band (x 161-165). Dragging it at x 155 scrolled the tree and left the width at 162 (unchanged). The meta pane's thumb is at the pane's right edge, away from its band. PASS.
- Layout at 160 px (both panes):
  - Tree rows truncate with an ellipsis (`2026110...`), the open folder's row shows `l...` and the `295` pill whole; the inline rename input opened on `short` fits between the folder icon and the scrollbar track (x about 98-145). PASS.
  - Meta pane rows: values wrap, and long words break mid-word: `Electroni` / `c`, `Standar` / `d`, `Compresse` / `d RAW`, `Mechani` / `cal`, `Manu` / `al`, and the file name `ILCE-1M2_sony_a1_ii_0` / `3.arw`. FINDING: at 160 px the meta pane breaks words inside, not at a space or hyphen.

## `### App: real-device checks of the drag-resizable strip height (native-strip-resize)`

### Box 1: hover the strip's top edge

- Cursor: `SIZENS` on y 802-807 (line at y 801), `ARROW` at 800-801 and 808-809, matching batch 7's y 802-806 within the 1 px read shift. PASS.
- The line is `#1a1a1a` at y 801 between the preview's black surround and the strip header's black: it reads as a thin line on black on both sides. PASS (Neutral dark).
- Clicks next to it still worked: a click on a cell made it current (position 84 -> 83), and the scrollbar's thumb drag and track clicks scrolled the strip (scrollbar item, box 1 and 2). PASS.

### Box 2: sharpness at 302 px

- At 302 px (96 DPI) a cell's image box is 405 x 270 and the cached thumbnails' long edge is 404 px (`JPEG_THUMBNAIL_LONG_EDGE`; ARW thumbnails at about 404 x 270), so they are drawn at about 1:1. A capture of `DSC-RX0M2_RX000224.ARW`'s cell was legible (the color checker's patches and the box's lettering) with the softness of the thumbnail's own JPEG, no visible upscaling blur. PASS at 100 %.
- HiDPI: not run (no HiDPI display).

### Box 3: release positions and restarts

| Release | Store right after release | After quit | After relaunch |
| --- | --- | --- | --- |
| Over the preview (edge to y 600) | `stripHeight` 302 (`MAX`) | 302 | line at y 627 |
| Over a pane (edge to (100, 750), over the tree) | 179 | 179 | line at y 750 |
| Over the status row (edge to y 985) | 96 (`MIN`) | 96 | line at y 833 |
| Outside the window, above (edge to client y -100) | 302 (`MAX`) | 302 | line at y 627 |

- The release outside the window wrote the height at once (stored before any click inside). PASS for every case.

### Box 4: 1:1 and `toggleStrip`

- `z`, then a strip drag: not re-run; batch 7 saw a 1:1 crop re-cut after a strip edge drag on 2026-11-22 (`docs/plans/_archived/20261122-windows-device-checks-7/results.md`, the scrollbars' box 7), and box 4 of the pane item above saw the same re-cut after a pane drag.
- `toggleStrip` (`ctrl+alt+down`) at a dragged 302 px: the strip, its header and its top line went (no `1a1a1a` row; the preview ran to the status row), and no `SIZENS` cursor showed on y 960-975; the second press brought the strip back with its line at y 627, the same 302 px. PASS.

### Box 5: `--auto`

- Stored `stripHeight` 199 and `panelWidths` 162 / 160. Launched with `--auto 40` on `long\`: the strip's line at y 801 (128 px) and the panes at 220 (lines at 220 / 1379). A drag of the strip's edge to y 650 moved the line there; the store still read `stripHeight` 199 right after the drag and after the run quit on its own (about 36 s). PASS.

## `### App: real-device checks of the strip, folder tree and meta pane scrollbars`

The bars are drawn at rest since `native-scrollbars-always` (`ScrollbarMode::Always`), so this item's fade-out no longer applies.

### Box 1: the strip's thumb, arrow keys, a refilter and a window resize

- On `long\` (295 files) at the default height the thumb spans x 5-52 at y 960-965. Dragging it to the right end put it at x 1548-1595, and within 3 s the last cells (`ILCE-6700_...`) showed thumbnails. The current file stayed position 1. PASS.
- Arrow keys and the app's keys, with MCP `get_view` after each: `→` -> position 2, thumb x 9-57; 40 more `→` -> 42, thumb 175-223, the current cell at the right end of the strip; `End` -> 295, thumb 1548-1595; 20 `←` -> 275, thumb 1490-1537, the current cell at the left end; `Home` -> 1, thumb 4-51. PASS.
- Refilter: at position 61 (thumb 278-326), `set_filter` `orientations: portrait` left 70 files with the current one at 4 and the thumb grew to x 73-301, the current cell in view; `reset` gave back 295 files, current at 91, thumb 492-539, the current cell in view. PASS.
- Window resize: the window's right edge dragged 500 px in (client 1100 wide): the thumb became x 333-372 (40 px against 48), the current cell still in view. PASS.

### Box 2: a press on the shown bar

- With position 91 current and one file selected (`set_judgment` gave it the `Red` label), a press on the thumb, a track click at x 800 (the view jumped) and one at x 20 (back to the start) each left position 91 and one selected file. PASS.
- Hover over the red cell's name band showed the `Red` tooltip before the presses and again after them (the thumb dragged back to bring the cell into view). PASS.

### Box 3: the tree next to its bar

- With the tree overflowing (its thumb at x about 152-157, width 160): a click on `short` at x 145 opened it (`get_view`: `short`, 3 files); a right press on `short` at x 110 and on `tree` at x 140 opened the folder menu; `Rename…` on `tree` opened the inline input, typing `tree-r` and Enter renamed the folder on disk and in the tree; on `short`, `escape` closed the input with the name kept. PASS.

### Box 4: the settings modal's lists and the menus

- Settings modal (`ctrl+,`): the Sidecar page's `Lightroom language` `Select` list scrolled by wheel from `English`... to ...`繁體中文` with its thumb moving; `escape` closed the list and kept the modal. The Keyboard Shortcuts page scrolled by wheel from `Filmstrip` to `Judgment`'s end with its thumb moving. PASS.
- `Filter ▾` (long, on `long\`): the wheel scrolled it from `Reset` down to the `Camera` section, thumb moving. `Sort ▾` holds three rows and does not overflow. PASS.
- Strip right-click menu: in the 1000 px window it fits (no bar; the wheel did nothing). In a 550 px window (the bottom edge dragged up) it opened with a bar, already scrolled so its top row was `Unflag` (the checked flag row); the wheel scrolled it up to `Select All` and down to `Delete Permanently…`. PASS. The window was dragged back to 1000 px.
- Not done: dragging the thumbs of these lists and menus (wheel only).

## Not run

- HiDPI (strip box 2).
- macOS and Linux boxes of all three items.
