# Parsing RAW metadata in `crates/core`

Read this before touching the TIFF / MakerNote parsing in
`crates/core/src/arw.rs`, which reads both ARW and DNG files, in either TIFF
byte order (IFD0, the SubIFDs, the ExifIFD, and the Sony, Leica and Sigma
MakerNotes), or in
`crates/core/src/nef.rs`, which reads Nikon NEF files (see [NEF](#nef)), or in
`crates/core/src/cr3.rs`, which reads Canon CR3 files (see [CR3](#cr3)), or in
`crates/core/src/raf.rs`, which reads Fujifilm RAF files (see [RAF](#raf)), or in
`crates/core/src/orf.rs`, which reads OM System / Olympus ORF files (see
[ORF](#orf)). It lists the
pitfalls this repository has already hit, each with the reason it happens.
`crates/core/src/reader.rs` only picks which bytes to read (and hands an HEVC
image to `crates/core/src/hevc.rs` to decode); no maker-specific parsing lives
there. For a new Sony MakerNote field, also read
[Sony MakerNote fields: verify each tag's type and model `Condition` in Sony.pm directly](./tauri-app.md#sony-makernote-fields-verify-each-tags-type-and-model-condition-in-sonypm-directly-hit).

The tags follow [`tauri-app.md`](./tauri-app.md): **Hit** broke something
here, **Measured** steered a design decision, **Inferred** comes from sources
or docs only.

## TIFF structure

### A single `SubIFDs` entry holds the IFD offset inline (Hit)

`SubIFDs` (0x014a) with `count == 1` carries the one SubIFD's offset in the
entry's value field; only with `count > 1` is the value an offset to an array
of IFD offsets. The `TAG_SUB_IFDS` branch of `parse` pushes the value itself
for `count == 1` and reads the array (range-checked) otherwise.

- Why: a LONG[1] fits in the entry's four value bytes, so TIFF stores it
  inline, like any value of four bytes or fewer.
- Testing: a synthetic-TIFF fixture with one SubIFD only exercises the inline
  path. Give it at least two SubIFDs (as `tiff_with_sub_ifds` in the tests
  does) to cover the array path.
- Source: [leica-dng-support learnings, Step 1](../plans/_archived/20260918-leica-dng-support/learnings.md#step-1).

### A count-1 `SHORT` entry's padding can be nonzero (Hit)

A `SHORT` with `count == 1` occupies the first two bytes of the entry's value
field (the low half of the decoded `u32` in a little-endian file; see the next
entry for big-endian); the other two bytes are padding, and some writers leave per-file
garbage there (SIGMA fp L DNGs do, on `Compression`, `PhotometricInterpretation`
and the like). `integer()` takes those two bytes through `inline_short` (it masks with
`& 0xFFFF` only for `II`; for `MM` it takes `value >> 16`).

- What broke: without the mask, `strip_jpeg` compared a padded `Compression`
  / `PhotometricInterpretation` against the JPEG / YCbCr values and rejected
  the strip JPEGs, so a SIGMA fp L DNG got the wrong full-size JPEG or no
  embedded JPEG at all.
- Rule: read any single `SHORT` / `LONG` through `integer()` rather than
  using the raw value field. The regression test
  `short_entries_ignore_the_padding_in_their_high_half` ORs garbage into the
  high half.
- Source: [tiff-short-padding learnings, Step 1](../plans/_archived/20260924-tiff-short-padding/learnings.md#step-1).

### `arw.rs` reads both byte orders, and `MM` inline values sit in the high bytes (Hit)

`arw::parse` opens the file through `sequence::Tiff::new(buf, 0, buf.len())`,
which accepts `II` and `MM` (magic 42), and reads every IFD entry, offset
value, `SubIFDs` array, RATIONAL, `SHORT[N]` and MakerNote IFD in the header's
order. The `Entry` value field is the four bytes decoded as a `u32` in that
order, so the values that ride inline need the order to pick their bytes:

- A count-1 `SHORT` is the first two bytes: `value & 0xFFFF` for `II`,
  `value >> 16` for `MM` (`inline_short`; orientation and `integer()` go
  through it).
- A count-1 `BYTE` is the first byte: the low byte for `II`, the top byte for
  `MM` (`inline_bytes(..)[0]`).
- An ASCII of four bytes or fewer is the value field's bytes in file order
  (`inline_bytes`), as is the Sigma `SHORT[2]` AF point.
- What broke: until this was in place, `parse` rejected anything but `II` with
  `not a little-endian TIFF/ARW`, so every big-endian DNG failed to open: the
  Pentax K-series and 645D, Samsung GX10 / GX20, Ricoh GR / GR II /
  GR DIGITAL 2 and 4 / GXR / GX200, Leica M (Typ 240) and M Monochrom
  (Typ 246), the iPhone 6s Plus through 17 Pro Max, the Canon EOS 350D DNG,
  the OpticFilm scanners and the Blackmagic Pocket Cinema Camera (52 `MM`
  files in the sample folder). Sony ARWs, Sigma and the newer Leica DNGs are
  `II`, so they read exactly as before.
- Testing: the `II` helpers (`tiff`, `ifd`, ...) stay little-endian; the `MM`
  tests use `crate::jpeg::tests::W(false)` or `ifd_be`, which writes nonzero
  padding after each inline `SHORT`.
- Source: [big-endian-dng learnings, Step 1](../plans/_archived/20261002-big-endian-dng/learnings.md#step-1).

### Older Sony ARW bodies carry no full-size JPEG, so `parse` falls back to the preview as `full` (Measured)

On the α9 II, α7R IV / IVA, α7C, α6400, α6600 and ZV-E10 the IFD chain holds
only IFD0 (the 1616x1080 preview, 1920x1080 on the ZV-E10, 1440x1080 on a 4:3
shot) and IFD1 (a 160x120 thumbnail); the SubIFD is the raw data. Recent
bodies add IFD2, the full-size JPEG. `arw::parse` picks the largest JPEG in
the chain and the SubIFDs, and when that one is smaller than the preview by
byte length (the thumbnail is a few KB against a 300-900 KB preview) it takes
the preview as `full` instead, so the 1:1 view (`read_focus_crop`) crops the
preview, as on ORF. `Embedded` carries no pixel size, so the comparison is by
bytes. A preview-only file (no other JPEG) keeps `full` at `None`, and the DNG
strip branch is not affected.

- Rule: a Sony body without IFD2 is still listed in `docs/humans/cameras.md`;
  note there that its 1:1 view is the preview.
- Placement: keep the fallback inside the non-DNG `else` of the strip gate in
  `arw::parse`. Applied after the whole `let (preview, full)` expression, it
  also hits the DNG strip results, whose synthetic test lengths do not grow
  with the pixel size. That breaks `picks_dng_strip_jpegs_by_size_not_by_index`,
  `short_entries_ignore_the_padding_in_their_high_half` and
  `picks_dng_strip_jpegs_out_of_a_big_endian_sub_ifd_array`.
- Consumers: only `reader::read_full` (`Kind::Full`) and the CLI `info` /
  `bench` / `check` read `full`. `scan.rs` and `crates/backend/src/index.rs` do not,
  so this change needs no `EXTRACTOR_VERSION` bump.
- Source: [sony-arw-coverage learnings, Step 1](../plans/_archived/20260930-sony-arw-coverage/learnings.md#step-1-sony-arw-sample-verification),
  [sony-arw-preview-full learnings, Step 1](../plans/_archived/20261005-sony-arw-preview-full/learnings.md#step-1).

## MakerNotes

### The Sony MakerNote gate skips only a known non-Sony `Make` (Hit)

`maker_note_ifd` reads the MakerNote (0x927c) as a Sony IFD unless the note
lacks the `SONY` header **and** `Make` is present and does not start with
`SONY`. A missing `Make` still gets the Sony parse.

- Why: recent Sony bodies (Sony5) start the IFD right at the tag's offset
  with no header, so "parse only when the note starts with `SONY` or `Make`
  starts with `SONY`" broke the existing synthetic fixtures, whose
  headerless Sony5-style notes carry no `Make`. Reading another maker's note
  as a Sony IFD walks garbage, hence the gate.
- Testing: a synthetic non-Sony note must set `Make` (the tests use
  `Leica Camera AG`); without it the Sony parse reads the note's first bytes
  (`LE`) as an entry count and errors.
- Source: [leica-dng-support learnings, Steps 1 and 3](../plans/_archived/20260918-leica-dng-support/learnings.md#step-1).

### Leica: `LEICA\0` header, an IFD at offset 8, `FocusDistance` in millimeters (Hit)

A Leica MakerNote is `LEICA\0` plus two bytes, then a plain IFD at note
offset 8 (`LEICA_HEADER_LEN`) in the container's byte order: the M10 / Q2
note (exiftool's `MakerNoteLeica9`) is `LEICA\0 02 00` and a little-endian
IFD in a little-endian DNG, while the M (Typ 240) / M Monochrom (Typ 246) note
(`MakerNoteLeica6` / `7`) is `LEICA\0 02 ff` and a big-endian IFD in a
big-endian DNG (`4c45 4943 4100 02ff 001f 0300 0007 ...` at the note start),
so `leica_focus_distance` reads it in the container's order, with no
per-note byte-order sniff. `leica_focus_distance` reads only `FocusDistance`,
tag 0x0304, a LONG count 1 (value inline), taken as millimeters. The note's
own `FNumber` is 1.0 on M-mount lenses and is not read.

- The aperture comes from the ExifIFD instead: `ApertureValue` (0x9202, APEX,
  RATIONAL) converts as `2^(AV/2)` into `estimated_f_number`, which `exif`
  clears when `FNumber` is present, so it is only used when `FNumber` is
  absent.
- Testing: a "non-Sony note is skipped" fixture needs a valid empty Leica IFD
  (`leica_note(&[])`). A `0xffff` entry count after the header, which the
  Sony-only parser used to ignore, is an out-of-range IFD error once the
  Leica parse reads it.
- Source: [leica-dng-support learnings, Step 3](../plans/_archived/20260918-leica-dng-support/learnings.md#step-3)
  (the exiftool comparison over the samples).

### Sony `FocusFrameSize` is `UNDEFINED[6]` on real files (Hit)

ExifTool documents `FocusFrameSize` (0x2037) as `int16u[3]` (width, height,
validity), but real α7 V files store it as `UNDEFINED[6]`; exiftool only
reinterprets the bytes. The `focus_frame` match in `exif` remaps
`(TYPE_UNDEFINED, 6)` to `(TYPE_SHORT, 3)` before `shorts::<3>` and still
accepts a genuine `SHORT[3]`. A zero validity SHORT gives `None`.

- What broke: a reader accepting only `SHORT[3]` returned `None` on a real
  file, so the eye-AF frame was never used.
- Rule: check a tag's type on a real file, not only in ExifTool's table; the
  table can describe the decoded value rather than the stored type.
- Source: [sony-eye-af-window learnings, Step 1](../plans/_archived/20260922-sony-eye-af-window/learnings.md#step-1).

### Sony `FocusLocation` frame: crop and reduced RAW sizes need no per-body handling (Measured)

`FocusLocation` is in the frame of the file's own image, not always the sensor
frame. In crop modes (α7 IV APS-C, α7R V S35) it is already in the crop frame
and matches the full JPEG (4608x3072, 6240x4160). For reduced RAW sizes the
bodies differ. The α7 IV M / S keep 7008x4672 while the full JPEG is smaller.
The α7R V, α9 III and α7CR M / S write the reduced size. The aspect ratio
always matches the preview, and the consumers scale by `sensor_w` /
`sensor_h` against the JPEG, so both work: no parser change and no
`EXTRACTOR_VERSION` bump.

- Manual focus: `FocusMode` 0 with the point at the exact center and
  `FocusFrameSize` validity 0 is dropped by `trusted_focus`. The α9 III and
  α7CR samples are all MF, so they prove nothing about the AF mapping.
- Sample limits: the raw.pixls.us Sony samples hold no `AFTracking` 1 (face
  tracking), but the local sample set has `AFTracking` 1 with a valid
  `FocusFrameSize` on the α7 IV, α7R V, α7S III, α6700 and ZV-E1, the frame
  on a face or eye except in two α6700 samples (an empty background, bread on
  a market stall). No sample holds a portrait frame, so that is unverified on
  every body. An `AFTracking` of 2 is lock-on AF, not face tracking.
- Source: [sony-arw-coverage learnings, Step 1](../plans/_archived/20260930-sony-arw-coverage/learnings.md#step-1-sony-arw-sample-verification).

### Sigma: inline values first, and `Make` differs by body (Hit)

`sigma_af_point` reads the Sigma BF's AF point, tag 0x0147, a `SHORT[2]`
that fits in the entry and so is read from the value field
(`value.to_le_bytes()`), never from `buf`.

- An entry whose value fits in the entry is inline, not an offset: with
  `count <= 4` the MakerNote tag's value field holds the note bytes
  themselves, so reading it as an offset lands anywhere (the test uses
  `0xffff_fff0`, past the buffer). And a note of `count <= header_len` has no
  room for an IFD after its header. So the size check comes before the
  offset / range check: `count <= SIGMA_HEADER_LEN` returns `None` first, and
  only a longer note whose range runs past the buffer bails. This is the
  same order `leica_focus_distance` uses with `LEICA_HEADER_LEN`. Test:
  `a_sigma_bf_maker_note_with_an_inline_value_is_none`.
- The same vendor writes `Make` differently by body: the BF writes `Sigma` /
  `Sigma BF`, the fp L `SIGMA` / `SIGMA fp L`. So the gate is a
  case-insensitive `SIGMA` prefix on `Make` plus an exact `Model` of
  `Sigma BF` (`SIGMA_BF_MODEL`).
- Source: [sigma-bf-af-point learnings, Step 1](../plans/_archived/20260924-sigma-bf-af-point/learnings.md#step-1)
  (the exiftool comparison and the grid check).

## NEF

`crates/core/src/nef.rs` parses a Nikon NEF over `sequence::Tiff`, the
byte-order-aware walker, and reads IFD0 and the Exif IFD through the shared
crate-private `crates/core/src/exif.rs` (also used by `jpeg.rs`); unlike
`arw.rs`, it keeps its own Exif fields lenient (an unreadable one is `None`)
but errors on any IFD or SubIFD that runs past the buffer, so a short prefix
triggers the reader's whole-file retry instead of a file with no JPEGs. The
same range-vs-contents split applies to the MakerNote (0x927c) that
`af_point` reads: a declared range running past the buffer is an error too
(so a truncated prefix retries with the whole file instead of caching a
missing AF point), while a note fully inside the buffer whose contents are
malformed (a bad inner TIFF header, an unreadable IFD entry, ...) is lenient
and yields `None`.

### The preview is a SubIFD JPEG, not IFD0's or the MakerNote's (Measured)

The embedded JPEGs are the SubIFDs (0x014a) with `Compression = 6` and
`JPEGInterchangeFormat` / `JPEGInterchangeFormatLength` (0x0201 / 0x0202).
`parse` takes the first as `full` (the JpgFromRaw) and, when there are more,
the last as `preview`; with one, it is both.

- Measured on raw.pixls.us NEFs of 24 bodies (Z 9 back to D70): the first
  JPEG SubIFD is always the full-size JPEG, and every body since about the
  D800 / Df adds a later one of 1620x1080 (1632x1080 on the D800). The
  MakerNote `PreviewIFD` (0x0011) JPEG is only 640x424 (570x375 on older
  bodies), below `PREVIEW_MIN_WIDTH`, so it is not read at all.
- IFD0 of the Z 9, Z 8 and Z f carries its own 0x0201 / 0x0202: a 160x120
  thumbnail. Taking IFD0's JPEG as the preview, as `arw.rs` does for ARW,
  would show a thumbnail; only SubIFDs are searched.
- Size or byte count cannot tell the two JPEGs apart: the JPEG SubIFDs carry
  no `ImageWidth`, and a Z 6 sample's full JPEG (3024x2016) is smaller in
  bytes than its 1620x1080 preview. Hence the order rule.
- The Z bodies are little-endian; older ones (D90, D3, D7000, Df, D800) are
  big-endian.
- Every Nikon MakerNote offset is relative to the TIFF header at note offset
  10 (after `Nikon\0`, two version bytes and two more). `af_point` builds a
  second `Tiff::new(buf, note + 10, note_end)` for it; only `AFInfo2` is read
  (see below).

### `AFInfo2`: the offsets depend on the version, top-left origin (Measured)

`nef::af_point` reads MakerNote tag 0x00b7 `AFInfo2` (UNDEFINED, in the
note's own byte order) into `Shot.focus` and, when the area size is nonzero,
`Shot.focus_frame`. The fields are `int16u`: `AFImageWidth`, `AFImageHeight`,
`AFAreaXPosition`, `AFAreaYPosition`, `AFAreaWidth`, `AFAreaHeight`, starting
at byte 0x2a for versions `0300` / `0301` and at 0x3e for `0400` / `0401` /
`0402` (ExifTool's `Nikon.pm` `AFInfo2V0300` / `AFInfo2V0400`).

- The plan first assumed bytes 16-26; that is the `AFInfo2V0100` layout of
  older DSLRs (contrast-detect AF only). Read the version's own table.
- The position is valid only when byte 7, `AFCoordinatesAvailable`, is 1.
  The Z 8 sample, in auto-area AF that never locked, writes 0 and zeros; the
  D850 / D500 write `0101`, whose position is a grid point name. Both give
  `None`, as does a zero position or a zero `AFImage` size.
- The position is the AF area's center in the unrotated image, origin at the
  top left, Y down, in the `AFImage` frame, which is the full image size
  (8256x5504 on the Z 9, 5568x3712 on the Z 30). So it maps directly to
  `FocusLocation { sensor_w: AFImageWidth, sensor_h: AFImageHeight, x, y }`,
  and the consumers' scaling handles a reduced-size JpgFromRaw.
- Checked on raw.pixls.us samples by drawing the point on the preview:
  off-center landscapes (the Z fc flower, the Z 6 eye at y 1160 of 4024, the
  Z 7II, the Z 9) and a portrait frame (Z 30, Orientation 8: the point lands
  on the subject after rotation). Versions seen: `0300` Z 6, Z 50; `0301`
  Z 5, Z 6II, Z 7II, Z fc, Z 30; `0400` Z 8, Z 9; `0401` Z f. All
  little-endian.

## CR3

`crates/core/src/cr3.rs` walks a Canon CR3's ISOBMFF boxes (big-endian sizes,
a 64-bit `largesize` after the type when the 32-bit size is 1) up to `mdat`,
and reads the two Exif TIFFs through `crates/core/src/exif.rs`: `CMT1`
(IFD0: `Make`, `Model`, `Orientation`) with `read_ifd0` and `CMT2` (whose
IFD0 *is* the Exif IFD) with `read_exif_ifd`, each over
`Tiff::new(buf, payload, box_end)`. Like `nef.rs`, it keeps the Exif fields
lenient but errors on any box that runs past the buffer or its parent.

### Where the JPEGs are (Measured)

- `preview`: the `PRVW` box inside the top-level `uuid`
  `eaf42b5e-1c98-4b88-b9fb-b7dc406e4d16`, after the 16 uuid bytes and 8
  more. Its payload is a 16-byte header (the data length is the big-endian
  u32 at 12) and then a 1620x1080 JPEG. Fallback: `THMB` in the Canon `uuid`
  (`85c0b687-820f-11e0-8111-f4ce462b6a48`, inside `moov`), 160x120, length
  at 8, data at 16; then `full`.
- `full`: the `trak` whose `stsd` sample entry (`CRAW`) carries a `JPEG`
  sub-box; the raw tracks carry `CMP1`. The sub-boxes start 82 bytes into the
  sample entry's payload. The size is `stsz`'s `sample_size` (or its first
  entry when that is 0) and the offset the first `co64` (or `stco`) entry.
  It is the full-resolution JPEG on every sample (8192x5464 on the R5).
- The Canon `CTBO` box lists the top-level offsets (index 2 is the preview
  `uuid`); it matched the walk on every sample and is not read.
- Measured on raw.pixls.us CR3s of 14 bodies (EOS R, RP, R3, R5, R5 Mark II,
  R6, R6 Mark II, R6 Mark III, R7, R8, R10, R50, R50 V, R100): `moov` ends by
  ~90 KB and the `PRVW` box by ~940 KB, so the 1 MiB `HEAD_LIMIT` prefix
  parses and slices the preview without a ranged read or a whole-file
  retry.

### `AFInfo2`: center origin, Y up, several points (Measured)

`cr3::af_point` reads `CMT3` (the Canon MakerNote as a little-endian TIFF,
whose IFD0 is the MakerNote IFD) tag 0x0026 `AFInfo2`, an `int16u` array
(ExifTool's `Canon.pm` `AFInfo2`): `AFInfoSize`, `AFAreaMode`,
`NumAFPoints` (n), `ValidAFPoints`, `CanonImageWidth` / `Height`,
`AFImageWidth` / `Height`, then four `int16s[n]` arrays (`AFAreaWidths`,
`AFAreaHeights`, `AFAreaXPositions`, `AFAreaYPositions`) and two bitmasks of
`ceil(n / 16)` words (`AFPointsInFocus`, `AFPointsSelected`).

- Positions are the area centers relative to the image center with **Y
  up**, as ExifTool notes for EOS models. PowerShots, which also write CR3,
  have Y down, so only a `Model` containing `EOS` is read. Settled on samples
  where the two directions land on different things: an R10 portrait
  (Orientation 8, y 640: Y up lands on the head, Y down on the blurred
  background), an R5 Mark II crop (y 818 of 3392: the tomato vs the plate)
  and an R6 Mark III (y 406: the windmill).
- `AFImage` equals `CanonImage` on every sample, which is the recorded image
  size (5088x3392 for the R5 Mark II APS-C crop), so it is the
  `FocusLocation` frame once the origin moves to the top left
  (`x = w / 2 + X`, `y = h / 2 - Y`).
- n is 143, 651 or 1053 by body, but in single-point modes only one point is
  valid. Zone and whole-area modes flag many points in focus (up to a few
  hundred), so the focus is the center of their bounding box and the frame
  its size, not the first point (the top-left corner of the cluster). With
  none in focus, a single selected point is used (the RP and R6 Mark III
  samples); several selected and none in focus (an area that never locked)
  gives `None`, as does manual focus (`ValidAFPoints` 0 on the R6 sample).
  Only indices below `ValidAFPoints` are read.
- Every sample writes 0x0026, not 0x003c `AFInfo3`.

### HDR PQ (HEIF) files carry HEVC, decoded by `hevc.rs` (Measured)

With HDR PQ on, the body writes HEVC instead of JPEG in `PRVW`, `THMB` and
the first track (whose sample entry carries `HEVC` / `hvcC` instead of
`JPEG`; on the local R8 and R5 Mark II samples its sub-boxes are `HEVC` and
`free`, and the other two tracks carry `CMP1` / `CDI1`). Every R8 sample and
two of the four R5 Mark II samples on raw.pixls.us are like this.

- The `PRVW` / `THMB` header's first byte is a version: 0 before a JPEG
  (`PRVW` length at 12, `THMB` length at 8), 1 before HEVC. A version-1
  header is `01 00 00 00 | 00 02 | u16 width | u16 height | ff ff | u32
  length` for both boxes (1620x1080 for `PRVW`, 320x214 for `THMB`), so the
  `THMB` length is at 12 there, not 8. The length spans every box after the
  header up to the end of the `PRVW` / `THMB` box.
- After the header come `CISZ` (20 bytes; the coded size), `hvcC` (a
  standard HEVCDecoderConfigurationRecord, `lengthSizeMinusOne` 3, whose
  arrays hold the VPS / SPS / PPS), `colr` (`nclx`: BT.2020 primaries, PQ
  transfer, BT.2020 NCL matrix, full range), `pixi` (3 channels of 10 bits),
  `IMGD` (a u32 total length, then 4-byte length-prefixed NAL units: four
  IDR slices for `PRVW`, one for `THMB`) and sometimes a trailing `free`.
- The stream is HEVC Main 4:2:2 10 (RExt), CTB 32, several slices, no
  conformance window: `PRVW` is coded 1664x1088 and `THMB` 320x320, so the
  decoded frame is cropped to the header's width and height (the excess is
  CTB padding on the right and bottom).
- `parse` takes a version-1 `PRVW` / `THMB` whole, header included, as an
  `Embedded` with `Codec::Hevc`; only the header must lie in the prefix.
  With no `JPEG` track, `full` is that HEVC `PRVW` (the full-size HEVC in
  the first track is a `GRID` of tiles and is not decoded), so `z` shows a
  crop of the 1620x1080 preview. `Arw::hevc` still marks an `HEVC` track
  with no `JPEG` one; the reader fails with `reader::HEVC_UNSUPPORTED` only
  when such a file has no `PRVW` / `THMB` it can read.
- `reader::embedded_from` passes an HEVC `Embedded`'s bytes to
  `hevc::to_jpeg`, which builds an Annex B stream (the `hvcC` parameter sets,
  then the `IMGD` NAL units), decodes it with the pure-Rust `hpvcd` crate,
  tone-maps the frame to sRGB (full-range BT.2020 Y'CbCr to R'G'B', the
  ST 2084 PQ EOTF, 203 nits as 1.0, BT.2020 to BT.709 primaries, the roll-off
  `v / (1 + v / 4) * 1.25`, the sRGB OETF) and encodes a JPEG, so every
  consumer of the reader's bytes still gets a JPEG. The tone map ignores the
  VUI and always assumes this Canon format.
- The HEVC `PRVW` box ends at 941 KB on the R8 sample and 562 KB on the
  R5 Mark II, inside the 1 MiB `HEAD_LIMIT` prefix. In release, `read_full`
  on them takes ~90-125 ms (the `hpvcd` decode 65-110 ms, the tone map ~7 ms
  over rayon, the JPEG ~17 ms with libjpeg's fastest settings; mozjpeg's
  defaults took ~200 ms), and one `riffle-cli scan` extraction ~150 ms more
  than a JPEG CR3 on one thread.
- `hpvcd` did not panic on 60 randomly corrupted or truncated `PRVW` streams
  (it returned a frame every time), so `hevc::to_jpeg` does not wrap the
  decode in `catch_unwind`; revisit that only if a panic is ever seen. The
  crate declares `rust-version = 1.93` (edition 2024) and has no system
  dependencies, so a toolchain older than 1.93 cannot build the workspace.

## RAF

`crates/core/src/raf.rs` reads a Fujifilm RAF's fixed header and then the one
embedded JPEG it points at; the file's Exif is that JPEG's APP1 segment, read
by `jpeg::read_exif` (the path `jpeg::parse` uses, but an error when the
buffer holds no complete Exif segment), and the AF point from the Fujifilm
MakerNote in that segment (see below). The RAF directory, the FujiIFD and the
sensor data are not read.

### Header words and the JPEG (Measured)

All header words are big-endian `int32u` (ExifTool's `FujiFilm.pm`
`ProcessRAF` / `RAFHeader`): `0x00` the magic `FUJIFILMCCD-RAW ` and a
version (`0201` on every sample), `0x1c` the model name, `0x48` / `0x4c` the
M-RAW header offset / length (0 unless multi-image), `0x54` / `0x58` the
embedded JPEG's offset / length, `0x5c` / `0x60` the RAF directory (tags
`0x100` `RawImageFullSize`, `0x111` `RawImageCroppedSize`, height first).

- `parse` errors on a zero offset or length, on an offset with bit `0x8000`
  set (ExifTool rejects those too), and on an offset past the buffer.
  `preview` and `full` are both that JPEG: no sample carries a second one
  (no MPF segment either).
- Measured on raw.pixls.us RAFs of 22 bodies (2018 X-T3 to 2025 X-T30 III,
  GFX 100 to GFX100RF): the JPEG always starts at `0x94`, is 4416x2944 on the
  X bodies and 4000x3000 on the GFX bodies whatever the sensor (26 to 102 MP)
  or the crop, and is 1.3 to 5.5 MiB, so it never fits the 1 MiB prefix and
  every open costs one ranged read. Its Exif segment ends at byte 65,600, so
  `read_metadata` always parses from the prefix.
- The JPEG is 2.5 to 2.7 times the ARW preview's long edge, so `scan`
  thumbnails a RAF with `thumbnail_jpeg_near` (404 px) as it does a JPEG
  file; the fixed 2/8 scale gave 1000 to 1104 px and ~110 KB thumbnails.
- The Exif carries `Make` `FUJIFILM`, `Orientation` (6 and 8 on the X-H2 and
  X100VI portrait samples) and `SubSecTimeOriginal` only on bodies from about
  2023 (GFX100 II, X100VI, X-T50, X-M5, X-E5, X-T30 III, GFX100RF and
  GFX100S II; not the X-H2, X-H2S, X-S20 or X-T5).
- The 4416x2944 / 4000x3000 sizes above hold for the 2018-and-later bodies
  only. The older CC0 raw.pixls.us bodies (57 bodies in the sweep, every
  file parsed from the 1 MiB prefix with preview and full decode `ok`,
  `0x48` = 0) embed a smaller JPEG: 1920x1280 on most X bodies, 2048x1536
  on the small-sensor X compacts (X10, X20, X30, XF1, XQ1, XQ2, X-S1, the
  EXR bridges), 2176x1448 on the FinePix X100, 1280x960 to 1600x1200 on
  the FinePix S-series and 1344x960 to 1440x960 on the S2/S3/S5 Pro and
  GX680. The X-T30 (4416x2944) and GFX 50S / 50R (4000x3000) match the
  newer sizes. Do not assume "1920x1280 or smaller" for the older
  bodies. `FocusPixel` is still in the JPEG's own frame (a centered point
  is half its size, e.g. 1024,768 on 2048x1536), so `sensor_w` /
  `sensor_h` stay the JPEG's size. The FinePix S2 Pro, S5 Pro and the DBP
  for GX680 write no `FocusPixel` at all, which is just no AF point; the
  X-A2, X-E3, X100F, XF10 and GFX 50S CC0 samples do write one but are
  `FocusMode` 1, so it is dropped. The Exif `Model` of many older FinePix
  bodies carries trailing spaces (e.g. `FinePix S5000 `); the shared
  reader (`exif::read_ifd0`) trims them from `Make` and `Model`, so do not
  add a second trim. Per-body table:
  `docs/plans/_archived/20260930-raf-older-bodies/learnings.md`.

### The prefix / truncation rule (Measured)

The JPEG normally runs past a 1 MiB prefix, so a JPEG that ends past the
buffer is not an error; only its Exif segment has to be there. When the
buffer ends before the Exif segment does (or before the JPEG's segment walk
reaches it), `parse` errors so the reader retries with the whole file; a JPEG
wholly inside the buffer without Exif is orientation 1 and a default `Shot`,
as in `jpeg::parse`.

### The Fujifilm MakerNote (Measured)

Exif IFD tag 0x927c of the embedded JPEG: `FUJIFILM` (8 bytes), then a
little-endian `int32u` offset of its IFD relative to the note start; every
offset in the note is relative to the note start and little-endian, with no
TIFF header (ExifTool's `MakerNotes.pm` `MakerNoteFujiFilm`), so `raf.rs`
walks it with `sequence::Tiff::with_order(.., true)` based at the note start,
whatever the Exif TIFF's own byte order. The note lies inside the Exif
segment, so once `read_exif` found that segment whole, nothing in the note
can be past the buffer: a malformed note is just no AF point, never an error.

- `0x1023 FocusPixel` (`int16u[2]`, x then y) is the AF point in the frame
  of the embedded JPEG (`PixelXDimension` x `PixelYDimension` of the same
  Exif IFD, equal to the JPEG's size on every sample), unrotated, origin top
  left. `raf.rs` stores that frame as `sensor_w` / `sensor_h`, so the
  consumers scale it like any other `FocusLocation`. Drawn on the samples:
  off-center points land on the subject on the X-E5, X-S20, X-T50, X-Pro3,
  X100V and GFX100S II, and on the in-focus flowers of the shallow-depth X-H2
  portrait (Orientation 6); the X100VI portraits (Orientation 8) agree. A
  centered point is (2207..2208, 1472) on 4416x2944 and (1999..2001,
  1499..1501) on 4000x3000, the JPEG's center, not the sensor's. Crop modes
  (the X-M5 "1.25x", the X-T5 "16:9" names) still write a 3:2 JPEG and use
  its frame. An older X-E3 writes a 1920x1280 JPEG and a `FocusPixel` in that
  frame (961, 775 centered).
- `0x1021 FocusMode` (`int16u`): 0 auto, 1 manual. Manual-focus frames still
  write a `FocusPixel` (the X-T3, X-T5 "16:9" and GFX 100 samples), so
  `FocusMode` 1 yields no AF point. The value is not put in
  `Shot.focus_mode`, whose meaning is Sony's.
- No frame size is read: per ExifTool, `0x102d FocusSettings` holds the AF
  area's point / zone size as bit fields, not pixels, so `focus_frame` stays
  `None`.

## ORF

`crates/core/src/orf.rs` reads an OM System / Olympus ORF: IFD0 and the Exif
IFD through `exif::read_ifd0` / `read_exif_ifd`, then the preview from the
Olympus MakerNote's CameraSettings sub-IFD. Nothing else in the note is read
yet.

### The `IIRO` header (Measured)

An ORF is a TIFF whose magic is 0x4f52 (`IIRO` little-endian, `MMOR`
big-endian) instead of 42; some old bodies (the SP-350, the C5050Z) write
0x5352 `RS`. `sequence::Tiff::new` rejects both, so `parse` opens the file
with `Tiff::new_with_magic(.., &[0x4f52, 0x5352])` and rejects 42. Every
raw.pixls.us ORF checked, from the E-1 (2003) to the OM-5 Mark II (2025), is
`IIRO` except those two.
Olympus pads IFD0's `Make` and `Model` with spaces (`"OM-1            "`);
the shared reader (`exif::read_ifd0`) trims them, so `parse` does not.
ORF bodies write a correct IFD0 `Orientation`: the XZ-10 (6) and the E-30 (8)
previews come out upright once rotated per the tag, so no per-body handling
is needed. No raw.pixls.us sample of a current OM / Olympus body is portrait
(only the XZ-10, the E-30 and the C5050Z are not 1), so those two older
bodies are the only portrait check.
raw.pixls.us answers `/data/...` with a 301 to `/download/...`, so fetch
samples with `curl -L` or the file comes back empty; ranged requests do not
work, but `curl -L ... | head -c 4096` reads a header cheaply.

### Three MakerNote headers, note-relative or absolute offsets (Measured)

Exif IFD 0x927c opens with `OLYMPUS\0` + `II` / `MM` + a 2-byte version
(IFD at note + 12; Olympus bodies write version `03 00`) or with
`OM SYSTEM\0\0\0` + `II` / `MM` + version (IFD at note + 16; the OM Digital
Solutions bodies write `04 00`). There is no TIFF header in the note, and
every offset in it (the sub-IFD pointers, `PreviewImageStart`) is relative to
the **note start**, not the file's TIFF header (ExifTool's `Olympus.pm`
`Base => '$start - 12'` / `'$start - 16'`). The byte order is the note's own
`II` / `MM`, so `parse` builds the note walker with `Tiff::with_order`. A note
with none of the three headers gives no preview and no error. The sub-IFDs
(0x2010 Equipment, 0x2020 CameraSettings, 0x2040, 0x2050) are type 13 (IFD)
count 1 with the offset inline on every body with these two headers.

The 2003-2006 Four Thirds bodies (E-1, E-300, E-330, E-400, E-500) and the
older compacts write a third header: `OLYMP\0` + a 2-byte version (`02 00` on
the DSLRs), IFD at note + 8. It has no byte order of its own (the note follows
the file's), and every offset in it is **absolute**, relative to the file's
TIFF header like IFD0's (ExifTool's `Olympus.pm`: `Start => '$valuePtr + 8'`
and no `Base`). So `parse` walks it with a base-0 walker in the file's byte
order and takes `PreviewImageStart` as is. Its 0x2020 CameraSettings is type 7
(`undefined`), count 384 on the E-300: the sub-IFD is written inline at the
entry's value offset, which `parse` accepts as it does a type 13 / 4 pointer.

### The preview is the only JPEG, and it lives inside the note (Measured)

CameraSettings 0x0100 `PreviewImageValid`, 0x0101 `PreviewImageStart`
(note-relative, or absolute in an `OLYMP\0` note), 0x0102
`PreviewImageLength`: a 3200x2400 JPEG on every body surveyed with the two
newer headers (14 bodies, 2008 E-30 to 2025 OM-5 Mark II), 1600x1200 on the
E-300, E-330, E-400 and E-500 and 1280x960 on the E-1. IFD0 has no 0x0201 and
its strips are the raw data, and a scan for SOI + SOF finds no other JPEG at
least 1000 px wide (the old bodies' other JPEG is the 160x120 thumbnail), so
`full` is the same JPEG and the 1:1 view is limited to the preview. The
`OLYMP\0` compacts (C5050Z to C8080WZ, E-10, E-20, the SP-series; versions
`01 00` / `02 01`) have no 0x2020 at all, only the 160x120 thumbnail, so they
have no preview and fail with `no embedded preview`.

AF values seen (not read yet): `AFTargetInfo` on the OM-1, OM-1 Mark II and
OM-5 Mark II holds a frame size plus focus / selected boxes; the OM-5 and OM-3
hold zeros after the frame size. `AFPointSelected` is nonzero on the PEN-F,
E-M1X, E-M5 Mark III, E-M10 Mark IV, E-P7 and OM-5 Mark II (for example E-P7
`320/640, 305/480`). Every value is near the center, so no raw.pixls.us
sample confirms the origin; an off-center sample is needed before reading an
AF point.

### The note is longer than the prefix (Measured)

The note holds the preview, so it is 1.45 to 1.82 MB long and runs past the
1 MiB `HEAD_LIMIT` on every body surveyed but the XZ-10 (whose 146 KB note
ends before its preview starts); the preview ends between 0.97 and 1.29 MB,
past the prefix on 9 of the 14 bodies. So, unlike `nef::af_point`, `parse` does
not demand the whole note in the buffer: the walker's end is
`min(note start + count, buf.len())`, and only the header, the main IFD and
the CameraSettings IFD (all within ~12 KB of the file start) must fit. When
the buffer cuts one of them, `parse` errors so the reader retries with the
whole file; a note wholly in the buffer that still does not read gives no
preview. The preview may lie past the declared note too (the XZ-10's does)
and is range-read like any other. The `OLYMP\0` notes are about 3 KB and hold
no preview; their previews lie after them and end below 450 KB, inside the
prefix.

## Related

- The detector that consumes these AF points runs on the upright preview and
  maps its results back to stored coordinates; see
  [Feed an upright image, map detections back to stored coordinates](./tract-onnx-inference.md#feed-an-upright-image-map-detections-back-to-stored-coordinates-hit).
