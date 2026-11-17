# Learnings

## Step 1

- The geometry lives in `strip::Geometry` (`Geometry::of(height)`), held by
  `Strip` itself, so the view needs no height field of its own: every
  rebuild passes `self.strip.geometry()` to `Strip::new`, the drag reads the
  current height from it, and the write at the drag's end takes it from
  there. The fixed `CELL_PITCH` / `CELL_WIDTH` / `CELL_HEIGHT` / `BOX_*` /
  `STRIP_HEIGHT` constants are gone; `strip.rs`'s tests keep test-local
  constants with the old numbers (154, 146, 114, 96), pinned to
  `Geometry::default()` by `the_default_geometry_is_the_tauri_strips`, so the
  existing expectations are unchanged.
- The anchor chosen for a height change: the current cell's center keeps its
  on-screen x when the current cell overlaps the viewport, else the cell at
  the viewport's left edge keeps its fraction there (offset scaled by the new
  pitch over the old); then the offset is clamped. Pinned by the three
  `a_new_height_*` tests.
- The drag payload is a type of its own, `StripDrag`, rather than
  `PaneDrag(Panel::Strip)`, so the root's `drag_pane` needs no routing.
- `on_drag_move` sits on a new wrapper `div` (`flex_1`, `min_h_0`,
  `flex_col`) around the middle row and the strip. Confirmed in gpui-pre
  0.3.12's `elements/div.rs`: the listener runs in the capture phase whenever
  `cx.active_drag` holds the payload type, with no hover check, and
  `DragMoveEvent::bounds` is the listener element's hitbox bounds, so the
  wrapper's bottom is the strip's bottom edge and its height what the
  preview and the strip share. The wrapper re-indents the whole middle row
  in `render`, which makes the diff large but changes nothing else in it.
- The end of the drag reuses `end_pane_drag` (the root's
  `capture_any_mouse_up` / `on_mouse_up_out`), which now also takes
  `strip_dragged` and writes `stripHeight`.
- A miscount caught by the tests: at `MIN` the cell is 98 px wide, so the
  pitch is 106, not 104.

## Deferred issues (todo candidates)

- **Pending manual check** (native strip resize, Windows first, then macOS
  and Linux): the Step 1 checkbox was ticked on the automated criteria (unit
  tests, `mise run ci:local`); nothing below was observed in the running
  app. Basis: Step 1's Done-when "What was not observed in the running app
  ... is listed as a pending manual check". Files:
  `crates/native/src/main.rs` (`strip_handle`, `StripDrag`, `drag_strip`,
  `end_pane_drag`, the wrapper `div` around the middle row and the strip in
  `render`), `crates/native/src/stripheight.rs`,
  `crates/native/src/strip.rs` (`Geometry`, `Strip::set_geometry`). Steps
  and expected results:
  1. `mise run native:dev` on a folder. Hover the strip's top edge: the
     cursor turns row-resize and the handle's 1 px line reads well on the
     theme; clicks on the cells and the scrollbar still work.
  2. Drag it up and down: the strip follows the pointer between 96 and
     302 px, the thumbnails scale with it, the preview never gets under
     240 px, and the cells that were in view (the current one when on
     screen) stay in view. Note whether the edge jumps a few px at the first
     move.
  3. At 96 px: the stars, the flag dot, the burst badge, the face icon and
     the file name are legible on the 96 x 64 boxes. At 302 px: the
     thumbnails' sharpness at 100 % and on a HiDPI display.
  4. Release the drag over the preview, over a pane, over the status row and
     outside the window; restart: the height comes back each time (for a
     release outside the window, note whether it is written at once or only
     at the next click inside).
  5. `z` for 1:1, then drag the edge: the crop is re-cut once the drag
     settles. `toggleStrip` hides the strip and its handle and brings both
     back at the dragged height.
  6. `--auto`: the strip starts at 128 whatever is stored and a drag writes
     nothing to `stripHeight`.
