//! The folder tree's `Rename by Capture Date…`: the plan that names a folder
//! after the capture date of the photos directly in it, or splits it into
//! one sibling folder per date when they span several, and the
//! `date_folder_preview` command that serializes it for the confirmation
//! dialog.
//!
//! The date is the Exif `DateTimeOriginal` as the camera wrote it, its local
//! date with no time-zone handling, taken from the index when the file has a
//! row and read from the file's bounded head otherwise. Only the files the
//! folder lists count (its RAWs, or its JPEGs when it is JPEG-only);
//! subfolders are not descended into. A target name that exists in the
//! parent is a collision, which the dialog shows as the reason the run is
//! disabled; nothing is merged.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

use crate::commands::{canonicalize, read_listing, AppIndex, Scans, SCAN_RUNNING};
use crate::index;
use crate::rename::folder_target;

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

/// Plan the date rename or split of `dir` for the confirmation dialog,
/// refusing while a scan runs. Nothing on disk changes; the run plans again.
#[tauri::command]
pub async fn date_folder_preview(app: tauri::AppHandle, dir: String) -> Result<Preview, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<Preview, String> {
        let dir = canonicalize(&dir);
        let indexed: HashMap<String, Option<String>> = match app.state::<AppIndex>().0.clone() {
            Some(index) => index::lock(&index)
                .entries(&dir)?
                .into_iter()
                .map(|entry| (entry.path, entry.capture_time))
                .collect(),
            None => HashMap::new(),
        };
        let dir = Path::new(&dir);
        let files = dated(dir, &indexed, |path| {
            riffle_core::reader::read_metadata(path)
                .ok()
                .and_then(|arw| arw.shot.capture_time)
        })?;
        Ok(preview(dir, plan(dir, &files)?))
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
}
