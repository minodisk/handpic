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

# Always-visible scrollbars in the native app

## Purpose

In the GPUI app (`crates/native`) every gpui-component scrollbar auto-hides:
it shows only for about 2 s after a scroll and takes a press only while
shown, so a bar cannot be seen or grabbed at rest. The user wants every
scrollbar always visible: the folder tree's vertical bar, the strip's
horizontal bar (`StripBar`), the meta pane's vertical bar and the dialog
bodies' `overflow_y_scrollbar`. The user chose "all of them", so the theme's
mode changes rather than each bar's. This reverses the 2026-11-14 decision
recorded in `../_archived/20261115-native-scrollbars/plan.md` (Trade-offs 1).

## Steps

- [x] Step 1: Set the theme's scrollbar mode to `Always` on every `theme::apply`
  - Done when:
    - After `theme::apply`, the global `Theme`'s `scrollbar_mode` is
      `ScrollbarMode::Always` on every path: startup, a choice on the
      Appearance page, and an OS appearance change in System mode; for
      Neutral and for a vendored theme, in both modes.
    - A unit test in `crates/native/src/theme.rs` asserts the mode is
      `Always` for a Neutral and for a vendored theme (see the approach for
      its form).
    - `docs/agents/native-app.md` and `CLAUDE.md` describe the new behaviour
      (English), with no statement left that the bars are in
      `ScrollbarMode::Scrolling` or take a press only while shown.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (the `native` job runs `cargo clippy --all-targets -- -D warnings` and
      `cargo test` in `crates/native`).
  - Implementation approach (as far as it is known; omit if unknown):
    - Only `crates/native` and the docs change. The Tauri app is frozen.
    - The field is `Theme::scrollbar_mode`, of type `ScrollbarMode`
      (`gpui_kit::component::scroll::ScrollbarMode`, variants `Scrolling`,
      `Hover`, `Always`) in gpui-component 0.9.1. `scrollbar_show` is only a
      serde alias of that field; there is no `ScrollbarShow` type.
    - Investigated at 0.9.1: `gpui_component::init` (run by
      `gpui_kit::init`) calls `Theme::sync_scrollbar_appearance` once, which
      sets `Scrolling` or `Hover` from the OS. Nothing calls it again.
      `Theme::change` and `Theme::apply_config` do not write
      `scrollbar_mode`, and the theme-file schema has no key for it; the
      vendored files under `crates/native/themes/` set only the scrollbar
      colors. `Theme::update` rebuilds the Base projection the scrollbars
      read (`sync_base`) and refreshes every window.
    - Set `theme.scrollbar_mode = ScrollbarMode::Always` inside `apply`'s
      second `Theme::update` closure, the one after `Theme::change` that
      already sets `font_size`, `radius` and `radius_lg`. That adds no extra
      `update`, and it stays correct even if a later gpui-component makes
      `apply_config` write the mode. Add a short comment saying why it is
      set here (`init` leaves the OS's mode; no theme file sets one).
    - All four `theme::apply` call sites are in `crates/native/src/main.rs`
      (around lines 735, 739, 2472 and 6688) and need no change.
    - Do not add `.scrollbar_mode` or a per-bar mode at the call sites
      (`vertical_scrollbar(&self.tree_scroll)`,
      `horizontal_scrollbar(&self.strip_bar)`, the meta pane's
      `overflow_y_scrollbar()`, the menus' `vertical_scrollbar(scroll)`):
      they all follow the theme.
    - Test: `crates/native` has no test that builds a GPUI `App`. The form
      proposed here is to move the body of that second closure into a small
      function taking `&mut Theme` (plus whether the theme is Neutral and the
      mode), and test it on a plain `Theme` value: `Theme::default()`, then
      the public `Theme::apply_config(&Rc<ThemeConfig>)` with a vendored
      config (e.g. `Catppuccin Mocha` from `SHIPPED`) or left as is for
      Neutral, then the function, then assert
      `scrollbar_mode == ScrollbarMode::Always`. Verify during
      implementation that `Theme::default()` and `apply_config` work without
      an `App`; if they do not, decide between the options in Trade-offs 3.
    - `docs/agents/native-app.md` passages to update:
      - "gpui-component is initialized; ..." (around line 72): add that
        `apply` sets the scrollbar mode.
      - The settings modal's rule "a page scrolls in the component's `list`
        ... in its default `ScrollbarMode::Scrolling`" (around line 1698).
      - The section "The strip, the tree and the meta pane draw the theme's
        scrollbar" (around line 2206): its opening paragraph (mode, shown for
        the idle time, "takes a press only while shown"), the rule "With a
        bar not shown, no listener is registered" (now only where the content
        fits), and "The strip's bar is drawn over its bottom edge ... while
        shown".
      - Keep the "Not verified by hand yet" note style for the on-screen
        checks; the agent has no screen.
    - `CLAUDE.md`: in the `src/theme.rs` description, add a clause that
      `apply` also sets the theme's scrollbar mode to always shown. Do not
      touch the rest of that paragraph.
    - Record in `learnings.md` the on-screen checks left pending: each of
      the four bars visible at rest and draggable without scrolling first;
      hidden where the content fits; the strip's bottom edge and the tree
      rows' right edge under the bar; the bar's look in Neutral dark and
      light and in one vendored theme.

## Trade-offs and risks

1. **The theme's mode is global.** Setting it also makes the settings
   modal's page lists, the `Sort ▾` / `Filter ▾` popovers, `PopupMenu` and
   `Select` lists always show their bar when they overflow, not only the
   four bars named in the request. The user approved this on 2026-11-17
   (the global route over hand-building the four bars with a per-bar mode).
2. **A bar now covers content at rest and always takes presses.** The bar
   stays an overlay (no gutter is added; track 16 px, thumb 6 px inset
   4 px). Where the content overflows:
   - The strip's bar permanently lies over the strip's bottom edge (the
     7 px `STRIP_PADDING` and possibly the bottom of the name band). A press
     there no longer selects a cell or opens the right-click menu.
   - The tree's bar lies over the right 16 px of the rows. A press there
     reaches neither the row nor the pane, so it does not give the tree the
     keyboard.
   - The meta pane's bar lies over the right edge of its values.

   The user accepted this on 2026-11-17. Padding under the bars is out of
   scope; revisit after a look on a real screen.
3. **How the test reaches `apply`.** `apply` needs a GPUI `App`, and
   `crates/native` has no test that builds one.
   - Proposed: extract the post-`change` edit into a function over
     `&mut Theme` and test it on a plain `Theme` after `apply_config`. It
     proves the vendored configs and the edit leave `Always`, but not that
     `apply` calls it.
   - Alternative: add `gpui-kit` with its `test-support` feature as a
     dev-dependency and test `apply` itself under a test `App`. It adds a
     dev feature and `Cargo.lock` churn.
   - Alternative: no new test.
4. **A gpui-kit bump can change this.** That `change` / `apply_config`
   leave `scrollbar_mode` alone is a property of 0.9.1 (pinned with `=`).
   Setting the mode after `change` in `apply` keeps it correct either way.
5. **Not verifiable by the agent on screen.** The visual checks are
   recorded as pending in `learnings.md`.

## Progress
