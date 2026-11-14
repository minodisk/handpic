<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Native App Icon

## Purpose

The GPUI native app (`crates/native`, `riffle-native`) has no app icon: on
Windows the exe, taskbar button and title bar show the generic icon, on macOS
there is no `.app` bundle at all, and on Linux the window carries no `app_id`
a desktop environment could match to an icon. Once this is done the native app
shows the Handpic icon on all three OSes, reusing the Tauri app's assets in
`crates/app/icons/` and the one identifier in `crates/native/src/datadir.rs`
(`IDENTIFIER`, `dev.minodisk.handpic`).

## Decisions

Taken with the user at plan approval (2026-11-14):

- The bundle identifier and the Linux `app_id` are `IDENTIFIER`, shared with
  the Tauri app (the native app replaces it, so no rename at the switch-over).
- The Windows icon is embedded with `winresource`.
- macOS: a mise task with a checked-in `Info.plist`, not `cargo-bundle`; no
  `.dmg`, no signing.
- The `native` CI job's macOS leg builds and checks the bundle.
- One step, one PR.

## Steps

- [x] Step 1: Give the native app its icon on Windows (exe resource), macOS
      (a `.app` bundle task) and Linux (`app_id`, a `.desktop` file and an
      install task)
  - Done when:
    - Windows: `crates/native/build.rs` embeds `crates/app/icons/icon.ico` as
      icon resource ID 1 on a Windows target, and the NASM check still behaves
      as before. Checked by hand on the dev machine: `mise run native:dev`
      shows the Handpic icon in the taskbar and the title bar, and Explorer
      shows it for `crates/native/target/debug/riffle-native.exe`.
    - Linux: the window is opened with `app_id: Some(IDENTIFIER)`, a
      checked-in `<IDENTIFIER>.desktop` exists, and a documented mise task
      installs the binary, the `.desktop` file and the icon for the current
      user.
    - macOS: a documented mise task produces `Handpic.app` (the binary, a
      checked-in `Info.plist`, `icon.icns`).
    - A unit test in the native crate, run on all three CI legs, asserts that
      the checked-in `Info.plist` and `.desktop` file agree with `IDENTIFIER`
      (bundle identifier; the `.desktop` file's name, `StartupWMClass` and
      `Icon`) and with the binary's name (`CFBundleExecutable`, `Exec`).
    - The `native` CI job's macOS leg builds the bundle and checks it
      (`plutil -lint` on its `Info.plist`, the executable and the `.icns`
      present).
    - `CLAUDE.md`'s layout paragraph and `docs/agents/native-app.md` describe
      the icon resource, the `app_id`, the packaging files and the two tasks.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass (the
      `native` job on Windows, Linux and macOS).
    - Recorded in the PR as pending manual checks, not blocking the merge:
      the Dock / Finder icon of the built `.app` on a Mac, and the icon of the
      running window after the install task on a Wayland and an X11 session.
  - Implementation approach:
    - Windows resource ID: `gpui-pre-windows` 0.3.12's `load_icon`
      (`src/platform.rs`) calls `LoadImageW(module, PCWSTR(1), IMAGE_ICON, 0,
      0, LR_DEFAULTSIZE | LR_SHARED)` and puts the handle in the window
      class's `hIcon` (`src/window.rs`, `register_window_class`); a failed
      load silently becomes no icon. So the icon must be resource ID 1 of the
      exe. Explorer shows the first icon group of the exe, so the one resource
      serves all three places.
    - Use `winresource` as Zed does (`res.set_icon(..)`, then `compile()`),
      added as a build-dependency for Windows only. It is a new dependency:
      note it in the PR and commit the `crates/native/Cargo.lock` change.
      Verify during implementation that `set_icon` emits ID `1`; if it does
      not, use `set_icon_with_id(.., "1")`.
    - `gpui-pre-platform` enables `gpui-pre`'s `windows-manifest` feature on
      Windows, so GPUI's own build script already embeds `1 RT_MANIFEST`. Do
      not set a manifest in the app's resource (a second one is a duplicate
      resource at link time). An icon with ID 1 is a different resource type
      and does not collide.
    - `build.rs` today returns early for a non-x86 arch and when NASM is
      found. Run the icon embed before those returns (or restructure into two
      functions) so it runs on every Windows build, and decide on the target
      with `CARGO_CFG_TARGET_OS`, as the file already does with
      `CARGO_CFG_TARGET_ARCH`. Add `cargo:rerun-if-changed` for the `.ico`
      path. Keep the NASM check's behavior and messages unchanged.
    - Reference the icon by path, `../app/icons/icon.ico` from
      `CARGO_MANIFEST_DIR`; no copy. This is a file path, not a Cargo
      dependency on `crates/app`, which stays forbidden.
    - Linux `app_id`: `WindowOptions.app_id: Option<String>` exists in
      `gpui-pre` 0.3.12 (`src/platform.rs`). Set it to
      `datadir::IDENTIFIER` in the `cx.open_window` call in
      `crates/native/src/main.rs`. It is harmless on Windows and macOS (the
      default `set_app_id` is a no-op).
    - Linux `.desktop`: Wayland matches the toplevel's `app_id` to the
      `.desktop` file's base name, so the file is named
      `dev.minodisk.handpic.desktop`. Add `StartupWMClass=<IDENTIFIER>` for
      X11. Read what `gpui-pre-linux`'s X11 `set_app_id` writes to `WM_CLASS`
      (`cargo fetch --target x86_64-unknown-linux-gnu` in `crates/native`
      brings the source) and make `StartupWMClass` equal to it.
    - Linux install: a mise task (suggested `native:install:linux`) that
      builds release and installs for the current user: the binary to
      `~/.local/bin/riffle-native`, the `.desktop` file to
      `~/.local/share/applications/`, and a PNG from `crates/app/icons/` to
      `~/.local/share/icons/hicolor/<size>/apps/dev.minodisk.handpic.png`,
      with `Icon=dev.minodisk.handpic`. The icon's size directory must equal
      the PNG's real pixel size. Decide between a static `Exec=riffle-native`
      (needs `~/.local/bin` on the session's `PATH`) and writing the absolute
      path at install time.
    - macOS bundle: a checked-in `Info.plist` plus a mise task (suggested
      `native:bundle:macos`). The task builds, then lays out
      `Handpic.app/Contents/{Info.plist, MacOS/riffle-native,
      Resources/icon.icns}` under `crates/native/target/`, copying
      `crates/app/icons/icon.icns`. The plist needs at least
      `CFBundleIdentifier` (= `IDENTIFIER`), `CFBundleName` (`Handpic`),
      `CFBundleExecutable` (`riffle-native`), `CFBundleIconFile`,
      `CFBundlePackageType` (`APPL`), `NSHighResolutionCapable` and a version.
      Let the task take the profile so CI can bundle the debug build. No
      signing or notarization.
    - Put the packaging files under `crates/native/packaging/macos/Info.plist`
      and `crates/native/packaging/linux/dev.minodisk.handpic.desktop`. Define
      the tasks inline in the root `mise.toml` next to `native:dev` /
      `native:release` / `native:check`, with the same `dir`, `tools` and
      `shell` keys (a new script directory under `tools/` would also need the
      shellcheck globs and `tools/ci/local.sh` extended).
    - The consistency test lives where `IDENTIFIER` does
      (`crates/native/src/datadir.rs` tests) and reads the files under
      `CARGO_MANIFEST_DIR` at test time.
    - CI: in `.github/workflows/ci.yml`'s `native` job, the bundle check is
      one macOS-only step after the existing lint and test step, run through
      `mise exec rust@1.98.1 --` like the others. Keep within the job's 30
      minute timeout; measure the added time on the first run.
    - Platform-only code must not fail `-D warnings` on the other two legs.
    - Docs: add a section to `docs/agents/native-app.md` (resource ID 1 and
      why, the manifest GPUI already embeds, the `app_id` / `.desktop` name
      rule, the two tasks, the pending manual checks), and extend the
      `crates/native` part of `CLAUDE.md`'s layout paragraph. Leave
      `README.md` / `README.ja.md` alone.

## Trade-offs and risks

- **`winresource` over the `embed-resource` already in the lock.** A
  hand-written `.rc` through `embed-resource` would add no crate and spell the
  ID out; `winresource` was chosen so version info can be added later without
  an `.rc`. Switching is a small change to `build.rs` and `Cargo.toml`.
- **The native app and the Tauri app share `IDENTIFIER`.** With both
  installed, macOS LaunchServices and Linux desktop matching can confuse them.
  Accepted, since the native app replaces the Tauri app.
- **What cannot be verified on the Windows dev machine.** The Dock / Finder
  icon and the Wayland / X11 icon match are manual checks on other machines;
  the PR records them as pending. A wrong `StartupWMClass` only loses the
  icon on X11.
- **Icon changes do not select the native check.** `tools/ci/local.sh` runs
  the native checks only for paths under `crates/native/`, so a later change
  to `crates/app/icons/icon.ico` alone is not built into the native exe
  locally; the PR's `native` job still builds it.

## Progress

- (2026-11-14) Step 1 complete
