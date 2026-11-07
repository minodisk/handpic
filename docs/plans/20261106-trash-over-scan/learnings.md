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

## Step 4

- Measured first, on real-size copies: per-file about 200 ms a file, a chunk of
  100 about 7 ms, so 27-30x, and `CHUNK = 100`, Windows only. See
  `trash-bench.md`.
- The plan's 100-RAW set makes `chunk-100` and `whole` the same operation. A
  300-RAW set was added to tell them apart.
- The failure probe changed the design. The plan's "a chunk `Err` falls back to
  the per-file mover" read as if nothing had moved, but the shell goes on past a
  locked file and moves the rest. The plan's risks section says to skip such
  paths with `GONE`; they have to be recorded as moved instead, or Undo loses
  them. `GONE` is for a file missing before the chunk, which is checked first
  because one missing path fails `delete_all` before it moves anything.
- Not in the plan: `moved` must keep each RAW directly before its sidecars
  though the moves are batched, since `restore`'s `raw_back` and `redo`'s
  regrouping read that order. `run_chunked` rebuilds it per group.
- `run_chunked` sits next to `run` rather than changing `run`'s signature:
  `foldersidecars::delete`, macOS, Linux and Step 5's `delete_files` stay on
  `run` untouched. `redo` split out `redo_groups` for `redo_chunked`.
  `commands.rs` chooses by platform in `move_groups` / `move_again`.
- Measuring trap: the bin's size and item count came back exact while 23,450
  `$I` files sat orphaned in `$Recycle.Bin`, since `restore_all` leaves them and
  the shell does not list them. Only a 19 MB free space gap showed something was
  off. Counting the folder found them; they were removed by the original path
  each records. See the safety record in `trash-bench.md`.
- Beyond the plan's list: `docs/humans/usage.md` / `usage.ja.md`, as the count
  now goes in steps on Windows, plus `docs/agents/tauri-app.md` and `CLAUDE.md`.

## Step 5

- `delete_files` calls `trash::delete_for_good` (`groups_of` + `run` + the
  `foreign` failures) instead of inlining them, the one piece beyond the plan's
  shape: the tests then run what the command runs, foreign refusal and
  `run_id: None` included, on temp folders and without an `AppHandle`.
  `remove_one` sits in `trash.rs` for the same reason.
- Further departures in `delete_for_good`, from the review: a `.xmp` shared with
  a RAW outside the selection is kept (deleting it would strip that RAW of its
  sidecar), and a `.xmp` shared by several selected RAWs is not in the groups
  but removed after `run` through the remover, only once every one of them is
  in `summary.moved` (a RAW that failed keeps its sidecar, and a fully
  successful delete reports no false failure). The dialog line's `.xmp` clause
  states the same rule.
- Enter and Space are swallowed by `event.key`, not `keyName`: `keyName` spells
  a modified key `shift+enter`, and such a key clicks a focused button too, so
  a match on `"enter"` would let it through.
- Beyond the plan, for "only a mouse click confirms": `runTrash` takes a click
  only with `detail` exactly 1, through `confirmsDelete`: `detail` 0 is a
  keyboard-made click, and `detail` 2 or more is the second click of a
  double-click on the menu item, which can land on the run button that appears
  under the cursor. Space clicks on keyup, and `preventDefault` on keydown stops
  that only because the engine then never arms the button; that could not be
  tried here (Riffle was not to be launched), so the guard does not rest on it.
- Beyond the plan: `trashed()` takes the mode too, or a failed delete would read
  `could not move to the Trash` in the error list.
- `crates/app/src/commands.rs`, `docs/agents/tauri-app.md` and this plan are CRLF
  in the working tree though the index holds LF (`.gitattributes` has `eol=lf`),
  so a multi-line match on `\n` misses them. `git add` normalizes them.
- Beyond the plan's docs list: `docs/agents/ui-styling.md` (the destructive
  button's users); in `usage.md` / `usage.ja.md` the "next to last" wording of
  `Rename…`, the list of what an import refuses and the view-only menu; in
  `README.md` / `README.ja.md` the shared `FOO.xmp` note, as a delete keeps
  it while a RAW outside the selection still uses it.
