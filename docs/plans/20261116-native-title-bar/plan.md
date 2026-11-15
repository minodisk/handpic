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

# Native Title Bar

## Purpose

The GPUI app (`crates/native`) opens its window under the OS title bar, a
system strip above an otherwise app-drawn window. Replacing it with
gpui-component 0.9.1's `TitleBar` (already a dependency) makes the window one
surface in the app's theme and gives the app a bar of its own to grow later.
In this plan the bar shows only the app icon and the title "Handpic": no
Sort / Filter, no folder name.

The switchable native theme work (`feature/native-themes`, plan PR #1086)
will map gpui-component's `title_bar` and `title_bar_border` theme tokens.
This plan does not touch `crates/native/src/theme.rs` and does not wait for
that work (the user's choice): until it lands, the bar paints with the values
gpui-component derives at theme load (`title_bar` from `background`,
`title_bar_border` from `border`).

## Steps

- [x] Step 1: Draw gpui-component's `TitleBar` with the app icon and "Handpic"
      in place of the OS title bar
  - Done when:
    - The PR's diff leaves `crates/native/src/theme.rs` untouched.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass (the
      `native` job on its three platforms included).
    - `icons.rs` has a test that the app icon's path loads PNG bytes through
      `Icons`.
    - On Windows, checked by running the built app: the window shows a 34 px
      bar with the app icon and "Handpic"; minimize,
      maximize / restore and close work; dragging the bar moves the window;
      the taskbar and Alt+Tab still read "Handpic".
    - The preview, the `f` overlay, the `d` performance band, the `z` 1:1
      crop, the drop outline, the tree, the meta pane, the strip and the
      status row lay out below the bar with nothing clipped, and an open
      `Sort ▾` / `Filter ▾` / right-click menu does not grow over the bar.
    - `docs/agents/native-app.md` has a section on the title bar, and
      `CLAUDE.md`'s layout prose names it briefly where it describes the
      window and `icons.rs`.
    - By hand, recorded as pending in this plan and in the PR description
      until someone does them (they do not block the merge):
      - Windows, gpui-pre 0.3.12: the resize borders on all four edges and
        corners (the top edge over the bar included), Snap Layouts on hover
        over the maximize button, double-click on the bar to maximize /
        restore, and the bar under a maximized window.
      - Windows: minimize / maximize / close and drag while the trash dialog,
        a folder operation's dialog or the settings modal is open (see
        Trade-offs).
      - Windows: a `windowBounds` saved before this change restores without
        the window drifting from launch to launch.
      - Once the theme work maps `title_bar` / `title_bar_border`, the bar
        takes its colors and follows a theme switch.
      - macOS: the traffic lights sit inside the bar without overlapping the
        icon and title (80 px left padding), the content starts below the
        bar, and double-click follows the system setting.
      - Linux: under client-side decorations the bar draws its controls and
        `Root`'s frame and shadow; under server-side decorations (X11 without
        a compositor, or a Wayland compositor that insists) no second set of
        controls appears. A first pass can run under WSLg on the Windows host
        once a WSL distribution is installed (none is today); WSLg's
        Weston-based compositor may treat decorations differently from GNOME
        or KDE, so it does not replace a check on a regular Linux desktop.
  - Implementation approach (as far as it is known; omit if unknown):
    - Touch `crates/native` and docs only; the Tauri app is frozen. Do not
      edit `crates/native/src/theme.rs`: `TitleBar` reads `theme.title_bar`
      (a gradient from it mixed 55 / 45 with `theme.background` down to
      `theme.title_bar`) and `theme.title_bar_border` (its 1 px bottom
      border), and the theme branch owns both.
    - Line numbers below were read on 2026-11-15 and may have moved (the
      settings layer and the StatusBar PRs merged since); re-read the code
      first.
    - A parallel branch, `refactor/native-gpui-kit`, moves `crates/native` onto
      the `gpui-kit` umbrella crate (imports through `gpui_kit::*`, start
      through `gpui_kit::init` / `gpui_kit::open_window`). If it has merged
      into `main` when this step starts or when its PR conflicts, follow it:
      take `TitleBar` from `gpui_kit::component` and pass the window options
      below to `gpui_kit::open_window` instead of `cx.open_window`.
    - Window options (`crates/native/src/main.rs`, `cx.open_window`, near
      line 5384 on 2026-11-15): spread `..TitleBar::window_options()`
      instead of `..Default::default()` (it sets `app_owns_titlebar_drag:
      true`), and keep an explicit `titlebar: Some(TitlebarOptions { title:
      Some("Handpic".into()), ..TitleBar::title_bar_options() })`.
      `title_bar_options()` has `title: None`, which would empty the taskbar
      and Alt+Tab text. `window_bounds`, `display_id` and `app_id` stay as
      they are.
    - Linux decorations (decided by the user: client-side): gpui-pre 0.3.12
      requests `window_decorations.unwrap_or(WindowDecorations::Server)`, so
      the default keeps the window manager's bar and puts ours, without
      controls, under it. Set `window_decorations:
      Some(WindowDecorations::Client)` so the bar replaces it; `Root`'s
      `window_border()` already draws the frame and shadow then.
    - The bar: `TitleBar::new().child(..)` as the first child of the root
      `div` in `impl Render for Riffle` (the `size_full().flex().flex_col()`
      one), before the main area. It is a `flex_shrink_0` element of
      `TITLE_BAR_HEIGHT` (34 px) in flow on every platform, so the `flex_1`
      main area shrinks by itself. Its child is an `h_flex` with a small gap
      holding the icon and the text "Handpic". Take the text color from the
      theme (`cx.theme().foreground`), not from a `main.rs` constant, so the
      bar follows the switchable theme; the root `div` sets pure white, which
      the bar's text must not inherit.
    - The icon: serve `crates/app/icons/64x64.png` through `Icons` in
      `crates/native/src/icons.rs` (`include_bytes!("../../app/icons/64x64.png")`,
      referenced by path as `build.rs` references `icon.ico`, under a new
      path constant such as `icons/app.png`), in both `load` and `list`, and
      draw it with `img(icons::APP)` at about 16 px. gpui's `img` treats a
      non-URI string as an embedded resource, loads it from the app's
      `AssetSource` and sniffs the format, and its `image` dependency has
      `png` on, so no dependency is added. `SVGS` is a table of `&str`; the
      PNG is bytes, so it is one more arm rather than a table entry. The
      icon is the app's own, so `LICENSE-lucide` and the header comment are
      unchanged apart from the module doc naming the PNG.
    - Menu caps: `status_row` and `context_popover` are given
      `window.viewport_size().height - px(40.)` as their menus' maximum
      height (lines 4512-4513 on 2026-11-15). Subtract `TITLE_BAR_HEIGHT`
      as well so a long menu stops under the bar.
    - Verified while planning that the rest follows the bar without a
      change, so confirm on the built app rather than editing: the preview
      canvas takes its size from its own bounds (`viewport.set`, which the
      1:1 crop reads), `perfgraph::paint`, `overlay::paint` and
      `fit_centered` take those bounds, `drop_outline` is `absolute`
      `inset_0` inside the main area, the tree is a `uniform_list`, the strip
      hit-tests on x only, the context menus are `anchored` at window
      coordinates, and the settings dialog centers on the viewport.
    - What to check during implementation: that the `--auto` benchmark's
      `page` lines still print (the bar adds an element, not a paint path),
      and that a press on the bar does not take the keyboard from the view
      (the bar has no focus handle).
    - Docs: add a section to `docs/agents/native-app.md` in its existing
      form (heading with `(Measured)` / `(Inferred)`, a `Source:` link to
      this plan, `- Rule:` lines) covering the window options and the kept
      title, the icon's route, the Linux decorations choice, that the bar's
      colors are the theme's `title_bar` / `title_bar_border`, and what
      stayed unmeasured. In `CLAUDE.md` add a few words where the layout
      prose describes the window (`app_id` `IDENTIFIER`) and `icons.rs`;
      nothing else.

