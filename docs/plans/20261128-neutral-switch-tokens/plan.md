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

# Neutral switch tokens

## Purpose

In the GPUI app's settings, an off `Switch` (auto-advance on the Culling
page, and the two switches of the MCP page) is hard to read in Neutral dark.
Measured on 2026-11-27 (Windows 11, Neutral dark, 100 %, a debug build with
the `screenshot` feature, MCP `screenshot`): off, the thumb is `#0a0a0a` on a
`#404040` track over the settings' `#171717` surface, so the thumb is darker
than the surface, about 1.9:1 against its track, and looks like a hole. On,
the track is `#e5e5e5` with the same thumb, which reads fine. The `Kbd` chips
were fine.

`crates/app/src/theme.rs`'s Neutral mapping leaves `switch` / `switch_thumb`
at gpui-component's `Default Dark` / `Default Light` values;
[the native-component-controls plan](../_archived/20261116-native-component-controls/plan.md)
decided to map them if the off state reads poorly. This plan maps them so
the off state reads clearly, and closes the `todo.md` item that asked for
the look.

Decision already made: follow shadcn/ui Neutral's switch as far as the
component allows. shadcn's dark switch has a light (`foreground`) thumb when
off and a dark (`primary-foreground`) thumb when on. gpui-component 0.9.1's
`Switch` cannot do that: it paints the on track with `tokens.primary`, the
off track with `tokens.switch` and the thumb, in both states, with the one
`tokens.switch_thumb` (`src/switch.rs`; the app's switches are
`SettingField::switch`'s, with no per-instance hook). A light thumb would
vanish on the `#e5e5e5` on track. So the fallback is taken: the thumb stays
dark and the off track (`switch`) is lightened. The PR body records this.

## Steps

- [x] Step 1: Map `switch` and `switch_thumb` in the Neutral mapping
  - Done when:
    - `theme.rs`'s `map` sets `colors.switch` and `colors.switch_thumb` from
      the palette, in dark and in light, and
      `maps_the_neutral_dark_tokens` / `maps_the_neutral_light_tokens` pin
      both fields' values.
    - `docs/agents/app.md` says what the mapping gives: the Neutral color
      listing (the "In dark: ..." / "In light, ..." paragraph of
      "gpui-component is initialized; the app's controls draw from it") and
      the settings modal's sentence that today reads "`primary` when on and
      `Default Dark`'s `switch` / `switch_thumb` (`theme.rs` maps neither)",
      with why the thumb is not shadcn's per-state one.
    - `todo.md`'s section headed "App: real-device check of the look of the
      switch and the `Kbd` keys (native-component-controls)" is deleted
      (heading, Background, Files, TODO).
    - `CLAUDE.md` is unchanged: its `theme.rs` description does not
      enumerate the mapped tokens.
    - No file under `crates/app/themes/` changes.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The PR body records the decision (shadcn's per-state thumb is not
      possible with one `switch_thumb` token, so the off track was lightened
      and the thumb kept), the values chosen and the result of the manual
      check below.
  - Manual check (run by the main agent in the wrap-up, not by the
    implementer; its result goes in the PR body):
    - On the Windows host, a debug build with the `screenshot` feature
      (`mise run app:dev:screenshot`), launched with its own `--index`,
      `--settings` and `--log` in a temp folder, `--mcp-port 47880` and
      `--import-settings` of `{"mcpEnabled": true}`; never the user's real
      index, settings or photos; `docs/agents/app.md`'s real-device-run
      rules apply (quit by the instance's own PID or `Escape` then `q`, the
      previous PID gone before the next launch on the same port).
    - Neutral dark and Neutral light, the switch on and off: set the theme
      and `auto_advance` through MCP `set_settings` before the settings open
      (a change is refused while they are open), open the settings with
      `ctrl+,`, select Culling with Win32 synthetic clicks, take MCP
      `screenshot`s. The MCP page shows an on switch (`mcpEnabled`) and an
      off one (`mcpAllowDestructive`) side by side under the same seed and
      can back the Culling shots.
    - Expected pixels at 100 %: dark off track `#737373` with a `#0a0a0a`
      thumb on the `#171717` surface, dark on track `#e5e5e5` with the same
      thumb; light off track `#d4d4d4` with a `#ffffff` thumb on the
      `#ffffff` surface, light on track `#171717` with the same thumb. The
      off switch reads as a switch, not a hole, in both modes.
    - Also look at an off switch while it has the keyboard focus (after a
      click or Space): its border is tinted `ring`, `#737373` in dark, the
      same as the new off track (see Trade-offs).
    - Optional: the strip's right-click menu `Kbd` shortcuts, the other half
      of the deleted `todo.md` item, if they were not looked at on
      2026-11-27 (see Trade-offs).
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/theme.rs`, `docs/agents/app.md`, `todo.md`,
      this `plan.md`.
    - `Palette` holds one value per mapped color and `map` knows no mode, so
      the off track needs its own `Palette` field (`switch`), set in `DARK`
      and `LIGHT`; `map` gets `colors.switch = rgba(p.switch).into()` and
      `colors.switch_thumb = rgba(p.background).into()` with a short comment
      in the style of the neighbouring groups (why: one thumb token for both
      states, so the off track carries the legibility).
    - Values: dark `switch` `0x737373ff` (Tailwind neutral-500; the
      `#0a0a0a` thumb is about 4.2:1 on it, the track about 3.8:1 against
      the `#171717` surface, and it stays well apart from the `#e5e5e5` on
      track); light `switch` `0xd4d4d4ff` (neutral-300, `Default Light`'s
      value today, so no visible change in light); `switch_thumb` is the
      palette's `background` in both (`#0a0a0a` / `#ffffff`, today's values,
      now pinned rather than left to the built-in theme's load-time
      fallback).
    - An edit of `colors.switch` / `colors.switch_thumb` inside
      `Theme::update` reaches `theme.tokens`, which the `Switch` reads:
      `Theme::edit` reconciles edited colors into the tokens
      (gpui-component 0.9.1 `src/theme/mod.rs`), the path the mapped
      `title_bar` / `status_bar` already rely on. No change outside `map`
      and the palettes is needed.
    - Tests: add the two `assert_eq!` lines per mode to the existing
      `maps_the_neutral_dark_tokens` and `maps_the_neutral_light_tokens`;
      the tests' `use super::{..}` line needs no change.
    - Keep the diff surgical: the in-flight
      `docs/plans/20261127-light-theme-dim-and-menu-green/plan.md` (branch
      `fix/light-theme-dim-and-menu-green`) edits `theme.rs`'s `Roles`,
      `darkest` neighbourhood and tests, `docs/agents/app.md` near the roles
      paragraph, and deletes another `todo.md` section. Do not reflow
      neighbouring lines in `map`, the palettes or `app.md`'s paragraphs
      beyond the sentences that change.
    - Commit as `fix(app): ...` (Conventional Commits, English).

