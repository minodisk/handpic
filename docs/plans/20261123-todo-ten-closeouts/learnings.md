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

## Step 2

- `openness` is shared as `meta::shown_openness` (the rounded integer, a
  `u8`), which the meta pane's `Eyes` row and `get_view` both call; the
  private `openness` / `js_round` stay where they were. The stored answer
  reads it from `StoredEyes::eyes_ear`, the judgment's from
  `EyesJudgment::ear`.
- The trash failure lines live in one tagged list, `trash::Failures`, keyed
  by (origin, path): a trash failure no longer replaces the same path's
  judgment line, so the next run's `run_started` drops only the `Trash` lines
  and "judgment lines stay" holds for a path that has both. The pane's order
  is still the order each line first failed.
- `write_failed` has more callers than the plan listed: `folderops.rs` pushes
  the sequencer's, the import's and the date split's failures. They are
  `Origin::Other` (kept as before, cleared only by a folder open), so the
  origins are `Trash` / `Other` rather than run / judgment.
- `run_started` is called in `run_trash`, `undo_trash` and `redo_trash`
  after their guards (where the run really starts), so a run held by the
  gate or a dialog still asking clears nothing. A `Delete Sidecars…` run's
  start (`run_sidecars` with `Kind::Delete`) calls it too: its result ends
  through `trashed` and its failures are `Trash` lines, the same family as
  its redo.
