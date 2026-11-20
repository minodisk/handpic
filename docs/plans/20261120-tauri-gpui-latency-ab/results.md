# Results: Tauri vs GPUI page-turn latency, keypress to the screen

Measured 2026-11-20, 08:19-08:49 local (the app logs stamp it in UTC,
2026-11-19 23:19:54 to 23:49:13), both apps in one session, alternating,
against one external ruler.

**Verdict: measured to the screen, the GPUI app turns a page faster than the
Tauri app on both folders.** Median of the three runs' p50 / p95 of `screen`
(keypress to the DWM present of the frame that shows the new image), in ms:

| Folder | App | `screen` | `keypressToPixels` (in-app) | `screen` minus `keypressToPixels` |
|---|---|---|---|---|
| ARW 1616x1080 | GPUI | **34.34 / 47.93** | 20.79 / 29.01 | 13.55 / 18.92 |
| ARW 1616x1080 | Tauri | **48.37 / 69.53** | 20.20 / 29.40 | 28.17 / 40.13 |
| RAF 4416x2944 | GPUI | **115.46 / 140.75** | 67.51 / 79.46 | 47.95 / 61.29 |
| RAF 4416x2944 | Tauri | **131.11 / 150.33** | 115.60 / 134.90 | 15.51 / 15.43 |

- ARW: GPUI is ahead by 14.03 ms at p50 and 21.60 ms at p95.
- RAF: GPUI is ahead by 15.65 ms at p50 and 9.58 ms at p95.

The two in-app `keypressToPixels` numbers do not share an endpoint (below), so
neither the even ARW numbers (20.79 against 20.20) nor the wide RAF gap
(67.51 against 115.60) predicted what reaches the screen. The last column is a
difference of the medians shown, not a per-press difference.

The raw output is kept beside this file: [raw/summary.md](raw/summary.md) (the
`summarize.py` output every table below comes from) and
[raw/runs.txt](raw/runs.txt) (every attempt, its load samples and why it was
discarded). The per-press CSVs and the app logs stay in the measurement
worktree's `out\20261120` folder.

## What each number measures

- `screen`: from the `SendInput` of the Right key to the DWM present of the
  first frame from which the patch at the window's client center holds the
  value it settles on before the next press. Taken outside the app, the same
  for both.
- `screenFirstChange`: from the same keypress to the first presented frame in
  which that patch differs from its value at the press.
- GPUI `keypressToPixels`: from the key event to the `present` stamp, after
  `frameWait` and `present`; its stages are `wait`, `read`, `decode`, `orient`,
  `hop`, `frameWait`, `paint` and `present`.
- Tauri `keypressToPixels`: from the key event to the return of the canvas
  `drawImage` (as in
  [the latency breakdown](../_archived/20261108-preview-latency-breakdown/results.md)).
  The vsync wait, WebView compositing and the present are in none of its
  stages (`wait`, `read`, `ipc`, `post`, `worker`, `decode`, `draw`, `total`).
- Tauri `frame1` / `frame2`: the first and the nested second
  `requestAnimationFrame` callback scheduled after the draw, in ms from the
  same start as `keypressToPixels`. Added for this measurement only.

Not measured: scanout and the panel. `LastPresentTime` is the DWM present, so
display and panel latency are in no number here; they are the same for both
apps.

## Measurement conditions

- A = GPUI: main at `dc6fa3d7`, a release build of `crates/native` (Rust
  1.98.1), run as
  `handpic.exe <folder> --no-prefetch --index <temp> --settings <temp> --log <file>`.
  The `page` lines are read from its stdout.
