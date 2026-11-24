//! The all-faces check of one frame: besides the face the cue judges, every
//! other confident face of the whole-image detection that is large enough is
//! meshed and measured the same way, and the frame gets the counts of the
//! faces judged, of those whose eyes are closed and of those whose eyes are
//! clearly soft on the full JPEG. `scan::extract_face_counts` runs it on one
//! file, outside pass 2; the caller runs it only on group photos, the frames
//! whose gate count (`gate_count`, which pass 2 stores) reaches its threshold.
//!
//! The gate count is the judged face and every confident face of a whole
//! preview detection whose box is of a similar size (`GATE_MIN_SIZE` to
//! `GATE_MAX_SIZE` of the judged face's long side): a face at half the linear
//! size stands about twice as far, a passer-by or a spectator, not a member of
//! the group. Confident is `FACE_CONFIDENCE` on the 640x448 search of a frame
//! without an AF point and `GATE_LOW_RES_CONFIDENCE` on the low-resolution
//! search of a frame with one.
//!
//! A candidate scores at least `sharpness::FACE_CONFIDENCE`, has a box long
//! side of at least `eyes::EYES_MIN_FACE` (`candidate::meshes_face`) and does
//! not overlap the judged face (an IoU above `faces::NMS_THRESHOLD`), which is
//! counted from the cue's own mesh instead. The largest come first, at most
//! `MAX_FACES` faces per frame with the judged one. A candidate counts only
//! when its mesh fits the face (`MAX_EYE_OFFSET`); the judged face always
//! counts when the cue has its EAR. See the all-faces-check plan's
//! `measurement.md`.
//! Everything is in the preview's stored (unrotated) coordinates.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::candidate::{mesh_eye_centers, mesh_eye_offset, meshes_face};
use crate::eyes::{mesh_of, more_closed_ear, EyeState, Eyes, Mesh};
use crate::faces::{iou, Face, NMS_THRESHOLD};
use crate::fullsharp::LOW_BELOW;
use crate::sharpness::FACE_CONFIDENCE;

/// The most faces judged per frame, the judged face included. No frame of
/// the measured folders had more than 4 candidates.
pub const MAX_FACES: usize = 8;
/// A candidate's mesh whose eyes sit farther than this from its YuNet eye
/// landmarks, in face box sides (`candidate::mesh_eye_offset`), is fitted
/// off the face and the face is not counted; the app's `MAX_EYE_OFFSET` cut.
pub const MAX_EYE_OFFSET: f64 = 0.1;
/// The smallest box long side, over the judged face's, a face counts toward
/// the gate with.
pub const GATE_MIN_SIZE: f32 = 0.5;
/// The largest box long side, over the judged face's, a face counts toward
/// the gate with.
pub const GATE_MAX_SIZE: f32 = 2.0;
/// The score floor of a face counted toward the gate on the low-resolution
/// `faces::GATE_INPUT` search, under `FACE_CONFIDENCE`: at 320x224 the small
/// faces score 0.6 to 0.8, and at this floor the search counts about as many
/// faces as the 640x448 one does at `FACE_CONFIDENCE` (see the
/// all-faces-check plan's `measurement.md`).
pub const GATE_LOW_RES_CONFIDENCE: f32 = 0.6;

/// The counts of one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FaceCounts {
    /// The faces judged: the cue's face when it has an EAR, and every
    /// candidate whose mesh fits.
    pub judged: u32,
    /// Those whose more closed eye `Eyes::from_ear` reads as closed.
    pub closed: u32,
    /// Those whose full-size eye sharpness is under `fullsharp::LOW_BELOW`;
    /// `None` when the file has no larger JPEG or its read failed.
    pub soft: Option<u32>,
}

/// One judged face: the EAR of its more closed eye and its full-size eye
/// sharpness, `None` when it could not be measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub ear: f64,
    pub full_eye_sharp: Option<f64>,
}

/// A candidate measured on the preview.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measured {
    pub ear: f64,
    /// `candidate::mesh_eye_centers` of its mesh, where its full-size eye
    /// sharpness is taken.
    pub eye_centers: [(f32, f32); 2],
}

