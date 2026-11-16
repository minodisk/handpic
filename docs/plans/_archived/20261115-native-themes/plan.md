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

# Native app themes

## Purpose

The GPUI app (`crates/native`) is dark only: `theme.rs` forces
`ThemeMode::Dark` and overwrites gpui-component's colors with the shadcn/ui
Neutral dark tokens, and the app's own `div`s and canvases paint fixed
`*_COLOR` / `*_FILL` constants. After this plan the user picks a theme and a
mode (System / Light / Dark) in the settings modal, the choice is stored and
applied at startup and live, System follows the OS appearance while the app
runs, and a chosen theme recolors the panes, the strip, the status row, the
menus and the dialogs. With nothing stored the app looks exactly as it does
today. `theme.rs` also maps `title_bar` and `title_bar_border`, which the
gpui-component `TitleBar` a peer session adds right after Step 1 reads.

Read [native-app.md](../../../agents/native-app.md) first, in particular
"gpui-component is initialized" and "`Root` wraps the window's content".

What the vendored gpui-component 0.9.1 source says (read at planning time, in
`src/theme/{mod,registry,schema,theme_color}.rs` and `src/title_bar.rs`):

- Only `Default Light` and `Default Dark` are embedded in the crate
  (`src/theme/default-theme.json`, itself derived from shadcn's Neutral). The
  other themes (Ayu, Catppuccin, Tokyo Night, ...) live in the upstream
  repository's `themes/` folder, 21 JSON files of about 180 KB in all, and are
  not in the published crate. They have to be vendored and embedded.
- A theme file is a `ThemeSet` (`name`, `author`, `url`, `themes`), and each
  `ThemeConfig` in it has its own `name` and one fixed `mode`. A set can hold
  several themes of one mode: `catppuccin.json` holds Catppuccin Latte (light)
  and Frappe, Macchiato and Mocha (dark). `Theme` keeps a `light_theme` and a
  `dark_theme` config and `Theme::change(mode, ..)` loads the one for the mode.
- `ThemeSet` and `ThemeConfig` are public and `Deserialize`, so the app can
  parse a theme file itself with `serde_json` and hand a config to
  `Theme::apply_config(&Rc<ThemeConfig>)`, which installs it, derives every
  unnamed token (`button_*`, `*_hover`, `list_*`, `title_bar`, ...) and
  switches to its mode.
- `ThemeRegistry::load_themes_from_str` mutates the registry global, and
  gpui-component observes that global and answers with `Theme::change(mode)`,
  which reloads the registered config over any color edit (the Neutral
  mapping). `watch_dir` creates and watches a directory. Neither is needed.
- `Theme::update`'s reconcile step compares the colors and tokens with their
  values before the closure. Installing a config and re-mapping the same
  colors in one closure can therefore put the config's value back over a
  mapped field whose color did not change; today's order (switch first, map in
  a second `Theme::update`) does not have that problem.
- `Theme::update` does not re-derive `button_hover` and the like from edited
  colors (already recorded in `theme.rs`).
- Nothing in gpui-component observes the OS appearance.
  `Theme::sync_system_appearance(window, cx)` is a one-shot
  `Theme::change(window.appearance())`; GPUI offers
  `Window::observe_window_appearance` / `Context::observe_window_appearance`
  and `App::window_appearance()`. The component's comment prefers
  `window.appearance()` on Linux.
- `TitleBar` paints `cx.theme().title_bar` blended with `background`, and
  `title_bar_border`. `Default Dark` sets them to `#171717` and `#262626`.

## Steps

