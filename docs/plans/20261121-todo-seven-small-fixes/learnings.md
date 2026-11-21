# Learnings: todo seven small fixes

## Step 1

- (4) No re-listing stops a Sequence or Import run: `sequence_progress` and
  `import_progress` are cleared only in `sequence_ran` / `import_ran`, and a
  cancel only sets the run's flag, so the run still ends through them.
  `progress_circle` needed nothing more than the `folder_op` argument.
- (5) The guard moved into `ClearFlow::idle(busy, up)` so it could be tested
  as a pure function; `drain_clear` is now a thin call. Auditing the other
  `gate.leave_in_flight()` sites found two more ends that can be idle with
  no listing after them: `trashed` with no folder open (a Delete Sidecars of
  a tree folder, say) and `sidecars_ran`'s rewrite with no change waiting.
  Both call `drain_clear` too. `cleared` needs none: a press while a clear
  runs is ignored, so nothing can be held at its end. `drain_clear` is safe
  after a re-listing starts, as `Resync::busy` is then set and the guard
  waits for `finished`.
- (6) The `Writer` behind the `Arc` keeps its sender in an `Option` so its
  `Drop` can drop it before joining the thread (joining with the sender
  alive would hang). The settings tests ran 20 times in a row on Windows
  without a failure.
- (7) On Git Bash, `node` is not on the PATH outside mise; run the script
  through `mise exec -- bash …`. The decoy case failed as expected with the
  `unset` line removed (the decoy's HEAD moved and it gained a worktree),
  and the real repository's HEAD and worktree list were untouched either
  way.

## Deferred issues (todo candidates)

- App: a held Clear Cache whose idle point came while a trash or folder
  dialog was up is not drained when that dialog is canceled. `drain_clear`
  returns early while `trash.busy()` or `folderops.dialog_open()`, and the
  dialogs' Cancel / Escape paths (`follow_trash`, `follow_folder_dialog`) do
  not call it, so the clear waits for the next listing's `finished` (or the
  user's next press, which asks at once). Found auditing the idle points for
  item (5). Files: `crates/app/src/main.rs` (`drain_clear`, `follow_trash`),
  `crates/app/src/folderops.rs` (`follow_folder_dialog`).
