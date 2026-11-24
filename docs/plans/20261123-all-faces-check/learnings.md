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

## Step 1 (rework): the gate count in pass 2, the all-faces check as its own entry point

- The first version's counts are reproduced exactly by
  `scan::extract_face_counts`: a throwaway `eprintln!` of
  `Analysis::face_counts` in a build of `50a0cb4f` against the new
  `handpic-cli allfaces` on `2026-02-01`, `2026-09-19`, the first 300 ARW of
  `2026-11-07` and the first 100 of `2026-10-03`: 0 mismatches of 828 files.
  The entry point is the first version's pass 2 minus the score, so this was
  expected; the existing `features` columns and `facechoice --check` did
  not change either.
- A cold release build of `handpic-cli` took 2.5 min here once and 12 min
  another time (after a build of the `3918064c` sources from a scratch copy
  into the same target dir); incremental ones took 13-16 s.
- `cargo` from a scratch copy of an older commit (`git archive` into the
  scratchpad, `--target-dir` pointing at this worktree's `target/`) is a
  quick way to time "before" without touching the worktree or the stash.
- Testing "no 640x448 detection and no other mesh on an AF frame" needed
  test-only counters: `faces::model_runs` (per model input) and
  `eyes::MESH_CALLS`, both `#[cfg(test)]` thread-locals. A synthetic preview
  has no face, so the AF frame's cue judges none; `af_gate_count` is tested
  on a `Cue` given a face directly.
- The plan's cost estimate for the low-resolution run held (about 20 ms per
  AF file on one thread at 320x224, the scaled decode included), but its
  recall did not: at the gate's 0.8 score floor it counts about half the
  faces the 640x448 run counts on the team-sport frames, and the gate
  reaches N = 5 on 1 AF frame of the 682 measured against 49 with 640x448
  (measurement.md, "The low-resolution input"). 320x224 was kept as the
  plan's rule asks (the size picked against its cost); a larger input at
  0.8 does not fix it within budget (480x320: about 38 ms, 15 of 44 on the
  `2026-11-07` slice).
- What does fix it is the score floor: the small faces are found at 0.6 to
  0.8. 320x224 with a 0.6 floor counts about as many faces as 640x448 at
  0.8 (884 against 909 on the `2026-11-07` slice). The user decided on
  2026-11-24: 0.6 (`allfaces::GATE_LOW_RES_CONFIDENCE`) on the AF path's
  320x224 run only, 0.8 kept on the 640x448 path. A release build with it
  gave the scratch sweep's gate counts on every AF frame of the three
  folders and left every other `features` value and the `2026-02-01` gate
  counts unchanged.
- No measured folder holds a posed group photo on a body with an AF point;
  the frames the gate passes at 5 or more are basketball frames with
  players, a bench or spectators of the judged player's size. The posed
  7-person group of `2026-02-01` counts 4 even at 640x448 (three faces score
  0.64-0.75), so the default N = 5 does not pass it.
- The first version's deferred "pass 2 about doubles" is resolved by this
  rework (the 640x448 run left pass 2) and was taken off the list below.

- After `main` added the face mesh presence rules to the cue, the judged
  face already followed them (it counts only with the cue's EAR, which they
  withhold), but the other faces did not: `allfaces::measured` now turns
  down an untrusted mesh and one under `EYES_MIN_PRESENCE` as well. On the
  828 measured files it drops 9 faces (1 closed, 5 soft readings); see
  measurement.md.
- When the cue's mesh is under the presence floor but trusted, the judged
  face keeps its eye centers, so `full_sharps` still measures its eyes on the
  full JPEG though the counts ignore that reading (no EAR, no `Reading`).
  It costs a full read only when no other face is measured; left as is.

## Step 2: Store the gate count and the counts, and pass 3 in the engine

- `FACES_VERSION` was already 13 on `main` when this step started (the
  no-AF luma change moved it after the plan was written), so the bump that
  fills `faces_group` is 14, not the plan's 12.
- The migration tests from v10 to v26 build their old database by dropping
  columns from a fresh one, so each gained the five `DROP COLUMN`s of the
  new columns; without them `prepare`'s `ADD COLUMN` fails and the index is
  discarded.
- `StoredEyes::new` kept its five arguments and leaves the four new fields
  `None`; `indexed_file` and `run_faces_scan` set them by struct update. The
  app's `new` callers compile unchanged; only the two full `StoredEyes`
  literals in its tests (`filter.rs`, `mark.rs`) needed the fields.
- One type, `AllFacesReady`, is both `write_face_counts`'s row and the
  runner's progress row, since the two hold the same values.
- `extract_face_counts` returning `Ok(None)` (the whole-image detection or a
  decode failed) is stored like an unreadable file, no counts at
  `ALLFACES_VERSION`, but not counted as an error, the way pass 2 treats a
  failed decode.
- An error row and a JPEG row stay off `allfaces_todo` because nothing gives
  them a gate count: `faces_todo`'s reset and the first pass's rewrite
  clear `faces_group`, so the query needs no extension check of its own.

## Deferred issues (todo candidates)

- On ARW team-sport folders 24-48% of the confident faces are under
  `EYES_MIN_FACE` and go unjudged (measurement.md, item 1); meshing them from
  the full JPEG (the plan's Option B) is the follow-up the plan names.
  Files: `crates/core/src/allfaces.rs`, `crates/core/src/eyes.rs`.
- A judged crop face whose whole-image box overlaps it at or under
  `NMS_THRESHOLD` IoU is counted twice (1 of 258 AF frames on
  `2026-09-19`). A matching rule by eye landmarks instead of IoU would close
  it. Files: `crates/core/src/allfaces.rs` (`candidates`).
- The default N = 5 passes no posed group photo of the measured folders:
  the 7-person `2026-02-01/L1005161.DNG` counts 4 at the 0.8 floor (three
  faces at 0.64-0.75; that frame has no AF point, so the AF path's 0.6
  floor does not apply). Revisit the default (Step 4).
  Basis: Step 1 rework (measurement.md). Files: the app's setting (Step 4),
  `crates/core/src/allfaces.rs`.
- The size band does not tell a group from a background row at the same
  distance: `2026-11-07/_DSC1611.ARW` counts 9 at 640x448, one player and
  eight spectators on the bench behind. A frame like that passes the gate
  and pays pass 3. Basis: Step 1 rework's look at the frames at 5 or more.
  Files: `crates/core/src/allfaces.rs` (`gate_count`).
