//! Minimal parser that locates the embedded JPEGs in an ARW or a DNG (both
//! TIFF variants, read in the byte order of their header).

use anyhow::{anyhow, bail, Result};

use crate::sequence::Tiff;

const TAG_IMAGE_WIDTH: u16 = 0x0100;
const TAG_IMAGE_HEIGHT: u16 = 0x0101;
const TAG_COMPRESSION: u16 = 0x0103;
const TAG_PHOTOMETRIC: u16 = 0x0106;
const TAG_STRIP_OFFSETS: u16 = 0x0111;
const TAG_STRIP_BYTE_COUNTS: u16 = 0x0117;
const TAG_MAKE: u16 = 0x010f;
const TAG_MODEL: u16 = 0x0110;
const TAG_ORIENTATION: u16 = 0x0112;
const TAG_JPEG_OFFSET: u16 = 0x0201;
const TAG_JPEG_LENGTH: u16 = 0x0202;
const TAG_SUB_IFDS: u16 = 0x014a;
const TAG_EXIF_IFD: u16 = 0x8769;
const TAG_EXPOSURE_TIME: u16 = 0x829a;
const TAG_F_NUMBER: u16 = 0x829d;
const TAG_ISO: u16 = 0x8827;
const TAG_APERTURE_VALUE: u16 = 0x9202;
const TAG_EXPOSURE_BIAS: u16 = 0x9204;
const TAG_FOCAL_LENGTH: u16 = 0x920a;
const TAG_LENS_MODEL: u16 = 0xa434;
const TAG_DATE_TIME_ORIGINAL: u16 = 0x9003;
const TAG_SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;
const TAG_MAKER_NOTE: u16 = 0x927c;
const TAG_FOCUS_LOCATION: u16 = 0x2027;
/// Sony `FocusMode`, a BYTE. Older bodies write it as 0xb04e / 0xb042
/// instead, which is not read, so they keep `None`.
const TAG_FOCUS_MODE: u16 = 0x201b;
/// Sony `ElectronicFrontCurtainShutter`, a LONG on the ILCE-7M5.
const TAG_ELECTRONIC_FRONT_CURTAIN_SHUTTER: u16 = 0x201a;
/// Sony `AFTracking`, a BYTE.
const TAG_AF_TRACKING: u16 = 0x2021;
/// Sony `AFAreaModeSetting`, a BYTE.
const TAG_AF_AREA_MODE_SETTING: u16 = 0x201c;
/// Sony `ReleaseMode`, an int16u.
const TAG_RELEASE_MODE: u16 = 0xb049;
/// Sony `SequenceNumber`, an int16u.
const TAG_SEQUENCE_NUMBER: u16 = 0xb04a;
/// Sony `ImageStabilization`, an int32u.
const TAG_IMAGE_STABILIZATION: u16 = 0xb026;
/// Sony `ExposureMode`, an int16u.
const TAG_EXPOSURE_MODE: u16 = 0xb041;
/// Sony `MeteringMode2`, an int16u.
const TAG_METERING_MODE2: u16 = 0x202c;
/// Sony `CreativeStyle`, an ASCII string.
const TAG_CREATIVE_STYLE: u16 = 0xb020;
/// Sony `DynamicRangeOptimizer`, an int32u.
const TAG_DYNAMIC_RANGE_OPTIMIZER: u16 = 0xb025;
/// Sony `RAWFileType`, an int16u.
const TAG_RAW_FILE_TYPE: u16 = 0x2029;
/// Sony `FocusFrameSize`: three SHORTs, width, height and a validity flag
/// (0 when the frame is not available). Bodies write it as `UNDEFINED[6]`.
const TAG_FOCUS_FRAME_SIZE: u16 = 0x2037;
/// Leica MakerNote `FocusDistance`, a LONG in millimeters.
const TAG_LEICA_FOCUS_DISTANCE: u16 = 0x0304;
/// A Leica MakerNote is `LEICA\0` plus two bytes, then a plain IFD in the
/// container's byte order.
const LEICA_HEADER: &[u8] = b"LEICA\0";
const LEICA_HEADER_LEN: usize = 8;
/// Leica MakerNote `PreviewImage`, an UNDEFINED JPEG whose value is its file
/// offset: 1472x976 on the M (Typ 240) and the M Monochrom (Typ 246), whose
/// strips hold only a 160x120 one.
const TAG_LEICA_PREVIEW_IMAGE: u16 = 0x0300;
/// Sigma MakerNote AF point, `SHORT[2]` (x, y) written by the Sigma BF.
const TAG_SIGMA_AF_POINT: u16 = 0x0147;
/// A Sigma MakerNote is `SIGMA\0\0\0` plus two version bytes, then a plain IFD.
const SIGMA_HEADER: &[u8] = b"SIGMA\0\0\0";
const SIGMA_HEADER_LEN: usize = 10;
const SIGMA_BF_MODEL: &str = "Sigma BF";
/// The frame the Sigma BF AF point is in, taken as unrotated sensor
/// coordinates like Sony's `FocusLocation`: a 3:2 frame normalized to 1000
/// wide. The BF writes no AF area size. An assumption derived from 11
/// landscape samples (the point lands on the subject read as `x/1000`,
/// `y/667`; `y/1000` misses); portrait frames are unverified.
const SIGMA_BF_AF_GRID_W: u16 = 1000;
const SIGMA_BF_AF_GRID_H: u16 = 667;

const COMPRESSION_JPEG: u32 = 7;
const PHOTOMETRIC_YCBCR: u32 = 6;
/// The preview tier takes the smallest strip JPEG at least this wide.
const PREVIEW_MIN_WIDTH: u32 = 1600;

const TYPE_BYTE: u16 = 1;
const TYPE_ASCII: u16 = 2;
const TYPE_UNDEFINED: u16 = 7;
const TYPE_SHORT: u16 = 3;
const TYPE_LONG: u16 = 4;
const TYPE_RATIONAL: u16 = 5;
const TYPE_SRATIONAL: u16 = 10;

/// Location of one embedded image inside an ARW.
#[derive(Debug, Clone, Copy)]
pub struct Embedded {
    pub offset: usize,
    pub length: usize,
    pub codec: Codec,
}

/// How an embedded image is coded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Jpeg,
    /// A CR3 `PRVW` / `THMB` shot with HDR PQ on: the whole box payload,
    /// header included, that `hevc::to_jpeg` decodes.
    Hevc,
}

/// Sony `FocusLocation`: the focus point in unrotated sensor coordinates,
/// together with the sensor size those coordinates are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusLocation {
    pub sensor_w: u16,
    pub sensor_h: u16,
    pub x: u16,
    pub y: u16,
}

/// Sony `FocusFrameSize`: the AF frame size in the same sensor coordinates
/// as `FocusLocation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusFrame {
    pub width: u16,
    pub height: u16,
}

/// A TIFF RATIONAL or SRATIONAL: numerator over denominator, kept unreduced
/// so the shutter speed can be shown the way the camera wrote it (1/250).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

impl Rational {
    pub fn value(self) -> Option<f64> {
        (self.den != 0).then(|| self.num as f64 / self.den as f64)
    }
}

/// The `DSC-` models ExifTool exempts from its model condition on
/// `FocusMode` (0x201b) and `AFTracking` (0x2021): `($$self{Model} !~
/// /^DSC-/) or ($$self{Model} =~
/// /^DSC-(RX10M4|RX100M6|RX100M7|RX100M5A|HX95|HX99|RX0M2|RX1RM3)/)`. On
/// every other `DSC-` body the tags "don't seem to apply" (ExifTool's
/// comment) and always read 0.
const DSC_EXCEPTIONS: [&str; 8] = [
    "DSC-RX10M4",
    "DSC-RX100M6",
    "DSC-RX100M7",
    "DSC-RX100M5A",
    "DSC-HX95",
    "DSC-HX99",
    "DSC-RX0M2",
    "DSC-RX1RM3",
];

/// Whether `model` is a `DSC-` body ExifTool excludes from `FocusMode` /
/// `AFTracking`. `None` (model unknown) is not excluded.
pub fn excluded_dsc(model: Option<&str>) -> bool {
    let Some(model) = model else {
        return false;
    };
    model.starts_with("DSC-") && !DSC_EXCEPTIONS.iter().any(|m| model.starts_with(m))
}

