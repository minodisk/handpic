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


- Stale scan state on a failed `prepare` (found while wiring Step 1). `list`
  takes and stops the previous `Scan`, but `opened` clears `progress` /
  `analyzing` only when it starts a new scan; when `Scan::prepare` fails
  (`(Some(Err(e)), _)`) after a re-listing that interrupted a scan, the status
  line keeps the stopped scan's `scanning` / `analyzing` text and `scanning()`
  stays true. Pre-existing for `progress`; `analyzing` follows the same path.
  Fix: clear both in that arm (or whenever `list` stops a scan).
  `crates/native/src/main.rs` (`list`, `opened`).
