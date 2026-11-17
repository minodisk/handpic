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

# Stop the false `blocked` abort of wait-pr-actionable.sh

## Purpose

The pr-runner agent aborted PRs #1123, #1131, #1144 and #1148 right after
creating them: `wait-pr-actionable.sh` returned `ACTION=blocked` with
`mergeStateStatus=BLOCKED`, `checksOverall=success`, `failedCount=0` and
`pendingCount=0`, although CI had only just started. A later re-dispatch
reached `ready` with no change.

Investigation result (the cause is not the classification of running checks):

- `pr-status.sh`, which `wait-pr-actionable.sh` calls for every decision,
  already maps a check run that is not `COMPLETED` to its `status` and counts
  `QUEUED`, `IN_PROGRESS`, `PENDING`, `WAITING` and `EXPECTED` as pending. Run
  against PR #1148 with five checks `IN_PROGRESS` (empty `conclusion`) it
  returned `ACTION=wait`, `pendingCount=5`. Only `REQUESTED` is missing from
  its `pending_selector`.
- The branch protection of `main` requires one context, `ci`: the aggregate
  job of `.github/workflows/ci.yml` (`needs: [lint, test, native]`,
  `if: always()`). Its check run does not exist until the other seven jobs
  have finished.
- So there are two windows in which the PR is `BLOCKED` (the required `ci`
  has not reported) while the rollup holds nothing pending and nothing failed:
  1. right after the PR is created, before any check run is registered (the
     rollup is empty; on #1144 the PR was created at 14:45:04 and the first
     check started at 14:45:10). This is the window the four aborts hit;
  2. between the seventh job finishing and the `ci` check run appearing (on
     #1144, 14:59:16 to 14:59:19).
  In both, `pr-status.sh` falls through to `mergeStateStatus == BLOCKED` and
  returns `blocked`, which `wait-pr-actionable.sh` treats as terminal. The
  `statusCheckRollup` read that showed seven `IN_PROGRESS` checks was
  pr-runner's follow-up query, made seconds after the script's snapshot.

Once this is done, `wait-pr-actionable.sh` keeps waiting through both windows
and through every non-completed check status, and returns `blocked` only when
it stands with every check concluded.

## Steps

- [x] Step 1: Make `wait-pr-actionable.sh` return `blocked` only when it
      persists, count `REQUESTED` as pending, and cover both with a bash test
  - Done when:
    - A `blocked` observation no longer ends the wait at once:
      `wait-pr-actionable.sh` keeps polling and returns `ACTION=blocked`
      (exit 0) only after `blocked` has been observed continuously for the
      grace period; any other ACTION observed meanwhile is handled as it is
      today (a `wait` keeps waiting and resets the grace, a terminal ACTION
      returns)
    - Check runs with status `QUEUED`, `IN_PROGRESS`, `WAITING`, `PENDING` or
      `REQUESTED` and status contexts in state `PENDING` or `EXPECTED` all
      count toward `pendingCount` in `pr-status.sh`'s output, so the ACTION is
      `wait`
    - A PR that stays `BLOCKED` with every check concluded and none failed
      (for example a required check that never reports, or a protection rule)
      still returns `ACTION=blocked` with exit 0
    - A new bash test, `.claude/skills/pr/scripts/wait-pr-actionable.test.sh`,
      covers at least: (a) every non-completed status above counted as
      pending; (b) the regression, an empty rollup with `BLOCKED` followed by
      running checks and then a clean PR, ending in `ready`, not `blocked`;
      (c) the seven jobs concluded without `ci`, then `ci` reporting, ending
      in `ready`; (d) a persistent `BLOCKED` with every check concluded ending
      in `blocked`
    - The test runs in `mise run lint` (so in `mise run ci`) and in
      `mise run ci:local` when either script or the test changes
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `.claude/skills/pr/scripts/wait-pr-actionable.sh` (the grace
      logic and its header comment, which lists the ACTIONs that end the wait
      and the exit codes), `.claude/skills/pr/scripts/pr-status.sh` (add
      `REQUESTED` to `pending_selector` only; do not restructure the
      `normalize` filter or the ACTION ordering), the new
      `.claude/skills/pr/scripts/wait-pr-actionable.test.sh`, and the test
      wiring: `mise.toml` (`[tasks.lint]`, next to the existing
      `bash ...test.sh` lines), `tools/ci/local.sh` (the `bash_tests` array
      and the `case "${path}"` block that maps changed paths to tests; map
      `wait-pr-actionable.sh`, `pr-status.sh` and the test itself to the new
      test) and `tools/ci/local.test.sh` (the `every` list and an `expect`
      case like the existing ones under `# --- bash tests`)
    - Keep `pr-status.sh` a one-shot snapshot: the persistence rule belongs in
      `wait-pr-actionable.sh`'s existing loop. Suggested shape: on
      `ACTION=blocked`, remember the elapsed time of the first consecutive
      `blocked`; return it once `elapsed - first >= grace`; otherwise sleep
      `INTERVAL` and poll again; any non-`blocked` observation clears the
      marker. The grace sleeps count toward `max_wait_sec`, so the script's
      "always returns within the Bash tool's 600 s" property holds; a timeout
      during the grace exits 2 with the last output, as for `wait`
    - Grace length: default about 60 s (the observed windows are about 6 s and
      3 s; two more polls at the default 30 s interval). Make it overridable
      by an environment variable so the test does not sleep for real, the way
      `wait-post-merge-runs.sh` takes `POLL_INTERVAL` / `INITIAL_GRACE`. Do
      not add a fourth positional argument: `pr-runner.md` and `merger.md`
      call the script with the PR number only, and their call lines must not
      change
    - Test convention: follow
      `.claude/skills/merge/scripts/wait-post-merge-runs.test.sh` (a `gh` stub
      placed first on `PATH`, an `expect` helper pinning the exit code and
      output lines, a final `N cases passed` line, failure exit 1). The stub
      here must answer successive `gh pr view ... --json ...` calls with
      successive fixtures (a counter file in the temp dir, the last fixture
      repeating), because the regression cases are sequences. Fixture JSON
      needs `state`, `mergeable`, `mergeStateStatus`, `reviewDecision`,
      `headRefOid`, `isDraft`, `number`, `url` and `statusCheckRollup` with
      `__typename` on each entry (`CheckRun`: `status`, `conclusion`, `name`,
      `detailsUrl`; `StatusContext`: `state`, `context`, `targetUrl`). Run the
      script with `interval_sec=1` and a grace of a second or two so the whole
      test stays within a few seconds
    - The new test file is matched by the `shellcheck -x
      .claude/skills/*/scripts/*.sh` glob of `mise run lint`; keep it clean
      under the repository's `.shellcheckrc`
    - Do not edit `.claude/agents/pr-runner.md` or `.claude/agents/merger.md`.
      Verify by reading that their exit-2 handling needs no change: both treat
      a timeout whose last ACTION is not one of their special cases as
      ordinary waiting and re-run, which is right for a timeout that lands
      inside the grace
    - Do not touch PR #1148

## Trade-offs and risks

- **Where the fix lives, and which fix (decided: the grace period).** The
  request named `wait-pr-actionable.sh` and assumed running checks were not
  counted as pending; they are. Two ways to close the real gap:
  - *Grace period in `wait-pr-actionable.sh` (chosen by the user).* Repository
    agnostic, no extra API call, stays in the named file, and a persistent
    `BLOCKED` still ends as `blocked`, so pr-runner's behavior is unchanged
    apart from the delay. Cost: a real `blocked` is reported one grace period
    later (about 60 s), and the grace length is a guess; if GitHub takes
    longer than the grace to register check runs, the false abort returns.
  - *Required-check awareness in `pr-status.sh` (not taken).* Read the base
    branch's required contexts and count a required context absent from the
    rollup as pending. Exact, but it does not see rulesets, and an Actions
    outage would end as a ~35-minute timeout instead of `aborted(blocked)`.
- **`pr-status.sh` is edited although the request listed only
  `wait-pr-actionable.sh`.** The acceptance criterion on non-completed
  statuses can only be met there, and the edit is one word (`REQUESTED`).
- **`blocked` can now be the last ACTION of an exit 2.** `pr-runner.md` says
  only `wait` / `review_required` are observed on a timeout. The existing
  exit-2 rules still handle it (ordinary waiting, re-run), and the file is out
  of scope, so its sentence is left slightly stale.
- **merger also calls the script.** It returns `NOT_READY` on `blocked`; with
  the grace it gets that answer about a minute later, and no longer gets it
  falsely in the second window.

## Progress
