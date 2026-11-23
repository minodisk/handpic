# Learnings

## Step 1: Measure every confident face in pass 2

- The measurement is in [measurement.md](./measurement.md). Group folders
  were available locally (`2026-02-01` with the swing group, the
  `2026-09-19` and `2026-11-07` team-sport ARW folders), so nothing of the
  Done-when is left to a manual check.
- The mesh-fit gate was added: every closed reading of another face whose
  mesh `eye_offset` was above 0.1 was doubtful or wrong on a visual check
  (a mesh off a blurred profile at 0.401, the back of a head, a face behind
  a hand). Yaw does not separate them (a real closed profile sat at yaw
  -91), so there is no yaw limit. The judged face counts whatever its
  offset, to stay consistent with the `Eyes` row.
- The cap never bound (at most 5 faces counted on any frame of five
  folders); `MAX_FACES` stays 8.
- The plan's cost estimate for the AF frames' whole-image detection ("about
  16 ms") was low: the 640x448 YuNet run is about 71 ms on one thread, and
  it is most of the added cost (profile in measurement.md). On the
  coordinator's request the step was optimized before commit: every eye of
  the frame is now cut from the full JPEG in one pass
  (`partial::decode_crops`, `fullsharp::full_eye_sharpnesses`), which also
  makes the judged face's existing measurement cheaper; a 480x320 input
  was tried and rejected (it lost 23-32% of the counted faces). On one
  thread pass 2 per AF file went from about 82-92 ms to 163-184 ms (the
  first version was 189-240 ms).
- Per-eye partial decodes and one union-window pass differ only in the
  outermost column of the per-eye window: libjpeg's chroma upsampling at a
  cropped window's edge replicates instead of using the real neighbor. The
  eye cut never reaches that column, so the scores are identical (checked
  on real files), but a pixel-exact test must skip that column.
- Timing on 8 threads was unusable while other agents built in other
  worktrees (repeats varied 2x, once 124 vs 276 ms for the same binary);
  one-thread runs, the fastest of three, and a stage profile repeated twice
  were stable.
- `scan::Analysis` gained a field, so the CLI's `features_line` test, which
  builds an `Analysis` without `..Default`, needed `face_counts: None`; the
  core tests did not change. `cue`, `whole_image_cue` and `full_eye_sharp`
  in `scan.rs` became `#[cfg(test)]` wrappers over the new paths
  (`af_cue`, `whole_image_cue_rgb`, `full_sharps`) so their tests still
  exercise the production code.
- The caller now owns the preview decode: `candidate::focus_cue_rgb_unless`
  takes the decoded RGB, and `focus_cue_unless` decodes then calls it, so
  the other faces are meshed on the cue's decode.
- On AF frames the judged (crop-detected) face and its whole-image
  counterpart are matched by IoU above `NMS_THRESHOLD`; on `2026-09-19` one
  of 258 AF frames had an overlap at or under 0.3, which could count that
  face twice.
- A full `cargo test -p handpic-core --lib` with the default thread count was
  killed once (exit 137) while the session ended; with `--test-threads=4` it
  passed (371 tests).

## Deferred issues (todo candidates)

- Pass 2 cost on AF frames still about doubles, mostly the whole-image
  YuNet run at 640x448 (about 71 ms on one thread). Ideas not tried: run it
  only on the parts of the preview outside the `CATCH_CROP` square, or pick
  a smaller input only for previews large enough that a 60 px face stays
  above YuNet's minimum. Basis: Step 1's profile (measurement.md, "(3) Cost
  and where it goes"). Files: `crates/core/src/scan.rs`
  (`extract_analysis_unless`), `crates/core/src/faces.rs`.
- On ARW team-sport folders 24-48% of the confident faces are under
  `EYES_MIN_FACE` and go unjudged (measurement.md, item 1); meshing them from
  the full JPEG (the plan's Option B) is the follow-up the plan names.
  Files: `crates/core/src/allfaces.rs`, `crates/core/src/eyes.rs`.
- A judged crop face whose whole-image box overlaps it at or under
  `NMS_THRESHOLD` IoU is counted twice (1 of 258 AF frames on
  `2026-09-19`). A matching rule by eye landmarks instead of IoU would close
  it. Files: `crates/core/src/allfaces.rs` (`candidates`).
