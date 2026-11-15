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

# File counts on every folder tree row the index knows (native app)

## Purpose

The GPUI app's folder tree shows a folder's file count only on an expanded
row, because the count comes from the one `read_dir` an expand does
(`riffle_backend::folders::list`) and probing every unexpanded child would
cost a `read_dir` each (slow on a network share). The SQLite index already
holds one `files` row per listed file of every folder opened before, so a
folder that was scanned once can show its count without touching the disk.

After this work a drawn tree row shows its count, expanded or collapsed,
whenever the count is above zero: from the index for a folder not listed in
this session, and from `read_dir` once the folder has been expanded or
opened. Both are drawn the same. A folder with no index rows that has not
been listed shows no count, as today. The Tauri app (`crates/app`) is frozen
and is not changed.

What the investigation found (the facts the steps rely on):

- `files` has `dir TEXT NOT NULL` and `CREATE INDEX files_dir ON files (dir)`
  (`crates/backend/src/index.rs`, `prepare`).
- The row count of a folder equals its listing once its pass 1 has finished:
  `write_batch` writes a row for a failed extraction too (the `error`
  column), `reconcile` deletes the rows of files no longer listed, and both
  the tree's `folders::list` and the open's `scan::list` / `read_listing`
  go through `listing::Media` (the RAW files, or the JPEGs of a folder with
  no RAW). The count is lower than the listing after a scan cancelled
  mid-pass, and absent after the 30-day eviction dropped the folder's rows.
- The index's `dir` is `listing::canonicalize` output: on Windows it carries
  the verbatim prefix (`\\?\C:\...`, `\\?\UNC\server\share\...`). Tree nodes
  are keyed by the tree's own spelling (the root's, then one separator per
  level; see "The folder tree is a `uniform_list` over rows kept in the
  view" in `../../agents/native-app.md`), compared with `tree::relation` /
  `fold`, case ignored on Windows and macOS (`tree::IGNORE_CASE`).
  `context::shown_path` strips the verbatim prefix.
- Every `open` already reveals the folder, and `reveal` expands and runs
  `folders::list` on each folder of the chain including the opened one, so
  an open already recounts the folder by `read_dir`. A re-listing (`relist`,
  a watcher change) does not go through `reveal`.

## Steps

- [x] Step 1: Add the per-folder row count query to the index
  - Done when:
    - `Index::dir_counts(&self) -> Result<Vec<(String, usize)>, String>` in
      `crates/backend/src/index.rs` returns each `dir` that has `files`
      rows with its row count
    - A unit test in `index.rs` covers: two folders with different counts, a
      row written for a failed extraction counted, and a folder whose rows
      were all removed (`reconcile` with an empty listing) absent
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - `SELECT dir, COUNT(*) FROM files GROUP BY dir`, read-only (`&self`), so
      it runs on the `open_reader` connection; next to `rank_dirs`, in the
      same style (errors as `String`)
    - Verify with `EXPLAIN QUERY PLAN` that it is answered from `files_dir`
      (a covering index scan), not a table scan, and note the result in
      `learnings.md`
    - Use the test module's existing helpers (`temp_dir`, `open`,
      `write_batch` with an `Err(Failure)` row, `remove_temp_dir`)
    - Nothing in `crates/app` calls it; do not add a Tauri command
