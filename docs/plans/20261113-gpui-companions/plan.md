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

# The MCP companion, `Handpic.log` and the backend's perf and AI modules for the GPUI app

## Purpose

This is a slice of milestone Step 9 of the GPUI port
(`../20261109-gpui-port/plan.md`). It covers three of that step's items and
leaves the rest (the settings modal, pass 3, genres, the ranking, the
recommended mark, the performance overlay) to later slices.

- **The MCP companion.** The loopback Streamable HTTP server, its seven tools
  and the request / reply bridge live in `crates/app/src/mcp.rs`, tied to Tauri
  by `tauri::async_runtime`, an `AppHandle` and the `mcp-request` / `mcp-state`
  events. The answering side is `crates/app/ui/src/companion.ts`. After this
  plan the server, the tool definitions and the bridge are in `crates/backend`
  behind a callback seam, the Tauri app calls them through a thin wrapper, and
  the GPUI app serves the same endpoint, answering from its window state, when
  `mcpEnabled` is on in the `settings.json` it reads read-only.
- **`Handpic.log` in the GPUI app.** `crates/native/src/logger.rs` writes to
  stderr only, which a bundled app does not capture. After this plan the
  native app writes a `Handpic.log` in the Tauri app's line format with the
  same greppable prefixes (`panic:` from the backend's panic hook,
  `invariant:` from `invariant!`, and the plain `log` lines).
- **`perf.rs` and `ai.rs` in the backend.** The CPU / disk sampler and the
  `aiEnabled` state with its model download move to `crates/backend` behind
  callbacks; the Tauri app keeps thin wrappers that own the event names. The
  native app does not use them yet; only the move is in scope.

### What the investigation found (2026-11-13, `main` at `40e3ec57`)

- `crates/app/src/mcp.rs` depends on Tauri only through
  `tauri::async_runtime::{spawn, spawn_blocking, JoinHandle, block_on}`,
  `Bridge::to_main_window(AppHandle)` (which emits `mcp-request`), and
  `state` / `switch`, which take an `AppHandle` and emit `mcp-state`. It also
  uses `commands::read_metadata` / `read_preview`, three-line wrappers over
  `riffle_core::reader` and the backend's `exif::metadata`. `index::lock`,
  `index::flag_name`, `Index::entry` and `Focus` are already the backend's.
- `crates/app/src/perf.rs` touches Tauri only in the `set_perf_sampling`
  command and in `run`'s `app.emit("perf-sample", ...)`.
- `crates/app/src/ai.rs` touches Tauri in `model_path` (the app's local data
  dir), `state`, `switch` and `spawn_download` (`app.state::<AppAi>()` and
  `app.emit("ai-state", ...)`). `AppAi::enabled` / `model_if_enabled` are read
  from `commands.rs` and `rank.rs` through `app.state::<crate::ai::AppAi>()`.
- The native view has no selection, no compare view and no burst grouping
  (`burst.ts` is not ported), and it judges the eyes only while the `f`
  overlay is on. Its judgments go through `judge` / `store` / `history::Undo`,
  whose entries are already batches.
- `scan::Event` in `crates/native/src/scan.rs` is the one channel a non-GPUI
  thread may send into (`../../agents/native-app.md`); it already carries the
  non-scan `Changed` and `SidecarError` variants.
- `crates/native/Cargo.lock` already resolves `tokio`, `sysinfo`, `sha2`,
  `base64`, `chrono` and `time` through GPUI; `rmcp`, `axum` and `reqwest` are
  new to it.
- The native `page` and `zoom` timing lines are `println!` to stdout, which
  the measurement scripts read; they are not `log` lines.
- `release-please-config.json` already bumps `crates/backend/Cargo.toml`.

### No A/B latency measurement

None of the three items runs anything periodically during paging, so no A/B
measurement is planned. With `mcpEnabled` off the native app creates no
runtime and no thread. With it on, one thread is parked in `accept` until a
client connects; a tool call costs one message through the view's channel.
The file logger adds one unbuffered, unsynced line write where an `eprintln!`
already happens, and no `log` line is written per page at `Info`. If a step
finds it must add a timer, a poll or per-page work, stop and record it here
before going on.

### Rules for every step

- `crates/native` must not depend on `crates/app`. Never add `tauri`, an event
  name or an `AppHandle` to `crates/backend`; take a callback or the state.
- Check every step with root `mise run ci` and with `mise run native:check`
  (in bash, prefix `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH"`). A
  dependency added to the backend also lands in `crates/native/Cargo.lock`;
  commit both lock files.
