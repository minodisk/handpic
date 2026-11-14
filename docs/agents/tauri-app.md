# Working on `crates/app`

Read this before touching Tauri commands, `crates/app/tauri.conf.json`, or the
`crates/app/ui/` frontend. It lists the pitfalls this repository has already
hit, each with the reason it happens.

Each item is tagged:

- **Hit**: actually broke something here.
- **Measured**: measured in this repo and steered a design decision; nothing
  broke, but the naive choice would have missed a stated budget or cost more.
- **Inferred**: taken from the sources or docs; nothing has broken on it yet.

Source: `docs/plans/_archived/20260917-tauri-skeleton/learnings.md` and the fix
in #11.

## Rust side

### Synchronous commands run on the main thread (Hit)

A `#[tauri::command]` without `async` runs **inline on the main thread**.

- Why: tauri-macros routes non-`async` commands through `body_blocking`, which
  calls the function directly from the IPC handler instead of spawning it.
- What broke: `pick_folder` called `blocking_pick_folder`, which parks the
  calling thread on a `sync_channel(0)` `recv`. The main thread stopped pumping
  the run loop, so the native dialog appeared but its buttons did nothing and
  the app hung.
- This passed `mise run ci`, the type check, and local review.
  It surfaced the first time the user launched the app. An earlier learnings
  note claimed sync commands run off the main thread; that note was wrong.

Rules:

- Anything that waits (dialogs, channels, locks held by the UI) goes in an
  `async` command. For callback APIs, await a
  `tauri::async_runtime::channel(1)`; see `pick_folder` in
  `crates/app/src/commands.rs`.
- An `async` command doing blocking IO wraps it in
  `tauri::async_runtime::spawn_blocking`, as `preview` does. Why: blocking
  inside the future stalls an async runtime worker instead.
- Confirmed again in Phase 3, with a related trap: HTML5 drag-and-drop never
  reaches the frontend. Tauri intercepts it, so `dataTransfer.files` carries no
  usable path and paths arrive only through the `tauri://drag-drop` (and
  `drag-enter` / `drag-over` / `drag-leave`) webview events, listened to via
  `event:listen` under `core:event`'s default `allow-listen`. Those events and
  `window.__TAURI__` are main-thread only; neither exists inside a worker.
- Exception: a synchronous command that only writes a small settings file
  (store set + save on the calling thread, as `update_keymap` and
  `set_auto_advance` do) does not need `spawn_blocking` — the write is a
  tiny JSON file, so the blocking cost is negligible. Reach for
  `spawn_blocking` when the IO is unbounded or can block on something other
  than a small local file.
- Making a fire-and-forget command off-main is only a signature change: an
  `async fn` with no return type and owned arguments (`String`, not `&str`)
  compiles as is, as `log_timing` does. Borrowed arguments (`&str`, `State`)
  in an `async` command force a `Result` return, so take owned values when
  the command only forwards a line. This applies only when the body does not
  block; blocking IO still needs `spawn_blocking` (see above). Source:
  `docs/plans/_archived/20261006-log-timing-off-main-thread/learnings.md`,
  Step 1.
- Hit again on the folder-open path: `list_arw` was a synchronous command
  that `stat`ed every entry (`path.is_file()`). With a scan reading the same
  drive, `open list` took 4.6-7.4 s for a few hundred RAWs on Windows, the
  main thread was blocked for all of it, and every later click queued behind
  it. `list_arw`, `remember_folder` and `start_scan` are now `async` commands
  wrapping their work in `spawn_blocking`, and the listing uses
  `DirEntry::file_type()` (free from the listing on Windows) with a
  `metadata()` fallback only for symlinks. Note that
  `#[tauri::command(async)]` on a non-`async` fn is not `spawn_blocking`:
  tauri-macros' `body_async` calls the fn inside the future, so it runs on
  an async runtime worker and stalls it. Source:
  `docs/plans/20260925-folder-open-off-main-thread/`.

### An index/scan mutation that must not race a rescan holds the `Scans` lock across the disk op and the index write (Hit)

`rename.rs` holds `Scans`' inner mutex across the rename on disk and the
subsequent index write, the same pattern `trash_files` uses. Do the same
for any new command that both changes what is on disk and writes rows for
it, so a scan cannot start mid-way and index the half-renamed state.

A blocking-IO step inside such a command (canonicalizing/stat-ing paths)
still goes in `spawn_blocking`, as the "Synchronous commands run on the main
thread" entry above requires, but do the refusal checks before anything on
disk changes and place the blocking step first, before any flush — a
pending flush can itself create the file the refusal needs to see.

- Source: `docs/plans/_archived/20260928-rename-from-tree-and-strip/learnings.md`, Steps 1 and 3.

`trash_files`, `delete_files`, `trash_rejected_undo` and `trash_rejected_redo`
hold the lock across the whole operation, even a move of minutes. Releasing it between files would let a rescan index a half-moved
folder and race the operation's own reconcile.

- A `scan_folder` issued meanwhile waits, which is acceptable because the
  status shows the move (`moving to Trash N / M`). An undo shows nothing yet,
  as `trash::restore` has no progress callback, so a reopen during it waits
  silently until that follow-up lands. The wait must not sit on an
  async runtime worker, so `scan_folder`'s first block (mint the id, take
  `running`, start `Preparing`) runs in `spawn_blocking` and hands `Preparing`
  back from it.
- Its later locks, the `pending` insert and `Preparing`'s drop, stay on the
  worker: they run with `preparing > 0`, so `scanning()` makes every long
  holder refuse. Only `spawn_eviction`, which checks `running` alone, can
  still make them wait, for its `VACUUM`.
- `cancel_scan` waiting on the lock is harmless. While such an operation holds
  it no scan runs short of its third pass, so by the time `cancel_scan` gets
  the lock its id is stale or in that pass, and `cancel` ignores both.
  `set_scan_focus` keeps dropping its calls (`try_lock`).
- Source: `docs/plans/20261106-trash-over-scan/plan.md`, Step 3.

### Undoing a trash run after a folder rename, and walking the tree without following links (Hit)

- `Trashed.path` stays the original path because the Windows / Linux Trash is
  looked up by it (`trash_key`). The renamed destination lives in
  `restore_to`, which `Runs::rename_dir` rebases with the same
  `old + MAIN_SEPARATOR` prefix rule as `Index::rename_dir`. `rename_folder`
  calls it under the same `Scans` lock, outside `if let Some(index)`, so the runs
  follow even with no index.
- trash 5.2.9's Windows `restore_all` fails with "file not found" when the
  original folder is gone. Only the freedesktop backend runs `create_dir_all`.
  For a file with `restore_to`, create the missing old ancestors first
  (`trash::create_missing`, returning exactly the ones created), then
  `restore_all`, then `fs::rename` to the new place. Afterwards `remove_dir` only
  the created folders, deepest first and only while empty, so a pre-existing
  empty old-name folder is never removed. Verified with a probe.
- Folder-tree `expandAll` lists a symlink / junction child (`is_link` from
  `list_subfolders`) without descending. `list` follows links, so `a/loop -> a`
  would otherwise grow without end. The walk also stops a branch as soon as any
  ancestor was collapsed, dropped or re-keyed by a rename.
- Source: `docs/plans/_archived/20260930-todo-five-more-items/learnings.md`,
  Steps 3 and 5.

### Moving files between folders (split by date): what the index, watchers and trash runs do (Hit)

- `Index::forget_dir` is not `evict_folder`. Eviction keeps dirty `ratings`
  rows and leaves `views`, so an unwritten judgment of a folder still on disk
  is not lost. `forget_dir` is for a folder removed from disk. It drops every
  row of exactly that `dir` (`files`, `ratings` including dirty ones,
  `embeddings`, `views`, `folders`), and touches neither subfolders nor prefix
  siblings such as `photos2`.
- `Index::move_file` leaves the old folder's `folders.last_viewed` naming a
  file now elsewhere. That is harmless, because `resume.ts` only takes it
  when the listing holds it.
- A split that removes the original folder must release the open folder's
  watcher and the tree watches first (`with_released`), or Windows returns
  `PermissionDenied`. `with_released` restores the tree watches only on
  `Err`, so after a split that keeps the original folder, put the open
  folder's watcher back with `watch::set`. The tree watches come back with
  the tree re-render after `folders.refresh`, since `set_tree_watches` makes
  the watched set equal to what the frontend sends.
- Create each target folder with `std::fs::create_dir`, not `create_dir_all`.
  It fails when the name appeared after the re-plan, so every file of that
  group fails and nothing is ever merged. A split target named like the
  source folder is a collision, because a split merges nothing.
- A move does not rebase `Runs` or the frontend's undo history. An undo of a
  trash run recorded before the split restores to the old path (which may be
  gone), and judgment undo entries still name the old paths.
- Source: `docs/plans/_archived/20261104-rename-folder-by-capture-date/learnings.md`, Steps 1-3.

### Resolve shortcut overrides order-independently, not in a single pass (Hit)

`from_overrides` in `crates/backend/src/shortcuts.rs` checks each stored override
against the keymap built so far. A single pass in action order silently drops
a valid override that wants a key another override in the same batch frees
later: `{"pick": ["q"], "reject": ["p"]}` left `reject` on its default,
because `reject` comes before `pick`, so `p` still looked taken.

- Fix: apply the non-conflicting overrides first, then retry the conflicting
  ones until no more apply.
- When two overrides genuinely want the same key, the first in action order
  keeps it; only after the retries settle does the later override apply its
  other keys without it (skipped only if no key is left), and the retries run
  again, as that replaces the action's keys and may free one. Applying a
  partial override can itself free a key an earlier partial override
  dropped, so once the outer retry settles, recompute every partly applied
  action's keys as its originally requested keys filtered by what is free
  at that point (never a conflict, since only free keys are added).
- Adding a new default key (`DEFAULTS`) wins over any stored override that
  holds the same key. `from_overrides` drops that key from the user's action
  silently on upgrade. In tests, a stored override that uses an arbitrary key
  (for example `a_stale_default_in_an_old_store_drops_only_that_key` used `d`)
  fails once a default takes that key, so use a key no default holds. A new
  action also needs a `MENU_LESS` entry in `main.rs`'s menu test if it has
  no menu item, not only a `DEFAULTS` entry and the default-keys test. A
  main-view key that the folder tree swallows is absent from `TREE_PASSTHROUGH`.
- Source: `docs/plans/_archived/20260920-pick-shortcut-editable/learnings.md`,
  Step 1; `docs/plans/20260929-shortcut-override-partial-conflict/learnings.md`,
  Step 1; `docs/plans/_archived/20261011-performance-graph-overlay/learnings.md`,
  Step 3.

### An accelerator string is not validated until Tauri parses it (Inferred)

`accelerator()` in `crates/backend/src/shortcuts.rs` is a plain table; muda is
not a dependency of `crates/app`, so nothing checks the string at compile
time. Tauri parses it with `.parse().ok()`, so an unrecognized name
silently becomes "no accelerator" instead of an error — a typo here fails
silently, not loudly.

A key override with only `shift` (or no modifier at all) also converts to
`None` on purpose: e.g. a plain `o` override leaves the corresponding menu
item without an accelerator rather than erroring.

- When adding or changing an accelerator mapping, don't rely on a build or
  test failure to catch a bad string or a modifier-less override; check the
  menu item's displayed accelerator by hand.
