<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Close four camera-support todo items on public samples

## Purpose

The 2026-11-27 raw.pixls.us survey left four camera-support items in
`todo.md`: Pentax and Ricoh DNGs show no lens name, the Leica M10 shows no
aperture and four phone DNGs show no shooting settings, nobody measured what
reading the full-size JPEG of a Pentax / Ricoh DNG costs, and the Nikon
DSLRs' `AFInfo2` `0100` / `0101` is not read. This plan closes each on the
public samples, checked against `exiftool`, so the meta pane shows the lens
and the aperture on those bodies and the todo list holds only what still
needs a sample nobody has.

Common rules for every step:

- Samples are untrusted data. Download each into its own new, empty
  directory under the session scratchpad (`samples/<body>/`), never into the
  repository; keep scripts elsewhere; run `exiftool` and the CLI with the
  sample path as an argument, from another directory. Fetch with `curl -L`
  from `https://raw.pixls.us/getfile.php/<id>/nice/<name>` (ids below; the
  `/data/...` URLs answer with a 301). No sample file is committed; tests use
  synthetic bytes as `crates/core/src/arw.rs` does.
- No load tests and no repeated measurement loops: the host is shared
  (`CLAUDE.md`).
- Each step's PR removes its own item from `todo.md` (Step 4 edits a
  checkbox instead).
- `docs/humans/cameras.md` and `docs/humans/cameras.ja.md` change together.
- `EXTRACTOR_VERSION` (`crates/engine/src/index.rs`) is bumped to the next
  free number above main's whenever a step changes what pass 1 stores, with
  a sentence added to its doc comment (`docs/agents/shared-crates.md`,
  "Bump `EXTRACTOR_VERSION`...").
- A local release build of `crates/cli` alone (not the workspace, not
  `ci:local` on the whole repository) is allowed, once per step as needed, to
  check the code against `exiftool` on the samples and for Step 3's single
  measurement (see "Decisions").

## Decisions

Taken on 2026-11-28 before Step 1, each the recommended option of
"Trade-offs and risks":

- Step 4: option B. Ship the live view (contrast-detect) coordinates and "no
  point used means no AF point"; keep mapping a phase-detect grid point to a
  position in the Tier 2 checkbox until an off-center AF sample exists.
- Step 1: embed the whole `%pentaxLensTypes` table with its source and
  ExifTool version noted next to it, as the repository's other ExifTool value
  ports do; show ExifTool's class names (`K or M Lens` and the like) as it
  does.
- Step 2: the M10's note `FNumber` goes into `estimated_f_number`
  (`f/5.6 (est.)`).
- Step 3: the threshold is the RAF pass 1 figure already accepted (89.7 ms
  per file); a result near it is recorded as inconclusive with no code.
- `EXTRACTOR_VERSION`: bumped by each step that changes what pass 1 stores.
- Local builds: the one `crates/cli` release build per step above is
  allowed, since the checks against the samples cannot run on CI.

## Steps

