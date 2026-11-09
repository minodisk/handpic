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

# Drop the full-size-eye-check manual check from todo.md

## Purpose

The user ran the manual check `full-size-eye-check` left in `todo.md` on the real app on 2026-11-09: in `D:\Photos\2026\2026-11-07`, `_DSC1508.ARW` and `_DSC1536.ARW` show no strip face icon, a white focus mark and `Eyes at full size` = `Soft` (0.104 and 0.093). The item is done and comes out of `todo.md`.

## Steps

- [x] Step 1: Remove the section `### App: real-device check that a frame soft at full size is no longer Good (full-size-eye-check)` from `todo.md`
  - Done when: that section (its heading, `#### Background`, `#### TODO` and the one checkbox, plus the blank line that separates it from the next section) is gone from `todo.md`; the two neighbouring sections `### Core: \`good_photo\` in the ranking leaves out the full-size eye cut that \`photoTier\` applies` and `### Core: the cold-disk cost of pass 2's full-size eye read is unmeasured` are byte-identical to before; no file outside `todo.md` and this plan folder changes; `mise run ci` passes.
  - Implementation approach:
    - The item is a self-contained `###` section, so the whole section is deleted; nothing is shared with the other two full-size-eye-check items, which follow it as their own sections. Find it by its heading, since line numbers may have moved.
    - Docs-only change; `docs/plans/_archived/20261108-full-size-eye-check/plan.md` stays untouched.
    - Verify with `git diff --stat` (only `todo.md` plus the plan folder) and `mise run ci`.

## Progress

- (2026-11-09) Step 1 complete
