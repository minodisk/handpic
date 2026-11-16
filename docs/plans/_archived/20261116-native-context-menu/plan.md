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

# Native right-click menus on gpui-kit's PopupMenu

## Purpose

`crates/native` draws its two right-click menus (the folder tree's and the
thumbnail strip's) with hand-built `div`s that imitate gpui-component's
`PopupMenu` (`context_popover`, `context_row` over `menu_surface` /
`menu_item` in `main.rs`). This plan replaces them with the component itself
(gpui-kit 0.9.1's `gpui_kit::component::menu::PopupMenu`, the menu its
`ContextMenu` shows), so the right-click menus get the component's look,
hover, keyboard navigation and scrolling, and the app keeps only the items
(`context.rs`) and what each one does. The `Sort ▾` / `Filter ▾` menus are
not touched.

## Steps

- [x] Step 1: Show the tree's and the strip's right-click menus as a gpui-kit `PopupMenu`
  - Done when:
    - A right-click on a tree folder and on a strip cell opens a
      gpui-component `PopupMenu` with the same items, groups (a separator
      between groups), shortcut texts, checked and disabled states as
      `context::folder_groups` / `context::strip_groups` give today, and each
      item runs the same action on the same target as before (the tree: the
      right-clicked folder; the strip: the selection rule of
      `strip_context`, the targets taken when the menu opens).
    - The strip's gaps and empty area still open nothing; a right-click in
      the tree still neither opens the folder nor moves the keyboard; a press
      on the menu still hands the keyboard back from the tree; a press
      outside, Escape, and picking an item close the menu; a right-click
      elsewhere while one is open replaces it; the menu still flips to the
      pointer's other side near the window's bottom and right edges.
    - `context_popover`, `context_row`, the `context_scroll` field and
      `context::Menu` / `context::Open` (with their three tests) are gone,
      and nothing else this change orphaned is left (imports included). The
      helpers the Sort / Filter menus use (`menu_surface`, `menu_separator`,
      `menu_item`, `menu_row`, `MENU_ROW`, `MENU_PAD`, `MENU_ICON`,
      `menu_scroll`) are unchanged.
    - The remaining `context.rs` tests (`folder_groups`, `strip_groups`,
      `shown_path`) pass unchanged.
    - `CLAUDE.md`'s description of `src/context.rs` and
      `docs/agents/native-app.md` describe the new drawing (see the
      approach).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The PR description lists the manual check on Windows as done or
      pending, and `learnings.md` records it: right-click in the tree (a
      root and a non-root folder, with the tree holding the keyboard and
      not) and on the strip (a cell inside and outside a selection, a gap,
      a view-only folder, a cell at the window's right edge so the menu
      flips, a short window so the menu scrolls), an item of each kind run
      (`Rename…`, `Copy Path`, a judgment, `Move to Trash…`), Escape, a
      keymap key while open, Up / Down / Enter, a press outside, a second
      right-click.
  - Implementation approach (as far as it is known; omit if unknown):
    - Facts read at gpui-component 0.9.1 (`menu/context_menu.rs`,
      `menu/popup_menu.rs` in the cargo registry; `crates/native/Cargo.lock`
      pins `gpui-kit` and `gpui-component` 0.9.1, `gpui-pre` 0.3.12). Not
      compiled or run during planning; verify each while implementing:
      - `ContextMenuExt::context_menu(builder)` wraps an element, opens on
        every right press in its hitbox (it stops propagation, then builds
        the menu deferred and focuses it, also when the built menu is empty,
        which is then never drawn), keeps the open state in element state
        the view cannot reach, and places the menu with
        `anchored().position(p).snap_to_window_with_margin(px(8.))` at
        `Anchor::TopLeft`. The anchor has no setter, and that fit mode only
        shifts the menu; it never flips it.
      - `PopupMenu::build(window, cx, |menu, window, cx| ..)` gives an
        `Entity<PopupMenu>`. Public: `.item(PopupMenuItem)`, `.separator()`,
        `.min_w`, `.max_h`, `.scrollable(true)`, `.action_context(handle)`
        (the focus given back on dismiss); `PopupMenuItem::new(label)` and
        `PopupMenuItem::element(|window, cx| ..)` with `.checked(bool)`,
        `.disabled(bool)` and `.on_click(|&ClickEvent, &mut Window, &mut App|)`.
        The menu emits `DismissEvent`, closes itself on a press outside
        (`on_mouse_down_out`), binds `enter`, `escape`, `up`, `down`, `left`,
        `right` in the `PopupMenu` key context, and `activate` runs the
        item's handler and then always dismisses.
      - A standard `Item` draws a shortcut only from a GPUI `Action` bound
        in GPUI's keymap. The app's keymap is its own (`Bindings`, resolved
        in `key_down`), so an item with a shortcut has to be an
        `ElementItem` that draws the label and
        `Kbd::new(keys::keystroke_of(..)).appearance(false)` itself, as
        `context_row` does today (muted, `text_xs`, at the right end).
      - GPUI's `on_mouse_down_out` tests `!hitbox.contains(mouse_position)`,
        not occlusion, so a press on a menu drawn over the tree pane is not
        "outside" the pane: the hand-back of the keyboard on a press on the
        menu still needs its own capture-phase handler.
    - Planned shape (decided over the `.context_menu()` wrapper; see
      "Trade-offs and risks"): the view builds the `PopupMenu` on the right
      press and places it itself.
      - `tree_context` / `strip_context` keep their rules (root detection,
        selection, focus move, `menus.close()`, the `strip_groups`
        arguments) and take the `Window`. In place of `context.open_*` they
        build the menu from the groups: a `separator()` between groups, one
        item per `context::Item` with `checked == Some(true)` as
        `.checked(true)`, `disabled` as `.disabled(true)`, and an `on_click`
        that updates the view through a `WeakEntity<Riffle>` with the item's
        `Action` and a clone of the `context::Target` (so `context_action`
        takes the target as an argument, not from `self.context.close()`).
      - The view holds the open menu in one field in place of
        `context: context::Menu` and `context_scroll`: the entity, the
        pointer position and the `DismissEvent` subscription, kept together
        so that replacing the menu drops the old subscription (a second
        right-click dismisses the old menu in the capture phase and opens
        the new one in the bubble phase; the old menu's `DismissEvent` must
        not clear the new one). The subscription clears the field and
        notifies.
      - The menu is focused when built (as `ContextMenu` does, before its
        first frame) and given `.action_context(self.focus.clone())` so a
        dismiss returns the focus to the view's handle. `PopupMenu::dismiss`
        leaves the focus alone when a handler already moved it (the inline
        `Rename…` focusing its `Input`, a dialog); check `Rename…` from the
        menu still ends with the input focused.
      - `render` draws it where `context_popover` is drawn today:
        `deferred(anchored().position(at).child(..))` with `anchored`'s
        default `SwitchAnchor` fit (the flip), the menu entity inside a
        wrapper `div` carrying the existing `capture_any_mouse_down` that
        calls `set_tree_keys(false, cx)`. `ContextMenu` gives its deferred
        layer `POPUP_PRIORITY` (100, `gpui_base`); use the same if it is
        reachable through `gpui_kit`.
      - Size: `.min_w(px(180.))`, `.scrollable(true)` and `.max_h(..)` of
        today's cap (the viewport height less the window paddings, 40 px and
        `TITLE_BAR_HEIGHT`, `menu_max` in `render`), computed when the menu
        is built.
      - Closing from the view: `key_down`'s "any key closes the right-click
        menu" arm and the five `self.context.close()` calls in
        `folderops.rs` (before a folder dialog opens) drop the field; the
        key arm also returns the focus to `self.focus`. The key listener is
        on the view's root `div`, which the menu is a descendant of, so a
        key the menu does not bind should still reach `key_down`; verify,
        and verify Left / Right (which the top-level menu propagates) still
        close the menu and page the strip.
    - `context.rs`: keep `Action`, `Item`, `folder_groups`, `strip_groups`,
      `shown_path` and `Target`; remove `Menu` and `Open` and their tests
      (`a_menu_opens_at_the_pointer_on_its_folder_and_closes_once`,
      `a_strip_menu_opens_on_its_files_and_closes_with_them`,
      `a_second_right_click_replaces_the_open_menu`); update the module
      comment and `Item::shortcut`'s.
    - Documentation in the same PR:
      - `CLAUDE.md`: the `src/context.rs` passage ("and `Menu`, the open
        menu's position and target), which the window draws as a `deferred`
        / `anchored` popover at the pointer").
      - `docs/agents/native-app.md`: rewrite "The right-click menu is a
        deferred popover at the pointer" (title included; other sections
        refer to it by name); the component list near the top (the
        right-click menu's `Kbd` text entry gains `PopupMenu`); the key
        table's `PopupMenu` row (its six keys now act while a right-click
        menu is open); in "The sort and filter menus are deferred popovers
        over the window", `menu_surface` "shared with the right-click menu"
        and the "`PopupMenu`, `Popover` and `ContextMenu` are not used"
        rule, which now holds for Sort / Filter only (their reason, an item
        click leaving the menu open, stays); the title bar rule naming both
        menus' height cap; the gpui-component `Tree` section's note that
        `PopupMenu` is not used; "The tree takes the keyboard through one
        flag in the view" (the hand-back on a press on the menu). Follow the
        guide's Measured / Inferred labels: what was only read is Inferred
        until the manual check.
      - `docs/humans/usage.md` and `usage.ja.md` only if the keyboard
        behaviour they state changes (they say Escape or a click elsewhere
        closes the menu, which still holds).
    - Scope: the Tauri app (`crates/app`) is frozen and not touched.

