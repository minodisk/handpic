# Learnings: gpui-bursts

## Step 1

- `mark_strip` already runs at every place the strip is rebuilt or its
  judgments re-read (the open, a rebuilding `refilter`, the entries read
  after its `refilter`), so the burst marks needed only one call there; the
  strip keeps them as a `Vec<Option<BurstMark>>` in cell order, which
  `Strip::new` drops with the rest.
- `group_bursts` needs the lookup twice (the capture order and the time
  parse); passing `&lookup` to `order_files` keeps one closure.
- The `.cell.burst::before` offsets are relative to the cell's padding box
  (inside its 1 px border), not its border box: the band reaches 4 px above
  and below the native cell's outer edge and meets the previous cell's
  right edge exactly across the 8 px gap.
- The strip spans the window from its left edge, so a band left of the first
  on-screen cell would only paint off-window; the band loop is masked to the
  strip's bounds anyway.
- The badge reuses `overlay::HALO` (made `pub`) for its eight dark copies
  rather than the stars' single 1 px copy, closer to `text-shadow: 0 0 2px`.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, from the plan's Trade-off 7; the step's
  checkbox was ticked on the automated criteria): on Windows and macOS, open
  a Sony burst folder (sub-second subsec) and a Leica whole-second burst
  folder in `riffle-native`. Expected: each burst of two or more shows a
  faint band behind its cells, rounded at its first and last cell, joined
  across the gaps, distinguishable from the current cell's outline, the
  selected cell's fill and a failed cell; every member shows `n/size` at the
  image box's bottom-right, dimmed on a rejected member; `arrowup` /
  `arrowdown` jump between bursts and `alt+arrowleft` / `alt+arrowright`
  step within one, also under a filter and a non-capture sort and in a JPEG
  (view-only) folder, and do nothing while the folder tree has the
  keyboard. Files: `crates/native/src/main.rs` (`paint_strip`,
  `move_burst`), `crates/native/src/burst.rs`.
