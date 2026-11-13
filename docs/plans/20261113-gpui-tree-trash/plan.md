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

# The folder tree, the strip's selection and its right-click menu in the GPUI app

## Purpose

This is part 1 of milestone Step 8 of the GPUI port
(`../20261109-gpui-port/plan.md`). Today the native app opens one folder, the
one given on the command line (or picked in the OS dialog at launch), judges
one file at a time, and has no way to act on a file other than judging it.

This plan ports from the Tauri app:

- **The left-hand folder tree**: the home and volume roots, a folder's
  subfolders and its RAW count, watchers on the expanded folders, opening a
  folder by clicking it in the tree, the tree's keyboard (cursor, Left /
  Right / Enter, type-ahead) and its right-click menu (reveal, copy,
  `Refresh`, `Expand All` / `Collapse All`). A launch without a folder
  argument opens nothing and shows the tree.
- **The strip's multi-selection**: Ctrl / Shift click, the extend keys,
  `Select All`, and judgments applied to the whole selection as one undo
  batch.
- **The strip's right-click menu**: `Select All`, reveal the file in the OS
  file manager, `Copy Path` / `Copy File Name`, the judgment groups, `Move
  to Trash…` and `Delete Permanently…` with their confirmation dialog and
  progress, the hold of these operations while a scan runs (with the scan
  canceled for a trash or a delete, as `idle.ts`'s `cancelsScan`), and the
  Trash undo / redo.

Part 2 (a later plan) ports the folder operation dialogs (Rename, Rename by
Capture Date…, Import…, Sequence JPEG Timestamps…, Rewrite / Delete
Sidecars…) with the tree's multi-folder selection and inline rename, and the
strip menu's `Rename…`. Port Step 8 stays open until then.

### Decisions (the user, 2026-11-13)

- The strip's multi-selection is in this plan; the strip menu's targets are
  the Tauri `targets(selection, files, index)` and the menu has `Select All`.
- The strip menu includes the judgment groups (flag, stars, label), left
  out in a view-only folder. `Rename…` is part 2's.
- The tree's keyboard is in this plan, with `Expand All` / `Collapse All`
  and the `treeGate`. The tree's multi-folder selection and inline rename
  are part 2's.
- A launch without a folder opens nothing: no OS folder picker, only the
  tree. The window must work with no folder open.
- For manual checks, the tree may list and watch folders outside
  `D:\Photos\tests\` (the roots, the open folder's ancestors); only folders
  under `D:\Photos\tests\` are opened.
- No performance comparison for the tree: this plan has no A/B gate, no
  `results.md` and no line in the performance docs.

### What the investigation found (2026-11-13, `main` at `40e3ec57`)

- **Tauri sources.** `crates/app/src/folders.rs` (`roots(home)`, `list(dir)
  -> Folder { raw_count, children: Vec<FolderNode { name, path, is_link }> }`,
  `REVEAL_LABEL`, `is_hidden`, 7 tests) uses `tauri` only for the home
  directory, `spawn_blocking` and the opener plugin's `reveal_item_in_dir`.
  `crates/app/src/treewatch.rs` (`TreeWatch`, `State::apply` / `add` /
  `remove` / `release_under`, `tree_paths`, the per-path debounce `run`,
  `with_released`, 4 tests) uses `tauri` only to emit `tree-changed` and to
  reach its managed state; it already calls the backend's `watch::triggers`
  and `watch::settle`.
- **The trash commands** (`crates/app/src/commands.rs`): `trash_files`,
  `delete_files`, `trash_rejected_undo`, `trash_rejected_redo` each flush the
  sidecar writer, take the `Scans` lock, refuse while a scan runs
  (`SCAN_RUNNING`) and hold the lock across the moves. Their Tauri-free
  helpers still live in `commands.rs`: `trash_one` (per platform, also used
  by `foldersidecars.rs`), `trash_many` (Windows), `move_groups`,
  `move_again`, `restore_run` (per platform, over the `trash` crate's
  `os_limited`). `crates/backend/src/trash.rs` already has `groups_of`,
  `run`, `run_chunked`, `redo`, `redo_chunked`, `restore`,
  `restore_recorded`, `delete_for_good`, `remove_one`, `Runs` (`record`,
  `take`, `undone`, `take_undone`, `Default`), `Summary { moved, failed,
  run_id }`, `Restored`, `RUN_GONE`, `UNDONE_GONE`. The backend already
  depends on the `trash` crate. `cancel_scan` only sets the scan's cancel
  flag; the scan still ends the normal way (`scan-done`, `faces-done`).
- **The Tauri tree.** `tree.ts` (521 lines, pure, `tree.test.ts` 774
  lines): the `Tree` of roots and nodes, `addRoots`, `expand`, `collapse`,
  `markFailed`, `setChildren`, `drawnChildren`, `rows`, `watchedFolders`,
  `descendants`, `collapseAll`, `ancestorsWithin`, `respell`, `rootOf`,
  `relation`; the keyboard: `step` (`up` / `down` / `home` / `end`),
  `treeKey` (`left` / `right` / `enter` giving `focus` / `expand` /
  `collapse` / `open`, the WAI-ARIA tree pattern), `appendTyped` /
  `typeAhead` with `TYPE_AHEAD_TIMEOUT` 500 ms; and, for part 2, the
  multi-folder `TreeSelection`, `rebase` and `renameFolder`. `folders.ts`
  (830 lines) draws it: an expander click toggles, a click on the name
  opens the folder, the RAW count shows on an expanded folder,
  `reveal(path)` expands the chain down to the open folder at every open,
  the roots are re-read when the window gains focus, `tree-changed`
  re-lists one node, a click in the tree gives it the keyboard and `Escape`
  or a click elsewhere hands the keys back, a right-click neither gives nor
  takes the keyboard, a printable key without Ctrl / Alt / Meta is
  type-ahead, and `expandAll` lists one folder at a time and does not
  descend into a link (`is_link`). `treekeys.ts`: while the tree has the
  keyboard only the actions in `TREE_PASSTHROUGH` (`open`, `toggleLeft`,
  `toggleRight`, `toggleStrip`, `toggleSides`) still run; every other
  action is swallowed.
- **The Tauri selection** (`selection.ts`, 147 lines, pure): `Selection {
  selected, anchor }` keyed by path, the focused file always a member;
  `single`, `selectionOf`, `click` (range from the anchor, toggle, plain),
  `extend`, `all`, `prune`, `settle`, `restore`, `targets` (the focused file
  alone when one or none is selected, else the selected files in `files`
  order) and `judgments(paths, focused, lookup, command, force)`: the value
  is decided once from the focused file, applied to each file's own state,
  focused file first, a file already at its new state skipped unless
  forced. `main.ts`: an arrow key or a plain click selects the landed file
  alone; `refilter` ends with `settle`; a multi-file undo / redo leaves the
  current file and says `Undid N files`; a one-file step that moves the
  focus selects that file (`restore`). The keymap binds `extendPrevious`
  (`shift+arrowleft`), `extendNext` (`shift+arrowright`) and `selectAll`
  (`ctrl+a`, `meta+a` on macOS).
- **The Tauri menus and trash flow.** `context.ts`: `contextMenuGroups`
  (`Select All`; the reveal item; `Copy Path` / `Copy File Name`; the
  judgment groups with their checked state from the focused file, the
  shortcut of each item, and `Rename…`, all left out in a view-only folder;
  `Move to Trash…` / `Delete Permanently…`, naming the count when several
  and disabled while one runs) and `folderMenuGroups` (reveal; `Copy Path`
  / `Copy Folder Name`; `Refresh`; not on a root: the renames and `Expand
  All` / `Collapse All`; then part 2's items). `trash.ts`: `dialogText`,
  `swallowedByDelete`, `confirmsDelete`, `progressStatus`, `trashedStatus`,
  `restoredStatus`, `restoredInto`, `stillHeld`, `opensTarget`, `shownPath`,
  `TrashFlow` (idle / asking / running). `idle.ts`: `IdleGate` (one held
  operation, a later press replacing it, `drain` at the scan's end, an
  in-flight count) and `cancelsScan`. `undo.ts`: an undo entry is a judgment
  batch or a `TrashEntry { runId, count, dirs, what }`; `withoutPaths`,
  `isEmptyBatch`, `undoneTrash`. In `main.ts`, `confirmTrashFiles` opens the
  dialog (the delete dialog focuses Cancel and takes only a single mouse
  click as a yes); `trashFilesIn` runs through the idle gate, and when held
  it closes the dialog, cancels the scan and queues a rescan;
  `trash-progress` updates a note in the pane; `trashed` prunes the view's
  state, pushes the run on the undo history, drops the deleted files'
  judgments from both histories after a delete, reports failures and
  re-lists. `undoTrash` / `redoTrash` go through the gate too, keep their
  entry until the backend answers (`stillHeld`), and an undo puts the
  restored files into `allFiles` at once (`restoredInto`) before the
  re-listing. Judgment batches are dropped when another folder opens; a
  trash entry stays.
- **The native app today** (`crates/native/src/main.rs`, 2462 lines): the
  window is a column of (preview canvas + the 220 px meta pane), the strip
  (spanning the window from its left edge, which `strip_clicked` relies on)
  and the status row. `start` shows `cx.prompt_for_paths` when no folder is
  given. `open(folder, wanted, max)` -> `list(folder, carry)` already stops
  the previous scan, resets the per-folder state when `carry` is false, and
  is only ever called once, from `start`. `Scan` has `stop` (cancel + join)
  and `stop_within`; nothing cancels without joining. `resync.rs` defers a
  watcher change while a listing or scan runs. `history::Undo` holds two
  `History<Batch>` whose `Batch.judged` is already a `Vec<Judged>`, and
  `clear`s both on another folder's open. `judge` acts on the current file;
  `judgment.rs` keeps `changed` apart from `writes` and has
  `should_advance`. The window has one `FocusHandle`; its key handler takes
  `home`, `end` and `q` literally before the keymap's actions, receives the
  `Keystroke` only (`KeyDownEvent.is_held` exists and is not passed on) and
  logs `toggleLeft`, `selectAll`, `extendPrevious` and `extendNext` as not
  handled.
- **GPUI at the pinned rev**: `App::reveal_path(&Path)` (on Windows it
  spawns `open_target_in_explorer`), `App::write_to_clipboard(ClipboardItem)`,
  `on_mouse_down(MouseButton::Right, ..)`, the modifiers on
  `MouseDownEvent`, `ClickEvent::click_count()`,
  `Context::observe_window_activation`, `FocusHandle` with `track_focus`,
  `uniform_list` (vertical, virtualized), and the `deferred(anchored())`
  popover pattern `menu.rs` and `main.rs` already use.
- `notify` resolves to 8.2.2 in `crates/native/Cargo.lock`; `native-app.md`
  records that `unwatch` + `settle` on it is unverified. The tree watcher
  calls both on every collapse.

### Rules every step follows

- `crates/native` must not depend on `crates/app`. A Tauri-free piece the
  native app needs moves into `crates/backend` behind a callback, with no
  event name and no `AppHandle`, and `crates/app` calls it from there. No
  Tauri smoke checks and no compatibility shims; if the Tauri app breaks,
  the move is reverted. Root `mise run ci` and `mise run native:check` pass
  on every step (in bash, prefix `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH"`).
- Pure models in their own modules with unit tests ported from the TS tests;
  the window wiring in `main.rs`.
- Popovers and menus follow the `deferred(anchored())` pattern of
  `crates/native/src/menu.rs` and `main.rs`. `menu.rs`, `filter.rs` and
  `sort.rs` belong to another session: this plan changes only the wiring it
  needs around them.
- Disk work (listing a folder, the roots, the moves to the Trash, the
  restore) runs on `cx.background_spawn`, never on the main thread. A
  callback from another thread (the tree watcher, a progress callback) only
  sends into the view's one event channel.
- Nothing new runs per page or per frame: the tree's rows are rebuilt when
  the tree changes, not in `render`; a menu or a dialog is laid out only
  while open.
- **Manual checks.** Only on copies under
  `D:\Photos\tests\20261113-gpui-tree-trash\`, with an explicit fresh
  `--index` and an explicit `--settings` test file; never the Tauri index or
  the default native index; never modify a file outside `D:\Photos\tests\`.
  The tree may list and watch the roots and the ancestors of a test folder
  (approved); open (click the name of, or press Enter on) only folders
  under `D:\Photos\tests\`. Trash and delete checks only on test copies.
  Never kill a process: rename a locked `riffle-native.exe` aside. `cargo
  build` before launching the exe. GUI checks cannot be driven headless and
  are done later by the user in bulk: each step records its check in
  `learnings.md` as a pending manual check (basis, steps, expected result)
  and the work proceeds; what can be automated is covered by unit tests.
- Docs move with the code: each step updates `docs/agents/native-app.md`
  with what it learned and `CLAUDE.md`'s layout paragraph names the new
  modules. Everything committed is in English, Conventional Commits.

## Steps

- [x] Step 1: Move the folder tree's and the trash commands' Tauri-free pieces into the backend
  - Done when:
    - `crates/backend/src/folders.rs` holds `FolderNode`, `Folder` (fields
      public, `serde::Serialize` kept), `pub fn roots(home: Option<PathBuf>)
      -> Vec<FolderNode>`, `pub fn list(dir: &Path) -> Result<Folder,
      String>`, `pub const REVEAL_LABEL`, `pub fn is_hidden`, the volume
      helpers and their tests, moved from `crates/app/src/folders.rs`. The
      app's `folders.rs` keeps the four commands (`folder_roots` passing the
      Tauri home directory, `list_subfolders`, `reveal_label`,
      `reveal_folder` over the opener plugin) as thin wrappers;
      `import.rs`'s `folders::kind` / `folders::is_hidden` uses follow.
    - `crates/backend/src/treewatch.rs` holds `TreeWatch`, its `State`, the
      per-path debounce and the tests, moved from
      `crates/app/src/treewatch.rs`, with `TreeWatch::spawn(on_changed: impl
      Fn(String) + Send + 'static)` in place of the `AppHandle`, a public
      "make the watched set equal to `dirs`" method, and `with_released`
      taking the `&TreeWatch`. The app's `treewatch.rs` keeps the
      `tree-changed` event name and payload, the `set_tree_watches` command
      and a wrapper for `with_released`'s two callers (`rename.rs`,
      `datefolder.rs`), as `crates/app/src/watch.rs` does for
      `folder-changed`.
    - `crates/backend/src/trash.rs` gains the helpers now in `commands.rs`
      (`trash_one`, `trash_many`, `move_groups`, `move_again`, `restore_run`,
      with their per-platform `cfg`s) and one function per command body that
      needs no app state: the trash of listed files (`read_listing`,
      `groups_of`, the move, the foreign failures, `Runs::record`), the
      delete (`delete_for_good` over the listing), the undo of a run (`take`,
      `restore_run`, `undone`) and the redo (`take_undone`, `move_again`,
      `record`), each taking `&Runs` and a `progress: impl FnMut(usize,
      usize)` where the command emits one. The four Tauri commands keep the
      writer flush, the `Scans` lock and its `SCAN_RUNNING` refusal, the
      `trash-progress` emit and the log line, and call these.
      `foldersidecars.rs` uses the backend's `trash_one`.
    - Behaviour is unchanged; the tests move with the code. `mise run ci` and
      `mise run native:check` pass. `crates/native/Cargo.lock` is unchanged
      (the backend gains no dependency). `CLAUDE.md` names the backend's
      `folders.rs` and `treewatch.rs`.
  - Implementation approach:
    - This is the only step that edits `crates/app`; keep it to the moves
      and the call sites, and name the moved items in the PR (the pattern of
      `../_archived/20261110-gpui-judgments/plan.md` Step 1).
    - `#[cfg(test)]` does not cross a crate boundary: a helper the app's
      remaining tests still use must lose its `#[cfg(test)]` or be copied
      into the app's test module (`native-app.md`).
    - The reveal itself is not shared: the Tauri app keeps the opener
      plugin, the native app uses GPUI's `reveal_path` (Step 3).
- [x] Step 2: The tree model and the left pane: roots, expanding a folder, the RAW count, opening a folder from the tree, a launch with no folder
  - Done when:
    - `crates/native/src/tree.rs` is a pure model with unit tests, a port of
      the tree's state from `tree.ts`: the `Tree` (roots, nodes with
      `expanded`, listed children, `raw_count`, `failed`, `is_link`),
      `add_roots`, `expand`, `collapse`, `mark_failed`, `set_children` (a
      known child keeps its state), `drawn_children` (a folder that is
      itself a root is drawn as that root only), `rows` (depth first, with
      depth), `watched_folders`, `ancestors_within`, `respell`, `root_of`,
      `relation` and the path normalization they share, with the matching
      cases of `tree.test.ts`. The keyboard functions come in Step 4; the
      multi-folder selection, `rebase` and `renameFolder` are part 2's and
      the test module lists them as left out.
    - The window gains a left pane, 220 px wide (the Tauri `#side`), beside
      the preview canvas and above the strip, which keeps spanning the
      window. It lists `tree::rows`: indentation by depth, an expander, the
      folder's name, the RAW count on an expanded folder that has one, the
      open folder highlighted, a folder that failed to list marked with its
      error. Only the rows on screen are laid out.
    - At startup the roots are read on a background task
      (`riffle_backend::folders::roots(dirs::home_dir())`). An expander
      click expands (listing the folder on a background task through
      `folders::list`, the cached children drawn meanwhile) or collapses; a
      listing error marks the folder and goes to the pane's notes.
    - A click on a folder's name opens it through the existing `open` path,
      with no `--size` / `--max` narrowing (those belong to the command-line
      folder). Every open, from the command line or the tree, reveals the
      folder in the tree as `folders.ts`'s `reveal` does: expand and list
      the chain from its root (adding a root for a folder no root holds),
      respell the chain from the listings, highlight it and scroll it into
      view; a second open while the chain is still listing owns the tree
      (the `opens` counter is the folder token, not `listing`, which every watcher re-listing bumps and would cut a reveal short).
    - A launch without a folder argument opens nothing: `start` no longer
      calls `cx.prompt_for_paths`. With no folder open the window shows the
      tree, an empty preview, an empty strip and `no folder` on the status
      line, and every key, click, wheel, menu and timer path is a no-op
      rather than a panic (the paging keys, the judgment keys, undo / redo,
      `f`, `z`, `e`, `home` / `end`, the sort and filter menus, a strip
      click and scroll, `--auto`). `q` and closing the window quit.
    - Opening a second folder in one process resets everything the first
      left: check each per-folder field of the view against `opened`'s
      non-`carry` branch (judgments, `touched`, notes, write errors, the
      `f` overlay's caches, zoom, peaking, the dwell, the viewed batches,
      `last_viewed`) and record what had to be added.
    - The keymap's `toggleLeft` action hides and shows the pane.
    - Unit tests cover the model and any pure helper of the pane (which row
      a click lands on, if the pane is a canvas). Pending manual check
      recorded in `learnings.md`: launch with no folder: only the tree, no
      dialog, the keys do nothing, `q` quits; launch on a copy under
      `D:\Photos\tests\20261113-gpui-tree-trash\tree\a`: the chain down to
      it is expanded and highlighted with its RAW count; expand and collapse
      a sibling; click sibling `b`: the strip, the preview, the pane and the
      status follow, `a`'s scan stops, and `a` reopened resumes at its last
      viewed file; the `toggleLeft` key hides and shows the pane.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `tree.rs`; `native-app.md` records how the pane is drawn and why, the
      preview canvas's new size in the default window, and that the folder
      picker is no longer shown at launch (its "native folder picker" entry
      is updated, `args::USAGE` too if its wording implies the picker).
  - Implementation approach:
    - Assumes Step 1 merged.
    - The step decides between `uniform_list` (vertical and virtualized, so
      a folder with hundreds of subfolders costs only its visible rows) and
      a `canvas` over a pure model as the strip. `native-app.md` rules out
      `uniform_list` for the horizontal strip only. A flex `div` with one
      child per row lays out every row each frame and is not an option for
      a large folder. Step 4 needs to scroll a given row into view, so pick
      the one that can.
    - Keep the rows in the view, rebuilt when the tree changes; do not call
      `tree::rows` from `render`.
    - Tree node keys are the tree's own spelling (the root's, then one
      separator per level), not the canonical `\\?\` path the index uses;
      compare the open folder with a node through `relation` /
      `ancestors_within` with case ignored on Windows and macOS, never by
      string equality with `self.dir`.
    - The strip's `strip_clicked` takes the event's window `x` as the
      strip's `x`; keep the strip spanning the window so that stays true.
    - A canceled picker already left the view with `dir: None`, so most
      paths tolerate it; the step walks every handler once rather than
      trusting that.
- [x] Step 3: The tree follows the disk, and its right-click menu
  - Done when:
    - The view holds one `riffle_backend::treewatch::TreeWatch` whose
      callback sends a new `Event::TreeChanged(String)` into the scan's
      channel. Whenever `tree::watched_folders` changes, the whole set is
      handed to the watcher off the main thread, one call at a time and
      nothing sent when the set is unchanged (the Tauri `set_tree_watches`
      chain). `TreeChanged` re-lists that node when it is still expanded and
      drawn; a failed re-list leaves it as it was.
    - The roots are read again when the window becomes active
      (`observe_window_activation`), `add_roots` skipping the ones already
      shown, a failure not reported.
    - `crates/native/src/context.rs` is a pure model with unit tests: the
      menu item type (action, label, shortcut, checked, disabled),
      `folder_groups(reveal_label)` with the reveal item
      (`riffle_backend::folders::REVEAL_LABEL`), `Copy Path`, `Copy Folder
      Name` and `Refresh`, and the open menu's state (where it was opened,
      what it targets).
    - A right-click on a tree row opens that menu at the pointer as a
      `deferred(anchored().position(..).snap_to_window())` popover;
      `escape` or a press outside closes it, as the status row's menus. The
      reveal item calls `cx.reveal_path`; the copy items
      `cx.write_to_clipboard`; `Refresh` re-lists the folder, expanded or
      not, and sends the watch set again.
    - Unit tests cover `context.rs`. Pending manual check recorded in
      `learnings.md`: with `...\tree` expanded, create, rename and delete a
      subfolder in Explorer: the tree follows within about a second without
      collapsing; collapse it and create another: nothing happens until it
      is expanded again; right-click a folder: the reveal item selects it in
      Explorer, `Copy Path` and `Copy Folder Name` paste as expected,
      `Refresh` re-lists.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `context.rs`; `native-app.md` records the watcher findings below.
  - Implementation approach:
    - Assumes Step 2 merged.
    - Verify on `notify` 8.2.2 (the native lock) that collapsing a folder
      (`unwatch` then `settle`) returns promptly and that a later expand
      watches again; `native-app.md` marks this as unverified today. Record
      the result there.
    - Verify what `cx.reveal_path` does with the paths the tree and the
      index hold on Windows (a verbatim `\\?\D:\…` path for a strip file in
      Step 6); if Explorer does not accept the prefix, strip it with a port
      of `trash.ts`'s `shownPath`.
    - On Windows a watch pins the folder's ancestors against a rename
      (`treewatch.rs`'s module comment); this plan renames nothing, so
      `with_released` is not called from the native app yet.
- [x] Step 4: The tree's keyboard: focus, the cursor, Left / Right / Enter, type-ahead, Expand All / Collapse All
  - Done when:
    - `tree.rs` gains, with the matching cases of `tree.test.ts`: `step`
      (`up` / `down` stopping at the ends, `home` / `end`, a cursor not
      among the rows landing on the first), `tree_key` (`left` / `right` /
      `enter` giving `Focus` / `Expand` / `Collapse` / `Open`), `Typed`,
      `append_typed` and `type_ahead` with `TYPE_AHEAD_TIMEOUT`,
      `descendants` and `collapse_all`. `crates/native/src/treekeys.rs` (or
      a section of `tree.rs`) holds `TREE_PASSTHROUGH` and `tree_gate` with
      `treekeys.test.ts`'s cases.
    - The focus model: a click in the tree pane gives the tree the
      keyboard; `escape` or a press anywhere else in the window hands it
      back to the photo; a right-click neither gives nor takes it. While
      the tree has it the cursor row is drawn, and the key handler routes
      as `folders.ts`'s `keydown` then `treeGate`: `escape` hands the keys
      back; the arrows up / down and `home` / `end` move the cursor and
      scroll it into view; left / right / `enter` run `tree_key`'s command
      (move, expand or collapse as the expander click does, open as the
      name click does); a printable key without Ctrl / Alt / Meta is
      type-ahead; any other key runs its keymap action only when
      `tree_gate` says so and is otherwise swallowed. So no judgment, no
      paging, no undo and no `q` reaches the photo while the tree has the
      keyboard. Opening a folder from the tree, by click or `enter`, leaves
      the keyboard where the Tauri app leaves it (check `openFolder` in
      `folders.ts` and record).
    - An open's reveal (Step 2) puts the cursor on the open folder.
    - `context.rs`'s `folder_groups` takes `root` and offers `Expand All`
      and `Collapse All` on a folder that is not a root. `Collapse All`
      collapses every folder under the target; `Expand All` expands the
      target and every subfolder under it, one listing at a time, stopping a
      branch whose chain was collapsed or dropped meanwhile, marking and
      reporting a folder that fails and going on, and never descending into
      a link (`is_link`). The watch set follows.
    - Unit tests cover the model and the gate. Pending manual check recorded
      in `learnings.md`: click a folder row's expander, then Down / Up /
      Home / End move the cursor, Right expands and then enters, Left
      collapses and then goes to the parent, typing `20` jumps to the next
      folder starting with it, Enter opens (under `D:\Photos\tests\` only);
      with the tree focused `3`, `p`, the arrows and `Ctrl+Z` change no
      photo and `q` does not quit; `toggleLeft` still works; `escape` or a
      click on the preview gives the keys back and `3` rates again;
      `Expand All` on a small test tree and `Collapse All` behave as above.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names the
      new module if one was added; `native-app.md` records the focus model
      chosen and why.
  - Implementation approach:
    - Assumes Step 3 merged.
    - The step decides between a second GPUI `FocusHandle` on the pane
      (`track_focus`, its own `on_key_down` stopping propagation for what it
      consumes or swallows) and one flag in the view checked at the top of
      the existing `key_down`. Either way the gate must sit before the
      literal `home` / `end` / `q` arms and before the menus' `escape` arm,
      and a press outside the pane must reach its normal handler as well as
      returning the keyboard (the status row's menus already rely on
      capture-phase `on_mouse_down_out` for this).
    - Type-ahead needs the typed character: `key_name` lower-cases and
      names keys the keymap's way, so check what a shifted letter, a digit
      and a space give on Windows (`native-app.md`, "Every keystroke goes
      through the keymap") and use the keystroke's own character where they
      differ. IME input is unverified in the native app; type-ahead with an
      IME on is not a goal here.
    - `toggleRight`, `toggleStrip`, `toggleSides` and `open` are in
      `TREE_PASSTHROUGH` but the native app handles none of them; port the
      set as it is so they pass once they exist.
- [x] Step 5: The strip's multi-selection: Ctrl / Shift click, the extend keys, Select All, judgments on the whole selection
  - Done when:
    - `crates/native/src/selection.rs` is a pure model with unit tests, a
      port of `selection.ts` with `selection.test.ts`'s cases: `Selection
      { selected, anchor }` keyed by listed path, `single`, `selection_of`,
      `click` (range, toggle, plain), `extend`, `all`, `prune`, `settle`,
      `restore`, `targets`, and `judgments` (the value decided once from the
      focused file, each file keeping the fields the command does not
      touch, focused file first, an unchanged file skipped unless forced)
      over the native `judgment::Command` / `State`.
    - The view holds the selection. A plain click on a cell, a paging key,
      `home` / `end` and an open select the landed file alone; a Ctrl click
      (Cmd on macOS) toggles a cell and a Shift click selects the range from
      the anchor, the focus moving as in `main.ts`'s strip click handler;
      `extendPrevious` / `extendNext` move the focus by one and select the
      range from the anchor; `selectAll` selects every shown file with the
      focus kept. `refilter` and a re-listing end with `settle` / `prune`,
      so a file that leaves the list leaves the selection. The selected
      cells are painted as the Tauri strip paints `.selected`, apart from
      the current file's outline.
    - A judgment key applies to `targets(selection, files, index)` through
      `selection::judgments`: the view and the strip cells first, then one
      store per changed file through the existing row-then-writer path, all
      recorded as one undo entry. A failed store reverts its own file and
      drops it from the entry (`Undo::forget`), as today. Auto-advance
      follows `advance.ts`'s `shouldAdvance` with its target count, ported
      with the cases `judgment.rs`'s tests currently leave out. Undo / redo
      of a batch of several files leaves the current file where it is and
      notes `Undid N files` / `Redid N files`; a one-file step that moves
      the focus selects that file (`restore`). A view-only folder has a
      selection (for the copy and trash items of Steps 6-7) and still takes
      no judgment.
    - Unit tests cover the model and the batch rules. Pending manual check
      recorded in `learnings.md` (copies under `...\select\`): click a
      cell, Shift click one four cells on: five selected; Ctrl click one to
      drop it; `3`: four files get three stars and their sidecars say so;
      `Ctrl+Z` once: all four are back, the current file unmoved, `Undid 4
      files`; `Shift+Right` twice extends, `Left` collapses to one;
      `Ctrl+A` selects all and `p` picks all; under a filter the files a
      judgment hides leave the selection.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `selection.rs`; `native-app.md` records the selection rules next to
      the judgment ones.
  - Implementation approach:
    - Independent of Steps 2-4 in code; ordered after them by the user's
      decision. Assumes nothing of them.
    - `judgment.rs` keeps `changed` apart from `writes` (a `ClearAll` on a
      file whose entry was not seen still goes out): `selection::judgments`'
      `force` is that rule per file; do not fold the two tests into one.
    - `label_known`, `seen` and `touched` stay per path, so a batch needs
      no new bookkeeping beyond looping.
    - The Tauri compare mode changes what the selection means; the native
      app has no compare mode, so those branches are not ported.
    - Read the exact focus and anchor behaviour of a Ctrl and a Shift click
      from `main.ts`'s strip click handler and `selection.ts` rather than
      from memory; record anything `selection.ts` alone does not say.
- [x] Step 6: The strip's right-click menu: Select All, reveal, copy and the judgment groups
  - Done when:
    - `context.rs` gains `strip_groups(bindings, state, reveal_label,
      view_only, count, trashing)`, a port of `contextMenuGroups` with the
      matching cases of `context.test.ts`: `Select All`; the reveal item;
      `Copy Path` / `Copy File Name`; the judgment groups (`Pick` / `Reject`
      / `Unflag`; `1 star` … `5 stars` / `No stars`; the seven colors / `No
      label`) with their checked state from the focused file and each
      item's first key as its shortcut text, left out in a view-only folder;
      then `Move to Trash…` / `Delete Permanently…`, naming the count when
      it is over one and disabled while `trashing`. `Rename…` is not in the
      groups (part 2). In this step the last two items are present and
      disabled.
    - A right-click on a strip cell opens the menu at the pointer through
      the same popover as Step 3, with the focus and the selection changed
      as the Tauri strip's `contextmenu` handler changes them (a cell
      outside the selection becomes the focused, single selection; one
      inside keeps the selection); a right-click on the strip's empty area
      opens nothing. `Select All` runs Step 5's; the reveal item reveals
      the focused file; the copy items write the targets' listed paths, or
      their file names, one per line; a judgment item runs the same command
      as its key on the targets.
    - Unit tests cover the groups. Pending manual check recorded in
      `learnings.md`: select three cells, right-click one of them: the menu
      opens at the pointer (flipping inside the window near the right and
      bottom edges) and the three stay selected; `Copy Path` pastes three
      paths, one per line, `Copy File Name` three names; `2 stars` rates the
      three and the menu's check marks follow on the next open; the reveal
      item selects the focused file in Explorer; right-click a cell outside
      the selection: it alone is selected; in a JPEG folder the judgment
      groups are absent; a press on the preview or `escape` closes the menu;
      a left click on a cell still pages.
    - `mise run ci` and `mise run native:check` pass; `native-app.md`
      records what was learned.
  - Implementation approach:
    - Assumes Steps 3 and 5 merged. The strip's wrapping `div` gains
      `on_mouse_down(MouseButton::Right, ..)` next to the left button's;
      `strip::hit_test` gives the cell.
    - The shortcut text is `keys.ts`'s `displayKey`; port only what the
      menu needs, or reuse a backend helper if `shortcuts.rs` already
      formats keys for display (check `accelerator_for`).
    - The menu's action names are the keymap's, so a judgment item goes
      through `Command::from_action` like a key.
    - Record whether the copied path carries the `\\?\` prefix on Windows.
      The Tauri app writes the listed path as it is; do the same.
- [x] Step 7: Move to Trash… and Delete Permanently…: the dialog, the run, its progress, the hold during a scan
  - Done when:
    - `crates/native/src/trash.rs` is a pure model with unit tests, a port
      of `trash.ts` with the matching cases of `trash.test.ts`:
      `dialog_text(mode, count)`, `swallowed_by_delete(key, held)`,
      `confirms_delete(click_count)`, `progress_status`, `trashed_status`,
      `shown_path`, and `TrashFlow` (idle / asking / running, `start`,
      `run`, `dismiss`, `end`) with which of the two buttons has the focus.
    - `crates/native/src/idle.rs` is a port of `idle.ts` with
      `idle.test.ts`'s cases: `IdleGate` (one held operation with its label,
      a later request replacing and canceling the held one, `drain`,
      `discard`, `waiting`, the in-flight count) and `cancels_scan(label)`.
    - `scan.rs` gains a cancel that sets the flag without joining, so the
      scan still ends through `Done` and `FacesDone`.
    - `main.rs`: the two menu items open a modal dialog over the window for
      the menu's targets, with the title, the line and the two buttons.
      While it is open every key stops there, whoever had the keyboard:
      `escape` dismisses the question, `tab` moves between the buttons,
      `enter` presses the focused one for a trash. The delete dialog opens
      on Cancel and takes only a single mouse click on its run button as a
      yes (`swallowed_by_delete`, `confirms_delete`). A second start while
      one is under way does nothing.
    - The confirmed run goes through the gate. With no scan running it runs
      at once on a background task: flush the sidecar writer
      (`DRAIN_TIMEOUT`), then the backend's trash or delete function of
      Step 1 with the view's `Arc<trash::Runs>`, its progress sent into the
      event channel and shown in the pane's notes (`moving to Trash 3 /
      10`, `deleting 3 / 10`). While a scan runs the operation is held, the
      dialog closes, the scan is canceled and the held operation runs at
      `FacesDone`; a folder switch discards it. The pane's notes show what
      is waiting.
    - While a run is in flight no listing or scan starts: a watcher change
      is deferred as it is during a scan and the run's end re-lists once
      (which also rescans what a canceled scan left).
    - After the run: the status says `trashed_status`, each failure goes to
      the pane's notes (`<name>: could not move to the Trash: <message>` /
      `could not delete`), a delete drops the deleted files' judgments from
      both histories (`withoutPaths`, `isEmptyBatch`), and the folder is
      re-listed, the current file landing where
      `filter::anchor_after_filter` puts it and the selection pruned. The
      run's result is reported even when another folder was opened
      meanwhile.
    - Unit tests cover `trash.rs`, `idle.rs` and the new history helper.
      Pending manual check recorded in `learnings.md` (copies under
      `...\trash\` only): `Move to Trash…` on a RAW with `.xmp` and `.dop`:
      the dialog's text, `enter` confirms, the three files are in the
      Recycle Bin, the strip moves to the neighbour; three selected cells:
      `Move 3 Files to Trash…` and its line; `Delete Permanently…`: the
      dialog opens on Cancel, `enter`, `space` and a double-click do not
      confirm, one click does, the files are gone and not in the Recycle
      Bin; on a large copy on a fresh `--index`, `Move to Trash…` during
      pass 1: the dialog closes, the scan ends early, the move runs, the
      folder re-lists and the scan resumes for the rest.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `trash.rs` and `idle.rs`; `native-app.md` records the dialog pattern
      and the gate.
  - Implementation approach:
    - Assumes Steps 1 and 6 merged.
    - The key handler needs `KeyDownEvent.is_held` for the delete dialog's
      "a repeat is a key still held from before the dialog opened"; pass it
      along with the keystroke.
    - The dialog is a `deferred`, `occlude()`d full-window layer; keep the
      focus between its two buttons in `TrashFlow`, not in GPUI focus
      handles, unless Step 4's focus model makes a handle simpler. The
      dialog's key routing sits before Step 4's tree gate.
    - The native guard replaces the Tauri `Scans` lock: check in the step
      that nothing can start a listing between the gate's "no scan" answer
      and the run's end (the tree's open, the watcher's resync, a
      `TreeChanged`), and that judgment stores still queued
      (`judgment::Queue`) are written before the writer's flush.
    - `resync.rs` already defers one pending change to the scan's end;
      extend that rule to an operation in flight rather than adding a
      second mechanism.
    - The run id the backend returns is kept for Step 8; this step only
      logs it.
- [x] Step 8: Trash undo and redo, and the plan's docs wrap-up
  - Done when:
    - `history.rs`: an undo entry is a judgment batch or a trash run
      (`run_id`, `count`, `dirs`), ported from `undo.ts` with its tests:
      `undone_trash`, and the rule that another folder's open drops the
      judgment batches and keeps the trash runs. `trash.rs` gains
      `restored_status`, `restored_into`, `still_held` and `opens_target`
      with their tests.
    - A trash run that moved something pushes its entry and clears the redo
      history. `undo` on a trash entry goes through the gate (`Undo Move to
      Trash`, waiting for a running scan without canceling it), runs the
      backend's undo of Step 1 on a background task and keeps the entry
      until the backend answers: on success the entry moves to the redo
      history when anything came back, failures go to the notes, the status
      says `restored_status`, and when the open folder is among the run's
      folders the restored files join `all` at once (`restored_into`, then
      `refilter`) and the folder is re-listed, the current file and the
      selection kept; a refusal other than `RUN_GONE` leaves the entry
      undoable. `redo` moves what came back to the Trash again with no
      dialog, with the progress note, as a new run pushed on the undo
      history. A second undo or redo while one is out does nothing.
    - Unit tests cover the model. Pending manual check recorded in
      `learnings.md` (copies only): trash a RAW with its sidecars, `Ctrl+Z`:
      the three files are back, the strip shows the file again with its
      judgment; `Ctrl+Shift+Z`: in the Recycle Bin again; rate a file, trash
      it, `Ctrl+Z` twice: the file is back and then its rating is undone;
      trash in folder `a`, open `b` from the tree, `Ctrl+Z`: the file is
      back in `a`; a delete is not undoable.
    - Wrap-up: `../20261109-gpui-port/plan.md` keeps Step 8's checkbox open
      and its Progress gains `(YYYY-MM-DD) Step 8: the folder tree, the
      strip's selection and its right-click menu, through` this plan's
      archived path in backticks, `; the folder operation dialogs remain`.
      `learnings.md` lists every pending manual check of Steps 2-8 in one
      place for the user's bulk check. `CLAUDE.md`'s `crates/native`
      sentence names `tree.rs`, `context.rs`, `selection.rs`, `trash.rs` and
      `idle.rs` (and Step 4's module if one was added), and its backend
      sentence `folders.rs` and `treewatch.rs` (done per step; checked
      here). `native-app.md` has an entry for each of: the tree pane, the
      tree watcher on `notify` 8.2.2, the focus model, the selection, the
      context menu and dialog pattern, the gate and the trash undo.
    - `mise run ci` and `mise run native:check` pass.
  - Implementation approach:
    - Assumes Step 7 merged. The restore on Windows lists the Recycle Bin
      (`restore_run`); it runs on the background task and has no progress
      callback, as in the Tauri app.
    - `Undo::forget` and `step` see only judgment batches; a trash entry on
      top is handled before `step`, as the Tauri `step` does.
    - No performance measurement is part of this plan (the user's
      decision): no A/B, no `results.md`, no change to
      `docs/humans/performance.md`.

## Trade-offs and risks

- **No latency gate (decided).** Earlier native plans closed with a
  same-session A/B; this one does not. The tree pane is drawn every frame
  beside the preview, and the pane takes 220 px, so a landscape frame is
  drawn smaller than in every earlier native measurement. A later plan's A/B
  against a pre-tree baseline will therefore carry the tree's cost and the
  narrower canvas together. Step 2 still keeps the design rules that make
  the cost small (only visible rows laid out, rows not rebuilt in `render`)
  and records the new canvas size.
- **A launch without a folder opens nothing (decided).** The Tauri app
  reopens `lastFolder`; the native app shows the tree alone. The OS folder
  picker is no longer reachable: the keymap's `open` action stays unwired.
  A folder outside every root is reachable only as a command-line argument.
- **The tree reads folders outside the test area (approved).** The roots
  (home, every drive) are listed at launch and revealing the open folder
  lists and watches each ancestor: a `read_dir` of names and a non-recursive
  watch, nothing written. A click on a folder's name or Enter on it does
  open it (index rows in the test index, sidecars read, the scan run), so
  the hand checks open only folders under `D:\Photos\tests\`. A watched
  folder also pins its ancestors against a rename on Windows while the
  native app runs.
- **What stays out of the tree.** The multi-folder selection and the inline
  rename are part 2's, with the folder operations that use them; the tree
  menu therefore always targets the one folder right-clicked.
- **What stays out of the strip menu.** `Rename…` (part 2). The Tauri
  compare mode's effect on the targets is not ported; the native app has no
  compare mode.
- **The selection changes the judgment path.** Until Step 5 every judgment,
  store, revert and undo entry is one file; afterwards each is a batch.
  `refilter`, auto-advance, `Undo::forget` and the `seen` / `touched` rules
  all have a one-file assumption somewhere. Step 5 is ordered before the
  menu and the trash so its review is about that change alone.
- **Type-ahead and key names.** The native key handler names keys for the
  keymap (`native-app.md` notes a shifted digit arrives as the character it
  types and a digit under Ctrl+Alt depends on the layout). Type-ahead needs
  the character typed, so Step 4 may need the raw keystroke beside the
  keymap name. IME input is unverified in the native app.
- **Swallowed keys.** While the tree has the keyboard, `q` no longer quits
  and undo does nothing, as in the Tauri app. That is the ported rule, but
  it is new for the native app, whose `q` has always quit; `--auto` runs
  never focus the tree, so measurements are unaffected.
- **The reveal is not shared code.** The Tauri app reveals through the
  opener plugin, the native app through GPUI's `reveal_path`; only
  `REVEAL_LABEL` is shared. Their behaviour on a verbatim Windows path may
  differ (Step 3 checks).
- **The native guard is not the Tauri lock.** The Tauri commands hold the
  `Scans` lock across the moves, so no scan can start under them. The
  native app has one view thread deciding when a listing starts, so the
  guard is the gate plus the deferred resync; Step 7 has to show there is no
  path that starts a listing mid-run. The backend functions of Step 1 do not
  check for a scan themselves.
- **Step 7 is the largest step** (two models, the dialog, the gate, the
  scan cancel, the run). Splitting the gate out would leave an interim
  state that refuses a trash during a scan, which the Tauri app replaced on
  purpose; it is kept as one PR.
- **The other session.** The filter plan's Step 5 (its A/B) is still open
  and that session owns `menu.rs` and the status row. This plan edits
  `main.rs` heavily (the layout, the key handler, the event handler, the
  strip's click handler), so merge conflicts with it are likely; rebase
  before each PR. That plan's A/B baseline predates the tree: if it
  measures after Step 2 merges, its A carries the tree pane.
- **Step 1 edits the shipped Tauri app with no smoke check** (the port's
  standing decision). The tests move with the code; a regression in the
  Tauri tree or trash is handled by reverting the move.
- **Eight steps.** One over the usual range, from the two features the user
  added; each is its own review unit and none can be folded into a
  neighbour without mixing unrelated changes.

## Progress

- (2026-11-13) Step 1 complete
- (2026-11-14) Step 2 complete
- (2026-11-14) Step 3 complete
- (2026-11-14) Step 4 complete
- (2026-11-14) Step 5 complete
- (2026-11-14) Step 6 complete
- (2026-11-14) Step 7 complete
- (2026-11-14) Step 8 complete
