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

# Bursts in the GPUI app: bands, badges, burst keys, reject the rest

## Purpose

The Tauri app groups a folder's frames into bursts (`crates/app/ui/src/burst.ts`:
consecutive frames in capture order no more than `BURST_GAP_MS` = 1000 ms
apart; Leica files carry whole seconds only, so the gap cannot go below 1 s),
draws each burst as a band behind its strip cells with a `position/size`
badge, and has five keymap actions on them: `burstPrevious` / `burstNext`
(`arrowup` / `arrowdown`), `burstFramePrevious` / `burstFrameNext`
(`alt+arrowleft` / `alt+arrowright`) and `rejectRest` (`shift+x`), the
defaults of `crates/backend/src/shortcuts.rs`.

The GPUI app (`crates/native`) has none of it: `key_down`'s action match in
`crates/native/src/main.rs` has no arm for the five actions (they are logged
as not handled; `settingsmodal.rs` only labels them), the strip paints no
band and no badge, and the MCP companion's `get_view` always answers
`"burst": []` (`crates/native/src/companion.rs`).

Once this is done the native strip shows the bands and badges, the five
actions behave as in the Tauri app, and `get_view` reports the current
file's burst. It is a slice of the port plan
(`../20261109-gpui-port/plan.md`), which names bursts in no step; this plan
records itself there.

Out of scope: Compare mode, grayscale, the AI features (the recommended mark
and sort, the ranking), and the remaining steps of
`../20261113-gpui-folder-dialogs/plan.md`, which another session is
implementing.

### What the investigation found (2026-11-14, `main` at `61b0ec3e`)

