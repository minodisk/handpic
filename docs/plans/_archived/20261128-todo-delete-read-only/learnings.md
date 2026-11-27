# Learnings

## Step 1

- The two tests were appended after `a_chunked_restore_reports_progress_per_chunk`
  as the last items of `mod tests`; no other line of
  `crates/engine/src/trash.rs` changed (no `use` edit: `std::fs` and
  `std::process::Command` are spelled by full path).
- The macOS test reads `remove_one`'s result and the file's existence into
  locals and runs `chflags nouchg` before any assertion, so a failure never
  leaves a locked file behind for the next run's `temp_dir`.
- Local checks: per the user's standing instruction no cargo build or test
  ran locally. The tests are verified only by the PR's `test` job on
  `ubuntu-latest`, `macos-latest` and `windows-latest`; if test (a) fails on
  one of them, follow the plan's "Trade-offs and risks" fallback.
- The `docs/agents/shared-crates.md` bullet names its source at the plan's
  archived path, where the wrap-up moves the folder.
