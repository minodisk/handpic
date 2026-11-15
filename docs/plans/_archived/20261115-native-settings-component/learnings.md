# Learnings: native-settings-component

## Step 1: the settings as a full-window layer

- `FocusTrapElement::focus_trap(id, handle)` calls `track_focus(handle)` on
  the wrapped element itself (`gpui-base` 0.9.1 `focus_trap.rs`), so the
  box's own `track_focus(&self.focus)` is redundant but harmless (same
  handle). The trap stays registered in `FocusTrapManager` while the view
  lives; `active_focus_trap` only picks it while it contains the focus, so a
  closed settings view does not capture the main view's Tab.
- The layer is a plain (not `deferred`) `absolute` child painted after the
  `Handpic` `div`, so its `occlude()` blocks every hitbox of the main layout
  (`on_mouse_down`, hovers, tooltips, `on_scroll`, `on_drop`). Deferred
  layers of the main view still paint above it; the menus are closed on
  open and the right-click menu closes on any key, `ctrl+,` included.
- The tree pane's `on_mouse_down_out` ignores occlusion, so a press inside
  the settings clears `tree_keys`, as a press inside the old dialog did.
  Kept as it was.
- A drop on the window while the settings are shown should be ignored: the
  root's `on_drop` fires only on its hovered hitbox, which the layer
  occludes. Not checked by hand (see the pending checks below).
- The hand checks below could not be run from this agent session (no way to
  drive the GUI's keys, pointer and an OS drag here); the step's checkbox was
  ticked on the automated criteria.

## Step 2: the body drawn with gpui-component's `Settings`

Decisions taken on the plan's "To verify during implementation" points:

- Typing in the search field: the box's key rule (`settingsmodal::stops`)
  is unchanged, but its `editing` argument is now "an `Input` key context is
  on the focused path" (`window.context_stack()`, the context gpui-base's
  `InputState` sets, `"Input"` at 0.9.1), not "one of the view's own fields
  is focused". That covers the component's private search field without
  reaching into it, and keeps the blanket stop for every other focus (the
  box, a button), so the `stops` tests are unchanged. The view's own
  `editing` stays for the commit on Escape and `leave_field`.
- Escape: left to the component. With a non-empty query, the sidebar
  header's `on_action(Escape)` clears it and does not propagate; with it
  empty the action propagates and the box closes (committing a focused name
  or pattern field first, as before). Not checked by hand.
- Capture cancel substitute: a capture-phase `capture_any_mouse_down` on
  the box cancels a pending capture on any pointer press inside it, before
  the pressed control's click runs (a `+` on another row then captures for
  that row). Pressing a sidebar page switches the page and cancels.
- Colors: new `theme.rs` entries rather than `sidebar_style` at the call
  site, since the sidebar's menu items read `sidebar_accent` /
  `sidebar_accent_foreground` from the theme and a style refinement could
  only set the sidebar's own background. `sidebar` is `PANE_COLOR` (the
  box's popover surface), the accent `ACCENT_COLOR`. The groups use the
  default `GroupBoxVariant::Normal`, which paints no surface, so no
  `group_box*` mapping was needed.
- The component's strings: `rust_i18n`'s locale is never set by the app or
  by `gpui_component::init` (only `gpui_component::set_locale` sets it), so
  the fallback `en` is used: "Search...", "No matching settings".
- The definite height: the slot is `relative` `flex_1` `min_h_0` with an
  `absolute` `inset_0` child holding `Settings`, rather than trusting a
  percentage height under a flex item.
- The closures: `SettingItem` / `SettingField::render` closures are built
  from snapshots of the shown values plus the view's `Entity`, handlers
  through `window.listener_for`. GPUI's view element calls `render` inside
  `entity.update` and only lays the returned element out after it returns
  (`gpui-pre` 0.3.12 `view.rs`), and `Settings` is a `RenderOnce` whose
  page `list` renders items during layout, so no closure runs under the
  view's lease.
- Tests removed with their behavior: `switching_tabs_cancels_a_capture`,
  `starts_closed_on_the_first_tab` and
  `closes_and_can_open_again_on_the_same_tab` (all read
  `SettingsModal::tab`), and the four `next_tab` tests. The remaining
  `settingsmodal.rs` tests are unchanged and pass. `Tab::at` went too (its
  only users were the tab handles).
- The Shortcuts page is one group of about 40 items, a single row of the
  component's virtualized `list`, so all of them render every frame, as the
  old hand-drawn list did.
- The hand checks below could not be run from this agent session (the GUI's
  keys, pointer and look cannot be driven here); the step's checkbox was
  ticked on the automated criteria.

## Deferred issues (todo candidates)

- Pending manual check (Windows, Step 1 of
  `docs/plans/20261115-native-settings-component/plan.md`, files
  `crates/native/src/settingsview.rs`, `crates/native/src/main.rs`). Run the
  native app with a test `--settings` store and an open folder, then:
  1. `ctrl+,` shows the settings over the whole window, opaque, with the
     title row, the tab list, the controls and the status line.
  2. Typing works in a label name field (Sidecar, XMP format) and in the
     Import pattern field; Enter and blur commit.
  3. Tab / Shift+Tab cycle only through the settings' controls and wrap
     (no stop in the tree, strip or preview underneath).
  4. On Keyboard Shortcuts, `+` captures the next key, `tab` and
     `shift+tab` included.
  5. Pointer presses, hovers (no strip tooltip) and wheel scrolls over the
     settings do not reach the panes underneath.
  6. A folder dropped on the window while the settings are shown opens
     nothing (record what actually happens).
  7. Escape (cancelling a capture first, committing a focused field first)
     and the `×` return to the main view, where Left / Right page at once.
  Expected: all as described. The step's checkbox was ticked on the
  automated criteria (`mise run ci:local`).
- Pending manual check (Windows, Step 2 of
  `docs/plans/20261115-native-settings-component/plan.md`, files
  `crates/native/src/settingsview.rs`, `crates/native/src/theme.rs`). Run
  the native app with a test `--settings` store and an open folder, press
  `ctrl+,`, then:
  1. The body is a sidebar (search field, Sidecar, Culling, Keyboard
     Shortcuts, Import) beside the selected page, on the box's dark surface,
     the selected page lit `#262626`; the search placeholder reads
     "Search...".
  2. Typing works in the search field, the label name fields (Sidecar, XMP
     format) and the Import pattern field; a name or pattern commits only on
     Enter or blur, an invalid pattern shows its reason in red.
  3. Escape in the search field with text clears it; pressed again it
     closes the settings. Escape in a name or pattern field commits and
     closes.
  4. On Keyboard Shortcuts, `+` captures the next key, `tab` and
     `shift+tab` included; `home`, `end`, `q` and a key bound elsewhere show
     their refusal on the status line; a press on the sidebar or another
     row's `+` cancels a pending capture.
  5. Tab / Shift+Tab walk the search field and the page's controls and
     wrap, never reaching the main view.
  6. Searching an action name (for example "burst") lists only its rows;
     clearing the search brings every item back.
  7. The user's visual check of the new look (group headers, label-left /
     control-right rows, the sidebar's colors).
  Expected: all as described. The step's checkbox was ticked on the
  automated criteria (`mise run ci:local`).
