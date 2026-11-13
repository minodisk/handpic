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

# The folder operation dialogs in the GPUI app

## Purpose

This is part 2 of milestone Step 8 of the GPUI port
(`../20261109-gpui-port/plan.md`). Part 1
(`../20261113-gpui-tree-trash/plan.md`) gives the native app the folder tree,
its right-click menu, the strip's menu, Move to Trash and the scan-idle gate.
This plan ports the folder operations opened from the tree's right-click
menu:

- **Rename…**: the folder's inline rename in the tree.
- **Rename by Capture Date…**: rename a folder to its one capture date or
  split it into one sibling folder per date.
- **Import…**: copy every photo under a folder into the date folders of the
  import settings, with progress and cancel.
- **Sequence JPEG Timestamps…**: write copies of a folder's JPEGs with unique
  capture seconds, with progress and cancel.
- **Rewrite Sidecars from Index…** / **Delete Sidecars…**.

Each has its preview dialog, its progress display, and the Tauri app's rule
for a press during a scan. Once it is done, port Step 8 is complete (given
part 1 has landed) and the native app no longer needs the Tauri app for any
folder operation.

### What the investigation found (2026-11-13, `main` at `cc6db501`)

- **Part 1 and the settings plan are not on `main` yet.** This plan is
  written against what `../20261113-gpui-tree-trash/plan.md` and
  `../20261113-gpui-settings/plan.md` say they land. No step of this plan
  starts before part 1 is merged. What this plan uses from part 1:
  `crates/backend/src/folders.rs` (`is_hidden`, `REVEAL_LABEL`), `listing::kind`,
  `crates/backend/src/treewatch.rs` (`TreeWatch`, `with_released`), the
  backend's `trash_one` and `Runs`; in the native crate `tree.rs`,
  `context.rs` (`folder_groups`, the menu popover), `idle.rs` (`IdleGate`,
  `cancels_scan`), `trash.rs` (`TrashFlow`, the modal dialog layer,
  `opens_target`, `shown_path`), `history.rs`'s trash-run entry, the
  non-joining scan cancel, and the rule that no listing or scan starts while
  an operation is in flight.
- **The Tauri sources and their Tauri coupling.**
  - `crates/app/src/rename.rs` (442 lines): `check_name`, `folder_target`,
    `FilePlan`, `file_plan`, `file_run` are pure. `rename_folder_blocking`
    flushes the writer, takes the `Scans` lock and refuses a running scan,
    stops the embedding pass, releases the folder watcher
    (`watch::release_under`) and the tree watches (`treewatch::with_released`)
    around `std::fs::rename`, puts the watcher back on failure, then
    `Index::rename_dir` and `Runs::rename_dir`; an index failure after a
    successful rename is a warning, not an error.
  - `crates/app/src/datefolder.rs` (876 lines): `date_of`, `plan`, `dated`,
    `preview`, `move_plan`, `split`, `remove_if_empty` are pure or disk-only.
    `plan_of` reads the capture times from the index (`Index::entries`),
    else from the file head. `split_blocking` has `rename_folder_blocking`'s
    guards, then `Index::move_file` per moved file and `Index::forget_dir`
    when the folder was removed. `date_folder_run` plans again and refuses a
    collision or an already named folder.
  - `crates/app/src/import.rs` (1528 lines): the settings pieces
    (`DEFAULT_PATTERN`, `ImportSettings`, `validate_pattern`, `format_date`,
    `check_segment`, `root_setting`, `pattern_setting`, `example`), then
    `walk`, `plan`, `refusal`, `summarize`, `plan_dir`, `prepare`,
    `run_plan`, `log_line`, all Tauri-free and on `chrono`. Tauri-bound:
    `AppImportSettings`, `load_settings` / `save`, `ImportPreviews` (the one
    previewed plan kept by id), `Imports` (the one run, its cancel flag and
    a `tauri::async_runtime::JoinHandle`), the commands and the
    `import-progress` / `import-done` emits. The run does not touch the index
    and does not check for a scan.
  - `crates/app/src/sequence.rs` (371 lines): `preview(dir)` and the mapping
    of `riffle_core::sequence::run`'s summary to the done payload are
    Tauri-free; `Sequences` (run state with a `JoinHandle`) and the emits are
    not. It touches neither the index nor the scan.
  - `crates/app/src/foldersidecars.rs` (820 lines): `targets`, `preview`,
    `rewrite` (mark dirty, `Writer::set_now`, flush up to `REWRITE_DRAIN`,
    10 minutes), `doomed`, `delete_preview`, `delete` take the index, the
    writer, the format and the names as parameters already. The commands add
    the writer flush, the `Scans` refusal, `Runs::record` and
    `Index::clear_judgments`. It uses `format_bytes`, `SIZE_BASE`,
    `SCAN_RUNNING` and `trash_one` from `commands.rs`.
  - `chrono` is a dependency of `crates/app` and `crates/core`, not of
    `crates/backend`.
