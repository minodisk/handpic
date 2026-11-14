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

# Restore the window and the last folder in the GPUI app

## Purpose

The Tauri app comes back where it was left: `tauri-plugin-window-state`
restores the window's position, size and maximized state, and the `lastFolder`
key of the settings store reopens the folder of the previous session
(`remember_folder` / `last_folder` in `crates/app/src/commands.rs`,
`reopenLastFolder` in `crates/app/ui/src/main.ts`). The native app
(`crates/native`) does neither: it always opens centered at 1600x1000 and
opens a folder only when one is given on the command line. This is a gap
before the switch-over of Step 10 of `../20261109-gpui-port/plan.md`.

Once this is done, quitting the native app with a folder open and launching
it again without arguments reopens that folder (at its last viewed file,
through the existing `folders.last_viewed` resume) in a window at the same
place, size and maximized state.

### What the investigation found (2026-11-14, `main` at `cb674451`)

- **The native app today.** `main()` in `crates/native/src/main.rs` opens the
  window with `Bounds::centered(None, size(px(1600.), px(1000.)), cx)` and
  `WindowBounds::Windowed`, with no `display_id`. `Riffle::start` opens only
  `Args.folder`. `Riffle::new` already has a `cx.observe_window_bounds`
  observer (for the strip) and an app-level `App::on_app_quit` handler that
  drains `settings.writes`.
- **The Tauri rule for the folder.** `remember_folder` writes `lastFolder` on
  every folder open (not on quit); `last_folder` returns it only when it is
  still a directory, else nothing opens. A new native settings store is
  seeded verbatim from a Tauri `settings.json`, so it may already hold
  `lastFolder`; `settings.rs`'s `from_values` does not read it yet.
