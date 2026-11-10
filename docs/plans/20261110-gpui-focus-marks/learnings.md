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

## Deferred issues (todo candidates)

- Stale scan state on a failed `prepare` (found while wiring Step 1). `list`
  takes and stops the previous `Scan`, but `opened` clears `progress` /
  `analyzing` only when it starts a new scan; when `Scan::prepare` fails
  (`(Some(Err(e)), _)`) after a re-listing that interrupted a scan, the status
  line keeps the stopped scan's `scanning` / `analyzing` text and `scanning()`
  stays true. Pre-existing for `progress`; `analyzing` follows the same path.
  Fix: clear both in that arm (or whenever `list` stops a scan).
  `crates/native/src/main.rs` (`list`, `opened`).
