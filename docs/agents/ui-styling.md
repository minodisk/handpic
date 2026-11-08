# Styling the frontend (`crates/app/ui`)

Read this before touching `crates/app/ui/style.css`, `index.html` or a `.ts`
file that creates a control or draws on the canvas. It states the one rule
the frontend's look follows and the names to reuse.

The tags follow [`tauri-app.md`](./tauri-app.md): **Hit** broke something
here, **Measured** steered a design decision, **Inferred** comes from sources
or docs only.

Source: `docs/plans/_archived/20260929-ui-design-tokens/` (`plan.md`,
`learnings.md`, and `audit.md`, the part-by-part table of every view and its
shadcn counterpart).

## The rule

UI code uses the `:root` tokens and the shared component classes in
`style.css`, never an ad-hoc color, radius or border. The palette is
shadcn/ui's **Neutral dark** theme; the classes are plain CSS in shadcn's
variants (no Tailwind, no React, nothing added to `package.json`).

- The component classes, in the `/* Components */` block right after
  `:root`, are the only rules that set a control's colors, border and
  radius. An id rule may set size, placement, spacing and layout, and may
  read a token for text (a note in `--muted-foreground`, an error in
  `--destructive`).
- A control built in TypeScript sets `className` to the same classes
  (`remove.className = "button ghost"`, `row.className = "folder
  sidebar-item"`, `count.className = "count badge"`).
