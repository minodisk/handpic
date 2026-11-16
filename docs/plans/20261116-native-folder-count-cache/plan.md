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

# A per-folder count cache table for the folder tree (native app)

## Purpose

The GPUI app's folder tree shows, on a row not listed in this session, the
number of `files` rows the index holds for that folder
(`Index::dir_counts`, a `GROUP BY dir` over `files`; see
`../_archived/20261115-native-tree-index-counts/plan.md`). That has three
limits this work removes:

1. The read scans every `files` row (through the `files_dir` covering index)
   while holding the reader mutex the thumbnails share, so its cost grows
   with the file count, not the folder count.
2. A folder that was only expanded (listed by `folders::list`, never opened
   and scanned) has no `files` rows, so its count is gone after a restart.
3. The count disappears when the 30-day eviction or the size cap drops the
   folder's `files` rows.

After this work the index database holds a `folder_counts` table, one row per
folder whose count was learned from disk (the `listing::Media` rule: its RAW
files, or its JPEGs when it holds no RAW). The tree writes a row whenever it
learns a count (an expand, a `Refresh`, a tree watcher change, an open's
listing) and reads the whole table once at startup. A folder expanded once
shows its count after a restart, the read never touches `files`, a rename
and a date split carry the rows, and the eviction keeps them. What the user
sees is otherwise unchanged: the listed count wins in the session, the same
pill is drawn expanded or collapsed, and a folder never counted shows no
count. The one visible difference is at the upgrade: the table starts empty,
so a count appears again only once its folder has been listed (decided by
the user, 2026-11-16). The Tauri app (`crates/app`) is frozen and is not
changed.

What the investigation found (the facts the steps rely on):

- Schema versioning (`crates/backend/src/index.rs`): `SCHEMA_VERSION` (25
  today) is stored in `PRAGMA user_version`; `prepare` lists every version it
  can migrate (0, 2 to 24, `SCHEMA_VERSION`), creates each table with
  `CREATE TABLE IF NOT EXISTS`, applies per-version steps and sets
  `user_version`. A version not in the list makes `open` discard the
  database. So v26 needs 25 added to the accepted list, the new table in the
  `CREATE TABLE IF NOT EXISTS` batch, and the `SCHEMA_VERSION` doc comment
  extended. The existing migration tests (`user_version = N` fixtures) are
  the model for the new one.
- `Index::evict` and `Index::clear` delete per folder through
  `evict_folder` (`files`, `embeddings`, clean `ratings`, the `folders`
  row); `clear` also deletes every `views` row. The native app has no Clear
  Cache; `Index::clear` is called only by the frozen Tauri app, which shares
  `riffle-backend`.
