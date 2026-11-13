<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# The settings modal and the native settings store in the GPUI app

## Purpose

This is a slice of milestone Step 9 of the GPUI port
(`../20261109-gpui-port/plan.md`). Today the native app reads the Tauri
app's `settings.json` read-only at startup (`crates/native/src/settings.rs`)
and has no settings UI: the sidecar format, the `xmp:Label` names, the keymap
and auto-advance can only be changed in the Tauri app, and a change in the
native `Sort` / `Filter` menus lasts for the session only.

This plan gives the native app its own settings store (a dedicated SQLite
file, seeded once from the Tauri `settings.json`), persists the strip's
filter and sort order in it, and ports the settings modal: the sidecar
format, the label names, the shortcut editor, the Import tab and the
auto-advance toggle, each taking effect live where the Tauri app's does
(the `aiEnabled` toggle is not ported: the user decided on 2026-11-14 not
to port the AI features to the native app). Once it is done the two apps'
settings diverge and the native app no longer depends on the Tauri app for
any of them.

### Decisions (the user, 2026-11-13)

- The store is a dedicated SQLite file, one key / JSON-value table, WAL, one
  write per key. It is not the index: the index is a cache that
  `Index::open` deletes wholesale on a schema failure and lives under the
  cache dir.
- The store's default path is `<config dir>/com.minodisk.handpic/settings.sqlite`
  (the `IDENTIFIER` folder, next to the Tauri `settings.json`), the final
  place after port Step 10.
- Only when the store does not exist yet, the values are imported once from
  the Tauri `settings.json`, found as `settings.rs` finds it today. After
  that the two apps' settings diverge. The real Tauri `settings.json` is
  never written.
- `filter` and `sortOrder` are persisted and restored at the next start.
  Only the persistence wiring of `filter.rs` / `sort.rs` / `menu.rs` may be
  touched, not the model.
- `--settings <path>` names the SQLite file. A test or hand-check run never
  reads the real Tauri `settings.json`.
- The modal opens on `ctrl+,` (`meta+,` on macOS) only; no always-drawn
  `Settings` button. The native app has no app menu: GPUI's `set_menus`
  builds a menu bar on macOS only and merely stores the menus on Windows and
  Linux, so a macOS `Settings…` menu item belongs with port Step 10.
- Out of scope: pass 3, genres, the ranking and the `recommended` sort, MCP,
  the log file, the performance overlay, the move of `perf.rs` / `ai.rs`
  into the backend (another session), port Step 8 (folder tree, context
  menus, folder dialogs), and the filter / sort / menu model internals.

### What the investigation found (2026-11-13, `origin/main` at `3a0b20ce`)

