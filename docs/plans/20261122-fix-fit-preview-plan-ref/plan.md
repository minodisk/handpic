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

# Fix the stale fit-preview-downscale plan reference

## Purpose

`docs/agents/app.md` names the fit-preview-downscale plan by the path it had
before its wrap-up moved the folder to
`docs/plans/_archived/20261121-fit-preview-downscale/`. A reader following that
path finds nothing. Once this is done, every reference in the living
documentation points at the archived folder.

## Steps

- [x] Step 1: Point the stale reference in `docs/agents/app.md` at the archived
      plan folder
  - Done when:
    - The code span in the section "The fit view decodes at the canvas's DCT
      scale (Inferred)" of `docs/agents/app.md` (line 3199 at planning time)
      reads `../plans/_archived/20261121-fit-preview-downscale/plan.md`.
    - A search of the tracked files for `20261121-fit-preview-downscale` not
      preceded by `_archived/` finds nothing outside
      `docs/plans/_archived/20261121-fit-preview-downscale/` and
      `docs/plans/review-history/` (for example
      `git grep -n "20261121-fit-preview-downscale" -- . ":!docs/plans/_archived/20261121-fit-preview-downscale" ":!docs/plans/review-history" | grep -v "_archived/20261121-fit-preview-downscale"`
      prints no line).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Docs-only, one line: in `docs/agents/app.md` replace
      `../plans/20261121-fit-preview-downscale/plan.md` with
      `../plans/_archived/20261121-fit-preview-downscale/plan.md`. Keep it a
      code span, as it is today; do not turn it into a link or reflow the
      paragraph.
    - Found at planning time and already correct, so not to be touched:
      `docs/agents/app.md` lines 3242 and 4773 and `todo.md` lines 1062 and
      1072 all use the `_archived` path.
    - Re-run the search during implementation rather than relying on the line
      numbers above; fix any other tracked, non-archived reference it shows
      outside the two excluded folders.

## Trade-offs and risks

- `docs/plans/review-history/fit-preview-downscale-step-3/review-20261121-1714.md`
  cites the pre-archive paths as review finding locations, in bold brackets
  rather than as links. This plan leaves them unchanged: the file is a
  timestamped record of a review made while the folder was at that path.
- The reference being fixed is a code span, not a Markdown link, so lychee does
  not necessarily check it. The search in "Done when" is the check that proves
  the fix.

## Progress
