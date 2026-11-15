# `full_eye_sharp` measured on the preview against the full JPEG

Step 2 as first planned measured a file without a larger JPEG (a RAF, an HDR
PQ CR3, a body whose full image is the preview) on its preview, with the same
metric and the same thresholds. This compares the two sources on the labeled ARW frames,
which all have a full JPEG, so the same eyes are measured both ways. It is a
record, not a gate.

## Method

- A scratch `riffle-cli` subcommand (not committed) ran Step 2's
  `scan::extract_analysis` on the five rated folders under `D:\Photos\tests`
  (`2026-06-14`, `2026-07-18`, `2026-08-01`, `2026-08-22`, `2026-06-05`,
  each `-focus-sample`; 500 ARW), 8 threads, release build, and for each
  frame with a judged face called `fullsharp::full_eye_sharpness` once more
  on the preview bytes with the same eye centers (scale 1).
- The full-size values it produced equal `frames.tsv`'s `full_eye_sharp` on
  all 345 rated frames (no mismatch beyond 1e-9), so Step 2's measure is the
  one Step 1 derived `LOW_BELOW` on, now for non-candidates too: all 478
  frames with a judged face were measured (432 `Candidate`, 46
  `NotCandidate`), the 22 `Unknown` without a face were not.
- The ratings are `frames.tsv`'s. `SOFT_BELOW` = 0.162, `LOW_BELOW` = 0.110
  (`low-threshold.md`).

## Result

Frames under each threshold, by the source measured:

| Folders | Stars | n | Under `SOFT_BELOW`, full | Under `SOFT_BELOW`, preview | Under `LOW_BELOW`, full | Under `LOW_BELOW`, preview |
| --- | --- | --- | --- | --- | --- | --- |
| Held-out | 3 | 154 | 29 | 146 | 3 | 91 |
| Held-out | 2 | 39 | 14 | 39 | 5 | 32 |
| Held-out | 1 | 88 | 47 | 84 | 13 | 68 |
| Fresh | 3 | 27 | 7 | 26 | 0 | 15 |
| Fresh | 2 | 18 | 10 | 18 | 1 | 14 |
| Fresh | 1 | 13 | 9 | 12 | 5 | 11 |

- The preview value is about half the full-size one: median 0.091 against
  0.190 held-out (median ratio 0.47), 0.082 against 0.177 fresh (0.44).
- The ranks agree only in part: Spearman 0.71 held-out, 0.81 fresh.
- 3 stars against the rest, AUC: held-out 0.648 on the preview against
  0.694 at full size; fresh 0.626 against 0.737.

## Reading

On a preview the thresholds calibrated at full size do not hold: 146 of 154
held-out 3-star frames (95%) would not be Good and 91 (59%) would be Low.
Under Step 3's rule, a file measured on its preview (RAF, HDR PQ CR3, the
bodies with no larger JPEG) would land in Low most of the time whatever its
focus. The preview still ranks the frames (AUC 0.63-0.65), so the gap is
mostly a scale offset, not noise.

## Decision

The user decided on 2026-11-15 that a file without a larger JPEG is not
measured: it has no `full_eye_sharp` and so is Neutral. Step 2 dropped the
preview path again; this record is why.
