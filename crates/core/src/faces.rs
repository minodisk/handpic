//! Offline face and eye detection on the embedded preview with the YuNet
//! model (OpenCV Zoo, `face_detection_yunet_2023mar.onnx`, MIT), run through
//! `tract`. The image is scaled to fit the model input (`INPUT` x `INPUT`
//! for a crop, `WHOLE_INPUT` for a whole preview) and padded into it; the
//! per-stride outputs are decoded as OpenCV's `FaceDetectorYN` does,
//! thresholded and merged by NMS. Results are in the caller's pixel
//! coordinates.
//!
//! `detect_around` is the one place that decides which region of a preview
//! is searched (a `CATCH_CROP` square around a trusted AF point, else the
//! whole upright image), at which model input, and at which decode size (the
//! full preview for the crop, a DCT-scaled one for the whole image).

use std::io::Cursor;
use std::sync::OnceLock;

use anyhow::{anyhow, Context, Result};
use tract_onnx::prelude::*;

use crate::arw::FocusLocation;
use crate::decode::{apply_orientation, decode_rgb, decode_upright_near, Upright};
use crate::partial::focus_point;
use crate::sharpness::{window_at, Window};

const MODEL: &[u8] = include_bytes!("../models/face_detection_yunet_2023mar.onnx");

/// Side of the square model input of `detect` and the `CATCH_CROP` search,
/// in pixels.
pub const INPUT: usize = 320;
/// Model input (width, height) of the whole-image search of a landscape
/// image, in pixels; a portrait image gets it transposed. A 2112 px preview
/// shrinks 3.3x instead of 6.6x, so a 55 px face stays above YuNet's stride-8
/// minimum, and the 3:2 shape spares the third of a square input a 3:2
/// preview would leave as padding.
pub const WHOLE_INPUT: (usize, usize) = (640, 448);
/// Minimum detection score kept.
pub const SCORE_THRESHOLD: f32 = 0.6;
/// Boxes overlapping a better one by more than this IoU are dropped.
pub const NMS_THRESHOLD: f32 = 0.3;
const STRIDES: [usize; 3] = [8, 16, 32];
/// Side of the square cut out of the upright preview around the AF point,
/// in pixels. Native resolution keeps a 45-60 px face large enough for the
/// `INPUT` model input.
pub const CATCH_CROP: usize = 480;
/// Minimum detection score of a face `detect_around` returns.
pub const CATCH_CONFIDENCE: f32 = SCORE_THRESHOLD;

/// What `detect_around` found, in the preview's stored pixel coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// The stored (unrotated) preview size.
    pub width: usize,
    pub height: usize,
    pub faces: Vec<Face>,
    /// The AF point, when one was given.
    pub point: Option<(usize, usize)>,
}

/// A detected face, in the pixel coordinates of the image given to `detect`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub score: f32,
    /// The eye on the left of the image, then the one on the right.
    pub left_eye: (f32, f32),
    pub right_eye: (f32, f32),
    pub nose: (f32, f32),
    /// The mouth corner on the left of the image, then the one on the right.
    pub mouth_left: (f32, f32),
    pub mouth_right: (f32, f32),
}

type Plan = Arc<TypedRunnableModel>;

/// The model inputs, one optimized plan each.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Input {
    Square,
    Landscape,
    Portrait,
}

impl Input {
    /// The whole-image input for an upright `width` x `height` image.
    fn whole(width: usize, height: usize) -> Self {
        if width >= height {
            Input::Landscape
        } else {
            Input::Portrait
        }
    }

    /// (width, height) in pixels.
    fn size(self) -> (usize, usize) {
        match self {
            Input::Square => (INPUT, INPUT),
            Input::Landscape => WHOLE_INPUT,
            Input::Portrait => (WHOLE_INPUT.1, WHOLE_INPUT.0),
        }
    }
}

struct Detector {
    plan: Plan,
    /// Output index of `cls`, `obj`, `bbox`, `kps` for each stride.
    outputs: [[usize; 4]; 3],
}

fn detector(input: Input) -> Result<&'static Detector> {
    static DETECTORS: [OnceLock<std::result::Result<Detector, String>>; 3] =
        [const { OnceLock::new() }; 3];
    DETECTORS[input as usize]
        .get_or_init(|| build(input.size()).map_err(|e| format!("{e:#}")))
        .as_ref()
        .map_err(|e| anyhow!("face model: {e}"))
}

