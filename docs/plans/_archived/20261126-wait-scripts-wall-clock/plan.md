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

# Wall-clock wait slices

## Purpose

`wait-pr-actionable.sh` and `wait-post-merge-runs.sh` bound one wait (a
slice) by the sum of their sleeps. `gh` latency (13-46 s a call, measured on
2026-11-23) adds on top, so a slice's real length and the total wait that
`pr-runner` and `merger` count in slices depend on the network: about 20-35
minutes on a slow one, less on a fast one. `wait-post-merge-runs.sh` also
keeps polling through `INITIAL_GRACE` whatever `--max-wait` says while no run
has appeared, which brings one slice close to the harness's 180 s
backgrounding limit.

Once this is done, each script bounds a slice by real elapsed time, the
no-runs path stays inside `--max-wait`, the slice-count caps in
`pr-runner.md` and `merger.md` give about 30 minutes whatever the network
speed, and the two items are gone from `todo.md`. Background:
[the agent-workflow-fixes plan](../_archived/20261123-agent-workflow-fixes/plan.md)
and its `learnings.md`.

## Steps

- [x] Step 1: Bound the wait scripts' slices by real elapsed time and close
      the two todo items
  - Done when:
    - `wait-pr-actionable.sh`: `max_wait_sec` bounds the slice's real elapsed
      time, the time spent inside `pr-status.sh` included. A slice ends at
      `max_wait_sec`, plus at most the one call in flight when the deadline
      passes. `BLOCKED_GRACE` is counted on the same clock.
    - `wait-post-merge-runs.sh`: `--max-wait` / `MAX_WAIT` bounds the slice
      the same way, and the no-runs path returns within it too.
    - Unchanged in both scripts: every exit code and its meaning, the
      `ACTION=` values, the `STATUS=` values, the stdout shape (the last
      observed state on a timeout, the table and the counts), the argument
      and environment variable names and their defaults.
    - `merger.md` step 5's exit-code list still reads exit 0 as
      `STATUS=success` or `STATUS=no_runs` and goes to step 6 on both.
    - `pr-runner.md` and `merger.md` pass slice arguments and hold
      slice-count caps under which each total wait is about 30 minutes on a
      fast and a slow network alike, with the arithmetic written next to the
      cap. Every sentence that describes the slice as a sum of sleeps or
      gives a latency-dependent range ("70-150 s", "about 20-35 minutes",
      "about 15-35 minutes, by the slice's 40-100 s", "30-second slices") is
      rewritten to the new definition.
    - The scripts' header comments describe the new bound.
    - `wait-pr-actionable.test.sh` and `wait-post-merge-runs.test.sh` each
      have cases that fail on the old sum-of-sleeps code and pass on the new:
      call latency counts toward the bound, and (merge script) the no-runs
      path ends within `--max-wait`. The existing cases still pin the same
      exit codes and `ACTION=` / `STATUS=` lines. No case's outcome depends
      on how fast the host runs a stubbed call.
    - The two headings "Agents: `wait-post-merge-runs.sh`'s no-runs path
      ignores `--max-wait`" and "Agents: the wait scripts count the sum of
      sleeps, not real elapsed time" are gone from `todo.md`, each from its
      heading through its last line, and nothing else in the file changed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `.claude/skills/pr/scripts/wait-pr-actionable.sh`,
      `.claude/skills/pr/scripts/wait-pr-actionable.test.sh`,
      `.claude/skills/merge/scripts/wait-post-merge-runs.sh`,
      `.claude/skills/merge/scripts/wait-post-merge-runs.test.sh`,
      `.claude/agents/pr-runner.md` (section 4), `.claude/agents/merger.md`
      (steps 1 and 5), `todo.md`. Both tests already run from
      `mise run lint` and are selected by `tools/ci/local.sh` when their
      script or test changes; no wiring change is needed. The scripts are
      shellchecked by `mise run lint`.
    - Current mechanics. Both scripts keep `elapsed` and add the interval
      after each `sleep`. In `wait-pr-actionable.sh` the same `elapsed` also
      measures the `blocked` grace (`blocked_since`) and appears in the
      `[elapsed Ns]` stderr lines. In `wait-post-merge-runs.sh` it also
      gates the no-runs path against `INITIAL_GRACE`, with no check against
      `max_wait` on that path.
    - Slice shape (see "Trade-offs and risks", decision 3). Treat
      `max_wait` as a deadline: after each call, if the clock is at or past
      it, time out; otherwise sleep `min(interval, time left)`; if the
      deadline is reached at the end of that sleep, time out with the last
      observed state and make no further call. The caller re-runs at once
      and the next slice starts with a call, so nothing is lost, and the
      slice is `max_wait` long unless a call straddles the deadline. Replacing
      `elapsed += interval` with a clock read alone is not enough: with an
      unclamped sleep and a final call, a slice still runs to
      `max_wait + interval + one call`.
    - `wait-pr-actionable.sh` rules to keep on the new clock: a slice in
      which every call failed exits 1; a slice that ends after a failed call
      with an earlier success exits 2 with the last successful output; a
      failed call neither resets nor ends the `blocked` grace; any other
      ACTION resets it; `max_wait_sec=0` makes exactly one call. Keep the
      timeout messages' `${MAX_WAIT}s` wording (the tests pin
      `timed out after 0s (last ACTION=blocked)` and `failed on every call
      for 2s`).
    - `BLOCKED_GRACE` constraint. The grace restarts on every call of the
      script, so one slice must hold two observations at least 60 s apart.
      Under a deadline, today's `60 60` makes one call and can never meet
      the grace: a lasting `blocked` would end in `wait_timeout` instead of
      exit 0. The archived learnings record the same failure for `30 30`.
      The slice passed by the agents must therefore be longer than the
      interval plus one call.
    - Arguments and caps (derived, to verify). `60 120` for
      `wait-pr-actionable.sh` in both agent files: calls at 0 and at
      `c1 + 60`, the grace met on the second whenever the first call takes
      under 60 s; the slice is 120 s, or up to `c1 + 60 + c2` when the
      second call straddles (152 s at the measured 46 s worst case, under
      180 s while a call stays under 60 s). Caps: `wait_timeouts < 15` and
      `pr_wait_timeouts < 15` stay (15 x 120 s = 30 minutes);
      `review_required_timeouts` `< 4` / `>= 4` stays and its note becomes
      about 8 minutes stalled. `--max-wait=120` for
      `wait-post-merge-runs.sh`, with `post_merge_timeouts < 15` (was 20;
      15 x 120 s = 30 minutes). Re-read what one exit 1 stands for
      (`pr_status_failures > 5`: six slices of 120 s) and reword it. Measure
      one real slice of each script on the host and record the wall time in
      `learnings.md`; change the numbers if a slice does not land clearly
      under 180 s.
    - No-runs path (see decision 2). Effective grace =
      `min(INITIAL_GRACE, max_wait)`, counted on the same clock, clamping
      the sleep to the time left in the grace; at its end with zero runs
      print `STATUS=no_runs` and exit 0 as today. With `--max-wait=120` the
      grace stays 60 s of real time, so `merger` sees `no_runs` after about
      60 s plus one call instead of three calls (up to about 170 s). The
      existing "no runs" case (`INITIAL_GRACE=0 --max-wait=0`) must keep
      passing.
    - Clock and tests (see decision 1). Host fact: the archived learnings
      record `wait-pr-actionable.test.sh` (9 cases, 1 s sleeps) taking
      7 min 51 s on the Windows host, so one stubbed `pr-status.sh` call
      takes many seconds there. Under a real clock, the cases with
      `max_wait` 2 ("fails on every call of the slice", "fails at the end
      after a success", "a failed call keeps the blocked grace") would end
      after one call on this host, and every pinned `[elapsed 1s]` line
      would vary. Approach: read the clock as `date +%s` in both
      scripts (kept to one small function) and, in the tests, put `date` and
      `sleep` stubs next to the `gh` stub on `PATH`: a clock file that
      `date +%s` prints, that `sleep N` advances by N without sleeping, and
      that the `gh` stub advances by a per-case latency. That makes every
      case deterministic and removes the real sleeps. `pr-status.sh` calls
      neither `date` nor `sleep`, so the stubs reach only the script under
      test. Check that the `date` stub does not break anything else the
      scripts or `pr-status.sh` spawn.
    - New cases, at least: (pr) a latency that alone exceeds `max_wait`
      ends the slice after one call with exit 2; (pr) with a `60 120`-like
      ratio and latency, `blocked` is returned with exit 0 on the second
      call; (pr) with a `60 60`-like ratio the slice exits 2 with
      `ACTION=blocked` after one call (pins the constraint above); (merge)
      zero runs with `INITIAL_GRACE` above `--max-wait` exits 0 with
      `STATUS=no_runs` within `--max-wait`; (merge) pending runs with call
      latency exit 2 with `STATUS=timeout` at the deadline, counting the
      calls made. The merge test's `gh` stub currently prints fixtures from
      environment variables and counts nothing; extend it as far as these
      cases need.
    - Agent definitions. Do not touch the "run exactly as written, as a
      standalone Bash call" rules, the `timeout: 600000` rule, the
      backgrounding rules or the exit-code branching; change only the
      arguments, the caps, and the sentences that explain a slice's length
      and the totals. `merger.md` step 5 has three places naming the slice:
      the command block, the paragraph under it (which also describes the
      three-call no-runs slice) and "Do not drop `--max-wait=30` ... in
      30-second slices". `wait-pr-actionable.sh`'s header names `60 60` and
      "the sum of the sleeps"; `wait-post-merge-runs.sh`'s header names
      `--max-wait=30`.
    - Compatibility. Keep the scripts' defaults (`max_wait_sec=240`,
      `MAX_WAIT=1800`, `INITIAL_GRACE=60`, `BLOCKED_GRACE=60`) and rename
      nothing, as the agent-workflow-fixes plan did.
    - `todo.md`: delete exactly the two items, heading through the last line
      before the next `###` heading ("Repo: post-merge check that GitHub
      detects the MIT OR Apache-2.0 license"). Re-read the file right before
      editing; it is edited concurrently and the line numbers move.

## Trade-offs and risks

- Decision 1, clock source and test style (decided: `date +%s` with stubs).
  - `date +%s` with `date` / `sleep` stubs on `PATH`: deterministic and fast
    tests with no test-only switch in the scripts, at the cost of one `date`
    process per clock read.
  - `SECONDS` with real sleeps and stub latency: no extra process, but the
    tests depend on host speed (see the 7 min 51 s measurement), need wide
    margins, and get slower.
  - `SECONDS` plus a test-only clock override in the script: deterministic,
    but puts a test seam in production code.
- Decision 2, what "honors `--max-wait`" means on the no-runs path (decided:
  cap the grace at `max_wait`, still exit 0 with `STATUS=no_runs`). Exit
  codes and `merger.md`'s reading stay as they are. A caller passing a
  `--max-wait` under 60 gets a shorter grace, and a run GitHub creates late
  would be missed as `no_runs`; `merger` passes 120, so its grace stays 60 s.
  Exiting 2 inside the grace was rejected: the grace restarts on every call,
  so every docs-only merge would end in `POST_MERGE_TIMEOUT`.
- Decision 3, slice arguments and caps (decided: `60 120` /
  `--max-wait=120`, caps of 15). Derived from the 2026-11-23 latency
  measurements, not measured under the new semantics. A shorter slice
  (`60 90`) leaves more room under 180 s but cannot meet `BLOCKED_GRACE`
  when the first call takes 30 s or more, which was measured.
- Transitional mismatch. A session whose agent definition still says `60 60`
  / `--max-wait=30` but whose working tree has the new scripts gets one call
  per `60 60` slice, so a lasting `blocked` ends in `wait_timeout` after about
  15 minutes instead of `blocked`; a 30 s no-runs grace; and total waits of
  about 15 and 10 minutes. Scripts and definitions normally come from the
  same checkout, so the window is short.
- Not bounded: a single `gh` call that hangs longer than about 60 s still
  pushes a slice past 180 s. A per-call timeout is out of scope.
- The totals are slices times `max_wait`. The agent's turn time between
  slices adds a few seconds each and is not counted.

## Progress

- (2026-11-26) Step 1 complete
