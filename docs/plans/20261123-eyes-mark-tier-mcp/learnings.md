# Learnings

## Step 1: the good-photo tier in the engine

- The rule is `StoredEyes::photo_tier(&self) -> bool` in
  `crates/engine/src/index.rs`, with the five cuts and `GOOD_FULL_EYE_SHARP`
  (defined as `handpic_core::fullsharp::SOFT_BELOW`) next to it. It reads
  only `StoredEyes`: the AF point and manual focus never took part, so the
  method sits on `StoredEyes`, not `Focus`, and the app calls
  `f.eyes.photo_tier()`.
- The moved boundary tests live in a `photo_tier_tests` module right after
  the constants rather than in `index.rs`'s large `tests` module at the end
  of the file, so they sit next to the rule. Their fixtures are built from
  `StoredEyes::default()` with struct update syntax.
- Two of `mark.rs`'s tier tests did not move as they were:
  `photo_tier_is_false_without_a_focus` tested the app's
  `Option<&Focus>` wrapper, which is gone; `af_eye_state(None)` being
  `Neutral` (`af_eye_state_is_neutral_without_a_focus_as_on_a_jpeg`) still
  covers it. `the_full_size_thresholds_are_fullsharp_s` lost its
  `GOOD_FULL_EYE_SHARP == SOFT_BELOW` half, now true by definition, and was
  renamed `the_low_full_size_threshold_is_fullsharp_s`.
- `get_view`'s `current.photo_tier` is always present for a current photo
  (`"good"` or `null`), so the existing tests that compared the whole
  `current` object gained `"photo_tier": null`.
- `README.md` / `README.ja.md` do not list what `get_view` reports (they
  point at the usage guide), so they needed no change.
- `docs/agents/app.md` and `docs/agents/shared-crates.md` named
  `mark.rs`'s `photo_tier` and the constants there; their paths were
  corrected so the guides do not point at code that is gone.
- Tooling: a Python script fed through a Bash heredoc lost the `\`
  line continuations of `mcp.rs`'s `#[tool(description = ...)]` string
  (the lines were joined); the Edit tool is the safe way to edit those.
