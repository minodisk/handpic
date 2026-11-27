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

## Deferred issues (todo candidates)

- Pending manual check (Windows 11): in Neutral Light and Neutral dark, reject
  a strip cell and open `Filter ▾`. Expect the rejected cell to darken (not
  wash toward white) on Neutral Light and to look as before on Neutral dark,
  and the `Face focus` `Good` icon to be a dark green (`#15803d`) on the light
  menu and `#33ff33` on the dark one. The step's checkbox was ticked on the
  automated criteria; the main agent takes these screenshots for the PR body.
  Basis: plan Step 1's Done-when. Files: `crates/app/src/theme.rs`,
  `crates/app/src/mark.rs`, `crates/app/src/main.rs`.
