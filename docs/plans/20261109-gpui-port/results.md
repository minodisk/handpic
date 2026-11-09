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
