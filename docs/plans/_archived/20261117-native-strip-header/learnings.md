# Learnings: native strip header

## Step 1

- `stripheight::dragged` now takes the whole pane's wanted height (what the
  wrapper's `on_drag_move` gives: its bottom less the pointer's `y`) and
  takes `HEADER` (32 px) off it itself, so the caller in `main.rs` did not
  change and the header rule is unit-tested in one place.
- The status row lost its tallest children (the 21 px buttons), so its
  content alone (the 19 px text line, the 12 px circle) would have shrunk
  it. It is pinned on the `StatusBar` itself with `.h(STATUS_ROW_HEIGHT)`
  (30 px: 21 + `py_1` x 2 + the 1 px top border, GPUI sizing border-box)
  rather than by wrapping the status text, since `native-app.md` rules
  "no wrapper around the status text" (the clipping of a long text).
- The popover's height cap is `menu_max_height` less the strip pane's
  height (header + cells), the room above the header, so `snap_to_window`
  should never need to push it over its own button. Not seen on screen.
- The strip pane is a `relative()` `flex_col` wrapper: the header, the cells'
  `div` (unchanged height and handlers) and `strip_handle()` last, so the
  handle's band is the header's top 5 px; the buttons start 6 px down.

## Deferred issues (todo candidates)

- Pending manual check (Windows; macOS and Linux by CI only), from Step 1's
  "What to verify" in `plan.md`; the step was ticked on the automated
  criteria. Files: `crates/native/src/main.rs` (`strip_header`,
  `status_row`, `menu_button`), `crates/native/src/stripheight.rs`. Run the
  native app on a folder and check:
  - The header shows on the strip's top edge with `Sort ▾` / `Filter ▾` at
    its right end; each opens its menu upward from its button, a second
    press closes it, a press outside dismisses it, `escape` closes it, the
    filter button is lit while a filter is active and the sort button dimmed
    in a view-only (JPEG-only) folder.
  - With a long filter menu (EXIF sections), the strip at its tallest and a
    short window, the menu stays inside the window, scrolls, and its button
    still toggles it (the popover is not snapped over the button). The
    popover is not clipped by the header.
  - The handle on the header's top edge: the resize cursor, the 1 px line,
    a drag down to `MIN` and up to `MAX`, the preview stopping at
    `PREVIEW_MIN`; a press on a button is not taken by the handle's band.
  - At a stored `stripHeight` of 128 the cell box is still 144 x 96.
  - A press, right-click or wheel on the header neither selects a cell,
    opens the strip's context menu nor scrolls the strip.
  - `toggleStrip` hides the header with the strip and closes an open menu;
    the status row keeps its height (no jump when the progress circle
    appears or goes) and shows `n / shown (all)` while a filter is active.
  - Re-measure the preview canvas in the default 1600 x 1000 window (expected
    1160 x 810, 32 px shorter than the old 842) and update the "not
    re-measured" notes in `docs/agents/native-app.md`.
  - Done when: the checks are run and any failure fixed; the "inferred /
    not re-measured" notes in `docs/agents/native-app.md` (the preview
    canvas "inferred 32 px shorter", the pre-header 1160 x 842 measurements,
    the status row height "inferred from the sources") are replaced with the
    measured values; this item is closed.
