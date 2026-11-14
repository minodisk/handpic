# Working on `crates/native`

Read this before touching the GPUI app in `crates/native`. It lists what the
port has met so far, each with the reason.

The tags follow [`tauri-app.md`](./tauri-app.md): **Hit** broke something here,
**Measured** steered a design decision, **Inferred** comes from sources or docs
only.

Source: [the gpui-port plan](../plans/20261109-gpui-port/plan.md), its spike
and Step 1's [learnings](../plans/20261109-gpui-port/learnings.md).

## Toolchain

### GPUI comes from one pinned Zed rev (Inferred)

`gpui` and `gpui_platform` are git dependencies on Zed's repository at
`b5cd310b3b54dfc659fcc06453e6358843dfe216`; crates.io's `gpui 0.2.2` is stale.
Zed pins Rust 1.98.1, and so does `crates/native/mise.toml`.

- Rule: bump the rev only in a deliberate chore PR, and record API changes met
  at the bump here.

### Only two of Zed's patches are needed (Inferred)

`[patch.crates-io]` carries Zed's `async-task` and `async-process` forks. Zed's
other patches (`cpal`, `notify`, `calloop`, `khronos-egl`, ...) are not needed
by `gpui` / `gpui_platform`.

- Rule: do not copy the others. Do not move these two to the root workspace
  while the Tauri app lives there: both crates are in its lock, so a workspace
  patch would swap them for Zed's forks in the shipped app.

### Linux needs system libraries, macOS and Windows do not (Inferred)

Per Zed's `script/linux`: `libfontconfig-dev`, `libwayland-dev`,
`libx11-xcb-dev`, `libxkbcommon-x11-dev`, `libvulkan1`, plus a Vulkan-capable
driver at run time. The `native` CI job's Linux leg installs exactly these.
macOS needs the Xcode CLT (Metal), Windows only MSVC.

- Rule: add a package to the CI job's apt step only when a build error names
  it, and keep this list equal to that step.

### macOS compiles its shaders with `xcrun metal` at build time (Inferred)

`gpui_apple`'s build script runs `xcrun metal` and `xcrun metallib` unless
`gpui_platform`'s `runtime_shaders` feature is on, so an Xcode without the
Metal toolchain fails the build there, not at run time. `macos-latest` is
arm64, where `build.rs` returns before the NASM check.

- Rule: if a macOS build fails at `xcrun metal`, install the toolchain
  (`xcodebuild -downloadComponent MetalToolchain`) rather than turning on
  `runtime_shaders` for every build.

### The `native` CI job runs on all three platforms (Inferred)

It is a matrix over `windows-latest`, `ubuntu-latest` and `macos-latest` with
`fail-fast` off, each leg running the same `mise exec rust@1.98.1 -- cargo
clippy` / `cargo test`; only the Windows leg adds `setup-nasm`'s
`$USERPROFILE/nasm` to `PATH` itself. Nothing under `crates/native/src` is
`cfg(windows)`-only, and `tree.rs`'s path helpers work on strings, so its
`C:\` and UNC test paths do not depend on the host's `std::path`.

- Rule: code reached only under one platform's `cfg` must not fail
  `-D warnings` on the other two; give it a `cfg_attr(..., expect(dead_code))`
  or the same `cfg`.

### Without NASM, mozjpeg silently loses SIMD (Measured)

`mozjpeg-sys`'s build script runs `nasm -v`, prints a `cargo:warning` and
continues without SIMD when that fails, and cargo hides build-script warnings
of non-path dependencies. RAF decode measured about 99 ms without SIMD and
46 ms with it. `crates/native/build.rs` therefore fails the build, and its own
`cargo:warning` is the only place the SIMD state is visible.

### winget's NASM is per-user and not on `PATH` (Hit)

`winget install NASM.NASM` installs to `%LOCALAPPDATA%\bin\NASM`, not
`C:\Program Files\NASM`, and adds nothing to `PATH`.

- Rule: put that directory on `PATH` for every build.

### `mozjpeg-sys` ignores the `NASM` variable (Inferred)

`nasm-rs` honors `NASM` as the binary path, but the `nasm_supported()` probe of
`mozjpeg-sys` 2.2.3 runs plain `nasm` from `PATH`. `build.rs` therefore runs
`nasm -v` from `PATH` only and fails when it does not run, whatever `NASM` holds.

- Rule: put `nasm` on `PATH`; setting `NASM` does not satisfy the build.

## Backend

### The backend names no event and no `AppHandle` (Inferred)

`riffle-backend` serves both apps, so it reports through callbacks:
`watch::Watch::spawn` takes `on_changed`, and `watch::set` / `release_under`
take the `&Watch`. Each app's own thin module owns the event name and payload;
in the Tauri app that is `crates/app/src/watch.rs` with `folder-changed`, and
`crates/app/src/treewatch.rs` with `tree-changed` over
`treewatch::TreeWatch::spawn(on_changed)`, `TreeWatch::set(dirs)` and
`treewatch::with_released(&TreeWatch, ..)`. The trash commands' bodies
(`trash::trash_listed`, `delete_listed`, `undo_run`, `redo_run`) take the
`&Runs` and a `progress(done, total)` callback; the Tauri commands keep the
writer flush, the `Scans` lock and its `SCAN_RUNNING` refusal, the
`trash-progress` emit and the log line around them.
The folder operations' bodies take their state the same way:
`rename::rename_folder` and `datefolder::split_folder` take a
`rename::Context` (the `&Watch`, the `&TreeWatch`, the optional index and the
`&Runs`), `datefolder::plan_of` the optional index, and
`foldersidecars::delete_run` the index and the `&Runs`. None of them checks
for a scan or drains the sidecar writer: the Tauri wrappers flush the writer,
take the `Scans` lock, refuse with `text::SCAN_RUNNING` and hold the lock
across the call, and the native app has to guard the call with its own gate.
The MCP companion's server (`mcp::Mcp`) takes a `Bridge` built over a send
callback and returns each switch's `McpState`; the Tauri app's `mcp.rs` owns
`mcp-request`, the main-window check and the `mcp-state` emit. The server
spawns its tasks with a bare `tokio::spawn`, so `Mcp::switch` and `Mcp::stop`
must run inside a tokio runtime: the Tauri app calls them from async commands,
a task on `tauri::async_runtime` and `tauri::async_runtime::block_on`, which
all enter Tauri's runtime; the native app's `mcp.rs` owns a current-thread
runtime on its one `mcp` thread, which runs the switch, waits for the quit's
signal and runs the stop. `Mcp::with_port` binds another port than
`MCP_PORT`, for the native `--mcp-port`.

- Rule: never add `tauri`, an event name or an `AppHandle` to `crates/backend`;
  take a callback or the state itself.
- Rule: a backend body that changes a folder on disk leaves the scan check to
  its caller and says so in its doc comment; call it only with the caller's
  refusal of a running scan held across the call.

### The backend is compiled twice (Inferred)

`crates/backend` is a member of the root workspace, and `crates/native`, a
separate workspace with its own `Cargo.lock` and Rust 1.98.1, uses it by path,
as it does `riffle-core`. A dependency added to the backend also lands in
`crates/native/Cargo.lock`, and clippy lints the backend only from the root.

- Rule: check a backend change with both `mise run ci` and
  `mise run native:check`. `mise run ci` runs only `lint` and `test` of the
  root workspace, so it never covers `crates/native`. This holds for a change
  to the app's identifier or to `native`'s own files (`datadir`, `logger`,
  `settings`) as well as for a backend change.
- Rule: `#[cfg(test)]` does not cross a crate boundary. When code moves into the
  backend, a helper the app's tests still use (for example `Index::row_flags`)
  must lose its `#[cfg(test)]` or be copied into the app's test module.
- Rule: `mise run native:check` runs clippy and tests but does not build
  `riffle-native.exe`. Run `cargo build` before launching the exe, or you run a
  stale binary. A still-running older `riffle-native.exe` locks
  `target/debug/riffle-native.exe` and `cargo build` fails with `os error 5`;
  rename the running exe aside (for example `riffle-native.stale-<pid>.exe`)
  instead of killing it, or ask the user to close it.
- Rule: `crates/native/Cargo.lock` resolves its own versions (`notify` 8.2.2
  against 8.2.0 in the root lock). `watch::set` (watch, replace, the debounce)
  works on 8.2.2: copying into and deleting from the open folder resynced it.
  The tree watcher's `unwatch` + `settle` returns promptly on 8.2.2 too (see
  "The tree follows the disk through one `TreeWatch`"); `release_under` is
  not called yet, so re-check the rename path when part 2's folder rename
  needs it.

### The judgments' Tauri-free pieces live in the backend (Inferred)

The folder-open sidecar pass (`sidecar::reconcile_of`, with `SidecarError`
and `MAX_SIDECAR_BYTES`), the meta pane's formatting (`exif::Metadata` and
`exif::metadata(name, &shot)`), the stored `labelNames` value
(`sidecar::label_names_setting` / `label_names_value`), the metadata and
preview reads (`exif::read_metadata`, `ondemand::read_preview`) and the stored
`mcpEnabled` value (`mcp::mcp_enabled_setting`) moved out of
`crates/app/src/commands.rs` so the native app can call them; the Tauri
commands call them from there. `raw_only` stayed in `crates/app`: the native
app's view-only rule is the frontend's `viewonly.ts`, ported on its own.
The folder tree's listing (`folders::roots(home)`, `list(dir)`,
`REVEAL_LABEL`, `is_hidden`) moved the same way out of
`crates/app/src/folders.rs`; the home directory is the caller's (Tauri's
path resolver there), and the reveal itself is not shared: the Tauri app
keeps the opener plugin, the native app has GPUI's `App::reveal_path`.

- Rule: a Tauri-free helper the native app needs moves into the backend, it is
  not copied, so the two apps cannot drift in what they read and show.

### The settings live in the native app's own store, seeded once (Inferred)

`settings.rs` reads the native app's settings store
(`riffle_backend::store`): one SQLite file, one `settings` table of
`key TEXT PRIMARY KEY` / `value TEXT NOT NULL` (JSON), WAL, the schema
version in `user_version`. It is `--settings <path>`, else
`settings.sqlite` in the `IDENTIFIER` folder under the config dir
(`datadir::settings_path`), next to the Tauri app's `settings.json`. The
values (`sidecarFormat`, `labelNames`, `shortcuts`, `autoAdvance`,
`sortOrder`, `filter`) go through the one `from_values` with the backend's
rules (`SidecarFormat::from_setting`, `sidecar::label_names_setting`,
`Keymap::from_overrides`, `autoAdvance` on unless a boolean says off,
`SortKey::from_setting`, the Tauri `parse_sort_order` with `recommended` read
as capture order and left stored as it is, `Stored::from_setting`).

Only a new store (no schema yet: a missing file or a zero-byte one left by a
crash) is seeded, once, with every key of a Tauri `settings.json` verbatim,
the schema and the values committed in one transaction. The seed is
`--import-settings <path>` when given; else, only without `--settings`, the
first that exists of `<config dir>/<IDENTIFIER>/settings.json` and each
`<config dir>/<LEGACY_IDENTIFIERS[i]>/settings.json`; with `--settings` and no
`--import-settings` nothing is seeded and nothing under the config dir is
read. The JSON is only read, never written, and never read again once the
store exists, so the two apps' settings diverge from then on. The log names
the store and, for a new one, how many keys were seeded from which file (or
the paths tried).

The store is never deleted or recreated, unlike the index (a cache): a file
that is not a database, or one with a newer `user_version`, is an error left
as it was, and the app runs on the defaults (`Xmp`, the English names, the
default keymap, auto-advance on) with no store behind `Settings::writes`, so
nothing is written; the startup `settings:` line ends `store=none`.

