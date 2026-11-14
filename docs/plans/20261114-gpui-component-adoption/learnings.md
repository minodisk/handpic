# Learnings: gpui-component-adoption

## Step 1: `init`, the theme, the key bindings

- `origin/main` had moved to `ec524107` (the bursts plan wrapped up in
  #1054) since the plan; `main.rs`'s `application().run` closure and the
  constant block were as the plan describes, so the edit there is two lines
  plus `mod theme;`.
- Neither `background` (`#0a0a0a`) nor the backdrops' `#00000099` had a
  constant in `main.rs` (the window root is black, the three backdrops wrote
  the overlay inline). `OVERLAY_COLOR` now lives in `main.rs` and the three
  backdrops and `theme.rs` read it (one source); `background` stays
  `theme.rs`'s own `BACKGROUND_COLOR`, which nothing else paints.
- The radii follow what the native app draws, not `style.css`: GPUI's
  `rounded_md` is 6 px (the buttons, the menus) and `rounded_lg` 8 px (the
  dialog boxes), so `radius` is 6 px and `radius_lg` 8 px. The settings
  modal's buttons and the text input use `rounded_sm` (4 px); a step that
  swaps them for components changes that by 2 px or sets the size per use.
- `Theme::update`'s write path reconciles `colors` into `tokens` and rebuilds
  the Base projection, so mapping `theme.colors` there is enough. The fields
  derived at load time from `primary` and friends (`button_primary`,
  `primary_hover`, `secondary_hover`, `list_*`, ...) are not recomputed when
  `primary` is edited afterwards: they keep `Default Dark`'s values. Step 3
  (the buttons) needs to map `button_primary*` / `button_secondary*` too, or
  check them against the current look.
- `ThemeColor` is `Default` and plain, so `map` is unit-tested without a
  window against the token hex values (not against the constants, which
  would be circular).
- The only binding outside a key context at 0.9.1 is the inspector's
  (`inspector.rs`); `command/state.rs`'s `ctrl-g` with `None` is inside its
  `#[cfg(test)]` module.

## Step 2: `Input` replaces `textinput.rs`

- `origin/main` at the start (`22d6e6a0`) had no other user of
  `textinput.rs`, so the step moved the settings modal's six fields (the
  five label names and the import pattern). The folder-dialogs plan's Step 7
  (the tree's inline rename, #1055) landed on `main` while the PR was open,
  with its editor on `textinput.rs`; the conflict resolution in this PR
  ported that editor in `crates/native/src/folderops.rs` to `InputState` /
  `Input` as well (`set_value` then `select_all`, `PressEnter` / `Blur`
  confirming, Escape canceling through an `on_key_down` on the wrapping
  `div`, since `Input` propagates an Escape it has no use for; the
  `PRIMARY_COLOR` border, a transparent fill, 13 px text, `px(3.)` /
  `py(px(1.))`). Nobody saw it on screen; the hand check is pending below.
  The folder-dialogs plan's Step 7 is done there, so its text, which still
  names `textinput.rs`, is left alone.
- The Escape path at 0.9.1: the `Input` context binds `escape` to
  `input::Escape`, whose handler on a single-line input (no
  `clean_on_escape`) ends in `cx.propagate()`. GPUI
  (`window.rs` `dispatch_key_event`) then runs the key-down listeners, so the
  key reaches `SettingsView::key_down` as `ModalKey::Close`; the commit was
  put there (the focused field), before `close`. No `on_action` was needed.
  `Enter` emits `PressEnter` and propagates; `tab` / `shift-tab`
  (`IndentInline` / `OutdentInline`) propagate on a single-line input
  (`apply_indent` returns early with `cx.propagate()`), so the modal's
  `ModalKey::Focus` still moves the focus. A propagated Enter / Tab / Escape
  is translated into a control character on Windows, which
  `parse_char_message` drops, so nothing is typed.
- The look is set per field with `Styled` refinements, which `Input`'s
  `render` applies after its own size and theme styling (`refine_style`
  comes after `input_h`, `input_px` / `input_py`, `input_text_size`, the
  `bg` / `rounded` / `border` of `appearance`). The focused border cannot be
  overridden that way (the focus ring is applied after the refinement, in
  `theme.ring`, `#737373`), so `focus_bordered(false)` drops it and the view
  picks the border color from the state's focus handle. `Input::h` is an
  inherent method for the multi-line height and shadows `Styled::h`, so the
  frame's height goes through `Styled::h(input, ..)`. The default `Medium`
  size would have been 32 px tall with 14 px text and 10 px padding at the
  window's 16 px rem (no `Root`, so `theme.font_size` does not set the rem).
- Step 1's note about the derived theme fields: `Input` reads none of
  `button_primary*` / `*_hover`; it reads `input` (for its border and, mixed,
  its fill, both overridden here), `ring` (dropped), `foreground`,
  `selection` and `caret`. The last two are `Default Dark`'s, unmapped.
- `set_value` always replaces the text, moves the caret to the end and
  clears the undo history, where the port's `set_text` was a no-op on the
  same text. A commit shows the normalized names back in every field, so
  `show` skips a field already showing its text.
- Right-click without `Root`: on Windows and macOS the component shows an
  OS-native menu (`native_menu/windows.rs`, no `WindowState`); on Linux the
  fallback looks up `Root`'s overlay through `WindowState::native_menu_overlay`
  (`Option`), finds none and shows nothing. The `deferred` the modal is drawn
  in is GPUI's, not gpui-component's `GlobalState` deferred context, so the
  menu is not suppressed there. `InputState` reaches `WindowState` nowhere
  else on these paths (no `WindowState::update` / `read` in `input/`).
- `unicode-segmentation` had no other direct user and is gone from
  `Cargo.toml`; it stays in `Cargo.lock` as a transitive dependency.

## Step 3: the settings modal's controls

- `origin/main` was at `6ff22c4c` (Step 2's #1059), as the branch.
- The Base `Button`, `Checkbox` and `Radio` under the components are `div`s
  with `on_click`, so Enter and Space on a focused one are GPUI's keyboard
  click (`div.rs`), the same path the hand-drawn `div`s took; the box's
  bubble-phase `key_down` runs after it. Each control is a tab stop at
  `tab_index(0)` by default with its own keyed focus handle, so the cycle is
  the same stops in the same order. The `Radio`s stay standalone:
  `RadioGroup` wraps them in the Base radio group with arrow-key roving,
  which would change the cycle.
- Behavior difference found and kept out: every component control calls
  `window.prevent_default()` on a left mouse down, so a pointer press does
  not focus it, nor (the default being prevented) the box behind it. The
  hand-drawn `div`s took the focus on a press, which blurred a focused field
  and committed it. Without that, a name typed and followed by a click on a
  format radio, a language, `Reset`, `Choose…` or an example pattern would
  not commit until a later blur. `leave_field` (called first in those
  handlers) focuses the box when a field has the focus, and the field
  commits on its `Blur` as before. Where the focus ends differs: the box,
  not the pressed control, so a Tab after a click starts from the box's
  first stop. Keyboard activation cannot happen with a field focused, so
  the call is a no-op there.
- Step 1's note on the derived theme fields mattered: `Button`'s default
  variant reads `tokens.button` / `button_hover` / `button_active` and
  `button_foreground`, which `Default Dark` derived at load. `theme.rs` now
  maps them (`BUTTON_FILL`, `ACCENT_COLOR` twice, `FOREGROUND_COLOR`);
  `Theme::update`'s `reconcile` carries `colors` into `tokens`, so setting
  the `ThemeColor` fields is enough. The border is `theme.input`
  (`INPUT_BORDER`, already mapped). The ghost close `×` hovers at
  `accent` at 50 % in dark mode (not mapped; the old one was full
  `ACCENT_COLOR`).
- Sizes: `Button::small()` is `h_6` (24 px at the window's 16 px rem, no
  `Root`) and `px_2`; the instance style refines last, so `px(6.)` and
  `rounded_sm` override. The label's size cannot be refined from outside:
  the content `h_flex` sets `text_sm` (14 px) itself and `content_style` is
  `pub(crate)`, so the label is a child `div` at 13 px rather than
  `.label(..)`. `Checkbox` / `Radio` `.small()` give a 14 px indicator (as
  the old box) and `text_size(px(13.))` refines over their `text_sm`.
- Look differences recorded, not fixed: a checked `Radio` is the
  component's filled `primary` circle with a check mark (the old one was a
  ring with a 6 px dot); the unchecked checkbox and radio have the
  `input_background()` fill (white 15 % at 30 %); the focus shows as the
  theme's `ring` border plus a ring outside the control (`focus_ring` is on
  in `Default Dark`), on any focus rather than only `focus_visible` (a
  pointer press does not focus a control, so in practice it is the keyboard
  focus). The ring of a row at the scrolling list's edge can be clipped by
  the scroll area.
