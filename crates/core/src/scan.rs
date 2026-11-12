//! Extract one folder's worth of metadata and thumbnails in parallel.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use crate::arw::{Arw, FocusLocation, Shot};
use crate::candidate::{face_cue_unless, focus_cue_unless, Cue, FocusCandidate};
use crate::decode::{decode_rgb, thumbnail_jpeg, thumbnail_jpeg_near};
use crate::embed::{embed_preview, Model};
use crate::eyes::judged_face;
use crate::faces::{detect_around, Detection, Face};
use crate::fullsharp::full_eye_sharpness;
use crate::reader::{read_full, read_metadata, read_preview};
use crate::sharpness::{eye_af_frame, score_preview, trusted_focus};

/// Quality of the cached thumbnails. 80 gives ~19KB for a 404x270 frame.
pub const THUMBNAIL_QUALITY: f32 = 80.0;

/// The long edge a JPEG file's or a RAF's thumbnail aims at: the ARW
/// thumbnail's.
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
        read_preview(path)
    };
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
    let jpeg = is_jpeg_file(path);
    // A RAF's one embedded JPEG is 4000 to 4416 px wide, which 2/8 would
    // leave at over 1000.
    let near = jpeg
        || path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("raf"));
    // mozjpeg aborts through a panic, not an error return, when the bytes are
    // not a JPEG. A single corrupt file out of thousands must cost that file
    // and nothing else, so the decode runs inside `catch_unwind`. Nothing here
    // is shared or observable after a panic, hence `AssertUnwindSafe`.
    let thumbnail = match catch_unwind(AssertUnwindSafe(|| {
        if near {
            thumbnail_jpeg_near(&preview, JPEG_THUMBNAIL_LONG_EDGE, THUMBNAIL_QUALITY)
        } else {
            thumbnail_jpeg(&preview, THUMBNAIL_QUALITY)
        }
    })) {
        Ok(Ok(thumbnail)) => thumbnail,
        Ok(Err(e)) => return Some(Err(failed(e.to_string()))),
        Err(_) => {
            return Some(Err(failed(format!(
                "panic while encoding the thumbnail of {}",
                path.display()
            ))))
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
    /// `sharpness::score_preview` of the preview; `None` when it could not be
    /// scored, which does not fail the file, and always for a JPEG file.
    pub sharpness: Option<f64>,
    /// `fullsharp::full_eye_sharpness` of the judged face's eyes on the full
    /// JPEG, measured when the cue is `Candidate`; `None` when it was not
    /// measured or the file has no usable larger JPEG, or on any failure.
    pub full_eye_sharp: Option<f64>,
}

/// Compute the focus candidate cue and the sharpness score of one file from
/// one read of its preview. The cue runs the face detector and, on the face
/// nearest the AF point, the face mesh (`candidate::focus_cue_unless`);
/// without a trusted AF point, on the largest confident face of the whole
/// preview (`eyes::judged_face`, `candidate::face_cue_unless`). Only
/// an unreadable file is `Err`; a decode, detection or scoring failure, or a
/// panic in any of them, is an unknown cue or no score; a mesh failure falls
/// back to the eye window. A candidate's eyes are measured once more on the
/// full JPEG, a second read (`full_eye_sharp`). A JPEG file gets none of
/// these, and is not read.
pub fn extract_analysis(path: &Path) -> Result<Analysis, String> {
    extract_analysis_unless(path, &AtomicBool::new(false), &ReadSlots::default())
        .expect("a never-set flag never abandons")
}

thread_local! {
    /// The read slots of the pass 2 file this thread is analyzing, which
    /// `yield_to_pause` waits on, with the instant its stage waits first saw a
    /// pause (one budget for the whole file); none outside
    /// `extract_analysis_unless`.
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

/// A stage boundary of pass 2: wait out a pause held on the slots of the file
/// this thread analyzes (`ReadSlots::wait_unpaused_since`). All of a file's
/// stage waits share one `MAX_PAUSE` budget. A no-op outside pass 2,
/// so `candidate`'s stages call it whoever runs them.
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
        read_preview(path)
    };
    let (arw, preview) = match read {
        Ok(read) => read,
        Err(e) => return Some(Err(e.to_string())),
    };
    yield_to_pause(cancel);
    if canceled(cancel) {
        return None;
    }
    let focus = trusted_focus(&arw.shot);
    // With an AF point the cue's detection runs around it and the score needs
    // no faces; without one the faces are searched for on the whole image,
    // the cue judges the largest confident one and the score falls back to
    // them.
    let (cue, faces) = match focus {
        Some(_) => (cue(&preview, arw.orientation, focus, cancel)?, Vec::new()),
        None => {
            let detection = whole_image_detection(&preview, arw.orientation);
            yield_to_pause(cancel);
            if canceled(cancel) {
                return None;
            }
            let faces = detection
                .as_ref()
                .map_or_else(Vec::new, |d| d.faces.clone());
            let cue = whole_image_cue(&preview, arw.orientation, detection, cancel)?;
            (cue, faces)
        }
    };
    if canceled(cancel) {
        return None;
    }
    yield_to_pause(cancel);
    let sharpness = score(&preview, &arw.shot, focus, &faces);
    yield_to_pause(cancel);
    let full_eye_sharp = full_eye_sharp(path, &arw, &cue, cancel, slots)?;
    Some(Ok(Analysis {
        cue,
        sharpness,
        full_eye_sharp,
    }))
}

