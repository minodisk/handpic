# Results: preview latency by stage on Windows

Measured by the user on 2026-11-08. Numbers come from the `page` lines of
`Riffle.log`, computed with an uncommitted script. Each cell is
`p50 / p95` in ms. The percentile is nearest-rank, so with n of 20 or less the
p95 is at or next to the largest sample, and with n below 20 it is the largest.
`wait` and `keypressToPixels` exist only on a keyed `page` line, so every table
counts keyed lines only.

Conditions common to every table: Windows 11, a
`mise run tauri:release:devtools` build (optimized), `D:` a SATA SSD, `Timing
logs` on and the performance overlay on. NVMe was not measured.

`keypressToPixels` ends when the canvas draw returns. WebView compositing and
input latency are not in any number below.

## Scenario 1: idle, warm, single presses

Folder, file count and body: not recorded with the log (the lines carry no
path); a folder scanned before, 1616x1080 previews of 253-319 KB. Scan pass:
idle on every keyed line. Warm: scanned earlier, not otherwise established.
20 keyed presses against the 30 planned; the 3 other lines of the run are not
keyed and are excluded.

| Case | n | keypressToPixels | wait | read | ipc | post | worker | decode | draw |
|---|---|---|---|---|---|---|---|---|---|
| Idle, warm, single presses | 20 | 19.4 / 22.6 | 0.9 / 1.5 | 6.0 / 9.0 | 4.0 / 5.8 | 0.1 / 0.2 | 0.3 / 0.8 | 6.0 / 6.7 | 1.0 / 2.0 |

## Scenario 2: idle, warm, key held

Same folder and conditions as scenario 1, 1616x1080 previews of 215-363 KB.
Scan pass: idle.

| Case | n | keypressToPixels | wait | read | ipc | post | worker | decode | draw |
|---|---|---|---|---|---|---|---|---|---|
| Idle, warm, key held | 146 | 20.8 / 26.5 | 1.3 / 1.8 | 5.8 / 6.4 | 4.9 / 11.7 | 0.1 / 0.2 | 0.2 / 1.7 | 6.2 / 6.6 | 0.8 / 1.8 |

## Scenario 3: during pass 1

Folder `D:\Photos\2026\10261103`, 3105 RAWs, 1752 to do, pass 1 took 11612 ms;
body not recorded, 1616x1080 previews of 247-357 KB. Scan pass: 1. Coldness was
not established beyond the folder never having been opened in the app: its
files may have been in the OS cache.

| Case | n | keypressToPixels | wait | read | ipc | post | worker | decode | draw |
|---|---|---|---|---|---|---|---|---|---|
| Pass 1, `10261103` (reference only, see below) | 3 | 138.2 / 151.5 | 1.2 / 71.2 | 123.0 / 139.5 | 4.3 / 5.3 | 0.2 / 0.2 | 0.5 / 0.5 | 5.7 / 7.9 | 0.4 / 0.5 |

**This is a reference value, not a p95.** Only 3 presses landed while pass 1
ran: `keypressToPixels` was 92.2, 138.2 and 151.5 ms, so the p50 is 138.2 and
the largest 151.5. With n=3 the "p95" column is simply the largest sample.
`read` was 10.9, 123.0 and 139.5 ms. The 92.2 ms press spent its time in `wait`
(71.2 ms) behind a request in flight, and its own `read` was 10.9 ms.

Two earlier attempts ended pass 1 before any keypress landed in it. The keyed
lines the three attempts left in other passes, for the record (`10161025` is
2199 RAWs, 299 to do, pass 1 2053 ms, pass 2 17598 ms; the first attempt's
folder was not noted; none of the three is established as cold):

| Case | n | keypressToPixels | wait | read | ipc | post | worker | decode | draw |
|---|---|---|---|---|---|---|---|---|---|
| First attempt, pass 2 | 27 | 34.5 / 46.0 | 4.0 / 16.8 | 2.7 / 4.5 | 10.6 / 17.1 | 0.1 / 0.2 | 1.8 / 5.2 | 10.4 / 13.9 | 2.8 / 4.6 |
| First attempt, idle after the scan | 66 | 14.5 / 24.4 | 1.4 / 3.5 | 1.0 / 1.8 | 4.7 / 14.7 | 0.0 / 0.1 | 0.3 / 1.1 | 4.6 / 5.3 | 0.8 / 1.5 |
| `10161025`, pass 2 | 30 | 33.7 / 39.2 | 3.2 / 5.9 | 2.9 / 5.2 | 12.0 / 19.0 | 0.2 / 0.3 | 0.6 / 4.0 | 10.3 / 16.1 | 1.5 / 2.9 |
| `10261103`, pass 2 | 4 | 31.8 / 42.0 | 2.6 / 4.6 | 2.6 / 3.0 | 11.1 / 22.2 | 0.1 / 0.3 | 0.8 / 5.5 | 9.5 / 13.3 | 1.4 / 1.7 |

## Scenario 4: during pass 2

