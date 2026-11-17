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

# Native strip header: Sort and Filter move from the status row to the strip

## Purpose

In the GPUI app (`crates/native`), `Sort ▾` and `Filter ▾` sit at the right
end of the status row, away from the filmstrip they act on. This work moves
them into a new header row on the strip's top edge, so the information
architecture is:

- the strip pane owns the controls over which files are listed and in what
  order;
- the status bar shows where the preview is within the whole (position /
  count, with a marker while a filter is active) and the scan / operation
  state (status text, progress circle).

The status bar's position stays visible while the strip is hidden
(`toggleStrip`), so the user still sees that a filter is in force when the
header and its lit `Filter ▾` are gone.

## Decisions (settled with the user before Step 1)

- The filter-active marker keeps today's text: `3 / 80 (120)` (no change to
  `pager::position`).
- The resize handle sits on the header's top edge (the pane's boundary with
  the preview).
- The buttons take the header's right end (the popover's anchor code
  unchanged).
- The status row is pinned to its former height (today's `MENU_BUTTON_HEIGHT`,
  21 px), so it does not jump when the progress circle appears and the
  preview canvas shrinks by exactly the header height.
- `docs/humans/usage.md` is left untouched.

## Steps

- [x] Step 1: Move `Sort ▾` / `Filter ▾` into a header row on the strip's top
      edge
  - Done when:
    - With the strip shown, a header row is drawn on its top edge holding
      `Sort ▾` and `Filter ▾`; each opens its menu anchored to its button
      there. The menus' behavior is unchanged: `Menus` open state, the
      same-press toggle, dismissal on a press outside, `escape`, `apply`,
      the lit filter button, the dimmed sort button in a view-only folder.
    - The status row no longer holds the two buttons. It keeps the status
      text, the position / count with its filter-active marker, and the
      progress circle, at its former height.
    - Hiding the strip (`toggleStrip`) hides the header with it. An open
      menu does not stay open with no button to anchor to.
    - The stored `stripHeight` still means the cells' area height:
      `strip::Geometry::of` is unchanged, there is no migration, and
      `stripheight::from_setting` / `setting` and their tests are unchanged.
      The drawn strip pane is the header height taller.
    - A drag of the strip's top edge keeps the preview at least
      `stripheight::PREVIEW_MIN` with the header counted, and keeps the
      cells' height within `MIN`..`MAX`.
    - A press, right-click or wheel scroll on the header does not select a
      cell, open the strip's context menu or scroll the strip.
    - MCP is unchanged: `set_sort` / `set_filter` still go through
      `apply_menu`, and the existing `companion.rs` tests pass unmodified.
    - Unit tests cover the pure parts: the height math with the header (the
      preview minimum, the `MIN` / `MAX` clamps, rounding). The status bar's
      position text is unchanged and its existing tests in `pager.rs` pass.
    - `CLAUDE.md` and `docs/agents/native-app.md` describe the new layout
      (see the approach for the places).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The checks that need the running app (below, under "What to verify")
      are done by hand on Windows, or listed as pending in the PR body and
      `learnings.md` so the wrap-up can carry them into `todo.md`.
  - Implementation approach:
    - Change `crates/native` only; the Tauri app (`crates/app`) is frozen.
      No change to `menu.rs`'s logic, `companion.rs`, `pager.rs`,
      `strip.rs`'s `Geometry::of` or the settings keys.
    - Layout (`main.rs`, `render`): the strip is today one `relative()`
      `div` of height `strip_geometry.height` that carries the cells'
      left-press, right-press and `on_scroll` handlers, the canvas, the
      tips, the horizontal scrollbar and `strip_handle()`. Put the header
      outside that `div`, in a new wrapper under the same
      `self.panels.strip.then(..)`, so the header hides with the strip and
      the cells' handlers never see a press on it. The cells' `div` keeps
      its height, so the strip's `x` math, `tip_regions`, `paint_strip` and
      the scrollbar are untouched.
    - Header: a fixed-height row (a named constant, so the height math and
      the layout cannot disagree) holding the two `menu_button` results at
      its right end. Use the theme's / `Roles`' existing colors
      (`roles.surface` is the surface behind the strip and the status row);
      add no new role unless one is needed.
    - `menu_button` and `popover` move as they are; only the caller
      changes (`status_row` stops calling `menu_button` and stops taking
      `max_menu_height` if it no longer needs it). The popover keeps
      `deferred` + `anchored` on the button's top-right opening upward.
    - Height math: the header height constant belongs next to the other
      strip height constants in `stripheight.rs`, and the drag rule there
      must account for it: the wrapper's `on_drag_move` gives the pane's
      bottom and the height the preview and the whole pane share, so the
      cells' height is the pointer's distance from the bottom less the
      header, and the room is `shared` less `PREVIEW_MIN` less the header.
      Whether `dragged` takes the header itself or the caller subtracts it
      is the step's choice; either way the rule is unit-tested in
      `stripheight.rs` (extend `a_drag_leaves_the_preview_its_minimum` and
      `a_drag_follows_the_pointer_within_the_range`).
    - Resize handle: move `strip_handle()` so its grab band is on the
      header's top edge (the pane's boundary with the preview), as the last
      child of a `relative()` element spanning the pane, as today. Its band
      is the top 5 px, so the header's padding must keep the buttons below
      it.
    - Status row: pin its height to the former `MENU_BUTTON_HEIGHT`
      (21 px) so it does not change when the progress circle appears or
      disappears.
    - Closing an open menu when the strip is hidden: `change_panels` is the
      one place the panes change (keys and the MCP `set_panes`); close
      `self.menus` there when the strip goes away, as the view-only open
      already closes a sort menu.
    - What to verify during implementation:
      - The popover with little room above. From the header the room above
        is the window less the strip pane and the status row, smaller than
        `menu_max_height(window)`. Check a long filter menu (EXIF sections)
        with the strip at its tallest and a short window: the menu must
        stay inside the window and scrollable, and a press on its button
        must still toggle it. If `snap_to_window` pushes the popover over
        its own button, cap the height to the room above the header.
      - `deferred` draws without a content mask, so the header's layout
        must not clip the popover; confirm on screen.
      - The handle on the header's top edge: the cursor, the 1 px line, a
        drag down to `MIN` and up to `MAX`, the preview stopping at
        `PREVIEW_MIN`, and that the first 5 px do not swallow a press meant
        for a button.
      - Cell size at a stored `stripHeight` (e.g. 128: 144 x 96 box) is the
        same before and after.
    - Docs to update in the same PR:
      - `CLAUDE.md`: the `src/menu.rs` clause ("the status row's `Sort ▾`
        and `Filter ▾` menus", "popovers above their gpui-component
        `Button`s"), the `src/progress.rs` clause ("in the status row's
        `StatusBar`, left of `Sort ▾`") and the `src/stripheight.rs` clause
        (the header, the pane's drawn height).
      - `docs/agents/native-app.md`: "The sort and filter menus are
        deferred popovers over the window", "One progress circle sits in
        the status row's right region", "The status row is gpui-component's
        `StatusBar` with its defaults" (the regions' content and the row
        height rule), the panes rule "hiding the strip removes only its own
        `div`; the status row (the status line and `Sort` / `Filter`)
        stays", and "The strip's height is one number, stored under
        `stripHeight`" (the handle's place, the drag rule). The "Measured:
        the preview canvas is 1160 x 842" statements go stale by the header
        height; reword them as not re-measured rather than inventing a
        number, unless the canvas is measured again.
      - Code comments that name the place: `menu.rs`'s and `progress.rs`'s
        module docs, `main.rs`'s `menus` field doc, `status_row`'s doc and
        `MENU_BUTTON_HEIGHT`'s doc.

## Trade-offs and risks

- **The preview loses the header's height** whenever the strip is shown,
  and the window's minimum useful height grows by the same amount. As
  today, the preview's minimum holds during a drag only; a window made
  shorter afterwards takes the space from the preview.
- **The popover has less room above the header** than above the status
  row. A long filter menu on a short window with a tall strip may have to
  scroll sooner, or be snapped over the header by `snap_to_window`.
- **`docs/humans/usage.md` is left untouched.** It already says the
  filmstrip's header bar holds the filter and sort menus (the Tauri
  layout), and also places the `N / M` counter there, which in the native
  app is in the status bar.
- **Hand checks.** The layout, the popover's placement and the drag can
  only be confirmed in the running app; macOS and Linux are covered by the
  CI build and the unit tests only.

## Progress

- (2026-11-17) Step 1 complete; the on-screen checks are pending (see learnings.md "Deferred issues (todo candidates)")
