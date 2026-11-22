# Learnings: Windows real-device checks, batch 7

## Step 1

- **Check the foreground window before every synthetic input.** On this run a launched window often did not come to the front at the first `SetForegroundWindow`, while the terminal kept it. Keys and the first burst of `cold-a\` went elsewhere, and the cold set was spent: pass 1 lasts only about 4 s on 600 cold ARWs. What worked: launch with no folder, retry the focus until `GetForegroundWindow` equals the instance's window (by PID), then open the folder over MCP `open_folder` with the burst already running. Basis: `results.md`, Incidents and box 1 of the status row item.
- **Keys sent while a listing runs are dropped**, so paging "during pass 1" has to start after the listing (3.7 s for 600 files here) and fit in what is left of pass 1. Basis: `results.md`, box 1.
- **`SetWindowPos` cannot shorten the app's window**: any asked height came out 1460 (the display's 1440 plus the frame), while width and position applied. Dragging the window's own bottom edge with synthetic mouse input resized it normally. Use edge drags for short-window checks. Basis: `results.md`, Incidents.
- **The cursor shape can be read with `GetCursorInfo`**, compared against `LoadCursor(IDC_SIZENS / IDC_ARROW / IDC_HAND)`. Move the pointer away and back, and wait about 400 ms, before reading it; one 250 ms read lagged a step behind. This mapped the handle's 5 px band (y 802–806) exactly. Basis: `results.md`, the strip header's box 3.
- **An OLE drag from Explorer can be made with `mouse_event`**: click, shift-click, press, about 60 small moves with 25 ms between them, a short wiggle over the target, release. The first two-file attempt was not received (no outline). The same drag again lit the outline and showed `DROP_TEXT`, so retry once before calling a drop a failure. Close the Explorer window afterwards through `Shell.Application`'s `Windows()` by its `LocationURL`, not with keys. Basis: `results.md`, the status row with no current-file info.
- **Measure layout by scanning pixel runs** along a row or column of a window capture. The pane borders (`#1a1a1a`), the status row's border (`#2f2f2f`) and the preview's black surround separate cleanly in Neutral dark. A no-folder window has no photo in the way. There is no line between the strip header and the cells, so the header's height comes from the cells' symmetric padding. Basis: `results.md`, the strip header's box 7.
- **The measured canvas (1158 x 767 of surround) does not match the inferred 1160 x 810.** The earlier 1160 x 842 came from a debug log of the paint bounds, which may count differently from the visible surround. The difference was not traced and is recorded as measured. Basis: `results.md`, box 7.
- **A crop re-cut prints no `zoom` line**: only a `z` press is timed. A re-cut after a pane or strip drag has to be judged from captures. Also, a drag that makes the canvas smaller re-cuts nothing, because the held crop still covers it (`Zoom::holds`); drag so that the canvas grows. Basis: `results.md`, the scrollbars' box 7.
- **The settings modal reopens on its first page (Sidecar)**, not the page last shown. A press aimed at a later page's control can hit a Sidecar control at the same place: it chose the `Both` format once. Open the page by its sidebar entry first. Basis: `results.md`, Incidents.
- **PowerShell variables are case-insensitive** (again): a window handle `$h` overwrote a `-H` height parameter in a helper. Name handles `$hw`.
- **Python on Windows writes CRLF** through `Path.write_text`. Convert edited repository files back to LF before committing.
- Deferred checks: listed in the narrowed `todo.md` boxes:
  - the canvas in the other pane layouts
  - the meta pane's thumb drag and its notes' right inset
  - the non-overflowing tree compared with a pre-gutter build
  - an OS appearance change in System mode

  Plus a new item: the window shrink leaving the preview under `PREVIEW_MIN`.
