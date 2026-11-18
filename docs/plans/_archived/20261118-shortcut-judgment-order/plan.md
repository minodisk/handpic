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

# Keyboard Shortcuts page: the Judgment group in the menus' order

## Purpose

In the native app's settings modal, the Keyboard Shortcuts page lists the rows
of its `Judgment` group in the backend keymap's `DEFAULTS` order (`rate1` …
`rate5`, `reject`, `rejectRest`, `pick`, `unflag`, `clear`, the colors,
`clearlabel`, `clearall`). The filmstrip's right-click menu
(`crates/native/src/context.rs`, `JUDGMENT_GROUPS` behind `strip_groups`) and
the `Filter ▾` menu (`crates/native/src/menu.rs`, `filter_rows`) list the same
judgments in another order: `pick`, `unflag`, `reject`, then `rate5` … `rate1`,
`clear` (0 stars), then `red`, `orange`, `yellow`, `green`, `blue`, `pink`,
`purple`, `clearlabel`. The two menus already agree with each other (the test
`strip_judgment_groups_follow_the_filter_menus_order_and_stars` in
`context.rs` asserts it). Once this is done, the user reads the judgments in
one order in all three places.

## Steps

- [x] Step 1: Order the Judgment group's rows after the strip menu's judgment items
  - Done when:
    - The `Judgment` group of the Keyboard Shortcuts page lists `pick`,
      `unflag`, `reject`, `rate5`, `rate4`, `rate3`, `rate2`, `rate1`, `clear`
      (0 stars, right after 1 star), `red`, `orange`, `yellow`, `green`,
      `blue`, `pink`, `purple`, `clearlabel`, then the Judgment actions neither
      menu holds (`rejectRest`, `clearall`) in the keymap's order.
    - The `Filmstrip`, `Preview`, `Panes` and `General` groups keep the
      keymap's order, and no row leaves the page (an action the page does not
      know still lands in `General`).
    - The order of the strip's right-click menu, of the filter menu and of the
      backend's `DEFAULTS` is unchanged.
    - A unit test asserts the Judgment group's order with the default keymap,
      its menu part derived from `context.rs`'s judgment groups rather than a
      second hand-written list, and a test asserts the other groups equal the
      keymap's order filtered by group.
    - `docs/agents/native-app.md` ("The keymap is edited in the modal and
      replaced whole") and the `settingsmodal.rs` sentence of `CLAUDE.md` say
      the Judgment group follows the strip menu's order.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Do not touch `crates/backend/src/shortcuts.rs`: the companion's `KEYMAP`
      table and its test, the backend's defaults test and `get_settings`'
      `shortcuts` list follow `DEFAULTS` order. The ordering is the native
      page's own, like its grouping.
    - `crates/native/src/context.rs`: expose the judgment actions of
      `JUDGMENT_GROUPS` in order (for example a small
      `pub fn judgment_actions() -> impl Iterator<Item = &'static str>` that
      flattens the three groups), without changing `JUDGMENT_GROUPS` or
      `strip_groups`. This is the single source the page's order derives from.
    - `crates/native/src/settingsmodal.rs`: add the pure rule next to
      `shortcut_group`, for example
      `shortcut_rows(bindings: &[Binding], group: ShortcutGroup) -> Vec<&Binding>`:
      the bindings of the group in the keymap's order, and for
      `ShortcutGroup::Judgment` a stable sort by the action's position in
      `context::judgment_actions()`, an action not in it sorting after all of
      them (so it keeps the keymap's order and no row is lost). Put the unit
      tests in this module's existing `tests`, next to
      `every_action_has_its_own_group`.
    - `crates/native/src/settingsview.rs`, `SettingsView::shortcuts` (about
      line 734): replace the inline
      `bindings.iter().filter(|b| shortcut_group(b.action) == group)` with the
      new function, and update the doc comment above the method ("in the
      keymap's order").
    - Row ids: gpui-component renders the rows under `group-{ix}` /
      `item-{ix}`, so the reordering needs no id change; each row's handlers
      capture the action name, not its index.
    - No change is expected in `docs/humans/usage.md` (it does not describe the
      rows' order); check while implementing, and keep `usage.ja.md` in sync if
      a sentence does need changing.

## Trade-offs and risks

- **Where `rejectRest` and `clearall` go.** They belong to the `Judgment`
  group but are in neither menu. Decided with the user: after every menu item,
  in the keymap's order (`…, clearlabel, rejectRest, clearall`), so the rule is
  purely derived and needs no second list.
- **Deriving from `context.rs` vs. an explicit list.** Deriving makes
  `settingsmodal.rs` depend on `context.rs`, but the page then cannot drift
  from the menu.
- **The rating rows now run from 5 stars down to 0 stars**, as in both menus,
  so the keys `1` … `5` read bottom-up in the page. This is what the request
  asks for.
- **The companion's `get_settings` `shortcuts` list** stays in `DEFAULTS`
  order; the request does not cover it.

## Progress

- (2026-11-18) Step 1 complete
