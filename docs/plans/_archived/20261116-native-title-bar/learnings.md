# Learnings: native-title-bar

## Step 1

- `refactor/native-gpui-kit` had not merged into `main` when the step
  started, so `TitleBar` comes from `gpui_component` and the options go to
  `cx.open_window`.
- `TitleBar`, `TITLE_BAR_HEIGHT` and `h_flex` are re-exported at the
  `gpui_component` crate root (`pub use title_bar::*`).
- The press on the bar still reaches the root `div`'s `track_focus`, so a
  click on the bar keeps the keyboard with the view (the bar itself has no
  focus handle).
- Writing a Rust byte-string escape such as `b"\x89PNG\r\n"` through a
  heredoc'd Python script in the Bash tool turned the escapes into raw bytes
  (a non-ASCII byte string literal error); build the backslashes with
  `chr(92)` or use the Edit tool instead.
- Seen in a debug build on Windows 11 (`--index` / `--settings` / `--log`
  in a scratch folder, no folder open): the window opened with a 34 px bar
  holding the app icon and "Handpic" at the left and minimize / maximize /
  close at the right; the process's main window title read "Handpic"; the
  tree, the empty-preview message and the status row (`Sort ▾` / `Filter ▾`)
  laid out below the bar. The bar's fill matched the default theme values
  (no `title_bar` mapping yet). Not exercised by hand: the controls' clicks,
  drag, the taskbar / Alt+Tab text, a folder's preview, the `f` / `d` / `z`
  overlays, the drop outline and the menus' cap.

## Deferred issues (todo candidates)

- Pending manual check (Windows, built app): click minimize, maximize /
  restore and close; drag the bar to move the window; read the taskbar and
  Alt+Tab text ("Handpic" expected). Open a folder and check the preview, the
  `f` overlay, the `d` performance band, the `z` 1:1 crop, the drop outline,
  the tree, the meta pane, the strip and the status row lay out below the bar
  with nothing clipped, and that an open `Sort ▾` / `Filter ▾` / right-click
  menu stops under the bar. The step's checkbox was ticked on the automated
  criteria and a launch screenshot only. Basis: plan Step 1 Done-when;
  files `crates/native/src/main.rs`.
- Pending manual check (Windows, gpui-pre 0.3.12): resize borders on all four
  edges and corners (the top edge over the bar included), Snap Layouts on
  hover over maximize, double-click on the bar to maximize / restore, the bar
  under a maximized window. Basis: plan Step 1 Done-when (by hand);
  `crates/native/src/main.rs`.
- Pending manual check (Windows): minimize / maximize / close and drag while
  the trash dialog, a folder operation's dialog or the settings modal is open
  (the backdrop may cover the `WindowControlArea` hitboxes; the fix would be a
  follow-up). Basis: plan Trade-offs "Window controls under a dialog";
  `crates/native/src/main.rs`, `crates/native/src/modal.rs`.
- Pending manual check (Windows): a `windowBounds` saved before this change
  restores without the window drifting from launch to launch. Basis: plan
  Trade-offs "Saved geometry"; `crates/native/src/session.rs`.
- Pending manual check (any platform, after the theme work lands): the bar
  takes the mapped `title_bar` / `title_bar_border` colors and follows a theme
  switch. Basis: plan Purpose; `crates/native/src/theme.rs`.
- Pending manual check (macOS): the traffic lights sit inside the bar without
  overlapping the icon and title (80 px left padding), the content starts
  below the bar, and double-click follows the system setting. Basis: plan
  Step 1 Done-when; `crates/native/src/main.rs`.
- Pending manual check (Linux): under client-side decorations the bar draws
  its controls and `Root`'s frame and shadow; under server-side decorations
  (X11 without a compositor, or a Wayland compositor that insists) no second
  set of controls appears. WSLg is a possible first pass, not a substitute
  for GNOME / KDE. Basis: plan Step 1 Done-when; `crates/native/src/main.rs`.
