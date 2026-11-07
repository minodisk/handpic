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
- The `MAX_PAUSE` skew reaches past cap 1. On a disk-bound pass a worker's cycle
  is `threads * ms/file` whatever the cap, nearly all of it in `acquire` for a
  small cap: `(threads - cap) * ms/file`, 0.9 s at cap 1, 0.86 s at 2 and 0.77 s
  at 4 by the plan's 42.8 ms. Any slowdown from the shallower queue (about
  48 ms/file at cap 1) passes `MAX_PAUSE`, and the workers then read through a
  live pause. The test prints that estimate as `permit_wait_ms`; read a pause-on
  cell near 1000 as understating the pause, and compare it with the same cap's
  pause-off cell before ruling the cap out.
- `summarize` now returns `(n, median, p95)` so the `sweep-cell` line can print
  plain numbers; `embed_pass_on_a_real_folder` wraps it to keep its output.
- The app-alive check compares the whole process name with `riffle-app.exe`, so
  it does not apply off Windows, where the sweep does not run. Run against the
  live app, the test stopped at that check, before reading the folder. A run
  past the check has not happened yet: the first sweep cell is its smoke test.

## Deferred issues (todo candidates)

- **`MAX_PAUSE` counts from the start of `acquire`, including the wait for a
  permit.** Implemented as the plan's Implementation approach says ("this call
  has waited `MAX_PAUSE`"). A worker that queued for a permit for over 1 s then
  ignores a live pause. It cannot happen at Step 1's cap of `scan_threads()`,
  but by the plan's 42.8 ms per read the queue at a cap of 1 with 22 workers is
  about 0.9 s, so that cell of Step 2's sweep may read worse than the pause
  deserves. If so, start the clock when the worker first sees the pause and
  reset it when the pause is off. Basis: Step 1's `ReadSlots::acquire`,
  `crates/core/src/scan.rs`.
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
- **The slot and the pause cover `hevc::to_jpeg` for an HDR PQ CR3, not the
  read alone.** `read_preview` ends in the HEVC decode (65-125 ms of CPU per
  file), which runs inside `slots.acquire` in the three scan sites and inside
  `preview`'s pause. At Step 1's cap of `scan_threads()` this changes nothing,
  but `preview` on such a file pauses the scan for the decode too, and a small
  cap (1 or 2) would also limit concurrent HEVC decodes while permits sit idle
  at the disk. Fix: have a `reader` function return the undecoded
  `(arw, bytes, codec)` so the guard covers the bytes and `to_jpeg` runs
  outside it. Until then Step 2's done-when requires an HDR PQ CR3 run so the
  cap is not picked blind to it. Basis: review round 1 item 1,
  `crates/core/src/scan.rs`, `crates/core/src/reader.rs`.
