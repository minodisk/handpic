# Step 5 results: the pager with the strip against the pre-strip pager

Measured 2026-11-10, 20:48-21:17 local, at `7a5eaf5f` (Steps 1-4 merged).

**Verdict: the gate is met on both folders.** Median of the three runs' p50
`keypressToPixels`, A minus the same-session B:

| Folder | A | B | A - B | Gate |
|--------|---|---|-------|------|
| ARW 1616x1080 | 17.44 | 16.64 | **+0.80** | <= +2.0 (met) |
| RAF 4416x2944 | 60.29 | 59.78 | **+0.51** | <= +5.0 (met) |

Not gated: the median of the three runs' p95 `keypressToPixels` is A 25.43 and
B 24.38 on ARW (+1.05), A 71.82 and B 72.26 on RAF (-0.44).

## Measurement conditions

- **A** = `riffle-native` built with `cargo build --release` in `crates/native`
  at `7a5eaf5f` (the strip, the index, pass 1, the hot list and the folder
  watcher). **B** = the pre-strip pager, `cargo build --release` of a detached
  `git worktree` at `fb7fcdf4` in a scratch directory with its own
  `CARGO_TARGET_DIR` (the pager with `orient`, no index, no strip). Both run
  directly, no `mise` / `cargo` in front.
- Rust 1.98.1, `opt-level = 3`, NASM 3.02 on `PATH`. SIMD on for both: each
  build's `crates/native/build.rs` ran and passed (only its two
  `cargo:rerun-if-env-changed` lines in B's `-vv` build output, no
  `cargo:warning`), and each `mozjpeg-sys` build output holds the assembled
  `jsimd.o` and `jsimdcpu.o`.
- Machine: Windows 11, i7-13700 (24 logical processors), RTX 4060 Ti (driver
  32.0.16.1742), 5120x1440 at 59 Hz. The window is the app's default 1600x1000
  on both.
- Drive: `D:`, a SATA SSD (Crucial BX500 4 TB). Files:
  `D:\Photos\tests\20261108-gpui-compare\{ARW,RAF}`, 20 files each,
  `--size 1616x1080` and `--size 4416x2944`.
- `--auto 60`, 1003 ms between presses after a 2 s settle: each run wraps
  around the 20 files three times, n=60 per run. Both binaries print the same
  `page` line and take p50 at sorted index `n / 2` (30) and p95 at
  `n * 0.95` (57), so A and B are the same statistic this time.
- A's index: a fresh `--index D:\Photos\tests\20261110-gpui-strip-index\ab-<folder>.sqlite`
  per folder. The throwaway warm run of A ran pass 1 over the folder (20 files),
  so every timed A run opened a warm index (`todo=0`). A resumes at the last
  viewed file; 60 presses over 20 files end where they started, so every file
  is timed three times on both binaries.
- Order per folder: one throwaway warm run of A, then A B A B A B. A run that
  failed the idle gate was redone in its own slot, so the alternation held.

### Idle

A PowerShell sampler, running throughout, recorded `Win32_Processor.LoadPercentage`
(retried when it came back empty) and the running `cargo`, `rustc`, `link`,
`riffle*`, `gpui-spike`, `node` and `mise` processes every 3 s. A run was used
when the sample before it was <= 15 %, no compiler, `riffle-app` or second
`riffle-native` appeared before or during it, and every sample during it
after the first was <= 15 % (the first falls in the 2 s settle).

**No `riffle-app`, `cargo`, `rustc` or `link` was present before or during any
run**, and the only `riffle-native` was the run's own. Present throughout and
left alone: three `node` processes (PIDs 33080, 33384, 46464) and one `mise`
(PID 16528), not started by this measurement. The user had closed the Tauri
app.

Discarded and redone (a sample over 15 %):

| Run | Sample before | Samples over 15 % after the first |
|-----|---------------|-----------------------------------|
| ARW-A1, attempt 1 | 32 | 28, 40, 25, 21, 23, 16, 20, 22 |
| ARW-A2, attempt 1 | 1 | 20 |
| ARW-A3, attempt 1 | 4 | 22 |
| ARW-A3, attempt 2 | 2 | 18 |
| ARW-B3, attempt 1 | 3 | 30 |
| RAF-B1, attempt 1 | 1 | 24 |
| RAF-B2, attempt 1 | 7 | 18 |
| RAF-B2, attempt 2 | 2 | 16 |
| RAF-B3, attempt 1 | 1 | 23 |

ARW's warm-up had samples up to 41 %; it is a throwaway and was not repeated.
The used runs' samples after the first were 0-13 %.

