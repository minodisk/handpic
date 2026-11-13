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
