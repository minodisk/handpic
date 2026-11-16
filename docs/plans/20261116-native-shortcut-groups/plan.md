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

# Native Keyboard Shortcuts Page Groups

## Purpose

The native app's settings modal lists all 41 keymap actions on the Keyboard
Shortcuts page as one flat list, in the keymap's order. Finding an action means
reading the whole list. Grouping the rows under headings named after the area
an operation acts on (the filmstrip, the judgment, the preview, the panes, the
app) lets the user go straight to the part they want to rebind.

## Steps

- [x] Step 1: Group the Keyboard Shortcuts page's rows by operation area
  - Done when:
    - The Keyboard Shortcuts page shows its rows under these headings, in this
      order, each group keeping the keymap's order (`Keymap::bindings()`)
      within it, with `Reset all` after the last group:
      1. `Filmstrip`: `previous`, `next`, `burstPrevious`, `burstNext`,
         `burstFramePrevious`, `burstFrameNext`, `extendPrevious`,
         `extendNext`, `selectAll`
      2. `Judgment`: `rate1` to `rate5`, `reject`, `rejectRest`, `pick`,
         `unflag`, `clear`, `red`, `orange`, `yellow`, `green`, `blue`, `pink`,
         `purple`, `clearlabel`, `clearall`
      3. `Preview`: `focus`, `zoom`, `grayscale`, `compare`, `performance`,
         `peaking`
      4. `Panes`: `toggleLeft`, `toggleRight`, `toggleStrip`, `toggleSides`
      5. `General`: `open`, `undo`, `redo`
    - The category of an action comes from pure code in
      `crates/native/src/settingsmodal.rs`, with unit tests that: every action
      of `Keymap::defaults().bindings()` has a category by an explicit match
      arm (not the fallback); every category holds at least one action; the
      categories' order and titles are the ones above.
    - Adding, removing and resetting a key, `Reset all`, the `APP_KEYS`
      refusal and the key capture (`Press a key...`) behave as before, checked
      by running the app (`mise run native:dev`) and editing a key in at least
      two different groups.
    - The settings search still finds an action by its label and `Reset all`
      by its keyword; a group none of whose rows match is not drawn.
    - `docs/agents/native-app.md` and `CLAUDE.md` describe the grouping.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Change only `crates/native` and the docs. `crates/app` (frozen) and
      `crates/backend/src/shortcuts.rs` stay as they are: the keymap's
      `DEFAULTS` order is unchanged; the grouping is the native page's own.
    - Pure model, in `crates/native/src/settingsmodal.rs` next to
      `shortcut_label`: an enum of the five categories (e.g. `ShortcutGroup`),
      a const array giving their order (as `TABS` does for `Tab`), a
      `title()` per category, and a function from an action name to its
      category (e.g. `shortcut_group(action: &str) -> ShortcutGroup`). As a
      total function it gives each action exactly one category by
      construction. Like `shortcut_label`, it needs a fallback for a name it
      does not know (a later backend action), so no row can vanish from the
      page: fall back to `General`. To let the test tell an explicit
      assignment from the fallback, either have an inner function return
      `Option` and the test assert `Some` for every default binding, or
      equivalent; follow the existing `every_action_has_its_tauri_label` test
      for shape.
    - View, in `SettingsView::shortcuts` of `crates/native/src/settingsview.rs`:
      build the rows as today, then emit one
      `SettingGroup::new().title(<category title>)` per category holding the
      rows of that category (filter `self.keymap.bindings()` by category, so
      the keymap's order is kept), then one last untitled group holding only
      the `Reset all` item with its `keywords(["Reset all"])`. gpui-component
      0.9.1's `SettingGroup::title` is already used on the Sidecar page
      (`Label names`); `SettingPage::group` / `groups` take several groups.
    - Known behavior of gpui-component 0.9.1 (`src/setting/` of the crate) to
      rely on, not rebuild:
      - `Settings` draws only the groups with a matching item
        (`group.is_match`), so the search hides empty groups and their
        headings by itself.
      - Each group renders under the element id `group-{ix}` and each item
        under `item-{ix}`, so the rows' repeated button ids (`"add"`,
        `"reset"`, `("remove", k)`) stay unique although the item index now
        restarts in every group.
      - A titled group also becomes a sub-entry of its page in the settings
        sidebar (see "Trade-offs and risks").
    - Verify while implementing: the capture still works on a row of a later
      group (the `capturing` state is keyed by the action name, so it should);
      the focus goes back to the box after a `×` as before; the page scrolls
      to a group when its sidebar sub-entry is clicked.
    - Docs: in `docs/agents/native-app.md`, update the three places that
      describe the page as a flat list ("the Keyboard Shortcuts page one item
      per action, then `Reset all`"; the section "The keymap is edited in the
      modal and replaced whole", which says the page lists
      `Keymap::bindings()` in order; and the search rule that names the
      shortcut items), stating the groups, their order, the fallback and the
      sidebar sub-entries. In `CLAUDE.md`, extend the `src/settingsmodal.rs`
      clause (which lists "the pages' order and titles, ... the keymap edits
      refusing `APP_KEYS`") with the shortcut groups, and the Keyboard
      Shortcuts page mention in the `src/settingsview.rs` clause if needed.
      No `docs/humans/` page mentions this page, so none changes.

## Trade-offs and risks

- **One `Judgment` group (decided by the user).** All 19 judgment actions sit in
  one `Judgment` group: the operation's target is the photo's judgment, and
  `clearall` (which clears stars, flag and label at once) has a natural home.
  Splitting into `Rating` / `Flag` / `Color label` was rejected.
- **Group order (decided by the user).** Filmstrip, Judgment, Preview, Panes,
  General, by how central they are to culling. Within a group the keymap's
  order is kept.
- **Group names.** `Filmstrip` holds both navigation and selection (they act
  on the same area). No keymap action targets the folder tree (its keys are not
  in the keymap), so there is no folder-tree group.
- **Sidebar sub-entries.** With gpui-component's `SettingGroup::title`, every
  titled group is also listed under `Keyboard Shortcuts` in the settings
  sidebar, and a click scrolls to it (as `Label names` already is under
  `Sidecar`). Accepted.
- **Unknown action fallback.** An action the mapping does not know falls back
  to `General` rather than being dropped; the unit test fails first, so this
  is only a guard for a build where the backend gained an action.
- **Actions the native app may not act on.** The search found no `compare` or
  `grayscale` match in `crates/native/src/main.rs`; they stay listed (under
  `Preview`) as today. Hiding or implementing them is out of scope.

## Progress
