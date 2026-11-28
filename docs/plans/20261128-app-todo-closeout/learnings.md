# Learnings: app-todo-closeout

## Step 1

- The strip's `Copy Path` now joins the targets through
  `context::copied_paths`, which maps each through `context::shown_path`.
  The UI test `the_strip_menu_takes_the_arrows_and_enter_and_hands_other_keys_on`
  compares against `shown_path` of the listed path; only on Windows does the
  listed path carry the prefix, so the OS-independent check is the unit test
  `copied_paths_drop_the_verbatim_prefix_one_per_line` in `context.rs`.
- A bare `python -` in Git Bash hung waiting on stdin (CLAUDE.md's warning);
  edits are simpler through the Edit tool than a Python heredoc with raw
  Windows paths, whose `\U` broke a non-raw Python string.
- The UI test's `b` is also the `judgments` key (the listed path) of its
  last assert; replacing `b` with the shown path failed that assert on
  Windows (`None` for the pick), so the clipboard expectation is a separate
  `copied`.
