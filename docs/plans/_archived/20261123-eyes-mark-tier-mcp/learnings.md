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

## Step 2: the closed-eyes strip mark

- all-faces-check Step 3 had not landed on `main` when this step started
  (its plan's Step 3 still unchecked, no `faces_closed` in the code), so the
  mark was built here as planned.
- The rule is `mark::closed_eyes_mark(Option<&Focus>) -> bool`, true only for
  `Focus.eyes.eyes == Some(EyeState::Closed)`. `StripCell` gained one
  `closed_eyes: bool`; all-faces-check Step 3 only widens the function.
- The painting reuses `paint_face_icon` with a new `slot` argument (0 the
  face focus icon, 1 the closed-eyes mark, `FACE_ICON + 2` px further
  right), so both icons share size, inset and `overlay::HALO`. The color is
  a fixed `CLOSED_EYES_COLOR` (amber `0xffb347`) in `main.rs`.
- Clearance at `stripheight::MIN` (a 96 x 64 image box): the two slots span
  x 2..28 px (29 with the halo); a `999/999` burst badge at 11 px is about
  45 px wide, right-aligned to x 94, so it starts near x 49. The stars and
  the flag dot are on the top edge.
- `eye-off` comes from the `gpui-kit-assets` catalog (`IconName::EyeOff`);
  `the_catalog_paths_equal_their_icon_names` and
  `every_icon_path_loads_an_svg` cover it.

## Step 3: `get_view`'s eyes from the stored values

- `get_view` reads the current file's `Facts.focus.eyes` (`StoredEyes`) and
  answers from it when `eyes` (the state) is `Some`, naming the state by a
  `match` on `handpic_core::eyes::EyeState` in `companion.rs` (`index.rs`'s
  `serialize_eyes` is private, and `ondemand.rs` has its own match); else it
  falls back to `View.eyes`, the on-demand judgment, as before.
- The stored row cannot tell "pass 2 has not reached the file" from "pass 2
  meshed no face" (both a default `StoredEyes`), so the absence tests build
  both from `StoredEyes::default()`, told apart only by the preview's
  sharpness score being set or not.
- The old description's "judged a moment after showing it" was already wrong
  in the GPUI app: the on-demand judgment runs only while the `f` overlay is
  on. The new wording names the overlay.
- `README.md` / `README.ja.md` do not say when `eyes` appears, and
  `CLAUDE.md`'s `companion.rs` description does not mention the eyes, so
  neither changed.

- `mise run ci:local` failed three times, each on a different timing-sensitive
  test in a file this step does not touch, all passing when run alone, on a
  host under memory pressure (an earlier run was killed for low memory):
  `sidecar::tests::a_judgment_under_both_lands_in_the_xmp_and_the_dop` /
  `a_partial_both_write_records_the_xmp_stat_so_a_reopen_still_replays_the_dop`,
  then `sidecar::tests::an_unknown_label_under_both_keeps_the_newest_sidecars_label_in_both_files`
  (`crates/engine/src/sidecar.rs`), then
  `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped`
  (`crates/app/src/scan.rs`). The user accepted them as load-related
  flakiness and had the step committed with the PR's GitHub checks as the
  check; this step's own tests all passed.

## Step 4: the on-demand model run, closed as already met

- Confirmed in the code (Option A, no production change):
  `ondemand::read_eyes` has one caller, `overlay::judge`
  (`crates/app/src/overlay.rs`), whose one caller is `analyze_current` in
  `crates/app/src/main.rs`, which runs it only on a ticket from
  `Marks::wanted_eyes`. That returns one only while the overlay is on, the
  shown image is the current file's and the file is a RAW file (and it is
  neither judged nor any judgment in flight). No other crate calls it (the
  engine's `mcp.rs` uses only `read_preview`; the CLI none).
- No other reader waits for the run when stored values exist: the meta pane
  reads the window's `Facts` (`meta.rs`), the `Eyes` filter and the strip's
  `closed_eyes_mark` read `Focus.eyes`, and `get_view` answers from
  `Focus.eyes` when its state is `Some`, reading `View.eyes` (the on-demand
  cache) only in the `else` branch.
- The existing `no_eyes_are_judged_or_drawn_while_off_or_on_a_jpeg` used the
  fixture focus with no stored values, so it was extended with a RAW file
  whose answered focus holds a stored EAR: no judgment is requested once the
  overlay is off.
- `CLAUDE.md`'s `src/ondemand.rs` description still says the module is
  "behind the app's `faces_of` and `eyes_of`"; `faces_of` is in
  `crates/app/src/faces.rs` / `companion.rs` but no `eyes_of` exists in the
  GPUI app. Left as is (out of this step's scope).

## Deferred issues (todo candidates)

- Pending manual check (Step 2, closed-eyes strip mark): on a real folder of
  RAW files with people (any platform; Windows is the author's), let pass 2
  finish, then check that the strip cells showing the amber `eye-off` mark at
  the bottom-left are exactly the files whose meta pane `Eyes` row says
  closed, and exactly the files the filter menu's `Eyes` > `Closed` keeps;
  also that the mark appears live while `analyzing N / M` runs, sits right of
  the face focus icon without overlapping it or the burst badge at the
  minimum strip height, and is dimmed with a rejected cell. The step's
  checkbox was ticked on the automated criteria (unit tests of
  `closed_eyes_mark`, the icon tests, `mise run ci:local`). Files:
  `crates/app/src/main.rs` (`strip_cells`, `paint_face_icon`),
  `crates/app/src/mark.rs`.
- `CLAUDE.md`'s `src/ondemand.rs` description names the app's `eyes_of`,
  which does not exist in the GPUI app (the on-demand eyes go through
  `overlay::judge`). Basis: Step 4's verification of the `read_eyes`
  callers. Files: `CLAUDE.md`, `crates/app/src/overlay.rs`.
