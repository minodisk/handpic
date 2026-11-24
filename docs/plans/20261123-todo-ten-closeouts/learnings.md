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

## Step 3

- `Latest` is no longer generic: its one user pushed `(dir, path)`, and the
  per-folder pending list needs the folder as the key, so `push(dir, path)`
  follows `Batches`' shape (each folder once, in first-push order, the
  latest file of each).
- The launch eviction is `scan::Eviction` (`Arc<Mutex<Option<JoinHandle>>>`,
  cloned into every listing's background task): the first waiter takes the
  handle and joins it under the lock, so a second waiter blocks on the lock
  until the join ends. `uitest.rs`'s `opens_with_no_folder` reads it through
  `Eviction::is_finished`. The `settled` oneshot chain is unchanged.
- `Eviction::is_finished` is `#[cfg(test)]`: only the UI test and the unit test read it, and clippy's `dead-code` failed the first `ci:local` run without it.
- The first draft of `every_waiter_waits_for_the_eviction` deadlocked: it
  called `is_finished` while a waiter held the lock through its join, so the
  release was never sent (the run showed the test "running for over 60
  seconds"). `is_finished` blocks while a listing joins; the test checks the
  thread's own `done` flag before the release instead. The second
  `ci:local` run was killed by the harness for low memory with that test
  hanging (the steps before `test app` had passed, since it got that far;
  only the output's last 30 lines were kept, all passing tests). The fix was
  committed without another run, the PR's GitHub checks verifying it.
