# Learnings: gpui-strip-index

## Step 1

- `riffle-native` gained three direct dependencies, all already in its lock
  through `gpui` / `riffle-backend`: `dirs` 6, `futures` 0.3 and `log` 0.4
  (`log` was not named in the plan but the logger needs the trait). The lock
  gained only those three dependency lines.
- `Scan::prepare` takes the listing (`scan::list`, which canonicalizes and
  calls `read_listing(dir, None)`) rather than the folder, so a failed
  `reconcile` still leaves the listing for the pager: the index error is
  logged and no scan starts, the pager pages as without an index.
- `std::fs::canonicalize` on Windows gives `\\?\D:\...`; every index key and
  every listed path in the native app carries that prefix, as in the Tauri
  app. `read_preview` reads such paths fine.
- The backend does not log `scan list` / `scan reconcile` (those lines are in
  `crates/app`'s `scan_folder`); `scan.rs` logs its own with the same names.
- The eviction thread is spawned when the view is created and the first open
  joins its handle on the background executor before `reconcile`, so the scan
  always waits for the `VACUUM` instead of racing it for the writer `Mutex`.
- A stale `riffle-native.exe` from an earlier session (PID 58260, started
  2026-11-09) held `target/debug/riffle-native.exe`, so `cargo build` failed
  with `os error 5`. Renaming the running exe aside
  (`riffle-native.stale-58260.exe`) let the build go through without killing
  the process.
- The logger shows GPUI's own lines too, among them an `ERROR` from
  `gpui_windows::directx_devices` (0x887A002D, no DXGI debug interface), which
  is harmless.

### Hand-run (debug build, 2026-11-10)

- `riffle-native.exe D:\Photos\tests\20261108-gpui-compare\ARW --auto 3
  --index D:\Photos\tests\20261110-gpui-strip-index\index.sqlite`, twice, a
  fresh index file. Run 1: `scan reconcile: ... todo=20 removed=0`,
  `scan extract: ... files=20 done=20 errors=0 threads=22 ... in 2136ms`.
  Run 2: `todo=0`, `scan extract: ... files=0 done=0 errors=0 ... in 1ms`
  (rows reused). Both quit by themselves with the `page` lines and summary.
- A copy of that folder plus a 64 KB random-bytes `DSC-RX100M3_corrupt.ARW`
  (`D:\Photos\tests\20261110-gpui-strip-index\ARW`, own fresh index),
  `--auto 23`: `scan extract: ... files=21 done=21 errors=1`, the status line
  read `1/21  DSC-RX0M2_sony_rx0_ii_40.arw  ...  1 failed`, the pager printed
  `error DSC-RX100M3_corrupt.ARW: invalid TIFF byte order` and went on: 22
  `page` lines, every press shown.
- The `scanning done/total` text itself was not caught on screen: the warm
  20-file pass ended (55 ms, 2.1 s cold in debug) before a screenshot did.

## Step 2

- `strip.rs` takes `width` as an argument (`visible_range(width)`,
  `scroll_into_view(index, width)`, `next_to_request(width)`, the viewport
  center computed inside) rather than storing it; the view keeps
  `strip_width`, the window's width, updated by `observe_window_bounds`. The
  strip spans the window from its left edge, so a mouse event's window `x` is
  the strip's `x`.
- `Pager` gained `Move::To(index)` for a click on a cell; a click goes through
  the same `go` as the arrows, so it also stamps the `page` key time and
  scrolls the strip `block: nearest`.
- `Event::Progress` now carries the scan's `ready` paths. They are
  `FileStat.path.to_string_lossy()`, the same `\\?\D:\...` strings the shown
  `PathBuf`s give back, so `Strip::new` keys on those.
- The reader connection is opened next to the writer; as in the Tauri app, the
  writer stands in for a reader that will not open.
- A `no cached thumbnail` answer is followed, in the same background read, by
  `Index::entry(path)` for the row's `error`, which is the failure text drawn
  in the cell (`invalid TIFF byte order` on the corrupt `.ARW`).
- GPUI at this rev deprecates `on_scroll_wheel` / `ScrollWheelEvent` in favour
  of `on_scroll` / `ScrollEvent`; `background_spawn` on `AsyncApp` needs the
  `AppContext` trait in scope.
- The plan's 144x96 image box is what was built: a portrait thumbnail
  (orientation 6 / 8, 270x404 upright) fits at 64x96. The Tauri cell's `img`
  is actually a 144x144 box (`style.css` `.cell img`, the cell 168 px tall),
  so a portrait there shows at 96x144. Not changed here (the plan names
  144x96 and a 144-tall box makes the strip ~50 px taller); recorded below.
- A temporary `eprintln!` in `Riffle::thumbnail` (removed before the commit),
  debug build, `D:\Photos\tests\20261110-gpui-strip-index\ARW` (20 ARW + the
  corrupt one), fresh index, `--auto 21`: the first reads came back
  `NotScanned` for cells 0-15 (the 1600 px window's 11 cells + margin, nearest
  the center first: 5, 6, 4, 7, 3, ...), the scan ended 59 ms later, its
  `ready` paths / `refresh` brought them back as 404x270 images (one 270x404,
  the orientation 6 file), cell 9 as `failed invalid TIFF byte order`; paging
  to the end asked for 16-20 and dropped 0-5, and the wrap to the first file
  asked for 0-5 again. 20 `page` lines, the corrupt file's `error` line, exit 0.

### Paint timing (release, 2026-11-10)

`riffle-native.exe D:\Photos\tests\20261108-gpui-compare\ARW --auto 20 --index
D:\Photos\tests\20261110-gpui-strip-index\step2.sqlite` (fresh file), twice,
no `--size`:

| run | index | `paint` p50 / p95 | `frameWait` p50 | `keypressToPixels` p50 / p95 |
| --- | --- | --- | --- | --- |
| 1 | cold (pass 1 of 20 files, 34 ms) | 0.66 / 1.53 ms | 9.79 ms | 18.51 / 25.77 ms |
| 2 | warm (`files=0`) | 0.71 / 1.40 ms | 10.76 ms | 19.14 / 24.72 ms |

`paint` is the preview canvas's paint only (the strip's canvas paints in the
same frame but outside that stamp); its p50 stays within 1 ms of port Step 3's
0.72 ms. No A/B against the pre-strip build was run here; that is Step 5's
gate.

