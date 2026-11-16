<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Color labels as gpui-component Tags (native app)

## Purpose

The GPUI app (`crates/native`) shows a color label in three places with its
own fixed Lightroom-like colors (`tint_color` in `main.rs`): a filled square
before each label item of the status row's `Filter ▾` menu, the file-name
band of a labeled strip cell, and, with no color at all, the label items of
the strip's right-click menu.

After this work the color labels use gpui-component's color palette
(`ColorName`) everywhere in the native app:

- the `Filter ▾` menu's color label items and the strip right-click menu's
  label items are each drawn as a gpui-component `Tag` holding the label's
  name (`Tag::color(ColorName::X)`, `No label` as an outline secondary Tag);
- the strip cell's label band is filled with the same `ColorName` at one
  fixed scale, so the band and the Tags agree.

The user accepts that the hues differ from the earlier fixed values. The
Tauri app (`crates/app`) is frozen and is not touched.

## Steps

- [x] Step 1: Draw the color labels with gpui-component's `Tag` and
      `ColorName` in the two menus and the strip band
  - Deferred by the user (2026-11-17): the visual check of both menus and a labeled strip cell in a dark and a light theme was not done before the PR; it is filed in `todo.md` as a pending manual check.
  - Done when:
    - In the `Filter ▾` menu, each of `Red`, `Orange`, `Yellow`, `Green`,
      `Blue`, `Pink`, `Purple` is a small `Tag::color(..)` of the same-named
      `ColorName` holding that text, and `No label` is a small outline
      secondary Tag holding `No label`; no square icon and no separate plain
      text remain on those rows. The check column, the click behavior and
      every other row (flags, stars, orientation, `Face focus`, `Eyes`,
      `Output`, EXIF groups, the sort menu) are unchanged.
    - In the strip's right-click menu, the eight label items (`red` …
      `purple`, `clearlabel`) are drawn the same way, the Tag in place of the
      plain label text, with the check mark column and the right-aligned
      `Kbd` shortcut where they were. Every other item of both right-click
      menus (folder and strip) is unchanged.
    - A labeled strip cell's name band is filled with the label's
      `ColorName` at scale 600 (`Tint::Other`: `ColorName::Gray`), the name
      still drawn in `LABELED_NAME_COLOR`; the band's geometry, the tooltip
      regions and the unlabeled cell are unchanged.
    - One `Tint` to `ColorName` mapping serves the menus and the band;
      `tint_color` and its hex values are gone, and nothing else refers to
      them.
    - `Tag::custom` is not used.
    - The pure-model tests of `menu.rs` and `context.rs` pass; `context.rs`
      has a test for which strip items carry a label tint (the seven colors
      their `Tint`, `clearlabel` the no-label case, every other item none).
    - The docs that became wrong are corrected (see the approach).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The native app was launched and both menus and a labeled strip cell
      were looked at in a dark and a light theme (row height unchanged at
      `MENU_ROW`, Tags not clipped, hover still readable); what was seen is
      noted in the PR description.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/main.rs` (`menu_row` near line 4900,
      `context_row` near line 4985, `tint_color` near line 5104, the band
      fill in `paint_strip` near line 5181, the constants' comment near line
      5075, the `gpui_kit::component` imports near line 87),
      `crates/native/src/context.rs`, `crates/native/src/menu.rs` (doc
      comments only), `crates/native/src/icons.rs`,
      `docs/agents/native-app.md`, `CLAUDE.md`.
    - API (gpui-component 0.9.1 through `gpui_kit::component`):
      `tag::Tag` is a `ParentElement` + `Sizable` (`Tag::color(ColorName)`,
      `Tag::secondary().outline()`, `.small()`); `ColorName` is re-exported
      at the component crate's root and has `scale(usize) -> Hsla`
      (50…950). A small Tag is 0.625 rem text with 0.5 vertical padding and
      a 1 px border, about 18.5 px tall, so it fits the 26 px `MENU_ROW`
      without changing the row. `Tag::color` picks its background, border
      and text from the theme's darkness itself (dark: 950 / 800 at 50 %
      alpha with 300 text; light: 50 / 200 with 600 text); do not override
      them.
    - Shared mapping: replace `tint_color` in `main.rs` with one function
      from `Tint` to `ColorName` (Red, Orange, Yellow, Green, Blue, Pink,
      Purple to the same name, `Other` to `Gray`). Keep it in `main.rs`:
      `strip.rs` is a pure model with no GPUI dependency and should stay so.
    - Filter menu: `menu.rs`'s model does not change (`Lead::Label(Option<Tint>)`
      and `Label::Text` stay, so its tests stay as they are); only the doc
      comments on `Lead` / `Lead::Label` that say "filled square" / "outline
      square" are reworded. In `menu_row`, a `Lead::Label` row draws one Tag
      holding the item's label text instead of the lead plus the text `div`.
    - Right-click menu: `context::Item` needs to tell `context_row` that an
      item is a color label and which. The known option is a new field on
      `Item` (for example `Option<Option<Tint>>`, or a small enum: not a
      label / a tint / no label), set in `strip_groups` for the third
      judgment group, where the action names `red` … `purple` are exactly
      `Tint::of`'s keys and `clearlabel` is the no-label case; `command()`
      sets it to "not a label". The step decides the exact shape; keep
      `Item`'s `PartialEq` / `Eq` derive (`Tint` already derives both).
      `context_row` then draws the Tag in place of `item.label.clone()`
      inside the same `justify_between` row, so the `Kbd` stays at the right
      end.
    - Band: `window.paint_quad(fill(band, rgb(tint_color(tint))))` becomes a
      fill with the mapped `ColorName`'s `scale(600)`. `scale` does not
      depend on the theme, so the band stays fixed in every theme, as the
      comment above the constants says of it; reword that comment, since it
      names `tint_color` and the `--label-*` tokens.
    - Orphans: `icons::SQUARE` and `icons::SQUARE_FILLED` are used only by
      the two `Lead::Label` arms. `crates/native` is a binary crate, so once
      unused outside tests they are expected to fail the build under
      `-D warnings` (verify). Remove what this change orphans and only that:
      the two path constants, `SQUARE_FILLED_SVG`, its `SVGS` entry (the
      array becomes 2 long), their entries in the tests' `PATHS` and in
      `the_catalog_paths_equal_their_icon_names`, and the mentions of the
      square in the module doc and the license header comment (including
      the Feather line, which is there only for `square`). `STAR` /
      `STAR_FILLED` stay.
    - Docs that state something this change makes wrong:
      - `docs/agents/native-app.md`: the menu leads rule ("a color label a
        filled Lucide `square` in `tint_color`, `No label` the outline") and
        the rule after it ("the labels are squares"); the theme section's
        "the label tints with `LABELED_NAME_COLOR`" among the fixed
        constants (the band is now a `ColorName` scale, the Tags follow the
        theme's darkness); the icons section and the gpui-kit rule wherever
        they name the filled square derivative or `square` from the default
        `Assets`. Keep the note that the differences from the Tauri menu are
        the user's decision.
      - `CLAUDE.md` (the `crates/native` description): "its leads Lucide
        icons from `src/icons.rs`" (the color label rows are now Tags), "the
        three own derivatives, the filled star and square and the frowning
        `scan-face`" (two after the removal), "which serves `star`, `square`
        and the icons gpui-component's controls draw", and the sentence
        ending "the judgment, label and over-the-photo colors staying fixed
        constants" if the label colors no longer fit it. Change only those
        clauses.
      - `docs/humans/usage.md` says the label tints the file-name band,
        which stays true; change it (and `usage.ja.md` with it) only if a
        statement there turns out wrong.
    - To verify during implementation: that the Tag's own hover (opacity
      0.9) and the row's hover (`accent` background, `accent_foreground`
      text) look right together, the Tag keeping its own text color; that
      the filter menu's popover width still fits its rows; that the name is
      readable on every band color at scale 600, the yellow band above all.

## Trade-offs and risks

- **The band's scale (decided: 600).** The name on the band is near-white
  (`LABELED_NAME_COLOR`, `0xfafafa`). Approximate contrast of that text on
  the palette (estimated from the hex values, not measured on screen):
  - 500: the most vivid, but yellow (`#eab308`) is about 1.9:1 and orange
    and green are under 3:1; about what the old yellow (`0xd8c032`) gave.
  - 600 (chosen by the user): yellow (`#ca8a04`) about 2.9:1, orange about
    3.6:1, green about 3.3:1, the rest 4.5:1 or more; better than today on
    every color while still reading as the named color.
  - 700: every color about 4.5:1 or more, but yellow becomes a brown
    (`#a16207`) and the bands are noticeably darker than the Tags' text.
  Switching `LABELED_NAME_COLOR` to a dark text on light bands is out of
  scope (the request keeps the name text and changes only the fill).
