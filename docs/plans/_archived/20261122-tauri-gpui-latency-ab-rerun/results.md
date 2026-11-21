# Results: Tauri vs GPUI page-turn latency, re-run after the fit-view decode scale

Measured 2026-11-21 23:35 to 2026-11-22 00:11 local (the app logs stamp it in
UTC, 2026-11-21 14:35:43 to 15:11:15), both apps in one session, alternating,
against the external ruler of
[the Tauri vs GPUI A/B](../20261120-tauri-gpui-latency-ab/results.md).
GPUI is `main` with the fit view decoding a preview at the smallest `n/8` DCT
scale that covers the canvas (#1192, #1196), which
[the fit-preview-downscale record](../20261121-fit-preview-downscale/results.md)
measured GPUI against GPUI only.

**Verdict: measured to the screen, the GPUI app turns a page faster than the
Tauri app on both folders, by 11.63 ms on ARW and 64.91 ms on RAF at p50.**
Median of the three runs' p50 / p95, in ms:

| Folder | App | `screen` | `keypressToPixels` (in-app) | `screen` minus `keypressToPixels` |
|---|---|---|---|---|
| ARW 1616x1080 | GPUI | **33.52 / 41.52** | 18.54 / 26.99 | 14.98 / 14.53 |
| ARW 1616x1080 | Tauri | **45.15 / 69.01** | 17.80 / 24.90 | 27.35 / 44.11 |
| RAF 4416x2944 | GPUI | **60.14 / 72.83** | 45.16 / 57.98 | 14.98 / 14.85 |
| RAF 4416x2944 | Tauri | **125.05 / 147.53** | 113.30 / 129.60 | 11.75 / 17.93 |

- ARW: GPUI is ahead on `screen` by 11.63 ms at p50 and 27.49 ms at p95.
- RAF: GPUI is ahead on `screen` by 64.91 ms at p50 and 74.70 ms at p95.
- Tauri's `frame1` / `frame2` (median of p50 / p95): ARW 25.60 / 36.30 and
  42.30 / 53.10; RAF 113.40 / 129.70 and 116.20 / 131.20.
- The last column is a difference of the medians shown, not a per-press
  difference. The two apps' `keypressToPixels` do not share an endpoint;
  [In-app endpoints, aligned](#in-app-endpoints-aligned) compares them where
  they do.
- For GPUI that last column is the toDwm part, from the `present` stamp to the
  DWM present, not the time from the image painted to the screen; see
  [Painted to screen, per press](#painted-to-screen-per-press).

The raw output is beside this file:

- [raw/summary.md](raw/summary.md), the `summarize.py` output every per-run
  table below comes from;
- [raw/runs.txt](raw/runs.txt), every attempt, its load samples and why it
  was discarded (the original log; see [Attempts](#attempts) for the one line
  the summaries treat differently);
- [raw/endpoints.py](raw/endpoints.py) and its output
  [raw/endpoints.txt](raw/endpoints.txt), the aligned in-app endpoints;
  `endpoints.py` is byte-identical to the 20261120 A/B's;
- [raw/screengap.py](raw/screengap.py) and its output
  [raw/screengap.txt](raw/screengap.txt), the per-press painted-to-screen
  gap;
- the drivers: [raw/run-ab.ps1](raw/run-ab.ps1) (the main pass),
  [raw/redo-raf-b3.ps1](raw/redo-raf-b3.ps1) and
  [raw/redo-raf-a3.ps1](raw/redo-raf-a3.ps1) (the two redos).

The per-press CSVs and the app logs stay in the measurement worktree's
`out\20261121` folder, not in the repository.

## What each number measures

As in [the Tauri vs GPUI A/B](../20261120-tauri-gpui-latency-ab/results.md#what-each-number-measures):
`screen` is the keypress to the DWM present of the frame the patch at the
window's client center settles on, taken outside the app; GPUI's
`keypressToPixels` ends at its `present` stamp, after `frameWait`; Tauri's
ends at the return of the canvas `drawImage`, and its `frame1` / `frame2` are
the first and the nested second `requestAnimationFrame` callback after the
draw. The aligned endpoints are those of
[the latency-ab-inapp-endpoints plan](../20261121-latency-ab-inapp-endpoints/plan.md).

## Measurement conditions

- A = GPUI: `main` at `d29807fb` (which contains `9f650e9f`, the B build of
  the fit-preview-downscale record), `cargo build --release --locked` of
  `crates/app` in a temporary worktree, run as
  `handpic.exe <folder> --no-prefetch --index <temp> --settings <temp> --log <file>`
  with an index and settings file of its own in the output folder, filled by
  the warm runs and reused by the measured runs. The `page` lines are read
  from its stdout.
- B = Tauri: `1e97309e` on the local-only branch `measure/tauri-latency-ab`
  (on `a9b0e165`), the same build as in the 20261120 A/B, with that branch's
  measurement changes (timing logs always on, prefetch off, wrapping paging,
  the `frame1` / `frame2` marks, the folder from `HANDPIC_MEASURE_FOLDER`).
  The `page` lines are read from its app log.
- Prefetch is off in both apps, so every press is an uncached page turn.
- Ruler: `tools/latprobe` at `1e97309e`, unchanged. The window is placed at
  (100,100), 1600x1000; 60 presses per run, one every 1003 ms, 8 s of settle
  after launch.
- **The window is at (100,100), 1600x1000, the same as in the 20261120 A/B
  and in #1201**, so the fit view's decode scale and the drawn size match
  both records.
- Driver: that branch's `run-ab.ps1` and `summarize.py` at `1e97309e`,
  unchanged, except that the driver copy used here
  ([raw/run-ab.ps1](raw/run-ab.ps1)) adds the 1 px mouse move and back before
  each run that #1201 added, 5 lines, to keep the foreground. Per folder one
  warm run of each app (unused), then A B A B A B. Before each run, an idle
  gate of at most 15 % load and no `cargo`, `rustc`, `link`, `msbuild` or
  `cl` process; during a run the load and those processes are sampled every
  3 s, and a load sample over 15 % after the first, a compiler process or a
  non-zero probe exit discards the run, up to 4 tries per slot. The two redo
  scripts are the same driver with the main loop replaced by one slot's tries.
- Percentiles: sorted index `n / 2` and `n * 0.95` per run; the median row is
  the median of the three runs' p50s and, separately, of their p95s.
- Folders: `D:\Photos\tests\20261108-gpui-compare\ARW` (1616x1080 previews)
  and `\RAF` (4416x2944), 20 files each; 60 presses wrap each folder three
  times.
- Machine: Windows 11 Home 10.0.26200; i7-13700 (16C/24T); 32 GB RAM; RTX
  4060 Ti, driver 32.0.16.1742; display 5120x1440 at 59 Hz; `D:` a Crucial
  CT4000BX500 SATA SSD; WebView2 157.0.4341.60. The same machine as both
  earlier records.

### The fit view's decode size

GPUI's `page` lines carry `decoded=WxH`. In every one of the six used GPUI
runs, RAF shows `decoded=1656x1104` on 51 pages and `decoded=1104x736` on 9
(the portrait ones), and ARW `decoded=1212x810` on 57 and `decoded=808x540`
on 3, the same scales as the fit-preview-downscale record's B build.

### Attempts

`runs.txt` holds 29 lines: 4 warm runs and 25 measured attempts. 12 are used,
one per slot; 12 were discarded and 1 passed but was superseded:

- Used: ARW `A1-try1`, `B1-try2`, `A2-try1`, `B2-try1`, `A3-try1`,
  `B3-try4`; RAF `A1-try3`, `B1-try1`, `A2-try1`, `B2-try3`, `A3-try2`,
  `B3-try5`.
- Discarded on ARW (4): `B1-try1` and `B3` tries 1 and 2 for a load sample
  over 15 % after the first; `B3-try3` for probe exit 2, the window losing
  the foreground to another process before press 13.
- Discarded on RAF (8): `A1` tries 1 and 2, `B2` tries 1 and 2, and `B3`
  tries 1 to 3 for a load sample over 15 % after the first; `B3-try4` for
  probe exit 2, the window losing the foreground to another process before
  press 4.
- No run was discarded for a compiler process.
- Of the warm runs, `ARW-warm-A` was flagged (load samples of 16-19 % and
  probe exit 2: another process took the foreground during the settle and no
  press changed the patch); as the driver does for warm runs it was not
  redone, and it is unused either way.

**The `RAF-B3` redo.** `RAF-B3` failed all 4 tries in the main pass, which
then ended with the slot empty. It was redone right after with
[raw/redo-raf-b3.ps1](raw/redo-raf-b3.ps1), whose first try, `RAF-B3-try5`,
passed.

**The `RAF-A3` substitution.** `RAF-A3-try1` passed in the main pass, in its
place before `RAF-B3`. At the user's request `RAF-A3` was then run again with
[raw/redo-raf-a3.ps1](raw/redo-raf-a3.ps1), after `RAF-B3-try5`, so the third
RAF pair sits together in time; `RAF-A3-try2` passed. In time the redo order
is: `RAF-B3-try4` (the main pass's last run), `RAF-B3-try5`, `RAF-A3-try2`.

- `RAF-A3-try2` replaces `RAF-A3-try1` in every median of this record.
- `summarize.py` and `endpoints.py` take every non-warm `bad=[]` line of
  `runs.txt`, so [raw/summary.md](raw/summary.md) and
  [raw/endpoints.txt](raw/endpoints.txt) were computed from a copy of
  `runs.txt` that marks `RAF-A3-try1` superseded.
  [raw/runs.txt](raw/runs.txt) is the original log, where that line still
  reads `bad=[]`.
- `RAF-A3-try1` read `screen` 60.19 / 73.94, try2 60.64 / 72.04. With try1
  instead, the RAF GPUI `screen` median would be 60.14 / 73.04 instead of
  60.14 / 72.83 (from the `RAF-A1-try3`, `RAF-A2-try1` and `RAF-A3-try1`
  lines of `runs.txt`); the p50 does not move.

| Run | Sample before | Samples during | Used |
|---|---|---|---|
| ARW-warm-A | 12 | 38, 19, 16, 13, 16, 9, 17, 15, 3, 9, 7, 3, 4, 0, 0, 1, 0, 1 | warm (16-19 %; probe exit 2) |
| ARW-warm-B | 1 | 8, 1, 6, 9, 4, 7, 5, 3, 4, 2, 0, 3, 9, 6, 4, 1, 6, 7 | warm |
| ARW-A1-try1 | 0 | 9, 0, 0, 2, 4, 0, 3, 3, 3, 0, 0, 1, 2, 3, 2, 3, 1, 3 | yes (A1) |
| ARW-B1-try1 | 2 | 26, 1, 12, 4, 5, 4, 3, 3, 3, 8, 5, 3, 1, 9, 4, 18, 2, 10 | no (18 %) |
| ARW-B1-try2 | 6 | 19, 10, 4, 5, 0, 7, 3, 6, 5, 9, 9, 5, 3, 13, 3, 6, 11, 15 | yes (B1) |
| ARW-A2-try1 | 15 | 13, 6, 9, 5, 4, 4, 6, 7, 12, 3, 0, 3, 1, 1, 0, 3, 11, 1 | yes (A2) |
| ARW-B2-try1 | 4 | 15, 1, 5, 0, 0, 2, 0, 11, 12, 3, 2, 1, 2, 5, 0, 2, 1, 3 | yes (B2) |
| ARW-A3-try1 | 3 | 8, 2, 2, 1, 1, 0, 0, 2, 4, 2, 0, 0, 4, 2, 2, 1, 4, 3 | yes (A3) |
| ARW-B3-try1 | 0 | 16, 1, 6, 2, 2, 6, 7, 7, 4, 3, 2, 2, 6, 6, 0, 9, 17, 8 | no (17 %) |
| ARW-B3-try2 | 2 | 27, 2, 8, 11, 7, 4, 7, 14, 1, 15, 5, 4, 6, 9, 7, 4, 3, 27 | no (27 %) |
| ARW-B3-try3 | 2 | 27, 4, 12, 12, 12, 9 | no (probe exit 2) |
| ARW-B3-try4 | 2 | 4, 1, 2, 0, 3, 0, 2, 6, 6, 0, 1, 3, 6, 5, 2, 2, 3, 1 | yes (B3) |
| RAF-warm-A | 2 | 10, 1, 1, 0, 6, 1, 1, 4, 2, 3, 2, 4, 4, 3, 0, 0, 4, 8 | warm |
| RAF-warm-B | 2 | 27, 2, 6, 3, 9, 2, 3, 5, 0, 9, 3, 3, 3, 2, 5, 12, 10, 6 | warm |
| RAF-A1-try1 | 2 | 12, 6, 1, 6, 5, 4, 4, 5, 2, 4, 6, 3, 39, 2, 4, 5, 6, 10 | no (39 %) |
| RAF-A1-try2 | 10 | 15, 6, 17, 2, 6, 10, 7, 4, 15, 14, 10, 12, 0, 0, 3, 4, 6, 5 | no (17 %) |
| RAF-A1-try3 | 6 | 0, 0, 1, 2, 1, 0, 0, 0, 1, 5, 0, 3, 0, 0, 4, 0, 4, 0 | yes (A1) |
| RAF-B1-try1 | 2 | 19, 1, 7, 0, 8, 3, 6, 1, 7, 6, 7, 0, 3, 5, 5, 6, 9, 8 | yes (B1) |
| RAF-A2-try1 | 1 | 16, 2, 8, 1, 3, 3, 2, 0, 4, 1, 7, 6, 6, 11, 8, 2, 6, 10 | yes (A2) |
| RAF-B2-try1 | 4 | 28, 3, 15, 17, 9, 9, 17, 7, 15, 17, 13, 5, 8, 14, 4, 8, 9, 9 | no (17 % three times) |
| RAF-B2-try2 | 3 | 19, 12, 17, 8, 11, 11, 26, 9, 20, 1, 2, 3, 0, 1, 1, 16, 1, 3 | no (16-26 %) |
| RAF-B2-try3 | 3 | 23, 4, 3, 0, 0, 2, 5, 1, 3, 0, 3, 9, 1, 6, 0, 3, 5, 6 | yes (B2) |
| RAF-A3-try1 | 0 | 6, 0, 1, 3, 4, 2, 1, 3, 0, 2, 2, 0, 3, 3, 6, 4, 0, 1 | no (passed; superseded by try2) |
| RAF-B3-try1 | 1 | 28, 7, 11, 1, 11, 5, 6, 2, 8, 5, 21, 6, 3, 3, 4, 2, 6, 5 | no (21 %) |
| RAF-B3-try2 | 0 | 28, 2, 7, 10, 9, 20, 12, 8, 7, 13, 0, 5, 6, 9, 4, 9, 4, 20 | no (20 % twice) |
| RAF-B3-try3 | 6 | 20, 3, 7, 9, 16, 16, 15, 3, 1, 3, 8, 4, 1, 0, 0, 0, 0, 7 | no (16 % twice) |
| RAF-B3-try4 | 2 | 27, 4, 6 | no (probe exit 2) |
| RAF-B3-try5 | 9 | 36, 3, 5, 6, 14, 3, 3, 1, 8, 3, 4, 2, 8, 2, 6, 2, 8, 8 | yes (B3, redo) |
| RAF-A3-try2 | 10 | 30, 0, 3, 3, 2, 8, 2, 0, 4, 1, 7, 3, 0, 12, 0, 3, 5, 13 | yes (A3, redo) |

The rows are in the order of `runs.txt`, which is the order in time. The
first sample during a run is taken while the app starts and is not gated.

## Tables

Cells are `p50 / p95` in ms, n = 60 presses on screen and 60 `page` lines for
every run. Columns are in pipeline order; the values are
[raw/summary.md](raw/summary.md)'s. GPUI's `wait` is 0.00 / 0.00 in every
run and `screenFirstChange` equals `screen` in every cell of `summary.md`, so
both are left out. Runs 1, 2 and 3 are the slots A1 / A2 / A3 and B1 / B2 /
B3.

### ARW, GPUI

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.77 / 0.97 | 6.51 / 7.92 | 0.00 / 0.56 | 0.02 / 1.65 | 10.23 / 16.86 | 0.39 / 0.57 | 0.48 / 0.99 | 18.54 / 26.99 | 33.52 / 41.52 |
| 2 | 0.75 / 1.02 | 6.48 / 7.96 | 0.00 / 0.64 | 0.02 / 1.03 | 9.53 / 16.87 | 0.39 / 0.63 | 0.47 / 0.93 | 18.05 / 25.72 | 33.03 / 39.98 |
| 3 | 0.74 / 0.91 | 6.47 / 8.54 | 0.00 / 0.81 | 0.02 / 1.49 | 10.91 / 16.90 | 0.38 / 0.52 | 0.47 / 0.79 | 19.63 / 27.36 | 34.11 / 41.54 |
| **median of 3** | **0.75 / 0.97** | **6.48 / 7.96** | **0.00 / 0.64** | **0.02 / 1.49** | **10.23 / 16.87** | **0.39 / 0.57** | **0.47 / 0.93** | **18.54 / 26.99** | **33.52 / 41.52** |

### ARW, Tauri

| Run | wait | read | ipc | post | worker | decode | draw | total | keypressToPixels | frame1 | frame2 | screen |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.80 / 1.60 | 1.30 / 1.90 | 6.60 / 9.70 | 0.00 / 0.10 | 0.30 / 0.70 | 8.30 / 11.20 | 0.40 / 0.60 | 16.90 / 23.70 | 17.60 / 24.90 | 26.40 / 36.50 | 42.90 / 53.20 | 46.43 / 69.01 |
| 2 | 0.90 / 1.70 | 1.20 / 1.70 | 6.60 / 10.20 | 0.00 / 0.00 | 0.30 / 0.60 | 8.30 / 11.60 | 0.40 / 0.60 | 17.00 / 23.60 | 17.80 / 24.80 | 25.50 / 35.10 | 41.60 / 51.80 | 45.14 / 69.34 |
| 3 | 0.80 / 1.60 | 1.40 / 1.90 | 6.80 / 10.50 | 0.00 / 0.00 | 0.30 / 0.50 | 8.30 / 11.60 | 0.40 / 0.60 | 17.10 / 24.50 | 17.80 / 25.50 | 25.60 / 36.30 | 42.30 / 53.10 | 45.15 / 56.30 |
| **median of 3** | **0.80 / 1.60** | **1.30 / 1.90** | **6.60 / 10.20** | **0.00 / 0.00** | **0.30 / 0.60** | **8.30 / 11.60** | **0.40 / 0.60** | **17.00 / 23.70** | **17.80 / 24.90** | **25.60 / 36.30** | **42.30 / 53.10** | **45.15 / 69.01** |

### RAF, GPUI

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screen |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 1.88 / 2.33 | 32.87 / 42.52 | 0.00 / 1.19 | 0.02 / 0.05 | 7.55 / 16.90 | 0.73 / 0.88 | 0.40 / 0.51 | 44.91 / 58.17 | 59.36 / 73.04 |
| 2 | 1.80 / 2.37 | 32.98 / 42.28 | 0.00 / 1.08 | 0.02 / 0.06 | 6.35 / 16.62 | 0.74 / 0.90 | 0.39 / 0.54 | 45.16 / 57.98 | 60.14 / 72.83 |
| 3 | 1.83 / 2.22 | 33.15 / 41.58 | 0.00 / 1.22 | 0.02 / 0.07 | 7.89 / 16.74 | 0.72 / 0.87 | 0.39 / 0.56 | 45.36 / 57.48 | 60.64 / 72.04 |
| **median of 3** | **1.83 / 2.33** | **32.98 / 42.28** | **0.00 / 1.19** | **0.02 / 0.06** | **7.55 / 16.74** | **0.73 / 0.88** | **0.39 / 0.54** | **45.16 / 57.98** | **60.14 / 72.83** |

Run 3 is `RAF-A3-try2`, the redo.

### RAF, Tauri

| Run | wait | read | ipc | post | worker | decode | draw | total | keypressToPixels | frame1 | frame2 | screen |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.80 / 1.70 | 2.50 / 3.60 | 31.40 / 38.80 | 0.00 / 0.10 | 0.90 / 1.50 | 54.20 / 62.70 | 23.30 / 27.30 | 114.10 / 128.80 | 114.90 / 129.60 | 115.00 / 129.70 | 116.20 / 131.20 | 129.65 / 147.86 |
| 2 | 0.80 / 1.60 | 2.50 / 3.60 | 32.00 / 40.10 | 0.00 / 0.10 | 0.90 / 1.30 | 52.70 / 62.10 | 22.70 / 24.90 | 111.30 / 129.60 | 112.90 / 130.30 | 113.00 / 130.40 | 116.40 / 131.20 | 125.05 / 146.18 |
| 3 | 0.70 / 1.60 | 2.40 / 3.30 | 31.80 / 38.40 | 0.00 / 0.10 | 0.90 / 2.10 | 53.90 / 62.20 | 22.60 / 24.90 | 111.70 / 126.80 | 113.30 / 127.40 | 113.40 / 127.40 | 115.80 / 129.90 | 123.62 / 147.53 |
| **median of 3** | **0.80 / 1.60** | **2.50 / 3.60** | **31.80 / 38.80** | **0.00 / 0.10** | **0.90 / 1.50** | **53.90 / 62.20** | **22.70 / 24.90** | **111.70 / 128.80** | **113.30 / 129.60** | **113.40 / 129.70** | **116.20 / 131.20** | **125.05 / 147.53** |

Run 3 is `RAF-B3-try5`, the redo.

## Reading

Every difference below is between the median cells above; differences of
percentiles are not per-press differences.

- **GPUI's gap after `keypressToPixels` is now the same on both formats.**
  GPUI's `screen` minus `keypressToPixels` is 14.98 ms at p50 on RAF and
  14.98 ms on ARW. The extra time on the 4416x2944 image that the 20261120
  A/B left as its open question does not show in this session.
- **Tauri ARW reaches the screen after its second animation frame
  callback.** `frame1` is 25.60 and `frame2` 42.30, 16.70 ms apart, about one
  refresh at 59 Hz; `screen` is 45.15, 2.85 ms after `frame2`. Tauri's
  `screen` is 27.35 ms after its `keypressToPixels` on ARW, against 14.98 ms
  for GPUI.
- **Tauri RAF: the frame marks sit on the draw's end.** `frame1` 113.40 and
  `frame2` 116.20 are 0.10 and 2.90 ms after `keypressToPixels` 113.30, as in
  the 20261120 A/B.
- **RAF: GPUI's in-app lead now holds on screen.** In-app GPUI is 68.14 ms
  ahead at p50 (45.16 against 113.30); on screen 64.91 ms (60.14 against
  125.05).
- **Every press settled at its first change** in all 12 used runs (checked
  per press in the CSVs: `first_change_ms` equals `settled_ms` on every
  press). The CSVs' `frames` column (presented frames between a press and the
  next) is 1-4 per press for GPUI ARW, 2-3 for GPUI RAF, 3-8 for Tauri ARW
  and 3-7 for Tauri RAF.

## In-app endpoints, aligned

The two `keypressToPixels` columns end at different points. As in the
20261120 A/B (its "In-app endpoints, aligned" section), from the same
per-press `page` lines, with no new measurement:

- **painted**: the image painted, before any vsync wait. GPUI:
  `keypressToPixels - frameWait - present`, per press. Tauri:
  `keypressToPixels` as logged (the return of `drawImage`).
- **next frame**: the next frame after the paint. GPUI: `keypressToPixels`
  (the `present` stamp). Tauri: `frame2`, with `frame1` for reference.

Every number below is [raw/endpoints.txt](raw/endpoints.txt)'s, the output
of [raw/endpoints.py](raw/endpoints.py) over the 12 used runs. The Tauri
painted cells equal its `keypressToPixels`, the GPUI next frame cells its
`keypressToPixels`, and the Tauri next frame cells its `frame2` in the tables
above. Cells are `p50 / p95` in ms.

### ARW, aligned

| Run | GPUI painted | Tauri painted | GPUI next frame | Tauri next frame (`frame2`) | Tauri `frame1` |
|---|---|---|---|---|---|
| 1 | 8.08 / 10.02 | 17.60 / 24.90 | 18.54 / 26.99 | 42.90 / 53.20 | 26.40 / 36.50 |
| 2 | 7.97 / 9.56 | 17.80 / 24.80 | 18.05 / 25.72 | 41.60 / 51.80 | 25.50 / 35.10 |
| 3 | 7.73 / 9.97 | 17.80 / 25.50 | 19.63 / 27.36 | 42.30 / 53.10 | 25.60 / 36.30 |
| **median of 3** | **7.97 / 9.97** | **17.80 / 24.90** | **18.54 / 26.99** | **42.30 / 53.10** | **25.60 / 36.30** |

### RAF, aligned

| Run | GPUI painted | Tauri painted | GPUI next frame | Tauri next frame (`frame2`) | Tauri `frame1` |
|---|---|---|---|---|---|
| 1 | 35.85 / 45.49 | 114.90 / 129.60 | 44.91 / 58.17 | 116.20 / 131.20 | 115.00 / 129.70 |
| 2 | 35.68 / 45.19 | 112.90 / 130.30 | 45.16 / 57.98 | 116.40 / 131.20 | 113.00 / 130.40 |
| 3 | 35.87 / 45.08 | 113.30 / 127.40 | 45.36 / 57.48 | 115.80 / 129.90 | 113.40 / 127.40 |
| **median of 3** | **35.85 / 45.19** | **113.30 / 129.60** | **45.16 / 57.98** | **116.20 / 131.20** | **113.40 / 129.70** |

### Reading, aligned

Differences are Tauri minus GPUI, of the median cells.

- ARW, painted: GPUI is faster by 9.83 ms at p50 and 14.93 ms at p95.
- ARW, next frame: GPUI is faster by 23.76 ms at p50 and 26.11 ms at p95
  against `frame2` (7.06 / 9.31 ms against `frame1`).
- RAF, painted: GPUI is faster by 77.45 ms at p50 and 84.41 ms at p95.
- RAF, next frame: GPUI is faster by 71.04 ms at p50 and 73.22 ms at p95
  against `frame2` (68.24 / 71.72 ms against `frame1`).

GPUI painted is not a timestamp but `keypressToPixels` less two stage
durations, and its percentiles are of the per-press differences; the
20261120 A/B's caveats on the aligned endpoints apply unchanged. The
`screen` column remains the fair end-to-end comparison.

## Painted to screen, per press

The last column of the header table is a difference of medians. Here the
time from the image painted to the screen is taken press by press, from the
same per-press CSVs and `page` lines, with no new measurement. Every number
below is [raw/screengap.txt](raw/screengap.txt)'s, the output of
[raw/screengap.py](raw/screengap.py) over the 12 used runs.

- **Pairing**: press i of each used run's latprobe CSV (`settled_ms`, the
  `screen` column) is paired with the i-th `page` line carrying
  `keypressToPixels` of that run's log (GPUI's stdout, Tauri's app log).
- **painted**: as in [In-app endpoints, aligned](#in-app-endpoints-aligned).
  GPUI: `keypressToPixels - frameWait - present`. Tauri: `keypressToPixels`.
- **gap**: `screen` - painted, per press.
- GPUI's parts of the gap: `frameWait`, `present`, and **toDwm**
  (`screen - keypressToPixels`, from the `present` stamp to the DWM present).
  Tauri's: **afterFrame2** (`screen - frame2`).

The pairing check:

- Every used run has 60 presses in its CSV and 60 such `page` lines.
- Pearson r of `screen` against `keypressToPixels`, aligned, is 0.91-1.00 in
  every run except Tauri ARW's, 0.29-0.53 (see
  [Caveats, per press](#caveats-per-press)). Shifted by one press either way
  it is -0.28 to 0.25.
- No press has a negative gap; the smallest gap is 5.14 ms (`RAF-B3-try5`).

Cells are `p50 / p95` in ms. Runs 1, 2 and 3 are the same slots as in the
tables above.

### ARW, GPUI, per press

| Run | gap | frameWait | present | toDwm |
|---|---|---|---|---|
| 1 | 25.85 / 31.96 | 10.23 / 16.86 | 0.48 / 0.99 | 14.59 / 15.18 |
| 2 | 24.80 / 31.50 | 9.53 / 16.87 | 0.47 / 0.93 | 14.51 / 15.16 |
| 3 | 26.06 / 31.70 | 10.91 / 16.90 | 0.47 / 0.79 | 14.59 / 15.18 |
| **median of 3** | **25.85 / 31.70** | **10.23 / 16.87** | **0.47 / 0.93** | **14.59 / 15.18** |

### ARW, Tauri, per press

| Run | gap | afterFrame2 |
|---|---|---|
| 1 | 29.73 / 42.29 | 1.73 / 18.39 |
| 2 | 28.31 / 45.79 | 1.66 / 18.29 |
| 3 | 27.76 / 36.59 | 1.69 / 18.16 |
| **median of 3** | **28.31 / 42.29** | **1.69 / 18.29** |

### RAF, GPUI, per press

| Run | gap | frameWait | present | toDwm |
|---|---|---|---|---|
| 1 | 22.76 / 31.99 | 7.55 / 16.90 | 0.40 / 0.51 | 14.75 / 24.15 |
| 2 | 23.50 / 31.94 | 6.35 / 16.62 | 0.39 / 0.54 | 14.79 / 25.67 |
| 3 | 24.76 / 32.41 | 7.89 / 16.74 | 0.39 / 0.56 | 14.68 / 25.14 |
| **median of 3** | **23.50 / 31.99** | **7.55 / 16.74** | **0.39 / 0.54** | **14.75 / 25.14** |

### RAF, Tauri, per press

| Run | gap | afterFrame2 |
|---|---|---|
| 1 | 9.35 / 22.76 | 8.28 / 20.34 |
| 2 | 8.54 / 20.70 | 7.56 / 17.81 |
| 3 | 8.74 / 20.73 | 7.80 / 19.93 |
| **median of 3** | **8.74 / 20.73** | **7.80 / 19.93** |

### Reading, per press

- **The per-press gaps are close to the median-difference estimates.**
  `screen` minus painted of the median cells is 25.55 (ARW GPUI), 24.29 (RAF
  GPUI), 27.35 (ARW Tauri) and 11.75 ms (RAF Tauri) at p50; per press the
  gap's p50 is 25.85, 23.50, 28.31 and 8.74 ms. RAF Tauri differs most, by
  3.01 ms.
- **GPUI's gap is a vsync wait plus about one refresh.** On both formats the
  gap's p50 (25.85 ARW, 23.50 RAF) is made of `frameWait` (10.23, 7.55),
  `present` (under 0.5 ms) and toDwm (14.59, 14.75), the last close to one
  16.9 ms refresh (its p95 is 15.18 on ARW; on RAF 25.14, some presses
  waiting one refresh more).
- **The cause of GPUI's gap.** GPUI draws a notified window only when its
  vsync thread's next tick invalidates it, so the painted image first waits
  for that tick (`frameWait`); the frame is then presented right after that
  vblank, and DWM composes it at the next one (toDwm).
- **Why RAF GPUI's gap is longer than RAF Tauri's.** Tauri's long RAF draw
  ends where a frame is already due (`frame1` 0.10 ms after
  `keypressToPixels` in the median cells above), and DWM shows it 7.80 ms
  after `frame2` at p50, so its gap is under one refresh (8.74). GPUI waits
  for its vsync tick first and then for one more vblank, 23.50 ms. On ARW,
  Tauri's frame goes through two animation frames after the draw (`frame2`
  24.50 ms after `keypressToPixels` in the median cells, afterFrame2 1.69),
  so its gap (28.31) is about GPUI's (25.85).

### Caveats, per press

- Input delivery is inside the gap of both apps: `screen` counts from the
  injected keypress and the in-app stages from the app's key event, and
  painted is an in-app time.
- Percentiles do not add up across the parts: the gap's p50 is not the sum
  of its parts' p50s, nor its p95 of their p95s.
- Tauri ARW's low r comes from `screen`'s quantization to the 16.9 ms refresh
  against a narrow spread of `keypressToPixels` (17.60-25.50 ms from p50 to
  p95 in the per-run table above), not from a misaligned pairing: the
  shifted r is no higher, and no gap is negative.

## Compared with the 20261120 A/B

An observation across sessions, not a measured difference: that record says
numbers from different sessions on this machine are not to be compared.

- Its medians (`screen` p50 / p95): GPUI ARW 34.34 / 47.93, RAF
  115.46 / 140.75; Tauri ARW 48.37 / 69.53, RAF 131.11 / 150.33.
- Tauri moved little between the sessions (ARW 48.37 to 45.15, RAF 131.11 to
  125.05 at p50), and so did GPUI on ARW (34.34 to 33.52).
- GPUI's RAF `screen` p50 is 55.32 ms lower (115.46 to 60.14). That is
  consistent with the same-session result of the fit-preview-downscale record
  (RAF `screen` 112.60 to 64.94 from the fit-view decode scale).
- GPUI's RAF `screen` minus `keypressToPixels`, 47.95 ms there, is 14.98 ms
  here, equal to its ARW one (14.98 ms).

## Caveats

- `ARW-B2-try1`, a used Tauri run, has `accumulated>1=1` in `runs.txt`: one
  press (press 2) whose Desktop Duplication `AccumulatedFrames` was 2, so two
  presents were folded into one captured frame there. The other 11 used runs
  have none.
- `LastPresentTime` is the DWM present, not scanout: display and panel
  latency are in no number, and are the same for both apps.
- At 59 Hz a frame is about 16.9 ms, and `screen` is quantized to the
  refresh.
- Differences of percentiles are not per-press differences.
- Prefetch is off in both apps: this is the uncached page turn, not what the
  apps do by default.
- The Tauri in-app numbers are logged to one decimal and summarized to two;
  GPUI's are logged to two.
- `RAF-B3` and `RAF-A3` ran out of the A B A B A B order: after the main pass,
  in that order (see [Attempts](#attempts)).
- The per-run tables are not reproducible from the repository alone: the
  per-press CSVs and app logs are not committed, and
  [raw/summary.md](raw/summary.md) and [raw/endpoints.txt](raw/endpoints.txt)
  came from a `runs.txt` copy with the `RAF-A3-try1` line marked superseded.

## Reproduction

On the local-only branch `measure/tauri-latency-ab` at `1e97309e` (on
`a9b0e165`); nothing under `tools/latprobe` exists on main.

1. A: `cargo build --release --locked` in `crates/app` at `d29807fb` (or any
   later `main` with the fit-view decode scale).
2. B: on the branch, `pnpm tauri build --no-bundle`.
3. The ruler: `cargo build --release` in `tools/latprobe`.
4. The run (PowerShell; `.ps1` files need `-ExecutionPolicy Bypass` here),
   with the photo folders under its `-Root`
   (`D:\Photos\tests\20261108-gpui-compare` by default):

   ```powershell
   powershell -ExecutionPolicy Bypass -File raw\run-ab.ps1 `
     -Gpui <handpic.exe> -Tauri <Tauri handpic.exe> -Probe <latprobe.exe> -Out out\20261121
   ```

   A slot that fails all its tries is redone with the same arguments through
   a copy like [raw/redo-raf-b3.ps1](raw/redo-raf-b3.ps1).
5. Before summarizing, mark the superseded `RAF-A3-try1` line of `runs.txt`
   bad (any text inside its `bad=[]`): `summarize.py` and `endpoints.py` take
   every non-warm `bad=[]` line, and need that line excluded to give
   [raw/summary.md](raw/summary.md) and [raw/endpoints.txt](raw/endpoints.txt).
   They read the per-press CSVs and app logs in the output folder, which for
   this record stay in the measurement worktree's `out\20261121` folder.
6. The tables: `python tools\latprobe\summarize.py out\20261121`.
7. The aligned in-app endpoints: `python raw\endpoints.py out\20261121`.
8. The per-press painted-to-screen gap:
   `python raw\screengap.py out\20261121`. It hard-codes the 12 used runs
   (`USED`) and reads their per-press CSVs and app logs, which stay in the
   measurement worktree's `out\20261121` folder, as the caveat on
   reproducibility says.
