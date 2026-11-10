# Learnings: gpui-strip-index

## Step 1

- `riffle-native` gained three direct dependencies, all already in its lock
  through `gpui` / `riffle-backend`: `dirs` 6, `futures` 0.3 and `log` 0.4
  (`log` was not named in the plan but the logger needs the trait). The lock
  gained only those three dependency lines.
- `Scan::prepare` takes the listing (`scan::list`, which canonicalizes and
  calls `read_listing(dir, None)`) rather than the folder, so a failed
  `reconcile` still leaves the listing for the pager: the index error is
  logged and no scan starts, the pager pages as without an index.
- `std::fs::canonicalize` on Windows gives `\\?\D:\...`; every index key and
  every listed path in the native app carries that prefix, as in the Tauri
  app. `read_preview` reads such paths fine.
- The backend does not log `scan list` / `scan reconcile` (those lines are in
  `crates/app`'s `scan_folder`); `scan.rs` logs its own with the same names.
- The eviction thread is spawned when the view is created and the first open
  joins its handle on the background executor before `reconcile`, so the scan
  always waits for the `VACUUM` instead of racing it for the writer `Mutex`.
- A stale `riffle-native.exe` from an earlier session (PID 58260, started
  2026-11-09) held `target/debug/riffle-native.exe`, so `cargo build` failed
  with `os error 5`. Renaming the running exe aside
  (`riffle-native.stale-58260.exe`) let the build go through without killing
  the process.
- The logger shows GPUI's own lines too, among them an `ERROR` from
  `gpui_windows::directx_devices` (0x887A002D, no DXGI debug interface), which
  is harmless.

### Hand-run (debug build, 2026-11-10)

- `riffle-native.exe D:\Photos\tests\20261108-gpui-compare\ARW --auto 3
  --index D:\Photos\tests\20261110-gpui-strip-index\index.sqlite`, twice, a
  fresh index file. Run 1: `scan reconcile: ... todo=20 removed=0`,
  `scan extract: ... files=20 done=20 errors=0 threads=22 ... in 2136ms`.
  Run 2: `todo=0`, `scan extract: ... files=0 done=0 errors=0 ... in 1ms`
  (rows reused). Both quit by themselves with the `page` lines and summary.
- A copy of that folder plus a 64 KB random-bytes `DSC-RX100M3_corrupt.ARW`
  (`D:\Photos\tests\20261110-gpui-strip-index\ARW`, own fresh index),
  `--auto 23`: `scan extract: ... files=21 done=21 errors=1`, the status line
  read `1/21  DSC-RX0M2_sony_rx0_ii_40.arw  ...  1 failed`, the pager printed
  `error DSC-RX100M3_corrupt.ARW: invalid TIFF byte order` and went on: 22
  `page` lines, every press shown.
- The `scanning done/total` text itself was not caught on screen: the warm
  20-file pass ended (55 ms, 2.1 s cold in debug) before a screenshot did.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the user's eyes): the status line's
  `scanning done/total` climb. Open a folder of a few hundred RAW files under
  `D:\Photos\` with `riffle-native.exe <folder> --index <a fresh file under
  D:\Photos\tests\>`; expect `scanning N/total` after the file info, N rising
  about every 100 ms, replaced at the end by nothing (or `N failed`). Step 1
  was ticked on the automated criteria and the stderr / `1 failed` checks
  above. Basis: Step 1's Done-when "the first run shows the progress climb to
  20/20"; files: `crates/native/src/main.rs` (`Riffle::status`).
