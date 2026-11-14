# Learnings: the folder operation dialogs in the GPUI app

## Step 1: the Tauri-free parts of the rename, the date rename and the folder sidecars

- Started before all of part 1 was merged, with the user's approval: only part
  1's Steps 1 and 2 (the backend's `folders.rs`, `treewatch.rs`, `trash_one`,
  `listing::kind`) are needed, and they are on `main`.
- Moved into `crates/backend`: `rename.rs` (`Renamed`, `check_name`,
  `folder_target`, `FilePlan`, `file_plan`, `file_run`, and the new
  `Context` + `rename_folder`), `datefolder.rs` (`date_of`, `Kind`, `Group`,
  `Plan`, `plan`, `dated`, `Target`, `Preview`, `preview`, `plan_of`,
  `move_plan`, `split`, `remove_if_empty`, `Moved`, `Split`, `Ran`,
  `split_folder`, and `file_name`, made public because `date_folder_run`'s
  "already named" message uses it), `foldersidecars.rs` (`REWRITE_DRAIN`,
  `Preview`, `Summary`, `targets`, `preview`, `rewrite`, `Doomed`,
  `KindCount`, `DeletePreview`, `doomed`, `delete_preview`, `delete`, and the
  new `delete_run`), and `text.rs` (`SCAN_RUNNING`, `SIZE_BASE`,
  `format_bytes` with its tests). `text.rs` is a new module rather than
  `index.rs` or `foldersidecars.rs` because both apps' settings and every
  folder operation use these, and none of the existing modules owns them.
- Parameter shape: one `rename::Context { watch, trees, index, runs }` shared
  by `rename_folder` and `split_folder`; `index` is `Option<&Mutex<Index>>`,
  which the Tauri app gets from `AppIndex.0.as_deref()`. `plan_of` and
  `delete_run` take the optional index directly.
- Behaviour kept: `rename_folder_blocking` in the app still runs
  `folder_target` before the writer flush and the `Scans` lock, so a bad name
  is refused before a drain, as before; the backend's `rename_folder` checks
  it again, which is harmless (a target that appeared meanwhile is refused
  either way).
- The app's `watch::release_under` and `treewatch::with_released` wrappers
  lost their last callers and were removed (the backend calls the backend's
  directly through the `Context`).
- `crates/app/src/import.rs` used `crate::datefolder::date_of`; it now uses
  `riffle_backend::datefolder`.
- No `#[cfg(test)]` helper crossed the boundary: the moved tests use only
  public `Index` / `Writer` API already in the backend, and the
  `include_bytes!` fixture path (`../../core/src/fixtures/...`) resolves the
  same from `crates/backend/src`.
