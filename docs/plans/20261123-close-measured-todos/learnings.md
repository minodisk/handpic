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