- The Tauri modal (`crates/app/ui/index.html` `#settings-dialog`,
  `crates/app/ui/src/settings.ts`) has seven tabs: Sidecar (three format
  radios; while the format writes XMP, the five `xmp:Label` name fields for
  Red, Yellow, Green, Blue and Purple, a language select over
  `riffle_core::i18n::presets()` (16 languages) and `Reset`), Culling
  (`autoAdvance`, `aiEnabled` with its status line, the ranking's refit),
  Keyboard Shortcuts (one row per action: its label, a chip per key with a
  remove button, `+` to capture a key, `Reset`; `Reset all`), Cache (index
  size, Clear Cache behind the scan-idle gate), Import (the readonly root
  with `Choose…`, the pattern field, the example line, the example pattern
  buttons, the specifier table of `importsettings.ts`), MCP, and Debug
  (timing logs, debug builds only). A status line under the content shows
  errors. `modal.ts` holds the open state and the key rule (while a row
  captures, Escape cancels and any other named key is added; otherwise
  Escape closes and Tab / Shift+Tab cycle focus inside the modal);
  `tabs.ts` moves the vertical tablist on ArrowUp / ArrowDown / Home / End.
  Every keydown stops at the modal while it is open. It opens from the app
  menu's `Settings…` (`open-settings`), whose accelerator `ctrl+,` /
  `meta+,` the keymap refuses to bind (`OTHER_MENU` / `MACOS_MENU` in
  `shortcuts.rs`).
- What each change does in the Tauri app (`crates/app/src/commands.rs`):
  - `set_sidecar_format` runs `switch_format`: write the state, flush the
    writer (`sidecar::DRAIN_TIMEOUT`), persist `sidecarFormat`,
    `Index::reset_sidecars`, then emit `sidecar-format`, on which the window
    reopens the open folder. `AppSwitchLock` serializes switches.
  - `set_label_names` normalizes with `sidecar::label_names_setting`,
    persists `sidecar::label_names_value` under `labelNames`, and when the
    names changed flushes the writer, resets the sidecars and emits
    `sidecar-format`.
  - The shortcut commands call `Keymap::add` / `remove` / `reset` /
    `reset_all` and persist `Keymap::overrides()` under `shortcuts`,
    deleting the key when there are none; the frontend rebuilds its
    key-to-action map from the returned bindings.
  - `set_auto_advance` persists `autoAdvance`.
  - `crates/app/src/import.rs` holds `DEFAULT_PATTERN` (`%Y/%F`),
    `ImportSettings`, `validate_pattern`, `format_date`, `check_segment`,
    `root_setting`, `pattern_setting` and `example`, all Tauri-free and on
    `chrono`, next to the Tauri commands (`set_import_root`,
    `set_import_pattern`, which refuses an invalid pattern and keeps the
    stored one, `import_pattern_example`).
  - `crates/app/src/ai.rs` (`AppAi`, the download with `reqwest` and
    `sha2`, the `ai-state` event) is Tauri-bound. `crates/backend/src` on
    `origin/main` has no `ai.rs`.
- The native app today: `settings.rs` builds `Settings { format, names,
  keymap, auto_advance, sort, filter }` from the JSON object through
  `from_values(&Map<String, Value>)`; `main.rs` keeps it in `self.settings`,
  passes `format` / `names` per call to `Scan::prepare` and
  `judgment::store`, reads `auto_advance` in `judge`, and builds
  `keys::Bindings::new(&settings.keymap)` once. `menu_action` runs
  `menu::apply` and `refilter` and writes nothing. `sort.rs` has
  `SortKey::setting()`; `filter.rs` has `Stored::from_setting` and
  `FilterState::apply_stored` but no way back to a value. `key_down` takes
  `home`, `end`, `q` and (while a menu is open) `escape` literally before
  the keymap's actions. `re-list` (`relist`) stops the running scan and
  lists the open folder again with the format and names then in force. The
  quit handler (`App::on_app_quit`) stops the scan and drains the judgment
  stores and the sidecar writer.
- `rusqlite` 0.40 (bundled) is a dependency of `crates/backend` only; the
  native crate reaches SQLite through the backend. `chrono` is in both lock
  files through `riffle-core`.
- GPUI at the pinned rev (`b5cd310b`) has no text input element. It has the
  pieces: `EntityInputHandler`, `ElementInputHandler`,
  `Window::handle_input`, the clipboard (`cx.write_to_clipboard`,
  `cx.read_from_clipboard`), tab stops (`tab_index`, `tab_stop`), and
  `cx.prompt_for_paths`. Its `crates/gpui/examples/input.rs` (920 lines,
  Apache-2.0) is a single-line input over those, with selection, the
  clipboard, mouse selection and IME marked text; it uses
  `unicode-segmentation`, which is already in `crates/native/Cargo.lock`
  transitively, and registers its editing keys through `actions!` and
  `cx.bind_keys`. Zed's own editor and `ui` crates are GPL and are not
  used. IME is still unverified in the native app (`native-app.md`).
- `crates/app/src/migrate.rs` moves a legacy identifier's folders into the
  current one's file by file, a destination that exists winning, so a
  `settings.sqlite` already present in `<config dir>/com.minodisk.handpic/`
  does not block the move of the Tauri `settings.json`.
- Pending hand checks written before this plan pass a JSON file as
  `--settings` (root `todo.md`, the filter-sort plan's `learnings.md`).

### Rules every step follows

- `crates/native` must not depend on `crates/app`. A Tauri-free helper the
  native app needs moves into `crates/backend`; it is not copied. No Tauri
  smoke checks and no compatibility shims; root `mise run ci` and
  `mise run native:check` (with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH"`)
  pass.
- Three other sessions edit `crates/native/src/main.rs` and the
  `crates/native` paragraph of `CLAUDE.md`. Keep the changes there to the
  few lines each step names; the modal lives in its own modules. Re-check
  `origin/main` at the start of each step for a piece another session
  already moved. A conflict with `main` is resolved with a merge commit,
  never a rebase and force push.
- Pure models in their own modules with unit tests, ported from the TS with
  its tests where there are any (`modal.test.ts`, `tabs.test.ts`); the GPUI
  elements apart from them.
- No A/B latency measurement: `native-app.md` puts the settings UI off the
  page path. The modal lays out nothing while closed, the key handler gains
  one branch, and `Bindings::action_for` is unchanged (its map is replaced
  on a keymap edit). A step that finds it has to add an always-drawn part
  or work per keystroke says so before doing it.
- Every hand check runs on copies under `D:\Photos\tests\20261113-settings\`
  with a fresh `--index` and an explicit `--settings` test store there
  (plus `--import-settings <test json>` where a seed is needed); never the
  default index, the default store or the real Tauri `settings.json`, and
  never a folder under `D:\Photos\` outside `D:\Photos\tests\`. `cargo
  build` before launching the exe; rename a locked running exe aside, do
  not kill it.
- GUI hand checks are the user's: each step records them in `learnings.md`
  as `Pending manual check (Step N, Windows, the user's)` with the exact
  command and what to look for.
- Docs move with the code: each step updates `docs/agents/native-app.md`
  with what it learned and names its new modules in `CLAUDE.md`'s
  `crates/native` sentence, in as few words as the neighbours use.

## Steps

- [x] Step 1: The settings store: a SQLite file seeded once from the Tauri `settings.json`
  - Done when:
    - `crates/backend/src/store.rs` (`riffle_backend::store`) is a key /
      JSON-value store over one SQLite file: one table (`key TEXT PRIMARY
      KEY`, `value TEXT NOT NULL` holding JSON), WAL, a schema version in
      `user_version`. It opens or creates the file (and its parent folder),
      reads every value as a `serde_json::Map`, and sets or deletes one key
      per call. It never deletes or recreates the file: a file that is not a
      database, or one written by a newer schema version, is an error and is
      left byte for byte as it was.
    - The seed runs only for a store that is new (no schema yet, which
      covers a missing file and a zero-byte one left by a crash): the caller
      passes a closure that is called only then, and the schema and the
      seeded values are committed in one transaction, so a store is never
      found half seeded. Every key of the seed is stored verbatim.
    - Unit tests in the backend: set / delete / read round trip; the seed
      closure is called for a new store and not on a reopen; a zero-byte
      file counts as new; a JSON file opened as a store is an error and its
      bytes are unchanged; a newer `user_version` is refused.
    - `crates/native/src/datadir.rs` has `settings_path(flag, config)` next
      to `index_path`: `--settings` when given, else
      `<config dir>/<IDENTIFIER>/settings.sqlite`.
    - `crates/native/src/args.rs` has `--import-settings <path>` (the JSON to
      seed a new store from), in `USAGE` and the tests.
    - `crates/native/src/settings.rs` loads from the store: `Settings` is
      built by the existing `from_values` from the store's values. The seed
      source is `--import-settings` when given; else, only when `--settings`
      is absent, the first existing Tauri `settings.json` by today's
      `candidates` rule (the current identifier's folder, then the legacy
      ones); with `--settings` and no `--import-settings` nothing is
      imported and nothing outside the given paths is read. The JSON is
      opened only when the store is new, and only for reading. A store that
      cannot be opened is logged with its path and gives the defaults, with
      the store handle absent so later steps skip their writes. The log
      says which store was opened and, for a new one, how many keys were
      seeded from which file.
    - The tests of `settings.rs` are carried over onto the store (the stored
      values, the defaults for a missing seed, the non-boolean
      `autoAdvance`, the sort order mapping, the identifier precedence of
      the seed source) and gain: a second load does not re-read a changed
      JSON; `--settings` without `--import-settings` never looks under the
      config dir; the seed file's bytes are unchanged after a load. All of
      them run on temp dirs through injected paths.
    - `main.rs` changes only the `settings::load` call and its log line.
      The app behaves as before otherwise (nothing is written after the
      seed yet).
    - `docs/agents/native-app.md`'s "The settings are the Tauri app's,
      read-only" section is rewritten for the store (its path, the one-time
      seed, the never-delete rule, the rule that a debug or hand-check run
      passes `--settings <test store>` and, for a seed, `--import-settings
      <test json>`); `CLAUDE.md`'s sentence about `settings.rs` and
      `datadir.rs` says the same in brief and names `store.rs` in the
      backend's list. The still-pending native hand checks that pass a JSON
      as `--settings` (root `todo.md`, `../20261113-gpui-filter-sort/learnings.md`)
      are reworded to `--settings <fresh sqlite> --import-settings <that json>`.
    - Hand check (headless, on test paths only): a first run with
      `--settings <new sqlite> --import-settings <json holding
      "sidecarFormat":"both","sortOrder":"rating">` logs the seed and
      `sidecarFormat=both sortOrder=rating`; after editing the JSON to
      `"dop"`, a second run with the same arguments still logs `both`; the
      JSON's bytes are what the edit left.
    - `mise run ci` and `mise run native:check` pass.
  - Implementation approach:
    - The store goes in the backend, not the native crate: it is
      Tauri-free, `rusqlite` (bundled, `links = "sqlite3"`) stays a
      dependency of one crate, and root CI lints it. The typed `Settings`,
      the path rule and the seed-source rule stay in the native crate. The
      Tauri app does not use the store.
    - Follow `index.rs` for the connection setup (`create_dir_all`, WAL),
      but not its discard-on-failure branch.
    - Keep `from_values` as the one place the stored values are parsed, so
      the rules stay the backend's (`SidecarFormat::from_setting`,
      `sidecar::label_names_setting`, `Keymap::from_overrides`,
      `SortKey::from_setting`, `Stored::from_setting`).
    - A stored `"recommended"` sort order still reads as capture order and
      must not be rewritten by the load.
- [x] Step 2: Persist the strip's filter and sort order
  - Done when:
    - `settings.rs` has the write path every later step uses: a handle that
      takes `(key, value)` or a delete, applies them to the store off the
      main thread in the order they were given, logs a failure (the
      in-memory value stands, as in the Tauri app), and can be drained with
      a timeout. Unit-tested against a temp store: the writes land, a later
      write of a key wins, the drain waits for them, and a missing store
      makes every write a no-op.
    - `filter.rs` gains only serialization: the remembered sections of a
      `FilterState` as a `Stored`, and a `Stored` as the stored `filter`
      value, each section in the menu's item order so the value is
      deterministic; a test round-trips it through `Stored::from_setting`.
      `outputs` is never stored.
    - `main.rs`'s `menu_action` writes `sortOrder` (`SortKey::setting()`)
      when the action changed the sort key and `filter` when it changed a
      remembered section (`Reset` included); the quit handler drains the
      settings writes next to the judgment stores. `menu.rs`'s model is
      untouched (a doc comment that says "for the session only" is
      corrected).
    - A restart with the same `--settings` store starts with the filter and
      the sort order the last run ended with; a view-only folder still
      shows capture order and keeps the stored key.
    - `learnings.md` records the pending manual check: change the sort and
      a filter section in the menus, quit, start again with the same
      `--settings` and a fresh `--index`, and see both restored (the
      startup log line names them); `Reset`, quit, restart: no filter.
    - `native-app.md` (the sort and filter menus section no longer says
      "Nothing is written") and `CLAUDE.md` are updated; `mise run ci` and
      `mise run native:check` pass.
  - Implementation approach:
    - Assumes Step 1 merged.
    - The step decides between one writer thread fed by a channel and the
      one-in-flight coalescing of `viewed.rs`'s `Latest`; either must keep
      per-key order (two quick toggles must not land reversed) and never
      write on the main thread.
    - Write only on a change made in the menu, never at load, so a seeded
      `"recommended"` survives until the user picks an order.
- [x] Step 3: The settings modal's shell and the Culling tab
  - Done when:
    - `crates/native/src/settingsmodal.rs` is the pure model with unit
      tests: the open state, the selected tab, the row capturing a key, and
      the key rule of `modal.ts` (`add` / `cancel` / `close` / `focus` /
      pass through) and of `tabs.ts`'s vertical tablist, with their test
      cases ported. The tabs are Sidecar, Culling, Keyboard Shortcuts and
      Import, in the Tauri order.
    - `crates/native/src/settingsview.rs` draws the modal as its own GPUI
      entity with its own focus handle: a backdrop over the whole window
      and a centered box (the title, a close button, the tab list on the
      left, the selected tab's content, a status line for errors), painted
      after the rest of the window and occluding it. It emits a typed
      change for the main view to apply. Nothing of it is laid out while
      it is closed.
    - It opens on `ctrl+,` (`meta+,` on macOS), taken literally in
      `key_down` like `home` / `end` / `q`, and closes an open sort or
      filter menu first. It closes on Escape, the close button and a press
      on the backdrop, and focus returns to the window. While it is open no
      key reaches the culling keymap or the native literal keys (paging,
      judgments, `q`), and a press or a wheel over it does not reach the
      strip or the menus' buttons.
    - Tab / Shift+Tab cycle focus inside the modal and the arrows move the
      tab list, as far as GPUI's tab stops give it; what does not work is
      recorded in `learnings.md` and in "Trade-offs and risks".
    - The Culling tab has `Auto-advance after a star, reject or pick`: a
      click flips `self.settings.auto_advance` at once and writes
      `autoAdvance`; the next judgment key follows it.
    - The AI toggle (`AI analysis of what each photo shows`) was dropped by
      the user's 2026-11-14 decision not to port the AI features (`aiEnabled`
      with the model download, pass 3, genres, the ranking, `recommended`)
      to the native app. The Culling tab holds auto-advance only, and a
      seeded `aiEnabled` key is left in the store unread.
    - The Sidecar, Keyboard Shortcuts and Import tabs exist and are empty
      until their steps.
    - `main.rs` gains only: the entity field, its subscription, one child
      in `render`, the guard at the top of `key_down`, and the `ctrl+,`
      arm.
    - `learnings.md` records the pending manual check: open with Ctrl+,;
      the arrows, `q`, a rating key and a click on the strip do nothing
      behind it; Escape, the close button and the backdrop close it and
      paging works again; with auto-advance off a rating key stays on the
      file, and the setting survives a restart with the same `--settings`.
    - `native-app.md` gains a section on the modal (the entity, the key
      routing, what was verified in GPUI's source against what was seen
      running); `CLAUDE.md` names the two modules; `mise run ci` and
      `mise run native:check` pass.
  - Implementation approach:
    - Assumes Step 2 merged (the write path).
    - Follow the sort and filter menus' pattern (`deferred`, `occlude()`,
      the capture-phase `on_mouse_down_out` rule in `native-app.md`), with
      a full-window backdrop instead of an anchored popover.
    - Verify at the pinned rev how a key reaches the root's `on_key_down`
      when the focus is inside the modal entity (the bubble from the
      focused node), and stop it in the modal's handler; keep the root
      guard as well, since a click can leave the focus on the root.
    - The modal reads the values from `self.settings` when it opens; the
      main view stays the owner of the values in memory.
    - The colors and sizes are `main.rs`'s existing constants (`PANE_COLOR`,
      `BORDER_COLOR`, `INPUT_BORDER`, `BUTTON_FILL`, ...), reachable from a
      child module as they are. No tab icons: GPUI's `svg()` needs an asset
      source the app does not have.
