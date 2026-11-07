//! Moving files of a folder to the OS trash, RAW first and its sidecars
//! after. The grouping and the per-file loop are pure so they can be tested
//! without touching the real Trash; only `commands::trash_files` supplies
//! the mover that calls into the `trash` crate.
//! Each run that moved something is recorded in `Runs`, and `restore` puts
//! a recorded run back for `commands::trash_rejected_undo`, again through
//! closures so the tests never touch the real Trash. What an undo brought
//! back is kept in `Runs` too, and `redo` moves exactly those files to the
//! Trash again for `commands::trash_rejected_redo`.
//! `delete_for_good` runs the same grouping and loop with a remover instead
//! of a mover, for `commands::delete_files`: nothing is recorded, as nothing
//! can come back.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use crate::index::SidecarStat;
use crate::sidecar::SidecarFormat;

/// One RAW and the sidecars that exist on disk for it, in the order they are
/// moved.
#[derive(Debug, PartialEq, Eq)]
pub struct Group {
    pub raw: PathBuf,
    pub sidecars: Vec<PathBuf>,
}

/// One file that could not be moved or restored.
#[derive(Debug, Serialize)]
pub struct Failure {
    pub path: String,
    pub message: String,
}

/// What `run` produces: the RAWs that went to the trash and the ones a move
/// attempt failed on.
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub moved: Vec<String>,
    pub failed: Vec<Failure>,
    /// The recorded run `trash_rejected_undo` takes back, `None` when nothing
    /// moved.
    pub run_id: Option<u64>,
}

/// One file a run moved to the Trash: where it came from and, when the
/// platform tells, where it went (`None` on Windows / Linux, where the
/// Trash is listed at undo time instead).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Trashed {
    pub path: PathBuf,
    pub trashed_at: Option<PathBuf>,
    /// `(dev, ino)` of the file at `trashed_at`, taken right after the move
    /// (macOS only; `None` elsewhere or when the file could not be stat'd).
    /// The Trash can reuse a name once it is free, so `restore_recorded`
    /// checks this against the file it is about to rename back, not just
    /// that a file exists at `trashed_at`.
    pub trashed_id: Option<(u64, u64)>,
    /// Where an undo puts the file back when a folder above it was renamed
    /// since the run (`Runs::rename_dir`); `None` puts it back at `path`,
    /// which stays the key the Trash is looked up by.
    pub restore_to: Option<PathBuf>,
}

impl Trashed {
    /// Where an undo puts the file back.
    pub fn destination(&self) -> &Path {
        self.restore_to.as_deref().unwrap_or(&self.path)
    }
}

/// One recorded run: every file it moved, each RAW followed by its sidecars
/// in the order they were moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashRun {
    pub id: u64,
    pub moved: Vec<Trashed>,
}

/// How many runs `Runs` keeps, the frontend's undo history limit.
pub const MAX_RUNS: usize = 100;

/// The error `trash_rejected_undo` returns for a run `Runs` no longer holds.
pub const RUN_GONE: &str = "this run can no longer be undone";
/// The error `trash_rejected_redo` returns for an undone run `Runs` no longer
/// holds.
pub const UNDONE_GONE: &str = "this run can no longer be redone";
/// The failure of a file redo finds gone from where the undo put it back.
pub const GONE: &str = "no longer at its location";

/// The recorded runs, the newest `MAX_RUNS`, managed as app state, and as
/// many undone runs, each holding only the files its undo brought back. The
/// ids come from a counter, so a forgotten run's id is never handed out
/// again.
#[derive(Default)]
pub struct Runs(Mutex<RunsState>);

#[derive(Default)]
struct RunsState {
    next_id: u64,
    runs: Vec<TrashRun>,
    undone: Vec<(u64, Vec<PathBuf>)>,
}

impl Runs {
    /// Record a run that moved `moved`, dropping the oldest past `MAX_RUNS`.
    /// Nothing moved records nothing.
    pub fn record(&self, moved: Vec<Trashed>) -> Option<u64> {
        if moved.is_empty() {
            return None;
        }
        let mut state = crate::index::lock(&self.0);
        state.next_id += 1;
        let id = state.next_id;
        state.runs.push(TrashRun { id, moved });
        if state.runs.len() > MAX_RUNS {
            state.runs.remove(0);
        }
        Some(id)
    }

    /// Remove and return the run `id`: an undone run is forgotten, whatever
    /// the outcome of its restore.
    pub fn take(&self, id: u64) -> Option<TrashRun> {
        let mut state = crate::index::lock(&self.0);
        let at = state.runs.iter().position(|run| run.id == id)?;
        Some(state.runs.remove(at))
    }

    /// Keep the files the undo of run `id` brought back, in the recorded
    /// order, for its redo, dropping the oldest past `MAX_RUNS`. Nothing
    /// back keeps nothing.
    pub fn undone(&self, id: u64, back: Vec<PathBuf>) {
        if back.is_empty() {
            return;
        }
        let mut state = crate::index::lock(&self.0);
        state.undone.push((id, back));
        if state.undone.len() > MAX_RUNS {
            state.undone.remove(0);
        }
    }

    /// Remove and return the files the undo of run `id` brought back: a
    /// redone run is forgotten, whatever the outcome of its move.
    pub fn take_undone(&self, id: u64) -> Option<Vec<PathBuf>> {
        let mut state = crate::index::lock(&self.0);
        let at = state.undone.iter().position(|(undone, _)| *undone == id)?;
        Some(state.undone.remove(at).1)
    }

    /// Follow the rename of the folder `old` to `new` (both canonical, as
    /// `Index::rename_dir` takes them): a recorded file under `old` is put
    /// back under `new` by its undo, still looked up in the Trash by its
    /// original path, and an undone run's files, which sit on disk under
    /// the renamed folder now, are redone from there.
    pub fn rename_dir(&self, old: &str, new: &str) {
        if old == new {
            return;
        }
        let old_prefix = format!("{old}{}", std::path::MAIN_SEPARATOR);
        let new_prefix = format!("{new}{}", std::path::MAIN_SEPARATOR);
        let rebase = |path: &Path| {
            path.to_string_lossy()
                .strip_prefix(&old_prefix)
                .map(|rest| PathBuf::from(format!("{new_prefix}{rest}")))
        };
        let mut state = crate::index::lock(&self.0);
        let state = &mut *state;
        for trashed in state.runs.iter_mut().flat_map(|run| run.moved.iter_mut()) {
            if let Some(to) = rebase(trashed.destination()) {
                trashed.restore_to = (to != trashed.path).then_some(to);
            }
        }
        for path in state
            .undone
            .iter_mut()
            .flat_map(|(_, back)| back.iter_mut())
        {
            if let Some(to) = rebase(path) {
                *path = to;
            }
        }
    }
}

/// What `trash_rejected_undo` returns: the RAWs (or, for a run of
/// `trash_files` in a JPEG folder, the JPEGs) that came back and every file,
/// RAW, JPEG or sidecar, that did not, with the reason.
#[derive(Debug, Default, Serialize)]
pub struct Restored {
    pub restored: Vec<String>,
    pub failed: Vec<Failure>,
    /// Every file, RAW or sidecar, that came back, in the recorded order:
    /// what a redo moves to the Trash again.
    #[serde(skip)]
    pub back: Vec<PathBuf>,
}

impl Restored {
    /// Every file of `run` failed with `message`, for a Trash that cannot be
    /// read or restored from at all.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub fn none(run: &TrashRun, message: &str) -> Self {
        Restored {
            restored: Vec::new(),
            failed: run
                .moved
                .iter()
                .map(|trashed| Failure {
                    path: trashed.destination().to_string_lossy().into_owned(),
                    message: message.to_string(),
                })
                .collect(),
            back: Vec::new(),
        }
    }
}

/// The sidecars of `format`'s kinds that exist for `path`, from the stats
/// keyed by lowercase file name.
pub(crate) fn sidecars_of<'a>(
    format: SidecarFormat,
    path: &str,
    sidecars: &'a HashMap<String, SidecarStat>,
) -> Vec<&'a SidecarStat> {
    format
        .kinds()
        .iter()
        .filter_map(|kind| {
            let name = kind
                .sidecar_path(Path::new(path))
                .file_name()?
                .to_string_lossy()
                .to_lowercase();
            sidecars.get(&name)
        })
        .collect()
}

