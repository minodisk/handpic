//! Where the focus mark goes and what color it takes, from the values pass 2
//! stored for the judged face. A port of the Tauri frontend's `focus.ts`
//! (`photoTier`, `afEyeState`, `focusMark`, `uprightPoint`, `faceMarks`,
//! `faceLabel`, `faceLabelAt`); its comments say where each cut comes from.

use riffle_backend::{index::Focus, ondemand::FaceBox};
use riffle_core::candidate::FocusCandidate;

/// The in-focus probability of the AF eyes.
pub const GOOD_EYE_FOCUS: f64 = 0.9;
/// The EAR of the more closed eye.
pub const GOOD_EYE_EAR: f64 = 0.25;
/// |yaw| in degrees.
pub const GOOD_MAX_YAW: f64 = 35.;
/// |pitch| in degrees.
pub const GOOD_MAX_PITCH: f64 = 45.;
/// The mesh's eyes farther than this from YuNet's eye landmarks, in face box
/// sides, mean a mesh fitted off the face.
pub const MAX_EYE_OFFSET: f64 = 0.1;
/// The face or a mesh eye region closer to the preview's edge than this, in
/// face box sides, means a face the frame cuts.
pub const MIN_EDGE_GAP: f64 = 0.02;
/// `fullsharp::SOFT_BELOW`: a frame under it on the full-size JPEG leaves the
/// tier; a file without the value is not demoted.
pub const GOOD_FULL_EYE_SHARP: f64 = 0.162;

/// The state the mark's color is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkState {
    Good,
    Neutral,
    Low,
}

impl MarkState {
    /// Bright green for a good photo, gray when the judged face's eyes are
    /// likely not in focus, white for everything else.
    pub fn color(self) -> u32 {
        match self {
            MarkState::Good => 0x33ff33,
            MarkState::Low => 0x999999,
            MarkState::Neutral => 0xffffff,
        }
    }
}

/// An axis-aligned rectangle of the unrotated drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The mark on the unrotated preview drawn `draw_width` x `draw_height` and
/// centered on the origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusMark {
    pub x: f32,
    pub y: f32,
    /// The AF frame, when the camera recorded its size.
    pub rect: Option<Rect>,
    pub state: MarkState,
}

fn clears(focus: &Focus) -> bool {
    let eyes = &focus.eyes;
    focus.eye_focus.is_some_and(|f| f >= GOOD_EYE_FOCUS)
        && eyes.eyes_ear.is_some_and(|e| e >= GOOD_EYE_EAR)
        && eyes
            .pose
            .is_some_and(|p| p.yaw.abs() <= GOOD_MAX_YAW && p.pitch.abs() <= GOOD_MAX_PITCH)
        && eyes.eye_offset.is_some_and(|o| o <= MAX_EYE_OFFSET)
        && eyes.edge_gap.is_some_and(|g| g >= MIN_EDGE_GAP)
        && eyes.full_eye_sharp.is_none_or(|s| s >= GOOD_FULL_EYE_SHARP)
}

/// Whether the frame is a good photo: a focus candidate whose AF eyes are in
/// focus, whose eyes are open and whose face is toward the camera, with the
/// mesh on the face, the face not cut by the frame's edge and the eyes not
/// soft at full size. `focus.ts`'s tier has the one value `good`.
pub fn photo_tier(focus: Option<&Focus>) -> bool {
    focus.is_some_and(|f| f.candidate == FocusCandidate::Candidate && clears(f))
}

/// `Low` for Rust's `not_candidate`, `Good` for the tier, else `Neutral`,
/// also without a focus.
pub fn af_eye_state(focus: Option<&Focus>) -> MarkState {
    match focus {
        None => MarkState::Neutral,
        Some(f) if f.candidate == FocusCandidate::NotCandidate => MarkState::Low,
        Some(f) if photo_tier(Some(f)) => MarkState::Good,
        Some(_) => MarkState::Neutral,
    }
}

