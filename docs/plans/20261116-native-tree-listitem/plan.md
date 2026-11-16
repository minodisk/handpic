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

# Folder tree rows as gpui-component `ListItem`s (native app)

## Purpose

The GPUI app's left-pane folder tree draws each row as a hand-built `div`:
a text expander, an `accent` hover and open row, and a `ring` border for the
keyboard cursor. This work gives the rows the look of gpui-kit's Tree
component (<https://gpui-kit.com/component/tree/>): one gpui-component
`ListItem` per row, indented by depth, with Lucide chevron and folder
icons, and selected / hover / cursor colors that come from the theme's list
tokens.

It changes visuals only. The tree's state model (`crates/native/src/tree.rs`:
lazy per-folder listing, treewatch, keyboard cursor, type-ahead), the
`uniform_list` in `tree_pane`, the inline rename and every interaction stay
as they are. gpui-component's `tree()` / `TreeState` are not adopted (see
"The folder tree is not gpui-component's `Tree`" in
`../../agents/native-app.md`). The Tauri app is frozen and is not touched.

## Steps

- [x] Step 1: Draw the folder tree's rows as `ListItem`s with chevron and
      folder icons
  - Done when:
    - `tree_row` in `crates/native/src/main.rs` builds a gpui-component
      `ListItem` (0.9.1, `gpui_kit::component::list::ListItem`) per row;
      the open folder's row is `.selected(true)`, the keyboard cursor's row
      while the tree has the keys (`self.tree_keys &&
      self.tree_cursor_row == Some(at)`) is `.secondary_selected(true)`,
      and the row's own `.hover()`, `border_1` / `border_color` and `accent`
      background are gone
    - the expander is a Lucide `chevron-right` (collapsed) / `chevron-down`
      (expanded) icon, a row that is not expandable keeps an empty slot of
      the same width, and each row shows a `folder` / `folder-open` icon
      before its name
    - every row is still exactly `TREE_ROW_HEIGHT` high and its text
      `TREE_TEXT` in size
    - unchanged in behavior: a click on the expander toggles and does not
      open the folder (`stop_propagation`), a click on the row opens it
      (`open_from_tree`), a right press opens the context menu
      (`tree_context`), the open row's name arms the slow-click rename, the
      inline rename input (`folderops.rs`'s `tree_name`) works in the row,
      a failed folder's name is red, a pending rename is italic and muted,
      the RAW count badge shows, and a left press anywhere in the pane
      still gives the tree the keyboard
    - `crates/native/src/icons.rs` serves the two folder icons, and its
      `every_icon_path_loads_an_svg` test covers every path the row draws
    - `docs/agents/native-app.md` no longer says the tree cursor is `ring`
      and the tree's hover and open row are `accent`; it names the list
      tokens the `ListItem` uses, and its icon rule lists the new icons
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
    - the PR description lists the manual visual check as pending (see the
      last approach item) unless the implementer could run the app
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/main.rs` (`tree_row`, the
      `gpui_kit::component` import list), `crates/native/src/icons.rs`,
      `docs/agents/native-app.md`. `crates/native/src/folderops.rs`'s
      `tree_name` should need no change. `CLAUDE.md` describes the tree's
      state and the icon mechanism, not the row visuals, so it should need
      no change either; re-read its `src/tree.rs` and `src/icons.rs`
      passages once the code is final and edit only what became false.
    - `ListItem::new(("tree-row", at))` carries the element id. Its
      inherent `.on_click(..)` and `.on_mouse_down(MouseButton::Right, ..)`
      take the closures `cx.listener(..)` returns, so the two row handlers
      move over as they are. `ListItem` gates them on not being disabled or
      a separator, which a tree row never is. It does not stop propagation,
      so `tree_pane`'s left-press handler still runs.
    - `.secondary_selected` is a method of the `Selectable` trait
      (`gpui_kit::component::Selectable`), which must be imported;
      `.selected` is inherent.
    - `ListItem` renders its children inside `div().w_full().children(..)`,
      a block and not a flex row. Put the row's content in one own
      `h_flex().w_full().items_center()` child holding the expander, the
      folder icon, `self.tree_name(at, cx)` and the count. Verify that the
      name still truncates (`flex_1().min_w_0().truncate()`) inside this
      extra wrapper and that a long name does not push the count out of
      the row; add `min_w_0` / `overflow_hidden` on the wrapper if it does.
    - `ListItem` applies `gap_x_1().py_1().px_3().text_base()` and then
      refines with its own style, so set on the `ListItem`:
      `.h(px(TREE_ROW_HEIGHT))`, `.py_0()`, the depth indent as `.pl(..)`,
      `.pr_2()`, `.text_size(px(TREE_TEXT))`, `.rounded_sm()` (the cursor
      outline repeats the item's corner radii), `.cursor_pointer()` and
      `.whitespace_nowrap()`.
    - Indent: the demo's `px(16.) * depth + px(12.)` (today
      `4 + 12 * depth`). See the trade-offs.
    - Icons: `chevron-right` and `chevron-down` are already in gpui-kit's
      default `Assets` (the `Select` draws `icons/chevron-down.svg`), so
      they need only path constants (or `IconName` if it has both). `folder`
      and `folder-open` are not in the default set (`default-icons.txt` of
      `gpui-kit-assets` 0.9.1) but are in its catalog: add them to
      `icon_assets!(Catalog, [..])` (the identifiers are expected to be
      `Folder` and `FolderOpen`; confirm against the crate), add
      `FOLDER` / `FOLDER_OPEN` constants (`icons/folder.svg`,
      `icons/folder-open.svg`) and extend the test's `PATHS`. Draw them as
      the menus do, `svg().path(..).size(..).flex_none()` with
      `text_color`; the chevron `muted_foreground` as the text expander
      was.
    - The expander keeps its own element id `("tree-expander", at)`, its
      fixed width and, when the row is expandable, `cursor_pointer` and the
      `on_click` that stops propagation and calls `tree_toggle`.
    - `folder-open` is shown on an expanded row, `folder` otherwise (the
      demo's rule). See the trade-offs.
    - Colors now come from `ListItem`: selected is `list_active` when
      `theme.list.active_highlight`, else `accent` with `accent_foreground`
      text; hover is `tokens.list_hover` (falling back to `accent` at
      60 %), not applied on a selected or cursor row; the cursor is a 1 px
      absolute outline in `selection` (falling back to `primary`).
      `theme.rs`'s `map` sets none of these for Neutral. Measure what
      Neutral dark and light show for the three states; see the trade-offs
      for what to do if they are off-palette.
    - The failed name's `danger` and the pending name's `muted_foreground`
      are set on the name element, below `ListItem`'s own text color, so
      they should still win; verify on a selected row.
    - Keep `font_weight(FontWeight::SEMIBOLD)` on the open row (see the
      trade-offs).
    - The inline rename's `line_height(px(TREE_ROW_HEIGHT - 8.))` assumes
      the row's height; with the border gone and `py_0`, check the input
      still fits the 22 px row without changing its height.
    - Manual check, by running the app (`mise run native:dev`): the three
      states in Neutral dark, Neutral light and one vendored theme; the
      cursor outline on the open row; a click on a cursor row still opens
      it (the outline is an absolute child over the row); expander click,
      row click, right-click, slow-click rename, inline rename with Escape,
      a failed folder, a deep path at the default 220 px pane width; and
      scrolling a long tree (uniform row height). If the implementer cannot
      run the app, list these in the PR as pending.

## Trade-offs and risks

1. **Neutral theme's list tokens.** Today the open row and the hover are
   `accent` and the cursor is `ring`. `ListItem` reads `list_active` (alpha
   clamped to 0.2 or less) or `accent`, `list_hover` and `selection`
   instead, and `theme.rs`'s `map` does not set them for Neutral; its
   comment says `Theme::update` does not re-derive derived colors, so
   Neutral may show the base theme's values rather than the Neutral
   palette's.
   - Option A: accept what the tokens resolve to. No `theme.rs` change;
     the look may be off the Neutral palette.
   - Option B: map `list_active`, `list_hover` and `selection` in `map`
     (for example to `accent` and `ring`). This keeps Neutral close to
     today's look, but touches `theme.rs` and its tests, and changes the
     same tokens for any other `ListItem` / `Table` user and for `Input`'s
     text selection (`selection`).
   - The step defaults to measuring first and taking B only if A is
     visibly wrong. The user approved this default.
2. **Indent.** The demo's `12 + 16 * depth` px against today's
   `4 + 12 * depth`. With the added folder icon, the demo's indent leaves
   noticeably less room for names in the default 220 px pane on deep
   paths. The user approved the demo's.
3. **Semibold open row.** The demo's selected row has no weight change.
   Keeping `SEMIBOLD` preserves today's emphasis, which matters more now
   that `list_active` is at most 20 % opaque. The step keeps it.
4. **What `folder-open` means.** The demo shows it on an expanded row; the
   step follows the demo.
5. **Count badge.** It keeps its `accent` background. On a `list_active`
   row it will stand out. Changing the badge is outside this work.
6. **Risk: row layout inside `ListItem`.** The children wrapper is a block
   `div().w_full()` inside a `justify_between` flex row. Name truncation
   and the count's position depend on how the extra wrapper sizes; this is
   read from source, not measured.
7. **Risk: unverified at planning time.** Nothing was built. The
   `icon_assets!` identifiers `Folder` / `FolderOpen`, and that the
   absolute cursor outline does not take pointer events from the row, are
   assumptions to confirm during implementation.

## Progress
