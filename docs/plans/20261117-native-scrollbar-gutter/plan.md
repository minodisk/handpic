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

# Reserve the scrollbars' space in the native app

## Purpose

Since `native-scrollbars-always` (PR #1139) every gpui-component scrollbar of
the GPUI app (`crates/native`) is drawn at rest whenever its content
overflows. The bars are overlays, so they cover content: the folder tree's bar
lies over the right edge of the rows' RAW-count badges, the strip's bar over
the bottom of the cells' name band, and the meta pane's bar over the right
edge of its values. The user accepted that on 2026-11-17 pending a look on a
real screen, and after that look wants the space reserved instead.

Once done, no tree row text or badge, strip cell or name, meta value or dialog
body text lies under a scrollbar, and a press on any of them is not taken by a
bar's track.

The user approved on 2026-11-17: reserve the whole 16 px track, always (not
only while overflowing), grow the strip's pane rather than shrink its cells,
pad the tree row, and leave the menus, `Select` and settings lists out of
scope.

## Steps

- [x] Step 1: Keep the tree rows, the meta pane's values and the dialog bodies clear of the vertical bar's track
  - Done when:
    - In the folder tree, a row's content (icon, name, inline rename field,
      count badge) ends at least the scrollbar track's width (16 px) from the
      pane's right edge, whether or not the tree overflows.
    - In the meta pane, the bar is drawn at the pane's right edge and the
      scrolled body's rows end at least the track's width from that edge; the
      notes and errors under the body keep their 12 px right inset.
    - The dialog bodies (the trash / delete dialog and the folder operations'
      dialog, both through `modal.rs`'s `shell`) are confirmed, from the
      source, to already keep their text clear of the track, and `shell` says
      so in a comment; no layout change there unless the reading below turns
      out wrong.
    - The track's width is one named constant in `crates/native`, with a unit
      test that fails when the pinned gpui-base's track width differs.
    - `docs/agents/native-app.md` and `CLAUDE.md` describe the new layout
      (English), with no statement left that the tree's or the meta pane's
      bar covers content or that the user accepted the covered edges there.
    - `learnings.md` records the on-screen checks as pending (the agent has
      no screen).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Only `crates/native` and the docs change. The Tauri app is frozen.
      Branch from `main`, which has PR #1139.
    - Facts read from gpui-base 0.9.1 `src/scrollbar.rs` (pinned with `=`
      through `gpui-kit`): the track is the last `WIDTH` = 16 px of the bar's
      parent and takes every press on it; the thumb is anchored 4 px in from
      the outer edge and is 6 px wide at rest, 8 px hovered or dragged, so it
      paints 4-10 px from the edge at rest and 4-12 px hovered or dragged. The
      track's fill is the theme's `scrollbar` color, which is fully
      transparent in Neutral (`theme.rs` does not set it; Default's is
      `...00`) and in every vendored file under `crates/native/themes/`.
    - The constant: `Scrollbar::width()` (`gpui_kit::component::scroll::Scrollbar`,
      a `#[doc(hidden)] pub const fn`) returns the 16 px. Proposed: an `f32`
      constant next to `finish` in `crates/native/src/theme.rs`, where the
      mode is set, plus a test asserting it equals
      `f32::from(Scrollbar::width())`. Verify that the hidden function is
      reachable through `gpui_kit`; if it is not, keep the constant with a
      comment naming the source line and no test. Step 2 reuses the constant.
    - Tree (`tree_pane` / `tree_row` in `crates/native/src/main.rs`): the
      `uniform_list` carries `px_1()` (4 px) and each `ListItem` row `pr_2()`
      (8 px), so the content's right edge is 12 px from the pane's edge today.
      Raise the row's right padding so list padding + row padding equals the
      track's width (row `pr` 12 px), reserved always. The list's `px_1()`,
      the wrapper carrying `vertical_scrollbar(&self.tree_scroll)` and the
      row's `w_full()` stay as they are (see the rule "a tree row takes
      `w_full()` and the `uniform_list` carries the 4 px inset" in
      `docs/agents/native-app.md`: no margin on the row). The row's hover and
      selected fill then still runs to 4 px from the edge, under the thumb.
    - By the numbers above the resting thumb (4-10 px) should not reach the
      badge (12 px) today, yet the user's screenshot shows the badge's right
      edge covered. This was not resolved at planning time. Do not tune the
      padding to the thumb; clear the whole track, and put "measure the gap
      between badge and thumb" in the pending on-screen checks.
    - Meta pane (`meta_pane` in `main.rs`): the pane has `p_3()` (12 px) and
      its first child is `body.overflow_y_scrollbar()`, so the bar sits 12 px
      in from the pane's edge, over the values. `Scrollable` keeps padding on
      the scrolled content and moves only size and flex to its wrapper
      (`root_style_from` in gpui-component 0.9.1 `src/scroll/scrollable.rs`),
      and the bar is drawn at the wrapper's edge. Drop the pane's right
      padding (`pl_3()` + `py_3()`), give `body` a right padding of the
      track's width, and give each note and error `div` under the body
      `pr_3()` so they stay where they are. The values then end 16 px from
      the pane's edge instead of 12 px.
    - Dialog bodies: no file in `crates/native` calls `overflow_y_scrollbar`
      except `meta_pane`; the dialogs scroll in gpui-component's `Dialog`
      (`src/dialog/dialog.rs` around line 826: a full-width
      `div().flex_1().overflow_hidden()` holding
      `v_flex().size_full().overflow_y_scrollbar().pl(paddings.left).pr(paddings.right)`).
      `shell` sets `px_4()`, 16 px at the 16 px rem, so the body's content
      already ends exactly where the track begins. Add one comment line to
      `shell` in `crates/native/src/modal.rs` saying its `px_4()` is also the
      body scrollbar's room. If re-reading shows the padding is not on the
      scrolled content, record it in `learnings.md` and fix it in this step.
    - Do not add a per-bar mode or style at the call sites; all bars keep
      following the theme.
    - `docs/agents/native-app.md` passages to update:
      - "The strip, the tree and the meta pane draw the theme's scrollbar":
        the tree and meta pane bullets, and the bullet "Every bar stays an
        overlay (no gutter; track 16 px) ... The user accepted these covered
        edges on 2026-11-17" (after this step only the strip's part of it
        remains, until Step 2), with the user's 2026-11-17 reversal and the
        constant's name; add that the dialog bodies' room is the shell's
        `px_4()`.
      - The tree rows rule that names the `px_1()` inset (near "The padding
        is on the list, not the pane, so the scrollbar stays at the pane's
        edge"): add the row's right padding and why.
      - Keep the "Not verified by hand yet" note style and point it at this
        plan's `learnings.md`.
    - `CLAUDE.md`: in the `src/theme.rs` description, add a clause for the
      track-width constant if it lives there. Touch nothing else in that
      paragraph.
    - Pending on-screen checks to record in `learnings.md` (Windows first):
      with an overflowing tree, no badge or name under the thumb at rest,
      hovered and dragged, and the measured gap between badge and thumb; a
      press on a badge's right edge selects the row; a short tree looks
      unchanged apart from 4 px more room right of the badges; the meta
      pane's bar is at the pane's edge with no value under it, and the notes
      under the body have not moved; a long Import… or trash dialog body has
      no text under its bar; Neutral dark, Neutral light and one vendored
      theme. These replace the "tree rows' right 16 px" half of check 4 in
      the `native-scrollbars-always` learnings and its `todo.md` item
      ("Native: real-device checks of the always-visible scrollbars").
- [x] Step 2: Give the strip a gutter under its cells for the horizontal bar
  - Done when:
    - The strip's pane is taller than its cells' height by a fixed gutter, so
      the bar's 16 px track lies below the cells' name band: no cell, name or
      burst band lies under the thumb, at rest, hovered or dragged, and a
      press on a name band reaches the cell.
    - The cells' geometry at a given `stripHeight` is unchanged
      (`strip::Geometry::of` and its tests untouched; `stripheight::DEFAULT`
      still gives the 144 x 96 image box), and a stored `stripHeight` means
      what it meant before.
    - A drag of the strip's top edge still follows the pointer, and the
      preview keeps `PREVIEW_MIN` with the gutter counted; `stripheight.rs`'s
      unit tests cover both.
    - `docs/agents/native-app.md` and `CLAUDE.md` describe the gutter, with
      no statement left that the strip's bar lies over its bottom edge.
    - `learnings.md` records the on-screen checks as pending.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (the track-width constant).
    - Today (`crates/native/src/strip.rs`, `paint_strip` and
      `paint_burst_band` in `main.rs`): the cells start `STRIP_PADDING` (7 px)
      below the strip's top and end 7 px above its bottom; a burst band
      reaches 4 px past the cell's bottom, to 3 px above the strip's bottom.
      The bar's parent is the strip `div` (`.h(px(strip_geometry.height))`
      with `.horizontal_scrollbar(&self.strip_bar)`), so the track is its
      last 16 px and overlaps the name band's bottom 9 px.
    - Gutter: the track's width less `STRIP_PADDING`, 9 px, reserved
      always. The track then starts exactly at the cells' bottom edge; the
      hovered thumb's top (12 px above the pane's bottom) meets the burst
      band's bottom (3 + 9 = 12 px) without overlapping it.
    - Shape: a constant in `crates/native/src/stripheight.rs` (the pane's
      extra height), the strip `div` in `Riffle::render` made
      `geometry.height` plus the gutter high, and `stripheight::dragged`
      taking the gutter off `wanted` and off the room left by `PREVIEW_MIN`.
      The canvas and the tooltip regions are positioned from the strip's top,
      so they need no change; `StripBar`, `Strip`, `strip_handle` and the
      stored `stripHeight` (still the cells' height, written by
      `end_pane_drag`) stay as they are. Check every other reader of
      `strip.geometry().height` in `main.rs` for whether it means the cells
      or the pane.
    - The in-flight branch `feature/native-strip-header` (not on `main` at
      planning time) does the same thing for a `stripheight::HEADER` row: the
      pane is `HEADER` taller than the cells and `dragged` subtracts it. If
      it has merged, add the gutter to that arithmetic (pane = header + cells
      + gutter) and to its menu-height computation that uses
      `stripheight::HEADER + strip_geometry.height`; if it has not, expect
      one of the two PRs to rebase over the other in `stripheight.rs`
      (`dragged` and its tests) and in the strip `div` of `render`.
    - With the content fitting, no bar is drawn and the gutter is an empty
      band; a press in it reaches the strip's own `on_mouse_down`, which
      selects by `x` only, as a press in the bottom padding does today.
    - `docs/agents/native-app.md` passages to update: the strip bullet and
      what is left of the "Every bar stays an overlay" bullet in "The strip,
      the tree and the meta pane draw the theme's scrollbar" (it also says
      "the strip's height (`stripheight.rs`) and the layout are unchanged by
      it"); the `stripheight.rs` section (near "`stripheight.rs` holds the
      strip's height in logical px") for the pane's height and `dragged`;
      the mention of `STRIP_PADDING` being 7 px at every height only if it
      becomes wrong.
    - `CLAUDE.md`: the `src/stripheight.rs` clause ("the strip's height
      (`DEFAULT` 128 px within `MIN` and `MAX`, ...; `dragged`, ...)"): add
      the gutter.
    - Pending on-screen checks to record in `learnings.md`: with a long
      folder, the bar sits below the names with nothing under the thumb at
      rest, hovered and dragged, a burst's band included; a press on a name
      band selects the cell and a right press opens its menu; a short folder
      shows an empty 9 px band and no jump in the preview's height when the
      folder changes; dragging the strip's top edge follows the pointer and
      stops at `MIN`, `MAX` and the preview's minimum; the 1:1 crop is re-cut
      after a drag as before. These replace the "strip's bottom edge" half of
      check 4 in the `native-scrollbars-always` learnings and its `todo.md`
      item.

## Trade-offs and risks

1. **How much to reserve.** The whole 16 px track (chosen) rather than the
   thumb's 12 px: no press meant for content is taken by the track, and it
   does not depend on the unresolved gap between the source's numbers and the
   screenshot. Cost: the tree's content and the meta values end 4 px further
   in, and the strip's pane is 9 px taller.
2. **Always, not only while overflowing** (chosen): the layout never moves;
   the room is spent even when no bar is drawn.
3. **Strip: grow the pane** (chosen) rather than shrink the cells: cells,
   `Geometry::of`, `MIN` / `MAX` / `DEFAULT` and stored heights keep their
   meaning; the preview loses 9 px.
4. **Tree: pad the row** (chosen, 8 -> 12 px) rather than the list: the hover
   / selected fill still runs to 4 px from the pane's edge, under the thumb.
5. **Other bars are left as overlays**: the `Sort ▾` / `Filter ▾` popovers,
   `PopupMenu`, `Select` lists and the settings modal's lists.
6. **Ordering against `feature/native-strip-header`.** Both change
   `stripheight::dragged`, its tests and the strip `div`. Whichever merges
   second rebases.
7. **The preview is 9 px shorter while the strip is shown.** Any recorded
   `--auto` latency baseline was taken on the taller canvas; not measured.
8. **A gpui-kit bump can change the track's width or the thumb's inset.**
   The constant's test catches the width; the inset is only documented.
9. **Not verifiable by the agent on screen.** Both steps tick their checkbox
   on the automated criteria and leave the visual checks pending in
   `learnings.md`.

## Progress

- (2026-11-18) Step 1 complete
- (2026-11-18) Step 2 complete
