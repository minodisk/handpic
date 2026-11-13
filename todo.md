# todo

## Cross-cutting / other

### App: real-device check of the strip menu's reveal and copy items

`strip-file-reveal-copy` (docs/plans/_archived/20261018-strip-file-reveal-copy/plan.md) added three items to the filmstrip's right-click menu, after `Select All`: the platform's reveal label (`Reveal in Finder` / `Reveal in File Explorer` / `Open Containing Folder`, calling `reveal_folder` on the focused file), `Copy Path` and `Copy File Name`. On a multi-selection the copy items write every targeted file, one per line with no trailing newline. CI and the `context.test.ts` unit tests cover the menu composition, including the view-only menu. The OS file manager reveal and the clipboard contents were never exercised on a real machine: the step was ticked on the automated criteria only.

Files: `crates/app/ui/src/main.ts` (`openContextMenu`), `crates/app/ui/src/context.ts` (`contextMenuGroups`), `crates/app/src/folders.rs` (`reveal_folder`, `REVEAL_LABEL`).

#### TODO

- [ ] On Windows, open a RAW folder, right-click a file and choose `Reveal in File Explorer`. File Explorer should open the folder with that file selected. Repeat in a view-only JPEG folder. If a file directly under a drive root fails with `NoParent`, check that the error lands on the status line.
- [ ] On Windows, select three files, right-click one of them and choose `Copy Path`, then paste into an editor. Expect three full paths, one per line, with no trailing newline. Then choose `Copy File Name` and paste. Expect three names with extensions, one per line.
- [ ] On macOS and Linux, repeat the reveal check (`Reveal in Finder` / `Open Containing Folder` should select the file in the file manager) and the multi-file copy check.

### App: real-device check of the `Face focus` filter (Good / Neutral / Low)

#### Background

The `face-focus-tiers` feature (Step 1) re-cut the filter's `AF eye` section into `Face focus` with `Good` / `Neutral` / `Low`. `afEyeState` now feeds the menu, the strip icon and the `f` mark, and `FILTER_CANDIDATES` in `commands.rs` drops a saved `Bad` / `Unknown`. CI and the unit tests (`focus.test.ts`, `filter.test.ts`, the `filter_setting_*` tests) cover the state mapping, the filter logic and the persistence whitelist. Nobody has run the built app on a real folder, and the Step 1 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261107-face-focus-tiers/plan.md`.

Files: `crates/app/ui/src/focus.ts`, `crates/app/ui/src/filter.ts`, `crates/app/ui/src/main.ts`, `crates/app/src/commands.rs`.

#### TODO

- [ ] On Windows, use a folder with faced, face-less and JPEG frames under `D:\Photos\tests\<date>-<topic>\`. Open it for its first scan with `Low` checked. Expected: the list grows as pass 2 reaches files.
- [ ] On Windows, after `faces-done`, step through the shown files. Expected: every one reads `AF eye in focus` under 77% in the meta pane.
- [ ] On Windows, check `Neutral` alone. Expected: the face-less frames and the JPEGs show.
- [ ] On Windows, launch with a filter saved before this change with `Bad` checked. Expected: the `Face focus` section opens unchecked and nothing is hidden.

### App: real-device check of the `?` beside `Face focus`

#### Background

The `face-focus-tiers` feature (Step 2) added a `?` to the `Face focus` row that shows each item's definition and thresholds. `focus.ts` builds the text from the constants, and vitest checks it. A Rust test pins the 77% figure. Nobody has opened the menu in the built app, so the menu staying open, the wrapping and the close behaviour were never exercised. Learnings estimate the open block at about 24 lines at the menu's 180px minimum width, from the CSS only. The Step 2 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261107-face-focus-tiers/plan.md`.

Files: `crates/app/ui/src/main.ts`, `crates/app/ui/index.html`, `crates/app/ui/style.css`.

#### TODO

- [ ] On Windows, at the default window size, open the filter menu and click the `?` beside `Face focus`. Expected: the menu stays open, the definitions appear under `Low`, no item's check changes, and a second click hides them.
- [ ] On Windows, with the definitions shown, look at the menu's width and scrolling. Expected: the text wraps at the menu's width and no horizontal scrollbar appears.
- [ ] On Windows, with the definitions shown, press `Escape`, then reopen the menu. Expected: the menu closes and reopens with the definitions hidden. Clicking outside the menu does the same.

### App: real-device check of the Face focus Good / Low icons and mark colors

