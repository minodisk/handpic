# Step 5 results: the pager with the filter and the sort against the pre-plan pager

Measured 2026-11-13 (the ARW session from 19:59 local, the RAF session from
20:13), at `40e3ec57` (Steps 1-4).

**Verdict: the gate is met on both folders.** Median of the three runs' p50
`keypressToPixels`, A minus the same-session B:

| Folder | A | B | A - B | Gate |
|--------|---|---|-------|------|
| ARW 1616x1080 | 19.62 | 22.25 | **-2.63** | <= +2.0 (met) |
| RAF 4416x2944 | 65.77 | 65.31 | **+0.46** | <= +5.0 (met) |

Not gated: the median of the three runs' p95 `keypressToPixels` is A 32.17 and
B 30.99 on ARW (+1.18), A 79.19 and B 76.40 on RAF (+2.79).

**What A measures.** A is the tree at `40e3ec57`, which holds this plan's
Steps 1-4 (#990-#993) and nothing else: B is `f080850a`, `main` right after
this plan's own PR (#989), and no other plan merged in between. So A - B is
this plan's cost on the arrow-key path: the `refilter` on a list change, the
filter and sort state and the status row's `Sort ▾` / `Filter ▾` buttons (the
menus stay closed during the runs).

## Measurement conditions

- **A** = `riffle-native` built in release in `crates/native` at `40e3ec57`.
  **B** = the pre-plan pager, a release build of a detached `git worktree` at
  `f080850a` in a scratch directory with its own `CARGO_TARGET_DIR`. Both run
  directly, no `mise` / `cargo` in front. Rust 1.98.1 (`crates/native/mise.toml`),
  NASM on `PATH`.
- Machine and drive: those of the strip plan's
  [results](../_archived/20261110-gpui-strip-index/results.md) (Windows 11,
  i7-13700, RTX 4060 Ti, 5120x1440 at 59 Hz, the SATA SSD `D:`), the app's
  default window on both.
- Files: `D:\Photos\tests\20261108-gpui-compare\ARW` at `--size 1616x1080` and
  `...\RAF` at `--size 4416x2944`, 20 files each.
- `--auto 60`, n=60 per run. Both binaries print the same `page` line; p50 is
  sorted index 30, p95 index 57.
- A and B both run with
  `--settings D:\Photos\tests\20261113-filter-sort\settings\ab.json`
  (`{"sidecarFormat":"xmp","sortOrder":"name"}`, no filter). B does not read
  `sortOrder`; both page the 20 files in the same name order. Only the arrows
  are pressed, so nothing is written.
