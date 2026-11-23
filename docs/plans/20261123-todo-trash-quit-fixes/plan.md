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

# Todo: quit wait, held clear, sidecar redo, chunked undo, run progress

## Purpose

Close out five `todo.md` items with code changes and tests:

1. `### App: quitting while a re-listing is stopping the previous scan does not wait for that scan`
2. `### App: a held Clear Cache is not drained when a trash or folder dialog that was up at its idle point is canceled`
3. `### App: the trash undo, Rewrite Sidecars and Delete Sidecars runs have no progress callback`
4. `### Engine: a redo of Delete Sidecars… leaves a closed folder's rows judged`
5. `### Engine: Undo of Move to Trash is still one shell operation per item`

Once done: a quit during a re-listing no longer cuts the previous scan's write
transaction; a held Clear Cache goes on when the dialog that blocked it is
canceled; a redo of Delete Sidecars leaves no judged rows behind in a closed
folder; an Undo of a large Move to Trash on Windows takes seconds instead of
minutes (baseline: 85.6 s for 605 files, 141 ms a file); and the progress
circle shows the undo, Rewrite Sidecars and Delete Sidecars runs.

Scope rules for every step:

- Only these five items. No change beyond what each item's TODO asks.
- **No step edits `todo.md`.** Other sessions edit other sections of it
  concurrently. The five headings above (and only those) are deleted in the
  `develop` skill's wrap-up (`todo-curator`, 3.3) after the last step merges.
- Do not touch `docs/plans/20261123-all-faces-check`.
- `crates/engine` is in the root workspace; `crates/app` is a separate Cargo
  workspace (GPUI). Where a step changes behavior that the `CLAUDE.md` layout
  paragraph or `docs/agents/app.md` describes, update that sentence in the
  same PR.

## Steps

- [x] Step 1: The quit handler waits for a scan a re-listing is stopping (item 1)
  - Done when:
    - The app-level `on_app_quit` handler waits, bounded by `STOP_TIMEOUT`
      in total, for a scan that `list` took out of the holder and is still
      stopping in its background task, as well as for the held scan.
    - The outcome is logged on the `quit:` line.
    - A test in `crates/app/src/scan.rs` covers the holder: a scan retired
      by a re-listing is still waited for by the holder's bounded stop until
      the background stop returns, and is gone from the holder after it.
    - The paragraph of `docs/agents/app.md` on the quit stop (around
      "The same handler first stops the running scan", and the "Rule: a
      re-listing still stops the previous scan with the unbounded …" item
      after it) describes the new behavior.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Today `list` (`crates/app/src/main.rs`) does `self.scan.take()` on the
      UI thread and calls the unbounded `Scan::stop` in a `background_spawn`
      after awaiting the earlier listing's `settled`. `Running`
      (`crates/app/src/scan.rs`) is `Arc<Mutex<Option<Scan>>>`, and its
      `stop_within` returns true when it holds none.
    - Keep the retired scan, or a join signal for it, in the shared
      `Running` until its stop returns. `Scan` already has `ended`, a
      channel the thread disconnects when it ends (`JoinHandle` has no timed
      join); the background task needs the `JoinHandle`, the quit handler
      only the signal. The step decides the shape (for example splitting a
      retired scan into the part the background task joins and the part the
      holder waits on).
    - At most one scan is retired at a time: a later listing awaits the
      earlier one's `settled` before stopping its own previous scan, and
      takes the holder's scan at call time.
    - Set the retired scan's cancel flag when the quit handler runs, since
      the background task may not have reached its `stop` yet.
    - `STOP_TIMEOUT` bounds the whole wait (one deadline for the held scan
      and the retired one), not each.
    - Keep `quit: scan stopped=<bool> in <ms>ms` greppable and add a field
      for the retired scan to that line (decision 7).
    - `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped` is already
      listed in `todo.md` as flaky on a loaded machine. Do not add a test
      that depends on a tight real-time bound.