/// The shooting settings read out of IFD0 and the ExifIFD. Every field is
/// optional: a body that does not write a tag is not an error.
#[derive(Debug, Clone, Default)]
pub struct Shot {
    /// Raw `DateTimeOriginal`, `YYYY:MM:DD HH:MM:SS`.
    pub capture_time: Option<String>,
    /// Raw `SubSecTimeOriginal`.
    pub subsec: Option<String>,
    pub focus: Option<FocusLocation>,
    /// Raw Sony `FocusMode`: 0 manual, 2 AF-S, 3 AF-C, 4 AF-A, 6 DMF.
    pub focus_mode: Option<u8>,
    /// Raw Sony `AFTracking`: 0 off, 1 face tracking, 2 lock-on AF.
    pub af_tracking: Option<u8>,
    /// Raw Sony `ElectronicFrontCurtainShutter`: 1 on, 0 off.
    pub electronic_front_curtain: Option<u32>,
    /// Raw Sony `AFAreaModeSetting`. On NEX / ILCE / ZV bodies: 0 wide,
    /// 1 center, 3 flexible spot, 4 flexible spot (LA-EA4), 9 center
    /// (LA-EA4), 11 zone, 12 expanded flexible spot, 13 custom AF area; other
    /// families use other tables.
    pub af_area_mode: Option<u8>,
    /// Raw Sony `ReleaseMode`: 0 normal, 2 continuous, 5 exposure bracketing,
    /// 6 white balance bracketing, 8 DRO bracketing, 65535 n/a.
    pub release_mode: Option<u32>,
    /// Raw Sony `SequenceNumber`, the shot number within a burst: 0 single,
    /// 65535 n/a, otherwise the frame number (restarting at 1 per burst).
    pub sequence_number: Option<u32>,
    /// Raw Sony `ImageStabilization`: 0 off, 1 on, 0xffffffff n/a.
    pub image_stabilization: Option<u32>,
    /// Raw Sony `ExposureMode`: 0 program AE, 7 aperture priority, 8 shutter
    /// priority, 15 manual, the scene modes in between and above (ExifTool's
    /// `Sony::Main` table), 65535 n/a.
    pub exposure_mode: Option<u32>,
    /// Raw Sony `MeteringMode2`: 0x100 multi-segment, 0x200 center-weighted
    /// average, 0x301 spot (standard), 0x302 spot (large), 0x400 average,
    /// 0x500 highlight.
    pub metering_mode: Option<u32>,
    /// Raw Sony `CreativeStyle` (the Creative Look on current bodies), as
    /// recorded: `Standard`, `Vivid`, `ST`, `VV2`, ...
    pub creative_style: Option<String>,
    /// Raw Sony `DynamicRangeOptimizer` (0xb025): 0 off, 1 standard,
    /// 2 advanced auto, 3 auto, 8-12 advanced Lv1-Lv5, 16-23 Lv1-Lv8.
    pub dynamic_range_optimizer: Option<u32>,
    /// Raw Sony `RAWFileType`: 0 compressed, 1 uncompressed, 2 lossless
    /// compressed, 3 compressed RAW 2, 65535 n/a.
    pub raw_file_type: Option<u32>,
    /// Sony `FocusFrameSize`, `None` when the camera flags it as unavailable.
    pub focus_frame: Option<FocusFrame>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens_model: Option<String>,
    pub exposure_time: Option<Rational>,
    pub f_number: Option<Rational>,
    /// `2^(AV/2)` from the APEX `ApertureValue`, only when `FNumber` is
    /// absent (manual lenses without a lens contact): the camera's estimate.
    pub estimated_f_number: Option<f64>,
    /// Leica MakerNote focus distance, in millimeters.
    pub focus_distance_mm: Option<u32>,
    pub focal_length: Option<Rational>,
    pub exposure_bias: Option<Rational>,
    pub iso: Option<u32>,
}

/// A Leica MakerNote that runs past the buffer `parse_noted` was given: where
/// it is, for a ranged read of its head that `FarNote::take` finishes the
/// parse with (the M (Typ 240) / M Monochrom (Typ 246) note sits tens of MB
/// into the file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FarNote {
    pub offset: usize,
    pub length: usize,
    little_endian: bool,
    /// No strip JPEG reached `PREVIEW_MIN_WIDTH`, so the note's
    /// `PreviewImage` may be taken (`take_note_preview`).
    fallback: bool,
}

impl FarNote {
    /// Read the note out of `head`, its first bytes, into `arw` as
    /// `parse_noted` would have out of a whole file: `FocusDistance`, and the
    /// `PreviewImage` unless it runs past `file_len`. A `head` that holds no
    /// Leica IFD leaves `arw` as it is.
    pub fn take(self, arw: &mut Arw, head: &[u8], file_len: u64) {
        let Ok(Some((t, ifd))) = leica_note_ifd(head, self.little_endian) else {
            return;
        };
        arw.shot.focus_distance_mm = ifd
            .iter()
            .find(|e| e.0 == TAG_LEICA_FOCUS_DISTANCE)
            .and_then(|e| integer(&t, e));
        let Some(e) = note_preview(&ifd).filter(|_| self.fallback) else {
            return;
        };
        if (e.offset as u64).saturating_add(e.length as u64) <= file_len {
            take_note_preview(arw, e);
        }
    }
}

#[derive(Debug)]
pub struct Arw {
    /// The small preview in IFD0 (Sony 1616x1080).
    pub preview: Option<Embedded>,
    /// The full-resolution JPEG (JpgFromRaw), or the preview when the file
    /// holds nothing larger.
    pub full: Option<Embedded>,
    pub orientation: u16,
    pub shot: Shot,
    /// The full-size image is HEVC (a CR3 shot with HDR PQ on), so `full` is
    /// the HEVC `PRVW` if there is one.
    pub hevc: bool,
}

/// One IFD entry: (tag, value-or-offset, type, count). The value field is
/// decoded as a `u32` in the file's byte order.
type Entry = (u16, u32, u16, u32);

/// Read an IFD and return its entries along with the offset of the next IFD.
fn read_ifd(t: &Tiff, at: usize) -> Result<(Vec<Entry>, usize)> {
    let count = t.u16(at).map_err(|_| anyhow!("IFD offset out of range"))? as usize;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let e = at + 2 + i * 12;
        t.bytes(e, 12)
            .map_err(|_| anyhow!("IFD entry out of range"))?;
        // (tag, value/offset, type, count)
        entries.push((t.u16(e)?, t.u32(e + 8)?, t.u16(e + 2)?, t.u32(e + 4)?));
    }
    let next_ifd = t
        .u32(at + 2 + count * 12)
        .map_err(|_| anyhow!("next IFD offset out of range"))? as usize;
    Ok((entries, next_ifd))
}

/// The four bytes of an entry's value field, in file order.
fn inline_bytes(t: &Tiff, value: u32) -> [u8; 4] {
    if t.little_endian() {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    }
}

/// The SHORT in the first two bytes of a value field: the low half of the
/// decoded `u32` in a little-endian file, the high half in a big-endian one.
fn inline_short(t: &Tiff, value: u32) -> u32 {
    if t.little_endian() {
        value & 0xFFFF
    } else {
        value >> 16
    }
}

fn find(entries: &[Entry], tag: u16) -> Option<(u32, u32)> {
    entries.iter().find(|e| e.0 == tag).map(|e| (e.1, e.3))
}

fn embedded(entries: &[Entry]) -> Option<Embedded> {
    let (offset, _) = find(entries, TAG_JPEG_OFFSET)?;
    let (length, _) = find(entries, TAG_JPEG_LENGTH)?;
    Some(Embedded {
        offset: offset as usize,
        length: length as usize,
        codec: Codec::Jpeg,
    })
}

/// A DNG-style embedded JPEG: an IFD whose single strip is a YCbCr JPEG. The
/// photometric check keeps a lossless-JPEG CFA raw (also `Compression=7`) out.
fn strip_jpeg(t: &Tiff, entries: &[Entry]) -> Option<(Embedded, u32, u32)> {
    let int = |tag| {
        entries
            .iter()
            .find(|e| e.0 == tag)
            .and_then(|e| integer(t, e))
    };
    if int(TAG_COMPRESSION)? != COMPRESSION_JPEG || int(TAG_PHOTOMETRIC)? != PHOTOMETRIC_YCBCR {
        return None;
    }
    let e = Embedded {
        offset: int(TAG_STRIP_OFFSETS)? as usize,
        length: int(TAG_STRIP_BYTE_COUNTS)? as usize,
        codec: Codec::Jpeg,
    };
    Some((e, int(TAG_IMAGE_WIDTH)?, int(TAG_IMAGE_HEIGHT)?))
}

/// Read an ASCII entry's value. Values of up to 4 bytes sit in the entry
/// itself; longer ones live at the offset the entry carries.
fn ascii(t: &Tiff, entry: &Entry) -> Result<Option<String>> {
    let (_, value, typ, count) = *entry;
    if typ != TYPE_ASCII {
        return Ok(None);
    }
    let count = count as usize;
    let inline = inline_bytes(t, value);
    let bytes: &[u8] = if count <= 4 {
        &inline[..count]
    } else {
        t.bytes(value as usize, count)
            .map_err(|_| anyhow!("ASCII value out of range"))?
    };
    let text = String::from_utf8_lossy(bytes);
    Ok(Some(text.trim_end_matches('\0').to_string()))
}

/// Read a `SHORT[N]` entry for `N > 2`. More than four bytes never fit in an
/// entry, so the value is always at the offset.
fn shorts<const N: usize>(t: &Tiff, entry: &Entry) -> Result<Option<[u16; N]>> {
    let (_, value, typ, count) = *entry;
    if typ != TYPE_SHORT || count as usize != N {
        return Ok(None);
    }
    let at = value as usize;
    if t.bytes(at, N * 2).is_err() {
        bail!("SHORT[{N}] value out of range");
    }
    let mut out = [0u16; N];
    for (i, v) in out.iter_mut().enumerate() {
        *v = t.u16(at + i * 2)?;
    }
    Ok(Some(out))
}

/// Find the Sony MakerNote IFD behind tag 0x927c. Recent bodies (Sony5) start
/// the IFD right at the tag's data offset; older ones prefix it with a 12-byte
/// `SONY DSC \0\0\0` / `SONY CAM \0\0\0` header. Either way the value
/// offsets inside it are absolute to the TIFF start, so the IFD can be read
/// straight out of the container.
fn maker_note_ifd(t: &Tiff, entries: &[Entry], make: Option<&str>) -> Result<Option<Vec<Entry>>> {
    let Some(entry) = entries.iter().find(|e| e.0 == TAG_MAKER_NOTE) else {
        return Ok(None);
    };
    let (at, count) = (entry.1 as usize, entry.3 as usize);
    // A Leica note is never a Sony one, and the M (Typ 240)'s lies past the
    // bounded prefix, which must not fail the parse (`FarNote`).
    if count <= 4 || is_leica(make) {
        // Too short to hold an IFD, and such a value is inline anyway.
        return Ok(None);
    }
    let sony_header = t
        .bytes(at, count)
        .map_err(|_| anyhow!("MakerNote out of range"))?
        .starts_with(b"SONY");
    // Another maker's note (Leica's starts with `LEICA\0`) is not an IFD at
    // this offset; reading it as one walks garbage. A missing Make is given
    // the benefit of the doubt, as Sony5 notes carry no header.
    if !sony_header && make.is_some_and(|m| !m.starts_with("SONY")) {
        return Ok(None);
    }
    let at = if count >= 14 && sony_header {
        at + 12
    } else {
        at
    };
    let (ifd, _) = read_ifd(t, at)?;
    Ok(Some(ifd))
}

