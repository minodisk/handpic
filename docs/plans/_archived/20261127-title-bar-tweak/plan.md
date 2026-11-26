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

# Title bar visual tweak

## Purpose

The title bar's text, app icon and settings gear were retuned in a dev build
and approved by the user: 13 px text, an 18 px app icon, and a gear with a
thinner stroke in a box shaped like a window control, flush with the Windows
window controls. The code is already in the working tree, uncommitted. This
plan ships it with the docs that describe the title bar and `icons.rs`
brought in line.

Earlier title bar work:
[the bar](../_archived/20261116-native-title-bar/plan.md),
[the gear](../_archived/20261117-native-title-bar-settings/plan.md),
[the folder name](../_archived/20261118-native-title-bar-folder-name/plan.md).

## Steps

- [x] Step 1: Ship the approved title bar diff with its docs
  - Done when:
    - The working tree's diff of `crates/app/src/main.rs` and
      `crates/app/src/icons.rs` is committed with its behavior unchanged from
      what the user approved:
      - "Handpic" and the centered folder name at `.text_size(px(13.))`
      - `img(icons::APP)` at 18 px
      - `settings_button`: the wrapper `div` with `.h_full()` and no
        `.mr_2()`; the `Button` as
        `.ghost().with_size(px(14. / 0.75)).w(TITLE_BAR_HEIGHT).h_full().rounded(ButtonRounded::None)`
        with `Icon::default().path(icons::SETTINGS_THIN)`
      - `icons.rs`: `SETTINGS_THIN` (`icons/settings-thin.svg`, Lucide
        `settings` at `stroke-width="1.5"`), three `SVGS` entries, the header
        comment naming `settings`, and `SETTINGS_THIN` in the test `PATHS`
    - Root `CLAUDE.md` matches the code:
      - the title bar clause (now "the app icon and "Handpic" at `text_sm`,
        the open folder's name at the same size centered ... a gear
        `Button`") gives the 13 px text, the 18 px icon and the gear's
        window-control box with the thin `settings` derivative
      - the `src/icons.rs` clause (now "the two own derivatives, the filled
        star and the frowning `scan-face`") names three, the thin `settings`
        gear among them
    - `docs/agents/app.md` matches the code:
      - the title bar section: its `h_flex().gap_2().text_sm()`, "16 px",
        "14 px", "ghost, small ... `IconName::Settings` in a `div` with a
        `mr_2` margin", and the folder name overlay's "at `text_sm`"
      - the `icons.rs` paragraph near "own derivatives (the filled star, the
        frowning `scan-face`)", and the add-an-icon rule's derivative
        examples "(a filled shape, the `scan-face` frown)"
      - a pending check noted: the margin-less square gear was seen only on
        Windows 11; macOS and Linux are unseen
    - The header comment in `icons.rs` is true about where the `settings`
      base came from: compare its path data with lucide-static v1.48.0's
      `settings.svg`; if they differ, reword the comment to name
      `gpui-kit-assets` 0.9.1 as this icon's source (no pixel change)
    - `mise run ci:local` passes locally and the PR's GitHub checks pass,
      the `handpic-app` `icons` tests among them
  - Implementation approach:
    - Do not restyle or refactor the diff; the user approved these exact
      values. Only docs are added, plus a header-comment wording fix if the
      base version check calls for one.
    - `README.md`, `README.ja.md` and `docs/humans/*` mention the gear only
      as the way to open the settings, with no sizes; no change, no `.ja.md`
      pair touched.
    - `crates/app/LICENSE-lucide` needs no change.
    - In `docs/agents/app.md`, record why the button is sized this way:
      gpui-component draws a button's icon at 0.75 of the button's size, so
      `with_size(px(14. / 0.75))` gives a 14 px icon, and the box is then
      set to `TITLE_BAR_HEIGHT` wide, full height, square corners, after the
      window controls' own box. Seen in a dev build on Windows 11.
    - Keep the rules in that section that still hold: the wrapper `div`
      stopping the left press's propagation, `tab_stop(false)`, the tooltip
      on the wrapper, `TITLE_NAME_MARGIN`.
    - Commit as `feat(app): retune the title bar's text, icon and settings gear`.

## Progress

(2026-11-27) Step 1 complete
