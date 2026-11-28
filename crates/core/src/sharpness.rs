//! A focus-quality score: the variance of the Laplacian of the luma, taken
//! over one of four regions of the preview:
//!
//! 0. A Sony frame whose camera tracked a face (`eye_af_frame`): a window
//!    centered on the AF point, its side the AF frame's long side in preview
//!    pixels clamped to `[EYE_WINDOW_MIN, WINDOW]`. Faces are ignored and
//!    the scan does not detect them.
//! 1. A trustworthy AF point: the `WINDOW`-sized window on it. Faces are
//!    ignored, even when the point lies outside every face box: the detector
//!    cannot tell the face the photographer wanted from a bystander, so the
//!    camera's AF point is trusted over a face it did not land on.
//! 2. No trustworthy AF point (none, or a Sony frame shot in manual focus;
//!    an older `DSC-` body whose `FocusMode` always reads 0 is never taken
//!    as manual focus, see `excluded_dsc`) and a face found (the best one
//!    scoring at least `FACE_CONFIDENCE`): a window centered between the two
//!    eyes, its side the face box's long side clamped to
//!    `[EYE_WINDOW_MIN, WINDOW]`.
//! 3. Neither: the maximum over a grid of tiles covering the preview, so a
//!    frame sharp anywhere ranks above one sharp nowhere.
//!
//! Only meaningful relative to other frames. `score_preview` decodes the
//! preview to grayscale (the JPEG's Y channel); `score_gray` scores luma the
//! caller already has, which the scan passes on paths 0, 1 and 2 from the
//! BT.601 luma of the face cue's RGB decode (path 3 has no cue decode and
//! decodes to grayscale), so those two inputs can differ slightly.

use std::panic::{catch_unwind, AssertUnwindSafe};

use anyhow::{anyhow, bail, Result};

use crate::arw::{excluded_dsc, FocusFrame, FocusLocation, Shot};
use crate::faces::Face;
use crate::partial::focus_point;

/// Side of the square window the score is taken over, in preview pixels.
pub const WINDOW: usize = 256;
/// Smallest side of the window around the eyes, in preview pixels.
pub const EYE_WINDOW_MIN: usize = 128;
/// Minimum detection score for a face to steer the window.
pub const FACE_CONFIDENCE: f32 = 0.8;

/// A rectangle in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

/// A `size` x `size` window centered on `(cx, cy)`, clamped into a
/// `width` x `height` image.
pub fn window_at(width: usize, height: usize, cx: usize, cy: usize, size: usize) -> Window {
    let w = size.min(width);
    let h = size.min(height);
    Window {
        x: cx.saturating_sub(w / 2).min(width - w),
        y: cy.saturating_sub(h / 2).min(height - h),
        width: w,
        height: h,
    }
}

/// Variance of the 3x3 Laplacian over the pixels of `window` whose
/// neighbors all lie inside it, so nothing outside the window counts.
/// `gray` is `width` x `height`, one byte per pixel.
pub fn laplacian_variance(gray: &[u8], width: usize, window: Window) -> f64 {
    if window.width < 3 || window.height < 3 {
        return 0.0;
    }
    let mut sum = 0i64;
    let mut sum_sq = 0i64;
    let mut n = 0i64;
    for y in window.y + 1..window.y + window.height - 1 {
        let row = y * width;
        for x in window.x + 1..window.x + window.width - 1 {
            let i = row + x;
            let l = gray[i - width] as i32
                + gray[i + width] as i32
                + gray[i - 1] as i32
                + gray[i + 1] as i32
                - 4 * gray[i] as i32;
            sum += l as i64;
            sum_sq += (l * l) as i64;
            n += 1;
        }
    }
    let mean = sum as f64 / n as f64;
    sum_sq as f64 / n as f64 - mean * mean
}

/// Sony `FocusMode` value for manual focus.
const MANUAL_FOCUS: u8 = 0;

/// Whether `shot` was taken in manual focus. DMF, bodies that write no
/// `FocusMode` and the `DSC-` bodies whose `FocusMode` always reads 0 count
/// as autofocus.
pub fn manual_focus(shot: &Shot) -> bool {
    shot.focus_mode == Some(MANUAL_FOCUS) && !excluded_dsc(shot.model.as_deref())
}

