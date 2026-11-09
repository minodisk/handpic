# Step 3 results: the pager against the spike

Measured 2026-11-09 at commit `fb41adb7` (Steps 1-2 merged).

**Verdict: the threshold is missed on both folders.** Median of the three runs'
p50 `keypressToPixels`:

| Folder | Spike p50 | Threshold | This step | Over the threshold by |
|--------|-----------|-----------|-----------|-----------------------|
| ARW 1616x1080 | 15.56 | <= 17.6 | **20.93** | 3.33 |
| RAF 4416x2944 | 57.89 | <= 62.9 | **64.76** | 1.86 |

Per the plan, Step 2 is reopened (a fix PR) before Step 4 starts. "Which stage
grew" below names the stages.

## Measurement conditions

- Build: `mise run native:release -- <folder> --size <WxH> --auto 20`, release
  (`opt-level = 3`), Rust 1.98.1. SIMD on: the `crates/native/build.rs` check
  passed with NASM 3.02 on `PATH`, and the `mozjpeg-sys` build output holds the
  assembled objects (`jsimd.o`, `jsimdcpu.o`).
- Machine: Windows 11, i7-13700, RTX 4060 Ti, 5120x1440 at 59 Hz. The window is
  the app's default 1600x1000.
- Drive: `D:`, a SATA SSD. Files: `D:\Photos\tests\20261108-gpui-compare\{ARW,RAF}`,
  20 files each.
- Warm: one throwaway run of the same command immediately before each folder's
  runs, reading every file; its numbers are not used.
- Each folder has exactly 20 files, so `--auto 20` wraps once: the 20th press
  returns to the file first shown (shown at open, untimed). All 20 files are
  timed once, n=20 per run.
- p95 at n=20 is the largest sample; p50 is the upper median (sorted index 10).
- A fraction of the pages is portrait and rotates: 1 of 20 on ARW and 3 of 20 on
  RAF. The p50 of landscape pages alone is given below. The spike did not rotate
  (its portrait files were drawn sideways).

### Idle

`Win32_Processor.LoadPercentage` and the running `cargo` / `rustc` / `riffle*`
processes were sampled before each run and every 3 s during it. The sampling
PowerShell process ran during the runs.

- A run counted as idle when the sample before it was <= 15 %, no `rustc` was
  seen before or during it, and every sample during it after the first was
  <= 15 %. The first sample is taken before the 2 s settle ends, while `mise`,
  `cargo` and the window start, so it covers no timed press.
- A debug `riffle-app` (a Tauri session of the main checkout, under
  `cargo run --no-default-features`) was running throughout, on screen, and was
  left alone, as this step did not start it. It was idle enough for the gate above but it is
  not the same as a machine with nothing else open.
- The machine was not quiet: the load reached 16-23 % during some runs, source
  not identified, and those runs were repeated (below).

Attempts under this gate (strict set, 09:41-09:47 local):

| Run | Sample before | Samples during | Used |
|-----|---------------|----------------|------|
| ARW-a1 | 2 | 2, 8, 7, 5, 3, 0 | yes |
| ARW-a2 | 14 | 3, 1, 7, 8, 1, 2 | yes |
| ARW-a3 | 6 | 11, 20, 6, 6, 2, 1 | no (20 %) |
| ARW-a4 | 2 | 5, 0, 1, 2, 2 | yes |
| RAF-a1 | 4 | 8, 10, 17, 23, 1, 6 | no (17, 23 %) |
| RAF-a2 | 7 | 17, 7, 4, 5, 12, 18 | no (18 %) |
| RAF-a3 | 8 | 11, 16, 17, 5, 3, 18 | no (16-18 %) |
| RAF-a4 | 10 | 17, 8, 5, 6, 12, 8 | yes |
| RAF-a5 | 8 | 10, 12, 12, 6, 1, 16 | no (16 %) |
| RAF-a6 | 8 | 20, 2, 0, 9, 0 | yes |
| RAF-a7 | 2 | 0, 0, 0, 15, 0, 16 | no (16 %) |
| RAF-a8 | 0 | 4, 5, 0, 0, 1 | yes |

