# Learnings: gpui-settings

## Step 1: the settings store

- `riffle_backend::store::Store::open` reads `user_version` before it sets
  `journal_mode = WAL`, so a store with a newer schema is refused before its
  header is touched (switching a rollback-journal database to WAL rewrites
  the header). The schema and the seed are then written inside a
  `BEGIN IMMEDIATE` transaction that re-reads `user_version`, so two processes
  opening one new store cannot both seed it; `PRAGMA user_version` is
  transactional, so the store is never found with the table but no version.
- Opening a JSON file with `Connection::open` succeeds; the first query
  (`user_version`) fails with "file is not a database" and the file's bytes
  stay as they were (tested). A zero-byte file reads `user_version = 0` and is
  seeded like a missing one.
- The backend has no `tempfile` dev-dependency; the store's tests follow
  `index.rs`'s `temp_dir` helper (a per-process folder under the OS temp dir)
  instead of adding one.
- `Settings` gained `store: Option<Store>`. Nothing reads it yet in a non-test
  build, which clippy's `dead_code` refuses, so `main.rs`'s startup
  `settings:` line now ends `store=open` / `store=none`. `main.rs` also needed
  `import_settings: _` in `Riffle::start`'s exhaustive `Args` destructure, one
  line beyond the planned load call and log line.
- The seed note (`settings store: new, seeded N keys from <path>`, or
  `nothing to seed; tried: …`) is captured inside the seed closure and logged
  only after the store opened, so a failed commit does not log a seed.
- Disk: the first `mise run native:check` failed with os error 112 (C: had
  356 KB free, the build outputs of several worktrees). This worktree's own
  `target/` and `crates/native/target/` were removed (nothing of the other
  worktrees was touched), which left 43 GB free, and the check then passed.
- A bash heredoc holding the new `settings.rs` failed with "unexpected EOF
  while looking for matching `'`" in the Bash tool; writing the file with the
  Write tool worked. A bare `python3` probe hung (as `todo.md` already notes)
  and was moved to the background.
- Hand check (headless, debug exe, all paths under
  `D:\Photos\tests\20261113-settings\`: two ARW copies in `photos`, `--index`
  a fresh file per run, `--settings settings.sqlite`, `--import-settings
  seed.json`, `--log Handpic.log`, `--auto 2`): the first run, with
  `{"sidecarFormat":"both","sortOrder":"rating"}`, logged `settings store:
  new, seeded 2 keys from …\seed.json` and `settings: sidecarFormat=both …
  sortOrder=rating … store=open`. After the JSON was edited to `"dop"`, the
  second run logged no seed line and still `sidecarFormat=both`; the JSON's
  SHA-256 was the same before and after that run (the bytes the edit left).