- [ ] Step 4: The Sidecar tab: the sidecar format, switched live
  - Done when:
    - The Sidecar tab shows `Lightroom (.xmp)`, `PhotoLab (.dop)` and `Both
      (.xmp and .dop)` with the stored one selected.
    - Choosing another does what the Tauri `switch_format` does, in its
      order: the view's format changes first (a judgment made meanwhile is
      queued in the new format), then off the main thread the sidecar
      writer is flushed (`sidecar::DRAIN_TIMEOUT`), `sidecarFormat` is
      written and `Index::reset_sidecars` runs, then the open folder is
      re-listed so the strip and the pane show the new format's judgments.
      One switch runs at a time; choosing the current format does nothing.
      A failure goes to the modal's status line.
    - There is one copy of that order: the Tauri-free body is shared from
      the backend, or the native sequence is the only other caller's;
      whichever the step picks, the two apps do not carry diverging copies
      of the flush / persist / reset order.
    - Unit tests cover the switch body (a dirty row keeps its judgment and
      loses the old format's stat, a clean row is dropped, no change when
      the format is the same), moved or added next to it.
    - `learnings.md` records the pending manual check on a copied folder
      with both `.xmp` and `.dop` sidecars holding different ratings:
      under `xmp` the strip shows the XMP ratings; switching to `dop`
      re-lists and shows the `.dop` ones; a rating key then writes only the
      `.dop`; switching to `both` writes both; a restart with the same
      `--settings` starts in the last format; a JPEG-only folder switches
      without an error.
    - `native-app.md` records the switch; `mise run ci` and `mise run
      native:check` pass.
  - Implementation approach:
    - Assumes Step 3 merged.
    - `switch_format` and `choose_format` in `crates/app/src/commands.rs`
      take a `&Mutex<SidecarFormat>`, the writer, the index and a `persist`
      closure and have no `tauri` in them; the native view keeps its format
      as main-thread state. The step decides how the shared body's
      signature fits both.
    - The re-list is the existing `relist` (it stops the running scan and
      reconciles with the format in force). Check what the Tauri
      `openDirectory` on the same folder does with the undo histories and
      the `touched` / `seen` sets, and do the same: the judgments are read
      again from other sidecars, so entries recorded under the old format
      may no longer describe the files.
    - The reset must have finished before the re-listing's
      `sidecar::reconcile_of` runs.