- **The frontend's models** (pure, each with a `.test.ts`):
  `datefolder.ts` (`dateTitleLine`, `dateRows`, `undatedLine`, `removalNote`,
  `disabledReason`, `splitStatus`, `parentOf`, `DateFolderFlow`),
  `import.ts` (`sizeText`, `totalLine`, `folderRows`, `collisionItems`,
  `importNote`, `canRun`, `progressStatus`, `failureText`, `doneStatus`, the
  headings and notes, `ImportFlow` with `reopen` / `aside`), `sequence.ts`
  (`rowParts`, `rebuildNotice`, `writtenLine`, `captureTimeLine`,
  `failureText`, `progressStatus`, `folderError`, `doneStatus`,
  `revealAfter`, `SequenceFlow`), `sidecars.ts` (`formatName`, `rewriteRows`,
  `rewriteTotalLine`, `rewrittenStatus`, `deleteRows`, `deleteTotalLine`,
  the running notes, `SidecarFlow`), `rename.ts` (`editKey`, `confirmName`,
  `PendingRename`, `displayName`, `pendingOutcome`, `editOutcome`,
  `stemLength`, `commit`, `SlowClick`, `RenamesInFlight`, `PENDING_TITLE`),
  and in `tree.ts` `rebase` and `renameFolder`.
- **The rule of each operation during a scan** (`crates/app/ui/src/main.ts`,
  `idle.ts`):
  - Rename: held by the gate under the label `Rename…`, the new name shown
    pending on the tree row (`holdRename`); a re-edit replaces the held
    rename, typing the real name back cancels it, a folder switch drops it
    with `Rename… was canceled`; refused with `Rename… is unavailable while
    an import runs` while an import is busy, checked again when the held
    rename runs. It does not cancel the scan.
  - Rewrite Sidecars from Index / Delete Sidecars: held by the gate under
    those labels; the preview is read once the scan has ended. They do not
    cancel the scan. The run itself is refused with `SCAN_RUNNING` if a scan
    started while the dialog was open.
  - Rename by Capture Date: not held. The preview and the run are refused
    with `SCAN_RUNNING` ("the plan could be stale by then").
  - Sequence JPEG Timestamps and Import: neither wait for the scan nor
    touch it.
  - `cancelsScan` is true only for `Move to Trash` and `Delete Permanently`,
    so no operation of this plan cancels a scan.
  - An operation's invoke is marked in flight (`settleIdle`) so a resync
    waits for it: Rename, the date run, the two sidecar runs.
  - One flow at a time: each start does nothing while the first-run format
    dialog or the settings modal is open or while the trash flow or another
    folder flow is busy. The one exception is `Import…` while an import runs
    with its dialog aside, which brings the dialog back.