## Trade-offs and risks

- Theme tokens left to the theme work. The bar's fill and border come only
  from the theme's `title_bar` and `title_bar_border`, which this plan does
  not map. Until `feature/native-themes` lands, they keep gpui-component's
  load-time values (`background` / `border`), so the bar reads as a flat
  strip with a border rather than the app's chosen bar color. Accepted by
  the user to avoid waiting; a conflict with that branch in `main.rs` is
  resolved on whichever PR lands second.
- Linux decorations. Taken (the user's choice): request
  `WindowDecorations::Client`; the bar replaces the window manager's and
  draws the controls, and `Root`'s `window_border()` draws the frame, shadow
  and resize edges. Nobody has run the app that way on this project. Not
  taken: the default (server-side), one line smaller but a second,
  control-less bar under the system one on Linux.
- Icon route. Taken: one PNG through `Icons` and `img()`, stateless and
  using the mechanism the app already has. Not taken: an `Arc<gpui::Image>`
  built from the bytes and held in the view (a new field for one image), or
  an SVG redraw of the icon (no SVG source exists under `crates/app/icons`).
- Window controls under a dialog. The bar is inside the view, so the view's
  `modal::backdrop` and `Root`'s dialog layer cover it. On Windows the
  controls are hit-tested through `WindowControlArea` hitboxes, so a
  backdrop may make minimize / maximize / close and drag dead while a dialog
  is open, where the OS bar always worked. It is a by-hand check; if it
  fails, the fix (mounting the bar outside the dialog layer, or leaving the
  bar out of the backdrop) is a follow-up, not part of this step.
- Saved geometry. `windowBounds` saved under the OS bar describe a window
  with a system caption. With `appears_transparent` the client area covers
  the caption, so the first restore may sit differently by the caption's
  height. The risk worth checking is a drift on every launch, not the
  one-time offset.
- gpui-pre 0.3.12 on Windows. Resize borders, Snap Layouts and double-click
  come from GPUI's non-client handling, which this project has not exercised
  with a transparent title bar; they are by-hand checks for that reason.
- One step. The template aims for 3-7, but the options, the bar and the icon
  are not reviewable apart: any subset leaves a window with no bar or a bar
  with nothing in it.

## Progress
