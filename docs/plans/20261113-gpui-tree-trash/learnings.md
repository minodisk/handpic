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