/// Read a RATIONAL/SRATIONAL entry. Eight bytes never fit in an entry, so the
/// value is always at the offset.
fn rational(t: &Tiff, entry: &Entry) -> Result<Option<Rational>> {
    let (_, value, typ, count) = *entry;
    if (typ != TYPE_RATIONAL && typ != TYPE_SRATIONAL) || count != 1 {
        return Ok(None);
    }
    let at = value as usize;
    if t.bytes(at, 8).is_err() {
        bail!("RATIONAL value out of range");
    }
    let (num, den) = (t.u32(at)?, t.u32(at + 4)?);
    Ok(Some(if typ == TYPE_SRATIONAL {
        Rational {
            num: i64::from(num as i32),
            den: i64::from(den as i32),
        }
    } else {
        Rational {
            num: i64::from(num),
            den: i64::from(den),
        }
    }))
}

/// Read a single SHORT or LONG entry, whose value always rides inside the
/// entry itself. A SHORT occupies the first two bytes of the value field; the
/// other two are padding, which some writers leave nonzero.
fn integer(t: &Tiff, entry: &Entry) -> Option<u32> {
    let (_, value, typ, count) = *entry;
    match (typ, count) {
        (TYPE_SHORT, 1) => Some(inline_short(t, value)),
        (TYPE_LONG, 1) => Some(value),
        _ => None,
    }
}

fn byte(t: &Tiff, entry: &Entry) -> Option<u8> {
    let (_, value, typ, count) = *entry;
    (typ == TYPE_BYTE && count == 1).then_some(inline_bytes(t, value)[0])
}

/// Follow IFD0 -> ExifIFD (0x8769) -> MakerNote (0x927c) for the shooting
/// settings and the focus point. Any of the three may be absent, which is not
/// an error; an offset pointing outside the container is.
fn exif(t: &Tiff, ifd0: &[Entry]) -> Result<Shot> {
    let mut shot = Shot::default();
    for e in ifd0 {
        match e.0 {
            TAG_MAKE => shot.make = ascii(t, e)?,
            TAG_MODEL => shot.model = ascii(t, e)?,
            _ => {}
        }
    }

    let Some((at, _)) = find(ifd0, TAG_EXIF_IFD) else {
        return Ok(shot);
    };
    let (exif_ifd, _) = read_ifd(t, at as usize)?;

    for e in &exif_ifd {
        match e.0 {
            TAG_DATE_TIME_ORIGINAL => shot.capture_time = ascii(t, e)?,
            TAG_SUB_SEC_TIME_ORIGINAL => shot.subsec = ascii(t, e)?,
            TAG_LENS_MODEL => shot.lens_model = ascii(t, e)?,
            TAG_EXPOSURE_TIME => shot.exposure_time = rational(t, e)?,
            TAG_F_NUMBER => shot.f_number = rational(t, e)?,
            TAG_APERTURE_VALUE => {
                shot.estimated_f_number = rational(t, e)?
                    .and_then(Rational::value)
                    .map(|av| 2f64.powf(av / 2.0))
            }
            TAG_FOCAL_LENGTH => shot.focal_length = rational(t, e)?,
            TAG_EXPOSURE_BIAS => shot.exposure_bias = rational(t, e)?,
            TAG_ISO => shot.iso = integer(t, e),
            _ => {}
        }
    }

    let maker = maker_note_ifd(t, &exif_ifd, shot.make.as_deref())?;
    shot.focus_mode = maker
        .as_ref()
        .and_then(|m| m.iter().find(|e| e.0 == TAG_FOCUS_MODE))
        .and_then(|e| byte(t, e));
    shot.af_tracking = maker
        .as_ref()
        .and_then(|m| m.iter().find(|e| e.0 == TAG_AF_TRACKING))
        .and_then(|e| byte(t, e));
    shot.electronic_front_curtain = maker
        .as_ref()
        .and_then(|m| {
            m.iter()
                .find(|e| e.0 == TAG_ELECTRONIC_FRONT_CURTAIN_SHUTTER)
        })
        .and_then(|e| integer(t, e));
    let maker_entry = |tag: u16| maker.as_ref().and_then(|m| m.iter().find(|e| e.0 == tag));
    shot.af_area_mode = maker_entry(TAG_AF_AREA_MODE_SETTING).and_then(|e| byte(t, e));
    shot.release_mode = maker_entry(TAG_RELEASE_MODE).and_then(|e| integer(t, e));
    shot.sequence_number = maker_entry(TAG_SEQUENCE_NUMBER).and_then(|e| integer(t, e));
    shot.image_stabilization = maker_entry(TAG_IMAGE_STABILIZATION).and_then(|e| integer(t, e));
    shot.exposure_mode = maker_entry(TAG_EXPOSURE_MODE).and_then(|e| integer(t, e));
    shot.metering_mode = maker_entry(TAG_METERING_MODE2).and_then(|e| integer(t, e));
    shot.creative_style = match maker_entry(TAG_CREATIVE_STYLE) {
        Some(e) => ascii(t, e)?,
        None => None,
    };
    shot.dynamic_range_optimizer =
        maker_entry(TAG_DYNAMIC_RANGE_OPTIMIZER).and_then(|e| integer(t, e));
    shot.raw_file_type = maker_entry(TAG_RAW_FILE_TYPE).and_then(|e| integer(t, e));
    shot.focus_frame = match maker
        .as_ref()
        .and_then(|m| m.iter().find(|e| e.0 == TAG_FOCUS_FRAME_SIZE))
    {
        Some(&(tag, value, typ, count)) => {
            let e = match (typ, count) {
                (TYPE_UNDEFINED, 6) => (tag, value, TYPE_SHORT, 3),
                _ => (tag, value, typ, count),
            };
            shorts::<3>(t, &e)?.and_then(|v| {
                (v[2] != 0).then_some(FocusFrame {
                    width: v[0],
                    height: v[1],
                })
            })
        }
        None => None,
    };
    shot.focus = match maker {
        Some(maker) => match maker.iter().find(|e| e.0 == TAG_FOCUS_LOCATION) {
            Some(e) => shorts::<4>(t, e)?.map(|v| FocusLocation {
                sensor_w: v[0],
                sensor_h: v[1],
                x: v[2],
                y: v[3],
            }),
            None => None,
        },
        None => None,
    };

    if shot.f_number.is_some() {
        shot.estimated_f_number = None;
    }
    shot.focus_distance_mm = leica_focus_distance(t, &exif_ifd)?;
    shot.focus = shot.focus.or(sigma_af_point(
        t,
        &exif_ifd,
        shot.make.as_deref(),
        shot.model.as_deref(),
    )?);

    Ok(shot)
}

/// `FocusDistance` out of a Leica MakerNote. Only the one tag is read: the
/// note's own `FNumber` is 1.0 on M-mount lenses and is not trusted. The
/// note's IFD is in the container's byte order: big-endian in an M (Typ 240)
/// DNG, little-endian in an M10 one.
fn leica_focus_distance(t: &Tiff, exif_ifd: &[Entry]) -> Result<Option<u32>> {
    let Some(entry) = exif_ifd.iter().find(|e| e.0 == TAG_MAKER_NOTE) else {
        return Ok(None);
    };
    let (at, count) = (entry.1 as usize, entry.3 as usize);
    if count <= LEICA_HEADER_LEN
        || !t
            .bytes(at, count)
            .is_ok_and(|note| note.starts_with(LEICA_HEADER))
    {
        return Ok(None);
    }
    let (note, _) = read_ifd(t, at + LEICA_HEADER_LEN)?;
    Ok(note
        .iter()
        .find(|e| e.0 == TAG_LEICA_FOCUS_DISTANCE)
        .and_then(|e| integer(t, e)))
}

/// The AF point out of a Sigma BF MakerNote, on the
/// `SIGMA_BF_AF_GRID_W` x `SIGMA_BF_AF_GRID_H` grid. A point outside the grid
/// is kept as is; the consumers clamp it into the JPEG.
fn sigma_af_point(
    t: &Tiff,
    exif_ifd: &[Entry],
    make: Option<&str>,
    model: Option<&str>,
) -> Result<Option<FocusLocation>> {
    let sigma = make.is_some_and(|m| m.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("SIGMA")));
    if !sigma || model != Some(SIGMA_BF_MODEL) {
        return Ok(None);
    }
    let Some(entry) = exif_ifd.iter().find(|e| e.0 == TAG_MAKER_NOTE) else {
        return Ok(None);
    };
    let (at, count) = (entry.1 as usize, entry.3 as usize);
    if count <= SIGMA_HEADER_LEN {
        return Ok(None);
    }
    let Ok(bytes) = t.bytes(at, count) else {
        bail!("MakerNote out of range");
    };
    if !bytes.starts_with(SIGMA_HEADER) {
        return Ok(None);
    }
    let (note, _) = read_ifd(t, at + SIGMA_HEADER_LEN)?;
    Ok(note
        .iter()
        .find(|e| e.0 == TAG_SIGMA_AF_POINT)
        .and_then(|&(_, value, typ, count)| {
            (typ == TYPE_SHORT && count == 2).then(|| {
                let v = inline_bytes(t, value);
                let short = |at: usize| {
                    if t.little_endian() {
                        u16::from_le_bytes([v[at], v[at + 1]])
                    } else {
                        u16::from_be_bytes([v[at], v[at + 1]])
                    }
                };
                FocusLocation {
                    sensor_w: SIGMA_BF_AF_GRID_W,
                    sensor_h: SIGMA_BF_AF_GRID_H,
                    x: short(0),
                    y: short(2),
                }
            })
        }))
}

pub fn parse(buf: &[u8]) -> Result<Arw> {
    Ok(parse_noted(buf)?.0)
}

