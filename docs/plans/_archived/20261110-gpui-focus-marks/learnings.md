# Learnings

## Step 1: pass 2 on the native scan thread

- `opened` already started the scan thread whenever the index was open and
  `prepare` succeeded, even with an empty pass 1 todo list; `run_scan` over no
  files returns in 1 ms and `faces_todo` is one query, so a no-op scan stays
  cheap and no change to `opened`'s condition was needed.
- `main.rs` keeps pass 1's `progress` and adds `analyzing`; pass 1's `Done`
  sets `analyzing = Some((0, 0))` so `scanning()` (the gate of `focus_scan` /
  `focus_scan_soon`) has no gap between the two passes. The status line can
  show `analyzing 0/0` for the few ms until the first pass 2 message.
- `FacesProgress.ready` and `FacesDone.summary` are not read yet, so clippy's
  `-D warnings` failed on dead code; each carries an `#[expect(dead_code)]`
  with its reason (Step 2 reads `ready`; the backend's `scan faces:` line
  already reports the summary). Step 2 removes the one on `ready`.
- The logger prints at `Info`, so a `debug` line for the hot-list check would
  not show. `scan.rs` logs one `scan faces first: <names>` info line per pass:
  the first non-empty `FaceReady` batch (`run_faces_scan` fills `ready` only
  when a 10-row batch is written, so the very first progress call is often
  empty).
