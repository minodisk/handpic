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

# Todo: seven small fixes (docs, agents, app)

## Purpose

Seven `todo.md` entries are small, self-contained and still open on main
(checked at 0630f927). Three are stale text (a heading in
`docs/agents/app.md`, a sentence in two agent definitions, two links in the
gpui-port plan). Four are small code fixes: the progress circle ignores
Sequence / Import runs, a held Clear Cache is not drained at two idle points,
the settings writer does not close its store on drop (a test retries around
it), and `merge.test.sh` writes to the real repository when `GIT_DIR` /
`GIT_WORK_TREE` are exported. Closing them removes seven entries from
`todo.md` and one flaky-test workaround.

## Decisions

- One step, one PR.
- (1) The heading's second half becomes a generic phrase (for example "the
  app's controls draw from it"), not an enumeration of components, so it does
  not go stale when another component is adopted.
- (3) Only the two links the todo names are retargeted; the other Progress
  links that may be stale are left as they are.
- (5) The two named idle points are fixed and the other in-flight exits
  audited; draining from `observe_self` is not taken.
- (6) The last `Writes` clone joins the writer thread on drop (accepted: the
  join is unbounded).
- (7) Only `GIT_DIR` and `GIT_WORK_TREE` are unset.

## Steps

- [x] Step 1: Fix the seven items and remove their `todo.md` entries
  - Done when:
    - (1) The heading at `docs/agents/app.md:71` no longer claims that only
      the settings modal and the tree's inline rename draw from
      gpui-component, and `rg -n 'gpui-component is initialized' docs/agents CLAUDE.md`
      shows no reference that quotes a title the heading no longer has.
    - (2) `.claude/agents/pr-runner.md` and `.claude/agents/merger.md` say
      that `blocked` can also be the last ACTION of an exit 2 (a timeout
      inside `BLOCKED_GRACE`) and is handled as ordinary waiting; the call
      lines, tables and handling are unchanged.
    - (3) The two Progress entries in `docs/plans/20261109-gpui-port/plan.md`
      (2026-11-14 sub-milestone 1, 2026-11-16 re-decision) point at
      `../_archived/20261114-gpui-ci-packaging/plan.md` and
      `../_archived/20261116-gpui-packaging-redecision/plan.md`.
    - (4) `progress_circle` in `crates/app/src/main.rs` passes the running
      Sequence / Import run's `(done, total)` as `progress::shown`'s
      `folder_op` argument, and the value is gone once the run ends or is
      canceled.
    - (5) A held Clear Cache asks by itself after a trash undo into another
      folder (`restored`) and after a failed rename (`settle_folder_run`),
      with a test of the pure part where one is possible.
    - (6) `Writes` in `crates/app/src/settings.rs` closes its store when its
      last clone is dropped, and
      `a_second_load_does_not_reread_a_changed_seed` has no retry loop and
      passes repeatedly on Windows.
    - (7) `.claude/skills/merge-settings/scripts/merge.test.sh` unsets
      `GIT_DIR` and `GIT_WORK_TREE`, and a test shows that a run with both
      exported leaves the repository they name untouched.
    - The seven entries are removed from `todo.md`, and no other entry is
      changed:
      - `### Native app docs: correct the stale "only the settings modal and the tree's inline rename draw from gpui-component" text`
      - `### Agents: \`pr-runner.md\` and \`merger.md\` say only \`wait\` / \`review_required\` are seen on an exit 2`
      - `### Docs: the gpui-port plan's Progress entries link plan folders that have since been archived`
      - `### Native: Sequence JPEG Timestamps and Import do not feed the progress bar`
      - `### Native: a held Clear Cache misses idle points that do not pass through \`finished\``
      - `### Native: make the settings writer close its store on drop (flaky seed reload test)`
      - `### Agents: \`GIT_DIR\` / \`GIT_WORK_TREE\` exported around \`mise run ci\` leak into \`merge.test.sh\` and write to the real repository`
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - General: the app crate is `crates/app` and the backend is
      `crates/engine`. Touch only what each item needs. Archived plans and
      `docs/plans/review-history/**` that quote old text are history; leave
      them. Do not commit the untracked `crates/native/` folder (stale build
      output).
    - (1) Heading, `docs/agents/app.md:71`: today it reads
      `### gpui-component is initialized; the settings modal and the tree's inline rename draw from it (Inferred)`.
      Keep the `gpui-component is initialized` prefix and the `(Inferred)`
      tag and reword only the second half generically. The only live
      reference is `docs/agents/app.md:3358`, which quotes just the prefix, so
      it stays correct; re-run the `rg` to confirm. The section body needs no
      change.
    - (2) Sentences to reword: `.claude/agents/pr-runner.md` around line 178
      and lines 338-340; `.claude/agents/merger.md` lines 84-85. `blocked`
      stays in both tables as an exit 0 ACTION: `wait-pr-actionable.sh`
      returns it with exit 0 only after it has persisted for `BLOCKED_GRACE`
      (60 s), and a timeout inside the grace exits 2 with `ACTION=blocked` as
      the last output. The existing "increment the timeout counter and
      re-run" branch already covers it.
    - (3) Lines 657 and 666 of `docs/plans/20261109-gpui-port/plan.md`. Both
      folders exist under `docs/plans/_archived/` with exactly those names;
      verify by listing them.
    - (4) `FolderOps` already keeps `sequence_progress()` and
      `import_progress()` (`crates/app/src/folderops.rs`), set at the run's
      start, updated by `folder_op_message`, and set to `None` in
      `sequence_ran` / `import_ran`. Replace the `None` in `progress_circle`
      with `self.folderops.sequence_progress().or(self.folderops.import_progress())`
      (the same order `status_of` uses). Confirm no re-listing stops a
      Sequence or Import run without passing through `sequence_ran` /
      `import_ran`; if one does, clear the value there. Update
      `docs/agents/app.md` around line 3178 ("`None` today").
    - (5) `drain_clear` (`crates/app/src/main.rs`) already returns early
      unless a clear is waiting, `clear_busy()` is false and no dialog is up.
      Call it at the end of `restored` and at the end of `settle_folder_run`
      (`crates/app/src/folderops.rs`). Audit the other
      `gate.leave_in_flight()` sites for a branch that ends idle without a
      relist or `settle_folder_run`, and add the call there if one exists.
      Test the guard as a pure function in `cache.rs`, or with a UI test in
      `src/uitest.rs` if the idle point is reachable cheaply. Update
      `docs/agents/app.md` around line 1886 ("drains it in `finished`").
    - (6) Share the sender and the thread's `JoinHandle` behind one `Arc`,
      whose `Drop` drops the sender first and then joins the thread. `Writes`
      stays `Clone`, and `set` / `delete` / `drain` / `is_open` keep their
      behavior. Replace the retry loop in
      `a_second_load_does_not_reread_a_changed_seed` with one `load_from` and
      the assert. Run the settings tests many times in a row on Windows to
      confirm. Update the `Writes` paragraph in `docs/agents/app.md` (around
      line 865).
    - (7) Unset both variables near the top of `merge.test.sh`, before any
      `git` call, with a comment saying why. Test by re-invoking the script
      under a guard variable with `GIT_DIR` / `GIT_WORK_TREE` pointing at a
      decoy repository in the script's temp dir; assert exit 0 and that the
      decoy's HEAD, refs and worktree list are unchanged. Confirm the case
      fails with the `unset` line temporarily removed. Keep it
      `shellcheck -x` clean.

## Progress
