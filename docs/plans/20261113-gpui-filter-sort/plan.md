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

# The strip's filter menu and sort order in the GPUI app

## Purpose

This is a slice of milestone Step 7 of the GPUI port
(`../20261109-gpui-port/plan.md`). Pass 2 with the `f` overlays and
`eyes_of` (`../_archived/20261110-gpui-focus-marks/plan.md`) and the 1:1
view with focus peaking (`../_archived/20261110-gpui-zoom-peaking/plan.md`)
are merged. What is left of Step 7 is the strip's filter menu and its sort
order: today the native strip shows every listed file in `natord` file-name
order, so a culling pass cannot narrow the strip to the picks, the rejects,
the unrated files, a color, the frames whose AF eyes are in focus or the
files PhotoLab has not exported, and cannot order it by capture time or
rating. A JPEG-only folder also still shows in name order where the Tauri
app shows it in capture order.

This plan ports `filter.ts` and `sort.ts` as pure models with their tests,
splits the view's one list into the listing (`all`) and the files the filter
lets through in sort order (`files`), wires every place that moves the
current file or rebuilds the list through one `refilter`, as the Tauri
`main.ts` does, and gives the native window the two menus as GPUI popovers.
The `Genre` section and the `recommended` sort (pass 3, the ranking) are
port Step 9's; this plan leaves their seam and ports nothing of them. The
per-folder EXIF sections are left to a follow-up slice (decided 2026-11-13),
so port Step 7 stays open after this plan with only those sections left.

### Decisions (the user, 2026-11-13)

- The filter and the sort key start from the Tauri `settings.json`, read
  read-only at startup (`filter`, `sortOrder`); a change in the native menus
  lives for the session only. Nothing is written to `settings.json`.
- `File name` order keeps `natord` (the native order since port Step 2), not
  the Tauri plain string compare.
- The per-folder EXIF sections are not in this plan; a follow-up slice ports
  them and closes port Step 7.
- The `Face focus` `?` help (`AF_EYE_HELP`) is omitted from the native menu.

### What the investigation found (2026-11-13, `main` at `788a8638`)

- The Tauri filter (`filter.ts`): the checked items of one section are
  OR-ed, the sections AND-ed, an empty section lets everything through.
  Sections: flags (`picked` / `untagged` / `rejected`), stars (`0`..`5`,
  `0` unrated, a reject keeping its stars), labels (the seven colors
  lower-cased plus `none`; a foreign label matches no item), orientations
  (`portrait` for tag 6 / 8, else `landscape`; unknown fails a non-empty
  section), candidates (`good` / `neutral` / `low` from `afEyeState`, a file
  the pass has not reached being `neutral`), eyes (`open` / `closed` /
  `unknown`, a file without one being `unknown`), genres (Step 9), outputs
  (`yes` / `no`; a file whose export state is unknown passes neither), and
  the per-folder EXIF groups (`exifKey`). `anchorAfterFilter(allFiles,
  pass, anchor, previous?)` picks the file to land on after a refilter:
  the anchor if it passes, else the next passing after it, else the last
  before; with `previous` (a re-listing) an anchor gone from the listing
  gives way to its surviving passing neighbour in `previous`.
  `filterListChanged(files, next, force)` is the early return.
  `outputOf(found, listing, path)` is the export state. `filter.test.ts`
  has 70 cases; `sort.test.ts` 9.
- The Tauri sort (`sort.ts`): `name` compares base names as plain strings
  (not natord), `capture` by `captureTime` then `subsec` right-padded,
  files without a time last, then name; `rating` by `ratingRank` (5 stars
  first, unrated as rank 6, a reject as rank 7 whatever its stars: the
  caller passes `-1` for a reject), then name; `recommended` is Step 9's.
  `viewonly.ts`'s `sortFor(viewOnly, key)` forces `capture` in a view-only
  folder, and the sort button is disabled there.
