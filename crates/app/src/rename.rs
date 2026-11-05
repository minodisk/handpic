//! Renaming a folder from the folder tree and a RAW file from the strip. The
//! name check and the target planning are pure so they can be tested on temp
//! dirs; `rename_folder` releases the folder watcher, renames on disk and
//! carries the index rows along, so nothing is re-extracted and no judgment
//! is lost, and `rename_file` does the same for one RAW and its sidecars.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

use crate::commands::{AppIndex, AppWriter, Scans, SCAN_RUNNING};
use crate::index;
use crate::sidecar::{existing_sidecar, SidecarFormat};

/// What a rename produced: the new path, spelled the caller's way, and a
/// warning when the disk rename succeeded but something after it (carrying
/// the index rows along) did not.
#[derive(Debug, Serialize)]
pub struct Renamed {
    pub path: String,
    pub warning: Option<String>,
}

/// Refuse a name that is not a single valid file name component. The
/// characters Windows forbids are refused on every platform, so a name typed
/// on macOS still copies to an NTFS card.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("the name is empty".to_string());
    }
    if name == "." || name == ".." {
        return Err(format!("{name}: not a valid name"));
    }
    if let Some(c) = name.chars().find(|c| {
        matches!(c, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control()
    }) {
        return Err(format!("{name}: a name cannot contain {c:?}"));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err(format!("{name}: a name cannot end with a dot or a space"));
    }
    Ok(())
}

/// The path `dir` is renamed to: `name` in `dir`'s parent. Refuses a `dir`
/// with no parent (a drive root or `/`), an invalid name, and a target that
/// exists as another entry. A target that canonicalizes to `dir` itself is
/// `dir` spelled in another case on a case-insensitive file system, which is
/// a case-only rename, not a collision.
pub fn folder_target(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let parent = dir
        .parent()
        .ok_or_else(|| format!("{}: a root cannot be renamed", dir.display()))?;
    check_name(name)?;
    let target = parent.join(name);
    if target.symlink_metadata().is_ok() {
        match (std::fs::canonicalize(&target), std::fs::canonicalize(dir)) {
            (Ok(a), Ok(b)) if a == b => {}
            _ => return Err(format!("{name} already exists")),
        }
    }
    Ok(target)
}

/// Rename the folder `dir` to `name`, refusing while a scan runs. The sidecar
/// writer is drained first and the `Scans` lock held across the rename and the
/// index write, as in `trash_files`. The watcher and the folder tree's
/// watches on the folder and under it are released before the rename
/// (Windows refuses to rename a folder with a watched descendant); the
/// frontend's reopen and tree re-render under the new path set them again.
/// The recorded trash runs follow the rename too (`Runs::rename_dir`), so an
/// undo or redo of a run over the folder targets its new path. A
/// failed rename changes nothing, and puts the released watches back since
/// there is no reopen to set them again; a failed index write after a successful rename still
/// returns `Ok` (the disk is the source of truth, so the caller must rebase
/// and reopen under the new path) with a warning, and only costs a
/// re-extraction on the next open.
#[tauri::command]
pub async fn rename_folder(
    app: tauri::AppHandle,
    dir: String,
    name: String,
) -> Result<Renamed, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || rename_folder_blocking(&app, &dir, &name))
        .await
        .map_err(|e| e.to_string())?
}

