# Step 1 measurement: pass 3's candidates from 0.6

Measured on 2026-11-28 on Windows 11, release `handpic-cli` built from
`cd802470` (before) and from the same tree with `allfaces::candidates`
filtering on `GATE_LOW_RES_CONFIDENCE` instead of `FACE_CONFIDENCE`
(after), files on the local drive (`D:\photos\2026`). Both sets of the plan
were found.

- `handpic-cli allfaces <files> 1`, once per set and build. The "after"
  build also carried an uncommitted, environment-gated scratch dump in
  `allfaces::measure_unless`: one line per candidate it meshed (box, score,
  EAR, closed by `eyes::EYES_CLOSED_EAR`, or unmeasured when `measured`
  turned it down) and an upright crop of every candidate under 0.8 that
  `measured` accepted. The "after" run had the dump on, so its times include
  writing 23 small JPEGs.
- `handpic-cli features <files> 4`, once per set with the "before" build,
  for the gate counts (the change does not touch the gate).
- Every crop of an added face was looked at, the ones read open too.

"Added faces" below are the candidates scoring 0.6 to 0.8 that `measured`
accepted, the faces that are now judged and were not before. "Added
meshes" are all the candidates scoring 0.6 to 0.8, accepted or not: each
costs a face mesh run.

## Totals

