# Culling cues: findings

The readings pre-registered in [plan.md](plan.md) ("Pre-registration",
committed in `68fee3f7` before the first run of the reader), computed by
[culling.py](culling.py) on 2026-11-28. The tables under "The reader's
output" are its stdout as printed (headings demoted one level); a second
run printed it byte-identically.

## The primary reading

The seven culled folders, 11,196 dump frames, 1,388 kept (12.4%). The
frames with a dump `|pitch|` > 45 against all other frames:

| Quantity | Bin (`\|pitch\|` > 45) | Complement |
| --- | ---: | ---: |
| Frames | 242 | 10,954 |
| Kept | 23 | 1,365 |
| Kept share | 9.5% (95%: 5.2% to 13.0%) | 12.5% |
| Culled rate | 90.5% | 87.5% |

The ratio of the kept shares is **0.763** (95%: 0.442 to 1.017;
leave-one-folder-out 0.635 to 0.813; no bootstrap draw dropped). The culled
rates differ by 3.0 points (95%: -0.2 to 6.7). The bin holds 1.7% of the
kept frames.

## Verdicts

- **Bar 1, a cue catches culled frames: not met.** The ratio is 0.763,
  above 0.5, and the 97.5% quantile of the ratio is 1.017, not below 1.
- **Bar 2, a cue is usable sorted last on its own: not met.** It needs
  bar 1. Its own conditions hold (kept share 9.5% <= 25%, its 97.5%
  quantile 13.0% <= 35%, 1.7% of the kept frames <= 5%), and, as the plan
  says, they do not discriminate at this base rate.

No secondary reading on the culled label meets either bar's form (and none
would be a pass on its own). The closest are EAR <= 0.137 (ratio 0.652, 95% 0.521 to 0.763)
and `eye_focus` < 0.9 (0.680, 0.570 to 0.755): both intervals sit below 1
but above 0.5. Pitch down alone reads 0.746 (0.368 to 1.023); pitch up, 16
frames, reads 1.008. `|yaw|` > 35 reads 1.053: no lift at all. On the pick
label over the 26 folders with a pick, `|pitch|` > 45 reads 0.871 (0.754 to
0.969), closer to 1 than in the culled folders; `2026-08-29` alone reads
0.892. The one secondary reading that does meet both bars' form is
`2026-08-29`'s bottom sharpness decile on the pick label (ratio 0.458, kept
share 6.7%, 4.8% of the picks); it means little: its one-cluster bootstrap
makes the interval the point, and a secondary reading cannot pass on its
own. The Decision does not change.

## Decision

By the follow-up rule, **branch 2: nothing.** The primary reading does not
meet bar 1, so no later plan reads today's pose cue against the picks, and
the `todo.md` section "Core: read the cue-labels round once the user has
labeled it" is removed. No bar was changed.

A head pitch past 45 degrees lowers the chance that a frame was kept from
12.5% to 9.5%, about a quarter, not the halving bar 1 asks, and seven
folders cannot rule out no effect. That agrees with reject-filter-eval's
reading of `face away` against the picks, which found no lift.

