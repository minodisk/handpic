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

## Step 3: the settings modal's shell and the Culling tab

- `crates/backend/src/ai.rs` was on `origin/main` (#1006), so the AI toggle
  is wired through `riffle_backend::ai::Ai` rather than persist-only:
  `Ai::switch` downloads the model on an enable, the status line is a port of
  `ai.ts`'s `aiStatus` (`settingsmodal::ai_status`, its tests ported), and the
  note under the toggle says the analysis does not run in this app yet.
  `AiState`'s fields were private (the Tauri app only serialized it), so they
  became `pub`; nothing else in the backend changed.
- Where the model goes: `datadir::model_path`, `<local data dir>/<IDENTIFIER>.native/<MODEL_FILE>`
  (the `.native` folder the index already uses, apart from the Tauri app's
  copy under `<local data dir>/<IDENTIFIER>`), or `--data-dir <path>`, a new
  flag added so a hand check that turns AI on never downloads into a real
  data dir. `Ai::new` runs at startup and only reads that folder (it removes
  stale `.part` files); the headless smoke run below created nothing there.
- Decided: no resume of a cut-short download at launch (the Tauri app calls
  `switch(true)` when `aiEnabled` is on): nothing in the native app uses the
  model, and a seeded `aiEnabled: true` would otherwise fetch 378 MB on the
  first start. Recorded in the plan's Step 3 text and its trade-offs.
- The `Ai` handle lives in `SettingsView`, not the main view: its state
  callback runs on the download thread and goes through a channel of the view
  (`futures::channel::mpsc`, as the scan's events do), which notifies only
  while the modal is open. The main view still owns the values in memory and
  writes `autoAdvance` / `aiEnabled` from its `subscribe_in` callback.
- Read from GPUI at the pinned rev (`b5cd310b`): a `KeyDownEvent` runs the
  capture phase root first, then the bubble phase from the focused node up
  (`window.rs` `dispatch_key_down_up_event`), so the box's `on_key_down` with
  `cx.stop_propagation()` keeps every key from the root's handler; the root's
  guard covers a focus left on the root. Tab is not built in:
  `window.focus_next` / `focus_prev` walk the window's tab stops and wrap
  (`tab_stop.rs`), and the main view's focus handle is not a tab stop, so the
  cycle stays in the modal. Enter / Space on a focused element with `on_click`
  become a `ClickEvent::Keyboard` (`div.rs`) for free. `occlude()` is
  `HitboxBehavior::BlockMouse`, which `hit_test` stops at, scroll included.
  `Hitbox::is_hovered` is false right after keyboard input
  (`last_input_was_keyboard`); a mouse press sets the modality first, so it
  does not affect the clicks.
- `cycleFocus` of `modal.ts` was not ported: GPUI's `focus_next` wraps on its
  own, and the function would have no caller (clippy `dead_code`).
  `tabs.ts`'s horizontal orientation was dropped for the same reason; the
  vertical cases are ported on the four tabs, and the Debug-tab cases have no
  counterpart.
- `key_down` gained a `window` parameter (to focus the modal's selected tab
  on open), so the root's listener passes it; `Riffle::new` gained the
  `model` path; `start`'s `Args` destructure gained `data_dir: _`. These are
  the Ai wiring beyond the plan's five named `main.rs` changes.
- Headless smoke run (debug exe, test paths only): `riffle-native.exe
  D:\Photos\tests\20261113-settings\photos --index ...\index-step3.sqlite
  --settings ...\step3.sqlite --data-dir ...\data --log ...\step3.log --auto 2`
  logged `aiEnabled=false` in the `settings:` line and `embedding model:
  D:\Photos\tests\20261113-settings\data\models/siglip2-base-patch32-256/vision_model.onnx`,
  paged twice and quit; no `data` folder was created.
- A stray `python - </dev/null` (typed by mistake ahead of a `perl` edit)
  still hung in the Bash tool and was moved to the background; it was left
  running, not killed. The edit was redone with the Edit tool.

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
- Pending manual check (Step 3, Windows, the user's): the settings modal's
  shell and the Culling tab. Basis: Step 3's Done-when (the open / close
  paths, no key or press reaching the window behind it, the auto-advance
  toggle taking effect and surviving a restart); the checkbox was ticked on
  the automated criteria (`settingsmodal.rs`'s tests, `settings.rs`'s
  `aiEnabled` read, `datadir.rs` / `args.rs` tests, clippy). Steps, on copies
  under `D:\Photos\tests\20261113-settings\` only, after `cargo build` in
  `crates/native`:
  1. `target\debug\riffle-native.exe D:\Photos\tests\20261113-settings\photos --index D:\Photos\tests\20261113-settings\index-s3a.sqlite --settings D:\Photos\tests\20261113-settings\step3-hand.sqlite --data-dir D:\Photos\tests\20261113-settings\data --log D:\Photos\tests\20261113-settings\Handpic.log`
  2. Open `Filter ▾`, then press Ctrl+,. Expected: the menu closes and the
     Settings box appears centered over a dimmed window, the Sidecar tab
     selected (empty), Culling, Keyboard Shortcuts and Import below it.
  3. With it open, press the arrow keys, `q`, `1` and `x`, click a strip
     cell and scroll the wheel over the strip. Expected: the preview, the
     strip and the file's rating do not change and the app does not quit.
     With the Sidecar tab focused, ArrowDown / ArrowUp / Home / End move the
     selected tab; Tab / Shift+Tab move a visible focus border through the
     close button, the selected tab and (on Culling) the two checkboxes,
     wrapping; Space or Enter on a focused checkbox flips it.
  4. Close it with Escape, open it again and close with the `×` button, then
     open and close with a press on the dimmed area outside the box. After
     each close, ArrowRight pages to the next file.
  5. On Culling, turn auto-advance off and close. Press `1`: the rating is
     set and the file stays. Quit, start again with the same `--settings`
     and a fresh `--index` (`index-s3b.sqlite`). Expected: the startup
     `settings:` line has `autoAdvance=false` and the checkbox is off.
  6. Optional, needs the network and 378 MB under the test `data` folder:
     on Culling turn AI on. Expected: the status line counts `Downloading
     the model: N / 378 MB`, then `On`; the file lands under
     `D:\Photos\tests\20261113-settings\data\models\`; turning it off
     mid-download leaves no `.part` file. The startup line of the next run
     shows `aiEnabled=true`.
