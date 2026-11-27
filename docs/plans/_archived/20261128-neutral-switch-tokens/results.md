# Results: Neutral switch tokens

## Setup

- Machine: Windows 11 Home 10.0.26200, 100 % scale, window 1600 x 1000.
- Build: debug, `cargo build --features screenshot` in `crates/app` (what
  `mise run app:dev:screenshot` runs), at `1c0426a5` (the later commits on the
  branch touch only the plan and the review history).
- Launch: `handpic.exe --index <tmp>\index.sqlite --settings
  <tmp>\settings.sqlite --log <tmp>\Handpic.log --import-settings
  <tmp>\seed.json --mcp-port 47880`, the seed `{"mcpEnabled": true}`, `<tmp>`
  a fresh folder in the session's scratchpad. No folder was opened.
- Driving: MCP `set_settings` for the theme (`Neutral` in both modes) and
  `auto_advance: false` while the settings were closed; `Ctrl+,`, `Escape` and
  left clicks posted to the instance's own window by PID (`PostMessage`, the
  rule in `docs/agents/app.md`'s "Driving the window in a real-device run on
  Windows"); MCP `screenshot` at `long_edge` 4096 (1600 x 1000, device
  pixels); colors read from pixel runs along the switch's middle row (y 165)
  with `System.Drawing`.
- Quit with `CloseMainWindow` on the instance's PID; the log shows the four
  `quit:` drain lines.

## Check 1: Neutral dark, auto-advance switch off and on — PASS

- Off (unfocused): surface `#171717`, track `#737373`, thumb `#0a0a0a`
  (x 1534-1549). The thumb reads as a dark knob on a mid-gray pill, no longer
  a hole.
- On (focused after the click): track `#e5e5e5`, thumb `#0a0a0a`, as before
  the change.
- Off and focused (a second click): the 1 px `ring` border (`#737373`) merges
  with the track as expected, but the focus halo outside it (`#454545`, 3 px,
  and up to `#454545` above the track at y 152) still marks the focus. Kept
  `#737373`; `#a1a1a1` was not needed.

Not done: the MCP page's switches (same tokens); the user's own look at the
running window.

## Check 2: Neutral light, auto-advance switch off and on — PASS

- Off (unfocused): surface `#ffffff`, track `#d4d4d4`, thumb `#ffffff`
  (today's look, now pinned). It reads as a light switch, shadcn's shape.
- On (focused): track `#171717`, thumb `#ffffff`, halo `#d0d0d0`.

Not done: a focused off switch in light; the MCP page's switches; the user's
own look at the running window.

## Check 3: right-click menu `Kbd` shortcuts — not run

The deleted `todo.md` item also named them; the 2026-11-27 run reported the
`Kbd` chips (Keyboard Shortcuts page and right-click menus) fine, so they
were not looked at again.
