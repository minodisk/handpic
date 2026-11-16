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

# Apply the Lightroom language preset on selection (native settings)

## Purpose

In the GPUI app's settings modal, the Sidecar page's `Lightroom language`
`Select` only records a language; the label names change when `Reset` beside
it is pressed. That is two actions for one intent, and the `Select` always
opens on the first language, whatever the names in force are.

After this work, choosing a language in the `Select` puts that preset's names
in force at once, through the path `Reset` takes today (`commit_names`), the
`Reset` button is gone, and the `Select` shows the language whose preset names
equal the label names in force, or its placeholder when none does. So the
`Select` states what the names are, and picking any language always applies.

## Steps

- [x] Step 1: Apply the chosen language's preset from the `Select`, show the
      matching preset in it, and remove `Reset`
  - Done when:
    - Choosing a language in the `Lightroom language` `Select` replaces the
      five label names with that preset's, shown in the fields and handed to
      the main view as `Change::LabelNames` exactly as `commit_names` does
      today (so persisted and put in force by `switch_names`); choosing the
      language already shown changes nothing.
    - There is no `Reset` button beside the `Select`; `reset_names` and the
      `language` field are removed, with any import they leave unused.
    - On every open of the modal, and after every commit of the names (a
      field's Enter / blur / Escape, or a language choice), the `Select`
      shows the preset whose names equal the names in force, else no
      selection (the placeholder). Committing an edit that makes the names
      match no preset clears the selection; committing an edit that makes
      them match a preset selects it.
    - A selection set by the view never applies names: only a user's choice
      in the menu does.
    - A pure helper that gives the index into `i18n::presets()` of the preset
      whose names equal given `LabelNames` (none when no preset matches) has
      unit tests: every preset's own names give its index, the default names
      give the English preset (index 0), and names differing from a preset in
      one color give none.
    - The search keywords of the language item no longer include `Reset`.
    - The doc comments in `settingsview.rs`, the `settingsview.rs` sentence
      in `CLAUDE.md` and the rules in `docs/agents/native-app.md` describe
      the new behavior and no longer mention `Reset` for the label names.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Native crate only: `crates/native/src/settingsview.rs` and
      `crates/native/src/settingsmodal.rs`, plus `CLAUDE.md` and
      `docs/agents/native-app.md`. Do not touch `crates/app` (frozen),
      `crates/core` or `crates/backend`.
    - The helper goes in `settingsmodal.rs` next to `label_texts`,
      `label_names_of` and `label_change`, with its tests in that file's
      `tests` module (e.g. `preset_of(names: &LabelNames) -> Option<usize>`,
      a `position` over `i18n::presets()` on `preset.names == *names`;
      `LabelNames` derives `PartialEq`). The names in force are already
      normalized (`label_names_setting`), so plain equality is the rule. The
      16 shipped presets are pairwise distinct, so the first match is the
      only one.
    - `show_names` is the one place `self.label_names` is set (`open` and
      `commit_names` both go through it), so syncing the `Select` there
      covers the open, a field commit and a language choice:
      `self.languages.update(cx, |state, cx| state.set_selected_index(
      preset_of(&names).map(IndexPath::new), window, cx))`. The theme pickers
      in `open` are the existing pattern for setting a `SelectState` from the
      view.
    - Re-entrancy, read from gpui-component 0.9.1 `src/select.rs`:
      `set_selected_index` / `set_selected_value` emit no `SelectEvent`;
      `Confirm` is emitted only by a user's confirm in the menu (and by
      `clean`, unused since the `Select` is not cleanable). So the sync in
      `show_names` cannot trigger the subscription, and no guard flag is
      needed. Verify this still holds while implementing (a choice applies
      once, and the open applies nothing).
    - The `SelectEvent::Confirm(Some(name))` subscription in `new` keeps its
      lookup of the preset by `preset.name` and calls
      `this.commit_names(preset.names.clone(), window, cx)` instead of
      setting `language`; the closure's window parameter, ignored today, is
      needed. The subscription runs as an effect after the `SelectState`'s
      own update, so updating `languages` from `commit_names` -> `show_names`
      inside it is not a nested update of the same entity; verify no
      double-borrow panic on a choice.
    - `SelectState::new` can keep `Some(IndexPath::new(0))` or take `None`;
      either way `open` sets the real selection before the modal is drawn.
    - In `sidecar`, the language item's field becomes the 160 px `Select`
      alone (the flex row and its `gap` existed for `Reset`); the `view`
      clone captured for the button's listener goes if nothing else in that
      closure uses it. Keywords become `["Language", "Lightroom"]`; the item
      keeps keywords, since a custom item is hidden during a search unless
      they match.
    - No `leave_field` call is needed on the `Select`: opening its menu moves
      the focus to the menu's list, so a focused name field blurs and commits
      first (the existing rule in `native-app.md`), and the choice then
      applies over the committed names.
    - `small_button` and `ClickEvent` are still used by the Keyboard
      Shortcuts page; remove only what this change leaves unused.
    - Docs to update: `CLAUDE.md`'s "(`Input` fields, the Lightroom language
      a `Select` beside `Reset`)"; in `docs/agents/native-app.md` the
      language picker rule ("built in `new` with the first chosen", "sets
      `language`, the preset `Reset` fills in", "cannot share its item with
      `Reset`"), the look rule ("holding the `Select` ... and `Reset`") and
      the label names rule ("a row of `i18n::presets()` languages and
      `Reset`", "or `Reset` normalizes the five"). `docs/humans/usage.md`
      does not describe this control and needs no change.

## Trade-offs and risks

- **When an edit clears the `Select`.** This plan clears it at the field's
  commit (Enter, blur, Escape), not per keystroke: the names in force change
  only at a commit, and opening the `Select`'s menu blurs and commits a
  focused field, so a pick always applies over the committed state. Clearing
  while typing would need an `InputEvent::Change` handler on the five fields
  comparing their normalized texts with the presets, and care that `show`'s
  `set_value` may itself emit `Change`. Not taken as the larger change.
- **The placeholder's text.** With no selection the `Select` shows
  gpui-component's default placeholder (`t!("Select.placeholder")`). An
  explicit `.placeholder(..)` (e.g. `Custom`) would read better but is new UI
  text nobody asked for. This plan keeps the default.
- **Picking the language already shown is a no-op** (`label_change` gives
  none), consistent with the `Select` showing a language only while the names
  equal its preset.
- **No explicit undo.** Without `Reset`, a mis-pick replaces hand-edited
  names at once with no confirmation, as `Reset` did after one more click;
  the previous names are not kept anywhere.
- **The helper's home.** `settingsmodal.rs` rather than `riffle-core`'s
  `i18n.rs`, which the frozen Tauri app also compiles.

## Progress

(2026-11-17) Step 1 complete