## Step 3

- `focus::scan_focus_paths` takes `first` / `last` as `usize`, so the Tauri
  test's negative `first` (`-4`) became `0`; the other cases are the same.
  It is generic over the element, so the view passes its `PathBuf`s.
- The hot list is set on the `ScanFocus` handle before `Scan::start` spawns
  the thread instead of right after it, as the Tauri frontend does: with it
  sent after, the first wave of the 22 workers took files 0-10 in listing
  order before the hot list arrived (see the order run below). Later changes
  go through `focus_scan_soon`, a `cx.spawn` timer task of
  `SCAN_FOCUS_DELAY` replaced on each strip move (which every page, scroll and
  resize goes through); it does nothing once the scan is done.
- `viewed.rs` is three pure state machines: `Latest` (`lastViewedWriter`),
  `Batches` (`viewedWriter`, folders in first-pushed order, paths in pushed
  order, each once) and `Dwell` (`dwellTimer` without the timer: `shown`
  hands out a ticket, `elapsed(ticket)` gives the path to record only if no
  later `shown` / `cancel` voided it; it also holds the folder's viewed set
  loaded at open). The view holds the dwell's timer as a `Task` and drops it
  to stop it.
- `set_last_viewed` is written on every page change (Tauri's `show()`), not
  at open; the dwell starts when a preview is decoded and shown, and is
  cancelled by a page change, a failed preview and an open.
- `last_viewed` and `viewed_files` are read through the reader connection in
  the open's background task, after `reconcile`.
- Seeding a test index by hand with `opened_at = 0` lost the row: the
  launch's eviction dropped the 30-day-old folder with its `last_viewed`
  before the open read it.

### Hand-run (debug build, 2026-11-10)

- Resume: `riffle-native.exe D:\Photos\tests\20261108-gpui-compare\ARW --auto 6
  --index D:\Photos\tests\20261110-gpui-strip-index\step3.sqlite` (fresh
  file) paged from the 1st to the 7th file (`DSC-RX100M2_Sony_RX100M2.ARW`).
  A second run with `--auto 1` on the same index printed one `page` line for
  `DSC-RX100M3_20210527_0205.ARW`, the 8th: it opened at the 7th. The index
  then held that 8th file as `last_viewed` and `views` rows for the 1st-8th
  files (each was on screen about 1 s, over the 500 ms dwell).
- Scan order: a fresh index with a `folders` row for
  `\\?\D:\Photos\2026\2026-06-27` (373 ARW) whose `last_viewed` is
  `_DSC9141.ARW` (the 200th, index 199), then `--auto 0` on that folder with a
  temporary `eprintln!` of each `Progress` message's `ready` paths as listing
  indices (removed before the commit). With the hot list set after the start:
  the first batch was 185-203 interleaved with 0-10. With it set before: the
  first 22 files committed (`200, 198, 194, 202, 1, 189, 197, 199, 185, ...`)
  were 185-204 and 0, 1, then the listing order went on from 2; 185-204 is
  the strip's range around 199 (the 1600 px window, 199 at its right edge,
  `RANGE_MARGIN` 4), and 0, 1 are the two of the 22 workers left over.

