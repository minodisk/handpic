//! Extract one folder's worth of metadata and thumbnails in parallel.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use crate::allfaces::{self, FaceCounts, Measured, Reading, MAX_FACES};
use crate::arw::{Arw, FocusFrame, FocusLocation, Shot};
use crate::candidate::{face_cue_unless, focus_cue_rgb_unless, luma, Cue};
use crate::decode::{decode_rgb, thumbnail_jpeg_near};
use crate::eyes::judged_face;
use crate::faces::{decode_gate, detect_around, detect_gate_upright, Detection, Face};
use crate::fullsharp::full_eye_sharpnesses;
use crate::reader::{read_full_undecoded, read_metadata, read_preview_undecoded};
use crate::sharpness::{eye_af_frame, score_gray, score_preview, trusted_focus};

/// Quality of the cached thumbnails. 80 gives ~19KB for a 404x270 frame.
pub const THUMBNAIL_QUALITY: f32 = 80.0;

/// The long edge every file's thumbnail aims at, whatever the size of its
/// preview: a 1616x1080 ARW preview's 2/8.
const JPEG_THUMBNAIL_LONG_EDGE: usize = 404;

/// Whether `path` has a RAW extension Handpic lists: `.ARW`, `.CR3`, `.DNG`,
/// `.NEF`, `.ORF` or `.RAF`, in any case.
pub fn is_raw_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| {
        ["arw", "cr3", "dng", "nef", "orf", "raf"]
            .iter()
            .any(|raw| e.eq_ignore_ascii_case(raw))
    })
}

/// Whether `path` has a JPEG extension: `.jpg` or `.jpeg`, in any case.
pub fn is_jpeg_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg"))
}

/// What one file contributes to the index.
#[derive(Debug, Clone)]
pub struct Entry {
    pub orientation: u16,
    pub shot: Shot,
    /// Unrotated thumbnail JPEG; the caller carries the Orientation.
    pub thumbnail: Vec<u8>,
}

/// Why one file has no `Entry`, with the metadata that was parsed anyway:
/// `shot` is `None` when none could be, and `orientation` is then `1`.
#[derive(Debug, Clone)]
pub struct Failure {
    pub message: String,
    pub orientation: u16,
    pub shot: Option<Box<Shot>>,
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Failure {
            message,
            orientation: 1,
            shot: None,
        }
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Read one file's metadata and thumbnail. Pure: no shared state, no IO beyond
/// `path`, and every failure comes back as `Err` rather than a panic, carrying
/// the metadata when it could be parsed. The sharpness score and the face
/// search are the analysis pass's (`extract_analysis`).
pub fn extract(path: &Path) -> Result<Entry, Failure> {
    extract_unless(path, &AtomicBool::new(false), &ReadSlots::default())
        .expect("a never-set flag never abandons")
}

/// `extract`, abandoned (`None`) once `cancel` is set between its stages,
/// its reads going through `slots`.
fn extract_unless(
    path: &Path,
    cancel: &AtomicBool,
    slots: &ReadSlots,
) -> Option<Result<Entry, Failure>> {
    let read = {
        let _slot = slots.acquire(cancel)?;
        #[cfg(test)]
        tests::cancel_mid_read(cancel);
        read_preview_undecoded(path)
    };
    let read = read.and_then(|(arw, image)| Ok((arw, image.decode()?)));
    let (arw, preview) = match read {
        Ok(read) => read,
        Err(e) => {
            // The preview can fail (missing, out of range, undecodable HEVC)
            // where the metadata parses, so read that alone.
            let metadata = {
                let _slot = slots.acquire(cancel)?;
                read_metadata(path)
            };
            let failure = match metadata {
                Ok(arw) => Failure {
                    message: e.to_string(),
                    orientation: arw.orientation,
                    shot: Some(Box::new(arw.shot)),
                },
                Err(_) => e.to_string().into(),
            };
            return Some(Err(failure));
        }
    };
    let failed = |message: String| Failure {
        message,
        orientation: arw.orientation,
        shot: Some(Box::new(arw.shot.clone())),
    };
    if canceled(cancel) {
        return None;
    }
    // mozjpeg aborts through a panic, not an error return, when the bytes are
    // not a JPEG. A single corrupt file out of thousands must cost that file
    // and nothing else, so the decode runs inside `catch_unwind`. Nothing here
    // is shared or observable after a panic, hence `AssertUnwindSafe`.
    let thumbnail = match catch_unwind(AssertUnwindSafe(|| {
        thumbnail_jpeg_near(&preview, JPEG_THUMBNAIL_LONG_EDGE, THUMBNAIL_QUALITY)
    })) {
        Ok(Ok(thumbnail)) => thumbnail,
        Ok(Err(e)) => return Some(Err(failed(e.to_string()))),
        Err(e) => {
            let message = match e.downcast::<String>() {
                Ok(message) => *message,
                Err(_) => "the thumbnail could not be made".to_string(),
            };
            return Some(Err(failed(message)));
        }
    };
    if canceled(cancel) {
        return None;
    }
    Some(Ok(Entry {
        orientation: arw.orientation,
        shot: arw.shot,
        thumbnail,
    }))
}

/// What the analysis pass contributes to the index for one file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Analysis {
    pub cue: Cue,
    /// The `sharpness` score of the preview: `sharpness::score_gray` on the
    /// luma of the cue's RGB decode when the cue made one, else
    /// `sharpness::score_preview`'s own grayscale decode; `None` when it could
    /// not be scored, which does not fail the file, and always for a JPEG file.
    pub sharpness: Option<f64>,
    /// `fullsharp::full_eye_sharpnesses` of the judged face's two eyes on the
    /// full JPEG (the value `full_eye_sharpness` gives); `None` when the cue judged no face or its mesh was turned down
    /// (`eyes::mesh_trusted`, no eye centers), the file has no usable larger
    /// JPEG, or on any failure.
    pub full_eye_sharp: Option<f64>,
    /// `allfaces::gate_count`, how many confident faces of a size similar to
    /// the judged face's the whole preview holds, the judged face included:
    /// over a `faces::GATE_INPUT` detection with an AF point, over the
    /// whole-image detection the cue already ran without one, faces counted
    /// from `allfaces::GATE_LOW_RES_CONFIDENCE` on both. 0 when the cue
    /// judged no face (no detection is run for it); `None` when that
    /// detection failed, and always for a JPEG file.
    pub gate_count: Option<u32>,
}

/// Compute the face cue and the sharpness score of one file from one read of
/// its preview. The cue runs the face detector and, on the face
/// nearest the AF point, the face mesh (`candidate::focus_cue_rgb_unless` on
/// the decode pass 2 keeps);
/// without a trusted AF point, on the largest confident face of the whole
/// preview (`eyes::judged_face`, `candidate::face_cue_unless`). Either way the
/// sharpness score takes its luma from the cue's full-size RGB decode, and
/// decodes the preview to grayscale itself only when the cue made none. Only
/// an unreadable file is `Err`; a decode, detection or scoring failure, or a
/// panic in any of them, is an unknown cue or no score; a mesh failure leaves
/// the face its YuNet eye landmarks, a mesh `eyes::mesh_trusted` turns down
/// leaves it no eye centers. An AF frame whose cue judged a face is
/// also searched whole at the low `faces::GATE_INPUT` for the gate count. The
/// judged face's eyes are measured once more on the full JPEG, a second
/// read, when the file holds one larger than the preview (`full_eye_sharp`).
/// A JPEG file gets none of these, and is not read.
pub fn extract_analysis(path: &Path) -> Result<Analysis, String> {
    extract_analysis_unless(path, &AtomicBool::new(false), &ReadSlots::default())
        .expect("a never-set flag never abandons")
}

thread_local! {
    /// The read slots of the pass 2 file this thread is analyzing, or of the
    /// file `extract_face_counts_unless` measures, which `yield_to_pause`
    /// waits on, with the instant its stage waits first saw a pause (one
    /// budget for the whole file); none outside those two.
    static PASS_SLOTS: std::cell::RefCell<Option<(ReadSlots, Option<Instant>)>> =
        const { std::cell::RefCell::new(None) };
}

/// Sets `PASS_SLOTS` for one file and clears it on drop, an unwind included.
struct PassSlots;

impl PassSlots {
    fn set(slots: &ReadSlots) -> Self {
        PASS_SLOTS.with(|s| *s.borrow_mut() = Some((slots.clone(), None)));
        PassSlots
    }
}

impl Drop for PassSlots {
    fn drop(&mut self) {
        PASS_SLOTS.with(|s| *s.borrow_mut() = None);
    }
}

/// A stage boundary of pass 2 or of the all-faces check: wait out a pause
/// held on the slots of the file this thread analyzes
/// (`ReadSlots::wait_unpaused_since`). All of a file's stage waits share one
/// `MAX_PAUSE` budget. A no-op outside those, so `candidate`'s stages call it
/// whoever runs them.
pub fn yield_to_pause(cancel: &AtomicBool) {
    PASS_SLOTS.with(|s| {
        if let Some((slots, since)) = &mut *s.borrow_mut() {
            slots.wait_unpaused_since(cancel, since);
        }
    });
}

/// `extract_analysis`, abandoned (`None`) once `cancel` is set between its
/// stages, its reads going through `slots` and its stages yielding to a pause
/// of them.
fn extract_analysis_unless(
    path: &Path,
    cancel: &AtomicBool,
    slots: &ReadSlots,
) -> Option<Result<Analysis, String>> {
    let _pass = PassSlots::set(slots);
    if is_jpeg_file(path) {
        return Some(Ok(Analysis::default()));
    }
    let read = {
        let _slot = slots.acquire(cancel)?;
        #[cfg(test)]
        tests::cancel_mid_read(cancel);
        read_preview_undecoded(path)
    };
    let (arw, image) = match read {
        Ok(read) => read,
        Err(e) => return Some(Err(e.to_string())),
    };
    yield_to_pause(cancel);
    if canceled(cancel) {
        return None;
    }
    let preview = match image.decode() {
        Ok(preview) => preview,
        Err(e) => return Some(Err(e.to_string())),
    };
    let focus = trusted_focus(&arw.shot);
    let frame = eye_af_frame(&arw.shot).map(|(_, frame)| frame);
    let (cue, faces, gate_count, scored) =
        cue_and_gate(&preview, arw.orientation, focus, frame, cancel)?;
    if canceled(cancel) {
        return None;
    }
    yield_to_pause(cancel);
    let sharpness = scored.or_else(|| score(&preview, &arw.shot, focus, &faces));
    yield_to_pause(cancel);
    let full_eye_sharp = full_eye_sharp(path, &arw, &cue, cancel, slots)?;
    Some(Ok(Analysis {
        cue,
        sharpness,
        full_eye_sharp,
        gate_count,
    }))
}

/// `cue_and_gate`'s answer: the cue, the faces the score falls back to, the
/// gate count and the score taken on the cue's decode.
type CueAndGate = (Cue, Vec<Face>, Option<u32>, Option<f64>);