- Persistence: the Tauri app stores `filter` (`flags`, `stars`, `labels`,
  `orientations`, `candidates`, `eyes`; `commands::filter_setting` drops
  unknown members and keeps stored order) and `sortOrder` (`name` /
  `capture` / `rating` / `recommended`, anything else `name`) in the same
  `settings.json` the native `settings.rs` reads read-only. `genres`,
  `outputs` and the EXIF sections are per session. There is no keymap action
  for either menu: both open from buttons in the strip bar (`#sort-toggle`,
  `#filter-toggle`, right end), `Escape` or a click elsewhere closes them,
  the filter menu stays open while items toggle, the sort menu closes on a
  pick. `Reset` clears every section. The filter button is lit while any
  section is checked.
- The Tauri flow (`main.ts`): `refilter(anchor, keepScroll, force,
  previous)` rebuilds `files` from `allFiles` (`ordered()` then `passes`),
  returns early when the list is unchanged, rebuilds the strip, and lands on
  `anchorAfterFilter`'s file (`show()` when it differs). It runs after every
  judgment (`commit`), on undo / redo (`step`: a one-file batch lands on the
  file if it still passes, else stays and notes `(hidden by the filter)`;
  the batch is filtered on `allFiles.includes`, not `files`), on the
  `folder_entries` refresh after pass 1, on `faces-progress` while a
  `Face focus` or `Eyes` item is checked (keeping the scroll offset), on
  resync (with `previous`) and on every menu change. Auto-advance
  (`advance.ts`'s `shouldAdvance`) requires `stillCurrent: files[index] ===
  current`: a judgment that filtered the file out already moved the cursor,
  so no second move. The strip pane's counter and the pane's `n / total` are
  over the filtered `files`; an empty filtered list shows `FILTERED_TEXT`
  ("No files match the current filter.") in place of the preview. The
  `Output` section is read with `outputs_of` at open and on resync, next to
  the listing, and hidden (its checks cleared) when the folder has no export
  folder or it could not be read.
- The native app today (`crates/native/src/main.rs`): `files: Vec<PathBuf>`
  is the listing in `scan::natural_order` narrowed by `--size` / `--max`
  (`shown`); the pager, the strip (`Strip::new(&paths)`, cells by index),
  `thumbs`, `set_judgments`, `set_state`, `scan_focus_paths`,
  `current_path` and the pane's `n / total` all index into it.
  `carry(&paths)` already rebuilds the pager, the strip (`carried_offset`,
  `carried_indices`, `keep`) and the thumbnails onto a new list with the
  current file carried by `resync::carried_current`, which is
  `anchorAfterFilter` with `previous` and `pass = listed`. `judgments_of`
  reads `Index::entries(dir)` (each `IndexedFile` carries `orientation`,
  `capture_time`, `subsec`, `focus: Option<Focus>`, `rating`, `flag`,
  `label`) at the open and at pass 1's `Done` and keeps only the `State`.
  `Event::FacesProgress { ready: Vec<FaceReady> }` carries each written
  file's `eye_focus`, `candidate` and `StoredEyes`, enough to rebuild its
  `Focus` with the entry's `point` / `manual_focus`, as the Tauri
  `applyFaceReady` does. `mark.rs` has `af_eye_state(Option<&Focus>) ->
  MarkState { Good, Neutral, Low }` and `StoredEyes.eyes: Option<EyeState>`
  is the stored eye state. `history::step`'s `listed` closure is over
  `self.files`. `judge` runs `go(Move::NextClamped)` after an advancing
  command with no "still current" check. The key handler takes `home`,
  `end`, `q` literally, then the keymap's actions (`zoom`, `peaking` among
  them). The status row is one `div().px_2()` below the strip.
- The backend: `riffle_backend::output::outputs_in(dir: &Path, files:
  &[String]) -> Result<Option<Vec<String>>, String>` is Tauri-free (the
  Tauri `outputs_of` wraps it in `spawn_blocking` after `canonicalize`).
  `shortcuts.rs` binds no filter or sort action.
- GPUI at the pinned rev has `anchored()` (`position`, `anchor`,
  `snap_to_window`) and `deferred()` for a popover drawn over the rest of
  the tree, and `Interactivity::on_mouse_down_out` / `on_click` / `on_hover`
  on a `div`.

### Rules every step follows

- `crates/native` must not depend on `crates/app`. The Tauri-free pieces
  this plan needs are already in the backend (`output::outputs_in`,
  `Index::entries`); nothing moves. No Tauri smoke checks, no Tauri
  compatibility shims; root `mise run ci` and `mise run native:check` pass.
- Pure models in their own modules with unit tests, ported 1:1 from the TS
  with its tests (`filter.test.ts`, `sort.test.ts`, `advance.test.ts`'s
  `stillCurrent` cases); the window wiring in `main.rs`.
- Every hand check runs on copies under `D:\Photos\tests\20261113-filter-sort\`
  with a fresh `--index` and an explicit `--settings` test file; never open a
  folder under `D:\Photos\` outside `D:\Photos\tests\`. Build with
  `C:\Users\daisu\AppData\Local\bin\NASM` on `PATH`; `cargo build` before
  launching the exe; rename a locked running exe aside, do not kill it. Index
  keys and listed paths carry the canonical `\\?\` prefix: key the facts map,
  the filter and the sort on the listed strings, never on a rebuilt path.
- Nothing new runs on the page path: the filter and the sort are applied
  when the list changes (a judgment, a menu change, a pass's rows, a
  re-listing), never per page; the menus are laid out only while open.
- Nothing is written to `settings.json`.
- Docs move with the code: each step updates `docs/agents/native-app.md`
  with what it learned and `CLAUDE.md`'s `crates/native` sentence names the
  new modules.

## Steps

- [x] Step 1: The sort order: `sort.rs`, the view's `all` / `files` split and one `refilter`, capture order in a view-only folder
  - Done when:
    - `crates/native/src/sort.rs` is a pure model with unit tests: `SortKey
      { Name, Capture, Rating }` (with `Recommended` left out; `SortKey::
      from_setting(Option<&str>) -> SortKey`, the `parse_sort_order` rule,
      maps `"recommended"` to `Capture` for now and says so in a comment: the
      Step 9 seam), `SortFacts { capture_time: Option<String>, subsec:
      Option<String>, rating: Option<i8> }` and `order_files(key, paths:
      &[String], lookup: impl Fn(&str) -> SortFacts) -> Vec<String>` with
      `sort.ts`'s rules (capture then padded subsec, no time last; the
      rating rank with a reject as `-1` sorting after the unrated) and the
      name tiebreak and `Name` order by `natord` (the user's decision: the
      native order, not the Tauri plain compare). The 9 cases of
      `sort.test.ts` are ported; a case that depends on the plain compare is
      adjusted to natord and named as such. `viewonly.rs` gains
      `sort_for(view_only, key) -> SortKey` (`Capture` when view-only) with
      `viewonly.ts`'s test.
    - `main.rs` keeps the listing apart from what is shown: `all: Vec<String>`
      (the `shown` list: natural order narrowed by `--size` / `--max`, the
      Tauri `allFiles`) and `files` (the ordered, later filtered, list the
      pager, strip, thumbnails, `set_judgments`, `set_state`,
      `scan_focus_paths`, `current_path`, the status line and the pane's
      `n / total` keep indexing). A `facts: HashMap<String, Facts>` keyed by
      listed path holds, per entry the index returned, `orientation`,
      `capture_time`, `subsec` and `focus: Option<Focus>`; `judgments_of`
      becomes a read that returns the judgments and the facts from the one
      `Index::entries` call (at the open and at pass 1's `Done`, as today).
    - One `refilter(anchor: Option<String>, keep_scroll: bool, previous:
      Option<&[String]>)` in `main.rs` builds `next` from `all` through
      `sort::order_files` (the sort in force from `sort_for`), returns
      early when `next == files` (and `previous` is `None`), else rebuilds
      the pager, strip and thumbnails through the existing `carry`
      generalized to take the target index, lands on the anchor (this step:
      the anchor itself, else `resync::carried_current`'s rule; Step 2
      replaces this with `anchor_after_filter`) and requests the preview when
      the current file changed. The resync path (`opened` with `carry`) and
      pass 1's `Done` go through it; the open builds `files` through the
      same ordering before resuming at `last_viewed`.
    - `settings.rs` gains `sort: SortKey` read from `sortOrder` with the
      Tauri rule, tested next to `autoAdvance`; the view starts with it. A
      view-only folder is in capture order whatever the key; the key the user
      has is kept, as `sortFor` does. No UI yet: the key is only the settings
      file's.
    - Manual check (copies under `D:\Photos\tests\20261113-filter-sort\sort\`,
      a folder of RAWs with mixed capture times and names, and a JPEG
      `-sequenced` copy): with `{"sortOrder":"capture"}` the strip is in
      capture order and `--auto 5` pages in that order; with `"rating"` the
      files rated in a copied XMP sidecar come first, a rejected one last;
      the JPEG folder is in capture order under `"name"`; pressing `3` under
      `"rating"` reorders the strip at once and the current file stays
      current (auto-advance then moves to its new neighbour).
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `sort.rs`; `native-app.md` records the `all` / `files` split and the
      one-`refilter` rule.
  - Implementation approach:
    - `files` may stay `Vec<PathBuf>` to keep the diff to the indexing
      sites small; `all` and the facts are `String`s, the index's keys.
    - `carry` already does the rebuild (`Pager::new`, `Strip::new`,
      `carried_offset`, `carried_indices`, `thumbs.carry`, `keep`); change
      only how the target index is chosen and whether the scroll is kept
      (`keep_scroll` is the resync's `true`; a sort change scrolls the
      current file into view).
    - Measure nothing new here beyond `native:check`: the sort runs on a
      list change, not per page. If `order_files` over 1 000 paths is over
      1 ms in a unit-test timing, say so in `learnings.md`; otherwise leave
      it.
- [x] Step 2: The filter model and the judgment sections, wired through every move of the current file
  - Done when:
    - `crates/native/src/filter.rs` is a pure model with unit tests, a 1:1
      port of `filter.ts` less the genres and the EXIF sections (the
      `genres` set is left out with a comment naming Step 9; the EXIF
      sections with a comment naming the follow-up slice): `FilterFlag
      { Picked, Untagged, Rejected }`, `Orientation { Portrait, Landscape }`
      with `orientation_of(tag)`, `AfEye = mark::MarkState`, `EyeState` (the
      backend's), `OutputState { Yes, No }`, `FilterState { flags, stars:
      HashSet<u8>, labels: HashSet<String>, orientations, candidates, eyes,
      outputs }` with `is_active()`, `passes(&FilterState, &State, facts:
      Option<&Facts>, af_eye: MarkState, eyes: Option<EyeState>, output:
      Option<OutputState>) -> bool`, `output_of(found: Option<&HashSet<String>>,
      listing: &HashSet<String>, path) -> Option<OutputState>`,
      `anchor_after_filter(all, pass, anchor, previous) -> Option<String>`
      and `filter_list_changed(files, next, force)`. The 70 cases of
      `filter.test.ts` are ported but the genre and EXIF ones, which are
      listed in the test module as left out.
    - The stored filter: `filter::Stored` with `from_setting(Option<&Value>)
      -> Stored` (the `filter_setting` rule: the six remembered sections,
      unknown members dropped, deduplicated) and `FilterState::apply_stored`,
      tested as `commands.rs`'s three `filter_setting` tests; `settings.rs`
      reads `filter` at startup and the view starts with it.
    - `refilter` applies `passes` after the sort, with the facts and the
      judgments of the view, and lands on `anchor_after_filter`'s file
      (`previous` on a re-listing); an empty `next` clears the image, the
      `shown` pane and the marks, and the status and the pane say
      `FILTERED_TEXT` ("No files match the current filter.", in `filter.rs`)
      while the listing is not empty. The pane's `n / total` and the status
      line's count are over the filtered `files` (unchanged code, now
      meaning the filtered list).
    - `judge`: after `set_state`, `refilter(Some(path), true, None)`; then
      auto-advance only when `judgment::should_advance(cmd, auto_advance,
      still_current)` holds, `still_current` being `current_path() ==
      Some(path)` (a file the judgment filtered out already moved the
      cursor), ported with `advance.test.ts`'s cases into `judgment.rs`.
    - `step` (undo / redo): the `listed` closure is over `all`, not `files`
      (the Tauri `allFiles.includes`), so a judgment on a file the filter
      hides is still undone; after applying, `refilter(anchor)` with the
      anchor the restored file when it passes, else the shown file; the
      note is `Undid <name>` when the view moved to it, `Undid <name>
      (hidden by the filter)` when it is not in `files`.
    - The open: `files` is `all` ordered and filtered before the
      `last_viewed` resume; a remembered file the filter hides resumes at
      `anchor_after_filter`'s neighbour. Pass 1's `Done` refreshes the facts
      and the judgments and refilters with the current file as anchor and
      the scroll kept (the early return makes an unchanged list free).
    - Manual check (copies, `D:\Photos\tests\20261113-filter-sort\filter\`,
      a `--settings` file holding a stored `filter`, since there is no menu
      yet): `{"filter":{"flags":["untagged"]}}`: the strip shows the
      unflagged files only and the pane's total is their count; `p` on the
      current file hides it and lands on the next shown file, and
      auto-advance does not move a second time; `Ctrl+Z` brings it back and
      the view lands on it; `{"filter":{"stars":[3]}}` with `x` on a
      3-star file keeps it shown (a reject keeps its stars);
      `{"filter":{"labels":["none"],"stars":[0],"flags":["untagged"]}}`
      shows the unjudged files only; a filter that matches nothing shows
      `No files match the current filter.`; copying a RAW into the folder
      while filtered re-lists without losing the current file.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `filter.rs`; `native-app.md` records the anchor and `listed` rules.
  - Implementation approach:
    - Assumes Step 1 merged. `passes` takes what the view has: the `State`
      from `judgments` (default unjudged), the `Facts` (orientation; `None`
      before pass 1 fails a non-empty orientation section, as `undefined`
      does), `af_eye_state(facts.focus)` and `focus.eyes.eyes`; the
      `Output` argument is `None` until Step 3 reads it.
    - `anchor_after_filter` with `previous` subsumes `resync::carried_current`
      (same rule with `pass = listed`): keep `carried_current` and its tests
      or make it call the new function, whichever is the smaller diff; do
      not keep two rules that can drift.
    - A refilter rebuilds `Strip::new`, so cells re-request thumbnails the
      `keep` did not carry; that is the Tauri `strip.setFiles` behaviour and
      needs no cache.
- [ ] Step 3: The `Face focus`, `Eyes` and `Output` sections' data: facts from pass 2 as it runs, the export folder read with the listing
  - Done when:
    - `Event::FacesProgress`'s `ready` updates the facts of the named paths
      (the `Focus` rebuilt from `FaceReady` over the entry's `point` /
      `manual_focus`, `None` when the file had no entry yet, as the Tauri
      `applyFaceReady`), and refilters with the current file as anchor and
      the scroll kept only while a `candidates` or `eyes` item is checked
      (the Tauri rule), so the sections fill in as the pass runs. Pass 2's
      `FacesDone` needs no read: the last `FacesProgress` carried the rows.
    - The open's and the re-listing's background task runs
      `riffle_backend::output::outputs_in(Path::new(&dir), &listed.files)`
      next to the listing and hands `Opened` an `outputs: Option<HashSet<String>>`
      (`None` for no export folder or a read error, the error logged once as
      `output filter: …` and shown in the pane's notes as the Tauri status
      does) with the listing set for `output_of`. The view keeps `outputs`
      and `outputs_listing`; `passes` gets `output_of(..)`. The `outputs`
      section is never stored.
    - The section's hidden rule is in `filter.rs`: `FilterState::
      set_output_known(bool)` clears `outputs` when the folder gives no
      answer (the menu of Step 4 hides the section on the same flag),
      tested.
    - Manual check (copies, `D:\Photos\tests\20261113-filter-sort\faces\`:
      a folder with faces and an `Output\` subfolder holding one exported
      JPEG per a few of the RAWs, PhotoLab's names): with
      `{"filter":{"candidates":["good"]}}` on a fresh `--index` the strip
      fills in as `analyzing n / m` runs and ends with the frames whose `f`
      mark is bright green; `{"filter":{"eyes":["closed"]}}` shows the
      closed-eyes frames; the `Output` section's check (the Step 4 menu, or
      a unit-level check until then) shows the exported ones under `yes` and
      the rest under `no`; renaming `Output\` away and back re-lists and the
      section follows.
    - `mise run ci` and `mise run native:check` pass; `native-app.md`
      records where the export read runs and its cost on the open (the
      `scan sidecars`-style log line, `output: dir=… found=N in …ms`).
  - Implementation approach:
    - Assumes Step 2 merged. `outputs_in` does two `read_dir`s at most; it
      runs on the open's background task after `scan::list`, before
      `Scan::prepare`, so the view gets the answer with the listing (the
      Tauri `Promise.all`), never on the main thread.
    - Rebuilding a `Focus` from `FaceReady` needs the entry's `point` and
      `manual_focus`; keep the `Facts` struct holding the whole `Focus` so
      `mark::af_eye_state` and the pane's future Analysis rows read one
      thing.
- [ ] Step 4: The two menus in GPUI: the Sort and Filter buttons, their popovers, Reset, Escape and click-out
  - Done when:
    - `crates/native/src/menu.rs` is a pure model with unit tests: the sort
      menu's items (`Capture time`, `File name`, `Rating`, in `index.html`'s
      order, the checked one the sort in force) and the filter menu's
      sections and items in `index.html`'s order (`Reset`; flags with the
      pick / reject dot colors; stars `★★★★★` … `★` and `Unrated` as the HTML
      labels them; the seven colors and `None` with their `Tint`; `Portrait`
      / `Landscape`; `Face focus` `Good` / `Neutral` / `Low` with
      `MarkState::color` on `Good` and `Low`, without the `?` help (the
      user's decision); `Eyes` `Open` / `Closed` / `Unknown`; `Output` /
      `No output` only while the folder gave an answer), each item's checked
      state from the `FilterState`, and what a click does (`Toggle(section,
      value)`, `Reset`, `Sort(key)`), plus `Menus { open: Option<Which> }`
      with `toggle`, `close`, and the rule that a sort pick closes the menu
      and a filter toggle keeps it open.
    - `main.rs`: the status row gains, at its right end, two buttons
      (`Sort ▾`, `Filter ▾`; the filter one lit while `is_active()`; the
      sort one disabled in a view-only folder) drawn as `div`s with
      `on_click`; an open menu is a `deferred(anchored().position(..)
      .snap_to_window())` popover above its button in the pane's colors
      (`PANE_COLOR`, `MUTED_COLOR`, the check mark as text), its items
      `div`s with `on_click` running the model's action, then `refilter`
      (the current file as anchor; a sort change scrolls it into view);
      nothing is written. `on_mouse_down_out` closes the open menu;
      `("escape", _)` in the key handler, taken literally before the actions
      like `home` / `end` / `q` (no default binding uses `escape`; check and
      record), closes it. The strip's click handler is unchanged: the
      popover is `deferred`, so it is hit before the strip. Nothing is laid
      out while both menus are closed.
    - Manual check (copies, `D:\Photos\tests\20261113-filter-sort\menu\`):
      click `Filter ▾`, check `Picked` and `Red`: the strip shows the picked
      or red files and the button lights; the menu stays open across the
      two clicks; `Escape` closes it; a click on the preview closes it;
      `Reset` shows everything; `Sort ▾` then `Rating` reorders and closes;
      in the JPEG folder the sort button is disabled and the strip is in
      capture order; a held `→` with the filter menu open still pages.
    - Peaking, zoom and the `f` overlay are unaffected: `e`, `z`, `f`
      still toggle with a menu open (the keys go through the window's
      handler as before).
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` names
      `menu.rs`; `native-app.md` records the `deferred` / `anchored` popover
      pattern, the `on_mouse_down_out` behaviour met and whether a `div`
      `on_click` inside a `deferred` element receives the click before the
      strip's `on_mouse_down`.
  - Implementation approach:
    - Assumes Steps 1-3 merged. Keep the popover's item model pure so the
      HTML order and labels are tested, and `main.rs` only maps items to
      `div`s.
    - The Tauri bar sits above the strip; the native status row below it is
      the smallest place for two buttons (the strip's canvas spans the window
      and owns the mouse). If the row's height has to grow for the buttons,
      measure the preview canvas's height change on the compare folders
      (it changes the fit scale, not the page cost).
- [ ] Step 5: The same-session latency gate and the docs
  - Done when:
    - **Gate**, `results.md` in this folder: a same-session A/B as the
      judgments plan's Step 5 (`../_archived/20261110-gpui-judgments/plan.md`),
      **A** = this plan's release build (Steps 1-4 merged), **B** = the
      pre-plan commit this plan branched from (built in release into a
      scratch worktree), both run on
      `D:\Photos\tests\20261108-gpui-compare\ARW` (`--size 1616x1080`) and
      `...\RAF` (`--size 4416x2944`), `--auto 60`, warm runs of A and of B
      on fresh per-binary `--index` files repeated until each binary's last
      scan pass ends uncanceled, then A B A B A B per folder, A and B with
      the same `--settings` test file holding `"xmp"`, no filter and
      `"sortOrder":"name"`, the idle gate, no other `cargo` build running.
      The median of the three runs' p50 `keypressToPixels` of A minus B is at
      most +2.0 ms on ARW and +5.0 ms on RAF; p95 and the per-stage medians
      are recorded, not gated. If missed, `results.md` names the stage and
      the step that added it (Step 1's `refilter` on `Done`, Step 4's status
      row) is reopened with a fix PR before this plan closes.
    - `docs/humans/performance.md` "Native preview (GPUI)" gains one line
      per folder for the pager with the filter and the sort and a pointer to
      this `results.md`; `performance.ja.md` the same in Japanese.
    - `../20261109-gpui-port/plan.md`: Step 7's checkbox stays open (the
      EXIF sections are left); Progress gains `(YYYY-MM-DD) Step 7: the
      filter and the sort, through ../_archived/20261113-gpui-filter-sort/plan.md;
      the per-folder EXIF filter sections remain` (backticks for the path, as
      the Step 6 entry did), and the follow-up slice that ports the EXIF
      sections ticks Step 7.
    - `native-app.md` records what the measurement taught; `CLAUDE.md`'s
      `crates/native` sentence names `sort.rs`, `filter.rs` and `menu.rs`
      (done per step; check here). `mise run ci` and `mise run native:check`
      pass.
  - Implementation approach:
    - Assumes Steps 1-4 merged. Reuse the earlier plans' measurement script
      approach (scratch, not committed; forward slashes in paths, distinct
      `--index` names per run). The measurement starts only after the user
      says the machine may be left idle.

