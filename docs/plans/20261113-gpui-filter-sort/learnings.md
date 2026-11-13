# Learnings: gpui-filter-sort

## Step 1 (the sort order)

- `sort.test.ts` has 9 cases; 7 are ported. The two `recommended` cases are
  port Step 9's (no `Recommended` key yet) and are named as left out in the
  test module's doc comment. None of the 7 depended on the Tauri plain compare,
  so a natord case was added instead (`name_is_natord_not_the_plain_compare`:
  `DSC9` before `DSC10`).
- `refilter` replaces the pager (`Pager::new`), which loses the pager's
  in-flight flag. A preview read still out would come back with an index into
  the old `files` and could be shown under the wrong cell. The view gained an
  `order` counter, bumped by every listing and every rebuild of `files`, which
  `send` checks instead of `listing`; `listing` still guards the index reads
  (so a rating key pressed while pass 1's `Done` read is out does not drop that
  read). After a rebuild, `refilter` re-requests the current preview whenever
  the image on screen (`shown`, remapped to its new index) is not the current
  file's.
- `refilter` takes `previous` as the plan names it; in this step it only
  disables the early return (a re-listing always carries) and tells `refilter`
  that its caller (`opened`) reads the current preview, the focus and the crop
  itself. Step 2's `anchor_after_filter` is where `previous` gets read as a
  list.
- The plan's Step 1 names `refilter` on the resync path and on pass 1's `Done`;
  its manual check also expects `3` under `"rating"` to reorder at once, so
  `judge`, `step` (undo / redo) and a failed store's revert call it too (the
  Tauri `commit` / `step`). `judge` has no "still current" check yet (Step 2's
  `should_advance`): without a filter the judged file is never hidden, so
  auto-advance moves from its new place, which is what the manual check wants.
- `Facts` keeps `orientation` and `focus` for Step 2's filter; unused until
  then, they carry `#[expect(dead_code, reason = ...)]` so clippy's
  `-D warnings` passes and the attribute fails once Step 2 reads them.
- Timing (a throwaway unit test, not committed): `order_files` over 1,000
  `\\?\D:\...` paths with a lookup that allocates two strings per call took
  0.40-0.50 ms per sort in a release test build and 13-17 ms in the debug
  one. The first version padded the subsec with `format!` per comparison
  (1.05 ms release for capture order); comparing the padded bytes without
  allocating halved it. The debug build is over 1 ms; the release app is not.
  It runs on a list change only, never per page.
