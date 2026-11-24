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

# Close five measured todo items

## Purpose

Five `todo.md` items were measured on 2026-11-23 and the user decided each one.
This plan records the measurements in `docs/humans/performance.md` (and
`performance.ja.md`), makes the three code changes the decisions call for, and
removes the five items:

1. `### App: a deep-row focus point still exceeds the 50ms budget`: crop
   prefetch in the app's 1:1 view, then a retake.
2. `### Core: a RAF folder scan costs ~222ms per file`: record and close, no
   code change.
3. `### Core: SIGMA fp L strip thumbnails have over ten times the pixels of other bodies'`:
   every format's thumbnail aims at 404 px.
4. `### Core: the Leica M (Typ 240) DNGs show only their 160x120 JPEG in the fit view and the 1:1 view`:
   read the Leica MakerNote preview.
5. `### Core: cold first scan on an internal SSD is far slower than the extrapolation`:
   record and close.

Rules for every step:

- All measurements are Windows 11, i7-13700, release builds, 2026-11-23, on a
  machine shared with other sessions' cargo builds (CPU load about 100 % at
  times). They were not idle-gated, and every section that records them says
  so.
- `todo.md` is edited concurrently by other sessions. A step removes only its
  own `###` sections (found by heading, not by line number) and touches
  nothing else in the file.
- Do not touch `docs/plans/20261123-all-faces-check` or other todo items.
- A change to `docs/humans/<name>.md` updates `docs/humans/<name>.ja.md` in the
  same PR.
- Do not bump `EXTRACTOR_VERSION` for the thumbnail change (user decision).

Decisions taken at approval (2026-11-23, the recommended options of
"Trade-offs and risks"): the Leica note is reached by a ranged read; the note
JPEG is taken only when no strip reaches `PREVIEW_MIN_WIDTH` and it is longer
in bytes than the largest strip; no `EXTRACTOR_VERSION` bump for the Leica
change either; every `z`-on that paints a held crop prints a `zoom` line;
`--no-prefetch` also turns the crop prefetch off; the crop prefetch follows
the preview ring's gate (off while a scan pass runs); the Typ 240 / 246 focus
distance stays out of scope.

## Steps

- [x] Step 1: Record the RAF scan cost and the bounded vs whole-file read, and
      close todo items 2 and 5 (docs only)
  - Done when:
    - The "Fujifilm RAF" section of `docs/humans/performance.md` no longer
      says its `scan` rows fail to match today's CLI without giving today's
      numbers. It keeps the old rows as the historical one-pass numbers and
      adds the 2026-11-23 ones, 1 thread, warm, 20 X-series RAFs against 20
      ARWs:
      - pass 1 (`handpic-cli scan`): RAF 89.7ms per file mean (p95 122.5),
        ARW 31.2 (p95 60.9);
      - pass 2 (`handpic-cli features <dir> 1`, `analysis_ms`): RAF mean
        299.0 / p50 294.4 / p95 393.7ms (20 of 20 with an AF point), ARW mean
        140.1 / p50 116.5 / p95 227.1;
      - the RAF preview (4416x2944) is fully decoded twice in pass 2 (the cue
        in RGB about 74ms, the score in grayscale about 50ms);
      - a mozjpeg DCT-scaled decode of those previews is not faster
        (means, n=20, the second of two decodes): grayscale 8/8 50.5ms, 4/8
        64.5, 3/8 73.1, 2/8 78.0; RGB 8/8 74.0, 4/8 84.7, 3/8 87.3, 2/8 80.0;
      - the decision: no scaled decode for the score, no code change.
    - "Real folders on Windows" gains the bounded vs whole-file comparison,
      4 threads, folders not read since boot, each once:
      - bounded 1MiB read (`handpic-cli scan`): 415 files in 4.04s
        (103 files/s) and 373 files in 6.17s (60 files/s);
      - whole file (`std::fs::read` + `arw::parse` + `thumbnail_jpeg`, a
        throwaway test that was not committed): 468 files in 48.32s
        (10 files/s, 22,048MB) and 324 files in 36.84s (9 files/s, 16,402MB),
        about 450MB/s, about the SATA link's rate;
      - a warm repeat was not faster (44 files/s bounded, 8 files/s whole)
        because of the concurrent load;
      - the 1 / 4 / 8 / 22 thread sweep is the 2026-09-22 table already
        there, and the NVMe drive is still unmeasured.
    - Both additions state the shared-machine caveat.
    - `docs/humans/performance.ja.md` carries the same content.
    - The two `todo.md` sections (items 2 and 5) are removed, nothing else.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `docs/humans/performance.md`, `docs/humans/performance.ja.md`,
      `todo.md`.
    - The measured folders: `D:\Photos\tests\20261108-gpui-compare\RAF` and
      `...\ARW` (item 2); `D:\Photos\2026\2026-08-08`, `2026-06-27`
      (bounded), `2026-07-11`, `2026-08-15` (whole file) (item 5). Describe
      them in the docs as the existing sections describe theirs (kind and
      count of files, not private paths).
    - The existing "DCT-scaled decode for the whole-image search (Windows 11)"
      section is the related earlier measurement; link to it rather than
      repeat it.
    - Commit type `docs`.