## Per run p50 / p95 (ms)

| Run | read | decode | orient | hop | frameWait | paint | present | keypressToPixels |
|-----|------|--------|--------|-----|-----------|-------|---------|------------------|
| ARW A1 | 0.92 / 1.26 | 6.45 / 8.50 | 0.00 / 1.59 | 0.02 / 0.05 | 8.50 / 16.16 | 0.71 / 0.99 | 0.18 / 0.85 | 16.77 / 25.43 |
| ARW B1 | 0.87 / 1.08 | 6.41 / 8.37 | 0.00 / 1.58 | 0.02 / 0.12 | 7.98 / 16.57 | 0.71 / 0.97 | 0.13 / 0.22 | 16.64 / 26.08 |
| ARW A2 | 0.93 / 1.37 | 6.33 / 8.74 | 0.00 / 1.57 | 0.02 / 0.06 | 9.67 / 16.61 | 0.69 / 0.90 | 0.18 / 0.55 | 17.81 / 25.39 |
| ARW B2 | 0.72 / 1.08 | 6.04 / 8.29 | 0.00 / 1.03 | 0.02 / 0.10 | 8.39 / 16.22 | 0.70 / 0.87 | 0.13 / 0.19 | 15.41 / 24.38 |
| ARW A3 | 0.77 / 1.31 | 6.31 / 8.07 | 0.00 / 1.22 | 0.02 / 0.06 | 8.75 / 16.38 | 0.71 / 1.45 | 0.17 / 0.58 | 17.44 / 25.54 |
| ARW B3 | 0.73 / 0.93 | 6.28 / 8.31 | 0.00 / 1.29 | 0.02 / 0.10 | 8.34 / 15.93 | 0.69 / 0.90 | 0.13 / 0.18 | 16.98 / 24.01 |
| RAF A1 | 1.76 / 2.29 | 45.41 / 53.44 | 0.00 / 9.29 | 0.43 / 0.65 | 9.00 / 16.42 | 3.35 / 3.77 | 0.15 / 0.22 | 60.29 / 76.85 |
| RAF B1 | 1.76 / 2.28 | 44.03 / 52.18 | 0.00 / 8.99 | 0.44 / 0.71 | 6.93 / 16.14 | 3.42 / 4.18 | 0.13 / 0.21 | 59.78 / 72.26 |
| RAF A2 | 1.79 / 2.11 | 44.74 / 52.10 | 0.00 / 9.39 | 0.44 / 0.63 | 9.56 / 16.32 | 3.41 / 3.77 | 0.15 / 0.26 | 61.63 / 71.82 |
| RAF B2 | 1.79 / 2.08 | 44.28 / 51.70 | 0.00 / 9.39 | 0.44 / 0.73 | 9.07 / 16.57 | 3.38 / 3.91 | 0.13 / 0.18 | 61.50 / 72.94 |
| RAF A3 | 1.78 / 2.20 | 43.48 / 51.79 | 0.00 / 9.22 | 0.43 / 0.65 | 6.23 / 15.21 | 3.39 / 3.66 | 0.15 / 0.21 | 59.28 / 71.42 |
| RAF B3 | 1.79 / 2.24 | 43.81 / 52.21 | 0.00 / 9.31 | 0.44 / 0.66 | 7.36 / 16.18 | 3.41 / 3.80 | 0.13 / 0.22 | 59.28 / 71.15 |

`wait` was 0.00 / 0.00 in every run.

## Median of the three p50s, and A minus B (ms)

| Stage | ARW A | ARW B | ARW A - B | RAF A | RAF B | RAF A - B |
|-------|-------|-------|-----------|-------|-------|-----------|
| read | 0.92 | 0.73 | +0.19 | 1.78 | 1.79 | -0.01 |
| decode | 6.33 | 6.28 | +0.05 | 44.74 | 44.03 | +0.71 |
| orient | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| hop | 0.02 | 0.02 | 0.00 | 0.43 | 0.44 | -0.01 |
| frameWait | 8.75 | 8.34 | +0.41 | 9.00 | 7.36 | +1.64 |
| paint | 0.71 | 0.70 | +0.01 | 3.39 | 3.41 | -0.02 |
| present | 0.18 | 0.13 | +0.05 | 0.15 | 0.13 | +0.02 |
| **keypressToPixels** | **17.44** | **16.64** | **+0.80** | **60.29** | **59.78** | **+0.51** |

