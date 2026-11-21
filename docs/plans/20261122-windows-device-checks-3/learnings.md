# Learnings

## Run results (2026-11-22, Windows 11)

What the third real-device run saw, moved here from the `results.md` the run
wrote. It is the source of truth for the `todo.md` and `docs/agents/app.md`
edits of Step 1.

Setup: Windows 11, debug build of crates/app with `--features screenshot` at origin/main f05d417c, window 1616x1008. Material: hardlinked ARW copies under `D:\Photos\tests\20261122-tree\` (tree/a, tree/b with b/20a/x, b/20b, b/2zz, b/c/d, ren/x1 + ren/taken, select (11), menu (6), menu-jpeg (3 JPEG), trash (8), undo/a, undo/b, slow/slow2/slow3/slow4 (264 each), a deep long-named chain under tree/a). Fresh `--index` / `--settings` / `--log` per run (run-1..run-3), MCP on a test port, synthetic mouse/keys (SendInput/keybd_event/mouse_event), window captures.

### todo "real-device checks of the folder tree, the strip's selection and right-click menu, and Move to Trash undo / redo" (gpui-tree-trash)

Step 2 (tree pane, launch with no folder) — PASS except one deviation:
- No-folder launch: no OS dialog; pane lists home (`daisu`), `C:\`, `D:\`; preview/strip empty; status `no folder`; Right, 3, f, z, e, Home, End, Ctrl+Z, Ctrl+Shift+Z, a strip click and a wheel did nothing, no panic in the log, process alive. q quits.
- DEVIATION: the `Sort ▾` and `Filter ▾` menus DO open with no folder (the Sort menu shows File name checked; the Filter menu shows all its sections). The check expected them to do nothing.
- Launch on tree\a: chain expanded, `a` highlighted, scrolled into view (as the last visible row), count 5.
- Clicking `b` opened it (strip/preview/status switched); paging then clicking `a` resumed at its last viewed file (_DSC1494). (Scan stop on switch not observable: the 5-file scans finished in ~1.5 s.)
- ctrl+alt+left hid the left pane and handed the keys back (cursor ring gone); again showed it.

Step 3 (tree follows disk, tree menu) — PASS:
- mkdir tree\c appeared within ~1.2 s; rename to zd followed; rmdir removed the row; pane notes empty.
- Collapsed `tree` (Space), created `e`, expanded (Space): `e` listed (log not checked for no re-list; needs RUST_LOG=debug).
- Folder menu: Reveal in File Explorer, Copy Path, Copy Folder Name, Refresh, Rename…, Rename by Capture Date…, Expand All, Collapse All, Rewrite Sidecars from Index…, Delete Sidecars…, Sequence JPEG Timestamps…, Import…. On a root (`D:\`): no Rename…/Rename by Capture Date…/Expand All/Collapse All.
- Escape and a press on the preview close the menu (no paging).
- Reveal on `a` opened Explorer on `tree` with `a` selected; on `D:\` it opened the drive. Copy Path pasted `D:\Photos\tests\20261122-tree\tree\a` (no `\\?\`), Copy Folder Name `a`.
- Refresh on the open folder re-lists its files (log `scan list` for b at the click). Created b\f while b collapsed; after Refresh and expand, f is listed.
- The tree menu near the window's bottom edge opens above the pointer.
- Not checked: Alt+Tab flicker, USB drive.

Step 4 (tree keyboard) — PASS:
- After a click in the pane the ring shows; Up/Down/End move it and scroll into view (End → last root row `tmp`).
- Space on collapsed `tree` collapsed/expanded it (subtree expansion kept).
- Type-ahead: `20` jumps to the next row starting with `20` after the cursor (wrapping); tapping `2` steps through the `2…` rows one by one.
- With the tree focused: 3, p, Shift+Left, Alt+Left, Ctrl+Z changed nothing; q did not quit.
- Enter on `b` opened it and the tree kept the keyboard (Down moved the ring); Escape removed the ring.
- Expand All on `tree` expanded everything under it (a\2026…\2026…\2026…, b\20a\x, b\c\d, e) one listing at a time; Collapse All collapsed everything under `tree` with `tree` staying expanded.
- Observation: a click on a row does not move the keyboard cursor to that row; the cursor follows an open (`tree_cursor = tree_open`) and the arrow keys only. A click on the open row's count/icon toggles it and leaves the cursor where it was.

Step 5 (strip multi-selection) — PASS (all points):
- click 1492, Shift-click 1496: five selected, focus on 1496 (outlined white, others filled gray); Ctrl-click 1494 dropped it; Ctrl-click current: nothing.
- `3`: four rated, no advance (auto-advance on), each .xmp `xmp:Rating="3"`; Ctrl+Z `Undid 4 files` (ratings 0); Ctrl+Shift+Z `Redid 4 files`.
- Shift+Right twice → 3 selected; Shift+Left → 2; Left → only the landed cell.
- Ctrl+A selected 11; `p` picked all 11 as one undo entry (`Undid 11 files`).
- Filter unrated (7 shown), select three, `2`: they left the strip (4 shown), selection = new current alone.
- JPEG-only folder: Shift-click selected the range; 3, p, 6 changed nothing.

Step 6 (strip right-click menu) — PASS (all points):
- Three selected, right-click a non-current one: menu at the pointer, above it on the bottom strip; near the right edge it opens to the left; selection stays, focus moves to the clicked cell.
- Items: Select All (Ctrl+A), Reveal in File Explorer, Copy Path / Copy File Name, Pick/Unflag/Reject with keys, star rows 5..0, label tags with Ctrl+Alt keys, focused file's state checked, `Move 3 Files to Trash…`, `Delete 3 Files Permanently…`.
- Copy Path: three paths one per line in strip order with `\\?\`; Copy File Name: three names.
- 2 stars: all three rated, one undo entry (`Undid 3 files`), shows checked on reopening.
- Reveal opened Explorer on `menu` with the focused file selected.
- Right-click outside the selection: it alone selected. Right-click the empty area right of the last cell: nothing.
- menu-jpeg: only Select All, Reveal, Copy Path, Copy File Name, Move to Trash…, Delete Permanently…

Step 7 (Move to Trash… / Delete Permanently…) — PASS except two points not run:
- Move to Trash on a RAW with .xmp and .dop (format `both`): dialog `Move to Trash` / `Move 1 file to the Trash?`, run button outlined, Tab moves the outline between `Move to Trash` and `Cancel`; with the dialog open q, 4, ctrl+, did nothing; Enter confirmed: the three files are in the Recycle Bin, the strip moved to the neighbour, pane `Moved 1 file to the Trash`, log `trash: moved=1 failed=0 run_id=Some(1)`.
- Three selected: `Move 3 files to the Trash?`; Escape and Cancel moved nothing.
- Delete Permanently…: Cancel outlined; Tab to `Delete Permanently`; Enter and Space did nothing; a click deleted (files and sidecars gone, not in the Recycle Bin), pane `Deleted 1 file`; Ctrl+Z after it brought nothing back. A double-click on the menu item opened the dialog and deleted nothing. (A double-click on the dialog's button deletes on its first click, as `confirms_delete` intends: the swallowed double-click is only the rest of the one that opened the dialog.)
- During a fresh 264-file scan (slow4) Move to Trash confirmed: log `Move to Trash held for the scan's end`, the scan ended early (`canceled=true` at 12/264), the file moved, the folder re-listed and the scan resumed (`analyzing 1/252` …). The waiting note was too short-lived to capture.
- Not run: the "open another folder before the scan ends" variant; an earlier judgment no longer undoable after delete.

