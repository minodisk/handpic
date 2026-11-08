# Learnings: preview-read-gate

## Step 1

- `READ_SLOTS` cannot be a `const` while it equals `scan_threads()`, which
  calls `available_parallelism`, so Step 1 has a `read_slots()` function next
  to `scan_threads()`. Step 3 replaces it with the constant the plan names.
- `AppReadSlots` implements `Default` in `commands.rs` so `scan_threads()`
  stays private and `main.rs` manages it the way it manages `Scans`.
- `acquire` looks at `cancel` before it looks at the permits, as the plan's
  "returns `None` once `cancel` is set" says. That changes one old assertion:
  `a_file_canceled_mid_pipeline_is_not_delivered` expected a missing file under
  a set flag to be an `Err`, and such a file is now abandoned unread (`None`).
  The test now asserts both cases, the `Err` under an unset flag.
- `free` is unsigned, so a counter inside `ReadSlots` could not show the cap
  being exceeded. The `extract_all` half of proof (a) instead holds the one
  permit and shows that nothing is delivered until it drops, which proves the
  read needs a permit; the same test runs `extract_analysis_all`.

## Step 2

- `preview_during_pass_1_on_a_real_folder` holds every third file out of the
  pass, which the plan does not say: the first is the coldness check and the
  rest are the probe's, each read once. A probe on `(i * 37) % len`, as
  `embed_pass_on_a_real_folder` has it, soon lands on files the pass has read
  and reads them from the cache, hiding the very wait the sweep measures. So
  ms/file compares between the sweep's cells, not with the plan's 42.8 ms on a
  whole folder. The probe runs out on a pass slower than about 125 ms/file;
  `probes` equal to `probe_files` shows it.
- The probe sleeps before its read, so its first sample does not land on a disk
  the workers have not reached yet.
- `MAX_PAUSE` counted from the start of `acquire`, which would have biased
  every small cap, not just cap 1. On a disk-bound pass a worker waits about
  `(threads - cap) * ms/file` for a permit: 0.9 s at cap 1, 0.86 s at 2 and
  0.77 s at 4 by the plan's 42.8 ms, so about 48 ms/file at cap 1 would have
  let workers read through a live pause. Fixed before the sweep: the clock
  starts when `acquire` first sees a pause on and starts over once it sees none,
  and a wait for a permit alone uses none of it. A worker that misses a gap
  between two pauses keeps its clock, which only ends the wait sooner.
- `summarize` now returns `(n, median, p95)` so the `sweep-cell` line can print
  plain numbers; `embed_pass_on_a_real_folder` wraps it to keep its output.
- The app-alive check compares the whole process name with `riffle-app.exe`, so
  it does not apply off Windows, where the sweep does not run. Run against the
  live app, the test stopped at that check, before reading the folder.

### Sweep results (2026-11-08)

All 13 runs of `preview_during_pass_1_on_a_real_folder`, D: (SATA SSD), 22
threads, the app closed, no errors. Twelve are the `sweep-cell` lines of the
sweep's log. The 2026-06-13 row was supplied separately and its line carries no
`threads`, `errors` or `probe_files` fields. Folders are named as the log names
them. Times are ms unless the column says otherwise.

| Folder       | Cap | Pause | Files | Cold check | Pass s | ms/file | Probes | Read p50 | Read p95 | Read + decode p50 | Read + decode p95 |
| ------------ | --- | ----- | ----- | ---------- | ------ | ------- | ------ | -------- | -------- | ----------------- | ----------------- |
| 2026-06-20   | 22  | off   | 700   | 5.3        | 5.1    | 7.2     | 12     | 153.6    | 158.9    | 164.7             | 169.7             |
| 2026-06-05   | 22  | off   | 891   | 5.5        | 6.5    | 7.3     | 15     | 155.1    | 163.4    | 166.8             | 174.3             |
| 2026-06-13   | 22  | off   | 622   | 5.6        | 4.5    | 7.2     | 11     | 155.7    | 162.9    | 166.0             | 173.5             |
| 2026-10-12a  | 22  | on    | 2123  | 7.9        | 13.8   | 6.5     | 35     | 128.7    | 140.1    | 142.8             | 155.3             |
| 2026-09-13-a | 22  | on    | 414   | 5.5        | 2.7    | 6.6     | 7      | 128.2    | 136.7    | 140.1             | 154.2             |
| 10361103     | 8   | on    | 984   | 7.8        | 8.0    | 8.1     | 25     | 29.4     | 45.7     | 58.5              | 84.5              |
| 2026-08-01   | 8   | on    | 308   | 6.3        | 2.3    | 7.5     | 7      | 57.2     | 62.3     | 75.3              | 82.5              |
| 10061024     | 4   | off   | 936   | 6.8        | 7.5    | 8.0     | 24     | 27.1     | 38.2     | 54.2              | 76.2              |
| 2026-09-27-a | 4   | on    | 3220  | 6.0        | 25.6   | 8.0     | 84     | 20.4     | 29.5     | 48.2              | 59.5              |
| 2026-09-27-b | 4   | on    | 826   | 8.8        | 5.9    | 7.1     | 20     | 24.8     | 30.3     | 38.9              | 47.4              |
| 10261103     | 2   | on    | 2070  | 5.3        | 14.4   | 6.9     | 52     | 13.9     | 19.0     | 26.0              | 31.8              |
| 2026-07-11   | 2   | on    | 312   | 6.0        | 2.3    | 7.3     | 8      | 15.6     | 18.4     | 30.2              | 40.4              |
| 2026-07-05   | 1   | on    | 943   | 6.3        | 13.2   | 14.0    | 44     | 10.6     | 18.3     | 40.2              | 54.9              |

