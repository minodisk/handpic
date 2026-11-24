# Learnings

## Step 1: the quit waits for a retired scan

- Shape chosen: `scan::Running` now holds the held scan and a list of
  `Retired` entries (id, cancel flag, the `ended` receiver).
  `Running::retire` splits the held `Scan`: the `Retired` part stays in the
  holder, the `Stopping` part (id, cancel flag, `JoinHandle`, a `Running`
  clone) goes to the re-listing's background task, whose `Stopping::stop`
  joins and then drops the entry by id. `Scan::stop` had no other caller and
  was removed.
- `Running::stop_within` takes both out under the lock and waits outside it,
  so the background `Stopping::stop`'s removal never blocks on the quit's
  wait. It sets every retired scan's cancel flag before waiting on any, and
  one deadline covers the held and retired waits.
- The quit line is now `quit: scan stopped=<bool> retired=<none|bool> in
  <ms>ms`; `stopped=` keeps its meaning (the held scan, true with none held).
- The holder test fakes the scan thread (a thread blocked on a release
  channel, built from `Scan`'s private fields in the child test module) so
  the wait is observable without a real-time bound: the only timing in it is
  a 100 ms "not returned yet" check, which a loaded machine can only make
  pass, not fail. The test first waits until the quit's stop has taken the
  retired entry out of the holder, otherwise the background stop could drop
  the entry first and the quit would see none.
- First full `mise run ci:local` (CARGO_BUILD_JOBS=4) built and ran all
  1103 app tests. Only `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped`
  failed (`held: false`: a real scan did not end within the 2 s
  `STOP_TIMEOUT` while other sessions were building), the flake `todo.md`
  already lists. Its held path (`Scan::stop_within`) is unchanged by this
  step. The two new holder tests passed. A re-run of the scan tests alone
  was killed for low memory.

## Step 2: a dialog cancel drains a held clear

- The MCP `cancel_operation` reaches the same two functions
  (`Cancel::TrashDialog` calls `dismiss_trash`, `Cancel::Folder`
  `dismiss_folder_dialog(false, ..)`), so the two cancel paths cover the
  user's Cancel / Escape and the MCP cancel alike. `drain_clear` was added
  after `follow_trash` in `dismiss_trash` and after `follow_folder_dialog` in
  each of `dismiss_folder_dialog`'s three branches; its `ClearFlow::idle`
  guard makes the call a no-op only where the dialog stays open or the view
  is still busy (the Cancel of a running import or sequencing, whose dialog
  stays open until the run ends). An import set aside lets a held clear go
  on, because its run does not hold the index.
- The reachable state is an MCP-confirmed clear (`confirmed_clear`): a
  pressed one needs the settings open, which `confirm_trash` and
  `start_sidecars` refuse. The UI tests in `crates/app/src/uitest.rs` open
  the dialog first (a real Move to Trash question, a real Rewrite Sidecars
  preview), then `resync.open()` + `relist` to make the view busy, confirm a
  clear (held), wait for the re-listing's `finished` (the idle point, which
  leaves the clear held under the dialog), cancel the dialog and check that
  the clear runs.
- The first build of the two new tests (`cargo test --bin handpic
  drains_a_held_clear`, CARGO_BUILD_JOBS=4) was killed for low memory
  before it finished, so they have not run yet, and neither has `mise run
  ci:local`.
- Local checks (`mise run ci:local`, cargo build / test / clippy) were
  skipped for this step by the user's decision, since this machine runs out
  of memory; only `mise run fmt` ran locally, and the PR's GitHub Actions
  checks verify the build, clippy and the new tests.

## Step 3: a redo of Delete Sidecars clears a closed folder's rows

- Shape: `trash::Trashed` got `owner: Option<PathBuf>` (the RAW a sidecar
  belongs to, set by `foldersidecars::delete` from each `Doomed`'s `raw`),
  and what an undo keeps for the redo became `trash::Back { path, owner }`
  (`Restored::back`, `Runs::undone` / `take_undone`). `Runs::rename_dir`
  rebases `owner` in both the recorded runs and the undone ones.
- `redo_run` now takes `index: Option<&Mutex<Index>>` and is a thin caller
  of `trash::redo_back(runs, index, back, again)`, which runs the move
  through the `again` closure (the real `move_again`, or `trash::redo` with
  a fake mover in the test), puts each moved sidecar's owner back on the new
  run's `Trashed`, records it, and clears `trash::cleared(&moved)` (the
  owners, each once, grouped by their parent folder) in the index. Only the
  moved files carry an owner, so a failed sidecar keeps its RAW's row, and a
  Move to Trash run has no owners and clears nothing.