## Step 4

- The watcher's callback sends a new `Event::Changed(dir)` into the Step 1
  channel; `watch::set` runs in the listing's background task, after the
  previous scan is joined and before `read_listing`, with the canonical path as
  both the watched dir and the owner string, so the event's string compares
  with `self.dir` directly.
- `resync.rs`'s `Resync` is two booleans (`busy`, `pending`): `open` sets busy
  and drops a pending change, `changed` re-lists when idle else marks pending,
  `finished` (the scan's `Done`, or a listing that started no scan: no index,
  a failed `reconcile`, a failed listing) runs the pending re-listing or goes
  idle. The current file's neighbour rule (`carried_current`) sits there too;
  `carriedOffset` / `carriedIndices` became `strip.rs`'s `carried_offset` /
  `carried_indices` with the Tauri cases (in `CELL_PITCH` units).
- A re-listing needs a guard the open did not: a preview read for the
  previous listing names an index into the old `files`, so `send` tags it with
  a `listing` counter and a stale result only retires its image. The
  re-listing always requests the current preview again; that keeps the
  pager's in-flight state consistent and refreshes the status line's `n/total`
  at the cost of one read per change.
- `opened` now takes `carry`: the initial open resumes at `last_viewed` and
  resets the strip and thumbnails; a re-listing skips the `last_viewed` /
  `viewed_files` reads and carries the pager, the strip's offset and the
  loaded thumbnails (`Thumbs::carry`, `Strip::keep` so they are not asked
  again). When the current file changed (deleted), the dwell is cancelled and
  the strip scrolls it into view; `set_last_viewed` is written either way.

### Hand-run (debug build, 2026-11-10)

