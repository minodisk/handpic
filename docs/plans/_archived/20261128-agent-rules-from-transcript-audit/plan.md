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

# Agent rules from the pr-runner / merger transcript audit

## Purpose

Three prose rules in `.claude/agents/merger.md` and `.claude/agents/pr-runner.md`
were verified only statically, and `todo.md` carries one section per rule asking
for a check on a real `/pr` run. An audit of every `pr-runner` (442) and
`merger` (355) subagent transcript on this host from 2026-11-13 to 2026-11-27,
all after the current rules landed, now answers those checks:

- `merger` step 6: all 289 runs that reached it ran `mise run git:main` and
  `./tools/git/delete_merged_branches.sh` as separate, verbatim Bash calls with
  `dangerouslyDisableSandbox: true`; 0 refusals, 0 `post_merge_sync_failed`.
  Two transient failures both passed on a re-run of the same command:
  `cannot lock ref refs/remotes/origin/main` (a fetch race) and
  `delete_merged_branches.sh` exit 1 with `branch ... not found`. Step 6's
  "If either of these fails, return `FAILED`" does not allow that re-run.
- `pr-runner` hand-off to children: 158 runs, 207 dispatches (`pr-check-fixer`
  84, `pr-conflict-resolver` 123); every one ended the turn and waited for the
  hand-back with no polling in between (one harmless `echo waiting`).
  `pr-review-planner` and `pr-review-addresser` were never dispatched.
- `pr-runner` §4 script calls: of 3,137 script calls, 0 used pipes, `tail`,
  redirects, `${PIPESTATUS[...]}` or `sleep`. The deviations left: 17 runs
  (127 calls) prefixed `cd <worktree> &&` though the cwd is already the
  worktree (still 3 of 24 runs after commit 739a17ee on 2026-11-26); 4 calls
  chained with other commands (for example `rm -f tmp/pr-body.md; git status
  --porcelain; bash .claude/skills/pr/scripts/wait-pr-actionable.sh ...` and
  `bash ...rerequest-review.sh ...; gh pr view ...`); 2 runs sent the same wait
  call twice in one message.

This plan turns those results into the rule changes they call for and trims
`todo.md` to what is still unconfirmed. The decisions are already made; the PR
body carries them and their reasons (the counts above).

## Decisions

- The merger re-run applies to any failure of either step 6 command, one re-run
  per command, verbatim, as its own Bash call with
  `dangerouslyDisableSandbox: true`. Simpler for a model to follow than
  classifying error messages; a deterministic failure costs one extra call.
- The two narrowed todo sections get headings matching their new scope.
- The audit counts are not written into `merger.md`; they live in the PR body
  and this plan.

## Steps

- [x] Step 1: Apply the audit's decisions to `merger.md`, `pr-runner.md` and
      the three `todo.md` sections
  - Done when:
    - `.claude/agents/merger.md` step 6 allows re-running a failed step 6
      command once, verbatim, and returns `FAILED` (reason:
      `post_merge_sync_failed`) only when the re-run fails too, still stating
      that the merge and post-merge succeeded
    - `.claude/agents/pr-runner.md` §4's "Run every script under
      `.claude/skills/**/scripts` exactly as written ..." paragraph states
      that a script call takes no `cd` prefix (the cwd is already the
      worktree) and that the same script call is not sent twice in one
      message; nothing else in `pr-runner.md` changes (the "Dispatching a
      child and waiting for its hand-back" wording stays as it is)
    - `todo.md`: the section "Agents: confirm `merger` runs its step 6
      commands as separate, verbatim calls on a real `/pr` run" is deleted;
      the section "Agents: confirm `pr-runner` waits for a child's hand-back
      on a real `/pr` run" is retitled and asks only for the review-round path
      (`pr-review-planner` / `pr-review-addresser`), its Background holding
      the dispatch counts; the section "Agents: confirm `pr-runner` runs its
      skill scripts verbatim as standalone calls on a real `/pr` run" is
      retitled and asks only that the `cd` prefix and the doubled call
      disappear in later runs
    - No other `todo.md` section, nothing under `.claude/skills/develop/**`
      and not `.claude/agents/todo-curator.md` is in the diff
    - The PR body lists the three decisions with the audit counts
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach:
    - `merger.md`: the only text to change is the closing paragraph of
      "### 6. Sync main and clean up branches" ("If either of these fails,
      return `FAILED` (reason: `post_merge_sync_failed`), stating explicitly
      that **the merge and post-merge succeeded** ..."). Keep that wording for
      the final failure and put the re-run ahead of it. The re-run is the same
      command as written in its code block, as its own Bash call, with
      `dangerouslyDisableSandbox: true` again, so it still matches the allow
      rule in `.claude/settings.json`. Give the reason in the file's own
      style (the two measured transient failures, both passing on the
      re-run). The `post_merge_sync_failed` mentions at the top of step 6 and
      in "## Output" need no change
    - `pr-runner.md`: add the two prohibitions to the existing list in the
      §4 paragraph (it already lists pipes, `tail`, redirects,
      `; echo exit=$?`, `${PIPESTATUS[...]}`, `;` / `&&` chaining and
      `sleep`). The reason for the `cd` rule fits the paragraph's existing
      explanation: `.claude/settings.json` allows the
      `bash .claude/skills/...` command lines alone, and a line that starts
      with `cd` is another shape. `merger.md` step 6 already says "no `cd`
      prefix"; use the same phrase. Keep the file's ~80-column hard wrap
    - `todo.md`: locate the three sections by their `### Agents:` headings,
      not by line number; other sessions edit this file concurrently. Rebase
      onto `origin/main` right before the push and check the diff touches
      only these three sections
    - `todo.md` format: keep `#### Background` / `#### TODO` and the
      `- [ ]` items. Match each section's own wrapping
    - Hand-back section: Background keeps why the rule exists and adds the
      result (158 runs, 207 dispatches: `pr-check-fixer` 84,
      `pr-conflict-resolver` 123, all clean; `pr-review-planner` and
      `pr-review-addresser` never dispatched). The TODO item narrows to the
      next real `/pr` run that reaches a review round
    - Verbatim-scripts section: Background keeps the PR #671 reason and adds
      the result (3,137 calls, none decorated; the `cd` prefix in 17 runs /
      127 calls, 3 of 24 runs after 739a17ee; 4 chained calls; 2 runs with a
      doubled wait call) and that this plan added the two prohibitions. The
      TODO item narrows to confirming, on runs after this PR merges, that no
      script call carries a `cd` prefix and none is sent twice in one message
    - When a todo section names this plan, use the path it will have once
      archived (`docs/plans/_archived/20261128-agent-rules-from-transcript-audit/plan.md`),
      in backticks
    - Everything is English

## Trade-offs and risks

- The 4 chained calls get no new wording: `;` / `&&` chaining is already
  banned in §4, and the narrowed todo item tracks only the two new
  prohibitions. If chaining recurs, that todo item does not catch it.
- `todo.md` is edited by other sessions. Resolve a rebase conflict by keeping
  the other sessions' sections untouched.
- The audit counts were taken from transcripts outside the repository and are
  carried here as given, not re-derived.

## Progress

- (2026-11-28) Step 1 complete
