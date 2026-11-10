<p align="center">English | <a href="./cameras.ja.md">日本語</a></p>

# What the camera records

Some features depend on what the camera records in the RAW file. The table
below lists every body verified on a real file; for the RAW formats, see
[Compatibility](../../README.md#compatibility); for the features themselves, see
[usage.md](./usage.md); for why these differ by body even within one RAW
format, see [How RAW files differ](./raw-formats.md).

Handpic culls from the JPEG preview embedded in the RAW file. A RAW with no
embedded JPEG preview is outside that purpose and is not supported: `no
embedded preview` is the intended outcome, and Handpic does not decode the
sensor data or render a tiny uncompressed thumbnail instead. A body released
more than ten years before the current year (as of 2026, released
before 2016), or whose largest embedded JPEG is under 1280 px on the long
edge, too small to judge focus with, is not actively supported: it is not
listed here or in [Compatibility](../../README.md#compatibility) and gets no
new work. Files from such a body that already open keep opening, since Handpic
does not block them. Once a year, re-read the table against the new cutoff
and delist the bodies that fell below it, as a docs-only change.

| Camera | AF point | AF frame size | Face tracking | Sub-second capture time |
|---|---|---|---|---|
| Sony α1 | ✓ | ✓ | – | ✓ |
| Sony α9 III | – | – | – | ✓ |
| Sony α9 II | ✓ | – | – | ✓ |
| Sony α7 V | ✓ | ✓ | ✓ | ✓ |
| Sony α7 IV | ✓ | ✓ | ✓ | ✓ |
| Sony α7R V | ✓ | ✓ | ✓ | ✓ |
| Sony α7R IV | ✓ | – | – | – |
| Sony α7R IVA | ✓ | – | – | – |
| Sony α7S III | ✓ | ✓ | ✓ | ✓ |
| Sony α7C II | ✓ | ✓ | – | ✓ |
| Sony α7CR | – | – | – | ✓ |
| Sony α7C | ✓ | – | – | ✓ |
| Sony α6700 | ✓ | ✓ | ✓ | ✓ |
| Sony α6400 | ✓ | – | – | – |
| Sony α6600 | ✓ | – | – | – |
| Sony ZV-E1 | ✓ | ✓ | ✓ | ✓ |
| Sony ZV-E10 | ✓ | – | – | ✓ |
| SIGMA BF | ✓ | – | – | – |
| SIGMA fp L | – | – | – | – |
| Leica M11-P | – | – | – | – |
| Canon EOS R | ✓ | ✓ | – | ✓ |
| Canon EOS RP | ✓ | ✓ | – | ✓ |
| Canon EOS R3 | ✓ | ✓ | – | ✓ |
| Canon EOS R5 | ✓ | ✓ | – | ✓ |
| Canon EOS R5 Mark II | ✓ | ✓ | – | ✓ |
| Canon EOS R6 | – | – | – | ✓ |
| Canon EOS R6 Mark II | ✓ | ✓ | – | ✓ |
| Canon EOS R6 Mark III | ✓ | ✓ | – | ✓ |
| Canon EOS R7 | ✓ | ✓ | – | ✓ |
| Canon EOS R8 | ✓ | ✓ | – | ✓ |
| Canon EOS R10 | ✓ | ✓ | – | ✓ |
| Canon EOS R50 | ✓ | ✓ | – | ✓ |
| Canon EOS R50 V | ✓ | ✓ | – | ✓ |
| Canon EOS R100 | ✓ | ✓ | – | ✓ |
| Nikon Z 9 | ✓ | ✓ | – | ✓ |
| Nikon Z 8 | – | – | – | ✓ |
| Nikon Z 7II | ✓ | ✓ | – | ✓ |
| Nikon Z 6II | ✓ | ✓ | – | ✓ |
| Nikon Z 6 | ✓ | ✓ | – | ✓ |
| Nikon Z 5 | ✓ | ✓ | – | ✓ |
| Nikon Z f | ✓ | ✓ | – | ✓ |
| Nikon Z fc | ✓ | ✓ | – | ✓ |
| Nikon Z 50 | ✓ | ✓ | – | ✓ |
| Nikon Z 30 | ✓ | ✓ | – | ✓ |
| Nikon D850 | – | – | – | ✓ |
| Nikon D500 | – | – | – | ✓ |
| Fujifilm X-H2S | ✓ | – | – | – |
| Fujifilm X-H2 | ✓ | – | – | – |
| Fujifilm X-T5 | ✓ | – | – | – |
| Fujifilm X-T50 | ✓ | – | – | ✓ |
| Fujifilm X-T4 | ✓ | – | – | – |
| Fujifilm X-T3 | – | – | – | – |
| Fujifilm X-T30 III | ✓ | – | – | ✓ |
| Fujifilm X-T30 II | ✓ | – | – | – |
| Fujifilm X-S20 | ✓ | – | – | – |
| Fujifilm X-S10 | ✓ | – | – | – |
| Fujifilm X-M5 | ✓ | – | – | ✓ |
| Fujifilm X-E5 | ✓ | – | – | ✓ |
| Fujifilm X-E4 | ✓ | – | – | – |
| Fujifilm X-Pro3 | ✓ | – | – | – |
| Fujifilm X100VI | ✓ | – | – | ✓ |
| Fujifilm X100V | ✓ | – | – | – |
| Fujifilm GFX100 II | ✓ | – | – | ✓ |
| Fujifilm GFX100S II | ✓ | – | – | ✓ |
| Fujifilm GFX100S | ✓ | – | – | – |
| Fujifilm GFX100RF | ✓ | – | – | ✓ |
| Fujifilm GFX 100 | – | – | – | – |
| Fujifilm GFX50S II | ✓ | – | – | – |
| Fujifilm X-T30 | ✓ | – | – | – |
| Fujifilm GFX 50R | ✓ | – | – | – |
| Fujifilm GFX 50S | – | – | – | – |
| Fujifilm X-H1 | ✓ | – | – | – |
| Fujifilm X-T2 | ✓ | – | – | – |
| Fujifilm X-T20 | ✓ | – | – | – |
| Fujifilm X-T200 | ✓ | – | – | – |
| Fujifilm X-T100 | ✓ | – | – | – |
| Fujifilm X-Pro2 | ✓ | – | – | – |
| Fujifilm X-E3 | – | – | – | – |
| Fujifilm X-E2S | ✓ | – | – | – |
| Fujifilm X-A7 | ✓ | – | – | – |
| Fujifilm X-A5 | ✓ | – | – | – |
| Fujifilm X-A3 | ✓ | – | – | – |
| Fujifilm X-A10 | ✓ | – | – | – |
| Fujifilm X100F | – | – | – | – |
| Fujifilm X70 | ✓ | – | – | – |
| Fujifilm XF10 | – | – | – | – |
| OM System OM-1 | – | – | – | – |
| OM System OM-1 Mark II | – | – | – | – |
| OM System OM-3 | – | – | – | – |
| OM System OM-5 | – | – | – | – |
| OM System OM-5 Mark II | – | – | – | – |
| Olympus E-M1X | – | – | – | – |
| Olympus E-M1 Mark III | – | – | – | – |
| Olympus E-M1 Mark II | – | – | – | – |
| Olympus E-M5 Mark III | – | – | – | – |
| Olympus E-M10 Mark IV | – | – | – | – |
| Olympus PEN E-P7 | – | – | – | – |
| Olympus PEN-F | – | – | – | – |

On the Canon and Nikon bodies, Handpic reads the AF point from the MakerNote
(`AFInfo2`); with several AF points in focus, the mark covers them all. The
Nikon Z 8 and Canon EOS R6 samples carried no AF position (an automatic area
that never locked, and manual focus), so those two are unconfirmed; the Nikon
D850 and D500 write an older `AFInfo2` that Handpic does not read.

CR3 files shot with HDR PQ on (HEIF) hold HEVC images instead of JPEGs.
Handpic decodes their 1620x1080 HEVC preview and tone-maps it to sRGB, so
they get a thumbnail, a preview and a sharpness score like any other file.
The full-size image is not decoded: the 1:1 view (`z`) on such a file shows
a crop of that 1620x1080 preview, not the sensor's pixels.

On the Fujifilm bodies, Handpic reads the AF point from the MakerNote
(`FocusPixel`), a point without a frame size. The X-T3 and GFX 100 samples
were all shot in manual focus, which Handpic treats as having no AF point, so
those two are unconfirmed; the same holds for the X-E3, X100F, XF10 and
GFX 50S samples. Sub-second capture time is marked as the samples recorded
it; none of the older X bodies at 1920x1280 records it.

A RAF holds one embedded JPEG, below the sensor's resolution. It is the
preview and the 1:1 view on every Fujifilm body: the 1:1 view shows that JPEG
at its own size, not the sensor's pixels. The older X bodies also open, but
their embedded JPEG is 1920x1280, so their 1:1 view is that small JPEG rather
than a pixel-level check:

| Embedded JPEG size | Fujifilm bodies |
|---|---|
| 4416x2944 | X-H2S, X-H2, X-T5, X-T50, X-T4, X-T3, X-T30 III, X-T30 II, X-T30, X-S20, X-S10, X-M5, X-E5, X-E4, X-Pro3, X100VI, X100V |
| 4000x3000 | GFX100 II, GFX100S II, GFX100S, GFX100RF, GFX 100, GFX50S II, GFX 50R, GFX 50S |
| 1920x1280 | X-H1, X-T2, X-T20, X-T200, X-T100, X-Pro2, X-E3, X-E2S, X-A7, X-A5, X-A3, X-A10, X100F, X70, XF10 |

An ORF holds one embedded JPEG, 3200x2400 on every body listed here, below
the sensor's resolution. The 1:1 view on ORF shows that JPEG at its own size,
not the sensor's pixels.

On the OM System and Olympus bodies, Handpic does not read the AF point yet:
the MakerNote records it as `AFTargetInfo` (OM bodies) and `AFPointSelected`,
which are left for later. None of the samples of these bodies records a
sub-second capture time, so their bursts group by whole seconds; a burst at a
high frame rate cannot be split within one second.

The Sony α9 III and α7CR samples were all shot in manual focus, which Handpic
treats as having no AF point, so those two are unconfirmed. On the remaining
Sony bodies, `–` under Face tracking means no sample of that body recorded
face tracking; it does not mean the body lacks it. A recorded face tracking
does not guarantee a face under the frame: two α6700 samples record it with
the frame on an empty background and on bread on a market stall. Handpic also
ignores the frame when the AF point sits at the exact sensor center, where
tracking never locked.

The Sony α9 II, α7R IV, α7R IVA, α7C, α6400, α6600 and ZV-E10 embed no
full-size JPEG, only the 1616x1080 preview (1920x1080 on the ZV-E10) and a
160x120 thumbnail. Their 1:1 view shows that preview at its own size, as ORF
does, not the sensor's pixels.

- **AF point**: the focus mark is drawn there, the 1:1 focus check opens
  centered on it, and sharpness is scored around it. Faces are then ignored,
  even when the point lies outside every face. Without one, there is no focus
  mark, the 1:1 focus check opens at the frame center, and sharpness is scored
  between the eyes of a detected face, else on the sharpest region. A
  manual-focus shot on a body that records the focus mode (Sony) is treated as
  having no AF point; DMF shots and bodies that record no focus mode are not.
- **AF frame size and face tracking**: with both, sharpness is scored on the
  camera's eye-AF frame, which is the most reliable because it does not rely
  on face detection: the score is measured on the eye the camera focused on,
  even when Handpic's own face detection misses the face. Face tracking without an AF
  frame size gives no such benefit: sharpness is scored around the AF point as
  usual. Neither changes the focus mark's color, whose AF eye in-focus
  probability comes from Handpic's face detection. On the α7 V, `Face tracking` is also recorded when the
  AF sits on the back of a head: it means the camera recognized a person's
  head, not strictly a face or an eye. The meta pane's `AF tracking` row
  shows this value.
- **Sub-second capture time**: frames within 1 s of the previous one form a
  burst. Without it, frames are grouped by whole seconds, so a shot taken up
  to about 2 s after the previous frame can still join its burst.
