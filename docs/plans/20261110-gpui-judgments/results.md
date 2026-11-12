# Step 5 results: the pager with the pane and the judgments against the pre-plan pager

Measured 2026-11-13 (the ARW session from 06:19 local), at `eb46913b`
(Steps 1-5).

**Verdict: the gate is met on both folders.** Median of the three runs' p50
`keypressToPixels`, A minus the same-session B:

| Folder | A | B | A - B | Gate |
|--------|---|---|-------|------|
| ARW 1616x1080 | 22.43 | 21.58 | **+0.85** | <= +2.0 (met) |
| RAF 4416x2944 | 64.02 | 65.47 | **-1.45** | <= +5.0 (met) |

Not gated: the median of the three runs' p95 `keypressToPixels` is A 31.97 and
B 31.63 on ARW (+0.34), A 78.90 and B 76.33 on RAF (+2.57).

**What A measures.** A is the tree at `eb46913b`, which holds this plan's
Steps 1-5 and also the other plans' work merged since `de249218`: the 1:1 view
on `z` (#979), the `f` focus-marks overlay (#974, #977, #978) and pass 2 of the
scan in the native app (#972). So the A - B is the cost of everything landed
since `de249218` on the arrow-key path, not of the meta pane and the strip's
marks alone.

## Measurement conditions

- **A** = `riffle-native` built in release in `crates/native` at `eb46913b`.
  **B** = the pre-plan pager, a release build of a detached `git worktree` at
  `de249218` in a scratch directory with its own `CARGO_TARGET_DIR`. Both run
  directly, no `mise` / `cargo` in front. Rust 1.98.1 (`crates/native/mise.toml`),
  NASM on `PATH`.
- Machine and drive: those of the strip plan's
  [results](../_archived/20261110-gpui-strip-index/results.md) (Windows 11,
  i7-13700, RTX 4060 Ti, 5120x1440 at 59 Hz, the SATA SSD `D:`), the app's
  default window on both.
- Files: `D:\Photos\tests\20261108-gpui-compare\ARW` at `--size 1616x1080` and
  `...\RAF` at `--size 4416x2944`, 20 files each.
- `--auto 60`, 1003 ms between presses after a 2 s settle, n=60 per run. Both
  binaries print the same `page` line and percentile rule (p50 at sorted index
  30, p95 at 57).
- A runs with `--settings D:\Photos\tests\20261110-gpui-judgments\settings\xmp.json`
  (`"xmp"`); only the arrows are pressed, so nothing is written. B has no
  `--settings` flag.
- Indexes: a fresh `--index` per binary and folder under
  `D:\Photos\tests\20261110-gpui-judgments\ab\`. Warm runs of A then B were
  repeated until the binary's last scan pass ended uncanceled (A: pass 2's
  `scan faces:` line, B: pass 1's `scan extract:` line); each finished in one
  warm run, so no timed run carries a scan.
- Order per folder: the warm runs, then A B A B A B, each slot retried (up to
  4 times) until the idle gate passed, so the alternation held.
- Script: ad hoc and not committed (a scratch `measure.py` and a PowerShell
  sampler), as in the strip plan.

### Idle

The sampler recorded the system load and the running processes throughout. A
run was used when the load was <= 15 % before and during it and no `cargo`,
`rustc`, `link` or `node` build process ran.

Tries per used run: ARW A1 3, A2 3, A3 2, each B 1; RAF A2 2, A3 2, the others 1.

The first ARW session (2026-11-13 05:52) is void: another session's `cargo`
build ran through its first half and A1 used up its 4 tries. ARW was rerun at
06:19 with no build running; the numbers below are the rerun's.

## Per run p50 / p95 (ms)

| Run | read | decode | frameWait | paint | present | keypressToPixels |
|-----|------|--------|-----------|-------|---------|------------------|
| ARW A1 | 2.66 / 3.56 | 8.96 / 12.99 | 8.02 / 16.88 | 0.68 / 0.87 | 0.28 / 0.72 | 22.43 / 31.97 |
| ARW B1 | 2.57 / 4.38 | 8.97 / 12.95 | 7.14 / 17.26 | 0.72 / 1.06 | 0.18 / 0.46 | 18.75 / 32.54 |
| ARW A2 | 2.62 / 5.25 | 8.53 / 12.07 | 9.33 / 17.24 | 0.69 / 0.89 | 0.28 / 0.46 | 23.39 / 31.39 |
| ARW B2 | 2.58 / 3.07 | 9.33 / 13.72 | 7.68 / 16.98 | 0.72 / 0.98 | 0.18 / 0.31 | 21.58 / 31.63 |
| ARW A3 | 2.41 / 4.79 | 9.28 / 14.26 | 8.79 / 17.00 | 0.69 / 0.89 | 0.28 / 0.68 | 21.51 / 32.08 |
| ARW B3 | 2.66 / 4.69 | 9.13 / 11.84 | 9.08 / 16.76 | 0.70 / 1.04 | 0.17 / 0.46 | 21.96 / 31.24 |
| RAF A1 | 4.53 / 6.36 | 46.82 / 53.00 | 8.15 / 16.37 | 3.37 / 3.78 | 0.22 / 0.30 | 64.08 / 78.90 |
| RAF B1 | 4.69 / 6.63 | 46.08 / 53.63 | 9.93 / 15.64 | 3.39 / 3.89 | 0.15 / 0.20 | 65.47 / 76.33 |
| RAF A2 | 4.53 / 6.67 | 46.40 / 53.97 | 8.60 / 15.16 | 3.35 / 3.79 | 0.22 / 0.32 | 64.02 / 76.44 |
| RAF B2 | 4.66 / 6.82 | 45.86 / 53.29 | 8.98 / 16.19 | 3.35 / 3.88 | 0.15 / 0.22 | 65.67 / 79.00 |
| RAF A3 | 4.56 / 6.44 | 46.32 / 53.91 | 7.83 / 16.62 | 3.37 / 3.92 | 0.22 / 0.31 | 63.79 / 79.90 |
| RAF B3 | 4.72 / 6.94 | 46.44 / 53.41 | 9.13 / 15.38 | 3.40 / 4.07 | 0.15 / 0.21 | 64.80 / 76.12 |

`wait` and `orient` were about 0 at p50 in every run; `hop` was 0.02 on ARW
and 0.43 in RAF A1.

## Median of the three p50s, and A minus B (ms)

| Stage | ARW A | ARW B | ARW A - B | RAF A | RAF B | RAF A - B |
|-------|-------|-------|-----------|-------|-------|-----------|
| read | 2.62 | 2.58 | +0.04 | 4.53 | 4.69 | -0.16 |
| decode | 8.96 | 9.13 | -0.17 | 46.40 | 46.08 | +0.32 |
| frameWait | 8.79 | 7.68 | +1.11 | 8.15 | 9.13 | -0.98 |
| paint | 0.69 | 0.72 | -0.03 | 3.37 | 3.39 | -0.02 |
| present | 0.28 | 0.18 | +0.10 | 0.22 | 0.15 | +0.07 |
| **keypressToPixels** | **22.43** | **21.58** | **+0.85** | **64.02** | **65.47** | **-1.45** |

`keypressToPixels` per pair (A1-B1, A2-B2, A3-B3): ARW +3.68, +1.81, -0.45;
RAF -1.39, -1.65, -1.01.

### What the stages say

- `present` is the one stage that moved the same way in every run: A's p50 is
  0.10 ms higher on ARW (0.28 against 0.17-0.18) and 0.07 ms higher on RAF
  (0.22 against 0.15). The frame now carries the meta pane's `div` rows, the
  strip's marks and the overlay's layers; the pane alone was seen at about
  +0.2 ms in `present` in Step 2. At most about 0.1 ms, well within the gate.
- `paint` did not move (-0.03 / -0.02 ms): the preview canvas's own work is
  the same.
- `frameWait`, the vsync phase, carries the rest and changes sign between the
  folders (+1.11 ARW, -0.98 RAF). B's ARW runs alone range 18.75-21.96 ms in
  `keypressToPixels`, wider than the A - B, so the +0.85 ms and the -1.45 ms
  are inside the run-to-run spread.
- `read` and `decode` are within 0.35 ms on both folders.
- The absolute numbers are higher than the strip plan's session (ARW B 21.58
  here against 16.64 then; `read` 2.6 against 0.7-0.9, `decode` 9.0 against
  6.3) and back near the port's 2026-11-09 levels. Same files and machine; not
  explained, and not compared across sessions.
