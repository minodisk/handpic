# Learnings: native-label-tags

## Step 1

- `ColorName` is re-exported at `gpui_kit::component`'s root (through
  `theme::*`), `Tag` at `gpui_kit::component::tag::Tag`; `Sizable` was
  already imported for `.small()`.
- The strip menu's label items are told apart by a new `context::Item` field,
  `tag: Option<Option<Tint>>`, set by `label_tag` from the keymap action
  name: `clearlabel` gives `Some(None)`, a name `Tint::of` maps to a color
  gives that tint, anything else (`Tint::Other`) none. This reuses
  `Tint::of`'s name list instead of repeating it.
- The filter menu draws its label Tag from the `(Label::Text, Lead::Label)`
  pair in `menu_row`; an `if let` match guard was avoided (not stable on
  every toolchain the workspace builds with), a tuple match does the same.
- The mapping function was renamed `tint_color` to `label_color` (the plan
  asks that `tint_color` be gone) and now returns a `ColorName`; the band
  paints `label_color(tint).scale(LABEL_BAND_SCALE)` with
  `LABEL_BAND_SCALE = 600`.
- The square icons were removed along with their tests' entries and the
  header comment's Feather line without first confirming the unused-item
  lint fires; the plan lists them as orphans either way.
  `crates/native/LICENSE-lucide` still lists `square` among the
  Feather-derived icons, but that is the upstream notice's full list, not a
  per-icon claim, so it was left as is.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, native app, Windows and any other desktop
  platform at hand): launch `crates/native`, open the status row's
  `Filter ▾` menu and a strip cell's right-click menu, and label a file, in
  a dark theme (Neutral dark) and a light theme (Neutral light). Expected:
  the seven color label rows and `No label` in both menus are small Tags
  holding the name, not clipped, the row height still `MENU_ROW` (26 px),
  the Tag readable while the row is hovered (accent background), the
  filter popover wide enough for its rows; a labeled strip cell's name band
  filled with the label's 600 scale color with the name readable, yellow
  above all. The step's checkbox was ticked on the automated criteria
  (`mise run ci:local`); the plan asks for what was seen to be noted in the
  PR description. Basis: plan Step 1's last Done-when item. Files:
  `crates/native/src/main.rs` (`menu_row`, `context_row`, `label_tag`,
  `paint_strip`).
