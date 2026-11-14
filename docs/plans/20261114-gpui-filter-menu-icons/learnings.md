# Learnings: gpui-filter-menu-icons

## Step 1

- `origin/main` at `ec524107` still had no `with_assets` call and the
  themed `menu_row` of the component-adoption plan had not landed, so
  `Icons` is registered alone. That plan's Step 3 must compose
  `gpui_kit_assets::Assets` with `icons::Icons` (own paths first) rather than
  replace it, and its Step 4 must keep these leads and headings.
- lucide-static 1.48.0 was fetched from unpkg; `star`, `square`,
  `rectangle-vertical` and `rectangle-horizontal` are byte-identical in their
  paths to the `gpui-kit-assets` 0.9.1 copies (Lucide 1.43.0). The SVGs are
  kept in the compact one-line form of `crates/app/ui/src/icons.ts`, without
  the `class`, `width` / `height` and the Tauri `aria-hidden`: usvg takes the
  size from the `viewBox` and GPUI renders at the element's size.
- `square` is on the Feather list in `LICENSE-lucide`; `star`,
  `rectangle-*` and `scan-face` are not. The header names `square` as
  Feather-derived.
- `gpui::AssetSource::load` returns `gpui::Result` (anyhow), so no new
  dependency was needed.
- A heredoc-fed Python script with `"\u{2605}"` inside a triple-quoted
  string failed under Git Bash with a `unicodeescape` error; a script file
  with raw strings worked.
- `mise run fmt` failed at its `vp fmt` stage in this fresh worktree
  (no `node_modules`; `pnpm` is not on the Git Bash `PATH`);
  `mise exec -- pnpm install --frozen-lockfile` fixed it.
- The size stays 12 px for every kind; whether the 1 px stroke reads well at
  12 px and at a fractional scale is part of the manual check below.

## Deferred issues (todo candidates)

- **Pending manual check (Windows, the user's): the filter menu's icons and
  headings.** Basis: Step 1's Done-when; no agent can see the window. Files:
  `crates/native/src/main.rs` (`menu_row`), `crates/native/src/menu.rs`,
  `crates/native/src/icons.rs`. Step 1's checkbox was ticked on the
  automated criteria. Steps:
  - Build with `cargo build` in `crates/native`, launch on a test copy with
    an explicit fresh `--index` and a test `--settings`, and open `Filter ▾`.
  - Expect a heading over every section (`Flag`, `Rating`, `Color label`,
    `Orientation`, `Face focus`, `Eyes`, `Output` when the folder has an
    export answer, the EXIF groups), none of them reacting to hover or a
    click.
  - Expect the flag dots unchanged; star rows 5 down to 0 with crisp star
    icons, lit and dim; seven filled squares and one outline square; the
    smiling and frowning `scan-face` in green and gray with `Neutral`
    aligned; the two rectangles for the orientations.
  - Expect the text of every row in a section to start at the same x, a
    click to still toggle each item, and the taller menu to scroll in a small
    window.
  - Also check a HiDPI display if available. If a shape reads badly at
    12 px, pick a size per kind (`MENU_ICON` in `main.rs`).
