# Windows real-device checks of the prefetch ring and of Clear Cache (2026-11-22)

Setup: Windows 11, the release build of `crates/app` (`cargo build --release`, NASM on `PATH`, origin/main `19bacdcf`), window 1616x1008, synthetic keys and mouse (`keybd_event`, `mouse_event`) sent only after `GetForegroundWindow` was the test instance's window (by PID), window captures and burst captures (one full-window capture every 50 to 70 ms), the `page` lines read from the redirected stdout. MCP on port 41981 seeded with `{"mcpEnabled": true}` through `--import-settings`, `mcpAllowDestructive` off. Fresh `--settings` and `--log` per launch; a fresh `--index` per check, the Clear Cache check reusing one test index (`cacheX.sqlite`) across its two launches. The real index and settings store were not touched. Material: copies under `D:\Photos\tests\20261122-prefetch-cache\`: `ring-a\` (20 ARWs of `2026-06-05-focus-sample`), `ring-b\` (20 ARWs of `2026-09-19-focus-sample`), `cache\c1\`, `cache\c2\`, `cache\c3\` (3, 5 and 7 ARWs of `2026-08-22-focus-sample`).

Incidents:
- After a click on a tree row the tree holds the keyboard, so a later `q` went to the tree and the instance stayed up; `Escape` then `q` quit it. A second instance launched meanwhile could not bind the MCP port (`os error 10048` in its log) and was quit and relaunched.
- The release build's launch update check ran (`update: offered Some("1.10.0")`, the running version being `0.0.0`); it only offered the release page and changed nothing.

## The prefetch ring on a page turn, a folder switch and a filter change

### Box 1: `→` after the scan
- `ring-a\`, fresh index, MCP `wait_for_idle` until both passes ended, then `→` from `_DSC0446.ARW` (`1 / 20`). The next capture showed `_DSC0500.ARW` (`2 / 20`) in the preview, the meta pane and the outlined cell. Its `page` line: `keypressToPixels=8.02ms ... source=prefetch file=_DSC0500.ARW`. Later presses in both folders read `source=prefetch` too (`_DSC1821` 9.04 ms, `_DSC1828` 7.49 ms). PASS.

### Box 2: another folder opened from the tree
- From `_DSC0500.ARW` of `ring-a\` (its neighbours in the ring), `ring-b\` opened through MCP `open_folder` (the tree's `open_from_tree` path) during a 70-frame burst: 15 frames of `_DSC0500.ARW`, then from 830 ms `ring-b\`'s first file `_DSC1807.ARW` (title `ring-b`, `1 / 20`, the strip's cells still gray, `scanning 0/20`), with no frame in between and no frame of another `ring-a\` image.
- Back the other way by a real click on the tree's `ring-a` row, after paging `ring-b\` to `_DSC1828.ARW` (its neighbours in the ring): one frame of `_DSC1828.ARW`, then from 117 ms `ring-a\`'s resumed `_DSC0500.ARW` on every frame to 4.1 s, nothing in between.
- An open prints no `page` line (only a key press is timed). PASS.

### Box 3: a filter that hides the neighbour, then `→`
- In `ring-a\` at `_DSC0500.ARW` (`2 / 20`, next `_DSC0521.ARW`, held in the ring), `_DSC0500.ARW` and `_DSC0522.ARW` were flagged pick and the filter set to `Picked` through MCP `set_judgment` / `set_filter` (the menu's `apply_menu` path): `2 / 2 (20)`, `_DSC0521.ARW` hidden.
- `→` during a 40-frame burst: from the first frame (48 ms) `_DSC0522.ARW` on every frame, no other image. Its `page` line: `keypressToPixels=6.65ms ... source=prefetch file=_DSC0522.ARW`. PASS.

## The Cache page and Clear Cache (native-clear-cache)

- Launch 1 (fresh `cacheX.sqlite`) opened `cache\c1\`, then `c2\` and `c3\` through MCP `open_folder`; the tree showed `c1 3`, `c2 5`, `c3 7`. Quit with `q` (the quit handlers' drains logged).
- Launch 2 on the same index opened `ring-a\`, and `cache\` was expanded through MCP `tree_folder`. Its rows `c1`, `c2`, `c3`, not listed in this session, showed the cached counts `3`, `5`, `7`; their sibling `ring-b` (never in this index) showed none.
- `ctrl+,` opened the settings on Sidecar; the `Cache` entry showed `Index cache: 1.7 MB`. `Clear Cache` opened `Clear the index cache?`; `Clear` ran it (log: `cache: cleared in 20ms`, `index cleared: EvictSummary { folders: 4, rows: 45, vacuumed: true }`) and the size line read `856.5 KB`.
- After `Escape`, the tree showed `c1`, `c2`, `c3` with no count; the open `ring-a` kept its listed `20`. PASS.
- The box names `mise run app:dev`; the run used the release build, the same code.
