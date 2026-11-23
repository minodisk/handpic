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

# Closed-eyes strip mark, engine-side good-photo rule and stored eyes over MCP

## Purpose

Four unfinished items of `todo.md`'s "Core: face/eye-aware focus check for
culling" section, all about values pass 2 already stores for the judged face:

1. > Mark closed eyes in the strip. Pass 2 now stores the EAR for the face
   > the cue meshes and the filter menu has an `Eyes` section (the
   > `mesh-eyes-index` work), but no tile shows a closed-eyes mark yet; the
   > stored `eyes` state (`open` / `closed` / `unknown`) is on `Focus`, so
   > the remaining work is the mark itself. Files:
   > `crates/app/src/strip.rs`, `crates/app/src/mark.rs`.
2. > Derive the good-photo rule in the engine, in
   > `crates/engine/src/index.rs`, so the MCP `get_view` tool can carry it.
   > The rule lives only in `crates/app/src/mark.rs` (`photo_tier`) and
   > `get_view` says nothing about it. Done when the index derives the tier
   > from the stored columns with the thresholds in one place (the app
   > reads it instead of recomputing), `get_view` carries it for the
   > current file, and the boundary tests of `mark.rs` move with the rule.
   > Files: `crates/engine/src/index.rs`, `crates/app/src/mark.rs`,
   > `crates/app/src/companion.rs`.
3. > Repoint the MCP `get_view` tool's `eyes` answer to the stored values.
   > Pass 2 now stores the EAR and the pose of the face it meshes (the
   > `mesh-eyes-index` work: `eyes_ear`, `eyes`, `eyes_closed` and `pose` on
   > `Focus` / `FaceReady`, `docs/plans/_archived/20261008-mesh-eyes-index/plan.md`),
   > but the app's `get_view`'s `eyes` still comes from the on-demand
   > judgment (`EyesCache`), so it answers only for a file already shown
   > (see `docs/plans/_archived/20261008-mcp-eyes-pose/plan.md`, written for
   > the Tauri app). Answer from the stored values when present, falling
   > back to the on-demand judgment, and keep the `eyes` key absent from
   > `get_view` until a file is judged. Done when `get_view` answers from
   > the stored values and a test covers the key's absence before a
   > judgment. Files: `crates/app/src/companion.rs`,
   > `crates/app/src/eyes.rs`.
4. > Skip the second model run in `ondemand::read_eyes` for a file whose
   > scan already stored the EAR and pose (the `mesh-eyes-index` work stores
   > them, and the meta pane reads them). The on-demand judgment still runs
   > for the shown file because the `f` overlay's face mesh needs the 478
   > points, and it is the fallback for files with no stored value (face
   > under 60 px, or a file pass 2 has not reached). Reusing the scan's
   > mesh needs the points stored or a path that runs the model only while
   > `f` is on. Files: `crates/engine/src/ondemand.rs` (`read_eyes`),
   > `crates/app/src/overlay.rs`, `crates/engine/src/index.rs`,
   > `crates/core/src/eyes.rs`.

Once done, a blink shows on the strip cell without opening the frame, an MCP
client reads the good-photo tier and the eyes of the current photo from the
index (whether or not the `f` overlay is on), the good-photo cuts live in one
place in the engine, and the on-demand model run is confirmed to happen only
for the `f` overlay's mesh.

The other items of that section are out of scope. No step changes the index
schema, `FACES_VERSION` or `EXTRACTOR_VERSION`.

`docs/plans/20261123-all-faces-check/plan.md` is in progress in another
session. Its Step 2 adds `faces_judged` / `faces_closed` / `faces_soft` to
`StoredEyes` (`SCHEMA_VERSION` 28, `FACES_VERSION` 12) and its Step 3 adds a
strip cell mark when anyone's eyes are closed. This plan's Step 2 builds that
mark for the judged face behind one pure rule, so all-faces-check Step 3 only
widens the rule and reuses the mark.

## Steps

