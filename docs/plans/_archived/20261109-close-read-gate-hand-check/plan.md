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

# close-read-gate-hand-check

## Purpose

The real-device hand check that `preview-read-gate` left as a todo (paging with the arrow keys through the release app during a first scan) was done and confirmed by the user on 2026-11-09. The todo item is finished, so it comes out of `todo.md`; otherwise it reads as still open.

## Steps

- [x] Step 1: Remove the finished hand-check todo from `todo.md`
  - Done when:
    - `rg "real-device check of the scan's read cap" todo.md` finds nothing
    - The preceding `### App: measure the scan's read cap on NVMe` and following `### App: test the scan's read slot in \`embed_all\` and the post-read cancel check` sections are untouched, with one blank line between them as elsewhere in the file
    - `mise run ci` passes
  - Implementation approach:
    - Delete the whole `### App: real-device check of the scan's read cap and the preview pause while paging` section in `todo.md` (the heading, `#### Background`, its paragraph, `#### TODO` and its one item), plus the blank line that would otherwise double up.
    - Nothing else changes: no other todo items, no code, no docs, and not the archived plan `docs/plans/_archived/20261107-preview-read-gate/plan.md`.

## Progress

- (2026-11-09) Step 1 complete