`keypressToPixels` per pair (A1-B1, A2-B2, A3-B3): ARW +0.13, +2.40, +0.46;
RAF +0.51, +0.13, 0.00.

### What the stages say

- `paint` did not move (+0.01 / -0.02 ms): the strip's canvas, which paints
  the visible cells, costs nothing measurable inside the preview's paint
  window, as Step 2's quick run suggested.
- `present` grew by 0.05 ms at p50 on ARW and its p95 went from 0.18-0.22 to
  0.55-0.85 ms (RAF: 0.21-0.26 against 0.18-0.22). A frame now carries the
  strip's thumbnails too; under 1 ms either way.
- `frameWait` carries most of the A - B (+0.41 ARW, +1.64 RAF), but it is the
  vsync phase and moves 6.2-9.6 ms between runs of the same binary on RAF
  (B: 6.9-9.1), as in the port's Step 3. The RAF pair differences of
  `keypressToPixels` (+0.51, +0.13, 0.00) do not follow it.
- `read` +0.19 ms on ARW: the runs' ranges overlap (A 0.77-0.93, B 0.72-0.87)
  and the difference is under 0.2 ms; RAF's read is
  equal. `decode` +0.71 ms on RAF is within the runs' spread (A 43.5-45.4,
  B 43.8-44.3). Neither is attributed to the strip.
- B is not the spike any more: it has `orient`, so the portrait pages cost the
  same on both, unlike the port's Step 3 re-measurement.
- These absolute numbers are lower than the port's Step 3 session (ARW B
  16.64 here against the pager's 20.16 then; `read` 0.7-0.9 against 2.4,
  `decode` 6.3 against 9.2). Same files and machine; not explained, and not
  compared across sessions, as the plan says.

## During-scan figure (recorded, not gated)

A run of A on 520 ARW files (`_DSC*.ARW` from `D:\Photos\2026\2026-10-17`,
Sony 1616x1080 previews) copied under `D:\Photos\tests\20261110-gpui-strip-index\`,
each run on a fresh `--index`, no `--size`, `--auto 60`. "Inside pass 1" is
the `page` lines printed before the run's `scan extract` line.

| Run | Folder state | Pass 1 | Pages inside pass 1 | read p50 / p95 / max | keypressToPixels p50 / p95 |
|-----|--------------|--------|---------------------|----------------------|----------------------------|
| A, pause on | copied with `cp` (buffered), first read | 520 files in 3.2 s | 2 | 0.39 / 0.39 / 0.39 | 15.03 / 15.03 |
| no-pause build | the same copy, second read (cached) | 520 files in 0.41 s | 0 | (none) | (none) |
| **A, pause on** | a fresh copy with `robocopy /J` (unbuffered, so not cached) | 520 files in 27.7 s | **26** | **0.96 / 1.22 / 1.26** | **16.95 / 21.50** |
| no-pause build | another fresh `robocopy /J` copy (the next 520 files) | 320 of 520 when the run quit, 63 s | 60 | 0.90 / 1.27 / 336.75 | 15.07 / 22.83 |

The no-pause build is `crates/native` at `7a5eaf5f` with
`let paused = slots.pause();` in `load` replaced by `let paused = ();`, built
into a separate target directory and not committed.

Against Tauri's `READ_SLOTS` figures (cap 2, pause on, cold first pass of ARW
files on the same SATA SSD: read p50 14-16 ms, p95 18-19 ms, from
`docs/agents/tauri-app.md`), the native preview read during pass 1 is
**0.96 ms p50 and 1.22 ms p95**. These are not the same measurement:

- The native presses step to the next file, and the hot list (`focus.rs`) puts
  the current file and the strip's range first, so the scan has usually read
  the next file's head before the press comes. The preview read is then a
  cache hit; that is what the hot list is for, and it is what a user stepping
  through a folder gets. Tauri's figure is a read of a file the scan had not
  read yet, queued behind the scan's reads in flight.
- **Whether the pause shortened `read`: not measurably at p50 / p95** (0.96 /
  1.22 on, 0.90 / 1.27 off), as both are mostly cache hits. The one slow read,
  336.75 ms on the second press, came in the no-pause run, which is consistent
  with a preview read queued behind the scan's reads without the pause. It is
  one sample, and that run's disk was much slower (320 files in 63 s against
  520 in 27.7 s, right after the SSD had taken 40 GB of writes), so it is not
  attributed to the pause.
- The load gate does not apply to these runs: during pass 1 the load is the
  app's own scan (the pause-on cold run sampled 13-40 %). The no-pause cold run
  sampled 0-9 %, as its scan was waiting on the disk.
