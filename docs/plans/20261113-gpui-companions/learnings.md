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
