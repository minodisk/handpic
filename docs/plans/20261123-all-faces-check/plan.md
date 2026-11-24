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

# All-faces check for group photos

## Purpose

Pass 2 judges one face per frame: the one nearest the AF point, else the
largest confident face. In a group photo that says nothing about everybody
else, so a frame where someone other than the judged face blinked, or is
clearly soft, looks the same as a clean one. This work measures every
confident face that is large enough on the frames that look like a group
photo, stores per-frame counts (faces judged, faces with closed eyes, faces
with soft eyes) in the index, and shows them in the filter, on the strip
cell and in the meta pane, so the user can find the frames where anyone's
eyes are closed or soft.

It is a group-photo feature only, in two parts:

- Pass 2 stores, for every frame, a cheap count of the confident faces of a
  size similar to the judged face over the whole preview (the gate count).
- A new pass 3, after pass 2, runs the all-faces measurement only on the
  frames whose gate count is at least N (a user setting, default 5).

A frame under the gate is never touched by pass 3 and its counts are
unknown.

The single judged-face behavior (`eyes_ear`, the pose columns, `eye_offset`,
`edge_gap`, `full_eye_sharp`, `sharpness`, `photo_tier`, `af_eye_state`) does
not change.

## Steps

- [x] Step 1: The gate count in pass 2 and the all-faces measurement as its own entry point (`handpic-core`)
  - State: the first version is committed on the branch
    `all-faces-check-step-1` as `7ef67c07` (no PR yet). It runs the
    640x448 whole-image detection and the other faces' measurement inside
    pass 2's analysis on every AF frame, which about doubled pass 2 (82 to
    163-184 ms per AF file on one thread, see
    [measurement.md](./measurement.md)). The rework below is added on top of
    that commit on the same branch; the commit stays. One PR for the step.
  - Done when:
    - Pass 2's analysis (`scan::extract_analysis`, `scan::Analysis`) carries
      the gate count next to the existing cue and no longer carries the
      face counts. It runs no 640x448 whole-image detection on a frame with
      an AF point, meshes no other face, and reads the full JPEG for the
      judged face only, as before this work; it takes no threshold
    - On a frame with a trusted AF point and a judged face, the gate count
      comes from one whole-preview YuNet run at a low-resolution input
      whose size this step picks and names as a constant. A frame whose cue
      judged no face has a gate count of 0 and runs no such detection
    - A frame without an AF point takes its gate count from the 640x448
      whole-image detection it already runs, by the same rule, at no extra
      cost
    - A separate core entry point does the all-faces measurement of one
      file and returns its counts (judged, closed, soft): it reads the
      preview, runs the 640x448 whole-image detection, meshes the
      candidates, and reads the full JPEG once for every eye of the frame.
      It takes no threshold either (the caller decides which files it runs
      on). Next to it, a pool runner over a list of paths with the same
      contract as `extract_analysis_all`: the priority, the `ScanFocus` hot
      list, reads through `ReadSlots`, `yield_to_pause` at its stage
      boundaries, a canceled file abandoned and not delivered
    - The counts of the new entry point are the ones the first version
      computed for the same file (checked on the measured folders)
    - The existing `Analysis` values (`cue`, `sharpness`, `full_eye_sharp`)
      are the same as before on every frame; the existing tests pass
    - Unit tests cover the pure parts: the gate count (the score floor, the
      size ratio at both boundaries, the judged face counted once whether
      or not the low-resolution detection found it, no judged face giving
      0); pass 2's analysis running no 640x448 detection and no other mesh
      on an AF frame; the new entry point (candidates, the cap, the closed
      and soft boundaries, soft unknown without a larger JPEG, one full
      JPEG read, a canceled file abandoned); the pool runner delivering
      every index once and stopping on cancel
    - The low-resolution input was chosen from a measurement recorded in
      [measurement.md](./measurement.md): for each size tried (start from
      320x224), its cost per AF file on one thread and its gate count per
      frame against the count the 640x448 detection gives by the same rule,
      on the measured AF folders; and, for the chosen size, how many frames
      reach N = 5 (and 3 and 4 for comparison) and whether they are the
      group photos
    - The times are measured again (one thread, the fastest of three, the
      same sets as before) and recorded in `docs/humans/performance.md` and
      `docs/humans/performance.ja.md`, whose "All-faces check" section is
      rewritten: pass 2 per file (expected: the cost before this work plus
      the low-resolution run, roughly 15 to 20 ms, not measured yet, less
      what the one-pass eye crops save) and the all-faces entry point per
      file
    - `CLAUDE.md`'s layout description names the gate count and the entry
      point
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/core/src/allfaces.rs` (the gate count, its constants),
      `crates/core/src/faces.rs` (the low-resolution whole-image input next
      to `WHOLE_INPUT` and the `Input` variants), `crates/core/src/scan.rs`
      (`Analysis`, `extract_analysis_unless`, the new entry point and its
      runner, the first version's tests moved to the entry point),
      `crates/cli/src/main.rs` and `crates/cli/src/facechoice.rs` (printing
      the gate count and the entry point's counts and time; the step picks
      the subcommand), the two performance docs, `CLAUDE.md`,
      `measurement.md`, `learnings.md`. Read
      `docs/agents/tract-onnx-inference.md` first (a new input size is a
      new model plan).
    - What stays from the first version: `allfaces` (`candidates`,
      `measured`, `measure_unless`, `counts`, `MAX_FACES`, the
      `MAX_EYE_OFFSET` mesh-fit gate, which also turns down a mesh
      `eyes::mesh_trusted` distrusts or one under `EYES_MIN_PRESENCE`, as
      the cue does), the caller-owned preview decode
      (`focus_cue_rgb_unless`), the one-pass eye crops
      (`partial::decode_crops`, `fullsharp::full_eye_sharpnesses`, which
      pass 2 keeps for the judged face's two eyes), the 640x448 input of
      the full whole-image detection, the IoU match of the judged face.
    - What moves: `other_faces`, the `others` half of `full_sharps` and the
      `face_counts` assembly leave `extract_analysis_unless` for the entry
      point; `Analysis::face_counts` goes, so `extract_analysis`'s
      signature and the engine's call site are as on `main`.
    - The entry point needs the judged face. Its box is not in the index,
      and the counts take the judged face from the cue's own mesh (so
      `Eyes: Closed` implies a closed count of at least 1) and drop its
      whole-image twin by overlap. So the entry point recomputes the cue on
      its own preview decode, the way pass 2 does (`af_cue`, or
      `whole_image_cue_rgb` on the detection it runs anyway), then measures
      the judged face's eyes and the others' in the one full JPEG pass. It
      is self-contained: a path in, counts out.
    - `PASS_SLOTS` is set only inside `extract_analysis_unless` today; the
      entry point sets it the same way so `yield_to_pause` works in it.
    - The gate count, one rule for both paths but the score floor, over a
      whole-preview detection's faces in stored preview coordinates: the
      judged face (`Cue::face`) counts as one; every face of the detection
      that scores at least the floor (`sharpness::FACE_CONFIDENCE`, 0.8, on
      the 640x448 search of a frame without an AF point;
      `allfaces::GATE_LOW_RES_CONFIDENCE`, 0.6, on the 320x224 search of a
      frame with one, decided after the measurement), has a box long side
      within 0.5x to 2x of the judged face's, and does not overlap the
      judged face (IoU above `faces::NMS_THRESHOLD`, the rule `candidates`
      already uses) counts as one more. No judged face gives 0. A failed
      detection gives `None`.
    - The judged face on an AF frame comes from the crop detection, so the
      low-resolution detection may not hold it. It needs no match to count:
      the size reference is always the cue's own box, and the judged face is
      always one of the N. When the low-resolution run did find it, the
      overlap rule keeps it from counting twice; when it did not, nothing
      is dropped.
    - Why 0.5x to 2x: a face at half the linear size stands at about twice
      the distance, which is a passer-by or a spectator behind the group,
      not a member of it, while the rows of one group stay well inside a
      factor of two; the band is wide enough for YuNet's box noise and for
      children next to adults. Name both bounds as constants and check them
      against the dump (the background players of the team-sport folders
      are the faces the band should drop).
    - Verify during implementation: (1) how often the low-resolution gate
      count is under N where the 640x448 count is at or above it (a group
      frame the gate misses), per input size; (2) how many frames of the
      measured folders reach N = 5 at all (no measured frame had more than
      5 counted faces); (3) that the stored values still match
      (`facechoice --check`).
    - No index change in this step; `FACES_VERSION` stays.

- [x] Step 2: Store the gate count and the counts, and pass 3 in the engine (`handpic-engine`)
  - Done when:
    - `files` has four nullable integer columns (suggested names
      `faces_group` for the gate count, `faces_judged`, `faces_closed`,
      `faces_soft`) and a version column for pass 3 (suggested
      `allfaces_extractor INTEGER NOT NULL DEFAULT 0`, written at a new
      `ALLFACES_VERSION`); a v27 database gains them in place with `ALTER
      TABLE` and keeps `files`, `ratings` and `folders`; `SCHEMA_VERSION`
      is 28 and its doc comment says so
    - `FACES_VERSION` is 12 with its doc comment extended, so pass 2 re-runs
      on every row at the next open of each folder (this is what fills the
      gate count); `EXTRACTOR_VERSION` stays 13
    - Pass 2 stores the gate count (`write_faces`), and a row pass 2 writes
      or resets loses its counts and its pass 3 version, so pass 3 runs on
      it again; `indexed_file` reads the five values and `faces-progress`
      (`FaceReady`) carries the gate count
    - A pass 3 todo query (suggested `allfaces_todo(dir, min_faces)`) lists,
      in file-name order, the rows of the folder that pass 2 has finished
      at `FACES_VERSION`, whose gate count is at least `min_faces` and
      whose pass 3 version is not `ALLFACES_VERSION`
    - A pass 3 runner (suggested `run_allfaces_scan`) runs the core entry
      point over those paths the way `run_faces_scan` runs pass 2: batched
      writes, `progress(done, total, ready)` at most every interval with
      the rows a batch committed, the priority and the `ScanFocus` given by
      the caller, a file abandoned by `cancel` neither written nor counted,
      a file that could not be read counted as an error and stored with no
      counts at `ALLFACES_VERSION` so it is not retried
    - `get_photo` reports the gate count and the three counts as stored,
      and its tool description says what they are and that the counts
      exist only on a frame whose gate count reached the group threshold
    - Tests: the values round-trip; the v27 migration test; the existing
      migration tests updated for the new columns; pass 2 storing the gate
      count and clearing a row's counts and pass 3 version; the todo query
      (under the threshold not listed, at or above it listed, already done
      not listed, done under a higher threshold and now over a lower one
      listed, not yet through pass 2 not listed, an error row and a JPEG
      row never listed); a pass 3 runner test showing a ready row carries
      its counts and a canceled run leaves its rows for the next
    - `CLAUDE.md`'s `src/index.rs` description names the columns and the
      pass
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged.
    - Files: `crates/engine/src/index.rs` (`SCHEMA_VERSION`, `FACES_VERSION`,
      `ALLFACES_VERSION`, the `CREATE TABLE`, the migration chain,
      `INDEXED_FILE` / `indexed_file`, `faces_todo`'s reset, `write_faces`,
      `FacesRow`, `FaceReady`, `StoredEyes`, the new todo query, write and
      runner, the migration tests' `DROP COLUMN` lists),
      `crates/engine/src/mcp.rs` (the `get_photo` description).
    - Put the gate count and the three counts in `StoredEyes`: it is
      flattened into both `Focus` and `FaceReady`, so `get_photo`'s JSON
      and the app's `Facts` get them with no further plumbing, and a pass 2
      `FaceReady` (counts unknown) replacing a row's `StoredEyes` whole in
      `filter::apply_face_ready` is the same clearing the index does.
      `StoredEyes` is built by name in a few app tests
      (`crates/app/src/filter.rs`), which this step must keep compiling.
    - Pass 3's ready rows need their own small type (the path and the three
      counts), since they update only those fields.
    - The stored counts are raw: nothing in the index hides them when the
      threshold later rises; that is the app's rule (Step 4).
    - `NULL` gate count: pass 2 has not run, or its detection failed. `NULL`
      counts: pass 3 has not run on the row, or failed on it. A frame pass
      3 ran on with no candidate face stores its judged count (0 or 1),
      not `NULL`.
    - Model the runner on `run_faces_scan` (its `flush`, its progress
      throttle, its no-panic rule for `on_item`); do not generalize the two
      into one unless it falls out with no change to pass 2's behavior.
    - No caller in this step: the app starts pass 3 in Step 3. The app
      crate builds unchanged (new `StoredEyes` fields aside).

- [x] Step 3: Run pass 3 in the app and show it in the status line (`handpic-app`)
  - Done when:
    - The scan task runs pass 3 after pass 2 on the same thread, on the
      rows the todo query gives for the default threshold (5, a named
      constant until Step 4), its workers at a priority no higher than pass
      2's, taking the on-screen files first through the same `ScanFocus`,
      stopped by the same cancel (a re-listing, the quit handler's
      `STOP_TIMEOUT`)
    - Its progress reaches the view through the scan's one channel as its
      own events (suggested `allfaces-progress` / `allfaces-done`
      counterparts of the pass 2 messages); the view applies each ready
      row's counts to the file's `Facts` the way it applies a pass 2 row,
      re-reads the shown file's meta and overlay data where it does for
      pass 2, and refilters
    - The status line shows the pass while it runs (its own name, done /
      total), the progress circle follows it, `get_status` reports it as
      the scan pass, and `wait_for_idle` does not answer while it runs
    - A folder with no row at the gate starts no pass 3 work and shows no
      third pass
    - Tests: `Status` with the third pass (its text, `running`, `scanning`,
      cleared by a re-listing), `companion.rs`'s `get_status` case, and a
      UI or scan-level test that pass 3 runs after pass 2 and its rows
      reach the view
    - The docs that describe the scan's passes (`README.md` /
      `README.ja.md`, `docs/humans/usage.md` / `docs/humans/usage.ja.md`,
      wherever they name the two passes or the status line) describe the
      third; `CLAUDE.md`'s `scan.rs`, `companion.rs` and `progress.rs`
      descriptions are updated
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged.
    - Files: `crates/app/src/scan.rs` (`Scan::start`, a `run_allfaces_pass`
      next to `run_faces_pass`, `Event`, `Status`), `crates/app/src/main.rs`
      (the event handling next to the pass 2 messages),
      `crates/app/src/companion.rs` (`Status`), `crates/app/src/filter.rs`
      (applying a pass 3 row to `Facts`, next to `apply_face_ready`),
      `crates/app/src/progress.rs` only if `shown` does not already follow
      `Status::running`.
    - Pass 2 already runs at `Priority::Lowest`. Whether each platform has a
      level below it is for the step to check (`lower_current_priority`
      takes a `lowest` flag today); if there is none, pass 3 runs at the
      same level and comes after pass 2 only by order.
    - Pass 3 starts when pass 2 has finished the whole folder, not per
      file.
    - `hold.rs`'s `PaintHold` and the read slots need no change: pass 3's
      reads go through the same `ReadSlots` inside the core runner.

- [x] Step 4: The group threshold setting and the rescan trigger (`handpic-app`, `handpic-engine`)
  - Done when:
    - The settings store has a new key (suggested `groupMinFaces`, an
      integer from 2 to 20, default 5; a missing or invalid value is the
      default), read at startup and written through `Writes` like the other
      settings
    - The settings modal's Culling page shows it as a number input under
      the auto-advance switch, applied when committed; a value outside 2 to
      20 is not committed
    - `get_settings` reports it and `set_settings` accepts it (suggested
      `group_min_faces`) under the modal's own rules: a value outside 2 to
      20 or a non-integer is refused, and it is refused while a folder
      operation runs like every setting; the tool descriptions name it
    - Pass 3 takes its threshold from the setting
    - Raising the value starts no scan. Lowering it makes pass 3 run on the
      open folder's rows that now qualify, with no pass 2 re-run; every
      other folder gets the same at its next open, with no global rebuild
    - One pure rule gives a frame's counts as the app shows them: the
      stored counts when the stored gate count is at or above the setting,
      else unknown. Steps 5 and 6 read the counts only through it
    - Unit tests: the setting's parse, default and range, the settings
      modal model's change, `companion.rs`'s `get_settings` /
      `set_settings` cases (a valid value, 1, 21, a non-integer), the
      visibility rule at the boundary, and a UI or scan-level test that a
      lowered threshold runs pass 3 on exactly the rows that now meet it
      and leaves pass 2 alone
    - `README.md` / `README.ja.md` and `docs/humans/usage.md` /
      `docs/humans/usage.ja.md` describe the setting; `CLAUDE.md`'s
      `settings.rs`, `settingsview.rs`, `scan.rs` and `companion.rs`
      descriptions are updated
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 3 is merged.
    - Files: `crates/app/src/settings.rs` (the field and its read),
      `crates/app/src/settingsview.rs` (a `Change` variant, the Culling
      page's item), `crates/app/src/settingsmodal.rs` if the page model
      needs it, `crates/app/src/main.rs` (`apply_setting`, the scan start),
      `crates/app/src/scan.rs` (the threshold handed to pass 3),
      `crates/app/src/companion.rs` (`get_settings`, `set_settings`'s
      changes), `crates/engine/src/mcp.rs` (`SettingsArgs`, the `switches`
      list, the descriptions), `crates/app/src/mark.rs` or a small new
      module (the visibility rule; the step decides).
    - Follow `autoAdvance` end to end: `Settings::auto_advance`,
      `Change::AutoAdvance`, `apply_setting`'s arm,
      `settings.writes.set(..)`, `companion.rs`'s `switch(args,
      "auto_advance")`, `SettingsArgs::auto_advance`.
    - The number input: gpui-component 0.9.1 ships `NumberInput`
      (`input/number_input.rs`) and a settings number field
      (`setting/fields/number.rs`); prefer the settings field, as the page
      already uses `SettingField::switch`. The page's text fields let their
      keys through by the `Input` key context; check the number field does
      too.
    - The trigger on lowering: the simplest is the view's existing
      re-listing (`relist` / `Resync`, the path a folder watcher change
      takes): passes 1 and 2 find nothing to do and pass 3 picks up the
      rows. It waits for a listing or scan under way, keeps the current
      file and the strip in place, and starts nothing under the gate's
      in-flight count. The step may instead start pass 3 alone if that
      turns out simpler than it looks; it decides.
    - A scan already running when the value is lowered: either pass 3 reads
      the value when it starts (so a change during passes 1 and 2 is in
      force), or the re-listing after it picks up the rest. The step
      decides.
    - On any change of the value, `refilter` and redraw, since the
      visibility rule's answer changes for rows already loaded.

- [x] Step 5: Show the counts: the meta pane's rows and the strip cell's mark (`handpic-app`)
  - Done when:
    - The meta pane's `Analysis` group shows a `Similar faces` row, the
      stored gate count, whenever it is known, on a frame under the gate
      too (it is what the user reads to choose N)
    - The `Analysis` group shows the three counts for the shown file
      (suggested rows: `Faces judged`, `Faces with closed eyes`, `Faces
      with soft eyes`), each left out while its value is unknown, and all
      three left out on a frame under the gate
    - A strip cell draws a mark when at least one face of the frame has
      closed eyes, updated live as pass 3's progress arrives and when the
      setting changes; never on a frame under the gate
    - Unit tests: the meta rows (values, the unknown cases, a frame under
      the gate showing `Similar faces` alone), the pure "anyone closed"
      rule the cell reads
    - `README.md` / `README.ja.md` and `docs/humans/usage.md` /
      `docs/humans/usage.ja.md` describe the rows and the mark, including
      the gate, the size floor and the cap; `CLAUDE.md`'s `mark.rs` /
      `meta.rs` / strip descriptions are updated
    - The manual check on a real group photo folder (the mark and the rows
      agree with what the photos show) is listed as a pending manual check
      in the PR body
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 4 is merged.
    - Files: `crates/app/src/meta.rs` (`analysis`, which already reads
      `focus.eyes`), `crates/app/src/mark.rs` (the pure rule),
      `crates/app/src/main.rs` (`StripCell`, the cell painting next to
      `paint_face_icon` / `paint_burst_badge`), `crates/app/src/icons.rs` if
      an icon is added.
    - Read the three counts through Step 4's visibility rule, never from
      `StoredEyes` directly; `Similar faces` is the stored gate count as it
      is.
    - The cell's corners in use: the face focus icon at the bottom-left, the
      burst badge at the bottom-right, the judgment `Marks` elsewhere. The
      step picks a free place and paints with the same halo
      (`overlay::HALO`). Prefer an unmodified Lucide catalog icon through
      `icon_assets!` (e.g. `eye-off` / `eye-closed`, whichever the vendored
      `gpui-kit-assets` holds) over a new drawing.
    - The existing face focus icon and its color (`af_eye_state`) do not
      change.

- [x] Step 6: Filter by anyone's eyes, in the menu and over MCP (`handpic-app`, `handpic-engine`)
  - Done when:
    - The filter menu's `Eyes` section has an `Anyone closed` item and an
      `Anyone soft` item that pass a frame whose closed (soft) count is at
      least 1; they are OR-ed with the section's other checked items like
      every item of a section; a frame under the gate matches neither
    - The items round-trip through the stored `filter` setting, `get_view`'s
      `filter` reports them, and `set_filter` accepts them; the tool
      descriptions name them
    - Unit tests: the predicate (count 0, at least 1, unknown, under the
      gate; OR with `Open` / `Closed` / `Unknown`; AND with the other
      sections), the stored-setting round trip, the menu rows,
      `companion.rs`'s `set_filter` / `get_view` cases; the coverage test
      still passes
    - `README.md` / `README.ja.md` and `docs/humans/usage.md` /
      `docs/humans/usage.ja.md` describe the items; `CLAUDE.md`'s
      `filter.rs` / `menu.rs` descriptions are updated
    - The manual check (the filter narrows a real group photo folder to the
      frames with a blink) is listed as a pending manual check in the PR body
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 4 is merged (independent of Step 5).
    - Files: `crates/app/src/filter.rs` (`EyesItem`, `EYES`, `Stored`'s
      parse, `passes`), `crates/app/src/menu.rs` (`filter_rows`),
      `crates/app/src/main.rs` (the view's `passes`),
      `crates/app/src/companion.rs` (tests; `filter_of`, `items` and the
      menu-click diff already go through `filter::EYES` and
      `Toggle::Eyes(_)`), `crates/engine/src/mcp.rs` (its own `EyesItem`
      enum for `set_filter`'s schema and the descriptions).
    - `passes` today maps the judged face's state to exactly one `EyesItem`
      (`EyesItem::of`), a partition. `Anyone closed` is not part of that
      partition (a frame can be `Open` and `Anyone closed`), so the section's
      test becomes "any checked item matches" and `passes` needs the counts
      as Step 4's visibility rule gives them (or the two booleans) as input.
    - `filter_offers` needs no change: the items are static, like the other
      `Eyes` items.

## Decisions

Made at plan approval (2026-11-23), each the recommended option:

- Faces under `EYES_MIN_FACE` are excluded, not meshed from the full JPEG
  (Option A below); full-size meshing is a follow-up.
- The mesh-fit gate for the other faces is added only if Step 1's
  measurement shows false closed readings. It did, and the gate is in
  (`allfaces::MAX_EYE_OFFSET`, see [measurement.md](./measurement.md)).
- The soft threshold is `LOW_BELOW`.
- `Anyone soft` is included in the filter step.
- The release PR is held until the last step is merged.

Made by the user on 2026-11-24, after `main` added the face mesh presence
rules (`eyes::mesh_trusted`, `EYES_MIN_PRESENCE`):

- The all-faces counts apply the same rules as the face cue: a candidate
  whose mesh is untrusted (a side face under `SIDE_FACE_PRESENCE`) or whose
  presence is under `EYES_MIN_PRESENCE` is neither judged nor closed, and its
  eyes are not measured for soft (`allfaces::measured`). The judged face
  already follows them through the cue's EAR.

Made by the user on 2026-11-24, after Step 1's measurement:

- The first version's cost is not acceptable: pass 2 on AF ARW frames went
  from about 82 ms to 163-184 ms on one thread, mostly the added
  whole-image YuNet run (about 71 ms), on every AF frame including
  single-subject ones.
- The gate cannot come from the camera: the Sony α7 V maker notes carry no
  face count (checked with ExifTool: no ShotInfo 0x3000 / FacesDetected), so
  it comes from our own detection.
- The all-faces check is a group-photo feature only, behind a gate: at
  least N confident faces of a size similar to the judged face. Under the
  gate the counts are unknown, so `Anyone closed` / `Anyone soft` never
  match the frame and its count rows are hidden.
- The gate's score floor (after Step 1's rework measurement): 0.6
  (`allfaces::GATE_LOW_RES_CONFIDENCE`) on the AF path's 320x224 run only,
  where the small faces score 0.6 to 0.8 and 0.8 counted about half the
  faces the 640x448 search counts; the path without an AF point keeps the
  640x448 search at 0.8 (`FACE_CONFIDENCE`).
- The gate counts over the whole preview on both paths. On an AF frame the
  count comes from a cheap low-resolution YuNet run over the whole preview
  (e.g. 320x224; Step 1 measures the cost and picks the input size). A
  frame without an AF point counts over the whole-image detection it
  already runs. Counting inside the AF point's `CATCH_CROP` square was
  considered and dropped: 480 px of a 1616 px preview cannot hold five
  faces of any size.
- The all-faces measurement is a new pass 3, not part of pass 2. Pass 2
  keeps only the gate count. Pass 3 runs after pass 2 on the same scan
  task, at a priority no higher than pass 2's, through `ReadSlots`,
  `yield_to_pause` and the cancel like pass 2, only on the rows whose
  stored gate count is at least N and that have no counts: it reads the
  preview and the full JPEG again, runs the 640x448 whole-image detection,
  meshes and measures the candidates, stores the counts and streams its own
  progress events, and the status line shows it.
- N is a user setting, default 5, in the settings store, shown on the
  settings modal's Culling page as a number input and exposed by MCP
  `get_settings` / `set_settings`. Its range is 2 to 20.
- Changing N does not rebuild every folder. The gate count is stored per
  frame, always. Raising N hides the counts of the frames under the new N
  when they are shown or filtered, with no rescan. Lowering N makes pass 3
  pick up the rows that now qualify; pass 2 does not re-run.
- "Similar size" is a box long side within 0.5x to 2x of the judged face's,
  and the judged face counts as one of the N.
- `get_photo` reports the raw stored counts with the gate count beside
  them, not hidden under the current N.
- On a frame that passes the gate, the faces measured are not restricted to
  the size band: the candidates stay the first version's (score, the 60 px
  floor, the cap).
- The meta pane shows the gate count as a `Similar faces` row, so the user
  can choose N.

Made in this revision (planning, 2026-11-24):

- Pass 3 has its own version column (`allfaces_extractor` at
  `ALLFACES_VERSION`). This replaces the earlier rule "a row is to do when
  its counts are `NULL`, no extra column", which was chosen while the
  measurement was still inside pass 2: with a pass of its own, a row pass 3
  failed on must be markable as done (as pass 2 marks its failures), and
  the all-faces rule can then change without re-running pass 2.
- Pass 3's core entry point recomputes the judged face's cue rather than
  taking it from the index, because the judged face's box is not stored.
- The setting and the rescan trigger are one step, after a step in which
  the app runs pass 3 at the default.

## Trade-offs and risks

- **A group frame is read and decoded twice.** Pass 3 reads the preview
  again, decodes it again, recomputes the judged face's cue (the crop
  detection and its mesh) and reads the full JPEG a second time, on top of
  the 640x448 detection and the other faces' meshes. That is roughly pass
  2's whole cost again plus the first version's added cost, per frame at
  the gate, and nothing on every other frame. Storing the judged face's box
  and mesh values in pass 2 would spare the recomputation at the price of
  more columns; not chosen. Step 1 measures the entry point's time.
- **The counts arrive after pass 2.** Pass 3 starts when pass 2 has
  finished the whole folder, so on a large folder the strip mark, the count
  rows and the `Anyone closed` / `Anyone soft` filter fill in late, and a
  filter applied during the scan shows only the frames pass 3 has reached.
  The status line names the pass, and `wait_for_idle` waits for it. The
  on-screen files go first through `ScanFocus`.
- **Pass 3's priority may equal pass 2's.** Pass 2 already uses the lowest
  level the code names; where a platform has nothing below it, pass 3 is
  "after" pass 2 only by order (Step 3 checks).
- **The low-resolution count misses faces, which matters near the
  threshold.** At a 480x320 input the whole-image detection lost 23 to 32%
  of the counted faces of the measured folders (their scores fell under
  0.8), and a 320x224 input loses more. For the gate this only matters when
  the true count is close to N: a group of exactly five where one face goes
  uncounted is a frame pass 3 silently skips, while a single-subject frame
  is unaffected. Step 1 measures, per input size, how often the
  low-resolution count is under N where the 640x448 count is not, and picks
  the size against its cost. What is left after that is the user's lever: a
  lower N. (Measured: at 0.8 320x224 counted about half the faces; with the
  0.6 floor decided for that run it counts about as many as 640x448 at 0.8,
  and misses 20 of the 49 frames 640x448 puts at N = 5, adding others of
  its own.)
- **The gate count is slightly stricter on an AF frame.** An AF frame
  counts on the low-resolution detection and a frame without an AF point on
  the 640x448 one, so the same group can count one or two faces fewer on a
  body with an AF point. Running the low-resolution detection on the no-AF
  frames too would make them equal, at an added cost on frames that already
  have a better detection; not chosen.
- **Pass 2 does not return to its old cost.** Every AF frame with a judged
  face pays the low-resolution run (expected roughly 15 to 20 ms on one
  thread, unmeasured). Step 1 measures it.
- **The default N = 5 may still pass few frames.** In the five measured
  folders no frame had more than 5 counted faces, by the stricter count
  that also needs a fitted mesh. Step 1 reports how many frames reach 3, 4
  and 5; the default is the user's decision and the meta pane's `Similar
  faces` row is what lets them tune it.
- **The size band and the low-resolution input are baked into the stored
  gate count.** Changing either later means a `FACES_VERSION` bump and a
  pass 2 re-run of every folder. The all-faces rule itself (the mesh-fit
  gate, the cap, the soft threshold) can change with an `ALLFACES_VERSION`
  bump alone, which re-runs pass 3 only on the frames at the gate.
- **A passing frame still counts a confident background face of another
  size**, since the candidates are not restricted to the band.
- **`get_photo`'s counts are not hidden under the current N.** A client
  must compare the gate count with `group_min_faces` itself; the tool
  description says so.
- **Lowering N while a scan runs** takes effect at pass 3's start or after
  that scan, whichever Step 4 implements.
- **Small faces: exclude under a floor (chosen) or mesh them from the
  full JPEG.** The embedded preview is about 1616 px wide on Sony bodies, so
  faces in a large group are often 30 to 60 px. `EYES_MIN_FACE` (60 px) is
  the floor the closed-eyes judgment was calibrated with: under it a human
  can call the eye state of about one face in five on the preview.
  - Option A, exclude faces under `EYES_MIN_FACE`: the EAR threshold stays
    the calibrated one, the judged face's existing mesh is reused so the
    single-face values and the counts never disagree, and the cost is one
    preview-sized mesh per face. Consequence: on the measured ARW team-sport
    folders 24 to 48% of the confident faces go unjudged; `Faces judged` in
    the meta pane is what tells the user how much was covered.
  - Option B, crop small faces from the full JPEG for the mesh: covers faces
    down to what YuNet finds at all. Consequences: `EYES_CLOSED_EAR` was
    fitted on preview-resolution crops and would need new labeled faces to
    trust on full-size crops; the judged face (preview mesh) and the counts
    (full-size mesh) could disagree about the same face; one more partial
    decode of the full JPEG per face; no effect on files without a larger
    JPEG. With pass 3 reading the full JPEG anyway, this follow-up is
    cheaper to add than it was inside pass 2.
- **False "closed" readings multiply with the number of faces.** The
  mesh-fit gate (`eye_offset` at most 0.1) is in for the other faces; yaw
  did not separate the false readings, so there is no yaw limit. A few
  heavily blurred background faces still read closed. The judged face
  always counts, to stay consistent with the `Eyes` row.
- **The judged face can be counted twice**, in the gate count and in the
  judged count alike, when its whole-image box overlaps the crop
  detection's at or under `NMS_THRESHOLD` IoU (1 of 258 AF frames on the
  measured folder at 640x448; a low-resolution box may differ more). Listed
  in `learnings.md` as a deferred issue.
- **The cap undercounts.** With more candidate faces than the cap, only the
  largest are judged, and `Faces judged` reads the cap. It never bound on
  the measured folders.
- **Soft threshold.** `LOW_BELOW` (0.110) over `SOFT_BELOW` (0.162), which
  depth of field would trip on most group frames. Even `LOW_BELOW` flags
  back-row faces (50 of 282 and 96 of 300 frames on the team-sport
  folders, before the gate); the manual check should look at this.
- **`Anyone soft` sits in the `Eyes` section** although softness is the
  `Face focus` section's subject, because `Face focus` is a partition by
  `MarkState`.
- **Which face is the closed one is not shown.** The `f` overlay still
  outlines only the judged face's mesh (on-demand, not from the index).
  Marking each face on the overlay is not in this plan.
- **Steps 1 to 4 merged alone show nothing but a third pass in the status
  line**, and Step 2's `FACES_VERSION` bump re-runs pass 2 on every folder
  at its next open. The release PR is held until Step 6.
- **A blink-free mark is not a guarantee**: frames under the gate, faces
  under the floor, past the cap, below the 0.8 score or missed by YuNet are
  not judged. The docs should say so.

## Progress

- (2026-11-23) Step 1's first version committed on `all-faces-check-step-1`
  (`7ef67c07`), no PR; measurement in [measurement.md](./measurement.md)
- (2026-11-24) Plan revised after the measurement: the group gate (a
  low-resolution whole-preview count in pass 2), the all-faces measurement
  as a new pass 3, the threshold setting and the rescan trigger; Step 1
  reopened for the rework
- (2026-11-24) Step 1 complete
- (2026-11-24) Step 2 complete
- (2026-11-24) Step 3 complete
- (2026-11-25) Step 4 complete
- (2026-11-25) Step 5 complete