The 900 copies of the cue-labels round at
`D:\Photos\tests\2026-11-28-cue-labels\` are kept, not deleted. The blind
"OK to discard?" round stays possible: its pre-registration
([cues.md](../_archived/20261128-cue-labels/cues.md)) is untouched and can
still be read if the user labels the round later.

## Caveats

- The label is "not picked", stricter than "OK to discard": all 1,388 kept
  frames are dump picks and 5 dump picks are gone (all in `2026-09-19`).
- The picks were made before the dump and, by the sidecar mtimes (in the
  output), before the pose cue existed (2026-08-02 to 2026-09-01 in five
  folders, 2026-09-26 / 27 in the two September folders); whether the user
  saw a strip mark while picking the two September folders is unknown.
- One photographer, seven folders, mostly basketball. `2026-08-01` holds 84
  of the 242 bin frames.
- The readings are the dump's, which today's app no longer makes on every
  frame.
- Seven clusters make the bootstrap coarse; the `pitch` > 45 reading
  dropped 17 of its 1,000 draws (an empty bin).
- The earlier labels in these folders read in the same direction as the
  culling (strict `bad` 0 of 54 kept, 24 of 355 culled), but on few kept
  frames.

## The reader's output

### Folders

| Folder | Role | Dump frames | On disk | Dump picks | `\|pitch\|` > 45 | Known (frames / on disk / picks / pitch) | Folder mtime |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-06-05` | intact (left out) | 1337 | 1337 | 0 | 19 | - | 2026-06-07 |
| `2026-06-13` | intact (left out) | 933 | 933 | 118 | 7 | - | 2026-06-13 |
| `2026-06-14` | intact (left out) | 1520 | 1520 | 161 | 32 | - | 2026-07-07 |
| `2026-06-20` | intact (left out) | 1050 | 1050 | 133 | 11 | - | 2026-06-20 |
| `2026-07-02` | intact (left out) | 106 | 106 | 15 | 0 | - | 2026-07-02 |
| `2026-07-05` | intact (left out) | 1415 | 1415 | 255 | 38 | - | 2026-11-09 |
| `2026-07-11` | intact (left out) | 468 | 468 | 57 | 17 | - | 2026-07-15 |
| `2026-07-18` | intact (left out) | 1545 | 1545 | 292 | 41 | - | 2026-07-19 |
| `2026-07-21` | intact (left out) | 32 | 32 | 7 | 0 | - | 2026-08-07 |
| `2026-07-22` | culled | 189 | 32 | 32 | 18 | 189 / 32 / 32 / 18 | 2026-11-07 |
| `2026-07-23` | intact (left out) | 18 | 18 | 4 | 0 | - | 2026-08-07 |
| `2026-07-24` | intact (left out) | 218 | 218 | 34 | 11 | - | 2026-08-05 |
| `2026-07-26` | culled | 64 | 7 | 7 | 1 | 64 / 7 / 7 / 1 | 2026-11-07 |
| `2026-07-30` | culled | 936 | 105 | 105 | 34 | 936 / 105 / 105 / 34 | 2026-11-07 |
| `2026-07-31` | intact (left out) | 349 | 349 | 52 | 13 | - | 2026-08-05 |
| `2026-08-01` | culled | 3401 | 463 | 463 | 84 | 3401 / 463 / 463 / 84 | 2026-11-07 |
| `2026-08-02` | intact (left out) | 150 | 150 | 28 | 11 | - | 2026-08-03 |
| `2026-08-08` | intact (left out) | 415 | 415 | 73 | 2 | - | 2026-08-09 |
| `2026-08-15` | intact (left out) | 324 | 324 | 37 | 3 | - | 2026-08-21 |
| `2026-08-16` | intact (left out) | 59 | 59 | 6 | 0 | - | 2026-08-22 |
| `2026-08-22` | culled | 2677 | 267 | 267 | 41 | 2677 / 267 / 267 / 41 | 2026-11-06 |
| `2026-08-29` | gone (left out) | 3451 | 0 | 475 | 65 | - | 2026-11-06 |
| `2026-09-13-b` | culled | 1795 | 232 | 232 | 13 | 1795 / 232 / 232 / 13 | 2026-11-07 |
| `2026-09-19` | culled | 2134 | 282 | 287 | 51 | 2134 / 282 / 287 / 51 | 2026-11-06 |
| `2026-09-27-a` | intact (left out) | 4830 | 4830 | 731 | 94 | - | 2026-10-14 |
| `2026-09-27-b` | intact (left out) | 1240 | 1240 | 171 | 23 | - | 2026-10-06 |
| `2026-10-03` | intact (left out) | 1803 | 1803 | 190 | 139 | - | 2026-10-11 |

Culled folders: 7 (2026-07-22, 2026-07-26, 2026-07-30, 2026-08-01, 2026-08-22, 2026-09-13-b, 2026-09-19), 11196 frames, 1388 kept. Known: 7 folders, 11196 frames, 1388 kept. Same folders as known: yes. Left out: 20 folders, 21263 frames.

### The first number

Kept share of the population: 1388 of 11196, 12.4% (culled 87.6%).

### The label's own check: kept against the dump's sidecar value, per folder

