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

## Step 3

- `Index::entries` is `files LEFT JOIN ratings`, so on a fresh index the
  open's read (right after `reconcile_of`) returns nothing: the files' rows are
  pass 1's. The view therefore reads the judgments at the open and again at
  pass 1's `Done` (`read_judgments`, dropped when another listing came in
  meanwhile). The plan named only the open and the re-listing. `rows_of` (the
  `ratings` rows alone) would show the marks before pass 1; kept on `entries`
  because Step 4's `label_known` follows the Tauri "entry seen" rule.
- No separate "seen" set: the view's `judgments` map holds one `State` for
  every entry the index returned, so its keys are the seen paths. Step 4 has
  to keep that true when it inserts a judged path (or split the set out then).
- The overlay's `Marks` (the `f` toggle) and the strip's new `Marks` (a
  cell's stars / flag / tint) share a name; `main.rs` imports the strip's and
  names the overlay's `overlay::Marks`.
- Coexisting keys: the handler takes `home`, `end` and `q` literally, then the
  keymap's actions; `f` became the keymap's `focus` action (its default key),
  not a literal, so the overlay keeps working and follows a user override.
  When the zoom / peaking sessions land (`z` / `e`), add them as `zoom` /
  `peaking` arms of the same `match` (both are keymap actions already); a
  literal `"e"` / `"z"` arm placed after the action arms would never fire with
  the default keymap, because the catch-all `(_, Some(action))` arm takes it.
- GPUI on Windows (read in `gpui_windows/src/keyboard.rs` at the pinned rev):
  a shifted digit or punctuation key comes as the typed character with
  `shift` cleared (`!`, not `shift+1`), where `keys.ts` names it from
  `event.code`. No default binding uses one; recorded in `native-app.md`.
- `logger.rs` now honours `RUST_LOG` (`debug` / `trace`) for this crate's
  own lines only, so the `key: <name> action=...` line can be read from stderr
  without GPUI's debug output.
- `serde_json` became a direct dependency (already in the lock through the
  backend) and `tempfile` a dev-dependency; `Cargo.lock` gained only the two
  dependency lines.
- Headless check (debug build, `--auto`, no window looked at) on
  `D:\Photos\tests\20261110-gpui-judgments\read\lr-check-4-lightroom-desktop`
  (a timestamp-preserving copy of `D:\Photos\tests\lr-check-4-lightroom-desktop`,
  8 DNGs, each with an XMP and a `.dop`): with `settings\xmp.json`
  (`{"sidecarFormat":"xmp"}`) and a fresh `index\read-xmp.sqlite`, the first
  open logged `scan sidecars: ... dirty=0 changed=8` and the second
  `changed=0`; the same with `settings\both.json` and `index\read-both.sqlite`.
  The `ratings` rows hold 3 stars + pick + `Yellow`, 4, 3, 2, 1 stars, a pick,
  a reject and `Red`. `ls -l --time-style=full-iso` of the 16 sidecars was
  identical before and after the four runs.
- One open logged `scan reconcile: ... in 11059ms` for 8 files (the others
  0-1382 ms): `reconcile` is unchanged by this step and the D: drive was
  shared with other sessions; not investigated.

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

- **Pending manual check (Step 3, Windows, the user's eyes and hands).** The
  step's checkbox was ticked on the automated criteria. Build
  (`mise exec -- cargo build` in `crates/native`, NASM on `PATH`), then:
  1. Strip marks against the Tauri strip. Run, with `RUST_LOG=debug` set,
     `target\debug\riffle-native.exe
     D:\Photos\tests\20261110-gpui-judgments\read\lr-check-4-lightroom-desktop
     --index D:\Photos\tests\20261110-gpui-judgments\index\manual-xmp.sqlite
     --settings D:\Photos\tests\20261110-gpui-judgments\settings\xmp.json`
     (a new index file name each time). Expected once pass 1 ends: L1005036
     three gold stars at the image box's top-right, a green dot at its
     top-left and a yellow name band; L1005037-40 four to one stars;
     L1005041 a green dot; L1005043 a red dot and the cell dimmed; L1005044
     a red name band. Compare with the Tauri app opened on the same copy
     (press no judgment key there). Then the same with `settings\both.json`
     and another fresh index: the `.dop` is newer for most files, so the marks
     follow `reconcile_of`'s newest-wins pick; compare with the Tauri app set
     to "both". Check that the star glyph renders (font fallback) and reads
     over a bright thumbnail. Afterwards `ls -l --time-style=full-iso *.xmp
     *.dop` in the folder still matches
     `D:\Photos\tests\20261110-gpui-judgments\read\sidecars-before.txt`.
  2. Key names. In the same run press Shift+X, Ctrl+Alt+1, Ctrl+Alt+0,
     Ctrl+Z, Ctrl+Shift+Z, `1`, `x`, Shift+1 and the arrows, and read the
     `[riffle_native][DEBUG] key: <name> action=...` lines on stderr.
     Expected: `shift+x` -> `rejectRest`, `ctrl+alt+1` -> `red`,
     `ctrl+alt+0` -> `clearlabel`, `ctrl+z` -> `undo`, `ctrl+shift+z` ->
     `redo`, `1` -> `rate1`, `x` -> `reject`, Shift+1 -> `!` with no action,
     the arrows page. Nothing is written (no sidecar mtime changes). Record in
     `docs/agents/native-app.md` what arrived, and whether Ctrl+Alt+digit
     reached the window at all (on a layout with AltGr it is AltGr+digit).
