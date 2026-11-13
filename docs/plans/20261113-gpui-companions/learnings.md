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
