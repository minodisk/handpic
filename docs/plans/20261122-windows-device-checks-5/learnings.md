# Learnings

## Run results (2026-11-22, Windows 11)

What the fifth real-device run saw, moved here from the `results.md` the run
wrote. It is the source of truth for the `todo.md` and `docs/agents/app.md`
edits of Step 1.

Setup: Windows 11, debug build of crates/app with `--features screenshot` (origin/main of 2026-11-22, docs-only changes since the build), window 1616x1008, a 5120x1440 virtual desktop. Material in `D:\Photos\tests\20261122-folderops\` (not the `20261113-gpui-folder-dialogs` paths the items name): `sidecars\` (12 hardlinked ARWs of `2026-08-22-focus-sample` with copies of their `.xmp` and `.dop`), `jpeg-only\` (2 sample JPEGs), `sequence\` (copies of the 10 JPEGs of `2026-09-28-gui-check-jpeg`, no repeated second), `seqbig\` (300 hardlinks of those, 30 per capture second), `no-jpeg\` (one ARW), `scan1\` / `scan2\` (100 hardlinked ARWs each, for a scan lasting ~50 s), `import-src\` (3 ARWs of 2026-08-29, 3 ARWs of 2026-09-13 with `.xmp`, `undated.jpg` re-encoded without Exif), `import-big\` (60 hardlinked ARWs, 3.16 GB), `import-dst\` (the import root), `step6\` (`one-date` 4 ARWs of 2026-08-29, `three-dates` 3 sample JPEGs of 1999 / 2008 / 2025 plus `undated.jpg`, `two-dates` 2 ARWs of 2026-08-22 and 2 of 2026-09-19, `collide` 3 ARWs of 2026-09-13). Settings seeded with `mcpEnabled` and `sidecarFormat: both` (and `importRoot` for the import), fresh `--index` / `--settings` / `--log` per run (run-3, run-5, run-6, run-7), synthetic mouse/keys, window and full-desktop captures; some dialogs were opened through the MCP tools that reach the same flows (`rename_by_capture_date`, `open_folder`).

Incidents: none to the photos. Twice a mis-positioned tree click opened the folder menu of a neighbouring test folder (`import-dst`): only a `Nothing left to copy` preview dialog opened, closed with Escape.

### Rewrite Sidecars from Index… / Delete Sidecars… and the folder menu (gpui-folder-dialogs Step 3)
- Folder menu order: Reveal, Copy Path, Copy Folder Name, Refresh, Rename…, Rename by Capture Date…, Expand All, Collapse All, Rewrite Sidecars from Index…, Delete Sidecars…, Sequence JPEG Timestamps…, Import…; on a root the rename and expand groups are gone (seen in batch 3). PASS.
- During a 100-file scan: Rewrite → pane `Rewrite Sidecars from Index: waiting for the scan to finish`, operation `held_for_scan`, scan not canceled (`canceled=false`). Delete → `Delete Sidecars: waiting for the scan to finish`. PASS. At that scan's end the held Rewrite on `scan1` (no sidecars, nothing judged) answered `No judgments indexed for scan1; open the folder first` — FINDING: the folder was open; the wording misleads.
- Opening another folder while a Delete waited dropped the note and opened no dialog (operation reported `failed`). PASS.
- Rewrite dialog: `• 12 files with a judgment`, then after clearing two judgments `• 10 files with a judgment` / `• 2 files without one (any existing sidecar is cleared)` and `Rewrite the XMP and .dop sidecars of 12 files in sidecars?`; `Rewrite` outlined, Tab moved to `Cancel`, Escape closed. With the dialog open a click on the tree / strip, `→`, `3` and `ctrl+,` did nothing. `Rewrite`: pane `Rewrote the sidecars of 12 files in sidecars`; the cleared files' `.xmp` read `Rating="0"`, the others kept theirs. PASS (the dimmed buttons / running note too fast to see).
- Delete dialog: `• 12 XMP sidecars (4.4 KB)`, `• 12 .dop sidecars (5.0 KB)`, `Move 24 sidecars (9.4 KB) of sidecars to the Trash?`, red `Move to Trash`. Confirm: 24 sidecars in the Recycle Bin, pane `Moved 24 sidecars to the Trash`. Ctrl+Z: `Restored 24 sidecars from the Trash` after ~3 s, all 24 back; Ctrl+Shift+Z moved them again, Ctrl+Z restored again. PASS. (A redo pressed while the ~3 s restore still ran took effect after it.)
- `jpeg-only`: `jpeg-only: rewriting sidecars applies to RAW files only` / `… deleting sidecars applies …`, no dialog. PASS.
- `Ctrl+,` while the preview is under way: not run.

### Rewrite Sidecars from Index reported "4 files not in the index (skipped)"
- Not reproduced: on a fully scanned 12-file folder the preview listed only `with a judgment` / `without one`; no `not in the index` row appeared.

### Sequence JPEG Timestamps… (gpui-folder-dialogs Step 4)
- Dialog: `Source:` / `Output: …\sequence-sequenced`, one row per file `name  old -> new`, unchanged rows muted, changed seconds in green (on `seqbig`: `10:00:15 -> 10:00:16` …), `10 files will be written to sequence-sequenced` / `0 of them get a new capture time` (`seqbig`: 300 / 292). `Run` outlined. PASS.
- `Run`: pane `Wrote 10 of 10 files to D:\…\sequence-sequenced`, Explorer opened on the parent with `sequence-sequenced` selected. PASS (progress too fast to see on 10 files).
- Reopened: `will be rebuilt: its JPEG files are replaced` under the output line. PASS.
- `seqbig` Run: `Run` dimmed, `Cancel` focused, the note `Cancel stops before the next file; …`, operation `running 20/300`; Escape: `canceled, 60 of 300 written`, no Explorer, log `canceled=true`. PASS.
- `no-jpeg`: no dialog, pane error `no-jpeg: no target JPEG files found in: D:\…\no-jpeg`. PASS. A second request right after the first and `ctrl+,` during the dialog: not run.
- FINDING: the dialog's file list leaves a blank band above its first row (about one row on 10 files; about 360 px on 300 files, the scrollbar thumb mid-track while the list shows its first rows). The same list in the Import and Rename by Capture Date dialogs shows a blank row above and clips its last row (the third date row and the `undated.jpg` row needed a scroll; a row was cut mid-line).

### Import… (gpui-folder-dialogs Step 5)
- Dialog on `import-src`: `Source: \\?\D:\…\import-src` (the known prefix item), `9 files (273.1 MB) to copy into 2 folders`, rows `…\import-dst\2026\2026-08-29  3 files, 140.3 MB` and `…\2026-09-13  6 files, 132.8 MB`, `Without a capture date, not copied:` with `undated.jpg` (after scrolling the list). `Run` outlined. PASS (the `reading …` note too fast to see).
- `Run`: the first date folder (`2026-08-29`) opened in the tree and strip during the run (operation `running 1/9`), then `Imported 9 files (273.1 MB)`; the files under `import-dst\2026\…` with the `.xmp` beside their RAW. PASS. While the first date folder was still empty the status line read `no RAW files in \\?\D:\…\2026-08-29` (the batch 4 prefix item).
- Mid-run reopen / Escape / Cancel: not run (the 3.16 GB source copied in a few seconds from cache; every reopen came after the end).
- Reopened after the run: `0 files (0.0 MB) to copy into 0 folders, 60 already there`, row `0 files, 0.0 MB, 60 skipped`, `Nothing left to copy: every dated file is already there.`, `Run` dimmed, `Cancel` focused. With one copy replaced by a 1000-byte file: `Already at the destination with another size, not copied:` `_DSC0745.ARW in 2026-08-22` and `Nothing to copy: the dated files are not copied (see the lists above) or there are none.` PASS.
- No root (`{}` settings): `the destination folder is not set (Settings > Import)`, `Run` dimmed. PASS. The log held one `import: files=… bytes=… mb_per_s=…` line per run (3 runs). PASS.
- During a run the MCP `import_folder` is refused with `a folder operation is under way in Handpic; try again once it ends`.

### OS notification when an import ends while inactive (native-import-notification)
- Import of 60 files (3.16 GB) started, then Alt+Tab away: a toast at the desktop's bottom-right, header `Handpic`, title `Import finished`, body `Imported 60 files (3.16 GB)`. PASS for delivery. The toast's app icon is a generic one, not Handpic's (observation). Click activation, the `Import canceled` title, the no-toast-while-active case, taskbar grouping and the log warnings: not checked (two further attempts did not start a run because of a tree mis-click).

### Rename by Capture Date… and the folder-rename aftermath (gpui-folder-dialogs Step 6)
- During a 100-file scan: no dialog, pane `a scan is running; wait for it to finish` (through MCP `rename_by_capture_date`, the same `folder_op` path). PASS.
- `one-date` after its scan: rated one 3★, picked one, moved the fourth RAW to the Trash, paged to the third file; dialog `Rename one-date to 2026-08-29?` with `• 2026-08-29  3 files`, `Run` outlined. `Run`: the tree row reads `2026-08-29` (re-sorted among its siblings), the strip kept its 3 files and the third file current, log `datefolder: … one-date renamed to … 2026-08-29 in 7ms` then `scan reconcile: dir=…\2026-08-29 todo=0`. Ctrl+Z: `Restored 1 file from the Trash` into the renamed folder (count 4). PASS.
- `collide`: dialog `Rename collide to 2026-09-13?`; with an empty sibling `2026-09-13` created: reason `2026-09-13 already exists next to collide; nothing is merged.`, `Run` disabled. PASS.
- `three-dates`: `Split three-dates into 3 folders by capture date?`, rows `1999-12-06 1 file`, `2008-03-07 1 file`, `2025-01-14 1 file`, `Without a capture date, 1 file stays in three-dates: undated.jpg`, `three-dates is removed if it ends up empty.`; `Run`: three sibling date folders, pane `Split three-dates into 1999-12-06 (1), 2008-03-07 (1), 2025-01-14 (1); three-dates kept (1 left)`, the strip re-listed with only `undated.jpg`. PASS.
- `two-dates`: `Split two-dates into 2 folders by capture date?` (`2026-08-22 2 files`, `2026-09-19 2 files`); `Run`: `two-dates` left the tree, `2026-08-22` opened, pane `Split two-dates into 2026-08-22 (2), 2026-09-19 (2)`. The item on the already renamed `2026-08-29`: `2026-08-29 is already named its capture date.`, `Nothing to rename.`, `Run` disabled. PASS.

## Step 1

- Every heading the plan names matched `todo.md` and `docs/agents/app.md` exactly once. As in the earlier batches, a Python script in the scratchpad (run with `python -I`) found each item by its exact `###` line, inserted each Background's 2026-11-22 summary right after `The step's checkbox was ticked on the automated criteria only.` (one match asserted), and dropped or replaced boxes by their index among the item's top-level `- [ ] ` lines. The two new items went in right after their named neighbours. The checks for a `###` line with no blank line before it and for doubled blank lines give the same counts as `origin/main` (one each, already there before).
- A first try at writing the script through a Bash heredoc failed: the tool's shell reported `unexpected EOF while looking for matching '` on a heredoc full of apostrophes, quoted delimiter and all. Writing the script with the Write tool and running it from Bash worked.
- Sub-points the results do not mention, carried under the plan's rule beyond its known cases:
  - Rename by Capture Date box 3: the tree row "in place" was dropped rather than kept, as list A5 says; the row re-sorted among its siblings, which the Background records as seen.
  - Sequence box 6: the `no-jpeg` message's red was taken as seen from the results' "pane error" (errors are drawn in the pane's error color); the box's remaining sub-points are the second request and `Ctrl+,`.
- In `docs/agents/app.md` the narrowed "Not verified headless" bullet of the folder operations section also carries the Rename by Capture Date dialog's keys, dimmed buttons and `Cancel` focus, since that section's rules cover the dialog all the folder operations share; the rename section's bullet keeps only the reopen keeping the judgments, as list D says.