/// The focus location of `shot` when the AF point can be trusted: `None`
/// when there is none or the frame was shot in manual focus.
pub fn trusted_focus(shot: &Shot) -> Option<FocusLocation> {
    if manual_focus(shot) {
        None
    } else {
        shot.focus
    }
}

/// Sony `AFTracking` value for face tracking.
const FACE_TRACKING: u8 = 1;

/// The AF point and frame of `shot` when the camera tracked a face: a
/// trustworthy AF point, `AFTracking` face tracking and a valid frame, with
/// the point off the exact sensor center (where bodies leave it when tracking
/// never locked).
pub fn eye_af_frame(shot: &Shot) -> Option<(FocusLocation, FocusFrame)> {
    let focus = trusted_focus(shot)?;
    let frame = shot.focus_frame?;
    if shot.af_tracking != Some(FACE_TRACKING)
        || (focus.x == focus.sensor_w / 2 && focus.y == focus.sensor_h / 2)
    {
        return None;
    }
    Some((focus, frame))
}

/// Side of the eye-AF window: the frame's long side in preview pixels,
/// clamped to `[EYE_WINDOW_MIN, WINDOW]`.
fn frame_side(frame: FocusFrame, sensor_w: u16, preview_w: usize) -> usize {
    let side = frame.width.max(frame.height) as usize * preview_w / (sensor_w.max(1) as usize);
    side.clamp(EYE_WINDOW_MIN, WINDOW)
}

/// Maximum `laplacian_variance` over `WINDOW`-sized tiles at stride `WINDOW`
/// covering the `width` x `height` image, the last column and row clamped
/// inside it.
pub fn tile_max(gray: &[u8], width: usize, height: usize) -> f64 {
    let tw = WINDOW.min(width);
    let th = WINDOW.min(height);
    let mut best = 0.0f64;
    for y in (0..height).step_by(WINDOW) {
        for x in (0..width).step_by(WINDOW) {
            let tile = Window {
                x: x.min(width - tw),
                y: y.min(height - th),
                width: tw,
                height: th,
            };
            best = best.max(laplacian_variance(gray, width, tile));
        }
    }
    best
}

/// The highest-scoring face at or above `FACE_CONFIDENCE`.
fn chosen_face(faces: &[Face]) -> Option<&Face> {
    faces
        .iter()
        .filter(|f| f.score >= FACE_CONFIDENCE)
        .max_by(|a, b| a.score.total_cmp(&b.score))
}

/// The window between the eyes of `face`, sized from its box.
fn eye_window(width: usize, height: usize, face: &Face) -> Window {
    let cx = (face.left_eye.0 + face.right_eye.0) / 2.0;
    let cy = (face.left_eye.1 + face.right_eye.1) / 2.0;
    let side = (face.width.max(face.height).max(0.0) as usize).clamp(EYE_WINDOW_MIN, WINDOW);
    window_at(
        width,
        height,
        (cx.max(0.0) as usize).min(width - 1),
        (cy.max(0.0) as usize).min(height - 1),
        side,
    )
}

/// Decode `preview` to grayscale and score it along the four paths of the
/// module doc. Pass `trusted_focus` as `focus`, the frame of
/// `eye_af_frame` as `frame` (used only with a `focus`), and `faces` in the pixel
/// coordinates of the preview as stored (before any orientation is
/// applied). mozjpeg aborts through a panic on bytes that are not a JPEG;
/// that comes back as `Err` too.
pub fn score_preview(
    preview: &[u8],
    focus: Option<FocusLocation>,
    frame: Option<FocusFrame>,
    faces: &[Face],
) -> Result<f64> {
    catch_unwind(AssertUnwindSafe(|| score(preview, focus, frame, faces)))
        .map_err(|_| anyhow!("panic while decoding the preview"))?
}

fn score(
    preview: &[u8],
    focus: Option<FocusLocation>,
    frame: Option<FocusFrame>,
    faces: &[Face],
) -> Result<f64> {
    let mut d = mozjpeg::Decompress::new_mem(preview)?.grayscale()?;
    let (w, h) = (d.width(), d.height());
    let gray: Vec<u8> = d.read_scanlines()?;
    d.finish()?;
    score_gray(&gray, w, h, focus, frame, faces)
}