/// Whether `arw` holds a JPEG larger than its preview to measure the eyes on:
/// not when it has no full image, an HEVC one, or only the preview itself.
fn has_larger_jpeg(arw: &Arw) -> bool {
    let same = match (arw.full, arw.preview) {
        (Some(full), Some(preview)) => full.offset == preview.offset,
        _ => false,
    };
    arw.full.is_some() && !arw.hevc && !same
}

/// The full-size sharpness of the eyes of a `Candidate` `cue`; `None` inside
/// when nothing was measured or anything failed, `None` outside when `cancel`
/// abandoned the file. The read goes through `slots`, the decode does not.
fn full_eye_sharp(
    path: &Path,
    arw: &Arw,
    cue: &Cue,
    cancel: &AtomicBool,
    slots: &ReadSlots,
) -> Option<Option<f64>> {
    let eyes = match cue.eye_centers {
        Some(eyes) if cue.state == FocusCandidate::Candidate && has_larger_jpeg(arw) => eyes,
        _ => return Some(None),
    };
    let read = {
        let _slot = slots.acquire(cancel)?;
        read_full(path)
    };
    yield_to_pause(cancel);
    if canceled(cancel) {
        return None;
    }
    let Ok((_, full)) = read else {
        return Some(None);
    };
    Some(
        catch_unwind(AssertUnwindSafe(|| {
            full_eye_sharpness(&full, eyes, cue.preview_width)
        }))
        .ok()
        .flatten(),
    )
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
fn whole_image_cue(
    preview: &[u8],
    orientation: u16,
    detection: Option<Detection>,
    cancel: &AtomicBool,
) -> Option<Cue> {
    let Some(detection) = detection else {
        return Some(Cue::unknown());
    };
    let Some(face) = judged_face(&detection.faces, None).copied() else {
        return Some(Cue::unknown());
    };
    guarded(|| {
        let (rgb, width, height) = match decode_rgb(preview) {
            Ok(decoded) => decoded,
            Err(e) => return Some(Err(e)),
        };
        if canceled(cancel) {
            return None;
        }
        face_cue_unless(&rgb, width, height, orientation, &face, detection, cancel).map(Ok)
    })
}

/// `score_preview` of `preview`; a failure or panic is no score.
fn score(preview: &[u8], shot: &Shot, focus: Option<FocusLocation>, faces: &[Face]) -> Option<f64> {
    let eye_af = eye_af_frame(shot);
    catch_unwind(AssertUnwindSafe(|| {
        score_preview(preview, focus, eye_af.map(|(_, frame)| frame), faces)
    }))
    .ok()
    .and_then(Result::ok)
}

fn cue(
    preview: &[u8],
    orientation: u16,
    focus: Option<FocusLocation>,
    cancel: &AtomicBool,
) -> Option<Cue> {
    guarded(|| focus_cue_unless(preview, orientation, focus, cancel))
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
/// delivered. Callers that share state behind a mutex (as Step 4's Tauri
/// command does) must keep `on_item` panic-free or wrap it themselves.
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

/// Run `embed::embed_preview` of `model` over `paths` the way `extract_all`
/// runs `extract`: same pool, same `focus` order, same `cancel`, same
/// `threads == 0` error, the same priority report, and the same rule that
/// `on_item` must not panic. A file whose preview cannot be read, decoded or
/// embedded, or that panics, is an `Err`; a JPEG file is read like a RAW, so
/// the caller leaves it out.
pub fn embed_all<F>(
    paths: &[PathBuf],
    threads: usize,
    priority: Priority,
    focus: &ScanFocus,
    model: &Model,
    on_item: F,
    cancel: &AtomicBool,
) -> Result<Option<String>, String>
where
    F: Fn(usize, Result<Vec<f32>, String>) + Send + Sync,
{
    let per_file = |path: &Path, cancel: &AtomicBool, slots: &ReadSlots| {
        let read = {
            let _slot = slots.acquire(cancel)?;
            read_preview(path)
        };
        let (arw, preview) = match read {
            Ok(read) => read,
            Err(e) => return Some(Err(e.to_string())),
        };
        if canceled(cancel) {
            return None;
        }
        Some(
            match catch_unwind(AssertUnwindSafe(|| {
                embed_preview(model, &preview, arw.orientation)
            })) {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(e)) => Err(format!("{e:#}")),
                Err(_) => Err(format!("panic while embedding {}", path.display())),
            },
        )
    };
    for_each_path(paths, threads, priority, focus, per_file, on_item, cancel)
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
    /// any other process), macOS `QOS_CLASS_BACKGROUND`, Linux nice 10.
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

#[cfg(target_os = "macos")]
fn lower_current_priority(lowest: bool) -> Result<(), String> {
    use libc::qos_class_t::{QOS_CLASS_BACKGROUND, QOS_CLASS_UTILITY};
    let class = if lowest {
        QOS_CLASS_BACKGROUND
    } else {
        QOS_CLASS_UTILITY
    };
    // SAFETY: sets the calling thread's own QoS class; no pointers involved.
    match unsafe { libc::pthread_set_qos_class_self_np(class, 0) } {
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
    use crate::candidate::FocusCandidate;
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
        let dir = std::env::temp_dir().join(format!("riffle-scan-{name}-{}", std::process::id()));
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

    #[cfg(any(windows, target_os = "linux"))]
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
            |i, r| seen.lock().unwrap().push((i, r.unwrap().cue.state)),
            &AtomicBool::new(false),
        )
        .unwrap();
        let mut seen = seen.into_inner().unwrap();
        seen.sort_by_key(|&(i, _)| i);
        assert_eq!(
            seen,
            (0..200)
                .map(|i| (i, FocusCandidate::Unknown))
                .collect::<Vec<_>>(),
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
        for analysis in [false, true] {
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
                    let result = if analysis {
                        extract_analysis_unless(path, cancel, slots).map(|r| r.map(|_| ()))
                    } else {
                        extract_unless(path, cancel, slots)
                            .map(|r| r.map(|_| ()).map_err(|e| e.message))
                    };
                    assert!(started.elapsed() < Duration::from_secs(5));
                    result
                },
                |_, _| *delivered.lock().unwrap() += 1,
                &cancel,
            )
            .unwrap();
            assert_eq!(
                delivered.into_inner().unwrap(),
                0,
                "analysis pass: {analysis}"
            );
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
    fn a_cue_that_is_not_a_candidate_reads_nothing_and_a_missing_full_jpeg_is_none() {
        let dir = dir("full-eye-sharp");
        let arw = Arw {
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
        let candidate = Cue {
            state: FocusCandidate::Candidate,
            eye_centers: Some([(10.0, 10.0), (30.0, 10.0)]),
            preview_width: 100,
            ..Cue::default()
        };
        let not_candidate = Cue {
            state: FocusCandidate::NotCandidate,
            ..candidate.clone()
        };
        let missing = dir.join("missing.ARW");
        let (slots, set, unset) = (
            ReadSlots::default(),
            AtomicBool::new(true),
            AtomicBool::new(false),
        );
        assert_eq!(
            full_eye_sharp(&missing, &arw, &not_candidate, &set, &slots),
            Some(None),
            "nothing to read, so not even a cancel abandons the file"
        );
        assert_eq!(
            full_eye_sharp(&missing, &arw, &candidate, &unset, &slots),
            Some(None),
            "a failed read is no score, not an error"
        );
        assert_eq!(
            full_eye_sharp(&missing, &arw, &candidate, &set, &slots),
            None,
            "a cancel abandons the file at the slot"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Long enough for the pool to start and reach the gate.
    const SETTLE: Duration = Duration::from_millis(150);
    /// A pause no test outlasts.
    const LONG_PAUSE: Duration = Duration::from_secs(60);

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
                let scan = s.spawn(|| pass(false, &paths, &slots, &cancel, &seen));
                std::thread::sleep(SETTLE);
                let canceled = Instant::now();
                cancel.store(true, Ordering::Relaxed);
                scan.join().unwrap();
                let took = canceled.elapsed();
                assert!(took < Duration::from_secs(1), "took {took:?}");
                drop((paused, slot));
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
        assert_eq!(no_af.state, FocusCandidate::Unknown);
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
        assert_eq!(flat.state, FocusCandidate::Unknown);
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
        let found = whole(vec![small, large, shy]);
        let cue = whole_image_cue(&jpeg(64, 48), 1, found.clone(), &never).unwrap();
        assert_ne!(cue.state, FocusCandidate::Unknown);
        assert!(cue.eye_focus.is_some());
        assert_eq!(cue.face, Some(large));
        assert_eq!(cue.detection, found);

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
