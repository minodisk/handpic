# Learnings: flaky-filter-uitest

## Cause

- `uitest::a_filter_choice_narrows_the_files_and_lands_on_the_anchor` failed
  intermittently on `main`'s CI: twice on 86bb0180, on d6e1b4ce (ubuntu) and
  on accb630d (windows); 913eb532 and 6ce7797e passed. Every failure was the
  10 s `wait_until` (`WAIT`) inside `open_folder` timing out with the status
  `analyzing 3/4`.
- Not a hang but the cost of pass 2 in the dev / test profile. `wait_idle`
  waits for both scan passes, and pass 2 (`run_faces_scan`: YuNet and the
  MediaPipe face mesh through tract) ran unoptimized, about ten times slower.
- On a 4-core CI runner `faces_threads()` (`crates/app/src/scan.rs`) gives
  each scan one pass 2 thread while about ten UI tests open folders at once in
  the same test binary. This is the only UI test with 4 files, so it alone
  crossed 10 s. The ubuntu failure log shows the watcher's `resync ...
  deferred` lines about 2 s apart, one per pass 2 file read.
- #1228 (d6e1b4ce) added three more concurrent UI tests, raising the
  contention; that is the link to the commit the failures started around.
- Instrumenting `ReadSlots` showed no `PaintHold` pause waits, so nothing was
  held: no hang.

## Fix

`crates/app/Cargo.toml`'s `[profile.dev.package]` now holds `opt-level = 3`
for `tract-core`, `tract-data`, `tract-extra`, `tract-hir`, `tract-linalg`,
`tract-nnef`, `tract-onnx`, `tract-onnx-opl`, `ndarray` and
`matrixmultiply`. `WAIT` stays 10 s; no UI test changed.

The comment in the first working-tree draft said the UI tests' pass 2 runs
"below normal priority"; `Ui::open` sets both passes to `Priority::Normal`,
so the committed comment drops that.

## Measurements (during planning)

- Before: one test alone, pass 2 of 4 files took 1775 ms (24 cores, 3 pass 2
  threads). Pinned to 4 cores, every UI test's folder wait was 5.0 to 7.3 s.
- After: one test alone, pass 2 took 182 ms. The longest folder wait was
  0.82 s pinned to 4 cores and 1.19 s pinned to 2 cores. The full app test
  binary passed 1095 / 1095, 3 runs at 4 cores and 3 runs at 2 cores.

## Re-run on the committed tree (Windows, 2026-11-22)

- `cargo test --bin handpic a_filter_choice_narrows` in `crates/app`, 10 runs
  in a row: all passed, each finishing in 0.71 to 1.08 s.
- The full app test binary (`target/debug/deps/handpic-*.exe`, found through
  `cargo test --bin handpic --no-run --message-format=json`) started from
  PowerShell with `Start-Process -PassThru -NoNewWindow` in `crates/app` and
  `ProcessorAffinity` set to `0xF` (4 cores): `test result: ok. 1095 passed;
  0 failed`, finished in 7.32 s.
- `Start-Process -PassThru -NoNewWindow` reports an empty `ExitCode` unless
  the process handle is touched before it exits; read the `test result` line
  from the redirected output instead.
