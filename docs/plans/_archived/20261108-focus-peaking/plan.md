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

# Focus peaking in the preview

## Purpose

Riffle judges focus today through the `f` focus mark (the AF frame and its
color from the face analysis) and the `z` 1:1 crop. Neither shows *where* the
image is sharp across the frame. Focus peaking, the colored highlight of
strong edges that cameras draw in their viewfinders, does: with it on, the
in-focus plane stands out in the fit view at a glance and the 1:1 crop shows
whether the eye, not the ear, carries the edges. Nothing like it exists in the
repo yet (no "peaking" anywhere).

The feature is a session-only view toggle on a key, like the focus mark
(`f`), Compare (`v`) and the performance graph (`d`): off at startup, not
persisted, no menu item and no button. The edge measure is a Sobel magnitude
on luminance against one fixed threshold, computed only for the file being
shown, at the resolution it is drawn at, never during the scan.

The default key is `e` (decided by the user at plan approval).

## Steps

- [x] Step 1: Add the `peaking` toggle: the pure edge module with tests, the
      overlay in the fit view and the 1:1 crop, the keymap action and the docs
  - Done when:
    - Pressing the `peaking` key toggles colored edge highlights over the
      shown image in the fit view and in the 1:1 (`z`) crop; the highlights
      sit on the image's edges under every orientation (1, 3, 6, 8) and at
      every window size / `devicePixelRatio`; pressing it again removes them
      with no trace. Compare draws none (like faces).
    - Paging with peaking on shows the new file's overlay only; a result
      computed for an older file or an older crop is never drawn (checked by
      holding `ArrowRight` through a folder).
    - The state is session-only (off on launch), the action appears in the
      settings' Shortcuts tab with its default key and can be rebound like
      every other action.
    - `crates/app/ui/src/peaking.ts` holds the pure computation and
      `peaking.test.ts` covers it (see below); `mise run ci` passes, including
      the Rust tests of `shortcuts.rs` and `main.rs` that enumerate the
      actions.
    - `README.md` / `README.ja.md` (Key features, next to the `1:1 focus
      check` and `Focus mark` bullets), `docs/humans/usage.md` /
      `docs/humans/usage.ja.md` (the feature bullet next to `1:1 focus check`
      / `Grayscale preview` and a row in the `Keys` table) and `CLAUDE.md`
      (the `crates/app/ui` layout list gains `src/peaking.ts`) describe it,
      each `.ja` pair updated in the same PR.
  - Implementation approach (as far as it is known):
    - **Pure module `crates/app/ui/src/peaking.ts`** (vitest runs with
      `environment: "node"`, so no canvas / DOM here): one function that takes
      RGBA pixels (`Uint8ClampedArray`), `width`, `height` and returns an RGBA
      overlay of the same size, transparent everywhere except the peaking
      color where the edge magnitude clears the threshold. Luminance by
      Rec. 601 (`0.299 R + 0.587 G + 0.114 B`), 3x3 Sobel `gx` / `gy`,
      magnitude `hypot(gx, gy)` (or `|gx| + |gy|` if measurably faster; say
      which in a comment). The border row / column has no full neighborhood;
      leave it unlit. Export the threshold and the color as named constants
      (`PEAKING_THRESHOLD`, `PEAKING_COLOR`, an opaque bright red or magenta
      that is not already used by the focus mark's green / cyan or the face
      mesh's magenta; pick one and keep it); no other configurability. Tune
      the threshold by eye on the RAW samples under `D:\photos\samples\`
      (sharp eye lit, smooth skin and bokeh dark, noise in shadows mostly
      dark) and record the value tried in `learnings.md`.
    - **Tests** (`peaking.test.ts`, `describe` / `test` like
      `decode.test.ts`): a flat image lights nothing; a vertical step edge
      lights exactly the columns adjacent to the step and nothing else; a
      horizontal step the same by rows; a step smaller than the threshold
      lights nothing; the 1-pixel border stays unlit; the output length is
      `width * height * 4` and the lit pixels carry `PEAKING_COLOR` with alpha
      255; a 1x1 / 2x2 image does not throw.
    - **Fit view (`draw()` in `main.ts`, ~line 1924):** the overlay must be
      at the displayed resolution and follow the canvas's
      `translate / scale(dpr) / rotate` transform. Rasterize `shown.bitmap`
      into an `OffscreenCanvas` of `round(drawWidth * dpr) x round(drawHeight
      * dpr)` (unrotated bitmap axes, device pixels), `getImageData`, run the
      module, `putImageData` into a second `OffscreenCanvas` (or
      `createImageBitmap(imageData)`), and inside the existing transform
      `drawImage(overlay, -drawWidth / 2, -drawHeight / 2, drawWidth,
      drawHeight)` right after the bitmap and before `drawFocusMark`, so the
      marks stay on top. Cache the overlay keyed by `shown.seq` and the
      rasterized size; invalidate (and `close()` any bitmap) where `shown` is
      replaced (the worker message handler, ~line 3594) and where it is
      nulled (~2437, 3406, 4332). Do not compute while peaking is off; on
      toggle-on just call `draw()`.
    - **1:1 crop (`drawZoom()` ~line 3226, crop arrival ~3300-3355):** the
      `focus_crop` payload is already raw RGBA at 1:1 in device pixels, so
      build the overlay from that `ImageData` directly (no re-rasterization)
      and draw it at `(-crop.pointX, -crop.pointY)` after `crop.bitmap`. Keep
      the overlay on the `crop` object (it already carries `cropSeq`, so the
      existing `crop.cropSeq === seq` check guards staleness); compute it
      lazily on the first `drawZoom()` with peaking on, or on arrival when
      peaking is on, and `close()` it with the crop. The placeholder (the
      scaled preview drawn while the crop is in flight) gets no overlay: it
      is at a different scale and would flash a mismatched pattern.
    - **Staleness:** with synchronous computation (the default below) nothing
      can be stale: `draw()` always computes from the current `shown` /
      `crop`, both already `seq`-guarded. If a worker is introduced, tag each
      request with `shown.seq` (fit) or `cropSeq` (crop) and drop any reply
      whose tag differs from the current one, exactly as the decode worker's
      handler does (`responseSeq !== seq` -> close and return).
    - **Sync vs worker, decided by measurement:** implement synchronously
      first, with `debugLog` timing of the compute (like the `page ... decode=`
      lines). Measure on a 4K / 2x display at full-window size (about 8 M
      device pixels in the fit view). If the fit-view compute stays within
      about one frame (~16 ms) with the cache so it runs once per file, keep
      it synchronous. If it does not, move the module call into a dedicated
      worker (`crates/app/ui/src/peakingworker.ts`, built like `worker.ts`
      with `new Worker(new URL("./peakingworker.js", import.meta.url), { type:
      "module" })`), transfer the `ImageData` buffer, and apply the
      seq-tagging above; the overlay then appears a moment after the image,
      as the faces do. Record the measured numbers in `learnings.md`.
    - **Keymap:** add `("peaking", &["e"])` to `DEFAULTS` in
      `crates/app/src/shortcuts.rs` after `("performance", &["d"])` and to the
      mirrored list in its `defaults` test; add `"peaking"` to `MENU_LESS` in
      `crates/app/src/main.rs` (its comment lists the key-only view toggles;
      extend it); add `peaking: "Focus peaking"` to `shortcutLabels` in
      `crates/app/ui/src/settings.ts`; add `case "peaking"` to `runAction` in
      `main.ts` next to `focus`, flipping a `let showPeaking = false` declared
      beside `showFocus` (~line 703) and calling `draw()`. Check `e` is not in
      `MACOS_MENU` / `MACOS_SYSTEM` (it is not today).
    - **Out of scope, say so in the PR:** Compare view, the companion's view
      state (`companion.ts` `ViewMode` stays `normal | zoom | compare`), any
      persistence, color / threshold settings, and the grayscale key's CSS
      filter (see risks).

## Trade-offs and risks

- **Default key.** `e` (edges), chosen by the user: free in `DEFAULTS`,
  `MACOS_MENU` and `MACOS_SYSTEM`, and next to the other single-letter view
  toggles. Users can rebind it in the Shortcuts tab.
- **Synchronous compute vs a worker.** Sync is simpler, has no staleness
  surface and the cache makes it a one-off per file, but a Sobel over ~8 M
  pixels in JS may take 30-80 ms on a slow machine, felt as one dropped frame
  on each page turn while peaking is on. A worker keeps the UI smooth but
  adds a second worker file, transfer plumbing and a visible delay before the
  overlay appears. The step measures and decides.
- **Resolution the Sobel runs at.** Computing at displayed (device-pixel)
  resolution keeps the overlay aligned and thin, as asked, but the fit view
  is a downscaled preview, so soft-but-not-sharp areas can light up at small
  windows where downscaling sharpens edges. Computing on the stored preview
  and scaling the overlay would be scale-independent but blurry when
  upscaled. The plan follows the request (displayed resolution); note in
  `learnings.md` if the threshold has to differ between fit view and crop,
  which would be a reason to allow two constants.
- **`g` grayscale.** It is a CSS `filter: grayscale(1)` on the whole `#canvas`
  (`crates/app/ui/style.css`), so holding `g` grays the peaking color too.
  Accepted: `g` is momentary. Avoiding it would need a second canvas layer or
  a per-draw grayscale of the bitmap, out of scope.
- **Linux preview pixel limit.** `preview_pixel_limit` downsizes the decoded
  bitmap on WebKitGTK; the fit-view overlay is rasterized from that bitmap at
  the draw size, so alignment holds, only the source is softer there. No
  action needed, just do not assume `shown.bitmap` is the stored preview's
  size.

## Progress

- 2026-11-08: Step 1 done. The worker was chosen from measurement (see
  `learnings.md`). The hands-on check (alignment under orientations and dpr,
  threshold tuning, the 4K raster timing) is still waiting on the user, so the
  ticked box is not fully verified yet.
