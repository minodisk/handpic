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

# Native app on the gpui-kit umbrella crate

## Purpose

`crates/native` depends on `gpui-pre`, `gpui-pre-platform`, `gpui-component`
and `gpui-kit-assets` one by one. The official GPUI Kit guides
(<https://gpui-kit.com/docs/installation/>,
<https://gpui-kit.com/docs/getting-started/>) have an application list
`gpui-kit` alone, import GPUI through `gpui_kit::*`, start through
`gpui_kit::init` / `gpui_kit::open_window`, and optimize the framework crates
in the dev profile. Following them leaves one dependency to bump, gives the
platform crate the features the Kit expects (`font-kit` for macOS text, the
X11 and Wayland backends on Linux) and makes `cargo run` debug builds render
at a usable speed.

## Decisions taken with the user

- The macOS and Linux behavior change the umbrella crate brings (see
  "Trade-offs and risks") is accepted as the intended outcome of following the
  guide.
- `gpui-kit` is pinned exactly, `"=0.9.1"`, following the repository's
  exact-pin rule rather than the guide's caret requirement.
- The dependency switch and the dev profile ship in one step.

## Steps

- [x] Step 1: Depend on `gpui-kit` alone, import through it, start through its `init` / `open_window`, and add the guide's dev profile
  - Done when:
    - `crates/native/Cargo.toml` has `gpui-kit = "=0.9.1"` and no direct
      `gpui-pre`, `gpui-pre-platform`, `gpui-component` or `gpui-kit-assets`
      entry, and `crates/native/Cargo.lock` is regenerated to match
    - No `use gpui::`, `use gpui_component::`, `use gpui_platform::`,
      `use gpui_kit_assets::` or fully-qualified `gpui_component::` path
      remains under `crates/native/src`
    - `crates/native/Cargo.toml` has the guide's `[profile.dev.package]` block
    - `CLAUDE.md` and `docs/agents/native-app.md` describe the new dependency,
      paths, startup and platform features
    - `cargo tree` in `crates/native` still shows one GPUI (`gpui-pre` at one
      version, no `gpui-fast*` package), and the two `[patch.crates-io]` forks
      are still the only git sources in `Cargo.lock`
    - `mise run ci:local` passes locally and the PR's GitHub checks pass (the
      `native` job on Windows, Linux and macOS, including the macOS bundle
      step)
  - Implementation approach (as far as it is known; omit if unknown):
    - **Manifest.** Replace the four dependencies with `gpui-kit = "=0.9.1"`
      (default features `component` + `assets`). Keep `[patch.crates-io]` as
      is. gpui-kit 0.9.1 depends on `gpui-pre` `=0.3.12`, `gpui-pre-platform`
      `=0.3.12` with `font-kit`, `x11`, `wayland` and `runtime_shaders`,
      `gpui-base` `0.9.1`, `gpui-component` `0.9.1` and `gpui-kit-assets`
      `0.9.1`, so the versions in the lock should not move; check that in the
      lock diff. The lock gains `gpui-kit` and the Linux X11 / Wayland crates
      that the two features turn on.
    - **What gpui-kit 0.9.1 re-exports** (read from its `src/lib.rs`):
      `gpui_kit::*` is all of `gpui` (`pub use ::gpui::*`, so
      `gpui_kit::prelude::*` and every type and function in the current
      `use gpui::{..}` lists resolve), `gpui_kit::platform` is
      `gpui_platform`, `gpui_kit::application` is `platform::application`,
      `gpui_kit::component` is `gpui_component`, `gpui_kit::assets` is
      `gpui_kit_assets`, `gpui_kit::base` is `gpui_base`.
    - **Imports.** Keep the explicit import lists; only the crate path
      changes: `use gpui::{..}` to `use gpui_kit::{..}`,
      `use gpui_component::{..}` to `use gpui_kit::component::{..}`,
      `use gpui_platform::application` to `use gpui_kit::application`,
      `use gpui_kit_assets::Assets` to `use gpui_kit::assets::Assets`, across
      every file under `crates/native/src`, their test modules included (grep
      for the current set; other native plans may have added files since this
      plan was written). Do not switch to a `use gpui_kit::*` glob.
    - **Macros.** `gpui_kit::actions!` / `derive(Action)` expand to bare
      `gpui::` paths (`gpui-pre-macros`), and no crate is called `gpui` any
      more. gpui-kit keeps a hidden `pub use crate as gpui` for this: bring it
      into any module using such a macro (`use gpui_kit::gpui;`). Known use:
      `actions!(test, [Focus])` in `main.rs`'s test module.
    - **Startup** (`main.rs`, the `application()` call near the end of
      `main`). Keep `.with_assets(icons::Icons)`: `Icons` stays the one
      `AssetSource` and keeps falling back to the Kit's `Assets`.
      `gpui_component::init(cx)` becomes `gpui_kit::init(cx)`, which with the
      `component` feature is exactly `gpui_component::init(cx)`. Keep the
      order of what follows it (key bindings, `theme::apply`).
    - **`gpui_kit::open_window` is adopted.** It is
      `cx.open_window(options, |window, cx| { let view = build(window, cx); cx.new(|cx| base::Root::new(view, window, cx)) })`
      and returns `Result<(AnyWindowHandle, Entity<V>)>`. `WindowOptions`
      passes through untouched, so the title, `window_bounds`, `display_id`
      and `app_id` stay. `gpui_base::Root` is the type `gpui_component::Root`
      re-exports, so the wrapper is the one built today. The build closure
      takes `(&mut Window, &mut App)`. The closure returns the view entity
      instead of the `Root`, `.unwrap()` stays, and `Root` leaves `main.rs`'s
      import list if nothing else names it.
    - **Dev profile.** Add to `crates/native/Cargo.toml` (its own workspace
      root), exactly as the guide lists:
      ```toml
      [profile.dev.package]
      gpui-pre = { opt-level = 3 }
      gpui-component = { opt-level = 3 }
      gpui-kit = { opt-level = 3 }
      gpui-kit-assets = { opt-level = 3 }
      gpui-pre-platform = { opt-level = 3 }
      rustybuzz = { opt-level = 3 }
      taffy = { opt-level = 3 }
      ttf-parser = { opt-level = 3 }
      ```
      Check Cargo prints no "did not match any packages" warning.
    - **Platform features change with the umbrella crate.** Today
      `cargo tree -e features -i gpui-pre-platform` shows only `default` /
      `multithreaded` (also with `--target x86_64-unknown-linux-gnu` and
      `--target aarch64-apple-darwin`). After the switch it must show
      `font-kit`, `x11`, `wayland` and `runtime_shaders`; rerun the command
      and record the output in `learnings.md`. None of the four reaches
      `gpui-pre-windows`, so Windows behaves as before.
    - **Linux CI packages.** The `native` job's apt step already installs
      `libfontconfig-dev`, `libwayland-dev`, `libx11-xcb-dev`,
      `libxkbcommon-x11-dev` and `libvulkan1`, but until now neither backend
      was compiled. Add a package to `.github/workflows/ci.yml` only when a
      build error names it, and keep the list in `native-app.md` equal to the
      step.
    - **Comments.** `theme.rs`'s doc comment on `apply` ("Runs after
      `gpui_component::init`") and `icons.rs`'s module comment
      (`gpui_kit_assets::Assets`) name the new paths.
    - **`CLAUDE.md`.** In the `crates/native` description: the
      `gpui_component::init` mention (the `src/theme.rs` clause) and
      "`gpui_kit_assets::Assets`" (the `src/icons.rs` clause).
    - **`docs/agents/native-app.md`.** Rewrite the sections on: GPUI being the
      `gpui-pre` snapshot gpui-component pins (now `gpui-kit` pins it; a bump is
      a `gpui-kit` bump); gpui-component's initialization (`gpui_kit::init`,
      and the paragraph saying no `runtime_shaders` is on no longer holds); the
      `Icons` fallback (`gpui_kit::assets::Assets`); `Root` wrapping the
      window's content (now done by `gpui_kit::open_window`); macOS compiling
      shaders with `xcrun metal` at build time (`runtime_shaders` is now on);
      Linux system libraries (the backends are now compiled). Record as
      Inferred (read from manifests and sources, not run on those platforms)
      that before this change macOS had no `font-kit` and Linux no `x11` /
      `wayland` backend.
    - **Local verification.** `mise run ci:local`. Regenerating the lock needs
      network once. On Windows, launch `mise run native:dev` once and confirm
      the window restores its place, the icons and the settings modal's
      controls draw, and `tab` behaves as before.
    - **Not in this step.** `crates/app` is frozen and untouched. The
      `gpui-fast` feature stays off.