- Icons: the checkbox's check and the checked radio's mark are
  `icons/check.svg`. `Icons` now falls back to `gpui_kit_assets::Assets`
  (`load` maps its not-found error to `None`; `list` joins both,
  deduplicated), the app's own `icons/star.svg` / `icons/square.svg` winning
  over the same paths in `Assets`. `gpui-kit-assets` was already in
  `Cargo.lock` at 0.9.1 on the same `gpui-pre` `=0.3.12`, so the lock gained
  only the direct edge.
- `ScrollableElement::overflow_y_scrollbar()` moves the list's size
  (`flex_1`, `min_h_0`) onto a wrapper and gives the list a new element id
  under it; the scrollbar's mode is the theme's default
  `ScrollbarMode::Scrolling` (shown while scrolling, fading out when idle),
  left as it is. Nothing scrolled a focused row into view before, so
  nothing does now.
- The tab list reads `cx.theme()`'s `border`, `foreground`,
  `muted_foreground`, `accent` and `primary`, the same values as the
  constants it read.

## Step 4: the look of the Sort / Filter menus and the right-click menus

- `origin/main` was at `26e4e15d` (Step 3's #1061), as the branch.
- The rows follow `popup_menu.rs`'s `render_item` and `menu_item.rs` at
  0.9.1: `px(0.5 rem)`, `py_0`, `h(1.625 rem)` (26 px at the window's 16 px
  rem; `Small` is 20 px with half the radius), `rounded(theme.radius)`,
  `gap_x_1`, a 12 px (`xsmall`) icon column holding `IconName::Check` or
  `Icon::empty()`, the list `p_1` with `gap_y_0p5` between rows, hover
  `accent` / `accent_foreground`, a disabled row in `muted_foreground` with
  no hover (the old one was at 50 % opacity), the key binding at the row's
  right end after `justify_between` / `gap_3` in `muted_foreground` at
  `text_xs`, and the separator a 2 px `border_b` in `border` at `my_0p5`,
  `mx_neg_1`. The text stays 13 px (`render_item` sets `text_sm`, 14 px).
  The rows are taller than before (26 px against about 22), so the Filter
  menu reaches its height cap sooner.
- `popover_style` was not used for the surface: its edge is a ring shadow in
  `foreground` at a fixed alpha plus `shadow-md`, not `theme.border`; the
  Done-when names `border`, so the surface is `popover` / `popover_foreground`
  / a 1 px `border` / `radius` set by hand.
- The scrollbar: `overflow_y_scrollbar()` (`Scrollable`) puts the caller's
  element inside a `size_full` wrapper and makes it the scrolled content, so
  the `occlude()` / `on_mouse_down_out` / border would be on the content,
  which is as tall as all the rows (`on_mouse_down_out` tests `hitbox`
  bounds, so a press below the visible menu but inside the content would not
  count as outside) and scrolls with them. `PopupMenu`'s own layout was taken
  instead: the surface stays the outer element, the row list inside it is
  `overflow_y_scroll().track_scroll(&handle)` with the max height, and the
  surface carries `vertical_scrollbar(&handle)` (an `absolute().inset_0()`
  overlay with no hitbox of its own besides the bar's). The press on the bar
  is inside the surface, so it is not a press outside.
- The `ScrollHandle` has to outlive a frame, so it lives on the view
  (`menu_scroll`, `context_scroll`). `window.use_keyed_state` would have
  needed `window` threaded into `status_row` / `menu_button`, the latter
  then at clippy's `too_many_arguments` limit. Before, the offset was GPUI's
  element state, dropped while the menu was not drawn, so a menu reopened at
  its top; the handle is replaced on every press of a menu's button and on
  every right-click open to keep that.
- The scrollbar's mouse-down listener (`gpui-base`'s `scrollbar.rs`) stops
  propagation in the bubble phase, and it is registered after (so runs
  before) the popover's own bubble listeners. The right-click popover's
  `on_any_mouse_down`, which hands the keyboard back from the tree, became
  `capture_any_mouse_down` so a press on the bar still does that. The
  Sort / Filter popover has no bubble listener of its own.
- Wheel scrolling and the occlusion are unchanged in kind (the row list is
  the scroll area as before); dragging the thumb runs through the bar's own
  `window.on_mouse_event` listeners. Neither was seen on screen here.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, Windows, the user's). Basis: Step 1's
  Done-when (the debug build's keys, `--auto`, and "nothing differs from
  `main`"); the step's checkbox was ticked on the automated criteria
  (`mise run ci:local`, the `theme.rs` unit test). Related files:
  `crates/native/src/main.rs`, `crates/native/src/theme.rs`.
  - Build: `cargo build --manifest-path crates/native/Cargo.toml` (rename a
    locked `riffle-native.exe` aside first; do not kill it).
  - Launch on a copy:
    `crates/native/target/debug/riffle-native.exe --index <fresh temp>/index.sqlite --settings <test>/settings.sqlite D:\Photos\tests\20261114-gpui-component-adoption\<copy>`.
  - Steps: page with the arrows; press `tab` (the side panes toggle), `f`,
    `z`, `e`, `d`; open and close the Sort / Filter menus; open the settings
    modal with `ctrl+,` and type a label name with the Japanese IME; open a
    right-click menu on the strip and on the tree; open and cancel the trash
    dialog; press `ctrl+shift+i` (the component inspector opens).
  - Expected: everything looks and behaves as on `main`; only
    `ctrl+shift+i` is new.
  - `--auto`: run the same command with `--auto 20` once and compare its
    timing lines with a `main` build's; they are the same in kind (not the
    A/B gate). If they visibly differ, record it here.
- Pending manual check (Step 2, Windows, the user's). Basis: Step 2's
  Done-when (each use site behaves as before, the fields look like the
  port's, the IME); the step's checkbox was ticked on the automated
  criteria (`mise run ci:local`; the crate has no GPUI view test). Related
  files: `crates/native/src/settingsview.rs`, `docs/agents/native-app.md`.
  - Build: `cargo build --manifest-path crates/native/Cargo.toml` (rename a
    locked `riffle-native.exe` aside first; do not kill it).
  - Launch on a copy:
    `crates/native/target/debug/riffle-native.exe --index <fresh temp>/index.sqlite --settings <test>/settings.sqlite D:\Photos\tests\20261114-gpui-component-adoption\<copy>`.
  - Steps and expected results:
    - Open the settings with `ctrl+,`, Sidecar tab, a format writing XMP:
      the five name fields show the stored names, 1 px white-15 % border,
      no fill, the same height and text size as `main` within a pixel or
      two; the focused one has the `#e5e5e5` border and no extra ring.
    - Type a label name with the Japanese IME: the composition is
      underlined, the candidate window sits at the caret, Enter inside the
      composition commits the composition and not the field.
    - Commit with Enter, with a click elsewhere in the box and with Tab
      (Tab and Shift+Tab move between the modal's stops from inside a
      field); the names are written (reopen the modal: they are shown).
    - Escape in a field closes the modal with the edited name kept.
    - `Reset` and a language pick show their names in the fields.
    - Import tab: type a pattern with and without the IME; the example line
      follows each key; an invalid pattern shows its refusal in red on
      commit; an example pattern button fills the field and its example.
    - `ctrl+a` then typing replaces the text; `ctrl+c` / `ctrl+x` /
      `ctrl+v` work; `ctrl+z` undoes (new).
    - Right-click in a field: no panic; the OS-native Cut / Copy / Paste /
      Select All menu shows and its items work.
    - A key typed in a field (e.g. `f`, `z`, `d`, the arrows) acts in the
      field only, not on the window behind.
    - After closing the modal the arrows page again.
- Pending manual check (Step 2 catch-up, the tree's inline rename,
  Windows, the user's). Basis: the folder-dialogs plan's Step 7 (#1055)
  landed while Step 2's PR was open, and the conflict resolution ported its
  editor to `InputState` / `Input` without seeing it on screen (see Step 2
  above); Step 2's checkbox was ticked on the automated criteria. Related
  file: `crates/native/src/folderops.rs` (`tree_name`, `start_rename`,
  `rename_event`).
  - Build and launch as in the Step 2 check above, on a copy under
    `D:\Photos\tests\20261114-gpui-component-adoption\` with a fresh
    `--index` and a test `--settings`.
  - Steps and expected results:
    - Start a rename from the tree (a slow second click on the open
      folder's name, or the right-click `Rename…`): an editor replaces the
      name with the whole name selected, a `#e5e5e5` border, no fill,
      13 px text, fitting the row's height.
    - Type a new name with the Japanese IME: the composition is underlined,
      the candidate window sits at the caret, Enter inside the composition
      commits the composition and not the rename.
    - Commit with Enter: the folder is renamed and the tree has the
      keyboard again. Start another and commit with a click elsewhere in
      the window: renamed as well.
    - Start another and press Escape: nothing is renamed, the name comes
      back, the tree has the keyboard.
    - Switch to another app with the editor open: no rename until the
      window comes back.
- Pending manual check (Step 3, Windows, the user's). Basis: Step 3's
  Done-when (the keyboard, the look, the scrollbars); the step's checkbox
  was ticked on the automated criteria (`mise run ci:local`, the
  `settingsmodal.rs`, `theme.rs` and `icons.rs` tests; the crate has no GPUI
  view test). Related files: `crates/native/src/settingsview.rs`,
  `crates/native/src/theme.rs`, `crates/native/src/icons.rs`.
  - Build and launch as in the Step 2 check above.
  - Steps and expected results:
    - Open the settings with `ctrl+,`: the tab the modal was left on is
      focused. Walk every tab with the keyboard only: Tab and Shift+Tab
      walk the tab, then each control in order (the format radios, the name
      fields, the languages and `Reset`; auto-advance; each row's chips'
      `×`, `+`, `Reset` and `Reset all`; the root's `Choose…`, the pattern,
      the example patterns; the close `×`) and wrap. ArrowUp / ArrowDown /
      Home / End on the focused tab select and focus another.
    - Space and Enter on each control kind press it: a button, the
      auto-advance checkbox (flips; the check mark draws), a format radio
      (switches; the folder re-lists).
    - Flip auto-advance with the mouse; switch the sidecar format with the
      mouse.
    - Type a label name, then click a format radio, a language or `Reset`
      without pressing Enter: the typed name is committed first (reopen the
      modal to see it stored). Same in the Import tab: type a pattern, then
      click `Choose…` or an example pattern.
    - Keyboard Shortcuts tab: `+`, then `tab`: added as a chip; `+`, then
      `enter`, likewise; `+` then `space`; a chip's `×` removes it;
      `+` then `home` shows the refusal in the status line; `Reset` and
      `Reset all` restore.
    - Scroll the Shortcuts list and the Import tab (make the window short)
      with the wheel: a scrollbar shows while scrolling; drag the
      scrollbar's thumb: the list follows.
    - Look: the outlined buttons are about as on `main` (24 px tall, 13 px
      labels, a faint fill, the white-15 % border, `#262626` on hover), the
      chosen language lit with the `#e5e5e5` border; a focused control
      shows the `#737373` ring; the checked radio is a filled circle with a
      check (new).
    - Click the backdrop: the modal closes. Reopen and click the close `×`:
      it closes. After closing, the arrows page again.
- Pending manual check (Step 4, Windows, the user's). Basis: Step 4's
  Done-when (the four menus' look and the behavior list by hand); the step's
  checkbox was ticked on the automated criteria (`mise run ci:local`, the
  `menu.rs` and `context.rs` unit tests unchanged; the crate has no GPUI view
  test). Related file: `crates/native/src/main.rs` (`popover`,
  `context_popover`, `menu_surface`, `menu_separator`, `menu_item`,
  `menu_row`, `context_row`, `menu_pressed`, `tree_context`,
  `strip_context`).
  - Build and launch as in the Step 2 check above, on a copy under
    `D:\Photos\tests\20261114-gpui-component-adoption\` with a fresh
    `--index` and a test `--settings`, on a folder with bursts and EXIF
    variety so the Filter menu is long.
  - Look: open the Sort menu, the Filter menu, the tree's right-click menu
    and the strip's: each is the `#171717` surface with the white-10 %
    border and 6 px radius, 26 px rows with 6 px rounded `#262626` hover,
    the component's check icon at the left of a checked item, the headings
    muted and indented to the labels, the separators across the menu's
    padding, the strip menu's shortcuts at the right end in muted 12 px, a
    disabled item muted with no hover; they read as the same family as the
    settings modal's controls.
  - Behavior, each line by hand:
    - The Sort menu opens above its button with the right edges aligned;
      the Filter menu likewise. A right-click menu opens at the pointer and
      flips near the window's right and bottom edges, staying inside.
    - An outside press closes an open Sort / Filter menu: in the strip, in
      the preview, and on the menu's own button (which does not reopen it).
    - A Sort / Filter item click applies (the strip reorders or refilters)
      and leaves the menu open.
    - With the Filter menu open: `escape` closes it; the arrows page; `e`,
      `z`, `f` toggle their overlays; the burst keys act; the menu stays
      open through all but `escape`.
    - Make the window short so the Filter menu exceeds its cap: it scrolls
      by the wheel, a scrollbar shows while scrolling, dragging the thumb
      scrolls it, and a press on the scrollbar does not close the menu.
      Close and reopen it: it opens at its top.
    - A right-click menu closes on any key; `escape` stops there, another key
      (e.g. an arrow) also acts. An outside press closes it; a right press on
      another row opens that row's menu.
    - Click in the tree (it takes the keyboard), right-click a folder: the
      tree keeps the keyboard while the menu is open; press on the menu (an
      item, or its scrollbar in a short window): the tree hands the keyboard
      back (the arrows page the photos, the cursor outline goes).
    - With a menu open, nothing takes the GPUI focus: closing it leaves the
      keys working on the window as before.
