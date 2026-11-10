//! The faces and the eye judgment of the file being shown, read on demand
//! rather than at scan time: neither touches the index or `ReadSlots`.

use std::path::Path;

use crate::index::EyesPose;

/// The faces `faces_of` found, each with its YuNet score and its five
/// landmarks (the eyes, the nose tip and the mouth corners, each pair left of
/// the image first), in the preview's stored pixel coordinates.
#[derive(Debug, serde::Serialize)]
pub struct FacesResponse {
    pub width: usize,
    pub height: usize,
    pub faces: Vec<FaceBox>,
}

#[derive(Debug, serde::Serialize)]
pub struct FaceBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub score: f32,
    pub left_eye: FacePoint,
    pub right_eye: FacePoint,
    pub nose: FacePoint,
    pub mouth_left: FacePoint,
    pub mouth_right: FacePoint,
}

#[derive(Debug, serde::Serialize)]
pub struct FacePoint {
    pub x: f32,
    pub y: f32,
}

impl From<(f32, f32)> for FacePoint {
    fn from((x, y): (f32, f32)) -> Self {
        FacePoint { x, y }
    }
}

/// Refuses `what` on a file that is not a RAW file: a JPEG folder is
/// view-only, so no client can rate it, write a sidecar next to it or run
/// face detection on its full-size image.
pub fn raw_only(path: &Path, what: &str) -> Result<(), String> {
    if riffle_core::scan::is_raw_file(path) {
        Ok(())
    } else {
        Err(format!(
            "{}: {what} applies to RAW files only",
            path.display()
        ))
    }
}

