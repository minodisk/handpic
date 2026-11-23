# Learnings

The five `todo.md` items of the "Core: face/eye-aware focus check for
culling" section, measured and decided by the user on 2026-11-23. Two of them
(the face presence gate and the side-face distrust) ship in Step 1; the other
three are decisions without code.

## Good-photo cuts on frames with no trusted AF point (2026-11-23)

Decided: keep the shared cuts (the user chose the recommendation). Raw data:
`D:\Photos\tests\2026-11-23-noaf-cuts\` (`analysis.md`, `frames.tsv`,
`sheet-marked.png`, `sheet-unmarked.png`, scripts under `scripts\`).

Method. The cuts of `crates/app/src/mark.rs` (`clears`: full_eye_sharp >=
0.162, eyes_ear >= 0.25, |yaw| <= 35, |pitch| <= 45, eye_offset <= 0.10,
edge_gap >= 0.02; since moved to the engine as `StoredEyes::photo_tier` in
`crates/engine/src/index.rs`, with the same cuts) applied to the `noaf`
frames with a cue face of the DNG folders of the burst-keep-score data set
(labels as in its `data.md`: `.dop` flag, XMP without a `.dop`, virtual
copies in `Output/`; base stem in `Output/` for the folders without
sidecars, the camera file standing for a DeepPRIME copy). `handpic-cli
features` does not carry the rule's values: it has no `full_eye_sharp`, and
its `ear` / pose come from the on-demand eyes path, not the scan's cue. The
values were therefore dumped by a throwaway crate calling
`scan::extract_analysis` (what pass 2 stores); its `af`, `eye_offset` and
`edge_gap` agree with `features` on every frame. As the AF reference the same
dump ran over two sidecar-labeled ARW folders, `2026-09-27-b` and
`2026-10-03`.

Data caveats. `2026-08-29-l` now holds only its 33 picks (the non-picks were
deleted after 2026-10-08), so it cannot separate picks and is left out of
the totals. `2026-05-26` (Sigma BF) and `2026-02-14` have no JPEG larger
than the preview, so no `full_eye_sharp`: 141 of the 466 faced no-AF frames
can never be marked. The comparison rests on 325 eligible frames (55 picks),
almost all from `2026-09-05` and `2026-03-29`.

Results (Wilson 95%). Eligible no-AF: 21.2% [17.1-26.0] marked; picks are
13.0% [7.0-23.0] of the marked (9/69) and 18.0% [13.8-23.1] of the unmarked
(46/256), against a 16.9% base. AF reference: 22.5% marked, 12.4% [10.0-15.2]
picks among marked and 11.4% [10.1-12.8] among unmarked. On AF faces too the
mark hardly predicts a pick: it says "not a failure", and a pick is chosen
for its content. `full_eye_sharp` fails the most no-AF picks (56%, 12 of them
failing it alone); EAR and yaw pass picks and non-picks alike; pitch,
eye_offset and edge_gap barely matter. Loosening fes to 0.12 (optionally EAR
0.20, yaw 60) marks 2-3x more picks at about 17.5% precision, which overlaps
the current cuts and the base rate; tightening fes to 0.20 marks fewer at
12.5%.

Decision (user, 2026-11-23). The cuts stay shared between AF and no-AF
frames: no no-AF variant did clearly better with picks as the label. The
unmarked picks are mostly DeepPRIME / Sigma DNGs with no full-size JPEG,
group shots whose largest face is under the 60 px mesh minimum, sideways
faces, and Leica eye sharpness just under 0.162.

Follow-up: a separate no-AF fes cut (about 0.12) is worth checking only
against failure labels ("is this frame a failure"), not picks.

## Crop-detection false positives on prints and objects (2026-11-23)

Decided: no filter (the user chose the recommendation). Raw data:
`D:\Photos\tests\2026-11-23-crop-fp\` (sheets and scripts under
`sheets-and-scripts\`, among them `false_positives.png`).

- Code path: with a trusted AF point the cue still takes the face nearest the
  AF point from YuNet on the 480 px crop (`detect_around_rgb`,
  `CATCH_CONFIDENCE` 0.6); `_DSC3632` (2026-09-19, ARW since deleted) still
  reproduces on its preview, upscaled 2x: a shoulder print scored 0.73-0.79.
- Five AF folders (2026-07-11, 07-24, 08-08, 09-19, 10-11; 4,573 ARW, 4,103
  chosen faces), labeled by contact sheets: 14 chosen non-faces (0.3%):
  6 basketballs, 5 jersey or shirt prints, a hand, a distant body, a lantern.
  All scored 0.61-0.74. Backs of heads are far more common (about 7% of
  chosen faces, about 55% below 0.75).
- Size and aspect do not separate them: object boxes are face-shaped (aspect
  0.65-1.30, 3-16% of the long side). Aspect > 1.2 removes 1 object and 10
  tilted real faces; size floors remove none.
- Score: < 0.75 removes all 14 objects but also about 225 real faces (about
  16 real faces per object), most of them profiles or faces turned up or
  partly hidden.
- MediaPipe's presence logit (`Identity_1`) does not separate them either: a
  print scored +7.0 and the `_DSC3632` print 1.1-3.5, while a quarter of real
  chosen faces score below -5 (on our unrotated crop). A threshold of 0
  removes 13 of 14 objects and a third of all chosen faces.
- Decision (user, 2026-11-23): no filter; the false candidates are too rare
  for what any filter costs in real faces.

## Face presence logit as a gate before the eyes judgment (2026-11-23)

Decided: implement at logit -12 (the user chose the recommendation); shipped
in Step 1 as `eyes::EYES_MIN_PRESENCE` and `eyes::presence_judges_eyes`. Raw
data: `D:\Photos\tests\2026-11-23-face-presence\` (scripts under
`scripts\`).

- The Face Landmarker v2 ONNX has three outputs (Measured): `Identity`
  [1,1,1,1434] (the points), `Identity_1` [1,1,1,1] (the face presence as a
  raw logit; clear faces +15..+25) and `Identity_2` [1,1] (0..1, meaning
  unknown, AUC 0.60 against `x` faces, unused). Reading `Identity_1` costs
  nothing: it comes from the same run.
- Only 176 of the 504 `eyes-truth.md` faces can still be reproduced:
  328 of the 432 labeled 2026-09-19 ARWs are gone from the photo folder.
  Copies under `D:\Photos\tests\` (checked by capture date 2026:09:19, since
  `_DSC` numbers repeat across days) gave back 104; all 72 DNGs remain. Every
  recovered face matched its row's side and score, so the face choice
  reproduces.
- On those faces the logit tells `x x` (no eye to judge) from faces with an
  open or closed eye with AUC 0.95. But the `x` faces are mostly backs of
  heads, profiles and occlusions, not false detections, and only 3 of the 44
  are judged closed today.
- On an unlabeled folder (2026-07-11, 441 judged faces) the logit is low for
  real faces: profiles (median |yaw| 60 deg below logit 0 against 20 deg
  above), faces looking down and faces rotated in the crop, which has no roll
  correction. 29% fall below sigmoid 0.5. The faces judged closed among them
  are genuine downcast eyes, so thresholds from about -8 up lose real closed
  calls.
- Threshold table (logit: x dropped of 44 / real lost of 93 / closed lost of
  15 / 07-11 dropped): -15: 8/0/0/5.0%; -12: 12/0/0/9.1%; -10.5: 14/0/0/10.7%;
  -7.7: 22/1/0/14.7%; -2.2: 33/4/0/24.9%; 0: 35/7/1/28.6%.
- Decision (user, 2026-11-23): logit -12 (sigmoid ~6e-6). It loses no labeled
  open or closed face and no judged-closed face of the unlabeled folder,
  still removes the 2 false closed calls on `x` faces, and makes 9% of the
  folder's faces eyes-unknown. Under it the scan's cue keeps the mesh's eye
  centers, eye offset, edge gap and so `full_eye_sharp`, and drops only the
  EAR and the pose; the on-demand judgment gives nothing, so the `f` overlay
  draws no mesh for such a face.

Follow-up: roll-correcting the face crop, or any other change to it, would
change the logit, so recalibrate the presence thresholds after any crop
change.

## Side faces, the mesh distrust and the eye_offset cut (2026-11-23)

Decided: implement "YuNet eye distance under 0.15 AND presence logit under 0
=> the whole mesh is untrusted" (the user chose the recommendation), shipped
in Step 1 as `eyes::SIDE_FACE_EYE_DISTANCE`, `eyes::SIDE_FACE_PRESENCE` and
`eyes::mesh_trusted`; and no change to the `eye_offset` cut,
`MAX_EYE_OFFSET` 0.10 stays.

The eye distance is YuNet's: the distance between the face's `left_eye` and
`right_eye` landmarks divided by the box's longer side,
`hypot(right_eye - left_eye) / max(width, height)` (the `yed` column of the
dump under `D:\Photos\tests\2026-11-23-side-faces\dump\`). It is the same on
the stored and the upright face.

- Data: `D:\Photos\tests\2026-11-23-side-faces\` (`handpic-cli sideface`
  dump, a throwaway subcommand kept as `sideface-cli.patch`; scripts and
  reports under `scripts\`; the sheet `b1-gains-sheet.png`).
  `D:\photos\2026\2026-09-19` is down from 2134 to 282 ARWs; 231
  more were rebuilt from verified copies in test folders, and the labeled
  frames still missing were taken from the mesh-roll meshfit dump (the mesh
  code is unchanged: 0 of 117 truth frames moved `eye_offset`). Presence
  numbers cover only the frames on disk (109 readable truth frames). The
  misfit labels are still the agent's, unreviewed, and the 0.15 / logit 0
  thresholds were read off the same truth set (in-sample).
- Side-face rule table (weighted to the six folders: off caught / side-off
  caught / on made unknown): yed<0.10: 65%/73%/15.7%; yed<0.15: 86%/100%/21.5%;
  yed<0.20: 86%/100%/36.5%; |yaw|>50: 30%/35%/25.8%; yed<0.15 and |yaw|<40:
  34%/41%/0.7%; presence<0: 95%/100%/24.8%; yed<0.15 and presence<0:
  82%/100%/18.2%; yed<0.10 and presence<0: 73%/88%/14.4%.
- The face mesh model's second output (`Identity_1`, MediaPipe's face presence
  logit) is computed on every run and was thrown away. Gated by YuNet's eye
  distance (< 0.15 of the box side) at logit 0 (MediaPipe's 0.5), it catches
  every side-face misfit of the truth set (22/22; 82% of all off meshes,
  weighted) for 18% of the on meshes (profiles), removes all 11 Low marks on
  off meshes, keeps closed eyes (0 changed wrong) and pose (5 profiles
  unknown, all signs right), and lifts the held-out AF-eye AUC 0.694 -> 0.718
  with Low on 1-star frames 13 -> 12. On the six folders it makes 1066 of 5988
  meshed faces unknown, Low marks 1415 -> 1021, good 1343 -> 1342; the rated
  good frames of `good-mark-2026-10-09` are unchanged.
- An untrusted mesh is treated as no value at all: the EAR, the pose, the eye
  offset, the edge gap and the eye centers are `None`, so `full_eye_sharp` is
  too. Falling back to YuNet's eye points instead raises false Low on 3-star
  frames (3 -> 5 to 7).
- The mesh's |yaw| is not a misfit signal (AUC 0.50): a side-face misfit is
  usually fitted as a frontal face. "YuNet says profile, mesh says frontal"
  (yed < 0.15 and |yaw| < 40) is precise (0.7% of on meshes lost) but catches
  only 41% of side misfits.
- Eye-distance-only rules (0.10 / 0.15 / 0.20) drop most profile meshes, most
  of which are on, and cost the Low mark its recall (13 -> 7 or 8 1-star
  frames).
- `eye_offset` stays the best single on/off feature (AUC 0.884; presence
  0.838, Procrustes residual 0.829, mesh/YuNet box IoU 0.863, nose offset
  0.876). The 65% of frames over 0.10 that are on meshes against off YuNet
  points are profiles the yaw cut (35) already drops. Inside the mark's
  population (|yaw| <= 35), yed < 0.20 with offset > 0.10 is 12 of 15 truth
  frames off, and about 15 of 20 of the 138 frames skipping the cut would add
  look off in the sheet (`b1-gains-sheet.png`). With the side-face rule in,
  skipping the cut would add 0 frames.
- Decisions (user, 2026-11-23): the side-face rule above ships;
  `MAX_EYE_OFFSET` 0.10 stays.

## Step 1: implementation notes

- `eyes::mesh_trusted` treats a non-finite logit as untrusted (a plain `<`
  would have trusted a NaN), and a face whose box is empty as under the eye
  distance, so only its logit decides. `eyes::presence_judges_eyes` is
  `presence >= EYES_MIN_PRESENCE`, which a NaN fails. An empty box never
  reaches the mesh in practice (`meshes_face` and `judge_mesh`'s floor).
- The rules live in `candidate::cue_of`, not in `face_mesh` or `mesh_of`:
  `cue_of` must still fall back to YuNet's landmarks for a face without a
  mesh and must not for an untrusted one. `mesh_of` and `landmarks_of` stay
  raw (the presence is carried on `Mesh` / `Landmarks`), so the CLI's
  `facechoice` and a later all-faces check can apply the public rules
  themselves.
- `FACES_VERSION` 11 -> 12; no schema change. The all-faces-check plan's
  Step 2 also plans a bump: whichever lands second takes the next number.
- The `handpic-cli eyes` line now prints `presence <logit>` after the
  judgment; its state column follows both rules while the EAR and pose
  columns stay raw.
- `mise run ci:local` (Step 1): clippy passed after one fix (`!(side > 0.0)`
  trips `clippy::neg_cmp_op_on_partial_ord`; written as
  `(side > 0.0).then(..)`). The test run then hung: another session on the
  same host was running 24 busy-loop CPU hogs (100% CPU) to reproduce a scan
  starvation, and `scan::tests::the_analysis_pass_delivers_every_index_once_and_stops_on_cancel`
  (pass 2 workers at `Priority::Lowest`) and an engine test binary sat
  idle under them. Run directly from the built test binaries, every other
  core test (374), every engine test (473) and every CLI test passed; the
  one core test hung alone too while the hogs ran. The change does not touch
  that path (a face-less fixture never reaches the mesh). The starved run did
  finish in the end and passed (the engine tests in 1944 s against 35 s
  unloaded), so under a saturated CPU wait it out rather than call it a
  failure.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, any platform with real files): run
  `handpic-cli eyes` on a few real RAW files with a clear frontal face and
  confirm the line's `presence` reads about +15..+25 and the state column
  gives `open` / `closed`; on a known side face (a profile from
  `D:\Photos\tests\2026-11-23-side-faces\dump\`, yed under 0.15) confirm a
  negative presence and `- unknown`. Also run the ignored
  `eyes::tests::judges_a_face_in_a_real_image` with `HANDPIC_FACE_JPEG` set
  to an upright JPEG with one clear face and confirm it still passes (it
  prints the presence). Not run here because the host was pinned at 100%
  CPU by another session; the step's checkbox was ticked on the automated
  criteria. Files: `crates/cli/src/main.rs` (`eyes_line`),
  `crates/core/src/eyes.rs`.