A frame's value is `Pick` when its `xmp` or `dop` says `Pick`, else `Reject`, else `None`, else `-`.

| Folder | Kept `Pick` | Kept `Reject` | Kept `None` | Kept `-` | Culled `Pick` | Culled `Reject` | Culled `None` | Culled `-` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 32 | 0 | 0 | 0 | 0 | 0 | 1 | 156 |
| `2026-07-26` | 7 | 0 | 0 | 0 | 0 | 0 | 0 | 57 |
| `2026-07-30` | 105 | 0 | 0 | 0 | 0 | 0 | 1 | 830 |
| `2026-08-01` | 463 | 0 | 0 | 0 | 0 | 0 | 32 | 2906 |
| `2026-08-22` | 267 | 0 | 0 | 0 | 0 | 0 | 13 | 2397 |
| `2026-09-13-b` | 232 | 0 | 0 | 0 | 0 | 7 | 7 | 1549 |
| `2026-09-19` | 282 | 0 | 0 | 0 | 5 | 50 | 39 | 1758 |
| all | 1388 | 0 | 0 | 0 | 5 | 57 | 93 | 9653 |

### Bin sizes in the population against "What is known"

| Bin | This run | Known |
| --- | --- | --- |
| `\|pitch\|` > 45 | 242 | 242 |
| `pitch` > 45 | 16 | 16 |
| `pitch` < -45 | 226 | 226 |
| EAR <= 0.137 | 937 | 937 |
| `\|yaw\|` > 35 | 2810 | 2810 |
| `eye_focus` < 0.9 | 3062 | 3062 |
| no `eye_focus` | 1294 | 1294 |
| bottom sharpness decile | 1116 | - |
| no `pitch` | 3855 | 3855 |

### Sidecars of the kept frames and folder mtimes, from the listing

| Folder | Kept frames' `.xmp` | `.xmp` mtime range | Kept frames' `.dop` | `.dop` mtime range | Folder mtime |
| --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 0 | - | 32 | 2026-08-07 to 2026-08-07 | 2026-11-07 |
| `2026-07-26` | 0 | - | 7 | 2026-08-07 to 2026-08-07 | 2026-11-07 |
| `2026-07-30` | 0 | - | 105 | 2026-08-04 to 2026-08-04 | 2026-11-07 |
| `2026-08-01` | 0 | - | 463 | 2026-08-02 to 2026-08-03 | 2026-11-07 |
| `2026-08-22` | 1 | 2026-09-22 to 2026-09-22 | 266 | 2026-08-30 to 2026-09-01 | 2026-11-06 |
| `2026-09-13-b` | 232 | 2026-09-27 to 2026-09-27 | 232 | 2026-09-27 to 2026-09-27 | 2026-11-07 |
| `2026-09-19` | 282 | 2026-09-26 to 2026-09-26 | 282 | 2026-09-26 to 2026-09-26 | 2026-11-06 |

### The primary and the secondary readings on the culled label

Population: the 7 culled folders. Kept = on disk. Intervals: 1000-draw folder-cluster bootstrap, seed 0, one draw matrix for every reading of this population, 2.5% and 97.5% quantiles; a draw whose bin is empty or whose complement has no kept frame is dropped and counted. The ratio is the bin's kept share over the complement's; the difference is the bin's culled rate minus the complement's.