- [ ] Step 2: A held Clear Cache is drained when a blocking dialog is canceled (item 2)
  - Done when:
    - Canceling the trash dialog (Cancel / Escape) and canceling a folder
      dialog each call `drain_clear`, so a Clear Cache held at an idle point
      that came while the dialog was up goes on once the dialog closes.
    - A test covers it: a UI test (`crates/app/src/uitest.rs`, or beside the
      `Ui` tests in `crates/app/src/folderops.rs`) that holds a clear, lets
      the idle point pass with a dialog up, cancels the dialog and checks
      that the clear asks or runs; or, if that cannot be staged, a
      `ClearFlow::idle` test in `crates/app/src/cache.rs` for the guard plus
      the reason in `learnings.md`.
    - The `drain_clear` sentence of the Cache page rule in
      `docs/agents/app.md` (search for `drain_clear`; around line 2240 now,
      not 1886 as the todo item says) lists the dialog cancels among the
      idle points.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - `drain_clear` (`crates/app/src/main.rs`) returns early through
      `ClearFlow::idle(busy, up)` while `clear_busy()` or
      `trash.busy() || folderops.dialog_open()`. The guard makes an extra
      call harmless, so the call sites only need to cover every cancel.
    - The cancel paths are `dismiss_trash` (after `follow_trash`) and
      `dismiss_folder_dialog` in `crates/app/src/folderops.rs` (the branches
      that set `folderops.dialog = None`). Check whether the MCP
      `cancel_operation` path goes through the same functions.
    - Prefer the cancel paths over putting the call inside `follow_trash` /
      `follow_folder_dialog`, which are also called when a dialog opens and
      when a run ends (those ends already call `drain_clear`).
    - A pressed clear is dropped when the settings close, and
      `confirm_trash` refuses while the settings are open. So check which
      path reaches the state: probably an MCP-confirmed clear
      (`ClearFlow::confirmed`) held while a dialog is opened. The test
      follows the path that exists.

- [ ] Step 3: A redo of Delete Sidecars clears the judgments of a closed folder's rows (item 4)
  - Done when:
    - A recorded trash run remembers, for each sidecar a `Delete Sidecars…`
      run moved, the RAW it belongs to, through the undo (`Runs::undone`)
      and into the run a redo records, so undo and redo can alternate.
    - `trash::redo_run` clears the index judgments of the RAWs whose
      sidecars it moved again (`Index::clear_judgments`, as
      `foldersidecars::delete_run` does), and only those. A sidecar whose
      move failed keeps its RAW's row. A Move to Trash run clears nothing.
    - `Runs::rename_dir` carries the remembered RAW paths along with the
      rest.
    - An engine test covers delete, undo, then redo with the folder closed:
      rows judged again after the undo (as a reopen's reconcile leaves them)
      are cleared by the redo. It goes through the closure-driven seams, not
      the real Trash.
    - The app's redo passes its index to the engine.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/engine/src/trash.rs` (`Trashed`, `Runs`, `RunsState`'s
      `undone: Vec<(u64, Vec<PathBuf>)>`, `Restored::back`, `redo`,
      `redo_chunked`, `redo_groups`, `redo_run`),
      `crates/engine/src/foldersidecars.rs` (`delete` knows each `Doomed`'s
      `raw`; `delete_run` records the run), `crates/app/src/main.rs`
      (`redo_trash` calls `redo_run`; the view has `self.index`).
    - The todo item suggests a field on `trash::Trashed`. What the undo
      keeps for the redo (`back`, plain paths today) has to carry it too.
    - The engine tests never touch the real Trash (`undo_run` and `redo_run`
      do), so put the "which RAWs does this redo clear" decision in a pure
      function the test can drive with closures, and keep `redo_run` as the
      thin caller that applies it to the index.
    - `clear_judgments(dir, paths)` matches on `dir` and `path`. Verify that
      the row's `dir` equals the RAW path's parent as the index spells it
      (Windows verbatim prefix included), or remember the folder too.
    - `redo_run`'s callers hold the idle gate's in-flight count already, the
      condition `delete_run`'s doc comment asks for.
    - Other constructors of `Trashed` are only in `trash.rs` itself (`run`,
      `trashed`, the tests).

