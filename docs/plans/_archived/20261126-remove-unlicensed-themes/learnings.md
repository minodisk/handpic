# Learnings

## Step 1

- The fallback to `NEUTRAL` needed no code: `Setting::from_setting` resolves
  each stored name through `shipped`, so a removed theme's name reads as
  Neutral. The new test `a_removed_theme_is_neutral_and_not_offered` in
  `crates/app/src/theme.rs` pins it for `Adventure`, `Adventure Time`,
  `Alduin` and `Kibble` in both fields and both modes' `names`.
- `crates/app/themes/LICENSE` notes were renumbered (1) to (9). The
  collection's "each theme's terms belong to its author" fact, which the
  removed note (1) held, moved into note (3) (Fahrenheit), and note (5)
  (Harper) refers to it.
- `docs/agents/app.md` records the earlier releases (up to 2.1.0) as having
  shipped all 21 files, as history.
- In Git Bash, a long Python heredoc inside the Bash tool failed to parse
  ("unexpected EOF while looking for matching `'`"); writing the script and
  text to the scratchpad with the Write tool and running
  `mise x -- python -I script.py` worked.
