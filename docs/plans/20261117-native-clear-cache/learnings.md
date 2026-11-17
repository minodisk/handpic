# Learnings: Native Clear Cache

## Step 1

- The flow lives in the window (`Riffle::clear`, a `cache::ClearFlow`), not
  in `SettingsView`: the busy rule, the dialog and the run are the window's.
  The settings view only shows the size, the waiting note and the disabled
  button through `show_index_size` / `show_clear`, and emits
  `Change::ClearCache` on a press.
- A confirmation answered while the view became busy again does not ask a
  second time: `ClearFlow` keeps `Waiting { confirmed: true }` and the next
  idle point runs it directly. The Tauri app refused there with
  `SCAN_RUNNING`; the plan's "held, not refused" rule was applied instead.
- `drain_clear` runs at the end of `finished`, after the gate's arms that
  return early and after `resync.finished()`; it re-checks the busy rule
  itself, so a re-listing that `resync.finished()` just started keeps it
  held. It also waits while a trash or folder dialog is up, so two
  component dialogs never stack in `Root`'s layer.
- On a failed clear the `resync.hold()` is released through
  `resync.finished()` (a change seen meanwhile still re-lists); with no
  folder open the same call releases it after a successful clear.
- `Indexes` now carries the index path `open_index` opened, so `--index` is
  honored for the size line.
- State mirroring the emptied tables, decided to leave as is: the `viewed`
  set and `last_viewed` in memory, `viewed.rs`'s `Batches` / `Latest` and a
  `CountWrites` write in flight. A write landing after the clear only
  re-adds a true row (a file really viewed, a folder really counted), so
  nothing is reset. `relist` carries the view, so the dropped
  `folders.last_viewed` does not move the current file.
- `VACUUM` with the reader connection open: the Tauri app clears the same
  way with its own reader open, and `Index::clear` only logs a failed WAL
  truncate. Not observed in the native app yet (see the pending check).
- `--auto`: nothing in this step reads or writes a setting; no special case.
- `cargo check --tests` in `crates/native` passed on the first try.

## Deferred issues (todo candidates)

- **Pending manual check** (Windows, the native app; Step 1's checkbox was
  ticked on the automated criteria only). Basis: Step 1's Done-when asks for
  a `todo.md` real-device item; this agent cannot run the GUI. Files:
  `crates/native/src/main.rs` (`press_clear`, `follow_clear`, `cache_dialog`,
  `run_clear`, `cleared`, `refresh_index_size`),
  `crates/native/src/settingsview.rs` (`cache`), `crates/native/src/cache.rs`.
  Steps and expected results:
  1. Launch with a test `--settings` / `--index`, open a scanned folder, open
     the settings (`ctrl+,`) and pick `Cache`. Expected: `Index cache: <size>`
     with a non-zero size, the paragraph and `Clear Cache`.
  2. Press `Clear Cache`. Expected: `Clear the index cache?` opens over the
     settings with the dimmed backdrop, focus on `Clear`. Press Escape.
     Expected: only the dialog closes, the settings stay open, nothing is
     cleared (size unchanged), and Tab still walks the settings' controls.
  3. Press `Clear Cache` again and confirm with Enter. Expected: the log has
     `index cleared: EvictSummary { … }`; the size shrinks; the open folder
     is listed again and pass 1 / pass 2 refill thumbnails, the meta pane and
     the filter facts, the current file staying put; the tree's cached
     counts of unlisted folders disappear.
  4. Open a large unscanned folder and, while `scanning` runs, press
     `Clear Cache`. Expected: the note `The cache is cleared as soon as the
     running scan finishes.`, then the dialog opens by itself when the scan
     ends. Repeat and close the settings while waiting: expected no dialog
     at the scan's end.