- Rule: `Store::open` reads `user_version` before it sets `journal_mode = WAL`,
  so a store with a newer schema is refused before its header is touched
  (switching a rollback-journal database to WAL rewrites the header). The
  schema and the seed are then written in one `BEGIN IMMEDIATE` transaction
  that re-reads `user_version`, so two processes opening one new store cannot
  both seed it, and (`PRAGMA user_version` being transactional) a store is
  never found with the table but no version. A JSON file opened with
  `Connection::open` succeeds and fails only at the first query ("file is not
  a database"), leaving its bytes as they were; keep that order if the open
  is ever reworked.

Every write goes through `Settings::writes` (`settings::Writes`): a
`set(key, value)` or `delete(key)` is sent to one `settings-store` thread,
which applies them to the store in the order given, so two quick changes of
one key cannot land reversed, and never on the main thread. A failed write is
logged (`settings store: failed to write <key>`) and the in-memory value
stands, as in the Tauri app. `App::on_app_quit` drains them (`drain`, at most
`settings::DRAIN_TIMEOUT`) in a handler of its own, registered whether or not
the index opened (`quit: settings writes drained=…`). One thread with a
channel was taken over `viewed.rs`'s `Latest`: a `Latest` per key would need
one per key and a way to wait for them all on quit, where the channel's
barrier gives the drain for free.

- Rule: every debug or hand-check run passes `--settings` with a test store
  (a fresh `.sqlite` path in the test folder), and `--import-settings` with a
  test JSON when it needs seeded values, or it opens and seeds the user's real
  store; and `--log` with a file in the test folder, or it appends to the
  native app's own `Handpic.log`.

### The folder open reads the sidecars before pass 1 (Measured)

`scan::list` lists with `read_listing(dir, Some(format))`, and
`Scan::prepare` runs `stat_sidecars` and `sidecar::reconcile_of` right after
`reconcile`, on the open's background task and before the scan thread starts,
as the Tauri `scan_folder` orders them, logging `scan sidecars: dir=…
dirty=N changed=N in …ms`; a view-only folder skips it. The sidecars it could
not read go to the pane's notes as `<name>: <message>`. On a copied folder of
8 DNGs with XMP and `.dop` sidecars and a fresh index, the first open logged
`changed=8` and the second `changed=0`, under `"xmp"` and under `"both"`, and
no sidecar's mtime moved.

- Rule: `Index::entries` joins `files`, so on a fresh index it returns nothing
  until pass 1 has written the files' rows; the view reads the judgments at
  the open and again at pass 1's `Done`. The `ratings` rows exist right after
  `reconcile_of`, so a ratings-only read (`Index::rows_of`) would show the
  marks before pass 1, at the cost of the "entry seen" set Step 4's
  `label_known` follows.

### A judgment goes view, row, writer (Measured)

A judgment key on the strip's selection (none in a view-only folder) applies
`judgment::apply` to the view's map and the strip's cells at once, then on
`cx.background_spawn` runs `judgment::store`: `Index::set_rating` on the
writer connection, then `Writer::set`, the Tauri `set_rating`'s order, so a
crash between the two leaves the row dirty. A failure of either puts the
file's previous state back (while the folder is still open) and goes to the
pane's notes, as does the writer's `on_error` through `Event::SidecarError`,
one line per path. The writer is spawned in `Riffle::new` when the index
opened; `Scan::prepare` hands `reconcile_of`'s dirty rows to `Writer::set_now`,
so a judgment lost to a crash inside the 300 ms debounce is written on the
next open (checked on the real exe by seeding a dirty row into a test index:
the reopen logged `dirty=1` and the `.xmp` got `xmp:Rating="4"`).

- Rule: the view keeps `seen` (the paths the index last returned an entry for,
  which make the label known) apart from `judgments`, and `touched` (the
  paths judged since the open), whose state a later `entries` read, such as
  the one at pass 1's `Done`, does not replace, as the Tauri `touched` set.
- Rule: undo and redo go through the same path. A judgment that writes
  records its files' states before it as one `history::Undo` entry (a batch
  over the selection's targets) and clears the redo history; `Ctrl+Z` /
  `Ctrl+Shift+Z` pop an entry, push the files' current states onto the other
  history, apply the popped states as a key would (view, row, writer) and
  move to the file of a one-file entry, selecting it alone when the focus
  moves (`selection::restore`); an entry of several files leaves the current
  file and notes `Undid N files` / `Redid N files`. Each entry has an id, so a failed store drops its file
  from the entry it belongs to (`Undo::forget`, the Tauri `forgetOnFail`).
  Another folder's open drops the judgment batches of both histories and
  keeps the trash runs; a re-listing keeps them and a step drops the files
  no longer listed.
- Rule: `label_known` for a store follows the Tauri `send` rule: the entry was
  seen, or (for an undo / redo, which has no command) the restored label
  differs from the current one. A `ClearAll` on a file whose entry was not seen
  still goes out when it changes nothing (`forceLabel`), so `judgment.rs` has
  `writes` next to `changed`; do not fold the two into one test. A failed
  undo / redo store also drops its file from the entry it pushed onto the
  other history (the Tauri `step` passes no `onFail` for that entry).

### The selection is keyed by path and ends every `refilter` (Inferred)

`selection.rs` is `selection.ts` over the native `judgment` types: a
`Selection { selected, anchor }` of listed paths, the current file always a
member. The view holds one; `set_selection` hands its paths to the strip
(`Strip::set_selected`, indices through the strip's `index_of`), which paints
a selected cell other than the current one as `.cell.selected` (the `--muted`
fill and a `--ring` border), so nothing maps paths per frame.

- Rule: `go` (paging, `home` / `end`, auto-advance, a one-file undo) selects
  the landed file alone only when the current file changed, as `main.ts`'s
  `move`; `focus_on` is the move without touching the selection, which a
  Ctrl / Shift click, `extendPrevious` / `extendNext` and the MCP `show`
  use after setting the selection themselves. A Ctrl click (Cmd on macOS;
  either modifier, as `strip.ts` reads `metaKey || ctrlKey`) toggles the cell
  and keeps the focus; a Shift click selects the range from the anchor and
  moves the focus to the clicked cell; a plain click selects it alone, also
  on the current cell. `selectAll` selects every shown file with the focus
  kept. None of them is refused in a view-only folder.
- Rule: every `refilter` that rebuilds `files` ends with `settle(.., false)`,
  that is `prune`, so a file a judgment or the filter hides leaves the
  selection; a new folder's open lands with `settle(.., true)`, the landed
  file alone. A re-listing goes through `refilter` and so prunes.
- Rule: a judgment's targets are `selection::targets` (the focused file alone
  while one or none is selected) and its changes `selection::judgments`: the
  value decided once from the current file, each file kept on the fields the
  command does not touch, the current file first, a file left out unless
  `judgment::writes` holds for it (its `seen` per path). Auto-advance needs
  exactly one target (`should_advance`'s `targets`).

### The writer drains in `App::on_app_quit` (Measured)

`Riffle::new` registers `App::on_app_quit` (the app-level one, holding an
`Arc<Writer>`, not the entity-level `Context::on_app_quit`: closing the window
can release the view before the quit handlers run) and calls
`writer.flush(DRAIN_TIMEOUT)` inside the handler itself, returning a ready
future. GPUI gives the returned futures only `SHUTDOWN_TIMEOUT` (200 ms), but
the handler is called synchronously from `App::shutdown`, which on Windows
runs once the message loop ends, after `cx.quit()` (`q`, `--auto`) and after
`on_window_closed`'s `cx.quit()` alike. An `--auto 1` run logged `quit: sidecar
writer drained=true` after its summary; the window-close path and a key
inside the debounce are the user's hand check.

The same handler first stops the running scan, before the drains: the view
holds its scan in a `scan::Running` (an `Arc<Mutex<Option<Scan>>>`) whose
clone the handler keeps, since the view may be gone by then.
`Scan::stop_within(STOP_TIMEOUT)` sets the cancel flag and waits on a channel
the scan thread drops its sender of when it ends (`JoinHandle` has no timed
join), logging `quit: scan stopped=<bool> in <ms>ms`. Mid-pass 2 on 3,360
hardlinked ARW files (release, 3 pass 2 threads) the stop took 6-10 ms and
the process exited 47-140 ms after the quit, `scan faces: ... canceled=true`
logged, by `--auto 10` and by `CloseMainWindow` alike. `STOP_TIMEOUT` is 2 s,
as `DRAIN_TIMEOUT`, for a read a slow or network volume holds; past it the
process exits with the scan running, as before.

- Rule: a re-listing still stops the previous scan with the unbounded
  `Scan::stop`: it must not reconcile while the old scan writes. A scan being
  stopped that way when the app quits is not in the holder, so the quit does
  not wait for it.

### The MCP companion answers from the view, one channel hop away (Measured)

With `mcpEnabled` true in the settings store, `mcp.rs` builds a tokio
current-thread runtime on one named `mcp` thread, which runs the backend's
`Mcp::switch(true)` (on `MCP_PORT`, 41917, or `--mcp-port`), waits for the
quit handler's signal and runs `Mcp::stop` (logged as `quit: MCP server
stopped=<bool> in <ms>ms`, at most `STOP_WAIT`). With it off or missing there
is no runtime, no thread and no server. A bind failure is logged with the
`mcp: McpState` line and the app runs on. The backend's `Bridge` send callback
only puts the request into the view's channel as `scan::Event::Mcp`; the
view's `mcp_request` builds a `companion::View` snapshot, `companion::answer`
returns the action or the refusal, the view applies it (`focus_on(Move::To(..))`
with the file selected alone,
`toggle_zoom`, or `judge_paths`) and replies through `Mcp::reply`.
`get_photo` and `get_preview` never reach the view: the backend reads them on
tokio's blocking pool against the index's read-only connection, outside
`ReadSlots`, as in the Tauri app.

`set_judgment` goes the judgment keys' way: `companion.rs` merges the given
fields over each photo's own state (an omitted field kept, a rating of 0 and a
null label clearing), keeps only the photos it changes, with the label known
when the entry was seen or the call changed the label (the Tauri `send`
rule), and `judge_paths` records them as one `history::Undo` entry, marks them
`touched`, applies them, refilters once with the current file kept, and stores
each under that entry's id. There is no auto-advance, and a view-only folder
refuses with `viewonly.ts`'s message. The reply lists every given photo's
resulting stars, flag and label, the unchanged ones too. The tool's English
color names are what `judgment::State.label` holds, so no mapping is needed.
A `curl` check on test copies found the stars, the pick and the label in each
`.xmp` and the `ratings` row after `--auto` quit.

Not supported yet, each refused with a message naming the native app: compare
and a `select_photos` with several paths. `selected` is the current file
alone, `burst` is always empty (no `burst.ts` port), and `current.eyes` is
present only once the `f` overlay has judged the file.

- Rule: a test run with MCP on passes `--mcp-port` with a free port other
  than 41917 (the user's Tauri app may hold it), seeds `"mcpEnabled": true`
  through `--import-settings` into a fresh `--settings` store, and passes
  `--log` with a file in the test folder, next to `--index`.
- Rule: on Git Bash, give `curl` the JSON body on stdin
  (`--data-binary @- <<< "$body"`); a `--data` argument with `\` escapes
  reaches the server mangled. The session id is the `Mcp-Session-Id` header of
  the `initialize` reply, and `set_judgment` paths are matched as the index
  keys them (`\\?\`-prefixed on Windows), not canonicalized.

### A folder change re-lists, never mid-scan (Measured)

`main.rs` holds one `watch::Watch` whose callback sends `Event::Changed` into
the scan's channel; each listing calls `watch::set` before `read_listing`, so a
change during the listing is not missed. `resync.rs` defers a change that
arrives while a listing or its scan runs to that scan's end, one pending at
most, as the Tauri frontend's `resyncPending`: a re-listing mid-scan would
cancel and restart it. A re-listing carries the current file by path, the
strip's offset (`carried_offset`) and the loaded thumbnails
(`carried_indices`), bumps a listing counter that drops the previous listing's
preview reads, and always reads the current preview again (it also refreshes
the status line's `n/total`).

- Rule: a file still being copied in changes size between events, so each
  debounced event re-extracts it (`todo=1 removed=1`) until the copy ends;
  that is expected, not a loop.

### The listing and the shown list are two lists, rebuilt by one `refilter` (Measured)

The view keeps `all` (the listing in natural order narrowed by `--size` /
`--max`, the Tauri `allFiles`) apart from `files` (`all` in the sort in force,
later also filtered), which the pager, the strip, the thumbnails, the hot
list, `current_path`, the status line and the pane's `n / total` index into.
The order reads `facts` (each listed file's orientation, capture time, subsec
and `Focus`, keyed by the listed path) and the judgments, both from the one
`Index::entries` read at the open and at pass 1's `Done`. A view-only folder
is in capture order whatever the key (`viewonly::sort_for`), the key kept for
the next RAW folder. On a fresh index the open has no facts, so a capture
order starts as name order and settles at pass 1's `Done`, the current file
kept: an `--auto 5` run on 6 copied ARWs under `"capture"` paged in capture
order from there, `"rating"` put the rated files first and the rejected one
last, and a JPEG folder under `"name"` paged in capture order.

- Rule: only the open and `refilter` rebuild `files`. `refilter` orders `all`,
  keeps what `filter::passes` lets through (the view's judgment, `facts` and
  `mark::af_eye_state` of each path), returns early when the list is
  unchanged (`filter_list_changed`, forced by a re-listing), else carries the
  pager, the strip and the loaded thumbnails onto the new list (`carry`),
  landing on `anchor_after_filter`'s file over the ordered `all`: the anchor
  when it passes, else the next passing file after it, else the last before
  it. It runs after a judgment, an undo / redo, a failed store's revert, pass
  1's `Done` and a re-listing; it never runs per page. An empty result clears
  the image and the pane's file and shows `FILTERED_TEXT` on the status line
  and in the pane.
- Rule: a re-listing passes the list shown before it (the old `files`, not
  the old `all`) as `previous`: an anchor gone from the listing gives way to
  its next surviving passing neighbour in that list, else its previous one.
  This replaced `resync::carried_current`, the same rule with every file
  passing. The open resumes at `anchor_after_filter`'s file for the
  remembered `last_viewed`, so a remembered file the filter hides resumes at
  its passing neighbour.
- Rule: undo / redo's `listed` is over `all`, not `files` (the Tauri
  `allFiles.includes`), so a judgment on a hidden file is still undone. A
  one-file step anchors on its file when it passes, else on the file shown
  before it, and notes `Undid <name> (hidden by the filter)` when the file is
  not in `files`. Auto-advance after a judgment needs the judged file to be
  still current (`judgment::should_advance`): a judgment that filtered it out
  already moved the cursor.
- Measured: with `{"filter":{"flags":["untagged"]}}` an `--auto 6` run on 6
  copied ARWs (one rejected in its XMP) cycled over the 5 others; untagged +
  0 stars + `none` cycled over the 3 unjudged ones; `stars: [3]` (no 3-star
  file) paged nothing; a RAW copied in mid-run re-listed with the current file
  kept and the new file paged in its natord place.
- Rule: a rebuild bumps `order`, which drops a preview read still out for an
  index into the old `files` (the pager is replaced, so its in-flight flag is
  gone), and `refilter` re-requests the current preview whenever the image on
  screen is not the current file's. `listing` still guards the index reads,
  so a judgment's reorder does not drop the entries read of pass 1's `Done`.
- Rule: a judgment's, a failed store's revert and a step's `refilter` scroll the current file into view
  (the Tauri `keepScroll` false); `Done` and a re-listing keep the same files in
  view (`carried_offset`).
- Measured: `order_files` over 1,000 paths takes 0.4-0.5 ms in a release
  test build and 13-17 ms in the debug one (natord dominates the name order).
- Rule: pass 2's `FacesProgress` patches each written file's `Focus` in
  `facts` (`filter::apply_face_ready`, the Tauri `applyFaceReady`: a file
  without facts or without a focus is left alone) and refilters with the
  current file as anchor and the scroll kept only while a `Face focus` or
  `Eyes` item is checked; `FacesDone` reads nothing. On a fresh index under
  `{"filter":{"candidates":["good"]}}` the open's list and pass 1's are empty
  (no `eye_focus` yet, so every file is `Neutral`) and the 3 good frames of 8
  came in from `FacesProgress` alone; `eyes: ["open"]` paged the 4 open-eyed
  frames.
- Rule: the `Output` section's answer is read on the open's (and the
  re-listing's) background task right after `scan::list`, before
  `Scan::prepare` (`scan::outputs` over `output::outputs_in`, at most two
  `read_dir`s), never on the main thread, and handed to `opened` with the
  listing it is about. No export folder or a read error is no answer:
  `FilterState::set_output_known(false)` clears the `outputs` checks, and an
  error is logged as `output filter: …` and kept in the pane's notes as
  `Output filter: …` until a read succeeds. The section is never stored.
- Measured: `output: dir=… found=N in …ms` (`found=none` without an answer)
  logged 0 ms on 8 copied ARWs with a 3-file `Output\`; renaming `Output\`
  away and back re-listed twice, `found=none` then `found=3`.
- Rule: the EXIF sections' groups (`exif::groups_of`, per group its labels in
  display order) are built from the same `Index::entries` rows on the same
  background task as `facts` (`entries_of`), in the index's order so a label
  two files share takes the last one's order as in Tauri, never per page.
  `set_entries` stores them and prunes the EXIF checks against them
  (`FilterState::prune_exif`) before any `refilter` that follows, so a check
  whose label left the folder cannot empty the strip. Another folder's open
  clears the EXIF checks (`clear_exif`), a re-listing only prunes; they are
  never stored. `passes` reads a file's EXIF from its `facts`
  (`exif::exif_key`), a file without one failing a non-empty group.
- Measured: `groups_of` over 5,000 synthetic entries (six groups, a few dozen
  labels) took 5.8 ms in a debug test build.
- Rule: the pane's `n / total` is the pager's current index over `files`
  (`meta::position`, the Tauri `positionEl`), not the shown file's index, so
  a refilter that hides the file on screen (`p` under `untagged`) moves it at
  once; the name still names the file on screen until the next preview
  arrives. The shown file's index becomes `None` when a refilter hides it
  (`filter::shown_after`): a stale index would name the file then at that
  place in the next `refilter`, which could skip the re-request of a preview
  its `order` bump had just dropped. The status line's `n/total` is left
  alone.
- Rule: a `refilter` that empties `files` keeps its anchor
  (`filter::KeptAnchor`), a non-empty one or another folder's open drops it.
  A menu change, and only a menu change, anchors on it while no file is
  current, so `Reset` or unchecking the item lands on the file shown before
  the list emptied, else on its `anchor_after_filter` neighbour, else on the
  first file. This is a native improvement: the Tauri `refilter` lands on the
  first file there.

### The sort and filter menus are deferred popovers over the window (Inferred)

The status row is a flex row: the status text, then `Sort ▾` and `Filter ▾`
at its right end (`menu.rs` holds the rows, the clicks' `apply` and `Menus`;
`main.rs` only maps rows to `div`s). The filter button is lit while
`FilterState::is_active`; the sort button is dimmed and has no handler in a
view-only folder, whose open closes a sort menu left open. A menu item runs
`menu::apply`, then `refilter` on the current file with the scroll not kept,
so a reorder scrolls the current file into view. `menu_action` then writes
`sortOrder` (`SortKey::setting`) when the sort key changed, and `filter`
(`FilterState::stored`, then `Stored::setting`: the remembered sections, each
in the menu's item order, never `Output` or the EXIF checks) when a remembered
section changed, `Reset` included; a change of `Output` or EXIF alone writes
nothing. Nothing is written at load, so a seeded `recommended` stays stored
until the user picks an order, and a view-only folder, whose sort button is
dimmed, shows capture order and keeps the stored key.

- Rule: the filter menu ends with the folder's EXIF sections, after `Output`
  (or `Eyes` without an export answer), in the Tauri order and headings
  (`Camera`, `Lens`, `Aperture`, `Shutter speed`, `ISO`, `Focal length`),
  each a separator, a heading and one item per label, a group the folder
  lacks left out. Numbers sort by value (`f64::total_cmp`), ties by label;
  `Camera` and `Lens` by a case-insensitive compare, ties by the plain one
  (Tauri uses `localeCompare`; no collator here). The focal length is
  bucketed into `exif::FOCAL_RANGES`, half-open. An item is keyed by its
  group and label (`Toggle::Exif`), not its row, since the groups can be
  rebuilt between the frame that drew the menu and the click, which made
  `Row`, `Item`, `Label` (with `Label::Owned`), `Toggle` and `Action` `Clone`
  rather than `Copy`. The longer menu relies on the popover's scroll.

- Rule: an open menu is `div().absolute().top_0().right_0()` inside a
  `relative()` wrapper of its button, holding `deferred(anchored()
  .anchor(Anchor::BottomRight).offset(0, -4).snap_to_window())`: `anchored`
  without `position` places itself where it is laid out (here the button's
  top-right corner), so no bounds need reading back, and `deferred` paints it
  after everything else. The popover is `occlude()`d, so the strip's
  `on_mouse_down`, which checks `hitbox.is_hovered`, does not see a press
  inside it, and its height is capped at the window's less 40 px with
  `overflow_y_scroll`. Nothing is laid out while both menus are closed.
- Rule: `on_mouse_down_out` (a capture-phase listener, run before any
  bubble-phase `on_mouse_down`) on the popover closes it on any press outside,
  including one on its own button, whose bubble-phase `on_mouse_down` then
  runs for the same press. `Menus` keeps the closed menu and the press's
  position, so that button press does not reopen it (the Tauri toggle sits
  inside `#sort` / `#filter`, which its `mousedown` listener spares). The
  buttons toggle on `on_mouse_down`, not `on_click`, for that reason; items
  use `on_click`.
- Rule: `escape` closes an open menu, taken in the key handler before the
  keymap's actions like `home` / `end` / `q`, and only while a menu is open,
  so a user binding on `escape` still works otherwise (no default binding
  uses it). Every other key goes through the window's handler as before, so
  paging, `e`, `z` and `f` work with a menu open.
- Measured: the preview canvas stays 1380 x 852 logical px in the default
  1600 x 1000 window with the two buttons in the status row (12 px text),
  the same as the plain status line, so the fit scale is unchanged.
- Not verified headless (mouse clicks cannot be driven by `--auto`): that a
  press on an item reaches its `on_click` and not the strip, and that
  `on_mouse_down_out` fires for a press in the strip or the preview. Both are
  read from GPUI's `div.rs` at the pinned rev; the hand check is pending in
  `../plans/_archived/20261113-gpui-filter-sort/learnings.md`.

### The settings modal is its own entity, opened on `ctrl+,` (Inferred)

`settingsview.rs`'s `SettingsView` is an `Entity` the window holds and adds as
its root's last child; `settingsmodal.rs` is its pure model (the open state,
the selected tab, the shortcut row capturing a key, `modal.ts`'s key rule,
`tabs.ts`'s vertical `nextTab`). While closed its
`render` returns `Empty`, one zero-size layout node. While open it is
`deferred(div().absolute().top_0().left_0().size_full())`: in taffy an
absolute child is placed against its parent, here the window's root, so it
covers the window, and `deferred` paints it after everything else. The
backdrop and the box are both `occlude()`d: the backdrop's `on_mouse_down`
closes it (the box blocks the backdrop's hitbox, so a press inside the box
does not), and the strip's `on_mouse_down` and `on_scroll`, the menus'
buttons and the root's focus-on-press all check their hitbox, which
`BlockMouse` drops from the hit test (`window.rs`'s `hit_test` stops at the
first blocking hitbox, for scroll too). The tabs are Sidecar, Culling,
Keyboard Shortcuts and Import (no Cache, MCP or Debug); the modal keeps the
tab across opens and focuses it on open, as the Tauri `open` does.

A change is emitted as a `settingsview::Change`; the window applies it in its
`subscribe_in` callback (the main view stays the owner of the values):
`AutoAdvance` sets `settings.auto_advance` and writes `autoAdvance`,
`Keymap` replaces the keymap (next section),
`Closed` focuses the window's handle again.
The status line under the box shows `NOT_SAVED` while the settings store did
not open.

- Rule: the key routing. GPUI dispatches a `KeyDownEvent` along the path from
  the root to the focused node, capture phase root first, then bubble from the
  focused node up (`dispatch_key_down_up_event`). The box has its own focus
  handle (`track_focus`) and an `on_key_down` that calls
  `cx.stop_propagation()` on every key, so a key with the focus inside the
  modal never reaches the root's handler; the root's `key_down` also returns
  at once while the modal is open, for a focus left on the root. `ctrl+,`
  (`meta+,` on macOS, `settingsmodal::OPEN_KEY`) is taken literally after
  `home` / `end` / `q`, closes an open sort or filter menu, and opens the
  modal; the keymap refuses to bind it.
- Rule: Tab and Shift+Tab are not built in. The box's handler calls
  `window.focus_next` / `focus_prev`, which walk the whole window's rendered
  tab stops and wrap; the modal's controls are the only tab stops (the root's
  handle comes from `cx.focus_handle()`, `tab_stop` false), so the cycle stays
  inside the modal. A control is `.id(..).tab_index(0)`; each tab tracks a
  handle of the view's with `tab_stop(selected)`, the Tauri roving
  `tabindex`, so only the selected tab is in the cycle and ArrowUp /
  ArrowDown / Home / End on it select and focus another (`next_tab`).
- Rule: Enter and Space on a focused control come for free: a focusable
  element with `on_click` turns a key down then up of either into a
  `ClickEvent::Keyboard` (`div.rs`), and its listener runs before the box's
  bubble-phase handler stops the event.
- Rule: no AI toggle. The user decided on 2026-11-14 not to port the AI
  features (`aiEnabled` with the model download, pass 3, genres, the ranking,
  `recommended`); a seeded `aiEnabled` key stays in the store, unread.
- Not verified headless (mouse clicks and the modal's keys cannot be driven
  by `--auto`): the clicks, the occlusion, the focus cycle and the keyboard
  click. All are read from GPUI's `window.rs`, `div.rs` and `tab_stop.rs` at
  the pinned rev; the hand check is pending in
  `../plans/20261113-gpui-settings/learnings.md`.

### The keymap is edited in the modal and replaced whole (Inferred)

The Keyboard Shortcuts tab lists `Keymap::bindings()` in order, labeled by
`settingsmodal::shortcut_label` (`settings.ts`'s `shortcutLabels`), each key a
chip (`display_key`: `space` as `Space`) with a `×`, then `+` or `Press a
key...`, then `Reset`; `Reset all` is under the scrolling list. The view
edits its own copy of the keymap, read from the window on every open, through
`settingsmodal::edit` (`Keymap::add` / `remove` / `reset` / `reset_all`); a
refusal shows the backend's message on the status line and leaves the copy
as it was, a success emits `Change::Keymap`, on which the window replaces
`settings.keymap` and `bindings` (`Bindings::new`) and writes
`Keymap::overrides()` under `shortcuts`, deleting the key when there are none
(`Writes::shortcuts`, the Tauri `update_keymap` rule).

- Rule: the capture sees the raw key in the box's handler. `+` sets
  `capturing` and focuses the box itself, since the pressed `+` is replaced by
  the prompt and a focus left on a node no longer drawn would send the key to
  the window's root, whose handler returns while the modal is open; a `×`
  focuses the box after its edit for the same reason. The next named key
  (`key_name`, a lone modifier ignored) is added, Escape cancels, and a tab
  switch or a close cancels (`SettingsModal::select` / `close`).
- Rule: `settingsmodal::APP_KEYS` (`home`, `end`, `q`) are refused with
  `"<key>" is taken by this app`: `key_down` takes them before the keymap, so
  a binding would never fire. `escape` cannot be captured (it cancels) and
  `ctrl+,` / `meta+,` the backend refuses as a menu accelerator. The folder
  tree's keys (arrows, Home / End, Enter, Escape, type-ahead) are not
  refused: they are taken only while the tree has the keyboard, when the
  keymap's photo actions are swallowed anyway (`treekeys.rs`).
- Not verified headless: the clicks, the capture and the names a shifted
  digit and Ctrl+Alt+digit arrive under ("Every keystroke goes through the
  keymap"); the hand check is pending in
  `../plans/20261113-gpui-settings/learnings.md`.

### A sidecar format switch is the Tauri order, shared (Inferred)

The Sidecar tab's three rows (`settingsmodal::FORMATS`) emit
`Change::SidecarFormat` only for a format other than the one shown
(`format_change`); the window's `switch_format` sets `settings.format` at
once, so the next judgment, from a key or the MCP companion's
`set_judgment` alike, is queued in the new format. Then, on the background
executor and under the view's `format_switch` lock (one switch at a time), it first checks
its format against `format_latest`, the one last asked for, written on the main
thread, and does nothing when a later click superseded it, so back-to-back
switches end persisted as the radio shows (the Tauri `AppSwitchLock` gets the
same by re-checking the current format inside the lock). Otherwise it drains the judgment stores
(`judgment::Queue::drain`: a store queued before the switch carries the old
format and must reach the writer before its flush) and runs the backend's
`sidecar::switch_to`: the writer's flush (`sidecar::DRAIN_TIMEOUT`),
`sidecarFormat` persisted through `Settings::writes`, `Index::reset_sidecars`.
The Tauri `switch_format` calls the same `switch_to` after its own state
write, so the two apps share one copy of that order. Back on the main thread
it clears `touched`, the undo histories and the step note, as the Tauri
reopen (`openDirectory`) clears `touched` and the judgment entries of its
histories, and re-lists the open folder with `relist`, whose
`sidecar::reconcile_of` runs only after the reset. A failure goes to the
modal's status line (`SettingsView::failed`) and, as in the Tauri app, the
folder is not re-listed.

- Rule: a listing already under way when the format changes reconciles with
  the format it captured at its start; the switch's own re-listing comes
  after the reset and supersedes it (`opens` token), so the view ends on the
  new format's judgments.
- Rule: a label names change takes the same path (`switch_names`, both
  through `reset_sidecars` under the `format_switch` lock). While the format
  writes XMP the Sidecar tab shows the five name fields
  (`textinput.rs`), a row of `i18n::presets()` languages and `Reset`; a
  field's commit (Enter or blur; Escape commits and closes, as the Tauri
  field's `change` fires when the closing dialog takes its focus) or `Reset`
  normalizes the five with `sidecar::label_names_setting`, shows them back
  and emits `Change::LabelNames` only when they differ. The window then
  writes `labelNames` and puts the names in force on the main thread (a
  judgment queued before keeps the names it carries; overlapping changes end
  stored as the fields show, so no `format_latest`-style guard), and off it
  drains the stores and runs `sidecar::switch_to` with nothing left to
  persist, then re-lists.

### The Import tab stores a root and a valid pattern, nothing reads them yet (Inferred)

`Settings` reads `importRoot` and `importPattern` with the backend's
`import::root_setting` / `pattern_setting` (an invalid stored pattern reads as
`%Y/%F`). The Import tab shows the root read-only with `Choose…`, the
`importsettings.ts` texts (`settingsmodal::IMPORT_NOTE`, `root_note`,
`IMPORT_PATTERN_EXAMPLES`, `IMPORT_SPECIFIERS`), the pattern in a
`textinput.rs` field and the example line, `import::today_example` (the
native crate has no `chrono`), recomputed in a `cx.observe` on the field on
every notify, so with no debounce; a refusal shows in the error color. A
commit (Enter, blur, Escape then close) or an example button goes through
`settingsmodal::pattern_change`: a valid new pattern emits
`Change::ImportPattern`, which the window writes and keeps in `settings`; an
invalid one leaves the stored one and shows its reason.

- Rule: `Choose…` calls `cx.prompt_for_paths` (directories only) from the
  view and awaits it in a `cx.spawn`; the answer comes on a later turn and the
  view entity lives as long as the window, so `Change::ImportRoot` is emitted
  and written whether or not the modal is still open. It sets only the root,
  so a pattern still being typed stays in the field. A dialog error (Linux)
  goes to the status line.
- Not verified headless: the OS dialog, the typing and the buttons; the hand
  check is pending in `../plans/20261113-gpui-settings/learnings.md`.
  `Import…` itself is port Step 8's.

### The native app keeps its own index (Inferred)

The index lives at `<dirs::cache_dir()>/dev.minodisk.handpic.native/index.sqlite`,
apart from the Tauri app's `dev.minodisk.handpic/index.sqlite`; `--index <path>`
overrides it. The folder is named after the Handpic identifier
(`datadir.rs`'s `IDENTIFIER`, next to `LEGACY_IDENTIFIERS`) and nothing is
migrated from the earlier `com.minodisk.riffle.native` or
`com.minodisk.handpic.native`: the native app never shipped, so an old
developer cache is simply rebuilt by pass 1. Sharing one file would put two processes on it (the Tauri
launch's `VACUUM` against a native scan's writes), make two
`EXTRACTOR_VERSION`s re-extract each other's rows, and let a native run's
`reconcile` drop the user's rows. The cost is a cold pass 1 on each folder's
first native open. An index that cannot be opened leaves the pager working
with `no index: …` on the status line.

- Rule: never point a debug or measurement run at the Tauri app's index; use
  `--index` with a fresh file.
- Rule: `reconcile` drops every row of the folder it was not given, so it
  always gets the whole listing; `--size` / `--max` narrow only what is shown.
- Rule: on Windows `canonicalize` gives `\\?\D:\...`, and every index key and
  listed path in the native app carries that prefix, as in the Tauri app. Key
  anything compared with the index or the scan's `ready` paths (strip cells,
  thumbnails, the watcher's owner string) on `FileStat.path.to_string_lossy()`
  or the canonical `PathBuf`, never on a path rebuilt from the user's argument.

### `run_scan` runs on a `std::thread` (Inferred)

`index::run_scan` blocks for the whole pass and has its own rayon pool. On
`cx.background_spawn` it would hold one of GPUI's background threads, which
the preview decode needs, for as long as the folder takes. `scan.rs` spawns a
thread per scan; a new open cancels and joins the previous one, and the first
scan joins the launch's eviction thread before it reconciles, so the
`VACUUM` never races it.

### The hot list is set before the scan's workers start (Measured)

`run_scan`'s workers take the first file of the hot list not yet taken, else
the next in listing order. With the hot list sent right after `Scan::start`
(as the Tauri frontend does after `start_scan`), the first wave of the 22
workers had already taken files 0-10 of a 373-file folder opened at file 199;
with `ScanFocus::set` called on the handle before `Scan::start`, the first
committed batch was the 20 files of the strip's range around 199 plus 0 and 1
(the two workers left over). Later updates are debounced by
`SCAN_FOCUS_DELAY` (a `cx.spawn` timer task replaced on each change; dropping
a GPUI `Task` cancels it).

- Rule: an index write from the view (`set_last_viewed`, `mark_viewed`) goes
  through the writer on `cx.background_spawn`, one in flight (`viewed.rs`);
  a failure is logged, never shown.
- Rule: `Index::evict` drops a folder whose `opened_at` is over 30 days old
  with its `last_viewed`; seeding a test index by hand needs a current
  `opened_at`.

### A preview holds the scan's pause until its image is painted (Measured)

`load` takes `ReadSlots::pause` before the read and hands the guard back in
`Loaded`; the view keeps it in `PaintHold` (`hold.rs`) and the preview
canvas's paint releases it right after painting the image. Pass 2's workers
wait out a held pause at every stage boundary (`scan::yield_to_pause`), not
only before a read, so the read, the decode, the orient and the paint get the
cores. `MAX_PAUSE` (1 s) still bounds what a worker honours: one budget for all of a
pass 2 file's stage waits, and one per `acquire` (two per pass 2 file, one per
pass 1 file), so a hold never released (no frame painted, say a minimized
window) costs a worker about 3 s per pass 2 file or 1 s per pass 1 file at
worst and only slows the scan. The longer hold also blocks pass 1's `acquire`
from the read through the decode, orient and paint; its effect on pass 1's
throughput was not measured (the pass 1 paging measurement below assumed a
pause around the read only). Holding the pause through the decode (not
yet the paint) cut RAF's A - B from +5.08 to +1.94 ms at 3 pass 2 threads, the
`decode` growth gone, without fewer pass 2 rows per run
([results](../plans/_archived/20261110-gpui-focus-marks/results.md)).

- Rule: anything new on the preview's path between the read and the paint
  runs while the pause is held; keep it there rather than after the release.

### A held key draws each stale preview before chasing the current file (Inferred)

The pager keeps one preview read in flight. When one read + decode outlasts
the key repeat, every result during a hold is another file's; throwing those
away left the fit view blank until the key was released. `Pager::finish`
now decides what a result leads to: the current file's is shown (or its
failure) and clears the in-flight marker; another file's is drawn when its
read succeeded and the current file's image is not already on screen (A shown,
press to B, back to A before B arrives: drawing B would flash the wrong file),
and then the current index is requested, staying in flight. A stale failure is
neither shown nor printed. The drawn frame therefore lags the cursor by at most
one load, the files in between are still skipped, and the number of reads per
key sequence is unchanged.

A stale frame sets the status line, `self.shown`, the image, `marks.show` and
the paint hold, and nothing else. `self.shown` names the file whose image is on
screen because `refilter` rewrites the status line's `n/total` from it and
re-requests when it is not the current file, and the meta pane follows it.
`marks.show` follows the image too: `Marks::mark`, `wanted_faces` and
`wanted_eyes` draw and ask only when the shown file is the current one, so a
stale frame gets no `f` overlay; left at the previous file, stepping back onto
it while a load is out would put its marks on the stale image. `zoom.shown` is
the current file's only, since it stamps the facts with the current page's
`seq`, so `zoom.preview()` stays `None` for a stale frame and the 1:1 canvas
never uses it as the placeholder. The fit view's peaking layer is keyed by the
image id and drawn size, so a stale frame gets its own edges or none.

- Rule: the on-demand analysis (`analyze_current`), the dwell and viewed mark
  (`start_dwell`), the keypress (`self.key`) and the latency `page` line
  (`self.pending`) are the current file's result's only. A `page` line is
  still recorded only for the current file, so the "4-6 pages in 5 s" of a
  held RAF key below counts current-file pages, not frames drawn.
- Rule: stepping back onto the file whose stale frame is on screen still
  reloads it; its analysis, dwell and `page` line wait for that reload.
- Pending: a held key flipping frames and the release showing the current file
  with its overlays is unverified by hand on Windows.

### A held key draws each stale crop before chasing the current one (Inferred)

The 1:1 view's crop chain has the same shape: one read in flight, and a crop
read for an earlier page (`seq`) used to be released. `Zoom` now records the
page each held crop was read for. While zoomed, `page()` keeps the held crop
on screen and asks for the new page's (once, while nothing is in flight); a
stale crop that arrives is held in place of the previous one and the current
page is asked for, so the drawn crop lags the cursor by at most one crop read.
A stale failed read holds nothing new, shows no error and still asks for the
current page; the current page's failure shows its error and releases a held
crop of another page. Not zoomed, a page and a stale arrival release the crop,
so `z` never opens on another file's crop.

- Rule: the canvas never mixes two files. `zoom_paint` draws the held crop
  whatever page it is for, but the placeholder (and the `full` size that
  scales it) only when `Zoom::placeholder_fits`: no crop held, or the held
  one is the current page's. The crop's peaking layer is keyed by its own
  image.
- Rule: the `zoom` timing line, the resize re-cut and the reuse of a held crop
  on `z` or a settled viewport (`toggle`, `settled`) are the current page's
  crop's only; a stale held crop never counts as already having it.
- Rule: a single press while zoomed goes old crop, new crop (no black frame or
  placeholder in between), and a folder switch or re-listing while zoomed
  keeps the old crop until the new one arrives.
- Pending: a held key flipping crops while zoomed, and a single press and `z`
  off / on still behaving, is unverified by hand on Windows.

### Pass 2 runs on pass 1's thread (Inferred)

`Scan::start`'s thread runs `index::run_faces_scan` over `faces_todo` right
after `run_scan`, with the same `ScanFocus` and cancel flag, as the Tauri
app's `run_faces_pass` does on its one task, but on `faces_threads()` (an
eighth of the cores) rather than pass 1's `scan_threads()`: on 22 threads it
cost paging +9.4 ms on ARW and +89 ms on RAF at p50, and on a quarter (6)
still +13 ms on RAF
([results](../plans/_archived/20261110-gpui-focus-marks/results.md)).
One thread and one flag mean `Scan::stop` (and the quit's bounded
`stop_within`) cancels and joins whichever pass is running, and the hot list reaches both passes through the one handle
(`focus_scan` gates on either pass running, as `scan::Status::scanning` tells,
cleared as soon as a re-listing takes the scan, whether or not a new one
starts). The thread starts on every open
with an index, even when pass 1 has nothing to do, so rows below
`FACES_VERSION` still get pass 2; a no-op pass costs one `faces_todo` query.
`Event::FacesDone` is sent also when the pass was canceled, had nothing to do
or `faces_todo` failed, so the view always learns the scan is over. The first
non-empty batch's file names go to the log as `scan faces first:`.

