# Learnings: gpui-empty-state

## Step 1

- The clear on failure was already right and needed no change: `loaded`'s
  `Err` arm for the current file calls `clear_image()` and records
  `Shown::Failed` at that index; `pager::finish` gives `Stale { draw }` only
  for a successful read and only while the pager has moved off the shown
  index, so a stale frame lands only after a page turn (when the message is
  already gone); the prefetch ring never holds a failed read
  (`prefetched` keeps only `Ok`); `zoom::arrived` drops a failed crop.
- `refilter` moves `shown`'s index through `filter::shown_after` and the
  pager lands on the anchor, so a failed file kept by a refilter stays the
  current one with its message; for one hidden by it `shown_after` gives
  `None`, which matches no current file, so the message goes.
- Wrapping the preview canvas in a `relative` div with the canvas's old
  `flex_1` / `h_full` (plus `min_w_0`) and the canvas `size_full` keeps the
  canvas's bounds as they were, so `viewport`, the fit, the 1:1 crop and the
  perf band are unchanged.
- GPUI's `on_click` fires for the primary button only (`on_aux_click` takes
  the others), so the no-folder message needs no button check.
- The session could not run the native app (no GUI), so the message layer
  and the click were not checked by hand.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the native app `crates/native`): the empty
  state of the preview area, from
  `docs/plans/20261114-gpui-empty-state/plan.md` Step 1, whose checkbox was
  ticked on the automated criteria (unit tests, `mise run ci:local`).
  Related files: `crates/native/src/empty.rs`, `crates/native/src/main.rs`
  (`render`, `empty_message`). Steps and expected results:
  1. Launch without a folder (`--settings` on a fresh store, no
     `lastFolder`): the preview area shows "Click here to choose a folder. Or
     press ctrl+o." centered in muted text; a left click on it opens the OS
     folder picker; rebinding `open` in the Keyboard Shortcuts tab changes the
     key clause at once (`space` shown as `Space`), and unbinding it removes
     the clause. A click on the settings modal over it does not open the
     picker.
  2. Open a folder without RAW or JPEG files: "This folder has no RAW or
     JPEG files." shows; a click does nothing.
  3. Set a filter that hides every file: "No files match the current
     filter." shows.
  4. Page from a normal NEF onto `D:\Photos\samples\NEF\NIKON_D70_Nikon.nef`:
     the previous image leaves the screen and "The preview of this file
     could not be shown." shows; paging on removes it, paging back shows it
     again; `z` on that file hides it.
  5. With a folder open, the preview still fits the area and 1:1 still
     centers on the focus point (the canvas's bounds did not change).
