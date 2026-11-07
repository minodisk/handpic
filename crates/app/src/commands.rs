//! The commands the frontend invokes: folder picking, ARW enumeration and
//! preview extraction.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use riffle_core::scan::ScanFocus;
use riffle_core::xmp::LabelNames;
use serde_json::Value;
use tauri::ipc::Response;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_store::StoreExt;

use crate::diagnostics::invariant;
use crate::index::{self, FileStat, Index, IndexedFile, SidecarStat};
use crate::shortcuts::{Binding, Keymap};
use crate::sidecar::{SidecarFormat, Writer};
use crate::trash;

/// Size of the header that precedes the JPEG bytes in a `preview` payload.
pub const PREVIEW_HEADER_LEN: usize = 8;

/// Tag identifying the payload kind and version: an IFD0 preview JPEG, v1.
pub const PREVIEW_KIND_JPEG_V1: u16 = 1;

/// Tag for the other payload kind: a cached thumbnail JPEG, v1.
pub const THUMBNAIL_KIND_JPEG_V1: u16 = 2;

/// Size of the header that precedes the pixels in a `focus_crop` payload.
pub const CROP_HEADER_LEN: usize = 32;

/// Tag for the third payload kind: a raw RGBA focus crop, v3. v1 was the same
/// pixels behind a 24-byte header without the timings; v2 left the last word
/// reserved instead of carrying the full JPEG size.
pub const CROP_KIND_RGBA_V3: u16 = 5;

/// The largest crop produced per axis, in JPEG pixels. The payload is raw
/// RGBA, so this caps it at 4 MB; the decode itself barely depends on the
/// size.
pub const CROP_MAX: usize = 1024;

/// Build a `preview` payload: a fixed-size little-endian header followed by the
/// embedded JPEG bytes, copied verbatim.
///
/// Header layout (8 bytes, little-endian):
///
/// | offset | size | field                                       |
/// |--------|------|---------------------------------------------|
/// | 0      | 2    | kind/version tag (`PREVIEW_KIND_JPEG_V1` or |
/// |        |      | `THUMBNAIL_KIND_JPEG_V1`)                   |
/// | 2      | 2    | EXIF Orientation (1..8)                     |
/// | 4      | 4    | reserved, zero                              |
///
/// Width and height are not carried: the JPEG itself has them and
/// `createImageBitmap` reports them.
fn payload(kind: u16, orientation: u16, jpeg: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(PREVIEW_HEADER_LEN + jpeg.len());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&orientation.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(jpeg);
    out
}

/// Build a `focus_crop` payload: a fixed-size little-endian header followed by
/// the crop's RGBA bytes.
///
/// Header layout (32 bytes, little-endian):
///
/// | offset | size | field                                     |
/// |--------|------|-------------------------------------------|
/// | 0      | 2    | kind/version tag (`CROP_KIND_RGBA_V3`)    |
/// | 2      | 2    | EXIF Orientation (1..8)                   |
/// | 4      | 4    | crop width in pixels                      |
/// | 8      | 4    | crop height in pixels                     |
/// | 12     | 4    | point of interest x, in crop pixels       |
/// | 16     | 4    | point of interest y, in crop pixels       |
/// | 20     | 4    | ranged read of the JpgFromRaw, in us      |
/// | 24     | 4    | partial decode, in us                     |
/// | 28     | 2    | full JPEG width in pixels                 |
/// | 30     | 2    | full JPEG height in pixels                |
///
/// The crop is cut in unrotated JPEG coordinates; the Orientation is the one
/// the frontend already applies to the preview.
fn crop_payload(
    orientation: u16,
    crop: &riffle_core::partial::FocusCrop,
    timing: CropTiming,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(CROP_HEADER_LEN + crop.crop.pixels.len());
    out.extend_from_slice(&CROP_KIND_RGBA_V3.to_le_bytes());
    out.extend_from_slice(&orientation.to_le_bytes());
    for value in [
        crop.crop.width,
        crop.crop.height,
        crop.point_x,
        crop.point_y,
    ] {
        out.extend_from_slice(&(value as u32).to_le_bytes());
    }
    out.extend_from_slice(&timing.read_us.to_le_bytes());
    out.extend_from_slice(&timing.decode_us.to_le_bytes());
    out.extend_from_slice(&(crop.crop.image_width as u16).to_le_bytes());
    out.extend_from_slice(&(crop.crop.image_height as u16).to_le_bytes());
    out.extend_from_slice(&crop.crop.pixels);
    out
}

/// The crop size in unrotated JPEG pixels for a viewport of `width` x `height`
/// device pixels: each axis is capped, and the two are swapped for the
/// quarter-turn orientations, where the viewport's width runs along the
/// unrotated JPEG's height.
fn crop_size(orientation: u16, width: usize, height: usize) -> (usize, usize) {
    let w = width.clamp(1, CROP_MAX);
    let h = height.clamp(1, CROP_MAX);
    if matches!(orientation, 6 | 8) {
        (h, w)
    } else {
        (w, h)
    }
}

/// How long the two phases of one `focus_crop` took, in microseconds.
#[derive(Clone, Copy)]
pub struct CropTiming {
    read_us: u32,
    decode_us: u32,
}

/// Cut the 1:1 crop around the focus point out of a file's full-resolution
/// JpgFromRaw, reading it by range rather than loading the whole ARW. The
/// ranged read and the partial decode are timed separately, so that an
/// end-to-end measurement can be split into its parts.
fn read_focus_crop(
    path: &Path,
    width: usize,
    height: usize,
) -> Result<(u16, riffle_core::partial::FocusCrop, CropTiming), String> {
    let started = std::time::Instant::now();
    let (arw, jpeg) =
        riffle_core::reader::read_full(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let read = started.elapsed();
    let (w, h) = crop_size(arw.orientation, width, height);
    let decode_started = std::time::Instant::now();
    let crop = riffle_core::partial::decode_focus_crop(&jpeg, arw.shot.focus, w, h)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let timing = CropTiming {
        read_us: read.as_micros() as u32,
        decode_us: decode_started.elapsed().as_micros() as u32,
    };
    Ok((arw.orientation, crop, timing))
}

/// List the RAW files directly in `dir`, or its JPEGs when it
/// holds no RAW, sorted by file name. Entries that cannot be read are
/// skipped; a directory that cannot be read is an error.
#[cfg(test)]
fn list_arw_in(dir: &Path) -> Result<Vec<String>, String> {
    read_listing(dir, None).map(|listing| listing.files)
}

/// The RAW files of `dir` as `list_arw_in` lists them, plus the sidecars of
/// `format` keyed by lower-cased file name so that a `FOO.XMP` written by
/// another tool is found for `FOO.ARW`.
///
/// One listing of the directory for both, rather than a second `read_dir` or
/// a `stat` of 5000 guessed names: the second open of a folder is meant to
/// cost no more than the listing the RAWs already pay for. `scan_folder` does
/// the same in two halves around the `AppListing` it may reuse.
#[cfg(test)]
fn list_folder_in(
    dir: &Path,
    format: SidecarFormat,
) -> Result<(Vec<String>, HashMap<String, SidecarStat>), String> {
    let listing = read_listing(dir, Some(format))?;
    let sidecars = stat_sidecars(&listing.sidecars);
    Ok((listing.files, sidecars))
}

/// One `read_dir` of a folder: its RAW files (or its JPEGs, see
/// `folders::Media`) sorted by file name, and the sidecars of the format it
/// was listed for as `(lower-cased name, path)`, not yet statted (see
/// `stat_sidecars`), none for a JPEG listing.
#[derive(Debug, PartialEq)]
pub(crate) struct Listing {
    pub(crate) files: Vec<String>,
    pub(crate) sidecars: Vec<(String, PathBuf)>,
}

/// Tells a RAW or JPEG file from the entry's `file_type()`, which comes with
/// the directory listing, rather than a `stat` per entry, which under disk
/// contention (a scan reading the same drive) costs seconds for a few hundred
/// files. A symlinked RAW or JPEG is still listed: only a symlink pays one
/// extra `stat` to follow it.
pub(crate) fn read_listing(dir: &Path, format: Option<SidecarFormat>) -> Result<Listing, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut media = crate::folders::Media::default();
    let mut sidecars = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if crate::folders::is_file(&entry) && media.add(&path, path.clone()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if format.is_some_and(|f| f.matches(&name)) {
            sidecars.push((name.to_lowercase(), path));
        }
    }
    let (mut files, jpegs) = media.listed();
    // A JPEG folder is view-only: nothing next to a JPEG is read or written.
    if jpegs {
        sidecars.clear();
    }
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    let files = files
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(Listing { files, sidecars })
}

/// The stats `reconcile_sidecars_of` compares, keyed by lower-cased file
/// name. A sidecar gone since the listing is left out.
pub(crate) fn stat_sidecars(sidecars: &[(String, PathBuf)]) -> HashMap<String, SidecarStat> {
    sidecars
        .iter()
        .filter_map(|(name, path)| {
            let stat = index::stat(path).ok()?;
            Some((name.clone(), (stat.path, stat.size, stat.mtime_ns)))
        })
        .collect()
}

/// The listing `list_arw` last produced, for the `scan_folder` that follows
/// it on every folder open and resync, so that the opened directory is read
/// with `read_dir` once rather than twice (each listing is the slow part of an
/// open under disk contention).
///
/// It is keyed by the canonical dir, the sidecar format it was listed for and
/// the directory's mtime taken just before the `read_dir`. `scan_folder` stats
/// the directory once and takes the listing only when all three still match
/// (`take_listing`); otherwise it lists on its own. The mtime check closes the
/// window between `list_arw`'s `read_dir` and `scan_folder`'s `watch::set`, in
/// which a file added or removed would not reach the watcher: such a change
/// bumps the directory's mtime, so the stale listing is not reused. On a file
/// system with a coarse mtime (FAT's 2 s) a change within the same tick can
/// slip past the check; the watcher's `folder-changed` -> resync covers the
/// changes after `watch::set`.
#[derive(Default)]
pub struct AppListing(Mutex<Option<CachedListing>>);

struct CachedListing {
    dir: String,
    format: SidecarFormat,
    mtime: std::time::SystemTime,
    listing: Listing,
}

fn dir_mtime(dir: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(dir).and_then(|m| m.modified()).ok()
}

/// The cached listing, taken out of `cached` if it was listed for `dir` and
/// `format` and the directory's mtime is still `mtime`. A listing is reused
/// once: a match leaves `cached` empty. One that does not match is left for
/// the `scan_folder` it may belong to. No mtime (the stat failed) never
/// matches.
fn take_listing(
    cached: &mut Option<CachedListing>,
    dir: &str,
    format: SidecarFormat,
    mtime: Option<std::time::SystemTime>,
) -> Option<Listing> {
    let matches = cached
        .as_ref()
        .is_some_and(|c| c.dir == dir && c.format == format && Some(c.mtime) == mtime);
    if !matches {
        return None;
    }
    cached.take().map(|c| c.listing)
}

/// Largest sidecar the folder-open pass reads. A Lightroom sidecar is tens of
/// KB; anything past this is not a sidecar this app should be parsing, and is
/// left alone rather than failing the open.
pub(crate) const MAX_SIDECAR_BYTES: i64 = 4 * 1024 * 1024;

/// A sidecar the folder-open pass could not use, reported to the frontend.
#[derive(Debug, serde::Serialize)]
pub struct SidecarError {
    /// The RAW file's path, not the sidecar's: it has to match the key the
    /// `sidecar-error` event uses for a write failure so the frontend's
    /// `ErrorList` replaces one with the other instead of showing both. The
    /// sidecar's file name goes in `message`.
    path: String,
    message: String,
}

/// Bring the `ratings` rows of `dir` in line with the sidecars on disk and
/// return the judgments that still have to be written, along with the
/// sidecars that could not be used and the number of `ratings` rows this
/// wrote or cleared (every parsed sidecar counts, even one whose row a
/// `set_rating` in the parse window keeps `store_sidecar_ratings` off).
///
/// The sidecar is the source of truth, so anything whose stat changed since
/// the app last saw it is read back here; a sidecar that cannot be read or
/// parsed leaves its row as it was rather than erroring the open. Under
/// `Both` the file's newest sidecar is the one compared and read (see
/// `sidecar::newest`).
fn reconcile_sidecars_of(
    dir: &str,
    listed: &[String],
    sidecars: &HashMap<String, SidecarStat>,
    index: &Arc<Mutex<Index>>,
    format: SidecarFormat,
    names: &LabelNames,
) -> Result<(Vec<index::DirtyRow>, Vec<SidecarError>, usize), String> {
    let pairs: Vec<(String, Option<SidecarStat>)> = listed
        .iter()
        .map(|path| {
            let stats = format.kinds().iter().filter_map(|kind| {
                let name = kind
                    .sidecar_path(Path::new(path))
                    .file_name()?
                    .to_string_lossy()
                    .to_lowercase();
                sidecars.get(&name).map(|stat| (stat, stat.2))
            });
            (path.clone(), crate::sidecar::newest(stats).cloned())
        })
        .collect();

    let (to_parse, cleared) = index::lock(index).reconcile_sidecars(dir, &pairs)?;
    // An oversize sidecar is rejected outright (see `MAX_SIDECAR_BYTES`), so
    // its row cannot be brought in line with it here. Its path is tracked
    // separately so it can be kept out of what the writer is handed below:
    // otherwise a dirty row for it would have the writer patch a sidecar
    // whose external edit was never read, overwriting it unread.
    let oversize: std::collections::HashSet<&str> = to_parse
        .iter()
        .filter(|(_, (_, size, _), _)| *size > MAX_SIDECAR_BYTES)
        .map(|(path, _, _)| path.as_str())
        .collect();
    let mut parsed: Vec<index::ParsedSidecar> = Vec::new();
    let mut problems: Vec<SidecarError> = Vec::new();
    for (path, (sidecar, size, mtime_ns), dirty) in &to_parse {
        let problem = |message: String| SidecarError {
            path: path.clone(),
            message: format!(
                "{}: {message}",
                sidecar
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_else(|| sidecar.to_string_lossy())
            ),
        };
        if oversize.contains(path.as_str()) {
            problems.push(problem(format!(
                "larger than {} MiB, not read",
                MAX_SIDECAR_BYTES / 1024 / 1024
            )));
            continue;
        }
        let kind = if sidecar
            .file_name()
            .is_some_and(|n| SidecarFormat::Dop.matches(&n.to_string_lossy()))
        {
            SidecarFormat::Dop
        } else {
            SidecarFormat::Xmp
        };
        let read = std::fs::read(sidecar)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                Ok((
                    kind.read_rating(&bytes)?,
                    kind.read_flag(&bytes)?,
                    kind.read_label(&bytes, names)?,
                ))
            });
        match read {
            Ok((rating, flag, label)) => {
                parsed.push((path.clone(), rating, flag, label, *size, *mtime_ns, *dirty))
            }
            Err(e) => problems.push(problem(e)),
        }
    }
    let mut index = index::lock(index);
    index.store_sidecar_ratings(dir, &parsed)?;
    // After storing, so a row the sidecar just won stays out of this: only
    // what still has nowhere to be read back from is written out. An
    // oversize sidecar's row is excluded too, even if still dirty, since the
    // writer must not patch a sidecar whose contents were never read.
    let dirty = index.dirty_rows(dir)?;
    let dirty = dirty
        .into_iter()
        .filter(|(path, _, _, _, _)| !oversize.contains(path.as_str()))
        .collect();
    Ok((dirty, problems, cleared + parsed.len()))
}

/// Extract a file's IFD0 preview JPEG along with the Orientation, reading only
/// the bounded prefix the metadata and the preview need rather than the whole
/// 48 MB file.
pub(crate) fn read_preview(path: &Path) -> Result<(u16, Vec<u8>), String> {
    let (arw, jpeg) =
        riffle_core::reader::read_preview(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((arw.orientation, jpeg))
}

/// The faces `faces_of` found, each with its YuNet score and its five
/// landmarks (the eyes, the nose tip and the mouth corners, each pair left of
/// the image first), in the preview's stored pixel coordinates.
#[derive(Debug, serde::Serialize)]
pub struct FacesResponse {
    width: usize,
    height: usize,
    faces: Vec<FaceBox>,
}

#[derive(Debug, serde::Serialize)]
struct FaceBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    score: f32,
    left_eye: FacePoint,
    right_eye: FacePoint,
    nose: FacePoint,
    mouth_left: FacePoint,
    mouth_right: FacePoint,
}

#[derive(Debug, serde::Serialize)]
struct FacePoint {
    x: f32,
    y: f32,
}

impl From<(f32, f32)> for FacePoint {
    fn from((x, y): (f32, f32)) -> Self {
        FacePoint { x, y }
    }
}

/// Refuses `what` on a file that is not a RAW file: a JPEG folder is
/// view-only, so no client can rate it, write a sidecar next to it or run
/// face detection on its full-size image.
fn raw_only(path: &Path, what: &str) -> Result<(), String> {
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
fn read_faces(path: &Path) -> Result<FacesResponse, String> {
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
    eyes: Option<EyesJudgment>,
    superseded: bool,
    read_us: u32,
    decode_us: u32,
    detect_us: u32,
    model_us: u32,
}

#[derive(Debug, PartialEq, serde::Serialize)]
struct EyesJudgment {
    state: &'static str,
    /// The probability that the eyes are closed, 0..1.
    probability: f64,
    /// The EAR of the more closed eye the judgment was read from.
    ear: f64,
    /// The head pose of the judged face (`pose::Pose`), `None` when the
    /// solve failed.
    pose: Option<index::EyesPose>,
    mesh: EyesMesh,
}

/// The face mesh the judgment was taken on, in the preview's stored pixel
/// coordinates like `FacesResponse`.
#[derive(Debug, PartialEq, serde::Serialize)]
struct EyesMesh {
    width: usize,
    height: usize,
    points: Vec<[f32; 2]>,
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
fn read_eyes(path: &Path, current: impl Fn() -> bool) -> Result<EyesResponse, String> {
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
        pose: j.pose.map(index::EyesPose::from),
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

/// Open the native folder picker and resolve once the user answers, or `None`
/// if they cancel.
///
/// The dialog is driven by the main thread's run loop, so the blocking variant
/// must not be used: a synchronous command runs on the main thread, and waiting
/// there freezes the very loop that delivers the dialog's button events. The
/// callback form hands the answer back over a channel this async command awaits
/// instead.
#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle) -> Option<String> {
    let (tx, mut rx) = tauri::async_runtime::channel(1);
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.try_send(path);
    });
    rx.recv().await.flatten().map(|p| p.to_string())
}

/// The settings store. Settings are preferences, not a cache, so the file
/// lives in the config dir rather than beside the index.
pub(crate) fn settings(
    app: &tauri::AppHandle,
) -> Result<Arc<tauri_plugin_store::Store<tauri::Wry>>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    app.store(dir.join("settings.json"))
        .map_err(|e| e.to_string())
}

/// Read and remove the plain file the last folder was kept in before the
/// settings store existed. `None` when there is none.
fn take_legacy_last_folder(file: &Path) -> Option<String> {
    let dir = std::fs::read_to_string(file).ok()?;
    if let Err(e) = std::fs::remove_file(file) {
        log::warn!("failed to remove {}: {e}", file.display());
    }
    Some(dir)
}

/// Load the settings at launch: move a legacy `last_folder` file into the
/// store once, then return the selected sidecar format. A store that cannot
/// be read is logged and falls back to the defaults, followed by the keymap,
/// the `autoAdvance` setting, the `labelNames` setting, the `mcpEnabled`
/// setting and the `aiEnabled` setting.
pub fn load_settings(
    app: &tauri::AppHandle,
) -> (SidecarFormat, Keymap, bool, LabelNames, bool, bool) {
    let store = match settings(app) {
        Ok(store) => store,
        Err(e) => {
            log::warn!("failed to open the settings: {e}");
            let format = SidecarFormat::default();
            return (
                format,
                Keymap::defaults(),
                auto_advance_setting(None),
                LabelNames::default(),
                mcp_enabled_setting(None),
                ai_enabled_setting(None),
            );
        }
    };
    if !store.has("lastFolder") {
        let legacy = app
            .path()
            .app_config_dir()
            .ok()
            .and_then(|dir| take_legacy_last_folder(&dir.join("last_folder")));
        if let Some(dir) = legacy {
            store.set("lastFolder", dir);
            if let Err(e) = store.save() {
                log::warn!("failed to save the settings: {e}");
            }
        }
    }
    let format =
        SidecarFormat::from_setting(store.get("sidecarFormat").as_ref().and_then(|v| v.as_str()));
    let keymap = Keymap::from_overrides(store.get("shortcuts").as_ref());
    let auto_advance = auto_advance_setting(store.get("autoAdvance").as_ref());
    let names = label_names_setting(store.get("labelNames").as_ref());
    let mcp_enabled = mcp_enabled_setting(store.get("mcpEnabled").as_ref());
    let ai_enabled = ai_enabled_setting(store.get("aiEnabled").as_ref());
    (format, keymap, auto_advance, names, mcp_enabled, ai_enabled)
}

/// The stored `autoAdvance` value; missing or non-boolean means on.
fn auto_advance_setting(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(true)
}

/// The stored `mcpEnabled` value; missing or non-boolean means off.
fn mcp_enabled_setting(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(false)
}

/// The stored `aiEnabled` value; missing or non-boolean means off.
fn ai_enabled_setting(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(false)
}

/// The stored `labelNames` value: an object keyed by lowercase color whose
/// missing, non-string or blank entries are the English default.
fn label_names_setting(value: Option<&Value>) -> LabelNames {
    let name = |color: &str, default: String| {
        value
            .and_then(|v| v.get(color))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map_or(default, str::to_string)
    };
    let d = LabelNames::default();
    LabelNames {
        red: name("red", d.red),
        yellow: name("yellow", d.yellow),
        green: name("green", d.green),
        blue: name("blue", d.blue),
        purple: name("purple", d.purple),
    }
}

