# todo

## Cross-cutting / other
### App: real-device Ctrl+C and window check of the app:watch task

#### Background

The `app:watch` mise task (the native-watch feature) runs the debug GPUI app under watchexec and rebuilds and restarts it on a `.rs`, `.toml` or `.json` change under `crates/app`, `crates/core` and `crates/engine`. It was checked by hand on Windows (Git Bash), but only from a non-interactive shell under `timeout`. That run covered the watchexec install, the launch, the restart on a source touch, no `os error 5` on the relink, no restart from the build's own output, and a folder argument holding spaces passed on every restart. The agent could see only processes and the app's log. Two things were never exercised: pressing Ctrl+C in a real terminal, and the window itself. A forced `taskkill /F` of `watchexec.exe` leaves `cargo` and the app running, because the job object is not kill-on-close. Ctrl+C is a different path (the console's ctrl event), so it needs a real terminal. Plan: `docs/plans/_archived/20261115-native-watch/plan.md`.

Files: `mise.toml` (`[tasks."app:watch"]`)

#### TODO

- [ ] On Windows (Git Bash), run `mise run app:watch` in a terminal and wait for the app's window. Press Ctrl+C in that terminal. Expect watchexec, `cargo` and `handpic.exe` all to exit (none left in Task Manager) and the prompt to return.
- [ ] On Windows (Git Bash), run `mise run app:watch`, save a `.rs` file under `crates/app/src`, and confirm the window reappears and repaints after the restart (the earlier check saw only the process and its log).

### App docs: correct the stale "only the settings modal and the tree's inline rename draw from gpui-component" text

#### Background

The `native-status-bar-decision` plan (`docs/plans/_archived/20261115-native-status-bar-decision/plan.md`) left the neighbouring sections of `docs/agents/app.md` untouched, and `native-status-bar` then rewrote its status-row sections (the sentence "only the settings modal and the tree's inline rename draw from it" went with them). One piece of text is still out of date since the gpui-component adoption work (dialogs, tooltips and scrollbars now draw from gpui-component too): the heading "gpui-component is initialized; the settings modal and the tree's inline rename draw from it". The heading is at about line 71 of `docs/agents/app.md`. Other places refer to the section by its exact heading name, for example the mention "table in "gpui-component is initialized"" near line 3227 of the same file. The native-progress-bar-reason feature fixed the same stale claim in the progress bar section's sentence and left the heading alone on purpose. Plan: `docs/plans/_archived/20261115-native-progress-bar-reason/plan.md`. Files: `docs/agents/app.md`.

#### TODO

- [ ] Reword the "gpui-component is initialized; ..." heading so it names the dialogs, tooltips and scrollbars as well, or make it generic. Update every quoted-title reference to the old heading name, found with `rg -n 'gpui-component is initialized' docs CLAUDE.md` (one is near `docs/agents/app.md:3227`), and check that `mise run lint` (lychee) still passes.

### Core: the `.dop` reader ignores a picked PhotoLab virtual copy

Found in `burst-keep-score` Step 1 (the inventory, `docs/plans/_archived/20261008-burst-keep-score/data.md`). `dop::read_flag` (`crates/core/src/dop.rs`, `locate`) reads only the first `Items` entry, so it returns `None` for 199 frames of `2026-06-05` and 228 of `2026-09-13-b` whose virtual copy (a later item) is picked in PhotoLab. Riffle shows those frames unflagged, so the strip's filter does not count them as picked. Whether Riffle should read any item's flag, and which item it writes, is a product decision.

Files: `crates/core/src/dop.rs`.

#### TODO

- [ ] Decide which `Items` entry's flag Riffle reads and writes, then change `read_flag` and its tests to match.

### App: the MCP `get_view` eyes summary carries no openness

`burst-keep-score` Step 4b made the meta pane's `Eyes` row show the openness (0-100 from the EAR) but kept `get_view`'s `state` / `probability` / `pose` fields as planned. Whether `EyesSummary` should carry the EAR or the openness is open.

Files: `crates/app/src/companion.rs` (the `eyes` of `get_view`).

#### TODO

- [ ] Decide whether `get_view`'s eyes summary carries the EAR or the openness, and add it with its test.

### App: real-device check of the 1:1 view on files with a malformed full-size JPEG

The `partial-decode-error-exit` work (`docs/plans/_archived/20261002-partial-decode-error-exit/plan.md`) made `decode_region` in `crates/core/src/partial.rs` return `Err` on a fatal libjpeg error instead of calling `exit(1)`. Unit tests (empty input, non-JPEG input, a JPEG truncated inside the header) and `riffle-cli crop` / `check` on the real files cover the core path. `crop` returned `libjpeg fatal error: Not a JPEG file: starts with 0x3c 0x44` for `NIKON_D70_Nikon.nef` and `... 0x80 0x03` for `CGO3P_YUN00007.dng`. `check` over the samples ran to its summary in 23.0 s. The app's 1:1 view (`z`, whose crop read calls the same `decode_focus_crop`) was never exercised on these files, and neither was what it shows for the error. Files: `crates/app/src/zoom.rs`, `crates/core/src/partial.rs`.

#### TODO

- [ ] On any platform, open `D:\photos\samples\NEF\NIKON_D70_Nikon.nef` and `D:\photos\samples\DNG\CGO3P_YUN00007.dng` in the app and switch to the 1:1 view. Expect the app to stay up and the crop read's error (`libjpeg fatal error: Not a JPEG file ...`) to surface however the 1:1 view shows a failed crop. Note what the UI shows; if it shows nothing useful, file a follow-up.

### Docs: the "Focus candidate pass" numbers in docs/humans/performance.md are missing the app's own scan/faces log lines

Step 3 of `docs/plans/_archived/20260925-focus-candidate/plan.md` asked for the
`scan extract` / `scan faces` log lines of one app open of the 2134-file
Sony folder to be recorded in `docs/humans/performance.md` "Focus candidate pass",
alongside the CLI numbers already there (`riffle-cli scan` and a CLI
subcommand removed since). The implementing agent could not drive the GUI, so only the CLI
numbers are recorded; `docs/humans/performance.md` says so in a note under the
section.

#### TODO

- [ ] Open the 2134-file Sony folder once in the app and record its
      `scan extract` / `scan faces` log lines in `docs/humans/performance.md`
      "Focus candidate pass", next to the existing CLI numbers.

### Core: cold first scan on an internal SSD is far slower than the extrapolation

A real cold first scan on Windows 11 (internal SSD, 22 threads, Sony ARW) costs ~16-19ms per file, ~82-97s extrapolated to 5000 files against the 30s target; see "Real folders on Windows" in docs/humans/performance.md. Excluding the folder from Defender did not help, and a warm-cache scan runs at ~1ms per file, so neither Defender nor CPU is the cause. The cause is unknown. A later data point (Windows 11, v0.2.0, 2026-09-22): a cold first scan after an index schema change took 25.5s on 3045 Sony ARW (~8.4ms/file, ~42s extrapolated to 5000), against the earlier 16-19ms/file; it is unknown whether the OS cache was cold for that run.

#### TODO

- [ ] Run `handpic-cli scan` on a cold real folder on Windows at thread counts 1 / 4 / 8 / 22 (cold each run) to separate IO concurrency from per-file cost, and compare the bounded 1MiB read against reading the whole file.

### App: a deep-row focus point still exceeds the 50ms budget

A focus point in a deep row of the unrotated JPEG measured 58-65ms keypress to pixels (n=2, before the #56 and #60 fixes); see "The 1:1 focus check path" in docs/humans/performance.md. Options are prefetching the neighboring files' crops (Phase 4's ring buffer) or a DCT-scaled placeholder; nothing is chosen. The measurement was the Tauri app's; the app's 1:1 view (`crates/app/src/zoom.rs`) reads the crop through the same `crates/core/src/partial.rs`.

#### TODO

- [ ] Retake the deep-row measurement on the app's 1:1 view, then decide between crop prefetch and a DCT-scaled placeholder.

### Docs: consider a guide for verifying Pillow pixel edits

`app-icon-enlarge-marks` hit two Pillow pitfalls while verifying an in-place
pixel edit to `crates/app/icons/source.png`: (1) `Image.getbbox()` defaults to
`alpha_only=True` on RGBA images (Pillow >= 9.5), so a "no difference" check
built on `ImageChops.difference(...).getbbox()` can silently pass while the
RGB channels differ — use `getbbox(alpha_only=False)` or a per-pixel scan;
(2) measuring a resized/pasted mark inside the padded crop box that produced
it clips the bbox and understates the mark's size — the measurement window
must be wider than the source crop.

#### TODO

- [ ] The next time a plan does Pillow-based pixel verification, check whether
      it hits the same two pitfalls. If it does, write
      `docs/agents/verifying-pixel-edits.md` (or a section in an existing
      guide such as `shared-crates.md`) covering both, linking
      `docs/plans/_archived/20260921-app-icon-enlarge-marks/learnings.md` for
      the concrete numbers. If it does not recur, drop this item.

### App: the macOS / Linux manual checks for Move to Trash are still open

The strip's `Move to Trash…` moves files through the engine's mover and records `trash::Runs` runs, and that mover has never been run by a human on macOS or Linux (the app checks in `### App: real-device checks of the folder tree, the strip's selection and right-click menu, and Move to Trash undo / redo` are Windows only). On macOS the mover is `trash::trash_file` (`crates/engine/src/trash.rs`, `undo-trash-rejected` #547), which calls `NSFileManager.trashItemAtURL` directly rather than the `trash` crate's `DeleteMethod::NsFileManager`; one point is specifically unverified: whether the Trash's "Put Back" entry is actually created, since the `trash` crate documents that on some macOS systems files moved through `NSFileManager` get no "Put Back" entry (trash-rs#14); dragging them out of the Trash still restores them. Files: `crates/engine/src/trash.rs`, `crates/app/src/trash.rs`, `crates/app/src/main.rs`. Sources: `docs/plans/_archived/20260928-undo-trash-rejected/learnings.md` Step 2 (#547) and `docs/plans/_archived/20261113-gpui-tree-trash/plan.md`.

#### TODO

- [ ] On macOS, right-click a strip cell and pick `Move to Trash…`: the dialog asks `Move 1 file to the Trash?` with `Move to Trash` / `Cancel`; Cancel leaves the folder untouched; confirming moves the RAW plus its `.xmp` and `.ARW.dop` to the Trash and the strip shows the neighbouring file; a "Put Back" from the Trash restores the file with its judgment, and if no "Put Back" entry exists, dragging it out does.
- [ ] Verify the same flow on Linux (the `trash` crate's freedesktop backend has never been run here).
- [ ] On macOS, move a rejected file to the Trash, then undo (`meta+z`): the file comes back with the reject flag, and a file with the same name placed at the original location beforehand is reported (`already exists at the original location`) and not overwritten. (Source: `docs/plans/_archived/20260928-undo-trash-rejected/learnings.md` Step 2, #547.)

### Core: face/eye-aware focus check for culling

Face/eye-aware detection and scoring (`crates/core/src/faces.rs`, `crates/core/src/sharpness.rs`) now runs at scan time: sharpness is scored on the Sony eye-AF frame, else on the AF point, else on the eyes of a detected face when there is no trusted AF point, else on the sharpest tile; an AF point off the face no longer scores a bystander's eyes (see `docs/plans/_archived/20260922-face-aware-sharpness/` and `docs/plans/20260924-face-catch-state/`). Without a trusted AF point the whole-preview search now runs at a 640x448 input on a DCT-scaled decode, which raised small-face recall at about 1.7x the earlier per-file cost; closed eyes were found to need a second model (see `docs/plans/_archived/20261005-face-detection-recall-cost/`), which now judges the shown file's eyes on demand, outside the scan, for the meta pane's `Eyes` row (see `docs/plans/_archived/20261007-closed-eyes-detection/`). What remains:

#### TODO

- [x] MakerNote check (exiftool 13.59, 2026-09-22): Sony α7 V ARW writes
      `AFTracking` + `FocusFrameSize` + `FocusLocation` but no per-face list;
      `AFAreaMode` is enciphered; Leica M11-P DNG writes only
      `FocusDistance`. The scan now uses the Sony AF frame and skips detection
      when face tracking was engaged
      (`docs/plans/20260922-sony-eye-af-window/`).
- [x] Measured on Linux WSL2 (not the Mac): detection latency on real
      α7 V ARW / M11-P DNG previews and `riffle-cli scan` before/after adding
      detection and the AF-frame skip; see `docs/humans/performance.md` "Face
      detection cost".
- [x] Optionally, detect closed eyes. YuNet's five landmarks carry no
      eyelid points, so it needed a second model. Four were surveyed on
      2026-10-07 against 504 hand-labeled faces (`model-survey.md`,
      `eyes-truth.md`); none fits the scan's second pass. Adopted: MediaPipe
      Face Landmarker v2 (Apache-2.0, 4.9 MB ONNX converted from Google's
      TFLite, `crates/core/src/eyes.rs`), judging closed eyes by the eye
      aspect ratio (closed iff at most 0.137) of one face, the one nearest a
      trusted AF point, else the largest face at or above 0.8, at or above a
      60 px face side, on demand for the shown file only (`eyes_of`), shown
      as the meta pane's `Eyes: Closed (NN%)` / `Open (NN%)` row. Cost:
      `riffle-app` 47.5 -> 52.5 MB, `riffle-cli` 24.6 -> 29.6 MB; ~35 ms
      per face (p95 41 ms), the whole call 80-92 ms on an ARW with an AF
      point and 140-156 ms on a DNG without one. Accuracy: face AUC 0.974,
      precision 0.94 / recall 0.86 at the threshold. See
      `docs/plans/_archived/20261007-closed-eyes-detection/` and
      `docs/humans/performance.md` "Closed-eyes judgment on demand
      (Windows 11)".
- [ ] Judge every face at or above the 60 px floor, not only the AF-nearest
      (or largest) one, so a bystander's blink shows too. Each face is one
      more model run (~35 ms; ~1.8 faces per file on the 2134-ARW folder,
      more in groups). Needs either a row per face or
      a worst-of summary (`Eyes: Closed (1 of 3)`), a pattern the meta pane
      does not have, and a check of the accuracy on non-subject faces, which
      `eyes-truth.md` did not label. Files: `crates/engine/src/ondemand.rs`
      (`read_eyes`), `crates/core/src/eyes.rs` (`judged_face`),
      `crates/app/src/meta.rs`.
- [ ] Use the model's face presence output (`Identity_1`, a logit the
      `eyes` module ignores) to drop false face detections before judging
      the eyes. Needs a threshold calibrated on labeled faces: the `x`
      labels of `docs/plans/_archived/20261007-closed-eyes-detection/eyes-truth.md`
      (no eye to judge, including false detections) can seed it. Files:
      `crates/core/src/eyes.rs`, `docs/agents/tract-onnx-inference.md`.
- [ ] Mark closed eyes in the strip. Pass 2 now stores the EAR for the face
      the cue meshes and the filter menu has an `Eyes` section (the
      `mesh-eyes-index` work), but no tile shows a closed-eyes mark yet; the
      stored `eyes` state (`open` / `closed` / `unknown`) is on `Focus`, so
      the remaining work is the mark itself. Files:
      `crates/app/src/strip.rs`, `crates/app/src/mark.rs`.
- [x] Estimate the head pose of the judged face. MediaPipe's face geometry
      pipeline (perspective unprojection at its 63 deg camera, weighted
      Procrustes onto the canonical face) is ported to
      `crates/core/src/pose.rs` and runs on the points of the `eyes_of`
      model run, shown as the meta pane's `Head pose` row (yaw, pitch,
      roll; right, up and clockwise positive). On demand only, nothing in
      the index, so there is no strip filter for it. Cost: 8.2 µs per face,
      no change to the `eyes_of` total. Accuracy on 159 labeled faces: the
      sign right on 94% (yaw), 95% (pitch), 15 of 18 (roll); the yaw class
      on 68%, the gap at the frontal / oblique boundary. See
      `docs/plans/20261007-head-pose/` and `docs/humans/performance.md`
      "Closed-eyes judgment on demand (Windows 11)".
- [ ] Derive the good-photo rule in the engine, in
      `crates/engine/src/index.rs`, so the MCP `get_view` tool can carry it.
      The rule lives only in `crates/app/src/mark.rs` (`photo_tier`) and
      `get_view` says nothing about it. Done when the index derives the tier
      from the stored columns with the thresholds in one place (the app
      reads it instead of recomputing), `get_view` carries it for the
      current file, and the boundary tests of `mark.rs` move with the rule.
      Files: `crates/engine/src/index.rs`, `crates/app/src/mark.rs`,
      `crates/app/src/companion.rs`.
- [ ] Measure the good-photo mark's precision on labels. Decision B rests
      on the user's stars and the mark rate; no labeled set was made. Done
      when 100-150 faced AF frames drawn stratified around the cuts and at
      random are labeled blind (`ok`, `miss` with `blur` / `focus` /
      `closed`, `turned`, `unsure`; the agent first, the user's review
      final) and the precision and coverage of the mark at its cuts are
      written with a Wilson interval, with `full_eye_sharp >= SOFT_BELOW`
      alone as the baseline (the procedure is in `docs/plans/20261008-burst-keep-score/plan.md` Step 5). Files: the plan
      folder of that work, `crates/app/src/mark.rs` if the cuts move.
- [ ] Re-check the good-photo cuts after the closed-eyes and head-pose label
      reviews (the items in the sections below). If the user's review
      moves `EYES_CLOSED_EAR` in `crates/core/src/eyes.rs` or changes the
      pose labels' frontal / oblique boundary, re-read `GOOD_EYE_EAR` (and
      `EYES_WIDE_OPEN_EAR` in `crates/app/src/meta.rs`, which anchors the
      openness against the core's `EYES_CLOSED_EAR`) and `GOOD_MAX_YAW` on
      the rated frames of `D:\photos\samples\ARW\good-mark-2026-10-09\`.
      Files: `crates/app/src/mark.rs`, `crates/app/src/meta.rs`,
      `crates/core/src/eyes.rs`.
- [x] Give the no-AF path a good-photo mark once it has a focus cue. Done
      in `docs/plans/_archived/20261010-af-eye-good-bad-cascade/` Step 2: pass 2 runs
      the cue, the EAR, the pose, the eye offset and the edge gap on the
      largest confident face of the whole preview when a frame has no
      trusted AF point (`FACES_VERSION` 9), and `Focus` carries the cue
      without a point, so the same cuts mark it. The check of those cuts on
      starred no-AF frames is the next item.
- [ ] Check the good-photo cuts on starred frames with no trusted AF point.
      The cuts in `crates/app/src/mark.rs` (`GOOD_FULL_EYE_SHARP`,
      `GOOD_EYE_EAR`, `GOOD_MAX_YAW`, `GOOD_MAX_PITCH`, `MAX_EYE_OFFSET`,
      `MIN_EDGE_GAP`) were fitted on AF faces only; since
      `docs/plans/_archived/20261010-af-eye-good-bad-cascade/` Step 2 they also judge
      the largest confident face of a frame with no AF point. Run
      `handpic-cli features` over the Leica folders with `.dop` picks
      (`D:\photos\2026\2026-09-05` and the other DNG folders of the
      `20261008-burst-keep-score` data set), apply the same cuts to the
      `noaf` rows in a script, and report the share of faced frames marked
      good, the share of `.dop` picks among them against the base rate, and
      10 marked and 10 unmarked picks for the user to look at. Done when the
      numbers are recorded in a plan's `learnings.md` and the cuts are either
      kept or a separate no-AF cut is proposed with them. Files:
      `crates/app/src/mark.rs`.
- [x] Measure the AF eye in-focus probability over each eye's eyelid
      region from the face mesh instead of the window between the eyes.
      The scan's second pass runs MediaPipe Face Landmarker v2 on the face
      nearest the AF point, scores each eye's contour bounding box (margin
      0.5 of its longer side) with a refitted logistic and takes the sharper
      eye; when neither eye's region counts (under 24 px or without a clear edge) it scores the window as
      before (61% of the training frames, 60% of the 2134-ARW folder's
      faces). The head pose was measured as an eye rule and did not beat the
      sharper eye, so it is not used. Held-out AUC 0.754 -> 0.800, precision
      / coverage 89.1% / 95.3% -> 88.6% / 95.9%; pass 2 on the 2134-ARW
      folder 12.9 -> 21.5 s at 24 threads (+66%). `FACES_VERSION` 6. See
      `docs/plans/_archived/20261008-mesh-eye-focus/` and
      `docs/humans/performance.md` "Focus candidate pass".
- [x] Measure whether de-rotating the face crop by YuNet's eye line (the
      MediaPipe pipeline's roll step) fits the face mesh better. It does
      not, in any variant (always, dead bands of 10 / 20 deg, a YuNet
      eye-distance guard): always-rotate turns 7 off meshes on and 15 on
      meshes off on 131 readable labeled frames and drops the held-out AF
      eye AUC 0.800 -> 0.783; the share of AF frames with `eye_offset` over
      0.10 stays 28.7% on six folders. Where the rotation moves a fit,
      YuNet's two eye points are not on two eyes. The residual misfits are
      side faces (78% of the off meshes, weighted), then cut, rotated past
      90 deg, seen from above and not-a-face. Nothing in the scan or the
      index changed. See `docs/plans/20261010-mesh-roll/` (Decision,
      `compare.md`, `misfit-truth.md`).
- [ ] Make `eye_offset` reliable where YuNet's eye distance is under 0.20
      of the face box side, for `20261008-burst-keep-score`, which owns the
      `MAX_EYE_OFFSET` 0.10 cut. `docs/plans/20261010-mesh-roll/` found that,
      weighted to its six folders, about 27% of the frames over 0.10 have a
      mesh off the face and about 65% an on-face mesh compared against YuNet
      eye points that are off the eyes (profiles whose two points sit
      together on one eye or the nose); frames under 0.20 eye distance are
      33.5% of the frames but 66% of those over 0.10. Done when the cut is
      skipped below that eye distance or `eye_offset` is replaced by a check
      of the mesh against its own geometry, and the good-photo mark's share
      and the rated frames of `D:\photos\samples\ARW\good-mark-2026-10-09\`
      are re-read. Files: `crates/core/src/candidate.rs`
      (`mesh_eye_offset`), `crates/app/src/mark.rs` (`MAX_EYE_OFFSET`).
- [ ] Distrust the mesh values on side faces, the dominant mesh misfit
      (78% of the off meshes, weighted, in
      `docs/plans/20261010-mesh-roll/misfit-truth.md`). Done when a rule
      (e.g. YuNet eye distance under 0.10 of the box side, or the mesh's
      |yaw| past a bound) leaves the EAR, the AF eye cue and the pose of
      such a face unknown instead of a plausible-looking wrong value, checked
      on the misfit truth set and on the AF-eye, closed-eyes and head-pose
      labeled sets. Files: `crates/core/src/candidate.rs`,
      `crates/core/src/eyes.rs`.
- [ ] Faces rolled past 90 deg (a baby lying head-down,
      `D:\photos\2026\2026-07-11\_DSC2638.ARW`) get a mesh fitted as if
      upright, and the eye line cannot de-rotate them: `faces.rs` orders
      YuNet's eye points by image x, so the eye-line roll folds into -90..+90
      deg. They need another roll source (a mesh pass on rotated crops,
      MediaPipe's tracking loop). Not planned; see
      `docs/plans/20261010-mesh-roll/` (Decision). Files:
      `crates/core/src/faces.rs`, `crates/core/src/eyes.rs`.
- [ ] Review the mesh misfit labels: open the sheets
      `D:\Photos\tests\2026-10-09-mesh-roll\sheets\s00.png` to `s15.png`
      and compare each tile to its row of
      `docs/plans/20261010-mesh-roll/misfit-truth.md` (`before` / `after`
      `on` or `off` with its kind, and `yunet ok` / `yunet off`). The
      Decision's shares rest on the agent's labels; if labels change,
      re-run `compare.py` in that plan folder and update `compare.md` and
      the Decision if a number moves.
- [ ] Repoint the MCP `get_view` tool's `eyes` answer to the stored values.
      Pass 2 now stores the EAR and the pose of the face it meshes (the
      `mesh-eyes-index` work: `eyes_ear`, `eyes`, `eyes_closed` and `pose` on
      `Focus` / `FaceReady`, `docs/plans/_archived/20261008-mesh-eyes-index/plan.md`),
      but the app's `get_view`'s `eyes` still comes from the on-demand
      judgment (`EyesCache`), so it answers only for a file already shown
      (see `docs/plans/_archived/20261008-mcp-eyes-pose/plan.md`, written for
      the Tauri app). Answer from the stored values when present, falling
      back to the on-demand judgment, and keep the `eyes` key absent from
      `get_view` until a file is judged. Done when `get_view` answers from
      the stored values and a test covers the key's absence before a
      judgment. Files: `crates/app/src/companion.rs`,
      `crates/app/src/eyes.rs`.
- [ ] Skip the second model run in `ondemand::read_eyes` for a file whose
      scan already stored the EAR and pose (the `mesh-eyes-index` work stores
      them, and the meta pane reads them). The on-demand judgment still runs
      for the shown file because the `f` overlay's face mesh needs the 478
      points, and it is the fallback for files with no stored value (face
      under 60 px, or a file pass 2 has not reached). Reusing the scan's
      mesh needs the points stored or a path that runs the model only while
      `f` is on. Files: `crates/engine/src/ondemand.rs` (`read_eyes`),
      `crates/app/src/overlay.rs`, `crates/engine/src/index.rs`,
      `crates/core/src/eyes.rs`.
- [ ] Suggest the sharpest-eye frame within a burst group. Measured and not
      shipped (`docs/plans/20261008-burst-keep-score/`, Decision 1, `results.md` and `fit.md`): the user
      picks within a burst by timing, not by a technical feature; the
      burst's length, not sharpness or the eyes, predicts a kept scene
      (79% held out at 15 or more frames, which the burst band's count
      already shows, against about 56% for the best technical rule); and
      the non-picks are mostly technically fine. Revisit only with a new
      signal; the per-frame good-photo mark above covers "is this a miss".
- [ ] Spot-check whether the sharpness ranking within a burst changes now
      that Sony frames with face tracking are scored on the camera's AF frame
      instead of YuNet's eye midpoint. Needs a per-file score output
      (`handpic-cli scan` prints none). Files: `crates/core/src/sharpness.rs`,
      `crates/core/src/scan.rs`, `crates/cli/src/main.rs`.
- [ ] Crop detection can false-positive on printed faces or logos near the AF
      point (e.g. `_DSC3632`: a shirt logo scored 0.67), yielding a false
      candidate (or a false not-a-candidate) when the logo is the face nearest
      the AF point. Consider a size or aspect filter on crop boxes. Files:
      `crates/core/src/faces.rs` (`detect_around`),
      `crates/core/src/candidate.rs`.
- [ ] AF on a person in the background gives a sharp face and a false Good:
      the measure says the eyes of the face nearest the AF point are sharp,
      not that it is the subject. Files: `crates/core/src/candidate.rs`.
- [ ] Back-of-head and upturned faces find no face in the crop, so their
      Face focus state is Neutral (white), even when the camera tracked
      them. Files: `crates/core/src/candidate.rs`, `crates/core/src/faces.rs`.

Related: `crates/core/src/sharpness.rs`, `crates/core/src/faces.rs`, `crates/core/src/arw.rs`, `crates/engine/src/index.rs`, `crates/app/src/burst.rs`.

### Core: the review of the closed-eyes labels is still open

`closed-eyes-detection` (`docs/plans/_archived/20261007-closed-eyes-detection/plan.md`) set the closed-eyes threshold (0.137) and measured its AUCs on a truth set the agent labeled. The user's review of its `closed` faces is still open. Files: `crates/core/src/eyes.rs`, `docs/plans/_archived/20261007-closed-eyes-detection/eyes-truth.md`.

#### TODO

- [ ] Review the faces labeled closed: for each file in the "Faces with a closed eye" list of `docs/plans/_archived/20261007-closed-eyes-detection/eyes-truth.md`, open its tile `D:\Photos\tests\2026-10-07-closed-eyes\tiles\<stem>.png` (or the crops under `arw\` / `dng\`) and confirm that the eye marked `c` shows no iris; note any that are open. If labels change, re-run `handpic-cli eyes` on `labeled-paths.txt` and `scratch\auc.py` there, and re-fit the threshold and the slope in `crates/core/src/eyes.rs` if the best F1 moves.

### Core: the review of the head-pose labels is still open

`head-pose` (`docs/plans/_archived/20261007-head-pose/plan.md`) measured the angles against 190 labels (159 readable) that the agent made from the face crops: yaw class and direction, pitch, roll, or `x`. The user's review is pending, and the plan's Decision rests on these labels. Step 2 was ticked on the automated criteria (the labels exist, the measurement and Decision are written, `mise run ci` passes).

Files: `docs/plans/_archived/20261007-head-pose/pose-truth.md`, `docs/plans/_archived/20261007-head-pose/pose-results.md`.

#### TODO

- [ ] Also check hand check 16 of `docs/plans/_archived/20261008-burst-keep-score/results.md`
      (`D:\photos\2026\2026-07-05\_DSC2445.ARW`): a profile looking up behind a ball
      reads pitch -68 deg. Decide whether `crates/core/src/pose.rs` should call that
      unknown or whether the label stands.
- [ ] On Windows, open the sheets `D:\Photos\tests\2026-10-07-head-pose\sheets\s00.png` to `s16.png` (4 tiles across, row by row in the `#` order of the `pose-truth.md` table; crops in `crops\`) and compare each tile to its row. Note any label to change. If labels change, re-run `aggregate.py` (in `D:\Photos\tests\2026-10-07-head-pose\`) and update `pose-results.md` and the plan's Decision if a number moves.

### Core: the no-AF-point path decodes the preview up to three times

#### Background

On a file with no trusted AF point, pass 2 decodes the preview up to three times: the DCT-scaled RGB decode for the whole-image face search, a full-size RGB decode for the face mesh and the eye regions of the judged face (since `docs/plans/_archived/20261010-af-eye-good-bad-cascade/` Step 2, only when a confident face is found), then a full-size grayscale decode in `sharpness::score_preview`. The face-detection-recall-cost work cut the first decode to 3/8. Step 2 measured on one thread (`riffle-cli features`, `D:\photos\2026\2026-02-01` and `2026-09-05`, Leica DNG): a faced no-AF file went from about 94 ms to about 122 ms of analysis, a file with no face did not change; the numbers are in that plan's `learnings.md`. Scoring from the full-size RGB decode (its luma) would drop the grayscale decode on faced files, but it changes the score's pixels and its stored values, so it needs a `FACES_VERSION` bump. See also `docs/plans/_archived/20261005-face-detection-recall-cost/learnings.md` (Step 3 measurements).

Files: `crates/core/src/scan.rs` (`extract_analysis_unless`, `whole_image_cue`, `score`), `crates/core/src/sharpness.rs` (`score_preview`).

#### TODO

- [ ] Score a faced no-AF-point file from the full-size RGB decode the cue already made (its `candidate::luma`) instead of `score_preview`'s own grayscale decode. Measure the per-file saving with `handpic-cli features` on one thread over `D:\photos\2026\2026-02-01` and `2026-09-05` before and after, check that the score's ranking within bursts holds on those folders (Spearman per burst against the old score), and bump `FACES_VERSION`. Files: `crates/core/src/scan.rs`, `crates/core/src/sharpness.rs`, `crates/engine/src/index.rs`.

### Agents: confirm `pr-runner` waits for a child's hand-back on a real `/pr` run

#### Background

The `long-wait-timeouts` fix is confirmed on 240 `pr-runner` and 225 `merger`
transcripts: every `wait-pr-actionable.sh` / `wait-post-merge-runs.sh` call
passed `timeout: 600000`, and the runs the harness still moved to the
background were `TaskStop`ped and re-run in the foreground. What remained was
`pr-runner` polling (`wait-pr-actionable.sh`, `sleep`, `until` / `while`,
`git ls-remote`, `gh pr view`) while a child it had dispatched was still
running. The `pr-runner-child-handback` plan added a rule to
`.claude/agents/pr-runner.md` that the runner ends its turn after an `Agent`
dispatch and waits for the child's hand-back. It is verified only statically.

#### TODO

- [ ] On the next real `/pr` run whose PR hits a conflict, a CI failure or a
      review round, read the `pr-runner` subagent transcript (the subagent
      `.jsonl` files under the Claude Code project directory for that
      session) and confirm that after each `Agent` dispatch the runner ended
      its turn, the next event is the "[Subagent hand-back]" message, and
      there is no `wait-pr-actionable.sh`, `sleep`, `until` / `while`,
      `git ls-remote` / `git fetch` or `gh pr view` call in between. If not
      clean, adjust `.claude/agents/pr-runner.md`.

### Agents: confirm `merger` runs its step 6 commands as separate, verbatim calls on a real `/pr` run

#### Background

The `merger-separate-sync-calls` plan added a rule to `.claude/agents/merger.md` step 6: `mise run git:main` and `./tools/git/delete_merged_branches.sh` run as two separate Bash calls, each exactly as written in its code block (no pipes, no `2>&1`, no `tail`, no `;` / `&&` chaining, no `cd` prefix). The rule exists because the chained or piped forms match neither allow rule in `.claude/settings.json`, so they fall to the auto mode classifier, which can deny the sandbox bypass as a Safety Bypass. The merger then stops with `post_merge_sync_failed` after a merge that already succeeded (PR #657). CI covers only the Markdown formatting. A prose rule cannot be unit-tested, so it has never been exercised by a real unattended merger run. Plan: `docs/plans/_archived/20261003-merger-separate-sync-calls/plan.md`

Files: `.claude/agents/merger.md`

#### TODO

- [ ] On the next real `/pr` run that reaches the merger, read the `merger` subagent transcript (the subagent `.jsonl` files under the Claude Code project directory for that session). Confirm that step 6 issued `mise run git:main` and `./tools/git/delete_merged_branches.sh` as two separate Bash calls, each with exactly the text in its code block and `dangerouslyDisableSandbox: true`, and that the run did not end with `post_merge_sync_failed`. If a pipe, redirect or chain appears, or a denial recurs, strengthen the wording in `.claude/agents/merger.md` step 6.

### Agents: confirm `pr-runner` runs its skill scripts verbatim as standalone calls on a real `/pr` run

#### Background

The `pr-runner-verbatim-script-calls` plan extended `.claude/agents/pr-runner.md` §4 so that every script under `.claude/skills/**/scripts` it runs, the `wait-pr-actionable` call above all, is run exactly as written in its code block as a standalone Bash call: no pipes, no `tail`, no redirects, no `; echo exit=$?`, no `${PIPESTATUS[...]}`, no `;` / `&&` chaining. The rule exists because of PR #671, where `bash .claude/skills/pr/scripts/wait-pr-actionable.sh 671 | tail -12; echo exit=${PIPESTATUS[0]}` missed the `Bash(bash .claude/skills/pr/scripts/*)` allow rule and waited about 62 minutes before it ran. The script then printed `ACTION=ready` at about 150 s, but the call did not return until the 600 s timeout, most likely because a lingering child held the pipe open. A bare re-run returned in 2 s. CI covers only the Markdown formatting. A prose rule cannot be unit-tested, so it has never been exercised by a real unattended pr-runner run. Plan: `docs/plans/_archived/20261004-pr-runner-verbatim-script-calls/plan.md`

Files: `.claude/agents/pr-runner.md`

#### TODO

- [ ] On the next real `/pr` run, read the `pr-runner` subagent transcript (the subagent `.jsonl` files under the Claude Code project directory for that session). Confirm that every `.claude/skills/**/scripts` call, especially `wait-pr-actionable.sh`, has exactly the text in its code block with no pipe, `tail`, redirect, `; echo exit=$?`, `${PIPESTATUS[...]}` or chaining, and that none waited long before starting or was moved to the background after the script finished. If a decorated call appears, strengthen the wording in `.claude/agents/pr-runner.md` §4.

### Core: SIGMA fp L strip thumbnails have over ten times the pixels of other bodies'

SIGMA fp L DNGs have no strip JPEG at or above `PREVIEW_MIN_WIDTH` (1600)
below the 9520x6328 full-size one, so the preview tier falls back to the
full-size JPEG. The strip does not decode that per thumbnail: at scan time
`thumbnail_jpeg` decodes the preview tier at 2/8 scale and re-encodes it,
the index caches the result, and the strip's thumbnail read
(`crates/app/src/thumbs.rs`) reads the cache. But 2/8 of 9520x6328 is about 2380x1582 (about 3.8 MP), more than
ten times the pixels of the ~404x270 thumbnail of other bodies, so each
strip cell costs more to read and to decode. Each scan also
decodes the ~60 MP JPEG (at 2/8 scale) once per file. OM System / Olympus
ORFs have the same cost on a smaller scale: their only JPEG is 3200x2400, so
the fixed 2/8 gives 800x600 thumbnails, about four times the pixels of the
~404x270 ones (`crates/core/src/orf.rs`; decided in
`docs/plans/_archived/20260930-olympus-orf/plan.md` to leave `scan.rs`
alone for now). Files: `crates/core/src/decode.rs` (`thumbnail_jpeg`), `crates/core/src/scan.rs`,
`crates/engine/src/index.rs`, `crates/app/src/thumbs.rs`,
`crates/core/src/arw.rs` (`PREVIEW_MIN_WIDTH`, tier selection).

#### TODO

- [ ] Measure strip thumbnail load time on a SIGMA fp L folder (per-cell
      read and decode).
- [ ] Candidate fix, not decided: cap the thumbnail size, e.g. pick a
      larger DCT scale-down (`d.scale(n)`) when the preview is large, or
      resize to the normal thumbnail width.

### Core: Nikon DX-crop and non-standard-crop AF point mapping is unverified

`nef::parse`'s Nikon `AFInfo2` AF point is read as top-left coordinates in
the `AFImageWidth` / `AFImageHeight` frame, assumed to already match the
JPEG's crop. No DX-crop NEF sample was available to confirm this: exiftool's
`Nikon.pm` hints that some DX-mode results are reported in FX (full-frame)
coordinates, in which case the point would land off by the crop factor when
`AFImageWidth` stays the FX size while the JPEG is the DX crop. Found in the
Step 5 sample survey of
`docs/plans/_archived/20260929-canon-nikon-raw/plan.md`. Files:
`crates/core/src/nef.rs`.

#### TODO

- [ ] Find a Nikon DX-crop NEF sample (or a full-frame Z body shot in DX
      crop mode) and confirm whether the AF point position and
      `AFImageWidth` / `AFImageHeight` are already in DX coordinates or
      need scaling by the crop factor; fix `nef.rs` if the latter.

### Core: widen camera support from public sample RAW files

The user no longer owns the Sigma fp L or BF and cannot shoot new
samples, so public sample files are the verification source for more
cameras. Two kinds exist: raw.pixls.us (CC0; a few files per camera,
often flat test scenes that are weak for AF-point checks; CC0 means a
small file could be committed as a fixture, but tests prefer synthetic
bytes as in `crates/core/src/arw.rs`) and review-site sample galleries
(real scenes, good for AF checks, but not redistributable, so local
verification only). Riffle reads ARW, DNG, NEF, CR3, RAF and ORF today (README.md
"RAW formats"). The work splits into tiers, cheapest first:

1. More DNG-writing cameras (Pentax, Ricoh GR, other Leica bodies,
   phones). DNG reading already exists, so only preview/EXIF
   extraction needs verifying on samples.
2. AF point from MakerNotes that exiftool already decodes (Canon
   `AFInfo`, Nikon `AFInfo2`, Fujifilm `FocusPixel`, Olympus
   `AFPointSelected`, Panasonic `AFPointPosition`). Tag meaning and
   coordinate system are known; samples only confirm. This presupposes
   the RAW container is readable (tier 3), except for bodies that
   write DNG.
3. New RAW containers (CR3 = ISOBMFF, NEF, RAF, ...). Each needs a new
   parser next to `crates/core/src/arw.rs`; implementation outweighs
   verification.

AF data exiftool does not decode needs inference from many off-center
samples, as done for the Sigma BF `0x0147` in
`docs/plans/_archived/20260924-sigma-bf-af-point/plan.md`. Files:
`crates/core/src/arw.rs`, `crates/core/src/reader.rs`, `README.md`.

#### TODO

- [ ] Tier 1: verify preview/EXIF extraction on public DNG samples from
      Pentax, Ricoh GR, other Leica bodies, and phones; add each
      working body to the `docs/humans/cameras.md` table.
- [ ] Tier 2: read the AF point from the exiftool-decoded MakerNote
      tags above, confirming on samples, for bodies whose container
      Riffle can already read. Canon `AFInfo2` (EOS bodies) and Nikon
      `AFInfo2` versions `03xx` / `04xx` (Z bodies) are done
      (`crates/core/src/cr3.rs`, `crates/core/src/nef.rs`). Left: the
      Nikon DSLRs' `AFInfo2` `0100` / `0101` (D850, D500), whose AF point
      is a grid point name rather than a position, and a sample with an
      AF position from the Nikon Z 8 and the Canon EOS R6 (their
      raw.pixls.us samples carry none). Fujifilm `FocusPixel` on the RAF
      bodies is done too (`crates/core/src/raf.rs`, in the embedded JPEG's
      frame); an autofocus sample from the X-T3 and the GFX 100 is still
      missing (theirs are manual focus). Next: the OM
      System / Olympus ORF bodies (`crates/core/src/orf.rs`): CameraSettings
      0x030a `AFTargetInfo` on the OM bodies (a 640x480 frame with the focus
      and selected areas) and 0x0305 `AFPointSelected` (percentages) on the
      Olympus bodies. Every raw.pixls.us sample puts the point near the
      center, so the origin and orientation of the frame are unconfirmed; an
      off-center landscape and a portrait sample are needed (see the Step 2
      survey in `docs/plans/_archived/20260930-olympus-orf/learnings.md`).
- [ ] Tier 3: decide per container whether a new parser is worth it,
      given the samples available. CR3, NEF, RAF and ORF are done
      (`crates/core/src/cr3.rs`, `crates/core/src/nef.rs`,
      `crates/core/src/raf.rs`, `crates/core/src/orf.rs`); the rest remain.
- [ ] Check the Sigma BF AF point's open assumptions against public BF
      samples: portrait orientation, manual-focus behavior, and the
      1000x667 scale (see the `SIGMA_BF_AF_GRID_W` doc comment in
      `crates/core/src/arw.rs` and the archived plan's
      [Trade-offs and risks](docs/plans/_archived/20260924-sigma-bf-af-point/plan.md#trade-offs-and-risks)).

### Core: a RAF folder scan costs ~222ms per file

#### Background

The `fujifilm-raf` Step 1 survey (`docs/plans/_archived/20260930-fujifilm-raf/learnings.md`) ran `riffle-cli scan` over 46 raw.pixls.us RAFs: 221.8ms per file mean on one thread (290.2ms p95), 36 files/s on 24 threads. The embedded JPEG is 4416x2944 / 4000x3000 (against ARW's 1616x1080 preview), it is decoded whole for the sharpness score, and the face search runs on the whole image on frames with no AF point. The Fujifilm `FocusPixel` is now read (Tier 2 of "Core: widen camera support from public sample RAW files"), so the face search is skipped on autofocus frames while manual-focus frames still search the whole image; a scaled decode for the score would be a separate change. Files: `crates/core/src/scan.rs`, `crates/core/src/sharpness.rs`, `crates/core/src/raf.rs`.

#### TODO

- [ ] Measure the scan on a real RAF folder after the AF point is read, and decide whether the sharpness score should work on a scaled decode of the RAF JPEG.

### Core: M-RAW RAF files are unverified

#### Background

No raw.pixls.us sample of the listed Fujifilm bodies is an M-RAW (multi-image) RAF: the M-RAW header words at `0x48` / `0x4c` are zero on all 46 samples of the `fujifilm-raf` Step 1 survey (`docs/plans/_archived/20260930-fujifilm-raf/learnings.md`). `raf::parse` reads the one embedded JPEG and ignores the M-RAW header, which ExifTool suggests is enough, but no such file has been opened. Files: `crates/core/src/raf.rs`.

#### TODO

- [ ] Open an M-RAW RAF (from a body that writes one) and confirm the preview, the 1:1 view and the EXIF rows.

The `raf-older-bodies` sweep of 228 RAFs from 87 bodies (`docs/plans/_archived/20260930-raf-older-bodies/learnings.md`) found no M-RAW either: `0x48` is zero on all of them.

### Docs: Sony AF point, face tracking and portrait orientation are unconfirmed for lack of samples

#### Background

The `sony-arw-coverage` verification (`docs/plans/_archived/20260930-sony-arw-coverage/plan.md`) marks these `–` in `docs/humans/cameras.md` because the raw.pixls.us samples don't show them:

- Every α9 III and α7CR sample is manual focus (`FocusMode` 0, point at the exact center, `FocusFrameSize` invalid). `trusted_focus` drops the point as designed, so both bodies show `–` for AF point and AF frame size.
- Face tracking: the raw.pixls.us samples hold no `AFTracking` 1. The local samples later checked in `sony-face-tracking-docs` (`docs/plans/_archived/20261001-sony-face-tracking-docs/plan.md`) record it on the α7 IV, α7R V, α7S III, α6700 and ZV-E1, so those show `✓`. The α1, α9 III, α7C II and α7CR stay `–` because no sample of them recorded it.
- No sample is a portrait frame (Orientation 1 on all 66 files), so orientation is unverified on every listed Sony body.

Files: `docs/humans/cameras.md`.

#### TODO

- [ ] Find an AF-C sample of a person from the α9 III and α7CR (portrait orientation if possible), and turn their `–` for AF point / AF frame size into `✓` if the point lands on the subject.
- [ ] Confirm orientation on a portrait ARW sample and record the result in `docs/humans/cameras.md`.

### Engine: measure whether a `notify` watch blocks deleting a folder on Windows

`docs/plans/_archived/20260928-tree-live-watch/plan.md` (Step 1) measured only
rename on Windows with `notify` 8.2: a watch pins the watched folder's
ancestors against rename, not the folder itself. Delete was never measured,
so `docs/humans/usage.md` and the plan's Trade-offs section say "rename" only.

#### TODO

- [ ] Using the same scratch-binary approach as the Step 1 rename
      measurement, measure on Windows whether deleting a watched folder
      itself, or a folder with a watched descendant, succeeds with `notify`
      8.2. Record the result next to the rename measurement and update
      `docs/humans/usage.md` and `docs/agents/shared-crates.md` to match (or confirm
      they need no change).

### App: a JPEG folder's preview decodes 15-20x longer than an ARW's

Backend measurement (docs/plans/_archived/20260927-jpeg-view-only/learnings.md,
Step 2): a 21.5 MP JPEG's `read_preview` + full `decode_rgb` took ~197 ms
and a 14.8 MP one ~141 ms, versus ~9-15 ms for an ARW/DNG preview. The
plan's remedy (a DCT-scaled, capped re-encode) was measured and found
slower than shipping the whole file, so the Tauri app's `preview` sent the
whole JPEG as is. The app decodes the whole JPEG for its preview too.

#### TODO

- [ ] Find a payload that skips the full decode/encode round trip, e.g. a
      DCT-scaled decode (4/8 or 3/8), or reading a JPEG's embedded Exif
      thumbnail first. Verify with the app's `page` timing lines before
      choosing. Files: `crates/app/src/main.rs` (the preview load),
      `crates/core/src/decode.rs`.

### Engine: PhotoLab Uuid lookup on macOS: real-device checks and external volumes

`crates/engine/src/photolab.rs` looks up the registered Uuids on macOS too
(`docs/plans/_archived/20261005-photolab-uuids-macos/plan.md`), verified by
hand on macOS (PhotoLab 10, 2026-10-05; see `docs/agents/photolab.md`). Paths
under `/Volumes` get no lookup yet, so a fresh `.dop` there still gets random
Uuids and may become a virtual copy.

#### TODO

- [ ] Real-device check: give a source a virtual copy in PhotoLab and confirm
      the master is the lowest `ZDOPINPUTITEM.Z_PK`.
- [ ] External volumes: open a folder on `/Volumes/X` in PhotoLab, find how it
      records the volume (`ZTYPE`, `ZNAME`, `ZUNIQUEID` in `ZDOPFOLDER`), then
      extend `folder_names` / `query`'s macOS mapping in
      `crates/engine/src/photolab.rs`.

### Engine: PhotoLab after a file or folder rename

From Step 3's check of the PhotoLab Uuid lookup (`crates/engine/src/photolab.rs`
queries `Sources` by `FolderId` and `Name`; `crates/core/src/dop.rs` writes
the RAW's `Name` into the `.dop`). The Tauri app's `rename_file` moved the
`.dop` unchanged (the app has no file rename yet, `not_in_app`),
so its inner `Name` still named the old file and PhotoLab's database still
held the old name. A folder rename (`crates/engine/src/rename.rs`) leaves
PhotoLab's database holding the old folder.

#### TODO

- [ ] Find out (checked with PhotoLab) whether PhotoLab re-matches a renamed
      RAW with its moved `.dop` or imports it as a new image / virtual copy,
      and either patch the `.dop`'s `Name` on rename or document what to
      expect.

### App: real-device check of an ORF folder in the strip, sidecars and Move to Trash

`orf.rs` and the `.orf` arm of `scan::is_raw_file` / `reader::parse_raw`
were verified by unit tests and `riffle-cli bench` / `scan` / `focusbox` on
the raw.pixls.us samples in `D:\photos\samples\ORF\` (never committed), but
the GUI was never exercised on them. See
`docs/plans/_archived/20260930-olympus-orf/plan.md` and its `learnings.md`.
Files: `crates/core/src/orf.rs`, `crates/core/src/scan.rs`,
`crates/core/src/reader.rs`.

#### TODO

- [ ] On Windows, open `D:\photos\samples\ORF\` in the app and check that
      (1) every `.ORF` appears in the strip with a thumbnail, and the folder
      tree's RAW count includes them; (2) the preview and the 1:1 view show
      the 3200x2400 embedded JPEG, the E-M1 Mark II's rotated frames
      (Orientation 8, e.g. `E-M1MarkII_olympus_om_d_e_m1_mark_ii_01.orf`)
      upright; (3) the meta pane shows the EXIF rows (camera without
      trailing spaces, lens, exposure, capture time); (4) a star, a flag and
      a color label write an XMP and a `.dop` next to the file; (5)
      the strip's `Move to Trash…` on a file moves the ORF and its
      sidecars, and Undo restores them.

### Core: real-device check of the scan priority's macOS QoS branch

`scan-priority` (docs/plans/_archived/20260930-scan-priority/plan.md) ran the scan workers below normal OS priority (first pass `BELOW_NORMAL` / `QOS_CLASS_UTILITY`, analysis pass `LOWEST` / `QOS_CLASS_BACKGROUND`; the analysis pass was later moved to `QOS_CLASS_UTILITY` at relative priority -5, `LOWEST_RELATIVE_PRIORITY`, after the UI tests timed out on macOS CI; docs/plans/_archived/20261119-uitest-macos-timeout/plan.md, whose option D was chosen on CI evidence alone: pass 2 took 15 s to 44 s at BACKGROUND against UTILITY passing, measured on the CI runner, never on a Mac). CI covers the priority read back inside a worker on Windows, Linux and macOS (the QoS class and relative priority); the macOS QoS branch was never exercised on a real Mac. The app's paging cost during the passes was measured on Windows (see `docs/agents/app.md`). Files: `crates/core/src/scan.rs` (`for_each_path`, `Priority`), `crates/engine/src/index.rs` (`run_scan`, `run_faces_scan`), `docs/humans/performance.md` ("Which pass carries which cost"), `docs/humans/performance.ja.md`, `docs/agents/shared-crates.md` ("The scan pool's priority levels").

#### TODO

- [ ] On macOS, open a large RAW folder with a cold index in the app and page through it while `scanning N / M` and then `analyzing N / M` run. Expect the analysis pass (`QOS_CLASS_UTILITY` at relative priority -5) not to crawl and paging to stay as responsive as on Windows (compare the `page` timing lines in `Handpic.log`); if paging suffers, lower `LOWEST_RELATIVE_PRIORITY` in `crates/core/src/scan.rs` and update the priority entry in `docs/agents/shared-crates.md`. No `ran at normal priority` warning may appear in the log.

### App: real-device check that big-endian DNGs open after the EXTRACTOR_VERSION bump

`big-endian-dng` (`docs/plans/_archived/20261002-big-endian-dng/plan.md`) made `arw::parse` follow the TIFF header's byte order, so the `MM` DNGs of Pentax, Samsung, Ricoh, Leica M (Typ 240 / 246) and iPhone no longer fail with `not a little-endian TIFF/ARW`. It also bumped `EXTRACTOR_VERSION` from 11 to 12, so an index that cached those parse errors should re-extract the rows. The unit tests, `riffle-cli check` on the sample tree (34 of the 52 `MM` files open at every stage) and `mise run ci` cover the parser. The Riffle app was never opened on such a folder, so the re-extraction of cached failed rows and the thumbnails, preview and 1:1 view for these files were never exercised. The step was ticked on the automated criteria only.

Files: `crates/core/src/arw.rs`, `crates/engine/src/index.rs` (`EXTRACTOR_VERSION`).

#### TODO

- [ ] On Windows, with an index written by `EXTRACTOR_VERSION` 11 (open `D:\Photos\samples\DNG\` in a build from before the bump, then in this build), open a folder containing the Pentax K-5 II, iPhone 12 Pro and Leica M (Typ 240) samples. Expected: the rows that cached `not a little-endian TIFF/ARW` are re-extracted, and the thumbnails, the preview and the 1:1 view show.

### App: real-device check of the 1:1 view on older Sony ARW bodies (no full-size JPEG)

The `sony-arw-preview-full` work (`docs/plans/_archived/20261005-sony-arw-preview-full/plan.md`) made `arw::parse` fall back to the preview as `full` when the largest other JPEG is smaller, so the α9 II, α7R IV, α7R IVA, α7C, α6400, α6600 and ZV-E10 no longer take the 160x120 IFD1 thumbnail for the 1:1 view. A synthetic-TIFF unit test, `riffle-cli info` / `bench` on the samples (`full` at the preview's offset / length; decodes of 1616x1080, 1440x1080 on the 4:3 α7C sample and 1920x1080 on the ZV-E10 read from the JPEG SOF) and `mise run ci` cover the core path. The app's 1:1 view (`z`, whose crop read in `crates/app/src/zoom.rs` goes through `reader::read_full` and `decode_focus_crop`) was never exercised on these files. The step was ticked on the automated criteria only. Files: `crates/core/src/arw.rs`, `crates/core/src/reader.rs`, `crates/app/src/zoom.rs`.

#### TODO

- [ ] On Windows (or any desktop build), open `D:\photos\samples\ARW` (or a folder with e.g. `ILCE-6400_DSC00087.ARW` / `ZV-E10_DSC00002.ARW`), select the file and press `z`. Expected: a sharp crop of the 1616x1080 preview (1920x1080 on the ZV-E10) around the AF point, not a blurry upscaled 160x120 thumbnail.

### Agents: bare `python` / `node` hang in Git Bash on the Windows host

#### Background

During `merge-settings-drop-hook-exclusions` (`docs/plans/_archived/20261002-merge-settings-drop-hook-exclusions/learnings.md`), a bare `python` or `node` in Git Bash on the Windows host hung instead of failing; running them through `mise x --` worked. Any agent session on this host that runs a script needing them hits the same hang, before any guide would be consulted.

Files: `CLAUDE.md` (Development) or a new `docs/agents/dev-environment.md`.

#### TODO

- [ ] Check that the note is not already in the user-level memory or instructions, then add it: preferably a short line under `CLAUDE.md` Development ("On Windows, run python / node through `mise x --`; a bare invocation hangs in Git Bash"), otherwise a new `docs/agents/dev-environment.md`. Done when the note exists in the chosen place.

---

### Engine: a redo of Delete Sidecars… leaves a closed folder's rows judged

#### Background

The folder-sidecars feature added `Delete Sidecars…`. A redo (`trash::redo_run`) moves the sidecars to the Trash again but writes no index row. When the target folder is open, the app's resync clears the judgments. When another folder is open, the rows keep the judgments that the undo's rescan restored, until the folder is next opened and its reconcile clears them. A `Rewrite Sidecars from Index…` run in between would mint the sidecars again. Fixing it needs the run to remember each sidecar's RAW, for example in `trash::Trashed`. Plan: `docs/plans/_archived/20261005-folder-sidecars/plan.md`. Files: `crates/engine/src/trash.rs` (`redo_run`), `crates/engine/src/foldersidecars.rs`, `crates/app/src/main.rs`.

#### TODO

- [ ] Record each sidecar's RAW in the trash run (for example in `trash::Trashed`), so that a redo clears the index rows of a closed folder.
- [ ] Add a test covering delete, undo, then redo with another folder open.

### Engine: Rewrite Sidecars from Index… patches an oversize sidecar

#### Background

The folder-sidecars feature added `Rewrite Sidecars from Index…`. `foldersidecars::rewrite` hands every listed row to the writer, including a file whose sidecar is larger than `MAX_SIDECAR_BYTES`. A folder open deliberately keeps such a file away from the writer through `sidecar::reconcile_of`, because its contents were never read. This was left as is because the user asked for the index to be pushed out. Plan: `docs/plans/_archived/20261005-folder-sidecars/plan.md`. Files: `crates/engine/src/foldersidecars.rs` (`rewrite`), `crates/engine/src/sidecar.rs` (`MAX_SIDECAR_BYTES`, `reconcile_of`).

#### TODO

- [ ] Decide whether the rewrite should skip oversize sidecars like a folder open does.
- [ ] If it should, skip them and count them in the dialog's "skipped" figure.

### Shortcuts: a newly added default key silently displaces a user's stored override

#### Background

`performance-graph-overlay` added the default key `d`, and `focus-peaking` added `e`. `Keymap::from_overrides` gives a key already held by a default precedence over a partly applicable stored override. A user whose stored `shortcuts` override bound `d` or `e` to another action (for example a WASD-style layout on `next`) therefore loses that key silently on upgrade. It surfaced when `a_stale_default_in_an_old_store_drops_only_that_key` failed in CI; that test's arbitrary `d` was changed to `e`, and `focus-peaking` then changed it to `n` when `e` became `peaking`'s default, but the real-user case is unaddressed. Plans: `docs/plans/_archived/20261011-performance-graph-overlay/plan.md`, `docs/plans/_archived/20261108-focus-peaking/plan.md`. Files: `crates/engine/src/shortcuts.rs` (`DEFAULTS`, `from_overrides`).

#### TODO

- [ ] Decide whether a newly added default should yield to an existing override, or whether the drop should be logged at `warn` (or both).
- [ ] Implement the chosen behavior with a Rust test covering an old store that binds the new default's key to another action.

### Core: the ranking's pull toward the prior is untested for one to three culled folders

#### Background

`rank.rs` `fit` shrinks toward the built-in prior with tau = 1/(2C) = 5 in total. A single culled folder (about 70 scorable bursts) may outweigh that several times over, and the one measured move (0.300) came from 1602 bursts. Found in the `image-embeddings` Step 7 local review. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/core/src/rank.rs`, `docs/plans/_archived/20261012-image-embeddings/ranking.md`.

#### TODO

- [ ] Refit from the prior on 1, 3 and 5 culled folders, compare the held-out weighted AUC with the prior's, and pick the strength (for example tau scaled by the prior's burst count).

### Core: the ranking's reject weight and confidence levels are choices, not fits

#### Background

`REJECT_WEIGHT` = 2 (`crates/core/src/rank.rs`) is a choice: the 58 explicit rejects (3 scorable bursts) in the author's data cannot measure how much stronger a reject should count than a frame merely not chosen. The Tauri app's confidence levels (`none` / `low` under 300 bursts / `medium` under 1000 / `high`, its `rank.rs`'s `confidence`, removed with it) were read off Step 2's learning curve of the author's own folders, where the prior already is the user's taste. For another photographer the prior is someone else's, and the levels say nothing about how far the fit has moved from it. Found in `image-embeddings` Step 7 (`ranking.md`). Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`.

#### TODO

- [ ] Revisit `REJECT_WEIGHT` once a user's data holds enough explicit rejects to measure it.
- [ ] If the app shows a ranking confidence, base it on how far the fit has moved from the prior, not only on the burst count.

### Core: portrait bursts lean negative under the per-genre ranking offsets

#### Background

The `genre-ranking-offsets` feature kept per-genre offsets on the shared ranking weights (`OFFSET_SHRINK` 30). No genre shows a significant gain. In the offline evaluation, portrait bursts (57) have a paired difference of -0.024 [-0.067, +0.003]; the technical-only shape's is significantly negative at -0.058 [-0.115, -0.016]. Unknown also leans negative, with an interval that includes zero. The plan's bar has no per-genre condition, so this did not fail it, but with only 57 portrait bursts it should be re-checked once more non-sports judgments have accumulated. Basis: `docs/plans/_archived/20261014-genre-ranking-offsets/offsets.md`. Files: `crates/core/src/rank.rs`, `docs/plans/_archived/20261014-genre-ranking-offsets/genre_offsets.py`.

#### TODO

- [ ] Once more non-sports judgments have accumulated, take a fresh app dump and re-run `genre_offsets.py` (it needs the scikit-learn venv, e.g. `D:\Photos\tests\.venv-eval\Scripts\python.exe` (the old `%TEMP%\embedvenv` was wiped on 2026-10-19); the run takes about 50 minutes on 20 workers and prints only at the end). If portrait's (or Unknown's) paired difference stays negative with an interval that excludes zero, raise `OFFSET_SHRINK` or drop the offsets for that genre; otherwise leave it as is.

### Engine: the AI model's footprint (resident plan, kept 378 MB file, 190 MB fp16 option)

#### Background

Three leftovers from `image-embeddings` Step 3. (1) `embed::shared` keeps the SigLIP 2 plan (about 395 MB of working set) in a `OnceLock` for the rest of the process once one pass has run, even with `aiEnabled` off. Dropping it would cost the ~0.5 s build and the 773 MB load peak again on the next folder. (2) Turning AI off keeps `models/siglip2-base-patch32-256/vision_model.onnx` (378 MB) in the local data dir, and a cut download restarts from zero (no HTTP range resume). (3) tract runs the weight-only fp16 re-export (190 MB, `weight_only.py`) at 2-5x FP32's cost because it re-widens the weights on every run. Widening the float16 initializers in the ONNX protobuf before tract builds the plan (`tract_onnx::onnx().proto_model_for_read`, then `model_for_proto_model`) might give the smaller download at FP32 speed. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `crates/core/src/embed.rs` (`shared`), `crates/engine/src/ai.rs`, `docs/plans/_archived/20261012-image-embeddings/weight_only.py`, `docs/plans/_archived/20261012-image-embeddings/models.md`.

#### TODO

- [ ] Revisit dropping the plan after a pass if memory becomes a complaint.
- [ ] Add a "remove the model" action next to the AI switch (the app has none yet), and consider HTTP range resume for the download.
- [ ] Try widening the fp16 initializers before the plan is built, and measure ms/image against the FP32 file (129 ms on one thread).

### Core: soccer and macro genres need labeled positives to come back

#### Background

Decision 4 of `image-embeddings` dropped soccer, macro and other from the genre list: the labels hold no soccer or macro frame, so no threshold could be set or justified. Plan: `docs/plans/_archived/20261012-image-embeddings/plan.md`. Files: `docs/plans/_archived/20261012-image-embeddings/genres.md`, `crates/core/genres/`.

#### TODO

- [ ] Label frames from a soccer game and from a macro session, add them to `genre_prompts.json`, then re-run `genre_vectors.py` and `genre_calibrate.py`.

### Core: keep or remove the ignored `detector_tract.rs` test left by ball-game-ranking-features

#### Background

The ball-game-ranking-features plan added `crates/core/tests/detector_tract.rs` in Step 3 as the check that a detector can ship. It is an `#[ignore]` test that reads `HANDPIC_DETECTOR_ONNX`, `HANDPIC_DETECTOR_INPUT` and `HANDPIC_DETECTOR_TENSORS`. Step 4 was meant to remove it or keep it as its seed, but Decision 1 (no detector ships) dropped Step 4. The test costs nothing in CI because it is ignored. It is the load / cost / box-agreement harness a later detector would reuse. Related: `docs/plans/_archived/20261014-ball-game-ranking-features/plan.md`, `docs/plans/_archived/20261014-ball-game-ranking-features/detect_eval.py` (`--tensors` / `--compare`).

#### TODO

- [ ] Decide whether to keep `crates/core/tests/detector_tract.rs` as the reusable harness or delete it. If kept, say in the file's header comment what it is for.

### Core: a plan to drop the genre from the personal ranking (scope decision pending)

#### Background

The `folder-wide-ranking-eval` feature re-measured the ranking folder-wide. Its Decision 1 (part 4) applied the pre-registered rule "keep the genre only when `+genre cols` or `+genre off` passes on either base". No genre comparison passed (bars met: 1, 3, 2 and 2 of 5). The shipped per-genre offsets (`W0+off` against `W0`) read +0.003 frame AUC, -0.7 points R-precision and -0.000 within-burst wAUC, so removing them changes nothing visible. The `reject-filter-eval` feature's Decision 2 (item 3) re-read the same question on a bottom-oriented, recall-constrained criterion (`RN@95`, the share of non-picks removed while keeping 95% of the picks) and confirms the drop. None of the four genre comparisons passes the comparison bar: `F0+ctx +genre cols` -0.7 points (p = 0.6159), `F0+ctx +genre off` +0.8 (p = 0.1498), `W0 +genre cols` +1.0 (p = 0.0640, 1 of 3 bars), `W0+off` +0.8 (p = 0.3110). The genre labels alone remove 7.2% of the non-picks at 95% pick recall, against 5.8% for random. The evidence is from one photographer's library, where 1,439 of 1,801 scorable bursts are sports. Decision 1 leaves the scope to the user: (a) ranking only, which the plan recommends, or (b) the whole feature. The `rank.rs` part touches `fit`, which another session owns, so this waits for that session to finish. Basis: `docs/plans/_archived/20261015-folder-wide-ranking-eval/plan.md` (Decision 1, parts 4-5), `docs/plans/_archived/20261015-folder-wide-ranking-eval/folderwide.md`, and `docs/plans/_archived/20261016-reject-filter-eval/plan.md` (Decision 2, item 3). Files: `crates/core/src/rank.rs`, `crates/core/src/genre.rs`, `crates/core/genres/siglip2.json`.

#### TODO

- [ ] Ask the user to choose the scope: (a) ranking only, or (b) the whole feature.
- [ ] After the session owning `rank.rs`'s `fit` is done, write and run a plan for the chosen scope.
  - (a) Remove `Weights.offsets`, `OFFSET_SHRINK` and the `of_burst` lookup in `fit` / `rank`. Keep `genre.rs` and `siglip2.json`.
  - (b) Do (a), and also remove `genre.rs`, `crates/core/genres/siglip2.json` and their docs (`CLAUDE.md`, `README.md` / `README.ja.md`, the usage docs). The embedding stays, because the ranking needs it.
- [ ] When this lands, delete `### Core: portrait bursts lean negative under the per-genre ranking offsets`, whose premise goes with the offsets.

### Core: the shipped ranking weights score single frames at chance

#### Background

The `folder-wide-ranking-eval` feature found that the within-burst weights do not carry over to single frames. Held out, `W0` reads singles AUC 0.508, against 0.611 for `F0+ctx` (13 folders with at least 5 single picks and 5 single non-picks). This is a reading aid, not a bar. The app ranks only within bursts today, so nothing shows it. If a later plan ranks or marks singles (a folder-wide "best" view), it should not reuse the within-burst weights as they are. Basis: `docs/plans/_archived/20261015-folder-wide-ranking-eval/folderwide.md`. Files: `crates/core/src/rank.rs`.

#### TODO

- [ ] Before any feature ranks or marks single frames, fit or evaluate a separate model for them (the pointwise `F0+ctx` shape reads 0.611) instead of reusing the within-burst weights.

### Agents: the archived plan scripts' import paths broke when their folders moved

#### Background

`docs/plans/_archived/20261014-folder-wide-pick-model/folder_gate.py`, `docs/plans/_archived/20261014-ball-game-ranking-features/ballgate.py` and `docs/plans/_archived/20261014-genre-ranking-offsets/genre_offsets.py` insert `<their folder>/../_archived/20261012-image-embeddings` on `sys.path`. They were written before the archive move, so that now resolves to `_archived/_archived/...`. `folder_gate.py`'s `GENRES` also points at `docs/crates/core/genres/siglip2.json`. Run directly, any of them fails at `import gate` (or at `extend`). They still work when imported after `gate`, as `folderwide.py` does, and `folderwide.py` overrides `GENRES`. Found in `folder-wide-ranking-eval` Step 2. Files: the three scripts above.

#### TODO

- [ ] Either fix the paths in the archived scripts, or state in the archive convention that a plan's scripts are frozen as run and must be imported the way `docs/plans/_archived/20261015-folder-wide-ranking-eval/folderwide.py` does.

### Agents: a plan linking a sibling plan in flight breaks lint on main once the sibling is archived

#### Background

PR #827 linked `../20261015-similar-scene-groups/plan.md`; PR #826 archived that folder and merged first. Each PR passed lint alone, but main's lychee (`mise run lint`) failed after both merged; the folder-wide-ranking-eval Step 1 PR fixed it by retargeting the link to `_archived/`. Basis: `docs/plans/_archived/20261015-folder-wide-ranking-eval/learnings.md` (Step 1, Deferred issues). Files: `.claude/skills/develop/SKILL.md` or the planner's plan-writing instructions, or a new `docs/agents/` plans guide.

#### TODO

- [ ] Put the rule in the chosen place: link only archived or stable plan targets from a plan, and re-run `mise run lint` after rebasing onto the latest main when a linked plan may move. Done when the procedure tells the author to re-run lint after rebasing onto a main where plans moved.

### Core: re-run the reject-filter evaluation once enough in-app rejects and views exist

#### Background

The `reject-filter-eval` feature measured the personal ranking, its cues and a few rule filters as a bottom filter (drop the clearly bad photos while keeping about 95% of the picks). It recommended no app change. The best filter, `G0` (gradient boosting on the `F0+ctx` columns), removed 16.8% of the non-picks at a label-free cut that kept 94.7% of the picks, against the pre-registered 25% floor. Its oracle cut removed 22.0%, also below the floor. Its pass of the comparison bar against the shipped weights also depended on a per-fold weight rescale that the pre-registration did not fix. The evaluation could not say whether a removed frame was bad: every non-pick counted as a negative, including fine-but-redundant frames, and only 58 explicit rejects in 3 folders were known to be bad. The conditions that would change the answer are:
- Explicit rejects and `views` rows (a viewed file left unflagged) accumulating in the app, so a filter can be measured and trained on "bad" rather than "unpicked".
- More folders and more photographers than the 27 folders, mostly basketball, of one photographer.
- `G0`'s weight scale pre-registered, or tuned in the inner CV, before it is read again.
- A calibrated label-free threshold (for example one learned on each fold's training folders) to close part of the 5-point gap between the label-free and oracle cuts. It cannot pass the bar alone on this data.
The `bad-photo-labels` feature later labeled 1,200 sampled frames for badness (plan: `docs/plans/_archived/20261017-bad-photo-labels/plan.md`, Decision). 5.52% of the dump is bad, about 94% of the non-picks are not bad, and no filter or dedicated model met its bar (best `R0` at q5: `BR` 19.3%, `P` 23.1%). So the non-pick criterion overstated the bad share about fifteenfold, and a re-run on in-app rejects would measure a different, smaller set of frames than that dump's non-picks.
The `discard-labels` feature measured the same premise for the "OK to discard?" question (plan: `docs/plans/_archived/20261018-discard-labels/plan.md`, Decision, item 1). 51.7% [44.9, 59.0] of the dump's non-picks are OK to discard, so "non-pick = negative" overstates it about twofold. It also misses the other way: 24.4% [8.6, 42.8] of the sampled picks (12 of 39) are OK to discard, so the picks are not a clean "keep" class. A re-run that treats unflagged or viewed-unflagged frames as negatives inherits both errors.
The `discard-retarget` feature (plan: `docs/plans/_archived/20261019-discard-retarget/plan.md`, Decision) re-read the premise from the other side: training on the discard labels instead of on picks (`D0`) did not give a better bottom cut than `G0`, partly because 24.4% of the sampled picks are themselves labeled `discard`. A re-run on in-app rejects therefore should not expect the pick-versus-non-pick target to be the main limit.
Basis: `docs/plans/_archived/20261016-reject-filter-eval/plan.md` (Decision 2, item 4) and `docs/plans/_archived/20261016-reject-filter-eval/reject.md`. Files: `docs/plans/_archived/20261016-reject-filter-eval/rejectfilter.py`, `docs/plans/_archived/20261016-reject-filter-eval/rejectablate.py`, `crates/core/src/rank.rs`.

#### TODO

- [ ] When the app's explicit rejects and `views` rows have accumulated (the user judges when), write a plan that re-runs the evaluation with those as the negatives, reusing `rejectfilter.py`, `rejectablate.py` and the `bad-photo-labels` `badfilter.py` bars (worth-a-filter `NL` <= 5%, `BR` >= 50%, `P` >= 50%). It must pre-register `G0`'s weight scale and a label-free threshold calibration before reading any score, and start from the finding that a bad-photo bar is far from met on the 80 labeled bad frames.

### Core: decide whether a bad-photo cue shown to the human replaces the removal filter

#### Background

The `bad-photo-labels` Decision found no usable removal filter. The base rate of bad frames is 5.52% [4.15, 7.04], and about 94% of the non-picks the earlier evaluations counted as negatives are not bad. The best filter, `R0` at q5, removes 19.3% of the bad frames at `P` 23.1%, against the 50% / 50% bars. The dedicated models `B0` / `L0` did not meet the +10 point bar. One cue catches bad frames: the folder's bottom sharpness decile, a bad rate of 17.7% [11.1, 25.5], which is 3.2x the base rate. Frames with no AF face read 14.6% (2.65x), outside the pre-registered bins. The Decision left open whether the goal stays a removal filter at these bars or a later plan pre-registers a different use, such as a cue shown to the human rather than a cut, with its own bar. No plan was opened. Basis: `docs/plans/_archived/20261017-bad-photo-labels/plan.md` (Decision, "Left for the user to decide") and `docs/plans/_archived/20261017-bad-photo-labels/badfilter.md`. Files: `docs/plans/_archived/20261017-bad-photo-labels/labels.py`, `docs/plans/_archived/20261017-bad-photo-labels/badfilter.py`, `docs/plans/_archived/20261017-bad-photo-labels/badmodel.py`.

The `discard-labels` feature later measured a different question, "OK to discard?" with "hide / sort last" semantics, on 600 blind labels (plan: `docs/plans/_archived/20261018-discard-labels/plan.md`, Decision). 48.45% [42.03, 55.11] of the frames are OK to discard, so the strict 5.52% base rate is the narrow subset. No `G0` cut is usable: q5 `RR` 17.3% [11.3, 24.0] against the 15% bar, q10 23.7%, q20 27.9%. A strict-bad rate sits below the discard rate in every `G0` band, so the two labels rank the bands alike. A cue shown to the human (a discard candidate marked, not cut) was not measured as such. The Decision found automation bias: the unblinded `RR@5` was about 1.4% (1 of 71) against 17.3% blind. A cue shown to the human therefore needs its own blind measurement.

The `discard-retarget` feature then tried to make a better removal cut on the same 600 blind labels (plan: `docs/plans/_archived/20261019-discard-retarget/plan.md`, Decision). A 2x2 of discard target and burst-relative features gave no usable cut under either bar. `G0+B` is not better than `G0` (`DP` -1.9 [-6.5, +2.9] at q5), and `D0` / `D0+B` have a weighted `RR` of 32.7-42.7% and a higher `PL`. The preview's in-focus probability (removed since by the full-eye-sharp-only plan) alone was still the best single score by AUC (0.777). So a cue shown to the human is still the only unmeasured use, and the retarget gives no evidence that a better model is nearby.

#### TODO

- [ ] Ask the user whether the goal stays a removal filter or becomes a cue shown to the human (for example marking the folder's bottom sharpness decile, or frames with no AF face).
- [ ] If a cue, write a plan that pre-registers its own bar (precision of the marked frames against the 5.52% base rate, share of the bad frames covered, and the cost of the marks) before reading any number. It reuses `labels.tsv` and the sampler.

### Core: decide whether a second bad-photo labeling round for the cue readings is wanted

#### Background

The `bad-photo-labels` labels hold 80 bad frames of 1,200, with 13 of 27 held-out folders holding none. The Decision does not expect a second round of the same kind to change its answer on a removal filter. A round for a different question could give the cue readings more power: `|pitch|` > 45 reads 3.2x the base rate on only 19 sampled frames, with an interval of [0.0, 40.5]. The Decision left this to the user. Basis: `docs/plans/_archived/20261017-bad-photo-labels/plan.md` (Decision, "Left for the user to decide"). Files: `docs/plans/_archived/20261017-bad-photo-labels/sample.py`, `docs/plans/_archived/20261017-bad-photo-labels/labeling.md`, `docs/plans/_archived/20261017-bad-photo-labels/labels.py`.

The `discard-labels` feature ran a blind 600-frame round for the "OK to discard?" question (plan: `docs/plans/_archived/20261018-discard-labels/plan.md`, Decision). It took the user about 30 minutes, about 3 s per frame against the 1.5-2.5 s estimated, so size a further round at about 3 s per frame (1,000 frames about 50 minutes). It did not oversample the `|pitch|` > 45 stratum.

The `discard-retarget` Decision (`docs/plans/_archived/20261019-discard-retarget/plan.md`) names what a further round would need if one is wanted: mostly more blind controls. The 150 controls stand for 25,071 frames (`π` down to 0.0042), so a `D0` fold's Kish effective sample size is only 166-195 of 504-597 labeled frames. The same Decision lists pre-registered model changes to try with it (clipped `1/π` weights, a linear `D0`, a target that keeps picks out of the cut). It sized the round at about 3 s per frame (600 frames about 30 minutes, 1,000 about 50 minutes). Rule branch 2 ("more labels") was not triggered, since effect A at q5 was negative.

#### TODO

- [ ] Ask the user whether a second labeling round is wanted and for which question (for example the `|pitch|` > 45 stratum). If so, write a plan that pre-registers the question and its bar, and oversamples that stratum with `sample.py`.

### Core: decide whether `G0` as-is at per-folder q5-q10, sorted last, is worth shipping

#### Background

The `discard-retarget` feature (plan: `docs/plans/_archived/20261019-discard-retarget/plan.md`, Decision, "Facts the user may weigh") found no new model that beats `G0` and recommended nothing. Under the looser bar the user chose (`RR` <= 25% with the stratified 97.5% quantile <= 35%, `PL` <= 5%, removed share >= 4.5%, `DP` 2.5% quantile above the 48.45% base rate), the existing `G0` itself meets all of those conditions at q5 and q10: `RR` 17.3% [11.3, 24.0] and 23.7% [18.7, 28.7], `PL` 0.68% and 2.10%, removed 4.92% and 9.87%. It was only a reference there, so the "beats `G0`" condition could not apply to it. That leaves a separate question this plan did not answer: is `G0` at q5-q10, sorted last (not hidden), worth shipping as it is? A yes needs its own pre-registered question and bars, and a fresh blind confirmation round. The looser bar was chosen after `G0`'s numbers on these 600 labels were read, and `discard-labels` found automation bias (unblinded 70 of 71 against the blind `RR@5` of 17.3%). Left to the user as a question only; no app work follows from the plan.

The `model-search` feature (plan: `docs/plans/_archived/20261020-model-search/plan.md`, Decision, "Facts the user may weigh") read `G0` on its 600 fresh labels. q5 again meets conditions 1-4: `RR` 23.5 [15.7, 30.9], cluster 97.5% 34.9, `PL` 0.68%, removed 4.53%, `DP` 2.5% 69.1 above the 42.31% base rate. q10 fails `RR` (30.1 [23.7, 35.8]). The strata were cut for the model-search winner, so this is a reading, not the pre-registered confirmation round this item asks for.

The `discard-model` feature found that the plain `ef` cue beats the learned `LR/T`, a cue to weigh against shipping `G0`. The question of shipping `ef` / `ef-fill` sorted last has since been closed: no sort-to-last, and the `Face focus` filter's `Low` item is the shipped form (plan: `docs/plans/_archived/20261107-face-focus-tiers/plan.md`).

Files: `docs/plans/_archived/20261019-discard-retarget/plan.md`, `docs/plans/_archived/20261019-discard-retarget/retarget.md`, `docs/plans/_archived/20261020-model-search/modelsearch.md`, `crates/core/src/rank.rs`.

#### TODO

- [ ] Ask the user: is the existing `G0` at per-folder q5-q10, sorted last (not hidden), worth shipping as it is? If no, delete this item. If yes, write a plan that pre-registers the question and its bars and starts with a fresh blind confirmation round (candidates and controls mixed and unmarked, about 3 s per frame, 600 frames about 30 minutes). Run the offline Python with `D:\Photos\tests\.venv-eval\Scripts\python.exe -I -B` (the old `%TEMP%\embedvenv` was wiped).

### Engine: a split moves a shared sidecar with whichever RAW moves first

#### Background

Two RAWs sharing a stem (`a.ARW` and `a.NEF`) share `a.xmp`. `Rename by Capture Date…` moves that sidecar with whichever RAW moves first, which leaves the other RAW without it when the two have different dates. This is the same ambiguity `rename::file_plan` has. Plan: docs/plans/_archived/20261104-rename-folder-by-capture-date/plan.md.

Files: `crates/engine/src/datefolder.rs` (`move_plan`), `crates/engine/src/rename.rs`.

#### TODO

- [ ] Decide whether a sidecar shared by RAWs of different dates should be refused, copied, or left. Apply the same rule to `rename::file_plan` if it changes.

### Agents: `pr-runner.md` and `merger.md` say only `wait` / `review_required` are seen on an exit 2

#### Background

The `pr-wait-blocked-grace` plan (`docs/plans/_archived/20261118-pr-wait-blocked-grace/plan.md`) made `.claude/skills/pr/scripts/wait-pr-actionable.sh` return `blocked` only after it has persisted for a grace period (`BLOCKED_GRACE`, 60 s). A timeout that lands inside the grace now exits 2 with `ACTION=blocked` as the last output. `.claude/agents/pr-runner.md` (around the "Waiting on `wait` / `review_required`" sentence in §4) and `.claude/agents/merger.md` (the sentence on `wait` / `review_required` and the exit-2 handling) still name only those two ACTIONs as the ones observed on a timeout. The handling is already right: pr-runner's "Otherwise increment `wait_timeouts` and re-run" and merger's "increment `pr_wait_timeouts` and re-run" treat it as ordinary waiting. Only the sentence is stale. The plan left it out of scope.

Files: `.claude/agents/pr-runner.md`, `.claude/agents/merger.md`.

#### TODO

- [ ] Reword the sentence in both files to say that `blocked` can also be the last ACTION of an exit 2 (a timeout inside the grace) and is handled as ordinary waiting. Do not change the call lines or the handling.

### Agents: `test (ubuntu-latest)` hangs on docs-only commits

#### Background

The `ef-gate-fill` feature (plan: `docs/plans/_archived/20261104-ef-gate-fill/plan.md`) hit the `test (ubuntu-latest)` job stalling on docs-only commits. PR 904's first run was cancelled. Main run 42128694204 hit the job's 30-minute `timeout-minutes` with the `riffle_core` test binary running. Both reruns took about 4 minutes, against the usual 5-6. PR 901's ubuntu job stalled in `actions/cache`. Files: `.github/workflows/`, `docs/agents/github-actions-workflows.md`.

#### TODO

- [ ] Collect the hung runs' logs and decide whether the cause is a flaky test, a cache stall or a runner problem. Then lower the job timeout, add a per-test timeout, or add a "rerun, then check X" note to `docs/agents/github-actions-workflows.md`. Done when the cause is identified, or a decision that reruns are enough is recorded with a rerun-only rule in that guide.

### Core: re-measure the import copy pipeline against Explorer on a fast destination

#### Background

The import-from-card feature's `riffle-cli copybench` measured the copy pipeline (8 MiB x 4 buffers, per-file `sync_all`) only against `D:` (a Crucial BX500, DRAM-less, 96 % full). That drive capped every method at 8-22 MB/s, including `robocopy`, while the card read alone ran at 135 MB/s. The numbers cannot tell the methods apart, and Explorer itself was never timed. The defaults stay at 8 MiB x 4, and `FILE_FLAG_SEQUENTIAL_SCAN` and dropping `sync_all` showed no gain on that drive. Plan: docs/plans/_archived/20261106-import-from-card/plan.md.

Files: `crates/core/src/copy.rs`, `crates/cli/src/main.rs`, `docs/humans/performance.md`.

#### TODO

- [ ] Copy the same fresh, never-read files from the card into a destination on a faster or emptier drive (the NVMe `C:`) by `handpic-cli copybench <card-dir> <dst-dir>` and by an Explorer copy, with idle time between runs, and record the MB/s. Done when the table in `docs/humans/performance.md` has the Explorer row and the 8 MiB x 4 defaults and the per-file `sync_all` are confirmed or changed.

### Core: `copy::copy_files` can replace a destination created between its check and the rename

#### Background

The import-from-card feature's `copy::copy_files` checks `dst.exists()` before reading and again just before `fs::rename`, but `fs::rename` replaces the destination on every platform. A file created at the destination in that window, such as a sidecar the app writes for a RAW that just landed, would be replaced. A per-OS no-replace rename closes it (`MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, `renameat2` with `RENAME_NOREPLACE`, `renamex_np` with `RENAME_EXCL`). Plan: docs/plans/_archived/20261106-import-from-card/plan.md.

Files: `crates/core/src/copy.rs`.

#### TODO

- [ ] Replace the final `fs::rename` with a no-replace rename on Windows, Linux and macOS, and keep the failing-job message `exists: ...`. Done when a unit test that creates the destination between the check and the rename (through a hook or a stub) leaves the existing file untouched on the platform CI runs.

### App: real-device check of the chunked Move to Trash on Windows (trash-over-scan Step 4)

#### Background

`trash-over-scan` Step 4 moves files to the Trash in chunks of 100 on Windows (`run_chunked`), measured at about 7 ms a file against 200 ms per-file (see `docs/plans/_archived/20261106-trash-over-scan/trash-bench.md`). Tests use a fake mover and the bench used copies; the real app path was never exercised, and a locked file in a real chunk is covered only by the fake mover and the bench probe. Basis: `docs/plans/_archived/20261106-trash-over-scan/plan.md`.
Files: `crates/engine/src/trash.rs` (`run_chunked`, `move_chunk`, `trash_listed`, `redo_run`), `crates/app/src/main.rs`.

#### TODO

- [ ] On Windows, in a test folder of copies, move 500+ files with the app. Expect the move to take about 7 ms a file, near `trash-bench.md` (a large gap means the app adds a cost the bench did not have). Expect the status count to go up in steps of 100. Undo restores all, each sidecar with its RAW. Redo moves them again.
- [ ] On Windows, hold one file open, then move a selection around it. Expect that file listed as a failure with its sidecars left, and the rest moved and restored by Undo.

### Engine: Undo of Move to Trash leaves an orphaned `$I` file per restored file on Windows

#### Background

Found by `trash-over-scan` Step 4's measurement. trash 5.2.9's Windows `restore_all` moves the `$R` file back and nothing removes its `$I`. Riffle's Undo restores through the same call, so it presumably leaves one for every file it brings back. D: already held 5327 older orphans naming other paths (source not traced, left alone). Each is small and the shell does not list them, but they pile up and cost a file record each. See `docs/plans/_archived/20261106-trash-over-scan/trash-bench.md` (safety record).
Files: `crates/engine/src/trash.rs` (`undo_run`, `restore`).

#### TODO

- [ ] Check whether Explorer's own restore cleans up its `$I`, and whether the crate (or Riffle after `restore_all`) can remove the orphan.

### Engine: Undo of Move to Trash is still one shell operation per item

#### Background

After `trash-over-scan` Step 4 chunked the move, an Undo (`trash::undo_run`) is still per-item, one shell operation each. Not timed, but if an operation costs what a move did (about 200 ms), an Undo of a large move takes far longer than the move. One `restore_all` for the whole run took 1 s for 150 files in the bench, but `trash::restore`'s per-file rule (a sidecar waits on its RAW, never overwrite) would need the same treatment `run_chunked` gave `run`. See `docs/plans/_archived/20261106-trash-over-scan/trash-bench.md`.
Files: `crates/engine/src/trash.rs` (`restore`, `undo_run`).

#### TODO

- [ ] Time Undo of a 500+ file move on Windows.
- [ ] If slow, restore in chunks while keeping the RAW-before-sidecar and never-overwrite rules.

### Engine: Delete Permanently cannot remove a read-only file on some platforms

#### Background

From `trash-over-scan` Step 5. A read-only file (a camera's protect flag) goes to the Trash but may fail `remove_file`, depending on the platform. It is then listed as a failure and stays, which is safe, but the user has no way to delete it from Riffle. Not tried.
Files: `crates/engine/src/trash.rs` (`remove_one`).

#### TODO

- [ ] On Windows, delete a read-only file with `Delete Permanently…` and see whether it fails.
- [ ] If it fails, decide whether to clear the read-only flag first or leave it.

### Core: split the disk read from `hevc::to_jpeg` so a read slot covers the read alone

`read_preview` ends in the HEVC decode of an HDR PQ CR3 (65-125 ms of CPU per file), and that runs inside `ReadSlots::acquire` in the three scan sites and inside the app's preview pause. With `READ_SLOTS` at 2 and no HDR PQ CR3 run in the sweep, a folder of such files pays a calculated 33-63 ms/file against 3-6 uncapped, while the permits sit idle at the disk. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`. Files: `crates/core/src/scan.rs`, `crates/core/src/reader.rs`, `crates/app/src/main.rs` (the preview load).

#### TODO

- [ ] Have a `reader` function return the undecoded `(arw, bytes, codec)` so the guards cover the bytes and `to_jpeg` runs outside them. Done when a slot and the pause no longer span the decode and a run on a folder of HDR PQ CR3 files shows the pass's ms/file at cap 2.

### App: measure the scan's read cap on NVMe

`READ_SLOTS` (2) was picked on a SATA SSD; no RAW folder was on the NVMe (`C:`) at the time. A shallower queue costs more on a drive that needs deep queues for throughput, so cap 2 may slow the thumbnail fill there. The harness the cap was measured with (`preview_during_pass_1_on_a_real_folder`) was a test of the Tauri crate and left with it. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`. Files: `crates/app/src/scan.rs` (`READ_SLOTS`), `docs/agents/shared-crates.md` (the read cap section).

#### TODO

- [ ] Put an unopened RAW folder on the NVMe, close the app and measure the preview read during pass 1 at caps 22 and 2 on cold data (a new harness, or the app's `page` timing lines). Done when `docs/agents/shared-crates.md`'s read cap section has the numbers and the cap is confirmed or changed.

### Core: test the scan's read slot in `embed_all` and the post-read cancel check

#### Background

`embed_all`'s per-file closure needs a loaded SigLIP model, which the tests lack, so its `ReadSlots::acquire` is checked only by reading the code. `a_file_canceled_mid_pipeline_is_not_delivered` now stops at `acquire` returning `None`, because nothing can set the flag during the read without a hook in `read_preview`, so the first post-read cancel check in `extract_unless` and `extract_analysis_unless` is untested. File: `crates/core/src/scan.rs`. Plan: `docs/plans/_archived/20261107-preview-read-gate/plan.md`.

#### TODO

- [ ] Add a test seam (a stub embedder or an injectable closure), and a test that holds the single permit and shows `embed_all` delivers nothing until it is dropped.
- [ ] Add a test-only hook or a reader stub that sets `cancel` during the read, and a test that a file canceled mid-read is not delivered, in both `extract_unless` and `extract_analysis_unless`.

### Core: the cold-disk cost of pass 2's full-size eye read is unmeasured

#### Background

`full-size-eye-check` Step 3 measured pass 2 on `D:\Photos\2026\2026-11-07` (3638 ARW) with a warm file cache: a measured frame cost about 51 ms more (mean and median), and the whole `riffle-cli features` run went from 80 s to 98 s. The cost of the second read on a cold disk is unmeasured, and the plan's "Cost" trade-off names it as the case to watch. Plan: `docs/plans/_archived/20261108-full-size-eye-check/plan.md`. Files: `crates/core/src/scan.rs` (`full_eye_sharp`).

#### TODO

- [ ] Time `handpic-cli features` (or a pass 2 in the app) on a freshly mounted card or after a reboot, with the same 8 threads and release build as Step 3, and compare it with the warm figures. If the second read is a noticeable share of the cost, consider reusing the first read's bytes.

### Engine: re-check that notify 8.2.2 still queues `configure` behind `Unwatch` when the app starts using `watch`

#### Background

`gpui-backend-crate` Step 4 moved the folder watcher into `crates/engine/src/watch.rs`, with `settle` moved in from `treewatch.rs`. `crates/app/Cargo.lock` resolves `notify` to 8.2.2, while the root `Cargo.lock` stays at 8.2.0. `docs/agents/shared-crates.md` says a notify upgrade must re-check that `configure` still queues behind `Unwatch`, which `watch::settle` relies on. Nothing in CI covers this on 8.2.2. The app now calls `watch::set` for the open folder's resync (`gpui-strip-index` Step 4, `crates/app/src/main.rs`), but `settle`'s `configure` barrier matters only for the rename path, so the re-check is still open. The resync's hand-runs (a copy and a delete in the watched folder) behaved as expected on 8.2.2 but did not exercise `Unwatch` / `configure` queuing. Plan: `docs/plans/_archived/20261109-gpui-backend-crate/plan.md`, `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/engine/src/watch.rs`, `crates/app/Cargo.lock`, `docs/agents/shared-crates.md`.

#### TODO

- [ ] When the app first uses `watch` (the ports' rename path), re-check on notify 8.2.2 that `configure` still queues behind `Unwatch`, so that `watch::settle` still holds. Either pin the app lock to the version the root lock holds, or confirm the queuing behavior and note the version in `docs/agents/shared-crates.md`.

### App: real-device checks of the strip, the scan progress, the resume and the resync

`gpui-strip-index` added the thumbnail strip, pass 1 of the scan, the resume at the last viewed file and the folder-watcher resync to the GPUI app (`crates/app`). CI and the unit tests cover the pure models (`strip.rs`, `focus.rs`, `viewed.rs`, `resync.rs`); stderr-instrumented debug runs covered the scan, the resume and the resync. Nobody has watched the running window: the strip's drawing, the status line's `scanning done/total` climb, the memory behavior while scrolling, the resume and steered scan on a 1000+ file folder, and the resync from Explorer copies and deletes are all unchecked. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/app/src/main.rs`, `crates/app/src/strip.rs`, `crates/app/src/thumbs.rs`, `crates/app/src/focus.rs`, `crates/app/src/viewed.rs`, `crates/app/src/resync.rs`.

Build with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build --release` in `crates/app`, and use a fresh `--index <file under D:\Photos\tests\>` for each run so pass 1 runs.

#### TODO

- [ ] On Windows, open a folder of a few hundred RAW files under `D:\Photos\` with `handpic.exe <folder> --index <fresh file>`. Expect `scanning N/total` after the file info, N rising about every 100 ms, replaced at the end by nothing (or `N failed`).
- [ ] On Windows, open a folder of 1000+ RAW files under `D:\Photos\` with a fresh index. Expect, in order: (1) while `scanning N/total` climbs, the gray placeholders turn into thumbnails from the current cell outward, the middle of the strip first, each cell with its file name and the current cell outlined; (2) holding `→` for a few seconds moves the outlined cell and the preview together, the strip scrolls just enough to keep the cell in view, and on release the preview, the outline and the status line name the same file, with `Home` / `End` jumping both to the first / last file; (3) wheeling the strip (toward you scrolls right) to a far cell and clicking it shows that file and moves the outline, while a click in the 8 px gap between cells does nothing; (4) with Task Manager's Details tab showing the `handpic.exe` row's private working set (or Commit size), wheeling the strip from one end to the other and back several times raises the value while scrolling and lets it settle to about the same level, not climbing run after run; (5) on a folder holding a corrupt RAW (e.g. `D:\Photos\tests\20261110-gpui-strip-index\ARW`, file `DSC-RX100M3_corrupt.ARW`) its cell shows `invalid TIFF byte order` in the gray box instead of a thumbnail.
- [ ] On Windows, on a 1000+ RAW folder with a fresh index (`manual3.sqlite`), wait for the scan to end, press `End`, then `←` a few times, wait a second and quit with `q`. (a) Run again with the same arguments: the preview, the outlined strip cell and the status line open at the file left at, and the strip is scrolled to it. (b) Run on the same folder with another fresh index (`manual3b.sqlite`) and press `End` right after the window opens: the thumbnails near the last cell appear within a second or so, before the middle of the folder is scanned (`scanning N/total` still climbing), and wheeling the strip to the middle and stopping fills the cells there next.
- [ ] On Windows, copy a few ARW files into a new folder under `D:\Photos\tests\` (e.g. `D:\Photos\tests\20261110-gpui-strip-index\manual4`) and run `handpic.exe <that folder> --index <fresh file>`. Page to the second file. (1) From Explorer, copy another ARW into the folder: within about a second its cell appears in the strip at its name's place and gets a thumbnail, the outlined cell, the preview and the status line still name the same file, the status line's total grows by one, and the other thumbnails do not flash back to gray. (2) From Explorer, delete a file that is not the current one: its cell disappears and the current file stays current. (3) Delete the current file: the preview, the outline and the status line move to the file after it (the one before it when it was the last).

### App: measure the scan pause in `load` with presses on files pass 1 has not read yet

`gpui-strip-index` Step 5's during-scan run stepped to the next file, which the hot list had already made the scan read (a cache hit of about 1 ms), so `ReadSlots::pause` around `read_preview` showed no effect at p50 / p95. The one 336.75 ms read without the pause came on a disk slowed by 40 GB of just-written copies, so it proves nothing. Tauri's figure (14-16 ms read wait at cap 2 with the pause) is for a read of an unscanned file. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md` (`results.md` in the same folder). Files: `crates/app/src/main.rs` (`load`), `crates/app/src/focus.rs`, `crates/app/src/args.rs` (`--auto`).

#### TODO

- [ ] Run `--auto` with presses that land on unread files (a jump such as `End`, or a stride), on equal cold copies (`robocopy /J`, one per run) and a settled disk, pause on against off, and record `read` p50 / p95 next to Tauri's. Done when `docs/agents/app.md` says whether the pause shortens the read.

### App: decide whether the strip's image box grows from 144x96 to the Tauri strip's 144x144

`gpui-strip-index` built the plan's 144x96 image box, but the Tauri strip's `img` was 144x144 (its `style.css`'s `.cell img`, `object-fit: contain` inside a 168 px cell), so a portrait thumbnail (orientation 6 / 8) shows at 64x96 in the native strip against 96x144 in the Tauri one. Since `native-strip-resize` the strip's height is draggable and the image box is derived from it (`strip::Geometry::of`, a 3:2 box; 144x96 at the default 128 px), so a taller strip enlarges portrait thumbnails too, but the box stays 3:2 and the default stays 144x96. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/app/src/strip.rs` (`Geometry`), `crates/app/src/main.rs` (`paint_strip`).

#### TODO

- [ ] Decide whether the default strip height, or the box's aspect, changes so the app's cell matches the Tauri size, now that the user can drag the strip taller (`stripheight.rs` `DEFAULT`, `strip::Geometry`).

### App: `present` p95 rose from about 0.2 ms to 0.55-0.85 ms on ARW with the strip

In `gpui-strip-index` Step 5's A/B (`results.md`), `present`'s p95 on the ARW compare folder rose with the strip while RAF was unchanged. It is not gated and stays under 1 ms. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/app/src/main.rs` (`paint_strip`).

#### TODO

- [ ] When a later strip change (marks, badges) lands, re-run the same-session A/B and check that `present` p95 has not grown past 1 ms; if it has, look at `paint_strip` first.

### App: the in-app `keypressToPixels` stops about 48 ms before the DWM present on 4416x2944 RAF previews

The `tauri-gpui-latency-ab` same-session A/B measured both apps against one external ruler, keypress (`SendInput`) to the DWM present of the frame that shows the new image. On the GPUI app the DWM-present `screen` number exceeds the in-app `keypressToPixels` (which includes `frameWait` and `present`) by 13.55 ms on the 1616x1080 ARW previews but by 47.95 ms on the 4416x2944 RAF previews (median of 3 runs, p50). So on the large image the in-app `present` stamp precedes the frame reaching the DWM by about 34 ms more than on ARW. A hypothesis, not a finding: the large texture's upload or sampling on the GPU. Nothing in the data separates it from other causes. The plan did not follow it up, and the measurement tooling stays on the local-only branch `measure/tauri-latency-ab`. Plan: `docs/plans/_archived/20261120-tauri-gpui-latency-ab/plan.md` (see `results.md` beside it). Files: `crates/app/src/main.rs` (`present`).

#### TODO

- [ ] Find where the extra ~34 ms on RAF goes (for example a GPU-side timestamp or a PresentMon trace of the GPUI process on the RAF folder, compared with the ARW folder), and decide whether the in-app `present` stamp should move to match the DWM present. Until then, treat the in-app RAF `keypressToPixels` (p50 67.51 ms) as under-reporting what the screen shows (`screen` p50 115.46 ms).

### Agents: `GIT_DIR` / `GIT_WORK_TREE` exported around `mise run ci` leak into `merge.test.sh` and write to the real repository

#### Background

`gpui-zoom-peaking` Step 2 Phase B: after the repository folder was renamed, a worktree's `.git` pointed at a stale path, and a session worked around it by exporting `GIT_DIR` / `GIT_WORK_TREE`. That leaked into `mise run ci`, where the merge-settings skill's `merge.test.sh` runs `git -C <tmp> init`, `commit` and `worktree add`; they landed in the real repository (an empty `init` commit, a branch `other`, a worktree entry `hooks-other`). The fix was `git worktree repair <worktree>` from the renamed repository. Plan: `docs/plans/_archived/20261110-gpui-zoom-peaking/plan.md`. Files: the merge-settings skill's `merge.test.sh`, `CLAUDE.md` (Development section).

#### TODO

- [ ] Either add a line to `CLAUDE.md`'s Development section ("never export `GIT_DIR` / `GIT_WORK_TREE` around `mise run ci`"), or make `merge.test.sh` unset both variables (fixing the cause). Done when the line is in `CLAUDE.md`, or the test script unsets them and a test shows it cannot touch the outer repository.

### App: private bytes grow while `→` is held in the 1:1 view

#### Background

In `gpui-zoom-peaking`'s Step 2 smoke run, two 4 s zoomed holds of `→` took private bytes from 418 to 511 to 557 MB without peaking (549 -> 593 MB with it), while the fit-view holds settled near 400-425 MB. Each zoomed page reads the full JPEG (`reader::read_full`) and decodes a crop. Whether this is allocator retention that plateaus, or a crop or texture that is not released, is unknown. Plan: `docs/plans/_archived/20261110-gpui-zoom-peaking/plan.md`. Files: `crates/app/src/zoom.rs`, `crates/app/src/main.rs` (`zoom_page`, `crop_arrived`).

#### TODO

- [ ] Hold `→` zoomed for longer runs (Task Manager, Details, Commit size) and check whether private bytes plateau. If they do not, find the crop or `RenderImage` that is not released (check `retired`).

### App: real-device checks of the held-key preview and crop (gpui-held-key-preview)

#### Background

`gpui-held-key-preview` made the GPUI app (`crates/app`) draw a stale result before requesting the current one, so holding Left / Right flips through frames in the fit view (Step 1, `Pager::finish` and `loaded()`) and in the 1:1 view (Step 2, `Zoom::page` and `Zoom::arrived`). Both steps were ticked on their automated criteria (`pager.rs` and `zoom.rs` unit tests, `mise run ci`, `mise run app:check`). Nobody has held a key in the running window, where a read outlasting the key repeat is what matters, so the drawn frames, the overlays and the `page` and `zoom` timing lines are unverified. Plan: `docs/plans/_archived/20261113-gpui-held-key-preview/plan.md`. Files: `crates/app/src/pager.rs`, `crates/app/src/zoom.rs`, `crates/app/src/main.rs`.

#### TODO

- [ ] On Windows, fit view: open a folder in the app (`mise run app:dev`), ideally RAF files whose preview load outlasts the key repeat, and hold `→`. Expected: the preview flips through frames during the hold (skipping some files) instead of staying on the first one until release. On release the current file is shown with its `f` overlay (AF mark, face boxes, mesh), its meta pane and a `page` line. No frame ever shows another file's marks.
- [ ] On Windows, 1:1 view: on a folder whose crop read outlasts the key repeat (RAF, ARW), press `z`, then hold `→`. Expected: the 1:1 view flips through crops during the hold (skipping some files) instead of staying on the first one until release. No crop is ever drawn over another file's scaled preview. On release the current file's crop is shown. A single press while zoomed goes old crop, new crop. `z` off / on on the same file reuses its crop with no new `zoom` line, and a `z` press prints one `zoom` line.

### App: real-device checks of the meta pane, the judgment keys, the sidecar writes and undo / redo

`gpui-judgments` added the meta pane, the view-only JPEG folder, the strip's judgment marks, the judgment keys through the keymap, the index and sidecar writes and undo / redo to the GPUI app (`crates/app`). Each step's checkbox was ticked on its automated criteria (unit tests, `--auto` runs); nobody has watched the running window or pressed the keys by hand, and the window close button's writer drain has not been seen. Plan: `docs/plans/_archived/20261110-gpui-judgments/plan.md` (the pending manual checks in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/meta.rs`, `crates/app/src/viewonly.rs`, `crates/app/src/strip.rs`, `crates/app/src/keys.rs`, `crates/app/src/judgment.rs`, `crates/app/src/history.rs`, `crates/app/src/settings.rs`.

Windows only. Build with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build` in `crates/app` and run `target\debug\handpic.exe <folder> --index D:\Photos\tests\20261110-gpui-judgments\index\<new name>.sqlite --settings D:\Photos\tests\20261110-gpui-judgments\index\<new name>-settings.sqlite [--import-settings <file>]`, a fresh index and settings store file name for every run (the settings store is seeded from the `--import-settings` JSON only when new). Work only on the copies under `D:\Photos\tests\20261110-gpui-judgments\` (or the read-only test folders named below), and never open a folder under `D:\Photos\` outside `D:\Photos\tests\`. The settings files are in `D:\Photos\tests\20261110-gpui-judgments\settings\` (`xmp.json`, `both.json`, `rot.json` = `{"sidecarFormat":"xmp","labelNames":{"red":"Rot"}}`, `default.json` = `{}`). Check each sidecar with `cat`.

#### TODO

- [ ] Meta pane (Step 2). (1) Open `D:\Photos\tests\20261108-gpui-compare\ARW` with `--index ...\index\manual-arw.sqlite`: a 220 px pane right of the preview shows the file name, `n / 20`, an `EXIF` group (Aperture, Shutter, ISO, Focal length, Exposure, Camera, Lens, Captured, each present in the file) and a `Maker note` group for a Sony file; the arrow keys update it with the preview; no notes line. (2) Open `D:\Photos\tests\2026-09-28-gui-check-jpeg` with `--index ...\index\manual-jpeg.sqlite`: `EXIF` rows only, no `Maker note` group, and `JPEG folder: view only` at the bottom of the pane. (3) Optionally, on a copy holding a corrupt file, the pane shows the file name and the error alone.
- [ ] Strip marks and key names (Step 3). With `RUST_LOG=debug` set, open `D:\Photos\tests\20261110-gpui-judgments\read\lr-check-4-lightroom-desktop` with `--import-settings ...\settings\xmp.json` (and a fresh `--settings` store). Once pass 1 ends: L1005036 three gold stars at the image box's top-right, a green dot at its top-left and a yellow name band; L1005037-40 four to one stars; L1005041 a green dot; L1005043 a red dot and the cell dimmed; L1005044 a red name band. The star glyph renders (font fallback) and reads over a bright thumbnail. Repeat with `both.json` and another fresh index: the `.dop` is newer for most files, so the marks follow `reconcile_of`'s newest-wins pick. In the same run press Shift+X, Ctrl+Alt+1, Ctrl+Alt+0, Ctrl+Z, Ctrl+Shift+Z, `1`, `x`, Shift+1 and the arrows, and read the `[handpic][DEBUG] key: <name> action=...` lines on stderr: `shift+x` -> `rejectRest`, `ctrl+alt+1` -> `red`, `ctrl+alt+0` -> `clearlabel`, `ctrl+z` -> `undo`, `ctrl+shift+z` -> `redo`, `1` -> `rate1`, `x` -> `reject`, Shift+1 -> `!` with no action, the arrows page; note whether Ctrl+Alt+digit reaches the window at all (on a layout with AltGr it is AltGr+digit). Nothing is written: afterwards `ls -l --time-style=full-iso *.xmp *.dop` in the folder still matches `D:\Photos\tests\20261110-gpui-judgments\read\sidecars-before.txt`. Record the key names that arrived in `docs/agents/app.md`'s "Every keystroke goes through the keymap" section.
- [ ] Sidecar writes (Step 4), on the copies in `D:\Photos\tests\20261110-gpui-judgments\write\` (four ARWs each, no sidecars; three JPEGs in `jpeg`). (1) `write\xmp`, `xmp.json`: on a file press `3`, go back (auto-advance moves on), `p`, back, `Ctrl+Alt+1`: within a second `<stem>.xmp` holds `xmp:Rating="3"`, `xmpDM:good="True"`, `photoshop:LabelColor="red"`, `xmp:Label="Red"`; then `0`, `u`, `Ctrl+Alt+0`: `Rating="0"`, no `good`, no label; on another file mash `1` `2` `3` within 300 ms: one mtime change, `Rating="3"`. (2) `write\both`, `both.json`: the same keys give `<stem>.xmp` and `<name>.dop` holding `Rating = 3`, `ShouldProcess = 0` (PhotoLab's pick) and `ColorLabel = "Red"`, random Uuids unless PhotoLab has the copy registered. (3) `write\rot`, `rot.json`: `Ctrl+Alt+1` writes `xmp:Label="Rot"`. (4) `write\crash`, `xmp.json`: press `4` and `taskkill /F /IM handpic.exe` within 300 ms (inside the debounce); reopen with the same `--index`: stderr shows `scan sidecars: ... dirty=1` and the sidecar gets `Rating="4"`. (5) `write\drain`, `xmp.json`: press `5` and `q` at once: stderr ends with `quit: sidecar writer drained=true` and the sidecar holds `5`; repeat closing the window with its close button instead of `q`: the same line, the same `5`. (6) `write\advance`, `default.json`: `3` moves to the next file; on the last file `3` stays; `Ctrl+Alt+2` stays on the file; the arrows still wrap. (7) `write\jpeg`, `xmp.json`: `3` does nothing (no stars, the view stays) and no file appears next to the JPEGs. No failure line in the pane's notes throughout. Record what was seen, the window close button's drain above all, in `docs/agents/app.md`'s "The writer drains in `App::on_app_quit`" section.
- [ ] Undo / redo (Step 5), on `D:\Photos\tests\20261110-gpui-judgments\undo\` (`DSC-RX0M2_sony_rx0_ii_40.arw`, `DSC-RX0_DSC00009.ARW`, `DSC-RX0_sony_rx0_07.arw`, `DSC-RX100M2_DSC00416.ARW`, no sidecars) with `--import-settings ...\settings\xmp.json` (and a fresh `--settings` store); wait for pass 1 to end and press `Home`. (1) `3` on file 1: three stars on its cell, the view moves to file 2; `p` on file 2: a green dot, the view moves to file 3; after a second file 1's `.xmp` holds `xmp:Rating="3"` and file 2's `xmpDM:good="True"`. (2) `Ctrl+Z`: file 2's dot goes, the view is on file 2, the notes say `Undid DSC-RX0_DSC00009.ARW`, its `.xmp` loses `good` within a second. (3) `Ctrl+Z` again: the view is back on file 1, its stars go, the note says `Undid DSC-RX0M2_sony_rx0_ii_40.arw`, its `.xmp` holds `Rating="0"`. (4) `Ctrl+Shift+Z`: file 1's stars come back (`Rating="3"`), the note says `Redid ...`. (5) A new key (`1` on any file), then `Ctrl+Shift+Z`: nothing changes, no note. (6) Quit with `q`, reopen with a new `--index`: `Ctrl+Z` does nothing; open `D:\Photos\tests\20261110-gpui-judgments\write\jpeg` the same way: `Ctrl+Z` does nothing there either. Record what was seen in `docs/agents/app.md`'s "A judgment goes view, row, writer" section.

### Core: `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` hits its 1 s bound under parallel load

During `gpui-judgments`' local `mise run ci`, this test took 2.91 s against its 1 s wall-clock bound under parallel load; run alone it took 0.38 s. The bound measures the machine's load as much as the cancel. Plan: `docs/plans/_archived/20261110-gpui-judgments/plan.md`. Files: `crates/core/src/scan.rs` (the test, around line 1441).

#### TODO

- [ ] Loosen the bound, or wait on a condition (the scan's end) instead of the wall clock, and confirm `mise run ci` passes with full parallelism.

### App: real-device checks of the f focus mark, face boxes and face mesh

`gpui-focus-marks` added pass 2, the `f` focus mark, the face boxes with landmark dots and labels, and the on-demand eye judgment with the face mesh to the GPUI app (`crates/app`). CI and the unit tests cover the pure ports (`mark.rs`, `faces.rs`, `eyes.rs`, `facemesh.rs`), and logged coordinates from debug runs matched hand computations. Nobody has put the GPUI window beside the Tauri app, because no screenshots were possible. The status line's `analyzing done/total` text and a folder switch mid-pass 2 from the UI were not checked either. Plan: `docs/plans/_archived/20261110-gpui-focus-marks/plan.md`. Files: `crates/app/src/mark.rs`, `crates/app/src/overlay.rs`, `crates/app/src/faces.rs`, `crates/app/src/eyes.rs`, `crates/app/src/facemesh.rs`, `crates/app/src/main.rs`.

Test files are under `D:\Photos\tests\20261110-gpui-pass2\marks\`. Run `mise run app:dev -- <folder> --index <fresh file in that test folder>` for the app, and, for the comparison, open the same folder in a reference Tauri build with its own index (never share one): an installed Riffle 1.10.0, or a build of the commit before #1150, which removed the Tauri app.

#### TODO

- [ ] On Windows, press `f` in the app and the reference Tauri build and step through `ILCE-1M2_sony_a1_ii_03.arw`, `ILCE-7M4_DSC07652.ARW`, `ILCE-6300_Free_raws_by_signatureeditsco_DSC03068.arw`, `ILCE-6400_DSC01340.ARW`, `ILCE-6400_DSC00405.ARW`, `_DSC1616.ARW`, `_DSC1758.ARW` and `_DSC1760.ARW`. Expect on each file the crosshair and the AF frame on the same spot of the picture with the same color (bright green on `_DSC1758`, gray on `_DSC1616`, white on the rest), no mark on `ILCE-6400_DSC00405.ARW`, and the arm ends keeping the dark outline.
- [ ] On Windows, with `f` on in both, check at least `_DSC1758.ARW` (nine faces), `ILCE-6400_DSC01340.ARW` (orientation 6), `ILCE-1M2_sony_a1_ii_03.arw` (orientation 8) and `ILCE-6400_DSC00405.ARW` (a face box without an AF mark). Expect the same boxes, five cyan dots per face and label text in both, each label upright above its box's top-left corner (inside the box at the image's top edge, pulled left at the right edge), and the labels readable over a bright face with the app's offset-copy halo (`docs/agents/app.md`). If they do not read, switch `paint_label` to a dark quad behind the text.
- [ ] On Windows, with `f` on in both, stop on `ILCE-6300_Free_raws_by_signatureeditsco_DSC03068.arw` (closed eyes, orientation 8), `_DSC1758.ARW` (open eyes) and `ILCE-6400_DSC01340.ARW` (portrait, orientation 6), waiting for the judgment (up to 2 s in a debug build). Expect the magenta mesh (face oval, eyes, brows, nose, lips) on the same face with the same lines in both, over a dark outline, and the iris rings and two iris dots on the open-eyed frames only. Also hold `→` with `f` on and check that the log's `eyes` lines name only files the paging stopped on or passed slowly.
- [ ] On Windows, on a folder of a few hundred RAW files with a fresh `--index`, check that the status line shows `analyzing N/total` after `scanning N/total` and ends clean, and that switching to another folder mid-pass 2 from the folder tree stops it (reopening the first folder resumes the remaining files).

### App: quitting while a re-listing is stopping the previous scan does not wait for that scan

The native-scan-stop work (`docs/plans/_archived/20261113-native-scan-stop/plan.md`) made every quit path stop the held scan with a bounded wait (`scan::Running::stop_within`, `STOP_TIMEOUT`) in the app-level `on_app_quit` handler. `list` moves the previous scan out of the holder into a background task that calls the unbounded `Scan::stop`. If the user quits while that task is still winding the scan down, the holder is empty, so the handler does not wait for it. The process can then exit before that scan's `scan faces:` line, and its write transaction may be cut. This was left out as rare and as needing more state, and the hand check did not exercise it. Files: `crates/app/src/scan.rs`, `crates/app/src/main.rs`, `docs/agents/app.md`.

#### TODO

- [ ] Let the quit handler also wait, bounded by `STOP_TIMEOUT`, for a scan that a re-listing is stopping in the background (for example by keeping that scan, or a join signal for it, in the shared holder until its stop returns), and log the outcome on the `quit:` line.

### App: real-device checks of the strip's sort order, the filter and the filter / sort menus

`gpui-filter-sort` added the strip's sort order, the filter (flags, stars, labels, orientation, `Face focus`, `Eyes`, `Output`) and the status row's `Sort ▾` / `Filter ▾` menus to the GPUI app (`crates/app`). CI, the unit tests (`sort.rs`, `filter.rs`, `menu.rs`) and debug `--auto` runs covered the page order and the log lines headlessly. Clicks and key presses cannot be driven by `--auto`, so nobody has watched the strip's drawing, pressed a judgment key under a sort or a filter, or clicked a menu. Windows only. Plan: `docs/plans/_archived/20261113-gpui-filter-sort/plan.md` (the pending manual checks in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/sort.rs`, `crates/app/src/filter.rs`, `crates/app/src/menu.rs`.

Build in `crates/app` with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build`, then run `target\debug\handpic.exe <folder> --index <fresh path> --settings <fresh .sqlite path> --import-settings <file>`, with a fresh index and settings store per run (the store is seeded from the `--import-settings` JSON only when new); "with `settings-x.json`" below means `--import-settings` that file. The folders and settings files below are copies under `D:\Photos\tests\20261113-filter-sort\`; use only those, and never open a folder under `D:\Photos\` outside `D:\Photos\tests\`. Record what was seen in `docs/agents/app.md`, above all whether a click inside the deferred popover reaches its `on_click` before the strip's `on_mouse_down`, and whether a click on a strip cell that closes the menu also selects the cell.

#### TODO

- [ ] Sort (Step 1): open `sort\raw` with `settings-rating.json`. Once pass 1 ends the strip shows HX99, RX0, RX0M2, RX100M2, RX100M3, HX95 (dimmed). On RX100M2 press `3`: the strip reorders at once with RX100M2 right after HX99, it stays current (outlined, preview unchanged), and auto-advance then moves to its new right neighbour (RX0). Repeat with `settings-capture.json` (strip in capture order), and open `sort\jpg-sequenced` with `settings-name.json` (strip in capture order: C2000Z, C960Z, COOLPIX, A065).
- [ ] Filter through the judgment keys and undo (Step 2): open `filter\raw` with `settings-filter-untagged.json`. Once pass 1 ends the strip shows the 5 unflagged files (HX95 absent) and the pane's total is 5; `p` on the current file hides it and lands on the next shown file, with no second move from auto-advance; `Ctrl+Z` brings it back and the view lands on it (`Undid <name>`). On `filter\stars3` (RX0 has 3 stars in its XMP) with `settings-filter-stars3.json`: only RX0 shows, and `x` on it keeps it shown (a reject keeps its stars). With `settings-filter-unjudged.json` only RX0M2, RX100M2 and RX100M3 show. `settings-filter-none.json` (picked only; no picked file) shows `No files match the current filter.` on the status line and in the pane, with no image. Copying a RAW into the folder while filtered re-lists without losing the current file. The `p` / undo / redo sidecar writes land in the copies only.
- [ ] `Face focus` and `Eyes` (Step 3): open `faces` with `settings-filter-good.json`. The strip starts empty and fills in while `analyzing n / m` runs, ending with DSC00405, _DSC1758 and ILCE-1M2_sony_a1_ii_03, each with a bright green `f` mark. With `settings-filter-closed.json` only DSC03068 shows.
- [ ] The menus and the `Output` section (Steps 3 and 4): open `menu\raw` with `settings-name.json`. `Sort ▾` and `Filter ▾` sit at the status row's right end and the preview is the same size as before. Click `Filter ▾`: the menu opens upward over the strip, its right edge on the button's; check `Picked`, then `Red`: the menu stays open, each gets a check mark, the strip narrows (no picked or red file in `raw`: `No files match the current filter.`; use `menu\faces` after picking one with `p` to see a narrowed strip) and the button lights; a click on an item does not also select a strip cell under it. `Escape` closes it; reopen and click the preview: it closes; reopen and click a strip cell: it closes (record whether the cell is also selected); click `Filter ▾` while open: it closes and stays closed. `Reset` unchecks all and shows every file, the button no longer lit. `Sort ▾` then `Rating`: the strip reorders (HX99, RX0, RX0M2, RX100M2, RX100M3, HX95), the menu closes and the current file stays current and in view. With the filter menu open, hold `→`: the pager pages; `e`, `z`, `f` still toggle peaking, 1:1 and the focus mark. On `menu\jpg-sequenced`: `Sort ▾` is dimmed and does nothing, the strip is in capture order. On `menu\faces` the filter menu shows the `Output` section (`Output`: DSC00405, _DSC1616, _DSC1758; `No output`: the other 5); renaming `Output\` away hides the section (its checks cleared) and renaming it back brings it back; on `menu\raw` (no `Output\`) the section is absent. The settings file is untouched.

### App: real-device checks of the EXIF filter sections and the refilter fixes

`gpui-exif-filter` added the per-folder EXIF sections (`Camera`, `Lens`, `Aperture`, `Shutter speed`, `ISO`, `Focal length`) to the GPUI app's filter menu, made the strip position (then the meta pane's `n / total`, now the status bar's, see `native-status-position`) follow a refilter that hides the shown file, and kept the anchor across an empty list (`filter::KeptAnchor`). CI, the unit tests (`exif.rs`, `filter.rs`, `menu.rs`) and one debug `--auto` run covered the build, the group order, the check semantics and the pure rules headlessly. Clicks and judgment keys cannot be driven by `--auto`, so no one has opened the menu, checked an item or pressed `p` under a filter. Windows only. Plan: `docs/plans/_archived/20261113-gpui-exif-filter/plan.md` (the pending manual checks in `learnings.md` in the same folder). Files: `crates/app/src/exif.rs`, `crates/app/src/filter.rs`, `crates/app/src/menu.rs`, `crates/app/src/main.rs`.

Build in `crates/app` with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build`, then run `target\debug\handpic.exe <folder> --index <fresh path> --settings <file>`, with a fresh index per run. Use only copies under `D:\Photos\tests\` (a folder with several cameras, such as `D:\Photos\tests\20261113-filter-sort\sort\raw`), and never open a folder under `D:\Photos\` outside `D:\Photos\tests\`.

#### TODO

- [ ] On Windows, the filter menu ends with `Camera`, `Lens`, `Aperture`, `Shutter speed`, `ISO`, `Focal length`, each listing the folder's values in order; it scrolls when taller than the window. On a fresh index the sections appear once pass 1 ends.
- [ ] On Windows, checking one camera narrows the strip and lights the button; a second camera widens it (OR); an ISO item on top narrows it (AND); `Reset` clears all.
- [ ] On Windows, copying in a RAW from another camera adds its item after the re-scan; removing the only file of a checked camera drops that check instead of leaving an empty strip.
- [ ] On Windows, under the `untagged` filter, `p` on the current file updates the status bar's `n / shown (listed)` position at once (the name follows when the next preview arrives).
- [ ] On Windows, with a filter that hides every file, `Reset` or unchecking the item comes back to the file shown before, not the first one.
- [ ] On Windows, the settings file is untouched (its modification time unchanged).
- [ ] On Windows, on a fresh index with `untagged` plus a `Face focus` item checked, press `p` repeatedly while pass 2 runs: the image on screen must always end up the current file's, never a judged file's left behind (the `shown_after` fix in `refilter`, found by reading only; the race is timing-dependent and may not show either way).

### App: `Camera` / `Lens` order is a case-insensitive compare, not the Tauri `localeCompare`

The app's EXIF filter orders the `Camera` and `Lens` items by a case-insensitive compare with a plain compare as the tie-break, because `crates/app` has no ICU collator. Tauri used `localeCompare`, so the two may differ for punctuation or non-ASCII model names. Found in `gpui-exif-filter` (the plan's trade-off). Plan: `docs/plans/_archived/20261113-gpui-exif-filter/plan.md`. Files: `crates/app/src/exif.rs` (`compare`).

#### TODO

- [ ] Compare the two orders on a folder with punctuated or non-ASCII camera / lens names, against the reference Tauri build named under "App: real-device checks of the f focus mark, face boxes and face mesh" (an installed Riffle 1.10.0, or a build of the commit before #1150) or against `Intl.Collator` / `localeCompare` results for the same names, and, if they differ in a way that matters, order them with a collator (`icu_collator`) or a closer rule.

### App: real-device checks of the settings store and the settings modal (sort / filter persistence, Culling, Sidecar, Keyboard Shortcuts, label names, Import)

`gpui-settings` gave the GPUI app (`crates/app`) its own settings store (a SQLite file seeded once from the Tauri `settings.json`), persisted the strip's filter and sort order in it, and ported the settings modal: the Culling, Sidecar, Keyboard Shortcuts and Import tabs, opened with Ctrl+, (Meta+, on macOS). CI and the unit tests (`settingsmodal.rs`, `settings.rs`, `textinput.rs`, `filter.rs`, the engine's `store.rs`, `sidecar.rs` `switch_to` and `import.rs` `today_example`) cover the pure models and the store. Only Step 1's store seed was hand-checked, headless. Nobody has opened the modal, clicked or typed in it, used the OS folder dialog, or seen the Japanese IME compose in the text input: every step's checkbox was ticked on the automated criteria. Windows only. Plan: `docs/plans/_archived/20261113-gpui-settings/plan.md` (the steps are in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/settings.rs`, `crates/app/src/settingsmodal.rs`, `crates/app/src/settingsview.rs`, `crates/app/src/textinput.rs`, `crates/engine/src/store.rs`, `crates/engine/src/sidecar.rs`, `crates/engine/src/import.rs`.

Build in `crates/app` with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build` first. Work only on copies under `D:\Photos\tests\20261113-settings\` with a fresh `--index` per run and an explicit `--settings <test store>.sqlite` (plus `--import-settings <test json>` where a seed is needed); never the default index or store, never the real Tauri `settings.json`, and never a folder under `D:\Photos\` outside `D:\Photos\tests\`. In every command below, `...\` stands for `D:\Photos\tests\20261113-settings\`, and the run is `target\debug\handpic.exe <folder> --index ...\<index> --settings ...\<store> --log ...\Handpic.log`. Record what was seen in `docs/agents/app.md`, above all the key names that arrive in the shortcut capture for Shift+1 and Ctrl+Alt+1 (the open question in "Every keystroke goes through the keymap": the code expects `!` and `ctrl+alt+1`).

#### TODO

- [ ] On Windows, sort order and filter survive a restart (Step 2). Run on `...\photos` with `--index ...\index-a.sqlite --settings ...\step2.sqlite` (fresh, nothing seeded). In `Sort ▾` pick `Rating`; in `Filter ▾` check `Picked` and `Landscape`, and also an `Output` or EXIF item if shown. Quit by closing the window; the log shows `quit: settings writes drained=true`. Start again with the same `--settings` and a fresh `--index` (`index-b.sqlite`). Expect the startup `settings:` line to show `sortOrder=rating` and a filter with `flags: [Picked]` and `orientations: [Landscape]`; the sort menu has `Rating` checked, the filter button is lit with `Picked` and `Landscape` checked, and the `Output` / EXIF items are unchecked. Then `Filter ▾` → `Reset`, quit, start again with a fresh `--index` (`index-c.sqlite`): the `settings:` line shows an empty filter, the filter button is not lit, and the sort is still `Rating`.
- [ ] On Windows, the modal's shell and the Culling tab (Step 3). Run on `...\photos` with `--index ...\index-s3a.sqlite --settings ...\step3-hand.sqlite`. Open `Filter ▾`, then press Ctrl+,: the menu closes and the Settings box appears centered over a dimmed window with the tabs Sidecar (selected), Culling, Keyboard Shortcuts and Import. With it open, press the arrow keys, `q`, `1` and `x`, click a strip cell and wheel over the strip: the preview, the strip and the rating do not change and the app does not quit. ArrowDown / ArrowUp / Home / End move the selected tab; Tab / Shift+Tab move a visible focus border through the close button, the selected tab and (on Culling) the auto-advance checkbox, wrapping; Space or Enter on the focused checkbox flips it. Close with Escape, with the `×` button, and with a press on the dimmed area; after each close ArrowRight pages to the next file. On Culling turn auto-advance off and close, press `1`: the rating is set and the file stays. Quit, start again with the same `--settings` and a fresh `--index` (`index-s3b.sqlite`): the `settings:` line has `autoAdvance=false` and the checkbox is off.
- [ ] On Windows, the sidecar format switched live (Step 4). Prepare `...\formats\` with two or three ARW copies, each with a `.xmp` and a `.ARW.dop` holding different ratings (e.g. XMP 2, `.dop` 4), and `...\jpegs\` with a few JPEGs only; write `...\step4-seed.json` holding `{"sidecarFormat":"xmp"}`. Run on `...\formats` with `--index ...\index-s4a.sqlite --settings ...\step4.sqlite --import-settings ...\step4-seed.json` (fresh store): the strip shows the XMP ratings. Ctrl+, → Sidecar: `Lightroom (.xmp)` is selected; choose `PhotoLab (.dop)`: the log has `sidecar format: xmp -> dop` then a `resync: dir=…` re-listing, and the strip and pane show the `.dop` ratings; choosing it again logs nothing. Close, press `5` on a file: only its `.ARW.dop` changes (the `.xmp` mtime and bytes stay). Switch to `Both (.xmp and .dop)`, press `3`: both files hold 3 (also, through the MCP companion if it is on, a `set_judgment` right after a switch writes the new format). Quit, start again with the same `--settings`, a fresh `--index` (`index-s4b.sqlite`) and no `--import-settings`: the `settings:` line has `sidecarFormat=both` and the tab shows `Both` selected. Open `...\jpegs` the same way (fresh `--index`) and switch the format: no error in the status line or the log, and the folder re-lists view-only.
- [ ] On Windows, the Keyboard Shortcuts tab edits the keymap live (Step 5). Run on `...\photos` with `--index ...\index-s5a.sqlite --settings ...\step5.sqlite` (fresh). Ctrl+, → Keyboard Shortcuts: about 40 rows in the Tauri order and labels (`Previous`, `Next`, ... `Clear`), the list scrolls, `Reset all` is under it, `Show / hide both side panes` holds `tab`. On `Pick` press `+` (`Press a key...` appears), then `k`: a `k` chip appears next to `p`; close, and `k` on a file picks it. Open again, remove `k` with its `×`; close; `k` does nothing. `+` on `Pick` then `x`: the status line says `"x" is bound to reject` and nothing changes; `+` then `q`: `"q" is taken by this app`; `+` then Escape: the capture ends and the modal stays open; `+` then switch to the Culling tab and back: no capture is shown; `×` on `p` alone: `"p" is the only key for pick; use Reset`. Add `k` to Pick again, quit, start again with the same `--settings` and a fresh `--index` (`index-s5b.sqlite`): `k` picks and the row shows `p` and `k`. `Reset all`, quit, restart (`index-s5c.sqlite`): only `p`, and the store holds no `shortcuts` key. Note which names arrive in the capture for Shift+1 and Ctrl+Alt+1 (Ctrl+Alt+1 is refused as bound to red, the message naming it) and record them in `docs/agents/app.md`.
- [ ] On Windows, the text input and the label names (Step 6). Prepare `...\labels\` with two or three ARW copies, one with a `.xmp` holding `xmp:Label="Rot"`. Run on it with `--index ...\index-s6a.sqlite --settings ...\step6.sqlite` (fresh, so the format is XMP): the `Rot` file shows no red label. Ctrl+, → Sidecar: under the three radios the paragraph, five fields (Red ... Purple) holding the English names, the `Lightroom language` `Select` showing English; choose `PhotoLab (.dop)`: the block hides; choose `Lightroom (.xmp)`: it shows. Click the Red field, Ctrl+A, type `Rot`, Enter: the log has `label names: ... -> ...` and a `resync: dir=…`, and the `Rot` file now shows red; close, label another file red (Ctrl+Alt+1 by default): its `.xmp` holds `xmp:Label="Rot"`. In a field check typing, Backspace / Delete, the arrows and Shift+arrows, Home / End, a mouse drag selection, Ctrl+C then Ctrl+V, and Ctrl+X; a typed `q`, `1` or arrow does nothing behind the modal; Tab moves to the next field and commits the one left; Escape commits a typed name and closes the modal. With the Japanese IME on, type a name in the Yellow field: the composition is underlined, the candidate window sits at the field, Enter commits the composition (not the field), a second Enter commits the field. Choose `日本語` in the `Lightroom language` `Select`: the fields show `レッド` ... `パープル` and the `Rot` file loses its red label after the re-listing. Quit, start again with the same `--settings` and a fresh `--index` (`index-s6b.sqlite`): Ctrl+, → Sidecar shows the last names.
- [ ] On Windows, the Import tab (Step 7). Create `...\import-root` first. Run on `...\photos` with `--index ...\index-s7a.sqlite --settings ...\step7.sqlite` (fresh, nothing seeded). Ctrl+, → Import: the paragraph, `Destination root` with an empty box, `Choose…` and the note `Choose the folder imports are copied into.`, the pattern field holding `%Y/%F`, the example `2026\2026-11-14` (today's date, relative while no root is set), the four example buttons and the eleven-row specifier table. `Choose…`: the Windows folder dialog opens; pick `...\import-root`: the root box shows it, the note is gone, and the example reads `...\import-root\2026\2026-11-14`; type into the field without committing, open `Choose…` again and pick the same folder: the typed text stays. Replace the pattern with `%Y/%m` by typing: the example changes as each character is typed and ends `...\import-root\2026\11`; press Enter. Type `%H` and press Enter: the example line shows the time-specifier refusal in red; type `%Y//%d` and press Enter: the empty-folder-name refusal; Escape closes the modal. Quit, start again with the same `--settings` and a fresh `--index` (`index-s7b.sqlite`): Ctrl+, → Import shows the chosen root and `%Y/%m` (the last valid pattern), not `%Y//%d`. Click `%F`: the field shows `%F` and the example `...\2026-11-14`; quit and restart as above: `%F` is kept. Open `Choose…` and close the modal with the `×` while the dialog is up (if the dialog lets the window be clicked), then pick a folder: it still shows on the next open and after a restart. Press Cancel in the dialog if the window cannot be clicked, and skip this last part.
- [ ] On Windows, the `Lightroom language` `Select` applies on selection (`native-label-language-apply`; plan: `docs/plans/_archived/20261116-native-label-language-apply/plan.md`; files: `crates/app/src/settingsview.rs`). Run with a fresh `--index`, `--settings` and `--log` under `D:\Photos\tests\`, format XMP. Ctrl+, → Sidecar: the `Select` shows English with the default names and there is no `Reset` button. Choose `日本語`: the five fields fill with the Japanese names at once, the strip's label names follow, and the log has `label names: ... -> ...`. Choosing the language already shown changes nothing. Edit one field and press Enter: the `Select` clears to its placeholder. Type the English names back and commit: English is selected again. Close and reopen the modal: the `Select` shows the language of the names in force. Expect no panic on any choice (the open applies nothing and a choice applies once).

### App: the settings-store tests are flaky on CI

After #1020 merged, main's CI `app` job failed in `settings::tests::reads_the_stored_values` and `settings::tests::a_stored_recommended_is_not_rewritten_by_the_load`. A stored `sidecarFormat` of `both` read back as `xmp`. Rerunning the job passed (run 43465601803). The tests come from the settings store (#1008 / #1012), not `gpui-companions`. The likely cause is the store's writer thread not having flushed the seed before the reopen. Found during `docs/plans/_archived/20261113-gpui-companions/plan.md`. Files: `crates/app/src/settings.rs`.

#### TODO

- [ ] Make the tests wait for the writer (or write the seed synchronously) so they pass repeatedly. Done when both tests pass across many repeated runs.

### App: real-device checks of the folder tree, the strip's selection and right-click menu, and Move to Trash undo / redo

`gpui-tree-trash` gave the GPUI app (`crates/app`) the left-hand folder tree (roots, expand, RAW count, watchers, the keyboard, a right-click menu with `Expand All` / `Collapse All`), the strip's multi-selection, the strip's right-click menu, `Move to Trash…` / `Delete Permanently…` with the hold during a scan, and Trash undo / redo. CI and the unit tests (`tree.rs`, `treekeys.rs`, `keys.rs`, `context.rs`, `selection.rs`, `strip.rs`, `judgment.rs`, `trash.rs`, `idle.rs`, `history.rs`, `resync.rs`, and the engine's `folders.rs`, `treewatch.rs`, `trash.rs`) cover the pure models. Only the `notify` 8.2.2 collapse / re-expand timing and the `SHParseDisplayName` rejection of the `\\?\` prefix were measured headless. Nobody has clicked the tree, opened the menus or dialog, used Explorer, the clipboard or the Recycle Bin, or seen the keyboard focus model run: every step's checkbox was ticked on the automated criteria. Windows only. Plan: `docs/plans/_archived/20261113-gpui-tree-trash/plan.md` (the full steps and their expected results are in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/tree.rs`, `crates/app/src/treekeys.rs`, `crates/app/src/context.rs`, `crates/app/src/selection.rs`, `crates/app/src/trash.rs`, `crates/app/src/idle.rs`, `crates/app/src/history.rs`, `crates/engine/src/folders.rs`, `crates/engine/src/treewatch.rs`, `crates/engine/src/trash.rs`.

Build in `crates/app` with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build` first (rename a locked `handpic.exe` aside, never kill it). Work only on copies under `D:\Photos\tests\20261113-gpui-tree-trash\` with a fresh `--index` per run and an explicit `--settings` test file; never the Tauri index or the app's default index. The tree may list and watch the roots and the ancestors of a test folder; open (click the name of, or press Enter on) only folders under `D:\Photos\tests\`. Trash and delete only the copies.

#### TODO

- [ ] On Windows, the tree pane and a launch with no folder (Step 2). Put a few ARWs in `...\tree\a` and `...\tree\b`. Launch with no folder argument: no OS dialog; the pane lists home and the volumes; the preview and strip are empty and the status line says `no folder`; the arrow, judgment, `f`, `z`, `e`, `home` / `end`, undo / redo keys, the Sort / Filter menus, a strip click and wheel do nothing and nothing panics; `q` and closing the window quit. Launch on `...\tree\a`: the chain `D:\` > `Photos` > `tests` > `20261113-gpui-tree-trash` > `tree` > `a` is expanded, `a` is highlighted and scrolled into view with its RAW count. The expander of `b` expands (count shows) and collapses. Click the name `b`: the strip, preview, meta pane and status switch to `b`, `a`'s scan stops (no further `a` progress in the log), the highlight moves; page a few files, click `a`: it resumes at its last viewed file. `ctrl+alt+arrowleft` (`toggleLeft`) hides the pane and the preview widens; again, it is back.
- [ ] On Windows, the tree follows the disk and its right-click menu (Step 3). With `...\tree` expanded, create subfolder `c` in Explorer: it appears within about a second, `tree` staying expanded; rename to `d`: the row follows; delete: the row goes with no error in the pane's notes. Collapse `tree`, create `e`: no re-list in the log; expand: `e` is listed. Right-click `b`: a menu with `Reveal in File Explorer`, `Copy Path`, `Copy Folder Name`, `Refresh`; `escape` or a press elsewhere closes it (a right press on another row opens that row's menu). Reveal opens Explorer on `tree` with `b` selected, on a root (`D:\`) it opens the drive; `Copy Path` pastes `D:\Photos\tests\20261113-gpui-tree-trash\tree\b`, `Copy Folder Name` pastes `b`. With `tree` collapsed, create `f`, right-click `tree` > `Refresh`: expanding lists `f`; `Refresh` on open folder `a` re-lists its files. Switch to another app and back: no flicker or collapse; a USB drive plugged in meanwhile appears as a new root.
- [ ] On Windows, the tree's keyboard and `Expand All` / `Collapse All` (Step 4). `b` holds nested folders (`b\20a\x`, `b\20b`, `b\c\d`). Launch on `...\tree\a`; `3` rates the current file. Click the expander of `b`: the tree takes the keyboard; Down / Up / Home / End move the outline and scroll an off-screen row into view. Space on collapsed `b` expands it and Space again collapses it (Left / Right now page the strip instead, see native-tree-left-right). Typing `20` jumps to `20a`; tapping `2` cycles the `2…` folders. With the tree focused `3`, `p`, Shift / Alt + the arrows and `Ctrl+Z` change nothing and `q` does not quit; `toggleLeft` hides the pane and hands the keys back. `escape`, a click on the preview, or the right-click rules hand the keys back / keep them as in the learnings. Enter on `b` opens it and the tree keeps the keyboard. Right-click `b`: the menu ends with `Expand All` / `Collapse All` (absent on a root); `Expand All` expands everything under `b` one listing at a time, `Collapse All` collapses under `b` with `b` staying expanded; a folder created in Explorer under an expanded one still appears.
- [ ] On Windows, the strip's multi-selection (Step 5). Ten or so ARWs in `...\select\`, Auto-advance on. Click a cell, Shift click four on: five selected (current outlined white, others filled dark gray), focus on the Shift-clicked cell. Ctrl click a middle one: it drops out, focus stays; Ctrl click the current cell: nothing. `3`: the four selected get three stars, focus does not advance, each `.xmp` says `xmp:Rating="3"`. `Ctrl+Z`: all four back, pane notes `Undid 4 files`; `Ctrl+Shift+Z`: `Redid 4 files`. `Shift+Right` twice: three selected; `Shift+Left`: two; `Left`: only the landed cell. `Ctrl+A` selects all and `p` picks every file as one undo entry. Filter to unrated, select three, press `2`: they leave the strip and the selection is the new current file alone. In a JPEG-only folder Shift click selects a range but judgment keys change nothing.
- [ ] On Windows, the strip's right-click menu (Step 6). ARWs in `...\menu\`, a few JPEGs in `...\menu-jpeg\`. Select three cells, right-click a non-current one: the menu opens at the pointer (to its left near the right edge, above it on the bottom strip), the selection stays and the focus moves to the clicked cell. It holds `Select All`, `Reveal in File Explorer`, `Copy Path` / `Copy File Name`, the flag, star and label groups with their keys and the focused file's state checked, then `Move 3 Files to Trash…` and `Delete 3 Files Permanently…`. `Copy Path` pastes three paths one per line in strip order (with `\\?\`), `Copy File Name` the three names. `2 stars` rates all three (one undo entry) and shows checked on reopening. Reveal opens Explorer on `...\menu` with the focused file selected (the prefix is stripped by `shown_path`). Right-click a cell outside the selection: it alone is selected; right-click the empty area right of the last cell: nothing opens. A press on the preview or `escape` closes the menu; a left click on a cell still pages. `...\menu-jpeg` has no flag, star or label groups. The tree's menu near the window's bottom edge opens above the pointer.
- [ ] On Windows, Move to Trash… and Delete Permanently… (Step 7). Ten or so ARWs, some with `.xmp` and `.dop`, in `...\trash\`; a few hundred in `...\trash-large\`. On `...\trash` after the scan, `Move to Trash…` on a RAW with both sidecars: dialog `Move to Trash` / `Move 1 file to the Trash?`, `tab` moves the outline between `Move to Trash` and `Cancel`, `enter` confirms; the three files are in the Recycle Bin, the strip moves to the neighbour, the pane says `Moved 1 file to the Trash`, the log has `trash: moved=1 failed=0 run_id=Some(..)`. Three selected: `Move 3 files to the Trash?`, `escape` / `Cancel` move nothing. `Delete Permanently…`: `Cancel` outlined; `tab` to the delete button, then `enter` and `space` and a double-click do nothing, one click deletes (files and sidecars gone, not in the Recycle Bin, pane `Deleted 1 file`, an earlier judgment on it no longer undoable). With the dialog open `q`, judgment keys, `ctrl+,` and clicks on the tree, strip and status menus do nothing. On `...\trash-large` with a fresh `--index`, during pass 1 pick `Move to Trash…` and confirm: the dialog closes at once, the pane says `Move to Trash: waiting for the scan to finish`, the scan ends early, `moving to Trash 0 / 1` shows briefly, the file goes, the folder re-lists and the scan resumes. Repeat but open another test folder from the tree before the scan ends: nothing is moved and the waiting note goes.
- [ ] On Windows, Trash undo and redo (Step 8). One ARW with `.xmp` and `.dop` in `...\undo\a\`, a few in `...\undo\b\`. Rate it three stars, trash it, `Ctrl+Z`: `Restored 1 file from the Trash`, the RAW and both sidecars are back (gone from the Recycle Bin), the strip shows it with three stars, current file and selection unmoved. `Ctrl+Shift+Z`: `moving to Trash 0 / 1` briefly, the files are in the Recycle Bin again, `Moved 1 file to the Trash`; `Ctrl+Z` brings them back again. Rate, trash, `Ctrl+Z` twice: file back, then the rating undone (`Undid <name>`). Trash in `a`, open `b`, `Ctrl+Z`: the file is back in `a` in Explorer, `b`'s strip unchanged, reopening `a` lists it. During a scan trash a file then `Ctrl+Z` while the re-listing's scan runs: `Undo Move to Trash: waiting for the scan to finish`, the scan is not canceled, the restore runs at its end. `Ctrl+Z` twice quickly: one `trash: undo of run` log line. `Delete Permanently…` then `Ctrl+Z`: nothing comes back. Empty the file from the Recycle Bin by hand, then `Ctrl+Z`: `Restored 0 files from the Trash, 1 failed`, the notes say `could not restore from the Trash: not in the Trash (emptied or restored by hand)`, nothing left to redo.

### App: real-device check of `open_folder` deferred by a run and refused during a rename over MCP

#### Background

`native-mcp-everything` Step 3 added the `open_folder` tool, which goes through the UI's own code (`open_from_tree` → `open`'s `deferred_open`, and `refuse_renaming`). Unit tests cover the rules. Neither state was reached end to end, because each needs the user's input. Plan: `docs/plans/_archived/20261117-native-mcp-everything/plan.md`. Files: `crates/app/src/main.rs` (`mcp_request`, `open_from_tree`), `crates/app/src/companion.rs`.

#### TODO

- [ ] On Windows, run the app with MCP on a test port as `docs/agents/app.md` prescribes, with a folder of test copies open. (1) Start a Move to Trash of a few files and, while its run is in flight, call `open_folder` on another folder: expect `opening` set to it and `idle: false` in the reply, then `wait_for_idle` returning with that folder open. (2) Rename a folder in the tree and, while the rename is in flight (a slow share helps), call `open_folder` on it: expect `A folder is being renamed; try again in a moment.`

### App: real-device check of the MCP busy guard's wiring (settings, trash dialog, folder dialog, inline rename)

#### Background

`native-mcp-everything` Step 1 made every MCP request but the reads refuse while the settings modal, the trash dialog or a folder dialog is open or an inline rename is under way. The refusal rules are unit-tested (`a_busy_view_refuses_every_request_but_get_view` in `crates/app/src/companion.rs`). The `busy` value that `mcp_request` builds was never exercised end to end, since opening those states needs the user's input. Plan: `docs/plans/_archived/20261117-native-mcp-everything/plan.md`. Files: `crates/app/src/main.rs` (`mcp_request`, `key_down`), `crates/app/src/companion.rs`.

#### TODO

- [ ] On Windows (or any platform), run the app with MCP on a test port as `docs/agents/app.md` prescribes. Open the settings (`ctrl+,`) and call `navigate {"to":"next"}`: expect `Handpic's settings are open; try again once the user closes them`, while `get_view` still answers. Repeat with the strip's `Move to Trash…` dialog open, a folder menu dialog (e.g. `Delete Sidecars…`) open, and a folder's inline rename open, each expecting its own message.

### App: real-device checks of the MCP settings page and a live theme change over MCP

#### Background

`native-mcp-everything` Step 4 added the MCP page with the `mcpAllowDestructive` switch to the settings modal and the `get_settings` / `set_settings` tools. The switch's rule and store key are unit-tested (`settings.rs`, `settingsmodal.rs`'s tab order) and the `set_settings` writes were checked end to end. The page itself and the repaint of a live theme change were never seen on screen. Plan: `docs/plans/_archived/20261117-native-mcp-everything/plan.md`. Files: `crates/app/src/settingsview.rs`, `crates/app/src/settingsmodal.rs`, `crates/app/src/settings.rs`, `crates/app/src/main.rs`.

#### TODO

- [ ] On Windows, run the app with MCP on a test port as `docs/agents/app.md` prescribes. (1) Open the settings (`ctrl+,`), pick the MCP page between Import and Appearance, flip "Let MCP clients run destructive operations without asking" on, close, and call `get_settings`: expect `mcp_allow_destructive: true` and `mcpAllowDestructive` `true` in the store; reopen the page: the switch shows on. (2) Open the settings and call `set_settings {"auto_advance":false}`: expect `Handpic's settings are open; try again once the user closes them`, while `get_settings` still answers.

### App: real-device check of Move to Trash over MCP with `mcpAllowDestructive` off, and the redo refusal

#### Background

`native-mcp-everything` Step 5 added `trash_photos`, `undo` and `redo` for trash runs over MCP. The request rules are unit-tested in `crates/app/src/companion.rs`, and the dialog path was seen end to end for a delete only. A click on the Move to Trash dialog, the undo and the redo refusal were not exercised. Plan: `docs/plans/_archived/20261117-native-mcp-everything/plan.md`. Files: `crates/app/src/companion.rs`, `crates/app/src/main.rs` (`confirm_trash`, `run_trash`, `step_trash`), `crates/app/src/trash.rs`.

#### TODO

- [ ] On Windows, run the app with MCP on a test port as `docs/agents/app.md` prescribes, with `mcpAllowDestructive` off and a folder of test copies open. (1) Call `trash_photos` on one photo: expect `awaiting_confirmation` and the Move to Trash dialog with its run button focused; click Move to Trash: `wait_for_idle` returns `finished` "Moved 1 file to the Trash". (2) Call `undo`: the file comes back; call `redo`: expect the refusal starting "the next redo moves files to the Trash again without a dialog". (3) Open the delete dialog over MCP, click Delete Permanently once: the file is deleted and `get_status` reports `Delete Permanently` `finished`; open it again and press Cancel: `wait_for_idle` returns idle with nothing deleted.

### App: real-device check of a folder operation confirmed in the app's dialog after an MCP call (Rewrite Sidecars, Delete Sidecars, Sequence)

#### Background

`native-mcp-everything` Step 6 added `rewrite_sidecars`, `delete_sidecars`, `sequence_timestamps` and `cancel_operation`. The dialog path was opened and canceled over MCP end to end, but a click on a dialog's run button needs the user. Plan: `docs/plans/_archived/20261117-native-mcp-everything/plan.md`. Files: `crates/app/src/folderops.rs`, `crates/app/src/sidecars.rs`, `crates/app/src/sequence.rs`, `crates/app/src/companion.rs`.

#### TODO

- [ ] On Windows, run the app with MCP on a test port as `docs/agents/app.md` prescribes, `mcpAllowDestructive` off, with a folder of test copies holding judged photos and their `.xmp`. Call `delete_sidecars {"path": <folder>}`, wait for the Delete Sidecars dialog, click Move to Trash: expect `wait_for_idle` to return `finished` "Moved N sidecars to the Trash" and the `.xmp` files gone. Repeat with `rewrite_sidecars` (click Rewrite: `finished` "Rewrote the sidecars of N files in <folder>") and `sequence_timestamps` (click Run: the `-sequenced` folder written and revealed).

### App: real-device check of Import, Rename by Capture Date and the folder rename confirmed in the app after an MCP call

#### Background

`native-mcp-everything` Step 7 added `import_folder`, `rename_by_capture_date` and `rename_folder`. The grant-off paths were opened and canceled over MCP end to end (`get_status` reported each dialog's content and the rename field awaiting). The dialogs, the prefilled rename field and a confirm need the user. Plan: `docs/plans/_archived/20261117-native-mcp-everything/plan.md`. Files: `crates/app/src/folderops.rs`, `crates/app/src/import.rs`, `crates/app/src/datefolder.rs`, `crates/app/src/rename.rs`, `crates/app/src/companion.rs`.

#### TODO

- [ ] On Windows, run the app with MCP on a test port as `docs/agents/app.md` prescribes, `mcpAllowDestructive` off, `importRoot` set to a scratch folder, the test folder outside any hidden folder. (1) `import_folder` on a folder of test copies: the Import dialog shows; click Run: `wait_for_idle` returns `finished` "Imported N files ...". (2) `rename_by_capture_date` on a folder of one date: click Run, the folder becomes `YYYY-MM-DD`. (3) `rename_folder {"path": <a drawn folder>, "name": "x"}`: the tree's row turns into a field holding `x`, selected; press Enter: `wait_for_idle` returns `Rename…` `finished` and the row (and the open folder, when it was that one) follows; repeat and press Escape: nothing renamed, `idle` true.
- [ ] On Windows, with `mcpAllowDestructive` on, confirm an `import_folder` preview and, while it runs, right-click the source folder's `Import…`: the dialog comes back running with Cancel enabled.

### App: real-device check of the Clear Cache dialog opened over MCP, and the grayscale latch on screen

#### Background

`native-mcp-everything` Step 8 added `clear_cache` (opening the Clear Cache dialog) and `set_overlays`' `grayscale`. Both were checked over MCP end to end (`get_status` reported the dialog awaiting, the busy refusals held, and `get_view` the grayscale), but no window capture was taken. Plan: `docs/plans/_archived/20261117-native-mcp-everything/plan.md`. Files: `crates/app/src/companion.rs`, `crates/app/src/cache.rs`, `crates/app/src/grayscale.rs`, `crates/app/src/main.rs`.

#### TODO

- [ ] On Windows, run the app with MCP on a test port as `docs/agents/app.md` prescribes, `mcpAllowDestructive` off, a folder of test copies open, the settings closed. (1) Call `clear_cache`: expect the "Clear the index cache?" dialog over the dimmed main window with Clear focused; click Clear: `wait_for_idle` returns `Clear Cache` `finished` "Cleared the index cache" and the folder is scanned again; call it again and press Escape: idle, nothing cleared, and the main window takes keys again. (2) Call `set_overlays {"grayscale": true}`: the photo turns gray and stays gray after clicking the tree and switching to another window and back; press and release `g`: it turns back to color.

### Agents: `local-review-runner` returns before its reviewer finishes, and `pr-runner` aborts on a transient `api.github.com` outage

#### Background

Seen by the orchestrating session of `native-mcp-everything` (plan: `docs/plans/_archived/20261117-native-mcp-everything/plan.md`). (1) In Step 5, `local-review-runner` returned before its reviewer finished: the Agent tool launched the reviewer in the background despite a foreground request. (2) In Step 7, `pr-runner` aborted with `pr_status_failure` on a transient `api.github.com` outage. Files: `.claude/agents/local-review-runner.md`, `.claude/agents/pr-runner.md`, `.claude/skills/pr/scripts`.

#### TODO

- [ ] Make `local-review-runner` wait for the reviewer's result or detect a background launch. Done when a run never returns before the reviewer has.
- [ ] Make `pr-runner` retry network errors for longer before counting a `pr_status_failure`. Done when a short `api.github.com` outage does not abort a run.

### App: the coalescing `last_viewed` writer can lose folder A's position when B is opened and paged first

The coalescing `last_viewed` writer (`viewed::Latest`) keeps one pending value across folders. A position pushed in folder A while a write is out is replaced by B's first push if B is opened and paged before that write settles, so A reopens one file earlier. The Tauri `lastViewedWriter` had the same shape. Found in `gpui-tree-trash` Step 2's second-open review of `opened`. Plan: `docs/plans/_archived/20261113-gpui-tree-trash/plan.md`. Files: `crates/app/src/viewed.rs`.

#### TODO

- [ ] Decide whether to key the pending value by folder (or flush on a folder switch), and fix the writer if so.

### App: a second open started before the first listing joins the launch eviction can scan before the eviction runs

The launch's eviction (`eviction`, joined by the first listing only) is not awaited by a second open started before the first listing joined it, so that listing's `Scan::prepare` may run before the eviction. It is only reachable by opening two folders within the eviction's run at launch. Found in `gpui-tree-trash` Step 2's concurrent-open guard. Plan: `docs/plans/_archived/20261113-gpui-tree-trash/plan.md`. File: `crates/app/src/main.rs` (`list`).

#### TODO

- [ ] Decide whether every listing should wait for the eviction (or the eviction should be a shared future), or accept the race as unreachable in practice.

### App: real-device check of the prefetch ring on a page turn, a folder switch and a filter change

`gpui-prefetch` added a ring of decoded neighbour previews to the GPUI app (`crates/app`). CI, the ring's unit tests and the `--auto 60` A/B on the ARW and RAF copies cover the logic and the page-turn timing (57 of 60 presses hit, RAF p50 `keypressToPixels` about 66 to 14 ms). Nobody has pressed the keys by hand: the Step 1 checkbox was ticked on the automated criteria only, so a folder switch and a filter change were never watched for another folder's or a hidden file's image appearing. Plan: `docs/plans/_archived/20261114-gpui-prefetch/plan.md`. Files: `crates/app/src/main.rs` (`take_held`, `clear_ring`, the `retain` in `refilter`), `crates/app/src/prefetch.rs`.

#### TODO

- [ ] On Windows, with the release build (`mise run app:release`, NASM on `PATH`) on a folder under `D:\Photos\tests\`, wait for the scan to end, then press `→` onto the next file. Expect it to show at once, and its `page` line to say `source=prefetch`.
- [ ] On Windows, in the same build, open another folder from the tree. Expect no image of the previous folder to show at any point.
- [ ] On Windows, in the same build, change the filter so the current file's neighbour is hidden, then press `→`. Expect a hidden file's image never to show.

### App: kept prefetch-ring entries keep their GPU texture

#### Background

`gpui-prefetch`'s memory measurement on the RAF folder showed about +110 MB working set and about +300 MB private bytes with the ring on, against the cap's 128 MB of BGRA (two 4416x2944 neighbours are 104 MB). An image that was painted and then handed back through `Ring::keep` holds its GPU texture until it is retired, because only a retired image gets `cx.drop_image`. Releasing the texture on `keep` and uploading it again on the next hit would trade memory for `paint` on a hit (which did not grow in the A/B). Plan: `docs/plans/_archived/20261114-gpui-prefetch/plan.md`. Files: `crates/app/src/main.rs` (`present`), `crates/app/src/prefetch.rs`.

#### TODO

- [ ] Decide whether to release a kept entry's texture on `keep`. Measure private bytes and `paint` on a hit on the RAF copy with and without the release.

### App: the prefetch ring never covers the wrap from the last file to the first

#### Background

`prefetch::neighbours` stops at the ends of the list, a faithful port of `prefetch.ts`, but the app's `Pager` wraps on `Next` / `Previous`. The press from the last file to the first therefore pays a full read. In `gpui-prefetch`'s `--auto 60` runs on 20 files, those 3 misses in 60 presses set the p95 at a read's latency (RAF 56 ms against a hit's 14 ms p50). Plan: `docs/plans/_archived/20261114-gpui-prefetch/plan.md`. Files: `crates/app/src/prefetch.rs`, `crates/app/src/pager.rs`.

#### TODO

- [ ] Decide whether `neighbours` should wrap when the pager does. If so, make it wrap with unit tests, and re-run `--auto 60` to check that the p95 falls.

### App: `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped` is flaky on a loaded machine

#### Background

The test failed once in four `cargo test` runs of the app workspace during `gpui-prefetch`, with no change near `scan.rs`, and passed on the reruns. It is probably a timing-bound `stop_within` on a loaded machine. Plan: `docs/plans/_archived/20261114-gpui-prefetch/plan.md`. Files: `crates/app/src/scan.rs`.

#### TODO

- [ ] Find what the test's timing bound depends on, and make the test independent of machine load (or widen the bound).

### App: real-device check of the performance overlay on `d`

`gpui-perf-overlay` ported the Tauri app's performance overlay to the GPUI app: the keymap action `performance` (default key `d`) toggles a 120 px band along the bottom of the preview area that plots the last 60 s of Handpic's and the system's CPU, Handpic's read throughput, the system's disk busy time and one stacked bar per preview switch, with a one-line legend. CI and the unit tests cover the pure model (`crates/app/src/perf.rs`) and the graph layout (`crates/app/src/perfgraph.rs`), and `mise run app:check` passes. The agent could not see the window, so the drawing, the 500 ms motion and the bars were never exercised; the step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261114-gpui-perf-overlay/plan.md`.

Files: `crates/app/src/perf.rs`, `crates/app/src/perfgraph.rs`, `crates/app/src/main.rs`, `crates/app/src/scan.rs` (`Event::Perf`).

#### TODO

- [ ] On Windows, in a release build of `crates/app`, open a folder with `mise run app:dev -- <folder>` and press `d`. Expected: a 120 px band along the bottom of the preview area, `#171717` at 0.85 alpha, with grid lines at 0, 50 and 100 %. The `Handpic CPU` / `System CPU` (blue, solid 2 px / dashed 1 px) and `Handpic disk` / `System disk` (green) lines move every 500 ms with the newest point at the right edge. The one-line legend at 11 px shows each line's swatch, `Latency` and the eight `page` stage names fading, with `<n> MB/s  <n> ms` at the right.
- [ ] On Windows, with the overlay on, switch previews with an arrow key, with a strip click and by opening a folder from the tree. Expected: one red stacked bar per preview switch, and the key or click works as before.
- [ ] On Windows, press `d` again. Expected: the band disappears, and on reopening it starts empty.
- [ ] On Windows, with the overlay off, hold `→` through a folder. Expected: the `page` lines and the `summary` table look as before the change.

### App: real-device checks of the restored window and last folder (gpui-session-restore)

#### Background

`gpui-session-restore` made the GPUI app (`crates/app`) reopen the last folder and the window's position, size, display and maximized state on a launch without arguments (`session.rs`, the `lastFolder` and `windowBounds` keys of the settings store, written on open and in the quit handler). Step 1 was ticked on its automated criteria (the `session.rs` unit tests, `mise run ci`, `mise run app:check`) and a hand check on one Windows 11 display at 96 DPI: a moved and resized window, a maximized quit, a minimized quit, bounds off every display, an unknown display UUID, a relaunch without arguments, a renamed remembered folder, and `--auto 1 <other folder>`. Never exercised: quitting with `q`, a window snapped to a screen half, a second monitor (especially one with a different scale factor), a taskbar docked at the top or left, paging to a file before the quit, and a visual check of `--auto`'s centered default. macOS and Linux are covered by the CI build and the unit tests only. Run each check with `--settings <fresh test store>`, `--index <test index>` and `--log <test file>`, never the real store. Record the results as `(Measured)` in `docs/agents/app.md` ("A launch restores the last folder and the window") and remove that section's "Not seen yet" bullet. Plan: `docs/plans/_archived/20261114-gpui-session-restore/plan.md`. Files: `crates/app/src/session.rs`, `crates/app/src/main.rs`, `crates/app/src/settings.rs`.

#### TODO

- [ ] On Windows, move and resize the window, quit with `q`, relaunch. Expected: the same place and size.
- [ ] On Windows, snap the window to a screen half, quit, relaunch. Expected: it opens on-screen at the restore bounds.
- [ ] On Windows with a second monitor, ideally with a different scale factor, move the window there, quit, relaunch, both windowed and maximized. Expected: the window comes back on that monitor, windowed and maximized respectively.
- [ ] On Windows with the taskbar docked at the top or left, move the window, quit, relaunch. Record whether the position drifts by the taskbar's size (`rcNormalPosition` is in workspace coordinates). Fix only if it drifts.
- [ ] On Windows, open a folder from the tree, page a few files, quit, relaunch without arguments. Expected: that folder is revealed in the tree at that file. Relaunch with another folder as the argument. Expected: the argument's folder opens.
- [ ] On Windows, run `--auto 3 <folder>`. Expected: the window visibly opens centered at 1600x1000 whatever is stored.

### App: real-device checks of the pane toggles and Open Folder (gpui-pane-toggles)

`gpui-pane-toggles` made `open`, `toggleRight`, `toggleStrip` and `toggleSides` run in the GPUI app (`crates/app`) and remembered the panes under the `panels` key of the settings store, `toggleLeft` included. Unit tests cover the pure rules (`panels.rs`, `settings.rs`'s load of `panels`), and `mise run ci` and `mise run app:check` pass. A launch with `--auto` on a store holding `{"panels":{"left":false}}` quit cleanly and left the store unchanged. Nobody has run the picker or the toggles in the window, or seen what the window drew: the picker, the canvas resize, the 1:1 re-cut, the hidden strip's bookkeeping, the keyboard hand-back, the persistence across a quit and a relaunch, and a rebound key are all unchecked. Plan: `docs/plans/_archived/20261114-gpui-pane-toggles/plan.md`. Files: `crates/app/src/main.rs` (`pick_folder`, `change_panels`, `render`), `crates/app/src/panels.rs`, `crates/app/src/settings.rs`.

Run each check with `--settings <fresh test store>` and `--log <test file>`, never the real store. When done, record the results and update `docs/agents/app.md`'s "Not verified" note on the picker.

#### TODO

- [ ] On Windows, press `ctrl+o` and pick a folder: it opens and the tree reveals it. Press it again and cancel: nothing changes. Press it again with the tree holding the keyboard. Also note whether a second `ctrl+o` while the picker is up opens a second dialog (if so, a flag on the view that ignores the press is needed) and whether the culling keys work right after the picker closes without a click.
- [ ] On Windows, give the tree the keyboard and hide the left pane with `tab`. Expect the arrow keys to page the photos again.
- [ ] On Windows, put `{"panels": {"left": false}}` in a Tauri-style `settings.json` and launch with `--import-settings` on a new store: the tree is hidden and the other two panes are shown.
- [ ] On Windows, rebind one of the four actions in the Keyboard Shortcuts tab: the new key runs it.

### Agents: a cold Rust `ci:local` is moved to the background by the Bash tool after 180 s

#### Background

During Step 1 the Bash tool moved a foreground command to the background after
about 180 s even with `timeout: 600000`, and the cold Rust `ci:local` runs hit
this. `.claude/agents/implementer.md` (step 7) and
`.claude/agents/local-review-addresser.md` tell the agent to run the check in
the foreground so the exit code is observable, and say nothing about the run
being backgrounded anyway.
Plan: `docs/plans/_archived/20261114-light-local-ci/plan.md`
`native-mcp-everything` saw the same with the PR wait scripts: the harness
moved every foreground wait script to the background after about 180 s, so
each runner had to `TaskStop` and re-run repeatedly (plan:
`docs/plans/_archived/20261117-native-mcp-everything/plan.md`).

#### TODO

- [ ] Reproduce the behavior once with a cold `ci:local`. If it holds, add to
      both agent definitions that an exit code arriving as a task notification
      is acceptable and is waited for, not retried, and that the rule is only
      about not choosing `run_in_background` yourself. Done when both
      definitions say what to do when the Bash tool backgrounds the run.
- [ ] Give the PR wait scripts an internal timeout under 180 s that returns a
      "still waiting" status, or document the re-run loop in
      `.claude/agents/pr-runner.md`. Done when a PR run needs no manual
      re-dispatch or `TaskStop`.

### App: real-device checks of the empty state in the preview area

#### Background

`gpui-empty-state` ported the Tauri app's empty state to the GPUI app (`crates/app`): the preview area now shows a centered muted message for no folder (the click opens the folder picker), no RAW or JPEG files, a filter hiding every file, and a failed preview of the current file. CI and the unit tests cover the pure module (`empty.rs`: the state judgment and the open hint, ported from `empty.test.ts`). The message layer, the click and the canvas wrapper were never exercised, because the implementing session could not run the app's window. Plan: `docs/plans/_archived/20261114-gpui-empty-state/plan.md`. Files: `crates/app/src/empty.rs`, `crates/app/src/main.rs` (`render`, `empty_message`).

#### TODO

- [ ] On Windows, launch the app without a folder (`mise run app:dev -- --settings <fresh file>` with no `lastFolder`). Expect "Drop a folder onto the window, or click here to choose one. Or press ctrl+o." centered in the preview area in muted text; a left click on it opens the OS folder picker; rebinding `open` in the Keyboard Shortcuts tab changes the key clause at once (`space` shown as `Space`), and unbinding it removes the clause; a click on the settings modal drawn over it does not open the picker.
- [ ] On Windows, open a folder without RAW or JPEG files. Expect a click on the "This folder has no RAW or JPEG files." message to do nothing.
- [ ] On Windows, page from a normal NEF onto `D:\Photos\samples\NEF\NIKON_D70_Nikon.nef`. Expect the previous image to leave the screen and "The preview of this file could not be shown." to show; paging on removes it, paging back shows it again, and `z` on that file hides it.
- [ ] On Windows, with a folder open, confirm the preview still fits the area and the 1:1 view (`z`) still centers on the focus point, because the canvas's bounds did not change with the new wrapper div.

### App: real-device checks of the status row's StatusBar and progress circle

`native-status-bar` rendered the GPUI app's status row (`crates/app`) through gpui-component's `StatusBar` and replaced the 2 px top-edge progress bar (`gpui-progress-bars`) with a 12 px `ProgressCircle` in the bar's right region, left of `Sort ▾`. CI, the `progress.rs` and `theme.rs` unit tests, a measured preview canvas (1160 x 842 in the default 1600 x 1000 window) and screenshots of the row, a forced-shown circle and a long status text cover the build. The menus, a live scan (the circle's fill and the pass 1 to pass 2 restart), the Move to Trash run and paging cost during a scan were never exercised by hand. Plan: `docs/plans/_archived/20261115-native-status-bar/plan.md` (and `docs/plans/_archived/20261114-gpui-progress-bars/plan.md` for the removed bar).

Files: `crates/app/src/main.rs` (`status_row`, `progress_circle`), `crates/app/src/progress.rs`, `crates/app/src/theme.rs`.

#### TODO

- [ ] On Windows, launch the app and open a folder. Expect the row to show as a bar with a top border, its fill and 12 px muted text. Open and dismiss both `Sort ▾` and `Filter ▾`, including a tall `Filter ▾` menu: each opens upward with its right edge on the button's, is not clipped by the regions' `overflow_hidden`, fits above the row under the 40 px cap, and is dismissed by a press outside.
- [ ] On Windows, open a cold folder of 500+ RAW files (fresh `--index`). Expect the circle to fill through pass 1, start again at empty for pass 2 and disappear at the end, with the status text and the buttons not moving. Judge whether the full-to-empty drop (animated over 180 ms) reads well, and confirm paging during the scan is not visibly slower from the circle's animation frames. The status line's `scanning d/t` / `analyzing d/t` must read as before; record what is seen in `docs/agents/app.md`.
- [ ] On Windows, select many files and run `Move to Trash…`, also with the right pane hidden (`toggleRight`) and again with the strip and the left pane hidden. Expect the circle to show the run's progress and go when it ends, and the preview canvas not to resize when it appears or disappears.
- [ ] On Windows, confirm a long status text does not push the circle or the buttons out and that an error text is still readable in the muted color.
- [ ] On macOS and Linux, repeat the menu, scan and Move to Trash checks above when at hand.

### App: Sequence JPEG Timestamps and Import do not feed the progress bar

`gpui-progress-bars` made the GPUI app's progress indicator take a folder operation's `(done, total)` through `progress::shown`'s `folder_op` argument, but `main.rs`'s `progress_circle` passes `None`, so Sequence and Import still show only their pane note. Both already have `(done, total)` callbacks. The folder-dialogs plan (`docs/plans/20261113-gpui-folder-dialogs/plan.md`) wires the dialogs themselves. Plan: `docs/plans/_archived/20261114-gpui-progress-bars/plan.md`.

Files: `crates/app/src/progress.rs`, `crates/app/src/main.rs`, `crates/app/src/sequence.rs`, `crates/app/src/import.rs`.

#### TODO

- [ ] Pass each run's `(done, total)` into `progress::shown`'s `folder_op` input, and clear it where the run ends, is canceled or is stopped by a re-listing.

### App: the trash undo, Rewrite Sidecars and Delete Sidecars runs have no progress callback

`gpui-progress-bars` feeds the bar from the scan and from Move to Trash / Delete Permanently / trash redo. `trash::undo_run` takes no progress callback, and neither do the Rewrite Sidecars from Index and Delete Sidecars runs, so the bar shows nothing for them (`redo_run` has one and already feeds the bar). Plan: `docs/plans/_archived/20261114-gpui-progress-bars/plan.md`.

Files: `crates/engine/src/trash.rs`, `crates/engine/src/foldersidecars.rs`, `crates/app/src/progress.rs`.

#### TODO

- [ ] Add a `(done, total)` callback to `trash::undo_run` and to the `foldersidecars` rewrite and delete runs, and feed the bar from them in `crates/app`.

### App: real-device checks of the burst bands, badges, burst keys and reject the rest

#### Background

`gpui-bursts` ported the Tauri app's burst grouping to the GPUI app (`crates/app`): `burst.rs` groups the listing (`BURST_GAP_MS` 1000 ms), the strip paints a band behind each burst of two or more cells with a `position/size` badge, `burstPrevious` / `burstNext` / `burstFramePrevious` / `burstFrameNext` move through them, `rejectRest` rejects the rest of the current file's burst as one undo entry, and the MCP companion's `get_view` reports the current file's `burst`. CI and the unit tests cover the pure parts (`burst.rs` with every `burst.test.ts` case, the `reject_rest` change builder, `companion.rs`'s burst cases). The band's and the badge's look, the keys on a real keyboard, and `rejectRest`'s sidecars and undo on real files were never exercised, because the implementing session could not run the app's window. Plan: `docs/plans/_archived/20261114-gpui-bursts/plan.md`. Files: `crates/app/src/main.rs` (`paint_strip`, `move_burst`, `reject_rest`, `mcp_request`), `crates/app/src/burst.rs`, `crates/app/src/companion.rs`.

#### TODO

- [ ] On macOS, open a Sony burst folder (sub-second subsec) and a Leica whole-second burst folder in `handpic`. Expect each burst of two or more to show a faint band behind its cells, rounded at its first and last cell, joined across the gaps, distinguishable from the current cell's outline, the selected cell's fill and a failed cell; every member shows `n/size` at the image box's bottom-right, dimmed on a rejected member; `arrowup` / `arrowdown` jump between bursts and `alt+arrowleft` / `alt+arrowright` step within one, also under a filter and a non-capture sort and in a JPEG (view-only) folder, and do nothing while the folder tree has the keyboard.
- [ ] On macOS, in `handpic` on test copies of a burst folder, put the current file in a burst of three or more (one member already rejected, one hidden by a filter) and press `shift+x`. Expect every other member not yet rejected to turn rejected (its `.xmp` / `.dop` gets the reject flag with the stars and label kept, the hidden one too), the current file to stay current, and nothing to advance; `Ctrl+Z` (`Cmd+Z` on macOS) restores them with `Undid N files` and leaves the current file, `Ctrl+Shift+Z` (`Cmd+Shift+Z`) redoes with `Redid N files`; on a two-frame burst the undo moves to the other frame. Nothing happens in a JPEG folder or on a singleton. With MCP on, `get_view`'s `burst` lists the members with their sharpness and judgments, `visible: false` on the hidden one.

### App: real-device check of the folder drop (gpui-folder-drop)

#### Background

`gpui-folder-drop` ported the Tauri window drop to the GPUI app (`crates/app`): a dropped folder opens through `open(path, None, None, cx)`, a dropped file opens its parent, and a drop of none or several paths shows `Drop a single folder or RAW file.` on the status line. While an OS file drag is over the window, a 3px `--ring` outline is drawn. CI and the unit tests of `dropped.rs` cover which single path a drop names and which folder it resolves to, and `mise run ci:local` passed. Nobody has dropped anything onto the running window: the drop, the outline drawing and clearing, and the swallowing of a drop by an occluded dialog are unverified. Step 1's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261114-gpui-folder-drop/plan.md` (the pending check is also in `learnings.md` in the same folder). Files: `crates/app/src/main.rs` (`drop_paths`, `drop_outline`), `crates/app/src/dropped.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows, with `mise run app:dev` running, drag a folder from Explorer over the window. Expect a 3px `#737373` outline on the main area (tree, preview, meta pane), also while the pointer is over the strip or the status row. Drop it: the folder opens and is revealed in the tree, and the outline goes.
- [ ] On Windows, drop one file (a RAW, then a non-RAW such as a `.txt`). Expect its parent folder to open and the file not to be selected.
- [ ] On Windows, drop two folders at once. Expect nothing to open and the status line to read `Drop a single folder or RAW file.`
- [ ] On Windows, open the settings modal (`ctrl+,`) and drop a folder over it. Expect nothing to open (the occluded layer swallows the drop).
- [ ] On Windows, drag a folder into the window and back out without dropping. Expect the outline to clear.
- [ ] Record the results in `learnings.md` and turn the "Not verified in the running app" bullet of "A folder dropped onto the window opens, a later drop winning" in `docs/agents/app.md` into a Measured one.

### App: make the settings writer close its store on drop (flaky seed reload test)

#### Background

`settings::tests::a_second_load_does_not_reread_a_changed_seed` in `crates/app/src/settings.rs` failed on the Windows CI job of PR #1052 (unrelated to that PR's change): the test reopened the settings store while the previous `Writes` still held it. The CI fix only added a retry in the test, with no production change. The cleaner fix is for `Writes` to close its store when dropped, so the test needs no retry.

#### TODO

- [ ] Make `Writes` in `crates/app/src/settings.rs` close its store on drop, and remove the retry from `a_second_load_does_not_reread_a_changed_seed`. Done when the test passes on Windows without the retry.

### App: real-device checks of the filter menu's icons and headings (gpui-filter-menu-icons)

`gpui-filter-menu-icons` gave the GPUI app's strip `Filter ▾` menu (`crates/app`) a shape per kind of row, drawn with Lucide icons from a new asset source, and a non-clickable heading over every section. The rating rows are now ordered 5 stars down to 0 and drawn as five star icons. The color labels are squares, the `Face focus` rows use `scan-face` and its frown derivative, and Portrait / Landscape use `rectangle-vertical` / `rectangle-horizontal`. New headings are `Flag`, `Rating`, `Color label` and `Orientation`. Unit tests cover the row model (order, headings, leads, checked items) and the asset source (every icon path loads SVG bytes, an unknown path loads `None`), and `mise run ci:local` passes. The window was never run, so how the icons and headings look and behave is unchecked: no agent can see the window, and Step 1's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261114-gpui-filter-menu-icons/plan.md`.

Files: `crates/app/src/main.rs` (`menu_row`), `crates/app/src/menu.rs`, `crates/app/src/icons.rs`.

#### TODO

- [ ] On Windows, build with `cargo build` in `crates/app`, launch on a test copy with an explicit fresh `--index` and a test `--settings`, and open `Filter ▾`. Expect a heading over every section (`Flag`, `Rating`, `Color label`, `Orientation`, `Face focus`, `Eyes`, `Output` when the folder has an export answer, the EXIF groups), none of them reacting to hover or a click.
- [ ] On Windows, in the same menu, expect the flag dots unchanged; star rows 5 down to 0 with crisp star icons, lit and dim; seven filled squares and one outline square; the smiling and frowning `scan-face` in green and gray with `Neutral` aligned; the two rectangles for the orientations.
- [ ] On Windows, expect the text of every row in a section to start at the same x, a click to still toggle each item, and the taller menu to scroll in a small window.
- [ ] On a HiDPI or fractional-scale display, if available, check that the 1 px stroke icons read cleanly at 12 px. If a shape reads badly, pick a size per kind (`MENU_ICON` in `main.rs`).

### App: real-device checks of Rewrite Sidecars from Index… and Delete Sidecars… and the folder menu (gpui-folder-dialogs Step 3)

#### Background

`gpui-folder-dialogs` Step 3 ported the folder menu's part-2 items, the shared confirmation dialog (`dialog.rs`), `folderops.rs` and the two sidecar operations to the GPUI app. CI and the unit tests of `dialog.rs`, `sidecars.rs`, `context.rs`, `trash.rs`, `history.rs` and `idle.rs` cover the models, and `mise run ci` and `mise run app:check` passed. Nobody has driven the dialog, its keys, the waiting note during a scan or the runs on disk in the running app. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261113-gpui-folder-dialogs/plan.md` (the full numbered steps are in the `learnings.md` of the same folder). Files: `crates/app/src/dialog.rs`, `crates/app/src/folderops.rs`, `crates/app/src/sidecars.rs`, `crates/app/src/context.rs`, `crates/app/src/trash.rs`, `crates/app/src/idle.rs`.

#### TODO

- [ ] On Windows, copy a dozen ARWs (a few with a `.xmp` and a `.dop`) into `D:\Photos\tests\20261113-gpui-folder-dialogs\sidecars\` and two JPEGs alone into `...\jpeg-only\`. `cargo build` in `crates/app` (rename a locked `handpic.exe` aside, never kill it), delete `index-step3.sqlite`, and run `crates\app\target\debug\handpic.exe D:\Photos\tests\20261113-gpui-folder-dialogs\sidecars --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step3.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step3.sqlite --log D:\Photos\tests\20261113-gpui-folder-dialogs\step3.log`. Only these copies.
- [ ] On Windows, right-click `sidecars` in the tree. Expect `Rename…` and `Rename by Capture Date…`, `Expand All` / `Collapse All`, `Rewrite Sidecars from Index…` / `Delete Sidecars…`, `Sequence JPEG Timestamps…` and `Import…` in that order. On a root (home or a drive) the rename and expand groups are gone.
- [ ] On Windows, pick `Rewrite Sidecars from Index…` right after launch, while pass 1 runs, then the same with `Delete Sidecars…`. Expect the pane to show `Rewrite Sidecars from Index: waiting for the scan to finish` with the scan not canceled, and the dialog to open once the scan ends.
- [ ] On Windows, judge a few files, then `Rewrite Sidecars from Index…`. Expect `n files with a judgment`, `m files without one (any existing sidecar is cleared)` and `Rewrite the XMP sidecars of k files in sidecars?`, with `Rewrite` outlined, Tab moving to `Cancel` and back, and Escape closing it. With `Rewrite`: the buttons dim, the running note shows, the dialog closes and the pane says `Rewrote the sidecars of k files in sidecars`; the `.xmp` files carry the judgments.
- [ ] On Windows, `Delete Sidecars…`. Expect per-kind counts and sizes and `Move n sidecars (…) of sidecars to the Trash?` with a red `Move to Trash`. Confirm: the sidecars are in the Recycle Bin, the strip's marks are gone, the pane says `Moved n sidecars to the Trash`. `Ctrl+Z` says `Restored n sidecars from the Trash`, the sidecars and marks return, and `Ctrl+Shift+Z` moves them again.
- [ ] On Windows, either item on `jpeg-only`. Expect `jpeg-only: rewriting sidecars applies to RAW files only` (or `deleting …`) and no dialog.
- [ ] On Windows, with a dialog open, a click on the tree or the strip does nothing and no key reaches the strip. With a request waiting during a scan, opening another folder drops the waiting note and opens no dialog. Pressing `Ctrl+,` while the preview is under way does not open the settings modal.
- [ ] Record the results in the archived `learnings.md`.

### App: real-device checks of Sequence JPEG Timestamps… (gpui-folder-dialogs Step 4)

#### Background

`gpui-folder-dialogs` Step 4 ported Sequence JPEG Timestamps… to the GPUI app (`sequence.rs`, a run on its own thread with a cancel flag and progress through the event channel). The unit tests of `sequence.rs`, `dialog.rs` and `context.rs` cover the model; `cargo clippy` and `cargo test` in `crates/app` passed locally and the full CI passed on GitHub Checks (the local `mise run ci` was skipped for memory). Nobody has seen the dialog, the run's progress, the cancel or the reveal in Explorer in the running app. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261113-gpui-folder-dialogs/plan.md` (the full steps are in the `learnings.md` of the same folder). Files: `crates/app/src/sequence.rs`, `crates/app/src/folderops.rs`, `crates/app/src/dialog.rs`, `crates/engine/src/sequence.rs`.

#### TODO

- [ ] On Windows, copy a dozen exported JPEGs whose `DateTimeOriginal` repeats a second into `D:\Photos\tests\20261113-gpui-folder-dialogs\sequence\`, plus a folder with no JPEG at `...\no-jpeg\`. `cargo build` in `crates/app`, delete `index-step4.sqlite`, and run `crates\app\target\debug\handpic.exe D:\Photos\tests\20261113-gpui-folder-dialogs\sequence --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step4.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step4.sqlite --log D:\Photos\tests\20261113-gpui-folder-dialogs\step4.log`. Only these copies.
- [ ] On Windows, right-click `sequence`, pick `Sequence JPEG Timestamps…`. Expect `Source:` / `Output: …\sequence-sequenced`, one row per file `name  old -> new` with the changed field in green and unchanged rows muted, `n files will be written to sequence-sequenced` and `k of them get a new capture time`. `Run` is outlined, Tab moves to `Cancel`, Escape closes.
- [ ] On Windows, `Run`. Expect `Run` dimmed, `Cancel` focused, the running note, the pane counting `sequencing n / m`. At the end the dialog closes, the pane says `Wrote n of n files to …\sequence-sequenced` and Explorer opens with `sequence-sequenced` selected.
- [ ] On Windows, open it a third time. Expect `will be rebuilt: its JPEG files are replaced` under the output line.
- [ ] On Windows, `Run` then `Cancel` (or Escape) at once. Expect the dialog to close, the pane to say `canceled, i of n written` and Explorer not to open.
- [ ] On Windows, pick the item on `no-jpeg`. Expect no dialog and the no-JPEG message in red in the pane. A second `Sequence JPEG Timestamps…` right after the first does nothing, and `Ctrl+,` does not open the settings while the dialog or run is up.
- [ ] Record the results in the archived `learnings.md`.

### App: real-device checks of Import… (gpui-folder-dialogs Step 5)

#### Background

`gpui-folder-dialogs` Step 5 ported Import… to the GPUI app (`import.rs`: preview on a background task, `prepare` / `run_plan` on a thread with a cancel flag, the dialog stepping aside, the first date folder opened through `open_from_tree`). CI and the unit tests of `import.rs`, `context.rs` and `dialog.rs` cover the model. Nobody has seen the dialog, the progress, the open of the first date folder, the reopen or the cancel in the running app. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261113-gpui-folder-dialogs/plan.md` (the full steps are in the `learnings.md` of the same folder). Files: `crates/app/src/import.rs`, `crates/app/src/folderops.rs`, `crates/engine/src/import.rs`.

#### TODO

- [ ] On Windows, copy RAWs shot on two dates (a few with `.xmp` and `.dop`), a few JPEGs of those dates and one JPEG stripped of its Exif as `undated.jpg` into `D:\Photos\tests\20261113-gpui-folder-dialogs\import-src\`. Write `...\settings-step5.json` as `{"importRoot": "D:\\Photos\\tests\\20261113-gpui-folder-dialogs\\import-dst"}` and `...\settings-step5-noroot.json` as `{}`. `cargo build` in `crates/app`, delete `index-step5.sqlite` and `settings-step5.sqlite`, and run `crates\app\target\debug\handpic.exe D:\Photos\tests\20261113-gpui-folder-dialogs\import-src --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step5.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step5.sqlite --import-settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step5.json --log D:\Photos\tests\20261113-gpui-folder-dialogs\step5.log`. Only these copies.
- [ ] On Windows, right-click `import-src`, pick `Import…`. Expect the pane to show `reading …\import-src…`, then the dialog with `n files (… MB) to copy into 2 folders`, one row per date folder, and the undated file under `Without a capture date, not copied:`. `Run` is outlined, Tab moves to `Cancel`, Escape closes.
- [ ] On Windows, `Run`. Expect the first date folder to open in the tree and strip, the dialog to step aside, the pane to count `importing n / m` while the strip fills, then `Imported n files (… MB)`.
- [ ] On Windows, with a larger source (or a slow disk), pick `Import…` again mid-run. Expect the dialog back with `Cancel` focused; Escape hides it and the run goes on; `Cancel` stops it with `Import canceled, k files copied` and no `.handpic-tmp` file left.
- [ ] On Windows, `Import…` on `import-src` again. Expect `Nothing left to copy: every dated file is already there.` with `Run` dimmed. Then replace one copied file in `import-dst` with a different-size file of the same name and re-open the dialog. Expect it under `Already at the destination with another size, not copied:`.
- [ ] On Windows, quit, delete `settings-step5.sqlite`, relaunch with `--import-settings ...\settings-step5-noroot.json`. Expect `Import…` to show `the destination folder is not set (Settings > Import)` with `Run` dimmed. `step5.log` holds one `import: files=… bytes=… mb_per_s=…` line per run.
- [ ] Record the results in the archived `learnings.md`.

### App: real-device checks of Rename by Capture Date… and the folder-rename aftermath (gpui-folder-dialogs Step 6)

#### Background

`gpui-folder-dialogs` Step 6 ported Rename by Capture Date… and the "a folder was renamed" path (`folderops::folder_renamed`, `tree.rs` `rebase` / `rename_folder`, the trash entries' rebase) to the GPUI app. CI and the unit tests of `datefolder.rs` (including `watch_release` on `notify` 8.2.2), `tree.rs`, `history.rs` and `context.rs` cover the models and the watcher release. Nobody has seen the dialog, the tree row following in place, the reopen or the undo of a trash run across the rename in the running app. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261113-gpui-folder-dialogs/plan.md` (the full steps are in the `learnings.md` of the same folder). Files: `crates/app/src/datefolder.rs`, `crates/app/src/folderops.rs`, `crates/app/src/tree.rs`, `crates/engine/src/datefolder.rs`, `crates/engine/src/rename.rs`.

#### TODO

- [ ] On Windows, under `D:\Photos\tests\20261113-gpui-folder-dialogs\step6\` make `one-date\` (a few RAWs of one day, a couple with `.xmp`), `three-dates\` (JPEGs of three days plus `undated.jpg` stripped of Exif), `two-dates\` (RAWs of two days) and `collide\` (RAWs of one day); no two folders share a shooting day. `cargo build` in `crates/app`, delete `index-step6.sqlite` and `settings-step6.sqlite`, and run `crates\app\target\debug\handpic.exe D:\Photos\tests\20261113-gpui-folder-dialogs\step6\one-date --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step6.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step6.sqlite --log D:\Photos\tests\20261113-gpui-folder-dialogs\step6.log`. Only these copies.
- [ ] On Windows, pick `Rename by Capture Date…` on `one-date` during pass 1. Expect no dialog and the pane to say `a scan is running; wait for it to finish`.
- [ ] On Windows, after the scan, judge two files, move another RAW to the Trash, page to the fourth file, then pick the item. Expect `Rename one-date to YYYY-MM-DD?` with one row `YYYY-MM-DD  n files`, `Run` outlined, Tab to `Cancel`, Escape closes. `Run`: the buttons dim, the dialog closes, the tree row reads `YYYY-MM-DD` in place, the strip keeps its files, judgments and the fourth file, and `step6.log` shows `datefolder: … renamed to …` then `scan reconcile: dir=…\YYYY-MM-DD todo=0`. `Ctrl+Z` brings the trashed RAW back into the renamed folder (`Restored 1 file from the Trash`).
- [ ] On Windows, open `collide`, note the date in the dialog, Cancel, create an empty sibling folder of that name, and re-open it. Expect `YYYY-MM-DD already exists next to collide; nothing is merged.`, `Run` dimmed and `Cancel` focused.
- [ ] On Windows, on `three-dates` (after its scan): `Split three-dates into 3 folders by capture date?`, three rows, the undated line and `three-dates is removed if it ends up empty.`. `Run`: three sibling date folders appear, the pane says `Split three-dates into … ; three-dates kept (1 left)`, and the strip re-lists `three-dates` with only `undated.jpg`.
- [ ] On Windows, on `two-dates` (after its scan) `Run`. Expect `two-dates` to leave the tree, the first date folder to open and the pane to say `Split two-dates into … (a), … (b)`. The item on the already-renamed folder shows `YYYY-MM-DD is already named its capture date.`, `Nothing to rename.` and `Run` dimmed.
- [ ] Record the results in the archived `learnings.md`.

### App: real-device checks of the tree's inline folder rename (gpui-folder-dialogs Step 7)

#### Background

`gpui-folder-dialogs` Step 7 ported the folder `Rename…` to the GPUI app: the inline editor over the tree row (`textinput.rs`), the held rename with its pending name during a scan, the slow second click and the cancel and refusal rules (`rename.rs`, `folderops::start_rename` / `gate_request` / `run_rename`). CI and the unit tests of `rename.rs`, `idle.rs` and `context.rs` cover the model and the gate. Nobody has used the editor, its keys, the Japanese IME in it, the slow click or the held rename in the running app. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261113-gpui-folder-dialogs/plan.md` (the full steps are in the `learnings.md` of the same folder). Files: `crates/app/src/rename.rs`, `crates/app/src/folderops.rs`, `crates/app/src/textinput.rs`, `crates/app/src/idle.rs`.

#### TODO

- [ ] On Windows, under `D:\Photos\tests\20261113-gpui-folder-dialogs\step7\` make `Photos\` (a few dozen RAWs so pass 1 runs for seconds on a fresh index, a few with `.xmp`; `Photos\sub\` with a couple of RAWs), an empty `taken\` and `other\` (a few RAWs). `cargo build` in `crates/app`, delete `index-step7.sqlite` and `settings-step7.sqlite`, and run `crates\app\target\debug\handpic.exe D:\Photos\tests\20261113-gpui-folder-dialogs\step7\Photos --index D:\Photos\tests\20261113-gpui-folder-dialogs\index-step7.sqlite --settings D:\Photos\tests\20261113-gpui-folder-dialogs\settings-step7.sqlite --log D:\Photos\tests\20261113-gpui-folder-dialogs\step7.log`. Only these copies.
- [ ] On Windows, after the scan, judge two files and page to the third, then right-click `Photos` > `Rename…`. Expect the row's name to become a field holding `Photos`, all selected. Type `Shots` and Enter: the row reads `Shots` in place, the strip keeps files, judgments and position, the tree keeps the keyboard, and `step7.log` shows `rename: …Photos renamed to …Shots` with no pass 1 work after it. A case-only rename (`shots`) works.
- [ ] On Windows, `Rename…` with `a/b` then Enter. Expect `a/b: a name cannot contain '/'` and the row keeping its name. With `taken`: `taken already exists`.
- [ ] On Windows, `Rename…`, type, Escape: nothing renamed. Clear the field, Enter: nothing happens. With the field open, `a`, `1`, `p`, `Ctrl+Z`, the arrows and Space edit the text only (no judgment, paging or tree move), and `Ctrl+A` / `C` / `X` / `V` work in the field.
- [ ] On Windows, `Rename…`, Alt+Tab away and back. Expect the field still open; clicking the strip confirms the edit.
- [ ] On Windows, with the Japanese IME on, `Rename…`, type `しゃしん` and convert to `写真`. Expect an underlined composition, the candidate window at the field, the first Enter committing the conversion only and the second renaming the folder to `写真`.
- [ ] On Windows, relaunch with a fresh index and, during pass 1, rename the open folder to `Later`. Expect the row `◷ Later` in muted italics (hover: `Renames when the scan finishes`), the pane `Rename…: waiting for the scan to finish`, the scan not canceled, and the rename plus reopen when it ends.
- [ ] On Windows, during a fresh scan rename to `Later2` and re-edit the pending row back to the real name: the row shows the real name and the status says `Rename… was canceled`. Rename to `Later3` and click `other` in the tree: `other` opens, the status says `Rename… was canceled`, the first folder keeps its name.
- [ ] On Windows, click the open folder's name once and wait half a second: the field opens. A double-click, or a click followed by a key or another click within half a second, opens no field. A single click on a root or another folder opens that folder as before.
- [ ] On Windows, during an import run (see the Import check), confirm a rename. Expect `Rename… is unavailable while an import runs`.
- [ ] Record the results in the archived `learnings.md`.

### App: real-device checks of the app icon on Windows, macOS and Linux

#### Background

The `native-app-icon` feature gave the GPUI app its icon: `crates/app/build.rs` embeds `crates/app/icons/icon.ico` as icon resource ID 1 on a Windows target (through `winresource`), `crates/app/packaging/macos/Info.plist` and the `app:bundle:macos` mise task produce `Handpic.app`, and the window opens with `app_id: Some(IDENTIFIER)` with `crates/app/packaging/linux/dev.minodisk.handpic.desktop` and the `app:install:linux` task for Linux. CI and the tests cover the resource: the icon resource ID 1 was verified in the built Windows exe with `FindResourceW` / `LoadImageW`, the `datadir.rs` consistency test checks the `Info.plist` and `.desktop` file against `IDENTIFIER` and the binary's name on all three legs, and the macOS CI leg runs `plutil -lint` and checks that the executable and the `.icns` exist. Nobody has looked at the icon as the OS shows it: the taskbar, title bar and Explorer on Windows, the Dock and Finder on macOS, and the dock / taskbar / alt-tab of a running window on a Wayland and an X11 session. The Step 1 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261114-native-app-icon/plan.md`.

Files: `crates/app/build.rs`, `crates/app/src/main.rs`, `crates/app/packaging/macos/Info.plist`, `crates/app/packaging/linux/dev.minodisk.handpic.desktop`, `mise.toml`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows, run `mise run app:dev`. Expect the Handpic icon, not the generic one, in the taskbar button and the title bar. Also expect it in Explorer for `crates/app/target/debug/handpic.exe`.
- [ ] On macOS, run `mise run app:bundle:macos`, then open `crates/app/target/release/Handpic.app`. Expect the Handpic icon in the Dock and in Finder.
- [ ] On Linux, on a Wayland session and on an X11 session, run `mise run app:install:linux` with `~/.local/bin` on `PATH`, then launch Handpic from the application menu. Expect the running window to show the Handpic icon in the dock / taskbar / alt-tab. If it does not, check `StartupWMClass` (X11) and the `.desktop` file's base name (Wayland) against `IDENTIFIER`.
- [ ] Once the checks above are done (or a failure is filed as its own issue), update the "Pending manual checks" bullet under `## Packaging` in `docs/agents/app.md`. It lists only macOS and Linux and leaves out the Windows check. Done when the bullet matches what is still pending.

### App: real-device checks of the strip's face focus icon and cell tooltips

#### Background

The `native-strip-face-icon-tooltips` feature gave the GPUI app's strip the Tauri strip's face focus icon (a smiling `scan-face` in bright green on a good frame, the frowning variant in gray on a low one, nothing on a neutral one, painted with `paint_svg` at the image box's bottom-left) and its hover texts (the scan's full error on a failed cell, the label's name over a labeled cell's name band). The tooltips are transparent region `div`s over the strip's canvas that build gpui-component's `Tooltip`. The tree's pending rename icon moved from the hand-drawn `Tip` to the same `Tooltip`. The unit tests of `MarkState::icon` and `strip::tip_regions`, clippy and the app tests cover the pure rules. Nobody has watched the running window: the icon's appearance during pass 2, its crispness at 100 % and 150 % scale, the dimming on a rejected cell, the tooltips' delay, wrapping and placement, and whether a region `div` leaves the strip's click, right-click and wheel input alone were never exercised. The Step 1 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261114-native-strip-face-icon-tooltips/plan.md`.

Files: `crates/app/src/main.rs` (`strip_tips`, `paint_face_icon`), `crates/app/src/strip.rs` (`tip_regions`), `crates/app/src/mark.rs` (`MarkState::icon`), `crates/app/src/folderops.rs` (`tree_name`).

#### TODO

- [ ] On Windows, run `mise run app:dev` and open a folder with faces on a fresh index (`--index` pointing at a new file). Expect the smiling `scan-face` in bright green at the image box's bottom-left on good frames and the frowning one in gray on low frames to appear while pass 2 runs, with no other input, and nothing on neutral frames or JPEGs. Expect the 12 px icon to be crisp at 100 % and 150 % display scale.
- [ ] On Windows, reject a good frame. Expect its icon to be dimmed with the cell.
- [ ] On Windows, hover a cell the scan failed on (e.g. a truncated RAW). Expect the full error in a themed tooltip after about 500 ms, wrapping at 360 px and staying inside the window. Hover a labeled cell's name band and expect the label's name. On a failed and labeled cell, expect the band to show the name and the rest of the cell the error. A plain cell shows nothing.
- [ ] On Windows, move from one cell's region to the next and expect the tooltip to swap. Scroll the strip under a still pointer and expect the tooltip of the cell that moved away to go. A cell cut by the window's left edge must still show its tooltip.
- [ ] On Windows, over a cell with a tooltip, click, ctrl / shift click, right-click and wheel. Expect them to still select, open the menu and scroll.
- [ ] On Windows, rename a folder during a scan and hover the tree's pending rename icon. Expect the `PENDING_TITLE` tooltip as before, now in gpui-component's style.
- [ ] On macOS and Linux, when at hand, repeat the checks above (the learnings call for Windows first and "when at hand" for the other two).

### App: real-device checks of the gpui-component adoption (theme, Input, controls, menus, Root, the three Dialogs)

`gpui-component-adoption` called `gpui_component::init` with a Neutral dark theme map, replaced `textinput.rs` with `Input`, moved the settings modal's controls to `Button` / `Checkbox` / `Radio` with scrollbars, gave the Sort / Filter and right-click menus the theme's look, mounted `Root`, and turned the trash / delete dialog, the folder operations' dialog and the settings modal into `Dialog`s. `mise run ci:local`, the pure models' unit tests, the keymap tests and the Step 5 A/B latency gate cover the build and the rules. The crate has no GPUI view test and `--auto` cannot click, so the IME, focus, Tab order, outside click, Escape behavior and look of every step were never exercised on screen. Plan: `docs/plans/_archived/20261114-gpui-component-adoption/plan.md`. Files: `crates/app/src/main.rs`, `crates/app/src/theme.rs`, `crates/app/src/settingsview.rs`, `crates/app/src/settingsmodal.rs`, `crates/app/src/folderops.rs`, `crates/app/src/dialog.rs`, `crates/app/src/trash.rs`, `crates/app/src/modal.rs`, `crates/app/src/icons.rs`, `docs/agents/app.md`.

Run only on copies under `D:\Photos\tests\20261114-gpui-component-adoption\`, with a fresh `--index` and a test `--settings`. Build with `cargo build --manifest-path crates/app/Cargo.toml` (rename a locked `handpic.exe` aside, do not kill it). Launch with `crates/app/target/debug/handpic.exe --index <fresh temp>/index.sqlite --settings <test>/settings.sqlite D:\Photos\tests\20261114-gpui-component-adoption\<copy>`.

#### TODO

- [ ] On Windows (Step 1: init and theme), page with the arrows, press `tab` (the side panes toggle), `f`, `z`, `e`, `d`, open and close the Sort / Filter menus, open the settings with `ctrl+,` and type a label name with the Japanese IME, open a right-click menu on the strip and on the tree, open and cancel the trash dialog, and press `ctrl+shift+i` (the component inspector opens, debug build). Expect everything to look and behave as on `main`, with only `ctrl+shift+i` new. Then run once with `--auto 20` and compare the timing lines with a `main` build's: they should be the same in kind (not the A/B gate).
- [ ] On Windows (Step 2: `Input`), in the settings' Sidecar tab check that the name fields show the stored names with a 1 px white-15 % border, no fill, and the same height and text size as `main` within a pixel or two, and that the focused one has the `#e5e5e5` border and no extra ring. Then:
  - Type a label name with the Japanese IME: the composition is underlined, the candidate window sits at the caret, and Enter inside the composition commits the composition, not the field.
  - Commit with Enter, with a click elsewhere in the box and with Tab (Tab and Shift+Tab move between the modal's stops from inside a field), then reopen the modal and see the names stored.
  - Escape in a field closes the modal with the edited name kept. `Reset` and a language pick show their names in the fields.
  - In the Import tab type a pattern with and without the IME: the example line follows each key, an invalid pattern shows its refusal in red on commit, and an example pattern button fills the field and its example.
  - `ctrl+a` then typing replaces the text; `ctrl+c` / `ctrl+x` / `ctrl+v` work; `ctrl+z` undoes (new).
  - Right-click in a field: no panic, and the OS-native Cut / Copy / Paste / Select All menu shows and works.
  - A key typed in a field (`f`, `z`, `d`, the arrows) acts in the field only.
  - After closing the modal the arrows page again.
- [ ] On Windows (Step 2 catch-up: the tree's inline rename editor, ported to `Input` without being seen on screen), start a rename from the tree (a slow second click on the open folder's name, or the right-click `Rename…`). Expect an editor with the whole name selected, a `#e5e5e5` border, no fill, 13 px text, fitting the row's height. Then:
  - Type a new name with the Japanese IME: Enter inside the composition commits the composition, not the rename.
  - Commit with Enter: the folder is renamed and the tree has the keyboard again. Start another and commit with a click elsewhere: renamed as well.
  - Start another and press Escape: nothing is renamed, the name comes back, the tree has the keyboard.
  - Switch to another app with the editor open: no rename until the window comes back.
- [ ] On Windows (Step 4: menus), the Sort and Filter menus are the `#171717` surface with a white-10 % border and 6 px radius, 26 px rows with 6 px rounded `#262626` hover, the check icon at the left of a checked item, muted headings, separators across the padding, and a disabled item muted with no hover. Use a folder with bursts and EXIF variety so the Filter menu is long. Then:
  - Sort and Filter open above their button with the right edges aligned.
  - An outside press closes an open Sort / Filter menu (in the strip, in the preview, and on the menu's own button, which does not reopen it). An item click applies and leaves the menu open.
  - With the Filter menu open: `escape` closes it, the arrows page, `e` / `z` / `f` toggle their overlays, the burst keys act, and the menu stays open through all but `escape`.
  - Make the window short so the Filter menu exceeds its cap: it scrolls by wheel, shows a scrollbar, dragging the thumb scrolls it, a press on the scrollbar does not close it, and reopening starts at the top.
  - With a Sort / Filter menu open nothing takes the GPUI focus.
- [ ] On Windows (Step 5: `Root`), everything looks as on `main` (menus' rows, settings buttons and fields, empty state's text the same size). Then:
  - `tab` toggles the side panes. Bind `shift+tab` to an action in the Keyboard Shortcuts tab (e.g. `toggleStrip`): `shift+tab` acts.
  - In the settings modal Tab / Shift+Tab walk its stops (from a field too), and the Shortcuts tab's `+` then `tab` adds `tab` as a chip.
  - IME in a label name composes and commits as before.
  - `ctrl+c` in a settings field with text selected copies it; `ctrl+c` outside does what the keymap binds it to, if anything.
  - The menus close on an outside press. The trash dialog's Tab / Shift+Tab move between its buttons, Enter runs the focused one, Escape cancels.
  - Click in the tree, use the arrows there, press `escape` or click the preview: the keyboard comes back to the photos.
  - Quit with `q` and relaunch without a folder: the last folder and the window's place and size come back.
- [ ] On macOS (Step 5: `Root::new`'s window hit-test forwarder), click on every pane (tree, preview, strip, meta pane) and on the status row's buttons, and drag the title bar. Expect each to land where it should. Also repeat the Windows Step 5 list when at hand.
- [ ] On Linux (Step 5: client-side decorations), launch on GNOME Wayland and on an X11 session. Expect the window's edges to look acceptable (record what the frame, shadow and resize bands look like), resizing from the edge to work, and a right-click in a settings field to open Cut / Copy / Paste / Select All (a drawn `PopupMenu` on Linux).
- [ ] On Windows (Step 6: trash / delete dialog), use a fresh copy for each destructive run.
  - Right-click a strip file, `Move to Trash…`: a 640 px dark (`#171717`) box with the title, the line, a red run button and Cancel, the window dimmed, no close button, no animation, and the focus ring on the run button.
  - Enter moves the file to the Trash; again with Space; again with a click on the run button. The dialog closes when the run ends and the pane says `Moved 1 file to the Trash`.
  - Tab, Shift+Tab, Tab: the ring goes Cancel, run, Cancel. Enter on Cancel and Escape close without moving anything.
  - A press on the dimmed backdrop (also a double-click, and near the top edge) or on the box's text does nothing, the window does not move or maximize, and Enter still presses the focused button.
  - `ctrl+,` and a paging key while open do nothing, and the wheel over the strip behind does not scroll.
  - `Delete Permanently…` opens on Cancel. Enter, Space, `shift+enter`, `ctrl+space` do nothing (on Cancel, and after Tab on the run button). Hold Enter from before the menu item's click: nothing happens. Double-click the menu item so the second click lands on the run button: nothing is deleted. A single click on the run button deletes.
  - Escape during a long run does nothing, and both buttons look disabled until it closes.
  - Press `Move to Trash…` during pass 1: the dialog closes, the scan stops, and the run happens at the scan's end without the dialog.
  - `ctrl+z` after a move brings the file back.
  - With the tree holding the keyboard before the dialog opened, Escape and a run's end both leave it holding it. After every close the arrows page and `tab` toggles the side panes.
- [ ] On Windows (Step 7: folder operations' dialog), with the settings' Import root set to a test folder under the same test directory, open each of `Rewrite Sidecars from Index…`, `Delete Sidecars…`, `Sequence JPEG Timestamps…`, `Import…` and `Rename by Capture Date…` from the tree's right-click menu. Expect a 640 px dark box with the title, text, row list, notes, a run button (light for primary, red for Delete Sidecars) and Cancel, the window dimmed, no close button, no animation, and the ring on the run button. Then:
  - Tab, Shift+Tab, Tab: ring goes Cancel, run, Cancel. Enter on Cancel closes; Space on the run button runs it; Escape closes; any other key (`f`, an arrow, `ctrl+,`) does nothing.
  - A press on the backdrop or the box's text (also a double-click near the top edge) does nothing, the window does not move, and Enter still presses the focused button.
  - A disabled run (e.g. Rename by Capture Date on a colliding target): ring on Cancel, Tab keeps it there, Enter closes.
  - A long list: rows scroll by wheel inside the box with a scrollbar, thumb drag scrolls, and notes and buttons stay in view.
  - Sequence on a big folder: the run button turns disabled, the ring moves to Cancel, the running note shows, the pane's progress moves, and Enter or Escape cancels.
  - Import: the dialog steps aside once the folders exist and the window's keys work; `ctrl+,` does nothing; `Import…` brings the dialog back with the ring on Cancel; Escape steps it aside again; Cancel cancels the import.
  - Rewrite Sidecars / Rename by Capture Date during a run: Escape does nothing, both buttons look disabled, and the dialog closes at the run's end.
  - An operation pressed during pass 1 opens its dialog when the scan ends.
  - After each close the tree still has the keyboard, and an inline rename then works (Enter commits, Escape cancels).

### App: real-device checks of the strip, folder tree and meta pane scrollbars

#### Background

The `native-scrollbars` feature gave the GPUI app's thumbnail strip, folder tree and meta pane gpui-component's scrollbar in the theme's `ScrollbarMode::Scrolling` (shown while the area scrolls, fading out after about 2 s idle, taking a press only while shown). The tree's bar sits on a `relative()` wrapper of its `uniform_list`, the meta pane's body ends in `overflow_y_scrollbar()`, and the strip's horizontal bar goes through `StripBar`, a `ScrollbarHandle` over the strip's own model whose pending offset `Handpic::render` applies through `Strip::scroll_to` and `strip_moved`. `stripbar.rs`'s unit tests and `mise run ci:local` cover the pure logic. Nobody has watched the running window: the bars' look, the fade-out, dragging and track clicks, the thumbnails following a drag, and what a press on a shown bar does and does not reach were never exercised. The Step 1 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261115-native-scrollbars/plan.md`.

Files: `crates/app/src/main.rs` (`tree_pane`, `meta_pane`, `render`), `crates/app/src/stripbar.rs` (`StripBar`), `crates/app/src/strip.rs` (`Strip::content_width`).

#### TODO

- [ ] On Windows, on the strip, drag the thumb to the far end of a long folder. Expect the thumbnails there to appear. Expect the bar to show with the thumb following the arrow keys (the current file scrolled into view), a refilter and a window resize.
- [ ] On Windows, on the strip, press on the shown bar. Expect the current file and the selection not to change. Expect the cells' tooltips to still show on hover.
- [ ] On Windows, on the tree, click a row, right-click a row and rename a row inline next to the bar. Expect all three to behave as before.
- [ ] On Windows, open the settings modal's lists and the sort, filter and right-click menus, and scroll them. Expect their scrollbars to behave as before.
- [ ] On macOS and Linux, when at hand, repeat the checks above (the learnings call for Windows first and ideally the other two).

### Dev setup: check that a plain `mise install` adds rust-analyzer to an already installed toolchain

#### Background

The `mise-rust-analyzer` feature added `rust-analyzer` to the rust `components` in the root `mise.toml` and in `crates/app/mise.toml`, so that `mise install` installs the component for both toolchains (Claude Code's rust-analyzer LSP plugin failed with "rust-analyzer was not found"). The automated checks passed. On the author's host, mise reported `rust@<version>` as already installed and the component was already present for both toolchains before the change. So no one has measured whether a host whose toolchain lacks the component gets it from a plain `mise install`, or whether `mise install --force rust` is needed. Plan: `docs/plans/_archived/20261115-mise-rust-analyzer/plan.md`.

Files: `mise.toml`, `crates/app/mise.toml`.

#### TODO

- [ ] At the repository root, run `mise current rust` (expect `1.97.1` or the current resolved version). Run `rustup component remove rust-analyzer --toolchain <resolved version>`, then a plain `mise install`, then `rustup component list --installed --toolchain <resolved version>`. Expected result: the list includes `rust-analyzer-<host triple>`. Pass the resolved full version to rustup, not `1.97`, because rustup treats `1.97` as a separate toolchain and auto-installs it.
- [ ] In `crates/app`, run `rustup component remove rust-analyzer --toolchain 1.98.1`, then a plain `mise install`, then `rustup component list --installed --toolchain 1.98.1`. Expected result: the list includes `rust-analyzer-<host triple>`.
- [ ] Record the result. If a plain `mise install` is not enough, add a `mise install --force rust` line to the setup docs (`docs/agents/app.md` or the README setup section).

### App: real-device check of the folder tree rows' name truncation inside the pane

#### Background

The native-tree-row-truncate work made a folder tree row in the GPUI app as wide as the list, so a long name ends in an ellipsis inside the 220 px pane and the RAW count badge stays whole. It gave the row `.w_full()`, dropped its `.mx_1()`, and put the 4 px inset on the `uniform_list` itself (`.px_1()`). `mise run ci:local` passed, but `crates/app` has no GPUI window test and the agent could not drive the app's window. The cause was read from the gpui-pre 0.3.12 and taffy 0.15.0 sources and never observed, and the `app.md` rule is marked Inferred. Plan: `docs/plans/_archived/20261115-native-tree-row-truncate/plan.md`.

Files: `crates/app/src/main.rs` (`tree_pane`, `tree_row`), `crates/app/src/folderops.rs` (`tree_name`), `docs/agents/app.md` (the folder tree section).

#### TODO

- [ ] On Windows (and any platform the app runs on), run the app (`mise run app:dev`) and open a folder tree holding a folder with a long name (e.g. `2026-10-03-dop-orientation`) at several depths. Check each of the following.
  - Long name: a name longer than the pane ends in an ellipsis inside the pane at every depth, and no row content reaches past the pane's right edge.
  - Count badge: expand a folder holding RAW files. Its count badge is fully visible at the row's right edge, both next to a short name and next to a truncated one.
  - Highlights: the keyboard cursor ring (click in the tree, move with the arrows), the hover highlight and the open row's highlight span the pane less a 4 px inset on both sides, with rounded corners.
  - Scrollbar: with enough rows to scroll, the vertical scrollbar sits at the pane's right edge, and dragging its thumb scrolls the list.
  - Rename: the inline rename editor (`Rename…` from the folder's right-click) fits inside its row, and a held rename's pending name (clock icon, italic) truncates like any other name.
  - Scrolling into view and clicks: the reveal on open, the keyboard cursor and the rename's row are brought fully into view by `scroll_to_item`, and a click and a right-click on a row's inset edge behave as before.
- [ ] Once all six are confirmed, change the folder tree rule in `docs/agents/app.md` ("a tree row takes `w_full()` ... not yet observed in the running app") from Inferred to Measured with the observed result. If a point fails, correct the rule with the observed behavior and, per the plan's risk note, record what the layout actually does in `learnings.md` before trying anything wider.

### App: real-device check of the status row with no current-file info

#### Background

`native-status-row-notes-only` stopped the GPUI app (`crates/app`) from writing the current file's info (`n/total`, file name, pixel size, orientation, shutter, aperture, ISO) into the status row. A preview load now clears the transient note, and `status()` trims the start of the built string so a suffix such as `1:1` or the scan's progress no longer starts with two spaces. The unit tests, clippy and `mise run ci:local` cover the code. The window was never run, so two things are unchecked: whether the preview area keeps its height when the status text is empty (the `Sort ▾` / `Filter ▾` buttons should hold the row's height), and the look of the suffixes at the row's left edge. Plan: `docs/plans/_archived/20261115-native-status-row-notes-only/plan.md`. Files: `crates/app/src/main.rs` (`status_row`, `status`).

#### TODO

- [ ] On Windows (or any platform the app runs on), build `crates/app`, open a folder and page through it with the scan finished and zoom off, so the status text is empty. Then press `z` (adds `1:1`) and drop two files on the window (shows `DROP_TEXT`). Expect the status row to show nothing on the left while paging. Expect the preview area above not to shift in height between the empty and the non-empty status. Expect `1:1` and a scan suffix to start at the row's left edge without leading spaces.

### App: real-device check of Left / Right paging the strip and Space toggling from the folder tree (native-tree-left-right)

#### Background

The native-tree-left-right work lets `previous` / `next` (Left / Right) pass the folder tree's key gate in the GPUI app (`crates/app`), so they page the strip while the tree has the keyboard. Space now expands or collapses the cursor's folder. `TreeKey::Left` / `TreeKey::Right` and `TreeCommand::Focus` are gone. This departs from the Tauri app on purpose. The unit tests in `tree.rs` and `treekeys.rs` and `mise run ci:local` cover the pure models. Nobody has pressed a key in the running window, so the paging with the ring still on the tree cursor, the Space toggle and the swallowed modified arrows are unverified. Plan: `docs/plans/_archived/20261115-native-tree-left-right/plan.md`.

Files: `crates/app/src/main.rs` (`tree_key_down`), `crates/app/src/treekeys.rs`, `crates/app/src/tree.rs`.

#### TODO

- [ ] On Windows (or any desktop platform the app runs on), open a folder with several photos and click a folder name in the tree so the tree has the keyboard (the cursor ring is drawn). Then check each of the following.
  - Press Right / Left: the strip's current file moves to the next / previous photo and the preview follows, while the ring stays on the tree cursor.
  - Press Up / Down / Home / End: the tree cursor still moves.
  - Press Space on a collapsed folder with subfolders: it expands. Press Space again: it collapses. Press Space on a folder listed empty: nothing happens and no type-ahead jump occurs.
  - Press Enter: the cursor's folder opens.
  - Press Shift+Left or Alt+Left: nothing happens (swallowed).

### App: real-device check of the meta pane's `Analysis` group and the AF rows in `Maker note`

`native-meta-analysis` made the GPUI app's (`crates/app`) right-hand meta pane show an `Analysis` group from the values the scan's pass 1 and pass 2 stored in the index (`Eye focus`, `Eyes`, `Eyes closed`, `Eye aspect ratio`, `Yaw`, `Pitch`, `Roll`, `Mesh eye offset`, `Edge gap`, `Sharpness`), and `AF point` / `AF frame` in the `Maker note` group after `AF tracking`. Unit tests in `meta.rs` and `mise run ci:local` cover the row builders, `groups` and the string formats (ported from `meta.test.ts`). Nobody has looked at the running window: what the pane shows for a scanned file, that the rows appear by themselves when the scan reaches the shown file, the failed-preview and hidden-by-refilter cases, and the first preview after a folder open (that path now sets `self.shown`'s index to `None`, so the pane shows the old file name with no index rows until the new folder's first preview arrives) were never exercised. Plan: `docs/plans/_archived/20261114-native-meta-analysis/plan.md`. Files: `crates/app/src/meta.rs`, `crates/app/src/main.rs` (`meta_pane`, the open path's `shown`).

#### TODO

- [ ] On Windows, open a RAW folder with a fresh `--index` in the app (`mise run app:dev`) and show a file while pass 2 is still running. Expected: no `Analysis` group before pass 2 reaches that file, then the group appears without another key press once it does. For a Sony ARW with a face, the group lists `Eye focus` (`Sharp · 0.162` / `Soft · 0.143`), and no `AF eye in focus` or `Focus candidate` row, `Eyes` (`Open · 41` / `Closed · 0`), `Eyes closed`, `Eye aspect ratio`, `Yaw` / `Pitch` / `Roll` (`12°`), `Mesh eye offset`, `Edge gap` and `Sharpness`, each label and value string as written here (the removed Tauri pane named the `Eye focus` row `Eyes at full size`). The `Maker note` group shows `AF point` (`3120, 2080 of 8640 × 5760`) and, when recorded, `AF frame` directly after `AF tracking`, and the `Analysis` group holds neither.
- [ ] On Windows, a file with no analysis value (a JPEG without an AF point, or a RAW pass 2 has not reached) shows no `Analysis` group (a JPEG shows at most `Sharpness`). A file hidden by a refilter shows neither the `Analysis` group nor the AF rows. A file whose preview failed still shows its `Analysis` group and a `Maker note` group holding only the AF rows. Turning the `f` overlay on or off does not change the `Analysis` rows (they never use the on-demand eyes judgment).
- [ ] On Windows, open a second folder while a file of the first is on screen. Expected: until the new folder's first preview arrives, the pane shows no rows from the index (not another file's), then the new file's rows.

### App: real-device checks of the settings as a full-window layer (native-settings-component Step 1)

#### Background

The `native-settings-component` feature's Step 1 replaced the settings `Dialog` with a full-window, opaque, occluding layer drawn by `Handpic::render`, under a focus trap. CI and `mise run ci:local` cover the build and the pure `settingsmodal.rs` tests. The view's keys, pointer, focus trap and the drop behavior were never exercised: the agent session could not drive the GUI, and the step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261115-native-settings-component/plan.md`.

Files: `crates/app/src/settingsview.rs`, `crates/app/src/main.rs`.

#### TODO

- [ ] On Windows, run the app with a test `--settings` store and an open folder, then check each of these. Expected: all as described.
  1. `ctrl+,` shows the settings over the whole window, opaque, with the title row, the controls and the status line.
  2. Typing works in a label name field (Sidecar, XMP format) and in the Import pattern field; Enter and blur commit.
  3. Tab / Shift+Tab cycle only through the settings' controls and wrap (no stop in the tree, strip or preview underneath).
  4. On Keyboard Shortcuts, `+` captures the next key, `tab` and `shift+tab` included.
  5. Pointer presses, hovers (no strip tooltip) and wheel scrolls over the settings do not reach the panes underneath.
  6. A folder dropped on the window while the settings are shown opens nothing (record what actually happens; if it differs, correct the drop rule in `docs/agents/app.md`).
  7. Escape (cancelling a capture first, committing a focused field first) and the `×` return to the main view, where Left / Right page at once.

### App: real-device checks of the settings body drawn with gpui-component's `Settings` (native-settings-component Step 2)

#### Background

The `native-settings-component` feature's Step 2 drew the settings body with gpui-component's `Settings` (sidebar with a search field, four `SettingPage`s), mapped the sidebar colors in `theme.rs`, and replaced the old tab-switch capture cancel with a capture-phase pointer-press cancel. The new `editing` rule (an `Input` key context on the focused path) lets the private search field take keys. CI and the remaining `settingsmodal.rs` tests cover the pure model. Typing in the search field, Escape on a non-empty query, the capture cancel, the Tab trap, the search filtering and the look were never run on a real machine (the step's checkbox was ticked on the automated criteria only). Plan: `docs/plans/_archived/20261115-native-settings-component/plan.md`.

Files: `crates/app/src/settingsview.rs`, `crates/app/src/theme.rs`.

#### TODO

- [ ] On Windows, run the app with a test `--settings` store and an open folder, press `ctrl+,`, then check each of these. Expected: all as described.
  1. The body is a sidebar (search field, Sidecar, Culling, Keyboard Shortcuts, Import) beside the selected page, on the box's dark surface, the selected page lit `#262626`; the search placeholder reads "Search...".
  2. Typing works in the search field, the label name fields (Sidecar, XMP format) and the Import pattern field; a name or pattern commits only on Enter or blur, an invalid pattern shows its reason in red.
  3. Escape in the search field with text clears it; pressed again it closes the settings. Escape in a name or pattern field commits and closes.
  4. On Keyboard Shortcuts, `+` captures the next key, `tab` and `shift+tab` included; `home`, `end`, `q` and a key bound elsewhere show their refusal on the status line; a press on the sidebar or another row's `+` cancels a pending capture.
  5. Tab / Shift+Tab walk the search field and the page's controls and wrap, never reaching the main view.
  6. Searching an action name (for example "burst") lists only its rows; clearing the search brings every item back.
  7. The user's visual check of the new look (group headers, label-left / control-right rows, the sidebar's colors).

### App: real-device checks of the drag-resizable tree and meta panes

`native-pane-resize` made the GPUI app's (`crates/app`) folder tree and meta pane drag-resizable by their boundary with the preview, stored under the `panelWidths` key of the settings store (default 220 px, a pane 160 to 480 px, the preview at least 240 px). Unit tests (`panewidths.rs`, `settings.rs`'s read of `panelWidths`) and `mise run ci:local` cover the arithmetic and the setting. Nobody has dragged a handle in the running window: the drag itself, the cursor and the handle's look, the narrow-window behavior, a release outside the window, the 1:1 re-cut, the 5 px bands against the preview's clicks and the layout at 160 px are all unchecked. The limits (160 / 480 / 240) were not judged by eye. Plan: `docs/plans/_archived/20261115-native-pane-resize/plan.md`. Files: `crates/app/src/main.rs` (`pane_handle`, `drag_pane`, `end_pane_drag`, the root `div` in `render`), `crates/app/src/panewidths.rs`.

Run each check with `--settings <fresh test store>`, `--index <test index>` and `--log <test file>`, never the real store (except where a check says to restart).

#### TODO

- [ ] On Windows, run `mise run app:dev` (or `app:watch`) on a folder and hover the preview's left edge beside the tree. Expect the cursor to turn col-resize and the handle's pill to show, with the 1 px line reading well on the theme.
- [ ] On Windows, drag that edge right and left, then the meta pane's edge. Expect each pane to follow the pointer between 160 and 480 px with the preview taking the rest. Note whether the boundary jumps a few px at the first move.
- [ ] On Windows, with both panes wide, narrow the window. Expect the preview to keep 240 px while the panes narrow (never under 160), and widening the window again to bring the panes back to the dragged widths.
- [ ] On Windows, release a drag over the preview, over a pane, over the strip and outside the window, and restart each time. Expect the widths to come back each time. For a release outside the window, note whether the width is written at once or only at the next click inside.
- [ ] On Windows, press `Tab` (`toggleSides`), `Ctrl+Alt+Left` (`toggleLeft`) and `Ctrl+Alt+Right` (`toggleRight`) to hide and show a pane. Expect it to come back at its last width.
- [ ] On Windows, press `z` for 1:1, then drag a boundary. Expect the crop to be re-cut once the drag settles.
- [ ] On Windows, check that the 5 px bands do not get in the way of the empty-state message's click, the preview's context menu or the panes' scrollbars, and that the tree's rows, its inline rename input and the meta pane's rows lay out at 160 px.
- [ ] On Windows, launch with `--auto`. Expect the panes to start at 220 whatever is stored and a drag to write nothing to `panelWidths`.
- [ ] On macOS and Linux, repeat the hover, drag, narrow-window, restart and hide / show checks above. CI only builds these platforms.

### App: real-device check of the folder tree's file counts on collapsed rows (native-tree-index-counts, native-folder-count-cache, native-folder-count-prune)

#### Background

The native-tree-index-counts work made every drawn row of the GPUI app's folder tree show its file count, expanded or collapsed, when it is above zero. The native-folder-count-cache work moved the count of a row not listed in this session to the index's `folder_counts` table: one row per folder, keyed by the tree node's own path, written off the UI thread whenever the tree learns a count from disk (an expand, `Refresh`, a tree watcher change, `Expand All`, a reveal's chain, an open's listing), carried by a folder rename and written for a date split's new folders, kept by the eviction, and read whole once at startup and after a rename or a date split. A listed folder shows its `read_dir` count. The table starts empty at the upgrade, so no collapsed row shows a count until its folder has been listed once. `mise run ci:local` and the `tree.rs` / `index.rs` / `rename.rs` / `datefolder.rs` unit tests cover the merge, the case folding, the write coalescing and the table's writes, carry and read. Nobody has seen the pills drawn, the cached count after a restart, or the re-read after a rename or a split in the running app. A write still queued at quit is lost (not drained), which the folder's next listing repairs. The native-folder-count-prune work made every successful tree listing also delete the `folder_counts` rows of the child folders the listing no longer holds, and the rows under them, keeping the child leading to any tree root under the listed folder; `index.rs` and `tree.rs` unit tests cover the prune and its order within `CountWrites`, but nobody has seen it in the running app. Plans: `docs/plans/_archived/20261115-native-tree-index-counts/plan.md`, `docs/plans/_archived/20261116-native-folder-count-cache/plan.md`, `docs/plans/_archived/20261117-native-folder-count-prune/plan.md`.

Files: `crates/app/src/tree.rs` (`set_cached_counts`, `set_listed_count`, `CountWrites`, `prune_keep`), `crates/app/src/main.rs` (the tree row's count pill, `tree_listed`, `cache_count`, `prune_counts`, the read at startup, `opened`), `crates/app/src/folderops.rs`, `crates/engine/src/index.rs` (`folder_counts`, `set_folder_counts`, `rename_folder_counts`, `prune_folder_counts`), `crates/engine/src/rename.rs`, `crates/engine/src/datefolder.rs`, `docs/agents/app.md` (the folder tree section).

#### TODO

- [ ] On Windows (`mise run app:dev`), launch the app on an index upgraded from before the cache table: expect no collapsed row to show a count yet. Expand a folder that was never opened and holds RAW files, collapse it, then quit and restart: expect its collapsed row to show the count before you expand it. Open a folder and let it list, restart: the same. A folder never listed shows no count. Expand a folder: its number is the `read_dir` count and looks the same expanded or collapsed. Right-click a folder > `Refresh` and add or remove a RAW outside the app: the count follows, and after a restart the new count shows. Rename a listed folder, restart: the renamed row keeps its count. Run `Rename by Capture Date…` with a split: each new date folder shows its count while collapsed, and the original folder (if undated files keep it) shows none until listed. Expand a folder reached through a mapped network drive, a symlink or a junction, restart: expect its count. Check a long name next to a count pill still truncates with the pill whole. Delete or rename a counted folder outside the app, expand or `Refresh` its parent, quit and restart, then create a folder of the old name: expect no count on it until it is listed. If the rules in `docs/agents/app.md` ("The folder tree is a `uniform_list` over rows kept in the view", the count's sources, writes and reads) are marked Inferred, change them to Measured with the observed result once the above is confirmed.
- [ ] On macOS (`mise run app:dev`), launch the app on an index upgraded from before the cache table: expect no collapsed row to show a count yet. Expand a folder that was never opened and holds RAW files, collapse it, then quit and restart: expect its collapsed row to show the count before you expand it. Open a folder and let it list, restart: the same. A folder never listed shows no count. Expand a folder: its number is the `read_dir` count and looks the same expanded or collapsed. Right-click a folder > `Refresh` and add or remove a RAW outside the app: the count follows, and after a restart the new count shows. Rename a listed folder, restart: the renamed row keeps its count. Run `Rename by Capture Date…` with a split: each new date folder shows its count while collapsed, and the original folder (if undated files keep it) shows none until listed. Expand a folder reached through a symlink or a network volume, restart: expect its count. Check a long name next to a count pill still truncates with the pill whole. Delete or rename a counted folder outside the app, expand or `Refresh` its parent, quit and restart, then create a folder of the old name: expect no count on it until it is listed.
- [ ] On Linux (`mise run app:dev`), the same steps as on macOS (a symlink and a mounted network folder for the last link check).

### App: real-device checks of the title bar (native-title-bar)

#### Background

The `native-title-bar` feature replaced the OS title bar of the GPUI app with gpui-component's `TitleBar` (34 px, the app icon from `icons/app.png` served through `Icons`, and the text "Handpic"). The window options are `..TitleBar::window_options()` with an explicit `titlebar` title "Handpic", and Linux requests `WindowDecorations::Client`. CI and the tests cover the build on all three platforms and a test that `Icons` loads the icon's PNG bytes. The author saw one debug-build launch on Windows 11 with no folder open. In it the bar showed the icon, "Handpic" and the three controls, the process's main window title read "Handpic", and the tree, the empty-preview message and the status row laid out below the bar. Nobody has exercised the controls' clicks, drag, the taskbar / Alt+Tab text, a folder's preview and overlays, the menus' height cap, the resize borders, Snap Layouts, the bar under a dialog, saved window geometry, macOS or Linux. The Step 1 checkbox was ticked on the automated criteria and that launch screenshot only. The theme item depends on `feature/native-themes` mapping `title_bar` / `title_bar_border`; until then the bar paints with gpui-component's load-time values. Plan: `docs/plans/_archived/20261116-native-title-bar/plan.md`.

Files: `crates/app/src/main.rs`, `crates/app/src/icons.rs`, `crates/app/src/modal.rs`, `crates/app/src/session.rs`, `crates/app/src/theme.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows, run the built app. Click minimize, maximize / restore and close, and drag the bar to move the window. Expect the 34 px bar to show the app icon and "Handpic", and the taskbar and Alt+Tab to read "Handpic".
- [ ] On Windows, open a folder. Expect the preview, the `f` overlay, the `d` performance band, the `z` 1:1 crop, the drop outline, the tree, the meta pane, the strip and the status row to lay out below the bar with nothing clipped, and an open `Sort ▾` / `Filter ▾` / right-click menu to stop under the bar instead of growing over it.
- [ ] On Windows (gpui-pre 0.3.12), check the resize borders on all four edges and corners (the top edge over the bar included), Snap Layouts on hover over the maximize button, double-click on the bar to maximize / restore, and the bar under a maximized window.
- [ ] On Windows, while the trash dialog, a folder operation's dialog or the settings modal is open, click minimize / maximize / close and drag the bar. Expect them to work; if the backdrop makes them dead, the fix (mounting the bar outside the dialog layer or leaving it out of the backdrop) is a follow-up issue.
- [ ] On Windows, launch with a `windowBounds` saved before this change. Expect the window to restore without drifting from launch to launch (a one-time offset by the old caption's height is fine).
- [ ] Once `feature/native-themes` maps `title_bar` / `title_bar_border` in `crates/app/src/theme.rs`, on any platform, expect the bar to take those colors and follow a theme switch.
- [ ] On macOS, expect the traffic lights to sit inside the bar without overlapping the icon and title (80 px left padding), the content to start below the bar, and double-click on the bar to follow the system setting.
- [ ] On Linux, under client-side decorations expect the bar to draw its controls and `Root`'s frame and shadow. Under server-side decorations (X11 without a compositor, or a Wayland compositor that insists) expect no second set of controls. WSLg can serve as a first pass once a WSL distribution is installed, but it does not replace a check on a regular GNOME / KDE desktop.

### App: real-device checks of the title bar's settings button (native-title-bar-settings)

#### Background

The `native-title-bar-settings` feature added a gpui-component `Button` with the Lucide `settings` icon as the second child of the app's `TitleBar`, at the right end of the bar. A click opens the settings modal through `Handpic::open_settings`, the same method `ctrl+,` (`meta+,` on macOS) uses, with the same refusals. The press does not move the window: a wrapping `div` stops the left mouse-down from propagating. The tooltip is "Settings" with the `OPEN_KEY` as a `Kbd`. CI and the unit tests cover the tooltip's text and key per platform and that `icons/settings.svg` loads through `Icons`. By hand on Windows 11 (debug build, driven by a script) the author saw the following:
- A press and drag on the gear left the window where it was, and a drag on the empty bar moved it.
- A click opened the settings.
- `Escape` closed the settings and `ctrl+,` reopened them right away.
- The tooltip showed on hover.
- The gear and its hover fill were visible in Neutral Light and Dark.

Never exercised: any shipped non-Neutral theme; minimize / maximize / close after the change; a judgment key or paging after closing the settings the gear opened; the gear pressed while the tree holds the keyboard, a menu is open or an inline rename is under way; macOS; Linux. The Step 1 checkbox was ticked on the automated criteria and the checks above. Plan: `docs/plans/_archived/20261117-native-title-bar-settings/plan.md`.

Files: `crates/app/src/main.rs` (`settings_button`, `open_settings`), `crates/app/src/settingsmodal.rs`, `crates/app/src/icons.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows (`mise run app:dev`), switch to one shipped (non-Neutral) theme in Settings > Appearance, in Light or Dark. Expect the gear and its hover fill to be visible and to follow the theme.
- [ ] On Windows, click minimize, maximize / restore and close after this change. Expect all three to work, and expect a drag on the bar's empty part to still move the window.
- [ ] On Windows, open the settings with the gear, close them, then press a judgment key and page with the arrow keys. Expect them to work at once, with the keyboard back on the view (only `ctrl+,` was tried).
- [ ] On Windows, click the gear (a) with the tree holding the keyboard, (b) with `Sort ▾` / `Filter ▾` open, (c) with a right-click menu open, (d) during an inline folder rename. Expect the settings to open, the menu to close, and the rename to end as a press elsewhere ends it.
- [ ] On macOS (`mise run app:dev`), expect the gear at the bar's right edge with its `mr_2` margin. A press and drag on it should not move the window (the bar's `start_window_move`), while the rest of the bar still drags. A click should open the settings and the tooltip should show `Cmd+,`.
- [ ] On Linux, under client-side decorations, expect the gear left of the window controls. A press and drag on it should not move the window, the rest of the bar should still drag, and a click should open the settings.

### App: real-device check of the settings controls drawn with gpui-component (native-component-controls)

#### Background

The `native-component-controls` feature drew the settings' language picker as a `Select` next to `Reset`, auto-advance as `SettingField::switch`, the Destination root as an `InputGroup` holding a read-only `Input` and `Choose…`, and the shortcut keys as `Kbd` chips. `mise run ci:local`, clippy and the unit tests (`keys.rs`, `settingsmodal.rs`) cover the pure models. The focus trap with the `Select` menu open, the `Select` / `List` key contexts (read from the sources, marked Inferred in the plan), Escape with the menu open and closed, the blur commit of a name field when the menu opens, and the read-only root field were never exercised: `--auto` cannot drive the GUI, and the step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261116-native-component-controls/plan.md`.

Files: `crates/app/src/settingsview.rs`.

#### TODO

- [ ] On Windows, run `mise run app:dev`, open the settings (`ctrl+,`) and check each of these. Expected: all as described.
  1. Tab / Shift+Tab through every page: the focus stays inside the settings on every Tab, including with the language menu open.
  2. Culling: Space and Enter toggle the auto-advance switch.
  3. Sidecar (XMP on): open the language menu by pointer and by keys (Space / Enter, Up / Down); Escape with the menu open closes only the menu, with it closed closes the settings; choose a language and press `Reset`, which fills in the chosen language's names.
  4. Type in a name field, then open the language menu: the field commits on blur. A name or pattern field still commits on Enter, blur and Escape.
  5. Import: press `Choose…`, then select and copy the root path in its field; the path is selectable and copyable, not editable; Escape in it closes the settings.
  6. Keyboard Shortcuts: press `+` then a key, and the capture takes the next key; remove a key with its `×`.

### App: real-device check of the look of the switch and the `Kbd` keys (native-component-controls)

#### Background

The `native-component-controls` feature drew auto-advance as a switch and the Keyboard Shortcuts chips and the right-click menu's shortcuts as `Kbd`. The switch's `switch` / `switch_thumb` tokens are `Default Dark`'s, because `theme.rs` maps neither; the plan decided to map them only if the off state is hard to read on screen. Nobody has looked at it. Mapping them would conflict with `20261115-native-themes` Step 1's rewrite of `map`. The key spelling also changed (`Ctrl+Alt+1`, `Left`, `Esc`). Plan: `docs/plans/_archived/20261116-native-component-controls/plan.md`.

Files: `crates/app/src/settingsview.rs`, `crates/app/src/main.rs` (`context_item`), `crates/app/src/theme.rs`.

#### TODO

- [ ] On Windows, run `mise run app:dev`. In the settings (`ctrl+,`), check that the off auto-advance switch and its thumb read clearly against the settings' `#171717` and that the Keyboard Shortcuts `Kbd` chips (13 px `foreground` on `muted`) look right. Then right-click a strip cell and check that the `Kbd` shortcuts (`Ctrl+Alt+1`, `Space`) are right-aligned in `muted_foreground`. Expected: if the off switch is hard to read, map `switch` / `switch_thumb` in `theme.rs` with the theme test.

### App: real-device check of the OS notification when an import ends while the window is inactive (native-import-notification)

#### Background

The native-import-notification work makes the GPUI app (`crates/app`) post one system-only OS notification when an `Import…` run ends while the main window is not active. The title is `Import finished`, or `Import canceled` after a cancel, and the body is the status line's summary (`import::done_status`). The app also now sets its process identity at startup (`cx.set_app_identity`), which on Windows sets the explicit AppUserModelID `dev.minodisk.handpic`. `mise run ci:local` and the unit tests of `import::done_notification` (the decision and the text) cover the pure part. The toast's actual delivery was never observed, nor was the click activation or the taskbar icon and grouping after the AppUserModelID was set. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261116-native-import-notification/plan.md`.

Files: `crates/app/src/folderops.rs` (`import_ran`, the deferred post), `crates/app/src/main.rs` (`set_app_identity`), `crates/app/src/import.rs` (`done_notification`), `docs/agents/app.md` (the section "An import's end posts a system-only notification while inactive").

#### TODO

- [ ] On Windows, build with `mise run app:dev` (or `app:release`), start an `Import…` from a card or folder, switch to another application, and wait for the run to end. Expect one toast titled `Import finished` (or `Import canceled` after a cancel) whose body is the status line's summary. Click the toast and expect the Handpic window to come forward. Run an import that ends with the Handpic window active and expect no toast. Check that the taskbar icon and grouping of the running app look unchanged (an existing pin of the exe may no longer group with the running window), and that `Handpic.log` has no `failed to register AppUserModelID` or `cannot show a system notification` warning.
- [ ] Once the above is confirmed, replace the "(Inferred)" marker on the `docs/agents/app.md` section "An import's end posts a system-only notification while inactive" with the observed result. If the toast, the click activation or the taskbar grouping misbehaves, file a fix issue instead.

### App: real-device checks of the gpui-kit umbrella crate (native-gpui-kit)

#### Background

`native-gpui-kit` Step 1 made `crates/app` depend on `gpui-kit` `=0.9.1` alone (no direct `gpui-pre`, `gpui-pre-platform`, `gpui-component` or `gpui-kit-assets`), import GPUI through `gpui_kit::*`, start through `gpui_kit::init` / `gpui_kit::open_window`, and optimize the framework crates in the dev profile. The umbrella crate also turns `font-kit`, `x11`, `wayland` and `runtime_shaders` on for `gpui-pre-platform` (read from manifests and sources, not run): before, macOS had no `font-kit` and Linux had neither backend. `mise run ci:local` (879 native tests, clippy) and the `native` CI job cover the build. On Windows a debug launch ran 20 s without a panic. No one has looked at the window on any platform, and macOS and Linux were never run. The Step 1 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261116-native-gpui-kit/plan.md`. Files: `crates/app/Cargo.toml`, `crates/app/Cargo.lock`, `crates/app/src/main.rs`, `crates/app/src/theme.rs`, `crates/app/src/icons.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows, run `mise run app:dev` with the real settings. Expected: the window restores its saved place, the Lucide icons (strip, menus) and the settings modal's controls (the `Checkbox` check mark, the `Radio`s, the `Input` fields) draw, and `tab` behaves as before (`toggleSides` in the main view, the focus walk inside a dialog).
- [ ] On macOS, run `mise run app:dev` (the build now has `font-kit` and `runtime_shaders`). Expected: the window draws text (the status row, the meta pane, the menus) and opens without a shader error at startup.
- [ ] On Linux, in an X11 session and in a Wayland session, run `mise run app:dev` (the build now compiles both backends). Expected: the window opens, draws, and takes the keyboard.

### App: real-device checks of the Appearance tab (mode, shipped themes) (native-themes)

The `native-themes` feature made the GPUI app's theme a setting: Step 2 stored `theme` (mode System / Light / Dark), added a Neutral light palette and an `Appearance` tab; Step 3 vendored gpui-component's 21 theme files (36 themes) and added a light-theme and a dark-theme `Select`. `mise run ci:local` and the unit tests cover the setting rule, the mode resolution, the palette mappings, the `Roles` derivations and the theme-file parsing. Nobody has looked at the result on a real machine: both steps' checkboxes were ticked on the automated criteria only, so Light, System following the OS, the shipped themes' readability, the fixed judgment colors on light surfaces and the pickers' keys were never exercised on screen. Plan: `docs/plans/_archived/20261115-native-themes/plan.md`.

Files: `crates/app/src/theme.rs`, `crates/app/src/main.rs`, `crates/app/src/settingsview.rs`.

#### TODO

- [ ] On Windows, open the settings (`ctrl+,`), Appearance, choose Light. Expect the panes, the strip, the status row, the menus, the dialogs, the settings box and the `TitleBar` to turn light and stay readable: the strip's stars (`#ffd050`) and the pick green on the light strip, the tree cursor's ring, the menus' hover and the progress bar. Choose System and switch the OS appearance while the app runs: the app follows it without a restart. Choose Dark: it stays dark whatever the OS does. Restart: the choice is restored before the window is drawn. Record what is hard to read in the archived plan's `learnings.md` (the fixed judgment colors are not retuned in this plan).
- [ ] On Windows, in Dark mode pick at least one shipped dark theme (e.g. Tokyo Night, Catppuccin Mocha) from `Dark theme`, and in Light mode one shipped light theme (e.g. Catppuccin Latte, Solarized Light) from `Light theme`. Expect the panes, the strip (names, burst band, reject dim), the status row, the menus, the dialogs, the settings box and the `TitleBar` to take the theme at once and stay readable, the preview's surround taking the darkest of the theme's surfaces (`Roles::preview`; black in Neutral dark, per `preview-theme-background`, plan `docs/plans/_archived/20261118-preview-theme-background/plan.md`). Check the derived `Roles` on both (the strip's file names and failures, the burst band, the dimmed reject) and the fixed stars / pick / reject colors on each. Pick Neutral again: the app looks exactly as before (Neutral dark unchanged). In System mode with two non-Neutral themes chosen, switch the OS appearance: the app swaps between them. Restart: the choice is restored. Pickers' keys: Space / Enter open the menu, the arrows move, Enter picks, Escape with the menu open closes the menu only, with it closed closes the settings.
- [ ] On macOS, repeat both Windows checks above (Light / System / Dark with `meta+,`; the shipped light and dark themes; the pickers' keys).
- [ ] On Linux, repeat both Windows checks above (Light / System / Dark; the shipped light and dark themes; the pickers' keys). Check System with `window.appearance()` following the desktop's appearance switch.

### App: check the upstream licenses of the shipped gpui-component themes before packaging (native-themes)

The 21 vendored files in `crates/app/themes/` carry no license of their own. Upstream (longbridge/gpui-kit) ships them in its Apache-2.0 repository, but each adapts a third-party scheme (iTerm2-Color-Schemes, Zed's Ayu, Catppuccin, Solarized, Tokyo Night, Twilight / MacroMates and others) whose own license was not checked one by one. `crates/app/themes/LICENSE` holds only the source, the per-file author / url table and the Apache-2.0 text, and `docs/agents/app.md`'s license section does not mention the gap. Plan: `docs/plans/_archived/20261115-native-themes/plan.md`.

Files: `crates/app/themes/LICENSE`, `docs/agents/app.md`.

#### TODO

- [ ] Check each adapted scheme's license and add any missing attribution to `crates/app/themes/LICENSE`, or drop a theme whose license is incompatible. Record the result in the guide's license section. Done when every file in `crates/app/themes/` has a checked upstream license noted in `LICENSE` and the guide's license section states the result.

### App: real-device checks of the right-click menus drawn as gpui-kit's PopupMenu (native-context-menu)

`native-context-menu` replaced the hand-built `div` menus behind the folder tree's and the thumbnail strip's right-click menus (`context_popover`, `context_row`, `context::Menu` / `Open`) with gpui-component's `PopupMenu` through gpui-kit, placed by the view itself with `deferred` / `anchored` so the menu still flips near the window's edges. `mise run ci:local` and the remaining `context.rs` tests (`folder_groups`, `strip_groups`, `shown_path`) cover the items and groups. The crate has no GPUI view test, so the menu itself was never exercised: the step was ticked on the automated criteria only. Four things were read from the source and never run: the key order (the menu takes `escape` / `up` / `down` / `enter` before the view's `key_down`, while `left` / `right` and every other key still reach it), the canvas-based hand-back of the tree's keyboard on a press on the menu, the focus returning to the view after every close path, and `Rename…` ending with the inline `Input` focused. Windows only. Plan: `docs/plans/_archived/20261116-native-context-menu/plan.md` (the steps and expected results are also in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/context.rs`, `crates/app/src/folderops.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows, the tree's menu. In `handpic` on a folder with RAW files, right-click a root and a non-root folder, once with the tree holding the keyboard (left-click in the pane first) and once without. Expected: a `PopupMenu` opens at the pointer with the folder groups (the root without `Rename…` and `Expand All` / `Collapse All`) and separators between groups; the folder does not open and the tree's keyboard state does not change.
- [ ] On Windows, the strip's menu. Right-click a cell inside a multi-selection and one outside it, a gap, the empty area, and a cell in a view-only (JPEG-only) folder. Expected: the selection rule holds; the judgment groups show their shortcuts as `Kbd` text with the current flag, stars and label checked; the gap and the empty area open nothing; the view-only menu has no judgment groups.
- [ ] On Windows, placement and size. Right-click a cell at the window's right edge: the menu flips to the pointer's left. Right-click on the strip near the bottom: it opens above the pointer. Shrink the window: the strip's menu scrolls within the window less the title bar and 40 px.
- [ ] On Windows, the items. Run `Rename…` (ends with the inline `Input` focused), `Copy Path`, a judgment (applies to the targets) and `Move to Trash…` (its dialog opens).
- [ ] On Windows, with a menu open. Escape closes it only. A keymap key (for example `p`) closes it and acts. Left / Right close it and page the strip. Up / Down move the highlight and Enter runs the item. A press outside closes it. A second right-click elsewhere replaces it. After each close the keymap works again (the focus is back on the view), and a press on the menu while the tree had the keyboard hands it back.

### App: real-device check of the catalog-served icons (native-catalog-icons)

#### Background

`native-catalog-icons` made the GPUI app (`crates/app`) serve `star`, `square`, `rectangle-vertical`, `rectangle-horizontal` and `scan-face` from `gpui-kit-assets` instead of its own copies of the Lucide SVGs. `star` and `square` come from the default `Assets` set, and the other three are selected with `icon_assets!`. The app keeps only the three derivatives the catalog lacks: the filled star, the filled square and the frowning `scan-face`. Unit tests cover the asset source: every one of the eight icon paths loads an SVG, each catalog const equals its `IconName` variant's `path()`, a derivative loads its own bytes, and an unknown or empty path loads nothing. `mise run ci:local` and clippy pass. The catalog's SVGs differ from the old copies only in whitespace and in carrying `width="24" height="24"`, so no visible change is expected. Nobody has looked at the rendered icons in the running window, and Step 1's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261116-native-catalog-icons/plan.md`.

Files: `crates/app/src/icons.rs`.

#### TODO

- [ ] On Windows, run `mise run app:dev` and open the status row's `Filter ▾` menu. Confirm the star rows (outlined and filled), the label squares (outlined and filled), the orientation rectangles (vertical, horizontal) and the `Face focus` faces (smiling and frowning) look as they did on `main`. Then, with a folder open and pass 2 finished, confirm the strip's face focus icon on a good frame and on a low frame looks as before. Expected: no visible change.
- [ ] On macOS and Linux (any platform at hand), repeat the Windows check above.

### Docs: the gpui-port plan's Progress entries link plan folders that have since been archived

#### Background

`docs/plans/20261109-gpui-port/plan.md` (Progress section) has an entry for sub-milestone 1 on 2026-11-14 that links `../20261114-gpui-ci-packaging/plan.md`. Its 2026-11-16 re-decision entry links `../20261116-gpui-packaging-redecision/plan.md`. Both folders are archived under `../_archived/`, or will be once their wrap-up runs. The gpui-packaging-answers plan fixed the stale path only in the lead of `packaging-updater.md`. It left these Progress entries as history, as its "Trade-offs and risks" says. Plan: `docs/plans/_archived/20261116-gpui-packaging-answers/plan.md`. Files: `docs/plans/20261109-gpui-port/plan.md`.

#### TODO

- [ ] Retarget the two Progress links to `../_archived/20261114-gpui-ci-packaging/plan.md` and `../_archived/20261116-gpui-packaging-redecision/plan.md` (check the folders' actual names under `_archived/` first), then run `mise run lint` (lychee) to confirm the links resolve.

### App: real-device check of the folder tree rows drawn as ListItems (native-tree-listitem)

#### Background

The native-tree-listitem work draws each row of the GPUI app's folder tree (`tree_row`) as a gpui-component `ListItem`, with a Lucide `folder` / `folder-open` icon. The native-tree-no-chevron work then removed the chevron and its slot: a row is the indent, the folder icon, the name and the count, and a left click on a row opens and toggles it by `tree::row_click`'s rules. The open row is `.selected(true)` and the keyboard cursor's row is `.secondary_selected(true)`. `theme.rs`'s `map` now sets `list_active` and `list_hover` to `accent` and `selection` to `ring` for Neutral, decided from the sources and not from a running app. `selection` also paints `Input` text selection, so the settings modal's and the inline rename's text selection should now read gray in Neutral. `mise run ci:local` and the `every_icon_path_loads_an_svg` test cover the icon paths and the build. `crates/app` has no GPUI window test, so the app was never run: the row layout inside `ListItem` (name truncation, the count position), the colors in each theme and the click behavior are unverified. Each step's checkbox was ticked on the automated criteria only. Plans: `docs/plans/_archived/20261116-native-tree-listitem/plan.md` and `docs/plans/20261117-native-tree-no-chevron/plan.md`.

Files: `crates/app/src/main.rs` (`tree_row`, `tree_row_clicked`, `reveal`), `crates/app/src/tree.rs` (`row_click`), `crates/app/src/icons.rs`, `crates/app/src/theme.rs` (`map`), `crates/app/src/folderops.rs` (`tree_name`), `docs/agents/app.md`.

#### TODO

- [ ] On Windows (and any platform the app runs on, best on Windows), run `mise run app:dev` and open a folder tree. Check each of the following in Neutral dark, Neutral light and one vendored theme.
  - States: the open row is lit (Neutral: `accent`, semibold). The hover lights other rows only, not the open or cursor row. The keyboard cursor is outlined (Neutral: `ring`) only after a click in the pane, and the outline also shows on the open row.
  - Clicks: a click on a collapsed row that is not open opens the folder and expands it. A click on an expanded row that is not open opens the folder and the row stays collapsed once the reveal ends. A click on the open row's icon, indent or count toggles it without re-listing the folder (the scan does not restart). A click on the open row's name only arms the slow-click rename and neither toggles nor reopens. A click on an open leaf row does nothing. A right-click opens the context menu.
  - Rename: a slow click on the open row's name starts the inline rename, whose input fits the 22 px row, and Escape cancels it.
  - Name states: a failed folder's name is red, also on the open row. A pending rename is italic and muted.
  - Layout: the RAW count badge sits at the row's right end. A deep path at the default 220 px pane width truncates the name and keeps the count in the row. A long tree scrolls with uniform row heights. The folder icons draw with no chevron and no empty slot before them (an expanded row shows `folder-open`), and the indent alone still reads the depth clearly.
  - Text selection: the settings modal's and the inline rename's text selection reads as gray in Neutral.
- [ ] If Neutral's open row, hover or cursor outline looks wrong, or the 16 px indent leaves too little room for names, record the observed result in the archived `learnings.md` and decide whether to adjust `theme.rs`'s `map` or the indent.

### App: real-device check of the strip menu's judgment order and star rows (native-strip-menu-order)

#### Background

`native-strip-menu-order` made the strip's right-click menu in the GPUI app (`crates/app`) follow the Filter menu for its judgment groups. The flag group now runs Pick, Unflag, Reject. The rating group runs from 5 stars down to the unrated item, and each row draws the Filter menu's star icons in place of the `N stars` / `No stars` text. The label group is unchanged. Each item keeps its shortcut, its checked state and its `Action::Judge` action. `context::Item.label` now reuses `menu::Label`, and `main.rs` draws both menus' labels through one `menu_label` helper. Unit tests in `context.rs` assert the new order and the star counts, and a pin test compares the flag, rating and label order with `filter_rows`. `menu.rs`'s tests pass unmodified, and `mise run ci:local` and the PR's GitHub checks pass. Nobody has right-clicked a strip cell in the running app, and Step 1's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261116-native-strip-menu-order/plan.md`.

Files: `crates/app/src/context.rs`, `crates/app/src/main.rs`.

#### TODO

- [ ] On Windows, run `mise run app:dev` with `--settings` and `--log` pointing into a test folder (as `docs/agents/app.md` requires, so the real store is not touched). Open a folder and right-click a strip cell of a picked, rated (for example 3 stars), labeled file. Expected: the flag group reads Pick, Unflag, Reject. The rating group shows five rows of 5 down to 1 lit yellow stars, then a row of five dim stars. The label group is Red to Purple, then No label. The checks sit on Pick, the 3-star row and the file's label. Each row shows its shortcut at the right end (`p`, `u`, `x`, `5` ... `1`, `0`). Clicking a star row sets that rating.
- [ ] On macOS and Linux (any platform at hand), repeat the Windows check above.

### App: real-device check of the Keyboard Shortcuts page's groups

#### Background

The `native-shortcut-groups` feature grouped the settings modal's Keyboard Shortcuts page into `Filmstrip`, `Judgment`, `Preview`, `Panes` and `General`, with `Reset all` alone in a final untitled group. Unit tests in `settingsmodal.rs` and `mise run ci:local` cover the category mapping, its order and titles, and that every default action has an explicit category. Nobody has run the page on screen: the key edits in the groups, the key capture, the sidebar sub-entries and the search were never exercised. The step's checkbox was ticked on the automated criteria. Plan: `docs/plans/_archived/20261116-native-shortcut-groups/plan.md`. Files: `crates/app/src/settingsview.rs`, `crates/app/src/settingsmodal.rs`.

#### TODO

- [ ] On Windows (any platform the app runs on), run `mise run app:dev`, open the settings (`ctrl+,`) and go to Keyboard Shortcuts. Expect the rows under `Filmstrip`, `Judgment`, `Preview`, `Panes` and `General`, then `Reset all`. In at least two groups (e.g. `Filmstrip` and `Panes`), add a key with `+` (`Press a key...` shows and the key is added), remove it with its `×` (the focus returns to the box), `Reset` a row, then `Reset all`. An `APP_KEYS` key (e.g. `ctrl+,`) is refused as before. The sidebar lists the five groups under `Keyboard Shortcuts` and a click scrolls to the group. Searching an action's label shows only its group, and `Reset all` is found by its keyword.

### App: a held open-and-collapse click leaves the row expanded when a rename moves the folder

#### Background

`native-tree-no-chevron` made a click on an expanded row that is not open open the folder and collapse the row. `tree_row_clicked` collapses the row and sets `Handpic::reveal_collapsed`, and `reveal` skips the chain's last step only when the path matches it through `tree::relation`. When the gate holds the open (`deferred_open`, `self.gate.in_flight()`), a rename that moves that folder before the deferred reveal runs makes the stored path no longer match, so `reveal` expands the end and the row ends up expanded. Local review noted this and no test covers it. `crates/app` has no GPUI window test. Plan: `docs/plans/_archived/20261117-native-tree-no-chevron/plan.md`.

Files: `crates/app/src/main.rs` (`tree_row_clicked`, `reveal`, `reveal_collapsed`, `deferred_open`), `crates/app/src/tree.rs` (`relation`), `crates/app/src/folderops.rs`.

#### TODO

- [ ] Confirm whether the case is real: hold an open-and-collapse click with a scan or listing in flight, rename that folder before the gate releases, and check whether the row ends up expanded.
- [ ] If it is, carry `reveal_collapsed` along with the folder's rename (compare with how the rename carries the tree rows and index rows), or clear it on a rename, and add a unit test for the chosen rule.

### App: real-device check of the color label Tags in the menus and the strip band (native-label-tags)

#### Background

The `native-label-tags` feature draws the color labels with gpui-component's `Tag` and `ColorName`: the `Filter ▾` menu's label rows and the strip right-click menu's eight label items are small Tags holding the label's name (`No label` an outline secondary Tag), and a labeled strip cell's name band is filled with the label's `ColorName` at scale 600 (`LABEL_BAND_SCALE`). `mise run ci:local`, clippy and the pure-model tests (`menu.rs`, `context.rs`) cover the build and the rules. Nobody has looked at the running window: the Tags' size and clipping, the row height, the hover readability and the band's contrast were never seen, and the step's checkbox was ticked on the automated criteria only. The user deferred this check on 2026-11-17, and no PR description records what was seen. Plan: `docs/plans/_archived/20261116-native-label-tags/plan.md`.

Files: `crates/app/src/main.rs` (`menu_row`, `context_item`, `label_tag`, `paint_strip`).

#### TODO

- [ ] On Windows (or any other desktop platform at hand), run `mise run app:dev`, open a RAW folder, open the status row's `Filter ▾` menu and a strip cell's right-click menu, and label a file; do this in a dark theme (Neutral dark) and again in a light theme (Neutral light). Expected: the seven color label rows (`Red`, `Orange`, `Yellow`, `Green`, `Blue`, `Pink`, `Purple`) and `No label` in both menus are small Tags holding the name, not clipped, with the row height still `MENU_ROW` (26 px); the Tag stays readable while its row is hovered (accent background); the filter popover is wide enough for its rows; the right-click menu keeps the check mark column and the right-aligned `Kbd` shortcut; a labeled strip cell's name band is filled with the label's 600-scale color with the name readable, the yellow band above all.

### App: real-device Windows check of the momentary grayscale preview

#### Background

The `gpui-grayscale` feature grays the shown photo in the GPUI app (`crates/app`) while the `grayscale` key (default `g`) is held. It uses the per-sprite `grayscale` flag of `Window::paint_image` on the fit image and on the 1:1 placeholder and crop. Only the photo is grayed: the peaking overlay (`e`), the `f` overlay and the performance graph (`d`) stay colored. The hold ends on the release of the held key, on the release of any modifier, on window deactivation, and when the app view's focus handle is blurred (`cx.on_blur`). Unit tests of `Hold` (`grayscale.rs`) and `mise run ci:local` cover the state machine. Nobody ran the GUI: the blur release was read from the GPUI sources only, and the visual result, the auto-repeat behavior and the focus-leaving paths were never exercised. Once all pass, change the "(Inferred)" tag of the grayscale section in `docs/agents/app.md` to "(Measured)". Plan: `docs/plans/_archived/20261117-gpui-grayscale/plan.md`.

Files: `crates/app/src/grayscale.rs`, `crates/app/src/main.rs`, `crates/app/src/zoom.rs`

#### TODO

- [ ] On Windows, run the GPUI app (`mise run app:dev` or the built binary), open a folder and hold `g` in the fit view. Expect the photo to turn gray, and color to return on release. Repeat in 1:1 (`z`): the placeholder and the crop both gray.
- [ ] On Windows, with `f` and `e` (and `d`) on, hold `g`. Expect the AF mark, face boxes, mesh, labels, the peaking layer and the performance graph to stay colored, and the strip, the panes and the empty-state text to be unaffected.
- [ ] On Windows, hold `g` long enough to auto-repeat. Expect no flicker.
- [ ] On Windows, rebind `grayscale` to `ctrl+g` in the settings. Hold Ctrl+G and release Ctrl first, then repeat releasing `g` first. Expect color to return on the first release either way.
- [ ] On Windows, hold `g` and Alt+Tab away and back. Expect the photo to be in color.
- [ ] On Windows, hold `g` and open the settings (`ctrl+,`) or a right-click menu, release `g`, then close it. Expect the photo to be in color (the view's `on_blur` released the hold). If it stays gray, release the hold where the view opens the settings, a dialog or a menu instead.
- [ ] On Windows, while `g` is held, page, press `z`, `f`, `e` and `d`. Expect them to behave as before.

### App: real-device checks of the drag-resizable strip height (native-strip-resize)

`native-strip-resize` made the thumbnail strip's height in the GPUI app (`crates/app`) draggable by its top edge, stored under the `stripHeight` key of the settings store (default 128 px, 96 to 302 px, the preview at least 240 px), with the cells' image box scaling with the height. Unit tests (`stripheight.rs`, `strip.rs`'s geometry, hit test, range, content width and carried-offset tests, `settings.rs`'s read of `stripHeight`) and `mise run ci:local` cover the arithmetic and the setting. Nobody has dragged the handle in the running window: the drag itself, the cursor and the handle's line, the marks on the smallest cells, the sharpness of the largest, a release outside the window, the 1:1 re-cut and `--auto` are all unchecked. The limits (96 / 302 / 240) were not judged by eye. Plan: `docs/plans/_archived/20261117-native-strip-resize/plan.md`. Files: `crates/app/src/main.rs` (`strip_handle`, `StripDrag`, `drag_strip`, `end_pane_drag`, the wrapper `div` around the middle row and the strip in `render`), `crates/app/src/stripheight.rs`, `crates/app/src/strip.rs` (`Geometry`, `Strip::set_geometry`).

Run each check with `--settings <fresh test store>`, `--index <test index>` and `--log <test file>`, never the real store (except where a check says to restart).

#### TODO

- [ ] On Windows, run `mise run app:dev` on a folder and hover the strip's top edge. Expect the cursor to turn row-resize and the handle's 1 px line to read well on the theme, with clicks on the cells and the scrollbar still working.
- [ ] On Windows, drag the strip's top edge up and down. Expect the strip to follow the pointer between 96 and 302 px, the thumbnails to scale with it, the preview never to get under 240 px, and the cells that were in view (the current one when on screen) to stay in view. Note whether the edge jumps a few px at the first move.
- [ ] On Windows, at 96 px, check that the stars, the flag dot, the burst badge, the face icon and the file name are legible on the 96 x 64 boxes. At 302 px, check the thumbnails' sharpness at 100 % and on a HiDPI display.
- [ ] On Windows, release the drag over the preview, over a pane, over the status row and outside the window, and restart each time. Expect the height to come back each time. For a release outside the window, note whether the height is written at once or only at the next click inside.
- [ ] On Windows, press `z` for 1:1, then drag the edge. Expect the crop to be re-cut once the drag settles. Press `toggleStrip` to hide the strip and its handle, and again to bring both back at the dragged height.
- [ ] On Windows, launch with `--auto`. Expect the strip to start at 128 whatever is stored and a drag to write nothing to `stripHeight`.
- [ ] On macOS and Linux, repeat the hover, drag, scaling, restart, 1:1 and hide / show checks above. CI only builds these platforms.

### App: real-device checks of the Cache page and Clear Cache (native-clear-cache)

#### Background

The `native-clear-cache` feature ported the Tauri settings modal's `Cache` tab to the app's settings modal: the index size line, a `Clear Cache` button with a confirmation `Dialog`, a clear held until a running scan finishes, and the refresh of the open folder, the tree's cached counts and the size afterwards. `mise run ci:local` and the unit tests (`cache.rs` flow, texts and on-disk size, the `TABS` order) cover the pure parts. The dialog opened over the settings layer (Escape and focus routing), the held clear, the refresh and the size shrinking were never exercised: `--auto` cannot drive the GUI, and the step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261117-native-clear-cache/plan.md`.

Files: `crates/app/src/main.rs` (`press_clear`, `follow_clear`, `cache_dialog`, `run_clear`, `cleared`, `refresh_index_size`), `crates/app/src/settingsview.rs` (`cache`), `crates/app/src/cache.rs`.

#### TODO

- [ ] On Windows, run `mise run app:dev` with a test `--settings` / `--index` and check each of these. Expected: all as described.
  1. Open a scanned folder, open the settings (`ctrl+,`) and pick `Cache`: `Index cache: <size>` with a non-zero size, the explanation paragraph and `Clear Cache`.
  2. Press `Clear Cache`: `Clear the index cache?` opens over the settings with the dimmed backdrop, focus on `Clear`. Press Escape: only the dialog closes, the settings stay open, nothing is cleared (size unchanged), and Tab still walks the settings' controls.
  3. Press `Clear Cache` again and confirm with Enter: the log has `index cleared: EvictSummary { … }`; the size shrinks; the open folder is listed again and pass 1 / pass 2 refill thumbnails, the meta pane and the filter facts, the current file staying put; the tree's cached counts of unlisted folders disappear.
  4. Open a large unscanned folder and, while `scanning` runs, press `Clear Cache`: the note `The cache is cleared as soon as the running scan finishes.` shows, then the dialog opens by itself when the scan ends. Repeat and close the settings while waiting: no dialog at the scan's end.

### App: a held Clear Cache misses idle points that do not pass through `finished`

#### Background

The `native-clear-cache` feature drains a held Clear Cache only in `Handpic::finished` (`crates/app/src/main.rs`). A view that goes idle without a listing ending, for example after a trash undo into another folder (`restored`) or a failed rename (`settle_folder_run`), leaves the clear held while the Cache page still says it clears when the scan finishes. It stays held until the user presses `Clear Cache` again (the local review's round 1 fix made that second press ask at once). Plan: `docs/plans/_archived/20261117-native-clear-cache/plan.md`.

Files: `crates/app/src/main.rs`.

#### TODO

- [ ] Drain the held clear at those idle points too. Done when a held clear asks by itself after a trash undo into another folder and after a failed rename, with a test of the pure part where possible.

### App: real-device check of the strip position in the status bar

`native-status-position` moved the GPUI app's strip position (`3 / 120`) from the meta pane to the status bar's right region, as its first item, left of the progress circle, `Sort ▾` and `Filter ▾`. While `FilterState::is_active` it appends the listed count (`3 / 80 (120)`), and with no shown file it shows nothing. The pure format is `pager::position`. CI, the `pager.rs` unit tests and `mise run ci:local` cover the format and the build. The window was never run, so the placement was never seen on screen: the step was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261117-native-status-position/plan.md`.

Files: `crates/app/src/main.rs` (`status_row`, the meta pane's name `div`), `crates/app/src/pager.rs` (`position`).

#### TODO

- [ ] On Windows, with a long status text (for example an error), expect the position not to be clipped by the right region's `overflow_hidden`. Expect the `Sort ▾` and `Filter ▾` popovers to still anchor to their buttons.

### App: real-device checks of the always-visible scrollbars

#### Background

The `native-scrollbars-always` feature made `theme::apply` set the GPUI app's theme `scrollbar_mode` to `ScrollbarMode::Always` (in `theme.rs`'s `finish`). The folder tree's bar, the strip's `StripBar`, the meta pane's bar and the dialog bodies' bars now show at rest and take a press without scrolling first. The mode is global, so the settings modal's lists, the `Sort ▾` / `Filter ▾` popovers, `PopupMenu` and `Select` lists show their bar when they overflow too. A unit test on a plain `Theme` and `mise run ci:local` cover that the mode is `Always` after a vendored config. Nobody has watched the running window: the bars' look and drag were never exercised. gpui-base 0.9.1 still skips a bar whose content fits, so a short folder should show no bar. Plan: `docs/plans/_archived/20261117-native-scrollbars-always/plan.md`. The `native-scrollbar-gutter` feature then reserved the bars' 16 px track instead of leaving them over content: the tree row's right padding and the meta pane's body padding (`theme::SCROLLBAR_TRACK`) and `stripheight::GUTTER`, a 9 px band under the strip's cells. `mise run ci:local` and the unit tests (`SCROLLBAR_TRACK` against gpui-base's `Scrollbar::width()`, `stripheight::dragged` with the gutter) cover the arithmetic; the on-screen result was never exercised, and both features' checkboxes were ticked on the automated criteria only. Plan: `docs/plans/_archived/20261117-native-scrollbar-gutter/plan.md`.

Files: `crates/app/src/theme.rs` (`finish`, `apply`, `SCROLLBAR_TRACK`), `crates/app/src/main.rs` (`tree_pane`, `meta_pane`, `render`), `crates/app/src/stripbar.rs` (`StripBar`), `crates/app/src/stripheight.rs` (`GUTTER`, `dragged`), `crates/app/src/modal.rs` (`shell`), `docs/agents/app.md` ("The strip, the tree and the meta pane draw the theme's scrollbar").

#### TODO

- [ ] On Windows, open a folder with more files than the strip holds, expand the tree past the pane's height, show a file whose meta pane overflows, and open a dialog whose body overflows (e.g. Import… on a large folder). Expect each of the four bars (tree, strip `StripBar`, meta pane, dialog body) to be visible at rest, without scrolling first.
- [ ] On Windows, drag each thumb and click each track without scrolling first. Expect the view to follow (the strip's range thumbnails load as on a wheel scroll).
- [ ] On Windows, where the content fits (a folder with few files, a collapsed tree), expect no bar to be drawn and presses to reach the strip cells and tree rows as before.
- [ ] On Windows, with a tree tall enough to overflow, check that no count badge or name lies under the thumb at rest, hovered or dragged, and measure the gap between the badge's right edge and the thumb (by the source the resting thumb paints 4-10 px from the pane's edge and the badge now ends 16 px from it; the planning screenshot showed the badge covered, which the numbers did not explain). Press a badge's right edge and expect the row to be selected, not taken by the track. A tree that does not overflow should look unchanged apart from 4 px more room right of the badges.
- [ ] On Windows, in the meta pane with a long file's metadata, expect the bar at the pane's right edge with no value under it, and the notes and errors under the body unmoved (still 12 px from the right edge). Open a long `Import…` or Move to Trash dialog and expect no body text under its bar.
- [ ] On Windows, with a folder long enough to overflow the strip, expect the bar to sit below the names with nothing under the thumb at rest, hovered and dragged, a burst's band included. A press on a name band should select the cell and a right press should open its menu.
- [ ] On Windows, with a short folder, expect an empty 9 px band under the cells and no jump in the preview's height when the folder changes. Drag the strip's top edge: it should follow the pointer and stop at `MIN`, `MAX` and the preview's minimum, and a 1:1 crop should be re-cut after a drag as before. In a short window, expect the `Sort ▾` / `Filter ▾` menus to still fit above the strip header.
- [ ] On Windows, look at the bar in Neutral dark, Neutral light and one vendored theme (e.g. Catppuccin Mocha), switching on the Appearance page and, in System mode, on an OS appearance change. Expect the bar's look to be acceptable in each.
- [ ] On macOS and Linux, repeat the checks above.

### App: real-device checks of the strip's header row with Sort and Filter (native-strip-header)

#### Background

`native-strip-header` moved `Sort ▾` and `Filter ▾` of the GPUI app (`crates/app`) from the status row to a new 32 px header row on the strip's top edge. The resize handle now sits on the header's top edge. The status row is pinned to 30 px (`STATUS_ROW_HEIGHT`), and the popover's height cap is `menu_max_height` less the strip pane's height. `stripheight::dragged` takes the pane's wanted height and takes `HEADER` off itself. `stripHeight` still means the cells' area height. CI, the `stripheight.rs` unit tests, the unchanged `companion.rs` / `pager.rs` tests and `mise run ci:local` cover the arithmetic and the build. The layout, the popover's placement and the drag were never seen in the running window. The step was ticked on the automated criteria only. macOS and Linux are covered by the CI build only. Plan: `docs/plans/_archived/20261117-native-strip-header/plan.md`.

Files: `crates/app/src/main.rs` (`strip_header`, `status_row`, `menu_button`, `change_panels`), `crates/app/src/stripheight.rs` (`HEADER`, `dragged`), `docs/agents/app.md`.

Run each check with `--settings <fresh test store>`, `--index <test index>` and `--log <test file>`, never the real store.

#### TODO

- [ ] On Windows, run `mise run app:dev` on a folder. Expect the header on the strip's top edge with `Sort ▾` and `Filter ▾` at its right end. Each opens its menu upward from its button, a second press closes it, a press outside dismisses it and `escape` closes it. The filter button is lit while a filter is active (its border goes to `input` while hovered, a known `Button` hover limit), and the sort button is dimmed in a view-only (JPEG-only) folder, where it does not hover and does not open.
- [ ] On Windows, with a long filter menu (EXIF sections), the strip at its tallest and a short window, expect the menu to stay inside the window, to scroll, and its button to still toggle it (the popover is not snapped over the button). Expect the popover not to be clipped by the header.
- [ ] On Windows, check the handle on the header's top edge: the resize cursor, the 1 px line, a drag down to `MIN` and up to `MAX`, and the preview stopping at `PREVIEW_MIN`. Expect a press on a button not to be taken by the handle's top 5 px band.
- [ ] On Windows, at a stored `stripHeight` of 128, expect the cell box to be still 144 x 96.
- [ ] On Windows, press, right-click and wheel-scroll on the header. Expect it neither to select a cell, open the strip's context menu nor scroll the strip.
- [ ] On Windows, press `toggleStrip`. Expect the header to hide with the strip and an open menu to close. Expect the status row to keep its height (no jump when the progress circle appears or goes) and to show `n / shown (all)` while a filter is active.
- [ ] On Windows, re-measure the preview canvas in the default 1600 x 1000 window (expected 1160 x 810, 32 px shorter than the old 842), the status row's height and the header's height. Replace the "inferred / not re-measured" notes in `docs/agents/app.md` (the preview canvas "inferred 32 px shorter", the pre-header 1160 x 842 measurements, the status row height "inferred from the sources") with the measured values.
- [ ] On macOS and Linux, repeat the header, popover, handle, drag and `toggleStrip` checks above. CI only builds these platforms.

### App: real-device checks of the packages the Package workflow builds (gpui-packaging)

The `gpui-packaging` feature added the manual `Package` workflow and `tools/package/` scripts. CI covers building, `hdiutil verify` and `codesign --verify` of the DMGs, the silent install, upgrade and uninstall of the setup `.exe` on an elevated runner, `install.sh` under a throwaway `HOME` (including the wrong-digest refusal), and `apt` installing and removing the `.deb`. Never exercised: the local mise package tasks on a real machine, a launch from any package, Gatekeeper's first-launch prompt, the absence of a UAC prompt for a standard Windows user (the hosted runner is elevated), the Start menu entry, the desktop menu entry and window icon matching on Wayland and X11, and the Vulkan-less GL fallback. Plan: `docs/plans/_archived/20261117-gpui-packaging/plan.md`.

Files: `mise.toml`, `tools/package/macos.sh`, `tools/package/windows.sh`, `tools/package/linux.sh`, `crates/app/packaging/windows/handpic.iss`, `crates/app/packaging/linux/install.sh`, `crates/app/packaging/linux/control`.

#### TODO

- [ ] On macOS, run `mise run app:package:macos`. Expect `crates/app/target/release/Handpic-0.0.0-macos-<arch>.dmg`. Open it, drag `Handpic.app` to `Applications` and start it. Gatekeeper asks for the ad hoc signed app's first launch to be allowed under System Settings, Privacy & Security. Expect the app to start.
- [ ] On Windows, install Inno Setup 6 (`winget install JRSoftware.InnoSetup`), then run `mise run app:package:windows`. Expect `crates/app/target/release/Handpic-0.0.0-windows-x64-setup.exe` and `Handpic-0.0.0-windows-x64.zip`. As a standard (non-admin) user, run the setup. Expect no UAC prompt, the app under `%LOCALAPPDATA%\Programs\Handpic`, and a Start menu entry `Handpic` that starts it. Run the setup again silently (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`) and expect no prompt. Uninstall from Settings, Apps.
- [ ] On Linux x86_64 with `dpkg-dev` and the GPUI apt list installed, run `mise run app:package:linux`. Expect `crates/app/target/release/Handpic-0.0.0-linux-x64.tar.gz`, `install.sh` and `handpic_0.0.0_amd64.deb`. Run `sh install.sh <tarball> <its sha256sum>`, then start Handpic from the desktop menu on a Wayland and an X11 session. Expect the window to show the Handpic icon (matched by `dev.minodisk.handpic`). Install the deb with `sudo apt install ./handpic_0.0.0_amd64.deb`, start it from the menu, and remove it with `sudo apt remove handpic`. If a machine without a Vulkan driver is at hand, check that the app still starts (wgpu's GL fallback through `libEGL`).

### App: decide the Linux deb's `Conflicts` / `Replaces: riffle` and whether it depends on `libegl1`

The app deb is `Package: handpic`. The shipped Tauri deb (v1.10.0) is `Package: riffle`, so both can be installed side by side, and the app deb does not upgrade it in place. The removed Tauri app's `tauri.conf.json` on `main` had `linux.deb.replaces` / `conflicts` `["riffle"]`, never released. Separately, `tools/package/linux.sh` adds `libvulkan1` and `libwayland-client0` to `dpkg-shlibdeps`'s result but not `libegl1`, which wgpu's GL fallback opens when no Vulkan adapter is found. A desktop normally has it, but a machine without Vulkan would need it. Plan: `docs/plans/_archived/20261117-gpui-packaging/plan.md` ("The deb does not upgrade a Riffle deb in place" risk and Step 3's `Depends` measurement).

Files: `crates/app/packaging/linux/control`, `tools/package/linux.sh`.

#### TODO

- [ ] Decide whether `control` gets `Conflicts` / `Replaces: riffle`, after the real-device deb check above. Done when the choice is in the template or `app.md` records why not.
- [ ] Decide whether `Depends` gets `libegl1`, or `libvulkan1 | libegl1`, after the Vulkan-less machine check. Done when `linux.sh` reflects the choice or the guide records why not.

### App: real-device checks of the title bar's smaller text and centered folder name (native-title-bar-folder-name)

#### Background

The `native-title-bar-folder-name` feature drew "Handpic" in the app's title bar at `text_sm` (14 px) instead of the inherited 16 px. It also draws the open folder's name (`context::folder_title`: the last path component, else the shown path for a root such as `C:\` or `/`) centered across the whole window width. The name is an `absolute().inset_0()` overlay `div` over `TitleBar`, padded `TITLE_NAME_MARGIN` (160 px) a side and truncated with an ellipsis. CI and the unit tests cover the build on all three platforms and `folder_title` in `context.rs`. Nobody has looked at the bar running. Never exercised: the text size, the name's centering and truncation in a narrow window, whether 160 px a side of `TITLE_NAME_MARGIN` is right (it was estimated from the control widths, not tuned against a running window; on macOS the 80 px traffic-light padding leaves only about 4 px between the name's edge and "Handpic"), and whether a press on the name still reaches the bar's `WindowControlArea::Drag` row. That last point rests on the source of `gpui-pre` 0.3.12, where a listener-free `div` inserts no hitbox; it was never seen running. The Step 1 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261118-native-title-bar-folder-name/plan.md`.

Files: `crates/app/src/main.rs` (the title bar in `impl Render for Riffle`, `TITLE_NAME_MARGIN`), `crates/app/src/context.rs` (`folder_title`).

#### TODO

- [ ] On Windows 11 (`mise run app:dev` or a debug build of `crates/app`), open a folder. Expect "Handpic" next to the icon to be visibly smaller than before (14 px) with the icon still 16 px. Expect the folder's name to be centered on the whole window width at the same size as "Handpic", and to disappear with no folder open. Then narrow the window: expect a long name to be truncated with an ellipsis before it reaches "Handpic" or the gear. If 160 px a side of `TITLE_NAME_MARGIN` looks wrong, tune it. Press and drag starting on the name: expect the window to move. Double-click on the name: expect the window to maximize. Expect the minimize / maximize / close controls and the gear to still take their clicks. If the drag over the name does not move the window, fall back to a third `TitleBar` child as the plan says. Also open another folder, rename the open folder and run `Rename by Capture Date…` on it: expect the name to follow.

### Engine: decide the fate of the engine code only the Tauri app reached

#### Background

The remove-tauri plan (`docs/plans/20261118-remove-tauri/plan.md`, Trade-offs) removed the Tauri app but left the engine and core code only it called, since the AI features are more likely to be wired into the app than deleted. Nothing in `crates/app` calls it today: `handpic_engine::ai` (the `aiEnabled` switch and the model download), `index::run_embed_scan` with the `embeddings` table, `Index::rank_rows` / `rank_dirs`, `handpic_core::rank`, `genre` and `embed`, and the seven-tool default mode of `mcp::Mcp` (`Mcp::new`; the app uses `with_all_tools`). The app refuses the matching MCP tools with `not_in_app` (the `recommended` sort, the Genre filter, `refit_ranking`, the AI switch). What this code needs, and what it got wrong, is in `docs/agents/shared-crates.md` ("The third pass").

Files: `crates/engine/src/ai.rs`, `crates/engine/src/index.rs`, `crates/engine/src/mcp.rs`, `crates/core/src/rank.rs`, `crates/core/src/genre.rs`, `crates/core/src/embed.rs`, `crates/app/src/companion.rs`.

#### TODO

- [ ] Decide, with the user, whether the app gets pass 3, the Genre filter and the ranking (and plan that), or whether this code goes. Done when either a plan wires it into `crates/app` or a change removes it with its tests and docs.

### Icon: the app icon still reads RIFFLE

#### Background

The artwork's edge print says `RIFFLE 60FPS CULLER`: `tools/macos/app-icon/gen.py` and its `icon.svg` draw it, and the rendered icons in `crates/app/icons/` (`source.png`, `icon.ico`, `icon.icns`, `64x64.png`, `128x128@2x.png`) carry it. The remove-tauri plan (`docs/plans/20261118-remove-tauri/plan.md`, Trade-offs) left it: `gen.py` needs macOS, and the step that derived `icon.ico` / `icon.icns` / the PNG sizes from `source.png` (`tauri icon`) left with the Tauri app, so no checked-in command derives them any more.

Files: `tools/macos/app-icon/gen.py`, `tools/macos/app-icon/icon.svg`, `crates/app/icons/`.

#### TODO

- [ ] On macOS, change the edge print to Handpic in `gen.py` / `icon.svg`, render `source.png`, and derive `icon.ico`, `icon.icns`, `64x64.png` and `128x128@2x.png` from it with a documented command (add one to the repository if none fits). Done when every icon in `crates/app/icons/` reads Handpic and the derivation is written down.

---

### App: the keymap's `compare` action is bound to `v` and listed, but does nothing

The keymap still has the `compare` action bound to `v`, and the `Keyboard Shortcuts` page lists it as `Compare frames`. The app does nothing on it (`key: compare is not handled yet` in the debug log). The remove-tauri plan's Step 5 left `v` out of `usage.md`'s key table. Plan: `docs/plans/_archived/20261118-remove-tauri/plan.md`.

Files: `crates/engine/src/shortcuts.rs`, `crates/app/src/settingsmodal.rs`, `crates/app/src/main.rs` (`key_down`), `docs/humans/usage.md`.

#### TODO

- [ ] Either hide unimplemented actions on the `Keyboard Shortcuts` page, or implement compare.

---

### App: real-device check of the screenshot tool's failed-capture path on Linux (native-qa)

#### Background

The `native-qa` feature added a `screenshot` MCP tool, listed only in a `--features screenshot` build of `crates/app`. Its Done-when says a failed capture (Linux's "not implemented for this platform", a lost device) is a tool error with a message, not a panic. The capture was exercised on Windows only. There was no Linux host, so the error path was reasoned about from the code, and no CI job builds or runs the feature on Linux. Plan: `docs/plans/_archived/20261118-native-qa/plan.md`. Files: `crates/app/src/main.rs` (`screenshot`), `crates/app/src/screenshot.rs` (`capture`).

#### TODO

- [ ] On Linux, build the app with `--features screenshot` (`cargo run --features screenshot -- --mcp-port <port>` with `mcpEnabled` on), call the `screenshot` tool through the MCP endpoint, and expect a tool error carrying `render_to_image`'s message ("not implemented for this platform"), the app still running and the window still drawing, not a panic or a bridge timeout.

---

### App: watch the macOS app CI job's cold-cache duration against the 30-minute timeout after the test-support dev-dependency (native-qa)

#### Background

`native-qa` Step 4 added `gpui-pre-platform` with `test-support` as a dev-dependency of `crates/app`, so the GPUI crates compile in a second variant (plain for the bundle step, `test-support` for clippy and tests). The macOS `app` job took 22m17s on PR #1165 (Step 4) against the job's `timeout-minutes: 30`. Plan: `docs/plans/_archived/20261118-native-qa/plan.md`. Files: `.github/workflows/ci.yml` (the `native` job), `crates/app/Cargo.toml` (the dev-dependency).

#### TODO

- [ ] Read the macOS `app` job's duration on later PRs, in particular a cold-cache run. If it nears 30 minutes, cache more or split the job, or raise the timeout, and record it in `docs/agents/app.md`'s CI entry.

---