- B = Tauri: `a9b0e165` (the parent of `80221f61`, which removed the app) plus
  the measurement commit `1e97309e` on the local-only branch
  `measure/tauri-latency-ab`, built with `pnpm tauri build --no-bundle`. The
  `page` lines are read from its app log. The measurement commit:
  - turns the timing logs on always;
  - disables prefetch, to match `--no-prefetch`;
  - makes paging wrap at the end, as the GPUI pager does (the Tauri app
    clamped);
  - adds the `frame1` / `frame2` marks to the keyed `page` lines;
  - makes `last_folder` honor `HANDPIC_MEASURE_FOLDER`, so the driver picks
    the folder;
  - sets the identifier to `dev.minodisk.handpic.latencyab` and disables the
    data migration, because the changed identifier would otherwise move the
    real app's data.
- Prefetch is off in both apps, so every press is an uncached page turn.
- Ruler: `tools/latprobe` on that branch (Rust, `windows` 0.58).
  - The QPC is stamped just before `SendInput(VK_RIGHT)`.
  - DXGI Desktop Duplication gives each desktop frame's `LastPresentTime`
    (the QPC of the DWM present).
  - Per frame, an FNV hash of a 64x64 patch at the window's client center.
  - `settled` (the `screen` column) and `first_change` (the
    `screenFirstChange` column) per press, as above.
  - The window is placed at (100,100), 1600x1000. 60 presses per run, one
    every 1003 ms.
  - Why not PresentMon: WebView2's presents come from a separate GPU process
    and are hard to tie to an input. Why not the Event Timing API: the Tauri
    draw is asynchronous after IPC and a worker, so the event's next paint is
    not the frame of the image.
- Driver: `tools/latprobe/run-ab.ps1` on that branch.
  - Per folder, one warm run of each app (unused), then A B A B A B.
  - Before each run, an idle gate: `Win32_Processor.LoadPercentage` <= 15 %
    and no `cargo`, `rustc`, `link`, `msbuild` or `cl` process.
  - During a run, the load and those processes are sampled every 3 s. A run
    with a load sample over 15 % after the first, a compiler process or a
    non-zero probe exit is discarded and redone in its slot, up to 4 tries.
  - 8 s of settle after launch before the first press.
- Percentiles (`summarize.py`): sorted index `n / 2` and `n * 0.95`, so at
  n = 60 the p50 is the 31st and the p95 the 58th sample. Per run p50 / p95;
  the median row is the median of the three runs' p50s and, separately, of
  their p95s.
- Files: `D:\Photos\tests\20261108-gpui-compare\ARW` (1616x1080 previews) and
  `\RAF` (4416x2944), 20 files each. 60 presses wrap the folder three times,
  so each file is timed three times per run.
- Machine: Windows 11 Home 10.0.26200; i7-13700 (16C/24T); 32 GB RAM; RTX
  4060 Ti, driver 32.0.16.1742; display 5120x1440 at 59 Hz; `D:` a Crucial
  CT4000BX500 SATA SSD; WebView2 157.0.4341.60.

### Attempts

`runs.txt` holds 25 lines: 4 warm runs and 21 measured attempts. 12 measured
attempts are used, one per slot; 9 were discarded, every one for a load sample
over 15 % after the first, none for a compiler process or a probe exit: 6 on
ARW (A1 two, B1 two, A2 one, B3 one) and 3 on RAF (B1, A3 and B3 one each).

Of the warm runs, 3 were flagged and, as the driver does for warm runs, not
redone; they are unused either way. `ARW-warm-A` also ended with probe exit 2:
the window lost the foreground before press 31, so the ARW GPUI warm run
pressed only 30 times.

The first sample during a run is taken while the app starts and is not gated.

