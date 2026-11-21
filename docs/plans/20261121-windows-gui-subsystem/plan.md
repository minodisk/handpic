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

# Windows GUI subsystem for the release app

## Purpose

The Windows release `handpic.exe` is a console-subsystem binary (PE
`Subsystem` = 3), so every launch from Explorer, the Start menu or an updater
relaunch opens a terminal that streams the log lines. The updater rehearsal
([native-updater plan](../_archived/20261118-native-updater/plan.md), Step 7) showed it
on each relaunch of rc.2, portable ZIP and Inno Setup alike. Once this is
done, a Windows release build is a GUI-subsystem binary that opens no console,
while debug builds keep theirs and `Handpic.log` keeps every line. This must
land before the release PR #928.

## Steps

- [x] Step 1: Build the Windows release app for the GUI subsystem
  - Done when:
    - `crates/app/src/main.rs` carries
      `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`
      as its first line; debug builds (`mise run app:dev`, `cargo test`) stay
      console binaries.
    - `crates/app/src/logger.rs`'s three `eprintln!`s become
      `let _ = writeln!(io::stderr(), ..)`, so a missing or broken stderr never
      panics the logger (which the panic hook also calls).
    - A release build made locally on Windows (`cargo build --release --locked`
      in `crates/app` with the app toolchain, as `mise run app:package:windows`
      does) reports `Subsystem` = 2: the UInt16 at `e_lfanew + 0x5c`, e.g.
      `$b=[IO.File]::ReadAllBytes($exe); $o=[BitConverter]::ToInt32($b,0x3c); [BitConverter]::ToUInt16($b,$o+0x5c)`.
      A debug build reports 3.
    - That release exe, started with no console (double-click in Explorer, or
      `Start-Process` from PowerShell) with `--log`, `--index` and `--settings`
      pointing at temporary paths, opens no console window, and the `--log`
      file holds the launch's lines (the `settings:` line and the scan lines
      of an opened folder).
    - A panic in that state still leaves its `panic:` line in the log and
      causes no secondary failure. Either provoke one in a throwaway build
      (not committed), or state in `learnings.md` that this rests on the
      planning measurement below.
    - Timing output is measured on the release build and the result recorded
      in `docs/agents/app.md`: whether the `page` lines and the quit summary
      of `mise run app:release -- <folder> --auto 5` reach (a) a Git Bash
      terminal, (b) PowerShell / cmd in Windows Terminal through
      `mise run app:release`, and (c) `handpic.exe` started directly from
      PowerShell, unredirected and redirected (`> file`, `| Out-Host`).
    - `docs/agents/app.md` has a section under "## Packaging" on the subsystem
      choice (the attribute and its condition, why debug keeps the console,
      the null std handles being a sink, what a console launch of a release
      build does and does not show, how to read the PE field), and its "The
      engine logs through `log`, printed by `logger.rs`" section and
      `docs/agents/app-log.md` (lines about stderr) say that a Windows release
      build has no stderr unless redirected.
    - If measurement (b) or (c) shows the timing lines no longer reach an
      unredirected console, `docs/humans/performance.md` and
      `docs/humans/performance.ja.md` (the "run Handpic from a terminal"
      paragraph) say how to get them on Windows (redirect, or Git Bash), in
      the same PR.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/app/src/main.rs` (one crate attribute above `mod args;`),
      `crates/app/src/logger.rs`, `docs/agents/app.md`,
      `docs/agents/app-log.md`; `docs/humans/performance.md` + `.ja.md` only
      if the measurement calls for it. Do not touch
      `crates/engine/src/update.rs`, `crates/app/src/update.rs` or the
      `native-updater-step-7` branch.
    - `not(debug_assertions)` is the repository's existing meaning of "release
      build" (`main.rs` passes `!cfg!(debug_assertions)` to the update flow),
      and `[profile.release]` in `crates/app/Cargo.toml` does not turn debug
      assertions on, so the condition selects exactly the release profile.
    - Measured at planning time with a throwaway program (rustc 1.97.1; the
      app pins 1.98.1): with `#![windows_subsystem = "windows"]` the PE field
      is 2; started with no std handles (`Start-Process`), `println!`,
      `eprintln!` and `writeln!(io::stderr(), ..)` all return normally and the
      program continues; a panic there exits 101 with no secondary failure.
      Started from Git Bash (pipes), redirected or not, stdout and stderr
      still arrive. This matches std's documented behaviour (a null std handle
      of a `windows_subsystem = "windows"` process is a sink). So the usage
      error in `main` and the `println!` timing lines need no change for the
      no-console case.
    - The panic hook in `crates/engine/src/diagnostics.rs` writes through
      `log::error!` (so into `Handpic.log`) before the default hook writes to
      stderr; it needs no change. Every `println!` / `eprintln!` in
      `crates/engine/src` is test-only.
    - Nothing in CI depends on the release exe's console:
      `.github/workflows/package.yml`'s Windows check starts the setup with
      `Start-Process -PassThru` and detects the relaunched app by its process
      or the log file's mtime; `ci.yml`, `tools/ci/local.sh` and
      `mise run app:check` only build and test debug; `tools/package/*.sh`
      never run the exe. No workflow change is required.
    - The app starts no console child on Windows (the only `Command::new`
      there is the detached Inno Setup setup), so the GUI subsystem brings no
      new console flashes from children.
    - Not measured at planning time: an unredirected launch from a real
      console (cmd / PowerShell / Windows Terminal), including through
      `mise run app:release` (mise, `bash -c`, `cargo run --release`). A GUI
      process does not attach to the parent's console, PowerShell and
      interactive cmd do not wait for it, and its unredirected stdout may be
      lost. Measure it and write what holds; do not assume.
    - The usage error (`eprintln!` + `process::exit(2)`) keeps its exit code;
      its text is visible only where stderr is attached. Record this in the
      doc section.

## Trade-offs and risks

Decided with the user at approval (the recommended options):

- Logger hardening: taken. The null-handle case does not need it (measured),
  but stderr inherited as a pipe whose reader has gone makes `eprintln!`
  panic, and the logger is called from the panic hook.
- Timing lines on a Windows console: if unredirected stdout of the release
  build is lost from PowerShell / cmd, document the redirect / Git Bash route
  rather than adding `AttachConsole(ATTACH_PARENT_PROCESS)` or a
  console-subsystem measurement build.
- No `Subsystem` = 2 assertion in `.github/workflows/package.yml`: it is
  outside the acceptance criteria and that workflow is `workflow_dispatch`
  only, so a PR's checks cannot exercise it.

Accepted consequences:

- A release-profile test run (`cargo test --release` in `crates/app`) would
  build the test harness as a GUI binary on Windows too, since the attribute
  is on the crate root. Nothing runs tests in release today; a piped runner
  would still receive the output.
- A user who starts the release exe from PowerShell / cmd gets the prompt back
  at once and sees no usage error for a bad flag (exit code 2 is kept). This
  is the normal behaviour of a Windows GUI app.

## Progress
