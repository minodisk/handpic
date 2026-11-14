# Learnings: gpui-component-adoption

## Step 1: `init`, the theme, the key bindings

- `origin/main` had moved to `ec524107` (the bursts plan wrapped up in
  #1054) since the plan; `main.rs`'s `application().run` closure and the
  constant block were as the plan describes, so the edit there is two lines
  plus `mod theme;`.
- Neither `background` (`#0a0a0a`) nor the backdrops' `#00000099` had a
  constant in `main.rs` (the window root is black, the three backdrops write
  `rgba(0x00000099)` inline). `theme.rs` holds them as `BACKGROUND_COLOR` and
  `OVERLAY_COLOR` rather than adding constants to `main.rs`; the inline
  backdrop literals stay until the dialogs move to `Dialog` (Steps 6-8).
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
