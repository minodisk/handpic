# Learnings

## Step 1

- `delete_for_good`'s shared-`.xmp` filtering moved into `take_shared_xmps`,
  and the grouping, the move and the late move of a `.xmp` several moved RAWs
  share into `run_listed`, which takes the mover and a `move_all` closure
  given `&mut` that mover. That shape lets the delete hand its one remover to
  both `run` and the late move without two closures borrowing it, and lets
  `trash_listed` pass `move_groups` (which ignores the mover on every
  platform) with `trash_one` for the late move.
- On the chunked (Windows) path the late move of a shared `.xmp` goes through
  the single-file mover, never `move_many`: one extra shell operation per
  shared `.xmp`, as Trade-offs item 1 accepted.
- The late `.xmp`'s `Trashed` record is inserted after the last sharing RAW's
  record and the non-media records following it, so `items`,
  `item_starts`, `restore` and `redo_groups` still see it as that RAW's
  sidecar even when an unrelated RAW was moved after it.
- `cargo test -p handpic-engine --lib trash::` from a cold `target/` ran past
  the harness's 180 s foreground limit and was moved to the background.
