# Step 1 measurement: thresholds and the cap on real group folders

Measured on 2026-11-23 on Windows 11 (Intel Core i7-13700), release
`handpic-cli` built from `3918064c` (before) and from Step 1 (after), files
on the local drive.

## Folders

| Folder | Files | Body | AF point | Faces |
| --- | --- | --- | --- | --- |
| `2026-02-01` | 146 DNG | Leica M11-P | none | groups, the 7-person swing frame `L1005161.DNG` included |
| `2026-09-19` | 282 ARW | Sony α7 V | trusted | a youth basketball team, several faces per frame |
| `2026-11-07`, first 300 ARW | 300 ARW | Sony α7 V | trusted | a team sport, up to 5 candidates per frame |
| `2026-09-05` | 411 DNG | Leica M11-P | none | mostly one or two people |
| `2026-10-31` | 954 DNG | Leica M11-P | none | mostly one or two people |

`handpic-cli facechoice <folder> 8 --out <tsv>` dumped every face of the
whole-image set (box, score, EAR, `mesh_ms`, `eye_offset`, pose, `edge_gap`,
`full_sharp`); a scratch Python script applied Step 1's rule to the dump, and
the implementation's own counts (printed from `features` by a throwaway
`eprintln!` in a local build) matched the script on `2026-02-01` and the
`2026-11-07` slice exactly (judged per frame, closed, frames with a closed
or a soft face).

## (1) Faces under `EYES_MIN_FACE`

Whole-image faces at score 0.8 or above whose box long side is under 60
preview px, so not judged:

| Folder | Whole faces | Score >= 0.8 | Of those under 60 px |
| --- | --- | --- | --- |
| `2026-02-01` (2112 px DNG previews) | 168 | 107 | 0 |
| `2026-09-19` (1616 px ARW previews) | 831 | 413 | 101 (24%) |
| `2026-11-07` slice | 1524 | 850 | 407 (48%) |

On the ARW team-sport frames about a quarter to a half of the confident
faces are the background players under the floor. Kept excluded
(Decision: Option A).

## (2) False closed readings and the mesh-fit gate

Without a gate, the other faces read closed (EAR at most
`EYES_CLOSED_EAR`) 7 times on `2026-02-01`, 13 on `2026-09-05`, 26 on
`2026-09-19`, 43 on the `2026-11-07` slice. Face crops of every closed
reading on `2026-02-01` and on `2026-09-19` were checked by eye:

