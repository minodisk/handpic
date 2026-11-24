# A JPEG folder's page time in the app (Step 4)

Measured 2026-11-23 on Windows 11 (i7-13700, RTX 4060 Ti, 5120x1440 at
59 Hz), release build of `crates/app` at `5a45d78a`, one session. Each run
is `mise run app:release -- <folder> --auto N --index <temp> --settings
<temp> | cat` (the trial build B below ran its exe directly with the same
arguments), one index per folder kept across that folder's runs, so the
folder scan's passes were still running during the first runs of the two
large folders. Per folder: one warm run, then prefetch on and
`--no-prefetch` alternated, three runs each. The first `page` line of a run
is dropped. Under `--auto` the window is the default one, so the canvas, and
the `n/8` `fitscale::scale_for` picks, are the default window's: the JPEGs
decoded at 1/8 or 2/8 (sizes below). Percentiles are the
nearest rank of each run; the table gives the median of the three runs' p50
/ p95, in ms.

All JPEGs are baseline (`exiftool -EncodingProcess`: `Baseline DCT, Huffman
coding` on all 782 files of the 500-file folder and the Output folder, and on
all 10 files of the 10-file folder, checked 2026-11-23).

Preview sizes, from the `px=` (source) and `decoded=` fields of the `page`
lines of the warm, on and off runs (the first line of each run dropped; the
`n/8` is `decoded` over `px`, every line matching mozjpeg's
`ceil(native * n / 8)` exactly):

| Folder | Lines | `px=` (source) | `decoded=` | `n/8` |
|---|---|---|---|---|
| ARW ruler | 126 | 1616x1080 on all | 1212x810 on 119, 808x540 on 7 (the one `orientation=6` file) | 6/8 on 119, 4/8 on 7 |
| `2026-09-28-gui-check-jpeg` | 56 | 9 sizes, 36.1-50.1 MP; 7910x6328 most common (12) | 9 sizes, width 973-2296, height 499-1500; 989x791 most common (12) | 1/8 on 35 (37.1-50.1 MP), 2/8 on 21 (36.1-45.0 MP) |
| `2026-09-28-gui-check-jpeg-500` | 203 | 88 sizes, 23.4-60.3 MP (median 50.1); 7910x6328 most common (102) | 85 sizes, width 668-2308, height 499-1529; 989x791 most common (102) | 1/8 on 147 (36.1-60.3 MP), 2/8 on 56 (23.4-52.0 MP) |
| `2026-09-19\Output` | 203 | 68 sizes, 14.7-32.7 MP (median 27.1); 4673x7008 most common (20) | 64 sizes, width 506-1460, height 767-1525; 585x876 most common (20) | 1/8 on 123 (all portrait, 25.1-32.7 MP), 2/8 on 80 (54 portrait, 26 landscape; 14.7-27.3 MP) |

No page decoded at 3/8 or more on a JPEG. The ARW lines put the default
canvas's fit height between 676 and 810 px (6/8 of 1080 is 810, 5/8 would be
675): the landscape 1/8 decodes are at most 791 px high, the 2/8 ones at least
978. The A / B trial's runs below logged the same sizes (ARW 6/8 on 104 lines,
4/8 on 4; the 10-file folder 1/8 on 28, 2/8 on 20).