fn build((width, height): (usize, usize)) -> Result<Detector> {
    // The file annotates intermediate and output shapes for a 640 px input.
    let model = tract_onnx::onnx()
        .with_ignore_value_info(true)
        .with_ignore_output_shapes(true)
        .model_for_read(&mut Cursor::new(MODEL))?
        .with_input_fact(0, f32::fact([1, 3, height, width]).into())?;
    let names: Vec<String> = model
        .output_outlets()?
        .iter()
        .map(|o| model.outlet_label(*o).unwrap_or_default().to_string())
        .collect();
    let mut outputs = [[0; 4]; 3];
    for (s, stride) in STRIDES.iter().enumerate() {
        for (k, kind) in ["cls", "obj", "bbox", "kps"].iter().enumerate() {
            let name = format!("{kind}_{stride}");
            outputs[s][k] = names
                .iter()
                .position(|n| *n == name)
                .with_context(|| format!("no output {name} in {names:?}"))?;
        }
    }
    let plan = model.into_optimized()?.into_runnable()?;
    Ok(Detector { plan, outputs })
}

/// Detect faces in an RGB image (`width` x `height`, 3 bytes per pixel) at
/// the `INPUT` x `INPUT` model input, best first.
///
/// YuNet is trained on upright faces: the caller must pass an image already
/// rotated to display orientation (e.g. via `decode::apply_orientation`),
/// or recall drops sharply on portrait frames.
pub fn detect(rgb: &[u8], width: usize, height: usize) -> Result<Vec<Face>> {
    detect_at(rgb, width, height, Input::Square)
}

/// `detect` at the `WHOLE_INPUT` model input the whole-image search uses.
pub fn detect_whole(rgb: &[u8], width: usize, height: usize) -> Result<Vec<Face>> {
    detect_at(rgb, width, height, Input::whole(width, height))
}

fn detect_at(rgb: &[u8], width: usize, height: usize, input: Input) -> Result<Vec<Face>> {
    if width == 0 || height == 0 || rgb.len() < width * height * 3 {
        return Ok(Vec::new());
    }
    let d = detector(input)?;
    let size = input.size();
    let scale = fit(width, height, size);
    let input = input_tensor(rgb, width, height, scale, size);
    let out = d.plan.run(tvec!(input.into()))?;
    let mut faces = Vec::new();
    for (s, &stride) in STRIDES.iter().enumerate() {
        let get = |k: usize| -> Result<&[f32]> {
            out[d.outputs[s][k]].try_as_plain_ram()?.as_slice::<f32>()
        };
        decode_stride(
            stride,
            size.0 / stride,
            get(0)?,
            get(1)?,
            get(2)?,
            get(3)?,
            &mut faces,
        );
    }
    let mut faces = nms(faces, NMS_THRESHOLD);
    for f in &mut faces {
        f.x /= scale;
        f.y /= scale;
        f.width /= scale;
        f.height /= scale;
        f.left_eye = (f.left_eye.0 / scale, f.left_eye.1 / scale);
        f.right_eye = (f.right_eye.0 / scale, f.right_eye.1 / scale);
        f.nose = (f.nose.0 / scale, f.nose.1 / scale);
        f.mouth_left = (f.mouth_left.0 / scale, f.mouth_left.1 / scale);
        f.mouth_right = (f.mouth_right.0 / scale, f.mouth_right.1 / scale);
    }
    Ok(faces)
}

/// The scale that fits a `width` x `height` image inside a model input of
/// `(width, height)`.
fn fit(width: usize, height: usize, (iw, ih): (usize, usize)) -> f32 {
    (iw as f32 / width as f32).min(ih as f32 / height as f32)
}

/// Box-average the image down by `scale` into the top-left of a zeroed BGR
/// NCHW tensor of the model input's `(width, height)`, raw 0..255 values, as
/// YuNet expects.
fn input_tensor(
    rgb: &[u8],
    width: usize,
    height: usize,
    scale: f32,
    (iw, ih): (usize, usize),
) -> Tensor {
    let ow = ((width as f32 * scale).round() as usize).clamp(1, iw);
    let oh = ((height as f32 * scale).round() as usize).clamp(1, ih);
    let plane = iw * ih;
    let mut data = vec![0f32; 3 * plane];
    for oy in 0..oh {
        let y0 = oy * height / oh;
        let y1 = ((oy + 1) * height / oh).max(y0 + 1);
        for ox in 0..ow {
            let x0 = ox * width / ow;
            let x1 = ((ox + 1) * width / ow).max(x0 + 1);
            let mut sum = [0u32; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * width + x) * 3;
                    sum[0] += rgb[i] as u32;
                    sum[1] += rgb[i + 1] as u32;
                    sum[2] += rgb[i + 2] as u32;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as f32;
            let o = oy * iw + ox;
            data[o] = sum[2] as f32 / n;
            data[plane + o] = sum[1] as f32 / n;
            data[2 * plane + o] = sum[0] as f32 / n;
        }
    }
    tract_ndarray::Array4::from_shape_vec((1, 3, ih, iw), data)
        .expect("shape matches the buffer")
        .into()
}

