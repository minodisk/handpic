# Step 6 results: paging while pass 2 runs against paging on a finished index

## Third run (3 pass 2 threads): invalid, the frames were throttled

Measured 2026-11-11, 21:10-21:48 local, with `riffle-native` built at
`4aefeb14`. That commit runs pass 2 on an eighth of the cores (3 of 24
logical). This session **cannot decide the gate**.

- `frameWait` was 120-145 ms at p50 in both A and B, against 8-10 ms in the
  first two runs.
- Its per-page sequence climbs from about 8 ms to about 260 ms and drops back
  each time, for example `9 26 56 69 86 102 121 ... 258 9`. That is what a
  present every about 250 ms looks like: the window was presented at about
  4 Hz.
- The throttling stopped partway through RAF B with `f` on (at about 21:47).
  The last run, RAF A with `f` on, is back at 0-16 ms.
- Inferred cause: the display slept, or the window was occluded, once the
  user left the machine idle, and DWM throttled the presents. Not verified.
- `keypressToPixels` (ARW A − B −0.20 ms, RAF +2.36 ms at the median p50)
  includes that wait, so it is not a measurement of the pass.

The stages before the frame are not affected by the throttle. Median of three
p50s, A minus B (ms):

| Stage | ARW A | ARW B | ARW A - B | RAF A | RAF B | RAF A - B |
|-------|-------|-------|-----------|-------|-------|-----------|
| read | 0.52 | 2.77 | -2.25 | 1.75 | 4.73 | -2.98 |
| decode | 6.62 | 8.62 | -2.00 | 49.59 | 46.86 | +2.73 |
| paint | 0.86 | 1.04 | -0.18 | 4.07 | 4.95 | -0.88 |
| present | 0.23 | 0.43 | -0.20 | 0.22 | 0.26 | -0.04 |

Against the second run, RAF's `decode` growth falls from +8.61 to +2.73 ms
and `paint`'s from +5.45 to -0.88 ms. B's `read` and `decode` are again slower
than A's, as in the second run.

**Pass 2 alone, untimed, at 3 threads:**

| Folder | 3 threads | 6 threads | 22 threads |
|--------|-----------|-----------|------------|
| ARW, 18,620 files | 301.2 s | 198.7 s | 85.7 s |
| RAF, 5,420 files | 225.8 s | 146.7 s | 79.6 s |

Rows written at quit: ARW 3,910-4,050 and RAF 1,410-1,420. No A run logged
`scan faces:`.

- Used runs: ARW B1, A1, B2r3, A2, B3r1, A3, Bfr3, Af; RAF B1r2, A1, B2r2, A2,
  B3, A3, Bf, Af. Discarded B attempts (a sample over 15 %): 3 ARW B2, 1 ARW B3,
  3 ARW Bf, 2 RAF B1 and 2 RAF B2. No compiler, `mise run` or other
  `riffle-native` appeared.