- [ ] Step 4: Undo of Move to Trash restores in chunks on Windows, with its timing recorded (item 5)
  - Done when:
    - On Windows, `trash::undo_run` restores a run in chunks of one shell
      operation each, not one operation per file, keeping `restore`'s rules:
      a RAW (or JPEG) before its sidecars, a sidecar left in the Trash when
      its RAW did not come back, never overwriting, never stopping at a
      failure, each failure naming its file.
    - The result is what `restore` gives for the same run and the same Trash
      and disk: `restored`, `failed` and its messages (`ALREADY_EXISTS`,
      `NOT_IN_TRASH`, `RAW_NOT_RESTORED`), `back` in the recorded order, and
      `failed_items`. Tests cover a full run, a colliding destination, an
      item missing from the Trash, a RAW that fails with its sidecars held
      back, a sidecar-only run, a run longer than one chunk, and a chunk
      call that fails after restoring part of its files.
    - macOS and Linux keep their current per-file restore.
    - `learnings.md` of this plan holds the Windows 11 timing of an Undo of
      at least 600 files before and after the change (total and ms per
      file), with the method, the volume, the file set and the safety
      checks, against the baseline of 85.6 s for 605 files.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/engine/src/trash.rs` (`restore`, `restore_run`,
      `undo_run`). The model is what `run_chunked` / `move_chunk` did for
      the move: a pure function over closures (a `restore_many` next to the
      per-item mover), `CHUNK` files per operation, and a per-item fallback
      after a failed chunk.
    - `trash` 5.2.9 on Windows (`restore_all` in its `src/windows.rs`):
      it first returns `RestoreCollision` if any item's original path
      exists, before anything moves; then one `IFileOperation` with
      `FOF_NO_UI | FOFX_EARLYFAILURE` and one `PerformOperations`. So filter
      out existing destinations before the call, and after an `Err` treat a
      file now at its destination as restored and send the rest through the
      per-item path. `TrashItem` is `Clone`; `restore_all` consumes its
      items, so keep copies for the fallback.
    - `restore_all` returns `RestoreTwins` when two items share an original
      path. `newest_by_path` already keeps one item per path.
    - On Linux `restore_all` is a loop of single restores and macOS restores
      through `restore_recorded`, so chunking is Windows only, as
      `run_chunked` is.
    - A file with `restore_to` set (its folder renamed since the run) stays
      on the existing per-item path (`create_missing`, restore, `relocate`)
      (decision 3).
    - The archived bench
      (`docs/plans/_archived/20261106-trash-over-scan/trash-bench.md`)
      measured one `restore_all` of 150 files at about 1 s, and about
      180-200 ms per shell operation.
    - Measurement (decision 2): a throwaway program in the session's scratch
      folder that depends on `handpic-engine` by path and calls
      `trash::trash_listed` then `trash::undo_run` on bench copies, built
      once against the branch's base and once against the branch. It runs
      without the folder watcher and the index, unlike the in-app baseline;
      `learnings.md` says so. Use bench copies only, in a folder made for it
      on the measured volume, never the user's photos. Record the Recycle
      Bin's item count before and after. Each restored file leaves an
      orphaned `$I` in `$Recycle.Bin` (a separate `todo.md` item, not fixed
      here); remove exactly the ones naming the bench folder, as the archived
      bench did. Do not turn on `mcpAllowDestructive` or seed a settings
      store with it: only the user flips that switch.
    - `foldersidecars::delete` still moves one sidecar per shell operation
      (`trash::run` with `trash_one`). That is outside this item; note it in
      `learnings.md`, do not change it.

- [ ] Step 5: The trash undo, Rewrite Sidecars and Delete Sidecars runs report progress to the bar (item 3)
  - Done when:
    - `trash::undo_run`, `foldersidecars::rewrite` and
      `foldersidecars::delete_run` each take a `(done, total)` callback,
      with engine tests of what each reports (as
      `progress_counts_every_group_a_failed_raw_included` and
      `progress_counts_the_groups_of_each_chunk` do for the move).
    - The app feeds the status row's progress circle from the three runs,
      and it is hidden again when each ends, also on a failure. The
      precedence in `progress::shown` is tested where it changes.
    - `get_status`'s `Operation.progress` reports the same counts
      (decision 6).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 4 is merged: the undo's callback is called per chunk on
      Windows and per item elsewhere, as the move's is.
    - Engine files: `crates/engine/src/trash.rs` (`restore`, the chunked
      restore, `undo_run`), `crates/engine/src/foldersidecars.rs`
      (`rewrite`, `delete`, `delete_run`). `delete` already calls
      `trash::run` with a `|_, _| {}` progress, so its callback is a
      pass-through.
    - `rewrite` hands every row to `Writer::set_now` and then waits on one
      `Writer::flush(bound)`; the writes happen on the writer thread, so
      there is no per-row completion to count today. Queue and flush in
      batches, reporting after each batch, under one overall `REWRITE_DRAIN`
      deadline, with no `Writer` change (decision 4); check that batching
      does not slow the run noticeably. `Summary.flushed` and the
      `REWRITE_DRAIN` bound keep their meaning.
    - App files: `crates/app/src/main.rs` (`undo_trash`, `restored`,
      `progress_circle`, the `Event::TrashProgress` handling),
      `crates/app/src/folderops.rs` (the sidecar run's `background_spawn`,
      `sidecars_ran`, `Message`), `crates/app/src/scan.rs` (`Event`),
      `crates/app/src/progress.rs` (`shown`).
    - Existing patterns: the move and the redo send `Event::TrashProgress`
      into the view's channel and keep the counts in `trashing`; a folder
      run sends `Message::SequenceProgress` / `ImportProgress` and keeps
      `folderops.sequence_progress` / `import_progress`, each dropped when
      the run has ended.
    - The undo does not set `trashing` (decision 5): it gets a count of its
      own, and the sidecar runs a `folderops` count like the sequence and
      import ones. `trashing` words the pane note, disables the strip menu's
      trash items, refuses `confirm_trash` and is the companion's
      `trashing`, none of which should change.
    - A Delete Sidecars run ends through `trashed`, which clears `trashing`;
      a Rewrite ends in `sidecars_ran`.

## Trade-offs and risks

Decided on 2026-11-23: the recommended (first) option of each.

1. **Where the five `todo.md` headings are deleted.**
   - Recommended: in the wrap-up, through `todo-curator`, one edit after the
     last merge. `todo.md` is edited by other sessions concurrently, so one
     late, small edit conflicts least, and the steps stay code-only.
   - Alternative: each step's PR deletes its own heading.

2. **How the Windows timing of Step 4 is taken.**
   - Recommended: a throwaway program outside the repository calling the
     engine functions on bench copies, before and after. It excludes the
     watcher and the index, unlike the 85.6 s baseline.
   - Alternative: time it in the app from its `trash: undo of run … in …ms`
     log line, which needs the user to turn on `mcpAllowDestructive`.

3. **Files of a renamed folder in the chunked restore (Step 4).**
   - Recommended: keep a file with `restore_to` set on the per-item path.
   - Alternative: create the missing folders before the chunk and relocate
     after it.

4. **How Rewrite Sidecars counts progress (Step 5).**
   - Recommended: queue and flush in batches under one overall deadline.
   - Alternatives: poll the folder's dirty rows during the flush, or a
     completion signal from the writer thread.

5. **Whether an undo sets `trashing` (Step 5).**
   - Recommended: no; separate counts.
   - Alternative: reuse `trashing` with a new `Mode`.

6. **Whether `get_status`'s `Operation.progress` also reports the new counts.**
   - Recommended: yes.
   - Alternative: the bar only.

7. **The `quit:` line of Step 1.**
   - Recommended: add a field to the existing line.
   - Alternative: a second `quit:` line.

8. **Step order 4 then 5.**
   - Recommended as planned, so the undo's callback is written once on the
     chunked shape.

Risks:

- Step 4 changes how files come back from the Recycle Bin. A chunk call can
  fail after restoring part of its files; the fallback must find what came
  back by looking at the disk, as `move_chunk` does for the move.
- Step 4's measurement writes to the real Recycle Bin of this machine. Bench
  copies only, bin counts checked before and after, the bench folder's `$I`
  orphans removed.
- Steps 3, 4 and 5 all change `crates/engine/src/trash.rs`. They are ordered
  and each assumes the earlier ones are merged.
- Step 3 changes `redo_run`'s signature and Step 5 changes `undo_run`'s,
  `rewrite`'s and `delete_run`'s. `crates/app` compiles the engine by path,
  so each step updates the app's call sites in the same PR.
- Seen during planning and outside these five items: Delete Sidecars' own
  move is one shell operation per sidecar on Windows. It goes to
  `learnings.md` for the wrap-up to curate.

## Progress

(2026-11-23) Step 1 complete
