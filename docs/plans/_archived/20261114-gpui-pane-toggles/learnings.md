# Learnings: GPUI pane toggles and Open Folder

## Step 1

- `Riffle::new` already takes seven arguments, so an eighth (`--auto`) would
  trip `clippy::too_many_arguments`. `main` instead sets `panels` to all shown
  and `remember_panels` to `false` on the value `Riffle::new` returns, inside
  the `cx.new` closure. No `--auto` suppression flag from session-restore had
  landed on `main` yet.
- The strip `div` is now drawn through `.children(self.panels.strip.then(..))`;
  its bookkeeping (range, thumbnails, `strip_width`) keeps running while it is
  hidden. Whether showing it again draws around the current file with its
  thumbnails is part of the pending hand check below.
- `mise run fmt` failed at first because this worktree had no `node_modules`
  (`vite-plus/bin/vp` missing); `mise exec -- pnpm install --frozen-lockfile`
  fixed it (environmental, `pnpm` is only on the PATH through mise).
- Launch check done here (Windows): a fresh store seeded with
  `{"panels":{"left":false}}` through `--import-settings`, launched with
  `--auto 0` on an empty folder, quit cleanly and the store still held
  exactly `{"left":false}` afterwards (nothing written under `--auto`).
  What the window drew was not inspected.

## Deferred issues (todo candidates)

- **Pending manual check (Windows, the user's): the pane toggles and Open
  Folder hand check of plan Step 1.** The step's checkbox was ticked on the
  automated criteria (unit tests for `panels.rs` and `settings.rs`'s load,
  `mise run native:check`, `mise run ci`, and the `--auto` launch above).
  Basis: plan.md Step 1's "Hand check (Windows)" and "Verify during
  implementation"; files `crates/native/src/main.rs` (`pick_folder`,
  `change_panels`, `render`), `crates/native/src/panels.rs`. Run each with
  `--settings <fresh test store>` and `--log <test file>`, never the real
  store:
  1. `ctrl+o`, pick a folder: it opens and the tree reveals it. Again,
     cancel: nothing changes. Again, with the tree holding the keyboard.
     Also note whether a second `ctrl+o` while the picker is up opens a
     second dialog (if so, a flag ignoring the press is needed) and whether
     the culling keys work right after the picker closes without a click.
  2. `ctrl+alt+arrowright`, `ctrl+alt+arrowdown`, `ctrl+alt+arrowleft` and
     `tab`, each twice, with a photo shown, again at 1:1 (`z`) and with
     peaking on (`e`): the canvas resizes and the crop is re-cut. With the
     strip hidden, paging, judging and the `Sort` / `Filter` menus still
     work, and showing the strip again draws it around the current file with
     its thumbnails; the perf overlay (`d`) still sits along the canvas's
     bottom with panes hidden.
  3. Give the tree the keyboard, hide the left pane with `tab`: the arrow
     keys page the photos again.
  4. Hide the right pane and the strip, quit with `q`, relaunch: both still
     hidden. Repeat quitting by the title bar's close button.
  5. Put `{"panels": {"left": false}}` in a Tauri-style `settings.json` and
     launch with `--import-settings` on a new store: the tree is hidden, the
     other two shown.
  6. Rebind one of the actions in the Keyboard Shortcuts tab: the new key
     runs it.
  Expected: as described in each item; record the results in this file and
  update `docs/agents/native-app.md`'s "Not verified" note on the picker.
