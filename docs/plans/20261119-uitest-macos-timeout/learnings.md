# Learnings: uitest-macos-timeout

## Step 1

### Cause: pass 2 slow under `QOS_CLASS_BACKGROUND`, not stuck

The diagnostics commit (`1b44b99c`, a test logger, a 90 s grace wait after
the 10 s timeout and a probe timing one file's `extract_analysis` at normal
priority on the test thread and through `extract_analysis_all` at
`Priority::Lowest`) ran on PR #1173, CI run 44345428381 attempt 2, macOS
`native` job 134951506173. All five folder-opening tests timed out (status
`analyzing 0/0` or `analyzing 1/N`), and every one became idle within the
grace wait: pass 2 ended, it did not hang. The key lines:

| test | pass 1 (`scan extract`) | pass 2 (`scan faces`) | probe normal | probe `Lowest` |
| --- | --- | --- | --- | --- |
| judgment key (2 files) | 2 ms | 16941 ms | 550 ms | 3244 ms |
| opening a folder (3 files) | 1 ms | 19086 ms | 675 ms | 8873 ms |
| settings modal (2 files) | 3 ms | 14787 ms | 610 ms | 4942 ms |
| paging keys (3 files) | 1 ms | 24774 ms | 518 ms | 5581 ms |
| filter choice (4 files) | 2 ms | 43644 ms | 880 ms | 7416 ms |

For example:
`19.288s [handpic_backend::index][INFO] scan faces: dir=/private/var/.../.tmpVuyC2P/photos files=3 done=3 errors=0 threads=1 canceled=false in 19086ms`
and
`done after 19.023529167s; probe normal true in 675.125333ms, lowest Some(true) in 8.872777667s`.

The same one-file analysis took 6 to 14 times longer at `Priority::Lowest`
(`QOS_CLASS_BACKGROUND` on macOS) than at normal priority on the same runner,
and pass 1 (`QOS_CLASS_UTILITY`) took milliseconds. The runner gives the scan
one worker (`threads=1`), so the five tests' pass 2 workers share it. No
"ran at normal priority" warning was logged, so the QoS was applied. This is
throttling, not an app bug: no app unit test is added for it.

An earlier session's evidence agrees: in runs 44322882065, 44323630264 and
44301956697 the tests started about 10 s later also got no pass 2 file done
in over 20 s, and main run 44317498801 passed using about 8.7 s of the 10 s.

A forced local timeout on Windows (`WAIT` set to 1 ms, run alone right after
a rebuild) showed pass 2 at 8.2 s for 3 files and the probe at about 0.95 s
normal against 1.5 s `Lowest`: Windows `THREAD_PRIORITY_LOWEST` costs little.

### Fix

Option A of the plan: the passes' priority is a parameter of
`index::run_scan` and `index::run_faces_scan` (the backend's tests pass the
app's values), `Scan::start` takes `scan::Priorities` (`APP`: pass 1
`BelowNormal`, pass 2 `Lowest`), the view holds `scan_priorities`, and
`Ui::open` sets both passes to `Priority::Normal`.

### Notes

- The user briefly asked to ignore the five tests on macOS instead, then
  withdrew it; that approach was considered and dropped, and nothing of it
  was pushed.
- A Python heredoc that writes Rust source turned `"\\n"` into a real
  newline inside a string literal; check the diff after such scripted edits.

## Deferred issues (todo candidates)

- macOS users may see `analyzing 0/N` for long: pass 2 runs at
  `Priority::Lowest`, `QOS_CLASS_BACKGROUND` on macOS, which throttled it
  heavily on the macOS CI runner. Measured there (PR #1173, run 44345428381
  attempt 2, job 134951506173): one tiny generated DNG's `extract_analysis`
  took 3.2 s to 8.9 s at `Lowest` against 0.5 s to 0.9 s at normal priority
  on the same runner, and pass 2 of 2 to 4 files 15 s to 44 s (pass 1 at
  `QOS_CLASS_UTILITY`: milliseconds). Not measured on a real Mac. The plan's
  option D (pass 2 at `QOS_CLASS_UTILITY` on macOS, perhaps with a negative
  relative priority through `pthread_set_qos_class_self_np(class, relpri)`;
  `libc` has no `QOS_MIN_RELATIVE_PRIORITY`, and the
  `every_worker_runs_at_the_priority_it_was_given` test is
  `cfg(windows, linux)` only) changes the paging latency the priority was
  chosen for, so it needs measuring on a Mac first. The UI tests no longer
  see it (they run the scan at normal priority). Related files:
  `crates/core/src/scan.rs` (`lower_current_priority`),
  `crates/native/src/scan.rs` (`Priorities::APP`),
  `docs/agents/shared-crates.md` ("The scan pool's priority levels"),
  `docs/humans/performance.md` and `docs/humans/performance.ja.md`.
