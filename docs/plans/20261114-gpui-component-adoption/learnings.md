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
  `textinput.rs`: the folder-dialogs plan's Step 7 (the tree's inline
  rename) has not landed, so only the settings modal's seven fields moved.
  That plan's Step 7 text still names `textinput.rs`; it needs its owner's
  one-line edit (see the deferred issue below), and the PR description says
  so.
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
- The folder-dialogs plan's Step 7 (the tree's inline rename) is written
  against `textinput.rs`, which Step 2 deleted. Basis: Step 2's Done-when
  and Trade-off 1 (decided 2026-11-14): that plan's owner edits the step's
  text to "the editor is an `InputState` / `Input` (`set_value`,
  `select_all`, `PressEnter` / `Blur` as the commit)". Related files:
  `docs/plans/20261113-gpui-folder-dialogs/plan.md` (Step 7, and the
  mentions at its "textinput.rs" trade-off),
  `crates/native/src/settingsview.rs` (the pattern to follow).