- `crates/native/src/main.rs` and the `crates/native` paragraph of `CLAUDE.md`
  are edited by two other sessions at the same time. Keep changes there to a
  module declaration, a field, a call and a `match` arm; put the logic in new
  modules, and bring in `main` with a merge commit (never a force push) before
  opening each PR.
- The native app never writes `settings.json`.
- Do not touch `.github/**`, the release-please files or
  `crates/app/tauri.conf.json`.
- Hand checks run only on copies under `D:\Photos\tests\<date>-<topic>\`, with
  `--index`, `--settings` and (from Step 1) `--log` given explicitly and
  pointing into that folder. Never open another `D:\Photos` folder, never use
  the Tauri app's or the default native index, change nothing outside
  `D:\Photos\tests\`, and never kill a process (rename a locked exe aside).
  Checks that need a person at the window are recorded in `learnings.md` as
  pending manual checks, not run.

## Steps

- [x] Step 1: Write `Handpic.log` from the GPUI app
  - Done when: the native app writes every `log` line it prints to stderr also
    to a `Handpic.log`, as `[YYYY-MM-DD][HH:MM:SS][target][LEVEL] message` in
    UTC; the backend's panic hook is installed, so a panic on any thread
    leaves a `panic:` line; an `invariant!` failure leaves an `invariant:`
    line; the file starts anew once the next write would pass 1 MB, as the
    Tauri app's does; `--log <path>` names the file for a test run; unit tests
    cover the line format, the size cap and the default path; a run on a test
    copy with `--log` under `D:\Photos\tests\` shows the `index:`,
    `settings:` and `scan extract` lines in the file; `docs/agents/app-log.md`
    describes the native file (path, targets `riffle_native` /
    `riffle_backend`, levels, no `uncaught-js:`, timing lines on stdout),
    `docs/agents/native-app.md`'s logger section and its "every debug run
    passes" rule name `--log`, and `CLAUDE.md`'s `logger.rs` sentence is
    updated.
  - Implementation approach (as far as it is known; omit if unknown):
    - Keep the work in `crates/native/src/logger.rs` (the file sink next to
      the stderr one, same `enabled` filter, so `RUST_LOG=debug` still adds
      only this crate's lines) and `args.rs` (`--log`, the `USAGE` string).
      `main.rs` changes only where it calls `logger::install`, which needs the
      parsed `--log`, and gains the one
      `riffle_backend::diagnostics::install_panic_hook()` call right after it
      (or `logger::install` makes that call itself).
    - Default path: mirror Tauri's `app_log_dir` shape, `~/Library/Logs/<id>/`
      on macOS and `<dirs::data_local_dir()>/<id>/logs/` elsewhere, with
      `<id>` being `com.minodisk.handpic.native` from `datadir.rs`'s
      `IDENTIFIER` (see Trade-offs). A file that cannot be opened is reported
      once on stderr and the app runs with stderr only.
    - Write one line per record with a single `write` on a `Mutex<File>`
      opened for append; do not `sync_all` per line. Track the size in memory
      instead of a `stat` per line.
    - Format the timestamp with a crate already in `crates/native/Cargo.lock`
      (`chrono` or `time`) or by hand; do not add a new crate to the lock for
      it.
    - A `panic:` record with a backtrace spans several lines, as in the Tauri
      app; do not collapse it.
    - Add the Tauri app's `scan progress within total` check to the native
      scan's progress callbacks in `scan.rs` through
      `riffle_backend::diagnostics::invariant!`, and add its native location
      to the table in `app-log.md`.
    - Leave the `println!` timing lines (`page`, `zoom`, the summary) on
      stdout.

- [x] Step 2: Move the CPU and disk sampler into `crates/backend`
  - Done when: `crates/backend/src/perf.rs` holds the sampler (the `Sample`
    payload, the per-platform disk readers, `Readings`, the thread's loop and
    the start / stop state) and reports each sample through a callback;
    `crates/app/src/perf.rs` is a thin wrapper with the `set_perf_sampling`
    command that emits `perf-sample`; the frontend is unchanged; the module's
    tests moved with the code; `CLAUDE.md`'s `crates/backend` and
    `crates/app` descriptions are updated.
  - Implementation approach (as far as it is known; omit if unknown):
    - The seam follows `watch::Watch::spawn(on_changed)`: the backend's start
      takes `impl Fn(Sample) + Send + 'static` (or one returning a `Result`
      so the wrapper's `perf-sample emit failed` warn-once keeps working;
      decide in the step).
    - `sysinfo` and the Windows `windows` crate with
      `Win32_System_Performance` move to `crates/backend/Cargo.toml`.
      `crates/app` keeps `sysinfo` (its `commands.rs` uses it); drop from
      `crates/app/Cargo.toml` only what this move leaves unused.
    - `crates/native` gains nothing but the lock entries.

