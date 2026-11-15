# Learnings

## Step 1

- `present` now sets `self.status` to an empty `SharedString`; `px`,
  `orientation` and `meta` stay destructured for `OnScreen`, `marks.show` and
  `Shown::Read`. `status_line`, `dash` and `refilter`'s `n/total` rewrite are
  gone; `Metadata` and `file_name` keep other callers.
- `status()` trims the start of the built string, so the suffixes keep their
  own two leading spaces (`scan::Status::suffix` included, its tests
  untouched) and lose them only when the note is empty.
- Row height: `status_row` always renders the `Sort ▾` and `Filter ▾` buttons
  (`menu_button` returns an element even when disabled), so they hold the
  row's height when the status text is empty. Not checked by running the app
  here (no GUI run in this environment); see the pending manual check below.

## Deferred issues (todo candidates)

- Pending manual check (Windows, also any platform the native app is run on):
  with `crates/native` built, open a folder and page through it with the scan
  finished and zoom off, so the status text is empty; then press `z` (adds
  `1:1`) and drop two files on the window (shows `DROP_TEXT`). Expected: the
  status row shows nothing on the left while paging, the preview area above
  does not shift in height between the empty and the non-empty status, and
  `1:1` / a scan suffix start at the row's left edge without leading spaces.
  Basis: the plan's "Verify while implementing" note on the status row's
  height; file `crates/native/src/main.rs` (`status_row`, `status`). Step 1's
  checkbox was ticked on the automated criteria.

## Local checks

- The first `mise run fmt` failed after `cargo fmt` with `Cannot find module
  ...node_modules/vite-plus/bin/vp`: this worktree had no `node_modules`.
  `pnpm` is not on the plain PATH here; `mise exec -- pnpm install
  --frozen-lockfile` fixed it, and `mise run fmt` then passed with no changes.