| Folder | Files | Prefetch | n per run | keypressToPixels | read | decode | orient |
|---|---|---|---|---|---|---|---|
| ARW ruler (`20261108-gpui-compare\ARW`), 1616x1080, 6/8 | 20 | on | 18 | 12.31 / 19.45 | 0.00 | 0.00 | 0.00 |
| ARW ruler | 20 | off | 18 | 19.55 / 25.84 | 1.00 | 6.72 | 0.00 |
| `2026-09-28-gui-check-jpeg`, Leica M11-P DxO exports, 36-50 MP, 10-28 MB | 10 | on | 8 | 9.68 / 108.71 | 0.00 | 0.00 | 0.00 |
| `2026-09-28-gui-check-jpeg` | 10 | off | 8 | 114.80 / 176.23 | 3.48 | 105.12 | 0.00 |
| `2026-09-28-gui-check-jpeg-500`, Leica M11-P, 23-60 MP (median 50) | 500 | on | 29 | 9.02 / 19.65 | 0.00 | 0.00 | 0.00 |
| `2026-09-28-gui-check-jpeg-500` | 500 | off | 29 | 230.48 / 309.20 | 50.87 | 160.17 | 0.00 |
| `D:\photos\2026\2026-09-19\Output`, Sony ILCE-7M5 exports, 13-33 MP (median 26) | 282 | on | 29 | 12.57 / 19.20 | 0.00 | 0.00 | 0.00 |
| `2026-09-19\Output` | 282 | off | 29 | 50.21 / 62.48 | 9.29 | 29.61 | 0.00 |

Per-run p50 `keypressToPixels`: ARW on 12.26 / 12.31 / 13.59, off 19.88 /
19.55 / 17.77; jpeg (10) on 7.50 / 15.99 / 9.68, off 125.55 / 114.30 /
114.80; jpeg-500 on 9.02 / 8.81 / 12.82, off 148.08 / 266.93 / 230.48;
Output on 14.84 / 12.21 / 12.57, off 102.06 / 49.92 / 50.21.

- With the prefetch ring on (the default), paging one file a second is served
  from the ring (`source=prefetch` on every page but one or two a run) and the
  p50 is 9-13 ms on every JPEG folder, at about the ARW ruler's 12.31 ms
  (the ARW per-run p50s span 12.26 to 13.59; only the Leica folders are
  under it, the Sony Output folder's 12.57 ms is just above). The
  p95 of 108.71 ms in the 10-file folder is one page a run that missed the
  ring and was read (n=8, so p95 is the largest sample).
- Without the ring, every page pays the decode. On the 50 MP Leica exports it
  is about 100 ms at 1/8 scale (the entropy decode of a 10-28 MB file does
  not shrink with the output scale), so `keypressToPixels` p50 is about
  115 ms, above the 65 ms RAF figure. On the 26 MP Sony exports it is about
  30 ms and the p50 is 50 ms, under it.
- The jpeg-500 `--no-prefetch` runs 2 and 3, and Output run 1, ran while
  the folder scan of the new index was still reading files: `read` rose to
  50-75 ms and `decode` with it. Run 1 of jpeg-500 (148.08 ms, `read`
  6.29 ms) is the uncontended figure.

## Trial: a faster decoder setting (Trade-offs 5 (b))

Build B added `d.do_fancy_upsampling(false)` and
`d.dct_method(mozjpeg::DctMethod::IntegerFast)` after `d.scale(..)` in
`load` (both exposed by `mozjpeg` 0.10.13). A and B alternated, A B A B A B,
`--no-prefetch`, same session:

| Folder | Build | keypressToPixels p50 (runs) | decode p50 (runs) | decode median |
|---|---|---|---|---|
| jpeg (10) | A | 146.13 / 125.33 / 111.88 | 96.62 / 103.94 / 94.58 | 96.62 |
| jpeg (10) | B | 115.09 / 125.18 / 111.36 | 99.87 / 102.19 / 95.85 | 99.87 |
| ARW ruler | A | 49.09 / 19.28 / 19.86 | 6.84 / 7.12 / 7.12 | 7.12 |
| ARW ruler | B | 21.47 / 18.53 / 21.71 | 6.88 / 7.00 / 6.68 | 6.88 |

(ARW A run 1 read cold, `read` 33 ms.) B does not cut `decode`: at 1/8 and
2/8 libjpeg-turbo already uses its reduced-size IDCT, and the time is the
entropy decode, which neither setting touches. Option (b) is out (bar: 20%).
The trial was reverted.
