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

# flaky-filter-uitest

## Purpose

The UI test `uitest::a_filter_choice_narrows_the_files_and_lands_on_the_anchor`
(`crates/app/src/uitest.rs`) fails intermittently on `main`'s CI: twice on
86bb0180, on d6e1b4ce (ubuntu) and on accb630d (windows), while 913eb532 and
6ce7797e passed. Every failure is the 10 s `wait_until` (`WAIT`) inside
`open_folder` timing out with the status `analyzing 3/4`.

The cause is the cost of pass 2 in the dev / test profile, not a hang:

- `wait_idle` waits for both scan passes. Pass 2 (`run_faces_scan`: YuNet and
  the MediaPipe face mesh through tract) runs unoptimized in the test
  profile. One test alone, locally: pass 2 of 4 files took 1775 ms on 24
  cores with 3 pass 2 threads.
- On a 4-core CI runner `faces_threads()` (`crates/app/src/scan.rs`) gives one
  pass 2 thread, while about ten UI tests open folders at once in the same
  test binary. This is the only UI test with 4 files, so it alone crosses
  10 s. The ubuntu failure log shows the watcher's `resync ... deferred` lines
  about 2 s apart, one per pass 2 file read.
- #1228 (d6e1b4ce) added three more concurrent UI tests, which raised the
  contention; that is the link to the commit the failures started around.
- Instrumenting `ReadSlots` showed no `PaintHold` pause waits, so nothing was
  held.
- Pinned to 4 cores, every UI test's folder wait was 5.0 to 7.3 s before the
  change.

Once this is merged, the tract inference crates are compiled optimized in the
dev profile, the test's wait has a wide margin on a 4-core runner, and `main`'s
CI stops failing on it. The 10 s wait is not extended: the fix removes the
tenfold debug-build inference cost instead of widening the wait.

## Steps

- [x] Step 1: Compile the tract inference crates optimized in the app's dev
      profile, and record the cause
  - Done when:
    - `crates/app/Cargo.toml`'s `[profile.dev.package]` holds `opt-level = 3`
      for `tract-core`, `tract-data`, `tract-extra`, `tract-hir`,
      `tract-linalg`, `tract-nnef`, `tract-onnx`, `tract-onnx-opl`, `ndarray`
      and `matrixmultiply` under a comment giving the reason, and the change
      is committed
    - That comment does not say the UI tests' pass 2 runs below normal
      priority (see the implementation approach)
    - `WAIT` in `crates/app/src/uitest.rs` is still 10 s, and no UI test is
      changed
    - `cargo test --bin handpic a_filter_choice_narrows` in `crates/app`
      passes 10 runs in a row
    - The full app test binary passes under a 4-core affinity pin
    - `docs/agents/app.md` names the tract entries where it describes the
      `[profile.dev.package]` block, and its UI test rules record this cause
    - `learnings.md` in this folder records the cause and the measurements
      below
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - The `Cargo.toml` change is already in the working tree, uncommitted. It
      follows the precedent of the block's existing `image` / `png` /
      `fdeflate` / `flate2` / `miniz_oxide` entries for the `screenshot`
      feature: a comment with the reason, then the entries. All ten package
      names are in `crates/app/Cargo.lock` (tract 0.23.10, `ndarray` 0.17.2,
      `matrixmultiply` 0.3.11), so none is an unmatched profile spec.
    - Correct the working tree's comment before committing. It reads "one pass
      2 thread each below normal priority", but `Ui::open` sets the view's
      `scan_priorities` to `Priority::Normal` for both passes
      (`crates/app/src/uitest.rs`), and `docs/agents/app.md` has a rule saying
      so. The facts to keep in the comment: pass 2's YuNet and face mesh
      inference is about ten times slower unoptimized, a 4-core CI runner gives
      each UI test's scan one pass 2 thread, and the concurrent UI tests then
      come near the tests' 10 s wait.
    - `docs/agents/app.md` changes in two places, and no other doc mentions
      the dev profile's overrides (`CLAUDE.md` and
      `docs/agents/tract-onnx-inference.md` do not):
      - The paragraph beginning "`crates/app/Cargo.toml` carries the guide's
        `[profile.dev.package]` block" lists the block's crates and gives
        rendering speed as its reason. Add that the block also holds the tract
        inference crates and why. The list there already omits the `image` /
        `png` group and `crc32fast`; leave that as it is.
      - The section "The main view opens on GPUI's test platform (Measured)"
        holds the UI test rules, each recording an earlier timeout and ending
        "Do not raise the wait instead". Add a rule in the same form: the
        tract crates are optimized in the dev profile because unoptimized
        pass 2 took the 4-file test past 10 s on a 4-core runner, with the
        before and after numbers.
    - Measurements to record in `learnings.md` (and, in short, in the
      `app.md` rule):
      - Before: one test alone, pass 2 of 4 files 1775 ms (24 cores, 3 pass 2
        threads); pinned to 4 cores, every UI test's folder wait 5.0 to 7.3 s
      - After: one test alone, pass 2 182 ms; maximum folder wait 0.82 s
        pinned to 4 cores and 1.19 s pinned to 2 cores; the full app test
        binary 1095 / 1095 passed, 3 runs at 4 cores and 3 runs at 2 cores
      - The CI history above, the `analyzing 3/4` status, the roughly 2 s
        spacing of the `resync ... deferred` lines, and the `ReadSlots`
        instrumentation that ruled out a hang
    - Re-run the two test checks of "Done when" on the committed tree rather
      than relying on the numbers above, and add those runs to `learnings.md`.
    - Out of scope: any other UI test, the `WAIT` value, `faces_threads`, and
      `todo.md`'s real-device items.

## Trade-offs and risks

- The working tree's `Cargo.toml` comment says the UI tests' pass 2 runs below
  normal priority; the code and `docs/agents/app.md` say normal. This plan
  corrects the comment. The alternative is to commit the diff verbatim, which
  leaves a comment that contradicts the rule next to the tests.
- `docs/agents/app.md`'s description of the `[profile.dev.package]` block is
  already incomplete (no `image` / `png` group, no `crc32fast`). This plan
  adds only the tract sentence. Rewriting the list in full would make it
  accurate but goes beyond this fix.
- Not taken: raising `WAIT` (an existing rule in `docs/agents/app.md` forbids
  it, and it would hide the cost rather than remove it), giving the tests more
  pass 2 threads, or reducing this test's file count (no other UI test change
  is allowed here).
- The fix buys margin, it does not remove the contention. With one pass 2
  thread on a 4-core runner and every UI test scanning at once, a much slower
  runner or many more concurrent UI tests could bring the wait back toward
  10 s. The measured maximum is 0.82 s at 4 cores.
- The ten crates now compile at `opt-level = 3` in the dev profile, so a cold
  dev or test build of `crates/app` takes longer. The increase was not
  measured.
- A `cargo run` debug build of the app also gets the faster pass 2. That is a
  side effect, not a goal.

## Progress

- 2026-11-22: Step 1 done. The ten tract / ndarray / matrixmultiply crates are
  compiled at `opt-level = 3` in `crates/app`'s dev profile, and the docs are
  updated. PR reference: to be added once the PR exists.
