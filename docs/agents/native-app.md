# Working on `crates/native`

Read this before touching the GPUI app in `crates/native`. It lists what the
port has met so far, each with the reason.

The tags follow [`tauri-app.md`](./tauri-app.md): **Hit** broke something here,
**Measured** steered a design decision, **Inferred** comes from sources or docs
only.

Source: [the gpui-port plan](../plans/20261109-gpui-port/plan.md), its spike
and Step 1's [learnings](../plans/20261109-gpui-port/learnings.md).

## Toolchain

### GPUI is the `gpui-pre` snapshot gpui-component pins (Inferred)

`gpui` and `gpui_platform` are the crates.io `gpui-pre` and
`gpui-pre-platform` at `=0.3.12`, a snapshot of Zed's GPUI at `zed@d2a4fb6`
published by the gpui-kit maintainer, renamed with `package =` so the code
keeps the `gpui` names. That is the exact version `gpui-component` 0.9.1
pins. Zed pins Rust 1.98.1 at that commit, and so does
`crates/native/mise.toml`; the published crates declare no `rust-version`.
Moving from the Zed git rev `b5cd310b` to the snapshot changed no source
file. The plan: [gpui-component-dep](../plans/_archived/20261114-gpui-component-dep/plan.md).

- Rule: the native crate never picks a GPUI version of its own. An upgrade
  moves `gpui-component`, `gpui-kit-assets`, `gpui-pre` and
  `gpui-pre-platform` together in one deliberate chore PR (all four exact
  pins), and records the API changes met at the bump here.
- Rule: `cargo tree` shows one GPUI: no package from
  `git+https://github.com/zed-industries/zed`, `gpui-pre` at one version. Two
  GPUIs have distinct types, so no gpui-component element could join the
  app's views, and `[patch]` cannot swap `gpui-pre` for Zed's `gpui` because
  the package names differ.

### Only two of Zed's patches are needed (Inferred)

`[patch.crates-io]` carries Zed's `async-task` and `async-process` forks, which
now apply to the crates.io `gpui-pre` crates' dependencies on them (Zed at
`d2a4fb6` carries the same two revs). They are the only git sources in
`Cargo.lock`. Zed's other patches (`cpal`, `notify`, `calloop`,
`khronos-egl`, ...) are not needed by `gpui` / `gpui_platform`.

- Rule: do not copy the others. Do not move these two to the root workspace
  while the Tauri app lives there: both crates are in its lock, so a workspace
  patch would swap them for Zed's forks in the shipped app.

### gpui-component is initialized; the settings modal and the tree's inline rename draw from it (Inferred)

`gpui-component` `=0.9.1` is compiled on all three platforms.
`gpui_component::init(cx)` runs first in the `application().run` closure,
before the window opens, then `theme.rs`'s `apply` sets the global `Theme`
to dark (`Theme::change(ThemeMode::Dark, ..)`, the built-in `Default Dark`)
and maps it over it in a second `Theme::update`, since a mode switch reloads
the registered theme over color edits made in the same closure. The mapping
reads `main.rs`'s color constants (one source of each color, the overlay included, which the three backdrops read; the canvases
keep reading them): `background` `#0a0a0a` (`theme.rs`'s own constant), `foreground` and every
`*_foreground` on a dark surface `#fafafa`, `popover` `#171717`, `primary`
`#e5e5e5` on `primary_foreground` `#171717`, `secondary`, `muted` and
`accent` `#262626`, `muted_foreground` `#a1a1a1`, `danger` `#ff6467`,
`border` and `scrollbar_thumb` white at 10 %, `input` white at 15 %, `ring`
`#737373`, `overlay` `#00000099`, `font_size` 16 px (the rem `Root` sets, see
"`Root` wraps the window's content"), `radius`
6 px (`rounded_md`, the buttons and menus) and `radius_lg` 8 px
(`rounded_lg`, the dialog boxes), and `Button`'s default variant: `button`
`BUTTON_FILL` (white at 4 %), `button_hover` and `button_active` `#262626`,
`button_foreground` `#fafafa`. `Default Dark` derives the `button_*`,
`*_hover` / `*_active` and `list_*` fields at load and `Theme::update` does
not derive them again, so the fields it does not name keep `Default Dark`'s
values. Nothing calls `Theme::sync_system_appearance` or observes
the window appearance, so a light OS setting leaves the app dark. The
components drawn are the settings modal's: `Input` (its text fields, see
"Text is typed through gpui-component's `Input`"), `Button`, `Checkbox`,
`Radio` and `ScrollableElement`'s scrollbar (see "The settings modal is its
own entity"), the tree's inline rename's `Input` (see "The tree's inline
rename is a text input in the row") and `Tooltip`, every tooltip of the app
(see "A strip cell's tooltips are transparent `div`s over the canvas"). The
window's content is wrapped in `Root` (see "`Root` wraps the window's content").
The app view's own black and white stay over `Root`'s surface; the theme is for
components.

None of its optional features (`tree-sitter*`, `syntect`, `speech`,
`inspector`, `decimal`, `gpui-fast`) is on, and it does not depend on
`gpui-pre-platform`, so it turns on no `runtime_shaders`
(`cargo tree -e features -i gpui-pre-platform` shows only the default
`multithreaded`).

`init` puts the first bindings into GPUI's keymap. As verified at 0.9.1
(`gpui-component` and `gpui-base`), every one is scoped to a key context
except the inspector's, and none is a multi-keystroke sequence, so none adds
a pending-key delay. GPUI dispatches a matching binding's action before any
key-down listener, so a binding that matches takes the key from
`Riffle::key_down`:

| Context | Keys | Action |
| --- | --- | --- |
| none (debug builds, or the `inspector` feature) | `ctrl-shift-i` (`cmd-alt-i` on macOS) | `ToggleInspector`, a global `cx.on_action` that toggles GPUI's inspector |
| `Root` | `tab`, `shift-tab`, `ctrl-c` (`cmd-c` on macOS) | `Tab` / `TabPrev`, which move the focus (inside a dialog's focus trap) and do not propagate, masked by `NoAction` in the app view's `Handpic` context; `Copy`, which propagates when no text is selected |
| `Input` | the editing keys, `enter` / `escape`, `tab` / `shift-tab`, the arrows, Home / End, word and line movement, undo / redo, the clipboard keys | the input's own; `PressEnter`, `Escape` and the indents propagate on a single-line input |
| `Dialog` (only with `.keyboard(true)`) | `escape`, `enter` | `dialog::Cancel`, `dialog::Confirm` |
| `Popover`, `PopupMenu`, `Select`, `Tree`, `TextView`, ... | their own | inert until such an element is drawn |

- Rule: the inspector binding is left in place. In a debug build
  `ctrl+shift+i` opens the inspector and a user binding on it is shadowed;
  a release build has no such binding. No default of
  `crates/backend/src/shortcuts.rs` and none of `settingsmodal::APP_KEYS`
  uses it.
- Rule: `Root`'s `tab` / `shift-tab` are masked inside the app's view, so
  the keymap keeps them (see "`Root` wraps the window's content").
- Rule: a version bump of `gpui-component` re-reads the bindings: grep
  `KeyBinding::new` in both crates for a `None` context and for the `Root`
  context, and update the table.
- Rule: the app already has its own `AssetSource` (`icons.rs`); GPUI takes
  one per application, so `Icons` falls back to `gpui_kit_assets::Assets`
  (`gpui-kit-assets` `=0.9.1`, the components' own icon set, not the
  whole-catalog `AllAssets`) for the icons the components draw, the
  `Checkbox`'s and the checked `Radio`'s `icons/check.svg` so far (see "The
  icons are Lucide SVGs served by one `AssetSource`").

### `Root` wraps the window's content (Measured)

`open_window`'s closure builds the app's `Riffle` entity first (it takes the
focus and starts as before) and returns `Root::new(view, window, cx)` as the
window's root view. Nothing reads the window's root by its type:
`on_app_quit`, the bounds saved on quit, `SettingsView`'s `Change::Closed`
focus restore and the tree's keyboard flag hold the view, its focus handle or
the `Window`, so they are unchanged. The trash dialog and the folder
operations' dialog are component dialogs (see "The trash dialog is a
component `Dialog`").

What `Root` adds to every frame, read at 0.9.1: its own `div` (the `Root` key
context and the `Tab` / `TabPrev` / `Copy` handlers), gpui-base's
`TextSelectionLayer` (a zero-size element that paints nothing while no
selectable text is drawn), the component layer's `WindowState` view (an
`absolute` `inset_0` `div` holding the empty sheet, dialog and notification
layers and the tooltip, touch-selection and native-menu fallback overlays)
and `window_border()`, a plain `size_full` `div` unless the window has
client-side decorations (Linux, where it draws a frame and a shadow).
`WindowState`'s `prepare` sets the window's rem to `theme.font_size` on every
frame, so `theme.rs` sets it to 16 px, GPUI's default, the rem the app's
rem-based sizes were drawn at (the menus' rows, the components' `small()`
sizes, the empty state's `0.9rem`); 13 px would have shrunk them all. Its
`style` gives `Root`'s surface `theme.background`, `theme.foreground` and
`.SystemUIFont`, GPUI's default font, under the app view's own black.

- `tab` / `shift-tab`: the app view's root `div` sets the key context
  `Handpic` (`KEY_CONTEXT`), and `main` binds both keys to `NoAction` in it
  (`keymap_keys`) right after `gpui_component::init`. GPUI's keymap
  (`keymap.rs`, `resolve_bindings_for_input`) sorts the matching bindings by
  context depth and drops every binding ranked after a `NoAction` from a
  source of equal or weaker precedence; `Handpic` is deeper than `Root`, so
  with the focus anywhere in the app's view (the settings modal included: a
  `deferred` keeps its parent dispatch node) no action is dispatched and the
  key reaches `key_down` as before. Nothing is stopped, so no typed character
  is lost. A component dialog is drawn in `Root`'s layer, a sibling of the
  app's view, so its context stack has `Root` and no `Handpic`, and `Root`'s
  `Tab` / `TabPrev` walk its focus trap. An `Input` binds the two keys deeper
  still (`IndentInline` / `OutdentInline`), which runs first and propagates
  on a single-line input, so the settings modal's fields move the focus as
  before. `cx.intercept_keystrokes` was not taken: it runs before every
  binding and would have had to tell a dialog's focus apart itself.
- `ctrl-c` / `cmd-c`: `Root`'s `Copy` copies the text selection's text and
  propagates when it is empty. The app draws no selectable text, so the key
  reaches `key_down`; an `Input`'s own `Copy` binding is deeper and copies
  its selection.
- Measured (2026-11-14, release, Windows): a temporary `f9` arm in
  `key_down`, not committed, opened `window.open_dialog` with three
  `tab_index(0)` `div`s and `close_button(false)`; the keys were posted as
  `WM_KEYDOWN` with the modifier state set through `AttachThreadInput` +
  `SetKeyboardState` (a posted Shift or Ctrl alone is not seen: GPUI reads
  the modifiers from the thread's key state). With no dialog, `tab` reached
  `key_down` as `toggleSides`, and `shift+tab` and `ctrl+c` reached it with
  no action. Inside the dialog `tab` dispatched `root::Tab` and walked A, B,
  C, A, `shift+tab` dispatched `root::TabPrev` back to C then B, `escape`
  closed it and gave the focus back to the app's handle, and the next `tab`
  reached `key_down` with no action dispatched.
- Rule: on macOS `Root::new` installs a window hit-test forwarder
  (`install_window_hit_test_forwarder`), not yet seen on this app.
- Rule: a version bump of `gpui-component`, `gpui-base` or `gpui-pre`
  re-checks the `Root` context's bindings (`KeyBinding::new` with
  `Some(CONTEXT)` in `gpui-base`'s `root.rs`), that a `NoAction` in a deeper
  context still masks a shallower binding (`main.rs`'s
  `tab_and_shift_tab_reach_the_keymap_only_inside_the_view` rebuilds that
  case on GPUI's `Keymap`), that dialogs are still drawn outside the app's
  view and a `deferred` keeps its parent dispatch node, what
  `RootPlugin::prepare` / `style` set (the rem above all), and runs the A/B
  gate, since the wrapper is drawn every frame.

### The license notices are due at packaging (Inferred)

`gpui-component` and the `gpui-pre-*` crates are Apache-2.0
(`LICENSE-APACHE`, Copyright Longbridge; GitHub shows NOASSERTION only for the
file name), and `gpui-kit-assets` embeds Lucide (ISC).

- Rule: no notice file while the native app has never shipped; add these
  notices when it is packaged.

### Linux needs system libraries, macOS and Windows do not (Inferred)

Per Zed's `script/linux`: `libfontconfig-dev`, `libwayland-dev`,
`libx11-xcb-dev`, `libxkbcommon-x11-dev`, `libvulkan1`, plus a Vulkan-capable
driver at run time. The `native` CI job's Linux leg installs exactly these.
macOS needs the Xcode CLT (Metal), Windows only MSVC.

- Rule: add a package to the CI job's apt step only when a build error names
  it, and keep this list equal to that step.

### macOS compiles its shaders with `xcrun metal` at build time (Inferred)

`gpui_apple`'s build script runs `xcrun metal` and `xcrun metallib` unless
`gpui_platform`'s `runtime_shaders` feature is on, so an Xcode without the
Metal toolchain fails the build there, not at run time. `macos-latest` is
arm64, where `build.rs` returns before the NASM check.

- Rule: if a macOS build fails at `xcrun metal`, install the toolchain
  (`xcodebuild -downloadComponent MetalToolchain`) rather than turning on
  `runtime_shaders` for every build.

### The `native` CI job runs on all three platforms (Inferred)

It is a matrix over `windows-latest`, `ubuntu-latest` and `macos-latest` with
`fail-fast` off, each leg running the same `mise exec rust@1.98.1 -- cargo
clippy` / `cargo test`; only the Windows leg adds `setup-nasm`'s
`$USERPROFILE/nasm` to `PATH` itself. Nothing under `crates/native/src` is
`cfg(windows)`-only, and `tree.rs`'s path helpers work on strings, so its
`C:\` and UNC test paths do not depend on the host's `std::path`.

- Rule: code reached only under one platform's `cfg` must not fail
  `-D warnings` on the other two; give it a `cfg_attr(..., expect(dead_code))`
  or the same `cfg`.

### Without NASM, mozjpeg silently loses SIMD (Measured)

`mozjpeg-sys`'s build script runs `nasm -v`, prints a `cargo:warning` and
continues without SIMD when that fails, and cargo hides build-script warnings
of non-path dependencies. RAF decode measured about 99 ms without SIMD and
46 ms with it. `crates/native/build.rs` therefore fails the build, and its own
`cargo:warning` is the only place the SIMD state is visible.

### winget's NASM is per-user and not on `PATH` (Hit)

`winget install NASM.NASM` installs to `%LOCALAPPDATA%\bin\NASM`, not
`C:\Program Files\NASM`, and adds nothing to `PATH`.

- Rule: put that directory on `PATH` for every build.

### `mozjpeg-sys` ignores the `NASM` variable (Inferred)

`nasm-rs` honors `NASM` as the binary path, but the `nasm_supported()` probe of
`mozjpeg-sys` 2.2.3 runs plain `nasm` from `PATH`. `build.rs` therefore runs
`nasm -v` from `PATH` only and fails when it does not run, whatever `NASM` holds.

- Rule: put `nasm` on `PATH`; setting `NASM` does not satisfy the build.

## Packaging

Source: [the native-app-icon plan](../plans/_archived/20261114-native-app-icon/plan.md).

### The Windows icon is resource ID 1 of the exe (Measured)

`gpui-pre-windows` 0.3.12's `load_icon` loads `LoadImageW(module, PCWSTR(1),
IMAGE_ICON, ..)` into the window class's `hIcon`, and a failed load silently
leaves the generic icon. `crates/native/build.rs` therefore embeds
`crates/app/icons/icon.ico` with `winresource`'s `set_icon`, which writes ID
`1`, on a Windows target only (`CARGO_CFG_TARGET_OS`; the crate is a
`cfg(windows)` build-dependency), before the NASM check's early returns. The
icon is referenced by path from `CARGO_MANIFEST_DIR`, not a Cargo dependency
on `crates/app`. Explorer shows the exe's first icon group, so the one
resource serves the taskbar, the title bar and Explorer. In the built debug
exe, `FindResourceW` finds `RT_GROUP_ICON` 1, one `RT_MANIFEST` 1 and
`winresource`'s `RT_VERSION` 1, and `LoadImageW(.., 1, IMAGE_ICON, ..)`
succeeds.

- Rule: never set a manifest in the app's resource. `gpui-pre-platform` turns
  on `gpui-pre`'s `windows-manifest` feature, whose build script already
  embeds `1 RT_MANIFEST`; a second one is a duplicate resource at link time.
- Rule: a change to `crates/app/icons/icon.ico` alone does not select the
  native checks in `mise run ci:local`; the PR's `native` job still builds it.

### Linux matches the window to its `.desktop` file by `IDENTIFIER` (Inferred)

The window opens with `app_id: Some(IDENTIFIER)`. Wayland matches a
toplevel's `app_id` to the `.desktop` file's base name, so the file is
`crates/native/packaging/linux/dev.minodisk.handpic.desktop`. On X11,
`gpui-pre-linux`'s `set_app_id` writes `WM_CLASS` as `IDENTIFIER` for both the
instance and the class, so `StartupWMClass` is `IDENTIFIER` too. The default
`set_app_id` is a no-op on Windows and macOS. `Exec=riffle-native` is static,
so `~/.local/bin` must be on the session's `PATH`.

`mise run native:install:linux` builds the release binary and installs it to
`~/.local/bin/riffle-native`, the `.desktop` file to
`~/.local/share/applications/` and `crates/app/icons/128x128@2x.png` (256 x
256 px) to `~/.local/share/icons/hicolor/256x256/apps/dev.minodisk.handpic.png`.

- Rule: in a `mise.toml` `run = """..."""` (a TOML basic string), a `\` at the
  end of a line is TOML's line-ending backslash and swallows the newline. A
  shell line continuation needs `\\`; the install task instead keeps each
  command on one line.
- Rule: the `hicolor` size directory equals the PNG's real pixel size.

### macOS gets `Handpic.app` from a mise task (Inferred)

`mise run native:bundle:macos [profile]` (release by default) builds and lays
out `crates/native/target/<profile>/Handpic.app` with the checked-in
`crates/native/packaging/macos/Info.plist`, `Contents/MacOS/riffle-native` and
`crates/app/icons/icon.icns` as `Contents/Resources/icon.icns`. No signing,
notarization or `.dmg`. The `native` CI job's macOS leg bundles the debug
build and checks it (`plutil -lint`, the executable and the `.icns` present).

- Rule: the bundle identifier and the Linux `app_id` are `IDENTIFIER`,
  shared with the Tauri app, which the native app replaces. `datadir.rs`'s
  tests check that `Info.plist` and the `.desktop` file agree with
  `IDENTIFIER` and the binary's name; change them together.
- Rule: the `native` CI job's macOS leg runs `mise run native:bundle:macos
  debug` itself, so CI exercises the task this section describes. The
  Windows-only NASM `PATH` problem (see "winget's NASM is per-user and not on
  `PATH`") that keeps other steps on `mise exec` does not apply on macOS.
- Pending manual checks, not done here: the Dock / Finder icon of the built
  `.app` on a Mac, and the running window's icon after the install task on a
  Wayland and an X11 session.

## Backend

### The backend names no event and no `AppHandle` (Inferred)

`riffle-backend` serves both apps, so it reports through callbacks:
`watch::Watch::spawn` takes `on_changed`, and `watch::set` / `release_under`
take the `&Watch`. Each app's own thin module owns the event name and payload;
in the Tauri app that is `crates/app/src/watch.rs` with `folder-changed`, and
`crates/app/src/treewatch.rs` with `tree-changed` over
`treewatch::TreeWatch::spawn(on_changed)`, `TreeWatch::set(dirs)` and
`treewatch::with_released(&TreeWatch, ..)`. The trash commands' bodies
(`trash::trash_listed`, `delete_listed`, `undo_run`, `redo_run`) take the
`&Runs` and a `progress(done, total)` callback; the Tauri commands keep the
writer flush, the `Scans` lock and its `SCAN_RUNNING` refusal, the
`trash-progress` emit and the log line around them.
The folder operations' bodies take their state the same way:
`rename::rename_folder` and `datefolder::split_folder` take a
`rename::Context` (the `&Watch`, the `&TreeWatch`, the optional index and the
`&Runs`), `datefolder::plan_of` the optional index, and
`foldersidecars::delete_run` the index and the `&Runs`. None of them checks
for a scan or drains the sidecar writer: the Tauri wrappers flush the writer,
take the `Scans` lock, refuse with `text::SCAN_RUNNING` and hold the lock
across the call, and the native app has to guard the call with its own gate.
The MCP companion's server (`mcp::Mcp`) takes a `Bridge` built over a send
callback and returns each switch's `McpState`; the Tauri app's `mcp.rs` owns
`mcp-request`, the main-window check and the `mcp-state` emit. The server
spawns its tasks with a bare `tokio::spawn`, so `Mcp::switch` and `Mcp::stop`
must run inside a tokio runtime: the Tauri app calls them from async commands,
a task on `tauri::async_runtime` and `tauri::async_runtime::block_on`, which
all enter Tauri's runtime; the native app's `mcp.rs` owns a current-thread
runtime on its one `mcp` thread, which runs the switch, waits for the quit's
signal and runs the stop. `Mcp::with_port` binds another port than
`MCP_PORT`, for the native `--mcp-port`.

The CPU and disk sampler and the AI model's state follow the same seam.
`perf::Sampler::start(on_sample)` takes an
`impl FnMut(Sample) -> Result<(), E: Display>` and has a separate `stop()`
(stopping needs no callback); the callback returns a `Result` so the warn-once
on a failed report stays in the backend (`perf sample report failed: ...`,
target `riffle_backend::perf`), and the Tauri wrapper is only
`app.emit("perf-sample", sample)`. `ai::Ai` is a cloneable `Arc` over the
status, the model path and an `on_state: Fn(AiState) + Send + Sync` given to
`Ai::new`, so the download thread holds a clone of the `Ai` itself and
reports progress and the final state through that callback; `switch` also
returns the state. The `on_state` callback returns nothing because the Tauri
app always ignored `ai-state` emit errors. The download's and the server's
`log` lines now sit under `riffle_backend::ai` / `riffle_backend::mcp`, not
`riffle_app::*`, so grep for the backend target. A native consumer takes these
as they are rather than copying the Tauri wrapper. Source:
`docs/plans/_archived/20261113-gpui-companions/learnings.md`, Steps 2 to 4.

- Rule: never add `tauri`, an event name or an `AppHandle` to `crates/backend`;
  take a callback or the state itself.
- Rule: a backend body that changes a folder on disk leaves the scan check to
  its caller and says so in its doc comment; call it only with the caller's
  refusal of a running scan held across the call.

### The backend is compiled twice (Inferred)

`crates/backend` is a member of the root workspace, and `crates/native`, a
separate workspace with its own `Cargo.lock` and Rust 1.98.1, uses it by path,
as it does `riffle-core`. A dependency added to the backend also lands in
`crates/native/Cargo.lock`, and clippy lints the backend only from the root.

- Rule: a backend change needs both workspaces checked. `mise run ci` runs
  only `lint` and `test` of the root workspace, so it never covers
  `crates/native`. The agents run `mise run ci:local` locally
  (`tools/ci/local.sh`), which checks only the changed crates: for a
  `crates/core` or `crates/backend` change, clippy and the tests of that root
  package plus the native clippy; for a `crates/native` change (the app's
  identifier, `datadir`, `logger`, `settings` and the rest), the native clippy
  and tests. The PR's `test` and `native` jobs, on all three platforms, run
  the rest: the root workspace's other crates and the native tests a backend
  change can break. Neither full task is run locally as a gate.
- Rule: do not cap `ci:local` with `--jobs` and do not reach for the full root
  workspace to be safe. `cargo clippy -p` / `cargo test -p` of `riffle-core`,
  `riffle-backend` or `riffle-cli` never compiles `tauri` or `riffle-app` (their
  graphs are 161 / 317 / 169 crates against 537 for `riffle-app`). The
  largest check's process-tree peak was 5.5 GB (the native test) and a cold
  `crates/core` change peaked at 3.3 GB, so the earlier out-of-memory kills
  came from the full root workspace, not from `ci:local`. The selected root
  packages go to one `cargo clippy` and one `cargo test` call (a `--package`
  each), because three separate cold calls for every check at once took 594 s,
  at the Bash tool's 600 s limit, against 382 s in one call. Cold wall times:
  Markdown or frontend only under 10 s, `crates/cli` 66 s,
  `crates/native` 154 s, `crates/backend` 199 s, `crates/core` 265 s. Source:
  `docs/plans/_archived/20261114-light-local-ci/learnings.md`, Step 1.
- Rule: `#[cfg(test)]` does not cross a crate boundary. When code moves into the
  backend, a helper the app's tests still use (for example `Index::row_flags`)
  must lose its `#[cfg(test)]` or be copied into the app's test module.
- Rule: `mise run native:check` runs clippy and tests but does not build
  `riffle-native.exe`. Run `cargo build` before launching the exe, or you run a
  stale binary. A still-running older `riffle-native.exe` locks
  `target/debug/riffle-native.exe` and `cargo build` fails with `os error 5`;
  rename the running exe aside (for example `riffle-native.stale-<pid>.exe`)
  instead of killing it, or ask the user to close it.
- Rule: `crates/native/Cargo.lock` resolves its own versions (`notify` 8.2.2
  against 8.2.0 in the root lock). `watch::set` (watch, replace, the debounce)
  works on 8.2.2: copying into and deleting from the open folder resynced it.
  The tree watcher's `unwatch` + `settle` returns promptly on 8.2.2 too (see
  "The tree follows the disk through one `TreeWatch`"), and so does the
  folder rename's release path (see "A folder rename releases the watches,
  then follows in the tree and reopens").

### The judgments' Tauri-free pieces live in the backend (Inferred)

The folder-open sidecar pass (`sidecar::reconcile_of`, with `SidecarError`
and `MAX_SIDECAR_BYTES`), the meta pane's formatting (`exif::Metadata` and
`exif::metadata(name, &shot)`), the stored `labelNames` value
(`sidecar::label_names_setting` / `label_names_value`), the metadata and
preview reads (`exif::read_metadata`, `ondemand::read_preview`) and the stored
`mcpEnabled` value (`mcp::mcp_enabled_setting`) moved out of
`crates/app/src/commands.rs` so the native app can call them; the Tauri
commands call them from there. `raw_only` stayed in `crates/app`: the native
app's view-only rule is the frontend's `viewonly.ts`, ported on its own.
The folder tree's listing (`folders::roots(home)`, `list(dir)`,
`REVEAL_LABEL`, `is_hidden`) moved the same way out of
`crates/app/src/folders.rs`; the home directory is the caller's (Tauri's
path resolver there), and the reveal itself is not shared: the Tauri app
keeps the opener plugin, the native app has GPUI's `App::reveal_path`.