- On `2026-02-01` the closed readings are closed or squinting eyes, or a
  face looking down (the same behavior as the judged face's EAR). A profile
  at yaw -91 read closed and its visible eye was indeed shut.
- On `2026-09-19` every closed reading whose mesh `eye_offset` is above 0.1
  (8 faces, offsets 0.110-0.401) was doubtful or wrong: a blurred profile
  with the mesh off the face (`_DSC2418`, offset 0.401), the back of a head
  (`_DSC2113`), a face behind a hand (`_DSC2853`), heavily blurred or dark
  faces. Those at 0.1 or under were closed, squinting or looking-down eyes,
  with a few heavily blurred background faces left.

So the gate is added (Decision: only if false readings show): another face
counts only when `candidate::mesh_eye_offset` is at most
`allfaces::MAX_EYE_OFFSET` (0.1, the app's `mark::MAX_EYE_OFFSET`). Yaw does
not separate the false readings (the true profile above is at -91), so no
yaw limit. The judged face counts whatever its offset. With the gate the
closed readings of other faces become 5, 12, 17 and 33, and the gate drops
6 of 35, 9 of 71, 60 of 146 and 97 of 302 other faces from the counts.

## The cap

No frame of the five folders had more than 4 candidates besides the judged
face (5 faces counted at most), so `MAX_FACES` = 8 never bound. Kept at 8.

## Soft threshold

Over every confident face of 60 px or more (no gate), `SOFT_BELOW` (0.162)
would mark 45 of 107 soft on `2026-02-01` and 152 of 312 on `2026-09-19`;
`LOW_BELOW` (0.110) marks 18 and 73. Frames with at least one soft face (gate on): 12 of 146 on
`2026-02-01`, 50 of 282 on `2026-09-19`, 96 of 300 on the `2026-11-07`
slice. Kept `LOW_BELOW` (Decision).

## Counts per frame (Step 1's rule, gate on)

| Folder | Faces judged per frame (frames) | Frames with a closed face (from other faces only) | Frames with a soft face |
| --- | --- | --- | --- |
| `2026-02-01` | 0: 74, 1: 52, 2: 12, 3: 7, 4: 1 | 12 (3) | 12 |
| `2026-09-19` | 0: 41, 1: 177, 2: 54, 3: 8, 4: 2 | 22 (15) | 50 |
| `2026-11-07` slice | 0: 80, 1: 103, 2: 69, 3: 37, 4: 11 | 57 (27) | 96 |

## Matching the judged face

On AF frames the cue's face comes from the crop detection and the other
faces from the whole-image detection. On `2026-09-19`, 228 of the 258 AF
frames with a judged face had a whole-image face over it at an IoU above
`faces::NMS_THRESHOLD` (dropped as the same face), 29 had none (the whole
search missed it, nothing to drop) and 1 had an overlap at or under 0.3,
which could count that face twice.

## (3) Cost and where it goes

Profiled stage by stage with a scratch `handpic-cli` subcommand (not
committed), one thread, the first 100 ARW of `2026-10-03` (mostly one
person, every file with a trusted AF point, 44 other faces measured) and of
`2026-11-07` (a team sport, 76 other faces), two runs that agreed within 3%.
Per AF file:

| Stage | One person (ms) | Groups (ms) |
| --- | --- | --- |
| Existing: preview read, decode, cue, score, full JPEG read | 61 | 55 |
| Existing: the judged face's two eye crops, one partial decode each | 22.8 | 36.6 |
| New: `decode_whole` | 3.8 | 4.3 |
| New: whole-image YuNet at 640x448 | 71.0 | 72.0 |
| New: the other faces' meshes | 19.4 | 33.5 |
| New: the other faces' eye crops, one partial decode each | 7.4 | 30.0 |

Each partial decode of the full JPEG entropy-decodes every row above its
crop, so a face's two eyes cost two near-full passes (about 11-15 ms each).
What was cut and what was not:

- The eye crops: every eye of the frame, the judged face's included, now
  comes out of one pass (`partial::decode_crops`, which decodes each needed
  row once over the columns of all the crops, and
  `fullsharp::full_eye_sharpnesses`): 13.6 ms instead of 22.8 + 7.4 on the
  one-person set, 24.4 instead of 36.6 + 30.0 on the group set. The values
  are the same: the pixels differ from the per-eye decodes only in the
  outermost column of a per-eye window (chroma upsampling at the window's
  edge), which the eye cut never reaches; the scratch profiler compared
  the scores of every face both ways on the two sets and on `2026-09-19`
  with 0 mismatches, and `facechoice --check` found 0 mismatches.
- The whole-image YuNet run is most of what is left. A 480x320 input
  halved it (57 against 117 ms on 8 loaded threads) but lost 69 of 302
  counted faces on the `2026-11-07` slice, 39 of 121 on `2026-10-03` and 44
  of 146 on `2026-09-19` (scores under 0.8 at the smaller size), so it stays
  at 640x448.
- `decode_whole` from the cue's decode instead of a second scaled decode:
  at most 4 ms, and the AF frames' face set would then differ from the
  no-AF path's and `facechoice`'s, so not done.
- No other face, no work: a frame without a candidate meshes nothing and
  reads the full JPEG only for the judged face, as before (already so in
  the first version).

`analysis_ms` of `handpic-cli features <files> 1`, fastest of three
alternated runs (other processes shared the machine; on 8 threads the
repeats varied up to 2x), mean per file: one person 81.5 ms before, 188.6
with the first version, 163.3 now; groups 92.4, 240.4, 183.9; the
`2026-02-01` DNGs (no AF point) 119.3, 138.9, 117.5. Details in "All-faces
check (Windows 11)" in `docs/humans/performance.md`. The stored values
(`sharpness`, the cue, `full_eye_sharp`) are the same before and after on
every file of the three timed sets (`features` columns compared,
`facechoice --check`: 0 mismatches of 146, 300 and 300 files).
