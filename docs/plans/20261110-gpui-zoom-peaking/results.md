# Step 2: the held-key measurement of focus peaking

**Verdict: the gate is met on both folders.** The median of the three p50
`keypressToPixels` with peaking on, minus off, is **-1.75 ms on ARW** (gate
+2.0 ms) and **+1.11 ms on RAF** (gate +5.0 ms). The RAF figure rests on 4-6
shown pages per run (see "RAF pages under a held key"), so it is weak evidence,
not a tight bound.

## Conditions

- Build: the release build of `f35d4151` (`cargo build --release` in
  `crates/native` on a clean tree, NASM on `PATH`, Rust 1.98.1), run directly,
  no rebuild during the runs.
- Machine: Windows 11, i7-13700, RTX 4060 Ti, 5120x1440 at 59 Hz, scale 1.0.
  The window is the app's default 1600x1000; the preview canvas is 852 device
  px high, so the fit view draws a landscape 3:2 frame at 1275x852 (ARW) or
  1278x852 (RAF).
- Files: `D:\Photos\tests\20261108-gpui-compare\ARW` (`--size 1616x1080`) and
  `...\RAF` (`--size 4416x2944`), 20 files each, on the `D:` SATA SSD. The
  index is a fresh `D:\Photos\tests\20261110-gpui-zoom-peaking\measure.sqlite`,
  made by the first (throwaway) run of the session and warm after it.
- Driver: `D:\Photos\tests\20261110-gpui-zoom-peaking-tools\measure.ps1`
  (`drive.ps1` for the keys, `summarize.ps1` for the percentiles). The key hold
  is **injected**, not by hand: `keybd_event` key-downs of `VK_RIGHT` every
  33 ms (PowerShell `Start-Sleep`, so the spacing jitters by a few ms) for 5 s,
  then one key-up, with the window brought to the foreground and checked before
  each event. 102-107 key-downs per run reached the window; none was skipped.
- One run: launch, 3 s settle, `e` and 1.5 s when peaking is on (`z` and 1.5 s
  more for the zoomed run), `→` held 5 s, 1.5 s, `q`. The percentiles are the
  `page` lines of that run, so the press that opened the folder and the `e` /
  `z` presses are not in them.
- Order per folder: throwaway (off), off, on, off, on, off, on, then one
  zoomed run with peaking on. A run that missed the idle gate was redone in its
  slot.
- Percentiles: p50 = sorted index `n / 2`, p95 = index `floor(n * 0.95)`, the
  native summary's rule.

### Idle

Before each run and every 3 s during it, `Win32_Processor.LoadPercentage` and
any `cargo` / `rustc` / `link` / `riffle-app` / `handpic*` process were
recorded. A run was used when the sample before it was <= 15 %, every sample
during it after the first was <= 15 %, and no such process was seen.

- First session (23:07-23:17 local, 2026-11-12): other sessions started
  building (`cargo`, `rustc`, `link`) during the ARW runs. ARW's off3, on2 and
  on3 slots failed all three attempts, so **the whole ARW set was run again**
  as a second session (23:18-23:21), every run of which passed on its first
  attempt except the zoomed run (23 % before it; its second attempt passed).
  The ARW figures below are that second session's. The RAF runs of the first
  session passed (on1 and on3 on their second attempt, 20 % and 33 % in the
  first), and no compiler was seen during them.
