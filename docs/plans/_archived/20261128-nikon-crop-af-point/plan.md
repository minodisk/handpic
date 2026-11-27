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

# Nikon crop-mode AF point mapping

## Purpose

A Nikon Z body shooting in a crop mode (DX on an FX body, 1:1, 16:9) writes
`AFInfo2`'s `AFImageWidth` / `AFImageHeight` at the uncropped sensor size
while the embedded JPEGs hold only the crop. `nef::af_point` hands that frame
to the consumers as it is, and they scale the point proportionally onto the
JPEG, so an off-center AF point lands in the wrong place (a point at sensor x
is drawn at `x / 6048` of a 3984-wide DX JPEG instead of `(x - 1032) / 3984`).
A centered point lands at the center either way, which is why the samples
looked right.

After this work the AF mark, the 1:1 view and the sharpness score of a
crop-mode NEF sit on the AF point, the Z6III, Z50II and Z5II are listed in the
camera table, and the todo entry is narrowed to the one check the samples
cannot give: an off-center AF point on a real crop frame.

Measured on raw.pixls.us samples (2026-11-27 survey):

| Body, mode | `AFImage` frame | Full JPEG = `CropArea` w x h | AF position |
|---|---|---|---|
| Z6III FX | 6048x4032 | 6048x4032 | (3023, 2016) |
| Z6III DX | 6048x4032 | 3984x2656 | (3023, 2016) |
| Z6III 1:1 | 6048x4032 | 4032x4032 | (3023, 2016) |
| Z6III 16:9 | 6048x4032 | 6048x3400 | (3023, 2016) |
| Z50II uncropped | 5568x3712 | 5568x3712 | (2789, 1868) |
| Z50II 1:1 | 5568x3712 | 3712x3712 | (2792, 1868) |
| Z50II 16:9 | 5568x3712 | 5568x3128 | (2789, 1865) |
| Z5II (1:1, DX, FX) | none (`AFCoordinatesAvailable` 0) | 4032x4032 / 3984x2656 / 6048x4032 | none |

`AFInfo2` version `0401` on the Z6III, `0402` on the Z50II and Z5II.