| Bin | Bin frames | Bin kept | Bin kept share | 95% | Complement frames | Complement kept | Complement kept share | Ratio | 95% | Leave-one-folder-out | Bin culled rate | Complement culled rate | Difference (points) | 95% | Share of all kept frames in the bin | Dropped draws |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `\|pitch\|` > 45 | 242 | 23 | 9.5% | 5.2% to 13.0% | 10954 | 1365 | 12.5% | 0.763 | 0.442 to 1.017 | 0.635 to 0.813 | 90.5% | 87.5% | 3.0 | -0.2 to 6.7 | 1.7% | 0 |
| `pitch` > 45 | 16 | 2 | 12.5% | 0.0% to 18.2% | 11180 | 1386 | 12.4% | 1.008 | 0.000 to 1.389 | 0.000 to 1.267 | 87.5% | 87.6% | -0.1 | -5.1 to 13.2 | 0.1% | 17 |
| `pitch` < -45 | 226 | 21 | 9.3% | 4.5% to 13.2% | 10970 | 1367 | 12.5% | 0.746 | 0.368 to 1.023 | 0.589 to 0.834 | 90.7% | 87.5% | 3.2 | -0.3 to 7.8 | 1.5% | 0 |
| EAR <= 0.137 | 937 | 78 | 8.3% | 6.7% to 10.3% | 10259 | 1310 | 12.8% | 0.652 | 0.521 to 0.763 | 0.597 to 0.690 | 91.7% | 87.2% | 4.4 | 3.1 to 6.4 | 5.6% | 0 |
| `\|yaw\|` > 35 | 2810 | 362 | 12.9% | 8.3% to 16.8% | 8386 | 1026 | 12.2% | 1.053 | 0.708 to 1.361 | 0.857 to 1.166 | 87.1% | 87.8% | -0.6 | -4.5 to 3.4 | 26.1% | 0 |
| `eye_focus` < 0.9 | 3062 | 283 | 9.2% | 7.0% to 10.7% | 8134 | 1105 | 13.6% | 0.680 | 0.570 to 0.755 | 0.611 to 0.716 | 90.8% | 86.4% | 4.3 | 3.3 to 5.9 | 20.4% | 0 |
| no `eye_focus` | 1294 | 127 | 9.8% | 8.4% to 11.3% | 9902 | 1261 | 12.7% | 0.771 | 0.628 to 0.981 | 0.698 to 0.875 | 90.2% | 87.3% | 2.9 | 0.2 to 5.2 | 9.1% | 0 |
| bottom sharpness decile | 1116 | 100 | 9.0% | 7.3% to 12.0% | 10080 | 1288 | 12.8% | 0.701 | 0.538 to 0.972 | 0.636 to 0.821 | 91.0% | 87.2% | 3.8 | 0.3 to 6.4 | 7.2% | 0 |

#### Bars, primary reading (`|pitch|` > 45)

| Condition | Value | Met |
| --- | --- | --- |
| Bar 1: ratio <= 0.5 | 0.763 | no |
| Bar 1: ratio 97.5% quantile < 1 | 1.017 | no |
| **Bar 1** | - | **not met** |
| Bar 2: bin kept share <= 25% | 9.5% | yes |
| Bar 2: its 97.5% quantile <= 35% | 13.0% | yes |
| Bar 2: bin holds <= 5% of all kept frames | 1.7% | yes |
| **Bar 2** (needs bar 1) | - | **not met** |

Follow-up rule: branch 2.

Secondary readings in the bars' form (not a pass on their own):

| Bin | Bar 1 form | Bar 2 form |
| --- | --- | --- |
| `pitch` > 45 | no | no |
| `pitch` < -45 | no | no |
| EAR <= 0.137 | no | no |
| `\|yaw\|` > 35 | no | no |
| `eye_focus` < 0.9 | no | no |
| no `eye_focus` | no | no |
| bottom sharpness decile | no | no |

#### Per folder, culled label

`|pitch|` > 45:

| Folder | Bin frames | Bin kept | Bin kept share | Complement frames | Complement kept | Complement kept share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 18 | 4 | 22.2% | 171 | 28 | 16.4% | 1.357 |
| `2026-07-26` | 1 | 0 | 0.0% | 63 | 7 | 11.1% | 0.000 |
| `2026-07-30` | 34 | 2 | 5.9% | 902 | 103 | 11.4% | 0.515 |
| `2026-08-01` | 84 | 11 | 13.1% | 3317 | 452 | 13.6% | 0.961 |
| `2026-08-22` | 41 | 2 | 4.9% | 2636 | 265 | 10.1% | 0.485 |
| `2026-09-13-b` | 13 | 0 | 0.0% | 1782 | 232 | 13.0% | 0.000 |
| `2026-09-19` | 51 | 4 | 7.8% | 2083 | 278 | 13.3% | 0.588 |

`pitch` > 45:

