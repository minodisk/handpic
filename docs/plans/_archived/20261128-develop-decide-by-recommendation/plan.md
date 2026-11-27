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

# Decide by recommendation in the develop skill

## Purpose

The `develop` skill's main agent stops and asks the user at many points: plan
open questions, the 0.2 plan approval, `needs_discussion` entries, the
parallel-work check's exit 1, and the direction after `aborted` / `closed` /
`FAILED` / `MAX_ROUNDS_REACHED` / `LOCAL_CHECK_FAILED`. When one option is
recommended, the user picks it nearly every time, so the question only costs a
wait. After this change the main agent proceeds with a clearly recommended
option, records the decision, reports every such decision in one list at the
end of the run, and then ends the Claude Code session when nothing else is
pending. It still asks when there is no clear recommendation, when the options
are close, or when the action is destructive or irreversible beyond what
develop does routinely.

## Steps

- [x] Step 1: State the decide-by-recommendation rule, the end-of-run decision
      report and the session end in the develop skill, and bring its README in
      line
  - Done when:
    - `.claude/skills/develop/SKILL.md` states the rule once, in one shared
      section, and every asking point in the file either refers to that
      section or is listed there as a stop that stays
    - 0.2 no longer waits for explicit approval when the plan has no contested
      open question, and still asks when the plan holds a choice without a
      clear recommendation
    - One shared section specifies the final "Decisions taken without asking"
      report and the session-end procedure, and both 3.7 and S.7 point to it
    - `.claude/skills/develop/README.md` agrees with SKILL.md wherever it
      describes 0.2, the user approval or the dialogue points
    - `mise run ci:local` passes locally (it runs lychee with
      `--include-fragments` over `.claude/**/*.md` for a Markdown change) and
      the PR's GitHub checks pass
  - Implementation approach:
    - Files: `.claude/skills/develop/SKILL.md` and
      `.claude/skills/develop/README.md` only. Everything in English. Keep the
      edits surgical and in the file's existing style (`§ Name` references,
      bold lead sentences, the "## Notes" bullets, the "## Reporting" block).
    - **Shared rule**: add `### Deciding without asking` under
      `## Shared procedures` in SKILL.md. Content:
      - When the main agent would ask the user to decide and one option is
        recommended and clearly better than the others, it does not ask. It
        takes that option and records the decision: what was decided, the
        options, and why.
      - It asks (with `AskUserQuestion`, not prose) only when no option is
        clearly recommended, when the options are close, or when the action is
        destructive or irreversible beyond what develop does routinely (for
        example deleting a branch or work that may be someone else's).
      - Retry bound: when a recovery taken under this rule returns the same
        failure state at the same point, the recommendation is no longer
        clear, so ask.
      - Stops that stay, whatever looks recommended: the release PR
        (`release-please--branches--main`), an `approval` line for a
        release-shaping path (`.github/**`, `release-please-config.json`,
        `.release-please-manifest.json`), a change to `planner`'s pinned
        model, a `needs_discussion` whose plan JSON is `[]` (a human has to
        dismiss the review), and the case in
        `references/pr-merge-lifecycle.md` where `merger`'s summary and the
        corroboration disagree.
      - The rule also governs every "ask the user for direction" in
        `references/pr-merge-lifecycle.md` when develop follows it. Do not edit
        that reference: the `pr` skill shares it and is not part of this
        change.
      - The running record: the main agent keeps a "Decisions taken without
        asking" list in its own context for the whole run and names each new
        decision in the progress report of the phase where it was taken.
    - **Asking points to bring in line** (each either points to
      § Deciding without asking or is covered by its list of stops). Reword
      "ask for direction" to a form such as "report the situation and decide
      the direction per § Deciding without asking":
      - § Creating a branch: the leftover-branch collision. Deleting a branch
        stays a question (it may be another session's work); say so by
        reference to the rule.
      - § Starting `pr-runner`: the `closed` and `aborted` rows, and the
        `needs_discussion` procedure (items 1 and 3). A `discuss` entry with a
        clearly recommended `fix` / `reject` is reassigned without asking; only
        the others go to `AskUserQuestion`. Reword "it is the one point of
        dialogue".
      - § Merge: "without stopping to ask the user" already agrees; leave the
        release exceptions as they are.
      - 0.1: the last paragraph (trade-offs that come back from `planner`).
      - 0.2: see below.
      - 1.1 parallel-work check, exit 1: continue without asking when the
        traces are clearly unrelated (a substring hit on another feature's
        name); ask when a trace could be the same work, since the main agent
        cannot tell its own leftovers from another live session.
      - 2.2 and S.3 (`LOCAL_CHECK_FAILED`), 2.3 (watchdog past 6 re-arms, the
        "no response" stop, `MAX_ROUNDS_REACHED` / `LOCAL_CHECK_FAILED`), S.4,
        2.4 and S.5 (the step's checkbox not `[x]`), 3.4
        (`settings-promoter`'s `FAILED`), "Recovering from an interruption or
        failure in S.6 / S.7" (its last line), and the demotion's item 1.
      - `## Notes`: rewrite the two bullets that start "Apart from the 0.2
        approval" and "When a subagent returns `aborted` / ..." so they state
        the new behavior and point to the shared section.
    - **0.2**: present the plan, then go straight to Phase 1 / Phase S when no
      open question is contested. Stop and ask only for a choice that has no
      clear recommendation. Keep "if they want changes, go back to 0.1" for a
      user who answers with changes. Keep the feature-name and branch-name
      decisions with the main agent. For a one-step plan, say that the run
      proceeds in single-PR mode (Phase S) and proceed; the user can still
      interrupt and ask for the normal mode. Rename the heading to
      `### 0.2 Presenting the plan` and update the `0.2 user approval` label in
      the "Phase layout" block; re-check fragment links with lychee.
    - **End of the run**: add `## Finishing the run` between Phase S and
      `## Reporting`, and have 3.7 and S.7 each end with a pointer to it
      (S.7's "`MERGED` completes develop" becomes the pointer). Content:
      1. After the final `MERGED`, report every entry of "Decisions taken
         without asking" in one list (what was decided, the options, why), or
         say that there were none.
      2. If everything the user asked for in this session is done, end the
         Claude Code session. When `HERDR_ENV=1`, make this the last tool call
         of the turn, after the final report is written, with the Bash tool's
         `run_in_background: true` and `dangerouslyDisableSandbox: true` (the
         herdr CLI fails under the sandbox):

         ```bash
         sleep 5; herdr pane send-text "$HERDR_PANE_ID" "/exit"; herdr pane send-keys "$HERDR_PANE_ID" enter
         ```

         so `/exit` lands once the turn is idle. The worktree stays for the
         `wrangle` skill's reap. Outside herdr, tell the user the session can
         be closed.
      3. Do not use the `EndConversation` tool; it is not for finishing tasks.
      4. Do not end the session when the run stopped on a failure state or
         when another request of the session is still pending.
    - `## Reporting`: add one optional line to the progress block for the
      decisions taken without asking in that phase.
    - **README** (`.claude/skills/develop/README.md`), places that describe the
      same rules:
      - the flow diagram's `APPROVE["0.2<br/>user approval"]` node label;
      - the "Phase list" row for 0.2;
      - the `Phase 0  plan → approval` line in "The shared pattern";
      - "The 0.2 user approval and the PR review overlap somewhat" in "Phase 1
        makes a PR too";
      - "User approval (subagents cannot have `AskUserQuestion`)" in
        "Persisting the plan does not need an agent";
      - any sentence near the "Hands back to main when" table that says main
        asks the user;
      - add one short design note under `## Design notes` with the rationale (a
        recommended option is chosen nearly every time, so asking is a wasted
        wait; the decisions stay visible through the end-of-run list) and the
        session end.
      - Leave the "Old → new phase mapping" row `2 User approval | 0.2` as it
        is: it names the old phase.
    - `herdr pane send-text <PANE_ID> <TEXT>` and
      `herdr pane send-keys <PANE_ID> <KEY>...` exist (`herdr pane --help`);
      the key is written `enter` as the `wrangle` skill writes it.
    - Not checkable in this step: that `/exit` lands and closes the session on
      a real run. Leave it as a pending manual check so the wrap-up files it in
      `todo.md`.

## Trade-offs and risks

- **0.2 heading.** Renamed to `### 0.2 Presenting the plan` so the heading is
  true to what the section does; keeping the old one is a smaller diff but
  names an approval the section no longer waits for.
- **Retry bound.** Added beyond the request: the same failure state at the same
  point a second time means ask, so a re-dispatch cannot loop unattended.
- **Stops that stay.** The release PR, the release-shaping paths, the `planner`
  model pin, the `[]` deadlock and the corroboration mismatch stay hard stops:
  they are gates or need a human action, not choices with a recommendation.
- **`references/pr-merge-lifecycle.md` is not edited**, since the `pr` skill
  follows it too; SKILL.md says develop reads its ask-the-user lines through
  the new rule. The `pr` and `merge` skills run outside develop and are left.
- **No allow rule for the herdr command.** The unsandboxed session-end call may
  raise a consent prompt at the end of a run; no permission is added.
- **The `/exit` timing.** The background command's exit re-invokes the main
  agent close to the moment `/exit` is submitted; only a real run shows it.
- **The record lives in the main agent's context.** Naming each decision in the
  phase's progress report keeps it in the conversation across a compaction.
- **Stale README text left alone.** The README still describes
  `NEEDS_APPROVAL` as a stopping point while SKILL.md merges with a standing
  approval; out of scope, a todo candidate.

## Progress

- (2026-11-28) Step 1 complete
