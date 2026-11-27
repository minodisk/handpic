# Results: 2026-11-27 Windows real-device checks through MCP (dlg-todo-c3)

Host: Windows 11, one 5120x1440 display, 32 GB RAM, other Claude sessions
compiling and running their own Handpic instances. App: debug build of
`f5cab4bf` with `--features screenshot` (sccache bypassed with
`RUSTC_WRAPPER=`), launched with its own `--index`, `--settings`, `--log`
under `D:\Photos\tests\20261127-c3\runs\<run>\`, `--mcp-port 47813`, and
`{"mcpEnabled": true, "mcpAllowDestructive": true}` through
`--import-settings`. Driven by a small Python MCP client (Streamable HTTP,
one session per call, or one session for a series of `navigate` calls) and
`screenshot`; windows closed with `CloseMainWindow` (WM_CLOSE). Test folders
are hardlinks (and, for the trash runs, copies) under
`D:\Photos\tests\20261127-c3\`.

## f overlay item ("App: real-device checks of the f focus mark, face boxes and face mesh")

- Box 2 (folder switch mid-pass 2): on `big\a` (400 ARW hardlinks, fresh
  index) `open_folder` of `big\b` at `analyzing 45/400` stopped pass 2
  (`scan faces: ... files=400 done=46 ... canceled=true`), `b` was listed and
  scanned; reopening `a` resumed at `analyzing 15/354` (the 354 files left).
  `open_folder` goes through `open_from_tree`, the tree click's path.
  CLOSED.
- Box 1 (eyes lines while paging with `f` on): with `set_overlays` focus on,
  60 `navigate next` calls on one MCP session at about 8 ms apart (faster
  than key repeat) logged three `eyes` lines: the first file (`_DSC1493`,
  where the burst started), `_DSC1533` (about page 40, the file current when
  the first judgment settled after 325 ms) and `_DSC1552` (the last, where
  the burst stopped). 40 calls about 35 ms apart (key-repeat pace) logged
  four: `_DSC1553` (start), `_DSC1567`, `_DSC1580`, `_DSC1592` (stop). This
  is the design: `EyesCache::request` (`crates/app/src/eyes.rs`) issues no
  ticket while one is in flight, so a hold runs one judgment at a time to
  its end and then asks for the file current at that moment; files passed
  between are never judged. A consequence: during a hold, the `current`
  check never fires for a newer request (no newer ticket is issued while one
  runs); only a `clear` supersedes a running judgment. CLOSED (behaviour
  recorded; the box's "only files stopped on or passed slowly" holds up to
  the one-in-flight sampling, about one file per 0.3-0.5 s judgment).

## private bytes item ("App: private bytes grow while → is held in the 1:1 view")

- `set_view zoom` on `big\a` (the ARW hardlinks of `D:\Photos\2026\2026-11-07`,
  1:1 crop shown), then six series of 120 `navigate next` at about 37 ms
  (4.3-4.9 s each) and one of 400 `previous` (15 s), 1120 zoomed pages in
  all. `PrivateMemorySize64` after each series: 242 (before) -> 208, 203,
  206, 206, 205, 210 -> 221 MB after the long one. No climb: plateau. Debug
  build; the 418 -> 557 MB of `gpui-zoom-peaking` was on the earlier app.
  CLOSED (the whole item).

## strip item ("App: real-device checks of the strip, the scan progress, the resume and the resync")

Folders: hardlinks of `D:\Photos\2026\2026-06-14` (1520 ARW, `cold1`) and
`2026-07-18` (1545 ARW, `cold2`), dates no run had opened that session (cache
state not measured; `robocopy /J` copies were not made, to spare D: while
other sessions used it). Pass 1 ran at about 120-150 files/s.

- Box 2 (End right after the window opens): fresh index on `cold2`,
  `navigate last` as soon as MCP answered (`scanning 1/1545`). At 0.5 s the
  ten cells in view were gray, at 1.2 s (`scanning 124/1545`... status row
  showed `124`) nine of ten held thumbnails, at 2.9 s all. Then (on `cold1`,
  fresh index) a `show_photo` of the middle file (`_DSC5247`, 761/1520) at
  `scanning 455/1520`: its cells gray at 0.5 s, all filled at 1.0 s with pass
  1 still at 650/1520. CLOSED (a jump stood in for the wheel; the range
  change is what steers the hot list either way).
- Box 1 (placeholders turning into thumbnails from the current cell outward):
  fresh index on `cold1`, current cell first: at 0.9 s (`scanning 59/1520`
  on screen) seven of ten cells in view held thumbnails, not strictly in
  order (cells 4, 6 and 9 still gray; parallel reads finish out of order),
  each with its name and the current cell outlined; at 1.4 s all ten. CLOSED.
- With both boxes closed the item goes.

## tree / trash item ("App: real-device checks of the folder tree and Move to Trash… left open (gpui-tree-trash)")

- Step 2 box: switching folders during a scan stops it (see the f overlay
  item's box 2, the same `open_from_tree` path); launched with no folder
  argument (fresh settings, so no `lastFolder`), `get_status` folder null,
  then WM_CLOSE: the process exited (the log's last line is
  `ERROR ... ウィンドウ ハンドルが無効です。 (0x80070578)`, the known
  window-placement line of `20261114-gpui-session-restore` learnings).
  CLOSED.
- Step 7 box: on `mixed` (2148 sample RAW hardlinks, fresh index) a
  `trash_photos` confirmed during pass 1 answered with
  `Move to Trash: waiting for the scan to finish` (operation
  `held_for_scan`), the scan was cancelled (`cancels_scan`) and the file
  moved (`Moved 1 file to the Trash`). Repeated, then `open_folder` of
  another folder before the scan ended: the operation became `failed`,
  `dropped: another folder was opened`, the note cleared and the file stayed.
  LEFT: the dialog's own parts (it closes at once, clicks on the tree, strip
  and status menus do nothing while it is open; an MCP-confirmed run opens no
  dialog) and the Delete Permanently judgment-undo check (every delete opens
  the dialog, even over MCP).
- Step 8 box: on `trash\a` (copies), a 3-star file moved to the Trash and
  restored by `undo` came back with its `.xmp`; the strip cell showed its three
  stars (screenshot). Cross-folder: trash in `a`, `open_folder b`, `undo`:
  `Restored 1 file from the Trash`, and reopening `a` listed 3 files. A
  `redo` of the trash run answered with `moving to Trash 0 / 1` (operation
  `Redo Move to Trash` running 0/1), then `Moved 1 file to the Trash`. LEFT:
  the Ctrl-click that drops a middle file from a selection.

## open_folder during a rename ("App: real-device check of `open_folder` refused during a rename over MCP")

- First attempts: the racing `open_folder` calls were sent concurrently with
  the confirm and reached the view before it, so the folder opened first and
  the confirmed rename was then held behind that open's listing and scan
  (`Rename…: waiting for the scan to finish`); nothing here contradicts the
  rule that an open drops a held rename (no open arrived after the hold). A
  rename of a folder with no index rows ran in 10 ms, before the opens
  arrived.
- Then: `mixed2` with 2148 index rows (pass 1 done), `trash\b` open and idle;
  `rename_folder` confirm, and eight `open_folder` calls on `mixed2` from
  other MCP sessions 1-16 ms after the confirm answered: all eight answered
  `A folder is being renamed; try again in a moment.`; the rename finished
  in 796 ms (`files=2148` carried). CLOSED.

## fit-preview-downscale item ("App: by-eye checks of the scaled fit-view decode")

On `D:\Photos\tests\20261108-gpui-compare\RAF`, debug build, `X-E5_DSCF1008`:
- Box 4 (grayscale): `set_overlays grayscale` (the latch the `g` key's hold
  shares): the photo gray in the fit view and in the 1:1 crop, the strip
  thumbnails and the red peaking overlay in color. CLOSED.
- Box 3 (peaking): `set_overlays peaking` showed the layer in the fit view
  and at 1:1; after `set_panes` hid both sides and the strip (the canvas grew
  from about 1150x765 to 1400x935 logical px), the layer was redrawn at the
  new size, its lines thin and on the edges. CLOSED.
- Boxes 1, 2, 5, 6: left (they compare against a build from before
  `d3794643` or need a burst of captures during a resize).

## HDR PQ CR3 pass 1 ("App: measure pass 1 on a folder of HDR PQ CR3 files at cap 2, before and after the HEVC split")

- Samples: of the six named files, `handpic-cli info` gives an HEVC preview
  and full image for five (`R8.CR3`, `R5m2.CR3`,
  `Canon_EOS-1D_X_Mark_III_AJKC0638.CR3`,
  `Canon_EOS-1D_X_Mark_III_AJKL0637.CR3`, `Canon_EOS_R10_IMG_1205.CR3`);
  `Canon_EOS_R5m2_102A6405.CR3` has JPEG ones (not HDR PQ). The five decode
  without a panic (`handpic-cli crop` wrote each). Folder:
  `D:\Photos\tests\20261127-c3\hdrpq`, 60 hardlinks of each, 300 files (so
  the reads are cache hits after the first run; the same for both builds).
- Builds: release `handpic.exe` of `39474bdb` (`4b1fd823^`, before the split)
  and of `f5cab4bf` (after), `READ_SLOTS` 2, 22 scan threads. Each run a
  fresh `--index`, the window opened on the folder, pass 1's time taken from
  the `scan extract:` line, the instance closed (WM_CLOSE) before the next.
  Alternated before / after, three each; total CPU load (other sessions
  compiling) sampled just before each launch.

  | Build  | CPU before | Pass 1 (300 files) | ms/file |
  | ------ | ---------- | ------------------ | ------- |
  | before | 42 %       | 28122 ms           | 93.7    |
  | after  | 52 %       | 5264 ms            | 17.5    |
  | before | 63 %       | 30535 ms           | 101.8   |
  | after  | 37 %       | 3833 ms            | 12.8    |
  | before | 53 %       | 29526 ms           | 98.4    |
  | after  | 60 %       | 3911 ms            | 13.0    |

  Before the split pass 1 paid 94-102 ms/file, above the calculated 33-63
  (the cause of the gap was not looked into; the other sessions' CPU load
  slows each decode too); after it 13-18 ms/file, about 7x faster. CLOSED.

## Not run here

- Cold-disk pass 2 (needs a reboot or a fresh card, and an 8-thread
  benchmark the host rule forbids while other sessions share it).
- SATA scan pause (needs D: idle; other sessions read it, 30-210 MB/s spikes
  in a 5 s typeperf sample).
- `present` p95 (only key presses print `page` lines; MCP `navigate` prints
  none, and synthetic keys would hit other sessions' windows).
- The filter / sort menus, the EXIF menu, the IME, the second monitor (one
  display here), the menus' clicks (need a real mouse or keyboard).
- Camera / Lens collation: delegated to `dlg-exif-collate`.
