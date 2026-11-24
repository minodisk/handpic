//! Decode an embedded JPEG to RGB and rotate it for display.

use anyhow::Result;

pub fn decode_rgb(jpeg: &[u8]) -> Result<(Vec<u8>, usize, usize)> {
    // mozjpeg reports malformed input by panicking, not by returning an error.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decode_rgb_unguarded(jpeg)))
        .map_err(|_| anyhow::anyhow!("panic while decoding the JPEG"))?
}

fn decode_rgb_unguarded(jpeg: &[u8]) -> Result<(Vec<u8>, usize, usize)> {
    let d = mozjpeg::Decompress::new_mem(jpeg)?.rgb()?;
    let (w, h) = (d.width(), d.height());
    let mut d = d;
    let pixels: Vec<[u8; 3]> = d.read_scanlines()?;
    d.finish()?;
    let flat = pixels.into_iter().flatten().collect();
    Ok((flat, w, h))
}

/// An image decoded at a DCT scale and rotated upright.
#[derive(Debug, Clone, PartialEq)]
pub struct Upright {
    pub rgb: Vec<u8>,
    /// The upright size of `rgb`.
    pub width: usize,
    pub height: usize,
    /// The stored (unrotated, unscaled) size of the JPEG it came from.
    pub stored: (usize, usize),
}

/// Decode a JPEG at the smallest `n/8` scale whose long edge is not below
/// `long_edge` (8/8 when the source is smaller), the rule of
/// `thumbnail_jpeg_near`, and rotate it upright per `orientation`.
pub fn decode_upright_near(jpeg: &[u8], orientation: u16, long_edge: usize) -> Result<Upright> {
    // mozjpeg reports malformed input by panicking, not by returning an error.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        decode_upright_near_unguarded(jpeg, orientation, long_edge)
    }))
    .map_err(|_| anyhow::anyhow!("panic while decoding the JPEG"))?
}

fn decode_upright_near_unguarded(
    jpeg: &[u8],
    orientation: u16,
    long_edge: usize,
) -> Result<Upright> {
    let mut d = mozjpeg::Decompress::new_mem(jpeg)?;
    let stored = (d.width(), d.height());
    d.scale(scale_near(stored.0.max(stored.1), long_edge));
    let mut d = d.rgb()?;
    let (w, h) = (d.width(), d.height());
    let pixels: Vec<[u8; 3]> = d.read_scanlines()?;
    d.finish()?;
    let rgb: Vec<u8> = pixels.into_iter().flatten().collect();
    let (rgb, width, height) = crate::faces::upright_rgb(&rgb, w, h, orientation);
    Ok(Upright {
        rgb,
        width,
        height,
        stored,
    })
}

/// The smallest `n/8` scale whose long edge, `ceil(native * n / 8)` as
/// mozjpeg sizes it, is not below `long_edge`; 8 when none is.
fn scale_near(native: usize, long_edge: usize) -> u8 {
    (1..=8u8)
        .find(|&n| (native * usize::from(n)).div_ceil(8) >= long_edge)
        .unwrap_or(8)
}

/// Rotate an RGB buffer per the Orientation tag. Only 1/6/8 show up in ARW files.
pub fn apply_orientation(
    rgb: &[u8],
    w: usize,
    h: usize,
    orientation: u16,
) -> (Vec<u8>, usize, usize) {
    let px = |x: usize, y: usize| {
        let i = (y * w + x) * 3;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    };
    match orientation {
        6 | 8 => {
            let (nw, nh) = (h, w);
            let mut out = vec![0u8; nw * nh * 3];
            for y in 0..nh {
                for x in 0..nw {
                    // 6 = Rotate 90 CW, 8 = Rotate 270 CW
                    let (sx, sy) = if orientation == 6 {
                        (y, nw - 1 - x)
                    } else {
                        (nh - 1 - y, x)
                    };
                    let p = px(sx, sy);
                    let i = (y * nw + x) * 3;
                    out[i..i + 3].copy_from_slice(&p);
                }
            }
            (out, nw, nh)
        }
        _ => (rgb.to_vec(), w, h),
    }
}

/// Thumbnail a JPEG of any size to a long edge of at most `long_edge`: decode
/// it at the smallest `n/8` scale whose long edge is not below `long_edge`
/// (8/8 when the source is smaller), box-average that down to `long_edge`, and
/// re-encode it as a baseline JPEG for the thumbnail cache. A 1616x1080
/// source takes 2/8 (404x270) and no resampling.
///
/// The output keeps the source's orientation, i.e. it is **unrotated**: the
/// caller carries the Orientation alongside it, as the preview tier does.
pub fn thumbnail_jpeg_near(jpeg: &[u8], long_edge: usize, quality: f32) -> Result<Vec<u8>> {
    let mut d = mozjpeg::Decompress::new_mem(jpeg)?;
    d.scale(scale_near(d.width().max(d.height()), long_edge));
    let mut d = d.rgb()?;
    let (w, h) = (d.width(), d.height());
    let pixels: Vec<[u8; 3]> = d.read_scanlines()?;
    d.finish()?;
    let rgb: Vec<u8> = pixels.into_iter().flatten().collect();
    let (rgb, w, h) = if w.max(h) > long_edge {
        box_down(&rgb, w, h, long_edge)
    } else {
        (rgb, w, h)
    };

    let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    c.set_size(w, h);
    c.set_quality(quality);
    // Baseline, not progressive: the strip decodes these one by one on the UI
    // thread, and mozjpeg's scan optimization costs more time than the few
    // kilobytes it saves at this size.
    c.set_optimize_scans(false);
    let mut c = c.start_compress(Vec::new())?;
    c.write_scanlines(&rgb)?;
    Ok(c.finish()?)
}

