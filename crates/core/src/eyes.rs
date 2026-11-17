//! Whether the eyes of a face are closed, from the eyelid points of the
//! MediaPipe Face Landmarker v2 face mesh (`face_landmarks_detector.onnx`,
//! Apache-2.0, converted from Google's TFLite; provenance in
//! `models/LICENSE-mediapipe`), run through `tract`.
//!
//! The face crop is the YuNet box squared on its longer side and grown
//! `FACE_CROP` times (`face_square`), cut from the full-size upright preview
//! with `faces::crop_rgb`, resized bilinearly to `INPUT` x `INPUT` and fed as
//! RGB NHWC 0..1, with no roll correction. Each eye's eye aspect ratio (EAR)
//! comes from six of the 478 points; the more closed eye is mapped to a
//! closed probability by a logistic on the EAR (`EYES_CLOSED_EAR`,
//! `EYES_LOGIT_SLOPE`). The same points, z included, give the face's head
//! pose (`pose::head_pose`).
//!
//! The geometry and the constants are the ones measured on the hand-labeled
//! faces of `docs/plans/20261007-closed-eyes-detection/eyes-truth.md`
//! (2026-10-07, 253 faces with an open or closed label, 57 closed): face AUC
//! 0.974. A change to the crop, the resize or the points is a new model:
//! re-validate it with `riffle-cli eyes` on those faces.

use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

use anyhow::{anyhow, Context, Result};
use tract_onnx::prelude::*;

use crate::faces::{crop_rgb, face_to_upright, point_to_stored, Face};
use crate::pose::{head_pose, Pose};
use crate::sharpness::{window_at, Window};

const MODEL: &[u8] = include_bytes!("../models/face_landmarks_detector.onnx");

/// Side of the square model input, in pixels.
pub const INPUT: usize = 256;
/// Number of points the model returns.
pub const LANDMARKS: usize = 478;
/// Side of a face crop as a multiple of the longer side of the face box.
/// YuNet's box runs from the forehead to the chin, so this is close to
/// MediaPipe's 1.5x of its own, tighter detector box.
pub const FACE_CROP: f32 = 1.25;
/// The smallest longer box side of a face that is judged, in preview pixels.
/// Below it a human can call the eye state of about one face in five on the
/// embedded preview, from it up about two in three (`eyes-truth.md`).
pub const EYES_MIN_FACE: f32 = 60.0;
/// The EAR at or below which the eyes are closed: the best F1 of `closed`
/// on the labeled faces (accuracy 0.957, precision 0.94, recall 0.86). It is
/// the logistic's midpoint, so a closed probability of at least 0.5 and
/// `EyeState::Closed` are the same test.
pub const EYES_CLOSED_EAR: f64 = 0.137;
/// The slope of the closed logit `EYES_LOGIT_SLOPE * (EYES_CLOSED_EAR - EAR)`:
/// the maximum-likelihood fit with the midpoint held at `EYES_CLOSED_EAR`,
/// on the per-face EARs of the Step 1 survey against the same labels
/// (28.996; a free intercept would put the midpoint at 0.155). An EAR of 0.10
/// reads 75% closed, 0.20 reads 14%.
pub const EYES_LOGIT_SLOPE: f64 = 29.0;
/// The EAR points of the eye on the left of the image (the subject's right
/// eye): the two corners `33` and `133`, the upper lid `160` / `158` and the
/// lower lid `153` / `144`, in the order `p1`..`p6` of Soukupová and Čech.
const LEFT_EYE: [usize; 6] = [33, 160, 158, 133, 153, 144];
/// The EAR points of the eye on the right of the image.
const RIGHT_EYE: [usize; 6] = [362, 385, 387, 263, 373, 380];
/// The eyelid contour of the eye on the left of the image, the 16 points of
/// MediaPipe's `FACEMESH_RIGHT_EYE` edges (the subject's right eye; the same
/// edges `crates/native/src/facemesh.rs` draws). The contour and iris tables
/// come from `mediapipe/python/solutions/face_mesh_connections.py`; notice in
/// `models/LICENSE-mediapipe`.
pub const LEFT_EYE_CONTOUR: [usize; 16] = [
    33, 7, 163, 144, 145, 153, 154, 155, 133, 173, 157, 158, 159, 160, 161, 246,
];
/// The eyelid contour of the eye on the right of the image (`FACEMESH_LEFT_EYE`).
pub const RIGHT_EYE_CONTOUR: [usize; 16] = [
    263, 249, 390, 373, 374, 380, 381, 382, 362, 398, 384, 385, 386, 387, 388, 466,
];
/// The iris of the eye on the left of the image: its center, then the four
/// points of MediaPipe's `FACEMESH_RIGHT_IRIS`.
pub const LEFT_IRIS: [usize; 5] = [468, 469, 470, 471, 472];
/// The iris of the eye on the right of the image (`FACEMESH_LEFT_IRIS`).
pub const RIGHT_IRIS: [usize; 5] = [473, 474, 475, 476, 477];

