# Box shape and size of the remaining twins: measurement

Measured on 2026-11-28 for Step 1.

## Method

A scratch `handpic-cli` subcommand (not committed) read each named file's
embedded preview once on 3 threads, took the judged face as pass 2 does on an
AF frame (`candidate::focus_cue` at `sharpness::trusted_focus`) and compared
it with every face of the whole-preview searches: `faces::decode_whole` +
`faces::detect_whole_upright` (640x448, what `candidates` sees) and
`faces::decode_gate` + `faces::detect_gate_upright` (320x224, what
`gate_count` sees). Per pair it printed the IoU, the overlap (intersection
over the smaller box's area), the score, both boxes' `width` and `height` and
`candidate::meshes_face`.

Files read: 36, the frames of the table "Every counted pair at or under IoU
0.3 with an overlap of 0.3 or more" of
[overlap-measurement.md](../_archived/20261126-judged-face-eye-match/overlap-measurement.md)
whose overlap is at or under 0.55: 35 of `D:/photos/2026/2026-11-07` and
`2026-09-19/_DSC3290`, all AF frames, read only. One run, about 9 s.

Kept were the pairs a count takes, as in that measurement: at 320x224 a score
of at least `GATE_LOW_RES_CONFIDENCE` (0.6) and a long side within
`GATE_MIN_SIZE` to `GATE_MAX_SIZE` (0.5 to 2.0) of the judged face's; at
640x448 a score of at least `FACE_CONFIDENCE` (0.8) and `meshes_face`. That
gave the same 39 pairs, with the same overlap values to the third decimal, as
the archived table: 24 same face, 10 different person, 3 spurious, 2 unclear.
Each label is taken from that table, matched by frame, search and overlap.

Both boxes are in the preview's stored coordinates. The aspect ratio of a box
is width / height; "aspect between" is the larger aspect ratio of the two over
the smaller; "long side" is the larger box's long side over the smaller's.

## Pairs

| Overlap | Frame | Search | Label | Judged box (w x h) | Judged aspect | Other box (w x h) | Other aspect | Aspect between | Long side |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0.549 | `_DSC3886` | 640x448 | same face | 47 x 45 | 1.04 | 69 x 56 | 1.23 | 1.178 | 1.489 |
| 0.543 | `_DSC2169` | 320x224 | same face | 89 x 71 | 1.26 | 71 x 59 | 1.21 | 1.036 | 1.248 |
| 0.532 | **`_DSC1970`** | 320x224 | **different person** | 51 x 42 | 1.20 | 88 x 70 | 1.26 | 1.045 | 1.725 |
| 0.532 | `_DSC2360` | 320x224 | same face | 89 x 52 | 1.72 | 56 x 48 | 1.16 | 1.481 | 1.595 |
| 0.530 | `_DSC1927` | 320x224 | same face | 72 x 57 | 1.28 | 81 x 66 | 1.24 | 1.035 | 1.121 |
| 0.530 | `_DSC4877` | 320x224 | same face | 57 x 69 | 0.82 | 71 x 86 | 0.83 | 1.015 | 1.232 |
| 0.526 | `_DSC3443` | 320x224 | same face | 62 x 43 | 1.43 | 50 x 40 | 1.24 | 1.161 | 1.244 |
| 0.524 | `_DSC1542` | 320x224 | same face | 91 x 68 | 1.34 | 65 x 53 | 1.23 | 1.088 | 1.405 |
| 0.522 | `_DSC2012` | 320x224 | same face | 100 x 77 | 1.30 | 75 x 58 | 1.30 | 1.003 | 1.325 |
| 0.511 | `_DSC4009` | 320x224 | same face | 86 x 75 | 1.16 | 74 x 62 | 1.19 | 1.031 | 1.172 |
| 0.492 | `_DSC4198` | 320x224 | same face | 111 x 134 | 0.83 | 98 x 125 | 0.78 | 1.065 | 1.070 |
| 0.482 | `_DSC3118` | 320x224 | same face | 44 x 35 | 1.23 | 63 x 49 | 1.29 | 1.045 | 1.441 |
| 0.478 | `_DSC1606` | 320x224 | same face | 48 x 41 | 1.17 | 64 x 53 | 1.21 | 1.032 | 1.314 |
| 0.477 | `_DSC4554` | 320x224 | same face | 57 x 46 | 1.24 | 54 x 44 | 1.21 | 1.021 | 1.058 |
| 0.474 | `_DSC3543` | 320x224 | spurious | 50 x 44 | 1.16 | 100 x 76 | 1.30 | 1.124 | 1.974 |
| 0.470 | **`_DSC3958`** | 320x224 | **different person** | 44 x 60 | 0.72 | 50 x 63 | 0.80 | 1.100 | 1.048 |
| 0.459 | `_DSC2391` | 320x224 | same face | 48 x 41 | 1.16 | 54 x 44 | 1.25 | 1.073 | 1.139 |
| 0.451 | **`_DSC3584`** | 320x224 | **different person** | 75 x 52 | 1.45 | 84 x 70 | 1.21 | 1.205 | 1.117 |
| 0.450 | `_DSC5037` | 320x224 | same face | 84 x 52 | 1.62 | 69 x 57 | 1.20 | 1.351 | 1.223 |
| 0.444 | **`_DSC2617`** | 640x448 | **different person** | 56 x 80 | 0.70 | 53 x 79 | 0.68 | 1.030 | 1.017 |
| 0.425 | `_DSC3911` | 320x224 | same face | 91 x 61 | 1.49 | 61 x 49 | 1.25 | 1.190 | 1.493 |
| 0.423 | `_DSC2305` | 320x224 | same face | 77 x 46 | 1.68 | 74 x 59 | 1.26 | 1.332 | 1.043 |
| 0.420 | `_DSC2222` | 320x224 | same face | 71 x 56 | 1.26 | 72 x 57 | 1.26 | 1.001 | 1.011 |
| 0.406 | **`_DSC1970`** | 640x448 | **different person** | 51 x 42 | 1.20 | 98 x 65 | 1.51 | 1.258 | 1.931 |
| 0.375 | `_DSC2012` | 640x448 | same face | 100 x 77 | 1.30 | 70 x 55 | 1.27 | 1.022 | 1.428 |
| 0.365 | `_DSC4368` | 320x224 | spurious | 102 x 79 | 1.29 | 98 x 77 | 1.28 | 1.007 | 1.039 |
| 0.363 | **`_DSC1517`** | 320x224 | **different person** | 76 x 63 | 1.21 | 59 x 47 | 1.26 | 1.042 | 1.286 |
| 0.360 | `_DSC3848` | 320x224 | unclear | 71 x 49 | 1.46 | 66 x 55 | 1.21 | 1.209 | 1.074 |
| 0.347 | **`_DSC3847`** | 640x448 | **different person** | 73 x 53 | 1.38 | 64 x 47 | 1.37 | 1.002 | 1.137 |
| 0.341 | `_DSC2780` | 640x448 | same face | 298 x 286 | 1.04 | 176 x 148 | 1.19 | 1.145 | 1.693 |
| 0.339 | `_DSC2011` | 320x224 | same face | 96 x 66 | 1.45 | 65 x 52 | 1.26 | 1.155 | 1.484 |
| 0.334 | `_DSC4194` | 320x224 | same face | 131 x 145 | 0.90 | 107 x 158 | 0.68 | 1.333 | 1.093 |
| 0.326 | `_DSC2780` | 320x224 | same face | 298 x 286 | 1.04 | 170 x 146 | 1.17 | 1.119 | 1.753 |
| 0.325 | **`_DSC3820`** | 320x224 | **different person** | 87 x 62 | 1.39 | 73 x 62 | 1.19 | 1.172 | 1.187 |
| 0.325 | `_DSC3248` | 320x224 | unclear | 113 x 80 | 1.41 | 105 x 78 | 1.35 | 1.048 | 1.077 |
| 0.315 | **`_DSC4113`** | 320x224 | **different person** | 138 x 233 | 0.59 | 107 x 155 | 0.69 | 1.165 | 1.499 |
| 0.306 | `2026-09-19/_DSC3290` | 320x224 | spurious | 76 x 56 | 1.36 | 75 x 56 | 1.35 | 1.009 | 1.009 |
| 0.306 | `_DSC3813` | 320x224 | same face | 60 x 49 | 1.21 | 52 x 41 | 1.25 | 1.037 | 1.146 |
| 0.301 | **`_DSC2800`** | 320x224 | **different person** | 96 x 72 | 1.33 | 124 x 98 | 1.27 | 1.047 | 1.293 |

## The ranges

| | Pairs | Aspect between | Long side |
| --- | --- | --- | --- |
| Same face | 24 | 1.001 to 1.481 | 1.011 to 1.753 |
| Different person | 10 | 1.002 to 1.258 | 1.017 to 1.931 |

Both ranges of the different people sit inside the twins' ranges, near their
middle. The twins' long-side ratios (at most 1.753) stay under the highest
neighbour (1.931, `_DSC1970` at 640x448), and the gate's size band caps a
gate pair at 2.0, so the size has no room above the neighbours.

## Verdict

The rule fixed for the plan: a threshold on the aspect between, the long
side, or either of the two that matches at least 20 of the 24 same-face pairs
(80%) and none of the 10 different-person pairs.

- Aspect between above 1.258 (the highest neighbour): matches 4 twins
  (`_DSC2360` 1.481, `_DSC5037` 1.351, `_DSC4194` 1.333, `_DSC2305` 1.332),
  17%.
- Long side above 1.931 (the highest neighbour): matches no twin.
- Either of the two, at those cuts: 4 twins, 17%.
- The other direction (matching a pair below a cut) fails too: below the
  lowest neighbour (aspect 1.002, long side 1.017) only `_DSC2222` falls.
- A cut low enough to take 20 twins takes most neighbours: aspect between at
  1.022 or more matches 9 of the 10 different people, long side at 1.093 or
  more 8 of the 10.

No threshold meets the rule. **Outcome B: the double count is accepted.**
`same_face` keeps its two rules (IoU above `NMS_THRESHOLD`, overlap above
`SAME_FACE_OVERLAP`), and the 21 gate frames and 3 candidate pairs of
`2026-11-07` with a twin at overlap 0.3 to 0.55 stay double-counted.
