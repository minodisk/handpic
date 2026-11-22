# Learnings: held folder rename, no Access denied at pass 2's end

## Step 1: the holder, confirmed

What was tried before the fix, on the Windows 11 host (NTFS), with throwaway
`#[ignore]` tests in `crates/app/src/uitest.rs` that were removed again:

- An end-to-end UI test: six generated DNGs of about 7 MB each (a noisy
  3000x2000 preview at quality 98) in a fresh folder, `open_from_tree` and
  `request_rename` in the same `Ui::update` (so the rename is held), then a
  wait for the rename to settle; five folders in a row. All five renamed,
  none failed. That is expected: on GPUI's test platform `background_spawn`
  runs on the deterministic test dispatcher, not on real threads, so the
  prefetch's read and the rename's background task never overlap in time.
  The UI test cannot reproduce the race itself.
- A temporary `log::info!` in `request_prefetch` at the spawn, with whether a
  folder rename was in flight, in that same test. On every one of the five
  runs the log showed, within about 10-25 ms of `scan faces: ... done=6 ...`,
  `prefetch start ...\photos<i>\1.dng renaming=true`, followed by the
  `rename: ... renamed to ...` line. So the first prefetch since the open
  starts in the `FacesDone` turn while the held rename is in flight, as the
  plan inferred from the code.
- A race on real threads: four real ARWs from
  `D:\Photos\tests\2026-06-05-focus-sample` copied into a temporary folder; a
  thread running `handpic_core::reader::read_preview` (what the prefetch's
  `load` opens the file with) over them while the test thread calls
  `std::fs::rename` on the folder, 40 times. The rename failed with
  `PermissionDenied` (os error 5) 39 times in 40; with no read in flight,
  40 renames in 40 succeeded.

Conclusion: confirmed. The prefetch started by `FacesDone` right after
`finished` drained the held rename is the handle that refused the rename on
the real device; the plan's fix applies unchanged.

What the plan asked to verify:

- A prefetch can be in flight at `FacesDone` only through that call: in the
  logged runs no prefetch started between the open and `FacesDone`
  (`request_prefetch` refuses while `scanning()`), and the open clears the
  ring, so a read left from the previous folder is of another folder.
- Other reads at that instant: none showed up. With five files the open's
  page read ended seconds before pass 2 did; the on-demand faces / eyes reads
  run only for the `f` overlay, which was off, and no 1:1 crop was asked for.
  They remain the residual window `docs/agents/app.md` describes.

Implementation notes:

- `Handpic::renaming` (in `folderops.rs`, since `folderops`' fields are
  private to that module) answers "a folder rename of the open folder or an
  ancestor is in flight" through `RenamesInFlight::blocks`, so a rename of an
  unrelated folder, which never reopens, holds no prefetch.
  `request_prefetch` refuses on it; `rename_ran`'s `Err` arm asks
  for a prefetch after `settle_folder_run`. A successful rename reopens the
  folder, whose `FacesDone` asks for it again.
- The regression test (`a_folder_rename_in_flight_starts_no_prefetch`) fails
  with the guard removed (`assertion failed: !started`) and passes with it.
  It first shows the control: with no rename, an emptied ring starts a read
  (`Ring::still_wanted` true right after `request_prefetch`).
- The end-to-end test needed a non-mutating "the ring holds this file"
  question; `Ring::holds` was added under `#[cfg(test)]`.
- One run of `cargo test uitest::` failed in an unrelated test,
  `the_paging_keys_move_through_the_keymap_and_wrap`, with `the quit handlers
  took 1.28s` (over `QUIT_MAX`, 1 s), on the first run after a build. Three
  reruns passed all ten UI tests; it looks like load on this host, not this
  change.

## Deferred issues (todo candidates)

- Pending manual check (Windows 11, real device): with a fresh index, open a
  folder of 264 ARWs and rename it from the tree during its scan, at least
  five times on fresh folders. Expected: every held rename succeeds at pass
  2's end and the folder reopens under its new name, the log showing
  `rename: ... renamed to ...` each time. The step's checkbox was ticked on
  the automated criteria only. Recorded in `todo.md` under "App: a held
  folder rename intermittently fails with Access denied when it runs as pass
  2 ends". Files: `crates/app/src/main.rs`, `crates/app/src/folderops.rs`.
- Possible flake: `uitest::the_paging_keys_move_through_the_keymap_and_wrap`
  failed once on `QUIT_MAX` (`the quit handlers took 1.28s`) right after a
  build, then passed three times. Basis: this step's local test runs. File:
  `crates/app/src/uitest.rs`.
