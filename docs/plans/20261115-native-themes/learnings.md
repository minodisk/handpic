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