- The A runs' system load was 0-40 % (pass 2 on 3 threads).
- Raw data: `runs\` and `analyze-*.txt` in the test folder. The second run's
  data is in `runs-6t\` and the `*-6t.*` files.
- For a valid rerun: keep the display awake and the window unoccluded for the
  whole session (for example, the user present, or display sleep off). Then
  check every run's `frameWait` p50 is near 8-10 ms before reading the gate.

## Second run (6 pass 2 threads): ARW met, RAF missed

Measured 2026-11-11, 18:04-18:36 local, with `riffle-native` built at
`8c91bef2`. That commit runs pass 2 on `faces_threads()`, a quarter of the
cores (6 of 24 logical), instead of `scan_threads()` (22).

**Verdict: the gate is met on ARW and missed on RAF.** Median of the three runs'
p50 `keypressToPixels`, A minus the same-session B:

| Folder | A | B | A - B | Gate |
|--------|---|---|-------|------|
| ARW 1616x1080 | 18.55 | 20.25 | **-1.70** | <= +2.0 (met) |
| RAF 4416x2944 | 77.32 | 64.24 | **+13.08** | <= +5.0 (missed) |

- Per pair: ARW -1.63, -4.18, -1.15; RAF +11.82, +18.14, +11.11.
- p95, not gated: ARW A 26.24 against B 31.62 (-5.38); RAF A 93.25 against
  B 77.47 (+15.78).
- On RAF the stages that grew are **`decode` (+8.61 ms, 45.70 to 54.31)** and
  **`paint` (+5.45 ms, 3.39 to 8.84)**. Both are about 7x smaller than at
  22 threads (+64.25 / +17.37), but together they still exceed the 5 ms
  budget.

### Conditions (what differs from the first run)

- Binary: `cargo build --release` in `crates/native` at `8c91bef2`. SIMD is on
  (`jsimd.o` and `jsimdcpu.o` in the `mozjpeg-sys` build output). The first
  run's binary is kept as `riffle-native-22t.exe`.
- Pass 2 alone, untimed, with no paging, on the A template: **ARW 18,620 files
  in 198.7 s** (85.7 s at 22 threads) and **RAF 5,420 files in 146.7 s**
  (79.6 s). Each run outlasted the 60 presses by far. Rows written at quit:

| Run | ARW (of 18,620) | RAF (of 5,420) |
|-----|-----------------|----------------|
| A1 | 7,780 | 2,470 |
| A2 | 7,640 | 2,630 |
| A3 | 7,710 | 2,630 |
| A, `f` on | 7,610 | 2,630 |

  No A run logged `scan faces:`, so all 60 presses of every A run counted.
- The same folders, templates, scripts, order (a throwaway A, B A B A B A, then
  the `f` pair) and idle rule as the first run. The run indexes were fresh
  copies of the same templates.
- Idle: the user had just given the go signal, and no compiler, `mise run`,
  Tauri app or second `riffle-native` appeared in any used run. The A runs'
  system load was 25-73 % (pass 2 on 6 threads), against 96-100 % at 22.
  Discarded B attempts (a sample over 15 %): ARW B2 twice (26 %, 18 %), RAF B1
  once (20-32 %), RAF B3 three times (16-33 %). Used runs: ARW B1, A1, B2r2,
  A2, B3, A3, Bf, Af; RAF B1r1, A1, B2, A2, B3r3, A3, Bf, Af.
- Raw data: `D:\Photos\tests\20261111-gpui-pass2-measure\runs\`,
  `sampler.csv`, `idle-report.txt`, `analyze-*.txt`. The first run's data is
  in `runs-22t\` and the `*-22t.*` files.

### Per run p50 / p95 (ms)

`wait` was 0.00 / 0.00 and `orient`'s p50 0.00 in every run.

| Run | read | decode | hop | frameWait | paint | present | keypressToPixels |
|-----|------|--------|-----|-----------|-------|---------|------------------|
| ARW B1 | 2.54 / 4.14 | 8.42 / 11.88 | 0.02 / 0.04 | 9.29 / 16.88 | 0.72 / 1.10 | 0.15 / 0.40 | 20.25 / 31.62 |
| ARW A1 | 0.64 / 1.18 | 6.98 / 9.64 | 0.04 / 0.18 | 9.14 / 16.61 | 1.44 / 2.34 | 0.24 / 0.53 | 18.62 / 27.97 |
| ARW B2 | 2.70 / 3.59 | 9.00 / 12.92 | 0.02 / 0.03 | 9.24 / 15.85 | 0.71 / 1.22 | 0.17 / 0.41 | 22.18 / 33.02 |
| ARW A2 | 0.71 / 1.39 | 7.06 / 9.80 | 0.04 / 0.15 | 7.93 / 16.68 | 1.48 / 2.96 | 0.23 / 0.78 | 18.00 / 26.24 |
| ARW B3 | 1.92 / 3.54 | 8.10 / 12.44 | 0.02 / 0.03 | 8.51 / 15.94 | 0.71 / 0.97 | 0.18 / 0.44 | 19.70 / 29.11 |
| ARW A3 | 0.62 / 1.37 | 6.94 / 10.22 | 0.04 / 0.10 | 8.05 / 15.88 | 1.59 / 2.08 | 0.24 / 0.94 | 18.55 / 26.22 |
| RAF B1 | 4.41 / 6.88 | 45.70 / 53.11 | 0.38 / 0.59 | 8.92 / 15.97 | 3.38 / 4.07 | 0.15 / 0.21 | 64.24 / 75.46 |
| RAF A1 | 2.39 / 3.92 | 53.32 / 65.63 | 0.05 / 0.81 | 9.07 / 15.27 | 7.55 / 14.45 | 0.22 / 0.67 | 76.06 / 93.25 |
| RAF B2 | 2.75 / 6.17 | 44.21 / 53.05 | 0.40 / 0.54 | 8.69 / 15.61 | 3.39 / 3.91 | 0.15 / 0.21 | 62.13 / 77.47 |
| RAF A2 | 2.45 / 3.27 | 55.08 / 63.96 | 0.08 / 0.73 | 10.86 / 16.61 | 8.92 / 13.69 | 0.23 / 0.40 | 80.27 / 93.84 |
| RAF B3 | 4.87 / 7.28 | 47.91 / 55.78 | 0.31 / 0.54 | 8.44 / 16.29 | 3.41 / 3.82 | 0.15 / 0.22 | 66.21 / 80.66 |
| RAF A3 | 2.47 / 3.77 | 54.31 / 63.63 | 0.28 / 0.90 | 8.36 / 15.42 | 8.84 / 13.65 | 0.22 / 0.48 | 77.32 / 91.03 |

### Median of the three p50s, and A minus B (ms)

| Stage | ARW A | ARW B | ARW A - B | RAF A | RAF B | RAF A - B |
|-------|-------|-------|-----------|-------|-------|-----------|
| read | 0.64 | 2.54 | -1.90 | 2.45 | 4.41 | -1.96 |
| decode | 6.98 | 8.42 | -1.44 | 54.31 | 45.70 | **+8.61** |
| hop | 0.04 | 0.02 | +0.02 | 0.08 | 0.38 | -0.30 |
| frameWait | 8.05 | 9.24 | -1.19 | 9.07 | 8.69 | +0.38 |
| paint | 1.48 | 0.71 | +0.77 | 8.84 | 3.39 | **+5.45** |
| present | 0.24 | 0.17 | +0.07 | 0.22 | 0.15 | +0.07 |
| **keypressToPixels** | **18.55** | **20.25** | **-1.70** | **77.32** | **64.24** | **+13.08** |

### What the stages say

- **`paint` still grows on both folders** (ARW 0.71 to 1.48, RAF 3.39 to 8.84),
  and on RAF its p95 is 13.7-14.5 ms against B's 3.8-4.1. The main-thread
  image upload still contends with pass 2's 6 workers.
- **`decode` grows on RAF (+8.61)** but not on ARW (-1.44).
- **This session's B is slower than the first run's B** (ARW B `read`
  1.9-2.7 ms against 0.8, `decode` 8.1-9.0 against 6.2, `keypressToPixels`
  19.7-22.2 against 17.1-17.3; RAF B `read` 2.8-4.9 against 1.8-2.0), while A's
  `read` is 0.6-0.7 (ARW) and 2.4-2.5 (RAF). So the ARW A - B comes out
  negative. Not measured; two candidates:
  - (a) the cores clocking down on an otherwise idle machine, which pass 2's
    load in A prevents;
  - (b) the paged files' heads dropping out of the page cache between
    presses in B, while A's pass 2 re-reads them through the hot list.
  Either way, B's own `keypressToPixels` moved by 3-5 ms between sessions on
  the same binary family. On ARW the gate's 2 ms is within that, so the ARW
  pass is a pass under this session's conditions only.
- With `f` on, not gated: ARW A 20.60 against B 21.70 (-1.10); RAF A 79.70
  against B 63.74 (+15.96). Each log has 20 `eyes` lines.

### Next candidate (not done)

On RAF the gap left is the main-thread `paint` (+5.45) and the decode thread's
`decode` (+8.61) while 6 workers run, about 13 ms over a 62-66 ms page. Options
for the plan's decision:
- fewer pass 2 threads still (for example a sixth or an eighth of the cores),
  which lengthens pass 2 further (it is already 2.3x longer on ARW and 1.8x on
  RAF);
- pausing pass 2's compute, not only its reads, while a preview decode and
  paint are in flight (the reads already pause through `ReadSlots`);
- accepting a RAF-specific budget.

## First run (22 pass 2 threads): gate missed

Measured 2026-11-11, 08:11-09:20 local, with `riffle-native` built at `397ad733`
(Steps 1-5 merged).

**Verdict: the gate is missed on both folders, by far.** Median of the three
runs' p50 `keypressToPixels`, A (pass 2 running) minus the same-session B (both
passes complete):

| Folder | A | B | A - B | Gate |
|--------|---|---|-------|------|
| ARW 1616x1080 | 26.57 | 17.13 | **+9.44** | <= +2.0 (missed) |
| RAF 4416x2944 | 151.31 | 62.45 | **+88.86** | <= +5.0 (missed) |

Per pair (A1-B1, A2-B2, A3-B3): ARW +8.96, +9.50, +9.92; RAF +92.07, +86.10,
+88.86. Every pair misses the gate, so the miss does not come from one noisy
run.

Not gated: the median p95 `keypressToPixels` is A 39.33 and B 25.06 on ARW
(+14.27), and A 224.40 and B 73.78 on RAF (+150.62).

**The stages that grew are `decode` and `paint`**, then `read` (see "What the
stages say"). Per the plan, a fix PR lands before this step closes. The first
candidate is fewer pass 2 threads in the native app.

## Measurement conditions

- **A** = paging while pass 2 runs over the whole folder. Each run opens a fresh
  copy of a template index where pass 1 is done and every row has
  `faces_extractor = 0`. **B** = paging the same files on a fresh copy of the
  index both passes completed, so its pass 2 logs `files=0` before the first
  press.
- Binary: one `riffle-native.exe` from `cargo build --release` in
  `crates/native` at `397ad733`, run directly (a byte-identical copy in the test
  folder, so the build output was never locked). Rust 1.98.1, NASM 3.02 on
  `PATH`. SIMD is on: the `mozjpeg-sys` build output holds `jsimd.o` and
  `jsimdcpu.o`, and `crates/native/build.rs` fails the build without NASM.
- Machine: Windows 11, i7-13700 (24 logical processors), RTX 4060 Ti, 5120x1440
  at 59 Hz, the app's default 1600x1000 window. Drive `D:`, a SATA SSD (Crucial
  BX500 4 TB).
- Folders under `D:\Photos\tests\20261111-gpui-pass2-measure\`. Every file is an
  NTFS hardlink, so the copies share their data and their page cache:
  - `ARW`: 18,620 files. The 20 files of `20261108-gpui-compare\ARW` under their
    own names, plus 62 rounds of the 300 `_DSC*.ARW` of
    `20261110-gpui-pass2\ARW` (a real shoot with faces), named
    `zzNNN_<name>`.
  - `RAF`: 5,420 files. The 20 files of `20261108-gpui-compare\RAF`, plus 270
    rounds of the same 20, since there are not enough RAFs. The padding is
    renamed copies, as the plan allows.
  - The `zz` prefix sorts after the compare files, so
    `--size 1616x1080 --max 20` (ARW) and `--size 4416x2944 --max 20` (RAF)
    page exactly the 20 compare files while the scan covers the whole folder.
- `--auto 60`: 1003 ms between presses after a 2 s settle, so each run wraps
  around the 20 files three times, n=60. p50 is the sorted value at index
  `n / 2` and p95 at `int(n * 0.95)`, as in `timing.rs`.
- Pass 2 outlasted every A run. With no paging it takes 85.7 s (ARW) and
  79.6 s (RAF) on the A template. `--auto`'s quit does not stop the scan, so a
  run whose pass 2 is still running logs no `scan faces:` line. No A run logged
  one, so all 60 presses of every A run fell while pass 2 ran. The rows pass 2
  had written at quit, read from each run's index copy:

| Run | ARW (of 18,620) | RAF (of 5,420) |
|-----|-----------------|----------------|
| A1 | 13,860 | 4,090 |
| A2 | 13,220 | 4,130 |
| A3 | 13,190 | 4,150 |
| A, `f` on | 13,580 | 4,140 |

- Pass 2 ran with its defaults: `scan_threads()` = 22 workers at
  `Priority::Lowest`, with the hot list first. The hot list puts the 20 paged
  files first, so after the first seconds the presses contend with pass 2 on
  the padding files. That CPU contention is what this A/B measures.
- Warmth: both indexes were built by scanning the folders just before the
  session (untimed). One throwaway A run per folder (`warm`) read every file
  again through pass 2, so the reads of both A and B hit the page cache. The
  difference is CPU contention, not I/O.
- Order per folder: the throwaway A, then B A B A B A, then the `f`-on pair (B
  then A). A run that failed the idle rule was redone in its own slot, so the
  alternation held.

### Idle

A PowerShell sampler, running throughout, recorded `Win32_Processor.LoadPercentage`
every 3-4 s, along with:
- the running `cargo`, `rustc`, `link`, `riffle*`, `node` and `mise` processes;
- any `mise run` / `mise exec`, `cargo`, `rustc`, `link` or Vite+ / vitest /
  pnpm `node` process, as `busy:`.

The rules:
- **B** runs were used when the sample before the run was <= 15 % and every
  sample during the run after the first was <= 15 % (the first falls in the 2 s
  settle).
- **A** runs, whose own pass 2 loads the cores, were used when the sample before
  was <= 15 %.
- Both: no compiler, no `busy:` process, no Tauri app and no second
  `riffle-native` before or during the run.

The user had closed the Tauri app. **The machine was not idle otherwise.** Other
sessions ran `mise run ci` / `mise run test`, `cargo test` and their own
`riffle-native` (another worktree's `20261110-gpui-zoom-peaking`) in bursts
throughout. Before each run the script waited until two consecutive samples were
<= 15 % with none of those processes. Even so, 17 B runs and 1 A run were
discarded:

| Slot | Discarded attempts | Why |
|------|--------------------|-----|
| ARW B1 | 2 | a compile just before; samples 19-31 % |
| ARW A1 | 1 | another session's `cargo` during |
| ARW B2 | 4 | samples 17-34 %; another session's build during |
| ARW B3 | 4 | samples 17-55 %; other builds and a second `riffle-native` |
| ARW B, `f` on | 1 | the sample before was 21 % |
| RAF B1 | 2 | samples 19-35 % |
| RAF B2 | 1 | a sample of 24 % |
| RAF B, `f` on | 3 | samples 16-20 % |

The used B runs' samples after the first were 0-15 %. Two used runs started on
an earlier sample above 15 % that the rule allows: RAF B2r1's first sample was
28 % (in the settle), and its before-sample was 8 %.

Used runs, by their labels in `runs/`: ARW B1r1, A1r1, B2r4, A2, B3r4, A3,
Bfr1, Af; RAF B1r2, A1, B2r1, A2, B3, A3, Bfr3, Af. Unrelated to this
measurement and left alone: two `node` processes (PIDs 33080, 33384) present
throughout.

## Per run p50 / p95 (ms)

`wait` was 0.00 / 0.00 and `orient`'s p50 0.00 in every run.

| Run | read | decode | orient p95 | hop | frameWait | paint | present | keypressToPixels |
|-----|------|--------|------------|-----|-----------|-------|---------|------------------|
| ARW B1 | 0.81 / 1.17 | 6.22 / 8.41 | 1.21 | 0.02 / 0.08 | 8.47 / 16.10 | 0.72 / 0.94 | 0.18 / 0.59 | 17.33 / 25.15 |
| ARW A1 | 1.32 / 2.37 | 11.58 / 16.57 | 4.97 | 0.09 / 0.95 | 9.40 / 16.09 | 3.10 / 4.68 | 0.51 / 1.18 | 26.29 / 39.33 |
| ARW B2 | 0.83 / 1.46 | 6.21 / 8.46 | 1.25 | 0.02 / 0.08 | 8.75 / 16.31 | 0.69 / 0.92 | 0.18 / 0.42 | 17.07 / 25.06 |
| ARW A2 | 1.22 / 2.01 | 11.99 / 18.16 | 9.33 | 0.10 / 0.49 | 8.47 / 17.85 | 3.10 / 4.40 | 0.55 / 0.99 | 26.57 / 38.91 |
| ARW B3 | 0.79 / 1.11 | 6.10 / 8.27 | 1.04 | 0.02 / 0.05 | 7.86 / 16.71 | 0.70 / 0.98 | 0.18 / 0.69 | 17.13 / 23.51 |
| ARW A3 | 1.38 / 2.30 | 12.29 / 19.07 | 6.12 | 0.09 / 0.23 | 9.39 / 16.41 | 2.74 / 4.48 | 0.48 / 1.64 | 27.05 / 40.52 |
| RAF B1 | 2.02 / 2.56 | 47.44 / 55.18 | 11.52 | 0.32 / 0.49 | 9.55 / 16.09 | 3.46 / 3.99 | 0.16 / 0.20 | 64.02 / 76.99 |
| RAF A1 | 5.97 / 10.03 | 114.15 / 136.74 | 75.30 | 0.12 / 0.85 | 10.22 / 17.07 | 21.26 / 27.90 | 0.68 / 1.01 | 156.09 / 240.20 |
| RAF B2 | 1.83 / 2.53 | 45.77 / 54.13 | 9.65 | 0.44 / 0.75 | 10.76 / 15.92 | 3.43 / 3.91 | 0.16 / 0.20 | 62.24 / 73.69 |
| RAF A2 | 4.98 / 9.36 | 106.53 / 135.42 | 71.21 | 0.13 / 1.15 | 10.77 / 17.05 | 20.83 / 30.04 | 0.65 / 0.97 | 148.34 / 206.55 |
| RAF B3 | 1.99 / 2.66 | 45.42 / 53.54 | 9.22 | 0.43 / 0.67 | 8.85 / 16.10 | 3.52 / 3.86 | 0.15 / 0.20 | 62.45 / 73.78 |
| RAF A3 | 5.26 / 10.20 | 110.02 / 135.93 | 67.89 | 0.11 / 1.12 | 9.87 / 16.66 | 19.90 / 43.13 | 0.61 / 0.96 | 151.31 / 224.40 |

## Median of the three p50s, and A minus B (ms)

| Stage | ARW A | ARW B | ARW A - B | RAF A | RAF B | RAF A - B |
|-------|-------|-------|-----------|-------|-------|-----------|
| read | 1.32 | 0.81 | +0.51 | 5.26 | 1.99 | +3.27 |
| decode | 11.99 | 6.21 | **+5.78** | 110.02 | 45.77 | **+64.25** |
| orient | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| hop | 0.09 | 0.02 | +0.07 | 0.12 | 0.43 | -0.31 |
| frameWait | 9.39 | 8.47 | +0.92 | 10.22 | 9.55 | +0.67 |
| paint | 3.10 | 0.70 | **+2.40** | 20.83 | 3.46 | **+17.37** |
| present | 0.51 | 0.18 | +0.33 | 0.65 | 0.16 | +0.49 |
| **keypressToPixels** | **26.57** | **17.13** | **+9.44** | **151.31** | **62.45** | **+88.86** |

For context only, from other sessions (not compared): the archived strip
plan's 17.44 ms (ARW) and 60.29 ms (RAF) with no scan running. This session's
B (17.13 / 62.45) is in the same range.

### What the stages say

- **`decode` about doubles** (ARW 6.2 to 12.0 ms, RAF 45.8 to 110.0 ms). It
  runs on a background executor thread, and while the 22 pass 2 workers keep
  every logical processor at 96-100 %, it gets about half the CPU it gets on an
  idle machine. The workers' `Priority::Lowest` does not prevent this (Inferred):
  priority decides who runs on a logical processor, not the SMT sibling or the
  memory bandwidth they share. The i7-13700's E-cores are also slower if the
  decode lands there.
- **`paint` grows about 4.4x on ARW (0.70 to 3.10 ms) and 6x on RAF (3.46 to
  20.83 ms)**. That is main-thread work, the image upload inside the paint
  window, contended the same way. On RAF its p95 reaches 27.9-43.1 ms.
- **`read` grows** (ARW +0.51 ms, RAF +3.27 ms) although the files are cached:
  the copy out of the page cache is CPU work too.
- `orient`'s p95 (the portrait pages' rotation) grows from about 1 ms to
  5-9 ms on ARW and from 9-12 ms to 68-75 ms on RAF, for the same reason.
- `frameWait` (+0.9 / +0.7 ms) and `present` (+0.3 / +0.5 ms) barely move. The
  vsync and the GPU are not where the time goes.
- Pass 2's throughput while paging stayed close to its no-paging throughput
  (ARW about 13,200-13,900 rows in about 63 s, against 18,620 in 85.7 s with no
  paging). The pass does not yield to the preview; it shares the CPU with it.

## Not gated: `f` on

One A/B pair with the overlay on (`f` posted to the app's own window during the
settle). Each log has 20 `eyes` lines: the on-demand detection and judgment ran
once per file, and the later laps answered from the cache.

| Run | read | decode | frameWait | paint | keypressToPixels p50 / p95 |
|-----|------|--------|-----------|-------|----------------------------|
| ARW B, `f` on | 0.95 | 6.37 | 7.99 | 0.79 | 16.53 / 24.91 |
| ARW A, `f` on | 1.43 | 12.18 | 10.77 | 3.07 | 28.01 / 38.45 |
| RAF B, `f` on | 2.02 | 44.49 | 8.68 | 3.55 | 61.20 / 71.29 |
| RAF A, `f` on | 5.66 | 106.61 | 10.79 | 21.95 | 156.02 / 204.46 |

A - B with `f` on: ARW +11.48 ms, RAF +94.82 ms, in line with the `f`-off
pairs. On B, `f` on did not move `keypressToPixels` (ARW 16.53 against the off
runs' 17.07-17.33, RAF 61.20 against 62.24-64.02).

## Not gated: `read` during pass 2

`read` p50 during pass 2: ARW 1.22-1.38 ms, RAF 4.98-5.97 ms. Against it, during
pass 1, `native-app.md` has 0.96 ms (ARW, cold, 520 files). Pass 1 is mostly
I/O and leaves CPU free; pass 2 saturates the CPU, so a cached read costs more
during pass 2 than an I/O-bound one during pass 1.

## Method notes

- The A template is the B index with `UPDATE files SET faces_extractor = 0`
  (Python `sqlite3`), with `folders.last_viewed` cleared and `views` emptied in
  both templates, so every run starts at the first compare file. Opening the A
  template ran `scan extract files=0` and then pass 2 over the whole folder
  (`scan faces: files=18620` / `files=5420`, untimed). Each run got its own copy,
  staged ahead of the session so the copy's write-back did not fall into a
  run.
- stdout (`page` lines) and stderr (the log) went to one file. In B the
  `scan faces: files=0` line precedes the first `page` line. No A run logged
  the line, so how the two streams interleave did not decide any cut.
- Scripts and raw logs: `D:\Photos\tests\20261111-gpui-pass2-measure\`
  (`run.sh`, `slot.sh`, `next.sh`, `stage.sh`, `sampler.ps1`, `busy.ps1`,
  `sendf.ps1`, `idle.py`, `analyze.py`, `templates.py`, `runs\`,
  `sampler.csv`).
