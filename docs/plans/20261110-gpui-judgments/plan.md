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

# The meta pane and the judgments with their sidecars in the GPUI app

## Purpose

This is milestone Step 6 of the GPUI port (`../20261109-gpui-port/plan.md`).
After Step 5 (`../_archived/20261110-gpui-strip-index/plan.md`) the native
app opens a folder, runs pass 1, shows the strip and resumes at the last
viewed file, but it cannot cull: nothing shows the shot's settings beyond a
three-value status line, no key rates, flags or labels a file, nothing is
written next to the photos, and a JPEG-only folder is not told apart from a
RAW one.

This plan gives the native app the meta pane (the EXIF and Maker note rows
the Tauri pane groups by provenance, with room left for the Analysis rows of
port Steps 7 and 9), the judgment keys of the backend's keymap (stars, the
tri-state flag, the color label, their clears), the judgments stored in the
index and written to the sidecars by the backend's coalescing writer (XMP,
`.dop` or both, the PhotoLab Uuid lookup for a fresh `.dop`), the sidecars
read back on every folder open, undo / redo of a judgment, auto-advance, and
the view-only mode of a JPEG folder. Selection (judging several files at
once), `Shift+X` (reject the rest of a burst, which needs the bursts of port
Step 7), the filter and the sort stay out (see Trade-offs).

