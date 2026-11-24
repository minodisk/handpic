# Learnings

## Step 1

- `docs/humans/performance.md` cross-references its own sections by quoted
  heading ("see ... below"), not by anchor links, so the RAF section points at
  "DCT-scaled decode for the whole-image search (Windows 11)" the same way.
- The 2026-09-22 thread sweep table stays as it was; the bounded vs whole-file
  comparison is a paragraph and table appended to "Real folders on Windows".
  The bounded runs' bytes read were not recorded, so that cell reads "not recorded".
- The two `todo.md` sections were removed by heading match (each heading up to
  the next `### `), leaving the rest of the concurrently edited file alone.

## Step 2

- `EXTRACTOR_VERSION` was deliberately not bumped, by user decision, although
  `docs/agents/shared-crates.md` says to bump it whenever extraction output
  changes. This is a recorded exception to that rule: rows indexed before the
  change keep their larger thumbnails until the file changes or Clear Cache;
  `performance.md` / `.ja.md` say so.
- The change is not output-neutral for the ~1620x1080 NEF and CR3 previews:
  the fixed 2/8 gave 405x270, `thumbnail_jpeg_near` gives 404x270 (2/8, then
  a one-column box-average). ARW's 1616x1080 is unchanged (2/8 is exactly
  404). `performance.md` now says 404x270 for NEF and CR3.
- With `thumbnail_jpeg` gone, `scaled_thumbnail`'s `scale` closure and
  `Option` long edge had one caller, so its body was folded into
  `thumbnail_jpeg_near`. `decode.rs`'s 1616x1080 assertion
  (`thumbnail_is_a_quarter_size_jpeg`) went with it; the table test still
  covers 1616x1080 -> 404x270.
- The new `scan.rs` test uses the TIFF `fixture` named `.DNG`, so it goes
  through the RAW container path; a 3200x2400 flat-gray JPEG stays well inside
  the 1 MiB prefix.
- `performance.md`'s Step 1 paragraph naming `thumbnail_jpeg` in the
  throwaway whole-file test was left as is: it describes what that test ran.

## Step 3

- The plan's premise "nothing triggers the reader's whole-file retry today"
  was wrong. `maker_note_ifd` (the Sony gate) checked
  `t.bytes(at, count)` before the `Make` gate, so on the M (Typ 240) /
  M Monochrom (Typ 246) DNGs, whose note starts 21 to 48 MB in, the bounded
  parse failed with `MakerNote out of range` and every read (`read_preview`,
  `read_full`, `read_metadata`) fell back to `std::fs::read` of the whole
  file (23 to 48 MB). The whole-file parse is also what gave these files their
  `FocusDistance` (131780, 0, 0, 3310 on the four samples). Fixed by turning
  a `LEICA` `Make` away before that check (`is_leica`).
- Since the bounded parse now succeeds, the focus distance would have been
  lost; `FarNote::take` reads it from the same 4 KiB ranged note head. This is
  keeping what the whole-file read gave, not the out-of-scope "read the
  Typ 240 focus distance" item. `FarNote` is handed back for any Leica note
  past the buffer (not only in the strip fallback) for that reason; the
  `fallback` flag gates the preview.
- Hand-off shape: `arw::parse` keeps its signature; `arw::parse_noted`
  returns `(Arw, Option<FarNote>)`, and `reader::parse_raw` (now given the
  open `File`) finishes it. `read_metadata` now opens the file once and keeps
  it for that ranged read. No `Arw` field was added, so the five other
  `Arw` constructors are untouched.
- All four samples (the three Typ 240 and the Typ 246): 0x0300 is UNDEFINED,
  count = length, value = absolute file offset (exiftool's `-102` hint does
  not apply), the note IFD big-endian with 31 or 32 entries, the JPEG
  1472x976 with no Exif orientation, stored unrotated like the 160x120 strip
  (checked by eye on MP006646, Orientation 8). The 0x0300 count is padded
  past the JPEG's EOI (66 to 464 bytes); the decoders take it as is.
- `handpic-cli info` sweep, every Leica DNG in `D:\Photos\samples\DNG` (60
  files by `Make`), old release binary vs the rebuilt one: no errors either
  way; only these four changed, each from the 160x120 strip at offset 4096 to
  the note JPEG as both `preview` and `full`:
  `LEICA_M_(Typ_240)_MP006646.DNG` (386,048 B),
  `M_(Typ_240)_ADG_RPUsubmit_compressed_L1006373.DNG` (514,560 B),
  `M_(Typ_240)_ADG_RPUsubmit_uncompressed_L1006374.DNG` (513,024 B),
  `M_Monochrom_(Typ_246)_M2462362.DNG` (274,944 B). `focusbox` decodes each
  preview at 1472x976. A throwaway probe (not committed) confirmed, for all
  60, that `read_metadata`'s preview, full and focus distance equal a
  whole-file `arw::parse`, and that the four now parse from the 1 MiB prefix
  (a `FarNote`) instead of failing it.
- Seen, not changed: ten M10 / M10-D / M10-P and the X-U (Typ 113) samples
  still fail the bounded parse with `IFD offset out of range` (an IFD past
  1 MiB) and are read whole, as before.
- The 1:1 view of these files and the fit view on screen were not checked in
  the app; the reader hands both tiers the same JPEG as on ORF.
