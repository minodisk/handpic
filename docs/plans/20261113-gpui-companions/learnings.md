# Learnings

## Step 1: `Handpic.log` from the GPUI app

- The timestamp is formatted by hand (Howard Hinnant's `civil_from_days`)
  rather than through `chrono` / `time`: both are in `crates/native/Cargo.lock`
  through GPUI, but turning on the features a direct dependency needs could
  pull further crates into the lock.
- The size cap starts the file anew with `File::create` (truncate) and
  replaces the handle, rather than deleting the open file: on Windows a file
  opened for append cannot be truncated by `set_len`, and a remove while the
  handle is open can leave the name pending deletion.
- The Tauri `start_scan` already had the `scan progress within total` check;
  the native scan now runs it in both progress callbacks under target
  `riffle_native::scan`, with `scan_id` being the native scan's `id`.
- Hand check (release build, `--auto 4` on 5 ARW copies in
  `D:\Photos\tests\20261113-native-log\photos`, with `--index`, `--settings`
  and `--log` in that folder): the file held the `index:`, `settings:`,
  `scan list`, `scan reconcile`, `scan sidecars`, `scan extract`,
  `scan faces` and `quit:` lines in the
  `[YYYY-MM-DD][HH:MM:SS][target][LEVEL] message` shape with UTC times, plus
  GPUI's own `gpui_windows::*` `INFO` lines; the `page` lines and the
  summary stayed on stdout only.
- No `panic:` line was provoked in a real run; the hook is the backend's
  (tested there) and is installed in `main` right after the logger.
- `Riffle::start` destructures `Args` exhaustively, so a new flag needs a
  `log: _` there too.
- Python's text-mode write on Windows turned `args.rs` into CRLF; write with
  `newline=''` when editing files through a script.

## Step 2: the CPU and disk sampler in `crates/backend`

- The seam is `perf::Sampler::start(on_sample)` with
  `on_sample: impl FnMut(Sample) -> Result<(), E: Display>` and a separate
  `stop()`, rather than one `set(enabled, ...)`: stopping needs no callback.
  The callback returns a `Result` so the warn-once on a failed report stays
  in the backend; the Tauri wrapper is just
  `move |sample| app.emit("perf-sample", sample)`. The backend's warn line now
  reads `perf sample report failed: ...` under target
  `riffle_backend::perf`, since the backend names no event.
- `windows` (`Win32_System_Performance`) left `crates/app/Cargo.toml`
  entirely (only `perf.rs` used it); `sysinfo` stays there for
  `commands.rs`.
- `crates/native/Cargo.lock` gains `sysinfo 0.39.6` next to GPUI's
  `sysinfo 0.31.4` (two versions, so the plan's "already resolves `sysinfo`"
  did not save a crate), plus `objc2-io-kit` for its macOS build; `windows
  0.62.2` was already in the lock.

## Step 3: the `aiEnabled` state and the model download in `crates/backend`

- The seam is `ai::Ai`, a cloneable `Arc` over the status, the model path and
  an `on_state: Fn(AiState) + Send + Sync` callback given to `Ai::new`, so the
  download thread holds a clone of the `Ai` itself (no `app.state::<AppAi>()`)
  to clear its own entry and report progress and the final state. `switch`
  reports through the same callback and also returns the state. Unlike the
  perf sampler, the callback returns nothing: the Tauri app ignored every
  `ai-state` emit error before (`let _ = app.emit(...)`), and the wrapper
  keeps doing so.
- `AppAi::new` now takes the `AppHandle` (it builds the emitting callback and
  calls `model_path` itself), so `main.rs`'s call changed shape; `enabled` /
  `model_if_enabled` delegate. `AiState` is re-exported from the app's `ai`
  so `commands.rs` keeps naming `crate::ai::AiState`.
- The download's `log` lines (`embedding model downloaded to ...`,
  `embedding model download failed: ...`) now come under target
  `riffle_backend::ai` instead of `riffle_app::ai`.
- `reqwest`, `rustls` and `sha2` left `crates/app/Cargo.toml` entirely; nothing
  else there used them.