The `af-eye-good-ok-bad` feature gave each judged state its own icon and color on the strip, the filter menu and the `f` focus mark; the `af-eye-good-bad-cascade` plan (Step 1, `docs/plans/_archived/20261010-af-eye-good-bad-cascade/plan.md`) merged `OK` into `Bad`, and the `face-focus-tiers` plan (`docs/plans/_archived/20261107-face-focus-tiers/plan.md`) then renamed the section `Face focus` with `Good` / `Neutral` / `Low`. A candidate face that is not good is now `Neutral` (white, no icon), and `Low` (gray frowning `scan-face`: Lucide's `scan-face` with its mouth turned down, by the `frown-low-icon` plan, `docs/plans/_archived/20261108-frown-low-icon/plan.md`) is a face whose in-focus probability is under about 77%. CI and the unit tests cover the state -> icon / color mapping (`stripState`, `FOCUS_MARK_ICONS`, `FOCUS_MARK_COLORS` in `focus.test.ts`) and the filter logic. Nobody has looked at the rendered result in the running app, including whether the gray `#999` reads on photos. Plan: `docs/plans/_archived/20261010-af-eye-good-ok-bad/plan.md`. The `frown-low-icon` plan's Step 1 was ticked on the unit test of `FOCUS_MARK_ICONS` and `mise run ci` only; the rendered 12 px icon was never looked at.

Files: `crates/app/ui/src/focus.ts`, `crates/app/ui/src/strip.ts`, `crates/app/ui/src/icons.ts`, `crates/app/ui/index.html`, `CLAUDE.md`.

#### TODO

- [ ] On any desktop platform, open a folder with faced AF frames and wait for `analyzing N / M` to finish. Check that the strip shows a bright green smiling `scan-face` on good frames, a gray frowning `scan-face` (the same frame and eyes, the mouth turned down) on low frames (a judged face whose in-focus probability is under about 77%) and nothing on neutral ones (no face, a candidate face that is not good, a JPEG). Check that the filter menu's `Face focus` items (`Good` / `Neutral` / `Low`) show the same icons (`Neutral` an empty aligned slot). Check that the `f` mark is bright green for good, gray for low and white for neutral, with the gray readable on photos. Check that at 12 px the Low frown is told apart from the Good smile on the strip cell and on the filter menu's `Low` item; if it is not, open a follow-up that adjusts the mouth path (`M8 16s1.5-2 4-2 4 2 4 2`) or the stroke width.
- [ ] Update the other `CLAUDE.md` Layout sentences left stale by `docs/plans/_archived/20261010-af-eye-good-bad-cascade/`: add "else the largest confident face on the whole preview when there is no trusted AF point" to the `run_faces_scan`, `eyes_of` and `src/candidate.rs` sentences; make the `src/focus.ts` sentence name the `afEyeState` helper (Good / Neutral / Low) and drop the sharpness displays; and make the `src/sharpness.rs` sentence say the score now feeds the MCP companion only. Done when a PR edits those sentences.

### App: real-device check of the Face focus judgment on frames with no AF point

#### Background

The `af-eye-good-bad-cascade` plan (Step 2) judges a frame with no trusted AF point (every Leica M and Sigma BF file, and manual-focus frames) on the largest confident face YuNet finds on the whole preview (`FACES_VERSION` 9). `Focus` now carries the cue without a point, and `focusMark` draws no crosshair for it. Unit tests and `mise run ci` cover the state mapping and the stored-vs-on-demand agreement (checked by a temporary example on `2026-02-01` and 12 manual-focus ARWs). Nobody has looked at the rendered strip icon, filter result and meta pane for these frames in the running app. Plan: `docs/plans/_archived/20261010-af-eye-good-bad-cascade/plan.md`. The `face-focus-tiers` plan (`docs/plans/_archived/20261107-face-focus-tiers/plan.md`) later renamed the states: bad is now `Low`, and a no-face frame is `Neutral`.

Files: `crates/app/ui/src/focus.ts`, `crates/app/ui/src/main.ts`, `crates/backend/src/index.rs`.

#### TODO

- [ ] On Windows or macOS, open `D:\photos\2026\2026-02-01` in a build with this change and let the second pass re-run (`FACES_VERSION` 9). Check that Leica DNGs with a face get the strip's `scan-face` (good) or `scan` (low) icon although `f` draws no crosshair, or no icon when the face is neutral; that the filter's `Face focus` `Good` / `Low` items list them; that the meta pane shows `AF eye in focus`, `Eyes`, `Yaw`, `Pitch` and `Roll` for them from the stored values, matching the on-demand mesh overlay; and that a DNG with no face stays without an icon and under `Neutral`.
- [ ] On Windows or macOS, repeat on a manual-focus ARW with a face (e.g. `D:\photos\samples\ARW\ILCE-6400_DSC00405.ARW` copied into a test folder under `D:\Photos\tests\`): same results, and still no crosshair.

### App: real-device check of the removed sharpness displays and the new Compare pairing

#### Background

The `af-eye-good-bad-cascade` plan (Step 5) removed the strip's sharpness bar, Compare's `BEST` label, score and green outline, and the meta pane's `Sharpness` row. A lone file in Compare is now paired with the first displayed good frame of its burst in capture order, else the burst's first frame. `compare.test.ts`, `meta.test.ts` and `mise run ci` cover the logic. The rendered result in the running app was never looked at. Plan: `docs/plans/_archived/20261010-af-eye-good-bad-cascade/plan.md`.

Files: `crates/app/ui/src/main.ts`, `crates/app/ui/src/compare.ts`, `crates/app/ui/src/strip.ts`, `crates/app/ui/style.css`.

#### TODO

- [ ] On Windows or macOS, open a folder with bursts (e.g. a copy of an ARW burst folder under `D:\Photos\tests\`). Expect no bar up the left edge of any strip cell and no `Sharpness` row in the meta pane's Analysis group. Press `v` on one file of a burst: it should show beside the burst's first bright-green (good) frame, else the burst's first frame, with labels of the file name (and `ACTIVE`) only, with no `BEST`, no score and no green outline.

### Core: the `.dop` reader ignores a picked PhotoLab virtual copy

Found in `burst-keep-score` Step 1 (the inventory, `docs/plans/_archived/20261008-burst-keep-score/data.md`). `dop::read_flag` (`crates/core/src/dop.rs`, `locate`) reads only the first `Items` entry, so it returns `None` for 199 frames of `2026-06-05` and 228 of `2026-09-13-b` whose virtual copy (a later item) is picked in PhotoLab. Riffle shows those frames unflagged, so the strip's filter does not count them as picked. Whether Riffle should read any item's flag, and which item it writes, is a product decision.

Files: `crates/core/src/dop.rs`.

#### TODO

- [ ] Decide which `Items` entry's flag Riffle reads and writes, then change `read_flag` and its tests to match.

### App: the MCP `get_view` eyes summary carries no openness

`burst-keep-score` Step 4b made the meta pane's `Eyes` row show the openness (0-100 from the EAR) but kept `get_view`'s `state` / `probability` / `pose` fields as planned. Whether `EyesSummary` should carry the EAR or the openness is open.

Files: `crates/app/ui/src/companion.ts` (`EyesSummary`), `crates/app/src/mcp.rs`.

#### TODO

- [ ] Decide whether `get_view`'s eyes summary carries the EAR or the openness, and add it with its test.

### App: real-device check of the 1:1 view on files with a malformed full-size JPEG

The `partial-decode-error-exit` work (`docs/plans/_archived/20261002-partial-decode-error-exit/plan.md`) made `decode_region` in `crates/core/src/partial.rs` return `Err` on a fatal libjpeg error instead of calling `exit(1)`. Unit tests (empty input, non-JPEG input, a JPEG truncated inside the header) and `riffle-cli crop` / `check` on the real files cover the core path. `crop` returned `libjpeg fatal error: Not a JPEG file: starts with 0x3c 0x44` for `NIKON_D70_Nikon.nef` and `... 0x80 0x03` for `CGO3P_YUN00007.dng`. `check` over the samples ran to its summary in 23.0 s. The desktop app's 1:1 view (the `focus_crop` command, which calls the same `decode_focus_crop`) was never exercised, and neither was what the UI shows for the error. Files: `crates/app/src/commands.rs` (`focus_crop`), `crates/core/src/partial.rs`.

#### TODO

- [ ] On any platform, open `D:\photos\samples\NEF\NIKON_D70_Nikon.nef` and `D:\photos\samples\DNG\CGO3P_YUN00007.dng` in the app and switch to the 1:1 view. Expect the app to stay up and the `focus_crop` error (`libjpeg fatal error: Not a JPEG file ...`) to surface however the 1:1 view shows a failed crop. Note what the UI shows; if it shows nothing useful, file a follow-up.

### App: real-device check of the preview-failed state when the selected file's preview fails

#### Background

The preview-failure-clears-stale-image feature clears the main preview and shows a "preview-failed" overlay (`The preview of this file could not be shown.`) when the current file's preview fails. Before this, the previous file's image stayed on screen. The automated criteria are covered by `empty.test.ts` and `mise run ci`. The real-app behavior (a real undecodable NEF, fast paging across it, `z` and Compare on it) was never exercised. Plan: `docs/plans/_archived/20261002-preview-failure-clears-stale-image/plan.md`.

Files: `crates/app/ui/src/main.ts`, `crates/app/ui/src/empty.ts`

#### TODO

- [ ] On Windows, open `D:\Photos\samples\NEF\`, view a normal NEF, then `NIKON_D70_Nikon.nef`. Expect the main preview no longer shows the previous NEF and reads "The preview of this file could not be shown.", with the error in the meta pane's status block.
- [ ] On Windows, page on from the bad file to a normal file. Expect it to show normally and the overlay to disappear.
- [ ] On Windows, page back and forth quickly across the bad file. Expect an older file's image never stays on screen.
- [ ] On Windows, press `z` (1:1) and open Compare on the bad file. Expect neither to throw.

### Core: validate the combined AF-eye score on the two reserved labeled folders

`docs/plans/_archived/20260926-af-eye-in-focus-probability/plan.md` reserved
`D:\Photos\tests\2026-08-29-focus-sample` and
`D:\Photos\tests\2026-09-13-b-focus-sample` (100 ARW each) as a
post-implementation check, but as of Step 1 neither carried XMP pick/reject
labels yet, so they were not run.

#### TODO

- [ ] Once `2026-08-29-focus-sample` and `2026-09-13-b-focus-sample` carry XMP
      pick/reject labels, run
      `riffle-cli candidates D:\Photos\tests\2026-08-29-focus-sample D:\Photos\tests\2026-09-13-b-focus-sample`
      and record AUC / precision / coverage. The cue now scores the mesh
      eye regions (`docs/plans/_archived/20261008-mesh-eye-focus/`), whose
      gain over the window rests on few off frames (9 held-out off frames
      among the meshed ones), so also run that plan's `fit.py` window /
      region comparison on them, and the eyelid contour mask comparison of
      `docs/plans/_archived/20261008-mesh-eye-mask/` (not adopted on the
      held-out set, but it ranked the training set's meshed frames better:
      meshed AUC 0.974 against the window's 0.951 on the same frames, where
      the rectangle reached 0.948 against the window's 0.946). Files: `crates/cli/src/main.rs`,
      `crates/core/src/candidate.rs`,
      `docs/plans/_archived/20261008-mesh-eye-focus/fit.py`,
      `docs/plans/_archived/20261008-mesh-eye-mask/fit.py`.

### Docs: the "Focus candidate pass" numbers in docs/humans/performance.md are missing the app's own scan/faces log lines

Step 3 of `docs/plans/_archived/20260925-focus-candidate/plan.md` asked for the
`scan extract` / `scan faces` log lines of one app open of the 2134-file
Sony folder to be recorded in `docs/humans/performance.md` "Focus candidate pass",
alongside the `riffle-cli scan` / `riffle-cli candidates` numbers already
there. The implementing agent could not drive the GUI, so only the CLI
numbers are recorded; `docs/humans/performance.md` says so in a note under the
section.

#### TODO

- [ ] Open the 2134-file Sony folder once in the app and record its
      `scan extract` / `scan faces` log lines in `docs/humans/performance.md`
      "Focus candidate pass", next to the existing CLI numbers.

### App: real-device check of the stacked latency bar and its legend in the performance overlay

#### Background

`preview-latency-breakdown` Step 1 split the overlay's per-switch latency bar into seven stages (`wait`, `read`, `ipc`, `post`, `worker`, `decode`, `draw`), stacked bottom to top in path order in steps of the `--chart-latency` red, and added the stage names to the legend after `Latency`. CI covers the model (`perf.test.ts`, `stagesOf`) and the build. Drawing the canvas was never exercised: the step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261108-preview-latency-breakdown/plan.md`. Files: `crates/app/ui/src/main.ts` (`drawPerf`), `crates/app/ui/src/perf.ts`.

#### TODO

- [ ] On Windows, open a folder, press `d` and page with `→`. Each red latency bar is stacked in seven shades of red, the legend names `wait` to `draw` after `Latency`, and it does not collide with the MB/s / ms text at the right on a normal window width.

### App: the prefetch hit-rate re-run of scenarios 1 and 5 for results.md is still open

#### Background

`preview-latency-breakdown` Step 4 keeps the neighbours' decoded previews in a ring (`prefetch.ts`) and draws a page turn to a held index with no invoke, logging `source=prefetch` on the `page` line. CI covers the ring model (`prefetch.test.ts`) and the build. The hit rate and the hit / miss `keypressToPixels` were never measured on a real machine, and the rename, delete and folder-switch invalidation was never seen: the step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261108-preview-latency-breakdown/plan.md`. Files: `crates/app/ui/src/main.ts` (`drawHeld`), `crates/app/ui/src/prefetch.ts`, `docs/plans/_archived/20261108-preview-latency-breakdown/results.md`.

#### TODO

- [ ] On Windows, in a `mise run tauri:release:devtools` build with `Timing logs` and the overlay (`d`) on, open a scanned folder of 1616x1080 previews with the status line idle and press `→` about once a second, 30 times. Then read `%LOCALAPPDATA%\com.minodisk.riffle\logs\Riffle.log`. Expected: after the first press every press is `source=prefetch` with `read`, `ipc`, `post`, `worker` and `decode` at 0.0ms and a `keypressToPixels` of a few ms (the `draw`, about 2 ms).
- [ ] On Windows, in the same build, open `D:\photos\samples\RAF` after its scan is idle and press `→` 20 times; read the log between the two runs. Expected: the same `source=prefetch` zeros, with `keypressToPixels` up to about 15 ms (the `draw` of a 4416x2944 RAF). Split the `page` lines of both runs by `source=prefetch` / `source=read` and write the hit rate and the hit / miss p50 / p95 of `keypressToPixels` side by side in `results.md`.
- [ ] On Windows, rename and delete a file and switch folders while paging. Expected: no stale image appears.

### App: cold first scan on an internal SSD is far slower than the extrapolation

A real cold first scan on Windows 11 (internal SSD, 22 threads, Sony ARW) costs ~16-19ms per file, ~82-97s extrapolated to 5000 files against the 30s target; see "Real folders on Windows" in docs/humans/performance.md. Excluding the folder from Defender did not help, and a warm-cache scan runs at ~1ms per file, so neither Defender nor CPU is the cause. The cause is unknown. A later data point (Windows 11, v0.2.0, 2026-09-22): a cold first scan after an index schema change took 25.5s on 3045 Sony ARW (~8.4ms/file, ~42s extrapolated to 5000), against the earlier 16-19ms/file; it is unknown whether the OS cache was cold for that run.

#### TODO

- [ ] Run `riffle-cli scan` on a cold real folder on Windows at thread counts 1 / 4 / 8 / 22 (cold each run) to separate IO concurrency from per-file cost, and compare the bounded 1MiB read against reading the whole file.

### App: a deep-row focus point still exceeds the 50ms budget

A focus point in a deep row of the unrotated JPEG measured 58-65ms keypress to pixels (n=2, before the #56 and #60 fixes); see "The 1:1 focus check path" in docs/humans/performance.md. Options are prefetching the neighboring files' crops (Phase 4's ring buffer) or a DCT-scaled placeholder; nothing is chosen.

#### TODO

- [ ] Retake the deep-row measurement post-fix, then decide between crop prefetch and a DCT-scaled placeholder.

### App: the silent update path is unverified end-to-end

The Updating paragraph in `docs/humans/usage.md` describes a background download and install on launch and the "Check for Updates…" menu item, but nothing has confirmed on a real build that an installed copy detects a newer release, installs it silently, and launches as the new version next time. The background flow is verified on Windows 11 (2026-09-22): 0.1.10 downloaded 0.2.0, installed it on quit, and launched as 0.2.0.

#### TODO

- [x] Background update flow on Windows (Windows 11, 0.1.10 -> 0.2.0, 2026-09-22): downloaded, installed on quit, launched as 0.2.0.
- [ ] Verify the **Check for Updates…** menu item reports the up-to-date / installed / already-installed outcomes correctly on macOS, Windows, and Linux AppImage.
- [ ] Verify the background flow on macOS and Linux AppImage: install the current release, publish the next one, then launch the installed build. Done when the newer release is installed after the signature check and used on the next launch.

### Docs: consider a guide for verifying Pillow pixel edits

`app-icon-enlarge-marks` hit two Pillow pitfalls while verifying an in-place
pixel edit to `crates/app/icons/source.png`: (1) `Image.getbbox()` defaults to
`alpha_only=True` on RGBA images (Pillow >= 9.5), so a "no difference" check
built on `ImageChops.difference(...).getbbox()` can silently pass while the
RGB channels differ — use `getbbox(alpha_only=False)` or a per-pixel scan;
(2) measuring a resized/pasted mark inside the padded crop box that produced
it clips the bbox and understates the mark's size — the measurement window
must be wider than the source crop.

#### TODO

- [ ] The next time a plan does Pillow-based pixel verification, check whether
      it hits the same two pitfalls. If it does, write
      `docs/agents/verifying-pixel-edits.md` (or a section in an existing
      `tauri-app.md`-style guide) covering both, linking
      `docs/plans/_archived/20260921-app-icon-enlarge-marks/learnings.md` for
      the concrete numbers. If it does not recur, drop this item.

### App: the macOS / Linux manual checks for Move to Trash are still open

The strip's `Move to Trash…` (`trash-from-strip`, which replaced the folder
tree's `Move Rejected to Trash…` family) moves files through the same mover
and records the same `trash::Runs` runs the removed feature did, and that
mover has never been run by a human on macOS or Linux. On macOS the mover
is `trash::trash_file` (`crates/backend/src/trash.rs`, `undo-trash-rejected`
#547), which calls `NSFileManager.trashItemAtURL` directly rather than the
`trash` crate's `DeleteMethod::NsFileManager`; one point is specifically
unverified: whether the Trash's "Put Back" entry is actually created, since
the `trash` crate documents that on some macOS systems files moved through
`NSFileManager` get no "Put Back" entry (trash-rs#14); dragging them out of
the Trash still restores them. The Windows checks of the strip flow are in
`### App: the Windows real-device checks for the filmstrip's Move to Trash are still open`.
Files: `crates/app/src/commands.rs` (`trash_files`, `trash_one`),
`crates/backend/src/trash.rs`, `crates/app/ui/src/trash.ts`,
`crates/app/ui/src/main.ts`. Sources:
`docs/plans/_archived/20260928-undo-trash-rejected/learnings.md` Step 2
(#547) and `docs/plans/_archived/20261013-trash-from-strip/plan.md`.

#### TODO

- [ ] On macOS, right-click a strip cell and pick `Move to Trash…`:
      `#trash-dialog` asks `Move 1 file to the Trash?` with `Move to Trash` /
      `Cancel`; Cancel leaves the folder untouched; confirming moves the RAW
      plus its `.xmp` and `.ARW.dop` to the Trash and the strip shows the
      neighbouring file; a "Put Back" from the Trash restores the file with
      its judgment, and if no "Put Back" entry exists, dragging it out does.
- [ ] Verify the same flow on Linux (the `trash` crate's freedesktop backend
      has never been run here).
- [ ] On macOS, move a rejected file to the Trash, then `Edit > Undo`: the
      file comes back with the reject flag, and a file with the same name
      placed at the original location beforehand is reported
      (`already exists at the original location`) and not overwritten.
      (Source: `docs/plans/_archived/20260928-undo-trash-rejected/learnings.md`
      Step 2, #547.)

### App: the Windows real-device checks for the filmstrip's Move to Trash are still open

`trash-from-strip` added `Move to Trash…` to the filmstrip's right-click menu (`Move N Files to Trash…` for a multi-selection) and removed the folder tree's `Move Rejected to Trash…` family. Step 2 was ticked on the automated criteria only (`vp test` and `mise run ci`: the menu item, its label, the view-only menu). Nobody has run the flow in the app on Windows, since an agent session cannot drive it. Use a copy of a folder under `D:\Photos\tests\<date>-trash-from-strip\`. See `docs/plans/_archived/20261013-trash-from-strip/plan.md`.
Files: `crates/app/ui/src/main.ts` (`confirmTrashFiles`, `trashFilesIn`), `crates/app/ui/src/context.ts`, `crates/app/src/commands.rs` (`trash_files`).

#### TODO

- [ ] On Windows (`mise run tauri:dev`), right-click one file and pick `Move to Trash…`. Expect the dialog to say `Move 1 file to the Trash?`. `Move to Trash` should move the RAW and its `.xmp` / `.dop` to the Recycle Bin, the strip should show the neighbouring file, and the status line should say `Moved 1 file to the Trash`.
- [ ] On Windows, select 3 files and right-click one of them. Expect the item to read `Move 3 Files to Trash…`, and confirming should move exactly those 3.
- [ ] On Windows, `Cancel` and `Escape` in the dialog leave everything untouched.
- [ ] On Windows, filter to rejects, press `Ctrl+A`, then pick `Move N Files to Trash…`. Expect only the filtered files to move.
- [ ] On Windows, `Ctrl+Z` brings the moved files back with their judgments, and `Ctrl+Shift+Z` moves them to the Trash again.
- [ ] On Windows, confirm the dialog while a scan is running. Expect the status line to say it is waiting for the scan, and the files to move when the scan ends.
- [ ] On Windows, in a JPEG-only folder the menu is `Select All` + `Move to Trash…`. A JPEG should move alone without sidecars, and undo should restore it (`Restored 1 file from the Trash`).

### App: the muda template-icon fork is a temporary bridge

Custom menu-item icons now tint with the menu appearance (white in dark mode,
black in light mode) via a `[patch.crates-io]` fork of muda 0.19.3
(`minodisk/muda`, `rev = "ef4fbfda53416382a2eeca7da683a64a8fe0e8ed"`) that
calls `nsimage.setTemplate(true)` unconditionally in `menuitem_set_icon`. This
is a temporary bridge, not the permanent fix: upstream PR
https://github.com/tauri-apps/muda/pull/413 carries the same change behind an
opt-in `set_icon_as_template` API; it merged on 2026-09-29 and shipped in
muda v0.21.0 (2026-09-30).

As of 2026-10-05: Tauri v2.12.1, the latest v2 release, depends on
`muda = "0.20"`, so it does not resolve muda 0.21; Tauri's menu-item API still
has no template flag (only tray icons have one); the workspace is still on
tauri 2.11.6 with the fork's muda 0.19.3.

#### TODO

- [ ] Once a Tauri v2 release depends on muda >= 0.21 (where muda#413's
      `set_icon_as_template` lives) **and** Tauri exposes the template flag
      for menu items, drop the `[patch.crates-io]` entry from the workspace
      `Cargo.toml` and switch to the upstream opt-in API.
- [ ] Re-verify in the running app that all bundled menu PNGs still tint
      correctly with the menu appearance after the switch.

Related: `Cargo.toml`, `docs/agents/tauri-app.md`,
`tools/macos/export-menu-icons.swift`.

### App: real-device checks for the View menu

`view-menu-panes` (docs/plans/_archived/20260930-view-menu-panes/plan.md) cut the native
`View` menu down to three `CheckMenuItem`s (`Folders`, `Metadata`,
`Filmstrip`) whose checks follow the shown panes, with the pane toggles moved
to modifier defaults (`Ctrl+Alt+Arrow` on Windows / Linux, `Alt+Cmd+Arrow` on
macOS) so they show an accelerator. CI covers the build and the tests. The
macOS `cfg` branch of `app_menu::build` / `refresh` and the revert of muda's
native toggle on click (muda toggles the check before it sends the event) were
confirmed on a real Mac on 2026-10-05 (`mise run tauri:dev`): the items, their
accelerators, the checks, the single fire per key and the rebuild on a rebind
all behaved. The Windows GUI checks (the menu, the checks, the accelerators)
were never run: the step was ticked on the automated criteria only.
`view-pane-names` (docs/plans/_archived/20260930-view-pane-names/plan.md) then
renamed the first two items from `Left Pane` / `Right Pane` to `Folders` /
`Metadata` (ids, actions and stored keys unchanged); its Windows label check
was likewise never run, and the first Windows TODO below covers it.

Files: `crates/app/src/main.rs` (`app_menu`, `apply_panels`),
`crates/app/src/commands.rs` (`set_panels`, `AppPanels`),
`crates/backend/src/shortcuts.rs` (defaults).

#### TODO

- [ ] On Windows, run `mise run tauri:dev` and confirm at launch that `View`
      shows `Folders`, `Metadata`, `Filmstrip` with `Ctrl+Alt+ArrowLeft` /
      `ArrowRight` / `ArrowDown` and checks matching the panes the last
      session left.
- [ ] On Windows, press each of the three keys once: the pane toggles exactly
      once (no double fire) and the check follows. `Tab`, `f`, `z`, `v` still
      work and are not in `View`. `F6` / `F7` / `F8` now do nothing.
- [ ] On Windows, click `View > Filmstrip` twice: hides, shows, check right
      each time. Open Settings, click `View > Folders`: nothing changes and
      the check stays.
- [ ] On Windows, rebind `toggleStrip` to `ctrl+alt+s`: the item shows it,
      `Ctrl+Alt+S` toggles once, and `Reset` restores `Ctrl+Alt+ArrowDown`.
      With the folder tree focused, `Ctrl+Alt+ArrowLeft` still hides the tree
      (tree passthrough).
- [ ] On Windows, watch for an Intel graphics hotkey taking `Ctrl+Alt+Arrow`
      (screen rotation). If it does, note it in `docs/humans/usage.md` as a driver
      setting to turn off (not a blocker; do not change the default).

### App: the real-device checks for the File menu accelerators are still open

From `menu-accelerators`'s implementation: the GUI could not be driven from
the agent session, so the double-fire, stale-key-equivalent,
settings-capture and menu-set-from-`setup` checks are unconfirmed on macOS,
and Windows / Linux are unconfirmed entirely. A later manual run on Windows
(`mise run tauri:dev`, `Ctrl` in place of `Cmd`) passed all six checks:
`Ctrl+O` opens the picker exactly once; `Shift+Ctrl+O` fires once (judged
only from a single status line, which a second identical message would
overwrite); rebinding `open` updates the menu accelerator and kills
`Ctrl+O`; the menu shows correctly with the settings window open and steals
no focus; a capturing shortcuts row swallows `Ctrl+O` / `Shift+Ctrl+O`;
both File items work by mouse. Files: `crates/app/src/main.rs`
(`app_menu`), `crates/backend/src/shortcuts.rs`, `crates/app/src/commands.rs`
(`update_keymap`), `crates/app/ui/src/main.ts`.

Since `undo-redo-keymap`, `Edit > Undo` / `Edit > Redo` are keymap actions
too (`Cmd+Z` / `Shift+Cmd+Z` on macOS, `Ctrl+Z` / `Ctrl+Shift+Z`
elsewhere), run by the frontend keydown with keymap-derived menu
accelerators. Before that change, `Ctrl+Z` did nothing on Windows 11
(v0.2.0) while clicking the Edit items worked. A manual run on Windows on
2026-09-28 (`mise run tauri:dev`) passed: `Ctrl+Z` / `Ctrl+Shift+Z` each
fire exactly once, the Edit menu shows the accelerators, rebinding `undo`
updates the menu in place and kills the old key with no `menu item undo not
found` in the log, and both Edit items work by mouse. Select All passed the
same run: `Ctrl+A` with the strip focused selects every file and keeps the
current one, it does nothing to the strip with the folder tree focused,
inside a settings text input it selects the input's text, and `Edit > Select
All` shows the accelerator and updates in place when `selectAll` is
rebound. (`Ctrl+A` first did nothing because of a global PowerToys Keyboard
Manager remap of left `Ctrl+A` to `Home`, not the app; the tell-tale is a
keydown sequence of `Unidentified` then `Home`.) macOS is still unrun.

#### TODO

- [ ] On macOS, verify: one `Cmd+O` press opens the folder picker exactly
      once (no double fire from keydown + menu accelerator); rebinding
      `open` updates the menu accelerator and kills the old key,
      with the macOS key equivalent not staying stale; a menu set from
      `setup` shows correctly and doesn't steal focus from the settings
      window; pressing `Cmd+O` while a shortcuts row is
      capturing does not trigger the menu action; `Open Folder…` works via
      mouse click.
- [ ] On macOS, verify: one `Cmd+Z` press undoes exactly once and one
      `Shift+Cmd+Z` redoes exactly once (no double fire from keydown + the
      Edit menu key equivalent); rebinding `undo`/`redo` updates the Edit
      menu accelerator and kills the old key; both Edit items work via mouse
      click. Files: `crates/app/src/main.rs` (`app_menu`),
      `crates/backend/src/shortcuts.rs`, `crates/app/ui/src/main.ts`.
- [ ] On macOS, verify Select All: pressing `Cmd+A` once with the strip
      focused selects every file (the current file stays current); pressing
      it with the folder tree focused does nothing to the strip; pressing it
      inside a settings text input selects that input's text. Check
      `Edit > Select All` shows the accelerator and that rebinding
      `selectAll` updates it. Files: same as above, plus
      `crates/app/src/commands.rs`, `crates/app/ui/src/settings.ts`.
- [ ] Verify the `Some`/`None` accelerator behavior on Linux (only
      reasoned from muda 0.19.3's sources so far, never run; Windows passed).

### App: the Clear Cache button's manual GUI verification is still open

From `clear-cache-stuck-guard`'s implementation: the button's guard used to
get stuck (never re-enabling after the first scan), so none of its GUI
behavior has been run by a human. GUI automation is unavailable on this
Mac and a native confirm dialog cannot be driven by an agent. A later manual
run on Windows passed (1) and showed the dialog in (2) immediately; the
Cancel half of (2) and checks (3)-(7) were blocked by a Clear Cache refusal
bug, since fixed (`focus-rescan-main-window`), and are still to be run.
`settings-modal` then moved the settings into a modal in the main window,
parented the confirmation on `main` and dropped the `index-clearing`
status text; none of that has been run by hand either. Files:
`crates/app/src/commands.rs`, `crates/app/ui/index.html`,
`crates/app/ui/src/settings.ts`, `crates/app/ui/src/main.ts` (the
`tauri://focus` listener). `wait-for-scan` then replaced the disabled
button during a scan with holding the clear until the scan ends (checks
(4) and (5) below were rewritten for it). A run on Windows was attempted
on 2026-09-28 with `mise run tauri:release:devtools` but not carried out:
the process was killed for low memory on the PC before the checks ran.

#### TODO

- [ ] Run `mise run tauri:release:devtools` and check, in order: (1) after
      a folder's scan finishes, Settings > Cache shows the button enabled
      and the note hidden; (2) pressing Clear Cache shows the confirmation
      dialog immediately, and Cancel leaves the size and `#settings-status`
      unchanged with the button re-enabled; (3) Clear Cache then Clear
      keeps the button disabled until the size drops and the main window
      rescans, and never refuses with `a scan is running` (closing the
      confirmation refocuses `main`, whose focus listener skips `resync()`
      while the modal is open); (4) pressing Clear Cache while a scan is
      running (or with a large folder opening under the settings modal)
      holds it: the button stays enabled, `#clear-index-note` shows the
      waiting text, and the clear runs by itself when the scan ends, after
      which the size figure updates; closing the modal while it is held
      drops it; (5) pressing Clear Cache during a large folder's prepare
      phase (before the first `scanning N / M` line) holds it the same
      way; (6) an error path, if reachable, writes the refusal to
      `#settings-status` and it stays until the next press; (7) note
      whether the very first press after opening the settings modal ever
      does nothing, and if so record the window focus state at that
      moment; (8) the native confirmation looks right over the settings
      modal on macOS and Windows.

### App: burst grouping's manual checks are still open

From `burst-grouping`'s implementation: GUI automation is unavailable on this
development machine, so several behaviors were never exercised by a human.
The Sony half of the band and badge check passed on Windows on 2026-09-28
with an ILCE-7M5 folder: band visible, the `position/count` badge on every
member and tracking the position, the gap filled, and the band
distinguishable from `.cell.current` and `.cell.failed` (`.failed` was
produced with an ARW truncated to its first 2 KB).

#### TODO

- [ ] Verify the burst band and count badge by hand on a real Leica burst
      folder and note the group sizes: band visible, every member cell
      shows its `position/size` (a two-digit one like `12/15` clear of the
      file name), unchanged by the selection, the gap above a
      non-first member is filled, and the band is distinguishable from
      `.cell.current` and `.cell.failed`. Files: `crates/app/ui/src/burst.ts`,
      `crates/app/ui/src/strip.ts`, `crates/app/ui/style.css`.
- [x] `Alt+ArrowUp` / `Alt+ArrowDown` (`burstFramePrevious` /
      `burstFrameNext`) reach the app on Windows and step through bursts
      (Windows 11, v0.2.0, 2026-09-22).
- [ ] In `mise run tauri:dev`, confirm `Alt+ArrowUp` / `Alt+ArrowDown`
      reach the app on macOS, i.e. the webview does not swallow them, and
      step through a burst stopping at its ends. Files:
      `crates/backend/src/shortcuts.rs`, `crates/app/ui/src/main.ts`.
- [x] `Shift+x` reject-rest and its one-step undo exercised by hand on a
      real burst folder (Windows 11, v0.2.0, 2026-09-22).
- [x] On Windows, the strip scrollbar is thin and dark and the thumbnail
      right edge and the burst bracket are not clipped (Windows 11, v0.2.0,
      2026-09-22).
- [ ] On macOS, confirm the strip looks unchanged (160px wide). Files:
      `crates/app/ui/style.css`.

### App: Windows real-device check of the merged folder-tree, scan-wait and strip-scroll work

Merged since the base of the 2026-09-28 Windows GUI check run and not run by
hand on any platform: `tree-live-watch` (#517, #520, #523),
`trash-rejected-from-tree` (#518, #521, #526, #529, #530), `wait-for-scan`
(#519, #522, #525) and `strip-keep-scroll-on-rescan` (#528). The last one
fixes the scroll jump-back and flicker seen on Windows on 2026-09-28 while
copying 100 ARWs into an open folder. Files: `crates/app/src/treewatch.rs`,
`crates/backend/src/trash.rs`, `crates/app/ui/src/folders.ts`,
`crates/app/ui/src/idle.ts`, `crates/app/ui/src/strip.ts`,
`crates/app/ui/src/main.ts`. #518's check of the relabeled File menu item is
not listed: that item was removed by `file-menu-folder-items` (#537). The
tree trash checks come from
`docs/plans/_archived/20260928-trash-rejected-from-tree/learnings.md`
Steps 2–4 and the bodies of #521, #526 and #529; the tree's trash items and
their confirmation dialog were removed by `trash-from-strip`, so only the
selection checks remain; the undo / redo checks are in
`### App: the manual GUI checks for Undo and Redo of Move to Trash are still open`.

#### TODO

- [ ] Tree watch on Windows: with a folder expanded, create, delete and
      rename a subfolder in Explorer and confirm the tree follows; confirm
      renaming a folder from the app still works with the watch on.
- [ ] Tree multi-selection on Windows (#526): `Ctrl+click` toggles a folder
      in and out of the selection (the open folder can be toggled off, and
      the selection can go empty); `Shift+click` selects the range from the
      anchor without selecting the rows' text; a plain click selects only
      the clicked folder; opening a folder any other way (Enter, drop,
      `File > Open Folder…`, the reopen at launch) resets the selection to
      it; selected rows draw `.selected` distinct from `.current` /
      `.cursor`; the container has `aria-multiselectable` and each row
      `aria-selected` following the selection; collapsing a parent or a
      `tree-changed` re-list drops folders no longer drawn; right-clicking
      any folder, selected or not, selects it alone and shows its own menu.
- [ ] Scan-wait on Windows: confirm the strip's Move to Trash and a rename while
      a large folder scans; confirm the status line says what is waiting and
      each runs when the scan ends. The Clear Cache hold is covered by
      checks (4) and (5) of "the Clear Cache button's manual GUI
      verification is still open".
- [ ] Strip scroll on Windows: repeat the 100-ARW copy into an open folder
      and confirm the strip's scroll position holds without flicker.

### App: the manual GUI checks for Undo and Redo of Move to Trash are still open

`undo-trash-rejected` merged as #540, #547, #549 and #551: a trash run (now
the strip's `Move to Trash…`, which replaced the folder tree's
`Move Rejected to Trash…` in `trash-from-strip`) is one undo unit;
`Edit > Undo` / `Ctrl+Z` restores it through `trash_rejected_undo`, and `Edit > Redo` / `Ctrl+Shift+Z` moves the restored
files to the Trash again through `trash_rejected_redo`, without the dialog.
The backend side was verified against the real Recycle Bin with throwaway
tests; the GUI side has not been run at all, since an agent session cannot
drive the app. The resume-selection check that used to sit here is in
`### App: the manual GUI checks for the resume landing's selection are still open`
(#553); the macOS undo check is in
`### App: the macOS / Linux manual checks for Move to Trash are still open`.
Files: `crates/backend/src/trash.rs`, `crates/app/src/commands.rs`,
`crates/app/ui/src/undo.ts`, `crates/app/ui/src/trash.ts`,
`crates/app/ui/src/main.ts`. Source:
`docs/plans/_archived/20260928-undo-trash-rejected/learnings.md` Steps 3
and 4 (#549, #551).

#### TODO

- [ ] On Windows, move rejected files of the open folder to the Trash with
      the strip's `Move to Trash…`, then `Ctrl+Z`: the
      files come back, the strip shows them again with the reject flag (read
      from the sidecars), the status line says
      `Restored N files from the Trash`, and the folder tree's count follows
      on its next re-list.
- [ ] The same with another folder opened since the move: only the status
      line changes.
- [ ] A conflict: copy a trashed RAW back by hand before the undo; the
      status line says `…, N failed`, the error list shows
      `could not restore from the Trash: already exists at the original location`,
      and the RAW's sidecars stay in the Trash, reported as
      `left in the Trash: its RAW could not be restored`.
- [ ] An emptied Recycle Bin: every file is reported
      `not in the Trash (emptied or restored by hand)` and the entry leaves
      the undo history.
- [ ] After a restore, the index picks the reject judgments back up on the
      `resync` (a following `Ctrl+Z` undoes the judgment batch beneath, not
      the trash run again).
- [ ] `Ctrl+Z` (files back), `Ctrl+Shift+Z` (moved to the Trash again with
      no dialog, the strip refreshed, status `Moved N files to the Trash`),
      then `Ctrl+Z` again (back again).
- [ ] The same undo / redo / undo with another folder open.
- [ ] A judgment made after the undo clears the redo: `Ctrl+Shift+Z` does
      nothing.

### App: the scan loading feedback's manual check is still open

The first scan pass now shows a thin progress bar above the strip bar
(`#scan-progress`) and every strip cell without a thumbnail pulses
(`.cell img:not([src])`); a failed cell does not. Neither has been seen on
a real machine yet. Files: `crates/app/ui/src/main.ts`,
`crates/app/ui/style.css`, `crates/app/ui/index.html`.

#### TODO

- [ ] On Windows, open a folder of 500+ files whose index is cold (or after
      Clear Cache) and confirm the bar appears before the first thumbnail,
      fills, and disappears at the end of the first pass, and that empty
      cells pulse until their thumbnail lands while failed cells do not.

### App: face/eye-aware focus check for culling

Face/eye-aware detection and scoring (`crates/core/src/faces.rs`, `crates/core/src/sharpness.rs`) now runs at scan time: sharpness is scored on the Sony eye-AF frame, else on the AF point, else on the eyes of a detected face when there is no trusted AF point, else on the sharpest tile; an AF point off the face no longer scores a bystander's eyes (see `docs/plans/_archived/20260922-face-aware-sharpness/` and `docs/plans/20260924-face-catch-state/`). Without a trusted AF point the whole-preview search now runs at a 640x448 input on a DCT-scaled decode, which raised small-face recall at about 1.7x the earlier per-file cost; closed eyes were found to need a second model (see `docs/plans/_archived/20261005-face-detection-recall-cost/`), which now judges the shown file's eyes on demand, outside the scan, for the meta pane's `Eyes` row (see `docs/plans/_archived/20261007-closed-eyes-detection/`). What remains:

#### TODO

- [x] MakerNote check (exiftool 13.59, 2026-09-22): Sony α7 V ARW writes
      `AFTracking` + `FocusFrameSize` + `FocusLocation` but no per-face list;
      `AFAreaMode` is enciphered; Leica M11-P DNG writes only
      `FocusDistance`. The scan now uses the Sony AF frame and skips detection
      when face tracking was engaged
      (`docs/plans/20260922-sony-eye-af-window/`).
- [x] Measured on Linux WSL2 (not the Mac): detection latency on real
      α7 V ARW / M11-P DNG previews and `riffle-cli scan` before/after adding
      detection and the AF-frame skip; see `docs/humans/performance.md` "Face
      detection cost".
- [x] Optionally, detect closed eyes. YuNet's five landmarks carry no
      eyelid points, so it needed a second model. Four were surveyed on
      2026-10-07 against 504 hand-labeled faces (`model-survey.md`,
      `eyes-truth.md`); none fits the scan's second pass. Adopted: MediaPipe
      Face Landmarker v2 (Apache-2.0, 4.9 MB ONNX converted from Google's
      TFLite, `crates/core/src/eyes.rs`), judging closed eyes by the eye
      aspect ratio (closed iff at most 0.137) of one face, the one nearest a
      trusted AF point, else the largest face at or above 0.8, at or above a
      60 px face side, on demand for the shown file only (`eyes_of`), shown
      as the meta pane's `Eyes: Closed (NN%)` / `Open (NN%)` row. Cost:
      `riffle-app` 47.5 -> 52.5 MB, `riffle-cli` 24.6 -> 29.6 MB; ~35 ms
      per face (p95 41 ms), the whole call 80-92 ms on an ARW with an AF
      point and 140-156 ms on a DNG without one. Accuracy: face AUC 0.974,
      precision 0.94 / recall 0.86 at the threshold. See
      `docs/plans/_archived/20261007-closed-eyes-detection/` and
      `docs/humans/performance.md` "Closed-eyes judgment on demand
      (Windows 11)".
- [ ] Judge every face at or above the 60 px floor, not only the AF-nearest
      (or largest) one, so a bystander's blink shows too. Each face is one
      more model run (~35 ms; ~1.8 faces per file on the 2134-ARW folder,
      more in groups). Needs either a row per face or
      a worst-of summary (`Eyes: Closed (1 of 3)`), a pattern the meta pane
      does not have, and a check of the accuracy on non-subject faces, which
      `eyes-truth.md` did not label. Files: `crates/app/src/commands.rs`
      (`read_eyes`), `crates/core/src/eyes.rs` (`judged_face`),
      `crates/app/ui/src/meta.ts`.
- [ ] Use the model's face presence output (`Identity_1`, a logit the
      `eyes` module ignores) to drop false face detections before judging
      the eyes. Needs a threshold calibrated on labeled faces: the `x`
      labels of `docs/plans/_archived/20261007-closed-eyes-detection/eyes-truth.md`
      (no eye to judge, including false detections) can seed it. Files:
      `crates/core/src/eyes.rs`, `docs/agents/tract-onnx-inference.md`.
- [ ] Mark closed eyes in the strip. Pass 2 now stores the EAR for the face
      the cue meshes and the filter menu has an `Eyes` section (the
      `mesh-eyes-index` work), but no tile shows a closed-eyes mark yet; the
      stored `eyes` state (`open` / `closed` / `unknown`) is on `Focus`, so
      the remaining work is the mark itself. Files:
      `crates/app/ui/src/strip.ts`, `crates/app/ui/src/eyes.ts`.
- [x] Estimate the head pose of the judged face. MediaPipe's face geometry
      pipeline (perspective unprojection at its 63 deg camera, weighted
      Procrustes onto the canonical face) is ported to
      `crates/core/src/pose.rs` and runs on the points of the `eyes_of`
      model run, shown as the meta pane's `Head pose` row (yaw, pitch,
      roll; right, up and clockwise positive). On demand only, nothing in
      the index, so there is no strip filter for it. Cost: 8.2 µs per face,
      no change to the `eyes_of` total. Accuracy on 159 labeled faces: the
      sign right on 94% (yaw), 95% (pitch), 15 of 18 (roll); the yaw class
      on 68%, the gap at the frontal / oblique boundary. See
      `docs/plans/20261007-head-pose/` and `docs/humans/performance.md`
      "Closed-eyes judgment on demand (Windows 11)".
- [ ] Derive the good-photo rule in the backend, next to `candidate` in
      `crates/backend/src/index.rs`, so the MCP `get_view` tool can carry it.
      The rule lives only in `crates/app/ui/src/focus.ts` (`photoTier`) and
      `get_view` says nothing about it. Done when the index derives the tier
      from the stored columns with the thresholds in one place (the
      frontend reads it instead of recomputing), `get_view` carries it for
      the current file, and the boundary tests of `focus.test.ts` move with
      the rule. Files: `crates/backend/src/index.rs`, `crates/app/src/mcp.rs`,
      `crates/app/ui/src/focus.ts`, `crates/app/ui/src/companion.ts`.
- [ ] Measure the good-photo mark's precision on labels. Decision B rests
      on the user's stars and the mark rate; no labeled set was made. Done
      when 100-150 faced AF frames drawn stratified around the cuts and at
      random are labeled blind (`ok`, `miss` with `blur` / `focus` /
      `closed`, `turned`, `unsure`; the agent first, the user's review
      final) and the precision and coverage of the mark at its cuts are
      written with a Wilson interval, with the focus candidate as the
      baseline (the procedure is in `docs/plans/20261008-burst-keep-score/plan.md` Step 5). Files: the plan
      folder of that work, `crates/app/ui/src/focus.ts` if the cuts move. Include some rated frames with eye_focus between 0.772 and 0.99 (0.90
      cut) that clear the other cuts: GOOD_EYE_FOCUS 0.90 has no held-out
      check, because batch 1 has none in that range.
- [ ] Re-check the good-photo cuts after the closed-eyes and head-pose label
      reviews (the items in the sections below). If the user's review
      moves `EYES_CLOSED_EAR` in `crates/core/src/eyes.rs` or changes the
      pose labels' frontal / oblique boundary, re-read `GOOD_EYE_EAR` (and
      `EYES_CLOSED_EAR` / `EYES_WIDE_OPEN_EAR` in
      `crates/app/ui/src/focus.ts`, which mirror the core value and anchor
      the openness) and `GOOD_MAX_YAW` on the rated frames of
      `D:\photos\samples\ARW\good-mark-2026-10-09\`. Files:
      `crates/app/ui/src/focus.ts`, `crates/core/src/eyes.rs`.
- [x] Give the no-AF path a good-photo mark once it has a focus cue. Done
      in `docs/plans/_archived/20261010-af-eye-good-bad-cascade/` Step 2: pass 2 runs
      the cue, the EAR, the pose, the eye offset and the edge gap on the
      largest confident face of the whole preview when a frame has no
      trusted AF point (`FACES_VERSION` 9), and `Focus` carries the cue
      without a point, so the same cuts mark it. The check of those cuts on
      starred no-AF frames is the next item.
- [ ] Check the good-photo cuts on starred frames with no trusted AF point.
      The cuts in `crates/app/ui/src/focus.ts` (`GOOD_EYE_FOCUS`,
      `GOOD_EYE_EAR`, `GOOD_MAX_YAW`, `GOOD_MAX_PITCH`, `MAX_EYE_OFFSET`,
      `MIN_EDGE_GAP`) were fitted on AF faces only; since
      `docs/plans/_archived/20261010-af-eye-good-bad-cascade/` Step 2 they also judge
      the largest confident face of a frame with no AF point. Run
      `riffle-cli features` over the Leica folders with `.dop` picks
      (`D:\photos\2026\2026-09-05` and the other DNG folders of the
      `20261008-burst-keep-score` data set), apply the same cuts to the
      `noaf` rows in a script, and report the share of faced frames marked
      good, the share of `.dop` picks among them against the base rate, and
      10 marked and 10 unmarked picks for the user to look at. Done when the
      numbers are recorded in a plan's `learnings.md` and the cuts are either
      kept or a separate no-AF cut is proposed with them. Files:
      `crates/app/ui/src/focus.ts`.
- [x] Measure the AF eye in-focus probability over each eye's eyelid
      region from the face mesh instead of the window between the eyes.
      The scan's second pass runs MediaPipe Face Landmarker v2 on the face
      nearest the AF point, scores each eye's contour bounding box (margin
      0.5 of its longer side) with a refitted logistic and takes the sharper
      eye; when neither eye's region counts (under 24 px or without a clear edge) it scores the window as
      before (61% of the training frames, 60% of the 2134-ARW folder's
      faces). The head pose was measured as an eye rule and did not beat the
      sharper eye, so it is not used. Held-out AUC 0.754 -> 0.800, precision
      / coverage 89.1% / 95.3% -> 88.6% / 95.9%; pass 2 on the 2134-ARW
      folder 12.9 -> 21.5 s at 24 threads (+66%). `FACES_VERSION` 6. See
      `docs/plans/_archived/20261008-mesh-eye-focus/` and
      `docs/humans/performance.md` "Focus candidate pass".
- [x] Measure whether de-rotating the face crop by YuNet's eye line (the
      MediaPipe pipeline's roll step) fits the face mesh better. It does
      not, in any variant (always, dead bands of 10 / 20 deg, a YuNet
      eye-distance guard): always-rotate turns 7 off meshes on and 15 on
      meshes off on 131 readable labeled frames and drops the held-out AF
      eye AUC 0.800 -> 0.783; the share of AF frames with `eye_offset` over
      0.10 stays 28.7% on six folders. Where the rotation moves a fit,
      YuNet's two eye points are not on two eyes. The residual misfits are
      side faces (78% of the off meshes, weighted), then cut, rotated past
      90 deg, seen from above and not-a-face. Nothing in the scan or the
      index changed. See `docs/plans/20261010-mesh-roll/` (Decision,
      `compare.md`, `misfit-truth.md`).
- [ ] Make `eye_offset` reliable where YuNet's eye distance is under 0.20
      of the face box side, for `20261008-burst-keep-score`, which owns the
      `MAX_EYE_OFFSET` 0.10 cut. `docs/plans/20261010-mesh-roll/` found that,
      weighted to its six folders, about 27% of the frames over 0.10 have a
      mesh off the face and about 65% an on-face mesh compared against YuNet
      eye points that are off the eyes (profiles whose two points sit
      together on one eye or the nose); frames under 0.20 eye distance are
      33.5% of the frames but 66% of those over 0.10. Done when the cut is
      skipped below that eye distance or `eye_offset` is replaced by a check
      of the mesh against its own geometry, and the good-photo mark's share
      and the rated frames of `D:\photos\samples\ARW\good-mark-2026-10-09\`
      are re-read. Files: `crates/core/src/candidate.rs`
      (`mesh_eye_offset`), `crates/app/ui/src/focus.ts` (`MAX_EYE_OFFSET`).
- [ ] Distrust the mesh values on side faces, the dominant mesh misfit
      (78% of the off meshes, weighted, in
      `docs/plans/20261010-mesh-roll/misfit-truth.md`). Done when a rule
      (e.g. YuNet eye distance under 0.10 of the box side, or the mesh's
      |yaw| past a bound) leaves the EAR, the AF eye cue and the pose of
      such a face unknown instead of a plausible-looking wrong value, checked
      on the misfit truth set and on the AF-eye, closed-eyes and head-pose
      labeled sets. Files: `crates/core/src/candidate.rs`,
      `crates/core/src/eyes.rs`.
- [ ] Faces rolled past 90 deg (a baby lying head-down,
      `D:\photos\2026\2026-07-11\_DSC2638.ARW`) get a mesh fitted as if
      upright, and the eye line cannot de-rotate them: `faces.rs` orders
      YuNet's eye points by image x, so the eye-line roll folds into -90..+90
      deg. They need another roll source (a mesh pass on rotated crops,
      MediaPipe's tracking loop). Not planned; see
      `docs/plans/20261010-mesh-roll/` (Decision). Files:
      `crates/core/src/faces.rs`, `crates/core/src/eyes.rs`.
- [ ] Review the mesh misfit labels: open the sheets
      `D:\Photos\tests\2026-10-09-mesh-roll\sheets\s00.png` to `s15.png`
      and compare each tile to its row of
      `docs/plans/20261010-mesh-roll/misfit-truth.md` (`before` / `after`
      `on` or `off` with its kind, and `yunet ok` / `yunet off`). The
      Decision's shares rest on the agent's labels; if labels change,
      re-run `compare.py` in that plan folder and update `compare.md` and
      the Decision if a number moves.
- [ ] Repoint the MCP `get_view` tool's `eyes` answer to the stored values.
      Pass 2 now stores the EAR and the pose of the face it meshes (the
      `mesh-eyes-index` work: `eyes_ear`, `eyes`, `eyes_closed` and `pose` on
      `Focus` / `FaceReady`, `docs/plans/_archived/20261008-mesh-eyes-index/plan.md`),
      but `ViewApi.eyes` in `main.ts` still reads `eyesCache.get`, so
      `get_view` answers only for a file already shown (see
      `docs/plans/_archived/20261008-mcp-eyes-pose/plan.md`). Answer from the
      stored values when present, falling back to `eyesCache`, and keep the
      `eyes` key absent from `get_view` until a file is judged. Done when
      `get_view` answers from the stored values and its existing test (key
      absent before and after the JSON round trip) still passes. Files:
      `crates/app/ui/src/main.ts`, `crates/app/ui/src/companion.ts`.
- [ ] Skip the second model run in `eyes_of` for a file whose scan already
      stored the EAR and pose (the `mesh-eyes-index` work stores them, and
      the meta pane reads them). `eyes_of` still runs for every shown file
      because the mesh overlay (`drawFaceMesh`) needs the 478 points, and it
      is the fallback for files with no stored value (face under 60 px, or
      a file pass 2 has not reached). Reusing the scan's mesh needs the points stored or a path that
      runs the model only while `f` is on. Files: `crates/app/src/commands.rs`
      (`read_eyes`), `crates/backend/src/index.rs`, `crates/core/src/eyes.rs`.
- [ ] Suggest the sharpest-eye frame within a burst group. Measured and not
      shipped (`docs/plans/20261008-burst-keep-score/`, Decision 1, `results.md` and `fit.md`): the user
      picks within a burst by timing, not by a technical feature; the
      burst's length, not sharpness or the eyes, predicts a kept scene
      (79% held out at 15 or more frames, which the burst band's count
      already shows, against about 56% for the best technical rule); and
      the non-picks are mostly technically fine. Revisit only with a new
      signal; the per-frame good-photo mark above covers "is this a miss".
- [ ] Spot-check whether the sharpness ranking within a burst changes now
      that Sony frames with face tracking are scored on the camera's AF frame
      instead of YuNet's eye midpoint. Needs a per-file score output
      (`riffle-cli scan` prints none). Files: `crates/core/src/sharpness.rs`,
      `crates/core/src/scan.rs`, `crates/cli/src/main.rs`.
- [ ] Crop detection can false-positive on printed faces or logos near the AF
      point (e.g. `_DSC3632`: a shirt logo scored 0.67), yielding a false
      candidate (or a false not-a-candidate) when the logo is the face nearest
      the AF point. Consider a size or aspect filter on crop boxes. Files:
      `crates/core/src/faces.rs` (`detect_around`),
      `crates/core/src/candidate.rs`.
- [ ] AF on a person in the background gives a sharp face and a false focus
      candidate: the cue says the face nearest the AF point is sharp, not that
      it is the subject. Files: `crates/core/src/candidate.rs`.
- [ ] Back-of-head and upturned faces find no face in the crop, so their
      focus candidate state is unknown (white), even when the camera tracked
      them. Files: `crates/core/src/candidate.rs`, `crates/core/src/faces.rs`.

Related: `crates/core/src/sharpness.rs`, `crates/core/src/faces.rs`, `crates/core/src/arw.rs`, `crates/backend/src/index.rs`, `crates/app/ui/src/burst.ts`.

### App: real-device check of the face boxes on the whole-image detection path

The face-detection-recall-cost work moved the whole-image face search (files with no trusted AF point, such as every Leica M file) to a 640x448 YuNet input on a DCT-scaled decode, and bumped `FACES_VERSION` to 5. `riffle-cli detect` / `candidates`, the unit tests and the `riffle-cli faces` PNGs cover the detector and the coordinate mapping back to stored preview pixels. The app GUI (the `f` mark drawing `faces_of` boxes after the second pass re-runs) was never exercised. Plan: `docs/plans/_archived/20261005-face-detection-recall-cost/plan.md`.

Files: `crates/core/src/faces.rs` (`detect_whole_upright`, `scaled_to_stored`), `crates/app/src/commands.rs` (`faces_of`), `crates/backend/src/index.rs` (`FACES_VERSION`).

#### TODO

- [ ] On Windows or macOS, open `D:\photos\2026\2026-02-01` in the app after updating and let the second pass re-run (`FACES_VERSION` 5). Press `f` on `L1005161.DNG`: seven face boxes should be drawn (all seven people, as `D:\Photos\tests\2026-10-06-face-recall\step3\png\L1005161.png` shows), on the stored preview's scale (boxes on the faces, not shrunk to the top-left 3/8). Press `f` on `L1005233.DNG` (portrait): no boxes.

### App: real-device check of the meta pane's `Eyes` row, and the review of the closed-eyes labels

`closed-eyes-detection` (`docs/plans/_archived/20261007-closed-eyes-detection/plan.md`) added `eyes_of`, which judges the shown file's eyes with MediaPipe Face Landmarker v2, and the meta pane's `Eyes` row (since `eyes-open-probability`, `docs/plans/_archived/20261007-eyes-open-probability/plan.md`, shown as `Eyes open: NN%`, the open probability `1 - p`, and since `burst-keep-score` Step 4b as `Open · NN` / `Closed · 0`, the openness 0-100 from the EAR). CI covers the unit tests (`eyes.test.ts`, `meta.test.ts`, the core `eyes` tests), and the `#[ignore]`d `times_eyes_of_on_real_files` timed the backend; the app itself was never run with it, so the preview-to-row time in `docs/humans/performance.md` "Closed-eyes judgment on demand (Windows 11)" is missing. Steps 1 and 3 were ticked on the automated criteria. The truth set the threshold (0.137) and the AUCs come from was labeled by the agent, and the user's review of its `closed` faces is still open.

Files: `crates/app/src/commands.rs` (`eyes_of`, `read_eyes`), `crates/app/ui/src/main.ts`, `crates/app/ui/src/eyes.ts`, `crates/app/ui/src/meta.ts`, `crates/core/src/eyes.rs`, `docs/plans/_archived/20261007-closed-eyes-detection/eyes-truth.md`.

#### TODO

- [ ] On Windows 11, in a `mise run dev` build with Settings > `Timing logs` on, open `D:\photos\2026\2026-09-19` and wait for the scan to finish. Select `_DSC1889.ARW` or `_DSC1890.ARW` (labeled closed): the Analysis group shows `Eyes  Closed · 0` after `AF eye in focus`. Select `_DSC1894.ARW`: `Eyes  Open · NN`. Select `_DSC1897.ARW` (no face judged): no `Eyes` row.
- [ ] Open `D:\photos\2026\2026-02-01`, let the second pass re-run (`FACES_VERSION` 9) and select `L1005161.DNG` (closed) and `L1005155.DNG` (open): the `Eyes` row shows the value pass 2 stored for the largest confident face (L1005161 `Closed · 0`, L1005155 `Open · NN`), and the strip icon and the filter's `Face focus` state follow the same face.
- [ ] Hold the page key through 30 files of the ARW folder: no row of a previous file stays on a later one, and the preview keeps pace with no added stall against the previous build.
- [ ] With the folder idle, read the `eyes total=... read=... decode=... detect=... model=... ipc=...` lines in `Riffle.log` for a few ARWs with an AF point: the row should appear within ~150 ms of the preview. Add the numbers (and the preview-to-row time) to `docs/humans/performance.md` "Closed-eyes judgment on demand (Windows 11)" and `performance.ja.md`. Since `face-mesh-overlay` (`docs/plans/_archived/20261007-face-mesh-overlay/plan.md`) the response also carries 478 mesh points (~8-10 KB of JSON): confirm the stages still add up to about `total` and `ipc=` stays within a few ms of the figure before that change (not measured; `crates/app/src/commands.rs` `read_eyes`, `crates/app/ui/src/main.ts` `requestEyes`).
- [ ] Review the faces labeled closed: for each file in the "Faces with a closed eye" list of `docs/plans/_archived/20261007-closed-eyes-detection/eyes-truth.md`, open its tile `D:\Photos\tests\2026-10-07-closed-eyes\tiles\<stem>.png` (or the crops under `arw\` / `dng\`) and confirm that the eye marked `c` shows no iris; note any that are open. If labels change, re-run `riffle-cli eyes` on `labeled-paths.txt` and `scratch\auc.py` there, and re-fit the threshold and the slope in `crates/core/src/eyes.rs` if the best F1 moves.

### App: real-device check of the face mesh overlay on the judged face

`face-mesh-overlay` (`docs/plans/_archived/20261007-face-mesh-overlay/plan.md`) returns the 478 face-mesh points from `eyes_of` and drew MediaPipe's `FACEMESH_TESSELATION` (1322 edges) over the judged face while `f` is on; on real files the tessellation was seen to bury the face. `face-mesh-parts-overlay` (`docs/plans/20261007-face-mesh-parts-overlay/plan.md`) replaced it with the face parts outline (`FACEMESH_CONTOURS` + `FACEMESH_NOSE`, 149 edges, `FACE_OUTLINE_EDGES`) and, when the `Eyes` judgment is open, the two iris rings (`FACEMESH_IRISES`, 8 edges, `FACE_IRIS_EDGES`) with a dot at each center (`FACE_IRIS_CENTERS`, points 468 and 473). CI covers the tables' shape, `meshEdges`'s open / closed rule, the `meshPoints` scaling and centering, the core point mapping and the `Eyes` fixtures. The canvas drawing itself (`drawFaceMesh`) and its line widths (3 px outline at 0.8 black, then 1.5 px `FACE_MARK_COLOR`, chosen without a GUI again) and the iris dot size (`FACE_MARK_EYE_RADIUS`) were never seen on a real file. The step was ticked on the automated criteria.

Files: `crates/app/ui/src/main.ts` (`drawFaceMesh`, `FACE_MESH_OUTLINE_WIDTH`, `FACE_MESH_LINE_WIDTH`), `crates/app/ui/src/facemesh.ts` (`FACE_OUTLINE_EDGES`, `FACE_IRIS_EDGES`, `FACE_IRIS_CENTERS`, `meshEdges`).

#### TODO

- [ ] On Windows or macOS, with `f` on, show (1) an upright ARW with an open-eyed face of 60 px or more: the cyan outline of the face parts and both iris rings with their center dots sit on the judged face a moment after the `Eyes` row appears; (2) a portrait ARW (orientation 6 or 8) with such a face: the same, rotated with the image and scaling with the window; (3) a file labeled closed (`D:\photos\2026\2026-09-19\_DSC1889.ARW` / `_DSC1890.ARW`, per the closed-eyes todo above): the outline only, no iris ring and no iris dot; (4) a file whose only face is below 60 px on the preview: nothing drawn and no error in `Riffle.log`. Tune `FACE_MESH_OUTLINE_WIDTH` / `FACE_MESH_LINE_WIDTH` (or give the iris dots their own radius) if the outline competes with the face box or the rings collapse onto the dots.

### App: real-device check of the meta pane's `Yaw`, `Pitch` and `Roll` rows

`head-pose` (`docs/plans/_archived/20261007-head-pose/plan.md`) ports MediaPipe's face geometry to `crates/core/src/pose.rs` and returns the yaw, pitch and roll of the judged face from `eyes_of` (`EyesJudgment.pose`). The meta pane's Analysis group shows it as `Yaw`, `Pitch` and `Roll` rows after `Eyes` (one `Head pose` row until `split-head-pose-rows`, `docs/plans/20261021-split-head-pose-rows/plan.md`). CI covers the unit tests (`pose.rs`, `meta.test.ts`, the `EyesJudgment` serialization), and the core solve was timed at about 8 µs per face. The app itself was never run with the rows. The Step 3 checkbox was ticked on the automated criteria.

Files: `crates/app/src/commands.rs` (`read_eyes`, `EyesJudgment`), `crates/app/ui/src/eyes.ts`, `crates/app/ui/src/meta.ts` (`metaGroups`), `crates/core/src/pose.rs`.

#### TODO

- [ ] On Windows, in a `mise run dev` build with Settings > `Timing logs` on, open `D:\photos\2026\2026-09-19`. Select a frontal ARW: the Analysis group shows `Yaw  N°`, `Pitch  N°` and `Roll  N°`, in that order, after `Eyes`, with |yaw| small. Select an ARW labeled oblique in `docs/plans/_archived/20261007-head-pose/pose-truth.md`: the yaw sign matches the labeled direction (right positive, up positive for pitch, clockwise positive for roll).
- [ ] Open `D:\photos\2026\2026-02-01`, let the second pass re-run (`FACES_VERSION` 9) and select a DNG with a judged face: the `Yaw`, `Pitch` and `Roll` rows show the pose pass 2 stored for the largest confident face.
- [ ] Select a file with no judged face (for example `_DSC1897.ARW`): none of `Eyes`, `Yaw`, `Pitch` and `Roll` shows.
- [ ] Read the `eyes total=` lines in `Riffle.log` for a few ARWs: the total is not noticeably longer than before (the solve adds about 8 µs).

### App: the review of the head-pose labels is still open

`head-pose` (`docs/plans/_archived/20261007-head-pose/plan.md`) measured the angles against 190 labels (159 readable) that the agent made from the face crops: yaw class and direction, pitch, roll, or `x`. The user's review is pending, and the plan's Decision rests on these labels. Step 2 was ticked on the automated criteria (the labels exist, the measurement and Decision are written, `mise run ci` passes).

Files: `docs/plans/_archived/20261007-head-pose/pose-truth.md`, `docs/plans/_archived/20261007-head-pose/pose-results.md`.

#### TODO

- [ ] Also check hand check 16 of `docs/plans/_archived/20261008-burst-keep-score/results.md`
      (`D:\photos\2026\2026-07-05\_DSC2445.ARW`): a profile looking up behind a ball
      reads pitch -68 deg. Decide whether `crates/core/src/pose.rs` should call that
      unknown or whether the label stands.
- [ ] On Windows, open the sheets `D:\Photos\tests\2026-10-07-head-pose\sheets\s00.png` to `s16.png` (4 tiles across, row by row in the `#` order of the `pose-truth.md` table; crops in `crops\`) and compare each tile to its row. Note any label to change. If labels change, re-run `aggregate.py` (in `D:\Photos\tests\2026-10-07-head-pose\`) and update `pose-results.md` and the plan's Decision if a number moves.

### Core: the no-AF-point path decodes the preview up to three times

#### Background

On a file with no trusted AF point, pass 2 decodes the preview up to three times: the DCT-scaled RGB decode for the whole-image face search, a full-size RGB decode for the face mesh and the eye regions of the judged face (since `docs/plans/_archived/20261010-af-eye-good-bad-cascade/` Step 2, only when a confident face is found), then a full-size grayscale decode in `sharpness::score_preview`. The face-detection-recall-cost work cut the first decode to 3/8. Step 2 measured on one thread (`riffle-cli features`, `D:\photos\2026\2026-02-01` and `2026-09-05`, Leica DNG): a faced no-AF file went from about 94 ms to about 122 ms of analysis, a file with no face did not change; the numbers are in that plan's `learnings.md`. Scoring from the full-size RGB decode (its luma) would drop the grayscale decode on faced files, but it changes the score's pixels and its stored values, so it needs a `FACES_VERSION` bump. See also `docs/plans/_archived/20261005-face-detection-recall-cost/learnings.md` (Step 3 measurements).

Files: `crates/core/src/scan.rs` (`extract_analysis_unless`, `whole_image_cue`, `score`), `crates/core/src/sharpness.rs` (`score_preview`).

#### TODO

- [ ] Score a faced no-AF-point file from the full-size RGB decode the cue already made (its `candidate::luma`) instead of `score_preview`'s own grayscale decode. Measure the per-file saving with `riffle-cli features` on one thread over `D:\photos\2026\2026-02-01` and `2026-09-05` before and after, check that the score's ranking within bursts holds on those folders (Spearman per burst against the old score), and bump `FACES_VERSION`. Files: `crates/core/src/scan.rs`, `crates/core/src/sharpness.rs`, `crates/backend/src/index.rs`.

### Agents: confirm `pr-runner` waits for a child's hand-back on a real `/pr` run

#### Background

The `long-wait-timeouts` fix is confirmed on 240 `pr-runner` and 225 `merger`
transcripts: every `wait-pr-actionable.sh` / `wait-post-merge-runs.sh` call
passed `timeout: 600000`, and the runs the harness still moved to the
background were `TaskStop`ped and re-run in the foreground. What remained was
`pr-runner` polling (`wait-pr-actionable.sh`, `sleep`, `until` / `while`,
`git ls-remote`, `gh pr view`) while a child it had dispatched was still
running. The `pr-runner-child-handback` plan added a rule to
`.claude/agents/pr-runner.md` that the runner ends its turn after an `Agent`
dispatch and waits for the child's hand-back. It is verified only statically.

#### TODO

- [ ] On the next real `/pr` run whose PR hits a conflict, a CI failure or a
      review round, read the `pr-runner` subagent transcript (the subagent
      `.jsonl` files under the Claude Code project directory for that
      session) and confirm that after each `Agent` dispatch the runner ended
      its turn, the next event is the "[Subagent hand-back]" message, and
      there is no `wait-pr-actionable.sh`, `sleep`, `until` / `while`,
      `git ls-remote` / `git fetch` or `gh pr view` call in between. If not
      clean, adjust `.claude/agents/pr-runner.md`.

### Agents: confirm `merger` runs its step 6 commands as separate, verbatim calls on a real `/pr` run

#### Background

The `merger-separate-sync-calls` plan added a rule to `.claude/agents/merger.md` step 6: `mise run git:main` and `./tools/git/delete_merged_branches.sh` run as two separate Bash calls, each exactly as written in its code block (no pipes, no `2>&1`, no `tail`, no `;` / `&&` chaining, no `cd` prefix). The rule exists because the chained or piped forms match neither allow rule in `.claude/settings.json`, so they fall to the auto mode classifier, which can deny the sandbox bypass as a Safety Bypass. The merger then stops with `post_merge_sync_failed` after a merge that already succeeded (PR #657). CI covers only the Markdown formatting. A prose rule cannot be unit-tested, so it has never been exercised by a real unattended merger run. Plan: `docs/plans/_archived/20261003-merger-separate-sync-calls/plan.md`

Files: `.claude/agents/merger.md`

#### TODO

- [ ] On the next real `/pr` run that reaches the merger, read the `merger` subagent transcript (the subagent `.jsonl` files under the Claude Code project directory for that session). Confirm that step 6 issued `mise run git:main` and `./tools/git/delete_merged_branches.sh` as two separate Bash calls, each with exactly the text in its code block and `dangerouslyDisableSandbox: true`, and that the run did not end with `post_merge_sync_failed`. If a pipe, redirect or chain appears, or a denial recurs, strengthen the wording in `.claude/agents/merger.md` step 6.

### Agents: confirm `pr-runner` runs its skill scripts verbatim as standalone calls on a real `/pr` run

#### Background

The `pr-runner-verbatim-script-calls` plan extended `.claude/agents/pr-runner.md` §4 so that every script under `.claude/skills/**/scripts` it runs, the `wait-pr-actionable` call above all, is run exactly as written in its code block as a standalone Bash call: no pipes, no `tail`, no redirects, no `; echo exit=$?`, no `${PIPESTATUS[...]}`, no `;` / `&&` chaining. The rule exists because of PR #671, where `bash .claude/skills/pr/scripts/wait-pr-actionable.sh 671 | tail -12; echo exit=${PIPESTATUS[0]}` missed the `Bash(bash .claude/skills/pr/scripts/*)` allow rule and waited about 62 minutes before it ran. The script then printed `ACTION=ready` at about 150 s, but the call did not return until the 600 s timeout, most likely because a lingering child held the pipe open. A bare re-run returned in 2 s. CI covers only the Markdown formatting. A prose rule cannot be unit-tested, so it has never been exercised by a real unattended pr-runner run. Plan: `docs/plans/_archived/20261004-pr-runner-verbatim-script-calls/plan.md`

Files: `.claude/agents/pr-runner.md`

#### TODO

- [ ] On the next real `/pr` run, read the `pr-runner` subagent transcript (the subagent `.jsonl` files under the Claude Code project directory for that session). Confirm that every `.claude/skills/**/scripts` call, especially `wait-pr-actionable.sh`, has exactly the text in its code block with no pipe, `tail`, redirect, `; echo exit=$?`, `${PIPESTATUS[...]}` or chaining, and that none waited long before starting or was moved to the background after the script finished. If a decorated call appears, strengthen the wording in `.claude/agents/pr-runner.md` §4.

### Agents: `merger.md`'s post-merge paragraph calls backgrounding "impossible"

#### Background

The `agent-foreground-rationale` plan replaced the inaccurate "a subagent exits the moment its turn ends" rationale in the agent files. It left `merger.md`'s post-merge paragraph alone to keep the change surgical. That paragraph still says backgrounding is "impossible for the same reason as step 1". With the corrected reason, backgrounding is ruled out (the foreground run keeps the exit code observable to branch on), not impossible. Plan: `docs/plans/_archived/20261007-agent-foreground-rationale/plan.md`

Files: `.claude/agents/merger.md`

#### TODO

- [ ] Reword the post-merge paragraph so it no longer calls backgrounding "impossible". Its wording should agree with step 1 and with `pr-runner.md` §4.

### App: SIGMA fp L strip thumbnails have over ten times the pixels of other bodies'

SIGMA fp L DNGs have no strip JPEG at or above `PREVIEW_MIN_WIDTH` (1600)
below the 9520x6328 full-size one, so the preview tier falls back to the
full-size JPEG. The strip does not decode that per thumbnail: at scan time
`thumbnail_jpeg` decodes the preview tier at 2/8 scale and re-encodes it,
the index caches the result, and the strip's `thumbnail` command reads the
cache. But 2/8 of 9520x6328 is about 2380x1582 (about 3.8 MP), more than
ten times the pixels of the ~404x270 thumbnail of other bodies, so each
strip cell costs more to decode and to ship over IPC. Each scan also
decodes the ~60 MP JPEG (at 2/8 scale) once per file. OM System / Olympus
ORFs have the same cost on a smaller scale: their only JPEG is 3200x2400, so
the fixed 2/8 gives 800x600 thumbnails, about four times the pixels of the
~404x270 ones (`crates/core/src/orf.rs`; decided in
`docs/plans/_archived/20260930-olympus-orf/plan.md` to leave `scan.rs`
alone for now). Files: `crates/core/src/decode.rs` (`thumbnail_jpeg`), `crates/core/src/scan.rs`,
`crates/backend/src/index.rs`, `crates/app/src/commands.rs` (`thumbnail`),
`crates/core/src/arw.rs` (`PREVIEW_MIN_WIDTH`, tier selection).

#### TODO

- [ ] Measure strip thumbnail load time on a SIGMA fp L folder (per-cell
      decode and IPC).
- [ ] Candidate fix, not decided: cap the thumbnail size, e.g. pick a
      larger DCT scale-down (`d.scale(n)`) when the preview is large, or
      resize to the normal thumbnail width.

### App: SIGMA fp L main view appears late on Linux

Since #383 and #385, the main view for SIGMA fp L DNGs displays on Linux
(WebKitGTK), but noticeably later than for other files. SIGMA fp L has no
mid-size embedded JPEG, so the `preview` payload is the 9520x6328 (about
60 MP, about 28 MB) full-size JPEG. On Linux the decode worker must pass
`createImageBitmap` resize options to fit it under `PREVIEW_PIXEL_LIMIT`
(6 MP), because WebKitGTK draws transferred bitmaps of about 6.87 MP or more
transparent. A MiniBrowser run on a synthetic 60 MP JPEG (not the app) took
about 200-400 ms per resized decode, on top of the IPC of the ~28 MB
payload; the real app is unmeasured. Same root cause as "App: SIGMA fp L
strip thumbnails have over ten times the pixels of other bodies'" (strip
side), and the Linux/SIGMA-specific case of "App: unmeasured end-to-end
per-page latency".
Files: `crates/app/ui/src/worker.ts` (resize on decode),
`crates/app/src/commands.rs` (`PREVIEW_PIXEL_LIMIT`).

#### TODO

- [ ] Measure first on the real app on Linux with `Timing logs` on: the
      `page read=… ipc=… post=… worker=… decode=… draw=… total=…
      keypressToPixels=…` line splits the read and IPC from the decode.
- [ ] Candidate fixes, none decided: prefetch and decode the neighbouring
      pages ahead; or have the backend downscale and cache a mid-size JPEG
      for files lacking one, which would serve the main view and, as a side
      effect, shrink the thumbnail noted in the SIGMA fp L strip item.

### Core: Nikon DX-crop and non-standard-crop AF point mapping is unverified

`nef::parse`'s Nikon `AFInfo2` AF point is read as top-left coordinates in
the `AFImageWidth` / `AFImageHeight` frame, assumed to already match the
JPEG's crop. No DX-crop NEF sample was available to confirm this: exiftool's
`Nikon.pm` hints that some DX-mode results are reported in FX (full-frame)
coordinates, in which case the point would land off by the crop factor when
`AFImageWidth` stays the FX size while the JPEG is the DX crop. Found in the
Step 5 sample survey of
`docs/plans/_archived/20260929-canon-nikon-raw/plan.md`. Files:
`crates/core/src/nef.rs`.

#### TODO

- [ ] Find a Nikon DX-crop NEF sample (or a full-frame Z body shot in DX
      crop mode) and confirm whether the AF point position and
      `AFImageWidth` / `AFImageHeight` are already in DX coordinates or
      need scaling by the crop factor; fix `nef.rs` if the latter.

### Core: widen camera support from public sample RAW files

The user no longer owns the Sigma fp L or BF and cannot shoot new
samples, so public sample files are the verification source for more
cameras. Two kinds exist: raw.pixls.us (CC0; a few files per camera,
often flat test scenes that are weak for AF-point checks; CC0 means a
small file could be committed as a fixture, but tests prefer synthetic
bytes as in `crates/core/src/arw.rs`) and review-site sample galleries
(real scenes, good for AF checks, but not redistributable, so local
verification only). Riffle reads ARW, DNG, NEF, CR3, RAF and ORF today (README.md
"RAW formats"). The work splits into tiers, cheapest first:

1. More DNG-writing cameras (Pentax, Ricoh GR, other Leica bodies,
   phones). DNG reading already exists, so only preview/EXIF
   extraction needs verifying on samples.
2. AF point from MakerNotes that exiftool already decodes (Canon
   `AFInfo`, Nikon `AFInfo2`, Fujifilm `FocusPixel`, Olympus
   `AFPointSelected`, Panasonic `AFPointPosition`). Tag meaning and
   coordinate system are known; samples only confirm. This presupposes
   the RAW container is readable (tier 3), except for bodies that
   write DNG.
3. New RAW containers (CR3 = ISOBMFF, NEF, RAF, ...). Each needs a new
   parser next to `crates/core/src/arw.rs`; implementation outweighs
   verification.

AF data exiftool does not decode needs inference from many off-center
samples, as done for the Sigma BF `0x0147` in
`docs/plans/_archived/20260924-sigma-bf-af-point/plan.md`. Files:
`crates/core/src/arw.rs`, `crates/core/src/reader.rs`, `README.md`.

#### TODO

- [ ] Tier 1: verify preview/EXIF extraction on public DNG samples from
      Pentax, Ricoh GR, other Leica bodies, and phones; add each
      working body to the `docs/humans/cameras.md` table.
- [ ] Tier 2: read the AF point from the exiftool-decoded MakerNote
      tags above, confirming on samples, for bodies whose container
      Riffle can already read. Canon `AFInfo2` (EOS bodies) and Nikon
      `AFInfo2` versions `03xx` / `04xx` (Z bodies) are done
      (`crates/core/src/cr3.rs`, `crates/core/src/nef.rs`). Left: the
      Nikon DSLRs' `AFInfo2` `0100` / `0101` (D850, D500), whose AF point
      is a grid point name rather than a position, and a sample with an
      AF position from the Nikon Z 8 and the Canon EOS R6 (their
      raw.pixls.us samples carry none). Fujifilm `FocusPixel` on the RAF
      bodies is done too (`crates/core/src/raf.rs`, in the embedded JPEG's
      frame); an autofocus sample from the X-T3 and the GFX 100 is still
      missing (theirs are manual focus). Next: the OM
      System / Olympus ORF bodies (`crates/core/src/orf.rs`): CameraSettings
      0x030a `AFTargetInfo` on the OM bodies (a 640x480 frame with the focus
      and selected areas) and 0x0305 `AFPointSelected` (percentages) on the
      Olympus bodies. Every raw.pixls.us sample puts the point near the
      center, so the origin and orientation of the frame are unconfirmed; an
      off-center landscape and a portrait sample are needed (see the Step 2
      survey in `docs/plans/_archived/20260930-olympus-orf/learnings.md`).
- [ ] Tier 3: decide per container whether a new parser is worth it,
      given the samples available. CR3, NEF, RAF and ORF are done
      (`crates/core/src/cr3.rs`, `crates/core/src/nef.rs`,
      `crates/core/src/raf.rs`, `crates/core/src/orf.rs`); the rest remain.
- [ ] Check the Sigma BF AF point's open assumptions against public BF
      samples: portrait orientation, manual-focus behavior, and the
      1000x667 scale (see the `SIGMA_BF_AF_GRID_W` doc comment in
      `crates/core/src/arw.rs` and the archived plan's
      [Trade-offs and risks](docs/plans/_archived/20260924-sigma-bf-af-point/plan.md#trade-offs-and-risks)).

### Core: a RAF folder scan costs ~222ms per file

#### Background

The `fujifilm-raf` Step 1 survey (`docs/plans/_archived/20260930-fujifilm-raf/learnings.md`) ran `riffle-cli scan` over 46 raw.pixls.us RAFs: 221.8ms per file mean on one thread (290.2ms p95), 36 files/s on 24 threads. The embedded JPEG is 4416x2944 / 4000x3000 (against ARW's 1616x1080 preview), it is decoded whole for the sharpness score, and the face search runs on the whole image on frames with no AF point. The Fujifilm `FocusPixel` is now read (Tier 2 of "Core: widen camera support from public sample RAW files"), so the face search is skipped on autofocus frames while manual-focus frames still search the whole image; a scaled decode for the score would be a separate change. Files: `crates/core/src/scan.rs`, `crates/core/src/sharpness.rs`, `crates/core/src/raf.rs`.

#### TODO

- [ ] Measure the scan on a real RAF folder after the AF point is read, and decide whether the sharpness score should work on a scaled decode of the RAF JPEG.

### Core: M-RAW RAF files are unverified

#### Background

No raw.pixls.us sample of the listed Fujifilm bodies is an M-RAW (multi-image) RAF: the M-RAW header words at `0x48` / `0x4c` are zero on all 46 samples of the `fujifilm-raf` Step 1 survey (`docs/plans/_archived/20260930-fujifilm-raf/learnings.md`). `raf::parse` reads the one embedded JPEG and ignores the M-RAW header, which ExifTool suggests is enough, but no such file has been opened. Files: `crates/core/src/raf.rs`.

#### TODO

- [ ] Open an M-RAW RAF (from a body that writes one) and confirm the preview, the 1:1 view and the EXIF rows.

The `raf-older-bodies` sweep of 228 RAFs from 87 bodies (`docs/plans/_archived/20260930-raf-older-bodies/learnings.md`) found no M-RAW either: `0x48` is zero on all of them.

### Docs: Sony AF point, face tracking and portrait orientation are unconfirmed for lack of samples

#### Background

The `sony-arw-coverage` verification (`docs/plans/_archived/20260930-sony-arw-coverage/plan.md`) marks these `–` in `docs/humans/cameras.md` because the raw.pixls.us samples don't show them:

- Every α9 III and α7CR sample is manual focus (`FocusMode` 0, point at the exact center, `FocusFrameSize` invalid). `trusted_focus` drops the point as designed, so both bodies show `–` for AF point and AF frame size.
- Face tracking: the raw.pixls.us samples hold no `AFTracking` 1. The local samples later checked in `sony-face-tracking-docs` (`docs/plans/_archived/20261001-sony-face-tracking-docs/plan.md`) record it on the α7 IV, α7R V, α7S III, α6700 and ZV-E1, so those show `✓`. The α1, α9 III, α7C II and α7CR stay `–` because no sample of them recorded it.
- No sample is a portrait frame (Orientation 1 on all 66 files), so orientation is unverified on every listed Sony body.

Files: `docs/humans/cameras.md`.

#### TODO

- [ ] Find an AF-C sample of a person from the α9 III and α7CR (portrait orientation if possible), and turn their `–` for AF point / AF frame size into `✓` if the point lands on the subject.
- [ ] Confirm orientation on a portrait ARW sample and record the result in `docs/humans/cameras.md`.

### App: Claude Desktop's MCP connection form is unverified

Whether Claude Desktop accepts a direct `url` entry for a local Streamable HTTP
server was not checked by hand (no Claude Desktop available during
implementation); the settings window currently shows the `npx -y mcp-remote
http://127.0.0.1:41917/mcp` fallback form. Files: `crates/app/ui/src/mcp.ts`,
`crates/app/ui/src/mcp.test.ts`.

#### TODO

- [ ] Verify by hand whether Claude Desktop accepts a direct `url` entry for a
      local Streamable HTTP server and, if so, replace the `mcp-remote`
      example.

### App: the settings window's Copy buttons are unverified across webviews

`navigator.clipboard.writeText` for the MCP settings tab's Copy buttons was not
checked in a running app on Linux (WebKitGTK) or macOS. The same call is used
by the folder tree's `Copy Path` / `Copy Folder Name` context-menu items. A
manual run on Windows (WebView2) on 2026-09-28 passed for the MCP tab's Copy
button and both tree items. Files: `crates/app/ui/src/settings.ts`,
`crates/app/ui/src/main.ts`.

#### TODO

- [ ] Verify the settings window's Copy buttons in the Linux (WebKitGTK) and
      macOS webviews, and confirm the select-text fallback fires when the
      write is refused.
- [ ] Verify the folder tree's `Copy Path` / `Copy Folder Name` items the same
      way on Linux and macOS; if a webview refuses the write, switch to
      `tauri-plugin-clipboard-manager` with a
      `clipboard-manager:allow-write-text` capability.

### App: the MCP `get_view` tool is unverified against a running app

Calling `get_view` from an MCP client (e.g. Claude Code) and checking it
returns the open folder's state was not done by hand; no GUI session was
available during implementation. Files: `crates/app/src/mcp.rs`,
`crates/app/ui/src/main.ts`, `crates/app/ui/src/companion.ts`.

#### TODO

- [ ] Verify by hand that `get_view` returns the open folder's state
      (current file, selection, burst, mode) in a running app.

### App: the MCP `get_photo` / `get_preview` tools are unverified against a running app

Checking that `get_photo` returns the index row and shooting settings, and
that `get_preview` shows the image in an MCP client, was not done against a
running app with a real ARW / DNG folder (no GUI session or sample file was
available). Files: `crates/app/src/mcp.rs`, `crates/core/src/decode.rs`.

#### TODO

- [ ] Verify by hand that `get_photo` and `get_preview` work end to end
      against a running app with a real ARW / DNG folder.

### App: the MCP `show_photo` / `select_photos` / `set_view` tools are unverified against a running app

Whether these tools move the strip, the selection, and the view mode of a
running app from an MCP client was not checked (no GUI session was
available). Files: `crates/app/src/mcp.rs`, `crates/app/ui/src/main.ts`,
`crates/app/ui/src/companion.ts`.

#### TODO

- [ ] Verify by hand that `show_photo`, `select_photos` and `set_view` drive
      a running app's strip, selection and view mode from an MCP client.

### App: `set_judgment` byte-identity with a key press is unverified

Whether a `set_judgment` call from an MCP client writes the XMP / `.dop` with
the same bytes a key press produces, and that `Cmd+Z` undoes it, was not
checked by hand (no GUI session was available); the code path is shared by
construction with the key-press path. Files: `crates/app/src/mcp.rs`,
`crates/app/ui/src/main.ts`, `crates/app/ui/src/companion.ts`.

#### TODO

- [ ] Verify by hand that `set_judgment` from an MCP client writes the same
      XMP / `.dop` bytes as a key press, and that `Cmd+Z` undoes it.

### App: the backend `folder_entries` read is the main cost of the remaining `refreshEntries`

Now that the `scan-done` refresh is skipped when neither the scan nor the reconcile changed anything (docs/plans/_archived/20260926-scan-done-refresh-skip/plan.md), the one refresh that still runs (the open-time one) is dominated by the backend `folder_entries` read: 321 ms cold on 2134 rows in the user's Windows debug-build measurement. Files: `crates/backend/src/index.rs` (`folder_entries` / `AppIndexReader`), `crates/app/ui/src/main.ts` (`refreshEntries`).

docs/plans/_archived/20260928-strip-keep-scroll-on-rescan/plan.md made the frontend `set_files=true` share of the `refresh entries:` timing line cheaper (no cell teardown and reload on a kept-scroll update), but this item is about the backend read, which is untouched.

#### TODO

- [ ] Investigate why the cold `folder_entries` read costs 321 ms on 2134 rows and whether it can be reduced (indexing, query shape, or caching), verified by a measurement with `Timing logs` on before/after.

### App: the manual GUI checks for the resume landing's selection are still open

Step 1 of `docs/plans/_archived/20260928-resume-selection/plan.md` (merged as
#535) collapsed the selection to the landed file when a pending resume
resolves. The user found the bug on a real Windows machine; the manual check
the step specified was not run, since the GUI cannot be driven from an agent
session. Files: `crates/app/ui/src/selection.ts`, `crates/app/ui/src/main.ts`.

#### TODO

- [ ] On Windows, in folder A focus a file other than the first, open folder
      B, then return to A: the status line shows no `selected` suffix, only
      the landed cell has `.selected`, and a rating key changes only that
      file.
- [ ] The same on macOS.
- [ ] The same with a judgment filter that hides the remembered file, so the
      landing falls on a neighbor: exactly that neighbor is selected.
- [ ] The same with a folder that has no index cache (the `catch` branch of
      `refreshEntries` in `main.ts`).

### App: measure whether a `notify` watch blocks deleting a folder on Windows

`docs/plans/_archived/20260928-tree-live-watch/plan.md` (Step 1) measured only
rename on Windows with `notify` 8.2: a watch pins the watched folder's
ancestors against rename, not the folder itself. Delete was never measured,
so `docs/humans/usage.md` and the plan's Trade-offs section say "rename" only.

#### TODO

- [ ] Using the same scratch-binary approach as the Step 1 rename
      measurement, measure on Windows whether deleting a watched folder
      itself, or a folder with a watched descendant, succeeds with `notify`
      8.2. Record the result next to the rename measurement and update
      `docs/humans/usage.md` and `docs/agents/tauri-app.md` to match (or confirm
      they need no change).

### App: real-device checks of Rewrite Sidecars from Index… and Delete Sidecars…

#### Background

The `folder-sidecars` feature added `Rewrite Sidecars from Index…` and
`Delete Sidecars…` to the folder tree's right-click menu (single folder
only), both behind the shared `#sidecar-dialog`. Rewrite marks the folder's
index rows dirty and drains them through the sidecar writer, patching only
the judgment fields of the current format's sidecars; delete moves those
sidecars to the OS trash as one `trash::Runs` run, clears the rows'
judgments, and is undone / redone through `trash_rejected_undo` /
`trash_rejected_redo`. Rust tests (a Lightroom-style `crs:` XMP and a
PhotoLab-style `.dop` patched with the foreign content intact, the sidecar
collection, a sidecar-only run's restore and redo), Vitest and `mise run ci`
cover it; the GUI, Lightroom, PhotoLab and a real Trash were never
exercised. See `docs/plans/_archived/20261005-folder-sidecars/plan.md` and
its `learnings.md`. Files: `crates/app/src/foldersidecars.rs`,
`crates/backend/src/trash.rs` (`redo`), `crates/backend/src/index.rs`
(`rows_of`, `mark_dirty`, `clear_judgments`), `crates/app/ui/src/sidecars.ts`,
`crates/app/ui/src/context.ts`, `crates/app/ui/src/main.ts`
(`rewriteSidecarsIn`, `deleteSidecarsIn`, `trashed`),
`crates/app/ui/src/trash.ts`, `crates/app/ui/src/undo.ts`.

#### TODO

- [ ] On Windows, on a copy under `D:\Photos\tests\<date>-folder-sidecars\`
      of RAWs with a Lightroom-written `.xmp` (with develop settings) and a
      PhotoLab-written `.dop` (with corrections), open the folder in Riffle
      under `XMP and .dop`, rate, flag and label a few files, then
      right-click the folder and choose `Rewrite Sidecars from Index…`.
      Expect the item after the `Expand All` / `Collapse All` group and before
      `Sequence JPEG Timestamps…`; the dialog titled `Rewrite Sidecars from Index` shows the
      judged / unjudged / skipped rows and `Rewrite the XMP and .dop
      sidecars of N files in <name>?` with a primary `Rewrite`; `Rewrite`
      disables both buttons and shows the running note, then the status
      line reads `Rewrote the sidecars of N files in <name>` and the strip
      is unchanged. In Lightroom (`Metadata > Read Metadata from File`) and
      PhotoLab the judgments show and the develop settings / corrections
      survived; a file with a judgment and no sidecar got a new one, an
      unjudged file without one got none.
- [ ] On Windows, on that copy, choose `Delete Sidecars…`. Expect the dialog
      titled `Delete Sidecars` listing `N XMP sidecars (size)` and `N .dop
      sidecars (size)`, ending `Move N sidecars (size) of <name> to the
      Trash?`, with a red `Move to Trash` and an outline `Cancel`; running
      it moves the files to the Recycle Bin, the status line reads `Moved N
      sidecars to the Trash`, and the open folder's stars, flags and labels
      disappear at once. `Edit > Undo` restores them (`Restored N sidecars
      from the Trash`, the judgments come back after the rescan), and
      `Edit > Redo` moves them again and clears the strip. Repeat the undo
      after opening another folder.
- [ ] On Windows, press each item during a scan (switch to a terminal and
      straight back to start one, or a long first scan): the status line
      shows `Rewrite Sidecars from Index: waiting for the scan to finish` /
      `Delete Sidecars: waiting for the scan to finish` and the dialog opens
      when the scan ends, with no `a scan is running` refusal from the
      backend.
- [ ] On Windows, the refusals: `Rewrite Sidecars from Index…` on a folder
      never opened shows `No judgments indexed for <name>; open the folder
      first`; `Delete Sidecars…` on a folder never opened works; on a folder
      with no sidecar of the current format, delete shows `No XMP sidecars in
      <name>` (naming the format); on a JPEG-only folder both show the
      `applies to RAW files only` refusal; none of them opens a dialog.

### App: a JPEG folder's preview decodes 15-20x longer than an ARW's

Backend measurement (docs/plans/_archived/20260927-jpeg-view-only/learnings.md,
Step 2): a 21.5 MP JPEG's `read_preview` + full `decode_rgb` took ~197 ms
and a 14.8 MP one ~141 ms, versus ~9-15 ms for an ARW/DNG preview. The
plan's remedy (a DCT-scaled, capped re-encode) was measured and found
slower than shipping the whole file, so `preview` sends the whole JPEG as
is.

#### TODO

- [ ] Find a payload that skips the full decode/encode round trip, e.g. a
      DCT-scaled decode (4/8 or 3/8) sent as raw pixels, or reading a
      JPEG's embedded Exif thumbnail first. Verify with `Timing logs` in the
      GUI before choosing. Files: `crates/app/src/commands.rs` (`preview`),
      `crates/core/src/decode.rs`, `crates/app/ui/src/` decode worker.

### App: PhotoLab Uuid lookup on macOS: real-device checks and external volumes

`crates/backend/src/photolab.rs` looks up the registered Uuids on macOS too
(`docs/plans/_archived/20261005-photolab-uuids-macos/plan.md`), verified by
hand on macOS (PhotoLab 10, 2026-10-05; see `docs/agents/photolab.md`). Paths
under `/Volumes` get no lookup yet, so a fresh `.dop` there still gets random
Uuids and may become a virtual copy.

#### TODO

- [ ] Real-device check: give a source a virtual copy in PhotoLab and confirm
      the master is the lowest `ZDOPINPUTITEM.Z_PK`.
- [ ] External volumes: open a folder on `/Volumes/X` in PhotoLab, find how it
      records the volume (`ZTYPE`, `ZNAME`, `ZUNIQUEID` in `ZDOPFOLDER`), then
      extend `folder_names` / `query`'s macOS mapping in
      `crates/backend/src/photolab.rs`.

### App: PhotoLab after a file or folder rename

From Step 3's check of the PhotoLab Uuid lookup (`crates/backend/src/photolab.rs`
queries `Sources` by `FolderId` and `Name`; `crates/core/src/dop.rs` writes
the RAW's `Name` into the `.dop`). `rename_file`
(`crates/app/src/rename.rs`) moves the `.dop` unchanged, so its inner `Name`
still names the old file and PhotoLab's database still holds the old name
and folder.

#### TODO

- [ ] Find out (checked with PhotoLab) whether PhotoLab re-matches a renamed
      RAW with its moved `.dop` or imports it as a new image / virtual copy,
      and either patch the `.dop`'s `Name` on rename or document what to
      expect.

### App: real-device check that an inline rename is refused while its rename is in flight

#### Background

`rename-in-flight-guard` made the folder tree's `startRename` and the strip's
`startRename` refuse to open the inline editor, with a status note, on a row
or cell whose `rename_folder` / `rename_file` invoke is in flight (the tree
reuses `A folder is being renamed; try again in a moment.`, the strip says
`This file is being renamed; try again in a moment.`), and made a pending
mark clear only from the rename that set it. Vitest covers the in-flight
check for a file path and the identity-keyed pending slot, and `mise run ci`
passes. The GUI behavior was never run on a real device. A rename that is
still held behind a scan must still be re-editable as before. Plan:
`docs/plans/_archived/20261007-rename-in-flight-guard/plan.md`. Files:
`crates/app/ui/src/folders.ts`, `crates/app/ui/src/strip.ts`,
`crates/app/ui/src/main.ts`.

#### TODO

- [ ] On macOS, confirm a folder rename in the tree and a file rename in the
      strip while a scan runs, let the scan end, and while `rename_folder` /
      `rename_file` is still running (a large folder or a slow drive) try
      `Rename…` and the slow second click on the same row / cell: the editor
      does not open and the status line shows `A folder is being renamed; try
      again in a moment.` / `This file is being renamed; try again in a
      moment.`; once the rename settles, editing works again. Also re-check
      that a rename still held behind the scan can be re-edited (replace,
      cancel, keep) as before, in both views.
- [ ] On Windows, run the same check as above.

### App: the wait-for-scan manual checks for Move to Trash and Rename are still open

#### Background

`wait-for-scan` made Move Rejected to Trash (now the strip's
`Move to Trash…`, which replaced it in `trash-from-strip`), the folder / file
`Rename…` and the settings modal's `Clear Cache` wait for a running scan
instead of refusing with `a scan is running; wait for it to finish`: the
frontend's `IdleGate` holds one pressed operation (a second press replaces
it), the status line names what is waiting, and the held operation runs when
the scan ends. The backend `SCAN_RUNNING` refusals (`crates/app/src/commands.rs`
for trash, `crates/app/src/rename.rs` for renames) remain as the last line of
defense. None of it has been run by hand; the user's Windows setup is where
these checks will
first be run. The trash confirmation is an in-window HTML dialog that asks
before the wait, so the archived plan's "closing the
confirm dialog refocuses the window" race no longer applies to trash, only to
Clear Cache's native confirm. The Clear Cache checks live in
`### App: the Clear Cache button's manual GUI verification is still open`
and are not repeated here. Basis: `wait-for-scan`
(`docs/plans/_archived/20260928-wait-for-scan/plan.md`, #519 and #522).
Files: `crates/app/ui/src/idle.ts`, `crates/app/ui/src/main.ts` (`whenIdle`,
`settleIdle`, `trashFilesIn`, `renameFolder`, `renameFile`, the
`faces-done` `idle.drain()` and the `openDirectory` `idle.discard()`),
`crates/app/ui/src/strip.ts` (`setFiles` carrying the live inline edit across
a rebuild).

#### TODO

- [ ] In an ARW folder, switch to a terminal and straight back (the
      `tauri://focus` `resync()` starts a rescan), right-click a strip cell,
      choose `Move to Trash…` and confirm: the dialog closes, the status
      line briefly shows `Move to Trash: waiting for the scan to finish`,
      and the file moves when the scan ends with no error.
- [ ] During a long first scan (thousands of files) confirm the strip's
      `Move to Trash…`: the status line stays visible for the whole scan
      and the files move when the scan ends.
- [ ] Switch folders while an operation waits: it is dropped
      (`idle.discard()` in `openDirectory`) and the status line clears.
- [ ] Rename a folder (tree inline edit) and a file (strip inline edit)
      during a scan: the edit starts at once, the status line shows
      `Rename…: waiting for the scan to finish`, the rename runs when the
      scan ends, and, when the renamed folder is the open one (or contains
      it), the folder reopens at the new path; a file rename keeps the
      cell's marks under the new name; an inline edit still being typed
      survives the strip being rebuilt by the scan's `setFiles` without
      losing typed text or confirming early.
- [ ] Pressing `Move to Trash` in the trash dialog (`trash_files`
      through `settleIdle`) and confirming a rename never make the backend
      refuse with `a scan is running`; the native-confirm refocus case
      (closing a dialog refocuses `main` and `resync()` starts a scan) is
      Clear Cache's and is covered by that item's check (3).
- [ ] The pending rename display (`pending-rename-display`,
      `docs/plans/_archived/20261006-pending-rename-display/plan.md`), in both the
      folder tree and the strip: a rename confirmed during a scan shows the
      new name muted and italic after the clock icon (hover: `Renames when
      the scan finishes`) at once, and the normal name when it runs; pressing
      Move to Trash (or another rename) meanwhile, or opening
      another folder, reverts the name with `Rename… was canceled`; a rename
      that fails at the scan's end reverts with its error; re-editing the
      pending cell starts from the pending name, a different name replaces
      it, the original name reverts it with `Rename… was canceled`, and
      `Escape` keeps it; on a labeled strip cell the name stays in the
      label's text color, italic with the clock; the pending row still opens
      its folder and the pending cell is still judged.

### App: real-device check of the File menu separator on Windows

`file-menu-separator` (docs/plans/_archived/20260929-file-menu-separator/plan.md)
fixed the doubled separator in the Windows and Linux File menu by inserting
Settings... and Check for Updates… at `own.len()`, the index right after the
prepended File items, in `app_menu::build`. CI (`mise run ci`) covers the build,
but the rendered menu was never looked at on a real machine, and Step 1 was
ticked on the automated criteria only. Linux follows the same code path but was
not built here.

Files: `crates/app/src/main.rs` (`app_menu::build`).

#### TODO

- [ ] On Windows, run the app and open the File menu: it reads Open Folder,
      Reload Folder, ─, Settings..., Check for Updates…, ─, Close Window, Quit,
      with a single separator between each group (no doubled separator).
- [ ] On Linux, open the File menu and confirm the same order and the single
      separators.

### App: real-device visual checks for the shadcn Neutral dark restyle are still open

#### Background

The `ui-design-tokens` feature moved the whole frontend onto shadcn/ui's Neutral dark tokens and shared component classes, and gave the settings modal a side navigation. CI and the unit tests cover class names, `nextTab` (vertical) and `theme.ts`. Nobody has looked at the result on a real machine: every step's checkbox was ticked on the automated criteria only, so the Windows passes for Steps 1 to 6 and the macOS / Linux passes were never run. The plan is `docs/plans/_archived/20260929-ui-design-tokens/plan.md`; the styling rules are in `docs/agents/ui-styling.md`.

Files: `crates/app/ui/style.css`, `crates/app/ui/index.html`, `crates/app/ui/src/settings.ts`, `crates/app/ui/src/main.ts`, `crates/app/ui/src/icons.ts`, `crates/app/ui/src/strip.ts`, `crates/app/ui/src/folders.ts`, `crates/app/ui/src/theme.ts`, `crates/app/ui/src/tabs.ts`.

#### TODO

- [ ] On Windows (`mise run tauri:dev`), Step 1, Settings: walk every tab (Sidecar with both XMP and `.dop` so `#label-names` shows and hides, Culling, Keyboard Shortcuts with a chip removed and a key captured, Cache, MCP with `Copy`). Expect the modal to read as shadcn Neutral dark: near-black backdrop over the `#0a0a0a` window, `#171717` box, hairline `rgb(255 255 255 / 10%)` borders, `#fafafa` text, outline buttons, `#737373` focus ring. Every control is consistent with the others, the Tab trap cycles through the same controls, and the modal's size and scrolling (a long shortcuts table) are unchanged. Also check that the selected-tab and hover fills, and the chip `×` hover, are told apart (`--muted` and `--accent` are both `#262626`); if the chip `×` hover is invisible, lift it to `--input`.
- [ ] On Windows, Step 2, dialogs: clear the `sidecarFormat` key (or use a fresh profile) to see the first-run dialog and pick a format; open `Move to Trash…` from a strip cell's right-click; open `Sequence JPEG Timestamps…` on a JPEG folder and run it. Expect the format dialog on the `#171717` card with three outline choices; the trash dialog with its count line, a red destructive `Move to Trash` and an outline `Cancel`; the sequence preview with changed (pick color) and unchanged (`#a1a1a1`) lines, a light primary `Run` and an outline `Cancel`; disabled buttons during a run at half opacity; all four dialogs (with Settings) look like one family.
- [ ] On Windows, Step 3, menus / strip bar / cells: open a folder; open the filter and sort menus and toggle a few items; right-click a cell for the context menu; click a cell and shift-click another; watch a fresh folder's placeholders fill in; rename a file inline. Expect hover on `#262626`; a checked item shows the check mark in its leading column with no fill, and it disappears when unchecked; labels stay aligned, nothing wraps and the filter menu still fits above the strip bar; the `Face focus` heading, the star rows (unlit stars dim), the `#filter-exif` groups and the separators are visible on `#171717` (if a separator vanishes, raise `.menu-separator` to `--input`); the context menu's `.shortcut` column is `#a1a1a1` and a radio item shows its check; the filter / sort toggles are outline buttons, the lit filter toggle is `#e5e5e5`, and the sort toggle is half-opacity in a JPEG folder; the focused cell has the light `--primary` border on `#262626`, the other selected cells the `#737373` border on the same fill, and the two read as different; the rejected dimming, burst band, badges and failed red are as before; placeholders are `#262626`; the inline rename field is `#0a0a0a` with a `#737373` outline; the strip and menu scrollbars are thin and visible.
- [ ] On Windows, Step 4, folder tree: expand a few roots, hover rows, select two folders and open one, move the keyboard cursor while the tree has focus, expand a folder with RAW files, find (or make, e.g. an unreadable folder) a failed folder, rename a folder inline, and drag a folder over the window. Expect `#fafafa` rows with rounded `#262626` hover; the selected folders and the open one share `#262626`, the open one bold and the selected ones normal weight; the keyboard cursor shows a `#737373` ring only while the tree has focus; the count is a pill badge (its fill merges into a hovered / selected / open row, so only the number shows there); a failed folder's name is `#ff6467`; a long name on the open row ellipsizes without wrapping or pushing the count out; the inline rename field is `#0a0a0a` with a `#737373` outline; the drop outline is `#737373`; the tree scrollbar is thin and visible.
- [ ] On Windows, Step 5, viewer / meta pane / empty states: start with no folder open, then open an empty folder and a folder whose files are all filtered out; open a fresh folder and watch it scan; make a sidecar write fail (mark a sidecar read-only and rate the file) and dismiss the error; read the meta pane of a RAW file; toggle 1:1 zoom; open Compare with three frames. Expect the empty hints (`Drop a folder…`, `This folder has no RAW or JPEG files.`, `No files match the current filter.`) centered in `#a1a1a1`; the `scanning N / M` then `analyzing N / M` notes, `1:1` and `Compare · N frames` in `#a1a1a1`; the error row in `#ff6467` (not the old amber) with a ghost `×` that hovers on `#262626` and dismisses; the meta pane's EXIF / Maker note / Analysis headings and keys in `#a1a1a1`, values `#fafafa`, a thin scrollbar; Compare's label bars `#171717` with `#fafafa` text, frames with a faint hairline border, and the active frame showing a light `#e5e5e5` inner ring.
- [ ] On Windows, Step 6, settings side-nav (a debug build so Debug shows, a 1280x720 window): open Settings. Expect the modal wider and taller (`min(860px, 90vw)` x `min(720px, 80vh)`) and fitting; the left nav listing Sidecar, Culling, Keyboard Shortcuts, Cache, MCP and Debug, each with its 16px icon, `#a1a1a1` at rest, hovering on `#262626`, the active one bold `#fafafa` on `#262626`; all six items fit without the nav scrolling and "Keyboard Shortcuts" reads acceptably (one line preferred); the nav's right hairline is visible (if not, raise `.sidenav`'s border to `--input`); `ArrowUp` / `ArrowDown` move and activate with wrapping, `Home` / `End` go to the first / last, `ArrowLeft` / `ArrowRight` do nothing, the focused item shows the `#737373` ring, Tab moves into the content, Shift+Tab back, Escape closes; the shortcuts table scrolls inside the content pane (thin scrollbar) while the header and nav stay put, and a status error shows under the content pane.
- [ ] On macOS (WKWebView), repeat the six Windows passes above (Settings, first-run / trash / sequence dialogs, menus / strip bar / cells, folder tree, empty states / meta pane / status lines / Compare, settings side-nav). Also confirm that `scrollbar-color` is honored or harmlessly ignored, that native checkboxes / radios take `accent-color` (Safari 15.4+; an older engine shows the UA control), and that `:focus-visible` rings appear.
- [ ] On Linux (WebKitGTK), repeat the six Windows passes above and the same three extra confirmations (`scrollbar-color`, `accent-color` on checkboxes / radios with WebKitGTK 2.36+, `:focus-visible` rings).

### App: real-device check of an ORF folder in the strip, sidecars and Move to Trash

`orf.rs` and the `.orf` arm of `scan::is_raw_file` / `reader::parse_raw`
were verified by unit tests and `riffle-cli bench` / `scan` / `focusbox` on
the raw.pixls.us samples in `D:\photos\samples\ORF\` (never committed), but
the GUI was never exercised on them. See
`docs/plans/_archived/20260930-olympus-orf/plan.md` and its `learnings.md`.
Files: `crates/core/src/orf.rs`, `crates/core/src/scan.rs`,
`crates/core/src/reader.rs`.

#### TODO

- [ ] On Windows, open `D:\photos\samples\ORF\` in the app and check that
      (1) every `.ORF` appears in the strip with a thumbnail, and the folder
      tree's RAW count includes them; (2) the preview and the 1:1 view show
      the 3200x2400 embedded JPEG, the E-M1 Mark II's rotated frames
      (Orientation 8, e.g. `E-M1MarkII_olympus_om_d_e_m1_mark_ii_01.orf`)
      upright; (3) the meta pane shows the EXIF rows (camera without
      trailing spaces, lens, exposure, capture time); (4) a star, a flag and
      a color label write an XMP and a `.dop` next to the file; (5)
      the strip's `Move to Trash…` on a file moves the ORF and its
      sidecars, and Undo restores them.

### App: real-device checks for the flex / grid gap layout (no-margin-layout) are still open

#### Background

The `no-margin-layout` feature replaced every layout `margin` in `crates/app/ui/style.css` with flex / grid `gap` and padding, added a reset block, wrapped the meta pane's groups in `div.group` and the shortcut chips in `div.keys`, and made `.folder` a three-column grid. CI, the greps and the unit tests cover the code. Nobody has looked at the result on a real machine: the Step 1 checkbox was ticked on the automated criteria only, so the Windows pass and the macOS / Linux passes were never run. The `div.keys` wrapper (instead of `display: flex` on the `td`) was chosen by reasoning, and the settings-panel spacing before / after values in that feature's `learnings.md` are Inferred from the stylesheet, not measured. The plan is `docs/plans/_archived/20260930-no-margin-layout/plan.md`; the layout rules are in `docs/agents/ui-styling.md`.

Files: `crates/app/ui/style.css`, `crates/app/ui/src/main.ts`, `crates/app/ui/src/settings.ts`, `docs/agents/ui-styling.md`.

#### TODO

- [ ] On Windows (`mise run tauri:dev`), measure in devtools the settings spacing on a pre-change build first (the old `p`-to-`p` and label-to-`#label-names` distances), then on the current `main`, and record it. Expect an even 8px rhythm in the settings panels. Then walk the pass. Dialogs: the first-run format dialog (clear `sidecarFormat` or use a fresh profile; the error line under the note when a write fails), the strip's `Move to Trash…` (the count line, the button row), and `Sequence JPEG Timestamps…` on a JPEG folder (source / output lines, rebuild note, rows, count, the running note during a run), all spaced as before. Settings: Sidecar with XMP and `.dop` so `#label-names` shows and hides; Culling; Keyboard Shortcuts with a chip removed and a key captured on a row with enough keys to wrap (chips, the `Press a key...` prompt and `+` vertically centered against the label and Reset cells, 0.2rem / 0.3rem gaps, a one-line row keeping its height, which validates the `div.keys` choice); Cache with the clear note visible during a scan; MCP with the endpoint and both example blocks. Strip bar: the `N / M` counter at the right edge and the toggles at the left, also with `· K selected`; the filter / sort / context menus open and their separators still bleed edge to edge. Folder tree: a long name ellipsizing before its badge, the badge at the right edge, the open folder bold, a collapsed folder without a badge, an inline rename filling the name column without the row jumping, the keyboard cursor ring. Meta pane: the EXIF / Maker note / Analysis headings 0.75rem below the previous block and 0.125rem above their lists; the status notes (scan progress, `1:1`, Compare, an error with its `×`) unchanged. Empty states: no folder, no files, filtered out.
- [ ] On macOS (WKWebView), repeat the Windows pass above, and confirm that flex `gap` renders (the guide's floor: Safari 14.1 / macOS 11).
- [ ] On Linux (WebKitGTK), repeat the Windows pass above, and confirm that flex `gap` renders (the guide's floor: WebKitGTK 2.32).
- [ ] On macOS and Linux, confirm that Settings > Sidecar's `#label-names > label` rows (`grid-template-columns: subgrid`, Chromium 117+ / Safari 16+, see the subgrid bullet in `docs/agents/ui-styling.md` "CSS features") still line up; if not, replace the subgrid with explicit columns.

### App: real-device check of focus after deleting the focused file from outside the app

#### Background

The `todo-five-small-items` feature (Step 3) made `resync` fall back to the deleted focused file's neighbour instead of the first file (`anchorAfterFilter` takes an optional `previous` list, and `refilter` passes it from `resync`). Vitest cases in `crates/app/ui/src/filter.test.ts` cover the picker (next neighbour, previous neighbour, a filtered-out neighbour, consecutive deletions), and `mise run ci` passes. The flow through a real window, where the file is deleted in Explorer / Finder and the app regains focus, was never exercised because the GUI cannot be driven from the implementing session. Plan: `docs/plans/_archived/20260929-todo-five-small-items/plan.md`.

Files: `crates/app/ui/src/main.ts` (`resync`, `refilter`), `crates/app/ui/src/filter.ts` (`anchorAfterFilter`).

#### TODO

- [ ] On Windows, open a folder of RAW files, focus a file in the middle of the strip, delete it in Explorer, and switch back to the app. Expected: the strip stays on the deleted file's next file (the previous one if it was the last), not the first file, and the preview shows that file. Repeat with a filter active whose next neighbour is filtered out. Expected: focus lands on the next passing file.
- [ ] On macOS, the same steps with Finder.

### App: real-device check of the focus-rescan throttle (todo-five-more-items Step 2)

#### Background

The `todo-five-more-items` feature made an idle focus rescan end at once and throttled repeated focus events. Vitest cases for `focusRescanDue` and `mise run ci` cover it. The GUI behaviour was never exercised: the implementation agent had no GUI session. See `docs/plans/_archived/20260930-todo-five-more-items/plan.md` (Step 2).

Files: `crates/app/ui/src/main.ts`, `crates/app/src/commands.rs`.

#### TODO

- [ ] On Windows, with `Timing logs` on, open a folder and let its scan and faces pass finish. Wait more than 5 s, then switch to another app and back to Riffle. Expect `Riffle.log` to show `scan prepare: … todo=0` with no `scan extract` or `scan faces` line after it, and the status line's `scanning` never appears. Then alt-tab away and back twice within 5 s: only the first focus logs a `scan list` line.

### App: real-device check of Expand All / Collapse All in the folder tree (todo-five-more-items Step 3)

#### Background

The `todo-five-more-items` feature added `Expand All` / `Collapse All` to the folder tree's context menu. Vitest cases in `tree.test.ts` / `context.test.ts` and `mise run ci` cover the logic. The real menu on a real photo archive was never exercised, and neither was the watcher count or the ancestor-rename interaction. See `docs/plans/_archived/20260930-todo-five-more-items/plan.md` (Step 3) and the `notify` ancestor-pinning item in `docs/agents/tauri-app.md`.

Files: `crates/app/ui/src/folders.ts`, `crates/app/ui/src/tree.ts`, `crates/app/ui/src/main.ts`.

#### TODO

- [ ] On Windows, right-click a folder with two or more levels of subfolders (e.g. under `D:\photos`) and choose `Expand All`: every level should appear. Then `Collapse All` on the same folder: all levels fold back with the clicked folder still open. Then `Expand All` on a folder containing a subfolder that cannot be listed (e.g. read permission denied): the unreadable folder shows its error (status line, row marked failed) and the rest still expand.
- [ ] On Windows, measure how many `tree-changed` watches a real `Expand All` on a photo archive makes (the number of expanded drawn folders; via `Timing logs` or a debugger on `set_tree_watches`). Also check that renaming an ancestor of the expanded folders still works.

### App: real-device check of the 16-language Lightroom label preset dropdowns (lightroom-label-presets-all-languages)

#### Background

The `lightroom-label-presets-all-languages` feature grew the Lightroom color label preset list from English and Japanese to 16 (de, es, fr, it, ko, nb, nl, pl, pt, ru, sv, th, zh-Hans, zh-Hant added) and made `defaultPreset` preselect a script-coded preset (`zh-Hans` / `zh-Hant`) through `Intl.Locale.maximize()`. `cargo test -p riffle-core`, the `firstrun.test.ts` cases and `mise run ci` cover it, but neither the dropdown contents nor the preselection were run in the GUI. See `docs/plans/_archived/20261005-lightroom-label-presets-all-languages/plan.md`.

Files: `crates/app/ui/src/main.ts`, `crates/app/ui/index.html`, `crates/app/ui/src/settings.ts`, `crates/app/ui/src/firstrun.ts`, `crates/core/i18n/*.json`.

#### TODO

- [ ] On Windows, with a fresh settings store, pick `Lightroom (XMP)` and open the first-launch language dropdown: it lists 16 entries in the order English, Deutsch, Español, Français, Italiano, 日本語, 한국어, Norsk bokmål, Nederlands, Polski, Português (Brasil), Русский, Svenska, ไทย, 简体中文, 繁體中文, and the preselection matches the OS locale (for example `ja` on a Japanese system). Choosing another entry fills the five label names.
- [ ] On Windows, open the settings window's label-name language dropdown: it lists the same 16 entries in the same order, and choosing one fills the five names. (This could also be checked on macOS.)

### App: real-device checks for trash undo / redo after a folder rename (todo-five-more-items Step 5)

#### Background

The `todo-five-more-items` feature rebased the trash runs and their undo / redo entries when a folder is renamed. On Windows / Linux, restore recreates the missing old folders, restores from the Trash, moves the file to the new place and removes only the folders it created; on macOS, `restore_recorded` renames straight to the new destination. `trash.rs` tests (`Runs::rename_dir`, `restore` with a destination, `relocate`), the `undo.test.ts` case and `mise run ci` cover it. No real Recycle Bin or macOS Trash was exercised. See `docs/plans/_archived/20260930-todo-five-more-items/plan.md` (Step 5).

Files: `crates/backend/src/trash.rs` (`restore_recorded`), `crates/app/src/commands.rs` (`restore_run`), `crates/app/src/rename.rs`, `crates/app/ui/src/main.ts`.

#### TODO

- [ ] On Windows, reject two files in a copy under `D:\photos\samples\X` (ideally one with both sidecar formats), move them to the Trash with the strip's `Move to Trash…`, rename the folder in the tree (`Rename…`), then `Undo`. Expect the files and their sidecars back in the renamed folder, the strip (reopened under the new path) showing them with their reject marks, and no empty old-name folder left behind. Repeat with the folder's parent renamed instead. Then `Redo` should move them to the Recycle Bin again.
- [ ] On macOS, repeat the undo / redo after a rename (`restore_recorded` renames the Trash file straight to the new destination, with no `relocate`). Expect the files back in the renamed folder and no old-name folder appearing.

### App: real-device check of the scan priority and on-screen-first order on a large RAW folder

`scan-priority` (docs/plans/_archived/20260930-scan-priority/plan.md) moved the
sharpness score into the second pass, ran the scan workers below normal OS
priority (first pass `BELOW_NORMAL` / `QOS_CLASS_UTILITY`, analysis pass
`LOWEST` / `QOS_CLASS_BACKGROUND`) and added `set_scan_focus`, which makes both
passes take the current file and the strip's visible range first. CI covers
the build, the unit tests (queue order, cancel, delivery once, the priority
read back inside a worker on Windows and Linux) and the Linux / macOS
compilation. Never exercised on a real machine: the effect of the priority on
paging, the on-screen-first order on a cold large folder, the queue's own
overhead in wall time, and the macOS QoS branch. Steps 4 and 6 were ticked on
the automated criteria only.
Files: `crates/core/src/scan.rs` (`for_each_path`, `Priority`, `WorkQueue`),
`crates/backend/src/index.rs` (`run_scan`, `run_faces_scan`),
`crates/app/ui/src/main.ts` (`sendScanFocus`), `crates/app/ui/src/scanfocus.ts`,
`crates/app/ui/src/strip.ts` (`visibleRange`), `docs/humans/performance.md` ("Which
pass carries which cost").

#### TODO

- [ ] On Windows, clear the cache (settings modal, `Clear Cache`), open a large RAW folder (a few thousand files) in a development build with the settings modal's `Timing logs` on, and page through it with the arrow keys while `scanning N / M` and then `analyzing N / M` run. Compare the `page read=.. ipc=.. post=.. worker=.. decode=.. draw=.. total=.. keypressToPixels=..` lines in `Riffle.log` with the same run on the previous release (and with paging after the scan ends). Expect that during the passes they are not worse and are closer to the idle value. Note the wall time of the `scan extract` / `scan faces` summary lines of both runs, to see what the lowered priority costs while paging. No `ran at normal priority` warning may appear in the log.
- [ ] On Windows, clear the cache, open the same kind of folder in a development build with `Timing logs` on, and at once jump to the middle of the strip (drag the scrollbar, then click a cell). Expect the cells around the current file to get their thumbnails during `scanning N / M`, and then their focus marks during `analyzing N / M`, before the cells at the folder's start do (by eye, or from the `scan-progress` / `faces-progress` `ready` lists in the webview devtools). Note the wall time of the `scan extract` / `scan faces` summary lines against the previous TODO's run of the same folder, so the queue's own overhead shows.
- [ ] On macOS, repeat the paging run once. Expect the analysis pass (`QOS_CLASS_BACKGROUND`, which also throttles disk IO) not to crawl; if it does, move it to `QOS_CLASS_UTILITY` and update the priority entry in `docs/agents/tauri-app.md`.
- [ ] Done when: the measured page-latency and `scan extract` / `scan faces` numbers, with their conditions, are in `docs/humans/performance.md` ("Which pass carries which cost"), and the README wording on paging is revisited from that result.

### App: real-device check that big-endian DNGs open after the EXTRACTOR_VERSION bump

`big-endian-dng` (`docs/plans/_archived/20261002-big-endian-dng/plan.md`) made `arw::parse` follow the TIFF header's byte order, so the `MM` DNGs of Pentax, Samsung, Ricoh, Leica M (Typ 240 / 246) and iPhone no longer fail with `not a little-endian TIFF/ARW`. It also bumped `EXTRACTOR_VERSION` from 11 to 12, so an index that cached those parse errors should re-extract the rows. The unit tests, `riffle-cli check` on the sample tree (34 of the 52 `MM` files open at every stage) and `mise run ci` cover the parser. The Riffle app was never opened on such a folder, so the re-extraction of cached failed rows and the thumbnails, preview and 1:1 view for these files were never exercised. The step was ticked on the automated criteria only.

Files: `crates/core/src/arw.rs`, `crates/backend/src/index.rs` (`EXTRACTOR_VERSION`).

#### TODO

- [ ] On Windows, with an index written by `EXTRACTOR_VERSION` 11 (open `D:\Photos\samples\DNG\` in a build from before the bump, then in this build), open a folder containing the Pentax K-5 II, iPhone 12 Pro and Leica M (Typ 240) samples. Expected: the rows that cached `not a little-endian TIFF/ARW` are re-extracted, and the thumbnails, the preview and the 1:1 view show.

### App: real-device check of the 1:1 view on older Sony ARW bodies (no full-size JPEG)

The `sony-arw-preview-full` work (`docs/plans/_archived/20261005-sony-arw-preview-full/plan.md`) made `arw::parse` fall back to the preview as `full` when the largest other JPEG is smaller, so the α9 II, α7R IV, α7R IVA, α7C, α6400, α6600 and ZV-E10 no longer take the 160x120 IFD1 thumbnail for the 1:1 view. A synthetic-TIFF unit test, `riffle-cli info` / `bench` on the samples (`full` at the preview's offset / length; decodes of 1616x1080, 1440x1080 on the 4:3 α7C sample and 1920x1080 on the ZV-E10 read from the JPEG SOF) and `mise run ci` cover the core path. The running app's 1:1 view (`read_focus_crop` via `reader::read_full`) was never exercised on these files. The step was ticked on the automated criteria only. Files: `crates/core/src/arw.rs`, `crates/core/src/reader.rs`.

#### TODO

- [ ] On Windows (or any desktop build), open `D:\photos\samples\ARW` (or a folder with e.g. `ILCE-6400_DSC00087.ARW` / `ZV-E10_DSC00002.ARW`), select the file and open the 1:1 focus check. Expected: a sharp crop of the 1616x1080 preview (1920x1080 on the ZV-E10) around the AF point, not a blurry upscaled 160x120 thumbnail.

### Agents: bare `python` / `node` hang in Git Bash on the Windows host

#### Background

During `merge-settings-drop-hook-exclusions` (`docs/plans/_archived/20261002-merge-settings-drop-hook-exclusions/learnings.md`), a bare `python` or `node` in Git Bash on the Windows host hung instead of failing; running them through `mise x --` worked. Any agent session on this host that runs a script needing them hits the same hang, before any guide would be consulted.

Files: `CLAUDE.md` (Development) or a new `docs/agents/dev-environment.md`.

#### TODO

- [ ] Check that the note is not already in the user-level memory or instructions, then add it: preferably a short line under `CLAUDE.md` Development ("On Windows, run python / node through `mise x --`; a bare invocation hangs in Git Bash"), otherwise a new `docs/agents/dev-environment.md`. Done when the note exists in the chosen place.

---

### App: real-device checks for the folder tree's Refresh item

#### Background

The error-row-exif-tree-refresh feature (Step 2) added a `Refresh` item to the folder tree's right-click menu. It re-lists a folder with the same `list_subfolders` + `setChildren` as `toggle`, keeps the folders open under it, and re-sends the watch set so a watch that failed is retried. `context.test.ts` covers the menu groups. The two checks below were never run on a real machine: they need a folder whose watch cannot be set (a network share) and a folder that vanishes without its parent's watch removing the row. Plan: docs/plans/_archived/20261002-error-row-exif-tree-refresh/plan.md.

Files: `crates/app/ui/src/folders.ts` (`refresh`), `crates/app/ui/src/context.ts`.

#### TODO

- [ ] On a network share (or any folder whose watch `set_tree_watches` cannot set), expand a folder, create a subfolder in it from another machine or the OS file manager, then right-click the folder in Riffle's tree and choose `Refresh`. Expected: the new subfolder appears and the RAW count updates, with the folders already open under it still open.
- [ ] On the same kind of folder, delete or unmount a folder shown in the tree (without its parent's watch removing the row, e.g. on a share), then choose `Refresh` on it. Expected: the row is marked failed (its tooltip shows the error), it collapses, and the error appears in the status line.

### App: real-device check that a folder open, a cache clear and a focus return each start exactly one scan

#### Background

The `picker-focus-double-scan` feature wrapped `openFolder`'s whole chain (the `pick_folder` invoke through `openDirectory`) in `settleIdle`. A `tauri://focus` rescan that arrives while the folder picker is open is now deferred, so it no longer calls `scan_folder` for the previous folder. `scan_folder` also logs `scan superseded: dir={dir} scan_id={scan_id} by={latest}` at info level when its id is superseded. `mise run ci` (vitest, Rust tests) passes. It does not cover the race itself, which is Tauri IPC ordering between two `invoke`s and a window event. No GUI session ever exercised it, so the check below was never run. See `docs/plans/_archived/20261003-picker-focus-double-scan/plan.md` (Step 1).

Files: `crates/app/ui/src/main.ts` (`openFolder`, the `tauri://focus` listener), `crates/app/src/commands.rs` (`scan_folder`).

#### TODO

- [ ] On Windows, run `mise run tauri:release:devtools` with timing logs on. With folder A open and its scan older than 5 s, choose File > Open Folder and pick folder B. Expect exactly one `scan list` / `scan prepare` / `scan extract` set, for B, and no `scan superseded` line.
- [ ] On Windows, repeat with Settings > Clear Cache. Expect one set, for the reopen.
- [ ] On Windows, alt-tab away from the app and back. Expect one set.

### App: real-device check of Refresh on the open folder also reloading the strip

#### Background

The refresh-reloads-open-folder feature made the folder tree's right-click `Refresh` on the open folder also run `resync("refresh")`, the same rescan as `File > Reload Folder` (`CmdOrCtrl+R`). It logs a dedicated `trigger=refresh`, so `Riffle.log` tells the menu item from `CmdOrCtrl+R`. `refresh.test.ts`, `trash.test.ts` and `mise run ci` cover the trigger line and the same-folder decision. The behavior on a folder whose watch does not fire was never exercised on a real machine. Plan: docs/plans/_archived/20261006-refresh-reloads-open-folder/plan.md.

Files: `crates/app/ui/src/main.ts` (the `refreshFolder` branch), `crates/app/ui/src/refresh.ts` (`RescanTrigger`).

#### TODO

- [ ] On Windows, open a folder on a share (or any volume) whose watch does not fire, add a RAW to it and edit a sidecar outside Riffle, then right-click the open folder in the tree and choose `Refresh`. Expected: the new file appears in the strip, the edited judgment is picked up, the tree's RAW count updates, and `Riffle.log` shows `rescan: trigger=refresh`.
- [ ] On Windows, choose `Refresh` on the open folder during a running scan. Expected: `Riffle.log` shows `rescan deferred: trigger=refresh`, and the rescan runs when the scan ends.
- [ ] On Windows, choose `Refresh` on a folder that is not open, including its parent or a subfolder. Expected: the strip is left untouched and only the tree re-lists.

### App: a redo of Delete Sidecars… leaves a closed folder's rows judged

#### Background

The folder-sidecars feature added `Delete Sidecars…`. `trash_rejected_redo` moves the sidecars to the Trash again but writes no index row. When the target folder is open, the frontend's resync clears the judgments. When another folder is open, the rows keep the judgments that the undo's rescan restored, until the folder is next opened and its reconcile clears them. A `Rewrite Sidecars from Index…` run in between would mint the sidecars again. Fixing it needs the run to remember each sidecar's RAW, for example in `trash::Trashed`. Plan: `docs/plans/_archived/20261005-folder-sidecars/plan.md`. Files: `crates/app/src/commands.rs` (`trash_rejected_redo`), `crates/app/src/foldersidecars.rs`, `crates/backend/src/trash.rs`, `crates/app/ui/src/main.ts` (`trashed`).

#### TODO

- [ ] Record each sidecar's RAW in the trash run (for example in `trash::Trashed`), so that a redo clears the index rows of a closed folder.
- [ ] Add a test covering delete, undo, then redo with another folder open.

### App: Rewrite Sidecars from Index… patches an oversize sidecar

#### Background

The folder-sidecars feature added `Rewrite Sidecars from Index…`. `rewrite_sidecars_run` hands every listed row to the writer, including a file whose sidecar is larger than `MAX_SIDECAR_BYTES`. A folder open deliberately keeps such a file away from the writer through `reconcile_sidecars_of`, because its contents were never read. This was left as is because the user asked for the index to be pushed out. Plan: `docs/plans/_archived/20261005-folder-sidecars/plan.md`. Files: `crates/app/src/foldersidecars.rs`, `crates/app/src/commands.rs` (`MAX_SIDECAR_BYTES`, `reconcile_sidecars_of`).

#### TODO

- [ ] Decide whether the rewrite should skip oversize sidecars like a folder open does.
- [ ] If it should, skip them and count them in the dialog's "skipped" figure.

### App: real-device checks for the main window being created hidden (startup window flash)

#### Background

The startup-window-flash feature sets `"visible": false` on the main window in `crates/app/tauri.conf.json`. `tauri-plugin-window-state` then restores the saved size and position before the window is shown. `crates/app/src/main.rs` also calls `show()` in the `setup` closure as a fallback. That fallback covers two cases:
- A saved `visible: false`, for example from quitting while minimized on macOS.
- A failed `set_position` / `set_size` / `set_fullscreen` that aborts `restore_state` before its own `show()`.

CI covers only the automated criteria (`mise run ci`). No GUI session was available, so none of the checks below was ever run, and the step's checkbox was ticked on the automated criteria alone. Plan: `docs/plans/_archived/20261010-startup-window-flash/plan.md`.

Files: `crates/app/tauri.conf.json`, `crates/app/src/main.rs`.

#### TODO

- [ ] On Windows, on a built binary (`pnpm tauri build`; the `pnpm tauri dev` binary is acceptable, so note which was used):
  1. With a saved `.window-state.json` (app config dir) at a non-default size and position, launch. The window appears directly at the saved size and position, with no 1280x800 frame visible first.
  2. Delete `.window-state.json` and launch. The window appears at 1280x800, OS-placed, and focused.
  3. Edit `.window-state.json` so `main.visible` is `false` and launch. The window still appears (the fallback `show()` path).
  4. With the app running (once normal, once minimized), launch it again. The existing window is unminimized and focused, and no second window appears.
  5. Quit maximized and relaunch. It comes back maximized with no normal-size frame first.
  6. Note whether a white (unpainted WebView) frame shows before the dark UI paints. If it does and bothers the user, the follow-up is `"backgroundColor": "#0a0a0a"` on the window entry in `tauri.conf.json`.
- [ ] On macOS, minimize the window, quit with Cmd+Q, and relaunch. The window appears. This is the real-world source of a saved `visible: false`.

### App: real-device check of the performance graph overlay's legend swatches

#### Background

`performance-graph-overlay` added an overlay toggled by the `performance` action (default key `d`) that plots per-switch preview latency, Riffle's CPU and the system CPU over 60 s. Its hand checks (a)-(c) passed on Windows (`results.md`). Afterwards the two CPU lines were restyled (Riffle CPU 2 px solid, system CPU 1 px dashed) and the legend gained a 14 px swatch of each line before its label. CI covers only the model (`perf.test.ts`) and the build. The swatch drawing and its spacing against the labels were never seen on a real screen. Plan: `docs/plans/_archived/20261011-performance-graph-overlay/plan.md`. Files: `crates/app/ui/src/main.ts` (`PERF_LINES`, `drawPerf`).

#### TODO

- [ ] On Windows (any platform works), open a folder and press `d`. Check at a glance that the overlay's legend shows a thick solid line before `Riffle CPU` and a thin dashed line before `System CPU`, matching the two plotted CPU lines, and that the labels do not overlap the swatches.

### App: real-device checks for focus peaking

#### Background

`focus-peaking` added a session-only `peaking` toggle (default key `e`, off at launch) that draws red edge highlights over the fit view and the 1:1 (`z`) crop. The edge measure is a Sobel magnitude (`|gx| + |gy|`) computed in `peakingworker.ts`. CI covers only the pure module (`peaking.test.ts`), the keymap tests and the build. Never exercised in the running app: alignment under orientations 1, 3, 6 and 8 and at several window sizes and display scalings, the 1:1 crop overlay, staleness while paging, Compare, rebinding, tuning of `PEAKING_THRESHOLD` (160, a starting value not tuned by eye), and the main-thread raster cost on a 4K / 2x display. Plan: `docs/plans/_archived/20261108-focus-peaking/plan.md`. Files: `crates/app/ui/src/peaking.ts`, `crates/app/ui/src/peakingworker.ts`, `crates/app/ui/src/main.ts` (`draw()`, `drawZoom()`, `runAction`), `crates/backend/src/shortcuts.rs`.

#### TODO

- [ ] On Windows (ideally also macOS and Linux), open a folder of RAW files and press `e`. Red edge highlights appear over the fit view a moment after the image and sit exactly on its edges for orientations 1, 3, 6 and 8 (use portrait shots rotated both ways), at several window sizes and at 100 % / 150 % / 200 % display scaling. Press `e` again: they vanish completely.
- [ ] On Windows, press `z` with peaking on. The 1:1 crop gets its own overlay aligned with its pixels, and the scaled placeholder shown while the crop loads has none.
- [ ] On Windows, hold `ArrowRight` through a folder with peaking on. No overlay from an earlier file ever shows over a later one, and the last file gets its overlay.
- [ ] On Windows, press `v` (Compare). It shows no peaking.
- [ ] On Windows, open Settings > Shortcuts. It lists `Focus peaking` on `e`, and it can be rebound.
- [ ] On Windows, tune `PEAKING_THRESHOLD` (`crates/app/ui/src/peaking.ts`) by eye on `D:\photos\samples\`: a sharp eye is lit, smooth skin and bokeh are dark, and shadow noise is mostly dark. Note whether the fit view and the 1:1 crop want different values (that would justify two constants).
- [ ] On a 4K / 2x display with `Timing logs` on, read the `peaking WxH raster=` lines at full window. If the main-thread rasterize plus `getImageData` costs well over a frame, consider moving the readback off the main thread (`OffscreenCanvas` in the worker; the `safari13` build target does not guarantee it).

### Shortcuts: a newly added default key silently displaces a user's stored override

#### Background

`performance-graph-overlay` added the default key `d`, and `focus-peaking` added `e`. `Keymap::from_overrides` gives a key already held by a default precedence over a partly applicable stored override. A user whose stored `shortcuts` override bound `d` or `e` to another action (for example a WASD-style layout on `next`) therefore loses that key silently on upgrade. It surfaced when `a_stale_default_in_an_old_store_drops_only_that_key` failed in CI; that test's arbitrary `d` was changed to `e`, and `focus-peaking` then changed it to `n` when `e` became `peaking`'s default, but the real-user case is unaddressed. Plans: `docs/plans/_archived/20261011-performance-graph-overlay/plan.md`, `docs/plans/_archived/20261108-focus-peaking/plan.md`. Files: `crates/backend/src/shortcuts.rs` (`DEFAULTS`, `from_overrides`).

#### TODO

- [ ] Decide whether a newly added default should yield to an existing override, or whether the drop should be logged at `warn` (or both).
- [ ] Implement the chosen behavior with a Rust test covering an old store that binds the new default's key to another action.

### App: real-device check of auto-advance on a repeated star, reject or pick

#### Background

The `repeat-judgment-advance` feature makes a repeat press of the value a file already holds (`3` on a 3-star file, `x` on a rejected file, `p` on a picked file) move to the next file when `Auto-advance after a star, reject or pick` is on. The press still writes nothing and adds no undo entry. The pure decision `shouldAdvance` is covered by Vitest in `advance.test.ts`, and `mise run ci` passes. The behavior was never exercised in the desktop app: the implementation agent had no GUI session. See `docs/plans/_archived/20261012-repeat-judgment-advance/plan.md` (Step 1). Files: `crates/app/ui/src/main.ts`, `crates/app/ui/src/advance.ts`.

#### TODO

- [ ] On Windows, with the app open and `Auto-advance after a star, reject or pick` on:
  - Press `3` twice on a file. The second press moves to the next file, adds no undo entry and leaves the status line untouched.
  - Press `x` on a rejected file and `p` on a picked file. Both move to the next file.
  - On the last file, a repeat press stays on it.
  - With a 2-file selection, a repeat press does not advance.
  - In Compare, a repeat press does not advance.
  - In a JPEG-only folder (view-only), the keys do nothing.
- [ ] On macOS, repeat the same checks as on Windows.

### App: real-device check of auto-advance after un-reject / un-pick

#### Background

The `advance-on-unflag` feature makes the `unflag` key (`u`, "Un-reject / un-pick") move to the next file when `Auto-advance after a star, reject or pick` is on, under the same gates as reject and pick. A repeat `u` on a file with no flag also advances, writing nothing and adding no undo entry. The pure decision `shouldAdvance` is covered by Vitest in `advance.test.ts`, and `mise run ci` passes. The behavior was never exercised in the desktop app: the implementation agent had no GUI session. See `docs/plans/_archived/20261104-advance-on-unflag/plan.md` (Step 1). Files: `crates/app/ui/src/advance.ts`, `crates/app/ui/src/main.ts`.

#### TODO

- [ ] On Windows, with the app open and `Auto-advance after a star, reject or pick` on:
  - Press `u` on a picked file and on a rejected file. Each moves to the next file.
  - Press `u` on an unflagged file. It moves to the next file, writes nothing and adds no undo entry.
  - With Auto-advance off, `u` stays on the file.
  - In Compare, `u` stays on the file.
  - With a 2-file selection, `u` stays on the file.
- [ ] On macOS, repeat the same checks as on Windows.

### App: real-device check of the disk read and disk busy series in the performance overlay

#### Background

`perf-overlay-disk` added Riffle's disk read throughput (`Riffle disk`, MB/s on its own axis, thick solid green) and the system's disk busy % (`System disk`, 0-100 %, thin dashed green) to the performance overlay (`d`). It also recolored the CPU lines blue and the latency bars red. Latency spikes of about 4.5 s have been seen with both CPU lines near 0, and the overlay is meant to show whether they line up with disk activity. CI covers the pure delta / ratio computations, the `PerfModel` (`perf.test.ts`) and the build. A PDH read of the busy counter was run ad hoc on Windows. The overlay drawing, the 500 ms updates, the colors, the latency-vs-disk alignment during a scan and the provisional `READ_FLOOR_BPS` (10 MB/s) were never seen on a real screen. The step's checkbox was ticked on the automated criteria only. The Linux `/sys/block` busy path and the macOS (no busy line) path were never run. Plan: `docs/plans/_archived/20261012-perf-overlay-disk/plan.md`. Files: `crates/app/src/perf.rs`, `crates/app/ui/src/perf.ts` (`READ_FLOOR_BPS`), `crates/app/ui/src/main.ts` (`drawPerf`, `PERF_LINES`), `crates/app/ui/style.css` (chart tokens).

#### TODO

- [ ] On Windows, with the overlay on (`d`), open a large RAW folder that has not been scanned (or force a rescan) and browse with the arrow keys during the scan. Expected: the green lines (`Riffle disk` thick solid in MB/s, `System disk` thin dashed on 0-100 %) update every 500 ms, the CPU lines are blue, the latency bars are red, and a latency spike lines up with high disk activity (or does not, which is also a result). Record the observed idle and scan read rates (MB/s) and retune `READ_FLOOR_BPS` in `crates/app/ui/src/perf.ts` (provisionally 10 MB/s).

### App: real-device checks of AI analysis (pass 3 and the model download) on Windows

#### Background

`image-embeddings` Step 3 added the `aiEnabled` setting, the first-enable download of the SigLIP 2 model, and pass 3 (`run_embed_pass`), which fills the `embeddings` table after pass 2. CI covers the table, the staleness rule, the `ScansState` phase and the download's `install` with a local reader. An ignored headless harness (`embed_pass_on_a_real_folder`) timed pass 3 on `D:\photos\2026\2026-09-19`, and `the_pinned_model_downloads_and_matches_its_hash` ran the download (9.8 s). The settings checkbox, the status line, the log lines, the working set seen in Task Manager, and the paging latency under the GUI were never exercised. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/app/src/commands.rs` (`run_embed_pass`), `crates/app/src/ai.rs`.

#### TODO

- [ ] On Windows 11, with AI off and the model not downloaded, open Settings > Culling and tick "AI analysis of what each photo shows". Expected: the status line counts `Downloading the model: N / 378 MB` up to `On`, and the model sits at `%LOCALAPPDATA%\com.minodisk.riffle\models\siglip2-base-patch32-256\vision_model.onnx` (378,478,689 bytes).
- [ ] On Windows, with `D:\photos\2026\2026-09-19` open, expect the folder to rescan by itself once the status reads `On`. `Riffle.log` should get an `embed model ready` line and a `scan embed: dir=... files=2134 ... errors=0 threads=6` line.
- [ ] On Windows, while that pass runs, Task Manager should show the app's working set about 800 MB above its usual footprint. Page through the strip with Timing logs on, or with the performance graph overlay. Page latency should sit close to its level with AI off (the harness proxy read 15 ms against 10 ms idle).
- [ ] On Windows, run `File > Reload Folder`. Expected: no new `scan embed` line (0 files to embed).
- [ ] On Windows, untick the checkbox during a pass. Expected: `Riffle.log` shows the pass ending with `canceled=true`. Ticking it again finishes only the remaining files.
- [ ] On macOS and on Linux, tick the setting in the app, or run `cargo test --release -p riffle-app the_pinned_model_downloads_and_matches_its_hash -- --ignored --nocapture`. Expected: the download completes and passes the hash check. rustls' platform verifier uses each OS's trust store, and the download was verified on Windows only.

### App: real-device check of the Genre filter section

#### Background

`image-embeddings` Step 5 added a `Genre` section to the strip's filter menu, fed by `genres_of` from the stored embeddings and hidden when `aiEnabled` is off. CI covers the calibration (`genres.md`) and the `genre.rs`, `filter.ts` and `index.rs` unit tests. The filter menu in a real window, the hide and show on the setting, and the live fill-in during pass 3 were never exercised. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/app/ui/src/main.ts` (`refreshGenres`, `setGenreSection`), `crates/app/ui/src/filter.ts`, `crates/app/src/commands.rs` (`genres_of`).

#### TODO

- [ ] On Windows 11, with AI analysis on, open `D:/photos/2026/2026-09-19` after its embeddings exist. Expected: the filter menu shows a `Genre` section (Sports, Basketball, Portrait, Group photo, Landscape, Birds / animals, Pets, Snap, Camping, Unknown) below `Eyes`.
- [ ] On Windows, check `Basketball`: the strip keeps the basketball frames (most of that folder). Check `Unknown` alone: the strip shows the frames with no genre and any not embedded yet.
- [ ] On Windows, untick "AI analysis of what each photo shows" in the settings. Expected: the section disappears and the strip shows everything again. Tick it again: the section returns with nothing checked.
- [ ] On Windows, on a folder never embedded, with a `Genre` item checked, expect the strip to fill in as pass 3 runs (`embed-progress`) and be complete at `embed-done`.

### App: real-device check of the personal ranking (Recommended sort, viewed frames, Refit Ranking)

#### Background

`image-embeddings` Step 7 added the `Recommended` sort key, the meta pane's `Recommended` / `Reason` / `Confidence` rows and the settings' `Refit Ranking` button, all behind `aiEnabled`; `per-file-viewed-ranking` Step 1 replaced the folder tree's `Culling Finished` check with the per-file `views` (an unflagged frame counts as not chosen once it was shown in the preview; plan `docs/plans/_archived/20261014-per-file-viewed-ranking/plan.md`). CI covers the re-run in `ranking.md`, the Rust / Python cross-check `the_scores_match_rank_prior_py` and the unit tests. The menus, the meta rows, the `Riffle.log` fit line and the settings status in the real app were never exercised. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/app/ui/src/main.ts` (`refreshRanks`, `setRanking`), `crates/app/ui/src/settings.ts`, `crates/app/src/rank.rs`.

#### TODO

- [ ] On Windows 11, with AI analysis on (model downloaded), open `D:\photos\2026\2026-09-19` and let pass 3 finish. Expected: the sort menu shows `Recommended` below `Rating`. Choosing it keeps each burst together (burst bands contiguous) and reorders frames within bursts. The meta pane's Analysis group shows `Recommended` (`N of M in its burst · +0.xx`), `Reason` (up to three, e.g. `Look +0.62, Burst position −0.31`) and `Confidence` (`Built-in only, nothing learned yet` on a fresh install).
- [ ] On Windows, page through the folder, pick a few frames, then press `Refit Ranking` in Settings > Culling. Expected: `Riffle.log` gets a `ranking fit: N bursts from 1 folders` line, the meta pane's `Confidence` changes to `Low · N bursts in 1 folder`, and the settings' Culling tab reads `Ranking: Low · ...`. The folder tree's right-click menu has no `Culling Finished` item.
- [ ] On Windows, in Settings > Culling, press `Refit Ranking`. Expected: `refitting…`, then the status. Untick AI analysis: the `Recommended` item, the meta rows and the refit block disappear, and a strip sorted by `Recommended` falls back to capture order. Tick it again: they return.
- [ ] On Windows, at every point above, no judgment changes (stars, flags and labels as before, no sidecar written by the ranking).
- [ ] On Windows, view a few frames of a folder, then set its `folders.opened_at` back more than 30 days (or wait) and trigger eviction. Expected: the folder's rows and embeddings stay, while a never-viewed folder without a reject of the same age is evicted; `Clear Cache` still removes it.
- [ ] (per-file-viewed-ranking Step 1: `rank_rows` reads `views`, `fit` counts an unflagged frame only when viewed; `crates/backend/src/index.rs`, `crates/core/src/rank.rs`; plan `docs/plans/_archived/20261014-per-file-viewed-ranking/plan.md`.) On Windows 11, with AI analysis on, in a fresh RAW folder whose frames all have their pass 3 embedding (a frame without one makes no pair), turn `Auto-advance after a star, reject or pick` off, click one frame of a burst in the strip and pick it without showing the others (do not page with the arrow keys), press `Refit Ranking` and note the fit line in `Riffle.log`. Then click another frame of that burst in the strip, leave it unflagged and press `Refit Ranking` again. Expected: the second fit line counts one more burst (the viewed, unpicked frame paired against the pick) and the meta pane's `Confidence` follows.
- [ ] On Windows, open a JPEG-only (view-only) folder and page through all of it, then press `Refit Ranking`. Expected: the fit line counts no folder for it.
- [ ] (per-file-viewed-ranking Step 2: the background refit at startup and on a folder switch; `crates/app/src/rank.rs` `refit_in_background`, `crates/app/ui/src/viewed.ts` `switchesFolder`; plan `docs/plans/_archived/20261014-per-file-viewed-ranking/plan.md`; the step's checkbox was ticked on the automated criteria only.) On Windows 11, with AI analysis on, in folder A (embedded by pass 3) sorted by `Recommended`, view and pick or reject a few frames of a burst, note A's order, then open folder B without pressing `Refit Ranking`. Expected: `Riffle.log` gets a `ranking fit:` line when B opens, and reopening A shows its Recommended order and `Confidence` changed. Clicking the open folder again in the folder tree, or switching the sidecar format in Settings (which reopens it), adds no fit line.
- [ ] On Windows, judge a few frames, quit and start Riffle again. Expected: a `ranking fit:` line at startup, and the Recommended order of the judged folder reflects the judgments without pressing `Refit Ranking`.
- [ ] On Windows, with AI analysis off, start Riffle and move between two folders. Expected: no `ranking fit:` line in `Riffle.log`.
- [ ] (per-file-viewed-ranking Step 3: a file counts as viewed only after its preview stayed on screen for `VIEW_DWELL_MS` = 500 ms; `crates/app/ui/src/viewed.ts` `dwellTimer`, `crates/app/ui/src/main.ts` `startDwell`; plan `docs/plans/_archived/20261014-per-file-viewed-ranking/plan.md`; the step's checkbox was ticked on the automated criteria only.) On Windows 11, open a fresh RAW folder Riffle has never shown, hold `ArrowRight` from the first frame across a dozen frames and release it, then wait. Expected: inspecting `index.sqlite` (`SELECT path FROM views WHERE dir = '<folder>'`) lists only the frame you started on (if it stayed up for half a second before you pressed) and the one you stopped on, not the frames flown past.
- [ ] On Windows, in that folder, press `ArrowRight` once and stay on the frame for over half a second, then move on. Expected: that frame gains a `views` row; a frame left within a quarter of a second does not.
- [ ] On Windows, with AI analysis on and the folder embedded by pass 3, pick one frame of a burst and dwell on another of the same burst without judging it, then press `Refit Ranking`. Expected: the `ranking fit:` line in `Riffle.log` counts the folder.
- [ ] (genre-ranking-offsets Step 2: one offset per top-level genre on top of the shared weights, `RANK_VERSION` 2; `crates/core/src/rank.rs` `fit` / `rank`, `crates/core/src/genre.rs` `of_burst`, `crates/app/src/rank.rs` `Stored`; plan `docs/plans/20261014-genre-ranking-offsets/plan.md`; the step's checkbox was ticked on the automated criteria only.) On Windows 11, with AI analysis on and a `ranking.json` from before this change in the app's local data dir, start Riffle. Expected: the startup `ranking fit:` line in `Riffle.log` ends with `per genre:` and the bursts of each genre that gave a pair (e.g. `per genre: sports 120, snap 9`), and `ranking.json` afterwards has `"rankVersion":2` and an `offsets` object keyed by `sports`, `portrait`, `landscape`, `birds / animals` and `snap`, each with 44 numbers.
- [ ] (genre-ranking-offsets Step 3: the meta pane's `Recommended` row names the genre the burst was ranked as; `crates/core/src/rank.rs` `Ranked.genre`, `crates/app/ui/src/meta.ts` `rankingRows`; plan `docs/plans/20261014-genre-ranking-offsets/plan.md`; the step's checkbox was ticked on the automated criteria only.) On Windows 11, with AI analysis on, open a folder embedded by pass 3 whose `Genre` filter lists `sports` and select a frame of a sports burst. Expected: the meta pane's `Recommended` row reads `N of M in its burst · +0.xx · as sports`, and a frame of a burst whose frames show no genre reads `N of M in its burst · +0.xx` with no suffix; `Reason` and `Confidence` read as before.

### Core: the ranking's pull toward the prior is untested for one to three culled folders

#### Background

`rank.rs` `fit` shrinks toward the built-in prior with tau = 1/(2C) = 5 in total. A single culled folder (about 70 scorable bursts) may outweigh that several times over, and the one measured move (0.300) came from 1602 bursts. Found in the `image-embeddings` Step 7 local review. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/core/src/rank.rs`, `docs/plans/_archived/20261012-image-embeddings/ranking.md`.

#### TODO

- [ ] Refit from the prior on 1, 3 and 5 culled folders, compare the held-out weighted AUC with the prior's, and pick the strength (for example tau scaled by the prior's burst count).

### App: no ranking refit when a viewed folder's embeddings arrive

#### Background

The ranking refits on `Refit Ranking`, at startup and when the open folder switches to another (`per-file-viewed-ranking` Step 2). A refit run before a viewed folder's pass 3 ran (AI off at the time, or the pass still running) fits without its frames, and nothing refits when its embeddings arrive. Deferred in `image-embeddings` Step 7. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/app/src/rank.rs` (`refit`), `crates/app/src/commands.rs` (`start_scan`).

#### TODO

- [ ] Refit at `embed-done` of a folder holding a viewed file or a reject.

### Core: the ranking's reject weight and confidence levels are choices, not fits

#### Background

`REJECT_WEIGHT` = 2 (`crates/core/src/rank.rs`) is a choice: the 58 explicit rejects (3 scorable bursts) in the author's data cannot measure how much stronger a reject should count than a frame merely not chosen. The confidence levels (`none` / `low` under 300 bursts / `medium` under 1000 / `high`, `crates/app/src/rank.rs` `confidence`) are read off Step 2's learning curve of the author's own folders, where the prior already is the user's taste. For another photographer the prior is someone else's, and the levels say nothing about how far the fit has moved from it. Found in `image-embeddings` Step 7 (`ranking.md`). Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`.

#### TODO

- [ ] Revisit `REJECT_WEIGHT` once a user's data holds enough explicit rejects to measure it.
- [ ] Base the confidence on how far the fit has moved from the prior, not only on the burst count.

### Core: portrait bursts lean negative under the per-genre ranking offsets

#### Background

The `genre-ranking-offsets` feature kept per-genre offsets on the shared ranking weights (`OFFSET_SHRINK` 30). No genre shows a significant gain. In the offline evaluation, portrait bursts (57) have a paired difference of -0.024 [-0.067, +0.003]; the technical-only shape's is significantly negative at -0.058 [-0.115, -0.016]. Unknown also leans negative, with an interval that includes zero. The plan's bar has no per-genre condition, so this did not fail it, but with only 57 portrait bursts it should be re-checked once more non-sports judgments have accumulated. Basis: `docs/plans/_archived/20261014-genre-ranking-offsets/offsets.md`. Files: `crates/core/src/rank.rs`, `docs/plans/_archived/20261014-genre-ranking-offsets/genre_offsets.py`.

#### TODO

- [ ] Once more non-sports judgments have accumulated, take a fresh app dump and re-run `genre_offsets.py` (it needs the scikit-learn venv, e.g. `D:\Photos\tests\.venv-eval\Scripts\python.exe` (the old `%TEMP%\embedvenv` was wiped on 2026-10-19); the run takes about 50 minutes on 20 workers and prints only at the end). If portrait's (or Unknown's) paired difference stays negative with an interval that excludes zero, raise `OFFSET_SHRINK` or drop the offsets for that genre; otherwise leave it as is.

### App: late embedding batches can outlive `clear_index` and the renames

#### Background

`stop_embedding` only sets the cancel flag, so a worker `flush` blocked on the index lock, or the task's trailing flush, can still write after `clear()` / `rename_dir` / `rename_file`. `drop_stray_embeddings` misses rows whose folder row survives. Found in the `image-embeddings` Step 7 local review. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/backend/src/index.rs`, `crates/app/src/commands.rs`.

#### TODO

- [ ] Make `write_embeddings` insert only when a `files` row with the same `path`, `size` and `mtime_ns` exists (`INSERT ... SELECT ... FROM files WHERE ...`), so every late write is a no-op, then remove `drop_stray_embeddings`. The embedding tests must create `files` rows first.

### App: the AI model's footprint (resident plan, kept 378 MB file, 190 MB fp16 option)

#### Background

Three leftovers from `image-embeddings` Step 3. (1) `embed::shared` keeps the SigLIP 2 plan (about 395 MB of working set) in a `OnceLock` for the rest of the process once one pass has run, even with `aiEnabled` off. Dropping it would cost the ~0.5 s build and the 773 MB load peak again on the next folder. (2) Turning AI off keeps `models/siglip2-base-patch32-256/vision_model.onnx` (378 MB) in the local data dir, and a cut download restarts from zero (no HTTP range resume). (3) tract runs the weight-only fp16 re-export (190 MB, `weight_only.py`) at 2-5x FP32's cost because it re-widens the weights on every run. Widening the float16 initializers in the ONNX protobuf before tract builds the plan (`tract_onnx::onnx().proto_model_for_read`, then `model_for_proto_model`) might give the smaller download at FP32 speed. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/core/src/embed.rs` (`shared`), `crates/app/src/ai.rs`, `docs/plans/_archived/20261012-image-embeddings/weight_only.py`, `docs/plans/_archived/20261012-image-embeddings/models.md`.

#### TODO

- [ ] Revisit dropping the plan after a pass if memory becomes a complaint.
- [ ] Add a "remove the model" action next to the AI checkbox, and consider HTTP range resume for the download.
- [ ] Try widening the fp16 initializers before the plan is built, and measure ms/image against the FP32 file (129 ms on one thread).

### App: AI analysis UI follow-ups (pass 3 progress, `genres_of` cost)

#### Background

Two leftovers from `image-embeddings` Steps 3 and 5. (1) `embed-progress` / `embed-done` are emitted but nothing listens, so the status line shows nothing while pass 3 runs (`crates/app/src/commands.rs` `start_scan`). Now that the Genre filter and the ranking read the vectors, an `embedding N / M` status may be wanted. (2) `genres_of` reads every valid embedding blob of the folder (about 3 KB each, 6.6 MB for 2134 files) and scores 9 labels on each call, after every `folder_entries` refresh and on each `embed-progress` while a Genre item is checked. It is expected to take tens of milliseconds and was never timed on a real folder (`crates/app/src/commands.rs` `genres_of`, `crates/backend/src/index.rs` `embeddings`). `aiEnabled` stays off by default: the user decided this on 2026-10-12, since on means a 378 MB download for every user. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`.

#### TODO

- [ ] Show an `embedding N / M` status from `embed-progress` / `embed-done`, or decide it is not wanted.
- [ ] Time `genres_of` with Timing logs on `D:\photos\2026\2026-09-19`. If it is slow, cache the labels per path in the backend.

### Core: soccer and macro genres need labeled positives to come back

#### Background

Decision 4 of `image-embeddings` dropped soccer, macro and other from the genre list: the labels hold no soccer or macro frame, so no threshold could be set or justified. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `docs/plans/_archived/20261012-image-embeddings/genres.md`, `crates/core/genres/`.

#### TODO

- [ ] Label frames from a soccer game and from a macro session, add them to `genre_prompts.json`, then re-run `genre_vectors.py` and `genre_calibrate.py`.

### App: real-device check of the recommended icon on the strip

#### Background

The recommended-icon feature marks the filmstrip cell of every frame the personal ranking places first in its burst with a Lucide `sparkles` icon, while AI analysis (`aiEnabled`) is on. CI and the vitest unit test cover the criterion `recommended(ranked)` (`place === 1 && of >= 2`). The icon's real rendering and its refresh behavior in the running app were never exercised: its placement beside the face icon in the bottom-left row, how that row sits against the burst count `n/m` at the bottom-right, and the updates after a refit, a sort or filter change, and turning AI off. Plan: `docs/plans/_archived/20261014-recommended-icon/plan.md`.

Files: `crates/app/ui/src/strip.ts`, `crates/app/ui/src/main.ts`, `crates/app/ui/style.css`, `crates/app/ui/src/recommend.ts`.

#### TODO

- [ ] On Windows, open a folder with bursts while AI analysis is on and the embeddings are in. Expect the cell of each burst's first-ranked frame to show the `sparkles` icon in the bottom-left row, right of the face icon when the frame is also a good frame (alone at the left edge otherwise). It must not collide with the burst count `n/m` at the bottom-right, and a lone frame must show none.
- [ ] On Windows, click `Refit Ranking` in Settings > Culling (icons may move). Change the sort and a filter (icons stay on the same files). Turn AI analysis off in Settings (every icon disappears).

### Core: keep or remove the ignored `detector_tract.rs` test left by ball-game-ranking-features

#### Background

The ball-game-ranking-features plan added `crates/core/tests/detector_tract.rs` in Step 3 as the check that a detector can ship. It is an `#[ignore]` test that reads `RIFFLE_DETECTOR_ONNX`, `RIFFLE_DETECTOR_INPUT` and `RIFFLE_DETECTOR_TENSORS`. Step 4 was meant to remove it or keep it as its seed, but Decision 1 (no detector ships) dropped Step 4. The test costs nothing in CI because it is ignored. It is the load / cost / box-agreement harness a later detector would reuse. Related: `docs/plans/_archived/20261014-ball-game-ranking-features/plan.md`, `docs/plans/_archived/20261014-ball-game-ranking-features/detect_eval.py` (`--tensors` / `--compare`).

#### TODO

- [ ] Decide whether to keep `crates/core/tests/detector_tract.rs` as the reusable harness or delete it. If kept, say in the file's header comment what it is for.

### Core: a plan to drop the genre from the personal ranking (scope decision pending)

#### Background

The `folder-wide-ranking-eval` feature re-measured the ranking folder-wide. Its Decision 1 (part 4) applied the pre-registered rule "keep the genre only when `+genre cols` or `+genre off` passes on either base". No genre comparison passed (bars met: 1, 3, 2 and 2 of 5). The shipped per-genre offsets (`W0+off` against `W0`) read +0.003 frame AUC, -0.7 points R-precision and -0.000 within-burst wAUC, so removing them changes nothing visible. The `reject-filter-eval` feature's Decision 2 (item 3) re-read the same question on a bottom-oriented, recall-constrained criterion (`RN@95`, the share of non-picks removed while keeping 95% of the picks) and confirms the drop. None of the four genre comparisons passes the comparison bar: `F0+ctx +genre cols` -0.7 points (p = 0.6159), `F0+ctx +genre off` +0.8 (p = 0.1498), `W0 +genre cols` +1.0 (p = 0.0640, 1 of 3 bars), `W0+off` +0.8 (p = 0.3110). The genre labels alone remove 7.2% of the non-picks at 95% pick recall, against 5.8% for random. The evidence is from one photographer's library, where 1,439 of 1,801 scorable bursts are sports. Decision 1 leaves the scope to the user: (a) ranking only, which the plan recommends, or (b) the whole feature. The `rank.rs` part touches `fit`, which another session owns, so this waits for that session to finish. Basis: `docs/plans/_archived/20261015-folder-wide-ranking-eval/plan.md` (Decision 1, parts 4-5), `docs/plans/_archived/20261015-folder-wide-ranking-eval/folderwide.md`, and `docs/plans/_archived/20261016-reject-filter-eval/plan.md` (Decision 2, item 3). Files: `crates/core/src/rank.rs`, `crates/core/src/genre.rs`, `crates/core/genres/siglip2.json`, `crates/app/src/commands.rs` (`genres_of`), `crates/app/ui/src/filter.ts`, `crates/app/ui/src/meta.ts`.

#### TODO

- [ ] Ask the user to choose the scope: (a) ranking only, or (b) the whole feature.
- [ ] After the session owning `rank.rs`'s `fit` is done, write and run a plan for the chosen scope.
  - (a) Remove `Weights.offsets`, `OFFSET_SHRINK` and the `of_burst` lookup in `fit` / `rank`. Bump `RANK_VERSION` so stored `ranking.json` offsets are ignored. Remove the meta pane's genre suffix after the ranking place. Keep `genre.rs`, `siglip2.json`, `genres_of` and `filter.ts`'s Genre section.
  - (b) Do (a), and also remove `genre.rs`, `crates/core/genres/siglip2.json`, `genres_of`, `filter.ts`'s Genre section and its docs (`CLAUDE.md`, `README.md` / `README.ja.md`, the usage docs). The embedding stays, because the ranking needs it.
- [ ] When this lands, delete `### Core: portrait bursts lean negative under the per-genre ranking offsets`, whose premise goes with the offsets.

### Core: the shipped ranking weights score single frames at chance

#### Background

The `folder-wide-ranking-eval` feature found that the within-burst weights do not carry over to single frames. Held out, `W0` reads singles AUC 0.508, against 0.611 for `F0+ctx` (13 folders with at least 5 single picks and 5 single non-picks). This is a reading aid, not a bar. The app ranks only within bursts today, so nothing shows it. If a later plan ranks or marks singles (a folder-wide "best" view), it should not reuse the within-burst weights as they are. Basis: `docs/plans/_archived/20261015-folder-wide-ranking-eval/folderwide.md`. Files: `crates/core/src/rank.rs`.

#### TODO

- [ ] Before any feature ranks or marks single frames, fit or evaluate a separate model for them (the pointwise `F0+ctx` shape reads 0.611) instead of reusing the within-burst weights.

### Agents: the archived plan scripts' import paths broke when their folders moved

#### Background

`docs/plans/_archived/20261014-folder-wide-pick-model/folder_gate.py`, `docs/plans/_archived/20261014-ball-game-ranking-features/ballgate.py` and `docs/plans/_archived/20261014-genre-ranking-offsets/genre_offsets.py` insert `<their folder>/../_archived/20261012-image-embeddings` on `sys.path`. They were written before the archive move, so that now resolves to `_archived/_archived/...`. `folder_gate.py`'s `GENRES` also points at `docs/crates/core/genres/siglip2.json`. Run directly, any of them fails at `import gate` (or at `extend`). They still work when imported after `gate`, as `folderwide.py` does, and `folderwide.py` overrides `GENRES`. Found in `folder-wide-ranking-eval` Step 2. Files: the three scripts above.

#### TODO

- [ ] Either fix the paths in the archived scripts, or state in the archive convention that a plan's scripts are frozen as run and must be imported the way `docs/plans/_archived/20261015-folder-wide-ranking-eval/folderwide.py` does.

### Agents: a plan linking a sibling plan in flight breaks lint on main once the sibling is archived

#### Background

PR #827 linked `../20261015-similar-scene-groups/plan.md`; PR #826 archived that folder and merged first. Each PR passed lint alone, but main's lychee (`mise run lint`) failed after both merged; the folder-wide-ranking-eval Step 1 PR fixed it by retargeting the link to `_archived/`. Basis: `docs/plans/_archived/20261015-folder-wide-ranking-eval/learnings.md` (Step 1, Deferred issues). Files: `.claude/skills/develop/SKILL.md` or the planner's plan-writing instructions, or a new `docs/agents/` plans guide.

#### TODO

- [ ] Put the rule in the chosen place: link only archived or stable plan targets from a plan, and re-run `mise run lint` after rebasing onto the latest main when a linked plan may move. Done when the procedure tells the author to re-run lint after rebasing onto a main where plans moved.

### Core: re-run the reject-filter evaluation once enough in-app rejects and views exist

#### Background

The `reject-filter-eval` feature measured the personal ranking, its cues and a few rule filters as a bottom filter (drop the clearly bad photos while keeping about 95% of the picks). It recommended no app change. The best filter, `G0` (gradient boosting on the `F0+ctx` columns), removed 16.8% of the non-picks at a label-free cut that kept 94.7% of the picks, against the pre-registered 25% floor. Its oracle cut removed 22.0%, also below the floor. Its pass of the comparison bar against the shipped weights also depended on a per-fold weight rescale that the pre-registration did not fix. The evaluation could not say whether a removed frame was bad: every non-pick counted as a negative, including fine-but-redundant frames, and only 58 explicit rejects in 3 folders were known to be bad. The conditions that would change the answer are:
- Explicit rejects and `views` rows (a viewed file left unflagged) accumulating in the app, so a filter can be measured and trained on "bad" rather than "unpicked".
- More folders and more photographers than the 27 folders, mostly basketball, of one photographer.
- `G0`'s weight scale pre-registered, or tuned in the inner CV, before it is read again.
- A calibrated label-free threshold (for example one learned on each fold's training folders) to close part of the 5-point gap between the label-free and oracle cuts. It cannot pass the bar alone on this data.
The `bad-photo-labels` feature later labeled 1,200 sampled frames for badness (plan: `docs/plans/_archived/20261017-bad-photo-labels/plan.md`, Decision). 5.52% of the dump is bad, about 94% of the non-picks are not bad, and no filter or dedicated model met its bar (best `R0` at q5: `BR` 19.3%, `P` 23.1%). So the non-pick criterion overstated the bad share about fifteenfold, and a re-run on in-app rejects would measure a different, smaller set of frames than that dump's non-picks.
The `discard-labels` feature measured the same premise for the "OK to discard?" question (plan: `docs/plans/_archived/20261018-discard-labels/plan.md`, Decision, item 1). 51.7% [44.9, 59.0] of the dump's non-picks are OK to discard, so "non-pick = negative" overstates it about twofold. It also misses the other way: 24.4% [8.6, 42.8] of the sampled picks (12 of 39) are OK to discard, so the picks are not a clean "keep" class. A re-run that treats unflagged or viewed-unflagged frames as negatives inherits both errors.
The `discard-retarget` feature (plan: `docs/plans/_archived/20261019-discard-retarget/plan.md`, Decision) re-read the premise from the other side: training on the discard labels instead of on picks (`D0`) did not give a better bottom cut than `G0`, partly because 24.4% of the sampled picks are themselves labeled `discard`. A re-run on in-app rejects therefore should not expect the pick-versus-non-pick target to be the main limit.
Basis: `docs/plans/_archived/20261016-reject-filter-eval/plan.md` (Decision 2, item 4) and `docs/plans/_archived/20261016-reject-filter-eval/reject.md`. Files: `docs/plans/_archived/20261016-reject-filter-eval/rejectfilter.py`, `docs/plans/_archived/20261016-reject-filter-eval/rejectablate.py`, `crates/core/src/rank.rs`.

#### TODO

- [ ] When the app's explicit rejects and `views` rows have accumulated (the user judges when), write a plan that re-runs the evaluation with those as the negatives, reusing `rejectfilter.py`, `rejectablate.py` and the `bad-photo-labels` `badfilter.py` bars (worth-a-filter `NL` <= 5%, `BR` >= 50%, `P` >= 50%). It must pre-register `G0`'s weight scale and a label-free threshold calibration before reading any score, and start from the finding that a bad-photo bar is far from met on the 80 labeled bad frames.

### App: real-device check that the bad-photo label sample opens in Riffle with 1,200 unflagged frames

#### Background

The `bad-photo-labels` feature sampled 1,200 frames of the dump into `D:\Photos\tests\2026-10-17-bad-labels\sample` for the user to label. Tests and the index cover that the folder holds 1,200 scanned `files` rows and that, after labeling, no copy carries a Pick, rating or color label (only 80 Rejects). The strip's visual reading was never confirmed: the user reported "done" without separately confirming it, and Step 1's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261017-bad-photo-labels/plan.md`.

Files: `docs/plans/_archived/20261017-bad-photo-labels/sample.py`, `docs/plans/_archived/20261017-bad-photo-labels/labeling.md`.

#### TODO

- [ ] On Windows, start Riffle, open `D:\Photos\tests\2026-10-17-bad-labels\sample`, sort by File name, clear all filters and let the scan finish. Expected: the strip shows 1,200 frames sorted from `0001_...`, and none shows a pick, reject or rating (labeled Rejects will now show; check an unlabeled copy of the sample if it still exists, otherwise treat the reading as moot and delete this item).

### Core: decide whether a bad-photo cue shown to the human replaces the removal filter

#### Background

The `bad-photo-labels` Decision found no usable removal filter. The base rate of bad frames is 5.52% [4.15, 7.04], and about 94% of the non-picks the earlier evaluations counted as negatives are not bad. The best filter, `R0` at q5, removes 19.3% of the bad frames at `P` 23.1%, against the 50% / 50% bars. The dedicated models `B0` / `L0` did not meet the +10 point bar. One cue catches bad frames: the folder's bottom sharpness decile, a bad rate of 17.7% [11.1, 25.5], which is 3.2x the base rate. Frames with no AF face read 14.6% (2.65x), outside the pre-registered bins. The Decision left open whether the goal stays a removal filter at these bars or a later plan pre-registers a different use, such as a cue shown to the human rather than a cut, with its own bar. No plan was opened. Basis: `docs/plans/_archived/20261017-bad-photo-labels/plan.md` (Decision, "Left for the user to decide") and `docs/plans/_archived/20261017-bad-photo-labels/badfilter.md`. Files: `docs/plans/_archived/20261017-bad-photo-labels/labels.py`, `docs/plans/_archived/20261017-bad-photo-labels/badfilter.py`, `docs/plans/_archived/20261017-bad-photo-labels/badmodel.py`.

The `discard-labels` feature later measured a different question, "OK to discard?" with "hide / sort last" semantics, on 600 blind labels (plan: `docs/plans/_archived/20261018-discard-labels/plan.md`, Decision). 48.45% [42.03, 55.11] of the frames are OK to discard, so the strict 5.52% base rate is the narrow subset. No `G0` cut is usable: q5 `RR` 17.3% [11.3, 24.0] against the 15% bar, q10 23.7%, q20 27.9%. A strict-bad rate sits below the discard rate in every `G0` band, so the two labels rank the bands alike. A cue shown to the human (a discard candidate marked, not cut) was not measured as such. The Decision found automation bias: the unblinded `RR@5` was about 1.4% (1 of 71) against 17.3% blind. A cue shown to the human therefore needs its own blind measurement.

The `discard-retarget` feature then tried to make a better removal cut on the same 600 blind labels (plan: `docs/plans/_archived/20261019-discard-retarget/plan.md`, Decision). A 2x2 of discard target and burst-relative features gave no usable cut under either bar. `G0+B` is not better than `G0` (`DP` -1.9 [-6.5, +2.9] at q5), and `D0` / `D0+B` have a weighted `RR` of 32.7-42.7% and a higher `PL`. `eye_focus` alone is still the best single score by AUC (0.777). So a cue shown to the human is still the only unmeasured use, and the retarget gives no evidence that a better model is nearby.

#### TODO

- [ ] Ask the user whether the goal stays a removal filter or becomes a cue shown to the human (for example marking the folder's bottom sharpness decile, or frames with no AF face).
- [ ] If a cue, write a plan that pre-registers its own bar (precision of the marked frames against the 5.52% base rate, share of the bad frames covered, and the cost of the marks) before reading any number. It reuses `labels.tsv` and the sampler.

### Core: decide whether a second bad-photo labeling round for the cue readings is wanted

#### Background

The `bad-photo-labels` labels hold 80 bad frames of 1,200, with 13 of 27 held-out folders holding none. The Decision does not expect a second round of the same kind to change its answer on a removal filter. A round for a different question could give the cue readings more power: `|pitch|` > 45 reads 3.2x the base rate on only 19 sampled frames, with an interval of [0.0, 40.5]. The Decision left this to the user. Basis: `docs/plans/_archived/20261017-bad-photo-labels/plan.md` (Decision, "Left for the user to decide"). Files: `docs/plans/_archived/20261017-bad-photo-labels/sample.py`, `docs/plans/_archived/20261017-bad-photo-labels/labeling.md`, `docs/plans/_archived/20261017-bad-photo-labels/labels.py`.

The `discard-labels` feature ran a blind 600-frame round for the "OK to discard?" question (plan: `docs/plans/_archived/20261018-discard-labels/plan.md`, Decision). It took the user about 30 minutes, about 3 s per frame against the 1.5-2.5 s estimated, so size a further round at about 3 s per frame (1,000 frames about 50 minutes). It did not oversample the `|pitch|` > 45 stratum.

The `discard-retarget` Decision (`docs/plans/_archived/20261019-discard-retarget/plan.md`) names what a further round would need if one is wanted: mostly more blind controls. The 150 controls stand for 25,071 frames (`π` down to 0.0042), so a `D0` fold's Kish effective sample size is only 166-195 of 504-597 labeled frames. The same Decision lists pre-registered model changes to try with it (clipped `1/π` weights, a linear `D0`, a target that keeps picks out of the cut). It sized the round at about 3 s per frame (600 frames about 30 minutes, 1,000 about 50 minutes). Rule branch 2 ("more labels") was not triggered, since effect A at q5 was negative.

#### TODO

- [ ] Ask the user whether a second labeling round is wanted and for which question (for example the `|pitch|` > 45 stratum). If so, write a plan that pre-registers the question and its bar, and oversamples that stratum with `sample.py`.

### App: real-device check that the discard-label sample opened with no judgment shown on the strip

#### Background

The `discard-labels` feature copied 600 frames into `D:\Photos\tests\2026-10-18-discard-labels\sample` for a blind "OK to discard?" review. Tests and the checks cover that the folder had 600 RAW copies and no `.xmp` / `.dop` before labeling (Step 2 listing), and that afterwards no sidecar held a Pick, Reject or color label (`labels.py`). The user viewed all 600 frames and reported none failing to open, with `aiEnabled` off throughout. Never confirmed: that the strip showed no judgment (rating, pick, reject, label) before labeling began. Plan: `docs/plans/_archived/20261018-discard-labels/plan.md`.

Files: `docs/plans/_archived/20261018-discard-labels/sample.py`, `docs/plans/_archived/20261018-discard-labels/labeling.md`.

#### TODO

- [ ] On Windows, if an unlabeled copy of the sample still exists (the labeled folder now carries 212 rating-1 sidecar pairs, so it no longer qualifies), open it in Riffle with `aiEnabled` off, sort by File name, clear all filters and let the scan finish. Expected: the strip shows 600 frames sorted from `0001_...` and none shows a pick, reject, rating or color label. Otherwise treat the reading as moot and delete this item.

### Core: decide whether `G0` as-is at per-folder q5-q10, sorted last, is worth shipping

#### Background

The `discard-retarget` feature (plan: `docs/plans/_archived/20261019-discard-retarget/plan.md`, Decision, "Facts the user may weigh") found no new model that beats `G0` and recommended nothing. Under the looser bar the user chose (`RR` <= 25% with the stratified 97.5% quantile <= 35%, `PL` <= 5%, removed share >= 4.5%, `DP` 2.5% quantile above the 48.45% base rate), the existing `G0` itself meets all of those conditions at q5 and q10: `RR` 17.3% [11.3, 24.0] and 23.7% [18.7, 28.7], `PL` 0.68% and 2.10%, removed 4.92% and 9.87%. It was only a reference there, so the "beats `G0`" condition could not apply to it. That leaves a separate question this plan did not answer: is `G0` at q5-q10, sorted last (not hidden), worth shipping as it is? A yes needs its own pre-registered question and bars, and a fresh blind confirmation round. The looser bar was chosen after `G0`'s numbers on these 600 labels were read, and `discard-labels` found automation bias (unblinded 70 of 71 against the blind `RR@5` of 17.3%). Left to the user as a question only; no app work follows from the plan.

The `model-search` feature (plan: `docs/plans/_archived/20261020-model-search/plan.md`, Decision, "Facts the user may weigh") read `G0` on its 600 fresh labels. q5 again meets conditions 1-4: `RR` 23.5 [15.7, 30.9], cluster 97.5% 34.9, `PL` 0.68%, removed 4.53%, `DP` 2.5% 69.1 above the 42.31% base rate. q10 fails `RR` (30.1 [23.7, 35.8]). The strata were cut for the model-search winner, so this is a reading, not the pre-registered confirmation round this item asks for.

The `discard-model` feature found that the plain `ef` cue beats the learned `LR/T`, a cue to weigh against shipping `G0`. The question of shipping `ef` / `ef-fill` sorted last has since been closed: no sort-to-last, and the `Face focus` filter's `Low` item is the shipped form (plan: `docs/plans/_archived/20261107-face-focus-tiers/plan.md`).

Files: `docs/plans/_archived/20261019-discard-retarget/plan.md`, `docs/plans/_archived/20261019-discard-retarget/retarget.md`, `docs/plans/_archived/20261020-model-search/modelsearch.md`, `crates/core/src/rank.rs`.

#### TODO

- [ ] Ask the user: is the existing `G0` at per-folder q5-q10, sorted last (not hidden), worth shipping as it is? If no, delete this item. If yes, write a plan that pre-registers the question and its bars and starts with a fresh blind confirmation round (candidates and controls mixed and unmarked, about 3 s per frame, 600 frames about 30 minutes). Run the offline Python with `D:\Photos\tests\.venv-eval\Scripts\python.exe -I -B` (the old `%TEMP%\embedvenv` was wiped).

### App: real-device check of the YuNet values and landmarks on the focus mark

The `face-detection-values` plan keeps YuNet's nose and mouth landmarks in `Face`, carries every face's score and landmarks through `faces_of`, and draws a label (score, box size, eye distance in stored preview px) and five landmark dots beside each face box in cyan `#3ff`, with the face mesh drawing switched to magenta `#f3f`. Unit tests (`focus.test.ts`, `faces.test.ts`, the `faces.rs` tests) and `mise run ci` cover the label text, the upright placement on all four orientations and the coordinate mappers. Drawing on a real preview was never exercised, so the label legibility on rotated frames, the clamp inside the image and the colors are unchecked. Plan: `docs/plans/_archived/20261021-face-detection-values/plan.md`.

Files: `crates/app/ui/src/main.ts`, `crates/app/ui/src/focus.ts`.

#### TODO

- [ ] On Windows, open a folder with faces in landscape and portrait (orientation 6 / 8) frames and, if available, a half-turned (3) one, and press `f`. Each face box has a cyan label like `0.93  212×254  eyes 61` reading upright above the box (just inside it when the box touches the image's top). There are five cyan dots on the eyes, nose tip and mouth corners, and no dot between the eyes. The face mesh outline and irises are magenta. The 1:1 view (`z`) and Compare draw no faces.

### App: real-device check that the model-search sample opened with no judgment shown in Riffle

#### Background

The `model-search` feature copied 600 frames into `D:\Photos\tests\2026-10-20-model-search\sample` for a blind "OK to discard?" review. `confirmsample.py` checked 0 sidecars in `sample\` and the copies' bytes, and Step 5 found the labeled sidecars hold no mark other than rating 1, with `aiEnabled` off throughout. Never exercised: that the app showed the folder unjudged before the first key press. The sample is now labeled, so this can only be checked on a fresh copy of the unlabeled files. Plan: `docs/plans/_archived/20261020-model-search/plan.md`.

Files: `docs/plans/_archived/20261020-model-search/confirmsample.py`, `docs/plans/_archived/20261020-model-search/labeling.md`.

#### TODO

- [ ] On Windows, with `aiEnabled` off, open an unlabeled copy of the sample folder (no `.xmp` / `.dop`), sort by File name and let the scan finish. Expected: 600 files `0001_...` to `0600_...`, every one without a rating, Pick / Reject flag or color label, no `.xmp` / `.dop` in the folder before the first key press, and no recommended icon or ranking rows.

### App: real-device checks for Rename by Capture Date…

#### Background

The rename-folder-by-capture-date feature added `Rename by Capture Date…` to the folder tree's right-click menu. It renames a one-date folder through `rename_folder`'s path, or splits a several-date folder into sibling per-date folders. Unit tests on temp dirs cover the planner, the move planning and rollback, and the "remove when empty" rule. Vitest covers the dialog's text helpers and flow. The real watcher release on Windows, and the dialog and the tree's follow-up in a running app, were never exercised. Plan: docs/plans/_archived/20261104-rename-folder-by-capture-date/plan.md.

Files: `crates/app/src/datefolder.rs` (`split_blocking`), `crates/app/src/treewatch.rs` (`with_released`), `crates/app/ui/src/main.ts`, `crates/app/ui/index.html`.

#### TODO

- [ ] On Windows, split a folder that is expanded in the folder tree and open in the strip, with photos of two dates, `.xmp` / `.dop` sidecars and no undated file, via `Rename by Capture Date…`. Expected: the two sibling date folders hold the files with their sidecars, the original folder is removed (no `PermissionDenied` from a held watch handle), and the first date folder opens with its judgments and thumbnails intact.
- [ ] On Windows and macOS, right-click a folder in the tree and choose `Rename by Capture Date…` on (a) a folder of one date: the dialog reads `Rename <name> to YYYY-MM-DD?` with the file count, and `Run` renames it in the tree and reopens it if it was open. (b) A folder of two dates with an undated file: one row per date folder, and the undated line and the removal note show. `Run` creates the sibling folders, the tree re-lists the parent, and the original stays with the undated file and is re-listed if open. (c) The same without an undated file while open: the original disappears from the tree and the first date folder opens. (d) A sibling already named a target date: the reason shows in red and `Run` is disabled. (e) During a scan: the status line shows the scan message and no dialog opens.

### App: the undo history's judgment entries name a split's moved files by their old paths

#### Background

After `Rename by Capture Date…` splits a folder, the undo history's judgment entries still name the moved files' old paths. Only a folder rename's trash entries are rebased, by `folderRenamed`. An undo of a judgment made before the split would write to a path that no longer exists. `date_folder_run`'s `Split` result does not list the per-file moves, so the frontend cannot rebase them. Plan: docs/plans/_archived/20261104-rename-folder-by-capture-date/plan.md.

Files: `crates/app/ui/src/main.ts` (`runDateFolder`), `crates/app/src/datefolder.rs`.

#### TODO

- [ ] Return the per-file moves (old path to new path) in the split result and rebase the judgment undo / redo entries from them, or drop the entries of the moved files.

### App: a split moves a shared sidecar with whichever RAW moves first

#### Background

Two RAWs sharing a stem (`a.ARW` and `a.NEF`) share `a.xmp`. `Rename by Capture Date…` moves that sidecar with whichever RAW moves first, which leaves the other RAW without it when the two have different dates. This is the same ambiguity `rename::file_plan` has. Plan: docs/plans/_archived/20261104-rename-folder-by-capture-date/plan.md.

Files: `crates/app/src/datefolder.rs` (`move_plan`), `crates/app/src/rename.rs`.

#### TODO

- [ ] Decide whether a sidecar shared by RAWs of different dates should be refused, copied, or left. Apply the same rule to `rename::file_plan` if it changes.

### Agents: `wait-pr-actionable.sh` returns `ACTION=blocked` while checks are still pending

#### Background

The `ef-gate-fill` feature (plan: `docs/plans/_archived/20261104-ef-gate-fill/plan.md`) saw `.claude/skills/pr/scripts/wait-pr-actionable.sh` return `ACTION=blocked` with `pendingCount=0` right after a PR was created, while `statusCheckRollup` still showed pending checks (PRs 887, 888, 899, 901, 903, 904). The cause is unverified. Likely candidates are a BLOCKED `mergeStateStatus`, or an empty check list read before the checks register. The import-from-card feature (plan: `docs/plans/_archived/20261106-import-from-card/plan.md`) saw the same on PRs 894, 898, 902, 906 and 908: `ACTION=blocked` with `checksOverall=success` and `pendingCount=0` right after a push while the PR's checks were still `IN_PROGRESS`; #898's `pr-runner` aborted on it. So the script is not counting in-progress checks as pending. Files: `.claude/skills/pr/scripts/wait-pr-actionable.sh`.

#### TODO

- [ ] Read the script and reproduce on a fresh PR. Treat an empty or pending rollup as "keep waiting", and make `pendingCount` count the rollup's pending entries. Count `IN_PROGRESS` checks as pending, and return `blocked` only once no check is in progress. Done when a PR polled right after creation never returns `blocked` while any check is pending, shown by a test or a recorded transcript. Merge into an existing pr-runner polling item if one covers it.

### Agents: `test (ubuntu-latest)` hangs on docs-only commits

#### Background

The `ef-gate-fill` feature (plan: `docs/plans/_archived/20261104-ef-gate-fill/plan.md`) hit the `test (ubuntu-latest)` job stalling on docs-only commits. PR 904's first run was cancelled. Main run 42128694204 hit the job's 30-minute `timeout-minutes` with the `riffle_core` test binary running. Both reruns took about 4 minutes, against the usual 5-6. PR 901's ubuntu job stalled in `actions/cache`. Files: `.github/workflows/`, `docs/agents/github-actions-workflows.md`.

#### TODO

- [ ] Collect the hung runs' logs and decide whether the cause is a flaky test, a cache stall or a runner problem. Then lower the job timeout, add a per-test timeout, or add a "rerun, then check X" note to `docs/agents/github-actions-workflows.md`. Done when the cause is identified, or a decision that reruns are enough is recorded with a rerun-only rule in that guide.

### App: real-device checks for Import… (import-from-card)

#### Background

The import-from-card feature added a Settings > `Import` tab (destination root and folder pattern), the folder tree's `Import…` item, and cull-while-importing (the first date folder opens at run start and the strip fills as files land). Unit tests on temp dirs, Vitest and `mise run ci` cover the pattern validation, the copy pipeline, the planner, the run, the flow's state machine and the watcher's temp-file filter. The settings tab, the dialog and the run in a running app were never exercised, nor was cull-while-importing on a real card. Plan: docs/plans/_archived/20261106-import-from-card/plan.md.

Files: `crates/app/ui/src/settings.ts`, `crates/app/ui/index.html`, `crates/app/ui/src/import.ts`, `crates/app/ui/src/main.ts`, `crates/app/src/import.rs`, `crates/app/src/watch.rs`.

#### TODO

- [ ] On Windows (and macOS if available), open Settings > Import and press `Choose…` to pick a folder. Expected: the root field shows it and the example line reads `<root>\<YYYY>\<YYYY-MM-DD>` for today. Type `%Y/%H`: the error shows in red under the input. Blur: the stored pattern is kept (reopen the app and the field shows the last valid pattern). Click each example button (`%Y/%F`, `%F`, `%Y/%m/%d`, `%Y/%m`): the input and the example follow, and after a restart the stored value follows too.
- [ ] On Windows, with a memory card (or any folder of RAWs / JPEGs) and the root set, right-click the card's root in the folder tree and choose `Import…`. Expected: the dialog lists the source, one row per date folder with its file count, size and skipped count, the colliding and undated names in their own lists, and `Run` enabled. Run it: the status line shows `importing N / M`. An uncanceled run ends with `Imported N files (X MB)...`. The date folders hold the copies with the card's modified times and no `.riffle-tmp` file, and `Riffle.log` has one `import:` line. Choose `Import…` again on the same card: the note "Nothing left to copy" shows and `Run` is disabled. With the root unset, the refusal shows in the dialog and `Run` is disabled. While the import dialog is up, Sequence, Rename by Capture Date, the sidecar items and Settings do not open.
- [ ] On Windows, with a card never written to and the root set to a test root such as `D:\Photos\tests\20261106-import-step5\` (delete it afterwards), open another folder first, then `Import…` on the card's root and `Run`. Expected: the dialog closes at once. The earliest date folder opens (empty at first) and is revealed and selected in the tree with its parents expanded. The first file appears in the strip and the preview about a second after it lands. The strip keeps growing in batches while the status line shows `importing N / M`. The current file stays current, and flags / ratings set meanwhile stay. No `*.riffle-tmp` ever shows in the strip. `Riffle.log` (Timing logs on) shows `rescan:` lines about once per scan cycle, not one per file. With 500+ files the arrow keys and the preview stay responsive. The date folder's count badge in the tree grows. While files land in a second date folder the strip does not change. `Import…` again during the run brings the dialog back, Escape only closes it (the run continues), and `Cancel` stops it with `Import canceled, ...`. With `aiEnabled` on, the embeddings fill mostly after the import ends and the strip does not stall.

### Core: re-measure the import copy pipeline against Explorer on a fast destination

#### Background

The import-from-card feature's `riffle-cli copybench` measured the copy pipeline (8 MiB x 4 buffers, per-file `sync_all`) only against `D:` (a Crucial BX500, DRAM-less, 96 % full). That drive capped every method at 8-22 MB/s, including `robocopy`, while the card read alone ran at 135 MB/s. The numbers cannot tell the methods apart, and Explorer itself was never timed. The defaults stay at 8 MiB x 4, and `FILE_FLAG_SEQUENTIAL_SCAN` and dropping `sync_all` showed no gain on that drive. Plan: docs/plans/_archived/20261106-import-from-card/plan.md.

Files: `crates/core/src/copy.rs`, `crates/cli/src/main.rs`, `docs/humans/performance.md`.

#### TODO

- [ ] Copy the same fresh, never-read files from the card into a destination on a faster or emptier drive (the NVMe `C:`) by `riffle-cli copybench <card-dir> <dst-dir>` and by an Explorer copy, with idle time between runs, and record the MB/s. Done when the table in `docs/humans/performance.md` has the Explorer row and the 8 MiB x 4 defaults and the per-file `sync_all` are confirmed or changed.

### Core: `copy::copy_files` can replace a destination created between its check and the rename

#### Background

The import-from-card feature's `copy::copy_files` checks `dst.exists()` before reading and again just before `fs::rename`, but `fs::rename` replaces the destination on every platform. A file created at the destination in that window, such as a sidecar the app writes for a RAW that just landed, would be replaced. A per-OS no-replace rename closes it (`MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, `renameat2` with `RENAME_NOREPLACE`, `renamex_np` with `RENAME_EXCL`). Plan: docs/plans/_archived/20261106-import-from-card/plan.md.

Files: `crates/core/src/copy.rs`.

#### TODO

- [ ] Replace the final `fs::rename` with a no-replace rename on Windows, Linux and macOS, and keep the failing-job message `exists: ...`. Done when a unit test that creates the destination between the check and the rename (through a hook or a stub) leaves the existing file untouched on the platform CI runs.

### Docs: CLAUDE.md's description of `src/filter.ts` omits the `Output` section

#### Background

`CLAUDE.md`'s layout paragraph says `src/filter.ts` covers the strip's filter menu, including the `Face focus`, `Eyes` and `Genre` sections. It does not mention the `Output` section added by `filter-by-output`, which is per-session like `Genre` and shown while an `Output` / `Output-sequenced` folder is readable. Plan: `docs/plans/_archived/20261106-filter-by-output/plan.md`. Files: `CLAUDE.md`, `crates/app/ui/src/filter.ts`.

#### TODO

- [ ] Add the `Output` section (`Output` / `No output`, from the `outputs_of` command) to that description in `CLAUDE.md`.

### App: real-device check of Move to Trash pressed during a scan (trash-over-scan Step 1)

#### Background

`trash-over-scan` Step 1 makes Move to Trash cancel a running scan and run, instead of waiting for it to end. Tests and CI cover the logic, but the step's checkbox was ticked on the automated criteria, and the app was never launched against a large cold folder on a real machine. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/app/src/commands.rs`, `crates/app/ui/src/main.ts`.

#### TODO

- [ ] On Windows, open a large folder cold (or after Clear Cache), filter to non-picked, select all, and press Move to Trash during `scanning N / M`, then again in another run during `analyzing N / M`. Expect the move to start within seconds. Expect `Riffle.log` (Timing logs on) to show `scan extract ... canceled=true` or `scan faces ... canceled=true`, then the trash's rescan with `scan reconcile ... removed=K` and a smaller `todo`. Expect no `faces-done after scan-done` or `start_scan once per scan` invariant line. Undo restores the files and the next rescan picks them up.
- [ ] On Windows, press Move to Trash during a scan, then rename a file before the move ran. Expect the rename to run and the folder to still finish its scan.

### App: real-device check of the Move to Trash progress line and menu guard (trash-over-scan Step 2)

#### Background

`trash-over-scan` Step 2 streams `trash-progress` as a status line (`moving to Trash N / M`), greys `Move to Trash…` in the strip's menu while a move runs, and refuses a second move. Automated criteria passed, but the app was never run against a real 200+ file move. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/app/ui/src/main.ts`, `crates/app/src/commands.rs`.

#### TODO

- [ ] On Windows, move 200+ files. Expect the status to count up (`moving to Trash N / M`) and clear when the run ends. Expect `Move to Trash…` to be greyed in the strip's menu while it runs. Reopening the folder during the move (File > Reload, or the tree) keeps the line and starts the rescan after the move. Redo shows the count too. Undo still restores. The strip's menu and the tree are only reachable in the held path, so press Move to Trash during a scan to check those two.

### App: real-device check that the UI stays responsive during a long Move to Trash (trash-over-scan Step 3)

#### Background

`trash-over-scan` Step 3 moved `scan_folder`'s `Scans` lock wait off the runtime workers, so a reopen during a move no longer parks them. Existing scan tests pass, but no test drives the `AppHandle` path and it was not run on a real machine. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/app/src/commands.rs` (`scan_folder`).

#### TODO

- [ ] On Windows, start a move of 200+ files and reopen the folder during it. Expect the UI to stay responsive (strip scroll, meta pane, other folders in the tree expand). Expect the `scan list` line to appear right after `moved files to the trash` in `Riffle.log`.

### App: real-device check of the chunked Move to Trash on Windows (trash-over-scan Step 4)

#### Background

`trash-over-scan` Step 4 moves files to the Trash in chunks of 100 on Windows (`run_chunked`), measured at about 7 ms a file against 200 ms per-file (see `docs/plans/_archived/20261106-trash-over-scan/trash-bench.md`). Tests use a fake mover and the bench used copies; the real app path was never exercised, and a locked file in a real chunk is covered only by the fake mover and the bench probe. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/backend/src/trash.rs` (`run_chunked`, `move_chunk`), `crates/app/src/commands.rs` (`move_groups`, `move_again`).

#### TODO

- [ ] On Windows, in a test folder of copies, move 500+ files. Expect `moved files to the trash` in `Riffle.log` at about 7 ms a file, near `trash-bench.md` (a large gap means the app adds a cost the bench did not have). Expect the status count to go up in steps of 100. Undo restores all, each sidecar with its RAW. Redo moves them again.
- [ ] On Windows, hold one file open, then move a selection around it. Expect that file listed as a failure with its sidecars left, and the rest moved and restored by Undo.

### App: real-device check of Delete Permanently (trash-over-scan Step 5)

#### Background

`trash-over-scan` Step 5 adds `Delete Permanently…` to the strip's menu (`delete_files`), with a mouse-click-only confirm (`confirmsDelete`, Enter/Space swallowed). The step's checkbox was ticked on the automated criteria, as Riffle was not launched. `confirmsDelete` assumes the second click of a double-click reaches `#trash-run` with `detail` 2 though its target changed; nothing has run that on WebView2. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/app/ui/src/main.ts` (`confirmTrashFiles`, `trashKeydown`, `runTrash`, `trashFilesIn`), `crates/app/src/commands.rs` (`delete_files`).

#### TODO

- [ ] On Windows, in a test folder of copies under `D:\Photos\tests\<date>-delete-permanently\` (never the user's photos), delete a selection with sidecars. Expect the files and sidecars to be gone from the folder and absent from the Recycle Bin, and free space on D: to go up by their size. `Edit > Undo` has no entry for it. The status shows `deleting N / M` then `Deleted N files`, and the strip rescans and shows the rest.
- [ ] On Windows, with the confirm dialog open, press Enter and Space, also after Tab to the run button and with Shift held. Expect nothing deleted; only a mouse click deletes.
- [ ] On Windows, press delete during a scan. Expect it to run after the scan stops. Press delete, then open another folder before it ran. Expect nothing deleted.
- [ ] On Windows, double-click `Delete Permanently…` from a menu opened over the viewer, so the run button appears under the cursor. Expect nothing deleted.
- [ ] On Windows, with `a.ARW`, `a.DNG` and `a.xmp` in the folder, delete `a.ARW` alone and expect `a.xmp` kept. Then select both RAWs and expect `a.xmp` removed.

### App: Undo of Move to Trash leaves an orphaned `$I` file per restored file on Windows

#### Background

Found by `trash-over-scan` Step 4's measurement. trash 5.2.9's Windows `restore_all` moves the `$R` file back and nothing removes its `$I`. Riffle's Undo restores through the same call, so it presumably leaves one for every file it brings back. D: already held 5327 older orphans naming other paths (source not traced, left alone). Each is small and the shell does not list them, but they pile up and cost a file record each. See `docs/plans/_archived/20261106-trash-over-scan/trash-bench.md` (safety record).
Files: `crates/app/src/commands.rs` (`restore_run`).

#### TODO

- [ ] Check whether Explorer's own restore cleans up its `$I`, and whether the crate (or Riffle after `restore_all`) can remove the orphan.

### App: Undo of Move to Trash is still one shell operation per item

#### Background

After `trash-over-scan` Step 4 chunked the move, `trash_rejected_undo` is still per-item, one shell operation each. Not timed, but if an operation costs what a move did (about 200 ms), an Undo of a large move takes far longer than the move. One `restore_all` for the whole run took 1 s for 150 files in the bench, but `trash::restore`'s per-file rule (a sidecar waits on its RAW, never overwrite) would need the same treatment `run_chunked` gave `run`. See `docs/plans/_archived/20261106-trash-over-scan/trash-bench.md`.
Files: `crates/backend/src/trash.rs` (`restore`), `crates/app/src/commands.rs` (`restore_run`).

#### TODO

- [ ] Time Undo of a 500+ file move on Windows.
- [ ] If slow, restore in chunks while keeping the RAW-before-sidecar and never-overwrite rules.

### App: other async commands park a runtime worker on the `Scans` lock during a move

#### Background

`trash-over-scan` Step 3 moved `scan_folder`'s lock wait off the runtime workers. `rename_folder`, `rename_file`, `date_folder_preview`, `date_folder_run`, `rewrite_sidecars_preview`, `delete_sidecars_preview` and `clear_index` still take `Scans` on a runtime worker for their `scanning()` guard, so each pressed during a move parks a worker until the move ends. They are single user actions so few can pile up, but moving the guards into `spawn_blocking` would close the same hole. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/app/src/rename.rs`, `crates/app/src/datefolder.rs`, `crates/app/src/foldersidecars.rs`, `crates/app/src/commands.rs`.

#### TODO

- [ ] Move those guards into `spawn_blocking`, as Step 3 did for `scan_folder`.

### App: `scan_folder` calls waiting out one move may take their ids out of order

#### Background

Found by reading in `trash-over-scan` Step 3, not reproduced. Two `scan_folder` calls waiting out one move (reopen, then open another folder) mint their ids in the order they get the lock, which `std::sync::Mutex` does not promise is the order they asked. Inverted, the frontend's newer call gets the stale id and its scan is dropped, and the older call's `pending` entry, which the frontend ignores, keeps `scanning()` true until the next `scan_folder`. Not introduced by Step 3 (the workers waited on the same mutex). Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/app/src/commands.rs` (`scan_folder`, `start_scan`), `crates/app/ui/src/main.ts` (`startScan`).

#### TODO

- [ ] Try to reproduce on Windows (reopen, then open another folder, both during a 200+ file move).
- [ ] If it reproduces, mint ids before waiting on the lock or order the waiters.

### App: Delete Permanently cannot remove a read-only file on some platforms

#### Background

From `trash-over-scan` Step 5. A read-only file (a camera's protect flag) goes to the Trash but may fail `remove_file`, depending on the platform. It is then listed as a failure and stays, which is safe, but the user has no way to delete it from Riffle. Not tried.
Files: `crates/backend/src/trash.rs` (`remove_one`).

#### TODO

- [ ] On Windows, delete a read-only file with `Delete Permanently…` and see whether it fails.
- [ ] If it fails, decide whether to clear the read-only flag first or leave it.

### App: a slow or shifted double-click on Delete Permanently… can confirm the dialog

#### Background

From `trash-over-scan` Step 5. `confirmsDelete` takes a click only with `detail` exactly 1, which stops the second click of a fast double-click (`detail` 2) from hitting the run button that appears under the cursor. A slow or shifted double-click arrives with `detail` 1, so if the run button sits under the cursor that click would confirm a permanent delete. Also, Space clicks on keyup and the guard relies on `preventDefault` on keydown, which could not be tried here. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/app/ui/src/main.ts` (`confirmsDelete`, `runTrash`, `confirmTrashFiles`).

#### TODO

- [ ] Decide on a guard, such as placing the dialog so the run button is never under the menu item, or arming the run button only after a short delay.
- [ ] Try Space on the focused run button on WebView2 (also covered by the Step 5 real-device item).

### App: Move to Trash progress is dimmed behind the dialog, and Delete Sidecars and Undo are not refused during a move

#### Background

From `trash-over-scan` Step 2. (1) A move that was not held keeps the confirm dialog open in its running phase until the invoke settles (deliberate, per the comment above `trashFilesIn`), so the `moving to Trash N / M` line shows dimmed behind the `.dialog` overlay and the dialog still asks its question with disabled buttons. Closing the dialog when the run starts, or showing the count in it, would make the progress plainer but gives the strip the keyboard back mid-move. (2) `Delete Sidecars…` and Undo are not refused while `trashing !== null`: they queue on the `Scans` lock as before, and neither touches `trashing`, so a long move can leave them waiting with nothing said about them. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/app/ui/src/main.ts` (`trashFilesIn`, `closeTrashDialog`, `undoTrash`, the sidecar flow).

#### TODO

- [ ] Show the running count in the dialog or close the dialog when the run starts, without giving the strip keyboard focus mid-move.
- [ ] Refuse or label `Delete Sidecars…` and Undo while `trashing !== null`.

### App: split the disk read from `hevc::to_jpeg` so a read slot covers the read alone

`read_preview` ends in the HEVC decode of an HDR PQ CR3 (65-125 ms of CPU per file), and that runs inside `ReadSlots::acquire` in the three scan sites and inside `preview`'s pause. With `READ_SLOTS` at 2 and no HDR PQ CR3 run in the sweep, a folder of such files pays a calculated 33-63 ms/file against 3-6 uncapped, while the permits sit idle at the disk. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`. Files: `crates/core/src/scan.rs`, `crates/core/src/reader.rs`, `crates/app/src/commands.rs` (`preview`).

#### TODO

- [ ] Have a `reader` function return the undecoded `(arw, bytes, codec)` so the guards cover the bytes and `to_jpeg` runs outside them. Done when a slot and the pause no longer span the decode and a run on a folder of HDR PQ CR3 files shows the pass's ms/file at cap 2.

### App: prefetch the neighbouring previews during a scan

`preview-read-gate` capped the wait of the first preview but left out prefetching the neighbours, one of the three fixes offered to the user. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`. Files: `crates/app/ui/src/main.ts`, `crates/app/src/commands.rs` (`preview`).

#### TODO

- [ ] Decide after measuring paging during a scan whether prefetching the previews next to the current file is still needed at a read cap of 2; each prefetch would pause the scan's reads too.

### App: stop waiting on a stale preview request

`requestPreview` is single-flight, so paging past a file can wait for that file's read before the newest one starts. `preview-read-gate` left this out, as one of the three fixes offered to the user. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`. Files: `crates/app/ui/src/main.ts`.

#### TODO

- [ ] Measure how often a stale request holds back the current file's preview at a read cap of 2, then decide whether the current file should stop waiting for it.

### App: pause the scan's reads for the other read commands if they show late

Only `preview` pauses the scan's reads. `metadata`, `faces_of`, `eyes_of` and `focus_crop` read the same prefix right after, normally from the cache, and each pause would be one more per page. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`. Files: `crates/app/src/commands.rs`.

#### TODO

- [ ] If the meta pane or the 1:1 crop is seen late during a first scan, measure those commands' reads and add a pause spanning the read only, never a model run or a decode.

### App: measure the scan's read cap on NVMe

`READ_SLOTS` (2) was picked on a SATA SSD; no RAW folder was on the NVMe (`C:`) at the time. A shallower queue costs more on a drive that needs deep queues for throughput, so cap 2 may slow the thumbnail fill there. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`. Files: `crates/app/src/commands.rs` (`READ_SLOTS`, `preview_during_pass_1_on_a_real_folder`).

#### TODO

- [ ] Put an unopened RAW folder on the NVMe, close the app and run `preview_during_pass_1_on_a_real_folder` at `RIFFLE_READ_SLOTS` 22 and 2, on cold data. Done when `docs/agents/tauri-app.md`'s read cap section has the numbers and the cap is confirmed or changed.

### App: test the scan's read slot in `embed_all` and the post-read cancel check

#### Background

`embed_all`'s per-file closure needs a loaded SigLIP model, which the tests lack, so its `ReadSlots::acquire` is checked only by reading the code. `a_file_canceled_mid_pipeline_is_not_delivered` now stops at `acquire` returning `None`, because nothing can set the flag during the read without a hook in `read_preview`, so the first post-read cancel check in `extract_unless` and `extract_analysis_unless` is untested. File: `crates/core/src/scan.rs`. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`.

#### TODO

- [ ] Add a test seam (a stub embedder or an injectable closure), and a test that holds the single permit and shows `embed_all` delivers nothing until it is dropped.
- [ ] Add a test-only hook or a reader stub that sets `cancel` during the read, and a test that a file canceled mid-read is not delivered, in both `extract_unless` and `extract_analysis_unless`.

### Core: `good_photo` in the ranking leaves out the full-size eye cut that `photoTier` applies

#### Background

`full-size-eye-check` Step 4 added a full-size eye-sharpness cut to the frontend's `photoTier`. `good_photo` in `crates/core/src/rank.rs` deliberately leaves it out, so its features do not move in that feature, and its doc comment says so. The Rust copy therefore disagrees with the frontend on a demoted frame. Applying the cut to the ranking moves the ranking's features, so the prior's calibration needs a look. `rank::Frame::full_eye_sharp` is already carried through `Index::rank_rows` for this and nothing reads it yet. Plan: `docs/plans/_archived/20261108-full-size-eye-check/plan.md`. Files: `crates/core/src/rank.rs`, `crates/backend/src/index.rs`.

#### TODO

- [ ] Decide whether `good_photo` applies the full-size cut (`full_eye_sharp` against `SOFT_BELOW`). If it does, apply it, check the effect on the prior (`crates/core/ranking/siglip2.json`, written by `rank_prior.py`) and refit or recalibrate as needed. If it does not, document the divergence from `photoTier` where both are defined.

### Core: the cold-disk cost of pass 2's full-size eye read is unmeasured

#### Background

`full-size-eye-check` Step 3 measured pass 2 on `D:\Photos\2026\2026-11-07` (3638 ARW) with a warm file cache: a measured frame cost about 51 ms more (mean and median), and the whole `riffle-cli features` run went from 80 s to 98 s. The cost of the second read on a cold disk is unmeasured, and the plan's "Cost" trade-off names it as the case to watch. Plan: `docs/plans/_archived/20261108-full-size-eye-check/plan.md`. Files: `crates/core/src/scan.rs` (`full_eye_sharp`).

#### TODO

- [ ] Time `riffle-cli features` (or a pass 2 in the app) on a freshly mounted card or after a reboot, with the same 8 threads and release build as Step 3, and compare it with the warm figures. If the second read is a noticeable share of the cost, consider reusing the first read's bytes.

### App: re-check that notify 8.2.2 still queues `configure` behind `Unwatch` when the native app starts using `watch`

#### Background

`gpui-backend-crate` Step 4 moved the folder watcher into `crates/backend/src/watch.rs`, with `settle` moved in from `treewatch.rs`. `crates/native/Cargo.lock` resolves `notify` to 8.2.2, while the root `Cargo.lock` stays at 8.2.0. `docs/agents/tauri-app.md` says a notify upgrade must re-check that `configure` still queues behind `Unwatch`, which `watch::settle` relies on. Nothing in CI covers this on 8.2.2. The native app now calls `watch::set` for the open folder's resync (`gpui-strip-index` Step 4, `crates/native/src/main.rs`), but `settle`'s `configure` barrier matters only for the rename path, so the re-check is still open. The resync's hand-runs (a copy and a delete in the watched folder) behaved as expected on 8.2.2 but did not exercise `Unwatch` / `configure` queuing. Plan: `docs/plans/_archived/20261109-gpui-backend-crate/plan.md`, `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/backend/src/watch.rs`, `crates/native/Cargo.lock`, `docs/agents/tauri-app.md`.

#### TODO

- [ ] When the native app first uses `watch` (the ports' rename path), re-check on notify 8.2.2 that `configure` still queues behind `Unwatch`, so that `watch::settle` still holds. Either pin the native lock to the version the root lock holds, or confirm the queuing behavior and note the version in `docs/agents/tauri-app.md`.

### Native: real-device checks of the strip, the scan progress, the resume and the resync

`gpui-strip-index` added the thumbnail strip, pass 1 of the scan, the resume at the last viewed file and the folder-watcher resync to the GPUI app (`crates/native`). CI and the unit tests cover the pure models (`strip.rs`, `focus.rs`, `viewed.rs`, `resync.rs`); stderr-instrumented debug runs covered the scan, the resume and the resync. Nobody has watched the running window: the strip's drawing, the status line's `scanning done/total` climb, the memory behavior while scrolling, the resume and steered scan on a 1000+ file folder, and the resync from Explorer copies and deletes are all unchecked. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/native/src/main.rs`, `crates/native/src/strip.rs`, `crates/native/src/thumbs.rs`, `crates/native/src/focus.rs`, `crates/native/src/viewed.rs`, `crates/native/src/resync.rs`.

Build with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build --release` in `crates/native`, and use a fresh `--index <file under D:\Photos\tests\>` for each run so pass 1 runs.

#### TODO

- [ ] On Windows, open a folder of a few hundred RAW files under `D:\Photos\` with `riffle-native.exe <folder> --index <fresh file>`. Expect `scanning N/total` after the file info, N rising about every 100 ms, replaced at the end by nothing (or `N failed`).
- [ ] On Windows, open a folder of 1000+ RAW files under `D:\Photos\` with a fresh index. Expect, in order: (1) while `scanning N/total` climbs, the gray placeholders turn into thumbnails from the current cell outward, the middle of the strip first, each cell with its file name and the current cell outlined; (2) holding `→` for a few seconds moves the outlined cell and the preview together, the strip scrolls just enough to keep the cell in view, and on release the preview, the outline and the status line name the same file, with `Home` / `End` jumping both to the first / last file; (3) wheeling the strip (toward you scrolls right) to a far cell and clicking it shows that file and moves the outline, while a click in the 8 px gap between cells does nothing; (4) with Task Manager's Details tab showing the `riffle-native.exe` row's private working set (or Commit size), wheeling the strip from one end to the other and back several times raises the value while scrolling and lets it settle to about the same level, not climbing run after run; (5) on a folder holding a corrupt RAW (e.g. `D:\Photos\tests\20261110-gpui-strip-index\ARW`, file `DSC-RX100M3_corrupt.ARW`) its cell shows `invalid TIFF byte order` in the gray box instead of a thumbnail.
- [ ] On Windows, on a 1000+ RAW folder with a fresh index (`manual3.sqlite`), wait for the scan to end, press `End`, then `←` a few times, wait a second and quit with `q`. (a) Run again with the same arguments: the preview, the outlined strip cell and the status line open at the file left at, and the strip is scrolled to it. (b) Run on the same folder with another fresh index (`manual3b.sqlite`) and press `End` right after the window opens: the thumbnails near the last cell appear within a second or so, before the middle of the folder is scanned (`scanning N/total` still climbing), and wheeling the strip to the middle and stopping fills the cells there next.
- [ ] On Windows, copy a few ARW files into a new folder under `D:\Photos\tests\` (e.g. `D:\Photos\tests\20261110-gpui-strip-index\manual4`) and run `riffle-native.exe <that folder> --index <fresh file>`. Page to the second file. (1) From Explorer, copy another ARW into the folder: within about a second its cell appears in the strip at its name's place and gets a thumbnail, the outlined cell, the preview and the status line still name the same file, the status line's total grows by one, and the other thumbnails do not flash back to gray. (2) From Explorer, delete a file that is not the current one: its cell disappears and the current file stays current. (3) Delete the current file: the preview, the outline and the status line move to the file after it (the one before it when it was the last).

### Native: measure the scan pause in `load` with presses on files pass 1 has not read yet

`gpui-strip-index` Step 5's during-scan run stepped to the next file, which the hot list had already made the scan read (a cache hit of about 1 ms), so `ReadSlots::pause` around `read_preview` showed no effect at p50 / p95. The one 336.75 ms read without the pause came on a disk slowed by 40 GB of just-written copies, so it proves nothing. Tauri's figure (14-16 ms read wait at cap 2 with the pause) is for a read of an unscanned file. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md` (`results.md` in the same folder). Files: `crates/native/src/main.rs` (`load`), `crates/native/src/focus.rs`, `crates/native/src/args.rs` (`--auto`).

#### TODO

- [ ] Run `--auto` with presses that land on unread files (a jump such as `End`, or a stride), on equal cold copies (`robocopy /J`, one per run) and a settled disk, pause on against off, and record `read` p50 / p95 next to Tauri's. Done when `docs/agents/native-app.md` says whether the pause shortens the read.

### Native: decide whether the strip's image box grows from 144x96 to the Tauri strip's 144x144

`gpui-strip-index` built the plan's 144x96 image box, but the Tauri strip's `img` is 144x144 (`crates/app/ui/style.css` `.cell img`, `object-fit: contain` inside a 168 px cell), so a portrait thumbnail (orientation 6 / 8) shows at 64x96 in the native strip against 96x144 in the Tauri one. Growing it makes the strip about 50 px taller. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/native/src/strip.rs` (`BOX_HEIGHT`, `CELL_HEIGHT`), `crates/native/src/main.rs` (`paint_strip`).

#### TODO

- [ ] Decide when the strip gains its marks (port Steps 6-8) whether the native cell grows to the Tauri size, and change `BOX_HEIGHT` / `CELL_HEIGHT` if so.

### Native: `present` p95 rose from about 0.2 ms to 0.55-0.85 ms on ARW with the strip

In `gpui-strip-index` Step 5's A/B (`results.md`), `present`'s p95 on the ARW compare folder rose with the strip while RAF was unchanged. It is not gated and stays under 1 ms. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/native/src/main.rs` (`paint_strip`).

#### TODO

- [ ] When a later strip change (marks, badges) lands, re-run the same-session A/B and check that `present` p95 has not grown past 1 ms; if it has, look at `paint_strip` first.

### Agents: `GIT_DIR` / `GIT_WORK_TREE` exported around `mise run ci` leak into `merge.test.sh` and write to the real repository

#### Background

`gpui-zoom-peaking` Step 2 Phase B: after the repository folder was renamed, a worktree's `.git` pointed at a stale path, and a session worked around it by exporting `GIT_DIR` / `GIT_WORK_TREE`. That leaked into `mise run ci`, where the merge-settings skill's `merge.test.sh` runs `git -C <tmp> init`, `commit` and `worktree add`; they landed in the real repository (an empty `init` commit, a branch `other`, a worktree entry `hooks-other`). The fix was `git worktree repair <worktree>` from the renamed repository. Plan: `docs/plans/_archived/20261110-gpui-zoom-peaking/plan.md`. Files: the merge-settings skill's `merge.test.sh`, `CLAUDE.md` (Development section).

#### TODO

- [ ] Either add a line to `CLAUDE.md`'s Development section ("never export `GIT_DIR` / `GIT_WORK_TREE` around `mise run ci`"), or make `merge.test.sh` unset both variables (fixing the cause). Done when the line is in `CLAUDE.md`, or the test script unsets them and a test shows it cannot touch the outer repository.

### Native: private bytes grow while `→` is held in the 1:1 view

#### Background

In `gpui-zoom-peaking`'s Step 2 smoke run, two 4 s zoomed holds of `→` took private bytes from 418 to 511 to 557 MB without peaking (549 -> 593 MB with it), while the fit-view holds settled near 400-425 MB. Each zoomed page reads the full JPEG (`reader::read_full`) and decodes a crop. Whether this is allocator retention that plateaus, or a crop or texture that is not released, is unknown. Plan: `docs/plans/_archived/20261110-gpui-zoom-peaking/plan.md`. Files: `crates/native/src/zoom.rs`, `crates/native/src/main.rs` (`zoom_page`, `crop_arrived`).

#### TODO

- [ ] Hold `→` zoomed for longer runs (Task Manager, Details, Commit size) and check whether private bytes plateau. If they do not, find the crop or `RenderImage` that is not released (check `retired`).

### Native: a held `→` on RAF previews barely draws

#### Background

In `gpui-zoom-peaking`'s Step 2 measurement, 5 s of key-downs every 33 ms showed only 4-6 RAF pages (`--size 4416x2944`), peaking off as well as on, against about 105 on ARW. A RAF preview load (40-45 ms) outlasts the key repeat, so when it lands the pager has moved and `Pager::finish` re-reads the newest file; a page shows only when a gap between key-downs is longer than a load. The Tauri app may behave the same (not compared). Showing the landed preview while the next one loads would let a held key draw. Plan: `docs/plans/_archived/20261110-gpui-zoom-peaking/plan.md` (see its `results.md`, "RAF pages under a held key"). Files: `crates/native/src/pager.rs`, `crates/native/src/main.rs` (`loaded`).

#### TODO

- [ ] Compare a held `→` on a RAF folder in the Tauri app, then decide whether the native pager should show a landed preview while the next one loads, so a held key draws.

### Native: real-device checks of the meta pane, the judgment keys, the sidecar writes and undo / redo

`gpui-judgments` added the meta pane, the view-only JPEG folder, the strip's judgment marks, the judgment keys through the keymap, the index and sidecar writes and undo / redo to the GPUI app (`crates/native`). Each step's checkbox was ticked on its automated criteria (unit tests, `--auto` runs); nobody has watched the running window or pressed the keys by hand, and the window close button's writer drain has not been seen. Plan: `docs/plans/_archived/20261110-gpui-judgments/plan.md` (the pending manual checks in `learnings.md` in the same folder). Files: `crates/native/src/main.rs`, `crates/native/src/meta.rs`, `crates/native/src/viewonly.rs`, `crates/native/src/strip.rs`, `crates/native/src/keys.rs`, `crates/native/src/judgment.rs`, `crates/native/src/history.rs`, `crates/native/src/settings.rs`.

Windows only. Build with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build` in `crates/native` and run `target\debug\riffle-native.exe <folder> --index D:\Photos\tests\20261110-gpui-judgments\index\<new name>.sqlite [--settings <file>]`, a fresh index file name for every run. Work only on the copies under `D:\Photos\tests\20261110-gpui-judgments\` (or the read-only test folders named below), and never open a folder under `D:\Photos\` outside `D:\Photos\tests\`. The settings files are in `D:\Photos\tests\20261110-gpui-judgments\settings\` (`xmp.json`, `both.json`, `rot.json` = `{"sidecarFormat":"xmp","labelNames":{"red":"Rot"}}`, `default.json` = `{}`). Check each sidecar with `cat`.

#### TODO

- [ ] Meta pane (Step 2). (1) Open `D:\Photos\tests\20261108-gpui-compare\ARW` with `--index ...\index\manual-arw.sqlite`: a 220 px pane right of the preview shows the file name, `n / 20`, an `EXIF` group (Aperture, Shutter, ISO, Focal length, Exposure, Camera, Lens, Captured, each present in the file) and a `Maker note` group for a Sony file; the arrow keys update it with the preview; no notes line. (2) Open `D:\Photos\tests\2026-09-28-gui-check-jpeg` with `--index ...\index\manual-jpeg.sqlite`: `EXIF` rows only, no `Maker note` group, and `JPEG folder: view only` at the bottom of the pane. (3) Optionally, on a copy holding a corrupt file, the pane shows the file name and the error alone.
- [ ] Strip marks and key names (Step 3). With `RUST_LOG=debug` set, open `D:\Photos\tests\20261110-gpui-judgments\read\lr-check-4-lightroom-desktop` with `--settings ...\settings\xmp.json`. Once pass 1 ends: L1005036 three gold stars at the image box's top-right, a green dot at its top-left and a yellow name band; L1005037-40 four to one stars; L1005041 a green dot; L1005043 a red dot and the cell dimmed; L1005044 a red name band. The star glyph renders (font fallback) and reads over a bright thumbnail. Repeat with `both.json` and another fresh index: the `.dop` is newer for most files, so the marks follow `reconcile_of`'s newest-wins pick. In the same run press Shift+X, Ctrl+Alt+1, Ctrl+Alt+0, Ctrl+Z, Ctrl+Shift+Z, `1`, `x`, Shift+1 and the arrows, and read the `[riffle_native][DEBUG] key: <name> action=...` lines on stderr: `shift+x` -> `rejectRest`, `ctrl+alt+1` -> `red`, `ctrl+alt+0` -> `clearlabel`, `ctrl+z` -> `undo`, `ctrl+shift+z` -> `redo`, `1` -> `rate1`, `x` -> `reject`, Shift+1 -> `!` with no action, the arrows page; note whether Ctrl+Alt+digit reaches the window at all (on a layout with AltGr it is AltGr+digit). Nothing is written: afterwards `ls -l --time-style=full-iso *.xmp *.dop` in the folder still matches `D:\Photos\tests\20261110-gpui-judgments\read\sidecars-before.txt`. Record the key names that arrived in `docs/agents/native-app.md`'s "Every keystroke goes through the keymap" section.
- [ ] Sidecar writes (Step 4), on the copies in `D:\Photos\tests\20261110-gpui-judgments\write\` (four ARWs each, no sidecars; three JPEGs in `jpeg`). (1) `write\xmp`, `xmp.json`: on a file press `3`, go back (auto-advance moves on), `p`, back, `Ctrl+Alt+1`: within a second `<stem>.xmp` holds `xmp:Rating="3"`, `xmpDM:good="True"`, `photoshop:LabelColor="red"`, `xmp:Label="Red"`; then `0`, `u`, `Ctrl+Alt+0`: `Rating="0"`, no `good`, no label; on another file mash `1` `2` `3` within 300 ms: one mtime change, `Rating="3"`. (2) `write\both`, `both.json`: the same keys give `<stem>.xmp` and `<name>.dop` holding `Rating = 3`, `ShouldProcess = 0` (PhotoLab's pick) and `ColorLabel = "Red"`, random Uuids unless PhotoLab has the copy registered. (3) `write\rot`, `rot.json`: `Ctrl+Alt+1` writes `xmp:Label="Rot"`. (4) `write\crash`, `xmp.json`: press `4` and `taskkill /F /IM riffle-native.exe` within 300 ms (inside the debounce); reopen with the same `--index`: stderr shows `scan sidecars: ... dirty=1` and the sidecar gets `Rating="4"`. (5) `write\drain`, `xmp.json`: press `5` and `q` at once: stderr ends with `quit: sidecar writer drained=true` and the sidecar holds `5`; repeat closing the window with its close button instead of `q`: the same line, the same `5`. (6) `write\advance`, `default.json`: `3` moves to the next file; on the last file `3` stays; `Ctrl+Alt+2` stays on the file; the arrows still wrap. (7) `write\jpeg`, `xmp.json`: `3` does nothing (no stars, the view stays) and no file appears next to the JPEGs. No failure line in the pane's notes throughout. Record what was seen, the window close button's drain above all, in `docs/agents/native-app.md`'s "The writer drains in `App::on_app_quit`" section.
- [ ] Undo / redo (Step 5), on `D:\Photos\tests\20261110-gpui-judgments\undo\` (`DSC-RX0M2_sony_rx0_ii_40.arw`, `DSC-RX0_DSC00009.ARW`, `DSC-RX0_sony_rx0_07.arw`, `DSC-RX100M2_DSC00416.ARW`, no sidecars) with `--settings ...\settings\xmp.json`; wait for pass 1 to end and press `Home`. (1) `3` on file 1: three stars on its cell, the view moves to file 2; `p` on file 2: a green dot, the view moves to file 3; after a second file 1's `.xmp` holds `xmp:Rating="3"` and file 2's `xmpDM:good="True"`. (2) `Ctrl+Z`: file 2's dot goes, the view is on file 2, the notes say `Undid DSC-RX0_DSC00009.ARW`, its `.xmp` loses `good` within a second. (3) `Ctrl+Z` again: the view is back on file 1, its stars go, the note says `Undid DSC-RX0M2_sony_rx0_ii_40.arw`, its `.xmp` holds `Rating="0"`. (4) `Ctrl+Shift+Z`: file 1's stars come back (`Rating="3"`), the note says `Redid ...`. (5) A new key (`1` on any file), then `Ctrl+Shift+Z`: nothing changes, no note. (6) Quit with `q`, reopen with a new `--index`: `Ctrl+Z` does nothing; open `D:\Photos\tests\20261110-gpui-judgments\write\jpeg` the same way: `Ctrl+Z` does nothing there either. Record what was seen in `docs/agents/native-app.md`'s "A judgment goes view, row, writer" section.

### Core: `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` hits its 1 s bound under parallel load

During `gpui-judgments`' local `mise run ci`, this test took 2.91 s against its 1 s wall-clock bound under parallel load; run alone it took 0.38 s. The bound measures the machine's load as much as the cancel. Plan: `docs/plans/_archived/20261110-gpui-judgments/plan.md`. Files: `crates/core/src/scan.rs` (the test, around line 1441).

#### TODO

- [ ] Loosen the bound, or wait on a condition (the scan's end) instead of the wall clock, and confirm `mise run ci` passes with full parallelism.

### Native: real-device checks of the f focus mark, face boxes and face mesh

`gpui-focus-marks` added pass 2, the `f` focus mark, the face boxes with landmark dots and labels, and the on-demand eye judgment with the face mesh to the GPUI app (`crates/native`). CI and the unit tests cover the pure ports (`mark.rs`, `faces.rs`, `eyes.rs`, `facemesh.rs`), and logged coordinates from debug runs matched hand computations. Nobody has put the native window beside the Tauri app, because no screenshots were possible. The status line's `analyzing done/total` text and a folder switch mid-pass 2 from the UI were not checked either. Plan: `docs/plans/_archived/20261110-gpui-focus-marks/plan.md`. Files: `crates/native/src/mark.rs`, `crates/native/src/overlay.rs`, `crates/native/src/faces.rs`, `crates/native/src/eyes.rs`, `crates/native/src/facemesh.rs`, `crates/native/src/main.rs`.

Test files are under `D:\Photos\tests\20261110-gpui-pass2\marks\`. Run `mise run native:dev -- <folder> --index <fresh file in that test folder>` for the native app, and open the same folder in the Tauri app with its own index (never share one).

#### TODO

- [ ] On Windows, press `f` in both apps and step through `ILCE-1M2_sony_a1_ii_03.arw`, `ILCE-7M4_DSC07652.ARW`, `ILCE-6300_Free_raws_by_signatureeditsco_DSC03068.arw`, `ILCE-6400_DSC01340.ARW`, `ILCE-6400_DSC00405.ARW`, `_DSC1616.ARW`, `_DSC1758.ARW` and `_DSC1760.ARW`. Expect on each file the crosshair and the AF frame on the same spot of the picture with the same color (bright green on `_DSC1758`, gray on `_DSC1616`, white on the rest), no mark on `ILCE-6400_DSC00405.ARW`, and the arm ends keeping the dark outline.
- [ ] On Windows, with `f` on in both apps, check at least `_DSC1758.ARW` (nine faces), `ILCE-6400_DSC01340.ARW` (orientation 6), `ILCE-1M2_sony_a1_ii_03.arw` (orientation 8) and `ILCE-6400_DSC00405.ARW` (a face box without an AF mark). Expect the same boxes, five cyan dots per face and label text in both apps, each label upright above its box's top-left corner (inside the box at the image's top edge, pulled left at the right edge), and the labels readable over a bright face with the native app's offset-copy halo (`docs/agents/native-app.md`). If they do not read, switch `paint_label` to a dark quad behind the text.
- [ ] On Windows, with `f` on in both apps, stop on `ILCE-6300_Free_raws_by_signatureeditsco_DSC03068.arw` (closed eyes, orientation 8), `_DSC1758.ARW` (open eyes) and `ILCE-6400_DSC01340.ARW` (portrait, orientation 6), waiting for the judgment (up to 2 s in a debug build). Expect the magenta mesh (face oval, eyes, brows, nose, lips) on the same face with the same lines in both apps, over a dark outline, and the iris rings and two iris dots on the open-eyed frames only. Also hold `→` with `f` on and check that the log's `eyes` lines name only files the paging stopped on or passed slowly.
- [ ] On Windows, on a folder of a few hundred RAW files with a fresh `--index`, check that the status line shows `analyzing N/total` after `scanning N/total` and ends clean, and that switching to another folder mid-pass 2 from the folder tree stops it (reopening the first folder resumes the remaining files).