- Writing Rust through a Git Bash heredoc into a Python script turned `\0`
  into `\0` and then a NUL byte in `arw.rs`; byte strings with `\0` are safer
  to edit with the Edit tool.

## Step 4

- Shape: `Zoom` keeps one read in flight across the current page's crop and
  the neighbour's (`Request::ahead`, `Flight`), and a second slot (`Ahead`)
  keyed by the file path, as planned. `toggle` now returns `Toggled`
  (`Held(press)` for the `zoom` line of a held crop), `page` takes the current
  file's path and returns a `Vec` of crops to release, `Arrived::Shown`
  carries a `z` that waited for the read in flight, and a not-zoomed failure
  is the new `Arrived::Missed`. `request_prefetch`'s gate became
  `prefetch_open`, shared with `prefetch_crop`, which `crop_arrived` and a
  `Toggled::Held` also call so the neighbour's read follows the current crop.
- A crop read for an earlier page while not zoomed is now released on arrival
  even when `z` came since (`Flight::zoomed`): without it, a `z` pressed on
  file B while A's read ahead was still out would have held A's crop on B's
  page (the held-key rule keeps stale crops only for a chain that was zoomed).
- The viewport drop runs in `fit_settled` (after `FIT_RESIZE_DEBOUNCE`), not in
  `zoom_resized`: the window-bounds observer calls `zoom_resized` before the
  canvas has stored its new size, so `viewport` there is still the old one.
- The `zoom` line's format is unchanged; for a held crop it prints the crop's
  own `read` / `decode` / `orient` (paid before the press), which
  `performance.md`'s "Measuring on your own folder" now says.
- Retake driver: `drive.ps1` gained `-Extra` for `--no-prefetch`. Passing an
  empty array from a PowerShell `if` expression gives `$null`, which made
  `Start-Process` fail on every run (no log, "no window"); the driver now
  filters empty entries. The `skipped` line is the driver's own output, not
  the app's log, so waiting on it in the log never ends.
- The deep set (`D:\Photos\tests\20261123-zoom-deep-row\deep`) holds 19
  ARWs, not 20; the 20th press wraps to the first file, in the before runs too.
- Retake, 2026-11-24, release build, fresh index per run (the scan ended about
  1 s after the open, before the first `z` at 5 s), CPU load sampled just
  before each run: read ahead runs 7 / 2 / 23 / 8 %, `--no-prefetch` runs 16 /
  18 / 50 / 11 %. `zoom` `keypressToPixels` p50 / p95 / over 50ms:
  - read ahead: deep-1 11.3 / 18.0 / 0, shallow-1 10.5 / 17.5 / 0, deep-2
    11.5 / 18.7 / 0, shallow-2 12.9 / 18.4 / 0;
  - `--no-prefetch`: deep-1 43.8 / 54.0 / 5, shallow-1 21.1 / 27.4 / 0,
    deep-2 50.4 / 64.6 / 10, shallow-2 26.8 / 34.2 / 0;
  - before (2026-11-23 logs, recomputed): deep 76.5 / 88.1 / 20 and 71.8 /
    83.6 / 18, shallow 31.9 / 36.5 / 0 and 34.9 / 36.8 / 0.
- `page` lines (the `→` presses of the same runs, 20 each):
  - read ahead: p50 9.5 / 11.9 / 12.4 / 11.5ms, p95 17.6 / 18.9 / 18.6 /
    17.3ms, 19 of 20 `source=prefetch` each;
  - before: p50 13.5 / 12.3 / 12.2 / 11.2ms, p95 27.0 / 19.6 / 20.7 / 20.9ms;
  - `--no-prefetch`: all `source=read`, p50 14.6-19.4ms.
  Paging did not get slower. A held `→` (two 3 s holds at a 33ms repeat, deep
  set, about 126 pages per run) on the previous release exe (built 2026-11-23,
  before Steps 2 and 3, no crop read ahead) against this one: p50 10.8 / 11.5
  vs 11.0 / 10.6ms, p95 18.1 / 18.9 vs 18.1 / 17.4ms. So no settle delay was
  added.
- The deep set's `JpgFromRaw` (and a shallow one's) carries no DRI and no RST
  marker, checked on three extracted JPEGs.

## Deferred issues (todo candidates)

- Pending manual check (Windows 11, the app): open `D:\Photos\samples\DNG`'s
  three Leica M (Typ 240) DNGs and the M Monochrom (Typ 246) one in a fresh
  index (`--index` to a new file, or `Clear Cache` first). Expect the fit view
  to show the 1472x976 MakerNote JPEG, sharp rather than an upscaled 160x120,
  upright (MP006646 is Orientation 8), the `page` lines to read
  `px=1472x976`, and `z` to show that JPEG at its own size around the
  center. Step 3's checkbox was ticked on the automated criteria (unit tests,
  the `info` sweep, the probe).
- Pending manual check (Windows 11, the app, Step 4): open a view-only
  JPEG-only folder (a folder of plain `.jpg` files, no RAW) and page through
  it with single and held `→` presses, then zoomed (`z`) with `→`. The crop
  read ahead reads and partially decodes the whole JPEG there, so expect
  paging to stay as smooth as with `--no-prefetch` (compare the `page`
  lines), `z` to paint a crop at once once a page has settled, and a page
  while zoomed to show the next file's crop without a wait. Step 4's
  checkbox was ticked on the automated criteria (unit tests and the ARW
  retake).
