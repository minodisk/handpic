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

# Close four measured todo items (lens trim, burst ranking check, no-AF decode, JPEG preview)

## Purpose

Four `todo.md` items have been waiting for a measurement on real data or a
small fix:

- "Core: the lens name of some ORFs keeps a trailing space"
- the sub-item "Spot-check whether the sharpness ranking within a burst
  changes ..." of "Core: face/eye-aware focus check for culling"
- "Core: the no-AF-point path decodes the preview up to three times"
- "App: a JPEG folder's preview decodes 15-20x longer than an ARW's"

This plan closes each one: the fix where one is warranted, the measurement on
`D:\photos` / `D:\Photos\tests` recorded where the repository keeps such
numbers, and the item removed from `todo.md`.

Rules for every step:

- The step's PR removes only its own `todo.md` item. Other sessions edit other
  sections of `todo.md` concurrently: touch no other item, and in Step 2 remove
  only the one sub-item, leaving the rest of the face/eye section as it is.
- Do not touch `docs/plans/20261123-all-faces-check`.
- Run Python through `mise x -- python` (a bare `python` hangs in Git Bash on
  this host), and keep scripts to the standard library.
- The user approved the recommended option of every trade-off below; decide
  by those recommendations without asking again, except where a trade-off
  says the result is the caller's decision.

## Steps

- [x] Step 1: Trim the trailing spaces of the displayed lens name
  - Done when:
    - A lens stored as `LEICA DG SUMMILUX 25/F1.4 ` (trailing space) comes out
      of `handpic_engine::exif::exif` without it, so the meta pane's `Lens` row
      and the filter menu's `Lens` section show one entry for that lens
    - A unit test in `crates/engine/src/exif.rs` covers a lens with trailing
      spaces, and the existing `a_blank_lens_is_no_lens` test still passes
    - `todo.md` no longer holds the section "Core: the lens name of some ORFs
      keeps a trailing space" (heading, Background and TODO), and nothing else
      in `todo.md` changed
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Recommended site (see Trade-offs 1): `crates/engine/src/exif.rs`, where
      `Exif.lens` is built from `shot.lens_model` (today
      `.filter(|l| !l.trim().is_empty())`): trim the end, then drop a blank
      one. The index keeps storing `shot.lens_model` as read
      (`crates/engine/src/index.rs` writes it raw), and both the meta pane
      (`crates/app/src/meta.rs`) and the filter's `Group::Lens`
      (`crates/app/src/exif.rs`) read the lens through `exif()`, so this needs
      no `EXTRACTOR_VERSION` bump and covers ARW / DNG as well as the
      `read_exif_ifd` formats.
    - `EXTRACTOR_VERSION` stays 13 on this route. Do not change
      `crates/core/src/exif.rs` or `arw.rs`.
    - Verify during implementation that no other consumer shows the index's
      raw `lens` without going through `exif()` (grep `lens_model` in
      `crates/app` and `crates/engine/src/mcp.rs`); if one does, route it
      through the same trimmed value rather than adding a second trim.

