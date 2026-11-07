<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Move to Trash: stop the scan, show the move, move faster, and Delete Permanently

## Purpose

The user frees disk space by filtering to the non-picked files, selecting
all and choosing the strip's `Move to Trash…`. Three things make that slow
or opaque today, seen in a real session (Windows 11, 2677 ARW, `aiEnabled`
off):

1. While a scan runs (`scanning N / M` or `analyzing N / M`), the move is held
   by the idle gate (`idle.ts`) until `faces-done`, minutes of work on files
   that are about to be thrown away.
2. The move itself ran for over 18 minutes at 3-5 RAWs per second (each with
   its sidecars) with no status text at all. A reopen of the folder during
   it showed nothing either, because `scan_folder` waits for the `Scans`
   lock `trash_files` holds across the whole move. The user pressed `Move to
   Trash…` again; that press was held and, once drained, targeted files
   already gone (`moved=0 failed=1845` in `Riffle.log`).
3. Each file is one `::trash::delete` call, i.e. one shell operation.

And the Trash itself does not free space: a user whose disk is full wants
the files gone.

After this plan: a Move to Trash pressed during a scan cancels the scan
through a new `cancel_scan` command and runs as soon as the canceled scan
has ended, the folder is rescanned over the remaining files only (the
trashed ones are never scanned); the move streams `trash-progress` and the
status shows `moving to Trash N / M`, a second press is refused while one is
in flight, and a reopen mid-move keeps showing the move; `scan_folder` no
longer parks an async runtime worker on the lock; if a measurement on
real-size files shows `::trash::delete_all` in chunks to be clearly faster
on Windows, the move is chunked while keeping RAW-before-sidecars, per-file
failures, the `Trashed` records and Undo / Redo; and the strip's menu gets
`Delete Permanently…` next to `Move to Trash…`, which removes the files with
`std::fs::remove_file` after a plain-spoken confirmation, with no Undo.

The index needs no change: after a move or a delete, `Index::reconcile` on
the operation's own rescan drops the `files` and `embeddings` rows of the
gone files, which is all the passes, `folder_entries` and `rank_rows` read
from. Their `ratings` and `views` rows stay, as today, which is what lets
`trash_rejected_undo` bring a file back with its judgment and viewed state
(see "Trade-offs and risks").

## Steps

