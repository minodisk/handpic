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

# Remove the Recycle Bin's `$I` file after an undo of Move to Trash on Windows

## Purpose

On Windows, undoing a Move to Trash restores each file through trash 5.2.9's
`os_limited::restore_all`, which moves the Recycle Bin's `$R` data file back
and leaves its `$I` index file behind. Every restored file therefore leaves
one orphaned `$I...` file in `C:\$Recycle.Bin\<SID>\` (or the volume's
equivalent). The shell does not list them, so they pile up unseen and cost a
file record each.

Measured on 2026-11-27 (Windows 11, this host):

- Explorer's own restore (the shell verb `undelete`) leaves the `$I` file
  too: C: held 18 `$I` files and 0 `$R` files.
- With trash 5.2.9, `TrashItem.id` is the filesystem path of the `$R` file
  (e.g. `C:\$Recycle.Bin\S-1-5-21-...\$R61PDVO.txt`). After `restore_all`
  the `$R` file is gone, the original path holds the file, and
  `$I61PDVO.txt` remains in the same folder.
- The `$I` file can be deleted with a plain user-permission delete, and the
  item does not show in the Recycle Bin afterwards.

Once this is done, an undo in Handpic leaves no orphan behind: after each
restore the engine deletes the `$I` partner of every item that really came
back.

## Steps

- [x] Step 1: Delete each restored item's `$I` file after `restore_all` on Windows, and record the findings
  - Done when:
    - `crates/engine/src/trash.rs` has a pure function that maps a `$R`
      file's path to its `$I` partner (same folder, the leading `$R` of the
      file name replaced by `$I`), giving nothing for a name that does not
      start with `$R`, with unit tests.
    - After `restore_all` in both Windows restore paths (the chunk closure
      of the Windows `restore_listed`, and `restore_item`), each item whose
      `$R` file (`TrashItem.id`) no longer exists and whose original path
      (`original_parent` joined with `name`) now exists has its `$I` file
      deleted, errors ignored. Nothing is deleted on other platforms.
    - The whole `todo.md` section
      `### Engine: Undo of Move to Trash leaves an orphaned `$I` file per restored file on Windows`
      (heading, Background, TODO) is deleted.
    - The findings above (Explorer leaves the `$I` too; the `$I` is
      deletable with user permission; Handpic now removes it) are recorded
      in `docs/agents/shared-crates.md` next to the Windows chunked restore,
      and as a dated follow-up under the safety record of
      `docs/plans/_archived/20261106-trash-over-scan/trash-bench.md`.
    - `CLAUDE.md`'s `src/trash.rs` description mentions the cleanup in one
      clause.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/engine/src/trash.rs`, `todo.md`,
      `docs/agents/shared-crates.md`,
      `docs/plans/_archived/20261106-trash-over-scan/trash-bench.md`,
      `CLAUDE.md`.
    - The name mapping: a pure function over `&Path` giving
      `Option<PathBuf>`, built on `Path::file_name` / `with_file_name` so it
      compiles and is tested on every platform (build test paths with
      `Path::join`, not a literal backslash path, so they hold on Unix).
      Keep it compiled everywhere and mark it
      `#[cfg_attr(not(windows), allow(dead_code))]`, the convention
      `restore_chunked` already uses. Test at least: `$R61PDVO.txt` gives
      `$I61PDVO.txt` in the same folder; a name without the `$R` prefix
      gives none; only the leading `$R` is replaced (a later `$R` in the
      name stays); a name with no extension.
    - The cleanup itself: a `#[cfg(windows)]` helper taking the `$R` path
      and the original path, both captured before the call.
      `restore_all` consumes its items, so collect each item's `id` and
      `original_path()` first (`TrashItem` is `Clone`; only these two values
      are needed). It deletes with `std::fs::remove_file` and drops the
      result.
    - Chunk path (`restore_listed`, `#[cfg(windows)]`): run the cleanup
      over the chunk's items after `restore_all` returns, on success and on
      failure. `restore_chunk` counts a file that is at its destination
      after a failed `restore_many` as back and never sends it through
      `mover`, so a cleanup that ran only on success would leave those
      files' `$I` behind. The two per-item conditions keep this safe: a
      `RestoreCollision` returns before anything moves, so the `$R` still
      exists and nothing is deleted.
    - Per-item path (`restore_item`, compiled on Windows and Linux): run the
      cleanup after `restore_all` succeeds and before `relocate`. For a
      file with `restore_to`, `relocate` moves the file off the original
      path, so a check after it would find the original path empty and skip
      the delete. Gate the added lines with `#[cfg(windows)]` so Linux
      builds without unused bindings.
    - Leave `restore`, `restore_chunked`, `restore_chunk` and their
      signatures alone: they are generic over the item type and tested over
      a fake `Bin`; the cleanup belongs in the two closures / functions that
      know a `::trash::TrashItem`.
    - No automated test deletes a real `$I`: no test in the repository
      touches the real Recycle Bin, and none should be added. The pure
      mapping is what the unit tests cover.
    - Docs: in `docs/agents/shared-crates.md`, add to the paragraph on the
      Windows chunked restore (the one starting "On Windows the undo
      restores in chunks") what `restore_all` leaves, that Explorer's
      restore leaves it too, the two conditions and the `$R` to `$I` name
      rule, each marked as measured on 2026-11-27 with trash 5.2.9. In
      `trash-bench.md`, keep the existing record as written and add a dated
      follow-up bullet under the `restore_all` leaves the `$I` bullet.
    - Manual check on the host, not a blocker: count the `$I` files in the
      volume's `$Recycle.Bin\<SID>\` folder, then run the app with
      `mise run app:dev:screenshot` (see `docs/agents/app.md`; precedent
      PR #1307) with a private `--index`, `--settings` and `--log` under a
      scratchpad folder and `--mcp-port 47842`, on fixtures copied to a
      temporary folder; move files to the Trash, undo, and count again. The
      count should be unchanged and the files back. Record the result in
      `learnings.md` and the PR description. Run no load test.

## Trade-offs and risks

- **Cleanup after a failed chunk call (taken) or only after a successful
  one.** The plan runs the chunk path's cleanup after a failure as well,
  because a partly applied `restore_all` puts files back that
  `restore_chunk` then counts as restored without another call. The two
  conditions (the `$R` gone, the original path present) are checked per
  item either way.
- **A `$I` deleted for a file the user restored by hand in the same
  instant.** The cleanup only runs for items just handed to `restore_all`,
  and a `$I` with no `$R` is an orphan whoever restored the file, so
  deleting it is correct in both cases.
- **The `$I` delete is not covered by an automated test.** It needs the real
  Recycle Bin; the manual check above covers it instead.
- **Older orphans stay.** The `$I` files earlier undos and Explorer left are
  not swept. Sweeping would mean deleting Recycle Bin files Handpic did not
  create, which is outside this fix.
- **Editing an archived plan's record.** `trash-bench.md` lives under
  `_archived/`. The plan appends a dated follow-up and leaves the original
  measurement text unchanged.
- **The trash crate's contract.** That `TrashItem.id` is the `$R` file's
  filesystem path is an implementation detail of trash 5.2.9's Windows
  backend. If a later version changes it, the mapping gives none for a name
  without `$R` and the existence check fails for a non-path id, so the fix
  degrades to today's behaviour and deletes nothing wrong.

## Progress

- 2026-11-28: Step 1 done. `mise run ci:local` was stopped by host memory
  pressure and not re-run; the PR's GitHub checks verify it. The manual `$I`
  file count check on Windows is still pending, as `learnings.md` records.