An earlier pass with a looser gate (the median of the samples during the run
<= 15 %) also missed: ARW p50 17.77 / 17.44 / 19.99, median 17.77; RAF
65.81 / 64.45 / 65.89, median 65.81. It is not used, as half of its runs had a
sample above 15 % after the first, and a `rustc` was running during its ARW warm-up. Under
both gates the result is a miss.

## Per-run p50 / p95 (ms)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels |
|-----|------|--------|--------|-----|-----------|-------|---------|------------------|
| ARW-a1 | 1.34 / 2.99 | 7.92 / 9.72 | 0.00 / 1.44 | 0.02 / 0.13 | 11.48 / 17.58 | 0.76 / 2.12 | 0.16 / 0.35 | 21.10 / 30.41 |
| ARW-a2 | 1.96 / 5.54 | 7.75 / 11.52 | 0.00 / 1.30 | 0.02 / 0.13 | 5.99 / 16.12 | 0.72 / 1.54 | 0.15 / 0.21 | 20.93 / 29.95 |
| ARW-a4 | 1.64 / 4.06 | 7.84 / 11.80 | 0.00 / 1.49 | 0.02 / 0.12 | 8.11 / 14.64 | 0.72 / 1.59 | 0.15 / 0.30 | 18.33 / 28.86 |
| **ARW median of p50s** | 1.64 | 7.84 | 0.00 | 0.02 | 8.11 | 0.72 | 0.15 | **20.93** |
| Spike ARW (p50; p95 of `keypressToPixels` 24.44) | 0.74 | 6.71 | not a stage | 0.03 | 6.27 | 0.83 (paint + present) | | 15.56 |
| RAF-a4 | 3.61 / 6.75 | 47.56 / 52.48 | 0.00 / 8.86 | 0.41 / 0.66 | 8.91 / 14.66 | 3.46 / 8.87 | 0.15 / 0.27 | 64.73 / 72.82 |
| RAF-a6 | 3.76 / 6.58 | 48.80 / 54.68 | 0.00 / 10.49 | 0.38 / 0.60 | 8.09 / 15.58 | 3.42 / 7.27 | 0.15 / 0.21 | 64.76 / 75.16 |
| RAF-a8 | 3.62 / 7.55 | 47.09 / 57.16 | 0.00 / 8.25 | 0.43 / 0.62 | 8.50 / 15.37 | 3.49 / 7.34 | 0.15 / 0.21 | 66.01 / 78.85 |
| **RAF median of p50s** | 3.62 | 47.56 | 0.00 | 0.41 | 8.50 | 3.46 | 0.15 | **64.76** |
| Spike RAF (p50; p95 of `keypressToPixels` 68.11) | 1.80 | 46.05 | not a stage | 0.39 | 6.81 | 3.55 (paint + present) | | 57.89 |

`wait` was 0.00 / 0.00 in every run. Stage p50s do not add up to
`keypressToPixels`; each p50 comes from a different page.

p95, not gated: 29.95 ms ARW and 75.16 ms RAF (median of the runs), each the
largest of 20 samples, against the spike's single run at 24.44 and 68.11.

## Which stage grew

Median-of-p50s against the spike, in ms:

| Stage | ARW | RAF |
|-------|-----|-----|
| read | +0.90 | +1.82 |
| decode | +1.13 | +1.51 |
| frameWait | +1.84 | +1.69 |
| paint + present | +0.04 | +0.06 |
| hop | -0.01 | +0.02 |
| orient (p50 over all pages) | 0.00 | 0.00 |

No single stage carries the miss. On ARW `frameWait` grew most, then `decode`
and `read`; on RAF `read` grew most, then `frameWait` and `decode`.

- The added `orient` stage is not the cause at p50: most pages are landscape
  and copy nothing. Taking the landscape pages alone gives p50 21.10 / 20.25 /
  17.77 (ARW, median 20.25) and 64.67 / 62.76 / 65.94 (RAF, median 64.67),
  still both over the threshold.
- `frameWait` is the wait for the next vsync, which depends on the phase at
  which the decode ends. It varied from 5.99 to 11.48 between ARW runs of the
  same 20 files, so a 2.0 ms ARW budget sits within its run-to-run spread at
  n=20. That is an observation about the gate, not a reason to pass it.
