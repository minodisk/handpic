# Learnings

## Step 1: windowed sharpness luma

- `sharpness::region` now returns the window `score_gray` scores (or
  `Region::Tiles`) and holds the `w < 3 || h < 3` error, so `score_rgb` and
  `score_gray` share the rule. `score_rgb` scores the window's luma at origin
  (`Window { x: 0, y: 0, ..window }`) with the window's width as the stride;
  `window_luma` reuses `candidate::luma` row by row, so the formula is not
  duplicated.
- `score_gray` is no longer used by the non-test code of `scan.rs`; its
  import moved into the test module.
- The real-file check (one release CLI run, one thread, a scratch
  subcommand, not committed) gave bit-identical scores on 11 files; see
  `luma-window.md`. All six ARW of the spot-check list were `eye_af=1`, so
  the plain AF `WINDOW` path's real-file evidence is the three X-series RAF
  files.
