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

# Bring todo.md's menu checks in line with the PopupMenu right-click menus

## Purpose

`native-context-menu` (PR #1118, plan
`docs/plans/_archived/20261116-native-context-menu/plan.md`) replaced the
GPUI app's hand-built right-click menus with gpui-kit's `PopupMenu` and filed
its own real-device check in `todo.md` under
`### Native: real-device checks of the right-click menus drawn as gpui-kit's PopupMenu (native-context-menu)`.
The older `gpui-component-adoption` item's "Step 4: menus" check still
describes the right-click menus as hand-built: its look (26 px rows, the
always-reserved check column, 12 px shortcuts), "closes on any key, an arrow
also acting" (Up / Down now move the menu's highlight) and "nothing takes the
GPUI focus" (the `PopupMenu` now does). This plan narrows that check to the
Sort / Filter menus and leaves the right-click menus to the new item. The
user reviewed and approved the exact edits below.

## Steps

- [x] Step 1: Narrow `gpui-component-adoption`'s "Step 4: menus" check in `todo.md` to the Sort / Filter menus
  - Done when:
    - In `todo.md`, the bullet starting `On Windows (Step 4: menus)` (under
      the `gpui-component-adoption` item) and its sub-bullets are edited so:
      - The lead sentence's look spec names only the Sort and Filter menus
        (drop "tree right-click and strip right-click", and drop "the strip
        menu's shortcuts muted 12 px at the right end", which only described
        the right-click menu).
      - The sub-bullet "Sort and Filter open above their button …" keeps its
        first half and drops "; a right-click menu opens at the pointer and
        flips near the right and bottom edges".
      - The sub-bullet "A right-click menu closes on any key, …" is removed.
      - The sub-bullet "Click in the tree (it takes the keyboard),
        right-click a folder: …" is removed.
      - The sub-bullet "With a menu open nothing takes the GPUI focus." is
        reworded to say it of an open Sort / Filter menu.
    - Nothing else in `todo.md` changes. In particular the
      `gpui-tree-trash` checks (the tree's and the strip's right-click menu
      items, Step 3 / Step 6), the `native-component-controls` `Kbd` look
      check, the gpui-pre snapshot comparison, the Step 1 init check and the
      scrollbar check stay as they are.
    - `mise run ci:local` passes and the PR's GitHub checks pass.
  - Implementation approach: a text edit of `todo.md` only; locate the bullet
    by its text, not by line number (main moves). No code changes.

## Trade-offs and risks

- The removed checks are not lost: the `native-context-menu` item covers the
  right-click menu's placement and flip, the keys with a menu open, the press
  outside, the second right-click and the tree's keyboard hand-back.

## Progress
