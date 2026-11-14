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

# Scrollbars on the native app's strip, folder tree and meta pane

## Purpose

The GPUI app (`crates/native`) scrolls three areas without showing a
scrollbar: the thumbnail strip along the bottom (a `canvas` over the pure
scroll model in `strip.rs`), the folder tree in the left pane (a
`uniform_list`) and the meta pane on the right (an `overflow_y_scroll`
`div`). There is no way to see where one is in a long folder or tree, nor to
jump by dragging.

This plan gives each of the three gpui-component's scrollbar
(`gpui_component::scroll`, pinned at `=0.9.1`), drawn the way the settings
modal's lists and the menus already draw theirs: in the theme's
`ScrollbarMode::Scrolling`, shown while the area scrolls and fading out when
idle. Once done, scrolling any of the three shows its scrollbar, and while
it is shown, dragging the thumb or clicking the track scrolls the area; on
the strip that goes through the same model as the wheel, so the kept range
and the thumbnail requests follow.

The folder operations' dialog row list (`folderops.rs`), the one other
scrolling area without a scrollbar, is not part of this plan: Step 7 of
[the gpui-component adoption plan](../20261114-gpui-component-adoption/plan.md)
gives it one.

## Steps

- [x] Step 1: Draw gpui-component's scrollbar on the strip, the folder tree and the meta pane
  - Done when:
    - The folder tree draws gpui-component's vertical scrollbar over its
      `uniform_list`: while its rows overflow the pane, a scroll shows the
      bar, and while the bar is shown, dragging the thumb and clicking the
      track scroll the list. The wheel, the reveal's and the keyboard's
      `scroll_to_item`, the row clicks and the inline rename behave as
      before.
    - The meta pane's body draws gpui-component's vertical scrollbar the
      same way: shown by a scroll while its rows overflow, and dragging and
      clicking scroll it while it is shown. The notes under the body stay
      where they are and do not scroll.
    - The strip draws gpui-component's horizontal scrollbar through a handle
      that implements the component's `ScrollbarHandle` over the strip's own
      model: while its cells overflow its width, a change of the strip's
      offset (a wheel scroll, `scroll_into_view`, a refilter, a re-listing,
      a window resize) shows the bar with the thumb at the offset, and while
      the bar is shown, dragging the thumb and clicking the track move the
      strip exactly as a wheel scroll does: the offset clamped by
      `Strip::scroll_to`, the thumbnails outside the range released and the
      ones inside requested, and the scan's hot list updated
      (`strip_moved`). A press on the shown scrollbar does not select the
      cell under it; with the bar not shown, a press in its place selects
      the cell as before.
    - No scrollbar is drawn where the content fits (an empty or short
      strip, a short tree, a short meta pane).
    - The three bars are in the theme's mode (`ScrollbarMode::Scrolling`);
      no bar sets a mode of its own, the theme's mode is not changed, and
      the strip's height and layout are unchanged.
    - The handle's pure logic has unit tests: the offset it reports is the
      strip's, negated; its content size is the cell count times
      `CELL_PITCH`; an offset the scrollbar sets is handed over once, as the
      strip's positive offset, and not again; a sync from the model does not
      count as a request. The existing tests pass unchanged.
    - `docs/agents/native-app.md` gains a section on the three scrollbars
      (the strip's handle and where its offset is applied, the theme's
      mode, what a press on a shown bar does and does not reach) and
      `CLAUDE.md`'s Layout paragraph names the handle where it describes
      `src/strip.rs` (or its own module).
    - `learnings.md` records the pending manual check: each of the three
      scrolled by wheel, the bar showing and fading out; within the time it
      is shown, the thumb dragged and the track clicked; on the strip,
      thumbnails appearing after a drag to the far end of a long folder, the
      bar showing with the thumb following the arrow keys, a refilter and a
      window resize, and a press on the shown bar not changing the current
      file; on the tree, a row click, a right-click and the inline rename
      next to the bar; no bar with short content; the settings modal's and
      the menus' scrollbars as before.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/strip.rs` (or a small new module beside it
      for the handle, declared in `main.rs`), `crates/native/src/main.rs`
      (`tree_pane`, `meta_pane`, the strip's `div` in `render`, the view's
      fields), `docs/agents/native-app.md`, `CLAUDE.md`, this folder's
      `learnings.md`.
    - The API, from the registry source of `gpui-base` 0.9.1
      (`src/scrollbar.rs`) and `gpui-component` 0.9.1
      (`src/scroll/scrollable.rs`):
      - `ScrollbarHandle` is public: `viewport_bounds`, `offset`,
        `set_offset(&self, ..)`, `content_size`, plus the defaulted
        `start_drag` / `end_drag`. Offsets are `ScrollHandle`'s: zero or
        negative. `UniformListScrollHandle` (the view's `tree_scroll`) and
        `ScrollHandle` implement it.
      - `ScrollableElement` (already imported in `main.rs`):
        `vertical_scrollbar(&handle)` and `horizontal_scrollbar(&handle)`
        take any `H: ScrollbarHandle + Clone` and add an
        `absolute().inset_0()` overlay child whose viewport is its own
        layout bounds (`viewport_from_layout`), in the theme's mode, so the
        element they are called on has to be `relative()` (as
        `menu_surface` is); `overflow_y_scrollbar()` wraps the element,
        moving its size onto a wrapper and keeping its own scroll handle
        (as `settingsview.rs` uses it). Each helper's id is its call site
        (`#[track_caller]`), so three call sites give three ids.
      - The bar is not drawn when the content size is not larger than the
        viewport; nothing has to hide it by hand.
      - In `Scrolling` mode the bar becomes visible when its prepaint sees
        the handle's offset differ from the last one it saw, and stays for
        the theme's idle time (2 s by default) or while hovered or dragged;
        its mouse-down listener is registered only while it is visible.
      - On a drag or a track click the bar calls `handle.set_offset(..)`
        and then `cx.notify(window.current_view())`; there is no callback.
        Its mouse-down listener stops propagation in the bubble phase and
        runs before its parents' listeners (the adoption plan's
        `learnings.md`, Step 4).
    - The tree: the pane's `div` becomes `relative()` and gets
      `.vertical_scrollbar(&self.tree_scroll)`. A press on the shown bar
      then does not reach the pane's `on_mouse_down` (the tree does not take
      the keyboard from it) nor a row's click; `on_mouse_down_out` is
      unaffected. Leave that as it falls out and document it (Trade-off 3).
    - The meta pane: the body ends in `.overflow_y_scrollbar()` in place of
      `.overflow_y_scroll()`, as the settings modal's lists do (`Scrollable`
      is a `ParentElement`, and the adoption plan's `learnings.md` records
      that it moves `flex_1` / `min_h_0` onto its wrapper), so the bar
      covers the body only, not the notes. If that does not lay out as
      before, the fallback is a `ScrollHandle` on the view with
      `track_scroll` and `vertical_scrollbar` on a `relative()` wrapper of
      the body, the menus' layout.
    - The strip's handle: a `Clone` type over an `Rc<Cell<..>>` shared with
      the view (the precedent is the preview canvas's size cell in
      `native-app.md`, "The canvas's size reaches the view through a shared
      cell"). It holds the model's offset and content width, which the view
      writes before the strip is drawn, and the offset the scrollbar last
      set, which the view takes. `Strip` needs a public content width
      (`len * CELL_PITCH`; `max_offset` is private). `strip.rs` imports no
      gpui type today; whether the handle lives there or in its own module
      is the step's choice, with the unit tests next to it.
    - The strip's `div` becomes `relative()` and gets
      `.horizontal_scrollbar(&handle)` after the `canvas` child (and after
      the cells' tooltip region `div`s, if they would otherwise sit above
      the bar). Its viewport is then the strip's `div`, which spans the
      window, so its width equals `strip_width` and the scrollbar's range
      (`content - viewport`) equals `Strip::max_offset`; the handle's
      `viewport_bounds()` is not read for placement.
    - Applying a set offset: at the top of `Riffle::render`, take the
      handle's pending offset and, when `self.strip.scroll_to(offset,
      self.strip_width)` moved, run what `strip_scrolled` runs
      (`strip_moved`). Verify that `window.current_view()` during the
      strip's paint is the `Riffle` entity (it is rendered inside `Root`),
      so the bar's notify re-renders it, and that the `cx.notify()` inside
      `strip_moved` during render causes at most one more frame and no loop
      (the pending offset is taken once).
    - The overlay is painted after the `canvas`, so the bar's press is
      handled before the strip's `on_mouse_down` listeners and stopped. The
      bar inserts a `HitboxBehavior::Normal` hitbox over its viewport;
      verify the strip's clicks, right-clicks, tooltips and `on_scroll`
      still arrive.
    - The strip's bar is drawn along the bottom of the strip, over the
      7 px `STRIP_PADDING` and, by the thumb's geometry (a 16 px track, a
      6 px thumb, 8 px while active), possibly the bottom of the cells' name
      band, for as long as it is shown. That is accepted (Trade-off 1); do
      not change `STRIP_HEIGHT`.
    - The scrollbar's colors come from the theme (`scrollbar_thumb` is
      already mapped in `theme.rs`); add no color.
    - No screen is available to the agent: follow the adoption plan's
      practice of recording the on-screen checks as pending in
      `learnings.md`.

## Trade-offs and risks

1. **The scrollbar's mode: the theme's `Scrolling`.** Decided (2026-11-14,
   user). The three bars use the theme's default mode through the
   `ScrollableElement` helpers, the same as the settings modal's lists and
   the menus: a bar is shown for the idle time after a scroll and takes a
   press only while shown, so it cannot be grabbed at rest without scrolling
   first. Not taken: `ScrollbarMode::Always` on these three (always there to
   drag, as the Tauri app's, but it needs hand-built overlays and covers
   content at rest), `ScrollbarMode::Hover` on these three, and changing the
   theme's mode (which would also change the settings modal's and the
   menus' bars). With it, no gutter is added under the strip: the bar
   overlays the strip's bottom edge only while shown, and the layout is
   unchanged.
2. **The strip's offset is applied in `render`.** The component reports a
   drag only by writing the handle and notifying the view, so the view
   reads the handle at the top of `render` and runs `strip_moved` there
   (thumbnail requests, the scan's hot list, a `cx.notify()`). The other
   option is to send the offset through the view's event channel, which
   keeps `render` free of side effects but needs a sender in the handle and
   a new event; not taken, for size.
3. **A press on the tree's scrollbar does not give the tree the keyboard.**
   The bar stops the press before the pane's listener. Giving it the
   keyboard would need a capture-phase listener, as the right-click popover
   got in the adoption plan's Step 4. Left as it falls out; a press outside
   the pane still hands the keyboard back.
4. **Scrolling a live inline rename away.** `native-app.md` records that a
   tree row scrolled out of the `uniform_list`'s range loses its edit and
   confirms; dragging the tree's bar can do that as the wheel already can.
   Unchanged.
5. **Other plans in flight.** The adoption plan's remaining steps touch the
   dialogs only; a native meta-analysis plan may edit `meta_pane`; whichever
   merges second rebases over the other's changes there.
6. **Not seen on screen.** The agent cannot drive the window, so the look
   and the pointer behavior are verified only by the manual check listed in
   `learnings.md`.

## Progress