/// Detect the faces of a file's preview through the scan's own
/// `detect_around`, so the region and the threshold match the faces the
/// focus candidate cue picks from, for display only.
pub fn read_faces(path: &Path) -> Result<FacesResponse, String> {
    raw_only(path, "face detection")?;
    let (arw, jpeg) =
        riffle_core::reader::read_preview(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let found = riffle_core::faces::detect_around(
        &jpeg,
        arw.orientation,
        riffle_core::sharpness::trusted_focus(&arw.shot),
    )
    .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(FacesResponse {
        width: found.width,
        height: found.height,
        faces: found
            .faces
            .iter()
            .map(|f| FaceBox {
                x: f.x,
                y: f.y,
                width: f.width,
                height: f.height,
                score: f.score,
                left_eye: f.left_eye.into(),
                right_eye: f.right_eye.into(),
                nose: f.nose.into(),
                mouth_left: f.mouth_left.into(),
                mouth_right: f.mouth_right.into(),
            })
            .collect(),
    })
}

/// The `eyes_of` answer: the judgment, or `None` when the eyes are unknown
/// (no face, a face below the floor, a model failure), whether a newer
/// request superseded this one before it finished, and how long each stage
/// took, in microseconds, for the `eyes` timing line.
#[derive(Debug, Default, serde::Serialize)]
pub struct EyesResponse {
    pub eyes: Option<EyesJudgment>,
    pub superseded: bool,
    pub read_us: u32,
    pub decode_us: u32,
    pub detect_us: u32,
    pub model_us: u32,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct EyesJudgment {
    pub state: &'static str,
    /// The probability that the eyes are closed, 0..1.
    pub probability: f64,
    /// The EAR of the more closed eye the judgment was read from.
    pub ear: f64,
    /// The head pose of the judged face (`pose::Pose`), `None` when the
    /// solve failed.
    pub pose: Option<EyesPose>,
    pub mesh: EyesMesh,
}

/// The face mesh the judgment was taken on, in the preview's stored pixel
/// coordinates like `FacesResponse`.
#[derive(Debug, PartialEq, serde::Serialize)]
pub struct EyesMesh {
    pub width: usize,
    pub height: usize,
    pub points: Vec<[f32; 2]>,
}

fn micros(since: std::time::Instant) -> u32 {
    since.elapsed().as_micros() as u32
}

/// Judge whether the eyes of a file's AF face (else its largest confident
/// face, `eyes::judged_face`) are closed, on the faces `read_faces` finds.
/// With an AF point, the one full-size decode serves both the detection and
/// the face crop; without one, the detection runs on `detect_around`'s
/// scaled whole-image decode, so the face is one the focus mark draws, and
/// the crop comes from a second, full-size decode. `current` is asked
/// before the detection and before the model, and a `false` stops the
/// judgment as superseded.
pub fn read_eyes(path: &Path, current: impl Fn() -> bool) -> Result<EyesResponse, String> {
    use riffle_core::{decode, eyes, faces};
    raw_only(path, "eye state judgment")?;
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    let mut out = EyesResponse::default();
    let superseded = |mut out: EyesResponse| {
        out.superseded = true;
        Ok(out)
    };
    let t = std::time::Instant::now();
    let (arw, jpeg) = riffle_core::reader::read_preview(path).map_err(|e| err(&e))?;
    out.read_us = micros(t);
    let orientation = arw.orientation;
    let focus = riffle_core::sharpness::trusted_focus(&arw.shot);
    let t = std::time::Instant::now();
    let (found, full) = if focus.is_some() {
        let (rgb, w, h) = decode::decode_rgb(&jpeg).map_err(|e| err(&e))?;
        out.decode_us = micros(t);
        if !current() {
            return superseded(out);
        }
        let t = std::time::Instant::now();
        let found =
            faces::detect_around_rgb(&rgb, w, h, orientation, focus).map_err(|e| err(&e))?;
        out.detect_us = micros(t);
        (found, Some((rgb, w, h)))
    } else {
        let whole = faces::decode_whole(&jpeg, orientation).map_err(|e| err(&e))?;
        out.decode_us = micros(t);
        if !current() {
            return superseded(out);
        }
        let t = std::time::Instant::now();
        let found = faces::detect_whole_upright(&whole, orientation).map_err(|e| err(&e))?;
        out.detect_us = micros(t);
        (found, None)
    };
    let Some(&stored) = eyes::judged_face(&found.faces, found.point) else {
        return Ok(out);
    };
    if !current() {
        return superseded(out);
    }
    let (rgb, w, h) = match full {
        Some(full) => full,
        None => {
            let t = std::time::Instant::now();
            let full = decode::decode_rgb(&jpeg).map_err(|e| err(&e))?;
            out.decode_us += micros(t);
            full
        }
    };
    let t = std::time::Instant::now();
    let (upright, uw, uh) = faces::upright_rgb(&rgb, w, h, orientation);
    let face = faces::face_to_upright(stored, orientation, w, h);
    out.eyes = eyes::judge_mesh(&upright, uw, uh, &face).map(|j| EyesJudgment {
        state: match j.eyes.state {
            eyes::EyeState::Open => "open",
            eyes::EyeState::Closed => "closed",
        },
        probability: j.eyes.probability,
        ear: j.ear,
        pose: j.pose.map(EyesPose::from),
        mesh: EyesMesh {
            width: w,
            height: h,
            points: j
                .points
                .into_iter()
                .map(|p| {
                    let (x, y) = faces::point_to_stored((p[0], p[1]), orientation, w, h);
                    [x, y]
                })
                .collect(),
        },
    });
    out.model_us = micros(t);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("riffle-ondemand-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn remove_temp_dir(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn gradient_jpeg(w: usize, h: usize) -> Vec<u8> {
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

    /// A TIFF shell whose IFD0 carries an Orientation and a preview of `body`,
    /// as `riffle_core::scan`'s tests build one.
    fn arw_with_preview(orientation: u16, body: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"II\x2a\x00");
        buf.extend_from_slice(&8u32.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes());
        let at = 8 + 2 + 3 * 12 + 4;
        for (tag, ty, value) in [
            (0x0112u16, 3u16, orientation as u32),
            (0x0201, 4, at as u32),
            (0x0202, 4, body.len() as u32),
        ] {
            buf.extend_from_slice(&tag.to_le_bytes());
            buf.extend_from_slice(&ty.to_le_bytes());
            buf.extend_from_slice(&1u32.to_le_bytes());
            buf.extend_from_slice(&value.to_le_bytes());
        }
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(body);
        buf
    }

    #[test]
    fn faces_come_back_in_the_stored_preview_size() {
        let dir = temp_dir("faces");
        let path = dir.join("a.arw");
        std::fs::write(&path, arw_with_preview(6, &gradient_jpeg(64, 48))).unwrap();

        let found = read_faces(&path).unwrap();
        assert_eq!((found.width, found.height), (64, 48));
        assert!(found.faces.is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn eyes_without_a_face_are_unknown_and_a_jpeg_is_refused() {
        let dir = temp_dir("eyes");
        let path = dir.join("a.arw");
        std::fs::write(&path, arw_with_preview(6, &gradient_jpeg(64, 48))).unwrap();

        let judged = read_eyes(&path, || true).unwrap();
        assert_eq!(judged.eyes, None);
        assert!(!judged.superseded);
        let jpeg = dir.join("a.jpg");
        std::fs::write(&jpeg, gradient_jpeg(64, 48)).unwrap();
        let err = read_eyes(&jpeg, || true).unwrap_err();
        assert!(err.contains("RAW files only"), "{err}");

        remove_temp_dir(&dir);
    }

    #[test]
    fn the_eyes_judgment_carries_the_pose_or_null() {
        let judgment = |pose| EyesJudgment {
            state: "open",
            probability: 0.25,
            ear: 0.2,
            pose,
            mesh: EyesMesh {
                width: 4,
                height: 3,
                points: vec![[1.0, 2.0]],
            },
        };
        let pose = riffle_core::pose::Pose {
            yaw: 12.5,
            pitch: -5.25,
            roll: 3.0,
        };
        assert_eq!(
            serde_json::to_value(judgment(Some(pose.into()))).unwrap()["pose"],
            serde_json::json!({ "yaw": 12.5, "pitch": -5.25, "roll": 3.0 })
        );
        assert_eq!(
            serde_json::to_value(judgment(None)).unwrap()["pose"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn a_superseded_eyes_request_stops_before_the_detection() {
        let dir = temp_dir("eyes-superseded");
        let path = dir.join("a.arw");
        std::fs::write(&path, arw_with_preview(1, &gradient_jpeg(64, 48))).unwrap();

        let judged = read_eyes(&path, || false).unwrap();
        assert!(judged.superseded);
        assert_eq!((judged.eyes, judged.detect_us), (None, 0));

        remove_temp_dir(&dir);
    }

    /// The `eyes_of` breakdown on real files, one line per file: set
    /// `RIFFLE_EYES_FILES` to RAW paths separated by `;`, then run in
    /// release with `--ignored --nocapture`. The first file pays the model
    /// builds.
    #[test]
    #[ignore]
    fn times_eyes_of_on_real_files() {
        let paths = std::env::var("RIFFLE_EYES_FILES").expect("RIFFLE_EYES_FILES");
        for path in paths.split(';').filter(|p| !p.is_empty()) {
            let started = std::time::Instant::now();
            let r = read_eyes(Path::new(path), || true).unwrap();
            println!(
                "{path}  read {:.1}  decode {:.1}  detect {:.1}  model {:.1}  total {:.1} ms  {:?}",
                r.read_us as f64 / 1000.0,
                r.decode_us as f64 / 1000.0,
                r.detect_us as f64 / 1000.0,
                r.model_us as f64 / 1000.0,
                started.elapsed().as_secs_f64() * 1000.0,
                r.eyes.map(|e| (e.state, e.probability))
            );
        }
    }

    #[test]
    fn a_preview_that_is_not_a_jpeg_is_a_faces_error() {
        let dir = temp_dir("faces-broken");
        let path = dir.join("a.arw");
        std::fs::write(&path, arw_with_preview(1, b"not a jpeg")).unwrap();

        assert!(read_faces(&path).is_err());

        remove_temp_dir(&dir);
    }
}
