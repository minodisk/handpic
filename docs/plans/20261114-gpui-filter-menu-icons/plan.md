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

# Filter menu icons and headings by kind in the GPUI app

## Purpose

In the native GPUI app (`crates/native`) every swatch of the strip's
`Filter ▾` menu is a small filled circle: the flag dots, the color label
dots and the `Face focus` dots look alike and are hard to tell apart, the
orientation rows have no swatch at all, and only some sections say what
they are. This plan gives each kind its own shape, with icons from Lucide
(lucide.dev, ISC), and a heading to every section:

- Picked / Untagged / Rejected: unchanged (the dots).
- Rating: Lucide star icons, the rows ordered from 5 stars at the top down
  to 0.
- Color label: a square filled with the label's color.
- Face focus: the icons the Tauri app uses for these items.
- Portrait / Landscape: an icon each.
- Every section starts with a non-clickable heading.

### What the investigation found (2026-11-14, `origin/main` at `a8a43d5a`)

- `crates/native/src/menu.rs` `filter_rows` builds the rows; `main.rs`
  `menu_row` (near line 4470) maps each `Row` to `div`s. Today:
  - `Lead::Flag`: a 7 px `rounded_full` dot (`PICK_COLOR`, `REJECT_COLOR`,
    `MUTED_COLOR` for `Untagged`).
  - `Lead::Label(Some(tint))`: the same dot in `tint_color(tint)`;
    `Lead::Label(None)` (`No label`) a hollow dot with a `BORDER_COLOR`
    border.
  - `Lead::Face(state)`: a 12 px slot holding a 9 px dot in
    `MarkState::color()` (`Good` `0x33ff33`, `Low` `0x999999`), empty for
    `Neutral`. It is not the Tauri icon.
  - Orientation rows: `Lead::None`.
  - `Label::Stars(on)`: text, `U+2605` repeated `on` times in `STARS_COLOR`
    then `5 - on` times in `MUTED_COLOR` at 40 % alpha.
  - `Row::Heading(text)`: a muted 11 px line indented by `MENU_INDENT`,
    with no id, no hover and no click handler.
- The star rows are pushed by `for stars in 0..=5`, so 0 stars (unrated) is
  at the top and 5 at the bottom, as in the Tauri `index.html`.
- **Headings.** The native menu and the Tauri `index.html` `#filter-menu`
  have headings on the same sections:
  - With a heading in both: `Face focus` (Tauri adds the `?` help, which
    the native menu leaves out), `Eyes`, `Output` (shown only with an
    export answer) and the per-folder EXIF groups (`Camera`, `Lens`,
    `Aperture`, `Shutter speed`, `ISO`, `Focal length`, from
    `exif::Group::heading`).
  - Without a heading in either: the flags, the rating, the color labels
    and the orientation. They follow a separator only.
  - `Reset` is an action above the first separator, not a category.
  - Tauri also has a `Genre` section (heading `Genre`, shown while AI
    analysis is on); the native menu has not ported it (port Step 9).
- **The native app has no icon mechanism**: no `svg()` call, no
  `AssetSource`, no vendored SVG; `application().run(..)` (`main.rs` near
  line 5050) is called without `with_assets`. GPUI's `svg().path(p)` loads
  `p` through the application's one `AssetSource`
  (`fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>>` and
  `fn list(..)`) and paints the rendered alpha mask in the element's
  `text_color`, so a stroked Lucide SVG and one with a fill both work as a
  one-color icon.
- The Tauri app's `Face focus` icons (`crates/app/ui/src/focus.ts`
  `FOCUS_MARK_ICONS`, set in `main.ts` on `.face`, 12 px, in the focus
  mark's color): `Good` is Lucide `scan-face` (`SCAN_FACE_SVG`), `Low` is
  `SCAN_FACE_FROWN_SVG`, `scan-face` with its mouth path mirrored (a
  derivative, not a Lucide icon), `Neutral` has none.
- The Tauri app's Lucide convention (`docs/agents/ui-styling.md` "Icons"):
  SVG strings in `crates/app/ui/src/icons.ts`, verbatim from
  `lucide-static` v1.48.0, under a `/*! ... */` header that names the
  icons, the version, the ISC text, the derivative and the Feather-derived
  icons; the full notice is `crates/app/ui/LICENSE-lucide`.