/// Box-average an RGB buffer down so its long edge is `long_edge`.
fn box_down(rgb: &[u8], w: usize, h: usize, long_edge: usize) -> (Vec<u8>, usize, usize) {
    let (ow, oh) = if w >= h {
        (long_edge, (h * long_edge).div_ceil(w).max(1))
    } else {
        ((w * long_edge).div_ceil(h).max(1), long_edge)
    };
    let mut out = Vec::with_capacity(ow * oh * 3);
    for oy in 0..oh {
        let y0 = oy * h / oh;
        let y1 = ((oy + 1) * h / oh).max(y0 + 1);
        for ox in 0..ow {
            let x0 = ox * w / ow;
            let x1 = ((ox + 1) * w / ow).max(x0 + 1);
            let mut sum = [0u32; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * w + x) * 3;
                    sum[0] += u32::from(rgb[i]);
                    sum[1] += u32::from(rgb[i + 1]);
                    sum[2] += u32::from(rgb[i + 2]);
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            out.extend(sum.map(|v| ((v + n / 2) / n) as u8));
        }
    }
    (out, ow, oh)
}

/// Decode a preview JPEG at the largest `n/8` scale whose long edge is at
/// most `long_edge` (1/8 when none is), rotate it upright per `orientation`,
/// and re-encode it. Returns the JPEG with its upright width and height.
pub fn preview_jpeg(
    preview: &[u8],
    orientation: u16,
    long_edge: usize,
    quality: f32,
) -> Result<(Vec<u8>, usize, usize)> {
    // mozjpeg reports malformed input by panicking, not by returning an error.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        preview_jpeg_unguarded(preview, orientation, long_edge, quality)
    }))
    .map_err(|_| anyhow::anyhow!("panic while scaling the JPEG"))?
}