- [x] Step 2: Add the eye-midpoint score to `handpic-cli features` and spot-check the within-burst ranking of face-tracked Sony frames
  - Done when:
    - `handpic-cli features` prints two more columns, appended after
      `eyes_ms` so existing column positions stay: whether the frame is a
      face-tracked one (`sharpness::eye_af_frame` gives a frame) and the
      sharpness of the window between the eyes of the cue's face (`-` when
      there is no such face or it is under `sharpness::FACE_CONFIDENCE`);
      `FEATURES_HEADER`, the doc comment of `features` and the
      `features_line` test cover them
    - `burst_spearman.py` in this plan folder groups a `features` table into
      bursts and prints, per comparison, the number of bursts used, the
      median, 10th percentile and minimum Spearman rank correlation per burst,
      and the share of bursts whose top-ranked frame is the same
    - The check was run on real Sony folders with face tracking and its result
      is recorded in `burst-rank.md` in this plan folder: the folders, the
      number of face-tracked frames and of bursts used, the numbers above, the
      bursts with the lowest correlation by file name, and a one-line
      conclusion (negligible or not, against the bar in Trade-offs 2)
    - `todo.md` no longer holds the sub-item "Spot-check whether the sharpness
      ranking within a burst changes ..." (its five lines) under "Core:
      face/eye-aware focus check for culling"; every other line of that
      section and of `todo.md` is unchanged
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/cli/src/main.rs` only for code. No change to
      `crates/core`: the eye-midpoint window is path 2 of
      `sharpness::score_preview`, reached with
      `score_preview(preview, None, None, &[face])` for the cue's face
      (`analysis.cue.face`, stored coordinates). Guard on
      `face.score >= FACE_CONFIDENCE` in the CLI, because below it
      `score_preview` silently falls to the sharpest tile.
    - Compute the two new values outside the `analysis_ms` and `eyes_ms`
      timers, so the existing timing columns keep their meaning (Step 3 and
      `docs/plans/20261123-all-faces-check` both read `analysis_ms`).
    - The `sharpness` column is already the stored score, which on a
      face-tracked frame is the AF-frame window; the comparison is that column
      against the new one, over frames where the flag column is set.
    - Burst grouping in the script: per folder, frames in capture order
      (`capture_time` then `subsec`), consecutive frames no more than 1000 ms
      apart, as `crates/app/src/burst.rs` (`BURST_GAP_MS`, `group_bursts`)
      does. Use only bursts with at least 3 frames carrying both values (a
      2-frame Spearman is always +1 or -1). Average ranks for ties.
    - Give the script a two-table mode as well (the same column from two
      `features` runs, joined on folder and file), because Step 3 uses it that
      way.
    - Folders: `D:\photos\2026\2026-09-19` (282 ARW) to start, then further
      Sony ARW date folders under `D:\photos\2026` or the
      `D:\Photos\tests\*-focus-sample` folders until at least 30 bursts
      qualify; if fewer exist, record the count reached. Build with
      `cargo build --release -p handpic-cli`.
    - Precedent for a script kept beside its plan:
      `../_archived/20261108-full-size-eye-check/rank.py`.
    - State in `burst-rank.md` that this is not the exact pre-#334 pipeline
      (Trade-offs 6).

- [ ] Step 3: Score a faced no-AF-point file from the cue's RGB decode instead of a third decode
  - Done when:
    - The per-file `analysis_ms` of faced no-AF files was measured before and
      after with `handpic-cli features <dir> 1` (one thread, release, warm
      cache, runs alternated) over `D:\photos\2026\2026-02-01` and
      `D:\photos\2026\2026-09-05`, and a file with no face was confirmed
      unchanged
    - The within-burst ranking was checked on those two folders with Step 2's
      `burst_spearman.py` in its two-table mode (old `sharpness` against new),
      over the faced no-AF frames
    - Both results are recorded in `no-af-decode.md` in this plan folder, and
      the per-file cost in `docs/humans/performance.md` ("Which pass carries
      which cost", and the "Focus candidate pass" text if it states the
      decode count) with the same change in `docs/humans/performance.ja.md`
    - If adopted (Trade-offs 3): a faced no-AF file is decoded twice, not
      three times, in `scan::extract_analysis`; a file whose RGB decode or cue
      fails still gets a score as before; the score of every other kind of
      file (trusted AF point, eye-AF frame, no face) is bit-for-bit what it
      was; unit tests cover the new scoring entry point; `FACES_VERSION` is
      bumped by one with its doc comment extended (the number, why, and the
      measured Spearman); `docs/agents/shared-crates.md`'s note on the
      no-AF branch's cost (the "+28 ms" paragraph) and the module docs of
      `sharpness.rs` / `scan.rs` say what is true now
    - If not adopted: no code or version change; the measurement and the
      reason are in `no-af-decode.md` and one sentence in both performance
      docs
    - Either way `todo.md` no longer holds the section "Core: the
      no-AF-point path decodes the preview up to three times", and nothing
      else in `todo.md` changed
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged (the script). No CLI change is needed here:
      run `features` with the binary built from `main` and with the branch's,
      and compare the `sharpness` columns.
    - Take the baseline numbers and both TSVs before changing code; keep the
      old binary (copy the exe aside) so runs can alternate.
    - Files: `crates/core/src/sharpness.rs` (split the part of `score` after
      the grayscale decode into a function that takes a luma buffer, width
      and height; `score_preview` keeps its behavior by calling it),
      `crates/core/src/scan.rs` (`whole_image_cue` holds the full-size RGB
      inside its closure today, so it has to hand back either
      `candidate::luma` of it or the score itself; `extract_analysis_unless`
      then skips `score`'s decode for that file),
      `crates/engine/src/index.rs` (`FACES_VERSION` and its comment).
    - The window's face stays `sharpness::chosen_face` (the highest-scoring
      confident face), which is not always the cue's `eyes::judged_face` (the
      largest confident one). Do not change which face is scored; only where
      the pixels come from. Whenever a judged face exists a chosen face does
      too, so the RGB decode exists exactly on the files that take the eye
      window path.
    - Known from `docs/humans/performance.md` ("DCT-scaled decode for the
      whole-image search"): mozjpeg's grayscale output is the JPEG's Y
      channel, while `candidate::luma` is the BT.601 luma of the decoded RGB,
      so scores will differ slightly. That is why the ranking check and the
      version bump are part of this step.
    - Baseline to compare with: about 94 ms to 122 ms per faced no-AF file
      (`../_archived/20261010-af-eye-good-bad-cascade/learnings.md`, Step 2).
    - The files with a trusted AF point also decode twice (RGB for the cue,
      grayscale for the score). That is outside this item; do not change it
      here.
    - Before starting, check whether `docs/plans/20261123-all-faces-check`
      Step 1 or 2 has merged or is open (Trade-offs 4): take the then-current
      `FACES_VERSION` plus one, and rebase onto its `scan.rs` if it landed.

- [ ] Step 4: Measure a JPEG folder's page time in the app and apply the smallest fix the numbers call for
  - Done when:
    - The app's `page` timing lines were taken on JPEG-only folders with the
      release build (`mise run app:release -- <folder> --auto N | cat` in Git
      Bash, see `../../agents/app.md`): `keypressToPixels`, `read`, `decode`,
      `orient`, the `px=` and `decoded=` sizes, three runs each, with the
      prefetch ring on and with `--no-prefetch`, next to an ARW folder in the
      same session as the ruler
    - The result is recorded in `jpeg-preview.md` in this plan folder and as a
      JPEG row (or short paragraph) under "Keypress to pixels (GPUI)" in
      `docs/humans/performance.md`, with the same change in
      `docs/humans/performance.ja.md`
    - The outcome chosen under Trade-offs 5 is carried out: either no code
      change (the numbers recorded as the reason), or the one decoder change,
      measured before and after by the same method and described in
      `../../agents/app.md`'s "The fit view decodes at the canvas's DCT scale"
      section
    - `todo.md` no longer holds the section "App: a JPEG folder's preview
      decodes 15-20x longer than an ARW's" (or, only if the caller chose so
      under Trade-offs 5, holds a narrowed replacement), and nothing else in
      `todo.md` changed
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - The todo's own remedy is already in the app: since #1196, `load` in
      `crates/app/src/main.rs` decodes every preview at
      `fitscale::scale_for`'s `n/8`, a view-only folder's whole JPEG included
      (`reader::read_preview` returns the whole file for a `.jpg`), and the
      prefetch ring holds the scaled neighbours. The 141-197 ms in the todo
      is a Tauri-era backend figure for a full 8/8 decode. So measure first;
      do not assume a fix is needed.
    - Known from `../../agents/app.md`: the entropy decode does not shrink
      with the output scale (3/8 cut a 13 MP RAF preview's decode by about a
      third only), and the gain that mattered there was the texture upload.
      Expect the same shape on a 15-21 MP JPEG.
    - Folders: `D:\Photos\tests\2026-09-28-gui-check-jpeg` (10 DxO exports)
      and `2026-09-28-gui-check-jpeg-500`; an `Output` folder under
      `D:\photos\2026\<date>` as a second real case; `D:\photos\samples\JPG`
      holds camera JPEGs of many makes if a non-DxO case is wanted. Record
      whether the files are baseline or progressive (exiftool
      `-EncodingProcess`), since progressive decodes slower.
    - The first `page` after launch decodes whole (the canvas is unknown
      before the first paint), so drop it from the statistics.
    - If a decoder change is tried, the candidates are settings on the
      `mozjpeg::Decompress` in `load` (fancy upsampling off, a faster DCT
      method); check what the `mozjpeg` crate version in
      `crates/app/Cargo.lock` exposes before planning around one. Any such
      change applies to the fit view only, never to `zoom.rs`'s 1:1 crop, and
      whether it applies to RAW previews as well is decided by the ARW / RAF
      numbers of the same session.
    - Run under `--auto`, which reads no stored panes or window bounds, so the
      canvas size is the default window's; state it in the results, since the
      chosen `n/8` depends on it.

## Trade-offs and risks

1. **Where the lens is trimmed (Step 1).** (a) At display, in
   `crates/engine/src/exif.rs`: one line, every format fixed, no
   re-extraction; the index keeps the padded string, which only matters to a
   consumer that bypasses `exif()`. (b) At the readers with
   `EXTRACTOR_VERSION` 14: the stored value is clean, but every folder's
   pass 1 re-runs on its next open for a cosmetic fix. Chosen: (a).

2. **What Step 2 does with a ranking change.** The step is a check, not a
   change of the score. Bar for "negligible": median per-burst Spearman at
   least 0.9 and the same top frame in at least 80% of bursts. If under the
   bar: (a) keep the AF-frame scoring and record the numbers, with a todo
   candidate in `learnings.md` for the wrap-up; (b) return face-tracked frames
   to the eye-midpoint window with a `FACES_VERSION` bump. Chosen: (a). The
   good-photo mark reads `full_eye_sharp`, not this score.

3. **Whether Step 3's change is adopted.** Adopt when the median per-burst
   Spearman is at least 0.95 with no burst under 0.8 and the saving is at
   least about 5 ms per faced no-AF file; otherwise record "measured, not
   adopted" and close the item without code. A middle result (large saving, a
   few low-correlation bursts) is the caller's decision; list those bursts by
   file name in `no-af-decode.md`.

4. **Collision with `docs/plans/20261123-all-faces-check`.** That plan's
   Step 1 changes `extract_analysis_unless` and shares the cue's RGB decode in
   the same region of `crates/core/src/scan.rs`, and its Step 2 bumps
   `FACES_VERSION`. Never run Step 3 in parallel with that work: land it
   before all-faces-check Step 1 starts, or hold it until that step merges and
   rebase onto it. The second to merge takes the next version number.

5. **What Step 4 does after measuring.** (a) No code change, when warm paging
   p50 `keypressToPixels` is at or under the 13 MP RAF figure the app accepts
   today (about 65 ms); (b) a decoder-setting change in `load`, when it cuts
   `decode` by at least 20% with no visible loss at fit size; (c) a
   placeholder-first design, too large for this plan. Chosen: (a) or (b) by
   the numbers; if only (c) would help, record the numbers and replace the
   todo item with a narrowed one naming (c), after asking the caller.

6. **Step 2's old score is an approximation.** Before #334 the scan picked
   the face from a whole-preview detection; the check takes the cue's face
   nearest the AF point. Good enough for a spot-check; `burst-rank.md` says
   so.

7. **Real-data runs depend on this Windows host.** `D:\photos` and
   `D:\Photos\tests` exist only here, and the app's `--auto` timing needs a
   desktop session. Only same-session comparisons count.

## Progress

- (2026-11-23) Step 1 complete