- Raw logs, `runs.csv` (the samples) and `summary.csv`:
  `D:\Photos\tests\20261110-gpui-zoom-peaking-tools\runs-20261112\` (first
  session; failed attempts end in `.log.failed`) and `...\runs-20261112-arw2\`
  (ARW, second session).

## ARW (n = 103-106 pages per run)

| Run | keypressToPixels p50 / p95 | read p50 | decode p50 / p95 | frameWait p50 | paint p50 | present p50 |
|-----|-----|-----|-----|-----|-----|-----|
| off1 | 19.12 / 28.05 | 0.96 | 7.35 / 10.34 | 8.76 | 0.69 | 0.27 |
| on1 | 16.59 / 23.93 | 0.60 | 6.02 / 8.06 | 8.05 | 0.70 | 0.25 |
| off2 | 18.55 / 28.47 | 0.98 | 7.27 / 10.20 | 8.85 | 0.68 | 0.27 |
| on2 | 16.96 / 25.44 | 0.63 | 6.12 / 8.06 | 9.43 | 0.66 | 0.24 |
| off3 | 18.44 / 28.79 | 0.99 | 7.52 / 11.41 | 8.66 | 0.67 | 0.28 |
| on3 | 16.80 / 24.37 | 0.63 | 6.01 / 8.28 | 8.99 | 0.68 | 0.24 |

`wait`, `orient` and `hop` p50 were 0.00 / 0.00 / 0.02 ms in every run.

- Median p50 `keypressToPixels`: off 18.55 ms, on 16.80 ms, **-1.75 ms**.
- The `peaking` line (fit view, 1275x852): `resize` p50 22.43 / 22.89 /
  22.76 ms, p95 23.14-25.38 ms; `compute` p50 5.33-5.39 ms, p95 6.47-6.63 ms.
  One overlay per shown page (105-107 lines against 104-106 pages, the extra
  one the file shown before the hold).
- Peaking on was faster in all three pairs, through `read` (about -0.35 ms)
  and `decode` (about -1.3 ms). This was not investigated; the likely cause is
  that the overlay job keeps a core busy between pages, so the CPU stays at a
  higher clock for the next decode (Inferred). The gate only bounds the cost,
  so nothing depends on it.

## RAF (n = 4-6 pages per run)

| Run | keypressToPixels p50 / p95 | read p50 | decode p50 / p95 | wait p50 | frameWait p50 | paint p50 |
|-----|-----|-----|-----|-----|-----|-----|
| off1 | 52.40 / 94.94 | 1.38 | 39.03 / 52.55 | 2.17 | 2.50 | 4.66 |
| on1 | 53.81 / 112.10 | 1.47 | 38.98 / 49.13 | 2.32 | 6.24 | 5.18 |
| off2 | 67.86 / 79.04 | 1.62 | 43.70 / 49.84 | 5.83 | 6.26 | 5.35 |
| on2 | 60.98 / 71.84 | 1.61 | 38.66 / 42.01 | 0.85 | 5.03 | 4.75 |
| off3 | 52.70 / 66.17 | 1.35 | 41.93 / 44.23 | 2.06 | 3.71 | 4.75 |
| on3 | 53.01 / 93.78 | 1.37 | 39.51 / 57.43 | 2.28 | 4.91 | 4.71 |

- Median p50 `keypressToPixels`: off 52.70 ms, on 53.81 ms, **+1.11 ms**.
- The `peaking` line (fit view, 1278x852 from 4416x2944): `resize` p50
  47.17 / 50.01 / 53.61 ms, p95 49.41-67.32 ms; `compute` p50 4.98-5.20 ms,
  p95 5.51-6.85 ms.

### RAF pages under a held key

Only 4-6 of about 105 key-downs per run ended in a shown page, with peaking
off as well as on. A RAF preview takes about 40-45 ms to read and decode,
longer than the 33 ms between key-downs, so when it arrives the pager has
already moved on and reads the newer file (`Pager::finish`); a page shows only
when a gap between key-downs happens to be longer than a load. So under a held
key the RAF folder barely draws, and with peaking on there are as few overlay
jobs as shown pages: the measurement cannot show much competition between the
overlay and the next decode on RAF. That pager behaviour predates this plan and
is recorded as a todo candidate in `learnings.md`.

## The zoomed runs (recorded, not gated)

`z` then a held `→` with peaking on; each page reads the full JPEG for the
crop as well as the preview. The `zoom` line is printed only for the crop the
`z` press asked for.

| Folder | n | keypressToPixels p50 / p95 | decode p50 | paint p50 | `zoom` line | crop `compute` p50 / p95 |
|-----|-----|-----|-----|-----|-----|-----|
| ARW | 106 | 16.69 / 24.91 | 6.26 | 1.02 | read 1.86, decode 8.24, orient 0.00, keypressToPixels 25.83 ms | 4.65 / 6.24 ms |
| RAF | 7 | 55.23 / 76.65 | 38.15 | 4.81 | read 6.09, decode 22.80, orient 0.00, keypressToPixels 42.71 ms | 4.25 / 5.26 ms |

- The crop overlays need no resize (`resize` 0.00 ms); they are 1024-1039 px
  wide (the crop grows to whole MCUs) and 852-857 high.
- RAF printed 108 crop overlays against 7 shown previews: the crop read
  (about 30 ms, under the 33 ms between key-downs) mostly lands while its page
  is still current, so the crops and their overlays kept arriving under the
  held key while the preview, slower than the key repeat, did not.
- In the first session's ARW zoomed attempt (failed by the idle gate anyway)
  the `z` press left no trace (no `zoom` line, fit-sized overlays only), although
  the driver reported it delivered. Not reproduced: the second session's zoomed
  run showed the crop.
