//! Partially decode only the area around the focus point out of JpgFromRaw.
//! Calls libjpeg-turbo's jpeg_crop_scanline + jpeg_skip_scanlines directly.

use anyhow::{anyhow, bail, Result};
use mozjpeg_sys as sys;
use std::mem;

use crate::arw::FocusLocation;

pub struct Crop {
    /// RGB, or RGBA when the crop was decoded for the app.
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
    /// Top-left actually produced. Cropping snaps to MCU boundaries, so it shifts.
    pub x: usize,
    pub y: usize,
    /// The whole decoded JPEG's size, which the crop was cut out of.
    pub image_width: usize,
    pub image_height: usize,
}

/// A crop centered on the focus point, together with that point's position
/// inside the crop: the MCU snap moves the crop's origin, so its center is not
/// the point of interest.
pub struct FocusCrop {
    pub crop: Crop,
    /// The point of interest in crop pixel coordinates.
    pub point_x: usize,
    pub point_y: usize,
}

/// The point of interest on the unrotated JPEG: the focus point scaled from
/// sensor coordinates, or the image center when the file has none.
pub fn focus_point(w: usize, h: usize, focus: Option<FocusLocation>) -> (usize, usize) {
    match focus {
        Some(f) if f.sensor_w > 0 && f.sensor_h > 0 => {
            let sx = w as f64 / f.sensor_w as f64;
            let sy = h as f64 / f.sensor_h as f64;
            (
                ((f.x as f64 * sx) as usize).min(w.saturating_sub(1)),
                ((f.y as f64 * sy) as usize).min(h.saturating_sub(1)),
            )
        }
        _ => (w / 2, h / 2),
    }
}

/// Cut out `width` x `height` RGBA centered on the focus point, mapped onto
/// this JPEG's own size.
pub fn decode_focus_crop(
    jpeg: &[u8],
    focus: Option<FocusLocation>,
    width: usize,
    height: usize,
) -> Result<FocusCrop> {
    let mut point = (0usize, 0usize);
    let crop = decode_region(jpeg, sys::J_COLOR_SPACE::JCS_EXT_RGBA, 4, |iw, ih| {
        point = focus_point(iw, ih, focus);
        (point.0, point.1, width, height)
    })?;
    Ok(FocusCrop {
        point_x: point.0 - crop.x,
        point_y: point.1 - crop.y,
        crop,
    })
}

/// Cut out size x size RGB centered on (cx, cy). The region is clamped to the image.
pub fn decode_crop(jpeg: &[u8], cx: usize, cy: usize, size: usize) -> Result<Crop> {
    decode_region(jpeg, sys::J_COLOR_SPACE::JCS_RGB, 3, |_, _| {
        (cx, cy, size, size)
    })
}

/// The width of `jpeg` from its header alone.
pub fn jpeg_width(jpeg: &[u8]) -> Result<usize> {
    guarded(|| unsafe {
        let mut err: sys::jpeg_error_mgr = mem::zeroed();
        let mut cinfo: sys::jpeg_decompress_struct = mem::zeroed();
        cinfo.common.err = sys::jpeg_std_error(&mut err);
        (*cinfo.common.err).error_exit = Some(unwind_error_exit);
        sys::jpeg_create_decompress(&mut cinfo);
        let _guard = scopeguard(&mut cinfo as *mut _);
        sys::jpeg_mem_src(&mut cinfo, jpeg.as_ptr(), jpeg.len() as _);
        if sys::jpeg_read_header(&mut cinfo, true as _) != 1 {
            bail!("jpeg_read_header failed");
        }
        Ok(cinfo.image_width as usize)
    })
}

/// The shared partial decode. `region` is handed the JPEG's size and returns
/// the wanted center and size; the result is clamped to the image and snapped
/// to MCU boundaries horizontally. A fatal libjpeg error comes back as `Err`.
fn decode_region(
    jpeg: &[u8],
    color: sys::J_COLOR_SPACE,
    channels: usize,
    region: impl FnOnce(usize, usize) -> (usize, usize, usize, usize),
) -> Result<Crop> {
    guarded(|| decode_region_unguarded(jpeg, color, channels, region))
}