- **GPUI at the pinned rev (`b5cd310`).**
  - `Window::window_bounds()` returns `WindowBounds::{Windowed, Maximized,
    Fullscreen}`, each carrying the restore bounds. On Windows it comes from
    `GetWindowPlacement` (`rcNormalPosition`, so also right while minimized or
    snapped); macOS never reports `Maximized`.
  - Opening with `WindowBounds::Maximized(restore)` places the window and
    then zooms it (`platform_window.zoom()`); `Fullscreen` toggles fullscreen.
  - `cx.displays()` gives each display's `id()`, `uuid()` (a `Result`),
    `bounds()` and `visible_bounds()`; `Window::display(cx)` the window's
    display; `Bounds::intersects` exists.
  - **Windows checks the bounds itself, against one display only.**
    `retrieve_window_placement` takes the display of
    `WindowOptions::display_id`, else the primary monitor, and keeps the given
    bounds only when their center is on that display
    (`check_given_bounds`); otherwise it silently uses that display's
    `default_bounds()` (GPUI's `DEFAULT_WINDOW_SIZE`, not 1600x1000). So
    bounds saved on a secondary monitor are discarded unless `display_id`
    names that monitor. Zed handles this by storing the display's UUID and
    looking it up in `cx.displays()` (`build_window_options`).
  - On Windows a window's bounds are logical pixels at the scale factor of
    the display it is on, origin included (`logical_point`), so with mixed
    DPI the bounds are only meaningful together with their display.
  - `observe_window_bounds` fires on moves as well as resizes (`on_moved` and
    `on_resize` both call `bounds_changed`).
  - `display.uuid()` returns a `uuid::Uuid`; the native crate does not depend
    on `uuid`, and its `to_string()` is enough to store and compare.

## Steps

- [x] Step 1: Restore the window geometry and the last folder on launch
  - Done when:
    - Quitting with a folder open and relaunching without arguments reopens
      that folder, revealed in the tree, at its last viewed file (the
      existing resume). A folder given on the command line wins over the
      remembered one. A remembered folder that is no longer a directory opens
      nothing (`no folder`), as in the Tauri app.
    - The window reopens at the position and size it had at the last quit,
      on the same display, maximized if it was. When the saved display is
      gone and the saved bounds intersect no display, or nothing valid is
      stored, it opens at the centered 1600x1000 default.
    - The rules are pure functions with unit tests: the startup folder
      choice, the stored value's parse and round trip (malformed, non-finite
      or non-positive sizes read as nothing), the display choice (the saved
      UUID first, else an intersecting display, else none) and the placement
      within the chosen display.
    - `mise run ci` and `mise run native:check` pass (the `native` CI job on
      the three platforms).
    - `docs/agents/native-app.md` and the `crates/native` part of `CLAUDE.md`
      describe the new module, the two keys and the rules below. The
      "A launch without a folder shows only the tree" section is updated.
    - The Windows hand check below is done and its results are recorded in
      `learnings.md` (and as `(Measured)` in the guide where it states a
      fact).
  - Implementation approach (as far as it is known; omit if unknown):
    - **A new pure module `crates/native/src/session.rs`** (no GPUI types, so
      it is testable as `strip.rs` and `viewed.rs` are):
      - `startup_folder(cli, remembered, suppressed, is_dir)`: the command
        line's folder, else the remembered one when it is a directory and
        restore is not suppressed.
      - A `Geometry` value (x, y, width, height as `f32`, `maximized: bool`,
        `display: Option<String>`, the display's UUID as text) with
        `from_setting(Option<&Value>) -> Option<Geometry>` and
        `setting() -> Value`, in the style of `SortKey::from_setting` /
        `Stored::from_setting`.
      - `place(saved, displays) -> Option<(index, rect)>` over plain
        `(uuid, bounds, visible_bounds)` rows: the display whose UUID matches
        when the saved bounds intersect it, else the first display the
        bounds intersect, else `None` (the caller's centered default). The
        returned rect is the saved one moved and shrunk to lie within that
        display's visible bounds. That guarantees the Windows center check
        passes, so GPUI never substitutes its own default size. See the
        trade-off on clamping.
    - **`crates/native/src/settings.rs`**: `from_values` reads `lastFolder`
      (a string) into `Settings.last_folder` and `windowBounds` into
      `Settings.window: Option<Geometry>`. Both are written through the
      existing `Settings::writes` (`Writes::set`), never on the main thread.
      The `lastFolder` key and its plain-string value are the Tauri app's, so
      a seeded store restores the Tauri app's last folder on the first native
      launch.
    - **`crates/native/src/main.rs`**:
      - In `main()`, before `Riffle::start(args, cx)`, replace `args.folder`
        with `session::startup_folder(...)` (`Path::is_dir` as the check).
        A restored folder opens through the same `open` as the command
        line's, with no `--size` / `--max` effect of its own.
      - Build the window options from `settings.window`: collect
        `cx.displays()` into the pure rows, call `place`, and pass
        `display_id` of the chosen display together with
        `WindowBounds::Maximized(rect)` or `Windowed(rect)`. Without a
        placement, keep today's `Bounds::centered(None, 1600x1000)`.
      - Remember the folder where a listing succeeds and sets `self.dir`
        (`self.dir = Some(listed.dir.clone())`, the one place it is set),
        only when it differs from the last value written, so a watcher
        re-listing writes nothing.
      - Track the geometry in the existing `observe_window_bounds` callback:
        read `window.window_bounds()` and `window.display(cx)`'s UUID into a
        shared cell (an `Arc<Mutex<Option<Geometry>>>`), and write it in the
        existing settings `App::on_app_quit` handler just before
        `writes.drain(...)`, when it differs from what was loaded. The
        handler must hold the shared cell, not the view: closing the window
        may release the view before the quit handlers run (see "The writer
        drains in `App::on_app_quit`" in the guide). A `Fullscreen` value is
        stored as its restore bounds, not maximized.
      - `--auto` suppresses both restores and both writes (see the
        trade-off); thread that one flag to the places above. `args.rs`
        needs no new flag.
    - **Verify during implementation (Windows hand check, each run with
      `--settings <fresh test store>` and `--log <test file>` per the guide's
      rule, never the real store):**
      1. Move and resize the window, quit with `q`; relaunch: same place and
         size. Repeat quitting with the title bar's close button.
      2. Maximize, quit, relaunch: maximized; un-maximizing gives the
         earlier windowed bounds back.
      3. Quit while minimized, and while snapped to a screen half: relaunch
         is on-screen at the restore bounds.
      4. On a second monitor (a different scale factor if available): the
         window comes back on that monitor, windowed and maximized.
      5. Edit `windowBounds` in the test store to bounds far off every
         display, and to a display UUID that does not exist: the centered
         default, respectively the intersecting display.
      6. Open a folder from the tree, page a few files, quit, relaunch
         without arguments: that folder at that file. Relaunch with another
         folder as the argument: the argument's. Rename or delete the
         remembered folder outside the app, relaunch: `no folder`.
      7. `--auto 3 <folder>`: opens centered at 1600x1000 whatever is
         stored, and the stored `windowBounds` / `lastFolder` are unchanged
         afterwards.
      8. Measure whether the restored position drifts by the taskbar's size
         with the taskbar docked at the top or left (`rcNormalPosition` is in
         workspace coordinates); record the result, fix only if it drifts.
    - macOS and Linux are covered by the CI build and the unit tests only;
      say so in the guide rather than claiming behavior there.

## Trade-offs and risks

Decisions 1-6 below were approved by the user as recommended (2026-11-14).

- **Where the geometry is stored.** Chosen: a `windowBounds` key in the
  native settings store. The store, its ordered writer thread, the quit drain
  and the `--settings` test isolation already exist, and a hand check with a
  test store then exercises the real path. The alternative is a separate file
  (as the Tauri plugin's `.window-state.json`): it keeps window state out of
  "settings", but needs its own path flag, writer and quit handling. Note the
  store's rule that a store that cannot be opened means nothing is written:
  then neither the geometry nor the folder is remembered, which matches every
  other setting.
- **When the geometry is written.** Chosen: once, in the quit handler, from a
  cell the bounds observer keeps current. A drag fires the observer
  continuously and each `Writes::set` is one SQLite write, so writing in the
  observer would need a debounce. The cost of the quit-only write is that a
  crash or a kill loses that session's geometry (the folder is written on
  open, so it survives).
- **Which flags suppress restore.** Chosen: `--auto` alone suppresses both
  restores and both writes, because it is the benchmark mode and its timings
  must not depend on a remembered window size or folder, nor leave a
  benchmark folder behind as `lastFolder`. `--settings`, `--index` and
  `--log` do not suppress anything: they only redirect storage, and the hand
  check of this feature runs under `--settings`.
- **Is a command-line folder remembered?** Chosen: yes, every opened folder,
  including one given on the command line, as the Tauri app remembers every
  open.
- **Fullscreen.** Not restored: a fullscreen window is stored by its restore
  bounds and reopens windowed. The native app has no fullscreen toggle of its
  own; restoring it is a one-line mapping to `WindowBounds::Fullscreen` if
  wanted later.
- **Clamping changes a window that was partly off-screen.** Chosen: `place`
  moves the saved bounds fully into the chosen display's visible area. A
  window deliberately left hanging off an edge comes back fully on-screen. The
  alternative (keep the bounds as saved whenever their center is on the
  display, the exact Windows check) preserves it but leaves two rules, ours
  and GPUI's, that must agree per platform.
- **Mixed-DPI coordinates.** On Windows the saved bounds are logical pixels
  at their display's scale. When the saved display is gone, comparing them
  with another display's bounds is approximate across different scale
  factors; the result is still on-screen because of the clamp, but may not be
  the display a user would expect.
- **Wayland.** A Wayland client cannot position its window, so only the size
  can come back there; nothing in this plan works around it.
- **The store diverges from the Tauri app's.** After the one-time seed the
  two apps remember their folders separately (the guide's existing rule);
  until the switch-over, the native app reopens its own last folder, not the
  Tauri app's latest.

## Progress