/// Whether the eyes of a face are open or closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeState {
    Open,
    Closed,
}

/// The judgment of one face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eyes {
    /// The probability that the eyes are closed, 0..1.
    pub probability: f64,
    pub state: EyeState,
}

impl Eyes {
    /// The judgment of a face whose more closed eye has this EAR.
    pub fn from_ear(ear: f64) -> Self {
        Eyes {
            probability: closed_probability(ear),
            state: if ear <= EYES_CLOSED_EAR {
                EyeState::Closed
            } else {
                EyeState::Open
            },
        }
    }
}

/// The closed probability of an EAR: the logistic of
/// `EYES_LOGIT_SLOPE * (EYES_CLOSED_EAR - ear)`.
pub fn closed_probability(ear: f64) -> f64 {
    1.0 / (1.0 + (-EYES_LOGIT_SLOPE * (EYES_CLOSED_EAR - ear)).exp())
}

type Plan = Arc<TypedRunnableModel>;

struct Model {
    plan: Plan,
    /// Output index of `Identity`, the points.
    points: usize,
}

fn model() -> Result<&'static Model> {
    static MODEL_PLAN: OnceLock<std::result::Result<Model, String>> = OnceLock::new();
    MODEL_PLAN
        .get_or_init(|| build().map_err(|e| format!("{e:#}")))
        .as_ref()
        .map_err(|e| anyhow!("face mesh model: {e}"))
}

fn build() -> Result<Model> {
    let model = tract_onnx::onnx()
        .with_ignore_value_info(true)
        .with_ignore_output_shapes(true)
        .model_for_read(&mut Cursor::new(MODEL))?
        .with_input_fact(0, f32::fact([1, INPUT, INPUT, 3]).into())?;
    let names: Vec<String> = model
        .output_outlets()?
        .iter()
        .map(|o| model.outlet_label(*o).unwrap_or_default().to_string())
        .collect();
    let points = names
        .iter()
        .position(|n| n == "Identity")
        .with_context(|| format!("no output Identity in {names:?}"))?;
    let plan = model.into_optimized()?.into_runnable()?;
    Ok(Model { plan, points })
}

/// The center and side of the square crop of an upright `face`: its box,
/// squared on the longer side and grown by `FACE_CROP`.
pub fn face_square(face: &Face) -> ((usize, usize), usize) {
    let center = (
        (face.x + face.width / 2.0).round() as usize,
        (face.y + face.height / 2.0).round() as usize,
    );
    (
        center,
        (face.width.max(face.height) * FACE_CROP).round() as usize,
    )
}

/// The face whose eyes are judged among a detection's `faces`: the one
/// nearest the AF `point` (`candidate::nearest_face`, the `AF eye` face),
/// else the largest at or above `sharpness::FACE_CONFIDENCE`, as the labeled
/// faces were picked.
pub fn judged_face(faces: &[Face], point: Option<(usize, usize)>) -> Option<&Face> {
    match point {
        Some(p) => crate::candidate::nearest_face(faces, p),
        None => faces
            .iter()
            .filter(|f| f.score >= crate::sharpness::FACE_CONFIDENCE)
            .max_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height))),
    }
}

/// Judge the eyes of `face` on the full-size upright RGB image
/// (`width` x `height`, 3 bytes per pixel), `face` in its coordinates.
/// `None` when the face's longer box side is below `EYES_MIN_FACE`, its
/// center lies outside the image, or the model fails or panics: a bad crop
/// leaves the eyes unknown instead of failing the caller.
pub fn judge(rgb: &[u8], width: usize, height: usize, face: &Face) -> Option<Eyes> {
    judge_mesh(rgb, width, height, face).map(|j| j.eyes)
}

