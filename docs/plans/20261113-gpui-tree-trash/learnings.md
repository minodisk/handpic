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
- The folder token is a new `opens: Arc<Mutex<u64>>`, bumped by `open`, not
  `listing`: `listing` is bumped by every re-listing of the open folder too,
  which would cut the reveal short on a watcher resync. The plan's text says
  "the `listing` counter"; this is the deviation.
- `list` had never been called twice concurrently (only `start` opened). With
  opens from the tree, a quick second open could be overwritten by the
  first's late listing and could be left with the first folder's watcher (its
  background task may still be stopping the previous scan when the second
  sets its watch). The listing now sets the watcher under the `opens` lock
  only while its open is still the latest, and `opened` is dropped otherwise.
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