- **What follows a run** (`main.ts`):
  - A folder rename (`folderRenamed`): the tree renames the node in place
    (`tree.ts`'s `renameFolder`), the tree's own state and the trash entries'
    folders are rebased (`rebase`, `mapTrashDirs`), and when the open folder
    is the renamed one or under it, it is reopened under the new path; the
    warning, if any, goes to the status.
  - A split: failures go to the sticky errors (`<name>: could not move:
    <message>`), the status says `splitStatus`, the parent is re-listed in
    the tree, and when the split folder was the open one it is re-listed if
    it remains, else the first date folder opens.
  - A sidecar rewrite: the status says `rewrittenStatus`; the strip is left
    alone. A sidecar delete: `trashed(summary, [dir], "sidecars", false)`:
    the shown judgments are cleared when the open folder is the target, the
    run goes on the undo history as a trash entry with `what: "sidecars"`,
    the redo history is cleared.
  - A sequence run: failures to the errors, `doneStatus` to the status, and
    the output folder is revealed in the file manager (`revealAfter`).
  - An import: once the run has started its first destination folder is
    opened and the dialog steps aside; progress shows in the pane's notes
    (`importing`), the result stays there (`imported`); at the end the open
    folder is resynced when it is one of the run's folders.
- **Every dialog** takes all keys while open: Escape dismisses (cancels a
  cancelable run, does nothing during a run that cannot be canceled), Tab /
  Shift+Tab cycle between the enabled buttons, and the focus starts on the
  run button, or on Cancel when the run is disabled.
- **The native app today**: `crates/native/src/main.rs` (2464 lines) holds
  one `Arc<watch::Watch>` set on every listing, `Scan` / `Running`, the one
  event channel (`scan::Event`), `relist`, `open`, the pane's notes, and the
  `deferred(anchored())` popovers of `menu.rs`. `settings.rs` reads the Tauri
  `settings.json` read-only into `Settings { format, names, keymap,
  auto_advance, sort, filter }`; it has no import fields. The settings plan
  replaces that with a SQLite store (`--settings <sqlite>`, seeded once,
  `--import-settings <json>` for a test seed), adds `textinput.rs` (its
  Step 6) and adds the import root and pattern to `Settings` with the Import
  tab (its Step 7), moving the import settings pieces into
  `crates/backend/src/import.rs`.
- `native-app.md` records that `watch::release_under` / `settle` on
  `notify` 8.2.2 (the native lock) is unverified and says to re-check it
  "when the folder rename (port Step 8) needs it".

### Rules every step follows

- `crates/native` must not depend on `crates/app`. The Tauri-free planning
  and run logic moves into `crates/backend` behind parameters and callbacks,
  with no `tauri`, no event name and no `AppHandle`; the Tauri commands stay
  as thin wrappers that keep the writer flush, the `Scans` lock and its
  refusal, the managed state and the emits. No Tauri smoke checks and no
  compatibility shims. Root `mise run ci` and `mise run native:check` pass
  on every step (in bash, prefix
  `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH"`).
- **No A/B latency measurement.** `native-app.md`'s "A same-session A/B
  gates only work on the page path" puts dialogs, context menus, file
  operations and code moves into the backend on the side that needs none.
  Nothing here is drawn or run per page or per frame: a dialog is laid out
  only while open, a progress note is a pane line present only while its
  operation runs, and a held operation costs one check at the scan's end. A
  step that finds it has to add an always-drawn part says so before doing
  it.
- Other sessions edit `crates/native/src/main.rs` and the `crates/native`
  paragraph of `CLAUDE.md` at the same time. The logic goes into new
  modules; `main.rs` gains only the lines each step names, and `CLAUDE.md`
  one short clause per new module. Re-check `origin/main` at the start of
  each step for a piece another session already moved or built. A conflict
  with `main` is resolved with a merge commit.
- Pure models (the dialogs' text and flows, ports of the `.ts` files) live
  in their own modules with unit tests ported from the matching `.test.ts`;
  the GPUI elements and the wiring are apart from them.
- Reuse part 1's mechanisms rather than adding parallel ones: its
  `IdleGate` and the drain at `FacesDone`, its in-flight rule for the
  deferred resync, its modal dialog layer and key routing, its menu popover.
- Disk work runs on `cx.background_spawn` (or a thread of its own for a
  long cancelable run), never on the main thread. A progress callback from
  another thread only sends into the view's one event channel.
- **Manual checks.** Only on copies under
  `D:\Photos\tests\20261113-gpui-folder-dialogs\`, the import destination
  root inside it too; never open another folder under `D:\Photos\`. Always
  pass an explicit fresh `--index` and an explicit test `--settings` (with
  `--import-settings <test json>` for a seed once the settings store has
  landed); never the default index, the default store or the real Tauri
  `settings.json`. `cargo build` before launching the exe; rename a locked
  `riffle-native.exe` aside, do not kill it. GUI checks are the user's: each
  step runs only automated checks and records its GUI check in
  `learnings.md` as `Pending manual check (Step N, Windows, the user's)`
  with the exact command, the steps and the expected result.
- Docs move with the code: each step updates `docs/agents/native-app.md`
  with what it learned. Everything committed is in English, Conventional
  Commits.

## Steps

- [x] Step 1: Move the Tauri-free parts of the folder rename, the date rename and the folder sidecar operations into the backend
  - Done when:
    - `crates/backend/src/rename.rs` holds `Renamed` (fields public,
      `Serialize` kept), `check_name`, `folder_target`, `FilePlan`,
      `file_plan`, `file_run` and the body of the folder rename after the
      scan check: canonicalize, release the folder watcher and the tree
      watches around `std::fs::rename`, put the watcher back on failure,
      `Index::rename_dir` (a failure becoming the warning) and
      `Runs::rename_dir`, taking the `&Watch`, the `&TreeWatch`, the index
      and the `&Runs` as parameters. Their tests move with them.
      `crates/app/src/rename.rs` keeps the two commands and
      `rename_folder_blocking` as wrappers that flush the writer, hold the
      `Scans` lock, refuse a running scan and stop the embedding pass, then
      call the backend. `rename_file` stays a command over the moved
      `file_plan` / `file_run`.
    - `crates/backend/src/datefolder.rs` holds `date_of`, `Kind`, `Group`,
      `Plan`, `plan`, `dated`, `Target`, `Preview`, `preview`, the planning
      of a canonical folder from the index's capture times (the index passed
      in), `move_plan`, `split`, `remove_if_empty`, `Moved`, `Split`, `Ran`,
      and the split's body after the scan check (the watcher releases, the
      moves, `Index::move_file` / `forget_dir`, the warning). The fields a
      dialog reads (`Preview`, `Target`, `Moved`, `Split`) are public.
      `crates/app/src/datefolder.rs` keeps `date_folder_preview` /
      `date_folder_run` and the guards.
    - `crates/backend/src/foldersidecars.rs` holds `REWRITE_DRAIN`,
      `Preview`, `Summary`, `targets`, `preview`, `rewrite`, `KindCount`,
      `DeletePreview`, `doomed`, `delete_preview`, `delete`, and one
      function for the delete run's body (collect, move with `trash_one`,
      `Runs::record`, `Index::clear_judgments`), with public fields where a
      dialog reads them. `crates/app/src/foldersidecars.rs` keeps the four
      commands with the writer flush and the `Scans` refusal.
    - `SCAN_RUNNING`, `format_bytes` and `SIZE_BASE` are public in the
      backend, with `format_bytes`'s tests; `commands.rs` uses them from
      there.
    - Behaviour is unchanged; every test of the three modules passes where
      it now lives. `mise run ci` and `mise run native:check` pass.
      `crates/native/Cargo.lock` is unchanged. `CLAUDE.md`'s backend
      sentence names the three modules and its `crates/app` sentence says
      the commands are wrappers.
  - Implementation approach:
    - Assumes part 1's Step 1 merged (the backend's `folders.rs`,
      `treewatch.rs` with `with_released(&TreeWatch, ..)`, `trash_one`).
    - Follow part 1's Step 1 and `../_archived/20261110-gpui-judgments/plan.md`
      Step 1: moves and call sites only, the moved items named in the PR.
    - The backend functions do not check for a scan: the Tauri wrappers keep
      the `Scans` lock held across the call, the native app guards with its
      gate (Steps 3, 6, 7). Say so in each function's doc comment.
    - The step decides the shape of the shared rename body's parameters (a
      small context struct or plain arguments); the split's body takes the
      same set. The index is optional in both apps.
    - `#[cfg(test)]` does not cross a crate boundary (`native-app.md`): a
      helper the app's remaining tests use loses its `#[cfg(test)]` or is
      copied.
    - The step picks the module `SCAN_RUNNING` and `format_bytes` live in;
      nothing else moves out of `commands.rs`.
