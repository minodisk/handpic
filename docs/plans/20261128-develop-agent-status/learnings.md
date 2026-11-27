# Learnings

## Step 1: Add the status report to the develop skill

- No text in SKILL.md, README.md or `.claude/agents/` referred to a step of
  "## Finishing the run" by number, so inserting the `done` write as step 2
  and renumbering the rest broke nothing. The new SKILL.md section points at
  § Finishing the run by name, not by step number, to keep it that way
- The kwcp reference also mirrors a `waiting`'s resume condition into the
  auto memory entry and warns that plan.md is swept by 2.1's stash; both were
  left out, as the plan decided (the contract given here does not include the
  memory mirror)
- The cross-file anchors (`../SKILL.md#status-report`,
  `../README.md#the-status-report-tmpagent-statusjson`) rely on lychee's
  `--include-fragments` slugging; the README heading's parentheses, slash and
  dot drop out of the slug