/// `parse`, plus the Leica MakerNote it would take the preview from when
/// that note's IFD lies past `buf`.
pub fn parse_noted(buf: &[u8]) -> Result<(Arw, Option<FarNote>)> {
    let t = &Tiff::new(buf, 0, buf.len())?;
    let (ifd0, next) = read_ifd(t, t.u32(4)? as usize)?;

    let orientation = find(&ifd0, TAG_ORIENTATION)
        .map(|(v, _)| inline_short(t, v) as u16)
        .unwrap_or(1);
    let preview = embedded(&ifd0);
    let shot = exif(t, &ifd0)?;

    // Walk the IFD chain (IFD1, IFD2, ...) and the SubIFDs, and take the
    // largest JPEG as the full-resolution one (JpgFromRaw), falling back to
    // the preview when that is only a thumbnail (older Sony bodies write no
    // full-size JPEG, just a 160x120 one in IFD1).
    let mut targets: Vec<usize> = Vec::new();
    let mut next = next;
    while next != 0 {
        targets.push(next);
        let (_, n) = read_ifd(t, next)?;
        next = n;
        if targets.len() > 16 {
            break; // do not spin forever on a broken chain
        }
    }
    if let Some((val, count)) = find(&ifd0, TAG_SUB_IFDS) {
        if count == 1 {
            targets.push(val as usize);
        } else {
            let count = count as usize;
            if t.bytes(val as usize, count * 4).is_err() {
                bail!("SubIFD offset array out of range");
            }
            for i in 0..count {
                targets.push(t.u32(val as usize + i * 4)? as usize);
            }
        }
    }

    let mut full: Option<Embedded> = None;
    let mut strips: Vec<(Embedded, u32, u32)> = strip_jpeg(t, &ifd0).into_iter().collect();
    for off in targets {
        let (ifd, _) = read_ifd(t, off)?;
        if let Some(e) = embedded(&ifd) {
            if full.is_none_or(|f| e.length > f.length) {
                full = Some(e);
            }
        }
        strips.extend(strip_jpeg(t, &ifd));
    }

    // A DNG carries its previews as strip JPEGs in no guaranteed order, so pick
    // by size: the largest is the 1:1 tier, and the smallest one wide enough
    // for the preview pane is the preview (skipping tiny thumbnails), falling
    // back to the 1:1 JPEG when nothing else qualifies.
    let mut fallback = false;
    let (preview, full) = if preview.is_none() && full.is_none() && !strips.is_empty() {
        let area = |s: &(Embedded, u32, u32)| u64::from(s.1) * u64::from(s.2);
        let big = strips.iter().max_by_key(|s| area(s)).map(|s| s.0);
        let small = strips
            .iter()
            .filter(|s| s.1 >= PREVIEW_MIN_WIDTH)
            .min_by_key(|s| area(s))
            .map(|s| s.0);
        fallback = small.is_none();
        (small.or(big), big)
    } else {
        match (preview, full) {
            (Some(p), Some(f)) if f.length < p.length => (preview, Some(p)),
            _ => (preview, full),
        }
    };

    let leica = is_leica(shot.make.as_deref());
    let mut arw = Arw {
        preview,
        full,
        orientation,
        shot,
        hevc: false,
    };
    let mut far = None;
    if leica {
        if let Some((at, count)) = maker_note_at(t, &ifd0)? {
            let end = at.saturating_add(count);
            if end > buf.len() {
                far = Some(FarNote {
                    offset: at,
                    length: count,
                    little_endian: t.little_endian(),
                    fallback,
                });
            } else if fallback {
                if let Ok(Some((_, ifd))) = leica_note_ifd(&buf[at..end], t.little_endian()) {
                    if let Some(e) = note_preview(&ifd) {
                        take_note_preview(&mut arw, e);
                    }
                }
            }
        }
    }
    Ok((arw, far))
}

fn is_leica(make: Option<&str>) -> bool {
    make.is_some_and(|m| m.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("LEICA")))
}

/// The MakerNote's file offset and length, from IFD0's ExifIFD.
fn maker_note_at(t: &Tiff, ifd0: &[Entry]) -> Result<Option<(usize, usize)>> {
    let Some((at, _)) = find(ifd0, TAG_EXIF_IFD) else {
        return Ok(None);
    };
    let (exif_ifd, _) = read_ifd(t, at as usize)?;
    Ok(find(&exif_ifd, TAG_MAKER_NOTE).map(|(at, count)| (at as usize, count as usize)))
}

/// A Leica MakerNote's IFD, out of `note`, the note's bytes from its start
/// (the offsets in it are the file's). `None` for another maker's note; an
/// error when `note` ends before its header or IFD does.
fn leica_note_ifd(note: &[u8], little_endian: bool) -> Result<Option<(Tiff<'_>, Vec<Entry>)>> {
    let header = note
        .get(..LEICA_HEADER.len())
        .ok_or_else(|| anyhow!("Leica MakerNote header out of range"))?;
    if header != LEICA_HEADER {
        return Ok(None);
    }
    let t = Tiff::with_order(note, 0, note.len(), little_endian)?;
    let (ifd, _) = read_ifd(&t, LEICA_HEADER_LEN)?;
    Ok(Some((t, ifd)))
}

/// A Leica MakerNote IFD's `PreviewImage`.
fn note_preview(ifd: &[Entry]) -> Option<Embedded> {
    ifd.iter()
        .find(|e| e.0 == TAG_LEICA_PREVIEW_IMAGE && e.2 == TYPE_UNDEFINED && e.3 > 0)
        .map(|e| Embedded {
            offset: e.1 as usize,
            length: e.3 as usize,
            codec: Codec::Jpeg,
        })
}

/// Take a Leica MakerNote's `PreviewImage` as both tiers of an `arw` whose
/// strip JPEGs all fall short of `PREVIEW_MIN_WIDTH`, when it is longer in
/// bytes than the largest of them (no tag gives its pixel size).
fn take_note_preview(arw: &mut Arw, e: Embedded) {
    if arw.full.is_none_or(|f| e.length > f.length) {
        arw.preview = Some(e);
        arw.full = Some(e);
    }
}

