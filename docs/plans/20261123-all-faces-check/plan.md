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
clearly soft, looks the same as a clean one. This work makes pass 2 measure
every confident face that is large enough, stores per-frame counts (faces
judged, faces with closed eyes, faces with soft eyes) in the index, and shows
them in the filter, on the strip cell and in the meta pane, so the user can
find the frames where anyone's eyes are closed or soft.

The single judged-face behavior (`eyes_ear`, the pose columns, `eye_offset`,
`edge_gap`, `full_eye_sharp`, `sharpness`, `photo_tier`, `af_eye_state`) does
not change.

## Steps

- [x] Step 1: Measure every confident face in pass 2 (`handpic-core`)
  - Done when:
    - `scan::Analysis` carries per-frame face counts (judged, closed, soft)
      next to the existing cue, and `extract_analysis` fills them from the one
      preview read and the one full JPEG read pass 2 does per file (never a
      second full read)
    - The existing `Analysis` values (`cue`, `sharpness`, `full_eye_sharp`) are
      the same as before on every frame; the existing tests pass unchanged
    - Unit tests cover the pure parts: which faces are candidates (score
      floor, size floor, the judged face not counted twice), the cap and its
      order, the closed count at `EYES_CLOSED_EAR`'s boundary, the soft count
      at its threshold's boundary, soft being unknown without a larger JPEG,
      and a canceled file being abandoned
    - The thresholds and the cap were checked on a real group photo folder
      with `handpic-cli facechoice` (its `whole` set already dumps every
      whole-image face with its box, score, EAR, `mesh_ms`, `eye_offset`,
      pose and `full_sharp`), and the result is recorded in a file in this
      folder
    - The pass 2 time before and after is measured (`handpic-cli features`
      prints the `extract_analysis` ms per file) on a single-subject folder
      and on the group folder, and recorded in `docs/humans/performance.md`
      and `docs/humans/performance.ja.md`
    - `CLAUDE.md`'s layout description names the new items
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/core/src/candidate.rs` (or a new module beside it; the
      step decides), `crates/core/src/scan.rs` (`Analysis`,
      `extract_analysis_unless`, `full_eye_sharp`), `crates/core/src/fullsharp.rs`
      only if a per-face helper is needed, the two performance docs, `CLAUDE.md`.
      Read `docs/agents/tract-onnx-inference.md` first.
    - Where the faces come from. With a trusted AF point the cue detects only
      inside a `CATCH_CROP` (480 px) square around the point, so the other
      faces of a group are never seen: an AF frame needs a whole-image
      detection as well. Use the same path a frame without an AF point
      already takes (`scan.rs`'s `whole_image_detection`, that is
      `faces::decode_whole` + `detect_whole_upright` at `WHOLE_INPUT`), whose
      recall is the one `performance.md` measured. A frame without an AF
      point already has that detection; reuse it, do not run it twice.
    - Candidate faces: score at least `sharpness::FACE_CONFIDENCE` (0.8, the
      existing meaning of "confident" in `eyes::judged_face`), box long side
      at least `eyes::EYES_MIN_FACE` (60 preview px, through
      `candidate::meshes_face`), largest first, at most a cap per frame
      (start from 8; the step fixes the number from the measurement and
      names it as a constant).
    - The judged face is counted from the cue's own mesh, not meshed again:
      when the cue has an EAR, that face is one of the judged faces, so the
      meta pane's `Eyes: Closed` always implies a closed count of at least 1.
      The whole-image face that overlaps the judged face (the two detections
      give different boxes for the same face; match by overlap, e.g. IoU
      above `faces::NMS_THRESHOLD`) is dropped from the candidates.
    - Per other face: `eyes::mesh_of` on the preview's full-size stored RGB
      (the cue already decodes it with `decode_rgb`; share that decode rather
      than decoding the preview again, which needs `focus_cue_unless` /
      `whole_image_cue` to hand the pixels on or the caller to own the
      decode), `eyes::more_closed_ear`, closed when `Eyes::from_ear` says
      `Closed`; then `fullsharp::full_eye_sharpness` on the full JPEG with
      the mesh's eye centers (`candidate::mesh_eye_centers`).
    - Soft threshold: `fullsharp::LOW_BELOW` (0.110, "clearly soft", the
      `Low` mark's cut), not `SOFT_BELOW`: people off the focus plane in a
      group are normally under 0.162, so `SOFT_BELOW` would flag most group
      frames. The soft count is unknown (`None`), not 0, when the file has no
      larger JPEG (`has_larger_jpeg`) or the read fails.
    - The full JPEG read: today it happens only when the cue has eye centers.
      It now happens when any face is to be measured, still once per file,
      through `slots` as before. Keep `yield_to_pause` and the cancel check
      between faces (one mesh is about 29 ms on one thread), and keep the
      `catch_unwind` rule: a failure or panic on one face leaves that face
      out, it does not fail the file.
    - Verify during implementation: (1) how many faces of the real group
      folder fall under `EYES_MIN_FACE` and so go unjudged; (2) how many
      other-face "closed" readings are false (profile faces, a mesh fitted
      off the face) and whether the other faces need the mesh-fit gate
      (`eye_offset` at most 0.1) or a yaw limit before they count, see
      "Trade-offs and risks"; (3) the per-face cost of the two
      `decode_crop`s on a large full JPEG.
    - No index change in this step; the counts are not stored yet, so
      `FACES_VERSION` stays (the stored values do not change).

- [ ] Step 2: Store the counts in the index (`handpic-engine`)
  - Done when:
    - `files` has three nullable integer columns (suggested names
      `faces_judged`, `faces_closed`, `faces_soft`); a v27 database gains
      them in place with an `ALTER TABLE` and keeps `files`, `ratings` and
      `folders`; `SCHEMA_VERSION` is 28 and its doc comment says so
    - `FACES_VERSION` is 12 with its doc comment extended, so pass 2 re-runs
      on every row at the next open of each folder; `EXTRACTOR_VERSION` stays
      13 (pass 1's output does not change)
    - `write_faces` stores the counts, `faces_todo`'s reset clears them,
      `indexed_file` reads them, and `faces-progress` (`FaceReady`) carries
      them
    - `get_photo` reports the counts, and its tool description says what
      they are
    - Tests: the counts round-trip through `write_faces` / `entry`, the v27
      migration test, the existing migration tests updated for the new
      columns, a `run_faces_scan` test showing a ready row carries them
    - `CLAUDE.md`'s `src/index.rs` description names the columns
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged.
    - Files: `crates/engine/src/index.rs` (`SCHEMA_VERSION`, `FACES_VERSION`,
      the `CREATE TABLE`, the migration chain, `INDEXED_FILE` /
      `indexed_file`, `faces_todo`, `write_faces`, `FacesRow`, `FaceReady`,
      `StoredEyes`, `run_faces_scan`'s `flush`, the migration tests' `DROP
      COLUMN` lists), `crates/engine/src/mcp.rs` (the `get_photo`
      description).
    - Put the counts in `StoredEyes`: it is flattened into both `Focus` and
      `FaceReady`, so `get_photo`'s JSON, the app's `Facts` and
      `filter::apply_face_ready` (which copies `focus.eyes` whole) get them
      with no further plumbing. `StoredEyes` is built by name in a few app
      tests (`crates/app/src/filter.rs`), which this step must keep
      compiling.
    - `NULL` means "pass 2 has not run, or nothing to compute"; a frame pass
      2 ran on with no candidate face stores 0 judged.

- [ ] Step 3: Show the counts: the meta pane's rows and the strip cell's mark (`handpic-app`)
  - Done when:
    - The meta pane's `Analysis` group shows the three counts for the shown
      file (suggested rows: `Faces judged`, `Faces with closed eyes`,
      `Faces with soft eyes`), each left out while its value is unknown
    - A strip cell draws a mark when at least one face of the frame has
      closed eyes, updated live as `faces-progress` arrives
    - Unit tests: the meta rows (values, the unknown cases), the pure
      "anyone closed" rule the cell reads
    - `README.md` / `README.ja.md` and `docs/humans/usage.md` /
      `docs/humans/usage.ja.md` describe the rows and the mark, including
      the size floor and the cap; `CLAUDE.md`'s `mark.rs` / `meta.rs` /
      strip descriptions are updated
    - The manual check on a real group photo folder (the mark and the rows
      agree with what the photos show) is listed as a pending manual check
      in the PR body
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged.
    - Files: `crates/app/src/meta.rs` (`analysis`, which already reads
      `focus.eyes`), `crates/app/src/mark.rs` (the pure rule),
      `crates/app/src/main.rs` (`StripCell`, the cell painting next to
      `paint_face_icon` / `paint_burst_badge`), `crates/app/src/icons.rs` if
      an icon is added.
    - The cell's corners in use: the face focus icon at the bottom-left, the
      burst badge at the bottom-right, the judgment `Marks` elsewhere. The
      step picks a free place and paints with the same halo
      (`overlay::HALO`). Prefer an unmodified Lucide catalog icon through
      `icon_assets!` (e.g. `eye-off` / `eye-closed`, whichever the vendored
      `gpui-kit-assets` holds) over a new drawing.
    - The existing face focus icon and its color (`af_eye_state`) do not
      change.

- [ ] Step 4: Filter by anyone's eyes, in the menu and over MCP (`handpic-app`, `handpic-engine`)
  - Done when:
    - The filter menu's `Eyes` section has an `Anyone closed` item and an
      `Anyone soft` item that pass a frame whose closed (soft) count is at
      least 1; they are OR-ed with the section's other checked items like
      every item of a section
    - The items round-trip through the stored `filter` setting, `get_view`'s
      `filter` reports them, and `set_filter` accepts them; the tool
      descriptions name them
    - Unit tests: the predicate (count 0, at least 1, unknown; OR with
      `Open` / `Closed` / `Unknown`; AND with the other sections), the
      stored-setting round trip, the menu rows, `companion.rs`'s
      `set_filter` / `get_view` cases; the coverage test still passes
    - `README.md` / `README.ja.md` and `docs/humans/usage.md` /
      `docs/humans/usage.ja.md` describe the items; `CLAUDE.md`'s
      `filter.rs` / `menu.rs` descriptions are updated
    - The manual check (the filter narrows a real group photo folder to the
      frames with a blink) is listed as a pending manual check in the PR body
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged (independent of Step 3).
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
      (or the two booleans) as input.
    - `filter_offers` needs no change: the items are static, like the other
      `Eyes` items.

## Decisions

Made at plan approval (2026-11-23), each the recommended option:

- Faces under `EYES_MIN_FACE` are excluded, not meshed from the full JPEG
  (Option A below); full-size meshing is a follow-up after Step 1's
  measurement.
- The mesh-fit gate for the other faces is added only if Step 1's
  measurement shows false closed readings.
- The soft threshold is `LOW_BELOW`.
- `Anyone soft` is included in Step 4.
- The release PR is held until Step 4 is merged.

## Trade-offs and risks

- **Small faces: exclude under a floor (chosen) or mesh them from the
  full JPEG.** The embedded preview is about 1616 px wide on Sony bodies, so
  faces in a large group are often 30 to 60 px. `EYES_MIN_FACE` (60 px) is
  the floor the closed-eyes judgment was calibrated with: under it a human
  can call the eye state of about one face in five on the preview.
  - Option A, exclude faces under `EYES_MIN_FACE`: the EAR threshold stays
    the calibrated one, the judged face's existing mesh is reused so the
    single-face values and the counts never disagree, and the cost is one
    preview-sized mesh per face. Consequence: in a large group (roughly more
    than 8 to 10 people across the frame) most faces go unjudged and the
    counts cover only the front faces; `Faces judged` in the meta pane is
    what tells the user how much was covered.
  - Option B, crop small faces from the full JPEG for the mesh: covers faces
    down to what YuNet finds at all (about 40 px at the 640x448 input, so
    detection is the next floor anyway). Consequences: `EYES_CLOSED_EAR` was
    fitted on preview-resolution crops and would need new labeled faces to
    trust on full-size crops; the judged face (preview mesh) and the counts
    (full-size mesh) could disagree about the same face; one more partial
    decode of the full JPEG per face; no effect on files without a larger
    JPEG.
- **False "closed" readings multiply with the number of faces.** The EAR
  cut has precision 0.94 on the judged face; the other faces include
  profiles, occluded faces and background people, where the mesh is less
  reliable. The mesh-fit gate (`eye_offset` at most 0.1, perhaps a yaw
  limit) applies to the other faces if Step 1's dump shows false closed
  readings; the judged face always counts, to stay consistent with the
  `Eyes` row. The gate lives in core (the counts are stored), so changing it
  later means another `FACES_VERSION` bump.
- **Scan time grows on every RAW frame with an AF point, not only on group
  photos.** Those frames gain a whole-image detection (about 16 ms on one
  thread per the existing measurements, plus a scaled decode) because the
  crop detection cannot see the other faces. Each further face costs a mesh
  (about 29 ms) and two crops of the full JPEG, bounded by the cap (8 faces:
  roughly a quarter of a second in the worst case). A frame whose cue judged
  no face but whose whole image holds faces now reads the full JPEG, which
  it did not before (still one read per file). Skipping the whole-image
  detection on AF frames would keep the cost but miss exactly the group
  photos shot with AF. Step 1 measures the real figures.
- **The cap undercounts.** With more candidate faces than the cap, only the
  largest are judged, and `Faces judged` reads the cap.
- **Soft threshold.** `LOW_BELOW` (0.110) over `SOFT_BELOW` (0.162), which
  depth of field would trip on most group frames. Even `LOW_BELOW` may flag
  back-row faces; the manual check should look at this.
- **`Anyone soft` sits in the `Eyes` section** although softness is the
  `Face focus` section's subject, because `Face focus` is a partition by
  `MarkState`.
- **Which face is the closed one is not shown.** The `f` overlay still
  outlines only the judged face's mesh (on-demand, not from the index).
  Marking each face on the overlay is not in this plan.
- **Step 1 merged alone makes pass 2 slower with nothing shown yet**, and
  Step 2's `FACES_VERSION` bump re-runs pass 2 on every folder at its next
  open. The release PR is held until Step 4.
- **A blink-free mark is not a guarantee**: faces under the floor, past the
  cap, below the 0.8 score or missed by YuNet are not judged. The docs
  should say so.

## Progress