Step 8 (Trash undo / redo) — PASS, one count difference:
- Rate 3, trash, Ctrl+Z: `Restored 1 file from the Trash`, RAW + .xmp + .dop back (gone from the Recycle Bin), xmp still Rating 3, current file and selection unmoved. Ctrl+Shift+Z: `Moved 1 file to the Trash` (3 items in the bin); Ctrl+Z restored again.
- Rate 2, trash, Ctrl+Z twice: file back, then `Undid _DSC1494.ARW` (Rating 0).
- Trash in undo\a, open undo\b, Ctrl+Z: `Restored 1 file from the Trash`, the file back in undo\a, b still 2 files.
- During the re-listing's scan Ctrl+Z twice quickly: `Undo Move to Trash: waiting for the scan to finish`, the scan not canceled, restored at its end; one `trash: undo of run 6` log line.
- Delete Permanently then Ctrl+Z: nothing comes back.
- Removed the trashed RAW and its two sidecars from the Recycle Bin by hand ($R files), Ctrl+Z: notes `Restored 0 files from the Trash, 3 failed`, errors `_DSC1497.ARW: could not restore from the Trash: not in the Trash (emptied or restored by hand)` and, for the .xmp and .dop, `left in the Trash: its RAW could not be restored` (they were not in the Trash either). Nothing left to redo. DIFFERENCE: the check expected `1 failed`; the count includes the sidecars, while the move's note counts RAWs (`Moved 1 file`), and the sidecars' reason is wrong when they too were emptied.

