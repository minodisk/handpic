# Working on `crates/app`

Read this before touching the GPUI app in `crates/app`. It lists what the
port has met so far, each with the reason.

The tags follow [`shared-crates.md`](./shared-crates.md): **Hit** broke something here,
**Measured** steered a design decision, **Inferred** comes from sources or docs
only.

Source: [the gpui-port plan](../plans/20261109-gpui-port/plan.md), its spike
and Step 1's [learnings](../plans/20261109-gpui-port/learnings.md).

## Toolchain

### GPUI is the `gpui-pre` snapshot `gpui-kit` pins (Inferred)

The app crate depends on one GUI crate, the `gpui-kit` umbrella crate at
`=0.9.1` (default features `component` and `assets`), as the GPUI Kit guides
(<https://gpui-kit.com/docs/installation/>) have it. `gpui-kit` 0.9.1 pins
`gpui-pre` and `gpui-pre-platform` at `=0.3.12`, a snapshot of Zed's GPUI at
`zed@d2a4fb6` published by the gpui-kit maintainer, and depends on
`gpui-base`, `gpui-component` and `gpui-kit-assets` 0.9.1 by caret
requirements `Cargo.lock` holds. Its re-exports, read from 0.9.1's
`src/lib.rs`: `gpui_kit::*` is all of GPUI (`pub use ::gpui::*`),
`gpui_kit::platform` is `gpui_platform`, `gpui_kit::application` its
`application`, `gpui_kit::component` is `gpui_component`,
`gpui_kit::assets` is `gpui_kit_assets` and `gpui_kit::base` is
`gpui_base`. The sources import through those paths with explicit lists
(`use gpui_kit::{..}`, `use gpui_kit::component::{..}`), never a
`use gpui_kit::*` glob; no crate is named `gpui` any more. `gpui_kit::actions!`
derives through `$crate::Action`, so the test module's `actions!` needs no
`gpui` alias. Zed pins Rust 1.98.1 at that commit, and so does
`crates/app/mise.toml`; the published crates declare no `rust-version`.
The plans: [gpui-component-dep](../plans/_archived/20261114-gpui-component-dep/plan.md),
then [native-gpui-kit](../plans/_archived/20261116-native-gpui-kit/plan.md).

`crates/app/Cargo.toml` carries the guide's `[profile.dev.package]` block
(`opt-level = 3` for `gpui-pre`, `gpui-component`, `gpui-kit`,
`gpui-kit-assets`, `gpui-pre-platform`, `rustybuzz`, `taffy` and
`ttf-parser`), so a `cargo run` debug build renders at a usable speed at the
cost of a longer cold build. The block also holds the tract inference crates
(`tract-core`, `tract-data`, `tract-extra`, `tract-hir`, `tract-linalg`,
`tract-nnef`, `tract-onnx`, `tract-onnx-opl`, `ndarray` and
`matrixmultiply`): unoptimized, pass 2's YuNet and face mesh inference is
about ten times slower, which took a UI test past its 10 s wait on a 4-core
CI runner (see the UI test rules below).

- Rule: the app crate never picks a GPUI version of its own. An upgrade
  is a `gpui-kit` bump (one exact pin) in one deliberate chore PR, which
  checks in the lock diff that `gpui-pre` and gpui-component moved with it,
  and records the API changes met at the bump here. A `gpui-component` bump
  (through `gpui-kit`) also copies `crates/app/themes/` again from the
  new release's `themes/` folder and updates the commit in its `LICENSE`,
  so the files match the schema of the pinned crate.
- Rule: the `gpui-fast` feature stays off.
- Rule: `cargo tree` shows one GPUI: no package from
  `git+https://github.com/zed-industries/zed`, `gpui-pre` at one version. Two
  GPUIs have distinct types, so no gpui-component element could join the
  app's views, and `[patch]` cannot swap `gpui-pre` for Zed's `gpui` because
  the package names differ.

### Only two of Zed's patches are needed (Inferred)

`[patch.crates-io]` carries Zed's `async-task` and `async-process` forks, which
now apply to the crates.io `gpui-pre` crates' dependencies on them (Zed at
`d2a4fb6` carries the same two revs). They are the only git sources in
`Cargo.lock`. Zed's other patches (`cpal`, `notify`, `calloop`,
`khronos-egl`, ...) are not needed by `gpui-pre` / `gpui-pre-platform`.

- Rule: do not copy the others. These two stayed out of the root workspace
  while the Tauri app lived there (both crates were in its lock, so a
  workspace patch would have swapped them for Zed's forks in the shipped
  app); moving them belongs to the workspace merge, which re-checks the root
  lock's users of both crates first.

### gpui-component is initialized; the app's controls draw from it (Inferred)

`gpui-component` 0.9.1 (through `gpui-kit`) is compiled on all three platforms.
`gpui_kit::init(cx)`, which with the `component` feature is exactly
`gpui_component::init(cx)`, runs first in the `application().run` closure,
before the window opens, then `theme.rs`'s `apply` sets the global `Theme`
to the mode the stored `theme` setting resolves to and that mode's chosen
theme. It puts the two chosen configs in `Theme`'s `light_theme` /
`dark_theme` slots in one `Theme::update` and then calls
`Theme::change(mode, ..)`, which installs the mode's slot
(`apply_config`). Neutral's slot is the registry's built-in `Default Dark`
or `Default Light`, which a second `Theme::update` maps, since a mode
switch reloads the slot's config over color edits made in the same closure;
a shipped theme is installed as it is, unmapped. `Theme::apply_config`
writes `shadow` only when a config sets it (Hybrid, macOS Classic,
Mellifluous and Molokai do), so `apply` sets `theme.shadow = true` before
`change`; otherwise Neutral chosen after Molokai would keep the shadow off.
`highlight` is not reset, since the app draws no code view. The shipped themes are
18 of gpui-kit's 21 theme files at the 0.9.1 release (the three whose
license could not be determined are left out, see below), copied unchanged into
`crates/app/themes/` (with its `LICENSE`), embedded with `include_str!`
and parsed once into `ThemeConfig`s (`serde_json` over `ThemeSet`); the app
never registers them (`ThemeRegistry::load_themes_from_str` would make
gpui-component's registry observer reload the registered theme over the
Neutral mapping) nor watches a folder. Every theme keeps the app's
`font_size` and radii below. The second `Theme::update` (`finish`) also sets
`scrollbar_mode` to `ScrollbarMode::Always` on every `apply`:
`gpui_component::init` sets the OS's mode once (`Scrolling` or `Hover`,
`sync_scrollbar_appearance`), and neither `change` nor `apply_config` writes
it, since the theme files have no key for it. With no `theme` key the mode is Dark and both
themes Neutral. The mapping reads `theme.rs`'s own `DARK` or
`LIGHT` palette, the one source of each color (the overlay included, which
the three backdrops read). In dark:
`background` `#0a0a0a`, `foreground` and every
`*_foreground` on a dark surface `#fafafa`, `popover` `#171717`, `primary`
`#e5e5e5` on `primary_foreground` `#171717`, `secondary`, `muted` and
`accent` `#262626`, `muted_foreground` `#a1a1a1`, `danger` `#ff6467`,
`border` and `scrollbar_thumb` white at 10 %, `input` white at 15 %, `ring`
`#737373`, `overlay` `#00000099`, `font_size` 16 px (the rem `Root` sets, see
"`Root` wraps the window's content"), `radius`
6 px (`rounded_md`, the buttons and menus) and `radius_lg` 8 px
(`rounded_lg`, the dialog boxes), and `Button`'s default variant: `button`
white at 4 %, `button_hover` and `button_active` `#262626`,
`button_foreground` `#fafafa`, and the settings' `Sidebar`: `sidebar`
`#171717` (the box's `popover`, not `Default Dark`'s `#0a0a0a`),
`sidebar_accent` `#262626` (the selected page, as the old tab list lit it),
`sidebar_foreground` and `sidebar_accent_foreground` `#fafafa`,
`sidebar_border` white at 10 %, `sidebar_primary` `#e5e5e5` on
`sidebar_primary_foreground` `#171717`, and `TitleBar`'s `title_bar`
`#171717` and `title_bar_border` white at 10 %, as the panes'. In light, the
same fields from shadcn/ui's Neutral light tokens (`neutral.json`'s oklch
values as hex): `background`, `popover`, `sidebar`, `status_bar`, `title_bar`
and `button` `#ffffff`, `foreground` `#0a0a0a`, `primary` `#171717` on
`#fafafa`, `secondary`, `muted`, `accent` and the button hovers `#f5f5f5` with
`#171717` on them, `muted_foreground` `#737373`, `danger` `#e7000b`
(`#e91a23` on hover), `border`, `input` and `scrollbar_thumb` `#e5e5e5`,
`ring` `#a1a1a1`, and the same `overlay`. The registered theme derives the `button_*`,
`*_hover` / `*_active` and `list_*` fields at load and `Theme::update` does
not derive them again, so the fields it does not name keep `Default Dark`'s
or `Default Light`'s values. The mode is the setting's (`theme::resolve`):
Light or Dark forced, or System, the OS appearance, read with
`cx.window_appearance()` at startup, then, once the window exists, with
`window.appearance()` (which gpui-component prefers on Linux) and on every
`observe_window_appearance` callback, which re-applies only in System mode.
`Theme::sync_system_appearance` is not used, since it does not map. The
components drawn are the settings modal's: `setting::Settings` (with its
`Sidebar`, search `Input`, `h_resizable` split, `GroupBox`es and the page
list's scrollbar), `Input` (its text fields, see "Text is typed through
gpui-component's `Input`"), `Button`, `Switch` (auto-advance, through
`SettingField::switch`), `Radio`, `Select` (the label names' language and
the Appearance page's light and dark themes, with its menu's `List`), `Kbd` (the Keyboard Shortcuts page's keys) and
`InputGroup` (the read-only Destination root with `Choose…`) (see "The
settings modal is its own entity"), the strip header's `Sort ▾` / `Filter ▾`
`Button`s (see "The sort and filter menus are deferred popovers over the
window"), the right-click menu (`PopupMenu`, its shortcuts as `Kbd` text;
see "The right-click menu is gpui-component's `PopupMenu` at the
pointer"), the tree's inline
rename's `Input` (see "The tree's inline
rename is a text input in the row") and `Tooltip`, every tooltip of the app
(see "A strip cell's tooltips are transparent `div`s over the canvas"). The
folder tree is not the component's `Tree` (see "The folder tree is not
gpui-component's `Tree`"). The
window's content is wrapped in `Root` (see "`Root` wraps the window's content").
The app's own `div`s and canvases read the same colors: the semantic ones
from the theme (`cx.theme()`; a canvas's paint closure reads it from its
`cx: &mut App`) and the ones no token equals from `theme.rs`'s `Roles`, a
GPUI global `apply` sets with the theme (`theme::roles(cx)`): the window's
surface `#000000` and text `#ffffff` (over `Root`'s surface, behind the
strip and the status row), the strip's file name `#d4d4d4` and scan failure
`#a3a3a3`, the burst band (white at 8 %, dimmed 3.6 %), the reject dim
(black at 55 %) and the preview's surround behind the photo `#000000`.
`Roles::of` gives today's values for Neutral dark and
otherwise derives them from the tokens (the surface and text from
`background` / `foreground`, the band from `foreground` with alpha, the dim
from `background` with alpha, black instead on a light `background`
(`theme::is_light`: relative luminance above 0.179, where black and white
text contrast equally), the surround the least luminant, by relative
luminance, of the opaque `background`, `popover`, `title_bar`, `status_bar`
and `sidebar`), which Neutral light takes. The pane colors map onto tokens exactly: the
panes `popover`, muted text `muted_foreground`, errors `danger`, the
selected cell and the placeholder `muted`, the current cell's outline and
lit controls `primary`, a selected cell's outline and the drop outline
`ring`, the hover `accent`, the tree's rows `ListItem`'s list tokens (the
open row `list_active`, the hover `list_hover`, the keyboard cursor's
outline `selection`, which Neutral maps to `accent`, `accent` and `ring`
(the built-in base theme's are blue), a vendored theme giving its own;
`selection` also paints `Input`'s selected text, and `list_active` /
`list_hover` also light a `Select`'s list row (also a `ListItem`) in
`accent`, so a change to either token reaches every `Select`),
the menu
buttons `button` with an `input` border. Fixed in every theme, as constants
with their reason: the judgment colors (`STARS_COLOR`, `PICK_COLOR`,
`REJECT_COLOR`), the strip's label band (the label's `ColorName` at
`LABEL_BAND_SCALE`, a palette value no theme changes) with
`LABELED_NAME_COLOR`, what is drawn
over the photo (`mark.rs`, `overlay.rs`, `peaking.rs`, `perfgraph.rs`, the
burst badge's `BADGE_COLOR`, the strip marks' dark copies and the face
labels' halos). The strip's marks (stars, face focus icon, closed-eyes mark,
burst badge) take one black copy at `(+1, +1)` (`MARK_SHADOW`, next to
`MARK_SIZE`), not `overlay::HALO`'s eight offsets: over a bright photo area
the eight copies merge into a dark box behind a small glyph and fill its
inside, and over a dark area they do not show. The halo stays only for the
`f` overlay's face labels (`paint_label`). Do not move the stars to the halo
(`../plans/_archived/20261127-strip-marks-outline/learnings.md`).

None of its optional features (`tree-sitter*`, `syntect`, `speech`,
`inspector`, `decimal`, `gpui-fast`) is on. `gpui-kit` itself turns on
`gpui-pre-platform`'s `font-kit`, `x11`, `wayland` and `runtime_shaders`
unconditionally (`cargo tree -e features -i gpui-pre-platform` shows those
four and the default `multithreaded`, for the host and for the Linux and
macOS targets alike); none of them reaches `gpui-pre-windows`. Before the
switch to `gpui-kit` only the default `multithreaded` was on, so, read from
the manifests and sources and not run on those platforms (Inferred), the
macOS build had no `font-kit` (no text) and the Linux build neither the X11
nor the Wayland backend.

`init` puts the first bindings into GPUI's keymap. As verified at 0.9.1
(`gpui-component` and `gpui-base`), every one is scoped to a key context
except the inspector's, and none is a multi-keystroke sequence, so none adds
a pending-key delay. GPUI dispatches a matching binding's action before any
key-down listener, so a binding that matches takes the key from
`Handpic::key_down`:

| Context | Keys | Action |
| --- | --- | --- |
| none (debug builds, or the `inspector` feature) | `ctrl-shift-i` (`cmd-alt-i` on macOS) | `ToggleInspector`, a global `cx.on_action` that toggles GPUI's inspector |
| `Root` | `tab`, `shift-tab`, `ctrl-c` (`cmd-c` on macOS) | `Tab` / `TabPrev`, which move the focus (inside a dialog's focus trap) and do not propagate, masked by `NoAction` in the app view's `Handpic` context; `Copy`, which propagates when no text is selected |
| `Input` | the editing keys, `enter` / `escape`, `tab` / `shift-tab`, the arrows, Home / End, word and line movement, undo / redo, the clipboard keys | the input's own; `PressEnter`, `Escape` and the indents propagate on a single-line input |
| `Dialog` (only with `.keyboard(true)`) | `escape`, `enter` | `dialog::Cancel`, `dialog::Confirm` |
| `Select` (the settings' language and theme pickers, while focused) | `space` (`Select && !Input`), `enter`, `ctrl-enter` (`cmd-enter` on macOS), `up`, `down`, `escape` | `Confirm` (opens the menu, or picks the highlighted row), `SelectUp` / `SelectDown`, `Cancel`, which closes an open menu and stops there, and propagates while it is closed |
| `List` (an open picker's menu, which takes the focus) | `escape`, `enter`, `ctrl-enter` (`cmd-enter` on macOS), `up`, `down` | the list's own; its `Cancel` propagates to the `Select`'s, which closes the menu only |
| `PopupMenu` (an open right-click menu, which takes the focus) | `enter`, `escape`, `up`, `down`, `left`, `right` | `Confirm` (runs the highlighted item, then dismisses), `Cancel` (dismisses), `SelectUp` / `SelectDown`; `SelectLeft` / `SelectRight` propagate in a menu without submenus, so they reach `Handpic::key_down` |
| `Popover`, `Tree`, `TextView`, ... | their own | inert until such an element is drawn |

- Rule: the inspector binding is left in place. In a debug build
  `ctrl+shift+i` opens the inspector and a user binding on it is shadowed;
  a release build has no such binding. No default of
  `crates/engine/src/shortcuts.rs` and none of `settingsmodal::APP_KEYS`
  uses it.
- Rule: `Root`'s `tab` / `shift-tab` are masked inside the app's view, so
  the keymap keeps them (see "`Root` wraps the window's content").
- Rule: a version bump of `gpui-component` re-reads the bindings: grep
  `KeyBinding::new` in both crates for a `None` context and for the `Root`
  context, and update the table.
- Rule: the app already has its own `AssetSource` (`icons.rs`); GPUI takes
  one per application, so `Icons` serves its own derivatives, then a few
  catalog icons selected with `gpui_kit::assets::icon_assets!`, and falls
  back to `gpui_kit::assets::Assets` (`gpui-kit-assets` 0.9.1, through `gpui-kit`, the
  components' own icon set, not the whole-catalog `AllAssets`) for `star`,
  the tree's `folder` and `folder-open` and the icons the
  components draw, the checked `Radio`'s and
  the language menu's `icons/check.svg` and the `Select`'s
  `icons/chevron-down.svg` so far (see "The icons are Lucide SVGs
  served by one `AssetSource`").

### `Root` wraps the window's content (Measured)

`main` opens the window with `gpui_kit::open_window`, whose build closure
returns the app's `Handpic` entity (it takes the focus and starts as before);
`open_window` wraps it in `gpui_base::Root` (the type `gpui_component::Root`
re-exports) as the window's root view and hands back the window and the
view. The `WindowOptions` pass through untouched. Nothing reads the window's root by its type:
`on_app_quit`, the bounds saved on quit and the tree's keyboard flag hold
the view, its focus handle or the `Window`, so they are unchanged. The trash
dialog and the folder operations' dialog are component dialogs (see "The
trash dialog is a component `Dialog`"); the settings are a layer under the
title bar, covering the rest of the window, that the view draws beside its
own root `div` (see "The settings modal is its own entity").

What `Root` adds to every frame, read at 0.9.1: its own `div` (the `Root` key
context and the `Tab` / `TabPrev` / `Copy` handlers), gpui-base's
`TextSelectionLayer` (a zero-size element that paints nothing while no
selectable text is drawn), the component layer's `WindowState` view (an
`absolute` `inset_0` `div` holding the empty sheet, dialog and notification
layers and the tooltip, touch-selection and native-menu fallback overlays)
and `window_border()`, a plain `size_full` `div` unless the window has
client-side decorations (Linux, where it draws a frame and a shadow).
`WindowState`'s `prepare` sets the window's rem to `theme.font_size` on every
frame, so `theme.rs` sets it to 16 px, GPUI's default, the rem the app's
rem-based sizes were drawn at (the menus' rows, the components' `small()`
sizes, the empty state's `0.9rem`); 13 px would have shrunk them all. Its
`style` gives `Root`'s surface `theme.background`, `theme.foreground` and
`.SystemUIFont`, GPUI's default font, under the app view's own black.

- `tab` / `shift-tab`: the app view's root `div` sets the key context
  `Handpic` (`KEY_CONTEXT`), and `main` binds both keys to `NoAction` in it
  (`keymap_keys`) right after `gpui_kit::init`. GPUI's keymap
  (`keymap.rs`, `resolve_bindings_for_input`) sorts the matching bindings by
  context depth and drops every binding ranked after a `NoAction` from a
  source of equal or weaker precedence; `Handpic` is deeper than `Root`, so
  with the focus anywhere in the app's view no action is dispatched and the
  key reaches `key_down` as before. Nothing is stopped, so no typed character
  is lost. A component dialog is drawn in `Root`'s layer, a sibling of the
  app's view, and the settings layer is a sibling of the view's `Handpic`
  `div`, so their context stacks have `Root` and no `Handpic`, and `Root`'s
  `Tab` / `TabPrev` walk their focus traps. An `Input` binds the two keys deeper
  still (`IndentInline` / `OutdentInline`), which runs first and propagates
  on a single-line input, so in the settings modal's fields `Root`'s `Tab` /
  `TabPrev` still run after it. `cx.intercept_keystrokes` was not taken: it runs before every
  binding and would have had to tell a dialog's focus apart itself.
- `ctrl-c` / `cmd-c`: `Root`'s `Copy` copies the text selection's text and
  propagates when it is empty. The app draws no selectable text, so the key
  reaches `key_down`; an `Input`'s own `Copy` binding is deeper and copies
  its selection.
- Measured (2026-11-14, release, Windows): a temporary `f9` arm in
  `key_down`, not committed, opened `window.open_dialog` with three
  `tab_index(0)` `div`s and `close_button(false)`; the keys were posted as
  `WM_KEYDOWN` with the modifier state set through `AttachThreadInput` +
  `SetKeyboardState` (a posted Shift or Ctrl alone is not seen: GPUI reads
  the modifiers from the thread's key state). With no dialog, `tab` reached
  `key_down` as `toggleSides`, and `shift+tab` and `ctrl+c` reached it with
  no action. Inside the dialog `tab` dispatched `root::Tab` and walked A, B,
  C, A, `shift+tab` dispatched `root::TabPrev` back to C then B, `escape`
  closed it and gave the focus back to the app's handle, and the next `tab`
  reached `key_down` with no action dispatched.
- Rule: on macOS `Root::new` installs a window hit-test forwarder
  (`install_window_hit_test_forwarder`), not yet seen on this app.
- Rule: a version bump of `gpui-component`, `gpui-base` or `gpui-pre`
  re-checks the `Root` context's bindings (`KeyBinding::new` with
  `Some(CONTEXT)` in `gpui-base`'s `root.rs`), that a `NoAction` in a deeper
  context still masks a shallower binding (`main.rs`'s
  `tab_and_shift_tab_reach_the_keymap_only_inside_the_view` rebuilds that
  case on GPUI's `Keymap`), that dialogs are still drawn outside the app's
  view and a `deferred` keeps its parent dispatch node, what
  `RootPlugin::prepare` / `style` set (the rem above all), and runs the A/B
  gate, since the wrapper is drawn every frame.

### The license notices are due at packaging (Inferred)

`gpui-component` and the `gpui-pre-*` crates are Apache-2.0
(`LICENSE-APACHE`, Copyright Longbridge; GitHub shows NOASSERTION only for the
file name), and `gpui-kit-assets` embeds Lucide (ISC).

The shipped themes under `crates/app/themes/` are copied unchanged from
gpui-kit's `themes/` at tag v0.9.1 (commit `39ab849a`), under the same
Apache-2.0 but for the two CC BY-SA 4.0 ones below; their `LICENSE` names that source, each file's own `author` and
`url` (the color scheme it adapts) and carries the license text.

Each adapted scheme's own upstream license was checked on 2026-11-23 and is
noted per file in that `LICENSE`, with the copyright lines its MIT and
Apache-2.0 upstreams require: MIT for Ayu, Catppuccin, Everforest,
Flexoki, Gruvbox, Hybrid, Jellybeans, macOS Classic, Mellifluous, Molokai,
Solarized and Spaceduck; Apache-2.0 for Asciinema (asciinema-player's
palette), Aurora (gpui-kit's own) and Tokyo Night; the TextMate themes
bundle's permissive notice for Twilight. The rest, decided on 2026-11-23
(releases up to 2.1.0 shipped all 21):

- Removed: `alduin.json`, `adventure.json` and `kibble.json`, whose
  schemes' terms could not be determined. They are not copied, and a
  re-copy at a `gpui-component` bump must leave them out (the comment on
  `theme.rs`'s `FILES` says so); a stored name of one of their themes reads
  as Neutral.
- Kept with attribution: `harper.json` (Jan T. Sott) and `fahrenheit.json`
  (a port of fcpg's vim-fahrenheit), whose schemes are Creative Commons
  Attribution-ShareAlike 4.0 International. The `LICENSE` gives each one's
  author, source, license and changes, and distributes the two files under
  CC BY-SA 4.0, not under gpui-kit's Apache-2.0 or Handpic's
  `MIT OR Apache-2.0`; the app only reads them as data.

`tools/package/notices.sh <dir>` copies the set every package ships into one
`licenses/` folder: `crates/app/packaging/NOTICE-gpui` (the GPUI crates'
copyright lines, Longbridge's, Zed Industries' and Sean McArthur's (for
`gpui-pre-reqwest`), and the Apache-2.0 text of
their `LICENSE-APACHE`), `crates/app/LICENSE-lucide`, the themes' `LICENSE`
as `LICENSE-themes`, and the models' `LICENSE` (YuNet) as `LICENSE-yunet`
and `LICENSE-mediapipe`, `crates/core/LICENSE-pillow`, and Handpic's own
license texts, the root `LICENSE-MIT` and `LICENSE-APACHE` (Handpic is
`MIT OR Apache-2.0`).

`crates/core/src/resample.rs` ports PIL's bilinear and bicubic resampling.
Decided on 2026-11-27, the conservative reading: the port is treated as
derived from Pillow, so it ships Pillow's MIT-CMU notice,
`crates/core/LICENSE-pillow` (a header naming the port, then Pillow's
upstream `LICENSE` unchanged).

- Rule: every package gets its notices from `notices.sh`, never a list of
  its own; a new bundled asset under another license adds its notice there,
  and so does a port of third-party code.
- Rule: a version bump of the GPUI crates re-checks `NOTICE-gpui`'s crate
  list, versions and copyright lines against the crates' `LICENSE-APACHE`.

### Linux needs system libraries, macOS and Windows do not (Inferred)

Per Zed's `script/linux`: `libfontconfig-dev`, `libwayland-dev`,
`libx11-xcb-dev`, `libxkbcommon-x11-dev`, `libvulkan1`, plus a Vulkan-capable
driver at run time. The `app` CI job's Linux leg installs exactly these.
At run time wgpu falls back to GL through `libEGL.so.1` when no Vulkan
adapter is found, so the deb depends on `libegl1` as well as `libvulkan1`
(see the deb's `Depends` rule under "Packaging"; not measured on a
Vulkan-less machine).
Since the switch to `gpui-kit` the X11 and Wayland backends are compiled (its
`x11` and `wayland` features); before, neither was.
macOS needs the Xcode CLT (Metal), Windows only MSVC.

- Rule: add a package to the CI job's apt step only when a build error names
  it, and keep this list equal to that step.

### macOS compiles its shaders at startup (Inferred)

`gpui_apple`'s build script runs `xcrun metal` and `xcrun metallib` unless
`gpui_platform`'s `runtime_shaders` feature is on. `gpui-kit` turns that
feature on, so the build does not need the Metal toolchain and the shaders are
compiled when the app starts instead. `macos-latest` is arm64, where
`build.rs` returns before the NASM check.

- Rule: `runtime_shaders` comes with `gpui-kit` and cannot be turned off from
  the app's manifest; do not add a build-time Metal step.

### The `app` CI job runs on all three platforms (Inferred)

It is a matrix over `windows-latest`, `ubuntu-latest` and `macos-latest` with
`fail-fast` off, each leg running the same `mise exec rust@1.98.1 -- cargo
clippy` / `cargo test`; only the Windows leg adds `setup-nasm`'s
`$USERPROFILE/nasm` to `PATH` itself. Nothing under `crates/app/src` is
`cfg(windows)`-only, and `tree.rs`'s path helpers work on strings, so its
`C:\` and UNC test paths do not depend on the host's `std::path`.

- Rule: code reached only under one platform's `cfg` must not fail
  `-D warnings` on the other two; give it a `cfg_attr(..., expect(dead_code))`
  or the same `cfg`.

The Windows leg alone also runs `cargo clippy --all-targets --features
screenshot -- -D warnings`, as do `mise run app:check` and `mise run
ci:local` (its `clippy app screenshot` check, selected wherever `clippy
app` is) on a Windows host. Windows is where the `screenshot` tool was
checked live, Linux's `render_to_image` is not implemented, and the macOS leg
already builds the app with the feature in its bundle step (see "The macOS
bundle step reuses the test build"), though not under clippy. The feature
turns on `test-support` in a direct `gpui-pre-platform` dependency, the same
crate and feature the UI tests' dev-dependency turns on (see "The main view
opens on GPUI's test platform"), so the default `clippy --all-targets` already
builds the GPUI crates that way and this run only re-checks `handpic-app`.

- Rule: keep the feature command string identical in `ci.yml`,
  `tools/ci/local.sh` and `app:check`.

### The macOS bundle step reuses the test build (Measured)

The `app (macos-latest)` job has `timeout-minutes: 30`, and cold-cache runs
(the `actions/cache` step restoring nothing) took 24 to 29.5 minutes and
were cancelled at the limit on some PRs: "Lint and test the app" 15 to
19 minutes and "Bundle and check Handpic.app" 8 to 10 minutes. A warm run
takes 2 to 5 minutes. The bundle step's plain `cargo build` had the
dev-dependency `gpui-pre-platform` with `test-support` off, so the GPUI
crates compiled a second time in another feature variant after `cargo test`
built them with it. On 2026-11-27 the step was changed to `mise run
app:bundle:macos debug --features screenshot` rather than the timeout
raised: the `screenshot` feature (`["dep:gpui-pre-platform", "image/png"]`,
`gpui-pre-platform` with `test-support`) turns on the same dependency
features the dev-dependencies do (`gpui-pre-platform` with `test-support`,
`image` with `png`), and `cargo test` and `cargo build` share the dev
profile's dependency artifacts, so the step compiles only `handpic-app`.
Raising the limit would have kept the duplicate compilation on every cold
run and the second variant in the macOS cache (2.4 GB) of a repository whose
caches overrun the 10 GB limit and are evicted. The step checks only the
bundle's layout, which the feature does not change; the release bundle comes
from `package.yml`, still with no feature. So `ci.yml` no longer builds a
default-feature, non-test app on any platform; `Package` still does.

- Rule: the CI bundle build keeps the dependency feature set of `cargo test`.
  A `Compiling gpui-*` line in the bundle step's log means they differ;
  compare `cargo tree -e features --features screenshot` with the test
  build's before raising `timeout-minutes`.

### Without NASM, mozjpeg silently loses SIMD (Measured)

`mozjpeg-sys`'s build script runs `nasm -v`, prints a `cargo:warning` and
continues without SIMD when that fails, and cargo hides build-script warnings
of non-path dependencies. RAF decode measured about 99 ms without SIMD and
46 ms with it. `crates/app/build.rs` therefore fails the build, and its own
`cargo:warning` is the only place the SIMD state is visible.

### winget's NASM is per-user and not on `PATH` (Hit)

`winget install NASM.NASM` installs to `%LOCALAPPDATA%\bin\NASM`, not
`C:\Program Files\NASM`, and adds nothing to `PATH`.

- Rule: put that directory on `PATH` for every build.

### `mozjpeg-sys` ignores the `NASM` variable (Inferred)

`nasm-rs` honors `NASM` as the binary path, but the `nasm_supported()` probe of
`mozjpeg-sys` 2.2.3 runs plain `nasm` from `PATH`. `build.rs` therefore runs
`nasm -v` from `PATH` only and fails when it does not run, whatever `NASM` holds.

- Rule: put `nasm` on `PATH`; setting `NASM` does not satisfy the build.

## Packaging

Source: [the native-app-icon plan](../plans/_archived/20261114-native-app-icon/plan.md).

### The Windows icon is resource ID 1 of the exe (Measured)

`gpui-pre-windows` 0.3.12's `load_icon` loads `LoadImageW(module, PCWSTR(1),
IMAGE_ICON, ..)` into the window class's `hIcon`, and a failed load silently
leaves the generic icon. `crates/app/build.rs` therefore embeds
`crates/app/icons/icon.ico` with `winresource`'s `set_icon`, which writes
ID `1`, on a Windows target only (`CARGO_CFG_TARGET_OS`; the crate is a
`cfg(windows)` build-dependency), before the NASM check's early returns. The
icon is referenced by path from `CARGO_MANIFEST_DIR`. Explorer shows the exe's first icon group, so the one
resource serves the taskbar, the title bar and Explorer. In the built debug
exe, `FindResourceW` finds `RT_GROUP_ICON` 1, one `RT_MANIFEST` 1 and
`winresource`'s `RT_VERSION` 1, and `LoadImageW(.., 1, IMAGE_ICON, ..)`
succeeds.

- Rule: never set a manifest in the app's resource. `gpui-pre-platform` turns
  on `gpui-pre`'s `windows-manifest` feature, whose build script already
  embeds `1 RT_MANIFEST`; a second one is a duplicate resource at link time.
- Rule: the icons live in `crates/app/icons/`, so a change to one selects
  the app checks in `mise run ci:local` like any change in the crate.

### A Windows release build is a GUI-subsystem binary (Measured)

Source: [the windows-gui-subsystem plan](../plans/_archived/20261121-windows-gui-subsystem/plan.md).

`crates/app/src/main.rs` starts with
`#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`,
so a release build's PE `Subsystem` is 2 (GUI) and a launch from Explorer,
the Start menu or an updater relaunch opens no console window. A debug build
(`mise run app:dev`, `cargo test`) has no attribute and stays a console
binary (3), so its terminal keeps the log lines and the timing output.
`not(debug_assertions)` is the repository's meaning of "release build" (`main`
passes `!cfg!(debug_assertions)` to the update flow) and `[profile.release]`
does not turn debug assertions on.

Read the field with
`$b=[IO.File]::ReadAllBytes($exe); $o=[BitConverter]::ToInt32($b,0x3c); [BitConverter]::ToUInt16($b,$o+0x5c)`
(the UInt16 at `e_lfanew + 0x5c`). Measured on Windows 11 with rust 1.98.1:
the release exe reads 2 and the debug exe 3. Started by `Start-Process`, the
debug exe gets a `conhost.exe` child and `AttachConsole` to it succeeds; the
release exe has neither, and its `--log` file holds the launch's lines
(`settings:`, the scan lines of the opened folder).

When checking for a console, look at the launched process's own children
(`Get-CimInstance Win32_Process` on its PID, and `AttachConsole(pid)`). Do not
compare system-wide `conhost.exe` counts before and after the launch, because
unrelated processes create conhosts all the time and the check gives false
positives. To read a terminal's output in a test, open a new window per case
and dump `$host.UI.RawUI.GetBufferContents`. Use a classic `conhost.exe`
window for `mise run` cases, because under the default ConPTY terminal the
buffer read came back blank or out of range.

A GUI process started with no std handles gets null handles, which std
treats as a sink: `println!`, `eprintln!` and `writeln!(io::stderr(), ..)`
return normally. A panic there (provoked in a throwaway build, on a spawned
thread and on the main thread) left both `panic:` lines in the log and exited
101 with no secondary failure. `logger.rs` still writes stderr with `let _ =
writeln!(io::stderr(), ..)`, not `eprintln!`, because a stderr inherited as a
pipe whose reader has gone makes `eprintln!` panic, and the panic hook calls
the logger.

What a launch of the release exe from a terminal shows (`page` lines and the
quit summary of `--auto 5`, measured on Windows 11):

| Launch                                                           | Reaches the terminal                                      |
| ---------------------------------------------------------------- | --------------------------------------------------------- |
| `handpic.exe` from Git Bash in mintty                            | yes, stdout and stderr                                    |
| `mise run app:release` from Git Bash in mintty                   | no; yes with `\| cat` or `> file`                         |
| `mise run app:release` from PowerShell / cmd (conhost or ConPTY) | no; yes with `\| Out-Host` (PowerShell) or `> file` (cmd) |
| `mise run app:release` from Git Bash running in a console        | no                                                        |
| `handpic.exe` from PowerShell 5.1, unredirected                  | no; the prompt returns at once                            |
| `handpic.exe ... > file` from PowerShell 5.1                     | no; the prompt returns at once and the file is empty      |
| `handpic.exe ... \| Out-Host` from PowerShell 5.1                | yes; PowerShell waits for the exit                        |
| `handpic.exe` from cmd, unredirected                             | no                                                        |
| `handpic.exe ... > file` from cmd                                | yes                                                       |
| A pipe all the way (an agent's shell tool, CI)                   | yes                                                       |

Wherever a console is the std handle (mise and cargo hand theirs down, as
does Git Bash in a console window rather than mintty), the GUI process's
writes are lost; a pipe or a file carries them. Under mintty the direct
launch gets pipes, but through `mise` the output is lost too, presumably
because mise, a console program, is given a pseudo console whose handles
reach `handpic.exe` through cargo. `Handpic.log` holds every log line in
every case; only the `println!` timing output and the usage error have
nowhere else to go.

- Rule: for timing on Windows, run `mise run app:release -- <folder>
  --auto N | cat` in Git Bash, `| Out-Host` in PowerShell or `> file` in
  cmd. A debug build keeps its console but is no use for timing.
- Rule: the usage error (`eprintln!` + `process::exit(2)`) keeps its exit
  code; its text shows only where stderr is a pipe or a file.
- Rule: nothing in CI reads the release exe's console (the `Package`
  workflow's Windows check finds the relaunched app by its process or the
  log file's mtime), and the app starts no console child on Windows (the only
  `Command::new` there is the detached Inno Setup setup).

### Linux matches the window to its `.desktop` file by `IDENTIFIER` (Inferred)

The window opens with `app_id: Some(IDENTIFIER)`. Wayland matches a
toplevel's `app_id` to the `.desktop` file's base name, so the file is
`crates/app/packaging/linux/dev.minodisk.handpic.desktop`. On X11,
`gpui-pre-linux`'s `set_app_id` writes `WM_CLASS` as `IDENTIFIER` for both the
instance and the class, so `StartupWMClass` is `IDENTIFIER` too. The default
`set_app_id` is a no-op on Windows and macOS. `Exec=handpic` is static,
so `~/.local/bin` must be on the session's `PATH` for the install task's copy;
the packaged copies' `Exec` is absolute (see "The `Package` workflow builds
the distributables").

`mise run app:install:linux` builds the release binary and installs it to
`~/.local/bin/handpic`, the `.desktop` file to
`~/.local/share/applications/` and `crates/app/icons/128x128@2x.png` (256 x
256 px) to `~/.local/share/icons/hicolor/256x256/apps/dev.minodisk.handpic.png`.

- Rule: in a `mise.toml` `run = """..."""` (a TOML basic string), a `\` at the
  end of a line is TOML's line-ending backslash and swallows the newline. A
  shell line continuation needs `\\`; the install task instead keeps each
  command on one line.
- Rule: the `hicolor` size directory equals the PNG's real pixel size.

### macOS gets `Handpic.app` from a mise task (Inferred)

`mise run app:bundle:macos [profile] [--features <features>]` (release and
no cargo feature by default) builds and lays out `crates/app/target/<profile>/Handpic.app` with the checked-in
`crates/app/packaging/macos/Info.plist`, `Contents/MacOS/Handpic` (the
Cargo bin `handpic`, copied under the shipped name) and
`crates/app/icons/icon.icns` as `Contents/Resources/icon.icns`, writes the
numeric form `tools/package/version.sh numeric` prints into the copied
`Info.plist`'s `CFBundleShortVersionString` and `CFBundleVersion` (`plutil
-replace`; the checked-in file keeps `0.0.0`) and copies the notices into
`Contents/Resources/licenses/` (`notices.sh`). It does not sign. The `app`
CI job's macOS leg bundles the debug build with the `screenshot` feature (see
"The macOS bundle step reuses the test build") and checks it (`plutil -lint`,
the executable and the `.icns` present).

- Rule: the bundle identifier and the Linux `app_id` are `IDENTIFIER`, the
  one the Tauri app had, which the app replaced. `datadir.rs`'s
  tests check that `Info.plist` and the `.desktop` file agree with
  `IDENTIFIER` and the binary's name; change them together.
- Rule: the `app` CI job's macOS leg runs `mise run app:bundle:macos
  debug --features screenshot` itself, so CI exercises the task this section describes. The
  Windows-only NASM `PATH` problem (see "winget's NASM is per-user and not on
  `PATH`") that keeps other steps on `mise exec` does not apply on macOS.
- Pending manual check, not done here: the running window's icon after the
  install task on a Wayland and an X11 session. The Dock / Finder icon of the
  built `.app` was seen on macOS on 2026-11-26
  (`docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`), and
  the taskbar, title bar and Explorer icon on Windows on 2026-11-27
  (`docs/plans/_archived/20261127-windows-device-checks-c5/results.md`).

### The `Package` workflow builds the distributables (Inferred)

`.github/workflows/package.yml` (dispatched by hand, or called by
`release.yml` through `workflow_call`; `contents: read`, no release touched)
builds each platform's packages and uploads each as a
workflow artifact, then a `checksums` job uploads one `SHA256SUMS` over all
of them. Each leg builds with `mise exec rust@1.98.1 -- cargo build --release
--locked` in `crates/app`, as the `app` CI job runs cargo, and then
packages the built `target/release` binary with a script under
`tools/package/`, which builds nothing.

- Rule: the shipped executable is `Handpic.exe` on Windows (the portable
  ZIP's `Handpic/Handpic.exe`, the setup's `{app}\Handpic.exe`) and
  `Contents/MacOS/Handpic` in `Handpic.app`, while the Cargo bin stays
  `handpic` (`crates/app/Cargo.toml`), so `cargo run`, `target/<profile>/`,
  the mise tasks and the Linux packages keep `handpic`. The rename happens
  in packaging only: `windows.sh` (`binary`, cargo's output, and `shipped`,
  passed to `handpic.iss` as `BuiltBinary` and `Binary`, the `[Files]`
  entry's `DestName`), `app:bundle:macos` in `mise.toml`, `macos.sh` and the
  `Info.plist`'s `CFBundleExecutable`. The updater looks the executable up
  in an archive by that fixed name per install kind, never by the running
  exe's own file name (below). An Inno Setup upgrade over an install holding
  `handpic.exe` writes the new casing: a throwaway setup with its own
  `AppId` installing `x.exe` and then one with `DestName` `X.exe` left only
  `X.exe` holding the new bytes (Measured 2026-11-23, Inno Setup 6.7.3), so
  `handpic.iss` has no `[InstallDelete]` for it.

- `version.sh [form] [version]` prints `[package].version` of
  `crates/app/Cargo.toml` (`0.0.0` until release-please owns it), or the
  version given, in the form a package format takes, and refuses a version
  that is not `x.y.z` or `x.y.z-<prerelease>` (the prerelease dot-separated
  alphanumeric identifiers, no `-`, which the deb form could not carry):

  | Form      | Stable    | Prerelease   | Used by                                              |
  | --------- | --------- | ------------ | ---------------------------------------------------- |
  | `semver`  | `2.0.0`   | `2.0.0-rc.1` | every asset file name, Inno `AppVersion`, the binary |
  | `numeric` | `2.0.0`   | `2.0.0`      | `CFBundleShortVersionString`, `CFBundleVersion`      |
  | `windows` | `2.0.0.0` | `2.0.0.0`    | Inno `VersionInfoVersion`                            |
  | `deb`     | `2.0.0`   | `2.0.0~rc.1` | the control file's `Version`                         |

  `semver` is the default. The deb's file name keeps the SemVer string
  (`handpic_2.0.0-rc.1_amd64.deb`); only the control field uses `~`, so a
  prerelease sorts before its release.
- `set-version.sh <version>` writes a version to `crates/app/Cargo.toml`
  and to the crate's entry in `crates/app/Cargo.lock`, so `cargo build
  --locked` still passes and the binary (on Windows, the exe's
  `ProductVersion`, which `winresource` takes from `CARGO_PKG_VERSION`)
  carries it. The workflow's optional `version` input runs it before the
  build (`gh workflow run package.yml --ref <branch> -f version=2.0.0-rc.1`);
  a tester runs it for local N and N+1 builds.
  The version rule it and `version.sh` share: `x.y.z` without leading zeros,
  and an optional prerelease of dot-separated alphanumeric identifiers. `-`
  inside the prerelease and build metadata (`+...`) are refused: a `-` in a deb
  upstream version reads as the revision separator, so `2.0.0~rc-1` would not
  mean what the control file intended. `version.sh [form] [version]` maps a
  given version too, so the workflow's deb ordering check goes through the
  script, not hand-written `~` forms.
- `macos.sh` signs the laid-out `Handpic.app` ad hoc (`codesign --force
  --sign -`, then `codesign --verify --strict`; no Developer ID, no
  notarization), stages it with `ditto` (not `cp -R`, so the ad hoc
  signature's extended attributes survive the copy) beside an
  `/Applications` link and writes
  `crates/app/target/release/Handpic-<version>-macos-<arch>.dmg`
  (`hdiutil create -format UDZO`, `<arch>` from `uname -m`). The workflow
  runs it on `macos-latest` (arm64) and `macos-15-intel` (x86_64), after
  `mise run app:bundle:macos`, and checks the DMG (`hdiutil verify`, then
  attached: the signature, the executable's architecture, both version
  fields numeric and equal to the `numeric` form, and the notices). It also
  writes `Handpic-<version>-macos-<arch>.tar.gz`, the staged `Handpic.app`
  alone, for the updater, with `COPYFILE_DISABLE=1` so bsdtar adds no `._`
  entries; the workflow checks it holds nothing else and that the extracted
  bundle passes `codesign --verify --strict` and has the runner's
  architecture.
- `mise run app:package:macos` is the same locally on a Mac: the bundle
  task (its release build included), then `macos.sh`.
- `windows.sh` compiles `crates/app/packaging/windows/handpic.iss` with
  Inno Setup 6's `ISCC.exe` (found on `PATH`, else in its per-user and
  machine-wide install folders, else it fails with the `winget install
  JRSoftware.InnoSetup` hint) into
  `crates/app/target/release/Handpic-<version>-windows-x64-setup.exe`,
  passing the version (`AppVersion`, the SemVer form, and `FileVersion`,
  the `windows` form the `.iss` sets as `VersionInfoVersion`, which Inno
  derives from `AppVersion` only when that is numeric), the executable's name
  and the folders as `/D` defines, and zips one `Handpic/` folder (the exe and `licenses/`) into
  `Handpic-<version>-windows-x64.zip` with Windows' own
  `System32/tar.exe -a` (Git Bash has no `zip` and its GNU `tar` writes
  none). The setup installs per user (`PrivilegesRequired=lowest`,
  `{autopf}\Handpic`, which is `%LOCALAPPDATA%\Programs\Handpic`), adds a
  Start menu shortcut (`{autoprograms}`) and takes its icon from
  `crates/app/icons/icon.ico`, read in place. The workflow's `windows` leg
  runs it on `windows-latest`, checks the setup's file version is the
  `windows` form, then installs the setup silently
  (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`), checks the exe under
  `%LOCALAPPDATA%\Programs\Handpic` and its `ProductVersion`, the `HKCU`
  uninstall entry and its `DisplayVersion` (both the SemVer form), no `HKLM` entry and the shortcut, runs the setup again
  over the install, uninstalls silently and checks all of it gone, then
  checks the ZIP's contents. It also writes
  `Handpic-<version>-windows-x64-setup.exe.zip`, the setup alone at the
  ZIP's root, for the updater; the workflow checks `Expand-Archive` gives that
  one file with the setup's digest. `windows-latest` ships Inno Setup 6.7.1 with
  `iscc` on `PATH` (Chocolatey's shim), so the workflow installs nothing.
  `mise run app:package:windows` is the release build plus `windows.sh`
  locally.
- `linux.sh` (x86_64 only, needs `dpkg-dev`) writes three files under
  `crates/app/target/release`:
  - `Handpic-<version>-linux-x64.tar.gz`, one `Handpic/` folder with
    `bin/handpic`, the checked-in desktop entry under
    `share/applications/`, `crates/app/icons/128x128@2x.png` as
    `share/icons/hicolor/256x256/apps/dev.minodisk.handpic.png`, and
    `licenses/`;
  - `install.sh`, a copy of `crates/app/packaging/linux/install.sh`
    (adapted from gpui-kit's guide, Apache-2.0, said in its header): `sh
    install.sh ARCHIVE EXPECTED_SHA256` reads the whole archive against the
    digest before anything changes, refuses an entry outside `Handpic/`, a
    link or a duplicate, and installs per user (Python 3, no root) into
    `app:install:linux`'s layout, the desktop entry's `Exec` rewritten to
    the quoted absolute `~/.local/bin/handpic`;
  - `handpic_<version>_amd64.deb` (`Package: handpic`), built by `dpkg-deb
    --root-owner-group --build` from `usr/lib/handpic/handpic`, the
    desktop entry with `Exec=/usr/lib/handpic/handpic`, the icon, and
    the notices under `usr/share/doc/handpic/`, its `DEBIAN/control` from the
    `crates/app/packaging/linux/control` template (the `deb` form of the
    version, installed size, `Depends`).

  The workflow's `linux` leg runs it on `ubuntu-latest` after the `app`
  CI job's apt list, prints the binary's `NEEDED` entries, the sonames it
  names and the image's glibc, runs `install.sh` under a throwaway `HOME`
  and checks the binary, the desktop entry (absolute `Exec`, `Icon`,
  `StartupWMClass`) and the icon, checks that a wrong digest installs
  nothing, prints `dpkg-deb --info` / `--contents`, checks the `Version` is
  the `deb` form and that `dpkg --compare-versions` puts `2.0.0~rc.2` before
  `2.0.0~rc.10` before `2.0.0`, checks the `Depends`,
  and installs and removes the deb with `apt-get`. Nothing is launched: the
  runner has no display and no GPU. `mise run app:package:linux` is the
  release build plus `linux.sh` locally.
- `sign.sh <private-key> <verifying-key> <archive>...` signs the four
  archives the updater installs (the portable ZIP, the setup ZIP, the Linux
  and the macOS tarballs) in place with zipsign 0.2.2, then runs `zipsign
  verify` on each with the verifying key, so a key pair that does not match
  fails at signing. A `.zip` gets its signature before its data, a `.tar.gz`
  as a gzip member after it. `mise run app:sign -- <args>` runs it with
  the pinned zipsign (`cargo:zipsign`, a task tool, so nothing else installs
  it); the script refuses another zipsign version. The DMG, the setup `.exe`,
  `install.sh` and the deb are installed by people, not the app, and stay
  unsigned, covered by `SHA256SUMS` only.
- The workflow's optional `update_test_repo` input (`owner/name`) makes a
  test build: each leg sets `HANDPIC_UPDATE_TEST_REPO` for the build (unset
  otherwise) and signs its archives with the test key pair checked in under
  `crates/app/packaging/test/` (its README says no released build trusts
  it), before the checks, so the checks and the uploads see the signed
  bytes. Without the input a dispatch signs nothing. The production
  signature is only for a caller: `release.yml` calls the workflow with
  `sign_production: true` and the `UPDATE_SIGNING_KEY` secret passed by
  name, and each leg then declares the `release` environment, which holds
  that secret, and runs `sign-production.sh` on its archives at the same
  place, before its checks. Without `sign_production` a leg's environment
  name is empty, so a dispatch declares no environment. `sign_production`
  is a `workflow_call` input a dispatch does not have, so a dispatch on
  `main`, which the environment's branch policy admits, still signs nothing
  with the production key. A dispatch's `version` and
  `update_test_repo` behave as before; a caller has no `update_test_repo`.
  The `checksums` job computes `SHA256SUMS`
  over the downloaded artifacts, so after signing, and installs the
  tarball with `install.sh` and the digest `SHA256SUMS` names.
- `test-release.sh <run-id> <owner/name>` publishes the artifacts of a
  `Package` run made with `update_test_repo` as the release `v<version>` of
  that scratch repository, the test feed such a build updates from
  (`minodisk/handpic-update-test`, public, holding only a README; the plan
  `docs/plans/20261118-native-updater/` made it). It downloads the run's
  artifacts with `gh run download`, reads the version from the portable
  ZIP's name, checks every file with `sha256sum --check` against the run's
  `SHA256SUMS`, and uploads them as they are (never renamed, so the zipsign
  signatures still verify), as a GitHub prerelease when the version has a
  prerelease part. It refuses `minodisk/handpic`: a test build trusts the
  test key and never goes on the real feed. A tester packages two versions,
  N and N+1 (`gh workflow run package.yml --ref <branch> -f
  version=2.0.0-rc.1 -f update_test_repo=minodisk/handpic-update-test`,
  then `rc.2`), publishes both, installs N and lets it find N+1. The feed
  lists every release of the repository, so delete a test release that
  should no longer be offered (`gh release delete v<version> --repo <owner/name>
  --cleanup-tag`).
- `mise run app:keygen <private-key>` generates the production key pair
  (`zipsign gen-key`): the private key to the given path, refused inside the
  repository, and the 32-byte verifying key to
  `crates/app/packaging/update.pub`, which is checked in. zipsign refuses
  to overwrite either file.

- Rule: zipsign binds a signature to the file's name (its default
  `--context`), so a signed archive is never renamed, and `SHA256SUMS` is
  always computed after signing. A copy under another name fails `zipsign
  verify` (measured).
- Rule: Windows' `Expand-Archive` (PowerShell 5.1 locally, and the runner's
  `pwsh`) opens a ZIP zipsign prepended its signature to, and GNU tar and
  Python's `tarfile` read a signed `.tar.gz` without a warning (measured
  locally with zipsign 0.2.2).
- Rule: the key files are binary (`-text` in `.gitattributes`): 32 random
  bytes usually hold no NUL, so `text=auto` would take them for text.
- Rule: the setup's `AppId` is `dev.minodisk.handpic` (`IDENTIFIER`), for
  good: Inno Setup finds the earlier install, its folder and its uninstall
  entry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\dev.minodisk.handpic_is1`)
  by it, so a changed `AppId` installs a second copy instead of upgrading.
- Rule: under Git Bash, `ISCC.exe`'s `/`-options need `MSYS_NO_PATHCONV=1`
  (and `MSYS2_ARG_CONV_EXCL='*'`), or MSYS rewrites them as paths.
- Rule: the setup's two updater switches, read in its `[Code]`:
  `/WAITPID=<pid>` makes `InitializeSetup` wait (at most 60 s) for that
  process to exit before anything is installed, so the quitting app's exe is
  no longer in use; `/RELAUNCH` turns on the `[Run]` entry that starts the
  installed app at the end. A setup run by hand passes neither and starts
  nothing. The Restart Manager settings are Inno's defaults. The `windows`
  leg checks both: an upgrade with `/WAITPID` of a stand-in holding the exe
  open waits for it, starts nothing, and one with `/RELAUNCH` starts the app.
  A CI step that runs the setup with `/RELAUNCH` must not wait with pwsh 7's
  `Start-Process -Wait`: that waits for the whole process tree, so it never
  returns while the relaunched Handpic runs (one run hung for 40 minutes).
  `Invoke-Silent` starts the setup with `-PassThru` and waits on its own
  handle (`WaitForExit` with a bound), and the step has `timeout-minutes`.
- Rule: the deb's `Depends` is `dpkg-shlibdeps`'s over the built binary plus
  what GPUI opens at run time, never a copied list. Measured on
  `ubuntu-latest` (Ubuntu 24.04.5, glibc 2.39; `Package` runs 44096260804 and
  44099086987):
  `NEEDED` holds only `libxcb.so.1`, `libxkbcommon.so.0`,
  `libxkbcommon-x11.so.0`, `libgcc_s.so.1`, `libm.so.6`, `libc.so.6` and
  `ld-linux-x86-64.so.2`, and the newest symbol version is `GLIBC_2.39`, so
  `dpkg-shlibdeps` gives `libc6 (>= 2.39), libgcc-s1 (>= 4.2), libxcb1 (>=
  1.12), libxkbcommon-x11-0 (>= 0.5.0), libxkbcommon0 (>= 0.5.0)`. Neither
  Vulkan nor Wayland is linked: wgpu (Vulkan and GL backends,
  `gpui-pre-wgpu`) loads `libvulkan.so.1`, or `libEGL.so.1` for GL, and
  GPUI's `wayland-backend` (`client_system`, `dlopen`) loads
  `libwayland-client.so.0`. `linux.sh` therefore adds `libvulkan1`, the GL
  fallback's `libegl1` and `libwayland-client0` by hand. `libegl1` is a
  plain dependency, not `libvulkan1 | libegl1`: the Vulkan loader alone
  gives no adapter on a machine without a Vulkan driver, where wgpu's GL
  fallback then opens `libEGL.so.1`, and the alternative form would let
  `libvulkan1` alone satisfy it. `libegl1` is small and on every desktop.
  Not measured on a Vulkan-less machine.
  The deb and the tarball need glibc 2.39 or later (Ubuntu 24.04, Debian
  13); an older distribution needs an older build image.
- Rule: the deb's `Package` is `handpic`. The Tauri v1.10.0 deb was
  `Package: riffle` (read from `Riffle_1.10.0_amd64.deb`), so the app deb
  does not upgrade it in place, and the control has no `Conflicts` /
  `Replaces` against `riffle`. Handpic and Riffle are separate products
  with their own package names, binaries and install paths
  (`/usr/lib/handpic`), so the two debs install side by side, as Riffle
  1.10.0 and Handpic 2.1.0 do on Windows (two uninstall entries, seen on
  2026-11-27); a `Replaces` / `Conflicts` would also make apt remove a
  Riffle the user may still want. Decided from the packaging, without a
  real-device deb check.
- Rule: the one desktop entry is the checked-in
  `dev.minodisk.handpic.desktop` with the bare `Exec=handpic`
  (`datadir.rs` tests it, and `app:install:linux` installs it as is); the
  packaged copies get an absolute `Exec`, `linux.sh` writing the deb's and
  `install.sh` the tarball install's.

- Rule: `app:bundle:macos` stays the one place that lays out
  `Handpic.app`; `macos.sh` only signs and packages it.
- Rule: the executable's name is one `binary` variable per script
  (`handpic`).
- Rule: the workflow's cargo cache is keyed by `matrix.os`, not `runner.os`:
  both macOS runners report `macOS` but build for different architectures.

### The `Release` workflow publishes a release (Inferred)

`.github/workflows/release.yml` runs `release-please` on every push to
`main` and builds, signs and publishes a release of `minodisk/handpic` with
the eleven assets `tools/package/assets.sh <version>` lists (the two DMGs,
the two macOS tarballs, the setup, the setup ZIP, the portable ZIP, the
Linux tarball, `install.sh`, the deb and `SHA256SUMS`; no `latest.json`),
the names `crates/engine/src/update.rs` asks for. It has two ways in:

- **A release candidate**, by hand: `gh workflow run release.yml --ref main
  -f version=2.0.0-rc.1`. The version must have a prerelease part (a stable
  one is refused: it is cut by merging the release PR), and on `main` the
  release `v<version>` and the tag must not exist yet. The version is
  written only in the runner's checkout (`set-version.sh`, inside the
  build); nothing is committed, `CHANGELOG.md` and the manifest stay as they
  are, and release-please is not involved. `gh release create` creates the
  tag `v<version>` at the dispatched commit and a GitHub **prerelease** with
  `--latest=false`, so `/releases/latest` does not move; with files it
  uploads to a draft and publishes at the end, so the candidate is never
  visible half-attached.
- **The final release**, by merging the release PR: when `release-please`
  reports `release_created`, the tag's version is built, the assets are
  uploaded to the draft release `release-please` made (`--clobber`, so a
  re-run replaces them), and the draft is published (`--draft=false
  --latest`) only after every asset is attached. A failed build leaves a
  draft, which neither the updater nor `/releases/latest` sees. This path
  first runs at the real release; it restores the draft-then-publish shape
  of the pre-removal Tauri workflow (#364).

The jobs, shared by both ways in:

1. `guard` (`ubuntu-latest`, minutes, in the `release` environment)
   resolves the version, and fails before any build on a version the run may
   not build or publish, on a ref other than `main` (only if the
   environment's policy were ever loosened; GitHub rejects such a dispatch
   before the job starts), or on an `UPDATE_SIGNING_KEY` secret that is empty,
   does not decode to a 64-byte key, or does not match
   `crates/app/packaging/update.pub`, each with a message that names the
   secret. It proves the key by signing a scratch `.tar.gz` with
   `sign-production.sh`, the path every leg signs through. On the final
   path it also asserts that `crates/app/Cargo.toml`'s version (written by
   the release PR) equals the tag's, because the updater pins `v<version>`
   and the asset names carry it; the build then writes the same version
   again (`set-version.sh` is idempotent).
2. `build` calls `package.yml` with the version, `sign_production: true` and
   the secret by name, so the four updater archives are signed with the
   production key in their own legs, each in the `release` environment, and
   `SHA256SUMS` is computed after signing. The calling job itself declares
   no environment: a job that calls a reusable workflow cannot.
3. `publish` (whenever the build succeeded) downloads the artifacts, checks
   that they are exactly `assets.sh`'s eleven, that `SHA256SUMS` names the
   other ten and that `sha256sum --check --strict` passes, and uploads them
   unrenamed (zipsign binds a signature to the file's name).

A dispatch on any ref but `main` (`gh workflow run release.yml --ref
<branch> -f version=2.0.0-rc.0`) is rejected at `guard` by the `release`
environment's branch policy: no step runs, and `build` and `publish` are
skipped. A branch is packaged through `package.yml` with the test key
instead (`gh workflow run package.yml --ref <branch> -f
update_test_repo=<owner/name>`). A dispatch has its own concurrency group
per version, so a candidate's build of about 75 minutes neither queues the
pushes' `release-please` runs nor another candidate.

- Rule: the secret `UPDATE_SIGNING_KEY` is the `release` environment's
  secret (not a repository secret), the 64-byte zipsign private key that
  `mise run app:keygen` wrote, base64-encoded on one line (`base64 -w0
  <private-key>`), set by the user on the environment's page on github.com
  or piped to `gh secret set UPDATE_SIGNING_KEY --env release` after
  checking it is non-empty. No agent reads, copies or asks for the key.
  `gh secret list --env release` cannot tell an empty secret from a set one
  ([`github-actions-workflows.md`](./github-actions-workflows.md#an-empty-secret-looks-set-in-gh-secret-list-hit)),
  which is what the guard is for.
- Rule: the key reaches a job only through `env` on the one step that signs;
  `sign-production.sh` decodes it into a `mktemp -d` folder under `umask
  077`, signs through `mise run app:sign` and removes the folder on exit. It
  never prints the key.
- Rule: `assets.sh` is the one list of a release's asset names; a new or
  renamed package changes it with the packaging script.
- Rule (decided 2026-11-26, plan `20261126-release-key-environment`): the
  production key lives only in the `release` environment, whose deployment
  branch policy allows only `main` (a custom branch policy, admins not
  allowed to bypass), so no workflow run on another ref can read it,
  whatever that ref's workflow file says: the repository-level secret is
  deleted (2026-11-27) and only the environment's copy exists. `release.yml`'s `guard` declares the environment
  unconditionally and `package.yml`'s `macos`, `windows` and `linux` legs
  declare it only under `sign_production` (`name: ${{ inputs.sign_production
  && 'release' || '' }}`), all with `deployment: false`, so the runs create
  no deployment records. The calling `build` job cannot declare an
  environment (a job that calls a reusable workflow cannot), so it passes
  `UPDATE_SIGNING_KEY` by name, which GitHub resolves to the environment's
  secret inside the legs; without that pass the secret is empty there. A
  branch dispatch of `release.yml` is rejected at `guard` before any step
  runs, with GitHub's environment protection annotation (measured on 2026-11-27,
  run 45846600132: "Branch "<branch>" is not allowed to deploy to release
  due to environment protection rules."); a branch dry run
  is `gh workflow run package.yml --ref <branch> -f
  update_test_repo=<owner/name>` with the test key. Verified (measured
  2026-11-27, run 45846600132): a `release.yml` dispatch on a branch fails
  at `guard` with no step run and `build`, `publish` and `release-please`
  skipped, from the branch policy alone (`deployment: false`). An empty
  environment name means "no environment" (measured, run 45846638280: the
  four legs of a plain `package.yml` dispatch started). Verified (measured
  2026-11-27, run 46003797551): the release of v2.2.0 from `main` passed
  `guard` and `verify-release.sh v2.2.0` reported `STATUS=ok`; with the
  repository-level secret already deleted, this shows the legs sign with
  the `release` environment's secret. The environment, its policy and its secret are the user's to
  change; an agent runs such a `gh api` or `gh secret` command only when
  the user approves it. Still not covered: the key reaches four runners
  that run third-party actions, there is no required reviewer, and anyone
  who can push to `main` can sign.

### The release runbook (Inferred)

Each of these is the user's call; an agent runs one only when asked.

- **Cut a release candidate.** `gh workflow run release.yml --ref main -f
  version=<x.y.z-rc.N>`, the next unused candidate. It takes about 75
  minutes; follow it with `gh run watch`. The result is the prerelease
  `v<version>` at the dispatched commit.
- **Cut the final release.** Merge the release PR (`chore(main): release
  x.y.z`) through `/release`, never by hand. Its push to `main` tags it,
  makes the draft, builds, signs, uploads and publishes it (see above).
  If the build fails, the draft stays; re-run the failed jobs of that run.
- **Expect the release PR to lag a `chore` merge.** release-please rewrites
  an open release PR only when its changelog changes. A `chore` commit on
  `main` leaves it "remained the same", so a change to the files it bumps
  (for example `crates/app/Cargo.toml` and `crates/app/Cargo.lock` after
  `release-please-config.json` gained them) shows up in the PR only after the
  next `feat` / `fix` merge. Check the PR's diff after that merge, not the
  `chore` one. Release candidates (prereleases) are ignored by release-please
  and leave the PR as it was, so they can stay on the feed. Never merge,
  close or hand-edit the release PR to force it. Measured in
  `docs/plans/_archived/20261121-native-release/learnings.md` (Step 1 and
  Step 3).
- **Verify a release.** `mise exec cargo:zipsign@0.2.2 -- bash
  .claude/skills/release/scripts/verify-release.sh v<version>` checks it is
  published, its prerelease flag agrees with the version, exactly
  `assets.sh`'s eleven assets are attached (no `latest.json`), they pass
  `sha256sum --check` against its `SHA256SUMS`, and the updater's five
  archives (the two macOS `.tar.gz`, the setup ZIP, the portable ZIP and
  the Linux `.tar.gz`) pass `zipsign verify` with `update.pub`, downloaded
  under their release names. It prints `ok` / `missing` lines and
  `STATUS=ok|incomplete` (exit 0 / 1; 2 when the release cannot be read or
  zipsign 0.2.2 is not on `PATH`, which `mise exec` provides). Optional
  arguments name another repository and verifying key: `verify-release.sh
  v2.0.0-rc.2 minodisk/handpic-update-test
  crates/app/packaging/test/update.pub` reported `STATUS=ok` on 2026-11-21,
  and `v1.10.0` of `minodisk/handpic`, a Tauri release, `STATUS=incomplete`.
- **Delete a release candidate.** `gh release delete v<version>
  --cleanup-tag --yes` removes the prerelease and its tag. Never delete a
  final release this way: installs pin its tag.

### `mise run app:watch` rebuilds and restarts the app (Inferred)

`mise run app:watch [-- <folder>]` runs the GPUI app under watchexec, which
the task's own `tools` installs next to rust, so the top-level `[tools]` (and
every CI job) stays without it. An edit under `crates/app`, `crates/core` or
`crates/engine` stops the app, rebuilds and relaunches it. Measured in
`docs/plans/_archived/20261115-native-watch/learnings.md`.

- Rule: the task watches the crate's inputs by name (`src`, `build.rs`,
  `Cargo.toml`, `../core`, `../engine`), not `.`, so `crates/app/target`,
  whose `.fingerprint` holds `.json`, never triggers a restart. Keep it that
  way when you edit the task.
- Rule: the watchexec line is the script's last line and runs with
  `--shell=none`, so mise's appended quoted argument (a folder path with
  spaces) reaches the app as one argument. A shell wrapper would re-split it.
- Rule: on Windows the restart relinks cleanly (no `os error 5`) because
  watchexec's job termination takes the app down together with `cargo`.
  `taskkill /F` on `watchexec.exe` leaves `cargo` and `handpic.exe`
  running. Pick processes by `Path` before killing them: the main checkout's own
  `handpic.exe` may be running.
- Pending manual check: Ctrl+C in the terminal stops watchexec, `cargo` and
  the app, and the window repaints after a restart.

## Engine

### The engine names no event and no `AppHandle` (Inferred)

`handpic-engine` knows no GUI, so it reports through callbacks (it served the
Tauri app and the app alike until the Tauri app was removed):
`watch::Watch::spawn` takes `on_changed`, and `watch::set` / `release_under`
take the `&Watch`; the tree's watcher is
`treewatch::TreeWatch::spawn(on_changed)`, `TreeWatch::set(dirs)` and
`treewatch::with_released(&TreeWatch, ..)`. The app owns what a report
becomes (the app sends it into the view's event channel). The trash
bodies (`trash::trash_listed`, `delete_listed`, `undo_run`, `redo_run`) take
the `&Runs` and a `progress(done, total)` callback; the app keeps the writer
flush, the scan guard, the progress report and the log line around them.
The folder operations' bodies take their state the same way:
`rename::rename_folder` and `datefolder::split_folder` take a
`rename::Context` (the `&Watch`, the `&TreeWatch`, the optional index and the
`&Runs`), `datefolder::plan_of` the optional index, and
`foldersidecars::delete_run` the index, the `&Runs` and a `progress(done,
total)` callback, as `foldersidecars::rewrite` does. None of them checks
for a scan or drains the sidecar writer: the Tauri app's wrappers flushed the
writer and held a `Scans` lock across the call, and the app guards the
call with its own gate.
The MCP companion's server (`mcp::Mcp`) takes a `Bridge` built over a send
callback and returns each switch's `McpState`. The server
spawns its tasks with a bare `tokio::spawn`, so `Mcp::switch` and `Mcp::stop`
must run inside a tokio runtime: the app's `mcp.rs` owns a current-thread
runtime on its one `mcp` thread, which runs the switch, waits for the quit's
signal and runs the stop. `Mcp::with_port` binds another port than
`MCP_PORT`, for the app's `--mcp-port`.

The CPU and disk sampler follows the same seam.
`perf::Sampler::start(on_sample)` takes an
`impl FnMut(Sample) -> Result<(), E: Display>` and has a separate `stop()`
(stopping needs no callback); the callback returns a `Result` so the warn-once
on a failed report stays in the engine (`perf sample report failed: ...`,
target `handpic_engine::perf`). The server's `log` lines sit under
`handpic_engine::mcp`, so grep for the engine target. Source:
`docs/plans/_archived/20261113-gpui-companions/learnings.md`, Steps 2 to 4.

- Rule: never add a GUI crate, an event name or an app handle to
  `crates/engine`; take a callback or the state itself.
- Rule: an engine body that changes a folder on disk leaves the scan check to
  its caller and says so in its doc comment; call it only with the caller's
  refusal of a running scan held across the call.

### The engine is compiled twice (Inferred)

`crates/engine` is a member of the root workspace, and `crates/app`, a
separate workspace with its own `Cargo.lock` and Rust 1.98.1, uses it by path,
as it does `handpic-core`. A dependency added to the engine also lands in
`crates/app/Cargo.lock`, and clippy lints the engine only from the root.

- Rule: an engine change needs both workspaces checked. `mise run ci` runs
  only `lint` and `test` of the root workspace, so it never covers
  `crates/app`. The agents run `mise run ci:local` locally
  (`tools/ci/local.sh`), which checks only the changed crates: for a
  `crates/core` or `crates/engine` change, clippy and the tests of that root
  package plus the app clippy; for a `crates/app` change (the app's
  identifier, `datadir`, `logger`, `settings` and the rest), the app clippy
  and tests. The PR's `test` and `app` jobs, on all three platforms, run
  the rest: the root workspace's other crates and the app tests an engine
  change can break. Neither full task is run locally as a gate.
- Rule: do not cap `ci:local` with `--jobs` and do not reach for the full root
  workspace to be safe. `cargo clippy -p` / `cargo test -p` of `handpic-core`,
  `handpic-engine` or `handpic-cli` compiled 161 / 317 / 169 crates against
  537 for the Tauri app's package, measured while it was in the workspace. The
  largest check's process-tree peak was 5.5 GB (the app test) and a cold
  `crates/core` change peaked at 3.3 GB, so the earlier out-of-memory kills
  came from the full root workspace, not from `ci:local`. The selected root
  packages go to one `cargo clippy` and one `cargo test` call (a `--package`
  each), because three separate cold calls for every check at once took 594 s,
  at the Bash tool's 600 s limit, against 382 s in one call. Cold wall times:
  Markdown only under 10 s, `crates/cli` 66 s,
  `crates/app` 154 s, `crates/engine` 199 s, `crates/core` 265 s. Source:
  `docs/plans/_archived/20261114-light-local-ci/learnings.md`, Step 1.
- Rule: `#[cfg(test)]` does not cross a crate boundary. When code moves into the
  engine, a helper the app's tests still use (for example `Index::row_flags`)
  must lose its `#[cfg(test)]` or be copied into the app's test module.
- Rule: the engine reads the app's updater keys with a relative
  `include_bytes!("../../app/packaging/...")` (`crates/engine/src/update.rs`).
  A crate rename or directory move therefore breaks it, and only the compiler
  sees that; a grep for `crates/<name>` does not. When renaming or moving a
  crate, also grep `(\.\./|\b)<old>/` and run clippy on the engine and the app.
  The `crates/native` to `crates/app` rename failed its first `ci:local` on this
  exact path. Source:
  [`learnings.md`](../plans/_archived/20261118-rename-app-crates/learnings.md), Step 2.
- Rule: `mise run app:check` runs clippy and tests but does not build
  `handpic.exe`. Run `cargo build` before launching the exe, or you run a
  stale binary. A still-running older `handpic.exe` locks
  `target/debug/handpic.exe` and `cargo build` fails with `os error 5`;
  rename the running exe aside (for example `handpic.stale-<pid>.exe`)
  instead of killing it, or ask the user to close it.
- Rule: `cargo` is not on the Git Bash `PATH` in a worktree session. Prepend
  `~/.cargo/bin` (`PATH="$HOME/.cargo/bin:$PATH" cargo test -p ...`) for a
  direct call; the mise tasks find it on their own. Source:
  `docs/plans/_archived/20261115-native-scrollbars/learnings.md`, Step 1.
- Rule: the app crate's binary target is `handpic`, not `handpic-app`
  (the package name); run its unit tests with `cargo test --bin handpic` in
  `crates/app`. To get a color's luminance in `crates/app`, call
  `Hsla::to_rgb()` (gpui-pre's `color.rs`) on the `gpui_kit` re-export; the
  method needs no import. Source:
  `docs/plans/_archived/20261118-preview-theme-background/learnings.md`, Step 1.
- Rule: a bare `cargo test` inside `crates/app` can pick a toolchain too old
  for `gpui-pre-macos` 0.3.12 (`result_option_map_or_default` is unstable
  there). Run the app's tests through `mise run ci:local`, or
  `mise x rust@1.98.1 -- cargo test --bin handpic`, which pins the app's
  toolchain. Source:
  `docs/plans/_archived/20261125-settings-under-title-bar/learnings.md`,
  Step 1.
- Rule: `crates/app/Cargo.lock` resolves its own versions (its `notify` is
  8.2.2, which the root lock matches since
  `docs/plans/_archived/20261126-notify-lock-align/plan.md`; the rest of
  `notify`'s subtree can still differ). `watch::set` (watch, replace, the debounce)
  works on 8.2.2: copying into and deleting from the open folder resynced it.
  The tree watcher's `unwatch` + `settle` returns promptly on 8.2.2 too (see
  "The tree follows the disk through one `TreeWatch`"), and so does the
  folder rename's release path (see "A folder rename releases the watches,
  then follows in the tree and reopens").

### The pieces both apps needed live in the engine (Inferred)

The folder-open sidecar pass (`sidecar::reconcile_of`, with `SidecarError`
and `MAX_SIDECAR_BYTES`), the meta pane's formatting (`exif::Metadata` and
`exif::metadata(name, &shot)`), the stored `labelNames` value
(`sidecar::label_names_setting` / `label_names_value`), the metadata and
preview reads (`exif::read_metadata`, `ondemand::read_preview`) and the stored
`mcpEnabled` value (`mcp::mcp_enabled_setting`) moved out of the Tauri
app's commands into the engine so the app could call them. The
app's view-only rule is `viewonly.rs`, a port of the Tauri
frontend's `viewonly.ts`. The folder tree's listing (`folders::roots(home)`,
`list(dir)`, `REVEAL_LABEL`, `is_hidden`) moved the same way; the home
directory is the caller's, and the reveal itself is the app's (GPUI's
`App::reveal_path`).

### The settings live in the app's own store, seeded once (Inferred)

`settings.rs` reads the app's settings store
(`handpic_engine::store`): one SQLite file, one `settings` table of
`key TEXT PRIMARY KEY` / `value TEXT NOT NULL` (JSON), WAL, the schema
version in `user_version`. It is `--settings <path>`, else
`settings.sqlite` in the `IDENTIFIER` folder under the config dir
(`datadir::settings_path`), next to where the Tauri app kept its
`settings.json`. The
values (`sidecarFormat`, `labelNames`, `shortcuts`, `autoAdvance`,
`sortOrder`, `filter`, `lastFolder`, `windowBounds`) go through the one `from_values` with the engine's
rules (`SidecarFormat::from_setting`, `sidecar::label_names_setting`,
`Keymap::from_overrides`, `autoAdvance` on unless a boolean says off,
`SortKey::from_setting`, the Tauri `parse_sort_order` with `recommended` read
as capture order and left stored as it is, `Stored::from_setting`, a string
`lastFolder`, `session::Geometry::from_setting`).

Only a new store (no schema yet: a missing file or a zero-byte one left by a
crash) is seeded, once, with every key of a Tauri `settings.json` verbatim,
the schema and the values committed in one transaction. The seed is
`--import-settings <path>` when given; else, only without `--settings`, the
first that exists of `<config dir>/<IDENTIFIER>/settings.json` and each
`<config dir>/<LEGACY_IDENTIFIERS[i]>/settings.json`; with `--settings` and no
`--import-settings` nothing is seeded and nothing under the config dir is
read. The JSON is only read, never written, and never read again once the
store exists, so the two apps' settings diverge from then on. The log names
the store and, for a new one, how many keys were seeded from which file (or
the paths tried).

The store is never deleted or recreated, unlike the index (a cache): a file
that is not a database, or one with a newer `user_version`, is an error left
as it was, and the app runs on the defaults (`Xmp`, the English names, the
default keymap, auto-advance on) with no store behind `Settings::writes`, so
nothing is written; the startup `settings:` line ends `store=none`.

- Rule: `Store::open` reads `user_version` before it sets `journal_mode = WAL`,
  so a store with a newer schema is refused before its header is touched
  (switching a rollback-journal database to WAL rewrites the header). The
  schema and the seed are then written in one `BEGIN IMMEDIATE` transaction
  that re-reads `user_version`, so two processes opening one new store cannot
  both seed it, and (`PRAGMA user_version` being transactional) a store is
  never found with the table but no version. A JSON file opened with
  `Connection::open` succeeds and fails only at the first query ("file is not
  a database"), leaving its bytes as they were; keep that order if the open
  is ever reworked.

Every write goes through `Settings::writes` (`settings::Writes`): a
`set(key, value)` or `delete(key)` is sent to one `settings-store` thread,
which applies them to the store in the order given, so two quick changes of
one key cannot land reversed, and never on the main thread. A failed write is
logged (`settings store: failed to write <key>`) and the in-memory value
stands, as in the Tauri app. `App::on_app_quit` drains them (`drain`, at most
`settings::DRAIN_TIMEOUT`) in a handler of its own, registered whether or not
the index opened (`quit: settings writes drained=…`). One thread with a
channel was taken over `viewed.rs`'s `Latest`: a `Latest` per key would need
one per key and a way to wait for them all on quit, where the channel's
barrier gives the drain for free. The clones of one `Writes` share the
queue and the thread behind one `Arc`; the last clone's drop closes the
queue and joins the thread (no bound: the thread only finishes the writes
already queued), so the store is closed when the drop returns and a second
open of the same file, which Windows refuses while the first is open,
succeeds at once.

- Rule: the shared `Writer` keeps its sender in an `Option` so its `Drop`
  can drop the sender before joining the thread; joining with the sender
  still alive hangs forever. A change to its ownership must keep that order.
  The settings tests repeated 20 times on Windows without a failure after the
  change
  (`docs/plans/_archived/20261121-todo-seven-small-fixes/learnings.md`, item 6).
- Rule: every debug or hand-check run passes `--settings` with a test store
  (a fresh `.sqlite` path in the test folder), and `--import-settings` with a
  test JSON when it needs seeded values, or it opens and seeds the user's real
  store; and `--log` with a file in the test folder, or it appends to the
  app's own `Handpic.log`.

### The folder open reads the sidecars before pass 1 (Measured)

`scan::list` lists with `read_listing(dir, Some(format))`, and
`Scan::prepare` runs `stat_sidecars` and `sidecar::reconcile_of` right after
`reconcile`, on the open's background task and before the scan thread starts,
as the Tauri `scan_folder` ordered them, logging `scan sidecars: dir=…
dirty=N changed=N in …ms`; a view-only folder skips it. The sidecars it could
not read go to the pane's notes as `<name>: <message>`. On a copied folder of
8 DNGs with XMP and `.dop` sidecars and a fresh index, the first open logged
`changed=8` and the second `changed=0`, under `"xmp"` and under `"both"`, and
no sidecar's mtime moved.

- Rule: `Index::entries` joins `files`, so on a fresh index it returns nothing
  until pass 1 has written the files' rows; the view reads the judgments at
  the open and again at pass 1's `Done`. The `ratings` rows exist right after
  `reconcile_of`, so a ratings-only read (`Index::rows_of`) would show the
  marks before pass 1, at the cost of the "entry seen" set Step 4's
  `label_known` follows.

### A judgment goes view, row, writer (Measured)

A judgment key on the strip's selection (none in a view-only folder) applies
`judgment::apply` to the view's map and the strip's cells at once, then on
`cx.background_spawn` runs `judgment::store`: `Index::set_rating` on the
writer connection, then `Writer::set`, the Tauri `set_rating`'s order, so a
crash between the two leaves the row dirty. A failure of either puts the
file's previous state back (while the folder is still open) and goes to the
pane's notes, as does the writer's `on_error` through `Event::SidecarError`,
one line per path. The writer is spawned in `Handpic::new` when the index
opened; `Scan::prepare` hands `reconcile_of`'s dirty rows to `Writer::set_now`,
so a judgment lost to a crash inside the 300 ms debounce is written on the
next open (checked on the real exe by seeding a dirty row into a test index:
the reopen logged `dirty=1` and the `.xmp` got `xmp:Rating="4"`).

The undo / redo hand check passed on Windows 11 on 2026-11-21 on a folder of
four ARWs: `Ctrl+Z` twice walked back with the notes `Undid
DSC-RX0_DSC00009.ARW` and `Undid DSC-RX0M2_sony_rx0_ii_40.arw`, each `.xmp`
following within a second; `Ctrl+Shift+Z` gave `Redid ...` and the stars
back; a new key then `Ctrl+Shift+Z` did nothing; after `q` and a new index
`Ctrl+Z` did nothing, and nothing in the JPEG folder either.

- Rule: the view keeps `seen` (the paths the index last returned an entry for,
  which make the label known) apart from `judgments`, and `touched` (the
  paths judged since the open), whose state a later `entries` read, such as
  the one at pass 1's `Done`, does not replace, as the Tauri `touched` set.
- Rule: undo and redo go through the same path. A judgment that writes
  records its files' states before it as one `history::Undo` entry (a batch
  over the selection's targets) and clears the redo history; `Ctrl+Z` /
  `Ctrl+Shift+Z` pop an entry, push the files' current states onto the other
  history, apply the popped states as a key would (view, row, writer) and
  move to the file of a one-file entry, selecting it alone when the focus
  moves (`selection::restore`); an entry of several files leaves the current
  file and notes `Undid N files` / `Redid N files`. Each entry has an id, so a failed store drops its file
  from the entry it belongs to (`Undo::forget`, the Tauri `forgetOnFail`).
  Another folder's open drops the judgment batches of both histories and
  keeps the trash runs; a re-listing keeps them and a step drops the files
  no longer listed.
- Rule: `label_known` for a store follows the Tauri `send` rule: the entry was
  seen, or (for an undo / redo, which has no command) the restored label
  differs from the current one. A `ClearAll` on a file whose entry was not seen
  still goes out when it changes nothing (`forceLabel`), so `judgment.rs` has
  `writes` next to `changed`; do not fold the two into one test. A failed
  undo / redo store also drops its file from the entry it pushed onto the
  other history (the Tauri `step` passed no `onFail` for that entry).
- Rule: `rejectRest` is not a `judgment::Command`: the selection neither
  widens nor narrows it. `burst::reject_rest` builds its changes, every other
  member of the current file's burst over `all` (a member the filter hides
  too) that is not a reject yet, flagged reject with its stars and label kept
  and its label known when `seen`; `judge_paths` records them as one entry
  (the redo history cleared), applies them, refilters once on the current
  file and stores each, with no auto-advance. Nothing happens in a view-only
  folder, with no file shown or outside a burst of two or more; its undo is
  the ordinary `step`.
- Seen (2026-11-22, Windows 11): under a Picked filter `shift+x` rejected the
  burst's hidden member, the current file stayed and nothing advanced, the
  undo and redo named it as hidden by the filter (`Undid _DSC1995.ARW
  (hidden by the filter)`), and on a singleton it did nothing (results in
  `../plans/_archived/20261122-windows-device-checks-4/learnings.md`).

### The selection is keyed by path and ends every `refilter` (Inferred)

`selection.rs` is `selection.ts` over the app's `judgment` types: a
`Selection { selected, anchor }` of listed paths, the current file always a
member. The view holds one; `set_selection` hands its paths to the strip
(`Strip::set_selected`, indices through the strip's `index_of`), which paints
a selected cell other than the current one as `.cell.selected` (the `--muted`
fill and a `--ring` border), so nothing maps paths per frame.

- Rule: `go` (paging, `home` / `end`, auto-advance, a one-file undo) selects
  the landed file alone only when the current file changed, as `main.ts`'s
  `move`; `focus_on` is the move without touching the selection, which a
  Ctrl / Shift click, `extendPrevious` / `extendNext` and the MCP `show`
  use after setting the selection themselves. A Ctrl click (Cmd on macOS;
  either modifier, as `strip.ts` reads `metaKey || ctrlKey`) toggles the cell
  and keeps the focus; a Shift click selects the range from the anchor and
  moves the focus to the clicked cell; a plain click selects it alone, also
  on the current cell. `selectAll` selects every shown file with the focus
  kept. None of them is refused in a view-only folder.
- Rule: every `refilter` that rebuilds `files` ends with `settle(.., false)`,
  that is `prune`, so a file a judgment or the filter hides leaves the
  selection; a new folder's open lands with `settle(.., true)`, the landed
  file alone. A re-listing goes through `refilter` and so prunes.
- Rule: a judgment's targets are `selection::targets` (the focused file alone
  while one or none is selected) and its changes `selection::judgments`: the
  value decided once from the current file, each file kept on the fields the
  command does not touch, the current file first, a file left out unless
  `judgment::writes` holds for it (its `seen` per path). Auto-advance needs
  exactly one target (`should_advance`'s `targets`).

### The writer drains in `App::on_app_quit` (Measured)

`Handpic::new` registers `App::on_app_quit` (the app-level one, holding an
`Arc<Writer>`, not the entity-level `Context::on_app_quit`: closing the window
can release the view before the quit handlers run) and calls
`writer.flush(DRAIN_TIMEOUT)` inside the handler itself, returning a ready
future. GPUI gives the returned futures only `SHUTDOWN_TIMEOUT` (200 ms), but
the handler is called synchronously from `App::shutdown`, which on Windows
runs once the message loop ends, after `cx.quit()` (`q`, `--auto`) and after
`on_window_closed`'s `cx.quit()` alike. An `--auto 1` run logged `quit: sidecar
writer drained=true` after its summary. On Windows 11 on 2026-11-21, `5` then
`q` at once (inside the debounce) logged `quit: judgment stores drained=true
sidecar writer drained=true` and the sidecar held `Rating="5"`; closing with
the title bar's close button gave the same line and value.

The other sidecar writes seen in that run (`xmp.json` unless named):

- `3` / `p` / `Ctrl+Alt+1` gave `xmp:Rating="3"`, `xmpDM:good="True"`,
  `photoshop:LabelColor="red"`, `xmp:Label="Red"`; `0` / `u` / `Ctrl+Alt+0`
  gave `Rating="0"`, no `good` and no label (`0` does not auto-advance).
- `1`, `2`, `4` within about 150 ms on the last file gave one mtime change
  and the final value.
- With `both`, the `.xmp` and the `.arw.dop` held `Rating = 3`,
  `ShouldProcess = 0`, `ColorLabel = "Red"` and random Uuids; with
  `rot.json`, `xmp:Label="Rot"`.
- `4` then a forced kill by PID within about 100 ms left no sidecar; the
  reopen on the same index logged `scan sidecars: ... dirty=1` and wrote
  `Rating="4"`.
- Auto-advance: `3` moves on and stays on the last file, `Ctrl+Alt+2` stays,
  the arrows wrap both ways. In a JPEG folder `3` does nothing and writes no
  file.

The same handler first stops the running scan, before the drains: the view
holds its scan in a `scan::Running` (an `Arc<Mutex<..>>` of the held scan and
the retired ones a re-listing is still stopping) whose clone the handler
keeps, since the view may be gone by then. `Running::stop_within(STOP_TIMEOUT)`
sets the cancel flag of the held scan and of every retired one and waits, one
deadline for them all, on a channel each scan thread drops its sender of when
it ends (`JoinHandle` has no timed join), logging
`quit: scan stopped=<bool> retired=<none|bool> in <ms>ms` (`stopped` the
held scan, true with none held; `retired` the retired ones, `none` with none
still stopping). Mid-pass 2 on 3,360
hardlinked ARW files (release, 3 pass 2 threads) the stop took 6-10 ms and
the process exited 47-140 ms after the quit, `scan faces: ... canceled=true`
logged, by `--auto 10` and by `CloseMainWindow` alike. `STOP_TIMEOUT` is 2 s,
as `DRAIN_TIMEOUT`, for a read a slow or network volume holds; past it the
process exits with the scan running, as before.

- Rule: a re-listing still stops the previous scan with the unbounded
  `Stopping::stop`: it must not reconcile while the old scan writes.
  `Running::retire` takes the scan out of the holder for it but keeps the
  scan's cancel flag and end signal there until that stop returns, so a quit
  during a re-listing cancels and waits for it too (the background task may
  not have reached its stop yet), and does not cut its write transaction
  within `STOP_TIMEOUT`.

### The MCP companion answers from the view, one channel hop away (Measured)

`mcp.rs`'s `Control` switches the server on `mcpEnabled`: at launch when the
settings store holds it true, and on every flip of the MCP page's
`Let MCP clients connect` (`Change::McpEnabled`, which `apply_setting` writes
to the store, clearing `mcp_pending` and `mcp_folder` on an off as the
destructive switch's off does). The first switch on builds a tokio
current-thread runtime on one named `mcp` thread over an `Mcp` (on
`MCP_PORT`, 41917, or `--mcp-port`; `0` binds a free port, which the state
reports) and registers the quit handler once; the thread stays for the
session, runs each switch's `Mcp::switch` in order, logs its `mcp: McpState`
line and sends the state as `scan::Event::McpState`, and on the quit
handler's command runs `Mcp::stop` (logged as `quit: MCP server
stopped=<bool> in <ms>ms`, at most `STOP_WAIT`). Off at launch, there
is no runtime, no thread and no server until the first switch on. The view
keeps a state only when its `enabled` is the switch in force (a late answer
of an earlier flip is dropped) and hands it with the port to the MCP page,
which shows the URL read-only and the state (`settingsmodal::mcp_status`:
stopped, starting, running or the bind error). A bind failure keeps the
switch and the stored value on, as the engine's rule has it, so the next
launch tries again; the app runs on. A reply to a call whose server stopped
finds no waiter in the `Bridge` and is dropped. The engine's `Bridge` send callback
only puts the request into the view's channel as `scan::Event::Mcp`; the
view's `mcp_request` builds a `companion::View` snapshot, `companion::answer`
returns the action or the refusal, the view applies it (`focus_on(Move::To(..))`
with the file selected alone for `show_photo` or with
`selection::selection_of` over the given paths for `select_photos`,
`toggle_zoom`, `judge_paths`, `perform` with a `companion::Key`,
`apply_menu`, `change_panels`, the overlays' toggles, `open_from_tree`,
`tree_toggle` or `folder_action`, `files_action`'s reveal, or
`apply_setting`) and replies through `Mcp::reply`.

`Mcp::new` lists every tool: the seven companion tools of the first
`#[tool_router]` block (`get_photo`, `get_preview`, `get_view`,
`select_photos`, `set_judgment`, `set_view`, `show_photo`) and the
`app_router` tools (`navigate`, `select_all`, `reject_rest`, `undo`, `redo`,
`set_sort`, `set_filter`, `set_panes`, `set_overlays`, `list_folders`,
`open_folder`, `tree_folder`, `reveal`, `get_status`, `wait_for_idle`,
`get_settings`, `set_settings`, `trash_photos`, `rewrite_sidecars`,
`delete_sidecars`, `sequence_timestamps`, `import_folder`,
`rename_by_capture_date`, `rename_folder`, `cancel_operation`, `clear_cache`,
`rename_file`), with `INSTRUCTIONS`, through a `ToolRouter` field the
`#[tool_handler(router = self.tools)]` handler reads. The removed Tauri app
listed only the seven, with instructions of their own; that mode went with the
AI features.

`screenshot` is listed only in a build with `crates/app`'s `screenshot` cargo
feature, where `mcp.rs` calls `with_screenshot`. Without the feature nothing
of it is compiled in and `tools/list` is the app's 34 tools. `mcp_request`
takes `screenshot` before `companion::answer`, so `READS`, `Busy` and the
coverage table never see it and it is answered with the settings, a dialog or
a rename open; the reply goes through the engine's `screenshot_reply`. Run it
with `mise run app:dev:screenshot`; a feature build is not the shipped binary,
so take no timing from it.

The app's tools run what their keys run: `key_down` and `mcp_request` both
call `perform` with a `companion::Key` (`Key::of_action` names the keymap
actions; Home and End are `First` and `Last`), so a key and its tool cannot
drift. `navigate` and `select_all` need a shown photo, `reject_rest` a current
photo in a burst of two or more outside a view-only folder, and `undo` /
`redo` a judgment batch on top of their history (`history::Top`), each
replying with the new `get_view`. A trash run on top is `Action::StepTrash`:
`perform` runs the key, so `step` hands it to `step_trash` (the gate, the
restore or the new run), and the reply is `get_status` at once, the
operation `running` or `held_for_scan`. Its redo moves files to the Trash
with no dialog, so it is refused unless `mcpAllowDestructive` is on (only recoverable operations run without a dialog); an undo or redo of one in
flight is refused.

The view-state tools set a state rather than toggle it, so a repeated call
changes nothing. `set_sort` and `set_filter` become the menu clicks
(`menu::Action`) that reach the asked state, which `apply_menu` applies, the
same tail a menu click runs: `sortOrder` and the remembered `filter` written
when they change, then `refilter` on `kept_anchor`, so the current file lands
where `filter::anchor_after_filter` puts it. A `set_filter` section given
replaces its checks, one left out keeps them, and `reset` is the menu's
`Reset` click first; the `Output` items need the folder's export answer and
an EXIF label one of the folder's groups (`get_view`'s `filter_offers`).
`set_sort` refuses a view-only
folder, whose Sort menu is disabled; `get_view.sort` there reports `capture`,
the order in force (`viewonly::sort_for`). `set_panes` goes through
`change_panels` (the tree's keyboard handed back from a hidden left pane, a
1:1 crop re-cut, `panels` written), and `set_overlays` flips only the `f`
(`toggle_marks`), `e` (`toggle_peaking`) and `d` (`toggle_perf`) overlays
and the grayscale that differ. The grayscale is a hold on `g` with no state
that outlives the key, so `set_overlays` latches it (`grayscale::Hold::set`):
on until a call sets it off or the user's own press of the key goes up; the
window's deactivation, the view's blur and a modifier going up, which end a
key's hold because its key-up may never arrive, leave a latch on. The pane widths and the strip height, set by a drag, have no
tool: they are pointer-only geometry, like the window's.

The folder tools act as the tree does. `list_folders` never reaches the view:
the engine answers it on the blocking pool with `folders::roots`, from the
home directory the app gives `Mcp::with_home` (`dirs::home_dir()`, the
tree's), or `folders::list` of one absolute folder (its `raw_count` as
`photo_count`). `open_folder` takes an absolute path to a folder, checked in
the engine (a file is refused, not opened by its parent as a drop is: a
client has `list_folders`), and runs `open_from_tree`, so an open is deferred
while a run is in flight (`deferred_open`, `get_status`'s `opening`) and
refused with `rename::RENAMING` while that folder is being renamed
(`refuse_renaming`, its tree note shown as for a click). It replies at once
with `get_status`, the listing under way, since a slow folder's listing
outlasts `BRIDGE_TIMEOUT`; the folder the tree shows open is not opened again,
as a click on its row only expands or collapses it. `tree_folder` finds the drawn row whose
path `tree::relation` calls the same (a `\\?\` prefix dropped, case ignored
on Windows and macOS), toggles it with `tree_toggle` when it is not already
as asked, or runs the tree menu's `Refresh`, `Expand All` or `Collapse All`
through `folder_action` (the last two refused on a root, whose menu lacks
them; `Collapse All` keeps the folder itself as it is); it replies with the
row's path and expansion, also when nothing changed, before any listing it
started ends. `reveal` runs `folder_action`'s `RevealFolder` (a root opens)
for any existing folder and `files_action`'s `RevealFile` for a file the strip
shows.

`get_status` is `companion::Status`, built by `status_of`: the open folder,
the deferred open, a listing (`Resync::busy` with no scan pass and no run in
flight), the scan pass as the status line names it (`scan::Status::pass`,
`scanning` or `analyzing` with done / total), a run in flight
(`IdleGate::in_flight`), the status line's text (`status()`), and the meta
pane's notes and error lines, which the pane draws from the same
`pane_notes` / `pane_errors`, and the `operation` (`operation_of`): the
trash dialog asking (`awaiting_confirmation`, with its line), an operation
the gate holds (`held_for_scan`, by its label), the run, undo or redo out
(`running`, its label kept in `running_op`, with the `trashing` done / total),
else the last one to come back (`last_op`, `finished` or `failed` with the
step note `record_op` copies; a held run, undo or redo dropped before it ran is
recorded `failed` by `record_dropped` with a `dropped: <reason>` message and
moved nothing, from its four drop sites: a folder open's `gate.discard`, a
dialog open when `finished` drains the gate, an undo or redo `finished` could
not run, and `gate_request`'s replacement of a held one by a later request). `idle` is none of them under way; a finished
or failed operation stays reported and does not hold `idle` off.

`trash_photos` is the strip menu's `Move to Trash…` / `Delete Permanently…`
on the given photos (each one the strip shows) or the selection's targets,
also in a view-only folder, whose menu keeps both items. It is refused while
a run, delete or redo is out (`trashing`, which no dialog blocks for an MCP
run), a folder operation runs, or no folder is open. The confirmation is
D1's: without `confirm`, a move while `mcpAllowDestructive` is on is
`Action::Preview(Op)`, which the view keeps as `companion::Pending` under the
next `mcp_next_id` and answers with the dialog's text, the paths and the id;
anything else is `Action::Ask(Op)`, `confirm_trash` opening the app's own
dialog, answered with `awaiting_confirmation`, so a delete only ever runs on
the user's single click. A call with that `id` and `confirm: true` is
`Action::Run(Op)`, refused for a delete, with `mcpAllowDestructive` off, for
an unknown id and once a previewed photo is no longer shown or the folder
changed; it forgets the `Pending` and calls `request_run`, the tail of
`confirm_run` (the gate, the scan canceled when held, the writer drain, the
recorded run and the undo entry) with `own` false, so no dialog opens and
the flow stays idle, as for a held run drained later: the backdrop and the
key lock of a running dialog are not drawn for a run the user never saw ask.
It replies with `get_status` at once. `Op`'s `tool` and `content` name
each operation's tool and what its dialog says.

`rewrite_sidecars`, `delete_sidecars` and `sequence_timestamps` are the
folder tree menu's `Rewrite Sidecars from Index…`, `Delete Sidecars…` and
`Sequence JPEG Timestamps…` on an absolute folder (checked in the engine),
`Op::Folder(FolderOp, Option<Dialog>)`. They are refused while another
folder flow (`FolderOps::busy`) or an MCP-confirmed trash run is under way.
A sidecar start without `confirm` is refused with `HELD_REFUSAL` while the
gate holds an operation, as the gate would drop it (a sequence start does not
touch the gate). Without `confirm` the call is `Action::Folder`: the view calls `folder_op`,
the menu item's own entry, so the sidecar request goes through the gate
(held for the scan's end without canceling it) and the preview is read as
for a click; the reply is `get_status` at once (`companion::started`, the
operation `previewing` or `held_for_scan`), since a sequencing preview of a
large folder outlasts `BRIDGE_TIMEOUT`. While `mcpAllowDestructive` is off
the dialog opens on the preview (`show_folder_dialog`), and `get_status`'s
`operation` is `awaiting_confirmation` with the dialog's content in
`dialog` (`companion::dialog_content`: the title, lines, rows, lists, notes,
errors, reason, run label and whether the run is enabled). While it is on,
the view marks the flow `mcp_folder` (cleared by `folder_op`, so the menu's
next click opens a dialog again, and by switching the grant off); when its
preview comes back, `mcp_previewed` ends the flow and keeps the same
`Dialog` as the `Pending` (`keep_pending`), which `get_status`'s `pending`
reports with its id once `wait_for_idle` returns. A call with that `id` and
`confirm: true` is `Action::Run`, refused when the dialog's run would be
disabled (its `reason`), and runs through `run_confirmed`: the flow taken to
its previewed state with no dialog, then `run_sidecars` / `run_sequence`, so
a sidecar run started while a listing or scan is under way is refused with
`SCAN_RUNNING` plus a pointer to `wait_for_idle` (the `Pending` is put back, so
the same id confirms again once idle), and a Delete Sidecars run is recorded for `undo` as the
menu's is. A sidecar `Pending` goes when the sidecar format switches (its
counts are of the old format); a folder operation's `Pending` survives a
folder open, a trash one does not. `get_status` reports the flows:
`previewing` while a preview is read with no dialog, `running` with the
sequencing run's done / total (`running_op` set in `run_sidecars` /
`run_sequence`, the sequencing run read from the flow when an overlapping
trash run took the slot, its result recorded either way), then `finished` with the status line or `failed`
(`record_failed`: a failed preview, the `SCAN_RUNNING` refusal, a held
sidecar request dropped).

`import_folder` and `rename_by_capture_date` are the tree menu's `Import…`
and `Rename by Capture Date…` through the same `Action::Folder` /
`run_confirmed` path. An import is refused up front while the import root or
pattern is unusable (`import::refusal`, with a pointer to `set_settings`).
Its `FolderOp::Import` carries the id of the plan its preview read
(`ImportPlan`): `import_previewed` keeps the plan and, under the
`mcp_folder` mark, ends the flow and keeps the dialog as the `Pending` with
that id, and the confirm runs only while the kept plan is still that one (a
preview read since, by the menu or a client, refuses it: "preview again").
A change of the import root or pattern drops an import `Pending`. The
confirmed run is `run_import` with no dialog shown: its dialog is put aside
(`aside`, running and cancelable), so the tree's `Import…` brings it back as
for a run the user started, and `import_started` keeps it rather than taking
the empty dialog slot; the run still opens its first destination folder, and
the OS notification still fires only while the window is not active.
`get_status` reports it `running` with the copied files' done / total
(`import_progress`) and `finished` with the pane's result line
(`import::done_status`); `cancel_operation` stops it with its dialog shown or
aside (`dismiss_folder_dialog` on `import_running`, the Cancel's flag).
`Rename by Capture Date…` names the folder by the tree's spelling when the
tree draws it (its rename ends in `folder_renamed`, which renames the tree's
row), is refused with `SCAN_RUNNING` while a listing, scan or run is under
way (at the start as at the confirm, the `Pending` kept for the same id),
cannot be canceled while it runs, and reports `finished` with the split's
status line or the new path.

`wait_for_idle` after a confirmed import can return idle with the destination
folder open but not yet showing the copies (`no RAW files in ...`): the
watcher's re-list shows them about a second later, and a menu import has the
same race (`import_ran` re-lists only on `resync.changed()`). A client that
needs the copies reads `get_view` again, or calls `wait_for_idle` once more,
before trusting the listing.

`rename_folder` is the tree's inline `Rename…` with the name given: the
engine checks the folder and the name with `rename::folder_target`
(`check_name`, a root, a name already taken), the companion refuses an
unchanged name (`rename::confirm_name`) and a rename while an import runs
(`RENAME_REFUSED`). While `mcpAllowDestructive` is on it is an
`Op::Rename` previewed as a `Pending`, whose confirm (refused while the gate
holds an operation, which `gate_request` would drop) runs `request_rename`,
the inline edit's own tail: run at once, or held for the scan's end with the
row's pending name, then `run_rename` and `folder_renamed`, so the open
folder follows its new path. While it is off, the equivalent of a dialog is
the inline editor itself: the folder must be a row the tree draws, and
`ask_rename` opens the editor on it with the name filled in and selected
(`start_rename`'s `filled`, deferred to reach the window), the reply
`awaiting_confirmation`; the user's Enter renames, Escape cancels, and a lost focus (a click
elsewhere in the active window) or another edit starting cancels it too
(`InlineRename::asked`, `implicit`), so only Enter confirms. While the
editor is live, `get_status` reports `Rename…` `awaiting_confirmation` (and
every other changing call is refused as for any inline edit), then
`held_for_scan`, `running` and `finished` with the new path or `failed`.

`cancel_operation` passes the busy guard (`companion::CANCEL`): it cancels
what the user's Cancel would (`cancel_of`): the trash dialog asking
(`dismiss_trash`), a folder dialog asking or a sequencing or import run, with
its dialog or without one (`dismiss_folder_dialog`, whose sequence and import
arms set the run's cancel flag); else it forgets the `Pending`; else it is refused naming
the operation under way that cannot be canceled (a run to the Trash, a
sidecar run, a held request, a preview being read), or with nothing to
cancel. `wait_for_idle` replies
at once when idle; otherwise the view keeps the call's id in `idle_waiters`
and answers it from an `observe_self` callback (registered from a task, once
the entity exists), which runs after every `cx.notify()` and replies to every
waiter once `Status::idle`, or from a timer at `timeout_ms` with the status as
it is. The engine caps the wait at 55 s (`IDLE_WAIT_MAX`, 30 s by default)
and waits `BRIDGE_TIMEOUT` longer than that through `Bridge::call_within`.

The settings tools answer from `Settings` and change it the settings
modal's way. `get_settings` reports what the modal shows: the sidecar format,
the label names per color with the Lightroom language whose preset they are
(`settingsmodal::preset_of`, null for edited names) and the presets, auto-advance,
every keymap action with its `shortcut_label` and keys, the import root (null
while unset) and pattern, the theme with the names `theme::names` offers per
mode, and `mcpAllowDestructive`. `set_settings` turns a call into the modal's
own `settingsview::Change` values through its rules (`format_change`,
`label_names_of` / `label_change`, `settingsmodal::edit`, `pattern_change`,
`mode_change`, `theme_change`), and the view applies them with
`apply_setting`, the method the modal's `subscribe_in` callback calls too, so
a format switch drains, persists and re-lists as `switch_format` does, and a
theme applies at once (under the window's appearance, read through the stored
`AnyWindowHandle`, the app's when the window is borrowed). A field left out is
kept, a value in force changes nothing (no action, and the reply is still
`get_settings`), and one refused field refuses the whole call: label names
while the format, after the call's own switch, writes no XMP (the modal hides
them), `label_names` with `label_language`, a key the keymap does not spell
as `keys::key_name` names a keystroke (one key after the modifiers in their
order), and an import root that is not an absolute folder (checked in the
engine). `set_settings` is refused while a folder operation runs
(`FolderOps::busy`), as `key_down` keeps the modal shut then.
`mcpAllowDestructive` is never set over MCP: `SettingsArgs` has no such field
(`deny_unknown_fields`), and the companion refuses it should it arrive.
`mcpEnabled` has no tool either (`SettingsArgs` has no such field; a test
pins the refusal): it is the user's switch on the MCP page, and a client
would be switching off its own connection.

Every request but the reads (`get_view`, `get_status`, `wait_for_idle`,
`get_settings`) is refused while `key_down` drops keys (`companion::Busy`): the settings modal
open, a Move to Trash or Delete Permanently flow under way (`trash.busy()`), a
folder operation's dialog open, the Clear Cache dialog asking with the
settings closed (an MCP call opened it; `key_down` drops keys then too), or an inline rename live
(`FolderOps::editing`, not `rename_owns_keys`, which also disarms the slow
click). `get_view` still answers, so `get_photo` and `get_preview`, which
resolve their path through it, keep working. `get_photo`, `get_preview` and
`list_folders` never reach the view: the engine reads them on tokio's
blocking pool, the photo reads against the index's read-only connection,
outside `ReadSlots`, as in the Tauri app.

`clear_cache` is the Cache page's `Clear Cache`, confirmed as D1 decides
(`Op::ClearCache`, refused while a clear is held, asked or running,
`ClearFlow::busy`, and while a folder operation runs, which keeps the
settings shut). While `mcpAllowDestructive` is off it presses the button
(`press_clear`): the confirmation opens over the main window (the settings
stay closed), or once the listing or scan under way ends. While it is on, a
preview with the dialog's text and an id, then a confirm that runs
`ClearFlow::confirmed`: the clear at once, or held for the idle point and
run there without asking (`confirmed_clear`). `get_status` reports it
(`awaiting_confirmation` with the dialog, `held_for_scan`, `running`, then
`finished` "Cleared the index cache" or `failed`; a held clear dropped by the
settings closing or by `cancel_operation` is `failed`, `dropped: ...`), and
`cancel_operation` presses the dialog's Cancel or drops a held clear.

Every user-facing action has a tool, a tool refused until the feature lands,
or a documented reason for none, in `companion.rs`'s coverage table (the
`coverage` module, compiled for its tests, which keep it complete, so the
app carries no unused table): `KEYMAP`, every keymap action in `DEFAULTS` order, which a
test compares with `Keymap::defaults()`; `context_coverage`,
`menu_coverage`, `folder_op_coverage` and `page_coverage`, exhaustive
`match`es over `context::Action`, `menu::Action` (with each `Toggle`),
`FolderOp` and `settingsmodal::Tab`, so a new variant does not compile
without its entry; and `OTHER`, the rest. A test asks the companion for every
tool the table names and fails on `unknown request`.

The features the Tauri app had and the app does not have yet keep
their tools (D6), refused with one message, `not_in_app`'s "`<feature>` is
not implemented in Handpic yet", which a test checks for each: file
rename (`rename_file`). Compare is not among them: on the user's decision
(2026-11-23) it left the keymap and the MCP surface, so `set_view` takes
only `normal` and `zoom` and refuses `compare` as an unknown mode.
`rename_file` is refused before the busy guard (`UNPORTED_TOOLS`). When a feature lands, its tool or
value is implemented in place and its table entry becomes a `Tool`.

Excluded, each with its reason (the table's `Excluded` entries):

- Quitting (`q`): a client would end the app the user is working in, and its
  own connection.
- Opening the Sort, Filter and right-click menus: a menu is a way to an
  action, and each item has its tool.
- Opening the settings modal: `get_settings` and `set_settings` read and
  change what it shows. Its MCP page, `mcpEnabled` and `mcpAllowDestructive`,
  is the user's.
- The tree's keyboard cursor and type-ahead: a way to a row, which the folder
  tools name by path.
- Strip scrolling, the pane widths, the strip height and the window's
  geometry: pointer-only geometry; `show_photo` brings a photo into view.
- The OS folder pickers (`open`, the Import page's `Choose…`) and drag and
  drop: replaced by the path arguments of `open_folder` and `set_settings`.
- `mcpEnabled`: the user's switch on the MCP page, and a client would cut
  its own connection.
- The clipboard copies (`Copy Path`, `Copy File Name`, `Copy Folder Name`):
  a client has the paths already, and a tool would write the user's
  clipboard unseen.

- Rule: a test run of the folder tools uses a test folder outside any hidden
  folder: the tree skips hidden folders (`AppData`, a dot folder), so a folder
  under one is opened but never drawn, and `tree_folder` cannot find its row.

`set_judgment` goes the judgment keys' way: `companion.rs` merges the given
fields over each photo's own state (an omitted field kept, a rating of 0 and a
null label clearing), keeps only the photos it changes, with the label known
when the entry was seen or the call changed the label (the Tauri `send`
rule), and `judge_paths` records them as one `history::Undo` entry, marks them
`touched`, applies them, refilters once with the current file kept, and stores
each under that entry's id. There is no auto-advance, and a view-only folder
refuses with `viewonly.ts`'s message. The reply lists every given photo's
resulting stars, flag and label, the unchanged ones too. The tool's English
color names are what `judgment::State.label` holds, so no mapping is needed.
A `curl` check on test copies found the stars, the pick and the label in each
`.xmp` and the `ratings` row after `--auto` quit.

`selected` is the selection's targets in strip order (`selection::targets`:
the current file alone when one or none is selected), which a `set_judgment`
without `paths` judges, as a judgment key does, and `current.eyes` comes from the file's stored `Facts.focus.eyes` once pass 2
has stored an eye state, else from the `f` overlay's on-demand judgment
(`View.eyes`), and is absent until one of the two has judged the file. The stored row cannot tell "pass 2 has not reached the file" from "pass 2 meshed no face" (both are a default `StoredEyes`), so a test of the absent case tells them apart only by whether the preview's sharpness score is set. `burst` is the Tauri `burstOf` over the view's `bursts` (empty for a
burst of one, else every member in position order, a hidden one with
`visible: false`); each frame's `sharpness` is `Facts.sharpness`, set from the
entries read and patched by pass 2's `ready` files (`apply_face_ready`), as
the Tauri `sharpness` map was filled from `folder_entries` and patched by
`faces-progress` (`applySharpnessReady`). Each frame also carries
`eye_sharpness`, its `Facts.focus.eyes.full_eye_sharp` (null without it), and
`rank`, `burst::burst_ranks` over every member, the hidden ones included:
the frames with a `full_eye_sharp` by it, higher first, then the frames with
only a `sharpness` by that, higher first, the two scales never compared,
ties in capture order, a frame with neither score null. Pass 2 patches both
scores, so the ranks move until it is over and a client wanting settled
ranks calls `wait_for_idle` first, as for `photo_tier`. It is a rank by eye
sharpness, not a best-of-burst pick (see `shared-crates.md` on what the
full-size eye measure says).

- Rule: a test run with MCP on passes `--mcp-port` with a free port other
  than 41917 (an installed Handpic or an earlier Tauri build may hold it), seeds `"mcpEnabled": true`
  through `--import-settings` into a fresh `--settings` store, and passes
  `--log` with a file in the test folder, next to `--index`.
- Rule: a real-device run that clicks the folder tree checks the target row
  before each click. Test folders are siblings with similar names, so a
  mis-positioned click opens the folder menu of the neighbour. Verify the
  menu's folder (the dialog's `Source:` line or the status line) before
  pressing `Run`, and start a long or destructive run through the MCP tool
  that names the path where the flow allows it. Mis-clicks cost two runs of
  the notification check on 2026-11-22
  (`../plans/_archived/20261122-windows-device-checks-5/learnings.md`).
- Rule: a real-device run quits its instance by key only after `Escape`. A
  click on a tree row gives the tree the keyboard, so `q` goes to the tree
  and the instance stays up; `Escape` hands the keyboard back, then `q` quits.
  An MCP `open_folder` does not move the keyboard. Confirm the previous PID is
  gone before launching the next instance on the same `--mcp-port`: until it
  exits, the new one logs `could not listen on 127.0.0.1:<port>` (`os error
  10048`) and runs without MCP.
- Rule: a folder open prints no `page` line (only a key press is timed), so
  judge a folder switch from burst captures, comparing each frame's preview
  region with the first and the last frame. Take the burst in the capturing
  process and start the action from a second process (a delayed Python MCP
  call); a burst through PowerShell `Start-Job` starts about a second late.
  A release build runs the launch update check (`--auto` is off), a network
  request that offers the latest release to the `0.0.0` build
  (`../plans/_archived/20261122-windows-device-checks-prefetch-cache/learnings.md`).
- Rule: on Git Bash, give `curl` the JSON body on stdin
  (`--data-binary @- <<< "$body"`); a `--data` argument with `\` escapes
  reaches the server mangled. The session id is the `Mcp-Session-Id` header of
  the `initialize` reply, and `set_judgment` paths are matched as the index
  keys them (`\\?\`-prefixed on Windows), not canonicalized.
- Rule: Git Bash also mangles `\\` in a JSON argument built in Python (even
  inside single quotes), and a large Python script in a heredoc fails with
  "unexpected EOF". Write the test client with the Write tool, run the file,
  and have it expand placeholders (`%S%` for the JSON-escaped test folder)
  itself. Python run with `-I` ignores `PYTHONIOENCODING`; add `-X utf8` to
  print Japanese preset names on a cp932 console.
  Never run `python -I -` (or `python -`) without real stdin, for example
  after an empty heredoc: it starts the REPL with no console, loops on
  `WinError 123` under Git Bash and spins a core until killed. Always write
  the script to a file. Without Pillow on the host, zoom crops of a capture
  need a small pure-Python PNG reader / writer (`zlib`, filters undone by
  hand).
- Rule: stop a test instance of `handpic.exe` by the PID whose command
  line names the scratch folder. `taskkill /IM handpic.exe` is refused by
  the permission classifier because it could kill the user's own instance.

### A folder change re-lists, never mid-scan (Measured)

`main.rs` holds one `watch::Watch` whose callback sends `Event::Changed` into
the scan's channel; each listing calls `watch::set` before `read_listing`, so a
change during the listing is not missed. `resync.rs` defers a change that
arrives while a listing or its scan runs to that scan's end, one pending at
most, as the Tauri frontend's `resyncPending`: a re-listing mid-scan would
cancel and restart it. A re-listing carries the current file by path, the
strip's offset (`carried_offset`) and the loaded thumbnails
(`carried_indices`), bumps a listing counter that drops the previous listing's
preview reads, and always reads the current preview again.

- Rule: a file still being copied in changes size between events, so each
  debounced event re-extracts it (`todo=1 removed=1`) until the copy ends;
  that is expected, not a loop.

### The listing and the shown list are two lists, rebuilt by one `refilter` (Measured)

The view keeps `all` (the listing in natural order narrowed by `--size` /
`--max`, the Tauri `allFiles`) apart from `files` (`all` in the sort in force,
later also filtered), which the pager, the strip, the thumbnails, the hot
list, `current_path` and the status bar's position index into.
The order reads `facts` (each listed file's orientation, capture time, subsec
and `Focus`, keyed by the listed path) and the judgments, both from the one
`Index::entries` read at the open and at pass 1's `Done`. A view-only folder
is in capture order whatever the key (`viewonly::sort_for`), the key kept for
the next RAW folder. On a fresh index the open has no facts, so a capture
order starts as name order and settles at pass 1's `Done`, the current file
kept: an `--auto 5` run on 6 copied ARWs under `"capture"` paged in capture
order from there, `"rating"` put the rated files first and the rejected one
last, and a JPEG folder under `"name"` paged in capture order.

- Rule: only the open and `refilter` rebuild `files`. `refilter` orders `all`,
  keeps what `filter::passes` lets through (the view's judgment, `facts` and
  `mark::af_eye_state` of each path), returns early when the list is
  unchanged (`filter_list_changed`, forced by a re-listing), else carries the
  pager, the strip and the loaded thumbnails onto the new list (`carry`),
  landing on `anchor_after_filter`'s file over the ordered `all`: the anchor
  when it passes, else the next passing file after it, else the last before
  it. It runs after a judgment, an undo / redo, a failed store's revert, pass
  1's `Done` and a re-listing; it never runs per page. An empty result clears
  the image and the pane's file and shows `FILTERED_TEXT` on the status line
  and in the pane.
- Rule: a re-listing passes the list shown before it (the old `files`, not
  the old `all`) as `previous`: an anchor gone from the listing gives way to
  its next surviving passing neighbour in that list, else its previous one.
  This replaced `resync::carried_current`, the same rule with every file
  passing. The open resumes at `anchor_after_filter`'s file for the
  remembered `last_viewed`, so a remembered file the filter hides resumes at
  its passing neighbour.
- Rule: undo / redo's `listed` is over `all`, not `files` (the Tauri
  `allFiles.includes`), so a judgment on a hidden file is still undone. A
  one-file step anchors on its file when it passes, else on the file shown
  before it, and notes `Undid <name> (hidden by the filter)` when the file is
  not in `files`. Auto-advance after a judgment needs the judged file to be
  still current (`judgment::should_advance`): a judgment that filtered it out
  already moved the cursor.
- Measured: with `{"filter":{"flags":["untagged"]}}` an `--auto 6` run on 6
  copied ARWs (one rejected in its XMP) cycled over the 5 others; untagged +
  0 stars + `none` cycled over the 3 unjudged ones; `stars: [3]` (no 3-star
  file) paged nothing; a RAW copied in mid-run re-listed with the current file
  kept and the new file paged in its natord place.
- Rule: a rebuild bumps `order`, which drops a preview read still out for an
  index into the old `files` (the pager is replaced, so its in-flight flag is
  gone), and `refilter` re-requests the current preview whenever the image on
  screen is not the current file's. `listing` still guards the index reads,
  so a judgment's reorder does not drop the entries read of pass 1's `Done`.
- Rule: a judgment's, a failed store's revert and a step's `refilter` scroll the current file into view
  (the Tauri `keepScroll` false); `Done` and a re-listing keep the same files in
  view (`carried_offset`).
- Measured: `order_files` over 1,000 paths takes 0.4-0.5 ms in a release
  test build and 13-17 ms in the debug one (natord dominates the name order).
- Rule: pass 2's `FacesProgress` patches each written file's `Focus` in
  `facts` (`filter::apply_face_ready`, the Tauri `applyFaceReady`: a file
  without facts or without a focus is left alone) and refilters with the
  current file as anchor and the scroll kept only while a `Face focus` or
  `Eyes` item is checked; `FacesDone` reads nothing. On a fresh index under
  `{"filter":{"candidates":["good"]}}` the open's list and pass 1's are empty
  (no pass 2 values yet, so every file is `Neutral`) and the 3 good frames of 8
  came in from `FacesProgress` alone; `eyes: ["open"]` paged the 4 open-eyed
  frames.
- Rule: the `Output` section's answer is read on the open's (and the
  re-listing's) background task right after `scan::list`, before
  `Scan::prepare` (`scan::outputs` over `output::outputs_in`, at most two
  `read_dir`s), never on the main thread, and handed to `opened` with the
  listing it is about. No export folder or a read error is no answer:
  `FilterState::set_output_known(false)` clears the `outputs` checks, and an
  error is logged as `output filter: …` and kept in the pane's notes as
  `Output filter: …` until a read succeeds. The section is never stored.
- Measured: `output: dir=… found=N in …ms` (`found=none` without an answer)
  logged 0 ms on 8 copied ARWs with a 3-file `Output\`; renaming `Output\`
  away and back re-listed twice, `found=none` then `found=3`.
- Rule: the EXIF sections' groups (`exif::groups_of`, per group its labels in
  display order) are built from the same `Index::entries` rows on the same
  background task as `facts` (`entries_of`), in the index's order so a label
  two files share takes the last one's order as in Tauri, never per page.
  `set_entries` stores them and prunes the EXIF checks against them
  (`FilterState::prune_exif`) before any `refilter` that follows, so a check
  whose label left the folder cannot empty the strip. Another folder's open
  clears the EXIF checks (`clear_exif`), a re-listing only prunes; they are
  never stored. `passes` reads a file's EXIF from its `facts`
  (`exif::exif_key`), a file without one failing a non-empty group.
- Measured: `groups_of` over 5,000 synthetic entries (six groups, a few dozen
  labels) took 5.8 ms in a debug test build.
- Rule: the status bar's position is the pager's current index over `files`
  (`pager::position`, the Tauri `positionEl`), not the shown file's index, so
  a refilter that hides the file on screen (`p` under `untagged`) moves it at
  once; the name still names the file on screen until the next preview
  arrives. The shown file's index becomes `None` when a refilter hides it
  (`filter::shown_after`): a stale index would name the file then at that
  place in the next `refilter`, which could skip the re-request of a preview
  its `order` bump had just dropped.
- Rule: `self.shown`'s index is remapped only by a re-listing's `refilter`
  (`filter::shown_after`). A fresh folder open does not run it, so `files` is
  already the new folder's while the previous folder's image stays on screen
  until the first preview arrives, and `files[at]` would name another file.
  The open path therefore sets the index to `None`: the pane shows the old name
  with no index rows in that window, and `pager.finish` treats the old image as
  not the current file's. Any new reader of the shown file's index must handle
  `None` on an open.
- Rule: `toFixed(n)` and Rust's `{:.n}` differ on an exact decimal tie (`0.0625`
  gives `0.063` in JavaScript and `0.062` in Rust, which rounds half to even).
  The port of the meta pane's number formatting keeps the divergence, because
  only exact dyadic ties differ and a measured score practically never is one;
  `an_exact_tie_rounds_to_even_unlike_to_fixed` pins it. `Math.round` is
  `(x + 0.5).floor()`, so `-5.5` gives `-5` and `-0.5` gives `0`.
- Rule: the meta pane's `Analysis` group names the full-size eye score
  `Eye focus`, the one eye-focus measure since the full-eye-sharp-only plan
  (`docs/plans/20261115-full-eye-sharp-only/plan.md`). The Tauri pane named
  it `Eyes at full size`.
- Rule: a `refilter` that empties `files` keeps its anchor
  (`filter::KeptAnchor`), a non-empty one or another folder's open drops it.
  A menu change, and only a menu change, anchors on it while no file is
  current, so `Reset` or unchecking the item lands on the file shown before
  the list emptied, else on its `anchor_after_filter` neighbour, else on the
  first file. This is an improvement of the app: the Tauri `refilter` landed on the
  first file there.
- Seen (2026-11-22, Windows 11): with the sort on rating, `3` on an unrated
  file moved it at once to its new place and auto-advance went to its new
  right neighbour. Under an Untagged filter `p` hid the file and landed on the
  next shown one with no second move, `Ctrl+Z` brought it back, and the status
  position went from `3 / 5 (6)` to `3 / 4 (6)` in the first captured frame
  (about 30 ms). `x` on a 3-star file under a 3-star filter kept it shown. A
  filter that hid every file and then `Reset` returned to the file shown
  before. Deleting the file behind a checked EXIF item dropped the check and
  the item. Not run: the pass-2 race of `shown_after`
  (`../plans/_archived/20261122-windows-device-checks-4/learnings.md`).
- Seen (2026-11-23, Windows 11, a debug build with `--features screenshot`
  driven through the MCP companion, the state read with `get_view` and the
  drawn frame with `screenshot`): `set_judgment` pick on `_DSC1616` of the
  fixture's `menu\faces`, then `set_filter` `Picked`, narrowed the strip to
  that one cell (`1 / 1 (8)`). On a 40-file folder longer than the strip,
  `set_sort` rating kept the current file current (`32 / 40`) and its
  outlined cell in view, and so did the switch back to name with a 3-star
  file current (`36 / 40`). Under `untagged` a `set_judgment` pick of the
  current file (the judgment keys' view, row and writer path, not the `p`
  key itself) landed on the next file, and 1.2 s later the meta pane's name
  heading was that file's next to `1 / 4 (6)`, three times in a row. From a
  filter that hid every file (`Picked` and `Red`), unchecking the two
  items through `set_filter` (the menu's toggles through `apply_menu`, not
  its `Reset`) came back to the file shown before, `DSC-RX0M2` at `3 / 6`,
  not the first. Six picks under `untagged` plus `Face focus` `Neutral` on
  a fresh index while pass 2 ran (40 ARW, batches landing between the
  presses) each ended with the meta pane, the image and the outlined cell
  on `get_view`'s current file: the `shown_after` race did not show in six
  presses
  (`../plans/_archived/20261123-todo-measurements/learnings.md`).

### The `Face focus` state is `mark.rs`'s port of `focus.ts` (Inferred)

- Rule: `mark::af_eye_state` is the one place the strip's icon, the filter's
  `Face focus` items and the overlay's mark color read a file's state from,
  as `afEyeState` was in the Tauri frontend. It reads `full_eye_sharp` alone
  for the eye focus: `Low` under `LOW_FULL_EYE_SHARP`, `Good` when
  the engine's `StoredEyes::photo_tier` holds (at or above
  `GOOD_FULL_EYE_SHARP` with the EAR, pose, eye offset and edge gap cuts, all
  defined in `crates/engine/src/index.rs`), else `Neutral`, a missing value
  included. `GOOD_FULL_EYE_SHARP` is `fullsharp::SOFT_BELOW` itself;
  `LOW_FULL_EYE_SHARP` mirrors `LOW_BELOW`, and a test compares it with the
  core's.
- Seen (2026-11-22, Windows 11): on the `20261113-filter-sort` fixture's
  `faces` folder, a Face focus Good filter left 2 files
  (`ILCE-1M2_sony_a1_ii_03`, `_DSC1758`), each with the green smiling
  `scan-face` icon, and `DSC00405` was no longer Good. A check written before
  the rule moved to the full-size eye sharpness must not expect the old set;
  re-derive the expected files from the current `photo_tier`
  (`../plans/_archived/20261122-windows-device-checks-4/learnings.md`).
- Seen (2026-11-23, Windows 11, through the MCP companion on a fresh index):
  the same filter showed an empty strip and `No files match the current
  filter.  analyzing n/8` while pass 2 ran, then the two files with their
  green icons (`1 / 2 (8)`). The strip filled in one step at the end of the
  pass, not file by file: pass 2 writes its rows `BATCH` (10) at a time, so
  an 8-file folder refilters once
  (`../plans/_archived/20261123-todo-measurements/learnings.md`).
- Source: `docs/plans/20261115-full-eye-sharp-only/plan.md`, Step 3.

### The bursts are regrouped with the entries and marked with the judgments (Inferred)

`burst.rs` ports `burst.ts`: a burst is consecutive frames in capture order
no more than `BURST_GAP_MS` (1000 ms) apart, a file without a usable capture
time a burst of its own. The view keeps the open folder's grouping in
`bursts`, keyed by the listed path, as the Tauri `bursts` map.

- Rule: regroup only in `set_entries`, over `all` with the capture time and
  subsec of `facts`, through `sort::order_files(SortKey::Capture, ..)` (no
  second capture order). Both callers (the open and pass 1's entries read)
  have set `all` first, and another folder's open replaces the grouping
  there, which stands in for the Tauri reset. `refilter`, a page, a judgment
  and a trash undo's `all.extend` do not regroup: as in the Tauri app the
  grouping was stale until the next entries read, a file not grouped yet being
  its own burst for the moves.
- Rule: the strip's per-cell marks (`burst_marks` over `files`) are set in
  `mark_strip` next to the judgments, so the open, every rebuilding
  `refilter` and an entries read that leaves the list unchanged all re-mark
  the cells. `Strip::new` drops them, so the marks follow every rebuild.
- Rule: `burstPrevious` / `burstNext` and `burstFramePrevious` /
  `burstFrameNext` are `move_burst`: `burst_step` / `burst_frame_step` over
  the shown files' ids (the burst id, else `-1 - at`), then
  `go(Move::To(..))`, which selects the landed file alone and starts the
  preview latency at the key. They run in a view-only folder too, and
  `tree_gate` swallows them while the tree has the keyboard.
- Rule: `paint_strip` paints every on-screen band first, masked to the
  strip, then the cells, so a band reaching across the gap never covers the
  neighbour's outline. The band is `.cell.burst::before`: `rgba(255, 255,
  255, 0.08)` from the cell's padding box 5 px above and below and 1 px
  right, on a member after another left across the 8 px gap, 2 px out with
  a 6 px radius at the band's first and last cell, the gap and the cell
  being the current `strip::Geometry`'s. `STRIP_PADDING` is 7 px at every
  strip height, the Tauri strip's, room for the band. The `position/size` badge is shaped
  text at the image box's bottom-right, 2 px in, in the fixed `BADGE_COLOR` (`#fafafa`, the same in every theme as it sits over the photo) over
  one black copy 1 px down and right, as the stars are, painted before the rejected dim so a
  rejected member's badge dims with its cell, and a rejected member's band
  is drawn at the dimmed alpha (`Roles::band_dimmed`, 0.08 x 0.45) as
  `.cell.rejected`'s opacity covers the `::before`.
- Deliberate difference: a selected member's band is painted before the
  cell's opaque `--muted` fill, so inside the cell the fill covers it; Tauri
  painted the 8% tint over `--muted` (`isolation: isolate`).
- Deliberate difference: `burst.ts` builds the time with `Date.UTC`, which
  rolls a month 00, a day 32 or an all-zero timestamp over into a finite
  time, so such files could share a burst in the Tauri app. `capture_ms` parses
  strictly (month, the month's days, 23:59:59), and such a file has no
  capture time, a burst of its own.

### The sort and filter menus are deferred popovers over the window (Inferred)

`Sort ▾` and `Filter ▾` sit at the right end of the strip's header row
(`main.rs`'s `strip_header`, `stripheight::HEADER` high, on the strip's top
edge; see "The strip's height is one number, stored under `stripHeight`"),
so the strip pane owns the controls over which files are listed and in what
order. The status row is gpui-component's `StatusBar` (see "The status row
is gpui-component's `StatusBar` with its defaults"): the status text in its
center region, then, in its right region, the strip position and the
progress circle while an operation runs. The status text is a transient note (an error, the drop
refusal, the empty-folder and filtered texts) that `present` clears on every
presented preview, as the Tauri `present`'s `setStatus()` did, followed by
`status()`'s suffixes with no leading separator when the note is empty; the
current file's name and shooting settings live in the meta pane only, which
holds only the shown photo's details. The position (`pager::position`) is
`n / shown` (`3 / 120`), and while `FilterState::is_active` the listing
`all`'s count follows in parentheses (`3 / 80 (120)`, also `3 / 120 (120)`
when the filter hides nothing), mirroring the lit `Filter ▾`, and while
more than one file is selected ` · K selected` follows (`3 / 80 (120) · 4
selected`), as the Tauri `renderMeta`. It shows
whenever `files` is non-empty, from the listing on, not only once a preview
has arrived, and not at all with no shown file (the pager's index is 0 on an
empty list, so a bare format would print `1 / 0`). The position stays in the
status row while the strip is hidden (`toggleStrip`), so the filter-active
count still shows when the header and its lit `Filter ▾` are gone. `menu.rs` holds the menus' rows, the clicks' `apply` and `Menus`;
`main.rs` only maps rows to `div`s. The filter button is lit while
`FilterState::is_active`; the sort button is dimmed and has no handler in a
view-only folder, whose open closes a sort menu left open.

- Rule: with no folder open (`Handpic.dir` unset) both buttons are inert the
  same way, dimmed with no handler, so neither menu opens (`menu::enabled`,
  which also holds the view-only rule): with no folder every other key and
  click does nothing too. `dir` is only ever set, never cleared, so no open
  menu needs closing when a folder goes. The MCP `set_sort` / `set_filter`
  path (`apply_menu`) opens no menu and is not gated by it. Verified on
  Windows 11 on 2026-11-23: both buttons were dimmed and a left click on
  either opened no menu
  (`docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`).

- Rule: the two buttons are gpui-component `Button`s (`main.rs`'s
  `menu_button`): the default variant at `.xsmall()` with
  `.tab_stop(false)` (the main view routes every key through its own
  handle), its height pinned to `MENU_BUTTON_HEIGHT` (21 px, the bordered
  `div`'s they replaced), `px(6.)`, `rounded_sm` and the text `Sort ▾` / `Filter ▾` as a
  12 px child, not `dropdown_caret`. The variant paints the mapped `button`
  fill, `input` border and `button_hover`. Lit, the border and the label
  child are `primary`, without `.selected(true)`, which would change the
  fill; on hover the component's hover style sets the border back to
  `input`, so a lit button shows its `primary` border only while not
  hovered (the label keeps `primary`). An inert button (the sort in a
  view-only folder, both with no folder) is
  `.disabled(true)` (no hover, no press) with the normal fill and border
  replayed over the disabled state's and `opacity(0.5)`, as the `div` was
  dimmed. The toggle stays the caller's `on_mouse_down` (`Button` implements
  `InteractiveElement`); `Button`'s own mouse-down only calls
  `window.prevent_default()`, so the window's focus handle keeps the focus. A menu item runs
`menu::apply`, then `refilter` on the current file with the scroll not kept,
so a reorder scrolls the current file into view. `menu_action` then writes
`sortOrder` (`SortKey::setting`) when the sort key changed, and `filter`
(`FilterState::stored`, then `Stored::setting`: the remembered sections, each
in the menu's item order except `stars`, which stays ascending (0 to 5)
whatever the menu's order, never `Output` or the EXIF checks) when a remembered
section changed, `Reset` included; a change of `Output` or EXIF alone writes
nothing. Nothing is written at load, so a seeded `recommended` stays stored
until the user picks an order, and a view-only folder, whose sort button is
dimmed, shows capture order and keeps the stored key.

- Rule: every filter section starts with a `Row::Heading` right after its
  separator (no hover, no click handler, no check slot); `Reset` sits above
  the first separator without one. Four headings are the app menu's own,
  as the Tauri `#filter-menu` had none there: `Flag`, `Rating`,
  `Color label` and `Orientation`. The sort menu has no heading.
- Rule: the leads are drawn per kind in `main.rs`'s `menu_row`: the flags
  keep their 7 px dots; a rating row is five 12 px Lucide stars, the `on`
  filled ones in `STARS_COLOR` and the rest outlined in `muted_foreground` at
  40 %; a color label no lead, its name drawn inside a small gpui-component
  `Tag::color` of its `ColorName` (`main.rs`'s `label_color`, the strip
  band's mapping too), `No label` inside a small outline secondary `Tag`,
  the Tag's colors left to its own dark / light choice; `Portrait` and `Landscape` the Lucide `rectangle-vertical` and
  `rectangle-horizontal`; `Face focus` the Tauri icons (`scan-face` for
  `Good`, its frown derivative for `Low`, in `MarkState::menu_color()`:
  `MarkState::color()`, but `Good` green-700 `#15803d` on a `popover`
  `theme::is_light` calls light, the strip's icon and the `f` overlay
  keeping `color()`) with an
  empty 12 px slot for `Neutral`. `menu.rs` names the kind (`Lead`), never an
  asset path.
- Rule: the rating rows run 5, 4, 3, 2, 1, 0 stars from the top, the labels
  are Tags and the orientations have icons, all unlike the Tauri menu on
  purpose (the user's decision); a later port must not "fix" them back.
- Rule: the filter menu ends with the folder's EXIF sections, after `Output`
  (or `Eyes` without an export answer), in the Tauri order and headings
  (`Camera`, `Lens`, `Aperture`, `Shutter speed`, `ISO`, `Focal length`),
  each a separator, a heading and one item per label, a group the folder
  lacks left out. Numbers sort by value (`f64::total_cmp`), ties by label;
  `Camera` and `Lens` by a case-insensitive compare, ties by the plain one
  (Tauri used `localeCompare`; no collator here). The focal length is
  bucketed into `exif::FOCAL_RANGES`, half-open. An item is keyed by its
  group and label (`Toggle::Exif`), not its row, since the groups can be
  rebuilt between the frame that drew the menu and the click, which made
  `Row`, `Item`, `Label` (with `Label::Owned`), `Toggle` and `Action` `Clone`
  rather than `Copy`. The longer menu relies on the popover's scroll.

- Rule: an open menu is `div().absolute().top_0().right_0()` inside a
  `relative()` wrapper of its button, holding `deferred(anchored()
  .anchor(Anchor::BottomRight).offset(0, -4).snap_to_window())`: `anchored`
  without `position` places itself where it is laid out (here the button's
  top-right corner), so no bounds need reading back, and `deferred` paints it
  after everything else. The popover is `occlude()`d, so the strip's
  `on_mouse_down`, which checks `hitbox.is_hovered`, does not see a press
  inside it, and its height is capped at the window's less 40 px: its row
  list (`main.rs`'s `menu_surface`) is
  `overflow_y_scroll` with `track_scroll` on a `ScrollHandle` the view keeps
  (`menu_scroll`, new on every press of a menu's button, so a menu opens at
  its top), and the popover carries gpui-component's scrollbar over it
  (`ScrollableElement::vertical_scrollbar`), as `PopupMenu` lays out its own.
  The scrollbar is a child of the popover, so a press on it is not one
  outside (`on_mouse_down_out`). Nothing is laid out while both menus are
  closed.
- Rule: the look comes from gpui-component's theme (`cx.theme()`), not the
  `u32` constants: the popover's `popover` fill, `popover_foreground` text,
  1 px `border` and `radius`; the headings in `muted_foreground`; the
  separators `PopupMenu`'s (a 2 px `border` line across the list's padding);
  the rows `PopupMenu`'s item at its default size (26 px tall, 8 px side
  padding, the theme's radius, `accent` / `accent_foreground` on hover, a
  12 px column with the component's `IconName::Check` for a checked item;
  `menu_item`), the text kept at 13 px. The leads keep their own colors.
  `PopupMenu`, `Popover` and `ContextMenu` are not used for these two menus:
  `PopupMenu::activate` always dismisses the menu, where a Sort / Filter item
  click applies and leaves it open, and they take the GPUI focus. Their row
  element
  (`MenuItemElement`) is `pub(crate)`, so the rows are this app's `div`s
  styled the same.
- Rule: `on_mouse_down_out` (a capture-phase listener, run before any
  bubble-phase `on_mouse_down`) on the popover closes it on any press outside,
  including one on its own button, whose bubble-phase `on_mouse_down` then
  runs for the same press. `Menus` keeps the closed menu and the press's
  position, so that button press does not reopen it (the Tauri toggle sat
  inside `#sort` / `#filter`, which its `mousedown` listener spared). The
  buttons toggle on `on_mouse_down`, not `on_click`, for that reason; items
  use `on_click`.
- Rule: `escape` closes an open menu, taken in the key handler before the
  keymap's actions like `home` / `end` / `q`, and only while a menu is open,
  so a user binding on `escape` still works otherwise (no default binding
  uses it). Every other key goes through the window's handler as before, so
  paging, `e`, `z`, `f` and the burst keys work with a menu open.
- Measured before the buttons moved to the strip header: the preview canvas
  was 1160 x 842 logical px in the default 1600 x 1000 window with the tree
  shown and the status row a `StatusBar` (a debug build at scale 1, the
  canvas's paint bounds logged), the same with the progress circle shown and
  hidden. Before the `StatusBar` (a plain flex `div`, no padding or border)
  it was 852 tall, so the row grew 10 px. Measured with the header
  (2026-11-22, Windows 11, a debug build at scale 1, by scanning a window
  capture's pixel runs): the canvas's surround is 1158 x 767 between the
  pane borders and the header's line, 1160 x 768 counting them, in the same
  window and layout, with the circle shown and gone; not the 810 the header's
  32 px alone would give, a difference not traced to its source
  (`../plans/_archived/20261122-windows-device-checks-7/results.md`).
- Rule: the header is a fixed-height row outside the cells' `div`, in a
  `relative()` `flex_col` wrapper with it and the resize handle, all under
  the one `panels.strip` check, so it hides with the strip and the cells'
  press, right-press and scroll handlers never see a press or a wheel on
  it. The buttons start 6 px down, below the handle's 5 px band. The
  popover's height cap is `menu_max_height` less the strip pane's height
  (header and cells), the room above the header, so `snap_to_window` has no
  reason to push it over its own button. `change_panels` closes an open menu
  when the strip goes, as a view-only open closes a sort menu.
- Not verified in the running app: that a press on an item reaches its
  `on_click` and not the strip (the menu opened over the preview and the
  meta pane and never overlapped the strip). It is read from GPUI's `div.rs`
  at the pinned rev; the hand check is pending in
  `../plans/_archived/20261113-gpui-filter-sort/learnings.md`.
- Seen (2026-11-22, Windows 11): `escape`, a press on the preview, on a strip
  cell and on the menu's own button each close the filter menu, the
  button's staying closed; the strip cell press also selects that cell; the
  pager and `e` / `f` / `z` work with the filter menu open (results in
  `../plans/_archived/20261122-windows-device-checks-4/learnings.md`).
- Seen (2026-11-23, Windows 11, through the MCP companion's `get_view`):
  the EXIF sections the menu builds from (`filter_offers.exif`) were empty
  through pass 1 of a fresh 1000-ARW index and appeared once it ended, in
  the order `camera`, `lens`, `aperture`, `shutter`, `iso`, `focal`, with
  the folder's one lens; on the fixture's `faces` folder `lens` listed six
  lenses after the five cameras. No tool opens the menu, so the drawn
  sections were not seen
  (`../plans/_archived/20261123-todo-measurements/learnings.md`).

### The icons are Lucide SVGs served by one `AssetSource` (Inferred)

`svg().path(p)` loads `p` through the application's `AssetSource` and paints
the rendered alpha mask in the element's `text_color`, so a stroked Lucide
icon and a filled derivative both draw in one color. `icons.rs` holds a
path constant per icon (`icons/<name>.svg`), the SVG strings of the app's
own derivatives (the filled star, the frowning `scan-face`, the title bar's
thin `settings` gear),
`Catalog`, the catalog icons selected with `gpui_kit::assets::icon_assets!`
(`rectangle-vertical`, `rectangle-horizontal`, `scan-face`), and `Icons`,
whose `load` answers the own paths with borrowed bytes, then `Catalog`'s,
then any other from `gpui_kit::assets::Assets`, which serves `star`
and the tree's `folder` and `folder-open`, a path in none of them with
`None`; `main()` registers it with
`application().with_assets(icons::Icons)`.

- Rule: to add an icon the `gpui-kit-assets` catalog has, add its variant to
  the `icon_assets!` list and a path constant, the literal its `IconName`
  variant's `path()` gives (`path()` is not `const`); an icon the default
  `Assets` already holds needs only the path constant. Only a derivative (a
  filled shape, the `scan-face` frown, a thinner stroke) is copied in as an own SVG: copy its
  base from `lucide-static` at the version the header names (v1.48.0, as
  the Tauri frontend's `icons.ts` did), add its SVG and path constants and its
  entry in `SVGS`, and name it in the header comment, with any
  Feather-derived base (the list in `LICENSE-lucide`). The full notice is
  `crates/app/LICENSE-lucide`, first copied from the Tauri app's.
- Rule: GPUI keeps one `AssetSource` per application. Other sources (here
  `Catalog` and `gpui_kit::assets::Assets`) are composed with `Icons`, in the
  order own derivatives, then the catalog selection, then `Assets`, never
  registered in its place: a missing asset draws nothing and fails no build.
  `star` comes from the default `Assets` rather than the `icon_assets!`
  list, so it is not embedded twice; if a gpui-kit-assets bump drops it
  from that set, the every-path test fails and the icon
  moves into the list. `icons.rs`'s tests check every path constant loads an
  SVG, each catalog constant equals its `IconName` variant's `path()`, a
  derivative loads its own bytes, `icons/check.svg` loads from `Assets`, an
  unknown and the empty path load nothing, and the listing names each path
  once.

### The settings modal is its own entity, opened on `ctrl+,` (Inferred)

`settingsview.rs`'s `SettingsView` is an `Entity` the window creates once and
holds; `settingsmodal.rs` is its pure model (the open state, the shortcut
row capturing a key, `modal.ts`'s key rule, the pages' order and titles in
`TABS`). It is shown over the whole window below the title bar, not in a
dialog (the user's choice, 2026-11-15): `Handpic::render` wraps its root
`div` (the one with the `Handpic` key context, the focus, the key listener
and `on_drop`) in an outer full-size `div` and, while the modal is open, adds
the view entity as a second child, in an `absolute` `div` set
`top(TITLE_BAR_HEIGHT)`, `left_0`, `right_0` and `bottom_0` (a full-height
box moved down would overflow the window's bottom). The title bar is the
first in-flow child of the root's `flex_col` and the outer `div` lies inside
`Root`'s `window_border()`, so on every platform that offset is the bar's
bottom and the Linux client-side frame's padding is already outside it. The
title bar (the icon, the folder's name, the gear, the window drag, the macOS
traffic lights and the Windows / Linux window controls) stays drawn and
takes presses while the settings are shown; before 2026-11-25 the layer
covered it too, so the window could not be dragged and the traffic lights
overlapped the `Settings` heading. The UI test
`the_settings_layer_lies_under_the_title_bar_and_fills_the_rest` reads the
box's bounds (its `settings-box` debug selector) back: its top is
`TITLE_BAR_HEIGHT` below the view's and it reaches the viewport's other
edges (Measured, on the test platform, whose window has no paddings; the
OS window drag and the traffic lights are not on it). The view's box is
`size_full`, opaque on `popover`, `occlude()`d
and under `FocusTrapElement::focus_trap` on its own focus handle. The main
layout stays rendered underneath, not replaced: the preview canvas's paint
takes the pending page and releases the scan pause (`hold.rs`'s
`PaintHold`), and a re-listing or an MCP tool call can change the current
file while the settings are shown. While closed the view's `render` returns
`Empty`. Under the title row (the `×`) and over the status line, the body is
one gpui-component `setting::Settings` element, rebuilt on every render,
with eight `SettingPage`s, Sidecar, Culling, Keyboard Shortcuts, Cache,
Import and MCP (no Debug), then Appearance and Updates, the app's own, in a `relative` `flex_1` slot holding an
`absolute` `inset_0` `div`, since `Settings` is an `h_resizable` that fills
its parent. Every open starts on Sidecar and `open` focuses the box's own
handle with `window.focus`.

A change is emitted as a `settingsview::Change`; the window applies it in its
`subscribe_in` callback, the setting changes through `apply_setting`, which
`set_settings` runs too (the main view stays the owner of the values):
`AutoAdvance` sets `settings.auto_advance` and writes `autoAdvance`,
`GroupMinFaces` (the Culling page's number field under it) sets
`settings.group_min_faces`, writes `groupMinFaces`, refilters and, when
lowered, re-lists the open folder through the watcher's path (`changed`), so
pass 3 runs on the rows that now meet it once any scan under way ends,
`McpAllowDestructive` (the MCP page's one switch, off by default, which lets
an MCP client run the recoverable destructive operations without the app's
dialog) sets `settings.mcp_allow_destructive` and writes
`mcpAllowDestructive`,
`Theme` (the Appearance page's System / Light / Dark radios and its light
and dark theme pickers) applies the setting under the window's appearance and writes `theme` (read and written
under `--auto` too, as the other settings of the modal),
`Keymap` replaces the keymap (next section),
`ClearCache` (the Cache page's button) hands the press to the window's
`cache::ClearFlow` (see the rule on the Cache page below),
`Closed` focuses the window's handle and redraws, dropping the layer; the
tree's keyboard is a flag the settings do not set (but see the pointer rule
below).
The status line under the box shows `NOT_SAVED` while the settings store did
not open.

- Rule: the closes. `close` (Escape or the view's own `×`) closes the model
  and emits `Closed`. The title bar is the only outside to press and a press
  there is not a close, so there is no press-outside close; `ctrl+,` while
  shown does nothing (the window's `key_down` is not on the path), nor does
  the title bar's gear (`open_settings` returns while the settings are open,
  Inferred from the code; the UI test above clicks the bar's folder-name
  overlay, not the gear, and checks only that the settings stay open). The box's
  `capture_any_mouse_down`, which cancels a shortcut capture, does not see
  a press on the title bar, so a capture under way survives a window drag
  (Inferred, not checked by hand).
- Rule: the pointer. The box's `occlude()` blocks the hitboxes painted
  before it under its bounds, the whole main layout but the title bar, so a
  press, a hover or a wheel scroll over the settings does not reach the tree, the preview canvas, the strip
  or their tooltips. The deferred layers the main view may still draw paint
  after it: the sort and filter menus are closed on open and the right-click
  menu on any key, the `ctrl+,` included; a trash or folder dialog held by
  the idle gate, and the Cache page's `Clear the index cache?`
  confirmation, open in `Root`'s layer above the settings, with their
  `backdrop` (deferred, so it paints over the settings layer too). The tree pane's `on_mouse_down_out` is a capture-phase
  listener that ignores occlusion, so a press inside the settings hands the
  tree's keyboard back to the photo, as a press inside the old dialog did.
  Over the title bar the hover and the gear's tooltip work while the
  settings are shown, and a file dropped on the bar reaches the root `div`'s
  `on_drop` and opens its folder under the open settings (Inferred from the
  hit test, not measured; the user chose not to guard it, 2026-11-25).
- Rule: the key routing. The layer is a sibling of the `Handpic` `div`,
  outside the app view's dispatch path, so no key with the focus inside it
  reaches the window's `key_down`; that handler also returns at once while
  the modal is open. The box has its own focus handle (`track_focus`) and an
  `on_key_down` that calls `cx.stop_propagation()` on every key but one typed
  into a text field (`settingsmodal::stops`). A text field is told by the
  `Input` key context on the focused path (`window.context_stack()`), not by
  the view's own handles, since the component's search field is private.
  There is no `Dialog` key context, so Escape closes from the listener
  (committing a focused name or pattern field first) and Enter presses the
  focused control. In the search field, Escape first clears a non-empty
  query (the sidebar header's `Escape` handler, which then does not
  propagate); with the query empty it propagates and closes.
  `ctrl+,` (`meta+,` on macOS, `settingsmodal::OPEN_KEY`) is taken literally
  after `home` / `end` / `q`, closes an open sort or filter menu, and opens
  the modal, not while a folder flow or the trash flow is busy; the keymap
  refuses to bind it.
- Rule: the Cache page (Inferred, not yet observed in the running app; a
  port of the Tauri `#cache` panel and `clear_index`). It shows
  `Index cache: <size>`, the index file plus its `-wal` and `-shm` on disk
  (`cache::on_disk_bytes`, a missing file zero, `format_bytes` /
  `SIZE_BASE`), read on `background_spawn` from the path `open_index` opened
  (`--index` honored) on every open of the modal, at every `finished` while
  it is open and after a clear; `0 B` without an index. `Clear Cache` emits
  `Change::ClearCache`; the window's `ClearFlow` (one slot of its own, not
  the idle gate, whose request would replace a held trash or rename) asks at
  once, or, while `resync.busy() || gate.in_flight()`, holds the press with
  the page's `WAITING_NOTE` and drains it (`drain_clear`, the guard
  `ClearFlow::idle`) at every idle point once nothing else started (nor a
  trash or folder dialog is up): `finished`, and the ends that need no
  listing: a trash run's with no folder open (`trashed`, a tree
  folder's Delete Sidecars; with a folder open it re-lists), a trash undo's (`restored`, one into
  another folder lists nothing), a sidecar rewrite's (`sidecars_ran`) and a
  folder run's that reopens nothing (`settle_folder_run`, a failed rename
  among them), and the cancel of the trash dialog (`dismiss_trash`) or of a
  folder dialog (`dismiss_folder_dialog`), by its Cancel, Escape or an MCP
  `cancel_operation`, so a clear held past an idle point that came while
  that dialog was up goes on once it closes. The confirmation is a
  component `Dialog` through `modal.rs`, the first one opened from inside
  the settings: its keys reach `clear_key_down` in the footer's capture
  phase, so Escape cancels the dialog, not the settings, and the component
  hands the focus back into the settings box when it closes. `Clear`
  re-checks the busy rule (a confirmed press then waits and runs without
  asking again). The run takes the gate's in-flight count and
  `resync.hold()`, so no listing or scan starts under it, then on
  `background_spawn`, under the `format_switch` mutex the sidecar switches
  take: the judgment stores' drain, the sidecar writer's flush and
  `Index::clear` on the writer connection (waiting for the launch's
  eviction on the same mutex), its `EvictSummary` logged as
  `index cleared: …`. Its end drops the touched paths and the judgment undo
  history, re-lists the open folder with carry (`relist`), re-reads the
  tree's cached counts and the size; a failure goes to the settings' status
  line and re-lists nothing. Closing the settings drops a held or asked
  clear, not a running one. Nothing here reads or writes a setting, so
  `--auto` needs no special case.
- Rule: Tab and Shift+Tab are `Root`'s. Its `Tab` / `TabPrev` actions run
  before the box's listener and walk the box's focus trap with
  `focus_next` / `focus_prev`, which wrap; the trap holds only the modal's
  controls, the same stops in the same order as the box's own `focus_next`
  walk had (the window's root handle is not a tab stop). So `ModalKey` has no
  focus move any more. The controls are the component's search `Input`,
  gpui-component's `Button` (the outlined `small_button`s: the example
  patterns, `Reset`, `+`, a key's `×`, `Reset all`; the ghost close `×`;
  the `InputGroupButton` `Choose…`), `Switch` (auto-advance, the built-in
  `SettingField::switch`), `Radio` (the three formats, standalone, not
  a `RadioGroup`, whose arrow-key roving would change the cycle) and the
  `Select`s of the label names' language and of the two themes, each a tab
  stop at the default `tab_index(0)` with a focus handle of its own (keyed
  state, or `track_focus`), and the `Input` fields, the read-only
  Destination root among them. `Kbd` is not a tab stop. The sidebar's page
  entries are not tab stops (next rule), so pages are switched with the
  pointer only; ArrowUp / ArrowDown / Home / End on a tab and the roving
  `tabindex` went with the hand-drawn tab list.
- Rule: what the component imposes, read from `gpui-component` 0.9.1's
  `src/setting/` (Inferred). The selected page and the search field live in
  the element's `window.use_keyed_state`, in a `pub(super)`
  `SettingsState`: the view can set only `default_selected_index`, neither
  read nor set the page later, and the state is dropped while the settings
  are hidden, so the page is not remembered across opens. The sidebar's
  `SidebarMenuItem`s have no focus handle. The sidebar always has the search
  field, with no option to hide it; a custom full-width item
  (`SettingItem::render`) is hidden while the query is not empty unless its
  `keywords` match, so the language row, `Reset all`, the example line and
  the specifier table carry keywords, and each Keyboard Shortcuts item is
  titled with its `shortcut_label` so the search finds actions; `Settings`
  draws only a group with a matching item, so a shortcut group none of whose
  rows match is not drawn, heading included.
  `SettingField::input` calls its setter on every `InputEvent::Change`, a
  write per keystroke (for the label names, `switch_names` with its writer
  drain and re-listing; for the pattern, a half-typed one stored), so the
  name and pattern fields are custom fields (`SettingField::render`) over
  the view's own `InputState`s with the Enter / blur commit. The component
  has no radio field, so the formats are a custom field too. The item and
  field closures are `Fn(.., &mut Window, &mut App)`; the view builds them
  with the values it shows and its `Entity`, through `window.listener_for`,
  and they run during layout, after the view's render returned, so there is
  no double lease. `Settings`, `Sidebar`, `GroupBox` and the resizable
  register no `KeyBinding`. The component's own strings
  (`Settings.search_placeholder`, the no-results texts) come from its
  `rust_i18n` locales; nothing calls `set_locale`, so they are English.
- Rule: a number setting is a custom field over the view's own `InputState`
  drawn as `NumberInput`, not `SettingField::number`. The component's number
  field calls its setter on every `InputEvent::Change` with the value clamped
  into `min..=max`, so typing `15` would first commit `2`, and a setting
  whose commit re-lists the folder (`Faces in a group photo`) would fire per
  keystroke. Give the `InputState` no `step`, `min` or `max`: with them a blur
  clamps the text before `InputEvent::Blur` (`25` would commit `20` where the
  rule is to refuse it), and a step button replaces the text silently, with no
  `Change`. Without a step, `apply_number_step` emits `NumberInputEvent::Step`,
  which the view takes (`settingsmodal::group_stepped`) and commits at once
  within the range. Commit on Enter / blur / step, and refuse an out-of-range
  value in the status line.
- Rule: Enter and Space on a focused control come for free: the Base
  `Button` and `Radio` under the components are `div`s with `on_click`, which
  turn a key down then up of either into a `ClickEvent::Keyboard` (`div.rs`),
  and their listener runs before the box's bubble-phase handler stops the
  event. The `Select`'s come from its own `Confirm` binding, not `on_click`.
  The `Switch` reaches its callback through `on_change` on a `BaseSwitch`
  (`switch.rs`); that it gets Enter / Space the same way is Inferred, pending
  the hand check in `learnings.md`.
- Rule: a pointer press on a component control does not move the focus (each
  calls `window.prevent_default()` on mouse down, which also keeps the box's
  focus-on-press from running). So a control pressed while a field has the
  focus calls `leave_field` first, which focuses the box, and the field
  commits on its `Blur` as it did when the press focused the old control.
  The `Select` needs none: opening its menu moves the focus to the menu's
  list, so a focused field blurs and commits by itself.
- Rule: the language picker (Inferred, read from `gpui-base` `select.rs`
  and gpui-component `select.rs` / `list/list.rs`). The view holds one
  `SelectState` over the presets' names, not searchable or cleanable, and a
  `SelectEvent::Confirm` subscription that commits the chosen preset's names
  through `commit_names`. `show_names`, which every open and every commit of
  the names goes through, selects the preset whose names equal those in
  force (`settingsmodal::preset_of`), else none (the placeholder). Setting
  the selection emits no `SelectEvent` (`set_selected_index` and
  `set_selected_value` alike), so only a choice in the menu applies names.
  `show_names` syncs with `set_selected_value` (it clears the search query
  first, while `set_selected_index` indexes the delegate's matched items, so
  a leftover query could shift it) or `set_selected_index(None)`, and the
  subscription runs as an effect after the `SelectState`'s
  own update, so its sync of the selection is no nested update. It is not
  `SettingField::dropdown`, which draws a `Button` with a `PopupMenu`. Escape with the menu open closes the
  menu only (the list's `Cancel` propagates to the `Select`'s, which stops);
  with it closed, the `Select`'s `Cancel` propagates, so Escape reaches the
  box's `key_down` and closes the settings. Actions run before the box's
  listener, so `settingsmodal::stops` is unchanged. The menu is `deferred`
  inside the view, under the box's key listener and focus trap.
- Rule: the theme pickers are two more `SelectState`s built the same way,
  each over `theme::names` of its mode (Neutral, then the shipped themes of
  that mode in file order), shown the stored name on every `open`
  (`set_selected_value`), their `Confirm` going through
  `settingsmodal::theme_change`, which refuses the name in force and a name
  not of that mode. Their keys are the language picker's above.
- Rule: the Destination root is an `InputGroup` holding a read-only `Input`
  over the view's own `root_field` (`show` on every open and after a
  `Choose…` answer) and, in an `InlineEnd` addon, the `InputGroupButton`
  `Choose…`. The path can be selected and copied, and scrolls in one line.
  The field is not in `editing()`'s commit list (nothing to commit), and
  Escape in it closes the settings through the `Input`'s propagating
  `Escape`. A press on `Choose…` does not focus the field (`InputGroup`
  checks `window.default_prevented()`).
- Rule: the look. The pages are the component's: a header with the page's
  title (the Import page's description is `IMPORT_NOTE`), groups in the
  default `GroupBoxVariant::Normal` (no surface), and items with their
  14 px title left and the control right, stacked when the page is 480 px
  wide or less. The Sidecar page holds a `Format` item (the three radios)
  and, while the format writes XMP, a `Label names` group (the
  `LABEL_NAMES_NOTE` as its description, one item per color, then a
  `Lightroom language` item, the Tauri `aria-label`, holding the `Select`
  (`.small()`, 160 px wide)); the Culling page the auto-advance
  switch and the `Faces in a group photo` `NumberInput` (`.small()`, 120 px
  wide, `GROUP_NOTE` as its description); the Keyboard Shortcuts page one titled group per
  `settingsmodal::SHORTCUT_GROUPS` entry (`Filmstrip`, `Judgment`,
  `Preview`, `Panes`, `General`) of one item per action, then an untitled
  group holding only `Reset all`;
  the Import page `Destination root` (its `root_note` as the description),
  `Folder pattern`, the example line, `Example patterns` and the specifier
  table; the Appearance page `Mode` (the three radios), then `Light theme`
  and `Dark theme`, each a `Select` (`.small()`, 200 px wide).
  The outlined buttons are `Button`'s default variant at
  `.small()` (24 px tall) with `px(6.)`, `rounded_sm` and a 13 px label
  child (the variant's own label is `text_sm`, 14 px at the window's 16 px
  rem, and its content style is crate-private), on the mapped `button` fill
  and `input` border. `Radio` is `.small()` (a 14 px indicator) with a
  13 px `text_size`; a checked `Radio` is the component's filled circle with
  the check mark, not the old ring and dot. The `Switch` is the
  `SettingField::switch`'s, `primary` when on and `Default Dark`'s `switch` /
  `switch_thumb` (`theme.rs` maps neither). Each key on the Keyboard
  Shortcuts page is a `Kbd` (the `muted` fill, half the theme radius) with
  its text set back to the old chip's 13 px `foreground`, its `×`
  `small_button` beside it rather than inside, since `Kbd` is not a
  `ParentElement`. The Destination root's `InputGroup` is 360 px wide with
  a transparent fill, `rounded_sm` and a 13 px text over the group's own
  `input` border; focused, the border and the ring outside it are the
  component's `ring`, set after the caller's style, not the name fields'
  `primary` border. The focus ring is the theme's: `focus_ring` is on in
  `Default Dark`, so a focused control gets the `ring` border and a ring
  outside it.
- Rule: a page scrolls in the component's `list` of its groups, with the
  theme's scrollbar in the `ScrollbarMode::Always` `apply` sets (shown
  whenever the list overflows). Nothing scrolls a focused row into view.
- Rule: no AI toggle. The user decided on 2026-11-14 not to port the Tauri
  app's AI features and on 2026-11-21 to remove them; the AI switch's key in a
  store seeded from the Tauri app stays there, unread.
- Not verified headless (mouse clicks and the modal's keys cannot be driven
  by `--auto`): the clicks, the occlusion, the focus cycle and the keyboard
  click. All are read from GPUI's `window.rs`, `div.rs` and `tab_stop.rs` at
  the pinned rev; the hand check is pending in
  `../plans/20261113-gpui-settings/learnings.md`.

### The update flow runs on threads of its own and restarts through GPUI (Measured)

`update.rs` is the pure model, as `cache.rs` is to the Cache page; the
engine's `update.rs` is the updater. The updater reports no phase, since
`self_update`'s `update()` downloads, verifies and installs in one blocking
call, so `Updates` sets the phases (`Idle`, `Checking`, `Available`,
`Downloading`, `Verifying`, `ReadyToRestart`, `Failed`, gpui-kit's guide's)
around `update::check` and `update::install`. The window runs each on a
`std::thread` (a download can take minutes, too long for the background
executor's pool) and the answers come back through the view's one channel
as `scan::Event::Update`. The download's `progress` callback is sent only
when its percent changes (`progress_step`; each MiB without a size), and the
download reaching its size turns `Downloading` into `Verifying`, the digest,
the signature, the extraction and the replace following inside the same
call.

- Rule: the launch check. Only in a release build (`cfg!(debug_assertions)`
  off), never under `--auto`, and at most once an hour: the time of the last
  automatic check is the settings store's `updateCheckedAt` key (Unix
  seconds, a time in the future counting as due), written before the check
  runs, so a launch that fails offline is not retried at the next launch
  within the hour either. It is a key in the store, not `self_update`'s
  `UpdateCheckGuard` stamp file: the store already keeps the session's
  state (`lastFolder`, `windowBounds`) and its path follows `--settings`.
  `Check for Updates` always checks and writes nothing.
- Rule: never two checks at once, and nothing installed twice. A check is
  refused while a check or an install is under way, and once this process
  installed a version (the Tauri `UpdateRun.installed` rule), until the
  restart. A failure leaves the app as it was and the button enabled.
- Rule: who installs. A portable ZIP or a tarball install is downloaded,
  verified and installed without asking (the plan's recorded policy, against
  the guide's consent step), then `ReadyToRestart`, with the pane's note
  naming `Restart Now`. An Inno Setup install gets its setup staged the same
  way and a `.app` install its whole bundle swapped (both below). Every other
  kind, the deb and an unknown copy (a `cargo run`, a `target/` binary),
  stops at `Available` with `Open Release Page` (`cx.open_url`).
  The kind comes from the executable's path only (`update::detect`): an
  `unins000.exe` beside it is Inno Setup, checked before a `licenses/` folder
  beside it, which is the portable ZIP (the installer writes `licenses/` too);
  `<x>.app/Contents/MacOS/<exe>` is a `.app`, `/usr/lib/handpic/<exe>` the deb,
  `<...>/.local/bin/<exe>` the tarball, else unknown. So a `cargo run` or
  `target/release` binary is unknown and never installs. The registry's
  uninstall entry is not used, since it cannot tell a moved install apart.
- Rule: the portable and tarball archive path. The engine's `install`
  takes the executable at a fixed path per install kind (`portable_asset`):
  `Handpic/Handpic.exe` in the Windows ZIP and `Handpic/bin/handpic` in the
  Linux tarball, matched exactly, and writes it onto the running exe's own
  path (`bin_install_path`), so a portable copy whose exe is `handpic.exe`
  keeps that name. Nothing is derived from the running exe's file name
  (`bin_name` is the constant `handpic`), so a renamed copy still updates. A
  2.0.0 portable copy looks for `Handpic/handpic.exe` and fails on a ZIP
  that ships `Handpic.exe`; it is reinstalled by hand.
- Rule: the Inno Setup install's setup. The engine's `stage_setup`
  downloads `Handpic-<version>-windows-x64-setup.exe.zip` of the pinned
  release into `datadir::update_staging` (`update` in the `<IDENTIFIER>`
  folder under the cache dir, emptied first), checks it against
  `SHA256SUMS` and with `self_update::verify_signature`, refusing an empty
  key list (with which that function passes anything), and extracts the
  setup; any failure removes the folder. `ReadyToRestart` then means
  "staged". The view keeps the setup's path in `update::Staged`, shared with
  an `on_app_quit` handler registered after the judgment and settings drains
  (GPUI calls the handlers in registration order), which starts it detached
  (`run_setup`: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /WAITPID=<pid>`)
  on every quit. `Restart Now` sets `relaunch` (adding `/RELAUNCH`) and calls
  `cx.quit()`, not `cx.restart()`, which would start the old exe while the
  setup replaces it. The staging folder of an Inno Setup install is removed
  at launch (a failure, such as the relaunching setup still running from it,
  is logged and left to the next launch). An uninstall leaves no next
  launch, and an uninstall from Settings → Apps left the staged setup and
  its ZIP in `<cache dir>\<IDENTIFIER>\update` (Observed 2026-11-21, rc.2),
  so the uninstaller deletes that folder too, through the one
  `[UninstallDelete]` entry of `handpic.iss`
  (`{localappdata}\dev.minodisk.handpic\update`, which a `datadir.rs` test
  holds to `update_staging`), and keeps the index, the settings and the logs.
  An install gains it once a setup built with the entry has installed over
  it. **Unverified**: the uninstall is not yet checked on a real machine
  (the `todo.md` item "App: the Inno Setup uninstaller leaves the update
  staging folder (native-release)").
- Rule: the `.app` install's bundle. The engine's `install` takes the
  bundle from the executable's path (`app_bundle`, the `<x>.app` of
  `<x>.app/Contents/MacOS/<exe>`) and hands it to `self_update`'s bundle
  mode (`bundle_path_in_archive("Handpic.app")`, `bundle_install_path`, no
  `bin_path_in_archive`, `bin_name` still set to the Cargo bin's `handpic`,
  which names no path) with the portable path's
  `SHA256SUMS`, zipsign keys, tag pin and exact asset name
  (`Handpic-<version>-macos-<arch>.tar.gz`, `arm64` for Rust's `aarch64`).
  `self_update` extracts the whole bundle into a temporary folder beside the
  installed one, runs the `verify_binary` hook on it (`codesign --verify
  --strict`, the ad hoc signature `macos.sh` made) and swaps the two folders
  by renames, putting the old one back on a failed step; the executable
  inside is never replaced alone. Before anything is downloaded the engine
  refuses a bundle under an `AppTranslocation` path (a quarantined copy
  opened from the DMG or Downloads) and one whose folder it cannot create a
  file in (a read-only disk image, another user's folder), with a `Failed`
  message saying to move Handpic.app to `/Applications`; `self_update`'s own
  `AppTranslocated` and `InstallPathNotWritable` get the same advice. An
  explicit `bundle_install_path` is passed, so `self_update`'s own
  translocation check (only on its default path) does not run; hence the
  engine's.
- Rule: the timeouts. The check's requests are bounded by `CHECK_TIMEOUT`
  (30 s) and the install's by `INSTALL_TIMEOUT` (10 min), the engine's: a
  stalled connection fails into `Failed` rather than holding `Checking` and
  so every later check. `ureq`'s per-call timeout is global, the body's read
  included, hence the long one for the download.
  To rehearse an offline launch, set `HTTPS_PROXY=http://127.0.0.1:9`
  on the copy under test: `ureq` 3 reads the proxy variables, so the check
  fails fast (`os error 10061`). A real network cut was not tried. Test copies
  get their own `--settings`, `--index`, `--log` and `--mcp-port` (seeded with
  `--import-settings` of `{"mcpEnabled": true}` so `get_status` shows the
  `update` object), and are quit with `Process.CloseMainWindow` on their own
  PID, never by image name, which could hit the user's own Handpic.
- Rule: the restart. `Restart Now` calls `cx.set_restart_path` with the
  executable `current_exe` gave at launch (on Linux it names the replaced
  file with a ` (deleted)` suffix after the update), or for a `.app` install
  with its bundle, which GPUI's macOS restart `open`s and the swap left at
  the same path, and `cx.restart()`, not
  `self_update::restart`, which would `exec` past the drains. GPUI's
  `restart` spawns a script that waits for this process's exit and quits
  the app (`PostQuitMessage` on Windows, the platform's `quit` on macOS and
  Linux), so the app-level `on_app_quit` handlers run first. Measured on
  Windows 11 (2026-11-19, a debug build whose `--auto` end called
  `cx.restart()` towards `whoami.exe` on an empty folder): `quit: scan
  stopped`, `quit: judgment stores drained=true sidecar writer
  drained=true` and `quit: settings writes drained=true` were logged after
  the restart and the process exited 0. The updates themselves were
  rehearsed on a test feed (`tools/package/test-release.sh`) on Windows 11
  on 2026-11-21: a portable rc.1 replaced its running exe with rc.2, an Inno
  Setup rc.1 staged the rc.2 setup and a plain quit installed it, a stable
  build was not offered the rc, an unreachable feed left the app usable,
  and a tampered asset installed nothing. `Restart Now` relaunched both the
  portable and the Inno Setup copy as rc.2 after the drains, the judgment
  pressed just before it in the sidecar. The relaunch drops the original
  command-line arguments, so a test run started with `--log`, `--index` or
  `--settings` comes back on the default data folders. The macOS and Linux
  rows are recorded as unverified in
  `docs/plans/20261109-gpui-port/packaging-updater.md`.
- Rule: MCP. `get_status` carries the state (`update`: the version, the
  install kind, the phase, the target version, the percent, the message and
  the release page, and whether the build trusts the test feed and key);
  no tool checks or restarts, and the coverage table
  excludes the page for that reason.

### The keymap is edited in the modal and replaced whole (Inferred)

The Keyboard Shortcuts page groups `Keymap::bindings()` by the area an action
acts on, the app page's own grouping (the engine's `DEFAULTS` order is
unchanged): `settingsmodal::shortcut_group` gives each action one
`ShortcutGroup`, an action it does not know (one the engine gained later)
falling back to `General` so no row leaves the page, and the page draws one
`SettingGroup` titled `ShortcutGroup::title` per `SHORTCUT_GROUPS` entry, in
that order (`Filmstrip`, `Judgment`, `Preview`, `Panes`, `General`), each
keeping the keymap's order within it but `Judgment`, which
`settingsmodal::shortcut_rows` orders after the strip menu's judgment items
(`context::judgment_actions`: the flags, 5 stars down to 0 stars, the labels,
as the filter menu lists them), the Judgment actions neither menu holds
(`rejectRest`, `clearall`) after them in the keymap's order. A titled group is also a sub-entry of
the page in the settings sidebar, a click scrolling to it. gpui-component
renders each group under `group-{ix}` and each item under `item-{ix}`, so the
rows' repeated button ids stay unique though the item index restarts in every
group. Each row is labeled by
`settingsmodal::shortcut_label` (`settings.ts`'s `shortcutLabels`), each key a
`Kbd` of `keys::keystroke_of(key)` with a `×` beside it, then `+` or `Press a
key...`, then `Reset`; `Reset all` is the last item, alone in an untitled
group. `Kbd::format` spells the
key: `ctrl+alt+1` as `Ctrl+Alt+1` (`⌃⌥1` on macOS), `arrowleft` as `Left`
(`←`), `shift+x` as `Shift+X`, `escape` as `Esc`, `meta+o` as `⌘O` on macOS;
there is no option to keep the keymap's lowercase names. The empty state's
sentence and the status line's refusals are text and keep the keymap's
spelling (`settingsmodal::display_key`), so the two spellings differ. The view
edits its own copy of the keymap, read from the window on every open, through
`settingsmodal::edit` (`Keymap::add` / `remove` / `reset` / `reset_all`); a
refusal shows the engine's message on the status line and leaves the copy
as it was, a success emits `Change::Keymap`, on which the window replaces
`settings.keymap` and `bindings` (`Bindings::new`) and writes
`Keymap::overrides()` under `shortcuts`, deleting the key when there are none
(`Writes::shortcuts`, the Tauri `update_keymap` rule).

- Rule: the capture sees the raw key in the box's handler. `+` sets
  `capturing` and focuses the box itself, the focus trap's container,
  since the pressed `+` is replaced by the prompt and a focus left on a node
  no longer drawn would leave the settings; a `×` focuses the box after its
  edit for the same reason. The next named key (`key_name`, a lone modifier
  ignored) is added, Escape cancels without closing, and a close cancels
  (`SettingsModal::close`). A page switch is the component's and the view
  cannot observe it, so in its place any pointer press inside the box
  cancels (`cancel_capture`, a capture-phase `capture_any_mouse_down`, which
  runs before the pressed control's own click: a `+` pressed on another row
  then captures for that row). This differed from the Tauri app, where only
  a tab switch or a close cancelled.
- Rule: `tab` and `shift+tab` can be captured. While a row captures, the box
  sets the key context `SettingsCapture` (`settingsview::CAPTURE_CONTEXT`),
  and `main` binds both keys to `NoAction` in it (`capture_keys`, bound with
  `keymap_keys`), Step 5's mechanism for the `Handpic` context: the deeper
  `NoAction` drops `Root`'s `Tab` / `TabPrev`, so the key reaches the box's
  listener and is taken as the shortcut. Enter, Space and Escape reach it
  because the focus is on the box, not on a control, and the box has no
  `Dialog` key context. Outside a capture the context is not set and `Root`
  walks the trap.
- Rule: `settingsmodal::APP_KEYS` (`home`, `end`, `q`) are refused with
  `"<key>" is taken by this app`: `key_down` takes them before the keymap, so
  a binding would never fire. `escape` cannot be captured (it cancels) and
  `ctrl+,` / `meta+,` the engine refuses as a menu accelerator. The folder
  tree's keys (Up / Down, Home / End, Enter, Space, Escape, type-ahead) are not
  refused: they are taken only while the tree has the keyboard, when the
  keymap's photo actions are swallowed anyway (`treekeys.rs`).
- Not verified headless: the clicks, the capture and the names a shifted
  digit and Ctrl+Alt+digit arrive under ("Every keystroke goes through the
  keymap"); the hand check is pending in
  `../plans/20261113-gpui-settings/learnings.md`.

### A sidecar format switch keeps the Tauri order (Inferred)

The Sidecar page's three radios (`settingsmodal::FORMATS`) emit
`Change::SidecarFormat` only for a format other than the one shown
(`format_change`); the window's `switch_format` sets `settings.format` at
once, so the next judgment, from a key or the MCP companion's
`set_judgment` alike, is queued in the new format. Then, on the background
executor and under the view's `format_switch` lock (one switch at a time), it first checks
its format against `format_latest`, the one last asked for, written on the main
thread, and does nothing when a later click superseded it, so back-to-back
switches end persisted as the radio shows (the Tauri `AppSwitchLock` got the
same by re-checking the current format inside the lock). Otherwise it drains the judgment stores
(`judgment::Queue::drain`: a store queued before the switch carries the old
format and must reach the writer before its flush) and runs the engine's
`sidecar::switch_to`: the writer's flush (`sidecar::DRAIN_TIMEOUT`),
`sidecarFormat` persisted through `Settings::writes`, `Index::reset_sidecars`.
The Tauri `switch_format` called the same `switch_to` after its own state
write, in the same order. Back on the main thread
it clears `touched`, the undo histories and the step note, as the Tauri
reopen (`openDirectory`) cleared `touched` and the judgment entries of its
histories, and re-lists the open folder with `relist`, whose
`sidecar::reconcile_of` runs only after the reset. A failure goes to the
modal's status line (`SettingsView::failed`) and, as in the Tauri app, the
folder is not re-listed.

- Rule: a listing already under way when the format changes reconciles with
  the format it captured at its start; the switch's own re-listing comes
  after the reset and supersedes it (`opens` token), so the view ends on the
  new format's judgments.
- Rule: a label names change takes the same path (`switch_names`, both
  through `reset_sidecars` under the `format_switch` lock). While the format
  writes XMP the Sidecar page shows the five name fields
  (`Input`) and a `Select` of `i18n::presets()` languages; a
  field's commit (Enter or blur; Escape commits and closes, as the Tauri
  field's `change` fired when the closing dialog takes its focus) or a
  language's choice normalizes the five with `sidecar::label_names_setting`, shows them back
  and emits `Change::LabelNames` only when they differ. The window then
  writes `labelNames` and puts the names in force on the main thread (a
  judgment queued before keeps the names it carries; overlapping changes end
  stored as the fields show, so no `format_latest`-style guard), and off it
  drains the stores and runs `sidecar::switch_to` with nothing left to
  persist, then re-lists.

### The Import page stores a root and a valid pattern, nothing reads them yet (Inferred)

`Settings` reads `importRoot` and `importPattern` with the engine's
`import::root_setting` / `pattern_setting` (an invalid stored pattern reads as
`%Y/%F`). The Import page shows the root in a read-only field with `Choose…`
(one `InputGroup`, see "The settings modal is its own entity"), the
`importsettings.ts` texts (`settingsmodal::IMPORT_NOTE`, `root_note`,
`IMPORT_PATTERN_EXAMPLES`, `IMPORT_SPECIFIERS`), the pattern in a
`Input` field and the example line, `import::today_example` (the
app crate has no `chrono`), recomputed on the field's every
`InputEvent::Change` (and after an example button's `set_value`), so with no
debounce; a refusal shows in the error color. A
commit (Enter, blur, Escape then close) or an example button goes through
`settingsmodal::pattern_change`: a valid new pattern emits
`Change::ImportPattern`, which the window writes and keeps in `settings`; an
invalid one leaves the stored one and shows its reason.

- Rule: `Choose…` calls `cx.prompt_for_paths` (directories only) from the
  view and awaits it in a `cx.spawn_in` (the root field's `set_value` needs
  the window); the answer comes on a later turn and the
  view entity lives as long as the window, so `Change::ImportRoot` is emitted
  and written whether or not the modal is still open. It sets only the root,
  so a pattern still being typed stays in the field. A dialog error (Linux)
  goes to the status line.
- Not verified headless: the OS dialog, the typing and the buttons; the hand
  check is pending in `../plans/20261113-gpui-settings/learnings.md`.
  `Import…` itself is port Step 8's.

### The app's index (Inferred)

The index lives at `<dirs::cache_dir()>/dev.minodisk.handpic/index.sqlite`;
`--index <path>` overrides it. The folder is named after the Handpic
identifier (`datadir.rs`'s `IDENTIFIER`, next to `LEGACY_IDENTIFIERS`).
An index an earlier build kept under another folder is not picked up; pass
1 rebuilds it. An index that cannot be opened leaves the pager working with
`no index: …` on the status line.

- Rule: never point a debug or measurement run at the user's index; use
  `--index` with a fresh file.
- Rule: `reconcile` drops every row of the folder it was not given, so it
  always gets the whole listing; `--size` / `--max` narrow only what is shown.
- Rule: on Windows `canonicalize` gives `\\?\D:\...`, and every index key and
  listed path in the app carries that prefix, as in the Tauri app. Key
  anything compared with the index or the scan's `ready` paths (strip cells,
  thumbnails, the watcher's owner string) on `FileStat.path.to_string_lossy()`
  or the canonical `PathBuf`, never on a path rebuilt from the user's argument.

### `run_scan` runs on a `std::thread` (Inferred)

`index::run_scan` blocks for the whole pass and has its own rayon pool. On
`cx.background_spawn` it would hold one of GPUI's background threads, which
the preview decode needs, for as long as the folder takes. `scan.rs` spawns a
thread per scan; a new open cancels and joins the previous one, and every
listing waits for the launch's eviction thread before it reconciles
(`scan::Eviction`, a shared handle the first waiter joins under its lock, so
a listing whose task was dropped before the join leaves the next one
waiting), so the `VACUUM` never races it.

### The hot list is set before the scan's workers start (Measured)

`run_scan`'s workers take the first file of the hot list not yet taken, else
the next in listing order. With the hot list sent right after `Scan::start`
(as the Tauri frontend did after `start_scan`), the first wave of the 22
workers had already taken files 0-10 of a 373-file folder opened at file 199;
with `ScanFocus::set` called on the handle before `Scan::start`, the first
committed batch was the 20 files of the strip's range around 199 plus 0 and 1
(the two workers left over). Later updates are debounced by
`SCAN_FOCUS_DELAY` (a `cx.spawn` timer task replaced on each change; dropping
a GPUI `Task` cancels it).

- Rule: an index write from the view (`set_last_viewed`, `mark_viewed`) goes
  through the writer on `cx.background_spawn`, one in flight (`viewed.rs`),
  the values pushed meanwhile kept per folder (`Latest` the latest file of
  each, `Batches` the files of each), so leaving a folder while a write is
  out does not lose its position; a failure is logged, never shown.
- Rule: `Index::evict` drops a folder whose `opened_at` is over 30 days old
  with its `last_viewed`; seeding a test index by hand needs a current
  `opened_at`.
- Seen (2026-11-27, Windows 11, through MCP): on fresh indexes over 1520 and
  1545 ARW hardlinks, pass 1 at about 120-150 files/s, a `navigate last` at
  `scanning 1/1545` left the ten cells in view gray at 0.5 s, nine of ten
  filled at 1.2 s (`scanning 124/1545`) and all at 2.9 s; a `show_photo` jump
  to the middle at `scanning 455/1520` had its cells filled at 1.0 s with
  pass 1 at 650/1520; on an open, seven of the ten cells in view filled at
  0.9 s (`scanning 59/1520`) and all at 1.4 s, not strictly from the current
  cell outward, since parallel reads finish out of order.
  `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`.

### A preview holds the scan's pause until its image is painted (Measured)

`load` takes `ReadSlots::pause` before the read and hands the guard back in
`Loaded`; the view keeps it in `PaintHold` (`hold.rs`) and the preview
canvas's paint releases it right after painting the image. Pass 2's workers
wait out a held pause at every stage boundary (`scan::yield_to_pause`), not
only before a read, so the read, the decode, the orient and the paint get the
cores. `MAX_PAUSE` (1 s) still bounds what a worker honours: one budget for all of a
pass 2 file's stage waits, and one per `acquire` (two per pass 2 file, one per
pass 1 file), so a hold never released (no frame painted, say a minimized
window) costs a worker about 3 s per pass 2 file or 1 s per pass 1 file at
worst and only slows the scan. The longer hold also blocks pass 1's `acquire`
from the read through the decode, orient and paint; its effect on pass 1's
throughput was not measured (the pass 1 paging measurement below assumed a
pause around the read only). Holding the pause through the decode (not
yet the paint) cut RAF's A - B from +5.08 to +1.94 ms at 3 pass 2 threads, the
`decode` growth gone, without fewer pass 2 rows per run
([results](../plans/_archived/20261110-gpui-focus-marks/results.md)).

- Rule: anything new on the preview's path between the read and the paint
  runs while the pause is held; keep it there rather than after the release.
- Measured on Windows 11 on 2026-11-23, on NVMe only: the pause made no
  measurable difference to the read of a file pass 1 had not read. Release
  builds of `3918064c` at the shipped cap 2, pause on (unpatched) against
  pause off (`load` pausing a throwaway `ReadSlots` instead), alternated, a
  cold `robocopy /J` copy of 1800 α7 V ARW on the NVMe `CT1000P5PSSD8` per
  run; the reads are the `page` lines' `read` of MCP `show_photo` jumps to
  files at least 100 past pass 1's done count, 40 apart, during pass 1
  (the method of `shared-crates.md`'s read cap section, whose cap 2 rows are
  the pause-on runs here). Other sessions were compiling; the CPU load
  sampled before each launch:

  | Pause | CPU before | Probes | Read p50 (ms) | Read p95 (ms) | ms/file |
  | ----- | ---------- | ------ | ------------- | ------------- | ------- |
  | on    | 100 %      | 28     | 14.6          | 39.2          | 11.09   |
  | on    | 37-43 %    | 18     | 11.0          | 13.6          | 5.50    |
  | on    | 9-10 %     | 16     | 8.8           | 10.2          | 4.19    |
  | on    | 12-14 %    | 16     | 9.6           | 15.1          | 4.46    |
  | off   | 21-24 %    | 14     | 8.3           | 9.4           | 4.01    |
  | off   | 64-92 %    | 23     | 16.3          | 21.7          | 6.78    |
  | off   | 11-12 %    | 16     | 9.3           | 17.3          | 4.26    |

  At about 10 % load the read p50 was 8.8 to 9.6 ms with the pause and 9.3
  ms without; the spread follows the machine's load, not the pause. Tauri's
  figure (14-16 ms p50 at cap 2 with the pause) was taken on the SATA SSD.
  The SATA runs could not be
  taken that day: another process was reading `D:` at 250 to 480 MB/s
  throughout, and the two runs made under it (26.6 ms p50 pause on; 47.5
  ms/file and only 2 of 36 jumps landing pause off) are not comparable.
  Whether the pause shortens the read on SATA is still open
  (`../plans/_archived/20261123-todo-measurements/learnings.md`).

### A held key draws each stale preview before chasing the current file (Measured)

The pager keeps one preview read in flight. When one read + decode outlasts
the key repeat, every result during a hold is another file's; throwing those
away left the fit view blank until the key was released. `Pager::finish`
now decides what a result leads to: the current file's is shown (or its
failure) and clears the in-flight marker; another file's is drawn when its
read succeeded and the current file's image is not already on screen (A shown,
press to B, back to A before B arrives: drawing B would flash the wrong file),
and then the current index is requested, staying in flight. A stale failure is
neither shown nor printed. The drawn frame therefore lags the cursor by at most
one load, the files in between are still skipped, and the number of reads per
key sequence is unchanged.

A stale frame clears the status note and sets `self.shown`, the image,
`marks.show` and the paint hold, and nothing else. `self.shown` names the file
whose image is on screen because `refilter` re-requests when it is not the
current file, and the meta pane follows it.
`marks.show` follows the image too: `Marks::mark`, `wanted_faces` and
`wanted_eyes` draw and ask only when the shown file is the current one, so a
stale frame gets no `f` overlay; left at the previous file, stepping back onto
it while a load is out would put its marks on the stale image. `zoom.shown` is
the current file's only, since it stamps the facts with the current page's
`seq`, so `zoom.preview()` stays `None` for a stale frame and the 1:1 canvas
never uses it as the placeholder. The fit view's peaking layer is keyed by the
image id and drawn size, so a stale frame gets its own edges or none.

- Rule: the on-demand analysis (`analyze_current`), the dwell and viewed mark
  (`start_dwell`), the keypress (`self.key`) and the latency `page` line
  (`self.pending`) are the current file's result's only. A `page` line is
  still recorded only for the current file, so the "4-6 pages in 5 s" of a
  held RAF key below counts current-file pages, not frames drawn.
- Rule: stepping back onto the file whose stale frame is on screen still
  reloads it; its analysis, dwell and `page` line wait for that reload. A
  prefetch ring hit is not a stale frame: it has had all of them, so
  `Pager::hit` makes the older result still out `Finished::Dropped` (released,
  nothing requested, nothing left in flight) until the current file changes.
- Measured on Windows 11 on 2026-11-22: a held `→` (key-downs every 33 ms)
  flipped frames on RAF files with files skipped and some `page` lines
  waiting on the read in flight, a stale frame was drawn with no `f`
  overlay, and the release showed the current file with its marks (face box
  and mesh on a face file), its meta pane and the last `page` line
  (`docs/plans/_archived/20261122-windows-device-checks-8/results.md`).

### A held key draws each stale crop before chasing the current one (Measured)

The 1:1 view's crop chain has the same shape: one read in flight, and a crop
read for an earlier page (`seq`) used to be released. `Zoom` now records the
page each held crop was read for. While zoomed, `page()` keeps the held crop
on screen and asks for the new page's (once, while nothing is in flight); a
stale crop that arrives is held in place of the previous one and the current
page is asked for, so the drawn crop lags the cursor by at most one crop read.
A stale failed read holds nothing new, shows no error and still asks for the
current page; the current page's failure shows its error and releases a held
crop of another page. Not zoomed, a page and a stale arrival release the crop,
so `z` never opens on another file's crop.

- Rule: the canvas never mixes two files. `zoom_paint` draws the held crop
  whatever page it is for, but the placeholder (and the `full` size that
  scales it) only when `Zoom::placeholder_fits`: no crop held, or the held
  one is the current page's. The crop's peaking layer is keyed by its own
  image.
- Rule: the `zoom` timing line, the resize re-cut and the reuse of a held crop
  on `z` or a settled viewport (`toggle`, `settled`) are the current page's
  crop's only; a stale held crop never counts as already having it.
- Rule: a single press while zoomed goes old crop, new crop (no black frame or
  placeholder in between), and a folder switch or re-listing while zoomed
  keeps the old crop until the new one arrives.
- Measured on Windows 11 on 2026-11-22: with crop reads of 51-62 ms against
  the 33 ms repeat, the crops changed during the hold with files skipped,
  every frame one file's crop or placeholder whole, and the release showed
  the current file's crop; a single press went old crop at 37 ms to new crop
  from 85 ms with no black or placeholder frame, and `z` off / on reused the
  crop with no new `zoom` line
  (`docs/plans/_archived/20261122-windows-device-checks-8/results.md`). That
  last part no longer holds: a `z` that shows a held crop now prints one (see
  the next section).
- Measured on Windows 11 on 2026-11-27 (debug build, through MCP): zoomed on
  400 ARW, six series of 120 `navigate next` at about 37 ms and one of 400
  `previous`, 1120 zoomed pages, `PrivateMemorySize64` went 242 MB before,
  203-210 MB after each short series and 221 MB after the long one, no
  climb. The 418 to 557 MB growth of the `gpui-zoom-peaking` smoke run did
  not repeat.
  `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`.

### The 1:1 crop is read ahead of `z` (Measured)

`Zoom::prefetch` reads a crop before it is asked for, under the prefetch
ring's gate (`prefetch_open`: the ring on, the current file's preview on
screen, no page read out, no scan pass, no folder rename), from
`request_prefetch` and after every crop that lands. Not zoomed, it reads the
current page's crop once `zoom.shown` has stamped the page, so `z` paints a
crop already held (`Toggled::Held`). Zoomed with the current page's crop held,
it reads the next neighbour's (the first of `prefetch::neighbours` in the
`heading` direction, which keeps the paging direction across a wrap) into a slot of its own keyed by the file
(`Request::ahead`, `ahead_arrived`); `page` makes it the held crop when the
new current file is that one at the same viewport, so the page shows it
without a read, and a page onto it while it is still in flight takes it when
it lands. One read is in flight across both, and each slot holds one crop.

- Rule: every `z` that turns the view on over a held crop of this page and
  viewport prints one `zoom` line (`Timed::of`, the crop's own `read`,
  `decode` and `orient`, which a read ahead paid before the press), and a `z`
  pressed while a read is in flight is kept (`waiting`) and handed to the
  `Shown` of this page's crop, or to the current page's read a stale or
  neighbour arrival asks for, so it prints one too. A page drops a waiting
  press.
- Rule: a crop read ahead goes when it no longer matches: the neighbour's on
  any page that does not land on it at the same viewport, on a re-listing
  (`relisted`, from `clear_ring`, which also voids a read in flight for it)
  and when the settled viewport differs (`fit`, from `fit_settled`); not
  zoomed, the current page's goes the same way. Zoomed, a re-listing keeps
  the current page's crop, as before.
- Rule: a neighbour's crop that lands after a page landed on that neighbour
  and left it (`Flight::passed`, set by `page` when the current file is the
  read's neighbour) is not parked in the slot: zoomed, it is held as a stale
  crop under the seq it was asked on, like `arrived`'s zoomed-stale branch,
  so a held key still draws each crop it paged through before chasing the
  current page. When no page landed on the neighbour (a reverse key, a
  click, a refilter) and the page has moved, the crop is released and the
  held one stays.
- Rule: a failed read ahead shows nothing. Not zoomed it is `Arrived::Missed`
  and `failed` stops a retry until `z`, which reads again and shows the error
  as before; a neighbour's failure is remembered (`ahead_failed`) until a
  re-listing, and a page onto it reads it as the current page's.
- Rule: a crop read for an earlier page while not zoomed is released on
  arrival even if `z` came since (`Flight::zoomed`), so a fresh `z` never opens
  on another file's crop.
- Rule: `--no-prefetch` turns the read ahead off with the ring, so a
  same-session A/B of the 1:1 view is a `--no-prefetch` run against a
  default one.
- Measured on Windows 11 on 2026-11-24 (release build, shared machine, α7 V
  ARWs, `z` once per file): `keypressToPixels` p50 11-13 ms and p95 under
  19 ms on both the deep-row and the shallow-row set, 0 of 80 presses over
  50 ms, against 44-50 / 54-65 ms on the deep set under `--no-prefetch` in the
  same session; the `page` lines of single and held `→` presses were not
  slower (see "Reading the crop ahead of `z` (Windows 11)" in
  `docs/humans/performance.md` and
  `docs/plans/_archived/20261123-close-measured-todos/learnings.md`).

### The prefetch ring holds the decoded neighbours (Inferred)

`prefetch.rs` ports `prefetch.ts`: `Ring` holds the decoded previews of the
current file's two neighbours (the next one in the paging direction first,
wrapping at the ends of the list as the pager's `Next` / `Previous` do, never
the current file and no file twice), at most `PREFETCH_PIXEL_CAP` (32 MP,
128 MB of BGRA) in all, the image on screen not counted. `request` takes the current
file from the ring before it reads; a hit does everything `loaded`'s current
arm does (`show_current`: the cleared status note, meta pane, `marks.show`, `zoom.shown`,
`analyze_current`, `start_dwell`, the `page` line with `source=prefetch`) but
the read and the paint hold. `request_prefetch` reads the next wanted file,
one at a time through `load` on the background executor, only when the pager
has nothing in flight, the image on screen is the current file's, no scan
pass runs and no folder rename of the open folder or an ancestor is in flight
(`renaming`, tested by the tree's spelling `tree_open`, which the in-flight
renames hold, else by the canonical `dir`); it is asked after a
current result (read or hit), a settled prefetch, a `refilter` that leaves the
current file shown, `FacesDone` and a failed folder rename (`rename_ran`'s
`Err` arm; a successful one reopens the folder, which refills the ring).
`--no-prefetch` turns the ring off, and the 1:1 crop's read ahead with it. A ring miss on a JPEG-only folder is the
number the ring hides: with `--no-prefetch` the 50 MP Leica exports take
about 115 ms p50 per page (`decode` about 100 ms at 1/8 scale) and the 26 MP
Sony exports about 50 ms, against 9-13 ms p50 with the ring on at one press a
second. Fancy upsampling off plus `DctMethod::IntegerFast` did not cut
`decode` (96.62 to 99.87 ms median), because at 1/8 and 2/8 the time is the
entropy decode, so do not retry decoder settings for it
(`docs/plans/_archived/20261123-todo-four-closeouts/learnings.md`, Step 4).
`PREFETCH_PIXEL_CAP` counts decoded
pixels, which are about the canvas's size since the fit view decodes at a
reduced scale (see "The fit view decodes at the canvas's DCT scale" below).

- Rule: an image is owned by one place at a time: the ring, or the screen.
  `take` hands it to the view; `present` hands the image it replaces back
  through `keep`, and only after another image replaced it. Only what a ring
  method returns is retired, so an image in the ring never reaches `retired`
  (whose `cx.drop_image` would pull its texture).
- Rule: an entry handed back through `keep` keeps its GPU texture; it is not
  released on `keep` and uploaded again on the next hit, a decision for
  paging speed over memory (a hit's `paint` stays flat, at the cost of the
  private bytes measured in "A prefetch hit leaves only the frame").
- Rule: `opened` (a folder open and a re-listing, which may have changed any
  file's bytes) clears the ring and marks the shown image not keepable;
  `refilter` calls `retain` with the new `files`. Whichever plan adds rename,
  trash or delete (`../plans/20261113-gpui-tree-trash/plan.md`,
  `../plans/20261113-gpui-folder-dialogs/plan.md`) must call `Ring::forget`
  with the paths that no longer name what the ring read.
- Rule: a prefetch's result is dropped by the ring's own stale mark, not by
  `order`; its `Paused` ends when it lands, never held for a paint.
- Rule: the page bump (`zoom_page`) comes before `request` in `go` and in
  `refilter`'s `moved` branch: a hit shows the image synchronously, and
  `zoom.shown` stamps the facts with the `seq` current then.
- Rule: what the ring gives up is retired with a `cx.notify()`: `retired`
  drains only in the preview canvas's paint, so an eviction on `settle` with
  nothing else to draw would otherwise hold the images past the cap.
- Rule: a ring method used only by its own tests (for example `Ring::forget`
  before a plan calls it) cannot carry a bare `#[expect(dead_code, ...)]`.
  Under `cargo test` the lint does not fire, so clippy's `-D warnings` fails
  it as an unfulfilled expectation. Use
  `#[cfg_attr(not(test), expect(dead_code, reason = ...))]`, as `settings.rs`
  does.
- Pending: an arrow key onto a neighbour showing it at once, and a folder
  switch and a filter change showing no other folder's or hidden file's image,
  are unverified by hand on Windows.

### Pass 2 runs on pass 1's thread (Inferred)

`Scan::start`'s thread runs `index::run_faces_scan` over `faces_todo` right
after `run_scan`, with the same `ScanFocus` and cancel flag, as the Tauri
app's `run_faces_pass` did on its one task, but on `faces_threads()` (an
eighth of the cores) rather than pass 1's `scan_threads()`: on 22 threads it
cost paging +9.4 ms on ARW and +89 ms on RAF at p50, and on a quarter (6)
still +13 ms on RAF
([results](../plans/_archived/20261110-gpui-focus-marks/results.md)).
One thread and one flag mean `Stopping::stop` (and the quit's bounded
`stop_within`) cancels and joins whichever pass is running, and the hot list reaches both passes through the one handle
(`focus_scan` gates on either pass running, as `scan::Status::scanning` tells,
cleared as soon as a re-listing takes the scan, whether or not a new one
starts). The thread starts on every open
with an index, even when pass 1 has nothing to do, so rows below
`FACES_VERSION` still get pass 2; a no-op pass costs one `faces_todo` query.
`Event::FacesDone` is sent also when the pass was canceled, had nothing to do
or `faces_todo` failed, so the view always learns pass 2 is over. The first
non-empty batch's file names go to the log as `scan faces first:`.
`FacesDone` carries how many files pass 3 runs on (`allfaces_todo` at
the group threshold `Scan::start` was given, the `groupMinFaces` setting
when the scan started, listed after pass 2 and skipped once canceled): with none
it ends the scan and no pass 3 starts, else the status line turns to
`checking faces 0/N` and `Event::AllFacesDone`, also sent on a cancel, ends
it.

- Rule: the deferred resync (`finished`) runs at the scan's end (`FacesDone`
  with no pass 3, else `AllFacesDone`), not at pass 1's
  `Done`, as the Tauri frontend drained its deferred rescan at `faces-done`. A
  re-listing during pass 2 would be harmless (a canceled file keeps its old
  `faces_extractor`, so the next scan resumes it) but would restart the pass;
  waiting for its end keeps a burst of changes from restarting it over and
  over.
- Rule: pass 2 writes its rows in `BATCH` (10) transactions, so a folder of
  fewer than 10 files shows its pass-2 columns (`Analysis`, face icons) only
  at `FacesDone`.

### The engine logs through `log`, printed by `logger.rs` (Hit)

Without a logger the engine's `scan extract` lines went nowhere. `logger.rs`
prints `[target][LEVEL] message` to stderr at `Info`, which also shows GPUI's
own lines (the UI font, the GPU, and an `ERROR` for the missing DXGI debug
interface on a machine without the Graphics Tools, which is harmless). A
Windows release build is a GUI-subsystem binary and has no stderr unless it
is redirected to a pipe or a file (see "A Windows release build is a
GUI-subsystem binary"); its writes go nowhere and `Handpic.log` is the only
record.
`RUST_LOG=debug` (or `trace`) adds this crate's own lines of that level, such
as `key: <name> action=…` for every keystroke, and not GPUI's.

`logger.rs` also writes each line, as
`[YYYY-MM-DD][HH:MM:SS][target][LEVEL] message` in UTC, to a `Handpic.log`
under the app's own folder, or to `--log <path>` (see
`app-log.md`'s "Size and rotation"), with one `write` per line on a
`Mutex<File>` opened for append and the size kept in memory; past 1 MB it
starts anew as the Tauri app's did: `File::create` (truncate) and a replaced
handle, never `set_len` or a remove on the open file, because on Windows a
file open for append cannot be truncated by `set_len` and a remove while the
handle is open can leave the name pending deletion. The timestamp is a
hand-written civil-from-days, not `chrono` / `time`, so the app lock does not
grow (turning on the features a direct dependency needs could pull further
crates in). `Handpic::start` destructures `Args` exhaustively, so a new flag
needs a `name: _` there too. `main` installs it once the arguments
are parsed and the engine's panic hook right after it. The `page` / `zoom`
timing lines stay `println!` to stdout. The logger's own stderr writes are
`let _ = writeln!(io::stderr(), ..)`, so a missing or broken stderr never
panics it.

## GPUI

### Callbacks from other threads go through one channel (Inferred)

`AsyncApp` is not `Send`, and a `BackgroundExecutor` task cannot update an
entity, so a callback on a rayon or watcher thread cannot touch the view. The
view owns one `futures::channel::mpsc::unbounded` sender; the callback only
sends, and a `cx.spawn` task drains the receiver with `this.update(...)` per
message. Each scan message carries the scan's id, so a stopped scan's late
messages are dropped.

- Rule: a callback from a non-GPUI thread only sends into that channel.

### `RenderImage` holds BGRA and must be released (Inferred)

`RenderImage` holds BGRA in an `image::RgbaImage`. Every image handed to the
renderer must be released with `cx.drop_image(image, None)` or it leaks on the
GPU.

### Textures have one mip level (Inferred)

A large downscale (a RAF fit to a 1600 px window) may alias. Not settled: one
captured 1:1 crop of a 4416x2944 RAF fit view, a clapboard wall without fine
repeating texture, shows no moire, but no comparison with the Tauri app or a
hard-case image has been made. Three app-side RAF frames showed no moire;
the comparison with the Tauri app was left pending (see the pending manual check in
the gpui-port plan's `learnings.md`).

### The image sprite cannot rotate (Inferred)

`PolychromeSprite` has no `transformation`; only the monochrome SVG sprite
does. A rotation has to happen on the pixels before painting.

### `render_to_image` captures the window behind the `screenshot` feature (Measured)

`Window::render_to_image` exists only under gpui-pre's `test-support`. The
`screenshot` feature turns it on through an optional direct
`gpui-pre-platform` dependency with `test-support` (which carries it to
gpui-pre and gpui-pre-windows), not through `gpui-kit/test-support`: that
one's weak `gpui-fast?/test-support` makes Cargo resolve the optional
gpui-fast crates, whose `wgpu = "=30.0.1"` conflicts with the locked 30.0.2.
gpui-base's `test-support` (its `Observed` element wrappers) stays off.
`Cargo.lock` gains the crates the feature names (proptest, postage,
futures-concurrency, and criterion and the rest through the bench-metrics
weak feature), compiled only in the feature build.

It re-renders `rendered_frame`'s scene into the live render target, copies it
to a staging texture and skips `present`; the next present re-renders the
whole scene, and no stale or blank frame was seen on screen after captures
(the window restored, uncovered, re-grabbed from the screen). On Windows it
is implemented (D3D11), on macOS through Metal (it captured through MCP on
a Mac, arm64, on 2026-11-26), on Linux
it fails with "not implemented", which the tool returns as its error; a lost
device is an error too.

A next-frame callback runs before that frame's draw, so the view's
`screenshot` registers one inside another: the inner one captures after the
frame the outer one let through, and a previous tool call's change is on the
image (checked with `set_overlays` then `screenshot`). On Windows a covered
window still draws and the capture is current. On macOS, once, a window
behind other apps' windows returned a frame that lacked a tree expand until
the window was brought to the front (2026-11-26,
`docs/plans/_archived/20261126-macos-device-checks-mcp/learnings.md`), so
bring it to the front before a capture there. A minimized one draws no frame, so after
`SCREENSHOT_FRAME_WAIT` (500 ms) the last drawn frame is taken as it is and a
change made while minimized is not on it. The image is in device pixels, the
render target's size.

Measured in a debug build on a 5058x1440 maximized window: the capture about
300 ms (two frames and the unoptimized BGRA-to-RGBA loop), the whole call
360-430 ms at the default 1568 and 620-720 ms at 4096, within the bridge's
5 s. Unoptimized, `imageops::resize` instantiated in `handpic-app` and the
PNG encode took 4.1 s at 4096; the resize now goes through `DynamicImage`
(compiled in the optimized `image` crate) and the encoding crates are in
`[profile.dev.package]`.

### The main view opens on GPUI's test platform (Measured)

`uitest.rs`'s `Ui::open` builds the real `Handpic` view on GPUI's
`TestAppContext` (`VisualTestContext` as its window handle) through
`open_main`, the function `main` calls too: `gpui_kit::init`, the key
bindings, `theme::apply` and `gpui_kit::open_window`, which wraps the view in
`Root`. The window used to be opened inline in `main`'s `run` closure; that
was the only thing in the way. `main` keeps what only a launch does: the
command line, the logger, the panic hook, the app identity, the quit on the
window's close, the assets and `activate`. `Ui` gives `open_main` an
`open_index` and a `settings::load` over files in a temporary folder and
`Args::default()`, so `remember` is on against that temporary store, no MCP
thread starts and no folder opens.

`TestAppContext` needs gpui-pre's `test-support`. `gpui-kit/test-support`
fails to resolve (the gpui-fast pin, see the `render_to_image` entry), so the
dev-dependency is `gpui-pre-platform` with `test-support`, as the
`screenshot` feature's; `Cargo.lock` does not change. `cargo test` and
`cargo clippy --all-targets` build the GPUI crates with it, `cargo run`,
`cargo build`, the release build and the packages without. gpui-base's
`test-support` stays off, so the tests draw the same elements as the app.
The test app has no asset source (`Icons` is given to `Application` only), so
the SVG icons do not load there.

The test platform runs the foreground and background executors, its timers
included (`run_until_parked` advances the clock to the next timer). The
view's eviction, watchers, sidecar writer, judgment queue, settings `Writes`
and scan run on `std::thread`s that it does not run; their results reach the
view through its channel. `Ui::open` calls `allow_parking`, and
`Ui::wait_until` runs `run_until_parked`, checks a condition on the view and
sleeps 10 ms, failing after 10 s. `Ui::open` installs a test-only `log`
logger once per process, which keeps every line stamped with the time since
the first `Ui`, so a timed-out `wait_until` puts the view's state and the
app's log lines naming its temporary folder (`scan list:`, `scan extract:`,
`scan faces:` with their times) in its panic message. `read_roots` lists the real home directory
and volumes: assert nothing about the tree's roots.

`Ui`'s drop releases its view handle; while the test is panicking it only
cancels the view's scan (the `scan::Running` it cloned at the open), so a
timed-out test leaves no pass 2 worker running next to the later tests.
Otherwise it calls `TestAppContext::quit`, which
runs the quit handlers (the scan stop, the stores and the writer drained, the
settings writes drained): about 0.5 ms with nothing queued, asserted under
1 s, below every drain's 2 s timeout. The writer and settings threads close
their SQLite files only after the senders the view held are dropped, a little
after the quit, so the drop retries removing the temporary folder (gone
after about 11 ms, one retry) and fails the test if it is still held after
10 s; nothing is written outside it. The smoke test passed 25 runs alone and
20 runs of the whole crate's tests on Windows.

Each `Ui` also runs a watchdog thread (`Watchdog`) from `open_with` to the
end of its drop, `cx`'s own drop included (the field declared last). The
harness sets its phase at each stage of the open, in `wait_until` and
`update`, at the quit, the folder removal and the context's drop, and on
return from those calls to `test body, after ...`. A phase older than
`PHASE_MAX` (60 s) writes the phase, its time and the `Ui`'s log lines with
`std::io::stderr().write_all` (libtest's capture is lost on an abort) and
calls `std::process::abort()`, so a hang inside a GPUI call or on a lock
ends the test binary in about a minute and names where it stopped, instead
of the CI job's 30-minute limit with no output.

The watchdog's `on_fire` receives only the phase line (`watchdog_report`,
pure in the label, the elapsed time and the limit). The production closure
(`abort_with`) writes and flushes that line first and only then calls
`log_lines`, which takes the `LOG` mutex, so the phase still reaches stderr
when the hung thread holds that lock. `Ui::read` and direct `ui.cx` calls are
not instrumented: a hang inside one is reported as
`test body, after <last phase>`.

- Rule: do not treat a `the quit handlers took` 1.5 s to 4.0 s failure
  against `QUIT_MAX` in `ci:local`'s whole-crate parallel run on a shared
  host as a regression of the quit path. It flaked before the watchdog, and
  the UI tests alone and a rerun passed. Run the UI tests alone first. See
  [the watchdog's learnings](../plans/_archived/20261128-uitest-watchdog/learnings.md).

- Rule: build a UI test on `Ui`, wait on the app's threads with
  `wait_until`, never a fixed sleep, and drop the `Ui` to end it.
- Rule: a UI test that times out is diagnosed from its log lines, not by
  raising the 10 s wait. On the macOS CI runner five tests of six used to
  time out with pass 2 at `analyzing 0/N`: its `QOS_CLASS_BACKGROUND` made
  one tiny DNG's analysis 6 to 14 times slower than at normal priority and
  pass 2 of 2 to 4 files take 15 s to 44 s. The app's pass 2 now runs at
  `QOS_CLASS_UTILITY` with a relative priority of -5 on macOS (see
  `docs/agents/shared-crates.md`, "The scan pool's priority levels").
- Rule: the UI tests' scans run at normal priority. `Ui::open` sets the
  view's `scan_priorities` (`scan::Priorities`, `APP` in the app) to
  `Priority::Normal` for both passes. At the app's `Priority::Lowest` pass 2
  starves behind the crate's other tests running in parallel on every OS: on
  the Windows CI runner (`THREAD_PRIORITY_LOWEST`) one test's pass 2 had done
  3 of 4 files after 10 s, where it normally takes about 0.7 s for 3. The
  tests check the wiring, not the scheduling. Do not raise the wait instead.
- Rule: the tract inference crates stay at `opt-level = 3` in
  `[profile.dev.package]`. Unoptimized, pass 2 ran about ten times slower,
  and on a 4-core CI runner (`faces_threads` giving each scan one pass 2
  thread, about ten UI tests scanning at once) the 4-file
  `a_filter_choice_narrows_the_files_and_lands_on_the_anchor` timed out
  intermittently at `analyzing 3/4` on Ubuntu and Windows. Before: pass 2 of
  4 files took 1775 ms for one test alone, and pinned to 4 cores every UI
  test's folder wait was 5.0 to 7.3 s. After: 182 ms alone, the longest
  folder wait 0.82 s pinned to 4 cores and 1.19 s pinned to 2. Do not raise
  the wait instead.
- Rule: non-test code must not use a test-only GPUI API. `cargo test` and
  `clippy --all-targets` see `test-support` and pass it; only a plain build
  (`cargo build`, the `Package` workflow's release build) fails. The macOS
  CI job's bundle step builds with `screenshot`, which turns `test-support`
  on too (see "The macOS bundle step reuses the test build"), so it no
  longer catches this.

### The UI tests drive the view with keys over generated DNGs (Measured)

The UI tests in `uitest.rs` check the wiring between the pure modules, not
their rules, which those modules' own tests cover: a key through `key_down`
and the keymap to `perform` or `judge`, a judgment to the view, the strip,
the index row and the sidecar, a menu click to `refilter`, and the settings
modal's hold on the keys.

- Fixtures: `Ui::open_folder(&[(name, orientation)])` writes each file as the
  smallest DNG the reader takes, generated at test time (a little-endian
  TIFF whose IFD0 holds the Orientation tag and a `JPEGInterchangeFormat`
  preview of 64 x 48 encoded with `mozjpeg`, no raw data), into a `photos`
  folder in the `Ui`'s temporary folder. The listing, pass 1 (thumbnail,
  facts, orientation), pass 2 and the preview read all take it, so the
  folder is not view-only and the judgment keys work. The repository holds no
  image file; do not add one.
- Opening: `open_folder` opens through `open_from_tree`, a click in the
  tree's path, and returns after `wait_idle`: the files listed, the view
  idle (`status_of().idle()`, no listing, scan or run) and the current
  file's preview on screen. Call `wait_idle` again at a test's end when a
  key started a preview read, so the drop does not race it.
- Input: `ui.cx.simulate_keystrokes` with GPUI's key names (`right`,
  `escape`, `ctrl-,`, `cmd-,` on macOS), which reach `key_down` through the
  focused view as a real key does. A mouse path whose target is drawn at
  computed coordinates (a menu item, a strip cell) is called at its handler
  through `Ui::update` instead (`apply_menu` for a filter click); it
  `run_until_parked`s after.
- Assertions: on the view's fields through `Ui::read` (`files`, `pager`,
  `judgments`, `strip_cells()`), on the index through `entries_of` on the
  view's `reader` within `wait_until` (the judgment queue is a thread), and
  on `window.painted_quads()` where a layer's presence is the point (the
  settings modal drew 33 -> 77 quads).
- A fresh settings store has auto-advance on: a star key moves to the next
  file. The app's own sidecar write did not re-list the folder (watched for
  1 s after the `.xmp` appeared).
- The six tests take about 2 s together; the set passed 20 runs in a row
  and the whole crate 5 runs on Windows.

They do not cover pixels (the `screenshot` tool's), pointer hit testing,
drags and scrolling, the tree's roots (the real machine's), the OS folder
picker and notifications, a real RAW's maker notes, AF point and faces (the
generated preview has none), the MCP companion's transport, or timings.

### There is no after-`Present` hook (Measured)

`cx.defer` from the paint closure runs after the frame's `draw()` and
`present()`, which is the stamp the spike's timings used. It is a workaround
and may change with a GPUI bump. `crates/app` follows that pattern: the
paint closure stamps `paint_started` and `paint_finished`, then the deferred
callback stamps `presented`, prints the `page` line and releases the retired
images with `cx.drop_image`. Doing the release earlier than that deferred
callback could pull a texture out from under the frame being presented.

### Rotating on the CPU needs threads for a RAF's budget (Measured)

Windows, release, idle, 31 runs of synthetic BGRA buffers, p50 in ms, 90 degree
turns (orientation 6 / 8): 1616x1080 plain loop 3.2, 64-pixel tiles 1.6, tiles
over 8 threads 0.9; 4000x3000 plain 62, tiles 21, tiles over 8 threads 9-11;
4416x2944 plain 70, tiles 23, tiles over 8 threads 12-13. Allocating the output
alone costs about 8 ms at 4000x3000 (a plain copy of the buffer takes that long).
In the app on real portrait files: 1616x1080 ARW `orient` 1.3-1.5 ms, 1920x1280
RAF about 2.2 ms, 4000x3000 RAF (GFX 100) 7.6-8.5 ms.

- Rule: `orient.rs` tiles over 8 scoped threads because the tiled loop alone
  misses the plan's 15 ms budget on a 4000x3000 RAF; orientation 1 returns the
  buffer without a copy and 3 reverses in place.

### mozjpeg reports corrupt input by unwinding (Inferred)

`mozjpeg` 0.10's decoder calls `resume_unwind` on a libjpeg fatal error, so a
corrupt file would otherwise end the background task and leave the pager's
request in flight forever.

- Rule: wrap every decode in `catch_unwind`, as `load` does through `guarded`.

### The strip is a `canvas` over a pure model, not a `uniform_list` (Inferred)

`uniform_list` is vertical-first; its horizontal mode
(`ListHorizontalSizingBehavior::Unconstrained`) is about item width, not a
horizontal virtual list. A flex `div` with one child per file lays out every
cell each frame. The strip is one `canvas` that paints only the cells on
screen, fed by `strip.rs` (offset, range, hit test, request order), whose
range can go to `ScanFocus` as it is. Its thumbnails come from
`thumbs.rs`, four reads at a time through the index's read-only connection,
and every image a cell drops goes through `retired`, like the preview's.

- Rule: a cell's name wider than the cell is shortened with `…` in the
  middle (`main.rs`'s `fitted_name`, GPUI's `LineWrapper::truncate_line` with
  `TruncateFrom::Middle`: about two thirds of the width for the start, one
  third for the tail with the frame number and the extension), against the
  current `cell_width`, which follows the strip's height, less `NAME_INSET` (2
  px) on each side, since `truncate_line` sums per-character widths rather
  than measuring the shaped line. A name that fits is drawn unchanged,
  centered. Before, a long name was centered and clipped on both sides by the
  cell's content mask (`)SC-RX100M2_DSC00416.ARW`). The user chose the middle
  over the end (cells of a prefixed folder would read the same, the extension
  lost) and the start (the prefix lost), 2026-11-22. The name band's tooltip
  still shows the label's name, not the file name. Seen on Windows 11 on
  2026-11-23 at the default strip height: `ILCE-6300_Free_ra…3068.arw` and
  `L1001880-DxO_D…ME 3.jpg` fitted inside their cells and short names stayed
  centered and unchanged
  (`docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`).
  Seen on Windows 11 on 2026-11-27 at `stripheight::MIN` and `MAX` (heights
  set through the store, not a drag): at `MIN` every long name was shortened
  in the middle and stayed inside its cell with a gap to the next, at `MAX`
  the longest name fitted whole, short names stayed centered and unchanged,
  and no glyph was clipped at either edge, so `NAME_INSET` is left as is
  (`docs/plans/_archived/20261127-windows-device-checks-c7/results.md`).

### A `canvas` gets no input; its wrapping `div` does (Inferred)

`canvas` registers no hitbox and takes no listeners. The strip's canvas sits in
a `div` with `on_mouse_down` and `on_scroll` (`on_scroll_wheel` is a deprecated
alias at this rev, and `ScrollWheelEvent` of `ScrollEvent`). The events carry
window coordinates; the strip spans the window from its left edge, so the
event's `x` is the strip's `x` and `strip::hit_test` takes it as is. The div's
hitbox already keeps clicks and wheels above the strip out.

- Rule: a canvas that needs input sits in a `div` that owns the listeners, and
  maps the event position against where the canvas is laid out.

### A strip cell's tooltips are transparent `div`s over the canvas (Inferred)

GPUI's `.tooltip(..)` belongs to a stateful element's hitbox: it is
scheduled while that hitbox is hovered and dropped when the pointer leaves
it, so one tooltip on the strip's `div` would not change from cell to cell.
Each on-screen cell's `strip::tip_regions` (the label's name over the name
band, the scan's error over the rest of the cell, an empty text giving none)
becomes an absolutely positioned, transparent `div` over the canvas inside the
strip's `div` (made `relative`), with an id of the region's kind and a
hash of the cell's file path (not its index, which a refilter, a sort change
or a re-listing can give to another file under a still pointer) so the
tooltip's state follows the file. A cell's regions never
overlap: stacked hitboxes that do not occlude are all hovered, and two would
both ask for a tooltip.

- The region `div`s take no listener and do not `occlude()`, so their
  hitboxes are `HitboxBehavior::Normal` and the strip `div`'s hitbox under
  them stays hovered (`Window::hit_test` stops only at a `BlockMouse`
  hitbox): its `on_mouse_down` (left and right) and `on_scroll` still fire.
  A press or a wheel off the tooltip's own hitbox clears it.
- The view is gpui-component's `Tooltip`, built in the `.tooltip(..)`
  closure with `Tooltip::new(text).build(window, cx)` (the tree's pending
  rename) or `Tooltip::element` around a `div` capped at `TIP_MAX_WIDTH`
  (the strip's, so a long error wraps). Its `Render` reads only the global
  `Theme` and needs no `Root`: `Root` is only where gpui-component's own
  managed tooltips (`Tooltip::apply` / `ManagedTooltipExt`) keep their
  overlay, which this app does not use. GPUI places the tooltip at the
  pointer and flips it above or left of the pointer when it would leave the
  window (`Window::prepaint_tooltip`), which keeps the strip's inside.
- Measured (2026-11-27, Windows 11,
  `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): the
  hover, the swap from one cell to the next, a scroll under a still pointer
  removing the tooltip, a cell cut by the window's left edge showing its own
  and the clicks over the regions. Still unmeasured: the wrapping of a long
  error at 360 px and the delay.

### The strip, the tree and the meta pane draw the theme's scrollbar (Inferred)

The three draw gpui-component's scrollbar (`ScrollableElement`, at `=0.9.1`)
in the theme's mode, as the settings modal's lists, the dialog bodies and the
menus do: no bar sets a mode of its own. `theme.rs`'s `apply` sets that mode
to `ScrollbarMode::Always` (the user's choice on 2026-11-17, over the
auto-hiding `Scrolling` of 2026-11-14), so a bar is drawn at rest whenever
its content overflows and takes a press without a scroll first. Where the
content fits, nothing is drawn, even in `Always`: gpui-base 0.9.1's
scrollbar prepaint skips a bar whose `scroll_area_size <=
scroll_viewport_size`. The mode is global: the `Sort ▾` /
`Filter ▾` popovers, `PopupMenu` and `Select` lists show their bar too when
they overflow.

- The tree: its `uniform_list` sits in a `relative()`, `size_full()` wrapper
  inside the pane's padding that carries `vertical_scrollbar(&tree_scroll)`,
  so the bar's viewport is the list's and its range the list's own. The
  list's 4 px `px_1()` and each row's 12 px right padding keep a row's
  content (icon, name, rename field, count badge) out of the bar's track.
- The meta pane: the body ends in `overflow_y_scrollbar()` (`Scrollable`,
  which moves `flex_1` / `min_h_0` onto its wrapper and keeps the padding on
  the scrolled content), so the bar covers the body only and the notes under
  it do not scroll. The pane has no right padding (`pl_3()` and `py_3()`), so
  the bar lies at the pane's edge; the body's right padding of
  `SCROLLBAR_TRACK` keeps its rows out of the track, and each note and error
  under it carries its own `pr_3()`, the 12 px it had from the pane.
- The dialog bodies: gpui-component 0.9.1's `Dialog` scrolls its body in a
  `Scrollable` padded by the dialog's own horizontal padding, which
  `modal.rs`'s `shell` sets to `px_4()`, 16 px at the 16 px rem, so the
  body's text already ends where the track begins.
- The strip: `stripbar.rs`'s `StripBar`, a `ScrollbarHandle` over an
  `Rc<Cell<..>>` the view holds, as the preview canvas's size cell is. On
  every render the view `sync`s it from `Strip` (the offset, reported
  negated, and `Strip::content_width`, the cell count times the current
  geometry's pitch);
  the strip's `div` (already `relative()`) gets
  `horizontal_scrollbar(&strip_bar)` after the canvas and the tooltip region
  `div`s, so the bar's hitbox is the topmost. Its viewport is the strip
  `div`, which spans the window, so the bar's range equals
  `Strip::max_offset`; `viewport_bounds` is not read. That `div` is the
  cells' height plus `stripheight::GUTTER` (9 px, `SCROLLBAR_TRACK` less the
  7 px `STRIP_PADDING`) high, so the bar's track, its last 16 px, starts at
  the cells' bottom edge: no cell or name band lies under it, and the burst
  band's bottom 4 px reach into the track's top but stay clear of the thumb,
  at rest, hovered or dragged (the hovered thumb's top meets the band's
  bottom, 3 + 9 = 12 px above the pane's bottom). The canvas and the tooltip regions are placed
  from the `div`'s top, so the gutter is an empty band under the cells.
- Rule: a drag or a track click reaches the strip only through the handle.
  The bar writes the offset (`set_offset`, kept as a positive request) and
  notifies the current view, the `Handpic` entity under `Root`; the top of
  `Handpic::render` `take`s it, once, and when `Strip::scroll_to` moves runs
  `strip_moved` as a wheel scroll does (the range's thumbnails released and
  requested, the scan's hot list, a `cx.notify()` that draws one more frame
  with nothing left to take). A `sync` is never a request.
- Rule: the bar's mouse-down listener runs in the bubble phase before its
  parents' and stops propagation. A press on a shown strip bar selects no
  cell and opens no right-click menu; a press on a shown tree bar reaches
  neither a row's click nor the pane's `on_mouse_down`, so it does not give
  the tree the keyboard (`on_mouse_down_out` is unaffected). With the
  content fitting, no bar is drawn, no listener is registered and the press
  goes where it went before.
  The wheel is not stopped: the strip's `on_scroll` still runs under it.
- Every bar is an overlay whose track, the last 16 px of its parent, takes
  every press on it; `theme.rs`'s `SCROLLBAR_TRACK` is that width, with a
  test against the pinned `Scrollbar::width()`. gpui-base also reads a
  theme's own `scrollbar` track width (`themed_track_width`, falling back to
  `WIDTH`); none of the vendored themes under `crates/app/themes/` sets
  one, so 16 px holds for every shipped theme, and a theme vendored later
  that sets one would break `SCROLLBAR_TRACK`'s assumption. The tree's rows, the meta
  pane's body, the dialog bodies and the strip's cells keep their content
  out of the whole track, always, not only while the bar is drawn: the user
  reversed the covered edges accepted on 2026-11-17 after a look on a real
  screen. The strip grows its pane by the gutter rather than shrinking its
  cells, so a stored `stripHeight` keeps its meaning; with the cells
  fitting, the gutter is an empty 9 px band whose press reaches the strip's
  `on_mouse_down`, which selects by `x` only.
- Seen (2026-11-22, Windows 11): the tree's, the strip's, the meta pane's
  and a dialog body's bars show at rest, and the strip and the tree draw no
  bar when their content fits; the tree's, the strip's and the dialog body's
  thumbs drag and their tracks jump; nothing lies under the tree's or the
  strip's thumb at rest, hovered or dragged, nor under the meta pane's or
  the dialog's at rest, the tree's badge pill ending 16 px from the pane's edge and 7 px from the
  resting thumb (x 218-223 in a pane ending at x 227); the strip's empty
  9 px gutter shows under a short folder; the bars read in Neutral dark,
  Neutral light and Catppuccin Mocha. Not yet seen: the meta pane's thumb
  drag, its notes' right inset and an OS appearance change in System mode
  (`../plans/_archived/20261122-windows-device-checks-7/results.md`).

### A canvas paints an icon with `paint_svg` (Inferred)

`Window::paint_svg(bounds, path, None, TransformationMatrix::unit(), color,
cx)` renders the path through the application's `AssetSource` (`Icons`, so
`icons::SCAN_FACE` resolves as it does for `svg().path(..)`) into a
monochrome sprite drawn in `color`; its `Err` is logged, not unwrapped. The
strip's face focus icon is 12 x 12 px, 2 px inside the image box's
bottom-left, over one black copy 1 px down and right (`MARK_SHADOW`), as
the stars are, and painted before the rejected cell's
`Roles::rejected_dim` quad so the dimming covers it. Its state is computed in `strip_cells()` from
`facts` as the filter's `passes` does, so it follows pass 2 with no
bookkeeping. The closed-eyes mark (`icons::EYE_OFF`) is painted the same
way in a second slot 14 px to the right, so neither icon moves when the
other appears. That the icon is crisp at 100 % and 150 % is not yet checked by
eye. The eight 1 px black copies at `overlay::HALO`'s offsets it had
before were drawn, but merged into a dark box filling the 12 px icon over a
bright photo, with no edge like the stars' single copy gives (seen on
Windows 11 on 2026-11-27 through the MCP `screenshot`).

### A Windows mouse wheel arrives in lines (Inferred)

`gpui_windows` turns `WM_MOUSEWHEEL` into `ScrollDelta::Lines`, the notch
times the system's wheel-scroll lines (3 by default), positive away from the
user; Shift moves it to `x` (`wheel_scroll_chars`). A precision touchpad
through Direct Manipulation gives `ScrollDelta::Pixels`. `pixel_delta(window.line_height())`
turns either into pixels; the strip subtracts it from its offset, `x` when
non-zero, else `y`, so a wheel toward the user scrolls right.

### A stroked path has butt caps, and no cap option is reachable (Inferred)

`PathBuilder::stroke(width)` builds lyon's `StrokeOptions::default()`, whose
caps are `LineCap::Butt` and joins `Miter`. `with_style(PathStyle::Stroke(..))`
could set square caps, but gpui re-exports `StrokeOptions` and not lyon's
`LineCap`, so naming one would need a direct `lyon` dependency. The `f` mark's
crosshair arms are therefore lengthened by half the stroke width at both ends
(`overlay.rs`'s `segments`), where the Tauri mark's `lineCap = "square"`
reaches, so each arm end keeps the 1 px dark outline. The AF frame is a closed
polygon (`add_polygon(.., true)`) and needs no caps.

- Rule: stroke an open path that must look like a canvas square cap by
  extending its ends by half the width; do not add `lyon` for it.

### A canvas paints text through the window's text system (Inferred)

A `canvas` has no child elements, so the file names are shaped with
`window.text_system().shape_line` (one line, centered with
`TextAlign::Center` over the cell width) and the failure text with
`shape_text` and a wrap width, each clipped with `with_content_mask` to its
cell. The run's font comes from `window.text_style().to_run(len)`.

### Text has no stroke; a label's halo is dark copies (Inferred)

GPUI shapes and paints glyphs filled only; there is no text stroke like the
canvas `strokeText` the Tauri face labels used for their 3 px dark halo. A
dark quad behind the label was the other option; it hides more of the face
than the Tauri halo did. `overlay.rs`'s `paint_label` instead paints the
label in `rgba(0, 0, 0, 0.8)` at the eight 1 px offsets and then in the
label color on top, a ring about as wide as the Tauri stroke's outer
1.5 px, which keeps the look close to the Tauri app's. That it reads on a
bright face is not yet checked by eye. A dot is a `paint_quad` whose corner radii are
half its side, with no `PathBuilder` arc. The label's line height is the
shaped line's `ascent + descent`, so its origin is the text's top, as the
canvas's `textBaseline = "top"`.

- Rule: halo text over a picture with offset dark copies; do not paint a box
  behind it.

### An on-demand judgment is superseded through a shared atomic (Measured)

`ondemand::read_eyes` runs on `cx.background_spawn` and asks its `current`
closure before the detection and before the model. The view cannot answer
from another thread, so `eyes.rs`'s `EyesCache` keeps the latest ticket's id
in an `Arc<AtomicU64>` the closure compares its own id with; a clear (another
folder, a re-listing) stores `0`, which no ticket has. `EyesCache::request`
issues no ticket while a judgment is in flight, so while one runs only a
clear supersedes it; a newer request's id follows only a settle or a clear.
Opening another folder while a judgment was in its detection
logged `eyes ... model=0.0ms superseded` on the `marks` hand-check folder,
twice out of two runs. Only one judgment runs at a time and the file current
when it settles is asked for next, so 60 presses 30 ms apart over a 300-ARW
folder logged 7 judgments, not 60.

- Rule: hand a background job a cheap `Fn() -> bool` over a shared atomic to
  stop it; do not try to reach the view from the worker thread.
- Seen (2026-11-27, Windows 11, through MCP): with the `f` overlay on, 60
  `navigate next` about 8 ms apart logged three `eyes` lines (the file the
  burst started on, the file current when the first judgment settled after
  325 ms, about 40 pages in, and the last), and 40 about 35 ms apart logged
  four; the files passed between are never judged, about one judgment per
  0.3-0.5 s.
  `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`.

### Key names on Windows are lowercase (Measured)

`KeyDownEvent.keystroke.key` is `"right"`, `"left"`, `"home"`, `"end"` and `"q"`,
checked with injected `keybd_event` key events in the running app. A non-extended
`VK_RIGHT` / `VK_LEFT`, the keypad's, gives the same names.

Injecting keys with `keybd_event` needs the app window in the foreground for
every key; in a long injected sequence most keys went missing once focus moved
away. Re-assert and check the foreground before each key (skip the key
otherwise), and pick the process by its path in this worktree, since another
worktree's `handpic.exe` may be running or exiting. Report a skipped key
rather than counting it as a page. Held-key measurements also need an idle
machine: another session's `cargo` build fails the gate, so redo the slot.
For a cold set the foreground check must pass before the folder opens: pass 1
lasts only about 4 s over 600 cold ARWs, and a launched window often loses its
first `SetForegroundWindow` to the terminal. Launch with no folder, retry the
focus until `GetForegroundWindow` is the instance's window (by PID), then open
the folder through MCP `open_folder` with the capture already running. Keys
sent while a listing runs are dropped (3.7 s for 600 files), so paging "during
pass 1" starts after the listing and has to fit in what is left. See
[the plan's learnings](../plans/_archived/20261122-windows-device-checks-7/learnings.md).

### Driving the window in a real-device run on Windows (Measured)

Measured on 2026-11-22 (Windows 11, a 1440 px tall display) in
[the plan's learnings](../plans/_archived/20261122-windows-device-checks-7/learnings.md).

- Rule: `SetWindowPos` cannot shorten the window: any asked height came out
  1460 (the display's 1440 plus the frame) while width and position applied.
  For a short-window check drag the window's own bottom edge with synthetic
  mouse input.
- Rule: read the cursor shape with `GetCursorInfo` against
  `LoadCursor(IDC_SIZENS / IDC_ARROW / IDC_HAND)`. Move the pointer away and
  back and wait about 400 ms first; a 250 ms read lagged a step behind.
- Rule: moving the pointer away and back with `SetCursorPos` alone (450 ms
  wait) read `ARROW` all along the pane boundaries; end the move with a few
  relative `mouse_event` moves before the read, and expect the band's edges
  to shift by about 1 px between reads (results in
  `../plans/_archived/20261122-windows-device-checks-resize-scrollbars/results.md`).
- Rule: an OLE drag from Explorer works through `mouse_event`: click,
  shift-click, press, about 60 small moves 25 ms apart, a short wiggle over the
  target, release. The first attempt can go unreceived (no outline); repeat it
  once before calling a drop a failure. Close the Explorer window afterwards
  through `Shell.Application`'s `Windows()` by `LocationURL`, not with keys.
- Rule: on a desktop other sessions share, capture the desktop
  (`CopyFromScreen`) before any real-pointer step: `WindowFromPoint` naming a
  `Handpic` window does not make it this session's, and on 2026-11-27 a drag
  aimed at this session's window landed on another session's Handpic lying
  over it (its settings layer swallowed the drop). Stop rather than raise the
  window over their work
  (`docs/plans/_archived/20261127-windows-device-checks-folder-dialogs/results.md`).
- Rule: posted messages (`WM_KEYDOWN` / `WM_KEYUP` with the modifier state set
  by `AttachThreadInput` + `SetKeyboardState`, and clicks posted at client
  coordinates) reach this session's window whatever lies over it, and cover
  Tab, Escape, Enter, `Ctrl+Z`, `Ctrl+,` and right and left clicks, the
  tree's right-click menu included. They cannot start an OLE drag:
  `DoDragDrop` from an own `IDropSource` failed with `E_FAIL`, and a WinForms
  one returned nothing with no outline drawn; one attempt left the left
  button held down until released by hand, so check the button state after a
  failed attempt.
- Rule: a short preview (a 125-file Delete preview, a 9-file import preview)
  ends before a posted key or a capture arrives. Take `ctrl+,` and `reading …`
  checks on a long one, such as a 500-JPEG import preview. Keep the material
  in copies and hardlinks outside the photo folders, and use a fresh
  `--index`, `--settings` and `--log` per launch with `mcpAllowDestructive`
  off so each folder operation opens its dialog
  (`docs/plans/_archived/20261127-windows-device-checks-folder-dialogs/learnings.md`).
- Rule: measure layout by scanning pixel runs along a row or column of a window
  capture (Neutral dark: pane borders `#1a1a1a`, the status row's border
  `#2f2f2f`, the preview's black surround). A no-folder window keeps the photo
  out of the way. There is no line between the strip header and the cells;
  take the header's height from the cells' symmetric padding. The canvas
  measured this way (1158 x 767 of surround) does not match the 1160 x 810
  inferred from an earlier paint-bounds log, which may count differently; the
  difference is untraced, so prefer capture measurements.
- Rule: a crop re-cut prints no `zoom` line (only a `z` press is timed), so
  judge a re-cut after a pane or strip drag from captures. A drag that shrinks
  the canvas re-cuts nothing because the held crop still covers it
  (`Zoom::holds`); drag so the canvas grows.
- Rule: the settings modal reopens on its first page (Sidecar), not the last
  one shown. Open the target page through its sidebar entry before pressing
  its controls; a press aimed at a later page once hit the Sidecar `Both`
  radio at the same position.
- Rule: `SetCursorPos` followed by a button press did not deliver the click
  to GPUI although `WindowFromPoint` named the instance; a few relative
  `mouse_event` moves before the press did, every time
  (`docs/plans/_archived/20261122-windows-device-checks-8/results.md`).
- Rule: a click on a tree row gives the tree the keyboard, so the next key
  goes to its type-ahead; send `Escape` before the next key
  (`docs/plans/_archived/20261122-windows-device-checks-8/results.md`).
- Rule: to catch a short-lived frame, such as the 1:1 scaled-preview
  placeholder (about 39-70 ms after `z`, before the crop at about 87 ms), take
  a fast burst of captures with preallocated bitmaps (about one every 15 ms);
  slower captures missed the placeholder
  (`docs/plans/_archived/20261122-windows-device-checks-8/results.md`).
- Rule: to see a stale frame during a held key on already-cached files, launch
  with `--no-prefetch`. With the ring on, most pages are prefetch hits, so the
  fit view's reads never outlast the key repeat and no stale frame is drawn
  (`docs/plans/_archived/20261122-windows-device-checks-8/results.md`).
- Rule: MCP plus the `screenshot` tool (an app built with `mise run
  app:dev:screenshot`) closes the checks that need only an open, a navigation,
  a setting, a confirmed run or a left click. It cannot close a key press (Tab
  or Escape in a dialog, `Ctrl+Z`, a held `→`), a `present` /
  `keypressToPixels` percentile (only a real key press prints a `page` line;
  MCP `navigate` prints none, though one MCP session can page faster than key
  repeat), the IME, a drag, a right-click, a menu's click, a Ctrl-click, a
  HiDPI display or macOS / Linux. A run confirmed through MCP
  with `mcpAllowDestructive` opens no dialog, so the dialog's keys and
  dimmed buttons stay open too. Colors, legibility and text size still need
  the user's eye on the running window.
  Launch with a fresh `--index`, `--settings` and `--log`, a fixed
  `--mcp-port`, and `{"mcpEnabled": true}` (plus `"mcpAllowDestructive":
  true` where a run must be confirmed) through `--import-settings`
  ([the plan's learnings](../plans/_archived/20261123-windows-device-checks-mcp-visual/learnings.md)).
- Rule: set the strip's height for a check by writing `stripHeight` into the
  test `--settings` store between launches; no drag is needed. Only a drag
  writes it in the app (see "The strip's height is one number, stored under
  `stripHeight`").
- Rule: to check the GUI subsystem (no console window) with test stores,
  start the release exe through `explorer.exe <file>.lnk`, a shortcut whose
  arguments carry `--index`, `--settings` and `--log`. That launches it
  through Explorer's ShellExecute, as a double-click does.
- Rule: make many files or a long burst without copying data by hard-linking
  one RAW (`ln` in Git Bash on NTFS): 300 RAW plus `.xmp` links gave a
  Delete Sidecars timing folder, and 120 links of one RAW (one capture time)
  a 120-frame burst for the three-digit `position/size` badges. The links
  share one file's data, so a check of per-file content needs real copies
  ([the plan's learnings](../plans/_archived/20261127-windows-device-checks-c7/learnings.md)).
- Rule: Git Bash mangles `\\` in JSON arguments, even through stdin
  here-strings. Give the MCP client `D:/...` paths; it converts them to
  backslashes. A Bash `python -` / `python3 -` with no stdin hangs the tool
  call; run a script as a file with `mise x -- python -I <file>`. The MCP
  client must print UTF-8 (`sys.stdout.reconfigure(encoding="utf-8")`): the
  Windows console's cp932 failed on a `–` in a reply.
- Rule: pass 2 commits rows in batches of 10 (`BATCH`,
  `crates/engine/src/index.rs`), so in a folder of fewer than 10 files the
  `Analysis` group and the strip's face icons appear only when pass 2 ends.
  Check "appears without input" in a folder of more than 10 files.
- Rule: a run's recorded findings can go stale against `main` before they are
  applied (a row's wording was reworded by a merge after the run's build).
  After the rebase, check that each anchor a finding was to extend still
  exists.
- Rule: a `cargo build` of `crates/app` can be stopped by the host's
  low-memory reaper while several other sessions build; build with
  `cargo build -j 2`
  (`docs/plans/_archived/20261127-windows-device-checks-c5/results.md`).
- Rule: the window has no frame above the client area, so the top resize
  band is the client's top pixel, over the title bar; drag from y = 0 of the
  client. A corner drag can do nothing on the first try; retry before
  calling it a failure
  (`docs/plans/_archived/20261127-windows-device-checks-c5/results.md`).
- Rule: a Python `subprocess` capture of a launcher that starts the app with
  redirected std handles hangs until the app exits, since the app inherits
  the handles; have the launcher write to files and read them instead
  (`docs/plans/_archived/20261127-windows-device-checks-c5/learnings.md`).
- Rule: a `screenshot` call takes about 0.3 s, so a delay or a transient
  shorter than that (a tooltip's 500 ms delay, the pane between a folder
  open and its first preview) cannot be measured with it
  (`docs/plans/_archived/20261127-windows-device-checks-c5/results.md`).

### Every keystroke goes through the keymap (Inferred)

`keys.rs`'s `key_name` names a keystroke as the keymap does: the modifiers
held in `ctrl+alt+shift+meta+` order (`platform` is `meta`), then the key
lower-cased, GPUI's `left` / `right` / `up` / `down` as `arrowleft` ...
`arrowdown`; a lone modifier has no name. `Bindings` maps each bound key to
its action, built at startup and again on every Keyboard Shortcuts edit
(`self.bindings` replaced, the lookup still one `HashMap` read). The window's handler, past the tree's gate while
the tree has the keyboard ("The tree takes the keyboard through one flag in
the view"; the settings key passes that gate), takes `home`, `end` and `q`
literally first (the app's own keys, bound to no action), then the
actions: `previous` / `next` page and `focus` toggles the `f` overlay and `zoom` the `z` 1:1 view, so a
user's override of those keys applies; a judgment action goes to
`judgment::Command::from_action`, `undo` / `redo` to `history::Undo`,
`rejectRest` to `reject_rest`, and any other action is logged at `debug` until its step lands. A
later step's key (peaking `e`) is added as its keymap action (`peaking`) in
that `match`, not as a literal key: a literal `"e"` arm placed after the action
arms never fires with the default keymap, because the catch-all
`(_, Some(action))` arm takes the key first.

The Keyboard Shortcuts capture saw the same names on Windows 11 on
2026-11-21: Shift+1 arrived as `!` and was added as a key; Ctrl+Alt+1
arrived as `ctrl+alt+1` and was refused with `"ctrl+alt+1" is bound to red`;
Tab arrived as `tab` and was refused with `"tab" is bound to toggleSides`.

Seen in the running app on Windows 11 on 2026-11-21 under `RUST_LOG=debug`
(the `key: <name> action=...` lines): `shift+x` -> `rejectRest`,
`ctrl+alt+1` -> `red`, `ctrl+alt+0` -> `clearlabel`, `ctrl+z` -> `undo`,
`ctrl+shift+z` -> `redo`, `1` -> `rate1`, `x` -> `reject`, Shift+1 -> `!`
with no action, `arrowright` -> `next`, `arrowleft` -> `previous`.
Ctrl+Alt+digit reached the window. That layout has no AltGr, so the AZERTY
and AltGr parts below are still as read from `gpui_windows` at the pinned
rev, not seen:

- A shifted letter arrives as the lower-case letter with `shift` set
  (`shift+x`). A shifted digit or punctuation key arrives as the character it
  types with `shift` cleared (`!` for Shift+1, from `get_shifted_key`),
  where `keys.ts` names it from `event.code` (`shift+1`); no default key is
  one, but a user override such as `shift+1` would not match.
- With Ctrl or Alt held, a digit is `MapVirtualKeyW`'s unshifted character,
  `1` on a US or JIS layout (`ctrl+alt+1`), but `&` on AZERTY, where the
  Tauri app's `event.code` said `1`. Windows reports AltGr as Ctrl+Alt,
  so on a layout with AltGr `ctrl+alt+<digit>` is also what AltGr+digit
  sends; the default color keys were the Tauri app's, so AltGr+digit behaved
  the same there.

### The canvas's size reaches the view through a shared cell (Inferred)

The preview canvas's bounds are known only inside its paint closure. The
closure writes the size in device pixels (`bounds.size * window.scale_factor()`,
rounded) into an `Rc<Cell<(u32, u32)>>` the view also holds, as `retired` and
`pending` are shared; the 1:1 view reads it when it asks for a crop. A deferred
entity update from the closure was the other option and needs more of
`main.rs`. The value is the last painted frame's: a resize is seen by
`observe_window_bounds` before the next paint, so the crop is re-cut only
after `zoom::CROP_RESIZE_DEBOUNCE` (150 ms), by when the new size has been
painted.

- Rule: read the cell after at least one paint; before that it is `(0, 0)`
  (`crop_size` clamps it to 1).

### 1:1 is one JPEG pixel per device pixel (Inferred)

`paint_image` snaps the visible bounds to device pixels (`snap_bounds`) and,
when the image runs past the clip bounds, cuts the atlas sub-tile by the
visible fraction, rounded. `zoom::Paint` therefore places the upright crop with
its point of interest on a whole device pixel (`canvas_center` floors the
canvas center in device pixels) and gives it its own size divided by
`scale_factor`, so both edges land on whole device pixels and the sub-tile is
cut on whole texels. The window ran at scale 1.0 (96 dpi, a 1600x1000 client)
during the check; that the crop shows without resampling blur by eye is
pending the user (see the zoom-peaking plan's `learnings.md`).

- Rule: anything meant to be 1:1 is placed in device pixels and divided by
  `scale_factor` at the end, never laid out in logical pixels.

### A crop is on screen 8-112 ms after `z` on a busy machine (Measured)

Release build, `zoom` lines of `z` presses on the zoom-peaking test folder,
not idle (other sessions ran), the file cache warm, 25 lines at scale 1.0
(crops of 1024 px per axis at most): four ARW files `read` 0.7-4.7 ms,
`decode` 3-27 ms, `orient` 0-20 ms on the portrait ones, `keypressToPixels`
8-43 ms (112 ms with the outlier below); two RAF files (X-E5, X-H2S) `read`
1.6-5.2 ms, `decode` 14-33 ms, `keypressToPixels` 19-89 ms; two JPEG-only
files `decode` 21 and 47 ms.
`decode` includes the RGBA to BGRA swap. Two `orient` outliers of 36 and 76 ms
(a 1024 px crop over 8 scoped threads, 1-2 ms otherwise) are attributed to the
load, not measured apart.

### The fit view decodes at the canvas's DCT scale (Inferred)

GPUI uploads a new image's whole texture on its first paint, and a 4416x2944
RAF preview's 52 MB upload after an idle second kept the frame off the screen
for about 48 ms after `present`
(`../plans/_archived/20261121-fit-preview-downscale/plan.md`). `load` therefore decodes
the preview JPEG at `fitscale::scale_for`'s `n/8`, the smallest whose decoded
size is not below the source's fit in the preview canvas (`viewport`, device
pixels, read when `send`, `request_prefetch` or a re-read builds its closure);
an unknown canvas (`(0, 0)`, before the first paint) decodes whole. `Held` and
`OnScreen` keep the source's sizes (`px`, `upright`), which the fit bounds,
the `f` overlay, the 1:1 placeholder's rectangle and the `page` line's `px=`
read; only the image is smaller. The `page` line's `decoded=WxH` is the
decoded size, unrotated like `px`.

The canvas paint compares the canvas with `viewport` and, when it changed,
asks through its `cx.defer` for `fit_resized`, which settles for
`FIT_RESIZE_DEBOUNCE` (the crop re-cut's 150 ms, its own timer) before
`fit_settled`: the ring's entries the new canvas outgrew are given up by
`Ring::release` (not marked crowded, so they are read again), and `refit`
re-reads the image on screen when `fitscale::serves` says it falls short.
Hanging it off the paint rather than the resize call sites catches every
change of the canvas (a scale factor change too), and the check runs only
after `viewport` holds the new size. `show_current` calls `refit` too, so a
page read decoded for an older canvas is re-read when shown, and a prefetch
that arrives below the canvas (it left for an older one) is released as it
settles, so the ring's entry is read again rather than painted upscaled.

- Rule: a re-read replaces only the image (`refitted`, the old one retired):
  no `present`, `page` line, dwell, `mark_viewed`, `marks.show` or meta pane
  change. `present` and `clear_image` bump `refit`, so a page turn or a clear
  supersedes the re-read in flight, as `order` does a re-listing.
- Rule: a smaller canvas reads nothing; an image larger than needed serves.
  `scale_for` must return 8 for a zero canvas before its search, because
  `serves` is true for any size on an unknown canvas and the search would
  otherwise pick 1/8 for a read before the first paint.
- Rule: the 1:1 placeholder is the fit image drawn into the rectangle
  `zoom::placeholder` gives from the source's upright size, so it is softer
  for a large preview but placed as before; the crop is `zoom.rs`'s own read.
  The fit view's peaking layer is keyed by the image id, so a re-read image
  gets its edges recomputed.
- Rule: expect the decode gain from the scale only on a large preview. On the
  RAF compare folder, 3/8 cut the decode about a third (49 to 34 ms p50)
  because the entropy decode does not shrink with the output. A 6/8 decode of
  an ARW (1616x1080 to 1212x810) was no faster (no SIMD IDCT at 6/8, 7.0 vs
  7.6 ms p50, within noise). The gain that matters is the texture upload:
  `screen` minus `keypressToPixels` fell from 47 to 15 ms p50 on RAF. Judge
  the change by that A/B ruler, not by `decode` alone
  (`../plans/_archived/20261121-fit-preview-downscale/learnings.md`, Steps 2
  and 3).

### Peaking runs off the main thread, one job at a time (Measured)

`peaking.rs`'s job (resize to the drawn size, then the Sobel) takes 25-60 ms on
a fit view, several frames, so it runs on `cx.background_spawn`, reading the
shown `RenderImage`'s bytes through its `Arc` without a copy. One job is in
flight, as the Tauri worker held one request: under a held key each page
would otherwise queue a job on the executor the next preview decode runs on.
A layer replaced while a job is out is never computed; the reply takes the
newest unsent layer next. The fit view's drawn size is known only in the paint
closure, so the closure compares it with the layer the view rendered with and,
when they differ, asks for the new layer through a `WeakEntity` update in the
`cx.defer` it already runs after paint; while the layer matches nothing is
deferred.

- Rule: an overlay is drawn only for the `ImageId` and device size it was
  computed for (`peaking::Paint`), so another file's or another size's never
  shows; replaced and cleared overlays go through `retired`.

### The fit view's overlay is resized bilinearly (Measured)

The overlay is computed at the drawn device size, as the Tauri app rasterised
the bitmap at that size first. `image::imageops::resize` with
`FilterType::Triangle`, the nearest to a canvas `drawImage`, was kept: at
scale 1.0 it takes about 22-25 ms for a 1616x1080 ARW to 1275x852 and 47-67 ms
for a 4416x2944 RAF to 1278x852, the Sobel 5-7 ms on top. So on a RAF the
overlay comes about 50-60 ms after the image; the held-key measurement showed
no cost to paging, so no cheaper filter was tried. The 1:1 crop needs no
resize (Sobel 4-6 ms).

### A held `g` grays only the photo, through the sprite flag (Measured)

`grayscale` (default `g`) is held, not toggled: `grayscale.rs`'s `Hold` is on
from the key's press until its release. The pinned `gpui-pre`'s
`Window::paint_image` takes a `grayscale: bool` that the sprite shader turns
into the color's luminance, alpha kept, on all three backends, so the fit
image in the preview canvas's closure and `zoom::Paint`'s placeholder and crop
pass `Hold::is_on()` and no texture is re-uploaded or copied. Unlike the Tauri
app, whose CSS filter on `#canvas` also grayed them, the peaking layer, the
`f` overlay and the performance graph keep their colors, by the user's choice,
so they stay readable over the gray photo.

The press is a `grayscale` arm of `key_down`'s action `match`, so the
settings, a dialog, an inline rename and the tree's gate swallow it as any
other key. GPUI has no `event.code`: the held key is `Keystroke.key`,
lower-cased, compared with the key-up's. `ModifiersChangedEvent` carries only
the new state, so `Hold` remembers the last modifiers; one going up ends the
hold, one going down does not, so a rebound `ctrl+g` ends on whichever of `g`
and Ctrl goes up first. Each method says whether the state changed, so an
auto-repeated press calls no `cx.notify()`.

- Rule: key-up and modifiers-changed are dispatched only along the focused
  node's path, so the main `div`'s `on_key_up` / `on_modifiers_changed` never
  see a release once the settings layer, a dialog, a `PopupMenu` or an
  `Input` has the focus. `cx.on_blur(&focus, ..)` releases the hold when the
  view's own focus handle loses the focus, and the window activation observer
  releases it on deactivation.
- Seen on Windows 11 on 2026-11-22: the strip's right-click `PopupMenu`
  opening did not end the hold (the photo stayed gray while `g` was down),
  and the held `g`'s key-up with the menu open did, which the rule's
  `PopupMenu` case does not predict; no cause was traced
  (`docs/plans/_archived/20261122-windows-device-checks-8/results.md`).

### The performance overlay is a band painted last in the preview canvas (Inferred)

`d` (the keymap's `performance`) toggles `perfgraph.rs`'s `Overlay`. Showing
it starts recording in `perf.rs`'s `Perf`, the engine's `perf::Sampler`,
whose thread only sends `Event::Perf` into the view's one channel (the
sample is stamped with `Instant::now()` when the view takes it, as the Tauri
listener stamped `performance.now()`), and a 500 ms `cx.notify()` loop held as
an `Option<Task<()>>`; hiding it stops all three and drops what was
recorded. `Sampler::stop` only signals, so one late sample can still arrive
and the model drops it. The graph is painted by the preview canvas's closure
after the image, the 1:1 crop and the `f` marks, and also with no image, as
the bottom 120 px of the canvas's bounds under a content mask, so it moves
neither view and takes no input.

A switch's latency point is completed in the same `cx.defer` as the `page`
line, from the same `timing::Stages`, its value `keypress_to_pixels` and its
bar the eight `page` stages stacked (they do not sum to the value, see
"`keypressToPixels` differs from the Tauri app's"). A page no key or strip
click turned (an open, a refilter, a re-listing) starts at `switched`, the
time `request` was last called, and is built only while the overlay is on;
the `pending` slot carries whether it was keyed, and only a keyed one prints
the `page` line and goes into `Summary`. A stale preview drawn on the way
(`stale`) and a failed one add no point.

- Rule: with the overlay off nothing samples, records or redraws; the only
  cost is the `on()` check in `paint` and in `loaded`.


### One progress circle sits in the status row's right region (Inferred)

`progress.rs` holds the indicator's model: `fraction`, a port of
`progress.ts`'s `progressWidth` (0 while `total` is 0, never above 1), and
`shown`, which picks the `(done, total)` the indicator shows when several
operations run: a Move to Trash, Delete Permanently or trash redo run first
(`Handpic.trashing`, the counts the pane's note is formatted from with
`trash::progress_status`), then the undo of a trash run
(`Handpic.undo_progress`), then a folder operation's run, then the scan's
running pass (`scan::Status::running`, pass 1's before pass 2's, the two
sharing the one indicator). `main.rs`'s `progress_circle` draws it as
gpui-component's `ProgressCircle` (`Size::Small`, 12 px, the row's text
size), colored `primary`, its value `fraction * 100`, in the
`StatusBar`'s right region right of the position, only while `shown`
yields a count. While it does not, an empty, non-shrinking `size_3` `div`
holds its slot, so the right region's content width is constant and the
position stays where it is when the circle appears or goes (the user's
decision, 2026-11-22; before, the position shifted about 20 px). The Tauri app drew a bar along the row's top edge
(`#scan-progress`); the circle is the user's decision.

The linear `Progress` was ruled out for that bar, and the reasons hold for
`ProgressCircle` too, as verified at 0.9.1: it draws its track as its
indicator's color at 20 % opacity with no setter for the track apart from
it, so the track is `primary` at 20 %, not `muted`; its value
always animates through `transition` over the theme's
`motion.duration_normal` (180 ms by default), turned off only by GPUI's
reduce-motion or by a zero `duration_normal`, which the dialogs read too;
while it runs it requests animation frames, and since it is keyed by the
element id (`"progress"`), a drop of the value (pass 1 to pass 2, a rescan)
is drawn as the ring emptying from full to 0. The user accepted both by
choosing the circle.

- Rule: the circle is in the row's flow inside the right region, never in
  `left` (see the `StatusBar` section). The row is pinned to
  `STATUS_ROW_HEIGHT`, so showing or hiding it changes neither the row's
  height nor the canvas (measured with the strip header on 2026-11-22: 1158 x
  767 of surround either way, the row 30 px), and the status text,
  start-aligned in the center region, does not move; the center region is
  always 20 px narrower (the circle's slot and the region's 8 px gap), with
  or without the circle, and the slot is empty space at the row's right end
  while idle. Verified on Windows 11 on 2026-11-23: the position `1 / 8`
  stayed at the same x with the circle shown (pass 2) and gone
  (`docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`).
  Measured in the other layouts on 2026-11-27: 1379 x 767 with the right
  pane hidden, 1600 x 767 with both side panes hidden and 1158 x 936 with
  the strip alone hidden, each the same with the circle shown and gone
  (`../plans/_archived/20261127-windows-device-checks-folder-dialogs/results.md`).
- Rule: do not use `.loading(true)`: nothing here is indeterminate. Pass 2
  starts at `(0, 0)` and pass 1 can start at a total of 0, so the empty
  track ring shows there.
- Rule: the circle adds no `notify` or timer of its own; it is redrawn by the
  notifies the scan's `Progress` / `FacesProgress`, the trash's
  `TrashProgress`, the undo's `UndoProgress` and the folder runs'
  `SequenceProgress` / `ImportProgress` / `SidecarProgress` already make,
  and it goes wherever `scan_status.clear()`, `trashing = None`,
  `undo_progress = None` and the folder runs' ends do. Its value's transition does request animation
  frames for 180 ms after each change. Seen (2026-11-22, Windows 11): at
  pass 2's start the full ring drained to empty within about 240 ms (frames 80 ms apart, consistent with the 180 ms animation) and filled
  again, judged to read as a reset; paging during pass 2 took 12 to 29 ms
  keypress to pixels against 5 to 21 ms after the scan (after a first 140 ms page), the difference
  following the read source (`read` during the scan, `prefetch` after), not
  the frames, judged not visibly slower on a debug build; paging during
  pass 1 not taken
  (`../plans/_archived/20261122-windows-device-checks-7/results.md`).
- Rule: do not zero `duration_normal` to stop the animation: the dialogs read
  it too.
- Seen (2026-11-22, Windows 11): the circle filled through pass 2 of a cold
  600-file folder and through a 600-file Move to Trash run and went at each
  end, the position shifting about 20 px with it (since fixed by the
  reserved slot above); pass 1 (about 1.1 s) was
  too short to see (results in
  `../plans/_archived/20261122-windows-device-checks-4/learnings.md`).
- Sequence JPEG Timestamps and Import feed it through `shown`'s `folder_op`
  argument: `FolderOps`' `sequence_progress()`, else `import_progress()`
  (the order `status_of` uses), set to `(0, 0)` at the run's start, updated
  by `folder_op_message` and cleared in `sequence_ran` / `import_ran`, which
  every end, a cancel included, passes through; no re-listing stops either
  run.
- The undo of a trash run feeds it through `shown`'s `undo` argument:
  `trash::undo_run`'s callback (per item, a RAW or JPEG with its sidecars or
  a sidecar alone, or per chunk on Windows) sends `Event::UndoProgress` into
  `Handpic.undo_progress`, set to `(0, 0)` in `undo_trash` and cleared in
  `restored`, which a failure passes through too. It does not set
  `trashing`, which words the pane's note, disables the strip menu's trash
  items, refuses `confirm_trash` and is the companion's `trashing`.
- Rewrite Sidecars and Delete Sidecars feed it after the sequencing and
  import counts: `FolderOps`' `sidecar_progress()`, set to `(0, 0)` in
  `run_sidecars`, updated by `folder_op_message`'s `SidecarProgress` and
  cleared at the top of `sidecars_ran`, which both runs end through, a
  failure included. The rewrite counts rows written:
  `foldersidecars::rewrite` hands the writer `REWRITE_BATCH` rows at a time
  and reports after each batch drained, all under the one `REWRITE_DRAIN`
  deadline; the delete counts sidecars moved.
- `get_status`'s `Operation.progress` reports the same counts as the
  circle while the run's `running_op` is set.
- A new operation needs a `(done, total)` callback before it can feed it.

### The status row is gpui-component's `StatusBar` with its defaults (Inferred)

`main.rs`'s `status_row` builds the row with gpui-component's `StatusBar`
(`=0.9.1`, `src/status_bar.rs`), a `RenderOnce` `h_flex` with `left`, center
(`child` / `children`) and `right` regions, and overrides none of its root
style: `items_center`, `gap_2`, `py_1`, `px_2`, `border_t_1` in the theme's
`status_bar_border`, the `bg` of `tokens.status_bar`, `text_xs` (12 px) in
`muted_foreground`. Each region is an
`h_flex().overflow_hidden().items_center().gap_2()`. This reverses the
earlier rule that kept the row a plain `div` (PR #1072), by the user's
decision, which also chose the component's look over the old one (the status
text was 16 px white, the row transparent over the root's black).

- Rule: `theme.rs`'s `map` sets `status_bar` to the panes' `#171717` and
  `status_bar_border` to the border (white at 10 %), and the theme test
  asserts both; `Theme::update` carries an edited `colors` field into
  `theme.tokens`, which the bar's `bg` reads. Unmapped, they would fall back
  to `Default Dark`'s `#171717` and a solid `#262626` border. Seen on screen
  in a debug build: a filled row with a thin top border.
- Rule: the status text is the center `child`, with no `left`; the position
  (while a file is shown) and the circle (while shown) are each their own
  `.right(..)`, in that order, 8 px apart (the region's `gap_2`). With only
  `right`, the center is `flex_1` and start-aligned. Never put anything in
  `left` while the right region holds something: with both ends pinned the
  center is `justify_center`, so the status text would jump to the middle of
  the row.
- Rule: no wrapper around the status text. Seen on screen with a text far
  longer than the row: it stays on one line, clipped by the center region's
  `overflow_hidden`, and the position and the circle keep their place.
- Unaffected: `StatusBar` has no id, focus, key context or listener, so the
  keymap is unchanged. `Deferred` in `gpui-pre` 0.3.12 calls
  `window.defer_draw` with no content mask (`elements/deferred.rs`), so no
  ancestor of the strip header clips the Sort / Filter popovers and
  `anchored` still opens them from the button's top-right; seen
  (2026-11-22, Windows 11): both menus opened upward with their right edge on
  their button's, unclipped by the header, and in a 603 px window the long
  filter menu stayed between the title bar and the header
  (`../plans/_archived/20261122-windows-device-checks-7/results.md`).
- Rule: on a gpui-component bump, re-check the root's padding, border, text
  size and color and the regions' gap and alignment rules in
  `status_bar.rs`, and `ProgressCircle`'s sizes, track and animation, and
  re-measure the preview canvas, which the row's height sets.
- Rule: the row's height is pinned to `STATUS_ROW_HEIGHT` (30 px), what it
  was while it held the `Sort ▾` / `Filter ▾` buttons: their 21 px
  (`MENU_BUTTON_HEIGHT`) plus `py_1` above and below and the 1 px border.
  Its own content is shorter (the status text's line is 19 px, 12 px at
  GPUI's default line height rounded by `line_height_in_pixels`, the circle
  12 px), so without the pin the row would shrink and the canvas grow when
  the buttons left. Measured (2026-11-22, Windows 11, scale 1): the row is
  30 px including its 1 px top border, the same with the circle shown and
  gone.

### The title bar is gpui-component's `TitleBar` with the icon, "Handpic", the open folder's name and the settings gear (Inferred)

The window draws no OS title bar. `main()` opens it with
`..TitleBar::window_options()` (gpui-component `=0.9.1`,
`src/title_bar.rs`, which sets `app_owns_titlebar_drag`) and an explicit
`titlebar: Some(TitlebarOptions { title: Some("Handpic".into()),
..TitleBar::title_bar_options() })`, so the client area covers the caption
(`appears_transparent`) and the macOS traffic lights sit at (9, 9).
`impl Render for Handpic` puts `TitleBar::new()` first in the root's
`flex_col`, a `flex_shrink_0` row of `TITLE_BAR_HEIGHT` (34 px) in flow on
every platform, so the main area below it shrinks by itself. Its first child
is an `h_flex().gap_2().text_size(px(13.))` of `img(icons::APP)` at 18 px and
the text "Handpic" at 13 px in `cx.theme().foreground`; its second, which the bar's
`justify_between` row puts at the right end (left of the window controls on
Windows / Linux), is `settings_button`: a ghost, `tab_stop(false)`
gpui-component `Button` with `Icon::default().path(icons::SETTINGS_THIN)` (Lucide
`settings` at `stroke-width="1.5"`) in a full-height `div` (`h_full()`, no
margin) holding the tooltip. The bar draws the window controls on
Windows (hit-tested through `WindowControlArea`) and Linux and moves the
window itself. Source: [the plan](../plans/_archived/20261116-native-title-bar/plan.md).

- Rule: the open folder's name (`context::folder_title` of `Handpic::dir`:
  the path's last component, else, for a root such as `C:\` or `/`, its
  `shown_path`) is centered across the whole window, not only `TitleBar`'s
  inner row, which starts after the left padding and ends before the window
  controls. A `relative`, `flex_shrink_0` `div` wraps `TitleBar` (its
  height stays `TITLE_BAR_HEIGHT`) and, while a folder is open, holds after
  it an `absolute().inset_0()` overlay at `text_size(px(13.))` in the theme's
  `foreground`, padded `TITLE_NAME_MARGIN` (160 px) a side so the name,
  `min_w_0().truncate()`, ends in an ellipsis before the icon, "Handpic",
  the gear or the controls. The name is read in `render`, so another open,
  a folder rename and a `Rename by Capture Date…` of the open folder change
  it with no state of its own. Keep the overlay free of listeners, ids,
  cursors and hover styles: `gpui-pre` 0.3.12's `div` inserts a hitbox only
  for those (`should_insert_hitbox`; `overflow_hidden` is not one), so a
  press on the name falls through to the bar's `WindowControlArea::Drag`
  row and the controls. Read from the source, not yet seen running (a
  manual check is pending in the plan); the fallback is a third `TitleBar`
  child, centered only between the left group and the gear. Source:
  [the plan](../plans/_archived/20261118-native-title-bar-folder-name/plan.md).
- Rule: the gear's `Button` is `.with_size(px(14. / 0.75))`,
  `.w(TITLE_BAR_HEIGHT)`, `.h_full()` and `.rounded(ButtonRounded::None)`.
  gpui-component draws a button's icon at 0.75 of the button's size, so
  `px(14. / 0.75)` gives a 14 px icon; the box is then set to
  `TITLE_BAR_HEIGHT` wide, the bar's full height, with square corners, after
  the window controls' own box, so its hover fill sits flush with them. Seen
  in a dev build on Windows 11, where the user approved it; the margin-less
  square gear is unseen on macOS (traffic lights on the left, none beside it) and
  Linux, a manual check pending. Source: the plan
  `docs/plans/20261127-title-bar-tweak/plan.md`.
- Rule: keep `title: Some("Handpic")`. `title_bar_options()` has `title:
  None`, which would empty the taskbar and Alt+Tab text.
- A press on the bar still reaches the root `div`'s `track_focus`, so a click
  on the bar keeps the keyboard with the view. The bar has no focus handle of
  its own: do not `track_focus` the bar or its children, and keep a `Button`
  in it at `tab_stop(false)` (its internal handle stays unfocused, its
  mouse-down preventing the focus change). Seen in a debug build on Windows
  11; the controls' clicks were not exercised by hand. Source:
  [learnings](../plans/_archived/20261116-native-title-bar/learnings.md), Step 1.
- Rule: a control in the bar stops the left press's propagation, or the press
  moves the window. `TitleBar` puts its children in a row marked
  `WindowControlArea::Drag`: on Windows `gpui-pre` 0.3.12 answers `HTCAPTION`
  there and starts the OS drag unless GPUI's dispatch of the non-client press
  stopped propagation; on macOS / Linux the bar's own mouse-down arms
  `start_window_move`. The gear's wrapping `div` calls `cx.stop_propagation()`
  on the left mouse-down, after the `Button`'s own listeners (gpui-component's
  Linux `ControlIcon` is the precedent), so the click still fires. Seen on
  Windows 11 in a debug build: a press and drag on the gear leaves the window
  in place, a drag on the rest of the bar moves it, the click opens the
  settings and the tooltip shows on hover. Source:
  [the plan](../plans/_archived/20261117-native-title-bar-settings/plan.md), Step 1.
- Rule: the gear opens the settings through `Handpic::open_settings`, the one
  method `key_down`'s `settingsmodal::OPEN_KEY` arm calls too; it applies the
  refusals itself (the settings already open, a trash or folder-operation
  dialog open, a folder flow under way), since a click does not pass through
  `key_down`'s early returns. Its tooltip is `Tooltip::new` with
  `settingsmodal::OPEN_TIP`'s text and its key as a `Kbd`
  (`keys::keystroke_of(OPEN_KEY)`), on the wrapping `div`'s `.tooltip`
  because `Button`'s own tooltip takes only a string or a GPUI action and
  `OPEN_KEY` binds none.
- Known limit: after a click on the gear the tooltip stays drawn over the
  opened settings modal while the pointer rests on the gear (the modal's
  `occlude()` covers the bar, but the tooltip lives in a higher layer); it
  goes when the pointer moves. Left as is. Scripted pointer checks of the
  bar must keep presses off the empty preview's "click here" message,
  which opens the folder picker. Source:
  [learnings](../plans/_archived/20261117-native-title-bar-settings/learnings.md), Step 1.
- Rule: the icon is `crates/app/icons/64x64.png`, served by `Icons` under
  `icons::APP` (`icons/app.png`) as one more arm of `load` and `list` (the
  PNG is bytes, `SVGS` a table of `&str`). gpui's `img` loads a non-URI
  string from the `AssetSource` and sniffs the format; no dependency is
  added. `icons.rs`'s test checks the path loads PNG bytes.
- Rule: Linux requests `window_decorations: Some(WindowDecorations::Client)`
  (the user's choice). `gpui-pre` 0.3.12 defaults to server-side, which would
  keep the window manager's bar and put this one, without controls, under it;
  under client-side decorations `Root`'s `window_border()` draws the frame,
  shadow and resize edges. That frame pads the content (about 21 px a
  side on Linux), so `strip_width` and the menus' `menu_max` subtract
  `gpui_component::window_paddings` from the viewport (zero under
  server-side decorations and on Windows / macOS).
- Rule: the bar's fill and bottom border are the theme's `title_bar` (a
  gradient from it mixed 55 / 45 with `background` down to `title_bar`) and
  `title_bar_border`. `theme.rs`'s `map` sets them in both palettes (dark
  `#171717` and white at 10 %, light `#ffffff` and `#e5e5e5`); a shipped
  theme supplies its own. The text color comes from the theme, not
  from the root's pure white.
- Rule: the Sort / Filter and right-click menus' height cap subtracts
  `TITLE_BAR_HEIGHT` as well as 40 px, so a long menu stops under the bar.
- Measured (2026-11-27, Windows 11,
  `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): the
  resize edges over the bar (the client's top pixel), Snap Layouts, and a
  stored `windowBounds` restoring without drift. Observed dead: the controls
  and the drag under the trash dialog's and a folder operation's backdrop
  (they work under the settings); the fix is in flight. Unmeasured:
  double-click to maximize, macOS's traffic lights against the 80 px left
  padding, and Linux under client- and server-side decorations.

### Text is typed through gpui-component's `Input` (Inferred)

The settings modal's text fields (the label names, the import pattern) are
`gpui_component::input::InputState` entities drawn with `Input`; GPUI at the
pinned rev has no text input of its own. The state registers itself as the
window's input handler while focused, so the platform's text input and the
IME reach its `EntityInputHandler`. The view reads the text with `value`, sets
it with `set_value` (which emits no `Change` and clears the undo history, so
`settingsview.rs`'s `show` leaves a field already showing the text alone; a
value the main view learns in a handler with no `Window` (the MCP page's URL,
from `scanned`'s state) is synced in `SettingsView::render`, where a `Window`
exists) and subscribes to `InputEvent`:

| Owner's meaning | `InputEvent` / key |
| --- | --- |
| commit | `PressEnter` (Enter) or `Blur` (the focus left the field) |
| commit, then close the modal | Escape: no event; the `Escape` action propagates on a single-line input, so the key reaches `SettingsView::key_down` as `ModalKey::Close`, which commits the focused field and closes |
| the text changed | `Change` (the pattern's example line follows it) |

- Rule: the editing keys are the `Input` key context's bindings (see the
  table in "gpui-component is initialized"), dispatched as actions before
  any key-down listener. A handled one stops there, so it reaches neither the
  box's `key_down` nor the window's keymap; `Enter`, `Escape` and the
  `IndentInline` / `OutdentInline` of `tab` / `shift-tab` propagate on a
  single-line input, so the box's handler still sees Escape and Tab
  (`ModalKey::Close`, `ModalKey::Focus`). The fields stay in the modal's tab
  order: the state's focus handle is a tab stop and the frame's is not.
- Rule: a stopped key down types nothing on Windows. `gpui_windows`'s message
  loop calls `translate_accelerator` first, which dispatches `WM_KEYDOWN` to
  the window; when an action handler or a listener stopped it (`propagate`
  false) the loop skips `TranslateMessage`, so no `WM_CHAR` follows. A
  consumed editing key therefore never also inserts a character, and a key
  meant to be typed must not be stopped by anything on the dispatch path: no
  `Input` binding is a bare printable key, the settings box's handler lets a
  key it does not take go on while an `Input` key context is on the focused
  path, the view's fields and the `Settings` search field alike
  (`settingsmodal::stops`), and the window's root handler is not on the
  settings' dispatch path. `WM_CHAR` then reaches the focused
  state's handler; a propagated Enter, Escape or Tab gives a control
  character, which `parse_char_message` drops. A key the IME takes arrives
  as `VK_PROCESSKEY`, for which `MapVirtualKeyW` gives no character, so GPUI
  raises no `KeyDownEvent` and `TranslateMessage` always runs: composition
  never passes through the bindings or the listeners, and Enter or Backspace
  inside a composition stay the IME's.
- Rule: the look is the port's, set per field (`settingsview.rs`'s `field`):
  `focus_bordered(false)` drops the component's focus ring (the theme's
  `ring`), and the border is the theme's `input`, `primary` while the
  state's handle is focused, with no fill, `rounded_sm`, 8 / 3 px padding,
  13 px text and a 29 px height (`FIELD_HEIGHT`, a 13 px line at GPUI's
  default line height plus the padding and border). These are `Styled`
  refinements, applied after the component's own size and theme styling;
  `Input::h` is the multi-line height, so the frame's is set through
  `Styled::h`. The caret color is the installed registered theme's
  (`Default Dark` or `Default Light`), which `theme.rs` does not map. The
  selected text's color is `selection`, which `theme.rs` maps to `ring` for
  Neutral (gray) and which a vendored theme gives its own.
- Rule: a right-click opens the component's edit menu (Cut, Copy, Paste,
  Select All) as an OS-native menu on Windows and macOS (`native_menu`),
  which needs no `Root`. On Linux it is a drawn `PopupMenu` held by `Root`'s
  overlay (`WindowState::native_menu_overlay`), which finds one now that
  `Root` is mounted; before, nothing opened there. Not seen on Linux.
- What the component adds over the port it replaced (`textinput.rs`, GPUI's
  `input.rs` example): undo / redo, word movement and deletion, horizontal
  scroll of a long text, double-click word selection and the right-click
  menu.
- Not verified headless: the typing, the IME, the commits, the look and the
  right-click menu; the hand check is pending in
  `../plans/_archived/20261114-gpui-component-adoption/learnings.md`.

### A launch without a folder shows only the tree (Inferred)

`start` opens the folder `main()` chose (`session::startup_folder`: the
command line's, else the remembered `lastFolder` while it is still a
directory and `--auto` is off), and nothing else: the OS folder picker
(`cx.prompt_for_paths`) is not shown at launch, and a folder is opened from
the left pane's tree, from the picker on `open` or by a drop onto the window
(see below). A remembered folder opens through the same `open` as
the command line's, so it is revealed in the tree and resumes at its last
viewed file. When neither gives a folder the window
shows the tree, the no-folder hint in the preview area (see "The preview
area says why it shows no photo" below), an empty strip, and `no folder` on
the status line; every handler already tolerated `dir: None` and an empty `files` (the
pager's `go` and `request` are no-ops at length 0, `judge` and `step` need a
current path, `zoom::toggle` needs files, the strip's hit test finds no cell).

When the launch still used it, `cx.prompt_for_paths` with `PathPromptOptions {
directories: true, files: false }` was the OS folder dialog; Enter on a typed
path navigated into the folder, and a second Enter selected it (Measured).

### A launch restores the last folder and the window (Measured)

`session.rs` holds the rules, free of GPUI types: `startup_folder`, the
`Geometry` stored under `windowBounds` (the restore bounds in GPUI's logical
pixels, `maximized` and the display's UUID as text; `from_setting` reads a
malformed value, a non-finite position or size or a size that is not positive
as nothing), and `place`, which picks the display (the saved UUID's when the
saved bounds intersect it, else the first display they intersect, else none:
the default) and moves and shrinks the bounds into that display's visible
bounds. `main()` passes that display's `display_id` with
`WindowBounds::Maximized` or `Windowed`: on Windows GPUI keeps given bounds
only when their center is on the `display_id`'s display (else the primary
one's) and otherwise silently uses its own default size, so the display and
the clamp are both needed. The default is `default_bounds`: 1600x1000,
shrunk to fit, centered in the primary display's visible area (the user's
decision, 2026-11-23), opened `Windowed` with no `display_id`.

- Rule: on Windows the default's position is given in workspace coordinates.
  GPUI hands the bounds to `SetWindowPlacement` as `rcNormalPosition`, which
  Windows reads relative to the visible area's corner, so GPUI's own
  `Bounds::centered` (the visible area's center in screen coordinates) landed
  one taskbar width off when the taskbar is docked left or at the top.
  `default_bounds` takes the visible area's offset from the display's corner
  back off when its `workspace` argument is set (`cfg!(windows)`), so both
  cases are unit-tested on every platform. The restore path (`place`) is
  unchanged.

- Rule: `lastFolder` is the Tauri app's key and plain-string value, so a store
  seeded from a Tauri `settings.json` reopens the Tauri app's last folder on
  the first launch. It is written where a listing sets `self.dir`, every open
  (the command line's included), as `context::shown_path` of the canonical
  path (a `\\?\` verbatim path would not reveal in the tree), and only when it
  differs from the last value, so a watcher re-listing writes nothing.
- Rule: the geometry is written once, by the settings quit handler before its
  drain, from an `Arc<Mutex<Option<Geometry>>>` the `observe_window_bounds`
  callback keeps current, and only when it differs from what was loaded. The
  handler holds the cell, not the view. A fullscreen window is kept by its
  restore bounds, windowed. A crash or a kill loses the session's geometry,
  not its folder.
- Rule: closing the window on Windows fires the bounds observer once more
  while the handle is going: `GetWindowPlacement` fails (`failed to get window
  placement` in the log) and `window_bounds` reports the frame insets around
  an empty rectangle (`x 8, y 19, width -16, height -39` here), so the
  observer keeps only a positive size (`Geometry::is_usable`). Without it the
  close button stored that value.
- Rule: `--auto` suppresses both restores and both writes, so a benchmark
  opens at the default size with only its own folder and leaves the store as
  it was. `--settings`, `--index` and `--log` suppress nothing.
- Measured (2026-11-14, Windows 11, one display at 96 DPI, a test store,
  quitting through the title bar's close message): a window moved to the
  outer rectangle (200, 150)-(1400, 950) stored `x 146, y 169, width 1184,
  height 761` and reopened at exactly that rectangle; GPUI's restore bounds
  are not the outer rectangle, but they round-trip. A maximized quit stored
  `maximized: true`, reopened maximized and un-maximized to the same
  rectangle; a minimized quit stored the restore bounds. Bounds far off the
  display opened at the default; an unknown UUID opened on the display the
  bounds intersect. `--auto 1` left both keys unchanged, and a renamed
  remembered folder opened nothing.
- Measured (2026-11-21, Windows 11, one 5120x1440 display, a test store): a
  moved and resized window quit with `q` reopened at the same bounds; a
  window snapped with Win+Left reopened on-screen at its restore bounds; with
  the taskbar docked left (62 px) a moved window reopened without drifting;
  after paging to `_DSC1495` (position 4) a relaunch without arguments
  revealed the folder in the tree at that file with the same index, and a
  folder argument overrode it; `--auto 3` opened at 1616x1008 outer
  (1600x1000 client), vertically centered and 93 px right of the display's
  center with the taskbar docked left: 31 px (the visible area's center) plus
  62 px (the taskbar), the workspace-coordinate offset `default_bounds` now
  takes off.
- Measured (2026-11-27, Windows 11, the same display, the taskbar docked
  left, a fresh `--settings` store with no `windowBounds`): the fixed default
  opened at 1616x1008 outer (1600x1000 client), the DWM frame's center at x
  2591, y 716.5 against the visible area's 2591, 720: centered to the pixel
  horizontally and within the frame's 3.5 px vertically, 31 px right of the
  display's center, no longer 93 px
  (`docs/plans/_archived/20261127-windows-device-checks-c7/results.md`).
- Not seen yet: the default window with the taskbar at the bottom (the
  `todo.md` item "App: real-device check of the default window's centering on
  the visible area"), a second monitor (with a different scale factor), and
  the taskbar docked at the top (the restore bounds are in workspace
  coordinates there, the clamp compares them with the display's screen
  coordinates): the pending check in
  `../plans/_archived/20261114-gpui-session-restore/learnings.md`. macOS and Linux are
  covered by the CI build and the unit tests only; Wayland cannot place a
  window, so only the size can come back there.

### `open` shows the OS folder picker, a later open winning (Inferred)

The `open` action (`ctrl+o`, `meta+o` on macOS) is `pick_folder`, the Tauri
`openFolder`: it reads the `opens` counter, calls `cx.prompt_for_paths` with
the options of the settings' `Choose…` (`directories: true, files: false,
multiple: false`) and, in a `cx.spawn`, opens the chosen folder through
`open(path, None, None)`, as a click in the tree does (whole, without
`--size` / `--max`, revealed in the tree).

- Rule: the answer is dropped when the picker was canceled, or when `opens`
  moved on while it was up (a folder opened from the tree meanwhile wins).
  A picker error (`Ok(Err(e))`) goes to the status line through `fail`.
- Rule: `open` runs while the tree has the keyboard (`TREE_PASSTHROUGH`) and
  not while the settings, the trash dialog or a folder dialog is open, since
  `key_down` returns before the keymap then.
- Seen (2026-11-21, Windows 11): a second `ctrl+o` while the picker is up
  opens no second dialog (one `#32770` window; the main window is disabled
  while the dialog is up), a cancel changes nothing, and after a pick the
  folder opens and the keys work at once without a click.

### The preview area says why it shows no photo (Inferred)

`empty.rs` is the port of `empty.ts`: `state` judges, in `emptyState`'s
order, no folder (`dir` unset), no listed file (`all` empty), no shown file
(`files` empty, the text being `filter::FILTERED_TEXT`), a failed preview of
the current file, a failed 1:1 read of it (`ZoomFailed`, `ZOOM_FAILED_TEXT`,
`The 1:1 view of this file could not be shown.`), else nothing; `text` gives the state's message and
`open_hint` the no-folder one, the `open` action's keys from the live
keymap's `bindings()` shown through `settingsmodal::display_key` and joined
by ` or `, the key clause left out when `open` has none. `render` wraps the
preview canvas in a `relative` div that takes the canvas's old `flex_1` /
`h_full` (plus `min_w_0`), the canvas `size_full` in it, so its bounds, and
with them `viewport`, the fit, the 1:1 crop and the perf band, are the same;
the message is an `absolute`, full-size, centered child in `muted_foreground` at
14.4 px (`0.9rem`) with `p_4` padding (`empty_message`).

- Rule: only the no-folder message has an `id`, `cursor_pointer` and an
  `on_click` (primary button) calling `pick_folder`; the others register no
  listener. The trash dialog, the folder dialogs, the context popover and
  the sort / filter menus are deferred, `occlude()`d layers, and the
  settings an `occlude()`d layer painted after the main layout, so a press
  on them does not reach the message.
- Rule: the message's text is a `min_w_0` child `div` of the centered
  container (`debug_selector` `empty-text`), never a bare string child, so
  it wraps within the canvas less the container's 16 px `p_4` on each side,
  centered by `text_center`. A bare string child is a flex item whose
  minimum width is its whole single line (GPUI's text element takes a wrap
  width only when its width is known or the available width is definite,
  `elements/text.rs`, as in "A meta pane value that does not fit beside its
  label goes under it"), so in a narrow preview (`PREVIEW_MIN` is 240 px) it
  stayed one line, spilling over the tree and under the meta pane. In a wide
  preview the child's basis is its one-line width, so it is still one line.
  The 16 px padding is wider than the 5 px drag bands of the pane borders
  (`pane_handle`), so the wrapped text never reaches them. The click target
  stays the full-size container, which the bands occlude. The UI test
  `the_no_folder_message_wraps_inside_a_narrow_preview_and_still_opens_the_picker`
  checks the text inside a 240 px preview, taller than in the default
  window, and its click opening the picker, on the test text system's
  synthetic glyph widths.
- Seen (2026-11-27, Windows 11, debug build with the `screenshot` feature,
  no folder): with `panelWidths` `{"left":480,"right":480}` in a 1200 px
  window (the preview's black surround x 481 to 718), the no-folder message
  wrapped onto three centered lines with its glyphs at x 500 to 696, nothing
  over the tree or under the meta pane; a synthetic click at the text's
  center opened the folder picker, one at x 481 or 715 (on a band) and one on
  the tree at x 420 opened nothing. In a 1600 px window with the default
  220 px panes it was one line, as before (results in
  `../plans/_archived/20261127-empty-text-wrap/results.md`).
- Rule: "the current file's preview failed" is `shown` being
  `Shown::Failed` at `pager.current()` with the 1:1 view off (Tauri's
  `previewFailedSeq === seq && !comparing && !zoomed`; there is no compare
  view). A page turn moves `pager.current()` off `shown`'s index, so the
  message goes at once; `refilter` moves `shown`'s index through
  `filter::shown_after`, so it stays with the failed file.
- Rule: "the current file's 1:1 read failed" is `Zoom::read_failed`: the
  1:1 view on, the last crop read for the current page (`seq`) failed and no
  crop of that page held. `arrived`'s `Err` arm records the page, a
  successful read of the current page clears it, `toggle` clears it and a
  page bumps `seq` past it, so the message goes on a page turn, a later read
  that succeeds (a re-cut after a resize) and `z` off, the fit view then
  showing its own state. A failed re-cut after a resize keeps the page's own
  crop on screen and shows no message. The status line still gets the
  read's error (`crop_arrived`'s `fail`). Without this the 1:1 view of a file
  whose crop read fails was a plain black canvas, with nothing to draw when
  its preview had failed too.
- Rule: a failure clears the image (`loaded`'s `Err` arm calls
  `clear_image`), and nothing puts an older one back: `Finished::Stale` never
  draws a failure and draws a success only after the pager has moved on, the
  ring never holds the failed file, and a failed crop releases an earlier
  page's held crop (`zoom::arrived`).
- Rule: `dir` is set when the listing arrives, as Tauri's `openDir`, so the
  hint shows while the first open's listing runs.
- The no-folder text is `empty.ts`'s, with its "Drop a folder onto the
  window" clause (see the drop section below).
- Seen (2026-11-22, Windows 11): rebinding `open` to `space` made the
  message end `Or press Space.` within 150 ms of the settings
  closing (the first capture), and a press on the settings layer over the message opened no
  picker. The no-key clause cannot be reached from the UI: the Keyboard
  Shortcuts page refuses to remove an action's last key (`"space" is the
  only key for open; use Reset`)
  (`../plans/_archived/20261122-windows-device-checks-7/results.md`).
- Seen (2026-11-22, Windows 11): the no-folder message centered and its
  click opening the folder picker, the no-files message's click doing
  nothing, and on `NIKON_D70_Nikon.nef` the failed-preview message, which
  `z` hides (results in
  `../plans/_archived/20261122-windows-device-checks-4/learnings.md`).
### A folder dropped onto the window opens, a later drop winning (Inferred)

The root `div` of the window's `render` takes the drop
(`on_drop::<ExternalPaths>`, `drop_paths`), a port of the Tauri
`tauri://drag-drop` listener and `dropped_dir`, whose pure rules are in
`dropped.rs` (copied, since the app crate did not depend on the Tauri
crate). One dropped folder opens through `open(path, None, None)`, as
a click in the tree does; one dropped file, of any kind, opens its parent
folder, the file not selected; no path, several paths, or a path that is
neither a directory nor a file opens nothing and puts the Tauri
`Drop a single folder or RAW file.` (`DROP_TEXT`) on the status line.

- Rule: the directory test is a `stat`, run on `cx.background_executor()`
  as Tauri ran it in `spawn_blocking`, then `open` in `this.update`.
- Rule: drops are ordered among themselves by the view's `drops` counter, the
  port of `dropCounter`: a drop whose resolve finishes after a later drop
  opens nothing. Unlike `pick_folder`, a drop is not dropped because `opens`
  moved on while it resolved; Tauri minted its folder token only after the
  path has resolved. `open` defers under a running trash or delete as for
  any open, so the drop has no gate of its own.
- Rule: GPUI fires `on_drop` only on a hovered hitbox, and the settings,
  the trash dialog and the folder dialogs are `occlude()`d layers, so
  a drop over an open dialog or the settings does not reach the root and is
  ignored (the settings layer starts below the title bar, so a drop on the
  bar itself is not; see the settings modal's pointer rule) (as a
  click in the tree is blocked by the same layers), where Tauri's drop had been
  window-wide except under its format gate.
- Rule: while an OS file drag is over the window, the main area (the tree,
  the preview and the meta pane, the Tauri `#main`) shows the outline of
  the Tauri `body.dragging #main`, 3px in `--ring`: `drop_outline`, an
  absolute child laid over that area, so the outline covers its content as
  the CSS `outline-offset: -3px` does instead of shrinking it. GPUI applies
  a drag-over style only to an element with a hitbox, and only while its own
  hitbox (`drag_over::<ExternalPaths>`) or its group's
  (`group_drag_over::<ExternalPaths>` on the root's `DROP_GROUP`) is under
  the drag, so the outline carries both: the first gives it the hitbox, the
  second lights it over the strip and the status row too. GPUI clears the
  drag on a drop or when it leaves the window (`FileDropEvent::Exited`
  refreshes the window).
- Not verified in the running app: the drop of a folder, of a file and of
  two folders, a drop while a dialog is open, and the outline drawing and
  clearing on leave and drop, on Windows (the pending hand check in
  `../plans/_archived/20261114-gpui-folder-drop/learnings.md`). macOS and Linux file
  drop delivery is covered by the CI build only.

### The folder tree is a `uniform_list` over rows kept in the view (Inferred)

The left pane (220 px by default, the Tauri `#side`; see the pane widths below) sits beside the preview canvas and
above the strip, which still spans the window, so `strip_clicked` still takes
the event's window `x` as the strip's. Its rows are `tree::Tree::rows`,
rebuilt into the view's `tree_rows` whenever the tree changes (never in
`render`), and drawn by a `uniform_list` over them: it lays out only the rows
on screen, its rows are gpui-component `ListItem`s (`tree_row`: one per row,
`selected` on the open folder, `secondary_selected` on the keyboard cursor
while the tree has the keys, indented `12 + 16 * depth` px, then a Lucide
`folder` / `folder-open` icon, open while expanded, the name and the count;
no chevron and no slot for one, as in gpui-component's `Tree`) with one
click listener on the row (`tree_row_clicked`; `ListItem` does not stop
propagation, so the pane's left press still gives the tree the keyboard), and
its `UniformListScrollHandle::scroll_to_item(.., ScrollStrategy::Nearest)` is
what the reveal and the tree's keyboard scroll a row into view with. A
`canvas` like the strip's would need its own hit testing and scroll model; the
strip needed one because `uniform_list` is vertical only.

- Rule: a left click on a row does what `tree::row_click` decides from the
  row's `expanded` / `expandable` and whether it is the open folder's row. A
  row that is not open and is collapsed or cannot expand opens; the open's
  reveal expands it. A row that is not open and is expanded opens and
  collapses at the click: `tree_row_clicked` collapses it and names it in
  `reveal_collapsed`, which the next `reveal` takes and, when it is of that
  folder (`tree::relation`), leaves the chain's end collapsed and unlisted
  (its spelling comes from its parent's listing, its count from the open's
  `set_listed_count`). The field survives an open deferred by the gate
  (`deferred_open`) or waiting in `reveal_wanted`, and a folder rename that
  lands under the deferred open (`folder_renamed`, the `Rename by Capture
  Date…` rename, whose run does not refuse the click) rebases it with
  `deferred_open`, so the renamed row stays collapsed; the tree's `Rename…`
  cannot land there, since it refuses the click while in flight and a held
  one is dropped by the deferred open. Any other reveal
  drops it, as does a later open of the folder by another route (a plain
  click, `Enter`, a pick or a drop), the last request winning. The open folder's row toggles through `tree_toggle` without
  reopening (no re-listing, no new scan), and does nothing when it cannot
  expand. A click on the open row's name without a modifier stops at the
  name (the slow click below); with Ctrl / Cmd / Shift it reaches the row and
  toggles. An open refused by `refuse_renaming` collapses nothing.
  Double-clicks are not special-cased. Seen (2026-11-22, Windows 11) for the
  clicks that were run: a collapsed and an expanded row that is not open
  open (the expanded one left collapsed), the open row's icon and count
  toggle it without a re-list, and its name arms the slow click. Not seen: a
  click on an open leaf row or on the open row's indent.
- Rule: tree nodes are keyed by the tree's own spelling (the root's, then one
  separator per level), never by the canonical `\\?\` path the index uses.
  The open folder's row is found with `tree::relation`, case ignored on
  Windows and macOS (`tree::IGNORE_CASE`).
- Rule: every open (`open`, from the command line or the tree) bumps the
  `opens` counter and reveals the folder: the chain from its root (a root
  added for a folder none holds) is expanded and listed one level at a time on
  `background_spawn`, respelled from each listing (the end left collapsed
  and unlisted after a row click that collapsed it, see above), and the end
  row is highlighted and scrolled to. A step that finds `opens` moved on stops, so a
  later open owns the tree. An open before the roots have been read waits in
  `reveal_wanted`.
- Rule: the same counter guards the listing of the open: its background task
  sets the folder watcher under a separate lock (never taken by the UI
  thread, which bumps the atomic counter without locking) only while its
  open is still the latest, and its result is dropped otherwise, so a quick second
  open is never overwritten by the first's late listing, nor left with the
  first folder's watcher.
- Rule: listings settle in order. Each one's background task first awaits the
  previous listing's `settled` signal (a oneshot kept on `Handpic`), then stops
  the previous scan and waits for the launch's eviction (`scan::Eviction`,
  which every listing waits on itself, since a dropped sender resolves
  `settled` too), then signals its own, so a
  second open never reconciles or scans while an earlier listing is still
  stopping the old scan or the eviction is still deleting rows.
- Rule: `ListItem` puts its children in a block `div().w_full()`, so a tree
  row's content is one `h_flex().w_full().min_w_0().overflow_hidden()` child
  of it, which keeps the name truncating and the count in the row.
- Rule: a tree row takes `w_full()` and the `uniform_list` carries the 4 px
  inset as its `px_1()`, never the row as a margin. A `uniform_list` item is
  a layout root, and taffy 0.15.0 stretches a root to the available width
  only when it is a block: a `flex()` row without a width is as wide as its
  content, so the name's `flex_1().min_w_0().truncate()` never truncates and
  the list clips the row at the pane edge. A margin next to `w_full()`
  overflows, since the percentage resolves against the whole available
  width. The padding is on the list, not the pane, so the scrollbar stays at
  the pane's edge. The row's right padding is `SCROLLBAR_TRACK` less that
  inset (12 px), so its content ends clear of the bar's track; its hover and
  selected fill still runs to 4 px from the edge, under the thumb. The cause
  is inferred from the gpui-pre 0.3.12 and taffy sources. Measured
  (2026-11-22, Windows 11): a long name ends in an ellipsis inside the pane
  at depths 2 to 4, the count stays whole next to a short and a truncated
  name, the cursor ring, hover and open-row highlights span the pane less
  the inset with rounded corners, and the vertical scrollbar sits at the
  pane's right edge. Measured (2026-11-27, Windows 11,
  `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): a
  drag of the scrollbar's thumb scrolls the tree; a held rename's long
  pending name truncates with its count whole; the left 0 to 5 px of the
  pane is the list's `px_1` padding and takes no press, the row starting at
  x = 6; the row's hover fill runs under the 16 px scrollbar track on the
  right, but a press there goes to the track, not the row.
- Rule: a row shows its file count, expanded or collapsed, whenever it is
  above zero, from one of two sources drawn the same. The listed count
  (`Node::raw_count`) is the last `read_dir` of the folder: an expand, a
  `Refresh`, a tree watcher change, and an open, which sets the open folder's
  node from `listed.files.len()` through `Tree::set_listed_count` (found by
  `relation` from `tree_open`, the tree's spelling, not the canonical
  `listed.dir`). Until a folder is listed in this session its row takes the
  cached count, the index's `folder_counts` row stored under the node's own
  path, which `Tree::set_cached_counts` keys by `fold` (case ignored on
  Windows and macOS). The listed count wins, a listed zero showing nothing; a
  folder never counted shows nothing. Since the key is the tree's spelling, a
  folder reached through a mapped drive, a symlink or a junction keeps its
  count across restarts; one folder reached through two spellings has a row
  for each. Measured (2026-11-22, Windows 11): the listed count, the cached
  counts after a restart, a folder renamed in the app keeping its count, a
  folder renamed outside the app and a new folder of an old name showing
  none until listed, and a folder never listed showing none. Measured
  (2026-11-27, Windows 11,
  `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): a
  junction and a folder on a mapped network drive keeping their counts after
  a restart, and an index whose `folder_counts` table was dropped showing no
  count on a collapsed row; a symlink was seen on macOS on 2026-11-26.
- Rule: every count the tree learns from disk is written to `folder_counts`
  under the tree node's path, the string handed to `folders::list`: each
  successful listing through `tree_listed` (an expand, `Refresh`, a tree
  watcher change, `Expand All`, a reveal's chain) and an open's or
  re-listing's count, under the paths `set_listed_count` returns (none while
  a reveal has not reached the node yet; its own listing writes it). A zero
  deletes the row. The writes go through `tree::CountWrites`, one
  `set_folder_counts` in flight on `background_spawn` under `index::lock` of
  the writable index, the counts learned meanwhile coalesced per folder with
  the latest winning; nothing is written without an index. A write still
  queued at quit is lost, not drained: it is a cache, and the folder's next
  listing writes it again. Measured (2026-11-27, Windows 11,
  `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): a RAW
  added outside the app and a `Refresh` of the collapsed folder showed the
  new count, kept after a restart.
- Rule: every successful tree listing (`tree_listed`'s `Ok` branch: an
  expand, `Refresh`, a tree watcher change, `Expand All`, a reveal's chain)
  also queues one prune of the listed folder (`Index::prune_folder_counts`):
  the rows under it whose first segment below it is not kept are deleted, the
  absent child's own row and every row under it. The keep set
  (`Tree::prune_keep`) is the listed children's paths plus, for each tree
  root `relation` puts under the folder, the folder's child leading to it, so
  a hidden root's rows survive. The paths are compared as spelled, byte for
  byte, against a parent prefix with exactly one separator (none added to
  `C:\` or `/`); the rows are selected under the prefix and the absent ones
  deleted by exact `dir`, so no SQL parameter is bound per child. A failed
  listing and an open's count prune nothing, and the prune reads nothing from
  disk. `CountWrites` keeps counts and prunes in one ordered batch, a
  folder's later count replacing its pending one and moving to the end, and
  `write_folder_counts` applies the batch in order under one `index::lock`
  guard, consecutive counts as one `set_folder_counts`; a failure stops that
  batch and is logged, and the next batch still goes out. The tree's
  in-memory cached map is not pruned; it is replaced at its next read.
  Measured (2026-11-27, Windows 11,
  `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): a
  counted folder deleted outside the app lost its row on a `Refresh` of its
  parent, and a new folder of that name showed no count after a restart.
- Rule: the cached counts are read whole (`Index::folder_counts`, the
  `folder_counts` table alone, never `files`) on `background_spawn` under
  `index::lock` of the reader, never touching a folder on disk, and replace
  the tree's map whole. They are read at startup and after `folder_renamed` /
  `date_folder_ran`, whose engine calls moved the rows (a rename's) or wrote
  them (a split's date folders, not listed yet); a write does not re-read,
  since a listed node shows its own count. The `count_reads` counter drops an
  answer older than the latest read.
- Rule: the engine keeps the `folder_counts` rows in step with a move, in the
  same call that moves the index rows. `rename::rename_folder` chains
  `Index::rename_folder_counts(old, new)` (`rename_dir`'s delete-under-`new`
  then `substr` update, on this table alone) after `rename_dir` under the same
  lock, so a failure of either takes the existing warning path.
  `datefolder::split_folder` writes the date folders' rows (each
  `Moved.count`, counted by the `folders::list` rule: the RAWs, or the JPEGs of
  a JPEG-only folder, each file once, sidecars not counted) and a zero for the
  original in one `set_folder_counts`, only after the file and index moves
  succeeded. The zero deletes the original's row even when undated files keep
  the folder, so its count comes back from its next listing. `Index::clear`
  counts the deleted `folder_counts` rows in `EvictSummary.rows`; `evict` and
  `evict_folder` leave the table alone. After the settings' `Clear Cache` the
  window re-reads the emptied table (`read_folder_counts`, bumping
  `count_reads` so a read from before the clear is dropped), which replaces
  the tree's cached counts whole; the listed counts of this session are not
  pushed back, so the table holds only what is learned afterwards (the open
  folder's re-listing first). A `CountWrites` write still in flight when the
  clear runs lands after it and re-adds a true row. A test asserting row order must sort
  the expected paths at run time, since `MAIN_SEPARATOR` sorts on opposite
  sides of `2` on Windows and elsewhere.
- Measured with the strip header (2026-11-22): the preview canvas's
  surround is 1158 x 767 logical px in the default 1600 x 1000 window with
  the tree shown at its default width, 1160 x 768 counting the pane borders
  and the header's line (see "The sort and filter menus are deferred
  popovers over the window"; before the header it was 1160 x 842).
  Inferred: 220 px wider with the tree hidden by `toggleLeft`, not
  measured. A pane change re-cuts a 1:1 crop through `zoom_resized`.

### The folder tree is not gpui-component's `Tree` (Inferred)

The folder tree (`main.rs`'s `tree_pane` / `tree_row`, `tree.rs`,
`treekeys.rs`) stays hand-built on a `uniform_list`. It is not replaced by
gpui-component 0.9.1's `Tree`, nor by gpui-base's unstyled `Tree` the styled
one wraps, which has the same click behavior, focus handle and key context.
The reasons come from reading `gpui-base` 0.9.1's `src/tree.rs` (`TreeItem`,
`TreeState`, the unstyled `Tree`) and `gpui-component` 0.9.1's `src/tree.rs`
(the styled wrapper) on 2026-11-15; nothing was built against the component,
so run-time behavior (the focus hand-over on a click, how the `"Tree"`
context meets the app view's) is not verified.

- `TreeItem::is_folder()` is `!self.children.is_empty()`. A folder not
  listed yet has no children, so it would need a dummy child to get an
  expander and to be toggled at all (`toggle_expand` returns early on a
  non-folder). Filling the children after a lazy listing or a disk change
  means `TreeState::set_items`, which sets both `selected_ix` and
  `right_clicked_ix` to `None`.
- A click on a row always selects it and toggles its expansion
  (`on_entry_click`: `selected_ix = Some(ix)`, then `toggle_expand`). Ours
  has no chevron either, but a click on a row opens the folder (collapsing an
  expanded row) and only the open folder's row toggles, its name arming the
  slow-click rename instead (`tree::row_click`).
- It has its own `FocusHandle` (`track_focus`) and the `"Tree"` key context.
  The app keeps one window focus and routes keys through the `tree_keys`
  flag and `treekeys::tree_gate` (see "The tree takes the keyboard through
  one flag in the view").
- Its keys differ. `up` / `down` wrap at the ends (`on_action_up` goes from
  the first row to the last, `on_action_down` from the last to the first);
  ours stop (`tree::step`). Its `left` / `right` collapse and expand, where
  ours leave Left / Right to the strip's paging and toggle on Space. There is
  no Home / End and no type-ahead. `Confirm` toggles the selected folder instead
  of opening it, and `gpui-base`'s `tree::init` binds only the four arrows in
  the `"Tree"` context, so Enter does nothing there unless the app binds it.
- It tracks one selected index and one right-clicked index. The folder tree
  needs the keyboard cursor and the open folder's row apart.
- The styled wrapper's `context_menu` wraps the `.context_menu()` extension,
  which "The right-click menu is gpui-component's `PopupMenu` at the
  pointer" says why the app does not use.
- A `TreeItem` holds its whole subtree (`children: Vec<TreeItem>`) and its
  expansion in an `Rc<RefCell<..>>`. That state would duplicate `tree.rs`'s
  `Tree`, which stays for the lazy listing, the failed listings, `respell`
  and the rename.
- What it would give: virtualization and the scrollbar, which the tree
  already has (`uniform_list` plus `vertical_scrollbar`, see "The strip, the
  tree and the meta pane draw the theme's scrollbar"), and gpui-base's
  accessibility roles (`Role::Tree`, `Role::TreeItem`, `aria_selected`,
  `aria_expanded`).
- Still possible, not adopted: drawing only the rows with
  `gpui_component::list::ListItem` (the theme's list colors), as a small
  later step.
- Rule: a `gpui-component` version bump that prompts a second look re-reads
  the points above in both crates' `src/tree.rs` before reopening the
  decision; the behavior listed is 0.9.1's.

### The panes are one `Panels` value, stored under `panels` (Inferred)

`panels.rs` is the port of `panels.ts` and of the Tauri `panels_setting`:
`Panels { left, strip, right }`, all shown by default, `toggle(Panel)`,
`toggle_sides` (Lightroom's Tab: hide both side panes when either is shown,
else show both), `from_setting` and `setting`. `settings.rs` reads it into
`Settings.panels`, and the view holds it as `panels`. `toggleLeft`,
`toggleRight`, `toggleStrip` and `toggleSides` all go through
`change_panels`, the port of `changePanels`.

- Rule: `change_panels` sets the panes, hands the keyboard back from the tree
  when the left pane is now hidden (by `toggleLeft` and `toggleSides` alike),
  re-cuts a 1:1 crop (`zoom_resized`), writes `panels` as
  `{"left": bool, "strip": bool, "right": bool}` and repaints. The preview
  canvas is `flex_1`, so it takes the space a hidden pane frees.
- Rule: the stored value is the Tauri key and shape, so a store seeded from
  the Tauri `settings.json` comes up with the Tauri app's panes; a missing or
  non-boolean entry, or a value that is not an object, means shown.
- Rule: under `--auto` every pane starts shown whatever the store holds, and
  nothing is written to `panels` (`remember_panels` off, set in `main` on the
  value `Handpic::new` returns, inside the `cx.new` closure). It is not a
  parameter because `Handpic::new` already takes seven arguments, and an eighth
  trips `clippy::too_many_arguments` (`-D warnings` in CI). Another
  launch-only suppression flag has the same choice: set a field after `new`, or
  add `#[allow]` as `shared-crates.md` does for `Writer::set`.
- Rule: hiding the strip removes its pane, the header with `Sort` / `Filter`
  included, as in the Tauri app, where the menus sat in the strip's header,
  and closes an open menu; the status row (the status line, the position and
  the circle) stays. The strip's bookkeeping (range, thumbnails) keeps
  running while it is hidden.
- Not measured: the toggles on macOS and Linux are covered by the CI build
  and the unit tests only.

### The side panes' widths are one `PaneWidths`, stored under `panelWidths` (Inferred)

`panewidths.rs` holds `PaneWidths { left, right }` in logical px, `DEFAULT`
220 (the former fixed `PANE_WIDTH`), each pane within `MIN` 160 and `MAX`
480, and `PREVIEW_MIN` 240 for the preview. The view holds it as
`pane_widths`, apart from `Panels`, so hiding a pane and showing it again
gives it its last width with no code.

- Rule: not gpui-component's `h_resizable`. Its `ResizablePanelGroup` keeps
  the sizes in its own `ResizableState`: a saved width only goes in as a
  panel's initial `.size()`, a hidden pane either stays an empty panel whose
  neighbour still draws a handle or has its size truncated away when left
  out, a container resize rescales the panels that are not `fixed`, and the
  clamping sits out of reach of a unit test (gpui-base 0.9.1's
  `resizable/mod.rs` and `panel.rs`).
- Rule: the handles are gpui-component's standalone `resize_handle`
  (`pane_handle` in `main.rs`, payload and dragged item `PaneDrag(Panel)`,
  drawn as nothing; GPUI tells drags apart by payload type and
  `ExternalPaths` is the folder drop's). Both sit inside the preview
  container, which is `relative`: the left one `HandleEdge::Leading` while the
  tree is shown, the right one `HandleEdge::Trailing` while the meta pane is,
  so they stay off the panes' own scrollbars and the panes change only their
  width. The band takes the preview's outer 5 px and draws a 1 px line on its
  edge.
- Rule: the root `div`'s `on_drag_move::<PaneDrag>` follows the drag (capture
  phase, every move while the drag is active, wherever the pointer is). The
  boundary moves by the pointer's travel from the press, keeping the press's
  offset from the line: a left-button `on_mouse_down` on each handle
  (`press_pane`; the band's own press listener does not stop propagation)
  records the pointer's `x` and the pane's drawn width (`drawn`, so a pane a
  small window narrows does not jump) as the view's `drag_press`, and
  `panewidths::wanted` gives the start plus the travel for the left pane and
  the start less it for the right one, which grows as the pointer goes left.
  No window geometry enters it, so the Linux client-side frame's padding does
  not either. `dragged` rounds that width to whole px, clamps it to
  `MIN`..`MAX` and to what leaves the preview `PREVIEW_MIN` beside the other
  shown pane's stored width. A press without a drag leaves its record, which
  the next press replaces. The first moves of a drag change nothing: GPUI
  starts a drag only on a move more than its private 2 px `DRAG_THRESHOLD`
  from the press, and the first `DragMoveEvent` is the move after that one;
  the boundary then catches up to the press-relative position at once
  (Measured on Windows 11, 2026-11-22, from gpui-pre 0.3.12's
  `elements/div.rs`: the first 1 to 4 px of a drag moved nothing, then the
  line kept the press's offset from the pointer in both directions and
  returned to its start when the pointer returned to the press, on the tree's
  boundary, the meta pane's and the strip's top edge; the handle's
  `on_mouse_down` fires in the running window, so the first-`DragMoveEvent`
  fallback is not needed. macOS and Linux are not yet seen). A move that
  changes the widths calls `zoom_resized` (the 1:1 crop is re-cut after
  `CROP_RESIZE_DEBOUNCE`, its timer replaced on every move) and repaints; the
  canvas's `viewport` and the peaking layer follow the canvas's own bounds,
  and the strip spans the window, so nothing else needs telling.
- Rule: the stored widths are the user's; `drawn`, called in `render` with the
  window's width, gives the widths drawn. When the window cannot hold the
  shown panes and `PREVIEW_MIN`, each shown pane gives up the excess in
  proportion to its width over `MIN`; no pane is drawn under `MIN`, so a
  window too narrow for both minimums and `PREVIEW_MIN` takes the rest from
  the preview. Widening the window brings the stored widths back.
- Rule: the widths are written at the drag's end, once. `on_drop::<PaneDrag>`
  on the root would miss it: a drop listener fires only while its hitbox is
  hovered and the handle's band occludes the root, and the pointer is on the
  band at most releases. The root instead listens to every release in the
  window with `capture_any_mouse_up` (hovered) and `on_mouse_up_out` (left
  button, not hovered); `drag_pane` sets `pane_dragged` and the first release
  after it writes `panelWidths` as `{"left": n, "right": n}`.
- Rule: a missing or non-object value, or a missing, non-number or
  non-finite entry, means 220; a number out of range is clamped. Under
  `--auto` the panes start at 220 whatever the store holds and nothing is
  written (`remember_panels` off, as for `panels`).
- Seen (2026-11-22, Windows 11): `SIZEWE` on the preview's outer 5 px beside
  each pane (x 221 to 226 with the tree's line at 220, about 1374 to 1380
  across reads with the meta pane's line at 1379; a press at 1380 moved
  nothing while 1379 and 1376 worked, so aim a synthetic press at the meta
  pane's boundary on or left of its line); the pill on hover; every
  release, outside the window included, writing the widths at once (Windows
  delivers the release); the 1:1 re-cut after a drag; the tree at 160 px.
  The bands against the no-folder message in a narrow window are seen on
  Windows (2026-11-27, see "The preview area says why it shows no photo"; a
  press at x 485, 5 px inside the tree's line at 480, already opened the
  picker, so the band's hit edge sat about a pixel nearer the line than the
  cursor's). Not yet seen: macOS and Linux. the fix for the second finding of that run, words broken mid-word at
  160 px, is Measured (see "A meta pane value that does not fit beside its
  label goes under it") (results in
  `../plans/_archived/20261122-windows-device-checks-resize-scrollbars/results.md`);
  the press's offset was lost then, before the drag moved by the pointer's
  travel from the press; the fix is Measured on Windows (see the drag rule
  above; macOS and Linux remain under the drag-resizable tree and meta panes
  checks). One first drag after a launch (no pointer move over the window
  before the press) did not move the boundary at all and did not recur in
  later drags or after a relaunch; not reproduced, so if a first drag after
  launch does nothing, move the pointer over the window and retry before
  suspecting the code (`../plans/_archived/20261122-pane-drag-offset-meta-wrap/learnings.md`,
  Wrap-up).

### The strip's height is one number, stored under `stripHeight` (Inferred)

`stripheight.rs` holds the strip's height in logical px, its cells' area:
`DEFAULT` 128 (the Tauri strip's, the former fixed `STRIP_HEIGHT`), `MIN` 96,
`MAX` 302 (a 270 px high image box, the stored thumbnail's height) and
`PREVIEW_MIN` 240 for the preview. The drawn strip pane is `HEADER` (32 px)
taller, the header row holding `Sort ▾` / `Filter ▾` over the cells, and
`GUTTER` (9 px) taller again, the room under the cells for the horizontal
scrollbar's track (see "The strip, the tree and the meta pane draw the
theme's scrollbar"); the cells' `div` is `GUTTER` taller than the cells, and
the room the header's menus are capped at counts it. The stored value still
means the cells' height, so it needed no migration. `Strip` holds the cells' `strip::Geometry`, which
`Geometry::of(height)` derives: the image box takes the height less the
7 px `STRIP_PADDING` above and below, the 16 px `NAME_HEIGHT` and the box's
1 px border each side, at 3:2 with its width rounded to whole px; the cell is
the box plus the border, the pitch the cell plus the 8 px gap. At 128 that is
the Tauri strip's 144 x 96 box, 146 x 114 cell and 154 px pitch. Only the
image box scales; the name, the stars, the flag dot, the burst badge and the
face icon keep their sizes. The box stays 3:2 and the default 144 x 96
rather than the 144 x 144 of the Tauri strip's `img`, which held a portrait
thumbnail at 96 x 144 (the user's decision, 2026-11-23): a portrait shows at
64 x 96 by default, and a taller strip enlarges it.

- Rule: every reader of the strip's layout takes the current geometry:
  `Strip`'s offset, range, hit test, request order and content width read
  its own, `Strip::new` is handed `self.strip.geometry()` on every rebuild,
  and `carried_offset` (the pitch), `tip_regions`, `paint_strip` and
  `paint_burst_band` take it as a parameter.
- Rule: the handle is a standalone `resize_handle` with `Axis::Vertical` and
  `HandleEdge::Leading` (`strip_handle` in `main.rs`, payload and dragged
  item `StripDrag`, drawn as nothing), the last child of the strip pane's
  `relative()` wrapper (the header, then the cells' `div`): it spans the
  window, goes with the strip when it is hidden, takes the header's top 5 px
  (its padding, the buttons starting 6 px down) and draws a 1 px line on the
  pane's top edge, its boundary with the preview.
- Rule: the middle row and the strip sit in one wrapper `div` (a `flex_col`
  with `flex_1`), whose `on_drag_move::<StripDrag>` follows the drag (capture
  phase, every move while the drag is active, wherever the pointer is). Its
  `DragMoveEvent::bounds` are the wrapper's, so its bottom is the strip's
  bottom edge and its height what the preview and the strip share, with no
  assumption about the status row's height or the Linux frame's padding
  below. The wanted pane height is the pane's height at the press less the
  pointer's travel from it, the pane growing as the pointer goes up: the
  handle's left-button `on_mouse_down` (`press_strip`) records the pointer's
  `y` and `HEADER` + the cells' height + `GUTTER` in the view's `drag_press`,
  and `stripheight::wanted` gives the height, so the press's offset from the
  edge is kept; the first moves change nothing, as for the panes (GPUI's
  2 px `DRAG_THRESHOLD`; Measured on Windows 11, 2026-11-22: the first 1 to
  4 px stayed still, then the edge kept the press's offset in both
  directions, y 804 pressed at line 801 gave 809 for 812 and 737 for 740). `dragged` takes the header and the gutter off it, rounds the cells' height
  to whole px, clamps it to `MIN`..`MAX` and to what leaves the preview
  `PREVIEW_MIN` with the header and the gutter counted, never under `MIN`.
- Rule: a move that changes the height calls `Strip::set_geometry`, which
  keeps the cells in view: the current cell's center stays where it is on
  screen when that cell overlaps the viewport, else the cell at the
  viewport's left edge stays there, then the offset is clamped to the new
  content. Then `strip_moved` (the range's thumbnails released and
  requested, the scan's hot list) and `zoom_resized` (the 1:1 crop re-cut)
  run and the view repaints.
- Rule: the height is written at the drag's end, once, through the same
  release listeners as the side panes' widths (`end_pane_drag`, which also
  takes `strip_dragged`), as one number under `stripHeight`. A missing,
  non-number or non-finite value means 128; a number out of range is
  clamped. Under `--auto` the strip starts at 128 and nothing is written
  (`remember_panels` off).
- Rule: the view keeps the height the user set (`strip_height`, from the
  stored value, `DEFAULT` under `--auto`) apart from the drawn geometry,
  which is `stripheight::drawn(strip_height, shared)`: no taller than leaves
  the preview `PREVIEW_MIN` with the header and the gutter counted (the
  bound `dragged` keeps), never under `MIN`. `shared` outside a drag is
  `shared_height` in `main.rs`, the window's viewport height less the
  client-side frame's top and bottom padding, `TITLE_BAR_HEIGHT` and
  `STATUS_ROW_HEIGHT`, computed rather than read from the wrapper's bounds so
  it holds from the first frame. It matched the wrapper's bounds on every
  frame on GPUI's test platform (Windows, 936 = 936); Linux with client-side
  decorations (non-zero `window_paddings`) and macOS are unmeasured, so a
  preview a few px off `PREVIEW_MIN` in a short window there points at this
  arithmetic first. `Handpic::new` builds the strip at the drawn
  height and `observe_window_bounds` runs `fit_strip`, which calls
  `Strip::set_geometry` when the drawn height changes (the existing
  `strip_moved` and `zoom_resized` there follow it). A drag sets
  `strip_height` to the dragged height, `press_strip` starts from the drawn
  height, and only a drag writes `stripHeight`, so a shorter window draws
  the strip shorter, a taller one gives the user's height back, and a resize
  writes nothing.
- Seen (2026-11-22, Windows 11): `SIZENS` on y 802 to 807 under the line at
  801; the releases writing the height at once, outside the window
  included; `--auto 40` starting at 128 with 199 stored and writing nothing;
  the 302 px cells drawn at about 1:1 at scale 1. Not yet seen: a HiDPI
  display, and macOS and Linux (results in
  `../plans/_archived/20261122-windows-device-checks-resize-scrollbars/results.md`).

### The tree follows the disk through one `TreeWatch` (Measured)

The view holds one `handpic_engine::treewatch::TreeWatch` whose callback sends
`Event::TreeChanged(path)` (the tree's spelling) into the scan's channel.
`tree_changed`, which every change of the tree runs, computes
`Tree::watched_folders` and, when it differs from the set last handed over,
hands the whole set to `TreeWatch::set` on `background_spawn`, each hand-over
awaiting the one before it (the Tauri `set_tree_watches` chain), so two sets
never apply out of order. `TreeChanged` re-lists the node only while it is
still expanded and drawn (the event and a collapse can cross); a failed
re-list is logged and leaves the node as it was, since deleting a folder
usually fails its own re-list before its parent's removes the row. The
menu's `Refresh` re-lists the folder expanded or not and forgets the set
last handed over, so the next change sends it again and a watch that failed
is retried; on the open folder it also re-lists the folder as a watcher
change would (through `Resync`). The roots are read again whenever the
window becomes active (`observe_window_activation`, once the first read has
answered); `add_roots` skips the roots already shown.

- Measured (a throwaway test against `crates/app/Cargo.lock`'s `notify`
  8.2.2 on Windows): `set` of one folder took 0.4-0.6 ms, a collapse
  (`unwatch` then `settle`) 0.1-0.3 ms, five collapse / expand rounds each
  stopped and resumed the events (none for a change while collapsed, one
  ~500 ms after a change once expanded again), and renaming the folder's
  parent after the last collapse succeeded, so the handle was released. The
  `configure` that queued behind `Unwatch` (`shared-crates.md`) does not stall
  here. `TreeWatch` is an engine test, which cannot run from the app
  workspace (a path dependency with dev-dependencies is not testable there),
  hence the throwaway.
- Rule: on Windows a watch pins its folder's ancestors against a rename;
  the folder rename and the date split go through the engine's
  `rename::rename_folder` / `datefolder::split_folder`, which call
  `with_released` (see "A folder rename releases the watches, then follows in
  the tree and reopens").

### The right-click menu is gpui-component's `PopupMenu` at the pointer (Inferred)

`context.rs` holds the menus' items (`folder_groups`, `strip_groups`) and
what a menu acts on (`Target`, a tree folder or the strip's focused file and
targets); `main.rs` builds them into a gpui-component `PopupMenu` and places
it. A right press (`on_mouse_down(MouseButton::Right, ..)`) on a tree row
opens the folder menu, and one on a strip cell (the strip's wrapping `div`,
`strip::hit_test` giving the cell) the strip's menu, closing the status
row's menus. `open_context` builds the menu with `PopupMenu::build`: a
`separator()` between groups, one `PopupMenuItem` per item with
`.checked(..)`, `.disabled(..)` and an `on_click` that updates the view
through a `WeakEntity` with the item's action and the target taken at the
open; `.min_w(180 px)`, `.scrollable(true)` and `.max_h` of the height cap
as the window stands at the open; `.action_context(self.focus)`. It focuses
the menu at once, as `ContextMenu` does, and keeps the entity, the pointer's
window position and the `DismissEvent` subscription together in the view's
`context` field (`ContextOpen`), the subscription clearing it, so a replaced
menu's late dismiss cannot clear the new one.

`context_layer` draws it as `div().absolute()` holding
`deferred(anchored().position(at))` at `POPUP_PRIORITY` (`gpui_kit::base`,
the layer priority `ContextMenu` gives its menu): `anchored`'s default
`AnchoredPositionMode::Window` takes the `MouseDownEvent`'s window position
as is, and its default fit, `AnchoredFitMode::SwitchAnchor`, puts the menu
on the pointer's other side when it overflows an edge and then still snaps
it inside, as the Tauri `menuPosition`.

- Rule: the `.context_menu()` extension (`ContextMenuExt`) is not used. It
  places its menu with `snap_to_window_with_margin` at a fixed
  `Anchor::TopLeft`, which only shifts and never flips, so a menu opened on
  the strip at the window's bottom would cover the pointer; its open state
  lives in element state the view cannot close (a key, a folder dialog); and
  it opens on any right press in its hitbox, where the strip's empty area
  must open nothing.
- Rule: the menu closes itself on a press outside (its own
  `on_mouse_down_out`, the capture phase, so a right press on another row
  dismisses it and then opens that row's in the bubble phase), on Escape and
  after any item runs (`activate` always dismisses); each dismiss gives the
  focus back to the `action_context`, the view's handle, unless the item's
  handler already moved it (the inline `Rename…` focusing its `Input`, a
  dialog). It binds Up / Down (move the highlight) and Enter (run it) in its
  `PopupMenu` key context, which act before `key_down`; any other key bubbles
  from the menu's focus to the view's root `div` (the deferred layer keeps
  its parent's dispatch node), where `key_down` drops the menu, refocuses the
  view and then acts as usual, as in the Tauri app. Opening a folder
  operation's dialog drops it too.
- Rule: the component's look: 14 px text, the check column only when some
  item is checked, a disabled item muted and inert, and a scrollable menu
  that overflows opens scrolled to its first checked item. A standard item
  draws a shortcut only from an action bound in GPUI's keymap, which the
  app's keymap is not, so an item with a shortcut is a
  `PopupMenuItem::element` drawing its label and, at the right end in
  `muted_foreground` at `text_xs`, `Kbd::new(keys::keystroke_of(..))
  .appearance(false)` (the formatted text alone, as gpui-component's
  `Tooltip` draws its key binding). The strip menu's color label items
  (`context::Item`'s `tag`) draw their name inside the same small `Tag` as the
  filter menu's label rows, in place of the plain text.

- Rule: the strip menu's judgment groups follow the filter menu
  (`menu::filter_rows`), not `context.ts`: the flags run Pick, Unflag,
  Reject, the rating runs from 5 stars down to the unrated (`clear`), and the
  labels Red to Purple, then `No label`. An item's label is a `menu::Label`,
  and the rating rows are its `Stars`, drawn by the same `menu_label` helper
  the filter menu's rows use (`on` lit `STAR_FILLED` in `STARS_COLOR`, then
  dim `STAR`, the unrated five dim stars; a `PopupMenuItem::element`), so the
  two menus cannot drift. A test in `context.rs` pins the order to
  `filter_rows`'. The filter menu's flag dots, label squares and section
  headings are not drawn here (the user's decision).
- Rule: the strip's right-click follows the Tauri strip's `contextmenu`
  handler: a cell outside the selection becomes the selection alone, one
  inside keeps it, and the focus moves to the cell either way; the strip's
  empty area opens nothing. The menu's targets are `selection::targets`,
  taken when it opens. Its judgment items carry the keymap's action names
  (`Action::Judge`) and run through `Command::from_action` and `judge` as a
  key does, so they act on the selection as it is when picked; each item's
  shortcut is its action's first key, carried as the keymap's name and
  spelled by `Kbd::format` (`ctrl+alt+1` as `Ctrl+Alt+1`, `space` as
  `Space`, `⌘A` on macOS), not `keys.ts`'s `displayKey` and not
  `Keymap::accelerator_for`, which spelled keys for the Tauri menu bar.
- Rule: the reveal item calls `cx.reveal_path`, which on Windows selects the
  item in its parent through the desktop folder's `ParseDisplayName` (`open_target_in_explorer`)
  and only logs a failure. A root (`D:\`) has no parent, so the folder menu
  opens it with `cx.open_with_system` instead. The tree's paths never carry
  the verbatim `\\?\` prefix, but the strip's files do (the index's
  canonical folder). The shell refuses a prefixed path: `SHParseDisplayName`
  returned `E_INVALIDARG` (`0x80070057`) for `\\?\C:\Windows\notepad.exe` and
  `S_OK` without the prefix (measured from PowerShell, 2026-11-14), so the
  strip's reveal passes `context::shown_path` (`trash.ts`'s `shownPath`).
- Rule: the copy items write the tree's spelling of the path and the row's
  name, or the strip's targets' listed paths (prefix and all, as the Tauri
  app wrote them) and file names, one per line, through
  `cx.write_to_clipboard(ClipboardItem::new_string(..))`.
- Not verified headless: the hand checks are pending in
  `../plans/20261113-gpui-tree-trash/learnings.md` and, for the `PopupMenu`
  version of these menus (the pointer-side flip, the key routing with a menu
  open, the hand-back of the tree's keyboard), in
  `../plans/_archived/20261116-native-context-menu/learnings.md`.

### The tree takes the keyboard through one flag in the view (Inferred)

The window keeps its one `FocusHandle`; `tree_keys` on the view says whether
the tree has the keyboard, and `key_down` checks it right after the
right-click menu's close and before every other arm (the literal `home` /
`end` / `q`, the status row's menus' `escape`, the keymap's actions). A
second `FocusHandle` on the pane would give the same routing, but the gate
must also sit before those literal arms in the window's handler, and a press
outside the pane must still reach its own handler, so a flag and the pane's
mouse listeners were the smaller change.

- Rule: a left press in the pane (`on_mouse_down(Left)`) gives the tree the
  keyboard; a press of any button outside it (`on_mouse_down_out`, the
  capture phase, so the press still reaches its target) hands it back; a
  right press in the pane does neither. A press on the right-click menu hands
  it back too, as a Tauri menu button took the focus: the menu may be drawn
  over the pane, which `on_mouse_down_out` (a bounds test, not occlusion)
  does not count as outside, so a `canvas` over the menu's wrapper registers
  a capture-phase `on_mouse_event` that tests the press against its bounds
  (the menu is `occlude()`d, so a hitbox listener on the wrapper, inserted
  before the menu's, would never be hovered). The menu taking the GPUI focus
  leaves the flag alone.
  `escape` and hiding the pane (`toggleLeft`, `toggleSides`) hand it back as
  well. The
  type-ahead buffer starts over whenever the keyboard moves.
- Rule: while the tree has it, `tree_key_down` takes `escape`, Up / Down,
  `home` / `end`, `enter` (open the cursor's folder), `space` (toggle the
  cursor's folder, when it can expand) and any printable key without Ctrl /
  Alt / Meta (type-ahead); any other key runs its action only when
  `treekeys::tree_gate` lets it through (`open`, the four pane toggles and the
  strip's `previous` / `next`, `TREE_PASSTHROUGH`) and is otherwise
  swallowed, so `q`, the judgment keys, the burst moves and undo never reach
  the photo. The cursor row is outlined (`ListItem`'s `secondary_selected`,
  `selection`) only while the tree has the keyboard, which Left / Right paging leaves with the tree.
- Rule: the settings key (`settingsmodal::OPEN_KEY`, `ctrl+,` / `meta+,`)
  opens the settings while the tree has the keyboard: it is no keymap
  action, so `tree_gate` would swallow it, and `key_down` lets it past the
  gate by name. An inline rename still owns every key
  (`rename_owns_keys` returns before the gate), so it does not open them
  there.
- Rule: this departs from the Tauri app on purpose (the user's request).
  There, Left / Right were the WAI-ARIA tree keys (expand, collapse, first
  child, parent); here they page the strip whatever has the keyboard, the
  cursor reaches a parent or child only through Up / Down, Home / End and
  type-ahead, and plain Space is the expand / collapse toggle, so it is no
  longer a type-ahead character (type-ahead cannot match past a space in a
  folder name). Modified arrows (`shift+arrow`, `alt+arrow`) stay
  swallowed. A parity pass with `tree.ts` / `treekeys.ts` keeps this.
- Rule: opening a folder from the tree, by a click on its name or `enter`,
  leaves the keyboard with the tree, as in the Tauri app (`folders.ts`'s
  `openFolder` and `main.ts`'s `openDirectory` never blur the tree); the
  open's reveal puts the cursor on the opened folder.
- Inferred from `gpui_windows`' `process_key`: type-ahead takes the
  keystroke's own `key_char` (`keys::typed_char`), since `key_name`
  lower-cases a shifted letter and names the space bar `space`. `key_char` is
  `ToUnicode`'s character with a control character dropped, so `enter`,
  `tab` and `escape` type nothing. Type-ahead with an IME on is not a goal.
- Rule: `Expand All` walks one listing at a time on `background_spawn`, depth
  first through an explicit stack of chains (no async recursion), skips a
  link (`is_link`), stops a branch whose chain was collapsed or dropped
  meanwhile and marks and reports a folder that fails, going on with the
  rest. `Collapse All` is `Tree::collapse_all`. The watch set follows through
  `tree_changed`.
- Seen on Windows 11 on 2026-11-22: a click on a row does not itself move the
  cursor, which follows an open (`tree_cursor = tree_open`) and the arrow
  keys, so a click on the open row's icon or count toggles it and leaves the
  cursor where it was; `enter` opens the cursor's folder and the tree keeps
  the keyboard; Right / Left page the strip with the ring on the cursor;
  `3`, `p`, `shift+arrowleft`, `alt+arrowleft` and `ctrl+z` are swallowed and
  `q` does not quit; `escape` and `toggleLeft` remove the ring.

### The trash dialog is a component `Dialog`, its focus in GPUI (Inferred)

The strip menu's `Move to Trash…` and `Delete Permanently…` open one
gpui-component `Dialog` in `Root`'s layer (`window.open_dialog`), drawn by
`main.rs`'s `trash_dialog` inside `modal.rs`'s shell: the title, the line and
a footer with a `danger()` run `Button` and a default Cancel `Button`, each
on a focus handle the view keeps (`trash_buttons`, through `track_focus`).
`trash::TrashFlow` holds the phase (idle, asking, running), the button the
dialog opens on (`initial_focus`: a move's run button, a delete's Cancel) and
what a key does (`key`); the focus itself is GPUI's.

- Shell (`modal::shell`): `.keyboard(false)`, since with the `Dialog` key
  context on, `enter` is bound to `Confirm` and runs `on_ok` whichever button
  has the focus, before any key listener; `.overlay_closable(false)`,
  `.close_button(false)`, `.entrance(DialogEntrance::None)` (no entrance or
  exit animation, so a close removes it in one frame) and `on_ok` /
  `on_cancel` returning false, so no path of the component's closes it
  behind the flow's back. `.overlay(false)` as well: the component's
  backdrop is a `WindowControlArea::Drag` area, which on Windows answers
  `WM_NCHITTEST` with `HTCAPTION` (`gpui_windows`' `events.rs`), so a press
  on it would drag the window and a double-click maximize it. The view draws
  `modal::backdrop` in its place while the flow is busy: a `deferred`
  `theme.overlay` layer under the dialog (the dialog's host is `deferred` at
  priority 10 and up) whose capture-phase mouse-down listener calls
  `prevent_default`, because the dialog's host tracks its own focus handle
  and would take the focus from the focused button on any press outside the
  buttons (the `Button`s already prevent it on their own press). The
  component's full-window `dialog` `div` occludes the window, so nothing
  under it is pressed, hovered or scrolled.
- Open and close (`modal::Modal`): the view calls `follow_trash` wherever the
  flow changes phase (the start, a dismissal, a run held for a scan's end,
  the run's end); it opens on the flow becoming busy and closes on it going
  idle, through `cx.defer` and the window's `AnyWindowHandle`, so a run's end
  reached from a background task without a `Window` takes the same path and
  the change lands before the next frame. The builder `open_dialog` takes is
  a render closure run on every rebuild of the dialog layer (every frame the
  window draws: `WindowState`'s view is not cached), so it reads the flow
  through a `WeakEntity` each time, and asking to running redraws the
  buttons disabled with the view's own `cx.notify()`. The initial focus is
  set on the builder's first run of each opening only (a `Cell<bool>` per
  opening), after `open_dialog` has focused the dialog's shell.
- Keys: a key pressed in the dialog never reaches the window's `key_down`
  (the dialog is outside the app's view), so the settings key and every
  other keymap key are refused there by construction; `key_down` still
  returns at once while the flow is busy. `Tab` / `Shift+Tab` are `Root`'s,
  walking the dialog's focus trap between the two buttons. Every other key
  reaches the footer's `capture_key_down`, which runs before the focused
  button's own listeners and asks `trash_key_down` (`TrashFlow::key` over
  `keys::key_name` and `KeyDownEvent::is_held`): Escape dismisses while
  asking; Enter and Space go on to the focused `Button`, which presses itself
  (GPUI's keyboard click: the key down recorded, the click fired on the key
  up with the focus unmoved, neither with a modifier); anything else, and
  Enter, Space and any repeat in a delete, is stopped with `prevent_default`
  and `stop_propagation`, so the button never records the key down.
- Rule: the delete dialog takes only a single mouse click on its run button
  as a yes. `ClickEvent::click_count` is 1 for a keyboard click at the
  pinned rev (`interactive.rs`), so the handler passes 0 when
  `is_keyboard()`, the DOM's `detail`, and `confirms_delete` refuses it and
  the rest of a double-click.
- Rule: while a run is in flight (not held) the dialog stays open with both
  buttons disabled. A disabled `Button` that has the focus keeps it (it
  leaves the tab order only), so Escape still reaches the footer's listener
  and does nothing; a Tab then finds no stop in the trap and `Root` focuses
  the dialog's host, where no key does anything either.
- Rule: a held `tab` is not swallowed in the delete dialog: `Root`'s binding
  is dispatched before any key listener, which alone sees `is_held`. It can
  only move the focus between the buttons; no key confirms a delete.
- Rule: the dialog gives the focus back on close to what had it when it
  opened, the app view's handle; the tree's keyboard is a flag in the view
  (`tree_keys`), which the dialog never touches.
- Seen on Windows 11 on 2026-11-22: in the delete dialog `enter` and `space`
  on `Delete Permanently` do nothing and one click deletes; a double-click on
  the strip menu's `Delete Permanently…` opens the dialog and deletes
  nothing; a double-click on the dialog's own button deletes on its first
  click, since the double-click `confirms_delete` refuses is only the rest of
  the one that opened the dialog.

### A trash or delete goes through the gate, and nothing lists under it (Inferred)

The app has no `Scans` lock; one view thread decides when a listing
starts, so the guard is `idle::IdleGate` plus `resync.rs`. A confirmed run
asks the gate with "busy" being `Resync::busy` (a listing or its scan under
way) or a run in flight: idle, it runs at once on `background_spawn`
(`judgment::Queue::drain`, then `Writer::flush`, both `DRAIN_TIMEOUT`, then
`trash::trash_listed` with the view's `Arc<trash::Runs>` or
`trash::delete_listed`, progress through the view's event channel as
`Event::TrashProgress`); busy, it is held, the dialog closes and the scan is
canceled without a join (`Running::cancel`), so it still ends through `Done`
and `FacesDone`, whose `finished` drains the gate first. A run held while the
listing itself ran has no scan yet; `opened` cancels the scan it starts when
`cancels_scan(gate.waiting())`.

- Rule: while a run is in flight the gate's in-flight count is up and
  `Resync::hold` keeps the resync busy, so a watcher change or a tree
  `Refresh` is deferred; `relist` returns at once (a sidecar format switch);
  an `open` (the tree's) is kept in `deferred_open` and runs at the run's
  end. The run's end calls `Resync::open` (dropping the pending change) and
  re-lists once, which also rescans what a canceled scan left. `TreeChanged`
  lists a tree folder only, never the open one.
- Rule: every caller of `relist` (a sidecar format switch included) calls
  `Resync::open` first, so `Resync::busy` covers the listing it starts and a
  trash or delete waits for it.
- Rule: a folder open discards a held run; a held run that drains while a
  dialog is asking is dropped, as the Tauri closure checked `trashFlow.busy`.
- Rule: the run's result goes to the pane: `trashed_status` in the
  `step_note` line (the Tauri `note`), each failure through `write_failed`
  (`<name>: could not move to the Trash: <message>`), kept apart from the
  judgment write failures and the folder operations' in `trash::Failures` by
  their `Origin`: the next trash-family run's start (`run_trash`,
  `undo_trash`, `redo_trash`, past their guards, and a `Delete Sidecars…`
  run's) drops the `Trash` lines, the undo's `could not restore from the
  Trash` ones included, while the `Other` lines stay until a folder open
  clears both, and after a delete the
  deleted files' judgments leave both histories (`Undo::without_paths`). A
  move whose `Summary::run_id` is set pushes its run on the undo history
  (see the next entry).
- Seen on Windows 11 on 2026-11-22: `Move to Trash` confirmed during a fresh
  264-file scan logged `Move to Trash held for the scan's end`, ended the
  scan early (`canceled=true` at 12/264), moved the file, re-listed the
  folder and resumed the scan; the waiting note was too short-lived to
  capture. With the dialog open `q`, `4` and `ctrl+,` did nothing; Tab moved
  the outline between the run button and `Cancel`; Escape and `Cancel` moved
  nothing; the RAW and both sidecars went to the Recycle Bin. Not observed:
  an earlier judgment no longer being undoable after a delete (pending in
  `todo.md`).
  Primary source:
  `../plans/_archived/20261122-windows-device-checks-3/learnings.md`.
- Seen (2026-11-27, Windows 11, through MCP): a `trash_photos` confirmed
  during pass 1 reported `held_for_scan` with `Move to Trash: waiting for the
  scan to finish`, the scan was canceled and the file moved; with another
  folder opened before the scan ended the operation became `failed` with
  `dropped: another folder was opened`, the note cleared and the file stayed.
  `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`.

### A trash run is undone through the same gate, and outlives a folder switch (Inferred)

`history.rs`'s entry is `Entry::Judgments(Batch)` or `Entry::Trash(TrashEntry
{ id, run_id, count, dirs })`, the Tauri `undo.ts` `Entry`. A move that
recorded a run pushes it and clears the redo history (`record_trash`); a
redo's new run is pushed without clearing it (`redone_trash`), as a redone
batch is. `step` and `Undo::forget` see only batches: the view's `step`
looks at the top first (`top_trash`) and hands a run to `step_trash`, as the
Tauri `step` did, so `history::step` returns `None` with a run on top and
leaves it there.

- Rule: an undo or a redo of a run goes through `idle::IdleGate` as the run
  does, held under `Undo Move to Trash` / `Redo Move to Trash` while a
  listing or scan is under way and drained at `finished`, but it never
  cancels the scan (`cancels_scan` is false for those labels). The gate's
  slot is the `Held` enum (`Run`, `Undo`, `Redo`) in `main.rs`.
- Rule: the entry stays on its history until the engine answers
  (`trash_step` names the one out, and every undo or redo key does nothing
  meanwhile); a drained step runs only while its entry is still on top
  (`Undo::on_top`), and entries are removed by id (`remove_trash`), not
  popped, since judgments may have been pushed above it meanwhile. A
  refusal other than `RUN_GONE` / `UNDONE_GONE` (`trash::still_held`)
  leaves it where it was.
- Rule: the restore (`trash::undo_run`, which lists the Recycle Bin on
  Windows) runs on `background_spawn`, reporting through
  `Event::UndoProgress` into `undo_progress`, not `trashing`; the redo
  (`trash::redo_run`, given the view's index so a redo of Delete Sidecars
  clears the judgments of a closed folder's rows) drains the judgment queue
  and flushes the writer first, as a run does, and reports through
  `Event::TrashProgress` and `trashed`.
  Both hold `Resync` and the gate's in-flight count, so nothing lists under
  them, and a tree open meanwhile waits in `deferred_open`.
- Rule: when the open folder is one of the run's `dirs` (`opens_target`),
  the restored files directly in it (`restored_into`, compared through
  `context::shown_path` as the engine spells them canonically) join `all` at
  once and `refilter` runs, so a second undo already reaches their
  judgments, and the folder is re-listed with the current file and the
  selection kept. Otherwise only a change seen meanwhile re-lists.
- Rule: another folder's open, and a sidecar format switch, drop only the
  judgment batches (`Undo::drop_judgments`); the runs stay, so a run made in
  one folder is undone from another, its files going back to theirs. A
  delete records no run and is not undoable.
- Seen on Windows 11 on 2026-11-22: `ctrl+z` restored the RAW, `.xmp` and
  `.dop` from the Recycle Bin (rating kept, current file and selection
  unmoved) and `ctrl+shift+z` moved them again; a second `ctrl+z` after a
  rating undid the judgment; a run made in `undo\a` was undone from `undo\b`,
  the file going back to `undo\a`; two quick `ctrl+z` during a re-listing's
  scan showed `Undo Move to Trash: waiting for the scan to finish`, did not
  cancel the scan and ran once at its end (one `trash: undo of run N` line).
  With the bin emptied by hand the note read `Restored 0 files from the
  Trash, 3 failed`, counting the sidecars unlike the move's RAW count. That
  was then changed: the undo's note counts one failure per RAW (or JPEG)
  group, a sidecar with no media file before it counted as its own item
  (`Restored::failed_items`), and a sidecar whose RAW did not come back
  reports `NOT_IN_TRASH` when it is no longer in the Trash and
  `RAW_NOT_RESTORED` only when it is still there. Primary sources:
  `../plans/_archived/20261122-windows-device-checks-3/learnings.md` for the
  observation and `../plans/20261122-todo-display-text-fixes/learnings.md`
  for the change.
- Seen (2026-11-27, Windows 11, through MCP): a restored 3-star file's cell
  showed its stars; after a cross-folder undo, reopening the run's folder
  listed the file; a `redo` reported `moving to Trash 0 / 1` (operation
  `Redo Move to Trash`, running 0/1), then `Moved 1 file to the Trash`.
  `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`.

### The folder operations share one dialog model and live in `folderops.rs` (Inferred)

The tree menu's folder operations (so far `Rewrite Sidecars from Index…`,
`Delete Sidecars…`, `Sequence JPEG Timestamps…` and `Import…`) open one dialog,
`dialog::Dialog`: its `Content` (title, lines, rows, each with an optional
accent part and a muted flag, headed lists after the rows, notes, errors,
reason, running note), the run
button's label and `Style` (primary or destructive), which buttons are
enabled, and the button the focus goes to (`Dialog::focus`: the run button,
or Cancel when the run is disabled). A run that cannot be canceled disables
both buttons (`set_running`); a cancelable one keeps Cancel enabled and moves
the focus there (`set_running_cancelable`). `Dialog::key` gives Escape as
`Dismiss` (the flow decides: a non-cancelable run ignores it), lets Enter or
Space go on to the focused button while one is enabled, and swallows every
other key. The trash dialog was not moved onto it.

The dialog is a gpui-component `Dialog` in `Root`'s layer, built by
`folderops.rs`'s `folder_dialog` inside `modal.rs`'s shell exactly as the
trash dialog is (see "The trash dialog is a component `Dialog`"): the same
`Modal` plumbing (`folder_modal`), the same `modal::backdrop` drawn while
`folderops.dialog_open()`, and a footer `capture_key_down` asking
`folder_dialog_key` before the focused `Button` presses itself. The title is
the component's, the lines its content, the rows and lists its body, which
scrolls under the component's scrollbar when long, and the notes, errors,
reason and running note sit in the footer above the `primary()` or
`danger()` run `Button` and Cancel, on focus handles the view keeps
(`folder_buttons`).

- Rule: the lines block is `flex_none`. The component's `DialogContent` and
  the wrapper of its scrolled body are both `flex_1` (grow 1, shrink 1, basis
  0 %) siblings in one column, so a dialog that fills both slots gives each
  half of its height: the lines were stretched into a blank band above the
  list and the list clipped its last row with room left. `flex_none` keeps
  the lines at their own height and leaves the body the only flexible
  child. `folderops.rs`'s tests check it through the `debug_selector`s
  `folder-dialog-line`, `folder-dialog-list` and `folder-dialog-footer`
  (and the component's `dialog-0`). `debug_bounds` takes a `&'static str`,
  so the selectors are fixed strings, and the list container's bounds move
  with the scroll offset (its top is the first row, its bottom the last
  row's), so check a long list's end by simulated `ScrollEvent`s. Without
  the fix the list started 134 px (10 rows) to 391 px (300 rows) below the
  last line in the 1600x1000 test window. Seen on Windows 11 (2026-11-22):
  the `Sequence JPEG Timestamps…`, `Import…` and `Rename by Capture Date…`
  dialogs with 10 and 300 rows showed the first row under the lines and the
  last row whole after scrolling (results in
  `../plans/_archived/20261122-folder-dialog-list-layout/learnings.md`).
- Rule: `follow_folder_dialog` runs wherever `folderops.dialog` changes: it
  opens the dialog when one is shown and closes it when none is. The
  import's stepping aside is a close with the flow alive (the focus goes
  back to the app's view), and `Import…` bringing it back a new opening.
- Rule: the builder focuses `Dialog::focus` on its first run of an opening
  and again whenever that changes (a `Cell` per opening), so a cancelable
  run's start moves the focus from the now disabled run button to Cancel
  and a Tab the user pressed is never undone. Tab / Shift+Tab are `Root`'s
  focus trap over the enabled buttons.

`folderops.rs` is a child module of `main.rs` holding `FolderOps` (the flow
under way and the open dialog) and an `impl Handpic` block: a child module
reaches the view's private fields, methods and constants, so the
operations need no entity of their own and `main.rs` gains only the fields
(the flows, the `Modal`, the buttons' focus), the menu arm, the key guard,
the backdrop's condition and the gate's `Held` variant.

- Rule: one flow at a time: a start does nothing while the settings modal is
  open, the trash flow is busy or another folder flow is busy, and the trash
  dialog's start refuses while a folder flow is busy.
- Rule: a sidecar request goes through the trash's `IdleGate` as
  `Held::Sidecars` under `Rewrite Sidecars from Index` / `Delete Sidecars`;
  neither cancels the scan, the pane's waiting note comes from
  `gate.waiting()`, `finished` drains it into the preview, and an open drops
  it. The preview reads on `background_spawn` through the read-only
  connection; the run, refused with `SCAN_RUNNING` when `Resync::busy` or a
  run is in flight, holds the gate's in-flight count and the resync as a
  trash run does.
- Rule: a sidecar delete ends in `trashed` with `trash::What::Sidecars`: its
  run goes on the undo history as a `TrashEntry` whose `what` words the
  statuses (`Moved 2 sidecars to the Trash`, `Restored …`) and the gate
  labels (`Undo Delete Sidecars`); the shown judgments are cleared when the
  open folder was the target (`sidecars_trashed`). A sidecar run's undo
  counts the sidecars that did not fail, since `Restored::restored` lists
  RAWs and JPEGs only (`trash::restored_count`).
- Rule: `Sequence JPEG Timestamps…` neither goes through the gate nor
  touches the scan, the index or the resync, and needs no open folder. The
  preview reads on `background_spawn`; the run is a `std::thread` of its own
  (it can take minutes, so it does not hold a background executor thread)
  with an `AtomicBool` cancel flag, its progress sent into the view's channel
  as `Event::FolderOp(Message::SequenceProgress)` tagged with the run id
  `SequenceFlow::run` hands out before the run starts, so a late message of an
  ended run is dropped; the result comes back through a `oneshot` awaited by
  `cx.spawn`. The quit sets the flag through a `cx.on_app_quit` subscription
  kept only while the run is under way, and waits for nothing.
- Rule: `Import…` has the sequencer's shape (no gate, no scan, no index,
  no open folder needed; a `std::thread`, an `AtomicBool`, the quit's
  subscription), reads the root and the pattern from `Settings` (never
  writing them) and keeps the previewed `Plan` in `FolderOps`, so the run
  copies what the dialog showed. The thread sends the created folders
  through one `oneshot`, then the result through another, both awaited in
  order by one `cx.spawn`: the first folder opens through `open_from_tree`
  (which returns the open's `Task`), and the report waits for that task,
  since a fresh listing clears `write_errors` and the import's result line.
  Progress is sent only when the copied count changes, not per buffer. The
  dialog steps aside (`FolderOps::aside`) once the folders exist, `Import…`
  brings it back, Escape on it steps aside again and Cancel cancels. While
  the flow is busy, `FolderOps::busy` keeps the settings, the trash dialog
  and the other folder flows shut, as `importFlow.busy` did in the Tauri
  app; `import_busy` is the flag the folder rename refuses on.
- Rule: a quit mid-import sets the flag and waits for nothing: the copy
  removes the temp file of the file in flight when it sees the flag, but a
  process that exits first can leave that file under its `.handpic-tmp` name in
  the destination; the files already copied stay, and a second import skips
  them by name and size.
- Seen (2026-11-22, Windows 11): during a scan a Rewrite and a Delete
  request each showed its waiting note with the scan not canceled, and
  another open dropped the waiting Delete with no dialog; with a dialog open
  a click on the tree or the strip, `→`, `3` and `ctrl+,` did nothing
  (results in
  `../plans/_archived/20261122-windows-device-checks-5/learnings.md`).
- Seen (2026-11-22, Windows 11): a Delete Sidecars run said `Moved 24
  sidecars to the Trash`, its undo `Restored 24 sidecars from the Trash`, and
  its redo moved them again (results in
  `../plans/_archived/20261122-windows-device-checks-5/learnings.md`).
- Seen (2026-11-22, Windows 11): a 300-file Sequence run dimmed `Run` and
  focused `Cancel`, and Escape ended it with `canceled, 60 of 300 written`
  and no Explorer window (results in
  `../plans/_archived/20261122-windows-device-checks-5/learnings.md`).
- Seen (2026-11-22, Windows 11): an import opened its first date folder
  during the run, and a reopen with nothing to copy had `Run` dimmed and
  `Cancel` focused (results in
  `../plans/_archived/20261122-windows-device-checks-5/learnings.md`).
- Seen (2026-11-27, Windows 11, keys and clicks posted to the window, the
  state read through the MCP companion and `screenshot`): Tab moved the
  outline from the run button to `Cancel` and back on the Rewrite, Sequence,
  Import and Rename by Capture Date dialogs, and Escape closed each; a
  125-file Rewrite and a date rename dimmed both buttons under their running
  note; a Delete Sidecars run took the strip's stars, flag dots and labels
  and `Ctrl+Z` brought them back; a colliding date rename had `Run`
  disabled and `Cancel` focused; `ctrl+,` did nothing while an import
  preview read, with a Sequence dialog up and during a Sequence run; a
  second `Sequence JPEG Timestamps…` during the preview did nothing; the
  pane showed `reading …`, `sequencing n / m` and `importing n / m`; a
  running import's dialog stepped aside, came back on `Import…` with `Run`
  dimmed and `Cancel` focused, Escape hid it with the run going on, and
  `Cancel` stopped a run with no `.handpic-tmp` file left; the strip of the
  open date folder stayed empty until that folder's copies ended (results
  in
  `../plans/_archived/20261127-windows-device-checks-folder-dialogs/results.md`).
- Seen (2026-11-23, Windows 11, through MCP): a `rewrite_sidecars` and a
  `delete_sidecars` during pass 1 of a fresh index noted `waiting for the
  scan to finish`, and the dialog opened once pass 2 ended (results in
  `../plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`).
- Not verified headless: the strip filling while an import into the open
  folder runs (it did not on 2026-11-27, above). The hand checks are pending in
  `../plans/20261113-gpui-folder-dialogs/learnings.md` and, for the
  component `Dialog`,
  `../plans/_archived/20261114-gpui-component-adoption/learnings.md`.

### A folder rename releases the watches, then follows in the tree and reopens (Measured)

`Rename by Capture Date…` (`datefolder.rs`, run from `folderops.rs`) renames
a folder to its one capture date through the engine's `rename_folder`, or
splits it into one sibling folder per date through `split_folder`, both on
`background_spawn` with the view's `Watch`, `TreeWatch`, write index and
`Runs` in a `rename::Context`. The engine releases the open folder's watcher
(`watch::release_under`, whose `settle` waits for notify's server to close the
handle) and the tree watches on the folder and under it (`with_released`,
holding the `TreeWatch` lock across the move) around the disk work, and puts
the watcher back when the rename fails or the split leaves the folder.

- Measured (`datefolder::watch_release`, a test in this crate so it runs on
  `crates/app/Cargo.lock`'s `notify` 8.2.2, on Windows): with the open
  folder's watcher on a folder and tree watches on it, its parent and its
  subfolder, a plain `std::fs::rename` of the folder fails; `rename_folder`
  renames it, a `watch::set` on the new path then reports a file written
  there within the debounce, and `split_folder` moves every file out and
  removes the folder while it is watched. Five runs in a row passed, each in
  about 0.5 s.
- Measured (a PowerShell check on Windows 11, NTFS): any open handle to a
  file inside a folder refuses the folder's rename, `FILE_SHARE_DELETE` or
  not. The app's reads inside the open folder (the preview, a
  prefetch, the on-demand faces and eyes, the 1:1 crop) open, read and close
  the file, with no memory map, and the scan is refused before the run by
  the scan rule below, so the window is a read in flight at the moment of
  the rename; a rename refused that way reports the OS error in the status
  and leaves the folder, its watcher and its index rows as they were.
- Measured (2026-11-22, Windows 11): a held rename runs in `FacesDone`'s
  `finished`, and `request_prefetch`, which refuses while a scan pass runs,
  came right after it in the same UI turn, so the first prefetch since the
  open started beside the rename (a UI test logged it on every one of five
  held renames). `handpic_core::reader::read_preview` of real ARWs on a
  thread concurrent with `std::fs::rename` of their folder failed the rename
  with `PermissionDenied` 39 times in 40; with no read, 40 in 40 renamed.
  That prefetch was the handle behind the held rename's `Access denied (os
  error 5)` at pass 2's end.
- Rule: `request_prefetch` starts no read while a folder rename is in flight
  (`RenamesInFlight::blocks` of the open folder), so the `FacesDone` that
  drains a held rename starts none; a rename of an unrelated folder holds
  nothing, and a failed rename asks for the prefetch again. A page turn's
  preview read, an on-demand read or a 1:1 crop read in flight at the
  instant of a rename can still refuse it.
- Rule: the date rename is not held by the gate: with a listing, a scan or a
  run under way (`Resync::busy` or the gate's in-flight count) the preview
  and the run are refused with `SCAN_RUNNING` in the status. The run holds
  the in-flight count and the resync as a sidecar run does, so no listing
  or scan starts under it, and drains the judgment stores and the sidecar
  writer first.
- Rule: every folder rename ends in `folderops`' `folder_renamed`, a port of
  `main.ts`'s `folderRenamed`: `Tree::rename_folder` re-keys the node and
  everything under it with their state, the tree's open row and cursor
  follow (`tree::rebase` over the tree's spelling), the trash runs of both
  histories follow (`Undo::map_trash_dirs` over the canonical paths, which is
  how a run's `dirs` and `Handpic::dir` are spelled; the engine's
  `Runs::rename_dir` already moved the recorded runs), and the tree's watched
  set is handed over afresh. When the open folder is the renamed one or under
  it, it is reopened under the new path as a tree click opens it; the
  non-carry listing drops the judgment batches of the undo histories as any
  folder switch does, and the judgments and the last viewed file come back
  from the index rows `Index::rename_dir` carried, so nothing is
  re-extracted. A warning, and a split's status, are set once that open has
  landed, since its fresh listing clears the status.
- Rule: a split re-lists the parent in the tree (`tree_refresh`, which also
  hands the watched set over again, since the split released the tree
  watches under the folder). When the split folder was the open one it is
  re-listed if it remains (its watcher put back by the engine), else the
  first date folder opens; a folder the tree asked for during the run opens
  instead, unless that deferred open (`deferred_open`) is the folder the
  split removed or one under it: `date_folder_ran` takes it out (when
  `split.removed` and `tree::relation` relates it to `dir`) and the first
  date folder opens, as for the open folder, so the missing folder is never
  opened. A file that failed to move goes to the notes as `<name>: could
  not move: <message>`.
- Seen (2026-11-22, Windows 11): during a scan the date rename was refused
  with `SCAN_RUNNING`; a one-date rename showed the row under its new name,
  kept the strip's files and current file, logged `todo=0` in the
  reconcile, and `Ctrl+Z` restored a trashed RAW into the renamed folder; a
  split kept the folder with its undated file, another removed it and
  opened the first date folder; an existing sibling of the date's name
  disabled `Run` (results in
  `../plans/_archived/20261122-windows-device-checks-5/learnings.md`).
- Seen (2026-11-27, Windows 11): a one-date rename run from its dialog
  reopened the folder under its new name with the strip's two stars kept
  (results in
  `../plans/_archived/20261127-windows-device-checks-folder-dialogs/results.md`).

### The tree's inline rename is a text input in the row, held by the gate (Inferred)

`Rename…` in the folder menu, or a slow second click on the open folder's
name, puts a fresh gpui-component `InputState` (drawn with `Input`) in place of the
row's name (`folderops`' `tree_name`), its text the name (or the pending one)
selected whole (`InputState::select_all`) and focused. The edit is keyed by the row's
path, not its index, so a re-listing of the tree keeps it with its text and
selection; `rename.rs` is the pure part (`commit`, `edit_outcome`,
`PendingRename`, `SlowClick`, `RenamesInFlight`).

- Rule: a live edit owns the keyboard. The `Input` takes its editing keys as
  its key context's actions (see "Text is typed through gpui-component's
  `Input`"); every other key goes on unstopped (on Windows a stopped key types nothing) and the window's
  `key_down` returns for it right after the dialogs' guards
  (`rename_owns_keys`), so neither the tree's keyboard nor the keymap sees
  it. A key the IME takes raises no `KeyDownEvent`, so composition never
  reaches a listener.
- Rule: the input's `InputEvent::PressEnter` or `Blur` (Enter, or the
  focus left) confirms and Escape cancels, once (`commit`); the edit and its subscription
  are dropped at once, so the lost focus that follows an Enter decides
  nothing. A confirm (`PressEnter` or `Blur`) while the window is inactive is ignored: GPUI blurs the
  focused element when the window deactivates (the focus path of an inactive
  frame is empty), and the edit waits for the window to come back as the
  Tauri blur checked `document.hasFocus()`. Any press outside the input
  focuses the window's root (`track_focus` on any button) and so confirms.
  After Enter or Escape the root is focused again and the tree has the
  keyboard.
- Inferred: a row scrolled out of the `uniform_list`'s range is not laid
  out, so a live edit scrolled away loses the focus and confirms; the Tauri
  input stayed in the DOM. A confirm on a folder the tree no longer has does
  nothing.
- Rule: a confirmed name goes through the gate as `HeldOp::Rename` under
  `Rename…` (`gate_request`): run at once with no listing, scan or run under
  way, else held with its row showing the new name muted and italic after a
  clock, `PENDING_TITLE` on hover, until it runs, without canceling the scan.
  `gate_request`, which the trash, the undo and the sidecar requests use
  too, discards first, so a held rename replaced by another request has its
  mark cleared and says `Rename… was canceled` (not when a re-edit of the
  same folder replaced it); an open drops it the same way and puts the note
  in the status after its listing (`note_after`). Typing the real name back
  on the pending row discards it; an import under way refuses it, when
  confirmed and again when it drains.
- Rule: the run is the date rename's: in flight with `Resync::hold`, the
  stores and the writer drained, the engine's `rename_folder` with the
  canonical path computed before and after, then `folder_renamed`; a refusal
  (an invalid name, an existing entry) goes to the status. Until it settles
  the folder and everything under it is refused for an open from the tree
  (`open_from_tree`) and for a rename with `A folder is being renamed; try
  again in a moment.`.
- Rule: the slow click (`SLOW_CLICK_DELAY`, 500 ms) is armed by a single
  click on the open folder's name (not a root, no Ctrl / Shift) and fired by
  a `spawn_in` timer; any press in or outside the tree pane, any key and a
  second click (`click_count` 2) disarm it. That click neither reopens the
  folder, as in the Tauri tree, nor toggles the row; the open row toggles
  from its icon, indent, gaps or count, or with a modified click on the
  name.
- The operations' rules during a scan, in one place: `Rename…`, `Rewrite
  Sidecars from Index…` and `Delete Sidecars…` are held by the gate and
  never cancel the scan; `Move to Trash` and `Delete Permanently` are held
  and cancel it; `Rename by Capture Date…` is refused with `SCAN_RUNNING`;
  `Sequence JPEG Timestamps…` and `Import…` ignore the scan.
- Seen on Windows 11 on 2026-11-22: a held rename's row showed `◷ Later`
  muted and italic with `Renames when the scan finishes` on hover, the pane
  said `Rename…: waiting for the scan to finish` and the scan was not
  canceled; typing the real name back and opening another folder each
  canceled it with `Rename… was canceled`. One held rename of two failed
  when it ran as pass 2 ended, with `Access denied (os error 5)`; the other,
  and the failed one run again by hand, succeeded. The cause was the first
  prefetch since the open, started in the same `FacesDone` turn as the held
  rename; no prefetch starts while a folder rename is in flight now (see "A
  folder rename releases the watches, then follows in the tree and
  reopens"). The real-device repeat is pending in `todo.md`.
- Seen on Windows 11 on 2026-11-22: a single click on the open row's name
  opened the field after about 0.5 s; a double-click, a click followed by Up
  and two clicks about 280 ms apart opened none; two clicks about 480 ms
  apart did open it, borderline with the 500 ms system double-click time.
- Not verified in the running app: the IME in the editor and a rename
  during an import run; the hand checks are pending in `todo.md`.
- Seen (2026-11-27, Windows 11, through MCP): while a rename was in flight
  (2148 index rows carried, 796 ms), eight `open_folder` calls on the folder
  1-16 ms after the confirm answered all returned `A folder is being renamed;
  try again in a moment.`; a rename of a folder with no index rows ran in
  10 ms, before any open arrived.
  `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`.

### An import's end posts a system-only notification while inactive (Measured)

- `import::done_notification` decides and words it: nothing while the main
  window is active, else `Import finished` or `Import canceled` over
  `done_status`, the status line's summary, so the two cannot drift.
- `import_ran` posts it after `ImportFlow::done` accepted the run (a stale
  run posts nothing), through the view's stored `AnyWindowHandle` and a
  `cx.defer`, since the run's task has no `Window`: the active check
  (`is_window_active`) and the post happen in the same window update.
- It goes through gpui-component's `push_notification` with `.system()`
  only: no in-app toast, since the status line already shows the result.
  The component's response handler, which `gpui_component::init` installs
  as the one global handler, activates the app and this window on a click;
  do not call `cx.on_system_notification_response` again, it would replace
  that handler. A fixed id (`Notification::id::<ImportEnded>()`) makes a
  later import's notification replace an earlier one still in the center.
- `main` calls `cx.set_app_identity(IDENTIFIER, "Handpic")` before the
  window opens. The window's `app_id` does not provide it: without the call
  the platform logs `cannot show a system notification without an app
  identity` and drops the toast. On Windows it sets the process
  AppUserModelID (the taskbar groups and pins by it) and the first toast
  writes `HKCU\Software\Classes\AppUserModelId\dev.minodisk.handpic` with
  `DisplayName`; no icon is registered, so the toast shows a generic one.
- Platform caveats: Focus Assist hides it on Windows; macOS posts only from
  `Handpic.app` in a trusted location and asks for authorization on the
  first post; Linux needs a notification daemon and cannot retract or
  replace one. Not unit-tested beyond the decision and the text (no
  `gpui::test` harness here); the delivery, the click and the active case
  were checked by hand on Windows (below), the taskbar button's icon and
  grouping were not.
- Seen (2026-11-22, Windows 11): an import that ended while the window was
  inactive posted a toast at the desktop's bottom-right with the header
  `Handpic`, the title `Import finished` and the status line's summary as its
  body, under a generic app icon, not Handpic's (results in
  `../plans/_archived/20261122-windows-device-checks-5/learnings.md`). On
  2026-11-27 a cancel with Notepad in front posted `Import canceled` /
  `Import canceled, 13 files copied`, a finished run `Import finished` /
  `Imported 18 files (676.5 MB)` about 0.9 s after `Run`, a click on that
  toast brought the Handpic window to the front, a run with the window kept
  in front posted nothing for 10 s, and no log had a
  `failed to register AppUserModelID` or `cannot show a system notification`
  line (results in
  `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`).

## Timings

### `keypressToPixels` differs from the Tauri app's (Inferred)

The `page` line uses the Tauri line's `wait` / `read` / `decode` meanings,
collapses `ipc + post + worker` into `hop` (the time from the background result
to the update on the main thread), and splits `draw` into
`frameWait + paint + present`. `keypressToPixels` includes `present`; Tauri's
stopped at `drawImage`. It starts at the latest press of the file that ended up
shown. `source=read|prefetch` says whether the image came from its own read or
the prefetch ring; a hit's `read`, `decode`, `orient` and `hop` are 0, and
its `wait` is the time `go` took before the hit. An HDR PQ CR3's HEVC decode
(`reader::Undecoded::decode`) counts in `decode`, after `read` ends. `px=` is the preview JPEG's
stored size and `decoded=` the size it was decoded at for the fit view, both
unrotated.

### A keyed page waits for the vsync tick on Windows (Measured)

On Windows GPUI (gpui-pre-windows 0.3.12) draws only on its vsync thread's
tick (`DwmFlush`, then `RedrawWindow(RDW_INVALIDATE)`, then `WM_PAINT`), so
a page's `cx.notify()` waits `frameWait` for the tick, and its frame,
presented just after that vblank, is composed at the next one: about one
refresh more than necessary. Forcing the paint with
`RedrawWindow(RDW_UPDATENOW)` was tried (#1219): 47 % (ARW) / 34 % (RAF) of
presses gained a refresh (`screen` p50 33.51 to 25.10 / 60.99 to 56.14 ms),
and it was reverted as outside GPUI's public API; a real fix belongs in GPUI
(e.g. drawing at once when idle). See
[the plan's results](../plans/_archived/20261122-page-present-now/results.md).

- Rule: do not re-add an app-side forced paint; propose it upstream instead.

### `keypressToPixels` does not include the texture upload (Measured)

GPUI uploads a new image's whole texture on its first paint
(`gpui-pre-windows` 0.3.12, `directx_atlas.rs`, `UpdateSubresource`), after
the `present` stamp and without `Present(0, 0)` waiting for it. Before the fit
view decoded at the canvas's DCT scale, a 4416x2944 RAF preview (52 MB)
had a median `screen` p50 47.02 ms above its median `keypressToPixels` p50,
against 14.36 ms for a 1616x1080 ARW (a difference of medians, not a
per-press one). Decoded at about the drawn size (`decoded=1656x1104` in a
1600x1000 window), the RAF difference is 15.09 ms, the ARW level (14.92 ms), and the
RAF `screen` p50 dropped from 112.60 to 64.94 ms (same-session A/B with the
external ruler, three runs each, `--no-prefetch`; see
[the plan's results](../plans/_archived/20261121-fit-preview-downscale/results.md)).

- Rule: never hand GPUI an image much larger than it draws on the page path.
  The upload's cost is invisible to every in-app stage and shows only on an
  external ruler; the about 15 ms between `present` and the DWM present is
  the floor for any image size.

### The meta pane is `div` rows, and its cost lands in `present` (Measured)

The right-hand pane (220 px by default, the Tauri `#info` width) lays out the file
name (and the error line of a failed preview), the `meta.rs` groups and the
notes as `div`s, with no position (that is the status bar's), built from
the `Metadata` that `load` formats on the background thread next to the
preview, so paging reads nothing new. Release, `--auto 20` on the 20-file ARW
compare folder, a fresh `--index` per run, before and after interleaved in one
session with other sessions building on the machine (CPU 12-31 % before each
run, so noisy): `paint` p50 0.70 / 0.82 ms before against 0.90 / 0.76 ms after,
`present` p50 0.19 / 0.22 against 0.44 / 0.41 ms, `keypressToPixels` p50
17.97 / 18.65 against 18.02 / 18.52 ms. `paint` times only the preview
canvas's closure; the pane's `div`s paint after it in the same frame, so their
cost shows as about +0.2 ms in `present`. The `div` rows were kept (the plan's
rule: a canvas only when `paint` grows by more than 1 ms).

- Rule: the view-only flag comes from the whole listing (`Listed.files`), not
  the `--size` / `--max` subset, and is set on every open and re-listing
  before the strip is built.
- Rule: a stage that times one canvas's closure does not see the rest of the
  element tree; look at `present` (and `frameWait` for layout) when adding
  elements beside it.

### A meta pane value that does not fit beside its label goes under it (Measured)

At the pane's 160 px minimum (132 px of content after `pl_3` and the 16 px
`SCROLLBAR_TRACK`) a value column beside the label was narrower than one
word, so `Electronic` broke as `Electroni` / `c` and the file name as
`ILCE-1M2_sony_a1_ii_0` / `3.arw` (Windows 11, 2026-11-22). The fix is
Measured on Windows 11 (96 DPI): at 160 px no value broke inside a word, and
the names broke after `_` (`DSC-HX95_` / `DSC00018.ARW`,
`ILCE-1M2_sony_a1_ii_` / `03.arw`). At 220 px only `AF point` and `Metering`
(`Center-weighted average`) went under their labels on those two files and
every other row stayed on one line, judged acceptable; at about 296 px every
row was one line.

- Rule: each row is a `flex_wrap` row with `gap_x_2` (no vertical gap on a
  wrapped row), the label `flex_none` and the value `flex_grow(1.)`, `min_w_0`
  and `text_right`, its basis left at auto. GPUI's text element measures at
  its whole single-line width when the available width is not definite
  (`elements/text.rs`: no `wrap_width` under min- / max-content), so the
  flex line breaks when label + gap + value is wider than the row, and the
  value, alone on its line, shrinks to the row and wraps at UAX #14 break
  opportunities, right-aligned. A value that fits stays on the label's line.
  Do not give the value `flex_1`: its 0 basis would never break the line.
- Rule: a value that does not fit whole beside its label goes under it at
  every width, not only at 160 px; at 220 px a long lens name or a date with
  its time zone does. Flexbox cannot say "only when the longest word does
  not fit" without measuring the text.
- Rule: the file name heading (`Shown::Read` and `Shown::Failed` alike) is a
  `flex flex_wrap` `div` of `meta::name_segments`, the name cut after each
  `_`, `-` and space, each segment a `min_w_0` text `div` with no gap, so it
  breaks between segments and a segment wider than the pane still wraps
  inside itself. `.` is not a separator, so the extension stays with the
  last segment. No U+200B: how the fonts draw it was not checked on the
  three platforms.
- Not yet seen: macOS and Linux (fonts, UAX #14 breaks).

### The meta pane does not use gpui-component's `DescriptionList` (Inferred)

On 2026-11-15, replacing the pane's `div` rows with gpui-component 0.9.1's
`DescriptionList` (optionally inside `GroupBox`) was looked into from the
component's source and declined: the pane keeps its `div` rows.

- Rule: do not port the pane to `DescriptionList` for less code. Its defaults
  (bordered, 3 columns, a fixed 120 px label width, left-aligned values) would
  all need overriding, and today's look (the label at its content width, the
  value `flex_grow(1.)` and `min_w_0`, right-aligned, going under the label when
  it does not fit beside it and wrapping there, an 11 px muted group heading
  with `gap_0p5`) means building each value as an `AnyElement`, no shorter
  than the `div` rows. In a 220 px pane a fixed label width also squeezes
  the value and can wrap a long label.
- `DescriptionList` has no group heading, and `GroupBox`'s title sits `gap_4`
  (16 px) above its content, much wider than today's gap.
- Its label colour, `description_list_label_foreground` (and, when bordered,
  `tokens.description_list_label`), is derived from Default Dark at theme load
  and not re-derived by `Theme::update`, so `theme.rs`'s `map` would have to
  set it to get `muted_foreground`'s `#a1a1a1`.
- Its label is fixed at `text_sm()` while the pane is 13 px, so the label and
  the value may differ in size. This was not verified.
- It is a rendering change, so "A same-session A/B gates only work on the
  page path" asks for the A/B latency gate; the cost outweighs the gain.

### A same-session A/B gates only work on the page path (Hit)

A same-session A/B needs the machine idle for 30 to 60 minutes, and other
sessions' builds spoil its runs (see the pass 2 section below). The user
decided on 2026-11-13 that it is needed only for work that can move the
pager's latency.

- Rule: a port plan includes the A/B latency gate only when it touches
  rendering (per-frame drawing, overlays, new drawn parts such as the status
  row, the strip's marks, the pane, menus' buttons), the face and eye models,
  the preview (read, decode,
  orient, prefetch, the 1:1 view), the thumbnails (their read, decode and the
  scan that makes them), or background work that runs while paging (scan
  passes, watchers, folder reads on the open). Dialogs, context menus, file
  operations, the settings UI, code moves into the engine and fixes off the
  page path need no A/B. When in doubt, the plan says which side it is on and
  why.

### The strip costs the pager under 1 ms (Measured)

Same-session A/B, the pager with the strip, the index and pass 1 against the
pre-strip pager (`fb7fcdf4`), three runs each, 60 presses, warm index: median
p50 `keypressToPixels` +0.80 ms on ARW and +0.51 ms on RAF. `paint` did not
move; `present`'s p95 rose from about 0.2 to 0.55-0.85 ms on ARW. See
[the plan's results](../plans/_archived/20261110-gpui-strip-index/results.md).

- Rule: compare against a binary built in the same session, alternating; the
  pre-strip pager's ARW p50 was 3.5 ms lower in this session than the pager's
  in the port's Step 3 session, and `frameWait`'s p50 ranges over 6-10 ms across runs of one binary.

### Pass 2 costs paging nothing once it yields and runs small (Measured)

Same-session A/B, paging while pass 2 runs over the folder against paging on a
finished index, three runs each, 60 presses, pass 2 on `faces_threads()` (3 of
24 logical cores) and pausing at its stage boundaries while the preview holds
the scan's pause from its read until its image is painted. The median p50
`keypressToPixels` A - B was -2.10 ms on ARW 1616x1080 and -0.30 ms on RAF
4416x2944 (gate +2.0 / +5.0). At 22 pass 2 threads with only the read paused
it was +9.44 / +88.86 ms: `decode` about doubled and `paint` grew 4-6x.
See [the plan's results](../plans/_archived/20261110-gpui-focus-marks/results.md).

- Rule: check every run's `frameWait` p50 before reading an A/B. One session
  ran at 120-145 ms (presents at about 4 Hz) once the user left the machine
  and the display slept. Hold `SetThreadExecutionState(ES_CONTINUOUS |
  ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED)` from a helper process for the
  session; that changes no power setting.
- Rule: the B runs' `read` and `decode` came out slower than A's in every
  session after the first (ARW `read` about 2.2 against 0.5 ms). A's pass 2
  keeps the cores clocked up and re-reads the paged files through the hot
  list. So A - B can go negative; compare pairs, not one run.
- Rule: an `--auto` quit inside pass 2 cancels it, so the run logs
  `scan faces: ... canceled=true` with the `done` count it reached. A quit
  inside pass 1 logs `scan extract: ... canceled=true` and no `scan faces:`
  line.
- Rule: size `--auto N` below pass 2's length (N presses at `AUTO_INTERVAL`
  ~1 s plus `AUTO_SETTLE`), and use a fresh `--index` each run. A re-used
  index let pass 2 finish on its own (3,066 files took 41 s against
  `--auto 60`), so the run logged `canceled=false` and
  `quit: scan stopped=true in 0ms`, which proves nothing about the stop.
- Rule: to page while pass 2 runs, a 60-press run needs about 13k ARW or 4.5k
  RAF files to stay inside pass 2 (4.6 ms per ARW file, 15-16 ms per RAF file
  at 22 threads). Build the padding with hardlinks (`os.link`): no disk cost,
  and every copy stays warm in the page cache. Name the padding to sort after
  the compare files (`zzNNN_`) so `--size` / `--max 20` page only the 20.
- Rule: to turn `f` on in an `--auto` run without a code change, post
  WM_KEYDOWN / WM_CHAR / WM_KEYUP for `F` to the process's `MainWindowHandle`.
  `AppActivate` + `SendKeys` does not reach the app and may type into whatever
  window has focus, so never use it. Close a run by PID with
  `CloseMainWindow`; never for every `handpic` process, because other
  sessions run their own. `Get-Process handpic` returns every leftover
  instance, so take the one you started by PID (`Select-Object -First 1` only
  when you know it is the only one), or the `IntPtr` argument becomes an
  array. `.ps1` files are blocked by the execution policy here, so pass the
  script with `-Command`.
- Rule: other sessions' `mise run ci`, `cargo` and `handpic` processes
  spoil runs (17 of 25 B attempts and 1 of 11 A attempts were discarded in the
  Step 6 session of
  `docs/plans/_archived/20261110-gpui-focus-marks/learnings.md`). Wait for two
  clean load samples and no build process before each run, and discard a run
  with a load sample over 15 %. This reduces but does not prevent load
  arriving mid-run.
- Use forward slashes in Windows paths in Bash-tool scripts. A doubled
  backslash in a heredoc collapses to one and can escape the next `$`.

### The pane and the judgments cost the pager under 1 ms (Measured)

Same-session A/B, `eb46913b` (the judgments plan's Steps 1-5, plus the 1:1
view, the focus-marks overlay and pass 2 merged meanwhile) against the
pre-plan pager (`de249218`), three runs each, 60 presses, warm index: median
p50 `keypressToPixels` +0.85 ms on ARW and -1.45 ms on RAF, inside the
run-to-run spread (B's ARW runs alone span 18.75-21.96 ms). `paint` did not
move; `present`'s p50 was 0.07-0.10 ms higher in every A run. See
[the plan's results](../plans/_archived/20261110-gpui-judgments/results.md).

- Rule: a warm run must end the binary's last scan pass uncanceled (A's
  `scan faces:` line once pass 2 exists, not only `scan extract:`), so no
  timed run carries pass 2.
- Rule: check that no other session's `cargo` build is running before a
  measurement session; the first ARW session was void because one ran through
  its first half and A1 used up its retries.
- Rule: in a measurement script, write Windows paths with forward slashes.
  `"$I\\$b-$round.sqlite"` made bash turn `\$` into a literal `$`, so every
  run of a round shared one index file (`index$b-1.sqlite`) and the "after"
  runs were warm; check that each cold run's `--index` file name is distinct
  before trusting a table.

### The filter and the sort cost the pager nothing measurable (Measured)

Same-session A/B, `40e3ec57` (the filter-sort plan's Steps 1-4 only) against
`f080850a` (`main` right after that plan's own PR), three runs each, 60
presses, warm index, both on the same `--settings` file (no filter, `name`
order): median p50 `keypressToPixels` -2.63 ms on ARW and +0.46 ms on RAF,
inside the run-to-run spread (A's ARW runs alone span 19.28-22.11 ms).
`paint` and `present` stayed within 0.04 ms: the status row's two buttons,
unlike the pane, add nothing to `present`. See
[the plan's results](../plans/_archived/20261113-gpui-filter-sort/results.md).

- Rule: when B is the commit right after the plan's own PR, A - B is the
  plan's cost alone; prefer that over a pre-plan commit with other plans'
  merges in between, so a delta is not shared out across them.
- Rule: a deliberately quiet machine still spoils runs. 11 of 23 timed
  attempts were discarded for one to three load samples of 16-31 % mid-run with no build
  process running, and one slot passed only on its fourth and last try. Keep
  the 4-try cap per slot; a slot that uses them all voids the folder's session
  rather than relaxing the 15 % gate.
- Rule: a delta the size of the bound can be pure spread. ARW's three pairs
  were all negative (-2.97, -0.40, -2.35 ms) for code that does not run per
  page; RAF's changed sign. Check what the plan runs per page before reading
  a delta as a cost or a gain.

### `Root` costs the pager nothing measurable (Measured)

Same-session A/B, the window wrapped in gpui-component's `Root` (the
adoption plan's Step 5) against its parent commit `2595b1bb`, three runs
each, `--auto 60`, warm index, prefetch on (57 of 60 pages hit the ring in
every run): median p50 `keypressToPixels` -0.60 ms on ARW and +0.16 ms on
RAF, `frameWait` -0.76 / -0.26 ms, `paint` and `present` within 0.05 ms;
`frameWait`'s p50 stayed in 8.8-10.4 ms in every run. See
[the plan's learnings](../plans/_archived/20261114-gpui-component-adoption/learnings.md).

- Rule: the `page` lines are printed to stdout, not to the `--log` file; a
  measurement script reads the process's stdout.
- Rule: `tasklist` and `typeperf` print in the console's code page (cp932 on
  this machine); a Python sampler decodes them with `mbcs`, or its idle check
  silently sees no samples.

### During pass 1 the preview reads what the scan already read (Measured)

Stepping with the arrow through 520 cold ARW files while pass 1 ran (27.7 s),
the preview `read` was 0.96 ms p50 and 1.22 ms p95, against Tauri's 14-16 ms
p50 at `READ_SLOTS` 2. The hot list puts the current file and the strip's range
first, so the next file's head is usually cached before the press. A build
without the pause read the same at p50 / p95; its one 336 ms read came on a
slower disk and is not attributed.

- Rule: copying test files with `cp` leaves them in the OS cache (the second
  pass 1 over a 520-file copy took 0.41 s). For a cold pass 1 copy them with
  `robocopy /J` (unbuffered) and use each copy once. Hardlinks of files an
  earlier run read are cached too: pass 1 over them ends before the debug
  window takes keys, so the placeholder phase and a steered-scan check (`End`
  right after the window opens) cannot be seen (on 2026-11-27 hardlinks of
  1500-file folders not opened that session, driven over MCP from the first
  answer, left pass 1 about 10 s, enough to see both;
  `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`).
  The `zoom` timing lines go to stdout like the `page` lines.
- Rule: to measure the pause itself, press onto files the scan has not read
  (a jump, not the next file); `--auto` only steps to the next file.

### Peaking costs a held key nothing measurable (Measured)

Release, `→` held 5 s as injected key-downs every 33 ms, peaking off and on
alternated three times each in one idle session, fit view at 1275x852 / 1278x852:
median p50 `keypressToPixels` -1.75 ms on ARW (18.55 off, 16.80 on, n of
about 105 per run) and +1.11 ms on RAF (52.70 off, 53.81 on), against the plan's
+2.0 / +5.0 ms gate. ARW was faster with peaking on in every pair, through
`read` and `decode`, not investigated (presumably the busy core keeps the clock
up). On RAF a held key shows only 4-6 pages in 5 s whether peaking is on or off:
a preview load (about 40-45 ms) outlasts the key repeat, so the pager keeps
re-reading the newest file, and the overlay has few pages to compete with. See
[the plan's results](../plans/_archived/20261110-gpui-zoom-peaking/results.md).

- Rule: measure a background job's cost with a held key, not `--auto`, whose
  1003 ms interval lets every job finish before the next press; and check how
  many pages the hold actually showed before reading a p50.

### A prefetch hit leaves only the frame (Measured)

Release at `f5cab4bf`, `--auto 60` on the 20-file compare folders, warm
index, 2026-11-27. Three runs with the prefetch ring alternated with three
under `--no-prefetch` in one session (a busy machine). Median p50 / p95
`keypressToPixels`: ARW 13.64 / 22.22 ms against 28.01 / 36.30 ms, RAF
13.34 / 22.04 ms against 55.39 / 71.95 ms. Every prefetch run hit 60 of 60
presses but one RAF run, 58 of 59 (one press coalesced), whose miss was a
mid-list read on the busy machine, not a wrap: the presses across the first /
last file now come from the ring
(`docs/plans/_archived/20261127-windows-device-checks-c7/results.md`).
Before `neighbours` wrapped at the ends of the list
(`../plans/_archived/20261123-todo-ten-closeouts/plan.md`, Step 4), every
prefetch run hit 57 of 60, the 3 misses being the wraps, which set the p95
(ARW 12.82 / 20.10 ms against 23.40 / 34.05 ms, RAF 14.13 / 56.27 ms against
66.15 / 78.95 ms). In that earlier run `paint` did not move on a hit, and
`present`'s p50 rose about 0.1 ms. Starting the prefetch when the current
result is shown therefore does not slow the current page, so it stays there.
Memory peaks on RAF in the earlier run: about
+110 MB working set and +300 MB private bytes. The cap counts 128 MB of BGRA
(two RAF neighbours are 104 MB). The rest is the prefetch's decode buffers
and the GPU textures that kept entries still hold. See
[the plan's learnings](../plans/_archived/20261114-gpui-prefetch/learnings.md).

- Rule: count hits from the `page` line's `source=prefetch`. Since the wrap
  the presses across the first / last file hit too; a miss is a read on a
  busy machine and sets the p95, so read the hits' latency from the p50.
- Rule: prefetch does not run while a scan pass runs. An A/B of the ring
  needs a warm index whose scan ends before the first press.