/// `score_preview` on an already decoded `width` x `height` luma image, one
/// byte per pixel, in the stored orientation of the preview `faces` were
/// found on.
pub fn score_gray(
    gray: &[u8],
    w: usize,
    h: usize,
    focus: Option<FocusLocation>,
    frame: Option<FocusFrame>,
    faces: &[Face],
) -> Result<f64> {
    Ok(match region(w, h, focus, frame, faces)? {
        Region::Window(window) => laplacian_variance(gray, w, window),
        Region::Tiles => tile_max(gray, w, h),
    })
}

/// What `score_gray` reads of a `w` x `h` image: one window, or the tiles
/// of `tile_max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Window(Window),
    Tiles,
}

/// The region `score_gray` scores for these arguments, along the four paths
/// of the module doc; `Err` when the image has no interior pixel.
pub fn region(
    w: usize,
    h: usize,
    focus: Option<FocusLocation>,
    frame: Option<FocusFrame>,
    faces: &[Face],
) -> Result<Region> {
    if w < 3 || h < 3 {
        bail!("a {w}x{h} preview has no interior pixel to score");
    }
    let point = focus.map(|f| focus_point(w, h, Some(f)));
    if let (Some(f), Some(frame), Some(p)) = (focus, frame, point) {
        let side = frame_side(frame, f.sensor_w, w);
        return Ok(Region::Window(window_at(w, h, p.0, p.1, side)));
    }
    Ok(match (point, chosen_face(faces)) {
        (Some(p), _) => Region::Window(window_at(w, h, p.0, p.1, WINDOW)),
        (None, Some(face)) => Region::Window(eye_window(w, h, face)),
        (None, None) => Region::Tiles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker(w: usize, h: usize, cell: usize) -> Vec<u8> {
        (0..h)
            .flat_map(|y| {
                (0..w).map(move |x| {
                    if (x / cell + y / cell).is_multiple_of(2) {
                        0
                    } else {
                        255
                    }
                })
            })
            .collect()
    }

    fn blur(gray: &[u8], w: usize, h: usize) -> Vec<u8> {
        let mut out = gray.to_vec();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let mut s = 0u32;
                for dy in 0..3 {
                    for dx in 0..3 {
                        s += gray[(y + dy - 1) * w + x + dx - 1] as u32;
                    }
                }
                out[y * w + x] = (s / 9) as u8;
            }
        }
        out
    }

    fn jpeg(gray: &[u8], w: usize, h: usize) -> Vec<u8> {
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_GRAYSCALE);
        c.set_size(w, h);
        c.set_quality(95.0);
        let mut c = c.start_compress(Vec::new()).unwrap();
        c.write_scanlines(gray).unwrap();
        c.finish().unwrap()
    }

    fn all(w: usize, h: usize) -> Window {
        Window {
            x: 0,
            y: 0,
            width: w,
            height: h,
        }
    }

    #[test]
    fn a_constant_image_scores_zero() {
        assert_eq!(laplacian_variance(&[128u8; 64 * 64], 64, all(64, 64)), 0.0);
    }

    #[test]
    fn a_blurred_checkerboard_scores_lower() {
        let sharp = checker(64, 64, 4);
        let soft = blur(&sharp, 64, 64);
        assert!(
            laplacian_variance(&sharp, 64, all(64, 64))
                > laplacian_variance(&soft, 64, all(64, 64))
        );
    }

    #[test]
    fn content_outside_the_window_does_not_count() {
        let mut gray = vec![100u8; 64 * 64];
        for y in 0..64 {
            for x in 32..64 {
                gray[y * 64 + x] = if (x + y).is_multiple_of(2) { 0 } else { 255 };
            }
        }
        let left = Window {
            x: 0,
            y: 0,
            width: 32,
            height: 64,
        };
        assert_eq!(laplacian_variance(&gray, 64, left), 0.0);
    }

    #[test]
    fn a_window_near_a_corner_is_clamped_inside() {
        let w = window_at(1616, 1080, 1610, 5, WINDOW);
        assert_eq!(
            w,
            Window {
                x: 1616 - 256,
                y: 0,
                width: 256,
                height: 256
            }
        );
        let small = window_at(100, 80, 0, 0, WINDOW);
        assert_eq!(small, all(100, 80));
    }

    fn with_checker(w: usize, h: usize, x0: usize, y0: usize, side: usize) -> Vec<u8> {
        let mut gray = vec![128u8; w * h];
        for y in y0..y0 + side {
            for x in x0..x0 + side {
                gray[y * w + x] = if (x / 4 + y / 4).is_multiple_of(2) {
                    0
                } else {
                    255
                };
            }
        }
        gray
    }

    #[test]
    fn with_a_focus_location_only_its_window_counts() {
        let (w, h) = (1024, 768);
        let jpeg = jpeg(&with_checker(w, h, w / 2 - 64, h / 2 - 64, 128), w, h);
        let center = FocusLocation {
            sensor_w: 1024,
            sensor_h: 768,
            x: 512,
            y: 384,
        };
        assert!(score_preview(&jpeg, Some(center), None, &[]).unwrap() > 0.0);
        let corner = FocusLocation {
            x: 0,
            y: 0,
            ..center
        };
        assert!(score_preview(&jpeg, Some(corner), None, &[]).unwrap() < 1.0);
    }

    #[test]
    fn without_a_focus_location_the_sharpest_tile_is_scored() {
        let (w, h) = (1000, 700);
        let gray = with_checker(w, h, w - 200, h - 200, 200);
        let sharp = jpeg(&gray, w, h);
        let soft = jpeg(&blur(&blur(&gray, w, h), w, h), w, h);
        let score = score_preview(&sharp, None, None, &[]).unwrap();
        assert!(score > score_preview(&soft, None, None, &[]).unwrap());
        let mut d = mozjpeg::Decompress::new_mem(&sharp)
            .unwrap()
            .grayscale()
            .unwrap();
        let decoded: Vec<u8> = d.read_scanlines().unwrap();
        d.finish().unwrap();
        let corner = window_at(w, h, w, h, WINDOW);
        let expected = laplacian_variance(&decoded, w, corner);
        assert!((score - expected).abs() < 1e-6);
    }

    #[test]
    fn only_focus_mode_zero_is_manual_focus() {
        let mode = |focus_mode| Shot {
            focus_mode,
            ..Shot::default()
        };
        assert!(manual_focus(&mode(Some(0))));
        assert!(!manual_focus(&mode(Some(3))));
        assert!(!manual_focus(&mode(Some(6))));
        assert!(!manual_focus(&mode(None)));
    }

    #[test]
    fn focus_mode_zero_on_an_excluded_dsc_body_is_not_manual_focus() {
        let zero_on = |model: Option<&str>| Shot {
            focus_mode: Some(0),
            model: model.map(str::to_owned),
            ..Shot::default()
        };
        assert!(!manual_focus(&zero_on(Some("DSC-RX100M3"))));
        assert!(manual_focus(&zero_on(Some("DSC-RX100M7"))));
        assert!(manual_focus(&zero_on(Some("ILCE-7M5"))));
        assert!(manual_focus(&zero_on(None)));
    }

    #[test]
    fn a_manual_focus_frame_ignores_its_focus_location() {
        let (w, h) = (1024, 768);
        let jpeg = jpeg(&with_checker(w, h, 0, 0, 200), w, h);
        let shot = Shot {
            focus: Some(FocusLocation {
                sensor_w: 1024,
                sensor_h: 768,
                x: 512,
                y: 384,
            }),
            focus_mode: Some(0),
            ..Shot::default()
        };
        assert!(score_preview(&jpeg, trusted_focus(&shot), None, &[]).unwrap() > 0.0);
        let af = Shot {
            focus_mode: Some(3),
            ..shot
        };
        assert!(score_preview(&jpeg, trusted_focus(&af), None, &[]).unwrap() < 1.0);
    }

    #[test]
    fn a_preview_too_small_to_score_is_an_error() {
        assert!(score_preview(&jpeg(&[0u8; 4], 2, 2), None, None, &[]).is_err());
    }

    #[test]
    fn a_preview_that_is_not_a_jpeg_is_an_error() {
        assert!(score_preview(&[0u8; 512], None, None, &[]).is_err());
    }

    fn face(x: f32, y: f32, side: f32, score: f32) -> Face {
        Face {
            x,
            y,
            width: side,
            height: side,
            score,
            left_eye: (x + side * 0.3, y + side * 0.4),
            right_eye: (x + side * 0.7, y + side * 0.4),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        }
    }

    fn focus_at(w: usize, h: usize, x: usize, y: usize) -> FocusLocation {
        FocusLocation {
            sensor_w: w as u16,
            sensor_h: h as u16,
            x: x as u16,
            y: y as u16,
        }
    }

    // A 200 px face at (100, 100) whose eyes (y = 180) are the only sharp
    // region; a larger, softer checkerboard sits in the bottom-right corner.
    fn portrait(w: usize, h: usize) -> Vec<u8> {
        let mut gray = blur(
            &blur(&with_checker(w, h, w - 300, h - 300, 300), w, h),
            w,
            h,
        );
        let sharp = with_checker(w, h, 150, 150, 100);
        for y in 150..214 {
            for x in 150..250 {
                gray[y * w + x] = sharp[y * w + x];
            }
        }
        gray
    }

    #[test]
    fn a_face_scores_its_eyes_instead_of_the_sharpest_tile() {
        let (w, h) = (1000, 700);
        let gray = portrait(w, h);
        let sharp = jpeg(&gray, w, h);
        let soft = jpeg(&blur(&blur(&gray, w, h), w, h), w, h);
        let f = [face(100.0, 100.0, 200.0, 0.95)];
        assert!(
            score_preview(&sharp, None, None, &f).unwrap()
                > score_preview(&soft, None, None, &[]).unwrap()
        );
        let mut d = mozjpeg::Decompress::new_mem(&sharp)
            .unwrap()
            .grayscale()
            .unwrap();
        let decoded: Vec<u8> = d.read_scanlines().unwrap();
        d.finish().unwrap();
        let expected = laplacian_variance(&decoded, w, window_at(w, h, 200, 180, 200));
        assert!((score_preview(&sharp, None, None, &f).unwrap() - expected).abs() < 1e-6);
    }

    #[test]
    fn a_focus_point_inside_the_face_keeps_the_af_window() {
        let (w, h) = (1000, 700);
        let sharp = jpeg(&portrait(w, h), w, h);
        let f = [face(100.0, 100.0, 200.0, 0.95)];
        let chin = focus_at(w, h, 200, 290);
        assert_eq!(
            score_preview(&sharp, Some(chin), None, &f).unwrap(),
            score_preview(&sharp, Some(chin), None, &[]).unwrap()
        );
    }

    #[test]
    fn a_focus_point_outside_the_face_keeps_the_af_window() {
        let (w, h) = (1000, 700);
        let sharp = jpeg(&portrait(w, h), w, h);
        let f = [face(100.0, 100.0, 200.0, 0.95)];
        let background = focus_at(w, h, 850, 550);
        assert_eq!(
            score_preview(&sharp, Some(background), None, &f).unwrap(),
            score_preview(&sharp, Some(background), None, &[]).unwrap()
        );
    }

    #[test]
    fn a_low_confidence_face_is_ignored() {
        let (w, h) = (1000, 700);
        let sharp = jpeg(&portrait(w, h), w, h);
        let f = [face(100.0, 100.0, 200.0, FACE_CONFIDENCE - 0.1)];
        assert_eq!(
            score_preview(&sharp, None, None, &f).unwrap(),
            score_preview(&sharp, None, None, &[]).unwrap()
        );
    }

    #[test]
    fn a_decoded_luma_scores_as_its_preview_does_on_every_path() {
        let (w, h) = (1000, 700);
        let sharp = jpeg(&portrait(w, h), w, h);
        let mut d = mozjpeg::Decompress::new_mem(&sharp)
            .unwrap()
            .grayscale()
            .unwrap();
        let decoded: Vec<u8> = d.read_scanlines().unwrap();
        d.finish().unwrap();
        let f = [face(100.0, 100.0, 200.0, 0.95)];
        let point = Some(focus_at(w, h, 850, 550));
        let frame = Some(FocusFrame {
            width: 300,
            height: 200,
        });
        for (focus, frame, faces) in [
            (None, None, &[][..]),
            (None, None, &f[..]),
            (point, None, &f[..]),
            (point, frame, &f[..]),
        ] {
            assert_eq!(
                score_gray(&decoded, w, h, focus, frame, faces).unwrap(),
                score_preview(&sharp, focus, frame, faces).unwrap()
            );
        }
    }

    #[test]
    fn a_luma_too_small_to_score_is_an_error() {
        assert!(score_gray(&[0u8; 4], 2, 2, None, None, &[]).is_err());
    }

    #[test]
    fn the_eye_window_side_is_clamped() {
        let small = face(100.0, 100.0, 40.0, 0.9);
        assert_eq!(eye_window(1000, 700, &small).width, EYE_WINDOW_MIN);
        let large = face(100.0, 100.0, 600.0, 0.9);
        assert_eq!(eye_window(1000, 700, &large).width, WINDOW);
    }

    fn tracked(x: u16, y: u16, tracking: Option<u8>, frame: Option<FocusFrame>) -> Shot {
        Shot {
            focus: Some(FocusLocation {
                sensor_w: 7008,
                sensor_h: 4672,
                x,
                y,
            }),
            focus_mode: Some(3),
            af_tracking: tracking,
            focus_frame: frame,
            ..Shot::default()
        }
    }

    const SMALL: FocusFrame = FocusFrame {
        width: 153,
        height: 154,
    };

    #[test]
    fn the_eye_af_gate_needs_engaged_face_tracking() {
        assert!(eye_af_frame(&tracked(2000, 1500, Some(0), Some(SMALL))).is_none());
        assert!(eye_af_frame(&tracked(2000, 1500, Some(2), Some(SMALL))).is_none());
        assert!(eye_af_frame(&tracked(2000, 1500, None, Some(SMALL))).is_none());
        assert!(eye_af_frame(&tracked(2000, 1500, Some(1), None)).is_none());
        assert!(eye_af_frame(&tracked(3504, 2336, Some(1), Some(SMALL))).is_none());
        let manual = Shot {
            focus_mode: Some(0),
            ..tracked(2000, 1500, Some(1), Some(SMALL))
        };
        assert!(eye_af_frame(&manual).is_none());
        let shot = tracked(2000, 1500, Some(1), Some(SMALL));
        assert_eq!(eye_af_frame(&shot), Some((shot.focus.unwrap(), SMALL)));
        assert!(eye_af_frame(&tracked(3504, 2297, Some(1), Some(SMALL))).is_some());
    }

    #[test]
    fn the_eye_af_window_side_follows_the_frame() {
        assert_eq!(frame_side(SMALL, 7008, 1616), 128);
        let close = FocusFrame {
            width: 1533,
            height: 1535,
        };
        assert_eq!(frame_side(close, 7008, 1616), 256);
    }

    #[test]
    fn with_an_eye_af_frame_faces_do_not_move_the_window() {
        let (w, h) = (1000, 700);
        let sharp = jpeg(&portrait(w, h), w, h);
        let f = [face(100.0, 100.0, 200.0, 0.95)];
        let background = focus_at(w, h, 850, 550);
        let frame = Some(FocusFrame {
            width: 200,
            height: 200,
        });
        let with_face = score_preview(&sharp, Some(background), frame, &f).unwrap();
        assert_eq!(
            with_face,
            score_preview(&sharp, Some(background), frame, &[]).unwrap()
        );
        assert_ne!(with_face, score_preview(&sharp, None, None, &f).unwrap());
        let mut d = mozjpeg::Decompress::new_mem(&sharp)
            .unwrap()
            .grayscale()
            .unwrap();
        let decoded: Vec<u8> = d.read_scanlines().unwrap();
        d.finish().unwrap();
        let expected = laplacian_variance(&decoded, w, window_at(w, h, 850, 550, 200));
        assert!((with_face - expected).abs() < 1e-6);
    }
}
