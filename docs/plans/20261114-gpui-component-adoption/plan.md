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

# Adopting gpui-component in the GPUI app

## Purpose

`crates/native` has had `gpui-component` `=0.9.1` as a dependency since
`8842e32a` (#1044), with `gpui` pinned to `gpui-pre` `=0.3.12`, but no source
file uses it and nothing calls `gpui_component::init`. Every control of the
native app is hand-built from `div`s: the text input (`textinput.rs`, a port
of GPUI's example), the settings modal's buttons, checkboxes, radios and
scrolling lists, the Sort / Filter menus, the right-click menus, and the
trash, folder-operation and settings dialogs.

This plan starts the adoption, incrementally and behavior-preserving: call
`init` and map its `Theme` to the app's current look (shadcn/ui Neutral
dark), replace `textinput.rs` with the component's `Input`, replace the
settings modal's controls, give the menus the theme's look, mount `Root`,
and turn the three dialogs (trash / delete, the folder operations', the
settings modal) into the component's `Dialog`. Once it is done the app's
controls and dialogs come from one library and one theme, and the port of
GPUI's input example is gone.

Out of scope: the canvas-drawn parts (the thumbnail strip, the preview, the
`f` / `e` / `d` overlays), the folder tree (`tree.rs`, `treekeys.rs`), and
any new feature. Keyboard, focus (including the tree taking and handing back
the keyboard), IME, outside-click close, the keymap, `--auto` and the
existing tests behave as before.

### What the investigation found (2026-11-14, `origin/main` at `6f1e7936`)

Verified in the cargo registry sources `gpui-component-0.9.1`,
`gpui-base-0.9.1` (gpui-component re-exports most of its engine from it) and
`gpui-pre-0.3.12`.

- **`gpui_component::init(cx: &mut App)`** runs, in order: `theme::init`,
  `global_state::init`, `inspector::init` (debug builds or the `inspector`
  feature only), `root::init`, `gpui_base::init`, `input::init`,
  `diff::init`, `date_picker::init`, `dock::init`, `sheet::init`,
  `list::init`, `command::init`, `carousel::init`, `notification::init`,
  `popover::init`, `questionnaire::init`, `menu::init`, `table::init`,
  `tooltip::init`.
- **Key bindings.** Every binding `init` registers is scoped to a key
  context, except the debug inspector's: `ctrl-shift-i` (`cmd-alt-i` on
  macOS) is bound with no context to `ToggleInspector` and handled by a
  global `cx.on_action`, so in debug builds that keystroke is consumed
  app-wide. No default of `crates/backend/src/shortcuts.rs` and none of
  `settingsmodal::APP_KEYS` (`home`, `end`, `q`) uses it. No binding is a
  multi-keystroke sequence, so none adds a pending-key delay. GPUI
  dispatches a matching binding's action before any key-down listener, and
  the listeners run only if no handler took the action. The contexts that
  matter here:
  - `Root` (`gpui-base` `root.rs`): `tab` -> `Tab`, `shift-tab` ->
    `TabPrev`, `ctrl-c` / `cmd-c` -> `Copy`. `Root`'s handlers for `Tab` /
    `TabPrev` move the focus (inside the active focus trap when a dialog is
    open) and do not propagate; `Copy` propagates when no text is selected.
    The `Tab` / `TabPrev` action types are not public (`gpui_base::root` is
    a private module). **This collides with the app's keymap** once `Root`
    is mounted: the default `toggleSides` is `tab`, and a user may bind
    `shift+tab`.
  - `Input` (`gpui-base` `input/base/state.rs`, 83 bindings): the editing
    keys, `enter` / `shift-enter` / `ctrl-enter` (`cmd-enter`) -> `Enter`,
    `escape` -> `Escape`, `tab` / `shift-tab` -> `IndentInline` /
    `OutdentInline`, the arrows, Home / End, word and line movement, undo /
    redo, the clipboard keys per platform. On a single-line input `Enter`
    emits `InputEvent::PressEnter` and propagates, `Escape` propagates
    (unless `clean_on_escape`), and the indent actions propagate.
  - `Dialog` (`gpui-base` `dialog.rs`): `escape` -> `Cancel`, `enter` ->
    `Confirm`, handled on the dialog's own node (`on_cancel` / `on_ok`,
    each returning `bool`, `false` vetoing the close). The context is set
    only `when(self.keyboard, ..)`: `.keyboard(false)` removes it, so
    neither binding fires. `dialog::Cancel` and `dialog::Confirm` are
    public.
  - `Popover` / `PopoverContent` and `PopupMenu`: not used by this plan
    (see the menus below); their bindings stay inert.
