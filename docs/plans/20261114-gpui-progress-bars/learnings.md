# Learnings: gpui-progress-bars

## Step 1

- `Riffle.trashing` now holds `(trash::Mode, done, total)` instead of the
  note's text; the pane formats the note with `trash::progress_status` where
  it is drawn, so the text is unchanged, and the bar reads the same counts.
  The late `TrashProgress` after the run came back stays ignored by the
  existing `is_some` check.
- The bar is an `absolute` child of the status row (made `relative`), so the
  row's flex layout and the canvas's `viewport` cell never see it.
- A scan with nothing to do still starts (`scan_status.started(0)` in
  `opened`, then `extracted` gives pass 2's `(0, 0)`), so a warm folder shows
  the empty 2 px track for as long as the two empty passes take (a few
  frames at most), then it disappears with `FacesDone`. Left as is: the
  Tauri bar does the same at a total of 0.
- `cargo` is not on the Git Bash `PATH` here; run the crate's tests through
  `mise run ci:local`.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the user's): open a cold folder of 500+ RAW
  files in `riffle-native`; the 2 px bar appears on the status row's top
  edge, fills through pass 1, starts empty again and fills through pass 2,
  then disappears, while the status line's `scanning d/t` / `analyzing d/t`
  read as before. Then select many files, `Move to Trash…` with the right
  pane hidden (`toggleRight`): the bar shows the run's progress and goes when
  it ends; repeat with the strip and the left pane hidden. Step 1's checkbox
  was ticked on the automated criteria (unit tests, `mise run ci:local`);
  the GUI was not run here. Basis: plan Step 1's Done-when (the hand check);
  files `crates/native/src/main.rs` (`progress_bar`),
  `crates/native/src/progress.rs`.