- `gpui-kit-assets` 0.9.1 is in `crates/native/Cargo.lock` (through
  `gpui-component`) but is not a direct dependency. It bundles Lucide
  1.43.0, has an `icon_assets!` macro embedding a chosen subset, and has no
  frown derivative.
- `filter.rs` writes the stored `stars` as `(0..=5)` whatever the menu's
  order; `native-app.md` describes the stored sections as "each in the
  menu's item order".
- `docs/plans/20261114-gpui-component-adoption/plan.md` is in flight with
  every step open: its Step 3 adds
  `application().with_assets(gpui_kit_assets::Assets)` and its Step 4
  restyles `menu_row`.

## Steps

- [x] Step 1: Give the filter menu's rows a shape per kind with Lucide icons, order the rating rows from 5 stars down, and head every section
  - Done when:
    - The flag rows (`Picked`, `Untagged`, `Rejected`) are drawn exactly as
      before.
    - The rating rows are ordered 5, 4, 3, 2, 1, 0 stars from the top (the
      unrated row last), and each row draws five Lucide star icons instead
      of the `U+2605` text: the `on` stars lit in `STARS_COLOR`, the other
      `5 - on` dim.
    - Each color label row leads with a square filled with the label's
      color (`tint_color`), and `No label` with an outline square, instead
      of the dots.
    - The `Face focus` rows lead with the Tauri app's icons in
      `MarkState::color()`: `Good` Lucide `scan-face`, `Low` the
      `scan-face` frown derivative, `Neutral` an empty slot of the same
      width.
    - `Portrait` leads with Lucide `rectangle-vertical` and `Landscape`
      with Lucide `rectangle-horizontal`.
    - Every section starts with a heading row, right after its separator:
      - new: `Flag` over Picked / Untagged / Rejected, `Rating` over the
        star rows, `Color label` over the label rows, `Orientation` over
        Portrait / Landscape;
      - unchanged: `Face focus`, `Eyes`, `Output` and the EXIF groups'
        headings;
      - `Reset` stays above the first separator without a heading.
    - A heading is not clickable: it is a `Row::Heading`, drawn as the
      existing headings are (no hover, no click handler, no check slot).
    - Every icon is a Lucide SVG (`star`, `square`, `rectangle-vertical`,
      `rectangle-horizontal`, `scan-face`) taken verbatim from
      `lucide-static` v1.48.0, the version `crates/app/ui/src/icons.ts`
      names, or a derivative the license header names (the frown, and the
      filled `star` / `square` if used). The license is handled as the
      Tauri app does it: a header on the file holding the SVGs and the
      full notice next to the crate (Trade-off 3).
    - Nothing else in the menu changes: the items' actions, the checked
      states, the menu staying open on a click, the Sort menu (no
      heading), the right-click menus and the stored filter (`filter.rs`
      keeps writing `stars` ascending).
    - `menu.rs`'s unit tests cover the new rows:
      - the row-order test expects, after `Reset` and its separator,
        `# Flag`, the three flags, `---`, `# Rating`, `stars 5` ...
        `stars 0`, `---`, `# Color label`, the eight labels, `---`,
        `# Orientation`, `Portrait`, `Landscape`, then the existing
        `# Face focus`, `# Eyes` and `# Output` sections;
      - a test that every separator of the filter menu is followed by a
        heading, with and without the `Output` section and with EXIF
        groups;
      - the checked-items test follows the new order (`Stars(4)` before
        `Stars(0)`);
      - the leads test expects the orientation rows' leads and still
        covers the flag, label and face leads;
      - `every_exif_heading_follows_the_tauri_names` skips the seven
        fixed headings instead of three;
      - the test names and the module comment no longer claim the HTML's
        order and labels where they now differ.
    - A unit test of the asset source: every icon path the menu draws
      loads `Some` bytes that start as an SVG, and an unknown path loads
      `None`.
    - `docs/agents/native-app.md`:
      - the "sort and filter menus" section says how the leads are drawn,
        that every section has a heading and which four the Tauri menu
        lacks, that the rating order, the label squares and the
        orientation icons differ from the Tauri menu on purpose, and that
        the stored `stars` order stays ascending (fix the "each in the
        menu's item order" wording for the stars);
      - a short section records the icon mechanism (the asset source,
        `svg().path(..)`, how to add an icon, the license header rule, one
        `AssetSource` per application);
      - the "gpui-component is a dependency only" rule's mention of "the
        icon assets" notes that the app now has its own source.
    - `CLAUDE.md`'s `crates/native` sentence names the new module, and the
      `src/menu.rs` clause no longer calls the filter rows a plain port
      where it differs.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (including the `native` job on all three platforms).
    - `learnings.md` records the pending manual check (Windows, the
      user's):
      - Build with `cargo build`, launch on a test copy with an explicit
        fresh `--index` and a test `--settings`, and open `Filter ▾`.
      - Expect a heading over every section (`Flag`, `Rating`,
        `Color label`, `Orientation`, `Face focus`, `Eyes`, `Output` when
        the folder has an export answer, the EXIF groups), none of them
        reacting to hover or a click.
      - Expect the flag dots unchanged; star rows 5 down to 0 with crisp
        star icons, lit and dim; seven filled squares and one outline
        square; the smiling and frowning `scan-face` in green and gray
        with `Neutral` aligned; the two rectangles for the orientations.
      - Expect the text of every row in a section to start at the same x,
        a click to still toggle each item, and the taller menu to scroll
        in a small window.
      - Also check a HiDPI display if available.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files:
      - `crates/native/src/menu.rs` (the order, the headings, `Lead`,
        tests, the module comment);
      - `crates/native/src/main.rs` (`menu_row`, `mod icons;`, the
        `application()` call);
      - `crates/native/src/icons.rs` (new);
      - `crates/native/LICENSE-lucide` (new, Trade-off 3);
      - `docs/agents/native-app.md`, `CLAUDE.md`.
      No `Cargo.toml` change.
    - Model (`menu.rs`):
      - Iterate the stars as `(0..=5).rev()`.
      - Push a `Row::Heading` after the separator of the flag, rating,
        label and orientation sections, as the `Face focus` section does.
        The heading drawing in `menu_row` needs no change.
      - Add a `Lead` variant for the orientation rows (for example
        `Lead::Orientation(Orientation)`).
      - `Lead::Label`, `Lead::Face` and `Label::Stars` keep their shape;
        only their doc comments ("dot") and their drawing change.
      - Keep the model free of GPUI and of asset paths: `menu_row` maps a
        lead to its icon.
    - Icons (`icons.rs`), mirroring `crates/app/ui/src/icons.ts`:
      - The SVG strings as constants under a header comment with the same
        content as the TypeScript one: the icon names, `lucide-static`
        v1.48.0, the ISC permission text, the derivatives, the
        Feather-derived icons among them (check `star` and `square`
        against the Feather list in `crates/app/ui/LICENSE-lucide`), and
        where the full notice is.
      - A unit struct implementing `gpui::AssetSource` that matches the
        path constants (for example `icons/star.svg`) and returns
        `Cow::Borrowed` bytes, `Ok(None)` otherwise; `list` returns the
        known paths under the prefix.
      - Register it with `application().with_assets(..)`.
    - SVG sources:
      - Copy `scan-face` and the frown from `icons.ts` (drop `aria-hidden`
        if the renderer objects; verify).
      - Take `star`, `square`, `rectangle-vertical` and
        `rectangle-horizontal` verbatim from lucide-static 1.48.0. It is
        not in `node_modules`; fetch that exact version. The copies under
        the cargo registry's `gpui-kit-assets-0.9.1/assets/icons/` are
        Lucide 1.43.0 and serve only as a cross-check.
    - Filled shapes (Trade-off 4):
      - Lucide `star` and `square` are outlines. The lit stars and the
        label squares use the same paths with `fill="currentColor"`, noted
        in the header as derivatives.
      - The dim stars and `No label` use the verbatim outlines (dim stars
        in the current `MUTED_COLOR` at 40 %, `No label` in a muted or
        border color that stays visible on the popover).
      - Do not use `gpui-kit-assets`'s `star-fill.svg` (an older polygon,
        not the Lucide 1.48.0 shape).
    - Drawing (`menu_row`):
      - `svg().path(..).size(px(12.)).flex_none().text_color(..)`, 12 px
        as the Tauri `.face`.
      - Keep the flag arm untouched.
      - The star row is five icons in a flex row with a small gap in place
        of the two text `div`s.
      - Verify at 100 % and at a fractional scale that the 2-unit stroke
        (1 px at 12 px) renders cleanly; if a shape reads badly at 12 px,
        choose the size per kind and record it.
    - Off the page path:
      - Nothing here is laid out or painted while the menu is closed, and
        `with_assets` only stores the source, so `native-app.md`'s A/B
        latency gate does not apply.
      - Do not change the always-drawn `Sort ▾` / `Filter ▾` buttons.
    - Coordination with `docs/plans/20261114-gpui-component-adoption/plan.md`:
      - Re-check `origin/main` at the start.
      - If its Step 3 has landed (`with_assets(gpui_kit_assets::Assets)`),
        the new source answers its own paths and delegates every other
        path to `gpui_kit_assets::Assets`, instead of replacing it.
      - If its Step 4 has landed, apply the lead changes on top of the
        themed `menu_row`.
      - If neither has, the PR description says that Step 3 there must
        compose the two sources the same way and that Step 4 must keep
        these leads and headings. Resolve conflicts in `main.rs`,
        `CLAUDE.md` and `native-app.md` with a merge commit.
    - Keep `crates/native` free of any dependency on `crates/app`: the
      SVGs and the notice are copies, not `include_str!`s of files under
      `crates/app`.
    - Everything committed is in English; Conventional Commits, for
      example `feat(native): draw the filter menu's icons by kind and head
      every section`.

## Trade-offs and risks

1. **Own asset source or `gpui-kit-assets`.** The native app has no icon
   mechanism to follow, so one is added.
   - Taken: an own `AssetSource` over SVG strings in `icons.rs`. It needs
     no new dependency or pin, uses the same Lucide version as the Tauri
     app (1.48.0), and can hold the frown and filled derivatives.
   - Not taken: a direct `gpui-kit-assets =0.9.1` pin with `icon_assets!`.
     It bundles Lucide 1.43.0, joins the exact pins the upgrade rule moves,
     and still needs an own source for the frown, so two mechanisms would
     exist.
2. **One `AssetSource` per application.** The component-adoption plan's
   Step 3 registers `gpui_kit_assets::Assets`. Whichever of the two lands
   second has to compose them (own paths first, then the kit's). If that
   is missed, either the checkbox's check or these icons silently draw
   nothing: a missing asset is not an error at build time. The asset
   source's unit test guards this plan's half.
3. **The license notice.**
   - Taken: as the Tauri app does it, a header on `icons.rs` plus a copy
     of the full notice at `crates/native/LICENSE-lucide`, so the crate
     carries its own notice and stays independent of `crates/app`.
   - Alternative: the header only, pointing at
     `crates/app/ui/LICENSE-lucide`. That is one file fewer, but the native
     crate's notice then lives in the Tauri app's folder.
   - `native-app.md`'s "no notice file while the native app has never
     shipped" rule is about the notices of compiled dependencies, not
     about SVGs copied into the repository, whose ISC terms ask for the
     notice in every copy.
4. **Filled star and square are derivatives.** Lucide draws outlines.
   - Taken: `fill="currentColor"` on the same paths (Lucide's documented
     way to fill an icon), named in the header as derivatives, with the
     unlit stars and `No label` as the verbatim outlines.
   - Alternative for the stars: outline stars for both states, differing
     only in color. Nothing derived, but weaker lit / dim contrast at
     12 px.
   - Alternative for the label: a plain `div` square with a small radius.
     No SVG, but not a Lucide icon, which the request asks for.
5. **The native menu no longer mirrors the Tauri menu.** `menu.rs` is
   documented as a port of `index.html`'s `#filter-menu` in the HTML's
   order and labels. After this the rating order, the label squares, the
   orientation icons and four headings differ. The Tauri menu is out of
   scope; the docs and the module comment record the divergence so a later
   port does not "fix" it. If parity is wanted, the Tauri menu needs its
   own change.
6. **The four new heading texts are proposals.** The Tauri menu has no
   heading for these sections, so there is no text to copy: `Flag`,
   `Rating` (the Sort menu's word), `Color label` (the docs' word) and
   `Orientation`. Other wordings (`Flags`, `Stars`, `Label`) change only
   the strings and the tests' expected arrays. The menu grows four rows
   taller; it already scrolls when it exceeds the window.
7. **The look can only be checked by hand.** No agent can see the window;
   the PR ships with unit tests of the model and the asset source and a
   pending manual check for the user. Small stroked icons may look soft at
   a fractional scale; the step measures and may pick a size per kind.

## Progress
