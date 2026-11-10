<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# The strip, the index and pass 1 of the scan in the GPUI app

## Purpose

This is milestone Step 5 of the GPUI port (`../20261109-gpui-port/plan.md`).
The native pager (`crates/native`) opens a folder and pages its previews, but
it knows nothing of the SQLite index: no thumbnails, no strip, no memory of
the last viewed file, no scan running alongside. Step 4 of the port (the
Tauri-free backend, done through `../_archived/20261109-gpui-backend-crate/plan.md`)
put `index`, `listing` and `watch` in `riffle-backend`, so the native app can
now drive pass 1 of the scan (thumbnails and metadata, `EXTRACTOR_VERSION`
re-extraction) through the same `riffle_core::scan` worker pool the Tauri app
uses, show a thumbnail strip filled from the index as the scan commits rows,
keep the strip and the preview in sync, steer the scan toward the on-screen
files (`ScanFocus`) and pause its reads around a preview read (`ReadSlots`),
as `commands::preview` does.

After this plan the native app is usable for looking through a folder: open,
strip, arrows and click, resume at the last viewed file, resync when files
change on disk. Judgments, the meta pane, filters, sort and passes 2 and 3
are the later milestones (port Steps 6-9) and are out of scope here.

### What the investigation found (2026-11-10, `main` at `aaf64dd3`)

- The native app today: `main.rs` (434 lines: `Riffle` view, `load` with the
  `guarded` catch-unwind, the `--auto` driver, the canvas with the
  `cx.defer`-after-paint stamp and the `retired` images released there),
  `pager.rs` (current index, one request in flight, stale re-request),
  `orient.rs`, `timing.rs`, `args.rs` (`[<folder>] [--auto N] [--size WxH]
  [--max N]`). It lists with its own `fs::read_dir` + `is_raw_file` + `natord`
  sort. Its only backend use is `exif::exif` on the status line.