/// One group per path of `paths` that lives directly in `dir`, in their
/// order, for `commands::trash_files`: a RAW with the `.xmp` and `.dop`
/// sidecars of `sidecars` (the listing of `dir`) that exist for it, whatever
/// format is selected, and a JPEG alone, since nothing next to a JPEG is read
/// or written. A path of another folder is returned as a `Failure` and not
/// grouped, so a sidecar of `dir` is never taken along with it.
pub fn groups_of(
    dir: &Path,
    paths: &[String],
    sidecars: &HashMap<String, SidecarStat>,
) -> (Vec<Group>, Vec<Failure>) {
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut groups = Vec::new();
    let mut foreign = Vec::new();
    for path in paths {
        let parent = Path::new(path).parent().unwrap_or_else(|| Path::new(""));
        let parent = std::fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
        if parent != dir {
            foreign.push(Failure {
                path: path.clone(),
                message: "not in the folder".to_string(),
            });
            continue;
        }
        groups.push(Group {
            raw: PathBuf::from(path),
            sidecars: if riffle_core::scan::is_raw_file(Path::new(path)) {
                sidecars_of(SidecarFormat::Both, path, sidecars)
                    .iter()
                    .map(|stat| stat.0.clone())
                    .collect()
            } else {
                Vec::new()
            },
        });
    }
    (groups, foreign)
}

/// Move every group with `mover`, never stopping at a failure. The RAW goes
/// first: when it fails its sidecars are left alone, so the judgment stays
/// with the file. `mover` returns where the file went when the platform
/// tells; every file moved is listed, in order, next to the summary.
/// `progress` is called after each group with the groups done and their
/// total; a group whose RAW failed counts as done.
pub fn run(
    groups: Vec<Group>,
    mut mover: impl FnMut(&Path) -> Result<Option<PathBuf>, String>,
    mut progress: impl FnMut(usize, usize),
) -> (Summary, Vec<Trashed>) {
    let mut summary = Summary::default();
    let mut moved = Vec::new();
    let total = groups.len();
    for (at, group) in groups.into_iter().enumerate() {
        let raw = group.raw.to_string_lossy().into_owned();
        match mover(&group.raw) {
            Ok(trashed_at) => {
                let trashed_id = trashed_at.as_deref().and_then(file_id);
                moved.push(Trashed {
                    path: group.raw,
                    trashed_at,
                    trashed_id,
                    restore_to: None,
                });
            }
            Err(message) => {
                summary.failed.push(Failure { path: raw, message });
                progress(at + 1, total);
                continue;
            }
        }
        summary.moved.push(raw);
        for sidecar in group.sidecars {
            match mover(&sidecar) {
                Ok(trashed_at) => {
                    let trashed_id = trashed_at.as_deref().and_then(file_id);
                    moved.push(Trashed {
                        path: sidecar,
                        trashed_at,
                        trashed_id,
                        restore_to: None,
                    });
                }
                Err(message) => summary.failed.push(Failure {
                    path: sidecar.to_string_lossy().into_owned(),
                    message,
                }),
            }
        }
        progress(at + 1, total);
    }
    (summary, moved)
}

/// The remover of `commands::delete_files`: the file is gone, not in the
/// Trash.
pub fn remove_one(path: &Path) -> Result<Option<PathBuf>, String> {
    std::fs::remove_file(path)
        .map(|()| None)
        .map_err(|e| e.to_string())
}

/// Delete `paths` of `dir` with `remover`, grouped and ordered as a move to
/// the Trash is: a RAW first, its sidecars only if it went, a JPEG alone, and
/// a path of another folder a `Failure`, never touched. The summary's
/// `run_id` stays `None`, so no undo is offered. `files` is the listing of
/// `dir`: a `.xmp` shared by name with a RAW that is not among `paths` (a
/// `FOO.xmp` of `FOO.ARW` and `FOO.DNG`) is kept, as nothing can bring it
/// back and the kept RAW still uses it.
pub fn delete_for_good(
    dir: &Path,
    paths: &[String],
    files: &[String],
    sidecars: &HashMap<String, SidecarStat>,
    remover: impl FnMut(&Path) -> Result<Option<PathBuf>, String>,
    progress: impl FnMut(usize, usize),
) -> Summary {
    let (mut groups, foreign) = groups_of(dir, paths, sidecars);
    let name_of = |path: &str| {
        Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
    };
    let stem_of = |path: &str| {
        Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_lowercase())
    };
    let chosen: std::collections::HashSet<String> = groups
        .iter()
        .filter_map(|group| name_of(&group.raw.to_string_lossy()))
        .collect();
    let kept_stems: std::collections::HashSet<String> = files
        .iter()
        .filter(|file| {
            riffle_core::scan::is_raw_file(Path::new(file.as_str()))
                && name_of(file).is_some_and(|name| !chosen.contains(&name))
        })
        .filter_map(|file| stem_of(file))
        .collect();
    for group in &mut groups {
        group.sidecars.retain(|sidecar| {
            let is_xmp = sidecar
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("xmp"));
            let shared = sidecar
                .file_stem()
                .is_some_and(|stem| kept_stems.contains(&stem.to_string_lossy().to_lowercase()));
            !(is_xmp && shared)
        });
    }
    // A `.xmp` shared by several selected RAWs goes after `run`, and only
    // when every one of them went: otherwise a RAW that failed would lose it.
    let mut users: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for group in &groups {
        for sidecar in &group.sidecars {
            if sidecar
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("xmp"))
            {
                users
                    .entry(sidecar.clone())
                    .or_default()
                    .push(group.raw.to_string_lossy().into_owned());
            }
        }
    }
    users.retain(|_, raws| raws.len() > 1);
    for group in &mut groups {
        group
            .sidecars
            .retain(|sidecar| !users.contains_key(sidecar));
    }
    let shared = users;
    let mut remover = remover;
    let (mut summary, _) = run(groups, &mut remover, progress);
    for (sidecar, raws) in shared {
        if !raws.iter().all(|raw| summary.moved.contains(raw)) {
            continue;
        }
        if let Err(message) = remover(&sidecar) {
            summary.failed.push(Failure {
                path: sidecar.to_string_lossy().into_owned(),
                message,
            });
        }
    }
    summary.failed.extend(foreign);
    summary
}

/// How many groups `run_chunked` moves per shell operation: about 30 times
/// faster per file than one operation each, as measured in the
/// trash-over-scan plan's `trash-bench.md`.
const CHUNK: usize = 100;

/// `run` for a platform where one call moves many files far faster than one
/// call per file (Windows): up to `CHUNK` RAWs in one `move_many`, then the
/// sidecars of the ones that went in another, so a sidecar still moves only
/// after its RAW. `moved` keeps `run`'s order, each RAW followed by its
/// sidecars, which `restore` and `redo` read. `progress` is called per chunk.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn run_chunked(
    mut groups: Vec<Group>,
    exists: impl Fn(&Path) -> bool,
    mut move_many: impl FnMut(&[PathBuf]) -> Result<(), String>,
    mut mover: impl FnMut(&Path) -> Result<Option<PathBuf>, String>,
    mut progress: impl FnMut(usize, usize),
) -> (Summary, Vec<Trashed>) {
    let mut summary = Summary::default();
    let mut moved = Vec::new();
    let total = groups.len();
    let mut done = 0;
    while !groups.is_empty() {
        let rest = groups.split_off(CHUNK.min(groups.len()));
        let chunk = std::mem::replace(&mut groups, rest);
        let raws: Vec<PathBuf> = chunk.iter().map(|group| group.raw.clone()).collect();
        let raw_results = move_chunk(&raws, &exists, &mut move_many, &mut mover);
        let sidecars: Vec<PathBuf> = chunk
            .iter()
            .zip(&raw_results)
            .filter(|(_, result)| result.is_ok())
            .flat_map(|(group, _)| group.sidecars.iter().cloned())
            .collect();
        let mut sidecar_results =
            move_chunk(&sidecars, &exists, &mut move_many, &mut mover).into_iter();
        done += chunk.len();
        for (group, raw_result) in chunk.into_iter().zip(raw_results) {
            let raw = group.raw.to_string_lossy().into_owned();
            match raw_result {
                Ok(trashed_at) => moved.push(trashed(group.raw, trashed_at)),
                Err(message) => {
                    summary.failed.push(Failure { path: raw, message });
                    continue;
                }
            }
            summary.moved.push(raw);
            for (sidecar, result) in group.sidecars.into_iter().zip(sidecar_results.by_ref()) {
                match result {
                    Ok(trashed_at) => moved.push(trashed(sidecar, trashed_at)),
                    Err(message) => summary.failed.push(Failure {
                        path: sidecar.to_string_lossy().into_owned(),
                        message,
                    }),
                }
            }
        }
        progress(done, total);
    }
    (summary, moved)
}