- `keypressToPixels` minus `frameWait` per page gives p50 10.62 ms (ARW) and
  56.93 ms (RAF), against the spike's 8.34 and 51.72 excluding vsync: 2.3 and
  5.2 ms slower, and the work of the page and not the vsync phase.
- Why `read` and `decode` are slower than in the spike's one run is not known
  from these numbers. The spike's conditions cannot be repeated (its code is
  gone), and these runs shared the machine with a running Tauri app and other
  sessions' load, as listed above.

## Portrait files (`orient`)

Two portrait files per case, `--max 2 --auto 10` (10 presses alternating them),
one idle run each after a throwaway warm run, from
`D:\Photos\tests\20261109-gpui-portrait` (hard links to `D:\photos\samples`,
nothing written in the samples folders).

| Preview | Files | `orient` p50 (ms) | Budget |
|---------|-------|-------------------|--------|
| ARW 1616x1080, orientation 6 | `DSC-RX100M7A_DSC00422.ARW`, `DSLR-A450_20170423_113243_A450_07665.ARW` | 1.67 | <= 2 (met) |
| RAF 4000x3000, orientation 8 | `GFX100_II_fujifilm_gfx_100_ii_33.raf`, `GFX_100_fujifilm_gfx_100_13.raf` | 8.22 | <= 15 (met) |
| RAF 4416x2944, orientations 6 and 8 | `X-H2S_fujifilm_x_h2s_30.raf`, `X-H2_cmp_6001.RAF` | 8.66 | (the 15 ms budget names 4000x3000) |

## Visual check of the RAF fit view

**Native side only.** In the native window (`riffle-native`, default size,
screenshots of the 1616x1039 window) the RAF fit views of
`X-H2_fujifilm_x_h2_08.raf` (brick walls, cobblestones),
`X-H2S_fujifilm_x_h2s_31.raf` (brickwork, signage) and
`X-H2_fujifilm_x_h2_50.raf` (brickwork) show no moire at the downscale of about
3x. The window's one-mip texture is not excluded as a cause on other images.
**The comparison with the Tauri app next to it was not made**, so the plan's
check is unfinished. See the pending manual check in `learnings.md`. The Step 5
follow-up was not added, as no aliasing was observed.

## Re-measurement 2026-11-09 (12:20-12:42 local): A/B against the spike binary

The first pass above is kept as it was. This section re-measures, on a quieter
machine, to separate noise from a real regression: same session, alternating,
with both binaries.

- **A** = the current `riffle-native` release build (`mise run native:release`
  built at `d97a6270`, whose native code is that of `fb41adb7`, then the binary
  run directly so that neither side has `mise` / `cargo` in front of it).
- **B** = the original throwaway `gpui-spike.exe` from the spike (built with
  SIMD on 2026-11-08, not rebuilt).

**Verdict, two separate lines:**

1. **Against the plan's original thresholds (unchanged): missed on both.** A's
   median p50 `keypressToPixels` is **20.16 ms on ARW** (threshold <= 17.6) and
   **65.85 ms on RAF** (threshold <= 62.9).
2. **A new comparison, not the plan's gate: A minus the same-session B is within
   the plan's margins on both.** A minus B of the median p50
   `keypressToPixels` is **+0.30 ms on ARW** (margin +2.0) and **+1.26 ms on
   RAF** (margin +5.0).

Same-session B itself does not meet those thresholds either: its median p50 was
19.86 ms on ARW and 64.59 ms on RAF, against 15.56 and 57.89 for its one run in
the spike. The same binary differs by 4.3 ms (ARW) and 6.7 ms (RAF) between
those two measurements. These runs do not say why. The spike's run timed n=20
pages of a 40-file listing; this one n=60 pages over 20 files.

### Conditions

- As the first pass: Windows 11, i7-13700, RTX 4060 Ti, 5120x1440 at 59 Hz,
  `D:` SATA SSD, `D:\Photos\tests\20261108-gpui-compare\{ARW,RAF}` (20 files
  each, `--size 1616x1080` and `--size 4416x2944`), default 1600x1000 window on
  both binaries, release builds, SIMD on for both (A: the `crates/native/build.rs`
  check with NASM on `PATH`).
