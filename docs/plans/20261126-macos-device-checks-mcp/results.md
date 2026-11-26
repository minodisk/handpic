# Results: macOS real-device checks through MCP and the shell (2026-11-26)

Eleven pending macOS checks of `todo.md`, driven only by the MCP companion's
tools and the shell: no key press, no right-click, no drag, no dialog button.
The tools used: `screenshot`, `get_view`, `get_status`, `set_settings`,
`tree_folder`, `rename_folder`, `rename_by_capture_date`, `set_judgment`,
`trash_photos`, `undo`, `redo`, `reject_rest`, `navigate`, `set_filter`,
`set_sort`, `select_photos` and `open_folder`. Five checks passed whole, one
passed in its first half, five passed in part, and nothing failed.

## Setup

- Machine: Darwin 27.2.0, arm64, a 5120x1440 display, Dock autohide on.
- Build: the worktree at `020e89c7` (main). The debug app through
  `mise run app:dev:screenshot` (`cargo run --features screenshot`); the
  release through `mise run app:package:macos`, version 2.1.0, so the files
  are `Handpic-2.1.0-macos-arm64.dmg` / `.tar.gz` (not `0.0.0`).
- Every launch had its own `--settings`, `--index` and `--log` in a scratch
  folder and `--mcp-port 41999`, the store seeded with `mcpEnabled`,
  `mcpAllowDestructive` and `sidecarFormat` both.
- Test folder: APFS clones of a 117-ARW Sony ILCE-7M5 folder (19 `.dop`,
  4 `.xmp`; bursts at 0.1 s, among them a 16-frame one, `_DSC0002` to
  `_DSC0017`).

## App: real-device checks of the gpui-kit umbrella crate (native-gpui-kit), macOS box: PASS

The debug window drew its text in the title bar, the tree, the no-folder
message, the status row, the meta pane and the `Sort ▾` / `Filter ▾`
buttons, with no shader error on stderr or in the log.

Not done: the popover menus were not opened.

## App: real-device checks of the Appearance tab (mode, shipped themes) (native-themes), macOS box: PARTIAL

- `set_settings` mode Light (Neutral): the panes, the strip, the title bar
  and the meta pane turned light and readable; the stars and the burst
  badges were readable.
