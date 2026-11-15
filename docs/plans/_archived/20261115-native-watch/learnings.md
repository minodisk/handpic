# Learnings

## Step 1: the `native:watch` task

- mise 2026.9.5 on Windows installed watchexec 2.8.0 from the task-level
  `tools` holding two entries (rust and watchexec) on the first
  `mise run native:watch`, in about 3 s
  (`watchexec-2.8.0-x86_64-pc-windows-msvc.zip`, the aqua backend). The
  top-level `[tools]` stays without it.
- Checked by hand on Windows (Git Bash), run non-interactively under
  `timeout`:
  - The first `cargo run` (cold `crates/native/target`, 1m 53s) launched the
    app; the build's own output under `crates/native/target` triggered no
    restart (one `[Running: ...]` line until a source was touched, and none
    in the 30 s idle after each rebuild).
  - `touch crates/native/src/main.rs` stopped the app, rebuilt (8.9 s) and
    relaunched it; `touch crates/backend/src/text.rs` did the same (16.5 s)
    through the `../backend` watch. No `os error 5` on the relink: the old
    `riffle-native.exe` was gone before the link, so watchexec's job
    termination takes the app down with `cargo`.
  - `mise run native:watch -- "<scratch>/dir with space"`: the app's command
    line held the path as one argument (`Get-CimInstance Win32_Process`), and
    its scan logged `dir=...\dir with space`, on the first launch and on both
    restarts. `--shell=none` passes mise's appended quoted argument through
    intact.
- A forced kill of `watchexec.exe` (`taskkill /F`) leaves `cargo` and the app
  running: the job object is not kill-on-close. Ctrl+C is a different path
  (the console's ctrl event reaches every process attached to it), which could
  not be sent from the agent's shell, so it stays a manual check below.
- Another `riffle-native.exe` from the main checkout (its own
  `crates/native/target`) can run at the same time; a worktree's build has its
  own exe and does not collide with it. Pick processes by `Path` before
  killing anything.

## Deferred issues (todo candidates)

- Pending manual check (Windows, Git Bash; the step's checkbox was ticked on
  the automated criteria and the checks above): run `mise run native:watch`
  in a terminal, wait for the app's window, then press Ctrl+C in that
  terminal. Expected: watchexec, `cargo` and `riffle-native.exe` all exit
  (none left in Task Manager) and the prompt returns. Also confirm the window
  actually appears and repaints after a restart (the agent saw only the
  process and its log). Basis: plan Step 1 Done-when "Ctrl+C in the terminal
  stops both watchexec and the app"; files `mise.toml`
  (`[tasks."native:watch"]`).