- [x] Step 2: Show the count on every drawn row, from the index until the folder is listed
  - Done when:
    - A collapsed or never-expanded tree row whose folder has index rows
      shows that count; a row shows its count whether expanded or collapsed,
      when above zero
    - Expanding a folder, opening it, `Refresh` and a tree watcher change
      replace the shown number with the `read_dir` count (the listed count
      wins over the index's, a listed zero showing nothing)
    - A re-listing of the open folder sets its row's count from the listing
    - A folder with no index rows that has not been listed shows no count
    - The index is read off the UI thread through the read-only connection;
      no folder is touched on disk for an index-derived count
    - `tree.rs` unit tests cover the merge: an unlisted node takes the index
      count; a `\\?\C:\...` and a `\\?\UNC\server\share\...` index dir match
      the tree's `C:\...` and `\\server\share\...` nodes; case is ignored
      only when asked; a listed count (zero included) wins; replacing the
      index counts drops a folder no longer in them; a renamed node keeps
      its listed count
    - `../../agents/native-app.md` and the `CLAUDE.md` layout paragraph for
      `src/tree.rs` describe the index-derived count
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged
    - `crates/native/src/tree.rs`: keep `Node::raw_count` as the listed
      (`read_dir`) count and add the index's counts to `Tree` as a map keyed
      by the folded path (verbatim prefix stripped, then `fold`), replaced
      whole by one setter; `rows()` gives each `Row` the listed count, else
      the index's. `Tree`'s functions take `ignore_case` as a parameter
      today (`rename_folder`), so pass it to the setter rather than reading
      the constant inside. Add a setter for one node's listed count that
      finds the node by `relation`-style comparison, for the open folder
    - The verbatim strip is `context::shown_path`'s rule; either call it in
      `main.rs` before handing the pairs to the tree or move the strip where
      both can use it, but do not duplicate the rule
    - `crates/native/src/main.rs`: one method that spawns
      `Index::dir_counts` on `background_spawn` under `index::lock` of
      `self.reader` (nothing when there is no reader), then sets the tree's
      index counts and calls `tree_changed`. Call it at startup, at the end
      of `opened` (after `Scan::prepare`'s reconcile dropped rows), on
      pass 1's `Event::Done` (the rows are committed by then), and after
      `folder_renamed` / `date_folder_ran` in `folderops.rs` (the index rows
      moved to another `dir`). No re-query on expand, roots load or
      `tree-changed`: the map already covers every folder
    - A later answer must not be overwritten by an earlier one: guard with a
      counter as the other reads do, or keep one in flight
    - In `opened`, set the listed count of the `tree_open` node to
      `listed.files.len()` (the same `Media` rule as `folders::list`); use
      `tree_open`, the tree's spelling, not `listed.dir`, which is canonical
    - The draw (`main.rs`, the `count` of a tree row, near line 4012): drop
      the `row.expanded &&` condition; keep the same pill for both sources
    - Measure the query on the real index (rows, folders, milliseconds, and
      whether a thumbnail read waits behind it on the shared reader mutex)
      and record it in `learnings.md`; if it is not cheap, fall back to the
      `IN (...)` variant in "Trade-offs and risks"
    - `docs/agents/native-app.md`: add the rule under "The folder tree is a
      `uniform_list` over rows kept in the view" (the count's two sources,
      the spelling match, when the index is re-read), marked Measured or
      Inferred as the file does

## Trade-offs and risks

- **Whole-table `GROUP BY` versus `WHERE dir IN (...)` for the drawn rows.**
  The request sketched the `IN` form; the user chose the whole-table form
  (2026-11-15). The index's `dir` is the canonical spelling and the tree's is
  not, so an `IN` list needs a rule from a tree path to the exact index
  string (prepend `\\?\`, the same case), which misses rows whose case
  differs and needs a re-query every time the drawn rows change. The
  whole-table form matches in Rust by folded path and needs no re-query on a
  tree change. Its cost grows with the index (bounded by the eviction's size
  cap) and is measured in Step 2. If it proves expensive: Step 1's method
  takes `&[String]`, Step 2 builds the candidates from the drawn rows and
  re-queries in `tree_changed`.
- **Folders that never match.** `canonicalize` resolves a mapped network
  drive to its UNC path, a symlink or junction to its target, and
  `/tmp` to `/private/tmp` on macOS, so a tree row reached through such a
  spelling finds no index count and shows one only once listed. Resolving
  the tree's paths would cost a disk access per folder, which the
  requirement rules out.
- **A collapsed folder's count can be old.** The listed count stays after a
  collapse and the folder is no longer watched; the index count is as old as
  the folder's last scan. Both are drawn the same by requirement. The number
  is corrected on the next expand, open or `Refresh`.
- **A cancelled scan undercounts** until the folder is listed or its scan
  completes, since only extracted files have rows.
- **The launch's eviction runs on its own thread**, so the startup query can
  read rows the eviction then deletes; the first `opened` re-reads after the
  eviction has been joined.
- **`../../humans/usage.md` and its `.ja.md`** say "An expanded folder shows
  how many RAW files it holds". Recent native-only changes did not edit the
  user guide, so this plan leaves it.

## Progress

(2026-11-16) Step 1 complete
(2026-11-16) Step 2 complete
