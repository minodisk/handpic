# Device check results: empty-text-wrap

## Setup

- Machine: Windows 11 Home (10.0.26200), 100 % scale.
- Build: debug with the `screenshot` feature (`mise run app:dev:screenshot`,
  `CARGO_BUILD_JOBS=2`) at commit `c26f4262` (the fix).
- Launch: a fresh `--index`, `--settings` and `--log` in a scratch folder,
  `--mcp-port 47890`, `--import-settings` with `{"mcpEnabled": true}` plus
  `panelWidths` `{"left":480,"right":480}` and a 1200 x 800 `windowBounds`
  (narrow run), or no `panelWidths` and a 1600 x 900 `windowBounds` (wide
  run). No folder opened.
- Captures through the MCP companion's `screenshot`; synthetic clicks
  (`SetCursorPos`, three relative `mouse_event` moves, the cursor read with
  `GetCursorPos`, then the press) while holding `D:\Photos\tests\.ui-lock`
  exclusively; the picker detected as a `#32770` window of the app's PID and
  closed with `WM_CLOSE`.

## PASS: the no-folder message wraps inside a 240 px preview

The preview's black surround spanned x 481 to 718. The message wrapped onto
three centered lines ("Drop a folder onto the window, / or click here to
choose one. Or / press ctrl+o."), its glyphs at x 500 to 696, y 290 to 348.
Nothing was drawn over the tree or under the meta pane.

Not done: macOS and Linux.

## PASS: one line in a wide window

At 1600 px with the default 220 px panes the message was one line, as before
the change.

Not done: macOS and Linux.

## PASS: a click on the wrapped text opens the picker; the bands and the tree do not

- The text's center (x 598, y 318): the picker opened.
- x 481 and x 715 (on the bands): nothing opened.
- x 420 (the tree): nothing opened.
- x 485 and x 488: the picker opened (x 485 was recorded as on the band in
  the brief's earlier run at x 486; the band's hit edge reads a pixel or so
  apart between runs).

Not done: macOS and Linux.