- **Tags follow the theme, the band does not.** `Tag::color` draws a dark,
  half-transparent background with light text in a dark theme and a pale
  background with dark text in a light theme; the band is a solid 600 fill
  in both. They share the hue family, not the exact color. Making them equal
  would need `Tag::custom`, which the user ruled out.
- **The Tag holds the label text (taken).** The label's name is the color
  label's own name, so the Tag replaces both the swatch and the plain text.
  With the text inside, a small Tag draws it at 10 px (0.625 rem), smaller
  than the neighbouring rows. If that reads too small, the default-size Tag
  (12 px text, about 26.5 px tall) does not fit the 26 px row without
  changing `MENU_ROW`, so the fallback is a swatch Tag plus plain text, not a
  bigger Tag.
- **Model change in `context.rs` versus matching in the view.** The plan
  adds a field to `context::Item` so the rule (which items are labels) is
  pure and tested. The alternative with no model change is for `context_row`
  to match `Action::Judge(name)` against the label names; it is smaller but
  puts the rule in untested view code and duplicates the name list.
- **Removing the square icons.** The removal is forced by the unused-item
  lint, not chosen; it touches `icons.rs`, its tests and three doc passages.
  If the lint turns out not to fire, leaving the constants in place is the
  smaller diff, at the price of dead assets.
- **Custom label names.** Both menus show the fixed English color names
  (`Red` …), as today; the user's `xmp:Label` names (`labelNames`) are not
  shown in the Tags. Unchanged by this work.

## Progress

- (2026-11-17) Step 1 complete (the visual check in a dark and a light theme deferred to `todo.md` by the user)