/// The outcome of each of `paths`, in order: where the file went, as `mover`
/// tells it, or why it did not. A file already gone fails without reaching
/// `move_many`, which would refuse the whole call for it. `move_many` can fail
/// having moved all but one file (the Windows shell goes on past a locked
/// file, then reports the operation aborted), so after a failure a file that
/// is gone went to the Trash, and the rest go through `mover` one at a time
/// so that each failure names its file.
fn move_chunk(
    paths: &[PathBuf],
    exists: &impl Fn(&Path) -> bool,
    move_many: &mut impl FnMut(&[PathBuf]) -> Result<(), String>,
    mover: &mut impl FnMut(&Path) -> Result<Option<PathBuf>, String>,
) -> Vec<Result<Option<PathBuf>, String>> {
    let was_there: Vec<bool> = paths.iter().map(|path| exists(path)).collect();
    let there: Vec<PathBuf> = paths
        .iter()
        .zip(&was_there)
        .filter(|(_, there)| **there)
        .map(|(path, _)| path.clone())
        .collect();
    let all_moved = there.is_empty() || move_many(&there).is_ok();
    paths
        .iter()
        .zip(was_there)
        .map(|(path, was_there)| {
            if !was_there {
                Err(GONE.to_string())
            } else if all_moved || !exists(path) {
                Ok(None)
            } else {
                mover(path)
            }
        })
        .collect()
}

/// The record of `path`, moved to `trashed_at`.
fn trashed(path: PathBuf, trashed_at: Option<PathBuf>) -> Trashed {
    let trashed_id = trashed_at.as_deref().and_then(file_id);
    Trashed {
        path,
        trashed_at,
        trashed_id,
        restore_to: None,
    }
}

/// `(dev, ino)` of `path`, `None` when it cannot be stat'd or the platform is
/// not Unix. A rename within the same volume keeps this identity, but the
/// Trash reusing a freed name after emptying does not.
#[cfg(unix)]
fn file_id(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn file_id(_path: &Path) -> Option<(u64, u64)> {
    None
}

/// The failure of a file whose original location holds a file again.
pub const ALREADY_EXISTS: &str = "already exists at the original location";
/// The failure of a file the Trash no longer holds.
pub const NOT_IN_TRASH: &str = "not in the Trash (emptied or restored by hand)";
/// The failure of a sidecar left in the Trash because its RAW did not come
/// back.
pub const RAW_NOT_RESTORED: &str = "left in the Trash: its RAW could not be restored";

/// Put the files of `run` back in the recorded order, never stopping at a
/// failure and never overwriting: a file whose destination `exists` fails,
/// as does one `in_trash` finds no item for; otherwise `mover` restores the
/// item. The RAW goes first, and when it does not come back its sidecars stay
/// in the Trash (each reported), the mirror of `run`'s rule: a reject sidecar
/// restored next to a different file that took the RAW's name would pin the
/// judgment on that file.
pub fn restore<I>(
    run: &TrashRun,
    mut in_trash: impl FnMut(&Trashed) -> Option<I>,
    exists: impl Fn(&Path) -> bool,
    mut mover: impl FnMut(&Trashed, I) -> Result<(), String>,
) -> Restored {
    let mut restored = Restored::default();
    let mut raw_back = true;
    for trashed in &run.moved {
        let is_media = riffle_core::scan::is_raw_file(&trashed.path)
            || riffle_core::scan::is_jpeg_file(&trashed.path);
        let destination = trashed.destination();
        let path = destination.to_string_lossy().into_owned();
        let result = if !is_media && !raw_back {
            Err(RAW_NOT_RESTORED.to_string())
        } else if exists(destination) {
            Err(ALREADY_EXISTS.to_string())
        } else {
            match in_trash(trashed) {
                Some(item) => mover(trashed, item),
                None => Err(NOT_IN_TRASH.to_string()),
            }
        };
        if is_media {
            raw_back = result.is_ok();
        }
        match result {
            Ok(()) => {
                restored.back.push(destination.to_path_buf());
                if is_media {
                    restored.restored.push(path);
                }
            }
            Err(message) => restored.failed.push(Failure { path, message }),
        }
    }
    restored
}

/// Move `paths`, what an undo brought back (each RAW followed by its
/// sidecars), to the Trash again with `run`, so its rule holds: a sidecar
/// moves only after its RAW. A sidecar with no RAW before it (a run of
/// `Delete Sidecars…` moves sidecars alone) is a group of its own, so one
/// failure does not hold back the others. No reject is collected again; a
/// file that no longer `exists` fails without reaching `mover`.
#[cfg_attr(windows, allow(dead_code))]
pub fn redo(
    paths: Vec<PathBuf>,
    exists: impl Fn(&Path) -> bool,
    mut mover: impl FnMut(&Path) -> Result<Option<PathBuf>, String>,
    progress: impl FnMut(usize, usize),
) -> (Summary, Vec<Trashed>) {
    run(
        redo_groups(paths),
        |path| {
            if exists(path) {
                mover(path)
            } else {
                Err(GONE.to_string())
            }
        },
        progress,
    )
}

/// `redo` through `run_chunked`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn redo_chunked(
    paths: Vec<PathBuf>,
    exists: impl Fn(&Path) -> bool,
    move_many: impl FnMut(&[PathBuf]) -> Result<(), String>,
    mover: impl FnMut(&Path) -> Result<Option<PathBuf>, String>,
    progress: impl FnMut(usize, usize),
) -> (Summary, Vec<Trashed>) {
    run_chunked(redo_groups(paths), exists, move_many, mover, progress)
}

/// The groups of `paths`, what an undo brought back.
fn redo_groups(paths: Vec<PathBuf>) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for path in paths {
        match groups.last_mut() {
            Some(group)
                if !riffle_core::scan::is_raw_file(&path)
                    && riffle_core::scan::is_raw_file(&group.raw) =>
            {
                group.sidecars.push(path)
            }
            _ => groups.push(Group {
                raw: path,
                sidecars: Vec::new(),
            }),
        }
    }
    groups
}

/// Put `run` back from where each file went, as recorded at trash time
/// (macOS): a file whose recorded location is gone (the Trash was emptied or
/// the file put back by hand) is "not in the Trash", else it is renamed back,
/// within the volume its Trash lives on. `std::fs::rename` replaces an
/// existing file on Unix, so `restore`'s `exists` check is the only guard
/// against overwriting. A file existing at the recorded location is not
/// enough: the Trash can hand the same name to a different file once the
/// original is gone (the Trash emptied, then something else trashed under
/// the same name), so this also requires the file's `(dev, ino)` to still
/// match the one recorded right after the move.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn restore_recorded(run: &TrashRun) -> Restored {
    restore(
        run,
        |trashed| {
            let at = trashed.trashed_at.clone()?;
            if file_id(&at) != trashed.trashed_id {
                return None;
            }
            Some(at)
        },
        Path::exists,
        |trashed, at| std::fs::rename(at, trashed.destination()).map_err(|e| e.to_string()),
    )
}

/// Create the missing folders of `dir` (a folder renamed since the run no
/// longer exists at its old path, and the Windows Recycle Bin does not
/// recreate it) and return exactly the ones created, deepest first.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn create_missing(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let created: Vec<PathBuf> = dir
        .ancestors()
        .take_while(|d| !d.as_os_str().is_empty() && !d.exists())
        .map(Path::to_path_buf)
        .collect();
    if !created.is_empty() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(created)
}

/// Remove the folders `created` (deepest first), each only while it is empty.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn remove_created(created: &[PathBuf]) {
    for dir in created {
        let _ = std::fs::remove_dir(dir);
    }
}

/// Move a file the Trash put back at its original path `from` to `to`, where
/// a folder rename since the run moved its folder (Windows / Linux, whose
/// Trash only restores to the original path), then remove the folders
/// `created` for the restore, each only while it is empty. A failed move
/// leaves the file at `from`.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn relocate(from: &Path, to: &Path, created: &[PathBuf]) -> Result<(), String> {
    std::fs::rename(from, to).map_err(|e| {
        format!(
            "put back at {} but not moved into the renamed folder: {e}",
            from.display()
        )
    })?;
    remove_created(created);
    Ok(())
}

/// Move `path` to the Trash through `NSFileManager`, as the `trash` crate's
/// `DeleteMethod::NsFileManager` does (no Automation permission needed),
/// but keep the URL the file got inside the Trash, which the crate discards.
#[cfg(target_os = "macos")]
pub fn trash_file(path: &Path) -> Result<PathBuf, String> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};

    let bytes = path.as_os_str().as_encoded_bytes();
    let path = match std::str::from_utf8(bytes) {
        Ok(utf8) => NSString::from_str(utf8),
        Err(_) => NSString::from_str(&percent_encode(bytes)),
    };
    let url = NSURL::fileURLWithPath(&path);
    let mut resulting = None;
    NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, Some(&mut resulting))
        .map_err(|e| format!("`trashItemAtURL` failed: {e}"))?;
    resulting
        .and_then(|url| url.path())
        .map(|path| PathBuf::from(path.to_string()))
        .ok_or_else(|| "the Trash did not tell where the file went".to_string())
}

