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

# Demote a `Face focus` Good frame whose eyes are soft at full size

## Purpose

Pass 2 (`run_faces_scan` in `crates/app/src/index.rs`, the cue in
`crates/core/src/candidate.rs`) judges the AF face's eye focus on the
1616x1080 embedded preview. On faces whose eyes are 20-40 preview px apart
the 4.3x downscale hides softness: `eye_focus` is 0.95-1.0 on frames whose
eyes are visibly soft at full size (`D:\Photos\2026\2026-11-07\_DSC1508.ARW`
at 0.976, `_DSC1536.ARW` at 0.948). The user culls by filtering `Face focus`
= Good and keeps seeing soft frames. Good is decided in the frontend by
`photoTier` / `clears` in `crates/app/ui/src/focus.ts` (eye_focus >= 0.9,
eyes_ear >= 0.25, |yaw| <= 35, |pitch| <= 45, eye_offset <= 0.1,
edge_gap >= 0.02); the strip icon, the mark color and the filter all read
`afEyeState`.

The fix: measure the judged face's eyes once more on the full-size embedded
JPEG (JpgFromRaw, `reader::read_full`, which already does a bounded prefix
read plus a ranged read of the JPEG) and take the frame out of Good when that
score is under a threshold. A demoted frame only moves to Neutral; it is not
rejected. The check accepts demoting a few sharp frames (about 3% of the
hand-labeled Good picks, see below) to take most soft frames out of Good, and
under-catching soft ones at high ISO is acceptable.

The metric, prototyped in Python
(`D:\Photos\tests\20261108-good-eye-blur\proto\full_all.py`, `met` / `job`;
per-frame scores `fu` in `m_*.json`) and to be reproduced in Rust:

- For each eye of the judged face, the eye center is the mean of the face
  mesh's `LEFT_EYE_CONTOUR` / `RIGHT_EYE_CONTOUR` points (what `facechoice`
  dumps as `mesh_left_eye` / `mesh_right_eye`), else the YuNet eye landmark
  (`Face::left_eye` / `right_eye`) when the face got no mesh.
- `d` = the distance between the two centers in preview px, floored at 8.
- `K` = full width / preview width (stored, unrotated coordinates on both
  images; `partial::decode_crop` takes unrotated coordinates too).