| Run | Sample before | Samples during | Used |
|---|---|---|---|
| ARW-warm-A | 8 | 34, 20, 7, 1, 10, 7, 6, 6, 2, 2 | warm (20 %; probe exit 2) |
| ARW-warm-B | 11 | 23, 17, 17, 16, 0, 5, 10, 8, 22, 6, 3, 12, 17, 2, 7, 2, 21 | warm (16-22 %) |
| ARW-A1-try1 | 0 | 18, 1, 6, 3, 16, 17, 4, 10, 34, 1, 2, 9, 1, 0, 13, 8, 7 | no (16, 17, 34 %) |
| ARW-A1-try2 | 12 | 30, 4, 14, 14, 7, 9, 6, 23, 9, 7, 12, 9, 3, 18, 12, 17, 6 | no (17, 18, 23 %) |
| ARW-A1-try3 | 7 | 23, 1, 0, 3, 0, 8, 1, 1, 0, 0, 3, 1, 7, 0, 3, 3, 0, 5 | yes (A1) |
| ARW-B1-try1 | 2 | 23, 2, 7, 5, 9, 0, 9, 13, 17, 1, 1, 1, 0, 0, 2, 0, 4, 2 | no (17 %) |
| ARW-B1-try2 | 1 | 14, 0, 0, 4, 1, 2, 0, 0, 0, 7, 31, 3, 7, 0, 0, 6, 2 | no (31 %) |
| ARW-B1-try3 | 5 | 14, 7, 8, 0, 11, 0, 3, 4, 9, 0, 2, 5, 2, 6, 6, 0, 6 | yes (B1) |
| ARW-A2-try1 | 1 | 8, 6, 6, 0, 0, 31, 3, 2, 2, 3, 12, 8, 3, 5, 4, 3, 11 | no (31 %) |
| ARW-A2-try2 | 7 | 0, 0, 0, 0, 2, 0, 1, 0, 4, 0, 5, 0, 0, 2, 2, 8, 0 | yes (A2) |
| ARW-B2-try1 | 0 | 46, 3, 8, 2, 2, 5, 0, 1, 0, 7, 1, 0, 3, 2, 4, 8, 1 | yes (B2) |
| ARW-A3-try1 | 3 | 8, 2, 1, 0, 4, 7, 15, 7, 1, 7, 0, 0, 0, 2, 2, 4, 0, 8 | yes (A3) |
| ARW-B3-try1 | 9 | 35, 4, 3, 1, 1, 7, 7, 9, 2, 18, 0, 8, 3, 3, 1, 11, 4 | no (18 %) |
| ARW-B3-try2 | 3 | 22, 2, 5, 1, 4, 7, 2, 5, 7, 2, 1, 1, 6, 4, 2, 4, 5 | yes (B3) |
| RAF-warm-A | 4 | 39, 6, 3, 2, 0, 6, 0, 3, 0, 0, 1, 0, 0, 3, 1, 4, 2 | warm |
| RAF-warm-B | 5 | 33, 1, 0, 4, 0, 1, 0, 0, 3, 4, 2, 2, 0, 4, 5, 1, 20 | warm (20 %) |
| RAF-A1-try1 | 0 | 6, 0, 0, 7, 2, 2, 8, 0, 1, 0, 0, 9, 2, 6, 2, 2, 1 | yes (A1) |
| RAF-B1-try1 | 0 | 15, 5, 0, 7, 7, 6, 5, 5, 7, 4, 1, 39, 5, 4, 6, 3, 4 | no (39 %) |
| RAF-B1-try2 | 4 | 27, 1, 3, 3, 5, 8, 8, 1, 0, 11, 4, 6, 4, 5, 11, 8, 2 | yes (B1) |
| RAF-A2-try1 | 0 | 9, 10, 5, 3, 1, 2, 3, 0, 0, 2, 2, 0, 3, 2, 1, 8, 2 | yes (A2) |
| RAF-B2-try1 | 0 | 15, 2, 0, 1, 0, 2, 4, 0, 5, 0, 0, 4, 4, 3, 8, 3, 0 | yes (B2) |
| RAF-A3-try1 | 2 | 4, 16, 2, 4, 1, 0, 6, 0, 0, 9, 0, 2, 2, 2, 3, 0, 4 | no (16 %) |
| RAF-A3-try2 | 0 | 2, 0, 1, 0, 3, 5, 2, 0, 0, 7, 2, 8, 0, 2, 4, 2, 7 | yes (A3) |
| RAF-B3-try1 | 2 | 16, 0, 9, 7, 6, 11, 10, 6, 7, 14, 5, 43, 12, 6, 12, 13, 8 | no (43 %) |
| RAF-B3-try2 | 6 | 23, 10, 7, 11, 5, 8, 0, 2, 2, 0, 4, 4, 0, 1, 8, 4, 5 | yes (B3) |

