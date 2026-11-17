//! Decode the HEVC `PRVW` / `THMB` of a CR3 shot with HDR PQ on into an sRGB
//! JPEG, so every consumer of the reader's bytes gets a JPEG as for any other
//! file. This is the only module that names `hpvcd`.
//!
//! The payload is the box's 16-byte version-1 header (the visible width and
//! height as u16 at 6 and 8) followed by `CISZ`, `hvcC`, `colr`, `pixi` and
//! `IMGD` boxes, the last holding a u32 total length and then 4-byte
//! length-prefixed NAL units. The frame is coded in whole CTBs (1664x1088 for
//! a 1620x1080 `PRVW`) with no conformance window, so it is cropped to the
//! header's size.

use anyhow::{anyhow, bail, ensure, Result};
use rayon::prelude::*;

use crate::cr3::{child, children, IMAGE_HEADER_LEN};

/// `hvcC`'s fixed fields before `numOfArrays` (ISO/IEC 14496-15 8.3.3.1).
const HVCC_FIXED: usize = 22;
const START_CODE: [u8; 4] = [0, 0, 0, 1];
/// The decoded image replaces the camera's own preview JPEG, which the viewer
/// shows and the scan scales down again, so it is kept close to lossless.
const QUALITY: f32 = 90.0;
/// The curves are tabulated at this many steps over 0..=1 and interpolated:
/// three `powf` per channel per pixel cost more than the decode itself.
const CURVE_STEPS: usize = 4096;

/// SMPTE ST 2084 (PQ) EOTF constants.
const PQ_M1: f32 = 2610.0 / 16384.0;
const PQ_M2: f32 = 2523.0 / 4096.0 * 128.0;
const PQ_C1: f32 = 3424.0 / 4096.0;
const PQ_C2: f32 = 2413.0 / 4096.0 * 32.0;
const PQ_C3: f32 = 2392.0 / 4096.0 * 32.0;
/// PQ's peak luminance, and the reference (diffuse) white it is scaled to 1.0
/// by (ITU-R BT.2408).
const PQ_PEAK_NITS: f32 = 10000.0;
const REFERENCE_WHITE_NITS: f32 = 203.0;
/// Full-range BT.2020 non-constant-luminance Y'CbCr to R'G'B' (ITU-R BT.2020,
/// Kr 0.2627, Kb 0.0593).
const CR_TO_R: f32 = 1.4746;
const CB_TO_G: f32 = 0.16455;
const CR_TO_G: f32 = 0.57135;
const CB_TO_B: f32 = 1.8814;
/// Linear BT.2020 to linear BT.709 primaries (ITU-R BT.2087).
const BT2020_TO_BT709: [[f32; 3]; 3] = [
    [1.6605, -0.5876, -0.0728],
    [-0.1246, 1.1329, -0.0083],
    [-0.0182, -0.1006, 1.1187],
];
/// The highlight roll-off `v / (1 + v / KNEE) * GAIN` after reference white is
/// scaled to 1.0; `GAIN` = 1 + 1 / `KNEE` keeps reference white at 1.0. A
/// visual choice, not Canon's rendering.
const KNEE: f32 = 4.0;
const GAIN: f32 = 1.25;

/// Decode the payload of an HEVC `PRVW` / `THMB` (its header included) into a
/// JPEG of the header's visible size.
pub fn to_jpeg(payload: &[u8]) -> Result<Vec<u8>> {
    let (width, height, stream) = annex_b(payload)?;
    let frames = hpvcd::decode_hevc(&stream).map_err(|e| anyhow!("HEVC preview: {e}"))?;
    let frame = frames
        .first()
        .ok_or_else(|| anyhow!("HEVC preview has no frame"))?;
    let rgb = to_rgb(&frame.to_yuv(), width, height)?;

    let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    // libjpeg's baseline settings: mozjpeg's trellis and scan search take
    // longer than the decode, for a JPEG that lives only in memory.
    c.set_fastest_defaults();
    c.set_size(width, height);
    c.set_quality(QUALITY);
    let mut c = c.start_compress(Vec::new())?;
    c.write_scanlines(&rgb)?;
    Ok(c.finish()?)
}

