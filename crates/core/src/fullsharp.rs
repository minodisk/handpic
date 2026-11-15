//! The eye sharpness of a judged face measured once more on the file's
//! full-size embedded JPEG, where softness the preview's downscale hides
//! still shows.
//!
//! Each eye is cropped to a square of half-side `CROP_HALF_SIDE` eye distances
//! around its center, brought to `RESIZED` px square so the score does not
//! depend on the eye's size, and scored as the 99th percentile of the gradient
//! magnitude over the contrast of the crop. The frame's score is the sharper
//! eye's. A port of the prototype in the full-size-eye-check plan
//! (`full_all.py`'s `met`): numpy's `gradient` and `percentile`, PIL's bicubic.

use crate::candidate::luma;
use crate::embed::{resample, Filter};
use crate::partial::{decode_crop, jpeg_width};

/// The frame score below which the eyes are soft at full size. Chosen by
/// blind review of Good frames sorted by the score on three folders of one
/// body; see the full-size-eye-check plan.
pub const SOFT_BELOW: f64 = 0.162;
/// The full width over the preview width at or under which the full JPEG has
/// no detail worth measuring.
const MIN_SCALE: f64 = 1.5;
/// The floor of the distance between the eyes, in preview px.
const MIN_EYE_DISTANCE: f64 = 8.0;
/// The half-side of an eye's crop, in eye distances.
const CROP_HALF_SIDE: f64 = 0.6;
/// The side of the square an eye's crop is resized to.
const RESIZED: usize = 154;
/// The extra side asked of the partial decode, so that rounding never leaves
/// the exact square short of pixels.
const CROP_MARGIN: usize = 4;

/// The metric of one already-cut `width` x `height` luma crop: the 99th
/// percentile of the gradient magnitude of the crop resized to `RESIZED`
/// square, over the larger of its p98 - p2 range and 1. Higher is sharper.
pub fn eye_crop_score(gray: &[u8], width: usize, height: usize) -> f64 {
    let resized = resample(gray, width, height, RESIZED, RESIZED, Filter::Bicubic, 1);
    let mut values: Vec<f64> = resized.iter().map(|&v| f64::from(v)).collect();
    let mut magnitudes = gradient_magnitude(&values, RESIZED, RESIZED);
    values.sort_unstable_by(f64::total_cmp);
    magnitudes.sort_unstable_by(f64::total_cmp);
    let contrast = percentile(&values, 98.0) - percentile(&values, 2.0);
    percentile(&magnitudes, 99.0) / contrast.max(1.0)
}

/// The frame score of `full_jpeg` for the two `eyes`, given in the stored
/// coordinates of a preview `preview_width` px wide: the higher of the two
/// eyes' `eye_crop_score`. `None` when the full JPEG is not more than
/// `MIN_SCALE` times the preview's width, or a crop cannot be decoded or lies
/// wholly outside the image.
pub fn full_eye_sharpness(
    full_jpeg: &[u8],
    eyes: [(f32, f32); 2],
    preview_width: usize,
) -> Option<f64> {
    if preview_width == 0 {
        return None;
    }
    let scale = jpeg_width(full_jpeg).ok()? as f64 / preview_width as f64;
    if scale <= MIN_SCALE {
        return None;
    }
    let [(lx, ly), (rx, ry)] = eyes.map(|(x, y)| (f64::from(x), f64::from(y)));
    let half = CROP_HALF_SIDE * (lx - rx).hypot(ly - ry).max(MIN_EYE_DISTANCE) * scale;
    let score = |x: f64, y: f64| eye_score(full_jpeg, (x * scale, y * scale), half);
    Some(score(lx, ly)?.max(score(rx, ry)?))
}

/// `eye_crop_score` of the square of half-side `half` around `center`, in
/// full-size px, as far as it lies inside the image. The square's corners
/// truncate as PIL's `crop` box did in the prototype.
fn eye_score(full_jpeg: &[u8], center: (f64, f64), half: f64) -> Option<f64> {
    let (cx, cy) = center;
    let side = (2.0 * half).ceil() as usize + CROP_MARGIN;
    let crop = decode_crop(full_jpeg, cx as usize, cy as usize, side).ok()?;
    let local = |v: f64, origin: usize, limit: usize| {
        (v as i64 - origin as i64).clamp(0, limit as i64) as usize
    };
    let (left, right) = (
        local(cx - half, crop.x, crop.width),
        local(cx + half, crop.x, crop.width),
    );
    let (top, bottom) = (
        local(cy - half, crop.y, crop.height),
        local(cy + half, crop.y, crop.height),
    );
    if right <= left || bottom <= top {
        return None;
    }
    let gray = luma(&crop.pixels, crop.width, crop.height);
    let cut: Vec<u8> = (top..bottom)
        .flat_map(|y| {
            gray[y * crop.width + left..y * crop.width + right]
                .iter()
                .copied()
        })
        .collect();
    Some(eye_crop_score(&cut, right - left, bottom - top))
}

