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

# Native OS Title Bar

## Purpose

PR #1100 (commit 75cedfd8, plan
`docs/plans/_archived/20261116-native-title-bar/plan.md`) replaced the OS
title bar of the GPUI app (`crates/native`) with gpui-component's `TitleBar`
(a 34 px app-drawn row with the app icon and "Handpic"). The user finds the
custom bar clunky and prefers the OS title bar, which reads as more app-like.
This plan takes the app back to the OS default title bar, titled "Handpic",
removes the code that only the bar needed, and makes `CLAUDE.md` and
`docs/agents/native-app.md` match the code again.

## Steps

- [x] Step 1: Restore the OS title bar in the native app and remove what only
      the app-drawn bar needed
  - Done when:
    - `main()` in `crates/native/src/main.rs` opens the window with
      `titlebar: Some(TitlebarOptions { title: Some("Handpic".into()), ..Default::default() })`,
      without `..TitleBar::title_bar_options()`, without
      `window_decorations: Some(WindowDecorations::Client)` and with
      `..Default::default()` in place of `..TitleBar::window_options()`.
      `window_bounds`, `display_id` and `app_id` are unchanged.
    - `impl Render for Riffle` no longer adds the `TitleBar::new()` child, so
      the main area starts at the top of the client area (no 34 px offset),
      and `menu_max` no longer subtracts `TITLE_BAR_HEIGHT`.
    - Nothing the revert leaves unused remains: the imports `TitleBar`,
      `TITLE_BAR_HEIGHT`, `h_flex`, `img` and `WindowDecorations` in
      `main.rs`; in `crates/native/src/icons.rs` the `APP` constant,
      `APP_PNG`, the `APP` arms of `Icons::load` and `Icons::list`, the test
      `the_app_icon_path_loads_png_bytes`, the `APP` assertion in
      `the_listing_names_each_icon_under_the_prefix_once`, and the module
      doc's sentence about the title bar's `img(..)`. No `#[allow(dead_code)]`
      or unused-import warning is added to keep any of them.
    - The remaining tests of `icons.rs` (the own SVGs, the catalog icons of
      #1106, the components' fallback, the listing) and of `theme.rs` (#1097 /
      #1102 / #1104) pass unchanged apart from the removed `APP` lines.
    - `CLAUDE.md` no longer says the window opens under gpui-component's
      `TitleBar` in place of the OS title bar, and no longer lists the title
      bar's `crates/app/icons/64x64.png` for `img(..)` in the `src/icons.rs`
      description; the `src/theme.rs` description matches what the code
      still does with `title_bar` / `title_bar_border`.
    - `docs/agents/native-app.md` no longer describes an app-drawn bar: the
      section "The title bar is gpui-component's `TitleBar` with the icon and
      "Handpic"" is replaced by a short section stating that the window keeps
      the OS title bar (rule: `title: Some("Handpic")` gives the taskbar and
      Alt+Tab text; do not bring `TitleBar` back, with the reason), the
      status row section's sentence "The popover's height cap is the
      window's less 40 px and the title bar's height" is corrected, and the
      theming section's mention of `TitleBar`'s `title_bar` /
      `title_bar_border` matches the code. No file under
      `docs/plans/_archived/` is edited; a link to the archived plan may
      stay as the source of the history.
    - `grep -rn "TitleBar\|TITLE_BAR_HEIGHT\|icons::APP\|app\.png" crates/native/src`
      finds nothing (`TitlebarOptions` is GPUI's and stays).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - A manual check on Windows (`mise run native:dev`): the OS title bar
      shows with the title "Handpic", there is no app-drawn bar, the preview
      / tree / meta pane / strip / status row fill the client area from its
      top, the window moves, maximizes / restores, minimizes and closes
      normally, and the taskbar and Alt+Tab read "Handpic". If the agent
      cannot run it, the check lands in `todo.md` at wrap-up.
  - Implementation approach (as far as it is known; omit if unknown):
    - This is a manual, partial revert, not `git revert 75cedfd8`: later PRs
      (#1098 gpui-kit, #1099, #1102 / #1104 themes, #1106 icons) touched the
      same lines. For example the paddings call is now
      `gpui_kit::component::window_paddings`, and `Icons::list` chains
      `Catalog` and `Assets`.
    - `crates/native/src/main.rs`: remove the `.child(TitleBar::new().child(h_flex()...))`
      block in `impl Render for Riffle` (around line 4503), the
      `- TITLE_BAR_HEIGHT` term of `menu_max` (around line 4471), and the
      three window option lines in `main()` (around lines 5559-5567). Then
      drop the imports the compiler reports unused (expected: `TitleBar`,
      `TITLE_BAR_HEIGHT`, `h_flex` from `gpui_kit::component`; `img`,
      `WindowDecorations` from `gpui_kit`). `h_flex` and `img` have no other
      use in `main.rs` today.
    - `content_width()` and the `gpui_kit::component::window_paddings`
      subtractions (in `menu_max` and in the strip's two `on_mouse_down`
      handlers) were also added by #1100, for the client-side frame on
      Linux. They are not dead after the revert: `gpui_kit::open_window`
      still wraps the view in `Root`, whose `window_border()` pads the
      content whenever the window ends up with client-side decorations (a
      Wayland compositor without server-side decorations), and
      `window_paddings` is zero under server-side decorations and on
      Windows / macOS. Keep them (the user's choice), and reword their
      comments and the doc rule so they no longer name the removed
      `WindowDecorations::Client` request.
    - `crates/native/src/icons.rs`: remove the items listed above.
      `crates/app/icons/64x64.png` itself stays (the Tauri app's icon set);
      only the `include_bytes!` goes.
    - `crates/native/src/theme.rs`: `map` pinning `colors.title_bar` /
      `colors.title_bar_border` was not added by #1100 (it came with #1097 /
      #1102 and was already in the tree before #1100) and its tests assert
      the values, so keep the code and tests (the user's choice). Reword only
      the comments that say the app draws a `TitleBar` (the `// TitleBar's
      surface and its border, as the panes'.` line and `Palette::popover`'s
      doc "the status and title bars").
    - `CLAUDE.md`: three places in the `crates/native` paragraph, namely
      "its window opens with `app_id` `IDENTIFIER` under gpui-component's
      `TitleBar` (the app icon and "Handpic") in place of the OS title bar"
      (back to "its window opens with `app_id` `IDENTIFIER`"), "`title_bar`
      and `title_bar_border` included" in the `src/theme.rs` description,
      and "with the title bar's `crates/app/icons/64x64.png` for `img(..)`"
      in the `src/icons.rs` description.
    - `docs/agents/native-app.md`: the section at "### The title bar is
      gpui-component's `TitleBar` with the icon and "Handpic" (Inferred)"
      (around lines 2248-2298), the status row sentence (around line 2236),
      and the theming paragraph (around lines 112-116). The mention of "the
      title bar" in the app icon section (around line 375) is about the OS
      bar showing the exe's icon and is correct again as written. Keep in
      the new short section the facts worth not relearning: `TitleBar` was
      tried in #1100 and reverted at the user's request, and
      `title: Some("Handpic")` is what the taskbar / Alt+Tab read.
    - Verify during implementation: after removing the bar, that the menus'
      height cap (`menu_max`) equals the pre-#1100 `viewport height - 40 px`
      on Windows (paddings are zero there).
    - Commit as a Conventional Commit, e.g.
      `refactor(native): restore the OS title bar`.

## Trade-offs and risks

- **`content_width` and the `window_paddings` subtractions.** Decided: keep
  them, drop only `TITLE_BAR_HEIGHT`. They cost nothing on Windows / macOS
  (zero paddings) and stay correct on a Linux session where gpui falls back
  to client-side decorations and `Root` draws its frame.
- **Linux without server-side decorations.** Removing
  `window_decorations: Some(WindowDecorations::Client)` lets gpui ask for
  server-side decorations again. On X11 and on Wayland compositors that
  provide them (KDE) the window manager's bar returns. On a Wayland
  compositor that does not (GNOME), the window has no title bar and no
  window controls at all, which is the pre-#1100 state. Decided: accept
  this. Not verified on any Linux desktop.
- **Theme `title_bar` / `title_bar_border` in `theme.rs`'s `map`.** Decided:
  keep, since they predate #1100 and the fields exist in gpui-component's
  `ThemeColor` regardless.
- **`todo.md` is left to the wrap-up.** Its section "Native: real-device
  checks of the title bar (native-title-bar)" becomes obsolete, and the
  native-themes items that list "the `TitleBar`" among the surfaces to check
  need that word dropped. The `develop` skill's `todo-curator` owns that
  file, so the step does not edit it; the wrap-up must remove or rewrite
  those items and add the manual Windows check if it was not done.
- **Saved window geometry.** A `windowBounds` saved while the app drew its
  own bar may restore shifted or resized by about the caption's height once
  on the first launch under the OS bar. It should not drift from launch to
  launch after that; not measured.
- **The OS bar does not follow the app's theme.** With a shipped theme or
  Light / Dark chosen in Appearance, the OS title bar keeps the color the OS
  (and gpui's window appearance handling) gives it, not the theme's. This is
  the pre-#1100 behavior and what "OS default title bar" means; noted so it
  is not reported as a regression.
- **Single step.** The code removal and the doc updates cannot be reviewed
  or merged apart (docs must match code in the same PR), so this plan has
  one step.

## Progress