- Plain `cargo` is not on the Bash tool's `PATH` here; `mise exec -- cargo
  ...` works.

## Step 2: the Tauri-free parts of the import and the sequencer

- The settings plan's Step 7 had not landed (`origin/main` had no
  `crates/backend/src/import.rs`), so this step created the module with the
  settings pieces too and added `chrono` to the backend; that plan's Step 7
  now extends it (its settings rules, `root_setting`, `pattern_setting`,
  `example`, are already public there).
- Moved into `crates/backend/src/import.rs`: `DEFAULT_PATTERN`,
  `ImportSettings`, `validate_pattern`, `format_date`, `check_segment`
  (private), `root_setting`, `pattern_setting`, `example`, `Found`, `Photo`,
  `walk`, `name_of` (made public, the app's progress event names the file
  with it), `Decision`, `Planned`, `Destination`, `Plan`, `plan`,
  `FolderSummary`, `Totals`, `Preview`, `refusal`, `summarize`, `plan_dir`,
  `Failure`, `Ran`, `Prepared` (public with public fields: `prepare` returns
  it and the native app reads `folders`), `prepare`, `run_plan`, `log_line`,
  with 25 of the 26 tests. The app keeps the settings state and commands,
  `ImportPreviews`, `Imports` / `begin` and its one test.
- `crates/backend/src/sequence.rs`: `Failure`, `Row`, `Preview`, `preview`,
  and the new `run(dir, cancel, progress) -> Ran`, the body `sequence_run`
  had inline (the output folder, the failures, a folder-level error as the
  one failure keyed by `dir`). Its progress callback is `Fn(usize, usize) +
  Sync`, not `FnMut`: `riffle_core::sequence::run` calls it from its worker
  threads. The two preview tests moved; the `begin` test stays in the app.
- `crates/native/Cargo.lock` gained only `chrono` under `riffle-backend`
  (chrono was already in the lock through `riffle-core`).

## Step 3: the folder menu's items, the shared dialog, and the folder sidecar operations

- `folderops.rs` is a child module of `main.rs` with an `impl Riffle` block,
  not an entity: a child module reaches the view's private fields, methods
  and the color constants, and its own methods called from `main.rs` need
  `pub(crate)`.
- No `Event` variant: the preview and the runs come back through
  `cx.spawn` + `update`, and nothing here reports progress. Step 4 adds the
  variant its run's progress needs.
- Part 1's `TrashEntry` had no `what`, and its `undone_trash` counted
  `Restored::restored`, which never lists a sidecar, so an undone sidecar run
  would have had nothing to redo. It gained `what: trash::What`, and
  `trash::restored_count` ports `restoredCount` (a sidecar run counts the
  sidecars that did not fail). `trashed_status` / `restored_status` take the
  `What`, and the gate's undo / redo labels come from it (`Undo Delete
  Sidecars`).
- A sidecar delete ends in part 1's `trashed` rather than a copy of it: it
  already leaves the in-flight count, records the run, writes the failures,
  sets the status and re-lists (or opens the deferred folder). The Tauri run
  re-reads the entries instead of re-listing (`rescan = false`); the native
  `trashed` re-lists, which reads the cleared rows back too.
- `resync.hold()` is taken only when a folder is open: `trashed` and the
  rewrite's end release it through `Resync::open` / `finished` only on that
  path, and a hold with no folder open would leave every later request held.
- The dialog's Enter and Space both press the focused button, as a focused
  HTML button and part 1's trash dialog do; the plan's Done-when names Enter
  only.
- `clippy::too_many_arguments` fired once `trashed` took `what`; it carries an
  `#[expect]` naming the Tauri `trashed`'s parameters.

## Step 4: Sequence JPEG Timestamps…

- `SequenceFlow` drops the Tauri `picking` phase, `started`, the early
  `done` and `cancelAsked`: `run` hands out the run id before the thread
  starts, so no message can precede it. `start` takes the folder directly.
- The run is a `std::thread`, not `background_spawn`: it can take minutes
  and would hold one of the executor's threads. `riffle_core::sequence::run`
  calls the progress callback from its worker threads, which only send into
  the view's `UnboundedSender` (it is `Sync`).
- The quit's cancel is a `cx.on_app_quit` subscription stored next to the
  flag and dropped at the run's end, so no handler piles up per run. If the
  window drops the view before the quit handlers run, the subscription goes
  with it and the flag is not set; the quit does not wait for the run
  either way, so a file in flight may be cut short, which the next run's
  rebuild replaces.
- The shared dialog had to grow: rows with a highlighted part (a
  `StyledText` with one highlight, which keeps the spaces a flex row of two
  texts might trim), several notes after the rows, and errors in the error
  color. The sidecar dialogs now pass `notes: vec![total line]`.
- Bug caught by the ported test: `done_status` first returned early with
  `folder_error(ran)?`, the inverse of the rule.
- The preview failure and the run's failures go to `write_errors` through
  `write_failed`, which renders `<file name>: <message>` exactly as
  `failureText` words it, and which the next folder open clears, as the
  Tauri `errors.clear()` does.

## Step 5: Import…

- The settings plan had fully landed: `Settings::import_root` /
  `import_pattern` were already read with the backend's `root_setting` /
  `pattern_setting` and kept current by the settings modal's `Change`
  events, so the import only reads them when it starts.
