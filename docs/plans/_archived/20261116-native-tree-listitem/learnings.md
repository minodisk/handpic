# Learnings: native-tree-listitem

## Step 1

- `folder` and `folder-open` are in `gpui-kit-assets` 0.9.1's default
  `Assets` (`default-icons.txt` lists `icons/folder.svg`,
  `icons/folder-open.svg` and `icons/folder-closed.svg`), not only in its
  catalog as the plan assumed. They need only path constants in `icons.rs`,
  no `icon_assets!` entry, like the chevrons.
- Trade-off 1 was decided for Option B from the sources, not from a running
  app: with `ListSettings::active_highlight` true by default, Neutral's
  `Default Dark` / `Default Light` give the selected row `list.active`
  `#1e40af33` / `#bfdbfe33` and the cursor outline `selection` `#1d4ed8` /
  `#55a0fc` (clamped to 30 % alpha), a blue off the grayscale Neutral
  palette. `theme.rs`'s `map` now sets `list_active` and `list_hover` to
  `accent` and `selection` to `ring`, today's open row, hover and cursor
  colors. `selection` also paints `Input`'s selected text (under the text:
  `gpui-base`'s input element paints the selection paths before the lines),
  so the settings modal's and the inline rename's text selection turns from
  blue to `ring` gray in Neutral, and a `Select`'s list row, also a
  `ListItem`, lights in `accent`.
- `ListItem`'s base is an `h_flex()` without a width, so the row needs its
  own `.w_full()` (the `uniform_list` layout-root rule in
  `docs/agents/native-app.md`); the plan's style list left it out.
- `ListItem` has an inherent `selected` and `Selectable::selected`; with the
  trait imported for `secondary_selected`, the inherent one is picked.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, every desktop platform, best on Windows):
  run `mise run native:dev` and verify the tree rows drawn by `tree_row` in
  `crates/native/src/main.rs`. Steps and expected results: in Neutral dark,
  Neutral light and one vendored theme, the open row lit (Neutral: `accent`,
  semibold), the hover lit on other rows only, and the keyboard cursor
  outlined (Neutral: `ring`) only after a click in the pane; the outline on
  the open row; a click on the cursor row still opens it; a click on a
  chevron toggles without opening; a right-click opens the context menu; a
  slow click on the open row's name starts the inline rename, whose input
  fits the 22 px row and Escape cancels; a failed folder's name red, also on
  the open row; a pending rename italic and muted; the RAW count badge at
  the row's right end; a deep path at the default 220 px pane width
  truncates the name and keeps the count in the row; a long tree scrolls
  with uniform rows; the settings modal's text selection reads as gray. The
  step's checkbox was ticked on the automated criteria (`mise run ci:local`).