| Set | Frames | Gate >= 5 | Judged before -> after | Closed | Soft | Added meshes | Added faces | Added read closed |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-02-01`, 146 M11-P DNG, no AF point | 146 | 3 | 100 -> 110 | 13 -> 16 | 13 -> 13 | 38 | 10 | 3 |
| same, gate >= 5 only | 3 | 3 | 9 -> 12 | 3 -> 3 | 2 -> 2 | 9 | 3 | 0 |
| `2026-11-07`, first 300 α7 V ARW, trusted AF point | 300 | 37 | 351 -> 364 | 63 -> 63 | 113 -> 117 | 84 | 13 | 0 |
| same, gate >= 5 only | 37 | 37 | 63 -> 64 | 9 -> 9 | 25 -> 25 | 17 | 1 | 0 |

The 37 frames at gate >= 5 of `2026-11-07` match the archived
gate-floor-no-af measurement.

## The added faces, looked at

| Set | Added faces | Real, open | Real, closed | Read closed: actually closed / open (false) / undecidable / not a face | Undecidable | Not a face |
| --- | --- | --- | --- | --- | --- | --- |
| `2026-02-01` | 10 | 7 | 3 | 3 / 0 / 0 / 0 | 0 | 0 |
| `2026-11-07` | 13 | 6 | 0 | 0 / 0 / 0 / 0 | 1 | 6 |
| Both | 23 | 13 | 3 | 3 / 0 / 0 / 0 | 1 | 6 |

The three read closed, all on `2026-02-01`, all actually closed:

| File | Box (stored preview px) | Score | EAR | Look |
| --- | --- | --- | --- | --- |
| `L1005563.DNG` | 237, 292, 209x353 | 0.775 | 0.101 | a child looking down, lids lowered |
| `L1005579.DNG` | 1072, 586, 146x170 | 0.735 | 0.108 | a laughing child, eyes shut |
| `L1005705.DNG` | 1168, 299, 56x70 | 0.786 | 0.126 | a laughing child, eyes squeezed to slits |

The seven read open on `2026-02-01` are all real open faces (the swing group
of `L1005161` / `L1005172` / `L1005173` / `L1005175` and a snow trip,
`L1005705` / `L1005708` / `L1005709`).

The thirteen on `2026-11-07` (all read open):

| File | Score | What the crop shows |
| --- | --- | --- |
| `_DSC1514.ARW` | 0.692 | a player looking up, open |
| `_DSC1676.ARW` | 0.750 | a child looking up, open |
| `_DSC1677.ARW` | 0.790 | a child looking up, open (the only added face at gate >= 5) |
| `_DSC1685.ARW` | 0.796 | a face half covered by another head, eyes open |
| `_DSC1751.ARW` | 0.775 | a player from below, open |
| `_DSC1754.ARW` | 0.751 | a child under a raised arm, open |
| `_DSC1501.ARW` | 0.747 | a face behind both hands: undecidable |
| `_DSC1544.ARW` | 0.687 | the ear and back of a head: not a face |
| `_DSC1627.ARW` | 0.674 | the `33` of a jersey: not a face |
| `_DSC1644.ARW` | 0.695 | the `33` of a jersey, blurred: not a face |
| `_DSC1653.ARW` | 0.686 | the `33` of a jersey: not a face |
| `_DSC1771.ARW` | 0.603 | a basketball: not a face |
| `_DSC1772.ARW` | 0.752 | a raised hand: not a face |

None of the six non-faces is on a frame at gate >= 5 (their gate counts are
1 to 4); they show up only under a lowered `Faces in a group photo`.

## Per frame

Frames whose closed count went from 0 to 1 or more: `L1005563.DNG` (gate
2) and `L1005705.DNG` (gate 4), both true (the faces above). Neither is at
gate >= 5. `L1005579.DNG` (gate 2) went from 1 to 2 closed, also true. None
false. On `2026-11-07` no closed count changed.

Soft counts changed only on `2026-11-07`, from 113 to 117 over the four
frames `_DSC1514` (2 -> 3, a real face looking up), `_DSC1544` (0 -> 1, the
ear), `_DSC1644` and `_DSC1653` (0 -> 1, the jersey): three of the four
added soft readings are non-faces, all on frames under gate 5.

No frame's `judged` went down. Fourteen `2026-02-01` frames went from
`0 0 0` to `- - -` (unknown): frames with no face at 0.8, so no judged face
and no decode (`whole_image_cue_rgb` returns no RGB), where a face of 0.6 to
0.8 is now a candidate and `other_faces` answers unknown for want of the
pixels. Their gate count is 0, so pass 3 never runs on them in the app;
only the CLI's dump shows the change.

Every changed row (gate, judged / closed / soft before -> after):

| File | Gate | Before | After |
| --- | --- | --- | --- |
| `L1005161.DNG` | 7 | 3 / 2 / 0 | 4 / 2 / 0 |
| `L1005172.DNG` | 7 | 3 / 1 / 1 | 4 / 1 / 1 |
| `L1005173.DNG` | 5 | 3 / 0 / 1 | 4 / 0 / 1 |
| `L1005175.DNG` | 4 | 3 / 0 / 0 | 4 / 0 / 0 |
| `L1005563.DNG` | 2 | 1 / 0 / 0 | 2 / 1 / 0 |
| `L1005579.DNG` | 2 | 1 / 1 / 0 | 2 / 2 / 0 |
| `L1005705.DNG` | 4 | 1 / 0 / 0 | 3 / 1 / 0 |
| `L1005708.DNG` | 4 | 1 / 0 / 0 | 2 / 0 / 0 |
| `L1005709.DNG` | 2 | 1 / 0 / 0 | 2 / 0 / 0 |
| 14 frames of `2026-02-01` | 0 | 0 / 0 / 0 | - / - / - |
| `_DSC1501.ARW` | 4 | 1 / 0 / 0 | 2 / 0 / 0 |
| `_DSC1514.ARW` | 4 | 2 / 0 / 2 | 3 / 0 / 3 |
| `_DSC1544.ARW` | 1 | 0 / 0 / 0 | 1 / 0 / 1 |
| `_DSC1627.ARW` | 3 | 1 / 0 / 0 | 2 / 0 / 0 |
| `_DSC1644.ARW` | 3 | 2 / 2 / 0 | 3 / 2 / 1 |
| `_DSC1653.ARW` | 2 | 0 / 0 / 0 | 1 / 0 / 1 |
| `_DSC1676.ARW` | 3 | 2 / 0 / 2 | 3 / 0 / 2 |
| `_DSC1677.ARW` | 5 | 4 / 0 / 2 | 5 / 0 / 2 |
| `_DSC1685.ARW` | 4 | 3 / 1 / 0 | 4 / 1 / 0 |
| `_DSC1751.ARW` | 4 | 3 / 0 / 0 | 4 / 0 / 0 |
| `_DSC1754.ARW` | 4 | 2 / 0 / 0 | 3 / 0 / 0 |
| `_DSC1771.ARW` | 2 | 2 / 0 / 0 | 3 / 0 / 0 |
| `_DSC1772.ARW` | 2 | 2 / 0 / 0 | 3 / 0 / 0 |

## `L1005161` / `L1005172` / `L1005173`

| File | Gate count | Judged before | Judged after | Scores 0.6 to 0.8 (accepted by `measured`?) |
| --- | --- | --- | --- | --- |
| `L1005161.DNG` | 7 | 3 | 4 | 0.645 (no), 0.750 (yes), 0.644 (no) |
| `L1005172.DNG` | 7 | 3 | 4 | 0.693 (no), 0.636 (no), 0.737 (yes), 0.677 (no) |
| `L1005173.DNG` | 5 | 3 | 4 | 0.758 (no), 0.725 (yes) |

`L1005161` does not reach 7: the lower floor gives its two 0.64 faces a
mesh, but `measured` turns both down (an untrusted side-face mesh, a
presence under the floor or an eye offset over `MAX_EYE_OFFSET`; the dump
did not say which). The 0.8 / 0.6 gap is gone; `Faces judged` still differs
from `Similar faces` where a mesh is not believed.

## `MAX_FACES`

The most candidates on one frame: 4 at 0.8 (both sets), 6 at 0.6
(`L1005161`, `L1005172`) and 5 on `2026-11-07`, so at most 7 faces with the
judged one. No frame reached the cap of 8, and no 0.8 face was pushed out.

## Time

| Set | Mean ms per file, before -> after | Over gate >= 5 frames | Added meshes per gate >= 5 frame |
| --- | --- | --- | --- |
| `2026-02-01` | 455.7 -> 549.2 | 994.6 -> 1193.3 | 3.0 (9 over 3) |
| `2026-11-07`, first 300 | 681.9 -> 1069.8 | 791.6 -> 1272.4 | 0.46 (17 over 37) |

The measured ms are single runs on a loaded host: the "before" means are
three to four times the 105-186 ms `docs/humans/performance.md` measured
on 2026-11-24 (fastest of three), and single files moved both ways by
hundreds of ms (`_DSC1751` 875 -> 497, `_DSC1514` 470 -> 2041). They do not
give the cost of the change. The noise-free figure is the added meshes:
the 2026-11-23 stage profile in `docs/humans/performance.md` puts the
other faces' meshes at 33.5 ms per file for 0.76 faces, about 44 ms per
mesh, so about 130 ms more per gate frame on the Leica groups and about
20 ms on the team sport, plus the added eyes in the one full-JPEG pass for
the faces `measured` accepts (3 and 1 of them). Over every frame it is
0.26 and 0.28 added meshes per frame (38 over 146, 84 over 300).

## Stop condition

Not met, on every reading of it that is about closed eyes:

- Literal (false closed over the added faces): 0 of 23.
- The closed readings' precision among the added faces: 3 of 3 actually
  closed, 0 false.
- Frames newly given the closed-eyes mark: 2 true, 0 false.

A broader reading, "misjudged" as any face wrongly counted, reaches half on
the AF set alone: 6 non-faces and 1 undecidable of its 13 added faces
(7 of 13), against 0 of 10 on `2026-02-01` (7 of 23 over both, and 0 of 4
on the frames at gate >= 5). The non-faces read open, so they move no
closed count; they add to `Faces judged` and, three of them, to the soft
count, only on frames under the default gate of 5. The floor is lowered.