/// Pass 2's cue of `preview`, the faces its score falls back to, its gate
/// count and its score taken on the cue's decode when it made one
/// (`score_rgb`), the fourth value. With the AF point `focus` the cue's
/// detection runs around it, the score needs no faces and is taken around
/// `focus` with the eye-AF `frame`, and the gate count takes a
/// `faces::GATE_INPUT` search of its own (`af_gate_count`); without one the
/// faces are searched for on the whole image at `faces::WHOLE_INPUT`, the cue
/// judges the largest confident one, the score falls back to them and the
/// gate counts over them from the same floor as the AF path
/// (`whole_gate_count`). `None` once `cancel` abandoned the file.
fn cue_and_gate(
    preview: &[u8],
    orientation: u16,
    focus: Option<FocusLocation>,
    frame: Option<FocusFrame>,
    cancel: &AtomicBool,
) -> Option<CueAndGate> {
    match focus {
        Some(at) => {
            let (cue, rgb) = af_cue(preview, orientation, at, cancel)?;
            let scored = rgb
                .as_ref()
                .and_then(|rgb| score_rgb(rgb, Some(at), frame, &[]));
            yield_to_pause(cancel);
            if canceled(cancel) {
                return None;
            }
            let gate = af_gate_count(preview, orientation, &cue);
            Some((cue, Vec::new(), gate, scored))
        }
        None => {
            let detection = whole_image_detection(preview, orientation);
            yield_to_pause(cancel);
            if canceled(cancel) {
                return None;
            }
            let faces = detection.as_ref().map(|d| d.faces.clone());
            let (cue, rgb) = whole_image_cue_rgb(preview, orientation, detection, cancel)?;
            let gate = faces.as_ref().map(|f| whole_gate_count(f, &cue));
            let faces = faces.unwrap_or_default();
            let scored = rgb
                .as_ref()
                .and_then(|rgb| score_rgb(rgb, None, None, &faces));
            Some((cue, faces, gate, scored))
        }
    }
}

/// The gate count of a frame without an AF point whose cue is `cue`, over
/// the `faces` of the cue's own whole-image search, faces counted from
/// `allfaces::GATE_LOW_RES_CONFIDENCE` as on the AF path.
fn whole_gate_count(faces: &[Face], cue: &Cue) -> u32 {
    allfaces::gate_count(faces, cue.face.as_ref(), allfaces::GATE_LOW_RES_CONFIDENCE)
}

/// The gate count of an AF frame whose cue is `cue`, over a whole-preview
/// search of `preview` at `faces::GATE_INPUT`, faces counted from
/// `allfaces::GATE_LOW_RES_CONFIDENCE`: 0 with no search when the cue judged
/// no face, `None` when the search failed or panicked.
fn af_gate_count(preview: &[u8], orientation: u16, cue: &Cue) -> Option<u32> {
    let Some(judged) = &cue.face else {
        return Some(0);
    };
    let detection = catch_unwind(AssertUnwindSafe(|| {
        detect_gate_upright(&decode_gate(preview, orientation)?, orientation)
    }))
    .ok()
    .and_then(Result::ok)?;
    Some(allfaces::gate_count(
        &detection.faces,
        Some(judged),
        allfaces::GATE_LOW_RES_CONFIDENCE,
    ))
}

/// The all-faces check of the file at `path` (`allfaces`): its counts of the
/// faces judged, of those with closed eyes and of those with soft eyes, over
/// the face its cue judges and the other confident faces of the whole
/// preview. It recomputes the cue as pass 2 does, on its own read and decode
/// of the preview, searches the whole preview at `faces::WHOLE_INPUT` on an
/// AF frame too, meshes the other faces on the same decode and measures
/// every eye of the frame on the full JPEG in one pass, a second read, when
/// the file holds one larger than the preview. It takes no threshold: the
/// caller picks the files, by their gate count. `Ok(None)` when the
/// whole-image detection or a needed preview decode failed, and for a JPEG
/// file, which is not read; only an unreadable file is `Err`.
pub fn extract_face_counts(path: &Path) -> Result<Option<FaceCounts>, String> {
    extract_face_counts_unless(path, &AtomicBool::new(false), &ReadSlots::default())
        .expect("a never-set flag never abandons")
}

/// `extract_face_counts`, abandoned (`None`) once `cancel` is set between its
/// stages, its reads going through `slots` and its stages yielding to a pause
/// of them.
fn extract_face_counts_unless(
    path: &Path,
    cancel: &AtomicBool,
    slots: &ReadSlots,
) -> Option<Result<Option<FaceCounts>, String>> {
    let _pass = PassSlots::set(slots);
    if is_jpeg_file(path) {
        return Some(Ok(None));
    }
    let read = {
        let _slot = slots.acquire(cancel)?;
        read_preview_undecoded(path)
    };
    let (arw, image) = match read {
        Ok(read) => read,
        Err(e) => return Some(Err(e.to_string())),
    };
    yield_to_pause(cancel);
    if canceled(cancel) {
        return None;
    }
    let preview = match image.decode() {
        Ok(preview) => preview,
        Err(e) => return Some(Err(e.to_string())),
    };
    let (cue, whole, rgb) = match trusted_focus(&arw.shot) {
        Some(at) => {
            let (cue, rgb) = af_cue(&preview, arw.orientation, at, cancel)?;
            let whole = rgb
                .as_ref()
                .and_then(|_| whole_image_detection(&preview, arw.orientation))
                .map(|d| d.faces);
            yield_to_pause(cancel);
            if canceled(cancel) {
                return None;
            }
            (cue, whole, rgb)
        }
        None => {
            let detection = whole_image_detection(&preview, arw.orientation);
            yield_to_pause(cancel);
            if canceled(cancel) {
                return None;
            }
            let whole = detection.as_ref().map(|d| d.faces.clone());
            let (cue, rgb) = whole_image_cue_rgb(&preview, arw.orientation, detection, cancel)?;
            (cue, whole, rgb)
        }
    };
    if canceled(cancel) {
        return None;
    }
    yield_to_pause(cancel);
    let others = other_faces(
        &cue,
        whole.as_deref(),
        rgb.as_ref(),
        arw.orientation,
        cancel,
    )?;
    if others.is_none() {
        return Some(Ok(None));
    }
    let eyes: Vec<_> = others.iter().flatten().map(|m| m.eye_centers).collect();
    let width = rgb.as_ref().map_or(0, |(_, w, _)| *w);
    let FullSharps {
        judged: full_eye_sharp,
        others: sharps,
    } = full_sharps(path, &arw, &cue, &eyes, width, cancel, slots)?;
    Some(Ok(others.map(|others| {
        let judged = cue.eyes_ear.map(|ear| Reading {
            ear,
            full_eye_sharp,
        });
        let readings: Vec<Reading> = judged
            .into_iter()
            .chain(others.iter().enumerate().map(|(i, m)| Reading {
                ear: m.ear,
                full_eye_sharp: sharps.as_ref().and_then(|s| s[i]),
            }))
            .collect();
        allfaces::counts(&readings, sharps.is_some())
    })))
}

/// A preview decoded to stored RGB: the pixels, the width and the height.
type Rgb = (Vec<u8>, usize, usize);

/// The score (`sharpness::score_gray`) of the AF point `focus` with its
/// eye-AF `frame`, else of `faces`, on the luma of `rgb`; a failure or panic
/// is no score.
fn score_rgb(
    (pixels, width, height): &Rgb,
    focus: Option<FocusLocation>,
    frame: Option<FocusFrame>,
    faces: &[Face],
) -> Option<f64> {
    catch_unwind(AssertUnwindSafe(|| {
        score_gray(
            &luma(pixels, *width, *height),
            *width,
            *height,
            focus,
            frame,
            faces,
        )
    }))
    .ok()
    .and_then(Result::ok)
}

/// `decode_rgb` of `preview`; a failure or panic is none.
fn decoded(preview: &[u8]) -> Option<Rgb> {
    catch_unwind(AssertUnwindSafe(|| decode_rgb(preview)))
        .ok()
        .and_then(Result::ok)
}

/// The other faces of the frame the all-faces check counts
/// (`allfaces::candidates` of the whole-image faces `whole` beside the face
/// `cue` judged), measured on `rgb`. `None` inside when `whole` is unknown, or
/// a face is to be measured and `rgb` is unknown; `None` outside when
/// `cancel` abandoned the file.
fn other_faces(
    cue: &Cue,
    whole: Option<&[Face]>,
    rgb: Option<&Rgb>,
    orientation: u16,
    cancel: &AtomicBool,
) -> Option<Option<Vec<Measured>>> {
    let Some(whole) = whole else {
        return Some(None);
    };
    let limit = MAX_FACES - usize::from(cue.eyes_ear.is_some());
    let faces = allfaces::candidates(whole, cue.face.as_ref(), limit);
    if faces.is_empty() {
        return Some(Some(Vec::new()));
    }
    let Some((rgb, width, height)) = rgb else {
        return Some(None);
    };
    allfaces::measure_unless(rgb, *width, *height, orientation, &faces, cancel).map(Some)
}

/// Whether `arw` holds a JPEG larger than its preview to measure the eyes on:
/// not when it has no full image, an HEVC one, or only the preview itself.
pub fn has_larger_jpeg(arw: &Arw) -> bool {
    let same = match (arw.full, arw.preview) {
        (Some(full), Some(preview)) => full.offset == preview.offset,
        _ => false,
    };
    arw.full.is_some() && !arw.hevc && !same
}

/// The full-size sharpness of the eyes of the face `cue` judged, whatever its
/// state; `None` inside when the cue judged no face or has no eye centers (an
/// untrusted mesh), the file has no larger
/// JPEG or anything failed, `None` outside when `cancel` abandoned the file.
/// The read goes through `slots`, the decode does not.
fn full_eye_sharp(
    path: &Path,
    arw: &Arw,
    cue: &Cue,
    cancel: &AtomicBool,
    slots: &ReadSlots,
) -> Option<Option<f64>> {
    full_sharps(path, arw, cue, &[], 0, cancel, slots).map(|s| s.judged)
}

/// What `full_sharps` measured on the full JPEG.
struct FullSharps {
    /// The judged face's; `None` when the cue judged no face or it failed.
    judged: Option<f64>,
    /// Each other face's, `None` where it failed; `None` when the file has no
    /// larger JPEG or the read failed.
    others: Option<Vec<Option<f64>>>,
}

/// From one read of the full JPEG, the full-size sharpness of the eyes of the
/// face `cue` judged, whatever its state (`None` when the cue judged no face
/// or it failed), and of each pair of eye centers in `others`, given in the
/// stored coordinates of a preview `width` px wide (each `None` when it
/// failed; all `None` when the file has no larger JPEG or the read failed).
/// Nothing is read when there is no eye to measure, and every eye is
/// decoded in one pass (`fullsharp::full_eye_sharpnesses`). `None` outside
/// when `cancel` abandoned the file, checked after the read and after the
/// pass. The read goes through `slots`, the decode does not.
fn full_sharps(
    path: &Path,
    arw: &Arw,
    cue: &Cue,
    others: &[[(f32, f32); 2]],
    width: usize,
    cancel: &AtomicBool,
    slots: &ReadSlots,
) -> Option<FullSharps> {
    if !has_larger_jpeg(arw) {
        return Some(FullSharps {
            judged: None,
            others: None,
        });
    }
    if cue.eye_centers.is_none() && others.is_empty() {
        return Some(FullSharps {
            judged: None,
            others: Some(Vec::new()),
        });
    }
    let read = {
        let _slot = slots.acquire(cancel)?;
        read_full_undecoded(path)
    };
    yield_to_pause(cancel);
    if canceled(cancel) {
        return None;
    }
    let Ok(full) = read.and_then(|(_, image)| image.decode()) else {
        return Some(FullSharps {
            judged: None,
            others: None,
        });
    };
    let eyes: Vec<[(f32, f32); 2]> = cue.eye_centers.iter().chain(others).copied().collect();
    let preview_width = match cue.eye_centers {
        Some(_) => cue.preview_width,
        None => width,
    };
    let mut sharps = catch_unwind(AssertUnwindSafe(|| {
        full_eye_sharpnesses(&full, &eyes, preview_width)
    }))
    .unwrap_or_else(|_| vec![None; eyes.len()]);
    yield_to_pause(cancel);
    if canceled(cancel) {
        return None;
    }
    let judged = match cue.eye_centers {
        Some(_) => sharps.remove(0),
        None => None,
    };
    Some(FullSharps {
        judged,
        others: Some(sharps),
    })
}

