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

# Settle whether Delete Permanently removes a read-only file, per platform

## Purpose

Close the `todo.md` section "Engine: Delete Permanently cannot remove a
read-only file on some platforms".

The section came from trash-over-scan's learnings
(`docs/plans/_archived/20261106-trash-over-scan/learnings.md`): a read-only
file (a camera's protect flag) may fail `remove_file` depending on the
platform, and only Windows 11 was tried by hand (2026-11-21, an `attrib +r`
ARW was removed). macOS and Linux were never tried.

This work replaces the manual check with tests that CI runs on macOS, Linux
and Windows, and records the answer in the docs. The behavior does not
change:

- A file that is read-only by its permissions or by the Windows read-only
  attribute is removed.
- A file locked on macOS (the `uchg` flag) is not removed. It is reported as
  a failure and stays, and Handpic does not clear the flag. Finder also asks
  before it deletes a locked item, so leaving it is the safe side.

Background that explains the split, for the docs wording:

- macOS's msdosfs shows the FAT read-only attribute, which is what a
  camera's protect sets, as `uchg`, and `unlink` of a `uchg` file fails with
  EPERM.
- A file that is read-only only by its permissions can be unlinked whenever
  its directory is writable (Linux vfat, APFS).

## Steps

- [x] Step 1: Pin the read-only and locked-file behavior of `remove_one`
      with tests, record it in the docs and remove the todo section
      (single-PR mode)
  - Done when:
    - `crates/engine/src/trash.rs` has two new tests, the last two items of
      `mod tests`:
      - On every OS: a file made read-only with
        `set_permissions` / `set_readonly(true)` is removed by `remove_one`.
        The call returns `Ok` and the file no longer exists.
      - On macOS only (`#[cfg(target_os = "macos")]`): a file given
        `chflags uchg` makes `remove_one` return `Err`, and the file still
        exists. The test then runs `chflags nouchg` so the temp dir can be
        removed.
    - No line of `trash.rs` outside those two appended tests changes. That
      means no implementation change, no edit to the `use super::{..}` list
      and nothing in the Undo / restore tests.
    - The result is recorded in:
      - `docs/humans/usage.md`, the `Delete Permanently…` bullet, and
        `docs/humans/usage.ja.md`, the same bullet, kept in sync.
      - `docs/agents/shared-crates.md`, next to the other trash rules.
      - The PR body, which names the three CI jobs that ran the tests.
    - The `todo.md` section "### Engine: Delete Permanently cannot remove a
      read-only file on some platforms" (heading, Background, TODO) is gone
      and no other `todo.md` item changed. If the all-OS test fails on one
      OS, the section is narrowed instead; see "Trade-offs and risks".
    - Local checks: the user's standing instruction is to skip local cargo
      builds and let the PR's GitHub Actions verify, so only the Markdown
      side (`mise run lint`, lychee) is run locally.
    - The PR's GitHub checks pass. The `test` job's `Build and test`
      (`mise run test`: clippy with `-D warnings`, then `cargo test`) must be
      green on `ubuntu-latest`, `macos-latest` and `windows-latest`; that is
      the verification of both tests.
  - Implementation approach:
    - Placement: append both tests after
      `a_chunked_restore_reports_progress_per_chunk`, the current last test,
      just before the module's closing brace. Another session is editing the
      Undo part of this file, so appending at the end and touching nothing
      else keeps the two changes from conflicting. Rebase before the PR and
      re-check that the tests are still the module's last items.
    - Existing pieces to use:
      - `remove_one` is `std::fs::remove_file` mapped to
        `Result<Option<PathBuf>, String>`, and it is the remover
        `delete_for_good` is given.
      - It is already in the tests module's `use super::{..}` list.
      - Use the module's `temp_dir(name)` and `write(path)` helpers.
      - Spell `std::process::Command` and `std::fs` by full path, so no
        `use` line changes.
    - Naming: follow the module's sentence style, for example
      `a_delete_removes_a_read_only_file` and
      `a_delete_leaves_a_locked_file_in_place`.
    - Test (a): read the file's `metadata().permissions()`, call
      `set_readonly(true)`, then `std::fs::set_permissions`. On Unix this
      clears the write bits; on Windows it sets the read-only attribute, the
      same thing as `attrib +r`.
    - Do not call `set_readonly(false)` anywhere, for cleanup either.
      `clippy::permissions_set_readonly_false` warns on it on Unix and the
      CI's clippy runs with `-D warnings`. Test (a) needs no cleanup of the
      flag, since the file is gone when it passes.
    - Test (b): run `chflags uchg <file>` through `std::process::Command`
      and assert the command succeeded, so a failure to set the flag is not
      mistaken for a pass. Take `remove_one`'s result and the file's
      existence into locals, run `chflags nouchg <file>`, and only then
      assert. A failed assertion must not leave a locked file in the temp
      dir, where the next run's `temp_dir` could not remove it. Assert only
      `is_err()` and that the file exists, not the OS error text.
    - Docs wording: say what the tests pin.
      - A read-only file is deleted like any other.
      - On macOS a locked file (Finder's `Locked` box, the `uchg` flag) is
        listed as an error and stays until it is unlocked; Handpic does not
        unlock it.
      - The link to a camera's protect flag on a FAT card is background, not
        something CI verifies. Give it as the reason, not as a tested claim.
      - Say nothing about `Move to Trash…` and locked files: it was not
        tested.
    - `docs/agents/shared-crates.md`: add one bullet to the trash rules
      (around the bullets on `trash::groups_of` and `trash::run_listed`). It
      names `remove_one`, the two tests, and the decision not to clear the
      flag and why.
    - `todo.md`: delete the whole section and nothing else. The
      neighbouring "Undo of Move to Trash leaves an orphaned `$I` file"
      section stays.
    - `CLAUDE.md` needs no change: no module, function or rule it describes
      changes.

## Trade-offs and risks

- **If the all-OS test (a) fails on one OS.** The decision for this plan is
  no implementation change, so do not make `remove_one` clear the flag in
  this PR. Instead:
  - Gate test (a) off that OS with `#[cfg(not(..))]`.
  - Add a test for that OS that pins the failure (the file stays, `Err`).
  - Document that platform's behavior.
  - Narrow the `todo.md` section to the one remaining decision, "clear the
    read-only flag first or leave it, on <OS>", instead of deleting it.

  The alternative, clearing the flag before the remove, would let the user
  delete a protected file without a second confirmation. It is left to a
  separate decision.
- **Test (b) pins `uchg`, not a real FAT card.** CI's temp dir is APFS. The
  test shows that `unlink` of a `uchg` file fails and the file is kept. That
  msdosfs maps a camera's protect to `uchg` is background knowledge, not
  verified here.
- **Test (a) covers permissions and the Windows attribute only.** On Linux
  it shows a mode without write bits in a writable directory is removable,
  which is what a vfat read-only file looks like there. No vfat mount is
  exercised.
- **`remove_one` directly, not `delete_for_good`.** The tests call the
  remover alone, as decided. Existing tests already cover the `Failure` path
  with a failing remover.
- **A leftover locked file after a crash of test (b).** If the test process
  dies between `chflags uchg` and `chflags nouchg`, the temp dir keeps a
  locked file. This matters only on a developer's Mac (CI runners are
  fresh); `chflags nouchg` on the file clears it.
- **Windows relies on the standard library.** The Windows result depends on
  std's `remove_file` removing a file with the read-only attribute, which
  matches the 2026-11-21 observation. Test (a) on `windows-latest` is what
  holds this in place across toolchain bumps.

## Progress

