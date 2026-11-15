//! The face cue of one preview: the face nearest the AF point, its face mesh,
//! and what the mesh gives (the EAR, the head pose, the eye offset, the edge
//! gap and the eye centers `fullsharp` measures around).
//!
//! The mesh (`eyes::mesh_of`) runs only on a face whose box long side is at
//! least `eyes::EYES_MIN_FACE` (`meshes_face`); a smaller face, or one the mesh
//! fails on, keeps only its YuNet eye landmarks as its eye centers.
//!
//! Faces come from `faces::detect_around_rgb` in a `CATCH_CROP` square around
//! the trusted AF point. A Sony eye-AF frame gets the same detection: the
//! camera tracking a face does not say whether that face is sharp. Without a
//! trusted AF point, `scan` judges the largest confident face of the whole
//! preview (`eyes::judged_face`) through `face_cue_unless`.
//! Everything is in the preview's stored (unrotated) coordinates.

use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;

use crate::arw::FocusLocation;
use crate::decode::decode_rgb;
use crate::eyes::{
    mesh_of, more_closed_ear, Mesh, EYES_MIN_FACE, LEFT_EYE_CONTOUR, RIGHT_EYE_CONTOUR,
};
use crate::faces::{detect_around_rgb, Detection, Face};
use crate::pose::Pose;

/// The margin `edge_gap` grows a mesh eye's bounding box by on each side, as
/// a fraction of the box's longer side (the eye's width, upright or on a
/// quarter-turned preview), so the lid edges and lashes lie inside
/// (`docs/plans/20261008-mesh-eye-focus/fit.md`).
pub const EYE_REGION_MARGIN: f32 = 0.5;

/// The cue of one preview.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cue {
    /// The judged face, in stored coordinates: the one nearest the AF point,
    /// else the largest confident face of the whole preview.
    pub face: Option<Face>,
    /// The detection the face was picked from; `None` without an AF point
    /// when no face was judged, and in the `Cue::unknown` a caller falls back
    /// to after a failure.
    pub detection: Option<Detection>,
    /// The EAR of the more closed eye of `face` (`eyes::more_closed_ear`), the
    /// closed-eyes judgment's input; `None` without a mesh.
    pub eyes_ear: Option<f64>,
    /// The head pose of `face`; `None` without a mesh or a solve.
    pub pose: Option<Pose>,
    /// How far the mesh's eyes sit from `face`'s YuNet eye landmarks
    /// (`mesh_eye_offset`); `None` without a mesh.
    pub eye_offset: Option<f64>,
    /// How close `face` and its mesh eye regions come to the preview's edge
    /// (`edge_gap`); `None` without a mesh.
    pub edge_gap: Option<f64>,
    /// The two eye centers of `face` in stored coordinates: the mesh's
    /// (`mesh_eye_centers`), else YuNet's landmarks; `None` when no face was
    /// judged.
    pub eye_centers: Option<[(f32, f32); 2]>,
    /// The stored width of the preview the cue was taken on; 0 in a
    /// `Cue::unknown`.
    pub preview_width: usize,
}

impl Cue {
    pub fn unknown() -> Self {
        Self::default()
    }
}

