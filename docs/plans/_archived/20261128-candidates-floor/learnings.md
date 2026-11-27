# Learnings: candidates-floor

## Step 1

- Outcome: the floor is lowered. On every closed-eyes reading of the stop
  rule the added faces were clean (0 false closed of 23 added faces; the 3
  read closed were closed; 2 frames newly marked, both rightly). The numbers
  are in `measurement.md`.
- Most added candidates never count. Of 122 candidates scoring 0.6 to 0.8
  that got a mesh, `allfaces::measured` turned down 99 (side-face mesh,
  presence or eye offset). `L1005161.DNG` went from 3 judged faces to 4,
  not 7, so `Faces judged` still trails `Similar faces` there.
- The AF path admits non-faces at 0.6: on the first 300 ARW of
  `2026-11-07`, 6 of the 13 added faces were a jersey number (three
  frames), a basketball, a raised hand and an ear; one more was a face
  behind both hands. They read open, so no closed count moved, but they
  add to `Faces judged` and three of them to the soft count. None is on a
  frame at gate 5 or more.
- A frame with no face at 0.8 has no judged face and no RGB decode, so a
  0.6 to 0.8 face there now makes `other_faces` answer unknown: 14 frames of
  `2026-02-01` went from `0 0 0` to `- - -` in `handpic-cli allfaces`.
  Their gate count is 0, so the app never runs pass 3 on them.
- Timing on this shared host is useless from single runs: the "before"
  means were 3 to 4 times the documented ones and single files moved by
  hundreds of ms either way. The added mesh count (3.0 per gate frame on
  the Leica groups, 0.46 on the team sport) with the stage profile's about
  44 ms per mesh is the figure to quote.
- The scratch dump lived in `allfaces::measure_unless` behind an
  environment variable, with the file path passed through a static set in
  `extract_face_counts_unless`: one build gave both the timing run and the
  dump. It wrote upscaled upright face crops with `mozjpeg`, since core has
  no `image` dependency outside its tests and the host's Python has no PIL.
- A bare `python3` in Git Bash hung (the Windows Store stub); `mise x --
  python -I` works, as `CLAUDE.md` says.
- `scan::tests::the_other_faces_need_a_detection_and_a_decode_only_when_there_is_one`
  used a face at `FACE_CONFIDENCE.next_down()` as the non-candidate; it now
  uses `GATE_LOW_RES_CONFIDENCE.next_down()`. The cue's own test at
  `FACE_CONFIDENCE.next_down()` stays, since the judged face keeps 0.8.

## Deferred issues (todo candidates)

- **Core: non-faces among pass 3's candidates on AF frames.** At the 0.6
  floor the 640x448 whole-image search of an AF frame counts a jersey
  number, a ball, a hand or an ear as a face (6 of 13 added faces on the
  first 300 ARW of `2026-11-07`), raising `Faces judged` and, for three, the
  soft count (`Anyone soft`). All were on frames under the default gate of
  5, so they show only under a lowered `Faces in a group photo`. A size
  band relative to the judged face (as the gate has) or a higher floor on
  the AF path would drop them; it changes the counts, so it needs an
  `ALLFACES_VERSION` bump. Basis: this step's measurement
  (`docs/plans/_archived/20261128-candidates-floor/measurement.md`). Files:
  `crates/core/src/allfaces.rs` (`candidates`), `crates/core/src/scan.rs`
  (`other_faces`).
- **Core / CLI: `allfaces` answers unknown on a frame with no judged face
  but a 0.6 to 0.8 face.** `whole_image_cue_rgb` gives no RGB without a
  judged face, and `other_faces` answers unknown when a candidate needs
  pixels; 14 frames of `2026-02-01` went from `0 0 0` to `- - -`. The gate
  count of such a frame is 0, so the app never runs pass 3 on it; only
  `handpic-cli allfaces` shows it. Basis: this step's measurement. Files:
  `crates/core/src/scan.rs` (`other_faces`, `whole_image_cue_rgb`).
- **The time cost of the 0.6 floor is unmeasured on an idle host.** Single
  runs on the shared host were too noisy; the cost is estimated from the
  added mesh count. Repeated alternated runs (as
  `docs/humans/performance.md` does) would need the user's go-ahead.
  Basis: this step's measurement. Files: `docs/humans/performance.md`,
  `docs/humans/performance.ja.md`.
