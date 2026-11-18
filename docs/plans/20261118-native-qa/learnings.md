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

## Deferred issues (todo candidates)

- Step 2's failed-capture path (Linux's "not implemented for this platform",
  a lost device) was only reasoned about, not exercised: no Linux host here.
  When the feature build is first run on Linux, call `screenshot` and check
  it is a tool error with that message, not a panic. Basis: the Done-when
  "A failed capture ... is a tool error"; files `crates/native/src/main.rs`
  (`screenshot`), `crates/native/src/screenshot.rs` (`capture`).
