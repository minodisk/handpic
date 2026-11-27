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

# Keep Git Bash from rewriting `/exit` in the develop session end

## Purpose

`.claude/skills/develop/SKILL.md` § Finishing the run ends the Claude Code
session under herdr by sending `/exit` to its own pane. On the first real run
(PR #1343's own run, 2026-11-28, Windows host, Git Bash),
`herdr pane send-text "$HERDR_PANE_ID" "/exit"` typed
`C:/Program Files/Git/exit` into the pane: MSYS path conversion rewrote the
leading-slash argument as a Windows path, so the session did not end. With the
command guarded against that conversion, a finished develop run can close its
pane as the earlier work
([plan](../20261128-develop-decide-by-recommendation/plan.md))
intended.

## Steps

- [x] Step 1: Guard the session-end command against MSYS path conversion and
      record the first real run
  - Done when:
    - The command in `.claude/skills/develop/SKILL.md` § Finishing the run
      sets `MSYS_NO_PATHCONV=1` on the `herdr pane send-text` call, and one
      sentence next to it says why (Git Bash converts an argument with a
      leading slash to a Windows path, so `/exit` would be typed as a path
      under Git's install folder)
    - `todo.md`'s item "Agents: confirm the develop session end through herdr
      `/exit` on a real run (develop-decide-by-recommendation)" quotes the
      corrected command, records that the first real run (PR #1343's own run,
      2026-11-28) showed the path conversion and that SKILL.md was corrected
      for it, and keeps the remaining check open: that `/exit`, with the fix,
      lands once the turn is idle and closes the session
    - `.claude/skills/develop/README.md` says nothing that contradicts the
      corrected command
    - `learnings.md` in this plan folder records the observation of the first
      real run
    - No file under `docs/plans/_archived/` or `docs/plans/review-history/` is
      changed
    - `mise run ci:local` passes locally (lychee runs because the branch
      changes Markdown) and the PR's GitHub checks pass
  - Implementation approach:
    - Documentation only; surgical edits, in English. Files:
      `.claude/skills/develop/SKILL.md`, `todo.md`, this folder's
      `learnings.md`
    - SKILL.md: the command is in the fenced block of item 2 of § Finishing the
      run. The target form is
      `sleep 5; MSYS_NO_PATHCONV=1 herdr pane send-text "$HERDR_PANE_ID" "/exit"; herdr pane send-keys "$HERDR_PANE_ID" enter`.
      The variable is a prefix of the one `send-text` command, so it does not
      reach `send-keys`, whose `enter` argument was delivered as is. Add the
      reason to the prose right after the block, keeping the rest of the item
      as it is
    - The repository already states this rule for another tool:
      `docs/agents/app.md` ("under Git Bash, `ISCC.exe`'s `/`-options need
      `MSYS_NO_PATHCONV=1`") and `tools/package/windows.sh`. Word the new
      sentence consistently with it; do not edit those files
    - README: its only mention is prose ("a background `/exit` sent to its own
      pane once the turn is idle") with no copy of the command, so no edit is
      expected. Re-check with a grep for `send-text` and `/exit` over
      `.claude/` and leave README untouched unless a copy turns up
    - todo.md: edit the existing item in place (its Background paragraph and
      its one TODO line); do not add a second item and do not tick it. In the
      Background, replace the quoted command with the corrected one, replace
      "Nothing exercised the call on a real run." with the first run's result
      (`/exit` arrived as `C:/Program Files/Git/exit`, Enter was delivered, the
      session stayed open; the background command's completion re-invoked the
      agent and did not visibly break anything), and name this plan by its
      archived path in backticks
      (`docs/plans/_archived/20261128-develop-exit-pathconv/plan.md`). In the
      TODO line keep the "Done when" clause and the "Not runnable here" clause,
      narrowed to the remaining check
    - learnings.md: record, as an observation of the 2026-11-28 run of
      PR #1343: the text argument `/exit` was converted to
      `C:/Program Files/Git/exit`; the key argument `enter` was delivered
      fine; the background command's completion re-invoked the main agent and
      nothing visibly broke (a note only, no rule change). The archived plan
      `docs/plans/_archived/20261128-develop-decide-by-recommendation/` is
      frozen and is not edited

## Trade-offs and risks

- **`MSYS_NO_PATHCONV=1` alone, not with `MSYS2_ARG_CONV_EXCL='*'`.** Git for
  Windows' variable is what the host's Git Bash honours; the second covers an
  MSYS2 shell no agent uses today.
- **Prefix on `send-text` only.** Only the text argument starts with a slash,
  and `enter` was delivered correctly on the real run.
- **The fix is not verifiable in the PR.** Only the next real develop run inside
  herdr shows whether `/exit` now lands; the todo item stays open for that.
- **The re-invoke by the background command's completion** is recorded as an
  observation only.

## Progress

- (2026-11-28) Step 1 complete