/// Decode one stride's grid (`cols` cells per row) into faces above
/// `SCORE_THRESHOLD`, in model-input coordinates.
fn decode_stride(
    stride: usize,
    cols: usize,
    cls: &[f32],
    obj: &[f32],
    bbox: &[f32],
    kps: &[f32],
    faces: &mut Vec<Face>,
) {
    let s = stride as f32;
    for i in 0..cls.len().min(obj.len()) {
        let score = (cls[i].clamp(0.0, 1.0) * obj[i].clamp(0.0, 1.0)).sqrt();
        if score < SCORE_THRESHOLD {
            continue;
        }
        let (c, r) = ((i % cols) as f32, (i / cols) as f32);
        let b = &bbox[i * 4..i * 4 + 4];
        let (cx, cy) = ((c + b[0]) * s, (r + b[1]) * s);
        let (w, h) = (b[2].exp() * s, b[3].exp() * s);
        let k = &kps[i * 10..i * 10 + 10];
        let at = |j: usize| ((k[j * 2] + c) * s, (k[j * 2 + 1] + r) * s);
        let leftmost = |a: (f32, f32), e: (f32, f32)| if a.0 <= e.0 { (a, e) } else { (e, a) };
        let (left_eye, right_eye) = leftmost(at(0), at(1));
        let (mouth_left, mouth_right) = leftmost(at(3), at(4));
        faces.push(Face {
            x: cx - w / 2.0,
            y: cy - h / 2.0,
            width: w,
            height: h,
            score,
            left_eye,
            right_eye,
            nose: at(2),
            mouth_left,
            mouth_right,
        });
    }
}

fn iou(a: &Face, b: &Face) -> f32 {
    let w = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let h = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    if w <= 0.0 || h <= 0.0 {
        return 0.0;
    }
    let inter = w * h;
    inter / (a.width * a.height + b.width * b.height - inter)
}

/// Greedy non-maximum suppression, best score first.
fn nms(mut faces: Vec<Face>, threshold: f32) -> Vec<Face> {
    faces.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut kept: Vec<Face> = Vec::new();
    for f in faces {
        if kept.iter().all(|k| iou(k, &f) <= threshold) {
            kept.push(f);
        }
    }
    kept
}

/// Map a face found on the upright image back to the image as stored, which
/// is `width` x `height` before the Orientation tag is applied: 6 and 8 are
/// quarter turns, 3 a half turn, anything else is the identity.
pub fn to_stored(face: Face, orientation: u16, width: usize, height: usize) -> Face {
    let point = |p| point_to_stored(p, orientation, width, height);
    let a = point((face.x, face.y));
    let b = point((face.x + face.width, face.y + face.height));
    Face {
        x: a.0.min(b.0),
        y: a.1.min(b.1),
        width: (a.0 - b.0).abs(),
        height: (a.1 - b.1).abs(),
        score: face.score,
        left_eye: point(face.left_eye),
        right_eye: point(face.right_eye),
        nose: point(face.nose),
        mouth_left: point(face.mouth_left),
        mouth_right: point(face.mouth_right),
    }
}

/// Map a point on the upright image back to the stored `width` x `height`
/// image, as `to_stored` maps a face.
pub fn point_to_stored(
    (x, y): (f32, f32),
    orientation: u16,
    width: usize,
    height: usize,
) -> (f32, f32) {
    let (w, h) = (width as f32, height as f32);
    match orientation {
        6 => (y, h - x),
        8 => (w - y, x),
        3 => (w - x, h - y),
        _ => (x, y),
    }
}

/// Map a pixel of the stored `width` x `height` image to the upright image,
/// the inverse of `to_stored`.
pub fn to_upright(
    (x, y): (usize, usize),
    orientation: u16,
    width: usize,
    height: usize,
) -> (usize, usize) {
    match orientation {
        6 => (height - 1 - y, x),
        8 => (y, width - 1 - x),
        3 => (width - 1 - x, height - 1 - y),
        _ => (x, y),
    }
}