/// The detection on the whole preview; any failure or panic is none.
fn whole_image_detection(preview: &[u8], orientation: u16) -> Option<Detection> {
    catch_unwind(AssertUnwindSafe(|| {
        detect_around(preview, orientation, None)
    }))
    .ok()
    .and_then(Result::ok)
}

/// The cue of the face `eyes::judged_face` picks from the whole-image
/// `detection` (the largest at or above `sharpness::FACE_CONFIDENCE`), on one
/// full-size decode of `preview`. No detection or no such face is an unknown
/// cue with no decode; a decode failure or a panic is an unknown cue.
#[cfg(test)]
fn whole_image_cue(
    preview: &[u8],
    orientation: u16,
    detection: Option<Detection>,
    cancel: &AtomicBool,
) -> Option<Cue> {
    whole_image_cue_rgb(preview, orientation, detection, cancel).map(|(cue, _)| cue)
}

/// `whole_image_cue` with the full-size decode it took, none when it did not
/// decode or the decode failed.
fn whole_image_cue_rgb(
    preview: &[u8],
    orientation: u16,
    detection: Option<Detection>,
    cancel: &AtomicBool,
) -> Option<(Cue, Option<Rgb>)> {
    let Some(detection) = detection else {
        return Some((Cue::unknown(), None));
    };
    let Some(face) = judged_face(&detection.faces, None).copied() else {
        return Some((Cue::unknown(), None));
    };
    let Some(rgb) = decoded(preview) else {
        return Some((Cue::unknown(), None));
    };
    if canceled(cancel) {
        return None;
    }
    let (pixels, width, height) = &rgb;
    let cue = guarded(|| {
        face_cue_unless(
            pixels,
            *width,
            *height,
            orientation,
            &face,
            detection,
            cancel,
        )
        .map(Ok)
    })?;
    Some((cue, Some(rgb)))
}

/// `score_preview` of `preview`, for a file whose cue made no RGB decode to
/// score on; a failure or panic is no score.
fn score(preview: &[u8], shot: &Shot, focus: Option<FocusLocation>, faces: &[Face]) -> Option<f64> {
    let eye_af = eye_af_frame(shot);
    catch_unwind(AssertUnwindSafe(|| {
        score_preview(preview, focus, eye_af.map(|(_, frame)| frame), faces)
    }))
    .ok()
    .and_then(Result::ok)
}

#[cfg(test)]
fn cue(
    preview: &[u8],
    orientation: u16,
    focus: Option<FocusLocation>,
    cancel: &AtomicBool,
) -> Option<Cue> {
    match focus {
        Some(at) => af_cue(preview, orientation, at, cancel).map(|(cue, _)| cue),
        None => Some(Cue::unknown()),
    }
}

/// The cue around the AF point `focus` (`candidate::focus_cue_rgb_unless`)
/// with the full-size decode of `preview` it took; a decode failure is an
/// unknown cue and no decode.
fn af_cue(
    preview: &[u8],
    orientation: u16,
    focus: FocusLocation,
    cancel: &AtomicBool,
) -> Option<(Cue, Option<Rgb>)> {
    let Some(rgb) = decoded(preview) else {
        return Some((Cue::unknown(), None));
    };
    let (pixels, width, height) = &rgb;
    let cue =
        guarded(|| focus_cue_rgb_unless(pixels, *width, *height, orientation, focus, cancel))?;
    Some((cue, Some(rgb)))
}

/// `run`'s cue, `None` when it was abandoned; an error or a panic is an
/// unknown cue.
fn guarded(run: impl FnOnce() -> Option<anyhow::Result<Cue>>) -> Option<Cue> {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(None) => None,
        Ok(Some(Ok(cue))) => Some(cue),
        Ok(Some(Err(_))) | Err(_) => Some(Cue::unknown()),
    }
}

fn canceled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Relaxed)
}

/// Run `extract` over `paths` on a pool of `threads` threads, handing each
/// result to `on_item` from the worker thread that produced it. Once `cancel`
/// is set no further file is started, and files already running stop at their
/// next stage and are not handed to `on_item` at all.
///
/// The workers take the paths `focus` lists first, in its order, then the rest
/// in index order; `focus` may be changed from another thread while they run.
/// Their file reads go through `focus`'s read slots, and one waiting there
/// stops on `cancel` like any other stage.
/// `threads` must be at least 1; `0` is rejected rather than silently falling
/// back to rayon's default pool size.
///
/// The pool is built here rather than taken from rayon's global one, so the
/// caller sizes it and nothing else in the process shares it.
///
/// `on_item` must not panic: a panic inside it unwinds out of the worker's
/// loop, aborts the scan, and propagates out of `extract_all` as a
/// panic rather than an `Err`, leaving an arbitrary subset of indices
/// delivered. Callers that share state behind a mutex must keep `on_item`
/// panic-free or wrap it themselves.
///
/// Each worker sets its own OS priority to `priority` once, as it starts. A
/// worker that cannot runs at normal priority anyway; the first such error is
/// returned as `Ok(Some(..))` once the scan ends, for the caller to log.
pub fn extract_all<F>(
    paths: &[PathBuf],
    threads: usize,
    priority: Priority,
    focus: &ScanFocus,
    on_item: F,
    cancel: &AtomicBool,
) -> Result<Option<String>, String>
where
    F: Fn(usize, Result<Entry, Failure>) + Send + Sync,
{
    for_each_path(
        paths,
        threads,
        priority,
        focus,
        extract_unless,
        on_item,
        cancel,
    )
}

/// Run `extract_analysis` over `paths` the way `extract_all` runs `extract`:
/// same pool, same `focus` order, same `cancel`, same `threads == 0` error,
/// the same priority report, and the same rule that `on_item` must not panic.
pub fn extract_analysis_all<F>(
    paths: &[PathBuf],
    threads: usize,
    priority: Priority,
    focus: &ScanFocus,
    on_item: F,
    cancel: &AtomicBool,
) -> Result<Option<String>, String>
where
    F: Fn(usize, Result<Analysis, String>) + Send + Sync,
{
    for_each_path(
        paths,
        threads,
        priority,
        focus,
        extract_analysis_unless,
        on_item,
        cancel,
    )
}

/// Run `extract_face_counts` over `paths` the way `extract_analysis_all` runs
/// `extract_analysis`: same pool, same `focus` order and read slots, same
/// `cancel`, a file abandoned by it not handed to `on_item`, same
/// `threads == 0` error, the same priority report, and the same rule that
/// `on_item` must not panic.
pub fn extract_face_counts_all<F>(
    paths: &[PathBuf],
    threads: usize,
    priority: Priority,
    focus: &ScanFocus,
    on_item: F,
    cancel: &AtomicBool,
) -> Result<Option<String>, String>
where
    F: Fn(usize, Result<Option<FaceCounts>, String>) + Send + Sync,
{
    for_each_path(
        paths,
        threads,
        priority,
        focus,
        extract_face_counts_unless,
        on_item,
        cancel,
    )
}

/// How the view steers a running scan: the paths it takes first, nearest the
/// user's view first, and the read slots its reads go through. One handle can
/// serve every pass of a scan; a handle never set leaves the order by index,
/// and the default one has uncapped read slots nothing can pause.
#[derive(Debug, Clone, Default)]
pub struct ScanFocus {
    paths: Arc<Mutex<Vec<String>>>,
    slots: ReadSlots,
}

impl ScanFocus {
    /// A handle whose scan reads through `slots`.
    pub fn with_slots(slots: ReadSlots) -> Self {
        ScanFocus {
            slots,
            ..Self::default()
        }
    }

    /// Replace the list whole. A path the scan does not have, or has already
    /// taken, is skipped.
    pub fn set(&self, paths: Vec<String>) {
        *self.hot() = paths;
    }

    fn hot(&self) -> MutexGuard<'_, Vec<String>> {
        self.paths.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// How long `ReadSlots::acquire` honours a pause, and how long all of one pass 2
/// file's stage waits together do. A stuck or leaked pause costs a worker at
/// most about `MAX_PAUSE` per `acquire` (one for pass 1's read, two for pass
/// 2's) plus `MAX_PAUSE` for pass 2's stages per file, so it slows a scan and
/// never stalls it; a preview normally ends the pause long before this.
const MAX_PAUSE: Duration = Duration::from_secs(1);

/// How often a worker waiting in `ReadSlots::acquire` looks at its cancel
/// flag, a plain `AtomicBool` that cannot wake it.
const CANCEL_TICK: Duration = Duration::from_millis(50);

/// The gate between a scan's workers and the disk: at most `cap` of them are
/// inside a file read at once, and none starts one while a `Paused` is held,
/// so the preview's read queues behind few reads and no new ones. Clones share
/// the gate. The default one is uncapped.
#[derive(Debug, Clone)]
pub struct ReadSlots {
    gate: Arc<(Mutex<Gate>, Condvar)>,
    max_pause: Duration,
}

#[derive(Debug)]
struct Gate {
    free: usize,
    paused: usize,
}

impl Default for ReadSlots {
    fn default() -> Self {
        Self::new(usize::MAX)
    }
}

impl ReadSlots {
    /// `cap` must be at least 1: nothing would ever read otherwise.
    pub fn new(cap: usize) -> Self {
        Self::with_max_pause(cap, MAX_PAUSE)
    }

    fn with_max_pause(cap: usize, max_pause: Duration) -> Self {
        ReadSlots {
            gate: Arc::new((
                Mutex::new(Gate {
                    free: cap,
                    paused: 0,
                }),
                Condvar::new(),
            )),
            max_pause,
        }
    }

