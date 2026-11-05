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

# Rename a folder by its photos' capture date

## Purpose

A card's folder usually comes off the camera named `100MSDCF` or `DCIM`, and the
user renames it by hand to the shooting date before culling. The folder tree
gets a right-click item, `Rename by Capture Date…`, that derives the name from
the capture dates (`DateTimeOriginal`, the camera's local date, no time-zone
handling) of the photos directly in the folder. The user confirms what will
happen in a preview dialog before anything on disk changes.

Decided design:

- The name is `YYYY-MM-DD` (e.g. `2026-10-31`).
- **One date**: the folder is renamed to that date through the existing
  `rename_folder` path (writer drain, `Scans` lock, watcher release, index
  rows carried along). Undated files come along with the folder.
- **Several dates**: one folder per date is created **as a sibling of the
  original folder** (in its parent), and each dated photo moves into its
  date's folder with its `.xmp` / `.dop` sidecars, its index rows
  (judgments, thumbnail, views, embedding) following it. **Undated files
  stay where they are** and are listed in the dialog. Afterwards the
  original folder is **removed only when it is empty** (its `folders` row,
  index state and watcher dropped; if it was the open folder, the first date
  folder is opened instead); otherwise it is left and the files left in it
  are reported.
- An **existing sibling entry with a target name refuses the run** (as
  `Rename…` does); nothing is merged.
- A folder with no dated file is refused with a message.
- Only the files directly in the folder count; subfolders are not descended
  into and are not renamed (they move with a rename, stay put in a split).
- A JPEG-only (view-only) folder is allowed; a JPEG moves alone, nothing
  next to it is touched (as `trash::groups_of`).

## Steps

- [x] Step 1: The plan of a folder's date rename or split (`crates/app/src/datefolder.rs`) and its `date_folder_preview` command
  - Done when: a pure planner groups the folder's photos by capture date and
    yields `Rename { name }` (one date), `Split { groups }` (several dates;
    each group a sibling target folder name and the files going into it) or a
    refusal (no dated file); the undated files are listed in the plan; a
    target that already exists in the parent (and is not the folder itself
    in another case) is reported as a collision (this case-only exemption
    applies to the one-date rename only; in a split a target named like the
    folder itself is a collision, since a split only creates siblings and
    never merges); a folder already named its
    one date is reported as "already named" (no-op). The plan is serialized
    for the frontend by a `date_folder_preview` command registered in
    `main.rs`. Unit tests on temp dirs cover: the Exif time to `YYYY-MM-DD`
    conversion (`2026:10:31 09:23:33` → `2026-10-31`, a malformed or empty
    value → undated), one date → `Rename`, several dates → `Split` with files
    grouped and sorted by name and the undated files listed, no dated file
    refused, already named, a colliding sibling (folder and file), and a
    JPEG-only folder planned like a RAW folder. `mise run ci` passes.
  - Implementation approach (as far as it is known; omit if unknown):
    - New module `crates/app/src/datefolder.rs` (declared in `main.rs` next
      to `foldersidecars`), module doc in the style of `foldersidecars.rs`.
      Keep the date derivation and the grouping pure (`fn date_of(capture_time:
      &str) -> Option<String>`, `fn plan(dir, dated: &[(path, Option<date>)])
      -> Plan`), so the tests need no index or Tauri handle; the command
      assembles the inputs.
    - Capture times come from the index first: `Index::entries(dir)` gives
      `IndexedFile.capture_time` (Exif `YYYY:MM:DD HH:MM:SS`). For a listed
      file with no index row, read it with
      `riffle_core::reader::read_metadata(path)` (`Arw.shot.capture_time`),
      which reads only the bounded head; do not pull a thumbnail
      (`scan::extract`) just for the date. The listing is
      `commands::read_listing(dir, None).files` (RAWs, or JPEGs when the
      folder is JPEG-only), non-recursive by design.
    - Target names are checked with `rename::check_name` and collisions with
      the same rule as `rename::folder_target` (an entry that canonicalizes to
      the folder itself is a case-only rename, not a collision). Reuse
      `folder_target` for the single-date case.
    - The command refuses while a scan runs (`SCAN_RUNNING`), as
      `rewrite_sidecars_preview` does, and runs its IO in `spawn_blocking`.
    - The `Preview` payload (serde `Serialize`) carries: the kind
      (`rename` / `split` / `named`), the folder's current name, the target
      name(s) with their file counts, the undated files' names, and the
      colliding target names (the dialog disables the run on any).