## Tables

Cells are `p50 / p95` in ms, n = 60 presses on screen and 60 `page` lines in
the log for every run. Columns are in pipeline order; the values are
`summary.md`'s.

### ARW, GPUI

| Run | wait | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screenFirstChange | screen |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.00 / 0.00 | 1.02 / 2.06 | 6.66 / 8.80 | 0.00 / 1.25 | 0.02 / 1.64 | 10.99 / 17.42 | 0.70 / 0.87 | 0.46 / 0.84 | 19.40 / 29.01 | 34.34 / 42.16 | 34.34 / 42.16 |
| 2 | 0.00 / 0.00 | 1.60 / 3.02 | 8.59 / 12.66 | 0.00 / 1.11 | 0.02 / 1.59 | 10.87 / 18.61 | 0.68 / 0.89 | 0.43 / 0.93 | 22.50 / 33.32 | 37.35 / 49.00 | 37.35 / 49.00 |
| 3 | 0.00 / 0.00 | 0.78 / 1.20 | 6.33 / 8.62 | 0.00 / 1.17 | 0.02 / 1.90 | 11.83 / 17.60 | 0.71 / 0.88 | 0.47 / 0.84 | 20.79 / 28.85 | 34.27 / 47.93 | 34.27 / 47.93 |
| **median of 3** | **0.00 / 0.00** | **1.02 / 2.06** | **6.66 / 8.80** | **0.00 / 1.17** | **0.02 / 1.64** | **10.99 / 17.60** | **0.70 / 0.88** | **0.46 / 0.84** | **20.79 / 29.01** | **34.34 / 47.93** | **34.34 / 47.93** |

### ARW, Tauri

| Run | wait | read | ipc | post | worker | decode | draw | total | keypressToPixels | frame1 | frame2 | screenFirstChange | screen |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 2.30 / 4.10 | 1.60 / 2.70 | 8.10 / 11.50 | 0.00 / 0.10 | 0.20 / 0.40 | 8.10 / 11.60 | 0.40 / 0.70 | 18.70 / 25.90 | 20.90 / 29.40 | 28.70 / 37.70 | 44.80 / 54.10 | 48.37 / 61.64 | 48.37 / 61.64 |
| 2 | 2.00 / 3.90 | 1.40 / 2.40 | 7.70 / 11.60 | 0.00 / 0.10 | 0.20 / 1.00 | 8.20 / 11.50 | 0.40 / 0.50 | 18.00 / 25.00 | 20.20 / 27.50 | 28.50 / 38.10 | 44.90 / 54.50 | 48.09 / 70.55 | 48.09 / 70.55 |
| 3 | 2.20 / 4.40 | 1.50 / 2.70 | 8.00 / 11.70 | 0.00 / 0.00 | 0.30 / 0.80 | 8.40 / 11.10 | 0.40 / 0.50 | 18.10 / 26.60 | 19.90 / 31.10 | 28.00 / 37.70 | 44.50 / 54.30 | 48.94 / 69.53 | 48.94 / 69.53 |
| **median of 3** | **2.20 / 4.10** | **1.50 / 2.70** | **8.00 / 11.60** | **0.00 / 0.10** | **0.20 / 0.80** | **8.20 / 11.50** | **0.40 / 0.50** | **18.10 / 25.90** | **20.20 / 29.40** | **28.50 / 37.70** | **44.80 / 54.30** | **48.37 / 69.53** | **48.37 / 69.53** |

### RAF, GPUI