    /// Wait for a free permit and no pause, then hold the permit until the
    /// slot drops. `None` once `cancel` is set, the worker then abandoning its
    /// file. A pause is honoured for `MAX_PAUSE` from when this call first sees
    /// it on, a free permit taken regardless after that; the clock starts over
    /// once it sees no pause, so a wait for a permit alone uses none of it.
    pub fn acquire(&self, cancel: &AtomicBool) -> Option<ReadSlot> {
        let mut paused_since = None;
        let mut gate = self.lock();
        loop {
            if canceled(cancel) {
                return None;
            }
            if gate.paused == 0 {
                paused_since = None;
            } else {
                paused_since.get_or_insert_with(Instant::now);
            }
            let pause_over = paused_since.is_none_or(|t| t.elapsed() >= self.max_pause);
            if gate.free > 0 && pause_over {
                gate.free -= 1;
                return Some(ReadSlot(self.clone()));
            }
            gate = self
                .gate
                .1
                .wait_timeout(gate, CANCEL_TICK)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// Wait while a pause is held, for at most `MAX_PAUSE`, or until `cancel`
    /// is set. Takes no permit.
    pub fn wait_unpaused(&self, cancel: &AtomicBool) {
        self.wait_unpaused_since(cancel, &mut None);
    }

    /// `wait_unpaused` measuring from `since`, the instant a wait first saw
    /// the pause, set here when empty and cleared once a wait sees no pause.
    /// Pass 2 calls it between its compute stages with one `since` per file,
    /// so a preview's decode and paint get the cores, not only the disk, and a
    /// pause that stays held costs the file `MAX_PAUSE` in all, not per stage.
    fn wait_unpaused_since(&self, cancel: &AtomicBool, since: &mut Option<Instant>) {
        let mut gate = self.lock();
        if gate.paused == 0 {
            *since = None;
            return;
        }
        let started = *since.get_or_insert_with(Instant::now);
        while gate.paused > 0 && started.elapsed() < self.max_pause && !canceled(cancel) {
            gate = self
                .gate
                .1
                .wait_timeout(gate, CANCEL_TICK)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        if gate.paused == 0 {
            *since = None;
        }
    }

    /// Keep new reads from starting until the guard drops. Reads already
    /// inside a slot run on.
    pub fn pause(&self) -> Paused {
        self.lock().paused += 1;
        Paused(self.clone())
    }

    fn lock(&self) -> MutexGuard<'_, Gate> {
        self.gate.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// One permit of `ReadSlots`, given back on drop, a panic unwind included.
#[derive(Debug)]
pub struct ReadSlot(ReadSlots);

impl Drop for ReadSlot {
    fn drop(&mut self) {
        self.0.lock().free += 1;
        self.0.gate.1.notify_all();
    }
}

/// A pause of `ReadSlots`, ended on drop, a panic unwind included.
#[derive(Debug)]
pub struct Paused(ReadSlots);

impl Drop for Paused {
    fn drop(&mut self) {
        self.0.lock().paused -= 1;
        self.0.gate.1.notify_all();
    }
}

/// The indices of a scan not yet taken. `take_next` is O(hot list) plus an
/// amortized O(1) walk of `next` over indices already taken out of order.
struct WorkQueue {
    index_of: HashMap<String, usize>,
    taken: Vec<bool>,
    next: usize,
}

impl WorkQueue {
    fn new(paths: &[PathBuf]) -> Self {
        WorkQueue {
            index_of: paths
                .iter()
                .enumerate()
                .map(|(i, p)| (p.to_string_lossy().into_owned(), i))
                .collect(),
            taken: vec![false; paths.len()],
            next: 0,
        }
    }

    fn take_next(&mut self, focus: &ScanFocus) -> Option<usize> {
        let hot = focus
            .hot()
            .iter()
            .filter_map(|p| self.index_of.get(p).copied())
            .find(|&i| !self.taken[i]);
        let i = match hot {
            Some(i) => i,
            None => {
                while self.taken.get(self.next) == Some(&true) {
                    self.next += 1;
                }
                if self.next == self.taken.len() {
                    return None;
                }
                self.next
            }
        };
        self.taken[i] = true;
        Some(i)
    }
}

/// The OS priority of a scan's worker threads. The app lowers both of its
/// passes so the viewer's preview decode and the UI win the contended cores,
/// the analysis pass below the thumbnails; the CLI benchmarks stay at
/// `Normal` so their figures compare with `docs/humans/performance.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// The OS default: the priority is left alone.
    Normal,
    /// Windows `THREAD_PRIORITY_BELOW_NORMAL`, macOS `QOS_CLASS_UTILITY`,
    /// Linux nice 5.
    BelowNormal,
    /// Windows `THREAD_PRIORITY_LOWEST` (not `IDLE`, which can starve behind
    /// any other process), macOS `QOS_CLASS_UTILITY` at relative priority
    /// `LOWEST_RELATIVE_PRIORITY`, Linux nice 10.
    Lowest,
}

fn set_current_priority(priority: Priority) -> Result<(), String> {
    match priority {
        Priority::Normal => Ok(()),
        Priority::BelowNormal => lower_current_priority(false),
        Priority::Lowest => lower_current_priority(true),
    }
}

#[cfg(windows)]
fn lower_current_priority(lowest: bool) -> Result<(), String> {
    use thread_priority::{set_current_thread_priority, ThreadPriority, WinAPIThreadPriority};
    let level = if lowest {
        WinAPIThreadPriority::Lowest
    } else {
        WinAPIThreadPriority::BelowNormal
    };
    set_current_thread_priority(ThreadPriority::Os(level.into())).map_err(|e| e.to_string())
}

/// `Priority::Lowest`'s relative priority within `QOS_CLASS_UTILITY` on
/// macOS, below `BelowNormal`'s 0 (the floor is -15). Not
/// `QOS_CLASS_BACKGROUND`, which throttled one file's analysis 6 to 14 times
/// on the macOS CI runner.
#[cfg(target_os = "macos")]
const LOWEST_RELATIVE_PRIORITY: libc::c_int = -5;

#[cfg(target_os = "macos")]
fn lower_current_priority(lowest: bool) -> Result<(), String> {
    let relative = if lowest { LOWEST_RELATIVE_PRIORITY } else { 0 };
    // SAFETY: sets the calling thread's own QoS class; no pointers involved.
    match unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_UTILITY, relative)
    } {
        0 => Ok(()),
        e => Err(std::io::Error::from_raw_os_error(e).to_string()),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn lower_current_priority(lowest: bool) -> Result<(), String> {
    // A new thread inherits its creator's nice, so never move it below the
    // level it already has: an absolute set could raise a process started
    // under `nice`. (`-1` is also a valid nice, so an error reads as -1 and
    // then only ever lowers the priority from there.)
    // SAFETY: plain syscall. On Linux `PRIO_PROCESS` with id 0 names the
    // calling thread alone, not the whole process.
    let current = unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) };
    let nice = current.max(if lowest { 10 } else { 5 });
    // SAFETY: as above.
    match unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, nice) } {
        0 => Ok(()),
        _ => Err(std::io::Error::last_os_error().to_string()),
    }
}

#[cfg(not(any(
    windows,
    target_os = "macos",
    target_os = "linux",
    target_os = "android"
)))]
fn lower_current_priority(_lowest: bool) -> Result<(), String> {
    Ok(())
}

