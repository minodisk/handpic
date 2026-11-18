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

# Native app QA: an MCP screenshot tool and UI regression tests

## Purpose

The native app (`crates/native`, GPUI) is tested today only through its pure
modules; nothing checks the window itself. This plan adds two complementary
ways to do that.

- **A, visual checks on the live app.** A `screenshot` tool in the MCP
  companion returns the app window as an image, so an agent that drives the
  app through the other tools can look at the result (layout, overlays,
  dialogs, theme). The capture uses GPUI's `Window::render_to_image`, which
  exists only under gpui's `test-support` feature, so the tool sits behind a
  new `screenshot` cargo feature of `handpic-native` and the release build and
  the packages stay as they are.
- **B, regression tests of the wiring.** The main view is built inside GPUI's
  `TestAppContext` (with `VisualTestContext` as its window handle) and driven
  with simulated keys, asserting on the view's state and on what the last
  scene painted (`painted_quads`). These run in the default `cargo test` on
  every platform. They check no pixels; visual checks are A's.

## Steps

- [x] Step 1: Add the `screenshot` tool to the backend's MCP server, listed only when the host opts in
  - Done when:
    - `Mcp` has a builder (`with_screenshot`, next to `with_all_tools`) that adds one `screenshot` tool to the listed tools; without it the tool lists of `Mcp::new` and `with_all_tools` are exactly what they are today (the two existing list tests stay unchanged and pass).
    - The tool takes an optional `long_edge` (1568 when omitted, clamped to 256 to 4096), sends a `screenshot` request through the `Bridge` with the clamped value, and turns the main window's reply into an MCP image content block (`image/png`) followed by a text block with the image's width and height, as `get_preview` does. A refusal or a malformed reply becomes a tool error.
    - Unit tests cover: the `long_edge` default and clamping, the tool list with and without `with_screenshot`, the reply-to-image conversion (a good reply, a reply missing its fields) and a bridge refusal.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - File: `crates/backend/src/mcp.rs` only. `handpic-backend` gets no cargo feature and no GPUI dependency; the opt-in is a runtime builder, consistent with "the tool list is the host's" (`with_all_tools`, `with_home`, `with_port`).
    - A third `#[tool_router(router = screenshot_router)]` block on `Companion`, added to `tools` by `with_screenshot`, mirrors how `native_router` is kept out of the seven-tool list.
    - `Bridge`'s `Reply` is `Result<Value, String>` and `tool_result` turns it into structured text, so the image travels as base64 inside the JSON reply and the tool builds `ContentBlock::image` itself (see `Companion::preview`). To keep the wire shape in one file, consider a small `pub` helper here that builds the reply value from the PNG bytes, width and height; `base64` is already a backend dependency and not a direct one of `crates/native`.
    - The constants sit next to `PREVIEW_LONG_EDGE*`. The cap at the window's own size is the native side's (Step 2); the tool never asks for upscaling.
    - Do not change `NATIVE_INSTRUCTIONS`; the tool's description carries what a client needs (a debugging aid, present only in a build with the feature, an error on Linux).
    - `Bridge::call` waits `BRIDGE_TIMEOUT` (5 s). Step 2 measures whether that is enough; if not, this tool uses `call_within` with a longer wait, as `wait_for_idle` does.
