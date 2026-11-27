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

# Record the real-device check of the `$I` cleanup after an undo of Move to Trash

## Purpose

PR #1348 (`docs/plans/_archived/20261128-undo-recycle-index-cleanup/plan.md`)
made the engine delete the Recycle Bin's `$I` index file of every file an undo
of Move to Trash restores on Windows. Its unit tests cover only the `$R` to
`$I` name mapping, so a real-device check was left pending in `todo.md`. That
check was run on 2026-11-28 and passed. This plan records the result in the
agent guide next to the rule it confirms and closes the todo item, so no
later session repeats the check or treats the delete as unverified.

The check, as run (2026-11-28, Windows 11):

- Build: main at `c67a3066` (PR #1348 merged), a debug build with
  `--features screenshot`, `cargo build -j 2`. The `trash` crate resolves to
  5.3.0 in `crates/app/Cargo.lock` (the root `Cargo.lock` still holds 5.2.9).
- Run: private `--index`, `--settings` and `--log` under a scratchpad folder,
  `--mcp-port 47842`, `{"mcpEnabled": true, "mcpAllowDestructive": true}`
  given through `--import-settings`.
- Fixture: copies of two ARWs and one `.xmp` (`_DSC0745.ARW` with its
  `.xmp`, `_DSC0786.ARW`, from
  `D:\Photos\tests\20261127-c5\step7\Photos`) in a folder on `C:`.
- Driven through the MCP companion: `select_all`, `trash_photos` (the
  preview, then its id with `confirm: true`), `undo`.
- `C:\$Recycle.Bin\<SID>\` counts: 17 `$I` / 0 `$R` before; 20 `$I` / 3 `$R`
  after Move to Trash (`$RH7FLX6.ARW`, `$RNW9FPT.ARW`, `$RWL4B3Y.xmp`);
  17 `$I` / 0 `$R` after the undo, none of the three new `$I` files left and
  all three files back in the folder. The 17 older orphans were untouched,
  as designed.

## Steps

- [x] Step 1: Record the check in `docs/agents/shared-crates.md` and delete its `todo.md` item
  - Done when:
    - `todo.md` no longer holds the item
      ``### Engine: real-device check that an undo of Move to Trash leaves no `$I` file on Windows (undo-recycle-index-cleanup)``
      under `## Cross-cutting / other`: its heading, its `#### Background`
      and its `#### TODO` are all gone, and the neighbouring items are
      unchanged and still separated by one blank line.
    - `docs/agents/shared-crates.md` holds a "Seen" entry with the result
      above, directly after the `$I` cleanup paragraph of the section
      `### Trash restore: plan and perform in one pass, and re-verify identity before renaming back (Hit)`.
    - The PR changes only `todo.md`, `docs/agents/shared-crates.md` and this
      plan folder.
    - The PR's GitHub checks pass (lychee and the Markdown lint among them).
      No local build or `mise run ci:local` run is needed for this
      docs-only change.
  - Implementation approach:
    - `todo.md`: delete the item whole with one `Edit`, found by its heading
      text; touch nothing else in the file. Other sessions edit `todo.md`
      concurrently: rebase onto `origin/main` right before the push, and on a
      conflict keep main's side and re-apply only this deletion.
    - `docs/agents/shared-crates.md`: the target is the paragraph starting
      "`restore_all` moves each item's `$R` data file back", which ends with
      "Source: `docs/plans/_archived/20261128-undo-recycle-index-cleanup/plan.md`."
      and is followed by the section's closing `- Source: ...` bullet. Add a
      short prose paragraph opening with "Seen (2026-11-28, Windows 11, ...)"
      between the two, leaving both as they are, at the file's 80-column
      wrapping.
    - Content of the entry: the date and OS, the build (main at `c67a3066`,
      debug, `--features screenshot`, `trash` 5.3.0 in the app's lock), the
      fixture (two ARWs and one `.xmp` on `C:`), that it was driven through
      the MCP companion (`select_all`, `trash_photos`, `undo`), the three
      counts (17 / 0, 20 / 3, 17 / 0 as `$I` / `$R`), that all three files
      came back, that the 17 older orphans stayed, and that it exercised the
      chunked restore only (not `restore_item` / `restore_to`, a failed
      chunk or another volume). Cite this plan as its source at the archived
      path `docs/plans/_archived/20261128-undo-recycle-index-check/plan.md`
      as a backticked path.
    - Write the Recycle Bin folder as `$Recycle.Bin\<SID>\`; do not put the
      real SID in the repository. Do not rewrite the existing paragraph's
      5.2.9 statements.
    - Commit: `docs(agents): record the real-device check of the $I cleanup after an undo`.

## Trade-offs and risks

- The check covered one chunked restore of three files on `C:`. It did not
  cover a file of a renamed folder (`restore_to`, the one-at-a-time
  `restore_item` path), a failed chunk's retry, or another volume's Recycle
  Bin. The entry says what was run rather than claim those paths; no further
  todo item is opened (the deleted item asked only for this check).
- The root `Cargo.lock` still resolves `trash` to 5.2.9 while
  `crates/app/Cargo.lock` resolves 5.3.0. The check covered only the app's
  build; aligning the locks is a separate change.
- `todo.md` is edited concurrently by other sessions; mitigated by the
  rebase-before-push rule in the step.

## Progress
