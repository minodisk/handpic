# Results: keypress to the screen before and after the fit-view decode scale

Measured 2026-11-21, 16:30-17:10 local (the app logs stamp it in UTC,
07:30:08 to 08:10:32), both builds in one session, alternating, against the
external ruler of
[the Tauri vs GPUI A/B](../_archived/20261120-tauri-gpui-latency-ab/results.md).

**Verdict: the target is met.** On RAF, B's `screen` minus
`keypressToPixels` p50 is 15.09 ms, at the ARW level of the same session
(14.36 ms for A, 14.92 ms for B), down from A's 47.02 ms. A page turn onto a
4416x2944 RAF preview now reaches the screen 47.66 ms sooner at p50 and
61.61 ms sooner at p95. B's RAF `page` lines show `decoded=1656x1104` (3/8)
for the 51 landscape pages and `decoded=1104x736` (2/8, upright 736x1104) for
the 9 portrait ones in every run.

Median of the three runs' p50 / p95, in ms:

| Folder | Build | `screen` | `keypressToPixels` (in-app) | `screen` minus `keypressToPixels` |
|---|---|---|---|---|
| ARW 1616x1080 | A (before) | **36.63 / 49.92** | 22.27 / 32.93 | 14.36 / 16.99 |
| ARW 1616x1080 | B (after) | **37.44 / 47.56** | 22.52 / 32.63 | 14.92 / 14.93 |
| RAF 4416x2944 | A (before) | **112.60 / 137.87** | 65.58 / 78.41 | 47.02 / 59.46 |
| RAF 4416x2944 | B (after) | **64.94 / 76.26** | 49.85 / 61.06 | 15.09 / 15.20 |

- RAF: B is 47.66 ms faster on screen at p50 (61.61 at p95). Of that, 15.73
  ms is in-app (`decode` 46.36 to 33.71, `paint` 3.35 to 0.69, `orient`'s p95
  8.72 to 1.15) and the remaining 31.93 ms is after the `present` stamp: the
  texture upload the plan's investigation found.
