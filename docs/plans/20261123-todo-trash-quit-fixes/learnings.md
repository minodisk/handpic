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

## Deferred issues (todo candidates)

- Pending manual check (Step 4 timing, Windows 11): measure an Undo of Move to
  Trash of at least 600 files before and after
  `perf(engine): restore an undo of Move to Trash in chunks on Windows`
  (`crates/engine/src/trash.rs`, `restore_chunked`). Method (plan decision 2):
  a throwaway program outside the repository depending on `handpic-engine` by
  path (release build), which writes bench copies (e.g. 404 synthetic `.ARW`
  plus 202 `.xmp`, 606 files) into a new folder on the measured volume (D:,
  as the baseline), calls `trash::trash_listed` then times
  `trash::undo_run`; built once against the branch's base and once against
  the change. Safety: bench copies only, never the user's photos; record the
  Recycle Bin item count before and after (`Shell.Application`
  `Namespace(10).Items().Count`); afterwards delete exactly the orphaned `$I`
  files in `D:\$Recycle.Bin\<SID>` naming the bench folder with no `$R`
  partner; never turn on `mcpAllowDestructive`. Expected: the undo well under
  the 85.6 s / 605 files (141 ms a file) baseline, in the region of the move's
  chunked 7 ms a file; record total and ms per file for both builds in this
  file. It runs without the watcher and the index, unlike the baseline. The
  Step 4 checkbox was ticked on the code and test criteria; this measurement
  was skipped because the bench build ran the machine out of memory.
- `foldersidecars::delete` (Delete Sidecars…) still moves one sidecar per
  shell operation on Windows (`trash::run` with `trash_one` in
  `crates/engine/src/foldersidecars.rs`); chunking it like `trash_listed`'s
  `run_chunked` would speed a large run up. Seen while planning this plan
  (item 5's investigation), outside its five items.