- **Theme.** `Theme` is a global (`Theme::global`, `cx.theme()` through
  `ActiveTheme`), derefs to `ThemeColor` (`background`, `foreground`,
  `popover`, `popover_foreground`, `primary`, `primary_foreground`,
  `secondary`, `muted`, `muted_foreground`, `accent`, `accent_foreground`,
  `danger`, `border`, `input`, `ring`, `scrollbar`, `scrollbar_thumb`,
  `scrollbar_thumb_hover`, `overlay`, ...) and carries `font_family`,
  `font_size` (default 16 px), `radius`, `radius_lg`, `shadow`,
  `scrollbar_mode`. `theme::init` leaves the mode `Light`;
  `Theme::change(ThemeMode::Dark, None, cx)` loads the built-in `Default
  Dark`, and `Theme::update(cx, |theme| ...)` is the write path that keeps
  the tokens and the Base layer's copy (scrollbars) in step. Nothing follows
  the OS appearance unless the app calls `Theme::sync_system_appearance`.
  `Default Dark` differs from the app's tokens at least in `popover`
  (neutral-950, the app's menus and boxes are `#171717`), `primary`
  (neutral-50, the app's is `#e5e5e5`), `border` (neutral-800, the app's is
  white at 10 %), `input` (`#2f2f2f`, the app's is white at 15 %) and
  `overlay` (`#00000033`, the app's backdrops are `#00000099`).
- **`Root`** (`gpui_component::Root`, from `gpui-base`): `Root::new(view,
  window, cx)` wraps the window's content view in a `div` with the `Root`
  key context and mounts, after the content, a text selection layer and the
  component layers (dialogs, sheets, notifications, tooltips), every frame.
  On macOS `Root::new` also installs a window hit-test forwarder.
  `window.open_dialog`, `close_dialog`, `has_active_dialog` (`WindowExt`)
  and `Dialog` need it (`WindowState::update` panics without one). `Input`
  reaches it only through `WindowState::try_read` / `try_update`.
- **A dialog is drawn in `Root`'s layer, not inside the app's view**, so a
  key pressed while the focus is in a dialog never reaches
  `Riffle::key_down`.
- **Dialog**: `dialog::Dialog` (`.title`, `.content`, `.footer`,
  `.button_props`, `.on_ok` / `.on_cancel` / `.on_close`,
  `.overlay_closable(bool)`, `.keyboard(bool)`, `.close_button(bool)`,
  `.entrance(DialogEntrance)`, `.w` / `.max_w`), opened with
  `window.open_dialog(cx, |dialog, window, cx| ..)`. The builder is a
  render closure that runs again whenever the dialog layer is rebuilt:
  entities are created before opening and captured, and the initial content
  focus is set only on its first run (`open_dialog` focuses the dialog
  shell first). The dialog restores the previously focused handle when it
  closes, plays an exit transition unless the entrance is
  `DialogEntrance::None`, and keeps its focus in GPUI behind a focus trap
  that `Root`'s `Tab` / `TabPrev` walk. The parts `DialogHeader`,
  `DialogTitle`, `DialogDescription`, `DialogContent`, `DialogFooter` are
  separate elements.
- **Input**: `input::InputState` (an entity: `InputState::new(window, cx)`,
  `.placeholder(..)`, `.default_value(..)`, `set_value(value, window, cx)`,
  which emits no `Change` and clears the history, `value()`, `select_all`,
  `focus`, `focus_handle`) and the element `input::Input::new(&state)`
  (`.appearance`, `.bordered`, `.focus_bordered`, `.cleanable`,
  `.disabled`, `.tab_index`, `Sizable`). `InputEvent` is `Change`,
  `PressEnter { secondary, shift }`, `Focus`, `Blur`; there is no Escape
  event. IME goes through its own `EntityInputHandler`.
- **Menus**: `menu::PopupMenu` takes the GPUI focus when it opens, consumes
  `up` / `down`, binds `left` / `right` / `enter` / `escape`, and its
  `activate()` always dismisses after an item runs: there is no keep-open
  mode. `popover::Popover` also moves the focus into its content. Neither
  can keep "the Sort / Filter menu stays open on a click" and "every other
  key reaches the keymap", so the menus keep their models and adopt only
  the look (decided, Trade-off 2).
- **Controls**: `button::Button::new(id)` with `ButtonVariants`
  (`.primary()`, `.secondary()`, `.danger()`, `.ghost()`, `.outline()`,
  `.link()`, `.text()`), `.label`, `.icon`, `.on_click`, `.tab_index`,
  `.tab_stop`, `.track_focus`, `Disableable`, `Sizable`;
  `checkbox::Checkbox::new(id)` (`.label`, `.checked`, `.on_click(|&bool,
  ..|)`, `.tab_index`); `radio::Radio` / `RadioGroup` (`.selected_index`,
  `.on_click(|&usize, ..|)`); `tab::TabBar` is **horizontal only**;
  `scroll::ScrollableElement` (`.overflow_y_scrollbar()`,
  `.overflow_scrollbar()`, `.vertical_scrollbar(&handle)`).
- **Icons.** The components draw their icons (the checkbox's check) from
  asset paths through the application's `AssetSource`.
  `gpui_kit_assets::Assets` is that source; gpui-component does not
  re-export it, and `crates/native` has no direct dependency on
  `gpui-kit-assets` (it is in `Cargo.lock` at 0.9.1 already).
- **Keystrokes before dispatch.** `App::intercept_keystrokes` runs before
  any binding or listener and can stop a keystroke. A `NoAction` binding
  disables bindings of the same keystroke whose context it matches
  (`keymap.rs`, `disabled_binding_matches_context`).
- **The native app today.** One root `div` tracks `self.focus` and routes
  every keystroke through `Riffle::key_down` (`keys::Bindings`); nothing is
  in GPUI's keymap. The trash dialog (`main.rs` `trash_dialog`,
  `trash_key_down`, `TrashFlow`) and the folder-operation dialog
  (`folderops.rs` `folder_dialog`, `folder_dialog_key`, model in
  `dialog.rs`) are `deferred`, `occlude()`d layers whose focused button
  lives in the model, not in GPUI; their backdrops do nothing on a press.
  The delete dialog confirms only on a single mouse click
  (`trash::swallowed_by_delete`, `confirms_delete`). The Sort / Filter
  menus (`menu.rs`, drawn by `main.rs` `popover` / `menu_row`) stay open
  after an item click, and paging, `e`, `z`, `f` and the burst keys keep
  working while one is open; the right-click menus (`context.rs`,
  `context_popover` / `context_row`) close on any key, only `escape`
  stopping there. The settings modal (`settingsview.rs`) is an entity with
  real GPUI focus (`tab_index`, `window.focus_next`), its own `key_down`
  over `settingsmodal::SettingsModal::key` / `stops`, a backdrop whose
  press closes it, a vertical tab list beside the content, local `button` /
  `checkbox` / `radio` helpers, `overflow_y_scroll` lists without a visible
  scrollbar, and six `TextInput` entities (five label names, the import
  pattern). `textinput.rs` is used only by `settingsview.rs`;
  `unicode-segmentation` only by `textinput.rs`. The tree's keyboard is a
  flag in the view (`tree_keys`), not GPUI focus. The colors are `u32`
  constants at the bottom of `main.rs` (`PANE_COLOR`, `MUTED_COLOR`,
  `ERROR_COLOR`, `PRIMARY_COLOR`, `FOREGROUND_COLOR`, `ACCENT_COLOR`,
  `RING_COLOR`, `BORDER_COLOR`, `INPUT_BORDER`, `BUTTON_FILL`, ...), the
  values of `crates/app/ui/style.css`'s tokens. The crate has no GPUI view
  test (`test-support` is off).
- **Other plans in flight.**
  - `docs/plans/20261113-gpui-folder-dialogs/plan.md`: Step 5 (Import)
    landed in `4ee65976` (#1046). Step 6 (Rename by Capture Date) is open
    and adds a dialog through `dialog.rs` / `folderops.rs`. Step 7 (the
    tree's inline rename) is open and is written against `textinput.rs`.
  - `docs/plans/20261114-gpui-bursts/plan.md`: Step 1 landed in `6f1e7936`
    (#1047); Step 2 (`rejectRest`, the companion's `burst`) is open and
    adds an arm to `key_down`.
  - `docs/plans/20261114-gpui-empty-state/plan.md` was named as in flight
    but is not on `origin/main` or in any local checkout at planning time,
    so its overlap is unknown. Each step reads it at its start if it has
    landed.
  - All of them edit `main.rs`, the `crates/native` sentence of `CLAUDE.md`
    and `docs/agents/native-app.md`.

### Rules every step follows

- Behavior is preserved. A step that finds a component cannot keep a
  behavior listed in its "Done when" stops and records it in `learnings.md`
  rather than shipping the difference.
- The pin rule of `docs/agents/native-app.md` holds: no version of
  `gpui-component`, `gpui-pre` or `gpui-pre-platform` moves in this plan. A
  new direct dependency on a gpui-kit crate is an exact pin at the same
  version.
- **A/B latency.** Per `native-app.md`'s "A same-session A/B gates only
  work on the page path", dialogs, context menus and the settings UI need
  none. Steps 1-4 and 6-8 stay off the page path: nothing they add is laid
  out or painted while paging with every menu, dialog and modal closed, and
  the always-drawn `Sort ▾` / `Filter ▾` buttons are not changed. Step 5
  mounts `Root`, which is drawn every frame, and runs the gate. Any other
  step that finds it must change an always-drawn element says so before
  doing it and runs the gate.
- The canvases keep reading the `u32` color constants; the theme is built
  from those same constants, so there is one source of each color.
- Other sessions edit `crates/native/src/main.rs`, the `crates/native`
  sentence of `CLAUDE.md` and `docs/agents/native-app.md`. Keep each step's
  edits there to the lines it names, re-check `origin/main` at the start of
  each step, and resolve a conflict with a merge commit.
- **Manual checks** are the user's: each step runs only automated checks
  and records its GUI check in `learnings.md` as `Pending manual check
  (Step N, Windows, the user's)` with the exact command, the steps and the
  expected result. Only on copies under
  `D:\Photos\tests\20261114-gpui-component-adoption\`, always with an
  explicit fresh `--index` and an explicit test `--settings`; `cargo build`
  before launching; rename a locked `riffle-native.exe` aside, do not kill
  it.
- Docs move with the code: each step updates `docs/agents/native-app.md`
  and the `crates/native` sentence of `CLAUDE.md` where a module's role
  changes. Everything committed is in English, Conventional Commits.

## Steps

- [x] Step 1: Call `gpui_component::init`, map the `Theme` to the Neutral dark look, and record the key bindings
  - Done when:
    - `gpui_component::init(cx)` runs once at startup, before the window
      opens. No component is drawn yet and the window's content is not
      wrapped in `Root`; the app looks and behaves exactly as before.
    - `crates/native/src/theme.rs` sets the global `Theme` to dark and maps
      it to the app's tokens from the existing color constants: at least
      `background`, `foreground`, `popover` / `popover_foreground`
      (`#171717` / `#fafafa`), `primary` / `primary_foreground` (`#e5e5e5`
      / `#171717`), `secondary`, `muted`, `accent` (`#262626`) and their
      foregrounds, `muted_foreground` (`#a1a1a1`), `danger` (`#ff6467`),
      `border` (white 10 %), `input` (white 15 %), `ring` (`#737373`),
      `overlay` (the backdrops' `#00000099`), the scrollbar thumb (the
      border color, as `ui-styling.md` says), the base font size the app's
      controls use (13 px) and the radii. The mapping is a function with a
      unit test if it can be built without a window (the step decides;
      `ThemeColor` is a plain struct).
    - The theme does not follow the OS appearance: a light OS setting leaves
      the app dark.
    - `docs/agents/native-app.md`'s "gpui-component is a dependency only"
      section is rewritten: `init` is called, what the theme maps, the table
      of contexts and keys above as verified at 0.9.1, that only the debug
      inspector binding is global and is left in place (a user binding on
      `ctrl+shift+i` is shadowed in a debug build only), that `Root` is not
      mounted yet and what its `tab` / `shift-tab` would take, and the rule
      that a version bump re-reads the bindings (grep `KeyBinding::new` for
      a `None` context and for the `Root` context).
    - With the debug build: `tab` still toggles the side panes, every
      default shortcut still acts, `ctrl+shift+i` opens the component
      inspector. `--auto` runs as before and its timing lines are unchanged
      in kind.
    - `CLAUDE.md`'s `crates/native` sentence names `src/theme.rs`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (including the `native` job on all three platforms).
    - `learnings.md` records the pending manual check: launch on a test
      copy, page with the arrows, press `tab`, `f`, `z`, `e`, `d`, open and
      close the Sort / Filter menus, the settings modal (type in a label
      name with the IME), a right-click menu and the trash dialog; nothing
      differs from `main`.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/main.rs` (the `init` call in the
      `application().run` closure, `mod theme;`), `crates/native/src/theme.rs`
      (new), `docs/agents/native-app.md`, `CLAUDE.md`.
    - Call `Theme::change(ThemeMode::Dark, None, cx)` first, then
      `Theme::update(cx, |theme| ..)` for the colors: a mode switch reloads
      the registered theme over color edits made in the same closure.
    - Read the colors from `main.rs`'s constants (`PANE_COLOR`,
      `PRIMARY_COLOR`, `BORDER_COLOR`, ...), which a child module can reach;
      do not add a second copy of a hex value. Do not touch the canvases'
      use of those constants.
    - Verify the resolved `Hsla` of each mapped field against the constant
      it came from (the default theme names Tailwind shades such as
      `neutral-400`; do not assume they equal the app's tokens).
    - The window root's own `bg(rgb(0x000000))` / `text_color(0xffffff)`
      stays as it is; the theme's `background` is for components.
    - Off the page path: `init` only sets globals and context-scoped
      bindings. Verify that a non-empty GPUI keymap does not change the
      `--auto` page timings in kind (one quick run, not the A/B gate); if it
      visibly does, stop and record it.

- [x] Step 2: Replace `textinput.rs` with gpui-component's `Input`
  - Done when:
    - The settings modal's five label name fields and the Import tab's
      pattern field are `InputState` entities drawn with `Input`, and
      `crates/native/src/textinput.rs` is deleted along with
      `unicode-segmentation` if nothing else uses it.
    - Each use site behaves as before: a name field commits on Enter and on
      losing the focus, and Escape commits and then closes the modal; the
      pattern field commits on Enter and on blur, Escape likewise, and the
      example line under it follows every edit; `Reset` and a language pick
      show their names in the fields; opening the modal shows the stored
      values; Tab and Shift+Tab move between the modal's stops from inside
      a field; a key typed in a field reaches neither the modal's own keys
      nor the window's keymap (`settingsmodal::stops` still decides);
      `ctrl+a` / `c` / `x` / `v` (`cmd` on macOS) work in a field.
    - The fields look like the current ones: the `INPUT_BORDER` border, the
      focused border in `PRIMARY_COLOR`, the same height and text size
      within a pixel or two.
    - If the tree's inline rename (the folder-dialogs plan's Step 7) is on
      `origin/main` when this step starts, its editor is migrated to
      `Input` in this PR too, with its behavior (the name selected whole,
      Enter or blur confirming, Escape canceling, each edit deciding once)
      kept. If it is not, that plan's Step 7 text needs a one-line edit by
      its owner: "the editor is an `InputState` / `Input` (`set_value`,
      `select_all`, `PressEnter` / `Blur` as the commit)" instead of
      `textinput.rs`; the PR description says so.
    - `docs/agents/native-app.md`: the "Text is typed through
      `textinput.rs`" section is replaced by one on `Input` (which events
      stand for commit and cancel, the Windows rule that a stopped key down
      types nothing and how it holds with the `Input` context's bindings,
      what the component adds that the port lacked: undo, word movement,
      horizontal scroll, double-click selection, a right-click menu), and
      the two other mentions of `textinput.rs` are updated. `CLAUDE.md`'s
      `src/textinput.rs` clause is removed and `settingsview.rs`'s wording
      follows.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - `learnings.md` records the pending manual check (Windows): in the
      settings' Sidecar tab type a label name with the Japanese IME
      (composition underlined, the candidate window at the caret, Enter
      inside the composition commits the composition and not the field),
      commit with Enter, with a click elsewhere and with Tab; Escape in a
      field closes the modal with the name kept; in the Import tab type a
      pattern with and without the IME, see the example line follow each
      key, an invalid pattern show its refusal, `ctrl+a` then typing
      replace the text; right-click in a field does not panic; after
      closing the modal the arrows page again.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 merged (`init`, the theme).
    - Files: `crates/native/src/settingsview.rs`,
      `crates/native/src/textinput.rs` (deleted), `crates/native/src/main.rs`
      (`mod textinput;`), `crates/native/Cargo.toml` / `Cargo.lock`,
      `docs/agents/native-app.md`, `CLAUDE.md`.
    - Map the old events: `Event::Commit` = `InputEvent::PressEnter` or
      `InputEvent::Blur`; `Event::Cancel` has no `InputEvent`. On a
      single-line input the `Escape` action propagates, so the key reaches
      `SettingsView::key_down`; take the "commit, then close" there (or
      with an `on_action` for `input::Escape` on the box). Verify which of
      the two actually fires at the pinned rev.
    - The example line now follows `InputEvent::Change` instead of
      `cx.observe` (an `InputState` also notifies for its caret blink).
    - `set_text` becomes `InputState::set_value`, which emits no `Change`:
      call `show_example` after it where the old `observe` did the work.
    - Verify that `tab` / `shift-tab` (bound to `IndentInline` /
      `OutdentInline` in the `Input` context) propagate on a single-line
      input and still reach the modal's `ModalKey::Focus`, and that the
      fields stay in the modal's tab order (`Input::tab_index`).
    - Verify, without `Root` mounted, that focusing, typing, the IME and a
      right-click in an `Input` do not panic (`WindowState::try_read` is
      used on those paths) and whether the component's right-click menu
      shows; if it needs `Root` or the icon assets to draw, record it and
      keep it from opening rather than mounting `Root` here.
    - `editing()` reads `InputState::focus_handle`.

- [x] Step 3: The settings modal's controls: buttons, the checkbox, radios and scrollbars
  - Done when:
    - `settingsview.rs`'s local `button`, `checkbox` and `radio` helpers
      are gone: the small outlined buttons (`Choose…`, `Reset`, the
      language row, the Keyboard Shortcuts tab's `+`, key chips' remove,
      `Reset all`) and the close `×` are `Button`s, the Culling tab's
      auto-advance toggle is a `Checkbox`, the Sidecar tab's format choice
      is `Radio`s.
    - The Import tab's and the Keyboard Shortcuts tab's scrolling areas show
      a scrollbar from `ScrollableElement` and scroll as before (wheel, and
      the focused row scrolled into view if it was).
    - The tab list stays the hand-drawn vertical list, restyled only so far
      as it reads the theme's tokens: the selected tab is the one tab stop,
      the arrow keys move between tabs while a tab is focused
      (`settingsmodal::next_tab`), a click selects, opening focuses the tab
      the modal was left on.
    - The modal's keyboard is unchanged: Tab / Shift+Tab walk the same
      stops in the same order, Enter and Space press the focused control,
      Escape closes, the shortcut capture (`+`, then a key) still takes the
      next key including Enter, Space and Tab, and a refusal shows in the
      status line.
    - The look stays close: outlined buttons on the `BUTTON_FILL` /
      `INPUT_BORDER` look, the focus ring in the theme's ring or primary
      color, the same text sizes.
    - The application loads `gpui_kit_assets::Assets` if a used component
      draws an icon (the checkbox's check), with `gpui-kit-assets` pinned
      `=0.9.1` in `crates/native/Cargo.toml`; `native-app.md` records the
      asset source and adds the crate to the upgrade rule's pins.
    - `settingsmodal.rs`'s unit tests pass unchanged.
    - `docs/agents/native-app.md`'s settings modal sections and `CLAUDE.md`'s
      `settingsview.rs` clause follow.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - `learnings.md` records the pending manual check: walk every tab with
      the keyboard only (Tab, Shift+Tab, arrows on the tab list, Space and
      Enter on each control kind), flip auto-advance, switch the sidecar
      format, add and remove a shortcut (capture `tab` and `enter`), scroll
      both lists with the wheel and by dragging the scrollbar, click the
      backdrop and `×` to close.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 and 2 merged.
    - Files: `crates/native/src/settingsview.rs`, `crates/native/src/main.rs`
      (`application().with_assets(..)`), `crates/native/Cargo.toml` /
      `Cargo.lock`, `docs/agents/native-app.md`, `CLAUDE.md`.
    - The modal's shell (the `deferred` backdrop, the box, its focus handle
      and `key_down`) stays in this step; Step 8 converts it.
    - The components take the GPUI focus themselves (`tab_index`,
      `tab_stop`, `track_focus`); keep the modal's stops by giving each the
      same `tab_index` the `div` had. Verify that Enter and Space on a
      focused `Button` / `Checkbox` / `Radio` activate it (no `Root`
      mounted yet), and that `settingsmodal::stops` still lets those keys
      reach the control and not the modal's own handling.
    - The capture state moves the focus to the box (`window.focus(&self.focus)`)
      because the pressed `+` is replaced by the prompt; keep that.
    - `Button`'s default size and padding are larger than the current
      `px(6.)` buttons; use `Sizable` (`.small()` / `.xsmall()`) and
      `.outline()`, and compare against the current look before choosing.
    - `Assets` embeds only the component icon set (`AllAssets` is the whole
      Lucide catalog; do not use it). Adding the asset source has no
      per-frame cost.

- [x] Step 4: The look of the Sort / Filter menus and the right-click menus
  - Done when:
    - The open Sort and Filter menus and the tree's and strip's right-click
      menus read the theme (popover background and foreground, border,
      radius, the muted heading color, the accent hover, the separators)
      instead of the `u32` constants where a theme token exists, their rows
      have gpui-component's menu item look (padding, radius, hover, the
      check mark and the shortcut hint's place) as far as it can be had
      without `PopupMenu`, and a list taller than the window allows shows a
      scrollbar from `ScrollableElement`.
    - Nothing else changes: `menu.rs` and `context.rs` keep their models;
      the placement stays `deferred` / `anchored` (above the button with
      the right edges aligned; at the pointer, flipped and kept inside the
      window); `on_mouse_down_out` closes on an outside press and the press
      on a menu's own button does not reopen it; a Sort / Filter item click
      applies and leaves the menu open; `escape` closes an open Sort /
      Filter menu; every other key goes through the window's keymap
      (paging, `e`, `z`, `f`, the burst keys); any key closes a right-click
      menu, only `escape` stopping there; a press on a right-click menu
      hands the keyboard back from the tree; the menus never take the GPUI
      focus; nothing is laid out while they are closed.
    - The always-drawn `Sort ▾` / `Filter ▾` buttons are not changed.
    - `menu.rs`'s and `context.rs`'s unit tests pass unchanged.
    - `docs/agents/native-app.md`'s "The sort and filter menus are deferred
      popovers" and "The right-click menu is a deferred popover" sections
      say that the look comes from the theme, why `PopupMenu` and `Popover`
      are not used (they take the focus; `PopupMenu::activate` always
      dismisses), and keep their rules.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - `learnings.md` records the pending manual check: the four menus look
      like the settings' controls; each line of the behavior list by hand,
      including an outside click in the strip, in the preview and on the
      menu's own button, `escape`, paging and the burst keys with the
      Filter menu open, a long Filter menu scrolled by wheel and scrollbar,
      and the tree keeping or handing back the keyboard around a
      right-click menu.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 merged (Step 3 too if a row draws a component icon);
      independent of Step 2 in content.
    - Files: `crates/native/src/main.rs` (`popover`, `menu_row`,
      `context_popover`, `context_row`), `docs/agents/native-app.md`.
    - gpui-component's row element (`menu::MenuItemElement`) is
      `pub(crate)`, so the row look is matched by styling the existing
      `div` rows from `cx.theme()`; read `menu/menu_item.rs` and
      `popup_menu.rs`'s render for the paddings, radius and colors rather
      than guessing.
    - Verify that a `ScrollableElement` scrollbar inside an `occlude()`d,
      `deferred` popover scrolls by wheel and by drag and that a press on
      the scrollbar does not count as a press outside the menu.

- [x] Step 5: Mount `Root` around the window's content, with `tab` / `shift-tab` kept for the keymap
  - Done when:
    - The window's root view is `Root::new(<the app's view>, window, cx)`.
      No dialog, notification or tooltip is opened yet; the app looks and
      behaves exactly as before.
    - With no dialog open, `tab` reaches the app's keymap (the default
      `toggleSides` toggles the side panes), `shift+tab` reaches it too (a
      user binding on it acts), and inside the settings modal Tab /
      Shift+Tab still walk its stops and the shortcut capture still takes
      `tab`. `ctrl+c` / `cmd+c` with nothing selected still reaches the
      keymap and an `Input` still copies its selection.
    - The mechanism leaves `Root`'s own `Tab` / `TabPrev` working when the
      focus is inside a component dialog (Steps 6-8 rely on it for the
      focus trap), shown by a note in `learnings.md` of how it was verified
      (for example a temporary `open_dialog` with two buttons, not
      committed).
    - Everything that named the window's root as the app's view still
      works: the focus given at startup, `SettingsView`'s `Change::Closed`
      focus restore, `on_app_quit`, the window bounds saved on quit, the
      tree's keyboard flag, `--auto`.
    - The same-session A/B latency gate of `native-app.md` has been run
      (this step against its parent commit, the pager's `page` lines, with
      `frameWait` p50 checked per run) and its numbers are recorded in
      `learnings.md` and in a new `native-app.md` Timings entry ("`Root`
      costs the pager ..."). If the wrapper costs the pager measurably, the
      step stops and reports it instead of merging (Trade-off 3).
    - `docs/agents/native-app.md` records: that `Root` is mounted and what
      it adds per frame, how `tab` / `shift-tab` are kept from `Root` with
      no dialog open, the `Copy` binding's propagation, the macOS hit-test
      forwarder `Root::new` installs, and the rule for a version bump.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - `learnings.md` records the pending manual check: `tab` and a
      user-bound `shift+tab`, the settings modal's Tab walk and capture of
      `tab`, IME in a settings field, the menus' outside click, the trash
      dialog's Tab / Enter / Escape, the tree taking and handing back the
      keyboard, quit and relaunch restoring the window; on macOS, a click
      on every pane and on the title bar still lands where it should.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 merged. Placed after Steps 2-4 so a failed gate blocks
      only the dialog steps; nothing in Steps 2-4 depends on it.
    - Files: `crates/native/src/main.rs` (the `open_window` closure, the
      root `div` in `render`, wherever the window's root entity is read),
      `docs/agents/native-app.md`.
    - `Root`'s `Tab` / `TabPrev` action types are private, so the app
      cannot handle them with `on_action`. The step decides between, and
      verifies at the pinned rev: (a) a key context on the app's root `div`
      (deeper than `Root`'s) with `NoAction` bindings for `tab` and
      `shift-tab` in it, so that with the focus inside the app's view the
      keys fall through to `key_down`, while a dialog (drawn in `Root`'s
      layer, outside that context) still gets `Root`'s bindings; read
      `keymap.rs`'s `disabled_binding_matches_context` for whether a
      deeper-context `NoAction` masks the shallower `Root` binding; (b)
      `cx.intercept_keystrokes`, handing `tab` / `shift-tab` to the app's
      key handler and stopping the keystroke when no component dialog has
      the focus. Whichever is taken must not stop a typed character
      (`native-app.md`: a stopped key down types nothing on Windows).
    - The settings modal is still inside the app's view in this step, so
      its Tab handling keeps going through `SettingsView::key_down`.
    - This step is on the page path: `Root` adds a wrapper `div`, a text
      selection layer and the (empty) plugin layers to every frame. Follow
      `native-app.md`'s A/B rules (idle machine, no other builds, compare
      `frameWait` p50 first).
    - `Root::new` takes `impl Into<AnyView>`; the app's entity is created
      first and kept wherever `main.rs` needs it.

- [x] Step 6: The trash / delete dialog as a `Dialog`
  - Done when:
    - `Move to Trash…` / `Delete Permanently…` opens a gpui-component
      `Dialog` through `window.open_dialog`: the title, the line, the
      destructive run `Button` and Cancel, on the theme's colors, at the
      current width, with no close button, no entrance or exit animation,
      and a backdrop press doing nothing.
    - Behavior is unchanged: nothing under the dialog can be pressed; the
      initial focus is where `TrashFlow` put it; Tab / Shift+Tab move
      between the two buttons (now `Root`'s focus trap); Enter and Space
      press the focused button in a Move to Trash; in a Delete Permanently
      only a single mouse click on the run button confirms: Enter and
      Space, with or without a modifier, and a key held from before the
      dialog opened are swallowed (`swallowed_by_delete`), except a held `tab` /
      `shift+tab`, which is `Root`'s action, dispatched before any key
      listener, and only moves the focus (accepted exception, recorded in
      `learnings.md`), and a keyboard
      activation or the rest of a double-click does not confirm
      (`confirms_delete`); Escape dismisses while asking and does nothing
      while running; both buttons are disabled while it runs and the
      dialog closes when the run ends; every other key is swallowed and
      none reaches the window's keymap, the settings key included; after
      it closes the focus is back on the app's root, the keymap works, and
      the tree has the keyboard if it had it before.
    - The dialog follows the flow: a run held by the gate, a run started
      and finished, a dismissal and a folder switch each open and close it
      exactly when the hand-built layer was shown and hidden, and the
      dialog's own close paths cannot close it behind the flow's back.
    - `TrashFlow` no longer holds the focused button; it gives the button
      the initial focus goes to. Its unit tests are rewritten to match (the
      Tab-cycle cases go, the initial-focus, dismiss, run and delete-key
      cases stay). `idle.rs`'s tests pass unchanged.
    - `docs/agents/native-app.md`'s "The trash dialog is a layer of the
      window, its focus in the model" section is rewritten (the `Dialog`
      options used and why, where the keys are taken, how the flow opens
      and closes it, the focus restore), and `CLAUDE.md`'s `trash.rs`
      clause follows.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - `learnings.md` records the pending manual check on a test copy: Move
      to Trash with Enter, with Space, with a click; Tab and Shift+Tab to
      Cancel and Enter; Escape; a click on the backdrop; `ctrl+,` and a
      paging key while it is open (nothing happens); Delete Permanently
      with Enter, Space, a held key from the menu, a double-click from the
      menu item, then a single click; Escape during a long run; a press
      during pass 1 (the held run); undo (`ctrl+z`) afterwards; the same
      with the tree holding the keyboard before the dialog opened.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1, 3 (`Button`) and 5 (`Root`) merged.
    - Files: `crates/native/src/main.rs` (`trash_dialog`, `trash_key_down`,
      `key_down`'s `trash.busy()` branch, the places the flow starts and
      ends), `crates/native/src/trash.rs` (`TrashFlow`, its tests),
      `docs/agents/native-app.md`, `CLAUDE.md`.
    - `.keyboard(false)`: with the `Dialog` key context on, `enter` is
      bound to `Confirm` and runs `on_ok` whichever button has the focus,
      and before any key listener, which would break "Enter presses the
      focused button" and the delete dialog's "only a mouse click
      confirms". With it off, Escape, Enter and Space arrive as key downs:
      take them in a listener on an element of the dialog's content that
      is on the focus path (it calls `trash::swallowed_by_delete` with
      `KeyDownEvent::is_held` and `keys::key_name`, as `trash_key_down`
      does). `.overlay_closable(false)`, `.close_button(false)`,
      `.entrance(DialogEntrance::None)`.
    - The buttons are `Button`s in the dialog's footer with focus handles
      the view keeps (`track_focus`), not the dialog's built-in OK / Cancel
      (`button_props`), so the run button's `on_click` can read the
      `ClickEvent` (`is_keyboard`, `click_count`) for `confirms_delete`.
      Verify that a focused `Button` activates on Enter and Space by itself
      and, for the delete dialog, that the content's key listener can
      swallow those keys before the button sees them (or the run button's
      `on_click` refuses a keyboard click, which `confirms_delete(0)`
      already does).
    - The builder passed to `open_dialog` is a render closure run again on
      every rebuild: capture a weak handle to the app's view and read the
      flow from it; set the initial focus only on the first run of each
      opening (a `Cell<bool>` guard per opening). Verify that a change of
      the flow (asking to running) redraws the dialog layer; if the view's
      `cx.notify()` does not reach it, refresh the window.
    - Open with `window.open_dialog` where the flow enters asking and close
      with `window.close_dialog` where it ends; several of those places run
      in a `Context<Self>` without a `Window` (events from the channel), so
      decide how the window is reached there (the view keeps its
      `WindowHandle`, or the open / close is deferred to the next render).
    - While both buttons are disabled, check where the focus rests (a
      disabled `Button` may leave the tab order) and that the Escape rule's
      listener is still on that path; give the content its own tracked
      focus handle if not.
    - The dialog restores the previously focused handle on close; verify it
      is the app's root handle and that `tree_keys` (a flag, not focus) is
      untouched.
    - This step sets the pattern Step 7 reuses; keep the shell options and
      the open / close plumbing in one place both can call.

- [x] Step 7: The folder operations' shared dialog as a `Dialog`
  - Done when:
    - `folderops.rs` opens the dialog every folder operation shares
      (Rewrite Sidecars from Index, Delete Sidecars, Sequence JPEG
      Timestamps, Import, Rename by Capture Date) as a gpui-component
      `Dialog` the way Step 6 does the trash dialog: the title, the text
      lines, the row list, the note, the reason, the running note, the run
      button (primary or destructive) and Cancel.
    - Behavior is unchanged, per `dialog.rs`'s rules: the initial focus is
      on the run button, or on Cancel when the run is disabled; Tab /
      Shift+Tab cycle the enabled buttons; Enter and Space press the
      focused one; Escape dismisses, cancels a cancelable run and does
      nothing during a run that cannot be canceled; every other key is
      swallowed; a backdrop press does nothing; a long row list scrolls
      inside the dialog, now with a scrollbar; the progress updates while
      running; the import's dialog steps aside during its run and comes
      back on `Import…`; the settings key is refused while a folder flow
      is busy (`folderops.busy()`), including with the import's dialog
      aside; one flow at a time as before.
    - `dialog.rs` no longer holds the focused button or the Tab rule; it
      keeps the content, the buttons' enabled state, which button gets the
      initial focus, and what Escape, Enter and Space mean in each state.
      Its unit tests are rewritten to match; `sidecars.rs`, `sequence.rs`,
      `import.rs` and `datefolder.rs` tests pass unchanged.
    - `docs/agents/native-app.md`'s "The folder operations share one dialog
      model" section and `CLAUDE.md`'s `dialog.rs` / `folderops.rs` clauses
      follow.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - `learnings.md` records the pending manual check on test copies: each
      operation's dialog opened from the tree's right-click menu; Tab,
      Shift+Tab, Enter, Space, Escape; a disabled run (focus on Cancel); a
      long list scrolled by wheel and scrollbar; Sequence's and Import's
      cancel during their runs; Escape during a run that cannot be
      canceled; the import stepping aside and coming back; `ctrl+,` while a
      flow is busy; an operation pressed during pass 1 opening its dialog
      when the scan ends; the tree's keyboard after each close.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 6 merged, and waits for the folder-dialogs plan's Step 6
      (Rename by Capture Date, which adds a dialog through `dialog.rs` and
      `folderops.rs`) to be merged, so this step converts the final set
      once instead of conflicting with it. Its Step 5 (Import) is already
      on `main`. Its Step 7 (the inline rename) does not touch the dialog
      drawing and need not be waited for.
    - Files: `crates/native/src/folderops.rs` (`folder_dialog`,
      `folder_dialog_key`, `rows`, where a flow opens and closes its
      dialog), `crates/native/src/dialog.rs`, `crates/native/src/main.rs`
      (`key_down`'s `folderops.dialog_open()` branch),
      `docs/agents/native-app.md`, `CLAUDE.md`.
    - Reuse Step 6's shell options and open / close plumbing; do not
      introduce a second mechanism. The same `.keyboard(false)` reasoning
      holds: Enter must press the focused button, not always the run
      button.
    - The dialog is opened from background results (a preview read, a held
      operation released at the scan's end); use whatever Step 6 settled
      for reaching the window from there.
    - "Stepping aside" is a close of the `Dialog` with the flow still
      alive, and "coming back" a new `open_dialog`; check the focus restore
      on each.

- [ ] Step 8: The settings modal's shell as a `Dialog`
  - Done when:
    - `ctrl+,` (`meta+,` on macOS) opens the settings as a gpui-component
      `Dialog` holding the `SettingsView`'s content: the hand-built
      backdrop and box are gone; the size (90 % of the window up to 860 px
      wide, 80 % up to 720 px high), the title row with the close `×`, the
      tab list, the content and the status line look as before.
    - Behavior is unchanged: it opens on the tab it was left on with that
      tab focused; Escape closes it, also from inside a text field (which
      commits first); a press on the backdrop closes it; `×` closes it;
      Tab / Shift+Tab walk the same stops in the same order and wrap; the
      arrows move between tabs while a tab is focused; Enter and Space
      press the focused control; a key typed in a field goes to the field
      (`settingsmodal::stops`); no key reaches the window's keymap while it
      is open; it does not open while a folder flow or the trash flow is
      busy; closing it emits `Change::Closed`, gives the focus back to the
      app's root, and the tree has the keyboard if it had it before.
    - The shortcut capture is unchanged: after `+`, the next key is taken
      as the shortcut, including `enter`, `space`, `tab`, `shift+tab` and
      modified keys; `escape` cancels the capture without closing the
      dialog; `APP_KEYS` and the backend's refusals show in the status
      line.
    - `settingsmodal.rs`'s unit tests pass (rewritten only where a rule
      moved to the `Dialog`); `--auto` neither opens nor is affected by it.
    - `docs/agents/native-app.md`'s "The settings modal is its own entity"
      and "The keymap is edited in the modal" sections and `CLAUDE.md`'s
      `settingsview.rs` / `settingsmodal.rs` clauses follow.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - `learnings.md` records the pending manual check: open and close by
      each path (Escape, backdrop, `×`, Escape from a name field and from
      the pattern field); the full Tab walk on every tab, forward and back;
      capture `enter`, `space`, `tab`, `shift+tab`, `ctrl+alt+1` and cancel
      a capture with `escape`; IME in a label name and in the pattern;
      `Choose…` (the OS folder picker) and the focus after it returns;
      paging keys while open; the tree's keyboard before and after.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 2, 3, 5 and 6 merged (the fields and controls are
      components, `Root` is mounted, the `Dialog` plumbing exists).
      Independent of Step 7, so it need not wait for the folder-dialogs
      plan.
    - Files: `crates/native/src/settingsview.rs`,
      `crates/native/src/settingsmodal.rs`, `crates/native/src/main.rs`
      (where the modal is opened, its child in `render`, the
      `settings_view.is_open()` guard in `key_down`),
      `docs/agents/native-app.md`, `CLAUDE.md`.
    - `SettingsView` stays an entity created once; the `open_dialog`
      builder captures it and puts it in the dialog's content (never
      `cx.new` inside the builder). Its own `key_down` stays on its box,
      which is on the focus path of every control inside it.
    - `.keyboard(false)`, as in Steps 6 and 7: the `Dialog` context's
      `escape` and `enter` bindings would otherwise fire before the box's
      key listener, so the capture could not take `escape` / `enter`, and
      Enter would run `on_ok` instead of pressing the focused control.
      Escape's close stays in `SettingsView::key_down`
      (`ModalKey::Close`). `.overlay_closable(true)` with the close routed
      to the view's `close` (so `Change::Closed` is emitted), no built-in
      close button (the `×` stays the view's), no footer buttons,
      `.entrance(DialogEntrance::None)`.
    - Tab now arrives as `Root`'s `Tab` / `TabPrev` action before the box's
      key listener, and `Root` walks the dialog's focus trap. For the
      ordinary walk that replaces `ModalKey::Focus`; verify the order and
      the wrap equal today's, and that `Input`'s own `tab` binding still
      propagates to it. For the capture, `tab` / `shift-tab` must reach
      `ModalKey::Add` instead: the step decides how (a keystroke
      interceptor active only while capturing, or a deeper key context on
      the capturing box with `NoAction` for the two keys), reusing Step 5's
      mechanism where it fits.
    - The capture moves the focus to the box's own handle
      (`window.focus(&self.focus)`); keep it inside the dialog's focus trap
      so `Root` does not pull the focus elsewhere.
    - `open_dialog` focuses the dialog shell first; focus the remembered
      tab on the builder's first run of each opening.
    - `Change::Closed`'s handler focuses the app's root itself today; with
      the `Dialog` restoring the previous focus, keep exactly one of the
      two.

## Trade-offs and risks

1. **`textinput.rs` and the tree's inline rename.** Decided (2026-11-14,
   user): Step 2 goes first and deletes `textinput.rs`; the folder-dialogs
   plan's Step 7 builds its editor on `InputState` / `Input`, which needs a
   one-line edit of that plan by its owner. If that Step 7 is already on
   `main` when Step 2 starts, Step 2 migrates its editor too.
2. **The menus keep their models; only the look is adopted.** Decided
   (2026-11-14, user), for all four menus. `PopupMenu` and `Popover` take
   the GPUI focus, `PopupMenu` handles the arrows, Enter and Escape itself,
   and `PopupMenu::activate` always dismisses, so there is no keep-open
   mode for Sort / Filter. Not taken: `PopupMenu` for the right-click menus
   only, which would change what the arrow keys and Enter do with a menu
   open.
3. **`Root` and `Dialog` for the dialogs.** Decided (2026-11-14, user):
   mount `Root` and use `window.open_dialog` for the trash / delete dialog,
   the folder-operation dialog and the settings modal. What it costs, and
   where the plan pays it:
   - `Root`'s `tab` / `shift-tab` against `toggleSides` and user bindings:
     Step 5.
   - The wrapper is drawn every frame: the A/B gate in Step 5. **If the
     gate shows a cost to the pager, Steps 6-8 are blocked** and the
     decision returns to the user (the fallback is the hand-built layers
     with `Button`, the dialog parts and the scrollbar, without `Root`).
   - The focus moves from the models into GPUI: `TrashFlow`'s and
     `dialog.rs`'s focused button and Tab rule go, their tests are
     rewritten (Steps 6, 7). The Tab cycle is then covered only by hand
     checks.
   - `.keyboard(false)` on all three dialogs, because the `Dialog`
     context's `enter` -> `Confirm` would run `on_ok` whichever button has
     the focus and before any key listener. The dialogs' Escape, Enter and
     Space rules are rebuilt as key listeners inside the dialog content.
   - A dialog is outside the app view's key path, so the "every other key
     is swallowed" and "the settings key is refused" rules now hold by
     construction rather than by `key_down`'s early returns; the flows'
     own guards (`folderops.busy()`) stay.
   - `Root::new` installs a macOS window hit-test forwarder, unverified on
     this app; Step 5's manual check covers it.
4. **The settings tab list stays hand-drawn and vertical.** Decided
   (2026-11-14, user): `TabBar` is horizontal only.
5. **`Checkbox` for auto-advance.** Decided (2026-11-14, user), not
   `Switch`.
6. **The debug inspector binding stays.** Decided (2026-11-14, user):
   `ctrl-shift-i` / `cmd-alt-i` is global in debug builds only and is left
   in place and documented; a user binding on that key is shadowed in a
   debug build, never in a release one.
7. **The settings modal's shell becomes a `Dialog` too.** Decided
   (2026-11-14, user), as Step 8. Its risk is the shortcut capture: `tab` /
   `shift+tab` are taken by `Root` before any listener inside a dialog, so
   the capture needs the interceptor or a deeper `NoAction` context, and
   `escape` / `enter` are only capturable with `.keyboard(false)`. If the
   capture of `tab` cannot be kept, Step 8 stops and reports it rather than
   shipping a capture that cannot take it.
8. **Visible additions that come with the components.** Scrollbars appear
   where there were none; `Input` adds undo, word movement, horizontal
   scroll, double-click selection and possibly a right-click menu;
   components animate hover and focus states. These are taken as part of
   the adoption, not as features; each step lists them in `native-app.md`.
   The dialogs' entrance and exit animations are turned off to keep the
   current feel.
9. **Icons need an asset source and a new direct pin.** `gpui-kit-assets`
   `=0.9.1` joins the exact pins the upgrade rule moves together (Step 3).
   Its license notice (Lucide, ISC) is already deferred to packaging by
   `native-app.md`.
10. **Conflicts with the plans in flight.** The folder-dialogs plan (Steps
    6 and 7 open), the bursts plan (Step 2 open) and this one all edit
    `main.rs`, the `crates/native` sentence of `CLAUDE.md` and
    `native-app.md`; a rebase or merge is to be expected at every step.
    Step 7 waits for the folder-dialogs plan's Step 6. Steps 4-8 must keep
    the burst keys (`arrowup` / `arrowdown`, `alt+arrowleft` /
    `alt+arrowright`, `shift+x`) reaching the keymap. The
    `20261114-gpui-empty-state` plan could not be read at planning time; if
    it draws a new always-visible element or a control, it should use the
    theme and components from the steps of this plan that have landed, and
    each step here checks it for overlap at its start.
11. **Eight steps.** More than the usual seven, because `Root` gets a step
    of its own for the A/B gate and the settings shell is a separate
    conversion with its own risk. Steps 6 and 7 could be one PR only if the
    folder-dialogs plan's Step 6 has landed by then; they are kept apart so
    the trash dialog does not wait for it.
12. **What automated checks cannot show.** The crate has no GPUI view test
    (`test-support` is off) and `--auto` cannot click, so the IME, focus,
    Tab order, outside-click and Escape behavior of every step is a hand
    check recorded in `learnings.md`; the pure models' unit tests only
    guard the rules, not the wiring.
13. **`mise run native:check` as the gate.** The requirement asked for it
    per step; the agents' local gate is `mise run ci:local` (which runs the
    native clippy and tests when `crates/native` changed) plus the PR's
    `native` job on three platforms, so the "Done when" lines are written
    against those.

## Progress

- (2026-11-14) Step 1 complete
- (2026-11-14) Step 2 complete
- (2026-11-14) Step 3 complete
- (2026-11-14) Step 4 complete
- (2026-11-14) Step 5 complete
- (2026-11-14) Step 6 complete
- (2026-11-14) Step 7 complete
