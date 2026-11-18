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

# Preview Background from the Theme

## Purpose

The native app's preview area (the surround behind the fit-view photo, the
1:1 crop and its placeholder, and the empty-state message) is painted a fixed
`#000000` (`PREVIEW_BACKGROUND` in `crates/native/src/main.rs`) whatever theme
is chosen, so under a shipped theme such as Catppuccin or Solarized it is the
one large surface that ignores the theme. Once this is done the surround is a
color of the theme in place, the darkest of its surfaces (in both modes, as
the user chose), and it follows a theme change live like the other `Roles`.

## Steps

- [x] Step 1: Paint the preview's surround from a new `Roles::preview`
  - Done when:
    - `Roles` has a `preview` field and the preview `div` in `main.rs` reads
      it (`roles.preview`) in place of `rgb(PREVIEW_BACKGROUND)`; the
      `PREVIEW_BACKGROUND` constant and its comment are gone, and no other
      fixed black is painted behind the photo.
    - Neutral dark's `preview` is `#000000` (pixel-identical to today).
    - For every other theme, light or dark, `preview` is the least-luminant of
      the theme's opaque surface tokens `background`, `popover`, `title_bar`,
      `status_bar` and `sidebar`.
    - Changing the mode or either theme in the Appearance page, and an OS
      appearance change in System mode, repaint the surround without a
      restart (it comes through `theme::apply`'s `cx.set_global(roles)`, as
      the other roles do).
    - Unit tests in `theme.rs` cover the derivation: Neutral dark's value;
      the pick among tokens with a hand-built `ThemeColor` (the darkest one
      wins whichever field holds it; a translucent candidate is skipped;
      `background` is the fallback); and, over every shipped theme of both
      modes, that `preview` is opaque and no more luminant than that theme's
      `background`.
    - `docs/agents/native-app.md` (the paragraph listing the `Roles` and the
      "Fixed in every theme" constants, which names `PREVIEW_BACKGROUND` and
      "the preview's black surround") and `CLAUDE.md` (the `theme.rs`
      description's list of what `Roles` holds) describe the new role and no
      longer list the surround as fixed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/native/src/theme.rs` (the field, the derivation, the
      tests), `crates/native/src/main.rs` (line ~5732 `.bg(...)`; delete the
      constant near line 6437), `docs/agents/native-app.md`, `CLAUDE.md`.
    - The black is painted in exactly one place: the `.bg(rgb(PREVIEW_BACKGROUND))`
      of the `div` wrapping the preview `canvas` and the empty message. The
      canvas itself fills nothing, so the fit view's letterbox, the 1:1
      crop's uncovered area and the empty state all show this one `div`.
      `render` already holds `let roles = theme::roles(cx);` a few lines
      above; use that local.
    - Keep `Roles::of(colors, neutral_dark)`'s shape: add `preview:
      rgb(0x000000).into()` to the Neutral dark branch (the same value as its
      `surface`, which is already darker than the palette's `background`
      `#0a0a0a`), and the derived value to the other branch. Put the pick in
      a small private pure function next to it (one that takes the candidates
      and returns the least-luminant opaque one), so the tests can drive it
      directly.
    - `ThemeColor`'s `background`, `popover`, `title_bar`, `status_bar` and
      `sidebar` are plain `Hsla` in gpui-component 0.9.1, so they compare
      directly.
    - Compare by relative luminance (linearized sRGB, 0.2126 / 0.7152 /
      0.0722) of the color converted to RGB, not by HSL `l`: the shipped
      themes have tinted surfaces where the two can order differently. Verify
      which `Hsla` to RGB conversion `gpui_kit` re-exports.
    - Skip a candidate whose alpha is under 1 rather than compositing it.
      `background` is opaque in every shipped theme and is the fallback.
    - Verify, from the resolved `ThemeColor` of a few shipped themes
      (`Theme::apply_config` as the existing
      `scrollbars_are_always_shown_after_finish` test does), what the pick
      comes out as; Catppuccin Mocha and Jellybeans should give their
      `title_bar` (`#11111B`, `#101010`). `status_bar` and `sidebar` are absent
      from most files and so come from gpui-component's defaults; if a default
      turns out to be an unrelated color for some theme, drop that token from
      the candidates and record it in `learnings.md`.
    - Update the existing tests `neutral_dark_roles_are_todays_colors` and
      `other_themes_derive_the_roles_from_their_tokens` for the new field.
    - Conventional Commit, for example
      `feat(native): paint the preview's surround in the theme's darkest surface`.

## Trade-offs and risks

- **Which color is "the darkest".** The least-luminant of the opaque surface
  tokens the app itself paints. `background` alone would leave the preview
  lighter than the chrome in Catppuccin Mocha, Jellybeans, Tokyo Night and
  others. Tokens the app never draws (`tab_bar`) are left out on purpose.
- **Neutral dark keeps `#000000`**, the darkest color that theme paints
  (its `Roles::surface`), so the default look does not change.
- **Light mode uses the same rule** (user's choice on 2026-11-18): white in
  Neutral light, a slightly recessed surface in tinted light themes.
- **Exposure judgment against a tinted or light surround** shifts perceived
  tones; inherent in the request. Neutral dark, the default, is unchanged.

## Progress

- (2026-11-18) Step 1 complete
