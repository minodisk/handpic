# Windows real-device checks, todo sweep c6 (2026-11-27)

Setup: Windows 11 Home 10.0.26200, one 5120 x 1440 display at scale 100 %.
A debug build of `crates/app` with `--features screenshot` (origin/main
`f5cab4bf` plus the light-theme fix of `light-theme-dim-and-menu-green`,
which only changes the reject dim and the menu's `Good` green on light
themes). Each instance ran with its own `--index`, `--settings`, `--log`
under the session scratchpad, `--mcp-port 47816` and `{"mcpEnabled": true}`
through `--import-settings`; `mcpAllowDestructive` off. Input was synthetic
(`keybd_event`, `mouse_event`, after checking the window was in front by
PID), reads through MCP `get_view` / `get_status` / `get_settings` and the
`screenshot` tool, pixels read from the PNGs, the desktop corner grabbed with
`BitBlt` for the toasts, `GetCursorInfo` for the cursor shape. Material: 14
`ILCE-1M2` ARWs and 4 JPEGs copied from the local sample set into the
scratchpad, and a scratch `V:\0c6-tree-test\` (`one`, `two`, a long `l…`
name) for the tree, removed afterwards. Other agent sessions shared the
desktop, so a few inputs found another window in front and were retried.

## Look of the switch and the `Kbd` keys (native-component-controls)

- `Kbd` chips on the Keyboard Shortcuts page (`Left`, `Alt+Left`,
  `Shift+Right`, `Ctrl+A`, `P`, `5`, `Ctrl+Alt+1`) read clearly, light text
  on the muted chip. The right-click menu's shortcuts (`Ctrl+A`, `P`, `U`,
  `X`, `5` to `0`, `Ctrl+Alt+1` to `Ctrl+Alt+0`) are right-aligned in the
  muted color. PASS.
- The auto-advance switch: on, a light track with a near-black thumb, reads
  well; off, a `#0a0a0a` thumb on a `#404040` track on the `#171717` page:
  the thumb is darker than the page and about 1.9:1 against its track, a
  hole rather than a thumb. Judged hard to read, so the mapping the plan
  deferred is due; handed to its own session (`dlg-c6-switch`, decision: the
  off state follows shadcn/ui Neutral dark, a light thumb on the `input`
  track).

## Import notification while inactive (native-import-notification)

- An import of 18 files (676.5 MB) started from the app's `Import` dialog
  (Enter on `Run`), Notepad brought to the front, then MCP
  `cancel_operation`: the toast `Handpic` / `Import canceled` /
  `Import canceled, 13 files copied` appeared at the bottom right. PASS.
- The same import to an empty destination with Notepad in front: the toast
  `Import finished` / `Imported 18 files (676.5 MB)` appeared about 0.9 s
  after `Run`; a click on it brought the Handpic window to the front (the
  foreground PID went from Notepad's to Handpic's). PASS.
- The same import with the Handpic window kept in front: no change in the
  toast corner for 10 s. PASS.
- No `failed to register AppUserModelID` or `cannot show a system
  notification` line in any instance's log.
- Not seen: the taskbar button's icon and grouping (the taskbar is
  auto-hidden on this host and showed none of the instances in a grab).

## gpui-kit umbrella crate on Windows (native-gpui-kit)

- A copy of the real settings store (taken with SQLite's backup API, the
  real file opened read-only) with `windowBounds` `{x 1338, y 106, 1409 x
  856}`: the log said `window: restored on display 0 at Rect { x: 1338.0,
  y: 106.0, width: 1409.0, height: 856.0 }`, the client was 1409 x 856, and
  after `q` the copy held the same bounds. The folder was given on the
  command line, so the real `lastFolder` was not opened. PASS.
- Lucide icons (tree folders, strip and menu stars, orientation, face
  focus), the settings' `Radio`s, `Input`s, `Select`s, `Switch` and
  `NumberInput` drew in Neutral dark and in the copy's Gruvbox Dark. The app
  no longer has a `Checkbox` (no `Checkbox::new` in `crates/app/src`).
- `tab` in the main view toggled both side panes (`get_view` `panes`
  `left`/`right` false, then true). In the `Move to Trash` dialog `tab`
  moved the focus ring from `Move to Trash` to `Cancel` and back; Escape
  cancelled and the files stayed. PASS.

## Appearance (native-themes)

- Light (Neutral) and Catppuccin Latte, then Tokyo Night, set through MCP:
  the settings box (Appearance page) and the `Move to Trash` dialog took
  each theme and stayed readable. The strip's stars and pick dot read on the
  light strip. The label Tags stayed readable on a hovered row in Neutral
  light and dark.
- Pickers' keys: a click opened the dark theme `Select`; Escape closed the
  menu only (the box focused, the settings open); Space opened it; Down
  twice moved from Tokyo Night to Tokyo Moon; Enter picked it (`get_settings`
  `dark` `Tokyo Moon`); Enter on the focused box opened it again; Escape
  closed the menu, a second Escape the settings. PASS.
- Restart: with Catppuccin Latte in Light mode, a quit and relaunch came up
  in Latte (the first frame was not captured).
- Not run: System mode following the OS appearance, Dark staying dark
  whatever the OS does, and the scrollbars under an OS switch, since
  switching the OS appearance changes the shared desktop under the other
  sessions' checks; the tree cursor's ring and the progress bar in Light;
  the burst band on a shipped theme.

## Shift+Delete of a folder holding a watched one (Engine)

