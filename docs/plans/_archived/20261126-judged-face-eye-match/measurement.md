# Eye-landmark match: measurement

Measured on 2026-11-27 on the two AF folders of the all-faces-check
measurement (`2026-09-19`, 282 ARW, and the first 300 ARW of `2026-11-07`),
both reachable on the local drive. A scratch `handpic-cli` subcommand (not
committed) took each AF frame's judged face from `candidate::focus_cue` and
compared it with every face of the whole-preview search at `WHOLE_INPUT`
(640x448, what `candidates` sees) and at `GATE_INPUT` (320x224, what
`gate_count` sees on an AF frame). For each pair it printed the IoU and the
eye distance `d`: the larger of |left - left| and |right - right| over the
judged face's box long side.

The IoU split reproduces the all-faces-check numbers: on `2026-09-19`, 228
frames with a whole-image twin above IoU 0.3, 29 with no overlapping face, 1
(`_DSC1899`) with only an overlap at or under 0.3.

## The same face found again (IoU above 0.3)

| Folder | Search | Pairs | `d` p50 | p90 | p99 | max |
| --- | --- | --- | --- | --- | --- | --- |
| `2026-09-19` | 640x448 | 228 | 0.07 | 0.18 | 0.32 | 0.43 |
| `2026-09-19` | 320x224 | 171 | 0.20 | 0.35 | 0.51 | 0.54 |
| `2026-11-07` slice | 640x448 | 267 | 0.09 | 0.21 | 0.37 | 0.42 |
| `2026-11-07` slice | 320x224 | 197 | 0.25 | 0.47 | 0.57 | 0.69 |

A face's own two eyes are 0.04 (a profile) to 0.4 of the box side apart, so
at 320x224 the same face's eye landmarks often move by more than its own eye
distance.

## Overlaps at or under 0.3, looked at by eye

| Frame | Search | IoU | `d` | What it is |
| --- | --- | --- | --- | --- |
| `2026-09-19/_DSC1899` | 640x448 | 0.24 | 0.35 | the same profile face (one detection puts the eyes on the ear) |
| `2026-09-19/_DSC1899` | 320x224 | 0.25 | 0.45 | the same |
| `2026-11-07/_DSC1543` | 640x448 | 0.20 | 0.44 | the same profile face, eyes on the ear |
| `2026-11-07/_DSC1524` | 320x224 | 0.25 | 0.57 | the same face, a smaller partial box |
| `2026-11-07/_DSC1517` | 320x224 | 0.15 | 0.47 | a different person, partly behind the judged one |
| `2026-09-19/_DSC3290` | 320x224 | 0.18 | 0.49 | a spurious box half on the judged head's hair |

Every other whole-image face (not overlapping, or not looked at) was 0.50 or
more away; the nearest distinct-looking ones were at 0.50 to 0.57.

## What the rules would do

Replacing the IoU match (the plan's recommendation): AF frames whose gate
count (score >= 0.6, size band) or whose candidate list (score >= 0.8) would
gain a face, the judged face's own twin now counted:

| Cut on `d` | `2026-09-19` gate / candidates | `2026-11-07` slice gate / candidates |
| --- | --- | --- |
| 0.2 | 84 / 4 | 133 / 6 |
| 0.3 | 33 / 2 | 78 / 1 |
| 0.45 | 4 / 0 | 29 / 0 |

against 1 frame (`_DSC1899`) double-counted today.

The union ("IoU above 0.3 or the eyes match") can only lower counts. To catch
the double counts seen (`d` 0.35 to 0.57) its cut would have to reach about
0.57, where it also drops the real neighbour of `_DSC1517` (0.47). A safe cut
(0.2) catches none of them and changes no measured frame.

No cut on `d` separates the same face from a neighbour on these previews: the
double counts are profiles and partial boxes, exactly where YuNet's eye
landmarks are least stable.
