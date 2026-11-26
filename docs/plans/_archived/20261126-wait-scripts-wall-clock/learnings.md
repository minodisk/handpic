# Learnings

## Step 1

- Real slices on the Windows host (2026-11-26, `api.github.com` fast that
  day, calls of 2-5 s):
  - `wait-pr-actionable.sh 1302 60 120` on an open PR with checks pending:
    exit 2 (`ACTION=wait`) after 120 s wall, calls at 3 s and 68 s, the last
    sleep cut to 52 s.
  - `wait-post-merge-runs.sh --max-wait=120` on a commit with no runs: exit 0
    (`STATUS=no_runs`) after 62 s wall, the second sleep cut to 27 s to end
    the 60 s grace.
  - `wait-post-merge-runs.sh --max-wait=120` on PR 1302's head with one run in
    progress: exit 2 (`STATUS=timeout`) after 121 s wall, four calls, the last
    sleep cut to 23 s. The message reads `timeout after 121s`: the clock read
    after the cut sleep can land a second past the deadline.
  - All land clearly under 180 s, so the numbers of decision 3 stay. A slow
    network day (46 s calls) was not reproduced; by the arithmetic it gives
    up to about 152 s / 156 s.
- Two existing `wait-pr-actionable.test.sh` cases had to change their
  fixtures or max wait (exit codes and `ACTION=` lines kept), because a sleep
  that reaches the deadline now ends the slice without another call:
  - "fails at the end after a success" now has its failed call at 1 s
    (`wait`, `FAIL`) instead of at 2 s; the timeout message line is the same.
  - "a failed call keeps the blocked grace" runs with max wait 3 instead of 2
    so the third call, at 2 s, still happens inside the slice.
- The clock stubs (`date`, `sleep` and a latency in the `gh` stub, over one
  clock file per case) took the suites from minutes to about 57 s (pr, 13
  cases) and 24 s (merge, 7 cases) on this host; what remains is process
  start-up. The `date` stub refuses anything but `+%s`; neither script nor
  `pr-status.sh` calls `date` or `sleep` otherwise.
- Checked that the new cases fail on the old scripts: 3 of 13 pr cases and
  3 of 7 merge cases fail against `HEAD`'s scripts with the new tests.
- The Bash tool collapses `\\` in a heredoc passed through it, so a Python
  edit script written that way turned `printf '%s\n'` into a literal newline.
  Edits with backslashes go through the Edit tool.
- The harness moved a foreground command chaining two real slices (62 s +
  121 s) to the background at 180 s: one slice per Bash call, as the agents
  run them.
