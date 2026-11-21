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

# Fit-view preview at about the drawn size

## Purpose

On a 4416x2944 RAF preview the GPUI app's frame reaches the screen about 48 ms
after the in-app `present` stamp, against about 14 ms on a 1616x1080 ARW
preview (`docs/plans/_archived/20261120-tauri-gpui-latency-ab/results.md`:
`screen` minus `keypressToPixels` 47.95 ms against 13.55 ms, `paint` 3.36 ms
against 0.70 ms).

A follow-up investigation in another session found the cause. GPUI
(`gpui-pre-windows` 0.3.12, `directx_atlas.rs`) creates an atlas texture on
the first paint of a new image and uploads the whole image with
`UpdateSubresource`: 52 MB for the RAF preview. `Present(0, 0)` does not wait
for it. With about 1 s between page turns the GPU and its PCIe link have
dropped to a low-power state, the upload takes about 33 ms and the frame
misses two DWM compositions. A 7 MB ARW upload takes about 5 ms in the same
state. A reproduction bench is kept outside the repository.

The fix: hand GPUI a fit-view image of about the drawn size in device pixels,
not the whole preview. The preview JPEG is decoded at a reduced DCT scale off
the UI thread, so the upload shrinks to ARW-like sizes and the decode gets
cheaper too. Once done, a page turn onto a large preview reaches the screen
about as soon after `present` as a small one does. GPUI is not patched; OS and
driver power settings are out of scope.

## Steps