/// `f`, its libjpeg error unwind turned into an `Err`.
fn guarded<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|e| {
        match e.downcast::<String>() {
            Ok(msg) => anyhow!(*msg),
            Err(_) => anyhow!("panic while partially decoding the JPEG"),
        }
    })?
}

fn decode_region_unguarded(
    jpeg: &[u8],
    color: sys::J_COLOR_SPACE,
    channels: usize,
    region: impl FnOnce(usize, usize) -> (usize, usize, usize, usize),
) -> Result<Crop> {
    unsafe {
        let mut err: sys::jpeg_error_mgr = mem::zeroed();
        let mut cinfo: sys::jpeg_decompress_struct = mem::zeroed();
        cinfo.common.err = sys::jpeg_std_error(&mut err);
        // The default error_exit calls exit(1); unwind instead so the guard
        // above turns it into an Err.
        (*cinfo.common.err).error_exit = Some(unwind_error_exit);
        sys::jpeg_create_decompress(&mut cinfo);

        let guard = scopeguard(&mut cinfo as *mut _);

        sys::jpeg_mem_src(&mut cinfo, jpeg.as_ptr(), jpeg.len() as _);
        if sys::jpeg_read_header(&mut cinfo, true as _) != 1 {
            bail!("jpeg_read_header failed");
        }
        cinfo.out_color_space = color;
        sys::jpeg_start_decompress(&mut cinfo);

        let (iw, ih) = (cinfo.output_width as usize, cinfo.output_height as usize);
        let (cx, cy, cw, ch) = region(iw, ih);
        let w = cw.min(iw);
        let h = ch.min(ih);
        let x0 = cx.saturating_sub(w / 2).min(iw - w);
        let y0 = cy.saturating_sub(h / 2).min(ih - h);

        // Horizontal: snapped to MCU boundaries, so trust the values it hands back.
        let mut xoff = x0 as sys::JDIMENSION;
        let mut width = w as sys::JDIMENSION;
        sys::jpeg_crop_scanline(&mut cinfo, &mut xoff, &mut width);

        // Vertical: skip ahead to the first row we need.
        if y0 > 0 {
            sys::jpeg_skip_scanlines(&mut cinfo, y0 as _);
        }

        let stride = width as usize * channels;
        let mut pixels = vec![0u8; stride * h];
        let mut read = 0usize;
        while read < h {
            let mut row = pixels.as_mut_ptr().add(read * stride);
            let n = sys::jpeg_read_scanlines(&mut cinfo, &mut row, 1);
            if n == 0 {
                break;
            }
            read += n as usize;
        }
        pixels.truncate(stride * read);

        // Abort without reading the rest; otherwise it expands the whole image.
        sys::jpeg_abort_decompress(&mut cinfo);
        drop(guard);

        Ok(Crop {
            pixels,
            width: width as usize,
            height: read,
            x: xoff as usize,
            y: y0,
            image_width: iw,
            image_height: ih,
        })
    }
}

/// Unwinds with libjpeg's message, skipping the panic hook so nothing is
/// printed for an expected bad file.
#[cold]
extern "C-unwind" fn unwind_error_exit(cinfo: &mut sys::jpeg_common_struct) {
    let msg = unsafe {
        let err = &*cinfo.err;
        match err.format_message {
            Some(fmt) => {
                // mozjpeg-sys declares the buffer `&[u8; 80]`, but libjpeg writes into it.
                let fmt = mem::transmute::<
                    unsafe extern "C-unwind" fn(&mut sys::jpeg_common_struct, &[u8; 80]),
                    unsafe extern "C-unwind" fn(&mut sys::jpeg_common_struct, &mut [u8; 80]),
                >(fmt);
                let mut buf = [0u8; 80];
                fmt(cinfo, &mut buf);
                let text = buf.split(|&c| c == 0).next().unwrap_or_default();
                format!("libjpeg fatal error: {}", String::from_utf8_lossy(text))
            }
            None => format!("libjpeg fatal error: code {}", err.msg_code),
        }
    };
    std::panic::resume_unwind(Box::new(msg));
}