/// The gate count of a frame whose cue judged the face `judged`, from the
/// `faces` of a whole-preview detection in the same stored coordinates: the
/// judged face, and every face scoring at least `floor` whose box long
/// side is within `GATE_MIN_SIZE` to `GATE_MAX_SIZE` of the judged face's and
/// that does not overlap it by more than `NMS_THRESHOLD` (the judged face
/// found again). 0 without a judged face.
pub fn gate_count(faces: &[Face], judged: Option<&Face>, floor: f32) -> u32 {
    let Some(judged) = judged else {
        return 0;
    };
    let side = long_side(judged);
    let band = GATE_MIN_SIZE * side..=GATE_MAX_SIZE * side;
    let similar = faces
        .iter()
        .filter(|f| f.score >= floor && band.contains(&long_side(f)))
        .filter(|f| iou(f, judged) <= NMS_THRESHOLD)
        .count();
    1 + similar as u32
}

fn long_side(face: &Face) -> f32 {
    face.width.max(face.height)
}

/// The candidates among a whole-image detection's `faces`: at or above
/// `FACE_CONFIDENCE`, accepted by `meshes_face`, not overlapping `judged` by
/// more than `NMS_THRESHOLD`, the largest box first, at most `limit`.
pub fn candidates(faces: &[Face], judged: Option<&Face>, limit: usize) -> Vec<Face> {
    let mut found: Vec<Face> = faces
        .iter()
        .filter(|f| f.score >= FACE_CONFIDENCE && meshes_face(f))
        .filter(|f| judged.is_none_or(|j| iou(f, j) <= NMS_THRESHOLD))
        .copied()
        .collect();
    found.sort_by(|a, b| (b.width * b.height).total_cmp(&(a.width * a.height)));
    found.truncate(limit);
    found
}

/// What `mesh` gives of `face`, when the mesh fits it (`MAX_EYE_OFFSET`) and
/// both its EAR and its eye centers are finite.
pub fn measured(face: &Face, mesh: &Mesh) -> Option<Measured> {
    if mesh_eye_offset(face, &mesh.points)? > MAX_EYE_OFFSET {
        return None;
    }
    Some(Measured {
        ear: more_closed_ear(&mesh.points)?,
        eye_centers: mesh_eye_centers(&mesh.points)?,
    })
}

/// Each of `faces` meshed on the stored `width` x `height` RGB image `rgb` and
/// `measured`; a face without a mesh, whose mesh does not fit or that panics
/// is left out. `None` once `cancel` is set, checked after each face.
pub fn measure_unless(
    rgb: &[u8],
    width: usize,
    height: usize,
    orientation: u16,
    faces: &[Face],
    cancel: &AtomicBool,
) -> Option<Vec<Measured>> {
    let mut out = Vec::new();
    for face in faces {
        let m = catch_unwind(AssertUnwindSafe(|| {
            mesh_of(rgb, width, height, orientation, face).and_then(|mesh| measured(face, &mesh))
        }))
        .ok()
        .flatten();
        crate::scan::yield_to_pause(cancel);
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        out.extend(m);
    }
    Some(out)
}

