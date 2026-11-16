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

# Native app icons from gpui-kit's catalog

## Purpose

`crates/native/src/icons.rs` embeds its own copies of eight Lucide SVGs. Five
of them (`star`, `square`, `rectangle-vertical`, `rectangle-horizontal`,
`scan-face`) are in the icon catalog of `gpui-kit-assets`, which the crate
already depends on (`=0.9.1`), with the same drawing. After this plan those
five come from gpui-kit (`star` and `square` from its default `Assets` set,
the other three selected from the catalog with `icon_assets!`), and the app
keeps only the three derivatives the
catalog does not have: the filled star, the filled square and the frowning
`scan-face`. The filter menu's leads and the strip's face focus icon look as
they do today.

Read [native-app.md](../../agents/native-app.md) first, in particular "The
icons are Lucide SVGs served by one `AssetSource`" and "A canvas paints an
icon with `paint_svg`".

What the `gpui-kit-assets` 0.9.1 source says (read at planning time, in
`src/lib.rs`, `src/icon.rs`, `src/native_assets.rs`, `build.rs` and
`default-icons.txt`):

- `icon_assets!($vis $name, [Variant, ...])` generates a unit struct with an
  `AssetSource` impl that embeds only the listed catalog icons. Its `load`
  answers a listed path with borrowed static bytes and any other path with
  `Ok(None)`; its `list` filters the listed paths by prefix.
- `IconName::path(self)` returns `gpui::SharedString`. It is not a `const fn`
  and does not return `&'static str`. The variants are `Star`, `Square`,
  `RectangleVertical`, `RectangleHorizontal` and `ScanFace`, with the paths
  `icons/star.svg`, `icons/square.svg`, `icons/rectangle-vertical.svg`,
  `icons/rectangle-horizontal.svg` and `icons/scan-face.svg`, the same
  strings as today's consts.
- `Assets.load` returns `Err` for a path it does not hold and `Ok(None)` for
  the empty path.
- `icons/star.svg` and `icons/square.svg` are also in the default `Assets`
  set (`default-icons.txt`); the other three are not.
- The catalog's SVGs differ from the app's copies only in whitespace and in
  carrying `width="24" height="24"`. The catalog is Lucide 1.43.0
  (`lucide.json`); the app's copies are lucide-static v1.48.0.
- The catalog's `icons/star-fill.svg` is a different drawing (a polygon), and
  there is no filled square and no frowning `scan-face`.

## Steps

