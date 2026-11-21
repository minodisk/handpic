# Results: keypress to the screen before and after the immediate paint

Measured 2026-11-22, 03:42-04:31 local (the app logs stamp it in UTC,
2026-11-21 18:42:44 to 19:31:35), both builds in one session, alternating,
against the external ruler of
[the Tauri vs GPUI A/B](../_archived/20261120-tauri-gpui-latency-ab/results.md).

**Verdict: the keyed page reaches the screen sooner, by one whole refresh on
about half the presses.** B's `frameWait` dropped from 9-11 ms to 1.5-1.8 ms
at p50 in every run, on both folders. `screen` moved by -8.41 / -7.52 ms
(p50 / p95) on ARW and by -4.85 / -3.46 ms on RAF. Per press, no A press
reached the screen at the first DWM composition after its page was pended
(0 / 180 on each folder); with B, 84 / 180 ARW presses (47 %) and 61 / 180
RAF presses (34 %) did, a whole refresh earlier, and the rest landed where A's
do.

Median of the three runs' p50 / p95, in ms:

| Folder | Build | `screen` | `keypressToPixels` (in-app) | `frameWait` | presses at the first composition |
|---|---|---|---|---|---|
| ARW 1616x1080 | A (before) | **33.51 / 41.13** | 18.73 / 26.73 | 10.50 / 16.85 | 0 / 180 |
| ARW 1616x1080 | B (after) | **25.10 / 33.61** | 10.15 / 12.16 | 1.74 / 2.11 | 84 / 180 (47 %) |
| RAF 4416x2944 | A (before) | **60.99 / 73.14** | 46.07 / 58.39 | 9.22 / 16.67 | 0 / 180 |
| RAF 4416x2944 | B (after) | **56.14 / 69.68** | 36.79 / 46.19 | 1.55 / 1.89 | 61 / 180 (34 %) |

- `frameWait` is about zero, as Step 2 expected: what remains, 1.5-1.8 ms at
  p50 and at most 2.4 ms at p95, is building the frame before the preview
  canvas's paint closure stamps `paint_started`, not a wait.
- The in-app gain (`keypressToPixels` p50 -8.58 ms on ARW, -9.28 ms on RAF)
  reaches the screen only through the presses that now make the earlier
  composition. The mean gain that predicts, share times one refresh, is
  7.9 ms on ARW and 5.7 ms on RAF, against the measured `screen` p50 moves
  of 8.41 and 4.85 ms.
- With prefetch on (in-app only, below), B's keyed `source=prefetch` pages
  have `frameWait` 1.85 / 2.81 ms (ARW) and 1.73 / 2.62 ms (RAF), against A's
  9.91 / 17.41 and 11.82 / 17.02 ms.

The raw output is beside this file: [raw/summary.md](raw/summary.md) (the
`summarize-gpui.py` output the stage tables come from),
[raw/screengap.txt](raw/screengap.txt) (the `screengap.py` output the
per-press tables come from), [raw/prefetch.txt](raw/prefetch.txt) (the
prefetch-on run's `prefetch.py` output), [raw/runs.txt](raw/runs.txt) and
[raw/runs-prefetch.txt](raw/runs-prefetch.txt) (every attempt, its load
samples and why it was discarded), and the drivers and scripts used,
[raw/run-ab-gpui.ps1](raw/run-ab-gpui.ps1),
[raw/redo-slots.ps1](raw/redo-slots.ps1),
[raw/run-prefetch.ps1](raw/run-prefetch.ps1),
[raw/summarize-gpui.py](raw/summarize-gpui.py),
[raw/screengap.py](raw/screengap.py) and [raw/prefetch.py](raw/prefetch.py).
The per-press CSVs, the redirected stdout and the app logs stay in the
measurement worktree's `out\20261122-present-now` folder (the local-only
branch `measure/tauri-latency-ab`), as with the earlier records.

## What each number measures