/// A judgment with the face mesh it was taken on.
#[derive(Debug, Clone, PartialEq)]
pub struct Judged {
    pub eyes: Eyes,
    /// The EAR of the more closed eye the judgment was read from.
    pub ear: f64,
    /// The `LANDMARKS` points in the full-size upright image's pixels, z at
    /// the scale of x (see `landmarks_of`).
    pub points: Vec<[f32; 3]>,
    /// The head pose of the face (`pose::head_pose`), `None` when the solve
    /// fails or the fit is upside down (the roll past `MAX_ROLL`).
    pub pose: Option<Pose>,
}

/// `judge` with the mesh points, under the same floor and failures.
pub fn judge_mesh(rgb: &[u8], width: usize, height: usize, face: &Face) -> Option<Judged> {
    if face.width.max(face.height) < EYES_MIN_FACE {
        return None;
    }
    // Nothing here is shared or observable after a panic, hence
    // `AssertUnwindSafe`.
    let points = catch_unwind(AssertUnwindSafe(|| landmarks_of(rgb, width, height, face)))
        .ok()
        .and_then(Result::ok)
        .flatten()?;
    let ear = more_closed_ear(&points)?;
    Some(Judged {
        eyes: Eyes::from_ear(ear),
        ear,
        pose: head_pose(&points, width, height),
        points,
    })
}

/// A face mesh in the stored (unrotated) image's pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// The `LANDMARKS` points, x and y only.
    pub points: Vec<[f32; 2]>,
    /// The head pose, solved on the upright points (as `judge_mesh` does).
    pub pose: Option<Pose>,
}

/// The mesh of `face` on the stored `width` x `height` RGB image of EXIF
/// `orientation`, `face` in its coordinates, whatever the face's size. Only
/// the face crop `landmarks_of` would cut from the upright image is rotated
/// (`upright_crop`), not the whole image. `None` when the face's center lies
/// outside the image, or the model fails or panics.
pub fn mesh_of(
    rgb: &[u8],
    width: usize,
    height: usize,
    orientation: u16,
    face: &Face,
) -> Option<Mesh> {
    let (uw, uh) = match orientation {
        6 | 8 => (height, width),
        _ => (width, height),
    };
    let up = face_to_upright(*face, orientation, width, height);
    let (cx, cy) = (up.x + up.width / 2.0, up.y + up.height / 2.0);
    if rgb.len() < width * height * 3
        || !(0.0..uw as f32).contains(&cx)
        || !(0.0..uh as f32).contains(&cy)
    {
        return None;
    }
    let (center, side) = face_square(&up);
    let win = window_at(uw, uh, center.0, center.1, side.max(1));
    let sub = upright_crop(rgb, width, height, orientation, win);
    // Nothing here is shared or observable after a panic, hence
    // `AssertUnwindSafe`.
    let points = catch_unwind(AssertUnwindSafe(|| {
        landmarks(input_tensor(&sub, win.width, win.height))
    }))
    .ok()?
    .ok()?;
    let points = to_full(&points, &win);
    let pose = head_pose(&points, uw, uh);
    let points = points
        .iter()
        .map(|p| {
            let (x, y) = point_to_stored((p[0], p[1]), orientation, width, height);
            [x, y]
        })
        .collect();
    Some(Mesh { points, pose })
}

/// The `win` window of the upright image of a stored `width` x `height` RGB
/// image of EXIF `orientation`, the pixels `faces::upright_rgb` would put
/// there, without rotating the rest.
fn upright_crop(rgb: &[u8], width: usize, height: usize, orientation: u16, win: Window) -> Vec<u8> {
    let mut out = Vec::with_capacity(win.width * win.height * 3);
    for y in win.y..win.y + win.height {
        for x in win.x..win.x + win.width {
            let (sx, sy) = match orientation {
                6 => (y, height - 1 - x),
                8 => (width - 1 - y, x),
                3 => (width - 1 - x, height - 1 - y),
                _ => (x, y),
            };
            let i = (sy * width + sx) * 3;
            out.extend_from_slice(&rgb[i..i + 3]);
        }
    }
    out
}

/// The EAR of the more closed eye of `face` (see `judge`), whatever the
/// face's size; `None` when its center lies outside the image or neither
/// eye's EAR is finite.
pub fn ear_of(rgb: &[u8], width: usize, height: usize, face: &Face) -> Result<Option<f64>> {
    Ok(landmarks_of(rgb, width, height, face)?.and_then(|p| more_closed_ear(&p)))
}