- Mode System, light Catppuccin Latte, dark Tokyo Night: flipping the OS
  appearance (System Events' dark mode) swapped the app between them live,
  in both directions.
- Mode Dark: stayed Tokyo Night with the OS in light.
- Restart: the theme was restored (log `theme=dark`, the first frame Tokyo
  Night).

Not done: the dialogs and the settings box under each theme (no MCP tool
opens the settings), and the pickers' keys.

## App: real-device check of the folder tree's file counts on collapsed rows, macOS box: PARTIAL (most passed)

- Expanded then collapsed never-opened folders `a` (3), `b` (2), a symlink
  `ln` -> `linktarget` (4) and `del` (2): the counts were in
  `folder_counts` and on the pills.
- A RAW added to `b` outside the app, then `tree_folder` refresh: 3.
- `rename_folder` `a` -> `a2` (MCP confirm): the row was carried.
- `del` deleted outside the app, its parent refreshed.
- Restart: collapsed `a2` 3, `b` 3, `ln` 4, the opened folder 117, `c`
  (never listed) none; `del` re-created (1 RAW) showed none until listed.
- `b` expanded: pill 3, the same look as collapsed.
- A long name with a pill: the name truncated with an ellipsis, the pill
  whole (pane at 480 px; at 220 px the deep rows' names shrank to an
  ellipsis with the pill whole).
- `rename_by_capture_date` split (3 files, one re-dated to 09-14):
  `2026-09-13` (2) and `2026-09-14` (1) shown collapsed; the original
  removed as empty.

Not done: an index upgraded from before the cache table, a network volume, a
folder renamed outside the app, and a split whose undated files keep the
original.

## App: the macOS / Linux manual checks for Move to Trash are still open, undo of a rejected file: PASS

- `_DSC0050` rated 2 and rejected: the `.xmp` held `xmpDM:good="False"`, the
  `.dop` `ShouldProcess = 1`. The `trash_photos` preview read
  `Move 1 file to the Trash?` / `Move to Trash`; the confirmed run moved the
  ARW, its `.xmp` and its `.ARW.dop` to `~/.Trash`, the strip showed
  `_DSC0051`, count 116. `undo`: `Restored 1 file from the Trash`, all three
  back, the reject flag and the 2 stars kept in the index and both sidecars.
- `_DSC0060` rejected and moved to the Trash, then a placeholder
  `_DSC0060.ARW` written at the original path, then `undo`:
  `Restored 0 files from the Trash, 1 failed`; the errors
  `_DSC0060.ARW: could not restore from the Trash: already exists at the original location`
  and the two sidecars `left in the Trash: its RAW could not be restored`;
  the placeholder untouched.

## App: the macOS / Linux manual checks for Move to Trash are still open, the flow: PARTIAL

Through MCP (not a right-click): the dialog's text, the RAW with its `.xmp`
and `.ARW.dop` moved to the Trash, the neighbour shown.

Not done: the dialog's `Cancel` button, Finder's "Put Back".

## App: real-device checks of the burst bands, badges, burst keys and reject the rest, rejectRest box: PASS

- The 16-frame burst `_DSC0002` to `_DSC0017` (picks, stars, all seven
  labels), the current file `_DSC0004`, `_DSC0008` already rejected,
  `_DSC0003` (5 stars) hidden by a stars filter 0 to 4. `reject_rest`
  rejected the 14 others, the hidden one included; each `.xmp` got
  `good=False` and each `.dop` `ShouldProcess=1` with the rating and
  `Label` / `ColorLabel` unchanged; the current file stayed `_DSC0004`, with
  no advance.
- `undo`: `Undid 14 files`, the current file kept; `redo`: `Redid 14 files`.
- The two-frame burst `_DSC0062` / `_DSC0063` from `_DSC0062`: `_DSC0063`
  rejected; the undo moved the current file to `_DSC0063`
  (`Undid _DSC0063.ARW`).
- The singleton `_DSC0001`: nothing. A JPEG folder: refused with
  `culling does not apply to a JPEG folder`.
- `get_view`'s `burst` listed the members with their sharpness and
  judgments, `visible: false` on the hidden one.

## App: real-device checks of the burst bands, badges, burst keys and reject the rest, band / badges / keys box: PARTIAL

- Seen: the band behind the burst's cells with rounded ends; the `n/size`
  badges at the bottom-right, dimmed on the rejected member; one band ends at
  `16/16` and a new burst starts at `1/3`; the selected cells' outline
  distinct from the band; a JPEG folder the same (`1/3` to `3/3`).
- MCP `navigate` `burst_next` / `burst_previous` and `burst_frame_next` /
  `burst_frame_previous`: right in name order, under a stars filter with a
  rating sort, and in the JPEG folder.

Not done: a Leica whole-second folder, a failed cell next to a band, the
keys while the tree holds the keyboard (no key was pressed at all; the tools
and the keys share the view's `perform`).

## Core: real-device check of the scan priority's macOS QoS branch: PASS

- A release binary (the bundle's executable run directly, for stdout), a
  cold index, 1755 ARW (15 clones of the 117).
- Pass 2: 1755 files in 104026 ms (about 17 per second), `threads=1`.
  Pass 3: 15 files in 2209 ms.
- MCP `navigate` next every 0.5 s or so, `keypressToPixels`:

  | When | Pages | p50 / p95 / max (ms) | Source |
  |---|---|---|---|
  | During pass 1 | 14 | 24.9 / 40.1 / not noted (the first page 271.7) | `read` |
  | During pass 2 | 106 | 16.3 / 35.5 / 41.3 | `read` |
  | After the scan | 120 | 12.2 / 19.5 / 33.9 | `prefetch` |

The rows differ in the page's source as well as the scan's priority: during the passes every page was its own read, after the scan the prefetch ring served it. The gap between the rows is therefore not the cost of the QoS; the like-for-like comparison is the Windows figure below.

- The Windows reference: 18.4 / 25.0 ms (ARW, p50 / p95) while the second
  pass runs, from `docs/humans/performance.md` (the table after "While the
  folder scan's second pass (faces, focus, eyes) runs, paging costs nothing
  measurable", 2026-11-13; the per-run numbers in
  `docs/plans/_archived/20261110-gpui-focus-marks/results.md`).
- No `ran at normal priority` line in the log.
- A debug build took about 5 s per file in pass 2, not usable for this check.

## App: real-device checks of the app icon on Windows, macOS and Linux, macOS box: PASS

`mise run app:package:macos` (which runs `app:bundle:macos`), then the bundle
launched: `NSRunningApplication.icon` (what the Dock draws, activation policy
regular) and `NSWorkspace`'s `iconForFile` (Finder) both gave the Handpic
film-strip icon. The Dock itself was autohidden, so not screen-captured.

## App: real-device checks of the capital executable name (`Handpic.exe`, `Contents/MacOS/Handpic`), macOS box: launch part PASS

The DMG passed `hdiutil verify` (VALID). Its app, copied to `~/Applications`
(not `/Applications`, which holds the user's 2.0.1), held
`Contents/MacOS/Handpic`; `codesign --verify --strict` was valid on disk and
satisfied its designated requirement (ad hoc, identifier
`dev.minodisk.handpic`); it launched with the test arguments.

Not done: the update from a 2.0.0 bundle to the new one (it needs a test
feed).

## App: real-device checks of the packages the Package workflow builds (gpui-packaging), macOS box: PARTIAL

The DMG was built (the 2.1.0 name), opened, and the app copied out of it
started. Gatekeeper's first-launch prompt cannot be reproduced: a local build
carries no `com.apple.quarantine` attribute (only `com.apple.provenance`),
and `spctl --assess` rejects the ad hoc signed app.

## Side findings

- The `screenshot` tool returns the last drawn frame. When the window is
  fully covered by other windows the frame does not advance (a tree expand
  did not show until the window was brought to the front).
- Every page during the passes was `source=read`, every page after them
  `source=prefetch`.
