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

# Native: draw the remaining hand-drawn controls with gpui-component

## Purpose

`crates/native` already draws its settings body (`setting::Settings`), its
dialogs, its status row (`StatusBar`), its tooltips and its text fields with
gpui-component 0.9.1. A few controls are still hand-drawn `div`s. This plan
replaces them with the component each one imitates, keeping the look as close
as the component allows, and adds no new UI.

Read [native-app.md](../../agents/native-app.md) first: "gpui-component is
initialized", "The settings modal is its own entity", "The sort and filter
menus are deferred popovers over the window", "The status row is
gpui-component's `StatusBar`" and "The right-click menu is a deferred popover
at the pointer".

The inventory, read from the working tree at `bc40cb3a` and from
`gpui-component-0.9.1/src`:

| Component | State today | Target in this plan |
| --- | --- | --- |
| `Button` | Used: the settings' `small_button`s and close `×`, the trash dialog, the folder dialog | The status row's `Sort ▾` / `Filter ▾` (`main.rs`'s `menu_button`, a bordered `div`) |
| `Kbd` | Not used | The Keyboard Shortcuts page's key chips (`settingsview.rs`'s `shortcuts`) and the right-click menu's shortcut text (`main.rs`'s `context_row`) |
| `Select` | Not used | The label names' language picker, today a wrapping row of 16 `Button`s; the Tauri app has a `<select aria-label="Lightroom language">` next to `Reset` |
| `Switch` | Not used | Auto-advance, today `SettingField::checkbox`; `SettingField::switch` is the Settings' own field type |
| `Input` | Used: the label name fields, the pattern field, the tree's inline rename, the Settings' search | The read-only Destination root box (through `InputGroup`) |
| `InputGroup` | Not used | Destination root with `Choose…`, today a bordered `div` beside a `small_button`; the Tauri app has `<input readonly>` beside a button |
| `Radio` | Used: the three sidecar formats | None |
| `Label` | Drawn by the component itself: `setting/item.rs`, `page.rs` and `group.rs` build the item titles and descriptions with `Label` | None. The app's own texts are plain `div`s; `Label` only adds a 1.25 rem line height, the `foreground` color, a secondary text and highlights, none of which a remaining text needs, and the meta pane's rows were decided against in `20261115-meta-pane-description-list-declined` |
| `Combobox` | Not used | None: no control combines free text with a list |
| `NumberInput` | Not used | None: no setting is a number |