## Trade-offs and risks

- **Manual `PopupMenu` or the `.context_menu()` extension.** Decided by the
  user (2026-11-16): manual `PopupMenu`. The plan uses the component's menu
  (`PopupMenu`) but not the `context_menu` wrapper, which leaves about ten
  lines of placement (`deferred` / `anchored` at the pointer, what
  `ContextMenu` does internally) in `main.rs`. Reasons: the wrapper cannot
  flip (it only shifts, so every strip menu, opened at the window's bottom,
  would be pushed up over the pointer, which then rests near
  `Move to Trash…` / `Delete Permanently…`; the current code avoids
  `snap_to_window` for this reason); its open state cannot be closed from the
  view; and it opens on any right press in its hitbox, so the strip's empty
  area would need workarounds, an empty menu otherwise taking the focus
  without being drawn.
- **Keys while the menu is open.** `PopupMenu` takes the GPUI focus and
  binds Up / Down (move the highlight) and Enter (activate it); today those
  keys close the menu and then act (the tree's cursor, the keymap). Escape
  is unchanged, and every other key stays "close, then act". Accepted as
  part of adopting the component.
- **Look.** The component's own: 14 px text (13 px today); the check column
  only when some item is checked (the folder menu and a view-only strip
  menu have none, today the column is always reserved); a scrollable menu
  that overflows opens scrolled to its first checked item, so in a short
  window the strip's menu opens at the flag group, not at `Select All`; the
  height cap is taken when the menu opens, not on every frame. Accepted.
- **Focus.** This is the first component in the app's view that takes the
  focus away from the view's one handle. The risks to check by hand: the
  focus returns to the view after every close path (item, Escape, press
  outside, a key, a replaced menu, a dialog opening over it), and the
  tree's `tree_keys` flag is unaffected by the focus moving.
- **Verification.** Nothing here is covered by an automated test (the crate
  has no GPUI UI test for the menus); the behaviour rests on the manual
  Windows check listed in the step.

## Progress

- (2026-11-17) Step 1 complete (the manual Windows check recorded in learnings.md is still pending)