### Outcomes

- **The plan's baseline was inflated.** The 1323 ms / 42.8 ms/file baseline and
  the 330-570 ms cold read were measured while the user's debug app was running
  and reading D:. With the app closed the baseline is about 155 ms read p50 and
  7.2 ms/file. The queue model holds: the wait is about threads x ms/file
  (22 x 7 = 155). These ms/file cover the files the test leaves in the pass
  (see above), so they are not strictly the same measure as the 42.8 on a whole
  folder.
- **The cap gives almost all of the gain; the pause is a small part.** Read p50
  at cap 22 is 155 off against 128 on; at cap 4 it is 27 off against 20-25 on.
- **The picked cap is 2, chosen by the user with this table in hand.** Cap 2
  has a read p95 of 18-19 ms at 6.9-7.3 ms/file, unchanged from the baseline
  within noise. Cap 1 costs twice the scan time (14.0 ms/file, one run). Step 3
  sets the constant.
- **The plan's rule would not have picked it.** Every cap met the 250 ms p95
  threshold (the worst run is 163.4 ms), so "lowest ms/file among those" picks
  the uncapped 22 within noise, which improves nothing. The threshold was too
  loose.

### Deviations from the Done-when

- **The cold check of at least 100 ms was never met.** It read 5-9 ms in every
  run, because the threshold came from the inflated numbers. Coldness is
  inferred from cap 22's 130-155 ms wait.
- **The sweep ran out of folders.** Cap 4 pause-off and cap 1 have one run each
  and the planned cells at the end were not run. The table has no cap 2
  pause-off row, so the chosen cap lacks the pause-off run the Done-when asked
  for.
- **The smoke was not a sweep cell.** The smoke on `D:\photos\samples\CR3`
  panicked in the probe's decode of a sample, so it produced no row.
- **NVMe is untested.** No RAW folder was found on the NVMe. A shallower queue
  costs more on a drive that needs deep queues for throughput, so cap 2 may
  cost throughput on C: that this table cannot show.
- **The HDR PQ CR3 run was not done.** Step 1 holds the slot across
  `hevc::to_jpeg` (65-125 ms of CPU per file), so its cost at cap 2 is a
  calculated 33-63 ms/file against 3-6 uncapped. See the deferred item.

## Step 3

- `READ_SLOTS` is now a `const` of 2, replacing the `read_slots()` function.
- The doc comment and `docs/agents/tauri-app.md` keep cap 22 pause off (155 ms)
  and pause on (128 ms) apart, because cap 2 has only pause-on runs.

## Deferred issues (todo candidates)

- **Pending hand check of Step 3 (pending manual check).** This is Step 3's one
  remaining Done-when, so its checkbox stays unticked. On Windows, in the
  running release app, open an unopened folder under `D:\Photos\2026` and page
  with the arrow keys during its first scan. Expected: the preview follows the
  keys without the multi-second lag and the thumbnails keep filling while
  paging. Note anything odd (the strip pausing, a scan that seems to stall).
  Not performed; no result is recorded. Basis: Step 3's Done-when,
  `crates/app/src/commands.rs` (`READ_SLOTS`, `preview`).

- **`embed_all`'s slot has no test.** Its per-file closure needs a loaded SigLIP
  model, which the tests do not have, so its acquire is only checked by reading
  the code; `extract_all` and `extract_analysis_all` are both tested. Basis:
  Step 1's proofs (a) and (b), `crates/core/src/scan.rs`.
- **The first post-read cancel check lost its test.**
  `a_file_canceled_mid_pipeline_is_not_delivered` set the flag before the read
  and so reached `extract_unless`'s and `extract_analysis_unless`'s check after
  it. `acquire` now returns `None` first and nothing can set the flag during
  the read without a test hook in `read_preview`. Basis: Step 1's change to
  that test, `crates/core/src/scan.rs`.
- **(Filed in `todo.md` by Step 3.) The slot and the pause cover
  `hevc::to_jpeg` for an HDR PQ CR3, not the read alone.** `read_preview` ends in the HEVC decode (65-125 ms of CPU per
  file), which runs inside `slots.acquire` in the three scan sites and inside
  `preview`'s pause. At Step 1's cap of `scan_threads()` this changes nothing,
  but `preview` on such a file pauses the scan for the decode too, and a small
  cap (1 or 2) would also limit concurrent HEVC decodes while permits sit idle
  at the disk. Fix: have a `reader` function return the undecoded
  `(arw, bytes, codec)` so the guard covers the bytes and `to_jpeg` runs
  outside it. Until then Step 2's done-when requires an HDR PQ CR3 run so the
  cap is not picked blind to it. Basis: review round 1 item 1,
  `crates/core/src/scan.rs`, `crates/core/src/reader.rs`.
- **(Filed in `todo.md` by Step 3.) Split the read from `hevc::to_jpeg` so the
  slot covers the disk read only.**
  With the picked cap of 2 and no HDR PQ CR3 run in the sweep, a folder of such
  files would pay a calculated 33-63 ms/file for the decode (65-125 ms of CPU)
  inside the two slots, against 3-6 ms/file today. The split removes that cost;
  the preceding item describes its shape. Basis: Step 2's sweep and the user's
  pick of cap 2, `crates/core/src/scan.rs`, `crates/core/src/reader.rs`.
- **(Filed in `todo.md` by Step 3.) The plan's other follow-ups.** Prefetching
  the neighbours, not waiting on a stale preview request, pausing for the other
  read commands, and measuring the cap on NVMe. Basis: the plan's Purpose and
  Step 3's Done-when.
