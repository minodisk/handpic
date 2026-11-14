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
