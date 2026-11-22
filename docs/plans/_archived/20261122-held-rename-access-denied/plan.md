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

# Held folder rename: no Access denied at pass 2's end

## Purpose

On Windows 11 a rename of the open folder confirmed during its scan is held
by the idle gate and runs when pass 2 ends. About one time in two it then
fails with `Access denied (os error 5)` in 1 ms, while the same rename by
hand right after succeeds (`todo.md`, "App: a held folder rename
intermittently fails with Access denied when it runs as pass 2 ends";
`docs/plans/_archived/20261122-windows-device-checks-3/learnings.md`). On
NTFS any open handle to a file inside a folder refuses the folder's rename.

Planning-time finding, from reading the code, not measured:

- The scan is not the holder. `Event::FacesDone` is sent after
  `index::run_faces_scan` returns, and `handpic_core::scan`'s
  `for_each_path` returns from `pool.broadcast` only once every worker's
  closure has ended; each file is opened, read and closed inside the
  per-file call. The watchers are released synchronously by
  `rename_folder` (`watch::release_under` with `settle`,
  `treewatch::with_released`), the thumbnails are read from the index, and
  nothing is memory-mapped.
- The likely holder is a prefetch. The `FacesDone` arm of `scanned` in
  `crates/app/src/main.rs` calls `self.finished(cx)`, which drains the gate
  and spawns the held rename on a background task (`run_rename` in
  `crates/app/src/folderops.rs`), and then `self.request_prefetch(cx)`,
  which spawns a background read of a neighbour's preview in the same
  folder. `request_prefetch` refuses while `scanning()`, so the first
  prefetch after the open starts exactly at pass 2's end, in the same UI
  turn as the rename, and it does not check for a rename in flight. The
  rename reaches `std::fs::rename` within a millisecond while that read
  has a file of the folder open.

Once this is done, a held rename runs with no read started by the scan's
end beside it, succeeds, and reopens the folder under its new name.

## Steps

- [x] Step 1: Start no prefetch while a folder rename is in flight
  - Done when:
    - The holder is confirmed or refuted and written down in this plan
      folder's `learnings.md`: what was tried to reproduce the failure
      before the fix (for example a UI test or a run of the app on the
      Windows host that holds a rename over a scan, repeated), what was
      seen, and whether the prefetch read is the handle. If it is refuted,
      stop and revise this plan before applying a fix.
    - `request_prefetch` starts no read while a folder rename is in
      flight, so the `FacesDone` that drains a held rename starts none.
    - Prefetching resumes after the rename settles: after a successful
      rename through the reopen `folder_renamed` already does, after a
      failed one explicitly.
    - Tests in `crates/app` cover: with a rename in flight
      `request_prefetch` leaves the ring with no read in flight; a rename
      held during the open's listing or scan runs at the scan's end, the
      folder is renamed on disk and reopened under the new name, and the
      ring fills again afterwards.
    - `docs/agents/app.md` records the cause and the rule (the "Seen on
      Windows 11 on 2026-11-22" bullet about the failed held rename, the
      bullet listing the reads inside the open folder under "A folder
      rename releases the watches, then follows in the tree and reopens",
      and the prefetch ring section's list of what calls
      `request_prefetch`).
    - `todo.md`: the item's two boxes are replaced by one pending manual
      check for the user (on Windows 11, a fresh index, a folder of 264
      ARWs: rename the open folder during its scan, at least five times on
      fresh folders; every held rename succeeds at pass 2's end and the
      folder reopens under its new name, the log showing `rename: ...
      renamed to ...` each time), with the Background updated to name the
      cause and the fix. The item is deleted only once that check has been
      done.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/main.rs` (`request_prefetch`),
      `crates/app/src/folderops.rs` (`run_rename`, `rename_ran`),
      `crates/app/src/rename.rs` (`RenamesInFlight`, which has `start`,
      `settle` and `blocks` but no "any in flight" question yet),
      `crates/app/src/uitest.rs`, `docs/agents/app.md`, `todo.md`.
    - `run_rename` already records the rename in
      `self.folderops.renames` before it spawns the background task, and
      `finished` runs before `request_prefetch` in the `FacesDone` arm, so
      a check of the renames in flight inside `request_prefetch` covers
      the held case without reordering the arm.
    - A failed rename ends in `rename_ran`'s `Err` arm
      (`settle_folder_run`), which does not re-list unless a resync was
      waiting; call `request_prefetch` there so the ring is not left
      empty. A successful one ends in `folder_renamed`, whose reopen
      clears and refills the ring.
    - Keep the guard to the folder rename. Other runs under the gate's
      in-flight count (trash, sidecars, Clear Cache) are not part of this
      item.
    - Test seam: inside one `Ui::update` closure, before the executor
      parks, `run_rename` followed by `request_prefetch` is deterministic;
      `Ring::still_wanted()` is true right after `Ring::next()` marked a
      read in flight, so it can serve as the "a prefetch is out" question
      without a new accessor. A rename requested in the same update as
      `open_from_tree` finds `resync.busy()` and is held. The generated
      DNGs of `uitest.rs` are tiny, so an end-to-end race on them is not
      expected to reproduce the Access denied reliably; the ordering test
      is the regression test, the end-to-end one checks the outcome.
    - What to verify during implementation: that a prefetch can be in
      flight when `request_prefetch` is reached at `FacesDone` only
      through that call (no prefetch survives from before the scan); and
      whether a preview read, an on-demand faces / eyes read or a 1:1
      crop read in flight at that instant showed up in the reproduction.
      Record both in `learnings.md`.
    - The manual Windows real-device repeat is the user's. Do not tick it
      on the automated criteria.

## Trade-offs and risks

- The cause is inferred from the code, not measured. If the reproduction
  before the fix shows another holder, or the real-device repeat still
  fails after the fix, this plan needs a revision (the retry below, or a
  rename that waits for every read in flight).
- A bounded retry in `rename_folder` (`crates/engine/src/rename.rs`) on
  `PermissionDenied` / a sharing violation, Windows only, is not in the
  step. Taking it would also cover what the guard does not: a page turn's
  preview read, an on-demand faces or eyes read or a 1:1 crop read in
  flight at the instant of any rename (held or by hand), and another
  process (Windows Defender, the search indexer) opening a file
  transiently. Its costs: a real permission denial is reported later by
  the retry's bound, a further holder inside the app would be masked
  rather than found, and the `datefolder` rename shares the function and
  would change with it. Decided at approval: ship the guard, let the
  real-device repeat decide; add the retry only if a failure remains.
- A rename that waits for every read in flight (the pager's read, the
  ring's, the crop's, the on-demand judgments') before it runs would close
  the residual window at its root, but it needs a shared count of the
  view's reads that does not exist today; not taken as out of proportion
  to a one-in-two failure whose trigger is a single call.
- Guard scope: the step refuses a prefetch only while a folder rename is
  in flight. Refusing it while the gate's in-flight count is non-zero
  would be one check but changes the trash, sidecar and Clear Cache runs,
  which this item does not ask for and which mostly end in a re-list
  anyway.
- The residual window stays as `docs/agents/app.md` already describes it:
  a rename refused by a read in flight reports the OS error and leaves the
  folder, its watcher and its index rows as they were.

## Progress


- (2026-11-22) Step 1 complete