The sidecars are the one thing this plan writes into the user's real
folders, shared with the Tauri app, PhotoLab and Lightroom; its steps are
ordered so that nothing is written before the reading side is checked, and
every hand check runs on copies under `D:\Photos\tests\<date>-<topic>\`.

The product is being renamed from Riffle to Handpic
(`../20261109-rename-to-handpic/plan.md`): its Step 2 changes the Tauri
identifier from `com.minodisk.riffle` to `com.minodisk.handpic`. On `main` at
the time of writing the identifier is still `com.minodisk.riffle`. This plan
names the native app's data folder after the new identifier and reads the
Tauri settings from either, so it works before and after the rename.

### What the investigation found (2026-11-10, `main` at `de249218`)

- The native app today: `main.rs` (1168 lines: the `Riffle` view, `load` on
  `cx.background_spawn` holding `ReadSlots::pause()` around `read_preview`,
  the preview and strip canvases, the `cx.defer`-after-paint stamp and the
  `retired` images), `pager.rs`, `strip.rs`, `thumbs.rs`, `scan.rs`
  (`list`, `Scan::prepare` = stat + `reconcile`, `Scan::start`), `focus.rs`,
  `viewed.rs`, `resync.rs`, `datadir.rs`, `logger.rs`, `args.rs`
  (`[<folder>] [--auto N] [--size WxH] [--max N] [--index <path>]`). `load`
  already computes `riffle_backend::exif::exif(&arw.shot)` for the status
  line's `shutter aperture iso`, so the meta pane's values cost no second
  read. The key handler matches `e.keystroke.key` against the literal
  `"right"` / `"left"` / `"home"` / `"end"` / `"q"`; the pager wraps around
  at both ends. Nothing in the native app reads or writes a `ratings` row or
  a sidecar; `read_listing(dir, None)` lists no sidecars. `datadir.rs` places
  the index under `<dirs::cache_dir()>/com.minodisk.riffle.native/`.
- The backend API this plan uses, all `pub` and Tauri-free:
  `sidecar::SidecarFormat` (`from_setting`, `kinds`, `sidecar_path`,
  `read_*`, `write_*`, `matches`), `sidecar::newest`,
  `sidecar::Writer::spawn(index, on_error) / set / set_now / flush(DRAIN_TIMEOUT)`
  (`DEBOUNCE` 300 ms, retries with backoff, each write to a `TEMP_SUFFIX`
  file renamed into place, `photolab::registered_uuids` for a fresh `.dop`
  through `riffle_core::dop`), `listing::read_listing(dir, Some(format))`
  (its `sidecars` are cleared for a JPEG folder) and `listing::stat_sidecars`,
  `Index::reconcile_sidecars` / `store_sidecar_ratings` / `dirty_rows` /
  `set_rating` / `mark_written` / `entries` (each `IndexedFile` carries
  `rating`, `flag`, `label`, `has_sidecar`), `index::flag_name` /
  `parse_flag`, `shortcuts::Keymap::defaults() / from_overrides(Option<&Value>)
  / bindings()` (the actions `previous`, `next`, `undo`, `redo`, `rate1`-`rate5`,
  `reject`, `rejectRest`, `pick`, `unflag`, `clear`, `red` ... `purple`,
  `clearlabel`, `clearall`, ... with key names like `arrowleft`, `shift+x`,
  `ctrl+alt+1`, `ctrl+z`; modifiers in `ctrl+alt+shift+meta+` order).
- Still in `crates/app`, Tauri-free but not in the backend:
  `commands::reconcile_sidecars_of` (the folder-open pass: pair each listed
  file with its newest sidecar's stat, `reconcile_sidecars`, parse the
  changed ones outside the lock, skip one over `MAX_SIDECAR_BYTES` = 4 MiB,
  `store_sidecar_ratings`, then `dirty_rows` minus the oversize ones; it
  returns `(dirty rows, Vec<SidecarError { path, message }>, changed)`),
  `commands::Metadata` + `read_metadata` (the pane's twenty formatted fields:
  `exif()`'s plus `shutter_type` from `electronic_front_curtain`,
  `exposure_bias` as `+0.3 EV`, `focus_distance` as `1.23 m`, `captured_at`),
  `commands::label_names_setting(Option<&Value>) -> LabelNames` /
  `label_names_value`, and `raw_only` (refuses a judgment on a non-RAW path).
- The Tauri judgment flow (`main.ts`): a key names an action through the
  keymap (`keys.ts`'s `keyName`: modifiers held in `ctrl+alt+shift+meta+`
  order, then the key; a plain key is `event.key` lower-cased); `judge`
  decides the new state once from the focused file and applies it to the
  targets (`selection.ts`'s `judgments`: a file already at the new state is
  skipped unless forced); the semantics are: `1`-`5` set the stars; `reject`
  / `pick` are sticky (not toggles) and keep the stars; `unflag` clears
  either; `0` clears the stars and leaves the flag; a color key toggles (same
  color again clears, another replaces); `clearlabel` clears the label; `c`
  clears all three in one undo entry and forces `label_known`. The maps and
  the strip update first, then `set_rating(path, rating 0..=5, flag, label,
  label_known)` goes out per file and a failure reverts that file and says so
  in the status line. `label_known` is true once `folder_entries` has told
  the frontend this path's label (else the writer keeps the sidecar's own
  label). Auto-advance (`advance.ts`): after `rate1`-`rate5`, `reject`,
  `pick` or `unflag`, one target, `autoAdvance` on (default true), the view
  moves to the next file (`move(1)` clamps at the last file, no wrap), also
  when the file already held the value. Undo (`undo.ts`'s `History`, bounded;
  `step` in `main.ts`): pop the last batch, push the current states onto the
  redo stack, restore, move to the file (a single-file batch), status
  `Undid <name>` / `Redid <name>`; the history is the open folder's and is
  cleared when another folder opens; redo is cleared by a new judgment.
- The Tauri strip cell draws the stars (`★` × n, top-right), a `●` flag dot
  (pick / reject colors, `--pick-color` / `--reject-color` in `style.css`),
  dims a rejected cell, and tints the file-name band along the bottom with
  the label's color (`--label-<color>`, `--label-other` for a name outside
  the seven). The meta pane shows the file name, the `EXIF` and `Maker note`
  groups (`meta.ts`'s `metaGroups`, a row left out when its value is `null`),
  the `Analysis` group (pass 2 and the ranking: out of scope here), and a
  notes area below (`JPEG folder: view only`, errors with a dismiss).
- View-only (`viewonly.ts`): a folder whose every listed path ends in
  `.jpg` / `.jpeg` (the backend lists JPEGs only when there is no RAW). No
  judgment key, no undo, no sidecar read or written; the note stays for as
  long as the folder is open. The Tauri strip then orders by capture time
  (the sort is port Step 7's; see Trade-offs).
- Settings: the Tauri app persists `sidecarFormat`, `labelNames`,
  `shortcuts`, `autoAdvance` (and more) in one flat JSON object,
  `settings.json` under `app_config_dir()`, i.e.
  `<dirs::config_dir()>/<identifier>/settings.json`
  (`%APPDATA%\com.minodisk.riffle\settings.json` on Windows today,
  `%APPDATA%\com.minodisk.handpic\settings.json` once the rename's Step 2
  ships and its migration has moved the folder). The user's file exists and
  holds `"sidecarFormat": "both"` (checked 2026-11-10), so the Tauri app
  writes an XMP and a `.dop` for every judgment on this machine.
- The folder watcher drops an event whose paths are all sidecars or
  `TEMP_SUFFIX` temporaries (`watch::triggers`), so the app's own writes do
  not resync the folder; an external sidecar edit lands on the next open.
- GPUI at the pinned rev: `Keystroke { modifiers: Modifiers { control, alt,
  shift, platform, function }, key: String, key_char }`; the key names seen
  on Windows are lowercase (`"left"`, `"right"`, `"home"`, `"end"`, `"q"`);
  `App::on_app_quit` exists (a hook returning a future, run before the app
  exits); IME is unverified but nothing here takes text input.

### Rules every step follows

- `crates/native` must not depend on `crates/app`. A Tauri-free piece the
  native app needs that still lives in `crates/app` moves into
  `crates/backend` behind no event name and no `AppHandle` (the Step 4
  pattern), and `crates/app` calls it from there; that is not a shim. Every
  backend change is checked with both `mise run ci` and
  `mise run native:check`.
- **Sidecar safety during development.** A debug or measurement run never
  opens a folder under `D:\Photos\` other than `D:\Photos\tests\`; every hand
  check runs on a copy under `D:\Photos\tests\<date>-<topic>\` with a fresh
  `--index` file and, from Step 3 on, an explicit `--settings <file>` so the
  sidecar format and label names in force are the test's, not the user's
  (`"both"`). Nothing in this plan edits a sidecar the user did not make in a
  test copy. The one real thing the writer touches outside the test folder
  is PhotoLab's database, read-only (`photolab::registered_uuids`).
- No Tauri smoke checks and no shims for Tauri behaviour (the user's
  decision for the port); root `mise run ci` must still pass.
- Build with `C:\Users\daisu\AppData\Local\bin\NASM` on `PATH`; `mise run
  native:check` before every PR, and `cargo build` before launching the exe
  (`native:check` does not produce it). A running `riffle-native.exe` locks
  the target binary: rename it aside, do not kill it.
- Index reads and writes, sidecar parsing and `reconcile_sidecars` run on
  `cx.background_spawn` or the scan thread, never on the main thread; a
  keypress never waits on disk (the Tauri rule: the `ratings` row, then the
  writer, the view updated before either).
- Callbacks from non-GPUI threads (the writer's `on_error`) only send into
  the view's one event channel.
- Pure models in their own modules with unit tests; the window wiring in
  `main.rs`. Manual checks by the user on the GPUI app are acceptable
  Done-when lines; their result goes in `learnings.md`.
- Docs move with the code: each step updates `docs/agents/native-app.md`
  with what it learned and `CLAUDE.md`'s `crates/native` sentence names the
  new modules.

## Steps

- [x] Step 1: Tick the port's Step 5, name the native data folder after the Handpic identifier, and move the folder-open sidecar pass, the pane's metadata formatting and the label-names setting into the backend
  - Done when:
    - `../20261109-gpui-port/plan.md`: Step 5's checkbox is ticked and
      Progress gains `(2026-11-10) Step 5 complete, through
      ../_archived/20261110-gpui-strip-index/plan.md` (as a link or in
      backticks; lychee passes).
    - `crates/native/src/datadir.rs` holds the identifiers in one place:
      `IDENTIFIER = "com.minodisk.handpic"` and
      `LEGACY_IDENTIFIERS = &["com.minodisk.riffle"]` (the rename plan's
      names), and the native index moves to
      `<dirs::cache_dir()>/com.minodisk.handpic.native/index.sqlite`. Nothing
      is migrated from `com.minodisk.riffle.native` (the native app has never
      shipped; a developer's old cache is simply re-built by pass 1). Its
      tests and `docs/agents/native-app.md` / `CLAUDE.md` say the new path.
    - `crates/backend/src/sidecar.rs` gains `pub fn reconcile_of(dir,
      listed: &[String], sidecars: &HashMap<String, SidecarStat>, index:
      &Arc<Mutex<Index>>, format, names: &LabelNames) -> Result<(Vec<DirtyRow>,
      Vec<SidecarError>, usize), String>`, `pub struct SidecarError { pub
      path: String, pub message: String }` (keeps `serde::Serialize`) and
      `pub const MAX_SIDECAR_BYTES`, moved from `crates/app/src/commands.rs`
      with their tests (the oversize test included); `scan_folder` calls
      `sidecar::reconcile_of`. Behaviour unchanged.
    - `crates/backend/src/exif.rs` gains `pub struct Metadata` (the twenty
      fields of `commands::Metadata`, `serde::Serialize` kept) and `pub fn
      metadata(name: String, shot: &Shot) -> Metadata` (the formatting now in
      `read_metadata`: `shutter_type`, `exposure_bias`, `focus_distance`,
      `captured_at`); `commands::read_metadata` becomes the read plus that
      call, with its tests moved or kept pointing at the backend.
    - `crates/backend/src/sidecar.rs` gains `pub fn label_names_setting(Option<&Value>) -> LabelNames`
      and `pub fn label_names_value(&LabelNames) -> Value`, moved from
      `commands.rs` next to `SidecarFormat::from_setting` (the same
      "stored value to type" pattern), with their tests.
    - `mise run ci` and `mise run native:check` pass; `crates/app` differs
      only by the moves and the call sites; `crates/native/Cargo.lock` is
      unchanged unless the backend gained a dependency (it should not:
      `serde_json` is already there).
  - Implementation approach:
    - This is the only step that edits `crates/app`; keep it to the moves.
      Name the moved functions in the PR so the Step 4 pattern is visible.
    - `raw_only` stays in `crates/app`: the native app's view-only rule is
      the frontend's (`viewonly.ts`), ported in Step 2.
- [ ] Step 2: The meta pane, with room for the Analysis rows, and the view-only JPEG folder
  - Done when:
    - `crates/native/src/meta.rs` is a pure model with unit tests: a port
      of `meta.ts`'s `metaGroups` over `riffle_backend::exif::Metadata`:
      `groups(meta: Option<&Metadata>, analysis: &[(&str, Option<String>)])
      -> Vec<Group { heading, rows }>` with the `EXIF` and `Maker note`
      headings and row order of `meta.ts`, a row dropped when its value is
      `None`, a group dropped when empty, and the `Analysis` group built
      from the `analysis` rows the caller passes (empty in this plan; port
      Steps 7 and 9 pass the eye / pose and ranking rows without touching
      the pane). A test pins that a `Metadata` with only EXIF fields yields
      one group and that an empty `analysis` yields no `Analysis` group.
    - `crates/native/src/viewonly.rs`: `is_view_only(paths) -> bool` (every
      listed path ends in `.jpg` / `.jpeg`, any case; false for an empty
      list), `VIEW_ONLY_NOTE = "JPEG folder: view only"`, tested with
      `viewonly.ts`'s cases.
    - `load` returns `Metadata` (built on the background thread from
      `arw.shot` through `exif::metadata`, replacing the `settings: String`
      the status line used; the status line keeps its three values from it)
      and the view holds the current file's `Metadata`; the window gains a
      right-hand pane (a fixed width in the Tauri pane's range, decided in
      the step; the preview canvas keeps `flex_1`) showing the file name,
      `n / total`, the groups as `div` rows (`label` left, `value` right,
      the group heading above), and a notes area at the bottom that shows
      `VIEW_ONLY_NOTE` in a JPEG folder. A file whose preview failed shows
      the file name and the error alone. A JPEG folder shows its `EXIF`
      rows (from `jpeg.rs`'s Exif) and no `Maker note` group, as the Tauri
      pane does.
    - The pane's cost is measured, recorded and not gated here: one
      `--auto 20` on `D:\Photos\tests\20261108-gpui-compare\ARW` (release,
      a fresh `--index`) before and after, `paint` and `keypressToPixels`
      p50 in `learnings.md`. If `paint`'s p50 grows by more than 1 ms the
      rows go on a `canvas` (shaped with `shape_line` as the strip's names
      are) instead of `div`s, in this step.
    - Manual check: open `D:\Photos\tests\20261108-gpui-compare\ARW`; the
      pane shows `EXIF` (Aperture, Shutter, ISO, Focal length, Exposure,
      Camera, Lens, Captured) and `Maker note` rows for a Sony file; open a
      folder of JPEGs copied under `D:\Photos\tests\` (for example a
      `-sequenced` output): `EXIF` rows only and the `JPEG folder: view
      only` note.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `meta.rs` and `viewonly.rs`; `native-app.md` records the pane's cost
      and whether `div` rows or a canvas were kept.
  - Implementation approach:
    - Assumes Step 1 merged. `Metadata` is computed inside the existing
      `load`, after `read_preview`, so paging reads nothing new; do not add
      a per-page metadata read as the Tauri `metadata` command does.
    - The view-only flag is set from the listing (`Listed.files`) on every
      open and re-listing, before the strip is built; Steps 3 and 4 read
      it to refuse judgments and to skip the sidecar pass. The capture-time
      order of a view-only strip is port Step 7's (sort); here the strip
      keeps its natural order.
- [ ] Step 3: Read the settings and the sidecars: the keymap bound to the window, the judgments of the open folder shown on the strip, nothing written yet
  - Done when:
    - `crates/native/src/settings.rs`: `Settings { format: SidecarFormat,
      names: LabelNames, keymap: Keymap, auto_advance: bool }` and
      `load(path: Option<&Path>) -> Settings`: reads the Tauri app's
      `settings.json` read-only and applies the backend's rules
      (`SidecarFormat::from_setting`, `sidecar::label_names_setting`,
      `Keymap::from_overrides`, `autoAdvance` missing or non-boolean meaning
      on). The default path is the first that exists of
      `dirs::config_dir()/<IDENTIFIER>/settings.json` and then each
      `dirs::config_dir()/<LEGACY_IDENTIFIERS[i]>/settings.json`
      (`datadir.rs`'s constants), so the same file is found before and after
      the Handpic rename; `--settings <path>` in `args.rs` overrides it,
      tested there. A missing or unreadable file is logged once (naming the
      paths tried) and gives the defaults (`Xmp`, English names, the default
      keymap, auto-advance on). Unit tests on temporary dirs: `"both"` is
      read, a malformed file gives the defaults, the override path wins, the
      new identifier's file wins over the legacy one, and the legacy file is
      used when only it exists. The native app never writes this file (the
      settings UI is port Step 9).
    - `crates/native/src/keys.rs`: `key_name(&Keystroke) -> Option<String>`,
      the port of `keys.ts`'s `keyName` onto GPUI: the modifiers held in
      `ctrl+alt+shift+meta+` order (`platform` is `meta`), then the key,
      with GPUI's `left` / `right` / `up` / `down` mapped to the keymap's
      `arrowleft` / `arrowright` / `arrowup` / `arrowdown` and `space` kept;
      a lone modifier is `None`. `Bindings::action_for(&str) -> Option<&str>`
      over `Keymap::bindings()`. Tested for `1`, `shift+x`, `ctrl+alt+1`,
      `ctrl+z`, `ctrl+shift+z`, `arrowleft`. The window's key handler
      resolves every keystroke through it: `previous` / `next` replace the
      literal `"left"` / `"right"` (`home` / `end` / `q` stay literal), and
      a judgment action is logged at `debug` (its effect is Step 4's).
      What GPUI reports on Windows for a shifted letter and for a digit with
      Ctrl+Alt is checked in the running app and recorded in `native-app.md`
      (a `ctrl+alt+<digit>` that arrives as AltGr on some layouts is noted).
    - The folder open reads the sidecars: `scan::list` lists with
      `read_listing(dir, Some(format))` (sidecars kept), `Scan::prepare`
      runs, after `reconcile`, `stat_sidecars` and `sidecar::reconcile_of`
      on the same background task and logs `scan sidecars: dir=… dirty=N
      changed=N in …ms` as the Tauri app does; the dirty rows are counted
      and logged here (there is no writer yet, and a fresh native index has
      none); the sidecar errors it returns are shown in the pane's notes
      area as `<name>: <message>`. A view-only folder skips the pass. After
      `prepare`, the open's background task reads `Index::entries(dir)`
      through the reader and the view keeps `judgments: HashMap<String,
      State>` (`State { rating: Option<u8>, flag: Flag, label:
      Option<String> }`, `crates/native/src/judgment.rs`) keyed by the
      listed path, plus the set of paths whose entry was seen (Step 4's
      `label_known`). A re-listing reads them again.
    - `strip.rs` gains `set_judgment(index, Option<State>)` and the strip
      canvas draws the stars (`★` × n at the box's top-right), the flag dot
      (pick and reject in the Tauri colors, the rejected cell dimmed) and
      the label as a tinted name band (the seven Lightroom colors from
      `style.css`'s tokens, a neutral gray for another name), pure data in
      the model, painting in `paint_strip`.
    - Manual check (read-only): copy a folder the Tauri app has rated,
      flagged and labeled (XMP; and a `.dop` from PhotoLab when one is at
      hand under `D:\photos\samples\`) to
      `D:\Photos\tests\20261110-gpui-judgments\read\`, run with a fresh
      `--index` and `--settings` pointing at a test file holding
      `{"sidecarFormat":"xmp"}` then `{"sidecarFormat":"both"}`: the strip
      shows the stars, dots and bands the Tauri strip shows on the same
      copy; stderr has the `scan sidecars` line with `changed=<n>` on the
      first open and `changed=0` on the second; no sidecar's mtime changed
      (checked with `ls -l --time-style=full-iso` before and after).
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `settings.rs`, `keys.rs` and `judgment.rs`; `native-app.md` records
      the settings decision and the key names met.
  - Implementation approach:
    - Assumes Steps 1-2 merged. `Keymap::bindings()` is a `Vec<Binding>`;
      build one `HashMap<key, action>` at startup, the keymap being
      immutable until Step 9.
    - `reconcile_of` takes the writer `Arc<Mutex<Index>>` and locks it
      twice (reconcile, then store); run it on the open's background task
      after `reconcile`, before the scan thread starts, exactly as
      `scan_folder` orders them.
    - Keep `judgment.rs` small in this step: `State` and its `From<&IndexedFile>`.
- [ ] Step 4: The judgment keys write the index and the sidecars through the backend's writer, with auto-advance and the drain on quit
  - Done when:
    - `judgment.rs` gains the pure part of `judge`: `Command` (`Rate(1..=5)`,
      `Reject`, `Pick`, `Unflag`, `ClearRating`, `Label(color)`,
      `ClearLabel`, `ClearAll`), `Command::from_action(&str)`, `apply(focused:
      &State, own: &State, cmd) -> State` with the Tauri semantics (sticky
      reject / pick keeping the stars, `0` keeping the flag, a color key
      toggling against the focused file's label, `ClearAll` clearing all
      three), `changed(own, after) -> bool`, `advances_after(cmd)`
      (`advance.ts`'s set), and the `label_known` rule (`true` when the
      path's entry was seen, or the command set the label itself, or the
      command is `ClearAll`). Unit tests mirror `selection.test.ts`'s
      judgment cases and `advance.test.ts`.
    - `main.rs`: `Writer::spawn(index, on_error)` at startup when the index
      opened, `on_error` logging `sidecar write failed: path=… …` and sending
      an `Event::SidecarError { path, message }` the view shows in the
      pane's notes area (one line per path, the newest message replacing
      the older). A judgment action on the current file (a view-only folder
      or an empty listing: nothing happens) applies `apply` to the view's
      `judgments` and the strip first, then on `cx.background_spawn`:
      `Index::set_rating(dir, path, rating, flag, label, label_known)` on
      the writer connection, then `Writer::set(path, rating, flag, label,
      label_known, format, names)` (the format and names from `Settings`);
      a failure of either reverts that file's state in the view and shows
      the error in the notes area, as the Tauri `send` does. `rating` 0 and
      unrated are the same state (`None` in the row, `0` in a sidecar only
      when one exists: the backend already does this).
    - Auto-advance: after `Rate`, `Reject`, `Pick` or `Unflag` with
      `auto_advance` on, the view moves to the next file through the pager
      without wrapping at the last file (a `Move::NextClamped` or an
      explicit bound check; the arrow keys keep wrapping as they do today),
      also when the file already held the value (which writes nothing).
    - The dirty rows `reconcile_of` returns at a folder open go to
      `Writer::set_now`, as `scan_folder` does, so a judgment the writer did
      not land before a crash is written on the next open.
    - Quit drains the writer: `q` and the window's close both run
      `writer.flush(DRAIN_TIMEOUT)` before the app exits (`App::on_app_quit`
      at the pinned rev, or the `quit` path plus `on_window_closed`; which
      works is recorded in `native-app.md`), so a judgment inside the 300 ms
      debounce is not lost.
    - Manual check (on copies only), under
      `D:\Photos\tests\20261110-gpui-judgments\write\` with a fresh
      `--index` and a test `--settings`:
      1. `{"sidecarFormat":"xmp"}`: press `3`, `p`, `Ctrl+Alt+1` on a file
         without a sidecar: within a second `<stem>.xmp` appears holding
         `xmp:Rating="3"`, `xmpDM:good="True"`, `photoshop:LabelColor="red"`
         and `xmp:Label="Red"`; `0`, `u`, `Ctrl+Alt+0` leave `Rating="0"`,
         no `good`, no label; mashing `1` `2` `3` within 300 ms writes once
         (one mtime change, `3`).
      2. `{"sidecarFormat":"both"}`: the same on another file gives both
         `<stem>.xmp` and `<name>.dop`; the `.dop` holds `Rating = 3`,
         `ShouldProcess = true`, `ColorLabel = "Red"` (or the names
         `crates/core/src/dop.rs` writes) and, when PhotoLab has the copy
         registered, its Uuids (else random ones; see
         `../../agents/photolab.md`).
      3. `{"sidecarFormat":"xmp","labelNames":{"red":"Rot"}}`: a red label
         writes `xmp:Label="Rot"`.
      4. Crash replay: press `4` and `taskkill /F` the exe within 300 ms;
         reopen the folder with the same `--index`: stderr shows `scan
         sidecars: … dirty=1`, and the sidecar gets `Rating="4"`.
      5. Quit drain: press `5` and `q` at once: the sidecar holds `5`.
      6. Auto-advance: with the default settings, `3` moves to the next
         file; on the last file it stays; `Ctrl+Alt+2` stays on the file.
      7. A JPEG folder: `3` does nothing and no file appears next to the
         JPEG.
      Each file's text is checked by eye (`cat`); the Tauri app may be
      opened on the same copy as a cross-check but is not required.
    - `mise run ci` and `mise run native:check` pass; `native-app.md`
      records the quit hook that worked and the judgment path's ordering
      (view, row, writer).
  - Implementation approach:
    - Assumes Step 3 merged. The writer is a `std::thread` with a sync
      `Sender`; `Writer::set` only sends, so it may run on the background
      task right after `set_rating` returns `Ok`, as the Tauri command
      orders them (row first, so a crash between the two leaves the row
      dirty).
    - Keep the in-flight count per path out of the view: the Tauri
      frontend does not await the write for anything visible either.
    - `rejectRest` (`shift+x`) stays a no-op logged at `debug` until the
      bursts exist (port Step 7).
- [ ] Step 5: Undo and redo of a judgment, and the same-session latency gate
  - Done when:
    - `crates/native/src/history.rs`: a port of `undo.ts`'s bounded
      `History<T>` (`push` dropping the oldest past the limit, `pop`,
      `peek`, `remove_where`, `clear`) and the `step` rule as a pure
      function over two histories and the current states: pop the last
      batch (a `Vec<Judged { path, state }>`, one entry here since there is
      no selection, kept as a batch for port Step 8), push the current
      states of its paths onto the other history, drop the paths no longer
      listed, give back the changes to apply and the file to move to (the
      single file of a one-file batch; a batch of more leaves the current
      file). The limit is the one `main.ts` constructs its `History` with.
      Unit tests mirror `undo.test.ts` plus: undo after a folder switch is
      empty, redo is cleared by a new judgment, a popped batch whose only
      path is gone is skipped.
    - `main.rs`: every judgment pushes its `before` state as one entry and
      clears the redo history; `undo` / `redo` (the keymap's `ctrl+z` /
      `ctrl+shift+z`) run `step`, apply the restored states through the
      same view-row-writer path as a key (a failure drops that path from
      the entry, as `forgetOnFail` does), move the pager to the restored
      file and show `Undid <name>` / `Redid <name>` in the notes area;
      opening another folder clears both histories; a view-only folder has
      no history.
    - **Gate**, `results.md` in this folder: a same-session A/B as Step 5
      of the strip plan, **A** = this plan's release build (Steps 1-4
      merged), **B** = the pre-plan pager built in release from `de249218`
      into a scratch worktree, both run directly on
      `D:\Photos\tests\20261108-gpui-compare\ARW` (`--size 1616x1080`) and
      `...\RAF` (`--size 4416x2944`), `--auto 60`, one throwaway warm run of
      A then A B A B A B per folder, fresh warm `--index` files per binary,
      A with a `--settings` test file (`"xmp"`; nothing is pressed but the
      arrows, so nothing is written), the idle gate and the conditions
      block of the earlier results. The median of the three runs' p50
      `keypressToPixels` of A minus B is at most +2.0 ms on ARW and +5.0 ms
      on RAF (the meta pane and the strip's marks are the only new
      per-page work); p95 and the per-stage medians are recorded, not
      gated. If missed, `results.md` names the stage and Step 2 (the pane)
      or Step 3 (the marks) is reopened with a fix PR before this plan
      closes. The measurement starts only after the user says the machine
      may be left idle, and not while another session is measuring.
    - Manual check (copies only, `D:\Photos\tests\20261110-gpui-judgments\undo\`):
      `3` on file 1 (auto-advance moves to file 2), `p` on file 2, `Ctrl+Z`
      twice: file 2's dot goes, the view is back on file 1 and its stars
      go, each sidecar rewritten accordingly (`Rating="0"`, no `good`);
      `Ctrl+Shift+Z` puts the stars back; a new key after an undo makes
      `Ctrl+Shift+Z` do nothing.
    - `docs/humans/performance.md` "Native preview (GPUI)" gains one line
      per folder for the pager with the pane and the judgments (and a
      pointer to this `results.md`), `performance.ja.md` the same in
      Japanese. `mise run ci` and `mise run native:check` pass; `CLAUDE.md`
      names `history.rs`; `native-app.md` records what the measurement
      taught.
  - Implementation approach:
    - Assumes Steps 1-4 merged. The measurement reuses the strip plan's
      ad-hoc script approach (scratch, not committed; the idle sampler's
      empty `LoadPercentage` retry). The user's only action is to leave the
      machine idle and close the Tauri app.
    - B at `de249218` prints the same `page` line and percentile rule as
      A, so no reconciliation of the timing arithmetic is needed. B keeps
      its old `com.minodisk.riffle.native` default, which the runs never
      use (both pass `--index`).

## Trade-offs and risks

- **Where the sidecar format, label names and keymap come from: the Tauri
  app's `settings.json`, read-only (taken, the user's decision on
  2026-11-10) vs built-in defaults until port Step 9.** Premises: the user's
  store holds `"sidecarFormat": "both"`, so the Tauri app writes an XMP and a
  `.dop` per judgment; the two apps share the user's folders;
  `SidecarFormat::from_setting`, `Keymap::from_overrides` and
  `label_names_setting` already parse the stored values; the native app has
  no settings UI until Step 9. Reading the store means a native judgment on
  a real folder writes the same files the Tauri app would, with the same
  label names and keys, so the two never disagree on the user's photos; its
  costs are a dependency on the Tauri store's location and shape and that a
  developer's run picks up the user's real format unless `--settings` is
  given, hence the rule that every hand check passes `--settings`. Defaults
  only would have left the `.dop` stale on this machine.
- **The Handpic rename.** The rename plan changes the Tauri identifier to
  `com.minodisk.handpic` and migrates the Tauri data folders. The native app
  keeps both identifiers in `datadir.rs` and reads the settings from the new
  folder first, then the old, so neither plan has to land first. Its own
  index folder takes the new name now (`com.minodisk.handpic.native`) with
  no migration, since it has never shipped. The binary and crate names
  (`riffle-native`, `riffle_backend`) are the rename plan's Step 3, not this
  plan's.
- **Moving `reconcile_sidecars_of`, `Metadata` and `label_names_setting`
  into the backend (taken) vs copying them into the native crate.** The
  copy keeps `crates/app` untouched but duplicates ~150 lines whose
  behaviour (the oversize rule, the parse-outside-the-lock order, the
  twenty fields' formatting) must stay identical in both apps until Step 10
  removes one; the move is the Step 4 pattern and costs one small
  `crates/app` PR. The user said Tauri compatibility need not be weighed,
  not that `crates/app` must not be edited; root CI still passes.
- **Selection, `Shift+X` and the capture-time order of a view-only folder
  are deferred.** Judging several selected files (`extendPrevious` /
  `extendNext` / `selectAll`, `targets`) is not named by the port's Step 6
  and the strip has no selection; it belongs with the right-click menu of
  port Step 8, which targets the selection. `rejectRest` needs the bursts
  of port Step 7 (`burst.ts`). The view-only strip's capture-time order is
  the sort of port Step 7. The undo entries are kept as batches so the
  selection fits later without changing the history.
- **Auto-advance must not wrap.** The native pager wraps at both ends (the
  measurement driver relies on it); the Tauri `move(1)` clamps, so a `3` on
  the last file stays there. Step 4 adds a clamped move for auto-advance
  only.
- **Sidecars are shared with the Tauri app and PhotoLab / Lightroom.** Both
  apps keep their own index of the same sidecar by stat; the sidecar is the
  truth, so a judgment made in one app lands in the other on its next open
  (`reconcile_sidecars` parses a changed stat), never live (the watcher
  drops all-sidecar events). Two apps open on the same folder at once can
  each write the same sidecar; the last write wins and the other app reads
  it back on reopen. This is the Tauri app's existing behaviour with an
  external editor, not new, but a dev run must still never point at a real
  folder (the rule above).
- **Parallel sessions.** Port Step 7's zoom / peaking and pass 2 are being
  planned in two other worktrees at the same time. All three edit
  `crates/native/src/main.rs`; keep this plan's changes there to wiring, with
  the logic in the new modules, and rebase on whichever lands first. Their
  measurements and this plan's Step 5 must not run at the same time.
- **The meta pane as `div` rows vs a canvas.** Twenty-odd text rows laid
  out per frame by GPUI's element tree are expected to cost well under a
  millisecond, and `div` rows are far simpler than hand-shaped text; Step 2
  measures and switches to a canvas only if `paint` grows over 1 ms.
- **Writer errors go to the pane's notes area, not an `ErrorList`.** The
  Tauri frontend keeps a dismissable list keyed by path; here one line per
  path, the newest replacing, is enough until the dialogs of port Step 8.
- **Key names on Windows.** `ctrl+alt+<digit>` is AltGr+digit on layouts
  that have AltGr; the keymap's default color keys are the Tauri app's, so
  the behaviour is the same, but Step 3 checks what GPUI reports and records
  it. A shifted letter may arrive as `"x"` with `shift` or as `"X"`; `keys.rs`
  lower-cases the key.
- **The gate's noise.** As before, `frameWait` moves 6-11 ms between runs;
  the same-session A/B absorbs it and n=60 cannot gate p95.
- **IME** is not exercised: no text input exists before Step 9.

## Progress

- (none yet)