As in [the Tauri vs GPUI A/B](../_archived/20261120-tauri-gpui-latency-ab/results.md#what-each-number-measures):

- `screen`: from the `SendInput` of the Right key to the DWM present of the
  first frame from which the patch at the window's client center holds the
  value it settles on before the next press.
- `keypressToPixels`: the `page` line's, from the key event to the `present`
  stamp; its stages are `wait`, `read`, `decode`, `orient`, `hop`,
  `frameWait`, `paint` and `present`. `frameWait` is from the page pended
  (`notified`) to the preview canvas's paint closure.

## Measurement conditions

- A = `c57ec2ff` (`main` before Step 2, the parent of `6809229f`). B =
  `6809229f` (Step 2's merge, #1219). Each built with
  `cargo build --release --locked` of `crates/app` with Rust 1.98.1 in a
  temporary worktree, removed afterwards; the two builds shared one target
  folder, built one after the other, and the binaries were copied out.
- Each ruler run: `handpic.exe <folder> --no-prefetch --index <temp>
  --settings <temp> --log <file>`, the `page` lines read from its redirected
  stdout. Each build had its own index and settings file in the output
  folder, filled by its warm run and reused by its measured runs.
- Ruler: `tools/latprobe` at `1e97309e` on the local-only branch
  `measure/tauri-latency-ab`, the binary already built in that worktree;
  unchanged. The window is placed at (100,100), 1600x1000; 60 presses per
  run, one every 1003 ms, 8 s of settle after launch.
- Driver: the fit-preview-downscale record's `run-ab-gpui.ps1` with one
  addition, `SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED |
  ES_SYSTEM_REQUIRED)` held by the driver for the session, as `docs/agents/app.md`
  advises against a sleeping display
  ([raw/run-ab-gpui.ps1](raw/run-ab-gpui.ps1)). The idle gate, the
  per-folder order (one warm run of each build, then A B A B A B), the 1 px
  mouse move before each run and the redo rule (up to 4 tries per slot) are
  unchanged. [raw/redo-slots.ps1](raw/redo-slots.ps1) is the same driver
  with its loop replaced by named slots from try 5 on.
- Summary: [raw/summarize-gpui.py](raw/summarize-gpui.py), unchanged.
  Percentiles: sorted index `n / 2` and `n * 0.95`; the median row is the
  median of the three runs' p50s and, separately, of their p95s.
- Folders: `D:\Photos\tests\20261108-gpui-compare\ARW` (1616x1080 previews,
  decoded at 1212x810) and `\RAF` (4416x2944, decoded at 1656x1104), 20
  files each; 60 presses wrap each folder three times.
- Prefetch-on run: [raw/run-prefetch.ps1](raw/run-prefetch.ps1), each build
  once per folder with `--auto 59` and prefetch on (keyed pages from the
  app's own timer, one every 1003 ms), reusing the build's index, the same
  idle gate before each run, no ruler, the window at its default size.
- Machine: Windows 11 Home 10.0.26200; i7-13700 (16C/24T); 32 GB RAM; RTX
  4060 Ti, driver 32.0.16.1742; display 5120x1440 at 59 Hz; `D:` a SATA SSD.
  The same machine as the archived A/Bs. Two background processes each held
  about one core through the session (Amazon Photos and another session's
  Python process), which is most of the discarded attempts below.

### Attempts

`runs.txt` holds 36 lines: 4 warm runs and 32 measured attempts. 12 are used,
one per slot; 20 were discarded, each for a load sample over 15 % after the
first (no compiler process was seen in any run): 9 on ARW (A1 three, A2
three, A3 three) and 11 on RAF (A1 one, B1 six, A2 four).

`RAF-B1` and `RAF-A2` failed their 4 tries in place, so the driver moved on
with those slots empty. Both were redone after `RAF-B3` with
`redo-slots.ps1` (B1 tries 5-7, then A2 try 5), the only slots out of the
A B A B A B order.

| Run | Sample before | Samples during | Used |
|---|---|---|---|
| ARW-warm-A | 1 | 13, 2, 3, 1, 0, 1, 0, 0, 0, 1, 0, 4, 5, 0, 0, 2, 3, 9 | warm |
| ARW-warm-B | 0 | 26, 5, 3, 2, 3, 1, 1, 2, 0, 0, 4, 3, 2, 1, 0, 3, 0, 3 | warm |
| ARW-A1-try1 | 5 | 10, 4, 3, 2, 3, 4, 2, 3, 3, 7, 4, 8, 16, 17, 2, 5, 13, 23 | no (16, 17, 23 %) |
| ARW-A1-try2 | 4 | 23, 6, 5, 1, 8, 13, 4, 12, 8, 11, 6, 13, 12, 15, 23, 17, 8, 14 | no (23, 17 %) |
| ARW-A1-try3 | 10 | 20, 12, 11, 11, 13, 19, 22, 20, 5, 8, 17, 12, 20, 5, 2, 0, 1, 2 | no (17-22 %) |
| ARW-A1-try4 | 1 | 5, 0, 0, 2, 1, 2, 3, 5, 2, 1, 2, 1, 2, 4, 5, 2, 6, 7 | yes (A1) |
| ARW-B1-try1 | 4 | 8, 4, 0, 2, 3, 0, 10, 0, 4, 3, 4, 0, 1, 4, 11, 3, 1, 8 | yes (B1) |
| ARW-A2-try1 | 9 | 12, 2, 6, 0, 1, 5, 4, 5, 3, 0, 2, 9, 7, 6, 6, 8, 9, 16 | no (16 %) |
| ARW-A2-try2 | 6 | 23, 6, 15, 11, 1, 16, 2, 11, 3, 10, 9, 29, 14, 24, 2, 6, 6, 26 | no (16-29 %) |
| ARW-A2-try3 | 8 | 30, 11, 12, 17, 17, 22, 16, 24, 15, 18, 11, 3, 2, 0, 2, 5, 0, 0 | no (16-24 %) |
| ARW-A2-try4 | 3 | 13, 4, 7, 1, 2, 0, 0, 3, 2, 3, 9, 5, 2, 5, 10, 0, 1, 10 | yes (A2) |
| ARW-B2-try1 | 6 | 25, 0, 2, 0, 1, 2, 0, 9, 0, 3, 5, 0, 3, 0, 0, 8, 4, 3 | yes (B2) |
| ARW-A3-try1 | 2 | 12, 0, 4, 5, 7, 8, 0, 10, 2, 4, 5, 5, 2, 8, 4, 2, 2, 19 | no (19 %) |
| ARW-A3-try2 | 4 | 30, 5, 2, 3, 11, 6, 4, 11, 15, 19, 9, 8, 14, 13, 13, 20, 23 | no (19-23 %) |
| ARW-A3-try3 | 10 | 36, 23, 17, 24, 26, 14, 17, 13, 18, 0, 2, 0, 1, 1, 3, 0, 1, 3 | no (17-26 %) |
| ARW-A3-try4 | 0 | 15, 1, 7, 1, 1, 0, 1, 3, 1, 2, 2, 5, 1, 2, 1, 2, 0, 4 | yes (A3) |
| ARW-B3-try1 | 4 | 12, 0, 1, 4, 7, 8, 6, 0, 5, 5, 2, 6, 2, 4, 6, 0, 0 | yes (B3) |
| RAF-warm-A | 3 | 29, 3, 2, 11, 1, 6, 2, 10, 8, 10, 6, 21, 5, 7, 12, 2, 9, 10 | warm (21 %) |
| RAF-warm-B | 10 | 32, 8, 6, 3, 34, 19, 14, 16, 6, 15, 12, 7, 5, 11, 13, 13, 13, 17 | warm (16-34 %) |
| RAF-A1-try1 | 10 | 31, 8, 8, 22, 6, 14, 11, 13, 0, 0, 2, 4, 2, 0, 3, 4, 1, 5 | no (22 %) |
| RAF-A1-try2 | 2 | 4, 6, 1, 1, 3, 1, 0, 0, 3, 1, 3, 4, 4, 0, 1, 0, 3, 3 | yes (A1) |
| RAF-B1-try1 | 4 | 7, 0, 5, 0, 5, 0, 1, 2, 1, 3, 0, 4, 17, 7, 4, 0, 7, 8 | no (17 %) |
| RAF-B1-try2 | 6 | 21, 3, 6, 3, 2, 39, 5, 5, 8, 11, 5, 3, 13, 6, 14, 5, 12, 14 | no (39 %) |
| RAF-B1-try3 | 1 | 15, 15, 9, 9, 22, 29, 20, 12, 6, 14, 13, 17, 25, 20, 6, 4, 19, 22 | no (17-29 %) |
| RAF-B1-try4 | 12 | 23, 17, 15, 16, 20, 20, 2, 0, 3, 0, 4, 5, 2, 4, 6, 9, 5, 3 | no (16-20 %) |
| RAF-A2-try1 | 2 | 9, 8, 9, 2, 7, 6, 6, 6, 5, 17, 4, 7, 6, 15, 12, 8, 5, 7 | no (17 %) |
| RAF-A2-try2 | 5 | 12, 20, 4, 6, 2, 2, 17, 7, 6, 2, 10, 8, 10, 9, 2, 9, 3, 10 | no (20, 17 %) |
| RAF-A2-try3 | 6 | 18, 6, 12, 10, 9, 8, 17, 13, 9, 15, 14, 16, 17, 18, 8, 11, 9, 22 | no (16-22 %) |
| RAF-A2-try4 | 6 | 21, 26, 8, 26, 20, 20, 10, 13, 17, 23, 17, 17, 25, 34, 24, 24, 14, 21 | no (17-34 %) |
| RAF-B2-try1 | 1 | 2, 2, 1, 0, 2, 1, 1, 3, 3, 5, 8, 8, 5, 4, 5, 3, 6, 5 | yes (B2) |
| RAF-A3-try1 | 13 | 5, 8, 9, 4, 3, 10, 8, 5, 13, 7, 3, 9, 14, 3, 4, 8, 6 | yes (A3) |
| RAF-B3-try1 | 6 | 8, 4, 2, 7, 7, 8, 9, 1, 2, 10, 6, 0, 11, 7, 1, 0, 10, 11 | yes (B3) |
| RAF-B1-try5 | 15 | 29, 8, 4, 4, 2, 7, 8, 5, 10, 22, 14, 5, 6, 23, 9, 12, 5 | no (22, 23 %) |
| RAF-B1-try6 | 11 | 20, 9, 17, 6, 13, 8, 18, 12, 17, 6, 13, 11, 7, 22, 1, 3, 1 | no (17-22 %) |
| RAF-B1-try7 | 1 | 0, 4, 1, 2, 2, 3, 3, 2, 0, 8, 3, 5, 3, 6, 2, 2, 1, 6 | yes (B1) |
| RAF-A2-try5 | 13 | 11, 1, 1, 1, 12, 3, 3, 1, 3, 8, 5, 4, 0, 3, 0, 3, 0, 7 | yes (A2) |

The first sample during a run is taken while the app starts and is not gated.
Every attempt's probe reported `unchanged=0`; `ARW-warm-A` alone saw one
press with Desktop Duplication's `AccumulatedFrames` above 1.

The prefetch-on run's four launches ([raw/runs-prefetch.txt](raw/runs-prefetch.txt))
each started under the idle gate (samples before 3, 4, 15 and 1 %) and
exited 0; their load during the run was not sampled.

## Tables

Cells are `p50 / p95` in ms, n = 60 presses on screen and 60 `page` lines in
stdout for every run. Columns are in pipeline order; the values are
`summary.md`'s. `wait` is 0.00 / 0.00 in every run and `screenFirstChange`
equals `screen` in every cell, so both are left out. Runs 1, 2 and 3 are the
slots A1 / A2 / A3 and B1 / B2 / B3.

### ARW, A (before)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.77 / 1.05 | 6.55 / 7.93 | 0.00 / 0.49 | 0.02 / 2.08 | 10.30 / 16.87 | 0.41 / 0.54 | 0.46 / 0.85 | 18.46 / 26.73 | 33.32 / 41.20 |
| 2 | 0.72 / 0.99 | 6.42 / 8.00 | 0.00 / 0.73 | 0.02 / 1.66 | 10.50 / 16.85 | 0.40 / 0.62 | 0.49 / 0.85 | 18.73 / 26.51 | 33.56 / 41.13 |
| 3 | 0.75 / 0.92 | 6.57 / 7.87 | 0.00 / 0.55 | 0.02 / 1.59 | 10.61 / 16.83 | 0.39 / 0.50 | 0.46 / 0.88 | 19.15 / 26.77 | 33.51 / 40.77 |
| **median of 3** | **0.75 / 0.99** | **6.55 / 7.93** | **0.00 / 0.55** | **0.02 / 1.66** | **10.50 / 16.85** | **0.40 / 0.54** | **0.46 / 0.85** | **18.73 / 26.73** | **33.51 / 41.13** |

### ARW, B (after)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.72 / 0.83 | 6.48 / 8.75 | 0.00 / 0.68 | 0.02 / 1.54 | 1.74 / 2.34 | 0.37 / 0.57 | 0.46 / 1.16 | 10.19 / 13.11 | 26.03 / 33.61 |
| 2 | 0.75 / 0.89 | 6.51 / 7.74 | 0.00 / 0.61 | 0.02 / 2.12 | 1.75 / 2.05 | 0.36 / 0.53 | 0.46 / 0.85 | 10.15 / 12.03 | 25.10 / 34.02 |
| 3 | 0.70 / 0.93 | 6.36 / 8.43 | 0.00 / 0.60 | 0.03 / 0.96 | 1.74 / 2.11 | 0.36 / 0.62 | 0.46 / 0.89 | 9.86 / 12.16 | 25.05 / 32.92 |
| **median of 3** | **0.72 / 0.89** | **6.48 / 8.43** | **0.00 / 0.61** | **0.02 / 1.54** | **1.74 / 2.11** | **0.36 / 0.57** | **0.46 / 0.89** | **10.15 / 12.16** | **25.10 / 33.61** |

### RAF, A (before)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 1.66 / 2.16 | 32.07 / 41.04 | 0.00 / 1.16 | 0.03 / 0.26 | 9.29 / 16.67 | 0.73 / 0.86 | 0.41 / 0.57 | 46.07 / 58.30 | 61.14 / 73.14 |
| 2 | 1.80 / 2.18 | 32.51 / 41.07 | 0.00 / 1.26 | 0.03 / 0.07 | 9.22 / 16.76 | 0.73 / 0.90 | 0.39 / 0.52 | 46.69 / 58.54 | 60.99 / 73.26 |
| 3 | 1.87 / 2.34 | 32.49 / 42.02 | 0.00 / 1.19 | 0.02 / 0.05 | 8.73 / 16.54 | 0.73 / 0.92 | 0.40 / 0.54 | 45.88 / 58.39 | 60.20 / 73.07 |
| **median of 3** | **1.80 / 2.18** | **32.49 / 41.07** | **0.00 / 1.19** | **0.03 / 0.07** | **9.22 / 16.67** | **0.73 / 0.90** | **0.40 / 0.54** | **46.07 / 58.39** | **60.99 / 73.14** |

### RAF, B (after)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 1.87 / 2.32 | 32.41 / 41.11 | 0.00 / 1.21 | 0.02 / 0.07 | 1.56 / 2.06 | 0.72 / 0.90 | 0.39 / 0.54 | 37.54 / 46.31 | 56.14 / 69.68 |
| 2 | 1.77 / 2.31 | 32.39 / 40.69 | 0.00 / 1.13 | 0.03 / 0.05 | 1.55 / 1.87 | 0.72 / 0.90 | 0.38 / 0.46 | 36.79 / 46.19 | 56.98 / 70.52 |
| 3 | 1.77 / 2.20 | 31.99 / 40.44 | 0.00 / 1.24 | 0.03 / 0.11 | 1.54 / 1.89 | 0.73 / 0.93 | 0.39 / 0.58 | 36.58 / 45.56 | 54.93 / 67.19 |
| **median of 3** | **1.77 / 2.31** | **32.39 / 40.69** | **0.00 / 1.21** | **0.03 / 0.07** | **1.55 / 1.89** | **0.72 / 0.90** | **0.39 / 0.54** | **36.79 / 46.19** | **56.14 / 69.68** |

### Prefetch on, in-app

One `--auto 59` run per build and folder, `page` lines split by `source`
([raw/prefetch.txt](raw/prefetch.txt)); p50 / p95 in ms. Each run also has two
`source=read` pages, too few for a percentile.

| Folder | Build | n (`source=prefetch`) | frameWait | keypressToPixels |
|---|---|---|---|---|
| ARW | A (before) | 57 | 9.91 / 17.41 | 10.90 / 18.30 |
| ARW | B (after) | 57 | **1.85 / 2.81** | 2.86 / 3.97 |
| RAF | A (before) | 57 | 11.82 / 17.02 | 13.11 / 18.60 |
| RAF | B (after) | 57 | **1.73 / 2.62** | 2.99 / 4.13 |

## Painted to screen, per press

As in the re-run record's
[per-press section](../_archived/20261122-tauri-gpui-latency-ab-rerun/results.md#painted-to-screen-per-press),
with both builds on its GPUI branch: press i of each used run's latprobe CSV
(`settled_ms`) is paired with the i-th `page` line carrying
`keypressToPixels` of that run's stdout. painted = `keypressToPixels -
frameWait - present`; gap = `screen` - painted, whose parts are `frameWait`,
`present` and toDwm (`screen - keypressToPixels`). This record adds
pended = `screen - (keypressToPixels - frameWait - paint - present)`, from the
page pended to the DWM present, and "first", the presses whose pended is at
most one refresh (1000 / 59 ms): those reached the screen at the first DWM
composition after the page was pended.

### Pairing check

60 / 60 presses per run. Pearson r of `screen` against `keypressToPixels`
aligned: A 0.92-1.00 on both folders, B RAF 0.76-0.80, B ARW 0.12-0.35;
shifted by one press either way -0.29 to 0.20. No negative gap; smallest gap
7.47 ms. B ARW's low r is the re-run record's Tauri ARW case: with the wait
gone, `keypressToPixels` spreads over a few ms (10.15 / 12.16 at the median)
against `screen`'s 16.9 ms quantization.

### Tables

Cells are `p50 / p95` in ms, the values `screengap.txt`'s; first is a count
of presses.

#### ARW, A (before)

| Run | gap | frameWait | present | toDwm | pended | first |
|---|---|---|---|---|---|---|
| 1 | 25.65 / 31.80 | 10.30 / 16.87 | 0.46 / 0.85 | 14.55 / 15.10 | 26.04 / 32.24 | 0 / 60 |
| 2 | 25.23 / 31.64 | 10.50 / 16.85 | 0.49 / 0.85 | 14.48 / 15.17 | 25.67 / 32.07 | 0 / 60 |
| 3 | 25.70 / 31.68 | 10.61 / 16.83 | 0.46 / 0.88 | 14.55 / 15.25 | 26.09 / 32.08 | 0 / 60 |
| **median of 3** | **25.65 / 31.68** | **10.50 / 16.85** | **0.46 / 0.85** | **14.55 / 15.17** | **26.04 / 32.08** | **0 / 180** |

#### ARW, B (after)

| Run | gap | frameWait | present | toDwm | pended | first |
|---|---|---|---|---|---|---|
| 1 | 18.04 / 26.35 | 1.74 / 2.34 | 0.46 / 1.16 | 15.31 / 24.03 | 18.54 / 26.85 | 27 / 60 |
| 2 | 17.44 / 25.76 | 1.75 / 2.05 | 0.46 / 0.85 | 14.94 / 23.36 | 17.81 / 26.08 | 28 / 60 |
| 3 | 17.62 / 25.36 | 1.74 / 2.11 | 0.46 / 0.89 | 15.03 / 23.01 | 18.05 / 26.17 | 29 / 60 |
| **median of 3** | **17.62 / 25.76** | **1.74 / 2.11** | **0.46 / 0.89** | **15.03 / 23.36** | **18.05 / 26.17** | **84 / 180** |

#### RAF, A (before)

| Run | gap | frameWait | present | toDwm | pended | first |
|---|---|---|---|---|---|---|
| 1 | 26.04 / 31.65 | 9.29 / 16.67 | 0.41 / 0.57 | 14.67 / 23.27 | 26.28 / 32.39 | 0 / 60 |
| 2 | 25.49 / 31.85 | 9.22 / 16.76 | 0.39 / 0.52 | 14.72 / 23.08 | 26.36 / 32.55 | 0 / 60 |
| 3 | 24.37 / 31.73 | 8.73 / 16.54 | 0.40 / 0.54 | 14.73 / 19.16 | 25.09 / 32.46 | 0 / 60 |
| **median of 3** | **25.49 / 31.73** | **9.22 / 16.67** | **0.40 / 0.54** | **14.72 / 23.08** | **26.28 / 32.46** | **0 / 180** |

#### RAF, B (after)

| Run | gap | frameWait | present | toDwm | pended | first |
|---|---|---|---|---|---|---|
| 1 | 19.24 / 27.99 | 1.56 / 2.06 | 0.39 / 0.54 | 17.42 / 26.01 | 20.03 / 28.71 | 21 / 60 |
| 2 | 20.30 / 27.91 | 1.55 / 1.87 | 0.38 / 0.46 | 18.13 / 25.97 | 21.01 / 28.62 | 19 / 60 |
| 3 | 18.18 / 27.09 | 1.54 / 1.89 | 0.39 / 0.58 | 16.10 / 25.15 | 19.24 / 27.92 | 21 / 60 |
| **median of 3** | **19.24 / 27.91** | **1.55 / 1.89** | **0.39 / 0.54** | **17.42 / 25.97** | **20.03 / 28.62** | **61 / 180** |

### Reading, per press

- **A always lands on the second composition.** Every A press's pended is
  between one and two refreshes but 3 of 360 just past two (17.36-37.61 ms):
  the page waits for the vsync thread's tick (`frameWait`), and the frame
  presented right after that vblank is composed at the next one (toDwm, 14.5-
  14.7 ms at p50), as the re-run record found.
- **B makes the first composition on about half the presses.** pended was at
  most one refresh on 84 of 180 ARW presses and 61 of 180 RAF presses; the
  others stayed between one and two refreshes (95 and 119), as A's do, and 1
  ARW press went past two. Those presses gained a whole refresh and the rest
  gained nothing on screen, which is the all-or-nothing gain the plan
  expected. The gap drops by 8.03 / 5.92 ms on ARW and 6.25 / 3.82 ms on RAF
  (p50 / p95), less than the `frameWait` p50 drop (8.76 ms on ARW, 7.67 ms on
  RAF), because toDwm
  now varies with the phase at which the frame is presented (B's toDwm p95
  23-26 ms against A's 15 ms on ARW).
- **The earliest a B page reached the screen was about 8 ms after it was
  pended** (smallest pended 7.95 ms on ARW, 9.01 ms on RAF over the 180
  presses each). In terms of what was measured: a page pended in the last
  8-9 ms of a refresh did not make that vblank. The presented side is
  tighter than that, since `frameWait`, `paint` and `present` take about
  2.6-2.7 ms after the page is pended: the smallest toDwm among the presses
  that made the first composition was 5.34 ms on ARW and 6.40 ms on RAF
  (`raw/screengap.txt`), so a frame presented in the last 5-6 ms of a
  refresh is the one that misses. Inferred, not measured, is why: on RAF the
  larger texture upload after `present` (the fit-preview-downscale record)
  widens that window, which fits RAF's smaller share (34 % against 47 %).

## Reading

Every difference is between the median cells above.

- **`frameWait` dropped to about zero in every B run** (1.54-1.75 ms at p50
  over the six runs, against A's 8.73-10.61 ms), with prefetch off (ruler)
  and on (in-app). No B run kept a 6-10 ms `frameWait`, so the immediate
  paint happened on every run.
- **`screen` moved by about half a refresh at p50.** ARW 33.51 to 25.10 ms
  (-8.41) at p50 and 41.13 to 33.61 (-7.52) at p95; RAF 60.99 to 56.14
  (-4.85) and 73.14 to 69.68 (-3.46). Each B run's `screen` p50 is below
  every A run's on its folder (ARW B 25.05-26.03 against A 33.32-33.56; RAF
  B 54.93-56.98 against A 60.20-61.14).
- **The in-app number overstates the screen gain.** `keypressToPixels` p50
  drops by 8.58 ms on ARW and 9.28 ms on RAF, but only the presses that make
  the earlier composition show it on screen (47 % and 34 %).
- Nothing else in the pipeline moved: `read`, `decode`, `orient`, `hop`,
  `paint` and `present` are within 0.1 ms of A's at the median p50.

## Caveats

- Only a same-session A/B is a valid comparison on this machine; these
  numbers are not to be compared with the earlier records'. This session's A
  is faster than the fit-preview-downscale record's B on both folders
  (`decode` 6.55 against 8.50 ms on ARW), as other sessions on this machine
  have varied.
- `LastPresentTime` is the DWM present, not scanout: display and panel
  latency are in no number.
- At 59 Hz a frame is about 16.9 ms, and `screen` is quantized to the
  refresh. "first" counts against 1000 / 59 ms; the bucket edges are not
  sharp to a fraction of a millisecond, but A's smallest pended is 17.36 ms.
- Differences of percentiles are not per-press differences; percentiles do
  not add up across the per-press parts.
- Input delivery (from `SendInput` to the app's key event) is inside the
  gap and pended, as in the re-run record.
- The ruler runs with prefetch off: everyday paging goes through the
  prefetch hit (`show_held`), whose on-screen gain was not measured; only
  its in-app `frameWait` was, in one run per build and folder at the default
  window size.
- `RAF-B1` and `RAF-A2` ran out of order, after `RAF-B3`, because background
  load spoiled their four tries in place.

## Reproduction

1. A: `cargo build --release --locked` in `crates/app` at `c57ec2ff`; B: the
   same at `6809229f` (or any later `main`).
2. The ruler: `tools/latprobe` of `1e97309e` on the local-only branch
   `measure/tauri-latency-ab`, `cargo build --release` in it.
3. The ruler runs (PowerShell; `.ps1` files need `-ExecutionPolicy Bypass`
   here):

   ```powershell
   powershell -ExecutionPolicy Bypass -File raw\run-ab-gpui.ps1 `
     -Gpui <A handpic.exe> -GpuiB <B handpic.exe> -Probe <latprobe.exe> -Out <out>
   ```

   A slot left empty after 4 tries:

   ```powershell
   powershell -ExecutionPolicy Bypass -Command "& raw\redo-slots.ps1 -Gpui <A> -GpuiB <B> -Probe <latprobe.exe> -Out <out> -Slots 'RAF-B1','RAF-A2'"
   ```

4. The prefetch-on run:
   `powershell -ExecutionPolicy Bypass -File raw\run-prefetch.ps1 -Gpui <A> -GpuiB <B> -Out <out>`.
5. The tables: `python raw\summarize-gpui.py <out>`,
   `python raw\screengap.py <out>` (its `USED` names this session's used
   runs) and `python raw\prefetch.py <out>`. The per-press CSVs and stdout
   they read stay in the measurement worktree's `out\20261122-present-now`.
