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

# Deferred open of a folder a date split removed

## Purpose

While a `Rename by Capture Date…` run is in flight, a click on a folder in the tree is not opened but kept in `deferred_open` (`open` in `crates/app/src/main.rs`, under `gate.in_flight()`), and the run's end opens it. Unlike the inline rename, a date run does not register in `folderops.renames`, so `refuse_renaming` does not refuse a click on the folder being split or on a folder under it. When the split then removes the folder (`Split::removed`), `date_folder_ran`'s split branch still goes to `settle_folder_run`, which opens the deferred folder: a folder that no longer exists.

After this work, a deferred open naming the removed split folder (or a folder under it) lands where an open of it would after the split, the first date folder, as the branch for the open folder already does; with no date folder it is dropped. The `todo.md` item that recorded this is removed. It was noticed and left out by the reveal-collapsed-rename work ([its plan](../_archived/20261127-reveal-collapsed-rename/plan.md)).

## Steps

- [x] Step 1: Redirect a deferred open of a folder the split removed to the first date folder
  - Done when:
    - In `date_folder_ran`'s split branch, when `split.removed` and `deferred_open` names `dir` or a folder under it, the first date folder (`split.moved.first()`) opens through `open_with_note` with the split's status, and the missing folder is never opened; with no first date folder the deferred open is dropped and the flow falls to `settle_folder_run`'s resync.
    - A deferred open of any other folder, and every case without a deferred open, behaves as before.
    - The doc comment on `date_folder_ran` says what happens to a deferred open of the removed folder.
    - A new UI test in `crates/app/src/folderops.rs`'s test module holds the gate, clicks the folder so its open is deferred, ends a split that removed it, and asserts the first date folder is the open one, `deferred_open` is empty and exactly one open ran.
    - The `### App: a `Rename by Capture Date…` split that removes the folder leaves a deferred open naming a missing folder` section is gone from `todo.md` (heading, `#### Background`, `#### TODO`, and one of the blank lines around it).
    - `CLAUDE.md` is unchanged unless the implementation adds something its layout text names (none expected).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass (the new test and the existing `folderops` tests among them).
  - Implementation approach (as far as it is known; omit if unknown):
    - Only the split branch of `date_folder_ran` changes (the `if self.deferred_open.is_some() || !opens { … } else if … ` chain). `settle_folder_run`, `open` and `folder_renamed` stay as they are.
    - Decide "the deferred open is gone with the folder" once, before the chain: `split.removed` and `tree::relation(&deferred.to_string_lossy(), &dir, IGNORE_CASE).is_some()` (`Same` or `Under`; `dir` is the tree's spelling the click also used). Take the deferred open out in that case (`Option::take_if` is already used in `main.rs`), then let the existing chain run: with it taken and the flag set, skip the first `settle_folder_run` arm so the flow reaches the existing `else if let Some(first) = first { self.open_with_note(…) } else { settle }` arms. The `!split.removed` arm cannot be reached with the flag set. Keep it to a few lines; no new helper unless it reads better.
    - `Under` needs no separate handling: `remove_if_empty` in `crates/engine/src/datefolder.rs` removes the folder only when it is empty, so a folder under a removed one is gone too.
    - `reveal_collapsed` needs no cleanup: when the deferring click collapsed the row, the first date folder's `reveal` takes it and ignores it because it names another path.
    - A click during a date run is deferred, not refused: `run_date_folder` never calls `folderops.renames.start`, so the test `a_click_on_a_folder_under_a_rename_in_flight_is_refused` does not apply here. Do not add a refusal in this step.
    - Test, next to `a_held_collapse_click_follows_a_date_folder_rename` and after its pattern: `Ui::open()`, `expanded_folder(&mut ui)`, take `inner` with `drawn_path(&ui, "inner")` as the folder to split; in one `ui.update`, `view.gate.enter_in_flight()`, `view.tree_row_clicked(inner.clone(), cx)`, assert `deferred_open.is_some()`; record `view.opens()`. On disk, take `canonicalize(&inner)` first, then remove `inner` and create the date folder (e.g. `2026-11-27`) beside it. Then call `view.date_folder_ran(inner.clone(), Ok(DateRan::Split(Split { moved: vec![Moved { name, path, count }], failed: Vec::new(), left: Vec::new(), removed: true, warning: None }, canonical)), cx)`; it calls `gate.leave_in_flight()` itself, so the test must not. `Moved` has to be imported from `handpic_engine::datefolder` in the test module. Wait with `ui.wait_until` (or `wait_revealed` if the tree cursor lands on the date folder's row; verify) until `view.dir` ends with the date folder's name and `view.status_of().idle()`, then assert `deferred_open == None` and `opens()` is the recorded value plus one. Check that the test fails without the fix (the open of the missing `inner` runs instead).
    - Worth adding if it stays small: a second test where the deferred click is on another folder (e.g. split `inner` removed while the deferred open names a sibling created for the test), asserting that folder opens, to pin the unchanged path.
    - The implementer may run the targeted test locally (`cargo test` in `crates/app` filtered to the new test's name, through the toolchain `mise` pins for `app:check`); the rest is left to `ci:local` and the PR's checks.

## Trade-offs and risks

- Redirect, not refuse. The inline rename refuses a click on the folder in flight (`refuse_renaming`); a date run does not, and this plan keeps it so and fixes where the deferred open lands. Refusing instead would mean registering the date run's folder in `folderops.renames` (or a parallel check) for both the date rename and the split, which changes what a click during a date rename does today (`a_held_collapse_click_follows_a_date_folder_rename` pins that it follows the rename). Not taken.
- Spelling of the comparison. The deferred open is compared against `dir`, the tree's spelling, which is what a tree click stores. A deferred open from a drop, the folder picker or an MCP `open_folder` may spell the same folder differently (canonical, verbatim prefix) and would then not match, keeping today's behavior (an open of a missing folder). Comparing against the split's `canonical` path as well would cover more of those; it is not in the requirement and is left for the caller to decide.
- No first date folder. When the folder was removed but nothing moved, the deferred open is dropped and `settle_folder_run` re-lists the open folder if the watcher saw a change. The user's click is lost silently; the status still shows the split's text.
- The test constructs the `DateRan::Split` by hand after changing the disk, as the date rename's test calls `folder_renamed` directly. It does not exercise `split_folder` itself (covered by the engine's tests); running the real split would need RAW fixtures with capture dates and the dialog flow.

## Progress
- (2026-11-27) Step 1 complete. The local test run, `mise run ci:local` and the check that the new test fails without the fix were not done because the host was out of memory; they are left to the PR's GitHub checks (see `learnings.md`).
