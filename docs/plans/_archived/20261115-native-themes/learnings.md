# Learnings: native-themes

## Step 1

- The burst badge's text (`paint_burst_badge`) was `FOREGROUND_COLOR`, but
  it is drawn over the photo on a black halo like the other marks, so it did
  not move to the theme's `foreground` (a light theme would put a dark text
  on the dark halo). It is the fixed `BADGE_COLOR` (`#fafafa`) instead.
- The root `div`'s black was the preview's surround as well as the strip's
  and the status row's surface. To keep the preview black in every theme
  while the rest follows `Roles::surface`, the preview's container got an
  explicit `PREVIEW_BACKGROUND` fill; identical pixels today.
- `modal.rs` already read every color from `cx.theme()`; it needed no change.
- `Roles::of`'s non-Neutral branch (the derivation from the tokens) is
  written and unit-tested now but only reached from Step 2 on.
- A `cx.theme()` borrow held across a `cx.listener(..)` does not compile;
  copy the `Hsla` values out first (they are `Copy`). In `meta_pane` a local
  named `error` shadowed the color, so the danger color is `danger`.
- The filter menu's empty star was `MUTED_COLOR` at alpha `0x66`; it is now
  `muted_foreground.alpha(0.4)` (102 / 255 = 0.4, the same 8-bit value).

## Step 2

- `TitleBar` is not on this branch yet (the peer's work had not landed in
  `main.rs` when the step was implemented); both palettes map `title_bar` /
  `title_bar_border` (light: `#ffffff` / `#e5e5e5`, as the panes) and the
  light mapping test asserts them, so it reads the theme in either mode once
  it lands.
- The shadcn/ui Neutral light values were checked against
  `apps/v4/public/r/colors/neutral.json` (oklch) and converted to Tailwind's
  hex. `Default Light` differs from them in `secondary` (`neutral-200`), `ring`
  (`#a3a3a3`), `danger`, `sidebar` (`#fafafa`) and `title_bar` /
  `status_bar` (`#F8F8F8`), so the light palette pins every field the dark
  one pins, not only the differences: one `map` over a `Palette` per mode.
- Light choices made by analogy with the dark map, not given by shadcn: the
  backdrop `overlay` kept at black 60 % (Default Light's `#0000000d` barely
  dims), the outline button's fill `#ffffff` (`bg-background`), the
  destructive hover `#e7000b` at 90 % over white (`#e91a23`), the sidebar
  and the status / title bars on the panes' `popover`, the scrollbar thumb
  on `border` (faint on white, as it is faint on the dark panes).
- Neutral light's `Roles` are the derived ones (`Roles::of(.., false)`): the
  window surface is `background` `#ffffff`, the same as the panes, so only
  the borders separate them.
- `--auto` reads and writes `theme` like the modal's other settings
  (`autoAdvance`, `sidecarFormat`), unlike the panes and the session keys.
- `Theme::change` always reloads the mode's registered config
  (`reload_mode`), so re-applying in the same mode (an appearance callback
  that changes nothing) is harmless and restores the map exactly.
- The Appearance page is appended after Import, so the Tauri pages keep
  their order and every open still starts on Sidecar.

## Step 3

- Vendored from `longbridge/gpui-kit` `themes/` at tag `v0.9.1`, commit
  `39ab849a93b5f15507b969164d49891db8d5b470`, which is the `sha1` in the
  published crate's `.cargo_vcs_info.json` (`path_in_vcs`
  `crates/component`). 21 files, byte-for-byte (fetched from
  `raw.githubusercontent.com` at that commit), 36 themes: 11 light, 25
  dark. No name repeats and none is
  `Neutral`. No file sets `font.size`, `radius` or `radius.lg`; Hybrid,
  macOS Classic, Mellifluous and Molokai set `shadow`.
  `Theme::apply_config` writes `shadow` (and `highlight`) only when the
  config sets it, so `apply` resets `theme.shadow = true` before `change`;
  otherwise Neutral after Molokai would keep `shadow` false. `highlight`
  is not reset: the app draws no code view.