- [x] Step 1: Separate the preview's source size from its decoded pixels (no behavior change)
  - Done when:
    - A held preview (`Held`, `OnScreen` in `crates/app/src/main.rs`) carries the upright size of the source preview beside the image. Everything that needs the source's geometry reads that, not `RenderImage::size`:
      - the fit view's bounds (the preview canvas's `fit_centered` call, which the `f` overlay, the peaking overlay and the image all share);
      - the 1:1 placeholder (`zoom_paint`, which passes `image.size(0)` to `zoom::placeholder` today).
    - The prefetch ring's `Pixels` for `Held` counts the decoded image's pixels, not the source's. With every decode still full-size the two are equal.
    - The strip's own `fit_centered` use on thumbnails is unchanged.
    - No decode changes: the image handed to GPUI is still the full preview. The `page` line and every pixel on screen are as before.
    - Unit tests cover any pure function this introduces (for example the fit bounds from a source size and a canvas).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/main.rs` (`Held`, `OnScreen`, `present`, `zoom_paint`, the preview canvas's paint closure, `fit_centered`).
    - `Held.px` is the unrotated decode size today and feeds the `page` line's `px=WxH`. Keep `px=` the source's size, since the archived measurements and `docs/humans/performance.md` read it that way.
    - `zoom::placeholder` takes the upright preview size and uses only its aspect ratio. Hand it the source's upright size, so a later scaled decode's rounding cannot move the placeholder.
    - `overlay::paint` and `mark.rs` map source-image coordinates onto the `fit` bounds only, never onto image pixels. Keeping `fit` computed from the source size keeps them exact.
- [x] Step 2: Decode the fit-view preview at the DCT scale the canvas needs, and re-decode when the canvas outgrows it
  - Done when:
    - `load` decodes the preview JPEG at the smallest `n/8` scale (1 to 8, never above 8) whose decoded size is not below the fit of the source in the preview canvas, in device pixels. The canvas size is taken when the read is requested. Page reads (`send`) and prefetch reads (`request_prefetch`) both do this, on the background executor as today. Nothing is resized in paint.
    - With no canvas size yet (nothing painted, `viewport` still `(0, 0)`) the decode is full-size, as today.
    - Growth is handled. When the canvas grows (window resize, pane or strip drag, a pane shown or hidden, a scale factor change) so that the on-screen image's scale is below what the settled canvas needs, the current file is read again at the new scale and replaces the image:
      - after a debounce;
      - with the old image staying on screen until the new one arrives;
      - superseded by a page turn;
      - without a `page` line, a restarted dwell, a `mark_viewed`, or lost `f` overlay marks or meta pane.
    - Ring entries below the needed scale are not left to be shown soft. They are given up and re-read, or a shown one is re-read by the same rule. A canvas that shrinks causes no read.
    - The 1:1 view is unchanged in behavior. Its crop is `zoom.rs`'s own read. Its placeholder is the (now smaller) fit image scaled to the full JPEG's scale, at the same position and size as before.
    - Focus peaking (`peaking.rs`) still keys its fit layer by the on-screen image and drawn size, and recomputes when a re-decoded image replaces the old one.
    - Grayscale is still the sprite flag on the same `paint_image` calls.
    - The pure logic is in a module of its own with unit tests:
      - the scale for a source size and a canvas, at 8 for a source smaller than the fit and for an unknown canvas;
      - with a quarter-turn orientation;
      - mozjpeg's `ceil(native * n / 8)` sizing;
      - whether an image decoded at one scale serves a given canvas.
    - The keyed `page` line reports the decoded size in a new field (for example `decoded=WxH`). On the RAF compare folder in a 1600x1000 window it shows a decoded size near the fit, expected around 1656x1104 at 3/8 and to be confirmed from the line, not 4416x2944.
    - `PREFETCH_PIXEL_CAP`'s doc comment in `crates/app/src/prefetch.rs` describes the decoded sizes it now counts. The cap's value is unchanged.
    - `CLAUDE.md`'s layout names the new module. `docs/agents/app.md` gains or updates the entries on the preview decode, the prefetch ring and the peaking source. `docs/humans/performance.md` and `docs/humans/performance.ja.md` describe the new `page` line field, updated together.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged: the source's upright size already travels beside the image.
    - mozjpeg 0.10.13: `Decompress::scale(numerator)` before `to_colorspace`. `Decompress::width()` / `height()` give the source size before the start; `DecompressStarted::width()` / `height()` give the scaled output. `crates/core/src/decode.rs` already decodes at `n/8` (`scale_near`, private, long edge only); follow its sizing rule. The app needs a both-axes fit, so the rule lives in the app's new pure module (no GPUI types, like `prefetch.rs`).
    - The canvas size is the existing `viewport` cell (`Rc<Cell<(u32, u32)>>`, device pixels, set in the canvas paint). Read it where `send` and `request_prefetch` build their closures and pass it into `load`.
    - The resize hooks exist: `observe_window_bounds`, `drag_pane`, `drag_strip` and `change_panels` already call `zoom_resized`, which debounces with `zoom::CROP_RESIZE_DEBOUNCE`. The fit re-decode can hang off the same call sites. Decide during implementation whether it shares that timer or has its own; `viewport` only updates on the next paint, so the check must run after it.
    - Decide during implementation how the re-read reaches the screen: through `present` with a guard, or a narrower replace-the-image path. `show_current` restarts the dwell and the analysis, and `present` calls `marks.show`, so reusing them as they are would break the "without" list above. The `order` check in `send` is the existing way a newer request supersedes a read.
    - Every image given up (the replaced on-screen image, ring entries dropped for scale) goes through `retire`, so `cx.drop_image` releases its texture.
    - Measure during implementation: the RAF `decode` stage at the reduced scale (libjpeg-turbo has SIMD IDCTs only for some scales), and `--auto 60` on the ARW and RAF compare folders before and after, to see that no stage regressed.
    - Check the scaled image by eye for quality at 3/8, 4/8 and 6/8. GPUI samples the texture bilinearly and the remaining minification is under 2x.
- [ ] Step 3: Measure keypress to the screen with the A/B ruler and record the result
  - Done when:
    - A same-session A/B of two GPUI release builds, A = `main` before Step 2 and B = `main` after it, is run with the procedure of `docs/plans/_archived/20261120-tauri-gpui-latency-ab/results.md` (manual verification; it needs the interactive desktop and an idle machine):
      1. Build both `crates/app` release binaries. Build the ruler with `cargo build --release` in `tools/latprobe` on the local-only branch `measure/tauri-latency-ab` (`1e97309e`).
      2. Per folder (`D:\Photos\tests\20261108-gpui-compare\ARW` and `\RAF`, 20 files each): one unused warm run of each build, then A B A B A B.
      3. Each run: `handpic.exe <folder> --no-prefetch --index <temp> --settings <temp> --log <file>` with stdout captured, then `latprobe --pid <pid> --presses 60 --settle-ms 8000 --out <run>.csv --close`. The probe places the window at (100,100), 1600x1000, and sends Right every 1003 ms. `run-ab.ps1` on that branch drives exactly this for GPUI against Tauri; point its B side at the second GPUI build, or run the same loop by hand.
      4. Idle gate as there: `Win32_Processor.LoadPercentage` <= 15 % before a run and at every 3 s sample after the first, and no `cargo`, `rustc`, `link`, `msbuild` or `cl` process. A failing run is redone in its slot.
      5. Per run, p50 and p95 (sorted index `n / 2` and `n * 0.95`) of the CSV's `settled_ms` (`screen`) and of the `page` lines' `keypressToPixels`. Report the median of the three runs for each, and `screen` minus `keypressToPixels` (`summarize.py` on that branch).
    - Target: on RAF, B's `screen` minus `keypressToPixels` p50 is at the ARW level of the same session (low teens of ms; ARW was 13.55 ms and RAF 47.95 ms in the archived run), and B's RAF `page` lines show the decoded size near the fit.
    - The tables and conditions are written to `results.md` in this plan's folder, with the raw summaries beside it.
    - The `todo.md` entry "Native: the in-app `keypressToPixels` stops about 48 ms before the DWM present on 4416x2944 RAF previews" is updated with the cause and the measured result. It is closed if the gap is gone.
    - The measured timing is added to `docs/agents/app.md`'s Timings section. `docs/humans/performance.md` and `docs/humans/performance.ja.md` are updated together if their RAF numbers are restated.
    - A manual check is done, or recorded in `todo.md` as not exercised. On a RAF folder with prefetch on:
      - paging;
      - `z` (the placeholder, then the crop) and paging while zoomed;
      - `e` in the fit view and at 1:1;
      - a held `g`;
      - `f` with face boxes and the AF mark sitting where they did on `main`;
      - enlarging the window, hiding the side panes and dragging the strip: the image sharpens after the resize settles.
    - If the implementing agent cannot run the measurement (it needs the foreground desktop and an idle machine), this step lands the documentation and records the measurement as a manual check in `todo.md`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged.
    - Only a same-session A/B is a valid comparison on this machine (the archived results' caveat). Do not compare B against the archived numbers.
    - ARW changes too: a 1616x1080 preview in a canvas narrower than it is now decoded below 8/8. So the ARW rows are a second before/after pair, not a fixed baseline.
    - `summarize.py` and `raw/endpoints.py` read the stage fields with `(\w+)=([\d.]+)ms`, so a new non-`ms` field in the `page` line does not disturb them.

## Decisions

Taken with the recommended option at approval (2026-11-21):

- The `n/8` DCT scale alone; an exact CPU resize after the decode is the fallback only if Step 3 misses the target or the quality check fails.
- The target size is the preview canvas, not the window or the display.
- On canvas growth the file is re-read from disk; the full image is not kept.
- The `page` line gains a decoded-size field.
- If the agent cannot run Step 3's measurement, it becomes a manual check in `todo.md`.

## Trade-offs and risks

- **Scaled decode against a CPU resize to the exact drawn size.** The plan takes the `n/8` DCT scale alone. It removes decode work instead of adding a resize to the keyed path, and has a precedent in `crates/core/src/decode.rs`. The image is then up to one `n/8` step larger than the drawn size (about 1656x1104 for a 1160 px wide fit of a RAF preview, about 7 MB, ARW-like) and GPUI's bilinear sampling does the rest. Not taken: a resize to the exact device size after the decode (`image::imageops::resize`, as `peaking.rs` does). It would give a pixel-exact texture and let peaking skip its own resize. But it adds CPU time per page and needs a re-decode on every canvas size change, not only when the canvas crosses an `n/8` step.
- **The target size: the preview canvas against the window or display.** A looser bound such as the display's size would need no re-decode on pane or window changes. On this 5120x1440 display it would upload about 2208x1472 (13 MB) where the canvas needs 7 MB, which may not be enough for the target.
- **Growth: re-read from disk against keeping the full image as source.** Re-reading costs a read and decode after a resize settles and holds nothing extra. Keeping the full decode in memory beside the scaled one would make the re-scale instant, but holds 52 MB per held RAF preview and brings back the full-size decode this plan removes.
- **A soft frame after an enlargement.** Between the canvas growing and the re-decode arriving, the old image is drawn upscaled. This is the same kind of interim the 1:1 view shows with its placeholder.
- **The 1:1 placeholder gets softer on large previews.** The placeholder is the fit image scaled up to the full JPEG's scale until the crop arrives. For a RAF it is drawn from about 1656 px instead of 4416 px, as an ARW's 1616 px preview already is. Position and size are unchanged.
- **Peaking pixels differ slightly.** The fit layer is still resized from the on-screen image to the drawn size, but that image is now DCT-scaled. Lit pixels near the threshold can differ from today's. The rule and the threshold are unchanged.
- **The first image of a launch is decoded full-size.** The canvas size is unknown before the first paint. It costs the old upload once and is never re-decoded, since larger than needed is acceptable.
- **The target is a cross-format comparison.** "ARW-like levels" assumes the gap scales with the uploaded bytes, as the investigation found. If Step 3 shows a remaining gap at ARW-sized uploads, the cause is elsewhere and outside this plan. Whether the in-app `present` stamp should move (the other half of the `todo.md` entry) is not addressed here.
- **Prefetch cap.** `PREFETCH_PIXEL_CAP` stays 32 MP but now counts decoded pixels, so neighbours that were too large to share it (whole JPEGs of a view-only folder) fit. Lowering the cap is not part of this plan.

## Progress

(2026-11-21) Step 1 complete
