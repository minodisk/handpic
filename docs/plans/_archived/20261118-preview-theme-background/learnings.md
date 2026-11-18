# Learnings

## Step 1

- The pick per shipped theme (from `Theme::apply_config`): `status_bar` and
  `sidebar`, absent from most files, come from gpui-component's defaults, which
  derive from the theme's own colors, so none is unrelated and all five
  candidates stay. Catppuccin Mocha gives `#11111b` and Jellybeans `#101010`
  (their `title_bar`); light themes mostly get their `title_bar` or
  `status_bar` (macOS Classic Light `#e9e9e9`, Aurora Light `#f8fafc`); many
  dark themes keep `background` (Ayu Dark, Gruvbox Dark, Solarized Dark).
- Every shipped theme's five candidates are opaque, so the translucency skip
  only guards the derivation.
- `Hsla::to_rgb()` (gpui-pre's `color.rs`) gives the `Rgba` the luminance is
  computed from; gpui_kit re-exports `Hsla`, and the method needs no import.
- The binary target is `handpic` (`cargo test --bin handpic` in
  `crates/native`), not `handpic-native`.
