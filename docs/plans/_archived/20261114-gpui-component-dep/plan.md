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

# gpui-component as a dependency of the GPUI app

## Purpose

The native app in `crates/native` draws every control by hand: the popovers of
`menu.rs` and `context.rs`, the text input of `textinput.rs`, the dialogs and
the folder tree. [gpui-component](https://github.com/longbridge/gpui-kit)
(crates.io `gpui-component` 0.9.1) offers those as ready components.

This plan only brings the crate into the dependency graph. No existing UI is
replaced here, and `gpui_component::init` is not called yet (the user's
decision, 2026-11-14): calling it, and checking what its global `Theme` and key
bindings do to the app's keymap, belongs to the first feature that adopts a
component. Once this is merged, such a feature can `use gpui_component::...`
without touching the toolchain, the GPUI pin or CI.

gpui-component does not build against Zed's git repository. It pins
`gpui-pre =0.3.12`, a crates.io snapshot of Zed's GPUI at `zed@d2a4fb6`. To
keep one GPUI in the build (two GPUIs would have distinct types, so no
gpui-component element could join the app's views), the native crate's own
`gpui` and `gpui_platform` move from the Zed git rev `b5cd310b` to the same
`gpui-pre` snapshot. The user accepted this source change (GPUI from the
gpui-kit maintainer's crates.io snapshots instead of Zed's repository) on
2026-11-14.

## Steps

- [x] Step 1: Move the native app's GPUI to the `gpui-pre` snapshot and add
      `gpui-component`
  - Done when:
    - `crates/native/Cargo.toml` depends on `gpui-component` `=0.9.1`, and its
      `gpui` and `gpui_platform` are `gpui-pre` and `gpui-pre-platform` at
      `=0.3.12`, renamed with `package =`, so no `use gpui::...` or
      `use gpui_platform::...` line changes.
    - `cargo tree` in `crates/native` shows exactly one GPUI: no package comes
      from `git+https://github.com/zed-industries/zed`, and `gpui-pre` appears
      at one version. `crates/native/Cargo.lock` is committed with that
      resolution.
    - No source file uses gpui-component and nothing calls
      `gpui_component::init`; the crate is compiled, not run.
    - The app runs as before on the developer's machine: a folder opens,
      paging, the judgment keys, the sort / filter menus, the right-click
      menus, the settings modal and the tree's keyboard behave as on `main`.
    - `mise run native:check` passes locally, and the `native` CI job passes
      on Windows, Linux and macOS.
    - `docs/agents/native-app.md` records the pin rule and the license notice
      decision (see the approach below).
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/Cargo.toml`, `crates/native/Cargo.lock`,
      `docs/agents/native-app.md`. Touch `.github/workflows/ci.yml` only if a
      build error or the timeout forces it. Other work is in flight on
      `crates/native` (folder dialogs, pane toggle keys), so change no source
      file unless the compiler forces it, and expect a `Cargo.lock` conflict
      with any of them that adds a dependency: resolve it by re-running the
      resolution, not by hand.
    - Dependencies:
      `gpui = { package = "gpui-pre", version = "=0.3.12" }`,
      `gpui_platform = { package = "gpui-pre-platform", version = "=0.3.12" }`,
      `gpui-component = "=0.9.1"`. An exact pin on gpui-component keeps a
      `cargo update` from moving it to a release that pins another `gpui-pre`
      and so fails to resolve against ours.
    - If a lint (`unused_crate_dependencies` or similar) or `cargo machete`-like
      check in `native:check` rejects an unused dependency, allow it with the
      narrowest exception and a comment naming this plan, rather than adding a
      call.
    - Features: add none. Today's git deps use default features, and the
      defaults of the snapshot are the same (`gpui-pre`: `font-kit`,
      `wayland`, `x11`, `windows-manifest`; `gpui-pre-platform`:
      `multithreaded`; `gpui-pre-linux`: `wayland`, `x11`). gpui-kit's own
      workspace also turns on `runtime_shaders` on the platform crate; do not
      copy it, the guide's rule on `xcrun metal` says to keep it off.
      gpui-component, `gpui-base` and `gpui-kit-assets` do not depend on
      `gpui-pre-platform`, so nothing turns it on behind our back; confirm
      with `cargo tree -e features -i gpui-pre-platform`.
    - Leave gpui-component's optional features (`tree-sitter*`, `syntect`,
      `speech`, `inspector`, `decimal`, `gpui-fast`) off.
    - Keep `[patch.crates-io]` (`async-task`, `async-process`) as it is.
      `gpui-pre` takes both from crates.io, so the patch still applies, and
      Zed at `d2a4fb6` carries the same two revs. After the change these two
      are expected to be the only git sources left in the lock; the Zed forks
      of `oo7`, `wasm_thread` and `font-kit` that the git GPUI pulled in
      should be gone. Record what the lock actually shows.
    - The toolchain stays Rust 1.98.1: Zed at `d2a4fb6` still pins it, and the
      published crates declare no `rust-version`.
    - The two Zed commits between `b5cd310b` and `d2a4fb6` touch no
      `crates/gpui*` file, so no source change is expected. If the snapshot
      does differ (a compile error, a changed signature), fix only what the
      compiler names and record it in the guide as an API change met at the
      bump, per the existing rule.
    - Linux: `gpui-base` adds `ashpd` with the `settings` feature. Add an apt
      package to the CI job only when a build error names it, and keep the
      guide's list equal to the step, per the existing rule.
    - CI time: the lock hash changes, so each leg's first run is close to a
      cold build with a larger graph (two versions of the `windows` crate,
      `markdown`, `lsp-types`, `ropey`, `html5ever`, a second `resvg`).
      Compare the job time against `timeout-minutes: 30` and raise the
      timeout only if a leg actually hits it.
    - `docs/agents/native-app.md`, Toolchain section:
      - Rewrite "GPUI comes from one pinned Zed rev" to the new rule: GPUI is
        the `gpui-pre` / `gpui-pre-platform` crates.io snapshot at the exact
        version gpui-component pins (`=0.3.12`, `zed@d2a4fb6`, for
        gpui-component 0.9.1), renamed so the code keeps the `gpui` names.
        The native crate never picks a GPUI version of its own: an upgrade
        moves `gpui-component`, `gpui-pre` and `gpui-pre-platform` together in
        one deliberate chore PR, and records the API changes met there.
        `cargo tree` must show one GPUI. Say why: two GPUIs have distinct
        types, and `[patch]` cannot swap `gpui-pre` for Zed's `gpui` because
        the package names differ.
      - Update "Only two of Zed's patches are needed" to say the patches now
        apply to the crates.io `gpui-pre` dependencies.
      - Add an entry on gpui-component: it is a dependency only, nothing calls
        `init` yet; `init` sets a global `Theme`, registers context-scoped
        key bindings (`gpui-base`'s `Root` binds `tab`, `shift-tab`,
        `cmd-c` / `ctrl-c`) and in debug builds a global `ctrl-shift-i` /
        `cmd-alt-i` inspector binding, the first time anything enters GPUI's
        keymap here; the first feature adopting a component calls it and
        checks those against the app's keymap, and decides on `Root` and the
        icon assets.
      - Record the license notice decision: gpui-component and the
        `gpui-pre-*` crates are Apache-2.0 (`LICENSE-APACHE`, Copyright
        Longbridge; GitHub shows NOASSERTION only for the file name), and
        `gpui-kit-assets` embeds Lucide (ISC). No notice file now, since the
        native app has never shipped; the notices are due when it is
        packaged.
      - Tag each entry **Hit**, **Measured** or **Inferred** as the guide does.
    - `CLAUDE.md`'s Layout section does not name the GPUI pin; leave it
      unless the change makes one of its sentences wrong.
    - Commit type: `build(native): ...` or `chore(native): ...`, in English.

## Trade-offs and risks

- **No `init` now (decided).** The crate is compiled on all three platforms
  but never run, so this PR changes no behaviour. The first adoption feature
  inherits the `init` questions (global `Theme`, the first entries in GPUI's
  keymap, the debug-only global inspector binding).
- **GPUI's source moves from Zed's repository to a third-party snapshot
  (accepted).** The `gpui-pre-*` crates are published on crates.io by the
  gpui-kit maintainer (a single owner), not by Zed, described as a snapshot of
  `zed@d2a4fb6`; the metadata cannot prove they are unmodified. In exchange
  the build no longer clones Zed's repository.
- **The upgrade cadence is no longer ours.** `gpui-pre` is released weekly and
  gpui-component pins one exact snapshot. A GPUI fix can only be taken when a
  gpui-component release pins a snapshot that has it.
- **`gpui-component` directly, not the `gpui-kit` umbrella.** Keeps every
  `use gpui::...` line, at the price of keeping the pin in step by hand (the
  guide's rule).
- **Keeping Zed's `async-task` / `async-process` forks.** Today's behaviour and
  Zed's; gpui-kit tests against the crates.io versions. Dropping them is not
  measured and stays out of scope.
- **Build time and CI.** The graph grows; the `native` CI job's first run after
  the lock change is nearly cold against a 30-minute timeout.
- **Parallel work.** Any branch that adds a dependency to `crates/native`
  conflicts with this one in `Cargo.lock`; resolve by re-resolving.

## Progress

- 2026-11-14: Step 1 done. GPUI moved to `gpui-pre` / `gpui-pre-platform`
  `=0.3.12` (renamed with `package =`), `gpui-component` `=0.9.1` added as a
  dependency only, no source change. `native:check` passed locally. The manual
  run check and the CI legs are still pending, as recorded in `learnings.md`.
