# Windows real-device checks, batch 7 (2026-11-22), empty state, status row, scrollbars and the strip header

Setup: Windows 11, debug build of `crates/app` with `--features screenshot` (origin/main `86bb0180`), one 5120 x 1440 display at scale 100 % (the default window 1616 x 1008 outside, a 1600 x 1000 client). Synthetic mouse and keys (`keybd_event`, `mouse_event`), window captures, burst captures of a screen region (one every 80 to 100 ms, stacked with the ms offset), the MCP `screenshot`, and `GetCursorInfo` for the cursor shape. MCP on port 41951 seeded with `{"mcpEnabled": true}` through `--import-settings`, `mcpAllowDestructive` off; a fresh `--index` / `--settings` / `--log` per run. The `page` / `zoom` lines were read from the redirected stdout. Material under `D:\Photos\tests\20261122-batch7\`:

- `cold-a\`, `cold-b\`: 600 ARWs each, `robocopy /J` copies of six `2026-*-focus-sample` folders, flattened with a date prefix by a same-volume rename.
- `many\`: 300 ARW hardlinks of three other `focus-sample` folders.
- `few\`: three ARW hardlinks.
- `nef\`: copies of `NIKON_D70_Nikon.nef` and two Z 30 NEFs.
- `mixed\`: 24 copies, four each of ARW, NEF, CR3, RAF, ORF and DNG from `D:\Photos\samples\`.
- `jpeg-only\`: four JPEGs.
- `deep\`: 40 folders, every third holding one ARW hardlink.
- `drop\`: two small text files.
- `longtext\`: `NIKON_D70_Nikon.nef` and 50 ARW hardlinks of `cold-a\`.
- `import-dst\`: the Import root.

Every instance was quit with `q` by its own PID.

Incidents:

- On `cold-a\` the window was not in front when pass 1 ran, so the first burst captured the terminal over it and the paging keys went nowhere. `cold-b\` was the spare. The second run launched with no folder, checked that the window was in front, then opened `cold-b\` over MCP `open_folder`. Every later input step first checked the foreground window by PID.
- A helper typo launched one instance with `--index` / `--settings` / `--log` under a stray `D:\Photos\tests\20261122-batch7$1\`. It was quit by its PID and the folder was deleted.
- A press on the tree's `C:\` root opened `C:\` in the test instance: a listing of no RAW file, no scan. It was the "presses reach the tree rows" check landing on a root row. `few\` was reopened right after.
- The settings modal opened on its Sidecar page, so a press meant for the Appearance page's `Dark` radio chose `Both (.xmp and .dop)`. It was set back to `Lightroom (.xmp)` at once in the same test store. No sidecar was written: `mixed\` held no `.xmp` or `.dop` afterwards.
- `SetWindowPos` on the app's window always gave a height of 1460 (the display's 1440 plus the frame) whatever height was asked, while width and position applied. Drags of the window's own edges resized it normally (269 to 1009 px tall), so short windows were made by dragging.
- The 300-file `Move to Trash…` moved the `many\` hardlinks to the Recycle Bin. The originals stay in their `focus-sample` folders.

## The empty state in the preview area

### Box 1: the key clause after a rebinding, and a click on the settings modal

- Launched with no folder and no `lastFolder`. In the Keyboard Shortcuts page, `+` on `Open folder` captured `space` (the row read `Ctrl+O × Space ×`), and `×` on `Ctrl+O` left `Space`. In the first capture 150 ms after `escape` closed the settings, the message read `Drop a folder onto the window, or click here to choose one. Or press Space.` PASS.
- `×` on the last key, `Space`, was refused. The settings showed `"space" is the only key for open; use Reset` at the bottom left, and the key stayed. An action always keeps a key, so the no-key clause cannot be reached from the UI. `Reset` then restored `Ctrl+O`. Settled as not reachable, not a failure.
- With the settings open, a press at the message's place (808, 418) opened no folder picker: the window stayed in front and the next press reached the settings sidebar. PASS.

### Box 2: the failed preview on a second visit

- On `nef\` (`NIKON_D70_Nikon.nef` first by name), paged right to `Z_30.NEF`, then back onto the D70 file: `The preview of this file could not be shown.` Paged right twice and back twice onto it: the same message again. PASS.

## The status row's StatusBar and progress circle

### Box 1: pass 1's circle, the drop to pass 2, paging during the scan

- `cold-b\` opened over MCP `open_folder` on a fresh index, with the status row burst-captured every 80 ms.
  - The listing took about 3.7 s.
  - `scanning 0/600` then rose by about 14 per frame to `scanning 600/600` in 4.3 s, and the circle filled steadily from empty to full.
  - On `analyzing 0/0` the circle drained over the next two frames (part full at +80 ms, near empty at +160 ms, empty at +240 ms), as the 180 ms value animation does. It then filled from empty through `analyzing 1/600`.
  - The position text `1 / 600` did not move while the circle came and changed (for the todo-three-decisions item, not edited here).
  - PASS. Judged: the drain reads as a reset rather than a jump.
- Paging during the scan was taken in pass 2 of `cold-a\`: 20 `→` key-downs 150 ms apart while `analyzing` ran, with the circle filling.
  - 17 `page` lines, `keypressToPixels` 11.9 to 29.0 ms (median about 19 ms), all `source=read`.
  - The same paging after the scan ended: 20 lines, 5.3 to 21.1 ms after a first 140 ms (median about 12.5 ms), all `source=prefetch`.
  - Judged not visibly slower, on a debug build. The difference follows the source (`read` during the scan, `prefetch` after), not the circle's frames.
  - Paging during pass 1 was not taken: the `→` keys sent during the listing were dropped, and pass 1 then lasted 4.3 s.

### Box 2: Move to Trash with the strip and the left pane hidden, and the canvas in each layout

- On `many\`, `ctrl+a`, `toggleStrip` and `toggleLeft`, then MCP `trash_photos` opened the app's dialog (`Move 300 files to the Trash?`). Enter confirmed it.
  - A burst of the canvas's bottom-right corner (the meta pane's border and the status row's top border) every 100 ms: the circle came with `moving to Trash 0 / 300`, filled through `100 / 300` and `200 / 300`, and went with `Moved 300 files to the Trash`.
  - The canvas's right and bottom edges stayed on the same pixels in every frame. PASS for this layout.
- The default layout: the canvas was the same 1158 x 767 (between the pane borders at x 228 / 1387 and from the title bar's border to the header's line) with the circle shown (pass 2 of `cold-b\`) and with it gone (no folder).
- Other pane layouts (the right pane hidden, both sides hidden, the strip alone hidden) were not recorded with the circle shown and gone.

### Box 3: a long status text and an error's readability

- A 563 px wide window on `longtext\`, the D70 file current while pass 2 ran. The row read `error: NIKON_D70_Nikon.nef: libjpeg fatal error: Not a JPEG file: starts with 0x3c 0x44  ar` (the text cut at the region's edge), then `51 / 51` and the circle at their usual place on the right. PASS.
- Judged: the error reads clearly in the row's muted color at both widths (1600 px and 563 px).

## The status row with no current-file info

- `DROP_TEXT`: two text files dragged from an Explorer window on `drop\` onto the window. The first attempt drew no outline and changed nothing; the same drag again worked (the first drag likely began before Explorer's selection settled). The outline lit, and the status row read `Drop a single folder or RAW file.` from x = 16 in the window, the same column as `no folder` and the scan's text. PASS. A one-file drag opened the file's folder, `drop\` (`no RAW files in D:\Photos\tests\20261122-batch7\drop`).
- The scan suffix: `scanning N/600` and `analyzing N/600` started at the same left column, with no leading spaces, in every frame of `cold-b\`'s burst. PASS.

## The strip position in the status bar

- A long status text: in a 544 px client (`nef\`, MCP `screenshot`) and in the 563 px window above, the text was cut first and the position (`1 / 3`, `51 / 51`) stayed whole at the right. PASS.
- The popovers on `few\`: `Sort ▾`'s menu opened upward with its right edge on the button's (x 1537), and `Filter ▾`'s with its right edge on the button's (x 1600). PASS.
- Seen for the todo-three-decisions item, not edited here: in the 544 px client with no circle, `1 / 3` ended 14 px from the client's right edge. At 563 px with the circle, `51 / 51` ended about 16 px further left, right of which was the circle. At 1600 px the position ended at x 1579 to 1580 whether or not the circle showed.

## The always-visible scrollbars

### Box 1: the four bars at rest

- Tree, at rest:
  - batch folders expanded past the pane: a thumb at x 218–223, 4 px from the pane's edge.
  - strip, `many\`: a thumb under the names at the bottom left, before any scroll.
  - meta pane, a DNG's metadata on `mixed\` and an ARW's on `many\`: a thumb at x 1598–1603.
  - dialog body, `Import…` on `mixed\` (21 destination folders) in a 602 px tall window: a bar at the body's right, x 1119, before any scroll.
- The settings' Keyboard Shortcuts list, a `PopupMenu` and the Dark theme `Select` list also showed their bar when they overflowed.
- PASS.

### Box 2: thumb drag and track click without scrolling first

- Strip: dragging the thumb right scrolled the strip, and the thumbnails of the new range loaded during and after the drag. A track click at x 1200 jumped the view there and its thumbnails loaded.
- Tree: dragging the thumb up scrolled the tree with it, and a track click jumped it.
- Dialog body: dragging the thumb down scrolled to the end of the plan, and a track click at the top scrolled back.
- PASS for these three. The meta pane's thumb was neither dragged nor its track clicked.

### Box 3: content that fits

- `few\` (three cells) with the tree's `C:\` and `D:\` collapsed: neither the strip nor the tree drew a bar.
- A press on the second cell selected it (position 2), and a press on a tree row reached it (the `C:\` incident). PASS.

### Box 4: the tree's thumb, the badge gap and a press on a badge

- Nothing lay under the thumb in any of three captures:
  - at rest
  - hovered (the thumb slightly wider)
  - dragged
- Names were cut with an ellipsis before x 212.
- Measured on `few`'s row:
  - the badge's pill ends at x 211, 16 px from the pane's right edge at x 227; its digit ends at x 205
  - the resting thumb spans x 218–223
  - the gap from the pill's right edge to the thumb is 7 px (x 211 to x 218)
- A press at x 208 (the pill's right end) on `few`'s row opened `few\` rather than going to the track. PASS.
- The collapsed tree drew no bar. Whether a non-overflowing tree looks otherwise unchanged ("4 px more room right of the badges") was not compared: that needs a build from before `native-scrollbar-gutter`.

### Box 5: the meta pane's and a dialog's bar over their text

- The meta pane: on `many\`'s ARW the values end at x 1591 and the thumb spans x 1598–1603, so no value is under it. PASS.
- The meta pane's notes (`JPEG folder: view only`) and errors (the D70's libjpeg error) start at x 1400, 12 px inside the pane's left edge (x 1388). The error wraps before the right edge, which is consistent with a 12 px inset but did not measure it. Their right inset was not measured.
- The `Import…` dialog: the plan's lines end at x 977, the bar is at x 1119, and no body text is under it. PASS.

### Box 6: the strip's bar, the burst band and the name band

- On `many\`, the bar sits below the names, in the gutter. Nothing was under the thumb at rest, hovered or dragged. A burst's band (`1/2`, `2/2`) ended above the bar. PASS.
- A press on a name band selected that cell (`_DSC0261.ARW`), and a right press on another opened the strip's menu with that cell selected (`_DSC0273.ARW`). PASS.

### Box 7: the 9 px band, the preview's height, the edge drag, the 1:1 re-cut and a short window's menus

- `few\`: the cells end at y 960, and y 961–969 is an empty band above the status row's border at y 970. PASS.
- The header's top line was at y 801 for no folder, `few\`, `mixed\` and `cold-b\` in the default window, so the preview's height did not jump between folders. PASS.
- The drag on the header's top edge (run once, cited under the strip header's box 3):
  - upward in the 1009 px window it stopped at `MAX`: the line at y 628, cells 302 px
  - downward in a 603 px window it stopped at `MIN`: cells 96 px
  - upward in that window it stopped where the preview is 240 px (the line at y 274), `PREVIEW_MIN`
  - PASS.
- `z` on `mixed\`'s first file, then the strip dragged shorter: once the drag settled, the crop filled the taller canvas (more of the image above and below). PASS. A re-cut prints no `zoom` line (only a `z` press does), so this rests on the captures.
- In the 603 px window, with the strip at the tallest it allowed, the `Filter ▾` menu fitted between the title bar and the header. PASS.
- Finding (H): shortening the window by its bottom edge kept the strip at `MAX`, leaving the preview 188 px tall, under `PREVIEW_MIN`. The next drag of the handle restored the 240 px preview.

### Box 8: the bar in three themes

- Neutral dark (all the captures above), Neutral light, and Catppuccin Mocha as the Dark theme, switched on the Appearance page. The tree's, strip's and meta pane's bars were visible and distinct from their pane in each. Judged acceptable.
- The OS appearance change in System mode was not made: the user's Windows setting was left alone.

## The strip's header row with Sort and Filter

### Box 1: the header, the buttons and their menus

- On `few\` the header held `Sort ▾` and `Filter ▾` at its right end:
  - each menu opened upward from its button
  - a second press on `Sort ▾` closed its menu
  - a press on the preview closed `Filter ▾`'s
  - `escape` closed it too
- With `Portrait` chosen, `Filter ▾` had a white border; hovered, the border went to the input color (the known `Button` limit). On `jpeg-only\` `Sort ▾` was dimmed, did not change when hovered, and a press opened nothing. PASS.

### Box 2: the long filter menu in a short window

- `mixed\` in a 603 px window with the strip as tall as that window allowed (the preview at 240 px):
  - `Filter ▾`'s menu stayed inside the window, between the title bar and the header, with its own bar
  - it scrolled on the wheel (from `Reset` to the ratings and `Color label`)
  - a press on its button closed it
  - the header did not clip it
- PASS.

### Box 3: the handle

- `GetCursorInfo` after settling the pointer at each y, on x 800 and on the `Sort ▾` button's x 1512:
  - y 801 and above: the arrow cursor
  - y 802–806: the north-south resize cursor, the 5 px band
  - y 807: the arrow again
  - from y 808 on `Sort ▾`: the hand cursor
- The 1 px line is at y 801 (`#1a1a1a` against the preview's black). The band ends a pixel above the button's border (y 807), so a press on the button cannot land in it.
- The drag to `MIN`, `MAX` and `PREVIEW_MIN` is box 7 above.
- PASS.