Sample files (not in the repository, named by raw.pixls.us getfile id):
`C:\Users\daisu\AppData\Local\Temp\claude\4d0f8a7a7e14c812\109664e4-4c19-433a-9e5f-07dfe5e7c936\scratchpad\samples\nikon_dx\`,
survey notes in `survey-report.md` of the same scratchpad ("Nikon crop NEFs").

## Decisions (made with the user, 2026-11-28)

- The crop is detected from the MakerNote (`CropHiSpeed` + `CropArea`,
  variant A below), not from the JPEG's size. The requested rule "the JPEG is
  smaller than the AF frame on each side" cannot be evaluated in `nef.rs` (no
  JPEG size there) and would misread the uncropped Z 6's reduced-size
  JpgFromRaw as a crop.
- The mapping itself is as requested: a center-crop offset, the AF frame size
  not scaled, a point outside the crop giving no AF point. Reason: the Z crop
  modes cut the sensor's center without downscaling, so an offset is
  physically right and scaling would shift every off-center point.
- `EXTRACTOR_VERSION` is bumped.
- Row names follow Nikon's spelling: `Z6III`, `Z50II`, `Z5II`.
- The sample check is one local `cargo build -p handpic-cli` and a run of
  `info` / `focusbox` on the samples, no loop, no load.

## Steps

- [x] Step 1: Map a crop-mode NEF's AF point into the cropped JPEG, list the
      three bodies, narrow the todo entry
  - Done when:
    - `nef::af_point` returns, for a frame the MakerNote declares a crop, a
      `FocusLocation` whose `sensor_w` / `sensor_h` are the crop's size and
      whose `x` / `y` are the `AFInfo2` position minus the centered offset
      `((AFImageWidth - crop width) / 2, (AFImageHeight - crop height) / 2)`;
      the AF frame size (`FocusFrame`) is passed through unscaled.
    - A mapped position outside the crop (left of / above the offset, or at
      or past the crop's width / height) gives no AF point and no frame.
    - An uncropped frame, and a NEF without the crop tags, give exactly what
      they give today, including a reduced-size JpgFromRaw (Z 6: 6048x4024
      frame, 3024x2016 JPEG), which the consumers' scaling keeps handling.
    - Unit tests in `crates/core/src/nef.rs`, on synthetic bytes in the style
      of the existing `af_info2` / `maker_note` helpers, cover: an off-center
      point on a DX crop (for example frame 6048x4032, crop 3984x2656, point
      (4000, 1000) giving 3984x2656 at (2968, 312)); a point outside the crop
      giving `None`; an uncropped `CropHiSpeed` beside a smaller JPEG leaving
      the frame untouched; a crop declared by `CropHiSpeed` without a
      readable `CropArea` leaving the frame untouched; both byte orders.
    - `EXTRACTOR_VERSION` in `crates/engine/src/index.rs` is bumped with a
      sentence for the new number in its doc comment.
    - `docs/humans/cameras.md` and `docs/humans/cameras.ja.md` both hold the
      rows `Nikon Z6III | ✓ | ✓ | – | ✓`, `Nikon Z50II | ✓ | ✓ | – | ✓` and
      `Nikon Z5II | – | – | – | ✓`, and their Canon / Nikon paragraph says
      the Z5II samples carried no AF position, so it is unconfirmed.
    - `docs/agents/raw-metadata-parsing.md`'s `AFInfo2` section describes the
      crop mapping and the measured values.
    - The todo.md section "Core: Nikon DX-crop and non-standard-crop AF point
      mapping is unverified" states what is now done and keeps one unchecked
      item: verify the offset assumption with a crop sample whose AF point is
      off-center.
    - The PR body states the decision (center-crop offset, no scaling of the
      AF frame, outside the crop means no AF point, the crop read from
      `CropHiSpeed` / `CropArea`) and its reason.
    - The sample check gives the expected focus lines below, or the
      difference is recorded in `learnings.md`.
    - The PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/core/src/nef.rs`, `crates/engine/src/index.rs`,
      `docs/humans/cameras.md`, `docs/humans/cameras.ja.md`,
      `docs/agents/raw-metadata-parsing.md`, `todo.md`. `CLAUDE.md` needs no
      change: its `nef.rs` clause names only the JPEG SubIFDs.
    - `nef::parse` cannot see the JPEG's size: the JPEG SubIFDs carry no
      `ImageWidth`, and the full JPEG's SOF is usually past the reader's
      1 MiB prefix. Take the crop from the Nikon MakerNote IFD `af_point`
      already walks (`entries`, the same `note` `Tiff`, note-relative
      offsets):
      - `CropHiSpeed`, tag 0x001b, `int16u[7]`: mode, original width,
        original height, cropped width, cropped height, x, y. The frame is a
        crop when the cropped size is no larger than the original on both
        sides and smaller on at least one.
      - `CropArea`, tag 0x0045, `int16u[4]`: left, top, width, height. Its
        width and height equal the full JPEG's size on all 14 samples
        (3984x2656, 4032x4032, 6048x3400, 3712x3712, 5568x3128, and the
        uncropped sizes).
    - Apply the mapping only when `CropHiSpeed` declares a crop, `CropArea`'s
      width / height are nonzero, no larger than the `AFImage` frame on both
      sides and smaller on at least one. Anything else (tag missing, wrong
      type or count, uncropped) falls through to today's return value. Keep
      the lenient style of the function: an unreadable crop tag is "no crop",
      never an error. The existing range rule (a MakerNote running past `buf`
      is an error) is unchanged.
    - `CropHiSpeed` is the trigger rather than the JPEG's size because an
      uncropped body can hold a JPEG smaller than the AF frame: the Z 6
      sample of the camera table (raw.pixls.us file 3587) has a 6048x4024
      frame, a 3024x2016 JpgFromRaw and `CropHiSpeed` `11 6064 4040 6064 4040
      0 0`. Whether that body writes `CropArea` at all was not checked; the
      rule does not depend on it.
    - The centered offset is backed by metadata: on every crop sample,
      `CropHiSpeed`'s origin plus the crop's `CropArea` left / top, minus the
      uncropped frame's `CropArea` left / top, equals
      `(AFImage - CropArea) / 2` exactly ((1032, 688) Z6III DX, (1008, 0)
      Z6III 1:1, (0, 316) Z6III 16:9, (928, 0) Z50II 1:1, (0, 292) Z50II
      16:9). The offset form is used because the uncropped margins are not in
      a crop frame's own file.
    - Expected `handpic-cli info` focus lines after the change: Z6III DX
      `3984 2656 1991 1328`, Z6III 1:1 `4032 4032 2015 2016`, Z6III 16:9
      `6048 3400 3023 1700`, Z6III FX `6048 4032 3023 2016` (unchanged),
      Z50II 1:1 `3712 3712 1864 1868`, Z50II 16:9 `5568 3128 2789 1573`,
      Z50II uncropped `5568 3712 2789 1868` (unchanged), Z5II `focus: -`.
      `focusbox` should draw the box at the JPEG's center on each, as it does
      today.
    - Tests: add `Field` builders for the two tags next to `af_info2` in the
      `tests` module (SHORT arrays in the note's byte order) and pass them to
      the existing `af(file, note, fields)` helper. Leave the existing tests'
      expectations as they are; they have no crop tags and must still pass.
    - `EXTRACTOR_VERSION`: pass 1's output changes for crop NEFs, so bump per
      `docs/agents/shared-crates.md`. Main is at 13; on a rebase where a
      sibling took 14, take the next free number above main's. Confirm while
      implementing that a re-extracted row also re-runs pass 2 (the sharpness
      score reads the AF point), and bump `FACES_VERSION` only if it does not.
    - `cameras.md` / `cameras.ja.md`: put the three rows with the other Z
      bodies (after `Nikon Z 30`, before `Nikon D850`). Another session is
      adding DNG rows to the same table: on a conflict keep their rows and
      re-add only ours. Extend the sentence about the Z 8 and EOS R6 in the
      Canon / Nikon paragraph with the Z5II (all three samples in AF-A with no
      position), in both languages.
    - `raw-metadata-parsing.md`: the `AFInfo2` section says the frame "is the
      full image size" and "maps directly"; add the crop case, the two tags,
      the measured table and versions `0401` Z6III, `0402` Z50II / Z5II.
    - todo.md: other sessions edit it. Rebase onto `origin/main` before the
      push and change only this section's lines. A title such as "Core: Nikon
      crop-mode AF point offset is verified only on centered points" fits what
      remains.

## Trade-offs and risks

- **Where the crop's size comes from (decided: A).**
  - A: `CropHiSpeed` decides whether the frame is a crop and `CropArea` gives
    its size, both from the MakerNote. No extra read, the change stays inside
    `nef.rs`, and a reduced-size JpgFromRaw on an uncropped body keeps
    today's scaling. Cost: a body that crops without writing both tags keeps
    today's wrong scaling; none of the 14 samples is such a body.
  - B (rejected): compare against the full JPEG's SOF size, which `reader.rs`
    would have to range-read on every NEF, and which misreads the Z 6's
    3024x2016 JPEG as a crop, moving a verified point from (1512, 580) to
    (0, 156).
- **Offset versus scaling stays unproven on real files.** Every sample's AF
  point is at the center. The metadata confirms the crop is the sensor's
  center, unscaled; it does not confirm that `AFAreaXPosition` /
  `AFAreaYPosition` stay in the uncropped frame for an off-center point. The
  todo item is there to catch that.
- **`EXTRACTOR_VERSION` bump** re-extracts every folder once.
- **`CropArea` on older Z bodies is unverified.** `CropHiSpeed` gates it.

## Progress

- (2026-11-28) Step 1 complete
