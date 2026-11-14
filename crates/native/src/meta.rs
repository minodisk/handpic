//! The meta pane's rows grouped by where each value comes from, a port of
//! `meta.ts`'s `metaGroups`: the standard EXIF/TIFF tags, the vendor
//! MakerNote with the AF point and frame the index stored (`af_rows`), and
//! the app's own analysis (`analysis`): every value pass 1 and pass 2 stored
//! for the file, never the on-demand eyes judgment `metaGroups` falls back to.

use riffle_backend::exif::Metadata;
use riffle_backend::index::{Focus, FocusPoint};
use riffle_core::candidate::FocusCandidate;
use riffle_core::eyes::{EYES_CLOSED_EAR, EyeState};

use crate::mark::GOOD_FULL_EYE_SHARP;

pub const EXIF_HEADING: &str = "EXIF";
pub const MAKER_NOTE_HEADING: &str = "Maker note";
pub const ANALYSIS_HEADING: &str = "Analysis";

/// What the pane shows of the file whose preview is on screen.
pub enum Shown {
    Read(Box<Metadata>),
    /// Its preview failed: the file name and why.
    Failed {
        name: String,
        error: String,
    },
}

/// The pane's `n / total`: the pager's current file over the shown list, as
/// the Tauri `positionEl`, so a refilter that hides the file on screen moves
/// it at once rather than when the next preview arrives.
pub fn position(current: usize, total: usize) -> String {
    format!("{} / {}", current + 1, total)
}

#[derive(Debug, PartialEq)]
pub struct Row<'a> {
    pub label: &'a str,
    pub value: &'a str,
}

#[derive(Debug, PartialEq)]
pub struct Group<'a> {
    pub heading: &'static str,
    pub rows: Vec<Row<'a>>,
}

fn group<'a>(heading: &'static str, rows: &[(&'a str, &'a Option<String>)]) -> Group<'a> {
    Group {
        heading,
        rows: rows
            .iter()
            .filter_map(|&(label, value)| value.as_deref().map(|value| Row { label, value }))
            .collect(),
    }
}

/// The EAR the openness reaches 100 at, `focus.ts`'s `EYES_WIDE_OPEN_EAR`.
const EYES_WIDE_OPEN_EAR: f64 = 0.41;

/// JavaScript's `Math.round`: `f64::round` takes `-5.5` to `-6` and an angle
/// in `[-0.5, 0)` to `-0`.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn percent(p: f64) -> String {
    format!("{}%", js_round(p * 100.))
}

/// `focus.ts`'s `eyesOpenness`: 0 at `EYES_CLOSED_EAR` and below to 100 at
/// `EYES_WIDE_OPEN_EAR` and above.
fn openness(ear: f64) -> f64 {
    ((ear - EYES_CLOSED_EAR) / (EYES_WIDE_OPEN_EAR - EYES_CLOSED_EAR)).clamp(0., 1.) * 100.
}

fn degrees(angle: f64) -> String {
    format!("{}°", js_round(angle))
}

/// The `Maker note` rows from the index: the AF point in sensor
/// coordinates and the AF frame when one was recorded.
pub fn af_rows(point: Option<&FocusPoint>) -> Vec<(&'static str, Option<String>)> {
    let Some(p) = point else {
        return Vec::new();
    };
    vec![
        (
            "AF point",
            Some(format!(
                "{}, {} of {} × {}",
                p.x, p.y, p.sensor_w, p.sensor_h
            )),
        ),
        (
            "AF frame",
            p.frame.map(|f| format!("{} × {}", f.width, f.height)),
        ),
    ]
}

