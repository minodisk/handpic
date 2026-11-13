//! Moves the app's data from the folders of an earlier identifier to the
//! current one on the first launch under a new identifier.
//!
//! This runs before `tauri::Builder`, because the log and window-state plugins
//! open their files while being registered, so it cannot ask `app.path()` for
//! the folders and computes them by Tauri's own formulas instead.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Newest first: an entry's files win a conflict with a later entry's, since
/// each is merged in turn and an existing destination file wins.
const LEGACY_IDENTIFIERS: &[&str] = &["com.minodisk.handpic", "com.minodisk.riffle"];

/// One entry per pair of folders that needed moving; `Ok` and `Err` both hold
/// the line to log.
pub type Report = Vec<Result<String, String>>;

type Rename = fn(&Path, &Path) -> io::Result<()>;

/// `complete` is false when a folder of an earlier identifier could not be
/// moved whole. The caller must then not start the app: it would create fresh
/// files (an empty `index.sqlite`, `settings.json`, a WAL) in the new folder
/// next to the only real copy of the user's data, and the next launch could
/// no longer tell the two apart.
pub struct Outcome {
    pub report: Report,
    pub complete: bool,
}

pub fn run(identifier: &str) -> Outcome {
    migrate_under(
        &base_dirs(),
        identifier,
        |from, to| fs::rename(from, to),
        remove_leftovers,
    )
}

#[cfg(target_os = "macos")]
fn remove_leftovers(legacy: &str) -> Report {
    remove_macos_leftovers(legacy)
}

#[cfg(not(target_os = "macos"))]
fn remove_leftovers(_: &str) -> Report {
    Report::new()
}

/// `leftovers` runs once per legacy identifier whose folders all moved.
fn migrate_under(
    bases: &[PathBuf],
    identifier: &str,
    rename: Rename,
    mut leftovers: impl FnMut(&str) -> Report,
) -> Outcome {
    let mut report = Report::new();
    let mut complete = true;
    for legacy in LEGACY_IDENTIFIERS.iter().filter(|id| **id != identifier) {
        let pairs = pairs_under(bases, legacy, identifier);
        let moved = migrate_pairs(&pairs, rename);
        let all_moved = moved.iter().all(Result::is_ok);
        complete &= all_moved;
        report.extend(moved);
        if all_moved {
            report.extend(leftovers(legacy));
        }
    }
    Outcome { report, complete }
}

fn base_dirs() -> Vec<PathBuf> {
    #[allow(unused_mut)]
    let mut bases = vec![
        dirs::config_dir(),
        dirs::data_local_dir(),
        dirs::cache_dir(),
    ];
    #[cfg(target_os = "macos")]
    bases.push(dirs::home_dir().map(|home| home.join("Library/Logs")));
    bases.into_iter().flatten().collect()
}

/// The `(old, new)` folder under each base. On Windows the cache and local data
/// bases are one folder and on macOS the config and local data bases are, so a
/// pair is kept once per old path.
fn pairs_under(bases: &[PathBuf], old_id: &str, new_id: &str) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
    for base in bases {
        let old = base.join(old_id);
        if pairs.iter().all(|(seen, _)| *seen != old) {
            pairs.push((old, base.join(new_id)));
        }
    }
    pairs
}

fn migrate_pairs(pairs: &[(PathBuf, PathBuf)], rename: Rename) -> Report {
    pairs
        .iter()
        .filter(|(old, new)| old != new && old.is_dir())
        .map(|(old, new)| {
            let pair = format!("pair={}→{}", old.display(), new.display());
            match migrate_pair(old, new, rename) {
                Ok(()) => Ok(format!("{pair} moved")),
                Err(e) => Err(format!("{pair} {e}")),
            }
        })
        .collect()
}

