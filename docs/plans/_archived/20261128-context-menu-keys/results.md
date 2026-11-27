# Results: right-click menus driven by the keyboard on Windows 11

## Setup

- Machine: Windows 11, one 5120x1440 display, the app window 1600x1000
  client pixels; other Claude sessions sharing the desktop.
- Build: debug build of `7561c479` (branch `fix/context-menu-keys`) with
  `--features screenshot`, `RUSTC_WRAPPER=` (sccache bypassed), toolchain
  1.98.1.
- Launch: `handpic.exe --index D:\Photos\tests\20261128-cmk\run\index.sqlite
  --settings D:\Photos\tests\20261128-cmk\run\settings.sqlite --log
  D:\Photos\tests\20261128-cmk\run\Handpic.log --mcp-port 47881
  --import-settings D:\Photos\tests\20261128-cmk\import.json
  D:\Photos\tests\20261128-cmk\photos`, the import holding
  `{"mcpEnabled": true}`. Closed with `CloseMainWindow` at the end.
- Test folder: `D:\Photos\tests\20261128-cmk\photos`, 12 ARW hardlinks of
  `D:\Photos\tests\20261127-c3\big\a` (`_DSC1492` to `_DSC1503`).
- Input: posted window messages to the app's own `hwnd`, found by its PID
  (`WM_MOUSEMOVE` + `WM_RBUTTONDOWN` / `WM_RBUTTONUP` at client coordinates,
  `WM_KEYDOWN` / `WM_KEYUP` with the scan code and the extended-key bit for
  the arrows). Before every input the foreground window's PID was read: it
  was always the user's terminal (`WindowsTerminal`), never the app, so no
  window was raised and nothing went through the shared keyboard or pointer;
  the posted messages reach only this PID's window. The real pointer was
  never moved. Read back through MCP `get_view`, `screenshot` (`long_edge`
  1600, 1:1 with the client area) and the clipboard, set to `SENTINEL` before
  each run.

## Strip menu: Up / Down move the highlight, Enter runs it — PASS

- Right-click on `_DSC1495` (cell 4): the menu opened at the pointer
  (`Select All`, the reveal item, `Copy Path`, `Copy File Name`, then the
  judgment groups and the trash items); `get_view` `current.position` 4.
- Down, Down: the menu stayed open with the highlight on the second item,
  `Reveal in File Explorer`; `current.position` still 4 (before the fix the
  first Down closed the menu and ran the burst move).
- Down, Down, Up: the highlight on `Copy Path` (the third item), the menu
  open, `current.position` 4.
- Enter: the menu closed and the clipboard held
  `\\?\D:\Photos\tests\20261128-cmk\photos\_DSC1495.ARW` (see the finding
  below); `current.position` 4.
- Right afterwards paged the strip to 5: the keyboard was back on the view.

## Tree menu: Up / Down move the highlight, Enter runs it — PASS

- Right-click on the `20261128-cmk` row: the menu opened (the reveal item,
  `Copy Path`, `Copy Folder Name`, `Refresh`, `Rename…`, …); the open
  folder did not change.
- Down, Down: the highlight on `Copy Path` (the second item), the menu open.
  Enter: the clipboard held `D:\Photos\tests\20261128-cmk`.
- Again: right-click, Down, Down, Down, Up, Enter: the clipboard held
  `D:\Photos\tests\20261128-cmk` (`Copy Folder Name` would have given
  `20261128-cmk`).
- Right afterwards paged the strip (5 to 6).

## Finding (not fixed here)

- The strip's `Copy Path` copies the listed path with the Windows verbatim
  prefix (`\\?\D:\...`), while the tree's copies it without
  (`D:\...`). `strip_action` (`crates/app/src/main.rs`) joins `paths` as
  they are, without `context::shown_path`. Filed in `todo.md`.

Not done: Escape, `p`, Left / Right and the press outside with a menu open,
the second right-click and the tree's keyboard hand-back (the other clauses
of the `todo.md` box; the UI tests cover Escape, `p` and Right headless).