Not targets, by earlier decisions that stand: the sort, filter and right-click
menu rows (`PopupMenu` takes the GPUI focus and dismisses on activate), the
folder tree and its expander glyph (not gpui-component's `Tree`), the empty
state's clickable message (text, not a button).

## Decisions taken with the user

- The plan's defaults are accepted as proposed (2026-11-16): `Kbd::format`'s
  spelling for the shortcut chips and the right-click menu, the chip as
  `[Kbd][×]`, `Select` for the language with `Reset` beside it,
  `SettingField::switch` for auto-advance, `InputGroup` for the Destination
  root, and one PR.
- The empty state's "Or press ctrl+o." sentence keeps the keymap's spelling
  (the plan's default).
- This plan goes before `20261115-native-themes` Step 1; the order against
  `20261116-native-gpui-kit` does not matter.

## Steps

- [x] Step 1: Draw the Sort / Filter buttons, the shortcut keys, the language picker, auto-advance and the Destination root from gpui-component
  - Done when:
    - `Sort ▾` and `Filter ▾` are gpui-component `Button`s; the toggle on
      press, the lit filter button, the dimmed inert sort button in a
      view-only folder and the popover's placement behave as before
    - The Keyboard Shortcuts page's keys and the right-click menu's shortcuts
      are drawn by `Kbd`, built from the keymap's key names by a tested
      helper
    - The label names' language is chosen in a `Select` next to `Reset`;
      `Reset` fills in the chosen language's names as before
    - Auto-advance is `SettingField::switch`
    - The Destination root is an `InputGroup` holding a read-only `Input` and
      the `Choose…` button
    - In the settings, Escape still closes them (an open language menu
      first), Tab and Shift+Tab still walk the focus trap, a name or pattern
      field still commits on Enter, blur and Escape, and a shortcut capture
      still takes the next key
    - No hand-drawn copy of a replaced control is left, and no constant or
      helper is left unused by this change
    - `docs/agents/native-app.md` records the components now drawn, the key
      contexts that became live (`Select`, `List`) in the key-binding table,
      and the changed rules of the sections named under Purpose; `CLAUDE.md`'s
      `crates/native` description names the changed controls where it names
      controls
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
    - The hand checks that `--auto` cannot drive (listed below) are done on
      Windows or recorded as pending in `learnings.md`
  - Implementation approach (as far as it is known; omit if unknown):
    - **General.** `crates/app` is frozen; touch nothing there. No new
      dependency: every component is in `gpui-component` `=0.9.1`. New code
      reads `cx.theme()` rather than adding `u32` constants. Tests stay pure
      (the crate has no GPUI test support). If the gpui-kit plan has merged,
      the import paths are `gpui_kit::component::..`.
    - **Sort / Filter (`main.rs`, `menu_button`).** `Button` implements
      `InteractiveElement`, so the caller's
      `.on_mouse_down(MouseButton::Left, ..)` stays the toggle (not
      `on_click`; see the `on_mouse_down_out` rule in native-app.md).
      `Button`'s own mouse-down only calls `window.prevent_default()`, so the
      window's one focus handle keeps the focus. Give it `.tab_stop(false)`:
      the main view routes every key through its own handle. The default
      variant already paints the mapped `button` fill, `input` border and
      `button_hover`, the colors the `div` paints today. Keep the lit look by
      overriding the border and the label's color to `primary`, without
      `.selected(true)`, which would change the fill. Keep the text
      `Sort ▾` / `Filter ▾` as a 12 px child, not `dropdown_caret`, with
      6 px side padding and `rounded_sm`, as `small_button` does for 13 px.
      Decide between `.disabled(true)` and today's `opacity(0.5)` with no
      handler for the view-only sort button by which looks closer. The
      `relative()` wrapper and the `deferred` / `anchored` popover stay.
      Measure the preview canvas (1160 x 842 at the default window, per
      native-app.md): `.xsmall()` is 20 px tall where the `div` is about
      21 px, so either pin the height to keep 842 or record the new size.
    - **Kbd.** `Kbd::new` takes a `gpui::Keystroke`. Add
      `keys::keystroke_of(name: &str) -> Keystroke` in `keys.rs`, the inverse
      of `key_name`: strip the `ctrl+` / `alt+` / `shift+` / `meta+` prefixes
      in that order (do not split on `+`, a key can be `+`), map
      `arrowleft` / `arrowright` / `arrowup` / `arrowdown` back to GPUI's
      `left` / `right` / `up` / `down`, `meta` to `modifiers.platform`,
      `key_char: None`. Test it as a round trip with `key_name` over every
      default key of `riffle_backend::shortcuts`.
      - Shortcuts page: each key is `Kbd::new(keys::keystroke_of(key))` with
        its remove `×` `small_button` beside it, in one small flex group.
        `Kbd` is not a `ParentElement`, so the `×` can no longer sit inside
        the chip. `Kbd` paints `muted` (`#262626`, the chip's fill today),
        `muted_foreground` text at `text_xs` and half the theme radius; it is
        `Styled`, so the text color and size can be set back to the chip's if
        the muted 12 px reads worse. `settingsmodal::display_key` stays for
        `empty.rs`.
      - Right-click menu: `context::Item::shortcut` carries the keymap's key
        name (drop `context.rs`'s private `display_key`, update its tests),
        and `context_row` draws
        `Kbd::new(keys::keystroke_of(..)).appearance(false)` inside the
        existing muted `text_xs` slot, as gpui-component's `Tooltip` draws
        its key binding. An item without a shortcut draws nothing there.
      - `Kbd::format` decides the spelling (see Trade-offs).
    - **Select (`settingsview.rs`, `sidecar`).** Replace the full-width
      custom item with a titled item, `Lightroom language` (the Tauri
      `aria-label`), whose `SettingField::render` holds the `Select` and the
      `Reset` button; keep the `Reset` keyword. The view holds an
      `Entity<SelectState<SearchableVec<SharedString>>>` built in `new` from
      `i18n::presets()` names with `Some(IndexPath::new(0))`, and subscribes
      to `SelectEvent::Confirm` to set `self.language`. Not searchable, not
      cleanable, `.small()`, a fixed width.
      `SettingField::dropdown` is not `Select`: `setting/fields/dropdown.rs`
      draws a `Button` with a `DropdownMenu` (`PopupMenu`), and a field
      cannot also hold `Reset`.
      Read from `gpui-base` `select.rs` and gpui-component `select.rs` /
      `list/list.rs` (Inferred, to hand-check): the `Select` context binds
      `space` (not inside an `Input`), `enter`, `up`, `down`, `escape`, and
      the open menu's `List` context `escape`, `enter`, `up`, `down`. With
      the menu closed the `Cancel` handler propagates, so Escape reaches the
      box's `key_down` and closes the settings. With it open, the list's
      handler propagates to `BaseSelect`'s, which stops and closes the menu
      only. Actions run before the box's listener, so `stops` needs no
      change. Opening the menu moves the focus to its list, so a focused name
      field blurs and commits by itself; check whether the `Reset` and
      `Select` presses still need `leave_field`. The popup is `deferred`
      inside the view, under the box's key listener and focus trap; confirm
      Tab does not leave the trap while it is open.
    - **Switch (`culling`).** `SettingField::checkbox` becomes
      `SettingField::switch`; the closures are unchanged. It paints
      `tokens.primary` when on, `tokens.switch` when off and
      `tokens.switch_thumb`; `theme.rs` maps neither `switch` field, so they
      are `Default Dark`'s. Look at it on screen and map them in `theme.rs`
      (with the theme test) only if the off state or the thumb is hard to
      read. `Checkbox` stays imported only if something else still uses it.
    - **InputGroup (`import`).** The view gets a second `InputState` for the
      root, filled by `show` on `open` and after `choose_root`. The field is
      `InputGroup::new(..).readonly(true).input(Input::new(&root_field))`
      with one `InputGroupAddon` aligned `InlineEnd` holding
      `InputGroupButton::new("import-root-choose").label("Choose…")`, whose
      `on_click` is today's (`leave_field`, then `choose_root`). The group
      draws the frame (`input` border, `input` at 30 % fill in dark, the
      theme radius, `ring` and a focus ring while focused) and renders the
      `Input` without its own. It is `Styled`, so bring it toward `field()`'s
      look (transparent fill, `rounded_sm`, the 13 px text) as far as its
      render order allows; the focused border is set after the caller's
      style and stays the component's. A press on the button does not focus
      the input (`InputGroup` checks `window.default_prevented()`). The root
      field is not in `editing()`'s commit list: it has nothing to commit,
      and Escape in it closes the settings through the `Input`'s propagating
      `Escape`. Keep the item's 360 px width and its `root_note` description.
    - **Docs.** `native-app.md`: the drawn-components list and the
      key-binding table in "gpui-component is initialized" (`Select` and
      `List` leave the "inert until drawn" row, with their keys); the tab
      stops and the look rules in "The settings modal is its own entity" (one
      `Select` in place of the language buttons, the switch, the root field
      as a new stop, the chips); the button rule in "The sort and filter
      menus" and the `StatusBar` section's measurement; the shortcut spelling
      rule in "The right-click menu". `CLAUDE.md`: the `src/settingsview.rs`
      clause's control list, the `src/keys.rs` clause (`keystroke_of`), and
      the `src/context.rs` / `src/menu.rs` clauses only if their wording
      names what changed.
    - **Hand checks (Windows, `mise run native:dev`).** Sort / Filter: press
      toggles, press on the open menu's own button closes without reopening,
      the lit and dimmed states, the popover's corner. Settings: Tab cycle
      through every page, Space / Enter on the switch, the language menu by
      pointer and by keys, Escape with the menu open and closed, `Reset`
      after choosing a language, `Choose…`, selecting and copying the root
      path, a capture after pressing `+`, removing a key. Right-click menu:
      the shortcuts' spelling and alignment.

## Trade-offs and risks

- **Key spelling changes with `Kbd`.** `Kbd::format` upper-cases a single
  character, capitalizes names and spells modifiers per platform: `ctrl+alt+1`
  becomes `Ctrl+Alt+1` (`⌃⌥1` on macOS), `arrowleft` `Left` (`←`), `shift+x`
  `Shift+X`, `x` `X`, `escape` `Esc`, `meta+o` `⌘O` on macOS. There is no
  option to keep the keymap's lowercase names; keeping them means not using
  `Kbd`. The empty state's sentence ("Or press ctrl+o.") and the status line's
  refusals are text, not controls, and keep the keymap's spelling in this
  plan, so the two spellings will differ.
- **The key chip's shape.** `Kbd` cannot hold the `×`, so the chip becomes
  `[Kbd][×]`.
- **Language picker.** `Select` makes one tab stop of sixteen, shows only the
  chosen language until opened, and makes the `Select` and `List` bindings live
  inside the settings. The native-themes plan's Step 3 may also choose
  `Select`; whichever lands first writes the key-binding rows.
- **Auto-advance as a switch** is a deliberate look change (the Tauri app and
  today's page show a checkbox).
- **Destination root as an `InputGroup`.** The path becomes a focusable,
  selectable, copyable read-only field (as the Tauri `<input readonly>`) and a
  new tab stop; `Choose…` moves inside the frame as a ghost button; a long
  path scrolls in one line instead of wrapping; the focused frame shows the
  component's `ring`, not the name fields' `primary` border.
- **Status row height.** A `Button` may change the row's height by about a
  pixel and with it the preview canvas; the step pins or records it.
- **Conflicts with in-flight plans.**
  - `20261116-native-gpui-kit` rewrites every `use gpui::` /
    `use gpui_component::` line to `gpui_kit::` paths. This plan adds imports
    in `settingsview.rs` (`kbd::Kbd`, `select::..`, `IndexPath`, `input::..`),
    `main.rs` (`kbd::Kbd`) and `keys.rs`. Whichever merges second rebases its
    import lines; mechanical, no design conflict. `Cargo.toml` and
    `Cargo.lock` are not touched here.
  - `20261115-native-themes` Step 1 replaces the `*_COLOR` constant reads in
    `main.rs` and `settingsview.rs` with theme reads. This plan deletes
    several of those sites: `menu_button` (`INPUT_BORDER`, `BUTTON_FILL`,
    `PRIMARY_COLOR`, `FOREGROUND_COLOR`, `ACCENT_COLOR`), the chip's
    `ACCENT_COLOR`, the root box's `INPUT_BORDER` / `MUTED_COLOR`, the chosen
    language's `PRIMARY_COLOR` border. If this plan has to map `switch` /
    `switch_thumb` in `theme.rs`, that also conflicts with Step 1's rewrite of
    `map`. Its Step 2 adds an `Appearance` tab to `settingsview.rs` (other
    regions; the docs sections overlap), and its Step 3 may use `Select`.

## Progress

- (2026-11-16) Step 1 complete (the hand checks recorded in learnings.md are still pending)
