# Learnings: record the Windows real-device checks of the folder dialogs

## Step 1

- Posted input closed on Windows what MCP alone could not: Tab, Escape,
  Enter, `Ctrl+Z` and `Ctrl+,` posted as `WM_KEYDOWN` / `WM_KEYUP` with the
  modifier state set through `AttachThreadInput` + `SetKeyboardState`, and
  right and left clicks posted at client coordinates. What it cannot do is
  an OLE drag: both `DoDragDrop` attempts failed (an own `IDropSource` with
  `E_FAIL`, a WinForms one returning `None` with no outline drawn), and one
  left the left button down until it was released by hand.
- The setup worth keeping: copies and hardlinks under
  `D:\handpic-c4-checks\`, outside the photo folders; a fresh `--index`,
  `--settings` and `--log` per launch with `--mcp-port` and
  `--import-settings`; `mcpAllowDestructive` off so every folder operation
  opens its dialog.
- Timing: a short preview ends before a posted key or a capture arrives (the
  Delete preview of 125 files, the 9-file import preview), so the `ctrl+,`
  and `reading …` checks were taken on a 500-JPEG import preview.
- The finding: the open folder's watcher (`crates/engine/src/watch.rs`)
  never settles during an unbroken copy, since it reports a burst `DEBOUNCE`
  after its last event; the strip of an import's open date folder stays
  empty until that folder's copies end. The fix (a maximum wait on the
  debounce) is delegated to the session `dlg-c4-import-watch`, and the
  narrowed Import box in `todo.md` carries the re-check.
- The checks' material was not under the
  `D:\Photos\tests\20261113-gpui-folder-dialogs\` paths the deleted setup
  boxes named.
- The edits were made by short Python scripts that assert each old string
  occurs exactly once, which keeps a concurrent rewording of `todo.md` on
  `main` from being overwritten silently.

## Wrap-up

- After the step, the folder drop was tried again with the `mouse_event` drag from Explorer that `docs/agents/app.md` records. It went unreceived twice; a desktop capture mid-drag showed other sessions' Handpic windows over this session's window at the target, one with its settings open, which swallowed the drop. The attempt was stopped there; the drop boxes stay open. Consolidated into `docs/agents/app.md` ("Driving the window in a real-device run on Windows") with the posted-input and short-preview rules, and the watcher caveat into `docs/agents/shared-crates.md`.
