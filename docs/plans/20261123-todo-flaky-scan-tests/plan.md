<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Flaky scan and settings tests, and the post-read cancel test

## Purpose

Three tests fail now and then on a loaded machine or on CI because they
measure the machine's load rather than the behavior they are about, and one
cancel check of the scan has no test. This work closes these four `todo.md`
items:

- `### Core: test the scan's post-read cancel check`
- ``### Core: `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` hits its 1 s bound under parallel load``
- `### App: the settings-store tests are flaky on CI`
- ``### App: `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped` is flaky on a loaded machine``

Once done, each of those tests passes or fails on what the code does, with at
most a bound that only a real hang reaches, and a file canceled during its
read is tested as not delivered.

The wrap-up phase deletes exactly those four headings (with their bodies) from
`todo.md` and nothing else: other sessions edit `todo.md` concurrently, so
find each item by its heading text, not by line number, and leave every other
item and `docs/plans/20261123-all-faces-check` alone.

## Steps

- [x] Step 1: Remove the timing dependence of the three tests and add the
      post-read cancel test
  - Done when:
    - `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit`
      (`crates/core/src/scan.rs`) no longer asserts how long the cancel took.
      It asserts that the scan ended while the pause or the permit was still
      held and that nothing was delivered, with at most a bound that only a
      real hang reaches.
    - `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped`
      (`crates/app/src/scan.rs`) no longer depends on the scan's thread ending
      within `STOP_TIMEOUT` of the call on a loaded machine.
    - `settings::tests::reads_the_stored_values` and
      `settings::tests::a_stored_recommended_is_not_rewritten_by_the_load`
      (`crates/app/src/settings.rs`) pass the repeated run below. Any failure
      it shows is fixed at its cause; if it shows none, no code changes and
      the evidence (command, run count, result) is recorded in `learnings.md`.
    - A new test in `crates/core/src/scan.rs` sets `cancel` during the read
      and asserts that `extract_unless` and `extract_analysis_unless` both
      return `None`. For `extract_unless` the test fails when the first
      post-read `canceled` check is removed; confirm this once by removing the
      check locally, and record it in `learnings.md`.
    - Each command below passes 30 runs in a row under the CPU load (24 busy
      loops, reduced from 48 with the user's approval after the host ran low
      on memory), on
      Windows, with the result recorded in `learnings.md`. Run each as one
      shell command, from the repository root for the core test and from
      `crates/app` for the app tests. The busy loops are plain bash because
      bare `python` / `node` hang in Git Bash on this host.
      - Core, the cancel test and the new test (replace `<new test>` with its
        name):

        ```bash
        hogs=(); for i in $(seq 24); do (while :; do :; done) & hogs+=($!); done
        for i in $(seq 30); do
          cargo test -p handpic-core --lib -- --exact \
            scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit \
            'scan::tests::<new test>' || { echo "FAILED on run $i"; break; }
        done
        kill "${hogs[@]}"
        ```

      - App, the three tests, in `crates/app` with the toolchain
        `tools/ci/local.sh` pins as `app_rust` and NASM on `PATH`:

        ```bash
        export PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH"
        hogs=(); for i in $(seq 24); do (while :; do :; done) & hogs+=($!); done
        for i in $(seq 30); do
          mise exec rust@1.98.1 -- cargo test --bin handpic -- --exact \
            scan::tests::a_bounded_stop_reports_an_ended_scan_stopped \
            settings::tests::reads_the_stored_values \
            settings::tests::a_stored_recommended_is_not_rewritten_by_the_load \
            || { echo "FAILED on run $i"; break; }
        done
        kill "${hogs[@]}"
        ```

      - The whole-module loops (core `scan::tests`, app
        `scan::tests settings::tests`, in parallel) are not required
        (user's decision, 2026-11-24): a debug-build core module run takes
        over 8 minutes under the load and the host ran low on memory twice.
        The two core tests above passing 30 of 30 under 48 busy loops, and in
        every whole-module log that reached them, stands in for it.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - General: test code only, plus one `#[cfg(test)]` hook in
      `crates/core/src/scan.rs`. No public API, signature or behavior changes
      in a non-test build. `crates/app` is its own Cargo workspace. Suggested
      PR title: `test: remove the timing dependence of the scan tests`.
    - Before changing a test, run its loop above on the unchanged code and
      record whether the failure reproduces. The fix does not depend on
      reproducing it; the 2.91 s and the one-in-four failures were seen under
      a full parallel `cargo test`.
    - `a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit`: the assertion
      `took < Duration::from_secs(1)` measures the cancel tick
      (`CANCEL_TICK`, 50 ms), the rayon pool's teardown and the scheduler.
      What the test is about is already observable without a clock:
      - Pause case: an ignored cancel ends only when `LONG_PAUSE` (60 s)
        runs out, and then the files are read and `seen` is not empty.
      - Permit case: an ignored cancel never ends, because the slot is
        dropped only after `scan.join()`.

      So drop the elapsed-time assertion and keep "the scan ends while
      `paused` / `slot` are still held" and "`seen` is empty". To turn a
      permit-case hang into a failure instead of a stuck run, let the scan
      thread signal its end (a `std::sync::mpsc` channel, or polling
      `is_finished`) and wait for that with a hang-only bound well under
      `LONG_PAUSE` (e.g. 30 s). On a timeout, drop the guards before failing
      so `thread::scope` can join. The step decides between that bound and no
      bound at all. The `SETTLE` sleep before the cancel stays.
    - `a_bounded_stop_reports_an_ended_scan_stopped`: `Scan::start` with an
      empty listing still spawns a thread, runs `index::run_scan` and
      `run_faces_pass` (each builds a rayon pool and touches the SQLite
      index), and the test gives all of it `STOP_TIMEOUT` (2 s, the app's quit
      bound, which must not change). The test's name is about an ended scan,
      so wait for the end first: `Scan`'s private `ended` receiver is visible
      to the `tests` module and disconnects when the thread ends, so block on
      it (a plain `recv`, or a hang-only `recv_timeout`) before
      `running.put(scan)`. Then `running.stop_within(STOP_TIMEOUT)` sees the
      disconnected channel at once and joins. Keep the other assertions
      (`stop_within` with none held, `is(7)`, `take()` empty afterwards). Only
      the test changes.
    - Settings-store tests: the todo item's guess (the writer thread had not
      flushed the seed) does not match the code. `Store::open` commits the
      seed with the schema in one transaction before `load_from` returns. The
      known cause of a reopen reading the defaults was the writer thread still
      holding the store open after the `Settings` drop, which Windows refuses
      a second open on. #1191 (`fc61ec59`) fixed that: the last `Writes`
      clone's drop closes the queue and joins the thread (`impl Drop for
      Writer`), and it removed the retry loop from the sibling seed-reload
      test. Both tests named in the item use `drop(seeded(..))` then a reopen,
      and neither has a wall-clock wait, so expect no change here. Only if the
      loop shows a failure, find its cause and fix that (never a sleep or a
      retry).
    - Post-read cancel test: add a test-only way to set `cancel` during the
      read in `crates/core/src/scan.rs`. The smallest known shape is a
      `#[cfg(test)]` thread-local flag and a `#[cfg(test)]` call just before
      the `read_preview(path)` call, inside the read slot, of `extract_unless` and of
      `extract_analysis_unless`, which stores `true` into `cancel` when the
      flag is on. The test calls both functions directly on its own thread, so
      a thread-local keeps parallel tests apart; reset the flag on drop. The
      other shape, passing the reader as a parameter, changes the functions
      `for_each_path` takes and is the larger diff. The step decides, keeping
      the non-test build unchanged.
    - In that test, use both a good fixture (`fixture(1, &jpeg(64, 48))`) and
      one whose preview is not a JPEG (`fixture(6, &[0u8; 512])`, as
      `a_file_whose_preview_is_not_a_jpeg_is_one_error_not_a_crash` uses),
      with `cancel` starting `false`. The bad-preview case is what makes the
      `extract_unless` assertion sensitive to the first post-read check:
      without it the thumbnail failure returns `Some(Err(..))` before the
      second check.
    - For `extract_analysis_unless`, assert `None` for a file canceled
      mid-read. Removing only its first post-read check does not change the
      outcome: with an AF point the code checks `cancel` again right after the
      cue, and without one right after the whole-image detection. See
      "Trade-offs and risks".
    - `docs/plans/20261123-all-faces-check` Step 1 also changes
      `extract_analysis_unless`. Keep this step's change there to the one hook
      line so either order rebases cleanly.

