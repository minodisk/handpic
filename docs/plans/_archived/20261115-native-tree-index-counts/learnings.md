# Learnings

## Step 1: `Index::dir_counts`

- Measured: `EXPLAIN QUERY PLAN SELECT dir, COUNT(*) FROM files GROUP BY dir
  ORDER BY dir` on a fresh index (the bundled SQLite) gives one line,
  `SCAN files USING COVERING INDEX files_dir`: answered from the `files_dir`
  index alone, no table scan and no temp B-tree for the `GROUP BY` or the
  `ORDER BY`. Checked once through a temporary print in the unit test, which
  was then removed (asserting the plan text would tie the test to the SQLite
  version's wording).
- The method orders by `dir` (free, since the index is already in `dir`
  order) so the result and the test are deterministic.
- A failed extraction's row (`write_batch` with an `Err(Failure)`) counts
  like any other, and `reconcile` with an empty listing deletes every row of
  the folder, so it drops out of the result.

## Step 2: the count on every drawn row

- Measured (2026-11-16, Windows 11, Python's bundled SQLite 3.50.4 on a copy
  of the real native index, `index.sqlite` plus its WAL, about 250 MB):
  10,634 `files` rows in 15 folders; `SELECT dir, COUNT(*) FROM files GROUP
  BY dir ORDER BY dir` took 0.70-0.83 ms over five runs, its plan `SCAN files
  USING COVERING INDEX files_dir`. So a thumbnail read that waits behind it on
  the reader mutex waits about a millisecond; the whole-table form stays, no
  fallback to the `IN (...)` variant. Not measured inside the running app.
- The verbatim strip is `context::shown_path`, called by
  `Tree::set_index_counts` itself rather than by `main.rs`, so the `tree.rs`
  tests can hand it `\\?\C:\...` and `\\?\UNC\...` dirs. `tree.rs` now
  imports `crate::context`.
- The open folder's listed count is taken as `listed.files.len()` before
  `listed` is consumed (`Scan::start` takes `listed.dir`), not `self.all`,
  which `--max` can shorten.
- The startup read cannot run inside `Riffle::new` (no `self` yet), so `new`
  spawns a task that updates the view once it exists.
