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

## Deferred issues (todo candidates)

- **Timing flake in `riffle-core`'s scan tests.** In the plan PR's local CI,
  `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` hit its
  1 s bound under parallel load (2.91 s; 0.38 s when run alone). Consider
  loosening the bound. Source: the plan PR's local `mise run ci` (reported by
  the develop flow when Step 1 was handed out). File:
  `crates/core/src/scan.rs`, around line 1456.
