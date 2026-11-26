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

# A held open-and-collapse click across a folder rename

## Purpose

`todo.md` holds the section "App: a held open-and-collapse click leaves the
row expanded when a rename moves the folder": a click on an expanded row that
is not open collapses the row and names it in `Handpic::reveal_collapsed`;
when the gate defers the open (`deferred_open`), a rename that moves the
folder before the deferred reveal would make the stored path miss in
`reveal`, and the row would end up expanded. Nobody confirmed whether that
can happen, and no test covers it.

Reading the code gives two answers, which this work pins down with UI tests:

- The tree's `Rename…` cannot do it. A rename in flight refuses the click
  (`refuse_renaming`, `RenamesInFlight::blocks`, which covers the folder,
  everything under it and its ancestors). A rename requested while a run is in
  flight is held in the gate, the gate drains only at a scan's end
  (`finished`), and every run end opens `deferred_open` first, whose `open`
  discards the held rename (`held_dropped`, `RENAME_CANCELED`).
- `Rename by Capture Date…` apparently can. `run_date_folder` enters the
  gate's in-flight count without registering in `RenamesInFlight`, so the
  click is not refused, and its `DateKind::Rename` result ends in
  `folder_renamed`, which rebases `deferred_open` but not `reveal_collapsed`.
  Under the app's dialog the backdrop occludes the tree, but a run an MCP
  client confirmed (`run_confirmed`, `mcpAllowDestructive` on) has no dialog,
  so the tree is clickable during it.

Once done, both rules are tested, the date-rename route keeps the row
collapsed (if the test confirms it is broken), and the todo section is gone.

## Steps