- [x] Step 2: Thumbnail every format towards 404 px, and close todo item 3
  - Done when:
    - `crates/core/src/scan.rs` calls
      `thumbnail_jpeg_near(.., JPEG_THUMBNAIL_LONG_EDGE, THUMBNAIL_QUALITY)`
      for every file; the RAF / JPEG-only `near` condition and its comment
      are gone, and the comment on `JPEG_THUMBNAIL_LONG_EDGE` no longer says
      it is a JPEG file's or a RAF's only.
    - `decode::thumbnail_jpeg` is removed (it has no other caller: only
      `scan.rs` and `decode.rs`'s own tests use it), with the doc comments
      that refer to it reworded.
    - `EXTRACTOR_VERSION` is unchanged.
    - Unit tests cover the size rule: a RAW container whose preview is larger
      than 1616 px (for example 3200x2400) gets a 404x303 thumbnail through
      `scan::extract`, and a 1616x1080 preview still gets 404x270. The
      existing RAF test stays.
    - `docs/humans/performance.md` records the measurements with the
      shared-machine caveat (per-cell decode is the minimum of 5):
      - SIGMA fp L (9520x6328): 2380x1582, 141,610B, encode 815ms, decode
        12.80ms before; 404x269, 9,829B, encode 405ms, decode 0.31ms after;
      - SIGMA fp (6000x4000, 3 files): 1500x1000, 73-86KB, encode 204-250ms,
        decode 5.2-5.4ms before; 404x270, 11-15KB, encode 109-153ms, decode
        0.34-0.37ms after;
      - ORF E-M10 II (3200x2400, 3 files): 800x600, 47-74KB, decode 2.1-4.1ms
        before; 404x303, 15-19KB, decode 0.43-0.80ms after;
      - ARW ILCE-1M2 (1616x1080): identical output either way (404x270,
        about 8.8KB).
    - The statements this makes false are updated: the ORF sentence
      ("come out 800x600, since the 3200x2400 preview is scaled at the fixed
      2/8") and the RAF paragraph ("instead of the fixed 2/8 scale") in
      `performance.md`, the M11-P "Thumbnails (528x352)" line (mark it as the
      size at the time, 404x270 now), the RAF bullet in
      `docs/agents/raw-metadata-parsing.md` ("`scan` thumbnails a RAF with
      `thumbnail_jpeg_near` (404 px) as it does a JPEG file"), and
      `performance.ja.md` to match.
    - The docs say that rows indexed before the change keep their larger
      thumbnails until the file changes or the cache is cleared, since the
      extractor version was not bumped.
    - The `todo.md` section of item 3 is removed, nothing else.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (both edit `performance.md` and `todo.md`).
    - Files: `crates/core/src/scan.rs`, `crates/core/src/decode.rs`,
      `docs/humans/performance.md`, `docs/humans/performance.ja.md`,
      `docs/agents/raw-metadata-parsing.md`, `todo.md`.
    - `decode.rs`: `thumbnail_is_a_quarter_size_jpeg` and the equality
      assertion at the end of
      `a_jpeg_thumbnail_is_scaled_down_to_the_long_edge` use
      `thumbnail_jpeg`; the table in the latter already covers 1616x1080 to
      404x270. `scaled_thumbnail`'s `scale` closure and `Option` long edge
      then have one caller; simplify only what the removal orphans.
    - `CLAUDE.md` and `crates/core/src/orf.rs` hold no "800x600" or "2/8"
      text (checked by grep); archived plans are not edited.
    - `docs/agents/shared-crates.md` ("Bump `EXTRACTOR_VERSION`, not
      `SCHEMA_VERSION`, when extraction output changes") states the general
      rule this step departs from by user decision; record the exception in
      `learnings.md`.
    - Commit type `perf(core)` or `feat(core)`.

- [x] Step 3: Read the Leica MakerNote preview of the M (Typ 240) DNGs, and
      close todo item 4
  - Done when:
    - The three samples
      (`LEICA_M_(Typ_240)_MP006646.DNG`,
      `M_(Typ_240)_ADG_RPUsubmit_compressed_L1006373.DNG`,
      `M_(Typ_240)_ADG_RPUsubmit_uncompressed_L1006374.DNG` in
      `D:\Photos\samples\DNG`) give their 1472x976 MakerNote JPEG as the
      preview and as the 1:1 tier instead of the 160x120 IFD0 JPEG, turned
      the right way up, through the bounded read plus ranged reads (no
      whole-file read).
    - Unit tests: `arw.rs` picks a Leica note's 0x0300 JPEG over a 160x120
      strip JPEG and leaves a DNG whose strips reach `PREVIEW_MIN_WIDTH`
      alone; `reader.rs` reads a note and its JPEG that lie past the prefix
      (in the style of
      `an_orf_reads_its_maker_note_preview_by_range_past_the_prefix`).
    - `handpic-cli info` was run over every Leica DNG in
      `D:\Photos\samples\DNG` before and after: none errors that did not
      before, none loses or changes a preview unexpectedly, and the files
      that gain one are listed in `learnings.md` and named in the docs.
    - `docs/agents/raw-metadata-parsing.md` records the layout and the rule;
      `docs/humans/cameras.md` (and `raw-formats.md` if its DNG text is
      touched) says what these bodies show, with the `.ja.md` files in sync.
    - `EXTRACTOR_VERSION` is unchanged; the docs say already-indexed
      Typ 240 / 246 folders keep their earlier rows until Clear Cache.
    - The `todo.md` section of item 4 is removed, nothing else.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/core/src/arw.rs`, `crates/core/src/reader.rs`,
      `docs/agents/raw-metadata-parsing.md`, `docs/humans/cameras.md`,
      `docs/humans/cameras.ja.md`, `todo.md`.
    - Seen in MP006646 at planning time (verify on the other two):
      - IFD0 holds the 160x120 JPEG as a strip (0x0111 / 0x0117), so `parse`
        is in its strip branch, where no strip reaches `PREVIEW_MIN_WIDTH`
        and both tiers fall to the largest strip;
      - the MakerNote (0x927c) is 472,064 bytes at file offset `0x159a000`
        of a 23,139,328-byte file, so the whole note, not only its JPEG, is
        past the 1 MiB `HEAD_LIMIT`. The notes of the other two files start
        at 29.5 MB and 47.9 MB;
      - the note is `LEICA\0 02 ff`, then a big-endian IFD of 32 entries at
        note offset 8 (`LEICA_HEADER_LEN`); its first entry is
        `0300 0007 0005e400 0159b000`: tag 0x0300, UNDEFINED, count 386,048,
        value `0x0159b000`, which is the absolute file offset of the JPEG's
        `FF D8`. So this tag's offset is file-absolute here, and exiftool's
        "fix by -102" hint does not apply to it; confirm that on the other
        two files before relying on it;
      - the M Monochrom (Typ 246) sample has the same layout (note at
        `0x1455c00`, 0x0300 of 274,944 bytes at `0x1456c00`), so it is
        expected to gain a preview too.
    - `leica_focus_distance` tests the note with
      `t.bytes(at, count).is_ok_and(..)`, so on a prefix that does not hold
      the note it returns `Ok(None)` and `parse` succeeds; nothing triggers
      the reader's whole-file retry today. Decided: a ranged read —
      `reader.rs` reads the note's header and IFD by range when `arw::parse`
      reports a Leica note past the buffer, resolves 0x0300, then
      range-reads the JPEG. This needs a new hand-off between `arw.rs` and
      `reader.rs` (the existing ranged reads are only for an `Embedded` the
      prefix already named).
    - The MakerNote JPEG has no width or height tag in the note's IFD, and
      `Embedded` carries no pixel size. Decided tier rule: only when no strip
      reaches `PREVIEW_MIN_WIDTH`, and the note JPEG is longer in bytes than
      the largest strip, take it as both the preview and the 1:1 tier.
      `arw.rs` already compares by byte length for the Sony fallback (see
      "Older Sony ARW bodies carry no full-size JPEG" in
      `raw-metadata-parsing.md`).
    - Keep the change inside the DNG strip branch's fallback so Sony ARWs and
      DNGs with a strip of at least `PREVIEW_MIN_WIDTH` (M10, M11, Q, SL) are
      not affected; the `info` sweep is the check.
    - Verify the orientation: IFD0's Orientation is 8 in MP006646. Extract
      the note JPEG (`exiftool -b -Leica:PreviewImage`) and check it is
      stored unrotated like the IFD0 JPEG, so `Arw.orientation` turns it
      correctly, and whether it carries an Exif orientation of its own
      (`reader::neutral` rewrites one to 1).
    - With one 1472x976 JPEG for both tiers, the 1:1 view shows the preview
      at its own size, as on ORF and the older Sony bodies; `cameras.md`
      already has that wording for those.
    - The Typ 240 / 246 `FocusDistance` the same note would give is out of
      scope.

- [ ] Step 4: Prefetch the 1:1 crop in the app, retake the deep / shallow
      measurement, and close todo item 1
  - Done when:
    - Once the current page's preview is on screen, the app reads its crop
      in the background, so a `z` press paints a crop already held.
    - While zoomed, the app also reads the crop of the next neighbour in the
      paging direction, and paging onto that file shows it without a read.
    - At most one prefetched crop per slot (the current page's and the
      neighbour's, about 4MB each at most); each is dropped when the page,
      the listing or the viewport no longer matches it; crop reads stay off
      the UI thread and one read is in flight at a time.
    - The crop prefetch follows the preview ring's gate: it does not run
      while a scan pass runs, and `--no-prefetch` turns it off too.
    - Every `z`-on that paints a held crop (prefetched or not), or that lands
      while the prefetch is still in flight, prints one `zoom` line with its
      `keypressToPixels`.
    - A failed background prefetch shows no error; the failure appears as it
      does today when the user presses `z`.
    - `zoom.rs` has unit tests for the new states in its existing style
      (`Zoom::<u8>`): prefetch after `shown`, `z` reusing it with a timing,
      `z` during the in-flight prefetch, the drop on page / listing /
      viewport change, the neighbour slot's hit and miss while zoomed, and a
      failed prefetch.
    - The retake is recorded in "The 1:1 focus check path" of
      `docs/humans/performance.md` as before / after, with the shared-machine
      caveat. Before (GPUI app, α7 V ARW, `JpgFromRaw` 7008x4672 baseline
      4:2:2, Orientation 8, default 1600x1000 window, `z` once per file, 20
      files per set, two runs each):
      - deep (focus row mean 0.94 of the JPEG height): decode p50 55-58ms,
        `keypressToPixels` p50 72-77 / p95 84-88ms, 38 of 40 over 50ms;
      - shallow (mean 0.06): decode p50 19ms, `keypressToPixels` p50 32-35 /
        p95 36-37ms, 0 of 40 over 50ms.
      After: the same four runs' `keypressToPixels` p50 / p95 and the count
      over 50ms, deep and shallow, plus a same-session `--no-prefetch`
      before.
    - The same section records why the other option was dropped: the JPEG
      has no restart markers (no DRI, no RST), so a deep row cannot be
      reached without entropy-decoding the rows above; the app already
      paints the upscaled fit-view preview as the placeholder at once
      (`zoom.rs` `placeholder`); and a DCT-scaled decode is not faster
      (Step 1's numbers).
    - The section states what the prefetch does not cover (a `z` pressed
      before the background read ends still waits for the rest of it, and a
      `z` during a scan pass is not prefetched).
    - The `page` lines of the retake's `right` presses are compared with the
      before runs' to check that paging did not get slower; the result is in
      `learnings.md`.
    - `CLAUDE.md`'s `src/zoom.rs` description (and `--no-prefetch`'s),
      the 1:1 rules in `docs/agents/app.md` ("A held key draws each stale
      crop before chasing the current one", and the rule that a reused crop
      prints no `zoom` line) and `docs/agents/app-log.md` (if the `zoom`
      line's format changes) match the new behavior; `performance.ja.md` is
      in sync.
    - The `todo.md` section of item 1 is removed, nothing else.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 to 3 are merged (`performance.md`, `todo.md`).
    - Files: `crates/app/src/zoom.rs` (the `Zoom<C>` state machine and its
      tests, `send`, `Timed`), `crates/app/src/main.rs` (`show_current`,
      `toggle_zoom`, `zoom_page`, `send_crop`, `crop_arrived`,
      `zoom_resized`, `clear_ring`'s callers, the canvas closure that takes
      `zoom_pending`), `CLAUDE.md`, `docs/agents/app.md`,
      `docs/humans/performance.md`, `docs/humans/performance.ja.md`,
      `todo.md`. `zoom.rs` gained `read_failed` on main after planning; keep
      it working.
    - What `Zoom` already does, to build on rather than duplicate:
      - `arrived` holds a current-page crop even while not zoomed, and
        `toggle` reuses a held crop of the current page and viewport
        (`holds`) without a request. So the current-page prefetch is mostly
        a `request(viewport, None)` once the preview is shown;
      - `page()` releases the held crop when not zoomed, which already drops
        a prefetched crop on a page or listing change, and `holds` already
        refuses a crop cut for another viewport;
      - a reused crop prints no `zoom` line today (`toggle` returns `None`
        and `crop_arrived` sets `zoom_pending` only from an arrival that
        carried a keypress), and a `z` pressed while a read is in flight
        loses its keypress (`request` returns `None`). Both need a way for
        the keypress to reach `zoom_pending`.
    - The neighbour slot cannot be keyed by `seq` (a page bump is what makes
      it current); key it by the file and drop it on every listing change
      (where `clear_ring` runs) so changed bytes are never shown. Take the
      direction from the view's `heading` and the file from
      `prefetch::neighbours`' order.
    - Keep the rule that the canvas never mixes two files
      (`placeholder_fits`) and the order "page bump before `request`"
      (`zoom_page` before `request` in `go`).
    - Not fighting the preview path: the preview ring's `request_prefetch`
      runs only when the pager has nothing in flight, the image on screen is
      the current file's and no scan pass runs (`scanning()`); `load` takes
      a scan pause (`slots.pause()`) that `PaintHold` keeps until the paint,
      while `zoom::read` takes none. Follow the ring's gate, and verify with
      the `page` lines that a held `right` key is not slowed by a crop read
      per settled page; if it is, add a short settle delay before the
      prefetch.
    - `Crop` carries its own `read` / `decode` / `orient` durations; for a
      prefetched crop the `zoom` line can print those or zeros. If the line
      gains a field or a new meaning, update `docs/agents/app-log.md` and
      the "Measuring on your own folder" text.
    - A view-only JPEG folder's "full JPEG" is the whole file, so its crop
      prefetch reads and partially decodes the whole JPEG; check one such
      folder by hand for paging smoothness.
    - Retake: folders `D:\Photos\tests\20261123-zoom-deep-row\deep` and
      `...\shallow` (20 hardlinked files each). Driver:
      `D:\Photos\tests\20261110-gpui-zoom-peaking-tools\drive.ps1`; a copy
      that also passes `--settings` / `--log`, and the step list
      (`steps.txt`: `wait:5000`, then 20 times
      `key:z,wait:900,key:z,wait:300,key:right,wait:700`, then `q`), was in
      the planning session's scratchpad folder
      `C:\Users\daisu\AppData\Local\Temp\claude\e11915a608de0abc\d9771702-f690-418c-a4ae-9aa39424427a\scratchpad\zoom\`,
      which may have been cleared; recreate both from this description if
      so. Runs as before: deep-1, shallow-1, deep-2, shallow-2, each with its
      own temporary `--index`, and the same four under `--no-prefetch` as
      the same-session before. The user has already allowed this
      keystroke-driving run. A release build prints to stdout only through a
      pipe or a file on Windows.
    - The neighbour prefetch while zoomed prints no `zoom` line (paging is
      not a `z` press), so it is covered by the unit tests; a screen check
      is optional.

## Trade-offs and risks

- **How the reader reaches the Leica note (Step 3), decided: ranged read.**
  The whole-file retry (the NEF / ORF precedent) would read 22-48 MB on every
  read of these files; Step 1's numbers put a whole-file scan at about
  10 files/s cold on the SATA drive.
- **Tier rule for a preview narrower than `PREVIEW_MIN_WIDTH` (Step 3),
  decided: byte-length fallback** inside the strip branch. Lowering
  `PREVIEW_MIN_WIDTH` or reading the note JPEG's SOF would change the tier
  choice for other DNGs or need a third ranged read.
- **No `EXTRACTOR_VERSION` bump (Steps 2 and 3).** fp L, fp, ORF, RAF, M11-P
  and Typ 240 / 246 folders indexed before keep their earlier thumbnails and
  rows until the file changes or the cache is cleared; the docs say so. A
  bump would re-extract every row of every folder for every user.
- **The `zoom` line on a reused crop (Step 4), decided: every `z`-on that
  paints a held crop prints one.** An off / on of an already shown crop now
  prints a line too; `docs/agents/app.md` changes accordingly.
- **`--no-prefetch` widens (Step 4).** It now covers the crop prefetch as
  well as the preview ring; `CLAUDE.md` and `app.md` say so.
- **Crop prefetch while the scan runs (Step 4), decided: off.** A `z` during
  a folder's scan costs what it does today (the deep-row 72-77ms).
- **Wasted crop reads while paging (Step 4).** Every page that settles
  starts a crop read that the next page drops: a core's 20-60ms per settled
  page off the UI thread. The retake's `page` lines are the check; a short
  settle delay is the fallback.
- **Retake on a loaded machine.** The before numbers were taken under other
  sessions' builds; the same-session `--no-prefetch` runs, the shallow set as
  the control and the caveat in the docs are the mitigation.
- **Concurrent `todo.md` and `performance.md` edits.** Four PRs touch both;
  run the steps in order and merge main before each PR.

## Progress

(2026-11-24) Step 1 complete
(2026-11-24) Step 2 complete
(2026-11-24) Step 3 complete
