# Learnings: agent-run-device-checks

## Step 1

- The MCP client is left open on purpose. `SKILL.md` 3.3 says to reach the
  app's endpoint "with whatever MCP client the session has (a script client on
  the launch's own `--mcp-port`, or a configured MCP server pointed at that
  port)". The text assumes neither; the earlier runs (PR #1307, PR #1262)
  used a script client against their own port. A configured `handpic` MCP
  server at a fixed port only helps when the launch uses that port, which the
  "other than 41917" rule of `docs/agents/app.md` may conflict with.
- `archive-plan.sh` was checked: it runs `git mv` on the folder and commits
  only what that staged. `git mv` moves the whole directory on disk, untracked
  files included, so a `results.md` written in 3.3 lands under `_archived/`
  untracked, and a `learnings.md` appended in 3.2 / 3.3 lands there with its
  additions unstaged. 3.6's "paths to commit" now names both.
- Lessons from a check run go to the feature's `learnings.md` and to the guide
  sentence that names the check as pending or unseen (the #1307 precedent);
  `learnings-extractor` is not re-run, since 3.2 runs before 3.3.
- The approved defaults shaped the reason list: synthetic mouse and keyboard
  input is counted under "the user" (input only the user gives), and a
  blocked runnable check is a "not run" outcome with what blocked it as its
  reason, listed apart in the PR body.
- The optional pointer bullet in `docs/agents/app.md` was not added; the
  wrap-up rule points to `app.md`, which is enough.

## Deferred issues (todo candidates)

- `planner.md` still lets a plan write "Manually verified on Windows" as a
  Done-when line; the wrap-up now resolves such lines, but rewording how plans
  state hands-on criteria (naming the input kind and material up front, as
  `implementer.md` step 6 now asks of a pending check) is a possible
  follow-up. Basis: this plan's "Trade-offs and risks" (`planner.md` is not
  changed). Files: `.claude/agents/planner.md`,
  `.claude/agents/implementer.md`.
  Completion criteria: `planner.md` asks a plan to name, for each hands-on
  Done-when line, the input kind (agent-runnable or user-only) and the
  material, consistent with the wrap-up's reason list in
  `.claude/skills/develop/SKILL.md` 3.3 and `implementer.md` step 6.