- [x] Step 1: Pin the held open-and-collapse click across a rename with UI tests, carry `reveal_collapsed` through `folder_renamed` if the date-rename test fails, and delete the todo section
  - Done when:
    - A UI test covers (a): with the gate in flight, a rename of folder X is
      requested (held, `gate.waiting() == Some(rename::LABEL)`), then
      `tree_row_clicked` on X's expanded, not-open row (X's row collapsed at
      the click, `reveal_collapsed` and `deferred_open` set); when the run
      ends, X is the open folder, its row is still collapsed, the gate holds
      nothing, X keeps its name on disk and the new name does not exist.
    - A UI test covers (b): with a rename of X in flight, `tree_row_clicked`
      on X and on a row under X sets `tree_note` to `rename::RENAMING`,
      leaves X's row expanded, and sets neither `reveal_collapsed` nor
      `deferred_open`.
    - A UI test covers (c): with a run in flight that ends in
      `folder_renamed` for X (the `Rename by Capture Date…` rename),
      `tree_row_clicked` on X's expanded, not-open row before the run ends;
      after it, the renamed folder is open and its row is collapsed.
    - Test (c) was run once before any fix and its result (fails: the case is
      real; passes: it is not) is recorded in `learnings.md` and the PR body.
      If it failed, `folder_renamed` carries `reveal_collapsed` and (c)
      passes; if it passed, no production code changes.
    - `docs/agents/app.md`'s `reveal_collapsed` rule says what a folder
      rename does to the field (only if the fix lands; otherwise it says why
      a `Rename…` cannot move the folder under a held click, in one sentence).
    - The `todo.md` section "App: a held open-and-collapse click leaves the
      row expanded when a rename moves the folder" is deleted, and the PR body
      carries the reasoning (why `Rename…` is safe, what the date rename
      did).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/folderops.rs` (the tests in its `tests` module,
      which already uses `crate::uitest::Ui` and can call this file's private
      methods such as `settle_folder_run`; the fix in `folder_renamed`),
      `docs/agents/app.md`, `todo.md`. Do not edit `crates/app/src/main.rs`:
      another session is changing it around the scan hot list and the
      thumbnails, and nothing here needs it (`folderops.rs` already reads and
      writes `deferred_open`, so `reveal_collapsed` is reachable from it the
      same way).
    - The fix, if (c) fails: in `folder_renamed`, next to the `tree_open` /
      `tree_cursor` lines, rebase the field with the same `moved` closure
      (`tree::rebase`), keeping it unchanged when the folder is outside the
      renamed one. One line; no new helper.
    - Getting X drawn as an expanded, not-open row: the `Ui`'s temporary
      folder is not reliably reached from the real roots (the existing test
      `the_settings_key_opens_the_settings_from_the_tree_but_not_from_a_rename`
      pushes a row by hand, which the next `tree_changed` would discard, so
      that trick does not survive a collapse). Proposed: wait for
      `roots_loaded`, add the temporary folder as a root with
      `tree.add_roots` (`tree::ancestors_within` takes the deepest root, so
      the reveal walks from it), `tree_toggle` it to list it, then
      `tree_toggle` X, which needs a subfolder to be expandable, and wait
      until X's row is `expanded`. Take X's path from its drawn row
      (`tree_rows`, found by name), not from a string built in the test, so
      the spelling is the tree's. Verify this holds on Windows, macOS and
      Linux through the PR's checks (macOS temp paths go through a symlink).
    - X need not hold photos, and no other folder needs to be open: a row is
      "not open" when nothing is open. If an empty folder's open makes the
      waits awkward, open `photos` through `Ui::open_folder` first and keep X
      beside it.
    - Holding a run in flight for (a): call `view.gate.enter_in_flight()` in
      the test, and end the run as a folder run ends, `gate.leave_in_flight()`
      then `settle_folder_run`. This is the same state every run end reaches
      (the trash run's, the undo / redo's and the clear's ends take
      `deferred_open` and call `open` the same way), without needing a real
      slow run.
    - For (b): `request_rename` on an idle view starts the rename
      synchronously (`renames.start`, in flight), so make the clicks inside
      the same `Ui::update` closure, as
      `a_folder_rename_in_flight_starts_no_prefetch` does with its asserts;
      `Ui::update` runs the executor afterwards. Then wait for the rename's
      end so the drop does not race it. The click on a row under X goes
      through `RowClick::Open` and `open_from_tree`, which refuses the same
      way.
    - For (c), decide between two ways while implementing: end to end,
      through `run_confirmed(FolderOp::Date(x), None, cx)` with files that
      plan as `DateKind::Rename` (check whether `uitest.rs`'s generated DNG
      gives `datefolder::plan_of` one capture date; it carries no date tag
      today, and `dng` is private to `uitest.rs`), clicking inside the same
      `Ui::update` closure that starts the run; or, if that needs a new
      fixture, hold the gate by hand as in (a), click, rename X on disk with
      `fs::rename`, `leave_in_flight` and call `folder_renamed` with the old
      and new paths, which is what `date_folder_ran` does on
      `DateRan::Renamed`. Prefer the end-to-end one only if it costs no new
      fixture.
    - Assert the row state on `tree_rows` (`expanded`) after waiting for the
      open (`view.dir`) and for the reveal's last update (`tree_open_row` is
      the renamed row), since `reveal` walks its chain on a spawned task.
    - Follow `docs/agents/app.md`'s UI test notes (mouse paths called at
      their handler through `Ui::update`, bounded waits through
      `Ui::wait_until`).

## Trade-offs and risks

- Which rule fixes the date-rename route (decide if test (c) fails):
  - Carry `reveal_collapsed` in `folder_renamed` with `tree::rebase`
    (planned). It matches the `deferred_open` rebase on the next lines, which
    already lets a plain deferred open follow the rename, and keeps the
    user's collapse.
  - Register the date-folder run in `RenamesInFlight`, so a click on the
    folder is refused with `RENAMING` during the run. Consistent with the
    tree's `Rename…`, but it also refuses the plain deferred open that works
    today, a behaviour change beyond this todo.
  - Clear the field in `folder_renamed`. Smallest, but the row ends up
    expanded, which is the reported defect made deliberate.
- The real-world window is narrow: an MCP client's confirmed
  `rename_by_capture_date` in flight and a user's click on that folder's
  expanded, not-open row. The plan fixes it because the fix is one line
  beside the existing rebase.
- Test (c) through a direct `folder_renamed` call is less faithful than a
  real date-folder run; it pins the carry rule, not `run_date_folder`'s
  missing refusal. The end-to-end form depends on a fixture that may not
  exist.
- The tree fixture (the temporary folder added as a root) is new to the UI
  tests and unverified on macOS and Linux; a path spelling difference would
  show as a timeout in `wait_until`. Taking paths from the drawn rows limits
  that.
- Noticed and left out: a date-folder split that removes the folder leaves a
  `deferred_open` naming a folder that no longer exists, which
  `settle_folder_run` then opens. It is a different defect; record it as a
  todo candidate in `learnings.md`, not a change in this PR.

## Progress