/// The header's visible width and height, and the Annex B stream of the
/// parameter sets in `hvcC` followed by the NAL units in `IMGD`.
fn annex_b(payload: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
    ensure!(
        payload.len() >= IMAGE_HEADER_LEN,
        "HEVC preview header cut short"
    );
    let u16_at = |at: usize| usize::from(u16::from_be_bytes([payload[at], payload[at + 1]]));
    let (width, height) = (u16_at(6), u16_at(8));
    ensure!(width > 0 && height > 0, "HEVC preview has no size");

    let boxes = children(payload, IMAGE_HEADER_LEN, payload.len())?;
    let hvcc = child(&boxes, b"hvcC").ok_or_else(|| anyhow!("HEVC preview without hvcC"))?;
    let imgd = child(&boxes, b"IMGD").ok_or_else(|| anyhow!("HEVC preview without IMGD"))?;
    let field = |at: usize, n: usize, end: usize| {
        at.checked_add(n)
            .filter(|&e| e <= end)
            .map(|e| &payload[at..e])
            .ok_or_else(|| anyhow!("HEVC preview box cut short"))
    };
    let be = |b: &[u8]| b.iter().fold(0usize, |v, &x| (v << 8) | usize::from(x));

    let mut out = Vec::new();
    let mut at = hvcc.payload + HVCC_FIXED;
    let arrays = be(field(at, 1, hvcc.end)?);
    at += 1;
    for _ in 0..arrays {
        // `array_completeness` / `NAL_unit_type`, then `numNalus`.
        let nalus = be(field(at + 1, 2, hvcc.end)?);
        at += 3;
        for _ in 0..nalus {
            let length = be(field(at, 2, hvcc.end)?);
            out.extend_from_slice(&START_CODE);
            out.extend_from_slice(field(at + 2, length, hvcc.end)?);
            at += 2 + length;
        }
    }

    // `IMGD` starts with the total length of the NAL units that follow.
    let mut at = imgd.payload + 4;
    while at < imgd.end {
        let length = be(field(at, 4, imgd.end)?);
        out.extend_from_slice(&START_CODE);
        out.extend_from_slice(field(at + 4, length, imgd.end)?);
        at += 4 + length;
    }
    Ok((width, height, out))
}

/// The top-left `width` x `height` of a decoded frame, tone-mapped to 8-bit
/// sRGB. The frame is taken as the full-range BT.2020 / PQ the Canon bodies
/// write, whatever its VUI signals.
fn to_rgb(yuv: &hpvcd::FrameYuv, width: usize, height: usize) -> Result<Vec<u8>> {
    let (y, cb, cr) = match (yuv.y.as_u16(), yuv.cb.as_u16(), yuv.cr.as_u16()) {
        (Some(y), Some(cb), Some(cr)) => (y, cb, cr),
        _ => bail!("HEVC preview is not 10-bit or deeper"),
    };
    let (cw, ch) = (yuv.chroma_width, yuv.chroma_height);
    ensure!(
        width <= yuv.width && height <= yuv.height && cw > 0 && ch > 0,
        "HEVC preview frame smaller than its header"
    );
    let (sx, sy) = (yuv.width.div_ceil(cw), yuv.height.div_ceil(ch));
    let max = ((1u32 << yuv.bit_depth) - 1) as f32;
    let mid = (1u32 << (yuv.bit_depth - 1)) as f32;

    let curves = Curves::new();
    let mut rgb = vec![0u8; width * height * 3];
    rgb.par_chunks_mut(width * 3)
        .enumerate()
        .for_each(|(j, row)| {
            for (i, px) in row.chunks_exact_mut(3).enumerate() {
                let c = j / sy * cw + i / sx;
                px.copy_from_slice(&curves.tone_map(
                    f32::from(y[j * yuv.width + i]) / max,
                    (f32::from(cb[c]) - mid) / max,
                    (f32::from(cr[c]) - mid) / max,
                ));
            }
        });
    Ok(rgb)
}

/// The per-channel curves of the tone map, tabulated over 0..=1.
struct Curves {
    /// A PQ code value to linear light, 1.0 at reference white.
    pq: Vec<f32>,
    /// Linear light to the rolled-off, sRGB-encoded value. The roll-off
    /// reaches 1.0 at reference white, so every brighter value clips.
    out: Vec<f32>,
}

impl Curves {
    fn new() -> Self {
        let table = |f: &dyn Fn(f32) -> f32| {
            (0..=CURVE_STEPS)
                .map(|i| f(i as f32 / CURVE_STEPS as f32))
                .collect()
        };
        Curves {
            pq: table(&|e| pq_eotf(e) * PQ_PEAK_NITS / REFERENCE_WHITE_NITS),
            out: table(&|v| srgb(v / (1.0 + v / KNEE) * GAIN)),
        }
    }

