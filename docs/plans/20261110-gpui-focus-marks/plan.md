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

# Pass 2, the focus mark, the face overlays and the on-demand eye judgment in the GPUI app

## Purpose

This is a slice of milestone Step 7 of the GPUI port
(`../20261109-gpui-port/plan.md`). After port Step 5
(`../_archived/20261110-gpui-strip-index/plan.md`) the native app
(`crates/native`) opens a folder, runs pass 1 of the scan and pages previews
with a strip, but it never runs pass 2, so the index's `eye_focus`,
`sharpness`, `eyes_ear`, `pose_*` and `full_eye_sharp` columns stay empty, and
it has no way to show where the camera focused or what the analysis saw.

After this plan the native app:

- runs pass 2 (`index::run_faces_scan`) after pass 1 on the same scan thread,
  at the lower priority the backend sets, on-screen files first;
- shows on `f` the AF focus mark, the detected face boxes with their five
  landmark dots and labels, and the face mesh of the judged face, colored
  exactly as the Tauri app colors them (`crates/app/ui/src/focus.ts`
  `photoTier` / `afEyeState`, `facemesh.ts`);
- judges the eyes of the shown file on demand (`eyes_of`), a newer request
  superseding an older one;
- is shown by measurement not to page slower while pass 2 runs.

