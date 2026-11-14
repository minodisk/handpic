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

## Step 5: the Keyboard Shortcuts tab, the keymap edited live

- The literal keys `key_down` takes before the keymap on `origin/main`
  (`0e5fe766`, after #1025 and #1020): `home`, `end`, `q`, `ctrl+,` /
  `meta+,`, and `escape` only while the right-click menu or a sort / filter
  menu is open. The refused list (`settingsmodal::APP_KEYS`) is `home`, `end`
  and `q`: `escape` cannot be captured (it cancels the capture), `ctrl+,` /
  `meta+,` the backend already refuses as a menu accelerator, and a binding
  to `escape` still fires while no menu is open. The folder tree's keys
  (#1025: the arrows, Home / End, Enter, Escape, type-ahead in
  `tree_key_down`) are not refused: they run only while the tree has the
  keyboard (`tree_keys`), when `treekeys::tree_gate` swallows every photo
  action anyway, as in the Tauri app. The MCP companion (#1020) adds no key.
- The view edits its own copy of the keymap (read on every open, like the
  format and auto-advance) and emits the whole result as `Change::Keymap`;
  the window replaces `settings.keymap` and `bindings` and writes
  `shortcuts` through the new `Writes::shortcuts` (overrides, or a delete
  when there are none). The `expect(dead_code)` on `Writes::delete` and its
  `Write` variant is gone. `main.rs` gained the `Change::Keymap` arm and the
  keymap passed to `open`, nothing else.
- Focus while capturing: GPUI sends a key to the root's dispatch path when
  the focused handle's node is not in the rendered frame, and the root's
  `key_down` returns while the modal is open, so a capture started by a `+`
  that is replaced by the prompt would hear nothing. `capture` focuses the
  box's own handle, and a `×` click does the same after its edit (its chip is
  gone). Read from `window.rs`'s dispatch at the pinned rev, not yet seen
  running.
- `mise run fmt` also reformatted `settingsview.rs` (the long closures inside
  the rows); `mise run native:check` passed on the first run (520 tests).

## Step 6: a text input, and the label names

- No `textinput.rs` existed on `origin/main` (`004dda85`). The port keeps the
  example's `EntityInputHandler`, the custom line `Element` and the mouse
  selection, and moves the text, selection and marked range into a plain
  `Text` so its offsets and boundaries are unit-tested without GPUI's
  `test-support` (not enabled in this crate). The example's first two tests
  port as they are; the third shapes a 10 000-character line, so it is
  ported onto the one-rectangle helper (`span`) instead.
- IME, from the user (another session, 2026-11-14): the example built
  standalone at `b5cd310b` on Windows composes Japanese, shows the candidate
  window and commits. But it binds its editing keys as `cmd-a` / `cmd-c` /
  `cmd-v` / `cmd-x` only (`input.rs:772-775`), so Ctrl+A and the rest do
  nothing on Windows. `textinput.rs` takes them in its own `on_key_down` on
  `ctrl+` (Windows, Linux) or `meta+` (macOS), registering nothing globally.
- The key path, read at the pinned rev: `gpui_windows`'s message loop runs
  `translate_accelerator` (the `WM_KEYDOWN` dispatch) before
  `TranslateMessage`, and skips the latter when a listener stopped the key.
  So a stopped key types nothing: the input stopping its editing keys is what
  keeps them from inserting, and the settings box, which stopped every key
  since Step 3, now lets a key it does not take go on while a name field is
  focused (`settingsmodal::stops`). The root's `key_down` returns without
  stopping while the modal is open. IME keys arrive as `VK_PROCESSKEY`, which
  raises no GPUI `KeyDownEvent`, so composition never meets the listeners.
- Escape in a field commits and closes the modal, the Tauri behaviour (its
  `change` fires as the closing dialog takes the focus). Enter commits and
  stays. The field's blur commits too, so Tab out of a field commits.
- A commit emits `Change::LabelNames` only when the normalized names differ
  from those in force (the Tauri `change` event fires only on a changed
  value; the native blur fires on every focus loss), so an unchanged field
  writes nothing.
- The main view's switch: `switch_format`'s background part became
  `reset_sidecars(job)` (the `format_switch` lock, the job, then clear the
  histories and re-list, or the failure on the status line), shared by
  `switch_format` and the new `switch_names`. The names are written and put
  in force on the main thread before the job, so two overlapping changes end
  stored in click order without a `format_latest`-style guard; the job
  drains the stores and calls `sidecar::switch_to` with a no-op persist, so
  the flush / reset order stays the backend's one copy. The failure log line
  is now `failed to switch the sidecars`.
- `SettingsView::open` takes `&Settings` instead of four values: a fifth
  (the names) would have put it over clippy's `too_many_arguments`, and the
  call in `main.rs` got shorter. `SettingsView::new` takes the window (the
  fields' `on_blur` needs it).
- The language chooser is a wrapping row of the 16 `i18n::presets()` names,
  English first and chosen by default (the Tauri select's first option),
  the chosen one lit; `Reset` commits that language's names. A popover would
  have needed the menus' deferred / anchored machinery for one control.
- A `select_all` method for the owner (asked for as optional by the folder
  rename plan) was dropped: unused here, clippy's `dead_code` refuses it.
  Ctrl+A in the field selects all; the rename step can add the method.

## Step 7: the Import tab, and the parent plan's note

- `crates/backend/src/import.rs` was already on `main` (#1024) with
  `DEFAULT_PATTERN`, `ImportSettings`, `validate_pattern`, `format_date`,
  `root_setting`, `pattern_setting` and `example`, plus the plan and run
  that the Import… session added, and `chrono` as a backend dependency.
  Nothing was moved; the only backend addition is `today_example` (the local
  date, so the native crate needs no `chrono`), with one test.
  `crates/app/src/import.rs` was left alone (its `import_pattern_example`
  still calls `example` with `chrono::Local` itself), since another session
  edits it.
- The example follows the field through `cx.observe` on the pattern field
  rather than a new `textinput::Event`: the input notifies on every edit (and
  on caret moves), and the observer only notifies the view when the example
  text changed, so `textinput.rs` is unchanged.
- An invalid committed pattern shows its reason in the example line, not the
  status line, as the Tauri `saveImportPattern` does; a live-typed invalid
  pattern already shows the same refusal, since both come from
  `format_date`. A commit of the stored pattern writes nothing
  (`pattern_change` gives `None`); the Tauri command rewrites it, which
  stores the same value.
- `Choose…` runs `prompt_for_paths` from the view, not the window: the
  `SettingsView` entity lives as long as the window, so the spawned task's
  `update` and its `Change::ImportRoot` reach the window's subscription even
  after the modal closed, which is what the Done-when asks for, with one
  `Change` arm in `main.rs` and no new method there. The OS dialog was not
  opened in any automated run.
- The specifier table's examples are not re-checked against the backend in
  the native tests (it would need a `NaiveDate`, hence `chrono`); the
  backend's `formats_every_accepted_specifier` already pins the same texts.
- `main.rs` gained the two `Change` arms only.

## Deferred issues (todo candidates)

- Pending manual check (Step 7, Windows, the user's): the Import tab. Basis:
  Step 7's Done-when (the root chosen in the OS dialog, the live example, the
  validation and the restart); the checkbox was ticked on the automated
  criteria (`settingsmodal.rs`'s and `settings.rs`'s tests, the backend's
  `today_example` test, clippy). Steps, on copies under
  `D:\Photos\tests\20261113-settings\` only, after `cargo build` in
  `crates/native`:
  1. `target\debug\riffle-native.exe D:\Photos\tests\20261113-settings\photos --index D:\Photos\tests\20261113-settings\index-s7a.sqlite --settings D:\Photos\tests\20261113-settings\step7.sqlite --log D:\Photos\tests\20261113-settings\Handpic.log`
     (a fresh `step7.sqlite`, nothing seeded).
  2. Ctrl+, → Import. Expected: the paragraph, `Destination root` with an
     empty box, `Choose…` and the note `Choose the folder imports are copied
     into.`, the pattern field holding `%Y/%F`, the example `2026\2026-11-14`
     (today's date, relative while no root is set), the four example
     buttons and the eleven-row specifier table.
  3. `Choose…`: the Windows folder dialog opens; pick
     `D:\Photos\tests\20261113-settings\import-root` (create it first).
     Expected: the root box shows it, the note is gone, and the example reads
     `D:\Photos\tests\20261113-settings\import-root\2026\2026-11-14`.
     Type into the field without committing, open `Choose…` again and pick
     the same folder: the typed text stays in the field.
  4. Replace the pattern with `%Y/%m` by typing: the example changes as each
     character is typed and ends `...\import-root\2026\11`; press Enter.
  5. Type `%H` and press Enter: the example line shows the time-specifier
     refusal in red. Type `%Y//%d` and press Enter: the empty-folder-name
     refusal. Escape closes the modal.
  6. Quit, start again with the same `--settings` and a fresh `--index`
     (`index-s7b.sqlite`). Expected: Ctrl+, → Import shows the chosen root
     and `%Y/%m` (the last valid pattern), not `%Y//%d`.
  7. Click `%F`: the field shows `%F` and the example `...\2026-11-14`;
     quit and restart as in 6: `%F` is kept.
  8. Open `Choose…` and close the modal (Escape is taken by the dialog, so
     press Cancel in the dialog, then reopen and repeat, this time picking a
     folder after the modal was closed with the `×` while the dialog is up,
     if the dialog lets the window be clicked). Expected: a root picked
     after the modal closed still shows on the next open and after a
     restart.

- Pending manual check (Step 6, Windows, the user's): the text input and the
  label names. Basis: Step 6's Done-when; the checkbox was ticked on the
  automated criteria (`textinput.rs`'s and `settingsmodal.rs`'s tests,
  clippy). Steps, on copies under `D:\Photos\tests\20261113-settings\` only,
  after `cargo build` in `crates/native`:
  1. Prepare `D:\Photos\tests\20261113-settings\labels\` with two or three
     ARW copies, one with a `.xmp` holding `xmp:Label="Rot"`.
  2. `target\debug\riffle-native.exe D:\Photos\tests\20261113-settings\labels --index D:\Photos\tests\20261113-settings\index-s6a.sqlite --settings D:\Photos\tests\20261113-settings\step6.sqlite --log D:\Photos\tests\20261113-settings\Handpic.log`
     (a fresh `step6.sqlite`, nothing seeded, so the format is XMP).
     Expected: the `Rot` file shows no red label.
  3. Ctrl+, → Sidecar. Expected: under the three radios the paragraph, five
     fields (Red ... Purple) holding the English names, the language row
     with `English` lit and `Reset`. Choose `PhotoLab (.dop)`: the block
     hides; choose `Lightroom (.xmp)` again: it shows.
  4. Click in the Red field, Ctrl+A, type `Rot`, press Enter. Expected: the
     log has `label names: ... -> ...` and a `resync: dir=…`; the `Rot` file
     now shows red. Close, label another file red (Ctrl+Alt+1 by default): its
     `.xmp` holds `xmp:Label="Rot"`.
  5. In a field, check: typing, Backspace / Delete, the arrows and
     Shift+arrows, Home / End, a mouse drag selection, Ctrl+C then Ctrl+V,
     Ctrl+X. A typed `q`, `1` or arrow key does nothing behind the modal.
     Tab moves to the next field and commits the one left. Escape commits a
     typed name and closes the modal.
  6. With the Japanese IME on, type a name in the Yellow field: the
     composition is underlined, the candidate window sits at the field, and
     Enter commits the composition (without committing the field); a second
     Enter commits the field.
  7. Choose `日本語` in the language row and press `Reset`. Expected: the
     fields show `レッド` ... `パープル` and the `Rot` file loses its red
     label after the re-listing.
  8. Quit, start again with the same `--settings` and a fresh `--index`
     (`index-s6b.sqlite`). Expected: Ctrl+, → Sidecar shows the last names in the
     fields.

- Pending manual check (Step 5, Windows, the user's): the keymap edited live.
  Basis: Step 5's Done-when; the checkbox was ticked on the automated
  criteria (`settingsmodal.rs`'s edit / capture / label tests,
  `settings.rs`'s `the_shortcuts_are_stored_as_overrides_and_deleted_without_any`,
  clippy). Steps, on copies under `D:\Photos\tests\20261113-settings\` only,
  after `cargo build` in `crates/native`:
  1. `target\debug\riffle-native.exe D:\Photos\tests\20261113-settings\photos --index D:\Photos\tests\20261113-settings\index-s5a.sqlite --settings D:\Photos\tests\20261113-settings\step5.sqlite --log D:\Photos\tests\20261113-settings\Handpic.log`
     (a fresh `step5.sqlite`, nothing seeded).
  2. Ctrl+, → Keyboard Shortcuts. Expected: about 40 rows in the Tauri
     order and labels (`Previous`, `Next`, ... `Clear`), the list scrolls,
     `Reset all` under it, `Show / hide both side panes` holding `tab`.
  3. On `Pick`, press `+`: `Press a key...` appears; press `k`. Expected:
     a `k` chip next to `p`. Close, press `k` on a file: it is picked.
  4. Open again, remove `k` with its `×`; close; `k` does nothing.
  5. `+` on `Pick`, press `x`. Expected: the status line says `"x" is bound
     to reject` and nothing changes. `+`, then `q`: `"q" is taken by this
     app`. `+`, then Escape: the capture ends and the modal stays open.
     `+`, then switch to the Culling tab and back: no capture is shown.
     `×` on `p` alone: `"p" is the only key for pick; use Reset`.
  6. Add `k` to Pick again, quit, start again with the same `--settings` and
     a fresh `--index` (`index-s5b.sqlite`). Expected: `k` picks and the row
     shows `p` and `k`. `Reset all`, quit, restart (`index-s5c.sqlite`):
     only `p`, and the store holds no `shortcuts` key.
  7. Note which names arrive in the capture for Shift+1 and Ctrl+Alt+1 (the
     open question of `native-app.md`'s "Every keystroke goes through the
     keymap": the code expects `!` and `ctrl+alt+1`; Ctrl+Alt+1 is refused as bound to red, the message naming it), and record them there.

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
