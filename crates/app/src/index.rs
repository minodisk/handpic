//! The SQLite index of a folder's metadata and thumbnails, and the scan that
//! fills it.
//!
//! One database holds every folder ever opened; a row is valid for a file only
//! while the file's `size` and `mtime_ns` still match what was indexed and it
//! was written at the current `EXTRACTOR_VERSION`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant, UNIX_EPOCH};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};

use riffle_core::arw::{Rational, Shot};
use riffle_core::candidate::{candidate, FocusCandidate};
use riffle_core::embed::{Model, EMBED_VERSION};
use riffle_core::eyes::{EyeState, Eyes};
use riffle_core::pose::Pose;
use riffle_core::rank;
use riffle_core::scan::{
    embed_all, extract_all, extract_analysis_all, is_raw_file, Analysis, Entry, Failure, Priority,
    ScanFocus,
};
use riffle_core::sharpness::manual_focus;
use riffle_core::Flag;

use crate::exif::{exif, Exif};

/// Schema version stored in `PRAGMA user_version`. Bump it when the layout
/// below changes; a version `prepare` cannot migrate is dropped and rebuilt.
///
/// From v2 on the database is no longer purely derivable: a `ratings` row with
/// `dirty = 1` is a judgment that has not reached its sidecar yet, so
/// discarding the database loses it. A future bump must migrate `ratings` or
/// flush every dirty row to its sidecar first. v3 added `ratings.pick`, and a
/// v2 database is migrated in place with an `ALTER TABLE`. v4 added the
/// shooting settings to `files`; a v2 or v3 database has its `files` table
/// dropped and recreated, so every folder is rescanned once, and keeps
/// `ratings`. v5 added `ratings.label`, the color label (`NULL` = none); a
/// v2, v3 or v4 database gains it in place with an `ALTER TABLE`. v6 added
/// `ratings.label_known`, persisting whether `label` reflects a judgment the
/// app actually asserted (see `set_rating`); older databases gain it in
/// place, defaulting to `1` since every label they hold was asserted. v7
/// added `files.sharpness`, the preview's focus-window score; a v2 to v6
/// database has its `files` table dropped and recreated, so every folder is
/// rescanned once, and keeps `ratings`. v8 added `folders`, when each folder
/// was last opened, for `evict`; a v7 database gains it in place, seeded with
/// every indexed folder opened "now", and keeps its `files` rows. v9 changed
/// how `files.sharpness` is scored (the sharpest tile when there is no
/// trustworthy AF point); a v2 to v8 database has its `files` table dropped
/// and recreated, so every folder is rescanned once, and keeps `ratings` and
/// `folders`. v10 changed it again (the eyes when a face is found); a v2 to v9
/// database likewise drops and recreates `files` and keeps `ratings` and
/// `folders`. v11 replaced `ratings.pick` with `ratings.flag` (`0` none, `1`
/// pick, `2` reject) and took the reject out of `rating`, which now only
/// holds `0`-`5`; older databases are migrated in place (a `-1` rating
/// becomes a reject with no stars, a pick a pick) and keep their `files`.
/// v12 added `files.extractor`, the `EXTRACTOR_VERSION` a row was written at;
/// a v10 or v11 database gains it in place with an `ALTER TABLE`, defaulting
/// to `0` so every existing row is re-extracted once, and keeps `files`,
/// `ratings` and `folders`. v13 added `files.frame_w` / `files.frame_h`, the
/// Sony AF frame size (`NULL` when not recorded), and `files.manual_focus`; a
/// v10 to v12 database gains them in place with an `ALTER TABLE` and keeps
/// `files`, `ratings` and `folders`. v14 added `files.face_catch`, the
/// face-catch state (`0` unknown, `1` caught, `2` missed); a v10 to v13
/// database gains it in place with an `ALTER TABLE` and keeps `files`,
/// `ratings` and `folders`. v15 replaced `files.face_catch` with
/// `files.eye_sharpness`, the focus candidate cue's eye sharpness (`NULL` =
/// none), and `files.faces_extractor`, the `FACES_VERSION` it was computed
/// at (`0` = not yet); a v10 to v14 database gains them in place with an
/// `ALTER TABLE`, a v14 one also drops `face_catch`, and keeps `files`,
/// `ratings` and `folders`. v16 replaced `files.eye_sharpness` with
/// `files.eye_focus`, the AF eye's in-focus probability (`NULL` = none); a
/// v10 to v14 database gains it in place next to `faces_extractor`, a v15
/// one drops `eye_sharpness` and adds `eye_focus` (not a rename, so a stale
/// Laplacian variance is never read as a probability), and both keep
/// `files`, `ratings` and `folders`. v17 added `folders.last_viewed`, the
/// file that was current in the strip when the folder was last viewed
/// (`NULL` = none); a v8 to v16 database gains it in place with an
/// `ALTER TABLE` and keeps `files`, `ratings` and `folders`. v18 added
/// `files.eyes_ear`, the EAR of the more closed eye of the AF face, and
/// `files.pose_yaw` / `files.pose_pitch` / `files.pose_roll`, its head pose
/// in degrees (`NULL` = none); a v10 to v17 database gains them in place with
/// an `ALTER TABLE` and keeps `files`, `ratings` and `folders`. v19 added
/// `files.eye_offset`, how far the AF face's mesh eyes sit from its YuNet eye
/// landmarks, and `files.edge_gap`, how close the face and its mesh eye
/// regions come to the preview's edge (`NULL` = none); a v10 to v18 database
/// gains them in place with an `ALTER TABLE` and keeps `files`, `ratings` and
/// `folders`. v20 added `embeddings`, one image embedding per file (pass 3);
/// every older database gains the empty table and keeps the rest. v21 added
/// `folders.culling_finished`, a hand-set mark that the folder's culling was
/// finished. v22 added `views`, the files shown in the preview, which every
/// older database gains empty, and `folders.culling_manual`, whether the mark
/// was toggled by hand. v23 dropped both `folders` columns, the ranking
/// reading `views` per file instead; a v21 or v22 database drops them in
/// place with an `ALTER TABLE` and keeps the rest.
const SCHEMA_VERSION: i64 = 23;

/// The version of what `riffle_core::scan::extract` produces, stored on every
/// `files` row. Bump it on any change to that output: ARW/DNG/NEF/CR3/RAF/ORF parsing
/// or embedded JPEG tier selection (`crates/core/src/arw.rs`, `nef.rs`,
/// `cr3.rs`, `raf.rs`, `orf.rs`, the shared Exif fields in `exif.rs`), which preview
/// bytes are read (`crates/core/src/reader.rs`), thumbnail generation
/// (`crates/core/src/decode.rs`, `scan.rs`). The first pass owns the thumbnail
/// and the metadata only; the sharpness score and the face search are the
/// second pass's, under `FACES_VERSION`. A bump re-extracts
/// every row, error rows included, on the next scan of each folder, and keeps
/// `ratings`. It starts at `1` so rows from before the column existed (`0`)
/// are stale; `2` fills in the AF frame size and the manual-focus flag; `3`
/// scores the AF window instead of a detected face's eyes when the AF point
/// lies outside the face; `4` fills in the face-catch state. It stayed `4`
/// when the face-catch state went and `extract` stopped detecting faces
/// around a trusted AF point: the score already ignored those faces, so
/// what `extract` stores did not change. `5` reads the Sony AF area, drive,
/// stabilization and picture settings; `6` stops reading `FocusMode` 0 as
/// manual focus on the `DSC-` bodies whose `FocusMode` always reads 0; `7`
/// reads the AF point of NEF (Nikon `AFInfo2`) and CR3 (Canon `AFInfo2`)
/// files; `8` names an HDR PQ (HEIF) CR3 in the error row; `9` reads the AF
/// point of RAF files (Fujifilm `FocusPixel`); `10` decodes the HEVC `PRVW` /
/// `THMB` of HDR PQ CR3 files; `11` reads the preview of the old `OLYMP\0`
/// Olympus MakerNote. It stayed `11` when the sharpness score moved to the
/// second pass: the score's computation did not change, so a row the first
/// pass scored keeps a correct score. `12` reads big-endian (`MM`) TIFF
/// containers, so the DNGs that failed with `not a little-endian TIFF/ARW`
/// get re-extracted. `13` keeps the metadata of a file whose extraction
/// failed in its error row, so existing error rows are re-extracted.
const EXTRACTOR_VERSION: i64 = 13;

/// The version of what the second pass, `riffle_core::scan::extract_analysis`,
/// produces, stored on every `files` row as `faces_extractor` next to
/// `eye_focus` and `sharpness`: the focus candidate cue and the sharpness
/// score. Bump it on any change to the cue's computation (the threshold, the
/// mesh eye regions, the eye window, the face mesh, the detector:
/// `crates/core/src/candidate.rs`, `eyes.rs`, `faces.rs`) or to
/// the score's (`sharpness::score_preview`, or what `extract_analysis` feeds
/// it); the second pass then re-runs on every row without redoing the first. `2`
/// re-runs pass 2 after `Cue::eye_focus` switched from the Laplacian
/// variance to the combined in-focus probability, so `eye_sharpness` holds
/// probabilities everywhere instead of a mix of old and new scales. `3`
/// re-runs it after the probability moved to its own `eye_focus` column. It
/// stayed `3` when the score moved here from the first pass: a row whose score
/// the first pass wrote keeps it, and one the second pass has not reached yet
/// gets the same score from it. `4` re-runs it after the whole-image face
/// search (files without a trusted AF point) moved from the 320 px square
/// model input to `faces::WHOLE_INPUT`, which finds more small faces, so the
/// sharpness score of those files can move to an eye window. `5` re-runs it
/// after the whole-image search started decoding the preview at a DCT scale
/// (`faces::decode_whole`) instead of full size, which changes the pixels the
/// model sees and so the faces it finds. `6` re-runs it after the cue moved
/// to the mesh eye regions (`eyes::mesh_of`, scored per eye with a window
/// fallback). It stayed `6` when the scan stopped running the mesh on faces
/// under `eyes::EYES_MIN_FACE` (`candidate::meshes_face`): no frame changed
/// state on the 806 labeled faced frames or the 2134 files of a real folder,
/// and the stored probability moved on one file, a 58 px face whose eye
/// region had counted (`_DSC2748.ARW`, in both the training set and that
/// folder), not worth re-running the pass everywhere. `7` re-runs it to fill
/// `eyes_ear` and the `pose_*` columns from the mesh the cue already runs; `8`
/// to fill `eye_offset` and `edge_gap` from it. `9` fills the cue, the EAR,
/// the pose, the eye offset and the edge gap for frames with no trusted AF
/// point from the largest confident face of the whole preview.
pub const FACES_VERSION: i64 = 9;

/// Files per transaction while scanning. `thumbnail` / `folder_entries` read
/// through their own connection (`Index::open_reader`) and do not wait on
/// these transactions, so the size is set by what still shares the writer:
/// quitting mid-scan loses at most one small batch, and `set_rating` and the
/// sidecar writer thread, which take the writer mutex, wait behind at most
/// one. The per-commit cost is small: writing 5000 synthetic rows in batches of
/// 10 instead of 50 measured ~15 ms more in total (M3 Pro, release), about
/// 0.3% of a 5000-file first scan.
const BATCH: usize = 10;

/// A folder not opened for this long loses its rows at the next launch
/// (`evict`). Thumbnails are cheap to rebuild on the next open (a rescan) and a
/// month covers returning to a recent shoot, so it errs on keeping the cache
/// warm for what is actually in use.
pub const MAX_AGE_SECS: i64 = 30 * 24 * 60 * 60;

/// The size `evict` shrinks the database under, least-recently-opened folder
/// first, once the aged folders are gone. At the measured ~20.8 KB per row it
/// is ~50k files, a few seasons of shooting, while staying a small share of a
/// laptop's disk.
pub const MAX_BYTES: i64 = 1024 * 1024 * 1024;

/// What `evict` removes.
#[derive(Debug, Clone, Copy)]
pub struct EvictPolicy {
    pub max_age_secs: i64,
    pub max_bytes: i64,
}

pub const EVICT_POLICY: EvictPolicy = EvictPolicy {
    max_age_secs: MAX_AGE_SECS,
    max_bytes: MAX_BYTES,
};

/// How an `evict` ended.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EvictSummary {
    pub folders: usize,
    pub rows: usize,
    pub vacuumed: bool,
}

/// Seconds since the epoch, 0 for a clock set before it.
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Shortest interval between two progress notifications.
pub const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// What `stat` says about one listed file.
#[derive(Debug, Clone)]
pub struct FileStat {
    pub path: PathBuf,
    pub size: i64,
    pub mtime_ns: i64,
}

/// One indexed file, as the frontend sees it.
///
/// `has_sidecar` is whether a sidecar was on disk the last time the app read
/// or wrote one (`ratings.xmp_size` is set), not a live stat.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexedFile {
    pub path: String,
    pub orientation: u16,
    pub capture_time: Option<String>,
    pub subsec: Option<String>,
    pub focus: Option<Focus>,
    pub has_thumb: bool,
    pub rating: Option<i8>,
    #[serde(serialize_with = "serialize_flag")]
    pub flag: Flag,
    pub label: Option<String>,
    pub has_sidecar: bool,
    pub sharpness: Option<f64>,
    /// `None` for a file whose extraction failed.
    pub exif: Option<Exif>,
    /// Why the extraction failed; `None` for a file whose extraction worked.
    pub error: Option<String>,
}

/// The name the frontend uses for `flag`.
pub fn flag_name(flag: Flag) -> &'static str {
    match flag {
        Flag::None => "none",
        Flag::Pick => "pick",
        Flag::Reject => "reject",
    }
}

/// The flag `flag_name` names.
pub fn parse_flag(name: &str) -> Result<Flag, String> {
    match name {
        "none" => Ok(Flag::None),
        "pick" => Ok(Flag::Pick),
        "reject" => Ok(Flag::Reject),
        _ => Err(format!("unknown flag {name:?}")),
    }
}

fn serialize_flag<S: serde::Serializer>(flag: &Flag, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(flag_name(*flag))
}

/// The `ratings.flag` code of `flag`.
fn flag_code(flag: Flag) -> i64 {
    match flag {
        Flag::None => 0,
        Flag::Pick => 1,
        Flag::Reject => 2,
    }
}

fn flag_from_code(code: i64) -> Flag {
    match code {
        1 => Flag::Pick,
        2 => Flag::Reject,
        _ => Flag::None,
    }
}

/// The name the frontend uses for a focus candidate state.
fn candidate_name(state: FocusCandidate) -> &'static str {
    match state {
        FocusCandidate::Candidate => "candidate",
        FocusCandidate::NotCandidate => "not_candidate",
        FocusCandidate::Unknown => "unknown",
    }
}

fn serialize_eyes<S: serde::Serializer>(state: &Option<EyeState>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(eyes_name(*state))
}

/// The name the frontend uses for an eye state, `unknown` for none.
fn eyes_name(state: Option<EyeState>) -> &'static str {
    match state {
        Some(EyeState::Open) => "open",
        Some(EyeState::Closed) => "closed",
        None => "unknown",
    }
}

fn serialize_candidate<S: serde::Serializer>(
    state: &FocusCandidate,
    s: S,
) -> Result<S::Ok, S::Error> {
    s.serialize_str(candidate_name(*state))
}

/// The AF point the camera recorded, in sensor coordinates.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct FocusPoint {
    pub sensor_w: u16,
    pub sensor_h: u16,
    pub x: u16,
    pub y: u16,
    /// The AF frame size, in the same sensor coordinates as `x` / `y`.
    pub frame: Option<FocusSize>,
}

/// The AF point of a RAW file, if one was recorded, and the cue pass 2
/// stored for its judged face: the one nearest a trusted point, else the
/// largest confident face of the whole preview.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Focus {
    pub point: Option<FocusPoint>,
    pub manual_focus: bool,
    /// The judged face's in-focus probability. `None` when no face was
    /// judged, or before the second pass has run.
    pub eye_focus: Option<f64>,
    /// Derived from `eye_focus`, not stored.
    #[serde(serialize_with = "serialize_candidate")]
    pub candidate: FocusCandidate,
    #[serde(flatten)]
    pub eyes: StoredEyes,
}

/// The head pose in degrees, with `pose::Pose`'s signs: yaw positive toward
/// the image's right, pitch positive up, roll positive clockwise on screen.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct EyesPose {
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
}

impl From<Pose> for EyesPose {
    fn from(p: Pose) -> Self {
        EyesPose {
            yaw: p.yaw,
            pitch: p.pitch,
            roll: p.roll,
        }
    }
}

/// What the second pass stored about the judged face's eyes and head
/// (`Cue::eyes_ear`, `Cue::pose`, `Cue::eye_offset`, `Cue::edge_gap`), with
/// the state and the closed probability derived from the EAR. All `None` when
/// the cue ran no mesh, or before the second pass has run.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct StoredEyes {
    /// The EAR of the more closed eye.
    pub eyes_ear: Option<f64>,
    /// Derived from `eyes_ear` with `Eyes::from_ear`, not stored; `unknown`
    /// without one.
    #[serde(serialize_with = "serialize_eyes")]
    pub eyes: Option<EyeState>,
    /// The probability that the eyes are closed, derived like `eyes`.
    pub eyes_closed: Option<f64>,
    pub pose: Option<EyesPose>,
    /// How far the mesh's eyes sit from the face's YuNet eye landmarks, in
    /// face box sides.
    pub eye_offset: Option<f64>,
    /// How close the face and its mesh eye regions come to the preview's
    /// edge, in face box sides; negative outside.
    pub edge_gap: Option<f64>,
}

impl StoredEyes {
    pub fn new(
        eyes_ear: Option<f64>,
        pose: Option<EyesPose>,
        eye_offset: Option<f64>,
        edge_gap: Option<f64>,
    ) -> Self {
        let judged = eyes_ear.map(Eyes::from_ear);
        StoredEyes {
            eyes_ear,
            eyes: judged.map(|e| e.state),
            eyes_closed: judged.map(|e| e.probability),
            pose,
            eye_offset,
            edge_gap,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct FocusSize {
    pub width: u16,
    pub height: u16,
}

/// How a finished scan ended.
#[derive(Debug, Clone, Copy)]
pub struct ScanSummary {
    pub total: usize,
    pub errors: usize,
}

/// One file the second pass has written, as `faces-progress` reports it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FaceReady {
    pub path: String,
    pub eye_focus: Option<f64>,
    #[serde(serialize_with = "serialize_candidate")]
    pub candidate: FocusCandidate,
    pub sharpness: Option<f64>,
    #[serde(flatten)]
    pub eyes: StoredEyes,
}

/// One file's second-pass values, as `write_faces` stores them.
#[derive(Debug, Clone, PartialEq)]
pub struct FacesRow {
    pub path: String,
    pub eye_focus: Option<f64>,
    pub sharpness: Option<f64>,
    pub eyes_ear: Option<f64>,
    pub pose: Option<EyesPose>,
    pub eye_offset: Option<f64>,
    pub edge_gap: Option<f64>,
}

/// One file the third pass has to embed, with the `files` row's stat the
/// embedding is stored against.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbedTodo {
    pub path: String,
    pub size: i64,
    pub mtime_ns: i64,
}

/// One file's embedding, as `write_embeddings` stores it: `None` when the
/// file could not be embedded, which is not retried until its stat or
/// `EMBED_VERSION` moves.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbedRow {
    pub file: EmbedTodo,
    pub vec: Option<Vec<f32>>,
}

/// One file as the ranking reads it (`rank_rows`).
#[derive(Debug, Clone)]
pub struct RankRow {
    pub path: String,
    pub flag: Flag,
    pub viewed: bool,
    pub frame: rank::Frame,
}

/// The folders the ranking reads, which the age rule of `evict` also spares:
/// the ones holding a viewed file or a reject.
const RANK_DIRS: &str =
    "SELECT DISTINCT dir FROM views UNION SELECT DISTINCT dir FROM ratings WHERE flag = 2";

/// The columns `indexed_file` reads, before the `WHERE` clause.
const INDEXED_FILE: &str = "SELECT path, orientation, capture_time, subsec,
            focus_w, focus_h, focus_x, focus_y, thumb IS NOT NULL,
            ratings.rating, ratings.xmp_size IS NOT NULL,
            COALESCE(ratings.flag, 0), error,
            make, model, lens, f_num, f_den, f_estimated, exposure_num,
            exposure_den, iso, focal_num, focal_den, ratings.label,
            sharpness, frame_w, frame_h, manual_focus, eye_focus,
            eyes_ear, pose_yaw, pose_pitch, pose_roll, eye_offset, edge_gap
     FROM files LEFT JOIN ratings USING (path)";

fn indexed_file(r: &rusqlite::Row<'_>) -> rusqlite::Result<IndexedFile> {
    let path: String = r.get(0)?;
    let point = match (r.get::<_, Option<u16>>(4)?, r.get::<_, Option<u16>>(5)?) {
        (Some(sensor_w), Some(sensor_h)) => Some(FocusPoint {
            sensor_w,
            sensor_h,
            x: r.get(6)?,
            y: r.get(7)?,
            frame: r
                .get::<_, Option<u16>>(26)?
                .zip(r.get::<_, Option<u16>>(27)?)
                .map(|(width, height)| FocusSize { width, height }),
        }),
        _ => None,
    };
    let focus = if point.is_some() || is_raw_file(Path::new(&path)) {
        let eye_focus = r.get(29)?;
        let pose = match (r.get(31)?, r.get(32)?, r.get(33)?) {
            (Some(yaw), Some(pitch), Some(roll)) => Some(EyesPose { yaw, pitch, roll }),
            _ => None,
        };
        Some(Focus {
            point,
            manual_focus: r.get(28)?,
            eye_focus,
            candidate: candidate(eye_focus),
            eyes: StoredEyes::new(r.get(30)?, pose, r.get(34)?, r.get(35)?),
        })
    } else {
        None
    };
    let rational = |num: usize| -> rusqlite::Result<Option<Rational>> {
        Ok(r.get::<_, Option<i64>>(num)?
            .zip(r.get::<_, Option<i64>>(num + 1)?)
            .map(|(num, den)| Rational { num, den }))
    };
    let error: Option<String> = r.get(12)?;
    // An error row whose failure carried no metadata has no orientation.
    let exif = if error.is_some() && r.get::<_, Option<u16>>(1)?.is_none() {
        None
    } else {
        Some(exif(&Shot {
            make: r.get(13)?,
            model: r.get(14)?,
            lens_model: r.get(15)?,
            f_number: rational(16)?,
            estimated_f_number: r.get(18)?,
            exposure_time: rational(19)?,
            iso: r.get(21)?,
            focal_length: rational(22)?,
            ..Shot::default()
        }))
    };
    Ok(IndexedFile {
        path,
        orientation: r.get::<_, Option<u16>>(1)?.unwrap_or(1),
        capture_time: r.get(2)?,
        subsec: r.get(3)?,
        focus,
        has_thumb: r.get(8)?,
        rating: r.get(9)?,
        flag: flag_from_code(r.get(11)?),
        label: r.get(24)?,
        has_sidecar: r.get(10)?,
        sharpness: r.get(25)?,
        exif,
        error,
    })
}

pub struct Index {
    pub(crate) conn: Connection,
}

/// Take a lock without caring whether a previous holder panicked. The scan
/// hands `on_item` to rayon workers, and a panic there would otherwise poison
/// the connection for the rest of the process.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Append a suffix to a path's file name, for the `-wal`/`-shm` sidecars SQLite
/// keeps next to the main database file.
pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// `stat` a file for validity checking. Times before the epoch, which no real
/// photo has, collapse to 0 rather than failing the scan.
pub fn stat(path: &Path) -> Result<FileStat, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mtime_ns = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos() as i64);
    Ok(FileStat {
        path: path.to_path_buf(),
        size: meta.len() as i64,
        mtime_ns,
    })
}