- `--auto 60`, 1003 ms between presses after a 2 s settle, on both. Each run
  **wraps around the 20 files three times**: n=60 per run (every file timed
  three times).
- Warm: one throwaway run of A with `--auto 60` before each folder's runs. Its
  numbers are not used.
- Order per folder: warm, then A B A B A B. A discarded run was redone in its
  own slot, so the alternation held. Nothing else was started during the
  sequence; the user had closed the Tauri app and left the machine.
- A has the `orient` stage and the status text, the spike has neither. The
  spike also draws the portrait files sideways (no rotation).
- Percentile rule: the printed p50 / p95 are each binary's own. A takes sorted
  index `n / 2` (30 of 60) and `n * 0.95` (57); the spike takes index
  `ceil(p * n) - 1` (29 and 56). The two are one rank apart, so the tables
  below are not strictly the same statistic; the same-rule check is below.
  The spike's `frame_wait` is shown as `frameWait`.

### Idle

Before each run and every 3 s during it, `Win32_Processor.LoadPercentage` and
the running `cargo` / `rustc` / `link` / `riffle*` / `gpui-spike` / `node`
processes were recorded; the sampling PowerShell ran during the runs. A run was
used when the sample before it was <= 15 %, no compiler or `riffle-app`
appeared before or during it, and every sample during it after the first was
<= 15 % (the first is during the 2 s settle, before any press).

**No `riffle-app`, `cargo`, `rustc` or `link` was present before or during any
run.** Three `node` processes (PIDs 33080, 33384, 46464) were present
throughout, not started by this measurement and left alone; they stayed under
the load gate.

Discarded and redone (a sample after the first above 15 %):

| Run | Samples over 15 % |
|-----|-------------------|
| ARW-A1, attempt 1 | 19 |
| ARW-A2, attempt 1 | 16 |
| ARW-B2, attempt 1 | 20, 17 |
| RAF-B1, attempt 1 | 16, 20 |
| RAF-A2, attempt 1 | 20 |
| RAF-A3, attempt 1 | 16, 39 |

The first sample of a used run was up to 20 % (ARW B2 and B3), in the settle,
as in the first pass. ARW's warm-up run also had a 17 % sample; it is a
throwaway and was not repeated.

### Per run p50 / p95 (ms)

`orient` is A only.

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels |
|-----|------|--------|--------|-----|-----------|-------|---------|------------------|
| ARW A1 | 2.44 / 2.87 | 9.24 / 12.79 | 0.00 / 1.28 | 0.02 / 0.11 | 8.36 / 16.81 | 0.71 / 1.04 | 0.14 / 0.24 | 20.16 / 30.68 |
| ARW B1 | 2.48 / 3.10 | 9.00 / 12.72 | n/a | 0.02 / 0.04 | 6.78 / 16.30 | 0.72 / 1.03 | 0.11 / 0.21 | 19.64 / 28.86 |
| ARW A2 | 2.26 / 3.35 | 8.61 / 12.96 | 0.00 / 1.17 | 0.02 / 0.12 | 6.80 / 15.17 | 0.73 / 1.10 | 0.14 / 0.25 | 18.38 / 29.57 |
| ARW B2 | 2.54 / 2.96 | 8.93 / 12.63 | n/a | 0.02 / 0.04 | 7.14 / 15.61 | 0.72 / 0.93 | 0.10 / 0.14 | 19.86 / 27.88 |
| ARW A3 | 2.62 / 3.22 | 9.25 / 12.03 | 0.00 / 1.32 | 0.02 / 0.11 | 6.47 / 16.76 | 0.74 / 1.14 | 0.14 / 0.26 | 20.33 / 30.45 |
| ARW B3 | 2.52 / 3.04 | 9.12 / 12.37 | n/a | 0.02 / 0.03 | 9.01 / 15.77 | 0.72 / 1.00 | 0.11 / 0.20 | 20.28 / 29.13 |
| RAF A1 | 4.52 / 6.69 | 47.80 / 54.32 | 0.00 / 9.04 | 0.42 / 0.56 | 9.27 / 15.27 | 3.37 / 3.97 | 0.13 / 0.19 | 65.85 / 79.62 |
| RAF B1 | 4.41 / 6.59 | 47.38 / 55.49 | n/a | 0.42 / 0.59 | 9.12 / 15.54 | 3.38 / 3.98 | 0.10 / 0.12 | 65.02 / 76.69 |
| RAF A2 | 4.91 / 7.02 | 46.70 / 54.65 | 0.00 / 8.79 | 0.42 / 0.64 | 9.87 / 16.44 | 3.38 / 4.11 | 0.13 / 0.19 | 66.05 / 80.43 |
| RAF B2 | 4.10 / 6.58 | 48.07 / 55.18 | n/a | 0.41 / 0.59 | 8.88 / 14.62 | 3.37 / 3.77 | 0.10 / 0.12 | 64.59 / 75.53 |
| RAF A3 | 4.45 / 7.34 | 47.30 / 52.80 | 0.00 / 8.90 | 0.45 / 0.61 | 8.40 / 16.15 | 3.41 / 3.85 | 0.13 / 0.21 | 65.74 / 76.71 |
| RAF B3 | 4.25 / 6.65 | 47.80 / 56.07 | n/a | 0.42 / 0.60 | 6.91 / 15.97 | 3.36 / 3.79 | 0.10 / 0.12 | 63.33 / 74.66 |