### Box 4: the image box at the default height

- `few\` at the stored default 128: the current cell's outline spans x 8–153 and y 840–953. The image box inside it is x 9–152 and y 841–936, 144 x 96. PASS.

### Box 5: presses, a right press and a wheel on the header

- On `many\` with the third cell current, a press, a right press and two wheel steps at (300, 817) on the header changed nothing:
  - the same cell stayed selected
  - no menu opened
  - the strip did not scroll
- PASS.

### Box 6: `toggleStrip`, the menu and the status row

- With `Filter ▾`'s menu open, `toggleStrip` hid the header with the strip and closed the menu. A second press brought the strip back.
- The status row's top border stayed at y 970 (a 30 px row) with the circle shown (pass 2 of `cold-b\`) and gone (no folder).
- Under a filter it read `1 / 3 (3)` and `3 / 265 (300)`, `n / shown (all)`.
- PASS.

### Box 7: the measurements

Measured on window captures at scale 100 %, 1600 x 1000 client, tree and meta pane at their default 220 px, strip at its default 128 px. The method was scanning pixel rows and columns for color runs.

- Preview canvas: x 229–1386 and y 34–800, 1158 x 767 px of preview surround. Counting the 1 px pane borders at x 228 and x 1387 and the header's line at y 801, the area is 1160 x 768. The note's expected 1160 x 810 does not hold: the canvas is 42 px shorter than that.
- Status row: y 970–999, 30 px including its 1 px top border.
- Strip pane: y 801–969, 169 px. That is the header's 32 (from the line at 801), the cells' 128 (y 833–960, the outline's 7 px padding equal above and below) and the gutter's 9.
  - The header's height comes from that arithmetic: the header and the cells share the black surround, with no line between them.
- Title bar: y 0–33, with its border at y 33.

## Other things seen

- The `no RAW files in …` status line shows the path without `\\?\` on this build.
- The meta pane's error on the D70 file reads `libjpeg fatal error: Not a JPEG file: starts with 0x3c 0x44`.
- Its strip cell reads `panic while encoding the thumbnail of \\?\D:\…`. Both are existing items.