impl Arw {
    pub fn slice<'a>(&self, buf: &'a [u8], e: Embedded) -> &'a [u8] {
        &buf[e.offset..e.offset + e.length]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Serialize one IFD: entry count, the entries, and a null next-IFD link.
    fn ifd(entries: &[(u16, u16, u32, u32)]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        for (tag, typ, count, value) in entries {
            buf.extend_from_slice(&tag.to_le_bytes());
            buf.extend_from_slice(&typ.to_le_bytes());
            buf.extend_from_slice(&count.to_le_bytes());
            buf.extend_from_slice(&value.to_le_bytes());
        }
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf
    }

    fn ifd_len(entries: usize) -> usize {
        2 + entries * 12 + 4
    }

    /// Build a minimal little-endian TIFF whose IFD0 carries the given entries.
    fn tiff(entries: &[(u16, u16, u32, u32)]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"II\x2a\x00");
        buf.extend_from_slice(&8u32.to_le_bytes());
        buf.extend_from_slice(&ifd(entries));
        buf
    }

    const DATE: &[u8] = b"2026:09:13 09:23:33\0";
    /// Four bytes, so it rides inside the entry instead of at an offset.
    const SUBSEC: &[u8; 4] = b"122\0";

    /// Build a TIFF whose IFD0 points at an ExifIFD holding DateTimeOriginal
    /// and SubSecTimeOriginal, optionally followed by a Sony MakerNote IFD
    /// that optionally holds FocusLocation.
    fn tiff_with_exif(maker: bool, focus: Option<[u16; 4]>, sony_header: bool) -> Vec<u8> {
        tiff_with_maker(maker, focus, sony_header, None)
    }

    /// `tiff_with_exif` whose MakerNote may also carry a `FocusMode` BYTE.
    fn tiff_with_maker(
        maker: bool,
        focus: Option<[u16; 4]>,
        sony_header: bool,
        focus_mode: Option<u8>,
    ) -> Vec<u8> {
        let exif_at = 8 + ifd_len(1);
        let exif_entries = if maker { 3 } else { 2 };
        let maker_at = exif_at + ifd_len(exif_entries);
        let header_len = if sony_header { 12 } else { 0 };
        let maker_entries = usize::from(focus.is_some()) + usize::from(focus_mode.is_some());
        let maker_len = if maker {
            header_len + ifd_len(maker_entries)
        } else {
            0
        };
        let date_at = maker_at + maker_len;
        let focus_at = date_at + DATE.len();

        let mut exif = vec![
            (
                TAG_DATE_TIME_ORIGINAL,
                TYPE_ASCII,
                DATE.len() as u32,
                date_at as u32,
            ),
            (
                TAG_SUB_SEC_TIME_ORIGINAL,
                TYPE_ASCII,
                4,
                u32::from_le_bytes(*SUBSEC),
            ),
        ];
        if maker {
            exif.push((TAG_MAKER_NOTE, 7, maker_len as u32, maker_at as u32));
        }

        let mut buf = tiff(&[(TAG_EXIF_IFD, 4, 1, exif_at as u32)]);
        assert_eq!(buf.len(), exif_at);
        buf.extend_from_slice(&ifd(&exif));
        assert_eq!(buf.len(), maker_at);
        if maker {
            if sony_header {
                buf.extend_from_slice(b"SONY DSC \0\0\0");
            }
            let entries: Vec<_> = focus_mode
                .map(|m| (TAG_FOCUS_MODE, TYPE_BYTE, 1, u32::from(m)))
                .into_iter()
                .chain(focus.map(|_| (TAG_FOCUS_LOCATION, TYPE_SHORT, 4, focus_at as u32)))
                .collect();
            buf.extend_from_slice(&ifd(&entries));
        }
        assert_eq!(buf.len(), date_at);
        buf.extend_from_slice(DATE);
        if let Some(f) = focus {
            for v in f {
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }
        buf
    }

    #[test]
    fn parses_ifd0_preview_and_orientation() {
        let buf = tiff(&[
            (TAG_ORIENTATION, 3, 1, 8),
            (TAG_JPEG_OFFSET, 4, 1, 1024),
            (TAG_JPEG_LENGTH, 4, 1, 2048),
        ]);
        let a = parse(&buf).unwrap();
        assert_eq!(a.orientation, 8);
        let p = a.preview.unwrap();
        assert_eq!((p.offset, p.length), (1024, 2048));
        assert!(a.full.is_none());
    }

    /// A TIFF whose IFD0 holds a preview JPEG and whose chained IFD1 holds
    /// another JPEG of `ifd1_length` bytes.
    fn tiff_with_ifd1_jpeg(ifd1_length: u32) -> Vec<u8> {
        let ifd0 = [(TAG_JPEG_OFFSET, 4, 1, 1024), (TAG_JPEG_LENGTH, 4, 1, 2048)];
        let mut buf = tiff(&ifd0);
        let ifd1_at = buf.len() as u32;
        buf[8 + ifd_len(ifd0.len()) - 4..8 + ifd_len(ifd0.len())]
            .copy_from_slice(&ifd1_at.to_le_bytes());
        buf.extend_from_slice(&ifd(&[
            (TAG_JPEG_OFFSET, 4, 1, 4096),
            (TAG_JPEG_LENGTH, 4, 1, ifd1_length),
        ]));
        buf
    }

    #[test]
    fn falls_back_to_the_preview_over_a_smaller_ifd1_thumbnail() {
        let a = parse(&tiff_with_ifd1_jpeg(64)).unwrap();
        let (p, f) = (a.preview.unwrap(), a.full.unwrap());
        assert_eq!((p.offset, p.length), (1024, 2048));
        assert_eq!((f.offset, f.length), (p.offset, p.length));
    }

    #[test]
    fn keeps_a_chained_jpeg_larger_than_the_preview_as_full() {
        let a = parse(&tiff_with_ifd1_jpeg(8192)).unwrap();
        let f = a.full.unwrap();
        assert_eq!((f.offset, f.length), (4096, 8192));
    }

    #[test]
    fn defaults_orientation_when_absent() {
        let buf = tiff(&[(TAG_JPEG_OFFSET, 4, 1, 16), (TAG_JPEG_LENGTH, 4, 1, 32)]);
        assert_eq!(parse(&buf).unwrap().orientation, 1);
    }

    #[test]
    fn reads_a_big_endian_container() {
        use crate::jpeg::tests::W;
        let w = W(false);
        let buf = w.tiff(
            &[
                w.short(TAG_ORIENTATION, 6),
                w.ascii(TAG_MAKE, "PENTAX"),
                w.ascii(TAG_MODEL, "K-5"),
            ],
            &[
                w.ascii(TAG_DATE_TIME_ORIGINAL, "2026:09:13 09:23:33"),
                w.ascii(TAG_SUB_SEC_TIME_ORIGINAL, "12"),
                w.rational(TAG_EXPOSURE_TIME, 1, 250),
                w.rational(TAG_F_NUMBER, 28, 10),
                w.rational(TAG_FOCAL_LENGTH, 50, 1),
                // An inline SHORT is the first two bytes; the padding after
                // it is the low half of the big-endian value field.
                (TAG_ISO, TYPE_SHORT, 1, vec![0x19, 0x00, 0x4D, 0x47]),
            ],
        );
        let a = parse(&buf).unwrap();
        assert_eq!(a.orientation, 6);
        let shot = a.shot;
        assert_eq!(shot.make.as_deref(), Some("PENTAX"));
        assert_eq!(shot.model.as_deref(), Some("K-5"));
        assert_eq!(shot.capture_time.as_deref(), Some("2026:09:13 09:23:33"));
        assert_eq!(shot.subsec.as_deref(), Some("12"));
        assert_eq!(shot.exposure_time, Some(Rational { num: 1, den: 250 }));
        assert_eq!(shot.f_number, Some(Rational { num: 28, den: 10 }));
        assert_eq!(shot.focal_length, Some(Rational { num: 50, den: 1 }));
        assert_eq!(shot.iso, Some(6400));
    }

    #[test]
    fn reads_inline_bytes_and_shorts_of_a_big_endian_maker_note() {
        use crate::jpeg::tests::W;
        let w = W(false);
        let mut note = 2u16.to_be_bytes().to_vec();
        for (tag, typ, value) in [
            (TAG_FOCUS_MODE, TYPE_BYTE, [3, 0, 0, 0]),
            (TAG_RELEASE_MODE, TYPE_SHORT, [0, 2, 0, 0]),
        ] {
            note.extend_from_slice(&tag.to_be_bytes());
            note.extend_from_slice(&typ.to_be_bytes());
            note.extend_from_slice(&1u32.to_be_bytes());
            note.extend_from_slice(&value);
        }
        note.extend_from_slice(&0u32.to_be_bytes());
        let buf = w.tiff(
            &[],
            &[(TAG_MAKER_NOTE, TYPE_UNDEFINED, note.len() as u32, note)],
        );
        let shot = parse(&buf).unwrap().shot;
        assert_eq!(shot.focus_mode, Some(3));
        assert_eq!(shot.release_mode, Some(2));
    }

    #[test]
    fn reads_the_leica_focus_distance_through_a_big_endian_note() {
        use crate::jpeg::tests::W;
        let w = W(false);
        let mut note = b"LEICA\0\x02\xff".to_vec();
        note.extend_from_slice(&1u16.to_be_bytes());
        note.extend_from_slice(&TAG_LEICA_FOCUS_DISTANCE.to_be_bytes());
        note.extend_from_slice(&TYPE_LONG.to_be_bytes());
        note.extend_from_slice(&1u32.to_be_bytes());
        note.extend_from_slice(&921u32.to_be_bytes());
        note.extend_from_slice(&0u32.to_be_bytes());
        let buf = w.tiff(
            &[w.ascii(TAG_MAKE, "Leica Camera AG")],
            &[(TAG_MAKER_NOTE, TYPE_UNDEFINED, note.len() as u32, note)],
        );
        let shot = parse(&buf).unwrap().shot;
        assert_eq!(shot.focus_distance_mm, Some(921));
        assert!(shot.focus.is_none());
    }

    #[test]
    fn rejects_an_unknown_byte_order() {
        let mut buf = tiff(&[(TAG_ORIENTATION, 3, 1, 1)]);
        buf[0..2].copy_from_slice(b"XX");
        assert!(parse(&buf).is_err());
    }

    #[test]
    fn reads_capture_time_subsec_and_focus_location() {
        let buf = tiff_with_exif(true, Some([7008, 4672, 3613, 1732]), false);
        let a = parse(&buf).unwrap();
        assert_eq!(a.shot.capture_time.as_deref(), Some("2026:09:13 09:23:33"));
        assert_eq!(a.shot.subsec.as_deref(), Some("122"));
        assert_eq!(
            a.shot.focus,
            Some(FocusLocation {
                sensor_w: 7008,
                sensor_h: 4672,
                x: 3613,
                y: 1732,
            })
        );
    }

    #[test]
    fn skips_the_older_sony_maker_note_header() {
        let buf = tiff_with_exif(true, Some([6000, 4000, 100, 200]), true);
        let a = parse(&buf).unwrap();
        assert_eq!(a.shot.focus.unwrap().x, 100);
    }

    #[test]
    fn a_maker_note_without_focus_location_is_none() {
        let a = parse(&tiff_with_exif(true, None, false)).unwrap();
        assert!(a.shot.focus.is_none());
        assert_eq!(a.shot.capture_time.as_deref(), Some("2026:09:13 09:23:33"));
    }

    #[test]
    fn reads_the_sony_focus_mode() {
        let manual = parse(&tiff_with_maker(true, None, false, Some(0))).unwrap();
        assert_eq!(manual.shot.focus_mode, Some(0));
        let af_c = parse(&tiff_with_maker(
            true,
            Some([6000, 4000, 1, 2]),
            false,
            Some(3),
        ))
        .unwrap();
        assert_eq!(af_c.shot.focus_mode, Some(3));
        assert_eq!(af_c.shot.focus.unwrap().x, 1);
        let absent = parse(&tiff_with_exif(true, None, false)).unwrap();
        assert!(absent.shot.focus_mode.is_none());
    }

    #[test]
    fn reads_the_sony_electronic_front_curtain_shutter() {
        for value in [1, 0] {
            let shot = parse(&tiff_with_sony_note(
                |_| vec![(TAG_ELECTRONIC_FRONT_CURTAIN_SHUTTER, TYPE_LONG, 1, value)],
                &[],
            ))
            .unwrap()
            .shot;
            assert_eq!(shot.electronic_front_curtain, Some(value));
        }
        let absent = parse(&tiff_with_exif(true, None, false)).unwrap();
        assert!(absent.shot.electronic_front_curtain.is_none());
    }

    #[test]
    fn reads_the_sony_shooting_setup() {
        let style = b"Standard\0";
        let shot = parse(&tiff_with_sony_note(
            |at| {
                vec![
                    (TAG_AF_AREA_MODE_SETTING, TYPE_BYTE, 1, 3),
                    (TAG_RAW_FILE_TYPE, TYPE_SHORT, 1, 2),
                    (TAG_METERING_MODE2, TYPE_SHORT, 1, 0x100),
                    (TAG_CREATIVE_STYLE, TYPE_ASCII, style.len() as u32, at),
                    (TAG_DYNAMIC_RANGE_OPTIMIZER, TYPE_LONG, 1, 3),
                    (TAG_IMAGE_STABILIZATION, TYPE_LONG, 1, 1),
                    (TAG_EXPOSURE_MODE, TYPE_SHORT, 1, 15),
                    (TAG_RELEASE_MODE, TYPE_SHORT, 1, 2),
                    (TAG_SEQUENCE_NUMBER, TYPE_SHORT, 1, 3),
                ]
            },
            style,
        ))
        .unwrap()
        .shot;
        assert_eq!(shot.af_area_mode, Some(3));
        assert_eq!(shot.release_mode, Some(2));
        assert_eq!(shot.sequence_number, Some(3));
        assert_eq!(shot.image_stabilization, Some(1));
        assert_eq!(shot.exposure_mode, Some(15));
        assert_eq!(shot.metering_mode, Some(0x100));
        assert_eq!(shot.creative_style.as_deref(), Some("Standard"));
        assert_eq!(shot.dynamic_range_optimizer, Some(3));
        assert_eq!(shot.raw_file_type, Some(2));
        let absent = parse(&tiff_with_exif(true, None, false)).unwrap().shot;
        assert!(absent.af_area_mode.is_none());
        assert!(absent.release_mode.is_none());
        assert!(absent.sequence_number.is_none());
        assert!(absent.image_stabilization.is_none());
        assert!(absent.exposure_mode.is_none());
        assert!(absent.metering_mode.is_none());
        assert!(absent.creative_style.is_none());
        assert!(absent.dynamic_range_optimizer.is_none());
        assert!(absent.raw_file_type.is_none());
    }

    /// A Sony TIFF whose MakerNote holds exactly `entries`, built by `f` from
    /// the offset at which `data` is appended.
    fn tiff_with_sony_note(f: impl Fn(u32) -> Vec<(u16, u16, u32, u32)>, data: &[u8]) -> Vec<u8> {
        let exif_at = 8 + ifd_len(1);
        let maker_at = exif_at + ifd_len(1);
        let count = f(0).len();
        let data_at = maker_at + ifd_len(count);
        let mut buf = tiff(&[(TAG_EXIF_IFD, TYPE_LONG, 1, exif_at as u32)]);
        buf.extend_from_slice(&ifd(&[(
            TAG_MAKER_NOTE,
            7,
            ifd_len(count) as u32,
            maker_at as u32,
        )]));
        buf.extend_from_slice(&ifd(&f(data_at as u32)));
        buf.extend_from_slice(data);
        buf
    }

    fn shorts_le(v: &[u16]) -> Vec<u8> {
        v.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn reads_af_tracking_and_focus_frame_size() {
        let shot = parse(&tiff_with_sony_note(
            |at| {
                vec![
                    (TAG_AF_TRACKING, TYPE_BYTE, 1, 1),
                    (TAG_FOCUS_FRAME_SIZE, TYPE_UNDEFINED, 6, at),
                ]
            },
            &shorts_le(&[153, 154, 257]),
        ))
        .unwrap()
        .shot;
        assert_eq!(shot.af_tracking, Some(1));
        assert_eq!(
            shot.focus_frame,
            Some(FocusFrame {
                width: 153,
                height: 154,
            })
        );
    }

    #[test]
    fn af_tracking_and_focus_frame_size_are_independent() {
        let only_tracking = parse(&tiff_with_sony_note(
            |_| vec![(TAG_AF_TRACKING, TYPE_BYTE, 1, 2)],
            &[],
        ))
        .unwrap()
        .shot;
        assert_eq!(only_tracking.af_tracking, Some(2));
        assert!(only_tracking.focus_frame.is_none());
        let only_frame = parse(&tiff_with_sony_note(
            |at| vec![(TAG_FOCUS_FRAME_SIZE, TYPE_SHORT, 3, at)],
            &shorts_le(&[832, 740, 257]),
        ))
        .unwrap()
        .shot;
        assert!(only_frame.af_tracking.is_none());
        assert_eq!(only_frame.focus_frame.unwrap().width, 832);
    }

    #[test]
    fn a_focus_frame_flagged_unavailable_is_none() {
        let shot = parse(&tiff_with_sony_note(
            |at| vec![(TAG_FOCUS_FRAME_SIZE, TYPE_SHORT, 3, at)],
            &shorts_le(&[5519, 3864, 0]),
        ))
        .unwrap()
        .shot;
        assert!(shot.focus_frame.is_none());
    }

    #[test]
    fn wrong_type_or_count_is_none() {
        let shot = parse(&tiff_with_sony_note(
            |at| {
                vec![
                    (TAG_AF_TRACKING, TYPE_SHORT, 1, 1),
                    (TAG_FOCUS_FRAME_SIZE, TYPE_SHORT, 4, at),
                ]
            },
            &shorts_le(&[153, 154, 257, 0]),
        ))
        .unwrap()
        .shot;
        assert!(shot.af_tracking.is_none());
        assert!(shot.focus_frame.is_none());
        let long = parse(&tiff_with_sony_note(
            |at| vec![(TAG_FOCUS_FRAME_SIZE, TYPE_LONG, 3, at)],
            &[0; 12],
        ))
        .unwrap()
        .shot;
        assert!(long.focus_frame.is_none());
    }

    #[test]
    fn no_maker_note_is_none() {
        let a = parse(&tiff_with_exif(false, None, false)).unwrap();
        assert!(a.shot.focus.is_none());
        assert_eq!(a.shot.subsec.as_deref(), Some("122"));
    }

    #[test]
    fn reads_the_shooting_settings_out_of_the_exif_ifd() {
        let exif_at = 8 + ifd_len(1);
        let rationals_at = exif_at + ifd_len(3);
        let mut buf = tiff(&[(TAG_EXIF_IFD, TYPE_LONG, 1, exif_at as u32)]);
        buf.extend_from_slice(&ifd(&[
            (TAG_EXPOSURE_TIME, TYPE_RATIONAL, 1, rationals_at as u32),
            (TAG_F_NUMBER, TYPE_RATIONAL, 1, (rationals_at + 8) as u32),
            (TAG_ISO, TYPE_SHORT, 1, 6400),
        ]));
        assert_eq!(buf.len(), rationals_at);
        for v in [1u32, 250, 28, 10] {
            buf.extend_from_slice(&v.to_le_bytes());
        }

        let shot = parse(&buf).unwrap().shot;
        assert_eq!(shot.exposure_time, Some(Rational { num: 1, den: 250 }));
        assert_eq!(shot.f_number, Some(Rational { num: 28, den: 10 }));
        assert_eq!(shot.f_number.unwrap().value(), Some(2.8));
        assert_eq!(shot.iso, Some(6400));
        assert_eq!(shot.focal_length, None);
    }

    #[test]
    fn no_exif_ifd_leaves_every_field_none() {
        let a = parse(&tiff(&[(TAG_ORIENTATION, 3, 1, 1)])).unwrap();
        assert!(a.shot.capture_time.is_none() && a.shot.subsec.is_none() && a.shot.focus.is_none());
    }

    /// A prefix too short for what the offsets point at must be an error, not a
    /// silently missing value.
    #[test]
    fn offsets_past_the_buffer_are_errors() {
        assert!(parse(&tiff(&[(TAG_EXIF_IFD, 4, 1, 0xffff)])).is_err());

        let mut buf = tiff_with_exif(true, Some([1, 2, 3, 4]), false);
        buf.truncate(buf.len() - 4);
        assert!(parse(&buf).is_err());

        let mut buf = tiff_with_exif(true, Some([1, 2, 3, 4]), false);
        buf.truncate(buf.len() - DATE.len() - 8);
        assert!(parse(&buf).is_err());
    }

    fn strip_entries(
        w: u32,
        h: u32,
        photometric: u32,
        offset: u32,
        length: u32,
    ) -> Vec<(u16, u16, u32, u32)> {
        vec![
            (TAG_IMAGE_WIDTH, TYPE_LONG, 1, w),
            (TAG_IMAGE_HEIGHT, TYPE_LONG, 1, h),
            (TAG_COMPRESSION, TYPE_SHORT, 1, COMPRESSION_JPEG),
            (TAG_PHOTOMETRIC, TYPE_SHORT, 1, photometric),
            (TAG_STRIP_OFFSETS, TYPE_LONG, 1, offset),
            (TAG_STRIP_BYTE_COUNTS, TYPE_LONG, 1, length),
        ]
    }

    /// A TIFF whose IFD0 carries `ifd0` plus a SubIFDs array pointing at one
    /// IFD per element of `subs`.
    fn tiff_with_sub_ifds(
        ifd0: &[(u16, u16, u32, u32)],
        subs: &[Vec<(u16, u16, u32, u32)>],
    ) -> Vec<u8> {
        let array_at = 8 + ifd_len(ifd0.len() + 1);
        let mut at = array_at + subs.len() * 4;
        let mut offsets = Vec::new();
        for s in subs {
            offsets.push(at as u32);
            at += ifd_len(s.len());
        }
        let mut entries = ifd0.to_vec();
        entries.push((TAG_SUB_IFDS, TYPE_LONG, subs.len() as u32, array_at as u32));
        let mut buf = tiff(&entries);
        for o in &offsets {
            buf.extend_from_slice(&o.to_le_bytes());
        }
        for s in subs {
            buf.extend_from_slice(&ifd(s));
        }
        buf
    }

    #[test]
    fn picks_dng_strip_jpegs_by_size_not_by_index() {
        let buf = tiff_with_sub_ifds(
            &strip_entries(9536, 6336, 32803, 900_000, 5_000_000),
            &[
                strip_entries(160, 120, PHOTOMETRIC_YCBCR, 100, 10),
                strip_entries(720, 480, PHOTOMETRIC_YCBCR, 200, 20),
                strip_entries(9504, 6320, PHOTOMETRIC_YCBCR, 300, 30),
                strip_entries(2112, 1408, PHOTOMETRIC_YCBCR, 400, 40),
            ],
        );
        let a = parse(&buf).unwrap();
        let (p, f) = (a.preview.unwrap(), a.full.unwrap());
        assert_eq!((p.offset, p.length), (400, 40));
        assert_eq!((f.offset, f.length), (300, 30));
    }

    #[test]
    fn short_entries_ignore_the_padding_in_their_high_half() {
        let padded = |w, h, offset, length| {
            let mut e = strip_entries(w, h, PHOTOMETRIC_YCBCR, offset, length);
            e[2].3 |= 0xFFFF_0000;
            e[3].3 |= 0x4D47_0000;
            e
        };
        let buf = tiff_with_sub_ifds(
            &strip_entries(9536, 6336, 32803, 900_000, 5_000_000),
            &[
                padded(640, 480, 100, 10),
                padded(9520, 6328, 200, 20),
                padded(1620, 1080, 300, 30),
            ],
        );
        let a = parse(&buf).unwrap();
        let (p, f) = (a.preview.unwrap(), a.full.unwrap());
        assert_eq!((p.offset, p.length), (300, 30));
        assert_eq!((f.offset, f.length), (200, 20));
    }

    /// `ifd` in big-endian order. A single SHORT goes in the first two bytes
    /// of the value field, followed by nonzero padding.
    fn ifd_be(entries: &[(u16, u16, u32, u32)]) -> Vec<u8> {
        let mut buf = (entries.len() as u16).to_be_bytes().to_vec();
        for &(tag, typ, count, value) in entries {
            buf.extend_from_slice(&tag.to_be_bytes());
            buf.extend_from_slice(&typ.to_be_bytes());
            buf.extend_from_slice(&count.to_be_bytes());
            if typ == TYPE_SHORT && count == 1 {
                buf.extend_from_slice(&(value as u16).to_be_bytes());
                buf.extend_from_slice(b"MG");
            } else {
                buf.extend_from_slice(&value.to_be_bytes());
            }
        }
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf
    }

    #[test]
    fn picks_dng_strip_jpegs_out_of_a_big_endian_sub_ifd_array() {
        let ifd0 = strip_entries(9536, 6336, 32803, 900_000, 5_000_000);
        let subs = [
            strip_entries(640, 480, PHOTOMETRIC_YCBCR, 100, 10),
            strip_entries(9520, 6328, PHOTOMETRIC_YCBCR, 200, 20),
            strip_entries(1620, 1080, PHOTOMETRIC_YCBCR, 300, 30),
        ];
        let array_at = 8 + ifd_len(ifd0.len() + 1);
        let mut at = array_at + subs.len() * 4;
        let mut offsets = Vec::new();
        for s in &subs {
            offsets.push(at as u32);
            at += ifd_len(s.len());
        }
        let mut entries = ifd0;
        entries.push((TAG_SUB_IFDS, TYPE_LONG, subs.len() as u32, array_at as u32));
        let mut buf = b"MM\0\x2a".to_vec();
        buf.extend_from_slice(&8u32.to_be_bytes());
        buf.extend_from_slice(&ifd_be(&entries));
        for o in offsets {
            buf.extend_from_slice(&o.to_be_bytes());
        }
        for s in &subs {
            buf.extend_from_slice(&ifd_be(s));
        }
        let a = parse(&buf).unwrap();
        let (p, f) = (a.preview.unwrap(), a.full.unwrap());
        assert_eq!((p.offset, p.length), (300, 30));
        assert_eq!((f.offset, f.length), (200, 20));
    }

    #[test]
    fn a_cfa_raw_in_ifd0_is_not_a_jpeg() {
        let buf = tiff(&strip_entries(9536, 6336, 32803, 900_000, 5_000_000));
        let a = parse(&buf).unwrap();
        assert!(a.preview.is_none() && a.full.is_none());
    }

    #[test]
    fn a_non_sony_maker_note_is_skipped() {
        let make = b"Leica Camera AG\0";
        let exif_at = 8 + ifd_len(2);
        let maker_at = exif_at + ifd_len(1);
        let note = leica_note(&[]);
        let make_at = maker_at + note.len();
        let mut buf = tiff(&[
            (TAG_MAKE, TYPE_ASCII, make.len() as u32, make_at as u32),
            (TAG_EXIF_IFD, TYPE_LONG, 1, exif_at as u32),
        ]);
        buf.extend_from_slice(&ifd(&[(
            TAG_MAKER_NOTE,
            7,
            note.len() as u32,
            maker_at as u32,
        )]));
        buf.extend_from_slice(&note);
        buf.extend_from_slice(make);
        let a = parse(&buf).unwrap();
        assert!(a.shot.focus.is_none());
        assert!(a.shot.focus_mode.is_none());
        assert!(a.shot.af_tracking.is_none());
        assert!(a.shot.electronic_front_curtain.is_none());
        assert!(a.shot.focus_frame.is_none());
        assert!(a.shot.focus_distance_mm.is_none());
        assert!(a.shot.af_area_mode.is_none());
        assert!(a.shot.release_mode.is_none());
        assert!(a.shot.sequence_number.is_none());
        assert!(a.shot.image_stabilization.is_none());
        assert!(a.shot.exposure_mode.is_none());
        assert!(a.shot.metering_mode.is_none());
        assert!(a.shot.creative_style.is_none());
        assert!(a.shot.dynamic_range_optimizer.is_none());
        assert!(a.shot.raw_file_type.is_none());
        assert_eq!(a.shot.make.as_deref(), Some("Leica Camera AG"));
    }

    #[test]
    fn no_embedded_jpeg_at_all_parses_with_both_none() {
        let buf = tiff_with_sub_ifds(
            &[(TAG_ORIENTATION, TYPE_SHORT, 1, 1)],
            &[
                vec![(TAG_IMAGE_WIDTH, TYPE_LONG, 1, 100)],
                vec![(TAG_IMAGE_WIDTH, TYPE_LONG, 1, 200)],
            ],
        );
        let a = parse(&buf).unwrap();
        assert!(a.preview.is_none() && a.full.is_none());
    }

    fn leica_note(entries: &[(u16, u16, u32, u32)]) -> Vec<u8> {
        let mut note = b"LEICA\0\x02\0".to_vec();
        note.extend_from_slice(&ifd(entries));
        note
    }

    /// A Leica-made TIFF whose ExifIFD holds only a MakerNote with the given
    /// bytes.
    fn tiff_with_leica_maker_note(note: &[u8]) -> Vec<u8> {
        let make = b"Leica Camera AG\0";
        let exif_at = 8 + ifd_len(2);
        let maker_at = exif_at + ifd_len(1);
        let make_at = maker_at + note.len();
        let mut buf = tiff(&[
            (TAG_MAKE, TYPE_ASCII, make.len() as u32, make_at as u32),
            (TAG_EXIF_IFD, TYPE_LONG, 1, exif_at as u32),
        ]);
        buf.extend_from_slice(&ifd(&[(
            TAG_MAKER_NOTE,
            7,
            note.len() as u32,
            maker_at as u32,
        )]));
        buf.extend_from_slice(note);
        buf.extend_from_slice(make);
        buf
    }

    #[test]
    fn reads_the_leica_focus_distance() {
        let note = leica_note(&[
            (0x0310, 1, 4, 0),
            (TAG_LEICA_FOCUS_DISTANCE, TYPE_LONG, 1, 921),
        ]);
        let shot = parse(&tiff_with_leica_maker_note(&note)).unwrap().shot;
        assert_eq!(shot.focus_distance_mm, Some(921));
        assert!(shot.focus.is_none());
        assert!(shot.af_tracking.is_none() && shot.focus_frame.is_none());
    }

    /// The offset and length of the 160x120 strip JPEG in `leica_dng`'s IFD0.
    pub(crate) const LEICA_STRIP: (usize, usize) = (2048, 100);
    /// The `FocusDistance` of `leica_dng`'s note.
    pub(crate) const LEICA_DISTANCE: u32 = 921;

    /// A Leica-made little-endian DNG like the M (Typ 240)'s: IFD0 holds the
    /// `LEICA_STRIP` JPEG, the SubIFDs `subs`, and a Leica MakerNote at
    /// `note_at` the `LEICA_DISTANCE` and a `PreviewImage` of `preview_len`
    /// bytes at `preview_at`. The file ends with the note.
    pub(crate) fn leica_dng(
        subs: &[Vec<(u16, u16, u32, u32)>],
        note_at: usize,
        preview_at: usize,
        preview_len: usize,
    ) -> Vec<u8> {
        let make = b"Leica Camera AG\0";
        let ifd0 = |exif_at: usize| {
            let (strip_at, strip_len) = LEICA_STRIP;
            let mut e = strip_entries(
                160,
                120,
                PHOTOMETRIC_YCBCR,
                strip_at as u32,
                strip_len as u32,
            );
            e.push((
                TAG_MAKE,
                TYPE_ASCII,
                make.len() as u32,
                (exif_at + ifd_len(1)) as u32,
            ));
            e.push((TAG_EXIF_IFD, TYPE_LONG, 1, exif_at as u32));
            e
        };
        let exif_at = tiff_with_sub_ifds(&ifd0(0), subs).len();
        let note = leica_note(&[
            (
                TAG_LEICA_PREVIEW_IMAGE,
                TYPE_UNDEFINED,
                preview_len as u32,
                preview_at as u32,
            ),
            (TAG_LEICA_FOCUS_DISTANCE, TYPE_LONG, 1, LEICA_DISTANCE),
        ]);
        let mut buf = tiff_with_sub_ifds(&ifd0(exif_at), subs);
        buf.extend_from_slice(&ifd(&[(
            TAG_MAKER_NOTE,
            TYPE_UNDEFINED,
            note.len() as u32,
            note_at as u32,
        )]));
        buf.extend_from_slice(make);
        assert!(buf.len() <= LEICA_STRIP.0 && LEICA_STRIP.0 + LEICA_STRIP.1 <= note_at);
        buf.resize(note_at, 0);
        buf.extend_from_slice(&note);
        buf
    }

    fn at(e: Option<Embedded>) -> Option<(usize, usize)> {
        e.map(|e| (e.offset, e.length))
    }

    #[test]
    fn a_leica_note_preview_is_taken_over_a_thumbnail_strip() {
        let note_at = 4096;
        let buf = leica_dng(&[], note_at, note_at + 1024, 386_048);
        let (a, far) = parse_noted(&buf).unwrap();
        assert_eq!(far, None);
        assert_eq!(at(a.preview), Some((note_at + 1024, 386_048)));
        assert_eq!(at(a.full), Some((note_at + 1024, 386_048)));
        assert_eq!(a.shot.focus_distance_mm, Some(LEICA_DISTANCE));

        // A note JPEG no longer than the strip is not taken.
        let buf = leica_dng(&[], note_at, note_at + 1024, LEICA_STRIP.1);
        let a = parse(&buf).unwrap();
        assert_eq!(at(a.preview), Some(LEICA_STRIP));
        assert_eq!(at(a.full), Some(LEICA_STRIP));
    }

    #[test]
    fn a_leica_note_is_ignored_when_a_strip_reaches_the_preview_width() {
        let subs = [
            strip_entries(1620, 1080, PHOTOMETRIC_YCBCR, 3000, 50),
            strip_entries(640, 480, PHOTOMETRIC_YCBCR, 3100, 20),
        ];
        let note_at = 4096;
        let buf = leica_dng(&subs, note_at, 5000, 386_048);
        let (a, far) = parse_noted(&buf).unwrap();
        assert_eq!(far, None);
        assert_eq!(at(a.preview), Some((3000, 50)));
        assert_eq!(at(a.full), Some((3000, 50)));

        // Past the buffer too: the note gives its focus distance only.
        let (mut a, far) = parse_noted(&buf[..note_at]).unwrap();
        far.unwrap().take(&mut a, &buf[note_at..], 1 << 20);
        assert_eq!(at(a.preview), Some((3000, 50)));
        assert_eq!(at(a.full), Some((3000, 50)));
        assert_eq!(a.shot.focus_distance_mm, Some(LEICA_DISTANCE));
    }

    #[test]
    fn a_leica_note_past_the_buffer_is_handed_back() {
        let note_at = 4096;
        let buf = leica_dng(&[], note_at, note_at + 1024, 386_048);
        let (mut a, far) = parse_noted(&buf[..note_at + 16]).unwrap();
        let far = far.unwrap();
        assert_eq!((far.offset, far.length), (note_at, buf.len() - note_at));
        assert_eq!(at(a.preview), Some(LEICA_STRIP));
        assert_eq!(a.shot.focus_distance_mm, None);

        far.take(&mut a, &buf[note_at..], (note_at + 1024 + 386_048) as u64);
        assert_eq!(at(a.preview), Some((note_at + 1024, 386_048)));
        assert_eq!(at(a.full), Some((note_at + 1024, 386_048)));
        assert_eq!(a.shot.focus_distance_mm, Some(LEICA_DISTANCE));

        // A JPEG running past the end of the file is not taken.
        let (mut a, _) = parse_noted(&buf[..note_at + 16]).unwrap();
        far.take(&mut a, &buf[note_at..], (note_at + 1024) as u64);
        assert_eq!(at(a.full), Some(LEICA_STRIP));
    }

    #[test]
    fn a_sony_maker_note_has_no_focus_distance() {
        let shot = parse(&tiff_with_exif(true, Some([1, 2, 3, 4]), true))
            .unwrap()
            .shot;
        assert!(shot.focus.is_some());
        assert!(shot.focus_distance_mm.is_none());
    }

    /// ExifIFD holding `ApertureValue` and, optionally, `FNumber`.
    fn tiff_with_aperture(av: (u32, u32), f_number: Option<(u32, u32)>) -> Vec<u8> {
        let exif_at = 8 + ifd_len(1);
        let entries = 1 + usize::from(f_number.is_some());
        let rationals_at = exif_at + ifd_len(entries);
        let mut exif = vec![(TAG_APERTURE_VALUE, TYPE_RATIONAL, 1, rationals_at as u32)];
        if f_number.is_some() {
            exif.push((TAG_F_NUMBER, TYPE_RATIONAL, 1, (rationals_at + 8) as u32));
        }
        let mut buf = tiff(&[(TAG_EXIF_IFD, TYPE_LONG, 1, exif_at as u32)]);
        buf.extend_from_slice(&ifd(&exif));
        for v in [av.0, av.1] {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        if let Some((n, d)) = f_number {
            buf.extend_from_slice(&n.to_le_bytes());
            buf.extend_from_slice(&d.to_le_bytes());
        }
        buf
    }

    #[test]
    fn estimates_the_f_number_from_the_apex_aperture_value() {
        // The M11-P writes AV 297/100 for f/2.8.
        let shot = parse(&tiff_with_aperture((297, 100), None)).unwrap().shot;
        assert!(shot.f_number.is_none());
        let f = shot.estimated_f_number.unwrap();
        assert!((f - 2.8).abs() < 0.01, "{f}");

        let shot = parse(&tiff_with_aperture((4, 1), None)).unwrap().shot;
        assert_eq!(shot.estimated_f_number, Some(4.0));
    }

    #[test]
    fn f_number_wins_over_aperture_value() {
        let shot = parse(&tiff_with_aperture((4, 1), Some((28, 10))))
            .unwrap()
            .shot;
        assert_eq!(shot.f_number, Some(Rational { num: 28, den: 10 }));
        assert!(shot.estimated_f_number.is_none());
    }

    fn sigma_note(entries: &[(u16, u16, u32, u32)]) -> Vec<u8> {
        let mut note = b"SIGMA\0\0\0\x01\x04".to_vec();
        note.extend_from_slice(&ifd(entries));
        note
    }

    /// A Sigma-made TIFF with the given `Model` whose ExifIFD holds only a
    /// MakerNote with the given bytes.
    fn tiff_with_sigma_maker_note(model: &str, note: &[u8]) -> Vec<u8> {
        let make = b"Sigma\0";
        let model = [model.as_bytes(), b"\0"].concat();
        let exif_at = 8 + ifd_len(3);
        let maker_at = exif_at + ifd_len(1);
        let make_at = maker_at + note.len();
        let model_at = make_at + make.len();
        let mut buf = tiff(&[
            (TAG_MAKE, TYPE_ASCII, make.len() as u32, make_at as u32),
            (TAG_MODEL, TYPE_ASCII, model.len() as u32, model_at as u32),
            (TAG_EXIF_IFD, TYPE_LONG, 1, exif_at as u32),
        ]);
        buf.extend_from_slice(&ifd(&[(
            TAG_MAKER_NOTE,
            7,
            note.len() as u32,
            maker_at as u32,
        )]));
        buf.extend_from_slice(note);
        buf.extend_from_slice(make);
        buf.extend_from_slice(&model);
        buf
    }

    fn sigma_point(x: u16, y: u16) -> u32 {
        u32::from(x) | u32::from(y) << 16
    }

    #[test]
    fn reads_the_sigma_bf_af_point() {
        let note = sigma_note(&[
            (0x0146, TYPE_BYTE, 1, 0),
            (TAG_SIGMA_AF_POINT, TYPE_SHORT, 2, sigma_point(386, 323)),
        ]);
        let shot = parse(&tiff_with_sigma_maker_note("Sigma BF", &note))
            .unwrap()
            .shot;
        assert_eq!(
            shot.focus,
            Some(FocusLocation {
                sensor_w: 1000,
                sensor_h: 667,
                x: 386,
                y: 323,
            })
        );
        assert!(shot.focus_mode.is_none());
        assert!(shot.af_tracking.is_none());
        assert!(shot.focus_frame.is_none());
        assert!(shot.focus_distance_mm.is_none());
        assert!(shot.af_area_mode.is_none());
        assert!(shot.release_mode.is_none());
        assert!(shot.sequence_number.is_none());
        assert!(shot.image_stabilization.is_none());
        assert!(shot.exposure_mode.is_none());
        assert!(shot.metering_mode.is_none());
        assert!(shot.creative_style.is_none());
        assert!(shot.dynamic_range_optimizer.is_none());
        assert!(shot.raw_file_type.is_none());
    }

    #[test]
    fn keeps_a_sigma_bf_af_point_outside_the_grid() {
        let note = sigma_note(&[(TAG_SIGMA_AF_POINT, TYPE_SHORT, 2, sigma_point(1200, 900))]);
        let focus = parse(&tiff_with_sigma_maker_note("Sigma BF", &note))
            .unwrap()
            .shot
            .focus
            .unwrap();
        assert_eq!((focus.x, focus.y), (1200, 900));
    }

    #[test]
    fn a_sigma_note_without_the_af_point_is_none() {
        let note = sigma_note(&[(0x0146, TYPE_BYTE, 1, 0)]);
        let shot = parse(&tiff_with_sigma_maker_note("Sigma BF", &note))
            .unwrap()
            .shot;
        assert!(shot.focus.is_none());
    }

    #[test]
    fn another_sigma_model_ignores_the_af_point() {
        let note = sigma_note(&[(TAG_SIGMA_AF_POINT, TYPE_SHORT, 2, sigma_point(386, 323))]);
        let shot = parse(&tiff_with_sigma_maker_note("Sigma fp L", &note))
            .unwrap()
            .shot;
        assert!(shot.focus.is_none());
    }

    #[test]
    fn a_sigma_bf_maker_note_with_an_inline_value_is_none() {
        // count <= SIGMA_HEADER_LEN means the entry's value field holds the
        // MakerNote bytes inline, not an offset. Reading it as an offset
        // (0xffff_fff0, past the buffer) must not bail the whole parse.
        let make = b"Sigma\0";
        let model = b"Sigma BF\0";
        let exif_at = 8 + ifd_len(3);
        let maker_at = exif_at + ifd_len(1);
        let make_at = maker_at;
        let model_at = make_at + make.len();
        let mut buf = tiff(&[
            (TAG_MAKE, TYPE_ASCII, make.len() as u32, make_at as u32),
            (TAG_MODEL, TYPE_ASCII, model.len() as u32, model_at as u32),
            (TAG_EXIF_IFD, TYPE_LONG, 1, exif_at as u32),
        ]);
        buf.extend_from_slice(&ifd(&[(TAG_MAKER_NOTE, 7, 4, 0xffff_fff0)]));
        buf.extend_from_slice(make);
        buf.extend_from_slice(model);
        let shot = parse(&buf).unwrap().shot;
        assert!(shot.focus.is_none());
    }

    #[test]
    fn a_sigma_af_point_of_the_wrong_shape_is_none() {
        for (typ, count) in [(TYPE_LONG, 2), (TYPE_SHORT, 1), (TYPE_SHORT, 4)] {
            let note = sigma_note(&[(TAG_SIGMA_AF_POINT, typ, count, sigma_point(386, 323))]);
            let shot = parse(&tiff_with_sigma_maker_note("Sigma BF", &note))
                .unwrap()
                .shot;
            assert!(shot.focus.is_none(), "{typ} {count}");
        }
    }
}
