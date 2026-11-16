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

# Resize the thumbnail strip's height by dragging (native app)

## Purpose

In the GPUI app (`crates/native`) the left and right panes can be resized by
dragging their boundary with the preview, but the thumbnail strip along the
bottom has a fixed height of 128 px (`strip::STRIP_HEIGHT`). After this work
the boundary between the preview and the strip can be dragged too: the strip
gets taller or shorter within a minimum and a maximum, its thumbnails scale
with it, the preview keeps a minimum height, and the height comes back at
the next launch. The Tauri app (`crates/app`) is frozen and is not changed.

What the investigation found (the facts the step relies on):

- The strip's geometry is all constants in `crates/native/src/strip.rs`:
  `CELL_PITCH` 154, `CELL_WIDTH` 146 (`BOX_WIDTH` 144 + 2), `BOX_HEIGHT` 96,
  `NAME_HEIGHT` 16, `CELL_HEIGHT` 114, `STRIP_PADDING` 7, `STRIP_HEIGHT` 128.
  The `Strip` model (`content_width`, `scroll_into_view`, `range`,
  `hit_test`, `next_to_request`), `carried_offset`, `tip_regions`, the tests
  of `strip.rs` and `stripbar.rs`, and `main.rs` (`strip_cells`,
  `strip_tips`, `paint_strip`, `paint_burst_band`, the strip `div`'s
  `.h(px(strip::STRIP_HEIGHT))`) all read them.
- The strip is one row of cells on a `canvas`, so a taller strip can only use
  its height by drawing larger cells. A thumbnail is painted with
  `fit_centered` into the image box, so it follows a larger box with no
  change. The stored thumbnails are about 404 x 270 px
  (`crates/core/src/scan.rs`), so a box taller than 270 logical px upscales
  them.
- The side panes' pattern (`crates/native/src/panewidths.rs`, the
  "The side panes' widths are one `PaneWidths`" section of
  `../../agents/native-app.md`,
  `../_archived/20261115-native-pane-resize/learnings.md`): a pure model
  with `DEFAULT` / `MIN` / `MAX` / `PREVIEW_MIN`, `from_setting` /
  `setting`, `dragged` (rounded to whole px so the stored JSON stays
  integral), the standalone `resize_handle` with a payload type drawn as
  nothing (`PaneDrag`), the root `div`'s `on_drag_move`, and the write at the
  first button release after a move (`pane_dragged`, `end_pane_drag`,
  `capture_any_mouse_up` + `on_mouse_up_out`), gated by `remember_panels`
  (off under `--auto`).
- gpui-base 0.9.1's `resize_handle` supports `Axis::Vertical` with
  `.inside(HandleEdge::Leading)`: a band over the container's top 5 px,
  full width, with the row-resize cursor and a 1 px line on the top edge
  (`resizable/resize_handle.rs`). The band occludes what is under it.
- `DragMoveEvent<T>` carries `bounds`, the bounds of the element the
  `on_drag_move` listener is on (gpui-pre 0.3.12, `elements/div.rs`), and
  the listener runs for every move while a drag of that payload type is
  active, wherever the pointer is.
- The strip's bottom edge is not the window's: the status row (a
  gpui-component `StatusBar`, whose height is not a constant in the code)
  and, on Linux, the client-side frame's padding sit below it.