fn preview_jpeg_unguarded(
    preview: &[u8],
    orientation: u16,
    long_edge: usize,
    quality: f32,
) -> Result<(Vec<u8>, usize, usize)> {
    let mut d = mozjpeg::Decompress::new_mem(preview)?;
    let native = d.width().max(d.height());
    let scale = (1..=8u8)
        .rev()
        .find(|&n| (native * usize::from(n)).div_ceil(8) <= long_edge)
        .unwrap_or(1);
    d.scale(scale);
    let mut d = d.rgb()?;
    let (w, h) = (d.width(), d.height());
    let pixels: Vec<[u8; 3]> = d.read_scanlines()?;
    d.finish()?;
    let rgb: Vec<u8> = pixels.into_iter().flatten().collect();
    let (rgb, w, h) = crate::faces::upright_rgb(&rgb, w, h, orientation);

    let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    c.set_size(w, h);
    c.set_quality(quality);
    let mut c = c.start_compress(Vec::new())?;
    c.write_scanlines(&rgb)?;
    Ok((c.finish()?, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a synthetic gradient so the test needs no image file.
    fn jpeg(w: usize, h: usize) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(w * h * 3);
        for y in 0..h {
            for x in 0..w {
                rgb.extend_from_slice(&[(x * 255 / w) as u8, (y * 255 / h) as u8, 128]);
            }
        }
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        c.set_size(w, h);
        c.set_quality(90.0);
        let mut c = c.start_compress(Vec::new()).unwrap();
        c.write_scanlines(&rgb).unwrap();
        c.finish().unwrap()
    }

    #[test]
    fn a_jpeg_thumbnail_is_scaled_down_to_the_long_edge() {
        for ((w, h), want) in [
            ((1616, 1080), (404, 270)),
            ((4000, 3000), (404, 303)),
            ((3000, 4000), (303, 404)),
            ((200, 100), (200, 100)),
        ] {
            let out = thumbnail_jpeg_near(&jpeg(w, h), 404, 80.0).unwrap();
            let (_, dw, dh) = decode_rgb(&out).unwrap();
            assert_eq!((dw, dh), want, "{w}x{h}");
        }
    }

    #[test]
    fn malformed_jpeg_is_an_error() {
        assert!(decode_rgb(b"not a jpeg").is_err());
        let truncated = jpeg(64, 64);
        assert!(decode_rgb(&truncated[..truncated.len() / 2]).is_err());
    }

    #[test]
    fn preview_takes_the_largest_eighth_within_the_long_edge() {
        let (out, w, h) = preview_jpeg(&jpeg(1616, 1080), 1, 1024, 75.0).unwrap();
        assert_eq!((w, h), (1010, 675));
        let (_, dw, dh) = decode_rgb(&out).unwrap();
        assert_eq!((dw, dh), (1010, 675));
    }

    #[test]
    fn preview_is_rotated_upright() {
        let (out, w, h) = preview_jpeg(&jpeg(1616, 1080), 6, 1616, 75.0).unwrap();
        assert_eq!((w, h), (1080, 1616));
        let (rgb, dw, dh) = decode_rgb(&out).unwrap();
        assert_eq!((dw, dh), (1080, 1616));
        // The source's left edge (red 0) is the top after a 90 degree CW turn.
        let top = rgb[(dw / 2) * 3];
        let bottom = rgb[((dh - 1) * dw + dw / 2) * 3];
        assert!(top < 32 && bottom > 224, "top {top}, bottom {bottom}");
    }

    #[test]
    fn preview_is_rotated_upright_for_a_half_turn() {
        let (out, w, h) = preview_jpeg(&jpeg(1616, 1080), 3, 1616, 75.0).unwrap();
        assert_eq!((w, h), (1616, 1080));
        let (rgb, dw, dh) = decode_rgb(&out).unwrap();
        assert_eq!((dw, dh), (1616, 1080));
        // The source's top edge (green 0) ends up at the bottom after a half turn.
        let top = rgb[(dw / 2) * 3 + 1];
        let bottom = rgb[((dh - 1) * dw + dw / 2) * 3 + 1];
        assert!(top > 224 && bottom < 32, "top {top}, bottom {bottom}");
    }

    #[test]
    fn preview_falls_back_to_an_eighth_below_it() {
        let (_, w, h) = preview_jpeg(&jpeg(1616, 1080), 1, 100, 75.0).unwrap();
        assert_eq!((w, h), (202, 135));
    }

    #[test]
    fn the_near_scale_is_the_smallest_eighth_not_below_the_long_edge() {
        assert_eq!(scale_near(2112, 640), 3);
        assert_eq!(scale_near(1616, 640), 4);
        assert_eq!(scale_near(6000, 640), 1);
        assert_eq!(scale_near(1704, 640), 4);
        assert_eq!(scale_near(1709, 640), 3);
        assert_eq!(scale_near(500, 640), 8);
    }

    #[test]
    fn a_near_decode_has_the_scaled_size_and_the_stored_one() {
        for ((w, h), orientation, want) in [
            ((2112, 1408), 1, (792, 528)),
            ((2100, 1401), 1, (788, 526)),
            ((1616, 1080), 6, (540, 808)),
            ((1616, 1080), 8, (540, 808)),
            ((1616, 1080), 3, (808, 540)),
            ((400, 300), 1, (400, 300)),
        ] {
            let d = decode_upright_near(&jpeg(w, h), orientation, 640).unwrap();
            assert_eq!((d.width, d.height), want, "{w}x{h} {orientation}");
            assert_eq!(d.stored, (w, h));
            assert_eq!(d.rgb.len(), d.width * d.height * 3);
        }
    }

    #[test]
    fn a_near_decode_is_rotated_upright() {
        // Red rises left to right and green top to bottom in the source.
        let at = |d: &Upright, x: usize, y: usize| {
            let i = (y * d.width + x) * 3;
            (d.rgb[i], d.rgb[i + 1])
        };
        let src = jpeg(1616, 1080);
        let d = decode_upright_near(&src, 1, 640).unwrap();
        let (r, g) = at(&d, d.width - 1, d.height / 2);
        assert!(r > 224 && (96..160).contains(&g), "1: ({r}, {g})");
        // 6: the source's left edge (red 0) is the top, its top (green 0)
        // the right.
        let d = decode_upright_near(&src, 6, 640).unwrap();
        let (r, _) = at(&d, d.width / 2, 0);
        let (_, g) = at(&d, d.width - 1, d.height / 2);
        assert!(r < 32 && g < 32, "6: red {r}, green {g}");
        // 8: the source's left edge is the bottom, its top the left.
        let d = decode_upright_near(&src, 8, 640).unwrap();
        let (r, _) = at(&d, d.width / 2, d.height - 1);
        let (_, g) = at(&d, 0, d.height / 2);
        assert!(r < 32 && g < 32, "8: red {r}, green {g}");
        // 3: the source's top-left corner is the bottom-right.
        let d = decode_upright_near(&src, 3, 640).unwrap();
        let (r, g) = at(&d, d.width - 1, d.height - 1);
        assert!(r < 32 && g < 32, "3: ({r}, {g})");
    }

    #[test]
    fn a_malformed_jpeg_near_decode_is_an_error() {
        assert!(decode_upright_near(b"not a jpeg", 1, 640).is_err());
    }

    #[test]
    fn a_malformed_preview_is_an_error() {
        assert!(preview_jpeg(b"not a jpeg", 1, 1024, 75.0).is_err());
    }
}