`wait` was 0.00 / 0.00 in every run.

### Median of the three p50s, and A minus B in the same session (ms)

| Stage | ARW A | ARW B | ARW A - B | RAF A | RAF B | RAF A - B |
|-------|-------|-------|-----------|-------|-------|-----------|
| read | 2.44 | 2.52 | -0.08 | 4.52 | 4.25 | +0.27 |
| decode | 9.24 | 9.00 | +0.24 | 47.30 | 47.80 | -0.50 |
| orient | 0.00 | n/a | n/a | 0.00 | n/a | n/a |
| hop | 0.02 | 0.02 | 0.00 | 0.42 | 0.42 | 0.00 |
| frameWait | 6.80 | 7.14 | -0.34 | 9.27 | 8.88 | +0.39 |
| paint | 0.73 | 0.72 | +0.01 | 3.38 | 3.37 | +0.01 |
| present | 0.14 | 0.11 | +0.03 | 0.13 | 0.10 | +0.03 |
| **keypressToPixels** | **20.16** | **19.86** | **+0.30** | **65.85** | **64.59** | **+1.26** |

Median of the three p95s of `keypressToPixels`, not gated: A 30.45 and B 28.86
on ARW, A 79.62 and B 75.53 on RAF.

`keypressToPixels` per A / B pair (A1-B1, A2-B2, A3-B3), printed values: ARW
+0.52, -1.48, +0.05; RAF +0.83, +1.46, +2.41. The ARW difference straddles
zero; the RAF difference is positive in all three pairs.

### Is there a real difference?

Between A and B, no stage shows a difference beyond run-to-run variation, with
the one exception named below. `frameWait` p50 still moves 6.47-8.36 ms between
A's ARW runs (B: 6.78-9.01), and its A - B changes sign by pair, so on ARW it
is noise, as the first pass suspected.

- Same percentile rule on both, from the per-page lines (index `n / 2`, A in
  0.01 ms and the spike in 0.1 ms): `keypressToPixels` A minus B **+0.06 ms on
  ARW and +1.05 ms on RAF**.
- `keypressToPixels` minus `frameWait` per page (the page's work, not the vsync
  phase), same rule: ARW A 12.69 and B 12.70, RAF A 56.53 and B 56.50. **The
  work of the page is equal on both.**
- Landscape files only (the same files; the portrait files excluded by name):
  ARW A 19.59 and B 20.00, RAF A 64.57 and B 65.00, so A is 0.41 and 0.43 ms
  faster.
- **The RAF A - B of about +1 ms comes with the `orient` stage on the portrait
  files.** They are 3 of the 20 RAF files, 9 of the 60 pages, about 9 ms each;
  the spike draws them sideways and pays nothing. The difference is present
  over all pages and gone on the landscape files. That is added functionality,
  not a slowdown of the shared work.

In short, the first pass's gap to the spike was not a regression of the page
work: A and the same-session B do it in the same time, and the remaining
difference is `orient` on portrait pages on RAF. Whether the plan's absolute
thresholds should stand is left to the user.
