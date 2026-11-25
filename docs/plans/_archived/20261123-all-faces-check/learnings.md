# Learnings

## Step 1: Measure every confident face in pass 2

- The measurement is in [measurement.md](./measurement.md). Group folders
  were available locally (`2026-02-01` with the swing group, the
  `2026-09-19` and `2026-11-07` team-sport ARW folders), so nothing of the
  Done-when is left to a manual check.
- The mesh-fit gate was added: every closed reading of another face whose
  mesh `eye_offset` was above 0.1 was doubtful or wrong on a visual check
  (a mesh off a blurred profile at 0.401, the back of a head, a face behind
  a hand). Yaw does not separate them (a real closed profile sat at yaw
  -91), so there is no yaw limit. The judged face counts whatever its
  offset, to stay consistent with the `Eyes` row.
- The cap never bound (at most 5 faces counted on any frame of five
  folders); `MAX_FACES` stays 8.
- The plan's cost estimate for the AF frames' whole-image detection ("about
  16 ms") was low: the 640x448 YuNet run is about 71 ms on one thread, and
  it is most of the added cost (profile in measurement.md). On the
  coordinator's request the step was optimized before commit: every eye of
  the frame is now cut from the full JPEG in one pass
  (`partial::decode_crops`, `fullsharp::full_eye_sharpnesses`), which also
  makes the judged face's existing measurement cheaper; a 480x320 input
  was tried and rejected (it lost 23-32% of the counted faces). On one
  thread pass 2 per AF file went from about 82-92 ms to 163-184 ms (the
  first version was 189-240 ms).
- Per-eye partial decodes and one union-window pass differ only in the
  outermost column of the per-eye window: libjpeg's chroma upsampling at a
  cropped window's edge replicates instead of using the real neighbor. The
  eye cut never reaches that column, so the scores are identical (checked
  on real files), but a pixel-exact test must skip that column.
- Timing on 8 threads was unusable while other agents built in other
  worktrees (repeats varied 2x, once 124 vs 276 ms for the same binary);
  one-thread runs, the fastest of three, and a stage profile repeated twice
  were stable.
- `scan::Analysis` gained a field, so the CLI's `features_line` test, which
  builds an `Analysis` without `..Default`, needed `face_counts: None`; the
  core tests did not change. `cue`, `whole_image_cue` and `full_eye_sharp`
  in `scan.rs` became `#[cfg(test)]` wrappers over the new paths
  (`af_cue`, `whole_image_cue_rgb`, `full_sharps`) so their tests still
  exercise the production code.
- The caller now owns the preview decode: `candidate::focus_cue_rgb_unless`
  takes the decoded RGB, and `focus_cue_unless` decodes then calls it, so
  the other faces are meshed on the cue's decode.
- On AF frames the judged (crop-detected) face and its whole-image
  counterpart are matched by IoU above `NMS_THRESHOLD`; on `2026-09-19` one
  of 258 AF frames had an overlap at or under 0.3, which could count that
  face twice.
- A full `cargo test -p handpic-core --lib` with the default thread count was
  killed once (exit 137) while the session ended; with `--test-threads=4` it
  passed (371 tests).

## Step 1 (rework): the gate count in pass 2, the all-faces check as its own entry point

- The first version's counts are reproduced exactly by
  `scan::extract_face_counts`: a throwaway `eprintln!` of
  `Analysis::face_counts` in a build of `50a0cb4f` against the new
  `handpic-cli allfaces` on `2026-02-01`, `2026-09-19`, the first 300 ARW of
  `2026-11-07` and the first 100 of `2026-10-03`: 0 mismatches of 828 files.
  The entry point is the first version's pass 2 minus the score, so this was
  expected; the existing `features` columns and `facechoice --check` did
  not change either.
- A cold release build of `handpic-cli` took 2.5 min here once and 12 min
  another time (after a build of the `3918064c` sources from a scratch copy
  into the same target dir); incremental ones took 13-16 s.
- `cargo` from a scratch copy of an older commit (`git archive` into the
  scratchpad, `--target-dir` pointing at this worktree's `target/`) is a
  quick way to time "before" without touching the worktree or the stash.
- Testing "no 640x448 detection and no other mesh on an AF frame" needed
  test-only counters: `faces::model_runs` (per model input) and
  `eyes::MESH_CALLS`, both `#[cfg(test)]` thread-locals. A synthetic preview
  has no face, so the AF frame's cue judges none; `af_gate_count` is tested
  on a `Cue` given a face directly.
- The plan's cost estimate for the low-resolution run held (about 20 ms per
  AF file on one thread at 320x224, the scaled decode included), but its
  recall did not: at the gate's 0.8 score floor it counts about half the
  faces the 640x448 run counts on the team-sport frames, and the gate
  reaches N = 5 on 1 AF frame of the 682 measured against 49 with 640x448
  (measurement.md, "The low-resolution input"). 320x224 was kept as the
  plan's rule asks (the size picked against its cost); a larger input at
  0.8 does not fix it within budget (480x320: about 38 ms, 15 of 44 on the
  `2026-11-07` slice).
- What does fix it is the score floor: the small faces are found at 0.6 to
  0.8. 320x224 with a 0.6 floor counts about as many faces as 640x448 at
  0.8 (884 against 909 on the `2026-11-07` slice). The user decided on
  2026-11-24: 0.6 (`allfaces::GATE_LOW_RES_CONFIDENCE`) on the AF path's
  320x224 run only, 0.8 kept on the 640x448 path. A release build with it
  gave the scratch sweep's gate counts on every AF frame of the three
  folders and left every other `features` value and the `2026-02-01` gate
  counts unchanged.
- No measured folder holds a posed group photo on a body with an AF point;
  the frames the gate passes at 5 or more are basketball frames with
  players, a bench or spectators of the judged player's size. The posed
  7-person group of `2026-02-01` counts 4 even at 640x448 (three faces score
  0.64-0.75), so the default N = 5 does not pass it.
- The first version's deferred "pass 2 about doubles" is resolved by this
  rework (the 640x448 run left pass 2) and was taken off the list below.

- After `main` added the face mesh presence rules to the cue, the judged
  face already followed them (it counts only with the cue's EAR, which they
  withhold), but the other faces did not: `allfaces::measured` now turns
  down an untrusted mesh and one under `EYES_MIN_PRESENCE` as well. On the
  828 measured files it drops 9 faces (1 closed, 5 soft readings); see
  measurement.md.
- When the cue's mesh is under the presence floor but trusted, the judged
  face keeps its eye centers, so `full_sharps` still measures its eyes on the
  full JPEG though the counts ignore that reading (no EAR, no `Reading`).
  It costs a full read only when no other face is measured; left as is.

## Step 2: Store the gate count and the counts, and pass 3 in the engine

- `FACES_VERSION` was already 13 on `main` when this step started (the
  no-AF luma change moved it after the plan was written), so the bump that
  fills `faces_group` is 14, not the plan's 12.
- The migration tests from v10 to v26 build their old database by dropping
  columns from a fresh one, so each gained the five `DROP COLUMN`s of the
  new columns; without them `prepare`'s `ADD COLUMN` fails and the index is
  discarded.
- `StoredEyes::new` kept its five arguments and leaves the four new fields
  `None`; `indexed_file` and `run_faces_scan` set them by struct update. The
  app's `new` callers compile unchanged; only the two full `StoredEyes`
  literals in its tests (`filter.rs`, `mark.rs`) needed the fields.
- One type, `AllFacesReady`, is both `write_face_counts`'s row and the
  runner's progress row, since the two hold the same values.
- `extract_face_counts` returning `Ok(None)` (the whole-image detection or a
  decode failed) is stored like an unreadable file, no counts at
  `ALLFACES_VERSION`, but not counted as an error, the way pass 2 treats a
  failed decode.
- An error row and a JPEG row stay off `allfaces_todo` because nothing gives
  them a gate count: `faces_todo`'s reset and the first pass's rewrite
  clear `faces_group`, so the query needs no extension check of its own.

## Step 3: Run pass 3 in the app and show it in the status line

- `Priority` has no level below `Lowest` on any platform (Windows
  `THREAD_PRIORITY_LOWEST`, macOS utility at -5, Linux nice 10), and adding
  one is a core change, so pass 3 runs at `Priority::Lowest` like pass 2
  (`Priorities::allfaces`) and comes after it by order only.
- `FacesDone` used to mean "the scan is over" (`finished`, the held run's
  drain, the prefetch). To keep a folder without frames at the gate exactly
  as before and avoid a moment between the passes where `scanning()` is
  false, the scan thread lists `allfaces_todo` before it sends `FacesDone`
  and puts the count in it (`checking`): 0 ends the scan there and no pass 3
  event follows; otherwise the status turns to `checking faces 0/N` at once
  and `AllFacesDone` ends the scan.
- The status line's name for pass 3 is `checking faces`; `get_status`
  reports the same string as its `scan.pass`, as it does for the other two.
- A pass 3 row is laid over the facts with its own `pending_counts` after
  `pending_faces`, so a snapshot read before it was written still gets it.
  The refilter on a pass 3 row runs only while an `Eyes` item is checked,
  the section Step 6 adds the counts' items to; nothing filters on the
  counts yet.
- The scan-level test seeds rows past pass 2 through the engine's public
  `write_batch` and `write_faces` with a `faces_group`, so pass 1 and pass 2
  have nothing to do; the unreadable file gives pass 3 a row with no counts,
  which still reaches the channel and leaves `allfaces_todo` empty.
- Per the user's standing rule (2026-11-24), `mise run ci:local` and local
  cargo builds were not run; the build, clippy and the tests are left to
  the PR's GitHub checks. Only `mise run fmt` ran.

## Step 4: The group threshold setting and the rescan trigger

- gpui-component's settings number field (`SettingField::number`) calls
  its setter on every `InputEvent::Change` with the parsed value clamped
  into `min..=max`, so typing `15` would first commit `2` (the `1` clamped),
  and a lowered commit re-lists the folder. The field is a custom
  `SettingField::render` over the view's own `InputState` drawn as
  `NumberInput`, committed on Enter / blur like the pattern field.
- The `InputState` carries no `step`, `min` or `max`: with them a blur
  clamps the text before `InputEvent::Blur` (so `25` would commit `20`
  where the plan wants it refused), and a step button replaces the text
  silently (`replace_text_in_range_silent`, no `Change`). Without a step
  `apply_number_step` emits `NumberInputEvent::Step`, which the view takes
  (`settingsmodal::group_stepped`) and commits at once within 2 to 20.
- The trigger on lowering is the watcher's path (`changed`): a re-listing
  now when idle, else once the listing or scan under way ends. `Scan::start`
  takes the threshold at the scan's start, so a value lowered during a
  scan reaches pass 3 through that re-listing. `Scan::start` went to eight
  arguments, so it carries `#[allow(clippy::too_many_arguments)]` like the
  engine's runners.
- The visibility rule (`group::shown`) has no production caller until
  Steps 5 and 6; it and `Counts` carry `#[cfg_attr(not(test),
  allow(dead_code))]` (an `expect` could go unfulfilled on `Counts` if the
  allowed function counts as a live root).
- The default stays 5, as the user decided; the deferred item about the
  7-person `L1005161.DNG` counting 4 is left as it was.
- `python3` from Git Bash hung on a heredoc edit (CLAUDE.md warns of this);
  the edits went through the Edit tool instead.
- Per the user's standing rule (2026-11-24), `mise run ci:local` and local
  cargo builds were not run; the build, clippy and the tests are left to
  the PR's GitHub checks. Only `mise run fmt` ran.

## Step 5: Show the counts in the meta pane and on the strip cell

- The strip cell already had a closed-eyes mark from the
  `eyes-mark-tier-mcp` plan, built for this step to widen: its one rule
  (`mark::closed_eyes_mark`) now also takes the group threshold and turns on
  for a closed count of at least 1 through `group::shown`. No new icon, slot
  or `StripCell` field. "Never on a frame under the gate" holds for the
  counts' part; the judged face's own closed state still marks any frame, as
  before this step (the single judged-face behavior does not change).
- Both readers run at render time with `self.settings.group_min_faces`, so
  the pass 3 rows (`AllFacesProgress`, then the event handler's
  `cx.notify()`) and a threshold change (`set_group_min_faces`'s
  `cx.notify()`) redraw them with no extra plumbing; that made the doc
  comment on `set_group_min_faces` true, and it now names the readers.
- With real callers in place, `group::Counts` and `group::shown` lost their
  `#[cfg_attr(not(test), allow(dead_code))]`; every field of `Counts` is read
  (`judged` and `soft` by the meta pane, `closed` by both).
- The count rows sit at the end of the `Analysis` group, after `Sharpness`:
  `Similar faces`, `Faces judged`, `Faces with closed eyes`, `Faces with soft
  eyes`.
- Per the user's standing rule (2026-11-24), `mise run ci:local` and local
  cargo builds were not run; the build, clippy and the tests are left to the
  PR's GitHub checks. Only `mise run fmt` ran.

## Step 6: Filter by anyone's eyes, in the menu and over MCP

- `EyesItem::of` mapped a file to exactly one item, a partition; with
  `AnyoneClosed` / `AnyoneSoft` a frame can match several, so `of` became
  `EyesItem::matches` and `passes` takes an `EyesFacts` (the stored eye
  state and the `group::shown` counts) in place of the bare
  `Option<EyeState>`. The view builds it with the threshold in force, so a
  threshold change's existing refilter and pass 3's (already gated on an
  `Eyes` item being checked) cover the new items with no extra plumbing.
- The stored and MCP names are `anyone_closed` / `anyone_soft`; the engine's
  `mcp::EyesItem` moved from `rename_all = "lowercase"` to `snake_case`,
  which leaves `open`, `closed` and `unknown` as they were.
- `companion.rs` needed no production change: `filter_of`, `items` and the
  menu-click diff go through `filter::EYES` and `Toggle::Eyes(_)`, and the
  coverage table maps `Toggle::Eyes(_)` as a whole.
- Per the user's standing rule (2026-11-24), `mise run ci:local` and local
  cargo builds were not run; the build, clippy and the tests are left to the
  PR's GitHub checks. Only `mise run fmt` ran.

## Deferred issues (todo candidates)

- On ARW team-sport folders 24-48% of the confident faces are under
  `EYES_MIN_FACE` and go unjudged (measurement.md, item 1); meshing them from
  the full JPEG (the plan's Option B) is the follow-up the plan names.
  Files: `crates/core/src/allfaces.rs`, `crates/core/src/eyes.rs`.
- A judged crop face whose whole-image box overlaps it at or under
  `NMS_THRESHOLD` IoU is counted twice (1 of 258 AF frames on
  `2026-09-19`). A matching rule by eye landmarks instead of IoU would close
  it. Files: `crates/core/src/allfaces.rs` (`candidates`).
- The default N = 5 passes no posed group photo of the measured folders:
  the 7-person `2026-02-01/L1005161.DNG` counts 4 at the 0.8 floor (three
  faces at 0.64-0.75; that frame has no AF point, so the AF path's 0.6
  floor does not apply). Revisit the default (Step 4).
  Basis: Step 1 rework (measurement.md). Files: the app's setting (Step 4),
  `crates/core/src/allfaces.rs`.
- The size band does not tell a group from a background row at the same
  distance: `2026-11-07/_DSC1611.ARW` counts 9 at 640x448, one player and
  eight spectators on the bench behind. A frame like that passes the gate
  and pays pass 3. Basis: Step 1 rework's look at the frames at 5 or more.
  Files: `crates/core/src/allfaces.rs` (`gate_count`).
- Pending manual check (Step 4, any desktop platform, the running app): open
  the settings (`ctrl+,`), Culling page. Expected: `Faces in a group photo`
  under the auto-advance switch shows 5 on a fresh store; typing `3` and
  Enter (or Tab away) applies it and, with a folder open, the status line
  shows `checking faces` only when some frame's `faces_group` is 3 or 4;
  `-` / `+` step by one within 2 to 20 and apply at once; typing `25` or
  `1` and Enter shows the refusal in the status line and the field goes
  back to the value in force; Escape in the field commits it and closes;
  typed digits are not taken as shortcuts; the value survives a restart.
  The step's checkbox was ticked on the automated criteria (unit and
  scan-level tests, left to GHA). Basis: Step 4's number field, which no
  test drives. Files: `crates/app/src/settingsview.rs`,
  `crates/app/src/main.rs` (`set_group_min_faces`).
- Pending manual check (Step 5, any desktop platform, the running app, a
  real group photo folder such as the `2026-11-07` team-sport ARW folder):
  open the folder and let the scan finish (`checking faces` gone from the
  status line). Expected: on a frame whose meta pane `Similar faces` is at
  or above `Faces in a group photo`, the `Analysis` group ends with `Faces
  judged`, `Faces with closed eyes` and `Faces with soft eyes` that agree
  with what the photo shows (blinks counted, a soft back row counted), and
  the amber `eye-off` mark sits on the strip cell exactly when that closed
  count is at least 1 or the judged face is closed; on a frame under the
  number only `Similar faces` shows and no mark comes from the counts;
  raising the number past a marked frame's `Similar faces` hides its count
  rows and its counts' mark at once, lowering it back brings them back. The
  step's checkbox was ticked on the automated criteria (unit tests, left to
  GHA). Basis: Step 5's meta rows and strip mark, which no test paints.
  Files: `crates/app/src/meta.rs`, `crates/app/src/mark.rs`,
  `crates/app/src/main.rs`.
- Pending manual check (Step 6, any desktop platform, the running app, a
  real group photo folder such as the `2026-11-07` team-sport ARW folder):
  open the folder, let the scan finish (`checking faces` gone from the
  status line), then check `Filter ▾` > `Eyes` > `Anyone closed`. Expected:
  the strip narrows to the frames whose meta pane `Faces with closed eyes`
  is at least 1 (each a real blink on some face, or a false closed reading
  worth noting), no frame whose `Similar faces` is under `Faces in a group
  photo` stays; `Anyone soft` likewise against `Faces with soft eyes`;
  adding `Open` widens the strip to those frames plus the open-eyed ones;
  raising the number in the settings past a shown frame's `Similar faces`
  hides it at once; the checks survive a restart. The step's checkbox was
  ticked on the automated criteria (unit tests, left to GHA). Basis: Step
  6's filter items, which no test drives through a real scan. Files:
  `crates/app/src/filter.rs`, `crates/app/src/menu.rs`,
  `crates/app/src/main.rs`.