- [ ] Step 1: Read the lens name of Pentax and Ricoh DNGs from the Pentax MakerNote
  - Done when:
    - `reader::read_metadata` gives a `shot.lens_model` equal to `exiftool`'s
      `LensType` on each sample below, and a DNG with an Exif `LensModel`
      keeps that one.
    - Synthetic-byte tests cover: a `PENTAX \0` note and a `RICOH\0` note,
      both byte orders, an unknown lens id (no lens name), an Exif
      `LensModel` winning, a note cut by the buffer, and a non-Pentax
      `DNGPrivateData`.
    - The "no lens name" half of the DNG paragraph is gone from
      `docs/humans/cameras.md` (about line 213) and `docs/humans/cameras.ja.md`
      (the M10 half stays until Step 2).
    - `EXTRACTOR_VERSION` is bumped (the index stores `lens_model`).
    - `docs/agents/raw-metadata-parsing.md` has a Pentax / Ricoh MakerNote
      section, `CLAUDE.md`'s layout names the new reading (and the new
      module, if one is added), and the todo item
      `### Core: Pentax and Ricoh DNGs show no lens name` is deleted.
    - `mise run ci:local` passes where the session runs local checks, and the
      PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Measured on the nine samples with exiftool 13.59: these DNGs have **no**
      Exif IFD MakerNote (0x927c). The note is IFD0 tag 0xc634
      (`DNGPrivateData`, type BYTE, count = note length, 121 to 195 KB),
      starting at file offset 0x302 to 0x3be, so it lies inside the 1 MiB
      `HEAD_LIMIT` prefix. All nine files are little-endian.
    - Two headers: `PENTAX \0` + `II` / `MM`, IFD at note + 10 (ExifTool's
      `MakerNotePentax5`; K-1, K-1 Mark II, K-3 Mark III, K-3 Mark III
      Monochrome, K-70, KP) and `RICOH\0` + `II` / `MM`, IFD at note + 8
      (`MakerNoteRicohPentax`; GR III, GR IIIx). Offsets inside the note are
      relative to the note start (K-1: `PreviewImageStart` 76930 + note
      0x39e = 77856, the value exiftool reports). Read the byte order from
      the header, not the container; confirm `Start` / `Base` in ExifTool's
      `MakerNotes.pm`.
    - The lens is tag 0x003f `LensRec`: BYTE count 4, inline in the entry on
      every sample; bytes 0 and 1 are the series and the model (ExifTool's
      `Pentax.pm` `LensRec`, `LensType` `int8u[2]`). Accept a count of 2 to 4
      (older bodies write fewer bytes per `Pentax.pm`); read the inline bytes
      with `inline_bytes`, never as an offset.
    - Expected values (`exiftool -LensType`, raw id in parentheses):

      | Sample (raw.pixls.us id) | `LensType` |
      |---|---|
      | Pentax K-1 (2470) | smc PENTAX-DA 35mm F2.4 AL (7 214) |
      | Pentax K-1 Mark II (3345) | HD PENTAX-D FA 24-70mm F2.8 ED SDM WR (8 62) |
      | Pentax K-1 Mark II (7134) | smc PENTAX-FA 43mm F1.9 Limited (4 3) |
      | Pentax K-3 Mark III (4678) | Sigma 24-70mm F2.8 IF EX DG HSM (8 7) |
      | Pentax K-3 Mark III Monochrome (7081) | K or M Lens (1 0) |
      | Pentax K-70 (1142) | smc PENTAX-DA 18-135mm F3.5-5.6 ED AL [IF] DC WR (8 215) |
      | Pentax KP (1824) | HD PENTAX-DA 16-85mm F3.5-5.6 ED DC WR (8 200) |
      | Ricoh GR III (3115) | 18.3mm F2.8 (31 1) |
      | Ricoh GR IIIx (5818) | 26.1mm F2.8 (31 4) |

    - The id table: embed the whole `%pentaxLensTypes` table of `Pentax.pm`
      keyed by the two integers (about 267 entries, about 19 KB of source),
      as a sorted `const` slice searched by binary search. Drop the decimal
      alternates (`'3 23.1'` and the like, 45 entries): they name other
      lenses sharing an id, which ExifTool only resolves in its Composite
      `LensID` from the focal length. Do not port the `OTHER` fallbacks
      (series 4 as 7, 7 as 8, 11 as 13) unless a sample needs one. An id
      not in the table gives no lens name. Record the ExifTool version the
      table was taken from next to it.
    - Fill `shot.lens_model` only when the Exif IFD gave none.
    - Follow the guide's range-vs-contents split: a note whose header or IFD
      is cut by the buffer is an error (so the reader's whole-file retry
      runs), a note wholly in the buffer that does not read gives no lens
      name. Gate on the header bytes, not on `Make` (all nine write
      `RICOH IMAGING COMPANY, LTD.`); another maker's `DNGPrivateData`
      (Adobe-converted DNGs write `Adobe\0`) is skipped silently.
    - Placement: `arw.rs` already holds the Sony, Leica and Sigma notes and is
      1875 lines; a small `crates/core/src/pentax.rs` (note walker + table)
      called from `arw::parse_noted` keeps the table out of it. Either way
      `reader.rs` stays free of maker-specific parsing.
    - If `handpic-cli info` does not print the lens, add the line there
      rather than writing a one-off tool.
    - The filter menu's `Lens` section and the meta pane read the stored
      `lens_model`; nothing in `crates/app` should need a change.