- The folder key: `doomed` lists the canonical `dir` with `read_dir`, so a
  RAW's path is `dir` joined with its name and its parent is `dir` spelled
  as `delete_run` passes it to `clear_judgments` (the verbatim prefix kept).
  No separate folder is remembered.
- A failure of `clear_judgments` after a redo is logged (`log::warn!`), not
  returned: the files have moved and the new run is recorded, and an `Err`
  would keep the app from pushing that run on its history.
- The engine test (`foldersidecars::tests::a_redo_with_the_folder_closed_clears_the_rows_the_undo_left_judged`)
  runs delete, the undo (`trash::restore` from a fake Trash folder, then
  `Runs::undone`), a re-judging of the rows as a reopen's reconcile would,
  and `redo_back` over `trash::redo` with one refused sidecar.
- Local checks: only `mise run fmt` ran; no build, clippy or tests (user's
  decision, this machine runs out of memory). The PR's GitHub checks are the
  first build of this change.

## Step 4: the undo restores in chunks on Windows

- Shape: `trash::restore` was split into the per-file decisions and a
  `tally` of the results (the `Restored` bookkeeping: `restored`, `failed`,
  `back`, `failed_items`), which the new `restore_chunked` shares, so the two
  cannot drift apart. `restore_chunked` takes the run's items (a RAW or JPEG
  with the sidecars after it, a sidecar with no RAW before it alone) `CHUNK`
  at a time: one `restore_many` for their heads, then one for the sidecars of
  the heads that came back; the sidecars of a head that did not stay in the
  Trash (`held_back`, the same `RAW_NOT_RESTORED` / `NOT_IN_TRASH` rule).
  `restore_chunk` (the restore's `move_chunk`) leaves out a file whose
  destination exists or that the Trash no longer holds, since one collision
  refuses the whole `restore_all`, and after a failed call counts a file now
  at its destination as back and retries the rest through the per-item
  `restore_item` (the old closure of `restore_run`, unchanged).
- `restore_run` picks the strategy through `restore_listed`: chunked under
  `cfg(windows)`, `restore` elsewhere on non-macOS; macOS keeps
  `restore_recorded`. A file with `restore_to` set never joins a chunk
  (decision 3).
- The tests drive `restore_chunked` and `restore` against the same fake Trash
  and disk (`Bin`) and assert that both give the same outcome, then check the
  chunk calls and the per-file retries.
- Local checks: only `cargo fmt` ran; no build, clippy or tests (user's
  decision, this machine runs out of memory). The PR's GitHub checks are the
  first build of this change.
- The Windows timing was not taken: the throwaway bench's release build
  (`handpic-engine` by path, `CARGO_BUILD_JOBS=2`, 7 GB free at its start)
  was stopped by the harness for low system memory before it finished. Nothing
  touched the Recycle Bin: the bench never ran, no bench folder was created,
  and the scratch worktree of the branch's base was removed.
- Unchanged and outside this item: `foldersidecars::delete` still moves one
  sidecar per shell operation on Windows (`trash::run` with `trash_one`).

## Step 5: progress for the trash undo and the sidecar runs

- The undo's unit is the item, as the move's is the group: a RAW or JPEG
  with the sidecars after it, or a sidecar with no RAW before it
  (`trash::item_starts`, which `restore_chunked` already computed inline and
  now shares with `restore`). `restore` reports after each item's last file,
  `restore_chunked` after each chunk (heads and their sidecars), so on
  Windows the circle moves per `CHUNK` items. `restore_recorded` (macOS)
  passes the callback through to `restore`.
- `foldersidecars::rewrite` now hands the writer `REWRITE_BATCH` (32) rows,
  then `flush`es with what is left of one `bound` deadline and reports
  `(rows drained, total)`; once a flush times out, the rest are queued with
  no wait, as before the change everything was, so `Summary.flushed` still
  means "drained within `REWRITE_DRAIN`" and `failed` still counts the rows
  left dirty. The writer is one thread writing in order, so a batch's flush
  only adds one channel round trip per 32 writes; the slowdown was not
  measured (no local build on this machine).
- `foldersidecars::delete` passes its callback to `trash::run`, one group per
  sidecar, so the delete counts sidecars.
- App: the undo keeps `Handpic.undo_progress` (decision 5, not `trashing`),
  fed by `Event::UndoProgress` and cleared in `restored`; the sidecar runs
  keep `FolderOps::sidecar_progress`, fed by `Message::SidecarProgress` and
  cleared at the top of `sidecars_ran`, which both a rewrite and a delete
  end through (the delete then goes on into `trashed`). Neither message
  carries a run id: like `TrashProgress`, a message is taken only while the
  count is `Some`, since only one such run is out at a time.
- `progress::shown` gained an `undo` argument between the trash run and the
  folder operation; the sidecar counts join the folder operation's after the
  sequencing and import ones. `get_status`'s running `Operation.progress`
  chains the same counts (decision 6).
- Local checks: only `cargo fmt` / `mise run fmt` ran; no build, clippy or
  tests (user's decision, this machine runs out of memory). The PR's GitHub
  checks are the first build of this change.

## Step 4 follow-up: Windows timing of an Undo of Move to Trash (measured 2026-11-24)

On D: (NTFS, the volume of the archived trash-bench baseline), an Undo of 600 files took 83.9 s and 88.3 s before the chunked restore (`0723c2cf^`, b4b5f0fa) and 3.99 s and 3.82 s after it (`0723c2cf`): about 140 ms a file before, 6.5 ms after, roughly 22x faster. Every run put all 600 files back with no failures.

| Run | Files | Undo total | ms/file | RAWs restored | Failed |
| --- | ---: | ---: | ---: | ---: | ---: |
| after-1 | 600 | 3,990 ms | 6.7 | 300 | 0 |
| before-1 | 600 | 83,910 ms | 139.9 | 300 | 0 |
| after-2 | 600 | 3,820 ms | 6.4 | 300 | 0 |
| before-2 | 600 | 88,270 ms | 147.1 | 300 | 0 |

- Method: a throwaway bench outside the repository per version (worktrees at each commit, release build, `CARGO_BUILD_JOBS=2`), listing a fresh folder with `listing::read_listing`, moving it with `trash::trash_listed`, and timing only `trash::undo_run` with `Instant`. Runs alternated after, before, after, before.
- File set: synthetic only, 300 `B0000.ARW`…`B0299.ARW` (10 KB of zeros) each with a `.xmp` sidecar, a new folder per run under `D:\handpic-undo-bench`.
- Against the in-app baseline of 85.6 s for 605 files (141 ms a file), the before build matches closely, so the per-operation shell cost dominates. The after build is near the chunked move's 7 ms a file.
- Caveat: the bench excludes the app's folder watcher, the index, the UI and the re-listing after the undo, so an in-app undo of ~600 files takes somewhat longer than 4 s.
- Safety: the Recycle Bin held 303 items before and after every run. Each run left exactly 600 orphaned `$I` files naming the bench folder (the separate `todo.md` item), and exactly those were deleted. `mcpAllowDestructive` and the settings store were not touched.

## Deferred issues (todo candidates)

- Issue heading: `Engine: Delete Sidecars… still moves one sidecar per shell operation on Windows`.
  `foldersidecars::delete` still moves each sidecar alone (`trash::run` with
  `trash_one` in `crates/engine/src/foldersidecars.rs`). The same
  per-operation cost was measured at about 140 ms a file for the undo, against
  about 6.5-7 ms a file once chunked (Step 4 follow-up). Change: route the
  delete through a chunked move like `trash_listed`'s `run_chunked`, keeping
  the per-sidecar `(done, total)` progress Step 5 added. Done when: a Windows
  timing of a large sidecar delete before and after is recorded; the outcome
  (moved and failed files, the recorded run, each `Trashed`'s `owner`)
  matches the per-file path in a unit test with a fake mover; the progress
  circle still moves. Seen while planning this plan, outside its five items.