- `ring` on Windows: `mise run native:check` (with NASM on `PATH` as the
  plan's rule says) built `ring` and the rest of the new TLS stack without any
  extra setup and passed. `crates/native/Cargo.lock` gained 35 packages,
  among them `reqwest`, `rustls`, `ring`, `hyper`, `hyper-rustls`,
  `rustls-platform-verifier` (with `jni` / `security-framework` for other
  platforms), `tower` / `tower-http`, `webpki-root-certs` and `base64`.

## Step 4: the MCP server, its tools and the bridge in `crates/backend`

- The tokio context: bare `tokio::spawn` / `tokio::task::spawn_blocking`,
  not a `Handle` passed in. Verified against Tauri 2.11.6's
  `async_runtime.rs`: its `spawn` enters the runtime before spawning, its
  `block_on` is tokio's `Runtime::block_on` / `Handle::block_on`, which both
  enter the context. So every Tauri caller of the moved code runs inside
  Tauri's runtime: `set_mcp_enabled` / `mcp_enabled` are async commands
  (run on `tauri::async_runtime`), `setup`'s switch runs in a
  `tauri::async_runtime::spawn` task, and the exit's `stop` runs under
  `tauri::async_runtime::block_on`. The rmcp handlers that call
  `spawn_blocking` run inside the server's own spawned task. Step 5's native
  app has to call `Mcp::switch` / `stop` inside the runtime it owns (for
  example `runtime.block_on` or a task spawned on it).
- The seam: the backend's `mcp::Mcp` (was `AppMcp`) with `new(bridge,
  index)`, `reply`, and async `state` / `switch` / `stop` methods; `switch`
  returns the `McpState` and emits nothing. `Bridge::new(send)` is public and
  `Request`'s `id` / `kind` / `args` are public fields (still `Serialize`
  for the Tauri emit). The Tauri `mcp.rs` keeps `AppMcp(Mcp)`, whose `new`
  now takes the `AppHandle` and builds the `mcp-request` bridge (with the
  "main window is not open" check), plus `state` / `switch` (with the
  `mcp-state` emit) / `stop`; `main.rs`'s `AppMcp::new` call changed shape.
- `read_metadata` moved to `riffle_backend::exif` and `read_preview` to
  `riffle_backend::ondemand`; `commands.rs` imports them, so its commands and
  tests are unchanged. `mcp_enabled_setting` moved to `riffle_backend::mcp`
  and gained a test there (it had none).
- `rmcp`, `axum`, `tokio`, `tokio-util`, `base64` and the `tower` dev
  dependency left `crates/app/Cargo.toml` entirely. The backend's `tokio`
  adds `rt` (for `tokio::spawn` without relying on another crate's feature
  unification, which the native workspace does not share with Tauri); the
  tests use a current-thread runtime and `io-util` as a dev feature.
- A new backend test, `a_client_over_loopback_initializes_and_lists_the_tools`,
  binds a real loopback port and drives `initialize` (reading the
  `Mcp-Session-Id` header), `notifications/initialized` (202) and
  `tools/list` over raw HTTP/1.1, checking the server name `handpic` and the
  seven tool names. The SSE response of a POST closes after its one message,
  so reading to the end with `Connection: close` works.
- `get_info`'s version is now `riffle-backend`'s `CARGO_PKG_VERSION`, which
  release-please bumps with the app's (both 1.10.0 now).
- The `log` lines `failed to start the MCP server` and `the MCP server
  stopped` now come under target `riffle_backend::mcp`.
- `crates/native/Cargo.lock` gained 23 packages, among them `rmcp` /
  `rmcp-macros`, `axum` / `axum-core`, `tokio-util`, `tokio-stream`,
  `tokio-macros`, `sse-stream`, `matchit` and a second `sha2` (0.11, rmcp's,
  next to the backend's 0.10); `mise run native:check` passed without other
  setup.

## Step 5: the MCP endpoint in the GPUI app (read and navigate tools)

- The runtime: a tokio **current-thread** runtime driven by one named `mcp`
  thread, rather than a multi-thread runtime with one worker. The thread's
  `block_on` runs `Mcp::switch(true)`, logs the returned `McpState`, waits on
  a oneshot the quit handler fires, then runs `Mcp::stop` and signals back.
  So `switch` and `stop` run inside the runtime (Step 4's rule), the native
  crate needs only tokio's `rt` and `sync` features as a direct dependency
  (the backend's `net` / `time` unify in), and the GPUI main thread never
  blocks on the bind. The quit handler (`App::on_app_quit`, registered only
  when the server starts) waits at most `STOP_WAIT` (2.5 s, the backend's
  2 s stop timeout plus a margin) and logs `quit: MCP server stopped=<bool>
  in <ms>ms`.
- The backend gained `Mcp::with_port`: `switch` always bound `MCP_PORT`, so
  `--mcp-port` had no way in. The Tauri app is unchanged (it never calls it);
  a new backend test, `a_switch_binds_the_given_port_and_reports_it`, covers
  it.
- `main.rs`: two `mod` lines, the `mcp` field (filled by `mcp::start` in the
  struct literal, before `events` / `reader` / `settings` move), one extra
  `Riffle::new` parameter for the port (`main` copies `args.mcp_port` before
  the `move` closure), `mcp_port: _` in `start`'s exhaustive destructure,
  the `Event::Mcp` arm in `scanned` (plus `Event::Mcp(_)` on the second
  match's no-op arm) and the `mcp_request` method. The snapshot is built by a
  nested `fn view(this: &Riffle) -> companion::View<'_>`: a closure
  returning a borrow of its argument does not get the right lifetime
  ("lifetime may not live long enough").
- `companion::answer` returns what the view has to do (`Show(index)` through
  `go(Move::To(..))`, `ToggleZoom` through `toggle_zoom`) and the view then
  replies with `get_view` over a fresh snapshot, as `companion.ts` calls the
  action then `getView`. `set_view` toggles only when the mode differs.
- `overlay::Marks` gained `eyes(path)`, the `EyesCache` entry, for
  `current.eyes`.
- The native view still has no selection (checked on `main` at `e341ef21`),
  so the plan's current-file fallback stands.