    /// One full-range BT.2020 PQ Y'CbCr pixel (Y' in 0..=1, Cb / Cr in
    /// -0.5..=0.5) to 8-bit sRGB.
    fn tone_map(&self, y: f32, cb: f32, cr: f32) -> [u8; 3] {
        let rgb = [
            y + CR_TO_R * cr,
            y - CB_TO_G * cb - CR_TO_G * cr,
            y + CB_TO_B * cb,
        ]
        .map(|v| lerp(&self.pq, v));
        BT2020_TO_BT709.map(|row| {
            let v = row[0] * rgb[0] + row[1] * rgb[1] + row[2] * rgb[2];
            (lerp(&self.out, v) * 255.0).round() as u8
        })
    }
}

/// `table` at `x`, clamped to 0..=1.
fn lerp(table: &[f32], x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0) * CURVE_STEPS as f32;
    let i = (x as usize).min(CURVE_STEPS - 1);
    table[i] + (table[i + 1] - table[i]) * (x - i as f32)
}

/// ST 2084: a PQ code value in 0..=1 to linear light, 1.0 at `PQ_PEAK_NITS`.
fn pq_eotf(e: f32) -> f32 {
    let p = e.clamp(0.0, 1.0).powf(1.0 / PQ_M2);
    ((p - PQ_C1).max(0.0) / (PQ_C2 - PQ_C3 * p)).powf(1.0 / PQ_M1)
}

