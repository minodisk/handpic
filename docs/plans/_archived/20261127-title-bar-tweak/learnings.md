# Learnings

## Step 1

- The thin gear's path data and circle are identical to lucide-static
  v1.48.0's `settings.svg` (fetched from unpkg); only `stroke-width` differs
  (2 to 1.5). The `icons.rs` header comment naming lucide-static v1.48.0 as
  the base is therefore true and was left as is.
- `docs/agents/app.md` names this plan as a backticked path, not a link, since
  the folder moves under `_archived/` when it completes.

## Deferred issues (todo candidates)

- Pending manual check (macOS, Linux): the title bar's retuned gear
  (`settings_button` in `crates/app/src/main.rs`: no `mr_2`, a
  `TITLE_BAR_HEIGHT`-wide, full-height, square-cornered ghost `Button` with
  the 14 px `icons::SETTINGS_THIN`), the 18 px app icon and the 13 px text
  were seen and approved only on Windows 11. Steps: run a dev build
  (`mise run app:dev`) on macOS and on Linux, open a folder, look at the
  title bar, hover and click the gear. Expected: the gear sits flush at the
  bar's right end (on Linux beside the window controls, on macOS at the right
  edge), its hover fill fills the bar's height with square corners, the icon
  is not clipped, the click opens the settings and a drag on the gear does not
  move the window. Step 1's checkbox was ticked on the automated criteria.
  Basis: plan Step 1's Done-when; documented in `docs/agents/app.md`'s title
  bar section.