- [ ] Step 5: The Keyboard Shortcuts tab: the keymap edited live
  - Done when:
    - The tab lists every action of the keymap in `Keymap::bindings()`
      order with the labels of `settings.ts`'s `shortcutLabels`, each key
      as a chip with a remove button (`space` shown as `Space`), a `+`
      that starts a capture (`Press a key...`), a per-row `Reset` and
      `Reset all`.
    - While a row captures, the next named key (`keys::key_name`, a lone
      modifier ignored) is added through `Keymap::add`, Escape cancels, and
      switching tabs or closing cancels. `Keymap::remove` / `reset` /
      `reset_all` back the buttons. A refusal (bound to another action, a
      menu accelerator, reserved by the system, the last key of an action)
      shows the backend's message in the status line and changes nothing.
    - A capture of one of the native app's own literal keys (`home`, `end`,
      `q`) is refused with a message, since those arms run before the
      keymap and the binding would never fire.
    - Every successful edit replaces `self.settings.keymap` and
      `self.bindings` (`Bindings::new`) at once and writes
      `Keymap::overrides()` under `shortcuts`, deleting the key when there
      are no overrides; the new key acts as soon as the modal closes and
      survives a restart.
    - The model's new parts (the capture flow, the reserved keys, the row
      text) are unit-tested in `settingsmodal.rs`; `keys.rs` changes only
      its comment that the bindings are built once.
    - `learnings.md` records the pending manual check: add `k` to Pick,
      close, `k` picks; remove it, `k` does nothing; adding `x` to Pick is
      refused as bound to reject; `q` is refused; `Reset all` restores the
      defaults; a restart keeps an override; and which name a shifted digit
      and Ctrl+Alt+digit arrive under in the capture (the open question of
      `native-app.md`'s "Every keystroke goes through the keymap").
    - `native-app.md` and `CLAUDE.md` are updated; `mise run ci` and
      `mise run native:check` pass.
  - Implementation approach:
    - Assumes Step 3 merged; independent of Step 4.
    - No A/B: the key path's lookup is the same `HashMap` read; only the
      map is replaced when the keymap changes.
    - The list is long (about 40 rows): it scrolls inside the tab
      (`overflow_y_scroll`), and rows are plain `div`s since they are laid
      out only while the tab is shown.
    - The actions the native app does not handle yet (bursts, the panes,
      compare, ...) are listed and editable like the rest; their keys only
      log until their steps land.
- [ ] Step 6: A text input, and the label names
  - Done when:
    - `crates/native/src/textinput.rs` is a single-line text input entity,
      ported from GPUI's `crates/gpui/examples/input.rs` at the pinned rev
      with a header comment naming the source and its Apache-2.0 license:
      typing, the caret, selection by keyboard and mouse, copy / cut /
      paste, Home / End, IME composition (marked text) through
      `EntityInputHandler`, a way to set its text, and a commit signal on
      Enter and on losing focus. Its offset and boundary helpers have unit
      tests (the example's three tests ported, plus grapheme boundaries on
      non-ASCII text).
    - While the format writes XMP (`xmp` or `both`), the Sidecar tab shows
      the Tauri paragraph and the five name fields (Red, Yellow, Green,
      Blue, Purple), a language chooser over `riffle_core::i18n::presets()`
      and `Reset`, which fills in the chosen language's names. Under `dop`
      the block is hidden.
    - A committed field or a `Reset` normalizes the five names with
      `sidecar::label_names_setting` (trimmed, an empty one falling back as
      the backend rules), writes `sidecar::label_names_value` under
      `labelNames`, shows the stored names back in the fields, and, when
      the names changed, does what Step 4's switch does after the state
      write: flush the writer, reset the sidecars, re-list the open folder.
      A judgment queued before the change keeps the names it was made
      under.
    - A key typed in a field reaches neither the culling keymap nor the
      modal's tab cycling beyond what a text field should give up (Tab
      moves on, Escape closes the modal after committing or discarding as
      the Tauri `change` event does; the step records which).
    - `learnings.md` records the pending manual check: set Red to `Rot`,
      label a file red, the `.xmp` holds `xmp:Label="Rot"`; a folder whose
      XMP says `Rot` shows red after the change and not before; `Reset`
      with Japanese chosen fills the Japanese set; typing a name with the
      Japanese IME composes and commits in the field; paste and select-all
      work; a restart keeps the names.
    - `native-app.md`'s "IME handling is unverified" section is updated
      with what the port relies on and what the user still has to see;
      `CLAUDE.md` names `textinput.rs`; `mise run ci` and `mise run
      native:check` pass.
  - Implementation approach:
    - Assumes Step 4 merged (the flush / reset / re-list sequence).
    - Handle the editing keys in the input's own `on_key_down` rather than
      through `actions!` and `cx.bind_keys`: nothing is then registered in
      GPUI's global keymap, so the pager's key path is untouched. The
      clipboard keys are `ctrl+` on Windows and Linux and `cmd+` on macOS.
    - `unicode-segmentation` becomes a direct dependency of
      `crates/native`; it is already in its `Cargo.lock`.
    - Verify at the pinned rev that a typed character still reaches the
      platform input handler after the key-down listeners ran (the modal
      and the root must not stop it before the insertion) and that a
      consumed editing key does not also insert its character.
    - GPUI has no select element: the language chooser is a small list in
      the tab or a popover in the menus' pattern; the step picks the
      smaller one.
    - The input is written to be reused by Step 7's pattern field; nothing
      beyond what those two uses need.
