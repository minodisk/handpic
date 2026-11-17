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

# Native title bar settings button

## Purpose

The GPUI app's (`crates/native`) title bar, gpui-component's `TitleBar`
added by [the title bar plan](../_archived/20261116-native-title-bar/plan.md),
holds only the app icon and "Handpic", and the settings modal can only be
opened with `ctrl+,` (`meta+,` on macOS). This adds a gear button at the right
end of the bar that opens the settings, so the settings have an on-screen
entry and the bar carries something useful. The `TitleBar` itself stays (its
revert was considered and abandoned in PR #1112).

## Steps

- [x] Step 1: Add the settings button to the title bar
  - Done when:
    - A gpui-component `Button` with the Lucide `settings` icon is the
      `TitleBar`'s second child, at the right end of the bar: directly left
      of the window controls on Windows / Linux, at the bar's right edge with
      a right margin on macOS (the traffic lights are on the left there).
    - A click opens the settings modal through the same method the
      `settingsmodal::OPEN_KEY` arm of `Riffle::key_down` calls, with the
      same refusals (nothing while a Move to Trash / folder-operation dialog
      is open or a folder flow is under way, nothing while the settings are
      already open).
    - The button's tooltip is a gpui-component `Tooltip` naming the action
      ("Settings") and the key that opens it, `settingsmodal::OPEN_KEY`,
      drawn as a `Kbd`.
    - A press on the button does not move the window and does not take the
      keyboard from the view; the bar gets no focus handle of its own and the
      button is not a tab stop.
    - The button's colors come from the theme (no fixed colors).
    - Unit tests: the tooltip's text and key come from `OPEN_KEY` (per
      platform), and `icons/settings.svg` loads an SVG through `Icons`.
    - `CLAUDE.md` (the `crates/native` sentence on gpui-component's
      `TitleBar`) and the title bar section of
      [native-app.md](../../agents/native-app.md) describe the button, the
      drag rule and the reworded focus rule.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - By hand on Windows (a debug build): the gear shows at the bar's right
      end left of the controls; a click opens the settings; pressing and
      dragging on the gear does not move the window, while dragging the rest
      of the bar still does and minimize / maximize / close still work; after
      closing the settings the keyboard shortcuts work at once (paging, a
      judgment key, `ctrl+,`); the tooltip shows "Settings" and the key; the
      button looks right in Light and Dark (Neutral) and in one shipped
      theme; with the tree holding the keyboard, an open `Sort ▾` /
      `Filter ▾` or right-click menu, and an inline folder rename under way,
      a click on the gear behaves sensibly (the menu closes, the rename ends
      as a press elsewhere ends it). Anything not checked goes to
      `learnings.md` as a pending manual check, as do macOS and Linux.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/main.rs` (the `TitleBar::new().child(..)` in
      `impl Render for Riffle`, and `key_down`'s `OPEN_KEY` arms),
      `crates/native/src/settingsmodal.rs` (the tooltip's text next to
      `OPEN_KEY`, with its test), `crates/native/src/icons.rs` (test only),
      `CLAUDE.md`, `docs/agents/native-app.md`.
    - One open path: move the body of the `OPEN_KEY` arm (`menus.close()`,
      `settings_view.update(.. view.open(settings, window, cx))`,
      `refresh_index_size`, `cx.notify()`) into one `Riffle` method that both
      the key arm and the button's click call. `key_down` reaches that arm
      only after its early returns (`settings_view` open, `trash.busy() ||
      folderops.dialog_open()`, `rename_owns_keys()`) and skips it while
      `folderops.busy()`; the click does not pass through `key_down`, so the
      method (or the click handler) must apply the refusals itself. Whether a
      dialog's backdrop covers the bar is unmeasured (the title bar plan's
      learnings), so do not rely on occlusion for them.
    - The settings key is not a keymap action: `OPEN_KEY` is a fixed
      constant the keymap refuses to bind, so the tooltip reads it directly
      (`keys::keystroke_of(OPEN_KEY)` gives the `Kbd` its keystroke, as the
      right-click menus' shortcuts do). `Button::tooltip_with_action` does
      not apply: the app binds no GPUI action for the key. `Tooltip::new(..)
      .key_binding(Some(Kbd::new(..)))` exists in gpui-component 0.9.1;
      decide whether it goes through `Button`'s tooltip builder or the
      element's `.tooltip(..)` as `strip_tips` does.
    - Window drag: `TitleBar` puts its children inside a row marked
      `WindowControlArea::Drag`. On Windows gpui-pre 0.3.12 answers
      `HTCAPTION` wherever that hitbox is in the hit test, dispatches the
      resulting non-client press to GPUI first and starts the OS drag unless
      propagation was stopped; on macOS / Linux the bar's own mouse-down
      arms `start_window_move` on the next mouse move. `Button`'s mouse-down
      only calls `window.prevent_default()`. So the press must not propagate
      past the button: either `cx.stop_propagation()` on the left mouse-down
      (on the `Button`, or on a wrapping `div` so it runs after the button's
      own listeners; gpui-component's Linux `ControlIcon` is the precedent)
      or an occluding hitbox (`.occlude()`), which takes the drag hitbox out
      of the hit test. Decide between them by trying it on Windows; check
      the click still fires and the hover / tooltip still shows in the
      non-client area.
    - Focus: `Button` keeps an internal focus handle but its mouse-down
      prevents the focus change; set `.tab_stop(false)` as the status row's
      `Sort ▾` / `Filter ▾` buttons do. Do not call `track_focus` on the bar
      or the button. The capture-phase `on_mouse_down_out` listeners (the
      tree pane's keyboard hand-back, the menus' dismissal) still see the
      press when the bubble phase is stopped; confirm by hand.
    - Look: `Button::new(..).ghost()` at `.small()` or `.xsmall()` with
      `.icon(IconName::Settings)`, within the 34 px `TITLE_BAR_HEIGHT`; the
      ghost variant's colors come from the theme tokens, so check the
      Neutral mapping in `theme.rs` gives a visible hover in both palettes.
      The bar's inner row is `justify_between`, so the second child lands at
      the right end without a spacer; give it a small right margin so it
      does not touch the controls (Windows / Linux) or the window edge
      (macOS).
    - Icon: `icons/settings.svg` is in gpui-kit-assets' `default-icons.txt`,
      so the default `Assets` behind `Icons` already serves it (as
      `icons/check.svg`); no `icon_assets!` entry is needed. Add it to the
      `icons.rs` test that loads a component icon; if it fails to load, add
      `Settings` to the `icon_assets!` list instead.
    - Docs: the native-app.md section's heading and body name the bar's one
      child, and its rule "do not add [a focus handle] to the bar or to its
      children" needs rewording for a `Button` that holds an unfocused
      internal handle. Record the drag rule there with its source and what
      was seen by hand.

## Trade-offs and risks

- The tooltip's key: the settings key is the fixed
  `settingsmodal::OPEN_KEY`, not a keymap action, and cannot be remapped.
  This plan shows `OPEN_KEY` (the user's choice). Making the settings a
  remappable keymap action is a separate, larger change.
- While a folder operation is under way the key is silently ignored. The
  plan keeps the click identical (ignored), not a disabled button (the
  user's choice).
- Drag prevention: `stop_propagation` on the press keeps the button inside
  the caption area and relies on gpui-pre's non-client dispatch order;
  `.occlude()` turns the button's area into client area on Windows but also
  hides the press from everything under it. Both are read from the sources,
  not run; the step decides on Windows. If neither stops the drag, the plan
  needs revisiting (the button would have to live outside the `TitleBar`'s
  children).
- Icon choice: Lucide `settings` (the gear) rather than `settings-2`
  (sliders); both are served already.
- `docs/humans/usage.md` describes opening the settings through the Tauri
  menus and has no native title bar text, so it is left alone.
- macOS and Linux cannot be checked on this host: the right margin against
  the window edge on macOS, the drag suppression through `start_window_move`
  on both, and Linux under client-side decorations stay pending manual
  checks in `learnings.md`.

## Progress
