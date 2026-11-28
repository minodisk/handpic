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

# Task indicators in the status row

## Purpose

The right-hand meta pane ends in a stack of status-like text: the view-only
note, the last step's note (`step_note`), the running trash run's progress,
a folder operation's note, "waiting for the scan to finish", the import's
note, the update's restart notice (`pane_notes()`), and the failure lines
(`pane_errors()`). That text is not metadata of the shown photo, it pushes
the metadata rows around, and the status row's single progress circle
(`progress::shown`) shows only one of several things that can run at once.

After this work the meta pane holds only the photo's metadata. Running work
is shown at the right end of the status row as one icon and progress circle
per task, with a tooltip each and one popover listing every task with its
progress, Cancel and failures. One-off results become toasts, and the
view-only state becomes a badge in the strip header. The MCP `get_status`
answer keeps its `notes` / `errors` content (except where a dismissed
failure is cleared, see Decisions).

No emoji and no glyph characters are used for the new UI: every mark (task
kinds, the failure indicator, the popover's dismiss and cancel) is a Lucide
icon served through `crates/app/src/icons.rs`, the failure indicator drawn
in the theme's danger color.

## Decisions

Taken from the planner's open questions, each with the recommended option:

- **A held operation gets no Cancel.** An operation the `IdleGate` holds is
  shown as "waiting" in the popover without a Cancel button, matching what
  `cancel_operation` does today; no new cancel path is built.
- **Dismissing a failure clears its state.** The popover's dismiss clears
  the underlying failure, so `get_status.errors` and the UI agree. One
  dismiss per line, plus one that clears them all.
- **Clear Cache is left out** of the task indicators; it keeps its settings
  page.
- **The restart toast is sticky** until the user restarts or closes it, as
  the pane's notice stayed until restart.
- **Every `step_note` becomes a toast of one style**, refusals included; no
  severity is added. `get_status` keeps the note.
- **Icon placement inside or beside the circle** is decided from a
  screenshot in Step 1; the row stays at `STATUS_ROW_HEIGHT`.
- **The existing `\u{25be}` and `\u{25f7}` glyphs are left alone** (outside
  this change).
- **Three PRs**, one per step.
- A kind's slot may hold more than one task (a held trash run and a running
  one); the slot shows the running one and the popover lists both.

## Steps

