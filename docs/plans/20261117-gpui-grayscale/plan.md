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

# Momentary grayscale preview in the native app

## Purpose

The Tauri app shows the viewed photo in grayscale while the `grayscale` key
(default `g`) is held, to judge composition without color
([the original plan](../_archived/20260922-grayscale-preview/plan.md)). The
Tauri app is frozen and the GPUI app in `crates/native` is the one that
grows; [its zoom and peaking plan](../_archived/20261110-gpui-zoom-peaking/plan.md)
left the hold unported. The keymap entry (`("grayscale", &["g"])` in
`crates/backend/src/shortcuts.rs`) and the settings label ("Grayscale" in
`crates/native/src/settingsmodal.rs`) already exist, so today the key is
bound, listed in the settings and does nothing. After this plan, holding the
key grays the shown photo itself (the fit view and the 1:1 crop), and
releasing it restores color. It stays display-only: nothing is written or
persisted.

**Deliberate difference from the Tauri app (the user's decision):** only the
photo is grayed. The focus peaking overlay (`e`), the `f` overlay (AF mark,
face boxes, landmark dots, mesh, labels) and the performance graph (`d`) keep
their colors. In the Tauri app a CSS filter on `#canvas` also grayed peaking
and the `f` overlay; here they stay colored so they remain readable over the
gray photo.

## Steps

- [x] Step 1: Gray the preview photo while the `grayscale` action's key is
      held in `crates/native`
  - Done when:
    - Holding `g` grays the fit view's image and the 1:1 view's placeholder
      and crop. The peaking overlay, the `f` overlay, the performance graph,
      the strip, the panes and the empty-state text keep their colors.
    - Color comes back on the release of the held key, on the release of any
      modifier, when the window is deactivated, and when the focus leaves the
      app view's own focus handle (the settings layer, a dialog, a
      right-click menu or a text input taking it).
    - Auto-repeat while held neither flickers nor redraws: a repeated
      key-down that changes nothing does not call `cx.notify()`.
    - A rebound key works, e.g. `ctrl+g`: the hold ends on the release of
      either `g` or Ctrl, whichever comes first.
    - While the folder tree has the keyboard the key is swallowed as in the
      Tauri app (`grayscale` stays out of `TREE_PASSTHROUGH`); the same holds
      while the settings, a dialog or an inline rename owns the keys
      (`key_down`'s existing early returns).
    - Paging, `z`, `f`, `e` and `d` behave as before whether or not the key
      is held; no texture is re-uploaded for the gray frame (the flag is per
      sprite).
    - The hold's state machine is unit-tested (press, repeated press, key-up
      of the same key, key-up of another key, a modifier going down, a
      modifier going up, a forced release).
    - `CLAUDE.md`'s `crates/native` description names the new module, and
      `docs/agents/native-app.md` has a section for the hold.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (the `native` job on all three platforms).
    - Checked by hand in the running app on Windows (there is no GUI
      automation), and the result recorded in `learnings.md`: hold / release
      `g` in the fit view and in 1:1, with `f` and `e` on (they stay
      colored); hold through auto-repeat; rebind to `ctrl+g` and release Ctrl
      first, then `g` first; hold `g` and Alt+Tab away and back; hold `g` and
      open the settings (`ctrl+,`) or a right-click menu, then come back. If
      the implementer cannot run the GUI, record the check as pending on the
      user.
  - Implementation approach:
    - Rendering: GPUI's image sprite has a grayscale flag. In the pinned
      `gpui-pre` 0.3.12, `Window::paint_image(bounds, image_bounds,
      corner_radii, data, frame_index, grayscale: bool)` passes it to the
      sprite shader, which on all three backends replaces the color by its
      luminance and keeps the alpha. Every preview-path `paint_image` call
      passes `false` today. Pass the hold's state instead in:
      - the fit image in `main.rs`'s preview canvas closure;
      - `zoom::Paint::paint` (`zoom.rs`), its `draw` closure for the
        placeholder and the crop, via a new field on `zoom::Paint` filled in
        `Riffle::zoom_paint` or a parameter.
      `peaking::Paint::paint`, `overlay::paint`, `perfgraph::paint` and the
      strip's `paint_image` calls are unchanged. No CPU conversion, no second
      `RenderImage`, no change to `retired` or the peaking keys.
    - State: a new pure module `crates/native/src/grayscale.rs` (add
      `mod grayscale;` to `main.rs`) holding the hold, e.g. `Hold` with the
      held key's name (`Keystroke.key`, lower-cased) and the modifiers last
      seen, with methods that each return whether the state changed:
      - press (from `key_down`);
      - key-up, which ends the hold when the released keystroke's `key`
        equals the held one;
      - modifiers-changed, which ends it when a modifier held before is no
        longer held and otherwise only remembers the new modifiers;
      - release, the forced end;
      - `is_on`.
      Unit tests live there too. `Riffle` gets one field; `render` copies
      `is_on()` into the canvas closure next to `fit_peaking` and
      `zoom_paint`.
    - Key-down: add a `(_, Some("grayscale"))` arm to `Riffle::key_down`'s
      action `match`, before the catch-all `(_, Some(action))` arm (see
      "Every keystroke goes through the keymap" in
      `docs/agents/native-app.md`). It sits after the settings / dialog /
      rename / tree gates, so those swallow the key with no further code.
      Call `cx.notify()` only when the press changed the state.
    - Key-up and modifiers: the main `div` in `render` has only
      `on_key_down` today; add `.on_key_up(..)` and
      `.on_modifiers_changed(..)` listeners beside it that feed the hold and
      notify on a change. GPUI has no `event.code`; the physical key is
      approximated by `Keystroke.key`. `ModifiersChangedEvent` carries only
      the new state, hence the remembered modifiers (a modifier going up ends
      the hold, a modifier going down does not, as in the Tauri app).
    - Window deactivation: extend the existing
      `cx.observe_window_activation` closure in `main.rs` to release the hold
      when `!window.is_window_active()`.
    - Focus leaving the view: GPUI dispatches key-up and modifiers-changed
      only along the focused node's dispatch path, so with the focus in the
      settings layer, a dialog, a `PopupMenu` or an `Input`, the main `div`
      never sees the release and the hold would stick. Register
      `cx.on_blur(&self.focus, window, ..)` releasing the hold. Verify it
      fires for those focus moves and not on an ordinary click inside the
      view; if it does not fire for one of them, release the hold where the
      view opens the settings, a dialog or a menu instead.
    - Docs: in `CLAUDE.md`, add `src/grayscale.rs` to the `crates/native`
      description, in the file's style. In `docs/agents/native-app.md`, add
      one section under `## GPUI` recording the sprite flag, that only the
      photo is grayed (peaking and the `f` overlay stay colored by choice),
      that key-up and modifiers-changed reach only the focus path (hence the
      blur release), and how the key is matched. `README.md`, `README.ja.md`
      and `docs/humans/usage*.md` need no change.
    - Out of scope: Compare mode, a `View` menu item, any setting or
      persistence, and changes to `crates/app` or `crates/backend`.

## Trade-offs and risks

- **GPU sprite flag vs a CPU grayscale copy.** The flag costs nothing per
  frame and needs no second texture. Risk: it is an API of the pinned
  `gpui-pre` snapshot; a `gpui-kit` bump re-checks that `paint_image` still
  has it.
- **Overlays stay colored**, unlike the Tauri app, by the user's decision:
  the marks and peaking stay readable over the gray photo.
- **Key identity without `event.code`.** `Keystroke.key` is compared between
  key-down and key-up. For a shifted punctuation binding on Windows the
  key-up after Shift went up first may name another key; the modifier
  release has already ended the hold by then. No default key is affected.
- **Release on focus leaving the view** goes beyond the Tauri app's three
  release paths; without it the hold can stick in the native app only.
- **Manual verification only** for the visual result and the hold on this
  Windows host; macOS and Linux behaviour is read from the sources.

## Progress