/// The `Analysis` rows from the values the scan stored: `metaGroups`' rows
/// with the same labels and strings, each computed value it does not show
/// next to the one it derives from, and the preview's sharpness.
pub fn analysis(
    focus: Option<&Focus>,
    sharpness: Option<f64>,
) -> Vec<(&'static str, Option<String>)> {
    let eyes = focus.map(|f| f.eyes).unwrap_or_default();
    let pose = eyes.pose;
    let fixed = |v: f64| format!("{v:.3}");
    vec![
        (
            "AF eye in focus",
            focus.and_then(|f| f.eye_focus).map(percent),
        ),
        (
            "Focus candidate",
            focus
                .and_then(|f| match f.candidate {
                    FocusCandidate::Candidate => Some("Yes"),
                    FocusCandidate::NotCandidate => Some("No"),
                    FocusCandidate::Unknown => None,
                })
                .map(str::to_owned),
        ),
        (
            "Eyes at full size",
            eyes.full_eye_sharp.map(|s| {
                let verdict = if s >= GOOD_FULL_EYE_SHARP {
                    "Sharp"
                } else {
                    "Soft"
                };
                format!("{verdict} · {s:.3}")
            }),
        ),
        (
            "Eyes",
            eyes.eyes.zip(eyes.eyes_ear).map(|(state, ear)| {
                let state = match state {
                    EyeState::Closed => "Closed",
                    EyeState::Open => "Open",
                };
                format!("{state} · {}", js_round(openness(ear)))
            }),
        ),
        ("Eyes closed", eyes.eyes_closed.map(percent)),
        ("Eye aspect ratio", eyes.eyes_ear.map(fixed)),
        ("Yaw", pose.map(|p| degrees(p.yaw))),
        ("Pitch", pose.map(|p| degrees(p.pitch))),
        ("Roll", pose.map(|p| degrees(p.roll))),
        ("Mesh eye offset", eyes.eye_offset.map(fixed)),
        ("Edge gap", eyes.edge_gap.map(fixed)),
        ("Sharpness", sharpness.map(|s| format!("{s:.1}"))),
    ]
}

/// A row is left out when its value is `None` and a group when it has no
/// row. The rows from the index (`maker_note`, after `AF tracking`, and
/// `analysis`) need no `meta`, so a file whose metadata could not be read
/// still shows them.
pub fn groups<'a>(
    meta: Option<&'a Metadata>,
    maker_note: &'a [(&'a str, Option<String>)],
    analysis: &'a [(&'a str, Option<String>)],
) -> Vec<Group<'a>> {
    let mut groups = Vec::new();
    let mut maker = Vec::new();
    if let Some(m) = meta {
        groups.push(group(
            EXIF_HEADING,
            &[
                ("Aperture", &m.aperture),
                ("Shutter", &m.shutter),
                ("ISO", &m.iso),
                ("Focal length", &m.focal_length),
                ("Exposure", &m.exposure_bias),
                ("Camera", &m.camera),
                ("Lens", &m.lens),
                ("Captured", &m.captured_at),
            ],
        ));
        maker.extend([
            ("Shutter type", &m.shutter_type),
            ("Focus mode", &m.focus_mode),
            ("AF area", &m.af_area),
            ("AF tracking", &m.af_tracking),
        ]);
    }
    maker.extend(maker_note.iter().map(|(l, v)| (*l, v)));
    if let Some(m) = meta {
        maker.extend([
            ("Drive", &m.drive),
            ("Stabilization", &m.stabilization),
            ("Exposure mode", &m.exposure_mode),
            ("Metering", &m.metering),
            ("Creative style", &m.creative_style),
            ("DRO", &m.dro),
            ("RAW type", &m.raw_type),
            ("Focus distance", &m.focus_distance),
        ]);
    }
    groups.push(group(MAKER_NOTE_HEADING, &maker));
    let analysis: Vec<_> = analysis.iter().map(|(l, v)| (*l, v)).collect();
    groups.push(group(ANALYSIS_HEADING, &analysis));
    groups.retain(|g| !g.rows.is_empty());
    groups
}

#[cfg(test)]
mod tests {
    use riffle_backend::index::{EyesPose, FocusSize, StoredEyes};

    use super::*;

    /// The one-based `n / total` text. Which index it is given (the pager's
    /// current, not the shown file's) is the call site's rule in `main.rs`.
    #[test]
    fn the_position_is_one_based() {
        assert_eq!(position(1, 2), "2 / 2");
    }

    fn empty() -> Metadata {
        Metadata {
            name: "a.ARW".into(),
            camera: None,
            lens: None,
            aperture: None,
            shutter: None,
            shutter_type: None,
            focus_mode: None,
            af_area: None,
            af_tracking: None,
            drive: None,
            stabilization: None,
            exposure_mode: None,
            metering: None,
            creative_style: None,
            dro: None,
            raw_type: None,
            iso: None,
            focal_length: None,
            exposure_bias: None,
            focus_distance: None,
            captured_at: None,
        }
    }

