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

# Native settings as a full-window view on gpui-component's `Settings`

## Purpose

The GPUI app's settings (`crates/native/src/settingsview.rs`) open as a
gpui-component `Dialog` holding a hand-made vertical tab list beside
gpui-component controls. The user asked for the settings screen to use
gpui-component's `Settings` component
(<https://gpui-kit.com/component/settings/>) and chose to show it as a
full-window view rather than inside the dialog. Once done, `ctrl+,`
(`meta+,` on macOS) switches the main window to a settings screen whose four
sections (Sidecar, Culling, Keyboard Shortcuts, Import) are `SettingPage`s of
one `Settings` element, and Escape or the close button switches back. Every
setting keeps reading and writing as before.

## Steps

- [x] Step 1: Show the settings as a full-window layer instead of a dialog
  - Done when:
    - `ctrl+,` (`meta+,` on macOS) shows the settings over the whole window,
      opaque, with the current title row, hand-drawn tab list, controls and
      status line; Escape (cancelling a capture first, committing a focused
      field first) and the `×` switch back to the main view, which has the
      keyboard again (paging keys work at once, the tree's keyboard flag as
      it was).
    - `SettingsView` no longer opens a `Dialog`: its `modal::Modal` field,
      the dialog builder and the press-outside close are gone, and
      `modal::shade` is removed. The trash and folder dialogs are untouched.
    - While the settings are shown: no key reaches the window's keymap; text
      can be typed into the label name and pattern fields; Tab and Shift+Tab
      cycle through the settings' controls only and wrap; `+` on a shortcut
      row captures the next key, `tab` and `shift+tab` included; pointer
      presses, hovers and wheel scrolls do not reach the panes underneath.
      These are checked by hand on Windows and recorded in `learnings.md`.
    - Every `settingsview::Change` is emitted and applied as before; the
      `is_open` guards in `main.rs` and `folderops.rs` still hold.
    - `docs/agents/native-app.md` and `CLAUDE.md` describe the new hosting.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/main.rs`, `crates/native/src/settingsview.rs`,
      `crates/native/src/modal.rs`, `docs/agents/native-app.md`, `CLAUDE.md`.
      `crates/native/src/settingsmodal.rs` and `crates/native/src/folderops.rs`
      should not need to change.
    - Where it is drawn: `Root::new` takes one view, so the settings are
      drawn by `Riffle::render`. Wrap the current root `div` (the one with
      `key_context(KEY_CONTEXT)`, `track_focus`, `on_key_down`, `on_drop`) in
      an outer full-size `div` and add the `settings_view` entity as a second
      child while `is_open()`: an absolute, full-size, opaque, `occlude()`d
      layer. The `modal::shade` child goes.
    - Keep the main layout rendered underneath, not replaced. The preview
      canvas's paint takes the pending page and releases the scan pause
      (`hold.rs`'s `PaintHold`), and a re-listing or an MCP tool call can
      change the current file while the settings are shown; a layout that is
      not painted could leave the scan paused.
    - Why a sibling of the `Handpic` `div`, not a child: it reproduces the
      routing the dialog had by being in `Root`'s layer.
      - Keys with the focus in the settings do not bubble to
        `Riffle::key_down` (which also still returns at once while
        `is_open()`), so the window keymap cannot fire.
      - The `NoAction` mask of `tab` / `shift-tab` in the `Handpic` context
        does not apply, so `Root`'s `Tab` / `TabPrev` still run, and
        `CAPTURE_CONTEXT`, `capture_keys`, `keymap_keys` and the test
        `tab_and_shift_tab_reach_the_keymap_only_inside_the_view` stay as
        they are.
      - The box's `on_key_down` and `settingsmodal::stops` stay as they are,
        so typing in a field works as it does now.
    - The focus trap: the dialog provided it. Put the box under
      `gpui_component::FocusTrapElement::focus_trap(id, &self.focus)`
      (re-exported from `gpui-base`; `Root`'s `Tab` handler walks
      `active_focus_trap` and wraps inside it), so Tab does not reach a tab
      stop of the main view underneath. Verify the cycle by hand.
    - Open: `SettingsView::open` keeps reading the values from `Settings`
      and, in place of the dialog builder's first-run focus, focuses the
      selected tab's handle with `window.focus`. `main.rs`'s `OPEN_KEY`
      branch (closing the menus, refusing while `folderops.busy()`) is
      unchanged. Check that the layer is above anything `deferred` the main
      view may still draw (the sort / filter menus are closed on open; the
      right-click menu closes on any key).
    - Close: `close` closes the model and emits `Change::Closed`; the
      subscriber in `main.rs` (it has the `Window`) focuses `self.focus` and
      notifies. Nothing restores the focus for the view any more, since the
      dialog did that.
    - `render`: the box fills the window (`size_full`) on the popover
      surface instead of the dialog's 860 px by 720 px; `Empty` while
      closed stays. Drop the viewport-size math and `on_mouse_down_out`.
    - Guards: keep every `settings_view.read(cx).is_open()` check
      (`key_down`, `confirm_trash`, `folderops.rs`'s `start_sidecars`,
      `start_sequence`, `start_import`, `start_date_folder`); their meaning
      is unchanged.
    - To verify during implementation: that the preview canvas's and the
      strip's pointer handling respects the occluding layer (no paging,
      selection, tooltip or wheel scroll through it), and what a folder
      dropped on the window does while the settings are shown (the root's
      `on_drop` is under the layer); record the result.
    - Documentation: `native-app.md`'s "The settings modal is its own
      entity, opened on `ctrl+,`" (hosting, the closes, the key routing, the
      Tab rule), "`Root` wraps the window's content" and the trash dialog's
      section where they name the settings as a component dialog, and
      `modal.rs`'s module comment; `CLAUDE.md`'s `src/settingsview.rs` and
      `src/modal.rs` (`shade`) sentences.
- [x] Step 2: Draw the settings' body with gpui-component's `Settings`
  - Done when:
    - The body under the title row is one
      `gpui_component::setting::Settings` element with four pages in the
      order Sidecar, Culling, Keyboard Shortcuts, Import, holding the same
      controls: the three format radios and, while the format writes XMP,
      the note, the five label name fields, the languages and `Reset`; the
      auto-advance toggle; one row per action with its key chips, `×`, `+` /
      `Press a key...`, `Reset`, and `Reset all`; the Import note, the
      read-only root with `Choose…` and its note, the pattern field, the
      example line, the example pattern buttons and the specifier table.
    - Each change still goes out as the same `settingsview::Change`: label
      names and the import pattern are committed on Enter or blur only
      (Escape committing and closing), an invalid pattern is refused with
      its reason, `APP_KEYS` and the backend's refusals show on the status
      line, `tab` / `shift+tab` can be captured.
    - Text can be typed into the component's search field as well as the
      label name and pattern fields (checked by hand on Windows).
    - The hand-drawn tab list and the model state only it used are removed;
      the remaining `settingsmodal.rs` tests pass unchanged.
    - `docs/agents/native-app.md` and `CLAUDE.md` describe the new body.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The user has looked at the result (the visual check is the user's).
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged: the box is a full-window layer with its own
      focus handle, focus trap and `on_key_down`. Only the middle child
      (`tab_list` + `content`) becomes `Settings::new(..)`; the title row
      with `×` and the status line stay. `Settings` renders an `h_resizable`
      at full size, so its parent needs a definite height (the
      `flex_1().min_h_0()` slot).
    - No dependency change. `gpui-component` `=0.9.1` already has
      `gpui_component::setting::{Settings, SettingPage, SettingGroup,
      SettingItem, SettingField, RenderOptions}`.
    - Files: `crates/native/src/settingsview.rs`,
      `crates/native/src/settingsmodal.rs`, possibly
      `crates/native/src/theme.rs` (and its test),
      `docs/agents/native-app.md`, `CLAUDE.md`.
    - What `Settings` is at 0.9.1 (read from the registry source,
      `src/setting/`):
      - A `RenderOnce` element. The selected page and the search input live
        in `window.use_keyed_state` under the element's id, in a
        `SettingsState` that is `pub(super)`: the view can set
        `default_selected_index` but can neither read nor set the selected
        page afterwards. The state is created anew each time the settings
        are shown again.
      - The sidebar always has a search `Input`; there is no option to hide
        it. Its Escape clears a non-empty query and otherwise propagates.
      - A page is a header (title, optional description, a reset button only
        when a field has a `default_value` / `on_reset`) over a `list` of
        groups, each a `GroupBox`, with the theme's scrollbar.
      - `SettingItem::new(title, field)` lays the title left and the field
        right (stacked when the page is 480 px wide or less).
        `SettingItem::render(..)` is a full-width custom element; it is
        hidden while the search query is non-empty unless it has
        `.keywords(..)`.
      - Built-in fields: `switch`, `checkbox`, `input`, `dropdown`,
        `number_input`; custom ones through `SettingField::render` /
        `SettingField::element`. There is no radio field.
      - `SettingField::input` calls `set_value` on every
        `InputEvent::Change` (each keystroke) and owns its `InputState`.
      - The item and field closures are `Fn(.., &mut Window, &mut App)`, not
        `Context<SettingsView>`: build the handlers from the view's `Entity`
        (`entity.update(cx, ..)` or `window.listener_for(&entity, ..)`) and
        read the shown values with `entity.read(cx)`. Verify that the
        closures run outside the view's own update (no double lease).
      - `Settings`, `Sidebar`, `GroupBox` and `resizable` register no
        `KeyBinding`, so the bindings table in `native-app.md` gains no row.
        The sidebar's menu items have no focus handle and are not tab stops.
    - Field mapping, keeping the existing view methods as the handlers
      (`flip_auto_advance`, `choose_format`, `commit_name_fields`,
      `reset_names`, `capture`, `edit`, `choose_root`, `commit_pattern`,
      `use_pattern`, `leave_field`):
      - Culling: the auto-advance item can be the built-in
        `SettingField::checkbox` (or `switch`).
      - Sidecar format: a custom field holding the three `Radio`s
        (`FORMATS`), since the component has none.
      - Label names and the import pattern: custom fields over the view's
        own `InputState`s (`name_fields`, `pattern_field`) with the existing
        Enter / blur commit. Do not use `SettingField::input`: a write per
        keystroke would run `switch_names` (writer drain, `reset_sidecars`,
        re-list) on every character and would store half-typed patterns.
      - Keyboard Shortcuts: one item per `Keymap::bindings()` entry, titled
        with `shortcut_label`, the chips / `+` / `Reset` as its custom
        field, so the search matches action names. `Reset all` stays a
        button (a custom item or the group's footer).
      - Import: the note, root row, example line, example buttons and
        specifier table as custom items or the page / group description.
      - The label names block stays conditional on `writes_xmp(format)`;
        the pages are rebuilt on every render.
    - Model (`settingsmodal.rs`): keep `Tab`, `TABS` and `Tab::label` as
      the page order and titles, and everything else pure (`format_change`,
      `label_*`, `pattern_change`, `edit`, `APP_KEYS`, `ModalKey`, `key`,
      `stops`, the texts). Remove what only the hand-drawn tab list used:
      `SettingsModal::tab`, `select`, `next_tab`, `Tab::at` if unused, and
      their tests. In the view, remove `tabs: Vec<FocusHandle>`,
      `tab_list`, `content`, `select` and the `ModalKey::Native` arrow-key
      branch; `open` focuses the box's own handle instead of a tab's.
    - To verify during implementation:
      - Typing in the search field. `key_down` stops every key unless
        `editing` is true, and `editing` only knows the view's own fields;
        on Windows a stopped key down is never translated into a character.
        The search input is private to `Settings`. Since Step 1 keeps the
        settings outside the app view's key dispatch path, decide how the
        box lets those keys through (for example stopping only the keys the
        modal takes) and check on Windows that typing works in all three
        kinds of field.
      - Escape: with a non-empty search query it clears the query;
        otherwise it reaches the box and closes, committing a focused name
        or pattern field first.
      - Tab walks the search field and the page's controls inside the focus
        trap; the capture of `tab` / `shift+tab` still works.
      - A page switch during a capture: the view cannot observe it, so
        `SettingsModal::select`'s cancel has no trigger. Decide the
        replacement (for example cancelling the capture on any pointer
        press inside the box before the pressed control's own click runs).
      - Colors: `Sidebar` paints `theme.tokens.sidebar` (`#0a0a0a` in
        `Default Dark`) and uses the `sidebar_*` and `group_box*` colors,
        none of which `theme.rs` maps. Decide between
        `Settings::sidebar_style` / `with_group_variant` at the call site
        and new entries in `theme.rs`'s `map` (then extend
        `maps_the_neutral_dark_tokens`).
      - The component's own strings (`Settings.search_placeholder`, the
        no-results text) come from its `rust_i18n` locales; check they show
        in English.
    - Documentation: `native-app.md`'s "gpui-component is initialized; ..."
      (the components drawn), the settings section (the tabs paragraph, the
      roving `tabindex` rule, the look and scrolling rules), "The keymap is
      edited in the modal and replaced whole" (the tab-switch cancel) and
      the Sidecar / Import sections where they say "tab"; record what was
      met at the component (the private selection state, the per-keystroke
      `input` field, the unfocusable sidebar) with its tag. `CLAUDE.md`'s
      `src/settingsmodal.rs` / `src/settingsview.rs` sentence.
      `docs/humans/usage.md` describes the Tauri app's settings; change it
      (and `usage.ja.md`) only if it states something about the native
      settings that this work makes false.

