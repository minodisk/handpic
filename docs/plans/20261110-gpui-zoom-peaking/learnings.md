# Learnings

## Step 1

- The canvas's device-pixel size reaches the view through an
  `Rc<Cell<(u32, u32)>>` the paint closure writes, the smaller of the two
  options the plan named. A resize is seen by `observe_window_bounds` before
  the new size is painted, so the cell is read only when the 150 ms debounce
  fires (and when a crop arrives, to notice a resize during its read).
- `Zoom::page` releases the held crop on every page and listing, zoomed or not.
  The Tauri app keeps it until the next crop replaces it, but it is never drawn
  again (its `seq` is gone), so releasing it at once is the same on screen and
  frees the texture sooner.
- The point of interest maps through 6 / 8 by pixel index (tested against
  `orient::rotate_bgra`); the placeholder rectangle turns by continuous
  coordinates as the Tauri canvas rotation does. The two differ by at most one
  JPEG pixel on a quarter turn, under the placeholder's own scale error.
- `unsized` is a reserved word in Rust; a test variable named so does not
  compile.
- Key injection: `keybd_event` needs the window in the foreground for every
  key. In one long injected sequence most keys went missing (presumably focus
  moved away mid-run); the script was changed to re-assert and check the
  foreground before each key and skip otherwise, and to pick the process by
  its path in this worktree (another `riffle-native.exe` was exiting at the
  time). With that, every injected key arrived.
