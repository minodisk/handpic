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
