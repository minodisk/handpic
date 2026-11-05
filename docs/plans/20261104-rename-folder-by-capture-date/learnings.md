# Learnings

## Step 1: the planner and `date_folder_preview`

- The planner (`datefolder::plan`) takes dates, not Exif times: the command
  maps each listed file's capture time through `date_of` first (in `dated`),
  so the tests of the grouping pass `YYYY-MM-DD` strings directly.
- `date_of` refuses the all-zero `0000:00:00 00:00:00` some cameras write
  when the clock was never set, an out-of-range month or day, and any value
  that is not `YYYY:MM:DD` in its first ten bytes; such a file is undated.
- A file with an index row takes the row's `capture_time` even when it is
  `None` (a failed extraction would most likely fail again); only a listed
  file with no row is read with `reader::read_metadata`.
- A split target named like the folder itself (a folder `2026-10-31` holding
  photos of 10-31 and 11-01) is reported as a collision: the split creates
  siblings and merges nothing, so the folder itself is an existing entry by
  that name. Only the one-date case keeps `folder_target`'s case-only
  exemption (and a folder already named its one date is `Named`). Step 3's
  re-plan inherits this, so its split never moves a file into the folder it
  comes from.
- The one-date collision check reuses `rename::folder_target` and treats any
  error as the collision; its other refusals (a root, an invalid name) cannot
  happen there, since the root is refused first and a date is a valid name.
- `cargo` is not on the Git Bash `PATH` here; `mise exec -- cargo test -p
  riffle-app datefolder` runs the module's tests alone.

## Step 2: `Index::move_file` and `Index::forget_dir`

- Eviction's per-folder delete (`evict_folder`) is not `forget_dir`: it keeps
  dirty `ratings` rows (and then the `folders` row) and leaves `views`, since
  eviction must not lose an unwritten judgment of a folder still on disk.
  `forget_dir` is for a folder removed from disk, so it drops every row of
  exactly that `dir` in `files`, `ratings`, `embeddings`, `views` and
  `folders`, dirty ratings included. It matches `dir = ?1` only, so neither a
  subfolder nor a prefix sibling (`photos2`) is touched.
- `move_file` rewrites `folders.last_viewed` wherever it names the old path,
  as the plan says, which leaves the old folder's `last_viewed` naming a file
  now in another folder. That is harmless (`resume.ts` only takes it when the
  listing holds it), and the old folder is usually forgotten right after.
- Both methods are unused outside the tests until Step 3, so `clippy -D
  warnings` flags them as dead code; they carry
  `#[cfg_attr(not(test), expect(dead_code))]` (the style of
  `crates/core/src/genre.rs`). Step 3 must remove those attributes, which the
  `expect` enforces by failing once they are used.
- `clippy::cloned_ref_to_slice_refs` rejects `&[x.clone()]` in tests; use
  `std::slice::from_ref(&x)`.

## Step 3: `date_folder_run`

- `rename_folder`'s blocking body is now `rename::rename_folder_blocking`,
  called by the command and by the one-date branch; the command's signature
  and behavior are unchanged. The one-date branch passes the caller's
  spelling of `dir` (not the canonical one), so `Renamed.path` stays spelled
  the frontend's way; the split's `Moved.path` is likewise the caller's
  parent joined with the date, while the index keys use the canonical form.
- The run returns `Ran`, internally tagged by `kind`: `renamed` carries
  `rename_folder`'s `path` / `warning`, `split` carries `moved` (each
  `{name, path, count}`, only the folders that received files), `failed`
  (`trash::Failure`), `left` (every entry still in the original folder,
  sorted, subfolders and stray files such as `Thumbs.db` / `.DS_Store`
  included), `removed` and `warning`. Step 4 reads `removed` to decide
  between a resync and opening the first date folder.
- A `Named` folder or any collision refuses the run with an `Err`; the
  re-plan happens before the writer drain (it is a refusal check, as the
  tauri-app guide asks), while the sidecar pairing of each file happens per
  file during the moves, after the drain, so a sidecar a pending judgment has
  just written moves too.
- A date folder is made with `std::fs::create_dir`, which fails when the
  name appeared after the re-plan; every file of that group is then a
  failure, so nothing is ever merged. A created folder nothing moved into is
  removed again.
- `with_released` only puts the tree watches back on `Err`, and the split's
  closure always returns `Ok` (its failures are per file). When the original
  folder remains, the open folder's watcher is put back with `watch::set`;
  the tree watches come back with the tree re-render after Step 4's
  `folders.refresh`, since `set_tree_watches` makes the watched set equal to
  what the frontend sends.
