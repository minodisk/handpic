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

# The 1:1 zoom and focus peaking in the GPUI app

## Purpose

This is a slice of milestone Step 7 of the GPUI port
(`../20261109-gpui-port/plan.md`): the 1:1 focus check and the focus peaking
overlay. After it, `z` in `crates/native` shows the full-size embedded JPEG at
one JPEG pixel per device pixel around the focus point, and `e` lights the
strong edges in red on the fit view and on the 1:1 crop, as the Tauri app
does (`../../humans/usage.md`, "1:1 focus check" and "Focus peaking").

Out of scope, worked on by other sessions in parallel: port Step 6 (meta
pane, judgments, sidecars) and the rest of Step 7 (pass 2, the `f` mark, face
boxes, the face mesh, `eyes_of`, filter, sort). Also not ported here because
their hosts are not in the native app yet: the grayscale hold (`g`), Compare,
and the MCP view mode.

### What the investigation found (2026-11-10, `main` at `de249218`)

- **Keys.** `crates/backend/src/shortcuts.rs` defaults: `zoom` = `z`,
  `peaking` = `e`. The native app has no keymap yet; `main.rs` matches
  `e.keystroke.key` by hand (`"right"`, `"q"`, ...).
- **Tauri 1:1** (`crates/app/ui/src/main.ts`: `toggleZoom`, `requestCrop`,
  `drawZoom`, `cropViewportStale`, `scheduleCropForResize`; `zoom.ts`:
  `placeholderRect`):
  - The frontend invokes `focus_crop(path, width, height)` with the canvas
    size in device pixels.
  - `crates/app/src/commands.rs` `read_focus_crop` runs
    `riffle_core::reader::read_full` (no `ReadSlots` pause), then
    `crop_size(orientation, w, h)`, then
    `riffle_core::partial::decode_focus_crop(&jpeg, arw.shot.focus, w, h)`.
  - `crop_size` clamps each axis to `1..=CROP_MAX` (1024) and swaps the axes
    for orientation 6 / 8.
  - The crop is RGBA in unrotated JPEG coordinates, with the point of
    interest in crop pixels (`point_x` / `point_y`; the MCU snap and the edge
    clamp move the crop's origin) and the full JPEG's size.
  - The frontend rotates on the canvas, draws the crop with its point of
    interest at the canvas center, and under it the scaled preview as a
    placeholder (only when the file has a focus point).
  - One request is in flight; a result for a file that is no longer current
    is dropped and the current one re-requested while zoomed.
  - The held crop is reused when `z` is toggled back on for the same file and
    viewport.
  - Paging while zoomed requests the new file's crop; paging while not zoomed
    drops the held crop.
  - A resize while zoomed re-requests after `CROP_RESIZE_DEBOUNCE_MS` = 150.
  - The status shows `1:1` while zoomed, and an empty file list leaves the
    view.
  - `CROP_MAX`, `crop_size` and `read_focus_crop` live in `crates/app`, which
    the native crate must not depend on.
- **Tauri peaking** (`peaking.ts`, `peakingworker.ts`, `main.ts`
  `peakingOverlay` / `sendPeaking` / `clearPeaking`):
  - Luminance is `0.299 R + 0.587 G + 0.114 B`, held in a `Float32Array`.
  - A 3x3 Sobel gives `|gx| + |gy|`; a pixel is lit when that is
    `>= PEAKING_THRESHOLD` (160).
  - Lit pixels are `PEAKING_COLOR` `[255, 32, 32]` at alpha 255 on a
    transparent overlay; the one-pixel border is unlit, and an image under
    3x3 gives an empty overlay.
  - The input is the bitmap rasterised at the size it is drawn in device
    pixels (the fit view's `round(drawWidth * dpr)` x `round(drawHeight * dpr)`,
    the crop at its own size).
  - There is one layer per view (`fit`, `crop`), keyed by source bitmap,
    width and height.
  - The worker holds one request at a time; a layer replaced meanwhile is
    never computed, and the redraw after the reply sends the newest.
  - A layer is drawn only while its source is the bitmap being drawn at that
    size.
  - `e` off clears both layers. The state is session-only and off at launch.
  - `peaking.test.ts` has seven cases.
- **Native today.**
  - `load` decodes the preview to upright BGRA (`orient::rotate_bgra`) and
    keeps only the `Arc<RenderImage>`; the focus point (`arw.shot.focus`) is
    read and dropped.
  - The preview canvas paints `fit_centered`; replaced images go through
    `retired` and are released in the `cx.defer` after paint.
  - A `listing` counter drops reads of a previous listing.
  - The drawn bounds are known only inside the paint closure.
- **GPUI at the pinned rev.**
  - `RenderImage::as_bytes(0)` gives the BGRA bytes and `RenderImage.id` is
    public, so a background job can read the shown image through its `Arc`
    without a copy.
  - `Window::paint_image(bounds, image_bounds, ..)` clips to `bounds` (a
    sub-tile is cut when they differ) and snaps the visible bounds to device
    pixels (`snap_bounds`).
  - `window.scale_factor()` gives the device pixel ratio.
  - `image` is built with `default-features = false`; `image::imageops::resize`
    needs no feature.

### Rules both steps follow

- `crates/native` must not depend on `crates/app`; no change to `crates/app`,
  `crates/backend` or `crates/core` is planned. If one turns out necessary it
  is checked with both `mise run ci` and `mise run native:check`.
- `main.rs` is edited by other sessions concurrently: its diff is wiring only
  (fields, the two keys, calls into the new modules, the paint closure's
  draws). State machines, geometry, the Sobel and the crop read live in the
  new modules with unit tests.
- Blocking work runs on `cx.background_spawn`; every `RenderImage` no longer
  drawn goes through the existing `retired` list, never dropped earlier.
- Build with `/c/Users/daisu/AppData/Local/bin/NASM` prepended to `PATH`;
  `mise run native:check` before each PR and `cargo build` before launching
  the exe.
- Manual checks use copies in `D:\Photos\tests\20261110-gpui-zoom-peaking\`
  with `--index` pointing at a fresh file in that folder; never the Tauri
  index or the default native index. No screenshots. No other process is
  killed (a stale `riffle-native.exe` is renamed aside, per `native-app.md`).
- Manual checks by the user are acceptable Done-when lines; their result goes
  in `learnings.md`.

## Steps

- [x] Step 1: The 1:1 view on `z`
  - Done when:
    - `crates/native/src/zoom.rs` exists with unit tests and holds:
      - `CROP_MAX` (1024) and `crop_size(orientation, width, height)`, ported
        from `crates/app/src/commands.rs` with its cases (the clamp, the swap
        for 6 / 8).
      - `CROP_RESIZE_DEBOUNCE` (150 ms).
      - The request state machine, with tests mirroring `toggleZoom` /
        `requestCrop`:
        - toggling on with no held crop asks for one;
        - toggling back on with the crop of the same file and viewport asks
          for nothing;
        - a page while zoomed asks for the new file's crop, and a page while
          not zoomed drops the held one;
        - one request is in flight at a time;
        - a result for a file that is no longer current (or for a previous
          listing) is dropped and the current file re-requested while zoomed;
        - a viewport that changed while a request was in flight is noticed
          when it settles, and re-requested through the debounce;
        - an empty listing leaves the view.
      - The geometry as pure functions:
        - the point of interest mapped through orientation 1 / 3 / 6 / 8 into
          the upright crop, tested against `orient::rotate_bgra` on a buffer
          with one marked pixel;
        - where the upright crop is painted so that point sits at the canvas
          center, in device pixels;
        - the placeholder rectangle of the upright preview, a port of
          `placeholderRect` (full size from the crop when one has arrived,
          else taken as the sensor size) in upright coordinates.
      - The crop read, run on `cx.background_spawn`: `reader::read_full`,
        `crop_size`, `partial::decode_focus_crop`, RGBA to BGRA,
        `orient::rotate_bgra`, a `RenderImage`. It returns the upright point
        and the full size, with `read` / `decode` / `orient` durations.
    - `main.rs` wiring:
      - `z` toggles the view.
      - `Loaded` carries the file's `FocusLocation`.
      - While zoomed the preview canvas paints the placeholder (only with a
        focus point, as `drawZoom`) and the crop over it. Both are clipped to
        the canvas bounds, at one JPEG pixel per device pixel
        (`window.scale_factor()`), with the point of interest at the canvas
        center.
      - Paging stays zoomed and moves to the next file's focus point (the
        frame center without one).
      - The status line shows `1:1` while zoomed.
      - A crop error shows on the status line and paging continues.
      - A replaced or dropped crop goes through `retired`.
    - One `zoom read=… decode=… orient=… keypressToPixels=…` line is printed
      on stdout per crop that a `z` press asked for, the counterpart of the
      Tauri debug line.
    - Manual check on the test folder (an ARW with a focus point, a RAF, a
      portrait file with orientation 6 or 8 from `D:\photos\samples\`, a
      JPEG-only subfolder):
      - `z` shows the crop sharp, with no resampling blur, at the focus point.
      - The portrait crop is upright and centered on the same subject as in
        the fit view.
      - A focus point near an edge still sits at the center, with the image
        edge visible.
      - Holding `→` while zoomed keeps up and ends on the right file's crop.
      - Resizing the window re-cuts the crop after the drag settles.
      - `z` again returns to the fit view.
      - The window's scale factor during the check is recorded.
    - `mise run ci` and `mise run native:check` pass; `crates/app`,
      `crates/backend` and `crates/core` have no diff.
    - `docs/agents/native-app.md` gains what was met: how 1:1 maps to device
      pixels and whether `paint_image`'s snapping kept it unblurred, the
      measured `zoom` line figures, and how the paint closure's bounds reach
      the view. `CLAUDE.md`'s `crates/native` sentence names `zoom.rs`.
  - Implementation approach:
    - The viewport size in device pixels is only known in the paint closure.
      Decide between recording the canvas bounds in a shared cell the view
      reads (as `retired` / `pending` are shared), and a deferred entity
      update from the closure; keep whichever needs less of `main.rs`.
    - Do not hold `ReadSlots::pause()` around `read_full`: the Tauri
      `focus_crop` does not, and the pause is for the preview read.
    - `decode_focus_crop` already turns libjpeg's unwind into an `Err`; the
      rest of the read still goes through `guarded`.
    - A listing bump (`self.listing`) drops a crop read for the previous
      listing, as it does a preview read.
    - Verify that a crop painted at `size / scale_factor` logical pixels
      lands on exactly its own device pixels (snap the origin to device
      pixels before painting if the canvas center is fractional).
    - `z` is matched by name next to the existing keys; the keymap of
      `shortcuts.rs` arrives with another milestone.
- [ ] Step 2: Focus peaking on `e`, on the fit view and on the 1:1 crop, with the held-key measurement
  - Done when:
    - `crates/native/src/peaking.rs` exists with unit tests and holds:
      - `PEAKING_THRESHOLD` (160) and `PEAKING_COLOR` (`[255, 32, 32]` as
        RGB).
      - `peaking(bgra, width, height) -> Vec<u8>`: BGRA in, BGRA out,
        transparent where unlit, the same arithmetic as `peaking.ts` (luma
        stored as `f32`, the gradient summed in `f64`, `>=` against the
        threshold, `|gx| + |gy|`).
      - The seven cases of `peaking.test.ts` ported, plus one that pins the
        channel order: a pure-blue step and a pure-red step of the same
        height light differently, as the 0.114 / 0.299 weights say.
      - The layer model as a pure state machine, with tests mirroring
        `peakingOverlay`:
        - one layer per view (fit, crop), keyed by source image id, width and
          height;
        - one job in flight, and a layer replaced meanwhile is never sent;
        - the reply for a layer that is gone is dropped, and the newest
          unsent layer goes next;
        - a layer's overlay is handed out only for the source and size it was
          computed for;
        - turning peaking off clears both layers and returns their overlays
          for release.
      - The job, run on `cx.background_spawn`: read the source `RenderImage`'s
        bytes through its `Arc`, resize to the drawn device-pixel size when it
        differs (the fit view; the crop is already 1:1), run `peaking`, wrap
        the overlay in a `RenderImage`. It returns `resize` / `compute`
        durations, printed as one
        `peaking WxH resize=… compute=…` line on stdout.
    - `main.rs` wiring:
      - `e` toggles peaking (off at launch, not persisted).
      - The preview canvas paints the fit layer's overlay over the fit image
        at the same bounds, and the crop layer's overlay over the crop while
        zoomed.
      - A replaced or cleared overlay goes through `retired`.
      - The overlay appears a moment after the image and never over another
        file's image or another size.
    - Manual check on the Step 1 test folder:
      - `e` lights the in-focus edges in red on the fit view and, with `z`,
        on the crop.
      - Paging shows each file's own edges.
      - A window resize recomputes them.
      - `e` again removes them at once.
      - On one ARW and one RAF the lit regions look like the Tauri app's on
        the same file at a similar window size (by eye, noted in
        `learnings.md`).
      - Private bytes settle while holding `→` with peaking on (overlays are
        released).
    - **Measurement. Stop here and wait for the user's go-ahead: it needs an
      idle PC and other sessions measure too.**
      - Why it is needed: with peaking on, every key move starts a resize and
        a Sobel over the drawn frame on the background executor the next
        preview decode also runs on, and its reply repaints. Only a held key
        can show that; `--auto`'s 1003 ms interval lets each job finish
        before the next press.
      - Why nothing else is gated: with both modes off the diff adds to a key
        move only a few branches and one `Copy` field, so no A/B against
        `main` is planned (see Trade-offs).
      - Method: one release build, run directly, on
        `D:\Photos\tests\20261108-gpui-compare\ARW` (`--size 1616x1080`) and
        `...\RAF` (`--size 4416x2944`) with a warm fresh `--index` in the
        test folder. `→` is held for about 5 s per run, alternating peaking
        off / on, three runs each after one throwaway run, with the idle gate
        and conditions block of the port's `results.md`.
      - `results.md` in this folder records per run the p50 / p95 of
        `keypressToPixels` and of each stage, and the `peaking` line's
        `resize` / `compute`.
      - **Gate:** the median of the three runs' p50 `keypressToPixels` with
        peaking on, minus off, is at most +2.0 ms on ARW and +5.0 ms on RAF.
        If missed, `results.md` names the stage that grew and the job is
        fixed (for example started only after the image has presented, or
        run at a lower thread priority) before the plan closes.
      - Recorded, not gated: one held-key run while zoomed (the preview and
        the crop read per page) with its `zoom` lines.
    - `mise run ci` and `mise run native:check` pass; `crates/app`,
      `crates/backend` and `crates/core` have no diff.
    - `docs/agents/native-app.md` gains: peaking runs off the main thread
      with one job in flight and why, the resize filter chosen and why, the
      measured figures with a link to `results.md`. `CLAUDE.md`'s
      `crates/native` sentence names `peaking.rs`. The port plan's Progress
      gains one line saying Step 7's 1:1 zoom and peaking slice is done
      through this plan (its path in backticks); Step 7's checkbox stays
      open.
  - Implementation approach:
    - Assumes Step 1 merged (the crop layer needs the crop, and the bounds
      plumbing is reused for the fit view's drawn size).
    - The source id is `RenderImage.id`; the model stays generic over it so
      its tests need no GPUI.
    - The resize works on the BGRA bytes as they are (the channel order does
      not matter to it). Start with `image::imageops::resize` and a bilinear
      (`Triangle`) filter, the nearest to a canvas `drawImage`; if its time
      on a 4416x2944 RAF makes the overlay visibly late, decide between a
      cheaper filter and another resizer, and record the choice.
    - The overlay's alpha is 0 or 255 only, so premultiplication does not
      matter.
    - The measurement's key hold is either the user's hand or injected key
      repeats (as the port's key-name check injected `keybd_event`);
      `results.md` says which.

## Trade-offs and risks

- **Two PRs or one.** The slice is split so each review stays near 400-500
  lines and the 1:1 view can merge without waiting for an idle PC. It can be
  one PR instead (Step 2 only adds to Step 1); the cost is a larger review
  and a merge held by the measurement, the gain is one round of `main.rs`
  conflicts instead of two. (Chosen by the user: two PRs.)
- **`CROP_MAX` stays 1024 (planned) or is lifted to the viewport.** The Tauri
  cap exists for the IPC payload ("caps it at 4 MB; the decode itself barely
  depends on the size"), which the native app does not have. Kept: on a
  viewport larger than 1024 device pixels the sharp crop is a 1024 px box
  with the scaled preview around it, exactly as the Tauri app. Lifted: the
  whole window is sharp, at about 12 MB of texture for 2400x1275, but the
  view differs from the Tauri app's and the crop read time must be
  re-measured. (Chosen by the user: keep 1024.)
- **`CROP_MAX` and `crop_size` are duplicated in `zoom.rs` (planned) or moved
  into `riffle-backend` / `riffle-core`.** Duplicating keeps `crates/app`
  untouched and costs two copies of ten lines until port Step 10 removes the
  Tauri one. Moving gives one definition but edits `crates/app`.
- **RGBA to BGRA in the native crate (planned) or a BGRA variant of
  `decode_focus_crop` in `riffle-core`.** The swap is over at most 4 MB and
  is timed inside the `zoom` line's `decode`; the core variant saves it but
  changes a crate both apps compile.
- **Peaking at the drawn device size (planned, as Tauri) or at the preview's
  own resolution.** The second needs no resize and no bounds plumbing for the
  fit view, but the GPU then scales a one-pixel-wide overlay and the lit
  edges differ from the Tauri app's for the same threshold. The first cannot
  be pixel-identical either (Chromium's rasteriser against the `image`
  crate's filter); "like the Tauri app" is judged by eye.
- **No A/B against `main` for the path with both modes off (planned).** The
  strip plan measured one because the strip added paints and decodes to
  every page; this diff does not. If belt and braces is wanted, add the
  same-session A/B of the strip plan (this build against `main` at the
  branch point, `--auto 60`, gate +2.0 ms ARW / +5.0 ms RAF); it costs a
  second build and another idle window.
- **The held-key gate is noisy.** Key repeat is about 30 Hz and `frameWait`
  moves 6-10 ms between runs of one binary, so p95 is recorded only and the
  gate is the median of three p50s, alternating within one session.
- **Zoomed paging reads the full JPEG per page.** `read_full` is a ranged
  read of several MB with no `ReadSlots` pause, next to the preview read; the
  Tauri app does the same. It is recorded, not gated, and one crop is in
  flight at a time.
- **The keys are hard-coded.** `z` and `e` are matched by name until the
  keymap is ported; the Step 6 session may bring `shortcuts.rs` wiring and
  conflict on the key match, so the diff there is one line per key.
- **Not ported here.** The grayscale hold (which also grays the peaking
  red), Compare (which leaves the 1:1 view), the `f` mark and face overlays
  over the fit view, the MCP view mode, and panning or free zoom (the Tauri
  app has none either).

## Progress