/// The `LANDMARKS` points of `face`'s mesh in the full-size upright image's
/// pixels, whatever the face's size; `None` when its center lies outside the
/// image. z is the model's relative depth (larger away from the camera)
/// scaled like x, as MediaPipe's `LandmarkProjectionCalculator` does.
pub fn landmarks_of(
    rgb: &[u8],
    width: usize,
    height: usize,
    face: &Face,
) -> Result<Option<Vec<[f32; 3]>>> {
    let (cx, cy) = (face.x + face.width / 2.0, face.y + face.height / 2.0);
    if rgb.len() < width * height * 3
        || !(0.0..width as f32).contains(&cx)
        || !(0.0..height as f32).contains(&cy)
    {
        return Ok(None);
    }
    let (center, side) = face_square(face);
    let (sub, win) = crop_rgb(rgb, width, height, center, side.max(1));
    let points = landmarks(input_tensor(&sub, win.width, win.height))?;
    Ok(Some(to_full(&points, &win)))
}

/// Map the model's points, in input pixels, onto the full image the crop
/// `win` was cut from, z by the horizontal factor.
fn to_full(points: &[[f32; 3]], win: &Window) -> Vec<[f32; 3]> {
    let (kx, ky) = (
        win.width as f32 / INPUT as f32,
        win.height as f32 / INPUT as f32,
    );
    points
        .iter()
        .map(|p| {
            [
                win.x as f32 + p[0] * kx,
                win.y as f32 + p[1] * ky,
                p[2] * kx,
            ]
        })
        .collect()
}

/// The EAR of the more closed eye of a face's mesh `points`; `None` when
/// neither eye's EAR is finite. Only x and y are read, so the 3D upright
/// points and `Mesh::points` (2D, stored coordinates) give the same EAR: a
/// quarter turn or a half turn keeps every distance.
pub fn more_closed_ear<P: AsRef<[f32]>>(points: &[P]) -> Option<f64> {
    let (left, right) = ears(points);
    let ear = left.min(right);
    ear.is_finite().then_some(ear as f64)
}

/// Run the model on an `input_tensor`: the 478 (x, y, z) points, x and y in
/// input pixels.
fn landmarks(input: Tensor) -> Result<Vec<[f32; 3]>> {
    let m = model()?;
    let out = m.plan.run(tvec!(input.into()))?;
    let v = out[m.points].try_as_plain_ram()?.as_slice::<f32>()?;
    if v.len() < LANDMARKS * 3 {
        return Err(anyhow!(
            "face mesh model: {} values, not {}",
            v.len(),
            LANDMARKS * 3
        ));
    }
    Ok(v.chunks_exact(3)
        .take(LANDMARKS)
        .map(|p| [p[0], p[1], p[2]])
        .collect())
}

/// Resize an RGB crop bilinearly to `INPUT` x `INPUT` into an RGB NHWC
/// tensor of 0..1 values.
fn input_tensor(rgb: &[u8], width: usize, height: usize) -> Tensor {
    let data: Vec<f32> = resize(rgb, width, height, INPUT, INPUT)
        .iter()
        .flat_map(|p| p.map(|v| v / 255.0))
        .collect();
    tract_ndarray::Array4::from_shape_vec((1, INPUT, INPUT, 3), data)
        .expect("shape matches the buffer")
        .into()
}

/// Bilinear resize of a `width` x `height` RGB image to `out_w` x `out_h`,
/// pixel centers aligned (half-pixel offsets), edges clamped.
fn resize(rgb: &[u8], width: usize, height: usize, out_w: usize, out_h: usize) -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(out_w * out_h);
    let kx = width as f32 / out_w as f32;
    let ky = height as f32 / out_h as f32;
    for oy in 0..out_h {
        let fy = ((oy as f32 + 0.5) * ky - 0.5).clamp(0.0, (height - 1) as f32);
        let y0 = fy.floor() as usize;
        let y1 = (y0 + 1).min(height - 1);
        let ty = fy - y0 as f32;
        for ox in 0..out_w {
            let fx = ((ox as f32 + 0.5) * kx - 0.5).clamp(0.0, (width - 1) as f32);
            let x0 = fx.floor() as usize;
            let x1 = (x0 + 1).min(width - 1);
            let tx = fx - x0 as f32;
            let p = |x: usize, y: usize, c: usize| rgb[(y * width + x) * 3 + c] as f32;
            let mut px = [0f32; 3];
            for (c, v) in px.iter_mut().enumerate() {
                let a = p(x0, y0, c) * (1.0 - tx) + p(x1, y0, c) * tx;
                let b = p(x0, y1, c) * (1.0 - tx) + p(x1, y1, c) * tx;
                *v = a * (1.0 - ty) + b * ty;
            }
            out.push(px);
        }
    }
    out
}

