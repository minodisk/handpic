# Learnings

## Run results (2026-11-22, Windows 11)

What the fourth real-device run saw, moved here from the `results.md` the run
wrote. It is the source of truth for the `todo.md` and `docs/agents/app.md`
edits of Step 1.

Setup: Windows 11, debug build of crates/app with `--features screenshot` (origin/main of 2026-11-22, docs-only changes since the build), window 1616x1008. Material: copies of `D:\Photos\tests\20261113-filter-sort\` (without its `ab\`) in `D:\Photos\tests\20261122-strip\` (sort, filter, faces, menu, the settings JSONs, each also seeded with `mcpEnabled: true`), plus `empty\`, `nef\` (copies of three `D:\Photos\samples\NEF` files incl. `NIKON_D70_Nikon.nef`), `cold\` (600 hardlinks of `2026-09-19-focus-sample-2`'s 100 ARWs under six prefixes), `burst\` (hardlinks of the same 100 ARWs, real copies of their .xmp). Fresh `--index` / `--settings` / `--log` per run, MCP on a test port, synthetic mouse/keys, window captures.

### Incident

A mistyped launch helper started the app once with a non-existent folder path and `--index`/`--settings`/`--log` under a new folder `D:\Photos\tests$QA$1` (outside `D:\Photos\tests\`); no existing folder was opened; the stray folder held only that run's index, settings and log and was deleted at once.

### todo "real-device checks of the strip's sort order, the filter and the filter / sort menus" (gpui-filter-sort)
- Step 1 sort: PASS. `sort\raw` with settings-rating: HX99 (4★), RX0 (2★), RX0M2, RX100M2, RX100M3, HX95 (dimmed reject). `3` on RX100M2: reordered at once right after HX99, then auto-advance moved to its new right neighbour RX0. settings-capture: capture order (RX100M2, RX0, HX99, RX0M2, RX100M3, HX95). `sort\jpg-sequenced` with settings-name: C2000Z, C960Z, COOLPIX, A065.
- Step 2 filter: PASS. `filter\raw` untagged: 5 shown; `p` on HX99 hid it and landed on the next shown file (RX0M2) with no second move; Ctrl+Z brought it back (`Undid DSC-HX99_DSC00001.ARW`). `filter\stars3`: only RX0; `x` kept it shown (xmp Rating 3 kept, good="False"). unjudged: RX0M2, RX100M2, RX100M3 (3). none: `No files match the current filter.` on the status line, in the pane and centered in the preview, no image; the Filter button lit. Copying a RAW into the filtered folder re-listed (6) with the current file kept. Sidecar writes landed in the copies.
- Step 3 Face focus / Eyes: DIFFERENCE. settings-filter-good on `faces` ends with 2 files (ILCE-1M2_sony_a1_ii_03, _DSC1758), each with the green smiling `scan-face` icon (the `f` mark the check names is now this icon); DSC00405 is not Good any more (the good rule changed after the check was written, e.g. the full-size eye sharpness). settings-filter-closed: only DSC03068. PASS.
- Steps 3/4 menus: PASS. Filter ▾ opens upward, right edge on the button's; checking Picked then Red keeps it open with check marks, narrows to `No files match…`, lights the button; Escape closes; a press on the preview closes; Reset unchecks all and shows every file, the button unlit; Sort ▾ > Rating reorders (HX99, RX0, RX0M2, RX100M2, RX100M3, HX95), closes the menu, the current file stays current. A click on `Filter ▾` while open closes it and it stays closed. With the menu open a strip cell click closes the menu AND selects/pages to that cell (recorded). With the filter menu open, `→` paged (synthetic held keys do not auto-repeat, so one step), `e`, `f`, `z` toggled peaking, the focus overlay and 1:1. `menu\jpg-sequenced`: Sort ▾ dimmed and opens nothing, capture order, `JPEG folder: view only` note. `menu\faces` offers Output (3 with an export: DSC00405, _DSC1616, _DSC1758; `No output` 5); renaming `Output\` away hid the section with its check cleared, back brought it back; `menu\raw` has no Output section. The seed settings file untouched. Not checked: whether a click inside the popover reaches its on_click before the strip's on_mouse_down (the menu never overlaps the strip: it opens upward over the preview / meta pane).
- Observation: long file names in strip cells are clipped on both sides (centered, no ellipsis), e.g. `)SC-RX100M2_DSC00416.ARW`.

### todo "real-device checks of the EXIF filter sections and the refilter fixes"
- Sections Camera, Aperture, Shutter speed, ISO, Focal length listed in order with the folder's values (no Lens section: the compacts record none); the menu scrolls. PASS (fresh-index appearance after pass 1 not watched separately).
- Camera RX0 → 1; + RX0M2 → 2 (OR); + ISO 80 → 0 (AND); Reset → 6. PASS.
- Copying in an ILCE-6400 RAW added `SONY ILCE-6400`; checking it (1 shown) then deleting the file dropped the check (filter empty, 6 shown, item gone). PASS.
- Untagged filter, `p`: the status position went from `3 / 5 (6)` to `3 / 4 (6)` in the first frame captured (~30 ms). PASS.
- A filter hiding every file, then Reset from the menu: back on the file shown before (RX0M2). PASS.
- Seed settings file untouched. PASS.
- The pass-2 race (`shown_after`): not run.

### todo "real-device checks of the empty state in the preview area"
- No folder: the message `Drop a folder onto the window, or click here to choose one. Or press ctrl+o.` centered in muted text; a click opened the OS folder picker; choosing `empty\` there opened it. PASS. Rebinding / unbinding `open` and the settings-modal click: not run.
- A folder without RAW/JPEG: `This folder has no RAW or JPEG files.`; a click on it opened nothing. PASS.
- Paging from a NEF onto `NIKON_D70_Nikon.nef`: the previous image left and `The preview of this file could not be shown.` showed; paging back showed the previous image; `z` on the D70 hid the message. PASS (the message re-showing on a second visit was not captured separately).
- Preview fit and the 1:1 view: unchanged in use. PASS.

### todo "real-device checks of the status row's StatusBar and progress circle"
- The row is a bar with a top border and muted small text; Sort ▾ / Filter ▾ (also the tall Filter menu) open upward with the right edge on the button's, unclipped, dismissed by a press outside. PASS.
- Cold 600-file folder (fresh index): pass 1 took ~1.1 s (not visible); during pass 2 (`analyzing n/600`, 161 s) the circle filled from empty right of the position; it disappeared at the end. DIFFERENCE: when the circle disappears the position text (`1 / 600`) shifts right by ~20 px (the circle's slot goes). The 180 ms drop and paging smoothness not judged.
- Move to Trash of 600 selected files with the right pane hidden: the circle showed the run's progress (`3 / 600 · 600 selected` with the circle, `done 300 / 600` mid-run) and went at the end; `Moved 600 files to the Trash`. The other pane layouts not run.
- FINDING: after the folder emptied, the status line read `no RAW files in \\?\D:\Photos\tests\20261122-strip\cold` — the Windows verbatim prefix is shown to the user (also seen in batch 3 on `ren\taken`).
- Long status text / error readability: not run.

### todo "real-device checks of the burst bands, badges, burst keys and reject the rest" (macOS boxes; run on Windows)
- On `burst\` (a Sony folder): bursts show a faint band behind their cells, `n/size` badges at the image box's bottom-right, dimmed on rejected members. `↓` / `↑` step between groups (a singleton is its own group); `alt+→` / `alt+←` step within a burst (1990 → 1993 → 1995, stop at the end) and do nothing on a singleton. PASS on Windows.
- Reject the rest: burst 1990 (pick) / 1993 (reject) / 1995 (unflagged, hidden by a Picked filter); `shift+x` on 1990 rejected 1995 (its .xmp `xmpDM:good="False"`), 1990 stayed current, nothing advanced; Ctrl+Z `Undid _DSC1995.ARW (hidden by the filter)`, Ctrl+Shift+Z `Redid _DSC1995.ARW (hidden by the filter)`; `shift+x` on a singleton did nothing. MCP `get_view.burst` lists members with sharpness, rating, flag, label and `visible: false` on the hidden ones. PASS on Windows. Two-frame burst undo move and the JPEG folder: not run.

### todo "real-device checks of the filter menu's icons and headings (gpui-filter-menu-icons)"
- Headings over every section (Flag, Rating, Color label, Orientation, Face focus, Eyes, Output, EXIF groups). Flag dots; star rows 5..0 lit and dim; orientations' two rectangles; Face focus smiling green / frowning gray `scan-face` with `Neutral` aligned to the text; each section's rows start at the same x; clicks toggle; the menu scrolls. PASS. DIFFERENCE: the color labels are now gpui-component `Tag`s, not "seven filled squares and one outline square" (a later change). HiDPI: not available.

### todo "real-device checks of the strip's face focus icon and cell tooltips"
- Icons: the frowning gray `scan-face` on low frames and the green smiling one on good frames at the image box's bottom-left (seen in `cold\`/`burst\` and `faces\`). Appearing during pass 2 with no input, the rejected-good dimming, 150 % scale: not checked.
- Tooltips: a truncated RAW copy (`_A0000.ARW`) shows `preview out of range` in a themed tooltip on hover; a Red-labeled cell's name band shows `Red`; a plain cell shows nothing. PASS. Region swaps, scroll under a still pointer, edge-cut cells, clicks through tooltips, the pending-rename tooltip (seen in batch 3): not run here.

### todo "real-device check of the status row with no current-file info"
- Paging with the scan finished and zoom off: the status row's left side empty; `z` showed `1:1` at the row's left edge without leading spaces; the preview's height did not change. PASS. The drop text (DROP_TEXT): not run.

## Step 1

- Every heading the plan names matched `todo.md` exactly once; no item had to be found by its text.
- As in the earlier batches, a Python script written to the scratchpad and run with `mise x -- python -I` located each item by its exact `###` line (one match asserted per anchor), inserted each Background's 2026-11-22 sentence after a named sentence with a one-match assert, and dropped or replaced boxes by their index among the item's top-level `- [ ] ` lines. The check for a `###` line without a blank line before it and for doubled blank lines found only what `main` already had (`todo.md`'s line 4, and in `docs/agents/app.md` a doubled blank line in the progress circle section and no blank line before "A folder dropped onto the window opens").
- The pending rename tooltip box of the face icon item was deleted after confirming in `../_archived/20261122-windows-device-checks-3/learnings.md` that `Renames when the scan finishes` was seen on hover.
- Sub-points the results do not mention, carried under the plan's rule beyond its known cases:
  - Sort / filter box 4: the current file staying *in view* after `Sort ▾` (the results say it stayed current), kept with "stays current and in view".
  - Face icon box 3: the tooltip staying inside the window (not in the results), kept.
  - Status row box 3: the preview canvas not resizing, widened to "in every pane layout", since no layout's canvas size was recorded.
- The results' clipped name example `)SC-RX100M2_DSC00416.ARW` is quoted as drawn in the new ellipsis item; the full file name was not recorded.
- C1 says the batch before saw the prefix on `ren\taken` on the run's own word: the batch 3 `learnings.md` does not record that status text.
