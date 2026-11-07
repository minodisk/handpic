# Learnings: face-focus-tiers

## Step 1

- `Bad` used to cover a focus candidate that missed the good rule, so `afEyeState`'s fallback moved from `not_candidate` to `neutral`, and the mark, the strip icon and the filter follow from that one change. A frame that is in focus but not good loses its gray icon on the strip, as the plan's trade-offs accept.
- The user docs said "each judged frame" gets a strip icon. That no longer holds, since a judged candidate that is not good is white and has no icon, so `usage.md`, `README.md` and their Japanese pairs now say the bright green or gray frames.
- The docs' "can be good or bad although it shows no crosshair" became "bright green or gray", because `Bad` is gone as a word.
- `MarkFocus.candidate`, `FaceReady.candidate` and `meta.ts`'s `FocusCandidate` keep their wire values, and the `Eyes` section keeps its own `unknown`. `not_candidate` now appears in non-test frontend code only in those types and in `afEyeState`.

## Step 2

- `#filter-menu` is absolutely positioned in the narrow `#filter`, so its shrink-to-fit width stays at its `min-width` of 180px (`.menu-item` wraps, nothing is `nowrap`). The help block therefore wraps at the menu's width without a width trick, read from the CSS only. At 0.7rem that is about 25 characters a line, so the open block is tall, about 24 lines by estimate.
- `.menu-help` sets `display: flex`, so it is written `.menu-help:not([hidden])`, as `ui-styling.md` asks. A plain class rule would beat the `hidden` attribute.
- The help lines read each item's name from its own button, so the names exist once, in `index.html`.
- `hidden` is typed `string | boolean` here, so the toggle needs `!!faceFocusHelp.hidden`, as `filterMenu.hidden` does. This failed `mise run ci` once at the type check.
- The Low text is the plan's wording unchanged: "A face was judged" plus the closing line carry that it is a cue on the judged face. The plan's trade-off says the `?` covers a deliberately defocused face and the largest face on a body with no AF point, but the agreed text names neither case.
- A face the detector picked is never `Unknown`: without an edge its probability is 0 and its state `not_candidate`, so it is Low. That matches the Low text only loosely, since no edge means "no sharp edge found", which is a fair reading of "likely not in focus".

## Deferred issues (todo candidates)

- **Pending manual check, Windows: the `Face focus` filter on a real folder.** Not run: it needs the built app on a real folder. The Step 1 checkbox was ticked on the automated criteria only. Basis: Step 1's implementation approach in `plan.md`. Files: `crates/app/ui/src/focus.ts`, `crates/app/ui/src/filter.ts`, `crates/app/ui/src/main.ts`, `crates/app/src/commands.rs`. Steps, on a folder with faced, face-less and JPEG frames under `D:\Photos\tests\<date>-<topic>\`:
  - Open the folder for its first scan with `Low` checked. Expected: the list grows as pass 2 reaches files.
  - After `faces-done`, step through the shown files. Expected: every one reads `AF eye in focus` under 77% in the meta pane.
  - Check `Neutral` alone. Expected: the face-less frames and the JPEGs show.
  - Launch with a filter saved before this change with `Bad` checked. Expected: the `Face focus` section opens unchecked and nothing is hidden.
- **`todo.md` items still name the old `AF eye` states.** Step 1 edited only the `ef` / `ef-fill` section and the `G0` cross-reference, as the plan lists. These now describe a menu that no longer exists and need rewording to `Face focus` / `Good` / `Neutral` / `Low`: `### App: real-device check of the AF eye Good / Bad / Unknown icons and mark colors` (its gray-on-bad-frames check and the `CLAUDE.md` rewording that names Good / Bad / Unknown), `### App: real-device check of the Good / Bad judgment on frames with no AF point`, `### App: re-measure a Good / Bad for frames with no face`, and the `CLAUDE.md` item that says `src/filter.ts` covers the `AF eye` section. Basis: the Step 1 grep of `todo.md` for `AF eye` and `Bad`. Files: `todo.md`.
- **Pending manual check, Windows: the `Face focus` `?`.** Not run: it needs the built app. The Step 2 checkbox was ticked on the automated criteria only. Basis: Step 2's implementation approach in `plan.md`. Files: `crates/app/ui/src/main.ts`, `crates/app/ui/index.html`, `crates/app/ui/style.css`. Steps, in the app at the default window size:
  - Open the filter menu and click the `?` beside `Face focus`. Expected: the menu stays open, the definitions appear under `Low`, no item's check changes, and a second click hides them.
  - With the definitions shown, look at the menu's width and scrolling. Expected: the text wraps at the menu's width and no horizontal scrollbar appears.
  - With the definitions shown, press `Escape`, then reopen the menu. Expected: the menu closes, and it reopens with the definitions hidden. Clicking outside the menu does the same.
