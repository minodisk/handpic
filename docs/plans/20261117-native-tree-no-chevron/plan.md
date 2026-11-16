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

# Native folder tree without the chevron: a row click opens and toggles

## Purpose

PR #1113 (`../_archived/20261116-native-tree-listitem/plan.md`) drew each row
of the GPUI app's folder tree as a gpui-component `ListItem` with a Lucide
`chevron-right` / `chevron-down` expander and a `folder` / `folder-open` icon.
gpui-kit's Tree component has no chevron: a click on a folder row toggles it.
This work removes the chevron and its 14 px slot from the native tree's rows
and makes a click on a row both open the folder and toggle its expansion, so
the tree reads and behaves like the Tree component while staying the
hand-built `uniform_list`. The Tauri app is frozen and is not touched.

## Steps

- [x] Step 1: Remove the chevron from the tree's rows and make a row click
      open and toggle
  - Done when:
    - `tree_row` draws no chevron and no empty slot in its place: a row is
      the indent (`12 + 16 * depth` px, unchanged), the `folder` /
      `folder-open` icon (open while expanded, unchanged), the name and the
      count. No element with the id `tree-expander` is left.
    - A left click on a row follows these rules, decided by one pure
      function in `crates/native/src/tree.rs` with a unit test per rule:
      - a row that is not the open folder and is collapsed, or cannot expand:
        the folder opens. The open's reveal expands it, as every open
        already does, so it ends up open and expanded.
      - a row that is not the open folder and is expanded and expandable: the
        folder opens and the row collapses, and stays collapsed once the
        reveal has finished.
      - the open folder's row, expandable: the click toggles it (expand and
        list, or collapse) and does not reopen the folder.
      - the open folder's row, not expandable: the click does nothing.
    - A click on the open folder's name (not a root, no Ctrl / Cmd / Shift)
      behaves exactly as before: it arms the slow click, whose rename starts
      `SLOW_CLICK_DELAY` later, and neither toggles nor reopens. The context
      menu's `Rename…` is unchanged.
    - The keyboard is unchanged: `tree::tree_key` (`Space` toggles, `Enter`
      opens), `tree::step`, the type-ahead and `treekeys.rs` are not
      modified and their tests pass as they are. The right-click menu and
      its `Expand All` / `Collapse All` are unchanged.
    - An open refused because a rename of the folder is in flight
      (`refuse_renaming`) collapses nothing.
    - `CHEVRON_RIGHT` and `CHEVRON_DOWN` are gone from
      `crates/native/src/icons.rs`, with their entries in the test `PATHS`
      (12 -> 10) and in `the_catalog_paths_equal_their_icon_names`, and the
      module comments naming the tree's chevrons are corrected.
    - Code comments that the change makes false are corrected: `tree_row`'s
      and `tree_toggle`'s doc comments and the "as an expander click does"
      comment in `main.rs`, `Row::expandable`'s in `tree.rs`, `tree_name`'s
      and `tree_name_clicked`'s in `folderops.rs` where they say the click
      "does not reopen".
    - `docs/agents/native-app.md` describes the new rows and click rules
      wherever it describes the old ones: the two lists of the icons the
      default `Assets` serve for the tree (no chevrons; the `Select`'s
      `icons/chevron-down.svg` stays), the "folder tree is a `uniform_list`"
      section (the row's parts, the click rules above, and that a reveal can
      leave the chain's end collapsed), the comparison with gpui-component's
      `Tree` ("Ours opens the folder on a click on the name and toggles only
      on the expander"), and the slow click rule.
    - `todo.md`'s item `### Native: real-device check of the folder tree rows
      drawn as ListItems (native-tree-listitem)` is updated in place: its
      background and checks no longer mention chevrons or the empty slot,
      and its `Clicks` check covers every click rule above (a collapsed row
      opens and expands, an expanded row that is not open opens and stays
      collapsed, the open row toggles from its icon / indent / count without
      re-listing, a click on the open row's name only arms the rename, an
      open leaf row does nothing) and that the indent still reads the depth
      clearly without the slot. The manual check stays pending (unticked),
      and it names this plan next to the archived one.
    - `CLAUDE.md` is unchanged unless the implementation makes one of its
      statements false (none is known: it mentions neither the chevron nor
      the expander).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/main.rs` (`tree_row`, `reveal`, the row's
      click listener, comments), `crates/native/src/tree.rs` (the decision
      function and its tests), `crates/native/src/icons.rs`,
      `crates/native/src/folderops.rs` (comments only),
      `docs/agents/native-app.md`, `todo.md`. Nothing under `crates/app`.
    - `open` -> `reveal` already expands and lists the whole chain from the
      root down to the opened folder itself (`for at in 0..chain.len()`), so
      the "open and expand" case needs no call to `tree_toggle`; calling it
      too would list the folder twice.
    - For the same reason an "open and collapse" needs `reveal` to leave the
      chain's end alone (skip the last iteration's `expand` and listing; the
      end's spelling comes from its parent's listing through `respell`, and
      its count from the open's `set_listed_count`), with the row collapsed
      synchronously at the click. The step decides how the intent reaches
      `reveal`:
      - a parameter threaded through `open` and `reveal` (`open` has about
        ten call sites, all of which would pass "expand"), or
      - a field on `Riffle` naming the folder the click collapsed, which
        `reveal` takes on every call and honors only for that path (compare
        with `tree::relation`, since `reveal` gets the absolute path and the
        row carries the tree's spelling). This also survives an open
        deferred by the gate (`deferred_open`), whose reveal runs later.
      Whichever is chosen, verify the deferred open (`self.gate.in_flight()`)
      and the `reveal_wanted` path do not re-expand the row or leave stale
      state behind.
    - The decision function takes what it needs from the `Row` (`expanded`,
      `expandable`) and whether the row is the open one
      (`tree_open_row == Some(at)`), and returns a small enum; keep it next
      to `tree_key` / `TreeCommand`. Reuse `TreeCommand` only if it fits
      without bending the keyboard's meaning. No further abstraction.
    - The open row's toggle goes through the existing `tree_toggle`, which
      already collapses, or expands and re-lists with the cached children
      drawn meanwhile.
    - `tree_name`'s `on_click` on the open row stops propagation, so the
      row's listener never sees a plain click on the open row's name. Leave
      that as it is. A click there with Ctrl / Cmd / Shift returns without
      stopping and so reaches the row's listener, which now toggles instead
      of reopening.
    - The name is `flex_1`, so on the open row the mouse targets that toggle
      are the indent, the folder icon, the gaps and the count badge. Do not
      add a click listener to the folder icon; the row's is enough.
    - Keep `TREE_ICON` (the folder icon uses it), the `h_flex` with
      `gap(px(4.))`, and `ListItem`'s `selected` / `secondary_selected`.
    - `ListItem`'s click does not stop propagation and the pane's left
      mouse-down still gives the tree the keyboard and disarms the slow
      click; do not change either.
    - `icons::FOLDER` / `FOLDER_OPEN` stay. `icons/chevron-down.svg` is
      still served by `gpui_kit::assets::Assets` for the `Select`; only the
      app's own constants go.
    - There is no GPUI window test in `crates/native`, so the click wiring
      and the drawing are covered by the pending manual check in `todo.md`,
      not by a test. Record in `learnings.md` what was and was not verified.

## Trade-offs and risks

- The open row's name does not toggle (taken, chosen by the user) versus
  removing the slow click. A plain click on the open folder's name keeps
  arming the slow-click rename and does not toggle, so one click never both
  collapses the folder and starts a rename. Consequence: the name fills most
  of the row, so the open row's mouse toggle is its icon, indent or count
  (or `Space`). Not taken: drop the slow click so a click anywhere on the
  open row toggles, as in the Tree demo, with rename only through the
  context menu's `Rename…`.
- A click on the open row toggles without reopening (taken) versus reopening
  as well. Reopening on every toggle would re-list the folder and restart
  its scan each time the user folds the open folder. Consequence: a click on
  the open row no longer re-lists the folder (the folder watcher still does
  on a change), and an open leaf row ignores clicks, so a failed open cannot
  be retried by clicking its row again.
- An expanded row that is not open is opened and collapsed (taken, the
  user's "opens and toggles") versus opened and left expanded. The taken
  rule needs `reveal` to skip the chain's end.
- Double-clicks are not special-cased. A double-click on a not-open row's
  icon opens it on the first click and toggles it on the second while the
  reveal may still be expanding it, so the end state can be either. If the
  manual check finds this confusing, ignore clicks with `click_count() > 1`
  on the row in a follow-up.
- `../20261116-native-folder-count-cache/plan.md` Step 3 is not merged and
  also changes the native tree's code in `main.rs`; whichever lands second
  may need a rebase.

## Progress
