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

# Drop the completed items from the face/eye focus check section of todo.md

## Purpose

`todo.md` is meant to list only unfinished work: the repository's convention
(`.claude/agents/todo-curator.md`, "Write the deletion proposals") is that a
completed item is deleted line and all, not changed to `- [x]` and kept, since
git history retains what was completed. The section
"### Core: face/eye-aware focus check for culling" still holds seven `- [x]`
items, the only checked items left in the file. Deleting them makes the
section read as what remains, and leaves `todo.md` with no checked item.

## Steps

- [x] Step 1: Delete the seven completed `- [x]` items from the section
      "### Core: face/eye-aware focus check for culling" of `todo.md`
  - Done when:
    - `grep -cE "^\s*- \[x\]" todo.md` prints `0`.
    - Each of the seven items is gone with its whole multi-line body (the
      `- [x]` line and every indented continuation line under it), leaving
      no orphaned continuation line and no blank line inside the list.
    - The rest of the section is byte-for-byte unchanged: the `### ` heading,
      the intro paragraph ending in "What remains:", the `#### TODO` heading,
      the eight `- [ ]` items with their bodies and in their order, and the
      trailing `Related:` line. `git diff --stat` shows `todo.md` with
      deletions only (72 lines at the snapshot below, 0 insertions), unless a
      dangling reference had to be fixed.
    - No remaining text in `todo.md` refers to a deleted item.
    - Nothing outside that section of `todo.md` is changed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - The only file that changes is `todo.md` at the repository root (plus
      this plan's own update). Documentation only, in English.
    - Locate the items with `grep -nE "^\s*- \[x\]" todo.md`, not by line
      number, in case `todo.md` moved on main. At the planning snapshot
      (commit `f0622e5c`, the section heading at line 78) the seven items
      and their bodies are:
      - lines 84–89: "MakerNote check (exiftool 13.59, 2026-09-22): Sony α7 V
        ARW writes ..."
      - lines 90–93: "Measured on Linux WSL2 (not the Mac): detection latency
        on real ..."
      - lines 94–110: "Optionally, detect closed eyes. ..."
      - lines 120–131: "Estimate the head pose of the judged face. ..."
      - lines 150–157: "Give the no-AF path a good-photo mark once it has a
        focus cue. ..."
      - lines 158–170: "Measure the AF eye in-focus probability over each
        eye's eyelid region ..."
      - lines 171–182: "Measure whether de-rotating the face crop by YuNet's
        eye line ..."

      These form three contiguous blocks: 84–110, 120–131 and 150–182.
    - An item's body ends at the line before the next line that starts with
      `- [` at column 0; continuation lines are indented six spaces. The
      last deleted item (the de-rotation one) is followed directly by the
      kept `- [ ] Faces rolled past 90 deg ...` item.
    - The `- [ ]` items that stay, in order, start with: "Judge every face at
      or above the 60 px floor", "Measure the good-photo mark's precision on
      labels", "Re-check the good-photo cuts after the closed-eyes and
      head-pose label reviews", "Faces rolled past 90 deg", "Review the mesh
      misfit labels", "Suggest the sharpest-eye frame within a burst group",
      "AF on a person in the background", "Back-of-head and upturned faces".
    - Dangling references: the planning read of the remaining section found
      none. The two cross-references that stay both point at surviving text:
      "the items in the sections below" (the closed-eyes and head-pose label
      review sections) and "the per-frame good-photo mark above" (the kept
      "Measure the good-photo mark's precision on labels" item). Re-check
      after the deletion; if one is found after all, change only that
      reference, minimally, and record it in `learnings.md`.
    - Do not reorganize, reword or re-wrap any other item of `todo.md`, the
      intro paragraph included, and do not touch the plan paths the kept
      text names in backticks.

## Progress

- (2026-11-26) Step 1 complete
