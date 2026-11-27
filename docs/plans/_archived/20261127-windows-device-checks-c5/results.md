# Results: Windows real-device checks through MCP, screenshots and synthetic input (2026-11-27)

Checks of the `todo.md` items from `### App: real-device checks of the tree's
inline folder rename (gpui-folder-dialogs Step 7)` up to (not including)
`### App: real-device check of the look of the switch and the `Kbd` keys
(native-component-controls)`, driven through the MCP companion and the
`screenshot` tool, plus synthetic mouse and keys (`mouse_event` /
`keybd_event` after a few relative moves, the window made foreground, under
`D:\Photos\tests\.ui-lock`). No IME, no HiDPI display, no drag from Explorer.
Colors were not judged by eye beyond "visible".

## Setup

- Build: `crates/app` at `f5cab4bf` (main, 2.2.0), debug,
  `--features screenshot`, built with `cargo build -j 2` (an unbounded build
  was stopped by the host's low-memory reaper while six other sessions ran).
- Material under `D:\Photos\tests\20261127-c5\`: ARW hardlinks of
  `2026-09-28-gui-check` and `2026-09-28-gui-check-cold`, sidecars copied:
  `step7\Photos` (43 ARWs, 3 `.xmp`; `sub\` 2), `step7\taken` (empty),
  `step7\other` (5); `held\slow1`..`slow5` (264 each); `held\pend` (100);
  `counts\{alpha 3, beta 2 (+ deep 1), gone 2, renamed-out 2, split 125}` and
  `counts\junction` (`mklink /J` to `alpha`); `net\netcount` (2) shared as
  `\\localhost\D$\...\net` mapped to `Y:` (`net use /persistent:no`, removed
  afterwards). A truncated ARW (`_DSC0000-broken.ARW`, first 300 KB) and a
  real copy whose `FF D8 FF` markers were zeroed after its scan with size and
  mtime kept (`_DSC9999-corrupt.ARW`).
- Each run had its own `--index`, `--settings`, `--log` under `run\r1`..`r4`,
  `--mcp-port 47815`, and `{"mcpEnabled": true}` (plus
  `"mcpAllowDestructive": true` for r1, r2, r4) through `--import-settings`.
  Window 1600 x 1000 client, Neutral dark unless stated, display 5120 x 1440
  at 100 %.

## Seen (PASS)

1. Inline rename, right-click `Rename…` (r1): after judging two files (3
   stars; pick + Red) and showing the third, the menu's `Rename…` put a field
   in the row with `shots` selected whole; typed `photos` + Enter renamed the
   folder in place (`rename: ... renamed to ...photos in 194ms`), the strip kept
   its 43 files and position, `↑` then moved the tree's cursor (the tree kept
   the keyboard). Through MCP: `Photos` -> `Shots` kept files, judgments and
   position 3 with `scan extract: files=0` after it; a case-only `Shots` ->
   `shots` worked.
2. A single click on a root's name (`C:¥`) opened it (`no RAW files in C:\`)
   with no rename field.
3. A rename during an import run: with `Import…` of 264 files running
   (`importing 3 / 264`), `rename_folder` was refused with
   `Rename… is unavailable while an import runs` (then the import was
   canceled at 32 files).
4. Held rename, 5 fresh folders of 264 ARWs on one fresh index: each rename
   was confirmed at `scanning 0/264` or `analyzing 6..13/264`, held
   (`Rename…: waiting for the scan to finish`), and ran at the scan's end:
   `renamed ... in 96ms`, `201ms`, `1850ms`, `309ms`, `300ms`; no
   `Access denied`. With pass 3 in the scan the held rename ran 26-46 s after
   `scan faces` (pass 2) ended, at the scan's end.
5. App icon: the window class's big and small icons (`GetClassLongPtr`
   `GCLP_HICON` / `GCLP_HICONSM`; `WM_GETICON` returns none), the taskbar
   button (left-docked taskbar captured from the screen) and Explorer's
   `ExtractAssociatedIcon` of `crates/app/target/debug/handpic.exe` all show
   the Handpic icon; the title bar draws it at its left end.
6. Strip face icon dimming: rejecting the good frame `_DSC0787.ARW` dimmed its
   green icon with the cell.
7. Strip tooltips: over a truncated ARW's cell the themed tooltip
   `preview out of range` showed (captured about 0.6 s after the move, the
   capture itself taking about 0.3 s, so the 500 ms delay was not measured);
   its Blue-labeled band showed `Blue`, the next cell's band `Red` (the
   tooltip swapped), a plain cell none; a wheel scroll under the still pointer
   removed the tooltip; the same cell cut by the window's left edge still
   showed its tooltip, inside the window. Over tooltip regions a click
   selected the cell, a Ctrl+click on the `Red` band added it, a Shift+click
   on the `Blue` band extended the range, a right-click opened the strip menu
   (`Blue` checked) and the wheel scrolled the strip.
8. Title bar layout: with a folder open, the preview, `f` (face boxes and
   labels), `d` (performance band), `z` (1:1 crop), the tree, the meta pane,
   the strip and the status row all lay out below the bar; in a 480 px tall
   window `Filter ▾`, the strip's right-click menu and the tree's right-click
   menu stop under the bar.
9. Title bar theme: Neutral dark, Gruvbox Dark, Solarized Light, Neutral light
   and Tokyo Night each gave the bar and its border the theme's colors at
   once on `set_settings`; the gear and the controls stayed visible. The
   gear's hover fill and its tooltip `Settings Ctrl+,` showed in Neutral dark,
   Gruvbox Dark and Solarized Light.
10. Gear during an inline rename: with the rename field open, a click on the
    gear opened the settings and the field closed with the name unchanged.
11. Resize borders (gpui-pre 0.3.12): drags from the left, right and bottom
    edges, the top edge (the client's top 1 px, the bar included) and all four
    corners resized the window (the top-right corner did nothing on its first
    try and resized on four retries). Hovering the maximize button showed the
    Windows 11 Snap Layouts flyout. Maximized, the bar spans the display with
    the restore glyph and the content below.
12. `windowBounds`: with a stored `{"x":400,"y":300,"width":1400,"height":900}`
    three launches and quits placed the window at the same
    `GetWindowRect` (454, 296, 1870, 1204) and left the stored value
    unchanged: no drift. (The stored bounds are in workspace coordinates; the
    taskbar is 62 px on the left.)
13. Settings layer: with the settings open, hovering where a strip cell's
    tooltip would show showed none, and three wheel notches there left the
    strip unscrolled after the settings closed.
14. Keyboard Shortcuts: `+` then `home` showed `"home" is taken by this app`
    and `+` then `end` `"end" is taken by this app`, in red in the settings
    body (the full-window settings layer covers the status line); the keymap
    kept `pick: ["p"]`.
15. Language `Select` by keys (Sidecar, XMP on): Tab from the last name field
    focused it, Space opened the list, Down twice and Enter chose `Español`
    (the names became `Roja`, `Amarilla`, ...; `label_language: es`). With the
    list open Tab kept it open with the ring on the `Select`, Shift+Tab closed
    it with the ring still on the `Select`: the focus stayed in the settings.
16. Scrollbars (480 px tall window): dragging the thumbs of `Filter ▾`, the
    strip's right-click menu, the Keyboard Shortcuts page, the Sidecar page and
    the `Lightroom language` list scrolled them; a press on the `Filter ▾`
    track scrolled it and left it open; the tree's right-click menu overflowed
    the window and scrolled by wheel. The tree's own thumb drag scrolled the
    tree.
17. Tree rows: a held rename's long pending name (`a-very-long-pending-...`)
    showed as `◷ a-ver…` in italic with its count `100` whole. A right-click at
    the row's left inset edge (x = 6, where the hand cursor starts) opened the
    row's menu; x = 0..5 is the list's `px_1` padding and takes nothing. On the
    right, the row's hover fill runs under the 16 px scrollbar track, but a
    right-click there (x = 214 of a 220 px pane) opened no menu: the track
    takes the press.
18. No-folder message (panes at 480 / 480, window 1200 wide, preview 240 px):
    a click on the message just inside each pane's 5 px band (x = 489 and
    711) opened the folder picker; inside a band (486, 716) it did not.
19. Meta pane on a failed preview: `_DSC9999-corrupt.ARW` (scanned, then its
    JPEG markers zeroed with size and mtime kept) showed `libjpeg fatal error:
    Not a JPEG file: starts with 0x00 0x00`, a `Maker note` group holding only
    `AF point` / `AF frame`, and its full `Analysis` group (`Eye focus Sharp ·
    0.184` ... `Similar faces 1`).
20. Folder counts on Windows (r2): an index whose `folder_counts` table was
    dropped (the state before the cache table; `user_version` 28 kept)
    showed no count on any collapsed child of `counts` after an expand; an ARW
    hardlinked into `alpha` outside the app and `Refresh` on the collapsed
    `alpha` showed 4, kept after a restart; `Rename by Capture Date…` of
    `split` showed `2026-08-22 100` and `2026-09-19 25` collapsed, the emptied
    original removed; the junction kept its count 3 after a restart; `gone`
    deleted outside, `Refresh` on `counts` dropped its row from the table, and
    a new `gone` created after quit showed no count after the restart; a
    folder on the mapped network drive `Y:` showed 2 after expand / collapse
    and again after a restart (`Y:\netcount` row in the table).

## Seen (FAIL)

1. Title bar under a dialog: with the trash dialog (`Move 3 Files to
   Trash…`) or a folder operation's dialog (`Delete Sidecars…`) open, a click
   on minimize or maximize and a drag on the bar did nothing (`IsIconic`,
   `IsZoomed` and `GetWindowRect` unchanged; the backdrop dims the bar too).
   The same actions worked with no dialog and with the settings open. Handed
   to its own session (dlg-titlebar-dialog).
2. No-folder message in a 240 px preview: the message is one unwrapped line
   centered on the canvas, so it is drawn over the tree pane's background on
   the left (from x = 360) and hidden under the meta pane on the right; a
   click on the part over the tree pane opens nothing. Handed to its own
   session (dlg-empty-wrap).

## Not run / inconclusive

- The IME (Japanese) in the rename field; HiDPI (150 %); a drop of a folder
  while the settings are shown and the drop outline under the title bar; the
  tooltip's wrapping at 360 px (the only error at hand was short); the
  `Rename by Capture Date…` split whose undated files keep the original
  folder (no undated RAW at hand).
- The meta pane while a second folder opens: the first capture landed 0.37 s
  after `open_folder`, when the new folder's preview was already shown, and
  the two folders held files of the same names, so whether the pane showed
  another file's index rows before the first preview could not be told.
- The gpui-component adoption boxes (Steps 1-7) beyond what is above.
