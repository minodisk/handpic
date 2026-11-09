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

## Step 2

- The plan's budget for `orient` (<= 15 ms on 4000x3000) could not be met by
  either loop it named: a plain loop took 62 ms and the tiled one about 21 ms, a
  fixed cost of about 8 ms being the output's first touch. Tiles over 8
  `thread::scope` threads take about 10 ms (measured numbers in
  `docs/agents/native-app.md`). The thread pool is the one departure from
  "the simpler loop"; the plan's other fallback (rotate after a scaled decode)
  stays for Step 5.
- The spike's `load` did not guard mozjpeg, which unwinds on corrupt input;
  `guarded` catches that, or an unreadable file would leave the request in
  flight. A garbage `.ARW` shows `error: …` and paging continued in the run.
- Checked by running the release build on Windows: the keys (`right`, `left`,
  `home`, `end`, `q`, plus non-extended `VK_RIGHT` / `VK_LEFT` for the keypad)
  through injected `keybd_event` events, the wraparound, the error line, the
  folder picker (`cx.prompt_for_paths`; Enter on a typed path navigates, a
  second Enter selects), and 6 / 8 orientation on ARW and RAF portrait files by
  screenshot: every file came up upright. Rapid presses (25 at 30 ms) did not
  hang and painted only the frames that were still current.
- Not confirmed: moire on the RAF fit view. One 1:1 crop of a 4416x2944 RAF fit
  shows none, but it is a low-detail frame and nothing was compared with the
  Tauri app; Step 3 owns that check. No orientation-3 sample turned up in the
  ARW / RAF samples, so 3 rests on its unit test.
- Timings here are from small, uncontrolled runs (n=4, cold cache for the first
  read), so they are not the Step 3 measurement.
- `hop` measures only `received - finished`, as the spike's did: the time a
  background task sits queued before it starts is in neither `hop` nor `read`.

## Step 3

- Threshold missed on both folders (ARW 20.93 ms against <= 17.6, RAF 64.76 ms
  against <= 62.9): see `results.md`. Step 3's checkbox stays unticked; the plan
  reopens Step 2 before Step 4.
- The spike's `read` of 0.74 ms was **not reproduced**: ARW `read` p50 was
  1.34-1.96 ms in the three runs used (0.89-1.20 in the discarded looser pass),
  on the 20 warmed files. RAF was 3.6 ms against the spike's 1.80. The cause is
  not known here.
- `frameWait` swung 5.99-11.48 ms between ARW runs on the same files, about the
  size of the ARW budget. n=20 runs of a vsync-phase-dependent stage cannot
  hold a 2 ms margin.
- A debug `riffle-app` and its `cargo run` parent stayed up during every run. A
  gate on "no cargo" would never have opened, so the gate is `rustc` plus load;
  the running app is written into the conditions.
- `Win32_Processor.LoadPercentage` can come back empty; the sampling has to
  retry.

## Deferred issues (todo candidates)

- **Pending manual check: the RAF fit view next to the Tauri app.** What to
  verify: on Windows, one RAF with fine repeating texture from
  `D:\Photos\tests\20261108-gpui-compare\RAF` shown in the fit view of both
  `riffle-native` and the Tauri app at about the same drawn size, and whether
  the native one-mip downscale shows moire the Tauri one does not. If it does,
  add "downscale on the CPU to the drawn size or use a mip chain" to the plan's
  Step 5. Only the native side was inspected in Step 3 (three frames, no moire
  seen, listed in `results.md`), because the Tauri app was not driven; this is
  part of why Step 3's checkbox is unticked.

- **Pending check: the first `native` CI job on GitHub.** What to verify: on
  `windows-latest`, the job passes, `mise run native:check` installs Rust 1.98.1
  through the task-level `tools` under CI's mise 2026.9.0 (local verification
  was on 2026.9.5 with 1.98.1 already installed), and whether NASM was on the
  image or came from `ilammy/setup-nasm`. Record the cold run's duration and
  raise the job's timeout if it exceeds 30 minutes. Step 1's checkbox was ticked
  on the criteria verifiable locally.

- **CI runs cargo directly instead of `mise run native:check`.** Issue: the CI
  `native` job in `.github/workflows/ci.yml` runs `mise exec rust@1.98.1 -- cargo
  clippy` and `cargo test` directly, because `nasm` was not visible to
  `crates/native/build.rs` through `mise run native:check`'s task shell on the
  runner (the exact point where `PATH` was lost is unconfirmed), while local
  checks use the `native:*` tasks. Goal: find where `PATH` is lost and have CI
  run `mise run native:check` again. Basis: Step 2's task brief. Paths:
  `.github/workflows/ci.yml`, `mise.toml`, `crates/native/build.rs`.
- **Pending manual check: the keypad's arrows on a physical keyboard.** What to
  verify: on Windows, with NumLock off, the keypad's Right and Left page the
  preview in `riffle-native`, as the main arrows do. Steps: open a folder and
  press them; expect the status line to change file. Step 2's checkbox was
  ticked on injected non-extended `VK_RIGHT` / `VK_LEFT` events, which gave the
  right behaviour; no physical keypad was pressed.