- Indexes: a fresh `--index` per binary and folder under
  `D:\Photos\tests\20261113-filter-sort\ab\`. Warm runs of A then B were
  repeated until the binary's last scan pass ended uncanceled (pass 2's
  `scan faces:` line, on both binaries); each finished in one warm run
  (ARW 280 / 289 ms, RAF 906 / 1065 ms for pass 2), so no timed run carries a
  scan.
- Order per folder: the warm runs, then A B A B A B, each slot retried (up to
  4 times) until the idle gate passed, so the alternation held.
- No other `cargo` build ran (checked before the start).
- Script: ad hoc and not committed (a scratch `measure.py` and a PowerShell
  sampler), as in the earlier plans.

### Idle

The sampler recorded the system load and the running processes throughout. A
run was used when the load was <= 15 % before it and in every sample during
it after the first (the binary's own start), and no `cargo`, `rustc`, `link`
or other `riffle` process ran.

Tries per used run: ARW A2, A3, B1, B2 and B3 2 each, A1 1; RAF A1 2, B2 3,
B3 4, the others 1. So 11 of 23 timed attempts were discarded (ARW 5 of 11,
RAF 6 of 12), every one for a single load sample of 16-31 % during the run,
none for a build process. RAF B3 passed on its last allowed try.

## Per run p50 / p95 (ms)

| Run | read | decode | frameWait | paint | present | keypressToPixels |
|-----|------|--------|-----------|-------|---------|------------------|
| ARW A1 | 2.07 / 5.01 | 7.89 / 12.22 | 8.84 / 15.95 | 0.69 / 0.93 | 0.32 / 0.80 | 19.28 / 29.70 |
| ARW B1 | 2.41 / 3.19 | 8.93 / 14.42 | 8.75 / 16.88 | 0.68 / 1.07 | 0.30 / 0.52 | 22.25 / 30.71 |
| ARW A2 | 2.66 / 4.48 | 8.77 / 11.93 | 9.57 / 17.54 | 0.69 / 1.03 | 0.30 / 0.64 | 22.11 / 32.81 |
| ARW B2 | 2.63 / 3.87 | 9.86 / 12.95 | 8.31 / 17.02 | 0.68 / 1.02 | 0.28 / 0.55 | 22.51 / 31.85 |
| ARW A3 | 2.59 / 3.63 | 9.06 / 11.61 | 8.96 / 16.54 | 0.69 / 0.95 | 0.30 / 0.62 | 19.62 / 32.17 |
| ARW B3 | 2.56 / 4.13 | 8.96 / 12.39 | 9.50 / 17.43 | 0.68 / 0.85 | 0.27 / 0.51 | 21.97 / 30.99 |
| RAF A1 | 4.73 / 7.11 | 47.29 / 53.28 | 8.29 / 14.79 | 3.35 / 4.00 | 0.23 / 0.33 | 65.00 / 79.19 |
| RAF B1 | 4.65 / 6.59 | 46.68 / 53.84 | 9.42 / 15.81 | 3.35 / 3.82 | 0.22 / 0.35 | 66.19 / 78.31 |
| RAF A2 | 4.25 / 7.13 | 48.31 / 54.64 | 7.14 / 16.01 | 3.39 / 4.18 | 0.24 / 0.34 | 65.77 / 77.91 |
| RAF B2 | 3.58 / 6.97 | 46.43 / 53.15 | 9.13 / 16.54 | 3.42 / 4.17 | 0.22 / 0.33 | 65.31 / 76.40 |
| RAF A3 | 3.66 / 6.55 | 46.46 / 52.83 | 9.43 / 16.83 | 3.38 / 5.98 | 0.23 / 0.44 | 65.87 / 80.07 |
| RAF B3 | 2.84 / 5.94 | 46.79 / 53.83 | 7.56 / 16.44 | 3.43 / 4.10 | 0.22 / 0.32 | 64.07 / 75.06 |

`wait` and `orient` were 0 at p50 in every run; `hop` was 0.02 on ARW and
0.40-0.45 on RAF, A and B alike.

## Median of the three p50s, and A minus B (ms)

| Stage | ARW A | ARW B | ARW A - B | RAF A | RAF B | RAF A - B |
|-------|-------|-------|-----------|-------|-------|-----------|
| read | 2.59 | 2.56 | +0.03 | 4.25 | 3.58 | +0.67 |
| decode | 8.77 | 8.96 | -0.19 | 47.29 | 46.68 | +0.61 |
| frameWait | 8.96 | 8.75 | +0.21 | 8.29 | 9.13 | -0.84 |
| paint | 0.69 | 0.68 | +0.01 | 3.38 | 3.42 | -0.04 |
| present | 0.30 | 0.28 | +0.02 | 0.23 | 0.22 | +0.01 |
| **keypressToPixels** | **19.62** | **22.25** | **-2.63** | **65.77** | **65.31** | **+0.46** |

`keypressToPixels` per pair (A1-B1, A2-B2, A3-B3): ARW -2.97, -0.40, -2.35;
RAF -1.19, +0.46, +1.80.

### What the stages say

- Both deltas sit inside the run-to-run spread. A's ARW runs alone span
  19.28-22.11 ms and B's 21.97-22.51 ms; RAF's span 65.00-65.87 (A) and
  64.07-66.19 (B), and the RAF pairs change sign. The -2.63 ms on ARW is not
  a speed-up: nothing this plan added runs less per page.
- `paint` and `present` did not move (within 0.04 ms on both folders), so the
  status row's two buttons cost the frame nothing measurable, unlike the
  judgments plan's pane, which raised `present` by about 0.1 ms in every run.
- `frameWait`, the vsync phase, changes sign between the folders (+0.21 ARW,
  -0.84 RAF) as in earlier sessions.
- RAF's `read` and `decode` are about 0.6 ms higher in A at the median, but
  `read` fell run by run on both binaries (A 4.73, 4.25, 3.66; B 4.65, 3.58,
  2.84), B always running after A in its pair, and A2's `decode` alone is
  1.0-1.9 ms above every other RAF run with no change in the decode path. Not
  attributed to this plan, whose code does not run per page.
