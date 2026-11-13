# Learnings: gpui-tree-trash

## Step 1: the Tauri-free pieces into the backend

- Moved to `crates/backend/src/folders.rs`: `FolderNode`, `Folder` (fields now
  public), `roots`, `list`, `REVEAL_LABEL`, `is_hidden`, the volume helpers and
  the 7 tests. The reveal label test now checks `REVEAL_LABEL` directly, as the
  `reveal_label` command stayed in the app.
- Moved to `crates/backend/src/treewatch.rs`: `TreeWatch`, `State`,
  `tree_paths`, the per-path debounce `run`, `with_released` (now taking the
  `&TreeWatch`) and the 4 tests. `TreeWatch::spawn(on_changed)` replaces the
  `AppHandle`, and `TreeWatch::set(dirs)` is the public "make the watched set
  equal to `dirs`". The app's `treewatch.rs` keeps `tree-changed`, its payload,
  `set_tree_watches` and an `AppHandle` wrapper of `with_released`, so
  `rename.rs` and `datefolder.rs` did not change.
- The app's `watch.rs` re-exported `riffle_backend::watch::triggers` only for
  its `treewatch.rs`; the move left it unused (a clippy warning), so it went.
- Moved to `crates/backend/src/trash.rs`: `trash_one` (now `pub`, used by
  `foldersidecars.rs` as `trash::trash_one`), `trash_many`, `move_groups`,
  `move_again`, `restore_run` with their `cfg`s, plus the four command bodies
  `trash_listed`, `delete_listed`, `undo_run` and `redo_run`. Inside the
  backend module `trash` the external crate must be spelled `::trash::`.
- `undo_run` stores a clone of `Restored::back` in `Runs::undone` and returns
  the `Restored` with `back` still filled, so the command's log line still
  shows it as before; `back` is `#[serde(skip)]`, so the reply is unchanged.
- Only the Windows `cfg` arms were compiled here; the macOS and Linux arms of
  `trash_one`, `move_groups`, `move_again` and `restore_run` moved verbatim
  with only their `trash::` prefixes dropped.
- The C: drive filled up (0 bytes free) between the root `mise run ci` and
  `mise run native:check`, and the native build failed on it. `cargo clean` of
  the root workspace's `target` (11.6 GiB, this worktree's own cache) freed
  enough for the native build; both checks then passed.
- One root `mise run ci` rerun failed with the `riffle-core` lib test process
  exiting abnormally (no failing test named) while the machine was loaded by
  other builds; `cargo test -p riffle-core --lib` alone then passed (409), and
  this step touches no `riffle-core` code. Treat it as a load flake.

## Step 2: the tree model and the left pane

- The pane is a `uniform_list` over `tree_rows`, not a `canvas`: it lays out
  only the rows on screen, its rows are `div`s with their own click listeners
  (the expander's `on_click` calls `cx.stop_propagation()` so the row's open
  does not also run), and `UniformListScrollHandle::scroll_to_item` gives
  Step 4 its scroll-into-view. So no pure hit-test helper was needed.
- `tree.rs` keeps `is_link` on the listed `Child` (as `tree.ts`'s
  `FolderNode`), not on the node; it and `watched_folders` are unused until
  Steps 3 and 4 and carry `#[allow(dead_code)]` (the `history.rs` precedent).
  `rebase` is private, used by `respell`; its four cases are ported anyway.
  `descendants` / `collapseAll` were left for Step 4 with the keyboard.
- The folder token is a new `opens: Arc<AtomicU64>`, bumped by `open`, not
  `listing`: `listing` is bumped by every re-listing of the open folder too,
  which would cut the reveal short on a watcher resync. The plan's text says
  "the `listing` counter"; this is the deviation.
- `list` had never been called twice concurrently (only `start` opened). With
  opens from the tree, a quick second open could be overwritten by the
  first's late listing and could be left with the first folder's watcher (its
  background task may still be stopping the previous scan when the second
  sets its watch). The listing now sets the watcher under a separate
  `watching_lock` (the UI thread only bumps the atomic, so it never waits on a
  watcher) only while its open is still the latest, and `opened` is dropped otherwise.
