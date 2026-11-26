# Step 1 measurement: the gate from 0.6 without an AF point

Measured on 2026-11-26 on Windows 11, release `handpic-cli` built from
`739a17ee` (before) and from the same tree with `cue_and_gate`'s `None` arm
counting from `allfaces::GATE_LOW_RES_CONFIDENCE` (after), 16 threads,
`handpic-cli features <folder or files> 16`, files on the local drive
(`D:\photos\2026`). Every folder of the plan was found. The `2026-11-07` and
`2026-10-03` slices are the first 300 and 100 ARW by name.

## Gate counts

Frames at gate count N or more, before -> after:

| Folder | Frames | AF point | N >= 3 | N >= 4 | N >= 5 | Gate counts changed | Newly at N >= 5 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-02-01` | 146 DNG | none | 9 -> 15 | 3 -> 9 | 0 -> 3 | 21 | 3 (3 groups, 0 not) |
| `2026-09-05` | 411 DNG | none | 10 -> 16 | 0 -> 0 | 0 -> 0 | 22 | 0 |
| `2026-10-31` | 954 DNG | none | 13 -> 26 | 1 -> 2 | 0 -> 0 | 51 | 0 |
| `2026-09-19` | 282 ARW | trusted | 63 -> 63 | 26 -> 26 | 8 -> 8 | 0 | 0 |
| `2026-11-07`, first 300 | 300 ARW | trusted | 181 -> 181 | 99 -> 99 | 37 -> 37 | 0 | 0 |
| `2026-10-03`, first 100 | 100 ARW | trusted | 35 -> 35 | 14 -> 14 | 6 -> 6 | 0 | 0 |

The `2026-02-01` "before" matches the known 9 / 3 / 0. Every other
`features` column (all but `gate` and the two timings) was identical before
and after on every file of the six sets.

## Frames newly at N >= 5

Checked on `handpic-cli faces` drawings of the upright preview (boxes and
scores of the whole-image search):

| File | Gate before -> after | Faces (scores) | Class |
| --- | --- | --- | --- |
| `2026-02-01/L1005161.DNG` | 4 -> 7 | 0.89, 0.86, 0.86, 0.80, 0.75, 0.64, 0.64 | group: 7 people posed on a swing, every face counted |
| `2026-02-01/L1005172.DNG` | 3 -> 7 | 0.90, 0.86, 0.84, 0.74, 0.69, 0.68, 0.64 | group: the same 7 on the swing |
| `2026-02-01/L1005173.DNG` | 3 -> 5 | 0.89, 0.85, 0.85, 0.76, 0.72 | group: the same swing, one child facing away and one face missed |

## Stop condition

Not met: no no-AF folder has a frame newly at N >= 5 that is not a group
(0 against 3 groups gained in `2026-02-01`, 0 of 0 in the other two).
