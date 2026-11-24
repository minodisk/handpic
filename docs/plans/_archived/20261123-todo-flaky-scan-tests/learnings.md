# Learnings

## Step 1

- Baseline, unchanged code: the core loop of
  `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` alone
  under 48 busy loops (24 logical CPUs) failed on run 8 of 30 with
  `took 1.0520229s`, so the 1 s bound reproduces on this host under load.
- The cancel test now waits on an `mpsc` channel the scan thread's end
  disconnects, with `HANG` (30 s, under `LONG_PAUSE`) as the only bound, drops
  the pause or the permit before asserting, then joins.
- The post-read hook (`tests::cancel_mid_read`) sits inside the read's slot
  just before `read_preview(path)`, not after it: `read_preview` never looks at
  `cancel`, so setting it there is the same as setting it during the read, and
  the hook stays one `#[cfg(test)]` statement per function without turning the
  block's tail into a binding.
- Sensitivity, confirmed once: with the first post-read `canceled` check of
  `extract_unless` removed, `a_file_canceled_during_its_read_is_not_delivered`
  failed on `bad.ARW` (the thumbnail failure returned `Some(Err(..))`); with
  it restored the test passes.
- After the change, the core loop of the cancel test and
  `scan::tests::a_file_canceled_during_its_read_is_not_delivered` under 48 busy
  loops: 30 of 30 passed.
- The core whole-module loop (`scan::tests -- --test-threads=32`) was stopped
  by Claude Code at run 2 or 3 because the host ran low on memory (other
  sessions were building at the same time); run 2 had exited non-zero but its
  log was overwritten before it could be read. Still to run: that loop, the
  app loops, and `mise run ci:local`.
- Reduced load, approved by the user (2026-11-23) after that memory stop: 24
  busy loops instead of 48 and `--test-threads=8` instead of 32 for the
  whole-module loops (core `scan::tests`, app `scan::tests settings::tests`),
  still 30 runs each, each run's output kept in its own log.
- Core whole-module loop at the reduced load, second attempt: run 1 passed
  (38 tests, 505.51 s: a debug-build `scan::tests` under 24 busy loops takes
  over 8 minutes a run, so 30 runs would take about four hours). Runs 2 and 3
  exited non-zero with no test failing in their logs (run 2 cut off mid-run,
  with no result line; run 3 an empty log), so the test process was terminated
  from outside rather than failing an assertion. Run 4 was cut off mid-run
  when Claude Code stopped the loop again for low memory. In every log that
  reached them, `a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` and
  `a_file_canceled_during_its_read_is_not_delivered` passed. Still to run:
  that loop, the app loops, and `mise run ci:local`.
- Decision (user-approved, 2026-11-24): the whole-module loops, core and app,
  are not run again and count as met by the evidence above: the two core
  target tests passed 30 of 30 under 48 busy loops, and passed in every
  whole-module log that reached them. The plan's Done-when no longer requires
  those loops.
- App loop, after the change, from `crates/app` with `rust@1.98.1` and NASM
  on `PATH`, under 24 busy loops: `cargo test --bin handpic -- --exact
  scan::tests::a_bounded_stop_reports_an_ended_scan_stopped
  settings::tests::reads_the_stored_values
  settings::tests::a_stored_recommended_is_not_rewritten_by_the_load`, 30 of
  30 passed. The three tests took from 0.25 s to 25.66 s a run (most 2 to
  4 s), well past the 2 s `STOP_TIMEOUT` the stop test used to give the
  whole scan, which is why it was flaky. The app loop was not run on the
  unchanged code first (the edit came before the app build).
- Settings-store evidence: the same 30 runs under load passed both settings
  tests with no change to `crates/app/src/settings.rs`. This matches the plan's
  reading that #1191 (`fc61ec59`, the writer thread joined on the last
  `Writes` drop) removed the cause; the one CI failure predates it. The item
  closes with no code change.
- Bare `python3` hangs in Git Bash here, as the plan warns; use the Edit tool
  or `sed` for scripted edits.
- `mise run ci:local` ran after the Round 1 fixes (2026-11-24): every selected check passed.
