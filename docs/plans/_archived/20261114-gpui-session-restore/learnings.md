# Learnings: gpui-session-restore

## Step 1: Restore the window geometry and the last folder on launch

- Closing the window on Windows fires `observe_window_bounds` once more while
  the handle is being destroyed: `GetWindowPlacement` fails (`failed to get
  window placement: ... (0x80070578)` in the log) and `window_bounds()`
  returns the frame insets around an empty rectangle
  (`x 8, y 19, width -16, height -39`). The first test run with the close
  button stored exactly that. The observer now keeps only a positive size
  (`Geometry::is_usable`); `from_setting` would also have refused it on the
  next launch, but the previous good value would have been overwritten.
- `listed.dir` is the canonical path, `\?\`-prefixed on Windows. Writing it
  as `lastFolder` would make the next launch's `reveal` look for a
  `\?\C:` root, so it is written as `context::shown_path` of it.
- GPUI's restore bounds are not the outer window rectangle: a window moved to
  the outer rectangle (200, 150)-(1400, 950) at 96 DPI reported
  `x 146, y 169, width 1184, height 761`, and the same stored value reopened at
  exactly that outer rectangle (a second check: stored (300, 200) 1000x700
  opened at (354, 181)-(1370, 920)). The values round-trip; only the clamp
  against `visible_bounds` mixes the two systems, so a window saved flush
  against an edge may come back shifted by those few tens of pixels.
- `Riffle::new` reached eight parameters with `remember`, so it carries
  `#[allow(clippy::too_many_arguments)]`, as `filter.rs` and `judgment.rs` do.
- The worktree had no `node_modules`, so `mise run fmt` failed on
  `vite-plus`; `mise exec -- pnpm install --frozen-lockfile` fixed it.
- Driving the window from PowerShell: `Get-Process riffle-native` returns
  every leftover instance, so take one (`Select-Object -First 1`) or the
  `IntPtr` argument becomes an array; `.ps1` files are blocked by the
  execution policy here, so pass the script with `-Command`.
- Checked by hand here (Windows 11, one display, 96 DPI, test store, test
  index, test log, quitting with `CloseMainWindow`, the title bar's close
  path): moved and resized window reopened at the same outer rectangle; a
  maximized quit reopened maximized and un-maximized to the earlier bounds; a
  minimized quit kept the restore bounds; bounds far off every display opened
  at the default; an unknown display UUID opened on the intersecting display;
  a relaunch without arguments reopened the folder; a renamed remembered folder
  opened nothing (no `scan list` line); `--auto 1 <other folder>` left both
  keys unchanged and logged no `window: restored`.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, Windows, the user's). The step's checkbox was
  ticked on the automated criteria and the checks listed above; these remain,
  each run with `--settings <fresh test store>`, `--index <test index>` and
  `--log <test file>`:
  1. Quit with `q` after moving and resizing: relaunch at the same place and
     size.
  2. Quit while snapped to a screen half: relaunch on-screen at the restore
     bounds.
  3. On a second monitor, ideally with a different scale factor: the window
     comes back on that monitor, windowed and maximized.
  4. With the taskbar docked at the top or left, move the window, quit,
     relaunch: record whether the position drifts by the taskbar's size
     (`rcNormalPosition` is in workspace coordinates); fix only if it drifts.
  5. Open a folder from the tree, page a few files, quit, relaunch without
     arguments: that folder is revealed in the tree at that file. Relaunch
     with another folder as the argument: the argument's opens.
  6. `--auto 3 <folder>`: visually opens centered at 1600x1000 whatever is
     stored.
  Record the results here and as `(Measured)` in `docs/agents/native-app.md`
  ("A launch restores the last folder and the window"), removing its "Not
  seen yet" bullet. Basis: plan.md Step 1's "Verify during implementation"
  list; files `crates/native/src/session.rs`, `crates/native/src/main.rs`.
