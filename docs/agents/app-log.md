# Reading `Handpic.log`

Read this before diagnosing a report about the running app. `Handpic.log` is
the only trace a session leaves: stderr is not captured in a bundled app, so
anything that matters is written here, and the failures an agent most needs
carry a fixed, greppable prefix (`panic:`, `invariant:`).

The log is written by `crates/app/src/logger.rs`; the prefixed lines come
from `crates/engine/src/diagnostics.rs` (the panic hook and the `invariant!`
macro).

## Where the file is

| OS      | Path                                                   |
| ------- | ------------------------------------------------------ |
| Windows | `%LOCALAPPDATA%\dev.minodisk.handpic\logs\Handpic.log` |
| macOS   | `~/Library/Logs/dev.minodisk.handpic/Handpic.log`      |
| Linux   | `~/.local/share/dev.minodisk.handpic/logs/Handpic.log` |

- `logger.rs`'s `log_path` builds these: `~/Library/Logs/<identifier>` on
  macOS, `dirs::data_local_dir()/<identifier>/logs` elsewhere (on Linux that
  honors `$XDG_DATA_HOME`), the shape of Tauri's `app_log_dir`, which the
  earlier app used.
- `--log <path>` names the file instead; every debug or hand-check run passes
  it (see `app.md`). A file that cannot be opened is reported once on
  stderr and the app runs with stderr only, which on a Windows release build
  means no log at all unless stderr is redirected.
- The same lines also go to stderr, which only a `mise run app:dev`
  terminal shows. A Windows release build is a GUI-subsystem binary with no
  stderr unless it is redirected to a pipe or a file (see `app.md`'s "A
  Windows release build is a GUI-subsystem binary").

## Size and rotation

The cap is 1 MB (`logger.rs`'s `MAX_FILE_SIZE`). The file starts anew when the
next line would pass it; no rotated copy is left beside it, and only
`Handpic.log` is ever on disk. The size is tracked in memory from the file's
length at startup. So the file holds the latest stretch of the session, and
the early lines of a long session may be gone. A failure that happened before
the restart cannot be recovered; ask for a reproduction.

## Line format

```text
[YYYY-MM-DD][HH:MM:SS][target][LEVEL] message
```

- The time is UTC, not the user's local time.
- `target` is the Rust module path that logged the line: `handpic` and its
  modules (`handpic::scan`, `handpic::settings`, ...), `handpic_engine::*`
  (`handpic_engine::index`, `handpic_engine::diagnostics`, ...), or a
  dependency such as GPUI's own `INFO` lines (`gpui_windows::*`: the UI font,
  the GPU). A log written by a build before the crate was renamed to
  `handpic-engine` carries the same targets with `backend` in place of
  `engine`.
- The app crate is bin-only, so its crate name, and with it the `target` and
  `CARGO_CRATE_NAME` the logger's own-lines filter uses, comes from
  `[[bin]].name` (`handpic`), not from the package name (`handpic-app`).
  `handpic` is also a string prefix of `handpic_engine` and `handpic_core`, so
  `logger.rs`'s `is_own` matches the target exactly or as a `handpic::` module
  path. A plain `starts_with` let the engine's debug lines through under
  `RUST_LOG=debug`. Renaming the binary changes every target, so recheck that
  filter and its test. Source:
  `docs/plans/_archived/20261118-remove-tauri/learnings.md`, Step 2.
- One record is one line, except a `panic:` record when a backtrace was
  captured (see below).

## Levels

- `INFO` and above in every build.
- `RUST_LOG=debug` (or `trace`) adds `handpic`'s lines of that level only, not
  the engine's or GPUI's.

## Timing lines

Many `INFO` lines are timing lines, `<label>: key=value ... in <N>ms`:
`scan list`, `scan reconcile`, `scan sidecars` (`handpic::scan`), `scan
extract`, `scan faces` (`handpic_engine::index`) and `import`
(one line per run with its throughput). They describe performance, not
failures.

The per-page timing lines (`page ...`, `zoom ...` and the `--auto` summary)
are printed to stdout, where the measurement scripts read them, and are not in
the file.

## Failure lines

Grep for everything that signals a problem:

```sh
grep -E '\[(ERROR|WARN)\]|panic:|invariant:' Handpic.log
```

Plain `[WARN]` / `[ERROR]` lines without a prefix are failures the app
survived and reported in its own words (settings falling back to defaults,
`sidecar write failed: path=<path> <message>`, an index cache that would not
open). The two prefixed kinds follow.

### `panic:` (`ERROR`, target `handpic_engine::diagnostics`)

```text
panic: thread=<name or unnamed> location=<file>:<line>:<col> payload=<text>
```

- Written by the panic hook, installed in `main` right after the logger and
  before the window opens, for a panic on any thread (main, a background
  task, a rayon worker, the sidecar writer), so a startup panic is logged
  too. The default stderr message still follows, which a Windows release
  build launched without a redirect writes nowhere.
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

### `invariant:` (`WARN`)

```text
invariant: <name>: <details>
```

An internal assumption broke. The app does not change course on it (the
checks only log), so later behavior may be off in ways that follow from the
broken assumption. A normal session produces none. The checks:

| Name                         | Where                                                                              | Details                                  | Means                                                                                                    |
| ---------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `scan progress within total` | `Scan::start`'s pass 1 and pass 2 progress callbacks in `crates/app/src/scan.rs` | `dir=... scan_id=... done=... total=...` | a pass reported more files done than it was given; the status line's `scanning` / `analyzing` counts overflow |

The check logs under `handpic::scan`. The checks are not exhaustive, so no
`invariant:` line proves nothing broke.

### Thumbnail read lines (strip cells that stay gray)

Two lines in `crates/app` name why a strip cell shows no image while pass 1
still runs. Grep for them when cells in view stay gray on a fresh-index open.

- `thumbnail read failed, the cell waits for the scan:` (`WARN`, from
  `thumbs::read`) is any read error except rusqlite's `Query returned no
  rows` (`no_row`; a row not yet committed is normal and silent).
- `thumbnail not scanned after the scan reported it: ... retried=<bool>` (from
  `Handpic::thumbnail`): the scan reported the cell but the read answered
  `NotScanned`. `retried=true` (`INFO`) is the expected first answer from a
  read that ran before the commit. `retried=false` (`WARN`) means the cell
  stayed missing until the pass ended.
- Neither line for a gray cell points at the scan order (rows not written
  early), or an image decoded but not painted, rather than the reader.
- The gray-cell cause was not reproduced in tests (the device's 90 s pass 1
  against about 2 s in the test), so these lines are the diagnosis tool.
  Source: `docs/plans/_archived/20261127-jpeg-strip-gray-cells/learnings.md`.