- `Index::rename_dir(old, new)` and `Index::forget_dir(dir)` take the
  canonical spelling (`listing::canonicalize`, with the `\\?\` prefix on
  Windows). `rename::rename_folder(cx, dir, name)` receives `dir` in the
  caller's spelling (the native app passes the tree's) and builds `target`
  from it. `datefolder::split_folder(cx, dir, caller, plan)` gets both the
  canonical `dir` and the `caller` spelling, and returns `Split.moved`, each
  `Moved` holding the new folder's `path` in the caller's spelling and its
  `count`; a split's target folder is always new (`create_dir` fails on an
  existing one).
- Tree nodes are keyed by the tree's own spelling (the root's, then one
  separator per level, respelled from the listings). `Tree::set_index_counts`
  keys its map by `fold(shown_path(dir))` and `count_of` looks a node up by
  `fold(node.path)`; `Tree::set_listed_count` finds the open folder's node by
  `relation` from `tree_open`.
- The native app's writable connection is `self.index`
  (`Result<Arc<Mutex<Index>>, String>`), shared with the scan thread, which
  locks it per batch; the view already writes through it off the UI thread
  (`set_last_viewed`, `mark_viewed` on `background_spawn` under
  `index::lock`, serialized one in flight at a time by `viewed.rs`'s `Latest`
  and `Batches`). The launch's eviction holds the same mutex for its
  `VACUUM`. All five `folders::list` calls of `main.rs` answer through
  `tree_listed`, and the open's count reaches the tree in `opened`
  (`set_listed_count`).

## Steps

- [x] Step 1: Add the `folder_counts` table to the index, with its write, read, eviction and clear rules
  - Done when:
    - `SCHEMA_VERSION` is 26; `prepare` creates
      `folder_counts(dir TEXT PRIMARY KEY, count INTEGER NOT NULL)` and
      accepts a v25 database, migrating it in place and keeping `files`,
      `ratings`, `folders`, `embeddings` and `views`
    - The table starts empty on a new and on a migrated database; nothing is
      seeded from `files`
    - `Index` has a write that upserts a batch of `(dir, count)`, a count of
      zero deleting the row, and a read-only method returning every row as
      `(dir, count)` in `dir` order, reading only `folder_counts`
    - `Index::evict` leaves `folder_counts` untouched; `Index::clear` deletes
      every row of it
    - `Index::dir_counts` (the `GROUP BY` over `files`) still exists and the
      native app still uses it; nothing in `crates/app` or `crates/native`
      changes
    - Unit tests in `index.rs` cover: write then read; a second write
      replacing a count; a zero deleting the row; a v25 fixture holding
      `files`, `ratings` and `folders` rows migrating to v26 with those rows
      kept and `folder_counts` empty; `evict` of a folder keeping its count
      row; `clear` dropping all
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - `crates/backend/src/index.rs` only. The table stores `dir` as an
      opaque string: the backend applies no canonicalization, prefix strip or
      case fold to it (the key is the caller's spelling, the tree's; see
      Step 3)
    - No per-version migration step is needed beyond the
      `CREATE TABLE IF NOT EXISTS` and the accepted-version list
    - The read's `ORDER BY dir` is for a deterministic result and test only
      (free from the primary key); nothing depends on the order
    - Name the new read distinctly from `dir_counts` in this step (both
      exist until Step 3 removes `dir_counts`)
    - Errors as `String`, in the style of `rank_dirs` / `set_last_viewed`;
      the write in one transaction
    - Whether `clear` adds the deleted count rows to `EvictSummary.rows`
      (which triggers the `VACUUM`) as it does for `views`: decide in the
      step and say so in the method's doc comment
    - Extend the `SCHEMA_VERSION` doc comment with the v26 sentence in the
      existing style
- [x] Step 2: Carry the count rows through a folder rename and a date split
  - Done when:
    - `rename::rename_folder` moves the `folder_counts` rows of the renamed
      folder and of the folders under it to the new path, in the caller's
      spelling (`dir` to `target`); rows already stored under the new path
      are dropped first, as `rename_dir` does for the other tables
    - `datefolder::split_folder` writes a count row for each date folder
      that received files (`Moved.path`, `Moved.count`) and deletes the
      original folder's row under the `caller` spelling
    - Unit tests: in `index.rs`, the rename of count rows (the folder itself,
      a folder under it, a prefix sibling such as `photos2` untouched, a row
      already under the target dropped, `_` and `%` in a name); in
      `rename.rs` / `datefolder.rs`, a rename carrying the count and a split
      giving each date folder its count and dropping the original's
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged
    - `crates/backend/src/index.rs`, `rename.rs`, `datefolder.rs`. Add one
      `Index` method that moves count rows from `old` to `new` with the same
      `substr` prefix match and `std::path::MAIN_SEPARATOR` rule as
      `rename_dir`, in its own transaction, and call it once from
      `rename_folder` with the caller's `dir` and `target`. Do not add
      `folder_counts` to `rename_dir`'s or `forget_dir`'s table lists: those
      take the canonical spelling, which the table never holds
    - Both functions' calls sit inside the `index::lock` they already take,
      and a failure joins the existing warning path (the disk changed, the
      cache did not follow)
    - Verify that `Moved.count` equals what `folders::list` would count for
      the new folder (the plan's groups come from the `Media` listing, a RAW
      with its sidecars counting once); if it does not, write no row for the
      date folders rather than a wrong one, and note it in `learnings.md`
    - A source folder that remains after a split (undated files left) loses
      its row rather than getting a guessed number; its next listing writes
      the true one
    - The Tauri app calls the same two functions; its table is empty (it
      never writes counts), so the calls are no-ops there. Do not edit
      `crates/app`
- [ ] Step 3: Write and read the tree's counts through the cache table in the native app, and drop `Index::dir_counts`
  - Done when:
    - Every count the tree learns from disk is written to `folder_counts`
      off the UI thread: each successful `folders::list` answer (an expand,
      `Refresh`, a tree watcher change, `Expand All`, a reveal's chain) and
      an open's or re-listing's count, keyed by the tree node's own path
    - The tree's stored counts come from one read of `folder_counts` on the
      reader connection, at startup and after `folder_renamed` /
      `date_folder_ran`; the reads at the end of `opened` and on pass 1's
      `Event::Done` are gone
    - A folder expanded once (never opened) shows its count on a collapsed
      row after a restart; a folder reached through a mapped drive, a
      symlink or a junction does too
    - `Index::dir_counts` and its test are removed, and nothing reads a
      count from `files`
    - Unchanged for the user: the listed count wins in the session (a listed
      zero showing nothing), the same pill expanded or collapsed, no count
      for a folder never counted, nothing written without an index
    - `tree.rs` unit tests are adjusted to the new source: the stored keys
      are tree spellings matched by `fold` (case ignored only when asked);
      the two tests matching a `\\?\C:\...` and a `\\?\UNC\server\share\...`
      dir go with the strip; `set_listed_count` reports the node path it set
      (and none when no node matches)
    - `../../agents/native-app.md` (the two count rules under "The folder
      tree is a `uniform_list` over rows kept in the view"), the `CLAUDE.md`
      layout text (`src/index.rs`: the `folder_counts` table and its
      eviction / clear rule; `src/tree.rs`: the cached counts in place of
      the index's row counts; `src/rename.rs` / `src/datefolder.rs`: the
      carried count rows) and the `todo.md` item "Native app: real-device
      check of the folder tree's file counts on collapsed rows" describe the
      cache table
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 and 2 are merged
    - `crates/native/src/main.rs`, `tree.rs`, `folderops.rs` (only if the
      read method is renamed), `crates/backend/src/index.rs` (the removal),
      the docs
    - Key: the tree node's path, the string handed to `folders::list`. No
      `canonicalize` is added. For the open's listing use the path of the
      node `set_listed_count` matched (have it return the matched path), not
      `tree_open` as given and not the canonical `listed.dir`, so every key
      in the table is a node path and a rename's prefix match (Step 2) finds
      it. When no node matches yet (the reveal is still walking), skip the
      write: the reveal's own listing of that folder writes it
    - One choke point for the tree listings: `tree_listed`'s `Ok` branch
      (it needs the means to queue the write; it has no `cx` today)
    - Serialize the writes as `viewed.rs` does (`Batches` / `Latest`): one
      write in flight on `background_spawn` under `index::lock` of
      `self.index`, the counts learned meanwhile coalesced per folder with
      the latest winning, so two listings of one folder cannot land out of
      order. The mutex already serializes them with the scan's batches and
      the launch's eviction; no idle gate and no scan refusal is needed, and
      a write must not wait for the eviction on the UI thread
    - Reading: keep the map in `Tree`, keyed by `fold(dir)` alone. Drop the
      `shown_path` strip from the keying: no canonical row exists any more,
      every key being a node path. `context::shown_path` itself stays for
      its other callers; remove only the import `tree.rs` no longer uses.
      Rename `set_index_counts` / `Indexed` / `read_dir_counts` and their doc
      comments so they no longer speak of the index's row counts or the
      canonical `dir`. Keep the stale-answer counter
    - No re-read after a write: a listed node shows `Node::raw_count`, and
      the map only serves nodes not listed in this session. The re-read
      after a rename or a split stays because the keys moved (a split's new
      date folders are not listed and take their counts from the table)
    - A write still queued at quit is lost unless the quit handler drains
      it; check whether the existing `on_app_quit` drain of the judgment
      stores can cover it cheaply, else accept the loss (a cache) and say so
      in `native-app.md`
    - `native-app.md`: replace the "index's row count" rule and the
      "read whole (`GROUP BY dir` ...)" rule; drop the sentences about the
      verbatim prefix match and about a mapped drive, symlink or junction
      never matching; mark the new rules Inferred until seen in the running
      app, as the file does
    - `todo.md`: rewrite the item's background and steps for the cache table
      (after the upgrade no collapsed row shows a count until listed; expand
      a never-opened folder, restart, expect its count; a renamed and a
      date-split folder keep theirs; the mapped-drive limit and the
      `IN (...)` fallback sentences removed). Keep it one unchecked item
    - `../../humans/usage.md` and its `.ja.md` are left as they are, as in
      the previous plan

## Trade-offs and risks

- **Key spelling: the tree's, not the canonical `files.dir` one** (decided
  by the user, 2026-11-16). It needs no extra disk access, is exactly the
  string the lookup uses, and gives a folder reached through a mapped drive,
  a symlink or a junction its count after a restart. Its cost: one physical
  folder reached through two spellings has two rows, each right for its own
  tree row as of its last listing. The canonical key would have meant one
  `canonicalize` per tree listing and alias paths still without a count
  after a restart.
- **The table starts empty at the upgrade** (decided by the user,
  2026-11-16). Every count on a collapsed row vanishes once: a folder shows
  its count again after it has been expanded, refreshed or opened. Seeding
  from `files` would have kept them, at the price of canonical rows beside
  the tree-spelled ones, a `counted_at` column to pick between them and a
  second rename call.
- **Two node spellings that differ only in case.** On Windows and macOS the
  map folds case, so if a folder was once listed under `C:\Photos` and later
  under `c:\photos` (a root spelled differently in another session), both
  rows fold to one key and the one later in `dir` order wins, which may be
  the older count. It is corrected in the session by the folder's next
  listing (the listed count wins) but the stale row stays. Accepted as rare;
  the fix would be a `counted_at` column.
- **Eviction and Clear Cache.** The eviction (30 days, size cap) never
  touches `folder_counts`, which is goal 3 and costs a few dozen bytes per
  folder; `Index::clear` empties it, since Clear Cache means "drop
  everything cached" (as it does for `views`). Only the frozen Tauri app
  calls `clear`, and it never fills the table, so today the clear rule is
  exercised by tests only.
- **Rows are never pruned.** A folder deleted or renamed outside the app
  leaves its row; it is never drawn unless a folder of the same path appears
  again, which then shows the old number until listed. Same class as "a
  collapsed folder's count can be old", accepted in the previous plan.
- **Schema bump on the shared backend.** v26 migrates the Tauri app's own
  index file too on its next launch (in place, nothing lost). A binary built
  before this work that opens a v26 database discards it as an unknown
  version; that affects only someone running an older build against the
  same index file.
- **Order of the steps.** Between Step 1 and Step 3 the table exists but is
  neither written nor read by the app; the carry of Step 2 runs on an empty
  table. Nothing user-visible changes until Step 3, which is also when the
  counts on collapsed rows vanish once.

## Progress

(2026-11-16) Step 1 complete