## Trade-offs and risks

- **The filter and the sort key start from the Tauri `settings.json`
  (decided: read-only).** The native app reads `settings.json` read-only
  (Step 6's decision; its settings UI is port Step 9) and the Tauri store is
  written whole by `tauri-plugin-store`, so writing two keys into it risks
  clobbering a Tauri write and waits for Step 9. The native strip opens with
  the filter and order the user last set in the Tauri app; a change in the
  native menu lives for the session. A stale Tauri filter reappears at every
  native launch (lit button, fewer files); the hand-check rule (always a test
  `--settings`) covers development runs.
- **`File name` order (decided: natord).** The Tauri `sort.ts` compares base
  names as plain strings, so `IMG_9.ARW` sorts after `IMG_10.ARW` there; the
  native strip keeps natord, so the two apps order a folder differently only
  where digit runs differ in length, which camera names rarely do. The
  measurement folders keep their order.
- **The sections ported.** Flags, stars, labels, orientation, `Face focus`,
  `Eyes` and `Output` are in; orientation is a remembered section, so
  ignoring it would silently drop part of a stored filter. The per-folder
  **EXIF sections** (`exifKey` over `IndexedFile.exif`: camera, lens,
  aperture, shutter, ISO, focal ranges) are a follow-up slice (decided); a
  stored filter holds none of them, so nothing is dropped meanwhile.
  `Genre` and `recommended` stay Step 9's; `SortKey::from_setting` maps a
  stored `recommended` to `Capture` (the Tauri `sortInForce` while the
  ranking is off) and `FilterState` has no `genres`, both marked as the seam.
- **One list becomes two.** Every index into `files` today means "the
  shown list"; after Step 1 it means the filtered, ordered list, and `all`
  the listing. The places that must use `all`: `history::step`'s `listed`
  (an undo of a hidden file), the `rejectRest` of the future (over
  `allFiles`), the scan's `reconcile` (already the whole listing), the
  watcher's owner. The places that stay on `files`: the pager, strip,
  thumbnails, hot list, `n / total`, `current_path`. Step 1 makes the split
  with the sort alone, so the review of the indexing change is separate
  from the filter's rules.
- **A refilter rebuilds the strip.** `Strip::new` drops the `requested` /
  `missing` / `ready` bookkeeping and `keep` carries only the loaded cells,
  so a judgment that changes the list makes the cells in range ask the
  index again (the Tauri `strip.setFiles` does the same). With the early
  return on an unchanged list this costs nothing on a judgment that does
  not change the list; under the rating sort every rating key reorders.
  Not on the page path, so not gated; if it shows in a hand check as a
  flicker, add a carried bookkeeping in a fix PR.
- **Pass 2 refilters while a `Face focus` or `Eyes` item is checked.**
  Each `FacesProgress` batch can move the current file's neighbours; the
  Tauri app accepts that (the file in view is the anchor, the scroll kept).
  A frame judged `Low` while shown under `good` leaves the view; that is the
  Tauri behaviour.
- **GPUI popover behaviour is unverified.** `deferred` + `anchored` and
  `on_mouse_down_out` exist at the pinned rev but nothing in the native app
  has used them; whether a click inside the popover reaches its `on_click`
  before the strip's `on_mouse_down`, and whether `on_mouse_down_out` fires
  on a click in the strip, is what Step 4 checks and records. The fallback
  is a plain `div` menu in the status row's flow (pushing the strip up while
  open), which needs no deferred element.
- **The gate's noise.** As before, `frameWait` moves 6-11 ms between runs;
  the same-session A/B absorbs it and n=60 cannot gate p95. The new work on
  the open (the entries read already existed; `outputs_in` is two
  `read_dir`s) is off the page path; the status row's two buttons are the
  only per-frame addition and land in `present`.

## Progress

- (2026-11-13) Step 1 complete