fn for_each_path<T, X, E, F>(
    paths: &[PathBuf],
    threads: usize,
    priority: Priority,
    focus: &ScanFocus,
    per_file: E,
    on_item: F,
    cancel: &AtomicBool,
) -> Result<Option<String>, String>
where
    E: Fn(&Path, &AtomicBool, &ReadSlots) -> Option<Result<T, X>> + Send + Sync,
    F: Fn(usize, Result<T, X>) + Send + Sync,
{
    if threads == 0 {
        return Err("threads must be at least 1".to_string());
    }
    let priority_error = Arc::new(OnceLock::new());
    let pool = {
        let priority_error = Arc::clone(&priority_error);
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .spawn_handler(move |thread| {
                let priority_error = Arc::clone(&priority_error);
                let mut b = std::thread::Builder::new();
                if let Some(name) = thread.name() {
                    b = b.name(name.to_owned());
                }
                if let Some(size) = thread.stack_size() {
                    b = b.stack_size(size);
                }
                b.spawn(move || {
                    if let Err(e) = set_current_priority(priority) {
                        let _ = priority_error.set(e);
                    }
                    thread.run()
                })?;
                Ok(())
            })
            .build()
            .map_err(|e| e.to_string())?
    };
    let queue = Mutex::new(WorkQueue::new(paths));
    pool.broadcast(|_| {
        while !canceled(cancel) {
            let Some(i) = queue
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take_next(focus)
            else {
                break;
            };
            if let Some(result) = per_file(&paths[i], cancel, &focus.slots) {
                on_item(i, result);
            }
        }
    });
    Ok(priority_error.get().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arw::{Codec, Embedded};
    use crate::eyes::MESH_CALLS;
    use crate::sharpness::FACE_CONFIDENCE;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    fn jpeg(w: usize, h: usize) -> Vec<u8> {
        let rgb = vec![128u8; w * h * 3];
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        c.set_size(w, h);
        c.set_quality(80.0);
        let mut c = c.start_compress(Vec::new()).unwrap();
        c.write_scanlines(&rgb).unwrap();
        c.finish().unwrap()
    }

    /// A TIFF shell whose IFD0 carries an Orientation and a preview of `body`.
    fn fixture(orientation: u16, body: &[u8]) -> Vec<u8> {
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
        assert_eq!(buf.len(), at);
        buf.extend_from_slice(body);
        buf
    }

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("handpic-scan-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn every_index_is_delivered_once() {
        let dir = dir("all");
        let jpeg = jpeg(64, 48);
        let paths: Vec<PathBuf> = (0..16)
            .map(|i| write(&dir, &format!("{i:02}.ARW"), &fixture(6, &jpeg)))
            .collect();

        for priority in [Priority::Normal, Priority::BelowNormal, Priority::Lowest] {
            let seen = Mutex::new(Vec::new());
            let report = extract_all(
                &paths,
                4,
                priority,
                &ScanFocus::default(),
                |i, r| seen.lock().unwrap().push((i, r.unwrap().orientation)),
                &AtomicBool::new(false),
            );
            assert_eq!(report, Ok(None), "{priority:?} is set on every worker");

            let mut seen = seen.into_inner().unwrap();
            seen.sort();
            assert_eq!(
                seen,
                (0..16).map(|i| (i, 6u16)).collect::<Vec<_>>(),
                "each index exactly once at {priority:?}"
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn canceling_stops_the_scan_early() {
        let dir = dir("cancel");
        let jpeg = jpeg(64, 48);
        let paths: Vec<PathBuf> = (0..200)
            .map(|i| write(&dir, &format!("{i:03}.ARW"), &fixture(1, &jpeg)))
            .collect();

        let cancel = AtomicBool::new(false);
        let done = Mutex::new(0usize);
        extract_all(
            &paths,
            2,
            Priority::BelowNormal,
            &ScanFocus::default(),
            |_, _| {
                let mut done = done.lock().unwrap();
                *done += 1;
                if *done >= 4 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
        )
        .unwrap();

        let done = done.into_inner().unwrap();
        assert!(done >= 4, "the files before the cancel are delivered");
        assert!(done < paths.len(), "the rest are not, got {done}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(windows)]
    fn worker_level() -> i32 {
        use thread_priority::{get_current_thread_priority, WinAPIThreadPriority};
        WinAPIThreadPriority::try_from(get_current_thread_priority().unwrap()).unwrap() as i32
    }

    #[cfg(windows)]
    fn expected_level(lowest: bool, _inherited: i32) -> i32 {
        use thread_priority::WinAPIThreadPriority;
        if lowest {
            WinAPIThreadPriority::Lowest as i32
        } else {
            WinAPIThreadPriority::BelowNormal as i32
        }
    }

    #[cfg(target_os = "linux")]
    fn worker_level() -> i32 {
        // SAFETY: plain syscall; on Linux it reads the calling thread's nice.
        unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) }
    }

    #[cfg(target_os = "linux")]
    fn expected_level(lowest: bool, inherited: i32) -> i32 {
        inherited.max(if lowest { 10 } else { 5 })
    }

    /// The calling thread's QoS class and relative priority.
    #[cfg(target_os = "macos")]
    fn worker_level() -> (u32, i32) {
        let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
        let mut relative = 0;
        // SAFETY: reads the calling thread's own QoS into the two locals.
        let e = unsafe {
            libc::pthread_get_qos_class_np(libc::pthread_self(), &mut class, &mut relative)
        };
        assert_eq!(e, 0);
        (class as u32, relative)
    }

    #[cfg(target_os = "macos")]
    fn expected_level(lowest: bool, _inherited: (u32, i32)) -> (u32, i32) {
        let utility = libc::qos_class_t::QOS_CLASS_UTILITY as u32;
        if lowest {
            (utility, super::LOWEST_RELATIVE_PRIORITY)
        } else {
            (utility, 0)
        }
    }

    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    #[test]
    fn every_worker_runs_at_the_priority_it_was_given() {
        let paths: Vec<PathBuf> = (0..32).map(|i| PathBuf::from(format!("{i}.ARW"))).collect();
        let inherited = worker_level();
        for (priority, lowest) in [(Priority::BelowNormal, false), (Priority::Lowest, true)] {
            let levels = Mutex::new(Vec::new());
            let report = for_each_path(
                &paths,
                4,
                priority,
                &ScanFocus::default(),
                |_, _: &AtomicBool, _: &ReadSlots| {
                    levels.lock().unwrap().push(worker_level());
                    Some(Ok::<_, String>(()))
                },
                |_, _| {},
                &AtomicBool::new(false),
            );
            assert_eq!(report, Ok(None));
            let levels = levels.into_inner().unwrap();
            assert_eq!(levels.len(), paths.len());
            assert!(
                levels
                    .iter()
                    .all(|l| *l == expected_level(lowest, inherited)),
                "{priority:?}: {levels:?}"
            );
        }
    }

    fn order(paths: &[PathBuf], focus: &ScanFocus, on_item: impl Fn(usize) + Send + Sync) {
        for_each_path(
            paths,
            1,
            Priority::Normal,
            focus,
            |_, _: &AtomicBool, _: &ReadSlots| Some(Ok::<_, String>(())),
            |i, _| on_item(i),
            &AtomicBool::new(false),
        )
        .unwrap();
    }

    fn names(n: usize) -> Vec<PathBuf> {
        (0..n).map(|i| PathBuf::from(format!("{i}.ARW"))).collect()
    }

    #[test]
    fn the_focused_paths_are_taken_first_in_their_order() {
        let paths = names(6);
        let focus = ScanFocus::default();
        focus.set(vec!["4.ARW".into(), "missing.ARW".into(), "1.ARW".into()]);
        let seen = Mutex::new(Vec::new());
        order(&paths, &focus, |i| seen.lock().unwrap().push(i));
        assert_eq!(seen.into_inner().unwrap(), vec![4, 1, 0, 2, 3, 5]);
    }

    #[test]
    fn a_focus_set_mid_run_is_honored_by_the_next_take() {
        let paths = names(6);
        let focus = ScanFocus::default();
        let seen = Mutex::new(Vec::new());
        order(&paths, &focus, |i| {
            seen.lock().unwrap().push(i);
            if i == 1 {
                focus.set(vec!["0.ARW".into(), "5.ARW".into(), "3.ARW".into()]);
            }
        });
        assert_eq!(
            seen.into_inner().unwrap(),
            vec![0, 1, 5, 3, 2, 4],
            "a path already done is skipped"
        );
    }

    #[test]
    fn a_focused_scan_delivers_every_index_once_and_stops_on_cancel() {
        let paths = names(200);
        let focus = ScanFocus::default();
        focus.set(
            (0..200)
                .rev()
                .step_by(3)
                .map(|i| format!("{i}.ARW"))
                .collect(),
        );
        let seen = Mutex::new(Vec::new());
        for_each_path(
            &paths,
            4,
            Priority::Normal,
            &focus,
            |_, _: &AtomicBool, _: &ReadSlots| Some(Ok::<_, String>(())),
            |i, _| {
                seen.lock().unwrap().push(i);
                if i == 100 {
                    focus.set(vec!["7.ARW".into(), "7.ARW".into(), "150.ARW".into()]);
                }
            },
            &AtomicBool::new(false),
        )
        .unwrap();
        let mut seen = seen.into_inner().unwrap();
        seen.sort();
        assert_eq!(seen, (0..200).collect::<Vec<_>>());

        let cancel = AtomicBool::new(false);
        let done = Mutex::new(0usize);
        for_each_path(
            &paths,
            2,
            Priority::Normal,
            &focus,
            |_, _: &AtomicBool, _: &ReadSlots| Some(Ok::<_, String>(())),
            |_, _| {
                let mut done = done.lock().unwrap();
                *done += 1;
                if *done >= 4 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
        )
        .unwrap();
        let done = done.into_inner().unwrap();
        assert!((4..paths.len()).contains(&done), "got {done}");
    }

    #[test]
    fn the_analysis_pass_delivers_every_index_once_and_stops_on_cancel() {
        let dir = dir("analysis-all");
        let jpeg = jpeg(64, 48);
        let paths: Vec<PathBuf> = (0..200)
            .map(|i| write(&dir, &format!("{i:03}.ARW"), &fixture(1, &jpeg)))
            .collect();

        let seen = Mutex::new(Vec::new());
        extract_analysis_all(
            &paths,
            4,
            Priority::Lowest,
            &ScanFocus::default(),
            |i, r| seen.lock().unwrap().push((i, r.unwrap().cue.face)),
            &AtomicBool::new(false),
        )
        .unwrap();
        let mut seen = seen.into_inner().unwrap();
        seen.sort_by_key(|&(i, _)| i);
        assert_eq!(
            seen,
            (0..200).map(|i| (i, None)).collect::<Vec<_>>(),
            "each index exactly once"
        );

        let cancel = AtomicBool::new(false);
        let done = Mutex::new(0usize);
        extract_analysis_all(
            &paths,
            2,
            Priority::Lowest,
            &ScanFocus::default(),
            |_, _| {
                let mut done = done.lock().unwrap();
                *done += 1;
                if *done >= 4 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
        )
        .unwrap();
        let done = done.into_inner().unwrap();
        assert!(done >= 4, "the files before the cancel are delivered");
        assert!(done < paths.len(), "the rest are not, got {done}");

        assert!(extract_analysis_all(
            &paths,
            0,
            Priority::Lowest,
            &ScanFocus::default(),
            |_, _| {},
            &AtomicBool::new(false)
        )
        .is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    thread_local! {
        static CANCEL_MID_READ: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// Set `cancel` inside the read's slot when this thread asked for it
    /// (`MidRead`).
    pub(super) fn cancel_mid_read(cancel: &AtomicBool) {
        if CANCEL_MID_READ.get() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Turns `cancel_mid_read` on for this thread until dropped.
    struct MidRead;

    impl MidRead {
        fn on() -> Self {
            CANCEL_MID_READ.set(true);
            MidRead
        }
    }

    impl Drop for MidRead {
        fn drop(&mut self) {
            CANCEL_MID_READ.set(false);
        }
    }

    #[test]
    fn a_file_canceled_during_its_read_is_not_delivered() {
        let dir = dir("cancel-read");
        let good = write(&dir, "good.ARW", &fixture(1, &jpeg(64, 48)));
        let bad = write(&dir, "bad.ARW", &fixture(6, &[0u8; 512]));
        let slots = ReadSlots::default();
        let mid_read = MidRead::on();
        for path in [&good, &bad] {
            let cancel = AtomicBool::new(false);
            assert!(
                extract_unless(path, &cancel, &slots).is_none(),
                "{}",
                path.display()
            );
            assert!(cancel.load(Ordering::Relaxed));
            let cancel = AtomicBool::new(false);
            assert!(
                extract_analysis_unless(path, &cancel, &slots).is_none(),
                "{}",
                path.display()
            );
            assert!(cancel.load(Ordering::Relaxed));
        }
        drop(mid_read);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_hevc_decode_of_an_hdr_pq_file_holds_no_read_slot() {
        use crate::reader::tests::AT_HEVC_DECODE;
        use std::cell::RefCell;
        use std::rc::Rc;
        let dir = dir("hevc-slot");
        // A lone `free` box: `hevc::to_jpeg` is reached and finds no `hvcC`.
        let mut data = 64u32.to_be_bytes().to_vec();
        data.extend_from_slice(b"free");
        data.resize(64, 0);
        let path = write(
            &dir,
            "hdr.CR3",
            &crate::cr3::tests::hdr_pq(None, Some(data)).build(),
        );
        let slots = ReadSlots::new(1);
        let free = Rc::new(RefCell::new(Vec::new()));
        let (seen, gate) = (free.clone(), slots.clone());
        AT_HEVC_DECODE.set(Some(Box::new(move || {
            seen.borrow_mut().push(gate.lock().free)
        })));
        let unset = AtomicBool::new(false);
        let failed = |message: &str| message == "HEVC preview without hvcC";
        assert!(failed(
            &extract_unless(&path, &unset, &slots)
                .unwrap()
                .unwrap_err()
                .message
        ));
        assert!(failed(
            &extract_analysis_unless(&path, &unset, &slots)
                .unwrap()
                .unwrap_err()
        ));
        assert!(failed(
            &extract_face_counts_unless(&path, &unset, &slots)
                .unwrap()
                .unwrap_err()
        ));
        let larger = Arw {
            preview: Some(Embedded {
                offset: 10,
                length: 1,
                codec: Codec::Jpeg,
            }),
            full: Some(Embedded {
                offset: 20,
                length: 1,
                codec: Codec::Jpeg,
            }),
            orientation: 1,
            shot: Shot::default(),
            hevc: false,
        };
        let others = [[(30.0, 30.0), (70.0, 30.0)]];
        let sharps = full_sharps(
            &path,
            &larger,
            &Cue::default(),
            &others,
            100,
            &unset,
            &slots,
        );
        assert_eq!(sharps.map(|s| s.others), Some(None), "the decode failed");
        AT_HEVC_DECODE.set(None);
        assert_eq!(
            *free.borrow(),
            [1, 1, 1, 1],
            "every site decodes with the one permit given back"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_canceled_mid_pipeline_is_not_delivered() {
        let dir = dir("cancel-mid");
        let path = write(&dir, "a.ARW", &fixture(1, &jpeg(64, 48)));
        let set = AtomicBool::new(true);
        let slots = ReadSlots::default();
        assert!(extract_unless(&path, &set, &slots).is_none());
        assert!(extract_analysis_unless(&path, &set, &slots).is_none());
        assert!(
            extract_unless(&dir.join("missing.ARW"), &set, &slots).is_none(),
            "a canceled file is not read, so it cannot fail"
        );
        assert!(
            extract_unless(&dir.join("missing.ARW"), &AtomicBool::new(false), &slots)
                .is_some_and(|r| r.is_err()),
            "an unreadable file is an error"
        );
        let focus = Some(FocusLocation {
            sensor_w: 6000,
            sensor_h: 4000,
            x: 3000,
            y: 2000,
        });
        assert_eq!(cue(&jpeg(64, 48), 1, focus, &set), None);

        // The flag is set while each file is in flight, after the pre-dispatch
        // check has let it through.
        let paths: Vec<PathBuf> = (0..8)
            .map(|i| write(&dir, &format!("{i}.ARW"), &fixture(1, &jpeg(64, 48))))
            .collect();
        for pass in 1..=3 {
            let cancel = AtomicBool::new(false);
            let delivered = Mutex::new(0usize);
            for_each_path(
                &paths,
                2,
                Priority::Normal,
                &ScanFocus::default(),
                |path, cancel: &AtomicBool, slots: &ReadSlots| {
                    let started = Instant::now();
                    cancel.store(true, Ordering::Relaxed);
                    let result = match pass {
                        1 => extract_unless(path, cancel, slots)
                            .map(|r| r.map(|_| ()).map_err(|e| e.message)),
                        2 => extract_analysis_unless(path, cancel, slots).map(|r| r.map(|_| ())),
                        _ => extract_face_counts_unless(path, cancel, slots).map(|r| r.map(|_| ())),
                    };
                    assert!(started.elapsed() < Duration::from_secs(5));
                    result
                },
                |_, _| *delivered.lock().unwrap() += 1,
                &cancel,
            )
            .unwrap();
            assert_eq!(delivered.into_inner().unwrap(), 0, "pass {pass}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_a_separate_jpeg_is_worth_a_second_read() {
        let embedded = |offset| Embedded {
            offset,
            length: 1,
            codec: Codec::Jpeg,
        };
        let arw = |full, hevc| Arw {
            preview: Some(embedded(10)),
            full,
            orientation: 1,
            shot: Shot::default(),
            hevc,
        };
        assert!(has_larger_jpeg(&arw(Some(embedded(20)), false)));
        assert!(!has_larger_jpeg(&arw(None, false)));
        assert!(!has_larger_jpeg(&arw(Some(embedded(20)), true)));
        assert!(!has_larger_jpeg(&arw(Some(embedded(10)), false)));
    }

    #[test]
    fn every_judged_face_with_a_larger_jpeg_is_read_and_a_preview_only_file_is_not() {
        let dir = dir("full-eye-sharp");
        let embedded = |offset| Embedded {
            offset,
            length: 1,
            codec: Codec::Jpeg,
        };
        let arw = |full| Arw {
            preview: Some(embedded(10)),
            full,
            orientation: 1,
            shot: Shot::default(),
            hevc: false,
        };
        let (larger, preview_only) = (arw(Some(embedded(20))), arw(None));
        let judged = Cue {
            eye_centers: Some([(30.0, 30.0), (70.0, 30.0)]),
            preview_width: 100,
            ..Cue::default()
        };
        let no_face = Cue {
            eye_centers: None,
            ..judged.clone()
        };
        let missing = dir.join("missing.ARW");
        let (slots, set, unset) = (
            ReadSlots::default(),
            AtomicBool::new(true),
            AtomicBool::new(false),
        );
        let measure = |arw: &Arw, cue: &Cue, cancel: &AtomicBool| {
            full_eye_sharp(&missing, arw, cue, cancel, &slots)
        };
        assert_eq!(
            measure(&preview_only, &judged, &set),
            Some(None),
            "a file without a larger JPEG reads nothing, so not even a cancel abandons it"
        );
        assert_eq!(
            measure(&larger, &no_face, &set),
            Some(None),
            "no judged face reads nothing"
        );
        assert_eq!(
            measure(&larger, &judged, &unset),
            Some(None),
            "a failed read is no score, not an error"
        );
        assert_eq!(
            measure(&larger, &judged, &set),
            None,
            "a judged face reads the full JPEG, so a cancel abandons it at the slot"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_other_faces_are_read_once_with_the_judged_one_and_unknown_without_a_larger_jpeg() {
        let dir = dir("other-faces");
        let embedded = |offset| Embedded {
            offset,
            length: 1,
            codec: Codec::Jpeg,
        };
        let arw = |full| Arw {
            preview: Some(embedded(10)),
            full,
            orientation: 1,
            shot: Shot::default(),
            hevc: false,
        };
        let (larger, preview_only) = (arw(Some(embedded(20))), arw(None));
        let no_face = Cue::default();
        let others = [[(30.0, 30.0), (70.0, 30.0)]];
        let missing = dir.join("missing.ARW");
        let (slots, set, unset) = (
            ReadSlots::default(),
            AtomicBool::new(true),
            AtomicBool::new(false),
        );
        let measure = |arw: &Arw, others: &[[(f32, f32); 2]], cancel: &AtomicBool| {
            full_sharps(&missing, arw, &no_face, others, 100, cancel, &slots)
                .map(|s| (s.judged, s.others))
        };
        assert_eq!(
            measure(&preview_only, &others, &set),
            Some((None, None)),
            "without a larger JPEG the other faces' sharpness is unknown and nothing is read"
        );
        assert_eq!(
            measure(&larger, &[], &set),
            Some((None, Some(Vec::new()))),
            "no eye to measure reads nothing"
        );
        assert_eq!(
            measure(&larger, &others, &unset),
            Some((None, None)),
            "a failed read leaves the soft count unknown"
        );
        assert_eq!(
            measure(&larger, &others, &set),
            None,
            "another face alone reads the full JPEG, so a cancel abandons it"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_other_faces_need_a_detection_and_a_decode_only_when_there_is_one() {
        let never = AtomicBool::new(false);
        let cue = Cue::default();
        assert_eq!(other_faces(&cue, None, None, 1, &never), Some(None));
        let shy = scored(FACE_CONFIDENCE.next_down(), 100.0);
        assert_eq!(
            other_faces(&cue, Some(&[shy]), None, 1, &never),
            Some(Some(Vec::new())),
            "no candidate needs no pixels"
        );
        let large = scored(0.9, 100.0);
        assert_eq!(
            other_faces(&cue, Some(&[large]), None, 1, &never),
            Some(None)
        );
        // Outside the image: no mesh, no model run.
        let away = Face { x: 500.0, ..large };
        let rgb = (vec![128u8; 64 * 48 * 3], 64, 48);
        assert_eq!(
            other_faces(&cue, Some(&[away]), Some(&rgb), 1, &never),
            Some(Some(Vec::new()))
        );
        assert_eq!(
            other_faces(&cue, Some(&[away]), Some(&rgb), 1, &AtomicBool::new(true)),
            None
        );
    }

    /// The model runs (`faces::model_runs`) and face meshes this thread made
    /// while `run` ran.
    fn runs_of<T>(run: impl FnOnce() -> T) -> (T, (usize, usize, usize), usize) {
        let ((crop, whole, gate), meshes) = (crate::faces::model_runs(), MESH_CALLS.get());
        let out = run();
        let (c, w, g) = crate::faces::model_runs();
        (
            out,
            (c - crop, w - whole, g - gate),
            MESH_CALLS.get() - meshes,
        )
    }

    #[test]
    fn pass_two_runs_no_whole_image_search_and_no_other_mesh_on_an_af_frame() {
        let never = AtomicBool::new(false);
        let preview = jpeg(640, 480);
        let focus = FocusLocation {
            sensor_w: 6000,
            sensor_h: 4000,
            x: 3000,
            y: 2000,
        };
        let (got, runs, meshes) = runs_of(|| cue_and_gate(&preview, 1, Some(focus), None, &never));
        let (cue, faces, gate, _) = got.unwrap();
        assert_eq!(
            (runs, meshes),
            ((1, 0, 0), 0),
            "the crop search alone: no judged face, no gate search"
        );
        assert_eq!((cue.face, faces, gate), (None, Vec::new(), Some(0)));

        let judged = Cue {
            face: Some(scored(0.9, 100.0)),
            ..Cue::default()
        };
        let (gate, runs, meshes) = runs_of(|| af_gate_count(&preview, 1, &judged));
        assert_eq!(
            (gate, runs, meshes),
            (Some(1), (0, 0, 1), 0),
            "a judged face takes one low-resolution search and counts though it found nothing"
        );
        assert_eq!(
            af_gate_count(b"not a jpeg", 1, &judged),
            None,
            "a failed search is unknown"
        );
        assert_eq!(
            cue_and_gate(&preview, 1, Some(focus), None, &AtomicBool::new(true)),
            None
        );
    }

    #[test]
    fn an_af_frame_is_scored_on_the_luma_of_the_cue_decode() {
        let never = AtomicBool::new(false);
        let (w, h) = (640, 480);
        let pattern: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                let v = ((i % w) * 7 + (i / w) * 13) as u8;
                [v, v / 2, 255 - v]
            })
            .collect();
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        c.set_size(w, h);
        c.set_quality(80.0);
        let mut c = c.start_compress(Vec::new()).unwrap();
        c.write_scanlines(&pattern).unwrap();
        let preview = c.finish().unwrap();
        let focus = FocusLocation {
            sensor_w: 6000,
            sensor_h: 4000,
            x: 2000,
            y: 1500,
        };
        let (rgb, rw, rh) = decode_rgb(&preview).unwrap();
        let gray = luma(&rgb, rw, rh);
        let frame = FocusFrame {
            width: 300,
            height: 300,
        };
        for frame in [None, Some(frame)] {
            let (_, _, _, scored) = cue_and_gate(&preview, 1, Some(focus), frame, &never).unwrap();
            assert_eq!(
                scored,
                Some(score_gray(&gray, rw, rh, Some(focus), frame, &[]).unwrap()),
                "{frame:?}"
            );
        }
        let (_, _, _, scored) =
            cue_and_gate(b"not a jpeg", 1, Some(focus), Some(frame), &never).unwrap();
        assert_eq!(
            scored, None,
            "no decode leaves the score to score_preview's own decode"
        );
    }

    #[test]
    fn without_an_af_point_the_gate_counts_over_the_whole_image_search() {
        let never = AtomicBool::new(false);
        let (got, runs, _) = runs_of(|| cue_and_gate(&jpeg(640, 480), 1, None, None, &never));
        assert_eq!(runs, (0, 1, 0), "no search of its own");
        assert_eq!(
            got.map(|(cue, _, gate, _)| (cue.face, gate)),
            Some((None, Some(0)))
        );
        let (got, runs, _) = runs_of(|| cue_and_gate(b"not a jpeg", 1, None, None, &never));
        assert_eq!(runs, (0, 0, 0));
        assert_eq!(
            got.map(|(_, _, gate, _)| gate),
            Some(None),
            "a failed detection is unknown"
        );
    }

    #[test]
    fn without_an_af_point_the_gate_counts_faces_from_the_low_resolution_floor() {
        let at = |x: f32, score: f32| Face {
            x,
            ..scored(score, 100.0)
        };
        let judged = Cue {
            face: Some(at(0.0, 0.9)),
            ..Cue::default()
        };
        let floor = at(200.0, allfaces::GATE_LOW_RES_CONFIDENCE);
        let between = at(400.0, 0.7);
        let below = at(600.0, allfaces::GATE_LOW_RES_CONFIDENCE.next_down());
        let sure = at(800.0, FACE_CONFIDENCE);
        assert_eq!(
            whole_gate_count(&[floor, between, below, sure], &judged),
            4,
            "the judged face, the faces at and over the floor, none under it"
        );
        assert_eq!(whole_gate_count(&[floor], &Cue::default()), 0);
    }

    #[test]
    fn the_face_counts_of_a_file_are_abandoned_on_cancel_and_none_for_a_jpeg() {
        let dir = dir("face-counts");
        let path = write(&dir, "a.ARW", &fixture(1, &jpeg(64, 48)));
        let slots = ReadSlots::default();
        assert_eq!(
            extract_face_counts_unless(&path, &AtomicBool::new(true), &slots),
            None
        );
        assert_eq!(
            extract_face_counts(&path),
            Ok(Some(FaceCounts::default())),
            "no face, and no larger JPEG for the soft count"
        );
        assert_eq!(
            extract_face_counts(&dir.join("missing.jpg")),
            Ok(None),
            "a JPEG file is not read"
        );
        assert!(extract_face_counts(&dir.join("missing.ARW")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_face_counts_pass_delivers_every_index_once_and_stops_on_cancel() {
        let dir = dir("face-counts-all");
        let jpeg = jpeg(64, 48);
        let paths: Vec<PathBuf> = (0..200)
            .map(|i| write(&dir, &format!("{i:03}.ARW"), &fixture(1, &jpeg)))
            .collect();

        let seen = Mutex::new(Vec::new());
        extract_face_counts_all(
            &paths,
            4,
            Priority::Lowest,
            &ScanFocus::default(),
            |i, r| seen.lock().unwrap().push((i, r.unwrap())),
            &AtomicBool::new(false),
        )
        .unwrap();
        let mut seen = seen.into_inner().unwrap();
        seen.sort_by_key(|&(i, _)| i);
        assert_eq!(
            seen,
            (0..200)
                .map(|i| (i, Some(FaceCounts::default())))
                .collect::<Vec<_>>(),
            "each index exactly once"
        );

        let cancel = AtomicBool::new(false);
        let done = Mutex::new(0usize);
        extract_face_counts_all(
            &paths,
            2,
            Priority::Lowest,
            &ScanFocus::default(),
            |_, _| {
                let mut done = done.lock().unwrap();
                *done += 1;
                if *done >= 4 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
        )
        .unwrap();
        let done = done.into_inner().unwrap();
        assert!(done >= 4, "the files before the cancel are delivered");
        assert!(done < paths.len(), "the rest are not, got {done}");

        assert!(extract_face_counts_all(
            &paths,
            0,
            Priority::Lowest,
            &ScanFocus::default(),
            |_, _| {},
            &AtomicBool::new(false)
        )
        .is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Long enough for the pool to start and reach the gate.
    const SETTLE: Duration = Duration::from_millis(150);
    /// A pause no test outlasts.
    const LONG_PAUSE: Duration = Duration::from_secs(60);
    /// Only a scan that ignores its cancel waits this long, well under
    /// `LONG_PAUSE`.
    const HANG: Duration = Duration::from_secs(30);

    fn fixtures(name: &str, n: usize) -> (PathBuf, Vec<PathBuf>) {
        let dir = dir(name);
        let bytes = fixture(1, &jpeg(64, 48));
        let paths = (0..n)
            .map(|i| write(&dir, &format!("{i}.ARW"), &bytes))
            .collect();
        (dir, paths)
    }

    /// Run one pass on two threads through `slots`, pushing each index
    /// delivered to `seen`.
    fn pass(
        analysis: bool,
        paths: &[PathBuf],
        slots: &ReadSlots,
        cancel: &AtomicBool,
        seen: &Mutex<Vec<usize>>,
    ) {
        let focus = ScanFocus::with_slots(slots.clone());
        let delivered = |i| seen.lock().unwrap().push(i);
        if analysis {
            let on_item = |i, _| delivered(i);
            extract_analysis_all(paths, 2, Priority::Normal, &focus, on_item, cancel)
        } else {
            let on_item = |i, _| delivered(i);
            extract_all(paths, 2, Priority::Normal, &focus, on_item, cancel)
        }
        .unwrap();
    }

    fn sorted(seen: Mutex<Vec<usize>>) -> Vec<usize> {
        let mut seen = seen.into_inner().unwrap();
        seen.sort();
        seen
    }

    #[test]
    fn no_more_slots_are_live_than_the_cap() {
        let cap = 2;
        let slots = ReadSlots::new(cap);
        let cancel = AtomicBool::new(false);
        let live = Mutex::new((0usize, 0usize));
        std::thread::scope(|s| {
            for _ in 0..8 {
                s.spawn(|| {
                    for _ in 0..5 {
                        let _slot = slots.acquire(&cancel).unwrap();
                        {
                            let mut live = live.lock().unwrap();
                            live.0 += 1;
                            live.1 = live.1.max(live.0);
                        }
                        std::thread::sleep(Duration::from_millis(2));
                        live.lock().unwrap().0 -= 1;
                    }
                });
            }
        });
        let (now, peak) = live.into_inner().unwrap();
        assert_eq!(now, 0);
        assert!((1..=cap).contains(&peak), "peak of {peak} live slots");

        let first = slots.acquire(&cancel);
        let second = slots.acquire(&cancel);
        assert!(
            first.is_some() && second.is_some(),
            "the whole cap is usable"
        );
    }

    #[test]
    fn a_scan_reads_only_through_a_free_slot_and_delivers_every_index_once() {
        let (dir, paths) = fixtures("capped", 6);
        for analysis in [false, true] {
            let slots = ReadSlots::new(1);
            let cancel = AtomicBool::new(false);
            let seen = Mutex::new(Vec::new());
            std::thread::scope(|s| {
                let held = slots.acquire(&cancel);
                let scan = s.spawn(|| pass(analysis, &paths, &slots, &cancel, &seen));
                std::thread::sleep(SETTLE);
                assert!(
                    seen.lock().unwrap().is_empty(),
                    "no read without the one slot, analysis pass: {analysis}"
                );
                drop(held);
                scan.join().unwrap();
            });
            assert_eq!(sorted(seen), (0..6).collect::<Vec<_>>());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_worker_reads_during_a_pause_and_all_do_once_it_ends() {
        let (dir, paths) = fixtures("paused", 6);
        for analysis in [false, true] {
            let slots = ReadSlots::with_max_pause(usize::MAX, LONG_PAUSE);
            let cancel = AtomicBool::new(false);
            let seen = Mutex::new(Vec::new());
            std::thread::scope(|s| {
                let paused = slots.pause();
                let scan = s.spawn(|| pass(analysis, &paths, &slots, &cancel, &seen));
                std::thread::sleep(SETTLE);
                assert!(
                    seen.lock().unwrap().is_empty(),
                    "no read while paused, analysis pass: {analysis}"
                );
                drop(paused);
                scan.join().unwrap();
            });
            assert_eq!(sorted(seen), (0..6).collect::<Vec<_>>());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn two_pauses_hold_until_the_last_one_ends() {
        let slots = ReadSlots::with_max_pause(1, LONG_PAUSE);
        let cancel = AtomicBool::new(false);
        let (first, second) = (slots.pause(), slots.pause());
        std::thread::scope(|s| {
            let worker = s.spawn(|| slots.acquire(&cancel).is_some());
            drop(first);
            std::thread::sleep(SETTLE);
            assert!(!worker.is_finished(), "one pause is still on");
            drop(second);
            assert!(worker.join().unwrap());
        });
    }

    #[test]
    fn a_pause_dropped_by_a_panic_releases_the_workers() {
        let (dir, paths) = fixtures("pause-panic", 4);
        let slots = ReadSlots::with_max_pause(usize::MAX, LONG_PAUSE);
        let cancel = AtomicBool::new(false);
        let seen = Mutex::new(Vec::new());
        std::thread::scope(|s| {
            let paused = slots.pause();
            let scan = s.spawn(|| pass(false, &paths, &slots, &cancel, &seen));
            std::thread::sleep(SETTLE);
            assert!(seen.lock().unwrap().is_empty());
            let panicked = catch_unwind(AssertUnwindSafe(move || {
                let _paused = paused;
                panic!("a panic while paused");
            }));
            assert!(panicked.is_err());
            scan.join().unwrap();
        });
        assert_eq!(sorted(seen), (0..4).collect::<Vec<_>>());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_slot_dropped_by_a_panic_frees_its_permit() {
        let slots = ReadSlots::new(1);
        let cancel = AtomicBool::new(false);
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let _slot = slots.acquire(&cancel);
            panic!("a panic inside the read");
        }));
        assert!(panicked.is_err());
        assert!(slots.acquire(&cancel).is_some());
    }

    #[test]
    fn a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit() {
        let (dir, paths) = fixtures("wait-cancel", 6);
        for on_pause in [true, false] {
            let slots = ReadSlots::with_max_pause(1, LONG_PAUSE);
            let cancel = AtomicBool::new(false);
            let seen = Mutex::new(Vec::new());
            std::thread::scope(|s| {
                let paused = on_pause.then(|| slots.pause());
                let slot = (!on_pause).then(|| slots.acquire(&cancel));
                let (alive, ended) = std::sync::mpsc::channel::<()>();
                let scan = s.spawn(|| {
                    let _alive = alive;
                    pass(false, &paths, &slots, &cancel, &seen)
                });
                std::thread::sleep(SETTLE);
                cancel.store(true, Ordering::Relaxed);
                let hung = matches!(
                    ended.recv_timeout(HANG),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                );
                drop((paused, slot));
                assert!(!hung, "the scan ended while held, paused: {on_pause}");
                scan.join().unwrap();
            });
            assert_eq!(sorted(seen), Vec::<usize>::new(), "paused: {on_pause}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_cancel_is_answered_before_a_free_permit() {
        let slots = ReadSlots::default();
        assert!(slots.acquire(&AtomicBool::new(true)).is_none());
    }

    #[test]
    fn a_wait_for_no_pause_returns_at_once() {
        let slots = ReadSlots::with_max_pause(1, Duration::from_secs(5));
        let started = Instant::now();
        slots.wait_unpaused(&AtomicBool::new(false));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_wait_ends_when_the_pause_does() {
        let slots = ReadSlots::with_max_pause(1, Duration::from_secs(5));
        let paused = slots.pause();
        let started = Instant::now();
        std::thread::scope(|s| {
            let waiter = s.spawn(|| slots.wait_unpaused(&AtomicBool::new(false)));
            std::thread::sleep(Duration::from_millis(100));
            assert!(!waiter.is_finished(), "the wait honours the pause");
            drop(paused);
            waiter.join().unwrap();
        });
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn a_wait_is_bounded_by_the_maximum_pause() {
        let max_pause = Duration::from_millis(100);
        let slots = ReadSlots::with_max_pause(1, max_pause);
        let _paused = slots.pause();
        let started = Instant::now();
        slots.wait_unpaused(&AtomicBool::new(false));
        let took = started.elapsed();
        assert!(took >= max_pause, "took {took:?}");
        assert!(took < Duration::from_secs(2), "took {took:?}");
    }

    #[test]
    fn a_wait_stops_on_cancel() {
        let slots = ReadSlots::with_max_pause(1, Duration::from_secs(5));
        let _paused = slots.pause();
        let started = Instant::now();
        slots.wait_unpaused(&AtomicBool::new(true));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_yield_outside_pass_two_ignores_a_pause() {
        let slots = ReadSlots::with_max_pause(1, Duration::from_secs(5));
        let _paused = slots.pause();
        let started = Instant::now();
        yield_to_pause(&AtomicBool::new(false));
        drop(PassSlots::set(&slots));
        yield_to_pause(&AtomicBool::new(false));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_yield_inside_pass_two_honours_a_pause() {
        let max_pause = Duration::from_millis(100);
        let slots = ReadSlots::with_max_pause(1, max_pause);
        let _paused = slots.pause();
        let _pass = PassSlots::set(&slots);
        let started = Instant::now();
        yield_to_pause(&AtomicBool::new(false));
        assert!(started.elapsed() >= max_pause);
    }

    #[test]
    fn a_files_stage_waits_share_one_pause_budget() {
        let max_pause = Duration::from_millis(100);
        let slots = ReadSlots::with_max_pause(1, max_pause);
        let _pass = PassSlots::set(&slots);
        let cancel = AtomicBool::new(false);
        let paused = slots.pause();
        let started = Instant::now();
        yield_to_pause(&cancel);
        assert!(started.elapsed() >= max_pause);
        let second = Instant::now();
        yield_to_pause(&cancel);
        assert!(
            second.elapsed() < max_pause / 2,
            "took {:?}",
            second.elapsed()
        );
        drop(paused);
        yield_to_pause(&cancel);
        let _again = slots.pause();
        let third = Instant::now();
        yield_to_pause(&cancel);
        assert!(third.elapsed() >= max_pause, "took {:?}", third.elapsed());
    }

    #[test]
    fn a_pause_held_throughout_only_slows_the_scan() {
        let (dir, paths) = fixtures("pause-held", 6);
        let max_pause = Duration::from_millis(20);
        let slots = ReadSlots::with_max_pause(usize::MAX, max_pause);
        let seen = Mutex::new(Vec::new());
        let _paused = slots.pause();
        let started = Instant::now();
        pass(false, &paths, &slots, &AtomicBool::new(false), &seen);
        let took = started.elapsed();
        assert_eq!(sorted(seen), (0..6).collect::<Vec<_>>());
        assert!(took >= max_pause * 3, "6 files on 2 workers, took {took:?}");
        assert!(took < Duration::from_secs(5), "took {took:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_wait_for_a_permit_alone_does_not_use_up_the_pause() {
        let max_pause = Duration::from_millis(400);
        let slots = ReadSlots::with_max_pause(1, max_pause);
        let cancel = AtomicBool::new(false);
        let held = slots.acquire(&cancel);
        std::thread::scope(|s| {
            let worker = s.spawn(|| slots.acquire(&cancel).is_some());
            std::thread::sleep(max_pause * 3 / 2);
            let paused = slots.pause();
            drop(held);
            std::thread::sleep(max_pause / 2);
            assert!(!worker.is_finished(), "the pause began after the wait");
            drop(paused);
            assert!(worker.join().unwrap());
        });
    }

    #[test]
    fn a_pause_seen_off_gives_the_next_one_the_whole_maximum() {
        let max_pause = Duration::from_millis(400);
        let slots = ReadSlots::with_max_pause(1, max_pause);
        let cancel = AtomicBool::new(false);
        let held = slots.acquire(&cancel);
        std::thread::scope(|s| {
            let first = slots.pause();
            let worker = s.spawn(|| slots.acquire(&cancel).is_some());
            std::thread::sleep(max_pause * 3 / 4);
            drop(first);
            std::thread::sleep(CANCEL_TICK * 2);
            let second = slots.pause();
            drop(held);
            std::thread::sleep(max_pause / 2);
            assert!(!worker.is_finished(), "the second pause began a new clock");
            drop(second);
            assert!(worker.join().unwrap());
        });
    }

    #[test]
    fn a_preview_that_cannot_be_scored_still_yields_a_thumbnail() {
        let dir = dir("unscored");
        let ok = extract_analysis(&write(&dir, "ok.ARW", &fixture(1, &jpeg(64, 48)))).unwrap();
        assert!(ok.sharpness.is_some());
        let tiny_path = write(&dir, "tiny.ARW", &fixture(1, &jpeg(2, 2)));
        assert_eq!(extract_analysis(&tiny_path).unwrap().sharpness, None);
        assert!(!extract(&tiny_path).unwrap().thumbnail.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_jpeg_file_yields_its_exif_and_a_thumbnail_but_no_score() {
        use crate::jpeg::tests::{plain_jpeg, with_exif, W};
        let dir = dir("jpeg");
        let w = W(false);
        let tiff = w.tiff(
            &[w.short(0x0112, 6)],
            &[w.ascii(0x9003, "2026:09:27 12:34:56")],
        );
        let a = write(&dir, "a.jpg", &with_exif(&plain_jpeg(64, 48), &tiff));
        let e = extract(&a).unwrap();
        assert_eq!(e.orientation, 6);
        assert_eq!(e.shot.capture_time.as_deref(), Some("2026:09:27 12:34:56"));
        assert_eq!(&e.thumbnail[..2], &[0xFF, 0xD8]);
        assert_eq!(extract_analysis(&a).unwrap(), Analysis::default());
        let b = write(&dir, "b.JPEG", &plain_jpeg(64, 48));
        let bare = extract(&b).unwrap();
        assert_eq!(bare.orientation, 1);
        assert!(!bare.thumbnail.is_empty());
        assert_eq!(extract_analysis(&b).unwrap().sharpness, None);
        assert_eq!(
            extract_analysis(&dir.join("missing.jpg")).unwrap(),
            Analysis::default(),
            "a JPEG file is not read"
        );
        assert!(extract(&write(&dir, "c.jpg", b"not a jpeg")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_raf_thumbnail_aims_at_the_arw_long_edge() {
        use crate::jpeg::tests::{plain_jpeg, with_exif, W};
        use crate::raf::tests::{raf, JPEG_AT};
        let dir = dir("raf");
        let w = W(true);
        let jpeg = with_exif(&plain_jpeg(2400, 1600), &w.tiff(&[w.short(0x0112, 8)], &[]));
        let e = extract(&write(&dir, "a.RAF", &raf(JPEG_AT, &jpeg))).unwrap();
        assert_eq!(e.orientation, 8);
        let d = mozjpeg::Decompress::new_mem(&e.thumbnail).unwrap();
        assert_eq!((d.width(), d.height()), (JPEG_THUMBNAIL_LONG_EDGE, 270));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_raw_thumbnail_aims_at_the_long_edge_whatever_the_preview_size() {
        let dir = dir("near");
        for ((w, h), want) in [((3200, 2400), (404, 303)), ((1616, 1080), (404, 270))] {
            let e = extract(&write(&dir, &format!("{w}.DNG"), &fixture(1, &jpeg(w, h)))).unwrap();
            let d = mozjpeg::Decompress::new_mem(&e.thumbnail).unwrap();
            assert_eq!((d.width(), d.height()), want, "{w}x{h}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_mixed_list_of_raws_and_jpegs_delivers_every_index() {
        use crate::jpeg::tests::plain_jpeg;
        let dir = dir("mixed");
        let paths: Vec<PathBuf> = (0..8)
            .map(|i| {
                if i % 2 == 0 {
                    write(&dir, &format!("{i}.ARW"), &fixture(3, &jpeg(64, 48)))
                } else {
                    write(&dir, &format!("{i}.jpg"), &plain_jpeg(64, 48))
                }
            })
            .collect();
        let seen = Mutex::new(Vec::new());
        extract_all(
            &paths,
            4,
            Priority::Normal,
            &ScanFocus::default(),
            |i, r| seen.lock().unwrap().push((i, r.unwrap().orientation)),
            &AtomicBool::new(false),
        )
        .unwrap();
        let mut seen = seen.into_inner().unwrap();
        seen.sort();
        assert_eq!(
            seen,
            (0..8)
                .map(|i| (i, if i % 2 == 0 { 3u16 } else { 1 }))
                .collect::<Vec<_>>()
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn jpeg_and_raw_extensions_are_told_apart_in_any_case() {
        for name in ["a.jpg", "a.JPG", "a.jpeg", "a.JpEg"] {
            assert!(is_jpeg_file(Path::new(name)) && !is_raw_file(Path::new(name)));
        }
        for name in ["a.ARW", "a.png", "a.jpg.xmp", "jpg"] {
            assert!(!is_jpeg_file(Path::new(name)), "{name}");
        }
        for name in [
            "a.ARW", "a.dng", "a.NEF", "a.nef", "a.CR3", "a.cr3", "a.RAF", "a.raf", "a.ORF",
            "a.orf",
        ] {
            assert!(is_raw_file(Path::new(name)), "{name}");
        }
        for name in [
            "a.nef.xmp",
            "nef",
            "a.cr3.dop",
            "cr3",
            "a.RAF.dop",
            "raf",
            "a.ORF.xmp",
            "orf",
        ] {
            assert!(!is_raw_file(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn the_cue_is_unknown_without_an_af_point_or_a_decodable_preview() {
        let dir = dir("faces");
        let no_af = extract_analysis(&write(&dir, "noaf.ARW", &fixture(1, &jpeg(64, 48))))
            .unwrap()
            .cue;
        assert_eq!(no_af.face, None);
        assert!(no_af.detection.is_none());
        let bad = extract_analysis(&write(&dir, "bad.ARW", &fixture(1, &[0u8; 512]))).unwrap();
        assert_eq!(bad, Analysis::default());
        let focus = Some(FocusLocation {
            sensor_w: 6000,
            sensor_h: 4000,
            x: 3000,
            y: 2000,
        });
        let never = AtomicBool::new(false);
        assert_eq!(cue(&[0u8; 512], 1, focus, &never), Some(Cue::unknown()));
        let flat = cue(&jpeg(64, 48), 1, focus, &never).unwrap();
        assert_eq!(flat.face, None);
        assert!(flat.detection.is_some_and(|d| d.point.is_some()));
        assert!(extract_analysis(&dir.join("missing.ARW")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn whole(faces: Vec<Face>) -> Option<Detection> {
        Some(Detection {
            width: 64,
            height: 48,
            faces,
            point: None,
        })
    }

    fn scored(score: f32, side: f32) -> Face {
        Face {
            x: 10.0,
            y: 8.0,
            width: side,
            height: side,
            score,
            left_eye: (18.0, 18.0),
            right_eye: (30.0, 18.0),
            nose: (0.0, 0.0),
            mouth_left: (0.0, 0.0),
            mouth_right: (0.0, 0.0),
        }
    }

    #[test]
    fn without_an_af_point_the_largest_confident_face_is_judged() {
        let never = AtomicBool::new(false);
        let small = scored(0.95, 20.0);
        let large = scored(0.9, 30.0);
        let shy = scored(FACE_CONFIDENCE.next_down(), 40.0);
        let faces = [small, large, shy];
        let found = whole(faces.to_vec());
        let (cue, rgb) = whole_image_cue_rgb(&jpeg(64, 48), 1, found.clone(), &never).unwrap();
        assert_eq!(cue.face, Some(large));
        assert_eq!(cue.detection, found);
        let rgb = rgb.expect("the cue's decode");
        let (pixels, w, h) = &rgb;
        let expected = score_gray(&luma(pixels, *w, *h), *w, *h, None, None, &faces).unwrap();
        assert_eq!(
            score_rgb(&rgb, None, None, &faces),
            Some(expected),
            "scored on the cue's decode"
        );

        for none in [None, whole(Vec::new()), whole(vec![shy])] {
            assert_eq!(
                whole_image_cue(&[0u8; 512], 1, none, &never),
                Some(Cue::unknown()),
                "no judged face is unknown, with no decode"
            );
        }
        assert_eq!(
            whole_image_cue(&[0u8; 512], 1, whole(vec![large]), &never),
            Some(Cue::unknown()),
            "a decode failure is unknown"
        );
        let set = AtomicBool::new(true);
        assert_eq!(
            whole_image_cue(&jpeg(64, 48), 1, whole(vec![large]), &set),
            None
        );
    }

    #[test]
    fn a_file_whose_preview_is_not_a_jpeg_is_one_error_not_a_crash() {
        let dir = dir("corrupt");
        let jpeg = jpeg(64, 48);
        let mut paths: Vec<PathBuf> = (0..4)
            .map(|i| write(&dir, &format!("{i}.ARW"), &fixture(1, &jpeg)))
            .collect();
        paths.push(write(&dir, "bad.ARW", &fixture(6, &[0u8; 512])));

        let errors = Mutex::new(Vec::new());
        extract_all(
            &paths,
            4,
            Priority::Normal,
            &ScanFocus::default(),
            |i, r| {
                if let Err(failure) = r {
                    errors.lock().unwrap().push((i, failure))
                }
            },
            &AtomicBool::new(false),
        )
        .unwrap();

        let errors = errors.into_inner().unwrap();
        assert_eq!(errors.len(), 1);
        let (i, failure) = &errors[0];
        assert_eq!(*i, 4);
        assert_eq!(failure.orientation, 6);
        assert!(failure.shot.is_some());
        assert_eq!(failure.to_string(), failure.message);
        assert!(
            failure.message.starts_with("libjpeg fatal error:"),
            "{}",
            failure.message
        );
        assert!(!failure.message.contains("panic"), "{}", failure.message);
        assert!(
            !failure.message.contains(&*paths[4].to_string_lossy()),
            "{}",
            failure.message
        );
        assert!(!failure.message.contains("bad.ARW"), "{}", failure.message);
        assert!(extract(&dir.join("missing.ARW"))
            .unwrap_err()
            .shot
            .is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_whose_preview_cannot_be_read_keeps_its_parsed_metadata() {
        let dir = dir("truncated");
        let mut bytes = fixture(6, &jpeg(64, 48));
        // The declared preview length now runs past the end of the file.
        bytes.truncate(bytes.len() - 100);
        let path = write(&dir, "short.ARW", &bytes);

        let failure = extract(&path).unwrap_err();
        assert_eq!(failure.orientation, 6);
        assert!(failure.shot.is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