/// The body of `rename_folder`, run on a blocking thread after its first
/// scan check, shared with the one-date branch of `date_folder_run`.
pub(crate) fn rename_folder_blocking(
    app: &tauri::AppHandle,
    dir: &str,
    name: &str,
) -> Result<Renamed, String> {
    let target = folder_target(Path::new(dir), name)?;
    if let Some(writer) = &app.state::<AppWriter>().0 {
        writer.flush(crate::sidecar::DRAIN_TIMEOUT);
    }
    let scans = app.state::<Scans>();
    let state = index::lock(&scans.0);
    if state.scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    state.stop_embedding();
    let old = std::fs::canonicalize(dir)
        .map_err(|e| format!("{dir}: {e}"))?
        .to_string_lossy()
        .into_owned();
    let released = crate::watch::release_under(app, &old);
    if let Err(e) = crate::treewatch::with_released(app, &old, || {
        std::fs::rename(dir, &target).map_err(|e| format!("{name}: {e}"))
    }) {
        if let Some((dir, owner)) = released {
            crate::watch::set(app, &dir, &owner);
        }
        return Err(e);
    }
    let new = std::fs::canonicalize(&target).map_or_else(
        |_| target.to_string_lossy().into_owned(),
        |p| p.to_string_lossy().into_owned(),
    );
    let mut warning = None;
    if let Some(index) = app.state::<AppIndex>().0.clone() {
        match index::lock(&index).rename_dir(&old, &new) {
            Ok(moved) => log::info!("renamed folder: {old} -> {new} files={moved}"),
            Err(e) => {
                log::warn!("renamed folder {old} -> {new} but not its index rows: {e}");
                warning = Some(format!(
                    "the folder was renamed but its cache was not ({e});                      it is rebuilt on the next open"
                ));
            }
        }
    }
    app.state::<crate::trash::Runs>().rename_dir(&old, &new);
    drop(state);
    Ok(Renamed {
        path: target.to_string_lossy().into_owned(),
        warning,
    })
}

/// The renames that move one RAW and its sidecars, RAW first, as
/// `(from, to)` pairs.
#[derive(Debug, PartialEq, Eq)]
pub struct FilePlan {
    pub moves: Vec<(PathBuf, PathBuf)>,
}

/// Plan renaming the RAW `path`, directly in `dir`, to `name`, carrying the
/// existing sidecars of both formats along (a format switch can leave both on
/// disk). A sidecar's target is minted from the new RAW path, since a `.dop`
/// name embeds the RAW's whole name. Refuses a `path` that is not a RAW in
/// `dir`, a `name` that is not a valid RAW file name, and any target that
/// exists as another entry than its source (a target that canonicalizes to
/// its source is a case-only rename).
pub fn file_plan(dir: &Path, path: &str, name: &str) -> Result<FilePlan, String> {
    let raw = PathBuf::from(path);
    if !riffle_core::scan::is_raw_file(&raw) {
        return Err(format!("not a RAW file: {path}"));
    }
    if raw.parent() != Some(dir) {
        return Err(format!("not in the open folder: {path}"));
    }
    check_name(name)?;
    let target = dir.join(name);
    if !riffle_core::scan::is_raw_file(&target) {
        return Err(format!("{name}: not a RAW file name"));
    }
    let mut moves = vec![(raw.clone(), target.clone())];
    for format in [SidecarFormat::Xmp, SidecarFormat::Dop] {
        if let Some(sidecar) = existing_sidecar(&raw, format) {
            moves.push((sidecar, format.sidecar_path(&target)));
        }
    }
    for (from, to) in &moves {
        if to.symlink_metadata().is_ok() {
            match (std::fs::canonicalize(to), std::fs::canonicalize(from)) {
                (Ok(a), Ok(b)) if a == b => {}
                _ => {
                    let taken = to.file_name().unwrap_or_default().to_string_lossy();
                    return Err(format!("{taken} already exists"));
                }
            }
        }
    }
    Ok(FilePlan { moves })
}

/// Carry out `plan` with `rename`, RAW first. When a sidecar fails, what
/// already moved is renamed back (best effort, logged) and the error
/// returned, so a failure leaves the folder as it was.
pub fn file_run(
    plan: FilePlan,
    mut rename: impl FnMut(&Path, &Path) -> Result<(), String>,
) -> Result<(), String> {
    for (i, (from, to)) in plan.moves.iter().enumerate() {
        if let Err(e) = rename(from, to) {
            for (from, to) in plan.moves[..i].iter().rev() {
                if let Err(back) = rename(to, from) {
                    log::warn!(
                        "could not rename {} back to {}: {back}",
                        to.display(),
                        from.display()
                    );
                }
            }
            return Err(e);
        }
    }
    Ok(())
}