- [x] Step 1: Cancel the running scan when Move to Trash is held
  - Done when:
    - `ScansState::cancel(scan_id)` in `crates/app/src/commands.rs` sets the
      cancel flag of the scan in `running` when its id is `scan_id` and it
      is not in its third pass (`in_embed_pass()`); a stale or unknown id and
      a scan in its embed pass are ignored (that pass has `stop_embedding`).
      `#[tauri::command] pub async fn cancel_scan(app, scan_id: u64)` takes
      the `Scans` lock inside `spawn_blocking` (like `start_scan`) and calls
      it; registered in `crates/app/src/main.rs` next to `set_scan_focus`.
      Rust tests next to
      `a_scan_in_its_embed_pass_is_not_scanning_and_stops_on_request` (with
      its `spawn_scan` helper): matching id sets the flag; stale id leaves
      it; embed pass left alone; `scanning()` stays true after the cancel
      until the task's own `finish`.
    - `crates/app/ui/src/idle.ts` exports `cancelsScan(label: string |
      null): boolean`, true for `"Move to Trash"` (Step 5 adds `"Delete
      Permanently"`), with a vitest case in `idle.test.ts`.
    - In `crates/app/ui/src/main.ts`, `trashFilesIn` cancels the running
      scan when `whenIdle("Move to Trash", ...)` returns held: invokes
      `cancel_scan` with the current `scanId` (skipped when `null`) and sets
      `resyncPending ??= "scan-canceled"` (new `RescanTrigger` value in
      `refresh.ts`, so `rescanLine` logs it), so the folder is rescanned off
      the canceled scan's `faces-done` whatever becomes of the held trash.
    - The preparing window is covered: `startScan`'s `.then`, after
      `start_scan` resolved (next to `sendScanFocus`), invokes `cancel_scan`
      with the new `scanId` when `cancelsScan(idle.waiting)`. The
      request-time call with a stale id (a trash pressed while `scan_folder`
      was in flight) is ignored by the backend's id check.
    - Verified by reading the flow and noted in the PR: `faces-done` ->
      `setScanRunning(false)` -> `idle.drain()` runs the trash (`settleIdle`
      marks `idle.inFlight`) -> `drainResync()` re-defers the pending rescan
      -> `trashed()`'s `resync("trash")` collapses into it (`??=` keeps the
      first trigger) -> one rescan when the invoke settles. A held trash
      replaced by a later `request` (rename, Undo / Redo, sidecar dialogs) or
      dropped still leaves the pending rescan to drain after that operation;
      a folder switch clears `resyncPending` in `openDirectory` and scans
      anyway.
    - Status after a canceled scan checked: `scan-done` with the partial
      total through `refreshOnScanDone`, `faces-done` through
      `refreshOnFacesDone`; no `faces-done after scan-done` or `start_scan
      once per scan` invariant line in `Riffle.log`.
    - Tests: the Rust tests above; vitest for `cancelsScan` and the
      `RescanTrigger` addition (`refresh.test.ts`).
    - Docs: `docs/agents/tauri-app.md` entry near the `IdleGate` / `resync`
      entries (~line 1776): Move to Trash is the held operation that cancels
      the scan, why it still waits for `faces-done` (`trash_files`'s
      `scanning()` guard), the `resyncPending ??= "scan-canceled"` rule and
      the `startScan` hook. `CLAUDE.md` Layout: `cancel_scan` in the
      `src/commands.rs` list, the note on `src/idle.ts`. The comment above
      `trashFilesIn` ("once no scan runs") updated.
    - Manual check on Windows: open a large folder cold (or after Clear
      Cache), filter to non-picked, select all, Move to Trash during
      `scanning N / M` and again during `analyzing N / M`; the move starts
      within seconds; `Riffle.log` (Timing logs on) shows `scan extract ...
      canceled=true` or `scan faces ... canceled=true`, then the trash's
      rescan with `scan reconcile ... removed=K` and a smaller `todo`; Undo
      restores the files and the next rescan picks them up; press Move to
      Trash during a scan, then rename a file before the move ran: the
      rename runs and the folder still finishes its scan.
    - `mise run ci` passes.
  - Implementation approach:
    - Model `cancel` on `stop_embedding` (~1192) with the id check of
      `set_focus` (~1216). No change to `scan_folder`, `start_scan` or the
      passes: `run_scan` abandons the files in flight and emits `scan-done`
      with the partial total, `run_faces_pass` returns at once,
      `embed_work` returns `None`, so the task calls `finish(scan_id)` and
      emits `faces-done` and an empty `embed-done`. Do not cancel `pending`
      entries: the frontend calls after `start_scan` resolved.
    - Only the labels `cancelsScan` lists trigger the cancel; `holdRename`,
      Undo / Redo (~2559 / 2605) and the sidecar dialogs (~1304 / 1331) are
      untouched.
    - `trash_files` keeps its `scanning()` refusal and its `Scans` lock
      pattern (`docs/agents/tauri-app.md`, "An index/scan mutation that must
      not race a rescan...").

- [x] Step 2: Stream the move's progress and refuse a second Move to Trash while one is in flight
  - Done when:
    - `trash::run` (`crates/app/src/trash.rs:284`) takes a
      `progress: impl FnMut(usize, usize)` called after each group with
      (groups done, groups total), counting a failed RAW as done; `trash::redo`
      passes it through; `foldersidecars::delete` passes a no-op. A unit test
      in `trash.rs`'s `tests` records the calls of a run with a failure.
    - `trash_files` emits `trash-progress { op: "trash", dir, done, total }`
      (a `serde::Serialize` struct in `commands.rs`, modelled on
      `sequence.rs`'s `Progress`; `op` is for Step 5's `"delete"`) from that
      callback, and `trash_rejected_redo` emits the same (`dir` empty);
      `trash_rejected_undo` is unchanged (its `restore` loop has no
      callback; noted as a follow-up in the doc comment).
    - Frontend: `trash.ts` exports `progressStatus(op, done, total)`
      returning `moving to Trash ${done} / ${total}` for `"trash"`
      (mirroring `sequence.ts`, vitest case). `main.ts` keeps `let trashing:
      string | null` next to `sequencing` / `importing`, rendered by
      `renderMeta` the same way; the `trash-progress` listener sets it; the
      held closure of `trashFilesIn` sets it to `progressStatus("trash", 0,
      paths.length)` when its invoke starts and `null` when it settles (the
      `done()` path), as does `redoTrash` around its invoke.
    - A second Move to Trash is refused while one is in flight: the
      context-menu item's guard (~line 1123, next to `trashFlow.busy`) and
      `confirmTrashFiles` (~928) also check `trashing !== null`, so the press
      neither opens the dialog nor gets held. A held trash is still
      replaced by a later request as before.
    - A reopen of the same or another folder mid-move keeps the
      `moving to Trash N / M` line (the flag is module-level, not tied to
      the folder); `trashed()`'s `refresh` guard already handles a different
      `openDir`.
    - Docs: `docs/agents/tauri-app.md` entry on `trash-progress`, the
      `trashing` flag as the second-press guard (and why `trashFlow` is not
      enough in the held path: `closeTrashDialog()` ends the flow at once
      when the trash is held); `CLAUDE.md` Layout for `trash_files` /
      `src/trash.rs` if the sentence would otherwise be stale.
    - Manual check on Windows: move 200+ files; the status counts up and
      clears when the run ends; `Move to Trash…` is greyed in the strip's
      menu while it runs; reopen the folder during the move (File > Reload
      or the tree): the line stays, the rescan starts after the move; Redo
      shows the count too; Undo still restores.
    - `mise run ci` passes.
  - Implementation approach:
    - The event needs no run id: the frontend never has two trash-like
      invokes in flight (the `Scans` lock serializes them on the backend;
      the `trashing` guard on the frontend), so the listener only sets the
      text.
    - Keep the per-file move as is (speed is Step 4).

- [x] Step 3: Keep holding `Scans` across the move, but stop `scan_folder` from parking a runtime worker on it
  - Done when:
    - Decision recorded in `docs/agents/tauri-app.md`: `trash_files` (and
      the undo / redo, and Step 5's delete) keep holding the `Scans` lock
      across the whole operation, so no scan can start and index a
      half-moved folder and the operation's own rescan cannot race it; a
      `scan_folder` issued meanwhile waits, which is acceptable now that the
      status shows the move (Step 2).
    - `scan_folder`'s first block (mint the id, take `running`, start
      `Preparing`) runs inside `tauri::async_runtime::spawn_blocking`, so a
      long lock holder no longer blocks a tokio worker (the "Synchronous
      commands run on the main thread" / `set_scan_focus` `try_lock` rules
      in `docs/agents/tauri-app.md`). `Preparing` (holds an `AppHandle`,
      decrements on drop) is returned from the blocking task; the rest of
      `scan_folder` is unchanged.
    - Checked and noted in the PR: `cancel_scan` (Step 1) waiting on the
      lock in `spawn_blocking` is harmless: while the trash holds the lock
      no scan runs, and the id it would cancel is stale by the time it gets
      the lock, so it is ignored. `set_scan_focus`'s `try_lock` keeps
      dropping focus updates during the move, as documented.
    - Existing `commands.rs` scan tests pass unchanged; `mise run ci`
      passes.
    - Manual check on Windows: start a move of 200+ files, reopen the
      folder during it; the UI stays responsive (strip scroll, meta pane,
      other folders in the tree expand), the `scan list` line appears right
      after `moved files to the trash` in `Riffle.log`.
  - Implementation approach:
    - Smallest change: wrap only the lock-taking block; do not restructure
      the rest of `scan_folder`. If moving `Preparing` across the blocking
      boundary proves awkward, mint the id and take `running` in the
      blocking task and start `Preparing` under the same lock there.

- [ ] Step 4: Measure `::trash::delete_all` in chunks against per-file `::trash::delete` on Windows, and chunk the move only if it is clearly faster
  - Done when:
    - A measurement note `trash-bench.md` in this plan folder: method
      (files, sizes, chunk sizes, repetitions, timings per file), the
      numbers, and the decision. The files are copies in a manual test
      folder under `D:\Photos\tests\<date>-trash-bench\` (never the user's
      photos), sized like real ARWs (25-60 MB each), with `.xmp` sidecars
      for half of them. D: has about 12 GB free, so keep the set small
      (for instance 100 RAWs) and restore from the Recycle Bin or re-copy
      between repetitions; note the free space in the record. Compare
      per-file `delete` against `delete_all` with chunks of about 20, 100
      and the whole set, at least 3 repetitions each, on D:.
    - If chunking is not clearly faster (say under 2x per file): the note
      says so, nothing else changes, the step ends here.
    - If it is clearly faster: `trash::run` on Windows (or non-macOS, the
      decision recorded) moves in chunks of the size the measurement picked
      (a `const CHUNK` with the number from the note): the RAWs of up to
      `CHUNK` groups in one `delete_all`, then their sidecars in one
      `delete_all`, so RAW-before-sidecars holds at chunk granularity and
      a chunk whose RAWs failed leaves its sidecars alone. A chunk `Err`
      falls back to the per-file mover for that chunk, so every `Failure`
      names the file that failed. `Trashed` records are built per path as
      now (`trashed_at: None` on Windows / Linux, so `trash_key` lookups,
      `Runs`, Undo and Redo are untouched); the macOS `trash_file` per-file
      path stays. The progress callback of Step 2 fires per chunk with the
      groups completed. Unit tests in `trash.rs` with a fake chunk mover:
      order, per-file fallback on a chunk failure, sidecars held back when
      the RAW chunk failed, `redo` through the same path.
    - `mise run ci` passes; manual check on Windows: move 500+ files,
      timing in `Riffle.log` (`moved files to the trash` after the
      `trash_files` start) compared with the note; Undo restores all; Redo
      moves them again.
  - Implementation approach:
    - The mover signature changes from `FnMut(&Path)` to one that takes a
      slice (or `run` gains a second mover `move_many: FnMut(&[PathBuf]) ->
      Result<(), String>` next to the per-file one). Choose the smaller
      change; `redo`, `foldersidecars::delete` and Step 5's `delete_files`
      (per-file only, `move_many` absent or `Err`) follow.
    - Measure before touching code; the plan folder note is the step's
      artifact either way.

- [ ] Step 5: Add `Delete Permanently…` to the strip's menu
  - Done when:
    - Backend: `#[tauri::command] pub async fn delete_files(app, dir,
      paths) -> Result<trash::Summary, String>` in `crates/app/src/commands.rs`
      next to `trash_files`, with the same shape: drain the sidecar writer,
      take the `Scans` lock in `spawn_blocking`, refuse while `scanning()`,
      list the folder's sidecars, `trash::groups_of` (a path outside `dir`
      lands in `foreign`, reported as a `Failure`, never touched), then
      `trash::run(groups, remove_one, progress)` where `remove_one` is
      `std::fs::remove_file(path).map(|()| None).map_err(|e| e.to_string())`
      (the RAW first, its sidecars only if it went; a JPEG alone). The
      `moved` records are dropped: no `Runs::record`, `summary.run_id` stays
      `None`. Progress is emitted as `trash-progress { op: "delete", ... }`.
      Logged as `deleted files permanently: {summary:?}`. Registered in
      `main.rs` next to `trash_files`. Not added to `shortcuts.rs`'s
      defaults and not reachable from `mcp.rs` (its "Nothing here can move
      a file to the trash or delete it" stays true).
    - Rust tests in `commands.rs`'s `tests` (or `trash.rs`'s) on temp
      files through `groups_of` + `run` with the real remover: RAW then
      sidecars gone in order, a RAW the remover fails on (a fake remover
      returning `Err` for it) keeps its sidecars on disk and is reported,
      a JPEG alone, a path outside `dir` refused and reported, `run_id`
      `None`.
    - Frontend, `crates/app/ui/src/context.ts`: the last section becomes
      `[["trashFiles", "Move to Trash…"], ["deleteFiles", "Delete
      Permanently…"]]`, the count naming extended (`Delete N Files
      Permanently…`), shown in view-only folders too; `context.test.ts`
      updated. `deleteFiles` has no binding, so its shortcut column is empty.
    - `crates/app/ui/src/trash.ts`: `TrashTarget` gains `mode: "trash" |
      "delete"`; new `dialogText(mode, count)` returning the title (`Move to
      Trash` / `Delete Permanently`), the line (`filesLine` for trash; for
      delete `Permanently delete N files and their sidecars? They are not
      moved to the Trash and this cannot be undone.`, singular for one) and
      the run button label (`Move to Trash` / `Delete Permanently`);
      `trashedStatus` says `Deleted N files` (plus failures) for the delete
      summary; `progressStatus("delete", ...)` gives `deleting N / M`.
      Vitest cases for each in `trash.test.ts`.
    - `main.ts`: the `deleteFiles` action opens the same `trash-dialog`
      through `confirmTrashFiles(paths, "delete")`, which sets the title,
      line and run button text from `dialogText`, keeps `.button.destructive`
      on the run button (`docs/agents/ui-styling.md`), and in delete mode
      focuses the **Cancel** button instead of the run button and makes
      `trashKeydown` `preventDefault()` `Enter` and `Space` (and any
      `event.repeat`), so only a mouse click on `Delete Permanently`
      confirms; `Escape` and `Cancel` close as today. `runTrash` passes the
      mode to `trashFilesIn(dir, paths, mode)`, which uses the label
      `"Delete Permanently"` for `whenIdle` (added to `cancelsScan`), invokes
      `delete_files`, sets `trashing` through `progressStatus(mode, ...)`,
      and calls `trashed(summary, [dir])` as today (`run_id === null` pushes
      no history entry, so no Undo / Redo). A held delete runs only when
      `dir === openDir && token === folderToken` and no other flow is busy,
      exactly `trashFilesIn`'s existing guard, and deletes only the
      confirmed `paths` (the backend's `groups_of` refuses a path outside
      `dir`). Step 2's `trashing !== null` guard refuses either operation
      while either runs.
    - Docs: `README.md` + `README.ja.md`, `docs/humans/usage.md` +
      `usage.ja.md` (the strip's menu description and the **Move to Trash…**
      section get a short **Delete Permanently…** paragraph: what it does,
      that it cannot be undone, that it waits for or stops the scan like
      Move to Trash), `CLAUDE.md` Layout (`delete_files` in the commands
      list, `src/context.ts` and `src/trash.ts` sentences),
      `docs/agents/tauri-app.md` (the delete shares the trash's guards and
      events; the dialog's no-Enter rule).
    - Manual check on Windows in a test folder under
      `D:\Photos\tests\<date>-delete-permanently\` with copied files (never
      the user's photos): delete a selection with sidecars; the files and
      sidecars are gone from the folder and absent from the Recycle Bin,
      free space on D: goes up by their size, `Edit > Undo` has no entry
      for it, the status shows `deleting N / M` then `Deleted N files`, the
      strip rescans and shows the rest; Enter on the open dialog does not
      delete; a delete pressed during a scan runs after the scan stops;
      a delete pressed, then another folder opened before it ran, deletes
      nothing.
    - `mise run ci` passes.
  - Implementation approach:
    - Depends on Steps 1 and 2 being merged (the `cancelsScan` predicate,
      the `trash-progress` event with `op`, the `trashing` guard). If Step
      4 adopted chunking, `delete_files` stays per-file.
    - Index: no direct delete; the delete's `trashed()` -> `resync("trash")`
      rescan drops the `files` and `embeddings` rows through `reconcile`
      as for a move. The `ratings` and `views` rows of the deleted files are
      left as they are (see "Trade-offs and risks").
    - Reuse over new code: no new dialog, no new flow class, no new Rust
      module; `trash::Summary` is reused with `run_id: None`.

## Trade-offs and risks

- Index rows of trashed files (decided: status quo). `Index::reconcile` on
  the trash's rescan drops their `files` and `embeddings` rows; `ratings`
  and `views` rows remain until eviction or Clear Cache. Nothing reads those
  leftovers: `folder_entries`, `faces_todo`, `embeddings_missing` and
  `rank_rows` start from `files`; `rank_dirs` may keep the folder listed
  through a leftover view / reject row, which only makes a refit read a
  folder with fewer files. A direct delete in `trash_files` would close only
  the sub-second window before the reconcile and would cost, on Undo, the
  `views` row (the restored file's "viewed and unflagged" negative signal and
  its eviction protection) and any index-only judgment (`dirty` rows the
  drained writer could not write, `label_known`), which the sidecar re-parse
  cannot restore; today both survive because `trash_rejected_undo` writes
  nothing and the rows are reused.
- Leftover rows after a permanent delete (Step 5): the same `ratings` /
  `views` rows remain, and here no Undo can ever reuse them. They are not
  removed, for the same reason: nothing reads them, they are a few hundred
  bytes per file, `evict_folder` drops the clean `ratings` rows when the
  folder is evicted, and a dirty one (a judgment whose sidecar write failed)
  would be the only row that outlives that, as it does for a file deleted
  by hand. Removing them would need a new `Index::forget_files(paths)`
  deleting from four tables under the `Scans` lock for no visible effect.
  If the user wants the index pristine after a delete, that helper is the
  place, and Clear Cache already does it wholesale.
- Holding `Scans` across the move (Step 3) versus releasing it between
  files: releasing would let a rescan index a half-moved folder and race the
  trash's own reconcile, against the documented rule; the cost of holding
  is a reopen that waits, now visibly. Not taken.
- Progress identity: no run id on `trash-progress`. If a future command
  runs two trash-like operations at once this breaks; the `Scans` lock
  makes that impossible today.
- Undo gets no progress (its `restore` loop is separate). A follow-up can
  add the same callback to `trash::restore`.
- Step 4 is conditional: the measurement may show per-file is as fast on
  the Recycle Bin, in which case only the note ships. If chunking lands,
  the risk is a chunk failure's fallback re-trying files already moved; the
  fallback must skip paths that no longer exist (`GONE`) rather than report
  them as failures. `delete_all` on Windows is one shell operation, so a
  cancel mid-chunk is not possible, which is fine since the move has no
  cancel today.
- Delete Permanently and the sidecar writer: like `trash_files`, the writer
  is drained first, so a judgment still in its debounce does not mint a
  sidecar for a file that is gone. A write that fails after the drain
  timeout leaves a dirty `ratings` row behind (the leftover case above).
- Pending-entry cancel (Step 1, not taken): `cancel` could also flag a
  `pending` entry; not needed with the `startScan` hook after `start_scan`.
- The `Move to Trash: waiting for the scan to finish` note stays as is; it
  is short-lived after Step 1. Step 5 shows `Delete Permanently: waiting for
  the scan to finish` the same way.

## Progress

- (2026-11-06) Step 1 complete
- (2026-11-07) Step 2 complete