- Rule: the deferred resync (`finished`) runs at `FacesDone`, not at pass 1's
  `Done`, as the Tauri frontend drains its deferred rescan at `faces-done`. A
  re-listing during pass 2 would be harmless (a canceled file keeps its old
  `faces_extractor`, so the next scan resumes it) but would restart the pass;
  waiting for its end keeps a burst of changes from restarting it over and
  over.

### The backend logs through `log`, printed by `logger.rs` (Hit)

Without a logger the backend's `scan extract` lines went nowhere. `logger.rs`
prints `[target][LEVEL] message` to stderr at `Info`, which also shows GPUI's
own lines (the UI font, the GPU, and an `ERROR` for the missing DXGI debug
interface on a machine without the Graphics Tools, which is harmless).
`RUST_LOG=debug` (or `trace`) adds this crate's own lines of that level, such
as `key: <name> action=…` for every keystroke, and not GPUI's.

`logger.rs` also writes each line, as
`[YYYY-MM-DD][HH:MM:SS][target][LEVEL] message` in UTC, to a `Handpic.log`
under the native app's own folder, or to `--log <path>` (see
`app-log.md`'s "The GPUI app's log"), with one `write` per line on a
`Mutex<File>` opened for append and the size kept in memory; past 1 MB it
starts anew as the Tauri app's does. `main` installs it once the arguments
are parsed and the backend's panic hook right after it. The `page` / `zoom`
timing lines stay `println!` to stdout.