- Ordering: the thread sends the created folders through one `oneshot` and
  the result through a second; one `cx.spawn` awaits them in that order, so
  the run's end can never be handled before its start, which is why the
  Tauri flow's early `done` is not ported. The open of the first folder is
  `open_from_tree`, changed to return the open's `Task` (its two callers
  detach it), and the report awaits it: the native open's non-carry listing
  clears `write_errors`, as the Tauri open clears `errors`, so reporting
  first would wipe the run's failures.
- `run_plan`'s progress callback fires per buffer written; the thread sends
  a message only when `files_done` changes, so the channel carries one
  message per file.
- `FolderOps::busy` now includes the import for its whole run, dialog aside
  or not, which shuts the settings modal, the trash dialog and the other
  folder flows meanwhile, as `importFlow.busy` does in `main.ts`.
  `import_busy` is the flag Step 7's rename refuses on.
- A quit mid-import sets the cancel flag and waits for nothing. The copy
  deletes the in-flight temp file when it sees the flag, but if the process
  exits first the file can stay under its `.handpic-tmp` name in the
  destination folder; the files copied before stay and a second import skips
  them by name and size. The same holds for the Tauri app.
- Python's text-mode write on Windows turned edited files' line endings to
  CRLF; write with `newline=''`. `plan.md` was CRLF in this worktree before
  the step (git normalizes it, so it showed no diff).
- Checks run per the caller's memory constraint (no root `mise run ci`):
  `mise run fmt`, and in `crates/native` `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo test` (756 passed). The backend was
  not touched.

## Step 6: Rename by Capture Date…, and what follows a folder rename

- `notify` 8.2.2 (the native lock): `datefolder::watch_release`, a test in
  the native crate (the backend's tests cannot run from the native
  workspace), puts the open folder's watcher on a temp folder and tree
  watches on it, its parent and its subfolder, checks a plain
  `std::fs::rename` of it fails on Windows, then that the backend's
  `rename_folder` renames it, that `watch::set` on the new path reports a
  file written there, and that `split_folder` empties and removes a watched
  folder. It passed five runs in a row (about 0.5 s each).
- Windows refuses a folder's rename while any handle to a file inside it is
  open, even one opened with `FILE_SHARE_DELETE` (as Rust's `File::open`
  opens): checked with a PowerShell `FileStream` and `Directory.Move`. The
  native reads inside the open folder (preview, prefetch, on-demand faces /
  eyes, 1:1 crop) are open-read-close with no memory map, and the scan is
  refused before the run, so only a read in flight at that instant can
  refuse the rename; the backend then puts the watcher back and the status
  shows the OS error. No extra guard was added.
- `Riffle::dir` and a trash entry's `dirs` are canonical (`\\?\` on Windows)
  while the tree's paths are not, so `folder_renamed` takes both spellings:
  the tree, its open row, its cursor and a deferred open are rebased over
  the tree's spelling, the trash entries and the reopen over the canonical
  pair the background task computes before and after the rename. The reopen
  opens `context::shown_path` of the rebased canonical path.
- The reopen is `open`, the tree-click path: its non-carry listing drops the
  undo histories' judgment batches, as the Tauri folder switch does, and
  clears the status, so the warning (and a split's status when the first
  date folder opens) is set after the open's task (`open_with_note`).
- A split's `tree_refresh` of the parent also forgets the watched set last
  handed over: the split released the tree watches under the folder, and an
  unchanged set would not have been sent again.
- `main.rs` gained only `mod datefolder;`: `folder_action`'s fallthrough arm
  already routes the item to `folderops`.
- `folderops.rs` is CRLF in this worktree, so the edit scripts normalized the
  line endings before matching. A bash heredoc holding a Python script with
  backslashes in its strings mangled the Windows paths (`\t`, `\2`); the
  scripts and texts went through files in the scratchpad instead.
- Checks run per the caller's memory constraint (no root `mise run ci`):
  `mise run fmt`, and in `crates/native` `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo test` (804 passed). The backend was
  not touched.

