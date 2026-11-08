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

# Frowning-face icon for Face focus Low

## Purpose

The Face focus states are shown as icons on the strip cell (bottom-left of the
image box) and on the filter menu's `Face focus` items, both at 12 px: Good is
Lucide's `scan-face` (a smiling face in the scan frame), Low is Lucide's
`scan`, the bare frame. The bare frame does not say what Low means. Low is a
judged face whose eyes are probably out of focus, so its icon becomes the same
`scan-face` with the mouth turned down: Good = smiling face, Low = frowning
face, the same frame and eyes, color unchanged (`#999`).

How the user got here (2026-11-08, from prototypes): a blurred face was tried
first and dropped, because at the 12 px the icon is drawn at the blur reads as
a faint face or a gray smudge, not as blur. Of the non-blur variants the
frowning face reads as a face and as Good's counterpart at 12 px; a face
without eyes stopped reading as a face, × eyes read as "eyes closed / knocked
out" and would mix with the separate Eyes judgment, a slash reads as "no face".
Red (the reject color) was considered and not taken: it would make a machine
guess look like the user's reject.

## Steps

- [x] Step 1: Replace the Low icon with `scan-face` whose mouth is turned down
  - Done when:
    - The Low icon is the `scan-face` frame and eyes with the mouth
      `M8 16s1.5-2 4-2 4 2 4 2` (Lucide's mouth `M8 14s1.5 2 4 2 4-2 4-2`
      mirrored vertically about y = 15), same stroke attributes as the other
      icons, shown on the strip cell of a Low frame and on the filter menu's
      `Low` item in `#999`; Good and Neutral are unchanged.
    - `SCAN_SVG` is gone from `icons.ts` (only `focus.ts` and `focus.test.ts`
      use it); the `/*! ... */` header of `icons.ts` no longer lists `scan` and
      says the Low icon is `scan-face` with its mouth turned down (a
      derivative, which the ISC license permits; the full notice stays
      `crates/app/ui/LICENSE-lucide`).
    - `focus.ts`'s `FOCUS_MARK_ICONS` comment says a smiling face for good, a
      frowning face for low, none for neutral; `focus.test.ts` asserts the new
      mapping.
    - The docs that call the Low icon Lucide's `scan` / "a bare frame" say the
      frowning `scan-face` instead: `README.md` (the strip bullet and the
      licenses bullet), `README.ja.md`, `docs/humans/usage.md` and
      `docs/humans/usage.ja.md`, each pair in the same PR; the `Icons` section
      of `docs/agents/ui-styling.md` notes the one derived icon (icons are
      otherwise verbatim from lucide-static).
    - `mise run ci` passes.
  - Implementation approach:
    - `crates/app/ui/src/icons.ts`: replace `SCAN_SVG` with a constant such as
      `SCAN_FACE_FROWN_SVG`: the `SCAN_FACE_SVG` string with the mouth path
      swapped for `M8 16s1.5-2 4-2 4 2 4 2`.
    - `crates/app/ui/src/focus.ts`: import it, use it for `low`, reword the
      comment. `strip.ts` and `main.ts` need no change.
    - `crates/app/ui/src/focus.test.ts`: update the `FOCUS_MARK_ICONS` test.
    - Docs: see "Done when".

## Trade-offs and risks

- The Good / Low difference is the mouth's direction plus the color; at 12 px
  the mouth is a few pixels, so the color still carries most of it. The user
  saw the 12 px prototype and chose it.

## Progress

- (none yet)
