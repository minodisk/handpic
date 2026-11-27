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

# Windows device checks c6

## Purpose

A todo sweep (dlg-todo-c6) ran the open Windows real-device checks of eleven
`todo.md` items on 2026-11-27 (Windows 11, one 5120 x 1440 display at 100 %).
This plan records what was seen (`results.md` in this folder), closes the
boxes that passed, rewrites the others to what is left and why it could not
be run on this host, and moves two results into the agent docs: the shell's
permanent delete under a watch (`docs/agents/shared-crates.md`) and the
import notification's delivery (`docs/agents/app.md`). Docs only.

Approved through the sweep's delegation brief, which asks for the recommended
option wherever a choice is open. Decisions taken: the `native-themes` merged
box keeps the old boxes' checks that `results.md` does not confirm (restart
restoring the choice before the first frame, the preview's surround and the
strip's names and failures on a shipped theme, Neutral again looking as
before) as still open, rather than closing them on no observation;
`native-tree-listitem` is deleted whole (its states were also seen in Neutral
light, Catppuccin Latte and Catppuccin Mocha in the same run's screenshots);
`app.md`'s notification section becomes (Measured) as its todo box asks;
`shared-crates.md` gets a paragraph rather than a fourth table column.

## Steps

- [x] Step 1: Record the 2026-11-27 results and curate `todo.md`
  - Done when:
    - `results.md` in this plan folder is the sweep's results text (already
      in place, uncommitted).
    - `todo.md`, by heading, nothing else in the file changed:
      - `native-import-notification`: box 1 is rewritten to what is left (the
        taskbar button's icon and grouping, not seen because the taskbar is
        auto-hidden on the host); box 2 is deleted; the Background has the
        2026-11-27 results.
      - `native-gpui-kit`: the Windows box is deleted, the result is in the
        Background (with a clause that the app no longer has a `Checkbox`),
        the Linux box stays.
      - `native-themes`: the two Windows boxes become one box holding what is
        left: System following the OS appearance live and Dark staying dark
        whatever the OS does (not run: switching the OS appearance changes
        the shared desktop under other sessions' checks), the tree cursor's
        ring and the progress bar in Light, the burst band, the preview's
        surround and the strip's names and failures on a shipped theme,
        Neutral again looking as before, and the restart restoring the choice
        before the first frame (restored, first frame not captured). The
        macOS and Linux boxes stay; the results are in the opening paragraph.
      - `Engine: measure a file manager's permanent delete (Shift+Delete)…`:
        the whole item is deleted.
      - `native-context-menu`: the first four Windows boxes (the tree's menu,
        the strip's menu, placement and size, the items) are deleted and their
        results are in the item's opening paragraph; the box that starts
        "On Windows, with a menu open" is byte-for-byte unchanged (another
        session owns it).
      - `native-tree-listitem`: the whole item is deleted.
      - `native-strip-menu-order`: the Windows box is deleted; the macOS /
        Linux box takes its steps and expectations.
      - `native-label-tags`: the whole item is deleted.
      - `native-strip-resize`, the HiDPI box: one added line saying why it
        cannot be done here (the host has one 5120 x 1440 display at 100 %;
        changing the scale would disturb the shared desktop).
      - Always-visible scrollbars: the meta pane drag box and the notes inset
        box are deleted; the pre-gutter comparison box stays with a note (it
        needs a build from before #1147, when the crate was `crates/native`);
        the System-mode box stays with the shared-desktop reason; the macOS /
        Linux box names the checks it still means.
      - `gpui-packaging`, the Windows box: one added line saying why not here
        (it installs Inno Setup and a per-user Handpic under
        `%LOCALAPPDATA%\Programs\Handpic`, where the host's own Handpic
        install lives).
      - `App: real-device check of the look of the switch and the Kbd keys`
        is unchanged (another session owns it).
      - Boxes that said "above" no longer point at deleted text.
    - `docs/agents/shared-crates.md` records the shell's permanent delete
      (free at depth 0, 1 and 2, the name gone and re-creatable,
      `Remove(Any)` for the two files, the path given without `\\?\`) and no
      longer says it "was not measured".
    - `docs/agents/app.md`'s section "An import's end posts a system-only
      notification while inactive" is marked (Measured), its stale "pending"
      caveat is replaced, and its Seen bullet gains the 2026-11-27 results.
    - `docs/humans/usage.md` and `usage.ja.md` are unchanged.
    - References to the results from `todo.md` and `docs/agents/` use
      `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`
      (backticked in todo.md, as its neighbours are).
    - `mise run lint` (lychee) passes and the PR's GitHub checks pass.
  - Implementation approach:
    - Single-PR mode: the plan folder is archived in this same PR.
    - Follow the file's wording for a Background addition: "Seen on Windows
      11 on 2026-11-27 (results in
      `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`):
      …". `native-themes` and `native-context-menu` have no Background
      heading; append to their opening paragraph.
    - Verify the range with `git diff --stat` (only `todo.md`,
      `docs/agents/app.md`, `docs/agents/shared-crates.md` and the plan
      folder).
    - Other sessions edit `todo.md` concurrently; on a rebase conflict keep
      their text.

## Progress