Folder `D:\Photos\2026\10261103` (the folder of scenario 3), body not recorded,
1616x1080 previews of 258-341 KB. Scan pass: 2. Warm: the folder's files had
just been read by pass 1, not otherwise established. Pass 3 (embedding) was not
measured. 16 presses against the 20 planned.

| Case | n | keypressToPixels | wait | read | ipc | post | worker | decode | draw |
|---|---|---|---|---|---|---|---|---|---|
| Pass 2, `10261103` | 16 | 38.9 / 45.2 | 4.2 / 5.2 | 3.1 / 5.8 | 13.4 / 21.2 | 0.2 / 0.4 | 0.5 / 7.8 | 11.6 / 16.0 | 1.7 / 2.3 |

## Scenario 5: large previews

Folder `D:\photos\samples\RAF`, mixed bodies and sizes; 804-4320 KB on the idle
lines. Warm: scanned before the presses, not otherwise established. Scan pass:
idle in the first table, 2 in the second. The run also has 1 unkeyed `scan=1`
line, excluded.

| Case | n | keypressToPixels | wait | read | ipc | post | worker | decode | draw |
|---|---|---|---|---|---|---|---|---|---|
| RAF, idle, mixed sizes | 11 | 78.1 / 113.0 | 1.9 / 3.0 | 2.1 / 4.0 | 20.4 / 32.7 | 0.5 / 0.8 | 0.7 / 1.9 | 45.9 / 58.2 | 4.9 / 15.3 |
| RAF, idle, 4416x2944 | 7 | 97.8 / 113.0 | 1.9 / 3.0 | 3.0 / 4.0 | 28.0 / 32.7 | 0.7 / 0.8 | 0.7 / 1.9 | 55.2 / 58.2 | 6.0 / 15.3 |
| RAF, idle, 1920x1280 | 4 | 23.9 / 26.6 | 1.9 / 2.2 | 1.1 / 1.5 | 8.6 / 10.2 | 0.1 / 0.3 | 0.3 / 1.0 | 11.1 / 12.3 | 0.4 / 0.4 |

| Case | n | keypressToPixels | wait | read | ipc | post | worker | decode | draw |
|---|---|---|---|---|---|---|---|---|---|
| RAF, pass 2, mixed sizes | 8 | 41.6 / 72.7 | 2.4 / 3.6 | 2.6 / 4.2 | 16.3 / 29.2 | 0.4 / 0.7 | 0.7 / 2.7 | 16.9 / 30.6 | 1.4 / 3.3 |

The CR3 HDR PQ case (`hevc::to_jpeg`) was not measured.

## Scenario 6: cold, idle

Skipped.

## Decision

Rules in order, on the numbers above.

- **R0 fires.** Every target is met:
  - Scenario 1: p50 19.4 ms (target 50), p95 22.6 ms (target 100).
  - Scenario 2: p50 20.8 ms, p95 26.5 ms.
  - Scenario 4: p50 38.9 ms, p95 45.2 ms.
  - Scenario 5, RAF idle: mixed sizes p95 113.0 ms, 4416x2944 p95 113.0 ms
    (target 150).
  - Scenario 3 (target p95 250 ms): the n=3 reference is under it (largest
    151.5 ms), with `read` up to 139.5 ms, the disk queue the read-gate plan
    owns. Three samples cannot establish a p95, so this part of R0 rests on a
    reference value.
- **R1 does not fire.** Scenario 2's `wait` p50 is 1.3 ms, under 20 ms and 6 %
  of `keypressToPixels`, under 25 %.
- **R2 would fire on RAF, but R0 comes first.** On 4416x2944, `ipc + post +
  worker` p50 is 29.4 ms (`ipc` 28.0), over 15 ms. It does not fire on scenario
  1 (4.4 ms, 24 % of `total`). `post` on RAF is 0.7 ms, under 5 ms.
- **R3 would fire on RAF, but R0 comes first.** On 4416x2944, `decode` p50 is
  55.2 ms, over 30 ms. It does not fire on scenario 1 (6.0 ms, under 15 ms).
- **R4 does not fire.** Scenario 1's `draw` p95 is 2.0 ms, under 16 ms. The
  largest `draw` anywhere is 15.3 ms, on a 4416x2944 RAF preview, just under.
- **R5, native rendering.** Scenario 1's p95 of `ipc + post + worker + draw` is
  8.5 ms, and its `read` p95 is 9.0 ms. By the rule's letter the sum is under
  20 ms, so a rewrite could win no more than that, and R5 does not fire.
  The limitation: `keypressToPixels` ends at the canvas draw, so WebView
  compositing and input latency are not included, and 8.5 ms is therefore not
  a true ceiling on a native rewrite. **The native question stays open**: the
  user chose to test it directly with a separate throwaway GPUI spike outside
  this plan, and it stays open until that spike reports.

**Which steps run.** R0 alone would end the plan here. The user overrode it:
fast display is the app's top priority, so **Step 4 (prefetch) runs**. Its
expected gain is to remove `read`, `ipc` and `decode` on a hit. Steps 3 and 5
do not run: Step 3 because R1 did not fire, Step 5 because of R0, the user
having chosen prefetch only.