| Run | wait | read | decode | orient | hop | frameWait | paint | present | keypressToPixels | screenFirstChange | screen |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.00 / 0.00 | 3.77 / 6.10 | 46.56 / 55.37 | 0.00 / 9.20 | 0.03 / 0.52 | 11.14 / 17.72 | 3.41 / 3.78 | 0.43 / 0.61 | 67.51 / 80.21 | 115.46 / 141.72 | 115.46 / 141.72 |
| 2 | 0.00 / 0.00 | 4.33 / 6.56 | 47.14 / 55.22 | 0.00 / 9.17 | 0.23 / 0.53 | 9.09 / 16.97 | 3.36 / 3.91 | 0.42 / 0.65 | 66.10 / 77.73 | 114.39 / 140.39 | 114.39 / 140.39 |
| 3 | 0.00 / 0.00 | 4.61 / 7.04 | 48.37 / 54.60 | 0.00 / 8.75 | 0.39 / 0.63 | 9.68 / 17.51 | 3.30 / 3.81 | 0.43 / 0.58 | 67.73 / 79.46 | 117.83 / 140.75 | 117.83 / 140.75 |
| **median of 3** | **0.00 / 0.00** | **4.33 / 6.56** | **47.14 / 55.22** | **0.00 / 9.17** | **0.23 / 0.53** | **9.68 / 17.51** | **3.36 / 3.81** | **0.43 / 0.61** | **67.51 / 79.46** | **115.46 / 140.75** | **115.46 / 140.75** |

### RAF, Tauri

| Run | wait | read | ipc | post | worker | decode | draw | total | keypressToPixels | frame1 | frame2 | screenFirstChange | screen |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 2.50 / 4.20 | 2.70 / 5.10 | 32.30 / 41.70 | 0.00 / 0.10 | 0.90 / 1.20 | 52.40 / 61.70 | 22.30 / 25.20 | 112.00 / 129.00 | 115.20 / 131.40 | 115.60 / 131.60 | 116.20 / 132.40 | 128.30 / 147.45 | 128.30 / 147.45 |
| 2 | 2.00 / 5.30 | 3.00 / 4.30 | 31.30 / 39.80 | 0.00 / 0.10 | 0.90 / 1.30 | 53.00 / 63.20 | 23.40 / 28.70 | 113.40 / 132.70 | 115.80 / 134.90 | 116.10 / 135.10 | 117.90 / 135.70 | 131.11 / 150.33 | 131.11 / 150.33 |
| 3 | 2.40 / 4.80 | 3.20 / 5.60 | 33.20 / 42.80 | 0.00 / 0.10 | 0.90 / 1.30 | 52.90 / 64.10 | 23.20 / 26.70 | 112.20 / 140.40 | 115.60 / 142.70 | 115.80 / 142.90 | 118.20 / 143.30 | 133.84 / 151.55 | 133.84 / 151.55 |
| **median of 3** | **2.40 / 4.80** | **3.00 / 5.10** | **32.30 / 41.70** | **0.00 / 0.10** | **0.90 / 1.30** | **52.90 / 63.20** | **23.20 / 26.70** | **112.20 / 132.70** | **115.60 / 134.90** | **115.80 / 135.10** | **117.90 / 135.70** | **131.11 / 150.33** | **131.11 / 150.33** |

## Reading

Every difference below is between the median cells above; differences of
percentiles are not per-press differences.

- **The old in-app comparison stopped too early for Tauri.** On ARW the two
  apps' `keypressToPixels` are even at p50 (GPUI 20.79, Tauri 20.20), as the
  earlier comparison read, but on screen the Tauri app is 14.03 ms later.
  28.17 ms of the Tauri app's `screen` comes after its `keypressToPixels`
  endpoint, against 13.55 ms for GPUI, whose in-app number already includes
  `frameWait` (10.99) and `present` (0.46).
