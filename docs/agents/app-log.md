# Reading `Handpic.log`

Read this before diagnosing a report about the running app. `Handpic.log` is
the only trace a session leaves: stderr is not captured in a bundled app, so
anything that matters is written here, and the failures an agent most needs
carry a fixed, greppable prefix (`panic:`, `uncaught-js:`, `invariant:`).

The log is set up by `tauri_plugin_log` in `crates/app/src/main.rs`; the
prefixed lines come from `crates/backend/src/diagnostics.rs` (the panic hook
and the `invariant!` macro), `crates/app/src/diagnostics.rs` (`log_frontend`)
and `crates/app/ui/src/uncaught.ts`.

The GPUI port (`crates/native`) writes a `Handpic.log` of its own; see
[The GPUI app's log](#the-gpui-apps-log) at the end.

## Where the file is

| OS      | Path                                                     |
| ------- | -------------------------------------------------------- |
| Windows | `%LOCALAPPDATA%\dev.minodisk.handpic\logs\Handpic.log`     |
| macOS   | `~/Library/Logs/dev.minodisk.handpic/Handpic.log`          |
| Linux   | `~/.local/share/dev.minodisk.handpic/logs/Handpic.log`     |

- The Windows path was checked against the running app. The macOS and Linux
  paths are taken from `app_log_dir` in tauri 2.11.6's
  `src/path/desktop.rs` (macOS: `~/Library/Logs/<identifier>`, elsewhere:
  `dirs::data_local_dir()/<identifier>/logs`; on Linux that honors
  `$XDG_DATA_HOME`).
- After an upgrade from Riffle, the old `Riffle.log` sits beside `Handpic.log`
  in this folder: the first launch's data migration carried it over.
- `Help > Open Log Folder` opens the folder in the OS file manager.
- The same lines also go to stdout, which only a `mise run dev` terminal
  shows.

## Size and rotation

The cap is 1 MB (`max_file_size(1_000_000)`) with the plugin's default
`RotationStrategy::KeepOne`. In tauri-plugin-log 2.9.2 `KeepOne` **deletes**
`Handpic.log` when the next write would pass the cap and starts a new one; no
rotated copy (no `Handpic_<date>.log`) is left beside it, and only
`Handpic.log` is ever on disk. So the file holds the latest stretch of the
session, and the early lines of a long session may be gone. A failure that
happened before the deletion cannot be recovered; ask for a reproduction.

## Line format

```text
[YYYY-MM-DD][HH:MM:SS][target][LEVEL] message
```

- The time is UTC (the plugin's default `TimezoneStrategy::UseUtc`), not the
  user's local time.
- `target` is the Rust module path that logged the line: `riffle_app::commands`,
  `riffle_backend::index`, `riffle_backend::diagnostics`, `riffle_app::diagnostics`, `riffle_app` (the
  frontend's timing lines), or a dependency such as
  `tauri_plugin_updater::updater`.
- One record is one line, except a `panic:` record when a backtrace was
  captured (see below).

## Levels per build

- Release builds log `INFO` and above.
- Debug builds (`mise run dev`) log `DEBUG` and above, which includes a lot
  of dependency noise (`tract_core::optim` writes thousands of lines while
  the face model loads). Filter by target (`grep -E '\]\[riffle_(app|backend)'`) to read
  the app's own lines.

## Timing lines

Most `INFO` lines are timing lines, `<label>: key=value ... in <N>ms`: from
the backend `open list`, `open entries`, `scan list`, `scan prepare`,
`scan sidecars`, `scan reconcile` (`riffle_app::commands`) and `scan extract`,
`scan faces` (`riffle_backend::index`), `import` (`riffle_app::import`, one line per run with its throughput), and from the frontend, through the
`log_timing` command, its timing lines (`page wait= read= ipc= post= worker= decode= draw= total= keypressToPixels= jpegKB= px=WxH scan=idle|1|2|3 source=read|prefetch`, the per-page preview timings by stage, with `wait` and `keypressToPixels` omitted for a switch no key timed and `source=prefetch` for a preview drawn from the decoded neighbours, whose stages other than `wait` and `draw` are 0.0ms; `zoom keypress`, `eyes` (the shown file's closed-eyes judgment: read, decode, detect, model, ipc) and the like) under target `riffle_app`.
The frontend ones appear only while the settings modal's `Timing logs` item
(development builds only) is on. They describe performance, not failures.

## Failure lines

Grep for everything that signals a problem:

```sh
grep -E '\[(ERROR|WARN)\]|panic:|uncaught-js:|invariant:' Handpic.log
```

Plain `[WARN]` / `[ERROR]` lines without a prefix are failures the app
survived and reported in its own words (settings falling back to defaults,
`sidecar write failed: path=<path> <message>`, an index cache that would not
open). The three prefixed kinds follow.

### `panic:` (`ERROR`, target `riffle_backend::diagnostics`)

```text
panic: thread=<name or unnamed> location=<file>:<line>:<col> payload=<text>
```

- Written by the panic hook installed at the start of `.setup()`, for a panic
  on any thread (main, a `spawn_blocking` worker, a rayon worker, the sidecar
  writer). The default stderr message still follows. A panic before `setup`
  (plugin initialization) reaches stderr only.
- `location=unknown` when the panic carried no location;
  `payload=<non-string payload>` when the payload was neither `&str` nor
  `String`. Newlines in the payload are collapsed to spaces.
- When `RUST_BACKTRACE` is set, the backtrace follows on additional lines in
  the same record; `grep panic:` finds only the head line, so read on from
  it.
- **A `panic:` line is not necessarily a crash.** The hook runs before
  `catch_unwind`, so panics the core catches on purpose and turns into an
  `Err` are logged too. mozjpeg 0.10's decode errors are the exception: its
  error handler ends in `std::panic::resume_unwind`, which skips the hook, so
  a corrupt or non-JPEG preview caught by the decode / thumbnail / sharpness
  paths (`scan.rs`, `decode.rs`, `sharpness.rs` in `crates/core`) leaves no
  `panic:` line. Treat the line as a crash only
  when no recovery follows, for example when no scan failure for that file is
  reported and the thread's work stops (a scan that never finishes, a
  sidecar writer that writes nothing more).

### `uncaught-js:` (`ERROR`, target `riffle_app::diagnostics`)

```text
uncaught-js: kind=error message=<text> source=<url> line=<n> col=<n> stack=<frame> | <frame> | ...
uncaught-js: kind=unhandledrejection message=<text> stack=<frame> | <frame> | ...
```

- An uncaught error or unhandled promise rejection in the main window,
  forwarded through the `log_frontend` command. It is written in every build,
  whatever the `Timing logs` setting.
- Fields the event did not carry are omitted rather than written empty: a
  rejection has no `source` / `line` / `col`, and `stack` is present only when
  the thrown value is an `Error`. For a rejection with an `Error` reason,
  `message` is `reason.message`; any other reason is `String(reason)`. Stack
  frames are joined with ` | `.
- A flood guard forwards the first 20 distinct lines per session (a repeated
  identical line counts once), then writes one
  `uncaught-js: ... suppressed further uncaught errors for this session` and
  nothing more until the window reloads. After that notice, the absence of
  lines proves nothing.
- `source` / `stack` point into the bundled frontend; map them back to
  `crates/app/ui/src/` by the function names in the stack.

### `invariant:` (`WARN`)

```text
invariant: <name>: <details>
```

An internal assumption broke. The app does not change course on it (the
checks only log), so later behavior may be off in ways that follow from the
broken assumption. A normal session produces none. The checks:

| Name                         | Where                                                         | Details                                 | Means                                                                                                          |
| ---------------------------- | ------------------------------------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `scan progress within total` | `start_scan`'s `scan-progress` / `faces-progress` callbacks   | `dir=... scan_id=... done=... total=...` | a pass reported more files done than it was given; progress bars overflow                                      |
| `scan progress within total` (GPUI app) | `Scan::start`'s pass 1 and pass 2 progress callbacks in `crates/native/src/scan.rs` | `dir=... scan_id=... done=... total=...` | the same, for the native status line's `scanning` / `analyzing` counts |
| `start_scan once per scan`   | `start_scan`, when nothing is pending for `scan_id`            | `scan_id=... latest=...`                | `start_scan` was called again for a scan that is still running                                                 |
| `faces-done after scan-done` | the frontend's `faces-done` listener (`crates/app/ui/src/main.ts`) | `dir=... scan_id=...`                   | the frontend got a scan's `faces-done` without having recorded its `scan-done` (events out of order, or `scanDone` reset mid-scan) |
| `data migration` | `setup` in `crates/app/src/main.rs`, reporting what `migrate::run` did before the logger existed | `pair=<old>→<new> <path>: <error>`, or `<path>: <error>` for a macOS leftover that could not be removed | a folder of an earlier identifier could not be moved whole; it stays, the app shows an error and exits without starting (so this line reaches only stderr), and the next launch resumes, an existing destination file winning. A macOS leftover failure is logged and does not stop the app |

The two backend checks log under `riffle_app::commands` (the GPUI app's under
`riffle_native::scan`); the frontend one comes through `log_frontend` under
`riffle_app::diagnostics`.

The checks are not exhaustive, so no `invariant:` line proves nothing broke.
`start_scan once per scan` fires only while the first scan is still running; a
second `start_scan` after that scan finished (`finish` has cleared `running`)
logs nothing. It also logs nothing when `scan_folder` inserted no pending entry
(idle rescan of an indexed folder, or index cache unavailable) and `running`
does not hold `scan_id`; that is normal, not a violation.

## The GPUI app's log

`crates/native/src/logger.rs` writes every `log` line it prints to stderr also
to a `Handpic.log` of the native app's own, so the two apps never append to
one file.

| OS      | Path                                                              |
| ------- | ----------------------------------------------------------------- |
| Windows | `%LOCALAPPDATA%\dev.minodisk.handpic.native\logs\Handpic.log`     |
| macOS   | `~/Library/Logs/dev.minodisk.handpic.native/Handpic.log`          |
| Linux   | `~/.local/share/dev.minodisk.handpic.native/logs/Handpic.log`     |

- `--log <path>` names the file instead; every debug or hand-check run passes
  it (see `native-app.md`). A file that cannot be opened is reported once on
  stderr and the app runs with stderr only.
- The line format, the UTC time and the 1 MB cap are the Tauri app's: the
  file starts anew when the next line would pass 1 MB, leaving no rotated
  copy. The size is tracked in memory from the file's length at startup.
- Targets: `riffle_native` and its modules (`riffle_native::scan`,
  `riffle_native::settings`, ...), `riffle_backend::*` (`scan extract`,
  `scan faces`, `panic:` under `riffle_backend::diagnostics`), and GPUI's own
  `INFO` lines (`gpui_windows::*`: the UI font, the GPU).
- Levels: `INFO` and above in every build. `RUST_LOG=debug` (or `trace`) adds
  `riffle_native`'s lines of that level only, not the backend's or GPUI's.
- `panic:` comes from the backend's hook, installed in `main` right after the
  logger and before the window opens, so a startup panic is logged too.
  `invariant:` comes from `invariant!` as in the Tauri app. There is no
  `uncaught-js:`: the native app has no frontend.
- The timing lines (`page ...`, `zoom ...` and the `--auto` summary) are
  printed to stdout, where the measurement scripts read them, and are not in
  the file.