/// The magnitude of numpy's `gradient` of a `width` x `height` image, both
/// sides at least 2: central differences inside, one-sided at the borders.
fn gradient_magnitude(a: &[f64], width: usize, height: usize) -> Vec<f64> {
    let along = |at: &dyn Fn(usize) -> f64, i: usize, n: usize| {
        if i == 0 {
            at(1) - at(0)
        } else if i == n - 1 {
            at(n - 1) - at(n - 2)
        } else {
            (at(i + 1) - at(i - 1)) / 2.0
        }
    };
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let gx = along(&|i| a[y * width + i], x, width);
            let gy = along(&|i| a[i * width + x], y, height);
            gx.hypot(gy)
        })
        .collect()
}

/// numpy's default (linear) `percentile` of the ascending, non-empty `sorted`.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = p / 100.0 * (sorted.len() - 1) as f64;
    let (below, above) = (rank.floor() as usize, rank.ceil() as usize);
    sorted[below] + (sorted[above] - sorted[below]) * (rank - below as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_rgb;

    /// A `side` px square, `bright` with a `dark` disc of radius `side / 3`.
    fn disc(side: usize, dark: u8, bright: u8) -> Vec<u8> {
        let c = side as f64 / 2.0;
        (0..side * side)
            .map(|i| {
                let (x, y) = ((i % side) as f64 + 0.5 - c, (i / side) as f64 + 0.5 - c);
                if x.hypot(y) < side as f64 / 3.0 {
                    dark
                } else {
                    bright
                }
            })
            .collect()
    }

    /// `gray` under a 5 x 5 box filter, the window clamped at the borders.
    fn box_blurred(gray: &[u8], side: usize) -> Vec<u8> {
        let at = |x: isize, y: isize| {
            let clamp = |v: isize| v.clamp(0, side as isize - 1) as usize;
            u32::from(gray[clamp(y) * side + clamp(x)])
        };
        (0..side * side)
            .map(|i| {
                let (x, y) = ((i % side) as isize, (i / side) as isize);
                let sum: u32 = (-2..=2)
                    .flat_map(|dy| (-2..=2).map(move |dx| (dx, dy)))
                    .map(|(dx, dy)| at(x + dx, y + dy))
                    .sum();
                (sum / 25) as u8
            })
            .collect()
    }

    fn encode(gray: &[u8], w: usize, h: usize) -> Vec<u8> {
        let rgb: Vec<u8> = gray.iter().flat_map(|&g| [g, g, g]).collect();
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        c.set_size(w, h);
        c.set_quality(98.0);
        let mut c = c.start_compress(Vec::new()).unwrap();
        c.write_scanlines(&rgb).unwrap();
        c.finish().unwrap()
    }

    /// `GOOD_FULL_EYE_SHARP` in `crates/app/ui/src/focus.ts` is this figure:
    /// update it with the threshold.
    #[test]
    fn the_threshold_is_the_figure_the_ui_cuts_good_at() {
        assert_eq!(SOFT_BELOW, 0.162);
    }

    #[test]
    fn a_blurred_crop_scores_under_the_threshold_and_a_sharp_one_over_it() {
        let sharp = disc(100, 30, 220);
        let blurred = box_blurred(&sharp, 100);
        let (s, b) = (
            eye_crop_score(&sharp, 100, 100),
            eye_crop_score(&blurred, 100, 100),
        );
        assert!(b < SOFT_BELOW, "blurred {b}");
        assert!(s > SOFT_BELOW, "sharp {s}");
        assert!(s > b * 1.5, "sharp {s}, blurred {b}");
    }

    #[test]
    fn a_uniform_brightness_scale_leaves_the_score_alone() {
        let (bright, dim) = (disc(100, 40, 200), disc(100, 20, 100));
        let (a, b) = (
            eye_crop_score(&bright, 100, 100),
            eye_crop_score(&dim, 100, 100),
        );
        assert!((a - b).abs() / a < 0.02, "{a} and {b}");
    }

    #[test]
    fn a_full_jpeg_not_larger_than_the_scale_floor_has_no_score() {
        let jpeg = encode(&disc(150, 30, 220), 150, 150);
        let eyes = [(40.0, 75.0), (110.0, 75.0)];
        assert_eq!(full_eye_sharpness(&jpeg, eyes, 150), None);
        assert_eq!(full_eye_sharpness(&jpeg, eyes, 100), None, "exactly 1.5x");
        assert!(full_eye_sharpness(&jpeg, eyes, 99).is_some());
        assert_eq!(full_eye_sharpness(&[0u8; 16], eyes, 100), None);
    }

    #[test]
    fn the_score_is_the_sharper_eye_cut_at_the_exact_square_of_the_full_image() {
        // Two discs side by side: a blurred one and a sharp one.
        let side = 160;
        let (sharp, blurred) = (disc(side, 30, 220), box_blurred(&disc(side, 30, 220), side));
        let (w, h) = (side * 2 + 16, side);
        let mut gray = Vec::with_capacity(w * h);
        for y in 0..h {
            gray.extend_from_slice(&blurred[y * side..(y + 1) * side]);
            gray.extend_from_slice(&[220; 16]);
            gray.extend_from_slice(&sharp[y * side..(y + 1) * side]);
        }
        let jpeg = encode(&gray, w, h);
        // A quarter-size preview, its eyes 17.5 px apart: 0.6 * 17.5 * 4 = 42.
        // The first crop sits on the blurred disc's rim, the second on the
        // sharp disc's.
        let eyes = [(33.25f32, 20.0f32), (50.75, 20.0)];
        let (decoded, dw, _) = decode_rgb(&jpeg).unwrap();
        let full = luma(&decoded, dw, h);
        let at = |cx: f64, cy: f64| {
            let (x0, x1) = ((cx - 42.0) as usize, (cx + 42.0) as usize);
            let (y0, y1) = ((cy - 42.0) as usize, (cy + 42.0) as usize);
            let cut: Vec<u8> = (y0..y1)
                .flat_map(|y| full[y * dw + x0..y * dw + x1].iter().copied())
                .collect();
            eye_crop_score(&cut, x1 - x0, y1 - y0)
        };
        let (soft, crisp) = (at(133.0, 80.0), at(203.0, 80.0));
        assert!(crisp > soft, "sharp {crisp}, blurred {soft}");
        let want = crisp;
        let got = full_eye_sharpness(&jpeg, eyes, w / 4).unwrap();
        assert!((got - want).abs() < 0.01, "{got} and {want}");
    }

    #[test]
    fn eyes_wholly_outside_the_image_have_no_score() {
        let jpeg = encode(&disc(160, 30, 220), 160, 160);
        let eyes = [(5000.0, 5000.0), (5010.0, 5000.0)];
        assert_eq!(full_eye_sharpness(&jpeg, eyes, 40), None);
    }

    #[test]
    fn the_gradient_follows_numpy_with_one_sided_borders() {
        // np.hypot(*np.gradient(np.array([[1., 2., 4.], [3., 7., 8.]])))
        let a = [1.0, 2.0, 4.0, 3.0, 7.0, 8.0];
        let want = [
            2.0f64.hypot(1.0),
            5.0f64.hypot(1.5),
            4.0f64.hypot(2.0),
            2.0f64.hypot(4.0),
            5.0f64.hypot(2.5),
            4.0f64.hypot(1.0),
        ];
        let got = gradient_magnitude(&a, 3, 2);
        assert!(
            got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-12),
            "{got:?}"
        );
    }

    #[test]
    fn the_percentile_interpolates_linearly_as_numpy_does() {
        let sorted = [1.0, 2.0, 4.0, 8.0];
        assert_eq!(percentile(&sorted, 0.0), 1.0);
        assert_eq!(percentile(&sorted, 50.0), 3.0);
        assert_eq!(percentile(&sorted, 100.0), 8.0);
        assert!((percentile(&sorted, 90.0) - 6.8).abs() < 1e-12);
    }
}
