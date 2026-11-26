<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Close the stale todo item on the cancel test's timing bound

## Purpose

`todo.md` carries the item "Core: load-sensitive timing bound in
`scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit`", which says
the test asserts a cancel ends within 1 s and fails at 1.10 to 1.14 s under
full-suite load.

The item is stale. The test was already rewritten by `6a5e5dfc` (#1269,
2026-11-24 08:36, "test: remove the timing dependence of the scan tests"; plan:
[todo-flaky-scan-tests](../_archived/20261123-todo-flaky-scan-tests/plan.md)).
At HEAD the test in `crates/core/src/scan.rs` has no wall-clock assertion. It
waits on an `mpsc` channel that the scan thread's end disconnects, bounded only
by the hang guard `HANG` (30 s, under `LONG_PAUSE`), drops the pause or the
permit, asserts, then joins. This is the shape
[shared-crates.md](../../agents/shared-crates.md) ("A cancel or stop test must
not bound the scan by wall-clock time") prescribes.

The two failing runs (`took 1.14s` / `took 1.10s`) were recorded in
[sidecar-and-keymap-decisions' learnings](../_archived/20261123-sidecar-and-keymap-decisions/learnings.md)
on 2026-11-23, in `5ea303ce` (#1254), against the old code. The `took` message
they quote no longer exists in the test. The wrap-up of that plan (`8f8271a6`,
#1283, 2026-11-24 23:15) filed the item from that pre-fix note, after #1269 had
already removed the original item for the same test.

Why the test takes about 0.57 s alone: it is not a production defect. That is
the whole test's runtime. Its two iterations (pause, permit) each sleep `SETTLE`
(150 ms) before canceling, each cancel is noticed within one `CANCEL_TICK`
(50 ms, the `Condvar::wait_timeout` poll in `ReadSlots::acquire` and
`wait_unpaused_since`, needed because the cancel flag is a plain `AtomicBool`
that cannot wake a waiter), and the rest is fixture writes, thread spawns and
the directory removal. The old 1 s bound measured only cancel to join, nominally
50 ms or less; the 1.1 s under load was scheduler starvation of the woken
workers, not the poll interval. No production change is called for.

Once this is done, `todo.md` no longer lists work that is already complete.

## Steps

- [x] Step 1: Delete the stale todo.md item
  - Done when:
    - The whole section "### Core: load-sensitive timing bound in
      `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit`"
      (heading, paragraph, `#### TODO` and its checkbox) is gone from `todo.md`,
      and no other item is touched.
    - `crates/core/src/scan.rs` is unchanged: the test at HEAD already waits on
      an event (`ended.recv_timeout(HANG)`), which the PR description states
      with the commit that did it (`6a5e5dfc`, #1269).
    - The PR description cites the evidence that the test does not fail under
      load: 30 of 30 runs under 48 busy loops after the change, and a pass in
      every whole-module log that reached it (todo-flaky-scan-tests'
      `learnings.md`, Step 1).
    - The PR's GitHub checks pass.
  - Implementation approach:
    - Files: `todo.md` only (plus this plan folder). Docs-only change;
      Conventional Commit such as
      `docs: drop the stale todo on the cancel test's timing bound`.
    - Before deleting, confirm that the test body at HEAD still has no
      `Instant` / `elapsed` assertion, in case main moved.
    - Do not edit `docs/agents/shared-crates.md` or the archived learnings that
      mention the old failure.

## Trade-offs and risks

- Re-verify under load, or rely on the recorded evidence: the user chose to
  rely on the recorded 30 of 30 under 48 busy loops plus the structural fact
  that the test has no bound short of the 30 s hang guard; the PR's GitHub
  checks run the core suite once more.
- Neighbouring tests still carry wall-clock upper bounds (`< 1 s` / `< 2 s`) in
  `crates/core/src/scan.rs`'s `tests`. None wait on worker threads being
  scheduled after a 50 ms tick the way the old cancel test did, and none has a
  recorded failure. Left alone.
- A residual, non-flaky weakness: if the workers have not reached the gate
  within `SETTLE` (150 ms), the cancel is seen at the first `acquire` check, not
  while waiting. The test still passes; it only exercises less on that run.
  Left as is.

## Progress

- (2026-11-26) Step 1 complete (stale todo.md item removed; the test already waits on ended.recv_timeout(HANG) since 6a5e5dfc, #1269)