A scratch binary on `notify = "=8.2.2"` (`recommended_watcher`,
`NonRecursive` on a canonical temp folder `g\p\w` holding `a.ARW` and
`a.xmp`) deleted the watched folder, its parent and its grandparent with
`SHFileOperationW(FO_DELETE)` without `FOF_ALLOWUNDO`, the shell's permanent
delete as Explorer's Shift+Delete, flags `FOF_NOCONFIRMATION |
FOF_NOERRORUI | FOF_SILENT`, the path without the `\\?\` prefix (with it
the call returns `0x7c`). Two runs, the same result:

```text
depth=0 shell_permanent_delete -> 0x0 | exists=false | recreate=Ok | events=["Remove(Any) [\"a.ARW\"]", "Remove(Any) [\"a.xmp\"]"]
depth=1 shell_permanent_delete -> 0x0 | exists=false | recreate=Ok | events=["Remove(Any) [\"a.ARW\"]", "Remove(Any) [\"a.xmp\"]"]
depth=2 shell_permanent_delete -> 0x0 | exists=false | recreate=Ok | events=["Remove(Any) [\"a.ARW\"]", "Remove(Any) [\"a.xmp\"]"]
```

So the shell's permanent delete behaves like `std::fs`: free at every depth,
unlike the move to the Recycle Bin. `usage.md` needs no change.

## Right-click menus (native-context-menu)

- Tree: a right-click on the home root and on a non-root folder, with and
  without the tree holding the keyboard (a press on the pane's empty area
  first): a `PopupMenu` at the pointer, the groups separated, the root
  without `Rename…` and `Expand All` / `Collapse All`; the open folder did
  not change; with the keyboard held, Down afterwards moved the tree cursor
  (outline on the root), not the strip. PASS.
- Strip: inside a three-file selection the selection stayed and the right
  clicked file became current; outside it the selection became that file;
  the gap between two cells and the empty area right of the last JPEG cell
  opened nothing; the JPEG-only folder's menu had no judgment groups; the
  judgment groups showed `Kbd` shortcuts with the current flag, stars and
  label checked. PASS.
- Placement: a cell at the window's right edge opened the menu to the
  pointer's left; near the bottom it opened above the pointer; in a 597 px
  window the menu ran from y 74 (the 34 px title bar plus 40) to the window's
  bottom and scrolled. PASS.
- Items: `Rename…` on `V:\0c6-tree-test\one` ended with the inline `Input`
  focused, the name selected (gray in Neutral dark), typing going into it,
  Escape cancelling; `Copy File Name` put the file name on the clipboard; a
  star row and `p` with the menu open set the rating and the flag on the
  target; `Move to Trash…` opened its dialog (`Move 1 file to the Trash?`),
  `Cancel` closed it. PASS.
- With a menu open: Escape closed it; `p` closed it and picked; Right closed
  it and paged; a press outside closed it; a second right-click replaced it;
  a press on a menu item while the tree held the keyboard ran it and Right
  then paged the strip. FAIL: Down (and Enter) did not reach the menu: Down
  closed it and ran the view's `nextBurst` (position 7 to 8), in the strip's
  and the tree's menus alike. Handed to its own session
  (`dlg-c6-menukeys`).

## Folder tree rows (native-tree-listitem)

- The open row's name is drawn slightly heavier than its siblings at 100 %
  (semibold, subtle); the keyboard cursor's outline shows on the open row.
- A click on the open row's indent toggled it (expanded, then collapsed) as
  the icon does; a click on an open leaf row's indent did nothing.
- A folder whose listing fails (`two` with a deny ACE, `os error 5`) is red,
  also as the lit row, in Neutral dark, Neutral light, Catppuccin Latte and
  Catppuccin Mocha.
- The settings modal's `Input` selection reads gray in Neutral dark.
- Neutral's open row, hover and outline looked right and the 16 px indent
  left room for the names; no change to `map` or the indent.

## Strip menu order and star rows (native-strip-menu-order)

On a picked, 3-star, Yellow file: Pick, Unflag, Reject; five rows of 5 to 1
lit stars and a row of five dim stars; Red to Purple, then No label; checks
on Pick, the 3-star row and Yellow; the shortcuts `P`, `U`, `X`, `5` … `0`,
`Ctrl+Alt+1` … `Ctrl+Alt+0` at the right end. A click on the 4-star row set
4 stars (`get_photo`). PASS.

## Color label Tags (native-label-tags)

In Neutral dark and Neutral light: the eight label rows in `Filter ▾` and in
the strip's menu are small Tags with the name, unclipped, at the menu's row
height; a hovered row keeps its Tag readable; the filter popover is wide
enough; the strip menu keeps the check column and the right-aligned `Kbd`;
a Yellow and a Red cell's name bands are filled with the label's color with
the name readable. PASS.

## Always-visible scrollbars

- With the window at 637 px the meta pane overflowed: a drag of its thumb
  scrolled the pane, and a press on its track above the thumb scrolled it
  back to the top, with no wheel first. PASS.
- The notes and errors' right inset: an error line of a folder named with
  72 `l` wrapped with its ink ending at x 1586 in a pane whose right edge is
  x 1600, so 13 px from the edge with the glyph's side bearing, the 12 px of
  `pr_3`; the values end at x 1583, 7 px left of the thumb. PASS.
- Not run: the comparison with a build from before `native-scrollbar-gutter`
  (it needs a build of a commit before #1147, when the crate was
  `crates/native`); the bars under an OS appearance switch (see above).
