# Learnings: native app on the gpui-kit umbrella crate

## Step 1

- The lock regenerated with additions only: `gpui-kit` 0.9.1 plus the Linux
  X11 / Wayland / AT-SPI stacks the `x11` and `wayland` features pull in
  (`x11rb`, `wayland-*`, `xkbcommon`, `zed-xim`, `atspi*`, `accesskit_unix`,
  `zbus-lockstep*`, ...). No existing package changed version (no `-name` /
  `-version` line in the lock diff), so `gpui-pre`, `gpui-pre-platform`,
  `gpui-component`, `gpui-base` and `gpui-kit-assets` stayed where they were.
- `cargo tree` shows `gpui-pre` at the one version 0.3.12 and no `gpui-fast*`
  package; the lock's only git sources are still Zed's `async-process` and
  `async-task` forks.
- `cargo tree -e features -i gpui-pre-platform` after the switch (the same
  feature set for the host, `--target x86_64-unknown-linux-gnu` and
  `--target aarch64-apple-darwin`):

  ```text
  gpui-pre-platform v0.3.12
  ├── gpui-pre-platform feature "default"
  │   └── gpui-kit v0.9.1
  │       ├── gpui-kit feature "assets"
  │       │   └── gpui-kit feature "default"
  │       │       └── riffle-native v0.0.0
  │       │           └── riffle-native feature "default" (command-line)
  │       ├── gpui-kit feature "component"
  │       │   └── gpui-kit feature "default" (*)
  │       └── gpui-kit feature "default" (*)
  ├── gpui-pre-platform feature "font-kit"
  │   └── gpui-kit v0.9.1 (*)
  ├── gpui-pre-platform feature "multithreaded"
  │   └── gpui-pre-platform feature "default" (*)
  ├── gpui-pre-platform feature "runtime_shaders"
  │   └── gpui-kit v0.9.1 (*)
  ├── gpui-pre-platform feature "wayland"
  │   └── gpui-kit v0.9.1 (*)
  └── gpui-pre-platform feature "x11"
      └── gpui-kit v0.9.1 (*)
  ```

- The `actions!(test, [Focus])` in `main.rs`'s test module compiles without
  `use gpui_kit::gpui;`: `use gpui_kit::actions` picks gpui-kit's own
  `actions!` (a `#[macro_export]` at the crate root shadows the glob-exported
  GPUI one), which derives through `$crate::Action`, and the derive's output
  resolves without a crate named `gpui`. The plan's `use gpui_kit::gpui;` was
  not needed.
- Fully-qualified `gpui::X` paths in function signatures (`gpui::Div`,
  `gpui::App`, `gpui::ShapedLine`, ...) became `gpui_kit::X`; they would not
  resolve otherwise, as no crate is named `gpui` any more.
- `gpui_kit::open_window`'s build closure gets `&mut App`, as `cx.open_window`'s
  did, so the `Riffle` construction moved over unchanged; `Root` left
  `main.rs`'s import list.
- `cargo check --tests` printed no "did not match any packages" warning for
  the `[profile.dev.package]` block.
- `mise run fmt` first failed with `Cannot find module ...node_modules\vite-plus\bin\vp`:
  the fresh worktree had no `node_modules`. `mise exec -- pnpm install
  --frozen-lockfile` fixed it (`pnpm` is only on the PATH through mise).
- `mise run ci:local` passed on the first run (879 native tests, clippy).
- Windows launch: the debug binary, run with `--index`, `--settings` and
  `--log` pointed at a scratch folder, opened its window, picked the GPU and
  ran 20 s without a panic until `timeout` stopped it. The log's
  `0x887A002D` line is the DXGI debug layer missing on this host ("Failed to
  get DXGI debug interface"), not this change. The visual checks the plan
  names were not made by the agent (see the pending manual check below).

## Deferred issues (todo candidates)

- Pending manual check (Windows): run `mise run native:dev` with the real
  settings and confirm the window restores its saved place, the Lucide icons
  (strip, menus) and the settings modal's controls (the `Checkbox` check
  mark, the `Radio`s, the `Input` fields) draw, and `tab` behaves as before
  (`toggleSides` in the main view, the focus walk inside a dialog). Step 1's
  checkbox was ticked on the automated criteria and a 20 s launch without a
  panic.
- Pending manual check (macOS): the macOS build now has `font-kit` and
  `runtime_shaders`. On a Mac, run `mise run native:dev` and confirm the window
  draws text (the status row, the meta pane, the menus) and opens without a
  shader error at startup. Step 1's checkbox was ticked on the automated
  criteria.
- Pending manual check (Linux): the Linux build now compiles the X11 and
  Wayland backends. In an X11 session and in a Wayland session, run
  `mise run native:dev` and confirm the window opens, draws, and takes the
  keyboard. Step 1's checkbox was ticked on the automated criteria.
