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

## Step 2: persist the strip's filter and sort order

- The write path is one `settings-store` thread fed by a `std::sync::mpsc`
  channel (`settings::Writes`), the shape of `judgment::Queue`, not
  `viewed.rs`'s `Latest`: a channel keeps every key's order by construction
  and its barrier message gives the quit drain for free, where a `Latest`
  would be needed per key plus a way to wait on all of them. `Store` holds a
  `rusqlite::Connection`, which is `Send`, so the store itself moves into the
  thread; `Settings::store` became `Settings::writes`, and the Step 1 tests
  that read the store back now reopen the file (`values(path)`).
- `Writes::delete` has no caller until Step 5 (the shortcut editor deletes
  `shortcuts`); clippy's `-D dead-code` refused it, so it and its `Write`
  variant carry `#[cfg_attr(not(test), expect(dead_code, reason = …))]`.
  Step 5 removes the attribute.
- The settings drain is its own `App::on_app_quit` handler, not a line in the
  judgment stores' one: that handler is registered only when the index
  opened, and a filter change made with no index must still be drained.
- `menu_action` compares `FilterState::stored()` before and after
  `menu::apply`, so a change of `Output` or an EXIF check alone (both per
  session) writes nothing, and `Reset` writes only when a remembered section
  was checked. The stored value always holds all six sections, as the Tauri
  `filter_setting` writes it.
- A python one-liner probe run without `</dev/null` hung on the Windows
  console (`_pyrepl` `WinError 123`) and was moved to the background; scripts
  run as `timeout 30 python -I <script> </dev/null` worked.

## Deferred issues (todo candidates)

- Pending manual check (Step 2, Windows, the user's): the sort order and the
  filter survive a restart. Basis: Step 2's Done-when ("A restart with the
  same `--settings` store starts with the filter and the sort order the last
  run ended with"); the checkbox was ticked on the automated criteria (the
  `settings.rs` writer tests and `filter.rs`'s round trip). Steps, on copies
  under `D:\Photos\tests\20261113-settings\` only, after `cargo build` in
  `crates/native`:
  1. `target\debug\riffle-native.exe D:\Photos\tests\20261113-settings\photos --index D:\Photos\tests\20261113-settings\index-a.sqlite --settings D:\Photos\tests\20261113-settings\step2.sqlite --log D:\Photos\tests\20261113-settings\Handpic.log`
     (a fresh `step2.sqlite`, nothing seeded).
  2. In `Sort ▾` pick `Rating`; in `Filter ▾` check `Picked` and `Landscape`,
     and also an `Output` or EXIF item if shown. Quit (close the window); the
     log shows `quit: settings writes drained=true`.
  3. Start again with the same `--settings` and a fresh `--index`
     (`index-b.sqlite`). Expected: the startup `settings:` line shows
     `sortOrder=rating` and a filter with `flags: [Picked]` and
     `orientations: [Landscape]`; the sort menu has `Rating` checked, the
     filter button is lit with `Picked` and `Landscape` checked, and the
     `Output` / EXIF items are unchecked.
  4. `Filter ▾` → `Reset`, quit, start again with a fresh `--index`
     (`index-c.sqlite`). Expected: the `settings:` line shows an empty
     filter and the filter button is not lit; the sort is still `Rating`.