## Trade-offs and risks

- **Two steps instead of one.** The hosting change (focus, Tab, key routing,
  occlusion) and the component swap (sidebar, search, fields) have separate
  risks and separate hand checks, so they are split; Step 1 merges as a
  working full-window settings screen with the old tab list.
- **Sibling layer or child of the app view.** The layer is a sibling of the
  `Handpic`-context `div`, which keeps today's rules: no key bubbles to the
  window's handler, `Root`'s Tab walks a focus trap, the capture mask stays.
  Drawing it as a child inside the `Handpic` context would let
  `CAPTURE_CONTEXT` and `capture_keys` be deleted (Tab would arrive as a
  plain key), but the view would have to walk the focus on Tab itself and
  every key would bubble to `Riffle::key_down`, guarded only by its early
  return. Not taken.
- **Layer over the main layout, or replacing it.** Replacing the layout
  while the settings are shown would stop the preview canvas's paint, which
  releases the scan pause and takes the pending page; a re-listing or an MCP
  call during the settings could then stall the scan. The layer keeps the
  main view painting underneath, at the cost of drawing it unseen.
- **Leaving by pointer.** A full-window view has no outside to press, so the
  settings are left only by Escape or the `×`. `ctrl+,` while shown still
  does nothing; making it a toggle is not in the plan.
