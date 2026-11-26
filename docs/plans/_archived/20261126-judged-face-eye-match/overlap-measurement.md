# Overlap over the smaller box: measurement

Measured on 2026-11-27 for Step 1. A scratch `handpic-cli` subcommand (not
committed) took each frame's judged face as pass 2 does (on an AF frame from
`candidate::focus_cue`; without an AF point the face `eyes::judged_face` picks
from the whole-image `faces::detect_around`) and compared it with every face
of the whole-preview search: at `faces::WHOLE_INPUT` (640x448, what
`candidates` sees) and, on an AF frame, at `faces::GATE_INPUT` (320x224, what
`gate_count` sees). For each pair it printed the IoU, the overlap
(intersection / min of the two box areas), the score, the long side over the
judged face's and `candidate::meshes_face`.

Folders, all files measured:

| Folder | Files | Frames with a judged face | AF frames |
| --- | --- | --- | --- |
| `2026-09-19` | 282 ARW | 261 | 261 |
| `2026-11-07` | 3638 ARW (the whole folder, not the 300-file slice) | 3354 | 3354 |
| `2026-02-01` | 146 DNG (no AF point) | 72 | 0 |

## The pairs the IoU rule matches (IoU above 0.3)

| Folder | Search | Pairs | overlap min | p1 | p5 | p50 |
| --- | --- | --- | --- | --- | --- | --- |
| `2026-09-19` | 640x448 | 228 | 0.562 | 0.716 | 0.862 | 0.977 |
| `2026-09-19` | 320x224 | 171 | 0.533 | 0.575 | 0.781 | 0.942 |
| `2026-11-07` | 640x448 | 2964 | 0.470 | 0.689 | 0.820 | 0.969 |
| `2026-11-07` | 320x224 | 2249 | 0.488 | 0.570 | 0.677 | 0.920 |
| `2026-02-01` | 640x448 | 72 | 1.000 | 1.000 | 1.000 | 1.000 |

