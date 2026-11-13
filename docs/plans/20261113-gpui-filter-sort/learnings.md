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

## Deferred issues (todo candidates)

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