/// `old` is removed only when every entry under it was moved. Anything else
/// leaves it for the next launch, which picks up where this one stopped; the
/// app does not run in between (see `Outcome`), so a destination that exists
/// is either a complete file moved earlier or the data of a Handpic that ran
/// before, and in both cases it wins.
fn migrate_pair(old: &Path, new: &Path, rename: Rename) -> Result<(), String> {
    let mut failures = Vec::new();
    move_tree(old, new, rename, &mut failures);
    if let Some(first) = failures.first() {
        return Err(match failures.len() - 1 {
            0 => first.clone(),
            more => format!("{first} (and {more} more)"),
        });
    }
    fs::remove_dir_all(old).map_err(|e| format!("{}: {e}", old.display()))
}

fn move_tree(from: &Path, to: &Path, rename: Rename, failures: &mut Vec<String>) {
    if let Err(e) = fs::create_dir_all(to) {
        failures.push(format!("{}: {e}", to.display()));
        return;
    }
    let entries = match fs::read_dir(from) {
        Ok(entries) => entries,
        Err(e) => {
            failures.push(format!("{}: {e}", from.display()));
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                failures.push(format!("{}: {e}", from.display()));
                continue;
            }
        };
        let source = entry.path();
        let dest = to.join(entry.file_name());
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => move_tree(&source, &dest, rename, failures),
            Ok(_) => {
                if let Err(e) = move_file(&source, &dest, rename) {
                    failures.push(format!("{}: {e}", source.display()));
                }
            }
            Err(e) => failures.push(format!("{}: {e}", source.display())),
        }
    }
}

