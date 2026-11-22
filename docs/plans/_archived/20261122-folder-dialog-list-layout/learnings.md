# Learnings: folder-dialog-list-layout

## Step 1

- The inferred cause held. Before the fix the new tests in `folderops.rs`
  failed in the test window (1600x1000 by default): with 10 rows and a
  headed list of three, the list started 134 px below the last line (21 px
  tall, at y 170, the list at y 325); with 300 rows, 391 px below. After
  `flex_none()` on the lines block both pass (gap within the surface's 8 px,
  the short list's bottom above the footer, the long list scrolled to the end
  by simulated `ScrollEvent`s ending at the body's bottom).
- `debug_bounds` takes a `&'static str`, so the selectors are fixed strings
  (`folder-dialog-line` on the last line, `folder-dialog-list` on the rows'
  container, `folder-dialog-footer`); the component already names the surface
  `dialog-{layer_ix}`, `dialog-0` here. The list container's bounds move with
  the scroll offset, so its top is the first row and its bottom the last
  row's.
- A dialog with lines and no list has only the lines block in the column, so
  `flex_none` changes nothing visible; one with a list and no lines never
  builds the content block.
- Real-device check, Windows 11, 2026-11-22: a `screenshot`-feature debug
  build with MCP on port 47917 (seeded through `--import-settings` with
  `mcpEnabled` and an `importRoot`), its own `--index`, `--settings` and
  `--log`, all under the new `D:\Photos\tests\20261122-folder-dialog-list-layout\`.
  Material: `dated-10` and `dated-300`, copies of
  `D:\Photos\tests\20261108-good-eye-blur\labeled_picks_under_cut.jpg` with
  one `DateTimeOriginal` per day from 2025-01-01 set by exiftool. Each dialog
  was opened through its MCP tool with `mcpAllowDestructive` off (the app's
  own dialog), captured with `screenshot`, and cancelled; no run happened
  (no `-sequenced` folder, `import-root` empty, the dated folders kept).
  - `Sequence JPEG Timestamps…`: 10 files, the first row right under the
    `Output:` line, all ten rows and the two notes whole, no scrollbar. 300
    files, the first row right under the lines, the scrollbar track starting
    there with its thumb at the top; after 200 real wheel notches over the
    list (`SendInput`-style `mouse_event`), the last row (`20251027_120002.jpg`)
    whole above the notes and the thumb at the track's bottom.
  - `Import…`: 10 date folders, the first right under `10 files (1.8 MB) to
    copy into 10 folders`, the last whole. 300, the same at the top with the
    thumb at the top; scrolled to the end, `…\2025\2025-10-27` whole.
  - `Rename by Capture Date…`: 10 dates under `Split dated-10 into 10
    folders by capture date?`, the last whole above the note. 300, the same
    at the top; scrolled to the end, `2025-10-27  1 file` whole.
- `todo.md`: the item was deleted. Its pointer phrase "the item on the folder
  dialogs' file list below" stood in both the Import and the Rename by
  Capture Date real-device items, so both were reworded along with the
  Sequence item's "its own item below".
