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

# Settle three "decide" items: menus with no folder, the progress circle's slot, long strip names

## Purpose

Three `todo.md` items left by the 2026-11-22 Windows device checks ask for a
decision rather than a fix. This plan settles each one and implements it, so
the items can be closed:

1. `### App: decide whether the Sort / Filter menus open with no folder`
   Decision: they do not open; the buttons are disabled and dimmed, as Sort
   already is in a view-only folder. With no folder every other key and click
   does nothing; the two menus were the only exception
   (`../_archived/20261122-windows-device-checks-3/learnings.md`).
2. `### App: decide whether the status row reserves the progress circle's slot`
   Decision: the slot is always reserved, so the position text (`1 / 600`)
   no longer shifts by about 20 px when the circle appears or goes
   (`../_archived/20261122-windows-device-checks-4/learnings.md`).
3. `### App: decide whether a strip cell's name longer than the cell ends in an ellipsis`
   Decision: a name wider than its cell is shortened with an ellipsis in the
   middle, keeping the start and the tail (frame number and extension),
   instead of being centered and clipped on both sides
   (drawn as `)SC-RX100M2_DSC00416.ARW` in the same learnings).

The user confirmed all three decisions (the recommended ones) and single-PR
mode at plan approval on 2026-11-22; the alternatives considered are under
"Trade-offs and risks".

## Steps