/// Map a face found on the stored `width` x `height` preview to the upright
/// one, the inverse of `to_stored`.
pub fn face_to_upright(face: Face, orientation: u16, width: usize, height: usize) -> Face {
    let (w, h) = (width as f32, height as f32);
    let point = |(x, y): (f32, f32)| match orientation {
        6 => (h - y, x),
        8 => (y, w - x),
        3 => (w - x, h - y),
        _ => (x, y),
    };
    let a = point((face.x, face.y));
    let b = point((face.x + face.width, face.y + face.height));
    Face {
        x: a.0.min(b.0),
        y: a.1.min(b.1),
        width: (a.0 - b.0).abs(),
        height: (a.1 - b.1).abs(),
        score: face.score,
        left_eye: point(face.left_eye),
        right_eye: point(face.right_eye),
        nose: point(face.nose),
        mouth_left: point(face.mouth_left),
        mouth_right: point(face.mouth_right),
    }
}

/// Rotate a stored RGB image to display orientation. `apply_orientation`
/// leaves orientation 3 alone, so the half turn is done here.
pub fn upright_rgb(
    rgb: &[u8],
    width: usize,
    height: usize,
    orientation: u16,
) -> (Vec<u8>, usize, usize) {
    let (mut upright, w, h) = apply_orientation(rgb, width, height, orientation);
    if orientation == 3 {
        upright = upright.chunks_exact(3).rev().flatten().copied().collect();
    }
    (upright, w, h)
}

/// Cut a `size` x `size` square centered on `(cx, cy)` out of an RGB image,
/// clamped inside it (the whole image when it is smaller), with the window
/// it came from.
pub fn crop_rgb(
    rgb: &[u8],
    width: usize,
    height: usize,
    (cx, cy): (usize, usize),
    size: usize,
) -> (Vec<u8>, Window) {
    let win = window_at(width, height, cx, cy, size);
    let mut out = Vec::with_capacity(win.width * win.height * 3);
    for y in win.y..win.y + win.height {
        let row = (y * width + win.x) * 3;
        out.extend_from_slice(&rgb[row..row + win.width * 3]);
    }
    (out, win)
}

fn translate(face: Face, dx: f32, dy: f32) -> Face {
    Face {
        x: face.x + dx,
        y: face.y + dy,
        left_eye: (face.left_eye.0 + dx, face.left_eye.1 + dy),
        right_eye: (face.right_eye.0 + dx, face.right_eye.1 + dy),
        nose: (face.nose.0 + dx, face.nose.1 + dy),
        mouth_left: (face.mouth_left.0 + dx, face.mouth_left.1 + dy),
        mouth_right: (face.mouth_right.0 + dx, face.mouth_right.1 + dy),
        ..face
    }
}

/// Decode `preview`, rotate it upright and detect faces once: in a
/// `CATCH_CROP` square around the AF point of `focus` at the `INPUT` model
/// input on the full-size decode, or, when `focus` is `None`, on the whole
/// image at `WHOLE_INPUT`, decoded at the DCT scale `decode_whole` picks.
/// Faces below `CATCH_CONFIDENCE` are dropped.
pub fn detect_around(
    preview: &[u8],
    orientation: u16,
    focus: Option<FocusLocation>,
) -> Result<Detection> {
    if focus.is_none() {
        return detect_whole_upright(&decode_whole(preview, orientation)?, orientation);
    }
    let (rgb, width, height) = decode_rgb(preview)?;
    detect_around_rgb(&rgb, width, height, orientation, focus)
}

/// Decode `preview` upright for the whole-image search: at the smallest
/// `n/8` scale whose long edge is not below `WHOLE_INPUT`'s, so the model
/// input is still box-averaged down from at least its own size.
pub fn decode_whole(preview: &[u8], orientation: u16) -> Result<Upright> {
    decode_upright_near(preview, orientation, WHOLE_INPUT.0)
}