- Checked by running the release build at scale 1.0 (96 dpi, 1600x1000 client)
  on `D:\Photos\tests\20261110-gpui-zoom-peaking\` with
  `--index D:\Photos\tests\20261110-gpui-zoom-peaking\index.sqlite`:
  - every `z` that asked for a crop printed one `zoom` line (25 over ARW, RAF
    and the `jpeg` subfolder), none printed an error, and `z` off / on on the
    same file printed nothing (the held crop reused);
  - 25 `→` at 30 ms while zoomed, then `z` off / on: no error and no new
    `zoom` line, so the crop held at the end was the current file's;
  - the measured figures are in `docs/agents/native-app.md` ("A crop is on
    screen 8-112 ms after `z`"); the machine was not idle.
- Test folder: `ILCE-7M3_PPR10K_1277_4.ARW` (focus point 458 px from the top
  of 4000, orientation 1, the edge case), `ILCE-7M3_PPR10K_1283_0.ARW` (an
  off-center focus point), `ILCE-7M4_DSC07652.ARW` (orientation 6),
  `ILCE-7M3_sony_a7_iii_36.arw` (orientation 8), `X-E5_DSCF1008.RAF`,
  `X-H2S_fujifilm_x_h2s_30.raf` (RAF, orientation 8), and `jpeg\` with three
  JPEGs (orientations 1, 6 and 8), all copies from `D:\photos\samples\`.

A carry re-listing (the folder watcher) that leaves the same file current
does not call `zoom_page`: the held crop stays on screen, as in the Tauri app's
`mustReshow` false case. Only a new folder or a changed current file bumps the
page (review round 1).

## Step 2

- Phase A (implementation, smoke run) only; the held-key measurement, the
  by-eye checks and the docs (`native-app.md`, `CLAUDE.md`, the port plan's
  Progress) are left for phase B, so the checkbox stays open.
- The fit view's drawn device size is known only in the paint closure, so the
  closure compares it with the layer the view rendered with
  (`peaking::Paint`, its source's `ImageId` and size) and, when they differ,
  asks for the layer through a `WeakEntity` update in the `cx.defer` it already
  runs after paint (`want_peaking`). The size cell of Step 1 would have needed
  the fit rectangle recomputed in the view from a size one frame old; the
  deferred update has the exact drawn size and costs nothing while the layer
  matches (no defer; the closure's early return is still taken).
- `RenderImage` is `PartialEq` by its id, so the model's source type in the
  app is the `Arc<RenderImage>` itself: the layer holds the source the next
  job reads, and the tests use `u32`.
- `Peaking::is_on` and an `overlay(view, source, size)` lookup were written
  first and failed clippy's dead-code lint: nothing outside the tests used
  them. The lookup moved into the tests (it is what `Paint::paint` checks).
- The crop's overlay is asked for through `zoom::Paint::paint`, which now
  returns the crop and its size when the crop layer is another's.
- Smoke run (release build, scale 1.0, default 1600x1000 window, canvas 852
  device px high; `D:\Photos\tests\20261110-gpui-zoom-peaking\` with its
  `index.sqlite`; keys injected by
  `D:\Photos\tests\20261110-gpui-zoom-peaking-tools\drive.ps1`; the PC was not
  idle, load 31 % before the runs):
  - `e`, `→`, `z`, `e` off / on while zoomed and held `→` printed one
    `peaking` line per computed layer, no `error` line, no injected key
    skipped.
  - Fit view `resize` (bilinear `Triangle`): 12-14 ms for a 1616x1080 ARW
    (portrait, upright 1080x1616) to 569x852, 22-24 ms for a landscape ARW to
    1275x852, 35-38 ms for a RAF portrait (upright 2944x4416) to 568x852,
    47-57 ms for a 4416x2944 RAF to 1278x852. `compute` (the Sobel)
    2-7.5 ms. The crop needs no resize: `compute` 3.7-5.0 ms on 1024x852-864.
  - So on a RAF the overlay lands about 50-60 ms after the image, the resize
    being most of it. Whether that is visibly late is the by-eye check; a
    cheaper filter or another resizer is the fallback, decided with the
    measurement.
  - While a key is held, a layer replaced before its job ran is skipped
    (fewer `peaking` lines than pages), as planned.
  - Private bytes with `→` held 4 s at a time (about 85 key-downs each): with
    peaking on, 147 -> 414 -> 423 MB over two fit-view holds (settles), then
    549 -> 593 MB over two zoomed holds and 581 MB after `e` off. The same
    sequence without peaking: 262 -> 424 -> 401 -> 418 MB fit, then 511 -> 557
    MB zoomed. The zoomed growth shows without peaking too, so it is not the
    overlays (deferred below).
- The 1:1 crop is a little taller than the canvas (852 high canvas, crops of
  861-864): `decode_focus_crop` grows it to whole MCUs. It is clipped; noted
  because the `peaking` line shows it.

## Deferred issues (todo candidates)

- None for Step 1. The by-eye checks below were done by the user on
  2026-11-11 and all held (the Windows display scale was not reported; the
  automated run recorded scale 1.0 / 96 dpi). The run printed three `zoom`
  lines at `keypressToPixels` 16-42 ms with no errors.
- **Manual check (Step 1, by eye, Windows; done 2026-11-11):** the checks the
  user ran (no screenshots). Launch the release build:
  `C:\Users\daisu\.herdr\worktrees\riffle\worktree-green-cloud-634b\crates\native\target\release\riffle-native.exe D:\Photos\tests\20261110-gpui-zoom-peaking --index D:\Photos\tests\20261110-gpui-zoom-peaking\index.sqlite`
  and check, recording the window's scale factor (Windows display scale):
  1. On `ILCE-7M3_PPR10K_1283_0.ARW`, `z` shows the crop sharp (no resampling
     blur, single pixels crisp) centered on the focus point, with the scaled
     preview around it beyond the 1024 px box, and `1:1` on the status line.
  2. On `ILCE-7M4_DSC07652.ARW` (orientation 6) and
     `ILCE-7M3_sony_a7_iii_36.arw` (orientation 8), the crop is upright and
     centered on the same subject the fit view shows at the focus point.
  3. On `ILCE-7M3_PPR10K_1277_4.ARW` (focus point near the top edge), the focus
     point still sits at the window center and the image's top edge is visible
     with black above it.
  4. Holding `→` while zoomed keeps up and ends on the file the status line
     names, showing that file's crop.
  5. Dragging the window edge while zoomed re-cuts the crop to the new size
     shortly after the drag stops.
  6. `z` again returns to the fit view; the `jpeg` subfolder (open it as the
     folder argument) behaves the same.
  Expected: all hold. Basis: plan Step 1's "Manual check on the test folder";
  files `crates/native/src/zoom.rs`, `crates/native/src/main.rs`.
- **Private bytes grow while `→` is held zoomed (pre-existing, seen without
  peaking).** In the Step 2 smoke run, two 4 s zoomed holds took private bytes
  from 418 to 511 to 557 MB without peaking (549 -> 593 MB with it), while the
  fit-view holds settled near 400-425 MB. Each zoomed page reads the full JPEG
  (`reader::read_full`) and decodes a crop; whether this is allocator retention
  that plateaus or a crop or texture not released is unknown. Check with longer
  zoomed holds whether it plateaus. Basis: Step 2 smoke run; files
  `crates/native/src/zoom.rs`, `crates/native/src/main.rs` (`zoom_page`,
  `crop_arrived`).
- **Pending manual check (Step 2, by eye, Windows; the user's).** The step's
  checkbox is left open for these, the measurement and the docs; the automated
  criteria (unit tests, `mise run ci`, `mise run native:check`) pass. Launch
  the release build:
  `C:\Users\daisu\.herdr\worktrees\riffle\worktree-green-cloud-634b\crates\native\target\release\riffle-native.exe D:\Photos\tests\20261110-gpui-zoom-peaking --index D:\Photos\tests\20261110-gpui-zoom-peaking\index.sqlite`
  and check:
  1. `e` lights the in-focus edges in red on the fit view, a moment after the
     image; with `z`, on the crop too.
  2. Paging (`→` / `←`) shows each file's own edges, never another file's over
     the new image.
  3. Dragging the window edge recomputes them for the new size (no overlay of
     the old size stretched over the image).
  4. `e` again removes them at once, on the fit view and on the crop.
  5. On one ARW (`ILCE-7M3_PPR10K_1283_0.ARW`) and one RAF
     (`X-E5_DSCF1008.RAF`), the lit regions look like the Tauri app's (`e`) on
     the same file at a similar window size; note any difference in density.
  6. On the RAF, whether the overlay arriving about 50-60 ms after the image
     looks late.
  7. Private bytes settle while holding `→` with peaking on in the fit view
     (Task Manager, Details, Commit size).
  Expected: all hold. Basis: plan Step 2's "Manual check on the Step 1 test
  folder"; files `crates/native/src/peaking.rs`, `crates/native/src/main.rs`,
  `crates/native/src/zoom.rs`.
- **Pending measurement (Step 2, needs an idle PC and the user's go-ahead).**
  `powershell -ExecutionPolicy Bypass -File D:\Photos\tests\20261110-gpui-zoom-peaking-tools\measure.ps1 -Exe <release riffle-native.exe>`
  runs, per folder (ARW `--size 1616x1080`, RAF `--size 4416x2944`, from
  `D:\Photos\tests\20261108-gpui-compare\`), one throwaway run, then peaking
  off / on alternated three times and one zoomed run with peaking on. Each run
  is a 3 s settle, `e` (and `z`), then `→` held 5 s as injected key-downs
  every 33 ms, against a fresh `measure.sqlite` in the test folder, with the
  load and the compilers sampled for the port's idle gate into `runs.csv`;
  `summarize.ps1` writes `summary.csv` (p50 / p95 per stage and of the
  `peaking` line's `resize` / `compute`). Gate: the median of three p50
  `keypressToPixels` with peaking on minus off is at most +2.0 ms on ARW and
  +5.0 ms on RAF; results go in `results.md` in this folder, then the docs
  figures. Basis: plan Step 2's "Measurement".