- [x] Step 1: Implement the three decisions in `crates/app` and record their rules
  - Done when:
    - With no folder open (`Handpic.dir` unset), the `Sort ▾` and `Filter ▾`
      buttons are disabled (dimmed, no press listener) and open no menu, and a
      test pins it. With a folder open both behave as before (Sort still inert
      in a view-only folder).
    - The status row keeps a slot of the progress circle's size at the right
      of the position whether or not an operation runs, so the position text
      stays where it is when the circle appears or goes. The
      "One progress circle sits in the status row's right region" section of
      `docs/agents/app.md` states the reserved slot in place of "so the
      position shifts by the circle's width when it appears or goes", and its
      first Rule's "only the center region is 20 px narrower … while it
      shows" is corrected to match (the center region is now always that
      much narrower).
    - A strip cell's name wider than the cell is drawn shortened with `…` in
      the middle, inside the cell, and a name that fits is drawn unchanged and
      still centered. A test pins the shortening rule (a long name keeps its
      start and its tail with the extension; a short name is untouched).
    - `docs/agents/app.md` records the two other rules too (menus do not open
      with no folder; how a long strip name is shortened), each marked
      Inferred or Verified according to whether it was seen on screen.
      `CLAUDE.md`'s layout paragraph is updated only if a named function or
      module is added that it would otherwise misdescribe.
    - The three `todo.md` items are closed out. Their headings, for the
      `todo-curator` at wrap-up:
      - `### App: decide whether the Sort / Filter menus open with no folder`
      - `### App: decide whether the status row reserves the progress circle's slot`
      - `### App: decide whether a strip cell's name longer than the cell ends in an ellipsis`

      The sentences elsewhere in `todo.md` that point at these items ("the
      two items below" under the gpui-tree-trash item, "the item below
      decides whether the row reserves the slot" under the StatusBar item)
      are the curator's to adjust.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - Items 2 and 3 are visual. If a Windows by-eye check is run (a debug
      build, `--features screenshot` for captures), it uses only copies under
      `D:\Photos\tests\`, a fresh `--index` and an explicit `--settings`
      test file; its result goes to `learnings.md`. If it is not run, say so
      in `learnings.md` so the wrap-up can leave a real-device check item.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/main.rs` (all three), `crates/app/src/menu.rs`
      and/or `crates/app/src/uitest.rs` (item 1's test),
      `crates/app/src/strip.rs` (item 3, if the rule is made a pure function),
      `docs/agents/app.md`.
    - Item 1. `strip_header` passes `menu_button` an `enabled` flag: today
      `!self.view_only` for Sort and `true` for Filter. A disabled button
      gets no `on_mouse_down` listener and is drawn at `opacity(0.5)`.
      Fold `self.dir.is_some()` into both flags, reusing that mechanism.
      `dir` is only ever set (`self.dir = Some(..)` in the listing handler),
      never cleared, so no open menu needs closing when a folder goes.
      For the test: `uitest.rs`'s `Ui::open()` is a view with no folder and
      `Ui::update` reaches the private `menu_pressed` and `menus`; no test
      there simulates a mouse click yet. Pin the guard either through a small
      pure function (e.g. in `menu.rs`, taking the folder and view-only state)
      that `strip_header` uses, or through a simulated click on the button.
      Do not change the MCP `set_sort` / `set_filter` path (`apply_menu`),
      which opens no menu.
    - Item 2. `status_row` adds the position and then, only when
      `progress_circle` returns one, the circle, both through
      `StatusBar::right` (the right region is an `h_flex` with `gap_2`).
      `ProgressCircle` at `Size::Small` is `size_3` (12 px) in
      gpui-component 0.9.1. Add an empty, non-shrinking element of that size
      in the circle's place when there is none, so the region's content
      width is constant. Keep the order (position, then circle) and
      `STATUS_ROW_HEIGHT`; add no `notify` or timer.
    - Item 3. `paint_strip` shapes `cell.name` at `px(11.)` with
      `shape_line` and paints it `TextAlign::Center` in `cell_width` under
      the cell's content mask, which is what clips both sides. GPUI
      (gpui-pre 0.3.12, re-exported through `gpui_kit`) has
      `TextSystem::line_wrapper(font, font_size)` and
      `LineWrapper::truncate_line(line, width, "…", runs, TruncateFrom::Middle)`,
      which keeps about two thirds of the width for the start and one third
      for the end and returns the line and runs unchanged when it fits; it
      measures by summed per-character widths, so verify on a long name that
      the shaped result does stay inside the cell (leave a small inset if it
      does not). At the default strip height the cell is 146 px wide, and it
      changes with the strip's height (`strip::Geometry::of`), so truncate
      against the current `cell_width`, at paint time or wherever the width
      is known.
      For the test, decide between (a) using `truncate_line` and pinning it
      through `Ui` (check first whether the test platform's text system gives
      usable character widths), and (b) a pure function in `strip.rs` that
      takes the name, the width and a character-width function, unit-tested
      there. Prefer (a) if it can be tested, since it adds no truncation code
      of its own.
    - Leave the name band's tooltip (`strip::tip_regions`: the label's name)
      as it is; showing the full file name on hover is not part of this item.
    - Out of scope, owned by another session, do not touch: the held folder
      rename's `Access denied`, the folder operation dialogs' list (blank
      band, clipping), the `\\?\` prefix in the status row and the Import
      dialog, the Trash Undo's count and reason, and the "open the folder
      first" wording.

## Trade-offs and risks

- One step or three. The three changes are independent, but each is a few
  lines in the same crate, mostly in `main.rs`, so they ship as one step in
  single-PR mode (confirmed by the user).
- Item 1, disabled look or silent press. Chosen: disabled and dimmed, so the
  state is visible. Alternatives considered: keep the normal look and ignore
  the press in `menu_pressed`; or keep them opening (the Filter choice would
  apply to the next folder opened).
- Item 2, reserve the slot or reorder. Chosen: always reserve 12 px plus the
  8 px gap right of the position; the cost is a status text region
  permanently 20 px narrower and a small empty space at the row's right end
  while idle. Alternatives considered: put the circle left of the position;
  or record the shift as intended.
- Item 3, where the ellipsis goes. Chosen: middle (`DSC-RX100M…416.ARW`-like):
  long names in a folder usually share their start and differ in the trailing
  frame number, so the tail and the extension stay readable, the start still
  identifies the series, and GPUI does it with no code of our own.
  Alternatives considered: end (matches the tree rows and the title bar, but
  cells of a prefixed folder read the same and the extension is lost); start
  (loses the prefix); a custom split always keeping the whole extension
  (needs its own truncation code).
- Risk: none of the three is observable in a unit test as pixels. The slot
  (item 2) has no test beyond the rule in `app.md`, and item 3's fit inside
  the cell depends on real font widths; without a Windows by-eye check both
  stay Inferred.

## Progress
