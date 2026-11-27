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

# Light themes: a readable reject dim and Face focus green

## Purpose

Two colors of `crates/app` were chosen for a dark surface and read badly on a
light one (seen on Windows 11 on 2026-11-23,
[results](../_archived/20261123-windows-device-checks-mcp-visual/results.md)):

- A rejected strip cell is dimmed with `Roles::rejected_dim`, which outside
  Neutral dark is the theme's `background` at 0.55 alpha. On a light
  background the cell washes toward white and melts into the surface.
- The `Filter ▾` menu's `Face focus` `Good` icon is drawn in
  `MarkState::color`'s `#33ff33`, chosen for drawing over a photo, and is hard
  to read on a light menu surface.

After this change a rejected cell darkens on a light theme, and the menu's
`Good` icon uses a darker green on a light popover. Neutral dark, every dark
theme, the strip's face icon and the `f` overlay keep today's colors. This
closes the `todo.md` items "App: a rejected strip cell's dim is too faint in
light themes" and "App: the filter menu's `Face focus` `Good` green is too
bright on light themes".

The plan was approved through the delegation brief of the todo sweep
(dlg-todo-c6), which asks for the recommended option wherever a choice is open.

## Steps

- [x] Step 1: Dim a rejected cell with black on a light background and give
      the filter menu's `Good` icon its own green on a light popover
  - Done when:
    - `Roles::of(colors, false).rejected_dim` is black at 0.55 alpha when
      `colors.background` is light and `colors.background.alpha(0.55)` when
      it is dark; `Roles::of(colors, true)` still gives `0x0000008c`.
    - `theme.rs` exposes `is_light(color)` (relative luminance above the
      black / white contrast crossover, about 0.179, over the existing
      `luminance`), with unit tests for a light and a dark color.
    - `mark.rs` has a pure `MarkState::menu_color(self, light_surface: bool)`:
      `Good` is a darker green (`0x15803d`) on a light surface and `0x33ff33`
      otherwise; `Low` and `Neutral` equal `color()` on both. It is unit
      tested, including that the light-surface green has at least 3:1
      contrast on white (or the value is asserted and the ratio stated in the
      doc comment, if computing the ratio in `mark.rs` would need a helper
      nothing else uses).
    - `menu_row`'s `Lead::Face(Some(state))` colors the icon with
      `state.menu_color(theme::is_light(cx.theme().popover))`; `paint_strip`'s
      face icon and the `f` overlay still call `MarkState::color`.
    - `theme.rs`'s role tests cover both branches: Neutral dark unchanged
      (`neutral_dark_roles_are_todays_colors`), a dark background outside
      Neutral dark (the `mapped()` case of
      `other_themes_derive_the_roles_from_their_tokens`), and a light
      background (`mapped_with(&LIGHT)`).
    - `docs/agents/app.md` describes the new rules where it describes these
      colors today (see the approach), and the two `todo.md` items are
      removed.
    - `cargo test` and `cargo clippy` in `crates/app` are clean for the
      changed modules, and the PR's GitHub checks pass.
    - The PR body holds the on-screen result on Windows in Neutral Light and
      Neutral dark, each showing a rejected strip cell and the open
      `Filter ▾` menu (taken by the main agent, not by this step's tests).
  - Implementation approach:
    - Files: `crates/app/src/theme.rs` (`Roles::of`, new `pub fn is_light`,
      the `rejected_dim` field's doc comment, tests), `crates/app/src/mark.rs`
      (`menu_color`, test), `crates/app/src/main.rs` (one line in
      `menu_row`), `docs/agents/app.md`, `todo.md`, and `CLAUDE.md` only if
      its `mark.rs` / `theme.rs` sentences need the new names (add
      `menu_color` / `is_light` in the same terse style if anything).
    - `luminance` in `theme.rs` is private and stays so; `is_light` is the
      one public entry. Keep the threshold a named constant with a comment
      saying where 0.179 comes from (the luminance at which black and white
      text contrast equally).
    - `mark.rs` takes a `bool`, not a color, so it stays free of the theme
      and GPUI types; the call site in `main.rs` does the `is_light` test.
    - Only the non-Neutral-dark branch of `Roles::of` changes.
    - `paint_strip` needs no change.
    - `docs/agents/app.md`: the `Roles` paragraph ("the dim from `background`
      with alpha") and the menu's lead list ("in `MarkState::color()`").
    - `todo.md`: remove the two items by their headings, whole sections.
    - `0x15803d` (Tailwind green-700) is about 5:1 on white; verify the ratio
      when writing the test.
    - CI runs on GitHub Actions; a full `mise run ci:local` may be skipped
      locally in favour of the targeted `cargo test` / `cargo clippy` in
      `crates/app`.

## Trade-offs and risks

- **What sits under the dim on a light theme.** The dim quad covers the whole
  cell, the file name included; on Neutral Light the name's dark text ends up
  on a mid-gray. The on-screen check decides whether 0.55 stays.
- **`band_dimmed` is not changed.**
- **`Low` stays `0x999999` in the menu** (about 2.85:1 on white). It was not
  reported; `menu_color` gives one place to darken it later.
- **Light test by luminance, not by theme mode**, so a vendored theme whose
  mode and surface disagree is still judged by its surface. `luminance`
  ignores alpha.

## Progress