- [x] Step 1: Read the app-painted UI colors from the active theme
  - Done when:
    - The semantic UI colors painted by `main.rs`, `settingsview.rs`,
      `folderops.rs` and `modal.rs` come from the active theme (through
      `cx.theme()` or the app roles below), not from fixed constants; the
      fixed colors listed below stay constants, each with a comment saying why
    - `theme.rs` maps `title_bar` and `title_bar_border`, and its test asserts
      them
    - The app still starts dark and Neutral, and every color it paints has the
      value it has today: tests assert each mapped token and each app role
      against today's constant values
    - `CLAUDE.md`'s `src/theme.rs` lines and `native-app.md`'s "gpui-component
      is initialized" section describe the new source of the colors
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - No behavior change in this step: no setting, no mode, no new theme.
    - `theme.rs` becomes the one owner of the Neutral dark values; the
      semantic constants leave `main.rs`.
    - Constants that already equal a mapped token in Neutral dark, so the
      swap is exact: `PANE_COLOR` = `popover`, `MUTED_COLOR` =
      `muted_foreground`, `ERROR_COLOR` = `danger`, `MUTED_FILL` and
      `PLACEHOLDER` = `muted`, `PRIMARY_COLOR` and `CURRENT_OUTLINE` =
      `primary`, `FOREGROUND_COLOR` = `foreground`, `ACCENT_COLOR` = `accent`,
      `RING_COLOR` and `DROP_RING_COLOR` = `ring`, `BORDER_COLOR` = `border`,
      `INPUT_BORDER` = `input`, `OVERLAY_COLOR` = `overlay`, `BUTTON_FILL` =
      `button`.
    - Colors with no equal token: the root view's `#000000` background and
      `#ffffff` text, `NAME_COLOR` `#d4d4d4`, `FAILURE_COLOR` `#a3a3a3`,
      `BAND_COLOR` / `BAND_COLOR_DIMMED` (white at 8 % and 3.6 %),
      `REJECTED_DIM` (black at 55 %, "over the strip's black"). To keep today's
      pixels these need app-level roles next to the theme (for example a small
      struct in a GPUI global that `theme.rs` sets together with the theme):
      today's values for Neutral dark, and, from Step 2 on, values derived
      from the theme's tokens otherwise (the band from `foreground` with
      alpha, the dim from the strip's background with alpha). Keep the
      derivation a pure function of `ThemeColor` and of whether the theme is
      Neutral dark, so it is testable without a GPUI context (the crate has no
      `gpui` test support). See the trade-off on strict pixel identity.
    - `title_bar` = the pane color `#171717`; `title_bar_border` = the border
      color (white at 10 %), like the panes' borders.
    - Colors that stay fixed in every theme, and why:
      - `mark.rs`'s `MarkState` colors, `overlay.rs`'s `FACE_COLOR`,
        `MESH_COLOR` and the label halos, `peaking.rs`'s `PEAKING_COLOR`:
        drawn over the photo, not over a themed surface, and ports of the
        frozen Tauri values, some asserted by tests.
      - `perfgraph.rs`'s `CPU` / `DISK` / `LATENCY` series colors and its
        band's `BACKGROUND` / `BORDER` / `MUTED`: a band over the photo whose
        series colors are chosen against that dark band.
      - `STARS_COLOR`, `PICK_COLOR`, `REJECT_COLOR` (strip, filter menu,
        `folderops.rs`): the judgment's meaning.
      - `tint_color`'s label tints with `LABELED_NAME_COLOR` (white on the
        tint): the Lightroom label colors, a fixed pair.
      - The preview canvas's black: see the trade-off.
    - The strip is a `canvas`; its paint closure receives `cx: &mut App`, so
      `paint_strip` and its helpers (`shape(color, window)`, which takes a
      `u32` today) can read the theme there. Other canvases follow the same
      pattern.
    - `settingsview.rs`, `main.rs` (menus, around lines 4640-4820) and
      `modal.rs` already read `cx.theme()` in places; match that style.
    - The peer session's `TitleBar` work also edits `main.rs`; land this step
      first and keep the diff to color reads.