## Step 7: the tree's inline rename, and the wrap-up

- `textinput.rs` (the settings plan's) already took `ctrl+a` / `c` / `x` /
  `v` on Windows and Linux; it only gained `select_all`. A fresh input entity
  is made per edit (`TextInput::new` needs the `Window`), so `context_action`
  now takes the `Window` and the menu's `Rename…` goes to `start_rename`
  rather than through `folder_action`.
- GPUI blurs the focused element when the window deactivates (the focus path
  of an inactive frame is empty, `window.rs`'s focus phase), so the input's
  `Commit` arrives on an app switch too; it is ignored while
  `!window.is_window_active()`, the Tauri `document.hasFocus()` check.
- A row scrolled out of the tree's `uniform_list` range is not laid out, so
  its input loses the focus and the edit confirms; the Tauri input stays in
  the DOM. Recorded in `native-app.md`, not worked around.
- The native `IdleGate::request` drops a replaced operation silently, so a
  held rename replaced by a trash, undo or sidecar request would have kept
  its pending name forever. `gate_request` discards first and hands a
  dropped rename to `held_dropped` (clear its mark, `Rename… was canceled`
  unless a rename of the same folder replaced it); every request site uses
  it. An open's discard sets the note after its listing (`note_after`), as
  the Tauri open does, since the fresh listing clears the status.
- `PendingRename`'s marks carry an id, standing in for the Tauri mark's
  object identity.
- A click on the open folder's name no longer reopens it (the Tauri tree
  stops that click too); a Ctrl or Shift click still does.
- The input in a 22 px row: its padding and border take 8 px, so the wrapper
  sets the line height to 14 px. Not seen on screen.
- `main.rs` gained: `mod rename;`, `HeldOp::Rename` and its `held_label` and
  `finished` arms, the open's discard through `held_dropped`, the
  `refuse_renaming` guard in `open_from_tree`, `gate_request` at the two
  trash request sites, the key guard, the `Window` in `context_action`, the
  slow click's disarm in the tree pane's two press listeners, and the row's
  name from `tree_name`.
- Wrap-up: every pending manual check of Steps 3-7 is under `## Deferred
  issues (todo candidates)` below. `CLAUDE.md` names `dialog.rs`,
  `folderops.rs`, `sidecars.rs`, `sequence.rs`, `import.rs`, `datefolder.rs`
  and `rename.rs` in the `crates/native` sentence and the backend's
  `rename.rs`, `datefolder.rs`, `foldersidecars.rs`, `import.rs` and
  `sequence.rs`. The port plan's Step 8 is ticked with its Progress line, and
  the strip's `Rename…` and the tree's multi-folder selection are recorded
  there as left out. Its Step 8 note still says folder drag-and-drop onto
  the window is not ported (another change); the tick follows the caller's
  instruction.
- Checks run per the caller's memory constraint (no root `mise run ci`):
  `mise run fmt`, and in `crates/native` `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo test` (855 passed). The backend was
  not touched.

## Deferred issues (todo candidates)

- **Pending manual check (Step 3, Windows, the user's).** Basis: Step 3's
  Done-when; the dialog, its keys and the runs on disk cannot be driven
  headless. The step's checkbox was ticked on the automated criteria
  (`dialog.rs`, `sidecars.rs`, `context.rs`, `trash.rs`, `history.rs` and
  `idle.rs` tests, `mise run ci`, `mise run native:check`). Setup: copy a
  dozen ARWs, a few with a `.xmp` and a `.dop` next to them, into
  `D:\Photos\tests\20261113-gpui-folder-dialogs\sidecars\`, and two JPEGs
  alone into `...\jpeg-only\`; `cargo build` in `crates/native` (rename a
  locked `riffle-native.exe` aside, never kill it); run
  `crates\native\target\debug\riffle-native.exe D:\Photos\tests\20261113-gpui-folder-dialogs\sidecars --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step3.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step3.sqlite --log D:\Photos\tests\20261113-gpui-folder-dialogs\step3.log`
  with `index-step3.sqlite` deleted first (a fresh index). Only these copies.
  Steps and expected results:
  1. Right-click `sidecars` in the tree: the menu shows `Rename…` and
     `Rename by Capture Date…` (dimmed), `Expand All` / `Collapse All`,
     `Rewrite Sidecars from Index…` / `Delete Sidecars…`, `Sequence JPEG
     Timestamps…` (dimmed), `Import…` (dimmed). On a root (home or a drive)
     the rename and expand groups are gone.
  2. Pick `Rewrite Sidecars from Index…` right after the launch, while pass
     1 runs: the pane shows `Rewrite Sidecars from Index: waiting for the
     scan to finish`, the scan is not canceled, and the dialog opens once
     the scan ends. Do the same with `Delete Sidecars…`.
  3. Judge a few files (stars, a pick, a label), then `Rewrite Sidecars from
     Index…`: the dialog lists `n files with a judgment`, `m files without
     one (any existing sidecar is cleared)` (and the skipped count when some
     RAW has no row) and `Rewrite the XMP sidecars of k files in sidecars?`;
     `Rewrite` is outlined; Tab moves the outline to `Cancel` and back;
     Escape closes it. Again with `Rewrite`: the buttons dim, the running
     note shows, the dialog closes and the pane says `Rewrote the sidecars
     of k files in sidecars`; the `.xmp` files carry the judgments.
  4. `Delete Sidecars…`: the dialog lists the per-kind counts and sizes
     (`n XMP sidecars (… KB)`) and `Move n sidecars (…) of sidecars to the
     Trash?` with a red `Move to Trash`. Confirm: the sidecars are in the
     Recycle Bin, the strip's marks are gone, the pane says `Moved n
     sidecars to the Trash`. `Ctrl+Z`: `Restored n sidecars from the
     Trash`, the sidecars are back and the marks return after the
     re-listing; `Ctrl+Shift+Z` moves them again.
  5. Right-click `jpeg-only` and pick either item: the pane says
     `jpeg-only: rewriting sidecars applies to RAW files only` (or
     `deleting …`) and no dialog opens.
  6. While a dialog is open, a click on the tree or the strip does nothing
     and no key reaches the strip. With a request waiting during a scan,
     open another folder from the tree: the waiting note goes and no dialog
     opens.
  7. Pick `Rewrite Sidecars from Index…` and press `Ctrl+,` before the
     dialog opens (while the preview is under way): the settings modal does
     not open.
- **Pending manual check (Step 4, Windows, the user's).** Basis: Step 4's
  Done-when; the dialog, the run's progress, the cancel and the reveal in
  Explorer cannot be driven headless. The step's checkbox was ticked on the
  automated criteria (`sequence.rs`, `dialog.rs` and `context.rs` tests,
  `mise run ci`, `mise run native:check`). Setup: copy a dozen exported JPEGs
  whose `DateTimeOriginal` repeats a second (a burst) into
  `D:\Photos\tests\20261113-gpui-folder-dialogs\sequence\`, plus one
  folder with no JPEG at
  `D:\Photos\tests\20261113-gpui-folder-dialogs\no-jpeg\` (an empty
  folder or one holding a text file); `cargo build` in `crates/native`
  (rename a locked `riffle-native.exe` aside, never kill it); delete
  `index-step4.sqlite` first and run
  `crates\native\target\debug\riffle-native.exe D:\Photos\tests\20261113-gpui-folder-dialogs\sequence --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step4.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step4.sqlite --log D:\Photos\tests\20261113-gpui-folder-dialogs\step4.log`.
  Only these copies. Steps and expected results:
  1. Right-click `sequence` in the tree: `Sequence JPEG Timestamps…` is
     enabled. Pick it: the dialog shows `Source: …\sequence`, `Output:
     …\sequence-sequenced`, one row per file `name  old -> new` with the
     changed field of the new time in green and the unchanged rows muted,
     `n files will be written to sequence-sequenced` and `k of them get a
     new capture time`; `Run` is outlined, Tab moves to `Cancel`, Escape
     closes the dialog.
  2. Again, then `Run`: `Run` dims, `Cancel` keeps the focus, the running
     note shows, the pane shows `sequencing n / m` counting up; at the end
     the dialog closes, the pane says `Wrote n of n files to
     …\sequence-sequenced` and Explorer opens with `sequence-sequenced`
     selected.
  3. Again: the dialog shows `will be rebuilt: its JPEG files are replaced`
     under the output line.
  4. Again with `Run`, then `Cancel` (or Escape) at once: the run stops, the
     dialog closes, the pane says `canceled, i of n written`, and Explorer
     does not open.
  5. Right-click `no-jpeg` and pick the item: no dialog opens and the pane
     shows `no-jpeg: <the no-JPEG message>` in red.
  6. While the dialog is open or the run is under way, `Rewrite Sidecars
     from Index…` from another folder's menu cannot be reached (the dialog
     covers the window) and `Ctrl+,` does not open the settings; a second
     `Sequence JPEG Timestamps…` right after the first (before its dialog
     opens) does nothing.
- **Pending manual check (Step 5, Windows, the user's).** Basis: Step 5's
  Done-when; the dialog, the run's progress, the open of the first date
  folder, the reopen and the cancel cannot be driven headless. The step's
  checkbox was ticked on the automated criteria (`import.rs`, `context.rs`
  and `dialog.rs` tests, `cargo clippy`, `cargo test` in `crates/native`).
  Setup, only under `D:\Photos\tests\20261113-gpui-folder-dialogs\`:
  - Copy into `...\import-src\` RAWs shot on two dates, a few with a `.xmp`
    and a `.dop` next to them, a few JPEGs of those dates, and one file with
    no capture date (e.g. a JPEG stripped of its Exif) as `undated.jpg`.
  - Write `...\settings-step5.json` holding
    `{"importRoot": "D:\\Photos\\tests\\20261113-gpui-folder-dialogs\\import-dst"}`
    and `...\settings-step5-noroot.json` holding `{}`.
  - `cargo build` in `crates/native` (rename a locked `riffle-native.exe`
    aside, never kill it). Delete `index-step5.sqlite` and
    `settings-step5.sqlite` first, then run
    `crates\native\target\debug\riffle-native.exe D:\Photos\tests\20261113-gpui-folder-dialogs\import-src --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step5.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step5.sqlite --import-settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step5.json --log D:\Photos\tests\20261113-gpui-folder-dialogs\step5.log`.

  Steps and expected results:
  1. Right-click `import-src`: `Import…` is enabled. Pick it: the pane shows
     `reading …\import-src…`, then the dialog: `Source: …\import-src`,
     `n files (… MB) to copy into 2 folders`, one row per date folder
     (`…\import-dst\YYYY\YYYY-MM-DD  k files, … MB`), and under `Without a
     capture date, not copied:` the undated file. `Run` is outlined; Tab
     moves to `Cancel`; Escape closes it.
  2. Again, then `Run`: the first date folder opens in the strip and the
     tree, the dialog steps aside, the pane shows `importing n / m` counting
     up while the strip fills; at the end the pane shows `Imported n files
     (… MB)` and the strip holds every copied RAW.
  3. With a larger source (or a slow disk), during a run pick `Import…` again
     from any folder: the dialog comes back with `Cancel` focused and the
     running note; Escape hides it and the run goes on; `Import…` again, then
     `Cancel`: the run stops, the dialog closes, the pane says `Import
     canceled, k files copied`; no `.handpic-tmp` file is left.
  4. `Import…` on `import-src` again: the note says `Nothing left to copy:
     every dated file is already there.` and `Run` is dimmed.
  5. Replace one copied file in `import-dst` with another file of a different
     size under the same name, then `Import…`: it is listed under `Already
     at the destination with another size, not copied:` as `NAME  in
     YYYY-MM-DD`.
  6. Quit, delete `settings-step5.sqlite`, and launch again with
     `--import-settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step5-noroot.json`:
     `Import…` shows `the destination folder is not set (Settings > Import)`
     and `Run` is dimmed.
  7. `step5.log` holds one `import: files=… bytes=… mb_per_s=…` line per run.
- **Pending manual check (Step 6, Windows, the user's).** Basis: Step 6's
  Done-when; the dialog, the tree row following in place, the reopen and the
  undo of a trash run across the rename cannot be driven headless. The
  step's checkbox was ticked on the automated criteria (`datefolder.rs`
  including `watch_release`, `tree.rs`, `history.rs` and `context.rs` tests,
  `cargo clippy`, `cargo test` in `crates/native`). Setup, only under
  `D:\Photos\tests\20261113-gpui-folder-dialogs\step6\`:
  - `one-date\`: a few RAWs shot on one day, a couple with a `.xmp`.
  - `three-dates\`: JPEGs only, shot on three days, plus one JPEG stripped of
    its Exif saved as `undated.jpg` (a JPEG-only folder lists its JPEGs, so
    the undated file counts).
  - `two-dates\`: RAWs shot on two days, nothing undated.
  - `collide\`: RAWs shot on one day (its sibling is created in step 6).
  - No two folders share a shooting day, so no target collides by accident.

  `cargo build` in `crates/native` (rename a locked `riffle-native.exe`
  aside, never kill it). Delete `index-step6.sqlite` and
  `settings-step6.sqlite` first, then run
  `crates\native\target\debug\riffle-native.exe D:\Photos\tests\20261113-gpui-folder-dialogs\step6\one-date --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step6.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step6.sqlite --log D:\Photos\tests\20261113-gpui-folder-dialogs\step6.log`.
  Only these copies. Steps and expected results:
  1. Right after the launch, while pass 1 runs, right-click `one-date` and
     pick `Rename by Capture Date…`: no dialog opens and the pane says `a
     scan is running; wait for it to finish`.
  2. Once the scan ends, judge two files (stars, a pick), move one other RAW
     to the Trash from the strip's menu, and page to the fourth file.
  3. `Rename by Capture Date…` on `one-date`: the dialog says `Rename
     one-date to YYYY-MM-DD?` and one row `YYYY-MM-DD  n files`; `Run` is
     outlined, Tab moves to `Cancel`, Escape closes it.
  4. Again, then `Run`: the buttons dim, the running note shows, the dialog
     closes, the tree row now reads `YYYY-MM-DD` in the same place (its
     parent's children re-sorted), the strip shows the same files with the
     same judgments on the fourth file, and `step6.log` shows `datefolder:
     … renamed to …` followed by `scan reconcile: dir=…\YYYY-MM-DD todo=0`
     (no pass 1 work).
  5. `Ctrl+Z`: the RAW moved to the Trash in step 2 comes back into the
     renamed folder (`Restored 1 file from the Trash`).
  6. Open `collide`, wait for its scan, pick the item and note the date in
     the dialog, Cancel; create an empty folder of that `YYYY-MM-DD` name
     next to `collide`; the item again: the dialog shows `YYYY-MM-DD already
     exists next to collide; nothing is merged.`, `Run` is dimmed and
     `Cancel` focused.
  7. Open `three-dates`, wait for its scan, then the item: `Split
     three-dates into 3 folders by capture date?`, three rows, `Without a
     capture date, 1 file stays in three-dates: undated.jpg` and
     `three-dates is removed if it ends up empty.`. `Run`: three sibling date
     folders appear in the tree, the pane says `Split three-dates into …
     (a), … (b), … (c); three-dates kept (1 left)`, and the strip re-lists
     `three-dates` with only `undated.jpg`.
  8. Open `two-dates`, wait for its scan, then the item and `Run`:
     `two-dates` leaves the tree, the first date folder opens with its
     files, and the pane says `Split two-dates into … (a), … (b)`.
  9. The item on the renamed `one-date` folder: `YYYY-MM-DD is already named
     its capture date.`, `Nothing to rename.`, and `Run` dimmed.
- **Pending manual check (Step 7, Windows, the user's).** Basis: Step 7's
  Done-when; the inline editor, its keys, the Japanese IME in it, the slow
  click and the held rename cannot be driven headless. The step's checkbox
  was ticked on the automated criteria (`rename.rs`, `idle.rs` and
  `context.rs` tests, `cargo clippy`, `cargo test` in `crates/native`).
  Setup, only under `D:\Photos\tests\20261113-gpui-folder-dialogs\step7\`:
  - `Photos\`: a few dozen RAWs (enough for pass 1 to run for several
    seconds on a fresh index), a few with a `.xmp`; `Photos\sub\` with a
    couple of RAWs.
  - `taken\`: an empty folder, a sibling of `Photos`.
  - `other\`: a few RAWs.

  `cargo build` in `crates/native` (rename a locked `riffle-native.exe`
  aside, never kill it). Delete `index-step7.sqlite` and
  `settings-step7.sqlite` first, then run
  `crates\native\target\debug\riffle-native.exe D:\Photos\tests\20261113-gpui-folder-dialogs\step7\Photos --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step7.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step7.sqlite --log D:\Photos\tests\20261113-gpui-folder-dialogs\step7.log`.
  Only these copies. Steps and expected results:
  1. Wait for the scan to end, judge two files and page to the third. Right
     click `Photos`, `Rename…`: the row's name becomes a field holding
     `Photos`, all selected. Type `Shots` and Enter: the row reads `Shots` in
     the same place, the strip keeps its files, judgments and position, the
     tree keeps the keyboard (the arrows move its cursor), and `step7.log`
     shows `rename: …Photos renamed to …Shots` with no pass 1 work after it.
  2. `Rename…` again, type `shots` (case only) and Enter: the row reads
     `shots`.
  3. `Rename…`, type `a/b`, Enter: the status says `a/b: a name cannot
     contain '/'` and the row keeps its name. Again with `taken`: `taken
     already exists`.
  4. `Rename…`, type something, Escape: the name is unchanged, nothing is
     renamed. `Rename…`, clear the field, Enter: nothing happens. While the
     field is open, `a`, `1`, `p`, `Ctrl+Z`, the arrows and Space edit the
     text only (no judgment, no paging, no tree move); `Ctrl+A`, `Ctrl+C`,
     `Ctrl+X`, `Ctrl+V` work in the field.
  5. `Rename…`, switch to another app with Alt+Tab and back: the field is
     still open. Click the strip: the edit confirms.
  6. With the Japanese IME on, `Rename…`, type `しゃしん` and convert to
     `写真`: the composition is underlined in the field, the candidate window
     sits at the field, Enter inside the composition commits the conversion
     only, a second Enter renames the folder to `写真`.
  7. Quit, delete `index-step7.sqlite`, launch again with the same command
     (the folder as renamed in step 6). During pass 1, rename the open
     folder to `Later`: the row shows `◷ Later` in muted italics (hovering
     says `Renames when the scan finishes`), the pane says `Rename…: waiting
     for the scan to finish`, the scan goes on; when it ends the folder is
     renamed and reopened.
  8. Again during a fresh scan (delete the index, relaunch): rename to
     `Later2`, then `Rename…` on the pending row and type the real name
     back: the row shows the real name and the status says `Rename… was
     canceled`. Rename to `Later3` and, still during the scan, click `other`
     in the tree: `other` opens and the status says `Rename… was canceled`;
     the first folder keeps its name.
  9. Open `Photos`'s renamed folder, wait, then click its name once and wait
     half a second: the field opens. Double-click the name: no field opens.
     Click it once and press a key or click elsewhere within half a second:
     no field opens. A single click on a root's name or on another folder's
     name opens that folder as before.
  10. While an import runs (Step 5's check), confirm a rename: the status
      says `Rename… is unavailable while an import runs`.
