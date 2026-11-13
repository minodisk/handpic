# Learnings: GPUI EXIF filter sections

## Step 1

- The port lives in a new `crates/native/src/exif.rs` (the one-`.rs`-per-`.ts`
  convention), not in `filter.rs`. `main.rs` imported
  `riffle_backend::exif::{self, Metadata}`, whose `exif` name clashes with
  `mod exif;`, so the import became `exif::Metadata` and the one
  `exif::metadata` call in `load` is spelled out as
  `riffle_backend::exif::metadata`. Two extra lines in `main.rs`.
- `FilterState::exif` is a `BTreeMap<Group, BTreeSet<String>>` whose emptied
  set is removed (`toggle_exif`, `prune_exif`), so a toggled on-then-off state
  still equals `FilterState::default()` and `is_active` is `!exif.is_empty()`.
- The groups (`exif::Groups`) ride on the `Entries` tuple from `entries_of`'s
  background task; `set_entries` prunes the checks before storing them, which
  keeps the prune ahead of the `refilter` in `opened`'s carry branch and in
  `read_entries`. `opened`'s other-folder branch calls `clear_exif` first.
- `menu::Label` gained `Owned(String)`; `Row`, `Item`, `Label`, `Toggle` and
  `Action` are `Clone` now. `Menus::acted` takes `&Action`, and
  `menu_action` calls it before `apply` consumes the action (the order does
  not matter: only a sort pick closes the menu).
- The stale `shown.0` the plan suspected is real by reading: after `p` under
  `untagged` hides `b` of `a b c`, `shown.0` stayed 1, which in `a c` names
  `c`; a second `refilter` that kept `c` current before `c`'s preview arrived
  took `c` as the file on screen, skipped the re-request, and the `order` bump
  had already dropped `c`'s read, leaving `b`'s image up with `c` current.
  Fixed in the same change: `shown` is now `Option<(Option<usize>, Shown)>`,
  the index set to `None` by `filter::shown_after` once a refilter hides the
  file (a few lines in `main.rs`). Not reproduced in the GUI.
- The pane's `n / total` is `meta::position(pager.current(), files.len())`.
- `filter::KeptAnchor` is updated inside `refilter` (every refilter that
  changes the list: a judgment emptying the list keeps the judged file too),
  and read only by `menu_action`. It is cleared by another folder's open and
  by any refilter that leaves the list non-empty.
- Timing: `exif::groups_of` over 5,000 synthetic entries (6 groups, about 110
  labels) took 5.8 ms in a debug test build (throwaway `#[ignore]` test, not
  committed). It runs on the entries read's background task, never per page.
- No A/B was run: the change is off the page path (the groups are built only
  when the entries are read, the menu rows only while the menu is open, the
  `passes` clause is skipped while no EXIF item is checked, `refilter` never
  runs per page, and the pane fix changes which number the existing line
  formats without adding an element), per native-app.md's "A same-session A/B
  gates only work on the page path".
- One debug `--auto 6` run on the copy
  `D:\Photos\tests\20261113-filter-sort\filter\raw` with a fresh index under
  `D:\Photos\tests\20261113-exif-filter\` and the
  `settings-filter-untagged.json` settings: the open, pass 1's `Done`
  (6 files, 137 ms), pass 2 and the paging over the 5 untagged files behaved
  as before; both quit drains reported true.
- The worktree had no `node_modules`, so `mise run fmt` failed on `vp` until
  `mise exec -- pnpm install --frozen-lockfile` (`pnpm` is only on the PATH
  through mise). The C: drive was briefly full (0 MB) at the start of the
  session and had recovered by the first build.
- Python's text-mode `open(..., "w")` on Windows wrote CRLF into the `.rs`
  files; normalize with `newline=""` or `sed -i 's/\r$//'` before committing.

## Deferred issues (todo candidates)

- Pending manual check (Windows, debug build, `mise run native:dev` on a copy
  with several cameras such as
  `D:\Photos\tests\20261113-filter-sort\sort\raw`, a fresh `--index` and a
  test `--settings`; Step 1's checkbox was ticked on the automated criteria):
  1. The filter menu ends with `Camera`, `Lens`, `Aperture`,
     `Shutter speed`, `ISO`, `Focal length`, each listing the folder's values
     in order; it scrolls when taller than the window.
  2. Checking one camera narrows the strip and lights the button; a second
     camera widens it (OR); an ISO item on top narrows it (AND); `Reset`
     clears all.
  3. On a fresh index the sections appear once pass 1 ends.
  4. Copying in a RAW from another camera adds its item after the re-scan;
     removing the only file of a checked camera drops that check instead of
     leaving an empty strip.
  5. Under the `untagged` filter, `p` on the current file updates the pane's
     `n / total` at once (the name follows when the next preview arrives).
  6. With a filter that hides every file, `Reset` or unchecking the item
     comes back to the file shown before, not the first one.
  7. The settings file is untouched (its modification time unchanged).
- Pending manual check of the stale shown index fix (same setup, on a fresh
  index so pass 2's `FacesProgress` refilters under a `Face focus` check
  while you judge): under `untagged` plus a `Face focus` item, press `p`
  repeatedly while pass 2 runs; the image on screen must always end up the
  current file's, never a judged file's left behind. Basis: the
  `shown_after` change in `crates/native/src/main.rs`'s `refilter`, found by
  reading only; the race is timing-dependent and may not show either way.
- `Camera` / `Lens` order is a case-insensitive compare, not the Tauri
  `localeCompare` (ICU); it may differ for punctuation or non-ASCII model
  names. Basis: the plan's trade-off; `crates/native/src/exif.rs` `compare`.
- The status line's `n/total` still lags a refilter that hides the shown file
  (it only follows the shown file's new place). Basis: the plan left it alone;
  `crates/native/src/main.rs` `refilter`.