- Hand check (release build, `D:\Photos\tests\20261110-gpui-pass2\`, 300
  ARW copied from the strip plan's `ARW-scan`, a fresh `index.sqlite` there,
  driven with `--auto N`, which quits after `N` presses):
  - quitting mid-pass 1 and reopening resumed pass 1 (`todo=90`), then
    `scan faces first:` and a quit mid-pass 2 left 80 rows at
    `FACES_VERSION`;
  - with `folders.last_viewed` set by hand to file 200 (`_DSC1816`), the
    reopen ran pass 1 with `todo=0` and still ran pass 2, whose first batch was
    `_DSC1802`-`_DSC1810` plus one listing-order file (`_DSC1687`), the
    strip's range around the current file; the next reopen resumed the
    remaining 130 (`scan faces: files=130 done=130 canceled=false in 916ms`);
  - a further open logged `scan faces: files=0`.
  - Not checked: switching folder mid-pass from the UI and the status line's
    text (no screenshots); both follow from `Scan::stop` and `status()`.
- The quick `--auto 20` sanity run was skipped: it is a timed run and Step 6
  waits for the user's go signal.

## Step 2: the `f` focus mark

- No folder-wide entries store existed in `crates/native` on `main` at
  `f14e49ac` (Step 6's #973 moved the sidecar pass and the pane metadata into
  the backend but added no store to the view), so the mark reads one row with
  `Index::entry` on the read-only connection, as the plan's default.
- `index::Focus` has no `PartialEq`, so `overlay::Mark` cannot derive it; the
  tests compare its fields.
- `photo_tier` returns `bool`: `focus.ts`'s `PhotoTier` has the one value
  `good`, so `Option<PhotoTier>` would be a one-variant enum.
- Not ported from `focus.test.ts` (out of this step's scope, listed for the
  PR): `FOCUS_MARK_ICONS`, `stripState`, `AF_EYE_HELP`, `eyesOpenness`,
  `applyFaceReady`, `applySharpnessReady`, and `faceMarks` / `faceLabel` /
  `faceLabelAt` (Step 4). Every case of the ported functions was ported.
- The focus is read only while `f` is on (a toggle on reads at once), so
  paging with the overlay off does no extra index read. Reads carry a ticket
  so an older answer cannot overwrite a newer one for the same file.
- `PathBuilder::stroke` gives butt caps and lyon's `LineCap` is not
  re-exported; the arms are lengthened by half the stroke width instead (see
  `native-app.md`).
- The plan listed the shown image's stored size among the view's fields; the
  mark does not need it (the draw size comes from the fit bounds, the scale
  from the sensor size), so it is left for Step 4's face boxes.
- Python's text mode on Windows writes CRLF; edit files with
  `newline=''` (or strip `\r` afterwards), else `cargo fmt --check` and the
  diff show every line changed.
- Hand check (debug build with a temporary `info` line in `overlay::paint`
  giving the mark's window coordinates and state, removed before the
  commit; overlay temporarily on at launch, `--auto 7`) on
  `D:\Photos\tests\20261110-gpui-pass2\marks\` with a fresh
  `--index ..\marks-index.sqlite`. Files and what the log showed:
  - `ILCE-1M2_sony_a1_ii_03.arw` (orientation 8, AF frame 702x703): the
    first read came back `None` (pass 1 not there yet), the pass 1 message
    re-read it, then Neutral (no face); the point at (808.30, 268.91) on a
    569x852 fit matches the hand computation through `upright_point`.
  - `ILCE-7M4_DSC07652.ARW` (orientation 6, AF frame 205x205): Neutral,
    re-read when pass 2's batch named it.
  - `ILCE-6300_Free_raws_by_signatureeditsco_DSC03068.arw` (orientation 8,
    no frame): Neutral, a candidate with closed eyes (EAR 0.036).
  - `ILCE-6400_DSC01340.ARW` (orientation 6, no frame): Neutral, a candidate
    turned past the yaw cut (-36.2).
  - `ILCE-6400_DSC00405.ARW` (manual focus): no mark.
  - `_DSC1616.ARW` (landscape, frame 350x351): Low (`eye_focus` 0.708).
  - `_DSC1758.ARW` (landscape, frame 350x351): Good.
  - `_DSC1760.ARW` (landscape, frame 832x740): Neutral (no face).

## Step 3: the on-demand faces and eyes in the backend

- The module is `crates/backend/src/ondemand.rs` (`riffle_backend::ondemand`),
  not `faces.rs`: `read_eyes` itself does `use riffle_core::{decode, eyes,
  faces}`, and a client importing both crates' `faces` would have to alias
  one. No other backend module (including #973's) uses that name.
- `raw_only` moved with them as `pub`, and the app's `set_rating` ("culling")
  and its `culling_and_face_detection_refuse_a_jpeg` test now import it from
  there; that test stays in the app because it guards the culling command.
- Besides the four tests the plan names, the ignored
  `times_eyes_of_on_real_files` and `a_preview_that_is_not_a_jpeg_is_a_faces_error`
  moved too, since they test only `read_eyes` / `read_faces`. That left the
  app's `arw_with_preview` unused, so it moved rather than being copied;
  `gradient_jpeg`, `temp_dir` and `remove_temp_dir` are copied (the app's
  remaining tests still use them).

## Step 4: face boxes, landmark dots and labels

- `FaceCache` is `crates/native/src/faces.rs` (the native crate has no other
  `faces`; it imports `riffle_core::faces` nowhere). It keeps
  `Arc<FacesResponse>` so the paint closure gets the faces without a clone:
  the backend's `FacesResponse` derives neither `Clone` nor `PartialEq`, and
  adding them would have been a backend diff for this alone. The tests compare
  face counts instead of whole values.
- `Marks::mark` no longer needs a read focus: it answers whenever the overlay
  is on and the shown image is the current file's, with `focus` and `faces`
  each optional, because the Tauri app draws the face boxes regardless of the
  AF mark (a manual-focus frame, `ILCE-6400_DSC00405.ARW`, has no mark but
  one face box). Step 2's tests now check `mark(..).focus` where they checked
  `mark(..)`.
- The JPEG (view-only) gate is `riffle_core::scan::is_raw_file` on the
  current path: `scan::list` gives a folder's RAW files or, without any, its
  JPEGs, so the file tells the folder, and `read_faces` refuses a JPEG anyway.
- Detection is asked for when the image is shown (`loaded`), when `f` turns
  on and after every listing (`opened`, which clears the cache first, so a
  re-listing re-detects the current file). Not on `go`, so a held arrow key
  does not detect the files it passes, as the Tauri app asks only from
  `draw()` once the bitmap is the current file's.
- The stored size the plan listed for Step 2 is not needed: the scale comes
  from `FacesResponse`'s own `width` / `height`, the preview the detection
  ran on.
- `face_label` formats the score with `{:.2}` on the `f32`; the Tauri label
  runs `toFixed(2)` on the JSON's shortest `f32` text read as an `f64`. The
  two can differ in the last digit only on a score sitting on a rounding tie
  (for example 0.125); not worth matching.
- Heredocs of a hundred-plus lines with backticks failed in the Bash tool
  (`unexpected EOF while looking for matching`); writing the patch script
  with the Write tool and running it worked. `overlay.rs` was checked out
  with CRLF although its attribute is `eol=lf`; normalize on read.
- Hand check (debug build, temporary `info` lines in `overlay::detect` and
  `paint_label` giving each detection's path and face count and each label's
  text, its computed position and the face box; the overlay temporarily on at
  launch, `--auto 7`; all removed before the commit) on
  `D:\Photos\tests\20261110-gpui-pass2\marks\` with a fresh
  `--index ..\step4-index.sqlite`. Each file was detected once per run:
  - `ILCE-1M2_sony_a1_ii_03.arw` (orientation 8): one face,
    `0.92  106×139  eyes 50`;
  - `ILCE-6300_Free_raws_by_signatureeditsco_DSC03068.arw` (orientation 8):
    one face, `0.92  291×388  eyes 137`;
  - `ILCE-6400_DSC01340.ARW` (orientation 6): one face,
    `0.90  199×244  eyes 62`;
  - `ILCE-6400_DSC00405.ARW` (landscape, manual focus): one face,
    `0.90  133×167  eyes 62`;
  - `_DSC1616.ARW`: four faces (0.73-0.75); `_DSC1758.ARW`: nine faces
    (0.65-0.91), the several-faces frame;
  - `ILCE-7M4_DSC07652.ARW` and `_DSC1760.ARW`: no face.
  The label positions on the orientation 6 and 8 frames match a hand
  computation through `upright_point` and `face_label_at` (for example
  `ILCE-6400_DSC01340`'s box at (-134.30, -134.77) 128.49x104.95 on an
  852x569.41 unrotated drawing puts the label at (29.8, -148.3)), and the
  quarter-turn labels give the upright box's width first.

## Step 5: the on-demand eye judgment and the face mesh

- `EyesCache` is `crates/native/src/eyes.rs` and `facemesh.ts` is
  `crates/native/src/facemesh.rs`. The edges are `[[usize; 2]; N]` rather than
  a flat list, so the counts (149, 8) are in the types and the tests still
  check them along with the index ranges and duplicates. The MediaPipe
  notice is carried over word for word, but as a plain `/* */` comment: Rust
  reads `/*!` as an inner doc comment, allowed only at the top of the module.
- The latest ticket's id is an `Arc<AtomicU64>` inside `EyesCache`;
  `current(ticket)` hands out the closure `read_eyes` asks. Besides a newer
  request (the plan's rule), `clear` stores `0`, so a folder switch or a
  re-listing stops the running judgment at once instead of only when the new
  folder's first image is shown and asks for its own judgment. The native
  app asks only once the image is shown (as for the faces), not on `go` as
  the Tauri `show()` does, so without this a switch would often let the old
  judgment run its model to the end.
- `EyesJudgment` derives neither `Clone` nor `Default`; the cache keeps
  `Option<Arc<EyesJudgment>>` per path, as `FaceCache` keeps
  `Arc<FacesResponse>`. `settle` takes `Option<Option<EyesJudgment>>`:
  `None` superseded, `Some(None)` unknown eyes (no face, a failure).
- `main.rs`'s `detect_faces` became `analyze_current` (faces and eyes, the
  same four call sites) and `Marks::clear_faces` became `clear`. The judgment's
  callback calls `analyze_current` once it settles, which asks for the file
  current by then.
- `paint` no longer returns early without faces: the mesh is drawn whenever
  a judgment exists, then the labels, as the Tauri `draw()` order (mark,
  boxes, mesh, labels).
- The `eyes` line is `info` (the logger prints `Info`), with the file name;
  the Tauri line's `ipc` has no counterpart.
- Hand check (debug build, temporary `info` lines in `overlay::paint` giving
  the face eyes' and the mesh's iris centers' window positions, the overlay
  temporarily on at launch, a temporary folder switch after the `--auto`
  presses; all removed before the commit) under
  `D:\Photos\tests\20261110-gpui-pass2\` with fresh indexes there:
  - `marks`, `--auto 7`: `ILCE-6300_..._DSC03068.arw` (orientation 8)
    came back `closed` with 149 edges and no dot, the iris centers at
    (783.8, 332.0) / (858.1, 327.2) against YuNet's eyes at (785.0, 332.1) /
    (857.1, 325.9); `_DSC1758.ARW` `open`, 157 edges, two dots,
    (888.9, 331.6) / (915.3, 339.8) against (884.5, 333.3) / (914.0, 335.0);
    `ILCE-6400_DSC01340.ARW` (orientation 6) `open`, the left iris at
    (849.7, 336.0) against YuNet's (846.0, 332.3). Files with no judged face
    (`ILCE-7M4_DSC07652`, `_DSC1760`, `_DSC1616`) logged `model` under 1 ms
    and drew no mesh. A judgment taking longer than the 1 s between presses
    (`ILCE-1M2_sony_a1_ii_03`, 2 s in the debug build) settled after the
    page turned and was not drawn.
  - `ARW` (300 files), `--auto 60` with the presses 30 ms apart: 7 `eyes`
    lines for the 60 files passed.
  - `marks` then `ARW` opened 250 ms after the second press: the judgment of
    `ILCE-6300_..._DSC03068.arw` logged `model=0.0ms superseded`, in both of
    two runs.

## Step 6: key-navigation latency while pass 2 runs

- The gate is missed: A - B is +9.44 ms on ARW and +88.86 ms on RAF
  (`results.md`). `decode` about doubles and `paint` grows 4-6x while pass 2's
  22 workers hold every logical processor near 100 % at `Priority::Lowest`.
  The step stays open until a fix PR (the plan's first candidate: fewer pass 2
  threads) is re-measured.
- The fix: pass 2 now runs on `faces_threads()`, a quarter of the cores (6 on
  24 logical), in both apps, mirroring the Tauri app's `embed_threads`;
  pass 1 keeps `scan_threads()` and `READ_SLOTS` stays 2. The Tauri app's
  `faces_threads` is a copy of `embed_threads`'s body rather than a call to it,
  so either can move on its own measurement.
- Second run at 6 pass 2 threads (`8c91bef2`): ARW A - B -1.70 ms (met),
  RAF +13.08 ms (missed; `decode` +8.61, `paint` +5.45). Pass 2 alone took
  198.7 s on 18,620 ARW and 146.7 s on 5,420 RAF (85.7 / 79.6 s at 22). This
  session's B was 3-5 ms slower than the first session's on ARW (`read` 2.5
  against 0.8 ms, `decode` 8.4 against 6.2), so A came out faster than B.
  Possible causes, not measured: an idle machine clocking down, or the paged
  files dropping out of the page cache without pass 2's hot-list reads. The
  step stays open until the plan decides on the RAF gap.
- The user then chose an eighth of the cores for pass 2 (3 on 24 logical) in
  both apps, to be re-measured the same way; the 6-thread binary and its runs
  are kept as `riffle-native-6t.exe` and `runs-6t\` in the test folder.
- Third run at 3 threads (`4aefeb14`) is invalid: `frameWait` climbed to a
  120-145 ms p50 in A and B alike, a sawtooth of presents at about 4 Hz,
  probably the display sleeping or the window occluded once the user left the
  machine idle. Rule for the next session: check that every run's
  `frameWait` p50 sits near 8-10 ms before reading the gate. The pre-frame
  stages did improve: RAF `decode` +2.73 ms, `paint` -0.88 ms. Pass 2 alone
  took 301.2 s (ARW) and 225.8 s (RAF).
- Writing a path into a heredoc through the Bash tool collapsed a doubled
  backslash to a single one, so the backslash before `$F` escaped it, the
  folder argument held a literal `$F`, and the app opened a path that did not
  exist. Use forward slashes in Windows paths in scripts.
- Rerun at 3 threads, once through with no redos (the user's call): ARW
  A - B -3.03 ms (met), RAF +5.08 ms (missed by 0.08; `paint` +2.85,
  `decode` +2.34). Most B runs had a load sample over 15 %, which can only
  slow B, and the ARW runs' `frameWait` p50 was 9.8-15.9 ms, so ARW carries
  vsync noise. One run (ARW B1) did not exit after `--auto`'s quit and was
  closed with `CloseMainWindow`. A watchdog (`watchdog.ps1`) then closed any
  run's window still open after 100 s; it never fired.
- Pausing pass 2's compute, not only its reads, while a RAF preview loads
  took RAF's A - B from +5.08 to +1.94 ms without slowing pass 2 (rows per run
  unchanged). The landed version adds `ReadSlots::wait_unpaused` and
  `scan::yield_to_pause` at pass 2's stage boundaries. The stage functions in
  `candidate.rs` take only `cancel`, so the file's slots reach them through a
  thread-local set per file (`PassSlots`) rather than a signature change. The
  native preview's `Paused` now travels in `Loaded` into `PaintHold`
  (`hold.rs`), which the canvas releases after painting the image.
- This worktree's `.git` still points at the old
  `C:/Users/daisu/ws/github.com/minodisk/riffle` path. The repo moved to
  `.../handpic`, so git runs here with an explicit `--git-dir` (the
  worktree's admin dir under `handpic/.git/worktrees/`) and `--work-tree=.`.
- Final run (`d7292b00`: 3 pass 2 threads, the pass yielding at its stage
  boundaries, the preview holding the pause through the paint) met the gate:
  ARW A - B -2.10 ms, RAF -0.30 ms, every `frameWait` p50 7.2-10.3 ms. A
  helper held `SetThreadExecutionState` with `ES_DISPLAY_REQUIRED` for the
  session, so the display could not sleep. Three B runs had one sample over
  15 % and were kept (no redos, the user's call).
- `--auto`'s quit, and closing the window, call `cx.quit()` without
  `Scan::stop`. A pass 2 still running then logs no `scan faces:` line, so the
  A runs' pass 2 progress came from each run's index copy (rows with
  `faces_extractor > 0`).
- Pass 2 runs about 4.6 ms per file on ARW and 15-16 ms on RAF at 22 threads
  (5420 RAF in 79.6-85.1 s, 18,620 ARW in 85.7-87.8 s). A 60-press run needs
  about 13k ARW or 4.5k RAF files to stay inside pass 2.
- Hardlinks (`os.link`) make a large test folder that costs no disk space and
  keeps every copy warm in the page cache. Name the padding to sort after the
  compare files (`zzNNN_`), so `--size` / `--max 20` page only the 20.
- Turning `f` on in an `--auto` run without a code change: post
  WM_KEYDOWN / WM_CHAR / WM_KEYUP for `F` to the process's `MainWindowHandle`
  (`sendf.ps1`). WScript.Shell `AppActivate` + `SendKeys` did not reach the app,
  and the keystroke probably went to whatever window had focus, so never use
  it.
- Closing a window by PID with `CloseMainWindow` is allowed; doing it for every
  `riffle-native` process was refused (other sessions run their own).
- Other sessions ran `mise run ci`, `cargo test` and their own `riffle-native`
  through the whole session despite the go signal. 17 of 25 B attempts and
  1 of 11 A attempts were discarded. A pre-run wait for two clean samples and
  no `mise run` / `cargo` / `rustc` / `link` / Vite+ process (`next.sh`,
  `busy.ps1`) helped but did not prevent load arriving mid-run.

## Deferred issues (todo candidates)

- Pending manual check (Step 2, Windows, the user's): the side-by-side of the
  `f` mark against the Tauri app could not be made here (no screenshots).
  Open `D:\Photos\tests\20261110-gpui-pass2\marks\` in the native app
  (`mise run native:dev -- <folder> --index <fresh file in that test folder>`)
  and in the Tauri app (its own index), press `f` in both and step through
  the eight files listed under Step 2 above. Expected: on each file the
  crosshair and the AF frame sit on the same spot of the picture, with the
  same color (bright green on `_DSC1758`, gray on `_DSC1616`, white on the
  rest), no mark on `ILCE-6400_DSC00405.ARW`, and the arm ends keep the dark
  outline. The step's checkbox was ticked on the automated criteria (unit
  tests and the logged coordinates).

- Pending manual check (Step 4, Windows, the user's): the side-by-side of the
  face boxes, landmark dots and labels against the Tauri app could not be
  made here (no screenshots). Open `D:\Photos\tests\20261110-gpui-pass2\marks\`
  in the native app (`mise run native:dev -- <folder> --index <fresh file in
  that test folder>`) and in the Tauri app (its own index), press `f` in both
  and step through the files listed under Step 4 above, at least
  `_DSC1758.ARW` (nine faces), `ILCE-6400_DSC01340.ARW` (orientation 6),
  `ILCE-1M2_sony_a1_ii_03.arw` (orientation 8) and `ILCE-6400_DSC00405.ARW`
  (a face box without an AF mark). Expected: the same boxes, five cyan dots
  per face and label text in both apps, each label upright above its box's
  top-left corner (inside the box at the image's top edge, pulled left at the
  right edge), and the labels readable over a bright face with the native
  app's offset-copy halo (`docs/agents/native-app.md`); if they do not read,
  switch `paint_label` to a dark quad behind the text. The step's checkbox was
  ticked on the automated criteria (unit tests and the logged positions).

- Pending manual check (Step 5, Windows, the user's): the side-by-side of the
  face mesh against the Tauri app could not be made here (no screenshots).
  Open `D:\Photos\tests\20261110-gpui-pass2\marks\` in the native app
  (`mise run native:dev -- <folder> --index <fresh file in that test folder>`)
  and in the Tauri app (its own index), press `f` in both and stop on
  `ILCE-6300_Free_raws_by_signatureeditsco_DSC03068.arw` (closed eyes,
  orientation 8), `_DSC1758.ARW` (open eyes) and `ILCE-6400_DSC01340.ARW`
  (portrait, orientation 6), waiting for the judgment (up to 2 s in a debug
  build). Expected: the magenta mesh (face oval, eyes, brows, nose, lips) on
  the same face with the same lines in both apps, over a dark outline; the
  iris rings and two iris dots on the open-eyed frames only. Also hold `→`
  with `f` on and check that the log's `eyes` lines name only files the paging
  stopped on or passed slowly. The step's checkbox was ticked on the automated
  criteria (unit tests, the logged positions and the `--auto` runs above).


- Stale scan state on a failed `prepare` (found while wiring Step 1). `list`
  takes and stops the previous `Scan`, but `opened` clears `progress` /
  `analyzing` only when it starts a new scan; when `Scan::prepare` fails
  (`(Some(Err(e)), _)`) after a re-listing that interrupted a scan, the status
  line keeps the stopped scan's `scanning` / `analyzing` text and `scanning()`
  stays true. Pre-existing for `progress`; `analyzing` follows the same path.
  Fix: clear both in that arm (or whenever `list` stops a scan).
  `crates/native/src/main.rs` (`list`, `opened`).

- `--auto`'s quit (and closing the window) does not stop the scan
  (`Riffle::quit` in `crates/native/src/main.rs` calls `cx.quit()` with no
  `Scan::stop`), so a quit mid-pass 2 logs no `scan faces:` summary and leaves
  the workers to die with the process. Found while measuring Step 6. Fix: stop
  (cancel and join) the scan in `quit` and on window close, so the summary is
  logged and the index is closed cleanly.