- [x] Step 2: Move the import's and the sequencer's Tauri-free parts into the backend
  - Done when:
    - `crates/backend/src/import.rs` holds, beside the settings pieces
      (`DEFAULT_PATTERN`, `ImportSettings`, `validate_pattern`,
      `format_date`, `root_setting`, `pattern_setting`, `example`): `Found`,
      `Photo`, `walk`, `Decision`, `Planned`, `Destination`, `Plan`, `plan`,
      `FolderSummary`, `Totals`, `Preview`, `refusal`, `summarize`,
      `plan_dir`, `Failure`, `Ran`, `prepare` (which creates the destination
      folders and returns them), `run_plan` (cancel flag and progress
      callback) and `log_line`, with their tests and public fields where a
      dialog reads them. If the settings plan's Step 7 has already moved the
      settings pieces, this step extends that module and moves only the
      rest.
    - `crates/backend/src/sequence.rs` holds `Failure`, `Row`, `Preview`,
      `preview(dir)` and one function that runs
      `riffle_core::sequence::run` with a cancel flag and a `(done, total)`
      callback and returns what the done payload carries (`output_dir`,
      `written`, `total`, `failed`, `canceled`, a folder-level error as the
      one failure).
    - `crates/app/src/import.rs` keeps `AppImportSettings`, the load / save,
      the settings commands, `ImportPreviews`, `Imports`, `import_preview`,
      `import_run`, `import_cancel` and the two emits; `crates/app/src/sequence.rs`
      keeps `Sequences`, the three commands and the two emits. Both call the
      backend.
    - `chrono` is a dependency of `crates/backend` (unless the settings
      plan already added it); `crates/native/Cargo.lock` follows.
    - Behaviour is unchanged; `mise run ci` and `mise run native:check`
      pass; `CLAUDE.md` names the backend's `import.rs` and `sequence.rs`.
  - Implementation approach:
    - Assumes Step 1 merged (`datefolder::date_of` in the backend) and part
      1's Step 1 (`listing::kind`, `folders::is_hidden`).
    - The run states stay per app: the Tauri ones hold a
      `tauri::async_runtime::JoinHandle`; the native app keeps its own cancel
      flag (Steps 4 and 5). Only the Tauri-free bodies are shared.
    - `ImportPreviews` (the previewed plan kept by id so the run copies
      exactly what was shown) stays in the app; the native app keeps the
      `Plan` next to its flow instead.
    - Independent of Step 1 in content; kept apart from it for review size
      (about 1,900 lines move here, about 2,100 there).
