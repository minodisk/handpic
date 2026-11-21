//! The DCT scale the fit view's preview is decoded at: the smallest `n/8`
//! whose decoded size is not below the source's fit in the preview canvas, in
//! device pixels, so GPUI uploads about the drawn size rather than the whole
//! preview. It names no GPUI type.

/// `size` turned by a quarter turn when `orientation` is one (6 or 8, the
/// turns `orient::rotate_bgra` makes): the upright size of a stored one, and
/// back.
pub fn turned((width, height): (u32, u32), orientation: u16) -> (u32, u32) {
    if matches!(orientation, 6 | 8) {
        (height, width)
    } else {
        (width, height)
    }
}

/// One edge of `native` decoded at `n/8`, `ceil(native * n / 8)` as mozjpeg
/// sizes it.
pub fn scaled(native: u32, n: u8) -> u32 {
    (native * u32::from(n)).div_ceil(8)
}

/// Whether an image decoded at `decoded` (upright) serves the fit of a source
/// of upright size `upright` in `canvas`: not below the fit on either axis,
/// or the source's own size when the fit is larger. An unknown canvas
/// (`(0, 0)`, nothing painted yet) is served by anything.
pub fn serves(decoded: (u32, u32), upright: (u32, u32), canvas: (u32, u32)) -> bool {
    if canvas.0 == 0 || canvas.1 == 0 || upright.0 == 0 || upright.1 == 0 {
        return true;
    }
    let ratio = (f64::from(canvas.0) / f64::from(upright.0))
        .min(f64::from(canvas.1) / f64::from(upright.1));
    let fit = |edge: u32| ((f64::from(edge) * ratio).round() as u32).min(edge);
    decoded.0 >= fit(upright.0) && decoded.1 >= fit(upright.1)
}

/// The smallest `n/8` (1 to 8) at which a JPEG stored at `stored` and shown
/// turned by `orientation` serves `canvas`; 8 when none below does.
pub fn scale_for(stored: (u32, u32), orientation: u16, canvas: (u32, u32)) -> u8 {
    if canvas.0 == 0 || canvas.1 == 0 {
        return 8;
    }
    let upright = turned(stored, orientation);
    (1..=8u8)
        .find(|&n| {
            serves(
                (scaled(upright.0, n), scaled(upright.1, n)),
                upright,
                canvas,
            )
        })
        .unwrap_or(8)
}

#[cfg(test)]
mod tests {
    use super::{scale_for, scaled, serves, turned};

    const RAF: (u32, u32) = (4416, 2944);
    const ARW: (u32, u32) = (1616, 1080);

    #[test]
    fn a_raf_preview_in_a_1600_by_1000_window_takes_three_eighths() {
        assert_eq!(scale_for(RAF, 1, (1160, 770)), 3);
        assert_eq!((scaled(RAF.0, 3), scaled(RAF.1, 3)), (1656, 1104));
    }

    #[test]
    fn the_scale_is_the_smallest_eighth_not_below_the_fit() {
        assert_eq!(scale_for(RAF, 1, (1104, 736)), 2);
        assert_eq!(scale_for(RAF, 1, (1105, 737)), 3);
        assert_eq!(scale_for(RAF, 1, (552, 368)), 1);
        assert_eq!(scale_for(ARW, 1, (1160, 770)), 6);
        assert_eq!(scale_for(ARW, 1, (1616, 1080)), 8);
    }

    #[test]
    fn a_source_smaller_than_the_fit_or_an_unknown_canvas_is_decoded_whole() {
        assert_eq!(scale_for(ARW, 1, (3000, 2000)), 8);
        assert_eq!(scale_for(RAF, 1, (0, 0)), 8);
        assert_eq!(scale_for(RAF, 1, (0, 700)), 8);
    }

    #[test]
    fn a_quarter_turn_fits_the_upright_size() {
        assert_eq!(turned(RAF, 6), (2944, 4416));
        assert_eq!(turned(RAF, 8), (2944, 4416));
        assert_eq!(turned(RAF, 3), RAF);
        // Upright 2944x4416 in 1160x770: the fit is about 513x770.
        assert_eq!(scale_for(RAF, 6, (1160, 770)), 2);
        assert_eq!(scale_for(RAF, 1, (1160, 770)), 3);
    }

    #[test]
    fn mozjpeg_rounds_a_scaled_edge_up() {
        assert_eq!(scaled(1616, 8), 1616);
        assert_eq!(scaled(1617, 1), 203);
        assert_eq!(scaled(1080, 3), 405);
        assert_eq!(scaled(1, 1), 1);
    }

    #[test]
    fn an_image_serves_a_canvas_its_fit_does_not_outgrow() {
        let decoded = (1656, 1104);
        assert!(serves(decoded, RAF, (1160, 770)));
        assert!(serves(decoded, RAF, (1656, 1200)));
        assert!(!serves(decoded, RAF, (1700, 1200)));
        assert!(!serves((1104, 736), RAF, (1160, 770)));
        assert!(serves((1104, 736), RAF, (0, 0)));
        assert!(serves(RAF, RAF, (8000, 6000)));
        assert!(!serves((2208, 1472), RAF, (8000, 6000)));
    }
}
