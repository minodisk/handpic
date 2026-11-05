//! The folder tree's `Rename by Capture Date…`: the plan that names a folder
//! after the capture date of the photos directly in it, or splits it into
//! one sibling folder per date when they span several, and the
//! `date_folder_preview` command that serializes it for the confirmation
//! dialog, and the `date_folder_run` command that carries it out.
//!
//! The date is the Exif `DateTimeOriginal` as the camera wrote it, its local
//! date with no time-zone handling, taken from the index when the file has a
//! row and read from the file's bounded head otherwise. Only the files the
//! folder lists count (its RAWs, or its JPEGs when it is JPEG-only);
//! subfolders are not descended into. A target name that exists in the
//! parent is a collision, which the dialog shows as the reason the run is
//! disabled; nothing is merged. A split moves each dated file with its
//! sidecars into its sibling date folder, its index rows following, leaves
//! the undated files, and removes the folder only when it ends up empty.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

use crate::commands::{canonicalize, read_listing, AppIndex, AppWriter, Scans, SCAN_RUNNING};
use crate::index;
use crate::rename::{file_run, folder_target, rename_folder_blocking, FilePlan, Renamed};
use crate::sidecar::{existing_sidecar, SidecarFormat};
use crate::trash::Failure;

/// The `YYYY-MM-DD` of an Exif `YYYY:MM:DD HH:MM:SS` time, `None` for an
/// empty or malformed one (including the all-zero time some cameras write
/// when the clock was never set).
pub fn date_of(capture_time: &str) -> Option<String> {
    let date = capture_time.trim().get(..10)?;
    let bytes = date.as_bytes();
    let digits = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    if !(digits(0..4) && bytes[4] == b':' && digits(5..7) && bytes[7] == b':' && digits(8..10)) {
        return None;
    }
    let year: u32 = date[0..4].parse().ok()?;
    let month: u32 = date[5..7].parse().ok()?;
    let day: u32 = date[8..10].parse().ok()?;
    if year == 0 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{}-{}-{}", &date[0..4], &date[5..7], &date[8..10]))
}

/// What the folder becomes: renamed to its one date, split into one sibling
/// folder per date, or left alone because it is already named its one date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rename,
    Split,
    Named,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Rename => "rename",
            Self::Split => "split",
            Self::Named => "named",
        }
    }
}

/// The dated files going into the folder `name`, sorted by file name.
#[derive(Debug, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub files: Vec<PathBuf>,
}