- `burst.ts` is pure: `groupBursts` (over every listed file, in `capture`
  order through `orderFiles`, a file without a usable capture time a burst of
  its own), `burstMarks` (per displayed file: `first`, `last`, `position`,
  `size`; `null` outside a burst of two or more), `burstBadge`
  (`position + 1` / `size`), `burstStep` and `burstFrameStep` (over the
  displayed files' burst ids). `burst.test.ts` has 20 cases.
- The Tauri app regroups only when `folder_entries` arrives (over `allFiles`),
  empties the map on a folder open, and re-applies the marks on every rebuilt
  list (`refilter`) and after each regroup (`applyBursts`).
- `moveBurst` / `moveBurstFrame` build the ids as
  `bursts.get(path)?.burst ?? -1 - at` (a file not grouped yet is its own
  id), move the index, select the landed file alone, stamp the keypress for
  the latency and show it. They are not refused in a view-only folder.
- `rejectRest` (`main.ts`): nothing in a view-only folder, with no files, or
  when the current file's burst has fewer than two members. Otherwise every
  other member over `allFiles` (so one the filter hides too) that is not
  already a reject gets `flag: reject` with its stars and label kept, as one
  undo entry; the redo history is cleared; the refilter is anchored on the
  current file; there is no auto-advance. Each store sends
  `labelKnown = entries.has(path)` (the label does not change).
- Undo of that entry is the ordinary `step`: an entry of several files
  leaves the current file where it is, an entry of one file (a two-frame
  burst) moves to it unless the filter hides it.
- The native side already has what the actions need: `sort::order_files` and
  `SortFacts`, `filter::Facts` with `capture_time` / `subsec` per listed
  path (`self.facts`), `go(Move::To(at))` (the move, the selection of the
  landed file alone, the latency key), `judge_paths` (one `history::Undo`
  entry, `touched`, `set_state`, one `refilter` anchored on the current
  file, a `store` per change, no auto-advance), `history::step` with the
  one-file / several-files rule, and `treekeys::tree_gate`, which swallows
  every action outside `TREE_PASSTHROUGH` while the tree has the keyboard.
- `Strip` is rebuilt (`Strip::new`) in `opened` and in `carry`, and
  `mark_strip` re-applies the per-index judgments after each; `set_entries`
  is where `all` and `facts` are both fresh (`opened` and `read_entries`).
- The Tauri band (`crates/app/ui/style.css`, `.cell.burst::before`):
  `rgba(255, 255, 255, 0.08)`, 5 px above and below the cell, 1 px outside
  it left and right, across the 8 px gap to the left on a member that is not
  `burst-first`, 2 px outside with a 6 px radius at `burst-first` /
  `burst-last`, behind the cell's contents. The badge (`.cell span.count`):
  bottom-right of the image box, 2 px in, just above the file name,
  `0.7rem`, `--foreground` with a dark halo.
- The native strip has `STRIP_PADDING` = 4 px above and below the cells
  (the Tauri strip has 7 px, "room for the burst band").
- The Tauri `get_view` `burst` (`companion.ts`, `burstOf`): empty unless the
  current file's burst has two or more members; else every member in
  position order with `path`, `position` (1-based), `sharpness`, `rating`,
  `flag`, `label` and `visible` (whether the filter shows it).
  `filter::Facts` does not carry `sharpness`; `IndexedFile.sharpness` does.

## Steps

- [x] Step 1: Burst grouping, the strip's bands and badges, and the four burst move actions
  - Done when:
    - `crates/native/src/burst.rs` is a port of `burst.ts` (`BURST_GAP_MS`,
      the member and mark types, `group_bursts`, `burst_marks`,
      `burst_badge`, `burst_step`, `burst_frame_step`) with every
      `burst.test.ts` case as a unit test.
    - The view regroups over `all` whenever the entries are set and keeps
      the result for the open folder; the strip's cells get their burst
      marks wherever they get their judgments, including after an entries
      read that leaves the list unchanged.
    - The strip paints the band behind the cells of a burst of two or more
      and the `position/size` badge on each member, with the Tauri geometry
      and colors; the band is distinguishable from the current cell's
      outline, the selected cell's fill and a failed cell; a rejected
      member's badge is dimmed with its cell.
    - `burstPrevious` / `burstNext` and `burstFramePrevious` /
      `burstFrameNext` move as the Tauri `moveBurst` / `moveBurstFrame`:
      over the displayed files' ids (so under any filter and sort), the
      landed file selected alone, clamped at the ends, also in a view-only
      folder, swallowed while the tree has the keyboard, the preview latency
      starting at the key.
    - `docs/agents/native-app.md` has a section on the burst grouping (when
      it is regrouped, where the marks are applied, how the band is
      painted); `CLAUDE.md`'s `crates/native` sentence names
      `src/burst.rs`; `../20261109-gpui-port/plan.md` records this plan (a
      sub-bullet and a Progress line).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/burst.rs` (new, `mod burst;` in `main.rs`),
      `crates/native/src/strip.rs`, `crates/native/src/main.rs`,
      `docs/agents/native-app.md`, `CLAUDE.md`,
      `docs/plans/20261109-gpui-port/plan.md`.
    - Grouping goes through `sort::order_files(SortKey::Capture, ..)` with
      `SortFacts` from `self.facts`, as `groupBursts` goes through
      `orderFiles("capture", ..)`; do not write a second capture order.
    - Regroup in `set_entries` (both callers have set `all` first); a new
      folder's open reaches it too, which replaces the Tauri reset. Do not
      regroup in `refilter` or on a page.
    - The strip model keeps the marks per index next to `judgments`
      (`Strip` is rebuilt by `opened` and `carry`), set from the place
      `mark_strip` runs so one call covers `opened`, `refilter` and
      `read_entries`; `StripCell` carries the cell's mark to `paint_strip`.
    - The ids for the moves are the Tauri ones: the burst id, else
      `-1 - at` for a file without a member. The move itself is
      `go(Move::To(next), cx)`; add the four arms next to `previous` /
      `next` in `key_down`. No change to `treekeys.rs`, `keys.rs` or
      `shortcuts.rs`.
    - Paint every on-screen band before any cell, so a band crossing the
      gap never covers the neighbour's outline. Verify the band of the
      first on-screen cell does not paint outside the strip's bounds (mask
      it to the strip if it does).
    - The badge is text on the canvas: `shape_line` through the window's
      text system with dark copies for the halo, as `paint_marks` does the
      stars (`native-app.md`: "Text has no stroke; a label's halo is dark
      copies"), painted before the rejected dim.
    - Raise `STRIP_PADDING` to 7 px (decided, Trade-off 2) and parse an
      impossible date as no capture time (decided, Trade-off 3, noted in
      `native-app.md` as a deliberate difference).
    - In the port plan add only one sub-bullet under Step 9 and one Progress
      line (decided, Trade-off 4); the folder-dialogs session appends to the
      same Progress list.
- [x] Step 2: `rejectRest` with its undo, and the MCP companion's `burst`
  - Done when:
    - `rejectRest` rejects every other member of the current file's burst
      over the whole listing (a member the filter hides too), skipping the
      ones already rejected, keeping each one's stars and label, as one
      undo entry with the redo history cleared; the current file stays
      current; nothing auto-advances; nothing happens in a view-only
      folder, with no file shown, or outside a burst of two or more.
    - Undo and redo of that entry behave as the Tauri `step`: several files
      leave the current file where it is with the `Undid N files` /
      `Redid N files` note, one file moves to it unless the filter hides
      it; a failed store drops its file from the entry.
    - The changes are built by a pure function with unit tests (the current
      file left out, the already rejected skipped, hidden members included,
      stars and label kept, empty for a singleton or an ungrouped file).
    - `get_view`'s `burst` is the Tauri `burstOf`: empty for a burst of
      one, else every member in position order with `path`, 1-based
      `position`, `sharpness`, `rating`, `flag`, `label` and `visible`;
      `companion.rs`'s tests cover it with `companion.test.ts`'s burst
      cases, and its module comment no longer says there are no bursts.
    - `docs/agents/native-app.md` describes `rejectRest` under the judgment
      rules and no longer says `burst` is always empty; `CLAUDE.md`'s
      `src/companion.rs` and `src/burst.rs` wording matches;
      `../20261109-gpui-port/plan.md` has the Progress line.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (the view's burst map and `burst.rs`).
    - Files: `crates/native/src/burst.rs` (or `judgment.rs`, for the pure
      change builder), `crates/native/src/companion.rs`,
      `crates/native/src/filter.rs` (`Facts.sharpness`),
      `crates/native/src/main.rs`, `docs/agents/native-app.md`,
      `CLAUDE.md`, `docs/plans/20261109-gpui-port/plan.md`.
    - Run the changes through the existing `judge_paths`: it already
      records one entry, marks `touched`, applies, refilters once anchored
      on the current file and stores each change, without auto-advance.
      Build `companion::Change`s (or move that type somewhere neutral) with
      `label_known` = the path is in `seen`, the Tauri
      `entries.has(path) || label !== before.label` with the label
      unchanged.
    - `rejectRest` is not a `judgment::Command`: it does not go through
      `selection::targets` / `selection::judgments`, and the selection does
      not widen or narrow its targets. Add its arm before the
      `Command::from_action` fallback in `key_down`.
    - `companion::View` gets the burst map and a way to read a path's
      sharpness; `visible` is membership in `View.files`. `rating`, `flag`
      and `label` come from `View.judgments` in the forms the `judged`
      reply already uses.
    - Verify during implementation when the Tauri `sharpness` map is
      refreshed (it is filled from `folder_entries` only) and give
      `Facts.sharpness` the same freshness; do not add a pass 2 update the
      Tauri app does not have.

## Trade-offs and risks

1. One PR or two. Decided (2026-11-14, user): two PRs, the read-only part
   (grouping, drawing, moves) first and the part that writes judgments and
   sidecars second.
2. The strip's height. The Tauri band reaches 5 px above and below the cell
   inside 7 px of padding; the native `STRIP_PADDING` is 4. Decided
   (2026-11-14, user): raise it to 7 for the Tauri look; the strip is 6 px
   taller and the preview canvas 6 px shorter.
3. Impossible dates. `burst.ts` builds the time with `Date.UTC`, which
   rolls a month 00, a day 32 or an all-zero timestamp over into a finite
   time, so such files can fall into one burst in the Tauri app. Decided
   (2026-11-14, user): parse strictly and treat such a file as having no
   capture time, a burst of its own; noted in `native-app.md` as a
   deliberate difference.
4. Where the port plan records this. Decided (2026-11-14, user): a
   sub-bullet under Step 9 (open, and the owner of the MCP companion) plus
   Progress lines.
5. Conflicts with the folder-dialogs session. Both plans change
   `crates/native/src/main.rs`, the `crates/native` sentence of `CLAUDE.md`,
   `docs/agents/native-app.md` and the port plan's Progress list (its Step
   7 appends there). The changes here are kept to new arms, new functions
   and appended lines, but a rebase is to be expected.
6. A stale grouping between a change and the next entries read. As in the
   Tauri app the map is rebuilt only when the entries are read, so after a
   Move to Trash the badges keep the old size until the re-listing's
   entries arrive. This is the Tauri behavior and is kept.
7. What unit tests cannot show. The band's and the badge's look on a real
   burst folder (Sony sub-second and Leica whole-second), the keys on
   Windows and macOS, and `rejectRest`'s sidecars and undo on real files
   are hand checks; the ones not run by hand are recorded in `todo.md` at
   wrap-up, as the Tauri burst checks were.

## Progress

- (2026-11-14) Step 1 complete
- (2026-11-14) Step 2 complete
