# Learnings

## Step 1

- `context::Item.label` now reuses `menu::Label` (text items as `Owned`, the
  judgment rows' fixed texts as `Text`, the rating rows as `Stars`), so
  `main.rs` draws both menus' labels through one `menu_label` helper and the
  pin test can compare the rating rows' labels with `filter_rows`' directly.
  `folder_groups` items stay plain text (`Owned`).
- The pin test maps each judgment action to the filter `Toggle` it stands
  for (`pick` to `Picked`, `unflag` to `Untagged`, `clear` to `Stars(0)`,
  `clearlabel` to `Label("none")`, a color to its own key) and compares the
  sequences, so a reorder on either side fails it.
- A bash heredoc holding a Python script with many apostrophes inside
  triple-quoted strings failed to parse when nested in `python - <<'EOF'`
  after another command; writing the script to the scratchpad first worked.

- Clippy (`explicit_auto_deref`) refused `Action::Judge(*action)` over the
  `&&'static str` the const table's iterator gives; `Action::Judge(action)`
  coerces, since the table is a `'static` const.
- In a fresh worktree `mise run fmt` failed on the missing
  `node_modules/vite-plus` (its `vp fmt` half); `mise exec -- pnpm install
  --frozen-lockfile` fixed it (plain `pnpm` is not on the Git Bash `PATH`).

## Deferred issues (todo candidates)

- Pending manual check (Step 1; the step's checkbox was ticked on the
  automated criteria): on Windows, run `mise run native:dev` with
  `--settings` and `--log` pointing into a test folder (as
  `docs/agents/native-app.md` requires), open a folder, right-click a strip
  cell of a picked, rated (for example 3 stars), labeled file, and confirm:
  the flag group reads Pick, Unflag, Reject; the rating group shows five
  rows of 5 to 1 lit yellow stars then a row of five dim stars; the label
  group is Red to Purple, then No label; the checks sit on Pick, the 3-star
  row and the file's label; each row shows its shortcut at the right end
  (`p`, `u`, `x`, `5` … `1`, `0`); clicking a star row sets that rating.
  Files: `crates/native/src/context.rs`, `crates/native/src/main.rs`.