- Rule: a Tauri-free helper the native app needs moves into the backend, it is
  not copied, so the two apps cannot drift in what they read and show.

### The settings live in the native app's own store, seeded once (Inferred)

`settings.rs` reads the native app's settings store
(`riffle_backend::store`): one SQLite file, one `settings` table of
`key TEXT PRIMARY KEY` / `value TEXT NOT NULL` (JSON), WAL, the schema
version in `user_version`. It is `--settings <path>`, else
`settings.sqlite` in the `IDENTIFIER` folder under the config dir
(`datadir::settings_path`), next to the Tauri app's `settings.json`. The
values (`sidecarFormat`, `labelNames`, `shortcuts`, `autoAdvance`,
`sortOrder`, `filter`, `lastFolder`, `windowBounds`) go through the one `from_values` with the backend's
rules (`SidecarFormat::from_setting`, `sidecar::label_names_setting`,
`Keymap::from_overrides`, `autoAdvance` on unless a boolean says off,
`SortKey::from_setting`, the Tauri `parse_sort_order` with `recommended` read
as capture order and left stored as it is, `Stored::from_setting`, a string
`lastFolder`, `session::Geometry::from_setting`).

Only a new store (no schema yet: a missing file or a zero-byte one left by a
crash) is seeded, once, with every key of a Tauri `settings.json` verbatim,
the schema and the values committed in one transaction. The seed is
`--import-settings <path>` when given; else, only without `--settings`, the
first that exists of `<config dir>/<IDENTIFIER>/settings.json` and each
`<config dir>/<LEGACY_IDENTIFIERS[i]>/settings.json`; with `--settings` and no
`--import-settings` nothing is seeded and nothing under the config dir is
read. The JSON is only read, never written, and never read again once the
store exists, so the two apps' settings diverge from then on. The log names
the store and, for a new one, how many keys were seeded from which file (or
the paths tried).

The store is never deleted or recreated, unlike the index (a cache): a file
that is not a database, or one with a newer `user_version`, is an error left
as it was, and the app runs on the defaults (`Xmp`, the English names, the
default keymap, auto-advance on) with no store behind `Settings::writes`, so
nothing is written; the startup `settings:` line ends `store=none`.

- Rule: `Store::open` reads `user_version` before it sets `journal_mode = WAL`,
  so a store with a newer schema is refused before its header is touched
  (switching a rollback-journal database to WAL rewrites the header). The
  schema and the seed are then written in one `BEGIN IMMEDIATE` transaction
  that re-reads `user_version`, so two processes opening one new store cannot
  both seed it, and (`PRAGMA user_version` being transactional) a store is
  never found with the table but no version. A JSON file opened with
  `Connection::open` succeeds and fails only at the first query ("file is not
  a database"), leaving its bytes as they were; keep that order if the open
  is ever reworked.

Every write goes through `Settings::writes` (`settings::Writes`): a
`set(key, value)` or `delete(key)` is sent to one `settings-store` thread,
which applies them to the store in the order given, so two quick changes of
one key cannot land reversed, and never on the main thread. A failed write is
logged (`settings store: failed to write <key>`) and the in-memory value
stands, as in the Tauri app. `App::on_app_quit` drains them (`drain`, at most
`settings::DRAIN_TIMEOUT`) in a handler of its own, registered whether or not
the index opened (`quit: settings writes drained=…`). One thread with a
channel was taken over `viewed.rs`'s `Latest`: a `Latest` per key would need
one per key and a way to wait for them all on quit, where the channel's
barrier gives the drain for free.

- Rule: every debug or hand-check run passes `--settings` with a test store
  (a fresh `.sqlite` path in the test folder), and `--import-settings` with a
  test JSON when it needs seeded values, or it opens and seeds the user's real
  store; and `--log` with a file in the test folder, or it appends to the
  native app's own `Handpic.log`.

### The folder open reads the sidecars before pass 1 (Measured)

`scan::list` lists with `read_listing(dir, Some(format))`, and
`Scan::prepare` runs `stat_sidecars` and `sidecar::reconcile_of` right after
`reconcile`, on the open's background task and before the scan thread starts,
as the Tauri `scan_folder` orders them, logging `scan sidecars: dir=…
dirty=N changed=N in …ms`; a view-only folder skips it. The sidecars it could
not read go to the pane's notes as `<name>: <message>`. On a copied folder of
8 DNGs with XMP and `.dop` sidecars and a fresh index, the first open logged
`changed=8` and the second `changed=0`, under `"xmp"` and under `"both"`, and
no sidecar's mtime moved.

- Rule: `Index::entries` joins `files`, so on a fresh index it returns nothing
  until pass 1 has written the files' rows; the view reads the judgments at
  the open and again at pass 1's `Done`. The `ratings` rows exist right after
  `reconcile_of`, so a ratings-only read (`Index::rows_of`) would show the
  marks before pass 1, at the cost of the "entry seen" set Step 4's
  `label_known` follows.

### A judgment goes view, row, writer (Measured)

A judgment key on the strip's selection (none in a view-only folder) applies
`judgment::apply` to the view's map and the strip's cells at once, then on
`cx.background_spawn` runs `judgment::store`: `Index::set_rating` on the
writer connection, then `Writer::set`, the Tauri `set_rating`'s order, so a
crash between the two leaves the row dirty. A failure of either puts the
file's previous state back (while the folder is still open) and goes to the
pane's notes, as does the writer's `on_error` through `Event::SidecarError`,
one line per path. The writer is spawned in `Riffle::new` when the index
opened; `Scan::prepare` hands `reconcile_of`'s dirty rows to `Writer::set_now`,
so a judgment lost to a crash inside the 300 ms debounce is written on the
next open (checked on the real exe by seeding a dirty row into a test index:
the reopen logged `dirty=1` and the `.xmp` got `xmp:Rating="4"`).

- Rule: the view keeps `seen` (the paths the index last returned an entry for,
  which make the label known) apart from `judgments`, and `touched` (the
  paths judged since the open), whose state a later `entries` read, such as
  the one at pass 1's `Done`, does not replace, as the Tauri `touched` set.
- Rule: undo and redo go through the same path. A judgment that writes
  records its files' states before it as one `history::Undo` entry (a batch
  over the selection's targets) and clears the redo history; `Ctrl+Z` /
  `Ctrl+Shift+Z` pop an entry, push the files' current states onto the other
  history, apply the popped states as a key would (view, row, writer) and
  move to the file of a one-file entry, selecting it alone when the focus
  moves (`selection::restore`); an entry of several files leaves the current
  file and notes `Undid N files` / `Redid N files`. Each entry has an id, so a failed store drops its file
  from the entry it belongs to (`Undo::forget`, the Tauri `forgetOnFail`).
  Another folder's open drops the judgment batches of both histories and
  keeps the trash runs; a re-listing keeps them and a step drops the files
  no longer listed.
- Rule: `label_known` for a store follows the Tauri `send` rule: the entry was
  seen, or (for an undo / redo, which has no command) the restored label
  differs from the current one. A `ClearAll` on a file whose entry was not seen
  still goes out when it changes nothing (`forceLabel`), so `judgment.rs` has
  `writes` next to `changed`; do not fold the two into one test. A failed
  undo / redo store also drops its file from the entry it pushed onto the
  other history (the Tauri `step` passes no `onFail` for that entry).
- Rule: `rejectRest` is not a `judgment::Command`: the selection neither
  widens nor narrows it. `burst::reject_rest` builds its changes, every other
  member of the current file's burst over `all` (a member the filter hides
  too) that is not a reject yet, flagged reject with its stars and label kept
  and its label known when `seen`; `judge_paths` records them as one entry
  (the redo history cleared), applies them, refilters once on the current
  file and stores each, with no auto-advance. Nothing happens in a view-only
  folder, with no file shown or outside a burst of two or more; its undo is
  the ordinary `step`.

### The selection is keyed by path and ends every `refilter` (Inferred)