- [x] Step 2: Capture the window in `crates/native` behind the `screenshot` feature and answer the tool
  - Done when:
    - `crates/native/Cargo.toml` has a `screenshot` feature enabling `gpui-kit/test-support` (and what the PNG encode needs). Without the feature nothing new is compiled into the binary, `with_screenshot` is not called, and `tools/list` and every answer are unchanged.
    - With the feature, on Windows, calling `screenshot` against the running app (`mcpEnabled` on) returns a correct PNG of the window: checked by hand or by an agent through the MCP endpoint for the fit view, an overlay on (`set_overlays`), the settings modal open, a confirmation dialog open, and the window fully covered by another window. The image's long edge is at most the requested / default value and never larger than the window's own.
    - The tool is answered while the view drops keys (settings, a dialog, a rename open), since those are what it is for.
    - A failed capture (Linux's "not implemented for this platform", a lost device) is a tool error with a message, not a panic.
    - After a capture the window keeps presenting correctly (no stale or blank frame on the next present); the check and its result are recorded in `learnings.md`.
    - The capture timing chosen (at once, or after the next drawn frame) and the measured request-to-reply time of a maximized window in a debug build are recorded in `learnings.md`.
    - Unit tests cover the pure parts: the scaled size for a long edge (landscape, portrait, already smaller, 1 px edge cases) and the scale + encode of a small RGBA image (decodes back to the expected size).
    - A mise task runs the app with the feature; `CONTRIBUTING.md` lists it; `CLAUDE.md`'s layout paragraph and `docs/agents/native-app.md` describe the new module, the feature and the backend builder.
    - `cargo clippy --all-targets --features screenshot -- -D warnings` passes in `crates/native` on the implementer's Windows host (CI builds the feature from Step 3 on).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged.
    - Files: `crates/native/Cargo.toml` (and `Cargo.lock`, which is expected to gain the crates gpui's `test-support` pulls in, such as `proptest`, `postage`, `futures-concurrency`: none of them is in the lock today), a new `crates/native/src/screenshot.rs`, `crates/native/src/main.rs` (`mcp_request`), `crates/native/src/mcp.rs` (`with_screenshot` under `#[cfg(feature = "screenshot")]`), `mise.toml`, `CONTRIBUTING.md`, `CLAUDE.md`, `docs/agents/native-app.md`.
    - `Window::render_to_image(&self) -> anyhow::Result<image::RgbaImage>` (gpui-pre 0.3.12 `src/window.rs`) renders the last completed frame's scene. The view holds its window as `self.window: AnyWindowHandle`; `mcp_request` has only a `Context<Self>`, so the capture goes through that handle (or a deferred / spawned update). Verify that updating the window from where `Event::Mcp` is handled does not fail on a re-entrant borrow.
    - Intercept `request.kind == "screenshot"` in `mcp_request` before `companion::answer`, under the feature. That keeps `companion.rs` untouched: `answer`, `READS`, `Busy` and the coverage table do not learn the tool. The coverage table maps user-facing actions to tools and is tested only in that direction, and a screenshot is not a user-facing action, so it needs no entry.
    - Scale and encode off the UI thread (`cx.background_executor()`), replying through `Server::reply` when done. `[profile.dev.package]` optimizes only the GPUI crates, so `image`'s resize and the PNG encode run unoptimized in a debug build. If the measured time nears 5 s, lengthen the tool's bridge wait (Step 1's note) or add the encoding crates to `[profile.dev.package]`.
    - Decide between capturing at once and capturing after the next drawn frame (`cx.notify()` then an on-next-frame callback). `render_to_image` reads `rendered_frame`, so a capture taken right after another tool's reply can predate that tool's redraw. Measure whether the next-frame callback fires while the window is occluded or minimized; if it does not, capture at once (or fall back after a short timer) so the call never runs into the bridge timeout, and say in the tool's description that a client may need to call again.
    - Side effects of the capture, from reading the source: the Windows renderer's `render_to_image` calls the same `render` as `draw` (clear, then every batch) into the live render target and skips `present`; every later present goes through `Window::present` -> `platform_window.draw(scene)`, which clears and re-renders the whole scene before presenting. No leftover is expected and no forced refresh is planned; confirm by hand (capture while idle, then expose / resize the window) and add a `window.refresh()` after the capture only if an artifact shows.
    - Pure parts (`screenshot.rs`): the target size for a long edge, never upscaling, and scale + PNG encode of an `RgbaImage`. `image` is already a dependency with `default-features = false`; `image::imageops::resize` needs no feature, the PNG encoder needs `image/png` (gpui already enables it in the same `image` 0.25, but name it rather than rely on unification). Gate the module so its tests run in the default `cargo test` (for example `#[cfg(any(test, feature = "screenshot"))]` for the pure parts and `#[cfg(feature = "screenshot")]` for the capture), without tripping `dead_code` under `clippy --all-targets -- -D warnings`.
    - The captured image is in device pixels (the render target's size), so a HiDPI window is larger than its logical size; the text block reports the returned size.
    - gpui-base swaps some element types for `test_support::Observed` wrappers under `test-support`, and gpui-component compiles extra hooks. Confirm the whole app compiles and behaves the same in the feature build.
    - mise task: `native:dev` passes `"$@"` after `--`, so a cargo feature cannot be given through it. Add a sibling task (same `dir`, `tools` and `shell` as `native:dev`) that runs `cargo run --features screenshot -- "$@"`.
    - Docs: `README.md` and `docs/humans/usage.md` list only the seven companion tools in their table, not the native app's, and this tool is a development aid absent from the packages, so the README / usage pairs are not changed. `docs/agents/native-app.md`: extend "The MCP companion answers from the view, one channel hop away" with the opt-in tool and add a GPUI entry on `render_to_image` (feature gate, per-platform support, no present, what was measured).
- [x] Step 3: Lint the `screenshot` feature build in CI and in the local checks
  - Done when:
    - On Windows, `cargo clippy --all-targets --features screenshot -- -D warnings` runs in `crates/native` in all three places that lint the native app: the `native` job of `.github/workflows/ci.yml`, `tools/ci/local.sh` (so `mise run ci:local` runs it when the branch touches `crates/native`) and the `native:check` mise task. On macOS and Linux none of the three runs it.
    - `tools/ci/local.test.sh` covers the new check and passes.
    - The `native` job stays within its 30-minute timeout on a cold cache; the Windows job's duration before and after is recorded in `learnings.md`.
    - `docs/agents/native-app.md` ("The `native` CI job runs on all three platforms") says what runs where and why.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged.
    - Files: `.github/workflows/ci.yml`, `tools/ci/local.sh`, `tools/ci/local.test.sh`, `mise.toml` (`native:check`), `docs/agents/native-app.md`.
    - Windows only: it is the platform the tool is accepted on, Linux has no `render_to_image`, and the macOS job already compiles a second variant of the app for its bundle step. Recent warm-cache `native` jobs take about 3 minutes on Windows, 5 on Linux and 1 on macOS, so the headroom is large.
    - Until Step 4 lands, this run compiles the GPUI crates a second time (with `test-support`). After Step 4 the default `clippy --all-targets` and `cargo test` builds use `test-support` too, so the two runs share the GPUI artifacts and only `handpic-native` itself is compiled twice.
    - The CI job runs its commands through `mise exec rust@1.98.1 -- ...` rather than `mise run native:check` (NASM lookup on Windows); keep that form for the new command, and keep the command string identical in the three places.
    - Whether the new run is a second command of the existing `clippy native` check or a check of its own in `local.sh` is decided here; `local.test.sh` asserts the planned check names, so its expectations change either way.
- [x] Step 4: Make the main view constructible in a GPUI test
  - Done when:
    - `gpui-kit`'s `test-support` feature is on for `crates/native`'s test builds through a dev-dependency, so the default `cargo test` gets `TestAppContext` without any flag, and `cargo run` / the release build still compile without it.
    - One function opens the main window (the init, the key bindings, the theme, `gpui_kit::open_window` with the `Handpic` view) and both `main` and the tests call it; the app's behavior at launch is unchanged.
    - A test helper builds the app in a `TestAppContext` over a temporary index file and a temporary settings store, and a smoke test opens the window with no folder, draws a frame and asserts the no-folder state (the view's state and a non-empty `painted_quads`). The test passes repeatedly (run it at least 20 times locally) and leaves no file outside its temporary folder.
    - The test ends cleanly: no hang at teardown, and the quit handlers' drains do not wait out their timeouts.
    - What blocked construction, the seam taken and how a test waits for work on the app's own threads are written in `docs/agents/native-app.md`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/Cargo.toml` (and `Cargo.lock` if Step 2 did not already add everything), `crates/native/src/main.rs`, a new `#[cfg(test)]` module for the helper and the UI tests (the crate is a binary, so the tests live inside it, not under `tests/`), `docs/agents/native-app.md`, `CLAUDE.md`'s layout paragraph for the new module.
    - What is already injectable: `Handpic::new` takes the index (`Result<Indexes, String>`, from `open_index(Some(path))`), the `Settings` (`settings::load(Some(path), None)` reads a given store), `mcp_port`, `prefetch` and `remember`. With `mcpEnabled` off no MCP thread starts. `remember = false` (the `--auto` path) keeps the session, panes and geometry from being read or written.
    - What is not: the window is opened inline in `main`'s `run` closure, which is the one seam to cut. Verify `gpui_kit::init`, `theme::apply` and `gpui_kit::open_window` (which wraps the view in `Root`) run on the test platform; the test app has no `icons::Icons` asset source unless the helper sets one.
    - Work outside the test dispatcher: the index eviction thread, `Watch::spawn`, `TreeWatch::spawn`, the sidecar `Writer`, the judgment queue, the settings `Writes` thread and the scan's threads are `std::thread`s whose results reach the view through its one channel. `run_until_parked` does not wait for them, so the helper needs a bounded "wait until this holds" loop (parking allowed, real time, a timeout that fails the test). Timers on the background executor (`SCAN_FOCUS_DELAY`, the dwell, the debounces) need `advance_clock`.
    - `read_roots` reads the real home directory and volumes (`dirs::home_dir()`); the tests must not assert on the tree's roots. Decide here whether passing the home directory in is worth it or leaving it is enough.
    - With the dev-dependency feature, `cargo clippy --all-targets` and `cargo test` compile the GPUI crates, and the binary target inside those invocations, with `test-support`; `cargo run`, `native:release`, the macOS bundle step and the packages compile them without. Confirm the app still compiles and its existing tests pass under `test-support` (gpui-base's `Observed` element wrappers, gpui-component's hooks), and that a plain `cargo build` still passes.
- [x] Step 5: Add the first UI regression tests through `TestAppContext`
  - Done when:
    - Tests in the default `cargo test`, built on Step 4's helper, each driving the real view with simulated input and asserting on its state and, where it adds something, on `painted_quads`:
      - opening a folder of generated files lists them: the file count, the current file, the status, the strip's cells;
      - the paging keys move the current file through the keymap, wrapping at the ends;
      - a judgment key changes the current file's judgment in the view, the index row and the strip cell's marks;
      - a filter choice (the menu's click path, `apply_menu`) narrows the shown files and lands the current file where the anchor rule puts it;
      - `ctrl+,` opens the settings modal, a paging key is dropped while it is open, Escape closes it and the keys work again.
    - Each test passes repeatedly (at least 20 local runs of the set) on Windows and passes on all three platforms in the PR's checks.
    - `docs/agents/native-app.md` says how to write such a test (the helper, waiting, fixtures) and what they do not cover.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 4 is merged.
    - Files: the UI test module from Step 4, `docs/agents/native-app.md`.
    - Pick wiring, not rules: `sort.rs`, `filter.rs`, `judgment.rs`, `strip.rs`, `settingsmodal.rs` and the rest already unit-test their pure parts. These tests cover what joins them (`key_down` to `perform`, `refilter`, `judge_paths` to the row and the writer, the modal's focus trap).
    - Fixtures: the repository holds no image file (only `.dop` fixtures). A JPEG with Exif can be generated at test time with `mozjpeg`, as `crates/core/tests/common/mod.rs`'s `sample_jpeg_with` does (that helper belongs to another crate's tests and cannot be imported). A JPEG-only folder is view-only and refuses judgments, so the judgment test needs a RAW file: verify whether a minimal generated DNG (a TIFF with an embedded preview JPEG) is accepted by the listing and the DNG parser. If it is not, record it and cover the judgment path up to what a generated file allows (see "Trade-offs and risks").
    - Pass 1 and pass 2 run on the scan's own threads against the generated files; wait for the listing (and the scan where a test needs it) with Step 4's bounded wait, never with a fixed sleep.
    - `VisualTestContext::from_window` gives the window handle `simulate_keystrokes` needs. Key names follow the keymap (`keys.rs`; lowercase on Windows).

## Trade-offs and risks

- **Absent, not refused, without the feature (decided).** The backend lists `screenshot` only after `with_screenshot`, which the native app calls only under the feature, so a release build's `tools/list`, instructions and answers are what they are today. Always listing it and refusing it, like `rename_file`, would change the release tool list.
- **PNG only, long edge 1568 by default, 256 to 4096, capped at the window's size (decided).** No `format` argument; JPEG through `mozjpeg` can be added later without breaking callers.
- **The feature clippy runs on Windows only.** macOS has a Metal `render_to_image` this plan neither verifies nor lints, and whether the feature build compiles on Linux is unchecked. Running it on all three is the alternative; after Step 4 its cost is small (shared GPUI artifacts), so it can be widened then if wanted.
- **`test-support` in the test build hides one class of mistake.** After Step 4, `clippy --all-targets` and `cargo test` see the GPUI crates with `test-support`, so non-test code that uses a test-only GPUI API by accident passes them on Windows and Linux and fails only in a plain build. The macOS job's bundle step is a plain build and catches it; `mise run ci:local` on Windows does not. A plain `cargo check` in the Windows / Linux jobs would close it at the price of a second GPUI compile there; not planned.
- **Compile time and disk.** A developer's `target` holds two variants of the GPUI crates after Step 4 (plain for `cargo run`, `test-support` for tests and clippy), and the macOS CI job compiles both on a cold cache. `Cargo.lock` grows by the `test-support` crates, which changes the CI cache key once.
- **`test-support` is broader than the APIs used.** It compiles gpui's test platform and debug bookkeeping in, and on Linux turns on gpui's `wayland`, `x11` and `proptest` features. A `screenshot` build is not the binary that ships: take no timing measurement with it.
- **Tests over real threads can be flaky.** The view's scan, watchers and writers stay real `std::thread`s, so the UI tests wait on real time with a bound. The alternative, routing every spawn through an injected spawner so the test dispatcher owns all work, is deterministic but touches the backend's scan, watch and writer APIs and is far beyond the smallest seam. If Step 4 or 5 cannot be made stable, record it and narrow the tests to what needs no scan thread.
- **The judgment test depends on a generated RAW.** If no minimal DNG the parser accepts can be generated, the options are a small real RAW committed as a fixture (a binary in the repository, with its license question) or leaving the judgment path to the pure tests. The caller decides if Step 5 hits this.
- **The tree's roots come from the real machine** in the tests unless Step 4 injects the home directory; assertions stay off them either way.
- **A stale frame.** If capturing after the next frame does not work while the window is occluded, `screenshot` returns the last completed frame, which can predate the previous tool's redraw by a frame.
- **Dropped: headless pixel snapshots.** A `HeadlessAppContext` + wgpu spike on the Linux runner was considered and left out; B asserts state and painted quads only, and pixels are A's live screenshot.

## Progress

- (2026-11-18) Step 1 complete
- (2026-11-18) Step 2 complete
- (2026-11-18) Step 3 complete
- (2026-11-18) Step 4 complete
