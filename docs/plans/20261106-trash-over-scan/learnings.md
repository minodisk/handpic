# Learnings

## Step 1

- Flow read for the PR: `faces-done` -> `setScanRunning(false)` ->
  `idle.drain()` runs the trash (`settleIdle` marks `idle.inFlight`) ->
  `drainResync()` re-defers the pending `scan-canceled` rescan -> `trashed()`'s
  `resync("trash")` collapses into it (`??=` keeps the first trigger) -> the
  `.finally` of `settleIdle` drains one rescan. A held trash replaced by a
  later `request` or dropped by its guards leaves the pending rescan to drain
  after that operation; `openDirectory` clears `resyncPending` and scans anyway.
- Status after a canceled scan, by reading: the task always emits `scan-done`
  (partial total) before `faces-done`, so `faces-done after scan-done` cannot
  fire; `run_faces_pass` returns zero at once on a set flag and `embed_work`
  returns `None`, so `finish` runs before `faces-done` and `trash_files`'s
  `scanning()` guard passes. `start_scan once per scan` is untouched since
  `start_scan` did not change.
- A cancel landing between `embed_work`'s check and `start_embedding` reaches
  the third pass with the flag set; that pass stops at once and is harmless.
- `cancelScan()` in `main.ts` takes no argument and reads `scanId`, as
  `sendScanFocus` does, so the request-time call and the `startScan` hook share
  it.
- Beyond the plan's docs list: the **Move to Trash…** sentence in
  `docs/humans/usage.md` / `usage.ja.md` said the move waits for the scan to
  end, which this step made stale, so both were updated.

## Deferred issues (todo candidates)

- **Pending manual check** (Step 1, Windows; the step's checkbox was ticked on
  the automated criteria). Open a large folder cold (or after Clear Cache),
  filter to non-picked, select all, and press Move to Trash during
  `scanning N / M`, then again in another run during `analyzing N / M`.
  Expected: the move starts within seconds; `Riffle.log` (Timing logs on) shows
  `scan extract ... canceled=true` or `scan faces ... canceled=true`, then the
  trash's rescan with `scan reconcile ... removed=K` and a smaller `todo`, and
  no `faces-done after scan-done` or `start_scan once per scan` invariant line;
  Undo restores the files and the next rescan picks them up. Also press Move to
  Trash during a scan, then rename a file before the move ran: the rename runs
  and the folder still finishes its scan. Basis: plan Step 1's manual check;
  files `crates/app/src/commands.rs`, `crates/app/ui/src/main.ts`.