- The backend API pass 1 needs, all `pub` and Tauri-free:
  `listing::canonicalize(&str) -> String` (the key every index row uses),
  `listing::read_listing(dir, None) -> Listing { files: Vec<String>, .. }`
  (RAW files, or JPEGs when the folder holds no RAW, sorted by plain
  `file_name` cmp, not natord), `index::stat(&Path) -> FileStat`,
  `Index::open(path)` / `Index::open_reader(path)` (writer + WAL read-only
  connection; the Tauri app reads `folder_entries` and `thumbnail` through the
  reader so they never wait behind a scan's write transaction),
  `Index::reconcile(dir, &[FileStat]) -> (todo, removed)` (also upserts
  `folders.opened_at`; **drops every row under `dir` not in the given list**),
  `index::run_scan(&Mutex<Index>, dir, &todo, threads, &ScanFocus, &AtomicBool,
  PROGRESS_INTERVAL, |done, total, ready: Vec<String>|)` (blocking; the
  callback runs on rayon workers and must not panic), `Index::entries(dir) ->
  Vec<IndexedFile>` (`path`, `orientation`, `capture_time`, `has_thumb`,
  `error`, ...), `Index::thumbnail(path) -> (orientation, jpeg)` (an `Err`
  containing `no cached thumbnail` is a processed-and-failed file; any other
  `Err` is "not scanned yet"), `Index::last_viewed` / `set_last_viewed`,
  `Index::mark_viewed` / `viewed_files`, `Index::evict(now, EVICT_POLICY)`
  (`VACUUM`; the Tauri app runs it once at launch on a thread when no scan is
  running).
- The scan core: `scan::ReadSlots::new(cap)` (Tauri's `READ_SLOTS = 2`),
  `.pause() -> Paused` (RAII; `commands::preview` holds it around
  `read_preview`), `ScanFocus::with_slots(slots.clone())`, `ScanFocus::set(Vec<String>)`
  (the hot list: current file first, then the strip's range nearest-first,
  `scanfocus.ts`'s `scanFocusPaths`, sent 100 ms after the last change,
  `SCAN_FOCUS_DELAY`). Workers run at `Priority::BelowNormal`; threads =
  cores − 2, min 1 (`scan_threads`). Thumbnails are unrotated 404-px JPEGs at
  quality 80 (~19 KB); the caller applies `orientation`.
- The Tauri open flow (`openDirectory` / `startScan` in `main.ts`): list,
  read `last_viewed` and `viewed_files`, show the resume target, then
  `scan_folder` (canonicalize, `watch::set`, cancel + join the previous scan,
  list, stat, `reconcile`) and `start_scan` (spawn `run_scan`, then
  `sendScanFocus()`), `scan-progress` → `strip.markReady(ready)` and
  `scan-done` → `strip.refresh()` + `folder_entries`. A folder-changed event
  → `resync`: deferred while a scan runs, else re-list, carry the current file
  and the scroll offset, `startScan` again. `set_last_viewed` writes are
  coalesced (one in flight, latest wins, `resume.ts`); `mark_viewed` fires
  after a decoded preview stayed on screen `VIEW_DWELL_MS` = 500 ms and is
  batched per folder (`viewed.ts`).
- `strip.ts`: a hand-rolled virtual list, fixed cell pitch (`--cell-width`,
  154 px; 144x96 image box + name), `RANGE_MARGIN = 4` cells beyond the
  visible range, `MAX_IN_FLIGHT = 4` thumbnail requests, nearest-to-center
  first (`pickNext`), `missing` / `ready` / `failed` sets, cells outside the
  range released (their bytes dropped) and re-requested when they return,
  `setCurrent` scrolls `block: nearest`, a vertical wheel scrolls sideways,
  `carriedOffset` keeps the view on the same files across a re-list.
- GPUI at the pinned rev: `img(Arc<RenderImage>)` exists (`ImageSource::Render`)
  and `canvas` + `window.paint_image` is what the pager uses; `uniform_list`
  is vertical-first (horizontal only through
  `ListHorizontalSizingBehavior::Unconstrained`); `div` has `on_scroll_wheel`,
  `on_mouse_down`, `on_click`; `ScrollHandle` has `offset` / `set_offset`.
  Every `RenderImage` handed to the renderer must be released with
  `cx.drop_image` after the frame that stopped using it has presented.
- The native lock already holds `futures`, `smol`, `flume`, `async-channel`,
  `dirs 6.0.0`, `natord`; not `env_logger`. The backend's `log::info!` lines
  (`scan extract: ...`) go nowhere in the native app today: no logger is set.
- Tauri's index lives at `app_cache_dir()/index.sqlite`, i.e.
  `<dirs::cache_dir()>/com.minodisk.riffle/index.sqlite` (`%LOCALAPPDATA%` on
  Windows, `~/Library/Caches` on macOS, `~/.cache` on Linux).
- The port plan's Step 4 line still links `../20261109-gpui-backend-crate/plan.md`,
  which the archive move turned into `../_archived/20261109-gpui-backend-crate/plan.md`.

### Rules every step follows

- `crates/native` must not depend on `crates/app`. The backend stays free of
  `tauri`, event names and `AppHandle`; this plan expects **no backend
  change** at all (if one turns out necessary, it takes a callback and is
  checked with both `mise run ci` and `mise run native:check`).
- No Tauri smoke checks and no shims for Tauri behaviour (the user's
  decision for the port; a revert is the fallback). Root `mise run ci` must
  still pass.
- Build with `C:\Users\daisu\AppData\Local\bin\NASM` on `PATH`; `mise run
  native:check` before every PR, and `cargo build` before launching the exe
  (`native:check` does not produce it).
- Blocking work (`run_scan`, SQLite reads and writes, eviction) runs on
  `std::thread` or `cx.background_spawn`; the preview `load` keeps its
  background task and now holds `ReadSlots::pause()` around `read_preview`
  only (not around the decode, as Tauri does not either).
- Every `RenderImage` the strip stops showing goes through the existing
  `retired` list and is dropped in the `cx.defer` after paint, never earlier.
- Manual checks by the user on the GPUI app are acceptable Done-when lines;
  their result goes in `learnings.md`.
- Docs move with the code: each step updates `docs/agents/native-app.md`
  with what it learned and `CLAUDE.md`'s `crates/native` sentence names the
  new modules.

## Steps

- [x] Step 1: Tick the port's Step 4, open the index and run pass 1 from the native app, progress on the status line
  - Done when:
    - `../20261109-gpui-port/plan.md`: Step 4's checkbox is ticked, its
      `../20261109-gpui-backend-crate/plan.md` link points at
      `../_archived/20261109-gpui-backend-crate/plan.md`, and Progress gains
      `(2026-11-09) Step 4 complete, through ../_archived/20261109-gpui-backend-crate/plan.md`
      (the path in backticks or as a link; `mise run lint`'s lychee passes).
    - `crates/native/src/datadir.rs`: the index path. Default
      `dirs::cache_dir()/com.minodisk.riffle.native/index.sqlite` (the user
      chose the native app's own file on 2026-11-10; see Trade-offs),
      overridable by a new `--index <path>` flag in `args.rs` (tested there).
      An unopenable index degrades to "no index" (the pager still works, the
      status line says `no index: …`), as the Tauri app's `AppIndex(None)`
      does.
    - `crates/native/src/logger.rs`: a `log::Log` implementation writing
      `[target][LEVEL] message` to stderr, installed in `main()` at `Info`,
      so the backend's `scan list` / `scan extract` lines and `log::warn!`s
      are visible. (No `env_logger`: not in the lock, and `Riffle.log` is
      port Step 9's.)
    - `crates/native/src/scan.rs`: the Tauri-free orchestration of one
      folder's pass 1. `Scan::prepare(index, dir)` canonicalizes, calls
      `read_listing(dir, None)`, stats, `reconcile`s and returns the todo
      list and the full listing; `Scan::start(...)` spawns one
      `std::thread` running `index::run_scan` with `scan_threads()` (cores −
      2, min 1, same rule as Tauri), `Priority` as `run_scan` sets it, the
      shared `ReadSlots::new(READ_SLOTS = 2)` through
      `ScanFocus::with_slots`, a cancel flag and a join handle; a new open
      cancels and joins the previous scan first. Progress and done reach the
      view through one `futures::channel::mpsc::unbounded` drained by a
      `cx.spawn` task (`this.update(...)` per message); the `run_scan`
      callback only sends. The `ScanFocus` is set to the current file alone
      for now (Step 3 adds the strip's range). The eviction
      (`Index::evict(now_secs(), EVICT_POLICY)`) runs once at startup on a
      thread, before the first scan takes the writer lock (the scan waits
      behind it, as Tauri's does).
    - `main.rs`: `load` holds `slots.pause()` around `read_preview` only. The
      listing the pager and strip use is `read_listing`'s files in `natord`
      file-name order, filtered by `--size` / `--max` as before; the index
      always reconciles the **full** listing, so a measurement run never drops
      rows. The status line appends `scanning done/total` while pass 1 runs
      and `N failed` after it when `errors > 0`.
    - Unit tests: `datadir` (flag beats default), `args` (`--index`), and
      `scan.rs`'s pure parts (the ordering of listing → natord; `scan_threads`
      bounds). The scan itself is exercised by hand: open
      `D:\Photos\tests\20261108-gpui-compare\ARW` twice; the first run shows
      the progress climb to 20/20 and the stderr `scan extract: ... files=20`,
      the second shows `files=0` (rows reused). A corrupt `.ARW` counts as
      `1 failed` and paging continues.
    - `mise run ci` and `mise run native:check` pass; `crates/app` and
      `crates/backend` have no diff; `crates/native/Cargo.lock` changes only
      by `cargo` (new direct deps `dirs`, `futures`), listed in the PR.
      `docs/agents/native-app.md` gains: the channel pattern for callbacks
      from non-GPUI threads, the data dir decision and why, the std::thread
      rule for `run_scan`.
  - Implementation approach:
    - Keep `scan.rs` minimal: one scan at a time, no `pending` /
      `preparing` / embed-pass states (those exist in Tauri for
      `clear_index`, Move to Trash and pass 3, all later milestones).
    - `reconcile` runs under the writer mutex; do it on
      `cx.background_spawn` and keep the main thread free. Verify that the
      first scan waits for the eviction instead of racing it (both take the
      writer `Mutex`).
    - Measure nothing here beyond a sanity `page` line: Step 5 owns the gate.
- [x] Step 2: The thumbnail strip, in sync with the preview
  - Done when:
    - `crates/native/src/strip.rs` is a pure model with unit tests: cell
      pitch (154 px, 144x96 image box + a one-line name, as the Tauri strip),
      scroll offset clamped to the content width, `visible_range(width) ->
      (first, last)` with `RANGE_MARGIN = 4`, `scroll_into_view(index,
      width)` with the `block: nearest` rule, `hit_test(x) -> Option<index>`,
      `next_to_request(center)` (nearest the viewport center first, skipping
      requested / in-flight / missing / failed), and the `missing` / `ready` /
      `failed` bookkeeping of `strip.ts` (`mark_ready(paths)` clears
      `missing`; a `no cached thumbnail` error is `failed`; any other error is
      `missing` unless the path was `ready`).
    - `crates/native/src/thumbs.rs`: the thumbnail cache. At most
      `MAX_IN_FLIGHT = 4` reads of `Index::thumbnail` through the **reader**
      connection on `cx.background_spawn`, decoded with mozjpeg to BGRA,
      rotated with `orient::rotate_bgra`, held as `Arc<RenderImage>` per
      index; cells leaving the range plus margin are released into the view's
      `retired` list and re-requested when they return. A folder switch bumps
      a generation and drops late results.
    - The window gains the strip along the bottom edge, drawn by a `canvas`
      (same pattern as the preview): the on-screen cells' thumbnails fit in
      their boxes, a gray placeholder for a cell without one, the error text
      of a failed cell, the file name under each, the current cell outlined.
      A click selects (the pager's index moves and the preview follows);
      arrows, Home and End move the pager **and** scroll the strip into view;
      a wheel over the strip scrolls it sideways (vertical delta included,
      as the Tauri strip). The preview and the strip never disagree about the
      current file.
    - As `scan-progress`'s `ready` paths arrive (Step 1's channel) the
      strip requests exactly those cells; on scan done it clears `missing`
      and re-pumps. Thumbnails show up while the scan runs, nearest the view
      first.
    - Preview latency is not touched by the strip's work: the strip's
      decodes stay on the background executor at 4 in flight, the canvas
      paints only visible cells, and the `page` line's `paint` on the ARW
      compare folder stays within 1 ms of Step 3's 0.72 ms in a quick
      `--auto 20` run (recorded in `learnings.md`; the gate is Step 5's).
    - Manual check: open a 1000+ file folder under `D:\Photos\`; the strip
      fills from the current file outward while the scan runs, holding `→`
      keeps the strip and preview in step, clicking a far cell shows it, and
      memory does not climb while scrolling the strip back and forth
      (thumbnails outside the range are dropped: check with Task Manager that
      private bytes settle).
    - `mise run ci` and `mise run native:check` pass. `CLAUDE.md`'s
      `crates/native` sentence names `strip.rs` and `thumbs.rs`;
      `native-app.md` records the canvas-vs-`uniform_list` decision and any
      GPUI fact met (hit testing on a canvas, wheel event units on Windows).
  - Implementation approach:
    - Assumes Step 1 merged. Canvas drawing with a pure model was chosen
      over `uniform_list` and over a flex `div` of N children (see
      Trade-offs); if the canvas cannot receive wheel / mouse events
      directly, wrap it in a `div().on_scroll_wheel(...).on_mouse_down(...)`
      and map the event position against the canvas bounds.
    - Reuse `pager.rs` for the current index; `strip.rs` only follows it.
      Do not add selection beyond the current file (multi-select is a
      judgment feature, Step 6+).
    - Thumbnail orientation: 6 / 8 turn the 404x270 box into 270x404; fit
      inside the 144x96 box as Tauri's `object-fit: contain` does.
- [x] Step 3: Steer the scan from the strip, resume at the last viewed file, record the viewed files
  - Done when:
    - `crates/native/src/focus.rs`: a port of `scanFocusPaths(files, current,
      first, last)` with the same unit cases (current first, then alternating
      nearest neighbours within the range, earlier side first; the current
      file leads even when outside the range). The view sends it to
      `ScanFocus::set` 100 ms after the last change of the current file or
      the strip's range (`SCAN_FOCUS_DELAY`, a `cx.spawn` timer that is
      replaced on each change), and once right after the scan starts, as the
      Tauri frontend does after `start_scan`.
    - Opening a folder reads `last_viewed(dir)` and starts at that file when
      it is still listed, else the first; `set_last_viewed` is written
      through a coalescing writer (one write in flight on
      `cx.background_spawn`, the latest value wins; `viewed.rs` holds it as
      a pure state machine with tests mirroring `lastViewedWriter`).
    - `mark_viewed(dir, [path])` is written after a decoded preview has
      stayed on screen `VIEW_DWELL_MS` = 500 ms (a dwell timer restarted on
      every page), batched per folder like `viewedWriter`; the dwell and the
      batching are pure, tested parts of `viewed.rs`.
    - Manual check: open the compare folder, page to the 7th file, quit,
      reopen: it opens at the 7th. With the Step 1 stderr log, a long folder
      opened mid-strip shows the `scan extract` of the files around the
      current one landing first (thumbnails near the cursor appear before the
      far ones).
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `focus.rs` and `viewed.rs`.
  - Implementation approach:
    - Assumes Step 2 merged. All index writes go through the writer
      `Arc<Mutex<Index>>`, never the reader. A write that fails is logged and
      dropped (Tauri logs a warning), never shown.
    - `ScanFocus::set` only takes the focus lock, so it can be called from
      the main thread directly; no channel needed in that direction.
- [ ] Step 4: Resync the open folder from the folder watcher
  - Done when:
    - `main.rs` creates one `riffle_backend::watch::Watch::spawn(on_changed)`
      whose callback sends the folder string into the Step 1 channel; opening
      a folder calls `watch::set(&watch, &canonical, &dir)`.
    - A change event triggers a resync: while a scan runs it is deferred to
      the scan's end (one pending resync at most); otherwise the folder is
      re-listed, the current file is carried over by path (the nearest
      listed neighbour when it is gone, the first when none), the strip's
      scroll keeps the same files in view (a port of `carriedOffset`'s rule
      as a tested pure function in `strip.rs`), loaded thumbnails of files
      still listed are kept, and a new scan is prepared and started
      (`reconcile` drops the rows of files that are gone or changed).
    - `crates/native/src/resync.rs` (or a section of `scan.rs`) holds the
      deferred / in-flight / pending states as a pure, tested state machine.
    - Manual check (Windows): copy a RAW into the open folder from
      Explorer; its cell appears within about a second and gets a thumbnail;
      delete another one; its cell disappears and the current file stays
      current (or moves to the neighbour when it was the deleted one).
    - `mise run ci` and `mise run native:check` pass; `native-app.md`
      records whether `notify` 8.2.2 behaved as 8.2.0 in `watch::set` (the
      `configure` barrier of `settle` matters only for the rename path, port
      Step 8, so only a note that it is still unverified unless the
      implementation hit it).
  - Implementation approach:
    - Assumes Step 3 merged. The backend's `watch.rs` tests cover
      `triggers` / `run`; the native side adds no watcher logic, only the
      resync flow.
    - `watch::set` on the same canonical dir is a no-op, so calling it on
      every open is safe.
- [ ] Step 5: Measure the pager with the strip against the pre-strip pager and record the verdict
  - Done when:
    - `results.md` in this folder holds a same-session A/B as the port's
      Step 3 re-measurement: **A** = this plan's `riffle-native` release
      build (Steps 1-4 merged), **B** = the pre-strip pager built in release
      from `fb7fcdf4` (the last commit before this plan) into a scratch
      directory, both run directly (no `mise` / `cargo` in front), on
      `D:\Photos\tests\20261108-gpui-compare\ARW` (`--size 1616x1080`) and
      `...\RAF` (`--size 4416x2944`), `--auto 60`, order per folder: one
      throwaway warm run of A, then A B A B A B, with the idle gate and the
      conditions block of the Step 3 results (build, drive, display, GPU,
      SIMD on, running processes). For A the index is warm (the folder's
      pass 1 finished before the timed presses; a first untimed run of A on
      each folder makes it so).
    - **Gate:** the median of the three runs' p50 `keypressToPixels` of A
      minus B is at most +2.0 ms on ARW and +5.0 ms on RAF. p95 and the
      per-stage medians (`read`, `decode`, `orient`, `hop`, `frameWait`,
      `paint`, `present`) are recorded and discussed, not gated.
    - **During-scan figure, recorded not gated:** on a copy of a 500+ file
      ARW folder under `D:\Photos\tests\` with its index rows cleared
      (`--index` pointing at a fresh file), one `--auto 60` run of A started
      so the presses fall inside pass 1; the `read` p50 / p95 and
      `keypressToPixels` p50 are written next to Tauri's `READ_SLOTS` figures
      (median 14-16 ms, p95 18-19 ms preview read wait at cap 2 with the
      pause, SATA SSD), and whether the pause measurably shortened `read`
      against one run with the pause commented out (that build is not
      committed).
    - If the gate is missed, `results.md` names the stage that grew and
      Step 2 (strip paint / thumbnail decodes) is reopened with a fix PR
      before this plan closes.
    - `docs/humans/performance.md` "Native preview (GPUI)" gains one line
      per folder for the pager with the strip (and a pointer to this
      `results.md`), `performance.ja.md` the same in Japanese.
      `native-app.md` records what the measurement taught. `mise run ci`
      passes.
  - Implementation approach:
    - Assumes Steps 1-4 merged. Reuse the Step 3 measurement script's
      approach (an ad-hoc script in the scratch directory, not committed;
      the idle sampling has to retry an empty `Win32_Processor.LoadPercentage`,
      per the port's learnings). The user's only action is to leave the
      machine idle and close the Tauri app.
    - B is built with `cargo build --release` from a `git worktree` at
      `fb7fcdf4` under the 1.98.1 toolchain with NASM on `PATH`; record its
      `build.rs` SIMD check passing.

## Trade-offs and risks

- **Index data dir: own file (decided 2026-11-10) vs sharing the Tauri app's.**
  Premises: the Tauri app opens
  `<cache_dir>/com.minodisk.riffle/index.sqlite` with a writer and a reader,
  runs `evict` with `VACUUM` at every launch when no scan of its own runs,
  and its `reconcile` deletes every row under a folder that the listing
  given to it does not contain; the backend plan's debug runs were held back
  because a debug build shared the user's real data. Sharing would give the
  native app warm thumbnails, `last_viewed` and `views` for every folder the
  user already culled, and one cache on disk; its costs are two processes on
  one SQLite file (a native scan writing while Tauri's launch `VACUUM` runs,
  or the reverse; WAL serializes it but a `VACUUM` can take seconds and a
  busy timeout of 300 ms on the reader), two `EXTRACTOR_VERSION`s that drift
  if one app is rebuilt and the other not (each open re-extracts the other's
  rows), and every manual check in this plan touching the real index. The
  own file (`com.minodisk.riffle.native`) costs a cold pass 1 on the first
  native open of each folder (~7 ms/file, 2000 files ≈ 15 s, thumbnails
  appearing meanwhile) and loses the Tauri `last_viewed`; port Step 10 can
  move or merge it through `migrate.rs`'s mechanism. The `--index <path>`
  flag exists either way for measurements. **The user chose the own file.**
- **Scan orchestration in the native crate, not the backend.** Tauri's
  `Scans` / `ScansState` is bound to `tauri::async_runtime` and carries the
  `clear_index`, pending, preparing and embed-pass concerns of later
  milestones; extracting it would be another Tauri-side refactor whose
  compatibility the user said not to weigh. The native `scan.rs` is small
  and grows with Steps 7-9; when it starts to mirror Tauri's, that is the
  time to lift the common part into the backend behind callbacks.
- **Canvas strip vs `uniform_list` vs a flex `div`.** `uniform_list` is
  vertical-first and its horizontal mode (`Unconstrained`) is about item
  width, not a horizontal virtual list; a flex `div` with one child per file
  lays out every cell each frame (thousands of files); the canvas paints only
  the visible cells, gives `visible_range` to `ScanFocus` directly and
  matches how the preview is drawn. Its cost is hand-rolled hit testing and
  scrolling, which `strip.ts` also hand-rolls, as pure, tested code.
- **Callbacks from non-GPUI threads.** `run_scan`'s progress and the
  watcher's `on_changed` fire on rayon and watcher threads; `AsyncApp` is not
  `Send`, so the plan uses one async channel drained by a foreground task.
  A `BackgroundExecutor` is `Send`, but its tasks still cannot update an
  entity. If the channel pattern shows latency in the `hop` stage, the
  alternative is `cx.background_executor()`-side batching of progress
  (progress is already at most every 100 ms).
- **`run_scan` on a `std::thread`.** GPUI's background executor pool is what
  the preview decode runs on; a blocking `run_scan` there for a minute would
  take one of its threads. The rayon pool is inside `run_scan` anyway.
- **`--size` / `--max` and `reconcile`.** `reconcile` drops the rows not in
  its list, so the filtered measurement listing must never be what the index
  sees; the plan reconciles the full listing and filters only the shown list.
  The hot list and the strip use the shown list.
- **Order of the listing.** `read_listing` sorts by plain `file_name` cmp,
  the pager by `natord`; the strip keeps `natord` (the Tauri strip's default
  `compareStrings`), and the index's own `entries` order does not matter
  here (Step 2 keys everything by path).
- **The gate's noise.** Step 3 showed `frameWait` moving 6-11 ms between
  runs of the same files; the same-session A/B absorbs that but n=60 still
  cannot gate p95, so p95 is recorded only, as before. A B built today from
  `fb7fcdf4` is not the Step 3 binary: its numbers are compared within the
  session only.
- **Memory of the thumbnail cache.** A decoded 404x270 BGRA thumbnail is
  436 KB; the range plus margin at a 1600 px window is about 20 cells, so
  under 10 MB on the GPU, released through `retired` as the pager's previews
  are. A leak would show as climbing private bytes in Step 2's manual check.
- **No aliasing follow-up.** Port Step 3 raised none (no moiré in three
  native frames); the side-by-side with the Tauri app stays a pending manual
  check in the port plan's `learnings.md` and is not this plan's work.
- **The strip's cells have no judgment marks, badges or rename.** Rating,
  flag, label, burst band, focus tier, recommended icon and the inline
  rename are Steps 6-8 of the port; Step 2 draws the thumbnail, the name,
  the placeholder, the failure text and the current outline only.

## Progress

- (2026-11-10) Step 1 complete
- (2026-11-10) Step 2 complete
- (2026-11-10) Step 3 complete