/// `input` with every byte that is not part of valid UTF-8 percent-encoded,
/// the `trash` crate's spelling of a non-UTF-8 path.
#[cfg(target_os = "macos")]
fn percent_encode(input: &[u8]) -> String {
    let mut encoded = String::with_capacity(input.len());
    for chunk in input.utf8_chunks() {
        encoded.push_str(chunk.valid());
        for byte in chunk.invalid() {
            encoded.push_str(percent_encoding::percent_encode_byte(*byte));
        }
    }
    encoded
}

/// `path` spelled the way the Trash's listing is matched against: without
/// the Windows verbatim prefix the app's canonical paths carry (the listing
/// has none) and, on Windows, lowercase, since its file names are
/// case-insensitive.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn trash_key(path: &Path) -> String {
    let path = path.to_string_lossy();
    let path = if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(&path).to_string()
    };
    if cfg!(windows) {
        path.to_lowercase()
    } else {
        path
    }
}

/// The items of the Trash keyed by `trash_key` of their original path, the
/// newest `time_deleted` winning when the same path was trashed more than
/// once.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn newest_by_path(items: Vec<::trash::TrashItem>) -> HashMap<String, ::trash::TrashItem> {
    let mut newest: HashMap<String, ::trash::TrashItem> = HashMap::new();
    for item in items {
        let key = trash_key(&item.original_path());
        if newest
            .get(&key)
            .is_none_or(|kept| kept.time_deleted < item.time_deleted)
        {
            newest.insert(key, item);
        }
    }
    newest
}

