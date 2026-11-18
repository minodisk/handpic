# Learnings: native-qa

## Step 1

- The reply's wire shape is `{"png": <base64>, "width", "height"}`, built by
  the `pub` helper `handpic_backend::mcp::screenshot_reply(png, width,
  height)`; Step 2's native side should answer through it rather than
  building the JSON itself.
- `with_screenshot` adds `screenshot_router` to whatever tools the server
  already lists (`Mcp::new(..).with_screenshot()` alone lists eight tools).
  It must come after `with_all_tools`, which replaces the router and would
  drop the tool: `Mcp::new(..).with_screenshot().with_all_tools()` does not
  list `screenshot`.
- The tool uses `Bridge::call` (the 5 s `BRIDGE_TIMEOUT`); Step 2 measures
  whether that is enough.
- A Python heredoc turned `b"\x89PNG"` into a raw 0x89 byte, which rustc
  refuses (byte literal non-ASCII, then invalid UTF-8). The tests use plain
  ASCII bytes; the content of the PNG is not what they check.

## Step 2

- `gpui-kit/test-support` cannot be the feature: its weak
  `gpui-fast?/test-support` makes Cargo resolve the optional gpui-fast crates,
  whose `gpui-fast-wgpu` pins `wgpu = "=30.0.1"` against the locked 30.0.2, and
  resolution fails. The feature is `["dep:gpui-pre-platform", "image/png"]`,
  an optional direct `gpui-pre-platform = "=0.3.12"` with `test-support`
  (which carries it to gpui-pre and gpui-pre-windows). gpui-base's
  `test-support` (the `Observed` wrappers) stays off, so the feature build
  draws the same elements as the plain one. `Cargo.lock` gained 36 crates
  (proptest, postage, futures-concurrency, and criterion, clap, plotters and
  the perf-event crates through gpui-pre's `bench_metrics?/test-support`
  weak feature); none is compiled without the feature. Step 4's
  dev-dependency may hit the same `gpui-kit/test-support` conflict.
- The `image` dev-dependency with `png` gives the default `cargo test` the
  encoder for `screenshot.rs`'s tests without adding `png` to the plain build.
- Capture timing chosen: after the next drawn frame. A next-frame callback runs
  before that frame's draw, so the view nests two; the inner one captures.
  Fallback: `SCREENSHOT_FRAME_WAIT` (500 ms), then the last frame as it is.
- Checked live on this Windows host through the MCP endpoint (debug feature
  build, temporary settings store with `mcpEnabled`, `--mcp-port 41999`, a
  folder of three ARWs), each PNG viewed:
  - fit view, windowed 1600x1000 device px: correct; default long edge gave
    1568x980, `long_edge` 4096 gave 1600x1000 (not upscaled), 256 gave
    256x160.
  - `set_overlays` focus + performance, then `screenshot` at once: both
    overlays on the image (the next-frame capture shows the previous call's
    change).
  - `clear_cache` (its confirmation dialog, `mcpAllowDestructive` off):
    dialog on the image, answered while the view drops keys; then
    `cancel_operation`.
  - settings modal opened with `ctrl+,` sent through `WScript.Shell.SendKeys`:
    modal on the image, answered while open.
  - window covered by a maximized topmost WinForms window: an overlay
    change made while covered was on the image (captured through the frame
    path in 126 ms, so a covered window still draws).
  - minimized: no frame in 500 ms, the fallback took the last frame, which
    predated a `navigate` made while minimized (as the tool's description
    says: call again).
  - after captures, the window restored and re-grabbed from the screen
    (`CopyFromScreen`) matched the state (x2.arw shown), no stale or blank
    frame; the same after the covering window closed. No `window.refresh()`
    was added.
  - default build (no feature): `tools/list` has the 35 tools, no
    `screenshot`.
- Request-to-reply time, debug build, maximized 5058x1440 window: capture
  ~300 ms, whole call 360-430 ms at the default 1568, 620-720 ms at 4096. The
  first version took 1.8 s at 1568 and 4.2 s at 4096 (near the 5 s bridge
  wait): `imageops::resize` is generic, so it was instantiated (unoptimized)
  in `handpic-native`; going through `DynamicImage::resize_exact` uses the
  `image` crate's own instantiation, and `image`, `png`, `fdeflate`,
  `flate2`, `miniz_oxide`, `crc32fast` went into `[profile.dev.package]`. The
  bridge wait stays `BRIDGE_TIMEOUT`.
- A failed capture (Linux, lost device) is not reachable on this host; the
  path maps `render_to_image`'s error to the tool error by code only.
- Launching `notepad` through `Start-Process` from the sandbox opened Windows'
  "choose an app" dialog instead; it was closed. A topmost WinForms form from
  PowerShell is the reliable way for an agent to cover the window.

## Step 3

- The Windows-only run is a check of its own in `tools/ci/local.sh`,
  `clippy native screenshot`, printed right after `clippy native` wherever
  that one is, on a host whose `uname -s` is `MINGW*` / `MSYS*` / `CYGWIN*`.
  `CI_LOCAL_UNAME` overrides the `uname -s` so `local.test.sh` pins it
  (`Linux` for the existing cases, a `MINGW64_NT-...` and a `Darwin` block
  for the new ones). `every_check` lists it on every host, so a Linux or
  macOS run shows it under "skipped (left to GitHub Actions)", which is true:
  the Windows leg runs it.
