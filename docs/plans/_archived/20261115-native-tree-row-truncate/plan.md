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

# Truncate the native folder tree's row names inside the pane

## Purpose

In the GPUI app (`crates/native`) a folder tree row is as wide as its
content, not as wide as the 220 px left pane. A long folder name (for
example `2026-10-03-dop-orientation`) runs past the pane and is cut by its
edge with no ellipsis, and the RAW count badge of an expanded folder is
pushed out with it and half hidden.

The cause, read from the pinned sources (not yet confirmed in a running
app): `uniform_list` (gpui-pre 0.3.12, `src/elements/uniform_list.rs`) lays
each item out as a layout root with a definite available width, the list's
padded width. taffy 0.15.0 stretches a root to that width only when it is a
block (`compute_in_flow_root_layout`); `tree_row`'s root is `.flex()` with no
width, so its main size is the sum of its children's flex bases
(`determine_container_main_size`). The name's `flex_1()` basis of `0%` does
not resolve against that indefinite width and falls back to its content
size. The row therefore grows to its content, the name's
`.min_w_0().truncate()` never has anything to truncate, and the list's
content mask clips the row at the pane edge.

Once done, a row is exactly as wide as the list, so a long name ends in an
ellipsis inside the pane and the count badge stays whole at the row's right
edge, as in the Tauri app (`.folder`'s grid `14px minmax(0, 1fr)
max-content` with the `.name` ellipsis in `crates/app/ui/style.css`).

The full-path hover tooltip of a row (the Tauri row's `title`) is not part
of this plan.

## Steps

- [x] Step 1: Make a tree row as wide as the list so its name truncates
  - Done when:
    - In the running native app, a folder name longer than the pane ends in
      an ellipsis inside the pane, at every depth, and a row's content
      never reaches past the pane's right edge.
    - The RAW count badge of an expanded folder is fully visible at the
      right edge of its row, next to a short name and next to a truncated
      one.
    - The cursor ring, the hover highlight and the open row's highlight
      keep their 4 px inset from both pane edges and their rounded corners.
    - The tree's vertical scrollbar is where it was, at the pane's right
      edge, and dragging its thumb still scrolls the list.
    - The inline rename editor fits inside its row, and the held rename's
      pending name (clock icon, italic) truncates like any other name.
    - If the agent cannot drive the app's window, the checks above are
      recorded in `learnings.md` as pending manual checks, naming each one,
      not claimed as done.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Change only `crates/native/src/main.rs`: `tree_row` (the row `div`,
      around lines 4058-4092) and `tree_pane` (around lines 3985-4021).
      `tree_name` in `crates/native/src/folderops.rs` already has
      `.flex_1().min_w_0()` on both of its branches (the name with
      `.truncate()`, the rename editor around the `Input`) and should need
      no change; verify that both fit once the row has a definite width.
    - Give the row `.w_full()` and drop its `.mx_1()`. Keeping the margin
      with `.w_full()` overflows: a percentage width on a layout root
      resolves against the whole available width, and taffy subtracts the
      root's margins only from the available space, so the row would be the
      full list width starting 4 px in and end 4 px past the right edge.
    - Put the 4 px horizontal inset on the list side instead. The step
      decides between `.px_1()` on the `uniform_list` itself (it lays its
      items out in its `padded_bounds`, so the row's `w_full` becomes the
      padded width) and `.px_1()` on the `relative()`, `size_full()`
      wrapper that carries `vertical_scrollbar(&self.tree_scroll)`. Do not
      put it on the outer pane `div` next to its `.py_1()`: that would move
      the scrollbar 4 px in from the pane edge.
    - Verify during implementation that the scrollbar keeps its position
      and its drag with the padding in place, that `scroll_to_item` (the
      reveal on open, the keyboard cursor, the rename's row) still brings
      a row fully into view, and that a right-click and a click on a row's
      inset edge behave as before.
    - Leave the row's own padding (`pl` by depth, `pr_2`), `gap`, border and
      `rounded_sm` as they are, and the count badge `flex_none`.
    - `crates/native` has no GPUI window test today (no `#[gpui::test]`),
      so the layout is checked by hand, not by a unit test.
    - Add a short rule to the folder tree section of
      `docs/agents/native-app.md` ("The folder tree is a `uniform_list` over
      rows kept in the view"): a `uniform_list` item is a layout root, a
      flex root without a width is as wide as its content, so the row takes
      `w_full` and the list carries the inset. Mark it Measured or Inferred
      according to what was actually observed.

## Trade-offs and risks

- The highlight's width changes with the fix. By the same reasoning as the
  cause, a row's hover and open highlight and its cursor ring are today only
  as wide as the row's content; with `w_full` they span the pane less the
  4 px inset on each side, as the Tauri app's rows do. This follows from the
  fix and cannot be separated from it. The user approved this reading: the
  inset and the rounded corners are kept, the content-wide highlight is not.
- Not taken: keeping `.mx_1()` and giving the row a fixed
  `w(px(PANE_WIDTH - 8.))`. It ties the row to the pane constant and breaks
  as soon as the pane's width or padding changes.
- Not taken: wrapping each row in an outer `w_full().px_1()` `div`. It works
  but adds an element per row for what one padding on the list does.
- Not taken: an automated layout test. It would need gpui's test support as
  a new dev-dependency of the separate `crates/native` workspace and the
  first window test in the crate, which is more than this fix calls for. The
  cost is that the acceptance rests on a manual check.
- Risk: the cause is read from the gpui-pre 0.3.12 and taffy 0.15.0 sources,
  not observed. If the name still does not truncate with `w_full` on the
  row, stop and record what the layout actually does in `learnings.md`
  before trying anything wider.

## Progress

- (2026-11-15) Step 1 complete