/// The EAR of the eye on the left of the image, then the right one:
/// `(|p2 - p6| + |p3 - p5|) / (2 |p1 - p4|)`, on x and y only.
fn ears<P: AsRef<[f32]>>(points: &[P]) -> (f32, f32) {
    let d = |a: &[f32], b: &[f32]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let ear = |i: [usize; 6]| {
        let p = |k: usize| points[i[k]].as_ref();
        (d(p(1), p(5)) + d(p(2), p(4))) / (2.0 * d(p(0), p(3)))
    };
    (ear(LEFT_EYE), ear(RIGHT_EYE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upright() -> Face {
        Face {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
            score: 0.9,
            left_eye: (15.0, 25.0),
            right_eye: (35.0, 26.0),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        }
    }

    #[test]
    fn a_face_square_grows_the_longer_side_around_the_box_center() {
        assert_eq!(face_square(&upright()), ((25, 40), 50));
        let f = Face {
            x: -10.0,
            width: 20.0,
            ..upright()
        };
        assert_eq!(face_square(&f), ((0, 40), 50));
    }

    #[test]
    fn the_judged_face_is_the_af_face_else_the_largest_confident_one() {
        let at = |x: f32, side: f32, score: f32| Face {
            x,
            y: 0.0,
            width: side,
            height: side,
            score,
            left_eye: (x + side / 4.0, side / 3.0),
            right_eye: (x + side * 3.0 / 4.0, side / 3.0),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        };
        let faces = [
            at(0.0, 100.0, 0.9),
            at(300.0, 200.0, 0.85),
            at(600.0, 300.0, 0.7),
        ];
        assert_eq!(judged_face(&faces, Some((25, 33))), Some(&faces[0]));
        assert_eq!(judged_face(&faces, None), Some(&faces[1]));
        assert_eq!(judged_face(&faces[2..], None), None);
        assert_eq!(judged_face(&[], Some((0, 0))), None);
    }

    /// An eye of width 4 (corners at x 0 and 4) whose lids are `open` apart
    /// at both lid points.
    fn eye(points: &mut [[f32; 3]], i: [usize; 6], x: f32, open: f32) {
        let (top, bottom) = (10.0 - open / 2.0, 10.0 + open / 2.0);
        points[i[0]] = [x, 10.0, 0.5];
        points[i[1]] = [x + 1.0, top, -2.0];
        points[i[2]] = [x + 3.0, top, 1.0];
        points[i[3]] = [x + 4.0, 10.0, 0.0];
        points[i[4]] = [x + 3.0, bottom, 3.0];
        points[i[5]] = [x + 1.0, bottom, -1.0];
    }

    #[test]
    fn the_ear_is_the_lid_gap_over_the_eye_width() {
        let mut points = vec![[0.0; 3]; LANDMARKS];
        eye(&mut points, LEFT_EYE, 0.0, 1.2);
        eye(&mut points, RIGHT_EYE, 20.0, 0.4);
        let (l, r) = ears(&points);
        assert!((l - 0.3).abs() < 1e-6, "{l}");
        assert!((r - 0.1).abs() < 1e-6, "{r}");
    }

    #[test]
    fn the_ear_on_stored_mesh_points_is_the_upright_one() {
        let (w, h) = (6000, 4000);
        let mut points = vec![[0.0; 3]; LANDMARKS];
        eye(&mut points, LEFT_EYE, 1234.5, 0.7);
        eye(&mut points, RIGHT_EYE, 1300.25, 1.3);
        for p in &mut points {
            p[0] += 0.37;
            p[1] += 2210.6;
        }
        let upright = more_closed_ear(&points).unwrap();
        for orientation in [1, 3, 6, 8] {
            let (sw, sh) = match orientation {
                6 | 8 => (h, w),
                _ => (w, h),
            };
            let stored: Vec<[f32; 2]> = points
                .iter()
                .map(|p| {
                    let (x, y) = point_to_stored((p[0], p[1]), orientation, sw, sh);
                    [x, y]
                })
                .collect();
            let ear = more_closed_ear(&stored).unwrap();
            assert!(
                (ear - upright).abs() < 1e-5,
                "{orientation}: {ear} {upright}"
            );
        }
    }

    #[test]
    fn the_logistic_is_one_half_at_the_boundary_and_falls_with_the_ear() {
        assert_eq!(closed_probability(EYES_CLOSED_EAR), 0.5);
        let p = closed_probability(0.037);
        assert!((p - 1.0 / (1.0 + (-2.9f64).exp())).abs() < 1e-12, "{p}");
        assert!(closed_probability(0.05) > closed_probability(0.10));
        assert!(closed_probability(0.20) > closed_probability(0.30));
        assert!(closed_probability(0.0) < 1.0 && closed_probability(1.0) > 0.0);
    }

    #[test]
    fn the_state_and_the_probability_agree_around_the_boundary() {
        for ear in [
            EYES_CLOSED_EAR,
            EYES_CLOSED_EAR.next_down(),
            EYES_CLOSED_EAR - 1e-6,
        ] {
            let e = Eyes::from_ear(ear);
            assert_eq!(e.state, EyeState::Closed, "{ear}");
            assert!(e.probability >= 0.5, "{ear}");
        }
        for ear in [EYES_CLOSED_EAR.next_up(), EYES_CLOSED_EAR + 1e-6] {
            let e = Eyes::from_ear(ear);
            assert_eq!(e.state, EyeState::Open, "{ear}");
            assert!(e.probability < 0.5, "{ear}");
        }
    }

    #[test]
    fn faces_below_the_floor_or_outside_the_image_are_not_judged() {
        let rgb = vec![128u8; 200 * 150 * 3];
        let small = Face {
            width: EYES_MIN_FACE - 1.0,
            height: EYES_MIN_FACE - 2.0,
            ..upright()
        };
        assert_eq!(judge(&rgb, 200, 150, &small), None);
        assert_eq!(judge_mesh(&rgb, 200, 150, &small), None);
        let outside = Face {
            x: 190.0,
            width: 80.0,
            height: 80.0,
            ..upright()
        };
        assert_eq!(ear_of(&rgb, 200, 150, &outside).unwrap(), None);
        assert_eq!(judge(&rgb, 200, 150, &outside), None);
        assert_eq!(landmarks_of(&rgb, 200, 150, &outside).unwrap(), None);
        assert_eq!(ear_of(&rgb[..30], 200, 150, &upright()).unwrap(), None);
    }

    #[test]
    fn the_model_points_are_scaled_from_the_input_onto_the_crop_window() {
        let win = Window {
            x: 100,
            y: 40,
            width: 512,
            height: 128,
        };
        let points = [[0.0, 0.0, 9.0], [128.0, 64.0, 0.0], [256.0, 256.0, -3.0]];
        assert_eq!(
            to_full(&points, &win),
            [
                [100.0, 40.0, 18.0],
                [356.0, 72.0, 0.0],
                [612.0, 168.0, -6.0]
            ]
        );
    }

    #[test]
    fn an_upright_crop_matches_the_crop_of_the_rotated_image() {
        let (w, h) = (7, 5);
        let rgb: Vec<u8> = (0..w * h * 3).map(|i| i as u8).collect();
        for orientation in [1, 3, 6, 8] {
            let (up, uw, uh) = crate::faces::upright_rgb(&rgb, w, h, orientation);
            for win in [
                Window {
                    x: 0,
                    y: 0,
                    width: uw,
                    height: uh,
                },
                Window {
                    x: 1,
                    y: 2,
                    width: 3,
                    height: 2,
                },
            ] {
                let mut want = Vec::new();
                for y in win.y..win.y + win.height {
                    let row = (y * uw + win.x) * 3;
                    want.extend_from_slice(&up[row..row + win.width * 3]);
                }
                assert_eq!(
                    upright_crop(&rgb, w, h, orientation, win),
                    want,
                    "{orientation} {win:?}"
                );
            }
        }
    }

    #[test]
    fn a_mesh_is_none_for_a_face_outside_the_image() {
        let rgb = vec![128u8; 200 * 150 * 3];
        let outside = Face {
            x: 190.0,
            width: 80.0,
            height: 80.0,
            ..upright()
        };
        assert_eq!(mesh_of(&rgb, 200, 150, 1, &outside), None);
        assert_eq!(mesh_of(&rgb, 150, 200, 6, &outside), None);
        assert_eq!(mesh_of(&rgb[..30], 200, 150, 1, &upright()), None);
    }

    #[test]
    fn the_contours_and_irises_are_distinct_mesh_points() {
        let all: Vec<usize> = [
            &LEFT_EYE_CONTOUR[..],
            &RIGHT_EYE_CONTOUR,
            &LEFT_IRIS,
            &RIGHT_IRIS,
        ]
        .concat();
        let mut sorted = all.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len());
        assert!(all.iter().all(|&i| i < LANDMARKS));
        assert!(LEFT_EYE.iter().all(|i| LEFT_EYE_CONTOUR.contains(i)));
        assert!(RIGHT_EYE.iter().all(|i| RIGHT_EYE_CONTOUR.contains(i)));
    }

    #[test]
    fn a_resize_interpolates_between_pixel_centers() {
        let rgb = [0, 0, 0, 100, 200, 40];
        assert_eq!(
            resize(&rgb, 2, 1, 4, 1),
            [
                [0.0, 0.0, 0.0],
                [25.0, 50.0, 10.0],
                [75.0, 150.0, 30.0],
                [100.0, 200.0, 40.0]
            ]
        );
        assert_eq!(resize(&rgb, 2, 1, 1, 1), [[50.0, 100.0, 20.0]]);
        assert_eq!(resize(&rgb, 2, 1, 2, 1), [[0.0; 3], [100.0, 200.0, 40.0]]);
    }

    #[test]
    fn the_input_is_rgb_nhwc_from_zero_to_one() {
        let rgb: Vec<u8> = (0..4).flat_map(|_| [255, 51, 0]).collect();
        let t = input_tensor(&rgb, 2, 2);
        assert_eq!(t.shape(), &[1, INPUT, INPUT, 3]);
        let v = t.try_as_plain_ram().unwrap().as_slice::<f32>().unwrap();
        assert_eq!(v[..3], [1.0, 0.2, 0.0]);
        assert_eq!(v[v.len() - 3..], [1.0, 0.2, 0.0]);
    }

    /// Set `RIFFLE_FACE_JPEG` to an upright JPEG with one clear face, then
    /// run with `--ignored`.
    #[test]
    #[ignore]
    fn judges_a_face_in_a_real_image() {
        let path = std::env::var("RIFFLE_FACE_JPEG").expect("RIFFLE_FACE_JPEG");
        let jpeg = std::fs::read(path).unwrap();
        let (rgb, w, h) = crate::decode::decode_rgb(&jpeg).unwrap();
        let face = crate::faces::detect(&rgb, w, h).unwrap()[0];
        let (center, side) = face_square(&face);
        let (sub, win) = crop_rgb(&rgb, w, h, center, side);
        let points = landmarks(input_tensor(&sub, win.width, win.height)).unwrap();
        assert_eq!(points.len(), LANDMARKS);
        for p in &points {
            assert!((0.0..=INPUT as f32).contains(&p[0]), "{p:?}");
            assert!((0.0..=INPUT as f32).contains(&p[1]), "{p:?}");
        }
        let ear = ear_of(&rgb, w, h, &face).unwrap().unwrap();
        let p = Eyes::from_ear(ear).probability;
        assert!((0.0..=1.0).contains(&p), "{p}");
        let judged = judge_mesh(&rgb, w, h, &face).unwrap();
        println!("ear {ear:.4} pose {:?}", judged.pose);
        // Stored as orientation 6, the upright image turned a quarter back.
        let (stored, sw, sh) = crate::decode::apply_orientation(&rgb, w, h, 8);
        let stored_face = crate::faces::to_stored(face, 6, sw, sh);
        let mesh = mesh_of(&stored, sw, sh, 6, &stored_face).unwrap();
        assert_eq!(mesh.pose, judged.pose);
        let judged_ear = more_closed_ear(&judged.points).unwrap();
        assert_eq!(judged.eyes, Eyes::from_ear(judged_ear));
        let stored_ear = more_closed_ear(&mesh.points).unwrap();
        assert!(
            (stored_ear - judged_ear).abs() < 1e-5,
            "{stored_ear} {judged_ear}"
        );
        for (m, p) in mesh.points.iter().zip(&judged.points) {
            let (x, y) = point_to_stored((p[0], p[1]), 6, sw, sh);
            assert!(
                (m[0] - x).abs() < 1e-3 && (m[1] - y).abs() < 1e-3,
                "{m:?} {p:?}"
            );
        }
    }
}