- [x] Step 1: Serve the five catalog icons from `gpui-kit-assets` and keep
      only the three derivatives as own SVGs
  - Done when:
    - `icons.rs` holds SVG strings only for `STAR_FILLED`, `SQUARE_FILLED`
      and `SCAN_FACE_FROWN`; `STAR_SVG`, `SQUARE_SVG`,
      `RECTANGLE_VERTICAL_SVG`, `RECTANGLE_HORIZONTAL_SVG` and
      `SCAN_FACE_SVG` are gone.
    - `Icons::load` answers the three own paths first, then the three catalog
      icons (`rectangle-vertical`, `rectangle-horizontal`, `scan-face`) from
      an `icon_assets!` source, then `gpui_kit_assets::Assets`, which serves
      `star` and `square`; a
      path in none of them still loads `Ok(None)`, as does the empty path.
    - `Icons::list` names each path under a prefix once, across the three
      sources.
    - The eight `pub const` paths keep their names and values, and
      `main.rs` and `mark.rs` are unchanged.
    - `icons.rs`'s tests cover: every one of the eight paths loads an SVG;
      each of the five catalog consts equals its `IconName` variant's
      `path()`; a derivative loads its own bytes; `icons/check.svg` loads
      from `Assets`; an unknown path and the empty path load nothing; the
      listing names each of the eight paths once and `icons/check.svg`, and
      is empty for `fonts/`.
    - `mark.rs`'s tests pass unchanged.
    - The header comment of `icons.rs` names only what the file still holds,
      and `CLAUDE.md`'s sentence on `src/icons.rs` and the two passages of
      `docs/agents/native-app.md` describe the new composition.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - Checked by hand in the running app (`mise run native:dev`): the filter
      menu's star rows, label squares, orientation rectangles and `Face
      focus` faces, and the strip's face focus icon on a good and on a low
      frame, look as before.
  - Implementation approach:
    - Files: `crates/native/src/icons.rs`, `CLAUDE.md`,
      `docs/agents/native-app.md`. No change to `crates/native/Cargo.toml`
      (the dependency is already there) and none expected to `Cargo.lock`.
    - Keep the five catalog path consts as string literals. `IconName::path`
      is not `const`, so the consts cannot be defined from it, and
      `MarkState::icon` returns `Option<&'static str>`, which `paint_svg` and
      the menu's `icon` closure take. A test ties each const to its
      `IconName` variant's `path()` instead.
    - Declare the catalog source in `icons.rs` with the macro, private to the
      module: `gpui_kit_assets::icon_assets!(Catalog, [RectangleVertical,
      RectangleHorizontal, ScanFace]);`. `star` and `square` are left out:
      the default `Assets` already holds them, so listing them would embed
      them twice (decided with the user). The test that every path loads an
      SVG guards them; if a later gpui-kit-assets drops either from the
      default set, that test fails and the icon moves into the `icon_assets!`
      list. Say so in a comment next to the macro. The macro needs no extra
      import; it resolves `AssetSource`, `Result` and `SharedString` through
      the crate's own hidden module.
    - `SVGS` shrinks to the three derivatives. `load`: the own table, then
      `Catalog.load(path)?` when it is `Some`, then
      `Assets.load(path).unwrap_or(None)` as today. `list`: the own paths,
      then `Catalog`'s, then `Assets`', skipping a path already present.
    - Tests: `an_own_path_wins_over_the_components_one` no longer holds for
      `STAR` (it has no own bytes). Replace it with a test that a derivative
      loads its own bytes (e.g. `STAR_FILLED` against `STAR_FILLED_SVG`) and
      one that the five consts equal `IconName::{Star, Square,
      RectangleVertical, RectangleHorizontal, ScanFace}.path()`. The
      start-of-SVG assertion in `every_icon_path_loads_an_svg` must accept
      the catalog's format: its files start with `<svg` followed by a
      newline, not `<svg ` with a space, and end with `</svg>` followed by a
      newline. The listing test iterates the eight consts, not `SVGS`.
    - Header comment of `icons.rs`: say that the file's own SVGs are
      derivatives of Lucide `star`, `square` and `scan-face` (lucide-static
      v1.48.0, ISC), keep the ISC permission paragraph, the derivative
      description, the Feather / MIT note for `square` (the filled square
      derives from it) and the pointer to `crates/native/LICENSE-lucide`.
      Drop `rectangle-vertical` and `rectangle-horizontal` from it, and say
      that the unmodified icons come from `gpui-kit-assets`, under its own
      `LICENSE-LUCIDE`. Update the module doc comment (`//!`) to the three
      sources.
    - `crates/native/LICENSE-lucide` stays unchanged: it is Lucide's notice
      verbatim, and all of it still applies to the three derivatives.
    - `CLAUDE.md`: the clause "from `src/icons.rs` (the SVGs under their
      license header, served by `Icons`, the application's one
      `AssetSource`, to `svg().path(..)`, falling back to
      `gpui_kit_assets::Assets` for the icons gpui-component's controls
      draw)" becomes one that names the three own derivatives, the catalog
      icons selected with `icon_assets!`, and the `Assets` fallback.
    - `docs/agents/native-app.md`: the rule near line 141 ("the app already
      has its own `AssetSource`") and the section "The icons are Lucide SVGs
      served by one `AssetSource`" (near line 1058). The section's "to add
      an icon" rule changes to: an icon the catalog has is added to the
      `icon_assets!` list with a path const; only a derivative is copied in
      as an own SVG, and an icon the default `Assets` already holds needs only
      the path const. The sentence "`icons/star.svg` and `icons/square.svg`
      are in both; the app's win" changes to the new order (own derivatives,
      then the catalog selection, then `Assets`, which serves `star` and
      `square`). The list of what the tests
      check follows the new tests.
    - Verify during implementation that the catalog's `width` / `height`
      attributes and whitespace change nothing on screen. `icons/check.svg`
      from `Assets` has the same format and already draws correctly, so no
      difference is expected; the hand check above confirms it.
    - `crates/native` is a separate workspace on Rust 1.98.1; run its clippy
      and tests the way `mise run ci:local` does (see `tools/ci/local.sh`),
      not from the root workspace.

## Trade-offs and risks

- **`star` and `square` from the default `Assets`** (decided with the user):
  nothing is embedded twice. A gpui-kit-assets bump that drops either from
  the default set builds fine but draws nothing; the every-path-loads test
  catches it, and the fix is adding the icon to the `icon_assets!` list.
- **Path consts stay literals; callers do not switch to `IconName`.**
  Switching would change `MarkState::icon`'s return type, its tests, the
  menu's `icon` closure and the strip's `paint_svg` call, for no visible
  gain. The const and the catalog path are tied only by a test.
- **The catalog is Lucide 1.43.0, the app's derivatives v1.48.0.** The five
  drawings are the same today. The dependency is pinned (`=0.9.1`), so only
  a deliberate bump can change them, and such a bump could make a derivative
  differ from its base icon.
- **Rendering is checked by hand, not by a test.**

## Progress

- (2026-11-16) Step 1 complete