### todo "real-device checks of the tree's inline folder rename (gpui-folder-dialogs Step 7)"
- `a/b` + Enter: note `a/b: a name cannot contain '/'`, row keeps its name. `taken`: `taken already exists`. PASS.
- Type then Escape: nothing renamed. Clear the field, Enter: nothing. With the field open `a`, `1`, `p`, Space, Ctrl+Z, Left/Right edit the text only (Ctrl+Z undid the field's typing; no judgment, paging or tree move); Ctrl+A/C copied `x1`, Ctrl+X emptied, Ctrl+V restored. PASS.
- Alt+Tab away (to Explorer) and back: field still open; clicking the strip confirmed (`x1` → `x1b`, reopened). PASS.
- IME: not run.
- Fresh index, during the scan (264 files) rename the open folder to `Later`: row `◷ Later` muted italic, hover tooltip `Renames when the scan finishes`, pane `Rename…: waiting for the scan to finish`, the scan not canceled. At the scan's end the first attempt FAILED: note `Later: アクセスが拒否されました。 (os error 5)`, log `rename: D:\Photos\tests\20261122-tree\slow failed: Later: Access denied (os error 5) in 1ms`, logged 0 s after `scan faces: ... done=264 ... canceled=false in 78604ms`. The same rename run by hand right after succeeded (`renamed ... in 62ms`). A second try on another fresh 264-file folder (slow3 → Later4) succeeded at the scan's end (`in 61ms`). So the held rename intermittently fails with ERROR_ACCESS_DENIED when it runs the moment pass 2 ends (something still holding a handle in the folder).
- Later2 then re-edit back to the real name: row shows the real name, note `Rename… was canceled`. Later3 then click `other`: `other` opens, note `Rename… was canceled`, the folder keeps its name. PASS.
- Slow click: a single click on the open row's name opens the field after ~0.5 s; a double-click opens none; a click followed by Up opens none (cursor moves); two clicks ~280 ms apart open none. Two clicks ~480 ms apart DID open the field (borderline with the 500 ms system double-click time; not a clear failure). A single click on another folder opens it. PASS.
- During an import run: not run.

### todo "real-device check of Left / Right paging the strip and Space toggling from the folder tree (native-tree-left-right)"
- Right/Left page the strip with the ring staying on the tree cursor; Up/Down/Home/End move the cursor; Space expands/collapses; Space on a folder listed empty (`f`) does nothing and no type-ahead jump; Enter opens the cursor's folder; Shift+Left / Alt+Left swallowed. ALL PASS.

### todo "real-device check of the folder tree's file counts on collapsed rows" (Windows bullet)
- Expanded/listed folders show the read_dir count; collapsed rows listed earlier in this index show their cached count after a quit and restart (menu 6, menu-jpeg 3, select 11 in the same session; after restart `Later` 264, `other` 1, `slow2` 264).
- A folder renamed in the app keeps its count after a restart (`Later` from slow, `Later4` from slow3, 264 each).
- A counted folder renamed outside the app (`big` counted 44, renamed in-app to `Later`, then moved back to `big` outside) shows no count after restart until listed; `select` re-created after renaming the old one outside shows no count.
- A folder never listed in this index shows no count (menu, ren, run-*).
- Long names next to a count pill truncate with the pill whole (`2026... 6`).
- Not run: index upgraded from before the cache table, Rename by Capture Date split, network drive / symlink / junction, Refresh after adding a RAW outside.

### todo "real-device check of the folder tree rows' name truncation inside the pane"
- Long name: ends in an ellipsis inside the pane at every depth (depths 2–4 of the 2026-10-03-dop-orientation-… chain), nothing past the pane's right edge. PASS.
- Count badge: fully visible at the row's right edge next to a short and a truncated name. PASS.
- Highlights: cursor ring, hover and open-row highlight span the pane less a small inset with rounded corners. PASS.
- Scrollbar: the vertical scrollbar sits at the pane's right edge (thumb drag not tried; wheel scrolls). PARTIAL.
- Rename: the inline editor fits the row; the pending `◷ Later` row truncates like others (short name only seen). PASS for the editor.
- Scrolling into view: reveal on open, keyboard cursor (End) and the rename's row are brought into view. Inset-edge clicks not tried. PARTIAL.

### todo "real-device check of the folder tree rows drawn as ListItems (native-tree-listitem)"
Checked in Neutral dark, Neutral light and Catppuccin Mocha:
- States: the open row is lit; hover lights other rows; the keyboard cursor is outlined only after a click in the pane (ring color follows the theme). Semibold on the open row not clearly visible at this size.
- Clicks: a click on a collapsed non-open row opens and expands it; a click on an expanded non-open row (`tree`) opens it and it stays collapsed once the reveal ends; a click on the open row's icon or count toggles it without a re-list (no new `scan` lines); a click on the open row's name arms the slow-click rename; a right-click opens the menu.
- Rename: the input fits the 22 px row; Escape cancels.
- Name states: pending rename italic and muted (seen); failed folder red: not run.
- Layout: count at the right end; deep path truncates and keeps the count; uniform row heights while scrolling; folder icons with no chevron, expanded rows `folder-open`; indent reads the depth.
- Text selection: the inline rename's selected text reads gray in Neutral dark.

### Not done in this batch (left open)
IME rename, rename during an import, `open_folder` refused during a rename over MCP, trash "open another folder before the scan ends", failed-folder red name, Refresh after adding a RAW outside (count follows), index-upgrade / split / network-drive count checks, macOS / Linux.

### Incident

A coordinate slip in the synthetic mouse input opened
`D:\Photos\tests\2026-09-19-focus-sample-2` in the app. It was only read: no
file in it changed.

## Step 1

- Every heading the plan names matched `todo.md` at `44f4a3f0` (one commit past the plan's `f05d417c`) exactly once; no item had to be found by its text.
- As in the earlier batches, a Python script written to the scratchpad with the Write tool and run with `mise x -- python -I` located each item by its exact `###` line (one match asserted per anchor), replaced the named sentences with one-match asserts and dropped or replaced boxes by their index among the item's top-level `- [ ] ` lines, a box's indented points going with it. A check for a `###` line without a blank line before it and for doubled blank lines found only `todo.md`'s line 4, which `main` already had.
- Sub-points the results do not mention, carried into the reworded boxes under the plan's rule beyond the plan's known cases:
  - gpui-tree-trash Step 2: the meta pane switching to `b` (the results name the strip, preview and status only). Step 2's "the expander of `b`" point was dropped as obsolete: native-tree-no-chevron removed the expander, and the ListItem item's click points cover the row click.
  - Step 6: a press on the preview or `escape` closing the strip's menu (the results saw them close the tree's menu, not the strip's).
  - Step 7: "the dialog closes at once" for the held run, kept with the short-lived notes.
  - Inline rename: the slow-click box's "a single click on a root opens it as before" (only another folder was clicked) stays as a box of its own; the held-rename box's "the rename plus reopen when it ends" moved into the new Access denied item's TODO ("every held rename succeeds and reopens the folder under its new name").
  - File counts: deleting a counted folder outside the app (only the outside rename was run) stays in the Windows box.
  - ListItems: a click on the open row's indent (only its icon and count were clicked) stays in the box.
- The results seen for the ListItems' "hover lights other rows" and the counts' "expand a folder never opened, collapse, restart" were read as covering those points: "other rows" excludes the open row, and the cached counts after a restart were seen on folders listed earlier in the index.
