# Learnings

## Step 1

- The new methods are `Index::set_folder_counts(&[(String, usize)])` (one
  transaction; an upsert per row, a zero deleting it) and
  `Index::folder_counts()` (every row in `dir` order). Step 3 calls these.
- `Index::clear` adds the deleted `folder_counts` rows to `EvictSummary.rows`,
  as it does for `views`, so clearing only counts still `VACUUM`s; the doc
  comment says so. `evict` and `evict_folder` do not touch the table.
- The v25 migration test builds its fixture by opening a fresh (v26)
  database, then `DROP TABLE folder_counts; PRAGMA user_version = 25;`, the
  same trick the v24 test uses with its dropped column. `DirtyRow` is a
  tuple, not a struct.
- No per-version migration step was needed: the `CREATE TABLE IF NOT EXISTS`
  and adding `25` to the accepted list are enough.

## Step 2

- The new method is `Index::rename_folder_counts(old, new)`: `rename_dir`'s
  delete-under-`new`-then-`substr`-update rule on `folder_counts` alone, in
  its own transaction. `rename::rename_folder` chains it after `rename_dir`
  under the same lock with the caller's `dir` and `target` (as spelled from
  `dir`), so either failure takes the existing warning path.
- `Moved.count` matches the `Media` rule `folders::list` counts: a plan's
  groups come from `read_listing` (the RAWs, or the JPEGs of a JPEG-only
  folder), each moved file counts once and only its sidecars go along, and a
  split's target folder is always new. The `datefolder` test asserts
  `folders::list(..).raw_count` equals each `Moved.count`.
- `split_folder` writes the date folders' rows and a zero for `caller` (which
  deletes the original's row, whether or not undated files keep the folder)
  in one `set_folder_counts`, after the `move_file` / `forget_dir` writes and
  only when they succeeded. `moved` is now built before the index block.
- The index test's expected order is sorted at run time: `MAIN_SEPARATOR`
  (`\` on Windows, `/` elsewhere) sorts on opposite sides of `2`, so a fixed
  literal order would pass on one platform only.
- `rename_folder` and `split_folder` had no end-to-end test before; a
  `Context` for one is `Watch::spawn(|_| {})`, `TreeWatch::spawn(|_| {})`,
  `Runs::default()` and an `Index` in a temp dir.

## Step 3

- `Tree::set_listed_count` now returns every node path it set (a `Vec`,
  since with case folding more than one node can match); `opened` queues a
  count write for each and none when the reveal has not reached the node.
- `tree_listed` took a `cx` parameter so its `Ok` branch can queue the
  write; all five callers already had one.
- The coalescer is `tree::CountWrites` (in `tree.rs`, beside the cached map,
  rather than in `viewed.rs`): unlike `Batches` it hands out the whole
  pending set as one `set_folder_counts` batch, a folder's later count
  replacing its earlier one in place.
- Quit drain: not done. The `on_app_quit` drain covers `judgment::Queue` and
  the sidecar `Writer`, both thread-backed queues; the count writes are
  `background_spawn` tasks driven from the view, so draining them would mean
  a new thread-backed queue. A lost write is a cache miss the folder's next
  listing repairs; `native-app.md` says so.
- Writing a Python replacement script through a Bash heredoc turned `\?` in
  a raw string into `\?`; the verbatim-prefix test edits were done with
  `sed` instead. Also, `s.index(...)` for an end marker must search from the
  start marker (`s.index(end, a)`): the first try duplicated half of
  `native-app.md` because the "end" heading also occurs earlier.

## Deferred issues (todo candidates)

- Pending manual check (Step 3): the real-device check of the folder tree's
  cached counts on Windows, macOS and Linux (an upgraded index showing no
  counts at first; a folder expanded once and never opened showing its count
  after a restart; `Refresh` updating it across a restart; a renamed folder
  and a date split's new folders keeping or getting theirs; a mapped drive,
  symlink or junction folder keeping its count after a restart). The steps
  and expected results are in `todo.md`'s rewritten item "Native app:
  real-device check of the folder tree's file counts on collapsed rows"
  (basis: the plan's Step 3 Done-when; files `crates/native/src/main.rs`,
  `crates/native/src/tree.rs`, `docs/agents/native-app.md`). The step's
  checkbox was ticked on the automated criteria (`mise run ci:local`, unit
  tests).