/// `names` in the shape stored under `labelNames`.
fn label_names_value(names: &LabelNames) -> Value {
    serde_json::json!({
        "red": names.red,
        "yellow": names.yellow,
        "green": names.green,
        "blue": names.blue,
        "purple": names.purple,
    })
}

/// Switch the sidecar format to `format`: apply the new format first so any
/// rating set while this runs is queued in it, drain what the writer still
/// holds queued in the old format, persist the new format, and reset the
/// index so the next folder open reads the new format's sidecars. Blocks on
/// the drain and SQLite, so it must not run on the main thread.
///
/// Holds `AppSwitchLock` for its whole body and re-checks the current format
/// once inside it, so two switches started back to back cannot interleave
/// and the second one is a no-op when it already matches.
pub fn switch_sidecar_format(app: &tauri::AppHandle, format: SidecarFormat) -> Result<(), String> {
    let switch_lock = app.state::<AppSwitchLock>();
    let _guard = index::lock(&switch_lock.0);
    let current = app.state::<AppSidecarFormat>();
    let writer = app.state::<AppWriter>();
    let index = app.state::<AppIndex>();
    switch_format(
        &current.0,
        writer.0.as_ref(),
        index.0.as_ref(),
        format,
        |format| save_sidecar_format(app, format),
    )
}

/// Switch the sidecar format to `format` as `switch_sidecar_format` does, but
/// persist it even when it is already the current one, and return a save
/// failure instead of logging it. The first-launch dialog uses this: choosing
/// XMP on a fresh store changes nothing in memory, yet it has to be saved so
/// the dialog does not come back.
pub fn choose_sidecar_format(app: &tauri::AppHandle, format: SidecarFormat) -> Result<(), String> {
    let switch_lock = app.state::<AppSwitchLock>();
    let _guard = index::lock(&switch_lock.0);
    let current = app.state::<AppSidecarFormat>();
    let writer = app.state::<AppWriter>();
    let index = app.state::<AppIndex>();
    choose_format(
        &current.0,
        writer.0.as_ref(),
        index.0.as_ref(),
        format,
        |format| save_sidecar_format(app, format),
    )
}

fn save_sidecar_format(app: &tauri::AppHandle, format: SidecarFormat) -> Result<(), String> {
    settings(app).and_then(|store| {
        store.set("sidecarFormat", format.setting());
        store.save().map_err(|e| e.to_string())
    })
}

/// The body of `choose_sidecar_format`, without the `AppHandle`.
fn choose_format(
    current: &Mutex<SidecarFormat>,
    writer: Option<&Writer>,
    index: Option<&Arc<Mutex<Index>>>,
    format: SidecarFormat,
    persist: impl FnOnce(SidecarFormat) -> Result<(), String>,
) -> Result<(), String> {
    switch_format(current, writer, index, format, |_| Ok(()))?;
    persist(format)
}

/// The body of `switch_sidecar_format`, without the `AppHandle`: the state
/// write, the drain, `persist` (a save failure is logged, not returned) and
/// the index reset. The caller serializes switches (the command does it with
/// `AppSwitchLock`); this function does no locking of its own beyond the
/// state and index mutexes.
fn switch_format(
    current: &Mutex<SidecarFormat>,
    writer: Option<&Writer>,
    index: Option<&Arc<Mutex<Index>>>,
    format: SidecarFormat,
    persist: impl FnOnce(SidecarFormat) -> Result<(), String>,
) -> Result<(), String> {
    if *index::lock(current) == format {
        return Ok(());
    }
    *index::lock(current) = format;
    if let Some(writer) = writer {
        writer.flush(crate::sidecar::DRAIN_TIMEOUT);
    }
    if let Err(e) = persist(format) {
        log::warn!("failed to save the sidecar format: {e}");
    }
    match index {
        Some(index) => index::lock(index).reset_sidecars(),
        None => Ok(()),
    }
}

