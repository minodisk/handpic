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