- [x] Step 2: Store the theme setting, add Light and System, and an `Appearance` tab
  - Done when:
    - The settings store has a new key (`theme`) read by a pure rule: a
      missing or invalid value is the default (Neutral, mode Dark), each field
      falling back on its own
    - The setting is applied at startup before the window is drawn and live
      when changed in the modal; in System mode the theme follows the OS
      appearance, also when the OS switches while the app runs
    - Neutral has a light variant from shadcn/ui's Neutral light tokens, with
      a mapping test like the dark one
    - The settings modal has an `Appearance` tab with the System / Light /
      Dark choice, written through `Writes`
    - With no `theme` key the app is unchanged from Step 1
    - Tests cover the setting rule (missing, invalid, each mode, round trip),
      the mode resolution (System + OS appearance to light / dark) and the
      light mapping; `settingsmodal.rs`'s tab tests cover the new tab
    - `CLAUDE.md` (`src/theme.rs`, `src/settings.rs`, `src/settingsmodal.rs` /
      `src/settingsview.rs` lines) and `native-app.md` (the theme section's
      "a light OS setting leaves the app dark", the settings modal section)
      are updated
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged: every semantic color already follows the
      theme, so this step only changes which theme is installed.
    - Stored shape (Option A, decided): a JSON object
      `{"mode": "system" | "light" | "dark"}` under `theme`; Step 3 adds the
      `light` and `dark` theme names to the same object, so write the rule per
      field from the start. Follow the existing rules' style
      (`SortKey::from_setting`, `Panels::from_setting` / `setting`), add the
      field to `Settings` and `from_values`.
    - The default mode is Dark, not System: with System a machine on a light
      OS appearance would change its look with nothing stored (see the
      trade-off).
    - Neutral dark stays `Default Dark` plus `map`, in two `Theme::update` /
      `Theme::change` calls in today's order. Neutral light: `Default Light`
      already derives from shadcn's Neutral, so measure its resolved
      `ThemeColor` against the shadcn Neutral light tokens (background
      `#ffffff`, foreground `#0a0a0a`, popover `#ffffff`, primary `#171717` on
      `#fafafa`, secondary / muted / accent `#f5f5f5`, muted foreground
      `#737373`, border and input `#e5e5e5`, ring `#a1a1a1`; check them
      against shadcn's source) and map only what differs or what the dark map
      pins explicitly (the `button_*` fields, `title_bar*`).
    - Keep `font_size` 16 px, `radius` and `radius_lg` forced after every
      theme install: the app's rem-based sizes assume them (`Root` sets the
      rem from `theme.font_size` every frame).
    - `apply` grows from `apply(cx)` to taking the setting and the OS
      appearance. At startup the window does not exist yet:
      `cx.window_appearance()` first, then `window.appearance()` and an
      appearance observer registered where the view already registers
      `observe_window_bounds` / `observe_window_activation` (`main.rs` around
      line 587). The observer re-applies only in System mode.
    - The modal applies the change directly (the theme is a global) and writes
      the key; `Theme::update` / `change` already refresh every window.
    - Add `Tab::Appearance` to `TABS`; the tab tests in `settingsmodal.rs`
      index the tabs by position (`next_tab` wrap-around), so they change with
      the order. Draw the mode as `Radio`s like the Sidecar tab's formats.
    - Whether `--auto` runs read and write the key: follow what `panels` and
      the other view settings do.
    - Verify in light mode by eye: the strip's stars (`#ffd050`) and the pick
      green on a light surface, the tree cursor, the menus' hover, the
      progress bar. Record what is hard to read in `learnings.md`; do not
      retune the fixed colors in this step unless something is unreadable.

- [x] Step 3: Ship gpui-component's themes and add the theme picker
  - Done when:
    - The upstream theme files are vendored under `crates/native`, embedded in
      the binary and offered in the `Appearance` tab next to Neutral, as a
      light-theme choice and a dark-theme choice
    - The choice is stored in the `theme` key, applied at startup and live; an
      unknown or wrong-mode name falls back to Neutral
    - A chosen theme recolors the panes, the strip, the status row, the menus
      and the dialogs in both modes
    - Tests: every embedded file parses as a `ThemeSet` with 0.9.1's schema,
      the theme names are unique and none is `Neutral`, and the setting rule
      covers the names (unknown, wrong mode, round trip)
    - The vendored files carry their license notice and their source
      (repository, commit)
    - `CLAUDE.md` and `native-app.md` are updated (the theme list's location,
      how a theme is installed, the license section)
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 and 2 are merged.
    - Stored shape: `{"mode", "light": <name>, "dark": <name>}`, each name
      defaulting to Neutral (Option A, decided by the user on 2026-11-15).
    - Vendor from `https://github.com/longbridge/gpui-kit` `themes/` at the
      commit of the 0.9.1 release, so the files match the schema of the pinned
      crate; record the commit. A `gpui-component` bump re-vendors them (add
      that to `native-app.md`'s upgrade rule).
    - Parse the embedded JSON in the app (`serde_json::from_str::<ThemeSet>`,
      `include_str!`) and keep the configs in the app; install one with
      `Theme::apply_config`. Do not go through
      `ThemeRegistry::load_themes_from_str` or `watch_dir`: mutating the
      registry triggers gpui-component's observer, which reloads the
      registered theme over the Neutral mapping. Check whether `serde_json`
      is already a direct dependency of the native crate (it is used in
      `settings.rs`).
    - Installing a non-Neutral theme must not run `map`; switching back to
      Neutral must restore it exactly (install `Default Dark` / `Default
      Light` first, map in a second `Theme::update`, as today). Mind the
      reconcile behavior described under Purpose when installing and editing
      in one closure.
    - The app roles from Step 1 take their derived values for a non-Neutral
      theme; this is the step where those derivations become visible, so
      check each on at least one light and one dark shipped theme.
    - The vendored JSON is seen by `vp check`'s formatter (`json` is one of
      its extensions): add the folder to `vite.config.ts`'s `fmt`
      `ignorePatterns`, as `crates/core/genres/**` is, rather than
      reformatting upstream files.
    - License: the upstream repository is Apache-2.0 and each theme file
      names its own `author` / `url`; `crates/native/LICENSE-lucide` is the
      precedent for a notice next to vendored assets. Check each file's
      origin while vendoring and say in the PR what was found.
    - The list is long (21 files, several themes each). The step decides
      between a scrolling list of rows like the Keyboard Shortcuts tab and
      gpui-component's `Select`. `Select` brings its own key bindings alive
      (`native-app.md`'s binding table lists its context as inert until
      drawn), so choosing it means re-checking Escape / Enter / arrows inside
      the modal and updating that table.

