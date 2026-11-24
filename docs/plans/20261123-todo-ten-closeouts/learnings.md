# Learnings: todo-ten-closeouts

## Step 1

- The settings key passes the tree's gate by name in `main.rs`'s `key_down`
  (`name != settingsmodal::OPEN_KEY` ahead of `tree_key_down` and
  `tree_gate`), not through `treekeys::tree_gate`: `ctrl+,` is no keymap
  action, so giving the gate the key name would have widened its signature
  for one key. The UI test `the_settings_key_opens_the_settings_from_the_tree_but_not_from_a_rename`
  pins both halves.
- The UI test cannot count on the open folder having a drawn tree row (the
  roots are the home folder and the volumes, and the test's temporary folder
  is under neither on every OS), so it pushes a row for the folder into
  `tree_rows` before `start_rename` when the tree does not draw one.
- `set_view`'s `compare` now fails in two places: the engine's `ViewMode`
  serde enum (the MCP argument parsing, `set_view_takes_normal_or_zoom_and_refuses_compare`)
  and the companion's own mode check (`set_view_refuses_compare_as_an_unknown_mode`).
- A heredoc passed through the Bash tool lost a `\` before a newline in a
  Python edit script; scripts written with the Write tool kept it.