- First `mise run ci` failed in `context.test.ts`, which pins the menu's shape in
  two more places than the plan names: `shows the default keys` and the item
  count in `gives an empty shortcut for an unbound action`. Both updated.

## Deferred issues (todo candidates)

- **Pending manual check** (Step 5, Windows; the step's checkbox was ticked on
  the automated criteria, as Riffle was not to be launched for this step). In a
  test folder of copies under `D:\Photos\tests\<date>-delete-permanently\`,
  never the user's photos, delete a selection with sidecars. Expected: the files
  and sidecars are gone from the folder and absent from the Recycle Bin, free
  space on D: goes up by their size, `Edit > Undo` has no entry for it, the
  status shows `deleting N / M` then `Deleted N files`, the strip rescans and
  shows the rest. Enter and Space on the open dialog delete nothing, also after
  Tab to the run button and with Shift held; only a mouse click does. A delete
  pressed during a scan runs after the scan stops; a delete pressed, then
  another folder opened before it ran, deletes nothing. A double-click on
  `Delete Permanently…`, from a menu opened over the viewer so that the run
  button appears under the cursor, deletes nothing (`confirmsDelete` assumes the
  second click reaches `#trash-run` with `detail` 2 though its target changed;
  nothing has run that on WebView2). With `a.ARW`, `a.DNG` and `a.xmp` in the
  folder, deleting `a.ARW` alone keeps `a.xmp`, and selecting both RAWs removes
  it. Basis: plan Step 5's
  manual check; files `crates/app/ui/src/main.ts` (`confirmTrashFiles`,
  `trashKeydown`, `runTrash`, `trashFilesIn`), `crates/app/src/commands.rs`
  (`delete_files`).
- A read-only file (a camera's protect flag) goes to the Trash but may fail
  `remove_file`, depending on the platform; it is then listed as a failure and
  stays, which is safe, but the user has no way to delete it from Riffle. Not
  tried. Basis: Step 5 implementation; file `crates/app/src/trash.rs`
  (`remove_one`).

- **Pending manual check** (Step 4, Windows; the step's checkbox was ticked on
  the automated criteria, as Riffle was not to be launched for this step). In a
  test folder of copies, move 500+ files. Expected: `moved files to the trash`
  arrives in `Riffle.log` at about 7 ms a file, near `trash-bench.md`'s numbers
  (a large gap means the app adds a cost the bench did not have); the status
  count goes up in steps of 100; Undo restores all, each sidecar with its RAW;
  Redo moves them again. Basis: plan Step 4's manual check; files
  `crates/app/src/trash.rs` (`run_chunked`), `crates/app/src/commands.rs`
  (`move_groups`, `move_again`).
- A locked file in a real chunk is only covered by the fake mover and the bench
  probe. Worth trying once by hand: hold one file open, move a selection around
  it, and expect that file listed as a failure with its sidecars left, and the
  rest moved and restored by Undo. Basis: Step 4 failure probe; file
  `crates/app/src/trash.rs` (`move_chunk`).
- Every restore leaves an orphaned `$I` file in `$Recycle.Bin`: trash 5.2.9's
  Windows `restore_all` moves the `$R` file back and nothing removes its `$I`.
  Riffle's Undo restores through the same call, so it presumably does this for
  every file it brings back. D: already held 5327 older orphans naming other
  paths (their source was not traced; they were left alone). Each is small and
  the shell does not list them, but they pile up and cost a file record each.
  Check whether Explorer's own restore cleans up, and whether the crate can.
  Basis: Step 4 measurement; `trash-bench.md`; file `crates/app/src/commands.rs`
  (`restore_run`).
- `trash_rejected_undo` is still per-item, one shell operation each. Not timed,
  but if an operation costs what a move does (about 200 ms), an Undo of a large
  move now takes far longer than the move.
  One `restore_all` for the whole run took 1 s for 150 files in the bench, but
  `trash::restore`'s per-file rule (a sidecar waits on its RAW, never overwrite)
  would need the same treatment `run_chunked` gave `run`. Basis: Step 4
  measurement; files `crates/app/src/trash.rs` (`restore`),
  `crates/app/src/commands.rs` (`restore_run`).

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
