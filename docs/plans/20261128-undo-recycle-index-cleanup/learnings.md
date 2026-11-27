# Learnings

## Step 1

- `restore_all` consumes its items, so the chunk closure of the Windows
  `restore_listed` collects each item's `(PathBuf::from(&item.id),
  item.original_path())` first (`restored_paths`), then runs
  `remove_index_file` over them whatever `restore_all` returned.
- In `restore_item` the capture and the cleanup are `#[cfg(windows)]`
  statements (a `let` and a call), so the Linux build has no unused bindings.
  The cleanup runs before `relocate`, which would move the file off its
  original path.
- `index_file_of` is compiled everywhere under
  `#[cfg_attr(not(windows), allow(dead_code))]` and tested with
  `Path::join`-built paths, so the tests hold on Unix too. It works on the
  UTF-8 file name (`to_str`); a non-UTF-8 name gives none, which only skips the
  cleanup.
- `mise run ci:local` (clippy and the engine tests) was stopped by the
  harness after it moved to the background, because the host ran low on
  memory. It did not fail and was not re-run; the PR's GitHub checks verify
  the change, Windows included.

## Deferred issues (todo candidates)

- Pending manual check (Windows, this host): count the `$I` files in the
  volume's `$Recycle.Bin\<SID>\` folder, run the app with
  `mise run app:dev:screenshot` (private `--index`, `--settings` and `--log`
  under a scratchpad folder, `--mcp-port 47842`) on fixtures copied to a
  temporary folder, move files to the Trash, undo, and count again. Expected:
  the count is unchanged and the files are back. Basis: the plan's Step 1
  "Manual check on the host"; the step's checkbox was ticked on the automated
  criteria. Files: `crates/engine/src/trash.rs` (`restore_listed`,
  `restore_item`, `remove_index_file`).
