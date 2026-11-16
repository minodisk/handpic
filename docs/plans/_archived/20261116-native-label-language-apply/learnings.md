# Learnings

## Step 1

- gpui-component 0.9.1's `SelectState::set_selected_index` /
  `set_selected_value` emit no `SelectEvent`; `Confirm` comes only from a
  user's confirm in the menu (and `clean`), so the view's sync of the
  selection in `show_names` needs no guard against applying names.
- `set_selected_index` looks the item up in the delegate's matched items,
  while `set_selected_value` clears the search query first. The sync uses
  `set_selected_value` with the preset's name for a match and
  `set_selected_index(None)` for none, so a leftover query could not shift
  the index.
- Not verified by hand here: a choice in the running app applying once with
  no double-borrow panic. The reasoning (the subscription runs as an effect
  after the `SelectState`'s own update) is in `docs/agents/native-app.md`.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the native app): open the settings
  (`ctrl+,`), Sidecar page with the format XMP. Expected: the `Lightroom
  language` `Select` shows English with the default names; choosing `日本語`
  fills the five fields with the Japanese names at once and the strip's label
  names follow; editing one field and pressing Enter clears the `Select` to
  its placeholder; typing the English names back and committing selects
  English again; reopening the modal shows the language of the names in
  force; no panic on any choice. Basis: Step 1's Done-when (the UI behavior
  is not exercised by the automated tests, which cover `preset_of` only).
  Files: `crates/native/src/settingsview.rs`. Step 1's checkbox was ticked
  on the automated criteria.