- **Dialogs over the settings.** A trash or folder dialog held by the idle
  gate opens in `Root`'s layer, above the settings, as it did above the
  settings dialog. The `is_open` guards keep new ones from starting.
- **A drop while the settings are shown.** The occluding layer should swallow
  a folder dropped on the window; Step 1 checks and records what happens.
- **Wide content in Step 1.** The hand-drawn layout was made for an 860 px
  box; on a wide window it may look sparse until Step 2. A maximum content
  width is optional there.
- **The page is no longer remembered across opens (Step 2).** The selected
  page lives in the component's private element state, dropped when the
  settings are hidden, and the view cannot read it. Every open starts on
  Sidecar.
- **No keyboard page switching (Step 2).** The sidebar entries are not
  focusable at 0.9.1, so ArrowUp / ArrowDown / Home / End on the tab list
  and the roving tab stop go away; pages are switched with the pointer.
- **A search field is added (Step 2).** It cannot be turned off, and it is
  the one place where the box's blanket key stop can break typing on
  Windows. Custom full-width items disappear from search results unless
  given keywords.
- **Capture cancel (Step 2).** A tab switch used to cancel a pending
  shortcut capture; the substitute rule differs slightly from the Tauri
  behavior.
- **The look changes (Step 2).** Group boxes, the page header, label-left /
  control-right rows, 14 px labels and the sidebar's colors replace the
  13 px hand-laid layout. This is the user's visual check.
- **Tests removed with their behavior (Step 2).** The `next_tab` tests and
  `switching_tabs_cancels_a_capture` go with the code they cover. The view
  has no automated test (clicks and its keys cannot be driven by `--auto`),
  so the behavior checks in both steps are by hand.

## Progress

- (2026-11-16) Step 1 complete
- (2026-11-16) Step 2 complete