/// A destination that already exists wins and the source is discarded, so the
/// old folder can always go away.
fn move_file(source: &Path, dest: &Path, rename: Rename) -> io::Result<()> {
    let partial = partial_path(dest);
    match fs::remove_file(&partial) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    match fs::symlink_metadata(dest) {
        Ok(_) => return fs::remove_file(source),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    if rename(source, dest).is_ok() {
        return Ok(());
    }
    copy_then_delete(source, dest, &partial)
}

/// The rename onto `dest` comes last, so a `dest` that exists is always a
/// complete file.
fn copy_then_delete(source: &Path, dest: &Path, partial: &Path) -> io::Result<()> {
    let copied = fs::copy(source, partial)?;
    let expected = fs::symlink_metadata(source)?.len();
    if copied != expected {
        let _ = fs::remove_file(partial);
        return Err(io::Error::other(format!(
            "copied {copied} of {expected} bytes"
        )));
    }
    fs::rename(partial, dest)?;
    fs::remove_file(source)
}

fn partial_path(dest: &Path) -> PathBuf {
    let mut name = dest.as_os_str().to_owned();
    name.push(".migrating");
    PathBuf::from(name)
}

#[cfg(target_os = "macos")]
fn remove_macos_leftovers(legacy: &str) -> Report {
    let Some(home) = dirs::home_dir() else {
        return Report::new();
    };
    macos_leftovers(&home, legacy)
        .iter()
        .filter_map(|path| {
            let removed = if path.is_dir() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            };
            match removed {
                Ok(()) => Some(Ok(format!("removed {}", path.display()))),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => Some(Err(format!("{}: {e}", path.display()))),
            }
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn macos_leftovers(home: &Path, legacy: &str) -> Vec<PathBuf> {
    let library = home.join("Library");
    vec![
        library.join("WebKit").join(legacy),
        library.join("HTTPStorages").join(legacy),
        library.join("Preferences").join(format!("{legacy}.plist")),
        library
            .join("Saved Application State")
            .join(format!("{legacy}.savedState")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_rename(from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn cross_volume(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::other("cross-device link"))
    }

    fn temp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("riffle-app-migrate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn pair(root: &Path) -> (PathBuf, PathBuf) {
        (root.join("old"), root.join("new"))
    }

    fn all_moved(report: &Report) -> bool {
        report.len() == 1 && report[0].is_ok()
    }

    #[test]
    fn moves_every_entry_and_removes_the_old_folder() {
        let root = temp_root("plain");
        let (old, new) = pair(&root);
        write(&old.join("index.sqlite"), "db");
        write(&old.join("index.sqlite-wal"), "wal");
        write(&old.join("logs/Riffle.log"), "log");
        write(&old.join("EBWebView/Default/Cache/data_0"), "web");
        let report = migrate_pairs(&[(old.clone(), new.clone())], real_rename);
        assert!(all_moved(&report), "{report:?}");
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(new.join("index.sqlite")).unwrap(), "db");
        assert_eq!(
            fs::read_to_string(new.join("index.sqlite-wal")).unwrap(),
            "wal"
        );
        assert_eq!(
            fs::read_to_string(new.join("logs/Riffle.log")).unwrap(),
            "log"
        );
        assert_eq!(
            fs::read_to_string(new.join("EBWebView/Default/Cache/data_0")).unwrap(),
            "web"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn falls_back_to_a_copy_when_the_rename_fails() {
        let root = temp_root("cross-volume");
        let (old, new) = pair(&root);
        write(&old.join("sub/settings.json"), "settings");
        let report = migrate_pairs(&[(old.clone(), new.clone())], cross_volume);
        assert!(all_moved(&report), "{report:?}");
        assert!(!old.exists());
        assert_eq!(
            fs::read_to_string(new.join("sub/settings.json")).unwrap(),
            "settings"
        );
        assert!(!new.join("sub/settings.json.migrating").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resumes_an_interrupted_run() {
        let root = temp_root("resume");
        let (old, new) = pair(&root);
        write(&new.join("moved"), "complete");
        write(&old.join("moved"), "complete");
        write(&old.join("pending"), "pending");
        write(&new.join("pending.migrating"), "half");
        let report = migrate_pairs(&[(old.clone(), new.clone())], cross_volume);
        assert!(all_moved(&report), "{report:?}");
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(new.join("moved")).unwrap(), "complete");
        assert_eq!(fs::read_to_string(new.join("pending")).unwrap(), "pending");
        assert!(!new.join("pending.migrating").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_rename_that_succeeds_clears_a_stale_partial() {
        let root = temp_root("stale");
        let (old, new) = pair(&root);
        write(&old.join("pending"), "pending");
        write(&new.join("pending.migrating"), "half");
        let report = migrate_pairs(&[(old.clone(), new.clone())], real_rename);
        assert!(all_moved(&report), "{report:?}");
        assert!(!new.join("pending.migrating").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn keeps_a_destination_that_has_data_and_discards_the_source() {
        let root = temp_root("existing");
        let (old, new) = pair(&root);
        write(&old.join("settings.json"), "old");
        write(&new.join("settings.json"), "new");
        let report = migrate_pairs(&[(old.clone(), new.clone())], real_rename);
        assert!(all_moved(&report), "{report:?}");
        assert!(!old.exists());
        assert_eq!(
            fs::read_to_string(new.join("settings.json")).unwrap(),
            "new"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failed_pair_resumes_without_replacing_what_landed() {
        let root = temp_root("resume-landed");
        let (old, new) = pair(&root);
        // The copy landed but removing the source failed; nothing ran since.
        write(&old.join("settings.json"), "old");
        write(&new.join("settings.json"), "old");
        write(&old.join("index.sqlite"), "db");
        write(&old.join("index.sqlite-wal"), "wal");
        let report = migrate_pairs(&[(old.clone(), new.clone())], real_rename);
        assert!(all_moved(&report), "{report:?}");
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(new.join("index.sqlite")).unwrap(), "db");
        assert_eq!(
            fs::read_to_string(new.join("index.sqlite-wal")).unwrap(),
            "wal"
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn assert_reported_and_kept(report: &Report, source: &Path) {
        assert_eq!(report.len(), 1);
        let line = report[0].as_ref().unwrap_err();
        assert!(line.contains("pair="), "{line}");
        assert!(line.contains(&*source.to_string_lossy()), "{line}");
        assert!(source.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_read_only_source_is_reported_and_nothing_is_deleted() {
        use std::os::unix::fs::PermissionsExt;
        let root = temp_root("read-only");
        let (old, new) = pair(&root);
        let source = old.join("index.sqlite");
        write(&source, "db");
        fs::set_permissions(&old, fs::Permissions::from_mode(0o555)).unwrap();
        let report = migrate_pairs(&[(old.clone(), new.clone())], real_rename);
        fs::set_permissions(&old, fs::Permissions::from_mode(0o755)).unwrap();
        assert_reported_and_kept(&report, &source);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn a_source_that_cannot_be_removed_is_reported_and_nothing_is_deleted() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;
        let root = temp_root("read-only");
        let (old, new) = pair(&root);
        let source = old.join("index.sqlite");
        write(&source, "db");
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&source)
            .unwrap();
        let report = migrate_pairs(&[(old.clone(), new.clone())], real_rename);
        assert_reported_and_kept(&report, &source);
        drop(held);
        let retry = migrate_pairs(&[(old.clone(), new.clone())], real_rename);
        assert!(all_moved(&retry), "{retry:?}");
        assert!(!old.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn identical_paths_are_left_alone() {
        let root = temp_root("identical");
        let dir = root.join("same");
        write(&dir.join("settings.json"), "keep");
        assert!(migrate_pairs(&[(dir.clone(), dir.clone())], real_rename).is_empty());
        assert_eq!(
            fs::read_to_string(dir.join("settings.json")).unwrap(),
            "keep"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_missing_old_folder_is_skipped() {
        let root = temp_root("missing");
        let (old, new) = pair(&root);
        assert!(migrate_pairs(&[(old, new.clone())], real_rename).is_empty());
        assert!(!new.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_base_listed_twice_yields_one_pair() {
        let bases = [PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("a")];
        assert_eq!(
            pairs_under(&bases, "old", "new"),
            [
                (Path::new("a").join("old"), Path::new("a").join("new")),
                (Path::new("b").join("old"), Path::new("b").join("new")),
            ]
        );
    }

    fn no_leftovers(_: &str) -> Report {
        Report::new()
    }

    #[test]
    fn the_current_identifier_is_not_a_migration() {
        let root = temp_root("current");
        let current = LEGACY_IDENTIFIERS[LEGACY_IDENTIFIERS.len() - 1];
        write(&root.join(current).join("settings.json"), "keep");
        let outcome = migrate_under(
            std::slice::from_ref(&root),
            current,
            real_rename,
            no_leftovers,
        );
        assert!(outcome.report.is_empty() && outcome.complete);
        assert_eq!(
            fs::read_to_string(root.join(current).join("settings.json")).unwrap(),
            "keep"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn every_legacy_folder_merges_and_the_newest_wins_a_conflict() {
        let root = temp_root("legacies");
        let handpic = root.join("com.minodisk.handpic");
        let riffle = root.join("com.minodisk.riffle");
        let current = root.join("dev.minodisk.handpic");
        write(&handpic.join("settings.json"), "handpic");
        write(&handpic.join("ranking.json"), "ranking");
        write(&riffle.join("settings.json"), "riffle");
        write(&riffle.join("logs/Riffle.log"), "log");
        let mut cleaned = Vec::new();
        let outcome = migrate_under(
            std::slice::from_ref(&root),
            "dev.minodisk.handpic",
            real_rename,
            |legacy| {
                cleaned.push(legacy.to_string());
                Report::new()
            },
        );
        assert!(outcome.complete, "{:?}", outcome.report);
        assert_eq!(outcome.report.len(), 2, "{:?}", outcome.report);
        assert_eq!(cleaned, ["com.minodisk.handpic", "com.minodisk.riffle"]);
        assert!(!handpic.exists());
        assert!(!riffle.exists());
        assert_eq!(
            fs::read_to_string(current.join("settings.json")).unwrap(),
            "handpic"
        );
        assert_eq!(
            fs::read_to_string(current.join("ranking.json")).unwrap(),
            "ranking"
        );
        assert_eq!(
            fs::read_to_string(current.join("logs/Riffle.log")).unwrap(),
            "log"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_macos_leftovers_are_named_after_the_legacy_identifier() {
        assert_eq!(
            macos_leftovers(Path::new("/h"), "a.b"),
            [
                PathBuf::from("/h/Library/WebKit/a.b"),
                PathBuf::from("/h/Library/HTTPStorages/a.b"),
                PathBuf::from("/h/Library/Preferences/a.b.plist"),
                PathBuf::from("/h/Library/Saved Application State/a.b.savedState"),
            ]
        );
    }
}