- Origin: the files carry no license of their own; upstream ships them in
  the Apache-2.0 repository (`LICENSE-APACHE`, Copyright 2024 - 2026
  Longbridge). Each file's `author` / `url` names the scheme it adapts:
  iTerm2-Color-Schemes (Adventure, Fahrenheit, Harper, Kibble), Zed
  Industries' Ayu, Catppuccin, Everforest, Flexoki, Gruvbox, vim-hybrid,
  Jellybeans, Alduin, Asciinema, macOS Classic (huacnlee's Zed theme),
  Mellifluous, Molokai, Solarized, Spaceduck, Tokyo Night, Twilight
  (MacroMates) and Longbridge's own Aurora. The upstream schemes' own
  licenses were not checked one by one; `crates/native/themes/LICENSE`
  records the source, the per-file author / url table and the Apache-2.0
  text.
- `Theme::change` reloads the config in the mode's `light_theme` /
  `dark_theme` slot, not the registry's default, so once a shipped theme
  sits in a slot, going back to Neutral must put `Default Light` / `Dark`
  back in the slot first. `apply` therefore always sets both slots in one
  `update`, then `change`s, then maps in a third `update` only for Neutral.
- `ThemeConfig` is `Send + Sync`, so the parsed configs live in a
  `LazyLock` static and `Setting` keeps `&'static str` names and stays
  `Copy`; `apply` wraps a clone of the config in a fresh `Rc` each time.
- The settings modal already had a `Select` (the label names' language), so
  the pickers are two more `Select`s; the binding table only needed its
  wording widened.

## Deferred issues (todo candidates)

- Pending manual check (Step 2, native-themes; the step's checkbox was
  ticked on the automated criteria): on Windows, macOS and Linux, open the
  settings (`ctrl+,` / `meta+,`), Appearance, choose Light and check by eye
  that the panes, the strip, the status row, the menus, the dialogs and the
  settings box turn light and stay readable: the strip's stars (`#ffd050`)
  and the pick green on the light strip, the tree cursor's ring, the menus'
  hover, the progress bar, and the `TitleBar` once it lands. Then choose
  System and switch the OS appearance while the app runs: the app should
  follow it without a restart; choose Dark and it should stay dark whatever
  the OS. Restart: the choice should be restored before the window is drawn.
  Record what is hard to read in this file (the fixed judgment colors are
  not retuned in this plan). Files: `crates/native/src/theme.rs`,
  `crates/native/src/main.rs`, `crates/native/src/settingsview.rs`.
- Pending manual check (Step 3, native-themes; the step's checkbox was
  ticked on the automated criteria): on Windows, macOS and Linux, open the
  settings (`ctrl+,` / `meta+,`), Appearance. In Dark mode pick at least
  one shipped dark theme (e.g. Tokyo Night, Catppuccin Mocha) from `Dark
  theme`, and in Light mode one shipped light theme (e.g. Catppuccin Latte,
  Solarized Light) from `Light theme`: the panes, the strip (names, burst
  band, reject dim), the status row, the menus, the dialogs, the settings
  box and the `TitleBar` should take the theme at once and stay readable,
  the preview's surround staying black. Check the derived `Roles` on both
  (the strip's file names and failures, the burst band, the dimmed reject)
  and the fixed stars / pick / reject colors on each. Pick Neutral again:
  the app should look exactly as before (Neutral dark unchanged). In System
  mode with two non-Neutral themes chosen, switch the OS appearance: the app
  should swap between them. Restart: the choice is restored. Also check the
  pickers' keys: Space / Enter open the menu, the arrows move, Enter picks,
  Escape with the menu open closes the menu only, with it closed closes the
  settings. Record what is hard to read in this file. Files:
  `crates/native/src/theme.rs`, `crates/native/src/settingsview.rs`.
- Check the upstream licenses of the shipped themes before packaging the
  native app. The 21 vendored files in `crates/native/themes/` carry no
  license of their own; upstream ships them under Apache-2.0, but each
  adapts a third-party scheme (iTerm2-Color-Schemes, Zed's Ayu, Catppuccin,
  Solarized, Tokyo Night, Twilight / MacroMates and others) whose own
  license was not checked one by one. `crates/native/themes/LICENSE` holds
  only the source, the author / url table and the Apache-2.0 text, and
  `docs/agents/native-app.md`'s license section does not mention the gap.
  Change: check each adapted scheme's license, add any missing attribution
  to `LICENSE` or drop a theme whose license is incompatible, and record the
  result in the guide's license section. Done when every file in
  `crates/native/themes/` has a checked upstream license noted in `LICENSE`
  and the guide's license section states the result.
