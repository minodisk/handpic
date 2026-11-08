# Learnings

## Step 1

- The synthetic threshold test passes at 100 px on purpose. A 5x5 box blur is
  a 5 px ramp, and only the upscale to 154 px spreads it far enough to fall
  under `SOFT_BELOW`: sharp 0.528, blurred 0.150 at 100 px. Back-of-envelope, the
  same blur on a crop of 154 px or more keeps a slope of about 0.2 of the
  contrast, over the threshold, so changing the crop size in that test can
  break it without the metric being wrong.
- No JPEG size helper existed in `riffle-core`, and K has to be known before
  the crop is asked for, so `partial::jpeg_width` reads the header alone.
  `decode_region`'s panic guard became the shared `guarded`.
- `Cue::preview_width` comes from `Detection::width`, documented as the stored
  preview size; the whole-image path passes its detection through the same
  `cue_of`.
- `Landmarks::of` in `facechoice` now returns `Option`, so a non-finite mesh
  point dumps `-` where it used to dump NaN.
- An eye's square clamped at the image edge is resized as a non-square cut,
  where the prototype's PIL `crop` padded it with black. Good frames have
  `edge_gap >= 0.02`, which covers the eye region but not the whole
  `0.6 * d` square, so a few frames near the edge may differ from the
  prototype; Step 2's rank check is where that would show.

## Step 2

- Claude Code killed the first three-folder `facechoice` dump (24 threads by
  default, in the background) when the system ran critically low on memory
  while the session sat idle. 2026-11-07 had finished; the other two were
  re-run one at a time with an explicit 8 threads (281 s and 145 s) and
  completed. The thread count does not change the scores.
- The ranks match the prototype (Spearman 0.9994-0.9996, cut ranks within one),
  so 0.162 stays. See `validation.md`. Step 1's concern about crops clamped at
  the image edge did not show: no Good frame differs by more than 0.009.
- The ranks are of the Good frames as the TSV re-derives them (903 / 473 / 303,
  equal to the index's counts).

## Step 3

- Cost of pass 2 on `D:\Photos\2026\2026-11-07` (3638 ARW), from the
  `analysis_ms` column of `riffle-cli features` with 8 threads, release build,
  one run before and one after, on a file cache warmed by the earlier runs:

  | Files | n | Before mean / median / p95 (ms) | After mean / median / p95 (ms) |
  | --- | --- | --- | --- |
  | All | 3638 | 91.2 / 94.9 / 119.6 | 132.0 / 140.7 / 182.1 |
  | `Candidate` (measured) | 2949 (81%) | 95.1 / 97.1 / 120.8 | 146.1 / 147.0 / 184.8 |
  | Others (not measured) | 689 | 74.2 / 67.9 / 110.7 | 71.5 / 65.6 / 109.4 |

  A measured frame costs about 51 ms more (mean and median of the per-file
  difference), under the prototype's 100-170 ms, so every `Candidate` stays
  measured. The whole `features` run went from 80 s to 98 s. `eye_focus` is
  identical on all 3638 files. The measurement was on 8 threads rather than
  the plan's one (a single-thread run of the folder does not fit a foreground
  command), and the cache was warm, so the cold-disk read is not covered.
- A fixture of every older schema version has to drop the new column, v19 to
  v22 included: their `DROP COLUMN edge_gap` did not exist to key a regex on, as
  those versions already had `edge_gap`.
- `rank::Frame` carries `full_eye_sharp` as the plan's `rank_rows` line asks,
  but nothing in the ranking reads it yet.

## Step 4

- `applyFaceReady` in `focus.ts` copies the `faces-progress` fields one by one,
  so the plan's "the new field rides along" needed one more line there: without
  it a frame scanned while on screen would stay Good until the folder reloads.
  `meta.ts`'s `FocusCue` and `main.ts`'s `Focus` needed the field too.
- The `SOFT_BELOW` parity test pins the value as `CANDIDATE_PERCENT`'s does: it
  fails when the Rust constant moves and its doc points at `focus.ts`; it does
  not read the TypeScript.

## Step 5

- Closed Step 4's deferred item: `CLAUDE.md`'s `src/focus.ts` description of
  `photoTier` now names the full-size cut.
- The docs' "about 20% of frames" and the 180-frame figures predate the
  full-size check, which only lowers the share, so both are now worded as
  taken before it (the 54 of 58 may be lower now). Re-measuring it is not part of this step.

## Deferred issues (todo candidates)

- `good_photo` in `crates/core/src/rank.rs` deliberately leaves out the
  full-size cut Step 4 added to `photoTier`, so its features do not move in
  this feature; its doc comment says so. The Rust copy therefore disagrees with
  the frontend on a demoted frame, and applying the cut to the ranking is a
  separate piece of work (it moves the ranking's features, so the prior's
  calibration needs a look). `rank::Frame::full_eye_sharp` is already carried
  through `Index::rank_rows` for it, unread. Found in Step 3, decided in Step 4.
- Pending manual check (Step 4): on Windows, in the real app, after a rescan of
  `D:\Photos\2026\2026-11-07` (`FACES_VERSION` 10 re-runs pass 2), open
  `_DSC1508.ARW` and `_DSC1536.ARW`. Expected: neither is Good (no face icon on
  the strip, a white focus mark, and filtered out by `Face focus` = Good), and
  the meta pane's `Eyes at full size` row reads `Soft`. Step 4's checkbox was
  ticked on the automated criteria; this check is the user's.
- The cold-disk cost of the second read is unmeasured: Step 3's figures are on
  a warm file cache (`crates/core/src/scan.rs`, `full_eye_sharp`). The plan's
  "Cost" trade-off names this as the case to watch; a pass 2 on a freshly
  mounted card or after a reboot would show it.