/// The face in `faces` whose closer eye is nearest `point`, the distance
/// divided by the face box's long side so a large face does not win on size
/// alone. Faces whose long side is not positive are skipped.
pub fn nearest_face(faces: &[Face], point: (usize, usize)) -> Option<&Face> {
    let (px, py) = (point.0 as f32, point.1 as f32);
    faces
        .iter()
        .filter(|f| f.width.max(f.height) > 0.0)
        .map(|f| {
            let eye = |(x, y): (f32, f32)| (x - px).hypot(y - py);
            let d = eye(f.left_eye).min(eye(f.right_eye)) / f.width.max(f.height);
            (f, d)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(f, _)| f)
}

/// The luma `(299 R + 587 G + 114 B) / 1000` of each pixel of a
/// `width` x `height` RGB image.
pub fn luma(rgb: &[u8], width: usize, height: usize) -> Vec<u8> {
    rgb.chunks_exact(3)
        .take(width * height)
        .map(|p| ((299 * p[0] as u32 + 587 * p[1] as u32 + 114 * p[2] as u32) / 1000) as u8)
        .collect()
}

/// The bounding box `[left, top, right, bottom]` of the mesh `points` named
/// by `indices`, grown by `EYE_REGION_MARGIN` times its longer side on each
/// side; `None` when an index is out of range or a point is not finite.
fn grown_bounds(points: &[[f32; 2]], indices: &[usize]) -> Option<[f32; 4]> {
    let (mut x0, mut y0) = (f32::INFINITY, f32::INFINITY);
    let (mut x1, mut y1) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for &i in indices {
        let [x, y] = *points.get(i)?;
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    let margin = (x1 - x0).max(y1 - y0) * EYE_REGION_MARGIN;
    Some([x0 - margin, y0 - margin, x1 + margin, y1 + margin])
}

/// The centers (the mean of the points) of `LEFT_EYE_CONTOUR` and
/// `RIGHT_EYE_CONTOUR`, in that order. `None` when a point is missing or not
/// finite.
pub fn mesh_eye_centers(points: &[[f32; 2]]) -> Option<[(f32, f32); 2]> {
    let center = |indices: &[usize]| -> Option<(f32, f32)> {
        let mut sum = (0.0, 0.0);
        for &i in indices {
            let [x, y] = *points.get(i)?;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            sum = (sum.0 + x, sum.1 + y);
        }
        let n = indices.len() as f32;
        Some((sum.0 / n, sum.1 / n))
    };
    Some([center(&LEFT_EYE_CONTOUR)?, center(&RIGHT_EYE_CONTOUR)?])
}

/// How far the mesh's eyes sit from YuNet's eye landmarks of the same
/// `face`: the larger of the two distances between an eyelid contour's
/// center (the mean of its points) and its landmark, in the pairing of the
/// two eyes with the landmarks that keeps that larger distance smaller,
/// divided by the face box's longer side. A mesh fitted off the face (an
/// in-plane rotation the upright crop does not undo) reads far. `None` when
/// a contour point is not finite or the box is empty.
pub fn mesh_eye_offset(face: &Face, points: &[[f32; 2]]) -> Option<f64> {
    let [left, right] = mesh_eye_centers(points)?;
    let side = face.width.max(face.height);
    if side <= 0.0 {
        return None;
    }
    let d = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).hypot(a.1 - b.1);
    let straight = d(left, face.left_eye).max(d(right, face.right_eye));
    let crossed = d(left, face.right_eye).max(d(right, face.left_eye));
    Some(f64::from(straight.min(crossed) / side))
}

/// How close `face` comes to the edge of the `width` x `height` image: the
/// smallest distance from its box, or from either eyelid contour region of
/// its mesh `points` (grown by `grown_bounds`), to
/// an image edge, divided by the box's longer side; negative when one
/// extends outside. YuNet's box covers only the visible part of a face the
/// frame's edge cuts, so such a box ends at (about) the edge. An eye whose
/// region cannot be built is left out. `None` when the box is empty.
pub fn edge_gap(face: &Face, points: &[[f32; 2]], width: usize, height: usize) -> Option<f64> {
    let side = face.width.max(face.height);
    if side <= 0.0 {
        return None;
    }
    let (w, h) = (width as f32, height as f32);
    let gap = |[x0, y0, x1, y1]: [f32; 4]| x0.min(y0).min(w - x1).min(h - y1);
    let gap = [&LEFT_EYE_CONTOUR[..], &RIGHT_EYE_CONTOUR[..]]
        .iter()
        .filter_map(|indices| grown_bounds(points, indices))
        .map(gap)
        .fold(
            gap([face.x, face.y, face.x + face.width, face.y + face.height]),
            f32::min,
        );
    Some(f64::from(gap / side))
}

/// Whether the cue runs the face mesh on `face`: its box long side is at
/// least `EYES_MIN_FACE`, the floor the closed-eyes judgment uses.
pub fn meshes_face(face: &Face) -> bool {
    face.width.max(face.height) >= EYES_MIN_FACE
}

/// The mesh of `face`, when `meshes_face` accepts it and the model fits one;
/// `None` (the outer one) once `cancel` is set while the mesh runs.
fn face_mesh(
    rgb: &[u8],
    width: usize,
    height: usize,
    orientation: u16,
    face: &Face,
    cancel: &AtomicBool,
) -> Option<Option<Mesh>> {
    let mesh = meshes_face(face)
        .then(|| mesh_of(rgb, width, height, orientation, face))
        .flatten();
    crate::scan::yield_to_pause(cancel);
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    Some(mesh)
}

/// The cue of `preview` for the AF point of `focus` (pass
/// `sharpness::trusted_focus`). Without one it is `Cue::unknown`, with no decode
/// and no detection; otherwise the preview is decoded once, faces are
/// detected around the point and the nearest one is meshed.
pub fn focus_cue(preview: &[u8], orientation: u16, focus: Option<FocusLocation>) -> Result<Cue> {
    focus_cue_unless(preview, orientation, focus, &AtomicBool::new(false))
        .expect("a never-set flag never abandons")
}

/// `focus_cue`, abandoned (`None`) once `cancel` is set between the decode,
/// the detection and the face mesh. The mesh runs only on a face
/// `meshes_face` accepts. The mesh model's plan is built once per process
/// (`eyes`' `OnceLock`), so the first meshed file of a scan pays about 180 ms
/// more.
pub fn focus_cue_unless(
    preview: &[u8],
    orientation: u16,
    focus: Option<FocusLocation>,
    cancel: &AtomicBool,
) -> Option<Result<Cue>> {
    if focus.is_none() {
        return Some(Ok(Cue::unknown()));
    }
    let canceled = || cancel.load(Ordering::Relaxed);
    let (rgb, width, height) = match decode_rgb(preview) {
        Ok(decoded) => decoded,
        Err(e) => return Some(Err(e)),
    };
    crate::scan::yield_to_pause(cancel);
    if canceled() {
        return None;
    }
    let detection = match detect_around_rgb(&rgb, width, height, orientation, focus) {
        Ok(detection) => detection,
        Err(e) => return Some(Err(e)),
    };
    let face = detection
        .point
        .and_then(|p| nearest_face(&detection.faces, p))
        .copied();
    crate::scan::yield_to_pause(cancel);
    if canceled() {
        return None;
    }
    match face {
        Some(f) => face_cue_unless(&rgb, width, height, orientation, &f, detection, cancel).map(Ok),
        None => Some(Ok(cue_of(None, detection, None, width, height))),
    }
}

/// The cue of `face`, picked from `detection`, on the full-size decode `rgb`
/// (`width` x `height`, stored coordinates): the face mesh when
/// `meshes_face` accepts the face. `None` once `cancel` is set while the mesh
/// runs.
pub fn face_cue_unless(
    rgb: &[u8],
    width: usize,
    height: usize,
    orientation: u16,
    face: &Face,
    detection: Detection,
    cancel: &AtomicBool,
) -> Option<Cue> {
    let mesh = face_mesh(rgb, width, height, orientation, face, cancel)?;
    Some(cue_of(Some(*face), detection, mesh.as_ref(), width, height))
}

/// The cue of `face`, picked from `detection`, with its `mesh` on the
/// `width` x `height` image; the EAR, the pose, the eye offset and the edge
/// gap come from the mesh, so a face without one has none of them and its eye
/// centers are `face`'s landmarks.
fn cue_of(
    face: Option<Face>,
    detection: Detection,
    mesh: Option<&Mesh>,
    width: usize,
    height: usize,
) -> Cue {
    let preview_width = detection.width;
    let measured = face.zip(mesh);
    Cue {
        face,
        detection: Some(detection),
        eyes_ear: mesh.and_then(|m| more_closed_ear(&m.points)),
        pose: mesh.and_then(|m| m.pose),
        eye_offset: measured.and_then(|(f, m)| mesh_eye_offset(&f, &m.points)),
        edge_gap: measured.and_then(|(f, m)| edge_gap(&f, &m.points, width, height)),
        eye_centers: face.map(|f| {
            mesh.and_then(|m| mesh_eye_centers(&m.points))
                .unwrap_or([f.left_eye, f.right_eye])
        }),
        preview_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(x: f32, y: f32, side: f32, left_eye: (f32, f32), right_eye: (f32, f32)) -> Face {
        Face {
            x,
            y,
            width: side,
            height: side,
            score: 0.9,
            left_eye,
            right_eye,
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        }
    }

    #[test]
    fn the_nearest_face_is_chosen_by_eye_distance_not_box_center() {
        // The point sits on the eye of `a` but closer to the box center of `b`.
        let a = face(0.0, 0.0, 100.0, (20.0, 20.0), (80.0, 20.0));
        let b = face(40.0, -30.0, 100.0, (60.0, 60.0), (120.0, 60.0));
        let faces = [b, a];
        assert_eq!(nearest_face(&faces, (80, 22)), Some(&a));
    }

    #[test]
    fn the_eye_distance_is_relative_to_the_face_size() {
        // Large: eye 50 px away on a 200 px face (0.25). Small: eye 10 px away
        // on a 20 px face (0.5).
        let large = face(0.0, 0.0, 200.0, (150.0, 100.0), (170.0, 100.0));
        let small = face(0.0, 0.0, 20.0, (110.0, 100.0), (115.0, 100.0));
        assert_eq!(nearest_face(&[small, large], (100, 100)), Some(&large));
        // Small eye 4 px away (0.2) now beats the large one.
        let small = face(0.0, 0.0, 20.0, (104.0, 100.0), (115.0, 100.0));
        assert_eq!(nearest_face(&[large, small], (100, 100)), Some(&small));
    }

    #[test]
    fn no_faces_or_empty_boxes_give_none() {
        assert_eq!(nearest_face(&[], (0, 0)), None);
        let empty = face(0.0, 0.0, 0.0, (0.0, 0.0), (0.0, 0.0));
        assert_eq!(nearest_face(&[empty], (0, 0)), None);
    }

    #[test]
    fn luma_uses_the_bt601_integer_weights() {
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        assert_eq!(luma(&rgb, 4, 1), vec![76, 149, 29, 255]);
    }

    /// A 120 x 60 image with a vertical step edge at column 20 blurred over
    /// `left` px and one at column 80 blurred over `right` px.
    fn two_steps(left: usize, right: usize) -> Vec<u8> {
        let step =
            |x: usize, at: usize, blur: usize| (x.saturating_sub(at).min(blur) * 200 / blur) as u8;
        let row: Vec<u8> = (0..120)
            .map(|x| {
                if x < 60 {
                    step(x, 20, left)
                } else {
                    step(x, 80, right)
                }
            })
            .collect();
        row.repeat(60)
    }

    /// A mesh whose eye contours span boxes `(x, y, width, height)` (`None`
    /// leaves that eye's points NaN), with `pose`.
    fn boxed_eyes(
        left: Option<(f32, f32, f32, f32)>,
        right: Option<(f32, f32, f32, f32)>,
        pose: Option<Pose>,
    ) -> Mesh {
        let mut points = vec![[f32::NAN; 2]; crate::eyes::LANDMARKS];
        for (contour, eye) in [(&LEFT_EYE_CONTOUR, left), (&RIGHT_EYE_CONTOUR, right)] {
            let Some((x, y, w, h)) = eye else { continue };
            for (k, &i) in contour.iter().enumerate() {
                points[i] = [x + (k % 2) as f32 * w, y + (k / 2 % 2) as f32 * h];
            }
        }
        Mesh { points, pose }
    }

    /// Both eyes 20 x 10, their regions 40 x 30 around the two step edges.
    fn both_eyes(pose: Option<Pose>) -> Mesh {
        boxed_eyes(
            Some((10.0, 20.0, 20.0, 10.0)),
            Some((70.0, 20.0, 20.0, 10.0)),
            pose,
        )
    }

    fn two_eyed() -> Face {
        face(0.0, 0.0, 60.0, (20.0, 25.0), (80.0, 25.0))
    }

    #[test]
    fn no_af_point_is_unknown_without_decoding() {
        assert_eq!(focus_cue(&[0u8; 16], 1, None).unwrap(), Cue::unknown());
    }

    #[test]
    fn a_set_flag_abandons_the_cue_after_the_decode() {
        let rgb = vec![128u8; 64 * 48 * 3];
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        c.set_size(64, 48);
        c.set_quality(80.0);
        let mut c = c.start_compress(Vec::new()).unwrap();
        c.write_scanlines(&rgb).unwrap();
        let jpeg = c.finish().unwrap();
        let focus = Some(FocusLocation {
            sensor_w: 6000,
            sensor_h: 4000,
            x: 3000,
            y: 2000,
        });
        let set = AtomicBool::new(true);
        assert!(focus_cue_unless(&jpeg, 1, focus, &set).is_none());
        assert!(
            focus_cue_unless(&[0u8; 16], 1, focus, &set).is_some_and(|r| r.is_err()),
            "a decode failure is still an error"
        );
        assert!(focus_cue_unless(&jpeg, 1, focus, &AtomicBool::new(false)).is_some());
    }

    #[test]
    fn a_set_flag_abandons_the_cue_after_the_mesh() {
        let gray = two_steps(2, 8);
        let rgb: Vec<u8> = gray.iter().flat_map(|&g| [g, g, g]).collect();
        // Outside the image: no mesh, no model run.
        let away = face(500.0, 500.0, 60.0, (520.0, 525.0), (580.0, 525.0));
        let set = AtomicBool::new(true);
        assert_eq!(face_mesh(&rgb, 120, 60, 1, &away, &set), None);
        assert_eq!(
            face_mesh(&rgb, 120, 60, 1, &away, &AtomicBool::new(false)),
            Some(None)
        );
    }

    #[test]
    fn a_given_face_gets_its_cue_unless_canceled() {
        let gray = two_steps(2, 8);
        let rgb: Vec<u8> = gray.iter().flat_map(|&g| [g, g, g]).collect();
        // Outside the image: no mesh, no model run.
        let away = face(500.0, 500.0, 60.0, (520.0, 525.0), (580.0, 525.0));
        let detection = Detection {
            width: 120,
            height: 60,
            faces: vec![away],
            point: None,
        };
        let set = AtomicBool::new(true);
        assert_eq!(
            face_cue_unless(&rgb, 120, 60, 1, &away, detection.clone(), &set),
            None
        );
        let never = AtomicBool::new(false);
        let cue = face_cue_unless(&rgb, 120, 60, 1, &away, detection.clone(), &never).unwrap();
        assert_eq!(cue, cue_of(Some(away), detection, None, 120, 60));
        assert_eq!(cue.face, Some(away));
        assert_eq!(cue.eye_centers, Some([away.left_eye, away.right_eye]));
    }

    #[test]
    fn the_cue_carries_the_ear_and_the_pose_of_its_mesh() {
        let pose = Pose {
            yaw: -20.0,
            pitch: 5.0,
            roll: 1.0,
        };
        let mut mesh = both_eyes(Some(pose));
        // The far corner of the left eye, inside its box, so its EAR is finite.
        mesh.points[133] = [30.0, 20.0];
        let ear = more_closed_ear(&mesh.points);
        assert!(ear.is_some());
        let detection = Detection {
            width: 120,
            height: 60,
            faces: vec![two_eyed()],
            point: Some((50, 25)),
        };
        let cue = cue_of(Some(two_eyed()), detection, Some(&mesh), 120, 60);
        assert_eq!((cue.eyes_ear, cue.pose), (ear, Some(pose)));
        assert_eq!(
            (cue.eye_offset, cue.edge_gap),
            (
                mesh_eye_offset(&two_eyed(), &mesh.points),
                edge_gap(&two_eyed(), &mesh.points, 120, 60)
            )
        );
        assert!(cue.eye_offset.is_some() && cue.edge_gap.is_some());
        assert_eq!(cue.eye_centers, Some([(21.25, 25.0), (80.0, 25.0)]));
        assert_eq!(cue.preview_width, 120);
    }

    #[test]
    fn the_eye_offset_is_the_farther_eye_in_the_closer_pairing_over_the_face_side() {
        // The contour centers are (20, 25) and (80, 25), on the landmarks.
        let mesh = both_eyes(None);
        assert_eq!(mesh_eye_offset(&two_eyed(), &mesh.points), Some(0.0));
        let off = face(0.0, 0.0, 60.0, (17.0, 21.0), (80.0, 31.0));
        assert_eq!(
            mesh_eye_offset(&off, &mesh.points),
            Some(f64::from(6.0f32 / 60.0))
        );
        let swapped = face(0.0, 0.0, 60.0, (80.0, 25.0), (20.0, 25.0));
        assert_eq!(mesh_eye_offset(&swapped, &mesh.points), Some(0.0));
        let one_eyed = boxed_eyes(Some((10.0, 20.0, 20.0, 10.0)), None, None);
        assert_eq!(mesh_eye_offset(&two_eyed(), &one_eyed.points), None);
        let empty = face(0.0, 0.0, 0.0, (20.0, 25.0), (80.0, 25.0));
        assert_eq!(mesh_eye_offset(&empty, &mesh.points), None);
    }

    #[test]
    fn the_edge_gap_is_the_nearest_of_the_box_and_the_grown_eye_regions_to_an_edge() {
        let side = 40.0f32;
        let inside = face(50.0, 20.0, side, (65.0, 37.0), (80.0, 37.0));
        // Grown by 5 px: the eye regions span y 30-45, the box y 20-60.
        let mesh = boxed_eyes(
            Some((60.0, 35.0, 10.0, 5.0)),
            Some((75.0, 35.0, 10.0, 5.0)),
            None,
        );
        assert_eq!(
            edge_gap(&inside, &mesh.points, 200, 100),
            Some(f64::from(20.0 / side))
        );
        let cut = Face { x: -4.0, ..inside };
        assert_eq!(
            edge_gap(&cut, &mesh.points, 200, 100),
            Some(f64::from(-4.0 / side))
        );
        let high = boxed_eyes(Some((60.0, 2.0, 10.0, 5.0)), None, None);
        assert_eq!(
            edge_gap(&inside, &high.points, 200, 100),
            Some(f64::from(-3.0 / side)),
            "an eye region past the top while the box is inside"
        );
        let none = boxed_eyes(None, None, None);
        assert_eq!(
            edge_gap(&inside, &none.points, 200, 100),
            Some(f64::from(20.0 / side))
        );
        let empty = Face {
            width: 0.0,
            height: 0.0,
            ..inside
        };
        assert_eq!(edge_gap(&empty, &mesh.points, 200, 100), None);
    }
    #[test]
    fn a_face_under_the_floor_gets_no_mesh_and_keeps_its_landmarks() {
        let gray = two_steps(2, 8);
        let rgb: Vec<u8> = gray.iter().flat_map(|&g| [g, g, g]).collect();
        // Inside the image, so only the gate keeps the model from running.
        let small = face(20.0, 0.0, 59.0, (35.0, 25.0), (65.0, 25.0));
        assert!(!meshes_face(&small));
        assert_eq!(
            face_mesh(&rgb, 120, 60, 1, &small, &AtomicBool::new(false)),
            Some(None)
        );
        let detection = Detection {
            width: 120,
            height: 60,
            faces: vec![small],
            point: Some((50, 25)),
        };
        let cue = cue_of(Some(small), detection, None, 120, 60);
        assert_eq!((cue.eyes_ear, cue.pose), (None, None));
        assert_eq!((cue.eye_offset, cue.edge_gap), (None, None));
        assert_eq!(cue.eye_centers, Some([small.left_eye, small.right_eye]));
        assert_eq!(cue.preview_width, 120);
        let side = |s: f32| face(0.0, 0.0, s, (20.0, 25.0), (40.0, 25.0));
        assert!(!meshes_face(&side(EYES_MIN_FACE.next_down())));
        assert!(meshes_face(&side(EYES_MIN_FACE)));
        let wide = Face {
            height: 10.0,
            ..side(EYES_MIN_FACE)
        };
        assert!(meshes_face(&wide));
    }
}
