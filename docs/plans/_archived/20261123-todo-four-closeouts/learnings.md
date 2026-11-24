# Learnings

## Step 1: Trim the trailing spaces of the displayed lens name

- The lens is trimmed at display in `crates/engine/src/exif.rs` (`trim_end`,
  then a now-empty lens dropped), so `EXTRACTOR_VERSION` stays 13 and the
  index keeps the string as read.
- No other consumer shows the index's raw `lens`: `crates/app/src/meta.rs`
  (the `Lens` row) and `crates/app/src/exif.rs` (`Group::Lens`) both read it
  through `exif()`, and `crates/engine/src/mcp.rs`'s `lens` is only the
  filter argument, matched against those labels.
- A bare `python` in Git Bash hangs on this host, as the plan warns; use
  `mise x -- python`.

## Step 2: Eye-midpoint score in `features` and the burst ranking check

- `features` gained `eye_af` and `eye_mid` after `eyes_ms`, computed by
  `eye_mid_features` outside both timers. It reads the preview a second time
  for the shot and the score; that cost is untimed, so `analysis_ms` and
  `eyes_ms` keep their meaning.
- The cue's face is guarded on `FACE_CONFIDENCE` in the CLI, because
  `score_preview` falls to the sharpest tile below it.
- Three Sony folders (2026-09-19, 2026-09-13-a, 2026-09-13-b) gave 69
  qualifying bursts: median Spearman 0.500, same top frame 62%, under the
  Trade-offs 2 bar. Per the approved option (a), the scoring stays; see
  `burst-rank.md`.
- In this Git Bash, backslash escapes in a `sed` pattern or a Python heredoc
  (`\t` inside Rust string literals) did not match reliably; the Edit tool
  did.

## Step 4: A JPEG folder's page time in the app

- Run before Step 3, because `docs/plans/20261123-all-faces-check` Step 1 is
  in progress on the same `crates/core/src/scan.rs` code (Trade-offs 4).
- With the prefetch ring on, JPEG paging at one press a second is served from
  the ring (p50 9-13 ms, about the ARW ruler's). With `--no-prefetch` the 50 MP
  Leica exports take about 115 ms p50 (`decode` about 100 ms at 1/8 scale),
  the 26 MP Sony exports about 50 ms. See `jpeg-preview.md`.
- Fancy upsampling off plus `DctMethod::IntegerFast` did not cut `decode` on
  the JPEGs (96.62 to 99.87 ms median): at 1/8 and 2/8 the time is the
  entropy decode, which no decoder setting changes.
- A fresh index on a large folder makes the scan's reads contend with the
  page reads (`read` 50-75 ms); finish the scan, or discount those runs,
  before comparing.
- Decision (user, via the caller): Trade-offs 5 (a). Warm paging is judged
  by the default prefetch-on figures (p50 9-13 ms, under 65 ms), so no code
  change; the todo item is removed, and the ring-miss cost (about 115 ms p50
  on the 50 MP Leica exports with `--no-prefetch`) is recorded next to them
  in `docs/humans/performance.md` / `.ja.md`. The step stopped once before
  committing to ask, because the `--no-prefetch` figure, the one comparable
  to the 65 ms RAF number, is above the bar.

- Step 4 follow-up: the review thought the runs' `page` lines were lost and
  derived the scales from `fitscale::scale_for`; the raw outputs were in the
  session scratchpad after all. The logged `decoded=` sizes put every JPEG
  page at 1/8 or 2/8 (not 1/8 to 3/8 as derived) and the median 26 MP Sony
  export (portrait) at 1/8, not 2/8, so a derived scale is no substitute for
  the logged field. Keep a measurement's raw output next to the plan (or
  summarize its sizes) before the scratchpad goes. `exiftool -EncodingProcess`
  on the 10-file folder: all 10 `Baseline DCT, Huffman coding`.
- The 10-file folder's files measure 36.1-50.1 MP (exiftool and the `px=`
  fields alike), while the step's table and `performance.md` / `.ja.md` gave
  the Leica row as 37-60 MP; corrected to 36-50 MP in all three.

## Step 3: Score a faced no-AF file from the cue's RGB decode

- Adopted (Trade-offs 3): median per-file saving 13.4 ms (`2026-02-01`) and
  16.2 ms (`2026-09-05`), per-burst Spearman 1.000 over 26 bursts, same top
  frame in all. `FACES_VERSION` 12 -> 13 (12 came from #1264, after the plan
  was written). See `no-af-decode.md`.
- The Spearman of 1.000 is trivial on these folders: on the Leica M11-P DNG
  previews `candidate::luma` of mozjpeg's RGB output equals its grayscale
  (Y) output on every pixel, so not one score changed. On ARW, CR3, NEF,
  RAF and ORF previews the two differ on about half the pixels (score
  0.1-1.1% off), but no real burst of faced no-AF frames of those makes
  exists on this host, so their ranking is unchecked.
- `analysis_ms` per file is noisy (p10 of the saving negative) because it
  includes `full_eye_sharp`'s full-JPEG read and decode; compare medians of
  per-file medians over alternated runs, not single files.
- The first `features` run of `2026-09-05` took 423 s against 119-140 s for
  the next ones: the 411 DNGs' full JPEGs were not in the page cache yet.
  Do the warm-up run before the timed ones.
- `whole_image_cue` now hands back `(Cue, Option<f64>)`; the score is taken
  inside the cue's `guarded` closure right after the decode, so a later mesh
  panic keeps the score, and a decode failure (no score) falls back to
  `score_preview`'s own decode.

## Deferred issues (todo candidates)

- Core: the within-burst sharpness ranking of face-tracked Sony frames differs
  between the AF-frame window (today's stored `sharpness`) and the eye-midpoint
  window it replaced: over 69 bursts of 2026-09-19 / 2026-09-13-a / -b, median
  per-burst Spearman 0.500, p10 -0.420, same top frame in 62% (bar: 0.9 and
  80%). Kept as is per Trade-offs 2 (a); decide whether the score that ranks
  frames within a burst should return to the eye-midpoint window (with a
  `FACES_VERSION` bump) or whether the burst ranking should read
  `full_eye_sharp` instead. Basis: Step 2's check,
  `docs/plans/20261123-todo-four-closeouts/burst-rank.md`. Files:
  `crates/core/src/sharpness.rs` (`score`, `eye_af_frame`),
  `crates/engine/src/index.rs` (`FACES_VERSION`).
- Core: a file with a trusted AF point still decodes its preview twice in
  pass 2 (full-size RGB for the cue's crop, grayscale for the score).
  Reusing the RGB's luma as Step 3 does for no-AF files would save a
  grayscale decode (about 4-5 ms on a 1616x1080 ARW preview) per file, but
  on ARW previews the BT.601 luma differs from the Y channel on about half
  the pixels, so it needs its own ranking check on Sony bursts and a
  `FACES_VERSION` bump. Basis: Step 3's scope note in `plan.md` and the
  luma-vs-Y check in `no-af-decode.md`. Files: `crates/core/src/scan.rs`
  (`cue`, `score`), `crates/core/src/candidate.rs` (`focus_cue_unless`),
  `crates/core/src/sharpness.rs` (`score_gray`).