- curl on Git Bash: a JSON body passed as a `--data` argument with `\`
  escapes reached the server mangled (`invalid escape`), apparently MSYS's
  argument conversion; feeding the body through stdin
  (`--data-binary @- <<< "$body"`) works.
- Automated check (debug build, `--auto 25` on 3 ARW copies in
  `D:\Photos\tests\20261114-native-mcp\photos`, `--index`, `--settings`
  (`settings.sqlite`), `--import-settings` (`settings.json` holding
  `{"mcpEnabled": true}`) and `--log` all in that folder, `--mcp-port 41931`),
  replies trimmed, paths with their JSON escapes undone (the `--auto` pager
  moves the current file between calls):
  - `initialize`: 200, SSE `serverInfo {"name":"handpic","version":"1.10.0"}`
    with the instructions; the `Mcp-Session-Id` header came back.
  - `notifications/initialized`: `HTTP/1.1 202 Accepted`.
  - `tools/list`: `get_photo get_preview get_view select_photos set_judgment
    set_view show_photo`.
  - `get_view`: `{"folder":"\\?\D:\Photos\tests\20261114-native-mcp\photos","count":3,"current":{"path":"\\?\D:\...\_DSC0745.ARW","position":1},"selected":["\\?\D:\...\_DSC0745.ARW"],"mode":"normal","compare_active":null,"burst":[],"sort":"name","filtered":false}`,
    `isError: false`.
  - `show_photo` `_DSC0787.ARW`: the same shape with `current.position: 3`
    and `selected` that file; a path outside the folder: `isError: true`,
    `D:\Photos\tests\nope.ARW is not shown in Handpic: not in the open
    folder, or hidden by the filter`.
  - `set_view` `zoom` then `normal`: `"mode":"zoom"`, then `"mode":"normal"`;
    `compare`: `compare is not supported by Handpic's native app yet`.
  - `select_photos` with two paths: `selecting more than one photo is not
    supported by Handpic's native app yet`; `set_judgment` `{rating: 3}`:
    `set_judgment is not supported by Handpic's native app yet`.
  - `get_photo` (no path): the current file's row and shooting settings
    (`rating: null`, `flag: "none"`, `sharpness: 371.2`, the focus with
    `eye_focus 0.9986`, `orientation 6`, `camera "SONY ILCE-7M5"`, ...).
  - `get_preview` `{long_edge: 256}`: an `image/jpeg` block (base64 starting
    `/9j/4AAQ`) and `{"path":"\\?\D:\...\_DSC0787.ARW","width":135,"height":202}`.
  - The log held `mcp: McpState { enabled: true, port: 41931, error: None }`
    at startup and `quit: MCP server stopped=true in 0ms` after the other
    `quit:` lines; the process exited 0 on its own.
  - Off: the same run with a store seeded from `{}` logged no `mcp` /
    `rmcp` line at all. Bind failure: with 41933 held by another listener
    the backend logged `failed to start the MCP server: could not listen on
    127.0.0.1:41933: ... (os error 10048)`, the state line carried the error,
    and the app scanned, paged and quit normally (exit 0).

## Deferred issues (todo candidates)

- Pending manual check (Step 4, the Tauri app's MCP toggle on the moved
  server): on Windows, in a Tauri app built from this branch, turn the MCP
  server on in the settings modal, connect an MCP client to
  `http://127.0.0.1:41917/mcp`, call `get_view` and `show_photo`, then turn
  the server off and on again and quit the app. Expected: the settings show
  the server on port 41917 with no error, the client lists the seven tools
  and the calls answer as before, the toggle frees and rebinds the port, and
  the app quits without hanging (the 2 s stop timeout). Basis: Step 4 moved
  the server onto bare `tokio::spawn` in `crates/backend/src/mcp.rs`, whose
  Tauri callers are `crates/app/src/mcp.rs`, `crates/app/src/commands.rs`
  (`set_mcp_enabled`, `mcp_enabled`, `mcp_reply`) and `crates/app/src/main.rs`
  (setup's switch, the exit's `stop`); the step's checkbox was ticked on the
  automated criteria (the backend's loopback test and the moved tests).
- Pending manual check (Step 5, an MCP client on the native app): on
  Windows, run the GPUI app from this branch with `"mcpEnabled": true` in
  the settings it is seeded from (on a test copy under `D:\Photos\tests\`,
  with `--index`, `--settings`, `--import-settings`, `--log` there and a free
  `--mcp-port`), connect a real MCP client (for example Claude Code with an
  `http` server at `http://127.0.0.1:<port>/mcp`), and ask it to show another
  photo, zoom and unzoom. Expected: the client lists the seven tools,
  `show_photo` visibly moves the window's preview and strip to that photo,
  `set_view` `zoom` / `normal` visibly enters and leaves the 1:1 view, and
  closing the window by hand leaves `quit: MCP server stopped=true` in the
  log. Basis: Step 5's `crates/native/src/mcp.rs`, `companion.rs` and
  `main.rs`'s `mcp_request`; the step's checkbox was ticked on the automated
  criteria (unit tests and the curl check above).
