# Learnings: gpui-port

## Step 1

- NASM: winget's `NASM.NASM` package installs per user, to
  `%LOCALAPPDATA%\bin\NASM` (here `C:\Users\daisu\AppData\Local\bin\NASM`), not
  under `C:\Program Files\NASM`, and is not added to `PATH` automatically. The
  machine `PATH` lists the stale `C:\Program Files\NASM`. Exact fix: put the
  per-user directory on `PATH` for every build of the crate. `CONTRIBUTING.md`
  says so.
- mise 2026.9.5 honors the task-level `tools` field. With
  `dir = "crates/native"` and `tools = { rust = ... }`, `cargo` is 1.98.1 and
  `RUSTUP_TOOLCHAIN` is `1.98.1`, also while `crates/native/mise.toml` is
  untrusted, which a `cd` and `mise exec` would depend on. The `native:*` tasks
  therefore use `tools`. The nested `mise.toml` still gives plain `cargo` inside
  the crate the same pin.
- `mise run <task> -- args`: mise appends the arguments, quoted, to the **last
  line** of a `shell = "bash -c"` task's script, and `"$@"` itself is empty
  (checked with a path holding spaces and backslashes). The `cargo run` line has
  to stay last in `native:dev` and `native:release`.
- GPUI at this rev also passes `cargo check --locked` on Rust 1.97.1. The nested
  1.98.1 pin stays so the crate tracks Zed's.
- A first `cargo check` including the clone of Zed took 1 min 48 s on this
  machine; `~/.cargo/git/db/zed-*` is 408 MB.
- `.gitignore` needed `crates/native/target`, which the plan's Done-when list
  does not name: the existing `/target` matches only the root.
- `build.rs` was exercised by hand: no NASM fails the build, `NASM` set to a real
  binary without `nasm` on `PATH` fails it (round 1 review fix), `nasm` on `PATH`
  passes, `RIFFLE_NATIVE_ALLOW_NO_SIMD=1` gives the
  warning.
- The window was checked on Windows: the title is `Riffle`, a real `q` key event
  and closing the window both end the process. `SendKeys` from a background
  PowerShell did not reach the window; a `keybd_event` after taking the
  foreground did.

## Deferred issues (todo candidates)

- **Pending check: the first `native` CI job on GitHub.** What to verify: on
  `windows-latest`, the job passes, `mise run native:check` installs Rust 1.98.1
  through the task-level `tools` under CI's mise 2026.9.0 (local verification
  was on 2026.9.5 with 1.98.1 already installed), and whether NASM was on the
  image or came from `ilammy/setup-nasm`. Record the cold run's duration and
  raise the job's timeout if it exceeds 30 minutes. Step 1's checkbox was ticked
  on the criteria verifiable locally.
