# Learnings: native-component-controls

## Step 1

- The old `Sort ▾` / `Filter ▾` `div` was 21 px tall, not "about 21": GPUI's
  default line height is `phi()` (1.618), and `TextStyle::line_height_in_pixels`
  rounds it, so a 12 px line is 19 px, plus the 1 px border on each side.
  `Button::xsmall()` is `h_5` (20 px), so the height is pinned to 21 px
  (`MENU_BUTTON_HEIGHT`) to keep the status row and the 842 px canvas. Read from
  `gpui-pre` 0.3.12 `style.rs`, not re-measured on screen.
- `Button`'s `render` calls `.hover(..)` after the caller's style, and GPUI's
  `hover` replaces the whole hover refinement, so a caller cannot keep its own
  border on hover. A lit `Filter ▾` therefore shows the `input` border while
  hovered; its label child keeps `primary`, since a child's own text color is
  not the button's hover color.
- `Button`'s caller style is replayed inside its disabled state
  (`styles(|s| s.disabled(.. refine_style(&instance_style)))`), so the view-only
  sort button can be `.disabled(true)` (no hover, no press) with the normal fill
  and border set back by the caller, then `opacity(0.5)`: the `div`'s dimmed
  look, closer than the component's disabled colors.
- `InputGroup` sets its fill before `refine_style`, so the caller's
  `.bg(transparent_black())` wins; its focused `ring` border is set after and
  stays the component's.
- `SelectEvent` has one variant, so matching `Confirm(Some(name))` needs an
  `if let` (edition 2024's let chain reads cleanly with the preset lookup).
- `choose_root` now runs in `cx.spawn_in(window, ..)`: the root field's
  `InputState::set_value` needs a `Window`, which a plain `cx.spawn` does not
  give.
- `Kbd::format` is a pure `pub fn`, so its spelling of the keymap's names is
  covered by a unit test in `keys.rs` without GPUI test support (non-macOS
  only, since macOS spells with symbols).

## Deferred issues (todo candidates)

- **Pending manual check (Windows, `mise run native:dev`): Sort / Filter
  buttons.** Steps: open a RAW folder; press `Sort ▾`, then press it again with
  the menu open; set a filter; open a JPEG-only folder. Expected: a press
  toggles the menu; a press on the open menu's own button closes it without
  reopening; `Filter ▾` shows the `primary` border and label while a filter is
  on (the border goes to `input` while hovered, see above); the view-only sort
  button is dimmed, does not hover and does not open; the popover opens upward
  from the button's top-right corner; the preview canvas stays 1160 x 842 in
  the default window. The step's checkbox was ticked on the automated criteria
  (clippy, the unit tests). Basis: plan Step 1 "Hand checks"; files
  `crates/native/src/main.rs` (`menu_button`).
- **Pending manual check (Windows, `mise run native:dev`): the settings
  controls.** Steps: open the settings (`ctrl+,`) and Tab / Shift+Tab through
  every page; on Culling press Space and Enter on the auto-advance switch; on
  Sidecar (XMP on) open the language menu by pointer and by keys (Space /
  Enter, Up / Down), press Escape with the menu open, then with it closed,
  choose a language and press `Reset`; type in a name field then open the
  language menu (the field should commit on blur); on Import press `Choose…`,
  then select and copy the root path in its field and press Escape in it; on
  Keyboard Shortcuts press `+` then a key, and remove a key with its `×`.
  Expected: the focus stays inside the settings on every Tab, including with
  the language menu open; Escape with the menu open closes only the menu, with
  it closed closes the settings; `Reset` fills in the chosen language's names;
  the root path is selectable and copyable and not editable; the capture takes
  the next key; a name or pattern field still commits on Enter, blur and
  Escape. The step's checkbox was ticked on the automated criteria. Basis: plan
  Step 1 "Hand checks"; file `crates/native/src/settingsview.rs`.
- **Pending manual check (Windows, `mise run native:dev`): the look of the
  new controls.** Check that the off switch and its thumb (`Default Dark`'s
  `switch` / `switch_thumb`, unmapped in `theme.rs`) read clearly against the
  settings' `#171717`, and that the `Kbd` chips (13 px `foreground` on
  `muted`) and the right-click menu's `Kbd` shortcuts (`Ctrl+Alt+1`, `Space`,
  right-aligned in `muted_foreground`) look right. Expected: if the off switch
  is hard to read, map `switch` / `switch_thumb` in `theme.rs` with the theme
  test (that conflicts with `20261115-native-themes` Step 1's rewrite of
  `map`). Basis: plan Step 1 "Switch" and "Kbd"; files
  `crates/native/src/settingsview.rs`, `crates/native/src/main.rs`
  (`context_row`), `crates/native/src/theme.rs`.