- [x] Step 1: Derive the good-photo tier in the engine and carry it in `get_view` (todo item 2)
  - Done when:
    - `crates/engine/src/index.rs` derives the tier from the stored values
      (`StoredEyes`), and the cuts (`GOOD_EYE_EAR`, `GOOD_MAX_YAW`,
      `GOOD_MAX_PITCH`, `MAX_EYE_OFFSET`, `MIN_EDGE_GAP`, the full-size
      sharpness cut) are defined only there
    - `crates/app/src/mark.rs` no longer holds the rule or those cuts:
      `af_eye_state` asks the engine for the tier, and every other app use of
      a moved constant (`meta.rs`, the tests of `filter.rs`) reads the
      engine's
    - The strip's face icon, the `f` overlay's mark color, the `Face focus`
      filter and the meta pane's `Eye focus` verdict behave as before; the
      existing `af_eye_state`, `focus_mark` and filter tests pass with only
      their imports changed
    - `get_view`'s current photo carries the tier (see the approach for the
      shape), the `get_view` tool description in `crates/engine/src/mcp.rs`
      says what it is, and `companion.rs` tests cover a good frame, a frame
      that is not good and a file without a focus (a JPEG)
    - The `photo_tier_*` boundary tests of `mark.rs` are moved to the engine
      next to the rule, not copied
    - `docs/humans/usage.md` / `docs/humans/usage.ja.md` (the MCP tools
      table's `get_view` row) and `README.md` / `README.ja.md` where they
      describe what `get_view` reports are updated; `CLAUDE.md`'s
      `src/index.rs`, `src/mark.rs` and `src/companion.rs` descriptions say
      where the rule lives
    - The item "Derive the good-photo rule in the engine" is removed from
      `todo.md` (only that item); the other items there that name the cuts
      in `crates/app/src/mark.rs` are left as they are unless their file
      reference would now be wrong, in which case only the path is corrected
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/engine/src/index.rs` (the rule, the cuts, the moved
      tests), `crates/app/src/mark.rs` (`clears` / `photo_tier` and the
      constants removed, `af_eye_state` calling the engine),
      `crates/app/src/meta.rs` and `crates/app/src/filter.rs` (imports of
      `GOOD_FULL_EYE_SHARP`), `crates/app/src/companion.rs` (`get_view`),
      `crates/engine/src/mcp.rs` (description),
      `crates/core/src/fullsharp.rs` (two test doc comments that point at
      `crates/app/src/mark.rs`).
    - Shape of the engine API: a method computed on call (for example
      `Focus::photo_tier` / `StoredEyes::photo_tier`), not a stored derived
      field. `StoredEyes` is built by struct literal and mutated field by
      field in app and engine tests, where a derived field would go stale,
      and a method leaves `StoredEyes::new` and the struct's fields alone,
      which all-faces-check Step 2 edits. See "Trade-offs and risks" for the
      field alternative.
    - The sharpness cut is `handpic_core::fullsharp::SOFT_BELOW` itself
      (`pub`, 0.162), not a second literal: define the engine's
      `GOOD_FULL_EYE_SHARP` from it, so the figure exists once.
      `fullsharp.rs`'s test `the_threshold_is_the_figure_the_ui_cuts_good_at`
      has a comment naming `crates/app/src/mark.rs`; update it.
    - `LOW_FULL_EYE_SHARP`, `MarkState`, `af_eye_state`, `focus_mark` and
      the face marks stay in `mark.rs`: the Low state and the colors / icons
      are the app's. The todo item moves the tier only.
    - The rule itself does not change: all five values present and inside
      their cuts, a focus with no recorded point or with manual focus still
      judged on its values (the existing tests
      `photo_tier_passes_a_candidate_with_no_recorded_point_on_its_values`
      and the `any_value_is_missing` cases move as they are).
    - Build the moved tests' fixtures from `StoredEyes::default()` with
      struct update syntax rather than a literal naming every field, so
      all-faces-check Step 2's new fields do not break them.
    - `get_view`: the current photo's `Focus` is already in the view
      (`View.facts`, `Facts.focus`); no new plumbing from `main.rs` is
      needed. Suggested key: `current.photo_tier`, `"good"` or `null`
      (`focus.ts`'s tier has the one value `good`), always present for a
      current photo. `null` covers both "not good" and "pass 2 has not
      reached the file"; say so in the description.
    - `get_photo` is not changed by this step.

- [x] Step 2: Mark closed eyes on the strip cell (todo item 1)
  - Done when:
    - A strip cell draws a closed-eyes mark when the stored eye state of the
      frame's judged face is closed (`Focus.eyes.eyes`), and none for open,
      unknown, a file without a focus (a JPEG) or a file pass 2 has not
      reached
    - The mark appears live as `faces-progress` reaches the file, with no
      re-listing
    - The existing face focus icon, burst badge, stars, flag dot and label
      band are drawn where they were, and the new mark overlaps none of them
      at the strip's minimum height (`stripheight::MIN`)
    - The rule is one pure function in `crates/app/src/mark.rs` with unit
      tests (closed, open, unknown, no focus), and the icon path loads
      (`icons.rs`'s `every_icon_path_loads_an_svg` covers the new icon)
    - `README.md` / `README.ja.md` and `docs/humans/usage.md` /
      `docs/humans/usage.ja.md` describe the mark, including that it judges
      one face (nearest the AF point, else the largest confident one) and
      that a face under 60 px is not judged; `CLAUDE.md`'s `mark.rs`,
      `icons.rs` and strip descriptions are updated
    - The manual check on a real folder (the mark agrees with the meta
      pane's `Eyes` row and with the `Eyes` filter's `Closed`) is listed as
      a pending manual check in the PR body
    - The item "Mark closed eyes in the strip" is removed from `todo.md`
      (only that item)
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (both edit `mark.rs`; the mark does not
      depend on the tier).
    - Files: `crates/app/src/mark.rs` (the pure rule),
      `crates/app/src/main.rs` (`StripCell`, the cell construction next to
      `face: mark::af_eye_state(..)`, the painting next to `paint_face_icon`
      / `paint_burst_badge`), `crates/app/src/icons.rs` (the icon).
      `crates/app/src/strip.rs` only if a layout constant belongs there; its
      `Marks` is the judgment's (stars, flag, label) and should not take the
      eye state.
    - Built for all-faces-check Step 3 to reuse: the cell holds one boolean
      (or small enum) filled by one function of `Option<&Focus>` in
      `mark.rs`, and the painting takes only that. All-faces-check Step 3
      then widens the function to "the judged face is closed, or
      `faces_closed` is at least 1" and changes neither `StripCell`'s shape
      nor the painting. Name the function for the mark, not for the judged
      face (for example `closed_eyes_mark`), and say this in its doc
      comment.
    - Icon: the unmodified Lucide `eye-off` from the vendored
      `gpui-kit-assets` 0.9.1 through `icon_assets!` (the catalog holds
      `eye-off` and `eye`, not `eye-closed`). No new drawing and no new
      license text.
    - Place: the cell's corners hold the flag dot (top-left), the stars
      (top-right), the face focus icon (bottom-left) and the burst badge
      (bottom-right, right-aligned text across the box). A closed frame is
      never `Good` (`EYES_CLOSED_EAR` 0.137 is under `GOOD_EYE_EAR` 0.25)
      but can be `Low`, so the frowning icon and the mark can both show.
      Suggested: a fixed second slot right of the face focus icon at the
      bottom-left, the same size (`FACE_ICON`) and halo (`overlay::HALO`),
      so neither icon moves when the other appears. Verify at
      `stripheight::MIN` that it clears a three-digit burst badge.
    - Color: a fixed constant like the other over-the-photo colors
      (`CLAUDE.md`: "the judgment, label band and over-the-photo colors
      staying fixed"), not a theme token.
    - Live update needs no new code: the cells are rebuilt on every render
      from `self.facts`, which `filter::apply_face_ready` already updates.
    - No tooltip in this step (`strip::tip_regions` holds only the label and
      failure texts, which must not overlap).

- [x] Step 3: Answer `get_view`'s `eyes` from the stored values (todo item 3)
  - Done when:
    - `get_view`'s `current.eyes` comes from the index's stored values of
      the current photo when they hold an eye state, whether or not the `f`
      overlay is on or the photo has been judged on demand; the shape
      (`state`, `probability`, `pose`) is unchanged
    - Without a stored state it falls back to the on-demand judgment as
      today: the judgment's values, or `null` when it found no face to judge
    - The `eyes` key is absent when there is neither a stored state nor a
      settled on-demand judgment, and tests cover it for a photo pass 2 has
      not reached, a photo pass 2 stored no mesh for and a photo with no
      focus (a JPEG)
    - Tests also cover: stored values answering with no on-demand judgment,
      the stored values winning over a differing on-demand judgment, and the
      fallback (a judgment, and `null`)
    - The `get_view` tool description in `crates/engine/src/mcp.rs` and the
      `get_view` row of `docs/humans/usage.md` / `docs/humans/usage.ja.md`
      (and `README.md` / `README.ja.md` if they say when `eyes` appears) no
      longer say the key appears "a moment after showing" the photo: they
      say it is there once the scan's second pass has stored it, or, before
      that, once the `f` overlay has judged the shown photo
    - `CLAUDE.md`'s `src/companion.rs` description is updated if its wording
      about the eyes changes
    - The item "Repoint the MCP `get_view` tool's `eyes` answer to the
      stored values" is removed from `todo.md` (only that item)
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (both edit `get_view` and its description).
    - Files: `crates/app/src/companion.rs` (`get_view`, `View.eyes`'s doc
      comment, tests), `crates/engine/src/mcp.rs` (description), the docs.
      `crates/app/src/eyes.rs` needs at most a module doc change: the cache
      stays as the fallback and as the `f` overlay's mesh source.
      `crates/app/src/main.rs` should need none: `View` already carries
      `facts` and `eyes`.
    - Stored source: `view.facts[path].focus.eyes` (`StoredEyes`): `eyes`
      for the state, `eyes_closed` for the probability, `pose` for the pose.
      "Present" means `eyes` is `Some`; the three are derived from the one
      stored EAR, so they are present together (the pose alone can be
      `None`, the fit having failed, as in the on-demand answer).
    - The state's name must be the same string the on-demand answer uses
      (`open` / `closed`); `index.rs` serializes `EyeState` with a private
      `serialize_eyes`, so the step either reuses a public name function if
      one exists or matches on `EyeState` in `companion.rs`.
    - The stored row cannot tell "pass 2 has not run" from "pass 2 ran and
      meshed no face": both leave the EAR `NULL`. So a photo with no face
      answers `null` only through the on-demand fallback (while `f` is on)
      and is absent otherwise. See "Trade-offs and risks".
    - The existing tests `reports_null_eyes_for_a_file_judged_without_a_face`
      and `leaves_the_eyes_key_out_until_the_file_has_been_judged` keep
      their meaning for the fallback path.

- [ ] Step 4: Close out the on-demand model run for files with stored values (todo item 4)
  - Done when:
    - It is confirmed in the code, and recorded in this folder's
      `learnings.md`, that the Face Landmarker run of `ondemand::read_eyes`
      happens only while the `f` overlay is on and only for the shown RAW
      file, so a file whose scan stored the EAR and pose costs no second
      model run unless the overlay needs its mesh, and that after Step 3 no
      other reader (the meta pane, the filter, the strip mark, `get_view`)
      waits for that run when stored values exist
    - A test pins that no judgment is requested while the overlay is off
      (the existing `no_eyes_are_judged_or_drawn_while_off_or_on_a_jpeg` in
      `crates/app/src/overlay.rs`; extended only if it does not already
      cover a file whose focus holds stored values)
    - `docs/humans/performance.md` / `docs/humans/performance.ja.md`
      ("Closed-eyes judgment on demand" and the summary table's row), the
      doc comments of `ondemand::read_eyes` and `crates/app/src/eyes.rs`,
      and `CLAUDE.md` agree with that: the on-demand judgment is the `f`
      overlay's mesh source and `get_view`'s fallback, nothing else
    - The item "Skip the second model run in `ondemand::read_eyes`" is
      removed from `todo.md` (only that item)
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 3 is merged.
    - What the investigation found: `Marks::wanted_eyes`
      (`crates/app/src/overlay.rs`) returns a ticket only while the overlay
      is on, the shown image belongs to the current file and the file is a
      RAW file; `analyze_current` in `main.rs` is its only caller, and
      `overlay::judge` the only caller of `ondemand::read_eyes`. The meta
      pane reads only the window's `Facts` (`meta.rs`), and the filter and
      the strip read the stored state. The todo item was written when the
      Tauri app judged every shown file for the meta pane's `Eyes` row; the
      GPUI app already takes the item's second route ("a path that runs the
      model only while `f` is on").
    - With the overlay on, the run stays: the 478 points are what the
      overlay draws, and they are not stored. No change to
      `crates/engine/src/ondemand.rs`'s behavior, `crates/core/src/eyes.rs`
      or `crates/engine/src/index.rs` is planned.
    - If the caller chooses to store the points instead (Option B in
      "Trade-offs and risks"), this step is replaced by an index change that
      must be sequenced with all-faces-check Step 2.

## Decisions

Made at plan approval (2026-11-23), each the recommended option:

- Item 4 is closed as already met (Option A); the mesh points are not stored.
- The tier is a method computed on call, not a serialized field.
- `get_view` carries `photo_tier` (`"good"` or `null`), not `face_focus`.
- The closed-eyes mark is the unmodified Lucide `eye-off` in a fixed second
  slot right of the face focus icon, with no tooltip.
- A photo with no face stays absent from `get_view`'s `eyes` while `f` is off.

## Trade-offs and risks

- **Item 4: close it as already met (planned) or store the mesh points.**
  - Option A, planned: no production change. The model runs on demand only
    while `f` is on, which is the item's own second route and already holds.
    Cost that remains: with `f` on, each shown file takes one read, decode,
    detection and model run (80 to 156 ms in `performance.md`), in the
    background, one at a time, superseded by paging.
  - Option B: store the 478 points per judged file in the index (about
    3.8 KB as `f32` pairs, about half quantized to `u16`; about 8 MB on a
    2134-file folder) and draw the overlay's mesh from them, dropping the
    on-demand run for stored files. Consequences: a new column
    (`SCHEMA_VERSION` bump) and a `FACES_VERSION` bump so pass 2 re-runs on
    every folder, both colliding with all-faces-check Step 2 (28 / 12),
    which would have to be merged first or share the bump; a larger index
    under the size cap; `get_photo` / `faces-progress` either carrying or
    deliberately skipping the points; and the on-demand path kept anyway as
    the fallback for files pass 2 has not reached. It saves background work
    only while `f` is on.
  - Under Option A, Step 4 is small enough to fold into Step 3's PR if the
    caller prefers three PRs; it is kept separate here because the two close
    different todo items.
- **Item 2: a method or a serialized derived field.** The plan uses a method
  on `Focus` / `StoredEyes`. The alternative is a derived field next to
  `eyes` and `eyes_closed`, filled in `StoredEyes::new`, which would put the
  tier in `get_photo`'s and `faces-progress`'s JSON with no further code.
  Its costs: tests that set a field after construction (many in `mark.rs`
  and `filter.rs`) would carry a stale tier unless rewritten, and it edits
  `StoredEyes`'s fields and `new`, where all-faces-check Step 2 also works.
- **Item 2: what `get_view` carries.** The plan carries `photo_tier`
  (`"good"` or `null`), the item as written. An MCP client that wants the
  strip's three states (`good` / `neutral` / `low`, the names `set_filter`'s
  `face_focus` uses) cannot derive `low` from it. Carrying `face_focus`
  instead or as well would mean moving `af_eye_state` and
  `LOW_FULL_EYE_SHARP` to the engine too, which the item does not ask for.
- **Item 2: `null` is ambiguous.** A frame that is not good and a frame pass
  2 has not reached both read `null`. A client that needs the difference
  calls `wait_for_idle` first; the description should say so.
- **Item 2: `LOW_FULL_EYE_SHARP` stays a second literal** of
  `fullsharp::LOW_BELOW` in `mark.rs`, guarded as today by `fullsharp.rs`'s
  test. Pointing it at the core constant is not part of the item.
- **Item 1: the mark's place.** A fixed second slot at the bottom-left keeps
  both icons still but leaves a gap when the face icon is absent. Drawing
  the mark in the face icon's slot when that is empty is tighter but makes
  the mark's position depend on the other icon. The corners are otherwise
  taken; the step verifies the minimum strip height.
- **Item 1: a mark of one face.** Until all-faces-check Step 3 widens the
  rule, the mark says nothing about a bystander's blink, and nothing for a
  face under 60 px or a file whose cue ran no mesh. The docs must say so,
  and all-faces-check Step 3 must update those sentences.
- **Item 1: merge order with all-faces-check Step 3.** That step describes
  adding the mark itself. If it lands first, this plan's Step 2 shrinks to
  nothing but the todo removal; if this plan's Step 2 lands first (the
  expected order), that step's "strip cell mark" becomes a change to the one
  rule function. Whoever goes second should re-read the other's merged
  code.
- **Item 1: false marks.** The EAR cut has precision 0.94 and recall 0.86 on
  the judged face; a profile or a downcast look reads as closed, as the
  usage guide already says for the `Eyes` row. The mark inherits that.
- **Item 3: a photo with no face stays absent, not `null`, while `f` is
  off.** Before this change `null` appeared only after the on-demand
  judgment, so nothing is lost, but a client cannot tell "no face" from "not
  scanned yet" from the stored values alone. Telling them apart needs
  `faces_extractor` exposed on `IndexedFile` and kept current in the view's
  `Facts` (no schema change, since the column exists, but a new field on a
  serialized struct); not planned.
- **Item 3: the stored and the on-demand values can differ.** They come
  from two runs of the same model on the same face choice, but a file
  changed on disk between them, or a `FACES_VERSION` difference, can make
  them disagree. The stored values win, matching the meta pane, the filter
  and the strip mark.
- **Conflicts between this plan's own steps.** Steps 1 and 3 both edit
  `get_view`, its tests and the `get_view` description in `mcp.rs`; Steps 1
  and 2 both edit `mark.rs`. The order (1, 2, 3, 4) assumes each is merged
  before the next starts.

## Progress

- (2026-11-23) Step 1 complete
- (2026-11-23) Step 2 complete
