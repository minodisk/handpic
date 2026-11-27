# Learnings

## Step 1

- The plan's Done-when asked to delete the `todo.md` section "with the `---`
  separator that precedes it", but that section had no `---` before it (it
  sat between two `###` sections after the `---`-separated part of the
  file). Only the section itself, heading through its TODO list and the
  blank line after it, was removed.
- The step's duration, the cache restore state and the Linux leg's total
  against its 30-minute limit come from the PR's first CI run; the
  measurement and its sentence in `docs/agents/app.md` are added in a
  follow-up commit on the same PR, as the plan says. The first run's log
  must also show `Checking gpui-*` lines to confirm the check builds the
  variant without `test-support`.
- On this Windows host a bare `python3 - <<'EOF'` heredoc chained with a
  second heredoc fallback ran an empty script and silently did nothing.
  Write the script to a scratchpad file and run it with
  `mise x -- python <file>`.
