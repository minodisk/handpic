# Learnings

## Step 1

- `Index::prune_folder_counts` selects the rows under the prefix with the
  `substr(dir, 1, length(?1)) = ?1` rule and decides in Rust against a
  `HashSet` of the kept paths, deleting by exact `dir`, so no parameter is
  bound per child and `_` / `%` need no escaping. `substr` / `length` count
  characters, but a row they match starts with the prefix byte for byte, so
  slicing the row at `prefix.len()` bytes is safe. The trailing-separator
  check uses `std::path::is_separator`, so `C:\` and `/` get no second one.
- `CountWrites` became an ordered `Vec<CountWrite>` (`Count` / `Prune`). A
  re-pushed count is removed and appended, which changed the expected order
  of the existing coalescing test (`/a` then `/b` instead of `/b` then `/a`).
  A second prune of the same parent is appended, not merged; both are
  correct, and appending keeps `push` and `prune` symmetrical.
- `write_folder_counts` folds consecutive counts into one
  `set_folder_counts` and flushes them before each prune under one
  `index::lock` guard; the first error stops that batch (logged as before)
  and the next batch still goes out on `settled`.
- The tree's test module already has a `count(tree, path)` helper, so the
  new entry builders are named `counted` / `pruned`.

## Deferred issues (todo candidates)

- Pending manual check (Windows, macOS, Linux; the step was ticked on the
  automated criteria): with `mise run native:dev`, delete or rename a counted
  folder outside the app, expand or `Refresh` its parent, quit and restart,
  then create a folder of the old name. Expected: no count on its collapsed
  row until it is listed. Added to the existing `todo.md` item "Native app:
  real-device check of the folder tree's file counts on collapsed rows".
  Basis: Step 1's prune (`crates/native/src/main.rs` `prune_counts`,
  `crates/backend/src/index.rs` `prune_folder_counts`).
