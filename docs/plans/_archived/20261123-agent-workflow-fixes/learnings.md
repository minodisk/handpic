# Learnings: agent-workflow-fixes

## Step 1

- The harness's backgrounding was observed twice during this step, on plain
  foreground Bash calls (not `ci:local`):
  - With the default timeout: "Command did not complete within its 120s
    timeout and was moved to the background (ID: ...). Output is being written
    to: <file>. You will be notified when it completes. It is running now: do
    not start a second copy while it runs."
  - With `timeout: 600000`: "Command was moved to the background (ID: ...)
    after running for 180s. Output is being written to: <file>. You will be
    notified when it completes. ..."
  - The result reached this leaf subagent as a `<task-notification>` message
    with the status (`completed` / `failed`), the exit code in its summary
    ("failed with exit code 2") and the output file's path; the output itself
    is read from that file. The rule added to `implementer.md` and
    `local-review-addresser.md` follows that shape.
- `api.github.com` was slow from this host on 2026-11-23: `curl` of
  `/zen` took about 4 s, a trivial `gh api rate_limit` 13 s, one
  `pr-status.sh` call on an open PR 28-46 s and one `gh run list --commit`
  20-36 s. The wait slices were sized against those worst cases, not against
  the few seconds a call usually takes:
  - `wait-pr-actionable.sh <PR> 30 30` measured 98 s of wall time (two calls),
    but cannot honor the 60 s `BLOCKED_GRACE`: each run starts the grace from
    0, so with 30 s of sleeps a lasting `blocked` would never return exit 0 and
    would end in `wait_timeout` instead of `aborted` (`blocked`). The agents
    therefore pass `60 60` (two calls 60 s apart, the grace met on the second),
    about 60 s + 2 calls = 70-150 s.
  - `wait-post-merge-runs.sh --max-wait=30` is two calls 30 s apart (40-100 s)
    while runs exist. While no run has appeared yet, `INITIAL_GRACE` (60 s)
    is not bounded by `--max-wait`, so that path takes three calls, up to about
    170 s at the slow measurement; it then exits 0 (`no_runs`) by itself.
  - The slice-count limits were rescaled against those wall times:
    `wait_timeouts` / `pr_wait_timeouts` `< 15`, `review_required_timeouts`
    `>= 4`, `post_merge_timeouts < 20`.
- `wait-pr-actionable.test.sh` (9 cases) took 7 min 51 s of wall time on this
  host (about 7 s of CPU), so a direct run of it is moved to the background
  too.
- A failed `pr-status.sh` call keeps `blocked_since` (it is not evidence of
  another state), pinned by the test "a failed call keeps the blocked grace".
- `python3` in Git Bash on this host resolved and ran (3.13.15), as did
  `mise x -- python`.

## Deferred issues (todo candidates)

- `wait-post-merge-runs.sh`'s no-runs path ignores `--max-wait`: while no run
  has appeared, it keeps polling until `INITIAL_GRACE` (60 s) whatever the
  slice, so with a slow `api.github.com` (about 36 s a `gh run list` call) one
  slice can take about 170 s, close to the harness's 180 s backgrounding.
  Basis: this step's slice measurement (see above). Files:
  `.claude/skills/merge/scripts/wait-post-merge-runs.sh`,
  `.claude/agents/merger.md` (step 5).
- The slices' wall time is dominated by `gh` latency, so the rescaled limits
  give about 20-35 minutes of total wait on a slow network and less on a fast
  one. A wall-clock bound inside the scripts (counting real elapsed time, not
  the sum of sleeps) would make both the slice and the totals predictable;
  that changes the scripts' semantics and was left out to keep runs holding
  the older definitions unaffected. Basis: this step's measurements. Files:
  `.claude/skills/pr/scripts/wait-pr-actionable.sh`,
  `.claude/skills/merge/scripts/wait-post-merge-runs.sh`,
  `.claude/agents/pr-runner.md`, `.claude/agents/merger.md`.
