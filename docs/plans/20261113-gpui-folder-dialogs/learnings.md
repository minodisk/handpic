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
