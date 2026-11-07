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

# `Face focus`: the filter's `AF eye` section as Good / Neutral / Low, with a `?` that defines them

## Purpose

The `ef-gate-fill` evaluation (`../_archived/20261104-ef-gate-fill/plan.md`)
found no gate, fill, face-size or reorder variant that stably improves the
`eye_focus` cue, so `ef` stays and sort-to-last is not done. What the user
wants instead is to narrow the strip to the frames whose judged face has its
eyes likely out of focus, review them and Reject by hand. On three label
rounds (33 folders, 46,984 frames, 6,214 picks; not pre-registered) frames
with `eye_focus` below the candidate threshold are 13.3% of the frames and
8.6% of the picks, discardable at 77 / 80 / 87%: a list to look through, not
a bulk delete.

The user chose to re-cut the existing `AF eye` section rather than add one,
ordered from the pick side to the reject side, and to rename it `Face focus`
(a body with no AF point is judged on a face too, and the section is not only
about the eye). The items are `Good` (unchanged), `Neutral` (everything that
is neither: a candidate face that misses a Good condition, no judged face, a
JPEG, a file pass 2 has not reached) and `Low` (the face's in-focus
probability below the candidate threshold, Rust's `not_candidate`). A frame
with no face is Neutral, not Low, on purpose: otherwise face-less frames
(landscapes, snaps, Leica frames with no face) would land on the reject side
regardless of focus, and the discardable rate of face-less frames was read on
sports-heavy data. Nothing is automatic; every state is a cue for a human.
Because the user finds the criteria hard to hold in mind, the section gets a
`?` that shows each item's definition with its thresholds.

What is established from the code:

- `focus.ts`'s `afEyeState` is the one place the filter (`AfEye =
  MarkState`), the strip icon (`stripState` -> `strip.ts` `paintTier`) and
  the `f` mark's color (`focusMark.state`) read the state from. Today it is
  `good` (bright green `#3f3`, `scan-face` icon), `not_candidate` (gray
  `#999`, `scan` icon; a judged face on a frame that is not good, in focus
  or not) and `unknown` (white `#fff`, no icon; no face, or pass 2 not yet
  there). The menu's `Good` / `Bad` items show the same icon and color;
  `Unknown` keeps an empty slot.
- The wire value `MarkFocus.candidate` (`candidate` / `not_candidate` /
  `unknown`, from Rust `index.rs`'s `serialize_candidate`, also
  `FaceReady.candidate` and `meta.ts`'s `FocusCandidate`) is the Rust cue's
  state derived by `riffle_core::candidate::candidate(eye_focus)`:
  `unknown` for `None`, `candidate` at or above `candidate_probability()`
  (`sigmoid(CANDIDATE_LOGIT)`, about 0.772), `not_candidate` below. It does
  not change. `candidate.rs`'s tests pin the boundary (`CANDIDATE_LOGIT`,
  `next_down`, `next_up`, `scored` storing a below-threshold probability
  strictly below `candidate_probability()`), so the frontend never compares
  against the number: `Low` is exactly `candidate === "not_candidate"`.
- The frontend cannot tell "pass 2 not run yet" from "pass 2 ran, no face"
  per file (both `eye_focus: null`, `candidate: "unknown"`; `faces_extractor`
  is not serialized). Both are Neutral under the new split, as both were
  `Unknown` before.
- Persistence: `StoredFilter.candidates` is whitelisted by
  `FILTER_CANDIDATES` in `crates/app/src/commands.rs`'s `filter_setting`,
  which already drops the retired `ok` (its tests near the
  `filter_setting_drops_unknown_members` test). A dropped member leaves the
  section unchecked, which shows everything: the safe direction.
- The `faces-progress` handler in `main.ts` refilters while
  `shownCandidates.size > 0 || shownEyes.size > 0`; that already covers
  files moving from Neutral to Good / Low as the pass runs.
- The meta pane shows `AF eye in focus` as a percentage only, Compare and
  the MCP companion use `photoTier(...) === "good"` (unchanged), and
  `companion.ts` does not read the state, so none of them changes. The meta
  pane's row keeps its name: it is a different value, the in-focus
  probability.