- ARW: no change beyond the run-to-run spread (`screen` p50 +0.81 ms; A's
  runs span 36.61-38.33 and B's 35.96-38.10). `paint` halves (0.71 to 0.38)
  at the 6/8 decode (`decoded=1212x810`), `decode` does not move (8.37
  against 8.50, no SIMD IDCT at 6/8, as Step 2 found).
- The last column is a difference of the medians shown, not a per-press
  difference.

The raw output is beside this file: [raw/summary.md](raw/summary.md) (the
`summarize-gpui.py` output every table below comes from),
[raw/runs.txt](raw/runs.txt) (every attempt, its load samples and why it was
discarded), and the driver and summarizer used,
[raw/run-ab-gpui.ps1](raw/run-ab-gpui.ps1) and
[raw/summarize-gpui.py](raw/summarize-gpui.py). The per-press CSVs and the
app logs were kept in the session's scratch folder only.

## What each number measures

As in [the Tauri vs GPUI A/B](../_archived/20261120-tauri-gpui-latency-ab/results.md#what-each-number-measures):

- `screen`: from the `SendInput` of the Right key to the DWM present of the
  first frame from which the patch at the window's client center holds the
  value it settles on before the next press.
- `screenFirstChange`: from the same keypress to the first presented frame in
  which that patch differs from its value at the press.
- `keypressToPixels`: the `page` line's, from the key event to the `present`
  stamp; its stages are `wait`, `read`, `decode`, `orient`, `hop`,
  `frameWait`, `paint` and `present`.

## Measurement conditions

- A = `6cf88896` (`main` before Step 2, the parent of `d3794643`), built in a
  temporary worktree with its own target folder. B = `9f650e9f` (`main` after
  Step 2 plus a release-please change that touches no code). Both
  `cargo build --release --locked` of `crates/app` with Rust 1.98.1.
- Each run: `handpic.exe <folder> --no-prefetch --index <temp> --settings
  <temp> --log <file>`, the `page` lines read from its redirected stdout.
  Each build had its own index and settings file in the output folder, filled
  by its warm run and reused by its measured runs.
- Ruler: `tools/latprobe` at `1e97309e` on the local-only branch
  `measure/tauri-latency-ab`, exported with `git archive` and built with
  `cargo build --release`; unchanged. The window is placed at (100,100),
  1600x1000; 60 presses per run, one every 1003 ms, 8 s of settle after
  launch.
- Driver: that branch's `run-ab.ps1` with its B side pointed at the second
  GPUI build ([raw/run-ab-gpui.ps1](raw/run-ab-gpui.ps1)): the Tauri launch,
  its settings seed and its log copy removed, a per-build index and settings
  file, and a 1 px mouse move and back before each run (below). The idle gate,
  the per-folder order (one warm run of each build, then A B A B A B) and the
  redo rule (up to 4 tries per slot) are the branch's.
- Summary: that branch's `summarize.py` reading both builds' `page` lines
  from stdout and labelling them A / B
  ([raw/summarize-gpui.py](raw/summarize-gpui.py)). Percentiles: sorted index
  `n / 2` and `n * 0.95`; the median row is the median of the three runs'
  p50s and, separately, of their p95s. The new `decoded=WxH` field does not
  match its `(\w+)=([\d.]+)ms` pattern.
- Folders: `D:\Photos\tests\20261108-gpui-compare\ARW` (1616x1080 previews)
  and `\RAF` (4416x2944), 20 files each; 60 presses wrap each folder three
  times.
- Machine: Windows 11 Home 10.0.26200; i7-13700 (16C/24T); 32 GB RAM; RTX
  4060 Ti, driver 32.0.16.1742; display 5120x1440 at 59 Hz; `D:` a SATA SSD.
  The same machine as the archived A/B.

### Attempts

`runs.txt` holds 32 lines: 4 warm runs and 28 measured attempts. 12 are used,
one per slot; 16 were discarded, for a load sample over 15 % after the first
or a `cargo` / `rustc` process of another session's build during the run: 4
on ARW (A1, B1, A2 and B3 one each) and 12 on RAF (B1 one, A2 two, B2 five,
A3 one, B3 three).

`RAF-B2` failed its 4 tries while other sessions built, so the driver moved on
with the slot empty. It was redone after `RAF-B3` with the same driver
(tries 5 and 6), the only slot out of the A B A B A B order.

| Run | Sample before | Samples during | Used |
|---|---|---|---|
| ARW-warm-A | 11 | 12, 17, 14, 53, 11, 10, 4, 3, 0, 7, 2, 0, 8, 5, 2, 7, 2 | warm (17, 53 %) |
| ARW-warm-B | 5 | 38, 0, 11, 0, 10, 55, 76, 30, 28, 15, 14, 51, 6, 3, 2, 36, 2 | warm (28-76 %; cargo / rustc) |
| ARW-A1-try1 | 9 | 1, 3, 3, 2, 2, 1, 0, 11, 5, 1, 4, 2, 50, 28, 2, 12, 5 | no (28, 50 %; cargo / rustc) |
| ARW-A1-try2 | 5 | 7, 15, 8, 3, 10, 10, 10, 14, 1, 10, 4, 13, 7, 14, 4, 7, 11 | yes (A1) |
| ARW-B1-try1 | 0 | 3, 1, 0, 1, 2, 3, 4, 3, 5, 10, 31, 0, 3, 3, 3, 3, 0 | no (31 %) |
| ARW-B1-try2 | 0 | 2, 11, 2, 7, 0, 5, 3, 0, 4, 7, 5, 4, 0, 1, 0, 6, 0 | yes (B1) |
| ARW-A2-try1 | 11 | 8, 3, 1, 4, 0, 33, 2, 5, 6, 4, 2, 5, 0, 4, 8, 10, 0 | no (33 %) |
| ARW-A2-try2 | 3 | 11, 5, 3, 10, 3, 7, 1, 3, 10, 3, 9, 9, 5, 4, 14, 3, 2 | yes (A2) |
| ARW-B2-try1 | 3 | 40, 15, 1, 9, 2, 5, 11, 6, 9, 6, 10, 2, 10, 9, 13, 7, 6 | yes (B2) |
| ARW-A3-try1 | 9 | 20, 0, 0, 7, 1, 6, 3, 0, 1, 5, 2, 1, 2, 2, 2, 0, 1 | yes (A3) |
| ARW-B3-try1 | 1 | 8, 3, 2, 6, 1, 6, 0, 0, 2, 0, 15, 14, 18, 0, 3, 11, 2 | no (18 %) |
| ARW-B3-try2 | 5 | 7, 5, 1, 5, 0, 4, 3, 3, 7, 3, 5, 2, 4, 3, 1, 1, 0 | yes (B3) |
| RAF-warm-A | 6 | 12, 2, 4, 9, 15, 15, 3, 3, 5, 5, 9, 3, 5, 3, 8, 2, 2 | warm |
| RAF-warm-B | 3 | 44, 11, 3, 6, 1, 3, 8, 0, 1, 9, 9, 5, 4, 5, 12, 2, 19 | warm (19 %) |
| RAF-A1-try1 | 15 | 25, 6, 4, 0, 0, 1, 1, 0, 3, 0, 4, 2, 5, 1, 2, 7, 4 | yes (A1) |
| RAF-B1-try1 | 7 | 15, 0, 0, 7, 0, 3, 0, 6, 4, 12, 0, 37, 0, 0, 1, 1, 2 | no (37 %) |
| RAF-B1-try2 | 2 | 2, 0, 0, 8, 3, 1, 2, 2, 4, 3, 2, 2, 2, 3, 6, 5, 0 | yes (B1) |
| RAF-A2-try1 | 11 | 5, 2, 7, 5, 2, 11, 38, 5, 0, 5, 11, 13, 6, 0, 0, 7, 8 | no (38 %) |
| RAF-A2-try2 | 5 | 15, 3, 2, 17, 33, 34, 18, 31, 51, 25, 16, 8, 8, 20, 22, 17, 6 | no (16-51 %; cargo / rustc) |
| RAF-A2-try3 | 4 | 0, 3, 1, 0, 2, 1, 7, 3, 5, 3, 0, 9, 13, 0, 1, 3, 2 | yes (A2) |
| RAF-B2-try1 | 8 | 11, 3, 5, 4, 2, 0, 0, 39, 5, 5, 6, 2, 7, 6, 6, 18, 3, 6 | no (18, 39 %; cargo / rustc) |
| RAF-B2-try2 | 0 | 12, 8, 4, 0, 4, 1, 25, 7, 3, 1, 13, 4, 4, 5, 10, 44, 21 | no (21-44 %; cargo / rustc) |
| RAF-B2-try3 | 9 | 8, 10, 20, 39, 12, 23, 6, 37, 4, 8, 16, 11, 3, 16, 6, 5, 8 | no (16-39 %; cargo / rustc) |
| RAF-B2-try4 | 9 | 5, 11, 9, 18, 7, 12, 23, 31, 27, 13, 62, 16, 3, 0, 0, 0, 2 | no (16-62 %; cargo / rustc) |
| RAF-A3-try1 | 3 | 0, 1, 0, 1, 0, 19, 21, 6, 5, 5, 27, 9, 3, 4, 3, 0, 3 | no (19-27 %; cargo / rustc) |
| RAF-A3-try2 | 1 | 8, 4, 1, 2, 9, 0, 4, 3, 2, 1, 3, 3, 0, 2, 3, 2, 2 | yes (A3) |
| RAF-B3-try1 | 3 | 4, 2, 0, 0, 0, 1, 3, 0, 0, 1, 12, 4, 2, 2, 4, 2, 31 | no (31 %) |
| RAF-B3-try2 | 12 | 14, 4, 6, 1, 11, 1, 3, 7, 3, 7, 5, 10, 16, 19, 18, 4, 2 | no (16-19 %) |
| RAF-B3-try3 | 7 | 9, 6, 14, 0, 13, 7, 11, 15, 1, 25, 11, 28, 7, 8, 0, 4, 3 | no (25, 28 %) |
| RAF-B3-try4 | 1 | 6, 0, 0, 0, 3, 0, 1, 1, 0, 2, 2, 4, 0, 2, 0, 3, 3 | yes (B3) |
| RAF-B2-try5 | 2 | 4, 4, 3, 2, 11, 0, 11, 1, 0, 7, 1, 1, 3, 0, 7, 0, 20 | no (20 %) |
| RAF-B2-try6 | 1 | 1, 2, 2, 4, 2, 0, 3, 1, 10, 1, 0, 3, 3, 5, 13, 1, 5 | yes (B2) |

The first sample during a run is taken while the app starts and is not gated.

## Tables

Cells are `p50 / p95` in ms, n = 60 presses on screen and 60 `page` lines in
stdout for every run. Columns are in pipeline order; the values are
`summary.md`'s. `wait` is 0.00 / 0.00 in every run and `screenFirstChange`
equals `screen` in every cell (below), so both are left out.

### ARW, A (before)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 2.38 / 9.00 | 8.01 / 12.28 | 0.00 / 1.01 | 0.02 / 0.39 | 11.56 / 17.70 | 0.71 / 1.03 | 0.47 / 0.83 | 25.01 / 34.34 | 38.33 / 64.15 |
| 2 | 1.79 / 3.00 | 8.38 / 11.98 | 0.00 / 1.14 | 0.02 / 1.51 | 10.64 / 17.33 | 0.71 / 0.84 | 0.45 / 0.74 | 22.27 / 32.20 | 36.63 / 49.92 |
| 3 | 2.23 / 3.24 | 8.37 / 13.03 | 0.00 / 1.02 | 0.02 / 0.38 | 9.88 / 17.65 | 0.69 / 0.81 | 0.46 / 0.86 | 22.14 / 32.93 | 36.61 / 48.11 |
| **median of 3** | **2.23 / 3.24** | **8.37 / 12.28** | **0.00 / 1.02** | **0.02 / 0.39** | **10.64 / 17.65** | **0.71 / 0.84** | **0.46 / 0.83** | **22.27 / 32.93** | **36.63 / 49.92** |

### ARW, B (after)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 1.48 / 3.11 | 8.50 / 11.49 | 0.00 / 0.71 | 0.02 / 0.75 | 9.39 / 17.05 | 0.38 / 0.66 | 0.46 / 0.89 | 20.78 / 30.50 | 35.96 / 45.69 |
| 2 | 1.76 / 3.15 | 8.96 / 12.45 | 0.00 / 0.87 | 0.02 / 1.38 | 10.35 / 17.47 | 0.37 / 0.52 | 0.43 / 0.74 | 23.08 / 33.08 | 38.10 / 49.08 |
| 3 | 1.52 / 3.31 | 7.53 / 11.97 | 0.00 / 0.76 | 0.02 / 1.82 | 12.97 / 18.16 | 0.40 / 0.63 | 0.45 / 0.63 | 22.52 / 32.63 | 37.44 / 47.56 |
| **median of 3** | **1.52 / 3.15** | **8.50 / 11.97** | **0.00 / 0.76** | **0.02 / 1.38** | **10.35 / 17.47** | **0.38 / 0.63** | **0.45 / 0.74** | **22.52 / 32.63** | **37.44 / 47.56** |

### RAF, A (before)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 4.39 / 6.66 | 46.45 / 54.62 | 0.00 / 8.97 | 0.39 / 0.66 | 8.62 / 17.10 | 3.31 / 3.89 | 0.42 / 0.60 | 65.69 / 78.41 | 113.71 / 132.74 |
| 2 | 2.57 / 6.20 | 46.36 / 52.91 | 0.00 / 8.71 | 0.06 / 0.55 | 9.21 / 17.64 | 3.35 / 3.91 | 0.42 / 0.63 | 64.10 / 78.72 | 112.30 / 137.87 |
| 3 | 3.31 / 6.99 | 45.99 / 52.98 | 0.00 / 8.72 | 0.42 / 0.67 | 10.65 / 17.73 | 3.35 / 3.80 | 0.43 / 0.57 | 65.58 / 78.09 | 112.60 / 138.87 |
| **median of 3** | **3.31 / 6.66** | **46.36 / 52.98** | **0.00 / 8.72** | **0.39 / 0.66** | **9.21 / 17.64** | **3.35 / 3.89** | **0.42 / 0.60** | **65.58 / 78.41** | **112.60 / 137.87** |

### RAF, B (after)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 4.41 / 6.98 | 34.32 / 42.92 | 0.00 / 1.21 | 0.02 / 0.06 | 9.21 / 17.34 | 0.68 / 0.85 | 0.40 / 0.46 | 49.85 / 61.06 | 64.94 / 76.26 |
| 2 | 3.79 / 6.84 | 33.71 / 42.80 | 0.00 / 1.14 | 0.02 / 0.07 | 8.97 / 17.70 | 0.69 / 0.89 | 0.40 / 0.49 | 50.20 / 62.15 | 65.51 / 77.88 |
| 3 | 2.87 / 6.52 | 33.58 / 41.43 | 0.00 / 1.15 | 0.02 / 0.04 | 8.68 / 16.70 | 0.69 / 0.82 | 0.37 / 0.53 | 47.42 / 58.68 | 62.86 / 73.71 |
| **median of 3** | **3.79 / 6.84** | **33.71 / 42.80** | **0.00 / 1.15** | **0.02 / 0.06** | **8.97 / 17.34** | **0.69 / 0.85** | **0.40 / 0.49** | **49.85 / 61.06** | **64.94 / 76.26** |

Runs 1, 2 and 3 are the slots A1 / A2 / A3 and B1 / B2 / B3.

## Reading

Every difference is between the median cells above.

- **The gap after the `present` stamp is gone on RAF.** A's RAF `screen`
  was 47.02 ms after its `keypressToPixels`, as the archived run's 47.95;
  B's is 15.09, within 0.2 ms of B's ARW 14.92 and 0.7 ms of A's ARW 14.36.
  The remaining ~15 ms is common to both formats and both builds: the
  frame's way through the DWM after `present`.
- **The upload, not the decode, was most of the RAF gain on screen.** Of
  the 47.66 ms RAF `screen` drop, 15.73 ms shows in the in-app number (the
  smaller decode and paint); 31.93 ms is after the `present` stamp.
- The CSVs' `frames` column (presented frames between a press and the next)
  is 2-4 on RAF and 1-3 on ARW for both builds. It counts the whole second
  between presses, so it does not show the missed compositions; the `screen`
  drop does.
- **ARW holds still.** The 6/8 decode halves `paint`; `screen` and
  `keypressToPixels` move less than one run's spread. ARW's gap is 14.36
  against 14.92 ms, inside the noise.
- **Every press settled at its first change** in all 12 used runs (checked
  per press in the CSVs), and no press saw Desktop Duplication's
  `AccumulatedFrames` above 1.

## Caveats

- Only a same-session A/B is a valid comparison on this machine; these
  numbers are not to be compared with the archived run's beyond the
  observation that A reproduces its RAF gap (47.02 against 47.95 ms).
- `LastPresentTime` is the DWM present, not scanout: display and panel
  latency are in no number.
- At 59 Hz a frame is about 16.9 ms, and `screen` is quantized to the
  refresh.
- Differences of percentiles are not per-press differences.
- Prefetch is off in both builds: this is the uncached page turn. With
  prefetch on, B's `page` lines are mostly `source=prefetch` (Step 2's
  measurement).
- `RAF-B2` ran out of order, after `RAF-B3`, because other sessions' builds
  spoiled its four tries in place.
- The display had gone to sleep before the first attempt of the session
  (DXGI Desktop Duplication then delivers no frame and the probe exits 2
  before pressing); that attempt was stopped and its output deleted, and
  the driver now moves the mouse 1 px and back before each run. Every used
  run's `frameWait` p50 is 8.6-13.0 ms, the normal range.

## Reproduction

1. A: `cargo build --release --locked` in `crates/app` at `6cf88896`; B: the
   same at `9f650e9f` (or any later `main`).
2. The ruler: `git archive 1e97309e tools/latprobe`, then
   `cargo build --release` in it.
3. The run (PowerShell; `.ps1` files need `-ExecutionPolicy Bypass` here):

   ```powershell
   powershell -ExecutionPolicy Bypass -File raw\run-ab-gpui.ps1 `
     -Gpui <A handpic.exe> -GpuiB <B handpic.exe> -Probe <latprobe.exe> -Out <out>
   ```

4. The tables: `python raw\summarize-gpui.py <out>`.
