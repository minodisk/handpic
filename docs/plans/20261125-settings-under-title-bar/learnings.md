# Learnings: settings layer under the title bar

## Step 1

- The layer's wrapper in `Handpic::render` is now `absolute`, `top(TITLE_BAR_HEIGHT)`, `left_0`, `right_0`, `bottom_0`; `settingsview.rs`'s box keeps `size_full` and gained a `settings-box` `debug_selector` for the UI test `the_settings_layer_lies_under_the_title_bar_and_fills_the_rest`, which also clicks inside the title bar with the settings open and finds them still open (the gear stays a no-op, as `open_settings` returns while open).
- The test compares against `window_paddings` rather than assuming zero, so it holds whether or not the test window reports a client-side frame.
- A bare `cargo test` inside `crates/app` picked a toolchain too old for `gpui-pre-macos` 0.3.12 (`result_option_map_or_default` unstable). Run the app's tests through `mise run ci:local` (or `mise x rust@1.98.1 -- cargo ...`), which pins the app's toolchain.
- File drop on the title bar while the settings are open: not measured. From the hit test it reaches the root `div`'s `on_drop` and opens the folder under the open settings; recorded as Inferred in `docs/agents/app.md`, no guard (user's choice).

## Deferred issues (todo candidates)

- Pending manual check (macOS, `mise run app:dev`): open the settings with `cmd+,`, then (1) drag the window by the title bar and confirm it moves, (2) confirm the traffic lights no longer overlap the "Settings" heading (the heading starts below the 34 px bar), (3) press the gear while open and confirm nothing happens, (4) start a shortcut capture on the Keyboard Shortcuts page, drag the window by the title bar and confirm the capture state still reads sensibly. Step 1's checkbox was ticked on the automated criteria (the UI test of the layer's bounds). Basis: plan Step 1 "Done when"; files `crates/app/src/main.rs`, `crates/app/src/settingsview.rs`.
- Pending manual check (Windows and Linux, a device): with the settings open, the window controls (minimize / maximize / close) at the title bar's right and the window drag work. Not checked on a device here. Basis: plan Step 1 "Done when"; file `crates/app/src/main.rs`.
- Pending manual check (any platform): drop a file or folder onto the title bar while the settings are open and record whether it opens that folder under the settings (expected from the hit test: it does). Update `docs/agents/app.md`'s pointer rule from Inferred to Measured. Basis: plan "Trade-offs and risks", drop on the title bar; files `crates/app/src/main.rs`, `docs/agents/app.md`.