/// Detect faces on the whole of an image `decode_whole` returned, at
/// `WHOLE_INPUT`, in the stored preview's pixel coordinates. Faces below
/// `CATCH_CONFIDENCE` are dropped.
pub fn detect_whole_upright(image: &Upright, orientation: u16) -> Result<Detection> {
    let (width, height) = image.stored;
    let faces = detect_whole(&image.rgb, image.width, image.height)?
        .into_iter()
        .filter(|f| f.score >= CATCH_CONFIDENCE)
        .map(|f| scaled_to_stored(f, orientation, (image.width, image.height), image.stored))
        .collect();
    Ok(Detection {
        width,
        height,
        faces,
        point: None,
    })
}

/// Map a face found on an upright image decoded at a reduced size
/// (`scaled`, upright) back to the stored full-size image (`stored`,
/// unrotated). Each axis is scaled by the ratio of the actual sizes, since
/// mozjpeg rounds a scaled side up.
fn scaled_to_stored(
    face: Face,
    orientation: u16,
    (sw, sh): (usize, usize),
    (width, height): (usize, usize),
) -> Face {
    let (uw, uh) = if matches!(orientation, 6 | 8) {
        (height, width)
    } else {
        (width, height)
    };
    let (kx, ky) = (uw as f32 / sw as f32, uh as f32 / sh as f32);
    let full = Face {
        x: face.x * kx,
        y: face.y * ky,
        width: face.width * kx,
        height: face.height * ky,
        left_eye: (face.left_eye.0 * kx, face.left_eye.1 * ky),
        right_eye: (face.right_eye.0 * kx, face.right_eye.1 * ky),
        nose: (face.nose.0 * kx, face.nose.1 * ky),
        mouth_left: (face.mouth_left.0 * kx, face.mouth_left.1 * ky),
        mouth_right: (face.mouth_right.0 * kx, face.mouth_right.1 * ky),
        ..face
    };
    to_stored(full, orientation, width, height)
}

