# Learnings

## Step 1

- Version: the workspace is at `1.10.0` (`crates/app`, `crates/core`, `crates/cli`), so the backend uses that.
- Test counts on Windows, `cargo test -p riffle-app -- --list`: before 476; after 394 in `riffle-app` + 82 in `riffle-backend` = 476. The sum holds. `riffle-backend`'s 82 are `exif` 22, `shortcuts` 50 and `photolab` 10.
- The plan's 498 and 92 are textual `#[test]` counts. They include each module's `cfg`-gated tests: `photolab.rs` has 20 `#[test]`, 10 in `#[cfg(all(test, windows))]` and 10 in `#[cfg(all(test, target_os = "macos"))]`. Expect platform-sized numbers in later steps too (macOS and Windows differ).
- Doc-tests: `riffle-app` is a bin, so it has none; `riffle-backend` runs `Doc-tests` with 0 tests.
- Root `Cargo.lock`: only the new `riffle-backend` package and `riffle-app`'s new dependency line on it, no version change. `rusqlite` stays in `riffle-app` for `index.rs` until Step 2.
- `crates/app/Cargo.toml` gained `riffle-backend = { path = "../backend" }` (not spelled out by the plan's Done-when, but needed).
- `crates/native/Cargo.lock` gained, via `cargo` only: `fallible-iterator 0.3.0`, `fallible-streaming-iterator 0.1.9`, `hashlink 0.12.2`, `libsqlite3-sys 0.38.2`, `riffle-backend 1.10.0`, `rsqlite-vfs 0.1.1`, `rusqlite 0.40.2`, `sqlite-wasm-rs 0.5.5`, `vcpkg 0.2.15`. Nothing became a duplicate version.
- `mise run native:check` with the backend: 44 s locally, incremental. The time of the native CI job with the new bundled SQLite is known only once the PR's CI has run.
- Clippy under 1.98.1 said nothing about the backend (a non-member of the native workspace, so not linted there). `cargo fmt --all` at the root formats the backend as a member, and `lint`'s native fmt line needed no change.
- `mise run native:check` runs `clippy` and `cargo test`, which do not produce `target/debug/riffle-native.exe`. Running that exe right after it hit a stale binary that sat on "no folder" and never quit. Run `cargo build` first; both the rebuilt pre-change binary and the post-change binary then ran and quit normally.
- Native link proof at run time: `riffle-native D:\photos\samples\ARW --auto 3 --max 4` showed the status line `4/4  DSC-RX0M2_sony_rx0_ii_29.arw  1616x1080  1  1/50  f/4  160`.
- Tauri smoke check, performed through `mise run tauri:dev`: the debug app launched and reopened the 2026-11-07 folder at `_DSC1508.ARW`; a screenshot showed the meta pane's `EXIF` rows (Aperture f/2, Shutter 1/1000, ISO 2500 and the rest) and `Maker note` rows. No `panic` or `invariant` line in the dev log. The Shortcuts rebind and the `.dop` / PhotoLab parts were not performed; see below.

## Deferred issues (todo candidates)

- **Pending manual check: rebinding a key in the Shortcuts tab (Windows).** From `gpui-backend-crate` Step 1's Tauri smoke check, not performed because it needs a person driving the app and changes the user's stored keymap. Step's checkbox was ticked on the automated criteria. Steps: `mise run tauri:dev` with the Step 1 change, open Settings, Shortcuts tab, rebind one action to an unused key, press the new key. Expected: the action fires on the new key, and the old key no longer fires it. Afterwards restore the binding. File: `crates/backend/src/shortcuts.rs`.
- **Pending manual check: a fresh `.dop` in a PhotoLab-registered folder (Windows, DxO PhotoLab installed).** From the same smoke check, not performed for the same reason (it writes a sidecar and needs PhotoLab open). `photolab.rs`'s 10 Windows tests cover the lookup. Step's checkbox was ticked on the automated criteria. Steps: with format `Dop` or `Both`, rate a file with no `.dop` in a folder PhotoLab has already indexed, then open that folder in PhotoLab. Expected: the image shows with its rating and no virtual copy. File: `crates/backend/src/photolab.rs`.
- Stale paths in `todo.md`: `crates/app/src/shortcuts.rs` and `crates/app/src/photolab.rs` are still named at about lines 335, 370, 405, 485, 1347, 1360, 1364, 1773 and 1789. Found while checking the moved paths in Step 1; the files are now under `crates/backend/src/`. Editing `todo.md` is the wrap-up phase's job.
