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

# Raise the UI tests' wait past the 4-file scan under load

## Purpose

`uitest::a_filter_choice_narrows_the_files_and_lands_on_the_anchor`
(`crates/app/src/uitest.rs`) fails intermittently on `main` at the `WAIT`
limit of 10 s with `scan Some(("analyzing", 3, 4)), running false`. On `main`,
d6e1b4ce failed on ubuntu, accb630d and 86bb0180 failed on windows, and
6ce7797e passed.

The cause is measured and is load, not app logic:

- This test is the only UI test that opens a 4-file folder. Pass 2 runs 3
  workers, so the 4th file starts only after one of the first three ends.
- In a debug build each tiny DNG takes 0.75 to 1.3 s of compute. The open
  takes about 7.6 to 8.9 s when the suite runs on 2 cores, which leaves 1 to
  2 s of margin under 10 s.
- The scanning UI tests added in #1228 and #1226 raised the concurrent load
  enough to cross 10 s often.
- `ReadSlots::acquire` and `yield_to_pause` wait under 20 ms. #1228's prefetch
  guard does not run during this open (`scanning()` is checked first).
- Pinned to 2 cores, `main` failed 3 runs of 9. With `WAIT` at 30 s it passed
  16 of 16, each run 11.0 to 11.9 s.

Once this is done, the UI tests no longer fail on a loaded 2-core runner, and
the reason the limit is 30 s is recorded next to the constant and in
`docs/agents/app.md`.

Decided with the user (2026-11-22): raise `WAIT` to 30 s and keep the
`docs/agents/app.md` rules against raising the wait, recording this one
measured exception (option (a) below). `remove_within(&dir, WAIT)` in `Ui`'s
drop follows `WAIT` to 30 s (the smallest diff); update its doc sentence.

## Steps

- [x] Step 1: Raise the UI tests' `WAIT` to 30 s and record why
  - Done when:
    - `WAIT` in `crates/app/src/uitest.rs` is 30 s, with a comment that says
      why: the measured 4-file pass 2 in a debug build under the suite's
      parallel load on 2 cores, and that it is only the failure deadline, so a
      passing test is not slowed.
    - `docs/agents/app.md` gives the new limit where it describes
      `wait_until` ("failing after 10 s") and the drop's removal deadline
      ("still held after 10 s"), and no longer contradicts the change in its
      two rules ("not by raising the 10 s wait", "Do not raise the wait
      instead"). The rules stay, with the measured exception recorded.
    - `learnings.md` in this plan folder records the diagnosis (the numbers
      under Purpose, the failing commits, and the rejected alternatives).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass on
      every platform.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/uitest.rs` (the `WAIT` constant and its doc
      comment only), `docs/agents/app.md` (the "failing after 10 s" sentence,
      the "still held after 10 s" sentence and the two rules in the UI test
      section), this folder's `plan.md` and `learnings.md`.
    - `WAIT` has two uses: `wait_until`'s deadline and
      `remove_within(&dir, WAIT)` in `Ui`'s drop. Both follow the new value.
    - Do not change the sentence "one test's pass 2 had done 3 of 4 files
      after 10 s" in `app.md`: it is a past measurement, not the limit.
    - The comment in `Ui::open_with` ("starves past `WAIT`") names no number
      and needs no change.
    - Keep the rule that a timeout is diagnosed from its log lines first.
      State in `app.md` that the limit was raised once, after such a
      diagnosis, with the measured numbers, so that a later timeout is not
      answered by raising it again without one. Write the numbers as
      measured.
    - Do not touch the test itself, the number of files it opens, or the
      scan. `QUIT_MAX` stays at 1 s.
    - Commit type: `test(app): ...`.

## Trade-offs and risks

- **`docs/agents/app.md` forbade this fix as written.** Options were (a) keep
  both rules and add the measured exception (chosen), or (b) reduce the load
  instead: fewer files in this test (the 4th file is what makes a worker hand
  over, so coverage changes) or serialized UI tests (slower suite).
- **The drop's folder-removal deadline shares `WAIT`** and rises to 30 s with
  it. A folder held after the quit then takes 30 s to fail instead of 10 s;
  removal normally takes about 11 ms.
- **A real hang now takes 30 s to fail instead of 10 s**, per timed-out test.
  A panicking `Ui` cancels its scan, so timed-out tests do not pile load onto
  the later ones.
- **30 s is sized from 2-core measurements.** More scanning UI tests will keep
  raising the load; if a timeout returns, the log lines decide, not a further
  raise.
- **The macOS runner was not part of the measurement.** The PR's checks are
  the only evidence for the other platforms.

## Progress

- 2026-11-22: Step 1 done. `WAIT` in `crates/app/src/uitest.rs` raised to 30 s, the two `docs/agents/app.md` sentences and two rules updated, and the diagnosis recorded in `learnings.md`.