/// The plan of a date rename or split: the groups in date order (one for a
/// rename or an already named folder), the undated files, which stay where
/// they are in a split and come along in a rename, sorted by file name, and
/// the target names that exist in the parent as another entry.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub kind: Kind,
    pub groups: Vec<Group>,
    pub undated: Vec<PathBuf>,
    pub collisions: Vec<String>,
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.to_string_lossy().into_owned(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Group the files of `dir`, each with its date (`date_of`), into the plan.
/// Refuses a folder with no parent (a root) and one with no dated file. A
/// rename's target is checked as `rename_folder` checks it; a split's
/// targets are siblings of `dir`, so any existing entry by that name, `dir`
/// itself included, is a collision.
pub fn plan(dir: &Path, dated: &[(PathBuf, Option<String>)]) -> Result<Plan, String> {
    let folder = file_name(dir);
    let parent = dir
        .parent()
        .ok_or_else(|| format!("{}: a root cannot be renamed", dir.display()))?;
    let mut by_date: BTreeMap<&str, Vec<PathBuf>> = BTreeMap::new();
    let mut undated = Vec::new();
    for (path, date) in dated {
        match date {
            Some(date) => by_date.entry(date).or_default().push(path.clone()),
            None => undated.push(path.clone()),
        }
    }
    if by_date.is_empty() {
        return Err(format!("{folder}: no photo has a capture date"));
    }
    let by_name = |a: &PathBuf, b: &PathBuf| a.file_name().cmp(&b.file_name());
    undated.sort_by(by_name);
    let groups: Vec<Group> = by_date
        .into_iter()
        .map(|(name, mut files)| {
            files.sort_by(by_name);
            Group {
                name: name.to_string(),
                files,
            }
        })
        .collect();
    let (kind, collisions) = if let [group] = groups.as_slice() {
        if group.name == folder {
            (Kind::Named, Vec::new())
        } else if folder_target(dir, &group.name).is_err() {
            (Kind::Rename, vec![group.name.clone()])
        } else {
            (Kind::Rename, Vec::new())
        }
    } else {
        let collisions = groups
            .iter()
            .filter(|group| parent.join(&group.name).symlink_metadata().is_ok())
            .map(|group| group.name.clone())
            .collect();
        (Kind::Split, collisions)
    };
    Ok(Plan {
        kind,
        groups,
        undated,
        collisions,
    })
}

/// The files `dir` lists, each with its date: the index's capture time from
/// `indexed` when the file has a row, else `read`'s.
fn dated(
    dir: &Path,
    indexed: &HashMap<String, Option<String>>,
    read: impl Fn(&Path) -> Option<String>,
) -> Result<Vec<(PathBuf, Option<String>)>, String> {
    let listing = read_listing(dir, None)?;
    Ok(listing
        .files
        .into_iter()
        .map(|path| {
            let time = match indexed.get(&path) {
                Some(time) => time.clone(),
                None => read(Path::new(&path)),
            };
            let date = time.as_deref().and_then(date_of);
            (PathBuf::from(path), date)
        })
        .collect())
}

/// One target folder of the dialog and the number of files going into it.
#[derive(Debug, PartialEq, Serialize)]
pub struct Target {
    name: String,
    count: usize,
}

/// What `date_folder_preview` gives the confirmation dialog: the kind
/// (`rename`, `split` or `named`), the folder's current name, the targets
/// with their file counts, the undated files' names and the colliding target
/// names, any of which disables the run.
#[derive(Debug, PartialEq, Serialize)]
pub struct Preview {
    kind: &'static str,
    name: String,
    targets: Vec<Target>,
    undated: Vec<String>,
    collisions: Vec<String>,
}

fn preview(dir: &Path, plan: Plan) -> Preview {
    Preview {
        kind: plan.kind.name(),
        name: file_name(dir),
        targets: plan
            .groups
            .into_iter()
            .map(|group| Target {
                name: group.name,
                count: group.files.len(),
            })
            .collect(),
        undated: plan.undated.iter().map(|path| file_name(path)).collect(),
        collisions: plan.collisions,
    }
}

/// Plan the canonical `dir` from its listing, the dates taken from the index
/// first.
fn plan_of(app: &tauri::AppHandle, dir: &str) -> Result<Plan, String> {
    let indexed: HashMap<String, Option<String>> = match app.state::<AppIndex>().0.clone() {
        Some(index) => index::lock(&index)
            .entries(dir)?
            .into_iter()
            .map(|entry| (entry.path, entry.capture_time))
            .collect(),
        None => HashMap::new(),
    };
    let dir = Path::new(dir);
    let files = dated(dir, &indexed, |path| {
        riffle_core::reader::read_metadata(path)
            .ok()
            .and_then(|arw| arw.shot.capture_time)
    })?;
    plan(dir, &files)
}

/// Plan the date rename or split of `dir` for the confirmation dialog,
/// refusing while a scan runs. Nothing on disk changes; the run plans again.
#[tauri::command]
pub async fn date_folder_preview(app: tauri::AppHandle, dir: String) -> Result<Preview, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<Preview, String> {
        let dir = canonicalize(&dir);
        let plan = plan_of(&app, &dir)?;
        Ok(preview(Path::new(&dir), plan))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The renames that move `file` into the folder `target`, file first: a RAW
/// takes the existing sidecars of both formats along, their targets minted
/// from the moved RAW's path (a `.dop` name embeds the RAW's whole name), and
/// a JPEG moves alone. Refuses any target that already exists, so nothing is
/// overwritten.
pub fn move_plan(file: &Path, target: &Path) -> Result<FilePlan, String> {
    let to = target.join(file.file_name().unwrap_or_default());
    let mut moves = vec![(file.to_path_buf(), to.clone())];
    if riffle_core::scan::is_raw_file(file) {
        for format in [SidecarFormat::Xmp, SidecarFormat::Dop] {
            if let Some(sidecar) = existing_sidecar(file, format) {
                moves.push((sidecar, format.sidecar_path(&to)));
            }
        }
    }
    if let Some((_, taken)) = moves.iter().find(|(_, to)| to.symlink_metadata().is_ok()) {
        return Err(format!("{} already exists", file_name(taken)));
    }
    Ok(FilePlan { moves })
}

/// One file a split moved: where it was, and the folder it is in now.
#[derive(Debug, PartialEq)]
struct Move {
    from: PathBuf,
    dir: PathBuf,
}

/// Create each group's folder in `parent` and move its files in with
/// `rename`, never stopping at a failure: a file whose move fails (rolled
/// back by `file_run`) is listed and stays, and a folder nothing moved into
/// is removed again. Returns each folder that received files with its count,
/// the failures and every file moved.
fn split(
    parent: &Path,
    groups: &[Group],
    mut rename: impl FnMut(&Path, &Path) -> Result<(), String>,
) -> (Vec<(String, usize)>, Vec<Failure>, Vec<Move>) {
    let mut counts = Vec::new();
    let mut failed = Vec::new();
    let mut moves = Vec::new();
    for group in groups {
        let target = parent.join(&group.name);
        if let Err(e) = std::fs::create_dir(&target) {
            failed.extend(group.files.iter().map(|file| Failure {
                path: file.to_string_lossy().into_owned(),
                message: format!("{}: {e}", group.name),
            }));
            continue;
        }
        let mut count = 0;
        for file in &group.files {
            match move_plan(file, &target).and_then(|plan| file_run(plan, &mut rename)) {
                Ok(()) => {
                    count += 1;
                    moves.push(Move {
                        from: file.clone(),
                        dir: target.clone(),
                    });
                }
                Err(message) => failed.push(Failure {
                    path: file.to_string_lossy().into_owned(),
                    message,
                }),
            }
        }
        if count == 0 {
            if let Err(e) = std::fs::remove_dir(&target) {
                log::warn!("could not remove the empty {}: {e}", target.display());
            }
        } else {
            counts.push((group.name.clone(), count));
        }
    }
    (counts, failed, moves)
}

/// Remove `dir` when it holds nothing, else list what is left in it, sorted.
/// `remove_dir` refuses a folder that is not empty, so an entry that appeared
/// meanwhile keeps it.
fn remove_if_empty(dir: &Path) -> (bool, Vec<String>) {
    let mut left: Vec<String> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(e) => {
            log::warn!("could not list {}: {e}", dir.display());
            return (false, Vec::new());
        }
    };
    if left.is_empty() {
        match std::fs::remove_dir(dir) {
            Ok(()) => return (true, left),
            Err(e) => log::warn!("could not remove {}: {e}", dir.display()),
        }
    }
    left.sort();
    (false, left)
}

/// One date folder a split moved files into: its name, its path spelled the
/// caller's way, and how many files went in.
#[derive(Debug, PartialEq, Serialize)]
pub struct Moved {
    name: String,
    path: String,
    count: usize,
}

/// What a split produced: the date folders with their file counts, the files
/// that could not be moved, the names left in the original folder, whether
/// the original folder was removed (its index state forgotten; the frontend
/// opens the first date folder if it was the open one), and a warning when
/// the disk changed but the index rows did not follow.
#[derive(Debug, Serialize)]
pub struct Split {
    moved: Vec<Moved>,
    failed: Vec<Failure>,
    left: Vec<String>,
    removed: bool,
    warning: Option<String>,
}

/// What `date_folder_run` did: a rename (`rename_folder`'s result) or a
/// split.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Ran {
    Renamed(Renamed),
    Split(Split),
}

