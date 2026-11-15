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

# native:watch mise task

## Purpose

`mise run tauri:dev` rebuilds and restarts the Tauri app when its Rust sources
change; the GPUI app (`crates/native`) has only `native:dev`, which has to be
stopped and rerun by hand after every edit. A `native:watch` task gives the
GPUI app the same loop: it launches the app and, on a source change under
`crates/native`, `crates/core` or `crates/backend`, stops the app, rebuilds
and relaunches it. This is a rebuild and restart, not a hot reload (GPUI has
none).

## Steps

- [x] Step 1: Add the `native:watch` task and document it
  - Done when:
    - The root `mise.toml` has `[tasks."native:watch"]`, placed next to
      `native:dev`, which runs the debug build of the GPUI app under
      watchexec, with watchexec pinned to an exact version and installed by
      mise.
    - `mise run native:watch` launches the app; saving a `.rs` file under
      `crates/native`, `crates/core` or `crates/backend` stops the running
      app, rebuilds and relaunches it. Checked by hand on Windows (Git Bash),
      including that the relink does not fail with `os error 5` on
      `target/debug/riffle-native.exe`, and that a build's own output (under
      `crates/native/target`) does not trigger another restart.
    - `mise run native:watch -- <folder>` passes `<folder>` to the app, also
      for a path holding spaces, on the first launch and on every restart.
    - Ctrl+C in the terminal stops both watchexec and the app.
    - `CONTRIBUTING.md`'s native task list names `native:watch`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `mise.toml`, `CONTRIBUTING.md` (the `sh` block under
      "Native app (GPUI)", around line 78, one line in the style of the three
      there), and `docs/agents/native-app.md` only if the manual check turns
      up a rule worth recording (see below). `README.md` / `README.ja.md` do
      not list mise tasks and `CLAUDE.md` names only `native:bundle:macos` and
      `native:install:linux` in the `packaging/` description, so none of the
      three needs a change.
    - Mirror `native:dev`: `dir = "crates/native"`, `shell = "bash -c"`,
      `set -euo pipefail`, a `description` in the same voice (say that it
      restarts, not hot-reloads).
    - Install watchexec through the task-level `tools` field, next to the
      Rust pin: `tools = { rust = { version = "1.98.1", components =
      "rustfmt,clippy" }, watchexec = "<exact version>" }`. Not the top-level
      `[tools]`: `jdx/mise-action` in `.github/workflows/ci.yml` installs
      everything there in every CI job, and no check uses watchexec. mise's
      registry name is `watchexec` (`aqua:watchexec/watchexec`, fallback
      `cargo:watchexec-cli`); `mise ls-remote watchexec` ends at 2.8.0 today.
      Verify that mise 2026.9.5 on Windows installs it from a task-level
      `tools` holding two entries.
    - The watchexec line has to be the **last line** of the script, with
      `cargo run -- "$@"` at its end: mise appends the extra arguments,
      quoted, to the script's last line and `"$@"` itself is empty
      (`docs/plans/20261109-gpui-port/learnings.md`).
    - Run the command without a shell (`--shell=none`), so the arguments mise
      appends reach `cargo run --` as separate arguments instead of being
      re-joined through `cmd.exe` / PowerShell on Windows.
    - `--restart`, so a change stops the running `cargo run` (and the app
      under it) before the rerun. On Windows watchexec puts the command in a
      job object and terminates the job, which should take the
      `riffle-native.exe` child down with `cargo` and release the exe before
      the relink; measure that this holds (a surviving exe is the `os error 5`
      `docs/agents/native-app.md` describes). If the child survives, record
      what made it die in `learnings.md` and in the guide.
    - Watched paths, relative to the task dir. Prefer naming the crate's
      inputs (`src`, `build.rs`, `Cargo.toml`) plus `../core` and `../backend`
      over watching `.`: `crates/native/target` lives under `.`, and its
      `.fingerprint` holds `.json` files, so watching `.` depends on
      watchexec reading the root `.gitignore` (`crates/native/target`) from
      inside a git worktree, where `.git` is a file. `../core` and
      `../backend` have no `target` of their own (the root workspace's is at
      the repository root). If `.` is watched anyway, verify the ignore holds
      or add an explicit ignore of `target`.
    - Extension filter: `rs` and `toml`, plus `json`, because `riffle-core`
      compiles in `crates/core/i18n/*.json` (`build.rs`,
      `rerun-if-changed=i18n`), `genres/siglip2.json` and
      `ranking/siglip2.json` (`include_str!`). Left out as rarely edited by
      hand: the ONNX models under `crates/core/models`,
      `crates/app/icons/icon.ico` (the native `build.rs`) and `Cargo.lock`
      (see Trade-offs).
    - `mise.toml` is in `vp`'s format check (`toml` is one of
      `tools/ci/local.sh`'s `vp_extensions`), and a `mise.toml` change makes
      `ci:local` select every check.

## Trade-offs and risks

- **The restart is a hard kill.** watchexec ends the app with a job
  termination on Windows and SIGTERM on Unix, so `App::on_app_quit` (the
  sidecar writer's drain, the settings `Writes` drain, the window geometry)
  does not run on a restart. A judgment made just before saving a source file
  may stay a dirty index row until the folder is next opened, and settings
  written in that moment may be lost. `tauri dev` restarts the same way.
  Option: accept it for a dev task (taken here); the alternative, a graceful
  stop, has no portable form (Windows has no SIGTERM for a GUI process) and
  would need code in the app, which is out of scope.
- **`Cargo.lock` is not watched.** A lock change normally comes with a
  `Cargo.toml` change, which is watched, and `cargo run` itself rewrites
  `Cargo.lock` after a manifest edit, which would restart the build it just
  started. The alternative, adding `lock` to the extensions, catches a bare
  `cargo update` at the price of that double restart.
- **Task-level `tools` vs the top-level `[tools]`.** Task-level keeps CI from
  installing watchexec in every job and matches how the `native:*` tasks pin
  Rust. The price: `watchexec` is not on `PATH` outside the task, and the
  version is pinned in the task line rather than with the other tools.
- **`json` in the filter.** Without it the task is `rs,toml` only, and an edit
  of a label preset or a ranking prior needs a manual restart. With it, a
  watched `target` would loop, which is why the approach names the watched
  paths.
- **Unverified at planning time** (nothing was installed): that the aqua
  package installs on Windows under mise 2026.9.5, and that the job
  termination frees the exe. Both are in the step's Done-when as manual
  checks; if the first fails, `cargo:watchexec-cli` is the registry's other
  backend (a long source build).

## Progress

- (2026-11-15) Step 1 complete (the Ctrl+C manual check described in `learnings.md` is still pending)