- [ ] Step 3: The folder menu's items, the shared dialog, and Rewrite Sidecars from Index… / Delete Sidecars…
  - Done when:
    - `context.rs`'s `folder_groups` offers part 2's items in the Tauri
      order with `context.test.ts`'s cases: `Rename…` and `Rename by Capture
      Date…` (not on a root, before `Expand All`), then `Rewrite Sidecars
      from Index…` / `Delete Sidecars…`, then `Sequence JPEG Timestamps…`,
      then `Import…`. An item whose step has not landed is present and
      disabled.
    - `crates/native/src/dialog.rs` is the pure model of the confirmation
      dialog all five operations share, with unit tests: its content (a
      title, text lines, list rows, a note, a reason, the running note), the
      run button (label, primary or destructive, enabled) and Cancel, which
      of the enabled buttons has the focus (the run button, or Cancel when
      the run is disabled), and the key rule (Escape asks to dismiss, Tab /
      Shift+Tab cycle the enabled buttons, Enter presses the focused one,
      every other key is swallowed). It is drawn by one function as a
      `deferred`, occluding, full-window layer; a long row list scrolls
      inside the dialog.
    - `crates/native/src/folderops.rs` owns the folder operations' view
      state and wiring: which flow is under way, the one-flow-at-a-time rule
      (no start while the trash flow, another folder flow or, once it
      exists, the settings modal is busy), the open dialog, the progress
      notes, and the dispatch of a folder menu action.
    - `crates/native/src/sidecars.rs` is a port of `sidecars.ts` with
      `sidecars.test.ts`'s cases: `format_name`, `rewrite_rows`,
      `rewrite_total_line`, `rewritten_status`, `delete_rows`,
      `delete_total_line`, the two running notes and `SidecarFlow`.
    - `Rewrite Sidecars from Index…` and `Delete Sidecars…` go through part
      1's gate under the labels `Rewrite Sidecars from Index` and `Delete
      Sidecars`: with no scan running the preview is read at once on a
      background task (the judgment stores still queued written and the
      writer flushed first, then the backend's `preview` / `doomed` +
      `delete_preview`), while a scan runs the request is held, shown in the
      pane's notes as waiting, and runs at the scan's end without canceling
      it; a folder switch drops it. A refusal (no RAW, no index rows, no
      sidecars) goes to the status and ends the flow.
    - Confirming runs the backend's `rewrite` (up to `REWRITE_DRAIN`) or the
      delete body on a background task, marked in flight so no listing or
      scan starts under it; the buttons are disabled and Escape does nothing
      while it runs. If a scan started while the dialog was open, the run is
      refused with `SCAN_RUNNING`. After a rewrite the status says
      `rewritten_status` and the strip is untouched. After a delete: when
      the open folder is the target and something moved, the shown judgments
      are cleared; the run is pushed on the undo history as a trash run that
      says it moved sidecars and the redo history is cleared; failures go to
      the pane's notes; undo and redo of that run work through part 1's
      trash undo.
    - `idle.rs`'s tests gain: neither label cancels the scan.
    - `main.rs` gains only: the `folderops` field, one arm routing the
      folder menu's actions to it, one event variant carrying its messages,
      one child in `render`, and the dialog's guard in the key handler next
      to the trash dialog's.
    - Unit tests cover the models. `learnings.md` records the pending manual
      check on a copy with judged RAWs: `Rewrite Sidecars from Index…` shows
      the judged / unjudged / skipped counts and the format, `Rewrite`
      writes the sidecars and reports the count; `Delete Sidecars…` shows
      the per-kind counts and sizes, `Move to Trash` empties the strip's
      marks, `Ctrl+Z` brings the sidecars and the marks back; either item
      pressed during pass 1 on a fresh `--index` shows the waiting note and
      opens its dialog when the scan ends; Escape and Tab behave as above; a
      JPEG-only folder is refused with the RAW-only message.
    - `native-app.md` records the dialog and `folderops` pattern; `CLAUDE.md`
      names `dialog.rs`, `folderops.rs` and `sidecars.rs`; `mise run ci` and
      `mise run native:check` pass.
  - Implementation approach:
    - Assumes Steps 1 and all of part 1 merged.
    - Build on what part 1's Step 7 landed for the trash dialog (the layer,
      the key routing before the tree gate, the focus kept in the model). If
      that dialog is already a reusable piece, use it and make `dialog.rs`
      only the content model; do not rework the trash dialog otherwise.
    - The gate's held operation is whatever type part 1 chose; extend it
      with these operations rather than adding a second gate.
    - The step decides whether `folderops.rs` is an `impl` block on the
      window's view in a child module (the view's private fields and
      `main.rs`'s color constants are reachable from a child module) or a
      GPUI entity of its own as the settings plan's modal; either way a
      background callback only sends into the view's channel.
    - Read `trashed(summary, [dir], "sidecars", false)` and
      `clearShownJudgments` in `main.ts` for exactly what the view drops,
      and `restoredStatus` for how a sidecar run is worded; part 1's trash
      entry may need a `what` field for that.
    - The preview reads the index through the connection the native app
      already has; `rewrite` needs the write connection and the sidecar
      `Writer` the view spawned.
- [ ] Step 4: Sequence JPEG Timestamps…
  - Done when:
    - `crates/native/src/sequence.rs` is a port of `sequence.ts` with
      `sequence.test.ts`'s cases: `row_parts`, `rebuild_notice`,
      `written_line`, `capture_time_line`, `failure_text`,
      `progress_status`, `folder_error`, `done_status`, `reveal_after`, the
      running note and `SequenceFlow` (the phases, `accepts`, `done`,
      `dismiss`).
    - The menu item reads the preview on a background task (the backend's
      `sequence::preview`) and opens the dialog: the source and output
      folders, the rebuild notice when the output exists, one row per file
      with the changed part of its time set apart, the written and
      capture-time lines, the failures; `Run` is disabled when there is no
      row. A preview failure goes to the pane's notes keyed by the folder.
    - `Run` starts the backend's run on a thread with a cancel flag; its
      progress reaches the pane's notes through the event channel
      (`progress_status`); Cancel or Escape during the run sets the flag and
      the dialog closes when the run ends. At the end the failures go to the
      notes, the status says `done_status`, and the output folder is
      revealed (`cx.reveal_path`) unless the run was canceled or wrote
      nothing.
    - It neither waits for a scan nor touches it or the index, and works
      with no folder open. A second start while one is under way does
      nothing. The app's quit does not wait for a run beyond setting its
      cancel flag.
    - Unit tests cover the model. `learnings.md` records the pending manual
      check on a copied folder of JPEGs with duplicate seconds: the preview
      lists old and new times, `Run` shows `sequencing n / m` in the pane,
      `<folder>-sequenced` appears and is revealed in Explorer; a second run
      shows the rebuild notice; Cancel mid-run stops it and reports the
      canceled count; a folder with no JPEG shows its error.
    - `native-app.md` and `CLAUDE.md` (`sequence.rs`) are updated; `mise run
      ci` and `mise run native:check` pass.
  - Implementation approach:
    - Assumes Steps 2 and 3 merged.
    - The Tauri flow's `picking` phase and its early-`done` / `cancelAsked`
      handling exist because the run id comes back after the run's events
      can; the native run's id is known before it starts, so port the flow's
      rules and drop only the cases that cannot occur, saying which in the
      test module.
    - A run's messages carry its id, as the scan's do, so a late message of
      an ended run is dropped.
- [ ] Step 5: Import…
  - Done when:
    - `crates/native/src/import.rs` is a port of `import.ts` with
      `import.test.ts`'s cases: `size_text`, `total_line`, `folder_rows`,
      `collision_items`, `import_note`, `can_run`, `progress_status`,
      `failure_text`, `done_status`, the headings and notes, and
      `ImportFlow` with `reopen`, the aside state and `dismiss(key)`.
    - The dialog reads the import root and pattern from the native
      `Settings` (the settings plan's Step 7). It never writes them and this
      plan has no settings UI. With no root set the preview is the Tauri
      refusal (`the destination folder is not set (Settings > Import)`) and
      `Import` is disabled.
    - The menu item shows `reading <dir>…` in the pane's notes, plans on a
      background task (the backend's `refusal`, `plan_dir`, `summarize`) and
      opens the dialog: the source, the total line, one row per destination
      folder, the collisions and the undated files under their headings, the
      note. The plan shown is the plan run.
    - `Import` creates the destination folders and starts the copy on a
      thread with a cancel flag (`prepare`, `run_plan`); once the folders
      exist the first one is opened in the window and the dialog steps
      aside, so the strip fills through the folder watcher while the copy
      runs. Progress shows in the pane's notes; `Import…` from the tree
      while the run is under way brings the dialog back, where Cancel
      cancels the run and Escape only steps aside again. At the end the run
      is logged with the backend's `import:` line, the failures go to the
      notes, the result line stays in the notes, and the open folder is
      re-listed when it is one of the run's folders.
    - It neither waits for a scan nor touches the index. While an import is
      busy a folder rename is refused (Step 7 uses the flag this step
      provides).
    - Unit tests cover the model. `learnings.md` records the pending manual
      check with `importRoot` set to
      `D:\Photos\tests\20261113-gpui-folder-dialogs\import-dst` in the test
      settings and a copied source tree of RAWs with sidecars and JPEGs
      across two dates plus one undated file: the preview's folders, counts
      and sizes; `Import` opens the first date folder, the strip fills, the
      pane shows `importing n / m` then the result; `Import…` again mid-run
      shows the dialog, Escape hides it, Cancel stops the run; a second
      import of the same source says nothing is left to copy; a same-name
      file of another size is listed as a collision; with no root set the
      dialog shows the refusal.
    - `native-app.md` and `CLAUDE.md` (`import.rs`) are updated; `mise run
      ci` and `mise run native:check` pass.
  - Implementation approach:
    - Assumes Steps 2 and 3 merged, and the settings plan's Step 1 (the
      store). If that plan's Step 7 has not landed, this step adds the two
      fields to `settings.rs`'s `Settings` through the backend's
      `root_setting` / `pattern_setting` and nothing else of that tab; a
      hand check then seeds them with `--import-settings <test json>`.
    - Opening the first folder goes through the same path as a tree click
      (part 1's open and reveal). The Tauri app waits for that open before
      reporting the run's end, because the open clears the notes; check
      whether the native open clears them too and order the two the same
      way.
    - A file landing between the listing and the watch being set is caught
      by the re-list at the run's end; the watcher's deferral during a scan
      (`resync.rs`) is unchanged.
    - The app's quit sets the run's cancel flag; a file in flight is dropped
      under its `TEMP_SUFFIX` name as in the Tauri app. Record what a quit
      mid-run leaves behind.
- [ ] Step 6: Rename by Capture Date…, and what follows a folder rename
  - Done when:
    - `crates/native/src/datefolder.rs` is a port of `datefolder.ts` with
      `datefolder.test.ts`'s cases: `date_title_line`, `date_rows`,
      `undated_line`, `removal_note`, `disabled_reason`, `split_status`,
      `parent_of`, the running note and `DateFolderFlow`.
    - `tree.rs` gains `rebase` and `rename_folder` with the matching cases
      of `tree.test.ts`; the trash entries of both histories can have their
      folders rebased (`undo.ts`'s `mapTrashDirs`, with its test).
    - The view has one "a folder was renamed" path, a port of
      `folderRenamed`: the tree node is renamed in place with its expanded
      state and children, the tree's cursor and the trash entries' folders
      are rebased, the tree watch set follows, and when the open folder is
      the renamed one or under it, it is reopened under its new path with
      its judgments, last viewed file and index rows intact; a warning goes
      to the status.
    - The menu item is not held by the gate: while a scan runs it is refused
      with `SCAN_RUNNING` in the status, at the preview and again at the
      run. Otherwise the preview is planned on a background task and the
      dialog shows the title line, one row per target folder with its file
      count, the undated files, the removal note, and the reason the run is
      disabled (a collision, already named).
    - Confirming runs on a background task, in flight for the resync rule,
      buttons disabled: the queued judgment stores written and the writer
      flushed, then the backend's date run with the view's `Watch`, the
      `TreeWatch`, the index and the `Runs`. A rename ends in the path
      above. A split: failures to the notes (`<name>: could not move:
      <message>`), the status says `split_status`, the parent is re-listed
      in the tree, and when the split folder was the open one it is
      re-listed if it remains, else the first date folder opens.
    - Unit tests cover the models. `learnings.md` records the pending manual
      check on copies: a folder of one date is renamed to it, the tree row
      follows in place and, when it was open, the strip keeps its judgments
      and position without a re-extraction (the log shows no pass 1 work); a
      folder of three dates with one undated file splits into three
      siblings, the undated file stays, the original remains and is
      re-listed; with no undated file the original is removed and the first
      date folder opens; an existing sibling named like a target disables
      the run with its reason; a press during a scan shows the scan message;
      a trash run made before the rename still undoes into the renamed
      folder.
    - `native-app.md` records the rename path and the watcher finding below;
      `CLAUDE.md` names `datefolder.rs`; `mise run ci` and `mise run
      native:check` pass.
  - Implementation approach:
    - Assumes Steps 1 and 3 merged.
    - Verify on `notify` 8.2.2 that `watch::release_under` and the tree
      watcher's `with_released` let Windows rename and remove the folder,
      and that the reopen watches the new path; `native-app.md` marks this
      as unverified. Record the result there.
    - On Windows an open handle inside the folder blocks its rename. Check
      what the native app may still hold when the run starts (a preview or
      thumbnail read in flight, an on-demand face or eye read, the 1:1 crop
      read, a prefetch) and whether the gate's "no scan" answer is enough;
      a refused rename is reported and leaves everything as it was, with the
      watcher put back by the backend.
    - The reopen is the tree-click open of part 1 on the new path; the
      undo histories' judgment batches are keyed by path, so check what the
      Tauri reopen does with them (a folder switch clears them) and do the
      same.
    - This step lands the rename aftermath without the inline editor, so
      Step 7 is only the editor and the hold.
- [ ] Step 7: Rename…: the tree's inline rename, held during a scan; and port Step 8's wrap-up
  - Done when:
    - `crates/native/src/rename.rs` is a port of `rename.ts`'s folder parts
      with the matching cases of `rename.test.ts`: `edit_key`,
      `confirm_name`, `PendingRename` (only the mark that set it clears it),
      `display_name`, `pending_outcome`, `edit_outcome`, `commit`,
      `SlowClick` with `SLOW_CLICK_DELAY`, `RenamesInFlight`,
      `PENDING_TITLE`. `stem_length` and the file kind are left out and the
      test module says so.
    - `Rename…` in the folder menu, and a slow second click on the open
      folder's name, turn the row's name into a text input holding the
      current (or pending) name, selected whole. Enter or losing the focus
      confirms, Escape cancels, each edit deciding once; every other key is
      the input's own and reaches neither the tree's keyboard nor the
      culling keymap. An empty or unchanged name just ends the edit. A
      folder whose rename is in flight, or anything under it, can be neither
      opened nor renamed (`A folder is being renamed; try again in a
      moment.`).
    - A confirmed name goes through the gate under the label `Rename…`: with
      no scan running it runs at once on a background task, in flight for
      the resync rule (the queued stores written, the writer flushed, the
      backend's folder rename, then Step 6's "a folder was renamed" path);
      while a scan runs it is held, the row shows the new name in the
      pending style until it runs, and the scan is not canceled. A re-edit
      of the pending row replaces the held rename, typing the real name
      back cancels it, a folder switch drops it with `Rename… was canceled`.
      While an import is busy a rename is refused with `Rename… is
      unavailable while an import runs`, checked again when a held rename
      runs. A refusal from the backend (an invalid name, an existing entry,
      a root) goes to the status and the row keeps its name.
    - Unit tests cover the model and the gate's `Rename…` cases
      (`idle.test.ts`'s replace-and-cancel). `learnings.md` records the
      pending manual check on copies: rename the open folder from the menu:
      the row follows, the strip keeps its judgments; a case-only rename
      works; `a/b` and an existing sibling's name are refused with their
      messages; Escape leaves the name; a rename confirmed during pass 1
      shows the pending name and runs at the scan's end; re-editing it to
      the real name cancels it; opening another folder meanwhile drops it
      with the canceled note; a slow second click on the open folder's name
      starts an edit and a double-click does not; typing with the Japanese
      IME composes in the field.
    - Wrap-up: `learnings.md` lists every pending manual check of Steps 3-7
      in one place. `CLAUDE.md`'s `crates/native` sentence names `dialog.rs`,
      `folderops.rs`, `sidecars.rs`, `sequence.rs`, `import.rs`,
      `datefolder.rs` and `rename.rs`, and its backend sentence the five
      moved modules (done per step; checked here). `native-app.md` has an
      entry for each of: the shared dialog, the per-operation scan rule, the
      import's run, the folder rename on `notify` 8.2.2, the inline editor.
    - `../20261109-gpui-port/plan.md`: when part 1's line is in its Progress
      (every step of `../20261113-gpui-tree-trash/plan.md` merged), Step 8's
      checkbox is ticked and Progress gains `(YYYY-MM-DD) Step 8 complete:
      the folder operation dialogs, through` this plan's archived path in
      backticks. When part 1 has not fully landed, the checkbox stays open
      and the line says the dialogs are done and what remains.
    - `mise run ci` and `mise run native:check` pass.
  - Implementation approach:
    - Assumes Step 6 merged, and the settings plan's Step 6
      (`crates/native/src/textinput.rs`: set its text, select all, a commit
      signal on Enter and on losing focus). The editor is that entity placed
      over the row; this step adds to it only what the rename needs.
    - Part 1 decides whether the tree pane is a `uniform_list` or a
      `canvas`. On a canvas the editor is a positioned element over the
      row's bounds; on a list it replaces the row's name. Check that the
      row stays laid out while it scrolls or the tree re-lists under a live
      edit (`folders.ts` redraws the editor from its state on every render
      and keeps the selection).
    - The key routing sits before part 1's tree gate and after the dialogs':
      a live edit owns the keyboard. Verify at the pinned GPUI rev that a
      typed character reaches the input and not the tree's type-ahead.
    - The slow click needs a timer (`SLOW_CLICK_DELAY`, 500 ms) disarmed by
      any other click or key, as `folders.ts` does; port its arming rule
      from there rather than from memory.

## Trade-offs and risks

- **What "Rename" covers.** Part 1's plan leaves three things to part 2:
  the tree's inline rename, the strip menu's `Rename…` (one RAW with its
  sidecars, `rename_file`) and the tree's multi-folder selection. This plan
  ports the folder rename only, as its requirements list, and its last step
  ticks port Step 8. Not taken: the strip's `Rename…`, which needs an
  editor over a strip cell (the strip is a canvas), the session's judgments,
  `touched` set and undo entries moved to the new path, and the pending-name
  hold on a cell; it would be one more step. Not taken: the multi-folder
  selection, which nothing consumes (the Tauri folder menu always acts on
  the one folder right-clicked). Decided with the user (2026-11-13): the
  folder rename only; Step 7's wrap-up records both in the port plan as left
  out.
- **The inline rename depends on another session's text input.** GPUI has
  no text input; the settings plan's Step 6 ports one. The rename is
  therefore last. Decided with the user (2026-11-13): if `textinput.rs` is
  not on `main` when Step 7 starts, Step 7 waits for it (Steps 1-6 are
  unaffected) rather than porting a second copy. The user built GPUI's
  `examples/input.rs` at the pinned rev on Windows: the Japanese IME
  composes, the candidate window and commit work, but its editing keys are
  bound only as `cmd-*` (`cmd-a`, `cmd-c`, `cmd-v`, `cmd-x`), so Ctrl+A did
  nothing. The rename's editor needs the `ctrl-*` bindings on Windows and
  Linux; Step 7 checks `textinput.rs` has them and adds them if not.
- **Where the import settings are read from.** Taken: the native `Settings`
  as the settings plan establishes it (the SQLite store; `--settings`
  names the store, `--import-settings` a JSON seed). If that plan's Step 7
  has not landed when Step 5 starts, Step 5 adds the two read-only fields
  to `settings.rs`, a file that session owns, and its Step 7 then finds
  them there. If not even the store has landed, Step 5 would read the Tauri
  `settings.json` through today's read-only path, and its hand-check
  commands would need rewording later, as that plan's Step 1 does for
  others.
- **Both plans move `import.rs`'s settings pieces.** The settings plan's
  Step 7 and this plan's Step 2 each move them into
  `crates/backend/src/import.rs` and add `chrono` to the backend; whichever
  runs second extends the module. A conflict there is expected.
- **Rename by Capture Date is refused during a scan, not held.** That is
  the Tauri rule (the plan could be stale by the scan's end). Holding it as
  the sidecar operations are held would be friendlier and is a behaviour
  change; not taken.
- **The native guard is not the Tauri lock.** The Tauri wrappers hold the
  `Scans` lock across a rename, a split, a rewrite and a sidecar delete.
  The backend bodies do not check for a scan, so the native app relies on
  part 1's gate and in-flight rule; each of Steps 3, 6 and 7 has to show no
  listing or scan can start under its run. The index rows of a folder
  renamed under a running scan would be written under the old path.
- **A rename on Windows can be refused by an open handle.** The watcher and
  the tree watches are released by the backend, but a read the native app
  has in flight inside the folder is not. Step 6 checks what can be open;
  the worst case is a refused rename reported in the status.
- **`notify` 8.2.2.** The release-and-settle path the rename needs is
  unverified on the native lock (`native-app.md`); part 1's Step 3 checks
  the tree watcher's half, Step 6 here the open folder's.
- **The trash dialog and the shared dialog.** Part 1's trash dialog lives
  in `main.rs`. Step 3 adds a shared dialog model; moving the trash dialog
  onto it is not required and is done only if part 1 already made it
  reusable, so two dialog paths may coexist.
- **One-flow-at-a-time across sessions.** The rule also names the settings
  modal and the first-run format dialog; the first exists only once the
  settings plan's Step 3 lands and the second not at all. `folderops`
  checks what exists and gains the modal's check when it lands, in
  whichever session comes second.
- **A long run and quitting.** A rewrite can take minutes
  (`REWRITE_DRAIN`), an import longer. Quitting mid-rewrite leaves dirty
  rows that the next open of the folder retries; quitting mid-import or
  mid-sequence sets the cancel flag and leaves a partial destination, which
  a second import skips over by name and size. Neither blocks the quit.
- **Steps 1 and 2 edit the shipped Tauri app with no smoke check** (the
  port's standing decision). The tests move with the code; a regression is
  handled by reverting the move.
- **The Step 8 tick is conditional.** It assumes part 1 has fully landed;
  Step 7 says what to write when it has not.

## Progress

(2026-11-14) Step 1 complete