- [ ] Step 3: Move the `aiEnabled` state and the model download into `crates/backend`
  - Done when: `crates/backend/src/ai.rs` holds the enabled / download state,
    `AiState`, the switch that starts or cancels the download, the part-file
    cleanup, `fetch` and `install` with the size and SHA-256 check, and
    reports state changes through a callback; `crates/app/src/ai.rs` keeps
    `model_path` (the app's local data dir), the `AppAi` the other modules
    read (`enabled`, `model_if_enabled`) and the `ai-state` emit;
    `commands.rs`, `rank.rs` and `main.rs` behave as before; the module's
    tests moved with the code; `CLAUDE.md` is updated.
  - Implementation approach (as far as it is known; omit if unknown):
    - The download thread reaches the state through `app.state::<AppAi>()`
      today. In the backend it needs the state itself, for example an
      `Arc`-held struct that owns the callback, so the thread can clear its
      own entry and report the final state.
    - The persisted `aiEnabled` key and its store write stay in
      `crates/app/src/commands.rs`.
    - `reqwest`, `rustls` and `sha2` move to `crates/backend/Cargo.toml` with
      the same features (`blocking`, `rustls-no-provider`, `ring`); remove
      them from `crates/app/Cargo.toml` if nothing else there uses them.
    - Verify that `mise run native:check` builds `ring` on Windows without
      extra setup, and record the result in `learnings.md`.
    - `index::with_suffix` and `index::lock` are already the backend's.

- [ ] Step 4: Move the MCP server, its tools and the bridge into `crates/backend`
  - Done when: `crates/backend/src/mcp.rs` holds `MCP_PORT`, the instructions,
    `Bridge` with a public constructor over a send callback, `Request` with
    fields a non-serializing caller can read, the `Companion` handler with its
    seven tools and argument types, `resolve`, the `get_photo` / `get_preview`
    reads, the server state with start / stop / switch and `McpState`, and the
    stored-`mcpEnabled` rule; `crates/app/src/mcp.rs` keeps only the bridge to
    the main window (`mcp-request`), the `mcp-state` emit and the functions
    `commands.rs` and `main.rs` call; the tool list, descriptions and replies
    are unchanged for a client; the module's tests moved and pass without
    Tauri; `CLAUDE.md` and `docs/agents/native-app.md` (the backend section)
    are updated.
  - Implementation approach (as far as it is known; omit if unknown):
    - Replace `tauri::async_runtime::spawn` / `spawn_blocking` / `JoinHandle`
      with tokio's. Decide between bare `tokio::spawn`, which needs every
      caller inside a runtime context, and passing a `tokio::runtime::Handle`
      into the server. Verify which holds for the Tauri callers: `switch` is
      awaited from async commands and from the task `setup` spawns, `stop`
      under `tauri::async_runtime::block_on` at exit.
    - The switch returns the new `McpState` and emits nothing; the Tauri
      wrapper emits `mcp-state`.
    - The moved tests need a tokio runtime in place of
      `tauri::async_runtime::block_on`; add the tokio features and `tower` as
      backend dev-dependencies as needed.
    - `rmcp`, `axum`, `tokio`, `tokio-util` and `base64` move to
      `crates/backend/Cargo.toml`; remove from `crates/app/Cargo.toml` what
      the move leaves unused.
    - The metadata and preview reads: move the helpers the app's own commands
      also use rather than copying them, per the native guide's rule.
    - Move `mcp_enabled_setting` (missing or non-boolean means off) from
      `commands.rs` so Step 5 reads the key by the same rule.
    - `env!("CARGO_PKG_VERSION")` becomes the backend crate's version, which
      release-please bumps with the app's; `get_info` keeps the name
      `handpic`.

- [ ] Step 5: Serve the MCP endpoint from the GPUI app: the read and navigate tools
  - Assumes Step 4 is merged (and Step 1, for the log lines the check reads).
  - Done when: with `"mcpEnabled": true` in the settings file the native app
    listens on `127.0.0.1:41917` (or `--mcp-port`), and with it off or
    missing it starts no server, no runtime and no thread; `get_view`,
    `get_photo`, `get_preview`, `show_photo`, `set_view` (`normal` / `zoom`)
    and `select_photos` with one path answer from the window state, a refused
    request carrying its message; `set_judgment` is refused with a message
    until Step 6; a bind failure is logged and the app runs on; the server is
    stopped on quit within its stop timeout; unit tests cover the answers over
    a view snapshot and the `mcpEnabled` read; an automated check on a test
    copy drives `initialize`, `tools/list` and `tools/call` for `get_view`,
    `show_photo` and `get_preview` with `curl` against the running exe and
    records the replies in `learnings.md`; `CLAUDE.md` names the new modules.
  - Implementation approach (as far as it is known; omit if unknown):
    - New `crates/native/src/mcp.rs`: owns the tokio runtime (created only
      when the setting is on; one named thread is enough), builds the
      backend's `Bridge` whose send callback only does
      `events.unbounded_send(...)` with a new `scan::Event` variant, and
      holds the server so the view can reply. A callback from a non-GPUI
      thread must not touch the view.
    - New `crates/native/src/companion.rs`: the pure port of `companion.ts`
      (`get_view`, the visible-path check, `show_photo`, `select_photos`,
      `set_view`, the argument validation and their messages) over a snapshot
      of the view (folder, `files`, the current index, the zoom state, the
      sort key's setting name, `FilterState::is_active`, view-only, the
      current file's eyes judgment, the judgments), returning either a reply
      or what the view has to do. Port the cases of `companion.ts`'s tests
      that apply.
    - `main.rs` gains the module declarations, one field, the call that
      starts the server in `Riffle::new` (it needs the events sender and the
      index reader), one arm in `scanned`, and one method that builds the
      snapshot, applies the effect through the existing `go(Move::To(..))` /
      `toggle_zoom`, and replies. Nothing else.
    - `settings.rs` gains `mcp_enabled`, read with the backend's rule from
      Step 4; `args.rs` gains `--mcp-port`.
    - What the native view cannot answer yet is fixed by this plan's
      decisions (see Trade-offs): `selected` is the current file alone,
      `burst` is empty, `mode` is `normal` or `zoom`, `compare` and a
      `select_photos` with several paths are refused with a message that says
      the native app does not support them yet, and `current.eyes` is present
      only when the `f` overlay has judged the file.
    - Paths: report `files` as the index keys them (`\\?\`-prefixed on
      Windows); the backend's `resolve` canonicalizes the parent, so either
      form of a client's path resolves.
    - `get_photo` and `get_preview` run in the backend on tokio's blocking
      pool against the index's read-only connection; they do not go through
      `ReadSlots`, as in the Tauri app.
    - The check: a `settings.json` with `mcpEnabled` true, `--index`, `--log`
      and a free `--mcp-port`, all under the test folder; `--auto N` keeps
      the window up long enough for the `curl` calls and quits on its own.
      The session id comes back in the `Mcp-Session-Id` header of
      `initialize`.
    - Pending manual checks for `learnings.md`: an MCP client connected to
      the native app, and `show_photo` / `set_view` visibly moving the view.

- [ ] Step 6: `set_judgment` in the GPUI app, and the documentation
  - Assumes Step 5 is merged.
  - Done when: `set_judgment` records stars, the flag and the label on the
    given paths, or on the current file when none are given, through the same
    view, row and writer path as a judgment key: shown at once, one undo
    entry for the whole call, each photo's sidecar written, no auto-advance,
    refused in a view-only folder with the Tauri app's message; the reply
    lists each photo's resulting stars, flag and label; unit tests cover the
    merge of the given fields over each photo's own state and the refusals;
    an automated check on a test copy calls `set_judgment` with `curl` and
    finds the rating in the file's sidecar and the row; `docs/agents/native-app.md`
    has a section on the companion (the runtime, the channel hop, what is not
    supported yet, the `--mcp-port` and `--log` rules for test runs);
    `CLAUDE.md` is current; Step 9 of `../20261109-gpui-port/plan.md` carries
    a line naming this plan for the companion, `Handpic.log` and the perf /
    AI move, with a Progress entry, the step staying open.
  - Implementation approach (as far as it is known; omit if unknown):
    - Omitted fields keep each photo's own value, a rating of 0 clears the
      stars, a null label clears the label, as `companion.ts`'s
      `setJudgment`.
    - Record one `history::Undo` entry holding every changed file
      (`undo.record` already takes a batch), mark the paths `touched`, apply
      with `set_state`, `refilter` once with the current file as anchor, then
      `store` each file with that entry's id so a failed store drops its file
      from the entry.
    - Verify against the Tauri `judge` (`selection.ts`, `main.ts`) which
      files are written when a field does not change, and the `label_known`
      rule: `judgment.rs`'s `writes` / `label_known` take a key `Command`, so
      either they gain a case for this or the companion applies the undo
      path's rule (`seen`, or the label differs).
    - Verify what `judgment::State.label` holds (the color's name or the
      configured `xmp:Label` name) and map the tool's English color names
      accordingly, as the Tauri app does.
    - Pending manual checks for `learnings.md`: a judgment made through a
      client showing in the strip and the pane at once, `Ctrl+Z` undoing a
      multi-photo call as one entry, and the log file after closing the
      window by hand.

## Trade-offs and risks

- **What the native `get_view` reports while the view lacks bursts, selection
  and compare.** Taken: `burst` is always empty, `selected` is the current
  file, compare and a multi-path `select_photos` are refused with a message.
  The tool descriptions are shared with the Tauri app and say an empty
  `burst` means the photo is in none, so a client is told less than the truth
  until the burst model is ported. Not taken: a further step porting
  `burst.ts`'s grouping as a pure module with a sharpness map; it needs the
  sharpness the native `Facts` (`filter.rs`, out of this plan's scope) does
  not keep, so it touches the entries read in `main.rs`.
- **Where the native `Handpic.log` lives.** Taken: the native app's own
  folder (`com.minodisk.handpic.native`), as its index, so the two apps never
  write one file and the Tauri plugin's delete at 1 MB never removes native
  lines. Not taken: the Tauri app's log folder, which gives one file to ask a
  user for but two processes appending to it. At the switch-over (port Step
  10) the folder becomes the plain identifier's either way.
- **Two new debug flags, `--log` and `--mcp-port`.** Without `--log` every
  hand-check and measurement run writes a log outside `D:\Photos\tests\`,
  which the hand-check rules forbid; without `--mcp-port` the automated check
  cannot bind while the user's Tauri app holds 41917 with MCP on. Not taken:
  no flags (checks limited to unit tests, and the log always in the default
  folder). The flags add to `args.rs` and its `USAGE` line.
- **`current.eyes` in the native app.** The native app judges the eyes only
  while the `f` overlay is on, so `eyes` stays absent with it off, where the
  tool description says to call again shortly. Not taken: asking for a
  judgment on `get_view`, which adds an on-demand detection and model run on
  the background executor the preview decode uses and would call for an A/B
  measurement.
- **No cargo features on the backend.** `rmcp`, `axum`, `reqwest`, `rustls`
  and the PDH bindings compile into the native build although it uses only
  the MCP ones in this plan; the rest of port Step 9 uses the others. Not
  taken: `mcp` / `ai` / `perf` features, which would keep the native build
  smaller for now at the cost of feature matrices in two workspaces.
- **Timing lines stay on stdout.** The native `page` and `zoom` lines are not
  written to `Handpic.log`; the measurement scripts read stdout, and the
  Tauri app writes its frontend timing lines only behind a development
  setting the native app does not have yet.
- **The same port in both apps.** The native app listens on 41917 like the
  Tauri app, so a client's configuration survives the switch-over; while both
  run with MCP on, the second fails to bind and logs it. The native app has
  no settings window to show that error in.
- **Concurrent sessions.** `main.rs` and `CLAUDE.md`'s native paragraph will
  conflict with the filter / sort and folder-tree sessions; the steps keep
  their edits there to a few lines. If a selection lands in the native view
  before Step 5 or 6, use it for `selected`, `select_photos` and the default
  judgment targets instead of the current-file fallback, and record the
  change here.
- **The tokio context in the Tauri app.** If a caller of the moved `switch`
  turns out not to run inside Tauri's tokio runtime, a bare `tokio::spawn`
  panics at run time, not at compile time; Step 4 settles this before
  merging, and the app's MCP toggle is a pending manual check for it.
- **A panic hook installed before the window opens.** The native app installs
  the hook right after the logger, earlier than the Tauri app does, so a
  startup panic is logged too. mozjpeg's `resume_unwind` still skips the
  hook, as documented in `../../agents/app-log.md`.

## Progress

- (2026-11-13) Plan approved by the user with the defaults in Trade-offs.
- (2026-11-13) Step 1 complete
- (2026-11-13) Step 2 complete