/// The counts of the judged faces' `readings`; `soft` only when the full JPEG
/// was read (`full_read`).
pub fn counts(readings: &[Reading], full_read: bool) -> FaceCounts {
    let count = |p: &dyn Fn(&Reading) -> bool| readings.iter().filter(|r| p(r)).count() as u32;
    FaceCounts {
        judged: readings.len() as u32,
        closed: count(&|r| Eyes::from_ear(r.ear).state == EyeState::Closed),
        soft: full_read.then(|| count(&|r| r.full_eye_sharp.is_some_and(|s| s < LOW_BELOW))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eyes::{
        EYES_CLOSED_EAR, EYES_MIN_FACE, LANDMARKS, LEFT_EYE_CONTOUR, RIGHT_EYE_CONTOUR,
    };

    fn face(x: f32, side: f32, score: f32) -> Face {
        Face {
            x,
            y: 0.0,
            width: side,
            height: side,
            score,
            left_eye: (x + side * 0.3, side * 0.4),
            right_eye: (x + side * 0.7, side * 0.4),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        }
    }

    #[test]
    fn a_candidate_is_confident_large_enough_and_not_the_judged_face() {
        let shy = face(0.0, 100.0, FACE_CONFIDENCE.next_down());
        let small = face(200.0, EYES_MIN_FACE.next_down(), 0.95);
        let floor = face(400.0, EYES_MIN_FACE, FACE_CONFIDENCE);
        let judged = face(600.0, 100.0, 0.9);
        // The same face as detected on the crop: shifted, IoU well above 0.3.
        let crop = face(610.0, 95.0, 0.85);
        let near = face(680.0, 100.0, 0.9);
        let all = [shy, small, floor, judged, near];
        assert_eq!(candidates(&all, Some(&crop), MAX_FACES), [near, floor]);
        assert_eq!(candidates(&all, Some(&judged), MAX_FACES), [near, floor]);
        assert_eq!(
            candidates(&all, None, MAX_FACES),
            [judged, near, floor],
            "without a judged face every confident large face is a candidate"
        );
        assert!(iou(&near, &judged) <= NMS_THRESHOLD);
    }

    #[test]
    fn the_gate_counts_the_judged_face_and_the_confident_faces_of_a_similar_size() {
        let judged = face(0.0, 100.0, 0.9);
        let count = |faces: &[Face], judged| gate_count(faces, judged, FACE_CONFIDENCE);
        assert_eq!(count(&[judged], None), 0, "no judged face, no group");
        assert_eq!(count(&[], None), 0);
        assert_eq!(
            count(&[], Some(&judged)),
            1,
            "the judged face counts though the detection missed it"
        );
        assert_eq!(
            count(&[judged], Some(&judged)),
            1,
            "the judged face found again counts once"
        );
        // The same face as detected on the crop: shifted, IoU above 0.3.
        let again = face(8.0, 96.0, 0.85);
        assert!(iou(&again, &judged) > NMS_THRESHOLD);
        assert_eq!(count(&[again], Some(&judged)), 1);
        let min = face(200.0, 50.0, 0.9);
        let max = face(400.0, 200.0, 0.9);
        let under = face(700.0, 50.0f32.next_down(), 0.9);
        let over = face(900.0, 200.0f32.next_up(), 0.9);
        let shy = face(1200.0, 100.0, FACE_CONFIDENCE.next_down());
        let sure = face(1400.0, 100.0, FACE_CONFIDENCE);
        assert_eq!(
            count(&[min, max, under, over, shy, sure], Some(&judged)),
            4,
            "the judged face, both size bounds and the face at the score floor"
        );
        let tall = Face {
            width: 30.0,
            height: 100.0,
            ..face(1600.0, 100.0, 0.9)
        };
        assert_eq!(count(&[tall], Some(&judged)), 2, "the long side decides");
        let low = face(1800.0, 100.0, GATE_LOW_RES_CONFIDENCE);
        let below = face(2000.0, 100.0, GATE_LOW_RES_CONFIDENCE.next_down());
        assert_eq!(
            gate_count(&[low, below, shy], Some(&judged), GATE_LOW_RES_CONFIDENCE),
            3,
            "the low-resolution floor counts a face at it and those above"
        );
        assert_eq!(count(&[low, below, shy], Some(&judged)), 1);
    }

    #[test]
    fn the_largest_come_first_up_to_the_limit() {
        let faces: Vec<Face> = (0..10)
            .map(|i| face(i as f32 * 300.0, 60.0 + i as f32 * 10.0, 0.9))
            .collect();
        let got = candidates(&faces, None, MAX_FACES - 1);
        assert_eq!(got.len(), MAX_FACES - 1);
        let sides: Vec<f32> = got.iter().map(|f| f.width).collect();
        assert_eq!(sides, [150.0, 140.0, 130.0, 120.0, 110.0, 100.0, 90.0]);
        assert!(candidates(&faces, None, 0).is_empty());
    }

    /// A mesh whose eye contours are centered on `left` and `right`: the EAR
    /// points of each eye `width` wide and `open` tall, the rest of the
    /// contour on its center.
    fn mesh(left: (f32, f32), right: (f32, f32), width: f32, open: f32) -> Mesh {
        let mut points: Vec<[f32; 2]> = (0..LANDMARKS)
            .map(|i| [i as f32 % 7.0, i as f32 % 5.0])
            .collect();
        let (w, o) = (width / 2.0, open / 2.0);
        for (contour, ear, (cx, cy)) in [
            (&LEFT_EYE_CONTOUR, [33, 160, 158, 133, 153, 144], left),
            (&RIGHT_EYE_CONTOUR, [362, 385, 387, 263, 373, 380], right),
        ] {
            for &i in contour {
                points[i] = [cx, cy];
            }
            let at = [
                (-w, 0.0),
                (-w / 2.0, -o),
                (w / 2.0, -o),
                (w, 0.0),
                (w / 2.0, o),
                (-w / 2.0, o),
            ];
            for (&i, (dx, dy)) in ear.iter().zip(at) {
                points[i] = [cx + dx, cy + dy];
            }
        }
        Mesh {
            points,
            pose: None,
            presence: 0.0,
        }
    }

    #[test]
    fn a_mesh_fitted_off_the_face_is_not_measured() {
        let f = face(0.0, 100.0, 0.9);
        let on = mesh(f.left_eye, f.right_eye, 20.0, 6.0);
        let m = measured(&f, &on).unwrap();
        assert_eq!(Some(m.eye_centers), mesh_eye_centers(&on.points));
        assert_eq!(
            mesh_eye_offset(&f, &on.points).map(|o| o < 1e-6),
            Some(true)
        );
        assert_eq!(Some(m.ear), more_closed_ear(&on.points));
        let shift = |d: f32| {
            mesh(
                (f.left_eye.0 + d, f.left_eye.1),
                (f.right_eye.0 + d, f.right_eye.1),
                20.0,
                6.0,
            )
        };
        assert!(
            measured(&f, &shift(9.5)).is_some(),
            "an offset under 0.1 fits"
        );
        assert!(measured(&f, &shift(10.5)).is_none());
        let empty = Face {
            width: 0.0,
            height: 0.0,
            ..f
        };
        assert!(measured(&empty, &on).is_none());
    }

    #[test]
    fn a_set_flag_abandons_the_measurement_after_a_face() {
        let rgb = vec![128u8; 120 * 60 * 3];
        // Outside the image: no mesh, no model run.
        let away = face(500.0, 60.0, 0.9);
        let never = AtomicBool::new(false);
        assert_eq!(
            measure_unless(&rgb, 120, 60, 1, &[away], &never),
            Some(Vec::new())
        );
        assert_eq!(
            measure_unless(&rgb, 120, 60, 1, &[], &AtomicBool::new(true)),
            Some(Vec::new())
        );
        assert_eq!(
            measure_unless(&rgb, 120, 60, 1, &[away], &AtomicBool::new(true)),
            None
        );
    }

    fn reading(ear: f64, full_eye_sharp: Option<f64>) -> Reading {
        Reading {
            ear,
            full_eye_sharp,
        }
    }

    #[test]
    fn a_face_at_the_closed_ear_is_closed_and_one_above_is_not() {
        let readings = [
            reading(EYES_CLOSED_EAR, None),
            reading(EYES_CLOSED_EAR.next_up(), None),
            reading(0.05, None),
        ];
        assert_eq!(
            counts(&readings, false),
            FaceCounts {
                judged: 3,
                closed: 2,
                soft: None
            }
        );
    }

    #[test]
    fn a_face_under_the_low_cut_is_soft_and_one_at_it_is_not() {
        let readings = [
            reading(0.3, Some(LOW_BELOW.next_down())),
            reading(0.3, Some(LOW_BELOW)),
            reading(0.3, None),
            reading(0.3, Some(0.05)),
        ];
        assert_eq!(counts(&readings, true).soft, Some(2));
        assert_eq!(
            counts(&readings, false).soft,
            None,
            "without the full JPEG the soft count is unknown"
        );
        assert_eq!(
            counts(&[], true),
            FaceCounts {
                judged: 0,
                closed: 0,
                soft: Some(0)
            }
        );
        assert_eq!(counts(&[], false), FaceCounts::default());
    }
}