- Source: `docs/plans/_archived/20260920-menu-accelerators/learnings.md`,
  Step 1 (unverified on a real device; see "GUI automation does not work on
  this Mac").

### Rebinding `selectAll` away from Cmd+A breaks native text-input select-all on macOS (Hit)

macOS WKWebView only runs a text input's native Select All through the Edit
menu's key equivalent. Once the predefined `Select All` menu item is replaced
by a custom one bound to the `selectAll` keymap action (see "App items go
into the default menu's own submenus"), rebinding that action away from
Cmd+A leaves Cmd+A doing nothing in a focused text input, because no menu
item claims that key equivalent anymore.

- Fix: handle `meta+a` directly in the app's own `keydown` handler for text
  inputs / textareas (the `native` decision in `crates/app/ui/src/settings.ts`),
  independent of the current `selectAll` binding.
- This applies to any future action whose menu item is user-rebindable and
  whose default key doubles as a macOS-native text-editing shortcut.
- Source: `docs/plans/_archived/20260926-strip-select-all/learnings.md`,
  Step 1.

### Volume listing: dedupe home by canonical path, plus a macOS-only ancestor rule (Hit)

`volumes()` in `crates/app/src/folders.rs` drops the home volume's duplicate
by canonical-path equality, and, on macOS only, also drops "a volume whose
canonical path is an ancestor of home": `/Volumes/Macintosh HD` resolves to
`/`, which never equals the home directory itself.

- This rule is macOS-specific: on Windows, home is `C:\Users\<name>`, whose
  ancestor `C:\` is a real, independently browsable root (unlike macOS's
  volume alias for `/`), so applying the same ancestor rule there would
  silently drop it from the tree. Don't port the ancestor check to Windows.
- Consequence: when home itself lives on an external volume on macOS, that
  volume is not listed separately.
- On Linux, `/mnt` lists every child directory as a root, unfiltered, since
  plain Linux users mount arbitrary names there. On WSL (`WSL_DISTRO_NAME`
  set) only the single-letter drive mounts like `/mnt/c` are kept, dropping
  WSL's own `/mnt/wsl` and `/mnt/wslg` (`wsl_drive_mounts`).
- `wsl_drive_mounts` is compiled under
  `cfg(any(test, not(any(target_os = "macos", target_os = "windows"))))`,
  not plain `cfg(test)`: its test runs on every platform while the function
  stays out of macOS / Windows builds without a dead-code warning. Reuse this
  shape for other Linux-only helpers that need tests on non-Linux CI.
- Windows also skips entries with `FILE_ATTRIBUTE_HIDDEN` (a `MetadataExt`
  one-liner) in addition to dot-names. `cargo check --target
  x86_64-pc-windows-gnu` in `crates/app` verifies `cfg(windows)` code
  without a Windows machine, when that target is installed.
- Source: `docs/plans/_archived/20260925-lightroom-layout/learnings.md`, Step 2.

### Measure before choosing a JPEG payload over raw pixels (Measured)

`mozjpeg::Compress`'s defaults turn on trellis quantization and optimized
Huffman tables, so re-encoding a crop costs more than the partial decode that
produced it: 49ms at q85 for a 1037x1024 crop, against 21.5ms to decode it.

- Why: the defaults optimize for file size, and both passes run over every MCU.
- Phase 5's `focus_crop` therefore returns raw RGBA (4.2MB at the 1024 cap)
  rather than a ~180KB JPEG. That trades IPC bytes for CPU; take the
  measurement before trading back, and measure with the optimizations off, not
  with the defaults.
- Confirmed at full-image scale: a DCT-scaled-and-reboxed re-encode capped at
  2048px (`decode::thumbnail_jpeg_near` + mozjpeg q85) took 183-535ms on a
  21.5MP source, slower than shipping and decoding the whole JPEG (~200ms).
  So `commands::preview` sends the whole JPEG for a JPEG-only folder rather
  than re-encoding a capped copy; `PREVIEW_PIXEL_LIMIT` still resizes it in
  the Linux worker. Source:
  `docs/plans/_archived/20260927-jpeg-view-only/learnings.md`, Step 2.

### A partial decode's cost is set by its row, not its size (Measured)

`jpeg_skip_scanlines` on a baseline JPEG still entropy-decodes the rows it
skips; it only skips the IDCT and color conversion.

- Why: baseline Huffman data is not randomly addressable, so libjpeg must walk
  every MCU row from the start of the scan to reach the wanted one.
- Measured on the 7008x4672 `JpgFromRaw`: a 1024 crop costs 10.8ms at row 300
  and 44.1ms at row 4400, while growing the crop from 512 to 2048 at the same
  row only moves 18.4ms to 29.8ms.
- So a budget for a crop has to be stated for the worst row, not an average
  one, and shrinking the crop is not a way to make it fit.

### A crop's origin snaps to an MCU boundary; don't assume centered or bit-identical (Hit, twice)

`jpeg_crop_scanline` snaps the requested crop origin **down** to an MCU
boundary (16px in the tested files) and does not necessarily widen the crop to
compensate.

- Why: baseline JPEG DCT blocks (MCUs) are the smallest unit libjpeg can crop
  to; it never crops mid-MCU.
- What broke: two independent guesses were wrong. `FocusCrop`'s `point_x`/
  `point_y` are not the crop's center (measured 269 vs a center of 262 on one
  file). `crop_size()`'s width does not grow to keep the point centered (a
  64px-wide request at x=200 came back as width 64 at x=160, landing the point
  at 40, not 32) — that behavior also isn't consistent across files (it does
  widen on a 4:2:2 test file). Always derive the point of interest as
  `center - crop.origin` from the actual returned crop, never assume a fixed
  offset or that width grows.
- A crop decoded via `jpeg_crop_scanline` is also **not bit-identical** to the
  same region of a full `decode_rgb` near its left/right edges (chroma
  upsampling has one fewer neighbor there; differences up to 2 in the first
  columns, 1 in the last, on the tested file). Equality tests against a crop
  must exclude ~8px from each edge or use a tolerance there, not assert exact
  equality everywhere.
- Source: `docs/plans/_archived/20260918-focus-check/learnings.md`, Steps 1
  and 2.

### mozjpeg panics, rather than returning `Err`, on non-JPEG bytes (Hit)

`Decompress::new_mem` and friends panic on bytes that are not a JPEG instead
of returning a `Result::Err`.

- Any code path that cannot guarantee its input is a real JPEG (e.g. scoring an
  arbitrary embedded preview) must wrap its own call into mozjpeg in
  `catch_unwind` and convert the panic to an `Err`; callers then just call
  `.ok()` on it.
- Source: `docs/plans/_archived/20260919-sharpness-cue/learnings.md`, Step 1.

### `partial.rs` replaces libjpeg's `error_exit`, which would call `exit(1)` (Hit)

`crates/core/src/partial.rs` (`decode_region`, behind `decode_focus_crop` and
`decode_crop`) calls libjpeg through `mozjpeg-sys` with `jpeg_std_error`, whose
default `error_exit` calls `exit(1)`: a fatal error there used to end the whole
process, and `catch_unwind` could not stop it.

- The guard is in place: `decode_region` installs `unwind_error_exit`, an
  `extern "C-unwind"` handler that formats libjpeg's message and
  `resume_unwind`s (no panic hook, so nothing is printed), and wraps the body in
  `catch_unwind`. `decode_focus_crop` / `decode_crop` now return `Err` with the
  text (e.g. `libjpeg fatal error: Not a JPEG file: starts with 0x3c 0x44`), so
  the app's `focus_crop` command reports the error instead of exiting, and
  callers no longer need the `decode_rgb` pre-check `riffle-cli check` used to
  run.
- Unwinding through libjpeg's C frames needs `mozjpeg-sys`'s `unwinding`
  feature; `crates/core/Cargo.toml` names it explicitly.
- A JPEG cut inside its scan data is not a fatal error: libjpeg warns
  (`Premature end of JPEG file`) and the crop comes back with filler rows. Only
  a cut inside the header, an empty input or a non-JPEG fails.
- `partial.rs`'s decoder and the `mozjpeg` crate's `decode_rgb` are not
  interchangeable validators: `decode_rgb` also fails on a JPEG cut inside its
  scan data, which `partial.rs` accepts with a warning. To test the `Err` path,
  cut inside the header (e.g. `&jpeg[..64]`), not the middle of the data.
- Set the handler through the pointer,
  `(*cinfo.common.err).error_exit = ...`, not on the local `err` after
  `jpeg_std_error(&mut err)`. rustc does not see the read through the raw
  pointer and trips `unused_assignments`.
- Seen on `NEF\NIKON_D70_Nikon.nef` and `DNG\CGO3P_YUN00007.dng`: before the
  guard, a run died mid-way with libjpeg's `Empty input file` / `Not a JPEG
  file` on stderr, exit code 1 and no summary.
- libjpeg warnings (`Corrupt JPEG data`, `Invalid SOS parameters`) still go
  straight to stderr, while `check`'s failure lines and summary are on stdout.
- Source: `docs/plans/_archived/20261001-cli-check-samples/learnings.md`, Step 1;
  `docs/plans/_archived/20261002-partial-decode-error-exit/learnings.md`, Step 1.

### Draining background work at quit needs `build()` + `run()` (Hit)

Work that must finish before the process ends — Phase 6's sidecar writer has a
300ms debounce, so a quit inside that window would otherwise leave the write
for the next folder open — is drained in `tauri::RunEvent::ExitRequested`.

- Why: `tauri::Builder::run(context)` takes only the context and gives no place
  to hook the run event. Use `Builder::build(context)?` and then
  `App::run(|app, event| ...)`; the closure's first argument is an `&AppHandle`,
  so `app.state::<...>()` works there.
- The writer's `Drop` is deliberately **not** the drain: a managed state's
  `Drop` is not guaranteed to run during process teardown, and even when it
  does, the channel disconnect races the exit instead of being waited on. Send
  an explicit flush message and block on a reply channel with a bounded wait
  (~2s) instead.
- This is the one place the "never block the main thread" rule above does not
  apply: the run loop is on its way out, and blocking there is what makes the
  drain a drain. Keep the wait bounded anyway.
- Source: `docs/plans/_archived/20260918-ratings-xmp-sidecars/learnings.md`,
  Step 3.

### A bounded drain must fail loudly, and a sidecar test must not swallow writer errors (Hit)

`Writer::flush` returns `bool` (`true` when the drain reply arrived within
`DRAIN_TIMEOUT`). Production callers may ignore it, but tests must not. The
switch-to-Both sidecar test was flaky on `windows-latest` because the 2 s drain
gave up while the writer was still inside `write_kind` (`sync_all` plus the
Windows `rename` retries), or because the write failed and `on_error` only
printed to stderr. Either way the failure surfaced one step later as `NotFound`
on the read.

- In a sidecar-writer test, collect every `on_error` message into a
  `Mutex<Vec<String>>`. Drain with a generous budget (30 s), and assert both the
  `flush` result and that the errors are empty before reading any sidecar.
- Filter out `" (retrying in "` messages when asserting. A write that fails once
  on the deadline path and succeeds on the rewrite is correct. Keep the full list
  in the assertion message so a transient sharing violation stays visible.
- Source: `docs/plans/_archived/20260930-todo-five-more-items/learnings.md`, Step 1.

### Snapshot the state you decided on, not just the lock, across a release (Hit)

`reconcile_sidecars` (decides what to parse) and `store_sidecar_ratings`
(stores the parse and clears `dirty`) run with the lock released in between,
so a `set_rating` landing in that window was silently discarded by the
"sidecar wins" write.

- Why: releasing a lock between "decide" and "act" always opens a window for
  the state to change underneath the decision; re-checking "is the lock still
  free" is not enough, the *value* the decision was based on has to be
  re-checked too.
- Fix pattern: `reconcile_sidecars` hands back the `dirty` flag it observed
  for each path alongside the parse job, and `store_sidecar_ratings` only
  applies its update when the row's `dirty` still matches that snapshot —
  otherwise the newer write is left in place.
- Source: `docs/plans/_archived/20260918-ratings-xmp-sidecars/learnings.md`,
  Step 4.

### A global `event.listen("tauri://focus")` fires for every window (Hit)

Listen for a window's own focus with
`window.__TAURI__.window.getCurrentWindow().listen("tauri://focus", …)`, not
the global `window.__TAURI__.event.listen`.

- Why: the global form registers with `target: { kind: "Any" }`, and Tauri
  2.11 lets an `Any` listener bypass the per-window filter, so the main
  webview also receives every other window's focus.
- What broke: when Settings was a separate window, closing the Clear Cache
  dialog refocused it, the main window ran a focus rescan, and `clear_index`
  refused with `a scan is running`, or the clear's own reopen superseded that
  rescan. Settings is now a modal inside the main window, so the same
  `tauri://focus` event fires on close instead; the `tauri://focus` listener
  in `main.ts` skips the resync while `settings.isOpen`, to avoid the race.
  Keep the scoped form for the next window too.
- The focus rescan is throttled (`focusRescanDue`). `lastScanAt` is stamped in
  `startScan` and also by the focus listener right before it calls `resync()`,
  because `resync()` reaches `startScan` only after its `list_arw` resolves and a
  second focus in that window would otherwise pass. A focus inside the interval
  is dropped, not deferred. `File > Reload Folder`, the tree's `Refresh` on the open
  folder (trigger `refresh`), `folder-changed` and the
  trash / rename paths call `resync()` directly and are not throttled.
  `scan_folder` skips the `pending` entry when both `todo` and `faces_todo` are
  empty, so an idle focus rescan ends at once. Source:
  `docs/plans/_archived/20260930-todo-five-more-items/learnings.md`, Step 2.
- Every native dialog's close refocuses the main window, so every path that
  opens one (`clear_index`'s confirm, `pick_folder`) needs the focus rescan
  held off until its follow-up open has run. Otherwise `resync()` on the
  previous folder mints a `scan_folder` id in the same second as the open's
  `startScan`, and the later one supersedes it (logged as `scan superseded`).
  The throttle alone misses the picker: `openFolder` mints `folderToken`
  before `pick_folder`, so `resync()`'s token guard lets a listing for the old
  folder through. `openFolder` wraps its whole chain (`pick_folder` through
  `openDirectory`) in `settleIdle`, so `resync()` defers while it is in
  flight; a picked folder's `openDirectory` discards the deferred rescan and
  a canceled picker drains it. Pass `settleIdle` the whole chain including
  its `.catch` (as the trash run does), never a promise that can reject:
  `promise.finally(...)` would otherwise leave an unhandled rejection.
  `openDirectory`'s `idle.discard()` clears only the held operation, not the
  in-flight count, so the wrap stays balanced. Source:
  `docs/plans/_archived/20261003-picker-focus-double-scan/plan.md`, Step 1.

### Emit `faces-done` only after the scan's `running` entry is dropped (Measured)

`start_scan`'s task calls `state.finish(scan_id)`, and releases the `Scans`
lock, before it emits `faces-done`.

- Why: the frontend drains a rescan deferred during the scan (focus, watcher,
  `File > Reload Folder`) off `faces-done`. If that rescan's `scan_folder`
  still finds the ended scan in `running`, it joins it, `joined_previous`
  makes `changed` nonzero, and `refreshOnScanDone` forces a `folder_entries`
  read (`open entries`) of an unchanged folder.
- `scanning()` turns false a moment before the frontend's `scanRunning`; a
  `clear_index` already in flight can proceed slightly earlier, which it
  re-checks itself, so nothing new becomes possible.
- Verified on Windows (2026-10-04, `Handpic.log` with Timing logs on, a
  1050-file folder cold for the extractor version): after
  `rescan deferred: trigger=focus` during the scan, the rescan drained off
  `faces-done` logged `rescan: trigger=focus deferred=true` with `todo=0` and
  read no `open entries` of its own. The only reads after the open were the
  `scan-done` and `faces-done` refreshes, which each followed a pass that
  wrote rows. A focus with no scan running logged
  `rescan: trigger=focus deferred=false`, `todo=0`, no `open entries`.
- Reading the log: the `faces-done` refresh's `open entries` is logged between
  the drained rescan's `scan list` and its reconcile, so it looks like the
  rescan's own read. Attribute each `open entries` to the pass that wrote rows
  (open, `scan-done`, `faces-done`), not to line order. A rescan with `todo=0`
  that is followed by an `open entries` is a regression only if no pass wrote
  rows just before it.
- Source: `docs/plans/_archived/20261003-follow-up-rescan-open-entries/plan.md`, Step 1;
  verification: `docs/plans/_archived/20261004-follow-up-rescan-log-check/plan.md`.

### The main window is created hidden; `setup` shows it as a fallback (Inferred)

The main window entry in `tauri.conf.json` has `"visible": false`, so
`tauri-plugin-window-state` restores the saved size and position before the
window is on screen (otherwise it flashes at the 1280x800 default first). The
plugin (2.4.1, default `StateFlags::all()`) ends `restore_state` with `show()`
+ `set_focus()`, and the window build runs that before the app's `setup`
closure, where an unconditional `show()` follows.

- Why the plugin's show is not enough: it skips the show when the saved state
  has `visible: false` (macOS saves that after a quit while minimized), and a
  failing `set_position` / `set_size` / `set_fullscreen` returns through `?`
  before its `show()`. Without the `setup` show the window would stay hidden.
- Source: `docs/plans/_archived/20261010-startup-window-flash/plan.md`, Step 1.

### `frontendDist` resolves from the `tauri.conf.json` directory (Hit)

`tauri.conf.json` lives in `crates/app/`, not the conventional `src-tauri/`, so
the Vite output under the sibling `ui/` is `"frontendDist": "ui/dist"`.

- Why: Tauri resolves the path relative to the directory holding
  `tauri.conf.json`.
- What broke: the plan's `"../ui"` pointed at `crates/ui`, and
  `tauri::generate_context!()` failed the build because the path did not exist.

### `tauri icon` also writes `ios/` and `android/` icons (Hit)

`source.png` is rendered by `python3 tools/macos/app-icon/gen.py` (macOS
only; see its header). Regenerate the icon set with
`pnpm exec tauri icon crates/app/icons/source.png -o crates/app/icons`. It
leaves `tauri.conf.json` alone, but it also creates `icons/ios/` and
`icons/android/`, which the app has no use for.

- Delete those two folders before committing.
- `gen.py`'s headless Chrome prints `CVDisplayLinkCreateWithCGDisplay failed`
  and `task_policy_set` errors to stderr on every run; they are harmless and
  the screenshot is still written.
- Source: `docs/plans/_archived/20260920-app-icon/learnings.md`, Step 1;
  `docs/plans/_archived/20260928-film-frame-icon/learnings.md`, Step 2.

### Enabling the updater plugin pulls in `serde_json` at compile time (Hit)

Adding a `plugins.updater` block to `tauri.conf.json` makes
`tauri::generate_context!()` expand to code that references `serde_json`
directly, so `crates/app` needs it as a direct dependency or the build fails
with "could not find `serde_json` in the list of imported crates".

- Why: the generated context code assumes the crate already depends on
  `serde_json`; Tauri's own transitive dependency does not satisfy that.
- Source: `docs/plans/_archived/20260918-github-releases-auto-update/learnings.md`,
  Step 1.

### Self-update defers the install to quit on Windows only (Hit)

`tauri-plugin-updater-2.11.0`'s macOS and Linux (AppImage) `install_inner`
only extract and rename bundles (`std::fs::rename`, with an authorized
fallback); they never exit or signal the process, so `update.rs` keeps
`download_and_install` there and `RunEvent::ExitRequested` runs normally on a
later real quit. Windows is different: the running exe is locked, so its
`install_inner` launches the installer and calls `std::process::exit(0)`
directly. `update.rs` therefore only calls `Update::download` on Windows (in
the background, signature-verified) and keeps `(Update, Vec<u8>)` in
`UpdateRun`; `main.rs`'s `ExitRequested` arm calls `update::install_pending`
right after the sidecar writer's flush, so the installer starts only once
every pending write is on disk. The updater is built with
`restart_after_install(false)` (the Windows default is `true`), so quitting
means quitting. Compiled, not exercised against a real install.

The menu's check then offers `Restart Now`. On macOS/Linux it calls
`AppHandle::request_restart` (not `restart`, which is `-> !` and parks the
dialog-callback thread), so the same `ExitRequested` arm runs. On Windows it
swaps the pending `Update` for `update.restart_after_install(true)` (a setter
on `Update` itself, so no re-check or second download) and calls
`app.exit(0)`; `install_pending` then launches the installer with the
relaunch arguments. The flag is set only on that click, never at build time:
building with `true` up front would relaunch after `Later` and a normal quit.

- Why: only Windows needs the exe unlocked before it can overwrite itself;
  macOS/Linux replace files the running process isn't holding open. Deferring
  on macOS would run `install_inner`'s `run_on_main_thread` fallback from the
  main-thread quit callback, a deadlock risk.
- When touching the update or sidecar-flush paths, keep `install_pending`
  after the flush in `ExitRequested`, and don't assume `install` returns on
  Windows.
- Source: `docs/plans/_archived/20260919-silent-auto-update/learnings.md`,
  Step 1; `docs/plans/_archived/20260920-windows-deferred-update/`;
  `docs/plans/_archived/20260925-restart-after-update/learnings.md`, Step 1
  (the `Update::restart_after_install` setter, which avoids a re-check).

### App items go into the default menu's own submenus (Hit)

The menu bar keeps the platform's default submenus, plus a `View` where the
default has none. `app_menu::build`
finds them in `Menu::default` by title and inserts into them: `Settings...`
(`CmdOrCtrl+,`) after About in the macOS app menu (in `File` elsewhere), and
`Open Folder…` and `Reload Folder` at the top of
`File`. Linux's default has no `File`, so one is prepended there. `Edit` ships a predefined Undo/Redo pair at its top
that owns `CmdOrCtrl+Z`; the app's `Undo` / `Redo` (emitting `undo` / `redo`
to the frontend) replace that pair rather than being added next to it. Their
accelerators come from the keymap's `undo` / `redo` actions, like the File menu's
`Open Folder…`, so the frontend keydown runs the keys and the menu only mirrors them. The replacement only
fires when the item at position 0 is still `MenuItemKind::Predefined`, so a
Tauri reordering cannot make it silently remove the wrong item. About (item 0
of the macOS app menu, of `Help` elsewhere) is rebuilt the same guarded way,
with `Menu::default`'s metadata plus `icons/128x128.png` as `icon`, since the
default metadata has none and macOS / GTK would show no app icon. Settings themselves (sidecar format, shortcuts, the
dev-only timing logs) live in a modal inside the main window
(`#settings-dialog` in `ui/index.html`, driven by `ui/src/settings.ts`), not
in menu check items, so the menu reads no plugin state. `Settings...` only
emits `open-settings`, which `main.ts` answers by opening the modal, the same
way `open-folder` and `undo` reach the frontend. There is one window, so
`capabilities/default.json` lists only `main`.

On Windows and Linux the items prepended to `File` (`Open Folder…`,
`Reload Folder` and the separator after them) are bound to one local array
typed `[&dyn tauri::menu::IsMenuItem<Wry>; N]`, the same trait-object form
the View menu's `view_kinds` uses, since the elements differ in concrete type
and the separator must outlive both the `prepend_items` and `insert_items`
calls. `Settings...` and `Check for Updates…` are inserted at `own.len()`, so
they land after File's own items however that list grows. A literal index
went stale when #537 removed two File items and left two separators in a row.

- Source: `docs/plans/_archived/20260929-file-menu-separator/learnings.md`, Step 1.

`Edit` also ships a predefined `Select All` on every platform, as the last
item of the submenu (Undo / Redo sit first). `app_menu::build` removes it the
same guarded way, only when the last item is still `MenuItemKind::Predefined`,
and appends a custom `Select All` (a plain `MenuItem`, no macOS icon) whose
accelerator comes from the keymap's `selectAll` action and which emits
`select-all` to the frontend like `undo` / `redo`.

`View` is created on Windows and Linux (the default menu has none there), so
`app_menu::build` builds one and inserts it right after `Edit` (appended when
`Edit` is missing); on macOS the default `View` exists and the items are
prepended above its Enter Full Screen, with a separator between. It holds
only `Folders`, `Metadata` and `Filmstrip`, `CheckMenuItem`s from the
`VIEW_ITEMS` table of `(id, action, label)`; each takes the action's
accelerator (`toggleLeft` / `toggleRight` / `toggleStrip`, whose defaults
are `Alt+Cmd+Arrow` on macOS and `Ctrl+Alt+Arrow` elsewhere), and all emit
one `menu-action` event with the action name, which `main.ts` runs through
`runAction` under the same gates as the keydown path: `modalOpen()`,
`treeGate`, the first-launch format dialog, and closing the strip's context
menu first. The checks follow `AppPanels`, the panels `set_panels` last
received (seeded from the store in `setup` before the first `refresh`):
`set_panels` calls `app_menu::apply_panels`, and a click on an item first
re-applies the stored panels, undoing muda's native toggle-on-click, before
emitting `menu-action`, so a gated click leaves the check as it was and an
effective one is set by the frontend's `set_panels` a moment later.
`toggleSides` (`Tab`), `focus`, `zoom` and `compare` are main-view / strip
keys with no menu item. A test (`menu_covers_every_action`) requires every
keymap action to be in the menu or in its `MENU_LESS` list.

- Why only the pane toggles: they have a state a check mark can show, and
  a modifier default that can be an accelerator; the main-view keys had
  neither and showed an empty accelerator column.

- Why: a submenu per setting cluttered the menu bar; macOS apps put
  `Settings...` in the app menu.

### Rebuild the menu on macOS, patch accelerators in place elsewhere (Inferred)

The menu is no longer passed to `Builder::menu`; `setup` calls
`app_menu::refresh` right after `load_settings`, and `update_keymap` calls
it again whenever an accelerator changes. On macOS `refresh` always rebuilds
the whole menu through `AppHandle::set_menu`, because muda's
`set_accelerator(None)` on an existing item does not clear a stale key
equivalent. On Windows and Linux `refresh` only calls `set_menu` the first
time (no menu yet, i.e. `setup`); afterwards it looks up `Open Folder…`,
`Edit > Undo`, `Edit > Redo`, `Edit > Select All` and the `View` items with `Submenu::get` on each top-level submenu
(`Menu::get` does not recurse) and calls `set_accelerator` on them (a
`MenuItemKind::MenuItem` or, for `View`, a `MenuItemKind::Check`),
which muda's Windows backend handles correctly (label and `HACCEL` are
rewritten, `None` removes the entry). On every platform `refresh` ends by
re-applying the `View` checks from `AppPanels`, since a macOS rebuild
creates the check items afresh.

- Do not call `set_menu` at runtime on Windows: it turns muda's dark menu
  bar white. The suspected cause was the separate Settings window, which
  inherited the app-wide menu, so `set_menu` attached one `HMENU` to two
  top-level windows (unproven). Settings is now a modal in the main window,
  so that suspicion no longer applies, but keep the rule until someone
  verifies `set_menu` on a Windows device.
- Source: `docs/plans/_archived/20260920-menu-accelerators/learnings.md`,
  Step 1, and `docs/plans/_archived/20260922-windows-dark-menu-bar/learnings.md`
  (unverified on a real device).

### Menu icons: bundled SF Symbols rather than `NativeIcon` (Hit)

On macOS seven app items carry an icon, and none of them is a `NativeIcon`.
`NativeIcon` has neither an undo nor a modern gear, and `NativeIcon::Folder`
and `NativeIcon::TrashFull` are color Finder bitmaps rather than template
images (`isTemplate == false`), so they would keep their color while every
icon around them tints. So `Settings...`,
`Undo`, `Redo`, `Open Folder…`, `Open Log Folder`,
`Reload Folder` and `Check for Updates…` use `IconMenuItem::with_id` with an
`Image::from_bytes(include_bytes!(...))` of a PNG committed under
`crates/app/icons/menu/` (which is why `crates/app/Cargo.toml` enables Tauri's
`image-png` feature). `folder.png` is deliberately shared by `Open Folder…`
and `Open Log Folder`: they live in different menus, which are never open at
the same time. Other platforms keep the plain `MenuItem` behind `cfg`.

Regenerate those PNGs with `swift tools/macos/export-menu-icons.swift`, and
only when a symbol, its size or weight changes; AppKit's rasterization
can differ between macOS releases, so the committed files are the source of
truth. The script never runs at build or run time.

- The bundled PNGs are alpha-only template images: the workspace `Cargo.toml`
  patches muda through `[patch.crates-io]` to `minodisk/muda` (muda 0.19.3 plus
  an unconditional `nsimage.setTemplate(true)` in `menuitem_set_icon`), so they
  tint with the menu appearance — white in dark mode, black in light mode —
  exactly like the OS-provided items. Stock muda never marks a custom menu
  image as a template and Tauri exposes no template flag for menu items, so
  without the patch the icons keep whatever color the PNG carries. The patch
  must use a `git` or `path` source (crates.io-to-crates.io patches are
  rejected) pinned to a version satisfying `tauri`'s `muda = "^0.19"`, hence
  0.19.3 and not 0.20; the unconditional fork was chosen over the opt-in API
  because wiring an opt-in flag through would need a Tauri fork as well.
  https://github.com/tauri-apps/muda/pull/413 carries the same change upstream
  as an opt-in `IconMenuItem::set_icon_as_template`. Drop the patch entry once
  that ships in a muda release Tauri resolves **and** Tauri exposes the
  template flag for menu items; until both hold, the fork stays. See
  `docs/plans/_archived/20260920-menu-icon-glyph-size/learnings.md`,
  "Side experiment: muda's missing `setTemplate` is a one-line gap".
- **Hit**: the PNG-backed items (`Settings...`, `Undo`, `Redo`, `Open Folder…`,
  `Open Log Folder`) rendered visibly larger than the rest of the menu. The cause is the
  glyph's padding, not the canvas: the export script drew the SF Symbol at
  `pointSize: 18` into an 18pt canvas — filling it edge to edge and in fact
  overflowing it, so the committed PNGs were clipped. muda does resize every
  custom `Image`-backed item to 18pt (`icon.inner.to_nsimage(Some(18.))` in
  `src/platform_impl/macos/mod.rs`), which is why the PNG's own pixel size has
  no effect on the rendered size, but that resize is not what made the icons
  look bigger. Fixed in `tools/macos/export-menu-icons.swift` by separating
  the canvas size (still 18pt) from the glyph `pointSize` (now 12, shared by
  all three symbols) and regenerating the PNGs. Match the **OS-provided** menu
  items (`Cut` / `Copy` / `Paste` in the Edit menu), not the `NativeIcon`
  templates: those pad their glyph to ~16pt of ink inside a 19–20pt canvas and
  themselves read larger than the rest of the menu, so a size tuned against
  them (14) still looked too big. Verify any future change with an alpha
  bounding box that also checks for edge contact. See
  `docs/plans/_archived/20260920-dependency-refresh/learnings.md` for the
  original observation, whose muda-based explanation is superseded by this
  bullet.

### What a new menu item needs (Inferred)

- A plain item is an `IconMenuItem` on macOS, with an SF Symbol PNG exported
  by `tools/macos/export-menu-icons.swift` (which needs a Mac), plus a
  `MenuItem` twin elsewhere, behind the `cfg` split below.
- If it mirrors a keymap action, it goes into `keyed_items()` (so `refresh`
  keeps its accelerator current) and the action's default key carries
  `ctrl`, `alt` or `meta`: a menu accelerator is app-global, so it would fire
  while typing in a text field, and `shortcuts::accelerator` shows nothing
  for a modifier-less key. The pane toggles moved off `F6` / `F7` / `F8` to
  `Ctrl+Alt+Arrow` / `Alt+Cmd+Arrow` for this.
- A `CheckMenuItem` cannot carry an icon (muda / Tauri have no icon-bearing
  check item), so it needs no `cfg` twin. Its state is applied by
  `app_menu::apply_panels` and re-applied at the end of `refresh`.
- Source: `docs/plans/20260930-view-menu-panes/plan.md`, Step 1.

### Adding a macOS menu item needs the same `cfg` split as its siblings (Hit)

Every macOS menu item in `app_menu` is behind `#[cfg(target_os = "macos")]`
(using `IconMenuItem`) with a `#[cfg(not(target_os = "macos"))]` twin (plain
`MenuItem`), because `MenuItem` is only imported under the latter cfg. A new
item added as a plain, un-cfg'd `MenuItem::with_id(...)` merges *textually*
clean with unrelated concurrent menu-icon changes but breaks macOS
compilation (`MenuItem` no longer in scope there).

- A branch-only CI pass does not catch this: the branch's own copy of the
  import still has `MenuItem` in scope, and the break only exists in the
  merge result. If a failure is isolated to one OS's CI job right after a
  merge/rebase and looks like a runner flake, re-run it once to rule out
  flakiness, then check out the actual merge ref (not the branch head) —
  GitHub builds the PR's merge commit, so line numbers and blame on the
  branch head alone can point at unrelated code.
- Give any new platform-specific menu item the existing `cfg` split up
  front, following the bundled-icon pattern above.
- Source: `docs/plans/_archived/20260920-scan-timing-logs/learnings.md`,
  "Merge conflict with concurrently-landed macOS menu icon work".

### This machine cannot compile the macOS target; check `objc2` code in a scratch crate (Hit)

`cargo check --target aarch64-apple-darwin` fails here because
`objc2-exception-helper`'s build script needs a C compiler for that target;
the macOS `cfg` branches of `crates/app` are verified only by CI's macOS
job. To partially self-check new macOS-only code (e.g. `objc2-foundation`
calls) before pushing, copy just that code into a scratch crate depending
on the same `objc2-foundation` / `percent-encoding` versions pinned in
`Cargo.lock`, and run `cargo check` / `cargo clippy -D warnings --target
aarch64-apple-darwin` on the scratch crate. This does not compile the rest
of the macOS cfg (command wiring, `dead_code` attributes elsewhere), so
still expect CI's macOS job to catch anything outside the copied slice.

- `objc2-foundation` 0.3.2's `trashItemAtURL_resultingItemURL_error` is a
  safe fn (no `unsafe` block needed); letting the out-pointer argument's
  `None` infer its type avoids a direct `objc2` dependency for `Retained`.
- Source: `docs/plans/_archived/20260928-undo-trash-rejected/learnings.md`,
  Step 2.

### Trash restore: plan and perform in one pass, and re-verify identity before renaming back (Hit)

A restore plan computed before any file is moved cannot know whether a
given RAW's move will actually succeed, but a sidecar's fate depends on it:
sidecars only come back when their RAW does (mirroring `trash::run`'s own
rule), otherwise a reject sidecar could land next to an unrelated file that
took the RAW's freed name. So `trash::restore` plans and performs the
restore in one pure function taking `(run, in_trash, exists, mover)`,
rather than returning a plan struct to execute later.

Also, a file existing at the recorded trashed-to path is not enough to call
it "still in the Trash and safe to restore": the OS trash can hand a freed
name to an unrelated file trashed later from elsewhere (e.g. repeating
camera file names like `DSC00001.ARW` across cards). Record the file's
identity at trash time (on Unix, `(dev, ino)` via `MetadataExt`) alongside
its trashed-to path, and require it to still match before renaming back.

- Source: `docs/plans/_archived/20260928-undo-trash-rejected/learnings.md`,
  Steps 1-2.

### A trash run of sidecars alone: check how `trash::redo` regroups the paths (Hit)

`Delete Sidecars…` records a run whose every path is a sidecar, and reuses
`trash_rejected_undo` / `trash_rejected_redo` for it. `trash::restore` takes
such a run as is (`raw_back` starts `true`), but `trash::redo` rebuilds the
RAW-and-sidecars groups from the flat path list, and it used to append every
non-RAW path to the previous group: a sidecar-only run became one group
headed by its first sidecar, so a redo reported one moved file and, when
that first sidecar was gone, skipped all the others. A non-RAW path now
joins the previous group only when that group's head is a RAW. Before
recording a new kind of run in `Runs`, run its undo and redo in a unit test
rather than reasoning from the code (both of these were "read, not run" in
the plan).

- `trash_rejected_redo` writes no index row (it cannot tell which RAW a
  sidecar path belongs to), so a redo of a sidecar delete relies on the open
  folder's resync to clear the judgments; a closed folder keeps them until
  its next open.
- Source: `docs/plans/_archived/20261005-folder-sidecars/learnings.md`,
  Step 2.

### A new kind of primary file in a trash run: check `trash::restore`'s notion of "primary" (Hit)

`trash::restore` counted only RAWs as a run's primary files, so a run of
JPEGs (the filmstrip's `Move to Trash…` in a view-only, JPEG-only folder)
would have come back from the Trash but listed nothing in
`Restored.restored`. The UI would then say `Restored 0 files`, and
`restoredInto` would not place them. It now treats a JPEG like a RAW
(`is_jpeg_file`). `redo` needed no change, because a JPEG after a JPEG
already forms its own group. When `Runs` starts recording a new file kind,
test restore's `restored` list for it, not just that the files return.

- `trash_files` (the strip's move) checks each path's parent against `dir`
  in `groups_of` and reports any other as a `Failure` without moving it.
  That keeps a sidecar from going to the Trash with a same-stem file of
  another folder. It also does not call `state.stop_embedding()`, because
  it moves no index rows. The third pass skips a vanished file, and the
  next rescan's reconcile drops the gone rows.
- On the UI side, after the `trash_files` invoke returns, `trashed` must run
  even if `openDir` / `folderToken` changed meanwhile. The backend has
  already recorded the run, so returning early would lose its undo entry.
  Only the strip refresh is conditional on the open folder being the run's
  folder.
- Source: `docs/plans/_archived/20261013-trash-from-strip/learnings.md`,
  Steps 1-2.

### Sony MakerNote fields: verify each tag's type and model `Condition` in Sony.pm directly (Hit)

Don't infer a new Sony MakerNote tag's type or model gate from a
similarly-numbered or similarly-named tag; check ExifTool's `Sony.pm` for
that exact tag.

- Types vary per-tag even among neighbors: `AFAreaModeSetting` (0x201c) is
  BYTE, count 1, while 0x201a is a LONG. `RAWFileType` (0x2029),
  `MeteringMode2` (0x202c), `ExposureMode` (0xb041), `ReleaseMode` (0xb049),
  `SequenceNumber` (0xb04a) are SHORT; `DynamicRangeOptimizer` (0xb025) and
  `ImageStabilization` (0xb026) are LONG; `CreativeStyle` (0xb020) is ASCII,
  count 16, out of line.
- `ReleaseMode`, `SequenceNumber` and `ExposureMode` use 65535 as
  ExifTool's `RawConv` sentinel for "no value"; match it by having no
  formatting arm for that value, not by special-casing it.
- Unlike `FocusMode` / `AFTracking`, none of `AFAreaModeSetting`,
  `RAWFileType`, `MeteringMode2`, `ExposureMode`, `ReleaseMode`,
  `SequenceNumber`, `DynamicRangeOptimizer`, `ImageStabilization` or
  `CreativeStyle` carries a model `Condition` in Sony.pm, so no model gate
  is needed when formatting them — but check Sony.pm per-tag rather than
  assuming this applies to the next field you add.
- The MakerNote parsing itself (the Sony gate, the stored type of
  `FocusFrameSize`, the Leica and Sigma notes) is covered in
  [`raw-metadata-parsing.md`](./raw-metadata-parsing.md).

Source: `docs/plans/_archived/20260925-sony-af-meta/learnings.md`, Steps 1
and 3.

### A case-insensitive file system makes `exists()` match the wrong spelling (Hit)

On macOS APFS, `sidecar_path(arw).exists()` is true for `H.ARW.DOP` even when
the code built the path as `h.arw.dop` (or vice versa), because the file
system does case-insensitive but case-preserving lookups.

- Why: the OS resolves the path regardless of case, so an existence/rename
  check can silently hit a sidecar with different case than the one just
  built.
- Any code that renames or writes a sidecar based on `exists()` needs to
  tolerate landing on the other case's file rather than assuming its own
  spelling was used; do not assert on the exact resulting name in tests, only
  that exactly one sidecar exists.
- Source: `docs/plans/_archived/20260918-photolab-dop-sidecar/learnings.md`,
  Step 2.

### chrono `format(...)` with a user-supplied pattern: `.to_string()` panics, so write with `write!` (Hit)

`NaiveDate::format` with a pattern the user typed does not fail up front. An
unknown specifier or a lone `%` becomes `Item::Error`. A specifier that needs
a time, an offset or a timestamp (`%H %M %S %p %I %f %z %Z %s %T %R %c %+`)
makes `Display` return `fmt::Error`. `.to_string()` on that value panics.

- Format with `write!` into a `String` and map the error to a refusal message
  (`import.rs`'s `format_date`).
- `%n` and `%t` are valid and emit a newline and a tab. A control-character
  check on each produced path segment is what refuses them.
- Validate a pattern by formatting a fixed date, and run the same segment
  checks on every real date. Re-validate a stored pattern at load, falling back
  to the default, so a bad stored value never reaches a run.
- Source: `docs/plans/_archived/20261106-import-from-card/learnings.md`, Step 1.

### Path comparisons in the UI take the platform `ignoreCase` flag; a DOM-touching module cannot be imported by a node-env test (Hit)

The UI's path helpers (`rebase`, `opensTarget`, `restoredInto`, and the trash
relation calls) fold case only when given the platform flag,
`folders.ignoreCase`. A call site that omits it compares `Photos` and `photos`
as different folders on Windows and macOS.

- `relation` folds the Windows drive letter (`C:` vs `c:`) even without the
  flag. A test that varies only the drive letter passes without the flag, so
  it proves nothing. Vary a folder name's case instead.
- `folders.ts` touches the DOM at load, so `trash.ts` cannot import it:
  `trash.test.ts` runs under Vitest's `node` environment and would break. A
  DOM-free module takes the flag as a defaulted trailing parameter, and its
  caller (`main.ts`) passes `folders.ignoreCase`.
- Source: `docs/plans/_archived/20261001-clear-three-todos/learnings.md`,
  Step 2.

### DxO PhotoLab and `.dop` sidecars

The `.dop` format and PhotoLab database pitfalls (indentation and keys, the
`Settings` block and `Orientation`, Uuid matching and virtual copies, and the
lookup's rename caveat) live in [`photolab.md`](./photolab.md).

### Removing an XMP element needs its end tag (Hit)

`xmp.rs`'s `locate` takes the property's local name (`LocalName` compares
against `&str`). For an element-form property it cannot return on the `Text`
event: it waits for the end tag so it can report the whole element's range for
removal.

- Clearing an element-form label removes its whole line only when nothing but
  whitespace shares that line; otherwise it removes just the element.
- Values are spliced raw with no escaping, so a value containing `"` or `<`
  would break the XMP. Fine for the label vocabularies; check it again for any
  new use of this splice path.
- Source: `docs/plans/_archived/20260919-color-labels/learnings.md`, Step 1.

### A sidecar write of two properties patches in two independent passes, and clearing one leaves its `xmlns` behind (Hit)

`xmp::write_rating` and `xmp::write_label` each touch two properties
(`Rating`/`xmpDM:good`, `xmp:Label`/`photoshop:LabelColor`): they patch one,
re-parse the spliced text, then patch the other, so each splice's byte
offsets are never computed against stale text.

- Clearing a property this way removes only the attribute or element itself;
  any `xmlns:xmpDM` / `xmlns:photoshop` declaration Handpic had added earlier
  is left in place. A sidecar that is labeled (or rated via `xmpDM:good`)
  and then cleared is therefore not byte-identical to one that was never
  touched — assert on the specific properties, not full-file equality, after
  a clear.
- Follow the same two-pass-with-re-parse pattern for any future property
  pair added to a sidecar write.
- Source: `docs/plans/_archived/20260922-lightroom-xmp-flags-labels/learnings.md`,
  Steps 1 and 2.

### Label-name reads fall back configured-name-then-English, not the other way (Hit)

`LabelNames::name(&self, color)` returns the English `color` itself when the
configured name for that color is empty — callers don't need a separate
"is this customized" check. Reads match a sidecar's `xmp:Label` against the
configured name first, then against the English name, so a sidecar written
by an English-locale Lightroom still reads as the right color even when
Handpic's settings hold Japanese (or other) names.

- Source: `docs/plans/_archived/20260922-lightroom-label-names/learnings.md`, Step 1.

### quick-xml 0.42 namespace resolution: `&str`, not `&[u8]`, and a borrowed `QName` (Hit)

`ResolveResult::Bound(Namespace)`'s `as_ref()` yields `&str` — compare it
against `XMP_NS` directly, not as bytes. A helper that resolves an
attribute's namespace and returns `ResolveResult<'a>` must tie the
lifetime to the *reader*, not to a local `String` holding the name: the
only variant that borrows (`Bound`) points into the reader's own
namespace buffer, so binding the lifetime to a temporary buffer fails to
compile (or is subtly wrong if it does).

- Source: `docs/plans/_archived/20260920-robustness-cleanup/learnings.md`, Step 2.

### Bumping `SCHEMA_VERSION` can strand an old per-version column guard (Hit)

A migration guard that adds a column for "any version other than the current
one" (e.g. `ratings.label_known` at v6) breaks once a later change bumps
`SCHEMA_VERSION` again (v6 -> v7): the guard now also fires for v6 databases
that already have the column, and the `ALTER TABLE` fails.

- Key such a guard to the version that introduced the column (`version < 6`),
  not to the ever-moving current version, and re-check the versions asserted by
  the existing migration tests whenever `SCHEMA_VERSION` moves.
- The same trap applies to a table drop: `files` used to be dropped for any
  `version != SCHEMA_VERSION`, which would have thrown away every thumbnail on
  the v7 -> v8 upgrade. It is now keyed to `version < 7`, and v8 seeds the new
  `folders` table from the surviving `files` rows.
- Audit every existing guard on each bump, not just the new one: the v11
  `ratings.flag` guard was still `version != SCHEMA_VERSION` and would have
  fired for v11 databases on the v12 bump. A range key also has to exclude
  versions whose table is dropped and recreated with the column: the v12
  `files.extractor` guard is `(10..12).contains(&version)`, because
  `version < 12` would also fire for v2-v9, where `files` has just been
  recreated with the column, and the duplicate-column `ALTER` would discard the
  cache, dirty ratings included.
- Same trap on the v16 -> v17 bump: the `folders` table was introduced at v8
  and its `CREATE TABLE IF NOT EXISTS` already carries `last_viewed`, so the
  `ALTER TABLE folders ADD COLUMN last_viewed` guard has to be
  `(8..17).contains(&version)`, not `version < 17` — the latter would also
  fire for v2-v7 databases, whose `folders` table has just been created with
  the column, adding a duplicate and discarding the cache.
- A migration test fixture built with `open` has the current schema, so faking
  an older version means dropping every column added since then too, and
  every table added since then (`DROP TABLE views;` for the v22 bump). Only
  the v8+ fixtures need this: the v2-v7 fixtures create their own tables or
  drop `folders` entirely, and `CREATE TABLE IF NOT EXISTS` recreates the new
  table anyway. When a bump touches many fixtures, a regex over the previous
  bump's `DROP COLUMN ...;` line adds the new drops in one pass. The regex
  has nothing to key on in fixtures that already had the neighbouring column
  (v19 to v22 had `edge_gap`, so the v23-era `full_eye_sharp` drop had no
  `DROP COLUMN edge_gap` line to anchor to): add the new drop to every
  older fixture by hand, v19 to v22 included.
- Put a new column at the end of `CREATE TABLE`, not next to the column it
  relates to: `ALTER TABLE ADD COLUMN` always appends, so only an end
  placement keeps fresh and migrated databases in the same column order
  (v18's `eyes_ear` and `pose_*` sit after `faces_extractor`, v19's
  `eye_offset` and `edge_gap` after them). Every fixture of an older version
  then has to drop the new columns too, and the version
  asserts should read `SCHEMA_VERSION`, not a literal.
- Dropping a column is the reverse: key the `DROP COLUMN` guard to the
  versions that had it (v23 drops `folders.culling_finished` for
  `(21..23)` and `culling_manual` for `version == 22`), remove the column
  from `CREATE TABLE` and from every older fixture's drop list, and have
  the fixtures faking a version that had it add the column back with
  `ALTER TABLE ... ADD COLUMN` instead of dropping it.
- Source: `docs/plans/_archived/20260919-sharpness-cue/learnings.md`, Step 2;
  `docs/plans/_archived/20260920-app-quick-fixes/learnings.md`, Step 3;
  `docs/plans/_archived/20260924-index-extractor-version/learnings.md`, Step 1;
  `docs/plans/_archived/20260927-resume-last-viewed/learnings.md`, Step 1;
  `docs/plans/_archived/20261008-mesh-eyes-index/learnings.md`, Step 1;
  `docs/plans/_archived/20261013-auto-culling-finished/learnings.md`, Step 1.

### Bump `EXTRACTOR_VERSION`, not `SCHEMA_VERSION`, when extraction output changes (Hit)

`reconcile` treats a `files` row as valid only while its size, mtime **and**
`extractor` match. Before `files.extractor` existed, a fix to extraction (e.g.
#376) never reached existing caches: error rows in particular were never
retried.

- Bump `EXTRACTOR_VERSION` (`crates/backend/src/index.rs`) on any change to what
  `riffle_core::scan::extract` produces: ARW/DNG parsing or embedded JPEG tier
  selection (`crates/core/src/arw.rs`), which preview bytes are read
  (`crates/core/src/reader.rs`), thumbnail generation (`crates/core/src/decode.rs`,
  `crates/core/src/scan.rs`). The first pass computes the thumbnail and the
  metadata only.
- Bump `SCHEMA_VERSION` only when the table layout changes.
- An extractor bump re-extracts every row, error rows included, on the next
  scan of each folder, and keeps `ratings`. Stale rows are deleted by
  `reconcile` right away, so the folder shows placeholders until its rescan
  fills them in.
- Parallel branches each bump `EXTRACTOR_VERSION` from the same base, so
  the merge conflict is a number collision. When you rebase or resolve, take
  the next free number above main's (a branch cut at 11 whose sibling landed
  12 resolves to 13). Never keep your own number.
- An error row can carry the metadata of its failed extraction: the failure
  keeps the parsed orientation and shot, and `indexed_file` tells "no
  metadata" apart by a NULL `orientation`. Rows written before the version
  that added this also have it NULL, so `exif` stays `None` for them
  until the rescan the bump forces.
- The second pass has its own version, `FACES_VERSION`, stored as
  `files.faces_extractor`: bump it, not `EXTRACTOR_VERSION`, on a change to
  what `riffle_core::scan::extract_analysis` produces, the focus candidate cue
  (the in-focus probability's coefficients or its threshold, the mesh eye
  regions and the face mesh, the eye window, the detector:
  `crates/core/src/candidate.rs`, `eyes.rs`, `faces.rs`) or the sharpness
  score (`sharpness::score_preview`, `crates/core/src/sharpness.rs`, or what
  `extract_analysis` feeds it) or the full-size eye sharpness
  (`full_eye_sharp`: `crates/core/src/fullsharp.rs`, its `SOFT_BELOW`, the
  eye crop size, or the full-size read it makes in `scan.rs`). Only the second pass then re-runs; thumbnails
  are kept. The column and the constant keep their names although they now
  cover the score too; a change to the columns it fills (`files.eye_focus`,
  `files.sharpness`, `files.full_eye_sharp`, and `files.eyes_ear` with `files.pose_yaw` /
  `pose_pitch` / `pose_roll`, the EAR of the more closed eye and the head
  pose from the mesh the cue already runs, and `files.eye_offset` /
  `files.edge_gap`, how well that mesh sits on the face) is a
  `SCHEMA_VERSION` bump as well. The cue covers the face nearest a trusted
  AF point and, on a frame with none, the largest confident face of the
  whole preview (`eyes::judged_face`); `9` added the latter, so a frame with
  no trusted AF point now fills every one of those columns too. That branch
  costs one extra full-size RGB decode, the luma and the mesh per faced
  file, about +28 ms in release (`riffle-cli features`, `analysis_ms`); a
  file with no face pays nothing, because `whole_image_cue` returns
  `Cue::unknown()` before any decode. The face-free frame itself stays
  Neutral: the sharpness score did not separate picks from non-picks
  (folder-stratified AUC 0.58 against 0.70 needed), so do not re-propose a
  score-based Good / Low for it without a new cue or 30 or more face-free
  rejects. See
  `docs/plans/_archived/20261010-af-eye-good-bad-cascade/learnings.md`,
  Steps 2 and 3. `FACES_VERSION` 10 added the full-size eye check: a frame
  whose AF eyes measure soft at full size (`SOFT_BELOW`, pinned by a Rust-side
  test against the figure in `focus.ts`, which the test does not read) is no
  longer Good. It costs about +51 ms per measured (`Candidate`) frame in pass
  2, with 8 threads and a warm file cache; the cold-disk cost is unmeasured. A
  new field of `faces-progress` must also be copied in `applyFaceReady`
  (`focus.ts`), which copies field by field, or a frame scanned while on screen
  stays Good until the folder reloads. See
  `docs/plans/_archived/20261108-full-size-eye-check/learnings.md`, Steps 3
  and 4.
- Moving the score from the first pass to the second bumped neither version:
  the score's computation did not change, so a row the old first pass scored
  (at `FACES_VERSION`) keeps a correct score, and a row the second pass has
  not reached (`faces_extractor = 0`) gets the same score from it.
- Source: `docs/plans/_archived/20260924-index-extractor-version/learnings.md`,
  Step 1; `docs/plans/_archived/20260930-scan-priority/learnings.md`, Step 2.

### `reset_sidecars` must not rewrite a field `mark_written` guards on (Hit)

`Index::mark_written` clears `dirty` only `WHERE path = ?1 AND rating IS ?4 AND
flag = ?5 AND label IS ?6`. A judgment made inside a sidecar format switch's
window has already been queued with the value the row held, so if
`Index::reset_sidecars` rewrites any of those fields in between, the guard no
longer matches: the row stays dirty with the rewritten value and the next
folder open replays that stale value over the sidecar. `reset_sidecars` used to
zero `pick` (now `flag`), which lost a pick set during a switch.

- `reset_sidecars` nulls the stat only (`xmp_size`, `xmp_mtime_ns`); it keeps
  the judgment (`rating`, `flag`, `label`). Keep it that way for any field
  added to `mark_written`'s guard.
- Source: `docs/plans/_archived/20260920-format-switch-pick-race/learnings.md`,
  Step 1.

### The good-photo mark's rule is validated against the user's stars; know what it can claim (Hit)

`focus.ts` `photoTier` returns `"good"` or `null`. The type is kept as a union
so a tier can be added back without rewriting the `filter.ts` / `strip.ts` /
`main.ts` plumbing. The rule needs the stored `eye_focus`, `eyes_ear`, `pose`,
`eye_offset` and `edge_gap`.

- The user's star scale: 1 = no subject, 2 = likely rejected, 3 and up =
  pass. The mark can say "not a miss" (the frames it marks were 93% 3+ stars,
  no 1-star, on 180 rated frames). It says nothing about taste above 3: good vs.
  not was no different at 4+ stars. Do not describe or tune it as a quality or
  best-of-burst ranking.
- Keep `MAX_EYE_OFFSET`. Every relaxation, including skipping it when YuNet's
  eye distance is small, let a 1-star frame in. Gating on YuNet's eye distance
  fails because YuNet sees a profile where the mesh sees a face
  (`2026-09-27-a__DSC6920`).
- Keep `MIN_EDGE_GAP` as a box-plus-grown-eye-region gap. Testing the 1.25x
  mesh crop flags whole faces near the edge (only hair outside).
- The EAR is inflated on a turned, foreshortened eye. That is why the yaw cut
  is tight (|yaw| <= 35, tightened from 60), not loose.
- `GOOD_EYE_FOCUS` 0.90 has no held-out check: the rated batches hold no
  frame between 0.90 and 0.99 (or 0.772 and 0.90) that clears the other cuts
  in batch 1. Include such frames in the next rated batch before moving it.
- Burst-level technical features do not predict a kept scene: burst length
  does (>= 15 frames: 79% hold a pick). Do not rebuild a burst "keep" or
  "best" mark from sharpness / eyes / pose. The first frame of a burst also
  out-ranks any feature.
- Source: `docs/plans/_archived/20261008-burst-keep-score/learnings.md`,
  Steps 3, 5, 5b, 7.

### A learned discard score over the stored technical features did not beat `eye_focus` (Hit)

The plan trained scores (`LR/T`: logistic regression over `eye_focus`, sharpness,
EAR and pose, with and without the `af-whole` fill) on the user's 1,200 blind
"fine to sort last" labels, then confirmed the winner on 600 fresh labels
from six unseen folders. `LR/T` won the exploration and failed the
confirmation: AUC - `eye_focus` -0.013 [-0.048, +0.023], with the `PL <= 5%`
bar failing at q10 and the model-contribution bar failing at q5 and q10.

- `eye_focus` stays the only discard-leaning cue and no app change was made. Do
  not retrain a model on `eye_focus`, sharpness, EAR and pose to rank frames
  for discard. Fitting looked better than `eye_focus` in the exploration and
  did not hold on fresh folders.
- At a per-folder q5 cut (about 5% of a folder sorted last) `eye_focus`
  itself keeps `PL` near 4% on the fresh labels (picks lost: 4.07%). Whether
  that is shippable as-is is undecided. The strata were cut for `LR/T`, so it
  is only a reading. Deciding it needs its own plan with strata cut for that
  score and pre-registered bars.
- Judge a sort-last or discard cue at q5. At q10 every candidate lost more
  than 5% of the picks. The strata and bars are in the plan. A candidate's
  bar 5 (AUC against `ef`) is the only one that measures the model's own
  contribution. A clause that `ef` also meets (for example "close") says
  nothing about the model.
- Frames with no `eye_focus` (about 9-12%) are a separate case. Today they sort
  by a constant. An `af-whole` fill was level on all frames (+0.002) and
  showed a gain only on the 201 missing-face fresh labels (+0.277
  [+0.131, +0.407]); that reading was not pre-registered.
- Source: `docs/plans/_archived/20261023-discard-model/learnings.md`, Steps 6
  and 7 (Decision); `discardmodel.md`, "Results of the confirmation".

### Writing the `Both` sidecar format: one comparator, one stat source (Hit)

`sidecar::newest` is the single newest-wins comparator (larger `mtime_ns`,
ties go to whichever kind is listed first in `kinds()`, i.e. XMP). `write` uses
it to pick the label kept when `label_known` is false and the stat handed to
`mark_written`; `sidecar::reconcile_of` uses the same comparator to pick which
file's stat is compared against the stored one. Keep any future multi-file
sidecar logic going through this one comparator rather than re-deriving
"newest" locally, or a reopen right after writing can re-parse a file that
was just written.

- The per-kind write body is `write_kind`; the final stat always comes from
  `index::stat`, the same function the folder listing uses to compute
  `mtime_ns`, so the write path and the listing path cannot drift in how they
  measure a file's freshness.
- Adding a new `SidecarFormat` or kind means updating every exhaustive
  `match` on it; the compiler catches most, but at least one test's match arms
  needed a manual addition.
- Source: `docs/plans/_archived/20260925-sidecar-format-both/learnings.md`,
  Step 1.

### Folder-index eviction: lock order and where it runs (Measured)

Eviction (`commands::spawn_eviction`) runs once from `setup`, right after
`app.manage(Scans)`, on a plain `std::thread`. It holds the `Scans` lock and
then the writer lock for the whole evict + `VACUUM`, and skips if
`Scans.running` is already set — `running` is set by `start_scan` and cleared
by `finish` when the scan task ends, so `is_some()` there means "a spawned
scan task is in flight". It deliberately misses the `preparing`/`pending`
phases, which is why `is_some()` is not a general test for "a scan is
running" (use `ScansState::scanning()` for that, see below). A
`scan_folder`/`start_scan` issued meanwhile just waits. Keep the order
`Scans` then writer. The size cap counts pages in use
(`page_count - freelist_count`), since a delete only moves pages to the
freelist until `VACUUM` runs.

`Index::clear` (behind the settings modal's Clear Cache button, via
`commands::clear_index`) is the second caller of `evict_folder` and keeps the
same order: `Scans` lock first (re-checked under it after the confirmation
dialog, since the dialog is awaited with no lock held), then the writer lock.
A running scan is refused, never canceled.

- Measured (`Index::clear`'s unit test, 2 folders x 5 rows of a 200KB
  thumbnail): `VACUUM` alone gives nothing back to the file system, because
  the rebuilt database is written through the WAL — the on-disk footprint
  (`index.sqlite` + `-wal`) went from 2,121,808 B to 2,212,448 B, i.e. it
  *grew*, though `page_count` dropped by more than 10x. Ending with
  `PRAGMA wal_checkpoint(TRUNCATE)` took it to 40,960 B. Any eviction path
  that is meant to shrink the file needs that checkpoint after the `VACUUM`.
- `rusqlite`'s `pragma_query(None, "wal_checkpoint(TRUNCATE)", ..)` compiles
  but silently does nothing (it quotes the argument, so `TRUNCATE` is lost);
  use `query_row("PRAGMA wal_checkpoint(TRUNCATE)", ..)` instead. A
  `SQLITE_BUSY` from it is fine to just log and continue.
- Measured (M3 Pro, release, ignored test `vacuum_cost_on_a_100_mb_index`):
  evicting half of 5000 rows of 20.8KB thumbnails (106MB in use) and vacuuming
  took ~220ms, leaving 53MB. That a WAL reader does not error during `VACUUM`
  is reasoned, not covered by a dedicated concurrent test.
- Source: `docs/plans/_archived/20260920-app-quick-fixes/learnings.md`, Step 3;
  `docs/plans/20260920-clear-index-cache/learnings.md`, Step 1.

### `AppListing`: reuse `list_arw`'s directory listing in `scan_folder`, keyed by dir + format + mtime (Measured)

`list_arw` (first paint) and `scan_folder` both need a directory listing.
`AppListing(Mutex<Option<CachedListing>>)` in `commands.rs`, registered next
to `Scans` in `main.rs`, lets `scan_folder` reuse the listing `list_arw` just
took instead of re-running `read_dir`.

- `take_listing` is the pure reuse decision: it takes the cached listing only
  on a full match (canonical dir, sidecar format, directory mtime), and
  leaves a non-matching one in place for whichever `scan_folder` it may
  belong to, so a cache entry is consumed at most once.
- The directory mtime is read **just before** `read_dir`, not after, so a
  file added or removed during or after the listing bumps the mtime past the
  cached value and forces a re-list; that closes the window between
  `list_arw`'s `read_dir` and `scan_folder`'s `watch::set` (changes after
  `watch::set` reach the watcher instead). A failed directory stat (no mtime)
  never matches, and nothing is cached when the mtime can't be read.
- Residual weakness: a coarse mtime (FAT: 2s resolution) can let a same-tick
  change slip past the check; documented on `AppListing` itself, not solved.
- Any new consumer of a directory listing in this area should go through the
  same take-once cache rather than adding its own `read_dir`.

Source: `docs/plans/_archived/20260926-folder-open-single-listing/learnings.md`, Step 1.

### A per-tick log line can rotate other lines out of the 40 KB default (Measured)

`tauri-plugin-log`'s `DEFAULT_MAX_FILE_SIZE` is 40 KB. A line logged once per
progress tick (e.g. one per `scan-progress` event, capped at 100 ms) at
~120 bytes each can reach ~300 lines (~36 KB) for a single ~30s/5000-file
operation — nearly filling the budget on its own and rotating out other
commands' timing lines from the same run. Before adding a log line inside a
polling/progress loop, check its expected line count × size against the
plugin's cap; raise `max_file_size` on the log plugin builder in `main.rs`
rather than trying to suppress the log (gating on state the command doesn't
have is more machinery than it's worth).

- Source: `docs/plans/_archived/20260920-scan-timing-logs/learnings.md`, Step 2.

### Forward frontend timing lines through a command, not the log plugin's JS API (Inferred)

The preview path's per-page timing line is sent from the frontend to
`Handpic.log` via a dedicated `log_timing` Tauri command, not
`tauri-plugin-log`'s JS API (`window.__TAURI__.log`). That global also needs
the `log:default` capability, which `crates/app/capabilities/default.json`
does not grant, so calling it directly from the frontend would fail silently
or need a capability change. `debugLog` joins its arguments with `String`, so
a line built from a label plus a number (e.g. `zoom keypress`) forwards
correctly through the same path.

- When a frontend line needs to land in `Handpic.log`, route it through a
  command like `log_timing` rather than granting `log:default` to call the
  plugin's JS API directly. `log_frontend` in `crates/app/src/diagnostics.rs`
  (the panic hook and the `invariant!` macro live in
  `crates/backend/src/diagnostics.rs`) is the second such command, carrying
  the `uncaught-js:` and `invariant:` lines (see
  [`app-log.md`](./app-log.md)).
- Source: `docs/plans/_archived/20260921-page-latency-timing/learnings.md`,
  Step 2.

### `scan-progress` carries flushed paths, not a done-index high-water mark (Inferred)

`done` cannot tell the frontend what became readable: `run_scan`
(`crates/backend/src/index.rs`) drives `extract_all` on rayon workers that finish
in no particular order, `done` is an unordered `AtomicUsize`, and rows reach
the index in `BATCH`-sized transactions, so a file counted in `done` is not
necessarily answerable by `Index::thumbnail` yet. The only promise the backend
can make is "these paths have been committed since the previous emit", which
is what the payload's `ready: Vec<String>` carries. A path is pushed onto that
list only after its batch's `write_batch` returned `Ok`, which is why the
trailing flush after `extract_all` (the last partial batch, on a normal finish
and on cancel alike) needs a final unconditional `progress` callback —
otherwise those paths would never be reported.

- Source: `docs/plans/20260921-scan-progress-ready-paths/`, Steps 1-2.

### `Scans.running.is_some()` is not "a scan is running" (Hit)

`Scans.running` was set by `start_scan` and only ever taken by the *next*
`scan_folder`; nothing cleared it when the scan task finished. So
`running.is_some()` stayed true from the first folder open until the app quit.
It never meant "a scan is running" — it meant "a scan was started and not yet
superseded", which was fine for its original two consumers (`scan_folder`,
which joins the handle, and `spawn_eviction`, which only needs to know
whether a scan has been started at all, and skips if so) but wrong as a
guard. `clear_index` used it as one and refused
every press of `Clear Cache` after the first folder open.

- `ScansState::scanning()` (`preparing > 0 || running.is_some() ||
  !pending.is_empty()`) is now the single definition of "a scan is genuinely
  in progress". Any new guard must use it, never the inline expression. The
  scan task clears `running` itself via `ScansState::finish(scan_id)`, which
  only clears when the stored id still matches, so a superseded task does not
  wipe the newer scan's entry.
- The bug shipped green precisely because no test covered the guard's state
  after a scan ended. It is covered now (`commands.rs`'s `tests`: a finished
  scan leaves nothing in progress, a superseded scan does not clear the newer
  one, `preparing` alone counts).
- Source: `docs/plans/_archived/20260920-clear-cache-stuck-guard/learnings.md`,
  Steps 1-2.

### The scan pool's priority levels, and `set_scan_focus` drops what it cannot apply (Inferred)

`riffle_core::scan::for_each_path` builds its rayon pool with a
`spawn_handler` that lowers each worker's OS priority once, before rayon's
`run`. `run_scan` passes `Priority::BelowNormal` (Windows
`THREAD_PRIORITY_BELOW_NORMAL`, macOS `QOS_CLASS_UTILITY`, Linux nice of at
least 5), `run_faces_scan` `Priority::Lowest` (`THREAD_PRIORITY_LOWEST`,
`QOS_CLASS_BACKGROUND`, nice of at least 10); the CLI passes
`Priority::Normal`, which makes no call, so its numbers stay comparable with
`docs/humans/performance.md`.

- Why lowered at all: priority only matters under contention, so an idle
  machine still gives every worker a full core, while the viewer's `preview`
  read (tokio's blocking pool, normal priority) and the webview win when the
  user pages during a scan.
- Why not `THREAD_PRIORITY_IDLE` / `QOS_CLASS_BACKGROUND` for the first pass:
  a disk-bound pass at idle priority can starve behind any other process,
  and on macOS `BACKGROUND` also throttles disk IO. The first pass reads the
  files cold, so it stays at `BELOW_NORMAL` / `UTILITY`; only the analysis
  pass, whose reads follow the first pass's, takes `BACKGROUND`. If a Mac
  shows that pass crawling, move it to `UTILITY` too.
- `thread-priority` has no macOS QoS API, so it is a Windows-only dependency
  and the unix branches call `libc` directly (`pthread_set_qos_class_self_np`,
  `setpriority(PRIO_PROCESS, 0, ..)`, which on Linux sets the calling
  thread's nice alone). A failure to lower the priority is not an error: the
  pass runs at normal priority and `extract_all` / `extract_analysis_all`
  return the message as `Ok(Some(msg))`, which the app logs once per pass.
- Each pass builds its own `WorkQueue`; both read their hot list from the
  one `ScanFocus` handle stored in `ScansState.running`. `set_scan_focus(scan_id,
  paths)` replaces that list only when `scan_id` is the running scan's: a
  stale id (a late debounce timer after a folder switch, or a call after
  `faces-done`) is a silent no-op by design, not an error.
- `for_each_path` starts its worker loops with `ThreadPool::broadcast`, not
  `(0..threads).into_par_iter()`: `broadcast` runs the loop exactly once per
  pool thread, while a `par_iter` over the thread count may run two items on
  one thread (the second finds the queue empty) and leave a thread idle. The
  queue's lock is held only for `take_next`, never across `per_file` /
  `on_item`; `ScanFocus::set` takes only the focus lock, so the two cannot
  deadlock.
- The frontend must call `sendScanFocus()` once `start_scan` resolves, not
  only from `show()` and the strip's scroll. `ScansState.running` (and so the
  `ScanFocus` handle) is set inside `start_scan`, so an earlier call is dropped
  by the id check. Without that call, a folder reopened at its remembered file
  would not tell the scan where it is until the user moved. The paths sent
  must be the `files` entries (`list_arw` strings, built with the same
  `to_string_lossy` as the scan's `FileStat` paths), or the queue will not
  match them.
- `set_scan_focus` is a synchronous command, so it runs on the main thread
  (see "Synchronous commands run on the main thread"). `clear_index`, the
  trash commands and `spawn_eviction`'s `VACUUM` hold the `Scans` lock for
  seconds while no scan runs, so the command takes it with `try_lock` and
  drops the call when it is busy (a busy lock means no scan is taking the
  focus); a poisoned lock is recovered. Do not turn it into a blocking
  `lock()`.
- Source: `docs/plans/_archived/20260930-scan-priority/learnings.md`, Steps 4-6.

### The scan's reads are capped and yield to the preview's at the disk (Measured)

Every scan read goes through one process-wide `riffle_core::scan::ReadSlots`
(`AppReadSlots`, app state): at most `READ_SLOTS` (2) workers are inside a
file read at once, on any of the three passes, while decoding and the models
stay on each pass's threads (`scan_threads()` for pass 1, `faces_threads()`,
an eighth of the cores, for pass 2 and `embed_threads()`, a quarter, for
pass 3). `preview` holds `pause()` across its
`read_preview`, so no worker starts a new read until it returns. Pass 2's
workers also wait out a held pause at their stage boundaries (after the read,
the decode, YuNet, the face mesh, the sharpness score and the full-size read;
`scan::yield_to_pause`, one `MAX_PAUSE` budget per file),
which in the Tauri app covers only `preview`'s read; the GPUI app holds the
pause until its image is painted (`native-app.md`). The CLI and
the one-file `extract` functions keep an uncapped, never-paused `ReadSlots`,
so `docs/humans/performance.md`'s numbers stay comparable.

- The wait follows the reads in flight (the cap, at most `scan_threads()`) x ms/file: a preview read queues behind the reads
  already at the disk (uncapped, 22 x 7 ms = 155 ms; at a cap of 2, 2 x 7 ms). So the cap gives most of the gain
  and the pause little. Cold first pass of ARW files on a SATA SSD, 22 threads,
  the app closed:

  | Cap | Pause | Read p50 (ms) | Read p95 (ms) | ms/file |
  | --- | ----- | ------------- | ------------- | ------- |
  | 22  | off   | 154-156       | 159-163       | 7.2-7.3 |
  | 22  | on    | 128-129       | 137-140       | 6.5-6.6 |
  | 8   | on    | 29-57         | 46-62         | 7.5-8.1 |
  | 4   | off   | 27            | 38            | 8.0     |
  | 4   | on    | 20-25         | 30            | 7.1-8.0 |
  | 2   | on    | 14-16         | 18-19         | 6.9-7.3 |
  | 1   | on    | 11            | 18            | 14.0    |

  Cap 2 has no pause-off run; cap 4 pause-off and cap 1 have one run each.
- The 1.3 s (42.8 ms/file) first measured was inflated by the debug app reading
  the same disk. Close the app before measuring;
  `preview_during_pass_1_on_a_real_folder` refuses to run while it is alive.
- The cap was measured on a SATA SSD only. NVMe is untested (no RAW folder on
  it), and a drive that needs deep queues for throughput may lose some at a cap
  of 2. HDR PQ CR3 is untested too.
- The cap also covers `hevc::to_jpeg` until the read is split from it:
  `read_preview` ends in that decode (65-125 ms of CPU per HDR PQ CR3), so at a
  cap of 2 such a folder pays a calculated 33-63 ms/file, and `preview` on such
  a file pauses the scan for the decode as well.
- `MAX_PAUSE` (1 s): a worker honours a pause for at most that long from when
  it first sees one, then reads anyway, so a stuck or leaked pause never stalls
  the scan. The worst case is about 1 s per `acquire` (one per pass 1 file, two
  per pass 2 file) plus 1 s for all of a pass 2 file's stage waits, so about
  3 s per pass 2 file. A wait for a
  permit alone does not use that clock, or a small cap would let workers read
  through a live pause.
- Cancel is polled inside the wait (`CANCEL_TICK`, 50 ms) because the cancel
  flag is a plain `AtomicBool` with no notifier and `ScansState::cancel` has no
  handle on the slots. `acquire` then returns `None` and the worker abandons
  the file, as `extract_unless` does at its other stages.
- `preview` takes the slots from app state, not `ScansState`: it must not wait
  for the `Scans` lock, which a clear, a trash run or the eviction holds for
  seconds.
- Source: `docs/plans/_archived/20261107-preview-read-gate/learnings.md`, Step 2.

### The third pass: `embeddings` is its own table, and the pass is not "scanning" (Inferred)

`start_scan`'s task runs a third pass, `run_embed_pass`, after `faces-done`.
It embeds each RAW file's preview with the SigLIP 2 model
(`riffle_core::embed`) into the `embeddings` table and reports
`embed-progress` (`done`, `total`), then `embed-done`. It shares the scan's
cancel flag and `ScanFocus`, at `Priority::Lowest`, on `embed_threads()`. It
is skipped, with `embed-done` emitted at once, when `aiEnabled` is off, the
model file is not in place, or `embeddings_missing` lists nothing. A
JPEG-only (view-only) folder never lists anything.

- **A table of its own.** `write_batch` does `INSERT OR REPLACE` of whole
  `files` rows, so a column on `files` would be wiped by every first-pass
  rewrite. `embeddings(path, dir, size, mtime_ns, version, vec)` carries a
  `dir` column (not in the plan's sketch) so `evict_folder`, `rename_dir`
  and `reconcile` treat it like `files` and `ratings`. A row is valid while
  `version` is `EMBED_VERSION` and `size` / `mtime_ns` match the `files`
  row; `vec` is little-endian f32. `NULL` marks a file that could not be
  embedded, which is not retried until its stat or the version moves.
  `reconcile` drops the embeddings of files gone or changed. A row
  re-extracted for an `EXTRACTOR_VERSION` bump keeps its embedding. Bump
  `EMBED_VERSION` (`crates/core/src/embed.rs`) on any change to the model,
  its preprocessing or its input decode.
- **Not scanning.** `faces-done` keeps its time, so the frontend's
  `scanRunning` clears and its held operations (trash, renames, Clear Cache)
  run while the third pass may still be embedding for minutes. The pass
  therefore keeps the `running` entry (`scan_folder` cancels and joins it,
  `set_scan_focus` reaches it) but marks it in `ScansState.embedding`. With
  that mark, `scanning()` is false, and `scan_folder`'s `joined_previous`
  does not count the joined scan, since it wrote no `files` row after
  `faces-done`. `stop_embedding` cancels the pass when `aiEnabled` turns off,
  and under the `Scans` lock before `clear_index` and the renames delete or
  move rows. A batch already in flight can still land after them. `evict` and `clear` drop the embeddings of any `dir` without a
  `folders` row (`drop_stray_embeddings`), and `reconcile` drops a renamed
  file's old path, but that does not cover everything: a late batch under a
  folder whose `folders` row survives a clear (it has dirty ratings) stays,
  and so does the old path after `rename_file` until the next reconcile. The
  deferred guarded insert in `write_embeddings` (see `learnings.md` of the
  plan) would close this.
- **The model is downloaded, not bundled** (the plan's Decision 1).
  `set_ai_enabled(true)` persists `aiEnabled` and, when the file is missing,
  `ai::switch` downloads `embed::MODEL_URL` on a plain thread through
  `reqwest::blocking`. Each `install` call writes its own `MODEL_FILE.part-{n}` file under
  `app_local_data_dir()` (a shared name let a canceled download delete the new
  one's file; `AppAi::new` removes leftover `MODEL_FILE.part*` files at launch), checks `MODEL_SIZE` and `MODEL_SHA256`, and only
  then renames the file into place. Progress and failures go out as
  `ai-state`. The main window rescans the open folder when the state turns
  enabled and ready, which runs the third pass. A launch with `aiEnabled`
  on and the file missing downloads again. reqwest is built with
  `rustls-no-provider`, the same crates the updater already pulls in, so
  `fetch` installs rustls' `ring` provider first when none is set, as the
  updater does. Its blocking reads time out after 30 s without data, not
  on the whole 378 MB.
- **Genres are derived, not stored.** `genres_of(dir)` reads the folder's
  valid embeddings (`Index::embeddings`) and scores each with
  `riffle_core::genre::genres_of` on every call, so a new genre data file
  never needs a rescan; files without a valid embedding are absent and the
  frontend counts them as `Unknown`. The main window calls it after each
  `folder_entries` refresh, on `embed-done`, and on `embed-progress` while
  a `Genre` item is checked, at most one call in flight. The `Genre`
  section and its checks exist only while `aiEnabled` is on; turning it
  off clears them, and they are not part of the stored filter.
- **`outputs_of(dir, files)` feeds a destructive filter, so it never
  guesses.** The `No output` filter section can be piped straight into
  `Move to Trash…`, so a wrong `no` trashes a file that has an export.
  `output.rs` therefore does not reuse `read_listing` or `listing::is_file`,
  which swallow per-entry errors (`entries.flatten()`, `kind` returning
  `None`); it propagates every read error. A missing `dir` is `Ok(None)`, any
  other failure to read it is `Err`, and a dangling symlink named `Output` is
  `Err`. The frontend's `readOutputs` turns only its own rejection into "no
  answer" and puts the failure on `outputsFailure`, its own `renderMeta` line,
  so an unreadable export folder never fails the folder open. Known
  over-matches (`X_1.jpg` also counts for `X.DNG`; any extension counts) are
  accepted because they only keep a file out of `No output`. The symlink
  tests are `#[cfg(unix)]`, so Windows does not run them.
  Source: `docs/plans/_archived/20261106-filter-by-output/learnings.md`, Step 1 and Step 2.
- **The ranking is derived too; only its weights are stored.**
  `ranking_of(dir)` reads `Index::rank_rows` (every `files` row of the
  folder with its flag, capture time, the pass-2 values when the row is at
  `FACES_VERSION`, and the valid embedding) and ranks each burst with
  `riffle_core::rank::rank` on every call. `rank.rs` forms the bursts as
  `burst.ts` `groupBursts` does (capture order, 1000 ms gap), so the
  frontend's burst ids and the backend's bursts agree; the `recommended`
  sort orders by the frontend's burst id, then the score. The shared
  weights and the per-genre offsets (`offsets`, one 44-vector per top-level
  genre of `genre::top_level`; `rank` adds the burst's `genre::of_burst`
  offset, none for `Unknown`; `Weights::of` itself ignores `genre::UNKNOWN`,
  so a `Weights` that carries an `Unknown` key still ranks unknown bursts
  with the shared weights, and `Stored::parse` drops it too) live in
  `ranking.json` under the app's local
  data dir with `RANK_VERSION` and `EMBED_VERSION`; a file at other
  versions, or none, means the prior (`crates/core/ranking/siglip2.json`,
  written by the plan's `rank_prior.py`, no offsets), so a version-1 file
  from before the offsets reads as the prior until the next refit (the
  startup one rebuilds it). `Stored::parse` drops an offset whose label is
  no longer a top-level genre or whose length is not `COLUMNS`, not the
  file. The `ranking fit:` log line ends with the bursts per genre
  (`per genre: sports 120, snap 9`). `refit_ranking` refits on `Index::rank_dirs` (the
  folders holding a `views` row or a reject, `RANK_DIRS`), one folder per
  reader lock, and emits `ranking-updated`, which `main.ts` answers with
  `refreshRanks` and the settings with the model's size. The same fit runs
  in the background, while `aiEnabled` is on, from `refit_in_background`:
  once in `setup` (after `AppAi` / `AppRank` are managed) and on
  `refit_ranking_in_background`, which `openDirectory` invokes when
  `viewed.ts`'s `switchesFolder` says the open folder changed from one
  folder to another (not the first open after launch, which startup covers,
  nor a reopen of the same folder; a rename of the open folder counts as a
  switch). `AppRank.queue` (`Queue`) runs one background fit at a time and
  collapses the requests made meanwhile into one rerun after it, so the
  judgments made during a fit are fit on one fit later rather than dropped;
  a rerun after `aiEnabled` turned off skips the fit. The fit keeps the prior's PCA
  basis and standardization and pulls the weights toward the prior's at
  the prior's own `1 / (2 C)`, and each genre's offset toward zero at
  `OFFSET_SHRINK` times that, so a genre with few judgments ranks as the
  shared weights do; the stored file is 44 weights plus 44 per genre, not a
  basis. `evict`'s 30-day age rule skips a folder `RANK_DIRS` lists (its
  rows are the fit's training data; the one SQL fragment keeps the
  exemption and the training set from drifting apart) while its size cap
  does not, and `clear` drops everything. A pick-only folder is not read
  (no negative to pair against) and so not spared.
- **`views` feeds the fit per file.** `main.ts` records a file as viewed
  once its preview has stayed on screen for `viewed.ts`'s `VIEW_DWELL_MS`
  (500 ms, `dwellTimer`), measured from the worker `message` handler's
  `draw()` of the current `seq`'s bitmap, not from `show()`: the preview
  request and decode take 10-100 ms and a held key repeats every ~30 ms,
  so a clock started at `show()` would count frames whose bitmap never
  reached the screen. `show()`, `openDirectory` and an emptied list cancel
  the clock. A failed preview (`failPreview`) does not count: the user saw
  an error, not the photo, and such a file has no embedding to rank. Nor
  do strip thumbnails, the compare pane, a provisional file before the
  resume target resolves, or a view-only folder. It sends
  the paths in batches through `viewed.ts`'s `viewedWriter`
  (`mark_viewed`), each path once per folder (`viewedFiles`, loaded from
  `viewed_files`). `Index::rank_rows` joins `views`, so `RankRow.viewed`
  becomes `Labeled.viewed`: an unflagged frame is a negative (weight 1)
  only when its file was viewed, a reject (`REJECT_WEIGHT`) whether viewed
  or not, and an unflagged unviewed frame is ignored. This replaced the
  folder-level `Culling Finished` mark (schema v21 / v22, dropped at v23),
  which let one unviewed frame keep a whole folder out and a hand-set mark
  count frames never shown. `views` is its own table, not a `files`
  column, because `reconcile` deletes and pass 1 `INSERT OR REPLACE`s a
  `files` row on any size / mtime / `EXTRACTOR_VERSION` change; it
  survives eviction, `rename_dir` / `rename_file` carry it, and only
  `clear` drops it. Since it is never pruned, a viewed folder is spared by
  the age rule for good; the size cap and `clear` still reclaim it.
  Test fixtures for this: `Index::set_rating` always writes `dirty = 1` and
  `evict_folder` keeps dirty `ratings` rows, so a reject the eviction may drop
  needs a clean `ratings` row inserted with SQL; `Index::clear` only walks
  `folders` rows, so a `write_batch` without a `folders` row (no `reconcile` /
  `set_opened_at`) survives a `clear`; and to show that eviction keeps `views`,
  evict through the size cap (`max_bytes: 0`), since the age rule now spares a
  folder holding a `views` row.
- **Threads: a quarter of the cores** (`embed_threads()`), not pass 1's
  `scan_threads()` (Measured; pass 2 runs on fewer still, an eighth, `faces_threads()`,
  since the GPUI port measured it costing paging +9.4 ms on ARW and +89 ms on RAF
  at `scan_threads()` and +13 ms on RAF at a quarter, see `native-app.md`). Priority does not keep the viewer's reads
  fast against 22 busy workers on 24 logical cores: shared physical cores
  (SMT) and an all-core clock do not care about thread priority. On
  `2026-09-19` (2134 ARW), a preview read + full decode on a
  normal-priority thread took a median 10 ms idle and 27-34 ms during pass 2
  on 22 threads. During pass 3 it took 33 ms on 22 threads (41 s for the
  pass), 22 ms on 11 (56 s) and 15 ms on 6 (70 s). Memory does not set the
  count: the model's load sets the process peak (745-812 MB in that run),
  and the workers' buffers add about 220 MB from 1 to 24 threads
  (`riffle-cli embed`). The harness is the ignored test
  `embed_pass_on_a_real_folder` in `commands.rs`.
- Source: [image-embeddings learnings](../plans/_archived/20261012-image-embeddings/learnings.md),
  Step 3.

### `focus_crop`'s header carries the full JPEG size (Hit)

`riffle_core::partial::Crop` carries the full JPEG's `image_width`/
`image_height`. The `focus_crop` header encodes them as two `u16`s at offsets
28/30 under kind tag 5 (`CROP_KIND_RGBA_V3`); the next header change takes tag
6. The zoom placeholder (`crates/app/ui/src/zoom.ts`) falls back to the sensor
size until the current file's crop arrives.

- Source: `docs/plans/_archived/20260920-app-quick-fixes/learnings.md`, Step 1.

### An on-demand per-file command takes its request id from the frontend and stores the latest, not the max (Inferred)

`eyes_of` (the meta pane's closed-eyes judgment) is a command that runs a slow
job for the shown file outside the scan. The frontend's cache (`EyesCache`)
counts the request ids, never resetting the count on `clear`. The backend
(`EyesRequests`) stores the latest id unconditionally rather than the maximum,
so a window reload that restarts the count at 1 is not superseded forever.
The job checks the id between stages (after the decode, before the
detection, and before the crop decode and model) and stops early.

- The response carries a `superseded` flag next to the result. The frontend
  frees its slot on it without caching the early stop as "unknown".
- Read, decode and detection errors, and a JPEG (`raw_only`), are an `Err`.
  The frontend logs it and caches it as unknown, the `faces_of` / `NO_FACES`
  pattern.
- With one request in flight, the backend supersede fires only after a
  `clear` (folder open, `refreshEntries`) lets a new request start while the
  old one still runs.
- Source: [closed-eyes-detection learnings, Step 3](../plans/_archived/20261007-closed-eyes-detection/learnings.md).

### This crate is on Rust edition 2021: no `if ... && let` chains (Hit)

`if let Some(x) = a && let Some(y) = b` does not compile here; nest the
`if let`s.

- Source: `docs/plans/_archived/20260920-app-quick-fixes/learnings.md`, Step 2.

### The MCP server runs on Tauri's runtime; `rmcp` 3.4 macro traps (Hit)

The backend's `crates/backend/src/mcp.rs` serves `rmcp`'s
`StreamableHttpService` through `axum::serve`, spawned with a bare
`tokio::spawn`; the Tauri app's `crates/app/src/mcp.rs` calls it on
`tauri::async_runtime` (Tauri's tokio multi-thread runtime), so no dedicated
runtime thread was needed. `tokio` is a backend dependency for its `net`,
`rt`, `sync` and `time` features, and the backend's tests drive the server on
a current-thread runtime of their own, so no tokio `macros` feature.

- `#[tool_router]` on an impl with no `#[tool]` fn does not compile in 3.4.1;
  an intentionally empty router needs `#[tool_router(allow_empty)]`.
- `#[tool_handler]` without `router = ...` calls `Self::tool_router()` per
  request, so the handler needs no `tool_router` field (an unread one trips
  `dead_code` under `-D warnings`).
- `ServerInfo` is deprecated in 3.4 in favor of `ServerConfig`.
- `CallToolResult::structured(value)` already adds a text block with the same
  JSON; do not add another. A failure the client should see is
  `CallToolResult::error`, not a protocol error.
- Tool arguments are `Parameters<T>` with `#[derive(JsonSchema)]` and
  `#[schemars(crate = "rmcp::schemars")]`, so no direct `schemars` dependency.
  To tell an omitted field (keep) from `null` (clear), as `set_judgment`'s
  `label` does, use `Option<Option<T>>` with a `deserialize_with` helper.
- `stop` bounds the graceful shutdown with a 2 s timeout and then aborts the
  task, so a lingering SSE stream cannot hold up the exit. The status is a
  `tokio::sync::Mutex` held across the bind, so two quick toggles cannot bind
  twice; a failed bind (port in use) only sets the `error` of `mcp-state`.
- Source: `docs/plans/_archived/20260924-mcp-companion/learnings.md`, Step 1.

### The MCP bridge: never hold the status lock, always reply (Inferred)

MCP tools that need view state ask the main window: Rust emits `mcp-request`
`{ id, kind, args }` with `emit_to("main", ..)`, waits up to 5 s on a
`oneshot` keyed by the id, and the sync `mcp_reply` command completes it.

- The pending senders live in `Bridge`, beside (not inside) the server's
  status mutex, so `mcp_reply` never waits on a toggle that holds that mutex
  across a bind or the 2 s shutdown wait.
- `Bridge` takes its `send` as a closure, so the tests build it without an
  `AppHandle` or `MockRuntime`; `Companion` holds the bridge and the index
  reader for the same reason.
- The frontend listener (`crates/app/ui/src/companion.ts` via `main.ts`) must
  catch every error and reply `ok: false`; an unanswered request costs the
  client the whole 5 s timeout.
- The folder the frontend reports is the one picked, not canonicalized, while
  the paths come from the canonical folder: compare canonicalized parents to
  decide "in the open folder", and use canonical parent + file name as the
  index key.
- Writes reuse `record()`, the tail of `judge()` (changes, undo entry,
  `commit()`), so a tool writes the same sidecar bytes a key press does;
  auto-advance lives in `runAction`, which the bridge never goes through.
- Source: `docs/plans/_archived/20260924-mcp-companion/learnings.md`, Steps 2-5.

## Frontend (`crates/app/ui`, Vite+)

For colors, radii, borders and the shared control classes, see
[`ui-styling.md`](./ui-styling.md).

### Undo must re-anchor conditionally, not unconditionally (Hit)

`refilter(anchor)` re-anchors the view on the given file when it is still
visible under the current filter, but anchoring on a file the filter now
hides moves the current file to a neighbor instead. So the shared
apply-and-invoke `commit(...)` behind judge and undo applies the state first,
then evaluates an optional anchor thunk against the new state: it anchors on
the target file only when it still passes the filter, and otherwise leaves the
current view alone.

- Why: anchoring on the just-undone file after every undo silently jumps the
  view when that file no longer passes the active filter.
- Source: `docs/plans/_archived/20260919-undo-judgments/learnings.md`, Step 1.
- A single-file undo / redo that moves the focus (`path !== shownPath`) also
  collapses the selection to that file, via `restore` in
  `crates/app/ui/src/selection.ts`. In compare mode that gate is not enough:
  `judge()` records `compareActivePath`, which a pane click can change without
  moving the strip's `index`, so the collapse could drop the multi-selection
  `loadCompare()` still uses as the comparison set. `step()`
  (`crates/app/ui/src/main.ts`) therefore skips the collapse while
  `comparing` is true.
  Source: `docs/plans/_archived/20260928-undo-selection/learnings.md`,
  Round 1 review.
- `commit(...)` now takes a *list* of changes (one per file in a batch), not
  a single change: it applies every change locally, refilters once, then
  sends one `set_rating` per file, reverting only that file's own change on
  failure. A batch of more than one keeps the currently shown file current
  (anchor on the shown file) and reports `Undid/Redid N files`; a one-file
  batch keeps the re-anchor-on-the-file behavior above.
- A failed file is spliced out of its batch; the batch itself is dropped from
  undo/redo history only once every file in it has failed. Only files that
  actually wrote successfully stay undoable, and undo never rewrites a file
  whose write failed and was already reverted. The same drop-when-empty rule
  applies to trash: a batch is removed from history only when every file in
  it was trashed, and `step()` skips paths no longer in `allFiles` for a
  partly-trashed batch.
- Source: `docs/plans/_archived/20260921-burst-grouping/learnings.md`, Step 3
  (`commit`, `forgetOnFail`, reject-rest in `crates/app/ui/src/main.ts`).

### `ErrorList`'s `Map` keeps a superseded entry in its original position (Hit)

Re-`set`ting an existing key on a `Map` used as `ErrorList`'s backing store
does not move that entry to the end; a superseded error stays where it was
first inserted, not where it was last updated.

- Why: don't assume replacing a `Map` entry reorders it — display order
  relying on "most recent first/last" needs an explicit ordering field, not
  `Map` iteration order.
- Source: `docs/plans/_archived/20260920-sidecar-error-display/learnings.md`,
  Step 1 (pinned by `errors.test.ts`).

### A judgment's own move must not double up with `refilter`'s move (Inferred)

`judge` returns how many targets it addressed. The keydown handler records the
current path *before* the switch/commit runs, then — only if that path is
still the current file afterward — calls `move(1)` for auto-advance.

- Why: `commit`'s `refilter` step already moves the view off a file that
  drops out of the active filter as a result of the judgment. If the
  keydown handler also unconditionally called `move(1)`, a filtered-out file
  would advance twice. Checking "is the just-judged file still current"
  before moving is what prevents the double skip.
- `move` already clamps at the last file, so the auto-advance call does not
  need its own bounds check.
- Source: `docs/plans/_archived/20260919-auto-advance/learnings.md`, Step 3.

- `judge` returns the number of targets it addressed, changed or not (0 only
  with no files or in a view-only folder); `record` keeps returning whether
  anything changed. Auto-advance fires only when that count is 1 (decided by
  the pure `shouldAdvance` in `advance.ts`), i.e. only for a single-file,
  non-batch judgment, and also when the file already held the value, which
  writes nothing. An unchanged file never refilters, so the repeat press
  always takes the `move(1)` branch. Sources:
  `docs/plans/_archived/20260922-strip-multi-select/learnings.md`, Step 3;
  `docs/plans/_archived/20261012-repeat-judgment-advance/learnings.md`, Step 1.

### A derived-state refresh has to run even when `refilter` short-circuits (Hit)

`refilter` returns early when the visible file list did not change, so per-file
UI state derived from the list (e.g. the burst bands' `applyBursts()`)
cannot rely on `refilter` alone to recompute it.

- Call the derivation explicitly wherever the underlying data is refreshed
  (`refreshEntries`), and also inside `refilter` after `setFiles`, which clears
  the strip's per-index store.
- Source: `docs/plans/_archived/20260919-sharpness-cue/learnings.md`, Step 3.

### A backend answer that is a subset of the request cannot tell "no" from "never asked" (Hit)

`outputs_of` returns only the files that have an export, so a file missing from
`fileOutputs` is ambiguous. Trash undo appends the restored files to `allFiles`
and refilters before `resync("restore")`, and that resync defers for a whole
scan. In that window a restored file with an export would read `no` and be
offered to the trash again.

- Give the lookup (`outputOf`) the listing the answer was made for, and answer
  `undefined` for a file outside it. An unknown file passes no checked `Output`
  item, so it is never offered as `No output`.
- `setOutputSection` deliberately does not `refilter()`, unlike
  `setGenreSection(false)`. In `openDirectory` it runs before `files`, `index`
  and `openDir` are switched, so `refilter` would call `show()` and could write
  the new folder's file as the old folder's last viewed. In `resync` it would
  drop `previous`. Call it only where a filter follows on the next lines.
- `outputs_of` needs the listing, so it cannot sit in the flat `Promise.all`
  with `list_arw`. Put `listed.then(readOutputs)` next to the `list_arw` promise
  in both `openDirectory` and `resync`.
- Source: `docs/plans/_archived/20261106-filter-by-output/learnings.md`, Step 2.

### Give the current folder one token, not one counter per feature (Hit, repeatedly)

Every async path that can outlive the folder it started for — a preview load, a
thumbnail fetch, a background scan, an open by drop — has to ask, when its
result comes back, whether that folder is still the current one. This repo
re-derived the same guard four times under four names (`seq`, `generation`,
`folderToken` / `openDir`, `dropCounter`), and every one was caught in review
rather than by a test.

- Why: any `await` or callback across the IPC boundary can resolve after the
  user has picked, dropped or paged to something else, and the frontend has no
  other signal that the result is now stale.
- When you add an async path that depends on "the current folder", check the
  existing token for the current open operation instead of minting a new
  counter, and check it before applying the result, not before starting.
- The four instances:
  `docs/plans/_archived/20260918-thumbnail-cache-filmstrip/learnings.md`
  (Steps 2, 5, 6, 7).
- Two corollaries from Phase 6's review (Step 5): comparing the directory
  string is **not** enough, because reopening the same folder is a new open
  with the same `dir` — `refreshEntries` checks `dir !== openDir ||
  token !== folderToken` for that reason. And a backend **event** listener,
  which is registered once and outlives every folder, needs the same kind of
  guard on its payload before it touches the UI; the `sidecar-error` listener
  drops a payload whose path is not in the current folder's index.
- When an async path clears its own in-flight guard before an `await` and
  re-checks the token after it resolves, the post-await stale branch must
  `return` without re-issuing the request: whatever invalidated it (the user
  paging again) already started its own fresh request from the normal trigger
  path, so re-issuing makes a duplicate in-flight request for a superseded
  token. Found in the preview `seq` check of `requestPreview()` in
  `crates/app/ui/src/main.ts`. Source:
  `docs/plans/_archived/20260924-linux-preview-pixel-limit/learnings.md`,
  Step 1.
- A per-path cache with its own in-flight tracking is a legitimate deviation
  from "drop on token mismatch": `FaceCache` in `crates/app/ui/src/faces.ts`
  keys its cache and in-flight `Set<string>` by file path, not by the folder
  token, because dropping a response on a page-away-and-back would leave the
  only in-flight request for that path discarded with nothing to redraw the
  boxes later. Folder staleness is instead handled by a generation counter
  that `clear()` bumps (folder open, format switch, post-resync
  `refreshEntries`); a response tagged with a stale generation is dropped, and
  the draw itself still checks that its path is the current one before
  painting. Source:
  `docs/plans/_archived/20260924-face-catch-state/learnings.md`, Step 5.
- A per-file failure flag in the preview UI (for example "this file's preview
  failed", which drives the empty-state overlay) must be stored as the `seq`
  that failed (`previewFailedSeq`) and compared with the current `seq` at
  render time. A plain boolean reset only in `show()` is not enough:
  `seq` is also bumped by `refilter`'s empty branch and `openDirectory`, which
  skip `show()`, so a boolean could leave the overlay up for a different
  file. Both failure branches share `failPreview()` in
  `crates/app/ui/src/main.ts`, which sets the flag and closes and nulls
  `shown` so a failed file never keeps the previous file's bitmap on screen.
  Source: `docs/plans/_archived/20261002-preview-failure-clears-stale-image/learnings.md`,
  Step 1.
- Source: `docs/plans/_archived/20260918-ratings-xmp-sidecars/learnings.md`,
  Step 5.

### Two folder paths: `openDirectory` resets, `resync` keeps state (Inferred)

`main.ts` has two ways to point the UI at what is on disk, and a new "the
folder changed" event has to pick the right one.

- `openDirectory(folder, token)` is the **reset** path: it mints a folder
  token, clears `entries`, `ratings`, `picks`, `labels`, `sharpness`,
  `touched`, `history` and `errors`, sets `index = 0` and scrolls the strip
  back to the top. Use it when the *judgments* are no longer valid — the
  `sidecar-format` and `index-cleared` listeners, which run after the backend
  reset the index.
- `resync()` is the **keep-state** path: the same open, so no new token; it
  re-lists the folder, runs the same `scan_folder` -> `start_scan` diff
  through the shared `startScan` helper, and re-anchors with
  `refilter(currentPath, true)`, which keeps the strip's scroll offset. Use it
  when only the *files* may have changed — the window focus,
  `File > Reload Folder`, the tree's `Refresh` on the open folder (trigger
  `refresh`) and `folder-changed` (the watcher) triggers.
- `resync` defers to `faces-done` (the end of the second pass) while a scan
  runs (`scanRunning`), because `scan_folder` cancels and joins the running
  scan first; a focus change during a 5000-file first scan would otherwise
  restart it. Repeat triggers collapse into the single `resyncPending` slot, which keeps the first deferred trigger (`??=`), so the drained run is logged with `deferred=true` and the trigger that first asked for it.
  It defers the same way while an operation held by `idle.ts`'s `IdleGate`
  (Move to Trash, the two renames) has its invoke out
  (`idle.inFlight`, set by `settleIdle`), whose settle drains it: the
  confirm dialog closing refocuses the window, and a rescan started then
  would make the command's own scan guard refuse it.
- Move to Trash is the held operation that cancels the scan (`cancelsScan`
  in `idle.ts`): its files are about to go, so scanning them is wasted. It is
  still held, because `trash_files` refuses while `scanning()`, which stays
  true until the canceled task's `finish`, just before `faces-done`, so
  `faces-done` remains the one point that runs it. `cancel_scan` only sets
  the flag of the scan whose id it is given, short of the third pass (which
  `stop_embedding` covers), and the task then ends the usual way: `scan-done`
  with the partial total, `faces-done`, an empty `embed-done`.
  Pressing it sets `resyncPending ??= "scan-canceled"`, so the canceled scan's
  `faces-done` leads to a rescan whatever becomes of the held move: run, it
  re-defers behind `idle.inFlight` and `trashed()`'s `resync("trash")`
  collapses into it, so one rescan follows the move; replaced by a later
  `request` or dropped by its guards, the rescan still finishes the folder's
  scan; a folder switch clears the slot and scans anyway.
  A press while `scan_folder` is in flight has no id to cancel (the current
  `scanId` is null or stale, which the backend ignores), so `startScan` asks
  again after `start_scan` resolves when `cancelsScan(idle.waiting)`, and
  `cancel` needs no case for `pending` entries.
- On Windows `trash_files` and `trash_rejected_redo` move through
  `trash::run_chunked`: `CHUNK` RAWs in one `::trash::delete_all`, then their
  sidecars in another. A shell operation costs about 200 ms whatever it moves,
  so this is about 30 times faster (the trash-over-scan plan's
  `trash-bench.md`). Elsewhere `delete_all` gains nothing (Linux) or the mover
  must report each file's location (macOS), so they stay on `trash::run`.
  Three rules hold it together:
  - A chunk's `Err` does not mean nothing moved. The shell goes on past a locked
    file and then reports the operation aborted, so `move_chunk` records as
    moved whatever is gone afterwards, or Undo would lose it, and retries only
    what is left, one by one, so each `Failure` names its file.
  - A path that does not exist makes `delete_all` fail before moving anything,
    so a file already gone is taken out first and fails alone with `GONE`.
  - `moved` stays in `run`'s order, each RAW followed by its sidecars, though
    the moves are not: `restore`'s `raw_back` and `redo`'s regrouping read it.
  - Do not trust the Recycle Bin's size or item count to tell that a test
    cleaned up. trash 5.2.9's Windows `restore_all` (so Undo too) moves the `$R`
    file back and leaves its `$I` metadata file in `$Recycle.Bin`, and the shell
    does not list `$I` files. A bench run left 23,450 of them with the count and
    size exact; only a free-space gap showed it. Count the `$Recycle.Bin`
    folder itself, and remove only the `$I` files whose recorded original path
    is your test folder. Source:
    `docs/plans/_archived/20261106-trash-over-scan/learnings.md`, Step 4, and
    `trash-bench.md`.
- `trash_files` and `trash_rejected_redo` stream `trash-progress { op, dir,
  done, total }` per group from `trash::run`'s callback, or per chunk from
  `run_chunked`'s, a group whose RAW
  failed counting as done (`dir` is empty for a redo, whose run can span
  folders; `op` is `"trash"`). `trash_rejected_undo` has none: `trash::restore`
  has no callback. The event carries no run id: the `Scans` lock runs such
  operations one at a time and the frontend starts one at a time. `main.ts`
  keeps the line in the module-level `trashing`, which `renderMeta` shows like
  `sequencing`: set when the invoke starts, `null` when it settles, not tied to
  the folder, so a reopen mid-move keeps it. The listener drops an event while
  `trashing` is `null`, since nothing orders the last event against the
  invoke's resolve, and a late one would bring the line back for good, and
  with it the guard below.
- `trashing !== null` is also what refuses a second Move to Trash:
  `confirmTrashFiles` opens no dialog, `contextMenuGroups` greys the item
  (`MenuItem.disabled`), and the held closure of `trashFilesIn` and of
  `redoTrash` return without invoking, so the press neither reaches the
  backend nor gets held. `trashFlow` is not enough: `closeTrashDialog()` ends
  the flow at once when the move is held, so it is idle through the whole run.
  Only the invoke's own `.then` / `.catch` clear the flag, not `done()`, which
  also runs when the guards drop the move and would clear another's flag. A
  held trash is still replaced by a later request as before.
- `Delete Permanently…` is the same operation with another mover, so it takes
  every guard above: `delete_files` is `trash_files`'s shape on `trash::run`
  with `trash::remove_one`, never `run_chunked` (a removal pays no shell
  operation), `trash-progress` with `op: "delete"`, the `trashing` flag for
  either, the `cancelsScan` label `"Delete Permanently"`, and `trashed()`'s
  `resync("trash")`, whose `reconcile` drops the index rows. It records no run,
  so `summary.run_id` is `null`, which pushes no history entry and clears no
  redo. `trashed()` therefore prunes the deleted paths from the judgment
  batches of `history` and `redoable` itself (`withoutPaths`, then
  `isEmptyBatch`), as no trash entry sits above them to guard an undo. The
  backend keeps a `.xmp` that a RAW of the listing outside the selection still
  shares by stem (`delete_for_good`'s `files`), and the dialog says so. A
  `.xmp` shared by several selected RAWs is taken out of their groups and
  removed after `run`, through the remover (a failure is reported), only once
  every one of them is in `summary.moved`: a RAW that failed keeps its sidecar,
  and a fully successful delete reports no false failure from the second group
  hitting NotFound. Do not put it back in the groups. `chosen` is built from the
  groups, not the paths, so a refused foreign path never stands for a kept RAW.
  It is on purpose not in `shortcuts.rs` or `mcp.rs`.
- It shares `trash-dialog` too (`dialogText`, `TrashTarget.mode`), but no key may
  say yes to it. The dialog focuses Cancel, and `trashKeydown` swallows Enter
  and Space by `event.key`, not `keyName`, whose `shift+enter` is a different
  name for a key that clicks the button just the same, plus any `event.repeat`,
  a key held from before the dialog opened. `runTrash` takes a click only when
  `confirmsDelete` says so, `detail` exactly 1: 0 is what a keyboard-made click
  carries, so a WebView that clicks on Space's keyup despite the
  `preventDefault` still deletes nothing, and 2 or more is the second click of
  a double-click on the menu item, which can land on the run button the first
  click's dialog put under the cursor. Tab can reach
  the run button; that is harmless for the same reason.
- `IdleGate.request(label, run, cancel?)` returns whether it held `run`; the
  optional `cancel` fires when the held operation is replaced by a later
  `request` or dropped by `discard()`, never when `drain()` runs it. The two
  renames pass one (`holdRename` in `main.ts`) that reverts the tree row's
  or strip cell's pending name (`markPending` / `clearPending`) and notes
  `Rename… was canceled`; a pending cell confirmed back to its real name
  calls `idle.discard()` through the views' `onCancelPending`. `request`
  itself calls `discard()` first, so a held operation is canceled whenever
  any later request arrives, busy or not. `holdRename` skips the
  `Rename… was canceled` note when the replacing request renames the same
  path (re-editing a pending cell to a different name), since it would
  contradict the `waiting for the scan to finish` line drawn right after it;
  every other replacement, the folder switch and a user-initiated cancel
  still note it. `openDirectory`'s `idle.discard()` fires the cancel note,
  but the `show()` / `setStatus()` that follow clear it, so `openDirectory`
  re-sets it after them when it dropped a held rename; the first preview
  decode's `setStatus()` can still wipe it within a frame or two (the
  existing transient-note limitation, left as is). Source:
  `docs/plans/_archived/20261006-pending-rename-display/learnings.md`, Step 1.
  Two guards keep a rename that has left the gate from colliding with
  another: an inline edit is refused with a status note while the path's
  `rename_folder` / `rename_file` invoke is in flight (`RenamesInFlight`,
  fed by each view's `renameStarted` / `renameSettled`; for a folder, also
  anything under it), and a pending mark is cleared only by the rename that
  set it (`markPending` returns the mark, `PendingRename.clear` takes it, and
  `holdRename` hands its own clear to `run`). A rename still *held* behind
  the scan stays re-editable.
- Strip cell CSS: `.cell span` positions every span in a cell absolutely, so
  a span nested inside the name span (such as the pending clock icon's
  wrapper) needs `position: static; width: auto` back
  (`.cell span.name span.pending-icon`). Prepend the icon rather than append
  it, so the name's ellipsis never clips it. Source:
  `docs/plans/_archived/20261006-pending-rename-display/learnings.md`, Step 1.
  The same applies to the bottom-left `.cell span.marks` flex row (face and
  sparkles icons): its child spans need `position: static; width: auto;
  padding: 0`, and the `[hidden] { display: none }` rule belongs on the row's
  children so a hidden face icon leaves no gap before the sparkles. The
  recommended-frame icons are applied by `applyRecommended()` in `refilter`
  (the one place that rebuilds the strip, since `strip.setFiles` clears the
  per-index sets), `refreshRanks` and `setRanking`'s off branch; a new
  per-index strip mark must be re-applied in `refilter`. Source:
  `docs/plans/_archived/20261014-recommended-icon/learnings.md`, Step 1.
- The scan progress bar (`#scan-progress`) is shown only from `openDirectory`
  (`setScanProgress("0%")`), never from `startScan`: every `resync()` ends in
  `startScan`, so showing it there would flash the bar on each focus / watcher
  resync. Updates arrive on `scan-progress`; `scan-done` and `startScan`'s
  catch hide it. All of this goes through the one `setScanProgress(width | null)`.
  Hiding resets the fill to `0%` so the next scan never flashes the previous
  fill. Source: `docs/plans/_archived/20261005-scan-loading-feedback/learnings.md`,
  Step 1.
- Rename's success handler must return the reopen's `openDirectory(...)`
  promise so `settleIdle`'s `.finally` (which calls `drainResync()`) runs
  after the reopen lands, not before — otherwise a rescan deferred during
  the invoke drains too early and runs `list_arw` on the old, renamed-away
  path.
- `settings.ts` owns a separate `IdleGate` (`clearGate`) for Clear Cache:
  `setScanRunning(false)` drains it before the scan-end `index_size`
  refresh, so a drained clear wins over that refresh and the button stays
  enabled while a clear is held (a second press replaces the held clear,
  same one-slot rule as the rename/trash gate).
- Do not anchor a post-mutation rescan with a parameter carrying "where to
  restore the view" (e.g. a path/index snapshot taken at call time): a scan
  can start while the mutating command is still in flight (window
  focus/blur firing `resync()`), and an anchor applied unconditionally
  whenever that deferred rescan later drains ignores anywhere the user has
  since moved. Patch the affected arrays (`allFiles`/`files`/`fileIndex`,
  etc.) in place at the mutation's resolve time instead, so the current
  view is already correct whether the rescan runs immediately or later —
  `resync()` itself then needs no anchor parameter.
- Source: `docs/plans/_archived/20260928-rename-from-tree-and-strip/learnings.md`, Step 4 (round 2).

### The backend reports through callbacks; the app owns the events (Inferred)

`riffle-backend` also serves `crates/native`, so it never names a Tauri event or
an `AppHandle`. A module of the app wraps it: `crates/app/src/watch.rs` hands
`riffle_backend::watch::Watch::spawn` a closure that emits `folder-changed`
`{dir}`, and its `set` / `release_under` read `app.state::<Watch>()` so the call
sites keep passing the handle.

- Rule: an event name and its payload stay in `crates/app`.

### The folder watcher cannot loop on the app's own sidecar writes (Inferred)

`crates/backend/src/watch.rs` watches the folder `scan_folder` canonicalizes
(non-recursively, set from inside `scan_folder` so the watched folder can never
diverge from the indexed one) and `crates/app/src/watch.rs` emits
`folder-changed` 500 ms after the last event of a burst; `main.ts` turns that into `resync()` when the payload's `dir`
is still `openDir`. Two things keep the app's own sidecar writes from feeding
back into it:

- `triggers()` drops an event whose paths are *all* sidecars (`*.xmp`,
  `*.arw.dop`, either format, case-insensitive) or `riffle_core::copy::TEMP_SUFFIX`
  temporaries, which is exactly what `sidecar::write` produces and what the
  import's copies produce (`<name>.handpic-tmp` in the date folder, renamed to
  `<name>` when complete). So a rating
  does not cost a rescan at all while culling.
- Even unfiltered it would terminate: `reconcile_sidecars` reparses a sidecar
  only when its on-disk stat differs from the one `mark_written` stored, so a
  rescan after the app's own write queues nothing.

The price is that an *external* sidecar edit (PhotoLab writing a `.dop`) is not
picked up live; it lands on the next focus / `Reload Folder` rescan, as it did
on reopen before.

On Windows, `ReadDirectoryChangesW` keeps a handle on the watched directory,
which pins the watched folder's ancestors against rename while Handpic has it
open; renaming the open folder itself succeeds (measured in the next entry).
Whether the watch blocks deleting the folder was not measured (an open item in
`todo.md`). `set` drops the previous watcher before creating the new one, so
leaving a folder releases it. A watch that cannot be set (SMB, say) is
`log::warn!`ed and ignored: the focus rescan is the fallback and the open must
not fail.

### A `notify` watch on Windows pins the watched folder's ancestors, not the folder itself (Hit)

Measured with `notify` 8.2 on Windows 11 (`RecommendedWatcher` =
`ReadDirectoryChangesWatcher`, `RecursiveMode::NonRecursive`): renaming a
watched folder itself **succeeds**; renaming a folder that has a watched
**descendant** fails with `PermissionDenied` (os error 5). So a watch's
`ReadDirectoryChangesW` handle blocks renaming any ancestor of the watched
path, not the watched path itself.

`crates/app/src/treewatch.rs` uses this to watch the whole folder tree with
one `RecommendedWatcher`, `watch`/`unwatch` per expanded folder, mapped
through a shared canonical-path -> tree-key list (several tree paths can
canonicalize to the same folder, e.g. a symlink; the last key's `unwatch`
is what actually drops the OS watch). Renaming a folder that has a watched
descendant needs `release_under` (unwatch everything at or below the
renamed path) before the rename and `restore` after — see
`treewatch::with_released`, which holds the `TreeWatch` lock for the whole
release/rename/restore-on-failure sequence so a concurrent
`set_tree_watches` cannot re-watch the path mid-rename. The notify event
handler only ever takes the separate canonical-path map's lock, never the
`TreeWatch` state lock, so this cannot deadlock.

Releasing a watch is **asynchronous** on Windows: in notify 8.2's
`ReadDirectoryChangesWatcher`, `watch` waits for the server thread's ack,
but `unwatch` only queues `Action::Unwatch` and `Drop` only queues
`Action::Stop`; the `CancelIo` + `CloseHandle` of the directory handle run
later on the `notify-rs windows loop` thread. So the handle is not released
the moment `unwatch` returns or the watcher is dropped (an earlier
measurement of 20/20 immediate renames was luck), and a rename issued at once
fails with `PermissionDenied` now and then (about 5 in 300 test runs under
full CPU load). `watch::settle` is the barrier: it calls
`watcher.configure(notify::Config::default())`, which goes through the same
action queue the server drains in order and blocks on its reply, so it
returns only after every earlier `unwatch` closed its handle (about 0.02 ms
when idle). `treewatch::State::remove` runs it after each real `unwatch`, and
`watch::release` unwatches the open folder and runs it before dropping the
watcher, so `rename_folder` needs no retry. `watch::set`'s plain drop of the
previous watcher (its comment says that releases the handle) is still only
eventually true; it is not on a rename path. A notify upgrade must re-check
that `configure` still queues behind `Unwatch` (loop the `treewatch` tests
under load).

Only rename was measured this way, not delete: don't assume the same
ancestor-pinning applies to deletion without measuring it.

Trashing (not renaming) a file inside a watched folder needs no such care: a
throwaway test put a `notify::recommended_watcher` `NonRecursive` watch on a
canonical temp dir (as `watch.rs` does for the open folder) and ran
`trash::run` with `TrashContext::default()` over a RAW and its `.xmp` inside
it — both went to the Recycle Bin with no failure, so trashing the rejects of
the open (watched) folder needs no watcher change, and a folder that is not
open has no watch at all.

- Source: `docs/plans/_archived/20260928-tree-live-watch/learnings.md`, Step 1;
  the asynchronous release is from
  `docs/plans/_archived/20260929-treewatch-unwatch-barrier/learnings.md`,
  Step 1; the trash check is from
  `docs/plans/_archived/20260928-trash-rejected-from-tree/learnings.md`, Step 1.

### Style the strip placeholder on `.cell img:not([src])`, never on `.cell img` (Hit)

`createCell` in `crates/app/ui/src/strip.ts` appends an `<img>` with no `src`
and sets `src` only once the thumbnail payload arrives; a cell is reused
across a refresh only for the same path, so `src` is never stale. That makes
`.cell img:not([src])` exactly "placeholder or failed load". Put the gray
placeholder background there, not on `.cell img` itself: the image box is the
144px square footprint and `object-fit: contain` letterboxes anything that is
not square, so a background on `.cell img` shows as gray bands around every
loaded thumbnail (24px above and below a 3:2 one).

- Source: `docs/plans/_archived/20260920-aspect-independent-strip-cells/learnings.md`,
  Step 1.

### An inline cell editor swaps the element in place, not by re-creating the cell (Hit)

The strip's rename editor swaps a cell's `.name` span for an `<input>` in
place (and back on end) rather than having `createCell` conditionally draw
an input. This keeps a stable reference (`Cell.name`) valid for any repaint
that runs mid-edit (e.g. a rating badge repaint), and lets a click that
lands on the very cell being edited (even its own thumbnail) still resolve
normally through `blur`, since only the input itself is detached. Whatever
drives the render loop must not release the row under active edit and must
cancel any live edit before a full `setFiles` replacement.

`setFiles`'s own cell-carrying path (used for `keepScroll` on a rescan of the
folder already shown) is the one exception: it never carries the cell under a
resumed inline rename. `setFiles` removes that cell's input while the `name`
span is still detached, so a carried cell would have no `.name` in its element
and the resume block's `cell.name.replaceWith(input)` would be a no-op. That
cell is released and recreated like any other cell outside the carried set,
and is then repainted with the just-cleared per-index maps (rating, burst,
tier) until `refilter` re-applies them right after — so it
briefly matches a fresh cell, whether or not `keepScroll` is set.

- Source: `docs/plans/_archived/20260928-rename-from-tree-and-strip/learnings.md`, Step 4;
  `docs/plans/_archived/20260928-strip-keep-scroll-on-rescan/learnings.md`, Step 1.

### Scope an id's `display` override to `:not([hidden])` when the element can also be hidden (Hit)

An `#label-names { display: grid; }` rule has id specificity, so it overrides
the global `[hidden] { display: none; }` rule (which only has attribute
specificity) — the block stays visible even when `hidden` is set (e.g. when
the sidecar format is `.dop` instead of XMP). Scope the rule as
`#label-names:not([hidden]) { display: grid; }` instead, so `[hidden]` still
wins when present.

- Source: `docs/plans/_archived/20260924-label-names-grid/learnings.md`, Step 1.

### `margin-left: auto` on a flex item only pushes items after it, not the item itself (Hit)

Moving `#filter` to the right edge of `#tools` by adding `margin-left: auto`
to `#filter` alone did not work: `#filter` is `#tools`'s first child, so the
auto margin pushed the whole `[filter][sort]` group rightward instead of
moving `#filter` past `#sort`, leaving `#sort` as the right-most item. Fix by
also giving the item to move `order: 1` (or otherwise reordering it after the
sibling it should end up right of) so the auto margin lands between it and
its new left neighbor.

- Why: an auto margin on a flex item consumes space on that item's own edge;
  it says nothing about the item's position relative to siblings, which is
  set by source order (or `order`).
- Source: `docs/plans/_archived/20260920-filter-button-right/learnings.md`, Step 1.

### Name a modified key from `event.code`, not `event.key` (Hit)

On macOS, Option and Shift change `event.key` (⌃⌥1 reports `¡`, not `1`), so
a key pressed with any modifier is named from `event.code` (`Digit1` -> `1`,
`Comma` -> `,`) after the held modifiers in `ctrl+alt+shift+meta` order; a
plain key keeps the lower-cased `event.key`. The shortcuts panel's capture must
use the same naming, and a lone modifier must be skipped before the derived
name is built, or holding Ctrl+Alt records `ctrl+alt+altleft`. Checking
`event.key` alone is not enough: it can arrive in an unexpected case
(`control`, not `Control`) or be unrecognizable while `event.code` still names
the modifier (`ControlLeft`/`Right`, `AltLeft`/`Right`, `ShiftLeft`/`Right`,
`MetaLeft`/`Right`, and the legacy `OSLeft`/`OSRight`). Reject a lone modifier
by checking both: `event.code` against that list, and a lower-cased `event.key`
against `control`/`alt`/`shift`/`meta`.

- Why: the keymap compares names; a name built one way on dispatch and another
  way on capture never matches.
- The `control` chip that still appeared after the lone-modifier guard landed
  was a stale `crates/app/ui/src/keys.js` shadowing `keys.ts` in the dev
  server, not a gap in the guard; do not widen the guard for it.
- Source: `docs/plans/20260919-color-labels/learnings.md`, Step 5,
  `docs/plans/_archived/20260920-ignore-lone-modifier-keys/learnings.md`, and
  `docs/plans/20260920-ignore-stale-ui-js/learnings.md`.

### An inline text editor (rename, etc.) must treat `event.isComposing` as a native key (Hit)

Any `keydown` handler for a live inline edit (folder/file rename input)
checks `event.isComposing` first and lets the key through natively when
true, so the `Enter` that commits an IME conversion (Japanese input, etc.)
does not also confirm/cancel the edit.

- Source: `docs/plans/_archived/20260928-rename-from-tree-and-strip/learnings.md`, Steps 2 and 4.

### A modal dialog needs an explicit Tab trap; `inert` is unavailable (Hit)

The build targets `safari13` (see `vite.config.ts`), which predates the
`inert` attribute, so a first-run/blocking dialog cannot rely on `inert` on
the elements behind it to keep Tab from reaching them.

- Instead, the main window's `keydown` handler must return early (without
  `preventDefault`) while the dialog is open, so Enter/Space still reach the
  dialog's own buttons natively and no app shortcut fires underneath it.
- Tab itself needs an explicit trap: `preventDefault` it and cycle focus
  within the dialog's own button list (honoring Shift for reverse), so focus
  never reaches controls behind the overlay (e.g. filter toggle, sort
  toggle, menus).

Source: `docs/plans/_archived/20260924-first-run-sidecar-format/learnings.md`
(`FormatGate` / the format-choice dialog in `crates/app/ui/src/firstrun.ts`).

The settings modal follows the same pattern: while it is open, the keydown
handler hands every key to `settings.keydown` and returns before the keymap.
The decision (add the captured key, cancel the capture, close, move focus, or
leave the key to the focused control) is the DOM-free `SettingsModal.key` in
`crates/app/ui/src/modal.ts`, so it is unit-tested without a DOM.

A dialog that runs a real backend operation (not just local state) should
also declare mutual exclusion with the other blocking dialogs explicitly,
rather than relying on only one keydown route existing: the trash
confirmation dialog (`TrashFlow`) and the sequence dialog exclude each other
and the settings modal (`trashFlow.busy` / `sequenceFlow.busy` /
`settings.isOpen` checked at every one of their open points), since the
window's keydown handler can only route to one of them at a time. A dialog
whose run cannot be canceled should disable every button and make Escape a
no-op while it runs, rather than leaving a race between the in-flight command
and a close.

Source: `docs/plans/_archived/20260928-trash-rejected-from-tree/learnings.md`,
Step 4.

### The strip context menu is HTML, not a native `tauri::menu` popup (Inferred)

Right-clicking a strip cell opens `#context-menu`, an HTML menu built by
`crates/app/ui/src/context.ts`, listing Pick / Reject / Unflag with their
current shortcut keys (Pick is omitted under XMP). A native menu cannot show
those keys: `accelerator()` only renders modifier combinations, so `p`, `x`
and `u` would have to be baked into the item title, plus a new command and
menu-event routing. The default WebView menu is suppressed only inside
`#strip`, so Inspect stays available elsewhere in devtools builds. Any key
other than Escape closes the menu first and then runs as usual.

Positioning: `menuPosition` flips the menu to the other side of the pointer
when it would overflow, then clamps at 0, so a menu bigger than the space on
both sides sticks to the top/left edge. It runs after the menu is un-hidden,
because `offsetWidth`/`offsetHeight` need the rebuilt items to measure
correctly; `#context-menu` overrides the shared `position: absolute` with its
own `position: fixed`. The menu is closed explicitly at the top of
`openDirectory` and in `refilter`'s empty-`files` branch, so it never floats
over a folder that just changed — any other place that swaps out `#strip`'s
contents needs the same explicit close.

- Source: `docs/plans/_archived/20260922-strip-context-menu/plan.md` and its
  `learnings.md`, Steps 2-3.

The menu also lists the star ratings and the color labels, in groups separated
by `<hr>`, and marks the focused file's current flag, rating and label.

- `main.ts`'s `labels` map stores the capitalized label name (`"Red"`), while
  the menu's action names are lower-case (`red`); code comparing a label
  against an action name must convert case, not compare directly.
- The rating/label buttons use `role="menuitemradio"`, not `menuitem`, because
  `aria-checked` is not a valid attribute on a plain `menuitem`.
- A rating of `0` counts as "No stars", same as `null` — check both when
  deciding whether a rating is set.
- Source: `docs/plans/_archived/20260923-context-menu-rating-label/learnings.md`,
  Step 1.

### The main-view right-click hit test rejects the vertical gap too (Hit)

`crates/app/ui/src/compare.ts` exports `comparePaneAt`, extracted from the
existing `click` handler on `#canvas` so the new `contextmenu` handler can
reuse it. The original `click` hit test only checked the horizontal gap
between panes; `comparePaneAt` also rejects the vertical gap, so a right-click
in a gap (horizontal or vertical) does nothing rather than opening the menu for
the nearest pane.

- While a comparison is still loading (`compareFrames` is empty), no pane is
  hit, so a right-click there only suppresses the native context menu.
- `openContextMenu` is a hoisted function declaration, so the new `contextmenu`
  handler on `#canvas` can be placed above it, next to the existing `click`
  handler.
- Source: `docs/plans/_archived/20260923-context-menu-rating-label/learnings.md`,
  Step 2.

### The strip's multi-selection keeps one invariant: the focused file is always selected (Inferred)

`selection.ts` (`crates/app/ui/src/selection.ts`) returns new `Selection`
values rather than mutating; callers replace the state held in `main.ts`.
Judgment commands are `(focused) => (own) => State`: the outer call decides
the value once from the focused file, the inner one applies it to each
selected file's own state, so fields the command does not touch survive per
file (e.g. `pick`/`unflag` only touch the rating field, so other files'
stars are kept). `judgments` always puts the focused file first, even on the
rare frame where it is outside the selection, so the focused file is always
judged.

Interaction rules that keep the invariant, and that diverge from what you
might otherwise assume:

- Shift+click moves the focus to the clicked file (and reloads the viewer),
  so the shift-range from the anchor always contains the focused file.
  Cmd/Ctrl+click only changes the selection and leaves the viewer as it is.
- `move`, `moveBurst` and `moveBurstFrame` collapse the selection to the new
  focused file only when the focus actually moves; a plain arrow at either
  end of the strip leaves an existing multi-selection as it is.
- A right-click on a strip cell outside the current selection collapses the
  selection to that cell first (as file managers do); a right-click inside
  the selection keeps it and only moves the focus.
- Pruning the selection after a filter/sort/judgment change happens only in
  `refilter`, which re-adds the focused file so the invariant holds; other
  paths (`resync`, `strip.setFiles`) all route through it
  rather than pruning themselves. The one exception is the resume landing:
  when `refilter`'s `force` marks a refresh that resolved a pending resume
  target, `settle` in `crates/app/ui/src/selection.ts` selects the landed file
  alone instead of pruning, since `openDirectory` had already selected
  `files[0]` (the same class of bug `restore` fixes for undo / redo).

- Source: `docs/plans/_archived/20260922-strip-multi-select/learnings.md`,
  Steps 1-4.

The folder tree has its own, separate selection model (`TreeSelection` in
`crates/app/ui/src/tree.ts`: `selectOnly`, `clickSelect`, `pruneSelection`)
rather than reusing `selection.ts`: the tree's selection has no
always-selected member (the open folder can be toggled off, and the selection
can go empty), unlike the strip's focused-file invariant above. Don't assume
both share one selection model when extending either. Pruning runs at the top
of the tree's `render()` against the drawn rows, so a collapse, a
`tree-changed` re-list and a reveal all drop what is no longer drawn in one
place, and the toggle modifier is `metaKey` on macOS / `ctrlKey` elsewhere
(the tree cannot accept either on every platform like the strip does, since
macOS Ctrl+click is the right-click the tree already guards).

- Source: `docs/plans/_archived/20260928-trash-rejected-from-tree/learnings.md`,
  Step 3.

### Case-insensitive path matching must be corrected level by level (Hit)

`relation`/`rebase` in `crates/app/ui/src/tree.ts` join the remaining path
segments in the caller's own spelling, not the tree's. When correcting a
path's case against the tree's listings, re-apply `respell` after each
listing rather than once at the end: only the level just matched against a
listing takes that listing's case, and the deeper segments are corrected
only when their own parent is later listed. Assuming one pass suffices will
leave deeper segments in the caller's original case.

- Source: `docs/plans/_archived/20260929-tree-reveal-ignore-case/learnings.md`,
  Step 1.

### `flex: none; width: min-content` to size a column by its fixed-width child (Hit)

Sizing `#side` with plain `flex: none` lets a long `#position` line
(`N / M · a / b in burst`) widen the column, shifting the layout as the text
changes. Adding `width: min-content` sizes the column to its narrowest
content instead: the fixed-width strip (plus its scrollbar gutter) sets the
width and the text wraps. Absolutely positioned children (the filter / sort
menus) do not feed into the intrinsic width.

- Update: the strip is now a horizontal row along the bottom (`#film`), so it
  no longer sizes `#side`; its height is fixed by `#strip-inner`.
- Source: `docs/plans/_archived/20260922-strip-scrollbar/learnings.md`, Step 1.

### Paint a full-bleed band under a cell's own background with `isolation: isolate` + negative `z-index` (Inferred)

The burst band and count badge are a `::before` with `z-index: -1` inside
`.cell.burst`, and `.cell` gets `isolation: isolate`. That makes the `::before`
paint under the cell's own background (so `.cell.current` / `.cell.failed`
backgrounds still cover it inside the box) while the few pixels it extends
outside the box still show, reading as a frame around the cell.

- Why: without `isolation: isolate` on the cell, a negative `z-index` child
  escapes to the nearest ancestor stacking context instead of staying local,
  so it can end up painting over unrelated siblings instead of just under its
  own parent.
- Source: `docs/plans/_archived/20260922-burst-frame-nav/learnings.md`, Step 2.

### Strip cell geometry: 168px cell, 144px image box, 2px corner inset (Hit)

`.cell` is 168px tall with a 144px square image box as its top; the star
rating and flag badges sit 2px in from their corners (e.g. `top: 2px`). When
placing a new badge (e.g. the burst count, at `bottom: 26px`), keep the same
2px inset from the image box's edge (not the cell's edge) rather than from
the cell's own bottom, and stay clear of the name strip below the image box
(about y 153-166 in a 168px cell). `bottom: 24px` would sit exactly on the
image box edge with no inset; `bottom: 26px` matches the 2px inset the other
corner badges use.

Absolutely positioned children of `.cell` are placed against the padding box,
not the border box, so with `box-sizing: border-box` the cell must be made
2px wider than the image box (144px image box → 146px cell) for a badge
positioned from the cell's edge to land flush with the image box's edge.

- Update: the strip is now horizontal; cells are placed by `left`, and
  `--cell-width` (the 154px pitch) on `#strip-inner` is the source of truth
  that `strip.ts` reads, replacing `--cell-height`.
- Source: `docs/plans/_archived/20260922-burst-count-bottom-right/learnings.md`,
  Step 1.
- Source: `docs/plans/_archived/20260923-strip-cell-geometry/learnings.md`, Step 1.

### Carry every judgment field on every write (Hit)

`set_rating` writes the whole judgment (stars, pick, label), so the frontend
keeps a `labels` map from `folder_entries` and passes the current label with a
star or flag keypress; otherwise a rating clears the label. Until the label is
known (`folder_entries` not yet arrived), the backend is told `labelKnown:
false` and keeps whatever the sidecar holds; that state is stored
(`ratings.label_known`, schema v6) so a crash before the write does not strip
the sidecar's label on the replay.

`clearall` needs `labelKnown: true` even for paths the frontend has not yet
loaded via `folder_entries` (`entries` in `crates/app/ui/src/main.ts`), so
`judge(next, forceLabel)` sets an optional `forceLabel` field on `Change`
that `send` turns into `labelKnown: true` regardless of `entries`.
`judge`'s idempotency check must also skip its "this path's local state
already matches" shortcut when `forceLabel` is set and `entries` lacks the
path — otherwise it would silently drop the clear for files whose label the
frontend doesn't know yet.

- Source: `docs/plans/20260919-color-labels/learnings.md`, Steps 4 and 6;
  `docs/plans/_archived/20260922-clear-all-flags/learnings.md`, Step 2.

### `Writer::set` / `set_now` carry `#[allow(clippy::too_many_arguments)]`, not a parameter struct (Hit)

Adding the `LabelNames` snapshot pushed `Writer::set` / `set_now` to 8
parameters, over clippy's `too_many_arguments` limit of 7 (`-D warnings` in
CI). The fix was `#[allow(clippy::too_many_arguments)]` on both, not a new
parameter struct — kept the change small and matches how this pair already
carries `format`. Expect the same call when adding another per-write field
here; every `sidecar.rs` test call site needs updating too.
The scan passes follow the same rule: `run_scan` and `run_faces_scan` went
over the limit (8/7) once they took the `ScanFocus` handle and got the same
`#[allow]`.

- Source: `docs/plans/_archived/20260922-lightroom-label-names/learnings.md`, Step 2.
- Source: `docs/plans/_archived/20260930-scan-priority/learnings.md`, Step 5.

### Clearing a rating writes `Rating = 0`, not "no rating", in both sidecar formats (Hit)

There is no way to represent "unrated" as an absent field once a sidecar
exists: `xmp::write_rating(None)` and `dop::write_rating(None)` both write
`Rating = 0` (see the doc comments in `crates/core/src/xmp.rs` and
`crates/core/src/dop.rs`), and the readers return `Some(0)`, which the app
already treats as unrated. Don't expect `read_rating() == None` after a clear
on a file that already has a sidecar — assert `Some(0)` instead.

- Source: `docs/plans/_archived/20260922-clear-all-flags/learnings.md`, Step 1.

### A `.ts` file with no `import`/`export` is a global script (Hit)

Its top-level `const`/`let` share scope with `lib.dom` globals; `const status`
collided with `window.status`. Every file under `src/` imports or exports
something today; keep it that way rather than adding `export {};` (Oxlint flags
that as `no-useless-empty-export`).

- Why: TypeScript only treats a file as a module when it has a top-level
  `import` or `export`.

### A `let` used only inside a function still needs to be declared above that function's first call, not just above its definition (Hit)

`main.ts`'s top-level code calls `renderMeta` on its last line as part of
module evaluation. A `let keyBindings` declared near the bottom of the file
(next to `applyKeymap`, its other user) sat in the temporal dead zone at that
call and threw. Declaring it next to `renderEmpty`, above `renderMeta`, fixed
it.

- Why: this file's top level runs functions immediately as part of module
  evaluation, unlike a file that only registers callbacks; a `let`/`const`
  used by such a function must be declared before that top-level call, which
  can be earlier in the file than where the variable "belongs" next to its
  other use.
- Source: `docs/plans/_archived/20260920-viewer-empty-state/learnings.md`, Step 2.

### Workers: declare the scope locally (Hit)

`worker.ts` declares the members it uses as a local `WorkerScope` interface and
casts `self` to it. Keep one `tsconfig.json` for both threads this way.

- Why: `DedicatedWorkerGlobalScope` is only in the `webworker` lib, and adding
  `webworker` next to `dom` clashes on the globals both declare.

### The preview payload's JPEG is orientation-neutral by construction (Hit)

The orientation in the `preview` payload header is the only rotation the
frontend applies (`draw()` in `main.ts`). Every JPEG `riffle_core::reader`
hands out (`read_preview`, `read_full`) leaves through one exit that rewrites
its Exif IFD0 Orientation to 1 in place (`jpeg::neutralize_orientation`), so
no container can add a second rotation, and a parser added later is covered
without touching the app or the worker.

- What broke: a portrait GFX 100 RAF (Orientation 8) showed upright in the
  filmstrip but sideways in the main preview. The RAF's embedded JPEG carries
  a full Exif APP1 with its own Orientation (so does a plain JPEG, which is
  its own preview), `createImageBitmap` rotated it once at decode, and
  `draw()` rotated it again by the header. The original design assumed an
  embedded preview is a bare JPEG stream without Exif, which holds for ARW
  only. The filmstrip was fine because the index thumbnail is re-encoded
  without Exif.
- Why not `createImageBitmap`'s `imageOrientation`: its behavior differs
  across WebView2, WKWebView and WebKitGTK, and `"none"` is deprecated in
  Chromium (`"from-image"` is the default), so the bytes are fixed instead.
- `reader::tests::every_format_hands_out_jpegs_without_an_exif_rotation`
  pins it for every supported format; a new format gets an entry there.
- Source: `docs/plans/_archived/20260930-orientation-neutral-preview/learnings.md`.

### The preview ring (`prefetch.ts`) has ordering and invalidation rules that a page turn depends on (Hit)

- `show()` must `take` the file's ring entry before it calls `plan`. `plan`
  drops every non-neighbour, and the file about to be drawn is no longer a
  neighbour of itself, so planning first closes the bitmap `show()` is about to
  draw.
- `requestPreview` returns early when `shown.seq === seq`. A ring hit draws
  while an older real request may still be in flight, and that request's
  stale-reply path would otherwise re-request and draw the same page twice.
- A prefetch is deliberately not in the `inFlight` flag, because a page turn
  issues its own request alongside it. A key that lands on a neighbour whose
  prefetch has not returned costs a duplicate read, and the late reply is
  closed.
- The worker tells a prefetch reply from a page reply by an echoed `prefetch`
  flag, not by `seq`, so the two id spaces cannot collide.
- Only a re-listing (`resync`) clears the ring, because it is the only signal
  that a file's bytes may have changed under the same path. A filter or sort
  change only `retain`s. Any new path that rewrites a file in place must go
  through `resync`, or the ring will show a stale image.
- `refused` (failed reads) lasts until forgotten. `crowded` (files evicted for
  the byte cap) lasts only while the file stays wanted. That is why two
  neighbours that cannot share the cap, such as 24 MP view-only JPEGs against
  the 32 MP cap, do not discard a read on every page turn. A RAF is 13 MP, so
  RAW folders never hit this.
- Timing: a keypress that finds a request in flight keeps its mark in
  `queuedKeypressAt` for the re-request that serves it. Dropping the mark
  would leave a held key with no `wait` in its `page` line.
- Source: `docs/plans/_archived/20261108-preview-latency-breakdown/learnings.md`
  (Step 1 and Step 4).

### WebKitGTK draws a large transferred `ImageBitmap` transparent (Hit)

On Linux (WebKitGTK 2.50.4, WSLg), an `ImageBitmap` decoded in the worker and
transferred to the main thread draws fully transparent once it has too many
pixels, whatever its shape: the threshold is about 6.87 MP (6,840,000 px drew,
6,900,000 px did not), and a 60 MP SIGMA fp L preview left the main view
blank with no error. The worker passes `resizeWidth` / `resizeHeight` to
`createImageBitmap` for previews over `PREVIEW_PIXEL_LIMIT` (`commands.rs`,
Linux only, 6 MP), reading the size from the JPEG's SOF (`decode.ts`).

- Why: decoding on the main thread, or resizing inside the worker, both draw;
  only the large transferred bitmap fails. Windows (WebView2) is unaffected.
- The first limit shipped (12 MP) was a secondhand number that did not hold;
  see [Verify a platform workaround's threshold on the production code path](#verify-a-platform-workarounds-threshold-on-the-production-code-path-hit)
  before changing this one.

### A hidden focused element does not always lose focus itself (Inferred)

Chromium (WebView2) runs a focus fixup when a focused element becomes hidden
(e.g. `display: none`), but WebKit has not always done so. Don't rely on the
browser to blur a focused control when you hide its container; call
`element.blur()` explicitly from the code path that hides it.

- Source: `docs/plans/_archived/20260926-folder-tree-keyboard/learnings.md`,
  Step 1 (`changePanels` calling `folders.blur()` when the left pane, which
  holds the keyboard-focused folder tree, ends up hidden). Not reproduced on
  a real WebKit build.

### Right-click on the folder tree must not steal keyboard focus (Hit)

`#folders` is `tabindex="0"`, so a right-button `mousedown` would focus it and
silently turn the culling keys off. `folders.ts` `preventDefault()`s a
`mousedown` with `button === 2` to keep focus wherever it was. On macOS,
Control+click is the other standard way to right-click (common on trackpads);
WebKit reports it as a primary-button `mousedown` with `ctrlKey === true` and
no `click`, so the guard also `preventDefault()`s a `button === 0` `mousedown`
with `ctrlKey` when `navigator.platform` reports macOS — on Windows/Linux
Ctrl+click is an ordinary click and must still focus the tree. The
document-level `mousedown` that closes an open menu still sees the event,
since only the default action is prevented, not propagation.

Keydown ordering: `folders.keydown` consumes `Escape` to blur the tree, so an
"any key closes the context menu, `Escape` is consumed doing so" check must
run _before_ the tree's keydown block, not after — placed after, the tree
swallows `Escape` first and the menu never closes. This also means any menu
open when the tree has focus closes on any key the tree consumes, not just
`Escape`.

- Source: `docs/plans/_archived/20260926-folder-reveal/learnings.md`, Step 1.
- Not verified on a real device: drive roots. `tauri-plugin-opener` 2.5.5's
  Windows `reveal_items_in_dir` resolves the parent via
  `windows_shell_path::shell_parent_path` and returns `Error::NoParent` when
  there is none; no `open_path` fallback was added, so such an error surfaces
  only via the status line — check on a real drive-root folder before relying
  on this.

### `renderMeta()` redraws the status line but does not clear `note` (Hit)

`setStatus()` and `renderMeta()` are not interchangeable: `renderMeta()` only
re-renders from the current state, while `note` (an older gray status line)
is cleared only by calling `setStatus()` with no argument. Calling
`renderMeta()` after a new error leaves a stale `note` on screen next to it
(e.g. Sequence succeeds on folder A leaving its gray note, then fails on
folder B — A's note stays visible above B's orange error). When a code path
needs to guarantee the status line reflects only the latest event, call
`setStatus()` with no argument, not `renderMeta()`.

- Source: `docs/plans/_archived/20260928-sequence-folder-error/learnings.md`,
  Review feedback round 1.

### Write relative imports with `.js` (Measured)

`import { x } from "./foo.js"`; Vite resolves the `.js` suffix to the `.ts`
source in dev and build, and it matches what the type check expects.

### A real `.js` beside a `.ts` shadows the source (Hit)

Because those imports carry the `.js` suffix, an actual `foo.js` sitting next
to `foo.ts` wins: the dev server serves the `.js` and the `.ts` is never
compiled. A `tsc`-era build left such files behind in `crates/app/ui/src/`, so
the dev build ran code months older than the sources. `.gitignore` now covers
`crates/app/ui/src/**/*.js`, so such files no longer show up in `git status`,
and `tsconfig.json` sets `"noEmit": true` so a stray `tsc` cannot regenerate
them.

- Why: when the running app disagrees with the source you are reading, check
  `ls crates/app/ui/src/*.js` (or `git clean -nX crates/app/ui/src`) before
  debugging the code, and restart the dev server after deleting them — Vite
  caches the old transform.
- Source: `docs/plans/20260920-ignore-stale-ui-js/learnings.md`, Step 1.

### `vp check` type-checks with TypeScript-Go and covers `vite.config.ts` (Hit)

- Its newer DOM lib types `HTMLElement.hidden` as `boolean | "until-found"`, so
  passing it where a `boolean` is expected fails (TS2345); `tsc` 5 accepted it.
  Coerce with `!!`.
- The root `vite.config.ts` is type-checked too, so `process` / `node:path`
  need `@types/node` as a devDependency.
- `vitest` has to be a direct devDependency even though `vite-plus` brings it
  at runtime: pnpm does not hoist it, and `import ... from "vitest"` in a test
  fails the type check with TS2307.
- Source: `docs/plans/20260919-vite-plus/learnings.md`, Steps 1 and 5.

### Vite+ config facts (Measured)

- `index.html` is the only entry, the default one under `root`, so there is
  no `build.rollupOptions.input`.
- `test.include` is relative to the Vite `root` (`crates/app/ui`), so it reads
  `src/**/*.test.ts`.
- `vp fmt` honors `.gitignore`. Its scope (and `lint.ignorePatterns`) is the
  frontend plus the root JS tooling files; Markdown, the release-please JSON
  and `.claude/**` are excluded.
- Adding `vp fmt` to `mise run fmt` before the reformat commit would have made
  every pre-commit `mise run fmt` reformat the frontend; land the reformat
  together with (or before) the task change.
- `vp fmt` formats every JSON file in the repository, so a generated data file
  such as `crates/core/genres/**` (768 floats per vector) would end up with
  one float per line. List such paths in `vite.config.ts`'s
  `fmt.ignorePatterns` when a script writes them.
  Source: `docs/plans/_archived/20261012-image-embeddings/learnings.md`, Step 5.
- Source: `docs/plans/20260919-vite-plus/learnings.md`, Steps 1-4.

### Rolldown drops `/*!` legal comments under minify; an explicit `comments` object replaces Vite's defaults rather than merging (Hit)

Vite 8 (vite-plus-core) sets rolldown's `output.comments` to
`{ annotation: !minify, jsdoc: !minify, legal: !minify }` by default, so a
minified build silently drops `/*! ... */` license banners (e.g. a vendored
icon's license notice).

- Fix: set `build.rolldownOptions.output.comments` in the root
  `vite.config.ts` explicitly, and restate `annotation` / `jsdoc` too —
  `{ legal: true }` alone keeps them at their non-minified value, leaving
  ~700 bytes of `@__PURE__` / `@vite-ignore` comments in the minified bundle.
  Mirror Vite's own derivation: `annotation: !!process.env.TAURI_ENV_DEBUG`,
  `jsdoc: !!process.env.TAURI_ENV_DEBUG`, `legal: true`.
- Verify with `pnpm exec vp build`: check the built JS under
  `crates/app/ui/dist/assets/` for the expected `/*! ... */` block and the
  absence of stray annotation comments.
- Source: `docs/plans/_archived/20260926-af-eye-sharpness-candidate-icon/learnings.md`, Step 1.

### A fresh worktree can report `vp` as not found even though it's installed (Hit)

`mise run fmt` failed with `Command "vp" not found` in a freshly created
worktree, although `node_modules/.bin/vp` already existed on disk.

- Fix: run `pnpm install --frozen-lockfile` (it may report "Already up to
  date" and still fix it); `mise run ci` then passes. Plain `pnpm` may not be
  on the Git Bash `PATH` in a fresh worktree, so run it as
  `mise exec -- pnpm install --frozen-lockfile`. The same goes for ad-hoc
  `vp` runs: use `mise exec -- pnpm exec vp {check,fmt,test,build}` from the
  repository root, because `crates/app/ui` is not a pnpm workspace package.
- Order matters: if `mise run fmt` ran before `node_modules` existed it exits 1
  without formatting, and `mise run ci`'s lint then flags the unformatted files
  (a new test file, for example). After `ci`'s `pnpm install`, re-run
  `mise run fmt`, then `ci` again. Better, install first and run `fmt` after.
- Source: `docs/plans/_archived/20260924-focus-mark-af-frame/learnings.md`,
  Step 1; `docs/plans/_archived/20261015-sequence-summary-lines/learnings.md`,
  Step 1; `docs/plans/_archived/20261109-prefetch-useless-spread/learnings.md`,
  Step 1.

### On Windows, run `vp` through `node`, not `pnpm exec`, in mise tasks (Hit)

The `test` task calls `node ./node_modules/vite-plus/bin/vp test`; `fmt`
does the same (`node ./node_modules/vite-plus/bin/vp fmt`). `lint` still
calls `pnpm exec vp check` (unconverted; it works today but carries the same
risk this note describes).

- Why: the mise task shell is bash; `pnpm exec vp` there resolves to the `.cmd`
  shim, which runs under cmd.exe with bash's POSIX-style `PATH` and cannot find
  `node`.
- Source: `docs/plans/20260919-vite-plus/learnings.md`, Step 5.

### On Windows, editing files with a Python heredoc can corrupt line endings and escapes (Hit)

A Python heredoc edit in text mode writes CRLF into this LF repository
(`core.autocrlf=false`), and a `'''...'''` string silently eats `\U` / `\\`
sequences — a risk for Windows-path test fixtures.

- Fix: open/write with `newline=""` to keep LF (or strip the `\r` afterwards),
  check with `git ls-files --eol`, and re-check backslashes in any
  Windows-path string literals afterward.
- Source: `docs/plans/_archived/20260929-tree-root-dedup/learnings.md`, Step 1.
- Source: `docs/plans/_archived/20261114-gpui-perf-overlay/learnings.md`,
  Step 1 (hit `crates/native/src/main.rs` and `scan.rs` too).

A related trap: a Python heredoc run through the Bash tool can also turn
escape text meant to stay literal (`\0`, `\x01` inside a byte-string
literal destined for Rust source) into real NUL / 0x01 bytes in the
written file, even when the heredoc is quoted (`'EOF'`). Build such bytes
with `bytes([0, 1, ...])` instead of escape literals, or make the edit
with the Edit tool rather than a Python heredoc. A Python script containing
nested `'''` strings can also fail outright in the Bash tool ("unexpected EOF
while looking for matching"); write the script to the scratchpad with the
Write tool and run that file.

A third failure: on this setup the Git Bash `python` / `python3` command can
be a stub, and a `python -` / `python3 -` heredoc hung on it twice without
running anything (the call ended up in the background). Kill the hung process
by its PID, not by image name. Use the venv's `python.exe` by full path on a
script file written with the Write tool, or make a text edit with the Edit
tool and no Python at all. To wait on a log a background job has not created
yet, use `tail -F` (not `tail -f`, which exits at once on a missing file)
bounded by `timeout N`, for example
`timeout 570 tail -F run.log | grep -m1 "done:"`.

- Source: `docs/plans/_archived/20261017-bad-photo-labels/learnings.md`, Steps 4 and 5.
- Source: `docs/plans/_archived/20261023-discard-model/learnings.md`, Steps 4 and 5.
- Source: [heif-cr3-message learnings, Step 1](../plans/_archived/20260930-heif-cr3-message/learnings.md#step-1); face-detection-recall-cost learnings, Steps 2 and 3.

### On Windows, the Bash tool halves doubled backslashes, even in a quoted heredoc (Hit)

Every `\\` in a Bash-tool command lands as `\` (single backslashes survive):
a `cat > f <<'EOF'` heredoc, quoted or not, a single-quoted `printf '%s'` or
`echo` argument, and a heredoc fed to `python -` alike. Text holding Windows
paths, verbatim `\\?\` / UNC prefixes or regex escapes comes out wrong with
no error; the Write tool writes the same text byte for byte.

- Fix: write text holding backslashes with the Write / Edit tools, or put it
  in a script file written by them and run that file; never pass it through
  a Bash command string.
- Source: `docs/plans/_archived/20260928-trash-rejected-from-tree/learnings.md`,
  Step 4; `docs/plans/20260929-todo-five-small-items/learnings.md`, Steps 4
  and 5.

## CI

### Do not hard-code the pnpm store path (Inferred)

Cache the output of `pnpm store path --silent` (see `.github/workflows/`).

- Why: the store location differs per OS.

### `tauri-action@v0` has no `uploadWorkflowArtifacts` input (Hit)

Setting it silently produces zero build artifacts (the run logs "Unexpected
input(s) 'uploadWorkflowArtifacts'" and continues). Upload bundles explicitly
with a separate `actions/upload-artifact@v4` step over
`target/release/bundle/**` instead of relying on the action to do it.

- Source: `docs/plans/_archived/20260918-github-releases-auto-update/learnings.md`,
  Steps 2 and 4.

### `gh release edit` resolves drafts by tag only once the tag is pushed up front (Hit)

`tauri-action`'s own `getReleaseByTag` lookup skips draft releases, so it
still needs `releaseId` passed explicitly rather than relying on tag
resolution. `gh release edit <tag>` only works once `force-tag-creation`
makes release-please push the tag before the draft release exists.

- Source: `docs/plans/_archived/20260923-draft-release-publish/learnings.md`,
  Step 1.

### Cache Rust builds by `matrix.os`, not `runner.os` (Hit)

`macos-latest` (arm64) and `macos-15-intel` (x86_64) both report
`runner.os == macOS`; a cache key built from `runner.os` would let them
restore each other's `target`, which is wrong across architectures. Key on
`matrix.os` instead.

### release-please: the `rust` strategy fails on a workspace root with no `[package]` (Hit)

`release-please release-pr --dry-run --release-type rust` (v17.9.0) aborts
with `is not a package manifest (might be a cargo workspace)`, because
`CargoToml.updateContent` tries to update the root `Cargo.toml` as a package
even when it only declares `[workspace]`. Use `simple` with `extra-files`
pointing at each crate's `Cargo.toml` instead.

- For a `Cargo.lock` extra-file, `$.package[?(@.source === undefined)].version`
  is the working JSONPath discriminator for "this workspace's own crates" —
  equality on `@.name` matches nothing, and `startsWith`/`match` are rejected
  by jsonpath-plus's safe evaluator.
- `release-pr --dry-run` reads `release-please-config.json` from the **remote**
  branch, not the local working tree, so testing a not-yet-merged config needs
  a scratch clone/bare-repo setup with the config pushed to its `main`.
- Source: `docs/plans/_archived/20260918-github-releases-auto-update/learnings.md`,
  Step 4.

### A green branch-only CI run does not prove the merge result compiles (Hit)

See "Adding a macOS menu item needs the same `cfg` split as its siblings"
above: two branches can each pass CI alone and still break when merged,
because each branch's own copy of an import/cfg is still in scope on that
branch. Only building the actual merge/rebase result (not the branch tip)
catches it; a single-OS failure right after a merge that looks like a
runner flake is a signal to check the merge ref before assuming flakiness.

### A `Release-As` footer must be on the squash commit itself, not a branch commit (Hit)

`Release-As: 0.4.0` on the last commit of a multi-commit PR branch did not
reach release-please: this repo squash-merges with
`squash_merge_commit_message: COMMIT_MESSAGES`, and for a multi-commit PR
GitHub appends a `---------` separator and the collected `Co-authored-by`
trailers after the concatenated messages, pushing `Release-As` out of the
final footer block. The release PR stayed at the old version after merge.

- Fix: squash-merge with an explicit body carrying the footer, e.g. `gh pr
  merge --squash --body 'Release-As: 0.4.0'`.
- When a PR needs `Release-As` and has more than one commit, don't rely on
  a commit message footer; set it on the squash merge itself.
- Source: `docs/plans/_archived/20260925-lightroom-layout/learnings.md`,
  "Release-As footer".

## Verification

### Write a performance number with its measurement conditions (Hit, twice)

Two performance claims here had to be walked back in review because they read
as general when they were not: a 30-second target that rested on warm `cp`
reads, and a batch of README numbers measured on folders of symlinks to a
single file.

- Why: a number without its conditions — page cache state, and whether the
  files were distinct or 5000 symlinks to one inode — is read as a general
  claim and copied forward as one.
- State what was actually measured next to the number, and say plainly what
  still needs a real-folder measurement.
- For a copy or write benchmark the destination is a condition too. A nearly
  full, DRAM-less SATA SSD lost its write cache under sustained writes, so
  throughput fell run after run with identical settings (21.6 then 8.1 MB/s)
  while the source read alone ran at 135 MB/s. Methods measured back to back on
  such a drive cannot be told apart. Compare on a destination faster than the
  source, wait or alternate the order, and do not time the source while the
  destination is still busy (read timings taken during a background copy came
  out at 16-22 MB/s). Source:
  `docs/plans/_archived/20261106-import-from-card/learnings.md`, Step 2.

### Run a multi-minute step in the background and poll it in short loops (Hit)

A single Bash tool call that blocks for several minutes (a feature dump of
11 minutes, a 1-hour index rebuild) trips the 600 s stream watchdog and stalls
the agent. Start the step with `run_in_background` (or detached) and poll its
output file with short bounded loops.

- Source: `docs/plans/_archived/20261012-image-embeddings/learnings.md`, Steps 2 and 7.

### Verify a platform workaround's threshold on the production code path (Hit)

Before shipping a numeric threshold for a platform workaround, measure it on
the exact code path production runs, not on a quoted or secondhand number or
a simpler reproduction.

- What broke: #383 shipped a 12 MP Linux preview limit (see
  [WebKitGTK draws a large transferred `ImageBitmap` transparent](#webkitgtk-draws-a-large-transferred-imagebitmap-transparent-hit))
  from "12.3 MP drew, 16 MP did not", which was never measured on the shipped
  path. The user's manual check still showed a blank main view, and a
  follow-up had to lower it to 6 MP.
- How it was measured: a MiniBrowser bisect that reproduced the real path (a
  worker `createImageBitmap` with resize options, the bitmap transferred,
  `drawImage` to a canvas, the center pixel read), with each result POSTed to
  a local Python HTTP server so the run could be logged outside the webview.
  It put the threshold at ~6.87 MP by pixel count, independent of shape.
- Rule: build the harness around the same API calls and transfer steps the
  app uses, bisect the number, and write the harness down with the result.
- Source: [linux-preview-pixel-limit-6mp learnings, Step 1](../plans/_archived/20260924-linux-preview-pixel-limit-6mp/learnings.md#step-1).

### GUI automation does not work on this Mac (Hit)

`osascript` is denied assistive access: the app's window cannot be brought
forward, and injected keystrokes are dropped silently.

- Why: macOS privacy settings on this machine, not something the repo controls.
- Plan any GUI check as a **manual confirmation by the user**, and list exactly
  what they should look at. Report unchecked behavior as "not verified"; that
  is more useful than an implied pass.

### `cargo` is not on the Bash tool's PATH (Hit)

- Run it through `mise exec -- cargo ...` instead of bare `cargo ...`.

### `riffle-app` is bin-only: use `cargo test <name>`, not `--lib` (Hit)

`cargo test --lib` fails because `crates/app` has no library target.

- Run `cargo test <test_name>` (optionally scoped with `cd crates/app`) instead.
- A `#[cfg(unix)]` test compiles and runs only on macOS/Linux CI; on a
  Windows dev machine it's silently absent from the run (not a failure, not
  a skip you'll see), so a passing `cargo test <name>` there proves nothing
  about it. Confirm unix-only coverage on CI, not locally on Windows.

### A rating/pick test needs a scan-populated `files` row before `rating_of` reads back (Hit)

`rating_of` joins `files`, which only a folder scan fills in. A unit test
that calls `write_batch` (or the extracted `switch_format` body in
`crates/app/src/commands.rs`) and then reads the rating back must first
call `index::stat` to populate that row, the same way
`a_foreign_sidecar_is_read_on_the_first_open_without_any_files_row` does —
otherwise the read returns nothing regardless of whether the write
succeeded.

- Source: `docs/plans/_archived/20260920-robustness-cleanup/learnings.md`, Step 4.

### Data-migration tests never call `migrate::run` (Inferred)

`crates/app/src/migrate.rs` `run` acts on the real OS data and cache folders.
With more than one `LEGACY_IDENTIFIERS` entry, a test that calls `run` would
move a developer's real legacy folder (for example `com.minodisk.riffle`) into
the current identifier's.

- Rule: test migration through `migrate_under(bases, identifier, rename,
  leftovers)` on temp bases only. This includes the "current identifier is not
  a migration" case. `run` is only the thin caller of `migrate_under`.
- Rule: platform leftover removal (macOS) is the `leftovers` hook, run under the
  `all_moved` condition. Tests pass a stub rather than touching real folders.
- Clippy's `cloned_ref_to_slice_refs` flags `&[root.clone()]` in such tests; use
  `std::slice::from_ref(&root)`.
- Source: `docs/plans/_archived/20261113-identifier-dev-minodisk/learnings.md`, Step 1.
