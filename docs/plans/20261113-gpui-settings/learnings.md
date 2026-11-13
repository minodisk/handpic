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
- Scope change (the user, 2026-11-14): the AI features are not ported to the
  native app at all (`aiEnabled` with the model download, pass 3, genres,
  the ranking, `recommended`). The AI toggle, its `aiStatus` line and note,
  `Change::AiEnabled`, the `Ai` handle, `datadir::model_path`, the
  `--data-dir` flag and `Settings::ai_enabled` were removed, and
  `AiState`'s fields went back to private, so the backend is unchanged by
  this step. The bullets above about the AI wiring describe what was
  removed. A seeded `aiEnabled` key stays in the store, unread.

## Step 4: the Sidecar tab, the sidecar format switched live

- Nothing of the switch had moved on `origin/main` (`d72386f4`). The shared
  body is the backend's `sidecar::switch_to(format, writer, index, persist)`:
  the flush, the persist (a failure logged) and `Index::reset_sidecars`. The
  current-format check and the state write stay with each caller, since the
  native view's format is main-thread state and the rest has to run off it:
  the Tauri `switch_format` keeps its `Mutex<SidecarFormat>` check and write
  and then calls `switch_to`; its race tests stay in `crates/app` unchanged.
  The backend gained two tests next to `switch_to` (a queued write of the old
  format lands before the persist; clean rows dropped, a dirty row keeps its
  judgment and loses the stat; a failed persist still resets), and the "no
  change for the same format" test is `settingsmodal::format_change`'s, the
  check the view applies before emitting (the window re-checks it in one
  line).
- The native app has a step the Tauri app does not: the judgment stores
  queue (`judgment::Queue`). A store submitted before the switch carries the
  old format and may not have reached the writer when it is flushed, so the
  switch drains the queue first (`Queue::drain`, a barrier). A judgment from
  a key or from the MCP companion's `set_judgment` after the click goes
  through the same `store` with `self.settings.format`, already the new one.
- The Tauri reopen on `sidecar-format` (`openDirectory`) clears `touched`
  and removes the judgment entries from its undo / redo stacks; the native
  `relist` carries both, so the switch's completion clears `touched`, the
  `Undo` histories and the step note before calling it. On a failure the
  Tauri command returns before emitting `sidecar-format`, so the native
  switch puts the message on the modal's status line and does not re-list.
- One switch at a time: a `format_switch: Arc<Mutex<()>>` field held by the
  background part, the `AppSwitchLock` counterpart. `main.rs` gained that
  field, the `Change::SidecarFormat` arm, the format passed to `open`, the
  `SidecarFormat` import and the `switch_format` method.

## Deferred issues (todo candidates)

- Pending manual check (Step 4, Windows, the user's): the sidecar format
  switched live. Basis: Step 4's Done-when; the checkbox was ticked on the
  automated criteria (`sidecar.rs`'s `switch_to` tests, `settingsmodal.rs`'s
  `format_change` tests, the Tauri race tests, clippy). Steps, on copies under
  `D:\Photos\tests\20261113-settings\` only, after `cargo build` in
  `crates/native`:
  1. Prepare `D:\Photos\tests\20261113-settings\formats\` with two or three
     ARW copies, each with a `.xmp` and a `.ARW.dop` holding different
     ratings (e.g. XMP 2, `.dop` 4), and `D:\Photos\tests\20261113-settings\jpegs\`
     with a few JPEGs only. Write `step4-seed.json` there holding
     `{"sidecarFormat":"xmp"}`.
  2. `target\debug\riffle-native.exe D:\Photos\tests\20261113-settings\formats --index D:\Photos\tests\20261113-settings\index-s4a.sqlite --settings D:\Photos\tests\20261113-settings\step4.sqlite --import-settings D:\Photos\tests\20261113-settings\step4-seed.json --log D:\Photos\tests\20261113-settings\Handpic.log`
     (a fresh `step4.sqlite`). Expected: the strip shows the XMP ratings.
  3. Ctrl+, → Sidecar: `Lightroom (.xmp)` is selected. Choose
     `PhotoLab (.dop)`. Expected: the log has `sidecar format: xmp -> dop`
     then a `resync: dir=…` re-listing; the strip and the pane show the
     `.dop` ratings. Choosing `PhotoLab (.dop)` again logs nothing.
  4. Close, press `5` on a file. Expected: only its `.ARW.dop` changes (the
     `.xmp`'s mtime and bytes stay).
  5. Switch to `Both (.xmp and .dop)`, press `3` on a file. Expected: both
     its `.xmp` and `.ARW.dop` hold 3. Also, through the MCP companion if it
     is on, a `set_judgment` right after a switch writes the new format.
  6. Quit, start again with the same `--settings` and a fresh `--index`
     (`index-s4b.sqlite`), no `--import-settings`. Expected: the startup
     `settings:` line has `sidecarFormat=both` and the tab shows `Both`
     selected.
  7. Open `D:\Photos\tests\20261113-settings\jpegs` the same way (fresh
     `--index`) and switch the format. Expected: no error in the status line
     or the log, and the folder re-lists view-only.

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
  the automated criteria (`settingsmodal.rs`'s tests, clippy). Steps, on copies
  under `D:\Photos\tests\20261113-settings\` only, after `cargo build` in
  `crates/native`:
  1. `target\debug\riffle-native.exe D:\Photos\tests\20261113-settings\photos --index D:\Photos\tests\20261113-settings\index-s3a.sqlite --settings D:\Photos\tests\20261113-settings\step3-hand.sqlite --log D:\Photos\tests\20261113-settings\Handpic.log`
  2. Open `Filter ▾`, then press Ctrl+,. Expected: the menu closes and the
     Settings box appears centered over a dimmed window, the Sidecar tab
     selected (empty), Culling, Keyboard Shortcuts and Import below it.
  3. With it open, press the arrow keys, `q`, `1` and `x`, click a strip
     cell and scroll the wheel over the strip. Expected: the preview, the
     strip and the file's rating do not change and the app does not quit.
     With the Sidecar tab focused, ArrowDown / ArrowUp / Home / End move the
     selected tab; Tab / Shift+Tab move a visible focus border through the
     close button, the selected tab and (on Culling) the auto-advance
     checkbox, wrapping; Space or Enter on the focused checkbox flips it.
  4. Close it with Escape, open it again and close with the `×` button, then
     open and close with a press on the dimmed area outside the box. After
     each close, ArrowRight pages to the next file.
  5. On Culling, turn auto-advance off and close. Press `1`: the rating is
     set and the file stays. Quit, start again with the same `--settings`
     and a fresh `--index` (`index-s3b.sqlite`). Expected: the startup
     `settings:` line has `autoAdvance=false` and the checkbox is off.
