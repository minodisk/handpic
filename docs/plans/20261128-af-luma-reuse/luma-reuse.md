# The luma-reuse score of files with a trusted AF point

Measured 2026-11-28 for Step 1 of [plan.md](plan.md). Decision: **adopted**
(`FACES_VERSION` 17).

## Conditions

- Host: Windows 11 Home 10.0.26200, Intel Core i7-13700, 32 GB, shared with
  other sessions.
- Build: `cargo build --release -p handpic-cli` (rustc 1.97.1) on `cd802470`
  plus the measurement subcommand of [lumascore.patch](lumascore.patch), one
  thread, one process, every file read once untimed first (warm page cache).
- Folder: `D:\photos\2026\2026-09-19`, read only. 50 bursts qualified there,
  above the plan's ~30, so `2026-09-13-b` was not measured.
- Files: 282 Sony ARW, all 1616x1080 previews. With a trusted AF point
  (`sharpness::trusted_focus`): 282. Face-tracked (`sharpness::eye_af_frame`
  gives a frame, `eye_af=1`): 230; other AF frames (`eye_af=0`): 52.
- Raw table: [lumascore.tsv](lumascore.tsv) (the first pass, before the core
  change).

## The two scores

- `sharpness`, the current score: `sharpness::score_preview(&jpeg,
  Some(focus), frame, &[])`, the frame being `eye_af_frame`'s, as
  `scan::score` calls it. Its time is `gray_ms`, the grayscale decode plus
  the window, the part pass 2 drops.
- `sharp_luma`, the luma-reuse score: `sharpness::score_gray` on
  `candidate::luma` of `decode::decode_rgb` of the same preview, same focus
  and frame. Its time is `luma_ms`, the full-frame luma plus the window; the
  RGB decode is the cue's and was not timed.

Spot check against pass 2: `scan::extract_analysis(..).sharpness` was taken
on the first 6 AF files (`_DSC1796`, `_DSC1797`, `_DSC1802`, `_DSC1806`,
`_DSC1807`, `_DSC1813`). Before the core change it equalled `sharpness` on
all 6 (bit for bit); after it, it equalled `sharp_luma` on all 6. The second
run's `sharpness` and `sharp_luma` columns were identical to the first's on
all 282 files.

## Saving per file

Over the 282 trusted-AF files (ms):

| | median | mean | p10 | p90 |
|---|---|---|---|---|
| `gray_ms` (decode + window) | 6.426 | 8.036 | 5.710 | 10.586 |
| `luma_ms` (luma + window) | 3.104 | 3.961 | 2.774 | 4.957 |
| saving (`gray_ms - luma_ms`) | 3.217 | 4.075 | 1.558 | 7.273 |

A second single pass (after the core change, same binary otherwise) gave a
median saving of 3.008 ms (mean 4.381, p10 0.694, p90 10.466). The
full-frame luma eats about half of the grayscale decode it replaces; the
window reads only its own pixels, so a window-only luma would give the same
values for most of the decode's time back (recorded as a deferred item in
[learnings.md](learnings.md), not done here).

## Within-burst ranking (`burst_spearman.py one`)

| Rows | Bursts (frames) | Spearman median | p10 | min | Same top frame |
|---|---|---|---|---|---|
| `af=af` | 50 (211) | 1.000 | 1.000 | 1.000 | 50/50 (100%) |
| `af=af eye_af=1` | 36 (148) | 1.000 | 1.000 | 1.000 | 36/36 (100%) |
| `af=af eye_af=0` | 2 (8) | 1.000 | 1.000 | 1.000 | 2/2 (100%) |

No burst was skipped as flat. Every burst is at 1.000, so the lowest ones
listed by the script are simply the first in order: `_DSC1824..1828`,
`_DSC1830..1834`, `_DSC1835..1843`, `_DSC1845..1850`, `_DSC1852..1857`
(over the AF frames), and `_DSC2850..2859`, `_DSC3046..3051` (the two
non-face-tracked bursts).

Bursts whose top frame differs: none, so there is no near-tie to list.

## The score difference

Over all 282 frames, `|sharp_luma - sharpness| / sharpness` has a median of
1.416% and a maximum of 22.366%; the luma score is higher on 281 of 282
frames (the exception `_DSC2392`, 355.25 -> 352.84). In absolute terms the
luma score sits a median 3.04 above the current one (from -2.41 to +6.34),
so the relative difference is large only on flat windows: the 13 frames more
than 5% off all score under about 61 (the largest, `_DSC3051`, 13.18 ->
16.13; `_DSC2864`, 12.35 -> 14.83; `_DSC2001`, 15.32 -> 18.14). That offset
fits the truncating integer luma adding a little quantization noise to the
Laplacian. It did not change any order within a burst here, but it does
mean a stored score is not comparable across the `FACES_VERSION` 16 / 17
boundary, which the bump handles by re-running pass 2.

## Decision

Rule (decided by the user before the plan): adopt only if the within-burst
Spearman is about as high as the precedent no-AF variant's was when adopted
and the top-ranked frame of each burst is almost unchanged; unclear means
not adopted. The bar it was checked against (plan Trade-offs 1): median
>= 0.99, no burst < 0.8, the same top frame in >= 95% of bursts, and every
differing top a near-tie (within ~1%).

Result: median 1.000 (>= 0.99), minimum 1.000 (no burst < 0.8), same top
frame 100% (>= 95%), no differing top. Every condition holds, in both the
face-tracked and the other AF frames, so the variant is **adopted**:
`scan::cue_and_gate`'s AF branch scores on the luma of `af_cue`'s RGB with
the trusted AF point and `eye_af_frame`'s frame, and `FACES_VERSION` is 17.

Limits:

- Sony ARW only (plan Trade-offs 2, decided (a)). The change covers every
  format with a trusted AF point (RAF, CR3, NEF, ORF, DNG), whose rankings
  were not checked; the luma-vs-Y difference measured earlier was largest on
  a GX680 RAF (1.1%).
- The saving, a median 3.0-3.2 ms per 1616x1080 ARW preview, is under the
  precedent's "at least about 5 ms"; this plan's rule has no saving
  threshold (Trade-offs 3), so it follows the ranking alone. The bump re-runs
  pass 2 once on every indexed folder at its next open.
- Two passes over one folder on a shared host; the times are medians, not a
  controlled benchmark.

## Commands

```sh
cargo build --release -p handpic-cli
./target/release/handpic-cli.exe lumascore 6 'D:\photos\2026\2026-09-19' > t.tsv
mise x -- python burst_spearman.py one t.tsv sharpness sharp_luma af=af
mise x -- python burst_spearman.py one t.tsv sharpness sharp_luma af=af eye_af=1
mise x -- python burst_spearman.py one t.tsv sharpness sharp_luma af=af eye_af=0
mise x -- python luma_stats.py t.tsv
# after the core change, the same build and run again into t2.tsv:
# `stored` (extract_analysis) now equals `sharp_luma` on the 6 checked files
```

`lumascore <checks> <dir|file>...` is the subcommand of
[lumascore.patch](lumascore.patch) (applied to `crates/cli/src/main.rs`; not
in the merged tree). [burst_spearman.py](burst_spearman.py) is an unchanged
copy of the precedent's; [luma_stats.py](luma_stats.py) gives the timing,
difference and spot-check summaries.