/// Split the canonical `dir` (spelled `caller` by the frontend) along
/// `plan`, on a blocking thread. The guards are `rename_folder`'s: the
/// sidecar writer is drained first, so a sidecar a pending judgment has just
/// written moves too, and the `Scans` lock is held across the moves and the
/// index writes. The open folder's watcher and the tree watches on `dir` and
/// under it are released before the moves (Windows refuses to remove a
/// watched folder); the open folder's watcher is put back when `dir`
/// remains, and the tree re-render sets the tree watches again. A failed
/// index write after the moves still returns `Ok` with a warning, as in
/// `rename_folder`.
fn split_blocking(
    app: &tauri::AppHandle,
    dir: &str,
    caller: &str,
    plan: &Plan,
) -> Result<Split, String> {
    if let Some(writer) = &app.state::<AppWriter>().0 {
        writer.flush(crate::sidecar::DRAIN_TIMEOUT);
    }
    let scans = app.state::<Scans>();
    let state = index::lock(&scans.0);
    if state.scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    state.stop_embedding();
    let path = Path::new(dir);
    let parent = path
        .parent()
        .ok_or_else(|| format!("{dir}: a root cannot be renamed"))?;
    let released = crate::watch::release_under(app, dir);
    let ((counts, failed, moves), (removed, left)) =
        crate::treewatch::with_released(app, dir, || {
            let split = split(parent, &plan.groups, |from, to| {
                std::fs::rename(from, to).map_err(|e| format!("{}: {e}", file_name(from)))
            });
            Ok((split, remove_if_empty(path)))
        })?;
    if !removed {
        if let Some((dir, owner)) = released {
            crate::watch::set(app, &dir, &owner);
        }
    }
    let mut warning = None;
    if let Some(index) = app.state::<AppIndex>().0.clone() {
        let mut index = index::lock(&index);
        let mut result = moves.iter().try_for_each(|m| {
            let to_dir = canonicalize(&m.dir.to_string_lossy());
            let to = Path::new(&to_dir).join(m.from.file_name().unwrap_or_default());
            index.move_file(&m.from.to_string_lossy(), &to_dir, &to.to_string_lossy())
        });
        if removed && result.is_ok() {
            result = index.forget_dir(dir);
        }
        if let Err(e) = result {
            log::warn!("split folder {dir} but not its index rows: {e}");
            warning = Some(format!(
                "the files were moved but their cache was not ({e}); \
                 it is rebuilt on the next open"
            ));
        }
    }
    drop(state);
    let caller_parent = Path::new(caller).parent().unwrap_or(parent);
    let moved = counts
        .into_iter()
        .map(|(name, count)| Moved {
            path: caller_parent.join(&name).to_string_lossy().into_owned(),
            name,
            count,
        })
        .collect();
    let split = Split {
        moved,
        failed,
        left,
        removed,
        warning,
    };
    log::info!("split folder by capture date: {dir} {split:?}");
    Ok(split)
}