`selection.rs` is `selection.ts` over the native `judgment` types: a
`Selection { selected, anchor }` of listed paths, the current file always a
member. The view holds one; `set_selection` hands its paths to the strip
(`Strip::set_selected`, indices through the strip's `index_of`), which paints
a selected cell other than the current one as `.cell.selected` (the `--muted`
fill and a `--ring` border), so nothing maps paths per frame.

- Rule: `go` (paging, `home` / `end`, auto-advance, a one-file undo) selects
  the landed file alone only when the current file changed, as `main.ts`'s
  `move`; `focus_on` is the move without touching the selection, which a
  Ctrl / Shift click, `extendPrevious` / `extendNext` and the MCP `show`
  use after setting the selection themselves. A Ctrl click (Cmd on macOS;
  either modifier, as `strip.ts` reads `metaKey || ctrlKey`) toggles the cell
  and keeps the focus; a Shift click selects the range from the anchor and
  moves the focus to the clicked cell; a plain click selects it alone, also
  on the current cell. `selectAll` selects every shown file with the focus
  kept. None of them is refused in a view-only folder.
- Rule: every `refilter` that rebuilds `files` ends with `settle(.., false)`,
  that is `prune`, so a file a judgment or the filter hides leaves the
  selection; a new folder's open lands with `settle(.., true)`, the landed
  file alone. A re-listing goes through `refilter` and so prunes.
- Rule: a judgment's targets are `selection::targets` (the focused file alone
  while one or none is selected) and its changes `selection::judgments`: the
  value decided once from the current file, each file kept on the fields the
  command does not touch, the current file first, a file left out unless
  `judgment::writes` holds for it (its `seen` per path). Auto-advance needs
  exactly one target (`should_advance`'s `targets`).

### The writer drains in `App::on_app_quit` (Measured)

`Riffle::new` registers `App::on_app_quit` (the app-level one, holding an
`Arc<Writer>`, not the entity-level `Context::on_app_quit`: closing the window
can release the view before the quit handlers run) and calls
`writer.flush(DRAIN_TIMEOUT)` inside the handler itself, returning a ready
future. GPUI gives the returned futures only `SHUTDOWN_TIMEOUT` (200 ms), but
the handler is called synchronously from `App::shutdown`, which on Windows
runs once the message loop ends, after `cx.quit()` (`q`, `--auto`) and after
`on_window_closed`'s `cx.quit()` alike. An `--auto 1` run logged `quit: sidecar
writer drained=true` after its summary; the window-close path and a key
inside the debounce are the user's hand check.

The same handler first stops the running scan, before the drains: the view
holds its scan in a `scan::Running` (an `Arc<Mutex<Option<Scan>>>`) whose
clone the handler keeps, since the view may be gone by then.
`Scan::stop_within(STOP_TIMEOUT)` sets the cancel flag and waits on a channel
the scan thread drops its sender of when it ends (`JoinHandle` has no timed
join), logging `quit: scan stopped=<bool> in <ms>ms`. Mid-pass 2 on 3,360
hardlinked ARW files (release, 3 pass 2 threads) the stop took 6-10 ms and
the process exited 47-140 ms after the quit, `scan faces: ... canceled=true`
logged, by `--auto 10` and by `CloseMainWindow` alike. `STOP_TIMEOUT` is 2 s,
as `DRAIN_TIMEOUT`, for a read a slow or network volume holds; past it the
process exits with the scan running, as before.

- Rule: a re-listing still stops the previous scan with the unbounded
  `Scan::stop`: it must not reconcile while the old scan writes. A scan being
  stopped that way when the app quits is not in the holder, so the quit does
  not wait for it.

### The MCP companion answers from the view, one channel hop away (Measured)

With `mcpEnabled` true in the settings store, `mcp.rs` builds a tokio
current-thread runtime on one named `mcp` thread, which runs the backend's
`Mcp::switch(true)` (on `MCP_PORT`, 41917, or `--mcp-port`), waits for the
quit handler's signal and runs `Mcp::stop` (logged as `quit: MCP server
stopped=<bool> in <ms>ms`, at most `STOP_WAIT`). With it off or missing there
is no runtime, no thread and no server. A bind failure is logged with the
`mcp: McpState` line and the app runs on. The backend's `Bridge` send callback
only puts the request into the view's channel as `scan::Event::Mcp`; the
view's `mcp_request` builds a `companion::View` snapshot, `companion::answer`
returns the action or the refusal, the view applies it (`focus_on(Move::To(..))`
with the file selected alone,
`toggle_zoom`, or `judge_paths`) and replies through `Mcp::reply`.
`get_photo` and `get_preview` never reach the view: the backend reads them on
tokio's blocking pool against the index's read-only connection, outside
`ReadSlots`, as in the Tauri app.

`set_judgment` goes the judgment keys' way: `companion.rs` merges the given
fields over each photo's own state (an omitted field kept, a rating of 0 and a
null label clearing), keeps only the photos it changes, with the label known
when the entry was seen or the call changed the label (the Tauri `send`
rule), and `judge_paths` records them as one `history::Undo` entry, marks them
`touched`, applies them, refilters once with the current file kept, and stores
each under that entry's id. There is no auto-advance, and a view-only folder
refuses with `viewonly.ts`'s message. The reply lists every given photo's
resulting stars, flag and label, the unchanged ones too. The tool's English
color names are what `judgment::State.label` holds, so no mapping is needed.
A `curl` check on test copies found the stars, the pick and the label in each
`.xmp` and the `ratings` row after `--auto` quit.

Not supported yet, each refused with a message naming the native app: compare
and a `select_photos` with several paths. `selected` is the current file
alone, and `current.eyes` is present only once the `f` overlay has judged the
file. `burst` is the Tauri `burstOf` over the view's `bursts` (empty for a
burst of one, else every member in position order, a hidden one with
`visible: false`); each frame's `sharpness` is `Facts.sharpness`, set from the
entries read and patched by pass 2's `ready` files (`apply_face_ready`), as
the Tauri `sharpness` map is filled from `folder_entries` and patched by
`faces-progress` (`applySharpnessReady`).

- Rule: a test run with MCP on passes `--mcp-port` with a free port other
  than 41917 (the user's Tauri app may hold it), seeds `"mcpEnabled": true`
  through `--import-settings` into a fresh `--settings` store, and passes
  `--log` with a file in the test folder, next to `--index`.
- Rule: on Git Bash, give `curl` the JSON body on stdin
  (`--data-binary @- <<< "$body"`); a `--data` argument with `\` escapes
  reaches the server mangled. The session id is the `Mcp-Session-Id` header of
  the `initialize` reply, and `set_judgment` paths are matched as the index
  keys them (`\\?\`-prefixed on Windows), not canonicalized.

### A folder change re-lists, never mid-scan (Measured)

`main.rs` holds one `watch::Watch` whose callback sends `Event::Changed` into
the scan's channel; each listing calls `watch::set` before `read_listing`, so a
change during the listing is not missed. `resync.rs` defers a change that
arrives while a listing or its scan runs to that scan's end, one pending at
most, as the Tauri frontend's `resyncPending`: a re-listing mid-scan would
cancel and restart it. A re-listing carries the current file by path, the
strip's offset (`carried_offset`) and the loaded thumbnails
(`carried_indices`), bumps a listing counter that drops the previous listing's
preview reads, and always reads the current preview again (it also refreshes
the status line's `n/total`).

- Rule: a file still being copied in changes size between events, so each
  debounced event re-extracts it (`todo=1 removed=1`) until the copy ends;
  that is expected, not a loop.

### The listing and the shown list are two lists, rebuilt by one `refilter` (Measured)

The view keeps `all` (the listing in natural order narrowed by `--size` /
`--max`, the Tauri `allFiles`) apart from `files` (`all` in the sort in force,
later also filtered), which the pager, the strip, the thumbnails, the hot
list, `current_path`, the status line and the pane's `n / total` index into.
The order reads `facts` (each listed file's orientation, capture time, subsec
and `Focus`, keyed by the listed path) and the judgments, both from the one
`Index::entries` read at the open and at pass 1's `Done`. A view-only folder
is in capture order whatever the key (`viewonly::sort_for`), the key kept for
the next RAW folder. On a fresh index the open has no facts, so a capture
order starts as name order and settles at pass 1's `Done`, the current file
kept: an `--auto 5` run on 6 copied ARWs under `"capture"` paged in capture
order from there, `"rating"` put the rated files first and the rejected one
last, and a JPEG folder under `"name"` paged in capture order.

- Rule: only the open and `refilter` rebuild `files`. `refilter` orders `all`,
  keeps what `filter::passes` lets through (the view's judgment, `facts` and
  `mark::af_eye_state` of each path), returns early when the list is
  unchanged (`filter_list_changed`, forced by a re-listing), else carries the
  pager, the strip and the loaded thumbnails onto the new list (`carry`),
  landing on `anchor_after_filter`'s file over the ordered `all`: the anchor
  when it passes, else the next passing file after it, else the last before
  it. It runs after a judgment, an undo / redo, a failed store's revert, pass
  1's `Done` and a re-listing; it never runs per page. An empty result clears
  the image and the pane's file and shows `FILTERED_TEXT` on the status line
  and in the pane.
- Rule: a re-listing passes the list shown before it (the old `files`, not
  the old `all`) as `previous`: an anchor gone from the listing gives way to
  its next surviving passing neighbour in that list, else its previous one.
  This replaced `resync::carried_current`, the same rule with every file
  passing. The open resumes at `anchor_after_filter`'s file for the
  remembered `last_viewed`, so a remembered file the filter hides resumes at
  its passing neighbour.
- Rule: undo / redo's `listed` is over `all`, not `files` (the Tauri
  `allFiles.includes`), so a judgment on a hidden file is still undone. A
  one-file step anchors on its file when it passes, else on the file shown
  before it, and notes `Undid <name> (hidden by the filter)` when the file is
  not in `files`. Auto-advance after a judgment needs the judged file to be
  still current (`judgment::should_advance`): a judgment that filtered it out
  already moved the cursor.
- Measured: with `{"filter":{"flags":["untagged"]}}` an `--auto 6` run on 6
  copied ARWs (one rejected in its XMP) cycled over the 5 others; untagged +
  0 stars + `none` cycled over the 3 unjudged ones; `stars: [3]` (no 3-star
  file) paged nothing; a RAW copied in mid-run re-listed with the current file
  kept and the new file paged in its natord place.
- Rule: a rebuild bumps `order`, which drops a preview read still out for an
  index into the old `files` (the pager is replaced, so its in-flight flag is
  gone), and `refilter` re-requests the current preview whenever the image on
  screen is not the current file's. `listing` still guards the index reads,
  so a judgment's reorder does not drop the entries read of pass 1's `Done`.
- Rule: a judgment's, a failed store's revert and a step's `refilter` scroll the current file into view
  (the Tauri `keepScroll` false); `Done` and a re-listing keep the same files in
  view (`carried_offset`).
- Measured: `order_files` over 1,000 paths takes 0.4-0.5 ms in a release
  test build and 13-17 ms in the debug one (natord dominates the name order).
- Rule: pass 2's `FacesProgress` patches each written file's `Focus` in
  `facts` (`filter::apply_face_ready`, the Tauri `applyFaceReady`: a file
  without facts or without a focus is left alone) and refilters with the
  current file as anchor and the scroll kept only while a `Face focus` or
  `Eyes` item is checked; `FacesDone` reads nothing. On a fresh index under
  `{"filter":{"candidates":["good"]}}` the open's list and pass 1's are empty
  (no `eye_focus` yet, so every file is `Neutral`) and the 3 good frames of 8
  came in from `FacesProgress` alone; `eyes: ["open"]` paged the 4 open-eyed
  frames.
- Rule: the `Output` section's answer is read on the open's (and the
  re-listing's) background task right after `scan::list`, before
  `Scan::prepare` (`scan::outputs` over `output::outputs_in`, at most two
  `read_dir`s), never on the main thread, and handed to `opened` with the
  listing it is about. No export folder or a read error is no answer:
  `FilterState::set_output_known(false)` clears the `outputs` checks, and an
  error is logged as `output filter: …` and kept in the pane's notes as
  `Output filter: …` until a read succeeds. The section is never stored.
- Measured: `output: dir=… found=N in …ms` (`found=none` without an answer)
  logged 0 ms on 8 copied ARWs with a 3-file `Output\`; renaming `Output\`
  away and back re-listed twice, `found=none` then `found=3`.
- Rule: the EXIF sections' groups (`exif::groups_of`, per group its labels in
  display order) are built from the same `Index::entries` rows on the same
  background task as `facts` (`entries_of`), in the index's order so a label
  two files share takes the last one's order as in Tauri, never per page.
  `set_entries` stores them and prunes the EXIF checks against them
  (`FilterState::prune_exif`) before any `refilter` that follows, so a check
  whose label left the folder cannot empty the strip. Another folder's open
  clears the EXIF checks (`clear_exif`), a re-listing only prunes; they are
  never stored. `passes` reads a file's EXIF from its `facts`
  (`exif::exif_key`), a file without one failing a non-empty group.
- Measured: `groups_of` over 5,000 synthetic entries (six groups, a few dozen
  labels) took 5.8 ms in a debug test build.
- Rule: the pane's `n / total` is the pager's current index over `files`
  (`meta::position`, the Tauri `positionEl`), not the shown file's index, so
  a refilter that hides the file on screen (`p` under `untagged`) moves it at
  once; the name still names the file on screen until the next preview
  arrives. The shown file's index becomes `None` when a refilter hides it
  (`filter::shown_after`): a stale index would name the file then at that
  place in the next `refilter`, which could skip the re-request of a preview
  its `order` bump had just dropped. The status line's `n/total` is left
  alone.
- Rule: a `refilter` that empties `files` keeps its anchor
  (`filter::KeptAnchor`), a non-empty one or another folder's open drops it.
  A menu change, and only a menu change, anchors on it while no file is
  current, so `Reset` or unchecking the item lands on the file shown before
  the list emptied, else on its `anchor_after_filter` neighbour, else on the
  first file. This is a native improvement: the Tauri `refilter` lands on the
  first file there.

### The bursts are regrouped with the entries and marked with the judgments (Inferred)

`burst.rs` ports `burst.ts`: a burst is consecutive frames in capture order
no more than `BURST_GAP_MS` (1000 ms) apart, a file without a usable capture
time a burst of its own. The view keeps the open folder's grouping in
`bursts`, keyed by the listed path, as the Tauri `bursts` map.

- Rule: regroup only in `set_entries`, over `all` with the capture time and
  subsec of `facts`, through `sort::order_files(SortKey::Capture, ..)` (no
  second capture order). Both callers (the open and pass 1's entries read)
  have set `all` first, and another folder's open replaces the grouping
  there, which stands in for the Tauri reset. `refilter`, a page, a judgment
  and a trash undo's `all.extend` do not regroup: as in the Tauri app the
  grouping is stale until the next entries read, a file not grouped yet being
  its own burst for the moves.
- Rule: the strip's per-cell marks (`burst_marks` over `files`) are set in
  `mark_strip` next to the judgments, so the open, every rebuilding
  `refilter` and an entries read that leaves the list unchanged all re-mark
  the cells. `Strip::new` drops them, so the marks follow every rebuild.
- Rule: `burstPrevious` / `burstNext` and `burstFramePrevious` /
  `burstFrameNext` are `move_burst`: `burst_step` / `burst_frame_step` over
  the shown files' ids (the burst id, else `-1 - at`), then
  `go(Move::To(..))`, which selects the landed file alone and starts the
  preview latency at the key. They run in a view-only folder too, and
  `tree_gate` swallows them while the tree has the keyboard.
- Rule: `paint_strip` paints every on-screen band first, masked to the
  strip, then the cells, so a band reaching across the gap never covers the
  neighbour's outline. The band is `.cell.burst::before`: `rgba(255, 255,
  255, 0.08)` from the cell's padding box 5 px above and below and 1 px
  right, on a member after another left across the 8 px gap, 2 px out with
  a 6 px radius at the band's first and last cell. `STRIP_PADDING` is 7 px,
  the Tauri strip's, room for the band. The `position/size` badge is shaped
  text at the image box's bottom-right, 2 px in, in `--foreground` over the
  eight dark copies of `overlay::HALO`, painted before the rejected dim so a
  rejected member's badge dims with its cell, and a rejected member's band
  is drawn at the dimmed alpha (`BAND_COLOR_DIMMED`, 0.08 x 0.45) as
  `.cell.rejected`'s opacity covers the `::before`.
- Deliberate difference: a selected member's band is painted before the
  cell's opaque `--muted` fill, so inside the cell the fill covers it; Tauri
  paints the 8% tint over `--muted` (`isolation: isolate`).
- Deliberate difference: `burst.ts` builds the time with `Date.UTC`, which
  rolls a month 00, a day 32 or an all-zero timestamp over into a finite
  time, so such files can share a burst in the Tauri app. `capture_ms` parses
  strictly (month, the month's days, 23:59:59), and such a file has no
  capture time, a burst of its own.

### The sort and filter menus are deferred popovers over the window (Inferred)

The status row is a flex row: the status text, then `Sort ▾` and `Filter ▾`
at its right end (`menu.rs` holds the rows, the clicks' `apply` and `Menus`;
`main.rs` only maps rows to `div`s). The filter button is lit while
`FilterState::is_active`; the sort button is dimmed and has no handler in a
view-only folder, whose open closes a sort menu left open. A menu item runs
`menu::apply`, then `refilter` on the current file with the scroll not kept,
so a reorder scrolls the current file into view. `menu_action` then writes
`sortOrder` (`SortKey::setting`) when the sort key changed, and `filter`
(`FilterState::stored`, then `Stored::setting`: the remembered sections, each
in the menu's item order except `stars`, which stays ascending (0 to 5)
whatever the menu's order, never `Output` or the EXIF checks) when a remembered
section changed, `Reset` included; a change of `Output` or EXIF alone writes
nothing. Nothing is written at load, so a seeded `recommended` stays stored
until the user picks an order, and a view-only folder, whose sort button is
dimmed, shows capture order and keeps the stored key.

- Rule: every filter section starts with a `Row::Heading` right after its
  separator (no hover, no click handler, no check slot); `Reset` sits above
  the first separator without one. Four headings are the native menu's own,
  as the Tauri `#filter-menu` has none there: `Flag`, `Rating`,
  `Color label` and `Orientation`. The sort menu has no heading.
- Rule: the leads are drawn per kind in `main.rs`'s `menu_row`: the flags
  keep their 7 px dots; a rating row is five 12 px Lucide stars, the `on`
  filled ones in `STARS_COLOR` and the rest outlined in `MUTED_COLOR` at
  40 %; a color label a filled Lucide `square` in `tint_color`, `No label`
  the outline; `Portrait` and `Landscape` the Lucide `rectangle-vertical` and
  `rectangle-horizontal`; `Face focus` the Tauri icons (`scan-face` for
  `Good`, its frown derivative for `Low`, in `MarkState::color()`) with an
  empty 12 px slot for `Neutral`. `menu.rs` names the kind (`Lead`), never an
  asset path.
- Rule: the rating rows run 5, 4, 3, 2, 1, 0 stars from the top, the labels
  are squares and the orientations have icons, all unlike the Tauri menu on
  purpose (the user's decision); a later port must not "fix" them back.
- Rule: the filter menu ends with the folder's EXIF sections, after `Output`
  (or `Eyes` without an export answer), in the Tauri order and headings
  (`Camera`, `Lens`, `Aperture`, `Shutter speed`, `ISO`, `Focal length`),
  each a separator, a heading and one item per label, a group the folder
  lacks left out. Numbers sort by value (`f64::total_cmp`), ties by label;
  `Camera` and `Lens` by a case-insensitive compare, ties by the plain one
  (Tauri uses `localeCompare`; no collator here). The focal length is
  bucketed into `exif::FOCAL_RANGES`, half-open. An item is keyed by its
  group and label (`Toggle::Exif`), not its row, since the groups can be
  rebuilt between the frame that drew the menu and the click, which made
  `Row`, `Item`, `Label` (with `Label::Owned`), `Toggle` and `Action` `Clone`
  rather than `Copy`. The longer menu relies on the popover's scroll.

- Rule: an open menu is `div().absolute().top_0().right_0()` inside a
  `relative()` wrapper of its button, holding `deferred(anchored()
  .anchor(Anchor::BottomRight).offset(0, -4).snap_to_window())`: `anchored`
  without `position` places itself where it is laid out (here the button's
  top-right corner), so no bounds need reading back, and `deferred` paints it
  after everything else. The popover is `occlude()`d, so the strip's
  `on_mouse_down`, which checks `hitbox.is_hovered`, does not see a press
  inside it, and its height is capped at the window's less 40 px: its row
  list (`main.rs`'s `menu_surface`, shared with the right-click menu) is
  `overflow_y_scroll` with `track_scroll` on a `ScrollHandle` the view keeps
  (`menu_scroll`, new on every press of a menu's button, so a menu opens at
  its top), and the popover carries gpui-component's scrollbar over it
  (`ScrollableElement::vertical_scrollbar`), as `PopupMenu` lays out its own.
  The scrollbar is a child of the popover, so a press on it is not one
  outside (`on_mouse_down_out`). Nothing is laid out while both menus are
  closed.
- Rule: the look comes from gpui-component's theme (`cx.theme()`), not the
  `u32` constants: the popover's `popover` fill, `popover_foreground` text,
  1 px `border` and `radius`; the headings in `muted_foreground`; the
  separators `PopupMenu`'s (a 2 px `border` line across the list's padding);
  the rows `PopupMenu`'s item at its default size (26 px tall, 8 px side
  padding, the theme's radius, `accent` / `accent_foreground` on hover, a
  12 px column with the component's `IconName::Check` for a checked item;
  `menu_item`), the text kept at 13 px. The leads keep their own colors.
  `PopupMenu`, `Popover` and `ContextMenu` are not used: they take the GPUI
  focus (the window routes every key through its own handler on its one
  focus handle), and `PopupMenu::activate` always dismisses the menu, where
  a Sort / Filter item click applies and leaves it open. Their row element
  (`MenuItemElement`) is `pub(crate)`, so the rows are this app's `div`s
  styled the same.
- Rule: `on_mouse_down_out` (a capture-phase listener, run before any
  bubble-phase `on_mouse_down`) on the popover closes it on any press outside,
  including one on its own button, whose bubble-phase `on_mouse_down` then
  runs for the same press. `Menus` keeps the closed menu and the press's
  position, so that button press does not reopen it (the Tauri toggle sits
  inside `#sort` / `#filter`, which its `mousedown` listener spares). The
  buttons toggle on `on_mouse_down`, not `on_click`, for that reason; items
  use `on_click`.
- Rule: `escape` closes an open menu, taken in the key handler before the
  keymap's actions like `home` / `end` / `q`, and only while a menu is open,
  so a user binding on `escape` still works otherwise (no default binding
  uses it). Every other key goes through the window's handler as before, so
  paging, `e`, `z`, `f` and the burst keys work with a menu open.
- Measured: the preview canvas stays 1380 x 852 logical px in the default
  1600 x 1000 window with the two buttons in the status row (12 px text),
  the same as the plain status line, so the fit scale is unchanged.
- Not verified headless (mouse clicks cannot be driven by `--auto`): that a
  press on an item reaches its `on_click` and not the strip, and that
  `on_mouse_down_out` fires for a press in the strip or the preview. Both are
  read from GPUI's `div.rs` at the pinned rev; the hand check is pending in
  `../plans/_archived/20261113-gpui-filter-sort/learnings.md`.

### The icons are Lucide SVGs served by one `AssetSource` (Inferred)

`svg().path(p)` loads `p` through the application's `AssetSource` and paints
the rendered alpha mask in the element's `text_color`, so a stroked Lucide
icon and a filled derivative both draw in one color. `icons.rs` holds the
SVG strings as constants, a path constant per icon (`icons/<name>.svg`), and
`Icons`, whose `load` answers those paths with borrowed bytes and any other
from `gpui_kit_assets::Assets`, a path in neither with `None`; `main()`
registers it with `application().with_assets(icons::Icons)`.

- Rule: to add an icon, copy it verbatim from `lucide-static` at the version
  the header names (v1.48.0, as `crates/app/ui/src/icons.ts`), add its SVG
  and path constants and its entry in `SVGS`, and name it in the header
  comment, with any derivative (a filled shape, the `scan-face` frown) and
  any Feather-derived icon (the list in `LICENSE-lucide`). The full notice is
  `crates/native/LICENSE-lucide`, a copy of the Tauri app's, not a reference
  into `crates/app`.
- Rule: GPUI keeps one `AssetSource` per application. Another source (here
  `gpui_kit_assets::Assets`) is composed with `Icons`, the own paths first
  (`icons/star.svg` and `icons/square.svg` are in both; the app's win),
  never registered in its place: a missing asset draws nothing and fails no
  build. `icons.rs`'s tests check every own path loads an SVG, the own path
  wins, `icons/check.svg` loads from `Assets` and an unknown path loads
  nothing.

### The settings modal is its own entity, opened on `ctrl+,` (Inferred)

`settingsview.rs`'s `SettingsView` is an `Entity` the window holds and adds as
its root's last child; `settingsmodal.rs` is its pure model (the open state,
the selected tab, the shortcut row capturing a key, `modal.ts`'s key rule,
`tabs.ts`'s vertical `nextTab`). While closed its
`render` returns `Empty`, one zero-size layout node. While open it is
`deferred(div().absolute().top_0().left_0().size_full())`: in taffy an
absolute child is placed against its parent, here the window's root, so it
covers the window, and `deferred` paints it after everything else. The
backdrop and the box are both `occlude()`d: the backdrop's `on_mouse_down`
closes it (the box blocks the backdrop's hitbox, so a press inside the box
does not), and the strip's `on_mouse_down` and `on_scroll`, the menus'
buttons and the root's focus-on-press all check their hitbox, which
`BlockMouse` drops from the hit test (`window.rs`'s `hit_test` stops at the
first blocking hitbox, for scroll too). The tabs are Sidecar, Culling,
Keyboard Shortcuts and Import (no Cache, MCP or Debug); the modal keeps the
tab across opens and focuses it on open, as the Tauri `open` does.

A change is emitted as a `settingsview::Change`; the window applies it in its
`subscribe_in` callback (the main view stays the owner of the values):
`AutoAdvance` sets `settings.auto_advance` and writes `autoAdvance`,
`Keymap` replaces the keymap (next section),
`Closed` focuses the window's handle again.
The status line under the box shows `NOT_SAVED` while the settings store did
not open.

- Rule: the key routing. GPUI dispatches a `KeyDownEvent` along the path from
  the root to the focused node, capture phase root first, then bubble from the
  focused node up (`dispatch_key_down_up_event`). The box has its own focus
  handle (`track_focus`) and an `on_key_down` that calls
  `cx.stop_propagation()` on every key, so a key with the focus inside the
  modal never reaches the root's handler; the root's `key_down` also returns
  at once while the modal is open, for a focus left on the root. `ctrl+,`
  (`meta+,` on macOS, `settingsmodal::OPEN_KEY`) is taken literally after
  `home` / `end` / `q`, closes an open sort or filter menu, and opens the
  modal; the keymap refuses to bind it.
- Rule: Tab and Shift+Tab are not built in. The box's handler calls
  `window.focus_next` / `focus_prev`, which walk the whole window's rendered
  tab stops and wrap; the modal's controls are the only tab stops (the root's
  handle comes from `cx.focus_handle()`, `tab_stop` false), so the cycle stays
  inside the modal. The controls are gpui-component's `Button` (the outlined
  `small_button`s: `Choose…`, the example patterns, the languages, `Reset`,
  `+`, a chip's `×`, `Reset all`; the ghost close `×`), `Checkbox`
  (auto-advance) and `Radio` (the three formats, standalone, not a
  `RadioGroup`, whose arrow-key roving would change the cycle), each a tab
  stop at the default `tab_index(0)` with a focus handle of its own (keyed
  state, or `track_focus`), and the `Input` fields. The tabs stay hand-drawn
  `div`s on the theme's tokens: each tracks a handle of the view's with
  `tab_stop(selected)`, the Tauri roving `tabindex`, so only the selected tab
  is in the cycle and ArrowUp / ArrowDown / Home / End on it select and focus
  another (`next_tab`). `TabBar` is horizontal only.
- Rule: Enter and Space on a focused control come for free: the Base
  `Button`, `Checkbox` and `Radio` under the components are `div`s with
  `on_click`, which turn a key down then up of either into a
  `ClickEvent::Keyboard` (`div.rs`), and their listener runs before the box's
  bubble-phase handler stops the event.
- Rule: a pointer press on a component control does not move the focus (each
  calls `window.prevent_default()` on mouse down, which also keeps the box's
  focus-on-press from running). So a control pressed while a field has the
  focus calls `leave_field` first, which focuses the box, and the field
  commits on its `Blur` as it did when the press focused the old control.
- Rule: the look. The outlined buttons are `Button`'s default variant at
  `.small()` (24 px tall) with `px(6.)`, `rounded_sm` and a 13 px label
  child (the variant's own label is `text_sm`, 14 px at the window's 16 px
  rem, and its content style is crate-private), on the mapped `button` fill
  and `input` border; the chosen language is `.selected(true)`
  (`button_active`) with the `PRIMARY_COLOR` border. `Checkbox` and `Radio`
  are `.small()` (a 14 px indicator) with a 13 px `text_size`; a checked
  `Radio` is the component's filled circle with the check mark, not the old
  ring and dot. The focus ring is the theme's: `focus_ring` is on in
  `Default Dark`, so a focused control gets the `ring` border and a ring
  outside it.
- Rule: the Import tab's and the Keyboard Shortcuts tab's lists are
  `overflow_y_scrollbar()` (`ScrollableElement`), which wraps the list in a
  root carrying its size and an inner scroll area, with the theme's
  scrollbar in its default `ScrollbarMode::Scrolling` (shown while
  scrolling, fading after). Nothing scrolls a focused row into view, before
  or after.
- Rule: no AI toggle. The user decided on 2026-11-14 not to port the AI
  features (`aiEnabled` with the model download, pass 3, genres, the ranking,
  `recommended`); a seeded `aiEnabled` key stays in the store, unread.
- Not verified headless (mouse clicks and the modal's keys cannot be driven
  by `--auto`): the clicks, the occlusion, the focus cycle and the keyboard
  click. All are read from GPUI's `window.rs`, `div.rs` and `tab_stop.rs` at
  the pinned rev; the hand check is pending in
  `../plans/20261113-gpui-settings/learnings.md`.

### The keymap is edited in the modal and replaced whole (Inferred)

The Keyboard Shortcuts tab lists `Keymap::bindings()` in order, labeled by
`settingsmodal::shortcut_label` (`settings.ts`'s `shortcutLabels`), each key a
chip (`display_key`: `space` as `Space`) with a `×`, then `+` or `Press a
key...`, then `Reset`; `Reset all` is under the scrolling list. The view
edits its own copy of the keymap, read from the window on every open, through
`settingsmodal::edit` (`Keymap::add` / `remove` / `reset` / `reset_all`); a
refusal shows the backend's message on the status line and leaves the copy
as it was, a success emits `Change::Keymap`, on which the window replaces
`settings.keymap` and `bindings` (`Bindings::new`) and writes
`Keymap::overrides()` under `shortcuts`, deleting the key when there are none
(`Writes::shortcuts`, the Tauri `update_keymap` rule).

- Rule: the capture sees the raw key in the box's handler. `+` sets
  `capturing` and focuses the box itself, since the pressed `+` is replaced by
  the prompt and a focus left on a node no longer drawn would send the key to
  the window's root, whose handler returns while the modal is open; a `×`
  focuses the box after its edit for the same reason. The next named key
  (`key_name`, a lone modifier ignored) is added, Escape cancels, and a tab
  switch or a close cancels (`SettingsModal::select` / `close`).
- Rule: `settingsmodal::APP_KEYS` (`home`, `end`, `q`) are refused with
  `"<key>" is taken by this app`: `key_down` takes them before the keymap, so
  a binding would never fire. `escape` cannot be captured (it cancels) and
  `ctrl+,` / `meta+,` the backend refuses as a menu accelerator. The folder
  tree's keys (arrows, Home / End, Enter, Escape, type-ahead) are not
  refused: they are taken only while the tree has the keyboard, when the
  keymap's photo actions are swallowed anyway (`treekeys.rs`).
- Not verified headless: the clicks, the capture and the names a shifted
  digit and Ctrl+Alt+digit arrive under ("Every keystroke goes through the
  keymap"); the hand check is pending in
  `../plans/20261113-gpui-settings/learnings.md`.

### A sidecar format switch is the Tauri order, shared (Inferred)

The Sidecar tab's three rows (`settingsmodal::FORMATS`) emit
`Change::SidecarFormat` only for a format other than the one shown
(`format_change`); the window's `switch_format` sets `settings.format` at
once, so the next judgment, from a key or the MCP companion's
`set_judgment` alike, is queued in the new format. Then, on the background
executor and under the view's `format_switch` lock (one switch at a time), it first checks
its format against `format_latest`, the one last asked for, written on the main
thread, and does nothing when a later click superseded it, so back-to-back
switches end persisted as the radio shows (the Tauri `AppSwitchLock` gets the
same by re-checking the current format inside the lock). Otherwise it drains the judgment stores
(`judgment::Queue::drain`: a store queued before the switch carries the old
format and must reach the writer before its flush) and runs the backend's
`sidecar::switch_to`: the writer's flush (`sidecar::DRAIN_TIMEOUT`),
`sidecarFormat` persisted through `Settings::writes`, `Index::reset_sidecars`.
The Tauri `switch_format` calls the same `switch_to` after its own state
write, so the two apps share one copy of that order. Back on the main thread
it clears `touched`, the undo histories and the step note, as the Tauri
reopen (`openDirectory`) clears `touched` and the judgment entries of its
histories, and re-lists the open folder with `relist`, whose
`sidecar::reconcile_of` runs only after the reset. A failure goes to the
modal's status line (`SettingsView::failed`) and, as in the Tauri app, the
folder is not re-listed.

- Rule: a listing already under way when the format changes reconciles with
  the format it captured at its start; the switch's own re-listing comes
  after the reset and supersedes it (`opens` token), so the view ends on the
  new format's judgments.
- Rule: a label names change takes the same path (`switch_names`, both
  through `reset_sidecars` under the `format_switch` lock). While the format
  writes XMP the Sidecar tab shows the five name fields
  (`Input`), a row of `i18n::presets()` languages and `Reset`; a
  field's commit (Enter or blur; Escape commits and closes, as the Tauri
  field's `change` fires when the closing dialog takes its focus) or `Reset`
  normalizes the five with `sidecar::label_names_setting`, shows them back
  and emits `Change::LabelNames` only when they differ. The window then
  writes `labelNames` and puts the names in force on the main thread (a
  judgment queued before keeps the names it carries; overlapping changes end
  stored as the fields show, so no `format_latest`-style guard), and off it
  drains the stores and runs `sidecar::switch_to` with nothing left to
  persist, then re-lists.

### The Import tab stores a root and a valid pattern, nothing reads them yet (Inferred)

`Settings` reads `importRoot` and `importPattern` with the backend's
`import::root_setting` / `pattern_setting` (an invalid stored pattern reads as
`%Y/%F`). The Import tab shows the root read-only with `Choose…`, the
`importsettings.ts` texts (`settingsmodal::IMPORT_NOTE`, `root_note`,
`IMPORT_PATTERN_EXAMPLES`, `IMPORT_SPECIFIERS`), the pattern in a
`Input` field and the example line, `import::today_example` (the
native crate has no `chrono`), recomputed on the field's every
`InputEvent::Change` (and after an example button's `set_value`), so with no
debounce; a refusal shows in the error color. A
commit (Enter, blur, Escape then close) or an example button goes through
`settingsmodal::pattern_change`: a valid new pattern emits
`Change::ImportPattern`, which the window writes and keeps in `settings`; an
invalid one leaves the stored one and shows its reason.

- Rule: `Choose…` calls `cx.prompt_for_paths` (directories only) from the
  view and awaits it in a `cx.spawn`; the answer comes on a later turn and the
  view entity lives as long as the window, so `Change::ImportRoot` is emitted
  and written whether or not the modal is still open. It sets only the root,
  so a pattern still being typed stays in the field. A dialog error (Linux)
  goes to the status line.
- Not verified headless: the OS dialog, the typing and the buttons; the hand
  check is pending in `../plans/20261113-gpui-settings/learnings.md`.
  `Import…` itself is port Step 8's.

### The native app keeps its own index (Inferred)

The index lives at `<dirs::cache_dir()>/dev.minodisk.handpic.native/index.sqlite`,
apart from the Tauri app's `dev.minodisk.handpic/index.sqlite`; `--index <path>`
overrides it. The folder is named after the Handpic identifier
(`datadir.rs`'s `IDENTIFIER`, next to `LEGACY_IDENTIFIERS`) and nothing is
migrated from the earlier `com.minodisk.riffle.native` or
`com.minodisk.handpic.native`: the native app never shipped, so an old
developer cache is simply rebuilt by pass 1. Sharing one file would put two processes on it (the Tauri
launch's `VACUUM` against a native scan's writes), make two
`EXTRACTOR_VERSION`s re-extract each other's rows, and let a native run's
`reconcile` drop the user's rows. The cost is a cold pass 1 on each folder's
first native open. An index that cannot be opened leaves the pager working
with `no index: …` on the status line.

- Rule: never point a debug or measurement run at the Tauri app's index; use
  `--index` with a fresh file.
- Rule: `reconcile` drops every row of the folder it was not given, so it
  always gets the whole listing; `--size` / `--max` narrow only what is shown.
- Rule: on Windows `canonicalize` gives `\\?\D:\...`, and every index key and
  listed path in the native app carries that prefix, as in the Tauri app. Key
  anything compared with the index or the scan's `ready` paths (strip cells,
  thumbnails, the watcher's owner string) on `FileStat.path.to_string_lossy()`
  or the canonical `PathBuf`, never on a path rebuilt from the user's argument.

### `run_scan` runs on a `std::thread` (Inferred)

`index::run_scan` blocks for the whole pass and has its own rayon pool. On
`cx.background_spawn` it would hold one of GPUI's background threads, which
the preview decode needs, for as long as the folder takes. `scan.rs` spawns a
thread per scan; a new open cancels and joins the previous one, and the first
scan joins the launch's eviction thread before it reconciles, so the
`VACUUM` never races it.

### The hot list is set before the scan's workers start (Measured)

`run_scan`'s workers take the first file of the hot list not yet taken, else
the next in listing order. With the hot list sent right after `Scan::start`
(as the Tauri frontend does after `start_scan`), the first wave of the 22
workers had already taken files 0-10 of a 373-file folder opened at file 199;
with `ScanFocus::set` called on the handle before `Scan::start`, the first
committed batch was the 20 files of the strip's range around 199 plus 0 and 1
(the two workers left over). Later updates are debounced by
`SCAN_FOCUS_DELAY` (a `cx.spawn` timer task replaced on each change; dropping
a GPUI `Task` cancels it).

- Rule: an index write from the view (`set_last_viewed`, `mark_viewed`) goes
  through the writer on `cx.background_spawn`, one in flight (`viewed.rs`);
  a failure is logged, never shown.
- Rule: `Index::evict` drops a folder whose `opened_at` is over 30 days old
  with its `last_viewed`; seeding a test index by hand needs a current
  `opened_at`.

### A preview holds the scan's pause until its image is painted (Measured)

`load` takes `ReadSlots::pause` before the read and hands the guard back in
`Loaded`; the view keeps it in `PaintHold` (`hold.rs`) and the preview
canvas's paint releases it right after painting the image. Pass 2's workers
wait out a held pause at every stage boundary (`scan::yield_to_pause`), not
only before a read, so the read, the decode, the orient and the paint get the
cores. `MAX_PAUSE` (1 s) still bounds what a worker honours: one budget for all of a
pass 2 file's stage waits, and one per `acquire` (two per pass 2 file, one per
pass 1 file), so a hold never released (no frame painted, say a minimized
window) costs a worker about 3 s per pass 2 file or 1 s per pass 1 file at
worst and only slows the scan. The longer hold also blocks pass 1's `acquire`
from the read through the decode, orient and paint; its effect on pass 1's
throughput was not measured (the pass 1 paging measurement below assumed a
pause around the read only). Holding the pause through the decode (not
yet the paint) cut RAF's A - B from +5.08 to +1.94 ms at 3 pass 2 threads, the
`decode` growth gone, without fewer pass 2 rows per run
([results](../plans/_archived/20261110-gpui-focus-marks/results.md)).

- Rule: anything new on the preview's path between the read and the paint
  runs while the pause is held; keep it there rather than after the release.

### A held key draws each stale preview before chasing the current file (Inferred)

The pager keeps one preview read in flight. When one read + decode outlasts
the key repeat, every result during a hold is another file's; throwing those
away left the fit view blank until the key was released. `Pager::finish`
now decides what a result leads to: the current file's is shown (or its
failure) and clears the in-flight marker; another file's is drawn when its
read succeeded and the current file's image is not already on screen (A shown,
press to B, back to A before B arrives: drawing B would flash the wrong file),
and then the current index is requested, staying in flight. A stale failure is
neither shown nor printed. The drawn frame therefore lags the cursor by at most
one load, the files in between are still skipped, and the number of reads per
key sequence is unchanged.

A stale frame sets the status line, `self.shown`, the image, `marks.show` and
the paint hold, and nothing else. `self.shown` names the file whose image is on
screen because `refilter` rewrites the status line's `n/total` from it and
re-requests when it is not the current file, and the meta pane follows it.
`marks.show` follows the image too: `Marks::mark`, `wanted_faces` and
`wanted_eyes` draw and ask only when the shown file is the current one, so a
stale frame gets no `f` overlay; left at the previous file, stepping back onto
it while a load is out would put its marks on the stale image. `zoom.shown` is
the current file's only, since it stamps the facts with the current page's
`seq`, so `zoom.preview()` stays `None` for a stale frame and the 1:1 canvas
never uses it as the placeholder. The fit view's peaking layer is keyed by the
image id and drawn size, so a stale frame gets its own edges or none.

- Rule: the on-demand analysis (`analyze_current`), the dwell and viewed mark
  (`start_dwell`), the keypress (`self.key`) and the latency `page` line
  (`self.pending`) are the current file's result's only. A `page` line is
  still recorded only for the current file, so the "4-6 pages in 5 s" of a
  held RAF key below counts current-file pages, not frames drawn.
- Rule: stepping back onto the file whose stale frame is on screen still
  reloads it; its analysis, dwell and `page` line wait for that reload. A
  prefetch ring hit is not a stale frame: it has had all of them, so
  `Pager::hit` makes the older result still out `Finished::Dropped` (released,
  nothing requested, nothing left in flight) until the current file changes.
- Pending: a held key flipping frames and the release showing the current file
  with its overlays is unverified by hand on Windows.

### A held key draws each stale crop before chasing the current one (Inferred)

The 1:1 view's crop chain has the same shape: one read in flight, and a crop
read for an earlier page (`seq`) used to be released. `Zoom` now records the
page each held crop was read for. While zoomed, `page()` keeps the held crop
on screen and asks for the new page's (once, while nothing is in flight); a
stale crop that arrives is held in place of the previous one and the current
page is asked for, so the drawn crop lags the cursor by at most one crop read.
A stale failed read holds nothing new, shows no error and still asks for the
current page; the current page's failure shows its error and releases a held
crop of another page. Not zoomed, a page and a stale arrival release the crop,
so `z` never opens on another file's crop.

- Rule: the canvas never mixes two files. `zoom_paint` draws the held crop
  whatever page it is for, but the placeholder (and the `full` size that
  scales it) only when `Zoom::placeholder_fits`: no crop held, or the held
  one is the current page's. The crop's peaking layer is keyed by its own
  image.
- Rule: the `zoom` timing line, the resize re-cut and the reuse of a held crop
  on `z` or a settled viewport (`toggle`, `settled`) are the current page's
  crop's only; a stale held crop never counts as already having it.
- Rule: a single press while zoomed goes old crop, new crop (no black frame or
  placeholder in between), and a folder switch or re-listing while zoomed
  keeps the old crop until the new one arrives.
- Pending: a held key flipping crops while zoomed, and a single press and `z`
  off / on still behaving, is unverified by hand on Windows.

### The prefetch ring holds the decoded neighbours (Inferred)

`prefetch.rs` ports `prefetch.ts`: `Ring` holds the decoded previews of the
current file's two neighbours (the next one in the paging direction first,
none across the pager's wrap), at most `PREFETCH_PIXEL_CAP` (32 MP, 128 MB of
BGRA) in all, the image on screen not counted. `request` takes the current
file from the ring before it reads; a hit does everything `loaded`'s current
arm does (`show_current`: status line, meta pane, `marks.show`, `zoom.shown`,
`analyze_current`, `start_dwell`, the `page` line with `source=prefetch`) but
the read and the paint hold. `request_prefetch` reads the next wanted file,
one at a time through `load` on the background executor, only when the pager
has nothing in flight, the image on screen is the current file's and no scan
pass runs; it is asked after a current result (read or hit), a settled
prefetch, a `refilter` that leaves the current file shown and `FacesDone`.
`--no-prefetch` turns the ring off.

- Rule: an image is owned by one place at a time: the ring, or the screen.
  `take` hands it to the view; `present` hands the image it replaces back
  through `keep`, and only after another image replaced it. Only what a ring
  method returns is retired, so an image in the ring never reaches `retired`
  (whose `cx.drop_image` would pull its texture).
- Rule: `opened` (a folder open and a re-listing, which may have changed any
  file's bytes) clears the ring and marks the shown image not keepable;
  `refilter` calls `retain` with the new `files`. Whichever plan adds rename,
  trash or delete (`../plans/20261113-gpui-tree-trash/plan.md`,
  `../plans/20261113-gpui-folder-dialogs/plan.md`) must call `Ring::forget`
  with the paths that no longer name what the ring read.
- Rule: a prefetch's result is dropped by the ring's own stale mark, not by
  `order`; its `Paused` ends when it lands, never held for a paint.
- Rule: the page bump (`zoom_page`) comes before `request` in `go` and in
  `refilter`'s `moved` branch: a hit shows the image synchronously, and
  `zoom.shown` stamps the facts with the `seq` current then.
- Rule: what the ring gives up is retired with a `cx.notify()`: `retired`
  drains only in the preview canvas's paint, so an eviction on `settle` with
  nothing else to draw would otherwise hold the images past the cap.
- Rule: a ring method used only by its own tests (for example `Ring::forget`
  before a plan calls it) cannot carry a bare `#[expect(dead_code, ...)]`.
  Under `cargo test` the lint does not fire, so clippy's `-D warnings` fails
  it as an unfulfilled expectation. Use
  `#[cfg_attr(not(test), expect(dead_code, reason = ...))]`, as `settings.rs`
  does.
- Pending: an arrow key onto a neighbour showing it at once, and a folder
  switch and a filter change showing no other folder's or hidden file's image,
  are unverified by hand on Windows.

### Pass 2 runs on pass 1's thread (Inferred)

`Scan::start`'s thread runs `index::run_faces_scan` over `faces_todo` right
after `run_scan`, with the same `ScanFocus` and cancel flag, as the Tauri
app's `run_faces_pass` does on its one task, but on `faces_threads()` (an
eighth of the cores) rather than pass 1's `scan_threads()`: on 22 threads it
cost paging +9.4 ms on ARW and +89 ms on RAF at p50, and on a quarter (6)
still +13 ms on RAF
([results](../plans/_archived/20261110-gpui-focus-marks/results.md)).
One thread and one flag mean `Scan::stop` (and the quit's bounded
`stop_within`) cancels and joins whichever pass is running, and the hot list reaches both passes through the one handle
(`focus_scan` gates on either pass running, as `scan::Status::scanning` tells,
cleared as soon as a re-listing takes the scan, whether or not a new one
starts). The thread starts on every open
with an index, even when pass 1 has nothing to do, so rows below
`FACES_VERSION` still get pass 2; a no-op pass costs one `faces_todo` query.
`Event::FacesDone` is sent also when the pass was canceled, had nothing to do
or `faces_todo` failed, so the view always learns the scan is over. The first
non-empty batch's file names go to the log as `scan faces first:`.

- Rule: the deferred resync (`finished`) runs at `FacesDone`, not at pass 1's
  `Done`, as the Tauri frontend drains its deferred rescan at `faces-done`. A
  re-listing during pass 2 would be harmless (a canceled file keeps its old
  `faces_extractor`, so the next scan resumes it) but would restart the pass;
  waiting for its end keeps a burst of changes from restarting it over and
  over.

### The backend logs through `log`, printed by `logger.rs` (Hit)

Without a logger the backend's `scan extract` lines went nowhere. `logger.rs`
prints `[target][LEVEL] message` to stderr at `Info`, which also shows GPUI's
own lines (the UI font, the GPU, and an `ERROR` for the missing DXGI debug
interface on a machine without the Graphics Tools, which is harmless).
`RUST_LOG=debug` (or `trace`) adds this crate's own lines of that level, such
as `key: <name> action=…` for every keystroke, and not GPUI's.

`logger.rs` also writes each line, as
`[YYYY-MM-DD][HH:MM:SS][target][LEVEL] message` in UTC, to a `Handpic.log`
under the native app's own folder, or to `--log <path>` (see
`app-log.md`'s "The GPUI app's log"), with one `write` per line on a
`Mutex<File>` opened for append and the size kept in memory; past 1 MB it
starts anew as the Tauri app's does: `File::create` (truncate) and a replaced
handle, never `set_len` or a remove on the open file, because on Windows a
file open for append cannot be truncated by `set_len` and a remove while the
handle is open can leave the name pending deletion. The timestamp is a
hand-written civil-from-days, not `chrono` / `time`, so the native lock does not
grow (turning on the features a direct dependency needs could pull further
crates in). `Riffle::start` destructures `Args` exhaustively, so a new flag
needs a `name: _` there too. `main` installs it once the arguments
are parsed and the backend's panic hook right after it. The `page` / `zoom`
timing lines stay `println!` to stdout.

## GPUI

### Callbacks from other threads go through one channel (Inferred)

`AsyncApp` is not `Send`, and a `BackgroundExecutor` task cannot update an
entity, so a callback on a rayon or watcher thread cannot touch the view. The
view owns one `futures::channel::mpsc::unbounded` sender; the callback only
sends, and a `cx.spawn` task drains the receiver with `this.update(...)` per
message. Each scan message carries the scan's id, so a stopped scan's late
messages are dropped.

- Rule: a callback from a non-GPUI thread only sends into that channel.

### `RenderImage` holds BGRA and must be released (Inferred)

`RenderImage` holds BGRA in an `image::RgbaImage`. Every image handed to the
renderer must be released with `cx.drop_image(image, None)` or it leaks on the
GPU.

### Textures have one mip level (Inferred)

A large downscale (a RAF fit to a 1600 px window) may alias. Not settled: one
captured 1:1 crop of a 4416x2944 RAF fit view, a clapboard wall without fine
repeating texture, shows no moire, but no comparison with the Tauri app or a
hard-case image has been made. Three native-side RAF frames showed no moire;
the comparison with the Tauri app is pending (see the pending manual check in
the gpui-port plan's `learnings.md`).

### The image sprite cannot rotate (Inferred)

`PolychromeSprite` has no `transformation`; only the monochrome SVG sprite
does. A rotation has to happen on the pixels before painting.

### There is no after-`Present` hook (Measured)

`cx.defer` from the paint closure runs after the frame's `draw()` and
`present()`, which is the stamp the spike's timings used. It is a workaround
and may change with a GPUI bump. `crates/native` follows that pattern: the
paint closure stamps `paint_started` and `paint_finished`, then the deferred
callback stamps `presented`, prints the `page` line and releases the retired
images with `cx.drop_image`. Doing the release earlier than that deferred
callback could pull a texture out from under the frame being presented.

### Rotating on the CPU needs threads for a RAF's budget (Measured)

Windows, release, idle, 31 runs of synthetic BGRA buffers, p50 in ms, 90 degree
turns (orientation 6 / 8): 1616x1080 plain loop 3.2, 64-pixel tiles 1.6, tiles
over 8 threads 0.9; 4000x3000 plain 62, tiles 21, tiles over 8 threads 9-11;
4416x2944 plain 70, tiles 23, tiles over 8 threads 12-13. Allocating the output
alone costs about 8 ms at 4000x3000 (a plain copy of the buffer takes that long).
In the app on real portrait files: 1616x1080 ARW `orient` 1.3-1.5 ms, 1920x1280
RAF about 2.2 ms, 4000x3000 RAF (GFX 100) 7.6-8.5 ms.

- Rule: `orient.rs` tiles over 8 scoped threads because the tiled loop alone
  misses the plan's 15 ms budget on a 4000x3000 RAF; orientation 1 returns the
  buffer without a copy and 3 reverses in place.

### mozjpeg reports corrupt input by unwinding (Inferred)

`mozjpeg` 0.10's decoder calls `resume_unwind` on a libjpeg fatal error, so a
corrupt file would otherwise end the background task and leave the pager's
request in flight forever.

- Rule: wrap every decode in `catch_unwind`, as `load` does through `guarded`.

### The strip is a `canvas` over a pure model, not a `uniform_list` (Inferred)

`uniform_list` is vertical-first; its horizontal mode
(`ListHorizontalSizingBehavior::Unconstrained`) is about item width, not a
horizontal virtual list. A flex `div` with one child per file lays out every
cell each frame. The strip is one `canvas` that paints only the cells on
screen, fed by `strip.rs` (offset, range, hit test, request order), whose
range can go to `ScanFocus` as it is. Its thumbnails come from
`thumbs.rs`, four reads at a time through the index's read-only connection,
and every image a cell drops goes through `retired`, like the preview's.

### A `canvas` gets no input; its wrapping `div` does (Inferred)

`canvas` registers no hitbox and takes no listeners. The strip's canvas sits in
a `div` with `on_mouse_down` and `on_scroll` (`on_scroll_wheel` is a deprecated
alias at this rev, and `ScrollWheelEvent` of `ScrollEvent`). The events carry
window coordinates; the strip spans the window from its left edge, so the
event's `x` is the strip's `x` and `strip::hit_test` takes it as is. The div's
hitbox already keeps clicks and wheels above the strip out.

- Rule: a canvas that needs input sits in a `div` that owns the listeners, and
  maps the event position against where the canvas is laid out.

### A strip cell's tooltips are transparent `div`s over the canvas (Inferred)

GPUI's `.tooltip(..)` belongs to a stateful element's hitbox: it is
scheduled while that hitbox is hovered and dropped when the pointer leaves
it, so one tooltip on the strip's `div` would not change from cell to cell.
Each on-screen cell's `strip::tip_regions` (the label's name over the name
band, the scan's error over the rest of the cell, an empty text giving none)
becomes an absolutely positioned, transparent `div` over the canvas inside the
strip's `div` (made `relative`), with an id of the region's kind and a
hash of the cell's file path (not its index, which a refilter, a sort change
or a re-listing can give to another file under a still pointer) so the
tooltip's state follows the file. A cell's regions never
overlap: stacked hitboxes that do not occlude are all hovered, and two would
both ask for a tooltip.

- The region `div`s take no listener and do not `occlude()`, so their
  hitboxes are `HitboxBehavior::Normal` and the strip `div`'s hitbox under
  them stays hovered (`Window::hit_test` stops only at a `BlockMouse`
  hitbox): its `on_mouse_down` (left and right) and `on_scroll` still fire.
  A press or a wheel off the tooltip's own hitbox clears it.
- The view is gpui-component's `Tooltip`, built in the `.tooltip(..)`
  closure with `Tooltip::new(text).build(window, cx)` (the tree's pending
  rename) or `Tooltip::element` around a `div` capped at `TIP_MAX_WIDTH`
  (the strip's, so a long error wraps). Its `Render` reads only the global
  `Theme` and needs no `Root`: `Root` is only where gpui-component's own
  managed tooltips (`Tooltip::apply` / `ManagedTooltipExt`) keep their
  overlay, which this app does not use. GPUI places the tooltip at the
  pointer and flips it above or left of the pointer when it would leave the
  window (`Window::prepaint_tooltip`), which keeps the strip's inside.
- Not verified by hand yet: the hover itself, the swap from one cell to the
  next, a scroll under a still pointer and the wrapping of a long error; the
  check is pending in
  `../plans/20261114-native-strip-face-icon-tooltips/learnings.md`.

### A canvas paints an icon with `paint_svg` (Inferred)

`Window::paint_svg(bounds, path, None, TransformationMatrix::unit(), color,
cx)` renders the path through the application's `AssetSource` (`Icons`, so
`icons::SCAN_FACE` resolves as it does for `svg().path(..)`) into a
monochrome sprite drawn in `color`; its `Err` is logged, not unwrapped. The
strip's face focus icon is 12 x 12 px, 2 px inside the image box's
bottom-left, over eight black copies at `overlay::HALO`'s offsets for the
Tauri `drop-shadow`, and painted before the rejected cell's `REJECTED_DIM`
quad so the dimming covers it. Its state is computed in `strip_cells()` from
`facts` as the filter's `passes` does, so it follows pass 2 with no
bookkeeping. That the icon is crisp at 100 % and 150 % is not yet checked by
eye.

### A Windows mouse wheel arrives in lines (Inferred)

`gpui_windows` turns `WM_MOUSEWHEEL` into `ScrollDelta::Lines`, the notch
times the system's wheel-scroll lines (3 by default), positive away from the
user; Shift moves it to `x` (`wheel_scroll_chars`). A precision touchpad
through Direct Manipulation gives `ScrollDelta::Pixels`. `pixel_delta(window.line_height())`
turns either into pixels; the strip subtracts it from its offset, `x` when
non-zero, else `y`, so a wheel toward the user scrolls right.

### A stroked path has butt caps, and no cap option is reachable (Inferred)

`PathBuilder::stroke(width)` builds lyon's `StrokeOptions::default()`, whose
caps are `LineCap::Butt` and joins `Miter`. `with_style(PathStyle::Stroke(..))`
could set square caps, but gpui re-exports `StrokeOptions` and not lyon's
`LineCap`, so naming one would need a direct `lyon` dependency. The `f` mark's
crosshair arms are therefore lengthened by half the stroke width at both ends
(`overlay.rs`'s `segments`), where the Tauri mark's `lineCap = "square"`
reaches, so each arm end keeps the 1 px dark outline. The AF frame is a closed
polygon (`add_polygon(.., true)`) and needs no caps.

- Rule: stroke an open path that must look like a canvas square cap by
  extending its ends by half the width; do not add `lyon` for it.

### A canvas paints text through the window's text system (Inferred)

A `canvas` has no child elements, so the file names are shaped with
`window.text_system().shape_line` (one line, centered with
`TextAlign::Center` over the cell width) and the failure text with
`shape_text` and a wrap width, each clipped with `with_content_mask` to its
cell. The run's font comes from `window.text_style().to_run(len)`.

### Text has no stroke; a label's halo is dark copies (Inferred)

GPUI shapes and paints glyphs filled only; there is no text stroke like the
canvas `strokeText` the Tauri face labels use for their 3 px dark halo. A
dark quad behind the label was the other option; it hides more of the face
than the Tauri halo does. `overlay.rs`'s `paint_label` instead paints the
label in `rgba(0, 0, 0, 0.8)` at the eight 1 px offsets and then in the
label color on top, a ring about as wide as the Tauri stroke's outer
1.5 px, which keeps the look close to the Tauri app's. That it reads on a
bright face is not yet checked by eye. A dot is a `paint_quad` whose corner radii are
half its side, with no `PathBuilder` arc. The label's line height is the
shaped line's `ascent + descent`, so its origin is the text's top, as the
canvas's `textBaseline = "top"`.

- Rule: halo text over a picture with offset dark copies; do not paint a box
  behind it.

### An on-demand judgment is superseded through a shared atomic (Measured)

`ondemand::read_eyes` runs on `cx.background_spawn` and asks its `current`
closure before the detection and before the model. The view cannot answer
from another thread, so `eyes.rs`'s `EyesCache` keeps the latest ticket's id
in an `Arc<AtomicU64>` the closure compares its own id with; a newer request
stores its id and a clear (another folder, a re-listing) stores `0`, which no
ticket has. Opening another folder while a judgment was in its detection
logged `eyes ... model=0.0ms superseded` on the `marks` hand-check folder,
twice out of two runs. Only one judgment runs at a time and the file current
when it settles is asked for next, so 60 presses 30 ms apart over a 300-ARW
folder logged 7 judgments, not 60.

- Rule: hand a background job a cheap `Fn() -> bool` over a shared atomic to
  stop it; do not try to reach the view from the worker thread.

### Key names on Windows are lowercase (Measured)

`KeyDownEvent.keystroke.key` is `"right"`, `"left"`, `"home"`, `"end"` and `"q"`,
checked with injected `keybd_event` key events in the running app. A non-extended
`VK_RIGHT` / `VK_LEFT`, the keypad's, gives the same names.

Injecting keys with `keybd_event` needs the app window in the foreground for
every key; in a long injected sequence most keys went missing once focus moved
away. Re-assert and check the foreground before each key (skip the key
otherwise), and pick the process by its path in this worktree, since another
worktree's `riffle-native.exe` may be running or exiting. Report a skipped key
rather than counting it as a page. Held-key measurements also need an idle
machine: another session's `cargo` build fails the gate, so redo the slot.

### Every keystroke goes through the keymap (Inferred)

`keys.rs`'s `key_name` names a keystroke as the keymap does: the modifiers
held in `ctrl+alt+shift+meta+` order (`platform` is `meta`), then the key
lower-cased, GPUI's `left` / `right` / `up` / `down` as `arrowleft` ...
`arrowdown`; a lone modifier has no name. `Bindings` maps each bound key to
its action, built at startup and again on every Keyboard Shortcuts edit
(`self.bindings` replaced, the lookup still one `HashMap` read). The window's handler, past the tree's gate while
the tree has the keyboard ("The tree takes the keyboard through one flag in
the view"), takes `home`, `end` and `q`
literally first (the native app's own keys, bound to no action), then the
actions: `previous` / `next` page and `focus` toggles the `f` overlay and `zoom` the `z` 1:1 view, so a
user's override of those keys applies; a judgment action goes to
`judgment::Command::from_action`, `undo` / `redo` to `history::Undo`,
`rejectRest` to `reject_rest`, and any other action is logged at `debug` until its step lands. A
later step's key (peaking `e`) is added as its keymap action (`peaking`) in
that `match`, not as a literal key: a literal `"e"` arm placed after the action
arms never fires with the default keymap, because the catch-all
`(_, Some(action))` arm takes the key first.

Read from `gpui_windows` at the pinned rev, not yet seen in the running app
(the pending manual check in the gpui-judgments plan's `learnings.md`):

- A shifted letter arrives as the lower-case letter with `shift` set
  (`shift+x`). A shifted digit or punctuation key arrives as the character it
  types with `shift` cleared (`!` for Shift+1, from `get_shifted_key`),
  where `keys.ts` names it from `event.code` (`shift+1`); no default key is
  one, but a user override such as `shift+1` would not match.
- With Ctrl or Alt held, a digit is `MapVirtualKeyW`'s unshifted character,
  `1` on a US or JIS layout (`ctrl+alt+1`), but `&` on AZERTY, where the
  Tauri app's `event.code` still says `1`. Windows reports AltGr as Ctrl+Alt,
  so on a layout with AltGr `ctrl+alt+<digit>` is also what AltGr+digit
  sends; the default color keys are the Tauri app's, so both apps behave the
  same there.

### The canvas's size reaches the view through a shared cell (Inferred)

The preview canvas's bounds are known only inside its paint closure. The
closure writes the size in device pixels (`bounds.size * window.scale_factor()`,
rounded) into an `Rc<Cell<(u32, u32)>>` the view also holds, as `retired` and
`pending` are shared; the 1:1 view reads it when it asks for a crop. A deferred
entity update from the closure was the other option and needs more of
`main.rs`. The value is the last painted frame's: a resize is seen by
`observe_window_bounds` before the next paint, so the crop is re-cut only
after `zoom::CROP_RESIZE_DEBOUNCE` (150 ms), by when the new size has been
painted.

- Rule: read the cell after at least one paint; before that it is `(0, 0)`
  (`crop_size` clamps it to 1).

### 1:1 is one JPEG pixel per device pixel (Inferred)

`paint_image` snaps the visible bounds to device pixels (`snap_bounds`) and,
when the image runs past the clip bounds, cuts the atlas sub-tile by the
visible fraction, rounded. `zoom::Paint` therefore places the upright crop with
its point of interest on a whole device pixel (`canvas_center` floors the
canvas center in device pixels) and gives it its own size divided by
`scale_factor`, so both edges land on whole device pixels and the sub-tile is
cut on whole texels. The window ran at scale 1.0 (96 dpi, a 1600x1000 client)
during the check; that the crop shows without resampling blur by eye is
pending the user (see the zoom-peaking plan's `learnings.md`).

- Rule: anything meant to be 1:1 is placed in device pixels and divided by
  `scale_factor` at the end, never laid out in logical pixels.

### A crop is on screen 8-112 ms after `z` on a busy machine (Measured)

Release build, `zoom` lines of `z` presses on the zoom-peaking test folder,
not idle (other sessions ran), the file cache warm, 25 lines at scale 1.0
(crops of 1024 px per axis at most): four ARW files `read` 0.7-4.7 ms,
`decode` 3-27 ms, `orient` 0-20 ms on the portrait ones, `keypressToPixels`
8-43 ms (112 ms with the outlier below); two RAF files (X-E5, X-H2S) `read`
1.6-5.2 ms, `decode` 14-33 ms, `keypressToPixels` 19-89 ms; two JPEG-only
files `decode` 21 and 47 ms.
`decode` includes the RGBA to BGRA swap. Two `orient` outliers of 36 and 76 ms
(a 1024 px crop over 8 scoped threads, 1-2 ms otherwise) are attributed to the
load, not measured apart.

### Peaking runs off the main thread, one job at a time (Measured)

`peaking.rs`'s job (resize to the drawn size, then the Sobel) takes 25-60 ms on
a fit view, several frames, so it runs on `cx.background_spawn`, reading the
shown `RenderImage`'s bytes through its `Arc` without a copy. One job is in
flight, as the Tauri worker holds one request: under a held key each page
would otherwise queue a job on the executor the next preview decode runs on.
A layer replaced while a job is out is never computed; the reply takes the
newest unsent layer next. The fit view's drawn size is known only in the paint
closure, so the closure compares it with the layer the view rendered with and,
when they differ, asks for the new layer through a `WeakEntity` update in the
`cx.defer` it already runs after paint; while the layer matches nothing is
deferred.

- Rule: an overlay is drawn only for the `ImageId` and device size it was
  computed for (`peaking::Paint`), so another file's or another size's never
  shows; replaced and cleared overlays go through `retired`.

### The fit view's overlay is resized bilinearly (Measured)

The overlay is computed at the drawn device size, as the Tauri app rasterises
the bitmap at that size first. `image::imageops::resize` with
`FilterType::Triangle`, the nearest to a canvas `drawImage`, was kept: at
scale 1.0 it takes about 22-25 ms for a 1616x1080 ARW to 1275x852 and 47-67 ms
for a 4416x2944 RAF to 1278x852, the Sobel 5-7 ms on top. So on a RAF the
overlay comes about 50-60 ms after the image; the held-key measurement showed
no cost to paging, so no cheaper filter was tried. The 1:1 crop needs no
resize (Sobel 4-6 ms).

### The performance overlay is a band painted last in the preview canvas (Inferred)

`d` (the keymap's `performance`) toggles `perfgraph.rs`'s `Overlay`. Showing
it starts recording in `perf.rs`'s `Perf`, the backend's `perf::Sampler`,
whose thread only sends `Event::Perf` into the view's one channel (the
sample is stamped with `Instant::now()` when the view takes it, as the Tauri
listener stamps `performance.now()`), and a 500 ms `cx.notify()` loop held as
an `Option<Task<()>>`; hiding it stops all three and drops what was
recorded. `Sampler::stop` only signals, so one late sample can still arrive
and the model drops it. The graph is painted by the preview canvas's closure
after the image, the 1:1 crop and the `f` marks, and also with no image, as
the bottom 120 px of the canvas's bounds under a content mask, so it moves
neither view and takes no input.

A switch's latency point is completed in the same `cx.defer` as the `page`
line, from the same `timing::Stages`, its value `keypress_to_pixels` and its
bar the eight native stages stacked (they do not sum to the value, see
"`keypressToPixels` differs from the Tauri app's"). A page no key or strip
click turned (an open, a refilter, a re-listing) starts at `switched`, the
time `request` was last called, and is built only while the overlay is on;
the `pending` slot carries whether it was keyed, and only a keyed one prints
the `page` line and goes into `Summary`. A stale preview drawn on the way
(`stale`) and a failed one add no point.

- Rule: with the overlay off nothing samples, records or redraws; the only
  cost is the `on()` check in `paint` and in `loaded`.


### One progress bar sits on the status row's top edge (Inferred)

`progress.rs` holds the bar's model: `fraction`, a port of `progress.ts`'s
`progressWidth` (0 while `total` is 0, never above 1), and `shown`, which
picks the `(done, total)` the bar shows when several operations run: a Move
to Trash, Delete Permanently or trash redo run first (`Riffle.trashing`, the
counts the pane's note is formatted from with `trash::progress_status`), then
a folder operation's run, then the scan's running pass
(`scan::Status::running`, pass 1's before pass 2's, the two sharing the one
bar). `main.rs`'s `progress_bar` draws it as two plain `div`s, a 2 px
`MUTED_FILL` track spanning the window and a `PRIMARY_COLOR` indicator whose
width is `relative(fraction)`, as the Tauri `#scan-progress`. It is not
gpui-component's progress element: only the settings modal and the tree's
inline rename draw from it (see "gpui-component is initialized; the settings
modal and the tree's inline rename draw from it").

- Rule: the bar is `absolute` inside the status row (made `relative`), on
  its top edge, so showing or hiding it changes no layout: the canvas's
  `viewport` cell, the fit scale, a 1:1 crop and the peaking layer are not
  recomputed by it. The status row stays with every pane hidden, so the bar
  does too. Do not give it a flex row of its own.
- Rule: the bar adds no `notify` or timer; it is redrawn by the notifies the
  scan's `Progress` / `FacesProgress` and the trash's `TrashProgress` already
  make, and it goes wherever `scan_status.clear()` and `trashing = None` do.
- Rule: pass 2 starts at `(0, 0)` and pass 1 can start at a total of 0, so
  the track shows empty there.
- Another operation with a count (Sequence JPEG Timestamps, Import) feeds it
  by passing its `(done, total)` as `shown`'s `folder_op` argument, `None`
  today, from the state its own progress messages update.
- The trash undo, Rewrite Sidecars and Delete Sidecars runs have no progress
  callback in the backend (`trash::undo_run`, `foldersidecars`), so the bar
  shows nothing for them. A new operation needs a `(done, total)` callback
  before it can feed the bar.

### Text is typed through gpui-component's `Input` (Inferred)

The settings modal's text fields (the label names, the import pattern) are
`gpui_component::input::InputState` entities drawn with `Input`; GPUI at the
pinned rev has no text input of its own. The state registers itself as the
window's input handler while focused, so the platform's text input and the
IME reach its `EntityInputHandler`. The view reads the text with `value`, sets
it with `set_value` (which emits no `Change` and clears the undo history, so
`settingsview.rs`'s `show` leaves a field already showing the text alone) and
subscribes to `InputEvent`:

| Owner's meaning | `InputEvent` / key |
| --- | --- |
| commit | `PressEnter` (Enter) or `Blur` (the focus left the field) |
| commit, then close the modal | Escape: no event; the `Escape` action propagates on a single-line input, so the key reaches `SettingsView::key_down` as `ModalKey::Close`, which commits the focused field and closes |
| the text changed | `Change` (the pattern's example line follows it) |

- Rule: the editing keys are the `Input` key context's bindings (see the
  table in "gpui-component is initialized"), dispatched as actions before
  any key-down listener. A handled one stops there, so it reaches neither the
  box's `key_down` nor the window's keymap; `Enter`, `Escape` and the
  `IndentInline` / `OutdentInline` of `tab` / `shift-tab` propagate on a
  single-line input, so the box's handler still sees Escape and Tab
  (`ModalKey::Close`, `ModalKey::Focus`). The fields stay in the modal's tab
  order: the state's focus handle is a tab stop and the frame's is not.
- Rule: a stopped key down types nothing on Windows. `gpui_windows`'s message
  loop calls `translate_accelerator` first, which dispatches `WM_KEYDOWN` to
  the window; when an action handler or a listener stopped it (`propagate`
  false) the loop skips `TranslateMessage`, so no `WM_CHAR` follows. A
  consumed editing key therefore never also inserts a character, and a key
  meant to be typed must not be stopped by anything on the dispatch path: no
  `Input` binding is a bare printable key, the settings box's handler lets a
  key it does not take go on while a field is focused
  (`settingsmodal::stops`), and the window's root handler returns without
  stopping it while the modal is open. `WM_CHAR` then reaches the focused
  state's handler; a propagated Enter, Escape or Tab gives a control
  character, which `parse_char_message` drops. A key the IME takes arrives
  as `VK_PROCESSKEY`, for which `MapVirtualKeyW` gives no character, so GPUI
  raises no `KeyDownEvent` and `TranslateMessage` always runs: composition
  never passes through the bindings or the listeners, and Enter or Backspace
  inside a composition stay the IME's.
- Rule: the look is the port's, set per field (`settingsview.rs`'s `field`):
  `focus_bordered(false)` drops the component's focus ring (the theme's
  `ring`), and the border is `INPUT_BORDER`, `PRIMARY_COLOR` while the
  state's handle is focused, with no fill, `rounded_sm`, 8 / 3 px padding,
  13 px text and a 29 px height (`FIELD_HEIGHT`, a 13 px line at GPUI's
  default line height plus the padding and border). These are `Styled`
  refinements, applied after the component's own size and theme styling;
  `Input::h` is the multi-line height, so the frame's is set through
  `Styled::h`. The caret and selection colors are `Default Dark`'s, which
  `theme.rs` does not map.
- Rule: a right-click opens the component's edit menu (Cut, Copy, Paste,
  Select All) as an OS-native menu on Windows and macOS (`native_menu`),
  which needs no `Root`. On Linux it is a drawn `PopupMenu` held by `Root`'s
  overlay (`WindowState::native_menu_overlay`), which finds one now that
  `Root` is mounted; before, nothing opened there. Not seen on Linux.
- What the component adds over the port it replaced (`textinput.rs`, GPUI's
  `input.rs` example): undo / redo, word movement and deletion, horizontal
  scroll of a long text, double-click word selection and the right-click
  menu.
- Not verified headless: the typing, the IME, the commits, the look and the
  right-click menu; the hand check is pending in
  `../plans/20261114-gpui-component-adoption/learnings.md`.

### A launch without a folder shows only the tree (Inferred)

`start` opens the folder `main()` chose (`session::startup_folder`: the
command line's, else the remembered `lastFolder` while it is still a
directory and `--auto` is off), and nothing else: the OS folder picker
(`cx.prompt_for_paths`) is not shown at launch, and a folder is opened from
the left pane's tree, from the picker on `open` or by a drop onto the window
(see below). A remembered folder opens through the same `open` as
the command line's, so it is revealed in the tree and resumes at its last
viewed file. When neither gives a folder the window
shows the tree, the no-folder hint in the preview area (see "The preview
area says why it shows no photo" below), an empty strip, and `no folder` on
the status line; every handler already tolerated `dir: None` and an empty `files` (the
pager's `go` and `request` are no-ops at length 0, `judge` and `step` need a
current path, `zoom::toggle` needs files, the strip's hit test finds no cell).

When the launch still used it, `cx.prompt_for_paths` with `PathPromptOptions {
directories: true, files: false }` was the OS folder dialog; Enter on a typed
path navigated into the folder, and a second Enter selected it (Measured).

### A launch restores the last folder and the window (Measured)

`session.rs` holds the rules, free of GPUI types: `startup_folder`, the
`Geometry` stored under `windowBounds` (the restore bounds in GPUI's logical
pixels, `maximized` and the display's UUID as text; `from_setting` reads a
malformed value, a non-finite position or size or a size that is not positive
as nothing), and `place`, which picks the display (the saved UUID's when the
saved bounds intersect it, else the first display they intersect, else none:
the centered 1600x1000 default) and moves and shrinks the bounds into that
display's visible bounds. `main()` passes that display's `display_id` with
`WindowBounds::Maximized` or `Windowed`: on Windows GPUI keeps given bounds
only when their center is on the `display_id`'s display (else the primary
one's) and otherwise silently uses its own default size, so the display and
the clamp are both needed.

- Rule: `lastFolder` is the Tauri app's key and plain-string value, so a store
  seeded from a Tauri `settings.json` reopens the Tauri app's last folder on
  the first launch. It is written where a listing sets `self.dir`, every open
  (the command line's included), as `context::shown_path` of the canonical
  path (a `\\?\` verbatim path would not reveal in the tree), and only when it
  differs from the last value, so a watcher re-listing writes nothing.
- Rule: the geometry is written once, by the settings quit handler before its
  drain, from an `Arc<Mutex<Option<Geometry>>>` the `observe_window_bounds`
  callback keeps current, and only when it differs from what was loaded. The
  handler holds the cell, not the view. A fullscreen window is kept by its
  restore bounds, windowed. A crash or a kill loses the session's geometry,
  not its folder.
- Rule: closing the window on Windows fires the bounds observer once more
  while the handle is going: `GetWindowPlacement` fails (`failed to get window
  placement` in the log) and `window_bounds` reports the frame insets around
  an empty rectangle (`x 8, y 19, width -16, height -39` here), so the
  observer keeps only a positive size (`Geometry::is_usable`). Without it the
  close button stored that value.
- Rule: `--auto` suppresses both restores and both writes, so a benchmark
  opens at the default size with only its own folder and leaves the store as
  it was. `--settings`, `--index` and `--log` suppress nothing.
- Measured (2026-11-14, Windows 11, one display at 96 DPI, a test store,
  quitting through the title bar's close message): a window moved to the
  outer rectangle (200, 150)-(1400, 950) stored `x 146, y 169, width 1184,
  height 761` and reopened at exactly that rectangle; GPUI's restore bounds
  are not the outer rectangle, but they round-trip. A maximized quit stored
  `maximized: true`, reopened maximized and un-maximized to the same
  rectangle; a minimized quit stored the restore bounds. Bounds far off the
  display opened at the default; an unknown UUID opened on the display the
  bounds intersect. `--auto 1` left both keys unchanged, and a renamed
  remembered folder opened nothing.
- Not seen yet: a quit with `q`, a snapped window, a second monitor (with a
  different scale factor), and the taskbar docked at the top or left (the
  restore bounds are in workspace coordinates there, the clamp compares them
  with the display's screen coordinates): the pending check in
  `../plans/20261114-gpui-session-restore/learnings.md`. macOS and Linux are
  covered by the CI build and the unit tests only; Wayland cannot place a
  window, so only the size can come back there.

### `open` shows the OS folder picker, a later open winning (Inferred)

The `open` action (`ctrl+o`, `meta+o` on macOS) is `pick_folder`, the Tauri
`openFolder`: it reads the `opens` counter, calls `cx.prompt_for_paths` with
the options of the settings' `Choose…` (`directories: true, files: false,
multiple: false`) and, in a `cx.spawn`, opens the chosen folder through
`open(path, None, None)`, as a click in the tree does (whole, without
`--size` / `--max`, revealed in the tree).

- Rule: the answer is dropped when the picker was canceled, or when `opens`
  moved on while it was up (a folder opened from the tree meanwhile wins).
  A picker error (`Ok(Err(e))`) goes to the status line through `fail`.
- Rule: `open` runs while the tree has the keyboard (`TREE_PASSTHROUGH`) and
  not while the settings, the trash dialog or a folder dialog is open, since
  `key_down` returns before the keymap then.
- Not verified in the running app: whether a second `open` while the picker
  is up shows a second dialog, and whether the window's focus returns to the
  photo when it closes (the pending hand check in
  `../plans/20261114-gpui-pane-toggles/learnings.md`).

### The preview area says why it shows no photo (Inferred)

`empty.rs` is the port of `empty.ts`: `state` judges, in `emptyState`'s
order, no folder (`dir` unset), no listed file (`all` empty), no shown file
(`files` empty, the text being `filter::FILTERED_TEXT`), a failed preview of
the current file, else nothing; `text` gives the state's message and
`open_hint` the no-folder one, the `open` action's keys from the live
keymap's `bindings()` shown through `settingsmodal::display_key` and joined
by ` or `, the key clause left out when `open` has none. `render` wraps the
preview canvas in a `relative` div that takes the canvas's old `flex_1` /
`h_full` (plus `min_w_0`), the canvas `size_full` in it, so its bounds, and
with them `viewport`, the fit, the 1:1 crop and the perf band, are the same;
the message is an `absolute`, full-size, centered child in `MUTED_COLOR` at
14.4 px (`0.9rem`) with `p_4` padding (`empty_message`).

- Rule: only the no-folder message has an `id`, `cursor_pointer` and an
  `on_click` (primary button) calling `pick_folder`; the others register no
  listener. The settings modal, the trash dialog, the folder dialogs, the
  context popover and the sort / filter menus are deferred, `occlude()`d
  layers, so a press on them does not reach the message.
- Rule: "the current file's preview failed" is `shown` being
  `Shown::Failed` at `pager.current()` with the 1:1 view off (Tauri's
  `previewFailedSeq === seq && !comparing && !zoomed`; there is no compare
  view). A page turn moves `pager.current()` off `shown`'s index, so the
  message goes at once; `refilter` moves `shown`'s index through
  `filter::shown_after`, so it stays with the failed file.
- Rule: a failure clears the image (`loaded`'s `Err` arm calls
  `clear_image`), and nothing puts an older one back: `Finished::Stale` never
  draws a failure and draws a success only after the pager has moved on, the
  ring never holds the failed file, and a failed crop releases an earlier
  page's held crop (`zoom::arrived`).
- Rule: `dir` is set when the listing arrives, as Tauri's `openDir`, so the
  hint shows while the first open's listing runs.
- The no-folder text is `empty.ts`'s, with its "Drop a folder onto the
  window" clause (see the drop section below).
- Not verified in the running app: the message's placement, the click
  opening the picker, the key clause following a rebinding, and the failed
  preview message on `D:\Photos\samples\NEF\NIKON_D70_Nikon.nef` (the
  pending hand check in `../plans/20261114-gpui-empty-state/learnings.md`).
### A folder dropped onto the window opens, a later drop winning (Inferred)

The root `div` of the window's `render` takes the drop
(`on_drop::<ExternalPaths>`, `drop_paths`), a port of the Tauri
`tauri://drag-drop` listener and `dropped_dir`, whose pure rules are in
`dropped.rs` (copied, since the native crate must not depend on
`crates/app`). One dropped folder opens through `open(path, None, None)`, as
a click in the tree does; one dropped file, of any kind, opens its parent
folder, the file not selected; no path, several paths, or a path that is
neither a directory nor a file opens nothing and puts the Tauri
`Drop a single folder or RAW file.` (`DROP_TEXT`) on the status line.

- Rule: the directory test is a `stat`, run on `cx.background_executor()`
  as Tauri runs it in `spawn_blocking`, then `open` in `this.update`.
- Rule: drops are ordered among themselves by the view's `drops` counter, the
  port of `dropCounter`: a drop whose resolve finishes after a later drop
  opens nothing. Unlike `pick_folder`, a drop is not dropped because `opens`
  moved on while it resolved; Tauri mints its folder token only after the
  path has resolved. `open` defers under a running trash or delete as for
  any open, so the drop has no gate of its own.
- Rule: GPUI fires `on_drop` only on a hovered hitbox, and the settings
  modal, the trash dialog and the folder dialogs are `occlude()`d layers, so
  a drop over an open dialog does not reach the root and is ignored (as a
  click in the tree is blocked by the same layers), where Tauri's drop is
  window-wide except under its format gate.
- Rule: while an OS file drag is over the window, the main area (the tree,
  the preview and the meta pane, the Tauri `#main`) shows the Tauri
  `body.dragging #main` outline, 3px in `--ring`: `drop_outline`, an
  absolute child laid over that area, so the outline covers its content as
  the CSS `outline-offset: -3px` does instead of shrinking it. GPUI applies
  a drag-over style only to an element with a hitbox, and only while its own
  hitbox (`drag_over::<ExternalPaths>`) or its group's
  (`group_drag_over::<ExternalPaths>` on the root's `DROP_GROUP`) is under
  the drag, so the outline carries both: the first gives it the hitbox, the
  second lights it over the strip and the status row too. GPUI clears the
  drag on a drop or when it leaves the window (`FileDropEvent::Exited`
  refreshes the window).
- Not verified in the running app: the drop of a folder, of a file and of
  two folders, a drop while a dialog is open, and the outline drawing and
  clearing on leave and drop, on Windows (the pending hand check in
  `../plans/_archived/20261114-gpui-folder-drop/learnings.md`). macOS and Linux file
  drop delivery is covered by the CI build only.

### The folder tree is a `uniform_list` over rows kept in the view (Inferred)

The left pane (220 px, the Tauri `#side`) sits beside the preview canvas and
above the strip, which still spans the window, so `strip_clicked` still takes
the event's window `x` as the strip's. Its rows are `tree::Tree::rows`,
rebuilt into the view's `tree_rows` whenever the tree changes (never in
`render`), and drawn by a `uniform_list` over them: it lays out only the rows
on screen, its rows are plain `div`s with their own click listeners (the
expander's `on_click` stops propagation so the row's open does not run), and
its `UniformListScrollHandle::scroll_to_item(.., ScrollStrategy::Nearest)` is
what the reveal and the tree's keyboard scroll a row into view with. A
`canvas` like the strip's would need its own hit testing and scroll model; the
strip needed one because `uniform_list` is vertical only.

- Rule: tree nodes are keyed by the tree's own spelling (the root's, then one
  separator per level), never by the canonical `\\?\` path the index uses.
  The open folder's row is found with `tree::relation`, case ignored on
  Windows and macOS (`tree::IGNORE_CASE`).
- Rule: every open (`open`, from the command line or the tree) bumps the
  `opens` counter and reveals the folder: the chain from its root (a root
  added for a folder none holds) is expanded and listed one level at a time on
  `background_spawn`, respelled from each listing, and the end row is
  highlighted and scrolled to. A step that finds `opens` moved on stops, so a
  later open owns the tree. An open before the roots have been read waits in
  `reveal_wanted`.
- Rule: the same counter guards the listing of the open: its background task
  sets the folder watcher under a separate lock (never taken by the UI
  thread, which bumps the atomic counter without locking) only while its
  open is still the latest, and its result is dropped otherwise, so a quick second
  open is never overwritten by the first's late listing, nor left with the
  first folder's watcher.
- Rule: listings settle in order. Each one's background task first awaits the
  previous listing's `settled` signal (a oneshot kept on `Riffle`), then stops
  the previous scan and joins the launch's eviction, then signals its own, so a
  second open never reconciles or scans while an earlier listing is still
  stopping the old scan or the eviction is still deleting rows.
- Inferred: the preview canvas is 1160 x 852 logical px in the default 1600 x
  1000 window with the tree shown (1380 less the pane's 220), and 1380 x 852
  with it hidden by `toggleLeft`. Not measured in the running app. A pane
  change re-cuts a 1:1 crop through `zoom_resized`.

### The panes are one `Panels` value, stored under `panels` (Inferred)

`panels.rs` is the port of `panels.ts` and of the Tauri `panels_setting`:
`Panels { left, strip, right }`, all shown by default, `toggle(Panel)`,
`toggle_sides` (Lightroom's Tab: hide both side panes when either is shown,
else show both), `from_setting` and `setting`. `settings.rs` reads it into
`Settings.panels`, and the view holds it as `panels`. `toggleLeft`,
`toggleRight`, `toggleStrip` and `toggleSides` all go through
`change_panels`, the port of `changePanels`.

- Rule: `change_panels` sets the panes, hands the keyboard back from the tree
  when the left pane is now hidden (by `toggleLeft` and `toggleSides` alike),
  re-cuts a 1:1 crop (`zoom_resized`), writes `panels` as
  `{"left": bool, "strip": bool, "right": bool}` and repaints. The preview
  canvas is `flex_1`, so it takes the space a hidden pane frees.
- Rule: the stored value is the Tauri key and shape, so a store seeded from
  the Tauri `settings.json` comes up with the Tauri app's panes; a missing or
  non-boolean entry, or a value that is not an object, means shown.
- Rule: under `--auto` every pane starts shown whatever the store holds, and
  nothing is written to `panels` (`remember_panels` off, set in `main` on the
  value `Riffle::new` returns, inside the `cx.new` closure). It is not a
  parameter because `Riffle::new` already takes seven arguments, and an eighth
  trips `clippy::too_many_arguments` (`-D warnings` in CI). Another
  launch-only suppression flag has the same choice: set a field after `new`, or
  add `#[allow]` as `tauri-app.md` does for `Writer::set`.
- Rule: hiding the strip removes only its own `div`; the status row (the
  status line and `Sort` / `Filter`) stays, unlike the Tauri app, where the
  menus sit in the strip's header. The strip's bookkeeping (range, thumbnails)
  keeps running while it is hidden.
- Not measured: the toggles on macOS and Linux are covered by the CI build
  and the unit tests only.

### The tree follows the disk through one `TreeWatch` (Measured)

The view holds one `riffle_backend::treewatch::TreeWatch` whose callback sends
`Event::TreeChanged(path)` (the tree's spelling) into the scan's channel.
`tree_changed`, which every change of the tree runs, computes
`Tree::watched_folders` and, when it differs from the set last handed over,
hands the whole set to `TreeWatch::set` on `background_spawn`, each hand-over
awaiting the one before it (the Tauri `set_tree_watches` chain), so two sets
never apply out of order. `TreeChanged` re-lists the node only while it is
still expanded and drawn (the event and a collapse can cross); a failed
re-list is logged and leaves the node as it was, since deleting a folder
usually fails its own re-list before its parent's removes the row. The
menu's `Refresh` re-lists the folder expanded or not and forgets the set
last handed over, so the next change sends it again and a watch that failed
is retried; on the open folder it also re-lists the folder as a watcher
change would (through `Resync`). The roots are read again whenever the
window becomes active (`observe_window_activation`, once the first read has
answered); `add_roots` skips the roots already shown.

- Measured (a throwaway test against `crates/native/Cargo.lock`'s `notify`
  8.2.2 on Windows): `set` of one folder took 0.4-0.6 ms, a collapse
  (`unwatch` then `settle`) 0.1-0.3 ms, five collapse / expand rounds each
  stopped and resumed the events (none for a change while collapsed, one
  ~500 ms after a change once expanded again), and renaming the folder's
  parent after the last collapse succeeded, so the handle was released. The
  `configure` that queued behind `Unwatch` (`tauri-app.md`) does not stall
  here. `TreeWatch` is a backend test, which cannot run from the native
  workspace (a path dependency with dev-dependencies is not testable there),
  hence the throwaway.
- Rule: on Windows a watch pins its folder's ancestors against a rename;
  the folder rename and the date split go through the backend's
  `rename::rename_folder` / `datefolder::split_folder`, which call
  `with_released` (see "A folder rename releases the watches, then follows in
  the tree and reopens").

### The right-click menu is a deferred popover at the pointer (Inferred)

`context.rs` holds the menus' items and the open menu (`Menu`: where it was
opened, in window coordinates, and its target, a tree folder or the strip's
focused file and targets); `main.rs` maps them to `div`s. A right press
(`on_mouse_down(MouseButton::Right, ..)`) on a tree row opens the folder
menu, and one on a strip cell (the strip's wrapping `div`, `strip::hit_test`
giving the cell) the strip's menu, closing the status row's menus. The
popover is `div().absolute()` holding `deferred(anchored().position(at))`:
`anchored`'s default `AnchoredPositionMode::Window` takes the
`MouseDownEvent`'s window position as is, and its default fit,
`AnchoredFitMode::SwitchAnchor`, puts the menu on the pointer's other side
when it overflows an edge and then still snaps it inside, as the Tauri
`menuPosition`. `snap_to_window` would only shift it, so a menu opened on
the strip at the window's bottom would cover the pointer. It is
`occlude()`d with `on_mouse_down_out` closing it on any press outside, of
any button, so a right press on another row closes the menu in the capture
phase and opens that row's in the bubble phase; any key closes it (`escape` stops there,
the others then act as usual), as in the Tauri app. Items use `on_click`; a disabled item is in
`muted_foreground` with no hover and no handler.

- Rule: the look is the sort and filter menus' (`menu_surface`,
  `menu_separator`, `menu_item`; see "The sort and filter menus are deferred
  popovers over the window" for the theme tokens and why `PopupMenu`,
  `Popover` and `ContextMenu` are not used), each item's shortcut at the
  row's right end in `muted_foreground` at `text_xs`, as `PopupMenu` places
  its key binding. A menu taller than the window less 40 px scrolls under the
  component's scrollbar on `context_scroll`, new on every open.

- Rule: the strip's right-click follows the Tauri strip's `contextmenu`
  handler: a cell outside the selection becomes the selection alone, one
  inside keeps it, and the focus moves to the cell either way; the strip's
  empty area opens nothing. The menu's targets are `selection::targets`,
  taken when it opens. Its judgment items carry the keymap's action names
  (`Action::Judge`) and run through `Command::from_action` and `judge` as a
  key does, so they act on the selection as it is when picked; each item's
  shortcut is its action's first key, shown as `keys.ts`'s `displayKey`
  (`space` as `Space`), not `Keymap::accelerator_for`, which spells keys for
  the Tauri menu bar.
- Rule: the reveal item calls `cx.reveal_path`, which on Windows selects the
  item in its parent through the desktop folder's `ParseDisplayName` (`open_target_in_explorer`)
  and only logs a failure. A root (`D:\`) has no parent, so the folder menu
  opens it with `cx.open_with_system` instead. The tree's paths never carry
  the verbatim `\\?\` prefix, but the strip's files do (the index's
  canonical folder). The shell refuses a prefixed path: `SHParseDisplayName`
  returned `E_INVALIDARG` (`0x80070057`) for `\\?\C:\Windows\notepad.exe` and
  `S_OK` without the prefix (measured from PowerShell, 2026-11-14), so the
  strip's reveal passes `context::shown_path` (`trash.ts`'s `shownPath`).
- Rule: the copy items write the tree's spelling of the path and the row's
  name, or the strip's targets' listed paths (prefix and all, as the Tauri
  app writes them) and file names, one per line, through
  `cx.write_to_clipboard(ClipboardItem::new_string(..))`.
- Not verified headless: the hand checks are pending in
  `../plans/20261113-gpui-tree-trash/learnings.md`.

### The tree takes the keyboard through one flag in the view (Inferred)

The window keeps its one `FocusHandle`; `tree_keys` on the view says whether
the tree has the keyboard, and `key_down` checks it right after the
right-click menu's close and before every other arm (the literal `home` /
`end` / `q`, the status row's menus' `escape`, the keymap's actions). A
second `FocusHandle` on the pane would give the same routing, but the gate
must also sit before those literal arms in the window's handler, and a press
outside the pane must still reach its own handler, so a flag and the pane's
mouse listeners were the smaller change.

- Rule: a left press in the pane (`on_mouse_down(Left)`) gives the tree the
  keyboard; a press of any button outside it (`on_mouse_down_out`, the
  capture phase, so the press still reaches its target) hands it back; a
  right press in the pane does neither. A press on the right-click menu hands
  it back too (`capture_any_mouse_down` on the popover, which is `occlude()`d
  and may be drawn over the pane; the capture phase, since the scrollbar stops
  a press on it from bubbling), as a Tauri menu button takes the focus.
  `escape` and hiding the pane (`toggleLeft`, `toggleSides`) hand it back as
  well. The
  type-ahead buffer starts over whenever the keyboard moves.
- Rule: while the tree has it, `tree_key_down` takes `escape`, the arrows,
  `home` / `end`, `enter` and any printable key without Ctrl / Alt / Meta
  (type-ahead); any other key runs its action only when
  `treekeys::tree_gate` lets it through (`open` and the four pane toggles,
  `TREE_PASSTHROUGH`) and is
  otherwise swallowed, so `q`, the judgment keys, paging and undo never reach
  the photo. The cursor row is outlined in `--sidebar-ring` only while the
  tree has the keyboard.
- Rule: opening a folder from the tree, by a click on its name or `enter`,
  leaves the keyboard with the tree, as in the Tauri app (`folders.ts`'s
  `openFolder` and `main.ts`'s `openDirectory` never blur the tree); the
  open's reveal puts the cursor on the opened folder.
- Inferred from `gpui_windows`' `process_key`: type-ahead takes the
  keystroke's own `key_char` (`keys::typed_char`), since `key_name`
  lower-cases a shifted letter and names the space bar `space`. `key_char` is
  `ToUnicode`'s character with a control character dropped, so `enter`,
  `tab` and `escape` type nothing. Type-ahead with an IME on is not a goal.
- Rule: `Expand All` walks one listing at a time on `background_spawn`, depth
  first through an explicit stack of chains (no async recursion), skips a
  link (`is_link`), stops a branch whose chain was collapsed or dropped
  meanwhile and marks and reports a folder that fails, going on with the
  rest. `Collapse All` is `Tree::collapse_all`. The watch set follows through
  `tree_changed`.

### The trash dialog is a component `Dialog`, its focus in GPUI (Inferred)

The strip menu's `Move to Trash…` and `Delete Permanently…` open one
gpui-component `Dialog` in `Root`'s layer (`window.open_dialog`), drawn by
`main.rs`'s `trash_dialog` inside `modal.rs`'s shell: the title, the line and
a footer with a `danger()` run `Button` and a default Cancel `Button`, each
on a focus handle the view keeps (`trash_buttons`, through `track_focus`).
`trash::TrashFlow` holds the phase (idle, asking, running), the button the
dialog opens on (`initial_focus`: a move's run button, a delete's Cancel) and
what a key does (`key`); the focus itself is GPUI's.

- Shell (`modal::shell`): `.keyboard(false)`, since with the `Dialog` key
  context on, `enter` is bound to `Confirm` and runs `on_ok` whichever button
  has the focus, before any key listener; `.overlay_closable(false)`,
  `.close_button(false)`, `.entrance(DialogEntrance::None)` (no entrance or
  exit animation, so a close removes it in one frame) and `on_ok` /
  `on_cancel` returning false, so no path of the component's closes it
  behind the flow's back. `.overlay(false)` as well: the component's
  backdrop is a `WindowControlArea::Drag` area, which on Windows answers
  `WM_NCHITTEST` with `HTCAPTION` (`gpui_windows`' `events.rs`), so a press
  on it would drag the window and a double-click maximize it. The view draws
  `modal::backdrop` in its place while the flow is busy: a `deferred`
  `theme.overlay` layer under the dialog (the dialog's host is `deferred` at
  priority 10 and up) whose capture-phase mouse-down listener calls
  `prevent_default`, because the dialog's host tracks its own focus handle
  and would take the focus from the focused button on any press outside the
  buttons (the `Button`s already prevent it on their own press). The
  component's full-window `dialog` `div` occludes the window, so nothing
  under it is pressed, hovered or scrolled.
- Open and close (`modal::Modal`): the view calls `follow_trash` wherever the
  flow changes phase (the start, a dismissal, a run held for a scan's end,
  the run's end); it opens on the flow becoming busy and closes on it going
  idle, through `cx.defer` and the window's `AnyWindowHandle`, so a run's end
  reached from a background task without a `Window` takes the same path and
  the change lands before the next frame. The builder `open_dialog` takes is
  a render closure run on every rebuild of the dialog layer (every frame the
  window draws: `WindowState`'s view is not cached), so it reads the flow
  through a `WeakEntity` each time, and asking to running redraws the
  buttons disabled with the view's own `cx.notify()`. The initial focus is
  set on the builder's first run of each opening only (a `Cell<bool>` per
  opening), after `open_dialog` has focused the dialog's shell.
- Keys: a key pressed in the dialog never reaches the window's `key_down`
  (the dialog is outside the app's view), so the settings key and every
  other keymap key are refused there by construction; `key_down` still
  returns at once while the flow is busy. `Tab` / `Shift+Tab` are `Root`'s,
  walking the dialog's focus trap between the two buttons. Every other key
  reaches the footer's `capture_key_down`, which runs before the focused
  button's own listeners and asks `trash_key_down` (`TrashFlow::key` over
  `keys::key_name` and `KeyDownEvent::is_held`): Escape dismisses while
  asking; Enter and Space go on to the focused `Button`, which presses itself
  (GPUI's keyboard click: the key down recorded, the click fired on the key
  up with the focus unmoved, neither with a modifier); anything else, and
  Enter, Space and any repeat in a delete, is stopped with `prevent_default`
  and `stop_propagation`, so the button never records the key down.
- Rule: the delete dialog takes only a single mouse click on its run button
  as a yes. `ClickEvent::click_count` is 1 for a keyboard click at the
  pinned rev (`interactive.rs`), so the handler passes 0 when
  `is_keyboard()`, the DOM's `detail`, and `confirms_delete` refuses it and
  the rest of a double-click.
- Rule: while a run is in flight (not held) the dialog stays open with both
  buttons disabled. A disabled `Button` that has the focus keeps it (it
  leaves the tab order only), so Escape still reaches the footer's listener
  and does nothing; a Tab then finds no stop in the trap and `Root` focuses
  the dialog's host, where no key does anything either.
- Rule: a held `tab` is not swallowed in the delete dialog: `Root`'s binding
  is dispatched before any key listener, which alone sees `is_held`. It can
  only move the focus between the buttons; no key confirms a delete.
- Rule: the dialog gives the focus back on close to what had it when it
  opened, the app view's handle; the tree's keyboard is a flag in the view
  (`tree_keys`), which the dialog never touches.

### A trash or delete goes through the gate, and nothing lists under it (Inferred)

The native app has no `Scans` lock; one view thread decides when a listing
starts, so the guard is `idle::IdleGate` plus `resync.rs`. A confirmed run
asks the gate with "busy" being `Resync::busy` (a listing or its scan under
way) or a run in flight: idle, it runs at once on `background_spawn`
(`judgment::Queue::drain`, then `Writer::flush`, both `DRAIN_TIMEOUT`, then
`trash::trash_listed` with the view's `Arc<trash::Runs>` or
`trash::delete_listed`, progress through the view's event channel as
`Event::TrashProgress`); busy, it is held, the dialog closes and the scan is
canceled without a join (`Running::cancel`), so it still ends through `Done`
and `FacesDone`, whose `finished` drains the gate first. A run held while the
listing itself ran has no scan yet; `opened` cancels the scan it starts when
`cancels_scan(gate.waiting())`.

- Rule: while a run is in flight the gate's in-flight count is up and
  `Resync::hold` keeps the resync busy, so a watcher change or a tree
  `Refresh` is deferred; `relist` returns at once (a sidecar format switch);
  an `open` (the tree's) is kept in `deferred_open` and runs at the run's
  end. The run's end calls `Resync::open` (dropping the pending change) and
  re-lists once, which also rescans what a canceled scan left. `TreeChanged`
  lists a tree folder only, never the open one.
- Rule: every caller of `relist` (a sidecar format switch included) calls
  `Resync::open` first, so `Resync::busy` covers the listing it starts and a
  trash or delete waits for it.
- Rule: a folder open discards a held run; a held run that drains while a
  dialog is asking is dropped, as the Tauri closure checks `trashFlow.busy`.
- Rule: the run's result goes to the pane: `trashed_status` in the
  `step_note` line (the Tauri `note`), each failure through `write_failed`
  (`<name>: could not move to the Trash: <message>`), and after a delete the
  deleted files' judgments leave both histories (`Undo::without_paths`). A
  move whose `Summary::run_id` is set pushes its run on the undo history
  (see the next entry).
- Not verified headless: the dialog, its keys and clicks, the Recycle Bin
  and the cancel of a running pass 1; the hand checks are pending in
  `../plans/20261113-gpui-tree-trash/learnings.md`.

### A trash run is undone through the same gate, and outlives a folder switch (Inferred)

`history.rs`'s entry is `Entry::Judgments(Batch)` or `Entry::Trash(TrashEntry
{ id, run_id, count, dirs })`, the Tauri `undo.ts` `Entry`. A move that
recorded a run pushes it and clears the redo history (`record_trash`); a
redo's new run is pushed without clearing it (`redone_trash`), as a redone
batch is. `step` and `Undo::forget` see only batches: the view's `step`
looks at the top first (`top_trash`) and hands a run to `step_trash`, as the
Tauri `step` does, so `history::step` returns `None` with a run on top and
leaves it there.

- Rule: an undo or a redo of a run goes through `idle::IdleGate` as the run
  does, held under `Undo Move to Trash` / `Redo Move to Trash` while a
  listing or scan is under way and drained at `finished`, but it never
  cancels the scan (`cancels_scan` is false for those labels). The gate's
  slot is the `Held` enum (`Run`, `Undo`, `Redo`) in `main.rs`.
- Rule: the entry stays on its history until the backend answers
  (`trash_step` names the one out, and every undo or redo key does nothing
  meanwhile); a drained step runs only while its entry is still on top
  (`Undo::on_top`), and entries are removed by id (`remove_trash`), not
  popped, since judgments may have been pushed above it meanwhile. A
  refusal other than `RUN_GONE` / `UNDONE_GONE` (`trash::still_held`)
  leaves it where it was.
- Rule: the restore (`trash::undo_run`, which lists the Recycle Bin on
  Windows) runs on `background_spawn` with no progress; the redo
  (`trash::redo_run`) drains the judgment queue and flushes the writer first,
  as a run does, and reports through `Event::TrashProgress` and `trashed`.
  Both hold `Resync` and the gate's in-flight count, so nothing lists under
  them, and a tree open meanwhile waits in `deferred_open`.
- Rule: when the open folder is one of the run's `dirs` (`opens_target`),
  the restored files directly in it (`restored_into`, compared through
  `context::shown_path` as the backend spells them canonically) join `all` at
  once and `refilter` runs, so a second undo already reaches their
  judgments, and the folder is re-listed with the current file and the
  selection kept. Otherwise only a change seen meanwhile re-lists.
- Rule: another folder's open, and a sidecar format switch, drop only the
  judgment batches (`Undo::drop_judgments`); the runs stay, so a run made in
  one folder is undone from another, its files going back to theirs. A
  delete records no run and is not undoable.
- Not verified headless: the restore from the Recycle Bin and the redo; the
  hand checks are pending in `../plans/20261113-gpui-tree-trash/learnings.md`.

### The folder operations share one dialog model and live in `folderops.rs` (Inferred)

The tree menu's folder operations (so far `Rewrite Sidecars from Index…`,
`Delete Sidecars…`, `Sequence JPEG Timestamps…` and `Import…`) open one dialog,
`dialog::Dialog`: its `Content` (title, lines, rows, each with an optional
accent part and a muted flag, headed lists after the rows, notes, errors,
reason, running note), the run
button's label and `Style` (primary or destructive), which buttons are
enabled, and the button the focus goes to (`Dialog::focus`: the run button,
or Cancel when the run is disabled). A run that cannot be canceled disables
both buttons (`set_running`); a cancelable one keeps Cancel enabled and moves
the focus there (`set_running_cancelable`). `Dialog::key` gives Escape as
`Dismiss` (the flow decides: a non-cancelable run ignores it), lets Enter or
Space go on to the focused button while one is enabled, and swallows every
other key. The trash dialog was not moved onto it.

The dialog is a gpui-component `Dialog` in `Root`'s layer, built by
`folderops.rs`'s `folder_dialog` inside `modal.rs`'s shell exactly as the
trash dialog is (see "The trash dialog is a component `Dialog`"): the same
`Modal` plumbing (`folder_modal`), the same `modal::backdrop` drawn while
`folderops.dialog_open()`, and a footer `capture_key_down` asking
`folder_dialog_key` before the focused `Button` presses itself. The title is
the component's, the lines its content, the rows and lists its body, which
scrolls under the component's scrollbar when long, and the notes, errors,
reason and running note sit in the footer above the `primary()` or
`danger()` run `Button` and Cancel, on focus handles the view keeps
(`folder_buttons`).

- Rule: `follow_folder_dialog` runs wherever `folderops.dialog` changes: it
  opens the dialog when one is shown and closes it when none is. The
  import's stepping aside is a close with the flow alive (the focus goes
  back to the app's view), and `Import…` bringing it back a new opening.
- Rule: the builder focuses `Dialog::focus` on its first run of an opening
  and again whenever that changes (a `Cell` per opening), so a cancelable
  run's start moves the focus from the now disabled run button to Cancel
  and a Tab the user pressed is never undone. Tab / Shift+Tab are `Root`'s
  focus trap over the enabled buttons.

`folderops.rs` is a child module of `main.rs` holding `FolderOps` (the flow
under way and the open dialog) and an `impl Riffle` block: a child module
reaches the view's private fields, methods and color constants, so the
operations need no entity of their own and `main.rs` gains only the fields
(the flows, the `Modal`, the buttons' focus), the menu arm, the key guard,
the backdrop's condition and the gate's `Held` variant.

- Rule: one flow at a time: a start does nothing while the settings modal is
  open, the trash flow is busy or another folder flow is busy, and the trash
  dialog's start refuses while a folder flow is busy.
- Rule: a sidecar request goes through the trash's `IdleGate` as
  `Held::Sidecars` under `Rewrite Sidecars from Index` / `Delete Sidecars`;
  neither cancels the scan, the pane's waiting note comes from
  `gate.waiting()`, `finished` drains it into the preview, and an open drops
  it. The preview reads on `background_spawn` through the read-only
  connection; the run, refused with `SCAN_RUNNING` when `Resync::busy` or a
  run is in flight, holds the gate's in-flight count and the resync as a
  trash run does.
- Rule: a sidecar delete ends in `trashed` with `trash::What::Sidecars`: its
  run goes on the undo history as a `TrashEntry` whose `what` words the
  statuses (`Moved 2 sidecars to the Trash`, `Restored …`) and the gate
  labels (`Undo Delete Sidecars`); the shown judgments are cleared when the
  open folder was the target (`sidecars_trashed`). A sidecar run's undo
  counts the sidecars that did not fail, since `Restored::restored` lists
  RAWs and JPEGs only (`trash::restored_count`).
- Rule: `Sequence JPEG Timestamps…` neither goes through the gate nor
  touches the scan, the index or the resync, and needs no open folder. The
  preview reads on `background_spawn`; the run is a `std::thread` of its own
  (it can take minutes, so it does not hold a background executor thread)
  with an `AtomicBool` cancel flag, its progress sent into the view's channel
  as `Event::FolderOp(Message::SequenceProgress)` tagged with the run id
  `SequenceFlow::run` hands out before the run starts, so a late message of an
  ended run is dropped; the result comes back through a `oneshot` awaited by
  `cx.spawn`. The quit sets the flag through a `cx.on_app_quit` subscription
  kept only while the run is under way, and waits for nothing.
- Rule: `Import…` has the sequencer's shape (no gate, no scan, no index,
  no open folder needed; a `std::thread`, an `AtomicBool`, the quit's
  subscription), reads the root and the pattern from `Settings` (never
  writing them) and keeps the previewed `Plan` in `FolderOps`, so the run
  copies what the dialog showed. The thread sends the created folders
  through one `oneshot`, then the result through another, both awaited in
  order by one `cx.spawn`: the first folder opens through `open_from_tree`
  (which returns the open's `Task`), and the report waits for that task,
  since a fresh listing clears `write_errors` and the import's result line.
  Progress is sent only when the copied count changes, not per buffer. The
  dialog steps aside (`FolderOps::aside`) once the folders exist, `Import…`
  brings it back, Escape on it steps aside again and Cancel cancels. While
  the flow is busy, `FolderOps::busy` keeps the settings, the trash dialog
  and the other folder flows shut, as `importFlow.busy` does in the Tauri
  app; `import_busy` is the flag the folder rename refuses on.
- Rule: a quit mid-import sets the flag and waits for nothing: the copy
  removes the temp file of the file in flight when it sees the flag, but a
  process that exits first can leave that file under its `.handpic-tmp` name in
  the destination; the files already copied stay, and a second import skips
  them by name and size.
- Not verified headless: the dialog, its keys and the runs; the hand checks
  are pending in `../plans/20261113-gpui-folder-dialogs/learnings.md` and,
  for the component `Dialog`,
  `../plans/20261114-gpui-component-adoption/learnings.md`.

### A folder rename releases the watches, then follows in the tree and reopens (Measured)

`Rename by Capture Date…` (`datefolder.rs`, run from `folderops.rs`) renames
a folder to its one capture date through the backend's `rename_folder`, or
splits it into one sibling folder per date through `split_folder`, both on
`background_spawn` with the view's `Watch`, `TreeWatch`, write index and
`Runs` in a `rename::Context`. The backend releases the open folder's watcher
(`watch::release_under`, whose `settle` waits for notify's server to close the
handle) and the tree watches on the folder and under it (`with_released`,
holding the `TreeWatch` lock across the move) around the disk work, and puts
the watcher back when the rename fails or the split leaves the folder.

- Measured (`datefolder::watch_release`, a test in this crate so it runs on
  `crates/native/Cargo.lock`'s `notify` 8.2.2, on Windows): with the open
  folder's watcher on a folder and tree watches on it, its parent and its
  subfolder, a plain `std::fs::rename` of the folder fails; `rename_folder`
  renames it, a `watch::set` on the new path then reports a file written
  there within the debounce, and `split_folder` moves every file out and
  removes the folder while it is watched. Five runs in a row passed, each in
  about 0.5 s.
- Measured (a PowerShell check on Windows 11, NTFS): any open handle to a
  file inside a folder refuses the folder's rename, `FILE_SHARE_DELETE` or
  not. The native app's reads inside the open folder (the preview, a
  prefetch, the on-demand faces and eyes, the 1:1 crop) open, read and close
  the file, with no memory map, and the scan is refused before the run by
  the scan rule below, so the window is a read in flight at the moment of
  the rename; a rename refused that way reports the OS error in the status
  and leaves the folder, its watcher and its index rows as they were.
- Rule: the date rename is not held by the gate: with a listing, a scan or a
  run under way (`Resync::busy` or the gate's in-flight count) the preview
  and the run are refused with `SCAN_RUNNING` in the status. The run holds
  the in-flight count and the resync as a sidecar run does, so no listing
  or scan starts under it, and drains the judgment stores and the sidecar
  writer first.
- Rule: every folder rename ends in `folderops`' `folder_renamed`, a port of
  `main.ts`'s `folderRenamed`: `Tree::rename_folder` re-keys the node and
  everything under it with their state, the tree's open row and cursor
  follow (`tree::rebase` over the tree's spelling), the trash runs of both
  histories follow (`Undo::map_trash_dirs` over the canonical paths, which is
  how a run's `dirs` and `Riffle::dir` are spelled; the backend's
  `Runs::rename_dir` already moved the recorded runs), and the tree's watched
  set is handed over afresh. When the open folder is the renamed one or under
  it, it is reopened under the new path as a tree click opens it; the
  non-carry listing drops the judgment batches of the undo histories as any
  folder switch does, and the judgments and the last viewed file come back
  from the index rows `Index::rename_dir` carried, so nothing is
  re-extracted. A warning, and a split's status, are set once that open has
  landed, since its fresh listing clears the status.
- Rule: a split re-lists the parent in the tree (`tree_refresh`, which also
  hands the watched set over again, since the split released the tree
  watches under the folder). When the split folder was the open one it is
  re-listed if it remains (its watcher put back by the backend), else the
  first date folder opens; a folder the tree asked for during the run opens
  instead. A file that failed to move goes to the notes as `<name>: could
  not move: <message>`.
- Not verified headless: the dialog, the tree row following in place and
  the reopen keeping the strip's judgments and position; the hand check is
  pending in `../plans/20261113-gpui-folder-dialogs/learnings.md`.

### The tree's inline rename is a text input in the row, held by the gate (Inferred)

`Rename…` in the folder menu, or a slow second click on the open folder's
name, puts a fresh gpui-component `InputState` (drawn with `Input`) in place of the
row's name (`folderops`' `tree_name`), its text the name (or the pending one)
selected whole (`InputState::select_all`) and focused. The edit is keyed by the row's
path, not its index, so a re-listing of the tree keeps it with its text and
selection; `rename.rs` is the pure part (`commit`, `edit_outcome`,
`PendingRename`, `SlowClick`, `RenamesInFlight`).

- Rule: a live edit owns the keyboard. The `Input` takes its editing keys as
  its key context's actions (see "Text is typed through gpui-component's
  `Input`"); every other key goes on unstopped (on Windows a stopped key types nothing) and the window's
  `key_down` returns for it right after the dialogs' guards
  (`rename_owns_keys`), so neither the tree's keyboard nor the keymap sees
  it. A key the IME takes raises no `KeyDownEvent`, so composition never
  reaches a listener.
- Rule: the input's `InputEvent::PressEnter` or `Blur` (Enter, or the
  focus left) confirms and Escape cancels, once (`commit`); the edit and its subscription
  are dropped at once, so the lost focus that follows an Enter decides
  nothing. A confirm (`PressEnter` or `Blur`) while the window is inactive is ignored: GPUI blurs the
  focused element when the window deactivates (the focus path of an inactive
  frame is empty), and the edit waits for the window to come back as the
  Tauri blur checks `document.hasFocus()`. Any press outside the input
  focuses the window's root (`track_focus` on any button) and so confirms.
  After Enter or Escape the root is focused again and the tree has the
  keyboard.
- Inferred: a row scrolled out of the `uniform_list`'s range is not laid
  out, so a live edit scrolled away loses the focus and confirms; the Tauri
  input stays in the DOM. A confirm on a folder the tree no longer has does
  nothing.
- Rule: a confirmed name goes through the gate as `HeldOp::Rename` under
  `Rename…` (`gate_request`): run at once with no listing, scan or run under
  way, else held with its row showing the new name muted and italic after a
  clock, `PENDING_TITLE` on hover, until it runs, without canceling the scan.
  `gate_request`, which the trash, the undo and the sidecar requests use
  too, discards first, so a held rename replaced by another request has its
  mark cleared and says `Rename… was canceled` (not when a re-edit of the
  same folder replaced it); an open drops it the same way and puts the note
  in the status after its listing (`note_after`). Typing the real name back
  on the pending row discards it; an import under way refuses it, when
  confirmed and again when it drains.
- Rule: the run is the date rename's: in flight with `Resync::hold`, the
  stores and the writer drained, the backend's `rename_folder` with the
  canonical path computed before and after, then `folder_renamed`; a refusal
  (an invalid name, an existing entry) goes to the status. Until it settles
  the folder and everything under it is refused for an open from the tree
  (`open_from_tree`) and for a rename with `A folder is being renamed; try
  again in a moment.`.
- Rule: the slow click (`SLOW_CLICK_DELAY`, 500 ms) is armed by a single
  click on the open folder's name (not a root, no Ctrl / Shift) and fired by
  a `spawn_in` timer; any press in or outside the tree pane, any key and a
  second click (`click_count` 2) disarm it. That click no longer reopens the
  folder, as in the Tauri tree.
- The operations' rules during a scan, in one place: `Rename…`, `Rewrite
  Sidecars from Index…` and `Delete Sidecars…` are held by the gate and
  never cancel the scan; `Move to Trash` and `Delete Permanently` are held
  and cancel it; `Rename by Capture Date…` is refused with `SCAN_RUNNING`;
  `Sequence JPEG Timestamps…` and `Import…` ignore the scan.
- Not verified headless: the editor, its keys, the IME in it and the held
  rename; the hand check is pending in
  `../plans/20261113-gpui-folder-dialogs/learnings.md`.

## Timings

### `keypressToPixels` differs from the Tauri app's (Inferred)

The `page` line uses the Tauri line's `wait` / `read` / `decode` meanings,
collapses `ipc + post + worker` into `hop` (the time from the background result
to the update on the main thread), and splits `draw` into
`frameWait + paint + present`. `keypressToPixels` includes `present`; Tauri's
stops at `drawImage`. It starts at the latest press of the file that ended up
shown. `source=read|prefetch` says whether the image came from its own read or
the prefetch ring; a hit's `read`, `decode`, `orient` and `hop` are 0, and
its `wait` is the time `go` took before the hit.

### The meta pane is `div` rows, and its cost lands in `present` (Measured)

The right-hand pane (220 px, the Tauri `#info` width) lays out the file
name, `n / total`, the `meta.rs` groups and the notes as `div`s, built from
the `Metadata` that `load` formats on the background thread next to the
preview, so paging reads nothing new. Release, `--auto 20` on the 20-file ARW
compare folder, a fresh `--index` per run, before and after interleaved in one
session with other sessions building on the machine (CPU 12-31 % before each
run, so noisy): `paint` p50 0.70 / 0.82 ms before against 0.90 / 0.76 ms after,
`present` p50 0.19 / 0.22 against 0.44 / 0.41 ms, `keypressToPixels` p50
17.97 / 18.65 against 18.02 / 18.52 ms. `paint` times only the preview
canvas's closure; the pane's `div`s paint after it in the same frame, so their
cost shows as about +0.2 ms in `present`. The `div` rows were kept (the plan's
rule: a canvas only when `paint` grows by more than 1 ms).

- Rule: the view-only flag comes from the whole listing (`Listed.files`), not
  the `--size` / `--max` subset, and is set on every open and re-listing
  before the strip is built.
- Rule: a stage that times one canvas's closure does not see the rest of the
  element tree; look at `present` (and `frameWait` for layout) when adding
  elements beside it.

### A same-session A/B gates only work on the page path (Hit)

A same-session A/B needs the machine idle for 30 to 60 minutes, and other
sessions' builds spoil its runs (see the pass 2 section below). The user
decided on 2026-11-13 that it is needed only for work that can move the
pager's latency.

- Rule: a port plan includes the A/B latency gate only when it touches
  rendering (per-frame drawing, overlays, new drawn parts such as the status
  row, the strip's marks, the pane, menus' buttons), AI (pass 3 embeddings,
  genres, the ranking, the face and eye models), the preview (read, decode,
  orient, prefetch, the 1:1 view), the thumbnails (their read, decode and the
  scan that makes them), or background work that runs while paging (scan
  passes, watchers, folder reads on the open). Dialogs, context menus, file
  operations, the settings UI, code moves into the backend and fixes off the
  page path need no A/B. When in doubt, the plan says which side it is on and
  why.

### The strip costs the pager under 1 ms (Measured)

Same-session A/B, the pager with the strip, the index and pass 1 against the
pre-strip pager (`fb7fcdf4`), three runs each, 60 presses, warm index: median
p50 `keypressToPixels` +0.80 ms on ARW and +0.51 ms on RAF. `paint` did not
move; `present`'s p95 rose from about 0.2 to 0.55-0.85 ms on ARW. See
[the plan's results](../plans/_archived/20261110-gpui-strip-index/results.md).

- Rule: compare against a binary built in the same session, alternating; the
  pre-strip pager's ARW p50 was 3.5 ms lower in this session than the pager's
  in the port's Step 3 session, and `frameWait`'s p50 ranges over 6-10 ms across runs of one binary.

### Pass 2 costs paging nothing once it yields and runs small (Measured)

Same-session A/B, paging while pass 2 runs over the folder against paging on a
finished index, three runs each, 60 presses, pass 2 on `faces_threads()` (3 of
24 logical cores) and pausing at its stage boundaries while the preview holds
the scan's pause from its read until its image is painted. The median p50
`keypressToPixels` A - B was -2.10 ms on ARW 1616x1080 and -0.30 ms on RAF
4416x2944 (gate +2.0 / +5.0). At 22 pass 2 threads with only the read paused
it was +9.44 / +88.86 ms: `decode` about doubled and `paint` grew 4-6x.
See [the plan's results](../plans/_archived/20261110-gpui-focus-marks/results.md).

- Rule: check every run's `frameWait` p50 before reading an A/B. One session
  ran at 120-145 ms (presents at about 4 Hz) once the user left the machine
  and the display slept. Hold `SetThreadExecutionState(ES_CONTINUOUS |
  ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED)` from a helper process for the
  session; that changes no power setting.
- Rule: the B runs' `read` and `decode` came out slower than A's in every
  session after the first (ARW `read` about 2.2 against 0.5 ms). A's pass 2
  keeps the cores clocked up and re-reads the paged files through the hot
  list. So A - B can go negative; compare pairs, not one run.
- Rule: an `--auto` quit inside pass 2 cancels it, so the run logs
  `scan faces: ... canceled=true` with the `done` count it reached. A quit
  inside pass 1 logs `scan extract: ... canceled=true` and no `scan faces:`
  line.
- Rule: size `--auto N` below pass 2's length (N presses at `AUTO_INTERVAL`
  ~1 s plus `AUTO_SETTLE`), and use a fresh `--index` each run. A re-used
  index let pass 2 finish on its own (3,066 files took 41 s against
  `--auto 60`), so the run logged `canceled=false` and
  `quit: scan stopped=true in 0ms`, which proves nothing about the stop.
- Rule: to page while pass 2 runs, a 60-press run needs about 13k ARW or 4.5k
  RAF files to stay inside pass 2 (4.6 ms per ARW file, 15-16 ms per RAF file
  at 22 threads). Build the padding with hardlinks (`os.link`): no disk cost,
  and every copy stays warm in the page cache. Name the padding to sort after
  the compare files (`zzNNN_`) so `--size` / `--max 20` page only the 20.
- Rule: to turn `f` on in an `--auto` run without a code change, post
  WM_KEYDOWN / WM_CHAR / WM_KEYUP for `F` to the process's `MainWindowHandle`.
  `AppActivate` + `SendKeys` does not reach the app and may type into whatever
  window has focus, so never use it. Close a run by PID with
  `CloseMainWindow`; never for every `riffle-native` process, because other
  sessions run their own. `Get-Process riffle-native` returns every leftover
  instance, so take the one you started by PID (`Select-Object -First 1` only
  when you know it is the only one), or the `IntPtr` argument becomes an
  array. `.ps1` files are blocked by the execution policy here, so pass the
  script with `-Command`.
- Rule: other sessions' `mise run ci`, `cargo` and `riffle-native` processes
  spoil runs (17 of 25 B attempts and 1 of 11 A attempts were discarded in the
  Step 6 session of
  `docs/plans/_archived/20261110-gpui-focus-marks/learnings.md`). Wait for two
  clean load samples and no build process before each run, and discard a run
  with a load sample over 15 %. This reduces but does not prevent load
  arriving mid-run.
- Use forward slashes in Windows paths in Bash-tool scripts. A doubled
  backslash in a heredoc collapses to one and can escape the next `$`.

### The pane and the judgments cost the pager under 1 ms (Measured)

Same-session A/B, `eb46913b` (the judgments plan's Steps 1-5, plus the 1:1
view, the focus-marks overlay and pass 2 merged meanwhile) against the
pre-plan pager (`de249218`), three runs each, 60 presses, warm index: median
p50 `keypressToPixels` +0.85 ms on ARW and -1.45 ms on RAF, inside the
run-to-run spread (B's ARW runs alone span 18.75-21.96 ms). `paint` did not
move; `present`'s p50 was 0.07-0.10 ms higher in every A run. See
[the plan's results](../plans/_archived/20261110-gpui-judgments/results.md).

- Rule: a warm run must end the binary's last scan pass uncanceled (A's
  `scan faces:` line once pass 2 exists, not only `scan extract:`), so no
  timed run carries pass 2.
- Rule: check that no other session's `cargo` build is running before a
  measurement session; the first ARW session was void because one ran through
  its first half and A1 used up its retries.
- Rule: in a measurement script, write Windows paths with forward slashes.
  `"$I\\$b-$round.sqlite"` made bash turn `\$` into a literal `$`, so every
  run of a round shared one index file (`index$b-1.sqlite`) and the "after"
  runs were warm; check that each cold run's `--index` file name is distinct
  before trusting a table.

### The filter and the sort cost the pager nothing measurable (Measured)

Same-session A/B, `40e3ec57` (the filter-sort plan's Steps 1-4 only) against
`f080850a` (`main` right after that plan's own PR), three runs each, 60
presses, warm index, both on the same `--settings` file (no filter, `name`
order): median p50 `keypressToPixels` -2.63 ms on ARW and +0.46 ms on RAF,
inside the run-to-run spread (A's ARW runs alone span 19.28-22.11 ms).
`paint` and `present` stayed within 0.04 ms: the status row's two buttons,
unlike the pane, add nothing to `present`. See
[the plan's results](../plans/_archived/20261113-gpui-filter-sort/results.md).

- Rule: when B is the commit right after the plan's own PR, A - B is the
  plan's cost alone; prefer that over a pre-plan commit with other plans'
  merges in between, so a delta is not shared out across them.
- Rule: a deliberately quiet machine still spoils runs. 11 of 23 timed
  attempts were discarded for one to three load samples of 16-31 % mid-run with no build
  process running, and one slot passed only on its fourth and last try. Keep
  the 4-try cap per slot; a slot that uses them all voids the folder's session
  rather than relaxing the 15 % gate.
- Rule: a delta the size of the bound can be pure spread. ARW's three pairs
  were all negative (-2.97, -0.40, -2.35 ms) for code that does not run per
  page; RAF's changed sign. Check what the plan runs per page before reading
  a delta as a cost or a gain.

### `Root` costs the pager nothing measurable (Measured)

Same-session A/B, the window wrapped in gpui-component's `Root` (the
adoption plan's Step 5) against its parent commit `2595b1bb`, three runs
each, `--auto 60`, warm index, prefetch on (57 of 60 pages hit the ring in
every run): median p50 `keypressToPixels` -0.60 ms on ARW and +0.16 ms on
RAF, `frameWait` -0.76 / -0.26 ms, `paint` and `present` within 0.05 ms;
`frameWait`'s p50 stayed in 8.8-10.4 ms in every run. See
[the plan's learnings](../plans/20261114-gpui-component-adoption/learnings.md).

- Rule: the `page` lines are printed to stdout, not to the `--log` file; a
  measurement script reads the process's stdout.
- Rule: `tasklist` and `typeperf` print in the console's code page (cp932 on
  this machine); a Python sampler decodes them with `mbcs`, or its idle check
  silently sees no samples.

### During pass 1 the preview reads what the scan already read (Measured)

Stepping with the arrow through 520 cold ARW files while pass 1 ran (27.7 s),
the preview `read` was 0.96 ms p50 and 1.22 ms p95, against Tauri's 14-16 ms
p50 at `READ_SLOTS` 2. The hot list puts the current file and the strip's range
first, so the next file's head is usually cached before the press. A build
without the pause read the same at p50 / p95; its one 336 ms read came on a
slower disk and is not attributed.

- Rule: copying test files with `cp` leaves them in the OS cache (the second
  pass 1 over a 520-file copy took 0.41 s). For a cold pass 1 copy them with
  `robocopy /J` (unbuffered) and use each copy once.
- Rule: to measure the pause itself, press onto files the scan has not read
  (a jump, not the next file); `--auto` only steps to the next file.

### Peaking costs a held key nothing measurable (Measured)

Release, `→` held 5 s as injected key-downs every 33 ms, peaking off and on
alternated three times each in one idle session, fit view at 1275x852 / 1278x852:
median p50 `keypressToPixels` -1.75 ms on ARW (18.55 off, 16.80 on, n of
about 105 per run) and +1.11 ms on RAF (52.70 off, 53.81 on), against the plan's
+2.0 / +5.0 ms gate. ARW was faster with peaking on in every pair, through
`read` and `decode`, not investigated (presumably the busy core keeps the clock
up). On RAF a held key shows only 4-6 pages in 5 s whether peaking is on or off:
a preview load (about 40-45 ms) outlasts the key repeat, so the pager keeps
re-reading the newest file, and the overlay has few pages to compete with. See
[the plan's results](../plans/_archived/20261110-gpui-zoom-peaking/results.md).

- Rule: measure a background job's cost with a held key, not `--auto`, whose
  1003 ms interval lets every job finish before the next press; and check how
  many pages the hold actually showed before reading a p50.

### A prefetch hit leaves only the frame (Measured)

Release, `--auto 60` on the 20-file compare folders, warm index. Three runs
with the prefetch ring alternated with three under `--no-prefetch` in one
session (a busy machine, ARW noisy). Median p50 / p95 `keypressToPixels`:
ARW 12.82 / 20.10 ms against 23.40 / 34.05 ms, RAF 14.13 / 56.27 ms
against 66.15 / 78.95 ms. Every prefetch run hit 57 of 60 presses. The 3
misses are the wraps (no neighbour across the pager's wrap), and they set the
p95. `paint` did not move on a hit, and `present`'s p50 rose about 0.1 ms.
Starting the prefetch when the current result is shown therefore does not
slow the current page, so it stays there. Memory peaks on RAF: about
+110 MB working set and +300 MB private bytes. The cap counts 128 MB of BGRA
(two RAF neighbours are 104 MB). The rest is the prefetch's decode buffers
and the GPU textures that kept entries still hold. See
[the plan's learnings](../plans/_archived/20261114-gpui-prefetch/learnings.md).

- Rule: count hits from the `page` line's `source=prefetch`. A prefetch run's
  p95 is the wraps' reads; read the hits' latency from the p50.
- Rule: prefetch does not run while a scan pass runs. An A/B of the ring
  needs a warm index whose scan ends before the first press.
