# Learnings: `eye_focus` gate and fill

## Step 1 (pre-registration of the exploration)

- The discard-model plan is already archived
  (`docs/plans/_archived/20261023-discard-model/`), so `efgate.md` links
  there; the scripts of Steps 3–6 still resolve its path at run time.
- Code references checked at `7085586e` before quoting them:
  `candidate::mesh_eye_offset`, `candidate::eye_focus`, `Scored::Window`,
  `candidate::meshes_face`, `eyes::EYES_MIN_FACE` (60.0), `eyes::Mesh.points`
  (stored coordinates), `faces::Face.nose`, and `pose.rs`'s tests reading
  point 4 as the nose tip.
- `eye_focus` falls back to `Scored::Window` even with a mesh when no eyelid
  region counts; on those faces `window_ef` equals `ef`, so the gate cannot
  change them. Stated in `efgate.md` under "The per-face values".
- Sources of the known numbers: round 3 and its subset reading from
  discardmodel.md "Results of the confirmation"; rounds 1–2 `ef-fill` from
  its "Results of the exploration"; rounds 1–2 `ef` and base rates from
  facechoice.md "Reproduction checks"; the fresh missing shares (9.00% /
  1.18%) from the discard-model learnings, Step 4; 6,214 picks = 4,419 +
  1,795.
- Added one sentence the plan implies but does not spell out: if no
  candidate passes the label-free conditions at q5 or q10, there is no
  winner and the plan ends as under the stop rule (as discardmodel.md did).
