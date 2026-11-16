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

# Align the strip's right-click menu with the Filter menu (native app)

## Purpose

In the GPUI app (`crates/native`), the strip's right-click menu
(`context.rs`'s `strip_groups`) and the status row's Filter menu (`menu.rs`'s
`filter_rows`) both list the flags, the rating and the color labels, but not
the same way:

| Group | Strip menu today | Filter menu |
| --- | --- | --- |
| Flags | Pick, Reject, Unflag | Picked, Untagged, Rejected |
| Rating | text `1 star` … `5 stars`, then `No stars` | star icons, 5 stars down to the unrated (five dim stars) |
| Color labels | Red … Purple, `No label` | Red … Purple, `No label` (same order) |

After this work the strip menu's judgment groups follow the Filter menu: the
flags run Pick, Unflag, Reject; the rating rows run from 5 stars down to the
unrated and draw the same star icons the Filter menu draws; the labels keep
their order, which already matches. Every item keeps its shortcut (`Kbd`), its
checked state and its action. The Tauri app (`crates/app`) is frozen and is
not touched.

## Decisions (by the user, 2026-11-16)

- The unrated item (`clear`) is drawn as five dim stars, as the Filter menu's
  `Stars(0)`, not as the text `No stars`.
- The Filter menu's flag dots and label squares are **not** added to the strip
  menu in this work.
- The Filter menu's section headings are **not** added either.

## Steps

- [x] Step 1: Reorder the strip menu's flag and rating items and draw the
      rating items as star icons
  - Done when:
    - The strip's right-click menu lists the flag group as Pick, Unflag,
      Reject, and the rating group as 5, 4, 3, 2, 1 stars and then the
      unrated item (`clear`), each rating row drawn with the Filter menu's
      star icons (`on` lit `icons::STAR_FILLED` in `STARS_COLOR`, then
      `5 - on` dim `icons::STAR`) in place of the `N stars` / `No stars`
      text; the label group is unchanged.
    - Each item still shows its action's first key at the row's right end,
      is checked from the focused file's state, and runs the same
      `Action::Judge` action; a view-only folder still has no judgment
      groups.
    - The tests in `context.rs` assert the new order and the rating items'
      star counts, and a test pins the strip menu's flag / rating / label
      order to `filter_rows`' (or, if that coupling is awkward, the expected
      lists are spelled out in the test next to a comment naming
      `filter_rows`).
    - The Filter menu's rows and look are unchanged (`menu.rs`'s tests pass
      unmodified).
    - `docs/agents/native-app.md`'s "The right-click menu is a deferred
      popover at the pointer" says the judgment groups follow the Filter
      menu's order and that the rating rows draw its stars; `CLAUDE.md` is
      updated only if its `context.rs` description became inaccurate.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/context.rs` (`JUDGMENT_GROUPS`, `Item`,
      `strip_groups`, the tests, the module doc), `crates/native/src/main.rs`
      (`context_row`, and the star drawing inside `menu_row`),
      `docs/agents/native-app.md`. Do not touch `crates/app`.
    - `context::Item.label` is a `String` today and `context_row` draws it as
      text. The item needs a way to say "N stars". The step decides between
      reusing `menu::Label` (`Text` / `Owned` / `Stars(u8)`; the reveal label
      and the `Move N Files to Trash…` labels would be `Owned`) and a
      `context.rs`-own representation (for example a small label enum of
      text or stars). Pick whichever leaves the smaller diff in `main.rs`
      and the tests; `folder_groups` items stay plain text either way.
    - Draw the stars once: move the `menu::Label::Stars` arm of `menu_row`
      (five `svg()` icons at `MENU_ICON`, 2 px gap, lit in `STARS_COLOR`, dim
      in `muted_foreground` at alpha 0.4) into a small helper both `menu_row`
      and `context_row` call, so the two menus cannot drift. No new icon is
      needed; `icons::STAR` and `icons::STAR_FILLED` are already served.
    - `context_row` keeps its layout: the label (now text or stars) at the
      left, the `Kbd::new(keys::keystroke_of(..)).appearance(false)` shortcut
      at the right, `justify_between`. A disabled row is only ever a trash /
      delete row, so the stars need no disabled look.
    - The unrated item keeps the `clear` action and its `s.rating.is_none()`
      check; it is drawn as zero lit stars, as the Filter menu's `Stars(0)`.
    - Existing tests that index into the groups and will shift:
      `strip_groups_show_the_first_key_when_an_action_has_several` reads
      `[3][1]` for `reject` (becomes `[3][2]`);
      `strip_groups_show_the_default_keys` expects `p, x, u` and
      `1..5, 0` (become `p, u, x` and `5, 4, 3, 2, 1, 0`);
      `strip_groups_list_select_all_reveal_copy_the_judgments_then_trash_and_delete`
      compares labels as strings. `[4][2]` for `rate3` happens to stay put.
      The checked-state tests keep their expected order (one checked item
      per group) and the 23-item count is unchanged.
    - The `context.rs` module doc and `strip_groups`' doc call it a port of
      `context.ts` / `contextMenuGroups`; add that the judgment groups'
      order and the rating rows' stars follow the Filter menu on purpose, as
      `menu.rs`'s module doc records its own differences from the HTML.
    - Verify during implementation: `grep` `docs/humans/usage.md` and
      `README.md` for a description of the strip menu's judgment item order
      or the `No stars` text. The planning search found none, so no user
      doc (and no `.ja.md` pair) should need a change; if one turns up,
      update both languages in the same PR.
    - Hand check on the real app (`mise run native:dev`), passing
      `--settings` and `--log` with paths in a test folder as
      `docs/agents/native-app.md` requires, so the user's real store is not
      touched: right-click a strip cell with a rated, picked, labeled file
      and confirm the order, the stars, the check marks and the shortcuts.

## Trade-offs and risks

- **Flag order changes muscle memory.** `Reject` moves from second to third
  in the flag group. This is the request, noted because it is the one
  behavior change a user of the menu will feel.
- **Divergence from the Tauri app.** `context.rs` stops being a straight
  port of `context.ts` for the judgment groups. The Tauri app is frozen and
  `menu.rs` already diverges from the HTML on purpose, so this is accepted;
  the module doc records it.
- **One step.** This change is one small PR; splitting the model from its
  drawing would leave an unreviewable half (stars in the model that nothing
  draws).

## Progress
