# Learnings

## Step 1

- `SettingGroup::items` takes any `IntoIterator<Item = SettingItem>`, so the
  view builds the row closure once and maps each group's filtered
  `bindings()` through `&row`; `Binding::keys` is cloned per row since the
  bindings are now iterated by reference, once per group.
- The `SHORTCUT_GROUPS` fold keeps the page builder's chain; the `Reset all`
  item moved into its own untitled group after the last one.

## Deferred issues (todo candidates)

- **Pending manual check (Step 1, the user's).** Platform: Windows (any
  platform the native app runs on). Run `mise run native:dev`, open the
  settings (`ctrl+,`), go to Keyboard Shortcuts and check: the rows sit under
  `Filmstrip`, `Judgment`, `Preview`, `Panes`, `General`, then `Reset all`;
  in at least two groups (e.g. `Filmstrip` and `Panes`) add a key with `+`
  (`Press a key...` shows, the key is added), remove it with its `×` (the
  focus returns to the box), `Reset` a row, and `Reset all`; an `APP_KEYS` key
  (e.g. `ctrl+,`) is refused as before; the sidebar lists the five groups
  under `Keyboard Shortcuts` and a click scrolls to the group; searching an
  action's label shows only its group, and `Reset all` is found by its
  keyword. The step's checkbox was ticked on the automated criteria (unit
  tests, `mise run ci:local`). Files: `crates/native/src/settingsview.rs`,
  `crates/native/src/settingsmodal.rs`.
