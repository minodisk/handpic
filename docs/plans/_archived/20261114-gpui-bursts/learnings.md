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

## Step 2

- The plan said the Tauri `sharpness` map is filled from `folder_entries`
  only; it is also patched by `faces-progress` (`applySharpnessReady` in
  `focus.ts`), with or without a focus. `Facts.sharpness` follows both:
  `From<&IndexedFile>` and `apply_face_ready`, which now takes the score for
  any file with facts (a JPEG too), and the `pending_faces` replay after an
  entries read covers the snapshot race as it does for the focus.
- `burst::reject_rest` returns `companion::Change`s so `judge_paths` (one
  entry, `touched`, one anchored `refilter`, a store per change, no
  auto-advance) serves it unchanged; undo and redo needed no change, since
  `step` already has the one-file / several-files rule and the failed-store
  `forget`.
- `companion::View` takes `&HashMap<String, Facts>` for the sharpness rather
  than a closure: `mcp_request`'s `view(this)` returns the snapshot, so a
  closure local to it could not outlive the call.

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
- Pending manual check (Step 2, from the plan's Trade-off 7; the step's
  checkbox was ticked on the automated criteria): on Windows and macOS, in
  `riffle-native` on test copies of a burst folder, put the current file in a
  burst of three or more (one member already rejected, one hidden by a
  filter) and press `shift+x`. Expected: every other member not yet rejected
  turns rejected (its `.xmp` / `.dop` gets the reject flag with the stars and
  label kept, the hidden one too), the current file stays current, nothing
  advances; `Ctrl+Z` restores them with `Undid N files` and leaves the
  current file, `Ctrl+Shift+Z` redoes with `Redid N files`; on a two-frame
  burst the undo moves to the other frame. Nothing happens in a JPEG
  folder or on a singleton. With MCP on, `get_view`'s `burst` lists the
  members with their sharpness and judgments, `visible: false` on the
  hidden one. Files: `crates/native/src/main.rs` (`reject_rest`,
  `mcp_request`), `crates/native/src/burst.rs`,
  `crates/native/src/companion.rs`.