#[cfg(test)]
mod tests {
    use super::{
        create_missing, delete_for_good, groups_of, newest_by_path, redo, redo_chunked, relocate,
        remove_one, restore, restore_recorded, run, run_chunked, trash_key, Group, Restored, Runs,
        Summary, TrashRun, Trashed, ALREADY_EXISTS, CHUNK, GONE, MAX_RUNS, NOT_IN_TRASH,
        RAW_NOT_RESTORED,
    };
    use crate::commands::{read_listing, stat_sidecars};
    use crate::sidecar::SidecarFormat;
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-trash-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path) {
        std::fs::write(path, b"x").unwrap();
    }

    fn key(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    /// A mover that renames into `trash` instead of the real Trash, and fails
    /// for any path whose file name contains `fail`.
    fn mover(trash: PathBuf) -> impl FnMut(&Path) -> Result<Option<PathBuf>, String> {
        move |path: &Path| {
            let name = path.file_name().unwrap();
            if name.to_string_lossy().contains("fail") {
                return Err("refused".to_string());
            }
            let to = trash.join(name);
            std::fs::rename(path, &to).map_err(|e| e.to_string())?;
            Ok(Some(to))
        }
    }

    fn trashed(path: &str) -> Trashed {
        Trashed {
            path: PathBuf::from(path),
            trashed_at: None,
            trashed_id: None,
            restore_to: None,
        }
    }

    fn trash_run(paths: &[&str]) -> TrashRun {
        TrashRun {
            id: 1,
            moved: paths.iter().map(|p| trashed(p)).collect(),
        }
    }

    /// Restore `run` against a Trash holding `in_trash` and a disk holding
    /// `on_disk`, recording the paths the mover was asked to restore.
    fn restore_with(
        run: &TrashRun,
        in_trash: &[&str],
        on_disk: &[&str],
    ) -> (Restored, Vec<String>) {
        let mut asked = Vec::new();
        let restored = restore(
            run,
            |t| {
                let path = t.path.to_string_lossy().into_owned();
                in_trash.contains(&path.as_str()).then_some(path)
            },
            |p| on_disk.contains(&p.to_string_lossy().as_ref()),
            |_, item| {
                asked.push(item);
                Ok(())
            },
        );
        (restored, asked)
    }

    fn failures(restored: &Restored) -> Vec<(&str, &str)> {
        restored
            .failed
            .iter()
            .map(|f| (f.path.as_str(), f.message.as_str()))
            .collect()
    }

    #[test]
    fn a_full_run_is_restored_raw_first() {
        let run = trash_run(&["a.ARW", "a.xmp", "a.ARW.dop", "b.DNG"]);
        let (restored, asked) = restore_with(&run, &["a.ARW", "a.xmp", "a.ARW.dop", "b.DNG"], &[]);
        assert_eq!(restored.restored, ["a.ARW", "b.DNG"]);
        assert!(restored.failed.is_empty());
        assert_eq!(asked, ["a.ARW", "a.xmp", "a.ARW.dop", "b.DNG"]);
        assert_eq!(
            restored.back,
            ["a.ARW", "a.xmp", "a.ARW.dop", "b.DNG"].map(PathBuf::from)
        );
    }

    #[test]
    fn only_what_came_back_is_kept_for_a_redo() {
        let run = trash_run(&["a.ARW", "a.xmp", "b.ARW", "b.xmp", "c.ARW", "c.xmp"]);
        let (restored, _) = restore_with(&run, &["a.ARW", "b.ARW", "b.xmp", "c.ARW"], &["a.ARW"]);
        assert_eq!(
            restored.back,
            ["b.ARW", "b.xmp", "c.ARW"].map(PathBuf::from)
        );
    }

    #[test]
    fn a_raw_back_already_keeps_its_sidecars_in_the_trash() {
        let run = trash_run(&["a.ARW", "a.xmp", "b.ARW", "b.xmp"]);
        let (restored, asked) =
            restore_with(&run, &["a.ARW", "a.xmp", "b.ARW", "b.xmp"], &["a.ARW"]);
        assert_eq!(restored.restored, ["b.ARW"]);
        assert_eq!(
            failures(&restored),
            [("a.ARW", ALREADY_EXISTS), ("a.xmp", RAW_NOT_RESTORED)]
        );
        assert_eq!(asked, ["b.ARW", "b.xmp"]);
    }

    #[test]
    fn a_raw_whose_restore_fails_keeps_its_sidecars_in_the_trash() {
        let run = trash_run(&["a.ARW", "a.xmp", "b.ARW"]);
        let mut asked = Vec::new();
        let restored = restore(
            &run,
            |t| Some(t.path.clone()),
            |_| false,
            |t, _| {
                asked.push(t.path.clone());
                if t.path == Path::new("a.ARW") {
                    Err("refused".to_string())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(restored.restored, ["b.ARW"]);
        assert_eq!(
            failures(&restored),
            [("a.ARW", "refused"), ("a.xmp", RAW_NOT_RESTORED)]
        );
        assert_eq!(asked, [PathBuf::from("a.ARW"), PathBuf::from("b.ARW")]);
    }

    #[test]
    fn a_sidecar_missing_from_the_trash_is_reported_and_its_raw_comes_back() {
        let run = trash_run(&["a.ARW", "a.xmp", "a.ARW.dop"]);
        let (restored, asked) = restore_with(&run, &["a.ARW", "a.ARW.dop"], &[]);
        assert_eq!(restored.restored, ["a.ARW"]);
        assert_eq!(failures(&restored), [("a.xmp", NOT_IN_TRASH)]);
        assert_eq!(asked, ["a.ARW", "a.ARW.dop"]);
    }

    #[test]
    fn an_emptied_trash_reports_every_file() {
        let run = trash_run(&["a.ARW", "a.xmp", "b.ARW"]);
        let (restored, asked) = restore_with(&run, &[], &[]);
        assert!(restored.restored.is_empty());
        assert_eq!(
            failures(&restored),
            [
                ("a.ARW", NOT_IN_TRASH),
                ("a.xmp", RAW_NOT_RESTORED),
                ("b.ARW", NOT_IN_TRASH)
            ]
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn an_unreadable_trash_fails_every_file() {
        let run = trash_run(&["a.ARW", "a.xmp"]);
        assert_eq!(
            failures(&Restored::none(&run, "no Trash")),
            [("a.ARW", "no Trash"), ("a.xmp", "no Trash")]
        );
    }

    #[test]
    fn a_recorded_run_is_renamed_back_never_over_a_file() {
        let dir = temp_dir("recorded");
        let trash = dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        let recorded = |name: &str| {
            let at = trash.join(name);
            write(&at);
            Trashed {
                path: dir.join(name),
                trashed_at: Some(at.clone()),
                trashed_id: super::file_id(&at),
                restore_to: None,
            }
        };
        let back = recorded("a.ARW");
        let back_sidecar = recorded("a.xmp");
        let taken = recorded("b.ARW");
        write(&taken.path);
        let emptied = Trashed {
            // Nothing is ever written at this path, so this covers the
            // Trash-emptied case: no file at `trashed_at` at all.
            path: dir.join("c.ARW"),
            trashed_at: Some(trash.join("c.ARW")),
            trashed_id: Some((0, 0)),
            restore_to: None,
        };
        let reused = Trashed {
            // A different file now occupies the recorded name (macOS Trash
            // name reuse), so this covers the id-mismatch case: something
            // exists at `trashed_at`, but its id does not match what was
            // recorded at trash time.
            path: dir.join("d.ARW"),
            trashed_at: Some(trash.join("d.ARW")),
            trashed_id: Some((0, 0)),
            restore_to: None,
        };
        write(&reused.trashed_at.clone().unwrap());
        let run = TrashRun {
            id: 1,
            moved: vec![
                back.clone(),
                back_sidecar.clone(),
                taken.clone(),
                emptied,
                reused.clone(),
            ],
        };
        let restored = restore_recorded(&run);
        assert_eq!(
            restored.restored,
            [back.path.to_string_lossy().into_owned()]
        );
        let failed: Vec<(String, &str)> = restored
            .failed
            .iter()
            .map(|f| (f.path.clone(), f.message.as_str()))
            .collect();
        assert_eq!(
            failed,
            [
                (taken.path.to_string_lossy().into_owned(), ALREADY_EXISTS),
                (
                    dir.join("c.ARW").to_string_lossy().into_owned(),
                    NOT_IN_TRASH
                ),
                (
                    dir.join("d.ARW").to_string_lossy().into_owned(),
                    NOT_IN_TRASH
                )
            ]
        );
        assert!(back.path.exists() && back_sidecar.path.exists());
        assert!(!back.trashed_at.unwrap().exists());
        assert!(taken.trashed_at.unwrap().exists());
        assert!(reused.trashed_at.unwrap().exists());
        assert!(!reused.path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn ns_file_manager_tells_where_the_file_went() {
        let dir = temp_dir("nsfilemanager");
        let file = dir.join("a.ARW");
        write(&file);
        let at = super::trash_file(&file).unwrap();
        assert!(!file.exists());
        assert!(at.exists());
        std::fs::rename(&at, &file).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn item(parent: &str, name: &str, time_deleted: i64) -> ::trash::TrashItem {
        ::trash::TrashItem {
            id: format!("{parent}/{name}@{time_deleted}").into(),
            name: name.into(),
            original_parent: PathBuf::from(parent),
            time_deleted,
        }
    }

    #[test]
    fn the_newest_item_of_a_path_wins() {
        let items = newest_by_path(vec![
            item("/photos", "a.ARW", 10),
            item("/photos", "a.ARW", 30),
            item("/photos", "a.ARW", 20),
            item("/photos", "b.ARW", 5),
        ]);
        assert_eq!(items.len(), 2);
        let a = &items[&trash_key(&Path::new("/photos").join("a.ARW"))];
        assert_eq!(a.time_deleted, 30);
    }

    #[cfg(not(windows))]
    #[test]
    fn a_plain_path_keys_as_it_is_spelled_off_windows() {
        assert_eq!(trash_key(Path::new("/photos/A.ARW")), "/photos/A.ARW");
    }

    #[cfg(windows)]
    #[test]
    fn a_verbatim_path_matches_the_listing_whatever_the_case() {
        assert_eq!(
            trash_key(Path::new(r"\\?\C:\Photos\A.ARW")),
            trash_key(Path::new(r"C:\photos\a.arw"))
        );
        assert_eq!(
            trash_key(Path::new(r"\\?\UNC\nas\share\A.ARW")),
            trash_key(Path::new(r"\\NAS\share\a.ARW"))
        );
        assert_eq!(trash_key(Path::new(r"\\?\C:\A.ARW")), r"c:\a.arw");
        let items = newest_by_path(vec![item(r"C:\Photos", "A.ARW", 1)]);
        assert!(items.contains_key(&trash_key(Path::new(r"\\?\C:\photos\a.ARW"))));
    }

    #[test]
    fn runs_keep_the_newest_and_forget_an_undone_one() {
        let runs = Runs::default();
        assert_eq!(runs.record(Vec::new()), None);
        let ids: Vec<u64> = (0..=MAX_RUNS)
            .map(|_| runs.record(vec![trashed("a.ARW")]).unwrap())
            .collect();
        assert_eq!(runs.take(ids[0]), None);
        let last = *ids.last().unwrap();
        assert_eq!(runs.take(ids[1]).unwrap().id, ids[1]);
        assert_eq!(runs.take(last).unwrap().moved, [trashed("a.ARW")]);
        assert_eq!(runs.take(last), None);
        assert!(runs.record(vec![trashed("b.ARW")]).unwrap() > last);
    }

    #[test]
    fn undone_runs_keep_the_newest_and_forget_a_redone_one() {
        let runs = Runs::default();
        runs.undone(1, Vec::new());
        assert_eq!(runs.take_undone(1), None);
        for id in 0..=MAX_RUNS as u64 {
            runs.undone(id, vec![PathBuf::from("a.ARW")]);
        }
        assert_eq!(runs.take_undone(0), None);
        assert_eq!(runs.take_undone(1), Some(vec![PathBuf::from("a.ARW")]));
        assert_eq!(runs.take_undone(1), None);
    }

    fn moved_to(path: &Path, to: Option<&Path>) -> Trashed {
        Trashed {
            restore_to: to.map(Path::to_path_buf),
            ..trashed(&key(path))
        }
    }

    #[test]
    fn a_folder_rename_moves_where_a_run_is_restored_to_but_not_its_trash_key() {
        let photos = Path::new("photos");
        let shoot = photos.join("shoot");
        let day1 = photos.join("day1");
        let day2 = photos.join("day2");
        let a = shoot.join("a.ARW");
        let b = shoot.join("sub").join("b.ARW");
        let sibling = photos.join("shoot2").join("c.ARW");
        let elsewhere = Path::new("other").join("d.ARW");
        let runs = Runs::default();
        let record = || {
            runs.record(
                [&a, &b, &sibling, &elsewhere]
                    .map(|path| trashed(&key(path)))
                    .to_vec(),
            )
            .unwrap()
        };
        let id = record();
        runs.rename_dir(&key(&shoot), &key(&shoot));
        runs.rename_dir(&key(&shoot), &key(&day1));
        assert_eq!(
            runs.take(id).unwrap().moved,
            [
                moved_to(&a, Some(&day1.join("a.ARW"))),
                moved_to(&b, Some(&day1.join("sub").join("b.ARW"))),
                moved_to(&sibling, None),
                moved_to(&elsewhere, None),
            ]
        );
        let id = record();
        runs.rename_dir(&key(&shoot), &key(&day1));
        runs.rename_dir(&key(&day1), &key(&day2));
        assert_eq!(
            runs.take(id).unwrap().moved[0],
            moved_to(&a, Some(&day2.join("a.ARW")))
        );
        let id = record();
        runs.rename_dir(&key(&shoot), &key(&day1));
        runs.rename_dir(&key(&day1), &key(&shoot));
        assert_eq!(runs.take(id).unwrap().moved[0], moved_to(&a, None));
    }

    #[test]
    fn a_folder_rename_moves_the_files_an_undo_brought_back() {
        let photos = Path::new("photos");
        let shoot = photos.join("shoot");
        let day1 = photos.join("day1");
        let runs = Runs::default();
        let back = vec![
            shoot.join("a.ARW"),
            shoot.join("a.xmp"),
            photos.join("shoot2").join("c.ARW"),
        ];
        runs.undone(1, back.clone());
        runs.undone(2, back.clone());
        runs.rename_dir(&key(&shoot), &key(&shoot));
        assert_eq!(runs.take_undone(2), Some(back.clone()));
        runs.rename_dir(&key(&shoot), &key(&day1));
        assert_eq!(
            runs.take_undone(1),
            Some(vec![
                day1.join("a.ARW"),
                day1.join("a.xmp"),
                photos.join("shoot2").join("c.ARW"),
            ])
        );
    }

    #[test]
    fn a_renamed_run_is_restored_to_its_destination() {
        let run = TrashRun {
            id: 1,
            moved: vec![
                moved_to(Path::new("a.ARW"), Some(Path::new("new/a.ARW"))),
                moved_to(Path::new("a.xmp"), Some(Path::new("new/a.xmp"))),
                moved_to(Path::new("b.ARW"), Some(Path::new("new/b.ARW"))),
            ],
        };
        let mut asked = Vec::new();
        let restored = restore(
            &run,
            |t| Some(key(&t.path)),
            |p| ["a.ARW", "new/b.ARW"].contains(&key(p).as_str()),
            |t, item| {
                asked.push((item, key(t.destination())));
                Ok(())
            },
        );
        assert_eq!(restored.restored, ["new/a.ARW"]);
        assert_eq!(failures(&restored), [("new/b.ARW", ALREADY_EXISTS)]);
        assert_eq!(restored.back, ["new/a.ARW", "new/a.xmp"].map(PathBuf::from));
        assert_eq!(
            asked,
            [
                ("a.ARW".to_string(), "new/a.ARW".to_string()),
                ("a.xmp".to_string(), "new/a.xmp".to_string())
            ]
        );
    }

    #[test]
    fn a_file_restored_under_the_old_name_is_moved_into_the_renamed_folder() {
        let dir = temp_dir("relocate");
        let from = dir.join("shoot").join("sub").join("a.ARW");
        let to = dir.join("day1").join("sub").join("a.ARW");
        std::fs::create_dir_all(from.parent().unwrap()).unwrap();
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        write(&from);
        let created = create_missing(from.parent().unwrap()).unwrap();
        assert!(created.is_empty());
        relocate(
            &from,
            &to,
            &[dir.join("shoot").join("sub"), dir.join("shoot")],
        )
        .unwrap();
        assert!(to.exists());
        assert!(!dir.join("shoot").exists());
        assert!(dir.exists());

        let from = dir.join("shoot").join("b.ARW");
        std::fs::create_dir_all(from.parent().unwrap()).unwrap();
        write(&from);
        assert!(relocate(&from, &dir.join("gone").join("b.ARW"), &[]).is_err());
        assert!(from.exists());

        // Only the folders the restore created go; a pre-existing empty one stays.
        let _ = std::fs::remove_dir_all(dir.join("shoot"));
        std::fs::create_dir_all(dir.join("shoot")).unwrap();
        let created = create_missing(&dir.join("shoot").join("sub")).unwrap();
        assert_eq!(created, [dir.join("shoot").join("sub")]);
        let from = dir.join("shoot").join("sub").join("c.ARW");
        write(&from);
        let to = dir.join("day1").join("sub").join("c.ARW");
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        relocate(&from, &to, &created).unwrap();
        assert!(to.exists());
        assert!(dir.join("shoot").exists());
        assert!(!dir.join("shoot").join("sub").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failing_raw_keeps_its_sidecars_and_is_reported() {
        let dir = temp_dir("raw-failure");
        let trash = dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        for name in ["ok.ARW", "ok.xmp", "fail.ARW", "fail.xmp"] {
            write(&dir.join(name));
        }
        let groups = ["fail", "ok"]
            .into_iter()
            .map(|name| Group {
                raw: dir.join(format!("{name}.ARW")),
                sidecars: vec![dir.join(format!("{name}.xmp"))],
            })
            .collect();
        let (summary, moved) = run(groups, mover(trash.clone()), |_, _| {});
        assert_eq!(
            moved,
            [
                Trashed {
                    path: dir.join("ok.ARW"),
                    trashed_at: Some(trash.join("ok.ARW")),
                    trashed_id: super::file_id(&trash.join("ok.ARW")),
                    restore_to: None,
                },
                Trashed {
                    path: dir.join("ok.xmp"),
                    trashed_at: Some(trash.join("ok.xmp")),
                    trashed_id: super::file_id(&trash.join("ok.xmp")),
                    restore_to: None,
                },
            ]
        );
        assert_eq!(summary.moved, [key(&dir.join("ok.ARW"))]);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("fail.ARW")));
        assert!(dir.join("fail.ARW").exists());
        assert!(dir.join("fail.xmp").exists());
        assert!(trash.join("ok.ARW").exists());
        assert!(trash.join("ok.xmp").exists());
    }

    #[test]
    fn progress_counts_every_group_a_failed_raw_included() {
        let dir = temp_dir("progress");
        let trash = dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        for name in ["ok.ARW", "ok.xmp", "fail.ARW", "fail.xmp", "last.ARW"] {
            write(&dir.join(name));
        }
        let groups = vec![
            Group {
                raw: dir.join("ok.ARW"),
                sidecars: vec![dir.join("ok.xmp")],
            },
            Group {
                raw: dir.join("fail.ARW"),
                sidecars: vec![dir.join("fail.xmp")],
            },
            Group {
                raw: dir.join("last.ARW"),
                sidecars: Vec::new(),
            },
        ];
        let mut calls = Vec::new();
        let (summary, _) = run(groups, mover(trash.clone()), |done, total| {
            calls.push((done, total));
        });
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(calls, [(1, 3), (2, 3), (3, 3)]);
    }

    #[test]
    fn a_failing_sidecar_is_reported_after_its_raw_went() {
        let dir = temp_dir("sidecar-failure");
        let trash = dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        write(&dir.join("d.ARW"));
        write(&dir.join("d.ARW.dop"));
        let groups = vec![Group {
            raw: dir.join("d.ARW"),
            sidecars: vec![dir.join("fail.xmp"), dir.join("d.ARW.dop")],
        }];
        let (summary, moved) = run(groups, mover(trash.clone()), |_, _| {});
        assert_eq!(
            moved.iter().map(|t| t.path.clone()).collect::<Vec<_>>(),
            [dir.join("d.ARW"), dir.join("d.ARW.dop")]
        );
        assert_eq!(summary.moved, [key(&dir.join("d.ARW"))]);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("fail.xmp")));
        assert!(trash.join("d.ARW").exists());
        assert!(trash.join("d.ARW.dop").exists());
    }

    #[test]
    fn a_run_of_jpegs_is_restored_and_listed() {
        let run = trash_run(&["a.jpg", "b.JPEG"]);
        let (restored, asked) = restore_with(&run, &["a.jpg", "b.JPEG"], &[]);
        assert_eq!(restored.restored, ["a.jpg", "b.JPEG"]);
        assert!(restored.failed.is_empty());
        assert_eq!(asked, ["a.jpg", "b.JPEG"]);
    }

    fn delete_in(
        dir: &Path,
        names: &[&str],
        remover: impl FnMut(&Path) -> Result<Option<PathBuf>, String>,
    ) -> Summary {
        let listing = read_listing(dir, Some(SidecarFormat::Both)).unwrap();
        let paths: Vec<String> = names.iter().map(|name| key(&dir.join(name))).collect();
        delete_for_good(
            dir,
            &paths,
            &listing.files,
            &stat_sidecars(&listing.sidecars),
            remover,
            |_, _| {},
        )
    }

    #[test]
    fn a_delete_removes_each_raw_then_its_sidecars_and_keeps_no_run() {
        let dir = temp_dir("delete-order");
        for name in ["a.ARW", "a.xmp", "a.ARW.dop", "b.ARW", "c.ARW"] {
            write(&dir.join(name));
        }
        let mut order = Vec::new();
        let summary = delete_in(&dir, &["a.ARW", "b.ARW"], |path| {
            order.push(path.file_name().unwrap().to_string_lossy().into_owned());
            remove_one(path)
        });
        assert_eq!(order[0], "a.ARW");
        assert_eq!(order[3], "b.ARW");
        order[1..3].sort();
        assert_eq!(order[1..3], ["a.ARW.dop", "a.xmp"]);
        assert_eq!(
            summary.moved,
            [key(&dir.join("a.ARW")), key(&dir.join("b.ARW"))]
        );
        assert!(summary.failed.is_empty());
        assert_eq!(summary.run_id, None);
        for name in ["a.ARW", "a.xmp", "a.ARW.dop", "b.ARW"] {
            assert!(!dir.join(name).exists(), "{name}");
        }
        assert!(dir.join("c.ARW").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_raw_a_delete_fails_on_keeps_its_sidecars_and_is_reported() {
        let dir = temp_dir("delete-failure");
        for name in ["fail.ARW", "fail.xmp", "ok.ARW", "ok.xmp"] {
            write(&dir.join(name));
        }
        let summary = delete_in(&dir, &["fail.ARW", "ok.ARW"], |path| {
            if path.file_name().unwrap() == "fail.ARW" {
                return Err("locked".to_string());
            }
            remove_one(path)
        });
        assert_eq!(summary.moved, [key(&dir.join("ok.ARW"))]);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("fail.ARW")));
        assert_eq!(summary.failed[0].message, "locked");
        assert!(dir.join("fail.ARW").exists());
        assert!(dir.join("fail.xmp").exists());
        assert!(!dir.join("ok.ARW").exists());
        assert!(!dir.join("ok.xmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_delete_keeps_an_xmp_a_kept_raw_shares() {
        let dir = temp_dir("delete-shared-xmp");
        for name in ["a.ARW", "a.DNG", "a.xmp", "a.ARW.dop", "b.ARW", "b.xmp"] {
            write(&dir.join(name));
        }
        let summary = delete_in(&dir, &["a.ARW", "b.ARW"], remove_one);
        assert_eq!(summary.moved.len(), 2);
        assert!(!dir.join("a.ARW").exists());
        assert!(!dir.join("a.ARW.dop").exists());
        assert!(dir.join("a.xmp").exists());
        assert!(dir.join("a.DNG").exists());
        assert!(!dir.join("b.xmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_delete_removes_an_xmp_two_selected_raws_share_once_both_went() {
        let dir = temp_dir("delete-shared-both");
        for name in ["a.ARW", "a.DNG", "a.xmp"] {
            write(&dir.join(name));
        }
        let summary = delete_in(&dir, &["a.ARW", "a.DNG"], remove_one);
        assert_eq!(summary.moved.len(), 2);
        assert!(summary.failed.is_empty(), "{:?}", summary.failed);
        assert!(!dir.join("a.xmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_delete_keeps_an_xmp_when_one_of_the_raws_sharing_it_fails() {
        let dir = temp_dir("delete-shared-failure");
        for name in ["a.ARW", "a.DNG", "a.xmp"] {
            write(&dir.join(name));
        }
        let summary = delete_in(&dir, &["a.ARW", "a.DNG"], |path| {
            if path.file_name().unwrap() == "a.ARW" {
                return Err("locked".to_string());
            }
            remove_one(path)
        });
        assert_eq!(summary.moved, [key(&dir.join("a.DNG"))]);
        assert_eq!(summary.failed.len(), 1);
        assert!(dir.join("a.ARW").exists());
        assert!(dir.join("a.xmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_refused_foreign_raw_does_not_stand_for_the_kept_one_of_its_name() {
        let dir = temp_dir("delete-foreign-same-name");
        let other = temp_dir("delete-foreign-same-name-other");
        for path in [
            dir.join("a.DNG"),
            dir.join("a.ARW"),
            dir.join("a.xmp"),
            other.join("a.DNG"),
        ] {
            write(&path);
        }
        let listing = read_listing(&dir, Some(SidecarFormat::Both)).unwrap();
        let paths = [key(&other.join("a.DNG")), key(&dir.join("a.ARW"))];
        let summary = delete_for_good(
            &dir,
            &paths,
            &listing.files,
            &stat_sidecars(&listing.sidecars),
            remove_one,
            |_, _| {},
        );
        assert_eq!(summary.moved, [key(&dir.join("a.ARW"))]);
        assert!(dir.join("a.DNG").exists());
        assert!(dir.join("a.xmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn a_delete_takes_a_jpeg_alone() {
        let dir = temp_dir("delete-jpeg");
        for name in ["d.jpg", "d.xmp"] {
            write(&dir.join(name));
        }
        let summary = delete_in(&dir, &["d.jpg"], remove_one);
        assert_eq!(summary.moved, [key(&dir.join("d.jpg"))]);
        assert!(!dir.join("d.jpg").exists());
        assert!(dir.join("d.xmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_delete_refuses_a_path_outside_the_folder_and_reports_it() {
        let dir = temp_dir("delete-foreign");
        let other = temp_dir("delete-foreign-other");
        for path in [dir.join("x.xmp"), dir.join("y.ARW"), other.join("x.ARW")] {
            write(&path);
        }
        let listing = read_listing(&dir, Some(SidecarFormat::Both)).unwrap();
        let paths = [key(&other.join("x.ARW")), key(&dir.join("y.ARW"))];
        let summary = delete_for_good(
            &dir,
            &paths,
            &listing.files,
            &stat_sidecars(&listing.sidecars),
            remove_one,
            |_, _| {},
        );
        assert_eq!(summary.moved, [key(&dir.join("y.ARW"))]);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, paths[0]);
        assert!(other.join("x.ARW").exists());
        assert!(dir.join("x.xmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn a_file_of_another_folder_is_not_grouped_with_the_sidecars_of_the_folder() {
        let dir = temp_dir("groups-of-foreign");
        let other = temp_dir("groups-of-foreign-other");
        for path in [dir.join("x.xmp"), dir.join("y.ARW"), other.join("x.ARW")] {
            write(&path);
        }
        let listing = read_listing(&dir, Some(SidecarFormat::Both)).unwrap();
        let sidecars = stat_sidecars(&listing.sidecars);
        let paths = [key(&other.join("x.ARW")), key(&dir.join("y.ARW"))];
        let (groups, foreign) = groups_of(&dir, &paths, &sidecars);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].raw.file_name().unwrap(), "y.ARW");
        assert_eq!(foreign.len(), 1);
        assert_eq!(foreign[0].path, paths[0]);
    }

    #[test]
    fn files_are_grouped_with_the_sidecars_that_exist_for_raws_only() {
        let dir = temp_dir("groups-of");
        for name in [
            "a.ARW",
            "a.xmp",
            "a.ARW.dop",
            "b.NEF",
            "B.XMP",
            "c.DNG",
            "d.jpg",
            "d.xmp",
        ] {
            write(&dir.join(name));
        }
        let listing = read_listing(&dir, Some(SidecarFormat::Both)).unwrap();
        let sidecars = stat_sidecars(&listing.sidecars);
        let paths = ["c.DNG", "a.ARW", "d.jpg", "b.NEF"].map(|name| key(&dir.join(name)));
        let (groups, foreign) = groups_of(&dir, &paths, &sidecars);
        assert!(foreign.is_empty());
        let found: Vec<(String, Vec<String>)> = groups
            .iter()
            .map(|g| {
                let mut names: Vec<String> = g
                    .sidecars
                    .iter()
                    .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                    .collect();
                names.sort();
                (
                    g.raw.file_name().unwrap().to_string_lossy().into_owned(),
                    names,
                )
            })
            .collect();
        assert_eq!(
            found,
            [
                ("c.DNG".to_string(), vec![]),
                (
                    "a.ARW".to_string(),
                    vec!["a.ARW.dop".to_string(), "a.xmp".to_string()]
                ),
                ("d.jpg".to_string(), vec![]),
                ("b.NEF".to_string(), vec!["B.XMP".to_string()]),
            ]
        );
    }

    #[test]
    fn a_run_of_sidecars_alone_is_restored_whole() {
        let run = trash_run(&["a.xmp", "a.ARW.dop", "b.xmp"]);
        let (restored, asked) = restore_with(&run, &["a.xmp", "a.ARW.dop", "b.xmp"], &[]);
        assert!(restored.restored.is_empty());
        assert!(restored.failed.is_empty());
        assert_eq!(asked, ["a.xmp", "a.ARW.dop", "b.xmp"]);
        assert_eq!(
            restored.back,
            ["a.xmp", "a.ARW.dop", "b.xmp"].map(PathBuf::from)
        );
    }

    #[test]
    fn a_redo_of_sidecars_alone_moves_each_on_its_own() {
        let dir = temp_dir("redo-sidecars");
        let trash = dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        for name in ["b.xmp", "c.ARW.dop"] {
            write(&dir.join(name));
        }
        let paths = ["a.xmp", "b.xmp", "c.ARW.dop"]
            .map(|name| dir.join(name))
            .to_vec();
        let (summary, moved) = redo(paths, Path::exists, mover(trash.clone()), |_, _| {});
        assert_eq!(
            summary.moved,
            [key(&dir.join("b.xmp")), key(&dir.join("c.ARW.dop"))]
        );
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("a.xmp")));
        assert_eq!(summary.failed[0].message, GONE);
        assert_eq!(
            moved.iter().map(|t| t.path.clone()).collect::<Vec<_>>(),
            ["b.xmp", "c.ARW.dop"].map(|name| dir.join(name))
        );
        assert!(trash.join("b.xmp").exists());
        assert!(trash.join("c.ARW.dop").exists());
    }

    #[test]
    fn a_redo_reports_progress_per_group() {
        let dir = temp_dir("redo-progress");
        let trash = dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        for name in ["a.ARW", "a.xmp", "c.ARW"] {
            write(&dir.join(name));
        }
        let paths = ["a.ARW", "a.xmp", "b.ARW", "c.ARW"]
            .map(|name| dir.join(name))
            .to_vec();
        let mut calls = Vec::new();
        redo(paths, Path::exists, mover(trash.clone()), |done, total| {
            calls.push((done, total));
        });
        assert_eq!(calls, [(1, 3), (2, 3), (3, 3)]);
    }

    #[test]
    fn a_redo_reports_a_file_gone_meanwhile_and_moves_the_rest() {
        let dir = temp_dir("redo");
        let trash = dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        for name in ["a.ARW", "a.xmp", "c.ARW", "c.ARW.dop"] {
            write(&dir.join(name));
        }
        let paths = ["a.ARW", "a.xmp", "b.ARW", "b.xmp", "c.ARW", "c.ARW.dop"]
            .map(|name| dir.join(name))
            .to_vec();
        let (summary, moved) = redo(paths, Path::exists, mover(trash.clone()), |_, _| {});
        assert_eq!(
            summary.moved,
            [key(&dir.join("a.ARW")), key(&dir.join("c.ARW"))]
        );
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("b.ARW")));
        assert_eq!(summary.failed[0].message, GONE);
        assert_eq!(
            moved.iter().map(|t| t.path.clone()).collect::<Vec<_>>(),
            ["a.ARW", "a.xmp", "c.ARW", "c.ARW.dop"].map(|name| dir.join(name))
        );
        for name in ["a.ARW", "a.xmp", "c.ARW", "c.ARW.dop"] {
            assert!(trash.join(name).exists());
            assert!(!dir.join(name).exists());
        }
    }

    /// A `move_many` that behaves like the Windows shell: it renames into
    /// `trash` all it can, going on past a `fail` file, then errs. Every call
    /// is kept in `calls`.
    fn move_many<'a>(
        trash: &'a Path,
        calls: &'a RefCell<Vec<Vec<String>>>,
    ) -> impl FnMut(&[PathBuf]) -> Result<(), String> + 'a {
        move |paths: &[PathBuf]| {
            calls.borrow_mut().push(
                paths
                    .iter()
                    .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
                    .collect(),
            );
            let mut result = Ok(());
            for path in paths {
                let name = path.file_name().unwrap();
                if name.to_string_lossy().contains("fail") {
                    result = Err("Some operations were aborted".to_string());
                } else {
                    std::fs::rename(path, trash.join(name)).unwrap();
                }
            }
            result
        }
    }

    fn chunk_dir(name: &str, files: &[&str]) -> (PathBuf, PathBuf) {
        let dir = temp_dir(name);
        let trash = dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        for file in files {
            write(&dir.join(file));
        }
        (dir, trash)
    }

    fn group(dir: &Path, raw: &str, sidecars: &[&str]) -> Group {
        Group {
            raw: dir.join(raw),
            sidecars: sidecars.iter().map(|name| dir.join(name)).collect(),
        }
    }

    fn moved_paths(moved: &[Trashed]) -> Vec<PathBuf> {
        moved.iter().map(|trashed| trashed.path.clone()).collect()
    }

    #[test]
    fn a_chunk_moves_its_raws_then_their_sidecars_and_records_them_raw_by_raw() {
        let (dir, trash) = chunk_dir("chunk", &["a.ARW", "a.xmp", "b.ARW", "c.ARW", "c.xmp"]);
        let groups = vec![
            group(&dir, "a.ARW", &["a.xmp"]),
            group(&dir, "b.ARW", &[]),
            group(&dir, "c.ARW", &["c.xmp"]),
        ];
        let calls = RefCell::new(Vec::new());
        let (summary, moved) = run_chunked(
            groups,
            Path::exists,
            move_many(&trash, &calls),
            |_: &Path| panic!("no per-file move after a chunk that went"),
            |_, _| {},
        );
        assert_eq!(
            calls.into_inner(),
            [vec!["a.ARW", "b.ARW", "c.ARW"], vec!["a.xmp", "c.xmp"]]
        );
        assert_eq!(
            moved_paths(&moved),
            ["a.ARW", "a.xmp", "b.ARW", "c.ARW", "c.xmp"].map(|name| dir.join(name))
        );
        assert!(moved.iter().all(|trashed| trashed.trashed_at.is_none()));
        assert_eq!(
            summary.moved,
            ["a.ARW", "b.ARW", "c.ARW"].map(|name| key(&dir.join(name)))
        );
        assert!(summary.failed.is_empty());
    }

    #[test]
    fn a_failed_chunk_falls_back_per_file_for_only_what_it_left() {
        let (dir, trash) = chunk_dir(
            "chunk-failure",
            &["a.ARW", "a.xmp", "fail.ARW", "fail.xmp", "c.ARW"],
        );
        let groups = vec![
            group(&dir, "a.ARW", &["a.xmp"]),
            group(&dir, "fail.ARW", &["fail.xmp"]),
            group(&dir, "c.ARW", &[]),
        ];
        let calls = RefCell::new(Vec::new());
        let mut one_by_one = Vec::new();
        let mut per_file = mover(trash.clone());
        let (summary, moved) = run_chunked(
            groups,
            Path::exists,
            move_many(&trash, &calls),
            |path: &Path| {
                one_by_one.push(path.to_path_buf());
                per_file(path)
            },
            |_, _| {},
        );
        assert_eq!(one_by_one, [dir.join("fail.ARW")]);
        assert_eq!(
            moved_paths(&moved),
            ["a.ARW", "a.xmp", "c.ARW"].map(|name| dir.join(name))
        );
        assert_eq!(
            summary.moved,
            ["a.ARW", "c.ARW"].map(|name| key(&dir.join(name)))
        );
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("fail.ARW")));
        assert_eq!(summary.failed[0].message, "refused");
        assert_eq!(
            calls.into_inner()[1],
            ["a.xmp"],
            "the failed RAW's sidecar stays out of the sidecars' chunk"
        );
        assert!(dir.join("fail.ARW").exists());
        assert!(dir.join("fail.xmp").exists());
    }

    #[test]
    fn a_failing_sidecar_of_a_chunk_is_reported_after_its_raw_went() {
        let (dir, trash) = chunk_dir("chunk-sidecar", &["a.ARW", "a.fail.xmp", "b.ARW", "b.xmp"]);
        let groups = vec![
            group(&dir, "a.ARW", &["a.fail.xmp"]),
            group(&dir, "b.ARW", &["b.xmp"]),
        ];
        let calls = RefCell::new(Vec::new());
        let (summary, moved) = run_chunked(
            groups,
            Path::exists,
            move_many(&trash, &calls),
            mover(trash.clone()),
            |_, _| {},
        );
        assert_eq!(
            moved_paths(&moved),
            ["a.ARW", "b.ARW", "b.xmp"].map(|name| dir.join(name))
        );
        assert_eq!(summary.moved.len(), 2);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("a.fail.xmp")));
    }

    #[test]
    fn a_file_gone_before_its_chunk_fails_alone() {
        let (dir, trash) = chunk_dir("chunk-gone", &["a.ARW", "c.ARW"]);
        let groups = vec![
            group(&dir, "a.ARW", &[]),
            group(&dir, "b.ARW", &["b.xmp"]),
            group(&dir, "c.ARW", &[]),
        ];
        let calls = RefCell::new(Vec::new());
        let (summary, moved) = run_chunked(
            groups,
            Path::exists,
            move_many(&trash, &calls),
            |_: &Path| panic!("the chunk of what is there goes whole"),
            |_, _| {},
        );
        assert_eq!(calls.into_inner(), [vec!["a.ARW", "c.ARW"]]);
        assert_eq!(
            moved_paths(&moved),
            ["a.ARW", "c.ARW"].map(|name| dir.join(name))
        );
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("b.ARW")));
        assert_eq!(summary.failed[0].message, GONE);
    }

    #[test]
    fn progress_counts_the_groups_of_each_chunk() {
        let (dir, trash) = chunk_dir("chunk-progress", &[]);
        let names: Vec<String> = (0..CHUNK + 1).map(|at| format!("{at}.ARW")).collect();
        for name in &names {
            write(&dir.join(name));
        }
        let groups = names.iter().map(|name| group(&dir, name, &[])).collect();
        let calls = RefCell::new(Vec::new());
        let mut progress = Vec::new();
        let (summary, _) = run_chunked(
            groups,
            Path::exists,
            move_many(&trash, &calls),
            mover(trash.clone()),
            |done, total| progress.push((done, total)),
        );
        assert_eq!(summary.moved.len(), CHUNK + 1);
        assert_eq!(calls.borrow().len(), 2);
        assert_eq!(progress, [(CHUNK, CHUNK + 1), (CHUNK + 1, CHUNK + 1)]);
    }

    #[test]
    fn a_chunked_redo_keeps_each_sidecar_behind_its_raw() {
        let (dir, trash) = chunk_dir("chunk-redo", &["a.ARW", "a.xmp", "c.ARW", "c.xmp"]);
        let paths = ["a.ARW", "a.xmp", "b.ARW", "b.xmp", "c.ARW", "c.xmp"]
            .map(|name| dir.join(name))
            .to_vec();
        let calls = RefCell::new(Vec::new());
        let (summary, moved) = redo_chunked(
            paths,
            Path::exists,
            move_many(&trash, &calls),
            mover(trash.clone()),
            |_, _| {},
        );
        assert_eq!(
            moved_paths(&moved),
            ["a.ARW", "a.xmp", "c.ARW", "c.xmp"].map(|name| dir.join(name))
        );
        assert_eq!(
            calls.into_inner(),
            [vec!["a.ARW", "c.ARW"], vec!["a.xmp", "c.xmp"]]
        );
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&dir.join("b.ARW")));
        assert_eq!(summary.failed[0].message, GONE);
    }
}
