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

# Face presence gate and side-face mesh distrust

## Purpose

Five `todo.md` items of the "Core: face/eye-aware focus check for culling"
section were measured and decided by the user on 2026-11-23. Two of them
change code: the face mesh model's face presence output (`Identity_1`), so
far thrown away, now decides when the mesh of a face is not to be believed.
A side face whose mesh is fitted off the face no longer gives a
plausible-looking wrong EAR, head pose, eye offset, edge gap or eye
sharpness, and a face the model itself says is hardly there no longer gets
an open / closed judgment. The other three are decisions without code (the
good-photo cuts stay shared for frames with no AF point, `MAX_EYE_OFFSET`
stays 0.10, no crop-box filter). This plan ships the two rules, records all
five measurements and decisions in `learnings.md`, and removes the five
items from `todo.md`.

## Steps

- [x] Step 1: Read the presence logit, apply the two rules, record the five
      decisions
  - Done when:
    - `crates/core/src/eyes.rs` resolves the `Identity_1` output by outlet
      label in `build`, next to `Identity`; the model run returns the points
      and the presence logit; `Mesh` and `Judged` carry `presence`
    - Rule (a), side-face distrust: when YuNet's eye distance (between the
      face's `left_eye` and `right_eye`, divided by the box's longer side) is
      under 0.15 AND the presence logit is under 0, the mesh is untrusted.
      In the scan's cue (`crates/core/src/candidate.rs`) `eyes_ear`, `pose`,
      `eye_offset`, `edge_gap` and `eye_centers` are all `None` for such a
      face (no fallback to YuNet's eye landmarks), so `full_eye_sharp` is
      `None` too; `eyes::judge_mesh` returns `None`, so
      `ondemand::read_eyes` answers unknown and the `f` overlay draws no mesh
    - Rule (b), presence gate: otherwise, when the presence logit is under
      -12 (any face, whatever its eye distance), the cue's `eyes_ear` and
      `pose` are `None` while `eye_centers` (the mesh's), `eye_offset`,
      `edge_gap` and so `full_eye_sharp` are kept; `eyes::judge_mesh` returns
      `None`
    - A face without a mesh (under `EYES_MIN_FACE`, or the model failed)
      behaves as before: its eye centers are YuNet's landmarks
    - The three thresholds are named constants in one place in `eyes.rs`,
      each with a doc comment that points at this plan's `learnings.md`
    - `FACES_VERSION` is bumped by one above main's in
      `crates/engine/src/index.rs` with its doc comment extended; no schema
      change, `EXTRACTOR_VERSION` and `SCHEMA_VERSION` unchanged
    - The `handpic-cli eyes` line prints the presence logit
    - `docs/agents/tract-onnx-inference.md` describes the model's three
      outputs (`Identity` the points, `Identity_1` the presence logit, now
      resolved, `Identity_2` unknown and unused)
    - `docs/humans/usage.md` and `docs/humans/usage.ja.md` add the side-face
      and low-presence cases where they say when the meta pane's `Eyes` row
      is left out and when the filter's `Eyes` is `Unknown`
    - Unit tests: both rules at their boundaries as pure functions of the eye
      distance and the logit (0.15 and just under, logit 0 and just under,
      -12 and just under, and that rule (a) needs both conditions); a cue
      built from an untrusted mesh has `None` for the EAR, the pose, the eye
      offset, the edge gap and the eye centers; a cue built from a mesh under
      -12 only has `None` for the EAR and the pose and keeps the mesh's eye
      centers
    - `learnings.md` in this folder carries the five measurements and
      decisions (no-AF cuts, crop false positives, presence gate, side-face
      distrust, `eye_offset` under 0.20), with the follow-ups recorded there
      (a separate no-AF eye-sharpness cut, about 0.12, is worth checking only
      against failure labels; any change to the face crop needs the presence
      thresholds recalibrated)
    - `todo.md` no longer lists the five items, and no other item is changed
      except a sentence that directly refers to one of them
    - `CLAUDE.md`'s layout description is updated where it describes what
      changed, and only there
    - `docs/plans/20261123-all-faces-check/` is untouched; no change to the
      good-photo rule's cuts
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach:
    - Read `docs/agents/tract-onnx-inference.md` and the `FACES_VERSION`
      paragraphs of `docs/agents/shared-crates.md` first.
    - `eyes.rs`, the model. `Model` gains the output index of `Identity_1`,
      found the same way as `points` (by `outlet_label`, failing the build
      with the names when missing). The private `landmarks` returns the
      points and the logit from the one run (the output is `[1,1,1,1]`, one
      `f32`). Its callers change with it: `mesh_of` (sets `Mesh::presence`),
      `landmarks_of` (public, used by `ear_of`, `judge_mesh` and
      `crates/cli/src/main.rs`'s `judge_file`; a tuple or a small struct),
      and the test `judges_a_face_in_a_real_image`.
    - `eyes.rs`, the rules. Pure functions next to the constants (suggested:
      `SIDE_FACE_EYE_DISTANCE: f32 = 0.15`, `SIDE_FACE_PRESENCE: f32 = 0.0`,
      `EYES_MIN_PRESENCE: f32 = -12.0`; one function for "is this mesh
      trusted" over a `Face` and a logit, one for "does this logit allow an
      eyes judgment"), shared by `candidate.rs`, `judge_mesh` and the CLI.
      "Under" is a strict `<`. The eye distance is
      `hypot(right_eye - left_eye) / max(width, height)` (the measurement's
      `yed` column, checked against
      `D:\Photos\tests\2026-11-23-side-faces\dump\`); it is invariant under
      the upright rotation. Decide and test what a non-finite logit and an
      empty box do (a plain `<` treats NaN as trusted).
    - `eyes.rs`, `judge_mesh`: return `None` when either rule fires, after
      the model run. `ear_of` and `landmarks_of` stay raw.
    - `candidate.rs`. The rule belongs in `cue_of`, not in `face_mesh`:
      `cue_of` falls back to `face`'s landmarks for `eye_centers` when it is
      given no mesh, which must stay true for a face without a mesh
      (`a_face_under_the_floor_gets_no_mesh_and_keeps_its_landmarks`) and
      must not happen for an untrusted mesh. `cue_of` tells the three cases
      apart: no mesh (landmarks), untrusted mesh (everything `None`,
      `eye_centers` included), presence under -12 (EAR and pose `None`, the
      rest from the mesh). Test helpers `boxed_eyes` / `both_eyes` need the
      new field. Reword the `Cue` field docs, `cue_of`'s doc and the module
      doc where "`None` without a mesh" is no longer the whole story.
    - `crates/core/src/scan.rs`, `crates/engine/src/ondemand.rs` and
      `crates/app` need no code change (`full_eye_sharp` is `None` without
      eye centers; `read_eyes` maps `judge_mesh`'s `None` to unknown).
    - `crates/engine/src/index.rs`: `FACES_VERSION` one above main's, with a
      sentence in its doc comment. `docs/plans/20261123-all-faces-check/`
      Step 2 plans a bump too; take the next free number at rebase time.
    - `crates/cli/src/main.rs`: the CLI's `Judged` and `eyes_line` gain the
      logit (one decimal or `-`); the pinned-format test changes with it. The
      state column follows the two rules; the EAR and pose columns stay raw.
    - `docs/agents/tract-onnx-inference.md`: rewrite the "Outputs by outlet
      label" bullet with the shapes (`Identity` `[1,1,1,1434]`, `Identity_1`
      `[1,1,1,1]`, a raw logit, clear faces +15..+25, `Identity_2` `[1,1]`,
      0..1, meaning unknown, unused), (Measured), what the logit is used for,
      and that it is low on real profiles and faces rotated in the crop, so
      the thresholds must be recalibrated after any crop change.
    - Doc comments name the post-archive path
      (`docs/plans/_archived/20261123-mesh-presence-side-faces/learnings.md`):
      single-PR mode archives in the same PR. Fix `eyes.rs`'s module doc's
      stale `eyes-truth.md` path if that paragraph is edited anyway.
    - `learnings.md` is seeded from the session's decisions file. Clean it:
      keep the measurements, tables and decisions; drop the "Implementation
      sketch" / "Implementation" paragraphs and every path into the session
      scratchpad (`C:\Users\...\Temp\...`, `scratchpad\...`). The
      `D:\Photos\tests\2026-11-23-*` paths stay, in backticks (scripts and
      sheets were copied there: `noaf-cuts\scripts\`,
      `crop-fp\sheets-and-scripts\`, `face-presence\scripts\`,
      `side-faces\b1-gains-sheet.png`). Add the eye-distance definition to
      the side-face section.
    - `todo.md`: remove exactly the five items by content ("Use the model's
      face presence output (`Identity_1`...", "Check the good-photo cuts on
      starred frames with no trusted AF point.", "Make `eye_offset` reliable
      where YuNet's eye distance is under 0.20", "Distrust the mesh values on
      side faces", "Crop detection can false-positive on printed faces or
      logos"). The done item "Give the no-AF path a good-photo mark once it
      has a focus cue" ends with "The check of those cuts on starred no-AF
      frames is the next item."; reword it to point at this plan's
      `learnings.md`.
    - `CLAUDE.md`: add the presence output and the distrust to the
      `src/eyes.rs` and `src/candidate.rs` clauses; nothing else.
    - Verify: the bundled test image still gets a judgment; `handpic-cli
      eyes` on a few real files prints a logit in +15..+25 for a frontal
      face.

## Trade-offs and risks

- `FACES_VERSION` collision with `docs/plans/20261123-all-faces-check/`
  Step 2; whichever lands second takes the next number.
- The all-faces check can reuse the public rule functions; `mesh_of` stays
  raw.
- Rule (b) asymmetry: the scan keeps the mesh's eye centers and
  `full_eye_sharp` for a face under -12, while the on-demand judgment returns
  nothing, so the `f` overlay draws no mesh for it. As decided.
- Fewer Low marks after the re-run (1415 -> 1021 on six folders), good
  1343 -> 1342. Intended.
- The thresholds are in-sample (unreviewed agent labels) and crop-dependent;
  recorded in `learnings.md`, the doc comments and the tract guide.

## Progress

- (2026-11-24) Step 1 complete