## Trade-offs and risks

- **The switch is not behavior-neutral on macOS and Linux.** gpui-kit turns
  `font-kit`, `x11`, `wayland` and `runtime_shaders` on unconditionally. Read
  from the manifests and sources: today's macOS build has no `font-kit`, so it
  draws no text; today's Linux build has neither backend. After the switch
  macOS renders text, Linux gets X11 and Wayland windows, and macOS compiles
  its shaders at startup instead of at build time. Accepted by the user.
  Neither platform can be checked on the development host; a hand check on a
  Mac and a Linux session stays pending.
- **Exact pin.** With `"=0.9.1"`, `gpui-pre` and `gpui-pre-platform` stay
  exact through the Kit, while `gpui-component`, `gpui-base` and
  `gpui-kit-assets` are caret requirements inside the Kit, held by
  `Cargo.lock`.
- **The dev profile lengthens cold builds**, and the Linux leg now also
  compiles the X11 and Wayland stacks; the `native` CI job has a 30-minute
  timeout. If a leg times out, raise the timeout or move the profile block to
  a PR of its own.
- **Conflicts with the in-flight native plans**, which edit the same `use`
  lines. Whichever merges second rebases its imports.
- **Not adopted: `use gpui_kit::*`.** The app keeps its explicit lists.

## Progress

- 2026-11-16: Step 1 shipped: `crates/native` depends on the `gpui-kit` umbrella crate alone, with the GPUI crates optimized in the dev profile. Manual checks on Windows, macOS and Linux are pending (see `learnings.md`).
