//! The MCP `screenshot` tool's capture, built only with the `screenshot`
//! feature: the window's last drawn frame (`Window::render_to_image`, a
//! gpui `test-support` API) scaled to a long edge and encoded as a PNG.

#[cfg(feature = "screenshot")]
use gpui_kit::Window;
use image::{DynamicImage, ImageEncoder, RgbaImage, codecs::png::PngEncoder, imageops};

/// The size `width` x `height` scaled so its long edge is at most
/// `long_edge`, never scaled up and never below 1 px.
fn scaled_size(width: u32, height: u32, long_edge: u32) -> (u32, u32) {
    let long = width.max(height);
    if long <= long_edge {
        return (width, height);
    }
    let scale = |side: u32| {
        ((u64::from(side) * u64::from(long_edge) + u64::from(long) / 2) / u64::from(long)).max(1)
            as u32
    };
    (scale(width), scale(height))
}

/// `image` scaled to `long_edge` and encoded as a PNG, with its size.
pub fn encode(image: RgbaImage, long_edge: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let (width, height) = scaled_size(image.width(), image.height(), long_edge);
    // Through `DynamicImage`, so the resize is the `image` crate's own
    // (optimized in a debug build) rather than instantiated here.
    let image = if (width, height) == image.dimensions() {
        image
    } else {
        DynamicImage::ImageRgba8(image)
            .resize_exact(width, height, imageops::FilterType::Triangle)
            .into_rgba8()
    };
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("could not encode the screenshot: {e}"))?;
    Ok((png, width, height))
}

/// The window's last drawn frame, in device pixels.
#[cfg(feature = "screenshot")]
pub fn capture(window: &Window) -> Result<RgbaImage, String> {
    window
        .render_to_image()
        .map_err(|e| format!("could not capture the window: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_landscape_frame_scales_by_its_width() {
        assert_eq!(scaled_size(3000, 2000, 1500), (1500, 1000));
    }

    #[test]
    fn a_portrait_frame_scales_by_its_height() {
        assert_eq!(scaled_size(1000, 4000, 1000), (250, 1000));
    }

    #[test]
    fn a_smaller_frame_is_not_scaled_up() {
        assert_eq!(scaled_size(800, 600, 1568), (800, 600));
        assert_eq!(scaled_size(1568, 1000, 1568), (1568, 1000));
    }

    #[test]
    fn a_thin_frame_keeps_at_least_one_pixel() {
        assert_eq!(scaled_size(4000, 1, 256), (256, 1));
        assert_eq!(scaled_size(1, 1, 256), (1, 1));
    }

    #[test]
    fn an_encoded_frame_decodes_back_at_its_scaled_size() {
        let image = RgbaImage::from_pixel(40, 20, image::Rgba([10, 20, 30, 255]));
        let (png, width, height) = encode(image, 10).unwrap();
        assert_eq!((width, height), (10, 5));
        let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (10, 5));
        assert_eq!(decoded.to_rgba8().get_pixel(3, 2).0, [10, 20, 30, 255]);
    }
}