- The second-open reset (`opened`'s non-`carry` branch) already cleared the
  judgments, `touched`, notes, write errors, the `f` overlay's caches
  (`marks.clear()`, both branches), zoom (`zoom_page`), the dwell and its
  timer, undo, the step note, the EXIF filter and the kept anchor; peaking is
  keyed by the shown image and `viewed::Batches` by folder. Added: `failed`
  (the previous scan's failure count stayed on the status line when the new
  folder's scan could not be prepared) and `key` (a key pressed in the
  previous folder would have timed the new folder's first preview).
- The command-line folder is revealed by its `std::path::absolute` spelling
  (no `\?\` prefix, symlinks not resolved), as the Tauri `reveal` takes the
  path as opened; the canonical `dir` stays the index's key.
- A failed listing marks the row's name red and puts the error in the pane's
  notes (`tree_note`, the newest replacing the older, cleared by the next
  open); a GPUI tooltip would need a view per row, so the row carries no
  `title` like the Tauri one.
- The local `python` heredoc edits wrote CRLF on Windows (text mode); the
  file had to be converted back to LF. Write with `newline=''` or binary.

## Step 3: the tree follows the disk, and its right-click menu

- The watched set is handed over from `tree_changed`, so every path that
  changes the tree (expand, collapse, a listing, a reveal step, a watcher
  re-list, `Refresh`) syncs the watcher without a call of its own. The chain
  is a `Task<()>` that awaits the previous one before `TreeWatch::set`; the
  previous task is moved into the new one, so it is never dropped (which
  would cancel it).
- `notify` 8.2.2 (the native lock) was checked with a throwaway test in the
  native crate driving the backend's `TreeWatch`: collapse (`unwatch` +
  `settle`) took 0.1-0.3 ms, a re-expand watched again, no event arrived
  while collapsed, and the parent folder could be renamed after the last
  collapse. `cargo test -p riffle-backend` from `crates/native` is refused
  ("requires dev-dependencies and is not a member of the workspace"), hence
  the throwaway; recorded in `native-app.md`.
- `Refresh` on the open folder also re-lists it through `Resync`, as the
  Tauri menu's `resync("refresh")` does (`opensTarget`, `Same` only). The plan
  did not list this; it is three lines and keeps the port's behaviour.
- GPUI's Windows `reveal_path` needs a parent folder (`target.parent()`),
  so revealing a root (`D:\`) would only log an error; the menu opens a root
  with `open_with_system` instead. The tree's paths never carry `\\?\`, so the
  prefix question is Step 6's (strip files); it stays unverified.
- `context::Open` holds the pointer position as `(f32, f32)`, so it cannot
  derive `Eq`; the tests compare its fields.
- The `bash` heredoc holding the large Rust edit failed to parse
  (`unexpected EOF while looking for matching '`), so the edit script went
  into a scratchpad file written with the Write tool and run with `python`.

## Step 4: the tree's keyboard

- Focus model: one `tree_keys` flag on the view, checked in `key_down` right
  after the right-click menu's close, not a second `FocusHandle`. The pane
  gives it on a left press (`on_mouse_down(Left)`) and hands it back from
  `on_mouse_down_out` (capture phase, any button), so the press outside still
  reaches its own handler. A right press in the pane does neither.
- `on_mouse_down_out` is bounds-based (`hitbox.contains(mouse_position)`), so
  a press on the right-click menu where it is drawn over the pane would not
  hand the keys back while one beyond the pane would. The popover's own
  `on_any_mouse_down` hands them back, matching the Tauri menu's buttons,
  which take the focus. The pane's own left listener does not fire there,
  since the popover is `occlude()`d.
- The Tauri app leaves the keyboard with the tree after opening a folder from
  it (`openFolder` -> `openDirectory` never calls `folders.blur()`; only
  `changePanels` hiding the pane does), so the native app does the same.
- `tree_key` takes the drawn rows rather than the tree: `Row::expandable` is
  `tree.ts`'s `canExpand`, so the rows the view already keeps suffice.
- `typed_char` (in `keys.rs`) uses the keystroke's `key_char` first:
  `key_name` lower-cases Shift+A and names the space bar `space`. Read from
  `gpui_windows`' `process_key` (`ToUnicode`, control characters dropped),
  not seen in the running app.
- `Typed::at` is an `Option<Instant>` (the TS `-Infinity` start), so the
  tests build their times from one `Instant::now()` plus offsets.
- `Expand All` is a loop over an explicit stack of chains rather than the TS
  recursive `walk`, which an `async` closure cannot recurse into without
  boxing; pushing a listed folder's children in reverse keeps the depth-first
  order, and the "chain still expanded" check runs before each child as in
  the TS.
- Python edit scripts passed through a `bash` heredoc lost backslashes (a
  replacement holding `r"D:\a"` did not match, and a `\x` in the text broke
  the script); scripts holding backslashes went into scratchpad files written
  with the Write tool instead, or `sed` by line number.

## Deferred issues (todo candidates)

- **Pending manual check (Step 2, Windows).** Basis: Step 2's Done-when;
  GUI clicks cannot be driven headless. The step's checkbox was ticked on the
  automated criteria (`tree.rs` tests, `mise run ci`, `mise run
  native:check`). Setup: copy a few ARWs into
  `D:\Photos\tests\20261113-gpui-tree-trash\tree\a` and `...\tree\b`; build
  with `cargo build` in `crates/native` (rename a locked
  `riffle-native.exe` aside, never kill it); run with a fresh `--index
  D:\Photos\tests\20261113-gpui-tree-trash\index.sqlite` and `--settings
  D:\Photos\tests\20261113-gpui-tree-trash\settings.json`. Steps and
  expected results:
  1. Launch with no folder argument: no OS dialog; the left pane lists home
     and the volumes; the preview and strip are empty and the status line
     says `no folder`; the arrow, judgment, `f`, `z`, `e`, `home` / `end`,
     undo / redo keys, the Sort / Filter menus, a strip click and wheel do
     nothing and nothing panics; `q` quits, and so does closing the window.
  2. Launch on `...\tree\a`: the chain `D:\` > `Photos` > `tests` >
     `20261113-gpui-tree-trash` > `tree` > `a` is expanded, `a` is
     highlighted (bold, filled) and scrolled into view with its RAW count.
  3. Click the expander of sibling `b`: it expands (its count shows), click
     again: it collapses.
  4. Click the name `b`: the strip, the preview, the meta pane and the status
     line switch to `b`, `a`'s scan stops (the log shows no further `a`
     progress), and the highlight moves to `b`. Page a few files in `b`,
     click `a`: it resumes at the file it was left at.
  5. Press `ctrl+alt+arrowleft` (`toggleLeft`): the pane hides and the
     preview widens; press again: it is back.
- The coalescing `last_viewed` writer (`viewed::Latest`) keeps one pending
  value across folders: a position pushed in folder A while a write is out
  is replaced by B's first push if B is opened and paged before that write
  settles, so A reopens one file earlier. The Tauri `lastViewedWriter` has
  the same shape. Basis: Step 2's second-open review of `opened`. Files:
  `crates/native/src/viewed.rs`, `crates/app/ui/src/resume.ts`.
- The launch's eviction (`eviction`, joined by the first listing only) is not
  awaited by a second open started before the first listing joined it, so
  that listing's `Scan::prepare` may run before the eviction. Only reachable
  by opening two folders within the eviction's run at launch. Basis: Step 2's
  concurrent-open guard. File: `crates/native/src/main.rs` (`list`).
- **Pending manual check (Step 3, Windows).** Basis: Step 3's Done-when; the
  watcher-driven re-list, the window activation and the right-click menu
  cannot be driven headless. The step's checkbox was ticked on the automated
  criteria (`context.rs` tests, `mise run ci`, `mise run native:check`, and
  the `notify` 8.2.2 throwaway test). Setup as Step 2's (copies under
  `D:\Photos\tests\20261113-gpui-tree-trash\tree\`, `cargo build` in
  `crates/native`, a fresh `--index` and an explicit `--settings` under
  `D:\Photos\tests\20261113-gpui-tree-trash\`). Steps and expected results:
  1. Launch on `...\tree\a`, so `...\tree` is expanded. In Explorer, create
     a subfolder `c` in `...\tree`: it appears in the tree within about a
     second, `tree` staying expanded. Rename `c` to `d`: the row follows.
     Delete `d`: the row goes, and no error shows in the pane's notes.
  2. Collapse `tree`, create `e` under it in Explorer: nothing happens (no
     re-list in the log); expand `tree`: `e` is listed. Remove `e`.
  3. Right-click a folder row (`b`): a menu opens at the pointer with the
     reveal item (`Reveal in File Explorer`), `Copy Path`, `Copy Folder
     Name` and `Refresh`; `escape` closes it, and so does a left or right
     press elsewhere (a right press on another row opens that row's menu).
  4. The reveal item opens Explorer on `tree` with `b` selected; on a root
     (`D:\`) it opens the drive. `Copy Path` then pasting gives the row's
     full path (e.g. `D:\Photos\tests\20261113-gpui-tree-trash\tree\b`);
     `Copy Folder Name` gives `b`.
  5. With `tree` collapsed, create `f` under it in Explorer, then
     right-click `tree` and pick `Refresh`: expanding it lists `f` with no
     wait on a re-list. `Refresh` on the open folder `a` re-lists its files.
     Remove `f`.
  6. Switch to another app and back: the tree does not flicker or collapse
     (the roots are re-read and the known ones skipped); with a USB drive
     plugged in meanwhile, its volume appears as a new root.
- **Pending manual check (Step 4, Windows).** Basis: Step 4's Done-when;
  the tree's keyboard, its focus model and the menu's `Expand All` /
  `Collapse All` cannot be driven headless. The step's checkbox was ticked on
  the automated criteria (`tree.rs`, `treekeys.rs`, `keys.rs` and
  `context.rs` tests, `mise run ci`, `mise run native:check`). Setup as Step
  2's (copies under `D:\Photos\tests\20261113-gpui-tree-trash\tree\`, with
  `a` and `b` holding a few ARWs and `b` holding nested subfolders, e.g.
  `b\20a\x`, `b\20b`, `b\c\d`; `cargo build` in `crates/native`, a fresh
  `--index` and an explicit `--settings` under
  `D:\Photos\tests\20261113-gpui-tree-trash\`). Steps and expected results:
  1. Launch on `...\tree\a`. Press `3`: `a`'s current file is rated (the
     photo has the keyboard).
  2. Click the expander of `b`: `b` expands and the tree takes the keyboard;
     the outline sits on the cursor row (`a`, where the open's reveal put
     it). Down / Up move the outline one row, Home / End to the first and
     last rows, and an off-screen row scrolls into view.
  3. With the cursor on `b`: Right on collapsed `b` expands it, Right again
     moves to its first child; Left on that child goes back to `b`, Left on
     expanded `b` collapses it, Left again goes to `tree`.
  4. Expand `b` and type `20` quickly: the cursor jumps to `20a`; tap `2`
     repeatedly: it cycles through the folders starting with `2`.
  5. With the tree focused, `3`, `p`, the arrows and `Ctrl+Z` change no photo
     and no judgment, and `q` does not quit; `ctrl+alt+arrowleft`
     (`toggleLeft`) hides the pane, after which `3` rates again (hiding the
     pane hands the keys back); show the pane again.
  6. Click a row, press `escape`: `3` rates again. Click a row, then click
     the preview: `3` rates again. With the photo holding the keys, right-click
     a row and close the menu with `escape`: `3` still rates (the right-click
     gave nothing). Click a row, right-click another, `escape`: the menu
     closes and the tree keeps the keys (a second `escape` hands them back).
  7. Move the cursor to `b` and press Enter: `b` opens (strip, preview,
     status follow), the cursor and highlight sit on `b`, and the tree keeps
     the keyboard (Down still moves the cursor). Open only folders under
     `D:\Photos\tests\`.
  8. Right-click `b` (not a root): the menu ends with `Expand All` /
     `Collapse All`; on a root (`D:\`) they are absent. `Expand All` on `b`
     expands `b` and every folder under it, one listing at a time;
     `Collapse All` on `b` collapses everything under `b`, `b` staying
     expanded. A folder created in Explorer under an expanded one still
     appears (the watch set follows).