- [x] Step 1: The task list model and one indicator per running task in the
      status row
  - Done when:
    - A pure module gives the list of all running tasks instead of one
      `(done, total)`: `progress::shown` is replaced by a function that
      returns every running task (kind, label, `done` / `total` or
      indeterminate, the one-line tooltip text), in a fixed kind order
      (scan, trash, folder, update).
    - The status row draws, at its right end, one slot per task kind in
      that fixed order, each a gpui-component `ProgressCircle` with the
      kind's Lucide icon (`scan-face`, `trash`, `folder`, `download`); a
      slot whose kind has no task is empty but keeps its width, so the
      circles and the strip position do not shift as tasks start and end.
    - A task without a known total (a held operation waiting for the scan,
      an update download of unknown size, the update's verify) is drawn
      dim / indeterminate.
    - Hovering an indicator shows its one-line text in a gpui-component
      `Tooltip` (for example the text `trash::progress_status` gives).
    - The meta pane is unchanged in this step (it still draws its notes
      and errors), and `get_status` is unchanged.
    - Unit tests cover the task list model: nothing running, each kind
      alone, several kinds at once keeping the fixed order, determinate
      and indeterminate, the tooltip texts. A UI test in
      `crates/app/src/uitest.rs` checks the view's task list during a run
      it can observe (a scan of an opened folder).
    - `CLAUDE.md`'s description of `progress.rs`, `icons.rs` and the
      status row, and the matching part of `docs/agents/app.md`, are
      updated.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/app/src/progress.rs` (extend it, or add a sibling
      pure module such as `tasks.rs` and leave `fraction` where it is;
      decide by size), `crates/app/src/icons.rs`,
      `crates/app/src/main.rs` (`status_row`, `progress_circle`),
      `crates/app/src/uitest.rs`, `CLAUDE.md`, `docs/agents/app.md`.
    - Keep the model pure: it takes plain inputs and builds no GPUI
      types. The inputs `progress_circle` passes today are the sources for
      the determinate tasks: `self.trashing` (mode, done, total),
      `self.undo_progress`, `folderops.sequence_progress()` /
      `import_progress()` / `sidecar_progress()`, `scan_status.running()`
      (and `scan_status.pass()` for the pass name). Add the held operation
      (`self.gate.waiting()`) and the update download
      (`update::Phase::Downloading { percent }`, `Phase::Verifying`).
    - Reuse the existing texts for the tooltips: `trash::progress_status`,
      `folderops.note()`, `folderops.import_note()`, the update's phase
      text in `update.rs`, the scan's `Status::suffix`.
    - Icons: `SCAN_FACE` and `FOLDER` are already constants in `icons.rs`.
      Add `trash` and `download` (and, for Step 2, `triangle-alert` and a
      close `x`) through `icon_assets!` unless the default `Assets` already
      serves them, and extend the `every_icon_path_loads_an_svg` test.
      Verify the catalog's names in `gpui-kit-assets` at the pinned 0.9.1.
    - Verify whether `ProgressCircle` has an indeterminate / loading state
      and whether it takes a child. If not, draw the indeterminate state as
      a dimmed ring and place the icon with the app's own `div`s. Measure
      from a screenshot whether an icon inside a `.small()` circle is
      legible; if not put it beside the circle.
    - The tooltip follows the existing uses of gpui-component `Tooltip` in
      `main.rs` (`settings_button`, the strip's tip regions).
- [ ] Step 2: The task popover, the failure indicator, and the meta pane
      stops drawing running notes and errors
  - Done when:
    - Clicking any indicator opens one popover, drawn `deferred` /
      `anchored` above the status row like the strip header's Sort /
      Filter menus, listing every task: its name, a progress bar,
      `done/total`, a Cancel button where the task is cancellable, held
      operations shown as "waiting" (no Cancel), and finished tasks
      lingering for a few seconds before they leave the list.
    - Failures (today's `pane_errors()` lines) have their own slot in the
      status row: a Lucide `triangle-alert` (or `circle-alert`) icon in
      the theme's danger color, shown while any failure is kept, with a
      tooltip. The popover lists the failure lines, each dismissable
      through an icon button (Lucide `x`), plus one control that clears
      them all; a dismiss clears the underlying failure state, so
      `get_status.errors` drops it too. No emoji or glyph character is
      used anywhere in the popover or the indicator.
    - A press outside the popover and Escape close it.
    - Cancel runs the same code paths the app's own Cancel and the MCP
      `cancel_operation` use.
    - The meta pane no longer draws the trash run's progress, the folder
      operation's note, the waiting note, the import's note or the error
      lines. `get_status` still reports the same `notes` and `errors`
      (less dismissed failures).
    - Unit tests cover the model's additions: the lingering of a finished
      task against a given clock, which tasks are cancellable, the failure
      list and its dismissal. UI tests cover opening the popover's state
      from a task and dismissing a failure.
    - `CLAUDE.md`, `docs/agents/app.md` and `docs/humans/usage.md` with
      `docs/humans/usage.ja.md` (the passages that say failures are listed
      "at the bottom of the right pane") are updated in the same PR.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Assumes Step 1 is merged.
    - Files: the Step 1 model module, `crates/app/src/main.rs` (the
      popover, `meta_pane`'s tail at the `pane_notes()` / `pane_errors()`
      children, the cancel wiring), `crates/app/src/icons.rs`,
      `crates/app/src/trash.rs` (`Failures`), `crates/app/src/folderops.rs`,
      `crates/app/src/uitest.rs`, the docs above.
    - Popover: copy `menu_button`'s pattern in `main.rs` (a `relative`
      wrapper, `deferred(anchored().anchor(Anchor::BottomRight)
      .snap_to_window())`, `on_mouse_down_out` to dismiss). Keep its open
      state in the view; it is not a modal, so it does not join
      `companion::Busy`'s refusals.
    - `pane_notes()` and `pane_errors()` stay as the source of
      `companion::Status.notes` / `.errors` (`status_of`); only their
      drawing in the pane goes. Reword their doc comments.
    - Cancel: for a sequencing or import run reuse
      `dismiss_folder_dialog(false, cx)` (what `companion::Cancel::Folder`
      runs). `cancel_of` in `main.rs` is the existing rule of what is
      cancellable. A trash run, its undo / redo, a sidecar run and a held
      operation get no Cancel button.
    - Lingering: take the finished task's text from `record_finished` /
      `last_op`. The model takes the clock as an argument; the view
      schedules one timer to redraw when the linger ends.
    - Failures come from four places joined by `pane_errors()`:
      `self.notes`, `outputs_failure`, `tree_note` and `write_errors`
      (`trash::Failures`); a dismiss clears the dismissed line's source.
    - The progress bar is a gpui-component control if it has one at 0.9.1,
      else a plain `div` track.
    - Check `companion.rs`'s `Coverage` table and its test for whether the
      popover's actions need an entry.
- [ ] Step 3: Toasts for the step note and the restart notice, the
      view-only badge, and the meta pane's notes area removed
  - Done when:
    - A one-off result that today sets `step_note` (a trash run's result,
      an undo's or redo's result, a folder operation's result or refusal)
      is shown as a transient gpui-component notification in the window.
    - The update's restart notice (`Updates::notice`) is shown as a sticky
      notification with a `Restart Now` action that runs the same
      `restart_now` as the Updates page, pushed once under one id.
    - A view-only (JPEG-only) folder shows a badge in the strip header
      with the view-only text; the badge uses no emoji or glyph character.
    - The meta pane draws no notes and no errors at all.
    - `get_status` still reports the same `notes` (the view-only note, the
      step note, the running / held note, the import note, the update
      notice) and `errors`.
    - UI tests cover the view-only badge's state and that a step note is
      still recorded for `get_status`; the existing assertions on
      `view.step_note` in `uitest.rs` keep passing or are updated.
    - `CLAUDE.md`, `docs/agents/app.md` and `docs/humans/usage.md` with
      `docs/humans/usage.ja.md` are updated in the same PR.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Assumes Step 2 is merged.
    - Files: `crates/app/src/main.rs`, `crates/app/src/folderops.rs`,
      `crates/app/src/update.rs`, `crates/app/src/viewonly.rs`,
      `crates/app/src/uitest.rs`, the docs above.
    - Keep the `step_note` field. It is assigned at about 25 sites across
      `main.rs` and `folderops.rs`; route them through one setter that
      stores the note and pushes the toast; the clears stay as they are.
    - The only `window.push_notification` today is the import's OS
      notification in `folderops.rs` (pushed from `cx.defer` through the
      stored window handle); use the same route where a site has no
      `Window`. Measure where an in-window notification appears relative
      to the title bar, the settings layer and an open dialog.
    - Verify at 0.9.1 how a notification carries an action button. The
      toast may need a shorter text than `Updates::notice`.
    - The badge goes in `strip_header`; reuse `viewonly::VIEW_ONLY_NOTE`
      or a shorter label, with a gpui-component `Tag` / `Badge` if one fits.
    - Under `--auto` and in the UI tests no toast may block or steal focus.

## Progress
