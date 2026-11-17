# Learnings

## Step 1

- The grace lives in `wait-pr-actionable.sh` as `BLOCKED_GRACE` (default
  60 s), measured in the loop's own `elapsed` (the sum of the sleeps), not wall
  time, so it counts toward `max_wait_sec` the same way and the test stays
  deterministic.
- The test stub answers successive `gh pr view` calls from numbered fixture
  files and a counter in a per-case directory, the last fixture repeating.
  Against the old scripts, every one of its six cases fails (checked by running
  it against `HEAD`'s copies of both scripts in a temp dir).
- On this Windows host the test takes about 13 s (each `pr-status.sh` call runs
  several `jq` processes); the sleeps alone are about 5 s.
- `shellcheck` is not on the plain Git Bash `PATH` here; it is reached through
  `mise run ci:local`.
- `pr-runner.md` and `merger.md` were read, not edited: both handle an exit 2
  whose last ACTION is `blocked` as ordinary waiting (pr-runner's "Otherwise
  increment `wait_timeouts` and re-run", merger's "increment
  `pr_wait_timeouts` and re-run").

## Deferred issues (todo candidates)

- `.claude/agents/pr-runner.md` and `.claude/agents/merger.md` say the waiting
  ACTIONs observed on an exit 2 are only `wait` / `review_required`; since the
  grace in `.claude/skills/pr/scripts/wait-pr-actionable.sh`, `blocked` can be
  one too (a timeout inside the grace). The handling is already right; only the
  sentence is stale. Left out of scope by the plan (Trade-offs and risks).
