# Results: Windows real-device checks, batch 8 (2026-11-22)

The performance overlay on `d`, the momentary grayscale preview on a held `g`,
and the held-key preview and crop, on Windows 11 on 2026-11-22.

## Setup

- Builds: `crates/app` at `19bacdcf`, `--features screenshot`, both the
  release build (the performance overlay's boxes, which ask for one) and the
  debug build (everything else).
- Material: copies under `D:\Photos\tests\20261122-heldkey-gray-perf\`:
  `raf\` (25 Fujifilm X-series RAF files, plus the sidecars copied with them)
  and `arw\` (39 Sony ILCE-7 series ARW files, three of them deliberately
  broken samples whose preview fails: `IFD entry out of range`, `IFD offset out
  of range`, `preview out of range`). Nothing else under `D:\Photos\` was opened.
- Every launch had a fresh `--index`, `--settings` and `--log` under
  `run\<name>\` (r1 to r6), `--mcp-port 41971`, and `{"mcpEnabled": true}` through
  `--import-settings`; stdout was redirected to a file, where the `page`,
  `zoom` and `summary` lines go. Window 1600 x 1000 client, Neutral dark.
- Input: synthetic `keybd_event` / `mouse_event`, after checking that
  `GetForegroundWindow` was the instance's window (found by PID), under the
  shared `D:\Photos\tests\.ui-lock`. A held key was repeated key-downs every
  33 ms with one key-up. Evidence: window captures, bursts of captures every
  15 to 100 ms, pixel scans of the captures, and the stdout timing lines.
- Runs: r1 release, `raf\` then `arw\`; r2 debug, `raf\` then `arw\`; r3 debug
  `--no-prefetch`, `arw\`; r4 debug `--no-prefetch`, `raf\`; r5 debug, `arw\`; r6 debug, `arw\` (one short hold).
  Every instance was quit with `q`.

## Incidents

- After a click on a tree row the tree holds the keyboard, so a following `d`
  went to the tree's type-ahead (the cursor jumped to a `d…` folder; nothing
  opened) instead of toggling the overlay. `Escape` handed the keyboard back.
  Expected behaviour (`treekeys.rs`), not a finding.
- `SetCursorPos` followed by a button press did not reach GPUI in r5 (no cell
  selected, no menu), although `WindowFromPoint` named the instance. A few
  relative `mouse_event` moves before the press fixed it every time.
- One capture taken without the UI lock showed another session's window over
  the instance; it was discarded and retaken under the lock.

## `### App: real-device check of the performance overlay on `d``

1. Band, colors, grid, motion, legend (release build, `raf\`): PASS.
   - Pixel column scan: the band spans y 681–800 of the client area, 120 px,
     ending at the preview canvas's bottom edge (801 is the strip's border).
   - Over a `#8C0600` photo pixel the band reads `#281414`, which is
     `#171717` at 0.85 alpha over it.
   - Grid lines at y 701, 749 and 797 (100, 50, 0 %, 48 px apart), under a
     20 px legend row.
   - The legend (11 px) shows `100%  60 s`, the four line swatches (Handpic CPU
     solid blue, System CPU dashed blue, Handpic disk solid green, System disk
     dashed green), `Latency` and the eight `page` stage names fading from red
     to dark red, and `10 MB/s  500 ms` at the right.
   - Motion: 40 captures of the band at 100 ms. The plot changed in pairs of
     consecutive captures once every 500 ms (e.g. at 200/308, 603/712,
     1102/1211, 1603/1713 ms) and was still in between; the newest point is at
     the right edge.
2. A bar per switch, and the input still works (release build): PASS, with a
   note.
   - Three `→` presses and one strip click gave four bars, one per switch, at
     the bottom right of the plot (columns about 30 px apart, 1.5 s). A folder
     open from the tree (`arw\`) added one more. Each key, click and open did
     what it does without the overlay (the `page` lines, the strip cursor, the
     opened folder).
   - Note: the bars are 1–3 px tall and mostly under the Handpic disk line at
     0 %, because the switches took 12–20 ms (`keypressToPixels`, prefetch
     hits) against the 500 ms latency axis. Seen only on 4x zooms and a pixel
     search for red; at normal size they are next to invisible.
3. `d` again hides it, reopening starts empty: PASS. The band went away
   entirely; 250 ms after the next `d` it was back with no plotted lines, and
   1.75 s later only the newest few points at the right edge. Observation:
   the 250 ms capture's legend had no `System disk` entry; it was there at
   1.75 s.
4. Overlay off, `→` held through a folder (release build, `arw\`): PASS on the
   format. 60 key-downs at 33 ms printed one `page` line per page turned (57,
   all fields of the Timings description, `source=prefetch` or `read`) and the
   three broken files' `error` lines; `q` printed `summary n=61` with the nine
   stage rows (`keypressToPixels p50=11.18ms p95=23.65ms`). No build from before
   the overlay was run to compare against; the lines carry no field the overlay
   adds.

## `### App: real-device Windows check of the momentary grayscale preview`

Debug build. Gray is judged by the mean RGB chroma (max − min) over a grid of
the photo area: 0.0 (max 0) means no colored pixel at all.

1. Fit view and 1:1: PASS.
   - Fit view (`raf\`, X-A1): chroma 0.0 in every capture while held, 59.7 on
     release.
   - 1:1 (`arw\`, an ILCE-7CM2 file with a full-size JPEG, crop read about
     55 ms): with `g` down, a burst from the `z` key-down caught the scaled
     preview placeholder (blurred, frames at 39–70 ms) and then the crop
     (from 87 ms), both chroma 0.0; color returned on release.
2. Overlays keep their colors, the rest is unaffected: PASS. With `f`, `e` and
   `d` on, held `g` left the photo gray and the peaking edges red, the face box
   and label cyan, the mesh purple, the landmark dots, the AF mark and the
   performance graph's lines in color. The tree, the meta pane and the strip
   were pixel-identical with and without `g`. On a broken file the
   `The preview of this file could not be shown.` text, the red error text in
   the meta pane and strip cell, and the status row were pixel-identical too.
3. Auto-repeat: PASS. 60 key-downs at 33 ms, 30 captures over 2.2 s: chroma
   0.0 in every one, no colored frame; color on release.
4. `ctrl+g`: PASS. Rebound in the settings (Keyboard Shortcuts > Preview >
   Grayscale: `+`, Ctrl+G, then `×` on `G`), closed. Ctrl+G held (10 G
   key-downs) grayed the photo; releasing Ctrl first restored color with `g`
   still down, and releasing `g` first restored it with Ctrl still down. A
   plain `g` no longer grayed. Reset put `G` back afterwards. (Earlier, `×` on
   `G` while it was the only key did not remove it; not investigated.)
5. Alt+Tab: PASS. Held `g`, Alt+Tab took the foreground elsewhere, then back
   with `g` still down: color (chroma 20.7), and still color after the key-up.
6. Settings and right-click menu: PASS.
   - Held `g`, `ctrl+,` (the settings layer covered the photo), released `g`,
     `Escape`: color.
   - Held `g`, right-click on a strip cell: the menu opened and the photo
     stayed gray while `g` was still down (the opening did not end the hold);
     releasing `g` with the menu open turned it to color, and it stayed so
     after `Escape`.
7. Keys while `g` is held: PASS. With `g` down: `→` twice paged (two `page`
   lines, gray), `z` zoomed (one `zoom` line, gray crop), `z` unzoomed, `f`
   drew the AF mark, `e` the peaking (one `peaking` line), `d` the band, all in
   their colors over the gray photo; the key-up restored color with all three
   overlays still on.

## `### App: real-device checks of the held-key preview and crop (gpui-held-key-preview)`

Debug build.

1. Fit view: PASS.
   - Default settings, `raf\`, `f` on, 45 key-downs at 33 ms with a capture every
     third: every capture showed a different frame (15 distinct), 28 `page`
     lines, some with `wait` 9–16 ms (a request waiting on the read in flight)
     and files skipped (e.g. `X-E1_DSCF6529new.RAF` the first time round).
   - `--no-prefetch`, `raf\`, the same hold: 18 `page` lines for 45 key-downs,
     most with `wait` 4–45 ms, the frames changing during the hold.
   - Default settings, `arw\` (faces), a capture per key-down: every page was a
     prefetch hit, each frame showing its own file's face boxes and AF mark;
     none showed another file's marks.
   - On release: the current file (19/25, `X-E3_DSCF6535.RAF`) with its AF
     mark, its meta pane, and its `page` line the last one printed.
   - Stale frames carry no `f` overlay: in the `--no-prefetch` `raf\` hold the
     frame at 1400 ms shows `X-E3_DSCF0034.RAF`, which has no `page` line (a
     stale result drawn on the way), and is drawn with no AF mark, while the
     current-file frames (e.g. 1104 ms) show theirs.
   - A short hold ending on a face file (r6, `arw\`, `f` on, three key-downs
     at 33 ms, then quit with `q`): the release showed
     `ILCE-7CM2_DSC00593.ARW` with its face box and label, the face mesh, the
     landmark dots and the AF mark, its meta pane, and its `page` line last.
2. 1:1 view: PASS (`arw\`, crop read 51–62 ms by the `zoom` lines, longer
   than the 33 ms repeat).
   - 30 key-downs while zoomed, a capture per key-down: the crops changed
     during the hold, some held over two captures; every frame showed one
     file's crop or placeholder whole, never a crop over another file's
     preview.
   - Files skipped: in a 12 key-down hold while zoomed, the crops went from
     `ILCE-7CM2_DSC00606.ARW` straight to `ILCE-7CM2_DSC02429.ARW`, with no
     crop of `ILCE-7CM2_DSC00621.ARW` drawn, and from the poles of
     `ILCE-7CM2_DSC02431.ARW` straight to the release's file, 13th in the
     folder.
   - On release after 12 key-downs: the current file's crop.
   - A single `→` while zoomed: the old crop at 37 ms, the new crop from 85 ms,
     no black or placeholder frame between (20 captures at 50 ms).
   - `z` off / on on the same file: the crop back with no new `zoom` line;
     `z` on a new file printed one `zoom` line.
