# Learnings

## Step 1

- `0x15803d` (Tailwind green-700) has relative luminance about 0.159, so
  `1.05 / 0.209` is about 5.0:1 on white; `#33ff33` is about 1.4:1. The
  ratio is stated in `menu_color`'s doc comment rather than computed, since
  `mark.rs` has no luminance helper and `theme::luminance` stays private.
- The `is_light` crossover: `(L + 0.05) / 0.05 = 1.05 / (L + 0.05)` gives
  `L = sqrt(0.0525) - 0.05`, about 0.179.
- Removing the two `todo.md` sections left two references to them in the
  native-themes item ("their own items below", "the dimmed reject has its own
  item below"); those phrases were adjusted in the same change.
- A cold `cargo test --bin handpic` in `crates/app` took over 3 minutes and was
  moved to the background by the harness.

- Seen on screen (2026-11-27, Windows 11, a debug build of this branch with
  `--features screenshot`, 100 %, MCP `set_settings` for the theme and
  `screenshot`): on Neutral Light a rejected cell darkens to a gray clearly
  apart from its neighbours and the white surface (a sky pixel `#919199` in
  a kept cell, `#424246` in the rejected one), its name still readable; on
  Neutral dark it looks as before. The `Filter ▾` menu's `Good` icon reads
  dark green on the light menu (an inner pixel `#278a4c`, antialiased
  `#15803d`) and `#31ed31` on the dark one. The pending check below is
  closed by this.

## Deferred issues (todo candidates)

- (Closed on 2026-11-27, see above.) Pending manual check (Windows 11): in Neutral Light and Neutral dark, reject
  a strip cell and open `Filter ▾`. Expect the rejected cell to darken (not
  wash toward white) on Neutral Light and to look as before on Neutral dark,
  and the `Face focus` `Good` icon to be a dark green (`#15803d`) on the light
  menu and `#33ff33` on the dark one. The step's checkbox was ticked on the
  automated criteria; the main agent takes these screenshots for the PR body.
  Basis: plan Step 1's Done-when. Files: `crates/app/src/theme.rs`,
  `crates/app/src/mark.rs`, `crates/app/src/main.rs`.