## Trade-offs and risks

- **The picker's shape (decided: Option A, by the user on 2026-11-15).** A
  gpui-component theme has one fixed mode, and a set can hold several themes
  of one mode, so "one theme + a mode" does not map onto the data.
  - Option A, chosen: store `{"mode", "light": <name>, "dark": <name>}` and
    show the mode plus a light-theme list and a dark-theme list, each
    defaulting to Neutral. It matches `Theme`'s own `light_theme` /
    `dark_theme`, System mode works with any pair, and every shipped theme is
    reachable. Cost: two lists instead of one.
  - Option B: store one family name (the file's `name`, e.g. `Catppuccin`)
    plus the mode, and take the family's first theme of the resolved mode,
    falling back to Neutral when it has none. One list, closest to the
    request's wording. Cost: Frappe / Macchiato and the like become
    unreachable, and a dark-only family silently shows Neutral in light mode.
  - Option C: store one theme name and derive the mode from it, with System
    only for Neutral. Simplest, but System stops working for the other themes.
- **Strict pixel identity versus tokens only (Step 1).** A few of today's
  colors equal no token: the root's `#000000` / `#ffffff`, the strip's name
  `#d4d4d4` and failure `#a3a3a3`, the burst band, the reject dim. The steps
  keep them exact through a small set of app roles beside the theme. The
  alternative is tokens only (`background` `#0a0a0a`, `foreground` `#fafafa`,
  ...): less code and no extra global, but Neutral dark shifts by a few levels
  in those places, which breaks "pixel-identical when nothing is stored".
- **The preview area's background.** Assumed: it stays black in every theme
  (a fixed surround for judging photos; the AF mark, the face boxes, peaking
  and the performance band are designed against the image), while the strip
  follows the theme. Alternative: the preview follows the theme's
  `background` too, which is more uniform in a light theme but puts a bright
  surround around the photo and a dark performance band on a light area.
- **The default mode.** Assumed Dark, so nothing changes on any machine until
  the user chooses. System as the default would follow the OS from the first
  launch but turns the app light on a light-appearance OS with nothing stored.
- **How many themes to ship.** Assumed all 21 upstream files (the request says
  "the built-in themes"), about 180 KB embedded. A subset is less to vendor
  and to re-vendor at each bump, but someone has to choose it. Whether
  `Default Light` / `Default Dark` appear in the lists next to Neutral is open
  too: they are near-duplicates of Neutral, so the steps leave them out.
- **Fixed judgment colors on light themes.** The stars' yellow and the pick
  green were chosen on black. They stay fixed for their meaning; on a light
  strip they may be hard to read. Step 2 checks and records it; retuning them
  per mode would be a follow-up, not part of this plan.
- **Theme files set their own `font.size` and radius.** The app keeps forcing
  16 px and its radii, since its layout assumes them, so a shipped theme will
  not look exactly as in gpui-component's gallery.
- **Conflict with the peer `TitleBar` session.** Both touch `main.rs` and
  `theme.rs`. Step 1 carries the `title_bar` tokens so the peer can start
  after it; Steps 2 and 3 then need the title bar checked in light mode and
  with a shipped theme.
- **Local gate.** The request names `mise run ci`; per the project's agent
  rules the steps gate on `mise run ci:local` plus the PR's GitHub checks
  (the `native` job runs the native crate's lint and tests on three
  platforms).
- **User docs.** `docs/humans/usage.md` describes the Tauri app only, so no
  human-facing doc changes are planned; if the GPUI app gets a usage page
  before this lands, the `Appearance` tab belongs there (with its `.ja.md`).

## Progress

- (2026-11-16) Step 1 complete
- (2026-11-16) Step 2 complete
- (2026-11-16) Step 3 complete