- **Tauri ARW reaches the screen after its second animation frame callback.**
  `frame1` is 28.50 and `frame2` 44.80, 16.30 ms apart, about one refresh at
  59 Hz; `screen` is 48.37, 3.57 ms after `frame2`. The image the canvas drew
  reaches the DWM after the second callback, not around the first.
- **Tauri RAF: the frame marks sit on the draw's end.** `frame1` 115.80 and
  `frame2` 117.90 are within 2.30 ms of `keypressToPixels` 115.60, not a
  refresh apart as on ARW. A reading, not established here: the 23.20 ms
  synchronous `draw` runs past the frame deadline, so both callbacks run as
  soon as it returns. `screen` is 15.51 ms after `keypressToPixels`.
- **RAF: GPUI's in-app lead shrinks on screen.** In-app GPUI is 48.09 ms ahead
  at p50 (67.51 against 115.60); on screen 15.65 ms (115.46 against 131.11).
  GPUI's `screen` is 47.95 ms after its `keypressToPixels` on RAF, against
  13.55 ms on ARW, although its in-app number already ends at the `present`
  stamp. So on the 4416x2944 image the stamp precedes the frame reaching the
  DWM by about 34 ms more than on ARW. See the open question.
- **Every press settled at its first change.** `first_change_ms` equals
  `settled_ms` on every press of all 12 used runs (checked per press in the
  CSVs), so `screenFirstChange` equals `screen` in every cell: the patch never
  showed an intermediate value. The CSVs' `frames` column (presented frames
  between a press and the next) is 1-3 per press for GPUI ARW, 2-3 for GPUI
  RAF and 3-7 for Tauri on both folders.

## Caveats

- `LastPresentTime` is the DWM present, not scanout: display and panel latency
  are in no number, and are the same for both apps.
- At 59 Hz a frame is about 16.9 ms, and `screen` is quantized to the refresh;
  the 14-16 ms p50 differences are about one frame.
- Differences of percentiles are not per-press differences.
- `ARW-B3-try2`, a used Tauri run, has one press (press 16) whose Desktop
  Duplication `AccumulatedFrames` was 2 (`accumulated>1=1` in `runs.txt`), so
  two presents were folded into one captured frame there. The other 11 used
  runs have none.
- Prefetch is off in both apps: this is the uncached page turn, not what the
  apps do by default.
- The Tauri in-app numbers are logged to one decimal and summarized to two
  (`28.50`); GPUI's are logged to two.
- `screen` starts at the `SendInput` stamp, before the OS input path, so it
  holds input delivery that neither in-app number has; it is the same ruler for
  both apps.
- Only a same-session A/B is a valid comparison on this machine, as
  [the gpui-port results](../20261109-gpui-port/results.md) found; the numbers
  here are not to be compared with other sessions'.

## Open question

What GPUI's extra ~34 ms on RAF between the `present` stamp and the DWM
present is. A hypothesis: the 4416x2944 texture's upload or sampling on the
GPU, which the in-app `present` stamp does not wait for. Nothing in this data
separates it from other causes; following it up (where the time goes, whether
the stamp should move) is outside this plan.

## Reproduction

On the local-only branch `measure/tauri-latency-ab` at `1e97309e` (on
`a9b0e165`); nothing under `tools/latprobe` exists on main.

1. A: a release build of `crates/native` at `dc6fa3d7` (Rust 1.98.1).
2. B: on the branch, `pnpm tauri build --no-bundle`.
3. The ruler: `cargo build --release` in `tools/latprobe`.
4. The run (PowerShell), with the photo folders under its `-Root`
   (`D:\Photos\tests\20261108-gpui-compare` by default):

   ```powershell
   tools\latprobe\run-ab.ps1 -Gpui <handpic.exe> -Tauri <Tauri handpic.exe> `
     -Probe <latprobe.exe> -Out out\20261120
   ```

5. The tables: `python tools\latprobe\summarize.py out\20261120`.