/// The sRGB OETF (IEC 61966-2-1).
fn srgb(l: f32) -> f32 {
    let l = l.clamp(0.0, 1.0);
    if l <= 0.003_130_8 {
        12.92 * l
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(typ: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = ((8 + payload.len()) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(typ);
        out.extend_from_slice(payload);
        out
    }

    /// A version-1 header of `width` x `height`, then `boxes`.
    fn payload(width: u16, height: u16, boxes: &[Vec<u8>]) -> Vec<u8> {
        let body = boxes.concat();
        let mut out = vec![1, 0, 0, 0, 0, 2];
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&[0xff, 0xff]);
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn hvcc(arrays: &[(u8, &[&[u8]])]) -> Vec<u8> {
        let mut out = vec![1; HVCC_FIXED];
        out.push(arrays.len() as u8);
        for (typ, nalus) in arrays {
            out.push(0x80 | typ);
            out.extend_from_slice(&(nalus.len() as u16).to_be_bytes());
            for n in *nalus {
                out.extend_from_slice(&(n.len() as u16).to_be_bytes());
                out.extend_from_slice(n);
            }
        }
        bx(b"hvcC", &out)
    }

    fn imgd(nalus: &[&[u8]]) -> Vec<u8> {
        let body: Vec<u8> = nalus
            .iter()
            .flat_map(|n| [(n.len() as u32).to_be_bytes().to_vec(), n.to_vec()].concat())
            .collect();
        bx(
            b"IMGD",
            &[(body.len() as u32).to_be_bytes().to_vec(), body].concat(),
        )
    }

    #[test]
    fn the_parameter_sets_and_slices_become_an_annex_b_stream() {
        let p = payload(
            1620,
            1080,
            &[
                bx(b"CISZ", &[0; 12]),
                hvcc(&[
                    (32, &[&[0x40, 1]]),
                    (33, &[&[0x42, 1, 2]]),
                    (34, &[&[0x44]]),
                ]),
                bx(b"colr", b"nclx\0\x09\0\x10\0\x09\x80"),
                bx(b"pixi", &[0, 0, 0, 0, 3, 10, 10, 10]),
                imgd(&[&[0x26, 1, 0xaa], &[0x26, 1, 0xbb, 0xcc]]),
                bx(b"free", &[0; 5]),
            ],
        );
        let (w, h, stream) = annex_b(&p).unwrap();
        assert_eq!((w, h), (1620, 1080));
        assert_eq!(
            stream,
            [
                &[0, 0, 0, 1, 0x40, 1][..],
                &[0, 0, 0, 1, 0x42, 1, 2],
                &[0, 0, 0, 1, 0x44],
                &[0, 0, 0, 1, 0x26, 1, 0xaa],
                &[0, 0, 0, 1, 0x26, 1, 0xbb, 0xcc],
            ]
            .concat()
        );
    }

    #[test]
    fn a_missing_or_cut_box_is_an_error() {
        let ok = [hvcc(&[(32, &[&[0x40, 1]])]), imgd(&[&[0x26, 1]])];
        assert!(annex_b(&payload(320, 214, &ok)).is_ok());
        assert!(annex_b(&payload(320, 214, &ok[..1])).is_err(), "no IMGD");
        assert!(annex_b(&payload(320, 214, &ok[1..])).is_err(), "no hvcC");
        assert!(annex_b(&payload(0, 214, &ok)).is_err(), "no size");
        assert!(annex_b(&[0, 0, 0, 1]).is_err(), "header cut short");

        let mut long = payload(320, 214, &ok);
        let n = long.len();
        long[n - 3] = 9;
        assert!(annex_b(&long).is_err(), "a NAL unit past IMGD");

        let mut cut = payload(320, 214, &ok);
        cut.truncate(cut.len() - 1);
        assert!(annex_b(&cut).is_err(), "IMGD past the payload");
    }

    /// ST 2084's inverse EOTF: `nits` to a PQ code value.
    fn pq(nits: f32) -> f32 {
        let l = (nits / PQ_PEAK_NITS).powf(PQ_M1);
        ((PQ_C1 + PQ_C2 * l) / (1.0 + PQ_C3 * l)).powf(PQ_M2)
    }

    #[test]
    fn pq_black_is_black_and_reference_white_is_white() {
        let c = Curves::new();
        assert_eq!(c.tone_map(0.0, 0.0, 0.0), [0, 0, 0]);
        for v in c.tone_map(pq(REFERENCE_WHITE_NITS), 0.0, 0.0) {
            assert!(v >= 254, "{v}");
        }
    }

    #[test]
    fn grays_stay_neutral_and_brighten_with_luminance() {
        let c = Curves::new();
        let mut last = 0;
        for i in 0..=100 {
            let [r, g, b] = c.tone_map(pq(REFERENCE_WHITE_NITS) * i as f32 / 100.0, 0.0, 0.0);
            assert!(r.abs_diff(g) <= 1 && g.abs_diff(b) <= 1, "{i}: {r} {g} {b}");
            assert!(g >= last, "{i}: {g} < {last}");
            last = g;
        }
        assert_eq!(last, 255);
    }

    #[test]
    fn the_tables_follow_the_curves() {
        let c = Curves::new();
        for i in 0..=1000 {
            let x = i as f32 / 1000.0;
            let pq = pq_eotf(x) * PQ_PEAK_NITS / REFERENCE_WHITE_NITS;
            assert!((lerp(&c.pq, x) - pq).abs() <= pq * 0.01 + 1e-4, "pq {x}");
            let out = srgb(x / (1.0 + x / KNEE) * GAIN);
            assert!((lerp(&c.out, x) - out).abs() * 255.0 < 0.5, "out {x}");
        }
    }

    #[test]
    fn chroma_moves_the_matching_channel() {
        let c = Curves::new();
        let y = pq(REFERENCE_WHITE_NITS / 4.0);
        let [r, g, b] = c.tone_map(y, 0.0, 0.1);
        assert!(r > g && r > b, "Cr toward red: {r} {g} {b}");
        let [r, g, b] = c.tone_map(y, 0.1, 0.0);
        assert!(b > r && b > g, "Cb toward blue: {r} {g} {b}");
    }

    /// Set `HANDPIC_HEVC_CR3` to a CR3 shot with HDR PQ on, then run with
    /// `--ignored`.
    #[test]
    #[ignore]
    fn decodes_the_preview_of_a_real_file() {
        let path = std::env::var("HANDPIC_HEVC_CR3").expect("HANDPIC_HEVC_CR3");
        let buf = std::fs::read(path).unwrap();
        let a = crate::cr3::parse(&buf).unwrap();
        let e = a.preview.unwrap();
        assert_eq!(e.codec, crate::arw::Codec::Hevc);
        let jpeg = to_jpeg(a.slice(&buf, e)).unwrap();
        let (rgb, w, h) = crate::decode::decode_rgb(&jpeg).unwrap();
        assert_eq!((w, h), (1620, 1080));
        assert!(rgb.chunks(3).any(|p| p != &rgb[..3]), "one color");
    }
}