## Trade-offs and risks

- **shadcn's per-state thumb vs. the lighter off track.** Taken: the lighter
  off track with the dark thumb, because gpui-component's `Switch` has one
  `switch_thumb` token for both states and the on track is the shared
  `primary`. Not taken: a light thumb (invisible on the `#e5e5e5` on track),
  and drawing an own switch in place of `SettingField::switch` (far outside
  a token mapping).
- **Dark off track `#737373` vs. `#a1a1a1`.** `#737373` is taken (the
  on and off tracks stay clearly different). It equals dark `ring`, and a
  focused switch's track border is tinted `ring`, so that 1 px line merges
  with the off track and only the 50 % halo outside it marks the focus.
  `#a1a1a1` (neutral-400) keeps the line visible and raises the thumb to
  about 7.7:1, but is only about 2:1 from the on track. If the manual check
  finds the focused off switch unclear, change the one `DARK.switch` value
  and its test line.
- **Neutral light.** Taken: pin today's `#d4d4d4`, no visible change;
  the light switch already has shadcn's shape (one white thumb, dark on
  track). Alternatives: shadcn's exact `input` `#e5e5e5`, which is fainter
  on the white surface (about 1.26:1 against 1.48:1), or `#a1a1a1`, more
  legible but further from shadcn. The manual check is the first look at
  Neutral light's switch on screen.
- **The deleted `todo.md` section also asked for the right-click menu's
  `Kbd` shortcuts** (right-aligned, `muted_foreground`). The 2026-11-27
  evidence says they were fine; the wrap-up looks again if convenient.
- **All three switches change**, not only auto-advance: the MCP page's
  `mcpEnabled` and `mcpAllowDestructive` read the same tokens. That is the
  intent, and the PR body says so.
- **Merge order with `fix/light-theme-dim-and-menu-green`.** No shared hunk
  is expected in `theme.rs` or `app.md`; `todo.md` has two separate section
  deletions (about 30 lines apart), which may need a trivial rebase.
- **Shipped themes are untouched**: they are installed unmapped and bring
  their own `switch.background` / `switch.thumb.background` or the
  component's fallbacks.

## Progress