/// `detect_around` on a preview already decoded to stored RGB
/// (`width` x `height`, before the Orientation tag is applied). Without
/// `focus` it searches this full-size image, not the scaled decode
/// `detect_around` uses.
pub fn detect_around_rgb(
    rgb: &[u8],
    width: usize,
    height: usize,
    orientation: u16,
    focus: Option<FocusLocation>,
) -> Result<Detection> {
    let (upright, uw, uh) = upright_rgb(rgb, width, height, orientation);
    let point = focus.map(|f| focus_point(width, height, Some(f)));
    let found = match point {
        Some(p) => {
            let center = to_upright(p, orientation, width, height);
            let (sub, win) = crop_rgb(&upright, uw, uh, center, CATCH_CROP);
            detect(&sub, win.width, win.height)?
                .into_iter()
                .map(|f| translate(f, win.x as f32, win.y as f32))
                .collect()
        }
        None => detect_whole(&upright, uw, uh)?,
    };
    let faces = found
        .into_iter()
        .filter(|f| f.score >= CATCH_CONFIDENCE)
        .map(|f| to_stored(f, orientation, width, height))
        .collect();
    Ok(Detection {
        width,
        height,
        faces,
        point,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(x: f32, score: f32) -> Face {
        Face {
            x,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            score,
            left_eye: (0.0, 0.0),
            right_eye: (0.0, 0.0),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        }
    }

    fn upright() -> Face {
        Face {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
            score: 0.9,
            left_eye: (15.0, 25.0),
            right_eye: (35.0, 26.0),
            nose: (25.0, 35.0),
            mouth_left: (18.0, 48.0),
            mouth_right: (32.0, 49.0),
        }
    }

    #[test]
    fn maps_an_upright_face_back_to_the_stored_image() {
        // Stored 200x100; upright is 100x200 for 6/8 and 200x100 for 3.
        let f = to_stored(upright(), 6, 200, 100);
        assert_eq!((f.x, f.y, f.width, f.height), (20.0, 60.0, 40.0, 30.0));
        assert_eq!((f.left_eye, f.right_eye), ((25.0, 85.0), (26.0, 65.0)));
        assert_eq!(f.nose, (35.0, 75.0));
        assert_eq!((f.mouth_left, f.mouth_right), ((48.0, 82.0), (49.0, 68.0)));
        let f = to_stored(upright(), 8, 200, 100);
        assert_eq!((f.x, f.y, f.width, f.height), (140.0, 10.0, 40.0, 30.0));
        assert_eq!((f.left_eye, f.right_eye), ((175.0, 15.0), (174.0, 35.0)));
        assert_eq!(f.nose, (165.0, 25.0));
        assert_eq!(
            (f.mouth_left, f.mouth_right),
            ((152.0, 18.0), (151.0, 32.0))
        );
        let f = to_stored(upright(), 3, 200, 100);
        assert_eq!((f.x, f.y, f.width, f.height), (160.0, 40.0, 30.0, 40.0));
        assert_eq!((f.left_eye, f.right_eye), ((185.0, 75.0), (165.0, 74.0)));
        assert_eq!(f.nose, (175.0, 65.0));
        assert_eq!(
            (f.mouth_left, f.mouth_right),
            ((182.0, 52.0), (168.0, 51.0))
        );
        assert_eq!(to_stored(upright(), 1, 200, 100), upright());
    }

    #[test]
    fn face_to_upright_is_the_inverse_of_to_stored() {
        for orientation in [1, 3, 6, 8] {
            let stored = to_stored(upright(), orientation, 200, 100);
            let back = face_to_upright(stored, orientation, 200, 100);
            assert_eq!(back, upright(), "orientation {orientation}");
            assert_eq!(
                (back.nose, back.mouth_left, back.mouth_right),
                ((25.0, 35.0), (18.0, 48.0), (32.0, 49.0)),
                "orientation {orientation}"
            );
        }
    }

    #[test]
    fn a_point_maps_back_like_the_corner_of_a_face() {
        let (w, h) = (200, 100);
        for orientation in [1, 3, 6, 8] {
            for p in [(0.0, 0.0), (12.5, 37.25), (99.0, 199.0)] {
                let at = Face {
                    x: p.0,
                    y: p.1,
                    width: 0.0,
                    height: 0.0,
                    score: 1.0,
                    left_eye: p,
                    right_eye: p,
                    nose: p,
                    mouth_left: p,
                    mouth_right: p,
                };
                let stored = point_to_stored(p, orientation, w, h);
                let face = to_stored(at, orientation, w, h);
                assert_eq!((face.x, face.y), stored, "orientation {orientation}");
                let back = face_to_upright(
                    Face {
                        x: stored.0,
                        y: stored.1,
                        left_eye: stored,
                        right_eye: stored,
                        ..at
                    },
                    orientation,
                    w,
                    h,
                );
                assert_eq!((back.x, back.y), p, "orientation {orientation}");
            }
        }
    }

    #[test]
    fn to_upright_is_the_inverse_of_to_stored() {
        let (w, h) = (200, 100);
        for orientation in [1, 3, 6, 8] {
            for p in [(0, 0), (199, 0), (0, 99), (199, 99), (37, 61)] {
                let (ux, uy) = to_upright(p, orientation, w, h);
                let at = Face {
                    x: ux as f32,
                    y: uy as f32,
                    width: 0.0,
                    height: 0.0,
                    score: 1.0,
                    left_eye: (0.0, 0.0),
                    right_eye: (0.0, 0.0),
                    nose: (0.0, 0.0),
                    mouth_left: (0.0, 0.0),
                    mouth_right: (0.0, 0.0),
                };
                let back = to_stored(at, orientation, w, h);
                assert!(
                    (back.x - p.0 as f32).abs() <= 1.0 && (back.y - p.1 as f32).abs() <= 1.0,
                    "orientation {orientation}: {p:?} -> ({ux}, {uy}) -> ({}, {})",
                    back.x,
                    back.y
                );
            }
        }
    }

    fn gradient(w: usize, h: usize) -> Vec<u8> {
        (0..w * h)
            .flat_map(|i| [(i % w) as u8, (i / w) as u8, 7])
            .collect()
    }

    #[test]
    fn a_crop_holds_the_source_pixels_at_its_origin() {
        let (w, h) = (200, 150);
        let rgb = gradient(w, h);
        let (sub, win) = crop_rgb(&rgb, w, h, (100, 70), 40);
        assert_eq!((win.x, win.y, win.width, win.height), (80, 50, 40, 40));
        assert_eq!(sub.len(), 40 * 40 * 3);
        for y in 0..40 {
            for x in 0..40 {
                let i = (y * 40 + x) * 3;
                let j = ((y + win.y) * w + x + win.x) * 3;
                assert_eq!(sub[i..i + 3], rgb[j..j + 3]);
            }
        }
    }

    #[test]
    fn a_crop_at_a_corner_is_clamped_inside_the_image() {
        let (w, h) = (200, 150);
        let rgb = gradient(w, h);
        let (sub, win) = crop_rgb(&rgb, w, h, (199, 149), 40);
        assert_eq!((win.x, win.y, win.width, win.height), (160, 110, 40, 40));
        assert_eq!(sub[..3], [160, 110, 7]);
        let (sub, win) = crop_rgb(&rgb, w, h, (0, 0), 480);
        assert_eq!((win.x, win.y, win.width, win.height), (0, 0, w, h));
        assert_eq!(sub, rgb);
    }

    #[test]
    fn decodes_a_cell_into_input_coordinates() {
        let cols = 4;
        let n = cols * cols;
        let mut cls = vec![0.0; n];
        let mut obj = vec![0.0; n];
        let mut bbox = vec![0.0; n * 4];
        let mut kps = vec![0.0; n * 10];
        let i = cols + 2;
        cls[i] = 0.9;
        obj[i] = 0.9;
        bbox[i * 4..i * 4 + 4].copy_from_slice(&[0.5, 0.5, 2f32.ln(), 1f32.ln()]);
        kps[i * 10..i * 10 + 10]
            .copy_from_slice(&[0.75, 0.25, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 0.25, 0.75]);
        let mut faces = Vec::new();
        decode_stride(8, cols, &cls, &obj, &bbox, &kps, &mut faces);
        assert_eq!(faces.len(), 1);
        let f = faces[0];
        assert!((f.score - 0.9).abs() < 1e-6);
        assert!((f.x - (20.0 - 8.0)).abs() < 1e-4);
        assert!((f.y - (12.0 - 4.0)).abs() < 1e-4);
        assert!((f.width - 16.0).abs() < 1e-4);
        assert!((f.height - 8.0).abs() < 1e-4);
        assert_eq!(f.left_eye, (18.0, 10.0));
        assert_eq!(f.right_eye, (22.0, 10.0));
        assert_eq!(f.nose, (20.0, 12.0));
        assert_eq!(f.mouth_left, (18.0, 14.0));
        assert_eq!(f.mouth_right, (22.0, 14.0));
    }

    #[test]
    fn drops_cells_below_the_threshold() {
        let mut faces = Vec::new();
        decode_stride(8, 1, &[0.5], &[0.5], &[0.0; 4], &[0.0; 10], &mut faces);
        assert!(faces.is_empty());
    }

    #[test]
    fn nms_keeps_the_best_of_overlapping_boxes() {
        let kept = nms(
            vec![face(0.0, 0.7), face(1.0, 0.9), face(50.0, 0.8)],
            NMS_THRESHOLD,
        );
        assert_eq!(kept, vec![face(1.0, 0.9), face(50.0, 0.8)]);
    }

    #[test]
    fn finds_no_face_in_flat_or_checkerboard_images() {
        let (w, h) = (640, 427);
        let flat = vec![128u8; w * h * 3];
        assert!(detect(&flat, w, h).unwrap().is_empty());
        let checker: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                let v = if ((i % w) / 16 + (i / w) / 16) % 2 == 0 {
                    0
                } else {
                    255
                };
                [v, v, v]
            })
            .collect();
        assert!(detect(&checker, w, h).unwrap().is_empty());
        assert!(detect_whole(&flat, w, h).unwrap().is_empty());
        assert!(detect_whole(&checker, w, h).unwrap().is_empty());
        assert!(detect_whole(&checker, h, w).unwrap().is_empty());
    }

    #[test]
    fn the_whole_input_follows_the_image_orientation() {
        assert_eq!(Input::whole(2112, 1408).size(), (640, 448));
        assert_eq!(Input::whole(1408, 2112).size(), (448, 640));
        assert_eq!(Input::whole(500, 500), Input::Landscape);
        assert_eq!(Input::Square.size(), (INPUT, INPUT));
    }

    #[test]
    fn fit_keeps_the_image_inside_the_input() {
        assert_eq!(fit(2112, 1408, (640, 448)), 640.0 / 2112.0);
        assert_eq!(fit(1408, 2112, (448, 640)), 640.0 / 2112.0);
        assert_eq!(fit(4000, 3000, (640, 448)), 448.0 / 3000.0);
        assert_eq!(fit(1616, 1080, (INPUT, INPUT)), 320.0 / 1616.0);
    }

    #[test]
    fn a_rectangular_input_holds_the_image_at_its_top_left() {
        let (w, h) = (4, 2);
        let rgb: Vec<u8> = (0..w * h).flat_map(|_| [10, 20, 30]).collect();
        let t = input_tensor(&rgb, w, h, 1.0, (8, 4));
        assert_eq!(t.shape(), &[1, 3, 4, 8]);
        let v = t.try_as_plain_ram().unwrap().as_slice::<f32>().unwrap();
        for c in 0..3 {
            for y in 0..4 {
                for x in 0..8 {
                    let want = if x < w && y < h {
                        [30.0, 20.0, 10.0][c]
                    } else {
                        0.0
                    };
                    assert_eq!(v[(c * 4 + y) * 8 + x], want, "c {c} y {y} x {x}");
                }
            }
        }
    }

    #[test]
    fn a_face_on_a_scaled_decode_maps_back_to_the_stored_preview() {
        // A 2100x1401 stored preview decoded at 3/8 is 788x526 (mozjpeg
        // rounds up) with its content scaled by exactly 3/8; mapping by the
        // sizes stays within one scaled pixel (8/3 stored ones).
        let stored = (2100, 1401);
        let on_full = Face {
            x: 600.0,
            y: 300.0,
            width: 240.0,
            height: 300.0,
            score: 0.9,
            left_eye: (660.0, 400.0),
            right_eye: (780.0, 402.0),
            nose: (720.0, 460.0),
            mouth_left: (670.0, 520.0),
            mouth_right: (770.0, 522.0),
        };
        for orientation in [1, 3, 6, 8] {
            let (sw, sh) = if matches!(orientation, 6 | 8) {
                (526, 788)
            } else {
                (788, 526)
            };
            let (kx, ky) = (3.0 / 8.0, 3.0 / 8.0);
            let on_scaled = Face {
                x: on_full.x * kx,
                y: on_full.y * ky,
                width: on_full.width * kx,
                height: on_full.height * ky,
                left_eye: (on_full.left_eye.0 * kx, on_full.left_eye.1 * ky),
                right_eye: (on_full.right_eye.0 * kx, on_full.right_eye.1 * ky),
                nose: (on_full.nose.0 * kx, on_full.nose.1 * ky),
                mouth_left: (on_full.mouth_left.0 * kx, on_full.mouth_left.1 * ky),
                mouth_right: (on_full.mouth_right.0 * kx, on_full.mouth_right.1 * ky),
                ..on_full
            };
            let want = to_stored(on_full, orientation, stored.0, stored.1);
            let got = scaled_to_stored(on_scaled, orientation, (sw, sh), stored);
            let close = |a: f32, b: f32| (a - b).abs() <= 8.0 / 3.0;
            assert!(
                close(got.x, want.x)
                    && close(got.y, want.y)
                    && close(got.width, want.width)
                    && close(got.height, want.height)
                    && close(got.left_eye.0, want.left_eye.0)
                    && close(got.left_eye.1, want.left_eye.1)
                    && close(got.right_eye.0, want.right_eye.0)
                    && close(got.right_eye.1, want.right_eye.1)
                    && close(got.nose.0, want.nose.0)
                    && close(got.nose.1, want.nose.1)
                    && close(got.mouth_left.0, want.mouth_left.0)
                    && close(got.mouth_left.1, want.mouth_left.1)
                    && close(got.mouth_right.0, want.mouth_right.0)
                    && close(got.mouth_right.1, want.mouth_right.1),
                "orientation {orientation}: {got:?} against {want:?}"
            );
            assert_eq!(got.score, 0.9);
        }
    }

    #[test]
    fn the_whole_image_detection_reports_the_stored_preview_size() {
        let image = Upright {
            rgb: vec![128; 540 * 808 * 3],
            width: 540,
            height: 808,
            stored: (1616, 1080),
        };
        let d = detect_whole_upright(&image, 6).unwrap();
        assert_eq!((d.width, d.height), (1616, 1080));
        assert!(d.faces.is_empty() && d.point.is_none());
    }

    /// Set `HANDPIC_FACE_JPEG` to a JPEG with one clear face, then run with
    /// `--ignored`.
    #[test]
    #[ignore]
    fn detects_a_face_in_a_real_image() {
        let path = std::env::var("HANDPIC_FACE_JPEG").expect("HANDPIC_FACE_JPEG");
        let jpeg = std::fs::read(path).unwrap();
        let (rgb, w, h) = crate::decode::decode_rgb(&jpeg).unwrap();
        let faces = detect(&rgb, w, h).unwrap();
        assert!(!faces.is_empty());
        let f = faces[0];
        for (x, y) in [f.left_eye, f.right_eye] {
            assert!(x >= f.x && x <= f.x + f.width && y >= f.y && y <= f.y + f.height);
        }
    }
}
