# Experiment: preview `eye_focus` against full-size `full_eye_sharp`

Run on 2026-11-15 before this plan, in a scratch session whose files are not
kept (`dump.tsv`, `results.md`, `rerun.txt` and `check0605.py` under the
user's temp folder). This file keeps the numbers the plan rests on;
[`frames.tsv`](frames.tsv) keeps the per-frame values the Low threshold is
derived from ([`low-threshold.md`](low-threshold.md)).

## Data

- Body: ILCE-7M5 only. Every rated frame of the held-out folders had a
  judged face (all from the AF crop set), a full-size JPEG and a
  `full_eye_sharp`.
- Measure: a scratch `riffle-cli fullfocus` subcommand (reverted) dumped the
  preview's `eye_focus` and cue state and `full_eye_sharp` for every judged
  face, not only the candidates. Its `eye_focus` matched
  `scan::extract_analysis` on all 806 judged frames of the dump.
- Labels: the user's `xmp:Rating` in the sidecars under
  `D:\Photos\tests\<folder>`, 0 = skipped. 3 stars = the focus plane is on
  the eyes (noise allowed); 1 star = visibly off; 2 stars = in between.
- Held-out folders: `2026-06-14-focus-sample`, `2026-07-18-focus-sample`,
  `2026-08-01-focus-sample`, `2026-08-22-focus-sample`. 281 rated frames:
  3 stars 154, 2 stars 39, 1 star 88.
- Fresh folder, rated after the held-out analysis:
  `2026-06-05-focus-sample`. 58 rated frames with a judged face: 3 stars 27,
  2 stars 18, 1 star 13 (6 more rated frames have no judged face).

## AUCs

Pooled AUC with a 2000-draw bootstrap 95% interval; the difference is
`full_eye_sharp` minus `eye_focus` on the same frames (paired).

| Comparison | Frames | `eye_focus` | `full_eye_sharp` | Difference [95% CI] |
| --- | --- | ---: | ---: | --- |
| 3 stars vs the rest | held-out (154 / 127) | 0.619 [0.554, 0.681] | 0.694 [0.629, 0.750] | +0.075 [+0.022, +0.130] |
| 3 stars vs the rest | fresh (27 / 31) | 0.640 | 0.737 | +0.097 [+0.010, +0.200] |
| not 1 star vs 1 star | held-out (193 / 88) | 0.603 [0.525, 0.671] | 0.681 | +0.078 [+0.019, +0.144] |
| not 1 star vs 1 star | fresh (45 / 13) | 0.723 | 0.737 | +0.014 [-0.090, +0.132] (across 0) |

Within-folder AUC (pairs from one folder only), held-out, 3 stars vs the
rest: `eye_focus` 0.630, `full_eye_sharp` 0.691. Where the preview eye region
counts (141 held-out frames), `eye_focus` is at chance (0.519) and
`full_eye_sharp` 0.596 (difference +0.077 [-0.026, +0.180]).

`full_eye_sharp` was used as is, with no refit, so it carries no
folder-specific calibration.

## Rejected: full-size Laplacian / edge-width refits

The preview model's features (the Laplacian variance and the mean edge width
of each eye region) were also measured on the full-size JPEG, three ways: the
mesh region at native full-size pixels (`nat`), that region resized to a
154 px longer side (`r154`), and `fullsharp`'s square crop resized to
154 x 154 (`sq154`). The preview model's weights applied to them are at or
below chance (3 stars vs the rest, held-out: 0.441 / 0.509 / 0.526). Refitted
on the ratings they look better under a stratified 5-fold CV (`nat` 0.645,
preview + `nat` 0.711) but not under leave-one-folder-out: `nat` 0.543,
`r154` 0.543, `sq154` 0.529, preview + `nat` 0.601 (0.53–0.60). Their CV gains
were folder-specific calibration, so none of them replaces the preview model.

## Relabeling

The first ratings set 3 stars as "lashes resolved at 1:1". Those labels
tracked ISO: ln ISO alone gave an AUC of 0.806 for 3 stars vs the rest (ISO +
face size 0.915), the 3-star share falling from 24/32 under ISO 800 to 10/121
at ISO 3200 and above. The user re-rated with 3 stars as "the focus plane is
on the eyes, noise allowed" (85 frames changed: 2 → 3 stars 79, 2 → 1 star 5,
2 → unrated 1; every 3 and 1 star kept). ISO alone then gives about 0.60
(0.61 pooled, 0.60 leave-one-folder-out), and every number above is on the
revised ratings.

## Ranking ablation behind `Good photo`

The folder-wide ranking evaluation
([`folderwide.md`](../20261015-folder-wide-ranking-eval/folderwide.md),
"Drop one") reran the held-out ranking without each feature group. Neither
`AF eye focus` (the `eye_focus` column) nor `Good photo` contributes: the
frame AUC moves by -0.000 [-0.005, +0.004] and +0.001 [-0.003, +0.006] on the
`F0+ctx` base and -0.002 [-0.005, +0.001] and -0.003 [-0.009, +0.002] on the
`W0` base, every interval across zero. So the plan leaves the ranking's
`Good photo` feature inert and its `eye_focus` column always missing, without
refitting the prior.

## Not measured

The preview-only fallback (`full_eye_sharp` measured on the preview itself,
for a file with no larger JPEG: a RAF, an HDR PQ CR3, the Sony bodies whose
largest JPEG is the preview) was never measured. Every number here is
`full_eye_sharp` on an ILCE-7M5 full-size JPEG. Step 2 records a
preview-against-full-size comparison on these folders.
