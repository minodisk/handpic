# Learnings

## Step 1

- Test (c), `a_held_collapse_click_follows_a_date_folder_rename`, was run
  once before any fix (2026-11-27, Windows 11) and **failed** at the last
  assertion: after `folder_renamed` the renamed folder opened and its row was
  expanded. The case is real: `folder_renamed` rebased `deferred_open` but
  not `reveal_collapsed`, so `reveal`'s `tree::relation` check missed the new
  path. The fix rebases `reveal_collapsed` with the same `moved` closure;
  (c) passes after it. Tests (a) and (b) passed as written, confirming the
  tree's `Rename…` cannot reach the case.
- (c) took the direct route (gate held by hand, `fs::rename`, then
  `folder_renamed`), not a real `run_confirmed(FolderOp::Date(..))`: the
  generated DNG in `uitest.rs` carries no capture date, so it would need a
  new fixture.
- The tree fixture: wait for `roots_loaded`, `tree.add_roots` the `Ui`'s
  temporary folder, `tree_toggle` it and then the folder, waiting for each
  listing; the folder's path is taken from its drawn row. The reveal's end
  is waited on through `tree_cursor`, which only the reveal's last update
  sets here. Verified on Windows only locally; macOS and Linux through the
  PR's checks.
- A bare `python3 -` in Git Bash blocks on stdin (as `CLAUDE.md` warns);
  several such hung processes from other sessions were seen on this host.

## Deferred issues (todo candidates)

- App: a `Rename by Capture Date…` split that removes the folder leaves a
  `deferred_open` naming a folder that no longer exists. In
  `date_folder_ran`'s split branch (`crates/app/src/folderops.rs`), when
  `self.deferred_open.is_some()` it goes to `settle_folder_run`, which opens
  the deferred folder even when `split.removed` means it is gone (a click on
  the folder, or a folder under it, during the run). Expected: a deferred
  open of the removed folder lands where an open of it would after the split
  (its first date folder, as the `opens` branch does), or is dropped. Basis:
  noticed while planning `docs/plans/20261127-reveal-collapsed-rename`
  (its "Trade-offs and risks"), left out of that PR. Files:
  `crates/app/src/folderops.rs` (`date_folder_ran`, `settle_folder_run`),
  `crates/app/src/main.rs` (`open`, `deferred_open`).