/// Makes sure jpeg_destroy_decompress always runs.
struct ScopeGuard(*mut sys::jpeg_decompress_struct);
impl Drop for ScopeGuard {
    fn drop(&mut self) {
        unsafe { sys::jpeg_destroy_decompress(&mut *self.0) }
    }
}
fn scopeguard(p: *mut sys::jpeg_decompress_struct) -> ScopeGuard {
    ScopeGuard(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_rgb;

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

    fn focus(sensor_w: u16, sensor_h: u16, x: u16, y: u16) -> FocusLocation {
        FocusLocation {
            sensor_w,
            sensor_h,
            x,
            y,
        }
    }

    #[test]
    fn the_crop_matches_the_same_region_of_a_full_decode() {
        let jpeg = jpeg(640, 480);
        let (full, fw, _) = decode_rgb(&jpeg).unwrap();
        let c = decode_focus_crop(&jpeg, Some(focus(640, 480, 300, 200)), 128, 64).unwrap();
        let crop = &c.crop;

        assert_eq!(crop.x % 16, 0);
        assert_eq!((c.point_x, c.point_y), (300 - crop.x, 200 - crop.y));
        assert_eq!(crop.pixels.len(), crop.width * crop.height * 4);
        for y in 0..crop.height {
            for x in 0..crop.width {
                let a = &crop.pixels[(y * crop.width + x) * 4..][..3];
                let b = &full[((crop.y + y) * fw + crop.x + x) * 3..][..3];
                // Chroma upsampling at the crop's side edges has one fewer
                // neighbor than in the full decode, so only the interior is
                // exact.
                if (8..crop.width - 8).contains(&x) {
                    assert_eq!(a, b, "at ({x},{y})");
                } else {
                    for (a, b) in a.iter().zip(b) {
                        assert!(a.abs_diff(*b) <= 4, "at ({x},{y}): {a} vs {b}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_crop_at_the_edge_is_clamped_into_the_image() {
        let jpeg = jpeg(320, 240);
        let c = decode_focus_crop(&jpeg, Some(focus(320, 240, 310, 5)), 128, 64).unwrap();
        assert_eq!(c.crop.y, 0);
        assert_eq!(c.crop.x + c.crop.width, 320);
        assert_eq!(c.crop.height, 64);
    }

    #[test]
    fn without_a_focus_location_the_crop_is_centered() {
        assert_eq!(focus_point(640, 480, None), (320, 240));
        let c = decode_focus_crop(&jpeg(640, 480), None, 64, 64).unwrap();
        assert_eq!(c.crop.y, 240 - 32);
    }

    #[test]
    fn the_focus_point_is_scaled_from_sensor_to_jpeg_coordinates() {
        assert_eq!(
            focus_point(3504, 2336, Some(focus(7008, 4672, 3613, 1732))),
            (1806, 866)
        );
        assert_eq!(
            focus_point(7008, 4672, Some(focus(7008, 4672, 3613, 1732))),
            (3613, 1732)
        );
    }

    #[test]
    fn a_malformed_jpeg_is_an_error() {
        let jpeg = jpeg(320, 240);
        // Cut inside the header: a cut inside the scan data is only a libjpeg
        // warning ("Premature end of JPEG file") and still yields a crop.
        let inputs: [&[u8]; 3] = [b"", b"not a jpeg", &jpeg[..64]];
        for input in inputs {
            assert!(decode_focus_crop(input, None, 64, 64).is_err());
            assert!(decode_crop(input, 160, 120, 64).is_err());
        }
        let e = decode_crop(b"not a jpeg", 0, 0, 64).err().unwrap();
        assert!(
            e.to_string()
                .starts_with("libjpeg fatal error: Not a JPEG file"),
            "{e}"
        );
    }

    #[test]
    fn decode_crop_still_returns_rgb() {
        let c = decode_crop(&jpeg(320, 240), 160, 120, 64).unwrap();
        assert_eq!(c.pixels.len(), c.width * c.height * 3);
    }
}