- [ ] Step 2: Read the Leica M10's aperture and the phone DNGs' Exif tags in IFD0
  - Done when:
    - The M10 sample (id 1603) shows an aperture of 5.6, as `exiftool`'s
      Leica `FNumber` does, and the M10-R sample (id 7853) still shows the
      5.6 of its `ApertureValue`.
    - `reader::read_metadata` on the HUAWEI ANE-LX1 (4099), OnePlus A6003
      (4128), Xiaomi M2010J19CG (4233) and Google Pixel 2 XL (2206) samples
      returns the exposure time, f-number, ISO, focal length and capture time
      `exiftool` reports (table below).
    - Synthetic-byte tests cover: a Leica `02 00` note's 0x035a used only
      when the Exif IFD has neither `FNumber` nor `ApertureValue`, another
      Leica note version ignored, the shooting tags read from an IFD0 with
      no Exif IFD, and an Exif IFD value winning over IFD0's.
    - The "no aperture" sentence is gone from both cameras docs (with Step 1
      merged, the whole "record them only in their MakerNote" sentence
      goes).
    - `EXTRACTOR_VERSION` is bumped; the guide's Leica section is corrected
      (it says the note's `FNumber` is not read) and gains the IFD0 Exif
      layout; the todo item
      `### Core: Leica M10 shows no aperture, and four phone DNGs show no shooting settings`
      is deleted.
    - `mise run ci:local` passes where the session runs local checks, and the
      PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - M10, measured: the note is at 0x1800 (4096 bytes, inside the prefix),
      header `LEICA\0 02 00` (ExifTool's `MakerNoteLeica9`, "M10 and S"),
      little-endian IFD at note + 8. Tag 0x035a `FNumber` is type SLONG
      (`int32s`), count 1, inline, value 5600; ExifTool divides by 1000.
      `arw::integer` accepts only SHORT and LONG, so the SLONG needs its own
      read.
    - The M10-R's note holds the same 5600 and its `ApertureValue` is 5.6.
      But the M11-P samples of `../_archived/20260918-leica-dng-support/plan.md`
      carry 1.0 in the note's `FNumber`, which is why the guide says it is
      not trusted. So: read 0x035a only from a note whose bytes 6 and 7 are
      `02 00`, and only when the Exif IFD has neither `FNumber` nor
      `ApertureValue`; ignore a value of 0 or less. Store it as
      `estimated_f_number`.
    - Extend the existing `leica_focus_distance` read rather than walking the
      note a second time. The far-note path (`FarNote::take`) serves the
      M (Typ 240) note (`02 ff`), which has no 0x035a; leave it unless a
      `02 00` note is found past the prefix.
    - Phones, cause found: the four DNGs have no Exif IFD (no 0x8769). The
      shooting tags sit directly in IFD0, as TIFF/EP allows and Android's
      `DngCreator` writes: 0x829a `ExposureTime` and 0x829d `FNumber`
      (RATIONAL), 0x8827 `ISO` (SHORT count 1), 0x9003 `DateTimeOriginal`
      (ASCII 20), 0x920a `FocalLength` (RATIONAL). `arw::exif` returns right
      after `Make` / `Model` when IFD0 has no 0x8769.
    - Fix: run the same tag match over IFD0's entries first, then over the
      Exif IFD's, so an Exif IFD value overrides IFD0's; keep the
      `f_number` / `estimated_f_number` cleanup after both.
    - Expected values:

      | Sample | Exposure | F-number | ISO | Focal length | `DateTimeOriginal` |
      |---|---|---|---|---|---|
      | HUAWEI ANE-LX1 | 1/385 | 2.2 | 50 | 3.8 mm | 2020:09:20 11:25:15 |
      | OnePlus A6003 | 1/1064 | 1.7 | 100 | 4.2 mm | 2020:10:19 09:36:59 |
      | Xiaomi M2010J19CG | 1/193 | 1.8 | 100 | 4.7 mm | 2020:12:24 14:13:23 |
      | Google Pixel 2 XL | 1/2127 | 1.8 | 51 | 4.5 mm | 2018:01:09 08:06:29 |

      The exposure times are stored as `n/1000000000`; the engine's `shutter`
      already prints `1/<rounded>`.
    - The four phones stay unlisted in the cameras docs: their only image is
      the uncompressed raw, so they still fail with `no embedded preview`.
      Their error rows now carry the shooting settings (the index keeps a
      failed extraction's metadata since extractor version 13).
    - Confirm the already-listed DNG bodies (Samsung, Pixel 7 Pro / 8 Pro,
      iPhone 12 Pro, Leica, Sigma) read the same values as before: the IFD0
      pass must not change a file that has an Exif IFD.

- [ ] Step 3: Measure what the full-size JPEG of Pentax and Ricoh DNGs costs, and act on it
  - Done when (both outcomes):
    - Pass 1 and one preview read + decode are measured **once each** on a
      folder of the Pentax / Ricoh samples and on a folder of a small-preview
      body, and the numbers, the date, the machine and the caveat (shared
      host, one run) are in `docs/humans/performance.md`,
      `docs/humans/performance.ja.md` and this plan's `learnings.md`.
    - The decision and its reason are written in `learnings.md` and the
      guide's new Pentax section.
    - The todo item
      `### Core: Pentax and Ricoh DNGs read the full-size JPEG for every thumbnail and preview`
      is deleted.
    - `mise run ci:local` passes where the session runs local checks, and the
      PR's GitHub checks pass.
  - Done when, if the cost does **not** show: no file under `crates/` changes.
  - Done when, if the cost shows:
    - Pass 1 makes the thumbnail of a Pentax / Ricoh DNG from the MakerNote's
      JPEG; the preview, the 1:1 view and pass 2 still read the full-size
      JPEG.
    - Synthetic-byte tests cover the note JPEG taken for the thumbnail, a
      note JPEG running past the file ignored, and a file without the note
      unchanged.
    - `EXTRACTOR_VERSION` is bumped (thumbnail bytes change), the
      re-measured pass 1 number is recorded next to the first, and
      `CLAUDE.md`'s layout describes the thumbnail tier.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (the note walker).
    - Size check, already measured: the MakerNote preview (tags 0x0002
      `PreviewImageSize`, 0x0003 `PreviewImageLength`, 0x0004
      `PreviewImageStart`, note-relative) is **720x480** on all nine samples
      (13 to 57 KB). That is under the 1280 px floor, so it can never be the
      preview; it can only make the 404 px thumbnail
      (`JPEG_THUMBNAIL_LONG_EDGE`). A page turn and pass 2 cannot gain from
      it, so the only code this step can add is a pass 1 thumbnail source.
    - Measurement, one run each, release build of `crates/cli` only:
      `handpic-cli scan <dir> 1` for pass 1 (per-file mean and p95) and
      `handpic-cli bench <files>` for the preview read and decode, on the
      nine Pentax / Ricoh DNGs and on a folder of Sony ARWs (or whichever
      small-preview samples are at hand). Do not use the 111 to 173 MB pixel
      shift samples (ids 1753, 2474, 3348, 7128, 7132) unless needed.
    - Yardstick: RAF is the same case (one 4416x2944 JPEG serves as preview
      and full) and is accepted as is; `docs/humans/performance.md` records
      its pass 1 at 89.7 ms against ARW's 31.2 ms per file and its page turn
      at about 65 ms against 21 ms. The Pentax / Ricoh JPEGs are 6000x4000
      to 7360x4912 (2.9 to 6.4 MB), 1.9 to 2.8 times the RAF's pixels.
    - If code is added: a third tier on `Arw` (a thumbnail `Embedded`) filled
      from the note, a `reader` read for it, and `scan::extract_unless`
      reading it instead of the preview. `Embedded` offsets are file offsets,
      so add the note's start to `PreviewImageStart`. Check the JPEG's
      orientation against the full JPEG's on a portrait sample if one
      exists; none of the nine was checked for it.

- [ ] Step 4: Read the AF point of Nikon `AFInfo2` versions `0100` / `0101` as far as the samples confirm it
  - Done when:
    - `nef::af_point` reads version `0101` (and `0100` where the layout is
      the same): the contrast-detect (live view) AF area position, and no AF
      point when `AFPointsUsed` is empty.
    - On every D850 and D500 raw.pixls.us sample the result is no AF point,
      matching `exiftool`'s `AFPointsUsed` `(none)`; on the D5 live view
      sample (id 2218) the point equals `exiftool`'s `AFAreaXPosition` /
      `AFAreaYPosition` in the `AFImageWidth` x `AFImageHeight` frame.
    - The phase-detect grid point mapping stays in `todo.md` (option B of
      "Decisions").
    - Synthetic-byte tests cover each branch added, a note too short for the
      fields, and `0300` / `0400` unchanged.
    - `EXTRACTOR_VERSION` is bumped if any file can now get an AF point it
      did not have; the guide's `AFInfo2` section describes `0100` / `0101`.
    - `docs/humans/cameras.md` line 153-155 and its Japanese pair no longer
      say the D850 and D500 "write an older `AFInfo2` that Handpic does not
      read"; they say what is read and that no sample confirmed a position.
      The D850 / D500 rows keep `–` for AF point unless a sample showed one.
    - In `todo.md`'s
      `### Core: widen camera support from public sample RAW files`: the
      Tier 2 checkbox drops the done part and keeps the rest (the Z 8 / R6
      AF samples, the X-T3 / GFX 100 samples, ORF, and the DSLR phase-detect
      grid mapping), and "Riffle" becomes "Handpic" in its two places
      ("Riffle reads ARW, DNG, ..." and "Riffle can already read").
    - `mise run ci:local` passes where the session runs local checks, and the
      PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Measured: both the D850 and the D500 write version `0101`
      (ExifTool's `AFInfo2V0101`: D5, D500, D850, D3400, D3500, D7500).
      `0100` (`AFInfo2V0100`) is the 2007-2015 DSLRs; neither named body
      writes it.
    - `0101` layout (`Nikon.pm`): byte 4 `AFDetectionMethod` (0 phase
      detect, 1 contrast detect), byte 5 `AFAreaMode`, byte 6
      `FocusPointSchema` (0 off, 1 51-point, 2 11-point, 7 153-point), byte
      8 `AFPointsUsed` (153-point: 20 bytes, bit `j` of byte `i` is point
      `i * 8 + j + 1` of `%afPoints153`, 9 rows A-I by 17 columns, E9 the
      center), 0x44 `PrimaryAFPoint`, and, valid only for contrast detect,
      `int16u` at 0x46 `AFImageWidth`, 0x48 `AFImageHeight`, 0x4a / 0x4c
      `AFAreaXPosition` / `YPosition` (the area's center), 0x4e / 0x50
      `AFAreaWidth` / `Height`. Read `0100`'s offsets from its own table;
      do not assume they match.
    - Every D850 sample (ids 1835-1842) and every D500 sample (2504-2518) is
      **manual focus**, phase detect, `AFPointsUsed` `(none)`;
      `PrimaryAFPoint` is E9 on the D850 and G11 on the D500 (the selected
      point, not a focused one). ExifTool notes `PrimaryAFPoint` reports a
      value even when focus failed, so use `AFPointsUsed`, never
      `PrimaryAFPoint` alone. These samples can therefore confirm only "no
      AF point", not a position.
    - Other `0101` samples on raw.pixls.us: D5 id 1636 (AF-S, point 1 = E9
      center), D5 id 2218 (AF-S, contrast detect on: the one sample with
      coordinates), D7500 ids 1531-1534 (AF-C, 51-point, point 1 = center),
      D3400 / D3500 (11-point). Every phase-detect point is the center, where
      any centered grid geometry gives the image center, so no public sample
      confirms a grid geometry.
    - The contrast-detect branch has the same shape as the `03xx` / `04xx`
      read (`AFImage` frame, top-left origin); reuse the existing mapping
      and the `CropHiSpeed` / `CropArea` handling in `af_point`.
    - `AF_COORDINATES_AVAILABLE` (byte 7) belongs to `03xx` / `04xx`; in
      `0101` byte 7 is not that flag. Gate on `AFDetectionMethod` instead.
    - Keep the note's range-vs-contents rule as it is in `af_point`.

## Trade-offs and risks

- **Step 4: the grid geometry cannot be confirmed (decided: B).** A
  153-point index is a name (E9, G11), and ExifTool holds no pixel position
  for it; the D850 (FX) and D500 (DX) cover different parts of the frame with
  the same module. Every public D850 / D500 sample is manual focus with no
  point used. Options:
  - A: add a per-body geometry (D5, D500, D850; optionally 51-point and
    11-point bodies) from an external source, for example the per-camera
    point maps of the open-source Lightroom "Focus Points" plugin, after
    checking its license, and mark it Inferred. The AF mark and pass 2's
    sharpness window then appear on DSLR files, unverified: a wrong
    geometry scores the wrong part of the frame, which is worse than no AF
    point (pass 2 falls back to a detected face or the sharpest tile).
  - B: ship only what the samples confirm (the live view coordinates and
    "no point used means no AF point") and keep "map a phase-detect grid
    point to a position, once an off-center AF sample exists" in the Tier 2
    checkbox.
- **Step 4 scope.** The request names `0100` / `0101` "(D850, D500)". Both
  bodies write `0101`. Covering `0100` adds the 51-, 39- and 11-point
  schemas of about twenty older bodies, most released before 2016 and so
  outside the support policy; its contrast-detect coordinates are cheap to
  add, its grids are the same open question.
- **Step 1: table size (decided: whole table).** Whole table (about 267 ids,
  19 KB of source, no maintenance decision per lens) against only the ids the
  samples carry plus current K-mount lenses (smaller, but every other lens
  shows nothing and the list needs curating by hand).
- **Step 1: license of the table (decided: embed with the source noted).**
  `Pentax.pm` is under Perl's terms (GPL or Artistic); this repository is
  MIT / Apache-2.0. The repository already ports ExifTool's value tables (the
  Sony labels in `crates/engine/src/exif.rs`, `DSC_EXCEPTIONS`), and
  id-to-name pairs are facts, but a 267-row table is a larger taking than
  those.
- **Step 1: class names as lens names (decided: shown).** ExifTool prints
  `K or M Lens`, `A Series Lens`, `M-42 or No Lens` and `Sigma` for ids that
  name a lens class. Showing them matches `exiftool`; hiding them keeps the
  lens row and the filter's `Lens` section to real lens names. Shared ids
  (`Sigma or Tamron Lens (3 44)` style entries) show ExifTool's generic first
  name either way.
- **Step 2: M10 aperture as an estimate (decided: estimate).** An M body has
  no aperture coupling; the note's value is the camera's guess, like the
  M10-R's `ApertureValue`, which Handpic shows as `f/5.6 (est.)`.
- **Step 2: the note's `FNumber` of 1.0.** The M11-P samples carry 1.0
  there. The gate (only without both Exif tags) keeps them unaffected, but a
  `02 00` body with neither Exif tag and a 1.0 in the note would show
  `f/1 (est.)`. No such sample is known; a real f/1 lens exists, so the
  value is not filtered.
- **Step 3: what "the cost shows" means (decided: the RAF figure).** No code
  when pass 1 per file stays within about the RAF figure already accepted
  (89.7 ms on the same host class); code when it is clearly beyond it. One
  run on a shared host is rough, so a result near the line is recorded as
  inconclusive and left without code.
- **`EXTRACTOR_VERSION` bumps.** Steps 1, 2 and 4 each bump (Step 3 only
  with code). Each bump re-extracts every indexed folder on its next open; a
  user who updates between the PRs sees it up to four times, one who gets
  them in one release sees it once.
- **Local builds (decided: allowed for `crates/cli`).** Checking the code
  against `exiftool` on the samples and Step 3's measurement need a local
  release build of `crates/cli`, which the PR's checks cannot replace since
  they cannot see the samples.
- **Samples may change.** The ids and values above were read on 2026-11-28
  with exiftool 13.59 from copies fetched on 2026-11-27; re-run `exiftool`
  on the fresh download rather than trusting the tables.
- **`cameras.ja.md` drift.** The Japanese file still lags the English one
  (its own todo item); change only the sentences these steps touch.

## Progress
