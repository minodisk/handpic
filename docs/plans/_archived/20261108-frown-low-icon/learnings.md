# Learnings

## Step 1

- `SCAN_SVG` was used only by `focus.ts` and `focus.test.ts`, so replacing it
  with `SCAN_FACE_FROWN_SVG` (the `SCAN_FACE_SVG` string with the mouth path
  `M8 16s1.5-2 4-2 4 2 4 2`) needed no change in `strip.ts` or `main.ts`.
- The docs naming the Low icon were `README.md` (strip bullet, licenses bullet),
  `README.ja.md` (the same two places), `docs/humans/usage.md` /
  `usage.ja.md` (the strip paragraph) and the `Icons` section of
  `docs/agents/ui-styling.md`.

## Deferred issues (todo candidates)

- Pending manual check (Step 1): on any platform's built app, open a folder
  with faced frames, let pass 2 finish and look at a gray (Low) strip cell
  and the filter menu's `Face focus` > `Low` item at 12 px; expected: a
  frowning face in the scan frame in `#999`, distinguishable from Good's
  smiling face. The step's checkbox was ticked on the automated criteria
  (unit test of `FOCUS_MARK_ICONS`, `mise run ci`). Done when: the two icons
  are told apart at 12 px (close it), else a follow-up adjusts the mouth path
  or the stroke width.
