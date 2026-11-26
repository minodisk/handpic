<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Chunked move for Delete Sidecars on Windows

## Purpose

Close the `todo.md` item
`### Engine: Delete Sidecars… still moves one sidecar per shell operation on Windows`.

`foldersidecars::delete` moves each sidecar to the Recycle Bin in a shell
operation of its own (`trash::run` with `trash_one`). The
[todo-trash-quit-fixes plan](../_archived/20261123-todo-trash-quit-fixes/plan.md)
measured that per-operation cost at about 140 ms a file for the Undo of Move
to Trash, against about 6.5-7 ms a file once chunked. Once this is done,
`Delete Sidecars…` on Windows moves its sidecars through the same chunked
move `trash::trash_listed` uses (`run_chunked`, `CHUNK` files per shell
operation), with the same outcome as today: the same files moved and failed,
the same recorded run, and each `Trashed`'s `owner`.

Scope rules:

- Only this item. No other `todo.md` item, no change beyond what it asks.
- The step does not edit `todo.md`. The item's heading is deleted in the
  `develop` skill's wrap-up.
- macOS and Linux keep their per-file move, as `trash_listed` does (macOS
  needs where each file went; `::trash::delete_all` is a loop of single moves
  on Linux).

## Decisions

Taken at plan approval (2026-11-26), the recommended option each time:

1. Progress on Windows is called per chunk, as `run_chunked` gives it (the
   same as Move to Trash); it still counts sidecars and ends at
   `(total, total)`. No chunk size parameter.
2. The Windows timing of the todo item's third checkbox is not part of the
   step. The wrap-up deletes the item and files the hands-on Windows timing
   and progress-circle check as a pending manual check in `todo.md`.
3. A `.xmp` shared by two listed RAWs: `doomed` stays as is; the step pins
   the chunked behavior with a test and accepts the difference in this edge.
4. A sidecar gone between the listing and the move fails as `trash::GONE` on
   Windows, the message Move to Trash already gives.

## Steps

- [x] Step 1: Route `foldersidecars::delete` through the chunked move on Windows
  - Done when:
    - On Windows, `foldersidecars::delete_run` moves the sidecars through
      `trash::run_chunked` (up to `CHUNK` per shell operation, the per-file
      fallback after a failed chunk), not one `trash_one` per sidecar. macOS
      and Linux still move one file at a time.
    - The `(done, total)` progress still counts sidecars, `total` being the
      number of sidecars, and ends at `(total, total)`. On Windows it is
      called per chunk (decision 1).
    - Each recorded `Trashed` of the run carries its `owner` (the RAW the
      sidecar belongs to), a failure is still reported under the RAW's path
      with the `"<sidecar name>: <message>"` text, and `cleared` still lists
      each RAW that lost a sidecar once.
    - A unit test in `crates/engine/src/foldersidecars.rs` runs the same
      set of sidecars (some succeeding, at least one failing, more than one
      RAW, a RAW with two kinds under `Both`) through `delete` once over the
      per-file `trash::run` with a fake mover and once over
      `trash::run_chunked` with a fake `move_many` and the same fake mover
      as its fallback, and shows the two agree on: `summary.moved`,
      `summary.failed` (path and message), `cleared`, the run `Runs::record`
      stores (taken back with `Runs::take`), and each `Trashed`'s `owner`.
      It also covers a `move_many` that errs after moving part of its files
      (the Windows shell's behavior), so the fallback is on the compared
      path.
    - A test pins the chunked behavior of a `.xmp` shared by two listed RAWs
      (decision 3).
    - The existing tests of `delete`
      (`the_run_moves_in_order_and_reports_a_failure_under_its_raw`,
      `a_redo_with_the_folder_closed_clears_the_rows_the_undo_left_judged`)
      still pass, adapted only as far as a signature change of `delete`
      requires.
    - The doc comments that the change makes untrue are updated: `delete`
      and `delete_run` in `foldersidecars.rs` ("`progress` is called after
      each sidecar"), and in `trash.rs` the comments naming the callers of
      `trash_many` / `move_groups`. `CLAUDE.md` and `docs/agents/app.md` are
      updated only where a sentence becomes untrue.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/engine/src/foldersidecars.rs` (`delete`, `delete_run`,
      the tests), `crates/engine/src/trash.rs` (visibility and doc comments
      only). `crates/app/src/folderops.rs` calls
      `foldersidecars::delete_run(&dir, format, index.as_deref(), &runs, progress)`;
      keep that signature so the app does not change.
    - `delete` already turns each `Doomed` into a
      `Group { raw: <sidecar path>, sidecars: [] }` and everything after the
      move (the `owner` assignment, the failure re-keying under the RAW,
      `cleared`) works on the `(Summary, Vec<Trashed>)` the move returns,
      matching by path. Only the move itself has to change; do not rewrite
      the rest.
    - `trash::move_groups(groups, progress)` is already the per-platform
      dispatch `trash_listed` uses (`run_chunked` with `Path::exists`,
      `trash_many`, `trash_one` on Windows; `run` with `trash_one`
      elsewhere). It is private to `trash.rs` today. Reuse it (for example
      as `pub(crate)`) rather than adding a second `cfg(windows)` dispatch
      in `foldersidecars.rs`.
    - One shape that matches the existing code is `trash::redo_back`'s
      `again` closure: `delete` takes an
      `FnOnce(Vec<Group>) -> (Summary, Vec<Trashed>)`, `delete_run` passes
      `|groups| trash::move_groups(groups, progress)`, and the tests pass
      `|groups| trash::run(groups, mover, progress)` or
      `trash::run_chunked(..)`. Whatever the shape, `delete` must stay
      drivable with fake movers on every platform.
    - With sidecar-only groups `run_chunked` puts the sidecars in the
      "RAW" slot of each chunk, so up to `CHUNK` (100) sidecars go in one
      `move_many`; the second `move_chunk` call of each chunk gets an empty
      list and makes no shell call.
    - What differs between the two paths by design, and so what the
      comparison test has to account for:
      - `move_chunk` gives `Ok(None)` (no `trashed_at`) for a file a
        successful chunk moved, while a fake mover may return `Some(to)`.
        Have the comparison's fake mover return `Ok(None)`, as the Windows
        `trash_one` does, or compare `path` and `owner` rather than whole
        `Trashed` values.
      - A file already gone before its chunk fails as `trash::GONE` without
        reaching the mover; the per-file path reports the mover's own
        error. Keep such a file out of the equivalence assertion.
      - `progress` is per group in `run` and per chunk in `run_chunked`.
        Compare the last call and the unit, not the call list.
    - Each of the two compared runs moves its files away, so each needs its
      own folder with the same file names (compare by file name), as the
      chunk tests in `trash.rs` build theirs with `chunk_dir`. `Failure`
      has no `PartialEq`; compare `path` and `message`. Write the fakes in
      `foldersidecars.rs`'s own tests.
    - A redo of a Delete Sidecars run already goes through `redo_chunked`
      on Windows (`move_again`), so only the first delete changes.

## Risks

- The change is to how files reach the real Recycle Bin on Windows, and the
  unit tests never touch it. `run_chunked` / `move_chunk` are unchanged and
  already carry Move to Trash, so the new exposure is only the sidecar-only
  group shape and decision 3's duplicate path.
- If `delete`'s signature changes, its two existing tests change with it;
  keep their assertions as they are so they still pin the per-file outcome.

## Progress

- (2026-11-26) Step 1 complete