- Canvas drawing cannot use `var()`: read the token once at startup with
  `token()` from `src/theme.ts` (`main.ts`'s `THEME`), with a CSS keyword
  fallback (`black`, `white`, `gray`), never per frame and never a new hex.
  The fallback matters: a canvas ignores an empty color string and keeps the
  previous fillStyle / strokeStyle, so a misspelled token would silently
  draw in the last color instead of transparent (Hit).
- A component rule that sets `display` must stay above
  `.dialog-box [hidden] { display: none }`, which is kept last in the block,
  and an id rule that sets `display` on something that can be hidden is
  scoped `:not([hidden])` (see [`tauri-app.md`](./tauri-app.md)).
- `.menu-item` sets `display: flex`, which beats the UA's `[hidden] { display: none }`
  (Hit). Do not put `hidden` on the item itself. Wrap it in a `<div hidden>`
  that the code toggles, as the sort menu's `Recommended` item and the filter
  menu's `Genre` section do.
  Source: `docs/plans/_archived/20261012-image-embeddings/learnings.md`, Step 7.

## Tokens

Converted from shadcn's oklch to hex / `rgb(... / ...)` for the `safari13`
build target (the grays are exact, `--destructive` rounds). Source: the
`.dark` block of <https://ui.shadcn.com/docs/theming>, Neutral, fetched
2026-09-29.

| Token | shadcn (oklch) | Riffle | Paints |
| --- | --- | --- | --- |
| `--background` | `oklch(0.145 0 0)` | `#0a0a0a` | the window |
| `--foreground` | `oklch(0.985 0 0)` | `#fafafa` | text on the window |
| `--card`, `--popover`, `--sidebar` | `oklch(0.205 0 0)` | `#171717` | dialog boxes, menus, the three side panes |
| `--card-foreground`, `--popover-foreground`, `--sidebar-foreground` | `oklch(0.985 0 0)` | `#fafafa` | text on those |
| `--primary` | `oklch(0.922 0 0)` | `#e5e5e5` | the primary button, the focused strip cell's border, the lit filter toggle |
| `--primary-foreground` | `oklch(0.205 0 0)` | `#171717` | text on the primary button |
| `--secondary`, `--muted`, `--accent`, `--sidebar-accent` | `oklch(0.269 0 0)` | `#262626` | badges; quiet surfaces (chips, code blocks, lists, placeholders); hovered / selected controls and rows |
| `--secondary-foreground`, `--accent-foreground` | `oklch(0.985 0 0)` | `#fafafa` | text on those |
| `--muted-foreground` | `oklch(0.708 0 0)` | `#a1a1a1` | secondary text: notes, labels, hints, headings |
| `--destructive` | `oklch(0.704 0.191 22.216)` | `#ff6467` | the destructive button, error text |
| `--border`, `--sidebar-border` | `oklch(1 0 0 / 10%)` | `rgb(255 255 255 / 10%)` | hairlines, separators, scrollbar thumbs |
| `--input` | `oklch(1 0 0 / 15%)` | `rgb(255 255 255 / 15%)` | input, select and outline button borders |
| `--ring`, `--sidebar-ring` | `oklch(0.556 0 0)` | `#737373` | focus rings, a selected strip cell's border |
| `--radius` | `0.625rem` | `0.625rem` | the base radius |
| `--radius-sm` / `-md` / `-lg` | `* 0.6` / `* 0.8` / `var(--radius)` | same `calc()` | chips, menu items, rows / buttons, inputs, menus / dialog boxes |

The performance overlay's chart palette is Riffle's own, not shadcn's
(Tailwind's 400 shades, chosen by the user): `--chart-cpu` (`#60a5fa`, blue,
both CPU lines), `--chart-disk` (`#4ade80`, green, both disk lines) and
`--chart-latency` (`#f87171`, red, the preview latency bars), read into
`main.ts`'s `THEME` as `chartCpu`, `chartDisk` and `chartLatency`. A pair of
lines in one color differs by weight and dash (Riffle thick solid, the system
thin dashed).

`--muted`, `--accent`, `--secondary` and `--sidebar-accent` are the same
`#262626` in Neutral, so hover and selection look alike; tell states apart
by weight, border or a check mark (below), not by a new color.

## Component classes

| Class | Goes on | Used by |
| --- | --- | --- |
| `button` + `primary` | `<button>` | only the sequence dialog's `Run` |
| `button` + `destructive` | `<button>` | `Move to Trash`, `Delete Permanently` |
| `button` + `outline` | `<button>` | `Cancel`, `Reset`, `Reset all`, `Clear Cache`, `Copy`, `#label-names-reset`, the three format choices and the first-launch dialog's `Continue`, the strip bar's filter / sort toggles |
| `button` + `ghost` | `<button>` | icon-only buttons: `#settings-close`, the shortcut chip `×`, the `+` key adder, the meta status dismiss `×`, the filter menu's `?` |
| `input` | text `<input>` | the `#label-name-*` fields |
| `select` | `<select>` | `#label-names-language` and `#format-language-select` (native arrow; its `option`s on `--popover`) |
| `checkbox` | native checkbox / radio | the settings checkboxes and the sidecar format radios (`accent-color`) |
| `field` | a `<label>` wrapping its control | the settings labels |
| `dialog` | the fixed backdrop | `#format-dialog`, `#trash-dialog`, `#sequence-dialog`, `#settings-dialog` |
| `dialog-box`, `dialog-title`, `dialog-actions` | the box, its title, its right-aligned button row | the same four; width / height stay on the ids |
| `menu` | the menu container | `#filter-menu`, `#sort-menu`, `#context-menu` |
| `menu-item` | each `role="menuitem*"` button | static items in `index.html`, `showMenu`'s and the `#filter-exif` builder's items in `main.ts` |
| `menu-separator`, `menu-label` | the `<hr>`, a section heading | the menus |
| `menu-label-row`, `menu-help-toggle`, `menu-help` | a heading with a control at its right, its small `?` button, the definitions it shows | the filter menu's `Face focus` section |
| `scrollarea` | any scrolling element | the menus, `#strip`, `#folders`, `#meta`, the settings panels |
| `sidebar-item` | a side pane row | `.folder` (`folders.ts`), with `current` / `selected` |
| `badge` | a small count | `.folder .count` (`folders.ts`) |
| `empty` | an empty-state hint | `#empty` |
| `sidenav`, `sidenav-item` | a vertical nav and its `role="tab"` items | `#settings-tabs` and its buttons |
| `icon` | a 16px `<span>` holding an `icons.ts` SVG | the settings nav icons (`settings.ts`) |
| `progress` | a determinate progress track (`--muted`, fully rounded); its height stays on the id | `#scan-progress` |
| `progress-indicator` | the track's fill (`--primary`); the script sets its `width` | `#scan-progress`'s child (`setScanProgress` in `main.ts`) |

`secondary` is not defined; add it only if a control needs it. The focus
ring is always `--ring` (`--sidebar-ring` in a side pane).

## Selection

- **Side pane rows and the settings nav:** the accent surface plus weight.
  The open folder (`.current`) and the selected nav item
  (`[aria-selected="true"]`) are `600` on `--sidebar-accent` / `--accent`; a
  multi-selected folder (`.selected`) shares the surface at normal weight.
- **Menus:** a disabled item is greyed with `.button:disabled`'s `opacity`
  and takes no pointer events (`MenuItem.disabled`, for `showMenu`'s items).
  A checked item (`aria-checked="true"`) shows Lucide's `check`
  in its leading column, no fill. `addMenuCheck` in `main.ts` prepends the
  `.menu-check` span; CSS alone shows it, so toggling `aria-checked` is
  enough.
- **Strip cells:** `.cell.current` has a `--primary` border on `--accent`,
  `.cell.selected` a `--ring` border on `--muted` (the same fill, so the
  border is the difference).
- **Dimmed elements on a hover fill:** dim with `--muted-foreground` plus
  `opacity`, never with `--muted`. `--muted` is the hover fill `#262626`, so
  the element vanishes on a hovered row (the unlit filter stars are
  `--muted-foreground` at `opacity: 0.4`).

## Layout

Source: `docs/plans/_archived/20260930-no-margin-layout/` (`plan.md`, `audit.md`, the
margin-by-margin table and what replaced each).

- Layout is flex or grid. Siblings are spaced with `gap` on the parent;
  spacing inside a component is `padding`. `margin` and `float` are not used
  for layout, in `style.css` or through `.style` in TypeScript.
- The `/* Reset */` block before the component block zeroes the UA margins
  of `body`, `h1`, `h2`, `h3`, `p`, `dl`, `dd`, `ul`, `ol`, `hr`, `pre`,
  `figure`, `table`, `input`, `button` and `select`, so no spacing is
  implicit. A dialog or settings panel is a flex column with a `gap`; a
  `hidden` or `:empty` child is `display: none` and adds no gap, which is
  why one `gap` reproduces the old per-element margins.
- A right-aligned item is pushed by `justify-content: space-between` on the
  parent (`#strip-bar`), a `flex: 1` spacer, or a grid column (`.folder`'s
  `14px minmax(0, 1fr) max-content`, whose name column keeps the name span
  hugging its text so the click target does not grow), never
  `margin-left: auto`.
- A wrapped row of inline items is a flex wrapper with a two-value `gap`
  (the shortcut row's `div.keys` inside its `td`: a `display: flex` `td`
  loses its table-cell box, so the flex box goes inside the cell; Inferred).
- The one exception is `.menu-separator`'s `margin: 4px -4px`: the negative
  inline margin bleeds the `hr` across `.menu`'s 4px padding so the line
  runs edge to edge, which `gap` and `padding` cannot express.

## Icons

Icons are Lucide SVG strings in `src/icons.ts` (`SCAN_FACE_SVG`,
`CHECK_SVG`, the settings nav icons), inlined with `innerHTML`, stroked in
`currentColor` and `aria-hidden`. Add a new one there, verbatim from the
`lucide-static` version the file's `/*! ... */` license header names, and
add its name to that header (and to the Feather note if `LICENSE-lucide`
lists it as Feather-derived). The one derived icon is
`SCAN_FACE_FROWN_SVG`, the Face focus Low icon: `scan-face` with its mouth
path mirrored to turn down, which the header notes. The full notice is
`crates/app/ui/LICENSE-lucide`.

## CSS features (Inferred)

The build target is `safari13` on macOS / Linux and `chrome105` on Windows
(`vite.config.ts`); Lightning CSS runs the CSS against it, and the app runs in
WebView2 (Windows), WKWebView (macOS) and WebKitGTK (Linux).

- In use: custom properties and `calc()` in them; `rgb()` space syntax with a
  slash alpha (emitted as 8-digit hex) and `inset`; `:not([hidden])`; flex
  `gap` (Safari 14.1, that is macOS 11 or 10.15 with the Safari 14.1 update,
  and WebKitGTK 2.32, the app's real WebKit floor; grid `gap` is older and
  safe everywhere);
  `:focus-visible` and `accent-color` (Chromium 86+ / 93+, Safari 15.4+,
  WebKitGTK 2.36+; an older WebKit shows its own focus ring and native
  control); `scrollbar-width` / `scrollbar-color` (ignored where
  unsupported).
- `safari13` is a syntax target: Lightning CSS neither down-levels nor warns
  on a layout feature such as flex `gap`, so the target does not mean
  Safari 13 works.
- Avoid: `oklch()` and `color-mix()` (not down-levelled with full fidelity
  at `safari13`), CSS nesting, `:has()`, `@layer`, anchor positioning and
  `appearance: base-select` (Chromium-only). A hover shade is a second
  solid hex, not `color-mix()`.
- `grid-template-columns: subgrid` (Chromium 117+, Safari 16+) is beyond
  both build targets and the Safari 14.1 / WebKitGTK 2.32 floor above. Its
  only use is `#label-names > label` in `style.css`, and where it is
  unsupported the row's columns do not line up with the parent grid. Do not
  add another; when touching that rule, check it on macOS / Linux or replace
  it with explicit columns (Inferred).

## Exceptions

These colors stay outside the tokens on purpose (app semantics, not
chrome):

- `:root`'s `--stars-color`, `--pick-color`, `--reject-color` and the
  `--label-*` set (ratings, pick / reject, color labels).
- The two hover-at-90% hexes in the component block: `#d0d0d0`
  (`.button.primary`) and `#e85c5f` (`.button.destructive`).
- The component block's translucent fills and black overlays, which have no
  shadcn token: the `rgb(255 255 255 / 4.5%)` fill of `.button.outline`,
  `.input` and `.select` (shadcn's `input/30` over the dark background), the
  `.dialog` backdrop's `rgb(0 0 0 / 60%)`, and the `rgb(0 0 0 / 50%)` shadows
  of `.dialog-box` and `.menu`.
- The strip cell's per-cell semantics: `.cell.failed`'s `#402020`, the burst
  band tint (`.cell.burst::before`'s `rgba(255, 255, 255, 0.08)`), and the
  `#000` `text-shadow` / `drop-shadow` halos.
- In `src/*.ts`: `focus.ts`'s `FOCUS_MARK_COLORS` (and `focus.test.ts`),
  `main.ts`'s `FACE_MARK_COLOR` (`#3ff`, YuNet's boxes, dots and labels),
  `FACE_MESH_COLOR` (`#f3f`, the face mesh) and the `rgba(0, 0, 0, 0.8)`
  mark halo.

## Before opening a PR

Run both greps and check every match is `:root` or listed above:

```sh
grep -nE '#[0-9a-f]{3,6}\b|rgba?\(' crates/app/ui/style.css
grep -nE '#[0-9a-f]{3,6}\b|rgba?\(' crates/app/ui/src/*.ts
```

A new match is a new ad-hoc color: use a token or a component class
instead, or, if it is truly app-semantic, add it to the exceptions here.

Then run the layout greps:

```sh
grep -nE 'margin|float' crates/app/ui/style.css
grep -nE '\.style\.(margin|float)|margin|float' crates/app/ui/src/*.ts crates/app/ui/index.html
```

The first matches only the `/* Reset */` block (its comment and
`margin: 0`) and `.menu-separator`'s `margin: 4px -4px`; the second matches
nothing. A new match is a margin used for layout: move the spacing to `gap`
on the parent or `padding`, as in "Layout" above.
