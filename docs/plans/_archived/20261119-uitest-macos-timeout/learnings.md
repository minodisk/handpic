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

Option D, on the user's decision once the evidence was in (option A was
implemented first as `9a4a6752`, the passes' priority a parameter of
`index::run_scan` / `index::run_faces_scan` that the harness set to
`Normal`; its CI run 44409033574 was cancelled and the code reverted, since
D also spares users the slow pass 2): on macOS `Priority::Lowest` is now
`QOS_CLASS_UTILITY` at relative priority -5 (`LOWEST_RELATIVE_PRIORITY` in
`crates/core/src/scan.rs`), `BelowNormal` stays `UTILITY` at 0, and
`every_worker_runs_at_the_priority_it_was_given` gained a macOS branch that
reads the class and relative priority back with `pthread_get_qos_class_np`.
`WAIT` stays 10 s. The harness keeps the test logger and the cascade fix; the
grace wait and the probe were hunt-only and are gone.

Result on `361d5c4e` (`fix(core): run the analysis pass at QoS utility on
macOS`), CI run 44410310963: every job passed on attempt 1, and the macOS
`native` job passed 3 times in a row, all six `uitest` tests ok each time
(job 135176364852 attempt 1, the crate's tests in 3.59 s; job 135183066315
attempt 2, 6.02 s; job 135185406000 attempt 3, 10.61 s, against 52.78 s with
the five timeouts before). `scan::tests::every_worker_runs_at_the_priority_it_was_given`
passed in the macOS `test` job (135176364686 of the same run, attempt 1), so the macOS
branch reads back `UTILITY` at -5 and 0. The commit that adds these lines
changes only this file.

`cargo check --target aarch64-apple-darwin` cannot run on this Windows host
(`mozjpeg-sys` needs a C compiler for the target), so the macOS branch is
only compiled and run by the macOS CI jobs.

### Option A added on top of D (Windows starvation)

PR #1173's CI run 44424162778 (head `3d0577fa`), Windows `native` job
135224757562, failed one UI test,
`uitest::a_filter_choice_narrows_the_files_and_lands_on_the_anchor`:

```text
timed out waiting for the folder's listing, scan and preview: ... listed 4, listing false, scan Some(("analyzing", 3, 4)), running false, ...
     0.548s [handpic::scan][INFO] scan list: dir=\\?\C:\...\.tmpcEvsJ7\photos files=4 in 0ms
     0.553s [handpic_backend::index][INFO] scan extract: dir=\\?\C:\...\.tmpcEvsJ7\photos files=4 done=4 errors=0 threads=2 canceled=false in 2ms
test result: FAILED. 1067 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.05s
```

Pass 1 took 2 ms, and pass 2, at `Priority::Lowest`
(`THREAD_PRIORITY_LOWEST`), had done 3 of 4 files after the 10 s wait,
where it normally takes about 0.7 s for 3 files: it starved behind the
crate's other tests running in parallel at normal priority. So the
starvation under test load is not macOS-only; D only removed macOS's
`BACKGROUND` throttling.

Decision (the user's): keep D for the app and add option A back on top of it,
adapted from `9a4a6752`: `index::run_scan` / `index::run_faces_scan` take a
`Priority` (the backend's tests pass the app's values; the embedding pass is
untouched), `scan::Scan::start` takes `scan::Priorities` (`APP`: pass 1
`BelowNormal`, pass 2 `Lowest`), the view holds `scan_priorities` (`APP`),
and `Ui::open` sets both passes to `Priority::Normal`. The CLI does not call
`run_scan` / `run_faces_scan`, so its benchmarks stay at `Priority::Normal`.

Local result on Windows: the native crate's whole `cargo test --bin handpic`
(the six `uitest` tests among the 1059, so they run under the same parallel
load as on CI) passed 20 runs of 20, each in 2.0 s to 3.1 s.

### Notes

- The user briefly asked to ignore the five tests on macOS instead, then
  withdrew it; that approach was considered and dropped, and nothing of it
  was pushed.
- A Python heredoc that writes Rust source turned `"\\n"` into a real
  newline inside a string literal; check the diff after such scripted edits.

## Deferred issues (todo candidates)

- Pending manual check (macOS, real machine): the paging latency with pass 2
  at `QOS_CLASS_UTILITY` relative priority -5 instead of
  `QOS_CLASS_BACKGROUND`. Steps: open a large RAW folder with a cold index in
  the native app and page through it while `scanning N / M` and then
  `analyzing N / M` run; expect paging to stay as responsive as on Windows
  (the `page` timing lines in `Handpic.log`), the analysis pass not to crawl
  and no `ran at normal priority` warning in the log. Basis: option D of this
  plan, chosen on the CI evidence alone (the throttling was measured on the
  macOS CI runner, not on a Mac; see "Cause" above). At the wrap-up, extend
  the existing todo item "### Core: real-device check of the scan priority's
  macOS QoS branch" (`todo.md`) with this rather than adding a duplicate: its
  check still names `QOS_CLASS_BACKGROUND` and its "move it to
  `QOS_CLASS_UTILITY`" remedy is now done. The step's checkbox was ticked on
  the automated criteria. Related files: `crates/core/src/scan.rs`
  (`lower_current_priority`, `LOWEST_RELATIVE_PRIORITY`),
  `docs/agents/shared-crates.md` ("The scan pool's priority levels"),
  `docs/humans/performance.md`, `docs/humans/performance.ja.md`.