Temporary `eprintln!`s in `carry` (and, for the second run, a 3 s sleep before
`run_scan`) were removed before the commit. Fresh `--index` files under
`D:\Photos\tests\20261110-gpui-strip-index\`, no `--auto`, the exe killed by
the script at the end.

- `step4` (10 ARW of the compare folder): a copy 0.3 s after launch landed
  before the listing (`files=11`). Copying in `AAA_first.ARW` (sorting first)
  gave `resync` → `files=12`, `todo=1`, the current file kept by path (index
  0 → 1, the same `DSC-RX0M2_sony_rx0_ii_40.arw`), the offset carried to
  154 px (one cell) and all 11 thumbnails kept at their shifted indices. That
  `cp` of a 21 MB file took about 11 s, and each debounced event in between
  re-listed with `todo=1 removed=1` (its size changed between stats), about
  one per second, then `todo=0 removed=0` once it was whole. Deleting the
  current file: `files=11`, `removed=1`, current index 1 stayed 1, now
  `DSC-RX0_DSC00009.ARW` (the next neighbour), offset clamped to 94 px. Two
  copies at once: one re-listing, `files=13`, `todo=2`.
- `step4b` (5 ARW, 3 s sleep before each scan): the copy at 2 s logged
  `resync: ... deferred` while scan 0 slept, and its re-listing ran right
  after scan 0's `scan extract` (`files=7`, `todo=1`); deleting a non-current
  file re-listed to `files=6`, `removed=1`, the current file unchanged.

## Deferred issues (todo candidates)

- The strip's image box is 144x96 per this plan, but the Tauri strip's is
  144x144 (`crates/app/ui/style.css` `.cell img`, `object-fit: contain`
  inside a 168 px cell), so a portrait thumbnail shows at 64x96 in the native
  strip against 96x144 in the Tauri one. Decide whether the native cell grows
  to the Tauri size (strip about 50 px taller) when the strip gains its marks
  (port Steps 6-8). Basis: Step 2 implementation; files:
  `crates/native/src/strip.rs` (`BOX_HEIGHT`, `CELL_HEIGHT`),
  `crates/native/src/main.rs` (`paint_strip`).
- Pending manual check (Windows, the user's eyes; Step 2 was ticked on the
  unit tests, the stderr-instrumented run and the `--auto 20` paint timing
  above): the strip in the running app. Build with
  `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build --release`
  in `crates/native`, then run
  `crates\native\target\release\riffle-native.exe <a folder of 1000+ RAW files under D:\Photos\> --index D:\Photos\tests\20261110-gpui-strip-index\manual.sqlite`
  (a fresh file, so pass 1 runs). Expect:
  1. While `scanning N/total` climbs, the strip's gray placeholders turn into
     thumbnails from the current (first) cell outward, the cells nearest the
     middle of the strip first; each cell's file name is under it and the
     current cell has a light outline.
  2. Holding `→` for a few seconds: the outlined cell and the preview move
     together, the strip scrolls just enough to keep the outlined cell in
     view, and on release the preview, the outline and the status line name
     the same file. `Home` / `End` jump both to the first / last file.
  3. Scroll the strip with the mouse wheel over it (toward you scrolls right)
     to a far cell and click it: the preview shows that file and the outline
     moves to it. A click in the 8 px gap between cells does nothing.
  4. With Task Manager's Details tab showing the `riffle-native.exe` row's
     "Memory (private working set)" (or "Commit size"), wheel the strip from
     one end to the other and back several times: the value rises while
     scrolling and settles back to about the same level, not climbing run
     after run (thumbnails outside the range plus 4 cells are dropped).
  5. A folder holding a corrupt RAW (e.g.
     `D:\Photos\tests\20261110-gpui-strip-index\ARW`, file
     `DSC-RX100M3_corrupt.ARW`): its cell shows `invalid TIFF byte order` in
     the gray box instead of a thumbnail.
  Basis: Step 2's Done-when "Manual check: open a 1000+ file folder ...";
  files: `crates/native/src/main.rs`, `crates/native/src/strip.rs`,
  `crates/native/src/thumbs.rs`.

- Pending manual check (Windows, the user's eyes): the status line's
  `scanning done/total` climb. Open a folder of a few hundred RAW files under
  `D:\Photos\` with `riffle-native.exe <folder> --index <a fresh file under
  D:\Photos\tests\>`; expect `scanning N/total` after the file info, N rising
  about every 100 ms, replaced at the end by nothing (or `N failed`). Step 1
  was ticked on the automated criteria and the stderr / `1 failed` checks
  above. Basis: Step 1's Done-when "the first run shows the progress climb to
  20/20"; files: `crates/native/src/main.rs` (`Riffle::status`).

- Pending manual check (Windows, the user's eyes; Step 3 was ticked on the
  unit tests and the two stderr / index runs above): the resume and the
  steered scan in the running app. Build with
  `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build --release`
  in `crates/native`, then:
  1. Run `crates\native\target\release\riffle-native.exe <a folder of 1000+ RAW files under D:\Photos\> --index D:\Photos\tests\20261110-gpui-strip-index\manual3.sqlite`
     (a fresh file), wait for the scan to end, press `End`, then `←` a few
     times and wait a second, and quit with `q`.
  2. Run it again with the same arguments: the preview, the outlined strip
     cell and the status line open at the file left at in step 1, and the
     strip is scrolled to it.
  3. Run it on the same folder with another fresh file,
     `--index D:\Photos\tests\20261110-gpui-strip-index\manual3b.sqlite`, and
     right after the window opens press `End`: the thumbnails near the last
     cell appear within a second or so, before the cells in the middle of
     the folder are scanned (`scanning N/total` still climbing), and wheeling
     the strip to the middle and stopping fills the cells there next.
  Basis: Step 3's Done-when "Manual check: ... thumbnails near the cursor
  appear before the far ones"; files: `crates/native/src/main.rs`
  (`focus_scan_soon`, `remember`, `start_dwell`), `crates/native/src/focus.rs`,
  `crates/native/src/viewed.rs`.

- Pending manual check (Windows, the user's eyes; Step 4 was ticked on the
  unit tests and the two stderr-instrumented runs above): the resync in the
  running app. Build with
  `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build --release`
  in `crates/native`, copy a few ARW files into a new folder under
  `D:\Photos\tests\` (for example `D:\Photos\tests\20261110-gpui-strip-index\manual4`),
  and run `crates\native\target\release\riffle-native.exe D:\Photos\tests\20261110-gpui-strip-index\manual4 --index D:\Photos\tests\20261110-gpui-strip-index\manual4.sqlite`
  (a fresh file). Then:
  1. Page to the second file. From Explorer, copy another ARW into the
     folder: within about a second its cell appears in the strip at its
     name's place and gets a thumbnail; the outlined cell, the preview and
     the status line still name the same file, the status line's total grows
     by one, and the other thumbnails do not flash back to gray.
  2. From Explorer, delete a file that is not the current one: its cell
     disappears and the current file stays current.
  3. Delete the current file: the preview, the outline and the status line
     move to the file after it (the one before it when it was the last).
  Basis: Step 4's Done-when "Manual check (Windows): copy a RAW into the open
  folder from Explorer ..."; files: `crates/native/src/main.rs` (`changed`,
  `relist`, `carry`), `crates/native/src/resync.rs`,
  `crates/native/src/strip.rs` (`carried_offset`, `carried_indices`).
