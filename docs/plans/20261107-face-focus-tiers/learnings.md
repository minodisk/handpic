# Learnings: face-focus-tiers

## Step 1

- `Bad` used to cover a focus candidate that missed the good rule, so `afEyeState`'s fallback moved from `not_candidate` to `neutral`, and the mark, the strip icon and the filter follow from that one change. A frame that is in focus but not good loses its gray icon on the strip, as the plan's trade-offs accept.
- The user docs said "each judged frame" gets a strip icon. That no longer holds, since a judged candidate that is not good is white and has no icon, so `usage.md`, `README.md` and their Japanese pairs now say the bright green or gray frames.
- The docs' "can be good or bad although it shows no crosshair" became "bright green or gray", because `Bad` is gone as a word.
- `MarkFocus.candidate`, `FaceReady.candidate` and `meta.ts`'s `FocusCandidate` keep their wire values, and the `Eyes` section keeps its own `unknown`. `not_candidate` now appears in non-test frontend code only in those types and in `afEyeState`.

## Deferred issues (todo candidates)

- **Pending manual check, Windows: the `Face focus` filter on a real folder.** Not run: it needs the built app on a real folder. The Step 1 checkbox was ticked on the automated criteria only. Basis: Step 1's implementation approach in `plan.md`. Files: `crates/app/ui/src/focus.ts`, `crates/app/ui/src/filter.ts`, `crates/app/ui/src/main.ts`, `crates/app/src/commands.rs`. Steps, on a folder with faced, face-less and JPEG frames under `D:\Photos\tests\<date>-<topic>\`:
  - Open the folder for its first scan with `Low` checked. Expected: the list grows as pass 2 reaches files.
  - After `faces-done`, step through the shown files. Expected: every one reads `AF eye in focus` under 77% in the meta pane.
  - Check `Neutral` alone. Expected: the face-less frames and the JPEGs show.
  - Launch with a filter saved before this change with `Bad` checked. Expected: the `Face focus` section opens unchecked and nothing is hidden.
- **`todo.md` items still name the old `AF eye` states.** Step 1 edited only the `ef` / `ef-fill` section and the `G0` cross-reference, as the plan lists. These now describe a menu that no longer exists and need rewording to `Face focus` / `Good` / `Neutral` / `Low`: `### App: real-device check of the AF eye Good / Bad / Unknown icons and mark colors` (its gray-on-bad-frames check and the `CLAUDE.md` rewording that names Good / Bad / Unknown), `### App: real-device check of the Good / Bad judgment on frames with no AF point`, `### App: re-measure a Good / Bad for frames with no face`, and the `CLAUDE.md` item that says `src/filter.ts` covers the `AF eye` section. Basis: the Step 1 grep of `todo.md` for `AF eye` and `Bad`. Files: `todo.md`.