- `native:check` picks Windows the same way (`uname -s` under its
  `bash -c`); `ci.yml` uses `$RUNNER_OS`. The three command strings are the
  same `cargo clippy --all-targets --features screenshot -- -D warnings`.
- Step 2 changed what this step's notes assumed: the feature turns on
  `test-support` in a direct optional `gpui-pre-platform` dependency, not
  `gpui-kit/test-support`, so the feature run re-checks the GPUI platform
  crates (gpui-pre, gpui-pre-windows, gpui-pre-platform) and everything above
  them (gpui-kit, gpui-component, `handpic-native`) plus the 36 crates the
  lock gained. Whether Step 4's dev-dependency lands on the same feature set
  (and so shares these artifacts) depends on how it gets around the
  `gpui-kit/test-support` resolution failure Step 2 hit.
- The `native` job's cargo cache is keyed on `crates/native/Cargo.lock`, and
  `actions/cache` saves nothing on an exact key hit. This PR does not change
  the lock, so the cache main saved after Step 2 (no feature artifacts) is
  hit and never re-saved: every Windows run pays the feature check's compile
  until the lock next changes, a warm run included. Measure it on the PR.
- Windows `native` job duration, before this step (from `gh run` on
  main and PR branches):
  - warm cache (exact key hit): 2m45s (run 44206444223) and 3m12s (run
    44205356383); the lint-and-test step about 2 minutes.
  - a restore-key hit after a lock change (Step 2's merge, run 44214212075):
    11m57s, the lint-and-test step 7m48s.
  - cold (`Cache not found`): 19m01s (run 44121287234, job 134173026923; the
    lint-and-test step 17m16s) and 16m58s (run 44202881195, job
    134461222844; the step 15m32s). Well within the 30-minute timeout, but a
    second check of the GPUI stack on top of the cold figure is the margin
    to watch.
- After this step: PENDING, to be filled from the PR's Windows `native` job
  (its duration and the lint-and-test step's, noting whether the cache was an
  exact hit, a restore-key hit or not found). A truly cold figure needs a run
  with no cache (a lock change on a branch with no prior cache, or the cache
  deleted).

## Step 4

- The dev-dependency is `gpui-pre-platform = { version = "=0.3.12",
  features = ["test-support"] }`, the same crate and feature the
  `screenshot` feature turns on (`gpui-kit/test-support` still fails on the
  gpui-fast `wgpu` pin). `Cargo.lock` did not change. `gpui_kit::*`
  re-exports gpui-pre, so `gpui_kit::TestAppContext` and
  `VisualTestContext` are there; `gpui_kit::test` (gpui-kit's own UI test
  module) is behind `gpui-kit/test-support` and is not.
- Effect on Step 3: the GPUI crates now have one feature set for
  `clippy --all-targets`, `cargo test` and the `--features screenshot`
  clippy (the dev-dependency's `image/png` was already there), so the feature
  clippy after the default one compiled only `handpic-native` (3 s here).
  `docs/agents/native-app.md`'s CI entry says so. `cargo build` (plain)
  still compiles; it rebuilt only `handpic-native`.
- The seam is `open_main(args, index, settings, saved, cx)` in `main.rs`:
  init, key bindings, theme, placement and `open_window`. `main` keeps the
  app identity, `on_window_closed` -> quit and `activate` around it (the
  order relative to `gpui_kit::init` moved; they do not depend on it).
  `mcp_port`, `prefetch` and `remember` are derived from `Args` inside, so
  the test passes `Args::default()` (`remember` on, against the temporary
  settings store; `--auto` would run its presses and quit).
- Teardown: the first version left `index.sqlite*` and `settings.sqlite*`
  behind. Two causes: `Ui` held a strong `Entity<Handpic>` (taken in `Drop`
  now, before `quit`), and the sidecar writer and settings `Writes` threads
  close their connections only once they see their senders dropped, a few
  ms after `quit`. The drop retries `remove_dir_all` (removed after ~11 ms,
  one retry) within a 10 s bound. `quit` (the quit handlers' drains) took
  ~0.5 ms; the drop asserts under 1 s, below the 2 s drain timeouts.
- `TestAppContext::single()` plus `executor().allow_parking()`;
  `run_until_parked` also advances the clock to the next timer, so the
  view's background timers do not need `advance_clock` to fire.
- The smoke test passed 25 runs alone and 20 runs of the whole crate's
  tests (1054) on Windows; it takes ~50 ms.

## Deferred issues (todo candidates)

- Step 2's failed-capture path (Linux's "not implemented for this platform",
  a lost device) was only reasoned about, not exercised: no Linux host here.
  When the feature build is first run on Linux, call `screenshot` and check
  it is a tool error with that message, not a panic. Basis: the Done-when
  "A failed capture ... is a tool error"; files `crates/native/src/main.rs`
  (`screenshot`), `crates/native/src/screenshot.rs` (`capture`).
