# Learnings

## Step 1

- The mapping needed only a `switch` field on `Palette` (set in `DARK` and
  `LIGHT`) and two lines in `map`; `switch_thumb` reads the palette's
  `background`, so no extra field was needed for it.
- `todo.md`'s section was deleted whole; the neighbouring
  "settings controls drawn with gpui-component (native-component-controls)"
  section, with the same plan tag, stays (it covers the keyboard checks, not
  the look).

## Deferred issues (todo candidates)

- Pending manual check (the main agent's wrap-up, per the plan's "Manual
  check"): on Windows, a debug build with the `screenshot` feature
  (`mise run app:dev:screenshot`) on a temp `--index` / `--settings` /
  `--log`, `--mcp-port 47880` and `--import-settings` of
  `{"mcpEnabled": true}`. In Neutral dark and Neutral light, set the theme
  and `auto_advance` through MCP `set_settings`, open the settings with
  `ctrl+,`, select Culling (or the MCP page, an on and an off switch side by
  side) and take MCP `screenshot`s. Expected at 100 %: dark off track
  `#737373` with a `#0a0a0a` thumb on `#171717`, dark on track `#e5e5e5`;
  light off track `#d4d4d4` with a `#ffffff` thumb on `#ffffff`, light on
  track `#171717`; the off switch reads as a switch, not a hole. Also look at
  a focused off switch (its `ring` border `#737373` in dark). Needs the
  Windows host, a build, MCP tools and screenshots. Files:
  `crates/app/src/theme.rs`. The step's checkbox was ticked on the automated
  criteria only.
