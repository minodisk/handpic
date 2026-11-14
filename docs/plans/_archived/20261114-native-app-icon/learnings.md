# Learnings: Native App Icon

## Step 1

- `winresource` 0.1.34's `set_icon` writes the icon under ID `"1"`
  (`DEFAULT_APPLICATION_ICON_ID`), so `set_icon_with_id` was not needed. It
  also adds an `RT_VERSION` 1 from the Cargo metadata by default; no manifest.
- Checked on the built debug exe with `FindResourceW` / `LoadImageW` through
  Python's `ctypes`: `RT_GROUP_ICON` 1 (90 bytes), one `RT_MANIFEST` 1 (GPUI's)
  and `RT_VERSION` 1 present, and `LoadImageW(module, 1, IMAGE_ICON, 0, 0,
  LR_DEFAULTSIZE | LR_SHARED)` succeeds, the same call GPUI makes.
- `winresource` writes the path into the `.rc` escaped, so the absolute path
  from `CARGO_MANIFEST_DIR` (mixed `\` and `/`) is fine.
- `gpui-pre-linux`'s X11 `set_app_id` writes `WM_CLASS` as
  `<app_id>\0<app_id>\0` (instance and class), so `StartupWMClass` is
  `IDENTIFIER`. `cargo fetch --target x86_64-unknown-linux-gnu` in
  `crates/native` brought its source onto the Windows host.
- In a mise `run = """..."""` (a TOML basic string), a `\` at the end of a
  line is TOML's line-ending backslash and swallows the newline, so a shell
  line continuation needs `\`; the install task keeps each command on one
  line instead.
- `.desktop`'s `Exec` is the static `riffle-native` (no `%f`: no `MimeType`
  is declared, so nothing would pass a file), which needs `~/.local/bin` on
  the session's `PATH`; the consistency test compares it with
  `CARGO_BIN_NAME`.
- The CI bundle step runs the `native:bundle:macos` task itself (`mise run
  native:bundle:macos debug`) rather than `mise exec rust@1.98.1 --` commands,
  so CI exercises the task the docs describe; the Windows-only nasm `PATH`
  problem that made the other step avoid `mise run` does not apply on macOS.
  The added time is to be measured on the PR's first run.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the dev machine): run `mise run native:dev`
  and confirm the Handpic icon in the taskbar button and the title bar, and in
  Explorer for `crates/native/target/debug/riffle-native.exe`. Expected: the
  Handpic icon, not the generic one. The step's checkbox was ticked on the
  automated criteria (the icon resource ID 1 verified in the built exe with
  `FindResourceW` / `LoadImageW`). Basis: plan Step 1's Done when; files
  `crates/native/build.rs`.
- Pending manual check (macOS): run `mise run native:bundle:macos`, open
  `crates/native/target/release/Handpic.app` and confirm the Handpic icon in
  the Dock and in Finder. The step's checkbox was ticked on the automated
  criteria (the CI macOS leg's `plutil -lint` and file checks). Basis: plan
  Step 1's Done when; files `crates/native/packaging/macos/Info.plist`,
  `mise.toml`.
- Pending manual check (Linux, a Wayland session and an X11 session): run
  `mise run native:install:linux` with `~/.local/bin` on `PATH`, launch
  Handpic from the application menu and confirm the running window shows the
  Handpic icon in the dock / taskbar / alt-tab. The step's checkbox was ticked
  on the automated criteria (the consistency unit test). Basis: plan Step 1's
  Done when; files `crates/native/packaging/linux/dev.minodisk.handpic.desktop`,
  `crates/native/src/main.rs`, `mise.toml`.
- Once the three checks above are done (or a failure is filed as its own
  issue), update the "Pending manual checks" bullet under `## Packaging` in
  `docs/agents/native-app.md`; it lists only macOS and Linux today and leaves
  out the Windows check. Done when the bullet matches what is still pending.
