# Learnings: chunked-sidecar-delete

## Step 1

- `foldersidecars::delete` now takes the move as an
  `FnOnce(Vec<Group>) -> (Summary, Vec<Trashed>)`, the shape of
  `trash::redo_back`'s `again`; `delete_run` passes
  `trash::move_groups`, now `pub(crate)`. The two existing tests only wrap
  their mover in `trash::run`, their assertions unchanged.
- The comparison test's fake mover returns `Ok(None)` (as the Windows
  `trash_one` does), so whole `trashed_at` values compare equal between
  `run` and `run_chunked`. Its fake `move_many` errs on the `fail` file
  after renaming the rest, so `move_chunk`'s per-file fallback is on the
  compared path. With sidecar-only groups the whole run is one
  `move_many` call; the second `move_chunk` of the chunk gets no file and
  makes no call.
- Decision 3, pinned: a `.xmp` shared by `a.ARW` and `a.NEF` goes into
  `move_many` twice; the second rename fails, the fallback finds the file
  gone and counts it moved, so the summary lists it twice, the run records
  it twice (both owned by `a.ARW`, the first match by path) and only
  `a.ARW` is cleared.
- A bare `python` in Git Bash hung (as `CLAUDE.md` warns); use
  `mise x -- python` or the Edit tool.

## Deferred issues (todo candidates)

- Pending manual check (Windows 11, the user's): on a folder of a few
  hundred sidecars, `Delete Sidecars…` should move them in about 7 ms a file
  (one shell operation per 100) rather than about 140 ms, the progress
  circle should step per chunk and end full, and Undo / Redo should restore
  and move them again. Basis: plan decision 2 and the todo item
  `Engine: Delete Sidecars… still moves one sidecar per shell operation on
  Windows`; `crates/engine/src/foldersidecars.rs`, `crates/engine/src/trash.rs`.
  Step 1's checkbox was ticked on the automated criteria.