- Headless checks (debug exe, a fresh `--index` per run in the scratchpad,
  `--settings` test files under `D:\Photos\tests\20261113-filter-sort\`):
  - `sort\raw` (6 ARWs from `D:\Photos\samples\ARW`, capture years 2016-2021
    out of name order; XMP sidecars: HX99 4 stars, RX0 2 stars, HX95 5 stars
    rejected), `"capture"`, `--auto 5`: paged RX100M2, RX0, HX99, RX0M2,
    RX100M3, capture order. The open had no facts (fresh index), so it started
    on HX95 in name order; pass 1's `Done` reordered with HX95 kept current
    (now last), and the first `Next` wrapped to the earliest.
  - Same folder, `"rating"`, `--auto 6`: HX99 (4), RX0 (2), RX0M2, RX100M2,
    RX100M3 (unrated, by name), HX95 (rejected, 5 stars) last.
  - `sort\jpg-sequenced` (4 JPEGs, 1999-2025, name order differs), `"name"`,
    `--auto 4`: C2000Z, C960Z, COOLPIX, A065: capture order in a view-only
    folder.

## Step 2 (the filter model and the judgment sections)

- `riffle_core::eyes::EyeState` has only `Open` / `Closed` and no `Hash`, so
  the `Eyes` section's set holds a filter-local `EyesItem { Open, Closed,
  Unknown }` (`EyesItem::of(Option<EyeState>)`); `passes` still takes the
  backend's `Option<EyeState>` as the plan names it. `MarkState` gained
  `Hash` for the `candidates` set.
- `Facts` moved from `main.rs` into `filter.rs`, since `passes` takes
  `Option<&Facts>` (orientation; `None` before pass 1 fails a non-empty
  orientation section). Its `expect(dead_code)` attributes went with it.
- `output_of` and `FilterState::is_active` have no caller until Steps 3 and 4;
  they carry `#[cfg_attr(not(test), expect(dead_code, ...))]`, since a plain
  `expect` would be unfulfilled in the test build where the unit tests use
  them.
- `resync::carried_current` was removed rather than kept beside
  `anchor_after_filter` (the plan's "no two rules that can drift"): nothing
  else called it, and its cases are the `anchor_after_filter` previous-list
  cases with every file passing. CLAUDE.md's `resync.rs` sentence now points
  at `filter::anchor_after_filter`.
- A re-listing's `previous` is now the old `files` (the list shown), as the
  Tauri `resync` passes `files`, not the old `all` Step 1 passed.
- `judge` keeps `refilter(Some(path), false, None)` (scroll the current file
  into view), not the plan's `true`: Step 1 and `native-app.md` already record
  the judgment's refilter as the Tauri `keepScroll` false, and under the
  rating sort `true` would leave the reordered current file off screen (its
  path does not change, so `carry` does not count it as moved).
- Test count: `filter.test.ts` has 82 `test` calls when the judgment
  describe's generated cases are counted one by one (the plan's 70 counted
  differently). Left out: the 6 `Genre` cases, the EXIF selection case and
  `applyStored leaves the EXIF groups alone`; `toStored` is not ported
  (nothing is written), so its round-trip case became `apply_stored` cases;
  the 12 generated `a judgment under untagged + 0 + none` cases are one Rust
  test looping over the four judgments. `advance.test.ts`'s `shouldAdvance`
  cases are ported less the selection-count and Compare inputs the native app
  does not have.
- Headless checks (debug exe, fresh `--index` per run in the scratchpad,
  copies under `D:\Photos\tests\20261113-filter-sort\filter\`, `--auto 6`):
  `settings-filter-untagged.json` cycled the 5 unflagged files, HX95 (rejected
  in its XMP) never paged; `settings-filter-unjudged.json` cycled RX100M2,
  RX100M3, RX0M2; `settings-filter-stars3.json` (no 3-star file) paged nothing.
  `filter\resync` with `--auto 9` under the untagged filter and a RAW copied in
  at 6 s: the re-listing kept HX99 current, the next pages went RX0M2_224,
  the new RX0M2_225, RX0, RX100M2, HX95 still hidden. `filter\stars3` (RX0's
  XMP edited to 3 stars) under `settings-filter-stars34.json` (`stars: [3, 4]`)
  paged RX0, HX99, RX0: on the fresh index the open's list is empty (no
  judgments yet, every file 0 stars) and pass 1's `Done` refilter brings the
  two files in and pages them, so the empty-to-filled transition works. The
  judgment keys,
  Ctrl+Z and the `FILTERED_TEXT` display cannot be driven by `--auto`; they
  are the pending manual check below.

## Step 3 (the Face focus, Eyes and Output sections' data)

- `FaceReady` carries `eye_focus`, `candidate` and the whole `StoredEyes`
  (all `Copy`), so `filter::apply_face_ready` patches the three into the
  existing `Focus` and keeps its `point` / `manual_focus`; a file without
  facts or without a focus is skipped, as the Tauri `applyFaceReady`.
- The export read is `scan::outputs(&Listed) -> Outputs { found, listing,
  failure }`, called in `list`'s background task right after `scan::list`.
  The failure is kept in its own `outputs_failure` field, not in `notes`
  (which `opened` clears and refills with the sidecar errors), so it stays
  until a read succeeds, as the Tauri `outputsFailure`. `output_of` lost its
  `expect(dead_code)`; `is_active` keeps its own (Step 4's button).
- `set_output_known(false)` runs before the open's ordering, so a re-listing
  that loses the export folder clears the `outputs` checks before `passes`
  reads them.
- Headless checks (debug exe, fresh `--index` per run in the scratchpad,
  `D:\Photos\tests\20261113-filter-sort\faces\`: the 8 ARWs of
  `20261110-gpui-pass2\marks` and an `Output\` holding `_DSC1616.jpg`,
  `ILCE-6400_DSC00405_1.jpg` (a virtual copy) and `_DSC1758-DxO_DeepPRIME
  3.jpg`):
  - `settings-filter-good.json` (`candidates: ["good"]`), `--auto 40`: the
    open's and pass 1's lists are empty on the fresh index; pass 2 (3.0 s)
    filled in DSC00405, _DSC1758 and ILCE-1M2_sony_a1_ii_03, which the run
    then cycled. FacesDone reads nothing, so the fill came from
    `FacesProgress`.
  - `settings-filter-open.json` (`eyes: ["open"]`), `--auto 12`: cycled
    a1_ii_03, DSC00405, DSC01340, _DSC1758.
    `settings-filter-closed.json` (`eyes: ["closed"]`): no page lines, as
    only DSC03068 is closed (its earlier pass-2 read: `eyes: Some(Closed)`)
    and a one-file list does not page.
  - `output: dir=… found=3 in 0ms` at the open; renaming `Output\` away at
    8 s and back at 12 s re-listed twice, logging `found=none` then
    `found=3`.
- The `Output` section cannot be checked end to end yet: nothing stores an
  `outputs` filter (the Tauri app never remembers it) and the menu is Step 4.
  It is covered by `filter.rs`'s unit tests (`output_of`, `passes` with an
  output, `set_output_known`) and the backend's `output.rs` tests for the
  matching; the log lines above show the answer reaching the view.

## Deferred issues (todo candidates)

- **The pane's `n / total` lags a refilter that hides the shown file
  (Step 2 local review, round 1, not acted on):** until the new current
  file's preview loads, the pane still names the old position. The same lag
  already followed a re-listing before this plan. Fix: rebuild the pane's
  position line in `refilter` when the current index changes, not on the
  preview's arrival.
- **Pending manual check (Step 2, Windows, the user's):** the filter through
  the judgment keys and undo. Basis: plan Step 2's manual check; the `--auto`
  runs above covered the page order headlessly, not key presses or the drawn
  text. Steps: build `crates/native` (`cargo build`, NASM on `PATH`), run
  `riffle-native D:\Photos\tests\20261113-filter-sort\filter\raw --index <fresh
  path> --settings D:\Photos\tests\20261113-filter-sort\settings-filter-untagged.json`.
  Expected: once pass 1 ends the strip shows the 5 unflagged files (HX95
  absent) and the pane's total is 5; `p` on the current file hides it and
  lands on the next shown file, with no second move from auto-advance; `Ctrl+Z`
  brings it back and the view lands on it (`Undid <name>`). On
  `...\filter\stars3` (RX0 has 3 stars in its XMP) with
  `settings-filter-stars3.json`: only RX0 shows; `x` on it keeps it shown (a
  reject keeps its stars). With `settings-filter-unjudged.json` only RX0M2, RX100M2 and RX100M3
  show. `settings-filter-none.json` (picked only; no picked file) shows `No
  files match the current filter.` on the status line and in the pane, with
  no image. Copying a RAW into the folder while filtered re-lists without
  losing the current file. The p / undo / redo sidecar writes land in the
  copies only. Step 2's checkbox was ticked on the automated criteria and the
  headless runs.

- **Pending manual check (Step 1, Windows, the user's):** the strip and the
  `3` key under `"rating"`. Basis: plan Step 1's manual check; the `--auto`
  runs above covered the page order headlessly, not the strip's drawing or a
  key press. Steps: build `crates/native` (`cargo build`, NASM on `PATH`), run
  `riffle-native D:\Photos\tests\20261113-filter-sort\sort\raw --index <fresh
  path> --settings D:\Photos\tests\20261113-filter-sort\settings-rating.json`.
  Expected: the strip shows HX99, RX0, RX0M2, RX100M2, RX100M3, HX95 (dimmed)
  once pass 1 ends; on RX100M2 press `3`: the strip reorders at once with
  RX100M2 right after HX99, it stays current (outlined, preview unchanged),
  and auto-advance then moves to its new right neighbour (RX0). Repeat with
  `settings-capture.json` (strip in capture order) and on
  `...\sort\jpg-sequenced` with `settings-name.json` (strip in capture order:
  C2000Z, C960Z, COOLPIX, A065). Step 1's checkbox was ticked on the
  automated criteria and the headless runs.

- **Pending manual check (Step 3, Windows, the user's):** the `Face focus`,
  `Eyes` and `Output` sections in the window. Basis: plan Step 3's manual
  check; the `--auto` runs above covered the page order and the log lines
  headlessly, not the strip's drawing or the `f` marks. Steps: build
  `crates/native` (`cargo build`, NASM on `PATH`), run `riffle-native
  D:\Photos\tests\20261113-filter-sort\faces --index <fresh path> --settings
  D:\Photos\tests\20261113-filter-sort\settings-filter-good.json`. Expected:
  the strip starts empty and fills in while `analyzing n / m` runs, ending with
  DSC00405, _DSC1758 and ILCE-1M2_sony_a1_ii_03, each with a bright green `f`
  mark. With `settings-filter-closed.json` only DSC03068 shows. Once Step 4's
  menu exists: `Output` shows DSC00405, _DSC1616 and _DSC1758, `No output` the
  other 5; renaming `Output\` away hides the section (its checks cleared) and
  renaming it back brings it back. Step 3's checkbox was ticked on the
  automated criteria and the headless runs.

- **A pass-1 entries read can overwrite pass-2 facts (Step 3, not acted
  on):** `read_entries` (pass 1's `Done`) replaces `facts` wholesale when its
  background read returns; pass 2 starts at the same time, so a
  `FacesProgress` applied before that read returns is overwritten by a read
  that started before the pass-2 write, and a `FacesProgress` arriving before
  the read returns finds no facts on a fresh index and is skipped (the Tauri
  `folder_entries` refresh after pass 1 looks to have the same window). The
  file keeps `Neutral` / unknown eyes until the next re-listing. The headless
  runs did not show it. Fix if it shows: re-apply the
  `FacesProgress` batches received since the read was issued, or read the
  entries again at `FacesDone`. Files: `crates/native/src/main.rs`
  (`read_entries`, `scanned`).