| Folder | Bin frames | Bin kept | Bin kept share | Complement frames | Complement kept | Complement kept share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 0 | 0 | - | 189 | 32 | 16.9% | - |
| `2026-07-26` | 0 | 0 | - | 64 | 7 | 10.9% | - |
| `2026-07-30` | 0 | 0 | - | 936 | 105 | 11.2% | - |
| `2026-08-01` | 0 | 0 | - | 3401 | 463 | 13.6% | - |
| `2026-08-22` | 4 | 0 | 0.0% | 2673 | 267 | 10.0% | 0.000 |
| `2026-09-13-b` | 1 | 0 | 0.0% | 1794 | 232 | 12.9% | 0.000 |
| `2026-09-19` | 11 | 2 | 18.2% | 2123 | 280 | 13.2% | 1.379 |

`pitch` < -45:

| Folder | Bin frames | Bin kept | Bin kept share | Complement frames | Complement kept | Complement kept share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 18 | 4 | 22.2% | 171 | 28 | 16.4% | 1.357 |
| `2026-07-26` | 1 | 0 | 0.0% | 63 | 7 | 11.1% | 0.000 |
| `2026-07-30` | 34 | 2 | 5.9% | 902 | 103 | 11.4% | 0.515 |
| `2026-08-01` | 84 | 11 | 13.1% | 3317 | 452 | 13.6% | 0.961 |
| `2026-08-22` | 37 | 2 | 5.4% | 2640 | 265 | 10.0% | 0.539 |
| `2026-09-13-b` | 12 | 0 | 0.0% | 1783 | 232 | 13.0% | 0.000 |
| `2026-09-19` | 40 | 2 | 5.0% | 2094 | 280 | 13.4% | 0.374 |

EAR <= 0.137:

| Folder | Bin frames | Bin kept | Bin kept share | Complement frames | Complement kept | Complement kept share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 33 | 4 | 12.1% | 156 | 28 | 17.9% | 0.675 |
| `2026-07-26` | 27 | 2 | 7.4% | 37 | 5 | 13.5% | 0.548 |
| `2026-07-30` | 135 | 12 | 8.9% | 801 | 93 | 11.6% | 0.766 |
| `2026-08-01` | 217 | 25 | 11.5% | 3184 | 438 | 13.8% | 0.837 |
| `2026-08-22` | 292 | 21 | 7.2% | 2385 | 246 | 10.3% | 0.697 |
| `2026-09-13-b` | 116 | 7 | 6.0% | 1679 | 225 | 13.4% | 0.450 |
| `2026-09-19` | 117 | 7 | 6.0% | 2017 | 275 | 13.6% | 0.439 |

`|yaw|` > 35:

| Folder | Bin frames | Bin kept | Bin kept share | Complement frames | Complement kept | Complement kept share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 58 | 9 | 15.5% | 131 | 23 | 17.6% | 0.884 |
| `2026-07-26` | 5 | 1 | 20.0% | 59 | 6 | 10.2% | 1.967 |
| `2026-07-30` | 227 | 22 | 9.7% | 709 | 83 | 11.7% | 0.828 |
| `2026-08-01` | 803 | 150 | 18.7% | 2598 | 313 | 12.0% | 1.550 |
| `2026-08-22` | 608 | 38 | 6.2% | 2069 | 229 | 11.1% | 0.565 |
| `2026-09-13-b` | 414 | 48 | 11.6% | 1381 | 184 | 13.3% | 0.870 |
| `2026-09-19` | 695 | 94 | 13.5% | 1439 | 188 | 13.1% | 1.035 |

`eye_focus` < 0.9:

| Folder | Bin frames | Bin kept | Bin kept share | Complement frames | Complement kept | Complement kept share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 27 | 3 | 11.1% | 162 | 29 | 17.9% | 0.621 |
| `2026-07-26` | 10 | 1 | 10.0% | 54 | 6 | 11.1% | 0.900 |
| `2026-07-30` | 62 | 6 | 9.7% | 874 | 99 | 11.3% | 0.854 |
| `2026-08-01` | 1277 | 141 | 11.0% | 2124 | 322 | 15.2% | 0.728 |
| `2026-08-22` | 585 | 36 | 6.2% | 2092 | 231 | 11.0% | 0.557 |
| `2026-09-13-b` | 519 | 48 | 9.2% | 1276 | 184 | 14.4% | 0.641 |
| `2026-09-19` | 582 | 48 | 8.2% | 1552 | 234 | 15.1% | 0.547 |

