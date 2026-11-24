# Step 3: scoring a faced no-AF-point file from the cue's RGB decode

Measured on 2026-11-24, Windows 11 (i7-13700), release `handpic-cli`, one
thread, files on the local `D:` drive, warm page cache (each folder scanned
once with the old binary before timing; that first pass of `2026-09-05`
took 423 s, the timed ones 119-140 s).

- Old: `main` at `cd7b2762` (`FACES_VERSION` 12), the exe copied aside.
- New: the same tree with the Step 3 change (`sharpness::score_gray`, the
  scan scoring a faced no-AF file on `candidate::luma` of the cue's
  full-size RGB decode).
- Runs alternated: new, old, new, old per folder
  (`handpic-cli features <dir> 1`); per file, the median `analysis_ms` of
  the two runs of each build.

"Faced no-AF" is `af` = `noaf` and a cue face (`cue_side` not `-`): exactly
the files where the cue makes its full-size RGB decode and the score takes
the eye window (a judged face exists whenever a chosen face does).

## Time (`analysis_ms`, per file)

| Folder | Files | Faced no-AF | Old median | New median | Saving median / mean / p10 / p90 |
|---|---|---|---|---|---|
| `D:\photos\2026\2026-02-01` | 146 | 72 | 286.6 ms | 245.2 ms | 13.4 / 24.7 / -57.7 / 121.2 ms |
| `D:\photos\2026\2026-09-05` | 411 | 204 | 226.5 ms | 211.3 ms | 16.2 / 17.7 / -31.2 / 64.9 ms |

The other files (no face, or the folder's one AF file) moved by -3.8 and
+1.8 ms median, the run-to-run noise. `analysis_ms` includes the full-JPEG
read and decode of `full_eye_sharp`, which dominates the spread; the saving
matches one grayscale decode of a 2112x1408 preview (about 14 ms, see
"DCT-scaled decode for the whole-image search" in
`docs/humans/performance.md`). Folder totals (`... total` line): 02-01
new 48.1 / 50.5 s, old 44.9 / 63.0 s; 09-05 new 118.5 / 135.3 s, old
130.2 / 140.0 s (the machine is shared, so the totals are noisy).

## Score

- Every file that is not faced no-AF (74 + 207) has a bit-for-bit
  identical `sharpness` (as printed, 4 decimals); only the faced no-AF path
  changed in code.
- On the faced no-AF files of both folders the score did not change at all
  either. Both folders are Leica M11-P DNGs, and on their previews
  `candidate::luma` of mozjpeg's RGB output equals mozjpeg's grayscale (Y)
  output on every pixel (checked with a throwaway program on one file of
  each Leica DNG folder: 2026-02-01, 2026-08-29-l, 2026-09-05, 2026-09-23,
  2026-09-23-l, 2026-10-31; 0 of 2,973,696 pixels differ).
- On other makes the two lumas differ on about half the pixels, and the
  tile score of the whole preview moved by 0.1-1.1% (luma vs Y): the first
  α7 V ARW of `2026-09-19` and of `2026-09-13-a` 414.60 vs 410.17 and
  814.38 vs 810.32; from `D:\photos\samples`, an EOS 250D CR3 1081.24 vs
  1079.03, a Nikon 1 AW1 NEF 871.49 vs 870.37, a GX680 RAF 14920.97 vs
  15091.29 and an E-1 ORF 1788.22 vs 1787.21. Those formats reach this
  path only without a trusted AF point and with a confident face; no real
  burst of such frames exists on this host, so their ranking was not
  checked.

## Ranking within bursts

`burst_spearman.py two old.tsv new.tsv sharpness af=noaf 'cue_side!=-'`
over the two folders' first alternated runs, joined:

- bursts used: 26 (101 frames), all from `2026-09-05` (`2026-02-01` has no
  burst of three or more faced frames within 1000 ms)
- Spearman median 1.000, p10 1.000, minimum 1.000
- same top frame: 26 / 26 (100%)

## Decision

Adopted under Trade-offs 3: median Spearman 1.000 >= 0.95, no burst under
0.8, median saving 13-16 ms >= 5 ms per faced no-AF file. `FACES_VERSION`
goes 12 -> 13, because on other makes the score's pixels do change.