- [ ] Step 7: The Import tab, and the parent plan's note
  - Done when:
    - The Tauri-free import settings are in the backend
      (`crates/backend/src/import.rs`: `DEFAULT_PATTERN`, `ImportSettings`,
      `validate_pattern`, `format_date`, `root_setting`, `pattern_setting`,
      `example` and a way to get today's example without the caller
      needing `chrono`), with their tests moved; `crates/app/src/import.rs`
      uses them from there with no behavior change; `chrono` is a backend
      dependency. If another session has already moved them when the step
      starts, the step uses that module and moves nothing.
    - `settings.rs`'s `Settings` carries the import root and pattern, read
      with `root_setting` / `pattern_setting` (an invalid stored pattern
      reads as the default).
    - The Import tab shows the Tauri paragraph, the destination root
      (read-only text) with `Choose…`, the note of `importsettings.ts`'s
      `rootNote` while no root is set, the pattern field, the example line
      (today's date under the root, or the refusal in the error color),
      the example pattern buttons (`%Y/%F`, `%F`, `%Y/%m/%d`, `%Y/%m`) and
      the specifier table, their text ported from `importsettings.ts` into
      the model with a test.
    - `Choose…` opens the OS folder dialog (`cx.prompt_for_paths`,
      directories only) and writes the chosen path under `importRoot`,
      keeping a pattern still being typed. The example follows the field
      as it is typed. A committed pattern or an example button is
      validated: a valid one is written under `importPattern`, an invalid
      one shows the reason and leaves the stored one. Both survive a
      restart. Nothing consumes them yet: `Import…` itself is port Step
      8's.
    - `learnings.md` records the pending manual check: choose a root under
      `D:\Photos\tests\20261113-settings\`, the example shows it with
      today's date; `%Y/%m` changes the example; `%H` and `%Y//%d` show
      their refusals and are not stored (a restart shows the last valid
      one); the example buttons fill and save.
    - `../20261109-gpui-port/plan.md` gets a line under Step 9 and a
      Progress entry saying the settings modal slice was done through this
      plan and what of Step 9's settings is still open (the Cache tab; the
      first-launch format dialog; a macOS app menu with `Settings…`), and
      that the AI features are not ported (the user, 2026-11-14);
      Step 9 stays unticked.
    - `native-app.md` and `CLAUDE.md` (the backend's `import.rs`, the
      native modules' final wording) are updated; `mise run ci` and
      `mise run native:check` pass.
  - Implementation approach:
    - Assumes Step 6 merged (the text input).
    - Move only the settings and pattern pieces; `walk`, `plan` and the
      run stay in `crates/app` for Step 8's session, which may extend the
      same backend module.
    - The example is a local call, so it is recomputed on every edit
      without the Tauri frontend's 200 ms debounce and request counter.
    - `prompt_for_paths` resolves on a later turn; the write of the chosen
      root does not depend on the modal still being open, and the modal is
      updated through its entity only if it is.

## Trade-offs and risks

- **Where the store lives.** Taken: the generic store in
  `crates/backend/src/store.rs`, the typed settings in the native crate.
  It keeps `rusqlite` in one crate and under root CI's clippy; the cost is
  backend code the Tauri app never calls. The alternative, a store inside
  `crates/native`, needs a direct `rusqlite` pin held equal to the
  backend's (`libsqlite3-sys` links once).
- **The store's default path.** Taken (the user): `<config dir>/com.minodisk.handpic/settings.sqlite`,
  the final place after port Step 10; it sits in the Tauri app's config
  folder next to its `settings.json`, which `migrate.rs`'s per-file merge
  tolerates. A Tauri uninstall that removes app data removes it too. Not
  taken: `<config dir>/com.minodisk.handpic.native/settings.sqlite`, which
  would need one more move at the switch-over.
- **How the modal opens.** Taken (the user): the literal `ctrl+,` /
  `meta+,` only. The native app has no app menu, and that key cannot be
  bound in the keymap. A `Settings` button in the status row would be
  discoverable, but it is a new always-drawn part, which `native-app.md`'s
  rule puts on the A/B side. A macOS menu bar item is cheap through
  `set_menus` and is left for port Step 10.
- **No `aiEnabled` toggle.** The user decided on 2026-11-14 not to port the
  AI features to the native app at all (`aiEnabled` with the model download,
  pass 3, genres, the ranking, `recommended`). Step 3 first wired the toggle
  to `riffle_backend::ai`'s download (#1006) and then removed it, with the
  `--data-dir` flag it needed. A seeded `aiEnabled` key stays in the store
  and is never read, so the Tauri value is kept should the decision change.
- **Left out of this slice.** The Cache tab (Clear Cache needs the
  scan-idle gate that port Step 8 brings), the first-launch format dialog,
  the MCP and Debug tabs, the ranking's refit, the tab icons. Without the
  first-launch dialog a user with no Tauri settings to seed from gets XMP
  without being asked, and no language question for the label names.
- **A store that cannot be opened gives the defaults.** As in the Tauri
  app, that means XMP even when the user's format was `both`, so judgments
  would go to the wrong sidecars for that run; the log names the failure
  and, once the modal exists, its status line can. Falling back to a
  read-only read of the Tauri JSON was not taken: it would re-couple the
  two apps after they are meant to diverge.
- **The import is all keys, once.** Keys the native app does not read yet
  (`lastFolder`, `panels`, `mcpEnabled`) are seeded too, because the
  import never runs again for a later step. Values changed in the Tauri
  app afterwards are not seen; that is the decided divergence.
- **`--settings` changes meaning.** It named a JSON and now names the
  SQLite store. A JSON passed by habit fails to open, is left untouched,
  and the run uses the defaults; Step 1 rewrites the pending hand-check
  commands that do so, in files other sessions may also be editing.
- **The native literal keys.** `home`, `end` and `q` are refused in the
  shortcut capture. The alternative, letting a bound action win over the
  literal arms, changes the order of the key handler's `match` and was
  left alone.
- **Keyboard use of the modal.** Escape closes; Tab / Shift+Tab call
  `window.focus_next` / `focus_prev`, which wrap over the window's tab stops,
  all of them the modal's; ArrowUp / ArrowDown / Home / End move the focused
  selected tab (a roving tab stop). Enter and Space activate a focused
  control for free (GPUI turns them into a keyboard `ClickEvent` on a
  focusable element with `on_click`). All of it is read from GPUI's source at
  the pinned rev; seeing it run is the user's hand check. The mouse path is
  the one the done criteria require.
- **The text input is a port of an example.** It is single-line and has
  no undo, no word movement and no double-click selection; IME behaviour
  is read from the source and seen only in the user's hand check. Copying
  it brings Apache-2.0 code into the crate with a source note in the file.
- **A format or label-name switch re-lists mid-scan.** It cancels and
  restarts the running scan, as the Tauri app's reopen does; a switch is
  rare enough that the deferral `resync.rs` applies to watcher events is
  not applied here.
- **Concurrent sessions.** Each step touches `main.rs` in a few named
  places and one sentence of `CLAUDE.md`; conflicts there are expected and
  resolved with a merge commit from `main` (no force push). Step 7's
  backend `import.rs` may collide with port Step 8's Import… work.
- **Two native instances** on one store are safe under WAL; the last
  write of a key wins and a running instance does not see the other's
  changes.

## Progress

- (2026-11-13) Step 1 complete
- (2026-11-13) Step 2 complete