Out of scope: port Step 6 (the meta pane and its Analysis rows, judgments,
sidecars, the keymap), and the rest of port Step 7 (1:1 zoom, focus peaking,
filter, sort, the strip's face icon). Other sessions are working on Step 6 and
on the zoom / peaking part of Step 7 at the same time, and all of them edit
`crates/native/src/main.rs`: this plan keeps its changes there to wiring and
puts logic in new modules.

### What the investigation found (2026-11-10, `main` at `de249218`)

- `crates/native/src/scan.rs`: `Scan::start` spawns one `std::thread` that
  calls `index::run_scan` and sends `Event::Progress` / `Event::Done` (tagged
  with the scan id) into the view's one channel. `Scan::stop` sets the cancel
  flag and joins. `main.rs` builds the `ScanFocus` with
  `ScanFocus::with_slots(self.slots.clone())` and sets the hot list before
  `Scan::start`; `focus_scan` / `focus_scan_soon` update it only while
  `self.progress.is_some()`; `Event::Done` calls `finished`, which lets a
  deferred resync re-list (`resync.rs`).
- Pass 2 in the backend is already Tauri-free:
  `Index::faces_todo(dir) -> Vec<String>` (marks JPEG and failed rows done
  first), then
  `index::run_faces_scan(index, dir, paths, threads, focus, cancel, progress_interval, progress)`
  with `progress: Fn(usize, usize, Vec<FaceReady>)`. Its workers run at
  `Priority::Lowest` and take `focus`'s files first; a canceled file keeps its
  old `faces_extractor`, so the next scan resumes it. The Tauri app runs it on
  the same task as pass 1, with the same `focus`, `cancel` and
  `scan_threads()` (`crates/app/src/commands.rs` `run_faces_pass`,
  `start_scan`), and drains a deferred rescan only at `faces-done`.
- The stored values reach a client as `index::Focus` inside
  `IndexedFile.focus` (`Index::entry(path)` / `Index::entries(dir)`): the AF
  `point`, `manual_focus`, `eye_focus`, `candidate`
  (`riffle_core::candidate::FocusCandidate`) and the flattened `StoredEyes`
  (`eyes_ear`, `pose`, `eye_offset`, `edge_gap`, `full_eye_sharp`).
  `FaceReady` (pass 2's progress item) carries the same values but not the
  point. The native view holds no `IndexedFile` today.
- The on-demand paths live in `crates/app/src/commands.rs`, not in the
  backend: `read_faces(path) -> FacesResponse` (through
  `riffle_core::faces::detect_around`) and
  `read_eyes(path, current: impl Fn() -> bool) -> EyesResponse` (asks
  `current` before the detection and before the model and returns
  `superseded`), with `raw_only`, the private structs `FacesResponse`,
  `FaceBox`, `FacePoint`, `EyesResponse`, `EyesJudgment`, `EyesMesh`, and four
  tests (`faces_come_back_in_the_stored_preview_size`,
  `eyes_without_a_face_are_unknown_and_a_jpeg_is_refused`,
  `the_eyes_judgment_carries_the_pose_or_null`,
  `a_superseded_eyes_request_stops_before_the_detection`). The Tauri command
  stores the request id in `EyesRequests(Mutex<u64>)` and passes
  `|| latest == request`. Neither path touches the index or `ReadSlots`.
- The frontend rules to port, all pure and tested:
  `focus.ts` (`GOOD_EYE_FOCUS` 0.9, `GOOD_EYE_EAR` 0.25, `GOOD_MAX_YAW` 35,
  `GOOD_MAX_PITCH` 45, `MAX_EYE_OFFSET` 0.1, `MIN_EDGE_GAP` 0.02,
  `GOOD_FULL_EYE_SHARP` 0.162, `photoTier`, `afEyeState`, `focusMark`,
  `faceMarks`, `faceLabel`, `uprightPoint`, `faceLabelAt`,
  `FACE_LABEL_HEIGHT` 14, `FOCUS_MARK_COLORS` good `#3f3` / low `#999` /
  neutral `#fff`), `faces.ts` (`FaceCache`), `eyes.ts` (`EyesCache`: one
  judgment in flight, a generation bumped by `clear`, an id per request),
  `facemesh.ts` (`FACE_OUTLINE_EDGES`, `FACE_IRIS_EDGES`,
  `FACE_IRIS_CENTERS` 468 / 473, `meshEdges`, `meshPoints`, with the MediaPipe
  attribution comment).
- The drawing itself is in `crates/app/ui/src/main.ts`: `FOCUS_MARK_ARM` 8,
  `FOCUS_MARK_GAP` 4, `FACE_MARK_COLOR` `#3ff`, `FACE_MESH_COLOR` `#f3f`,
  `FACE_MARK_EYE_RADIUS` 2.5, `FACE_MESH_OUTLINE_WIDTH` 3,
  `FACE_MESH_LINE_WIDTH` 1.5; every shape is stroked twice, first
  `rgba(0, 0, 0, 0.8)` at width 4 (3 for the mesh), then the color at width 2
  (1.5 for the mesh); labels are 11 px text with a 3 px dark stroke. Face
  boxes, labels and the mesh are skipped on a view-only (JPEG) folder and
  while the shown bitmap is still the previous file's. The Tauri app clears
  both caches on a folder switch and on a refresh of the folder's entries.
- The native preview canvas paints an image that `orient::rotate_bgra`
  already turned upright, into `fit_centered(image, bounds)`. `Loaded` carries
  the stored size `px` and `orientation`; the view does not keep them, nor
  which file the shown image belongs to.
- GPUI at the pinned rev has `PathBuilder::stroke(width)` / `fill()` /
  `arc_to` and `Window::paint_path` / `paint_quad`; text on a canvas goes
  through `window.text_system().shape_line` (`docs/agents/native-app.md`).
  There is no text stroke.
- Existing numbers to compare against: the pager with the strip, warm index,
  no scan running, median p50 `keypressToPixels` 17.44 ms on ARW 1616x1080 and
  60.29 ms on RAF 4416x2944
  (`../_archived/20261110-gpui-strip-index/results.md`); during pass 1 the
  preview `read` was 0.96 ms p50 (`docs/agents/native-app.md`). The rule
  recorded there: compare against a run of the same session, alternating,
  because `frameWait` alone ranges over 6-10 ms across runs of one binary.
- `--size` / `--max` narrow only what the pager shows; the scan always gets
  the whole listing. A large folder opened with `--max 20` therefore pages 20
  files while pass 2 runs over all of it.
- Pass 2 costs roughly 7-10 ms per file at 24 threads in the CLI
  (`docs/humans/performance.md`), so it lasts well under a second on the
  20-file compare folders and some tens of seconds on a 2000-file folder.

## Steps

- [x] Step 1: Run pass 2 after pass 1 on the native scan thread
  - Done when:
    - `crates/native/src/scan.rs`: after `index::run_scan` returns and
      `Event::Done` is sent, the same thread, unless canceled, takes
      `Index::faces_todo(dir)` and runs `index::run_faces_scan` with the same
      `ScanFocus`, the same cancel flag, `scan_threads()` and
      `index::PROGRESS_INTERVAL`, as `run_faces_pass` in
      `crates/app/src/commands.rs` does. Two new events tagged with the scan
      id carry it to the view: the progress (`done`, `total`, the
      `Vec<FaceReady>`) and the end (`ScanSummary`). The end event is sent
      also when the pass was canceled, had nothing to do or `faces_todo`
      failed (logged), so the view always learns the scan is over.
    - A folder whose pass 1 has nothing to do but whose rows are not at
      `FACES_VERSION` still gets pass 2 (the scan thread starts whenever an
      index is open, not only when pass 1's todo list is non-empty; check how
      `opened` decides today and keep a no-op scan cheap).
    - `main.rs` (wiring only): the status line shows `analyzing done/total`
      while pass 2 runs, after `scanning done/total`; the hot list keeps
      reaching the scan through both passes (`focus_scan` /
      `focus_scan_soon` gate on "a pass is running", not on pass 1's
      progress alone); `finished` (the deferred resync) runs at the end of
      pass 2, not of pass 1, as the Tauri frontend drains its deferred rescan
      at `faces-done`; `Scan::stop` on a new open or a re-listing cancels and
      joins pass 2 as it does pass 1.
    - Hand check on a copy of an ARW folder under
      `D:\Photos\tests\<date>-gpui-pass2\` with a fresh `--index` file in
      that folder: stderr shows `scan extract: ...` then
      `scan faces: dir=... files=N done=N ... canceled=false`; a second open
      shows no `scan faces` work (`files=0` or no pass); quitting or
      switching folder mid-pass and reopening resumes the remaining files;
      with the strip scrolled far from the start, the first `analyzing`
      batches are the on-screen files (log the first `FaceReady` paths at
      `debug` or check through the status line; record how in
      `learnings.md`).
    - A quick sanity run (`--auto 20` on the ARW compare folder inside a
      larger folder, see Step 6 for the method) is noted in `learnings.md`;
      it is not the gate.
    - `mise run ci` and `mise run native:check` pass; `crates/app` and
      `crates/backend` have no diff. `docs/agents/native-app.md` gains the
      rule that pass 2 shares pass 1's thread, focus handle and cancel flag
      and why the resync waits for its end.
  - Implementation approach:
    - Keep it on the existing `std::thread` of `Scan::start`; do not add a
      second thread or a second cancel flag. `run_faces_scan` sets
      `Priority::Lowest` itself; pass nothing for it.
    - The callback only sends into the channel (the rule in
      `native-app.md`).
    - A re-listing during pass 2 cancels it and the next scan resumes the
      unwritten rows, so deferring the resync is about not restarting, not
      about correctness; keep `Resync`'s API and only move the `finished`
      call.
    - Do not store the `FaceReady` values in the view here; Step 2 decides
      how the mark reads them. The strip does not change.
- [x] Step 2: The `f` focus mark, colored by the stored analysis
  - Done when:
    - A new pure module (suggested `crates/native/src/mark.rs`) ports from
      `focus.ts`: the seven cut constants with their values, `photo_tier`,
      `af_eye_state` (Good / Neutral / Low) over `index::Focus`,
      `focus_mark` (the point and the optional AF frame rectangle on the
      unrotated drawing centered on the origin, `None` without a point or on
      a manual-focus shot), `upright_point`, and the three colors `#3f3` /
      `#999` / `#fff`. Its unit tests are the cases of `focus.test.ts` for
      those functions, ported one for one (list any case that cannot be
      ported in the PR).
    - A new module (suggested `crates/native/src/overlay.rs`) paints the
      mark over the preview: the four crosshair arms (arm 8, gap 4) and the
      AF frame rectangle when there is one, stroked first in
      `rgba(0, 0, 0, 0.8)` at 4 px and then in the state color at 2 px, in
      logical pixels, mapped from the unrotated coordinates through
      `upright_point` onto the bounds `fit_centered` gave the upright image
      (on orientation 6 / 8 the unrotated draw width and height are the
      fit's height and width).
    - `f` toggles the overlay (off at launch). The mark is drawn only when
      the shown image belongs to the current file, so a mark never sits on
      the previous file's picture.
    - The current file's `Focus` comes from the index through the read-only
      connection on `cx.background_spawn`, and is read again when the file
      becomes current, when a pass 1 progress message names it and when a
      pass 2 progress message names it, so the mark turns from white to
      green or gray as pass 2 reaches the file. A late answer for a file
      that is no longer current is dropped. Without an index there is no
      mark.
    - `main.rs` changes are wiring: the `"f"` arm of the key match, the
      fields the view needs (the toggle, the current `Focus`, the shown
      image's stored size, orientation and path), one call from the preview
      canvas's paint closure into `overlay.rs`, and the calls that trigger
      the read.
    - Hand check, same test folder and `--index` as Step 1, the Tauri app
      open on the same files for comparison (its own index; never share
      one): on a landscape frame, a portrait frame (orientation 6 or 8), a
      frame with an AF frame size (Sony `FocusFrameSize`), a manual-focus
      frame (no mark) and one frame of each state, the mark sits on the same
      spot and has the same color in both apps. Record the files used in
      `learnings.md`.
    - `mise run ci` and `mise run native:check` pass. `CLAUDE.md`'s
      `crates/native` sentence names the new modules.
  - Implementation approach:
    - Assumes Step 1 merged.
    - Default: read one row with `Index::entry(path)` for the current file
      only. Nothing in this slice needs the whole folder's values (the
      strip's icon, the filter and the sort are out of scope). If port
      Step 6 has merged a folder-wide entries store by then, read `focus`
      from it instead and patch it from `FaceReady` as `applyFaceReady`
      does; do not build a second store next to it.
    - The Good / Low rule must stay Rust's `candidate` for Low (the
      `FocusCandidate` the index derives) and the ported cuts for Good; do
      not re-derive the candidate threshold.
    - Hardcode `"f"` like the arrow keys; the keymap (`shortcuts.rs`'s
      `focus` action) is port Step 6's.
    - Painting: `PathBuilder::stroke(px(w))` + `window.paint_path`. Check
      that a stroked open path gets square caps or extend each arm by half
      the stroke width so the arm ends keep the 1 px dark outline the Tauri
      mark has; record what GPUI does in `native-app.md`.
    - The zoom / peaking session edits the same paint closure: keep the
      overlay call a single line after the image is painted, and take the
      image bounds as an argument so a later 1:1 view can pass its own or
      skip the call.
- [x] Step 3: Move the on-demand face detection and eye judgment into `riffle-backend`
  - Done when:
    - A new backend module (suggested `crates/backend/src/faces.rs`; name to
      confirm, it must not be confused with `riffle_core::faces`) holds
      `read_faces(path)` and `read_eyes(path, current: impl Fn() -> bool)`
      with `raw_only`, their response structs (public fields, still
      `serde::Serialize` with the same field names, so the Tauri payloads do
      not change) and the four tests listed in the investigation, moved with
      whatever test helpers they need (`#[cfg(test)]` does not cross the
      crate boundary: copy a helper the app's remaining tests still use).
    - `crates/app/src/commands.rs` keeps the `faces_of` and `eyes_of`
      commands and `EyesRequests`, now calling the backend functions; the
      app's behavior and the frontend are unchanged.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md`'s
      `crates/backend` sentence names the module.
  - Implementation approach:
    - Independent of Steps 1-2; it can merge in any order before Step 4.
    - A pure move: no signature change beyond visibility, no new behavior.
      The backend names no event and no `AppHandle`
      (`docs/agents/native-app.md`), which these functions already satisfy.
    - The alternative (reimplement in `crates/native`, no `crates/app` diff)
      was considered and not chosen; see "Trade-offs and risks".
- [x] Step 4: Face boxes with their landmark dots and labels
  - Done when:
    - Pure code in new or Step 2's modules ports `FaceCache` (`faces.ts`:
      never detect a path twice, a generation that drops answers issued
      before a clear, an answer kept by path even after paging away),
      `faceMarks`, `faceLabel` (score to two decimals, the box size and the
      eye distance in stored preview pixels, width and height swapped on a
      quarter turn) and `faceLabelAt` with `FACE_LABEL_HEIGHT` 14, with the
      cases of `faces.test.ts` and of `focus.test.ts` for those functions.
    - With `f` on, a RAW file whose image is shown and whose faces are not
      cached is detected once through the backend's `read_faces` on
      `cx.background_spawn` (wrapped like `load` in `guarded`); a failure is
      logged and cached as no faces. The answer repaints only when its file
      is still current.
    - The overlay draws, per face, the box and a dot of radius 2.5 on each
      of the five landmarks in `#3ff` over the same dark outline as the
      mark, and the label upright at `faceLabelAt`'s position in 11 px text
      in `#3ff`, readable over a bright face (see the approach).
    - Nothing is detected or drawn on a JPEG (view-only) folder or while
      `f` is off. The cache is cleared when another folder is opened and
      when a re-listing happens (the file may have changed), as the Tauri
      app clears it.
    - Hand check against the Tauri app on the Step 2 files plus a frame
      with several faces and a portrait frame: same boxes, dots and label
      text.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` updated
      for any new module.
  - Implementation approach:
    - Assumes Steps 2 and 3 merged.
    - The detection runs at normal priority on GPUI's background executor,
      next to the preview decode; it does not take `ReadSlots` (the Tauri
      path does not either). Step 6 measures a run with `f` on.
    - GPUI cannot stroke text. Decide between a dark
      `rgba(0, 0, 0, 0.8)` quad behind the label and painting the text in
      black at small offsets under the colored text; keep the one that reads
      on a bright face and record it in `native-app.md`.
    - The dots: a filled circle with a dark ring, by `paint_quad` with
      corner radii or a `PathBuilder` arc; whichever is simpler at this rev.
- [x] Step 5: The on-demand eye judgment and the face mesh overlay
  - Done when:
    - A pure port of `EyesCache` (`eyes.ts`) with its test cases: a cached
      path or any judgment in flight gives no ticket; a ticket carries an id
      and a generation; an answer from before a clear is dropped and leaves
      the newer judgment in flight; a superseded answer frees the slot and
      caches nothing; an unknown result (no face, a failure) is cached so it
      is not judged again.
    - A pure port of `facemesh.ts`: `FACE_OUTLINE_EDGES` (149 edges),
      `FACE_IRIS_EDGES` (8), `FACE_IRIS_CENTERS`, `mesh_edges` (no irises on
      closed eyes) and `mesh_points`, with the MediaPipe attribution comment
      carried over verbatim and the cases of `facemesh.test.ts`.
    - With `f` on, the current RAW file's eyes are judged through the
      backend's `read_eyes` on `cx.background_spawn`, one at a time; when it
      settles, the file current by then is asked for, so a held arrow key
      never queues one judgment per file passed. The latest request id lives
      in a shared atomic the `current` closure reads, so a request issued
      after a clear makes the still-running older one stop at its next stage
      and come back `superseded`. A unit test covers that rule on the pure
      part (two tickets across a clear: the older closure reports not
      current).
    - The overlay draws the mesh of the judged face: the edges stroked in
      `rgba(0, 0, 0, 0.8)` at 3 px and then `#f3f` at 1.5 px, and, for open
      eyes, the two iris centers as dots of radius 2.5 in `#f3f`. Same gates
      and the same cache clearing as the face boxes.
    - One `eyes` timing line per judgment goes to the log (`read`,
      `decode`, `detect`, `model`, and whether it was superseded), as the
      Tauri frontend's `eyes total=...` debug line.
    - Hand check against the Tauri app: the mesh sits on the same face with
      the same lines on an open-eyed and a closed-eyed frame and on a
      portrait frame; holding `→` with `f` on logs judgments only for files
      the paging stopped on or passed slowly, and a folder switch during a
      judgment logs it as superseded.
    - `mise run ci` and `mise run native:check` pass; `CLAUDE.md` and
      `native-app.md` updated.
  - Implementation approach:
    - Assumes Step 4 merged.
    - Judge only while `f` is on. In the Tauri app `requestEyes` runs on
      every shown file because the meta pane's `Eyes` row reads it; that row
      is out of scope here. See "Trade-offs and risks".
    - Do not write the judgment to the index; `eyes_of` touches neither the
      index nor the stored state.
- [ ] Step 6: Measure key-navigation latency while pass 2 runs, and record it
  - Done when:
    - **Before any timed run the agent stops and waits for the user's go
      signal**: other sessions measure on the same machine, and the runs
      need it otherwise idle. Building and preparing the test folders and
      index templates may happen before the signal; nothing timed does.
    - `results.md` in this folder holds a same-session A/B of one release
      binary (`cargo build --release` in `crates/native`, run directly):
      **A** = paging while pass 2 runs over the folder, **B** = paging the
      same files on an index where both passes are complete. Alternating
      B A B A B A after one throwaway run, the per-run p50 / p95 of every
      stage of the `page` line and of `keypressToPixels`, the medians, and
      the measurement conditions as `docs/agents/tauri-app.md` "Write a
      performance number with its measurement conditions" demands (commit,
      build, SIMD on, machine, drive, display, how warmth and idleness were
      established, the folder, its file count and how long pass 2 took in
      each A run).
    - Every timed press of an A run falls while pass 2 runs: a press after
      the run's `scan faces:` line is not counted, and a run with fewer than
      20 counted presses is redone with a larger folder or fewer presses.
    - **Gate** (see "Trade-offs and risks"): the median of the three runs'
      p50 `keypressToPixels`, A minus B, is at most +2.0 ms on ARW
      1616x1080 and +5.0 ms on RAF 4416x2944, the thresholds the port has
      used since its Step 3. p95 is recorded and discussed, not gated. The
      archived strip plan's 17.44 / 60.29 ms and `native-app.md`'s numbers
      are quoted next to B for context only.
    - Not gated, recorded: one A/B pair with `f` on (the on-demand
      detection and judgment running per page), and the `read` stage during
      pass 2 next to the 0.96 ms p50 measured during pass 1.
    - If the gate is missed, `results.md` names the stage that grew, and a
      fix PR lands before this step closes; the first candidate is fewer
      pass 2 threads in the native app (the precedent is the Tauri app's
      `embed_threads`, a quarter of the cores for the pass that runs while
      the user culls), re-measured the same way.
    - `docs/agents/native-app.md` gains a "Timings" entry with the verdict
      and the method's rules; `docs/humans/performance.md`'s "Native preview
      (GPUI)" subsection and `performance.ja.md` gain the row;
      `../20261109-gpui-port/plan.md` Step 7 gains one line saying pass 2,
      the `f` overlays and `eyes_of` were done by this plan. `mise run ci`
      passes.
  - Implementation approach:
    - Assumes Steps 1-5 merged.
    - Test data lives under `D:\Photos\tests\<date>-gpui-pass2\`, each
      index a fresh `--index` file there; never the Tauri app's index or the
      default native one. No screenshots. Do not kill any process: a stale
      `riffle-native.exe` that locks the build output is renamed aside
      (`native-app.md`), and the user is asked to close the Tauri app.
    - The compare folders hold 20 files, which pass 2 finishes in under a
      second. Use a large folder holding the 20 compare files and open it
      with `--size` / `--max 20`: the pager wraps over the 20 while the scan
      covers the whole folder. Size the folder so pass 2 outlasts the
      presses (at roughly 10 ms per file, 60 presses at 1003 ms need several
      thousand files; fewer presses per run and more runs is the fallback).
    - To start an A run at "pass 1 done, pass 2 not": decide between a
      template index whose rows are reset with
      `UPDATE files SET faces_extractor = 0` (Python's `sqlite3`, an ad-hoc
      script in the scratch directory, a fresh copy of the template per run;
      no code change) and a measurement-only flag that skips pass 2. Prefer
      the template; check on the first run that `scan faces:` reports the
      whole folder.
    - The idle gate of the archived plan (system CPU at most 15 % during a
      run) cannot hold for A, whose own pass 2 loads the cores. Gate B runs
      as before; for A runs require the sample before the run to be at most
      15 % and no compiler, `riffle-app` or second `riffle-native` present
      during it.
    - `page` lines go to stdout and the backend's log to stderr; capture
      both into one file and confirm on the first run that their order is
      reliable enough to tell which presses preceded `scan faces:`; if it is
      not, record how the cut was made instead.
    - The whole-folder scan reads files the pager never shows; keep the
      copies warm for both A and B (one throwaway run) so the comparison is
      about CPU contention, which is what pass 2 adds, and say so in the
      conditions.

## Trade-offs and risks

- **Where `read_faces` / `read_eyes` live (Step 3).** Chosen: move them from
  `crates/app/src/commands.rs` into `riffle-backend` with their tests, as port
  Step 4 moved the other Tauri-free code. One implementation, its tests kept,
  and port Step 6's meta pane can use the same `read_eyes`. Cost: a diff in
  `commands.rs`, a file other Tauri-side work also edits, and `mise run ci`
  must stay green for the app. Alternative not taken: reimplement both in
  `crates/native` (about 150 lines over `riffle_core::{reader, faces, eyes,
  decode}`), with no `crates/app` diff, at the cost of two copies until the
  switch-over removes the Tauri app.
- **How the mark gets a file's `Focus` (Step 2).** Default: one
  `Index::entry` read for the current file, refreshed when a scan message
  names it. It is the smallest thing that satisfies this slice. A folder-wide
  store (`Index::entries` once, patched from `FaceReady`) is what the strip's
  face icon, the filter and the sort will need later, and what port Step 6's
  meta pane may introduce in parallel; building it here risks two competing
  stores in `main.rs`. If Step 6 merges first, Step 2 should read from its
  store instead.
- **When the eyes are judged (Step 5).** Chosen: only while `f` is on, since
  the mesh overlay is this slice's only reader. The Tauri app judges every
  shown file (for the meta pane's `Eyes` row). Judging always would cost
  80-160 ms of background CPU per file stopped on with nothing to show for it
  until Step 6's Analysis rows exist; judging only with `f` on means the
  session that ports those rows has to widen the trigger.
- **The gate's thresholds (Step 6).** "Does not worsen" needs a number. This
  plan reuses the port's +2.0 ms (ARW) / +5.0 ms (RAF) on the median p50,
  same-session A/B. Risk: pass 2 loads `cores - 2` threads, and although they
  run at the lowest priority, an all-core load can lower the clock for the
  decode thread; a RAF's 46 ms decode has more to lose than the 5 ms
  allowance if that effect is large. The plan's answer is the fix PR (fewer
  pass 2 threads) and a re-measure, which trades a longer pass 2 for paging.
- **A RAF folder large enough for pass 2 to outlast the presses may not
  exist.** The 20-file RAF compare folder inside a large folder of other
  RAFs is needed; if there are not enough RAF files on the machine, the
  options are to gate on ARW only and record RAF as not measured, or to pad
  the folder with copies of the same RAFs under other names (pass 2 does the
  same work per copy). The implementation records which it did.
- **Not ported here, although it reads the same state:** the strip's face
  icon (`stripState`), the `Face focus` filter and its help text
  (`AF_EYE_HELP`), `eyesOpenness` and the meta pane rows. `mark.rs` should
  not grow them speculatively.
- **`main.rs` is edited by three sessions.** Each step limits itself to
  fields, one key arm, one paint call and event arms, but the `Riffle`
  struct, `scanned` and the preview canvas closure are shared ground; expect
  rebases, and re-run the hand checks after a rebase that touched the paint
  closure.
- **Label rendering will not be pixel-identical.** GPUI has no text stroke,
  so the label's halo is an approximation; the colors, positions and text are
  the same as the Tauri app's.
- **No pass 3 state.** The native scan still has no `pending` / `preparing`
  / embedding states; pass 2 only extends the one running scan. Port Step 9
  adds the rest.

## Progress

- (2026-11-11) Step 1 complete
- (2026-11-11) Step 2 complete
- (2026-11-11) Step 3 complete
- (2026-11-11) Step 4 complete