    fn rows<'a>(group: &Group<'a>) -> Vec<(&'a str, &'a str)> {
        group.rows.iter().map(|r| (r.label, r.value)).collect()
    }

    #[test]
    fn only_exif_fields_give_one_group_and_no_analysis() {
        let meta = Metadata {
            aperture: Some("f/2.8".into()),
            shutter: Some("1/250".into()),
            camera: Some("ILCE-1".into()),
            ..empty()
        };
        let groups = groups(Some(&meta), &[], &[]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].heading, EXIF_HEADING);
        assert_eq!(
            rows(&groups[0]),
            [
                ("Aperture", "f/2.8"),
                ("Shutter", "1/250"),
                ("Camera", "ILCE-1")
            ]
        );
    }

    #[test]
    fn keeps_meta_ts_order_across_groups() {
        let meta = Metadata {
            captured_at: Some("2026-11-10 10:00:00".into()),
            iso: Some("ISO 100".into()),
            focus_distance: Some("1.23 m".into()),
            shutter_type: Some("Electronic".into()),
            dro: Some("Off".into()),
            ..empty()
        };
        let groups = groups(Some(&meta), &[], &[]);
        let headings: Vec<_> = groups.iter().map(|g| g.heading).collect();
        assert_eq!(headings, [EXIF_HEADING, MAKER_NOTE_HEADING]);
        assert_eq!(
            rows(&groups[0]),
            [("ISO", "ISO 100"), ("Captured", "2026-11-10 10:00:00")]
        );
        assert_eq!(
            rows(&groups[1]),
            [
                ("Shutter type", "Electronic"),
                ("DRO", "Off"),
                ("Focus distance", "1.23 m")
            ]
        );
    }

    #[test]
    fn empty_metadata_gives_no_group() {
        assert!(groups(Some(&empty()), &[], &[]).is_empty());
        assert!(groups(None, &[], &[]).is_empty());
    }

    #[test]
    fn analysis_rows_follow_and_drop_none() {
        let meta = Metadata {
            iso: Some("ISO 100".into()),
            ..empty()
        };
        let analysis = [
            ("AF eye in focus", Some("82%".to_owned())),
            ("Eyes", None),
            ("Yaw", Some("3°".to_owned())),
        ];
        let groups = groups(Some(&meta), &[], &analysis);
        let headings: Vec<_> = groups.iter().map(|g| g.heading).collect();
        assert_eq!(headings, [EXIF_HEADING, ANALYSIS_HEADING]);
        assert_eq!(
            rows(&groups[1]),
            [("AF eye in focus", "82%"), ("Yaw", "3°")]
        );
    }

    #[test]
    fn analysis_shows_without_metadata() {
        let analysis = [("Yaw", Some("3°".to_owned()))];
        let groups = groups(None, &[], &analysis);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].heading, ANALYSIS_HEADING);
    }

    #[test]
    fn all_none_analysis_gives_no_group() {
        let analysis = [("Eyes", None), ("Yaw", None)];
        assert!(groups(None, &[], &analysis).is_empty());
    }

    fn sony() -> Metadata {
        Metadata {
            name: "DSC00001.ARW".into(),
            camera: Some("SONY ILCE-1".into()),
            lens: Some("FE 50mm F1.2 GM".into()),
            aperture: Some("f/2.8".into()),
            shutter: Some("1/500".into()),
            shutter_type: Some("Electronic front curtain".into()),
            focus_mode: Some("AF-C".into()),
            af_area: Some("Custom AF Area".into()),
            af_tracking: Some("Face tracking".into()),
            drive: Some("Continuous, frame 2".into()),
            stabilization: Some("On".into()),
            exposure_mode: Some("Manual".into()),
            metering: Some("Multi-segment".into()),
            creative_style: Some("Standard".into()),
            dro: Some("Auto".into()),
            raw_type: Some("Lossless Compressed RAW".into()),
            iso: Some("ISO 100".into()),
            focal_length: Some("50 mm".into()),
            exposure_bias: Some("+0.3 EV".into()),
            focus_distance: None,
            captured_at: Some("2026-09-24 12:00:00".into()),
        }
    }

    const POINT: FocusPoint = FocusPoint {
        sensor_w: 8640,
        sensor_h: 5760,
        x: 3120,
        y: 2080,
        frame: Some(FocusSize {
            width: 400,
            height: 300,
        }),
    };

    fn focus(eye_focus: Option<f64>, candidate: FocusCandidate) -> Focus {
        Focus {
            point: None,
            manual_focus: false,
            eye_focus,
            candidate,
            eyes: StoredEyes::default(),
        }
    }

    fn with_eyes(eyes: StoredEyes) -> Focus {
        Focus {
            eyes,
            ..focus(None, FocusCandidate::Unknown)
        }
    }

    fn pose(yaw: f64, pitch: f64, roll: f64) -> Option<EyesPose> {
        Some(EyesPose { yaw, pitch, roll })
    }

    /// The rows the scan's values give, in the order shown.
    fn shown(focus: Option<&Focus>, sharpness: Option<f64>) -> Vec<(&'static str, String)> {
        analysis(focus, sharpness)
            .into_iter()
            .filter_map(|(l, v)| v.map(|v| (l, v)))
            .collect()
    }

    fn value(focus: &Focus, label: &str) -> Option<String> {
        analysis(Some(focus), None)
            .into_iter()
            .find(|(l, _)| *l == label)
            .and_then(|(_, v)| v)
    }

    #[test]
    fn splits_sony_like_input_into_exif_maker_note_and_analysis() {
        let meta = sony();
        let analysis = analysis(Some(&focus(Some(0.87), FocusCandidate::Unknown)), None);
        let groups = groups(Some(&meta), &[], &analysis);
        let headings: Vec<_> = groups.iter().map(|g| g.heading).collect();
        assert_eq!(
            headings,
            [EXIF_HEADING, MAKER_NOTE_HEADING, ANALYSIS_HEADING]
        );
        assert_eq!(
            rows(&groups[0]),
            [
                ("Aperture", "f/2.8"),
                ("Shutter", "1/500"),
                ("ISO", "ISO 100"),
                ("Focal length", "50 mm"),
                ("Exposure", "+0.3 EV"),
                ("Camera", "SONY ILCE-1"),
                ("Lens", "FE 50mm F1.2 GM"),
                ("Captured", "2026-09-24 12:00:00"),
            ]
        );
        assert_eq!(
            rows(&groups[1]),
            [
                ("Shutter type", "Electronic front curtain"),
                ("Focus mode", "AF-C"),
                ("AF area", "Custom AF Area"),
                ("AF tracking", "Face tracking"),
                ("Drive", "Continuous, frame 2"),
                ("Stabilization", "On"),
                ("Exposure mode", "Manual"),
                ("Metering", "Multi-segment"),
                ("Creative style", "Standard"),
                ("DRO", "Auto"),
                ("RAW type", "Lossless Compressed RAW"),
            ]
        );
        assert_eq!(rows(&groups[2]), [("AF eye in focus", "87%")]);
    }

    #[test]
    fn shows_only_the_analysis_group_without_metadata() {
        let analysis = analysis(Some(&focus(Some(0.87), FocusCandidate::Unknown)), None);
        let groups = groups(None, &[], &analysis);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].heading, ANALYSIS_HEADING);
        assert_eq!(rows(&groups[0]), [("AF eye in focus", "87%")]);
    }

    #[test]
    fn leaves_out_the_analysis_group_without_a_value() {
        let meta = sony();
        let none = analysis(None, None);
        let headings: Vec<_> = groups(Some(&meta), &[], &none)
            .iter()
            .map(|g| g.heading)
            .collect();
        assert_eq!(headings, [EXIF_HEADING, MAKER_NOTE_HEADING]);
        assert!(groups(None, &[], &none).is_empty());
    }

    #[test]
    fn shows_the_af_eye_in_focus_probability_as_a_percentage() {
        let percent = |p| value(&focus(Some(p), FocusCandidate::Unknown), "AF eye in focus");
        assert_eq!(percent(0.8701).as_deref(), Some("87%"));
        assert_eq!(percent(0.).as_deref(), Some("0%"));
        assert_eq!(
            shown(Some(&focus(Some(0.07), FocusCandidate::Unknown)), None),
            [("AF eye in focus", "7%".to_owned())]
        );
    }

    #[test]
    fn an_unknown_candidate_without_eye_focus_gives_no_group() {
        let analysis = analysis(Some(&focus(None, FocusCandidate::Unknown)), None);
        assert!(groups(None, &[], &analysis).is_empty());
    }

    #[test]
    fn shows_the_stored_eye_state_openness_and_head_pose() {
        let f = Focus {
            eye_focus: Some(0.87),
            ..with_eyes(StoredEyes::new(
                Some(0.09),
                pose(12.4, -5.2, 3.5),
                None,
                None,
                None,
            ))
        };
        let tauri = ["AF eye in focus", "Eyes", "Yaw", "Pitch", "Roll"];
        let rows: Vec<_> = shown(Some(&f), None)
            .into_iter()
            .filter(|(l, _)| tauri.contains(l))
            .collect();
        assert_eq!(
            rows,
            [
                ("AF eye in focus", "87%".to_owned()),
                ("Eyes", "Closed · 0".to_owned()),
                ("Yaw", "12°".to_owned()),
                ("Pitch", "-5°".to_owned()),
                ("Roll", "4°".to_owned()),
            ]
        );
    }

    #[test]
    fn shows_the_openness_of_the_ear() {
        let eyes = |ear| {
            let f = with_eyes(StoredEyes::new(Some(ear), None, None, None, None));
            value(&f, "Eyes")
        };
        assert_eq!(eyes(0.25).as_deref(), Some("Open · 41"));
        assert_eq!(eyes(0.3).as_deref(), Some("Open · 60"));
        assert_eq!(eyes(0.45).as_deref(), Some("Open · 100"));
    }

    #[test]
    fn rounds_the_head_pose_as_javascript_does() {
        let angles = |yaw, pitch, roll| {
            let f = with_eyes(StoredEyes {
                pose: pose(yaw, pitch, roll),
                ..StoredEyes::default()
            });
            ["Yaw", "Pitch", "Roll"].map(|l| value(&f, l).unwrap())
        };
        assert_eq!(angles(-110.2, 0., -1.6), ["-110°", "0°", "-2°"]);
        assert_eq!(angles(-0.4, -0.2, -0.49), ["0°", "0°", "0°"]);
        assert_eq!(angles(-5.5, 2.5, -0.5), ["-5°", "3°", "0°"]);
    }

    #[test]
    fn shows_the_full_size_eye_score_against_the_good_cut() {
        let full = |s| {
            let f = with_eyes(StoredEyes {
                full_eye_sharp: s,
                ..StoredEyes::default()
            });
            value(&f, "Eyes at full size")
        };
        assert_eq!(full(Some(0.143)).as_deref(), Some("Soft · 0.143"));
        assert_eq!(
            full(Some(GOOD_FULL_EYE_SHARP)).as_deref(),
            Some("Sharp · 0.162")
        );
        assert_eq!(full(None), None);
    }

    #[test]
    fn shows_the_focus_candidate_for_a_known_state() {
        let candidate = |c| value(&focus(None, c), "Focus candidate");
        assert_eq!(candidate(FocusCandidate::Candidate).as_deref(), Some("Yes"));
        assert_eq!(
            candidate(FocusCandidate::NotCandidate).as_deref(),
            Some("No")
        );
        assert_eq!(candidate(FocusCandidate::Unknown), None);
    }

    #[test]
    fn keeps_the_rows_in_order_with_every_stored_value() {
        let f = Focus {
            eye_focus: Some(0.87),
            candidate: FocusCandidate::Candidate,
            ..with_eyes(StoredEyes::new(
                Some(0.25),
                pose(12.4, -5.2, 0.),
                Some(0.0314),
                Some(-0.02),
                Some(0.2),
            ))
        };
        let closed = percent(f.eyes.eyes_closed.unwrap());
        assert_eq!(
            shown(Some(&f), Some(123.45)),
            [
                ("AF eye in focus", "87%".to_owned()),
                ("Focus candidate", "Yes".to_owned()),
                ("Eyes at full size", "Sharp · 0.200".to_owned()),
                ("Eyes", "Open · 41".to_owned()),
                ("Eyes closed", closed),
                ("Eye aspect ratio", "0.250".to_owned()),
                ("Yaw", "12°".to_owned()),
                ("Pitch", "-5°".to_owned()),
                ("Roll", "0°".to_owned()),
                ("Mesh eye offset", "0.031".to_owned()),
                ("Edge gap", "-0.020".to_owned()),
                ("Sharpness", "123.5".to_owned()),
            ]
        );
    }

    #[test]
    fn formats_the_closed_probability_like_the_in_focus_one() {
        let f = with_eyes(StoredEyes {
            eyes_closed: Some(0.125),
            ..StoredEyes::default()
        });
        assert_eq!(value(&f, "Eyes closed").as_deref(), Some("13%"));
    }

    /// `toFixed(3)` takes an exact tie up (`0.063`), `{:.3}` to even. Only
    /// exact dyadic ties differ, which a measured value practically never is.
    #[test]
    fn an_exact_tie_rounds_to_even_unlike_to_fixed() {
        let f = with_eyes(StoredEyes {
            eye_offset: Some(0.0625),
            ..StoredEyes::default()
        });
        assert_eq!(value(&f, "Mesh eye offset").as_deref(), Some("0.062"));
    }

    #[test]
    fn leaves_out_each_new_row_without_its_value() {
        let f = focus(Some(0.5), FocusCandidate::Unknown);
        for label in [
            "Focus candidate",
            "Eyes closed",
            "Eye aspect ratio",
            "Mesh eye offset",
            "Edge gap",
            "Sharpness",
        ] {
            assert_eq!(value(&f, label), None, "{label}");
        }
    }

    #[test]
    fn a_sharpness_alone_shows_only_the_sharpness() {
        assert_eq!(shown(None, Some(98.04)), [("Sharpness", "98.0".to_owned())]);
    }

    #[test]
    fn no_value_or_only_an_af_point_gives_no_analysis_group() {
        assert!(shown(None, None).is_empty());
        let point = Focus {
            point: Some(POINT),
            ..focus(None, FocusCandidate::Unknown)
        };
        let analysis = analysis(Some(&point), None);
        assert!(groups(None, &[], &analysis).is_empty());
    }

    #[test]
    fn af_rows_give_the_point_and_the_recorded_frame() {
        let at = ("AF point", Some("3120, 2080 of 8640 × 5760".to_owned()));
        assert_eq!(
            af_rows(Some(&POINT)),
            [at.clone(), ("AF frame", Some("400 × 300".to_owned()))]
        );
        let no_frame = FocusPoint {
            frame: None,
            ..POINT
        };
        assert_eq!(af_rows(Some(&no_frame)), [at, ("AF frame", None)]);
        assert!(af_rows(None).is_empty());
    }

    #[test]
    fn the_af_rows_sit_between_af_tracking_and_drive() {
        let meta = sony();
        let af = af_rows(Some(&POINT));
        let groups = groups(Some(&meta), &af, &[]);
        assert_eq!(groups[1].heading, MAKER_NOTE_HEADING);
        let labels: Vec<_> = groups[1].rows.iter().map(|r| r.label).collect();
        assert_eq!(
            labels[3..7],
            ["AF tracking", "AF point", "AF frame", "Drive"]
        );
    }

    #[test]
    fn the_af_rows_show_without_metadata_before_the_analysis() {
        let af = af_rows(Some(&POINT));
        let analysis = analysis(None, Some(1.));
        let groups = groups(None, &af, &analysis);
        let headings: Vec<_> = groups.iter().map(|g| g.heading).collect();
        assert_eq!(headings, [MAKER_NOTE_HEADING, ANALYSIS_HEADING]);
        assert_eq!(
            rows(&groups[0]),
            [
                ("AF point", "3120, 2080 of 8640 × 5760"),
                ("AF frame", "400 × 300")
            ]
        );
    }

    #[test]
    fn the_af_rows_alone_give_the_maker_note_group() {
        let meta = Metadata {
            iso: Some("ISO 100".into()),
            ..empty()
        };
        let headings = |af: &[(&str, Option<String>)]| -> Vec<&str> {
            groups(Some(&meta), af, &[])
                .iter()
                .map(|g| g.heading)
                .collect()
        };
        assert_eq!(
            headings(&af_rows(Some(&POINT))),
            [EXIF_HEADING, MAKER_NOTE_HEADING]
        );
        assert_eq!(headings(&[]), [EXIF_HEADING]);
    }
}
