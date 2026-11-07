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

## Step 2

- The plan's "the listener only sets the text" is not enough: nothing orders
  the last `trash-progress` against the invoke's resolve (the event goes through
  the webview's eval, the response through IPC), and a late event after
  `trashing = null` would leave the line up and the guard shut for good. The
  listener drops events while `trashing` is `null`.
- The plan's "context-menu item's guard (~line 1123)" did not exist: the strip's
  menu had no disabled state, and the only gate was `confirmTrashFiles`. The
  manual check wants the item greyed, so `MenuItem` gained `disabled`,
  `contextMenuGroups` a `trashing` argument, and `style.css` `.menu-item:disabled`.
- Beyond the plan's list, to make its "never two trash-like invokes in flight"
  true: `redoTrash`'s closure also returns while `trashing !== null` (Redo during
  a move would otherwise queue on `Scans` and overwrite the flag), and so does
  `trashFilesIn`'s (a Redo in flight when a confirmed move starts).
- `trashing` is cleared in the invoke's `.then` / `.catch`, not inside `done()`
  as the plan words it: `done()` also runs when the guards drop the move, and
  would then clear the flag of another run in flight.
- `progressStatus` uses a `Record<TrashOp, string>`, so Step 5 adds one row and
  one union member.
- Beyond the plan's docs list: `docs/humans/usage.md` / `usage.ja.md` (the status
  count and the greyed item) and `docs/agents/ui-styling.md` (the disabled item).

## Step 3

- `cancel_scan` waiting on the lock, checked by reading: `trash_files` starts
  only with `scanning()` false, and a `start_scan` needs the lock too, so while
  the move runs `running` is empty or a scan in its third pass. When the waiting
  `cancel_scan` gets the lock, its id is stale or in that pass, and `cancel`
  ignores both. It parks a blocking-pool thread, not a runtime worker.
  `set_scan_focus`'s `try_lock` is untouched.
- Wrapping only the first block is enough for `scan_folder`: its later locks
  (the `pending` insert, `Preparing`'s drop) run with `preparing > 0`, so every
  long holder refuses instead of holding. The exception is `spawn_eviction`,
  which checks `running` alone: its startup `VACUUM` can still make them wait,
  as before.
- `let scans = app.state::<Scans>();` moved down to the `pending` insert, its
  only remaining use in the async body, so the closure's own binding shadows
  nothing.
- No new test: the block needs an `AppHandle`, and the plan asks for the existing
  scan tests to pass unchanged.

## Deferred issues (todo candidates)

- **Pending manual check** (Step 3, Windows; the step's checkbox was ticked on
  the automated criteria). Start a move of 200+ files and reopen the folder
  during it. Expected: the UI stays responsive (strip scroll, meta pane, other
  folders in the tree expand), and the `scan list` line appears right after
  `moved files to the trash` in `Riffle.log`. Basis: plan Step 3's manual check;
  file `crates/app/src/commands.rs` (`scan_folder`).
- Other async commands still take the `Scans` lock on a runtime worker for their
  `scanning()` guard: `rename_folder`, `rename_file`, `date_folder_preview`,
  `date_folder_run`, `rewrite_sidecars_preview`, `delete_sidecars_preview` and
  `clear_index`. Pressed during a move, each parks a worker until the move ends.
  They are single user actions, so few can pile up, but moving the guards into
  `spawn_blocking` would close the same hole Step 3 closed. Basis: Step 3
  implementation; files `crates/app/src/rename.rs`, `crates/app/src/datefolder.rs`,
  `crates/app/src/foldersidecars.rs`, `crates/app/src/commands.rs`.
- Two `scan_folder` calls waiting out one move (reopen, then open another folder)
  mint their ids in the order they get the lock, which `std::sync::Mutex` does not
  promise is the order they asked. Inverted, the frontend's newer call gets the
  stale id and its scan is dropped, and the older call's `pending` entry, which
  the frontend ignores, keeps `scanning()` true until the next `scan_folder`. Not
  introduced by Step 3 (the workers waited on the same mutex) and not reproduced;
  found by reading. Basis: Step 3 implementation; files
  `crates/app/src/commands.rs` (`scan_folder`, `start_scan`),
  `crates/app/ui/src/main.ts` (`startScan`).

- **Pending manual check** (Step 2, Windows; the step's checkbox was ticked on
  the automated criteria). Move 200+ files. Expected: the status counts up
  (`moving to Trash N / M`) and clears when the run ends; `Move to Trash…` is
  greyed in the strip's menu while it runs; reopening the folder during the move
  (File > Reload, or the tree) keeps the line and the rescan starts after the
  move; Redo shows the count too; Undo still restores. Basis: plan Step 2's
  manual check; files `crates/app/ui/src/main.ts`, `crates/app/src/commands.rs`.
  The strip's menu and the tree are only reachable in the held path (see the
  next item), so press Move to Trash during a scan for those two.
- A move that was not held keeps the confirm dialog open in its running phase
  until the invoke settles (the comment above `trashFilesIn` says this is
  deliberate), so the `moving to Trash N / M` line shows dimmed behind the
  `.dialog` overlay, the dialog still asks its question with disabled buttons,
  and the strip and tree are out of reach. Closing the dialog when the run starts,
  or showing the count in it, would make the progress plainer but gives the strip
  the keyboard back mid-move. Basis: Step 2 implementation; files
  `crates/app/ui/src/main.ts` (`trashFilesIn`, `closeTrashDialog`).
- `Delete Sidecars…` and Undo are not refused while `trashing !== null`: they
  queue on the `Scans` lock as before, and neither touches `trashing`. A long
  move can leave them waiting with nothing said about them. Basis: Step 2
  implementation; file `crates/app/ui/src/main.ts` (`undoTrash`, the sidecar flow).

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