On `2026-02-01` the only IoU-matched pair of each frame is the judged face
itself (it comes from the same search). For an IoU above 0.3 the overlap is
at least 2 x 0.3 / 1.3 = 0.4615 (two boxes of equal area), so at a cut at or
under 0.46 replacing the IoU rule and the union ("IoU above 0.3 or overlap
above the cut") are the same rule. Above that cut, replacing turns
IoU-matched same-face pairs into double counts (see the table below).

## Every counted pair at or under IoU 0.3 with an overlap of 0.3 or more

Only pairs that can change a count are listed: a 320x224 face the gate
counts (score at least `GATE_LOW_RES_CONFIDENCE`, long side within
`GATE_MIN_SIZE` to `GATE_MAX_SIZE`), or a 640x448 face `candidates` keeps
(score at least `FACE_CONFIDENCE`, `meshes_face`). Each was looked at with
both boxes drawn on the preview. All are in `2026-11-07` unless named.

| Overlap | IoU | Frame | Search | What the other box is |
| --- | --- | --- | --- | --- |
| 0.740 | 0.27 | `_DSC2285` | 320x224 | the same face, a larger box |
| 0.655 | 0.25 | `2026-09-19/_DSC1899` | 320x224 | the same profile face, a box on the ear |
| 0.643 | 0.22 | `_DSC3052` | 320x224 | the same head, a larger box |
| 0.628 | 0.25 | `_DSC1524` | 320x224 | the same face, a larger partial box |
| 0.598 | 0.26 | `_DSC3246` | 320x224 | the same blurred foreground head |
| 0.595 | 0.29 | `_DSC3997` | 320x224 | the same face, a box inside |
| 0.594 | 0.26 | `_DSC3514` | 320x224 | the same face, shifted |
| 0.567 | 0.17 | `_DSC2572` | 320x224 | the same face, a small box on the cheek |
| 0.567 | 0.24 | `_DSC3211` | 320x224 | the same face |
| 0.564 | 0.27 | `_DSC3062` | 320x224 | the same face, a box on the ear |
| 0.558 | 0.30 | `_DSC2854` | 320x224 | the same face, a box over the hand beside it |
| 0.549 | 0.24 | `_DSC3886` | 640x448 | the same person (face behind a hand) |
| 0.543 | 0.28 | `_DSC2169` | 320x224 | the same face |
| **0.532** | 0.16 | **`_DSC1970`** | 320x224 | **a different person, a blurred face in front** |
| 0.532 | 0.24 | `_DSC2360` | 320x224 | the same face, a box on the ear |
| 0.530 | 0.26 | `_DSC4877` | 320x224 | the same head, a box on its side |
| 0.530 | 0.30 | `_DSC1927` | 320x224 | the same face |
| 0.526 | 0.29 | `_DSC3443` | 320x224 | the same face, a box on the ear |
| 0.524 | 0.23 | `_DSC1542` | 320x224 | the same face, a box on the ear |
| 0.522 | 0.23 | `_DSC2012` | 320x224 | the same face, a box on the eyes |
| 0.511 | 0.27 | `_DSC4009` | 320x224 | the same face, a box on the ear |
| 0.492 | 0.29 | `_DSC4198` | 320x224 | the same face |
| 0.482 | 0.19 | `_DSC3118` | 320x224 | the same blurred face |
| 0.478 | 0.22 | `_DSC1606` | 320x224 | the same face, a larger box |
| 0.477 | 0.29 | `_DSC4554` | 320x224 | the same face, shifted toward a face below |
| 0.474 | 0.12 | `_DSC3543` | 320x224 | a spurious box over the face and the torso |
| **0.470** | 0.27 | **`_DSC3958`** | 320x224 | **a different person, the player in front** |
| 0.459 | 0.26 | `_DSC2391` | 320x224 | the same face |
| **0.451** | 0.22 | **`_DSC3584`** | 320x224 | **a different person, the player above** |
| 0.450 | 0.27 | `_DSC5037` | 320x224 | the same face |
| **0.444** | 0.27 | **`_DSC2617`** | 640x448 | **a different person, the player beside** |
| 0.425 | 0.17 | `_DSC3911` | 320x224 | the same face |
| 0.423 | 0.23 | `_DSC2305` | 320x224 | the same head, seen from behind |
| 0.420 | 0.26 | `_DSC2222` | 320x224 | the same face |
| **0.406** | 0.11 | **`_DSC1970`** | 640x448 | **a different person, as at 320x224** |
| 0.375 | 0.14 | `_DSC2012` | 640x448 | the same face, a box on the eyes |
| 0.365 | 0.21 | `_DSC4368` | 320x224 | a spurious box on the shirt beside the face |
| **0.363** | 0.15 | **`_DSC1517`** | 320x224 | **a different person, partly behind** |
| 0.360 | 0.21 | `_DSC3848` | 320x224 | unclear, between the judged face and the face of the player above |
| **0.347** | 0.18 | **`_DSC3847`** | 640x448 | **a different person, the player above** |
| 0.341 | 0.09 | `_DSC2780` | 640x448 | the same blurred person |
| 0.339 | 0.13 | `_DSC2011` | 320x224 | the same face |
| 0.334 | 0.19 | `_DSC4194` | 320x224 | the same face, a box on the ear |
| 0.326 | 0.08 | `_DSC2780` | 320x224 | the same blurred person |
| **0.325** | 0.17 | **`_DSC3820`** | 320x224 | **a different person, the player behind** |
| 0.325 | 0.18 | `_DSC3248` | 320x224 | unclear |
| **0.315** | 0.12 | **`_DSC4113`** | 320x224 | **a different person, partly hidden behind** |
| 0.306 | 0.15 | `_DSC3813` | 320x224 | the same face |
| 0.306 | 0.18 | `2026-09-19/_DSC3290` | 320x224 | a spurious box on the judged head's hair |
| **0.301** | 0.12 | **`_DSC2800`** | 320x224 | **a different person, the player beside** |

`2026-11-07/_DSC1543`, a twin the first measurement named, overlaps by 0.42
at 640x448 but scores 0.67 there, under `FACE_CONFIDENCE`, and does not
appear at 320x224: no count takes it today. `2026-02-01` has no pair at or
under IoU 0.3 with any overlap, so no cut changes a frame there.

The highest different person is at 0.532 (`_DSC1970`), and the same face
appears from 0.306 up, with people at 0.444, 0.451 and 0.470 among the twins.
**No cut separates the twins from the neighbours.** Above 0.532 every pair
is the same face (11 frames at the gate, 1 among the candidates).

## What each cut changes, against today's rule

Frames whose gate count or candidate list changes, both folders together
(`2026-02-01` changes at no cut). "Lower" is a face no longer counted; "higher"
a face newly counted, an IoU-matched twin no longer matched.

| Cut | Union: gate lower | Union: candidates lower | Replace: gate higher | Replace: candidates higher |
| --- | --- | --- | --- | --- |
| 0.35 | 35 | 4 | 0 | 0 |
| 0.40 | 32 | 3 | 0 | 0 |
| 0.45 | 28 | 1 | 0 | 0 |
| 0.50 | 20 | 1 | 2 | 0 |
| 0.55 | 11 | 0 | 18 | 0 |
| 0.60 | 4 | 0 | 42 | 1 |
| 0.65 | 2 | 0 | 88 | 2 |
| 0.70 | 1 | 0 | 148 | 3 |

Replacing lowers the same frames as the union at each cut, and adds the
"higher" ones. Under 0.46 the two rules are the same.

At 0.45 the union drops 3 real people (`_DSC3958`, `_DSC3584`, and
`_DSC1970` at 320x224) and the 640x448 `_DSC2617` neighbour sits just under
it. A cut above 0.532 (for example 0.55 or 0.6) drops only twins: 11 or 4
gate frames, `_DSC1899` and `_DSC1524` among them, and leaves the about 20
twins between 0.3 and 0.532 double-counted. Only the union can take such a
cut; replacing at it would add 18 or 42 double counts.

The user chose the union at 0.55 on 2026-11-27: it lowers 11 gate frames, all the same face, and drops no person, while the twins under 0.532 stay double-counted (see `learnings.md` and the reworded `todo.md` item).