- The recorded trash runs (`Runs`) are not rebased for moved files: an undo
  of a run recorded before the split restores to the old path, which may be
  gone if the original folder was removed (`restore` reports the failure).
  Documentation is Step 5's.
- Removing the `#[cfg_attr(not(test), expect(dead_code))]` from
  `Index::move_file` / `Index::forget_dir` was required, as Step 2 planned.

## Step 4: the `Rename by Capture Date…` item and its dialog

- The plan says a press during a scan is refused "like the sidecar
  commands", but `rewriteSidecarsIn` / `deleteSidecarsIn` do go through
  `whenIdle` (held for the scan's end). The date flow follows the plan's
  intent instead: no `whenIdle`, the preview is invoked at once and the
  backend's `SCAN_RUNNING` refusal lands on the status line.
- The dialog is its own `#datefolder-dialog` (not the sidecar dialog
  reused), since it needs the undated line, the removal note and the
  disabled reason as separate paragraphs; empty ones are hidden with
  `#datefolder-box p:empty`. With a collision or an already named folder,
  `Run` is disabled and focus goes to `Cancel`.
- `renameFolder`'s post-rename continuation is now `folderRenamed(path,
  newPath, name, warning)` in `main.ts`, shared with the one-date result
  (the name passed is `baseName` of the returned path).
- A split re-lists the parent with `folders.refresh(parentOf(dir))`;
  `parentOf` (in `datefolder.ts`) keeps a root's separator (`E:\DCIM` ->
  `E:\`, `/DCIM` -> `/`) so it names the tree's root node.
- Python's text-mode write on Windows turns `\n` into `\r\n`; files
  edited by a script need `newline=""` or a `sed -i 's/\r$//'` afterwards.
- `pnpm` is not on the Git Bash `PATH`; `mise exec -- pnpm exec vp check`
  and `mise exec -- node ./node_modules/vite-plus/bin/vp test` run the
  frontend checks alone.

## Deferred issues (todo candidates)

- **Pending manual check (Windows)**: split a folder that is expanded in the
  folder tree and open in the strip, with photos of two dates and no undated
  file, via `date_folder_run` (Step 4's dialog once it lands). Expected: the
  two sibling date folders hold the files with their `.xmp` / `.dop`, the
  original folder is removed (no `PermissionDenied` from a held watch
  handle), and the first date folder opens with its judgments and
  thumbnails intact. Step 3's checkbox was ticked on the automated criteria
  (unit tests on temp dirs); the real watcher release can only be verified
  on Windows by hand. Basis: plan "Trade-offs and risks", Windows watchers;
  files `crates/app/src/datefolder.rs` (`split_blocking`),
  `crates/app/src/treewatch.rs` (`with_released`).
- Two RAWs sharing a stem (`a.ARW` and `a.NEF`) share `a.xmp`; a split
  moves that sidecar with whichever RAW moves first, the same ambiguity
  `rename::file_plan` has. Basis: Step 3 implementation of
  `datefolder::move_plan`; files `crates/app/src/datefolder.rs`,
  `crates/app/src/rename.rs`.
- After a split, the undo history's judgment entries still name the moved
  files' old paths (only a folder rename's trash entries are rebased, by
  `folderRenamed`); an undo of a judgment made before the split would write
  to a path that no longer exists. `date_folder_run`'s `Split` does not list
  the per-file moves, so the frontend cannot rebase them. Basis: Step 4
  implementation of `runDateFolder`; files `crates/app/ui/src/main.ts`,
  `crates/app/src/datefolder.rs`.
- **Pending manual check (Windows / macOS)**: in the running app, right-click
  a folder in the tree and choose `Rename by Capture Date…` on (a) a folder
  of one date: the dialog reads `Rename <name> to YYYY-MM-DD?` with the
  file count, `Run` renames it in the tree and reopens it if it was open;
  (b) a folder of two dates with an undated file: one row per date folder,
  the undated line and the removal note show, `Run` creates the sibling
  folders, the tree re-lists the parent, the original stays with the
  undated file and is re-listed if open; (c) the same without an undated
  file while open: the original disappears from the tree and the first date
  folder opens; (d) a sibling already named a target date: the reason shows
  in red and `Run` is disabled; (e) during a scan: the status line shows the
  scan message and no dialog opens. Step 4's checkbox was ticked on the
  automated criteria (Vitest on the text helpers and the flow, `mise run
  ci`); the dialog and the tree's follow-up were not exercised in a running
  app. Basis: plan Step 4 Done when; files `crates/app/ui/src/main.ts`,
  `crates/app/ui/index.html`.
