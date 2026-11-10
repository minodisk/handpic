# Learnings: gpui-judgments

## Step 1

- `exif::metadata` takes `&Shot` rather than the `Shot` `read_metadata` used
  to consume, so the native `load`, which keeps the shot for other uses, can
  call it without a clone of the whole struct; only `capture_time` is cloned.
- `crate::sidecar::*` in `crates/app` is the backend module (`main.rs` imports
  `riffle_backend::{exif, index, shortcuts, sidecar, trash}` at the crate
  root), so importing `label_names_setting`, `label_names_value` and
  `SidecarError` into `commands.rs` kept the app's remaining tests, which name
  them as `super::…`, unchanged.
- Most of the app's sidecar reconcile tests are integration tests over
  `scan_folder`'s listing helpers (`list_folder_in`, `list_arw_in`,
  `index_files`) and stay in `crates/app`, now calling
  `crate::sidecar::reconcile_of`. The two that test `reconcile_of`'s own
  rules (the oversize `.dop` and the unparseable sidecar) and the
  `labelNames` fallback test moved to `crates/backend/src/sidecar.rs`, with a
  small `reconcile_listed` helper over `listing::read_listing`.
- `LEGACY_IDENTIFIERS` has no reader until Step 3's `settings.rs`, so it
  carries `#[allow(dead_code)]` until then (the native crate is a binary, so
  an unused `pub` item still warns).

## Step 2

- The pane sits beside the preview canvas above the strip, not beside the
  whole window: the strip keeps spanning the window, so `strip_width` (the
  window's width) and `hit_test` on the event's `x` stay as they were.
- `meta::groups` borrows: `Row { label: &str, value: &str }` over the
  `Metadata` and the caller's `analysis` slice, so `render` allocates only the
  strings it hands to the `div`s. The pane's file is a `(index, Shown)` set
  where the preview image is (`Shown::Read(Metadata)` or `Shown::Failed { name,
  error }`), so the pane always describes the image on screen, and is cleared
  with it on an empty listing.
- Pane cost, **noisy figures**: release, `--auto 20` on
  `D:\Photos\tests\20261108-gpui-compare\ARW`, a fresh `--index` per run under
  `D:\Photos\tests\20261110-gpui-judgments\index\`, before (`f14e49ac`) and
  after interleaved (before, after, before, after) in one session while two
  other sessions were editing and building `crates/native`. CPU
  `LoadPercentage` sampled just before each run: 12, 15, 31, 23 %. p50 in ms:

  | run      | paint | present | frameWait | keypressToPixels |
  | -------- | ----- | ------- | --------- | ---------------- |
  | before 1 | 0.70  | 0.19    | 9.14      | 17.97            |
  | after 1  | 0.90  | 0.44    | 7.86      | 18.02            |
  | before 2 | 0.82  | 0.22    | 8.23      | 18.65            |
  | after 2  | 0.76  | 0.41    | 8.73      | 18.52            |

  `paint` moved +0.20 and -0.06 ms, so the two rounds do not agree on any
  growth, let alone over 1 ms: the `div` rows stay. `present` rose about
  0.2 ms in both rounds: `paint` times only the preview canvas's closure, and
  the pane's elements paint after it in the same frame. Step 5's same-session
  A/B is the real gate.
- Tripped up: the first measurement script quoted the `--index` path as
  `"$I\\$b-$round.sqlite"`, where bash turned `\$` into a literal `$`, so all
  runs of a round shared one file named `index$b-1.sqlite` (the "after" runs
  were warm). Those figures were discarded and the files removed; forward
  slashes in the Windows path avoid the escape.
- A headless `--auto 3` run of the after build on
  `D:\Photos\tests\2026-09-28-gui-check-jpeg` (10 JPEGs, fresh index) logged no
  error; its pane was not looked at.

## Deferred issues (todo candidates)

- **Pending manual check (Step 2, Windows, the user's eyes).** The step's
  checkbox was ticked on the automated criteria. Build
  (`mise exec -- cargo build --release` in `crates/native`, NASM on `PATH`),
  then:
  1. Run `riffle-native.exe D:\Photos\tests\20261108-gpui-compare\ARW --index
     D:\Photos\tests\20261110-gpui-judgments\index\manual-arw.sqlite`. Expected:
     a 220 px pane right of the preview shows the file name, `n / 20`, an
     `EXIF` group (Aperture, Shutter, ISO, Focal length, Exposure, Camera,
     Lens, Captured, each present in the file) and a `Maker note` group for a
     Sony file; arrow keys update it with the preview; no notes line.
  2. Run it on a folder of JPEGs under `D:\Photos\tests\` (for example
     `D:\Photos\tests\2026-09-28-gui-check-jpeg`) with
     `--index D:\Photos\tests\20261110-gpui-judgments\index\manual-jpeg.sqlite`.
     Expected: `EXIF` rows only, no `Maker note` group, and
     `JPEG folder: view only` at the bottom of the pane.
  3. Optionally, on a copy with a corrupt file, the pane shows the file name
     and the error alone.

- **Timing flake in `riffle-core`'s scan tests.** In the plan PR's local CI,
  `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` hit its
  1 s bound under parallel load (2.91 s; 0.38 s when run alone). Consider
  loosening the bound. Source: the plan PR's local `mise run ci` (reported by
  the develop flow when Step 1 was handed out). File:
  `crates/core/src/scan.rs`, around line 1456.
