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

## Deferred issues (todo candidates)

- Pending manual check (Windows 11, the app): open `D:\Photos\samples\DNG`'s
  three Leica M (Typ 240) DNGs and the M Monochrom (Typ 246) one in a fresh
  index (`--index` to a new file, or `Clear Cache` first). Expect the fit view
  to show the 1472x976 MakerNote JPEG, sharp rather than an upscaled 160x120,
  upright (MP006646 is Orientation 8), the `page` lines to read
  `px=1472x976`, and `z` to show that JPEG at its own size around the
  center. Step 3's checkbox was ticked on the automated criteria (unit tests,
  the `info` sweep, the probe).