- In full-res coordinates (center x K), crop a square centered on the eye
  with half-side `0.6 * d * K`, take the luma, resize bicubically to
  154x154 (PIL's bicubic, a = -0.5, which `embed.rs` already ports), take the
  gradient magnitude with `np.gradient` semantics (central differences in
  the interior, one-sided at the borders), and score = the 99th percentile
  of the gradient magnitude divided by `max(p98 - p2 of the resized luma, 1)`
  (numpy's linear-interpolation percentile).
- Frame score = the max over the two eyes. Below the threshold = soft.
  Prototype threshold: 0.162.

Blind validation by the user on contact sheets of Good frames sorted by the
score (`D:\Photos\tests\20261108-good-eye-blur\`):

- `D:\Photos\2026\2026-11-07` (ILCE-7M5, FE 50-150mm F2 GM, mean ISO 3160,
  903 Good): clear frames start at rank ~118; 0.162 falls at rank 117/118.
- `D:\Photos\2026\2026-10-12a` (mean ISO 1401, 473 Good): boundary ~106;
  0.162 falls at rank 106.
- `D:\Photos\2026\2026-09-27-c` (mean ISO 5876, 303 Good): boundary ~121-160
  with sharp and soft mixed in 120-160; 0.162 falls at rank 77 and not one
  sharp frame sits in ranks 1-77. At high ISO the threshold under-catches
  but never demotes a sharp frame.
- Against the 806 hand-labeled frames of the 2026-10-08 mesh-eye-focus sets
  (`D:\Photos\tests\20261108-good-eye-blur\labeled_picks_under_cut.jpg`):
  the 145 frames clearing the Good rule are all picks, and 13 fall under
  0.162. The user judged 5 of those 13 sharp (scores 0.145-0.157) and chose to
  keep 0.162 over 0.15 (2 sharp) and 0.14 (0 sharp), which would leave many
  more soft frames in Good.
- Six noise-robust variants (half-size resize, Gaussian s1/s2, median 3/5,
  p99.5) did not align the three boundaries better; the simple metric stays.
- Prototype cost: 0.1-0.17 s per frame with a whole-JPEG decode; the Rust
  port decodes only the two eye crops (`partial::decode_crop`).

Design settled here (the reasoning is in "Trade-offs and risks"):

- The score is a new value, `full_eye_sharp`, next to `eye_focus`; it does
  not fold into `eye_focus`, which is a calibrated probability that also
  decides Low.
- Pass 2 measures it for every frame whose cue is `FocusCandidate::Candidate`
  (not only Good-eligible frames), so the Good rule keeps living in
  `focus.ts` alone. If Step 3's cost measurement says otherwise, switch to
  Good-eligible frames only (see Trade-offs). The user chose this option at
  plan approval (2026-11-08).
- No full JPEG, a full JPEG no larger than the preview, an HEVC full (an HDR
  PQ CR3), a read or decode failure: `None`, stored `NULL`, and the frame is
  not demoted. Only a present value under the threshold demotes.
- The full JPEG's read goes through the scan's `ReadSlots` like the preview's
  read, so a preview request pauses it, and the step checks `cancel` around
  it.
- `FACES_VERSION` moves to 10 so every row is re-measured; the column needs
  `SCHEMA_VERSION` 24 with an in-place `ALTER TABLE`.

## Steps

- [x] Step 1: Add the full-size eye sharpness metric to `riffle-core` and the eye centers to `Cue`
  - Done when:
    - `crates/core/src/fullsharp.rs` exists with (names indicative):
      `pub const SOFT_BELOW: f64 = 0.162` (the threshold, re-derived in
      Step 2), `pub fn eye_crop_score(gray: &[u8], width: usize, height:
      usize) -> f64` (the metric of one already-cut crop: bicubic resize
      to 154x154, gradient p99 / contrast), and `pub fn full_eye_sharpness(
      full_jpeg: &[u8], eyes: [(f32, f32); 2], preview_width: usize) ->
      Option<f64>` (K from the full JPEG's own width over `preview_width`,
      `None` when K <= 1.5 or a crop cannot be decoded, the max over the two
      eyes). The crop goes through `partial::decode_crop` (its `Crop.x` /
      `Crop.y` give the MCU-snapped origin, so the eye center is re-mapped
      into the crop before cutting the exact square) and `candidate::luma`.
    - `Cue` carries what the metric needs: the two eye centers in stored
      preview coordinates (the mesh contour centers, else the YuNet
      landmarks; `None` when no face was judged) and the preview's decoded
      width. `cue_of` / `face_cue_unless` fill them; `facechoice`'s
      `Landmarks::of` and `candidate::mesh_eye_offset`'s `center` closure
      compute the same centers, so one shared helper replaces the copies.
    - `embed.rs`'s PIL-port resize (`coefficients`, the two-pass resize) is
      usable on a 1-channel image (a channel-count parameter or a gray
      variant), with its existing RGB tests unchanged.
    - Unit tests in `fullsharp.rs`: (a) a synthetic eye-like crop (a dark
      disc with a sharp edge, or a sharp step edge, 100-200 px) scores
      clearly above the same crop after a 5x5 box or Gaussian blur, and the
      blurred one is under `SOFT_BELOW` while the sharp one is over it;
      (b) the score is invariant to a uniform brightness scale (the
      contrast normalization); (c) `full_eye_sharpness` returns `None` for
      a JPEG no larger than the preview width (K <= 1.5); (d) the gradient
      and percentile helpers match numpy on a tiny hand-computed array
      (one-sided borders, linear interpolation).
    - `CLAUDE.md`'s layout paragraph names `src/fullsharp.rs`.
    - `mise run ci` passes.
  - Implementation approach:
    - Build on `origin/main` (2b7c09f8 or later): `scan.rs` there has
      `ReadSlots`, which Step 3 uses.
    - Floor `d` at 8 preview px as the prototype does; the half-side is
      `0.6 * d * K` in full-res px, clamped into the image by
      `decode_crop` (ask it for a square of side `ceil(1.2 * d * K)` plus a
      margin, then cut the exact square from `Crop` using its `x` / `y`).
    - `np.gradient` on a 2-D array: interior `(a[i+1] - a[i-1]) / 2`, first
      and last rows / columns one-sided `a[1] - a[0]` and `a[n-1] - a[n-2]`;
      magnitude `hypot(gx, gy)`; percentiles with numpy's default linear
      interpolation (`rank = p/100 * (n-1)`, interpolate between floor and
      ceil). Exact float parity with PIL is not required, but the ordering
      must hold (Step 2 verifies).
    - Luma: `candidate::luma` (the same BT.601 integer luma the cue uses).
    - Do not touch `eye_focus`, its logit or `FACES_VERSION` in this step.

- [x] Step 2: Dump the score from `facechoice` and fix the threshold on the three folders
  - Done when:
    - `riffle-cli facechoice` gains two columns after the existing ones:
      `edge_gap` (the mesh's, as pass 2 stores it; `-` without a mesh) and
      `full_sharp` (`fullsharp::full_eye_sharpness` of the face's eye
      centers on the file's full JPEG, read once per file with
      `reader::read_full`; `-` when `None`). The columns are filled for
      every face of both sets so the dump stays one row per face; the file
      is read once and the crop decoded per face.
    - A rank script in this plan folder (Python, run with
      `D:\Photos\tests\.venv-eval`, reading the TSV only) applies the Good
      rule (eye_focus >= 0.9, ear >= 0.25, |yaw| <= 35, |pitch| <= 45,
      eye_offset <= 0.1, edge_gap >= 0.02) to the `crop` set's `nearest=1`
      rows (and the whole set's `judged_face` rows for AF-less files, as
      `af_rule` picks), sorts the Good frames by `full_sharp` ascending,
      and prints the rank of the first frame at or above a threshold and
      the Spearman correlation against the prototype's `fu` in
      `D:\Photos\tests\20261108-good-eye-blur\proto\m_*.json`.
    - `validation.md` in this plan folder records, per folder, the Good
      count, the Spearman against the prototype, the Rust score at the
      prototype's cut ranks (117 / 106 / 77), the rank 0.162 falls at in
      the Rust ordering, and the ranks of `_DSC1508` and `_DSC1536`. The
      threshold `SOFT_BELOW` is kept at 0.162 if it cuts within a few frames
      of 117 / 106 and at or below 77 on 2026-09-27-c; otherwise it is
      re-derived (the value that reproduces 117 / 106 while staying at or
      below 77) and the new figure and its derivation go in `validation.md`
      and the constant's doc comment.
    - `mise run ci` passes.
  - Implementation approach:
    - `HEADER` and `face_columns`' count (24 -> 26) and the empty-set
      placeholder row move together; the `--check` comparison is untouched.
    - Reading the full JPEG per file adds several MB per file to the dump's
      I/O; run with the default thread count, the three folders take
      minutes, not hours.
    - The acceptance figures are among Good frames only (903 / 473 / 303);
      the Good count from the dump should match those within a few frames
      (the index applied the same rule when the user made the sheets).

- [ ] Step 3: Compute and store `full_eye_sharp` in pass 2
  - Done when:
    - `scan::Analysis` gains `full_eye_sharp: Option<f64>`.
      `extract_analysis_unless` measures it after the cue when
      `cue.state == FocusCandidate::Candidate` and the cue has eye centers:
      it reads the full JPEG with `reader::read_full` inside
      `slots.acquire(cancel)?` (the slot held for the read only), checks
      `cancel` after the read, and runs `fullsharp::full_eye_sharpness`
      under `catch_unwind` like `score`; any failure is `None`, never an
      `Err` for the file. It skips the read entirely when the preview's
      `Arw` says there is nothing to gain: `arw.full` is `None`, `arw.hevc`
      is set, or `arw.full == arw.preview` (the same offset: "the preview
      when the file holds nothing larger").
    - `Index`: `files.full_eye_sharp REAL`; `SCHEMA_VERSION` 24 with a
      `(10..24).contains(&version)`-style `ALTER TABLE files ADD COLUMN`
      (follow the `eye_offset` / `edge_gap` migration at v19 and the schema
      doc comment's history list); `FACES_VERSION` 10 with a sentence in
      its doc comment; `faces_todo` NULLs it with the other pass-2 columns;
      `FacesRow`, `write_faces`, the row reader behind `folder_entries` /
      `rank_rows`, `StoredEyes` (`full_eye_sharp: Option<f64>`, serialized
      with the rest) and `FaceReady` carry it; the existing index tests for
      the pass-2 columns extend to it (write, read back, re-extraction on
      version bump).
    - The per-file cost of pass 2 is measured before and after on
      `D:\Photos\2026\2026-11-07` (`riffle-cli features` or `facechoice`'s
      timing on one thread, or the app's `faces-done` log line) and
      recorded in `learnings.md` with the share of files measured.
    - `CLAUDE.md`'s description of `run_faces_scan` and the pass-2 columns
      names `full_eye_sharp`.
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Steps 1 and 2 are merged.
    - The second read is the only new I/O in pass 2; taking a `ReadSlot`
      keeps it behind a preview read (the pause) and under the cap the
      preview-read-gate plan picks. Do not hold the slot across the decode.
    - Pass 2 runs at `Priority::Lowest` on its workers; nothing changes
      there.
    - The whole-image (no trusted AF point) path produces the same `Cue`,
      so it gets the same measurement.
    - `faces-progress` consumers in `main.ts` already refilter on each
      batch; the new field rides along in `FaceReady` via `StoredEyes`.

- [ ] Step 4: Take a soft-at-full-size frame out of Good in the frontend
  - Done when:
    - `eyes.ts`'s `StoredEyes` has `full_eye_sharp: number | null`.
    - `focus.ts`: `export const GOOD_FULL_EYE_SHARP = <SOFT_BELOW>` with a
      comment naming `fullsharp::SOFT_BELOW` and the three-folder validation;
      `clears` adds `(full_eye_sharp === null || full_eye_sharp >=
      GOOD_FULL_EYE_SHARP)`; `AF_EYE_HELP.good` says the eyes are also
      sharp on the full-size image where the file has one, and
      `AF_EYE_HELP.neutral` lists "soft at full size" among the missed Good
      conditions.
    - `meta.ts`: an Analysis row (e.g. `Eyes at full size`) showing the
      stored score to two or three decimals, with `sharp` / `soft` against
      `GOOD_FULL_EYE_SHARP`, only when there is a value.
    - Tests: `focus.test.ts` covers a frame that clears every other cut and
      is `neutral` with `full_eye_sharp` under the threshold, `good` at it,
      and `good` with `null` (unavailable); `AF_EYE_HELP` tests cover the
      new wording; `meta.test.ts` covers the row's presence and absence;
      `filter.test.ts` needs no change unless it builds `StoredEyes`
      literals (then add the field).
    - Manual check recorded in `learnings.md`: after a rescan of
      `D:\Photos\2026\2026-11-07` in the app, `_DSC1508.ARW` and
      `_DSC1536.ARW` are no longer Good (no strip face icon, white mark,
      filtered out by `Face focus` = Good).
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Step 3 is merged (the wire shape).
    - `afEyeState` and the filter read `photoTier`, so no change in
      `filter.ts`, `strip.ts`, Compare or `companion.ts`.
    - Every `StoredEyes` literal in tests (`focus.test.ts`,
      `filter.test.ts`, `meta.test.ts`, `eyes.test.ts`) gains the field;
      search for `edge_gap:` to find them.

- [ ] Step 5: Describe the new Good condition in the user docs
  - Done when:
    - `README.md` / `README.ja.md` (the focus mark paragraph around "bright
      green for a good photo") and `docs/humans/usage.md` /
      `docs/humans/usage.ja.md` (the cuts list, "Bright green marks a good
      photo ...") add the full-size condition: the eyes of the judged face
      are also checked on the file's full-size embedded JPEG and a frame
      whose eyes are soft there is not good; files with no full-size JPEG
      (RAF, ORF, HDR PQ CR3) skip the check; a sentence on the validation
      (three folders; a few sharp frames can leave Good, and it under-catches
      at high ISO). Both pairs updated in the same PR.
    - `mise run ci` (lychee included) passes.
  - Implementation approach:
    - Assumes Step 4 is merged so the text describes shipped behavior.

## Trade-offs and risks

- **Which frames are measured.** Chosen: every `Candidate` cue. Alternative:
  only frames that clear the other five Good cuts, which needs the six
  constants in Rust with a parity test against `focus.ts` (as
  `CANDIDATE_PERCENT` is pinned today) and halves (roughly; Good is 25-30%
  of faced frames, Candidate more) the full-JPEG reads. Switch to the
  alternative if Step 3's measurement shows pass 2 slowing noticeably or the
  preview stalling; the plan's shape does not change, only the condition in
  `extract_analysis_unless`.
- **A new column vs. folding into `eye_focus`.** Folding would move Low
  (`not_candidate`) on frames that are sharp on the preview and soft at full
  size, and lose the preview probability. The column keeps both values
  visible in the meta pane and lets the threshold change without a pass-2
  rerun of the whole cue.
- **Threshold parity.** PIL's bicubic, libjpeg's decode of a cropped region
  (not bit-identical to a whole decode, see `docs/agents/tauri-app.md` on
  `jpeg_crop_scanline`) and numpy's percentile each move the Rust values a
  little. Step 2 therefore checks ranks, not values, and may re-derive the
  constant; the hard rule is the 2026-09-27-c cut staying at or below rank
  77.
- **One body calibrated.** The three folders are all ILCE-7M5 (K = 4.34).
  NEF, CR3 (JPEG track), DNG and other Sony sizes get the same threshold
  untested. The crop side scales with `d * K` and is resized to 154 px, so
  the metric is roughly scale-invariant, but a body with a lower-resolution
  full JPEG (K near 2) may score systematically differently. A follow-up
  could dump `full_sharp` on a NEF / CR3 folder; it is not in this plan.
- **Unavailable vs. not measured.** `NULL` means either; the frontend treats
  both as "no demotion". A user cannot tell from the meta pane whether a
  Good frame was checked. Acceptable for a cue, and the meta row's absence
  says as much.
- **Cost.** One more ranged read of several MB per measured frame plus two
  partial decodes that still entropy-decode every MCU row above the crop (a
  crop low in the frame costs most of a full decode's entropy pass). Pass 2
  is already the lowest-priority pass and the read takes a `ReadSlot`, so a
  preview read pauses it; the cold-disk case is the one to watch in Step 3.
- **Schema and version bumps.** `SCHEMA_VERSION` 24 migrates in place;
  `FACES_VERSION` 10 re-runs pass 2 on every indexed folder at the next
  open, which is the usual cost of a cue change.
- **The two acceptance files.** The prototype placed `_DSC1508` and
  `_DSC1536` below the cut on 2026-11-07; Step 2's `validation.md` must show
  their Rust ranks below the Rust cut too, else the threshold or the port is
  wrong, not the files.

## Refit experiment (not adopted)

Before Step 1, full-size features were tried on the 806 hand-labeled frames
(406 training, 400 held-out). Held-out AUC: current `eye_focus` 0.800, the
full-size gradient score alone 0.705, `eye_focus` plus it 0.803, plus a
full-size Laplacian 0.803. Full-size features did not improve the logistic
model, so `eye_focus` stays and the score remains a separate check. These
labels contain no reject among Good frames, so they measure only how many
picks the check catches, not how many soft frames it removes.

## Progress

- Step 1 (2026-11-08): added `crates/core/src/fullsharp.rs`, the full-size eye sharpness metric, with its unit tests.
- (2026-11-08) Step 1 complete