## Trade-offs and risks

- **The analysis pass's first post-read check cannot fail a test by its
  outcome alone.** In `extract_analysis_unless` every path after the read
  reaches another `canceled` check before anything is returned, so removing
  only the first one still gives `None`; it saves work, not a delivery.
  Decided (user approval, 2026-11-23): assert the outcome (`None`) for a file
  canceled mid-read, which is what the todo item asks for, and make the test
  strictly sensitive only in `extract_unless`. A test-only counter of the
  detection / cue stage was the rejected alternative.
- **The app's stop test waits for the scan's end, then calls `stop_within`**
  (decided). Deterministic and matches the test's name, but the test no
  longer exercises "cancel a still-running scan, then wait". The rejected
  alternative kept that path with a test-local hang-only bound (e.g. 60 s).
- **The settings-store item may close with no code change.** If the repeated
  run shows no failure, the item is closed on the evidence that #1191 removed
  the cause. The one CI failure (run 43465601803) predates that fix.
- **Other wall-clock bounds stay.** `scan::tests` in `crates/core` has more
  tests with 1–2 s bounds (`a_wait_for_no_pause_returns_at_once`,
  `a_wait_stops_on_cancel`, `a_wait_ends_when_the_pause_does`,
  `a_wait_is_bounded_by_the_maximum_pause`,
  `a_yield_outside_pass_two_ignores_a_pause`). They are not in the four items
  and are not changed. The whole-module load run may show them failing, which
  is recorded, not fixed.
- **`SETTLE` stays.** The cancel test still sleeps 150 ms so the pool reaches
  the gate. If it has not by then, the workers see the cancel before their
  first file and the test passes without having waited at the gate: weaker
  under extreme load, never a false failure.
- **Same file as the all-faces-check plan.** A merge conflict in
  `crates/core/src/scan.rs` is possible; this plan's production-side change is
  one `#[cfg(test)]` line per function.

## Progress
- (2026-11-24) Step 1 complete (whole-module load loops waived by the user; settings tests closed on evidence)