- [x] Step 2: `Index::move_file` carries a file's rows to another folder, and `Index::forget_dir` drops a folder's state
  - Done when: `Index::move_file(old_path, new_dir, new_path)` moves the
    `files`, `ratings`, `embeddings` and `views` rows of one file to a new
    `path` **and** `dir` in one transaction, dropping any stale rows already at
    `new_path` first (as `rename_file` and `rename_dir` do), and rewrites
    `folders.last_viewed` where it names the old path; `Index::forget_dir(dir)`
    deletes the `folders` row and any remaining rows of exactly that folder
    (not of a prefix sibling, as `rename_dir`'s matching). Unit tests in
    `index.rs` next to `rename_file`'s cover the move across folders, the
    thumbnail / judgment / embedding / view surviving, a stale row at the
    target replaced, a sibling file in the old folder untouched, and
    `forget_dir` leaving a prefix sibling alone. `mise run ci` passes.
  - Implementation approach (as far as it is known; omit if unknown):
    - Model `move_file` on `rename_file` (`crates/app/src/index.rs` around
      line 1571), adding `dir = ?new_dir` to the `UPDATE`s. The `folders` row
      of a new date folder is created by the normal `reconcile` when it is
      first opened; do not insert one here. Check whether an existing helper
      (eviction's per-folder delete) already does what `forget_dir` needs
      before adding one.
    - Independent of Step 1; Step 3 needs both.

- [x] Step 3: The `date_folder_run` command: rename through `rename_folder`'s path, or split into sibling per-date folders
  - Done when: `date_folder_run(dir)` re-plans the folder (the preview is
    not trusted across the dialog) and refuses on any collision or while a
    scan runs; then either (a) one date: renames the folder exactly as
    `rename_folder` does and returns its `Renamed`; or (b) several dates:
    creates the per-date folders in the parent, moves each dated file and
    its existing `.xmp` / `.dop` sidecars (RAW first, as `rename::file_plan`
    / `file_run` order them), never overwriting an existing file, carries
    the rows with `Index::move_file`, leaves the undated files, removes a
    created folder nothing moved into, and finally, when the original folder
    is empty, removes it (`std::fs::remove_dir`, which refuses a non-empty
    dir), drops its index state with `Index::forget_dir` and its watcher, and
    tells the frontend so; it returns a summary (files moved per folder, the
    failures with their messages, the files left in the original folder, and
    whether the original folder was removed). The sidecar writer is drained
    first and the `Scans` lock held across the disk ops and the index writes;
    the open folder's watcher and the tree watches under `dir` are released
    before the moves (they are set again by the reopen / tree re-render, or
    put back on failure as `rename_folder` does); a failure on one file does
    not stop the rest (as `trash::run`), and a group whose sidecar fails is
    rolled back by `file_run`. Unit tests cover the move planning (targets,
    sidecar pairing, collision refusal, rollback) with an injected `rename`
    closure as `rename.rs`'s tests do, and the "remove when empty" rule on a
    temp dir. `mise run ci` passes.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 and 2 are merged.
    - Do not duplicate `rename_folder`'s body: lift it into a crate-private
      fn in `rename.rs` (e.g. `pub(crate) fn rename_folder_locked(app, dir,
      name) -> Result<Renamed, String>`) that both the `rename_folder` command
      and the one-date branch call, keeping the command's signature and
      behavior unchanged (its tests still pass).
    - The split branch follows the guard order of `rename_folder` and
      `trash_files` (`docs/agents/tauri-app.md`, "An index/scan mutation that
      must not race a rescan ..."): refusal checks first, `spawn_blocking`,
      writer flush, `Scans` lock, `stop_embedding`, then disk, then index.
      Use `watch::release_under(&app, dir)` and run the moves and the removal
      inside `treewatch::with_released(&app, dir, || ...)` as `rename_folder`
      does (Windows refuses to remove or rename a watched folder).
    - Each file's move plan is `(from, to)` pairs RAW first, the sidecar
      targets minted from the new RAW path with `SidecarFormat::sidecar_path`
      (a `.dop` embeds the RAW's whole name), using `sidecar::existing_sidecar`
      for both formats; run them with `rename::file_run` for its rollback.
      `std::fs::rename` is enough: the siblings are on the same volume.
    - The recorded trash runs (`Runs`) reference files by path; a moved
      file's undo would restore it to its old path (the same limitation
      `rename_file` has today). Do not extend `Runs`; note it in
      `learnings.md` and the docs (Step 5).

- [ ] Step 4: The `Rename by Capture Date…` item in the folder tree and its confirmation dialog
  - Done when: the folder menu (`context.ts` `folderMenuGroups`) offers
    `Rename by Capture Date…` right under `Rename…` (not on a root); choosing
    it calls `date_folder_preview` and shows a dialog in the style of the
    sidecar / sequence dialogs: for one date `Rename <name> to 2026-10-31?`
    with the file count, for several dates one row per target folder with its
    file count, a row listing the undated files that stay, a note that the
    original folder is removed if it ends up empty, and any collision shown
    as the reason the run button is disabled; `Run` invokes `date_folder_run`;
    the tree follows the result (one date: `folders.renamed` + the reopen
    `renameFolder` in `main.ts` does; split: `folders.refresh` of the parent,
    and when the open folder was the split one, a resync if it still exists
    or `openDirectory` of the first date folder if it was removed); errors
    go to the status line; a press during a scan is refused with the scan
    message like the sidecar commands (not held through `idle.ts`, since the
    preview could go stale); the dialog's pure text helpers live in
    `crates/app/ui/src/datefolder.ts` with Vitest tests for the rows, the
    title line, the undated list and the disabled reason. `mise run ci`
    passes.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 3 is merged.
    - Model the flow on `sidecars.ts` + `rewriteSidecarsIn` in `main.ts` and
      the dialog markup on `sequence-dialog` / the sidecar dialog in
      `crates/app/ui/index.html`, with the shared component classes of
      `style.css` (`docs/agents/ui-styling.md`). The menu guard in `main.ts`
      (`sequenceFlow.busy || ...`) gets this flow's busy flag too, so two
      folder operations cannot overlap.
    - The one-date result reuses the existing post-rename handling in
      `main.ts` `renameFolder` (tree update, `opensTarget` reopen, trash
      entries' folders): factor that continuation out so both paths share it
      rather than copying it.
    - Add the new action to the `showMenu` `switch` near line 3531.

- [ ] Step 5: Documentation
  - Done when: `CLAUDE.md`'s layout lists `src/datefolder.rs` and
    `ui/src/datefolder.ts` in the style of the neighbors; `README.md` /
    `README.ja.md` mention the item next to `Rename…` in the folder-tree
    bullet; `docs/humans/usage.md` / `usage.ja.md` describe the item in the
    folder menu paragraph and in a short `**Rename by Capture Date…**`
    section (one date vs. several dates, sibling date folders, undated files
    stay, the empty original removed, collisions refuse, not recursive, JPEG
    folders, that the index follows, and the trash-undo limitation);
    `mise run ci` (incl. lychee) passes.
  - Implementation approach (as far as it is known; omit if unknown):
    - Keep each English / Japanese pair in sync in the same PR, per
      `CLAUDE.md`'s Language rule.

## Trade-offs and risks

- **Day boundary**: the date is the Exif local date as the camera wrote it;
  a shoot across midnight becomes two folders. Not configurable.
- **Trash undo after a split**: a recorded trash run that moved a file out
  of the folder before the split restores it to the old path, which may no
  longer exist if the original folder was removed (`restore` never
  overwrites and reports the failure). Same limitation as `rename_file`;
  documented, not fixed.
- **Partial split**: when some files fail to move, the original folder is
  not empty and stays, with the failures listed; running the item again on
  it refuses (the date folders now exist as siblings). Acceptable for now;
  a later merge mode could lift it.
- **Windows watchers**: the removal of the emptied folder needs the tree
  watches under it released (existing `treewatch::with_released`); verify on
  Windows that an expanded original folder can be removed.

## Progress

- (2026-11-05) Step 1 complete
- (2026-11-06) Step 2 complete
