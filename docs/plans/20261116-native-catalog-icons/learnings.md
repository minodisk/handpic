# Learnings

## Step 1

- `icon_assets!` needs no import beyond the macro path: it resolves
  `AssetSource`, `Result` and `SharedString` through
  `gpui_kit_assets::__private`. The struct it generates is private when no
  visibility is given, which is what `icons.rs` wants.
- `IconName::path` is an inherent method, so the test calls `name.path()`
  without importing the `IconNamed` trait.
- The catalog's SVG files end with `</svg>` and a newline, so the every-path
  test trims trailing whitespace before checking the end, and checks the
  start with `<svg` (no trailing space).

## Deferred issues (todo candidates)

- **Pending manual check (Step 1, the user's)**: in the running native app
  (`mise run native:dev`, Windows, plus any other platform at hand), open the
  status row's `Filter ▾` menu and confirm the star rows (outlined and
  filled), the label squares (outlined and filled), the orientation
  rectangles (vertical, horizontal) and the `Face focus` faces (smiling and
  frowning) look as before; then confirm the strip's face focus icon on a
  good frame and on a low frame looks as before. Expected: no visible change
  from `main` (the catalog SVGs differ only in whitespace and
  `width` / `height`). Basis: the plan's Done-when hand check for Step 1;
  files `crates/native/src/icons.rs`. The step's checkbox was ticked on the
  automated criteria (tests, clippy, `mise run ci:local`).
