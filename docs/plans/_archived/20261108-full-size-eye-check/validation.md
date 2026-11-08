# Validation of the full-size eye sharpness port

`riffle-cli facechoice` dumps (`D:\Photos\tests\20261108-good-eye-blur\rust\fc_*.tsv`)
ranked by [`rank.py`](./rank.py), Good frames only, against the prototype's
`fu` in `D:\Photos\tests\20261108-good-eye-blur\proto\m_*.json`.

| Folder | Good (plan) | Spearman vs. prototype | Prototype cut rank | Rust score at that rank | Frames under 0.162 (Rust) | Frames under 0.162 (prototype) | Frames on the other side of 0.162 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-07 | 903 (903) | 0.9996 | 117 / 118 | 0.1616 / 0.1620 | 118 | 118 | 4 |
| 2026-10-12a | 473 (473) | 0.9995 | 106 | 0.1622 | 105 | 106 | 3 |
| 2026-09-27-c | 303 (303) | 0.9994 | 77 | 0.1618 | 77 | 77 | 4 |

"Frames under 0.162" is the rank the threshold falls at: the last demoted
frame's rank, one less than the first frame at or above it (`rank.py` prints
the latter).

The acceptance files in 2026-11-07, both under the cut:

| File | Rust rank | Rust score |
| --- | --- | --- |
| `_DSC1536.ARW` | 2 | 0.0929 |
| `_DSC1508.ARW` | 7 | 0.1043 |

(2026-09-27-c holds an unrelated `_DSC1508.ARW`, which `rank.py` also prints:
rank 166, score 0.2037.)

## Threshold

`SOFT_BELOW` stays 0.162. The criterion holds on all three folders: it cuts
within a few frames of 117 / 106 (118 and 105) and at 77, not above it, on
2026-09-27-c.

## Differences from the prototype

The frames that change side all score within 0.003 of 0.162 in both
implementations, and they cancel out: the three cut ranks move by at most one.
The largest difference of any Good frame between the two scores is 0.009
(`_DSC0031.ARW` of 2026-09-27-c, 0.586 against 0.577, far from the cut), so the
Rust port's partial decode, its bicubic and the non-square resize of a crop
clamped at the image edge (Step 1's learnings) do not move the ranks. The
frames with the largest differences are not edge frames (`edge_gap` above 3).

Not measured: how many of the 13 hand-labeled Good picks in `labfit.json` fall
under the Rust threshold. It would need dumps of the labeled folders; the
prototype's figure is in the plan.
