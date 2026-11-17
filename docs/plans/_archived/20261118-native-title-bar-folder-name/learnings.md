# Learnings

## Step 1

- `Riffle::dir` is set only when a listing lands (`self.dir = Some(listed.dir…)`),
  and a folder rename or a `Rename by Capture Date…` that moves the open folder
  reopens it under its new path (`folderops.rs`'s deferred reopen), so reading
  the name in `render` follows all three with no state of its own.
- The name's helper is `context::folder_title`, next to `shown_path`: the last
  component, else the shown path (`C:\`, `/`), tested in `context.rs`.
- `TitleBar`'s window controls are each `TITLE_BAR_HEIGHT` (34 px) wide, so on
  Windows the right side holds about 102 px of controls plus the gear and its
  `mr_2`, and the left about 12 px of padding, the 16 px icon, the gap and
  "Handpic" at 14 px (80 px of padding on macOS). The overlay pads the name
  `TITLE_NAME_MARGIN` (160 px) a side, symmetric so it stays centered; the
  value is an estimate from those widths, not tuned against a running window.
- `truncate()` is `overflow_hidden` + `whitespace_nowrap` + `text_ellipsis`;
  `overflow_hidden` sets no scroll offset, so it inserts no hitbox in
  `gpui-pre` 0.3.12 (`should_insert_hitbox` checks `scroll_offset`, not the
  overflow). The name's `div` needs `min_w_0` to shrink below its text width
  as a flex item.
- Writing Rust raw strings with backslashes through a Bash heredoc into a
  Python script mangled them (`\20` became a control byte); the Edit tool
  wrote them correctly.

## Deferred issues (todo candidates)

- Pending manual check (Windows 11, `mise run native:run` or a debug build of
  `crates/native`), from this plan's Step 1 Done-when: open a folder and check
  that "Handpic" next to the icon is visibly smaller than before (14 px) and the
  icon is still 16 px; that the folder's name is centered on the whole window
  width at the same size as "Handpic" and disappears with no folder open;
  that narrowing the window truncates a long name with an ellipsis before it
  reaches "Handpic" or the gear (and that 160 px a side of `TITLE_NAME_MARGIN`
  in `crates/native/src/main.rs` looks right, else tune it); that a press and
  drag starting on the name moves the window, a double-click on it maximizes,
  and the minimize / maximize / close controls and the gear still take their
  clicks. If the drag over the name does not move the window, fall back to a
  third `TitleBar` child as the plan says. The step's checkbox was ticked on
  the automated criteria only.
