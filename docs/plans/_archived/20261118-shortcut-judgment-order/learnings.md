# Learnings

## Step 1

- `Keymap::bindings()` returns an owned `Vec<Binding>`, so `shortcut_rows`
  takes `&[Binding]` and the tests bind the vector before borrowing it.
- `context::judgment_actions` flattens `JUDGMENT_GROUPS` and is the only
  source of the Judgment rows' order; `shortcut_rows` sorts stably by the
  action's position in it, `usize::MAX` for the rest, so `rejectRest` and
  `clearall` keep the keymap's order after the menu items.
- `docs/humans/usage.md` does not describe the rows' order, so it and
  `usage.ja.md` needed no change.