- `strip_moved` (thumbnails released and requested for the new range, the
  scan's hot list) and `zoom_resized` (the 1:1 crop re-cut) are what a
  change of the strip's range and of the preview canvas's size already call.

## Steps

- [x] Step 1: Make the strip's height draggable, scale its cells with it and
      persist it
  - Done when:
    - Dragging the strip's top edge changes its height between the model's
      `MIN` and `MAX`, never leaving the preview less than its minimum
      height during the drag; the cells (image box, cell width, pitch) are
      derived from the height, and at the default height every number equals
      today's (pitch 154, cell 146 x 114, box 144 x 96, strip 128).
    - Hit testing, the on-screen and kept ranges, the request order,
      `scroll_into_view`, `carried_offset`, the tooltip regions, the burst
      band and the scrollbar's content width all use the current geometry,
      and a height change keeps the cells that were in view in view (the
      offset is carried, then clamped) and runs `strip_moved` and
      `zoom_resized`.
    - The height is written to the settings store's `stripHeight` key once at
      the drag's end and read at startup (missing, non-number or non-finite
      means the default, out of range is clamped); under `--auto` the strip
      starts at the default and nothing is written.
    - Unit tests cover the pure parts: the stored value's fallback, clamp and
      round trip; the drag (follows the pointer, rounded, `MIN`, `MAX`, the
      preview's minimum); the geometry at the default height and at another
      height; hit test, range and content width at a non-default geometry;
      the carried offset across a height change.
    - `CLAUDE.md`'s description of `crates/native` names the new module and
      the strip's height-derived geometry; `docs/agents/native-app.md` has a
      section for the strip's height and its mentions of the fixed
      `CELL_PITCH` / `STRIP_HEIGHT` are corrected.
    - What was not observed in the running app (the drag itself, the
      handle's line and cursor, the marks on the smallest cells, the
      sharpness of the largest, a release outside the window) is listed as a
      pending manual check under "Deferred issues (todo candidates)" in this
      folder's `learnings.md`, as the pane-resize plan did.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: a new `crates/native/src/stripheight.rs` (the pure model),
      `crates/native/src/strip.rs`, `crates/native/src/stripbar.rs` (tests
      only), `crates/native/src/settings.rs`, `crates/native/src/main.rs`,
      `CLAUDE.md`, `docs/agents/native-app.md`. Do not touch `crates/app`.
    - Model, after `panewidths.rs`: the height in logical px with `DEFAULT`
      128 (today's `STRIP_HEIGHT`), `MIN` 96 (a 64 px high image box), `MAX`
      302 (a 270 px high box, the stored thumbnail's height) and the
      preview's minimum height 240; `from_setting` / `setting` over the
      `stripHeight` key (one number); `dragged`, given the wanted height and
      the height the preview and the strip share, rounded to whole px,
      clamped to `MIN`..`MAX` and to what leaves the preview its minimum,
      never under `MIN`. No `drawn` rule (see Decisions).
    - Geometry: derive it from the height in one place in `strip.rs`. The
      fixed parts stay constants (`STRIP_PADDING` 7, `NAME_HEIGHT` 16, the
      1 px border each side, the 8 px gap between cells, the 3:2 image box):
      box height = height - 2 x 7 - 16 - 2, box width = box height x 1.5
      rounded to whole px, cell width = box width + 2, pitch = cell width
      + 8. The name's text, the stars, the flag dot, the burst badge and the
      face icon keep their fixed sizes; only the image box scales.
    - `Strip` needs the pitch and the cell width at run time. `Strip::new`
      is called on every rebuild of the list, so the view has to hand it the
      current geometry each time; `carried_offset` and `tip_regions` are
      free functions and take what they need as parameters. Keep the
      existing tests meaningful at the default geometry rather than
      rewriting their expectations.
    - A height change with files listed moves the offset so the same cells
      stay in view (scale it by the new pitch over the old one around an
      anchor, then clamp to the new content). The step decides the anchor
      (the viewport's left edge, its center, or the current cell when on
      screen) and pins it with a test.
    - Handle: `resize_handle` with `Axis::Vertical` and
      `.inside(HandleEdge::Leading)` as a child of the strip's `div` (already
      `relative()`), so it spans the window, is hidden with the strip, sits
      in the 7 px top padding above the cells and stays off the scrollbar at
      the bottom. Its payload is drawn as nothing, as `PaneDrag` is. The
      step decides between reusing `PaneDrag(Panel::Strip)` (the root's
      `drag_pane` then has to leave it alone or route it) and a payload type
      of its own.
    - The drag needs two numbers the window's size does not give: the y of
      the strip's bottom edge and the height the preview and the strip
      share. Take them from `DragMoveEvent::bounds` of the element the
      listener is on, not from the window's height less assumed row heights
      (the status row's height is inferred from gpui-component's source and
      not a constant here). One way: put the listener on a wrapper `div`
      around the middle row and the strip, whose bounds give both. Verify
      during implementation that the listener fires for every move while the
      pointer is outside that element, as the root's does.
    - End of the drag: reuse the release listeners the root already has
      (`capture_any_mouse_up`, `on_mouse_up_out`) and the `pane_dragged`
      pattern, writing the height once and only when `remember_panels` is
      on. Do not add an `on_drop` listener (it misses most releases, see the
      pane-resize learnings).
    - `settings.rs`: read the value into `Settings` next to `pane_widths`,
      with a test like `the_pane_widths_are_read_with_their_rule`.
      `Riffle::new` takes it only when `remember` is on, as it does for
      `pane_widths`.
    - Run `strip_moved` and `zoom_resized` and `cx.notify()` only when the
      height actually changed, as `drag_pane` does.
    - `docs/agents/native-app.md`: the places that state the fixed geometry
      today are the scrollbar section ("the cell count times `CELL_PITCH`",
      "`STRIP_HEIGHT` and the layout are unchanged") and the burst band rule
      ("`STRIP_PADDING` is 7 px").

## Decisions (settled with the user on 2026-11-17)

- The thumbnails scale with the height: the cells are sized from it. The
  alternatives were a taller strip with empty space around fixed 144 x 96
  thumbnails, or a multi-row grid (a different feature). Consequence: the
  pitch becomes a run-time value throughout `strip.rs`, and a shorter strip
  shows more cells, so more thumbnails are kept decoded (about 0.4 MB each).
- The height is stored under a new `stripHeight` key holding one number,
  with its own module `stripheight.rs`, not as a `"strip"` entry in
  `panelWidths` (whose name and `dragged` signature are width-shaped).
- A short window keeps the stored height: the preview's minimum is enforced
  during the drag and on read only, and a window made shorter afterwards
  takes the space from the preview, as today's fixed 128 px strip does. No
  `drawn` rule like the side panes' (it would be one frame late or rest on
  inferred row heights, and would re-request thumbnails during a resize).
- The limits: `DEFAULT` 128 (today's strip exactly), `MIN` 96, `MAX` 302,
  the preview's minimum 240. Their look is a hand check.

## Trade-offs and risks

- Only the image box scales; the file name, stars, flag dot, burst badge and
  face icon keep their sizes, so a small cell is denser and a large one
  sparser. Scaling them too is not part of this work.
- At `MIN` the image box is 96 x 64 px, where five stars (about 55 px) and a
  burst badge share the width; at `MAX` a thumbnail is drawn at its stored
  size at 100 % scale and upscaled on a HiDPI display. A higher maximum
  needs larger stored thumbnails, which is out of scope.
- The handle's band occludes the strip's top 5 px (its padding; a burst
  band's top 4 px is drawn there but takes no click) and draws a 1 px line on
  the strip's top edge, where nothing is drawn today. Its look is a hand
  check.
- The first move can shift the boundary by up to 5 px toward where the band
  was grabbed, as with the side panes; not compensated there either.
- If the PR turns out too large to review, the natural split is the geometry
  refactor with no visible change first (constants to a height-derived value,
  still 128), then the handle, the drag and the persistence.
- `docs/humans/usage.md` does not describe the side panes' drag, so it is
  not updated for the strip's either.

## Progress