/// Rename the RAW `path` in the folder `dir` to `name`, with its sidecars,
/// refusing while a scan runs. The guard order is `rename_folder`'s, without
/// the watcher release: the watcher holds the directory, not its files. The
/// plan is made after the writer is drained, so a sidecar a pending judgment
/// has just written moves too. A failed index write after a successful
/// rename still returns `Ok` with a warning, as in `rename_folder`.
/// `Renamed.path` is the canonical folder joined with `name`, the spelling
/// `list_arw` lists.
#[tauri::command]
pub async fn rename_file(
    app: tauri::AppHandle,
    dir: String,
    path: String,
    name: String,
) -> Result<Renamed, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(writer) = &app.state::<AppWriter>().0 {
            writer.flush(crate::sidecar::DRAIN_TIMEOUT);
        }
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
        state.stop_embedding();
        let dir = std::fs::canonicalize(&dir).map_err(|e| format!("{dir}: {e}"))?;
        let plan = file_plan(&dir, &path, &name)?;
        file_run(plan, |from, to| {
            std::fs::rename(from, to).map_err(|e| {
                let from = from.file_name().unwrap_or_default().to_string_lossy();
                format!("{from}: {e}")
            })
        })?;
        let new = dir.join(&name).to_string_lossy().into_owned();
        let mut warning = None;
        if let Some(index) = app.state::<AppIndex>().0.clone() {
            match index::lock(&index).rename_file(&path, &new) {
                Ok(()) => log::info!("renamed file: {path} -> {new}"),
                Err(e) => {
                    log::warn!("renamed file {path} -> {new} but not its index rows: {e}");
                    warning = Some(format!(
                        "the file was renamed but its cache was not ({e}); \
                         it is rebuilt on the next open"
                    ));
                }
            }
        }
        drop(state);
        Ok(Renamed { path: new, warning })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{check_name, file_plan, file_run, folder_target};
    use std::path::{Path, PathBuf};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-rename-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn check_name_accepts_an_ordinary_name() {
        assert!(check_name("2026-09-28 Tokyo").is_ok());
        assert!(check_name("a.b").is_ok());
        assert!(check_name(".hidden").is_ok());
    }

    #[test]
    fn check_name_refuses_what_a_file_system_would() {
        for name in [
            "", ".", "..", "a/b", "a\\b", "a<b", "a>b", "a:b", "a\"b", "a|b", "a?b", "a*b", "a\tb",
            "a\u{0}b", "a.", "a ",
        ] {
            assert!(check_name(name).is_err(), "{name:?} was accepted");
        }
    }

    #[test]
    fn folder_target_joins_the_name_to_the_parent() {
        let root = temp_dir("target");
        let dir = root.join("photos");
        std::fs::create_dir(&dir).unwrap();
        assert_eq!(folder_target(&dir, "shoot").unwrap(), root.join("shoot"));
        assert!(folder_target(&dir, "a/b").is_err());
    }

    #[test]
    fn folder_target_refuses_an_existing_entry() {
        let root = temp_dir("collision");
        let dir = root.join("photos");
        std::fs::create_dir(&dir).unwrap();
        std::fs::create_dir(root.join("taken")).unwrap();
        std::fs::write(root.join("file"), b"x").unwrap();
        assert!(folder_target(&dir, "taken").is_err());
        assert!(folder_target(&dir, "file").is_err());
    }

    #[test]
    fn folder_target_allows_a_case_only_rename() {
        let root = temp_dir("case");
        let dir = root.join("photos");
        std::fs::create_dir(&dir).unwrap();
        let target = folder_target(&dir, "Photos").unwrap();
        std::fs::rename(&dir, &target).unwrap();
        // On a case-insensitive file system the result may keep either
        // spelling as far as `exists()` is concerned; compare canonically.
        assert_eq!(
            std::fs::canonicalize(&target).unwrap(),
            std::fs::canonicalize(root.join("Photos")).unwrap()
        );
    }

    #[test]
    fn folder_target_refuses_a_root() {
        let root = if cfg!(windows) {
            Path::new("C:\\")
        } else {
            Path::new("/")
        };
        assert!(folder_target(root, "x").is_err());
    }

    fn write(path: &Path) {
        std::fs::write(path, b"x").unwrap();
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

    #[test]
    fn a_raw_moves_with_the_sidecars_of_both_formats() {
        let dir = temp_dir("file-all");
        for name in ["a.ARW", "a.xmp", "a.ARW.dop", "b.ARW"] {
            write(&dir.join(name));
        }
        let path = dir.join("a.ARW").to_string_lossy().into_owned();
        let plan = file_plan(&dir, &path, "shoot.ARW").unwrap();
        assert_eq!(
            plan.moves,
            vec![
                (dir.join("a.ARW"), dir.join("shoot.ARW")),
                (dir.join("a.xmp"), dir.join("shoot.xmp")),
                (dir.join("a.ARW.dop"), dir.join("shoot.ARW.dop")),
            ]
        );
        file_run(plan, fs_rename).unwrap();
        assert_eq!(
            names(&dir),
            ["b.ARW", "shoot.ARW", "shoot.ARW.dop", "shoot.xmp"]
        );
    }

    #[test]
    fn a_case_only_file_rename_is_allowed() {
        let dir = temp_dir("file-case");
        write(&dir.join("a.ARW"));
        write(&dir.join("a.xmp"));
        let path = dir.join("a.ARW").to_string_lossy().into_owned();
        file_run(file_plan(&dir, &path, "A.ARW").unwrap(), fs_rename).unwrap();
        // The listing carries the names as stored, whatever the file
        // system's case sensitivity, unlike `exists()`.
        assert_eq!(names(&dir), ["A.ARW", "A.xmp"]);
    }

    #[test]
    fn a_bad_name_an_outside_path_or_a_collision_moves_nothing() {
        let dir = temp_dir("file-refuse");
        let outside_dir = temp_dir("file-refuse-outside");
        for name in ["a.ARW", "a.xmp", "b.ARW", "c.xmp", "d.jpg"] {
            write(&dir.join(name));
        }
        write(&outside_dir.join("e.ARW"));
        let path = dir.join("a.ARW").to_string_lossy().into_owned();
        let before = names(&dir);
        assert!(file_plan(&dir, &path, "a.jpg").is_err());
        assert!(file_plan(&dir, &path, "a").is_err());
        assert!(file_plan(&dir, &path, "x/y.ARW").is_err());
        assert!(file_plan(&dir, &path, "b.ARW").is_err());
        assert!(file_plan(&dir, &path, "c.ARW").is_err());
        let outside = outside_dir.join("e.ARW").to_string_lossy().into_owned();
        assert!(file_plan(&dir, &outside, "f.ARW").is_err());
        let jpeg = dir.join("d.jpg").to_string_lossy().into_owned();
        assert!(file_plan(&dir, &jpeg, "f.ARW").is_err());
        assert_eq!(names(&dir), before);
        assert!(outside_dir.join("e.ARW").exists());
    }

    #[test]
    fn a_failing_sidecar_rolls_the_raw_back() {
        let dir = temp_dir("file-rollback");
        for name in ["a.ARW", "a.xmp", "a.ARW.dop"] {
            write(&dir.join(name));
        }
        let path = dir.join("a.ARW").to_string_lossy().into_owned();
        let plan = file_plan(&dir, &path, "b.ARW").unwrap();
        let result = file_run(plan, |from, to| {
            if from.extension().is_some_and(|e| e == "dop") {
                return Err("refused".to_string());
            }
            fs_rename(from, to)
        });
        assert_eq!(result, Err("refused".to_string()));
        assert_eq!(names(&dir), ["a.ARW", "a.ARW.dop", "a.xmp"]);
    }
}