/// Carry out the date rename or split of `dir`, refusing while a scan runs.
/// The folder is planned again (the preview is not trusted across the
/// dialog) and the run refused on any collision or when the folder is
/// already named its date. One date renames the folder as `rename_folder`
/// does; several dates split it (`split_blocking`).
#[tauri::command]
pub async fn date_folder_run(app: tauri::AppHandle, dir: String) -> Result<Ran, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<Ran, String> {
        let canonical = canonicalize(&dir);
        let plan = plan_of(&app, &canonical)?;
        if !plan.collisions.is_empty() {
            return Err(format!("{} already exists", plan.collisions.join(", ")));
        }
        match plan.kind {
            Kind::Named => Err(format!(
                "{} is already named its capture date",
                file_name(Path::new(&canonical))
            )),
            Kind::Rename => {
                rename_folder_blocking(&app, &dir, &plan.groups[0].name).map(Ran::Renamed)
            }
            Kind::Split => split_blocking(&app, &canonical, &dir, &plan).map(Ran::Split),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("riffle-datefolder-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn folder(name: &str) -> PathBuf {
        let dir = temp_dir(name).join("100MSDCF");
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    fn file(dir: &Path, name: &str, date: Option<&str>) -> (PathBuf, Option<String>) {
        (dir.join(name), date.map(str::to_string))
    }

    #[test]
    fn date_of_takes_the_date_of_an_exif_time() {
        assert_eq!(
            date_of("2026:10:31 09:23:33").as_deref(),
            Some("2026-10-31")
        );
        assert_eq!(date_of("2026:10:31").as_deref(), Some("2026-10-31"));
    }

    #[test]
    fn date_of_refuses_an_empty_or_malformed_time() {
        for time in [
            "",
            "   ",
            "2026-10-31 09:23:33",
            "2026:1:31 09:23:33",
            "0000:00:00 00:00:00",
            "2026:13:01 00:00:00",
            "2026:10:00 00:00:00",
            "    :  :     :  :  ",
            "２０２６:10:31 09:23:33",
        ] {
            assert_eq!(date_of(time), None, "{time:?} was accepted");
        }
    }

    #[test]
    fn one_date_renames() {
        let dir = folder("rename");
        let files = [
            file(&dir, "b.ARW", Some("2026-10-31")),
            file(&dir, "a.ARW", Some("2026-10-31")),
            file(&dir, "c.ARW", None),
        ];
        let plan = plan(&dir, &files).unwrap();
        assert_eq!(plan.kind, Kind::Rename);
        assert_eq!(
            plan.groups,
            vec![Group {
                name: "2026-10-31".to_string(),
                files: vec![dir.join("a.ARW"), dir.join("b.ARW")],
            }]
        );
        assert_eq!(plan.undated, vec![dir.join("c.ARW")]);
        assert!(plan.collisions.is_empty());
    }

    #[test]
    fn several_dates_split_grouped_and_sorted() {
        let dir = folder("split");
        let files = [
            file(&dir, "d.ARW", Some("2026-11-01")),
            file(&dir, "c.ARW", Some("2026-10-31")),
            file(&dir, "z.ARW", None),
            file(&dir, "a.ARW", Some("2026-10-31")),
            file(&dir, "b.ARW", Some("2026-11-01")),
            file(&dir, "y.ARW", None),
        ];
        let plan = plan(&dir, &files).unwrap();
        assert_eq!(plan.kind, Kind::Split);
        assert_eq!(
            plan.groups,
            vec![
                Group {
                    name: "2026-10-31".to_string(),
                    files: vec![dir.join("a.ARW"), dir.join("c.ARW")],
                },
                Group {
                    name: "2026-11-01".to_string(),
                    files: vec![dir.join("b.ARW"), dir.join("d.ARW")],
                },
            ]
        );
        assert_eq!(plan.undated, vec![dir.join("y.ARW"), dir.join("z.ARW")]);
        assert!(plan.collisions.is_empty());
    }

    #[test]
    fn no_dated_file_is_refused() {
        let dir = folder("undated");
        assert!(plan(&dir, &[]).is_err());
        assert!(plan(&dir, &[file(&dir, "a.ARW", None)]).is_err());
    }

    #[test]
    fn a_folder_named_its_one_date_is_already_named() {
        let dir = temp_dir("named").join("2026-10-31");
        std::fs::create_dir(&dir).unwrap();
        let plan = plan(&dir, &[file(&dir, "a.ARW", Some("2026-10-31"))]).unwrap();
        assert_eq!(plan.kind, Kind::Named);
        assert!(plan.collisions.is_empty());
    }

    #[test]
    fn a_rename_onto_an_existing_sibling_collides() {
        let dir = folder("rename-collision");
        let parent = dir.parent().unwrap();
        std::fs::create_dir(parent.join("2026-10-31")).unwrap();
        std::fs::write(parent.join("2026-11-01"), b"").unwrap();
        for date in ["2026-10-31", "2026-11-01"] {
            let plan = plan(&dir, &[file(&dir, "a.ARW", Some(date))]).unwrap();
            assert_eq!(plan.kind, Kind::Rename);
            assert_eq!(plan.collisions, vec![date.to_string()]);
        }
    }

    #[test]
    fn a_split_onto_existing_siblings_collides() {
        let dir = folder("split-collision");
        let parent = dir.parent().unwrap();
        std::fs::create_dir(parent.join("2026-10-31")).unwrap();
        std::fs::write(parent.join("2026-11-02"), b"").unwrap();
        let files = [
            file(&dir, "a.ARW", Some("2026-10-31")),
            file(&dir, "b.ARW", Some("2026-11-01")),
            file(&dir, "c.ARW", Some("2026-11-02")),
        ];
        let plan = plan(&dir, &files).unwrap();
        assert_eq!(plan.kind, Kind::Split);
        assert_eq!(plan.collisions, vec!["2026-10-31", "2026-11-02"]);
    }

    #[test]
    fn a_split_target_named_like_the_folder_itself_collides() {
        let dir = temp_dir("split-self").join("2026-10-31");
        std::fs::create_dir(&dir).unwrap();
        let files = [
            file(&dir, "a.ARW", Some("2026-10-31")),
            file(&dir, "b.ARW", Some("2026-11-01")),
        ];
        let plan = plan(&dir, &files).unwrap();
        assert_eq!(plan.kind, Kind::Split);
        assert_eq!(plan.collisions, vec!["2026-10-31"]);
    }

    #[test]
    fn dated_takes_the_index_first_and_reads_the_rest() {
        let dir = folder("dated");
        for name in ["a.ARW", "b.ARW", "c.ARW", "notes.txt"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }
        std::fs::create_dir(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub").join("d.ARW"), b"").unwrap();
        let key = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let indexed = HashMap::from([
            (key("a.ARW"), Some("2026:10:31 09:23:33".to_string())),
            (key("b.ARW"), None),
        ]);
        let files = dated(&dir, &indexed, |path| {
            assert_eq!(path, dir.join("c.ARW"));
            Some("2026:11:01 10:00:00".to_string())
        })
        .unwrap();
        assert_eq!(
            files,
            vec![
                file(&dir, "a.ARW", Some("2026-10-31")),
                file(&dir, "b.ARW", None),
                file(&dir, "c.ARW", Some("2026-11-01")),
            ]
        );
    }

    #[test]
    fn a_jpeg_only_folder_is_planned_like_a_raw_folder() {
        let dir = folder("jpeg");
        for name in ["a.JPG", "b.jpg"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }
        let files = dated(&dir, &HashMap::new(), |path| {
            (path.file_name().unwrap() == "a.JPG").then(|| "2026:10:31 09:23:33".to_string())
        })
        .unwrap();
        let plan = plan(&dir, &files).unwrap();
        assert_eq!(plan.kind, Kind::Rename);
        assert_eq!(plan.groups[0].files, vec![dir.join("a.JPG")]);
        assert_eq!(plan.undated, vec![dir.join("b.jpg")]);
    }

    #[test]
    fn preview_counts_the_targets_and_names_the_undated() {
        let dir = folder("preview");
        let files = [
            file(&dir, "a.ARW", Some("2026-10-31")),
            file(&dir, "b.ARW", Some("2026-11-01")),
            file(&dir, "c.ARW", Some("2026-11-01")),
            file(&dir, "d.ARW", None),
        ];
        let preview = preview(&dir, plan(&dir, &files).unwrap());
        assert_eq!(
            preview,
            Preview {
                kind: "split",
                name: "100MSDCF".to_string(),
                targets: vec![
                    Target {
                        name: "2026-10-31".to_string(),
                        count: 1,
                    },
                    Target {
                        name: "2026-11-01".to_string(),
                        count: 2,
                    },
                ],
                undated: vec!["d.ARW".to_string()],
                collisions: Vec::new(),
            }
        );
    }

    fn touch(dir: &Path, names: &[&str]) {
        for name in names {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn fs_rename(from: &Path, to: &Path) -> Result<(), String> {
        std::fs::rename(from, to).map_err(|e| e.to_string())
    }

    fn group(name: &str, files: Vec<PathBuf>) -> Group {
        Group {
            name: name.to_string(),
            files,
        }
    }

    #[test]
    fn a_raw_moves_with_the_sidecars_of_both_formats_and_a_jpeg_alone() {
        let dir = folder("move-plan");
        let target = dir.parent().unwrap().join("2026-10-31");
        touch(&dir, &["a.ARW", "a.xmp", "a.ARW.dop", "b.JPG", "b.xmp"]);
        assert_eq!(
            move_plan(&dir.join("a.ARW"), &target).unwrap().moves,
            vec![
                (dir.join("a.ARW"), target.join("a.ARW")),
                (dir.join("a.xmp"), target.join("a.xmp")),
                (dir.join("a.ARW.dop"), target.join("a.ARW.dop")),
            ]
        );
        assert_eq!(
            move_plan(&dir.join("b.JPG"), &target).unwrap().moves,
            vec![(dir.join("b.JPG"), target.join("b.JPG"))]
        );
    }

    #[test]
    fn a_move_onto_an_existing_file_is_refused() {
        let dir = folder("move-collision");
        let target = dir.parent().unwrap().join("2026-10-31");
        std::fs::create_dir(&target).unwrap();
        touch(&dir, &["a.ARW", "a.xmp", "b.ARW"]);
        touch(&target, &["a.xmp", "b.ARW"]);
        assert!(move_plan(&dir.join("a.ARW"), &target).is_err());
        assert!(move_plan(&dir.join("b.ARW"), &target).is_err());
    }

    #[test]
    fn split_moves_each_file_into_its_date_folder_and_leaves_the_undated() {
        let dir = folder("split-run");
        let parent = dir.parent().unwrap();
        touch(&dir, &["a.ARW", "a.xmp", "b.ARW", "b.ARW.dop", "c.ARW"]);
        let groups = [
            group("2026-10-31", vec![dir.join("a.ARW")]),
            group("2026-11-01", vec![dir.join("b.ARW")]),
        ];
        let (counts, failed, moves) = split(parent, &groups, fs_rename);
        assert_eq!(
            counts,
            vec![("2026-10-31".to_string(), 1), ("2026-11-01".to_string(), 1)]
        );
        assert!(failed.is_empty());
        assert_eq!(
            moves,
            vec![
                Move {
                    from: dir.join("a.ARW"),
                    dir: parent.join("2026-10-31"),
                },
                Move {
                    from: dir.join("b.ARW"),
                    dir: parent.join("2026-11-01"),
                },
            ]
        );
        assert_eq!(names(&parent.join("2026-10-31")), ["a.ARW", "a.xmp"]);
        assert_eq!(names(&parent.join("2026-11-01")), ["b.ARW", "b.ARW.dop"]);
        assert_eq!(names(&dir), ["c.ARW"]);
    }

    #[test]
    fn a_failing_sidecar_rolls_its_file_back_and_the_rest_still_move() {
        let dir = folder("split-rollback");
        let parent = dir.parent().unwrap();
        touch(&dir, &["a.ARW", "a.xmp", "b.ARW", "c.ARW", "c.ARW.dop"]);
        let groups = [
            group("2026-10-31", vec![dir.join("a.ARW"), dir.join("b.ARW")]),
            group("2026-11-01", vec![dir.join("c.ARW")]),
        ];
        let refuse = |from: &Path, to: &Path| {
            if from
                .file_name()
                .is_some_and(|n| n == "a.xmp" || n == "c.ARW.dop")
            {
                return Err("refused".to_string());
            }
            fs_rename(from, to)
        };
        let (counts, failed, moves) = split(parent, &groups, refuse);
        assert_eq!(counts, vec![("2026-10-31".to_string(), 1)]);
        let failed: Vec<_> = failed.iter().map(|f| f.path.clone()).collect();
        assert_eq!(
            failed,
            [dir.join("a.ARW"), dir.join("c.ARW")].map(|p| p.to_string_lossy().into_owned())
        );
        assert_eq!(moves.len(), 1);
        assert_eq!(names(&parent.join("2026-10-31")), ["b.ARW"]);
        assert!(!parent.join("2026-11-01").exists());
        assert_eq!(names(&dir), ["a.ARW", "a.xmp", "c.ARW", "c.ARW.dop"]);
    }

    #[test]
    fn split_never_merges_into_an_existing_folder() {
        let dir = folder("split-existing");
        let parent = dir.parent().unwrap();
        std::fs::create_dir(parent.join("2026-10-31")).unwrap();
        touch(&dir, &["a.ARW"]);
        let groups = [group("2026-10-31", vec![dir.join("a.ARW")])];
        let (counts, failed, moves) = split(parent, &groups, fs_rename);
        assert!(counts.is_empty());
        assert_eq!(failed.len(), 1);
        assert!(moves.is_empty());
        assert_eq!(names(&dir), ["a.ARW"]);
        assert!(names(&parent.join("2026-10-31")).is_empty());
    }

    #[test]
    fn the_folder_is_removed_only_when_empty() {
        let dir = folder("remove-empty");
        assert_eq!(remove_if_empty(&dir), (true, Vec::new()));
        assert!(!dir.exists());

        let dir = folder("remove-left");
        touch(&dir, &["b.ARW", "a.txt"]);
        std::fs::create_dir(dir.join("sub")).unwrap();
        assert_eq!(
            remove_if_empty(&dir),
            (
                false,
                vec!["a.txt".to_string(), "b.ARW".to_string(), "sub".to_string()]
            )
        );
        assert!(dir.exists());
    }
}
