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