/// `None` without a recorded point or for a manual-focus shot, whose recorded
/// point is not trusted.
pub fn focus_mark(focus: Option<&Focus>, draw_width: f32, draw_height: f32) -> Option<FocusMark> {
    let focus = focus.filter(|f| !f.manual_focus)?;
    let point = focus.point?;
    let (sensor_w, sensor_h) = (f32::from(point.sensor_w), f32::from(point.sensor_h));
    let x = -draw_width / 2. + f32::from(point.x) * draw_width / sensor_w;
    let y = -draw_height / 2. + f32::from(point.y) * draw_height / sensor_h;
    let rect = point.frame.map(|frame| {
        let width = f32::from(frame.width) * draw_width / sensor_w;
        let height = f32::from(frame.height) * draw_height / sensor_h;
        Rect {
            x: x - width / 2.,
            y: y - height / 2.,
            width,
            height,
        }
    });
    Some(FocusMark {
        x,
        y,
        rect,
        state: af_eye_state(Some(focus)),
    })
}

/// A point of the unrotated drawing centered on the origin, turned the way
/// the image is turned upright for `orientation`.
pub fn upright_point((x, y): (f32, f32), orientation: u16) -> (f32, f32) {
    match orientation {
        6 => (-y, x),
        8 => (y, -x),
        3 => (-x, -y),
        _ => (x, y),
    }
}

/// A detected face scaled onto the unrotated drawing centered on the origin.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceMark {
    pub rect: Rect,
    /// The eyes, the nose tip and the mouth corners.
    pub points: [(f32, f32); 5],
    pub label: String,
}

/// The faces scaled onto the unrotated preview drawn `draw_width` x
/// `draw_height` and centered on the origin, like `focus_mark`. The scale
/// comes from the stored preview size, not the drawn image's.
pub fn face_marks(
    faces: &[FaceBox],
    preview_width: f32,
    preview_height: f32,
    draw_width: f32,
    draw_height: f32,
    orientation: u16,
) -> Vec<FaceMark> {
    let (sx, sy) = (draw_width / preview_width, draw_height / preview_height);
    let at = |x: f32, y: f32| (x * sx - draw_width / 2., y * sy - draw_height / 2.);
    faces
        .iter()
        .map(|face| {
            let (x, y) = at(face.x, face.y);
            FaceMark {
                rect: Rect {
                    x,
                    y,
                    width: face.width * sx,
                    height: face.height * sy,
                },
                points: [
                    &face.left_eye,
                    &face.right_eye,
                    &face.nose,
                    &face.mouth_left,
                    &face.mouth_right,
                ]
                .map(|p| at(p.x, p.y)),
                label: face_label(face, orientation),
            }
        })
        .collect()
}

/// YuNet's score, the box size and the distance between the eyes, in the
/// stored preview's pixels, where the scan's thresholds are measured. The
/// label is drawn upright, so a quarter turn swaps the width and height.
pub fn face_label(face: &FaceBox, orientation: u16) -> String {
    let eyes = (face.right_eye.x - face.left_eye.x).hypot(face.right_eye.y - face.left_eye.y);
    let (width, height) = if matches!(orientation, 6 | 8) {
        (face.height, face.width)
    } else {
        (face.width, face.height)
    };
    format!(
        "{:.2}  {}×{}  eyes {}",
        face.score,
        width.round(),
        height.round(),
        eyes.round()
    )
}

/// The height a face label takes, its 11 px text plus the halo.
pub const FACE_LABEL_HEIGHT: f32 = 14.;

