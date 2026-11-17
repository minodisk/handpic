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

# Native Clear Cache

## Purpose

The GPUI app (`crates/native`) has no way to empty the SQLite index cache.
`Index::clear` (`crates/backend/src/index.rs`) is only called by the frozen
Tauri app's settings modal (`clear_index` in `crates/app/src/commands.rs`), so
in the native app the thumbnails, metadata, `views` rows and, since the
native-folder-count-cache work, the `folder_counts` rows can never be dropped
by the user. This plan ports the Tauri settings modal's `Cache` tab to the
native settings modal: the index's size on disk, a `Clear Cache` button with a
confirmation, the wait for a running scan, and the refresh of the open folder
and of the tree's cached counts afterwards.

## Steps

- [x] Step 1: Add the Cache page with Clear Cache to the native settings modal
  - Done when:
    - The native settings modal has a `Cache` page, placed as in the Tauri
      app (after `Keyboard Shortcuts`, before `Import`), showing
      `Index cache: <size>` (the index file plus its `-wal` and `-shm`,
      formatted with the backend's `format_bytes` / `SIZE_BASE`), the Tauri
      explanation paragraph and a `Clear Cache` button.
    - Pressing `Clear Cache` and confirming runs `Index::clear` on the
      writer connection, off the UI thread, after the queued judgments
      reached the sidecar writer and the writer was flushed, and logs the
      `EvictSummary`. Cancelling the confirmation changes nothing.
    - A press while the open folder is being listed or scanned, or while a
      run holding the folder is in flight, is held, not refused: the page
      shows `The cache is cleared as soon as the running scan finishes.`,
      the clear goes on by itself when the view becomes idle, and closing
      the settings modal drops the held clear (the Tauri `clearGate`
      behavior).
    - No listing or scan of the open folder starts while the clear runs.
    - After a clear: the open folder (if any) is listed again so pass 1 and
      pass 2 refill its rows, the view staying on a consistent set of
      files and thumbnails; the tree's cached counts are re-read from the
      now emptied `folder_counts` table; the size line is refreshed. A
      failure is shown on the settings' status line and nothing is
      re-listed.
    - The size line is also refreshed on every open of the modal and when a
      scan ends while the modal is open.
    - Without an index (`self.index` is `Err`) the page shows `0 B` and the
      button clears nothing, as the Tauri commands do.
    - Unit tests cover the new pure parts (the clear flow's states: idle,
      waiting for the scan, asking, running, the discard on close; the page
      and dialog texts; the on-disk size over the three files with a
      missing one counting zero) and the `TABS` order / titles tests are
      updated.
    - `CLAUDE.md`'s layout description names the new or changed native
      modules; `docs/agents/native-app.md`'s settings section (the page
      list, "no Cache, MCP or Debug") and folder tree section (the
      `folder_counts` rules: what a clear does to the cached counts) are
      updated; `docs/plans/20261109-gpui-port/plan.md` Step 9's "Still open
      of Step 9's settings: the Cache tab, ..." note no longer lists the
      Cache tab; `todo.md` gains a real-device check item for what nobody
      has seen in the running app (the dialog over the settings, the held
      clear, the refresh, the size shrinking).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Do not change `crates/app` (frozen). Read it as the reference:
      `clear_index` / `index_size` / `on_disk_bytes` in
      `crates/app/src/commands.rs`, the `#cache` panel in
      `crates/app/ui/index.html`, `clearGate` / `clearCache` /
      `setScanRunning` / `close` in `crates/app/ui/src/settings.ts`, and the
      `index-cleared` listener in `crates/app/ui/src/main.ts`. Reuse its
      texts: panel paragraph "Clearing drops every cached thumbnail, so the
      next open of a folder scans it again; your judgments are kept, because
      they live in the sidecars."; dialog title "Clear the index cache?",
      message "Every cached thumbnail and the cached metadata of every
      folder are removed. Judgments are kept. The next open of a folder
      scans it again.", buttons `Clear` / `Cancel`.
    - Files expected to change: `crates/native/src/settingsmodal.rs`
      (`Tab::Cache`, `TABS`, its doc comment "less the Cache, MCP and Debug
      ones"), `crates/native/src/settingsview.rs` (the page, a new `Change`
      variant for the press, setters the window calls for the size, the
      waiting note and the in-flight state, the discard on `close`),
      `crates/native/src/main.rs` (the `Change` handler in the
      `subscribe_in` callback, the clear job, the drain of a held clear in
      `finished`, the index path), a new pure-model module (for example
      `crates/native/src/cache.rs`: flow, texts, `on_disk_bytes`), plus the
      docs above. No backend change is expected: `Index::clear`,
      `index::with_suffix`, `text::format_bytes` and `SIZE_BASE` already
      exist. `on_disk_bytes` lives in the frozen `crates/app`, so the
      native app needs its own copy (native side, to avoid compiling a
      backend change twice for one caller).
    - The index path is not kept today: `open_index` in `main.rs` computes
      it from `datadir::index_path` and drops it. Carry it into the view
      (it must honor `--index <path>`). Read the size on
      `background_spawn`, never on the UI thread.
    - Busy rule: the view's existing one is
      `self.resync.busy() || self.gate.in_flight()` (see `step_trash`,
      the trash run). The native app has no third (embedding) pass, so
      there is no `stop_embedding` counterpart. Hold the clear in a slot of
      its own, not in the view's `gate` (`IdleGate<HeldOp>`): a request
      there would replace a held trash / rename, and the Tauri app also
      uses a separate `clearGate`. Drain it where the view becomes idle
      (`finished`, after the `gate.drain()` arms that start a run return
      early, i.e. only when nothing else started).
    - Order as in the Tauri app: wait for the scan first, then confirm,
      then re-check the busy rule before running (a re-listing may have
      started while the dialog was up).
    - While the clear runs, keep listings out the way a trash run does:
      `self.gate.enter_in_flight()` + `self.resync.hold()` before the
      background job, `leave_in_flight()` after it, then
      `self.resync.open(); self.relist(cx)` (the shape of
      `reset_sidecars` and of the trash run's end). `relist` already
      returns while `gate.in_flight()`.
    - The background job, in this order: `stores.drain(sidecar::DRAIN_TIMEOUT)`,
      `writer.flush(sidecar::DRAIN_TIMEOUT)` (as `start_sidecars` in
      `folderops.rs` does), then `index::lock(&index).clear()`. Run it one
      at a time with the sidecar format / label names switches by taking
      the `format_switch` mutex `reset_sidecars` takes, so a switch's
      `sidecar::switch_to` and the clear do not interleave. The launch's
      eviction thread holds the same index mutex, so the clear simply waits
      for it.
    - After a successful clear, on the UI thread: follow `reset_sidecars`
      (`touched.clear()`, `undo.drop_judgments()`, `step_note = None`,
      `resync.open()`, `relist`), call `read_folder_counts(cx)` (it bumps
      `count_reads`, so a read started before the clear is dropped, and
      `tree.set_cached_counts` replaces the whole cache), and refresh the
      size. With no folder open, only the counts and the size.
    - Confirmation dialog (decided with the user: a gpui-component `Dialog`):
      the existing native mechanism, a `Dialog` in `Root`'s layer through
      `modal.rs` with the `dialog.rs` model; `docs/agents/native-app.md`
      records that such a dialog paints above the settings layer with its
      `backdrop`. Note that `start_sidecars` and the `OPEN_KEY` arm of
      `key_down` deliberately keep folder dialogs and the settings apart;
      this dialog is the first one opened from inside the settings.
    - Verify during implementation:
      - Key routing with the dialog over the settings: Escape / Enter /
        Space must act on the dialog only (Escape must not close the
        settings underneath), and the focus must return into the settings
        box when the dialog closes (the settings box has its own focus
        trap and an `on_key_down` that stops every key).
      - That `VACUUM` succeeds with the read-only `reader` connection open
        (thumbnail reads on `thumbs.rs` lock the reader for short reads); a
        failed WAL truncate is only logged by `Index::clear`.
      - After the re-listing, strip cells whose thumbnail was not loaded
        behave as in a cold open (asked again as pass 1 progresses), and
        the meta pane / filter `Facts` come back as the scan refills them.
      - Which in-memory state mirrors tables the clear empties: the
        `viewed` set and `last_viewed` (`views`, `folders.last_viewed`),
        and writes still queued in `CountWrites`, `viewed.rs`'s `Batches`
        and `Latest`. A count or view write landing after the clear only
        re-adds a true row; decide whether anything needs resetting and
        record it in `learnings.md`.
      - `--auto` runs: nothing here reads or writes a setting, so no
        special case is expected; confirm.

## Decisions (approved by the user)

- Confirmation UI: a gpui-component `Dialog` through `modal.rs` /
  `dialog.rs` (not GPUI's OS-native `window.prompt`, not an inline two-step
  confirm).
- Re-list with carry (`relist`), not a fresh open: the clear deletes
  `folders.last_viewed`, so a fresh open would jump to the first file.
  Loaded thumbnails stay on screen until pass 1 rewrites their rows.
- The judgment undo history is dropped after a clear, as the Tauri reopen
  and the native format switch do.
- The counts of folders listed in this session are not re-pushed to
  `folder_counts` after the clear; the table really ends up empty apart from
  the open folder's re-listing.
- `on_disk_bytes` is copied into the native app rather than moved to the
  backend.

## Trade-offs and risks

- **Held clear and the gate's held operation.** With both a held trash /
  rename and a held clear at a scan's end, the gate's operation runs first
  and the clear waits for the next idle point. It can therefore wait through
  a second scan; closing the modal drops it.
- **User documentation.** `docs/humans/usage.md`'s Clear Cache paragraph
  describes the same behavior and names `Handpic > Settings...`; it does not
  distinguish the two apps, so the step leaves it (and its `.ja.md`) alone.
- **Risk: unverified GUI.** `ci:local` cannot see the dialog, the held
  clear or the refresh; these go to `todo.md` as a real-device check.

## Progress