/// Remember `dir` as the folder to reopen on the next launch. Failing to write
/// it only means starting with nothing open, so it is logged, not returned.
#[tauri::command]
pub async fn remember_folder(app: tauri::AppHandle, dir: String) {
    let saved = tauri::async_runtime::spawn_blocking(move || {
        settings(&app).and_then(|store| {
            store.set("lastFolder", dir);
            store.save().map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|saved| saved);
    if let Err(e) = saved {
        log::warn!("failed to remember the folder: {e}");
    }
}

/// The folder remembered by `remember_folder`, or `None` if there is none or
/// it is no longer a directory.
#[tauri::command]
pub fn last_folder(app: tauri::AppHandle) -> Option<String> {
    let dir = settings(&app)
        .ok()?
        .get("lastFolder")?
        .as_str()?
        .to_string();
    Path::new(&dir).is_dir().then_some(dir)
}

/// The strip sort order a stored `sortOrder` value names; anything unknown or
/// missing is the default, file name order.
fn parse_sort_order(value: Option<&str>) -> &'static str {
    match value {
        Some("capture") => "capture",
        Some("rating") => "rating",
        Some("recommended") => "recommended",
        _ => "name",
    }
}

/// The strip sort order remembered by `set_sort_order`.
#[tauri::command]
pub fn sort_order(app: tauri::AppHandle) -> &'static str {
    let store = settings(&app).ok();
    parse_sort_order(
        store
            .as_ref()
            .and_then(|s| s.get("sortOrder"))
            .as_ref()
            .and_then(|v| v.as_str()),
    )
}

/// Remember the strip sort order. Failing to write it only means starting in
/// file name order, so it is logged, not returned.
#[tauri::command]
pub fn set_sort_order(app: tauri::AppHandle, order: String) {
    let saved = settings(&app).and_then(|store| {
        store.set("sortOrder", parse_sort_order(Some(&order)));
        store.save().map_err(|e| e.to_string())
    });
    if let Err(e) = saved {
        log::warn!("failed to remember the sort order: {e}");
    }
}

#[tauri::command]
pub async fn list_arw(app: tauri::AppHandle, dir: String) -> Result<Vec<String>, String> {
    let format = *index::lock(&app.state::<AppSidecarFormat>().0);
    tauri::async_runtime::spawn_blocking(move || {
        let dir = canonicalize(&dir);
        let started = std::time::Instant::now();
        let mtime = dir_mtime(Path::new(&dir));
        let listing = read_listing(Path::new(&dir), Some(format))?;
        let files = listing.files.clone();
        log::info!(
            "open list: dir={dir} raws={} in {}ms",
            files.len(),
            started.elapsed().as_millis()
        );
        *index::lock(&app.state::<AppListing>().0) = mtime.map(|mtime| CachedListing {
            dir,
            format,
            mtime,
            listing,
        });
        Ok(files)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The folder a dropped path stands for: a directory is taken as it is, a
/// file is taken by its parent directory (dragging one ARW is the obvious
/// gesture; a file at a filesystem root resolves to that root, since
/// `Path::parent` only yields `None` for the root itself). Anything else — a
/// path that is neither a directory nor a regular file, such as one gone by
/// the time it lands, or a broken symlink or socket — is `None`.
fn dropped_dir(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return Some(path.to_path_buf());
    }
    if path.is_file() {
        return path.parent().map(Path::to_path_buf);
    }
    None
}

/// Resolve a path dropped on the window to the folder to open. Whether the
/// path is a directory can only be answered by a `stat`, so the frontend asks
/// instead of guessing from the string.
#[tauri::command]
pub async fn dropped_folder(path: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        dropped_dir(Path::new(&path)).map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())
}

/// The metadata shown in the right pane: the file name plus the shooting
/// settings, already formatted for display so the frontend only lays them out.
/// Fields the file does not carry are `None` and are left out of the pane.
#[derive(serde::Serialize)]
pub struct Metadata {
    name: String,
    camera: Option<String>,
    lens: Option<String>,
    aperture: Option<String>,
    shutter: Option<String>,
    shutter_type: Option<String>,
    focus_mode: Option<String>,
    af_area: Option<String>,
    af_tracking: Option<String>,
    drive: Option<String>,
    stabilization: Option<String>,
    exposure_mode: Option<String>,
    metering: Option<String>,
    creative_style: Option<String>,
    dro: Option<String>,
    raw_type: Option<String>,
    iso: Option<String>,
    focal_length: Option<String>,
    exposure_bias: Option<String>,
    focus_distance: Option<String>,
    captured_at: Option<String>,
}

pub(crate) fn read_metadata(path: &Path) -> Result<Metadata, String> {
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let arw =
        riffle_core::reader::read_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let shot = arw.shot;
    let exif = crate::exif::exif(&shot);
    let label = |l: Option<crate::exif::Labeled>| l.map(|l| l.label);
    Ok(Metadata {
        name,
        camera: exif.camera,
        lens: exif.lens,
        aperture: label(exif.aperture),
        shutter: label(exif.shutter),
        // Sony has no shutter type tag on ILCE bodies; EFCS is assumed always
        // enabled, so Off means the electronic shutter. A fully mechanical
        // shutter with EFCS disabled would also read Off.
        shutter_type: shot.electronic_front_curtain.and_then(|v| match v {
            1 => Some("Mechanical".to_owned()),
            0 => Some("Electronic".to_owned()),
            _ => None,
        }),
        focus_mode: exif.focus_mode,
        af_area: exif.af_area,
        af_tracking: exif.af_tracking,
        drive: exif.drive,
        stabilization: exif.stabilization,
        exposure_mode: exif.exposure_mode,
        metering: exif.metering,
        creative_style: exif.creative_style,
        dro: exif.dro,
        raw_type: exif.raw_type,
        iso: label(exif.iso),
        focal_length: label(exif.focal_length),
        exposure_bias: shot.exposure_bias.and_then(|r| {
            let v = r.value()?;
            let text = crate::exif::decimal(r, 1)?;
            Some(if v > 0.0 {
                format!("+{text} EV")
            } else {
                format!("{text} EV")
            })
        }),
        focus_distance: shot
            .focus_distance_mm
            .map(|mm| format!("{:.2} m", f64::from(mm) / 1000.0)),
        captured_at: shot.capture_time,
    })
}

/// The metadata of one file, read from the same bounded prefix as `preview`.
#[tauri::command]
pub async fn metadata(path: String) -> Result<Metadata, String> {
    tauri::async_runtime::spawn_blocking(move || read_metadata(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn preview(path: String) -> Result<Response, String> {
    let bytes = tauri::async_runtime::spawn_blocking(move || read_preview(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())??;
    Ok(Response::new(payload(
        PREVIEW_KIND_JPEG_V1,
        bytes.0,
        &bytes.1,
    )))
}

/// The 1:1 crop of one file around its focus point, as raw RGBA. `width` and
/// `height` are the viewport in device pixels.
#[tauri::command]
pub async fn focus_crop(path: String, width: u32, height: u32) -> Result<Response, String> {
    let (orientation, crop, timing) = tauri::async_runtime::spawn_blocking(move || {
        read_focus_crop(Path::new(&path), width as usize, height as usize)
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(Response::new(crop_payload(orientation, &crop, timing)))
}

/// The faces the focus mark draws for one file, detected when it is shown
/// rather than at scan time. Touches neither the index nor the stored state.
#[tauri::command]
pub async fn faces_of(path: String) -> Result<FacesResponse, String> {
    tauri::async_runtime::spawn_blocking(move || read_faces(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
}

/// The id of the latest `eyes_of` request, the pattern of
/// `ScansState::latest_id`: a judgment a newer request has superseded stops
/// at its next stage instead of running the model for a file no longer shown.
#[derive(Default)]
pub struct EyesRequests(Mutex<u64>);

/// Whether the eyes of one file's AF face are closed, judged when it is
/// shown rather than at scan time. Touches neither the index nor the stored
/// state.
#[tauri::command]
pub async fn eyes_of(
    app: tauri::AppHandle,
    path: String,
    request: u64,
) -> Result<EyesResponse, String> {
    *index::lock(&app.state::<EyesRequests>().0) = request;
    tauri::async_runtime::spawn_blocking(move || {
        let requests = app.state::<EyesRequests>();
        read_eyes(Path::new(&path), || *index::lock(&requests.0) == request)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The id handed out to each `scan_folder` call, the cancel flag and join
/// handle of the one scan that may be running, and the work queued for
/// `start_scan` to actually spawn — all behind one lock so that minting an
/// id, taking `running`, checking the latest id and storing into `running`
/// or `pending` are never interleaved between two concurrent `scan_folder`/
/// `start_scan` calls (the two `scan_folder` futures are independent IPC
/// tasks with no ordering guarantee between them, and likewise for
/// `start_scan`).
#[derive(Default)]
pub struct Scans(pub(crate) Mutex<ScansState>);

#[derive(Default)]
pub(crate) struct ScansState {
    next_id: u64,
    /// The id `scan_folder` most recently handed out. Both `scan_folder`
    /// (before inserting into `pending`) and `start_scan` (before storing
    /// into `running`) re-check against this so an id that has since been
    /// superseded is refused rather than acted on late.
    latest_id: u64,
    /// The scan `start_scan` most recently spawned, with the id it was
    /// spawned under so the task's own clean-up can tell whether the entry
    /// is still its own, and the focus handle both its passes take their
    /// files in the order of. `Some` does not mean a scan is running; see
    /// `scanning`.
    running: Option<RunningScan>,
    pending: HashMap<u64, PendingScan>,
    /// The number of `scan_folder` calls currently past minting their id but
    /// not yet done (listing, `reconcile`, `reconcile_sidecars_of`), i.e.
    /// past the point where `running` is invisible for them. `clear_index`
    /// refuses while this is nonzero, or it could run against a folder that
    /// is mid-open with no scan indicated by `running`. See `Preparing`.
    preparing: u32,
    /// The id of the scan in `running` once it has reached its third pass
    /// (`run_embed_pass`). That pass writes only `embeddings`, so it does not
    /// count as scanning, and a `scan_folder` that joins it reports no change.
    embedding: Option<u64>,
}

impl ScansState {
    fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Whether a scan is genuinely in progress: a `scan_folder` past minting
    /// its id and not yet returned, a prepared scan waiting for `start_scan`,
    /// or a spawned scan task that has not yet run to completion, short of
    /// its third pass. The single definition every guard uses.
    pub(crate) fn scanning(&self) -> bool {
        self.preparing > 0
            || (self.running.is_some() && !self.in_embed_pass())
            || !self.pending.is_empty()
    }

    /// Whether the scan in `running` is in its third pass.
    fn in_embed_pass(&self) -> bool {
        self.running
            .as_ref()
            .is_some_and(|(id, _, _, _)| self.embedding == Some(*id))
    }

    /// Record that the scan spawned under `scan_id` entered its third pass,
    /// if it is still the one stored.
    fn start_embedding(&mut self, scan_id: u64) {
        if self
            .running
            .as_ref()
            .is_some_and(|(id, _, _, _)| *id == scan_id)
        {
            self.embedding = Some(scan_id);
        }
    }

    /// Cancel the running scan if it is in its third pass, which then stops
    /// at its next file; its stored embeddings stay. Called when `aiEnabled`
    /// turns off and before an operation that deletes or moves index rows
    /// (`clear_index`, the renames). It only sets the flag: a batch already
    /// waiting on the index lock may still land afterwards, and the index's
    /// `drop_stray_embeddings` covers only rows without a live folder.
    pub(crate) fn stop_embedding(&self) {
        if let Some((_, cancel, _, _)) = self.running.as_ref().filter(|_| self.in_embed_pass()) {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Cancel the scan spawned under `scan_id`, short of its third pass. A
    /// stale or unknown id is ignored, and so is a scan in that pass, which
    /// `stop_embedding` covers and no guard waits for. It only sets the flag:
    /// `scanning()` stays true until the task's own `finish`.
    fn cancel(&self, scan_id: u64) {
        if let Some((id, cancel, _, _)) = &self.running {
            if *id == scan_id && !self.in_embed_pass() {
                cancel.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Drop the entry of the scan spawned under `scan_id`, if it is still the
    /// one stored. A later `scan_folder` may have taken it and a later
    /// `start_scan` stored a newer one, which this must not clear.
    fn finish(&mut self, scan_id: u64) {
        if self
            .running
            .as_ref()
            .is_some_and(|(id, _, _, _)| *id == scan_id)
        {
            self.running = None;
        }
        if self.embedding == Some(scan_id) {
            self.embedding = None;
        }
    }

    /// Hand `paths` to the running scan's focus if `scan_id` is its id; a
    /// stale or unknown id is ignored.
    fn set_focus(&self, scan_id: u64, paths: Vec<String>) {
        if let Some((id, _, focus, _)) = &self.running {
            if *id == scan_id {
                focus.set(paths);
            }
        }
    }
}

type RunningScan = (
    u64,
    Arc<AtomicBool>,
    ScanFocus,
    tauri::async_runtime::JoinHandle<()>,
);

struct PendingScan {
    dir: String,
    todo: Vec<FileStat>,
    cancel: Arc<AtomicBool>,
}

/// Marks one `scan_folder` call as being in its prepare phase (listing,
/// `reconcile`, `reconcile_sidecars_of`) by holding `ScansState::preparing`
/// above zero for as long as it is alive. `scan_folder` mints its id, takes
/// `running` and spends this whole phase with no lock held and `running ==
/// None`, so without this, a `clear_index` issued while a folder is opening
/// would pass both of its checks and run against the folder's
/// soon-to-be-reconciled rows. `start` increments under the very guard that
/// mints the id and takes `running`, so `preparing` is never zero while that
/// guard is released and the previous scan's task is still alive; the
/// decrement on drop covers every early return of `scan_folder` as well as
/// its normal one.
struct Preparing {
    app: tauri::AppHandle,
}

impl Preparing {
    /// Increments `preparing` under `state`, the same guard the caller used
    /// to mint the scan id and take `running`.
    fn start(app: tauri::AppHandle, state: &mut ScansState) -> Self {
        state.preparing += 1;
        Self { app }
    }
}

impl Drop for Preparing {
    fn drop(&mut self) {
        let scans = self.app.state::<Scans>();
        let mut state = index::lock(&scans.0);
        state.preparing -= 1;
    }
}

/// The index, shared between the commands and the scan thread. It is a cache:
/// `None` when the cache directory could not be opened (see `main.rs`), in
/// which case the commands below degrade to "no thumbnails" instead of the
/// app failing to launch.
pub struct AppIndex(pub Option<Arc<Mutex<Index>>>);

/// A read-only connection on the same index for `folder_entries` and
/// `thumbnail`, so they do not wait behind the scan's write transactions. It
/// is the writer's `Arc` when the reader could not be opened.
pub struct AppIndexReader(pub Option<Arc<Mutex<Index>>>);

/// Evict stale folders from the index once, on a thread of its own. It holds
/// the `Scans` lock throughout and skips if a scan has already been started,
/// so no scan runs during the `VACUUM`: a `scan_folder`/`start_scan` issued
/// meanwhile waits for it (~0.2 s on a ~100 MB index, M3 Pro).
pub fn spawn_eviction(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let Some(index) = app.state::<AppIndex>().0.clone() else {
            return;
        };
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.running.is_some() {
            return;
        }
        match index::lock(&index).evict(index::now_secs(), index::EVICT_POLICY) {
            Ok(summary) => log::info!("index eviction: {summary:?}"),
            Err(e) => log::error!("failed to evict stale folders from the index: {e}"),
        }
        drop(state);
    });
}

/// Threads the scan runs on: two fewer than the cores. Step 3 measured that
/// this costs ~10% of scan throughput against using every core, and leaves two
/// cores for the paging path so the app stays responsive while scanning; more
/// threads than cores did not help and doubled the per-file p95.
fn scan_threads() -> usize {
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    cores.saturating_sub(2).max(1)
}

/// Threads the third pass runs on: a quarter of the cores. It runs while the
/// user culls, after `faces-done`. On 2134 ARW files (24 logical cores), a
/// preview read + full decode on a normal-priority thread took a median 10
/// ms idle, 33 ms during the pass on 22 threads (41 s), 22 ms on 11 (56 s)
/// and 15 ms on 6 (70 s), so it trades a longer pass for paging. Memory does
/// not set it: the model's load sets the peak whatever the count.
fn embed_threads() -> usize {
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    (cores / 4).max(1)
}

/// What `scan_folder` returns: the number of files that need scanning and the
/// id the caller must match against `scan-progress`/`scan-done` and
/// `faces-progress`/`faces-done` events to tell this scan's events apart from
/// an older, still-draining one for the same folder. `sidecar_errors` lists
/// the sidecars the open could not read. `changed` counts the index rows the
/// prepare phase changed: the `files` rows `Index::reconcile` dropped plus
/// the `ratings` rows `reconcile_sidecars_of` wrote or cleared, plus 1 when a
/// previous scan was still running and got joined here, whatever folder it
/// was scanning (its `dir` is not compared): a cancelled scan's in-flight
/// batch can still land after this scan's reconcile read, so if it was
/// scanning the same folder its rows must not be assumed unchanged. Counting
/// every joined scan, even one of a different folder, is a deliberate
/// over-count that costs one extra refresh on a folder switch mid-scan.
#[derive(serde::Serialize)]
pub struct ScanStarted {
    total: usize,
    scan_id: u64,
    sidecar_errors: Vec<SidecarError>,
    changed: usize,
}

/// Bring the index of `dir` up to date: wait for a previous scan's last write
/// to land, then drop the rows of files that are gone or changed and report
/// how many have no valid row. The actual scan does not start until the
/// frontend calls `start_scan` with the returned `scan_id`, so this only
/// prepares the work; call `start_scan` right after storing the id. A no-op
/// (nothing to scan) when the index cache is unavailable, or when neither the
/// scan pass nor the faces pass has a file to do; `start_scan` still emits
/// `scan-done` and `faces-done` for this `scan_id` in those cases.
#[tauri::command]
pub async fn scan_folder(app: tauri::AppHandle, dir: String) -> Result<ScanStarted, String> {
    // A trash run holds the `Scans` lock for as long as its move takes.
    let (scan_id, previous, previous_embedding, _preparing) = {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let scans = app.state::<Scans>();
            let mut state = index::lock(&scans.0);
            let scan_id = state.next_id();
            state.latest_id = scan_id;
            let previous_embedding = state.in_embed_pass();
            let previous = state.running.take();
            let preparing = Preparing::start(app.clone(), &mut state);
            (scan_id, previous, previous_embedding, preparing)
        })
        .await
        .map_err(|e| e.to_string())?
    };
    let canonical = canonicalize(&dir);
    crate::watch::set(&app, &canonical, &dir);
    let dir = canonical;
    let cancel = Arc::new(AtomicBool::new(false));

    // A scan joined in its third pass wrote no `files` row since its
    // `faces-done`, so it changes nothing the frontend shows.
    let joined_previous = previous.is_some() && !previous_embedding;
    if let Some((_, previous_cancel, _, previous_handle)) = previous {
        previous_cancel.store(true, Ordering::Relaxed);
        let _ = previous_handle.await;
    }

    let Some(index) = app.state::<AppIndex>().0.clone() else {
        return Ok(ScanStarted {
            total: 0,
            scan_id,
            sidecar_errors: Vec::new(),
            changed: 0,
        });
    };

    let format = *index::lock(&app.state::<AppSidecarFormat>().0);
    let names = index::lock(&app.state::<AppLabelNames>().0).clone();
    let scan_started = std::time::Instant::now();
    let (listed, sidecars, reused) = {
        let (dir, app) = (dir.clone(), app.clone());
        tauri::async_runtime::spawn_blocking(move || {
            let path = Path::new(&dir);
            let mtime = dir_mtime(path);
            let cached = take_listing(
                &mut index::lock(&app.state::<AppListing>().0),
                &dir,
                format,
                mtime,
            );
            let reused = cached.is_some();
            let listing = match cached {
                Some(listing) => listing,
                None => read_listing(path, Some(format))?,
            };
            let sidecars = stat_sidecars(&listing.sidecars);
            Ok::<_, String>((listing.files, sidecars, reused))
        })
        .await
        .map_err(|e| e.to_string())??
    };
    log::info!(
        "scan list: dir={dir} scan_id={scan_id} raws={} sidecars={} reused={reused} in {}ms",
        listed.len(),
        sidecars.len(),
        scan_started.elapsed().as_millis()
    );

    let reconcile_started = std::time::Instant::now();
    let embedding = app.state::<crate::ai::AppAi>().model_if_enabled().is_some();
    let (todo, removed, idle) = {
        let (dir, index, listed) = (dir.clone(), index.clone(), listed.clone());
        tauri::async_runtime::spawn_blocking(move || {
            let files: Vec<_> = listed
                .iter()
                .filter_map(|p| index::stat(Path::new(p)).ok())
                .collect();
            let mut index = index::lock(&index);
            let (todo, removed) = index.reconcile(&dir, &files)?;
            // No pass has anything to do, so no scan task is queued. A
            // failed faces or embeddings listing leaves it to that pass to
            // report.
            let idle = todo.is_empty()
                && index.faces_todo(&dir).is_ok_and(|faces| faces.is_empty())
                && (!embedding
                    || index
                        .embeddings_missing(&dir)
                        .is_ok_and(|missing| missing.is_empty()));
            Ok::<_, String>((todo, removed, idle))
        })
        .await
        .map_err(|e| e.to_string())??
    };

    log::info!(
        "scan reconcile: dir={dir} scan_id={scan_id} todo={} removed={removed} in {}ms",
        todo.len(),
        reconcile_started.elapsed().as_millis()
    );

    let sidecars_started = std::time::Instant::now();
    let dirty = {
        let (dir, index) = (dir.clone(), index);
        let names = names.clone();
        tauri::async_runtime::spawn_blocking(move || {
            reconcile_sidecars_of(&dir, &listed, &sidecars, &index, format, &names)
        })
        .await
        .map_err(|e| e.to_string())?
    };
    let sidecars_ms = sidecars_started.elapsed().as_millis();
    let mut sidecar_errors = Vec::new();
    let mut dirty_count = 0;
    let mut changed = 0;
    match dirty {
        // A dirty row waited out its debounce in an earlier session already,
        // so it goes to the writer with none. Its own `label_known` (see
        // `Index::set_rating`/`mark_written`) travels with it: a row created
        // before the app learned the label, and never written before this
        // open (e.g. a crash), must still reach the writer as unknown, or an
        // existing sidecar label would be stripped.
        Ok((dirty, problems, rows)) => {
            sidecar_errors = problems;
            dirty_count = dirty.len();
            changed = rows;
            if let Some(writer) = &app.state::<AppWriter>().0 {
                for (path, rating, flag, label, label_known) in dirty {
                    if let Err(e) = writer.set_now(
                        PathBuf::from(path),
                        rating,
                        flag,
                        label,
                        label_known,
                        format,
                        names.clone(),
                    ) {
                        log::error!("failed to queue a pending sidecar: {e}");
                    }
                }
            }
        }
        // The sidecars are a cache layer over the folder; failing to read them
        // must not stop the folder from opening.
        Err(e) => log::error!("failed to reconcile the sidecars of {dir}: {e}"),
    }
    log::info!(
        "scan sidecars: dir={dir} scan_id={scan_id} dirty={dirty_count} changed={changed} in {sidecars_ms}ms"
    );
    let changed = removed + changed + usize::from(joined_previous);
    log::info!(
        "scan prepare: dir={dir} scan_id={scan_id} todo={} in {}ms",
        todo.len(),
        scan_started.elapsed().as_millis()
    );

    let total = todo.len();
    let scans = app.state::<Scans>();
    let mut state = index::lock(&scans.0);
    if state.latest_id != scan_id {
        // A later `scan_folder` has superseded this one while it was
        // listing/reconciling; its own `start_scan` (or none at all) owns
        // `pending`/`running` now, so queuing this work would either be
        // overwritten or, worse, race the id check below in `start_scan`.
        let latest = state.latest_id;
        drop(state);
        log::info!("scan superseded: dir={dir} scan_id={scan_id} by={latest}");
        return Ok(ScanStarted {
            total,
            scan_id,
            sidecar_errors,
            changed,
        });
    }
    // Any entry still here belongs to a scan this one has already superseded
    // (only one `scan_folder`/`start_scan` pair is ever live at a time); it
    // never started, so there is nothing to cancel or join, just drop it.
    state.pending.clear();
    if !idle {
        state
            .pending
            .insert(scan_id, PendingScan { dir, todo, cancel });
    }
    drop(state);
    Ok(ScanStarted {
        total,
        scan_id,
        sidecar_errors,
        changed,
    })
}

/// Emit an empty `scan-done`, `faces-done` and `embed-done`, for a `scan_id`
/// `start_scan` is not going to spawn a real scan for. The frontend clears
/// its `scanRunning` flag off `faces-done`, so every `scan_id` it hands a scan
/// for must eventually get one, even the ids that turn out to be no-ops here.
fn emit_empty_scan_events(app: &tauri::AppHandle, dir: &str, scan_id: u64) {
    let done = Done {
        dir,
        scan_id,
        total: 0,
        errors: 0,
    };
    let _ = app.emit("scan-done", done.clone());
    let _ = app.emit("faces-done", done.clone());
    let _ = app.emit("embed-done", done);
}

/// The model and the files of `dir` the third pass would embed: `None`, so
/// the pass is skipped, once `cancel` is set, while `aiEnabled` is off or the
/// model is not in place, or when no RAW file lacks a valid embedding (a
/// JPEG-only, view-only folder never has one).
fn embed_work(
    app: &tauri::AppHandle,
    index: &Mutex<Index>,
    dir: &str,
    cancel: &AtomicBool,
) -> Option<(PathBuf, Vec<index::EmbedTodo>)> {
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let model = app.state::<crate::ai::AppAi>().model_if_enabled()?;
    match index::lock(index).embeddings_missing(dir) {
        Ok(todo) if !todo.is_empty() => Some((model, todo)),
        Ok(_) => None,
        Err(e) => {
            log::error!("failed to list the embed pass of {dir}: {e}");
            None
        }
    }
}

/// The third pass of a scan: the image embedding of every file `embed_work`
/// listed, with the model at `model` (built once per process, on the first
/// pass that needs it). A model that fails to build embeds nothing.
#[allow(clippy::too_many_arguments)]
fn run_embed_pass<P>(
    index: &Mutex<Index>,
    dir: &str,
    model: &Path,
    todo: &[index::EmbedTodo],
    threads: usize,
    focus: &ScanFocus,
    cancel: &AtomicBool,
    progress: P,
) -> index::ScanSummary
where
    P: Fn(usize, usize) + Send + Sync,
{
    let started = std::time::Instant::now();
    let model = match riffle_core::embed::shared(model) {
        Ok(model) => model,
        Err(e) => {
            log::error!(
                "failed to load the embedding model {}: {e}",
                model.display()
            );
            return index::ScanSummary {
                total: 0,
                errors: 0,
            };
        }
    };
    log::info!(
        "embed model ready: dir={dir} in {}ms",
        started.elapsed().as_millis()
    );
    index::run_embed_scan(
        index,
        dir,
        todo,
        model,
        threads,
        focus,
        cancel,
        index::PROGRESS_INTERVAL,
        progress,
    )
}

/// The second pass of a scan: the AF eye's in-focus probability and the
/// sharpness score of every file of `dir` whose row is not yet at
/// `FACES_VERSION`. Nothing runs, and nothing is reported,
/// once `cancel` is set; the rows keep their old `faces_extractor`, so the
/// next scan of the folder picks them up again.
fn run_faces_pass<P>(
    index: &Mutex<Index>,
    dir: &str,
    threads: usize,
    focus: &ScanFocus,
    cancel: &AtomicBool,
    progress: P,
) -> index::ScanSummary
where
    P: Fn(usize, usize, Vec<index::FaceReady>) + Send + Sync,
{
    let none = index::ScanSummary {
        total: 0,
        errors: 0,
    };
    if cancel.load(Ordering::Relaxed) {
        return none;
    }
    let todo = match index::lock(index).faces_todo(dir) {
        Ok(todo) => todo,
        Err(e) => {
            log::error!("failed to list the faces pass of {dir}: {e}");
            return none;
        }
    };
    index::run_faces_scan(
        index,
        dir,
        &todo,
        threads,
        focus,
        cancel,
        index::PROGRESS_INTERVAL,
        progress,
    )
}

/// Start the scan `scan_folder` prepared for `scan_id`, in the background.
/// Still emits `scan-done` and `faces-done` (with no work done) when there is
/// no pending work under that id (the index cache was unavailable, neither
/// pass had a file to do, or this scan has since been superseded), so the
/// frontend's `scanRunning` flag always clears.
///
/// The task runs two passes: `run_scan` (thumbnails and metadata), ending in
/// `scan-done`, then, unless canceled, `run_faces_scan` (the focus candidate
/// cue's in-focus probability and the sharpness score) over what `faces_todo`
/// lists,
/// reporting `faces-progress` and ending in `faces-done`. Because both passes
/// share the one `running` entry: `ScansState::scanning()` stays true through
/// the second pass, so `clear_index` refuses and the settings modal shows
/// scanning until it ends; `scan_folder` cancels and joins the second pass
/// like the first, and the rows it had not written yet keep their old
/// `faces_extractor`, so the next scan of the folder resumes them; and a
/// resync's `reconcile` runs only after that join, so `write_faces` never
/// races a row deletion.
///
/// After `faces-done`, at its unchanged time, a third pass,
/// `run_embed_scan`, embeds the RAW files `embeddings_missing` lists when
/// `aiEnabled` is on and the model is in place, reporting `embed-progress`
/// and ending in `embed-done`, which is emitted at once when the pass is
/// skipped. It keeps the `running` entry, so `scan_folder` cancels and joins
/// it and `set_scan_focus` reaches it, but `scanning()` is false through it:
/// it writes only `embeddings`, and the operations that guard on a scan
/// would otherwise wait out minutes of embedding after the frontend's
/// `scanRunning` cleared.
///
/// The latest-id check and the store into `running` happen under the same
/// lock as `scan_folder`'s own id-minting and `running`-taking, so a
/// `scan_folder` that supersedes this id can never interleave between the
/// check and the store: either it runs first, in which case this call sees
/// its own id is stale and does not spawn at all, or it runs after, in which
/// case it takes the handle this call just stored and joins it before
/// reconciling.
#[tauri::command]
pub async fn start_scan(app: tauri::AppHandle, scan_id: u64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scans = app.state::<Scans>();
        let mut state = index::lock(&scans.0);
        if state.latest_id != scan_id {
            let dir = state
                .pending
                .remove(&scan_id)
                .map(|pending| pending.dir)
                .unwrap_or_default();
            drop(state);
            emit_empty_scan_events(&app, &dir, scan_id);
            return Ok(());
        }
        let Some(pending) = state.pending.remove(&scan_id) else {
            // An idle folder or a missing index cache queues nothing, so only
            // a scan already running under this id means a second call.
            invariant!(
                state
                    .running
                    .as_ref()
                    .is_none_or(|(id, _, _, _)| *id != scan_id),
                "start_scan once per scan",
                "scan_id={scan_id} latest={}",
                state.latest_id
            );
            drop(state);
            emit_empty_scan_events(&app, "", scan_id);
            return Ok(());
        };
        let Some(index) = app.state::<AppIndex>().0.clone() else {
            drop(state);
            emit_empty_scan_events(&app, &pending.dir, scan_id);
            return Ok(());
        };
        let PendingScan { dir, todo, cancel } = pending;
        let focus = ScanFocus::default();

        let handle = tauri::async_runtime::spawn_blocking({
            let cancel = cancel.clone();
            let focus = focus.clone();
            let app = app.clone();
            move || {
                let summary = index::run_scan(
                    &index,
                    &dir,
                    &todo,
                    scan_threads(),
                    &focus,
                    &cancel,
                    index::PROGRESS_INTERVAL,
                    |done, total, ready| {
                        invariant!(
                            done <= total,
                            "scan progress within total",
                            "dir={dir} scan_id={scan_id} done={done} total={total}"
                        );
                        let _ = app.emit(
                            "scan-progress",
                            Progress {
                                dir: &dir,
                                scan_id,
                                done,
                                total,
                                ready,
                            },
                        );
                    },
                );
                let _ = app.emit(
                    "scan-done",
                    Done {
                        dir: &dir,
                        scan_id,
                        total: summary.total,
                        errors: summary.errors,
                    },
                );
                let summary = run_faces_pass(
                    &index,
                    &dir,
                    scan_threads(),
                    &focus,
                    &cancel,
                    |done, total, ready| {
                        invariant!(
                            done <= total,
                            "scan progress within total",
                            "dir={dir} scan_id={scan_id} done={done} total={total}"
                        );
                        let _ = app.emit(
                            "faces-progress",
                            FacesProgress {
                                dir: &dir,
                                scan_id,
                                done,
                                total,
                                ready,
                            },
                        );
                    },
                );
                let mut embed = embed_work(&app, &index, &dir, &cancel);
                // Drop the entry before `faces-done`, or mark it as in its
                // third pass: the frontend drains a deferred rescan off that
                // event, and its `scan_folder` must not find this scan still
                // in `running` short of that pass, or `joined_previous`
                // makes `changed` nonzero and forces a `folder_entries` read
                // of an unchanged folder (see `refreshOnScanDone` in
                // `refresh.ts`). `scanning()` thus goes false a moment before
                // the frontend's `scanRunning` does, which only lets a
                // `clear_index` already in flight run slightly earlier.
                {
                    let scans = app.state::<Scans>();
                    let mut state = index::lock(&scans.0);
                    // `set_ai_enabled(false)` switches off before it takes
                    // this lock to `stop_embedding`, so checking again under
                    // the lock misses no switch-off.
                    if !app.state::<crate::ai::AppAi>().enabled() {
                        embed = None;
                    }
                    if embed.is_some() {
                        state.start_embedding(scan_id);
                    } else {
                        state.finish(scan_id);
                    }
                }
                let _ = app.emit(
                    "faces-done",
                    Done {
                        dir: &dir,
                        scan_id,
                        total: summary.total,
                        errors: summary.errors,
                    },
                );
                let summary = match embed {
                    Some((model, todo)) => {
                        let summary = run_embed_pass(
                            &index,
                            &dir,
                            &model,
                            &todo,
                            embed_threads(),
                            &focus,
                            &cancel,
                            |done, total| {
                                let _ = app.emit(
                                    "embed-progress",
                                    EmbedProgress {
                                        dir: &dir,
                                        scan_id,
                                        done,
                                        total,
                                    },
                                );
                            },
                        );
                        let scans = app.state::<Scans>();
                        index::lock(&scans.0).finish(scan_id);
                        summary
                    }
                    None => index::ScanSummary {
                        total: 0,
                        errors: 0,
                    },
                };
                let _ = app.emit(
                    "embed-done",
                    Done {
                        dir: &dir,
                        scan_id,
                        total: summary.total,
                        errors: summary.errors,
                    },
                );
            }
        });
        state.running = Some((scan_id, cancel, focus, handle));
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Have the scan `scan_id` take `paths` first, in their order, from now on,
/// replacing the list a previous call gave. A scan that has ended or been
/// superseded ignores it, so a late call is harmless.
///
/// This runs on the main thread, so it never waits for `Scans`: a long holder
/// (`clear_index`, the trash commands, the eviction's `VACUUM`) means no scan
/// is taking the focus, and the call is dropped.
#[tauri::command]
pub fn set_scan_focus(app: tauri::AppHandle, scan_id: u64, paths: Vec<String>) {
    let scans = app.state::<Scans>();
    let state = match scans.0.try_lock() {
        Ok(state) => state,
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
        Err(std::sync::TryLockError::WouldBlock) => return,
    };
    state.set_focus(scan_id, paths);
}

/// Stop the scan `scan_id` before its third pass, for a Move to Trash the
/// frontend holds until the scan has ended. It still ends the normal way:
/// `scan-done` with the partial total, then `faces-done`. A scan that has
/// ended or been superseded ignores it, so a late call is harmless.
#[tauri::command]
pub async fn cancel_scan(app: tauri::AppHandle, scan_id: u64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scans = app.state::<Scans>();
        index::lock(&scans.0).cancel(scan_id);
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(serde::Serialize, Clone)]
struct Progress<'a> {
    dir: &'a str,
    scan_id: u64,
    done: usize,
    total: usize,
    /// The paths the index committed since the previous event, so the
    /// frontend can request exactly those thumbnails instead of every cell
    /// it has nothing for yet.
    ready: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
struct FacesProgress<'a> {
    dir: &'a str,
    scan_id: u64,
    done: usize,
    total: usize,
    /// The files the second pass committed since the previous event, with
    /// their result, so the frontend can patch its entries in place.
    ready: Vec<index::FaceReady>,
}

#[derive(serde::Serialize, Clone)]
struct EmbedProgress<'a> {
    dir: &'a str,
    scan_id: u64,
    done: usize,
    total: usize,
}

#[derive(serde::Serialize, Clone)]
struct Done<'a> {
    dir: &'a str,
    scan_id: u64,
    total: usize,
    errors: usize,
}

/// The indexed rows of `dir`, in the same order as `list_arw`. Empty when the
/// index cache is unavailable.
#[tauri::command]
pub async fn folder_entries(
    app: tauri::AppHandle,
    dir: String,
) -> Result<Vec<IndexedFile>, String> {
    let dir = canonicalize(&dir);
    let Some(index) = app.state::<AppIndexReader>().0.clone() else {
        return Ok(Vec::new());
    };
    let started = std::time::Instant::now();
    let rows = {
        let dir = dir.clone();
        tauri::async_runtime::spawn_blocking(move || index::lock(&index).entries(&dir))
            .await
            .map_err(|e| e.to_string())??
    };
    log::info!(
        "open entries: dir={dir} rows={} in {}ms",
        rows.len(),
        started.elapsed().as_millis()
    );
    Ok(rows)
}

/// The genre labels of `dir`'s files with a valid embedding (`genre`), keyed
/// by path; a file without one is absent. Empty when the index cache is
/// unavailable.
#[tauri::command]
pub async fn genres_of(
    app: tauri::AppHandle,
    dir: String,
) -> Result<HashMap<String, Vec<&'static str>>, String> {
    let dir = canonicalize(&dir);
    let Some(index) = app.state::<AppIndexReader>().0.clone() else {
        return Ok(HashMap::new());
    };
    tauri::async_runtime::spawn_blocking(move || {
        let rows = index::lock(&index).embeddings(&dir)?;
        Ok(rows
            .into_iter()
            .map(|(path, vec)| (path, riffle_core::genre::genres_of(&vec)))
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The file that was current in the strip when `dir` was last viewed, as
/// stored; the frontend decides whether it is still listed. `None` when the
/// index cache is unavailable or the read fails.
#[tauri::command]
pub async fn last_viewed(app: tauri::AppHandle, dir: String) -> Option<String> {
    let dir = canonicalize(&dir);
    let index = app.state::<AppIndexReader>().0.clone()?;
    let read = tauri::async_runtime::spawn_blocking(move || index::lock(&index).last_viewed(&dir))
        .await
        .map_err(|e| e.to_string())
        .and_then(|read| read);
    read.unwrap_or_else(|e| {
        log::warn!("failed to read the last viewed file: {e}");
        None
    })
}

/// Record `path` as the file current in the strip for `dir`. Failing to
/// write it only means resuming at the first file, so it is logged, not
/// returned.
#[tauri::command]
pub async fn set_last_viewed(app: tauri::AppHandle, dir: String, path: String) {
    let dir = canonicalize(&dir);
    let Some(index) = app.state::<AppIndex>().0.clone() else {
        return;
    };
    let written = tauri::async_runtime::spawn_blocking(move || {
        index::lock(&index).set_last_viewed(&dir, &path)
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|written| written);
    if let Err(e) = written {
        log::warn!("failed to remember the last viewed file: {e}");
    }
}

/// The files of `dir` shown in the preview at least once. Empty when the
/// index cache is unavailable or the read fails.
#[tauri::command]
pub async fn viewed_files(app: tauri::AppHandle, dir: String) -> Vec<String> {
    let dir = canonicalize(&dir);
    let Some(index) = app.state::<AppIndexReader>().0.clone() else {
        return Vec::new();
    };
    let read = tauri::async_runtime::spawn_blocking(move || index::lock(&index).viewed_files(&dir))
        .await
        .map_err(|e| e.to_string())
        .and_then(|read| read);
    read.unwrap_or_else(|e| {
        log::warn!("failed to read the viewed files: {e}");
        Vec::new()
    })
}

/// Record `paths` of `dir` as shown in the preview. Failing to write them
/// only loses view history, so it is logged, not returned.
#[tauri::command]
pub async fn mark_viewed(app: tauri::AppHandle, dir: String, paths: Vec<String>) {
    let dir = canonicalize(&dir);
    let Some(index) = app.state::<AppIndex>().0.clone() else {
        return;
    };
    let written =
        tauri::async_runtime::spawn_blocking(move || index::lock(&index).mark_viewed(&dir, &paths))
            .await
            .map_err(|e| e.to_string())
            .and_then(|written| written);
    if let Err(e) = written {
        log::warn!("failed to record the viewed files: {e}");
    }
}

/// The cached thumbnail of one file, in the same envelope as `preview`.
#[tauri::command]
pub async fn thumbnail(app: tauri::AppHandle, path: String) -> Result<Response, String> {
    let Some(index) = app.state::<AppIndexReader>().0.clone() else {
        return Err("no index cache available".to_string());
    };
    let (orientation, jpeg) =
        tauri::async_runtime::spawn_blocking(move || index::lock(&index).thumbnail(&path))
            .await
            .map_err(|e| e.to_string())??;
    Ok(Response::new(payload(
        THUMBNAIL_KIND_JPEG_V1,
        orientation,
        &jpeg,
    )))
}

/// The sidecar writer, shared between `set_rating` and the quit-time drain.
/// `None` when there is no index to record judgments in, in which case
/// `set_rating` is an error rather than a silent no-op.
pub struct AppWriter(pub Option<Writer>);

/// The sidecar format selected in the settings, read once at launch.
pub struct AppSidecarFormat(pub Mutex<SidecarFormat>);

/// The `xmp:Label` names selected in the settings.
pub struct AppLabelNames(pub Mutex<LabelNames>);

/// The culling keymap resolved from the defaults and the `shortcuts` setting.
pub struct AppKeymap(pub Mutex<Keymap>);

/// Whether the selection moves to the next file after a judgment.
pub struct AppAutoAdvance(pub AtomicBool);

/// The panels last passed to `set_panels`, seeded from the store at launch,
/// which the `View` checks follow.
pub struct AppPanels(pub Mutex<Value>);

/// Serializes `switch_sidecar_format` calls, so two quick clicks cannot run
/// concurrent switches whose drain, save, state write and reset would
/// otherwise interleave. `set_label_names` takes it for its drain and reset.
pub struct AppSwitchLock(pub Mutex<()>);

/// The sidecar format currently selected, as stored under `sidecarFormat`
/// (`"xmp"` or `"dop"`).
#[tauri::command]
pub fn sidecar_format(app: tauri::AppHandle) -> &'static str {
    index::lock(&app.state::<AppSidecarFormat>().0).setting()
}

/// Whether a sidecar format has been saved under `sidecarFormat`. The main
/// window asks for one on launch while this is false. A store that cannot be
/// opened counts as saved: an answer could not be persisted there anyway.
#[tauri::command]
pub fn sidecar_format_saved(app: tauri::AppHandle) -> bool {
    format_saved(settings(&app).map(|store| store.has("sidecarFormat")))
}

fn format_saved(has: Result<bool, String>) -> bool {
    has.unwrap_or_else(|e| {
        log::warn!("failed to open the settings: {e}");
        true
    })
}

/// Whether auto-advance is on.
#[tauri::command]
pub fn auto_advance(app: tauri::AppHandle) -> bool {
    app.state::<AppAutoAdvance>().0.load(Ordering::Relaxed)
}

/// Turn auto-advance on or off and persist it under `autoAdvance`. A save
/// failure is logged and the in-memory change stands.
#[tauri::command]
pub fn set_auto_advance(app: tauri::AppHandle, enabled: bool) {
    app.state::<AppAutoAdvance>()
        .0
        .store(enabled, Ordering::Relaxed);
    let saved = settings(&app).and_then(|store| {
        store.set("autoAdvance", enabled);
        store.save().map_err(|e| e.to_string())
    });
    if let Err(e) = saved {
        log::warn!("failed to save the auto-advance setting: {e}");
    }
}

/// Which of the left pane, the filmstrip and the right pane are shown, as
/// stored under `panels`.
#[tauri::command]
pub fn panels(app: tauri::AppHandle) -> Value {
    stored_panels(&app)
}

/// The panels stored under `panels`, all shown when the store cannot be read.
pub fn stored_panels(app: &tauri::AppHandle) -> Value {
    let store = settings(app).ok();
    panels_setting(store.as_ref().and_then(|s| s.get("panels")).as_ref())
}

/// Remember which panes are shown and check the `View` items to match.
/// Failing to write it only means the next launch restores the previously
/// saved panes, so it is logged, not returned.
#[tauri::command]
pub fn set_panels(app: tauri::AppHandle, panels: Value) {
    let panels = panels_setting(Some(&panels));
    *index::lock(&app.state::<AppPanels>().0) = panels.clone();
    crate::app_menu::apply_panels(&app, &panels);
    let saved = settings(&app).and_then(|store| {
        store.set("panels", panels);
        store.save().map_err(|e| e.to_string())
    });
    if let Err(e) = saved {
        log::warn!("failed to remember the panels: {e}");
    }
}

/// The stored `panels` value as `{"left", "strip", "right"}` booleans; a
/// missing or non-boolean entry means shown.
fn panels_setting(value: Option<&Value>) -> Value {
    let shown = |name: &str| {
        value
            .and_then(|v| v.get(name))
            .and_then(Value::as_bool)
            .unwrap_or(true)
    };
    serde_json::json!({
        "left": shown("left"),
        "strip": shown("strip"),
        "right": shown("right"),
    })
}

/// The strip filter remembered by `set_filter`, empty (letting every file
/// through) when the store cannot be read or holds nothing usable.
#[tauri::command]
pub fn filter(app: tauri::AppHandle) -> Value {
    let store = settings(&app).ok();
    filter_setting(store.as_ref().and_then(|s| s.get("filter")).as_ref())
}

/// Remember the strip filter. Failing to write it only means the next launch
/// restores the previously saved filter, so it is logged, not returned.
#[tauri::command]
pub fn set_filter(app: tauri::AppHandle, filter: Value) {
    let filter = filter_setting(Some(&filter));
    let saved = settings(&app).and_then(|store| {
        store.set("filter", filter);
        store.save().map_err(|e| e.to_string())
    });
    if let Err(e) = saved {
        log::warn!("failed to remember the filter: {e}");
    }
}

// The filter menu's states. The source of truth is the `data-*` items of
// `ui/index.html` and the types in `ui/src/filter.ts`, `ui/src/focus.ts`
// (`MarkState`) and `ui/src/eyes.ts` (`EyeState`); rename a state in both.
const FILTER_FLAGS: &[&str] = &["picked", "untagged", "rejected"];
const FILTER_LABELS: &[&str] = &[
    "red", "orange", "yellow", "green", "blue", "pink", "purple", "none",
];
const FILTER_ORIENTATIONS: &[&str] = &["portrait", "landscape"];
const FILTER_CANDIDATES: &[&str] = &["good", "not_candidate", "unknown"];
const FILTER_EYES: &[&str] = &["open", "closed", "unknown"];

/// The stored `filter` value with every section present, each keeping only
/// its known members, deduplicated in stored order. A non-object value or a
/// non-array section is empty, and keys other than the six sections are
/// dropped.
fn filter_setting(value: Option<&Value>) -> Value {
    let members = |name: &str| -> Vec<Value> {
        let mut kept: Vec<Value> = Vec::new();
        let items = value
            .and_then(|v| v.get(name))
            .and_then(Value::as_array)
            .into_iter()
            .flatten();
        for item in items {
            let allowed = match name {
                "stars" => item.as_u64().is_some_and(|n| n <= 5),
                "flags" => item.as_str().is_some_and(|s| FILTER_FLAGS.contains(&s)),
                "labels" => item.as_str().is_some_and(|s| FILTER_LABELS.contains(&s)),
                "orientations" => item
                    .as_str()
                    .is_some_and(|s| FILTER_ORIENTATIONS.contains(&s)),
                "candidates" => item
                    .as_str()
                    .is_some_and(|s| FILTER_CANDIDATES.contains(&s)),
                _ => item.as_str().is_some_and(|s| FILTER_EYES.contains(&s)),
            };
            if allowed && !kept.contains(item) {
                kept.push(item.clone());
            }
        }
        kept
    };
    serde_json::json!({
        "flags": members("flags"),
        "stars": members("stars"),
        "labels": members("labels"),
        "orientations": members("orientations"),
        "candidates": members("candidates"),
        "eyes": members("eyes"),
    })
}

/// Whether the MCP server is on, its port and its last bind error.
#[tauri::command]
pub async fn mcp_enabled(app: tauri::AppHandle) -> crate::mcp::McpState {
    crate::mcp::state(&app).await
}

/// The main window's answer to an `mcp-request`.
#[tauri::command]
pub fn mcp_reply(app: tauri::AppHandle, id: u64, ok: bool, value: Value) {
    app.state::<crate::mcp::AppMcp>().reply(id, ok, value);
}

/// Turn the MCP server on or off and persist it under `mcpEnabled`. A save
/// failure is logged and the switch stands; a bind error is reported through
/// the returned state and the `mcp-state` event.
#[tauri::command]
pub async fn set_mcp_enabled(app: tauri::AppHandle, enabled: bool) -> crate::mcp::McpState {
    let saving = app.clone();
    let saved = tauri::async_runtime::spawn_blocking(move || {
        settings(&saving).and_then(|store| {
            store.set("mcpEnabled", enabled);
            store.save().map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|saved| saved);
    if let Err(e) = saved {
        log::warn!("failed to save the MCP setting: {e}");
    }
    crate::mcp::switch(&app, enabled).await
}

/// Whether AI analysis is on, whether its model is in place, and how far a
/// running download of it got.
#[tauri::command]
pub fn ai_enabled(app: tauri::AppHandle) -> crate::ai::AiState {
    crate::ai::state(&app)
}

/// Turn AI analysis on or off and persist it under `aiEnabled`. On downloads
/// the model when it is not in place (see `ai::switch`); the frontend rescans
/// the open folder once the state says it is ready, which runs the third
/// pass. Off cancels a running third pass, keeping the stored embeddings. A
/// save failure is logged and the switch stands.
#[tauri::command]
pub async fn set_ai_enabled(app: tauri::AppHandle, enabled: bool) -> crate::ai::AiState {
    let saving = app.clone();
    let saved = tauri::async_runtime::spawn_blocking(move || {
        settings(&saving).and_then(|store| {
            store.set("aiEnabled", enabled);
            store.save().map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|saved| saved);
    if let Err(e) = saved {
        log::warn!("failed to save the AI setting: {e}");
    }
    let state = crate::ai::switch(&app, enabled);
    if !enabled {
        // The `Scans` lock can be held for seconds (a clear, a trash run).
        let _ = tauri::async_runtime::spawn_blocking(move || {
            index::lock(&app.state::<Scans>().0).stop_embedding();
        })
        .await;
    }
    state
}

/// The `xmp:Label` names written for Red ... Purple, in the shape stored
/// under `labelNames`, and the Lightroom presets the settings modal offers
/// (see `label_names_payload`).
#[tauri::command]
pub fn label_names(app: tauri::AppHandle) -> Value {
    let names = index::lock(&app.state::<AppLabelNames>().0).clone();
    label_names_payload(&names)
}

/// `{"names": {...}, "presets": [{"code", "name", "names": {...}}, ...]}`,
/// every `names` in the shape stored under `labelNames` and the presets in
/// `riffle_core::i18n::presets()` order (English first).
fn label_names_payload(names: &LabelNames) -> Value {
    let presets: Vec<Value> = riffle_core::i18n::presets()
        .iter()
        .map(|preset| {
            serde_json::json!({
                "code": preset.code,
                "name": preset.name,
                "names": label_names_value(&preset.names),
            })
        })
        .collect();
    serde_json::json!({
        "names": label_names_value(names),
        "presets": presets,
    })
}

/// Normalize and apply `names` (see `label_names_setting`), persist them
/// under `labelNames`, reset the index's sidecar state so the open folder
/// re-reads its XMP labels under the new names (after draining the writer,
/// whose queued judgments keep the names they were made under), and emit
/// `sidecar-format` so the main window reopens the folder. Returns the stored
/// names. A save failure is logged and the in-memory change stands.
///
/// `async` because the reset touches SQLite, which must not block the main
/// thread.
#[tauri::command]
pub async fn set_label_names(app: tauri::AppHandle, names: Value) -> Result<Value, String> {
    let names = label_names_setting(Some(&names));
    let stored = label_names_value(&names);
    let changed = {
        let state = app.state::<AppLabelNames>();
        let mut current = index::lock(&state.0);
        let changed = *current != names;
        *current = names;
        changed
    };
    let saved = settings(&app).and_then(|store| {
        store.set("labelNames", stored.clone());
        store.save().map_err(|e| e.to_string())
    });
    if let Err(e) = saved {
        log::warn!("failed to save the label names: {e}");
    }
    if !changed {
        return Ok(stored);
    }
    let reset = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let switch_lock = reset.state::<AppSwitchLock>();
        let _guard = index::lock(&switch_lock.0);
        if let Some(writer) = &reset.state::<AppWriter>().0 {
            writer.flush(crate::sidecar::DRAIN_TIMEOUT);
        }
        match reset.state::<AppIndex>().0.as_ref() {
            Some(index) => index::lock(index).reset_sidecars(),
            None => Ok(()),
        }
    })
    .await
    .map_err(|e| e.to_string())??;
    let format = *index::lock(&app.state::<AppSidecarFormat>().0);
    let _ = app.emit("sidecar-format", format.setting());
    Ok(stored)
}

/// The resolved keymap, one binding per action in the order the shortcuts
/// panel shows them.
#[tauri::command]
pub fn shortcuts(app: tauri::AppHandle) -> Vec<Binding> {
    index::lock(&app.state::<AppKeymap>().0).bindings()
}

/// Add `key` to `action`'s keys and persist the overrides.
#[tauri::command]
pub fn add_shortcut_key(
    app: tauri::AppHandle,
    action: String,
    key: String,
) -> Result<Vec<Binding>, String> {
    update_keymap(&app, |keymap| keymap.add(&action, &key))
}

/// Remove `key` from `action`'s keys and persist the overrides.
#[tauri::command]
pub fn remove_shortcut_key(
    app: tauri::AppHandle,
    action: String,
    key: String,
) -> Result<Vec<Binding>, String> {
    update_keymap(&app, |keymap| keymap.remove(&action, &key))
}

/// Restore `action`'s default keys and persist the overrides.
#[tauri::command]
pub fn reset_shortcut(app: tauri::AppHandle, action: String) -> Result<Vec<Binding>, String> {
    update_keymap(&app, |keymap| keymap.reset(&action))
}

/// Restore every default key and drop the `shortcuts` setting.
#[tauri::command]
pub fn reset_shortcuts(app: tauri::AppHandle) -> Vec<Binding> {
    update_keymap(&app, |keymap| {
        keymap.reset_all();
        Ok(())
    })
    .unwrap_or_default()
}

/// The menu accelerators the keymap gives the menu-backed actions.
fn accelerators(keymap: &Keymap) -> Vec<Option<String>> {
    crate::app_menu::keyed_actions()
        .map(|action| keymap.accelerator_for(action))
        .collect()
}

/// Apply `change` to the keymap and save its overrides under `shortcuts`,
/// removing the key when there are none. A save failure is logged and the
/// in-memory change stands, as in `remember_folder`.
fn update_keymap(
    app: &tauri::AppHandle,
    change: impl FnOnce(&mut Keymap) -> Result<(), String>,
) -> Result<Vec<Binding>, String> {
    let state = app.state::<AppKeymap>();
    let mut keymap = index::lock(&state.0);
    let before = accelerators(&keymap);
    change(&mut keymap)?;
    if accelerators(&keymap) != before {
        if let Err(e) = crate::app_menu::refresh(app, &keymap) {
            log::warn!("failed to refresh the app menu: {e}");
        }
    }
    let overrides = keymap.overrides();
    let saved = settings(app).and_then(|store| {
        if overrides.as_object().is_some_and(|o| o.is_empty()) {
            store.delete("shortcuts");
        } else {
            store.set("shortcuts", overrides);
        }
        store.save().map_err(|e| e.to_string())
    });
    if let Err(e) = saved {
        log::warn!("failed to save the shortcuts: {e}");
    }
    Ok(keymap.bindings())
}

/// Record a judgment for one file: `0` unrated and `1`-`5` stars, plus the
/// pick / reject flag (`"none"`, `"pick"` or `"reject"`) and the color label
/// beside them, all kept under both formats. The label is any raw name; an
/// empty one is no label.
///
/// `label_known` is false when the frontend has not yet learned this path's
/// label from `folder_entries` (e.g. a judgment made before the first
/// refresh lands, or before the sidecar has even been parsed): `label` is
/// then ignored, the `ratings.label` column is left untouched rather than
/// being cleared, and the writer keeps whatever label the sidecar itself
/// currently holds instead of stripping it (see `Index::set_rating` and
/// `sidecar::write`).
///
/// The `ratings` row is written before the writer is told, so a crash between
/// the two still leaves the row dirty and the sidecar is written on the next
/// open of that folder. The command is `async` because it touches SQLite; the
/// frontend redraws without awaiting it.
#[tauri::command]
pub async fn set_rating(
    app: tauri::AppHandle,
    path: String,
    rating: u8,
    flag: String,
    label: Option<String>,
    label_known: bool,
) -> Result<(), String> {
    raw_only(Path::new(&path), "culling")?;
    if rating > 5 {
        return Err(format!("rating {rating} is outside 0..=5"));
    }
    let flag = index::parse_flag(&flag)?;
    // 0 and "unrated" are the same state; the row keeps NULL and the sidecar
    // gets a `0` only when one already exists.
    let rating = Some(rating as i8).filter(|r| *r != 0);
    let format = *index::lock(&app.state::<AppSidecarFormat>().0);
    let names = index::lock(&app.state::<AppLabelNames>().0).clone();
    let Some(index) = app.state::<AppIndex>().0.clone() else {
        return Err("no index cache available".to_string());
    };
    let label = label.filter(|l| !l.is_empty());
    let dir = Path::new(&path)
        .parent()
        .map_or_else(String::new, |d| d.to_string_lossy().into_owned());
    {
        let (path, label) = (path.clone(), label.clone());
        tauri::async_runtime::spawn_blocking(move || {
            index::lock(&index).set_rating(&dir, &path, rating, flag, label.as_deref(), label_known)
        })
        .await
        .map_err(|e| e.to_string())??;
    }
    match &app.state::<AppWriter>().0 {
        Some(writer) => writer.set(
            PathBuf::from(path),
            rating,
            flag,
            label,
            label_known,
            format,
            names,
        ),
        None => Err("the sidecar writer is not running".to_string()),
    }
}

/// The base the size figure is divided by. The label mirrors what the user's
/// file manager shows next to the same file: Finder counts 1 GB as 1000^3,
/// Explorer (and the Linux file managers) as 1024^3, so the base follows the
/// platform rather than the labels changing to `GiB`.
pub(crate) const SIZE_BASE: u64 = if cfg!(target_os = "macos") {
    1000
} else {
    1024
};

/// The largest preview, in pixels, the decode worker hands over unresized.
/// WebKitGTK draws an `ImageBitmap` created in a Worker and transferred to the
/// main thread as fully transparent once it has too many pixels, whatever its
/// shape: about 6.87 MP (6,840,000 px drew, 6,900,000 px did not; WebKitGTK
/// 2.50.4 under WSLg). Above the limit the worker decodes with resize options;
/// 6 MP keeps a margin since the threshold may vary by environment. Other
/// platforms are unaffected.
pub const PREVIEW_PIXEL_LIMIT: Option<u32> = if cfg!(target_os = "linux") {
    Some(6_000_000)
} else {
    None
};

/// The preview pixel limit, or `None` where previews go unresized.
#[tauri::command]
pub fn preview_pixel_limit() -> Option<u32> {
    PREVIEW_PIXEL_LIMIT
}

/// The message shown when the button is pressed while a scan is running. The
/// clear is refused rather than canceling the scan.
pub(crate) const SCAN_RUNNING: &str = "a scan is running; wait for it to finish";

/// Format `bytes` for display with plain `B`/`KB`/`MB`/`GB` labels: whole
/// bytes below `base`, one decimal above it. Steps up a unit once the
/// *rounded* value would reach `base` (e.g. 999_999_999 rounds to 1000.0 MB
/// at base 1000, so it must show as 1.0 GB, not 1000.0 MB), not just the raw
/// one, or a value one rounding step below a power of the base prints a
/// figure equal to the base.
pub(crate) fn format_bytes(bytes: u64, base: u64) -> String {
    if bytes < base {
        return format!("{bytes} B");
    }
    let base = base as f64;
    let round1 = |v: f64| (v * 10.0).round() / 10.0;
    let kb = bytes as f64 / base;
    if round1(kb) < base {
        return format!("{:.1} KB", kb);
    }
    let mb = kb / base;
    if round1(mb) < base {
        return format!("{:.1} MB", mb);
    }
    format!("{:.1} GB", mb / base)
}

/// The index's footprint on disk: the database file plus the `-wal` and
/// `-shm` files SQLite keeps beside it. A missing file counts as zero.
fn on_disk_bytes(path: &Path) -> u64 {
    [
        path.to_path_buf(),
        index::with_suffix(path, "-wal"),
        index::with_suffix(path, "-shm"),
    ]
    .iter()
    .map(|p| std::fs::metadata(p).map_or(0, |m| m.len()))
    .sum()
}

/// The path the index is opened from, recomputed as `main.rs` does; the path
/// itself is not kept in any managed state.
fn index_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_cache_dir()
        .ok()
        .map(|dir| dir.join("index.sqlite"))
}

/// The index cache's size on disk, already formatted for display. No index
/// cache (or no file yet) is `"0 B"`, not an error: there is nothing to
/// report and nothing to clear.
#[tauri::command]
pub async fn index_size(app: tauri::AppHandle) -> Result<String, String> {
    let path = index_path(&app).filter(|_| app.state::<AppIndex>().0.is_some());
    let Some(path) = path else {
        return Ok(format_bytes(0, SIZE_BASE));
    };
    let bytes = tauri::async_runtime::spawn_blocking(move || on_disk_bytes(&path))
        .await
        .map_err(|e| e.to_string())?;
    Ok(format_bytes(bytes, SIZE_BASE))
}

/// Empty the index cache after a native confirmation; `false` when the user
/// canceled. A running scan is refused before the dialog is shown, and again
/// under the `Scans` lock in case one started while the dialog was up.
///
/// Lock order is `Scans` then the writer, as in `spawn_eviction`, and the
/// clear runs in `spawn_blocking`: the drain, the `VACUUM` and the WAL
/// checkpoint all block.
#[tauri::command]
pub async fn clear_index(app: tauri::AppHandle) -> Result<bool, String> {
    {
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
    }
    let (tx, mut rx) = tauri::async_runtime::channel(1);
    let mut dialog = app
        .dialog()
        .message(
            "Every cached thumbnail and the cached metadata of every folder are removed. \
             Judgments are kept. The next open of a folder scans it again.",
        )
        .title("Clear the index cache?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Clear".to_string(),
            "Cancel".to_string(),
        ));
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.parent(&window);
    }
    dialog.show(move |confirmed| {
        let _ = tx.try_send(confirmed);
    });
    if !rx.recv().await.unwrap_or(false) {
        return Ok(false);
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        if let Some(writer) = &handle.state::<AppWriter>().0 {
            writer.flush(crate::sidecar::DRAIN_TIMEOUT);
        }
        let Some(index) = handle.state::<AppIndex>().0.clone() else {
            return Ok(());
        };
        let scans = handle.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
        state.stop_embedding();
        let summary = index::lock(&index).clear()?;
        log::info!("index cleared: {summary:?}");
        drop(state);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit("index-cleared", ());
    Ok(true)
}

/// How far a move to the Trash got, in groups (a RAW with its sidecars). No
/// run id: the `Scans` lock runs one such operation at a time.
#[derive(serde::Serialize, Clone)]
struct TrashProgress<'a> {
    op: &'a str,
    dir: &'a str,
    done: usize,
    total: usize,
}

fn emit_trash_progress(app: &tauri::AppHandle, op: &str, dir: &str, done: usize, total: usize) {
    let _ = app.emit(
        "trash-progress",
        TrashProgress {
            op,
            dir,
            done,
            total,
        },
    );
}

/// Move `paths`, files of the folder `dir` as the listing gave them, to the
/// OS trash in their order, each RAW together with the `.xmp` and `.dop`
/// sidecars that exist for it and a JPEG alone (see `trash::groups_of`). A
/// path outside `dir` fails without being moved, one gone meanwhile fails
/// too. Each group moved, or each chunk on Windows (see `move_groups`), emits
/// `trash-progress`.
///
/// A running scan is refused under the `Scans` lock, which is held across
/// the moves, and the sidecar writer is drained first, as in `clear_index`:
/// a judgment still inside its debounce window would otherwise mint a
/// sidecar for a file that is already gone.
#[tauri::command]
pub async fn trash_files(
    app: tauri::AppHandle,
    dir: String,
    paths: Vec<String>,
) -> Result<trash::Summary, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<trash::Summary, String> {
        if let Some(writer) = &app.state::<AppWriter>().0 {
            writer.flush(crate::sidecar::DRAIN_TIMEOUT);
        }
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
        let listing = read_listing(Path::new(&canonicalize(&dir)), Some(SidecarFormat::Both))?;
        let (groups, foreign) = trash::groups_of(
            Path::new(&canonicalize(&dir)),
            &paths,
            &stat_sidecars(&listing.sidecars),
        );
        let (mut summary, moved) = move_groups(groups, |done, total| {
            emit_trash_progress(&app, "trash", &dir, done, total);
        });
        summary.failed.extend(foreign);
        summary.run_id = app.state::<trash::Runs>().record(moved);
        log::info!("moved files to the trash: {summary:?}");
        drop(state);
        Ok(summary)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Move every file the run `run_id` of `trash_files` (or of `Delete
/// Sidecars…`) moved back to where it came from, RAW before its sidecars,
/// never overwriting (see `trash::restore`). An unknown run is an `Err`; a
/// run is forgotten once undone, whatever came back, and what came back is
/// kept for `trash_rejected_redo` under the same id.
///
/// A running scan is refused under the `Scans` lock, which is held across
/// the moves. The sidecar writer is not drained: nothing pending can target
/// a file that is in the Trash. No index row is written either; the
/// frontend re-lists the folder and the scan picks the files up again.
///
/// No `trash-progress` yet: `trash::restore` has no progress callback, which
/// a follow-up can add the way `trash::run` has one.
#[tauri::command]
pub async fn trash_rejected_undo(
    app: tauri::AppHandle,
    run_id: u64,
) -> Result<trash::Restored, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<trash::Restored, String> {
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
        let run = app
            .state::<trash::Runs>()
            .take(run_id)
            .ok_or_else(|| trash::RUN_GONE.to_string())?;
        let mut restored = restore_run(&run);
        log::info!("restored rejected files from the trash: {restored:?}");
        app.state::<trash::Runs>()
            .undone(run.id, std::mem::take(&mut restored.back));
        drop(state);
        Ok(restored)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Move the files the undo of run `run_id` brought back to the Trash again,
/// exactly those, RAW before its sidecars, with no dialog and no new
/// collection (see `trash::redo`); a file gone meanwhile is a failure. The
/// move is recorded as a new run, whose id the `Summary` carries. An
/// unknown run is an `Err`; it is forgotten once redone. Each group moved, or
/// each chunk on Windows (see `move_again`), emits `trash-progress`, with an
/// empty `dir`: a run can span folders.
///
/// The guards are `trash_files`'s: the sidecar writer is drained
/// first, and a running scan is refused under the `Scans` lock, which is
/// held across the moves.
#[tauri::command]
pub async fn trash_rejected_redo(
    app: tauri::AppHandle,
    run_id: u64,
) -> Result<trash::Summary, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<trash::Summary, String> {
        if let Some(writer) = &app.state::<AppWriter>().0 {
            writer.flush(crate::sidecar::DRAIN_TIMEOUT);
        }
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
        let runs = app.state::<trash::Runs>();
        let paths = runs
            .take_undone(run_id)
            .ok_or_else(|| trash::UNDONE_GONE.to_string())?;
        let (mut summary, moved) = move_again(paths, |done, total| {
            emit_trash_progress(&app, "trash", "", done, total);
        });
        summary.run_id = runs.record(moved);
        log::info!("moved restored files to the trash again: {summary:?}");
        drop(state);
        Ok(summary)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Restore `run` from the Trash listed once, each file matched by its
/// original path (the newest item of a path) and restored on its own, so one
/// missing or colliding item cannot fail the others. The Trash puts a file
/// back at its original path, and the Windows Recycle Bin fails when that
/// folder is gone, so for a file whose folder was renamed since the missing
/// folders are created first (`trash::create_missing`), the file is put back
/// and then moved into the renamed folder (`trash::relocate`), which removes
/// exactly the folders created.
#[cfg(not(target_os = "macos"))]
fn restore_run(run: &trash::TrashRun) -> trash::Restored {
    let items = match ::trash::os_limited::list() {
        Ok(items) => items,
        Err(e) => return trash::Restored::none(run, &format!("could not list the Trash: {e}")),
    };
    let mut items = trash::newest_by_path(items);
    trash::restore(
        run,
        |trashed| items.remove(&trash::trash_key(&trashed.path)),
        Path::exists,
        |trashed, item| {
            let created = match (&trashed.restore_to, trashed.path.parent()) {
                (Some(_), Some(parent)) => trash::create_missing(parent)?,
                _ => Vec::new(),
            };
            if let Err(e) = ::trash::os_limited::restore_all([item]) {
                trash::remove_created(&created);
                return Err(match e {
                    ::trash::Error::RestoreCollision { .. } => trash::ALREADY_EXISTS.to_string(),
                    e => e.to_string(),
                });
            }
            match &trashed.restore_to {
                Some(to) => trash::relocate(&trashed.path, to, &created),
                None => Ok(()),
            }
        },
    )
}

/// Restore `run` from the locations its files got in the Trash, recorded by
/// `trash_one`.
#[cfg(target_os = "macos")]
fn restore_run(run: &trash::TrashRun) -> trash::Restored {
    trash::restore_recorded(run)
}

/// The mover of `trash_files`. On macOS it goes through
/// `NSFileManager` itself to keep where the file went; the crate's default
/// there drives the Finder through AppleScript, which needs Automation
/// permission.
#[cfg(target_os = "macos")]
pub(crate) fn trash_one(path: &Path) -> Result<Option<PathBuf>, String> {
    trash::trash_file(path).map(Some)
}

/// The mover of `trash_files`: the Trash is listed at undo time, so
/// where the file went is not kept.
#[cfg(not(target_os = "macos"))]
pub(crate) fn trash_one(path: &Path) -> Result<Option<PathBuf>, String> {
    ::trash::delete(path)
        .map(|()| None)
        .map_err(|e| e.to_string())
}

/// The chunk mover of `trash_files` and `trash_rejected_redo`: one shell
/// operation for all of `paths`.
#[cfg(windows)]
fn trash_many(paths: &[PathBuf]) -> Result<(), String> {
    ::trash::delete_all(paths).map_err(|e| e.to_string())
}

/// Move `groups` to the Trash in chunks: a shell operation costs far more
/// than the files it moves.
#[cfg(windows)]
fn move_groups(
    groups: Vec<trash::Group>,
    progress: impl FnMut(usize, usize),
) -> (trash::Summary, Vec<trash::Trashed>) {
    trash::run_chunked(groups, Path::exists, trash_many, trash_one, progress)
}

/// Move `groups` to the Trash one file at a time: macOS needs where each file
/// went, and `::trash::delete_all` is a loop of single moves on Linux.
#[cfg(not(windows))]
fn move_groups(
    groups: Vec<trash::Group>,
    progress: impl FnMut(usize, usize),
) -> (trash::Summary, Vec<trash::Trashed>) {
    trash::run(groups, trash_one, progress)
}

/// `trash::redo` of `paths`, chunked like `move_groups`.
#[cfg(windows)]
fn move_again(
    paths: Vec<PathBuf>,
    progress: impl FnMut(usize, usize),
) -> (trash::Summary, Vec<trash::Trashed>) {
    trash::redo_chunked(paths, Path::exists, trash_many, trash_one, progress)
}

/// `trash::redo` of `paths`, one file at a time like `move_groups`.
#[cfg(not(windows))]
fn move_again(
    paths: Vec<PathBuf>,
    progress: impl FnMut(usize, usize),
) -> (trash::Summary, Vec<trash::Trashed>) {
    trash::redo(paths, Path::exists, trash_one, progress)
}

/// Resolve symlinks and normalize a folder path so the same folder reached
/// through different spellings (a trailing separator, a symlinked parent,
/// `/tmp` vs `/private/tmp` on macOS) shares one row set in the index. Falls
/// back to the original string when canonicalization fails (e.g. the folder
/// was removed between picking and scanning).
pub(crate) fn canonicalize(dir: &str) -> String {
    std::fs::canonicalize(dir)
        .map_or_else(|_| dir.to_string(), |p| p.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, Scans, ScansState, PREVIEW_PIXEL_LIMIT, SIZE_BASE};
    use crate::index;
    use riffle_core::scan::ScanFocus;
    use riffle_core::Flag;
    use std::sync::{atomic::AtomicBool, mpsc, Arc, Mutex};

    /// Spawn a task that clears its own entry once `go` fires, the way the
    /// real scan task does at the end of `start_scan`'s closure.
    fn spawn_scan(
        scans: &Arc<Scans>,
        scan_id: u64,
    ) -> (mpsc::Sender<()>, tauri::async_runtime::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel();
        let scans = scans.clone();
        let handle = tauri::async_runtime::spawn_blocking(move || {
            let _ = rx.recv();
            index::lock(&scans.0).finish(scan_id);
        });
        (tx, handle)
    }

    #[test]
    fn a_finished_scan_leaves_no_scan_in_progress() {
        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (go, handle) = spawn_scan(&scans, 1);
        index::lock(&scans.0).running = Some((
            1,
            Arc::new(AtomicBool::new(false)),
            ScanFocus::default(),
            handle,
        ));
        let _ = go.send(());
        // Wait for the task the way `scan_folder` does, without taking the
        // entry: what is asserted is that the task cleared it itself.
        let waited = tauri::async_runtime::spawn_blocking({
            let scans = scans.clone();
            move || {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                while index::lock(&scans.0).running.is_some()
                    && std::time::Instant::now() < deadline
                {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        });
        tauri::async_runtime::block_on(waited).unwrap();
        assert!(!index::lock(&scans.0).scanning());
    }

    #[test]
    fn a_scan_started_after_the_done_signal_finds_nothing_to_join() {
        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (go, rx) = mpsc::channel::<()>();
        let (done_tx, done_rx) = mpsc::channel::<()>();
        // The task's tail in the order `start_scan`'s closure has it: drop
        // the entry, then signal `faces-done`.
        let handle = tauri::async_runtime::spawn_blocking({
            let scans = scans.clone();
            move || {
                let _ = rx.recv();
                index::lock(&scans.0).finish(1);
                let _ = done_tx.send(());
            }
        });
        index::lock(&scans.0).running = Some((
            1,
            Arc::new(AtomicBool::new(false)),
            ScanFocus::default(),
            handle,
        ));
        let _ = go.send(());
        done_rx.recv().unwrap();
        let mut state = index::lock(&scans.0);
        assert!(!state.scanning());
        assert!(state.running.take().is_none());
    }

    #[test]
    fn a_finished_faces_pass_leaves_no_scan_in_progress() {
        let dir = temp_dir("faces-pass");
        let raw = dir.join("a.ARW");
        std::fs::write(&raw, b"x").unwrap();
        let index = sidecar_index(&dir);
        let dir_name = dir.to_string_lossy().into_owned();
        index_files(&index, &dir_name, &[raw.to_string_lossy().into_owned()]);

        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (go, rx) = mpsc::channel::<()>();
        let (summary_tx, summary_rx) = mpsc::channel();
        let handle = tauri::async_runtime::spawn_blocking({
            let (scans, index, dir_name) = (scans.clone(), index.clone(), dir_name.clone());
            move || {
                let _ = rx.recv();
                let summary = super::run_faces_pass(
                    &index,
                    &dir_name,
                    2,
                    &ScanFocus::default(),
                    &AtomicBool::new(false),
                    |_, _, _| {},
                );
                let _ = summary_tx.send(summary.errors);
                index::lock(&scans.0).finish(1);
            }
        });
        index::lock(&scans.0).running = Some((
            1,
            Arc::new(AtomicBool::new(false)),
            ScanFocus::default(),
            handle,
        ));
        let _ = go.send(());
        assert_eq!(summary_rx.recv().unwrap(), 0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while index::lock(&scans.0).running.is_some() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!index::lock(&scans.0).scanning());
        assert!(index::lock(&index)
            .faces_todo(&dir_name)
            .unwrap()
            .is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_superseded_scan_does_not_clear_the_newer_scan() {
        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (old_go, old_handle) = spawn_scan(&scans, 1);
        let old_cancel = Arc::new(AtomicBool::new(false));
        index::lock(&scans.0).running = Some((1, old_cancel, ScanFocus::default(), old_handle));

        // `scan_folder` supersedes it: take the entry, cancel, join.
        let (_, old_cancel, _, old_handle) = index::lock(&scans.0).running.take().unwrap();
        old_cancel.store(true, std::sync::atomic::Ordering::Relaxed);

        // The newer scan is already stored when the old task gets to run.
        let (new_go, new_handle) = spawn_scan(&scans, 2);
        index::lock(&scans.0).running = Some((
            2,
            Arc::new(AtomicBool::new(false)),
            ScanFocus::default(),
            new_handle,
        ));

        let _ = old_go.send(());
        tauri::async_runtime::block_on(old_handle).unwrap();

        let state = index::lock(&scans.0);
        assert!(state.scanning());
        assert_eq!(state.running.as_ref().unwrap().0, 2);
        drop(state);
        let _ = new_go.send(());
    }

    #[test]
    fn a_scan_in_its_embed_pass_is_not_scanning_and_stops_on_request() {
        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (go, handle) = spawn_scan(&scans, 1);
        let cancel = Arc::new(AtomicBool::new(false));
        let mut state = index::lock(&scans.0);
        state.running = Some((1, cancel.clone(), ScanFocus::default(), handle));
        state.stop_embedding();
        assert!(!cancel.load(Ordering::Relaxed), "passes 1 and 2 go on");
        assert!(state.scanning());

        state.start_embedding(2);
        assert!(state.scanning(), "another id's pass 3 is not this one's");
        state.start_embedding(1);
        assert!(!state.scanning());
        assert!(state.in_embed_pass());
        state.stop_embedding();
        assert!(cancel.load(Ordering::Relaxed));

        state.finish(1);
        assert_eq!(state.embedding, None);
        assert!(!state.scanning());
        drop(state);
        let _ = go.send(());
    }

    #[test]
    fn canceling_a_scan_by_its_id_sets_its_flag_and_it_scans_until_it_finishes() {
        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (go, handle) = spawn_scan(&scans, 1);
        let cancel = Arc::new(AtomicBool::new(false));
        let mut state = index::lock(&scans.0);
        state.running = Some((1, cancel.clone(), ScanFocus::default(), handle));

        state.cancel(1);
        assert!(cancel.load(Ordering::Relaxed));
        assert!(state.scanning(), "the task has not finished yet");

        state.finish(1);
        assert!(!state.scanning());
        drop(state);
        let _ = go.send(());
    }

    #[test]
    fn canceling_a_stale_id_leaves_the_running_scan_alone() {
        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (go, handle) = spawn_scan(&scans, 2);
        let cancel = Arc::new(AtomicBool::new(false));
        let mut state = index::lock(&scans.0);
        state.cancel(1);
        state.running = Some((2, cancel.clone(), ScanFocus::default(), handle));
        state.cancel(1);
        assert!(!cancel.load(Ordering::Relaxed));
        drop(state);
        let _ = go.send(());
    }

    #[test]
    fn canceling_leaves_a_scan_in_its_embed_pass_alone() {
        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (go, handle) = spawn_scan(&scans, 1);
        let cancel = Arc::new(AtomicBool::new(false));
        let mut state = index::lock(&scans.0);
        state.running = Some((1, cancel.clone(), ScanFocus::default(), handle));
        state.start_embedding(1);
        state.cancel(1);
        assert!(!cancel.load(Ordering::Relaxed));
        drop(state);
        let _ = go.send(());
    }

    /// The plan's Step 3 measurement of the third pass without the GUI: the
    /// folder `RIFFLE_EMBED_DIR` through the app's three passes into a fresh
    /// index, at the app's priorities and thread counts (`RIFFLE_EMBED_THREADS`
    /// overrides the third pass's), with the model file
    /// `RIFFLE_EMBED_MODEL`. It prints each pass's wall time and the
    /// process's largest working set sampled during it, and the read + full
    /// decode time of a preview on a normal-priority thread (a stand-in for
    /// the viewer's paging) idle and during the second and third passes; then
    /// checks that a rescan has nothing left to embed. `cargo test --release
    /// -p riffle-app embed_pass_on_a_real_folder -- --ignored --nocapture`.
    #[test]
    #[ignore = "needs RIFFLE_EMBED_DIR and RIFFLE_EMBED_MODEL"]
    fn embed_pass_on_a_real_folder() {
        use std::time::{Duration, Instant};
        let (Ok(dir), Ok(model)) = (
            std::env::var("RIFFLE_EMBED_DIR"),
            std::env::var("RIFFLE_EMBED_MODEL"),
        ) else {
            panic!("set RIFFLE_EMBED_DIR and RIFFLE_EMBED_MODEL");
        };
        let embed_threads = std::env::var("RIFFLE_EMBED_THREADS")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or_else(embed_threads);
        let tmp = temp_dir("embed-real");
        let index = sidecar_index(&tmp);
        let mut files: Vec<FileStat> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| riffle_core::scan::is_raw_file(p))
            .filter_map(|p| index::stat(&p).ok())
            .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let (todo, _) = index::lock(&index).reconcile(&dir, &files).unwrap();
        let (focus, cancel, threads) =
            (ScanFocus::default(), AtomicBool::new(false), scan_threads());

        let probe = |i: usize| {
            let path = &files[(i * 37) % files.len()].path;
            let t = Instant::now();
            let (_, jpeg) = riffle_core::reader::read_preview(path).unwrap();
            riffle_core::decode::decode_rgb(&jpeg).unwrap();
            t.elapsed().as_secs_f64() * 1000.0
        };
        let summarize = |mut ms: Vec<f64>| {
            ms.sort_by(f64::total_cmp);
            let at = |q: f64| ms[((ms.len() - 1) as f64 * q).round() as usize];
            format!(
                "n={} median {:.1} ms, p95 {:.1} ms",
                ms.len(),
                at(0.5),
                at(0.95)
            )
        };
        // Runs `pass` with the working-set sampler and the preview probe
        // alongside, and prints what they saw.
        let measured = |name: &str, pass: &dyn Fn() -> index::ScanSummary| {
            let running = AtomicBool::new(true);
            let (summary, wall, peak, busy) = std::thread::scope(|s| {
                let sampler = s.spawn(|| {
                    let pid = sysinfo::get_current_pid().unwrap();
                    let mut sys = sysinfo::System::new();
                    let mut peak = 0;
                    while running.load(Ordering::Relaxed) {
                        sys.refresh_processes_specifics(
                            sysinfo::ProcessesToUpdate::Some(&[pid]),
                            false,
                            sysinfo::ProcessRefreshKind::nothing().with_memory(),
                        );
                        peak = peak.max(sys.process(pid).map_or(0, |p| p.memory()));
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    peak
                });
                let prober = s.spawn(|| {
                    let mut ms = Vec::new();
                    while running.load(Ordering::Relaxed) {
                        ms.push(probe(ms.len()));
                        std::thread::sleep(Duration::from_millis(250));
                    }
                    ms
                });
                let t = Instant::now();
                let summary = pass();
                let wall = t.elapsed();
                running.store(false, Ordering::Relaxed);
                (
                    summary,
                    wall,
                    sampler.join().unwrap(),
                    prober.join().unwrap(),
                )
            });
            eprintln!(
                "{name}: {} files, {} errors, in {:.1} s ({:.1} ms/file); peak working set {:.0} MB; preview read + decode {}",
                summary.total,
                summary.errors,
                wall.as_secs_f64(),
                wall.as_secs_f64() * 1000.0 / summary.total.max(1) as f64,
                peak as f64 / 1e6,
                summarize(busy)
            );
            summary
        };

        let t = Instant::now();
        let first = index::run_scan(
            &index,
            &dir,
            &todo,
            threads,
            &focus,
            &cancel,
            index::PROGRESS_INTERVAL,
            |_, _, _| {},
        );
        eprintln!(
            "pass 1: {} files in {:.1} s",
            first.total,
            t.elapsed().as_secs_f64()
        );
        let idle: Vec<f64> = (0..40).map(probe).collect();
        eprintln!("preview read + decode, idle: {}", summarize(idle));
        measured(&format!("pass 2 ({threads} threads)"), &|| {
            run_faces_pass(&index, &dir, threads, &focus, &cancel, |_, _, _| {})
        });
        let todo = index::lock(&index).embeddings_missing(&dir).unwrap();
        let summary = measured(&format!("pass 3 ({embed_threads} threads)"), &|| {
            run_embed_pass(
                &index,
                &dir,
                Path::new(&model),
                &todo,
                embed_threads,
                &focus,
                &cancel,
                |_, _| {},
            )
        });
        assert_eq!(summary.total, todo.len());
        assert!(
            index::lock(&index)
                .embeddings_missing(&dir)
                .unwrap()
                .is_empty(),
            "a rescan embeds 0 files"
        );
        remove_temp_dir(&tmp);
    }

    /// The ranking's input as the app stores it, for the plan's `ranking.md`:
    /// indexes every folder of `RIFFLE_RANK_DIRS` (separated like `PATH`)
    /// into `RIFFLE_RANK_OUT/index.sqlite` with the app's three passes and
    /// its sidecar reconciliation (XMP and `.dop`), resuming what an earlier
    /// run left, then writes what `Index::rank_rows` reads:
    /// `RIFFLE_RANK_OUT/app/<folder>.tsv` and `RIFFLE_RANK_OUT/embed.tsv`.
    /// `RIFFLE_EMBED_MODEL` is the SigLIP 2 file. Run it with `cargo test
    /// --release -p riffle-app rank_dump_of_real_folders -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore = "needs RIFFLE_RANK_DIRS, RIFFLE_RANK_OUT and RIFFLE_EMBED_MODEL"]
    fn rank_dump_of_real_folders() {
        use std::io::Write;
        let (Some(dirs), Ok(out), Ok(model)) = (
            std::env::var_os("RIFFLE_RANK_DIRS"),
            std::env::var("RIFFLE_RANK_OUT"),
            std::env::var("RIFFLE_EMBED_MODEL"),
        ) else {
            panic!("set RIFFLE_RANK_DIRS, RIFFLE_RANK_OUT and RIFFLE_EMBED_MODEL");
        };
        let out = PathBuf::from(out);
        std::fs::create_dir_all(out.join("app")).unwrap();
        let index = sidecar_index(&out);
        let (focus, cancel, threads) =
            (ScanFocus::default(), AtomicBool::new(false), scan_threads());
        let mut embed = std::fs::File::create(out.join("embed.tsv")).unwrap();
        let opt = |v: Option<f64>| v.map_or("-".to_string(), |v| v.to_string());
        for dir in std::env::split_paths(&dirs) {
            let started = std::time::Instant::now();
            let dir = canonicalize(&dir.to_string_lossy());
            let (listed, sidecars) = list_folder_in(Path::new(&dir), SidecarFormat::Both).unwrap();
            let files: Vec<FileStat> = listed
                .iter()
                .filter_map(|p| index::stat(Path::new(p)).ok())
                .collect();
            let (todo, _) = index::lock(&index).reconcile(&dir, &files).unwrap();
            reconcile_sidecars_of(
                &dir,
                &listed,
                &sidecars,
                &index,
                SidecarFormat::Both,
                &LabelNames::default(),
            )
            .unwrap();
            let first = index::run_scan(
                &index,
                &dir,
                &todo,
                threads,
                &focus,
                &cancel,
                index::PROGRESS_INTERVAL,
                |_, _, _| {},
            );
            let second = run_faces_pass(&index, &dir, threads, &focus, &cancel, |_, _, _| {});
            let todo = index::lock(&index).embeddings_missing(&dir).unwrap();
            let third = run_embed_pass(
                &index,
                &dir,
                Path::new(&model),
                &todo,
                threads,
                &focus,
                &cancel,
                |_, _| {},
            );
            let rows = index::lock(&index).rank_rows(&dir).unwrap();
            let folder = Path::new(&dir).file_name().unwrap().to_string_lossy();
            let mut tsv =
                std::fs::File::create(out.join("app").join(format!("{folder}.tsv"))).unwrap();
            writeln!(tsv, "file\tflag\tcapture_time\tsubsec\tsharpness\tstate\teye_focus\tear\teyes_open\tyaw\tpitch\troll\teye_offset\tedge_gap").unwrap();
            for row in &rows {
                let f = &row.frame;
                writeln!(
                    tsv,
                    "{}\t{:?}\t{}\t{}\t{}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    f.name,
                    row.flag,
                    f.capture_time.as_deref().unwrap_or("-"),
                    f.subsec.as_deref().unwrap_or("-"),
                    opt(f.sharpness),
                    riffle_core::candidate::candidate(f.eye_focus),
                    opt(f.eye_focus),
                    opt(f.eyes_ear),
                    opt(f
                        .eyes_ear
                        .map(|e| 1.0 - riffle_core::eyes::closed_probability(e))),
                    opt(f.pose.map(|p| p.yaw)),
                    opt(f.pose.map(|p| p.pitch)),
                    opt(f.pose.map(|p| p.roll)),
                    opt(f.eye_offset),
                    opt(f.edge_gap),
                )
                .unwrap();
                if let Some(vec) = &f.vec {
                    let vec: Vec<String> = vec.iter().map(f32::to_string).collect();
                    writeln!(embed, "{}\t{}", row.path, vec.join("\t")).unwrap();
                }
            }
            eprintln!(
                "{folder}: {} rows, {} with a vector; pass 1 {} files ({} errors), pass 2 {} ({}), pass 3 {} ({}); {:.1} s",
                rows.len(),
                rows.iter().filter(|r| r.frame.vec.is_some()).count(),
                first.total,
                first.errors,
                second.total,
                second.errors,
                third.total,
                third.errors,
                started.elapsed().as_secs_f64()
            );
        }
    }

    #[test]
    fn the_scan_focus_reaches_the_running_scan_only() {
        let scans = Arc::new(Scans(Mutex::new(ScansState::default())));
        let (go, handle) = spawn_scan(&scans, 2);
        let focus = ScanFocus::default();
        index::lock(&scans.0).running =
            Some((2, Arc::new(AtomicBool::new(false)), focus.clone(), handle));
        let order = || {
            let paths: Vec<std::path::PathBuf> = ["a.ARW", "b.ARW", "c.ARW"].map(Into::into).into();
            let seen = Mutex::new(Vec::new());
            riffle_core::scan::extract_all(
                &paths,
                1,
                riffle_core::scan::Priority::Normal,
                &focus,
                |i, _| seen.lock().unwrap().push(i),
                &AtomicBool::new(false),
            )
            .unwrap();
            seen.into_inner().unwrap()
        };

        index::lock(&scans.0).set_focus(1, vec!["c.ARW".into()]);
        assert_eq!(order(), vec![0, 1, 2], "a stale id is ignored");

        index::lock(&scans.0).set_focus(2, vec!["c.ARW".into(), "b.ARW".into()]);
        assert_eq!(order(), vec![2, 1, 0], "the running id reaches the handle");

        let _ = go.send(());
    }

    #[test]
    fn a_folder_being_prepared_counts_as_a_scan_in_progress() {
        let mut state = ScansState::default();
        assert!(!state.scanning());
        state.preparing = 1;
        assert!(state.scanning());
    }

    #[test]
    fn format_bytes_uses_whole_bytes_below_the_base() {
        assert_eq!(format_bytes(0, 1000), "0 B");
        assert_eq!(format_bytes(0, 1024), "0 B");
        assert_eq!(format_bytes(999, 1000), "999 B");
        assert_eq!(format_bytes(1023, 1024), "1023 B");
    }

    #[test]
    fn format_bytes_steps_up_a_unit_at_each_power_of_the_base() {
        assert_eq!(format_bytes(1000, 1000), "1.0 KB");
        assert_eq!(format_bytes(1024, 1024), "1.0 KB");
        assert_eq!(format_bytes(1000 * 1000, 1000), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024, 1024), "1.0 MB");
        assert_eq!(format_bytes(1000 * 1000 * 1000, 1000), "1.0 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024, 1024), "1.0 GB");
    }

    #[test]
    fn format_bytes_shows_one_decimal_from_kb_up() {
        assert_eq!(format_bytes(1_200_000_000, 1000), "1.2 GB");
        assert_eq!(format_bytes(1288490189, 1024), "1.2 GB");
        assert_eq!(format_bytes(312_000, 1000), "312.0 KB");
    }

    #[test]
    fn format_bytes_steps_up_when_rounding_would_reach_the_base() {
        // Each of these divides to a rounded X.0 at the current unit, so it
        // must already step up rather than print e.g. "1000.0 MB".
        assert_eq!(format_bytes(999_999_999, 1000), "1.0 GB");
        assert_eq!(format_bytes(1_073_741_823, 1024), "1.0 GB");
        assert_eq!(format_bytes(999_999, 1000), "1.0 MB");
    }

    #[test]
    fn the_size_base_follows_the_platforms_file_manager() {
        // macOS Finder reports sizes in decimal units (1 GB = 1000^3 bytes);
        // Windows Explorer and most Linux file managers report binary units
        // (1 GB = 1024^3 bytes).
        let expected = if cfg!(target_os = "macos") {
            "1.1 GB"
        } else {
            "1.0 GB"
        };
        assert_eq!(format_bytes(1_073_741_824, SIZE_BASE), expected);
    }

    #[test]
    fn the_preview_pixel_limit_applies_on_linux_only() {
        // WebKitGTK drops large transferred bitmaps; other webviews do not.
        let expected = if cfg!(target_os = "linux") {
            Some(6_000_000)
        } else {
            None
        };
        assert_eq!(PREVIEW_PIXEL_LIMIT, expected);
    }

    #[test]
    fn label_names_setting_falls_back_to_english_per_color() {
        use serde_json::json;
        assert_eq!(super::label_names_setting(None), LabelNames::default());
        let names = super::label_names_setting(Some(&json!({
            "red": " レッド ",
            "yellow": "",
            "green": 3,
            "purple": "パープル",
        })));
        assert_eq!(
            names,
            LabelNames {
                red: "レッド".to_string(),
                purple: "パープル".to_string(),
                ..LabelNames::default()
            }
        );
        assert_eq!(
            super::label_names_setting(Some(&super::label_names_value(&japanese()))),
            japanese()
        );
    }

    #[test]
    fn label_names_payload_lists_the_presets_english_first() {
        use serde_json::json;
        let payload = super::label_names_payload(&japanese());
        assert_eq!(payload["names"], super::label_names_value(&japanese()));
        let presets = payload["presets"].as_array().unwrap();
        assert_eq!(
            presets[0],
            json!({
                "code": "en",
                "name": "English",
                "names": super::label_names_value(&LabelNames::default()),
            })
        );
        assert!(presets.contains(&json!({
            "code": "ja",
            "name": "日本語",
            "names": super::label_names_value(&japanese()),
        })));
    }

    #[test]
    fn panels_setting_falls_back_to_shown() {
        use serde_json::json;
        let all = json!({"left": true, "strip": true, "right": true});
        assert_eq!(super::panels_setting(None), all);
        assert_eq!(super::panels_setting(Some(&json!("hidden"))), all);
        assert_eq!(
            super::panels_setting(Some(&json!({"left": "no", "strip": null}))),
            all
        );
        assert_eq!(
            super::panels_setting(Some(&json!({"left": false, "right": false, "extra": 1}))),
            json!({"left": false, "strip": true, "right": false})
        );
    }

    #[test]
    fn filter_setting_falls_back_to_empty() {
        use serde_json::json;
        let empty = json!({
            "flags": [], "stars": [], "labels": [],
            "orientations": [], "candidates": [], "eyes": [],
        });
        assert_eq!(super::filter_setting(None), empty);
        assert_eq!(super::filter_setting(Some(&json!("x"))), empty);
        assert_eq!(super::filter_setting(Some(&json!([]))), empty);
        assert_eq!(
            super::filter_setting(Some(&json!({"flags": "picked", "stars": {"0": 1}}))),
            empty
        );
    }

    #[test]
    fn filter_setting_drops_unknown_members() {
        use serde_json::json;
        assert_eq!(
            super::filter_setting(Some(&json!({
                "flags": ["picked", "bogus", "picked"],
                "stars": [5, 7, "3", 2.5, 0, 6],
                "labels": ["Red", "red", "none"],
                "orientations": ["square", "landscape"],
                "candidates": ["ok", "good", "unknown"],
                "eyes": ["closed", 1],
            }))),
            json!({
                "flags": ["picked"],
                "stars": [5, 0],
                "labels": ["red", "none"],
                "orientations": ["landscape"],
                "candidates": ["good", "unknown"],
                "eyes": ["closed"],
            })
        );
    }

    #[test]
    fn filter_setting_keeps_a_valid_value_and_drops_extra_keys() {
        use serde_json::json;
        let valid = json!({
            "flags": ["rejected", "picked"],
            "stars": [3, 1],
            "labels": ["purple", "green"],
            "orientations": ["portrait"],
            "candidates": ["not_candidate"],
            "eyes": ["unknown", "open"],
        });
        assert_eq!(super::filter_setting(Some(&valid)), valid);
        let mut extra = valid.clone();
        extra["exif"] = json!({"ISO": ["100"]});
        assert_eq!(super::filter_setting(Some(&extra)), valid);
    }

    #[test]
    fn auto_advance_setting_reads_a_boolean_or_defaults_to_on() {
        use serde_json::json;
        assert!(super::auto_advance_setting(None));
        assert!(super::auto_advance_setting(Some(&json!(true))));
        assert!(!super::auto_advance_setting(Some(&json!(false))));
        assert!(super::auto_advance_setting(Some(&json!("true"))));
    }

    #[test]
    fn ai_enabled_setting_reads_a_boolean_or_defaults_to_off() {
        use serde_json::json;
        assert!(!super::ai_enabled_setting(None));
        assert!(super::ai_enabled_setting(Some(&json!(true))));
        assert!(!super::ai_enabled_setting(Some(&json!(false))));
        assert!(!super::ai_enabled_setting(Some(&json!("true"))));
    }

    use std::time::Duration;

    use super::*;

    /// The names of Lightroom's Japanese default color label set.
    fn japanese() -> LabelNames {
        riffle_core::i18n::preset("ja").unwrap().names.clone()
    }

    /// `reconcile_sidecars_of` over a fresh listing of `dir`, as `scan_folder`
    /// hands it the sidecars of its own single listing.
    fn reconcile_listed(
        dir: &str,
        listed: &[String],
        index: &Arc<Mutex<Index>>,
        format: SidecarFormat,
    ) -> Result<Vec<index::DirtyRow>, String> {
        reconcile_listed_with_errors(dir, listed, index, format).map(|(dirty, _, _)| dirty)
    }

    fn reconcile_listed_with_errors(
        dir: &str,
        listed: &[String],
        index: &Arc<Mutex<Index>>,
        format: SidecarFormat,
    ) -> Result<(Vec<index::DirtyRow>, Vec<SidecarError>, usize), String> {
        let (_, sidecars) = list_folder_in(Path::new(dir), format)?;
        reconcile_sidecars_of(
            dir,
            listed,
            &sidecars,
            index,
            format,
            &LabelNames::default(),
        )
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // Best effort, like the removal in temp_dir above: on Windows a directory
    // holding an open SQLite database cannot be removed, and several of these
    // tests still hold the Index when they finish. The next run's temp_dir
    // clears whatever is left over.
    fn remove_temp_dir(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A foreign sidecar, shaped like the one Bridge writes.
    fn sidecar(dir: &Path, name: &str, rating: i32) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(
            &path,
            format!(
                r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="{rating}"/>
 </rdf:RDF>
</x:xmpmeta>"#
            ),
        )
        .unwrap();
        path
    }

    fn sidecar_index(dir: &Path) -> Arc<Mutex<Index>> {
        Arc::new(Mutex::new(Index::open(&dir.join("index.sqlite")).unwrap()))
    }

    fn rating_of(index: &Arc<Mutex<Index>>, dir: &str, path: &str) -> Option<i8> {
        index::lock(index)
            .entries(dir)
            .unwrap()
            .into_iter()
            .find(|e| e.path == path)
            .unwrap()
            .rating
    }

    fn flag_of(index: &Arc<Mutex<Index>>, dir: &str, path: &str) -> Flag {
        index::lock(index)
            .entries(dir)
            .unwrap()
            .into_iter()
            .find(|e| e.path == path)
            .unwrap()
            .flag
    }

    /// Give a file the `(size, mtime)` it had before it was rewritten, so a
    /// content change that the reconciliation is meant to miss really is
    /// invisible to it.
    fn restore_mtime(path: &Path, mtime_ns: i64) {
        let time = std::time::UNIX_EPOCH + Duration::from_nanos(mtime_ns as u64);
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(time)
            .unwrap();
    }

    #[test]
    fn a_foreign_sidecar_is_read_on_the_first_open_without_any_files_row() {
        let root = temp_dir("sidecar-first-open");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        sidecar(&root, "a.xmp", 4);
        let index = sidecar_index(&root);

        let listed = list_arw_in(&root).unwrap();
        let dirty = reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();

        assert!(dirty.is_empty());
        // No `files` row exists yet, so `entries` cannot show it; the row is
        // there all the same, which is what the next scan will join against.
        assert!(index::lock(&index).entries(&dir).unwrap().is_empty());
        index::lock(&index)
            .write_batch(
                &dir,
                &[(index::stat(Path::new(&listed[0])).unwrap(), Err("x".into()))],
            )
            .unwrap();
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(4));

        remove_temp_dir(&root);
    }

    #[test]
    fn a_parsed_sidecar_counts_as_changed_and_an_unchanged_one_does_not() {
        let root = temp_dir("sidecar-changed-count");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        sidecar(&root, "a.xmp", 4);
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        let (_, _, changed) =
            reconcile_listed_with_errors(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        assert_eq!(changed, 1);
        let (_, _, changed) =
            reconcile_listed_with_errors(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        assert_eq!(changed, 0);

        remove_temp_dir(&root);
    }

    #[test]
    fn an_external_edit_wins_over_a_dirty_row_and_an_unchanged_stat_parses_nothing() {
        let root = temp_dir("sidecar-external");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        let file = sidecar(&root, "a.xmp", 2);
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        // An app edit that never reached disk, then someone else's edit.
        index::lock(&index)
            .set_rating(&dir, &listed[0], Some(5), Flag::None, None, true)
            .unwrap();
        sidecar(&root, "a.xmp", 3);

        let dirty = reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        assert!(dirty.is_empty(), "the sidecar won, so nothing is written");
        assert_eq!(
            index::lock(&index).dirty_rows(&dir).unwrap(),
            Vec::new(),
            "and the row is no longer dirty"
        );

        // The same stat parses nothing: the content is changed underneath
        // without touching size or mtime, and the stored rating does not move.
        let stat = index::stat(&file).unwrap();
        sidecar(&root, "a.xmp", 1);
        restore_mtime(&file, stat.mtime_ns);
        assert!(reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp)
            .unwrap()
            .is_empty());

        index::lock(&index)
            .write_batch(
                &dir,
                &[(index::stat(Path::new(&listed[0])).unwrap(), Err("x".into()))],
            )
            .unwrap();
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(3));

        remove_temp_dir(&root);
    }

    #[test]
    fn a_deleted_sidecar_clears_the_rating_and_a_dirty_row_survives_it() {
        let root = temp_dir("sidecar-deleted");
        let dir = root.to_string_lossy().into_owned();
        for name in ["a.ARW", "b.ARW", "c.ARW"] {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        let file = sidecar(&root, "a.xmp", 1);
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();
        assert!(reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp)
            .unwrap()
            .is_empty());

        // `a` had a sidecar and lost it: the truth is gone with it. `b` has a
        // judgment that never reached a sidecar, and must be written instead.
        // `c` has neither a sidecar nor a row, and is not a case at all.
        std::fs::remove_file(&file).unwrap();
        index::lock(&index)
            .set_rating(&dir, &listed[1], None, Flag::Reject, None, true)
            .unwrap();

        let dirty = reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        assert_eq!(dirty, [(listed[1].clone(), None, Flag::Reject, None, true)]);

        let files: Vec<_> = listed
            .iter()
            .map(|p| (index::stat(Path::new(p)).unwrap(), Err("x".into())))
            .collect();
        index::lock(&index).write_batch(&dir, &files).unwrap();
        assert_eq!(rating_of(&index, &dir, &listed[0]), None);
        assert_eq!(rating_of(&index, &dir, &listed[2]), None);

        remove_temp_dir(&root);
    }

    #[test]
    fn a_sidecar_differing_only_in_case_is_the_one_that_is_read() {
        let root = temp_dir("sidecar-case");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        sidecar(&root, "A.XMP", 5);
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        index::lock(&index)
            .write_batch(
                &dir,
                &[(index::stat(Path::new(&listed[0])).unwrap(), Err("x".into()))],
            )
            .unwrap();
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(5));

        remove_temp_dir(&root);
    }

    const PHOTOLAB_PICK: &[u8] = include_bytes!("../../core/src/fixtures/dop/_DSC0001.ARW.dop");
    const PHOTOLAB_REJECT: &[u8] = include_bytes!("../../core/src/fixtures/dop/_DSC0002.ARW.dop");
    const PHOTOLAB_THREE: &[u8] = include_bytes!("../../core/src/fixtures/dop/_DSC0003.ARW.dop");

    /// Store the `files` rows of `listed` so `entries` can show their ratings.
    fn index_files(index: &Arc<Mutex<Index>>, dir: &str, listed: &[String]) {
        let files: Vec<_> = listed
            .iter()
            .map(|p| (index::stat(Path::new(p)).unwrap(), Err("x".into())))
            .collect();
        index::lock(index).write_batch(dir, &files).unwrap();
    }

    #[test]
    fn under_both_the_newer_of_two_disagreeing_sidecars_is_read() {
        let root = temp_dir("both-newest");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        let xmp = sidecar(&root, "a.xmp", 4);
        let dop = root.join("a.ARW.dop");
        std::fs::write(&dop, PHOTOLAB_THREE).unwrap();
        let listed = list_arw_in(&root).unwrap();
        let (old, new) = (1_600_000_000_000_000_000, 1_700_000_000_000_000_000);

        for (run, (xmp_ns, dop_ns, expected)) in [
            (new, old, Some(4)),
            (old, new, Some(3)),
            (old, old, Some(4)),
        ]
        .into_iter()
        .enumerate()
        {
            let index = Arc::new(Mutex::new(
                Index::open(&root.join(format!("index-{run}.sqlite"))).unwrap(),
            ));
            restore_mtime(&xmp, xmp_ns);
            restore_mtime(&dop, dop_ns);
            reconcile_listed(&dir, &listed, &index, SidecarFormat::Both).unwrap();
            index_files(&index, &dir, &listed);
            assert_eq!(
                rating_of(&index, &dir, &listed[0]),
                expected,
                "xmp at {xmp_ns}, dop at {dop_ns}"
            );
        }

        remove_temp_dir(&root);
    }

    #[test]
    fn under_both_a_lone_xmp_or_dop_is_read() {
        let root = temp_dir("both-lone");
        let dir = root.to_string_lossy().into_owned();
        for name in ["a.ARW", "b.ARW"] {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        sidecar(&root, "a.xmp", 4);
        std::fs::write(root.join("b.ARW.dop"), PHOTOLAB_REJECT).unwrap();
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        let dirty = reconcile_listed(&dir, &listed, &index, SidecarFormat::Both).unwrap();

        assert!(dirty.is_empty());
        index_files(&index, &dir, &listed);
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(4));
        assert_eq!(flag_of(&index, &dir, &listed[0]), Flag::None);
        assert_eq!(flag_of(&index, &dir, &listed[1]), Flag::Reject);

        remove_temp_dir(&root);
    }

    #[test]
    fn photolab_ratings_and_rejects_are_read_on_the_first_open_and_xmps_ignored() {
        let root = temp_dir("dop-first-open");
        let dir = root.to_string_lossy().into_owned();
        for name in ["a.ARW", "b.ARW", "c.ARW"] {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        std::fs::write(root.join("a.ARW.dop"), PHOTOLAB_REJECT).unwrap();
        std::fs::write(root.join("b.ARW.dop"), PHOTOLAB_THREE).unwrap();
        sidecar(&root, "c.xmp", 4);
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        let dirty = reconcile_listed(&dir, &listed, &index, SidecarFormat::Dop).unwrap();

        assert!(dirty.is_empty());
        index_files(&index, &dir, &listed);
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(0));
        assert_eq!(flag_of(&index, &dir, &listed[0]), Flag::Reject);
        assert_eq!(rating_of(&index, &dir, &listed[1]), Some(3));
        assert_eq!(flag_of(&index, &dir, &listed[1]), Flag::None);
        assert_eq!(rating_of(&index, &dir, &listed[2]), None);

        remove_temp_dir(&root);
    }

    fn label_of(index: &Arc<Mutex<Index>>, dir: &str, path: &str) -> Option<String> {
        index::lock(index)
            .entries(dir)
            .unwrap()
            .into_iter()
            .find(|e| e.path == path)
            .unwrap()
            .label
    }

    #[test]
    fn photolab_labels_are_read_on_the_first_open() {
        let root = temp_dir("dop-labels");
        let dir = root.to_string_lossy().into_owned();
        let fixtures: [(&[u8], &str); 7] = [
            (
                include_bytes!("../../core/src/fixtures/dop/_DSC0009.ARW.dop"),
                "Red",
            ),
            (
                include_bytes!("../../core/src/fixtures/dop/_DSC0010.ARW.dop"),
                "Orange",
            ),
            (
                include_bytes!("../../core/src/fixtures/dop/_DSC0011.ARW.dop"),
                "Yellow",
            ),
            (
                include_bytes!("../../core/src/fixtures/dop/_DSC0012.ARW.dop"),
                "Green",
            ),
            (
                include_bytes!("../../core/src/fixtures/dop/_DSC0013.ARW.dop"),
                "Blue",
            ),
            (
                include_bytes!("../../core/src/fixtures/dop/_DSC0014.ARW.dop"),
                "Pink",
            ),
            (
                include_bytes!("../../core/src/fixtures/dop/_DSC0015.ARW.dop"),
                "Purple",
            ),
        ];
        for (i, (bytes, _)) in fixtures.iter().enumerate() {
            std::fs::write(root.join(format!("_DSC00{}.ARW", 9 + i)), b"x").unwrap();
            std::fs::write(root.join(format!("_DSC00{}.ARW.dop", 9 + i)), bytes).unwrap();
        }
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        reconcile_listed(&dir, &listed, &index, SidecarFormat::Dop).unwrap();
        index_files(&index, &dir, &listed);

        for (i, (_, label)) in fixtures.iter().enumerate() {
            let path = root.join(format!("_DSC00{}.ARW", 9 + i));
            let path = listed
                .iter()
                .find(|p| Path::new(p).file_name() == path.file_name())
                .unwrap();
            assert_eq!(label_of(&index, &dir, path).as_deref(), Some(*label));
        }

        remove_temp_dir(&root);
    }

    #[test]
    fn an_xmp_label_name_is_read_under_the_configured_names() {
        for (names, want) in [(japanese(), "Red"), (LabelNames::default(), "レッド")] {
            let root = temp_dir("xmp-label-names");
            let dir = root.to_string_lossy().into_owned();
            std::fs::write(root.join("a.ARW"), b"x").unwrap();
            std::fs::write(
                root.join("a.xmp"),
                r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/"
   xmp:Label="レッド"/>
 </rdf:RDF>
</x:xmpmeta>"#,
            )
            .unwrap();
            let index = sidecar_index(&root);
            let listed = list_arw_in(&root).unwrap();
            let (_, sidecars) = list_folder_in(&root, SidecarFormat::Xmp).unwrap();

            reconcile_sidecars_of(&dir, &listed, &sidecars, &index, SidecarFormat::Xmp, &names)
                .unwrap();
            index_files(&index, &dir, &listed);
            assert_eq!(label_of(&index, &dir, &listed[0]).as_deref(), Some(want));

            remove_temp_dir(&root);
        }
    }

    #[test]
    fn an_xmp_label_is_read_and_a_deleted_sidecar_clears_it() {
        let root = temp_dir("xmp-label");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        let file = root.join("a.xmp");
        std::fs::write(
            &file,
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/"
   xmp:Rating="2" xmp:Label="Blue"/>
 </rdf:RDF>
</x:xmpmeta>"#,
        )
        .unwrap();
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        index_files(&index, &dir, &listed);
        assert_eq!(label_of(&index, &dir, &listed[0]).as_deref(), Some("Blue"));

        std::fs::remove_file(&file).unwrap();
        reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        assert_eq!(label_of(&index, &dir, &listed[0]), None);

        remove_temp_dir(&root);
    }

    #[test]
    fn a_pick_is_read_from_a_photolab_dop_and_from_a_lightroom_xmp() {
        let root = temp_dir("dop-pick");
        let dir = root.to_string_lossy().into_owned();
        for name in ["a.ARW", "b.ARW"] {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        std::fs::write(root.join("a.ARW.dop"), PHOTOLAB_PICK).unwrap();
        std::fs::write(root.join("b.ARW.dop"), PHOTOLAB_THREE).unwrap();
        std::fs::write(root.join("b.xmp"), lightroom_xmp(r#"xmpDM:good="True""#)).unwrap();
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();
        let flags = |index: &Arc<Mutex<Index>>| -> Vec<Flag> {
            index::lock(index)
                .entries(&dir)
                .unwrap()
                .into_iter()
                .map(|e| e.flag)
                .collect()
        };

        reconcile_listed(&dir, &listed, &index, SidecarFormat::Dop).unwrap();
        index_files(&index, &dir, &listed);
        assert_eq!(flags(&index), [Flag::Pick, Flag::None]);
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(0));

        index::lock(&index).reset_sidecars().unwrap();
        reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        assert_eq!(flags(&index), [Flag::None, Flag::Pick]);

        remove_temp_dir(&root);
    }

    /// A trimmed copy of a Lightroom desktop 9.5.1 (Japanese UI) sidecar,
    /// with `extra` as the judgment attributes beside `xmp:Rating`.
    fn lightroom_xmp(extra: &str) -> String {
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 7.0-c000 1.000000, 0000/00/00-00:00:00        ">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="Leica Camera AG"
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/"
    xmlns:xmpDM="http://ns.adobe.com/xmp/1.0/DynamicMedia/"
   xmp:CreatorTool="2.6.0"
   {extra}/>
 </rdf:RDF>
</x:xmpmeta>"#
        )
    }

    #[test]
    fn a_lightroom_folder_reads_its_flags_labels_and_stars() {
        let root = temp_dir("lightroom-folder");
        let dir = root.to_string_lossy().into_owned();
        let shots: [(&str, &str); 12] = [
            (
                "L1005428",
                r#"xmp:Rating="0" xmp:Label="パープル" photoshop:LabelColor="purple""#,
            ),
            (
                "L1005429",
                r#"xmp:Rating="0" xmp:Label="ブルー" photoshop:LabelColor="blue""#,
            ),
            (
                "L1005430",
                r#"xmp:Rating="0" xmp:Label="グリーン" photoshop:LabelColor="green""#,
            ),
            (
                "L1005431",
                r#"xmp:Rating="0" xmp:Label="イエロー" photoshop:LabelColor="yellow""#,
            ),
            (
                "L1005432",
                r#"xmp:Rating="0" xmp:Label="レッド" photoshop:LabelColor="red""#,
            ),
            ("L1005433", r#"xmp:Rating="5""#),
            ("L1005434", r#"xmp:Rating="4""#),
            ("L1005435", r#"xmp:Rating="3""#),
            ("L1005436", r#"xmp:Rating="2""#),
            ("L1005437", r#"xmp:Rating="1""#),
            ("L1005438", r#"xmp:Rating="0" xmpDM:good="False""#),
            ("L1005439", r#"xmp:Rating="0" xmpDM:good="True""#),
        ];
        for (name, extra) in shots {
            std::fs::write(root.join(format!("{name}.DNG")), b"x").unwrap();
            std::fs::write(root.join(format!("{name}.xmp")), lightroom_xmp(extra)).unwrap();
        }
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        index_files(&index, &dir, &listed);

        let judged: Vec<(Option<i8>, Flag, Option<String>)> = listed
            .iter()
            .map(|p| {
                (
                    rating_of(&index, &dir, p),
                    flag_of(&index, &dir, p),
                    label_of(&index, &dir, p),
                )
            })
            .collect();
        let label = |l: &str| Some(l.to_string());
        assert_eq!(
            judged,
            [
                (Some(0), Flag::None, label("Purple")),
                (Some(0), Flag::None, label("Blue")),
                (Some(0), Flag::None, label("Green")),
                (Some(0), Flag::None, label("Yellow")),
                (Some(0), Flag::None, label("Red")),
                (Some(5), Flag::None, None),
                (Some(4), Flag::None, None),
                (Some(3), Flag::None, None),
                (Some(2), Flag::None, None),
                (Some(1), Flag::None, None),
                (Some(0), Flag::Reject, None),
                (Some(0), Flag::Pick, None),
            ]
        );

        remove_temp_dir(&root);
    }

    #[test]
    fn a_dop_differing_only_in_case_is_the_one_that_is_read() {
        let root = temp_dir("dop-case");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        std::fs::write(root.join("A.ARW.DOP"), PHOTOLAB_THREE).unwrap();
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();

        reconcile_listed(&dir, &listed, &index, SidecarFormat::Dop).unwrap();
        index_files(&index, &dir, &listed);
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(3));

        remove_temp_dir(&root);
    }

    #[test]
    fn an_oversize_dop_is_neither_read_nor_handed_to_the_writer() {
        let root = temp_dir("dop-oversize");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        let mut big = PHOTOLAB_THREE.to_vec();
        big.resize(MAX_SIDECAR_BYTES as usize + 1, b'\n');
        std::fs::write(root.join("a.ARW.dop"), big).unwrap();
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();
        index::lock(&index)
            .set_rating(&dir, &listed[0], Some(5), Flag::None, None, true)
            .unwrap();

        let (dirty, problems, _) =
            reconcile_listed_with_errors(&dir, &listed, &index, SidecarFormat::Dop).unwrap();

        assert!(dirty.is_empty(), "the writer must not patch it unread");
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].path, listed[0]);
        assert!(problems[0].message.starts_with("a.ARW.dop: "));
        index_files(&index, &dir, &listed);
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(5));

        remove_temp_dir(&root);
    }

    #[test]
    fn an_unparseable_sidecar_is_reported_on_every_open_and_leaves_the_rating() {
        let root = temp_dir("sidecar-unparseable");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();
        index::lock(&index)
            .set_rating(&dir, &listed[0], Some(2), Flag::None, None, true)
            .unwrap();
        let xmp = root.join("a.xmp");
        std::fs::write(
            &xmp,
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="4"#,
        )
        .unwrap();
        for _ in 0..2 {
            let (_, problems, _) =
                reconcile_listed_with_errors(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
            assert_eq!(problems.len(), 1);
            assert_eq!(problems[0].path, listed[0]);
            assert!(problems[0].message.starts_with("a.xmp: "));
        }
        index_files(&index, &dir, &listed);
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(2));

        remove_temp_dir(&root);
    }

    #[test]
    fn scan_started_serializes_the_fields_the_frontend_reads() {
        let started = ScanStarted {
            total: 3,
            scan_id: 7,
            sidecar_errors: vec![SidecarError {
                path: "/d/a.xmp".into(),
                message: "bad".into(),
            }],
            changed: 2,
        };
        assert_eq!(
            serde_json::to_value(&started).unwrap(),
            serde_json::json!({
                "total": 3,
                "scan_id": 7,
                "sidecar_errors": [{ "path": "/d/a.xmp", "message": "bad" }],
                "changed": 2,
            })
        );
    }

    #[test]
    fn after_a_format_switch_only_the_selected_formats_ratings_are_read() {
        let root = temp_dir("format-switch");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        std::fs::write(root.join("b.ARW"), b"x").unwrap();
        sidecar(&root, "a.xmp", 4);
        std::fs::write(root.join("b.ARW.dop"), PHOTOLAB_THREE).unwrap();
        let index = sidecar_index(&root);
        let listed = list_arw_in(&root).unwrap();
        reconcile_listed(&dir, &listed, &index, SidecarFormat::Xmp).unwrap();
        index_files(&index, &dir, &listed);
        assert_eq!(rating_of(&index, &dir, &listed[0]), Some(4));
        assert_eq!(rating_of(&index, &dir, &listed[1]), None);

        index::lock(&index).reset_sidecars().unwrap();
        let dirty = reconcile_listed(&dir, &listed, &index, SidecarFormat::Dop).unwrap();

        assert!(dirty.is_empty());
        assert_eq!(rating_of(&index, &dir, &listed[0]), None);
        assert_eq!(rating_of(&index, &dir, &listed[1]), Some(3));

        remove_temp_dir(&root);
    }

    #[test]
    fn an_unknown_sort_order_falls_back_to_name() {
        assert_eq!(parse_sort_order(Some("capture")), "capture");
        assert_eq!(parse_sort_order(Some("rating")), "rating");
        assert_eq!(parse_sort_order(Some("recommended")), "recommended");
        assert_eq!(parse_sort_order(Some("size")), "name");
        assert_eq!(parse_sort_order(None), "name");
    }

    #[test]
    fn the_legacy_last_folder_file_is_taken_once() {
        let root = temp_dir("legacy-last-folder");
        let file = root.join("last_folder");
        assert_eq!(take_legacy_last_folder(&file), None);
        std::fs::write(&file, "/some/folder").unwrap();

        assert_eq!(
            take_legacy_last_folder(&file).as_deref(),
            Some("/some/folder")
        );
        assert!(!file.exists());
        assert_eq!(take_legacy_last_folder(&file), None);

        remove_temp_dir(&root);
    }

    #[test]
    fn lists_only_arw_files_sorted_by_name() {
        let dir = temp_dir("list");
        for name in [
            "b.ARW",
            "a.arw",
            "c.Arw",
            "d.jpg",
            "e.arw.txt",
            "f.DNG",
            "g.dng",
            "f.dop",
        ] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        std::fs::create_dir(dir.join("sub.arw")).unwrap();

        let files = list_arw_in(&dir).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| {
                Path::new(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, ["a.arw", "b.ARW", "c.Arw", "f.DNG", "g.dng"]);
        assert!(files.iter().all(|p| Path::new(p).is_absolute()));

        remove_temp_dir(&dir);
    }

    #[test]
    fn one_listing_finds_the_raw_files_and_the_sidecars_of_each_format() {
        let dir = temp_dir("list-folder");
        for name in [
            "b.ARW",
            "a.arw",
            "c.DNG",
            "a.xmp",
            "B.XMP",
            "a.arw.dop",
            "C.DNG.DOP",
            "d.jpg",
        ] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        std::fs::create_dir(dir.join("sub.arw")).unwrap();

        for format in [SidecarFormat::Xmp, SidecarFormat::Dop, SidecarFormat::Both] {
            let (files, sidecars) = list_folder_in(&dir, format).unwrap();
            assert_eq!(files, list_arw_in(&dir).unwrap());

            let expected: HashMap<String, SidecarStat> = std::fs::read_dir(&dir)
                .unwrap()
                .flatten()
                .filter(|e| format.matches(&e.file_name().to_string_lossy()))
                .map(|e| {
                    let stat = index::stat(&e.path()).unwrap();
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    (name, (stat.path, stat.size, stat.mtime_ns))
                })
                .collect();
            let mut names: Vec<&String> = sidecars.keys().collect();
            names.sort();
            match format {
                SidecarFormat::Xmp => assert_eq!(names, ["a.xmp", "b.xmp"]),
                SidecarFormat::Dop => assert_eq!(names, ["a.arw.dop", "c.dng.dop"]),
                SidecarFormat::Both => {
                    assert_eq!(names, ["a.arw.dop", "a.xmp", "b.xmp", "c.dng.dop"])
                }
            }
            assert_eq!(sidecars, expected);
        }

        remove_temp_dir(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_raw_file_is_listed() {
        let dir = temp_dir("list-symlinks");
        let target = temp_dir("list-symlinks-target").join("photo.ARW");
        std::fs::write(&target, b"x").unwrap();
        std::os::unix::fs::symlink(&target, dir.join("linked.ARW")).unwrap();
        std::os::unix::fs::symlink(dir.join("gone.ARW"), dir.join("broken.ARW")).unwrap();

        let files = list_arw_in(&dir).unwrap();
        assert_eq!(files, [dir.join("linked.ARW").to_string_lossy()]);

        remove_temp_dir(&dir);
        remove_temp_dir(target.parent().unwrap());
    }

    #[test]
    fn unreadable_directory_is_an_error() {
        let missing = std::env::temp_dir().join("riffle-app-does-not-exist");
        assert!(list_arw_in(&missing).is_err());
    }

    fn names_of(files: &[String]) -> Vec<String> {
        files
            .iter()
            .map(|p| {
                Path::new(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn a_folder_without_raw_files_lists_its_jpegs() {
        let dir = temp_dir("list-jpegs");
        for name in ["b.JPEG", "a.jpg", "c.Jpg", "d.png", "e.txt", "a.xmp"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        std::fs::create_dir(dir.join("sub.jpg")).unwrap();
        assert_eq!(
            names_of(&list_arw_in(&dir).unwrap()),
            ["a.jpg", "b.JPEG", "c.Jpg"]
        );

        std::fs::write(dir.join("f.ARW"), b"x").unwrap();
        assert_eq!(names_of(&list_arw_in(&dir).unwrap()), ["f.ARW"]);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_jpeg_listing_reads_no_sidecar_and_queues_nothing() {
        let root = temp_dir("jpeg-sidecars");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("foo.jpg"), b"x").unwrap();
        sidecar(&root, "foo.xmp", 4);
        std::fs::write(root.join("foo.jpg.dop"), b"x").unwrap();
        let index = sidecar_index(&root);

        let listed = list_arw_in(&root).unwrap();
        assert_eq!(names_of(&listed), ["foo.jpg"]);
        for format in [SidecarFormat::Xmp, SidecarFormat::Dop, SidecarFormat::Both] {
            let (_, sidecars) = list_folder_in(&root, format).unwrap();
            assert!(sidecars.is_empty(), "{format:?}");
            let (dirty, errors, _) =
                reconcile_listed_with_errors(&dir, &listed, &index, format).unwrap();
            assert!(dirty.is_empty() && errors.is_empty(), "{format:?}");
        }
        index::lock(&index)
            .write_batch(
                &dir,
                &[(index::stat(Path::new(&listed[0])).unwrap(), Err("x".into()))],
            )
            .unwrap();
        let entries = index::lock(&index).entries(&dir).unwrap();
        assert_eq!(entries[0].rating, None);
        assert!(!entries[0].has_sidecar);

        remove_temp_dir(&root);
    }

    #[test]
    fn culling_and_face_detection_refuse_a_jpeg() {
        let dir = temp_dir("jpeg-refused");
        let path = dir.join("a.jpg");
        std::fs::write(
            &path,
            crate::index::tests::jpeg_with_exif(64, 48, 1, "2026:09:27 10:00:00", 400),
        )
        .unwrap();

        assert!(raw_only(&path, "culling").is_err());
        assert!(raw_only(Path::new("a.ARW"), "culling").is_ok());
        assert!(raw_only(Path::new("a.dng"), "culling").is_ok());
        let err = read_faces(&path).unwrap_err();
        assert!(err.contains("RAW files only"), "{err}");

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_jpeg_is_served_whole_with_a_neutral_orientation_and_a_centered_crop() {
        use crate::index::tests::jpeg_with_exif;
        let dir = temp_dir("jpeg-serve");
        let path = dir.join("a.jpg");
        let bytes = jpeg_with_exif(64, 48, 6, "2026:09:27 10:00:00", 400);
        std::fs::write(&path, &bytes).unwrap();

        let neutral = jpeg_with_exif(64, 48, 1, "2026:09:27 10:00:00", 400);
        assert_eq!(read_preview(&path).unwrap(), (6, neutral));

        let meta = read_metadata(&path).unwrap();
        assert_eq!(meta.name, "a.jpg");
        assert_eq!(meta.captured_at.as_deref(), Some("2026:09:27 10:00:00"));
        assert!(meta.iso.is_some());
        for maker_note in [
            &meta.shutter_type,
            &meta.focus_mode,
            &meta.af_area,
            &meta.af_tracking,
            &meta.drive,
            &meta.stabilization,
            &meta.exposure_mode,
            &meta.metering,
            &meta.creative_style,
            &meta.dro,
            &meta.raw_type,
            &meta.focus_distance,
        ] {
            assert_eq!(*maker_note, None);
        }

        let (orientation, crop, _) = read_focus_crop(&path, 16, 16).unwrap();
        assert_eq!(orientation, 6);
        assert_eq!(
            (crop.crop.x + crop.point_x, crop.crop.y + crop.point_y),
            (32, 24)
        );

        remove_temp_dir(&dir);
    }

    fn cached_listing(
        dir: &str,
        format: SidecarFormat,
        mtime: std::time::SystemTime,
    ) -> CachedListing {
        CachedListing {
            dir: dir.to_string(),
            format,
            mtime,
            listing: Listing {
                files: vec![format!("{dir}/a.ARW")],
                sidecars: vec![("a.xmp".to_string(), PathBuf::from(format!("{dir}/a.xmp")))],
            },
        }
    }

    #[test]
    fn a_matching_listing_is_reused_once() {
        let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(100);
        let mut cached = Some(cached_listing("/photos", SidecarFormat::Xmp, mtime));
        let expected = cached_listing("/photos", SidecarFormat::Xmp, mtime).listing;

        let taken = take_listing(&mut cached, "/photos", SidecarFormat::Xmp, Some(mtime));
        assert_eq!(taken, Some(expected));
        assert!(cached.is_none());
        assert_eq!(
            take_listing(&mut cached, "/photos", SidecarFormat::Xmp, Some(mtime)),
            None
        );
    }

    #[test]
    fn a_listing_of_another_dir_format_or_mtime_is_not_reused() {
        let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(100);
        let later = mtime + std::time::Duration::from_secs(1);
        let mut cached = Some(cached_listing("/photos", SidecarFormat::Xmp, mtime));

        for (dir, format, at) in [
            ("/other", SidecarFormat::Xmp, Some(mtime)),
            ("/photos", SidecarFormat::Dop, Some(mtime)),
            ("/photos", SidecarFormat::Both, Some(mtime)),
            ("/photos", SidecarFormat::Xmp, Some(later)),
            ("/photos", SidecarFormat::Xmp, None),
        ] {
            assert_eq!(take_listing(&mut cached, dir, format, at), None);
            assert!(cached.is_some());
        }
        assert!(take_listing(&mut cached, "/photos", SidecarFormat::Xmp, Some(mtime)).is_some());
    }

    #[test]
    fn nothing_cached_is_not_reused() {
        let mut cached = None;
        let mtime = Some(std::time::UNIX_EPOCH);
        assert_eq!(
            take_listing(&mut cached, "/photos", SidecarFormat::Xmp, mtime),
            None
        );
    }

    #[test]
    fn payload_header_encodes_kind_and_orientation() {
        let jpeg = [0xff, 0xd8, 0xff, 0xd9];
        let out = payload(PREVIEW_KIND_JPEG_V1, 8, &jpeg);
        assert_eq!(out.len(), PREVIEW_HEADER_LEN + jpeg.len());
        assert_eq!(u16::from_le_bytes([out[0], out[1]]), PREVIEW_KIND_JPEG_V1);
        assert_eq!(u16::from_le_bytes([out[2], out[3]]), 8);
        assert_eq!(u32::from_le_bytes([out[4], out[5], out[6], out[7]]), 0);
        assert_eq!(&out[PREVIEW_HEADER_LEN..], &jpeg);
    }

    #[test]
    fn thumbnail_payload_header_encodes_kind_two_and_orientation() {
        let jpeg = [0xff, 0xd8, 0xff, 0xd9];
        let out = payload(THUMBNAIL_KIND_JPEG_V1, 6, &jpeg);
        assert_eq!(u16::from_le_bytes([out[0], out[1]]), THUMBNAIL_KIND_JPEG_V1);
        assert_eq!(u16::from_le_bytes([out[2], out[3]]), 6);
        assert_eq!(&out[PREVIEW_HEADER_LEN..], &jpeg);
    }

    #[test]
    fn dropped_directory_is_taken_as_is_and_a_file_by_its_parent() {
        let dir = temp_dir("dropped");
        let file = dir.join("a.arw");
        std::fs::write(&file, b"x").unwrap();

        assert_eq!(dropped_dir(&dir), Some(dir.clone()));
        assert_eq!(dropped_dir(&file), Some(dir.clone()));
        assert_eq!(dropped_dir(&dir.join("gone.arw")), None);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_files_parent_at_the_filesystem_root_is_the_root_not_none() {
        // The boundary the `dropped_dir` doc comment describes: `parent()`
        // only yields `None` for the root itself, so a file directly under
        // it resolves to the root. There is no writable file directly under
        // `/` to exercise `dropped_dir` itself against, so this pins the
        // `Path::parent` behavior the function relies on instead.
        assert_eq!(Path::new("/a.arw").parent(), Some(Path::new("/")));
    }

    /// Encode a synthetic gradient so the test needs no image file.
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

    /// A TIFF whose IFD0 holds a tiny preview and whose IFD1 points at the
    /// JpgFromRaw, mirroring the builder in `riffle_core::reader`'s tests.
    fn arw_with_full(jpeg: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"II\x2a\x00");
        buf.extend_from_slice(&8u32.to_le_bytes());
        let ifd1_at = 8 + 2 + 2 * 12 + 4;
        let at = 256;
        for (entries, next) in [
            ([(0x0201u16, 8u32), (0x0202, 1)], ifd1_at as u32),
            ([(0x0201, at as u32), (0x0202, jpeg.len() as u32)], 0),
        ] {
            buf.extend_from_slice(&2u16.to_le_bytes());
            for (tag, value) in entries {
                buf.extend_from_slice(&tag.to_le_bytes());
                buf.extend_from_slice(&4u16.to_le_bytes());
                buf.extend_from_slice(&1u32.to_le_bytes());
                buf.extend_from_slice(&value.to_le_bytes());
            }
            buf.extend_from_slice(&next.to_le_bytes());
        }
        buf.resize(at, 0);
        buf.extend_from_slice(jpeg);
        buf
    }

    #[test]
    fn crop_payload_header_encodes_every_field() {
        let crop = riffle_core::partial::FocusCrop {
            crop: riffle_core::partial::Crop {
                pixels: vec![1, 2, 3, 4, 5, 6, 7, 8],
                width: 2,
                height: 1,
                x: 16,
                y: 32,
                image_width: 6000,
                image_height: 4000,
            },
            point_x: 7,
            point_y: 9,
        };
        let out = crop_payload(
            8,
            &crop,
            CropTiming {
                read_us: 731,
                decode_us: 21_500,
            },
        );
        assert_eq!(out.len(), CROP_HEADER_LEN + crop.crop.pixels.len());
        let u16at = |at: usize| u16::from_le_bytes([out[at], out[at + 1]]);
        let u32at =
            |at: usize| u32::from_le_bytes([out[at], out[at + 1], out[at + 2], out[at + 3]]);
        assert_eq!(u16at(0), CROP_KIND_RGBA_V3);
        assert_eq!(u16at(2), 8);
        assert_eq!(u32at(4), 2);
        assert_eq!(u32at(8), 1);
        assert_eq!(u32at(12), 7);
        assert_eq!(u32at(16), 9);
        assert_eq!(u32at(20), 731);
        assert_eq!(u32at(24), 21_500);
        assert_eq!(u16at(28), 6000);
        assert_eq!(u16at(30), 4000);
        assert_eq!(&out[CROP_HEADER_LEN..], &crop.crop.pixels);
    }

    #[test]
    fn a_quarter_turn_swaps_the_viewport_and_the_cap_bounds_each_axis() {
        assert_eq!(crop_size(1, 300, 200), (300, 200));
        assert_eq!(crop_size(6, 300, 200), (200, 300));
        assert_eq!(crop_size(8, 300, 200), (200, 300));
        assert_eq!(crop_size(8, 4000, 2000), (CROP_MAX, CROP_MAX));
        assert_eq!(crop_size(1, 0, 0), (1, 1));
    }

    #[test]
    fn a_synthetic_arw_crops_around_the_center_without_a_focus_point() {
        let dir = temp_dir("crop");
        let path = dir.join("a.arw");
        std::fs::write(&path, arw_with_full(&gradient_jpeg(400, 300))).unwrap();

        let (orientation, crop, _) = read_focus_crop(&path, 64, 48).unwrap();
        assert_eq!(orientation, 1);
        // The crop snaps to an MCU boundary horizontally, so it can come back
        // wider than asked for, never narrower.
        assert!((64..64 + 16).contains(&crop.crop.width));
        assert_eq!(crop.crop.height, 48);
        assert_eq!(crop.crop.pixels.len(), crop.crop.width * 48 * 4);
        // Center of a 400x300 image, minus the crop's own snapped origin.
        assert_eq!(crop.point_x, 200 - crop.crop.x);
        assert_eq!(crop.crop.x % 16, 0);
        assert_eq!(crop.point_y, 24);
        assert_eq!((crop.crop.image_width, crop.crop.image_height), (400, 300));

        remove_temp_dir(&dir);
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

    #[test]
    fn missing_preview_is_an_error() {
        let dir = temp_dir("preview");
        let path = dir.join("broken.arw");
        std::fs::write(&path, b"II\x2a\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00").unwrap();
        assert!(read_preview(&path).is_err());
        remove_temp_dir(&dir);
    }

    /// A writer whose errors are collected, so `drain` can fail with the
    /// writer's own message instead of a later read's `NotFound`.
    fn switch_writer(index: Arc<Mutex<Index>>) -> (Writer, Arc<Mutex<Vec<String>>>) {
        let errors = Arc::new(Mutex::new(Vec::new()));
        let collected = errors.clone();
        let writer = Writer::spawn(index, move |path, message| {
            index::lock(&collected).push(format!("{}: {message}", path.display()))
        });
        (writer, errors)
    }

    /// Wait for the writer to finish everything queued. The budget is far
    /// longer than `DRAIN_TIMEOUT` because it only costs time when the test
    /// is failing anyway, and a loaded Windows runner can take seconds.
    fn drain(writer: &Writer, errors: &Mutex<Vec<String>>) {
        assert!(
            writer.flush(Duration::from_secs(30)),
            "the sidecar writer did not drain; reported so far: {:?}",
            *index::lock(errors)
        );
        let all = index::lock(errors).clone();
        // A `(retrying in ..)` message is transient: the entry was requeued
        // and the flush rewrote it. Only errors not followed by a retry are
        // final, and those fail the test; `all` stays in the message so a
        // transient sharing violation is still visible in the log.
        let last: Vec<&String> = all
            .iter()
            .filter(|m| !m.contains(" (retrying in "))
            .collect();
        assert!(
            last.is_empty(),
            "final sidecar write errors; all reported: {all:?}"
        );
    }

    /// A judgment made in the window between the format swap and
    /// `reset_sidecars` must land in the newly selected format and leave the
    /// row clean, not be replayed on the next open.
    #[test]
    fn a_rating_set_during_a_switch_lands_in_the_new_format_and_leaves_no_dirty_row() {
        let root = temp_dir("switch-race-rating");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        let index = sidecar_index(&root);
        let (writer, errors) = switch_writer(index.clone());
        let listed = list_arw_in(&root).unwrap();
        let path = listed[0].clone();
        let current = Mutex::new(SidecarFormat::Xmp);
        let observed = Mutex::new(None);

        switch_format(
            &current,
            Some(&writer),
            Some(&index),
            SidecarFormat::Dop,
            |_| {
                let format = *index::lock(&current);
                *index::lock(&observed) = Some(format);
                index::lock(&index)
                    .set_rating(&dir, &path, Some(4), Flag::None, None, true)
                    .unwrap();
                writer
                    .set(
                        PathBuf::from(&path),
                        Some(4),
                        Flag::None,
                        None,
                        true,
                        format,
                        LabelNames::default(),
                    )
                    .unwrap();
                Ok(())
            },
        )
        .unwrap();
        drain(&writer, &errors);

        assert_eq!(*index::lock(&observed), Some(SidecarFormat::Dop));
        let dop = SidecarFormat::Dop.sidecar_path(Path::new(&path));
        let bytes = std::fs::read(&dop).unwrap();
        assert_eq!(SidecarFormat::Dop.read_rating(&bytes).unwrap(), Some(4));
        assert!(!SidecarFormat::Xmp.sidecar_path(Path::new(&path)).exists());
        // `entries` joins `files`, which only a scan fills in.
        index::lock(&index)
            .write_batch(
                &dir,
                &[(index::stat(Path::new(&path)).unwrap(), Err("x".into()))],
            )
            .unwrap();
        assert_eq!(rating_of(&index, &dir, &path), Some(4));
        assert!(index::lock(&index).dirty_rows(&dir).unwrap().is_empty());

        remove_temp_dir(&root);
    }

    /// The same race with a pick: `reset_sidecars` must not zero the flag of
    /// the row the writer is about to confirm, or `mark_written`'s `flag`
    /// guard misses it and the next open replays it over the `.dop`.
    #[test]
    fn a_pick_set_during_a_switch_lands_in_the_dop_and_leaves_no_dirty_row() {
        let root = temp_dir("switch-race-pick");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        let index = sidecar_index(&root);
        let (writer, errors) = switch_writer(index.clone());
        let listed = list_arw_in(&root).unwrap();
        let path = listed[0].clone();
        let current = Mutex::new(SidecarFormat::Xmp);

        switch_format(
            &current,
            Some(&writer),
            Some(&index),
            SidecarFormat::Dop,
            |_| {
                let format = *index::lock(&current);
                index::lock(&index)
                    .set_rating(&dir, &path, Some(4), Flag::Pick, None, true)
                    .unwrap();
                writer
                    .set(
                        PathBuf::from(&path),
                        Some(4),
                        Flag::Pick,
                        None,
                        true,
                        format,
                        LabelNames::default(),
                    )
                    .unwrap();
                Ok(())
            },
        )
        .unwrap();
        drain(&writer, &errors);

        let dop = SidecarFormat::Dop.sidecar_path(Path::new(&path));
        let bytes = std::fs::read(&dop).unwrap();
        assert_eq!(SidecarFormat::Dop.read_rating(&bytes).unwrap(), Some(4));
        assert_eq!(SidecarFormat::Dop.read_flag(&bytes).unwrap(), Flag::Pick);
        assert!(!SidecarFormat::Xmp.sidecar_path(Path::new(&path)).exists());
        // `entries` joins `files`, which only a scan fills in.
        index::lock(&index)
            .write_batch(
                &dir,
                &[(index::stat(Path::new(&path)).unwrap(), Err("x".into()))],
            )
            .unwrap();
        let entry = index::lock(&index)
            .entries(&dir)
            .unwrap()
            .into_iter()
            .find(|e| e.path == path)
            .unwrap();
        assert_eq!((entry.rating, entry.flag), (Some(4), Flag::Pick));
        assert!(index::lock(&index).dirty_rows(&dir).unwrap().is_empty());

        remove_temp_dir(&root);
    }

    /// Switching to `Both` writes a row left dirty in the old format into
    /// both sidecars, and stores the stat the next open compares against.
    #[test]
    fn a_dirty_row_is_written_to_both_sidecars_after_a_switch_to_both() {
        let root = temp_dir("switch-both");
        let dir = root.to_string_lossy().into_owned();
        std::fs::write(root.join("a.ARW"), b"x").unwrap();
        let index = sidecar_index(&root);
        let (writer, errors) = switch_writer(index.clone());
        let listed = list_arw_in(&root).unwrap();
        let path = listed[0].clone();
        index::lock(&index)
            .set_rating(&dir, &path, Some(4), Flag::Pick, None, true)
            .unwrap();
        let current = Mutex::new(SidecarFormat::Xmp);

        switch_format(
            &current,
            Some(&writer),
            Some(&index),
            SidecarFormat::Both,
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(*index::lock(&current), SidecarFormat::Both);
        let dirty = reconcile_listed(&dir, &listed, &index, SidecarFormat::Both).unwrap();
        assert_eq!(dirty.len(), 1);
        for (path, rating, flag, label, label_known) in dirty {
            writer
                .set_now(
                    PathBuf::from(path),
                    rating,
                    flag,
                    label,
                    label_known,
                    SidecarFormat::Both,
                    LabelNames::default(),
                )
                .unwrap();
        }
        drain(&writer, &errors);

        for kind in [SidecarFormat::Xmp, SidecarFormat::Dop] {
            let bytes = std::fs::read(kind.sidecar_path(Path::new(&path))).unwrap();
            assert_eq!(kind.read_rating(&bytes).unwrap(), Some(4));
            assert_eq!(kind.read_flag(&bytes).unwrap(), Flag::Pick);
        }
        assert!(index::lock(&index).dirty_rows(&dir).unwrap().is_empty());
        let (_, sidecars) = list_folder_in(&root, SidecarFormat::Both).unwrap();
        let newest = crate::sidecar::newest(
            ["a.xmp", "a.arw.dop"]
                .into_iter()
                .map(|name| (&sidecars[name], sidecars[name].2)),
        )
        .unwrap();
        let stored: (i64, i64) = index::lock(&index)
            .conn
            .query_row(
                "SELECT xmp_size, xmp_mtime_ns FROM ratings WHERE path = ?1",
                [&path],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored, (newest.1, newest.2), "a reopen re-parses nothing");

        remove_temp_dir(&root);
    }

    #[test]
    fn a_format_counts_as_saved_only_when_the_key_exists_or_the_store_is_broken() {
        assert!(format_saved(Ok(true)));
        assert!(!format_saved(Ok(false)));
        assert!(format_saved(Err("no config dir".into())));
    }

    #[test]
    fn choosing_the_current_format_still_persists_it() {
        let current = Mutex::new(SidecarFormat::Xmp);
        let saved = Mutex::new(Vec::new());

        choose_format(&current, None, None, SidecarFormat::Xmp, |format| {
            index::lock(&saved).push(format);
            Ok(())
        })
        .unwrap();

        assert_eq!(*index::lock(&saved), vec![SidecarFormat::Xmp]);
        assert_eq!(*index::lock(&current), SidecarFormat::Xmp);
    }

    #[test]
    fn choosing_another_format_switches_and_persists_it_once() {
        let current = Mutex::new(SidecarFormat::Xmp);
        let saved = Mutex::new(Vec::new());

        choose_format(&current, None, None, SidecarFormat::Dop, |format| {
            index::lock(&saved).push(format);
            Ok(())
        })
        .unwrap();

        assert_eq!(*index::lock(&saved), vec![SidecarFormat::Dop]);
        assert_eq!(*index::lock(&current), SidecarFormat::Dop);
    }

    #[test]
    fn a_failed_save_of_the_chosen_format_is_returned() {
        let current = Mutex::new(SidecarFormat::Xmp);

        let result = choose_format(&current, None, None, SidecarFormat::Xmp, |_| {
            Err("read-only".into())
        });

        assert_eq!(result, Err("read-only".to_string()));
    }
}
