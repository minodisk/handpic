# Learnings

## Step 1

- gpui-component 0.9.1 has a public `Theme::set_scrollbar_mode`, but it runs
  its own `Theme::update`; setting `scrollbar_mode` inside `apply`'s second
  `update` (now `theme.rs`'s `finish`) avoids an extra refresh.
- `Theme::default()` and `Theme::apply_config` need no `App`, so the test
  runs `finish` on a plain `Theme` (Trade-offs 3, the proposed form). It
  proves the edit leaves `Always` after a vendored config, not that `apply`
  calls `finish`.
- In `ScrollbarMode::Always`, gpui-base 0.9.1 still skips a bar whose
  content fits (`scroll_area_size <= scroll_viewport_size` in the
  scrollbar's prepaint), so "hidden where the content fits" holds by
  construction.

## Deferred issues (todo candidates)

- **Pending manual check** (Windows, macOS and Linux; the agent has no
  screen; the step's checkbox was ticked on the automated criteria). Basis:
  plan Step 1's on-screen checks, `crates/native/src/theme.rs` (`finish`),
  `docs/agents/native-app.md` ("The strip, the tree and the meta pane draw
  the theme's scrollbar"). Steps and expected results:
  1. Open a folder with more files than the strip holds, expand the tree
     past the pane's height, show a file whose meta pane overflows, and open
     a dialog whose body overflows (e.g. Import… on a large folder). Each of
     the four bars (tree, strip `StripBar`, meta pane, dialog body) is
     visible at rest, without scrolling first.
  2. Drag each thumb and click each track without scrolling first; the view
     follows (the strip's range thumbnails load as on a wheel scroll).
  3. Where the content fits (a folder with few files, a collapsed tree), no
     bar is drawn and presses reach the strip cells and tree rows as before.
  4. The strip's bar lies over its bottom edge and the tree's over the
     rows' right 16 px; a press there hits the bar, not a cell or row
     (accepted trade-off; note whether padding is wanted).
  5. The bar's look is acceptable in Neutral dark, Neutral light and one
     vendored theme (e.g. Catppuccin Mocha), switching on the Appearance
     page and, in System mode, on an OS appearance change.