- Help patterns in the app: `title` tooltips on the `Sort` / `Filter`
  toggles and strip cells; no `?` affordance, no in-app docs link
  (`tauri_plugin_opener` is used for `open_path` only). The filter menu
  closes on `mousedown` outside `#filter`, so a control inside the menu can
  toggle text without closing it; items `blur()` on click so the zoom key
  does not re-press them.
- Filter-by-output (`../20261106-filter-by-output/plan.md`) reached `main`
  while this plan was being written (`crates/app/src/output.rs`, the strip's
  `Output` filter). It adds an `Output` section after `Eyes` and extends
  `passes` and the `main.ts` wiring. The investigation above read the code
  before it landed, so read the current `filter.ts`, `main.ts` and
  `index.html` first; this plan edits the `AF eye` block and the `AfEye`
  type, and the two are independent AND-ed sections.

## Steps

- [x] Step 1: The three-way `Face focus` state (Good / Neutral / Low) across the menu, the strip icon and the mark, its persistence, the todo item and the docs
  - Done when:
    - `crates/app/ui/src/focus.ts`: `MarkState = PhotoTier | "neutral" |
      "low"`. `afEyeState(focus)`: `null` / `undefined` -> `neutral`;
      `focus.candidate === "not_candidate"` -> `low`; `"unknown"` ->
      `neutral`; `"candidate"` -> `photoTier(focus) ?? "neutral"`. The
      comment says `low` is Rust's `not_candidate` (threshold in
      `candidate.rs`) and that no face and a file pass 2 has not reached are
      both `neutral`. `FOCUS_MARK_COLORS` / `FOCUS_MARK_ICONS` are keyed
      `good` (`#3f3`, `SCAN_FACE_SVG`), `low` (`#999`, `SCAN_SVG`), `neutral`
      (`#fff`, `null`): one state feeds menu, icon and mark. `stripState`,
      `focusMark` and `strip.ts` (`tiers` deletes a `neutral` entry as it
      deleted `unknown`) follow. `MarkFocus.candidate`, `FaceReady.candidate`
      and `meta.ts`'s `FocusCandidate` keep their wire values.
    - `focus.test.ts` covers the boundaries through the wire values Rust
      sends: `{ candidate: "unknown", eye_focus: null }` -> `neutral`;
      `null` focus (a JPEG) -> `neutral`; `{ candidate: "not_candidate",
      eye_focus: 0.7719 }` -> `low`; `{ candidate: "candidate", eye_focus:
      0.772 }` not clearing the rule -> `neutral`; `{ candidate:
      "candidate", eye_focus: GOOD_EYE_FOCUS }` with the other conditions
      met -> `good`, and `GOOD_EYE_FOCUS - 0.01` -> `neutral`. `stripState`,
      `FOCUS_MARK_COLORS` / `FOCUS_MARK_ICONS` tests follow the new keys.
    - `crates/app/ui/src/filter.ts`: `AfEye = MarkState` as before; the
      header comment describes the three items (Neutral includes a file the
      pass has not reached). `filter.test.ts`'s cases use `neutral` / `low`,
      including: `Low` checked passes a `low` frame only; `Neutral` passes
      `neutral` and an `undefined` state; `Good` + `Low` checked excludes
      `neutral`; `toStored` / `applyStored` round-trip `["neutral", "low"]`.
      `passes` falls back to `neutral`, not `unknown`, for an undefined state.
    - `crates/app/ui/index.html`: the section's `menu-label` reads
      `Face focus`, and its three buttons are, in order,
      `data-candidate="good"` `Good`, `data-candidate="neutral"` `Neutral`,
      `data-candidate="low"` `Low`, each keeping its `<i class="face">`.
      `main.ts`'s icon loop fills `good` and `low` (`neutral` keeps the empty
      slot); its `faces-progress` comment names the three states; everything
      else in the wiring is unchanged. Internal identifiers (`afEyeState`,
      `AfEye`, `data-candidate`, `candidates`) are not renamed.
    - `crates/app/src/commands.rs`: `FILTER_CANDIDATES = ["good", "neutral",
      "low"]`, the comment above it still naming the sources of truth. Its
      tests: `filter_setting_drops_unknown_members` drops `ok`,
      `not_candidate` and `unknown` and keeps `good` / `neutral`;
      `filter_setting_keeps_a_valid_value_and_drops_extra_keys` uses `low`.
      So a filter saved with `Bad` or `Unknown` checked reads back with that
      check gone (the section unchecked, nothing hidden). No mapping: `Bad`
      was broader than `Low` and `Unknown` is only part of `Neutral`, so
      neither can be carried over honestly.
    - `todo.md`: the whole `### Core: decide whether `ef` / `ef-fill` at
      per-folder q5, sorted last, is worth shipping` section (Background,
      Files, TODO) is deleted, and the `G0` section's cross-reference to it
      is reworded to say the question was closed (no sort-to-last; the
      `Face focus` filter's `Low` item is the shipped form) with this plan's
      archived path in backticks.
    - Docs, same PR: `docs/humans/usage.md`'s **Focus mark** entry (the
      gray / white sentences and the strip-icon sentence: gray and the bare
      frame now mean Low, white means Neutral, with no face, a candidate
      face that is not good, and a file not yet reached all white) and its
      **Filter menu** entry (the `Face focus` section, `Good` / `Neutral` /
      `Low` with what each holds, that each file falls under exactly one,
      that the strip refills as the pass runs, and that the thresholds are
      90% for Good and about 77% for Low, and every other mention of the
      `AF eye` section, such as the persistence sentence);
      `docs/humans/usage.ja.md` the same in Japanese; `README.md`'s
      focus-mark paragraph (the gray / white sentences, the section sentence,
      and the "focus mark icons, also on the filter menu's `Good` and `Bad`
      items" sentence) and `README.ja.md` likewise; `CLAUDE.md`'s
      `src/focus.ts` and `src/filter.ts` sentences (`afEyeState`: Good /
      Neutral / Low; the `Face focus` section's `Low` item is Rust's
      `not_candidate`).
    - `mise run ci` passes (format, lint, type-check, vitest, cargo tests).
  - Implementation approach:
    - Rename only the derived `MarkState` values and the menu's
      `data-candidate` values; never the `candidate` wire values, so no Rust
      serialization, `index.rs` test or `faces-progress` payload changes.
    - Grep `not_candidate` and `"unknown"` in `crates/app/ui/src` before
      finishing to separate the two meanings; `meta.ts`'s `FocusCandidate`
      and `MarkFocus.candidate` keep theirs. The `Eyes` section has its own
      `unknown`, which stays.
    - No change to `eye_focus`, `GOOD_EYE_FOCUS`, `CANDIDATE_LOGIT` or the
      sort order.
    - Hand check on Windows on a folder with faced, face-less and JPEG
      frames (under `D:\Photos\tests\<date>-<topic>\`): during the first
      scan with `Low` checked the list grows as pass 2 reaches files; after
      `faces-done` every shown file's meta pane reads `AF eye in focus`
      under 77%; `Neutral` shows the face-less frames and the JPEGs; a
      filter saved before the change with `Bad` checked opens unchecked.
      Record in `learnings.md`.

- [ ] Step 2: The `?` in the `Face focus` section that shows each item's definition
  - Done when:
    - `crates/app/ui/src/focus.ts` exports the definition text, built from
      the constants so it cannot drift from the rule: `AF_EYE_HELP:
      Record<MarkState, string>` plus one closing line, read as follows
      (wording may be tightened, the facts and figures may not):
      - Good: "A face was judged and all of these hold: its eyes are likely
        in focus (90% or more), open (openness 41 or more) and toward the
        camera (turned at most 35°, tilted at most 45°), the face mesh sits
        on the face, and the frame's edge does not cut it." (90 from
        `GOOD_EYE_FOCUS`, 41 from `eyesOpenness(GOOD_EYE_EAR)`, 35 / 45 from
        `GOOD_MAX_YAW` / `GOOD_MAX_PITCH`.)
      - Neutral: "Everything else: a judged face whose eyes are likely in
        focus (77% or more) but misses a Good condition, no face found (a
        landscape, a snap without a face), a JPEG, or a file the analysis
        has not reached yet."
      - Low: "A face was judged and its eyes are likely not in focus: the
        in-focus probability is under 77%."
      - Closing line: "A cue for your review, not a verdict; nothing is
        rejected for you."
      The 77 comes from a `CANDIDATE_PERCENT = 77` constant in `focus.ts`
      whose comment names `candidate.rs`'s `CANDIDATE_LOGIT` /
      `candidate_probability()`, and `crates/core/src/candidate.rs` gains a
      unit test asserting `(candidate_probability() * 100.0).round() ==
      77.0` with a comment that `ui/src/focus.ts`'s `CANDIDATE_PERCENT` and
      the user docs show this figure, so a threshold change fails a test
      that names the file to update. `focus.test.ts` checks the three texts
      contain `90%`, `41`, `35°`, `45°` and `77%` where they belong.
    - `crates/app/ui/index.html`: the `Face focus` `menu-label` row gains a
      small `?` button at its right (`type="button"`, `aria-expanded`,
      `aria-controls="filter-face-focus-help"`, `title="What these mean"`)
      and, after the three items, a `<div id="filter-face-focus-help" hidden>`
      the frontend fills with one line per item (the item name, then its
      text) and the closing line.
    - `crates/app/ui/src/main.ts`: the `?` click `blur()`s, toggles the
      block's `hidden` and `aria-expanded`, and stops propagation to the
      item handlers; the block is hidden again whenever the menu closes
      (`setFilterMenuOpen(false)`), so it never pops up unasked. The
      `mousedown`-outside close is untouched (the `?` is inside `#filter`).
    - `crates/app/ui/style.css`: a `.menu-help` class for the text (the
      `.menu-label` tone: muted, small, indented, wrapping at the menu's
      width, each line's item name in the foreground color) and the `?`
      button's style reusing the existing button / `menu-item` tokens (see
      `../../agents/ui-styling.md`); no new colors.
    - Docs: the **Filter menu** entry in `docs/humans/usage.md` /
      `usage.ja.md` says the `?` shows the definitions; `README.md` /
      `README.ja.md`'s section sentence mentions it in a few words;
      `CLAUDE.md`'s `src/focus.ts` sentence names the help text if it lists
      the file's exports at that level.
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Step 1 is merged.
    - Keep the text DOM-free in `focus.ts` so vitest checks it; `main.ts`
      only renders it (the pattern of `importsettings.ts` holding the
      Import tab's text).
    - Verify by hand that the menu stays open while the `?` is clicked, the
      block fits the menu without horizontal scroll at the default window
      size, and `Escape` (which closes the menu) hides the block.

## Trade-offs and risks

- **One state for menu, icon and mark (decided).** The strip and the `f` mark
  no longer show per file that pass 2 has reached it or that a judged face is
  in focus but not Good (both white now). What remains is the status line
  `analyzing N / M`, the meta pane's `AF eye in focus` / `Eyes` rows per
  file, and the Good / Low icons filling in.
- **`Low` is a face-based cue.** A deliberately defocused face is `Low`, and
  on a body with no AF point the judged face is the largest one, which may
  not be the subject the photographer focused on. The list is for review, so
  this is accepted; the `?` text says so.
- **Face-less sports frames leave the reject side.** They were discardable at
  78 / 90 / 87%, so `Low` recalls about half of what the original condition
  did. `Neutral` with the `Genre` section reaches them by hand while
  `aiEnabled` is on.
- **Saved `Bad` / `Unknown` dropped, not mapped.** Dropping leaves the section
  unchecked on the first launch after the update, which shows everything;
  the user re-checks once.
- **The help text's numbers.** 90 / 41 / 35 / 45 come from `focus.ts`'s
  constants; 77 is a frontend literal guarded by a Rust test on
  `candidate_probability()`. The filter itself never reads the number.
- **Out of scope.** Sort-to-last, per-folder q5, `ef-fill`, nose offset, face
  size, a new blind evaluation, bulk delete, genre-based routing, and any
  change to `eye_focus`, `GOOD_EYE_FOCUS` or the candidate threshold values.

## Progress

- (none yet)