no `eye_focus`:

| Folder | Bin frames | Bin kept | Bin kept share | Complement frames | Complement kept | Complement kept share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 13 | 2 | 15.4% | 176 | 30 | 17.0% | 0.903 |
| `2026-07-26` | 12 | 1 | 8.3% | 52 | 6 | 11.5% | 0.722 |
| `2026-07-30` | 76 | 3 | 3.9% | 860 | 102 | 11.9% | 0.333 |
| `2026-08-01` | 533 | 47 | 8.8% | 2868 | 416 | 14.5% | 0.608 |
| `2026-08-22` | 196 | 22 | 11.2% | 2481 | 245 | 9.9% | 1.137 |
| `2026-09-13-b` | 282 | 31 | 11.0% | 1513 | 201 | 13.3% | 0.827 |
| `2026-09-19` | 182 | 21 | 11.5% | 1952 | 261 | 13.4% | 0.863 |

bottom sharpness decile:

| Folder | Bin frames | Bin kept | Bin kept share | Complement frames | Complement kept | Complement kept share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-07-22` | 18 | 5 | 27.8% | 171 | 27 | 15.8% | 1.759 |
| `2026-07-26` | 6 | 0 | 0.0% | 58 | 7 | 12.1% | 0.000 |
| `2026-07-30` | 93 | 8 | 8.6% | 843 | 97 | 11.5% | 0.748 |
| `2026-08-01` | 340 | 23 | 6.8% | 3061 | 440 | 14.4% | 0.471 |
| `2026-08-22` | 267 | 23 | 8.6% | 2410 | 244 | 10.1% | 0.851 |
| `2026-09-13-b` | 179 | 24 | 13.4% | 1616 | 208 | 12.9% | 1.042 |
| `2026-09-19` | 213 | 17 | 8.0% | 1921 | 265 | 13.8% | 0.579 |

### Secondary readings on the pick label

A dump pick against the rest, the same bins; the sharpness decile is taken over each folder's dump frames.

#### `2026-08-29` alone

3451 frames, 475 picks (13.8%). One cluster: the bootstrap draws the one folder every time, so its interval is the point.

| Bin | Bin frames | Bin pick | Bin pick share | 95% | Complement frames | Complement pick | Complement pick share | Ratio | 95% | Leave-one-folder-out | Bin non-pick rate | Complement non-pick rate | Difference (points) | 95% | Share of all pick frames in the bin | Dropped draws |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `\|pitch\|` > 45 | 65 | 8 | 12.3% | 12.3% to 12.3% | 3386 | 467 | 13.8% | 0.892 | 0.892 to 0.892 | - | 87.7% | 86.2% | 1.5 | 1.5 to 1.5 | 1.7% | 0 |
| `pitch` > 45 | 7 | 1 | 14.3% | 14.3% to 14.3% | 3444 | 474 | 13.8% | 1.038 | 1.038 to 1.038 | - | 85.7% | 86.2% | -0.5 | -0.5 to -0.5 | 0.2% | 0 |
| `pitch` < -45 | 58 | 7 | 12.1% | 12.1% to 12.1% | 3393 | 468 | 13.8% | 0.875 | 0.875 to 0.875 | - | 87.9% | 86.2% | 1.7 | 1.7 to 1.7 | 1.5% | 0 |
| EAR <= 0.137 | 185 | 26 | 14.1% | 14.1% to 14.1% | 3266 | 449 | 13.7% | 1.022 | 1.022 to 1.022 | - | 85.9% | 86.3% | -0.3 | -0.3 to -0.3 | 5.5% | 0 |
| `\|yaw\|` > 35 | 816 | 114 | 14.0% | 14.0% to 14.0% | 2635 | 361 | 13.7% | 1.020 | 1.020 to 1.020 | - | 86.0% | 86.3% | -0.3 | -0.3 to -0.3 | 24.0% | 0 |
| `eye_focus` < 0.9 | 1521 | 139 | 9.1% | 9.1% to 9.1% | 1930 | 336 | 17.4% | 0.525 | 0.525 to 0.525 | - | 90.9% | 82.6% | 8.3 | 8.3 to 8.3 | 29.3% | 0 |
| no `eye_focus` | 294 | 36 | 12.2% | 12.2% to 12.2% | 3157 | 439 | 13.9% | 0.881 | 0.881 to 0.881 | - | 87.8% | 86.1% | 1.7 | 1.7 to 1.7 | 7.6% | 0 |
| bottom sharpness decile | 345 | 23 | 6.7% | 6.7% to 6.7% | 3106 | 452 | 14.6% | 0.458 | 0.458 to 0.458 | - | 93.3% | 85.4% | 7.9 | 7.9 to 7.9 | 4.8% | 0 |

#### every dump folder with at least one pick (26 folders)

31122 frames, 4232 picks (13.6%). Own 1000-draw matrix over these 26 folders, seed 0.

| Bin | Bin frames | Bin pick | Bin pick share | 95% | Complement frames | Complement pick | Complement pick share | Ratio | 95% | Leave-one-folder-out | Bin non-pick rate | Complement non-pick rate | Difference (points) | 95% | Share of all pick frames in the bin | Dropped draws |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `\|pitch\|` > 45 | 749 | 89 | 11.9% | 10.0% to 13.6% | 30373 | 4143 | 13.6% | 0.871 | 0.754 to 0.969 | 0.853 to 0.892 | 88.1% | 86.4% | 1.8 | 0.4 to 3.3 | 2.1% | 0 |
| `pitch` > 45 | 185 | 27 | 14.6% | 12.2% to 21.2% | 30937 | 4205 | 13.6% | 1.074 | 0.951 to 1.528 | 1.014 to 1.300 | 85.4% | 86.4% | -1.0 | -7.2 to 0.7 | 0.6% | 0 |
| `pitch` < -45 | 564 | 62 | 11.0% | 8.8% to 13.2% | 30558 | 4170 | 13.6% | 0.806 | 0.660 to 0.938 | 0.778 to 0.839 | 89.0% | 86.4% | 2.7 | 0.9 to 4.6 | 1.5% | 0 |
| EAR <= 0.137 | 2326 | 228 | 9.8% | 8.2% to 11.3% | 28796 | 4004 | 13.9% | 0.705 | 0.610 to 0.794 | 0.674 to 0.719 | 90.2% | 86.1% | 4.1 | 2.9 to 5.4 | 5.4% | 0 |
| `\|yaw\|` > 35 | 7714 | 1113 | 14.4% | 12.2% to 16.3% | 23408 | 3119 | 13.3% | 1.083 | 0.949 to 1.226 | 1.033 to 1.117 | 85.6% | 86.7% | -1.1 | -2.9 to 0.7 | 26.3% | 0 |
| `eye_focus` < 0.9 | 8853 | 836 | 9.4% | 8.3% to 10.2% | 22269 | 3396 | 15.2% | 0.619 | 0.563 to 0.670 | 0.601 to 0.632 | 90.6% | 84.8% | 5.8 | 4.7 to 6.9 | 19.8% | 0 |
| no `eye_focus` | 3593 | 439 | 12.2% | 10.0% to 14.7% | 27529 | 3793 | 13.8% | 0.887 | 0.737 to 1.066 | 0.825 to 0.936 | 87.8% | 86.2% | 1.6 | -0.9 to 3.7 | 10.4% | 0 |
| bottom sharpness decile | 3101 | 255 | 8.2% | 6.8% to 9.6% | 28021 | 3977 | 14.2% | 0.579 | 0.477 to 0.695 | 0.554 to 0.607 | 91.8% | 85.8% | 6.0 | 4.1 to 7.8 | 6.0% | 0 |

Per folder, `|pitch|` > 45 on the pick label:

| Folder | Bin frames | Bin pick | Bin pick share | Complement frames | Complement pick | Complement pick share | Ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `2026-06-13` | 7 | 0 | 0.0% | 926 | 118 | 12.7% | 0.000 |
| `2026-06-14` | 32 | 2 | 6.2% | 1488 | 159 | 10.7% | 0.585 |
| `2026-06-20` | 11 | 2 | 18.2% | 1039 | 131 | 12.6% | 1.442 |
| `2026-07-02` | 0 | 0 | - | 106 | 15 | 14.2% | - |
| `2026-07-05` | 38 | 7 | 18.4% | 1377 | 248 | 18.0% | 1.023 |
| `2026-07-11` | 17 | 3 | 17.6% | 451 | 54 | 12.0% | 1.474 |
| `2026-07-18` | 41 | 7 | 17.1% | 1504 | 285 | 18.9% | 0.901 |
| `2026-07-21` | 0 | 0 | - | 32 | 7 | 21.9% | - |
| `2026-07-22` | 18 | 4 | 22.2% | 171 | 28 | 16.4% | 1.357 |
| `2026-07-23` | 0 | 0 | - | 18 | 4 | 22.2% | - |
| `2026-07-24` | 11 | 1 | 9.1% | 207 | 33 | 15.9% | 0.570 |
| `2026-07-26` | 1 | 0 | 0.0% | 63 | 7 | 11.1% | 0.000 |
| `2026-07-30` | 34 | 2 | 5.9% | 902 | 103 | 11.4% | 0.515 |
| `2026-07-31` | 13 | 2 | 15.4% | 336 | 50 | 14.9% | 1.034 |
| `2026-08-01` | 84 | 11 | 13.1% | 3317 | 452 | 13.6% | 0.961 |
| `2026-08-02` | 11 | 2 | 18.2% | 139 | 26 | 18.7% | 0.972 |
| `2026-08-08` | 2 | 0 | 0.0% | 413 | 73 | 17.7% | 0.000 |
| `2026-08-15` | 3 | 1 | 33.3% | 321 | 36 | 11.2% | 2.972 |
| `2026-08-16` | 0 | 0 | - | 59 | 6 | 10.2% | - |
| `2026-08-22` | 41 | 2 | 4.9% | 2636 | 265 | 10.1% | 0.485 |
| `2026-08-29` | 65 | 8 | 12.3% | 3386 | 467 | 13.8% | 0.892 |
| `2026-09-13-b` | 13 | 0 | 0.0% | 1782 | 232 | 13.0% | 0.000 |
| `2026-09-19` | 51 | 5 | 9.8% | 2083 | 282 | 13.5% | 0.724 |
| `2026-09-27-a` | 94 | 11 | 11.7% | 4736 | 720 | 15.2% | 0.770 |
| `2026-09-27-b` | 23 | 2 | 8.7% | 1217 | 169 | 13.9% | 0.626 |
| `2026-10-03` | 139 | 17 | 12.2% | 1664 | 173 | 10.4% | 1.176 |

### Earlier labels against kept and culled

Read beside the culling, not pooled into it. Weighted shares use each round's `1/π` (`inclusion_prob`).

| Round | Side | Labeled frames | `bad` / `discard` | Other | Share | Weighted frames | Weighted share |
| --- | --- | --- | --- | --- | --- | --- | --- |
| strict | kept | 54 | 0 | 54 | 0.0% | 1527.7 | 0.0% |
| strict | culled | 355 | 24 | 331 | 6.8% | 9182.0 | 6.4% |
| discard | kept | 13 | 5 | 8 | 38.5% | 1147.4 | 37.3% |
| discard | culled | 176 | 117 | 59 | 66.5% | 9343.7 | 53.2% |
| model-search | kept | 12 | 3 | 9 | 25.0% | 1616.5 | 30.4% |
| model-search | culled | 230 | 153 | 77 | 66.5% | 9646.6 | 56.4% |

### Checks

| Check | Expected | This run |
| --- | --- | --- |
| Library files (27 folders, recursive) before / after | equal | 44243 / 44243 |
| Library paths changed / added / removed | 0 / 0 / 0 | 0 / 0 / 0 |
| `labels.tsv` rows | 11196 | 11196 |
| `labels.tsv` SHA-256 | - | 3eb276dacd849b6452dc1951c961f859202e97711478ac35bb857dc64abc42a1 |