## GPUI

### Callbacks from other threads go through one channel (Inferred)

`AsyncApp` is not `Send`, and a `BackgroundExecutor` task cannot update an
entity, so a callback on a rayon or watcher thread cannot touch the view. The
view owns one `futures::channel::mpsc::unbounded` sender; the callback only
sends, and a `cx.spawn` task drains the receiver with `this.update(...)` per
message. Each scan message carries the scan's id, so a stopped scan's late
messages are dropped.

- Rule: a callback from a non-GPUI thread only sends into that channel.

### `RenderImage` holds BGRA and must be released (Inferred)

`RenderImage` holds BGRA in an `image::RgbaImage`. Every image handed to the
renderer must be released with `cx.drop_image(image, None)` or it leaks on the
GPU.

### Textures have one mip level (Inferred)

A large downscale (a RAF fit to a 1600 px window) may alias. Not settled: one
captured 1:1 crop of a 4416x2944 RAF fit view, a clapboard wall without fine
repeating texture, shows no moire, but no comparison with the Tauri app or a
hard-case image has been made. Three native-side RAF frames showed no moire;
the comparison with the Tauri app is pending (see the pending manual check in
the gpui-port plan's `learnings.md`).

### The image sprite cannot rotate (Inferred)

`PolychromeSprite` has no `transformation`; only the monochrome SVG sprite
does. A rotation has to happen on the pixels before painting.

### There is no after-`Present` hook (Measured)

`cx.defer` from the paint closure runs after the frame's `draw()` and
`present()`, which is the stamp the spike's timings used. It is a workaround
and may change with a GPUI bump. `crates/native` follows that pattern: the
paint closure stamps `paint_started` and `paint_finished`, then the deferred
callback stamps `presented`, prints the `page` line and releases the retired
images with `cx.drop_image`. Doing the release earlier than that deferred
callback could pull a texture out from under the frame being presented.

### Rotating on the CPU needs threads for a RAF's budget (Measured)

Windows, release, idle, 31 runs of synthetic BGRA buffers, p50 in ms, 90 degree
turns (orientation 6 / 8): 1616x1080 plain loop 3.2, 64-pixel tiles 1.6, tiles
over 8 threads 0.9; 4000x3000 plain 62, tiles 21, tiles over 8 threads 9-11;
4416x2944 plain 70, tiles 23, tiles over 8 threads 12-13. Allocating the output
alone costs about 8 ms at 4000x3000 (a plain copy of the buffer takes that long).
In the app on real portrait files: 1616x1080 ARW `orient` 1.3-1.5 ms, 1920x1280
RAF about 2.2 ms, 4000x3000 RAF (GFX 100) 7.6-8.5 ms.

- Rule: `orient.rs` tiles over 8 scoped threads because the tiled loop alone
  misses the plan's 15 ms budget on a 4000x3000 RAF; orientation 1 returns the
  buffer without a copy and 3 reverses in place.

### mozjpeg reports corrupt input by unwinding (Inferred)

`mozjpeg` 0.10's decoder calls `resume_unwind` on a libjpeg fatal error, so a
corrupt file would otherwise end the background task and leave the pager's
request in flight forever.

- Rule: wrap every decode in `catch_unwind`, as `load` does through `guarded`.

### The strip is a `canvas` over a pure model, not a `uniform_list` (Inferred)

`uniform_list` is vertical-first; its horizontal mode
(`ListHorizontalSizingBehavior::Unconstrained`) is about item width, not a
horizontal virtual list. A flex `div` with one child per file lays out every
cell each frame. The strip is one `canvas` that paints only the cells on
screen, fed by `strip.rs` (offset, range, hit test, request order), whose
range can go to `ScanFocus` as it is. Its thumbnails come from
`thumbs.rs`, four reads at a time through the index's read-only connection,
and every image a cell drops goes through `retired`, like the preview's.

### A `canvas` gets no input; its wrapping `div` does (Inferred)

`canvas` registers no hitbox and takes no listeners. The strip's canvas sits in
a `div` with `on_mouse_down` and `on_scroll` (`on_scroll_wheel` is a deprecated
alias at this rev, and `ScrollWheelEvent` of `ScrollEvent`). The events carry
window coordinates; the strip spans the window from its left edge, so the
event's `x` is the strip's `x` and `strip::hit_test` takes it as is. The div's
hitbox already keeps clicks and wheels above the strip out.

- Rule: a canvas that needs input sits in a `div` that owns the listeners, and
  maps the event position against where the canvas is laid out.

### A Windows mouse wheel arrives in lines (Inferred)

`gpui_windows` turns `WM_MOUSEWHEEL` into `ScrollDelta::Lines`, the notch
times the system's wheel-scroll lines (3 by default), positive away from the
user; Shift moves it to `x` (`wheel_scroll_chars`). A precision touchpad
through Direct Manipulation gives `ScrollDelta::Pixels`. `pixel_delta(window.line_height())`
turns either into pixels; the strip subtracts it from its offset, `x` when
non-zero, else `y`, so a wheel toward the user scrolls right.

### A stroked path has butt caps, and no cap option is reachable (Inferred)

`PathBuilder::stroke(width)` builds lyon's `StrokeOptions::default()`, whose
caps are `LineCap::Butt` and joins `Miter`. `with_style(PathStyle::Stroke(..))`
could set square caps, but gpui re-exports `StrokeOptions` and not lyon's
`LineCap`, so naming one would need a direct `lyon` dependency. The `f` mark's
crosshair arms are therefore lengthened by half the stroke width at both ends
(`overlay.rs`'s `segments`), where the Tauri mark's `lineCap = "square"`
reaches, so each arm end keeps the 1 px dark outline. The AF frame is a closed
polygon (`add_polygon(.., true)`) and needs no caps.

- Rule: stroke an open path that must look like a canvas square cap by
  extending its ends by half the width; do not add `lyon` for it.

### A canvas paints text through the window's text system (Inferred)

A `canvas` has no child elements, so the file names are shaped with
`window.text_system().shape_line` (one line, centered with
`TextAlign::Center` over the cell width) and the failure text with
`shape_text` and a wrap width, each clipped with `with_content_mask` to its
cell. The run's font comes from `window.text_style().to_run(len)`.

### Text has no stroke; a label's halo is dark copies (Inferred)

GPUI shapes and paints glyphs filled only; there is no text stroke like the
canvas `strokeText` the Tauri face labels use for their 3 px dark halo. A
dark quad behind the label was the other option; it hides more of the face
than the Tauri halo does. `overlay.rs`'s `paint_label` instead paints the
label in `rgba(0, 0, 0, 0.8)` at the eight 1 px offsets and then in the
label color on top, a ring about as wide as the Tauri stroke's outer
1.5 px, which keeps the look close to the Tauri app's. That it reads on a
bright face is not yet checked by eye. A dot is a `paint_quad` whose corner radii are
half its side, with no `PathBuilder` arc. The label's line height is the
shaped line's `ascent + descent`, so its origin is the text's top, as the
canvas's `textBaseline = "top"`.

- Rule: halo text over a picture with offset dark copies; do not paint a box
  behind it.

### An on-demand judgment is superseded through a shared atomic (Measured)

`ondemand::read_eyes` runs on `cx.background_spawn` and asks its `current`
closure before the detection and before the model. The view cannot answer
from another thread, so `eyes.rs`'s `EyesCache` keeps the latest ticket's id
in an `Arc<AtomicU64>` the closure compares its own id with; a newer request
stores its id and a clear (another folder, a re-listing) stores `0`, which no
ticket has. Opening another folder while a judgment was in its detection
logged `eyes ... model=0.0ms superseded` on the `marks` hand-check folder,
twice out of two runs. Only one judgment runs at a time and the file current
when it settles is asked for next, so 60 presses 30 ms apart over a 300-ARW
folder logged 7 judgments, not 60.

- Rule: hand a background job a cheap `Fn() -> bool` over a shared atomic to
  stop it; do not try to reach the view from the worker thread.

### Key names on Windows are lowercase (Measured)

`KeyDownEvent.keystroke.key` is `"right"`, `"left"`, `"home"`, `"end"` and `"q"`,
checked with injected `keybd_event` key events in the running app. A non-extended
`VK_RIGHT` / `VK_LEFT`, the keypad's, gives the same names.

Injecting keys with `keybd_event` needs the app window in the foreground for
every key; in a long injected sequence most keys went missing once focus moved
away. Re-assert and check the foreground before each key (skip the key
otherwise), and pick the process by its path in this worktree, since another
worktree's `riffle-native.exe` may be running or exiting. Report a skipped key
rather than counting it as a page. Held-key measurements also need an idle
machine: another session's `cargo` build fails the gate, so redo the slot.

### Every keystroke goes through the keymap (Inferred)

`keys.rs`'s `key_name` names a keystroke as the keymap does: the modifiers
held in `ctrl+alt+shift+meta+` order (`platform` is `meta`), then the key
lower-cased, GPUI's `left` / `right` / `up` / `down` as `arrowleft` ...
`arrowdown`; a lone modifier has no name. `Bindings` maps each bound key to
its action, built at startup and again on every Keyboard Shortcuts edit
(`self.bindings` replaced, the lookup still one `HashMap` read). The window's handler, past the tree's gate while
the tree has the keyboard ("The tree takes the keyboard through one flag in
the view"), takes `home`, `end` and `q`
literally first (the native app's own keys, bound to no action), then the
actions: `previous` / `next` page and `focus` toggles the `f` overlay and `zoom` the `z` 1:1 view, so a
user's override of those keys applies; a judgment action goes to
`judgment::Command::from_action`, `undo` / `redo` to `history::Undo`, and any
other action (`rejectRest`, ...) is logged at `debug` until its step lands. A
later step's key (peaking `e`) is added as its keymap action (`peaking`) in
that `match`, not as a literal key: a literal `"e"` arm placed after the action
arms never fires with the default keymap, because the catch-all
`(_, Some(action))` arm takes the key first.

Read from `gpui_windows` at the pinned rev, not yet seen in the running app
(the pending manual check in the gpui-judgments plan's `learnings.md`):

- A shifted letter arrives as the lower-case letter with `shift` set
  (`shift+x`). A shifted digit or punctuation key arrives as the character it
  types with `shift` cleared (`!` for Shift+1, from `get_shifted_key`),
  where `keys.ts` names it from `event.code` (`shift+1`); no default key is
  one, but a user override such as `shift+1` would not match.
- With Ctrl or Alt held, a digit is `MapVirtualKeyW`'s unshifted character,
  `1` on a US or JIS layout (`ctrl+alt+1`), but `&` on AZERTY, where the
  Tauri app's `event.code` still says `1`. Windows reports AltGr as Ctrl+Alt,
  so on a layout with AltGr `ctrl+alt+<digit>` is also what AltGr+digit
  sends; the default color keys are the Tauri app's, so both apps behave the
  same there.

### The canvas's size reaches the view through a shared cell (Inferred)

The preview canvas's bounds are known only inside its paint closure. The
closure writes the size in device pixels (`bounds.size * window.scale_factor()`,
rounded) into an `Rc<Cell<(u32, u32)>>` the view also holds, as `retired` and
`pending` are shared; the 1:1 view reads it when it asks for a crop. A deferred
entity update from the closure was the other option and needs more of
`main.rs`. The value is the last painted frame's: a resize is seen by
`observe_window_bounds` before the next paint, so the crop is re-cut only
after `zoom::CROP_RESIZE_DEBOUNCE` (150 ms), by when the new size has been
painted.

- Rule: read the cell after at least one paint; before that it is `(0, 0)`
  (`crop_size` clamps it to 1).

### 1:1 is one JPEG pixel per device pixel (Inferred)

`paint_image` snaps the visible bounds to device pixels (`snap_bounds`) and,
when the image runs past the clip bounds, cuts the atlas sub-tile by the
visible fraction, rounded. `zoom::Paint` therefore places the upright crop with
its point of interest on a whole device pixel (`canvas_center` floors the
canvas center in device pixels) and gives it its own size divided by
`scale_factor`, so both edges land on whole device pixels and the sub-tile is
cut on whole texels. The window ran at scale 1.0 (96 dpi, a 1600x1000 client)
during the check; that the crop shows without resampling blur by eye is
pending the user (see the zoom-peaking plan's `learnings.md`).

- Rule: anything meant to be 1:1 is placed in device pixels and divided by
  `scale_factor` at the end, never laid out in logical pixels.

### A crop is on screen 8-112 ms after `z` on a busy machine (Measured)

Release build, `zoom` lines of `z` presses on the zoom-peaking test folder,
not idle (other sessions ran), the file cache warm, 25 lines at scale 1.0
(crops of 1024 px per axis at most): four ARW files `read` 0.7-4.7 ms,
`decode` 3-27 ms, `orient` 0-20 ms on the portrait ones, `keypressToPixels`
8-43 ms (112 ms with the outlier below); two RAF files (X-E5, X-H2S) `read`
1.6-5.2 ms, `decode` 14-33 ms, `keypressToPixels` 19-89 ms; two JPEG-only
files `decode` 21 and 47 ms.
`decode` includes the RGBA to BGRA swap. Two `orient` outliers of 36 and 76 ms
(a 1024 px crop over 8 scoped threads, 1-2 ms otherwise) are attributed to the
load, not measured apart.

### Peaking runs off the main thread, one job at a time (Measured)

`peaking.rs`'s job (resize to the drawn size, then the Sobel) takes 25-60 ms on
a fit view, several frames, so it runs on `cx.background_spawn`, reading the
shown `RenderImage`'s bytes through its `Arc` without a copy. One job is in
flight, as the Tauri worker holds one request: under a held key each page
would otherwise queue a job on the executor the next preview decode runs on.
A layer replaced while a job is out is never computed; the reply takes the
newest unsent layer next. The fit view's drawn size is known only in the paint
closure, so the closure compares it with the layer the view rendered with and,
when they differ, asks for the new layer through a `WeakEntity` update in the
`cx.defer` it already runs after paint; while the layer matches nothing is
deferred.

- Rule: an overlay is drawn only for the `ImageId` and device size it was
  computed for (`peaking::Paint`), so another file's or another size's never
  shows; replaced and cleared overlays go through `retired`.

### The fit view's overlay is resized bilinearly (Measured)

The overlay is computed at the drawn device size, as the Tauri app rasterises
the bitmap at that size first. `image::imageops::resize` with
`FilterType::Triangle`, the nearest to a canvas `drawImage`, was kept: at
scale 1.0 it takes about 22-25 ms for a 1616x1080 ARW to 1275x852 and 47-67 ms
for a 4416x2944 RAF to 1278x852, the Sobel 5-7 ms on top. So on a RAF the
overlay comes about 50-60 ms after the image; the held-key measurement showed
no cost to paging, so no cheaper filter was tried. The 1:1 crop needs no
resize (Sobel 4-6 ms).

### Text is typed through `textinput.rs`, the port of GPUI's input example (Inferred)

GPUI at the pinned rev has no text input element. `textinput.rs` is a port
of `crates/gpui/examples/input.rs` (Apache-2.0, the source named in its
header): a focusable `div` around a custom `Element` that shapes one line,
paints the caret, the selection and the IME's marked text underlined, and
registers the entity as the window's input handler
(`window.handle_input` with an `ElementInputHandler`) on every paint, so the
platform's text input and IME reach `EntityInputHandler`'s
`replace_text_in_range` / `replace_and_mark_text_in_range` only while the
input is focused. The owner reads the text with `text`, sets it with
`set_text` and hears `Event::Commit` (Enter, or the focus left: `cx.on_blur`)
and `Event::Cancel` (Escape).

- Rule: the editing keys (backspace, delete, the arrows and Shift+arrows,
  Home / End, select-all, copy, cut, paste, Enter, Escape) are taken in the
  input's own `on_key_down` from `keys::key_name`, the clipboard ones on
  `ctrl+` on Windows and Linux and `meta+` (cmd) on macOS. The example binds
  them with `actions!` and `cx.bind_keys` as `cmd-a` / `cmd-c` / `cmd-v` /
  `cmd-x` only, so on Windows Ctrl+A and the rest did nothing there; here
  nothing enters GPUI's global keymap, so the pager's key path is unchanged.
- Rule: a stopped key down types nothing on Windows. `gpui_windows`'s message
  loop calls `translate_accelerator` first, which dispatches `WM_KEYDOWN` to
  the window; when a listener stopped it (`propagate` false) the loop skips
  `TranslateMessage`, so no `WM_CHAR` follows. A consumed editing key
  therefore never also inserts a character, and a key meant to be typed must
  not be stopped by anything on the dispatch path: the settings box's handler
  lets a key it does not take go on while a name field is focused
  (`settingsmodal::stops`), and the window's root handler returns without
  stopping it while the modal is open. `WM_CHAR` then reaches the focused
  input's handler. A key the IME takes arrives as `VK_PROCESSKEY`, for which
  `MapVirtualKeyW` gives no character, so GPUI raises no `KeyDownEvent` and
  `TranslateMessage` always runs: composition never passes through the
  listeners, and Enter or Backspace inside a composition stay the IME's.
- Seen running (the user, 2026-11-14, Windows, the example itself built
  standalone at the pinned rev): Japanese IME composition, the candidate
  window placed at the text and the commit all work. Not yet seen in the
  native app (the settings' label name fields): the pending check in
  `../plans/20261113-gpui-settings/learnings.md`.
- Limits kept from the example: one line, no horizontal scroll (long text is
  clipped), no undo, no word movement, no double-click selection. Its third
  test (one selection rectangle on a 10 000-character line) needs a shaped
  line and so GPUI's `test-support`, which this crate does not enable; it is
  ported onto the rectangle helper (`span`).

### A launch without a folder shows only the tree (Inferred)

`start` opens the command line's folder, if any, and nothing else: the OS
folder picker (`cx.prompt_for_paths`) is no longer shown at launch, and a
folder is opened from the left pane's tree. With no folder open the window
shows the tree, an empty preview and strip, and `no folder` on the status
line; every handler already tolerated `dir: None` and an empty `files` (the
pager's `go` and `request` are no-ops at length 0, `judge` and `step` need a
current path, `zoom::toggle` needs files, the strip's hit test finds no cell).

When it was still used, `cx.prompt_for_paths` with `PathPromptOptions {
directories: true, files: false }` was the OS folder dialog; Enter on a typed
path navigated into the folder, and a second Enter selected it (Measured).

### The folder tree is a `uniform_list` over rows kept in the view (Inferred)

The left pane (220 px, the Tauri `#side`) sits beside the preview canvas and
above the strip, which still spans the window, so `strip_clicked` still takes
the event's window `x` as the strip's. Its rows are `tree::Tree::rows`,
rebuilt into the view's `tree_rows` whenever the tree changes (never in
`render`), and drawn by a `uniform_list` over them: it lays out only the rows
on screen, its rows are plain `div`s with their own click listeners (the
expander's `on_click` stops propagation so the row's open does not run), and
its `UniformListScrollHandle::scroll_to_item(.., ScrollStrategy::Nearest)` is
what the reveal and the tree's keyboard scroll a row into view with. A
`canvas` like the strip's would need its own hit testing and scroll model; the
strip needed one because `uniform_list` is vertical only.

- Rule: tree nodes are keyed by the tree's own spelling (the root's, then one
  separator per level), never by the canonical `\\?\` path the index uses.
  The open folder's row is found with `tree::relation`, case ignored on
  Windows and macOS (`tree::IGNORE_CASE`).
- Rule: every open (`open`, from the command line or the tree) bumps the
  `opens` counter and reveals the folder: the chain from its root (a root
  added for a folder none holds) is expanded and listed one level at a time on
  `background_spawn`, respelled from each listing, and the end row is
  highlighted and scrolled to. A step that finds `opens` moved on stops, so a
  later open owns the tree. An open before the roots have been read waits in
  `reveal_wanted`.
- Rule: the same counter guards the listing of the open: its background task
  sets the folder watcher under a separate lock (never taken by the UI
  thread, which bumps the atomic counter without locking) only while its
  open is still the latest, and its result is dropped otherwise, so a quick second
  open is never overwritten by the first's late listing, nor left with the
  first folder's watcher.
- Rule: listings settle in order. Each one's background task first awaits the
  previous listing's `settled` signal (a oneshot kept on `Riffle`), then stops
  the previous scan and joins the launch's eviction, then signals its own, so a
  second open never reconciles or scans while an earlier listing is still
  stopping the old scan or the eviction is still deleting rows.
- Inferred: the preview canvas is 1160 x 852 logical px in the default 1600 x
  1000 window with the tree shown (1380 less the pane's 220), and 1380 x 852
  with it hidden by `toggleLeft`. Not measured in the running app. The toggle
  lasts for the session only and re-cuts a 1:1 crop through `zoom_resized`.

### The tree follows the disk through one `TreeWatch` (Measured)

The view holds one `riffle_backend::treewatch::TreeWatch` whose callback sends
`Event::TreeChanged(path)` (the tree's spelling) into the scan's channel.
`tree_changed`, which every change of the tree runs, computes
`Tree::watched_folders` and, when it differs from the set last handed over,
hands the whole set to `TreeWatch::set` on `background_spawn`, each hand-over
awaiting the one before it (the Tauri `set_tree_watches` chain), so two sets
never apply out of order. `TreeChanged` re-lists the node only while it is
still expanded and drawn (the event and a collapse can cross); a failed
re-list is logged and leaves the node as it was, since deleting a folder
usually fails its own re-list before its parent's removes the row. The
menu's `Refresh` re-lists the folder expanded or not and forgets the set
last handed over, so the next change sends it again and a watch that failed
is retried; on the open folder it also re-lists the folder as a watcher
change would (through `Resync`). The roots are read again whenever the
window becomes active (`observe_window_activation`, once the first read has
answered); `add_roots` skips the roots already shown.

- Measured (a throwaway test against `crates/native/Cargo.lock`'s `notify`
  8.2.2 on Windows): `set` of one folder took 0.4-0.6 ms, a collapse
  (`unwatch` then `settle`) 0.1-0.3 ms, five collapse / expand rounds each
  stopped and resumed the events (none for a change while collapsed, one
  ~500 ms after a change once expanded again), and renaming the folder's
  parent after the last collapse succeeded, so the handle was released. The
  `configure` that queued behind `Unwatch` (`tauri-app.md`) does not stall
  here. `TreeWatch` is a backend test, which cannot run from the native
  workspace (a path dependency with dev-dependencies is not testable there),
  hence the throwaway.
- Rule: on Windows a watch pins its folder's ancestors against a rename;
  nothing in the native app renames yet, so `with_released` is not called.

### The right-click menu is a deferred popover at the pointer (Inferred)

`context.rs` holds the menus' items and the open menu (`Menu`: where it was
opened, in window coordinates, and its target, a tree folder or the strip's
focused file and targets); `main.rs` maps them to `div`s. A right press
(`on_mouse_down(MouseButton::Right, ..)`) on a tree row opens the folder
menu, and one on a strip cell (the strip's wrapping `div`, `strip::hit_test`
giving the cell) the strip's menu, closing the status row's menus. The
popover is `div().absolute()` holding `deferred(anchored().position(at))`:
`anchored`'s default `AnchoredPositionMode::Window` takes the
`MouseDownEvent`'s window position as is, and its default fit,
`AnchoredFitMode::SwitchAnchor`, puts the menu on the pointer's other side
when it overflows an edge and then still snaps it inside, as the Tauri
`menuPosition`. `snap_to_window` would only shift it, so a menu opened on
the strip at the window's bottom would cover the pointer. It is
`occlude()`d with `on_mouse_down_out` closing it on any press outside, of
any button, so a right press on another row closes the menu in the capture
phase and opens that row's in the bubble phase; any key closes it (`escape` stops there,
the others then act as usual), as in the Tauri app. Items use `on_click`; a disabled item is dimmed with no
handler.

- Rule: the strip's right-click follows the Tauri strip's `contextmenu`
  handler: a cell outside the selection becomes the selection alone, one
  inside keeps it, and the focus moves to the cell either way; the strip's
  empty area opens nothing. The menu's targets are `selection::targets`,
  taken when it opens. Its judgment items carry the keymap's action names
  (`Action::Judge`) and run through `Command::from_action` and `judge` as a
  key does, so they act on the selection as it is when picked; each item's
  shortcut is its action's first key, shown as `keys.ts`'s `displayKey`
  (`space` as `Space`), not `Keymap::accelerator_for`, which spells keys for
  the Tauri menu bar.
- Rule: the reveal item calls `cx.reveal_path`, which on Windows selects the
  item in its parent through the desktop folder's `ParseDisplayName` (`open_target_in_explorer`)
  and only logs a failure. A root (`D:\`) has no parent, so the folder menu
  opens it with `cx.open_with_system` instead. The tree's paths never carry
  the verbatim `\\?\` prefix, but the strip's files do (the index's
  canonical folder). The shell refuses a prefixed path: `SHParseDisplayName`
  returned `E_INVALIDARG` (`0x80070057`) for `\\?\C:\Windows\notepad.exe` and
  `S_OK` without the prefix (measured from PowerShell, 2026-11-14), so the
  strip's reveal passes `context::shown_path` (`trash.ts`'s `shownPath`).
- Rule: the copy items write the tree's spelling of the path and the row's
  name, or the strip's targets' listed paths (prefix and all, as the Tauri
  app writes them) and file names, one per line, through
  `cx.write_to_clipboard(ClipboardItem::new_string(..))`.
- Not verified headless: the hand checks are pending in
  `../plans/20261113-gpui-tree-trash/learnings.md`.

### The tree takes the keyboard through one flag in the view (Inferred)

The window keeps its one `FocusHandle`; `tree_keys` on the view says whether
the tree has the keyboard, and `key_down` checks it right after the
right-click menu's close and before every other arm (the literal `home` /
`end` / `q`, the status row's menus' `escape`, the keymap's actions). A
second `FocusHandle` on the pane would give the same routing, but the gate
must also sit before those literal arms in the window's handler, and a press
outside the pane must still reach its own handler, so a flag and the pane's
mouse listeners were the smaller change.

- Rule: a left press in the pane (`on_mouse_down(Left)`) gives the tree the
  keyboard; a press of any button outside it (`on_mouse_down_out`, the
  capture phase, so the press still reaches its target) hands it back; a
  right press in the pane does neither. A press on the right-click menu hands
  it back too (`on_any_mouse_down` on the popover, which is `occlude()`d and
  may be drawn over the pane), as a Tauri menu button takes the focus.
  `escape` and hiding the pane (`toggleLeft`) hand it back as well. The
  type-ahead buffer starts over whenever the keyboard moves.
- Rule: while the tree has it, `tree_key_down` takes `escape`, the arrows,
  `home` / `end`, `enter` and any printable key without Ctrl / Alt / Meta
  (type-ahead); any other key runs its action only when
  `treekeys::tree_gate` lets it through (`toggleLeft` today) and is
  otherwise swallowed, so `q`, the judgment keys, paging and undo never reach
  the photo. The cursor row is outlined in `--sidebar-ring` only while the
  tree has the keyboard.
- Rule: opening a folder from the tree, by a click on its name or `enter`,
  leaves the keyboard with the tree, as in the Tauri app (`folders.ts`'s
  `openFolder` and `main.ts`'s `openDirectory` never blur the tree); the
  open's reveal puts the cursor on the opened folder.
- Inferred from `gpui_windows`' `process_key`: type-ahead takes the
  keystroke's own `key_char` (`keys::typed_char`), since `key_name`
  lower-cases a shifted letter and names the space bar `space`. `key_char` is
  `ToUnicode`'s character with a control character dropped, so `enter`,
  `tab` and `escape` type nothing. Type-ahead with an IME on is not a goal.
- Rule: `Expand All` walks one listing at a time on `background_spawn`, depth
  first through an explicit stack of chains (no async recursion), skips a
  link (`is_link`), stops a branch whose chain was collapsed or dropped
  meanwhile and marks and reports a folder that fails, going on with the
  rest. `Collapse All` is `Tree::collapse_all`. The watch set follows through
  `tree_changed`.

### The trash dialog is a layer of the window, its focus in the model (Inferred)

The strip menu's `Move to Trash…` and `Delete Permanently…` open one dialog,
drawn by `main.rs`'s `trash_dialog` as the settings modal is (a `deferred`,
`occlude()`d `absolute` layer over the window's root) but without an entity
or a focus handle of its own: `trash::TrashFlow` holds the phase (idle,
asking, running) and which of its two buttons has the focus, and the
window's `key_down` hands every key to it first, before the right-click
menu's close and the tree's gate, while it is busy. The buttons take mouse
clicks only (no `tab_index`), so `tab` and Enter are the model's.

- Rule: the delete dialog opens on Cancel and takes only a single mouse click
  on its run button as a yes. `ClickEvent::click_count` is 1 for a keyboard
  click at the pinned rev (`interactive.rs`), so the handler passes 0 when
  `is_keyboard()`, the DOM's `detail`; `KeyDownEvent.is_held` (set from the
  repeat bit on Windows, `gpui_windows`' `events.rs`) is passed along with
  the keystroke for `swallowed_by_delete`.
- Rule: while a run is in flight (not held) the dialog stays open with both
  buttons disabled, so the window takes no click and no key until it comes
  back, as the Tauri dialog's running phase.

### A trash or delete goes through the gate, and nothing lists under it (Inferred)

The native app has no `Scans` lock; one view thread decides when a listing
starts, so the guard is `idle::IdleGate` plus `resync.rs`. A confirmed run
asks the gate with "busy" being `Resync::busy` (a listing or its scan under
way) or a run in flight: idle, it runs at once on `background_spawn`
(`judgment::Queue::drain`, then `Writer::flush`, both `DRAIN_TIMEOUT`, then
`trash::trash_listed` with the view's `Arc<trash::Runs>` or
`trash::delete_listed`, progress through the view's event channel as
`Event::TrashProgress`); busy, it is held, the dialog closes and the scan is
canceled without a join (`Running::cancel`), so it still ends through `Done`
and `FacesDone`, whose `finished` drains the gate first. A run held while the
listing itself ran has no scan yet; `opened` cancels the scan it starts when
`cancels_scan(gate.waiting())`.

- Rule: while a run is in flight the gate's in-flight count is up and
  `Resync::hold` keeps the resync busy, so a watcher change or a tree
  `Refresh` is deferred; `relist` returns at once (a sidecar format switch);
  an `open` (the tree's) is kept in `deferred_open` and runs at the run's
  end. The run's end calls `Resync::open` (dropping the pending change) and
  re-lists once, which also rescans what a canceled scan left. `TreeChanged`
  lists a tree folder only, never the open one.
- Rule: every caller of `relist` (a sidecar format switch included) calls
  `Resync::open` first, so `Resync::busy` covers the listing it starts and a
  trash or delete waits for it.
- Rule: a folder open discards a held run; a held run that drains while a
  dialog is asking is dropped, as the Tauri closure checks `trashFlow.busy`.
- Rule: the run's result goes to the pane: `trashed_status` in the
  `step_note` line (the Tauri `note`), each failure through `write_failed`
  (`<name>: could not move to the Trash: <message>`), and after a delete the
  deleted files' judgments leave both histories (`Undo::without_paths`). A
  move whose `Summary::run_id` is set pushes its run on the undo history
  (see the next entry).
- Not verified headless: the dialog, its keys and clicks, the Recycle Bin
  and the cancel of a running pass 1; the hand checks are pending in
  `../plans/20261113-gpui-tree-trash/learnings.md`.

### A trash run is undone through the same gate, and outlives a folder switch (Inferred)

`history.rs`'s entry is `Entry::Judgments(Batch)` or `Entry::Trash(TrashEntry
{ id, run_id, count, dirs })`, the Tauri `undo.ts` `Entry`. A move that
recorded a run pushes it and clears the redo history (`record_trash`); a
redo's new run is pushed without clearing it (`redone_trash`), as a redone
batch is. `step` and `Undo::forget` see only batches: the view's `step`
looks at the top first (`top_trash`) and hands a run to `step_trash`, as the
Tauri `step` does, so `history::step` returns `None` with a run on top and
leaves it there.

- Rule: an undo or a redo of a run goes through `idle::IdleGate` as the run
  does, held under `Undo Move to Trash` / `Redo Move to Trash` while a
  listing or scan is under way and drained at `finished`, but it never
  cancels the scan (`cancels_scan` is false for those labels). The gate's
  slot is the `Held` enum (`Run`, `Undo`, `Redo`) in `main.rs`.
- Rule: the entry stays on its history until the backend answers
  (`trash_step` names the one out, and every undo or redo key does nothing
  meanwhile); a drained step runs only while its entry is still on top
  (`Undo::on_top`), and entries are removed by id (`remove_trash`), not
  popped, since judgments may have been pushed above it meanwhile. A
  refusal other than `RUN_GONE` / `UNDONE_GONE` (`trash::still_held`)
  leaves it where it was.
- Rule: the restore (`trash::undo_run`, which lists the Recycle Bin on
  Windows) runs on `background_spawn` with no progress; the redo
  (`trash::redo_run`) drains the judgment queue and flushes the writer first,
  as a run does, and reports through `Event::TrashProgress` and `trashed`.
  Both hold `Resync` and the gate's in-flight count, so nothing lists under
  them, and a tree open meanwhile waits in `deferred_open`.
- Rule: when the open folder is one of the run's `dirs` (`opens_target`),
  the restored files directly in it (`restored_into`, compared through
  `context::shown_path` as the backend spells them canonically) join `all` at
  once and `refilter` runs, so a second undo already reaches their
  judgments, and the folder is re-listed with the current file and the
  selection kept. Otherwise only a change seen meanwhile re-lists.
- Rule: another folder's open, and a sidecar format switch, drop only the
  judgment batches (`Undo::drop_judgments`); the runs stay, so a run made in
  one folder is undone from another, its files going back to theirs. A
  delete records no run and is not undoable.
- Not verified headless: the restore from the Recycle Bin and the redo; the
  hand checks are pending in `../plans/20261113-gpui-tree-trash/learnings.md`.

## Timings

### `keypressToPixels` differs from the Tauri app's (Inferred)

The `page` line uses the Tauri line's `wait` / `read` / `decode` meanings,
collapses `ipc + post + worker` into `hop` (the time from the background result
to the update on the main thread), and splits `draw` into
`frameWait + paint + present`. `keypressToPixels` includes `present`; Tauri's
stops at `drawImage`. It starts at the latest press of the file that ended up
shown.

### The meta pane is `div` rows, and its cost lands in `present` (Measured)

The right-hand pane (220 px, the Tauri `#info` width) lays out the file
name, `n / total`, the `meta.rs` groups and the notes as `div`s, built from
the `Metadata` that `load` formats on the background thread next to the
preview, so paging reads nothing new. Release, `--auto 20` on the 20-file ARW
compare folder, a fresh `--index` per run, before and after interleaved in one
session with other sessions building on the machine (CPU 12-31 % before each
run, so noisy): `paint` p50 0.70 / 0.82 ms before against 0.90 / 0.76 ms after,
`present` p50 0.19 / 0.22 against 0.44 / 0.41 ms, `keypressToPixels` p50
17.97 / 18.65 against 18.02 / 18.52 ms. `paint` times only the preview
canvas's closure; the pane's `div`s paint after it in the same frame, so their
cost shows as about +0.2 ms in `present`. The `div` rows were kept (the plan's
rule: a canvas only when `paint` grows by more than 1 ms).

- Rule: the view-only flag comes from the whole listing (`Listed.files`), not
  the `--size` / `--max` subset, and is set on every open and re-listing
  before the strip is built.
- Rule: a stage that times one canvas's closure does not see the rest of the
  element tree; look at `present` (and `frameWait` for layout) when adding
  elements beside it.

### A same-session A/B gates only work on the page path (Hit)

A same-session A/B needs the machine idle for 30 to 60 minutes, and other
sessions' builds spoil its runs (see the pass 2 section below). The user
decided on 2026-11-13 that it is needed only for work that can move the
pager's latency.

- Rule: a port plan includes the A/B latency gate only when it touches
  rendering (per-frame drawing, overlays, new drawn parts such as the status
  row, the strip's marks, the pane, menus' buttons), AI (pass 3 embeddings,
  genres, the ranking, the face and eye models), the preview (read, decode,
  orient, prefetch, the 1:1 view), the thumbnails (their read, decode and the
  scan that makes them), or background work that runs while paging (scan
  passes, watchers, folder reads on the open). Dialogs, context menus, file
  operations, the settings UI, code moves into the backend and fixes off the
  page path need no A/B. When in doubt, the plan says which side it is on and
  why.

### The strip costs the pager under 1 ms (Measured)

Same-session A/B, the pager with the strip, the index and pass 1 against the
pre-strip pager (`fb7fcdf4`), three runs each, 60 presses, warm index: median
p50 `keypressToPixels` +0.80 ms on ARW and +0.51 ms on RAF. `paint` did not
move; `present`'s p95 rose from about 0.2 to 0.55-0.85 ms on ARW. See
[the plan's results](../plans/_archived/20261110-gpui-strip-index/results.md).

- Rule: compare against a binary built in the same session, alternating; the
  pre-strip pager's ARW p50 was 3.5 ms lower in this session than the pager's
  in the port's Step 3 session, and `frameWait`'s p50 ranges over 6-10 ms across runs of one binary.

### Pass 2 costs paging nothing once it yields and runs small (Measured)

Same-session A/B, paging while pass 2 runs over the folder against paging on a
finished index, three runs each, 60 presses, pass 2 on `faces_threads()` (3 of
24 logical cores) and pausing at its stage boundaries while the preview holds
the scan's pause from its read until its image is painted. The median p50
`keypressToPixels` A - B was -2.10 ms on ARW 1616x1080 and -0.30 ms on RAF
4416x2944 (gate +2.0 / +5.0). At 22 pass 2 threads with only the read paused
it was +9.44 / +88.86 ms: `decode` about doubled and `paint` grew 4-6x.
See [the plan's results](../plans/_archived/20261110-gpui-focus-marks/results.md).

- Rule: check every run's `frameWait` p50 before reading an A/B. One session
  ran at 120-145 ms (presents at about 4 Hz) once the user left the machine
  and the display slept. Hold `SetThreadExecutionState(ES_CONTINUOUS |
  ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED)` from a helper process for the
  session; that changes no power setting.
- Rule: the B runs' `read` and `decode` came out slower than A's in every
  session after the first (ARW `read` about 2.2 against 0.5 ms). A's pass 2
  keeps the cores clocked up and re-reads the paged files through the hot
  list. So A - B can go negative; compare pairs, not one run.
- Rule: an `--auto` quit inside pass 2 cancels it, so the run logs
  `scan faces: ... canceled=true` with the `done` count it reached. A quit
  inside pass 1 logs `scan extract: ... canceled=true` and no `scan faces:`
  line.
- Rule: size `--auto N` below pass 2's length (N presses at `AUTO_INTERVAL`
  ~1 s plus `AUTO_SETTLE`), and use a fresh `--index` each run. A re-used
  index let pass 2 finish on its own (3,066 files took 41 s against
  `--auto 60`), so the run logged `canceled=false` and
  `quit: scan stopped=true in 0ms`, which proves nothing about the stop.
- Rule: to page while pass 2 runs, a 60-press run needs about 13k ARW or 4.5k
  RAF files to stay inside pass 2 (4.6 ms per ARW file, 15-16 ms per RAF file
  at 22 threads). Build the padding with hardlinks (`os.link`): no disk cost,
  and every copy stays warm in the page cache. Name the padding to sort after
  the compare files (`zzNNN_`) so `--size` / `--max 20` page only the 20.
- Rule: to turn `f` on in an `--auto` run without a code change, post
  WM_KEYDOWN / WM_CHAR / WM_KEYUP for `F` to the process's `MainWindowHandle`.
  `AppActivate` + `SendKeys` does not reach the app and may type into whatever
  window has focus, so never use it. Close a run by PID with
  `CloseMainWindow`; never for every `riffle-native` process, because other
  sessions run their own.
- Rule: other sessions' `mise run ci`, `cargo` and `riffle-native` processes
  spoil runs (17 of 25 B attempts and 1 of 11 A attempts were discarded in the
  Step 6 session of
  `docs/plans/_archived/20261110-gpui-focus-marks/learnings.md`). Wait for two
  clean load samples and no build process before each run, and discard a run
  with a load sample over 15 %. This reduces but does not prevent load
  arriving mid-run.
- Use forward slashes in Windows paths in Bash-tool scripts. A doubled
  backslash in a heredoc collapses to one and can escape the next `$`.

### The pane and the judgments cost the pager under 1 ms (Measured)

Same-session A/B, `eb46913b` (the judgments plan's Steps 1-5, plus the 1:1
view, the focus-marks overlay and pass 2 merged meanwhile) against the
pre-plan pager (`de249218`), three runs each, 60 presses, warm index: median
p50 `keypressToPixels` +0.85 ms on ARW and -1.45 ms on RAF, inside the
run-to-run spread (B's ARW runs alone span 18.75-21.96 ms). `paint` did not
move; `present`'s p50 was 0.07-0.10 ms higher in every A run. See
[the plan's results](../plans/_archived/20261110-gpui-judgments/results.md).

- Rule: a warm run must end the binary's last scan pass uncanceled (A's
  `scan faces:` line once pass 2 exists, not only `scan extract:`), so no
  timed run carries pass 2.
- Rule: check that no other session's `cargo` build is running before a
  measurement session; the first ARW session was void because one ran through
  its first half and A1 used up its retries.
- Rule: in a measurement script, write Windows paths with forward slashes.
  `"$I\\$b-$round.sqlite"` made bash turn `\$` into a literal `$`, so every
  run of a round shared one index file (`index$b-1.sqlite`) and the "after"
  runs were warm; check that each cold run's `--index` file name is distinct
  before trusting a table.

### The filter and the sort cost the pager nothing measurable (Measured)

Same-session A/B, `40e3ec57` (the filter-sort plan's Steps 1-4 only) against
`f080850a` (`main` right after that plan's own PR), three runs each, 60
presses, warm index, both on the same `--settings` file (no filter, `name`
order): median p50 `keypressToPixels` -2.63 ms on ARW and +0.46 ms on RAF,
inside the run-to-run spread (A's ARW runs alone span 19.28-22.11 ms).
`paint` and `present` stayed within 0.04 ms: the status row's two buttons,
unlike the pane, add nothing to `present`. See
[the plan's results](../plans/_archived/20261113-gpui-filter-sort/results.md).

- Rule: when B is the commit right after the plan's own PR, A - B is the
  plan's cost alone; prefer that over a pre-plan commit with other plans'
  merges in between, so a delta is not shared out across them.
- Rule: a deliberately quiet machine still spoils runs. 11 of 23 timed
  attempts were discarded for one to three load samples of 16-31 % mid-run with no build
  process running, and one slot passed only on its fourth and last try. Keep
  the 4-try cap per slot; a slot that uses them all voids the folder's session
  rather than relaxing the 15 % gate.
- Rule: a delta the size of the bound can be pure spread. ARW's three pairs
  were all negative (-2.97, -0.40, -2.35 ms) for code that does not run per
  page; RAF's changed sign. Check what the plan runs per page before reading
  a delta as a cost or a gain.

### During pass 1 the preview reads what the scan already read (Measured)

Stepping with the arrow through 520 cold ARW files while pass 1 ran (27.7 s),
the preview `read` was 0.96 ms p50 and 1.22 ms p95, against Tauri's 14-16 ms
p50 at `READ_SLOTS` 2. The hot list puts the current file and the strip's range
first, so the next file's head is usually cached before the press. A build
without the pause read the same at p50 / p95; its one 336 ms read came on a
slower disk and is not attributed.

- Rule: copying test files with `cp` leaves them in the OS cache (the second
  pass 1 over a 520-file copy took 0.41 s). For a cold pass 1 copy them with
  `robocopy /J` (unbuffered) and use each copy once.
- Rule: to measure the pause itself, press onto files the scan has not read
  (a jump, not the next file); `--auto` only steps to the next file.

### Peaking costs a held key nothing measurable (Measured)

Release, `→` held 5 s as injected key-downs every 33 ms, peaking off and on
alternated three times each in one idle session, fit view at 1275x852 / 1278x852:
median p50 `keypressToPixels` -1.75 ms on ARW (18.55 off, 16.80 on, n of
about 105 per run) and +1.11 ms on RAF (52.70 off, 53.81 on), against the plan's
+2.0 / +5.0 ms gate. ARW was faster with peaking on in every pair, through
`read` and `decode`, not investigated (presumably the busy core keeps the clock
up). On RAF a held key shows only 4-6 pages in 5 s whether peaking is on or off:
a preview load (about 40-45 ms) outlasts the key repeat, so the pager keeps
re-reading the newest file, and the overlay has few pages to compete with. See
[the plan's results](../plans/_archived/20261110-gpui-zoom-peaking/results.md).

- Rule: measure a background job's cost with a held key, not `--auto`, whose
  1003 ms interval lets every job finish before the next press; and check how
  many pages the hold actually showed before reading a p50.