impl Index {
    /// Open, creating the file and the schema on first use. `v2` to `v4` are migrated
    /// to the current schema in place; a database written by any other
    /// unrecognized schema version is discarded, since it is a cache.
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let mut index = Self { conn };
        if let Err(e) = index.prepare() {
            log::warn!("discarding the index cache at {}: {e}", path.display());
            drop(index);
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_file(with_suffix(path, "-wal"));
            let _ = std::fs::remove_file(with_suffix(path, "-shm"));
            let conn = Connection::open(path).map_err(|e| e.to_string())?;
            index = Self { conn };
            index.prepare()?;
        }
        Ok(index)
    }

    /// Open a second, read-only connection on a database `open` has already
    /// returned `Ok` for, so the schema and the WAL files exist. Under WAL it
    /// reads a committed snapshot without waiting for the writer connection.
    pub fn open_reader(path: &Path) -> Result<Self, String> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| e.to_string())?;
        conn.busy_timeout(Duration::from_millis(300))
            .map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }

    fn prepare(&mut self) -> Result<(), String> {
        self.conn
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        // A commit is then a WAL append with no fsync, which is what keeps the
        // `set_rating` write off the critical path of a keypress.
        self.conn
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| e.to_string())?;
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if ![
            0,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            11,
            12,
            13,
            14,
            15,
            16,
            17,
            18,
            19,
            20,
            21,
            22,
            SCHEMA_VERSION,
        ]
        .contains(&version)
        {
            return Err(format!("unsupported index schema version {version}"));
        }
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        if version != 0 && version < 10 {
            tx.execute_batch("DROP TABLE IF EXISTS files;")
                .map_err(|e| e.to_string())?;
        }
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     size INTEGER NOT NULL,
                     mtime_ns INTEGER NOT NULL,
                     orientation INTEGER,
                     capture_time TEXT,
                     subsec TEXT,
                     focus_w INTEGER,
                     focus_h INTEGER,
                     focus_x INTEGER,
                     focus_y INTEGER,
                     thumb BLOB,
                     error TEXT,
                     make TEXT,
                     model TEXT,
                     lens TEXT,
                     f_num INTEGER,
                     f_den INTEGER,
                     f_estimated REAL,
                     exposure_num INTEGER,
                     exposure_den INTEGER,
                     iso INTEGER,
                     focal_num INTEGER,
                     focal_den INTEGER,
                     sharpness REAL,
                     extractor INTEGER NOT NULL DEFAULT 0,
                     frame_w INTEGER,
                     frame_h INTEGER,
                     manual_focus INTEGER NOT NULL DEFAULT 0,
                     eye_focus REAL,
                     faces_extractor INTEGER NOT NULL DEFAULT 0,
                     eyes_ear REAL,
                     pose_yaw REAL,
                     pose_pitch REAL,
                     pose_roll REAL,
                     eye_offset REAL,
                     edge_gap REAL
                 );
                 CREATE INDEX IF NOT EXISTS files_dir ON files (dir);
                 CREATE INDEX IF NOT EXISTS files_capture ON files (capture_time, subsec);
                 -- `xmp_size` / `xmp_mtime_ns` are the stat of the effective
                 -- sidecar of the selected format (XMP or `.dop`; the newest
                 -- one under Both), whatever its name says.
                 -- `flag` is the pick / reject (`0` none, `1` pick, `2`
                 -- reject), kept apart from the `0`-`5` `rating` because the
                 -- two coexist in both an XMP and a `.dop`.
                 CREATE TABLE IF NOT EXISTS ratings (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     rating INTEGER,
                     flag INTEGER NOT NULL DEFAULT 0,
                     label TEXT,
                     label_known INTEGER NOT NULL DEFAULT 1,
                     xmp_size INTEGER,
                     xmp_mtime_ns INTEGER,
                     dirty INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE INDEX IF NOT EXISTS ratings_dir ON ratings (dir);
                 -- `opened_at` is seconds since the epoch, set by `reconcile`.
                 -- `last_viewed` is the path of the file that was current
                 -- in the strip, set by `set_last_viewed`.
                 CREATE TABLE IF NOT EXISTS folders (
                     dir TEXT PRIMARY KEY,
                     opened_at INTEGER NOT NULL,
                     last_viewed TEXT
                 );
                 -- One image embedding per file, apart from `files` so a
                 -- first-pass `INSERT OR REPLACE` keeps it. Valid while
                 -- `version` is `EMBED_VERSION` and `size` / `mtime_ns`
                 -- match the `files` row; `vec` is little-endian f32, `NULL`
                 -- for a file that could not be embedded.
                 CREATE TABLE IF NOT EXISTS embeddings (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     size INTEGER NOT NULL,
                     mtime_ns INTEGER NOT NULL,
                     version INTEGER NOT NULL,
                     vec BLOB
                 );
                 CREATE INDEX IF NOT EXISTS embeddings_dir ON embeddings (dir);
                 -- The files shown in the preview at least once, apart from
                 -- `files` so a re-extraction keeps them; eviction keeps
                 -- them too, only `clear` drops them. `viewed_at` is seconds
                 -- since the epoch of the first view.
                 CREATE TABLE IF NOT EXISTS views (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     viewed_at INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS views_dir ON views (dir);",
        )
        .map_err(|e| e.to_string())?;
        // `files` was just dropped, so this seeds nothing; kept so a v7
        // database still gains the `folders` table in the same way.
        if version == 7 {
            tx.execute(
                "INSERT OR IGNORE INTO folders (dir, opened_at)
                 SELECT DISTINCT dir, ?1 FROM files",
                params![now_secs()],
            )
            .map_err(|e| e.to_string())?;
        }
        if version == 2 {
            tx.execute_batch("ALTER TABLE ratings ADD COLUMN pick INTEGER NOT NULL DEFAULT 0;")
                .map_err(|e| e.to_string())?;
        }
        if version != 0 && version != SCHEMA_VERSION && version < 5 {
            tx.execute_batch("ALTER TABLE ratings ADD COLUMN label TEXT;")
                .map_err(|e| e.to_string())?;
        }
        if version != 0 && version != SCHEMA_VERSION && version < 6 {
            tx.execute_batch(
                "ALTER TABLE ratings ADD COLUMN label_known INTEGER NOT NULL DEFAULT 1;",
            )
            .map_err(|e| e.to_string())?;
        }
        if version != 0 && version < 11 {
            // Clean rows carry stats matching values written by the old,
            // lossy `read_pick` / `read_rating` shims (a Lightroom pick
            // stored as `pick = 0`, a starred reject stored with its stars
            // dropped). Invalidate their stat so `reconcile_sidecars`
            // re-parses the sidecar with the real readers instead of trusting
            // those stale values.
            tx.execute_batch(
                "ALTER TABLE ratings ADD COLUMN flag INTEGER NOT NULL DEFAULT 0;
                 UPDATE ratings SET flag = CASE WHEN rating = -1 THEN 2
                     WHEN pick = 1 THEN 1 ELSE 0 END;
                 UPDATE ratings SET rating = NULL WHERE rating = -1;
                 ALTER TABLE ratings DROP COLUMN pick;
                 UPDATE ratings SET xmp_size = NULL, xmp_mtime_ns = NULL WHERE dirty = 0;",
            )
            .map_err(|e| e.to_string())?;
        }
        // Below v10 `files` was just dropped and recreated with the column.
        if (10..12).contains(&version) {
            tx.execute_batch("ALTER TABLE files ADD COLUMN extractor INTEGER NOT NULL DEFAULT 0;")
                .map_err(|e| e.to_string())?;
        }
        if (10..13).contains(&version) {
            tx.execute_batch(
                "ALTER TABLE files ADD COLUMN frame_w INTEGER;
                 ALTER TABLE files ADD COLUMN frame_h INTEGER;
                 ALTER TABLE files ADD COLUMN manual_focus INTEGER NOT NULL DEFAULT 0;",
            )
            .map_err(|e| e.to_string())?;
        }
        if (10..15).contains(&version) {
            tx.execute_batch(
                "ALTER TABLE files ADD COLUMN eye_focus REAL;
                 ALTER TABLE files ADD COLUMN faces_extractor INTEGER NOT NULL DEFAULT 0;",
            )
            .map_err(|e| e.to_string())?;
        }
        if version == 14 {
            tx.execute_batch("ALTER TABLE files DROP COLUMN face_catch;")
                .map_err(|e| e.to_string())?;
        }
        if version == 15 {
            tx.execute_batch(
                "ALTER TABLE files DROP COLUMN eye_sharpness;
                 ALTER TABLE files ADD COLUMN eye_focus REAL;",
            )
            .map_err(|e| e.to_string())?;
        }
        // Below v8 `folders` was just created with the column.
        if (8..17).contains(&version) {
            tx.execute_batch("ALTER TABLE folders ADD COLUMN last_viewed TEXT;")
                .map_err(|e| e.to_string())?;
        }
        if (10..18).contains(&version) {
            tx.execute_batch(
                "ALTER TABLE files ADD COLUMN eyes_ear REAL;
                 ALTER TABLE files ADD COLUMN pose_yaw REAL;
                 ALTER TABLE files ADD COLUMN pose_pitch REAL;
                 ALTER TABLE files ADD COLUMN pose_roll REAL;",
            )
            .map_err(|e| e.to_string())?;
        }
        if (10..19).contains(&version) {
            tx.execute_batch(
                "ALTER TABLE files ADD COLUMN eye_offset REAL;
                 ALTER TABLE files ADD COLUMN edge_gap REAL;",
            )
            .map_err(|e| e.to_string())?;
        }
        if (21..23).contains(&version) {
            tx.execute_batch("ALTER TABLE folders DROP COLUMN culling_finished;")
                .map_err(|e| e.to_string())?;
        }
        if version == 22 {
            tx.execute_batch("ALTER TABLE folders DROP COLUMN culling_manual;")
                .map_err(|e| e.to_string())?;
        }
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// Drop the rows under `dir` whose file is gone, whose `size`/`mtime_ns`
    /// no longer match, or that were written at an older `EXTRACTOR_VERSION`,
    /// and the embeddings of the files gone or changed (a row at an older
    /// `EXTRACTOR_VERSION` keeps its embedding),
    /// and return the files that have no valid row, in the
    /// order given, along with the number of rows dropped. Paths are compared
    /// with the same lossy conversion
    /// `write_batch` uses to key rows, so a non-UTF-8 path matches the row it
    /// wrote instead of being rescanned on every open.
    pub fn reconcile(
        &mut self,
        dir: &str,
        files: &[FileStat],
    ) -> Result<(Vec<FileStat>, usize), String> {
        let known: Vec<(String, i64, i64, i64)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT path, size, mtime_ns, extractor FROM files WHERE dir = ?1")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![dir], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<rusqlite::Result<_>>()
                .map_err(|e| e.to_string())?
        };
        let embedded: Vec<(String, i64, i64)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT path, size, mtime_ns FROM embeddings WHERE dir = ?1")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![dir], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(|e| e.to_string())?;
            rows.collect::<rusqlite::Result<_>>()
                .map_err(|e| e.to_string())?
        };
        let listed: std::collections::HashMap<String, (i64, i64, i64)> = files
            .iter()
            .map(|f| {
                (
                    f.path.to_string_lossy().into_owned(),
                    (f.size, f.mtime_ns, EXTRACTOR_VERSION),
                )
            })
            .collect();

        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO folders (dir, opened_at) VALUES (?1, ?2)
             ON CONFLICT (dir) DO UPDATE SET opened_at = excluded.opened_at",
            params![dir, now_secs()],
        )
        .map_err(|e| e.to_string())?;
        let mut removed = 0;
        for (path, ..) in known
            .iter()
            .filter(|(p, s, m, v)| listed.get(p.as_str()) != Some(&(*s, *m, *v)))
        {
            removed += tx
                .execute("DELETE FROM files WHERE path = ?1", params![path])
                .map_err(|e| e.to_string())?;
        }
        for (path, ..) in embedded.iter().filter(|(p, s, m)| {
            listed
                .get(p.as_str())
                .map(|(size, mtime, _)| (*size, *mtime))
                != Some((*s, *m))
        }) {
            tx.execute("DELETE FROM embeddings WHERE path = ?1", params![path])
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;

        let known: std::collections::HashMap<&str, (i64, i64, i64)> = known
            .iter()
            .map(|(p, s, m, v)| (p.as_str(), (*s, *m, *v)))
            .collect();
        let todo = files
            .iter()
            .filter(|f| {
                let path = f.path.to_string_lossy();
                known.get(path.as_ref()) != Some(&(f.size, f.mtime_ns, EXTRACTOR_VERSION))
            })
            .cloned()
            .collect();
        Ok((todo, removed))
    }

    /// Drop the `files` rows and clean `ratings` rows of folders last opened
    /// more than `policy.max_age_secs` before `now`, except a folder the
    /// ranking reads (any viewed file or reject, `RANK_DIRS`), then of the
    /// least-recently-opened folders, read by the ranking or not, until the
    /// pages in use fit `policy.max_bytes`, and `VACUUM` if anything was
    /// deleted. A folder keeping a dirty rating keeps its `folders` row, so it
    /// is reconsidered next time. `VACUUM` rewrites the whole file, so call
    /// this with no scan running.
    pub fn evict(&mut self, now: i64, policy: EvictPolicy) -> Result<EvictSummary, String> {
        let folders: Vec<(String, i64, bool)> = {
            let mut stmt = self
                .conn
                .prepare(&format!(
                    "SELECT dir, opened_at, dir IN ({RANK_DIRS}) FROM folders
                     ORDER BY opened_at, dir"
                ))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(|e| e.to_string())?;
            rows.collect::<rusqlite::Result<_>>()
                .map_err(|e| e.to_string())?
        };
        let mut summary = EvictSummary::default();
        for (dir, opened_at, ranked) in &folders {
            let young = *opened_at >= now - policy.max_age_secs;
            if (young || *ranked) && self.used_bytes()? <= policy.max_bytes {
                if young {
                    break;
                }
                continue;
            }
            summary.rows += self.evict_folder(dir)?;
            summary.folders += 1;
        }
        summary.rows += self.drop_stray_embeddings()?;
        self.vacuum_if_deleted(&mut summary)?;
        Ok(summary)
    }

    /// Drop the `files` rows and clean `ratings` rows of every folder, then
    /// `VACUUM` and truncate the WAL so the space is given back to the file
    /// system. A folder keeping a dirty rating keeps its `folders` row and
    /// that rating, as in `evict`. Unlike `evict`, it also drops every
    /// `views` row. Like `evict`, call this with no scan running.
    pub fn clear(&mut self) -> Result<EvictSummary, String> {
        let folders: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT dir FROM folders ORDER BY dir")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            rows.collect::<rusqlite::Result<_>>()
                .map_err(|e| e.to_string())?
        };
        let mut summary = EvictSummary::default();
        for dir in &folders {
            summary.rows += self.evict_folder(dir)?;
            summary.folders += 1;
        }
        summary.rows += self.drop_stray_embeddings()?;
        summary.rows += self
            .conn
            .execute("DELETE FROM views", [])
            .map_err(|e| e.to_string())?;
        if self.vacuum_if_deleted(&mut summary)? {
            // `VACUUM` writes the rebuilt database through the WAL, which
            // keeps the old size on disk until a checkpoint truncates it. A
            // reader in a read transaction makes this fail, and the clear
            // itself still succeeded, so only log it.
            if let Err(e) = self
                .conn
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            {
                log::warn!("could not truncate the index WAL: {e}");
            }
        }
        Ok(summary)
    }

    fn vacuum_if_deleted(&mut self, summary: &mut EvictSummary) -> Result<bool, String> {
        if summary.rows == 0 {
            return Ok(false);
        }
        self.conn
            .execute_batch("VACUUM")
            .map_err(|e| e.to_string())?;
        summary.vacuumed = true;
        Ok(true)
    }

    /// The bytes of the pages holding data, which a delete lowers at once
    /// while the file itself only shrinks at the `VACUUM`.
    fn used_bytes(&self) -> Result<i64, String> {
        let pragma = |name: &str| -> Result<i64, String> {
            self.conn
                .pragma_query_value(None, name, |r| r.get(0))
                .map_err(|e| e.to_string())
        };
        Ok((pragma("page_count")? - pragma("freelist_count")?) * pragma("page_size")?)
    }

    /// Drop the embeddings of folders with no `folders` row. A third pass
    /// keeps writing while a clear, a trash or a rename runs (it does not
    /// count as scanning), so a late batch can land under a folder just
    /// evicted or renamed away.
    fn drop_stray_embeddings(&mut self) -> Result<usize, String> {
        self.conn
            .execute(
                "DELETE FROM embeddings WHERE dir NOT IN (SELECT dir FROM folders)",
                [],
            )
            .map_err(|e| e.to_string())
    }

    /// One folder's share of the deletion `evict` and `clear` do: the `files`
    /// rows, the embeddings, the clean `ratings` rows, and the `folders` row
    /// when no rating is left. Returns the rows deleted.
    fn evict_folder(&mut self, dir: &str) -> Result<usize, String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        let mut rows = tx
            .execute("DELETE FROM files WHERE dir = ?1", params![dir])
            .map_err(|e| e.to_string())?;
        rows += tx
            .execute("DELETE FROM embeddings WHERE dir = ?1", params![dir])
            .map_err(|e| e.to_string())?;
        rows += tx
            .execute(
                "DELETE FROM ratings WHERE dir = ?1 AND dirty = 0",
                params![dir],
            )
            .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM folders WHERE dir = ?1
                 AND NOT EXISTS (SELECT 1 FROM ratings WHERE dir = ?1)",
            params![dir],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Write one batch of results in a single transaction. An error row keeps
    /// the metadata its failure carries, with no thumbnail.
    pub fn write_batch(
        &mut self,
        dir: &str,
        rows: &[(FileStat, Result<Entry, Failure>)],
    ) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        let unknown = Shot::default();
        for (file, result) in rows {
            let path = file.path.to_string_lossy();
            let (orientation, shot, thumb, error) = match result {
                Ok(entry) => (
                    Some(entry.orientation),
                    &entry.shot,
                    Some(&entry.thumbnail),
                    None,
                ),
                Err(failure) => (
                    failure.shot.as_ref().map(|_| failure.orientation),
                    failure.shot.as_deref().unwrap_or(&unknown),
                    None,
                    Some(&failure.message),
                ),
            };
            let focus = shot.focus;
            tx.execute(
                "INSERT OR REPLACE INTO files (path, dir, size, mtime_ns, orientation,
                     capture_time, subsec, focus_w, focus_h, focus_x, focus_y, thumb, error,
                     make, model, lens, f_num, f_den, f_estimated, exposure_num,
                     exposure_den, iso, focal_num, focal_den, sharpness, extractor,
                     frame_w, frame_h, manual_focus, eye_focus, faces_extractor)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                     ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, NULL, ?25,
                     ?26, ?27, ?28, NULL, 0)",
                params![
                    path,
                    dir,
                    file.size,
                    file.mtime_ns,
                    orientation,
                    shot.capture_time,
                    shot.subsec,
                    focus.map(|f| f.sensor_w),
                    focus.map(|f| f.sensor_h),
                    focus.map(|f| f.x),
                    focus.map(|f| f.y),
                    thumb,
                    error,
                    shot.make,
                    shot.model,
                    shot.lens_model,
                    shot.f_number.map(|r| r.num),
                    shot.f_number.map(|r| r.den),
                    shot.estimated_f_number,
                    shot.exposure_time.map(|r| r.num),
                    shot.exposure_time.map(|r| r.den),
                    shot.iso,
                    shot.focal_length.map(|r| r.num),
                    shot.focal_length.map(|r| r.den),
                    EXTRACTOR_VERSION,
                    shot.focus_frame.map(|f| f.width),
                    shot.focus_frame.map(|f| f.height),
                    manual_focus(shot),
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// The files of `dir` the second pass still has to run on, in file-name
    /// order like `entries`. Rows it has nothing to compute for (a failed
    /// extraction, or a JPEG file, told by the extension as
    /// `riffle_core::scan::is_jpeg_file` does; SQLite's `LIKE` ignores ASCII
    /// case) are marked done at `FACES_VERSION` with no in-focus probability
    /// and no score first, in the same transaction. A RAW without a trusted AF
    /// point is listed: its cue judges the largest confident face of the whole
    /// preview, and it gets a score.
    pub fn faces_todo(&mut self, dir: &str) -> Result<Vec<String>, String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE files SET faces_extractor = ?2, eye_focus = NULL, sharpness = NULL,
                 eyes_ear = NULL, pose_yaw = NULL, pose_pitch = NULL, pose_roll = NULL,
                 eye_offset = NULL, edge_gap = NULL
             WHERE dir = ?1 AND faces_extractor != ?2
                 AND (error IS NOT NULL OR path LIKE '%.jpg' OR path LIKE '%.jpeg')",
            params![dir, FACES_VERSION],
        )
        .map_err(|e| e.to_string())?;
        let mut paths: Vec<String> = {
            let mut stmt = tx
                .prepare("SELECT path FROM files WHERE dir = ?1 AND faces_extractor != ?2")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![dir, FACES_VERSION], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            rows.collect::<rusqlite::Result<_>>()
                .map_err(|e| e.to_string())?
        };
        tx.commit().map_err(|e| e.to_string())?;
        paths.sort_by(|a, b| Path::new(a).file_name().cmp(&Path::new(b).file_name()));
        Ok(paths)
    }

    /// Store the in-focus probability, the sharpness score, the EAR, the head
    /// pose, the eye offset and the edge gap of each path, `None` for none, at
    /// `FACES_VERSION`, in a single transaction.
    pub fn write_faces(&mut self, rows: &[FacesRow]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for row in rows {
            tx.execute(
                "UPDATE files SET eye_focus = ?2, sharpness = ?3, faces_extractor = ?4,
                     eyes_ear = ?5, pose_yaw = ?6, pose_pitch = ?7, pose_roll = ?8,
                     eye_offset = ?9, edge_gap = ?10
                 WHERE path = ?1",
                params![
                    row.path,
                    row.eye_focus,
                    row.sharpness,
                    FACES_VERSION,
                    row.eyes_ear,
                    row.pose.map(|p| p.yaw),
                    row.pose.map(|p| p.pitch),
                    row.pose.map(|p| p.roll),
                    row.eye_offset,
                    row.edge_gap,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// The files of `dir` the third pass still has to embed, in file-name
    /// order like `entries`: the RAW rows (not a JPEG, told by the extension
    /// as `faces_todo` does, and not a failed extraction, whose preview is
    /// unreadable) with no embedding, or one at another `EMBED_VERSION`, or
    /// one stored against another `size` / `mtime_ns` than the row's.
    pub fn embeddings_missing(&self, dir: &str) -> Result<Vec<EmbedTodo>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT files.path, files.size, files.mtime_ns FROM files
                 LEFT JOIN embeddings USING (path)
                 WHERE files.dir = ?1 AND files.error IS NULL
                     AND files.path NOT LIKE '%.jpg' AND files.path NOT LIKE '%.jpeg'
                     AND (embeddings.path IS NULL OR embeddings.version != ?2
                         OR embeddings.size != files.size
                         OR embeddings.mtime_ns != files.mtime_ns)",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![dir, EMBED_VERSION], |r| {
                Ok(EmbedTodo {
                    path: r.get(0)?,
                    size: r.get(1)?,
                    mtime_ns: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut todo: Vec<EmbedTodo> = rows
            .collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())?;
        todo.sort_by(|a, b| {
            Path::new(&a.path)
                .file_name()
                .cmp(&Path::new(&b.path).file_name())
        });
        Ok(todo)
    }

    /// Store each embedding at `EMBED_VERSION` against the stat it was
    /// listed with, replacing what the path had, in a single transaction.
    pub fn write_embeddings(&mut self, dir: &str, rows: &[EmbedRow]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for row in rows {
            let vec: Option<Vec<u8>> = row
                .vec
                .as_ref()
                .map(|v| v.iter().flat_map(|x| x.to_le_bytes()).collect());
            tx.execute(
                "INSERT INTO embeddings (path, dir, size, mtime_ns, version, vec)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (path) DO UPDATE SET dir = ?2, size = ?3, mtime_ns = ?4,
                     version = ?5, vec = ?6",
                params![
                    row.file.path,
                    dir,
                    row.file.size,
                    row.file.mtime_ns,
                    EMBED_VERSION,
                    vec
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// The valid embeddings of `dir`'s files, `(path, vec)`: at
    /// `EMBED_VERSION`, stored against the `files` row's `size` / `mtime_ns`,
    /// and not a failed embedding.
    pub fn embeddings(&self, dir: &str) -> Result<Vec<(String, Vec<f32>)>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT files.path, embeddings.vec FROM files
                 JOIN embeddings USING (path)
                 WHERE files.dir = ?1 AND embeddings.version = ?2
                     AND embeddings.size = files.size
                     AND embeddings.mtime_ns = files.mtime_ns
                     AND embeddings.vec IS NOT NULL",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![dir, EMBED_VERSION], |r| {
                let vec: Vec<u8> = r.get(1)?;
                Ok((
                    r.get(0)?,
                    vec.chunks_exact(4)
                        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect(),
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())
    }

    /// The indexed files of `dir`, in file-name order like `list_arw`.
    pub fn entries(&self, dir: &str) -> Result<Vec<IndexedFile>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("{INDEXED_FILE} WHERE files.dir = ?1"))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![dir], indexed_file)
            .map_err(|e| e.to_string())?;
        let mut entries: Vec<IndexedFile> = rows
            .collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by(|a, b| {
            Path::new(&a.path)
                .file_name()
                .cmp(&Path::new(&b.path).file_name())
        });
        Ok(entries)
    }

    /// The indexed row of one file, `None` when the index has no row for it.
    pub fn entry(&self, path: &str) -> Result<Option<IndexedFile>, String> {
        self.conn
            .query_row(
                &format!("{INDEXED_FILE} WHERE files.path = ?1"),
                params![path],
                indexed_file,
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    /// The file that was current in the strip when `dir` was last viewed,
    /// `None` when none was recorded.
    pub fn last_viewed(&self, dir: &str) -> Result<Option<String>, String> {
        self.conn
            .query_row(
                "SELECT last_viewed FROM folders WHERE dir = ?1",
                params![dir],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()
            .map(Option::flatten)
            .map_err(|e| e.to_string())
    }

    /// Record `path` as the file current in the strip for `dir`, creating
    /// the `folders` row when `reconcile` has not yet.
    pub fn set_last_viewed(&mut self, dir: &str, path: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO folders (dir, opened_at, last_viewed) VALUES (?1, ?2, ?3)
                 ON CONFLICT (dir) DO UPDATE SET last_viewed = excluded.last_viewed",
                params![dir, now_secs(), path],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Record `paths` of `dir` as shown in the preview, keeping the first
    /// `viewed_at` of a path already recorded.
    pub fn mark_viewed(&mut self, dir: &str, paths: &[String]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare("INSERT OR IGNORE INTO views (path, dir, viewed_at) VALUES (?1, ?2, ?3)")
                .map_err(|e| e.to_string())?;
            let now = now_secs();
            for path in paths {
                stmt.execute(params![path, dir, now])
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// The files of `dir` shown in the preview at least once, in path order.
    pub fn viewed_files(&self, dir: &str) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM views WHERE dir = ?1 ORDER BY path")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![dir], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())
    }

    /// The folders a ranking fit reads (any viewed file or reject,
    /// `RANK_DIRS`), in `dir` order.
    pub fn rank_dirs(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("{RANK_DIRS} ORDER BY dir"))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())
    }

    /// What the ranking reads of every `files` row of `dir`, in file-name
    /// order: the flag, whether the file was viewed, the capture time, the
    /// second pass's values when the row is at `FACES_VERSION`, and the
    /// embedding when it is valid (as `embeddings` tells).
    pub fn rank_rows(&self, dir: &str) -> Result<Vec<RankRow>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT files.path, COALESCE(ratings.flag, 0), capture_time, subsec,
                     faces_extractor = ?3, sharpness, eye_focus, eyes_ear,
                     pose_yaw, pose_pitch, pose_roll, eye_offset, edge_gap, embeddings.vec,
                     views.path IS NOT NULL
                 FROM files LEFT JOIN ratings USING (path)
                 LEFT JOIN embeddings ON embeddings.path = files.path
                     AND embeddings.version = ?2 AND embeddings.size = files.size
                     AND embeddings.mtime_ns = files.mtime_ns
                 LEFT JOIN views ON views.path = files.path
                 WHERE files.dir = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![dir, EMBED_VERSION, FACES_VERSION], |r| {
                let path: String = r.get(0)?;
                let current: bool = r.get(4)?;
                let value = |i: usize| -> rusqlite::Result<Option<f64>> {
                    Ok(if current { r.get(i)? } else { None })
                };
                let pose = match (value(8)?, value(9)?, value(10)?) {
                    (Some(yaw), Some(pitch), Some(roll)) => Some(Pose { yaw, pitch, roll }),
                    _ => None,
                };
                let vec: Option<Vec<u8>> = r.get(13)?;
                Ok(RankRow {
                    flag: flag_from_code(r.get(1)?),
                    viewed: r.get(14)?,
                    frame: rank::Frame {
                        name: Path::new(&path)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        capture_time: r.get(2)?,
                        subsec: r.get(3)?,
                        sharpness: value(5)?,
                        eye_focus: value(6)?,
                        eyes_ear: value(7)?,
                        pose,
                        eye_offset: value(11)?,
                        edge_gap: value(12)?,
                        vec: vec.map(|v| {
                            v.chunks_exact(4)
                                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                                .collect()
                        }),
                    },
                    path,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut rows: Vec<RankRow> = rows
            .collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())?;
        rows.sort_by(|a, b| a.frame.name.cmp(&b.frame.name));
        Ok(rows)
    }

    /// Move every row of the folder `old` and of the folders under it to
    /// `new`, in one transaction: the `dir` and `path` of `files`, `ratings`,
    /// `embeddings` and `views`, and the `dir` and `last_viewed` of `folders`. Every other
    /// column is kept, so nothing is re-extracted and no dirty judgment is
    /// lost. Rows already stored under `new` belong to a folder that no
    /// longer exists there (the rename's target did not exist) and are
    /// dropped first, or they would collide with the moved ones. Returns the
    /// number of `files` rows moved.
    ///
    /// "Under" is matched with `substr` on `old` plus the separator rather
    /// than `LIKE`, whose `_` and `%` are ordinary path characters.
    pub fn rename_dir(&mut self, old: &str, new: &str) -> Result<usize, String> {
        if old == new {
            return Ok(0);
        }
        let old_prefix = format!("{old}{}", std::path::MAIN_SEPARATOR);
        let new_prefix = format!("{new}{}", std::path::MAIN_SEPARATOR);
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for table in ["files", "ratings", "embeddings", "views", "folders"] {
            tx.execute(
                &format!(
                    "DELETE FROM {table}
                     WHERE dir = ?1 OR substr(dir, 1, length(?2)) = ?2"
                ),
                params![new, new_prefix],
            )
            .map_err(|e| e.to_string())?;
        }
        let mut moved = 0;
        for table in ["files", "ratings", "embeddings", "views"] {
            let count = tx
                .execute(
                    &format!(
                        "UPDATE {table}
                         SET dir = ?2 || substr(dir, length(?1) + 1),
                             path = ?2 || substr(path, length(?1) + 1)
                         WHERE dir = ?1 OR substr(dir, 1, length(?3)) = ?3"
                    ),
                    params![old, new, old_prefix],
                )
                .map_err(|e| e.to_string())?;
            if table == "files" {
                moved = count;
            }
        }
        tx.execute(
            "UPDATE folders
             SET dir = ?2 || substr(dir, length(?1) + 1),
                 last_viewed = CASE WHEN substr(last_viewed, 1, length(?3)) = ?3
                     THEN ?2 || substr(last_viewed, length(?1) + 1)
                     ELSE last_viewed END
             WHERE dir = ?1 OR substr(dir, 1, length(?3)) = ?3",
            params![old, new, old_prefix],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(moved)
    }

    /// Move the rows of the file `old` to `new` in the same folder, in one
    /// transaction: the `path` of `files`, `ratings`, `embeddings` and `views` (their
    /// `dir` stays)
    /// and `folders.last_viewed` where it names `old`. Every other column is
    /// kept. Rows already stored under `new` belong to a file that no longer
    /// exists there and are dropped first, as in `rename_dir`.
    pub fn rename_file(&mut self, old: &str, new: &str) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for table in ["files", "ratings", "embeddings", "views"] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE path = ?1"),
                params![new],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                &format!("UPDATE {table} SET path = ?2 WHERE path = ?1"),
                params![old, new],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "UPDATE folders SET last_viewed = ?2 WHERE last_viewed = ?1",
            params![old, new],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// Move the rows of the file `old` to `new` in the folder `new_dir`, in
    /// one transaction: the `path` and `dir` of `files`, `ratings`,
    /// `embeddings` and `views`, and `folders.last_viewed` where it names
    /// `old`. Every other column is kept. Rows already stored under `new`
    /// belong to a file that no longer exists there and are dropped first, as
    /// in `rename_file`. No `folders` row is made for `new_dir`; `reconcile`
    /// makes it when the folder is opened.
    // Used by the date-folder split (`date_folder_run`), its next step.
    #[cfg_attr(not(test), expect(dead_code))]
    pub fn move_file(&mut self, old: &str, new_dir: &str, new: &str) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for table in ["files", "ratings", "embeddings", "views"] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE path = ?1"),
                params![new],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                &format!("UPDATE {table} SET path = ?2, dir = ?3 WHERE path = ?1"),
                params![old, new, new_dir],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "UPDATE folders SET last_viewed = ?2 WHERE last_viewed = ?1",
            params![old, new],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// Drop every row of exactly the folder `dir`, dirty ratings included,
    /// and its `folders` row, in one transaction, for a folder removed from
    /// disk. A folder under it and a prefix sibling are untouched.
    // Used by the date-folder split (`date_folder_run`), its next step.
    #[cfg_attr(not(test), expect(dead_code))]
    pub fn forget_dir(&mut self, dir: &str) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for table in ["files", "ratings", "embeddings", "views", "folders"] {
            tx.execute(&format!("DELETE FROM {table} WHERE dir = ?1"), params![dir])
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// The raw `orientation` column of one file, `None` when the index has
    /// no row for it, `Some(None)` when the row has no Orientation (an
    /// error row, or a not-yet-extracted one).
    pub fn orientation(&self, path: &str) -> Result<Option<Option<u16>>, String> {
        self.conn
            .query_row(
                "SELECT orientation FROM files WHERE path = ?1",
                params![path],
                |r| r.get::<_, Option<u16>>(0),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    /// The cached thumbnail of one file with its Orientation.
    pub fn thumbnail(&self, path: &str) -> Result<(u16, Vec<u8>), String> {
        self.conn
            .query_row(
                "SELECT orientation, thumb FROM files WHERE path = ?1",
                params![path],
                |r| Ok((r.get::<_, Option<u16>>(0)?, r.get::<_, Option<Vec<u8>>>(1)?)),
            )
            .map_err(|e| format!("{path}: {e}"))
            .and_then(|(orientation, thumb)| match thumb {
                Some(thumb) => Ok((orientation.unwrap_or(1), thumb)),
                None => Err(format!("{path}: no cached thumbnail")),
            })
    }

    /// Record a judgment for one file, pending a sidecar write.
    ///
    /// The row is independent of `files`, so a rescan or a failed extraction
    /// never drops a rating. `dirty = 1` until the writer has landed exactly
    /// this value in the sidecar.
    ///
    /// `label_known` is false when the caller has not yet learned this
    /// path's label (e.g. a judgment made before the first `folder_entries`
    /// refresh, or before the sidecar parse has stored it). The `label`
    /// column is then left untouched rather than being set to `label`
    /// (typically `None`), so a later sidecar parse or read is still free to
    /// fill it in and the app-side row never asserts a label it does not
    /// actually know. A fresh row instead records `label_known = 0`, so that
    /// state survives a crash before the writer drains it and a replayed
    /// `dirty` row (see `dirty_rows`) does not wrongly assert "no label" to
    /// the writer.
    pub fn set_rating(
        &mut self,
        dir: &str,
        path: &str,
        rating: Option<i8>,
        flag: Flag,
        label: Option<&str>,
        label_known: bool,
    ) -> Result<(), String> {
        let flag = flag_code(flag);
        let result = if label_known {
            self.conn.execute(
                "INSERT INTO ratings (path, dir, rating, flag, label, label_known, dirty)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, 1)
                 ON CONFLICT (path) DO UPDATE SET dir = ?2, rating = ?3, flag = ?4, label = ?5,
                     label_known = 1, dirty = 1",
                params![path, dir, rating, flag, label],
            )
        } else {
            self.conn.execute(
                "INSERT INTO ratings (path, dir, rating, flag, label_known, dirty)
                 VALUES (?1, ?2, ?3, ?4, 0, 1)
                 ON CONFLICT (path) DO UPDATE SET dir = ?2, rating = ?3, flag = ?4,
                     dirty = 1",
                params![path, dir, rating, flag],
            )
        };
        result.map(|_| ()).map_err(|e| format!("{path}: {e}"))
    }

    /// Clear `dirty` and store the sidecar's stat, but only while the row
    /// still holds the judgment that was written: a keypress during the write
    /// leaves the row dirty so the newer value is written in turn.
    ///
    /// `stat` is `None` when no sidecar exists (clearing a rating on a file
    /// that never had one writes nothing).
    ///
    /// `label_known` is false when the label `set_rating` was given for this
    /// judgment was not asserted by the app: the writer instead kept the
    /// sidecar's own current label, passed here as `label` (the label
    /// actually resolved and written, see `sidecar::write`). That resolved
    /// label is now known to match the sidecar, so it and `label_known = 1`
    /// are stored regardless, which is what lets a later folder open (or a
    /// crash before this write even lands) treat the label as known instead
    /// of replaying it as "no label" and stripping the sidecar's own label
    /// (see `dirty_rows` and its caller).
    pub fn mark_written(
        &mut self,
        path: &str,
        rating: Option<i8>,
        flag: Flag,
        label: Option<&str>,
        label_known: bool,
        stat: Option<(i64, i64)>,
    ) -> Result<bool, String> {
        let flag = flag_code(flag);
        let (size, mtime_ns) = (stat.map(|s| s.0), stat.map(|s| s.1));
        let n = if label_known {
            self.conn.execute(
                "UPDATE ratings SET dirty = 0, xmp_size = ?2, xmp_mtime_ns = ?3
                 WHERE path = ?1 AND rating IS ?4 AND flag = ?5 AND label IS ?6",
                params![path, size, mtime_ns, rating, flag, label],
            )
        } else {
            self.conn.execute(
                "UPDATE ratings SET dirty = 0, xmp_size = ?2, xmp_mtime_ns = ?3,
                     label = ?6, label_known = 1
                 WHERE path = ?1 AND rating IS ?4 AND flag = ?5
                     AND (label_known = 0 OR label IS ?6)",
                params![path, size, mtime_ns, rating, flag, label],
            )
        }
        .map_err(|e| format!("{path}: {e}"))?;
        Ok(n > 0)
    }

    /// Record the stat of the sidecar a `Both` write did manage to write
    /// before its other sidecar failed and the retries ran out, without
    /// clearing `dirty`: the row still holds a judgment unwritten to that
    /// other sidecar. Guarded on the row still holding the judgment that was
    /// written and still being dirty, the same way `mark_written` is, so a
    /// keypress during the write or a write that already got cleaned up does
    /// not have its stat overwritten. Without this, the next folder open
    /// would see the freshly written sidecar's changed stat and mistake it
    /// for an external edit (see `sidecar::write`), storing it and clearing
    /// `dirty` while the other sidecar stays stale for good.
    pub fn mark_partial_write(
        &mut self,
        path: &str,
        rating: Option<i8>,
        flag: Flag,
        stat: (i64, i64),
    ) -> Result<bool, String> {
        let flag = flag_code(flag);
        let n = self
            .conn
            .execute(
                "UPDATE ratings SET xmp_size = ?2, xmp_mtime_ns = ?3
                 WHERE path = ?1 AND rating IS ?4 AND flag = ?5 AND dirty = 1",
                params![path, stat.0, stat.1, rating, flag],
            )
            .map_err(|e| format!("{path}: {e}"))?;
        Ok(n > 0)
    }

    /// Forget every sidecar the index has seen, for a sidecar format switch:
    /// clean rows are dropped so the next open reads the newly selected
    /// format, and dirty rows keep their judgment but lose the old format's
    /// stat, so the next open writes them into the new one. Their whole
    /// judgment is kept, including the flag, since both formats hold it.
    pub fn reset_sidecars(&mut self) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM ratings WHERE dirty = 0", [])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE ratings SET xmp_size = NULL, xmp_mtime_ns = NULL WHERE dirty = 1",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// The rows of `dir` whose judgment has not reached its sidecar yet,
    /// with each row's own `label_known` (see `set_rating`) rather than a
    /// blanket "known": a row created before the label was learned must
    /// still be replayed as unknown, or the writer would strip whatever
    /// label the sidecar already holds.
    pub fn dirty_rows(&self, dir: &str) -> Result<Vec<DirtyRow>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT path, rating, flag, label, label_known FROM ratings
                 WHERE dir = ?1 AND dirty = 1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![dir], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    flag_from_code(r.get(2)?),
                    r.get(3)?,
                    r.get(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())
    }

    /// Every `ratings` row of `dir`, dirty or not, in the shape `dirty_rows`
    /// hands out, for the counts of `Rewrite Sidecars from Index…`.
    pub fn rows_of(&self, dir: &str) -> Result<Vec<DirtyRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, rating, flag, label, label_known FROM ratings WHERE dir = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![dir], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    flag_from_code(r.get(2)?),
                    r.get(3)?,
                    r.get(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())
    }

    /// Mark the rows of `paths` in `dir` dirty, in one transaction, and
    /// return them as `dirty_rows` would, for `Rewrite Sidecars from Index…`
    /// to hand to the writer. The stored sidecar stat is kept: a write that
    /// lands replaces it through `mark_written`, and one that fails leaves
    /// the row dirty with the stat the next open compares against, so it is
    /// replayed rather than re-parsed.
    pub fn mark_dirty(&mut self, dir: &str, paths: &[String]) -> Result<Vec<DirtyRow>, String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare("UPDATE ratings SET dirty = 1 WHERE dir = ?1 AND path = ?2")
                .map_err(|e| e.to_string())?;
            for path in paths {
                stmt.execute(params![dir, path])
                    .map_err(|e| format!("{path}: {e}"))?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        let wanted: std::collections::HashSet<&str> = paths.iter().map(String::as_str).collect();
        Ok(self
            .dirty_rows(dir)?
            .into_iter()
            .filter(|(path, _, _, _, _)| wanted.contains(path.as_str()))
            .collect())
    }

    /// Clear the judgments of the rows of `paths` in `dir`, in one
    /// transaction, as `reconcile_sidecars` clears a row whose sidecar is
    /// gone, for `Delete Sidecars…` once their sidecars went to the Trash.
    /// The row is left clean too: the writer was drained before the delete,
    /// and a row still dirty would mint the deleted judgment again on the
    /// next open of the folder.
    pub fn clear_judgments(&mut self, dir: &str, paths: &[String]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare(
                    "UPDATE ratings SET rating = NULL, flag = 0, label = NULL,
                     label_known = 1, xmp_size = NULL, xmp_mtime_ns = NULL, dirty = 0
                     WHERE dir = ?1 AND path = ?2",
                )
                .map_err(|e| e.to_string())?;
            for path in paths {
                stmt.execute(params![dir, path])
                    .map_err(|e| format!("{path}: {e}"))?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// Every `ratings` row of `dir`, keyed by path: its flag, the sidecar
    /// stat it was stored with and whether it is dirty, for the tests that
    /// check what a run left in the index.
    #[cfg(test)]
    pub fn row_flags(
        &self,
        dir: &str,
    ) -> Result<std::collections::HashMap<String, RowFlag>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, flag, xmp_size, xmp_mtime_ns, dirty FROM ratings WHERE dir = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![dir], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    RowFlag {
                        flag: flag_from_code(r.get(1)?),
                        stat: (r.get(2)?, r.get(3)?),
                        dirty: r.get::<_, i64>(4)? != 0,
                    },
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())
    }

    /// Apply the reconciliation rules for the sidecars of `dir` on a folder
    /// open. `sidecars` pairs each listed file with its sidecar's stat, or
    /// `None` when the folder listing found none.
    ///
    /// The sidecar is the source of truth: a sidecar that changed under us is
    /// read back, never silently overwritten, even when the row is dirty. The
    /// rules are, per file: a sidecar whose stat differs from the stored one
    /// is parsed (which is also the first open of a folder Lightroom wrote);
    /// an unchanged stat costs nothing, which is what keeps the second open of
    /// a 5000-file folder cheap; a sidecar that is gone while the row is
    /// clean clears the rating, flag and label, because the truth is gone with it; and a row
    /// that is still dirty is left dirty, for `dirty_rows` to hand to the
    /// writer once the parsed sidecars have been stored.
    ///
    /// Returns the sidecars to read and parse outside the lock, each paired
    /// with the `dirty` flag observed here, along with the number of rows the
    /// gone sidecars cleared; the parse result goes back through
    /// `store_sidecar_ratings`, which must only apply the "sidecar wins over
    /// a dirty row" rule to the dirtiness this snapshot actually saw, not to
    /// a `set_rating` that lands while the lock is released for the parse.
    pub fn reconcile_sidecars(
        &mut self,
        dir: &str,
        sidecars: &[(String, Option<SidecarStat>)],
    ) -> Result<(Vec<SidecarToParse>, usize), String> {
        let known: std::collections::HashMap<String, RatingRow> = {
            let mut stmt = self
                .conn
                .prepare("SELECT path, xmp_size, xmp_mtime_ns, dirty FROM ratings WHERE dir = ?1")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![dir], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        RatingRow {
                            stat: (r.get(1)?, r.get(2)?),
                            dirty: r.get::<_, i64>(3)? != 0,
                        },
                    ))
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<rusqlite::Result<_>>()
                .map_err(|e| e.to_string())?
        };

        let mut to_parse = Vec::new();
        let mut cleared = 0;
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for (path, stat) in sidecars {
            let row = known.get(path);
            let dirty = row.is_some_and(|r| r.dirty);
            match stat {
                Some((sidecar, size, mtime_ns)) => {
                    if row.map(|r| r.stat) != Some((Some(*size), Some(*mtime_ns))) {
                        to_parse.push((path.clone(), (sidecar.clone(), *size, *mtime_ns), dirty));
                    }
                }
                None => {
                    if !dirty && row.is_some_and(|r| r.stat.0.is_some() || r.stat.1.is_some()) {
                        cleared += tx
                            .execute(
                                "UPDATE ratings SET rating = NULL, flag = 0, label = NULL,
                                 label_known = 1, xmp_size = NULL, xmp_mtime_ns = NULL
                                 WHERE path = ?1",
                                params![path],
                            )
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok((to_parse, cleared))
    }

    /// Store what parsing the sidecars of `dir` found: the rating, flag and label
    /// become the sidecar's and `dirty` is cleared, since the sidecar wins over an app
    /// edit that never reached disk.
    ///
    /// `rows` carries the `dirty` flag `reconcile_sidecars` observed for each
    /// path before the lock was released for the parse. The update is
    /// conditioned on the row's `dirty` still matching that snapshot: a
    /// `set_rating` landing in that window makes the row dirty when the
    /// snapshot was not, and such a row is left untouched here rather than
    /// having its fresh rating and `dirty` flag overwritten by a sidecar read
    /// taken before the edit.
    pub fn store_sidecar_ratings(
        &mut self,
        dir: &str,
        rows: &[ParsedSidecar],
    ) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for (path, rating, flag, label, size, mtime_ns, dirty) in rows {
            let flag = flag_code(*flag);
            tx.execute(
                "INSERT INTO ratings (path, dir, rating, flag, label, label_known, xmp_size, xmp_mtime_ns, dirty)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, 0)
                 ON CONFLICT (path) DO UPDATE SET dir = ?2, rating = ?3, flag = ?4, label = ?5,
                     label_known = 1, xmp_size = ?6, xmp_mtime_ns = ?7, dirty = 0
                 WHERE dirty = ?8",
                params![path, dir, rating, flag, label, size, mtime_ns, *dirty as i64],
            )
            .map_err(|e| format!("{path}: {e}"))?;
        }
        tx.commit().map_err(|e| e.to_string())
    }
}

/// The part of a `ratings` row the folder-open reconciliation looks at: the
/// stat of the sidecar as the app last saw it, and whether a judgment is
/// still waiting to be written.
struct RatingRow {
    stat: (Option<i64>, Option<i64>),
    dirty: bool,
}

/// The part of a `ratings` row `row_flags` hands out: the flag, the stat of
/// the sidecar it was stored with (`xmp_size`, `xmp_mtime_ns`) and whether
/// it is still waiting to be written.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowFlag {
    pub flag: Flag,
    pub stat: (Option<i64>, Option<i64>),
    pub dirty: bool,
}

/// What one file's sidecar looks like on disk, as seen by the single
/// directory listing a folder open does.
pub type SidecarStat = (PathBuf, i64, i64);

/// A sidecar `reconcile_sidecars` hands back to be parsed: `(path, stat,
/// dirty)`, the last being the `dirty` flag it observed.
pub type SidecarToParse = (String, SidecarStat, bool);

/// What parsing one file's sidecar found, for `store_sidecar_ratings`:
/// `(path, rating, flag, label, size, mtime_ns, dirty)`, the last being the
/// `dirty` flag `reconcile_sidecars` observed.
pub type ParsedSidecar = (String, Option<i8>, Flag, Option<String>, i64, i64, bool);

/// A row `dirty_rows` hands to the writer: `(path, rating, flag, label,
/// label_known)`.
pub type DirtyRow = (String, Option<i8>, Flag, Option<String>, bool);

/// Extract `files` (the first pass: thumbnails and metadata; the sharpness
/// score and the focus candidate cue are `run_faces_scan`'s) and write them
/// into `index` in batched transactions, reporting `(done, total, ready)` through `progress` at most every
/// `progress_interval` (`PROGRESS_INTERVAL` in the app), always on the first
/// file and once more after the trailing flush. `ready` holds the paths whose
/// rows a batch committed since the previous notification, so a path is
/// reported only once the index can answer for it; a batch whose write failed
/// is not reported at all. Its workers run at `Priority::BelowNormal`, so the
/// viewer's preview decode and the UI win the cores they contend for, and take
/// the files `focus` lists first.
///
/// Once `cancel` is set, files still in flight are abandoned at their next
/// pipeline stage: they get no row, are not in `ready` and are not counted in
/// `done` or the summary's `total`, so the next scan of the folder extracts
/// them again.
///
/// `on_item` runs on rayon worker threads and a panic there would abort the
/// whole scan, so nothing inside it unwraps: locks are taken with `lock` (which
/// ignores poisoning) and a failed write is counted like a failed file.
#[allow(clippy::too_many_arguments)]
pub fn run_scan<P>(
    index: &Mutex<Index>,
    dir: &str,
    files: &[FileStat],
    threads: usize,
    focus: &ScanFocus,
    cancel: &AtomicBool,
    progress_interval: Duration,
    progress: P,
) -> ScanSummary
where
    P: Fn(usize, usize, Vec<String>) + Send + Sync,
{
    let total = files.len();
    let paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    let pending: Mutex<Vec<(FileStat, Result<Entry, Failure>)>> = Mutex::new(Vec::new());
    let done = AtomicUsize::new(0);
    let errors = AtomicUsize::new(0);
    let last = Mutex::new(None::<Instant>);
    let ready: Mutex<Vec<String>> = Mutex::new(Vec::new());

    let flush = |batch: Vec<(FileStat, Result<Entry, Failure>)>| {
        if batch.is_empty() {
            return;
        }
        if let Err(e) = lock(index).write_batch(dir, &batch) {
            log::error!("failed to write a batch for {dir}: {e}");
            // Files that failed extraction are already counted in `on_item`;
            // only the ones that would otherwise have landed as a success are
            // newly lost here.
            let newly_failed = batch.iter().filter(|(_, r)| r.is_ok()).count();
            errors.fetch_add(newly_failed, Ordering::Relaxed);
            return;
        }
        lock(&ready).extend(
            batch
                .iter()
                .map(|(file, _)| file.path.to_string_lossy().into_owned()),
        );
    };

    let on_item = |i: usize, result: Result<Entry, Failure>| {
        if result.is_err() {
            errors.fetch_add(1, Ordering::Relaxed);
        }
        let batch = {
            let mut pending = lock(&pending);
            pending.push((files[i].clone(), result));
            if pending.len() >= BATCH {
                std::mem::take(&mut *pending)
            } else {
                Vec::new()
            }
        };
        flush(batch);

        let done = done.fetch_add(1, Ordering::Relaxed) + 1;
        let now = Instant::now();
        let due = {
            let mut last = lock(&last);
            if last.is_none_or(|t| now.duration_since(t) >= progress_interval) {
                *last = Some(now);
                true
            } else {
                false
            }
        };
        if due {
            progress(done, total, std::mem::take(&mut *lock(&ready)));
        }
    };

    let started = Instant::now();
    match extract_all(
        &paths,
        threads,
        Priority::BelowNormal,
        focus,
        on_item,
        cancel,
    ) {
        Ok(Some(e)) => log::warn!("scan of {dir} ran at normal priority: {e}"),
        Ok(None) => {}
        Err(e) => log::error!("scan of {dir} failed: {e}"),
    }
    flush(std::mem::take(&mut *lock(&pending)));
    let done_count = done.load(Ordering::Relaxed);
    progress(done_count, total, std::mem::take(&mut *lock(&ready)));

    let summary = ScanSummary {
        total: done_count,
        errors: errors.load(Ordering::Relaxed),
    };
    log::info!(
        "scan extract: dir={dir} files={total} done={} errors={} threads={threads} canceled={} in {}ms",
        summary.total,
        summary.errors,
        cancel.load(Ordering::Relaxed),
        started.elapsed().as_millis()
    );
    summary
}

/// Run the second pass (`extract_analysis`) over `paths` and store each
/// in-focus probability and sharpness score in `index`, the way `run_scan`
/// runs the first:
/// `BATCH`-sized transactions through `write_faces`, `progress(done, total,
/// ready)` at most every `progress_interval`, always on the first file and once more after the
/// trailing flush, where `ready` holds the files a batch committed since the
/// previous notification. A file `extract_analysis` could not read counts as an
/// error and is stored with no probability and no score at `FACES_VERSION`, so
/// it is not
/// retried until that version moves. A file abandoned mid-pipeline by
/// `cancel` is neither written nor counted, the way `run_scan` treats it, so
/// it keeps its old `faces_extractor`. The same no-panic rule as `run_scan`
/// holds for `on_item`. Its workers run at `Priority::Lowest`, below the
/// first pass's, and take the files `focus` lists first.
#[allow(clippy::too_many_arguments)]
pub fn run_faces_scan<P>(
    index: &Mutex<Index>,
    dir: &str,
    paths: &[String],
    threads: usize,
    focus: &ScanFocus,
    cancel: &AtomicBool,
    progress_interval: Duration,
    progress: P,
) -> ScanSummary
where
    P: Fn(usize, usize, Vec<FaceReady>) + Send + Sync,
{
    let total = paths.len();
    let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let pending: Mutex<Vec<(String, Analysis, bool)>> = Mutex::new(Vec::new());
    let done = AtomicUsize::new(0);
    let errors = AtomicUsize::new(0);
    let last = Mutex::new(None::<Instant>);
    let ready: Mutex<Vec<FaceReady>> = Mutex::new(Vec::new());

    let flush = |batch: Vec<(String, Analysis, bool)>| {
        if batch.is_empty() {
            return;
        }
        let rows: Vec<FacesRow> = batch
            .iter()
            .map(|(path, analysis, _)| FacesRow {
                path: path.clone(),
                eye_focus: analysis.cue.eye_focus,
                sharpness: analysis.sharpness,
                eyes_ear: analysis.cue.eyes_ear,
                pose: analysis.cue.pose.map(EyesPose::from),
                eye_offset: analysis.cue.eye_offset,
                edge_gap: analysis.cue.edge_gap,
            })
            .collect();
        if let Err(e) = lock(index).write_faces(&rows) {
            log::error!("failed to write a faces batch for {dir}: {e}");
            // Unreadable files are already counted in `on_item`.
            let newly_failed = batch.iter().filter(|(_, _, ok)| *ok).count();
            errors.fetch_add(newly_failed, Ordering::Relaxed);
            return;
        }
        lock(&ready).extend(rows.into_iter().map(|row| FaceReady {
            eyes: StoredEyes::new(row.eyes_ear, row.pose, row.eye_offset, row.edge_gap),
            candidate: candidate(row.eye_focus),
            path: row.path,
            eye_focus: row.eye_focus,
            sharpness: row.sharpness,
        }));
    };

    let on_item = |i: usize, result: Result<Analysis, String>| {
        let (analysis, ok) = match result {
            Ok(analysis) => (analysis, true),
            Err(_) => {
                errors.fetch_add(1, Ordering::Relaxed);
                (Analysis::default(), false)
            }
        };
        let batch = {
            let mut pending = lock(&pending);
            pending.push((paths[i].clone(), analysis, ok));
            if pending.len() >= BATCH {
                std::mem::take(&mut *pending)
            } else {
                Vec::new()
            }
        };
        flush(batch);

        let done = done.fetch_add(1, Ordering::Relaxed) + 1;
        let now = Instant::now();
        let due = {
            let mut last = lock(&last);
            if last.is_none_or(|t| now.duration_since(t) >= progress_interval) {
                *last = Some(now);
                true
            } else {
                false
            }
        };
        if due {
            progress(done, total, std::mem::take(&mut *lock(&ready)));
        }
    };

    let started = Instant::now();
    match extract_analysis_all(
        &path_bufs,
        threads,
        Priority::Lowest,
        focus,
        on_item,
        cancel,
    ) {
        Ok(Some(e)) => log::warn!("faces scan of {dir} ran at normal priority: {e}"),
        Ok(None) => {}
        Err(e) => log::error!("faces scan of {dir} failed: {e}"),
    }
    flush(std::mem::take(&mut *lock(&pending)));
    let done_count = done.load(Ordering::Relaxed);
    progress(done_count, total, std::mem::take(&mut *lock(&ready)));

    let summary = ScanSummary {
        total: done_count,
        errors: errors.load(Ordering::Relaxed),
    };
    log::info!(
        "scan faces: dir={dir} files={total} done={} errors={} threads={threads} canceled={} in {}ms",
        summary.total,
        summary.errors,
        cancel.load(Ordering::Relaxed),
        started.elapsed().as_millis()
    );
    summary
}

/// Run the third pass (`embed_all`) of `model` over `todo` and store each
/// embedding in `index`, the way `run_faces_scan` runs the second:
/// `BATCH`-sized transactions through `write_embeddings`, `progress(done,
/// total)` at most every `progress_interval`, always on the first file and
/// once more after the trailing flush. A file that could not be embedded
/// counts as an error and is stored with no vector, so it is not retried
/// until its stat or `EMBED_VERSION` moves; a file abandoned by `cancel` is
/// neither written nor counted. The same no-panic rule as `run_scan` holds
/// for `on_item`. Its workers run at `Priority::Lowest`, as the second
/// pass's, and take the files `focus` lists first.
#[allow(clippy::too_many_arguments)]
pub fn run_embed_scan<P>(
    index: &Mutex<Index>,
    dir: &str,
    todo: &[EmbedTodo],
    model: &Model,
    threads: usize,
    focus: &ScanFocus,
    cancel: &AtomicBool,
    progress_interval: Duration,
    progress: P,
) -> ScanSummary
where
    P: Fn(usize, usize) + Send + Sync,
{
    let total = todo.len();
    let paths: Vec<PathBuf> = todo.iter().map(|t| PathBuf::from(&t.path)).collect();
    let pending: Mutex<Vec<(EmbedRow, bool)>> = Mutex::new(Vec::new());
    let done = AtomicUsize::new(0);
    let errors = AtomicUsize::new(0);
    let last = Mutex::new(None::<Instant>);

    let flush = |batch: Vec<(EmbedRow, bool)>| {
        if batch.is_empty() {
            return;
        }
        let embedded = batch.iter().filter(|(_, ok)| *ok).count();
        let rows: Vec<EmbedRow> = batch.into_iter().map(|(row, _)| row).collect();
        if let Err(e) = lock(index).write_embeddings(dir, &rows) {
            log::error!("failed to write an embeddings batch for {dir}: {e}");
            // Files that could not be embedded are already counted in `on_item`.
            errors.fetch_add(embedded, Ordering::Relaxed);
        }
    };

    let on_item = |i: usize, result: Result<Vec<f32>, String>| {
        let ok = result.is_ok();
        if !ok {
            errors.fetch_add(1, Ordering::Relaxed);
        }
        let batch = {
            let mut pending = lock(&pending);
            pending.push((
                EmbedRow {
                    file: todo[i].clone(),
                    vec: result.ok(),
                },
                ok,
            ));
            if pending.len() >= BATCH {
                std::mem::take(&mut *pending)
            } else {
                Vec::new()
            }
        };
        flush(batch);

        let done = done.fetch_add(1, Ordering::Relaxed) + 1;
        let now = Instant::now();
        let due = {
            let mut last = lock(&last);
            if last.is_none_or(|t| now.duration_since(t) >= progress_interval) {
                *last = Some(now);
                true
            } else {
                false
            }
        };
        if due {
            progress(done, total);
        }
    };

    let started = Instant::now();
    match embed_all(
        &paths,
        threads,
        Priority::Lowest,
        focus,
        model,
        on_item,
        cancel,
    ) {
        Ok(Some(e)) => log::warn!("embed scan of {dir} ran at normal priority: {e}"),
        Ok(None) => {}
        Err(e) => log::error!("embed scan of {dir} failed: {e}"),
    }
    flush(std::mem::take(&mut *lock(&pending)));
    let done_count = done.load(Ordering::Relaxed);
    progress(done_count, total);

    let summary = ScanSummary {
        total: done_count,
        errors: errors.load(Ordering::Relaxed),
    };
    log::info!(
        "scan embed: dir={dir} files={total} done={} errors={} threads={threads} canceled={} in {}ms",
        summary.total,
        summary.errors,
        cancel.load(Ordering::Relaxed),
        started.elapsed().as_millis()
    );
    summary
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use riffle_core::arw::{FocusFrame, FocusLocation, Shot};
    use riffle_core::candidate::candidate_probability;
    use riffle_core::eyes::{closed_probability, EYES_CLOSED_EAR};
    use std::collections::HashSet;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-index-{name}-{}", std::process::id()));
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

    fn open(dir: &Path) -> Index {
        Index::open(&dir.join("index.sqlite")).unwrap()
    }

    fn file(dir: &Path, name: &str, bytes: &[u8]) -> FileStat {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        stat(&path).unwrap()
    }

    fn entry() -> Entry {
        Entry {
            orientation: 6,
            shot: Shot {
                capture_time: Some("2026:09:13 09:23:33".to_string()),
                subsec: Some("122".to_string()),
                focus: Some(FocusLocation {
                    sensor_w: 7008,
                    sensor_h: 4672,
                    x: 3613,
                    y: 1732,
                }),
                make: Some("SONY".to_string()),
                model: Some("ILCE-7M5".to_string()),
                lens_model: Some("FE 50mm F1.4 GM".to_string()),
                exposure_time: Some(Rational { num: 10, den: 2500 }),
                f_number: Some(Rational { num: 14, den: 10 }),
                iso: Some(800),
                focal_length: Some(Rational { num: 500, den: 10 }),
                ..Shot::default()
            },
            thumbnail: vec![0xff, 0xd8, 0xff, 0xd9],
        }
    }

    #[test]
    fn a_written_row_survives_re_opening_the_database() {
        let dir = temp_dir("reopen");
        let a = file(&dir, "a.ARW", b"a");
        let mut index = open(&dir);
        index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
        drop(index);

        let mut index = open(&dir);
        assert!(index
            .reconcile("d", std::slice::from_ref(&a))
            .unwrap()
            .0
            .is_empty());
        let entries = index.entries("d").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].orientation, 6);
        assert_eq!(entries[0].subsec.as_deref(), Some("122"));
        assert_eq!(entries[0].focus.unwrap().point.unwrap().x, 3613);
        assert!(entries[0].has_thumb);
        assert_eq!(index.thumbnail(&entries[0].path).unwrap().0, 6);
        assert_eq!(entries[0].exif, Some(exif(&entry().shot)));
        assert!(entries[0].error.is_none());
        let e = entries[0].exif.as_ref().unwrap();
        assert_eq!(e.camera.as_deref(), Some("SONY ILCE-7M5"));
        assert_eq!(e.lens.as_deref(), Some("FE 50mm F1.4 GM"));
        assert_eq!(e.aperture.as_ref().unwrap().label, "f/1.4");
        assert_eq!(e.shutter.as_ref().unwrap().label, "1/250");
        assert_eq!(e.iso.as_ref().unwrap().label, "800");
        assert_eq!(e.focal_length.as_ref().unwrap().label, "50 mm");

        remove_temp_dir(&dir);
    }

    #[test]
    fn an_estimated_aperture_round_trips() {
        let dir = temp_dir("estimated");
        let a = file(&dir, "a.ARW", b"a");
        let mut index = open(&dir);
        let mut e = entry();
        e.shot = Shot {
            estimated_f_number: Some(2.0),
            ..Shot::default()
        };
        index.write_batch("d", &[(a, Ok(e))]).unwrap();
        let exif = index.entries("d").unwrap()[0].exif.clone().unwrap();
        assert_eq!(exif.aperture.unwrap().label, "f/2 (est.)");
        assert_eq!(exif.camera, None);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v3_database_drops_its_files_rows_and_keeps_its_dirty_ratings() {
        let dir = temp_dir("migrate-v3");
        let db = dir.join("index.sqlite");
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch(
                "CREATE TABLE files (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     size INTEGER NOT NULL,
                     mtime_ns INTEGER NOT NULL,
                     orientation INTEGER,
                     capture_time TEXT,
                     subsec TEXT,
                     focus_w INTEGER,
                     focus_h INTEGER,
                     focus_x INTEGER,
                     focus_y INTEGER,
                     thumb BLOB,
                     error TEXT
                 );
                 INSERT INTO files (path, dir, size, mtime_ns, thumb)
                     VALUES ('/a.ARW', 'd', 1, 1, x'ffd8ffd9');
                 CREATE TABLE ratings (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     rating INTEGER,
                     pick INTEGER NOT NULL DEFAULT 0,
                     xmp_size INTEGER,
                     xmp_mtime_ns INTEGER,
                     dirty INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT INTO ratings (path, dir, rating, pick, dirty)
                     VALUES ('/a.ARW', 'd', 4, 1, 1);
                 PRAGMA user_version = 3;",
            )
            .unwrap();
        }

        let index = Index::open(&db).unwrap();
        assert!(index.entries("d").unwrap().is_empty());
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/a.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        drop(index);
        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index.dirty_rows("d").unwrap().len(), 1);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v6_database_drops_its_files_rows_and_keeps_its_ratings() {
        let dir = temp_dir("migrate-v6");
        let db = dir.join("index.sqlite");
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch(
                "CREATE TABLE files (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     size INTEGER NOT NULL,
                     mtime_ns INTEGER NOT NULL,
                     orientation INTEGER,
                     capture_time TEXT,
                     subsec TEXT,
                     focus_w INTEGER,
                     focus_h INTEGER,
                     focus_x INTEGER,
                     focus_y INTEGER,
                     thumb BLOB,
                     error TEXT,
                     make TEXT,
                     model TEXT,
                     lens TEXT,
                     f_num INTEGER,
                     f_den INTEGER,
                     f_estimated REAL,
                     exposure_num INTEGER,
                     exposure_den INTEGER,
                     iso INTEGER,
                     focal_num INTEGER,
                     focal_den INTEGER
                 );
                 INSERT INTO files (path, dir, size, mtime_ns, thumb)
                     VALUES ('/a.ARW', 'd', 1, 1, x'ffd8ffd9');
                 CREATE TABLE ratings (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     rating INTEGER,
                     pick INTEGER NOT NULL DEFAULT 0,
                     label TEXT,
                     label_known INTEGER NOT NULL DEFAULT 1,
                     xmp_size INTEGER,
                     xmp_mtime_ns INTEGER,
                     dirty INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT INTO ratings (path, dir, rating, pick, label, dirty)
                     VALUES ('/a.ARW', 'd', 4, 1, 'Red', 1);
                 PRAGMA user_version = 6;",
            )
            .unwrap();
        }

        let index = Index::open(&db).unwrap();
        assert!(index.entries("d").unwrap().is_empty());
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [(
                "/a.ARW".to_string(),
                Some(4),
                Flag::Pick,
                Some("Red".to_string()),
                true
            )]
        );
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v10_database_turns_its_picks_and_rejects_into_flags_and_keeps_its_files() {
        let dir = temp_dir("migrate-v10");
        let db = dir.join("index.sqlite");
        let a = synthetic(&dir, 0);
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index
                .conn
                .execute_batch(
                    "ALTER TABLE ratings RENAME COLUMN flag TO pick;
                     ALTER TABLE files DROP COLUMN extractor;
                     ALTER TABLE files DROP COLUMN frame_w;
                     ALTER TABLE files DROP COLUMN frame_h;
                     ALTER TABLE files DROP COLUMN manual_focus;
                     ALTER TABLE files DROP COLUMN eye_focus;
                     ALTER TABLE files DROP COLUMN faces_extractor;
                     INSERT INTO ratings (path, dir, rating, pick, dirty, xmp_size, xmp_mtime_ns) VALUES
                         ('/rejected.ARW', 'd', -1, 0, 1, 10, 20),
                         ('/picked.ARW', 'd', 3, 1, 0, 10, 20),
                         ('/plain.ARW', 'd', 2, 0, 0, 10, 20);
                     ALTER TABLE files DROP COLUMN eyes_ear;
                     ALTER TABLE files DROP COLUMN pose_yaw;
                     ALTER TABLE files DROP COLUMN pose_pitch;
                     ALTER TABLE files DROP COLUMN pose_roll;
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 10;",
                )
                .unwrap();
        }

        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index.entries("d").unwrap().len(), 1, "files are kept");
        let rows: Vec<(String, Option<i8>, i64, i64)> = index
            .conn
            .prepare("SELECT path, rating, flag, dirty FROM ratings ORDER BY path")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            rows,
            [
                ("/picked.ARW".to_string(), Some(3), 1, 0),
                ("/plain.ARW".to_string(), Some(2), 0, 0),
                ("/rejected.ARW".to_string(), None, 2, 1),
            ]
        );
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rejected.ARW".to_string(), None, Flag::Reject, None, true)]
        );
        let stats: Vec<(String, Option<i64>, Option<i64>)> = index
            .conn
            .prepare("SELECT path, xmp_size, xmp_mtime_ns FROM ratings ORDER BY path")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            stats,
            [
                ("/picked.ARW".to_string(), None, None),
                ("/plain.ARW".to_string(), None, None),
                ("/rejected.ARW".to_string(), Some(10), Some(20)),
            ],
            "the stat of clean rows is invalidated so reconcile_sidecars re-parses \
             them with the real readers, but a dirty row keeps its stat"
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_sharpness_score_round_trips() {
        let dir = temp_dir("sharpness");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let mut index = open(&dir);
        index
            .write_batch("d", &[(a.clone(), Ok(entry())), (b.clone(), Ok(entry()))])
            .unwrap();
        assert!(index
            .entries("d")
            .unwrap()
            .iter()
            .all(|e| e.sharpness.is_none()));
        index
            .write_faces(&[
                faces_row(&a.path.to_string_lossy(), None, Some(123.5)),
                faces_row(&b.path.to_string_lossy(), None, None),
            ])
            .unwrap();
        let entries = index.entries("d").unwrap();
        assert_eq!(entries[0].sharpness, Some(123.5));
        assert_eq!(entries[1].sharpness, None);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_changed_mtime_invalidates_the_row() {
        let dir = temp_dir("mtime");
        let a = file(&dir, "a.ARW", b"a");
        let mut index = open(&dir);
        index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();

        let changed = FileStat {
            mtime_ns: a.mtime_ns + 1,
            ..a.clone()
        };
        assert_eq!(index.reconcile("d", &[changed]).unwrap().0.len(), 1);
        // The stale row must be gone too, not just reported as "to scan":
        // otherwise `entries` would keep serving it until the rescan finishes.
        assert!(index.entries("d").unwrap().is_empty());

        index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
        let bigger = FileStat { size: 99, ..a };
        assert_eq!(index.reconcile("d", &[bigger]).unwrap().0.len(), 1);
        assert!(index.entries("d").unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_row_at_an_older_extractor_version_is_re_extracted() {
        let dir = temp_dir("extractor");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let mut index = open(&dir);
        index
            .write_batch(
                "d",
                &[(a.clone(), Ok(entry())), (b.clone(), Err("broken".into()))],
            )
            .unwrap();
        index
            .conn
            .execute_batch("UPDATE files SET extractor = 0")
            .unwrap();

        let (stale, _) = index.reconcile("d", &[a.clone(), b.clone()]).unwrap();
        assert_eq!(
            stale.iter().map(|f| f.path.clone()).collect::<Vec<_>>(),
            [a.path.clone(), b.path.clone()],
            "both the success row and the error row are returned"
        );
        assert!(index.entries("d").unwrap().is_empty());

        index
            .write_batch(
                "d",
                &[(a.clone(), Ok(entry())), (b.clone(), Err("broken".into()))],
            )
            .unwrap();
        assert_eq!(index.entries("d").unwrap().len(), 2);
        assert!(index.reconcile("d", &[a, b]).unwrap().0.is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn rows_at_the_current_extractor_version_are_kept() {
        let dir = temp_dir("extractor-current");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let mut index = open(&dir);
        index
            .write_batch(
                "d",
                &[(a.clone(), Ok(entry())), (b.clone(), Err("broken".into()))],
            )
            .unwrap();

        assert!(index.reconcile("d", &[a, b]).unwrap().0.is_empty());
        assert_eq!(index.entries("d").unwrap().len(), 2);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v11_database_gains_the_extractor_column_and_keeps_its_files_and_ratings() {
        let dir = temp_dir("migrate-v11");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating, flag, dirty) VALUES
                         ('/rated.ARW', 'd', 4, 1, 1);
                     ALTER TABLE files DROP COLUMN extractor;
                     ALTER TABLE files DROP COLUMN frame_w;
                     ALTER TABLE files DROP COLUMN frame_h;
                     ALTER TABLE files DROP COLUMN manual_focus;
                     ALTER TABLE files DROP COLUMN eye_focus;
                     ALTER TABLE files DROP COLUMN faces_extractor;
                     ALTER TABLE files DROP COLUMN eyes_ear;
                     ALTER TABLE files DROP COLUMN pose_yaw;
                     ALTER TABLE files DROP COLUMN pose_pitch;
                     ALTER TABLE files DROP COLUMN pose_roll;
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 11;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index.entries("d").unwrap().len(), 1, "files are kept");
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rated.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        assert_eq!(index.reconcile("d", &[a]).unwrap().0.len(), 1);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v12_database_gains_the_frame_and_manual_focus_columns_and_keeps_its_files_and_ratings() {
        let dir = temp_dir("migrate-v12");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating, flag, dirty) VALUES
                         ('/rated.ARW', 'd', 4, 1, 1);
                     UPDATE files SET extractor = 1;
                     ALTER TABLE files DROP COLUMN frame_w;
                     ALTER TABLE files DROP COLUMN frame_h;
                     ALTER TABLE files DROP COLUMN manual_focus;
                     ALTER TABLE files DROP COLUMN eye_focus;
                     ALTER TABLE files DROP COLUMN faces_extractor;
                     ALTER TABLE files DROP COLUMN eyes_ear;
                     ALTER TABLE files DROP COLUMN pose_yaw;
                     ALTER TABLE files DROP COLUMN pose_pitch;
                     ALTER TABLE files DROP COLUMN pose_roll;
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 12;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let entries = index.entries("d").unwrap();
        assert_eq!(entries.len(), 1, "files are kept");
        let focus = entries[0].focus.unwrap();
        assert!(focus.point.unwrap().frame.is_none());
        assert!(!focus.manual_focus);
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rated.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        assert_eq!(index.reconcile("d", &[a]).unwrap().0.len(), 1);

        remove_temp_dir(&dir);
    }

    #[test]
    fn the_af_frame_and_manual_focus_round_trip() {
        let dir = temp_dir("frame");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let c = file(&dir, "c.DNG", b"c");
        let j = file(&dir, "d.jpg", b"j");
        let mut index = open(&dir);
        let mut framed = entry();
        framed.shot.focus_frame = Some(FocusFrame {
            width: 832,
            height: 624,
        });
        framed.shot.focus_mode = Some(0);
        let mut no_point = entry();
        no_point.shot.focus = None;
        index
            .write_batch(
                "d",
                &[
                    (a, Ok(framed)),
                    (b, Ok(entry())),
                    (c, Ok(no_point.clone())),
                    (j, Ok(no_point)),
                ],
            )
            .unwrap();

        let entries = index.entries("d").unwrap();
        let framed = entries[0].focus.unwrap();
        assert_eq!(
            framed.point.unwrap().frame,
            Some(FocusSize {
                width: 832,
                height: 624
            })
        );
        assert!(framed.manual_focus);
        let plain = entries[1].focus.unwrap();
        assert!(plain.point.unwrap().frame.is_none());
        assert!(!plain.manual_focus);
        let json = serde_json::to_value(plain).unwrap();
        assert_eq!(json["point"]["x"], 3613);
        assert!(json["point"]["frame"].is_null());
        assert_eq!(json["candidate"], "unknown");
        let pointless = entries[2].focus.unwrap();
        assert!(
            pointless.point.is_none(),
            "a RAW with no AF point has a cue"
        );
        let json = serde_json::to_value(pointless).unwrap();
        assert!(json["point"].is_null());
        assert_eq!(json["manual_focus"], false);
        assert!(json["eye_focus"].is_null());
        assert!(
            entries[3].focus.is_none(),
            "a JPEG with no AF point has none"
        );

        remove_temp_dir(&dir);
    }

    fn faces_extractor(index: &Index, path: &Path) -> i64 {
        index
            .conn
            .query_row(
                "SELECT faces_extractor FROM files WHERE path = ?1",
                params![path.to_string_lossy()],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn faces_row(path: &str, eye_focus: Option<f64>, sharpness: Option<f64>) -> FacesRow {
        FacesRow {
            path: path.to_string(),
            eye_focus,
            sharpness,
            eyes_ear: None,
            pose: None,
            eye_offset: None,
            edge_gap: None,
        }
    }

    #[test]
    fn the_eyes_ear_and_pose_round_trip_and_derive_the_eye_state() {
        let dir = temp_dir("eyes-ear");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let mut index = open(&dir);
        let dir_name = dir.to_string_lossy().into_owned();
        index
            .write_batch(
                &dir_name,
                &[(a.clone(), Ok(entry())), (b.clone(), Ok(entry()))],
            )
            .unwrap();
        let a_path = a.path.to_string_lossy().into_owned();
        let b_path = b.path.to_string_lossy().into_owned();
        let pose = EyesPose {
            yaw: -12.5,
            pitch: 3.25,
            roll: 0.5,
        };
        let closed = EYES_CLOSED_EAR - 0.01;
        index
            .write_faces(&[
                FacesRow {
                    eyes_ear: Some(closed),
                    pose: Some(pose),
                    eye_offset: Some(0.0625),
                    edge_gap: Some(-0.125),
                    ..faces_row(&a_path, Some(0.9), None)
                },
                FacesRow {
                    eyes_ear: Some(0.3),
                    ..faces_row(&b_path, Some(0.1), None)
                },
            ])
            .unwrap();
        let focus = index.entry(&a_path).unwrap().unwrap().focus.unwrap();
        let judged = Eyes::from_ear(closed);
        assert_eq!(
            focus.eyes,
            StoredEyes {
                eyes_ear: Some(closed),
                eyes: Some(EyeState::Closed),
                eyes_closed: Some(judged.probability),
                pose: Some(pose),
                eye_offset: Some(0.0625),
                edge_gap: Some(-0.125),
            }
        );
        let json = serde_json::to_value(focus).unwrap();
        assert_eq!(json["eyes_ear"], closed);
        assert_eq!(json["eyes"], "closed");
        assert_eq!(json["eyes_closed"], judged.probability);
        assert_eq!(
            json["pose"],
            serde_json::json!({ "yaw": -12.5, "pitch": 3.25, "roll": 0.5 })
        );
        assert_eq!(json["eye_offset"], 0.0625);
        assert_eq!(json["edge_gap"], -0.125);
        assert_eq!(json["candidate"], "candidate");
        let open = index.entry(&b_path).unwrap().unwrap().focus.unwrap();
        assert_eq!(open.eyes.eyes, Some(EyeState::Open));
        assert_eq!(open.eyes.pose, None);
        let json = serde_json::to_value(open).unwrap();
        assert_eq!(json["eyes"], "open");
        assert!(json["pose"].is_null());

        index
            .write_faces(&[faces_row(&a_path, None, None)])
            .unwrap();
        let focus = index.entry(&a_path).unwrap().unwrap().focus.unwrap();
        assert_eq!(focus.eyes, StoredEyes::default());
        let json = serde_json::to_value(focus).unwrap();
        assert_eq!(json["eyes"], "unknown");
        assert!(json["eyes_ear"].is_null());
        assert!(json["eyes_closed"].is_null());
        assert!(json["eye_offset"].is_null() && json["edge_gap"].is_null());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_face_ready_serializes_the_stored_eyes_next_to_the_cue() {
        let ready = FaceReady {
            path: "/d/a.ARW".into(),
            eye_focus: Some(0.875),
            candidate: candidate(Some(0.875)),
            sharpness: Some(12.0),
            eyes: StoredEyes::new(
                Some(0.3),
                Some(EyesPose {
                    yaw: 1.0,
                    pitch: 2.0,
                    roll: 3.0,
                }),
                Some(0.05),
                Some(1.5),
            ),
        };
        let json = serde_json::to_value(&ready).unwrap();
        assert_eq!(json["path"], "/d/a.ARW");
        assert_eq!(json["candidate"], "candidate");
        assert_eq!(json["eyes_ear"], 0.3);
        assert_eq!(json["eyes"], "open");
        assert_eq!(json["eyes_closed"], closed_probability(0.3));
        assert_eq!(
            json["pose"],
            serde_json::json!({ "yaw": 1.0, "pitch": 2.0, "roll": 3.0 })
        );
        assert_eq!(json["eye_offset"], 0.05);
        assert_eq!(json["edge_gap"], 1.5);
        let none = FaceReady {
            eyes: StoredEyes::default(),
            ..ready
        };
        let json = serde_json::to_value(&none).unwrap();
        assert_eq!(json["eyes"], "unknown");
        assert!(json["eyes_ear"].is_null() && json["eyes_closed"].is_null());
        assert!(json["pose"].is_null());
        assert!(json["eye_offset"].is_null() && json["edge_gap"].is_null());
    }

    #[test]
    fn a_faces_version_6_row_is_listed_again_with_its_first_pass_untouched() {
        let dir = temp_dir("faces-v6");
        let a = file(&dir, "a.ARW", b"a");
        let j = file(&dir, "b.jpg", b"j");
        let mut index = open(&dir);
        let dir_name = dir.to_string_lossy().into_owned();
        index
            .write_batch(
                &dir_name,
                &[(a.clone(), Ok(entry())), (j.clone(), Ok(entry()))],
            )
            .unwrap();
        let a_path = a.path.to_string_lossy().into_owned();
        let j_path = j.path.to_string_lossy().into_owned();
        index
            .write_faces(&[
                faces_row(&a_path, Some(0.5), Some(7.0)),
                FacesRow {
                    eyes_ear: Some(0.3),
                    pose: Some(EyesPose {
                        yaw: 1.0,
                        pitch: 2.0,
                        roll: 3.0,
                    }),
                    ..faces_row(&j_path, Some(0.5), Some(7.0))
                },
            ])
            .unwrap();
        index
            .conn
            .execute("UPDATE files SET faces_extractor = 6", [])
            .unwrap();
        let first_pass = |index: &Index, path: &str| -> (Option<Vec<u8>>, Option<String>, i64) {
            index
                .conn
                .query_row(
                    "SELECT thumb, make, extractor FROM files WHERE path = ?1",
                    params![path],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap()
        };
        let before = first_pass(&index, &a_path);

        assert_eq!(
            index.faces_todo(&dir_name).unwrap(),
            std::slice::from_ref(&a_path)
        );
        assert_eq!(first_pass(&index, &a_path), before);
        assert_eq!(before.2, EXTRACTOR_VERSION);
        let jpeg = index.entry(&j_path).unwrap().unwrap().focus.unwrap();
        assert_eq!(jpeg.eyes, StoredEyes::default(), "the JPEG row is reset");
        assert_eq!(faces_extractor(&index, &j.path), FACES_VERSION);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v17_database_gains_the_eyes_and_pose_columns_and_keeps_its_files_ratings_and_folders() {
        let dir = temp_dir("migrate-v17");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index.reconcile("d", std::slice::from_ref(&a)).unwrap();
            index
                .write_faces(&[faces_row(&a.path.to_string_lossy(), Some(0.875), Some(5.0))])
                .unwrap();
            index.set_last_viewed("d", "/d/a.ARW").unwrap();
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating, flag, dirty) VALUES
                         ('/rated.ARW', 'd', 4, 1, 1);
                     ALTER TABLE files DROP COLUMN eyes_ear;
                     ALTER TABLE files DROP COLUMN pose_yaw;
                     ALTER TABLE files DROP COLUMN pose_pitch;
                     ALTER TABLE files DROP COLUMN pose_roll;
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     UPDATE files SET faces_extractor = 6;
                     DROP TABLE views;
                     PRAGMA user_version = 17;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let columns: Vec<String> = index
            .conn
            .prepare("SELECT name FROM pragma_table_info('files')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        for c in [
            "eyes_ear",
            "pose_yaw",
            "pose_pitch",
            "pose_roll",
            "eye_offset",
            "edge_gap",
        ] {
            assert!(columns.iter().any(|n| n == c), "{c}");
        }
        let entries = index.entries("d").unwrap();
        assert_eq!(entries.len(), 1, "files are kept");
        let focus = entries[0].focus.unwrap();
        assert_eq!(focus.eye_focus, Some(0.875));
        assert_eq!(entries[0].sharpness, Some(5.0));
        assert_eq!(focus.eyes, StoredEyes::default());
        assert_eq!(
            index.faces_todo("d").unwrap(),
            [a.path.to_string_lossy().into_owned()]
        );
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rated.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        assert_eq!(index.last_viewed("d").unwrap().as_deref(), Some("/d/a.ARW"));
        assert!(index.reconcile("d", &[a]).unwrap().0.is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v18_database_gains_the_eye_offset_and_edge_gap_columns_and_keeps_its_files_ratings_and_folders(
    ) {
        let dir = temp_dir("migrate-v18");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index.reconcile("d", std::slice::from_ref(&a)).unwrap();
            index
                .write_faces(&[FacesRow {
                    eyes_ear: Some(0.3),
                    ..faces_row(&a.path.to_string_lossy(), Some(0.875), Some(5.0))
                }])
                .unwrap();
            index.set_last_viewed("d", "/d/a.ARW").unwrap();
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating, flag, dirty) VALUES
                         ('/rated.ARW', 'd', 4, 1, 1);
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     UPDATE files SET faces_extractor = 7;
                     DROP TABLE views;
                     PRAGMA user_version = 18;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let columns: Vec<String> = index
            .conn
            .prepare("SELECT name FROM pragma_table_info('files')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        for c in ["eye_offset", "edge_gap"] {
            assert!(columns.iter().any(|n| n == c), "{c}");
        }
        let entries = index.entries("d").unwrap();
        assert_eq!(entries.len(), 1, "files are kept");
        let focus = entries[0].focus.unwrap();
        assert_eq!(focus.eye_focus, Some(0.875));
        assert_eq!(entries[0].sharpness, Some(5.0));
        assert_eq!(focus.eyes, StoredEyes::new(Some(0.3), None, None, None));
        assert_eq!(
            index.faces_todo("d").unwrap(),
            [a.path.to_string_lossy().into_owned()]
        );
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rated.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        assert_eq!(index.last_viewed("d").unwrap().as_deref(), Some("/d/a.ARW"));
        assert!(index.reconcile("d", &[a]).unwrap().0.is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v19_database_gains_the_embeddings_table_and_keeps_its_files_ratings_and_folders() {
        let dir = temp_dir("migrate-v19");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index.reconcile("d", std::slice::from_ref(&a)).unwrap();
            index.set_last_viewed("d", "/d/a.ARW").unwrap();
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating, flag, dirty) VALUES
                         ('/rated.ARW', 'd', 4, 1, 1);
                     DROP TABLE embeddings;
                     DROP TABLE views;
                     PRAGMA user_version = 19;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index.entries("d").unwrap().len(), 1, "files are kept");
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rated.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        assert_eq!(index.last_viewed("d").unwrap().as_deref(), Some("/d/a.ARW"));
        assert_eq!(index.embeddings_missing("d").unwrap(), [todo_of(&a)]);
        assert!(index.reconcile("d", &[a]).unwrap().0.is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v20_database_gains_views_and_keeps_its_folders() {
        let dir = temp_dir("migrate-v20");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index.reconcile("d", std::slice::from_ref(&a)).unwrap();
            index.set_last_viewed("d", "/d/a.ARW").unwrap();
            index
                .conn
                .execute_batch(
                    "DROP TABLE views;
                     PRAGMA user_version = 20;",
                )
                .unwrap();
        }

        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index.entries("d").unwrap().len(), 1, "files are kept");
        assert_eq!(index.last_viewed("d").unwrap().as_deref(), Some("/d/a.ARW"));
        assert!(index.viewed_files("d").unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    /// The column names of `folders`, in order.
    fn folder_columns(index: &Index) -> Vec<String> {
        index
            .conn
            .prepare("SELECT name FROM pragma_table_info('folders')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn a_v21_database_gains_views_and_drops_culling_finished() {
        let dir = temp_dir("migrate-v21");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index.reconcile("d", std::slice::from_ref(&a)).unwrap();
            index
                .conn
                .execute_batch(
                    "ALTER TABLE folders ADD COLUMN culling_finished INTEGER NOT NULL DEFAULT 0;
                     UPDATE folders SET culling_finished = 1 WHERE dir = 'd';
                     DROP TABLE views;
                     PRAGMA user_version = 21;",
                )
                .unwrap();
        }

        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index.entries("d").unwrap().len(), 1, "files are kept");
        assert!(index.viewed_files("d").unwrap().is_empty());
        assert_eq!(folder_columns(&index), ["dir", "opened_at", "last_viewed"]);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v22_database_drops_the_culling_columns_and_keeps_its_folders() {
        let dir = temp_dir("migrate-v22");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index.reconcile("d", std::slice::from_ref(&a)).unwrap();
            index.set_last_viewed("d", "/d/a.ARW").unwrap();
            index
                .conn
                .execute_batch(
                    "ALTER TABLE folders ADD COLUMN culling_finished INTEGER NOT NULL DEFAULT 0;
                     ALTER TABLE folders ADD COLUMN culling_manual INTEGER NOT NULL DEFAULT 0;
                     UPDATE folders SET culling_finished = 1, culling_manual = 1;
                     PRAGMA user_version = 22;",
                )
                .unwrap();
        }

        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index.entries("d").unwrap().len(), 1, "files are kept");
        assert_eq!(index.last_viewed("d").unwrap().as_deref(), Some("/d/a.ARW"));
        assert_eq!(folder_columns(&index), ["dir", "opened_at", "last_viewed"]);

        remove_temp_dir(&dir);
    }

    #[test]
    fn mark_viewed_round_trips_and_keeps_the_first_view() {
        let dir = temp_dir("views");
        let mut index = open(&dir);
        assert!(index.viewed_files("/d").unwrap().is_empty());
        index
            .mark_viewed("/d", &["/d/b.ARW".to_string(), "/d/a.ARW".to_string()])
            .unwrap();
        index
            .conn
            .execute("UPDATE views SET viewed_at = 1", [])
            .unwrap();
        index
            .mark_viewed("/d", &["/d/a.ARW".to_string(), "/d/c.ARW".to_string()])
            .unwrap();
        index
            .mark_viewed("/other", &["/other/a.ARW".to_string()])
            .unwrap();
        assert_eq!(
            index.viewed_files("/d").unwrap(),
            ["/d/a.ARW", "/d/b.ARW", "/d/c.ARW"]
        );
        let first: i64 = index
            .conn
            .query_row(
                "SELECT viewed_at FROM views WHERE path = '/d/a.ARW'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(first, 1, "a second view keeps the first viewed_at");

        remove_temp_dir(&dir);
    }

    #[test]
    fn renames_carry_the_views() {
        let dir = temp_dir("views-rename");
        let mut index = open(&dir);
        let sep = std::path::MAIN_SEPARATOR;
        let (old, new) = ("/old".to_string(), "/new".to_string());
        let sub = format!("{old}{sep}sub");
        index
            .mark_viewed(&old, &[format!("{old}{sep}a.ARW")])
            .unwrap();
        index
            .mark_viewed(&sub, &[format!("{sub}{sep}b.ARW")])
            .unwrap();
        index
            .mark_viewed(&new, &[format!("{new}{sep}stale.ARW")])
            .unwrap();

        index.rename_dir(&old, &new).unwrap();
        assert!(index.viewed_files(&old).unwrap().is_empty());
        assert_eq!(
            index.viewed_files(&new).unwrap(),
            [format!("{new}{sep}a.ARW")]
        );
        assert_eq!(
            index.viewed_files(&format!("{new}{sep}sub")).unwrap(),
            [format!("{new}{sep}sub{sep}b.ARW")]
        );

        index
            .rename_file(&format!("{new}{sep}a.ARW"), &format!("{new}{sep}z.ARW"))
            .unwrap();
        assert_eq!(
            index.viewed_files(&new).unwrap(),
            [format!("{new}{sep}z.ARW")]
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn eviction_keeps_the_views_and_clear_drops_them() {
        let dir = temp_dir("views-evict");
        let mut index = open(&dir);
        index
            .write_batch("old", &[(synthetic(&dir, 0), Ok(entry()))])
            .unwrap();
        set_opened_at(&index, "old", NOW - 31 * DAY);
        index
            .mark_viewed("old", &["old/a.ARW".to_string()])
            .unwrap();

        let policy = EvictPolicy {
            max_age_secs: MAX_AGE_SECS,
            max_bytes: 0,
        };
        assert_eq!(index.evict(NOW, policy).unwrap().folders, 1);
        assert_eq!(index.viewed_files("old").unwrap(), ["old/a.ARW"]);

        index.clear().unwrap();
        assert!(index.viewed_files("old").unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn the_rank_dirs_are_the_folders_holding_a_view_or_a_reject() {
        let dir = temp_dir("rank-dirs");
        let mut index = open(&dir);
        set_opened_at(&index, "/untouched", NOW);
        index
            .mark_viewed("/viewed", &["/viewed/a.ARW".to_string()])
            .unwrap();
        index
            .conn
            .execute_batch(
                "INSERT INTO ratings (path, dir, flag) VALUES
                     ('/rejects/a.ARW', '/rejects', 2),
                     ('/rejects/b.ARW', '/rejects', 2),
                     ('/picks/a.ARW', '/picks', 1);",
            )
            .unwrap();
        assert_eq!(index.rank_dirs().unwrap(), ["/rejects", "/viewed"]);

        remove_temp_dir(&dir);
    }

    #[test]
    fn rank_rows_read_the_current_second_pass_and_the_valid_embedding_only() {
        let dir = temp_dir("rank-rows");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let mut index = open(&dir);
        index
            .write_batch("d", &[(b.clone(), Ok(entry())), (a.clone(), Ok(entry()))])
            .unwrap();
        let path = |f: &FileStat| f.path.to_string_lossy().into_owned();
        index
            .write_faces(&[
                FacesRow {
                    path: path(&a),
                    eye_focus: Some(0.95),
                    sharpness: Some(120.0),
                    eyes_ear: Some(0.3),
                    pose: Some(EyesPose {
                        yaw: 10.0,
                        pitch: -5.0,
                        roll: 2.0,
                    }),
                    eye_offset: Some(0.05),
                    edge_gap: Some(0.2),
                },
                FacesRow {
                    path: path(&b),
                    eye_focus: Some(0.5),
                    sharpness: Some(80.0),
                    eyes_ear: None,
                    pose: None,
                    eye_offset: None,
                    edge_gap: None,
                },
            ])
            .unwrap();
        index
            .conn
            .execute(
                "UPDATE files SET faces_extractor = ?2 WHERE path = ?1",
                params![path(&b), FACES_VERSION - 1],
            )
            .unwrap();
        index
            .write_embeddings(
                "d",
                &[
                    embedded(&a, Some(vec![0.6, 0.8])),
                    EmbedRow {
                        file: EmbedTodo {
                            size: b.size + 1,
                            ..todo_of(&b)
                        },
                        vec: Some(vec![1.0]),
                    },
                ],
            )
            .unwrap();
        index
            .conn
            .execute(
                "INSERT INTO ratings (path, dir, flag) VALUES (?1, 'd', 1)",
                params![path(&a)],
            )
            .unwrap();
        index.mark_viewed("d", &[path(&b)]).unwrap();

        let rows = index.rank_rows("d").unwrap();
        assert_eq!(
            rows.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
            [path(&a), path(&b)],
            "in file-name order"
        );
        let (ra, rb) = (&rows[0], &rows[1]);
        assert_eq!((ra.flag, rb.flag), (Flag::Pick, Flag::None));
        assert_eq!((ra.viewed, rb.viewed), (false, true));
        assert_eq!(ra.frame.name, "a.ARW");
        assert_eq!(
            ra.frame.capture_time.as_deref(),
            Some("2026:09:13 09:23:33")
        );
        assert_eq!(ra.frame.subsec.as_deref(), Some("122"));
        assert_eq!(
            (ra.frame.eye_focus, ra.frame.sharpness, ra.frame.eyes_ear),
            (Some(0.95), Some(120.0), Some(0.3))
        );
        assert_eq!(
            ra.frame.pose,
            Some(Pose {
                yaw: 10.0,
                pitch: -5.0,
                roll: 2.0
            })
        );
        assert_eq!(
            (ra.frame.eye_offset, ra.frame.edge_gap),
            (Some(0.05), Some(0.2))
        );
        assert_eq!(ra.frame.vec, Some(vec![0.6, 0.8]));
        assert_eq!(
            (rb.frame.eye_focus, rb.frame.sharpness),
            (None, None),
            "a row behind FACES_VERSION has no second-pass values"
        );
        assert_eq!(rb.frame.vec, None, "an embedding stale by stat is left out");

        remove_temp_dir(&dir);
    }

    fn todo_of(file: &FileStat) -> EmbedTodo {
        EmbedTodo {
            path: file.path.to_string_lossy().into_owned(),
            size: file.size,
            mtime_ns: file.mtime_ns,
        }
    }

    fn embedded(file: &FileStat, vec: Option<Vec<f32>>) -> EmbedRow {
        EmbedRow {
            file: todo_of(file),
            vec,
        }
    }

    /// The stored rows of `embeddings`: `(path, dir, version, vec)`.
    fn embedding_rows(index: &Index) -> Vec<(String, String, i64, Option<Vec<f32>>)> {
        index
            .conn
            .prepare("SELECT path, dir, version, vec FROM embeddings ORDER BY path")
            .unwrap()
            .query_map([], |r| {
                let vec: Option<Vec<u8>> = r.get(3)?;
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    vec.map(|b| {
                        b.chunks_exact(4)
                            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                            .collect()
                    }),
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn embeddings_missing_lists_the_raw_rows_without_a_valid_embedding() {
        let dir = temp_dir("embed-missing");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let c = file(&dir, "c.ARW", b"c");
        let d = file(&dir, "d.ARW", b"d");
        let jpeg = file(&dir, "e.JPG", b"e");
        let broken = file(&dir, "f.ARW", b"f");
        let mut index = open(&dir);
        index
            .write_batch(
                "d",
                &[
                    (a.clone(), Ok(entry())),
                    (b.clone(), Ok(entry())),
                    (c.clone(), Ok(entry())),
                    (d.clone(), Ok(entry())),
                    (jpeg.clone(), Ok(entry())),
                    (broken.clone(), Err("unreadable".into())),
                ],
            )
            .unwrap();
        assert_eq!(
            index.embeddings_missing("d").unwrap(),
            [todo_of(&a), todo_of(&b), todo_of(&c), todo_of(&d)],
            "a JPEG and a failed extraction are never listed"
        );

        index
            .write_embeddings(
                "d",
                &[
                    embedded(&a, Some(vec![0.6, 0.8])),
                    embedded(&b, None),
                    EmbedRow {
                        file: EmbedTodo {
                            mtime_ns: c.mtime_ns + 1,
                            ..todo_of(&c)
                        },
                        vec: Some(vec![1.0]),
                    },
                    embedded(&d, Some(vec![1.0])),
                ],
            )
            .unwrap();
        index
            .conn
            .execute(
                "UPDATE embeddings SET version = ?2 WHERE path = ?1",
                params![todo_of(&d).path, EMBED_VERSION - 1],
            )
            .unwrap();
        assert_eq!(
            index.embeddings_missing("d").unwrap(),
            [todo_of(&c), todo_of(&d)],
            "stale by stat or by version; a failed embedding is not retried"
        );

        index
            .write_embeddings("d", &[embedded(&c, Some(vec![0.0, 1.0]))])
            .unwrap();
        index
            .write_embeddings("d", &[embedded(&d, Some(vec![1.0, 0.0]))])
            .unwrap();
        assert!(index.embeddings_missing("d").unwrap().is_empty());
        let rows = embedding_rows(&index);
        assert_eq!(rows.len(), 4, "an upsert replaces the row");
        assert_eq!(rows[0].3, Some(vec![0.6, 0.8]));
        assert_eq!(rows[1].3, None);
        assert!(rows.iter().all(|r| r.1 == "d" && r.2 == EMBED_VERSION));

        remove_temp_dir(&dir);
    }

    #[test]
    fn embeddings_returns_the_valid_vectors_only() {
        let dir = temp_dir("embed-valid");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let c = file(&dir, "c.ARW", b"c");
        let d = file(&dir, "d.ARW", b"d");
        let mut index = open(&dir);
        index
            .write_batch(
                "d",
                &[
                    (a.clone(), Ok(entry())),
                    (b.clone(), Ok(entry())),
                    (c.clone(), Ok(entry())),
                    (d.clone(), Ok(entry())),
                ],
            )
            .unwrap();
        index
            .write_embeddings(
                "d",
                &[
                    embedded(&a, Some(vec![0.6, 0.8])),
                    embedded(&b, None),
                    EmbedRow {
                        file: EmbedTodo {
                            size: c.size + 1,
                            ..todo_of(&c)
                        },
                        vec: Some(vec![1.0]),
                    },
                    embedded(&d, Some(vec![1.0])),
                ],
            )
            .unwrap();
        index
            .conn
            .execute(
                "UPDATE embeddings SET version = ?2 WHERE path = ?1",
                params![todo_of(&d).path, EMBED_VERSION - 1],
            )
            .unwrap();
        assert_eq!(
            index.embeddings("d").unwrap(),
            [(todo_of(&a).path, vec![0.6, 0.8])],
            "a failed, stale-by-stat or stale-by-version embedding is left out"
        );
        assert!(index.embeddings("other").unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn reconcile_drops_the_embeddings_of_changed_and_gone_files_only() {
        let dir = temp_dir("embed-reconcile");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let c = file(&dir, "c.ARW", b"c");
        let mut index = open(&dir);
        let rows = [a.clone(), b.clone(), c.clone()].map(|f| (f, Ok(entry())));
        index.write_batch("d", &rows).unwrap();
        index
            .write_embeddings(
                "d",
                &[
                    embedded(&a, Some(vec![1.0])),
                    embedded(&b, Some(vec![1.0])),
                    embedded(&c, Some(vec![1.0])),
                ],
            )
            .unwrap();
        index
            .conn
            .execute(
                "UPDATE files SET extractor = 0 WHERE path = ?1",
                params![todo_of(&a).path],
            )
            .unwrap();
        let b_changed = FileStat {
            mtime_ns: b.mtime_ns + 1,
            ..b.clone()
        };
        index.reconcile("d", &[a.clone(), b_changed]).unwrap();
        let paths: Vec<String> = embedding_rows(&index).into_iter().map(|r| r.0).collect();
        assert_eq!(
            paths,
            [todo_of(&a).path],
            "a re-extracted row keeps its embedding"
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn eviction_and_clear_drop_the_embeddings_and_strays() {
        let dir = temp_dir("embed-evict");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let mut index = open(&dir);
        index
            .write_batch("old", &[(a.clone(), Ok(entry()))])
            .unwrap();
        index
            .write_batch("new", &[(b.clone(), Ok(entry()))])
            .unwrap();
        index.reconcile("old", std::slice::from_ref(&a)).unwrap();
        index.reconcile("new", std::slice::from_ref(&b)).unwrap();
        set_opened_at(&index, "old", 0);
        let stray = FileStat {
            path: PathBuf::from("/renamed-away/c.ARW"),
            size: 1,
            mtime_ns: 1,
        };
        index
            .write_embeddings("old", &[embedded(&a, Some(vec![1.0]))])
            .unwrap();
        index
            .write_embeddings("new", &[embedded(&b, Some(vec![1.0]))])
            .unwrap();
        index
            .write_embeddings("/renamed-away", &[embedded(&stray, Some(vec![1.0]))])
            .unwrap();

        let summary = index.evict(now_secs(), EVICT_POLICY).unwrap();
        assert_eq!(summary.folders, 1);
        let dirs: Vec<String> = embedding_rows(&index).into_iter().map(|r| r.1).collect();
        assert_eq!(dirs, ["new"], "the evicted folder's and the stray go");

        index.clear().unwrap();
        assert!(embedding_rows(&index).is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn used_bytes_counts_the_embedding_pages() {
        let dir = temp_dir("embed-bytes");
        let mut index = open(&dir);
        let before = index.used_bytes().unwrap();
        let rows: Vec<EmbedRow> = (0..100)
            .map(|i| EmbedRow {
                file: EmbedTodo {
                    path: format!("/d/{i}.ARW"),
                    size: 1,
                    mtime_ns: 1,
                },
                vec: Some(vec![0.5; 768]),
            })
            .collect();
        index.write_embeddings("d", &rows).unwrap();
        assert!(index.used_bytes().unwrap() >= before + 100 * 768 * 4);

        remove_temp_dir(&dir);
    }

    #[test]
    fn renames_carry_the_embeddings() {
        let dir = temp_dir("embed-rename");
        let mut index = open(&dir);
        let sep = std::path::MAIN_SEPARATOR;
        let row = |path: String| EmbedRow {
            file: EmbedTodo {
                path,
                size: 1,
                mtime_ns: 1,
            },
            vec: Some(vec![1.0]),
        };
        let old = format!("{sep}p{sep}old");
        let new = format!("{sep}p{sep}new");
        let sub = format!("{old}{sep}sub");
        index
            .write_embeddings(&old, &[row(format!("{old}{sep}a.ARW"))])
            .unwrap();
        index
            .write_embeddings(&sub, &[row(format!("{sub}{sep}b.ARW"))])
            .unwrap();
        index
            .write_embeddings(&new, &[row(format!("{new}{sep}stale.ARW"))])
            .unwrap();

        index.rename_dir(&old, &new).unwrap();
        let rows: Vec<(String, String)> = embedding_rows(&index)
            .into_iter()
            .map(|r| (r.0, r.1))
            .collect();
        let new_sub = format!("{new}{sep}sub");
        assert_eq!(
            rows,
            [
                (format!("{new}{sep}a.ARW"), new.clone()),
                (format!("{new_sub}{sep}b.ARW"), new_sub.clone()),
            ]
        );

        index
            .rename_file(&format!("{new}{sep}a.ARW"), &format!("{new}{sep}c.ARW"))
            .unwrap();
        let paths: Vec<String> = embedding_rows(&index).into_iter().map(|r| r.0).collect();
        assert_eq!(
            paths,
            [format!("{new}{sep}c.ARW"), format!("{new_sub}{sep}b.ARW")]
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn the_eye_focus_round_trips_and_derives_the_candidate_state() {
        let dir = temp_dir("eye-sharpness");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let c = file(&dir, "c.ARW", b"c");
        let mut index = open(&dir);
        let mut manual = entry();
        manual.shot.focus_mode = Some(0);
        let mut no_af = entry();
        no_af.shot.focus = None;
        let dir_name = dir.to_string_lossy().into_owned();
        index
            .write_batch(
                &dir_name,
                &[
                    (a.clone(), Ok(entry())),
                    (b.clone(), Ok(manual)),
                    (c.clone(), Ok(no_af)),
                ],
            )
            .unwrap();
        let a_path = a.path.to_string_lossy().into_owned();
        let b_path = b.path.to_string_lossy().into_owned();
        let c_path = c.path.to_string_lossy().into_owned();

        assert_eq!(
            index.faces_todo(&dir_name).unwrap(),
            [a_path.clone(), b_path.clone(), c_path.clone()],
            "a RAW without a trusted AF point is listed for its score"
        );
        for file in [&a, &b, &c] {
            assert_eq!(faces_extractor(&index, &file.path), 0);
        }

        index
            .write_faces(&[
                faces_row(&a_path, Some(0.875), None),
                faces_row(&b_path, None, Some(1.0)),
                faces_row(&c_path, Some(0.875), Some(2.0)),
            ])
            .unwrap();
        let no_af = index.entry(&c_path).unwrap().unwrap().focus.unwrap();
        assert!(no_af.point.is_none());
        assert_eq!(
            (no_af.eye_focus, no_af.candidate),
            (Some(0.875), FocusCandidate::Candidate),
            "a frame with no AF point carries the cue of its judged face"
        );
        let focus = index.entry(&a_path).unwrap().unwrap().focus.unwrap();
        assert_eq!(focus.eye_focus, Some(0.875));
        assert_eq!(focus.candidate, FocusCandidate::Candidate);
        let json = serde_json::to_value(focus).unwrap();
        assert_eq!(json["candidate"], "candidate");
        assert_eq!(json["eye_focus"], 0.875);
        assert!(json.get("eye_sharpness").is_none());
        for (p, state) in [
            (candidate_probability(), FocusCandidate::Candidate),
            (
                candidate_probability().next_down(),
                FocusCandidate::NotCandidate,
            ),
        ] {
            index
                .write_faces(&[faces_row(&a_path, Some(p), None)])
                .unwrap();
            let focus = index.entry(&a_path).unwrap().unwrap().focus.unwrap();
            assert_eq!(focus.eye_focus, Some(p), "the probability round-trips");
            assert_eq!(focus.candidate, state);
        }
        let manual = index.entries(&dir_name).unwrap()[1].focus.unwrap();
        assert_eq!(manual.eye_focus, None);
        assert_eq!(
            serde_json::to_value(manual).unwrap()["candidate"],
            "unknown"
        );
        assert!(index.faces_todo(&dir_name).unwrap().is_empty());

        index
            .write_batch(&dir_name, &[(a.clone(), Ok(entry()))])
            .unwrap();
        let focus = index.entry(&a_path).unwrap().unwrap().focus.unwrap();
        assert_eq!(focus.eye_focus, None);
        assert_eq!(focus.candidate, FocusCandidate::Unknown);
        assert_eq!(
            index.faces_todo(&dir_name).unwrap(),
            std::slice::from_ref(&a_path)
        );
        index
            .write_faces(&[faces_row(&a_path, None, None)])
            .unwrap();
        assert_eq!(faces_extractor(&index, &a.path), FACES_VERSION);
        assert!(index.faces_todo(&dir_name).unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v14_database_swaps_face_catch_for_the_eye_focus_columns_and_keeps_its_files_and_ratings() {
        let dir = temp_dir("migrate-v14");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating, flag, dirty) VALUES
                         ('/rated.ARW', 'd', 4, 1, 1);
                     ALTER TABLE files DROP COLUMN eye_focus;
                     ALTER TABLE files DROP COLUMN faces_extractor;
                     ALTER TABLE files ADD COLUMN face_catch INTEGER NOT NULL DEFAULT 0;
                     UPDATE files SET face_catch = 1;
                     ALTER TABLE files DROP COLUMN eyes_ear;
                     ALTER TABLE files DROP COLUMN pose_yaw;
                     ALTER TABLE files DROP COLUMN pose_pitch;
                     ALTER TABLE files DROP COLUMN pose_roll;
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 14;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let columns: Vec<String> = index
            .conn
            .prepare("SELECT name FROM pragma_table_info('files')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!columns.iter().any(|c| c == "face_catch"));
        assert!(columns.iter().any(|c| c == "eye_focus"));
        assert!(columns.iter().any(|c| c == "faces_extractor"));
        let entries = index.entries("d").unwrap();
        assert_eq!(entries.len(), 1, "files are kept");
        let focus = entries[0].focus.unwrap();
        assert_eq!(focus.eye_focus, None);
        assert_eq!(focus.candidate, FocusCandidate::Unknown);
        assert_eq!(faces_extractor(&index, &a.path), 0);
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rated.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        assert!(index.reconcile("d", &[a]).unwrap().0.is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v15_database_swaps_eye_sharpness_for_eye_focus_and_keeps_its_files_ratings_and_folders() {
        let dir = temp_dir("migrate-v15");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index.reconcile("d", std::slice::from_ref(&a)).unwrap();
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating, flag, dirty) VALUES
                         ('/rated.ARW', 'd', 4, 1, 1);
                     ALTER TABLE files DROP COLUMN eye_focus;
                     ALTER TABLE files ADD COLUMN eye_sharpness REAL;
                     UPDATE files SET eye_sharpness = 120.0, faces_extractor = 2;
                     ALTER TABLE files DROP COLUMN eyes_ear;
                     ALTER TABLE files DROP COLUMN pose_yaw;
                     ALTER TABLE files DROP COLUMN pose_pitch;
                     ALTER TABLE files DROP COLUMN pose_roll;
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 15;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let columns: Vec<String> = index
            .conn
            .prepare("SELECT name FROM pragma_table_info('files')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!columns.iter().any(|c| c == "eye_sharpness"));
        assert!(columns.iter().any(|c| c == "eye_focus"));
        let entries = index.entries("d").unwrap();
        assert_eq!(entries.len(), 1, "files are kept");
        let focus = entries[0].focus.unwrap();
        assert_eq!(focus.eye_focus, None, "the stale eye sharpness is gone");
        assert_eq!(focus.candidate, FocusCandidate::Unknown);
        assert_eq!(faces_extractor(&index, &a.path), 2);
        assert_eq!(
            index.faces_todo("d").unwrap(),
            [a.path.to_string_lossy().into_owned()]
        );
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rated.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        let folders: i64 = index
            .conn
            .query_row("SELECT COUNT(*) FROM folders WHERE dir = 'd'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(folders, 1, "folders are kept");
        assert!(index.reconcile("d", &[a]).unwrap().0.is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v13_database_gains_the_eye_focus_columns_and_keeps_its_files_and_ratings() {
        let dir = temp_dir("migrate-v13");
        let db = dir.join("index.sqlite");
        let a = file(&dir, "a.ARW", b"a");
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating, flag, dirty) VALUES
                         ('/rated.ARW', 'd', 4, 1, 1);
                     UPDATE files SET extractor = 3;
                     ALTER TABLE files DROP COLUMN eye_focus;
                     ALTER TABLE files DROP COLUMN faces_extractor;
                     ALTER TABLE files DROP COLUMN eyes_ear;
                     ALTER TABLE files DROP COLUMN pose_yaw;
                     ALTER TABLE files DROP COLUMN pose_pitch;
                     ALTER TABLE files DROP COLUMN pose_roll;
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 13;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let entries = index.entries("d").unwrap();
        assert_eq!(entries.len(), 1, "files are kept");
        let focus = entries[0].focus.unwrap();
        assert_eq!(focus.eye_focus, None);
        assert_eq!(focus.candidate, FocusCandidate::Unknown);
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/rated.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        assert_eq!(index.reconcile("d", &[a]).unwrap().0.len(), 1);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_deleted_file_loses_its_row_on_the_next_reconciliation() {
        let dir = temp_dir("deleted");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let mut index = open(&dir);
        index
            .write_batch("d", &[(a.clone(), Ok(entry())), (b, Ok(entry()))])
            .unwrap();
        assert_eq!(index.entries("d").unwrap().len(), 2);

        assert!(index
            .reconcile("d", std::slice::from_ref(&a))
            .unwrap()
            .0
            .is_empty());
        let entries = index.entries("d").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, a.path.to_string_lossy());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_deletion_only_reconcile_reports_the_removed_rows() {
        let dir = temp_dir("removed-count");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let c = file(&dir, "c.ARW", b"c");
        let mut index = open(&dir);
        index
            .write_batch(
                "d",
                &[(a.clone(), Ok(entry())), (b, Ok(entry())), (c, Ok(entry()))],
            )
            .unwrap();

        let (todo, removed) = index.reconcile("d", &[a]).unwrap();
        assert!(todo.is_empty());
        assert_eq!(removed, 2);

        remove_temp_dir(&dir);
    }

    #[test]
    fn row_flags_reads_the_flag_stat_and_dirtiness_of_one_dir() {
        let dir = temp_dir("row-flags");
        let mut index = open(&dir);
        index
            .set_rating("d", "/d/a.ARW", None, Flag::Reject, None, true)
            .unwrap();
        index
            .mark_written("/d/a.ARW", None, Flag::Reject, None, true, Some((42, 7)))
            .unwrap();
        index
            .set_rating("d", "/d/b.ARW", Some(2), Flag::Pick, None, true)
            .unwrap();
        index
            .set_rating("e", "/e/c.ARW", None, Flag::Reject, None, true)
            .unwrap();

        let rows = index.row_flags("d").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows["/d/a.ARW"],
            RowFlag {
                flag: Flag::Reject,
                stat: (Some(42), Some(7)),
                dirty: false,
            }
        );
        assert_eq!(
            rows["/d/b.ARW"],
            RowFlag {
                flag: Flag::Pick,
                stat: (None, None),
                dirty: true,
            }
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_gone_sidecar_over_a_clean_row_reports_one_cleared_row() {
        let dir = temp_dir("cleared-count");
        let mut index = open(&dir);
        index
            .set_rating("d", "/a.ARW", Some(3), Flag::None, None, true)
            .unwrap();
        assert!(index
            .mark_written("/a.ARW", Some(3), Flag::None, None, true, Some((42, 7)))
            .unwrap());

        let (to_parse, cleared) = index
            .reconcile_sidecars("d", &[("/a.ARW".to_string(), None)])
            .unwrap();
        assert!(to_parse.is_empty());
        assert_eq!(cleared, 1);

        remove_temp_dir(&dir);
    }

    #[test]
    fn an_unchanged_folder_reports_no_removed_or_cleared_rows() {
        let dir = temp_dir("unchanged-count");
        let a = file(&dir, "a.ARW", b"a");
        let path = a.path.to_string_lossy().into_owned();
        let mut index = open(&dir);
        index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
        index
            .set_rating("d", &path, Some(3), Flag::None, None, true)
            .unwrap();
        assert!(index
            .mark_written(&path, Some(3), Flag::None, None, true, Some((42, 7)))
            .unwrap());

        let (todo, removed) = index.reconcile("d", &[a]).unwrap();
        assert!(todo.is_empty());
        assert_eq!(removed, 0);
        let (to_parse, cleared) = index
            .reconcile_sidecars("d", &[(path, Some((dir.join("a.xmp"), 42, 7)))])
            .unwrap();
        assert!(to_parse.is_empty());
        assert_eq!(cleared, 0);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_rating_survives_a_rescan_and_shows_up_on_the_entry() {
        let dir = temp_dir("rating");
        let a = file(&dir, "a.ARW", b"a");
        let path = a.path.to_string_lossy().into_owned();
        let mut index = open(&dir);
        index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
        index
            .set_rating("d", &path, Some(2), Flag::Reject, None, true)
            .unwrap();
        let judged = &index.entries("d").unwrap()[0];
        assert_eq!((judged.rating, judged.flag), (Some(2), Flag::Reject));

        // A reconcile that drops the `files` row must leave `ratings` alone:
        // the sidecar, not the scan, is what a rating belongs to.
        assert!(index.reconcile("d", &[]).unwrap().0.is_empty());
        assert!(index.entries("d").unwrap().is_empty());
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [(path.clone(), Some(2), Flag::Reject, None, true)]
        );

        index.write_batch("d", &[(a, Ok(entry()))]).unwrap();
        let judged = &index.entries("d").unwrap()[0];
        assert_eq!((judged.rating, judged.flag), (Some(2), Flag::Reject));

        remove_temp_dir(&dir);
    }

    #[test]
    fn entry_reads_one_row_by_path() {
        let dir = temp_dir("entry");
        let a = file(&dir, "a.ARW", b"a");
        let b = file(&dir, "b.ARW", b"b");
        let path = b.path.to_string_lossy().into_owned();
        let mut index = open(&dir);
        index
            .write_batch("d", &[(a, Ok(entry())), (b, Ok(entry()))])
            .unwrap();
        index
            .set_rating("d", &path, Some(4), Flag::Pick, Some("Red"), true)
            .unwrap();

        let row = index.entry(&path).unwrap().unwrap();
        assert_eq!(row.path, path);
        assert_eq!(
            (row.rating, row.flag, row.label.as_deref()),
            (Some(4), Flag::Pick, Some("Red"))
        );
        assert!(index.entry("d/missing.ARW").unwrap().is_none());

        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_everything_nulls_the_rating_flag_and_label_of_a_row() {
        let dir = temp_dir("clear-all");
        let mut index = open(&dir);
        index
            .set_rating("d", "/a.ARW", Some(4), Flag::Pick, Some("Red"), true)
            .unwrap();
        index
            .mark_written("/a.ARW", Some(4), Flag::Pick, Some("Red"), true, None)
            .unwrap();

        index
            .set_rating("d", "/a.ARW", None, Flag::None, None, true)
            .unwrap();

        let row: (Option<i8>, i64, Option<String>, i64, i64) = index
            .conn
            .query_row(
                "SELECT rating, flag, label, label_known, dirty FROM ratings WHERE path = ?1",
                ["/a.ARW"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(row, (None, 0, None, 1, 1));

        remove_temp_dir(&dir);
    }

    #[test]
    fn mark_written_only_clears_a_row_that_still_holds_the_written_value() {
        let dir = temp_dir("written");
        let mut index = open(&dir);
        index
            .set_rating("d", "/a.ARW", Some(3), Flag::None, None, true)
            .unwrap();

        assert!(index
            .mark_written("/a.ARW", Some(3), Flag::None, None, true, Some((42, 7)))
            .unwrap());
        assert!(index.dirty_rows("d").unwrap().is_empty());

        index
            .set_rating("d", "/a.ARW", Some(5), Flag::None, None, true)
            .unwrap();
        assert!(
            !index
                .mark_written("/a.ARW", Some(3), Flag::None, None, true, Some((42, 7)))
                .unwrap(),
            "a keypress during the write keeps the row dirty"
        );
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/a.ARW".to_string(), Some(5), Flag::None, None, true)]
        );

        // An unrated row is matched by NULL, not skipped.
        index
            .set_rating("d", "/a.ARW", None, Flag::None, None, true)
            .unwrap();
        assert!(index
            .mark_written("/a.ARW", None, Flag::None, None, true, None)
            .unwrap());
        assert!(index.dirty_rows("d").unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn mark_written_for_an_unknown_label_leaves_a_row_dirty_once_it_asserts_a_different_label() {
        let dir = temp_dir("written-unknown-label-superseded");
        let mut index = open(&dir);
        // An unknown-label write for "Red" is in flight...
        index
            .set_rating("d", "/a.ARW", Some(3), Flag::None, None, false)
            .unwrap();

        // ...but a newer judgment with the same rating/flag asserts "Blue"
        // before that write lands.
        index
            .set_rating("d", "/a.ARW", Some(3), Flag::None, Some("Blue"), true)
            .unwrap();

        // The stale unknown-label write must not overwrite the newer,
        // known label, and must leave the row dirty so it is retried.
        assert!(
            !index
                .mark_written(
                    "/a.ARW",
                    Some(3),
                    Flag::None,
                    Some("Red"),
                    false,
                    Some((42, 7))
                )
                .unwrap(),
            "a row that has since asserted a different label is left dirty"
        );
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [(
                "/a.ARW".to_string(),
                Some(3),
                Flag::None,
                Some("Blue".to_string()),
                true
            )]
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_sidecar_reset_drops_clean_rows_and_clears_the_stat_of_dirty_ones() {
        let dir = temp_dir("reset-sidecars");
        let mut index = open(&dir);
        index
            .set_rating("d", "/a.ARW", Some(3), Flag::None, None, true)
            .unwrap();
        assert!(index
            .mark_written("/a.ARW", Some(3), Flag::None, None, true, Some((42, 7)))
            .unwrap());
        index
            .set_rating("d", "/b.ARW", Some(5), Flag::None, None, true)
            .unwrap();
        index
            .store_sidecar_ratings(
                "d",
                &[("/b.ARW".to_string(), Some(5), Flag::None, None, 9, 9, true)],
            )
            .unwrap();
        index
            .set_rating("d", "/b.ARW", None, Flag::Reject, None, true)
            .unwrap();

        index.reset_sidecars().unwrap();

        let rows: Vec<(String, bool)> = index
            .conn
            .prepare("SELECT path, xmp_size IS NULL AND xmp_mtime_ns IS NULL FROM ratings")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(rows, [("/b.ARW".to_string(), true)]);
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/b.ARW".to_string(), None, Flag::Reject, None, true)]
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_flag_is_stored_beside_the_rating_and_guards_mark_written() {
        let dir = temp_dir("flag");
        let a = file(&dir, "a.ARW", b"a");
        let path = a.path.to_string_lossy().into_owned();
        let mut index = open(&dir);
        index.write_batch("d", &[(a, Ok(entry()))]).unwrap();
        assert_eq!(
            index.entries("d").unwrap()[0].flag,
            Flag::None,
            "no ratings row"
        );

        index
            .set_rating("d", &path, Some(3), Flag::Pick, None, true)
            .unwrap();
        let entry = &index.entries("d").unwrap()[0];
        assert_eq!((entry.rating, entry.flag), (Some(3), Flag::Pick));

        assert!(
            !index
                .mark_written(&path, Some(3), Flag::None, None, true, None)
                .unwrap(),
            "an unpick during the write keeps the row dirty"
        );
        assert!(index
            .mark_written(&path, Some(3), Flag::Pick, None, true, None)
            .unwrap());

        index
            .store_sidecar_ratings(
                "d",
                &[(path.clone(), Some(0), Flag::None, None, 1, 2, false)],
            )
            .unwrap();
        assert_eq!(
            index.entries("d").unwrap()[0].flag,
            Flag::None,
            "the sidecar wins"
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_label_is_stored_beside_the_rating_and_guards_mark_written() {
        let dir = temp_dir("label");
        let a = file(&dir, "a.ARW", b"a");
        let path = a.path.to_string_lossy().into_owned();
        let mut index = open(&dir);
        index.write_batch("d", &[(a, Ok(entry()))]).unwrap();
        assert_eq!(index.entries("d").unwrap()[0].label, None, "no ratings row");

        index
            .set_rating("d", &path, Some(3), Flag::None, Some("Red"), true)
            .unwrap();
        assert_eq!(index.entries("d").unwrap()[0].label.as_deref(), Some("Red"));

        assert!(
            !index
                .mark_written(&path, Some(3), Flag::None, Some("Blue"), true, None)
                .unwrap(),
            "a relabel during the write keeps the row dirty"
        );
        assert!(!index
            .mark_written(&path, Some(3), Flag::None, None, true, None)
            .unwrap());
        assert!(index
            .mark_written(&path, Some(3), Flag::None, Some("Red"), true, None)
            .unwrap());

        remove_temp_dir(&dir);
    }

    #[test]
    fn an_unknown_label_leaves_the_ratings_label_untouched() {
        let dir = temp_dir("label-unknown");
        let a = file(&dir, "a.ARW", b"a");
        let path = a.path.to_string_lossy().into_owned();
        let mut index = open(&dir);
        index.write_batch("d", &[(a, Ok(entry()))]).unwrap();

        // A judgment before the row's label is known must not stamp `label`
        // as `None`: a later sidecar parse still needs to be free to fill it
        // in, and mark_written must not be guarded on a value never asserted.
        index
            .set_rating("d", &path, Some(3), Flag::None, None, false)
            .unwrap();
        assert_eq!(
            index.entries("d").unwrap()[0].label,
            None,
            "no label was ever asserted, so the row stays as the sidecar parse left it"
        );

        // Simulate the sidecar parse landing afterwards: it is free to set
        // the label, since `set_rating` above never touched the column. The
        // row is still dirty from that judgment, so the snapshot passed
        // here must say so too.
        index
            .store_sidecar_ratings(
                "d",
                &[(
                    path.clone(),
                    Some(3),
                    Flag::None,
                    Some("Red".into()),
                    1,
                    2,
                    true,
                )],
            )
            .unwrap();
        assert_eq!(index.entries("d").unwrap()[0].label.as_deref(), Some("Red"));

        // `mark_written` for the same unknown-label judgment must not check
        // `label` at all, since `ratings.label` was never set to it, but the
        // resolved label the writer actually put in the sidecar ("Red", the
        // one already there) is stored and marked known regardless, so a
        // later replay of a still-dirty row (see `dirty_rows`) never sends
        // "no label" and strips it.
        assert!(index
            .mark_written(&path, Some(3), Flag::None, Some("Red"), false, None)
            .unwrap());
        assert_eq!(
            index.entries("d").unwrap()[0].label.as_deref(),
            Some("Red"),
            "the resolved label is now stored and marked known"
        );

        // A later judgment that again does not know the label yet leaves
        // that now-known label untouched, and a replay of the resulting
        // dirty row carries it as known.
        index
            .set_rating("d", &path, Some(4), Flag::None, None, false)
            .unwrap();
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [(
                path.clone(),
                Some(4),
                Flag::None,
                Some("Red".to_string()),
                true
            )],
            "the row survives a crash before the writer drains it: a replay must \
             not treat the label as unknown and strip it from the sidecar"
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_sidecar_reset_keeps_the_judgment_of_a_dirty_row() {
        let dir = temp_dir("reset-label");
        let mut index = open(&dir);
        index
            .set_rating("d", "/a.ARW", Some(2), Flag::Pick, Some("Green"), true)
            .unwrap();

        index.reset_sidecars().unwrap();

        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [(
                "/a.ARW".to_string(),
                Some(2),
                Flag::Pick,
                Some("Green".to_string()),
                true
            )]
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v4_database_gains_the_label_column_and_keeps_its_dirty_rows() {
        let dir = temp_dir("migrate-v4");
        let db = dir.join("index.sqlite");
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch(
                "CREATE TABLE ratings (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     rating INTEGER,
                     pick INTEGER NOT NULL DEFAULT 0,
                     xmp_size INTEGER,
                     xmp_mtime_ns INTEGER,
                     dirty INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT INTO ratings (path, dir, rating, pick, dirty)
                     VALUES ('/a.ARW', 'd', 4, 1, 1);
                 PRAGMA user_version = 4;",
            )
            .unwrap();
        }

        let index = Index::open(&db).unwrap();
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/a.ARW".to_string(), Some(4), Flag::Pick, None, true)]
        );
        drop(index);
        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index.dirty_rows("d").unwrap().len(), 1);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v2_database_gains_the_flag_column_and_keeps_its_dirty_rows() {
        let dir = temp_dir("migrate-v2");
        let db = dir.join("index.sqlite");
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch(
                "CREATE TABLE ratings (
                     path TEXT PRIMARY KEY,
                     dir TEXT NOT NULL,
                     rating INTEGER,
                     xmp_size INTEGER,
                     xmp_mtime_ns INTEGER,
                     dirty INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT INTO ratings (path, dir, rating, dirty) VALUES ('/a.ARW', 'd', 4, 1);
                 PRAGMA user_version = 2;",
            )
            .unwrap();
        }

        let index = Index::open(&db).unwrap();
        assert_eq!(
            index.dirty_rows("d").unwrap(),
            [("/a.ARW".to_string(), Some(4), Flag::None, None, true)]
        );
        drop(index);
        let index = Index::open(&db).unwrap();
        assert_eq!(index.dirty_rows("d").unwrap().len(), 1, "a reopen is v11");

        remove_temp_dir(&dir);
    }

    #[test]
    fn has_sidecar_follows_the_stored_sidecar_stat() {
        let dir = temp_dir("has-sidecar");
        let a = file(&dir, "a.ARW", b"a");
        let path = a.path.to_string_lossy().into_owned();
        let mut index = open(&dir);
        index.write_batch("d", &[(a, Ok(entry()))]).unwrap();
        let has_sidecar = |index: &Index| index.entries("d").unwrap()[0].has_sidecar;

        assert!(!has_sidecar(&index), "no ratings row");

        index
            .set_rating("d", &path, Some(3), Flag::None, None, true)
            .unwrap();
        assert!(!has_sidecar(&index), "dirty row, nothing written yet");

        assert!(index
            .mark_written(&path, Some(3), Flag::None, None, true, Some((42, 7)))
            .unwrap());
        assert!(has_sidecar(&index));

        index
            .set_rating("d", &path, None, Flag::None, None, true)
            .unwrap();
        assert!(index
            .mark_written(&path, None, Flag::None, None, true, None)
            .unwrap());
        assert!(!has_sidecar(&index));

        index
            .store_sidecar_ratings(
                "d",
                &[(path.clone(), None, Flag::Reject, None, 10, 20, false)],
            )
            .unwrap();
        assert!(has_sidecar(&index));

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_failed_file_is_remembered_as_an_error_row_without_a_thumbnail() {
        let dir = temp_dir("error");
        let a = file(&dir, "a.ARW", b"a");
        let mut index = open(&dir);
        index
            .write_batch("d", &[(a.clone(), Err("broken".into()))])
            .unwrap();

        let entries = index.entries("d").unwrap();
        assert!(!entries[0].has_thumb);
        assert!(entries[0].exif.is_none());
        assert_eq!(entries[0].error.as_deref(), Some("broken"));
        assert!(index.thumbnail(&entries[0].path).is_err());
        assert!(index.reconcile("d", &[a]).unwrap().0.is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_failed_file_keeps_the_metadata_its_failure_carries() {
        let dir = temp_dir("error-meta");
        let a = file(&dir, "a.ARW", b"a");
        let mut index = open(&dir);
        let Entry {
            orientation, shot, ..
        } = entry();
        let failure = Failure {
            message: "broken".to_string(),
            orientation,
            shot: Some(Box::new(shot)),
        };
        index
            .write_batch("d", &[(a.clone(), Err(failure))])
            .unwrap();

        let entries = index.entries("d").unwrap();
        let e = &entries[0];
        assert!(!e.has_thumb);
        assert_eq!(e.orientation, 6);
        assert_eq!(e.capture_time.as_deref(), Some("2026:09:13 09:23:33"));
        assert!(e.focus.is_some());
        assert!(e.exif.as_ref().is_some_and(|x| x.camera.is_some()));
        assert_eq!(e.error.as_deref(), Some("broken"));
        assert!(index.thumbnail(&e.path).is_err());
        assert!(index.faces_todo("d").unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    /// A TIFF shell whose IFD0 carries an Orientation and a preview of `body`,
    /// the same synthetic fixture the core's scan tests use.
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
        buf.extend_from_slice(body);
        buf
    }

    pub(crate) fn jpeg(w: usize, h: usize) -> Vec<u8> {
        let rgb = vec![128u8; w * h * 3];
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        c.set_size(w, h);
        c.set_quality(80.0);
        let mut c = c.start_compress(Vec::new()).unwrap();
        c.write_scanlines(&rgb).unwrap();
        c.finish().unwrap()
    }

    /// A `w` x `h` JPEG with an APP1 Exif segment spliced after SOI: a
    /// little-endian TIFF whose IFD0 holds `orientation` and whose Exif IFD
    /// holds `DateTimeOriginal` (`capture_time`, 19 characters) and `iso`.
    pub(crate) fn jpeg_with_exif(
        w: usize,
        h: usize,
        orientation: u16,
        capture_time: &str,
        iso: u16,
    ) -> Vec<u8> {
        let ifd = |entries: &[(u16, u16, u32, u32)], out: &mut Vec<u8>| {
            out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            for (tag, ty, count, value) in entries {
                out.extend_from_slice(&tag.to_le_bytes());
                out.extend_from_slice(&ty.to_le_bytes());
                out.extend_from_slice(&count.to_le_bytes());
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&0u32.to_le_bytes());
        };
        let exif_at = 8 + 2 + 2 * 12 + 4;
        let data_at = exif_at + 2 + 2 * 12 + 4;
        let mut tiff = b"II\x2a\x00".to_vec();
        tiff.extend_from_slice(&8u32.to_le_bytes());
        ifd(
            &[(0x0112, 3, 1, orientation as u32), (0x8769, 4, 1, exif_at)],
            &mut tiff,
        );
        ifd(
            &[(0x8827, 3, 1, iso as u32), (0x9003, 2, 20, data_at)],
            &mut tiff,
        );
        tiff.extend_from_slice(capture_time.as_bytes());
        tiff.push(0);

        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let body = jpeg(w, h);
        let mut out = body[..2].to_vec();
        out.extend_from_slice(&[0xff, 0xe1]);
        out.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(&app1);
        out.extend_from_slice(&body[2..]);
        out
    }

    #[test]
    fn a_scanned_jpeg_has_its_exif_and_thumbnail_and_skips_the_faces_pass() {
        let dir = temp_dir("scan-jpeg");
        let files = [
            file(
                &dir,
                "a.jpg",
                &jpeg_with_exif(64, 48, 6, "2026:09:27 10:00:01", 400),
            ),
            file(
                &dir,
                "b.JPEG",
                &jpeg_with_exif(64, 48, 1, "2026:09:27 10:00:00", 800),
            ),
        ];

        let index = Mutex::new(open(&dir));
        let summary = run_scan(
            &index,
            "d",
            &files,
            2,
            &ScanFocus::default(),
            &AtomicBool::new(false),
            PROGRESS_INTERVAL,
            |_, _, _| {},
        );

        assert_eq!((summary.total, summary.errors), (2, 0));
        let mut index = lock(&index);
        let entries = index.entries("d").unwrap();
        let summary: Vec<_> = entries
            .iter()
            .map(|e| (e.orientation, e.capture_time.as_deref()))
            .collect();
        assert_eq!(
            summary,
            [
                (6, Some("2026:09:27 10:00:01")),
                (1, Some("2026:09:27 10:00:00"))
            ]
        );
        assert!(entries.iter().all(|e| e.has_thumb
            && e.sharpness.is_none()
            && e.focus.is_none()
            && e.exif.is_some()));
        assert!(index.faces_todo("d").unwrap().is_empty());
        for file in &files {
            assert_eq!(faces_extractor(&index, &file.path), FACES_VERSION);
            let eye_focus: Option<f64> = index
                .conn
                .query_row(
                    "SELECT eye_focus FROM files WHERE path = ?1",
                    params![file.path.to_string_lossy()],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(eye_focus, None);
        }

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_scan_writes_every_file_and_reports_progress() {
        let dir = temp_dir("scan");
        let body = jpeg(64, 48);
        let files: Vec<FileStat> = (0..8)
            .map(|i| file(&dir, &format!("{i:02}.ARW"), &fixture(6, &body)))
            .collect();

        let index = Mutex::new(open(&dir));
        let progress = Mutex::new(Vec::new());
        let summary = run_scan(
            &index,
            "d",
            &files,
            2,
            &ScanFocus::default(),
            &AtomicBool::new(false),
            PROGRESS_INTERVAL,
            |done, total, ready| progress.lock().unwrap().push((done, total, ready)),
        );

        assert_eq!(summary.total, 8);
        assert_eq!(summary.errors, 0);
        let entries = lock(&index).entries("d").unwrap();
        assert_eq!(entries.len(), 8);
        assert!(entries.iter().all(|e| e.has_thumb && e.orientation == 6));
        let progress = progress.into_inner().unwrap();
        let (done, total, _) = progress.last().unwrap();
        assert_eq!((*done, *total), (8, 8), "the last file is reported");

        remove_temp_dir(&dir);
    }

    /// Every written path is reported exactly once, for a file count that is
    /// not a multiple of `BATCH`.
    #[test]
    fn a_scan_reports_every_written_path_exactly_once() {
        let dir = temp_dir("ready");
        let body = jpeg(64, 48);
        let files: Vec<FileStat> = (0..(BATCH * 2 + 3))
            .map(|i| file(&dir, &format!("{i:03}.ARW"), &fixture(6, &body)))
            .collect();

        let index = Mutex::new(open(&dir));
        let reported = Mutex::new(Vec::new());
        let summary = run_scan(
            &index,
            "d",
            &files,
            2,
            &ScanFocus::default(),
            &AtomicBool::new(false),
            PROGRESS_INTERVAL,
            |_, _, ready| reported.lock().unwrap().extend(ready),
        );

        assert_eq!(summary.total, files.len());
        let mut reported = reported.into_inner().unwrap();
        let written: Vec<String> = {
            let mut paths: Vec<String> = lock(&index)
                .entries("d")
                .unwrap()
                .into_iter()
                .map(|e| e.path)
                .collect();
            paths.sort();
            paths
        };
        let unique: HashSet<&String> = reported.iter().collect();
        assert_eq!(unique.len(), reported.len(), "no path is reported twice");
        reported.sort();
        assert_eq!(reported, written, "every written path is reported");

        remove_temp_dir(&dir);
    }

    /// An extraction error is authoritative too: its row exists, so its path
    /// is reported like a successful one.
    #[test]
    fn a_scan_reports_the_paths_of_files_that_failed_extraction() {
        let dir = temp_dir("ready-err");
        let body = jpeg(64, 48);
        let mut files: Vec<FileStat> = (0..4)
            .map(|i| file(&dir, &format!("{i:03}.ARW"), &fixture(6, &body)))
            .collect();
        let bad = file(&dir, "bad.ARW", b"not a raw file at all");
        files.push(bad.clone());

        let index = Mutex::new(open(&dir));
        let reported = Mutex::new(Vec::new());
        let summary = run_scan(
            &index,
            "d",
            &files,
            2,
            &ScanFocus::default(),
            &AtomicBool::new(false),
            PROGRESS_INTERVAL,
            |_, _, ready| reported.lock().unwrap().extend(ready),
        );

        assert_eq!(summary.errors, 1);
        let reported = reported.into_inner().unwrap();
        let bad_path = bad.path.to_string_lossy().into_owned();
        assert!(reported.contains(&bad_path), "the failed file is reported");
        let entries = lock(&index).entries("d").unwrap();
        assert!(
            entries
                .iter()
                .any(|e| e.path == bad_path && e.exif.is_none()),
            "the failed file has a row"
        );
        assert_eq!(reported.len(), entries.len());

        remove_temp_dir(&dir);
    }

    #[test]
    fn canceling_after_the_first_batch_keeps_what_was_written() {
        let dir = temp_dir("cancel");
        let body = jpeg(64, 48);
        // The cancel fires from `progress` once `done >= BATCH`, so the count
        // only has to leave room for the items already in flight on the
        // worker threads when it does; it no longer races the machine speed.
        let files: Vec<FileStat> = (0..(BATCH * 4))
            .map(|i| file(&dir, &format!("{i:03}.ARW"), &fixture(1, &body)))
            .collect();

        let index = Mutex::new(open(&dir));
        let cancel = AtomicBool::new(false);
        let reported = Mutex::new(Vec::new());
        // A full batch is flushed before `done` counts its last item, so
        // `done >= BATCH` means the first batch has been written.
        let summary = run_scan(
            &index,
            "d",
            &files,
            2,
            &ScanFocus::default(),
            &cancel,
            Duration::ZERO,
            |done, _, ready| {
                reported.lock().unwrap().extend(ready);
                if done >= BATCH {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
        );

        assert!(summary.total >= BATCH, "the first batch ran");
        assert!(summary.total < files.len(), "the rest did not");
        let written = lock(&index).entries("d").unwrap();
        assert_eq!(
            written.len(),
            summary.total,
            "everything scanned is persisted"
        );
        let mut reported = reported.into_inner().unwrap();
        reported.sort();
        let mut persisted: Vec<String> = written.into_iter().map(|e| e.path).collect();
        persisted.sort();
        assert_eq!(reported, persisted, "the reported paths are the rows kept");
        let rows: i64 = lock(&index)
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            rows as usize,
            persisted.len(),
            "abandoned files have no row at all"
        );

        remove_temp_dir(&dir);
    }

    /// Files scanned by the first pass, ready for the second, and their paths.
    fn faces_fixture(dir: &Path, count: usize) -> (Mutex<Index>, Vec<String>) {
        let body = jpeg(64, 48);
        let files: Vec<FileStat> = (0..count)
            .map(|i| file(dir, &format!("{i:03}.ARW"), &fixture(1, &body)))
            .collect();
        let paths = files
            .iter()
            .map(|f| f.path.to_string_lossy().into_owned())
            .collect();
        let index = Mutex::new(open(dir));
        run_scan(
            &index,
            "d",
            &files,
            2,
            &ScanFocus::default(),
            &AtomicBool::new(false),
            PROGRESS_INTERVAL,
            |_, _, _| {},
        );
        (index, paths)
    }

    #[test]
    fn the_faces_pass_writes_and_reports_every_path() {
        let dir = temp_dir("faces-scan");
        let (index, paths) = faces_fixture(&dir, BATCH * 2 + 3);
        assert!(
            lock(&index)
                .entries("d")
                .unwrap()
                .iter()
                .all(|e| e.sharpness.is_none()),
            "the first pass does not score"
        );
        let score = riffle_core::scan::extract_analysis(Path::new(&paths[0]))
            .unwrap()
            .sharpness;
        assert!(score.is_some());

        let reported = Mutex::new(Vec::new());
        let summary = run_faces_scan(
            &index,
            "d",
            &paths,
            2,
            &ScanFocus::default(),
            &AtomicBool::new(false),
            PROGRESS_INTERVAL,
            |_, _, ready| reported.lock().unwrap().extend(ready),
        );

        assert_eq!((summary.total, summary.errors), (paths.len(), 0));
        let mut reported = reported.into_inner().unwrap();
        reported.sort_by(|a, b| a.path.cmp(&b.path));
        let expected: Vec<FaceReady> = paths
            .iter()
            .map(|path| FaceReady {
                path: path.clone(),
                eye_focus: None,
                candidate: FocusCandidate::Unknown,
                sharpness: score,
                eyes: StoredEyes::default(),
            })
            .collect();
        assert_eq!(reported, expected, "every path is reported once");
        let index = lock(&index);
        for path in &paths {
            assert_eq!(faces_extractor(&index, Path::new(path)), FACES_VERSION);
        }
        assert!(index
            .entries("d")
            .unwrap()
            .iter()
            .all(|e| e.sharpness == score));

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_row_the_old_first_pass_scored_keeps_its_score_and_a_new_one_gets_it_from_the_faces_pass() {
        let dir = temp_dir("faces-migrate");
        let (index, paths) = faces_fixture(&dir, 2);
        let (old, new) = (&paths[0], &paths[1]);
        lock(&index)
            .conn
            .execute(
                "UPDATE files SET sharpness = 42.0, faces_extractor = ?2 WHERE path = ?1",
                params![old, FACES_VERSION],
            )
            .unwrap();

        let todo = lock(&index).faces_todo("d").unwrap();
        assert_eq!(todo, std::slice::from_ref(new));
        run_faces_scan(
            &index,
            "d",
            &todo,
            2,
            &ScanFocus::default(),
            &AtomicBool::new(false),
            PROGRESS_INTERVAL,
            |_, _, _| {},
        );

        let index = lock(&index);
        let score = |path: &str| index.entry(path).unwrap().unwrap().sharpness;
        assert_eq!(score(old), Some(42.0), "the old row keeps its score");
        assert_eq!(
            score(new),
            riffle_core::scan::extract_analysis(Path::new(new))
                .unwrap()
                .sharpness
        );
        assert!(score(new).is_some());
        assert_eq!(faces_extractor(&index, Path::new(new)), FACES_VERSION);

        remove_temp_dir(&dir);
    }

    #[test]
    fn canceling_the_faces_pass_still_reports_what_was_written() {
        let dir = temp_dir("faces-cancel");
        let (index, paths) = faces_fixture(&dir, BATCH * 4);

        let cancel = AtomicBool::new(false);
        let reported = Mutex::new(Vec::new());
        let summary = run_faces_scan(
            &index,
            "d",
            &paths,
            2,
            &ScanFocus::default(),
            &cancel,
            Duration::ZERO,
            |done, _, ready| {
                reported
                    .lock()
                    .unwrap()
                    .extend(ready.into_iter().map(|r| r.path));
                if done >= BATCH {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
        );

        assert!(summary.total >= BATCH, "the first batch ran");
        assert!(summary.total < paths.len(), "the rest did not");
        let index = lock(&index);
        let mut written: Vec<String> = paths
            .iter()
            .filter(|p| faces_extractor(&index, Path::new(p)) == FACES_VERSION)
            .cloned()
            .collect();
        written.sort();
        assert_eq!(written.len(), summary.total, "everything done is written");
        assert!(
            paths
                .iter()
                .filter(|p| !written.contains(p))
                .all(|p| faces_extractor(&index, Path::new(p)) == 0),
            "abandoned files keep their old faces_extractor"
        );
        let mut reported = reported.into_inner().unwrap();
        reported.sort();
        assert_eq!(reported, written, "the trailing flush is reported too");

        remove_temp_dir(&dir);
    }

    #[test]
    fn an_unreadable_file_in_the_faces_pass_is_one_error_stored_as_none() {
        let dir = temp_dir("faces-err");
        let (index, mut paths) = faces_fixture(&dir, 3);
        let bad = file(&dir, "bad.ARW", b"not a raw file at all");
        lock(&index)
            .write_batch("d", &[(bad.clone(), Ok(entry()))])
            .unwrap();
        let bad_path = bad.path.to_string_lossy().into_owned();
        paths.push(bad_path.clone());

        let summary = run_faces_scan(
            &index,
            "d",
            &paths,
            2,
            &ScanFocus::default(),
            &AtomicBool::new(false),
            PROGRESS_INTERVAL,
            |_, _, _| {},
        );

        assert_eq!((summary.total, summary.errors), (4, 1));
        let mut index = lock(&index);
        assert_eq!(faces_extractor(&index, &bad.path), FACES_VERSION);
        let focus = index.entry(&bad_path).unwrap().unwrap().focus.unwrap();
        assert_eq!(focus.eye_focus, None);
        assert!(index.faces_todo("d").unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    fn synthetic(dir: &Path, i: usize) -> FileStat {
        FileStat {
            path: dir.join(format!("{i:05}.ARW")),
            size: 1,
            mtime_ns: 1,
        }
    }

    #[test]
    fn the_reader_does_not_wait_on_an_open_write_transaction() {
        let dir = temp_dir("reader");
        let path = dir.join("index.sqlite");
        let mut writer = Index::open(&path).unwrap();
        let rows: Vec<_> = (0..3).map(|i| (synthetic(&dir, i), Ok(entry()))).collect();
        writer.write_batch("d", &rows).unwrap();
        let reader = Mutex::new(Index::open_reader(&path).unwrap());
        let first = rows[0].0.path.to_string_lossy().into_owned();

        let writer = Mutex::new(writer);
        // The reads are joined while the transaction is still open: a blocked
        // reader would hit the busy timeout `Index::open_reader` sets and fail
        // here, since the rollback below cannot run until the join returns.
        std::thread::scope(|scope| {
            let guard = lock(&writer);
            guard.conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            let read = scope.spawn(|| {
                let reader = lock(&reader);
                let entries = reader.entries("d").unwrap();
                let thumb = reader.thumbnail(&first).unwrap();
                (entries.len(), thumb.0)
            });
            assert_eq!(read.join().unwrap(), (3, 6));
            guard.conn.execute_batch("ROLLBACK").unwrap();
        });

        lock(&writer)
            .write_batch("d", &[(synthetic(&dir, 3), Ok(entry()))])
            .unwrap();
        assert_eq!(lock(&reader).entries("d").unwrap().len(), 4);

        remove_temp_dir(&dir);
    }

    fn opened_at(index: &Index, dir: &str) -> Option<i64> {
        index
            .conn
            .query_row(
                "SELECT opened_at FROM folders WHERE dir = ?1",
                params![dir],
                |r| r.get(0),
            )
            .ok()
    }

    fn set_opened_at(index: &Index, dir: &str, at: i64) {
        index
            .conn
            .execute(
                "INSERT OR REPLACE INTO folders (dir, opened_at) VALUES (?1, ?2)",
                params![dir, at],
            )
            .unwrap();
    }

    fn big_entry() -> Entry {
        Entry {
            thumbnail: vec![0x55; 200_000],
            ..entry()
        }
    }

    const DAY: i64 = 24 * 60 * 60;
    const NOW: i64 = 100 * DAY;
    const NO_CAP: EvictPolicy = EvictPolicy {
        max_age_secs: MAX_AGE_SECS,
        max_bytes: i64::MAX,
    };

    #[test]
    fn reconcile_records_when_the_folder_was_opened() {
        let dir = temp_dir("opened-at");
        let mut index = open(&dir);
        set_opened_at(&index, "d", 0);
        let before = now_secs();
        index.reconcile("d", &[]).unwrap();
        assert!(opened_at(&index, "d").unwrap() >= before);

        remove_temp_dir(&dir);
    }

    #[test]
    fn set_last_viewed_round_trips_and_an_unknown_dir_has_none() {
        let dir = temp_dir("last-viewed");
        let mut index = open(&dir);
        assert_eq!(index.last_viewed("d").unwrap(), None);
        index.set_last_viewed("d", "/d/a.ARW").unwrap();
        index.set_last_viewed("d", "/d/b.ARW").unwrap();
        assert_eq!(index.last_viewed("d").unwrap().as_deref(), Some("/d/b.ARW"));
        assert_eq!(index.last_viewed("e").unwrap(), None);

        remove_temp_dir(&dir);
    }

    #[test]
    fn set_last_viewed_before_reconcile_creates_the_row_and_reconcile_keeps_it() {
        let dir = temp_dir("last-viewed-reconcile");
        let mut index = open(&dir);
        let before = now_secs();
        index.set_last_viewed("d", "/d/a.ARW").unwrap();
        assert!(opened_at(&index, "d").unwrap() >= before);
        index.reconcile("d", &[]).unwrap();
        assert_eq!(index.last_viewed("d").unwrap().as_deref(), Some("/d/a.ARW"));

        remove_temp_dir(&dir);
    }

    #[test]
    fn rename_dir_moves_the_folder_and_its_subfolders_but_not_a_prefix_sibling() {
        let dir = temp_dir("rename-dir");
        let mut index = open(&dir);
        let photos = dir.join("photos");
        let sub = photos.join("sub");
        let sibling = dir.join("photos2");
        let renamed = dir.join("shoot");
        let s = |p: &Path| p.to_string_lossy().into_owned();
        for folder in [&photos, &sub, &sibling] {
            index
                .write_batch(&s(folder), &[(synthetic(folder, 0), Ok(entry()))])
                .unwrap();
        }
        let a = s(&photos.join("00000.ARW"));
        let b = s(&sub.join("00000.ARW"));
        let c = s(&sibling.join("00000.ARW"));
        index
            .set_rating(&s(&photos), &a, Some(3), Flag::Pick, Some("Red"), true)
            .unwrap();
        index
            .set_rating(&s(&sibling), &c, Some(1), Flag::None, None, true)
            .unwrap();
        index.set_last_viewed(&s(&photos), &a).unwrap();
        index.set_last_viewed(&s(&sub), &b).unwrap();
        index.set_last_viewed(&s(&sibling), &c).unwrap();

        assert_eq!(index.rename_dir(&s(&photos), &s(&renamed)).unwrap(), 2);

        assert!(index.entries(&s(&photos)).unwrap().is_empty());
        assert!(index.entries(&s(&sub)).unwrap().is_empty());
        let moved = index.entries(&s(&renamed)).unwrap();
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[0].path, s(&renamed.join("00000.ARW")));
        assert!(moved[0].has_thumb);
        let moved_sub = index.entries(&s(&renamed.join("sub"))).unwrap();
        assert_eq!(moved_sub[0].path, s(&renamed.join("sub").join("00000.ARW")));
        assert_eq!(
            index.dirty_rows(&s(&renamed)).unwrap(),
            vec![(
                s(&renamed.join("00000.ARW")),
                Some(3),
                Flag::Pick,
                Some("Red".to_string()),
                true
            )]
        );
        assert!(index.dirty_rows(&s(&photos)).unwrap().is_empty());
        assert_eq!(
            index.last_viewed(&s(&renamed)).unwrap(),
            Some(s(&renamed.join("00000.ARW")))
        );
        assert_eq!(
            index.last_viewed(&s(&renamed.join("sub"))).unwrap(),
            Some(s(&renamed.join("sub").join("00000.ARW")))
        );
        assert_eq!(index.last_viewed(&s(&photos)).unwrap(), None);

        assert_eq!(index.entries(&s(&sibling)).unwrap()[0].path, c);
        assert_eq!(index.dirty_rows(&s(&sibling)).unwrap().len(), 1);
        assert_eq!(index.last_viewed(&s(&sibling)).unwrap(), Some(c));

        remove_temp_dir(&dir);
    }

    #[test]
    fn rename_file_moves_its_rows_and_last_viewed() {
        let dir = temp_dir("rename-file");
        let mut index = open(&dir);
        let photos = dir.join("photos");
        let s = |p: &Path| p.to_string_lossy().into_owned();
        index
            .write_batch(
                &s(&photos),
                &[
                    (synthetic(&photos, 0), Ok(entry())),
                    (synthetic(&photos, 1), Ok(entry())),
                ],
            )
            .unwrap();
        let a = s(&photos.join("00000.ARW"));
        let b = s(&photos.join("00001.ARW"));
        let renamed = s(&photos.join("shoot.ARW"));
        index
            .set_rating(&s(&photos), &a, Some(4), Flag::Reject, Some("Blue"), true)
            .unwrap();
        index.set_last_viewed(&s(&photos), &a).unwrap();

        index.rename_file(&a, &renamed).unwrap();

        let mut paths: Vec<_> = index
            .entries(&s(&photos))
            .unwrap()
            .into_iter()
            .map(|e| (e.path, e.has_thumb))
            .collect();
        paths.sort();
        assert_eq!(paths, vec![(b, true), (renamed.clone(), true)]);
        assert_eq!(
            index.dirty_rows(&s(&photos)).unwrap(),
            vec![(
                renamed.clone(),
                Some(4),
                Flag::Reject,
                Some("Blue".to_string()),
                true
            )]
        );
        assert_eq!(index.last_viewed(&s(&photos)).unwrap(), Some(renamed));

        remove_temp_dir(&dir);
    }

    #[test]
    fn move_file_carries_its_rows_to_another_folder_and_replaces_stale_ones() {
        let dir = temp_dir("move-file");
        let mut index = open(&dir);
        let photos = dir.join("photos");
        let dated = dir.join("2026-10-31");
        let s = |p: &Path| p.to_string_lossy().into_owned();
        index
            .write_batch(
                &s(&photos),
                &[
                    (synthetic(&photos, 0), Ok(entry())),
                    (synthetic(&photos, 1), Ok(entry())),
                ],
            )
            .unwrap();
        let a = s(&photos.join("00000.ARW"));
        let b = s(&photos.join("00001.ARW"));
        let moved = s(&dated.join("00000.ARW"));
        let embed = |path: String| EmbedRow {
            file: EmbedTodo {
                path,
                size: 1,
                mtime_ns: 1,
            },
            vec: Some(vec![1.0]),
        };
        index
            .set_rating(&s(&photos), &a, Some(4), Flag::Reject, Some("Blue"), true)
            .unwrap();
        index
            .set_rating(&s(&photos), &b, Some(2), Flag::None, None, true)
            .unwrap();
        index.set_last_viewed(&s(&photos), &a).unwrap();
        index
            .mark_viewed(&s(&photos), &[a.clone(), b.clone()])
            .unwrap();
        index
            .write_embeddings(&s(&photos), &[embed(a.clone()), embed(b.clone())])
            .unwrap();
        index
            .write_batch(&s(&dated), &[(synthetic(&dated, 0), Err("stale".into()))])
            .unwrap();
        index
            .set_rating(&s(&dated), &moved, Some(1), Flag::Pick, None, true)
            .unwrap();
        index
            .mark_viewed(&s(&dated), std::slice::from_ref(&moved))
            .unwrap();
        index
            .write_embeddings(
                &s(&dated),
                &[EmbedRow {
                    vec: None,
                    ..embed(moved.clone())
                }],
            )
            .unwrap();

        index.move_file(&a, &s(&dated), &moved).unwrap();

        let paths = |index: &Index, folder: &Path| -> Vec<(String, bool)> {
            index
                .entries(&s(folder))
                .unwrap()
                .into_iter()
                .map(|e| (e.path, e.has_thumb))
                .collect()
        };
        assert_eq!(paths(&index, &photos), vec![(b.clone(), true)]);
        assert_eq!(paths(&index, &dated), vec![(moved.clone(), true)]);
        assert_eq!(
            index.dirty_rows(&s(&dated)).unwrap(),
            vec![(
                moved.clone(),
                Some(4),
                Flag::Reject,
                Some("Blue".to_string()),
                true
            )]
        );
        assert_eq!(
            index.dirty_rows(&s(&photos)).unwrap(),
            vec![(b.clone(), Some(2), Flag::None, None, true)]
        );
        assert_eq!(index.viewed_files(&s(&dated)).unwrap(), [moved.as_str()]);
        assert_eq!(index.viewed_files(&s(&photos)).unwrap(), [b.as_str()]);
        let rows: Vec<(String, String, Option<Vec<f32>>)> = embedding_rows(&index)
            .into_iter()
            .map(|r| (r.0, r.1, r.3))
            .collect();
        assert_eq!(
            rows,
            [
                (moved.clone(), s(&dated), Some(vec![1.0])),
                (b.clone(), s(&photos), Some(vec![1.0])),
            ]
        );
        assert_eq!(index.last_viewed(&s(&photos)).unwrap(), Some(moved));

        remove_temp_dir(&dir);
    }

    #[test]
    fn forget_dir_drops_exactly_one_folder() {
        let dir = temp_dir("forget-dir");
        let mut index = open(&dir);
        let photos = dir.join("photos");
        let sub = photos.join("sub");
        let sibling = dir.join("photos2");
        let s = |p: &Path| p.to_string_lossy().into_owned();
        for folder in [&photos, &sub, &sibling] {
            let path = s(&folder.join("00000.ARW"));
            index
                .write_batch(&s(folder), &[(synthetic(folder, 0), Ok(entry()))])
                .unwrap();
            index
                .set_rating(&s(folder), &path, Some(3), Flag::Pick, None, true)
                .unwrap();
            index.set_last_viewed(&s(folder), &path).unwrap();
            index
                .mark_viewed(&s(folder), std::slice::from_ref(&path))
                .unwrap();
            index
                .write_embeddings(
                    &s(folder),
                    &[EmbedRow {
                        file: EmbedTodo {
                            path,
                            size: 1,
                            mtime_ns: 1,
                        },
                        vec: Some(vec![1.0]),
                    }],
                )
                .unwrap();
        }

        index.forget_dir(&s(&photos)).unwrap();

        assert!(index.entries(&s(&photos)).unwrap().is_empty());
        assert!(index.dirty_rows(&s(&photos)).unwrap().is_empty());
        assert!(index.viewed_files(&s(&photos)).unwrap().is_empty());
        assert_eq!(index.last_viewed(&s(&photos)).unwrap(), None);
        let folders: Vec<String> = index
            .conn
            .prepare("SELECT dir FROM folders ORDER BY dir")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let mut kept = vec![s(&sub), s(&sibling)];
        kept.sort();
        assert_eq!(folders, kept);
        let mut embedded: Vec<String> = embedding_rows(&index).into_iter().map(|r| r.1).collect();
        embedded.sort();
        assert_eq!(embedded, kept);
        for folder in [&sub, &sibling] {
            assert_eq!(index.entries(&s(folder)).unwrap().len(), 1);
            assert_eq!(index.dirty_rows(&s(folder)).unwrap().len(), 1);
            assert_eq!(index.viewed_files(&s(folder)).unwrap().len(), 1);
        }

        remove_temp_dir(&dir);
    }

    #[test]
    fn evicting_a_folder_without_ratings_forgets_its_last_viewed() {
        let dir = temp_dir("last-viewed-evict");
        let mut index = open(&dir);
        index
            .write_batch("old", &[(synthetic(&dir, 0), Ok(entry()))])
            .unwrap();
        set_opened_at(&index, "old", NOW - 31 * DAY);
        index.set_last_viewed("old", "/old/a.ARW").unwrap();

        assert_eq!(index.evict(NOW, NO_CAP).unwrap().folders, 1);
        assert_eq!(index.last_viewed("old").unwrap(), None);

        remove_temp_dir(&dir);
    }

    #[test]
    fn the_age_rule_spares_a_viewed_or_rejected_folder_and_clear_does_not() {
        let dir = temp_dir("evict-ranked");
        let mut index = open(&dir);
        let files: Vec<FileStat> = (0..4).map(|i| synthetic(&dir, i)).collect();
        let folders = ["viewed", "rejected", "picked", "stale"];
        for (name, file) in folders.iter().zip(&files) {
            index
                .write_batch(name, &[(file.clone(), Ok(entry()))])
                .unwrap();
            index
                .write_embeddings(name, &[embedded(file, Some(vec![1.0]))])
                .unwrap();
            set_opened_at(&index, name, NOW - 90 * DAY);
        }
        let path = |f: &FileStat| f.path.to_string_lossy().into_owned();
        index.mark_viewed("viewed", &[path(&files[0])]).unwrap();
        index
            .conn
            .execute(
                "INSERT INTO ratings (path, dir, flag) VALUES
                     (?1, 'rejected', 2), (?2, 'picked', 1)",
                params![path(&files[1]), path(&files[2])],
            )
            .unwrap();

        let summary = index.evict(NOW, NO_CAP).unwrap();
        assert_eq!(summary.folders, 2, "the pick-only and untouched folders go");
        for gone in ["picked", "stale"] {
            assert!(index.entries(gone).unwrap().is_empty());
            assert!(index.embeddings(gone).unwrap().is_empty());
        }
        for kept in ["viewed", "rejected"] {
            assert_eq!(index.entries(kept).unwrap().len(), 1);
            assert_eq!(index.embeddings(kept).unwrap().len(), 1);
        }

        let policy = EvictPolicy {
            max_age_secs: MAX_AGE_SECS,
            max_bytes: 0,
        };
        index.evict(NOW, policy).unwrap();
        assert!(
            index.entries("viewed").unwrap().is_empty(),
            "the size cap still removes a spared folder"
        );

        index
            .write_batch("viewed", &[(files[0].clone(), Ok(entry()))])
            .unwrap();
        set_opened_at(&index, "viewed", NOW - 90 * DAY);
        index.clear().unwrap();
        assert!(index.entries("viewed").unwrap().is_empty());
        assert!(index.rank_dirs().unwrap().is_empty());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v16_database_gains_last_viewed_and_keeps_its_folders() {
        let dir = temp_dir("migrate-v16");
        let db = dir.join("index.sqlite");
        {
            let index = open(&dir);
            set_opened_at(&index, "d", 42);
            index
                .conn
                .execute_batch(
                    "ALTER TABLE files DROP COLUMN eyes_ear;
                     ALTER TABLE files DROP COLUMN pose_yaw;
                     ALTER TABLE files DROP COLUMN pose_pitch;
                     ALTER TABLE files DROP COLUMN pose_roll;
                     ALTER TABLE files DROP COLUMN eye_offset;
                     ALTER TABLE files DROP COLUMN edge_gap;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 16;",
                )
                .unwrap();
        }

        let mut index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(opened_at(&index, "d"), Some(42), "folders are kept");
        assert_eq!(index.last_viewed("d").unwrap(), None);
        index.set_last_viewed("d", "/d/a.ARW").unwrap();
        assert_eq!(index.last_viewed("d").unwrap().as_deref(), Some("/d/a.ARW"));

        remove_temp_dir(&dir);
    }

    #[test]
    fn an_old_folder_is_evicted_and_a_fresh_one_kept() {
        let dir = temp_dir("evict-age");
        let mut index = open(&dir);
        index
            .write_batch("old", &[(synthetic(&dir, 0), Ok(entry()))])
            .unwrap();
        index
            .write_batch("new", &[(synthetic(&dir, 1), Ok(entry()))])
            .unwrap();
        index
            .set_rating(
                "old",
                &synthetic(&dir, 0).path.to_string_lossy(),
                Some(3),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        index
            .mark_written(
                &synthetic(&dir, 0).path.to_string_lossy(),
                Some(3),
                Flag::None,
                None,
                true,
                None,
            )
            .unwrap();
        set_opened_at(&index, "old", NOW - 31 * DAY);
        set_opened_at(&index, "new", NOW - 29 * DAY);

        let summary = index.evict(NOW, NO_CAP).unwrap();
        assert_eq!(summary.folders, 1);
        assert!(summary.vacuumed);
        assert!(index.entries("old").unwrap().is_empty());
        assert_eq!(opened_at(&index, "old"), None);
        assert_eq!(index.entries("new").unwrap().len(), 1);
        let ratings: i64 = index
            .conn
            .query_row("SELECT COUNT(*) FROM ratings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ratings, 0);

        assert_eq!(index.evict(NOW, NO_CAP).unwrap(), EvictSummary::default());

        remove_temp_dir(&dir);
    }

    #[test]
    fn the_size_cap_evicts_the_least_recently_opened_folders_first() {
        let dir = temp_dir("evict-lru");
        let mut index = open(&dir);
        for (i, name) in ["a", "b", "c"].iter().enumerate() {
            index
                .write_batch(name, &[(synthetic(&dir, i), Ok(big_entry()))])
                .unwrap();
        }
        set_opened_at(&index, "b", NOW - 3 * DAY);
        set_opened_at(&index, "a", NOW - 2 * DAY);
        set_opened_at(&index, "c", NOW - DAY);
        let used = index.used_bytes().unwrap();
        let policy = EvictPolicy {
            max_age_secs: MAX_AGE_SECS,
            max_bytes: used - 250_000,
        };

        let summary = index.evict(NOW, policy).unwrap();
        assert_eq!((summary.folders, summary.rows), (2, 2));
        assert!(index.entries("b").unwrap().is_empty());
        assert!(index.entries("a").unwrap().is_empty());
        assert_eq!(index.entries("c").unwrap().len(), 1);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_dirty_rating_survives_the_eviction_of_its_folder() {
        let dir = temp_dir("evict-dirty");
        let mut index = open(&dir);
        let path = synthetic(&dir, 0).path.to_string_lossy().into_owned();
        index
            .write_batch("d", &[(synthetic(&dir, 0), Ok(entry()))])
            .unwrap();
        index
            .set_rating("d", &path, Some(5), Flag::Pick, None, true)
            .unwrap();
        set_opened_at(&index, "d", 0);

        let summary = index.evict(NOW, NO_CAP).unwrap();
        assert_eq!(summary.rows, 1);
        assert!(index.entries("d").unwrap().is_empty());
        assert_eq!(index.dirty_rows("d").unwrap().len(), 1);
        assert_eq!(opened_at(&index, "d"), Some(0));

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_vacuum_after_eviction_shrinks_the_file() {
        let dir = temp_dir("evict-vacuum");
        let mut index = open(&dir);
        let rows: Vec<_> = (0..10)
            .map(|i| (synthetic(&dir, i), Ok(big_entry())))
            .collect();
        index.write_batch("d", &rows).unwrap();
        set_opened_at(&index, "d", 0);
        let pages = |index: &Index| -> i64 {
            index
                .conn
                .pragma_query_value(None, "page_count", |r| r.get(0))
                .unwrap()
        };
        let before = pages(&index);

        assert!(index.evict(NOW, NO_CAP).unwrap().vacuumed);
        assert!(
            pages(&index) * 10 < before,
            "{} -> {}",
            before,
            pages(&index)
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn clear_empties_the_index_and_gives_the_space_back() {
        let dir = temp_dir("clear-vacuum");
        let mut index = open(&dir);
        for name in ["a", "b"] {
            let rows: Vec<_> = (0..5)
                .map(|i| (synthetic(&dir, i), Ok(big_entry())))
                .collect();
            index.write_batch(name, &rows).unwrap();
            set_opened_at(&index, name, NOW);
        }
        let pages = |index: &Index| -> i64 {
            index
                .conn
                .pragma_query_value(None, "page_count", |r| r.get(0))
                .unwrap()
        };
        let on_disk = || -> u64 {
            let db = dir.join("index.sqlite");
            [db.clone(), with_suffix(&db, "-wal")]
                .iter()
                .map(|p| std::fs::metadata(p).map_or(0, |m| m.len()))
                .sum()
        };
        let before = pages(&index);
        let before_disk = on_disk();

        let summary = index.clear().unwrap();
        assert_eq!(summary.folders, 2);
        assert!(summary.vacuumed);
        assert!(index.entries("a").unwrap().is_empty());
        assert!(index.entries("b").unwrap().is_empty());
        assert_eq!(opened_at(&index, "a"), None);
        assert!(
            pages(&index) * 10 < before,
            "pages {} -> {}",
            before,
            pages(&index)
        );
        // Without the `wal_checkpoint(TRUNCATE)` the rebuilt database stays in
        // the WAL and the footprint on disk does not drop at all.
        assert!(
            on_disk() < before_disk / 10,
            "bytes {} -> {}",
            before_disk,
            on_disk()
        );

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_dirty_rating_survives_a_clear() {
        let dir = temp_dir("clear-dirty");
        let mut index = open(&dir);
        let path = synthetic(&dir, 0).path.to_string_lossy().into_owned();
        index
            .write_batch("d", &[(synthetic(&dir, 0), Ok(entry()))])
            .unwrap();
        index
            .set_rating("d", &path, Some(5), Flag::Pick, None, true)
            .unwrap();
        set_opened_at(&index, "d", NOW);

        let summary = index.clear().unwrap();
        assert_eq!(summary.rows, 1);
        assert!(index.entries("d").unwrap().is_empty());
        assert_eq!(index.dirty_rows("d").unwrap().len(), 1);
        assert_eq!(opened_at(&index, "d"), Some(NOW));

        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_an_empty_index_does_nothing() {
        let dir = temp_dir("clear-empty");
        let mut index = open(&dir);

        assert_eq!(index.clear().unwrap(), EvictSummary::default());

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v7_database_drops_its_files_rows_and_gains_folders() {
        let dir = temp_dir("migrate-v7");
        let db = dir.join("index.sqlite");
        let a = synthetic(&dir, 0);
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            index
                .conn
                .execute_batch(
                    "DROP TABLE folders;
                     ALTER TABLE ratings RENAME COLUMN flag TO pick;
                     PRAGMA user_version = 7;",
                )
                .unwrap();
        }

        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert!(index.entries("d").unwrap().is_empty());
        assert_eq!(opened_at(&index, "d"), None);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v8_database_drops_its_files_rows_and_keeps_ratings_and_folders() {
        let dir = temp_dir("migrate-v8");
        let db = dir.join("index.sqlite");
        let a = synthetic(&dir, 0);
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            set_opened_at(&index, "d", NOW);
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating) VALUES ('x', 'd', 3);
                     ALTER TABLE ratings RENAME COLUMN flag TO pick;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 8;",
                )
                .unwrap();
        }

        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert!(index.entries("d").unwrap().is_empty());
        assert_eq!(opened_at(&index, "d"), Some(NOW));
        let rating: i64 = index
            .conn
            .query_row("SELECT rating FROM ratings WHERE path = 'x'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(rating, 3);

        remove_temp_dir(&dir);
    }

    #[test]
    fn a_v9_database_drops_its_files_rows_and_keeps_ratings_and_folders() {
        let dir = temp_dir("migrate-v9");
        let db = dir.join("index.sqlite");
        let a = synthetic(&dir, 0);
        {
            let mut index = open(&dir);
            index.write_batch("d", &[(a.clone(), Ok(entry()))]).unwrap();
            set_opened_at(&index, "d", NOW);
            index
                .conn
                .execute_batch(
                    "INSERT INTO ratings (path, dir, rating) VALUES ('x', 'd', 3);
                     ALTER TABLE ratings RENAME COLUMN flag TO pick;
                     ALTER TABLE folders DROP COLUMN last_viewed;
                     DROP TABLE views;
                     PRAGMA user_version = 9;",
                )
                .unwrap();
        }

        let index = Index::open(&db).unwrap();
        let version: i64 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert!(index.entries("d").unwrap().is_empty());
        assert_eq!(opened_at(&index, "d"), Some(NOW));
        let rating: i64 = index
            .conn
            .query_row("SELECT rating FROM ratings WHERE path = 'x'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(rating, 3);

        remove_temp_dir(&dir);
    }

    /// `VACUUM` time after evicting half of a ~100 MB index. Run with
    /// `cargo test -p riffle-app --release -- --ignored --nocapture vacuum_cost`.
    #[test]
    #[ignore]
    fn vacuum_cost_on_a_100_mb_index() {
        const N: usize = 5000;
        let dir = temp_dir("vacuum-cost");
        let mut index = open(&dir);
        let thumb = Entry {
            thumbnail: vec![0x55; 20_800],
            ..entry()
        };
        for (i, chunk) in (0..N).collect::<Vec<_>>().chunks(100).enumerate() {
            let name = format!("d{}", i % 2);
            let rows: Vec<_> = chunk
                .iter()
                .map(|&i| (synthetic(&dir, i), Ok(thumb.clone())))
                .collect();
            index.write_batch(&name, &rows).unwrap();
        }
        set_opened_at(&index, "d0", 0);
        set_opened_at(&index, "d1", NOW);
        let size = |index: &Index| index.used_bytes().unwrap() / 1_000_000;
        println!("before: {} MB in use", size(&index));
        let start = Instant::now();
        let summary = index.evict(NOW, NO_CAP).unwrap();
        println!(
            "evicted {} rows and vacuumed in {:.0} ms, {} MB left",
            summary.rows,
            start.elapsed().as_secs_f64() * 1000.0,
            size(&index)
        );
        remove_temp_dir(&dir);
    }

    /// Commit cost per chunk size. Run with
    /// `cargo test -p riffle-app --release -- --ignored --nocapture write_batch_cost`.
    #[test]
    #[ignore]
    fn write_batch_cost_per_chunk_size() {
        const N: usize = 5000;
        for chunk in [10, 50, 100] {
            let dir = temp_dir(&format!("cost-{chunk}"));
            let mut index = open(&dir);
            let rows: Vec<_> = (0..N).map(|i| (synthetic(&dir, i), Ok(entry()))).collect();
            let start = Instant::now();
            for batch in rows.chunks(chunk) {
                index.write_batch("d", batch).unwrap();
            }
            let total = start.elapsed();
            let per_tx = total.as_secs_f64() * 1000.0 / N.div_ceil(chunk) as f64;
            println!(
                "chunk {chunk}: {:.1} ms total, {per_tx:.3} ms per transaction",
                total.as_secs_f64() * 1000.0
            );
            drop(index);
            remove_temp_dir(&dir);
        }
    }
}