/// The top-left of a face label `label_width` wide, upright, in coordinates
/// centered on the image: above the upright box's top-left corner, or just
/// inside the box when the box touches the image's top, and shifted left
/// when it would run past the image's right edge.
pub fn face_label_at(
    rect: Rect,
    orientation: u16,
    draw_width: f32,
    draw_height: f32,
    label_width: f32,
) -> (f32, f32) {
    let a = upright_point((rect.x, rect.y), orientation);
    let b = upright_point((rect.x + rect.width, rect.y + rect.height), orientation);
    let (left, top) = (a.0.min(b.0), a.1.min(b.1));
    let (image_width, image_height) = if matches!(orientation, 6 | 8) {
        (draw_height, draw_width)
    } else {
        (draw_width, draw_height)
    };
    let x = (-image_width / 2.).max(left.min(image_width / 2. - label_width));
    let above = top - FACE_LABEL_HEIGHT;
    (
        x,
        if above < -image_height / 2. {
            top + 2.
        } else {
            above
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use riffle_backend::index::{EyesPose, FocusPoint, FocusSize, StoredEyes};
    use riffle_backend::ondemand::FacePoint;

    const AT: FocusPoint = FocusPoint {
        sensor_w: 7008,
        sensor_h: 4672,
        x: 1752,
        y: 1168,
        frame: None,
    };

    fn no_eyes() -> StoredEyes {
        StoredEyes {
            eyes_ear: None,
            eyes: None,
            eyes_closed: None,
            pose: None,
            eye_offset: None,
            edge_gap: None,
            full_eye_sharp: None,
        }
    }

    fn point() -> Focus {
        Focus {
            point: Some(AT),
            manual_focus: false,
            eye_focus: None,
            candidate: FocusCandidate::Unknown,
            eyes: no_eyes(),
        }
    }

    fn pose(yaw: f64, pitch: f64, roll: f64) -> Option<EyesPose> {
        Some(EyesPose { yaw, pitch, roll })
    }

    fn good() -> Focus {
        Focus {
            candidate: FocusCandidate::Candidate,
            eye_focus: Some(1.),
            eyes: StoredEyes {
                eyes_ear: Some(0.35),
                pose: pose(10., -5., 3.),
                eye_offset: Some(0.03),
                edge_gap: Some(2.),
                full_eye_sharp: Some(0.3),
                ..no_eyes()
            },
            ..point()
        }
    }

    fn with(focus: Focus, edit: impl FnOnce(&mut Focus)) -> Focus {
        let mut focus = focus;
        edit(&mut focus);
        focus
    }

    fn ear(ear: f64) -> Focus {
        with(good(), |f| f.eyes.eyes_ear = Some(ear))
    }

    fn candidate(candidate: FocusCandidate) -> Focus {
        with(point(), |f| f.candidate = candidate)
    }

    fn good_photo(focus: &Focus) -> bool {
        photo_tier(Some(focus))
    }

    fn rated(eye_focus: f64, eyes_ear: f64, p: Option<EyesPose>, offset: f64, gap: f64) -> Focus {
        with(good(), |f| {
            f.eye_focus = Some(eye_focus);
            f.eyes.eyes_ear = Some(eyes_ear);
            f.eyes.pose = p;
            f.eyes.eye_offset = Some(offset);
            f.eyes.edge_gap = Some(gap);
        })
    }

    #[test]
    fn focus_mark_draws_nothing_without_a_focus_point() {
        assert_eq!(focus_mark(None, 700., 467.), None);
    }

    #[test]
    fn focus_mark_draws_nothing_for_a_manual_focus_shot() {
        let frame = Some(FocusSize {
            width: 876,
            height: 584,
        });
        let manual = with(point(), |f| {
            f.point = Some(FocusPoint { frame, ..AT });
            f.manual_focus = true;
        });
        assert_eq!(focus_mark(Some(&manual), 700., 467.), None);
    }

    #[test]
    fn focus_mark_draws_nothing_for_a_focus_with_no_recorded_point_whatever_its_state() {
        let good = with(good(), |f| f.point = None);
        let point = with(point(), |f| f.point = None);
        assert_eq!(focus_mark(Some(&good), 700., 467.), None);
        assert_eq!(focus_mark(Some(&point), 700., 467.), None);
    }

    #[test]
    fn focus_mark_of_a_point_without_a_frame_is_a_crosshair_only() {
        assert_eq!(
            focus_mark(Some(&point()), 700., 468.),
            Some(FocusMark {
                x: -175.,
                y: -117.,
                rect: None,
                state: MarkState::Neutral,
            })
        );
    }

    #[test]
    fn focus_mark_scales_a_frame_onto_the_preview_centered_on_the_point() {
        let framed = with(point(), |f| {
            f.point = Some(FocusPoint {
                frame: Some(FocusSize {
                    width: 876,
                    height: 584,
                }),
                ..AT
            });
        });
        assert_eq!(
            focus_mark(Some(&framed), 700., 468.),
            Some(FocusMark {
                x: -175.,
                y: -117.,
                rect: Some(Rect {
                    x: -218.75,
                    y: -146.25,
                    width: 87.5,
                    height: 58.5,
                }),
                state: MarkState::Neutral,
            })
        );
    }

    #[test]
    fn focus_mark_maps_the_candidate_cue_to_its_state() {
        let state = |c| focus_mark(Some(&candidate(c)), 700., 468.).map(|m| m.state);
        assert_eq!(state(FocusCandidate::NotCandidate), Some(MarkState::Low));
        assert_eq!(state(FocusCandidate::Unknown), Some(MarkState::Neutral));
    }

    #[test]
    fn focus_mark_of_a_focus_candidate_carries_its_photo_tier_else_neutral() {
        let state = |f: Focus| focus_mark(Some(&f), 700., 468.).map(|m| m.state);
        assert_eq!(state(good()), Some(MarkState::Good));
        assert_eq!(state(ear(GOOD_EYE_EAR)), Some(MarkState::Good));
        assert_eq!(state(ear(0.1)), Some(MarkState::Neutral));
        assert_eq!(
            state(candidate(FocusCandidate::Candidate)),
            Some(MarkState::Neutral)
        );
    }

    #[test]
    fn focus_mark_of_a_manual_focus_or_missing_point_stays_none_whatever_the_state() {
        let manual = with(candidate(FocusCandidate::Candidate), |f| {
            f.manual_focus = true;
        });
        assert_eq!(focus_mark(Some(&manual), 700., 468.), None);
        assert_eq!(focus_mark(None, 700., 468.), None);
    }

    #[test]
    fn colors_are_bright_green_for_good_gray_for_low_white_for_neutral() {
        assert_eq!(MarkState::Good.color(), 0x33ff33);
        assert_eq!(MarkState::Low.color(), 0x999999);
        assert_eq!(MarkState::Neutral.color(), 0xffffff);
    }

    #[test]
    fn af_eye_state_is_neutral_without_a_focus_as_on_a_jpeg() {
        assert_eq!(af_eye_state(None), MarkState::Neutral);
    }

    #[test]
    fn af_eye_state_is_good_for_the_photo_tier() {
        assert_eq!(af_eye_state(Some(&good())), MarkState::Good);
    }

    #[test]
    fn af_eye_state_is_good_at_good_eye_focus_with_the_other_conditions_met() {
        let edge = with(good(), |f| f.eye_focus = Some(GOOD_EYE_FOCUS));
        assert_eq!(af_eye_state(Some(&edge)), MarkState::Good);
    }

    #[test]
    fn af_eye_state_is_neutral_for_a_focus_candidate_that_is_not_good() {
        let below = with(good(), |f| f.eye_focus = Some(GOOD_EYE_FOCUS - 0.01));
        let bare = with(candidate(FocusCandidate::Candidate), |f| {
            f.eye_focus = Some(0.772);
        });
        assert_eq!(af_eye_state(Some(&ear(0.1))), MarkState::Neutral);
        assert_eq!(af_eye_state(Some(&below)), MarkState::Neutral);
        assert_eq!(af_eye_state(Some(&bare)), MarkState::Neutral);
    }

    #[test]
    fn af_eye_state_is_low_for_a_not_candidate_focus_whatever_else_clears_the_rule() {
        let low = |f: Focus| {
            with(f, |f| {
                f.candidate = FocusCandidate::NotCandidate;
                f.eye_focus = Some(0.7719);
            })
        };
        assert_eq!(af_eye_state(Some(&low(point()))), MarkState::Low);
        assert_eq!(af_eye_state(Some(&low(good()))), MarkState::Low);
    }

    #[test]
    fn af_eye_state_is_neutral_for_an_unknown_focus() {
        let unknown = |f: Focus| {
            with(f, |f| {
                f.candidate = FocusCandidate::Unknown;
                f.eye_focus = None;
            })
        };
        assert_eq!(af_eye_state(Some(&unknown(point()))), MarkState::Neutral);
        assert_eq!(af_eye_state(Some(&unknown(good()))), MarkState::Neutral);
    }

    #[test]
    fn photo_tier_passes_a_candidate_whose_eyes_and_pose_clear_every_cut() {
        assert!(good_photo(&good()));
    }

    #[test]
    fn photo_tier_passes_a_candidate_with_no_recorded_point_on_its_values() {
        let pointless = with(good(), |f| f.point = None);
        let manual = with(pointless, |f| f.manual_focus = true);
        let below = with(pointless, |f| f.eye_focus = Some(GOOD_EYE_FOCUS - 0.01));
        assert!(good_photo(&pointless));
        assert!(good_photo(&manual));
        assert!(!good_photo(&below));
    }

    #[test]
    fn photo_tier_is_false_without_a_focus() {
        assert!(!photo_tier(None));
    }

    #[test]
    fn photo_tier_is_false_for_a_frame_off_the_candidate_even_with_good_eyes_and_pose() {
        for c in [FocusCandidate::NotCandidate, FocusCandidate::Unknown] {
            assert!(!good_photo(&with(good(), |f| f.candidate = c)));
        }
    }

    #[test]
    fn photo_tier_is_false_when_any_value_is_missing() {
        assert!(!good_photo(&with(good(), |f| f.eye_focus = None)));
        assert!(!good_photo(&with(good(), |f| f.eyes.eyes_ear = None)));
        assert!(!good_photo(&with(good(), |f| f.eyes.pose = None)));
        assert!(!good_photo(&with(good(), |f| f.eyes.eye_offset = None)));
        assert!(!good_photo(&with(good(), |f| f.eyes.edge_gap = None)));
    }

    #[test]
    fn the_eye_offset_passes_at_and_below_its_cut_not_above() {
        let at = |o: f64| good_photo(&with(good(), |f| f.eyes.eye_offset = Some(o)));
        assert!(at(MAX_EYE_OFFSET));
        assert!(at(MAX_EYE_OFFSET - 0.001));
        assert!(!at(MAX_EYE_OFFSET + 0.001));
    }

    #[test]
    fn the_edge_gap_passes_at_and_above_its_cut_not_below_or_outside() {
        let at = |g: f64| good_photo(&with(good(), |f| f.eyes.edge_gap = Some(g)));
        assert!(at(MIN_EDGE_GAP));
        assert!(at(MIN_EDGE_GAP + 0.001));
        assert!(!at(MIN_EDGE_GAP - 0.001));
        assert!(!at(-0.1));
    }

    #[test]
    fn a_frame_soft_at_full_size_is_not_good_at_the_cut_or_unmeasured_it_is() {
        let sharp = |s: Option<f64>| with(good(), |f| f.eyes.full_eye_sharp = s);
        let soft = sharp(Some(GOOD_FULL_EYE_SHARP - 0.001));
        assert!(!good_photo(&soft));
        assert_eq!(af_eye_state(Some(&soft)), MarkState::Neutral);
        assert_eq!(
            af_eye_state(Some(&sharp(Some(GOOD_FULL_EYE_SHARP)))),
            MarkState::Good
        );
        assert_eq!(af_eye_state(Some(&sharp(None))), MarkState::Good);
    }

    #[test]
    fn the_three_frames_the_user_named_are_not_good() {
        // `_DSC2638`: the mesh off a rotated face.
        assert!(!good_photo(&rated(
            0.999,
            0.3601,
            pose(3., -11.8, 11.8),
            0.1247,
            0.2241
        )));
        // `_DSC2827`: turned, the EAR inflated.
        assert!(!good_photo(&rated(
            0.9984,
            0.3032,
            pose(-48.6, 5.2, 9.4),
            0.0866,
            1.6918
        )));
        // `_DSC3345`: cut by the left edge, its mesh off too.
        assert!(!good_photo(&rated(
            0.9984,
            0.314,
            pose(-15.3, 2.7, -3.6),
            0.3521,
            0.0076
        )));
    }

    #[test]
    fn the_one_star_frames_a_looser_yaw_or_eye_offset_cut_lets_in_are_not_good() {
        // `_DSC2050`: turned past the yaw cut.
        assert!(!good_photo(&rated(
            0.9966,
            0.3228,
            pose(-36.6, 11.7, 5.1),
            0.0241,
            5.1189
        )));
        // `_DSC6920`: the mesh off the eyes of a face YuNet sees in profile.
        assert!(!good_photo(&rated(
            0.9913,
            0.4054,
            pose(26.8, -17.4, 3.6),
            0.1769,
            3.3674
        )));
    }

    #[test]
    fn a_five_star_frame_the_step_5b_cuts_left_out_is_good() {
        // `_DSC3280`: the AF eyes at 0.92.
        assert!(good_photo(&rated(
            0.9218,
            0.3468,
            pose(-23.9, -16.1, -4.7),
            0.0298,
            7.2376
        )));
        // `_DSC4912`: turned 34.5 degrees.
        assert!(good_photo(&rated(
            0.9986,
            0.4194,
            pose(34.5, 3.7, 1.6),
            0.0456,
            4.7066
        )));
    }

    #[test]
    fn the_eye_focus_cut_passes_at_and_above_not_below() {
        let at = |e: f64| good_photo(&with(good(), |f| f.eye_focus = Some(e)));
        assert!(at(GOOD_EYE_FOCUS));
        assert!(at(GOOD_EYE_FOCUS + 0.0005));
        assert!(!at(GOOD_EYE_FOCUS - 0.0005));
    }

    #[test]
    fn the_ear_cut_passes_at_and_above_not_below() {
        assert!(good_photo(&ear(GOOD_EYE_EAR)));
        assert!(good_photo(&ear(GOOD_EYE_EAR + 0.001)));
        assert!(!good_photo(&ear(GOOD_EYE_EAR - 0.001)));
    }

    #[test]
    fn the_yaw_cut_passes_at_and_within_not_beyond_either_sign() {
        for sign in [1., -1.] {
            let at =
                |yaw: f64| good_photo(&with(good(), |f| f.eyes.pose = pose(sign * yaw, 0., 0.)));
            assert!(at(GOOD_MAX_YAW));
            assert!(at(GOOD_MAX_YAW - 0.1));
            assert!(!at(GOOD_MAX_YAW + 0.1));
        }
    }

    #[test]
    fn the_pitch_cut_passes_at_and_within_not_beyond_either_sign() {
        for sign in [1., -1.] {
            let at = |pitch: f64| {
                good_photo(&with(good(), |f| f.eyes.pose = pose(0., sign * pitch, 0.)))
            };
            assert!(at(GOOD_MAX_PITCH));
            assert!(at(GOOD_MAX_PITCH - 0.1));
            assert!(!at(GOOD_MAX_PITCH + 0.1));
        }
    }

    #[test]
    fn the_roll_is_not_a_cut() {
        assert!(good_photo(
            &with(good(), |f| f.eyes.pose = pose(0., 0., 90.))
        ));
    }

    #[test]
    fn the_folded_tier_is_good_or_not() {
        assert!(photo_tier(Some(&good())));
        assert!(!photo_tier(Some(&ear(0.1))));
    }

    #[test]
    fn the_cuts_are_the_step_7_cuts() {
        assert_eq!(GOOD_EYE_FOCUS, 0.9);
        assert_eq!(GOOD_EYE_EAR, 0.25);
        assert_eq!(GOOD_MAX_YAW, 35.);
        assert_eq!(GOOD_MAX_PITCH, 45.);
        assert_eq!(MAX_EYE_OFFSET, 0.1);
        assert_eq!(MIN_EDGE_GAP, 0.02);
        assert_eq!(GOOD_FULL_EYE_SHARP, 0.162);
    }

    #[test]
    fn a_frame_below_the_step_5_good_cuts_that_cleared_the_fair_ones_is_good() {
        assert!(good_photo(&with(good(), |f| f.eye_focus = Some(0.995))));
        assert!(good_photo(&ear(0.27)));
        assert!(good_photo(&with(ear(0.25), |f| f.eye_focus = Some(0.99))));
    }

    #[test]
    fn the_exclusions_still_apply_at_the_folded_cuts() {
        let edge = with(ear(GOOD_EYE_EAR), |f| f.eye_focus = Some(GOOD_EYE_FOCUS));
        assert!(good_photo(&edge));
        assert!(!good_photo(&with(edge, |f| {
            f.eyes.eye_offset = Some(MAX_EYE_OFFSET + 0.001);
        })));
        assert!(!good_photo(&with(edge, |f| {
            f.eyes.edge_gap = Some(MIN_EDGE_GAP - 0.001);
        })));
    }

    #[test]
    fn upright_point_turns_a_point_as_the_image_is_turned() {
        let p = (10., 20.);
        assert_eq!(upright_point(p, 1), (10., 20.));
        assert_eq!(upright_point(p, 6), (-20., 10.));
        assert_eq!(upright_point(p, 8), (20., -10.));
        assert_eq!(upright_point(p, 3), (-10., -20.));
    }

    fn face(x: f32, y: f32) -> FaceBox {
        let p = |dx: f32, dy: f32| FacePoint {
            x: x + dx,
            y: y + dy,
        };
        FaceBox {
            x,
            y,
            width: 100.,
            height: 120.,
            score: 0.934,
            left_eye: p(30., 40.),
            right_eye: p(70., 40.),
            nose: p(50., 70.),
            mouth_left: p(35., 95.),
            mouth_right: p(65., 95.),
        }
    }

    const LABEL: &str = "0.93  100×120  eyes 40";

    #[test]
    fn a_face_at_the_previews_origin_sits_at_the_drawn_images_corner() {
        assert_eq!(
            face_marks(&[face(0., 0.)], 1600., 1080., 1600., 1080., 1),
            vec![FaceMark {
                rect: Rect {
                    x: -800.,
                    y: -540.,
                    width: 100.,
                    height: 120.,
                },
                points: [
                    (-770., -500.),
                    (-730., -500.),
                    (-750., -470.),
                    (-765., -445.),
                    (-735., -445.),
                ],
                label: LABEL.into(),
            }]
        );
    }

    #[test]
    fn a_face_at_the_far_corner_ends_at_the_drawn_images_far_corner() {
        assert_eq!(
            face_marks(&[face(1500., 960.)], 1600., 1080., 1600., 1080., 1),
            vec![FaceMark {
                rect: Rect {
                    x: 700.,
                    y: 420.,
                    width: 100.,
                    height: 120.,
                },
                points: [
                    (730., 460.),
                    (770., 460.),
                    (750., 490.),
                    (735., 515.),
                    (765., 515.),
                ],
                label: LABEL.into(),
            }]
        );
    }

    #[test]
    fn a_preview_drawn_smaller_scales_the_faces_by_the_preview_size() {
        assert_eq!(
            face_marks(&[face(400., 200.)], 1600., 1080., 800., 540., 1),
            vec![FaceMark {
                rect: Rect {
                    x: -200.,
                    y: -170.,
                    width: 50.,
                    height: 60.,
                },
                points: [
                    (-185., -150.),
                    (-165., -150.),
                    (-175., -135.),
                    (-182.5, -122.5),
                    (-167.5, -122.5),
                ],
                label: LABEL.into(),
            }]
        );
    }

    fn labeled() -> FaceBox {
        let p = |x: f32, y: f32| FacePoint { x, y };
        FaceBox {
            x: 0.,
            y: 0.,
            width: 211.6,
            height: 254.4,
            score: 0.934,
            left_eye: p(100., 100.),
            right_eye: p(160., 111.),
            nose: p(130., 140.),
            mouth_left: p(110., 180.),
            mouth_right: p(150., 180.),
        }
    }

    #[test]
    fn face_label_gives_the_score_the_box_size_and_the_eye_distance_in_preview_pixels() {
        assert_eq!(face_label(&labeled(), 1), "0.93  212×254  eyes 61");
        let score = FaceBox {
            score: 0.6,
            ..labeled()
        };
        assert_eq!(face_label(&score, 1), "0.60  212×254  eyes 61");
    }

    #[test]
    fn face_label_swaps_the_box_size_on_a_quarter_turn_so_it_reads_as_drawn_upright() {
        assert_eq!(face_label(&labeled(), 6), "0.93  254×212  eyes 61");
        assert_eq!(face_label(&labeled(), 8), "0.93  254×212  eyes 61");
        assert_eq!(face_label(&labeled(), 3), "0.93  212×254  eyes 61");
    }

    // A 300x200 unrotated drawing: 200x300 upright on a quarter turn.
    const LABELED: Rect = Rect {
        x: -50.,
        y: -40.,
        width: 40.,
        height: 60.,
    };

    #[test]
    fn face_label_at_sits_above_the_upright_boxs_top_left_corner() {
        let at = |orientation| face_label_at(LABELED, orientation, 300., 200., 80.);
        assert_eq!(at(1), (-50., -40. - FACE_LABEL_HEIGHT));
        assert_eq!(at(6), (-20., -50. - FACE_LABEL_HEIGHT));
        assert_eq!(at(8), (-40., 10. - FACE_LABEL_HEIGHT));
        assert_eq!(at(3), (10., -20. - FACE_LABEL_HEIGHT));
    }

    #[test]
    fn face_label_at_drops_inside_the_box_when_the_box_touches_the_images_top() {
        let at = |rect, orientation| face_label_at(rect, orientation, 300., 200., 80.).1;
        assert_eq!(
            at(
                Rect {
                    y: -100.,
                    ..LABELED
                },
                1
            ),
            -98.
        );
        assert_eq!(
            at(
                Rect {
                    x: -150.,
                    ..LABELED
                },
                6
            ),
            -148.
        );
        assert_eq!(at(Rect { x: 110., ..LABELED }, 8), -148.);
        assert_eq!(at(Rect { y: 40., ..LABELED }, 3), -98.);
    }

    #[test]
    fn face_label_at_stays_inside_the_images_right_edge() {
        let at = |rect, orientation| face_label_at(rect, orientation, 300., 200., 80.).0;
        assert_eq!(at(Rect { x: 100., ..LABELED }, 1), 70.);
        assert_eq!(
            at(
                Rect {
                    y: -100.,
                    ..LABELED
                },
                6
            ),
            20.
        );
    }
}
