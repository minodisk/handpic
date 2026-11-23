//! Renaming a folder from the folder tree and a RAW file from the strip. The
//! name check and the target planning are pure so they can be tested on temp
//! dirs; `rename_folder` releases the folder watcher and the tree watches,
//! renames on disk and carries the index rows and the recorded trash runs
//! along, so nothing is re-extracted and no judgment is lost. A RAW file
//! rename carries its sidecars along, but copies a `.xmp` it shares with a
//! RAW of the same stem that stays, so neither loses it. The scan check and
//! the sidecar writer's drain are the caller's.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use crate::index::{self, Index};
use crate::sidecar::{existing_sidecar, SidecarFormat};
use crate::trash::Runs;
use crate::treewatch::{self, TreeWatch};
use crate::watch::{self, Watch};

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

/// What a folder rename or a date split touches besides the disk: the open
/// folder's watcher, the folder tree's watches, the index when there is one
/// and the recorded trash runs.
pub struct Context<'a> {
    pub watch: &'a Watch,
    pub trees: &'a TreeWatch,
    pub index: Option<&'a Mutex<Index>>,
    pub runs: &'a Runs,
}

/// Rename the folder `dir` to `name`. Does not check for a scan: the caller
/// refuses a running scan and holds that refusal across the call (the
/// app's idle gate), and drains the sidecar writer first. The watcher and the
/// folder tree's watches on the folder and under it are released before the
/// rename (Windows refuses to
/// rename a folder with a watched descendant); the caller's reopen and tree
/// re-render under the new path set them again. The recorded trash runs
/// follow the rename too (`Runs::rename_dir`), so an undo or redo of a run
/// over the folder targets its new path. The `folder_counts` rows, keyed by
/// the caller's spelling, move from `dir` to the target as spelled from it.
/// A failed rename changes nothing,
/// and puts the released watcher back since there is no reopen to set it
/// again; a failed index write after a successful rename still returns `Ok`
/// (the disk is the source of truth, so the caller must rebase and reopen
/// under the new path) with a warning, and only costs a re-extraction on the
/// next open.
pub fn rename_folder(cx: &Context, dir: &str, name: &str) -> Result<Renamed, String> {
    let target = folder_target(Path::new(dir), name)?;
    let old = std::fs::canonicalize(dir)
        .map_err(|e| format!("{dir}: {e}"))?
        .to_string_lossy()
        .into_owned();
    let released = watch::release_under(cx.watch, &old);
    if let Err(e) = treewatch::with_released(cx.trees, &old, || {
        std::fs::rename(dir, &target).map_err(|e| format!("{name}: {e}"))
    }) {
        if let Some((dir, owner)) = released {
            watch::set(cx.watch, &dir, &owner);
        }
        return Err(e);
    }
    let new = std::fs::canonicalize(&target).map_or_else(
        |_| target.to_string_lossy().into_owned(),
        |p| p.to_string_lossy().into_owned(),
    );
    let mut warning = None;
    if let Some(index) = cx.index {
        let mut index = index::lock(index);
        match index.rename_dir(&old, &new).and_then(|moved| {
            index
                .rename_folder_counts(dir, &target.to_string_lossy())
                .map(|()| moved)
        }) {
            Ok(moved) => log::info!("renamed folder: {old} -> {new} files={moved}"),
            Err(e) => {
                log::warn!("renamed folder {old} -> {new} but not its index rows: {e}");
                warning = Some(format!(
                    "the folder was renamed but its cache was not ({e}); \
                     it is rebuilt on the next open"
                ));
            }
        }
    }
    cx.runs.rename_dir(&old, &new);
    Ok(Renamed {
        path: target.to_string_lossy().into_owned(),
        warning,
    })
}

/// One step of a `FilePlan`: a rename, or a copy that never overwrites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Move,
    Copy,
}

/// The steps that move one RAW and its sidecars, RAW first, as
/// `(op, from, to)`.
#[derive(Debug, PartialEq, Eq)]
pub struct FilePlan {
    pub steps: Vec<(Op, PathBuf, PathBuf)>,
}

/// The RAWs of `dir` that share one `.xmp`: two RAWs share when the paths
/// `existing_sidecar` gives them canonicalize to the same entry on disk (so
/// `a.xmp` and `A.xmp` are one file on a case-insensitive file system), which
/// is the file the sidecar writer patches for each. The key is one member's
/// spelling of the sidecar (the first RAW's, in sorted order), not a path
/// every member's `existing_sidecar` returns, so check membership by RAW, not
/// by looking the sidecar up. Only a sidecar of two or more RAWs is kept.
pub fn shared_xmps(dir: &Path) -> HashMap<PathBuf, Vec<PathBuf>> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    let mut by_name: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for raw in entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| handpic_core::scan::is_raw_file(path))
    {
        let name = SidecarFormat::Xmp
            .sidecar_path(&raw)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        by_name.entry(name).or_default().push(raw);
    }
    let mut shared: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for mut raws in by_name.into_values().filter(|raws| raws.len() > 1) {
        raws.sort();
        // Bucket by the entry on disk, since each RAW spells the sidecar from
        // its own stem and a case-insensitive file system maps those
        // spellings to one file; the key is the first RAW's spelling.
        let mut buckets: Vec<(PathBuf, PathBuf, Vec<PathBuf>)> = Vec::new();
        for raw in raws {
            let Some(xmp) = existing_sidecar(&raw, SidecarFormat::Xmp) else {
                continue;
            };
            let entry = std::fs::canonicalize(&xmp).unwrap_or_else(|_| xmp.clone());
            match buckets.iter_mut().find(|(e, _, _)| *e == entry) {
                Some((_, _, members)) => members.push(raw),
                None => buckets.push((entry, xmp, vec![raw])),
            }
        }
        for (_, xmp, members) in buckets {
            if members.len() > 1 {
                shared.insert(xmp, members);
            }
        }
    }
    shared
}

/// Whether `a` and `b` are one entry on disk.
fn same_entry(a: &Path, b: &Path) -> bool {
    matches!(
        (std::fs::canonicalize(a), std::fs::canonicalize(b)),
        (Ok(a), Ok(b)) if a == b
    )
}

/// Plan renaming the RAW `path`, directly in `dir`, to `name`, carrying the
/// existing sidecars of both formats along (a format switch can leave both on
/// disk). A sidecar's target is minted from the new RAW path, since a `.dop`
/// name embeds the RAW's whole name. A `.xmp` shared with a RAW that stays
/// (`shared_xmps`) is copied to the new name and the original left, and
/// nothing is planned for it when that copy would land on the shared sidecar
/// itself (a rename to another RAW extension, or a case-only rename).
/// Refuses a `path` that is not a RAW in `dir`, a `name` that is not a valid
/// RAW file name, and any target that exists as another entry than its
/// source (a target that canonicalizes to its source is a case-only rename).
pub fn file_plan(dir: &Path, path: &str, name: &str) -> Result<FilePlan, String> {
    let raw = PathBuf::from(path);
    if !handpic_core::scan::is_raw_file(&raw) {
        return Err(format!("not a RAW file: {path}"));
    }
    if raw.parent() != Some(dir) {
        return Err(format!("not in the open folder: {path}"));
    }
    check_name(name)?;
    let target = dir.join(name);
    if !handpic_core::scan::is_raw_file(&target) {
        return Err(format!("{name}: not a RAW file name"));
    }
    let mut steps = vec![(Op::Move, raw.clone(), target.clone())];
    for format in [SidecarFormat::Xmp, SidecarFormat::Dop] {
        if let Some(sidecar) = existing_sidecar(&raw, format) {
            let to = format.sidecar_path(&target);
            if format == SidecarFormat::Xmp
                && shared_xmps(dir).values().any(|raws| raws.contains(&raw))
            {
                if !same_entry(&to, &sidecar) {
                    steps.push((Op::Copy, sidecar, to));
                }
            } else {
                steps.push((Op::Move, sidecar, to));
            }
        }
    }
    for (_, from, to) in &steps {
        if to.symlink_metadata().is_ok() && !same_entry(to, from) {
            let taken = to.file_name().unwrap_or_default().to_string_lossy();
            return Err(format!("{taken} already exists"));
        }
    }
    Ok(FilePlan { steps })
}

/// Carry out one step on disk: `std::fs::rename`, or a copy into a new file
/// that fails when `to` exists (`std::fs::copy` would overwrite it), its
/// partial target removed on a failed write.
pub fn apply(op: Op, from: &Path, to: &Path) -> std::io::Result<()> {
    match op {
        Op::Move => std::fs::rename(from, to),
        Op::Copy => {
            let mut source = std::fs::File::open(from)?;
            let mut target = std::fs::File::create_new(to)?;
            if let Err(e) = std::io::copy(&mut source, &mut target) {
                drop(target);
                let _ = std::fs::remove_file(to);
                return Err(e);
            }
            Ok(())
        }
    }
}

/// Carry out `plan` with `apply`, RAW first. When a step fails, what already
/// happened is undone (best effort, logged): a move renamed back with
/// `apply`, a copy deleted with `remove`. The error is returned, so a
/// failure leaves the folder as it was.
pub fn file_run(
    plan: FilePlan,
    mut apply: impl FnMut(Op, &Path, &Path) -> Result<(), String>,
    mut remove: impl FnMut(&Path) -> Result<(), String>,
) -> Result<(), String> {
    for (i, (op, from, to)) in plan.steps.iter().enumerate() {
        if let Err(e) = apply(*op, from, to) {
            for (op, from, to) in plan.steps[..i].iter().rev() {
                let undone = match op {
                    Op::Move => apply(Op::Move, to, from),
                    Op::Copy => remove(to),
                };
                if let Err(back) = undone {
                    log::warn!(
                        "could not undo the {op:?} of {} to {}: {back}",
                        from.display(),
                        to.display()
                    );
                }
            }
            return Err(e);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        apply, check_name, file_plan, file_run, folder_target, rename_folder, Context, Op,
    };
    use crate::index::{self, Index};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("handpic-rename-{name}-{}", std::process::id()));
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

    fn fs_apply(op: Op, from: &Path, to: &Path) -> Result<(), String> {
        apply(op, from, to).map_err(|e| e.to_string())
    }

    fn fs_remove(path: &Path) -> Result<(), String> {
        std::fs::remove_file(path).map_err(|e| e.to_string())
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
            plan.steps,
            vec![
                (Op::Move, dir.join("a.ARW"), dir.join("shoot.ARW")),
                (Op::Move, dir.join("a.xmp"), dir.join("shoot.xmp")),
                (Op::Move, dir.join("a.ARW.dop"), dir.join("shoot.ARW.dop")),
            ]
        );
        file_run(plan, fs_apply, fs_remove).unwrap();
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
        file_run(
            file_plan(&dir, &path, "A.ARW").unwrap(),
            fs_apply,
            fs_remove,
        )
        .unwrap();
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
        let result = file_run(
            plan,
            |op, from, to| {
                if from.extension().is_some_and(|e| e == "dop") {
                    return Err("refused".to_string());
                }
                fs_apply(op, from, to)
            },
            fs_remove,
        );
        assert_eq!(result, Err("refused".to_string()));
        assert_eq!(names(&dir), ["a.ARW", "a.ARW.dop", "a.xmp"]);
    }

    #[test]
    fn a_shared_xmp_is_copied_and_an_unshared_one_moved() {
        let dir = temp_dir("file-shared");
        for name in ["a.ARW", "a.NEF", "a.xmp", "a.ARW.dop", "b.ARW", "b.xmp"] {
            std::fs::write(dir.join(name), name).unwrap();
        }
        let path = dir.join("a.ARW").to_string_lossy().into_owned();
        let plan = file_plan(&dir, &path, "c.ARW").unwrap();
        assert_eq!(
            plan.steps,
            vec![
                (Op::Move, dir.join("a.ARW"), dir.join("c.ARW")),
                (Op::Copy, dir.join("a.xmp"), dir.join("c.xmp")),
                (Op::Move, dir.join("a.ARW.dop"), dir.join("c.ARW.dop")),
            ]
        );
        file_run(plan, fs_apply, fs_remove).unwrap();
        assert_eq!(
            names(&dir),
            [
                "a.NEF",
                "a.xmp",
                "b.ARW",
                "b.xmp",
                "c.ARW",
                "c.ARW.dop",
                "c.xmp"
            ]
        );
        assert_eq!(std::fs::read(dir.join("c.xmp")).unwrap(), b"a.xmp");

        let path = dir.join("b.ARW").to_string_lossy().into_owned();
        let plan = file_plan(&dir, &path, "d.ARW").unwrap();
        assert_eq!(
            plan.steps,
            vec![
                (Op::Move, dir.join("b.ARW"), dir.join("d.ARW")),
                (Op::Move, dir.join("b.xmp"), dir.join("d.xmp")),
            ]
        );
    }

    #[test]
    fn a_shared_xmp_renamed_onto_itself_plans_nothing() {
        let dir = temp_dir("file-shared-self");
        for name in ["a.ARW", "a.NEF", "a.xmp"] {
            write(&dir.join(name));
        }
        let path = dir.join("a.ARW").to_string_lossy().into_owned();
        let plan = file_plan(&dir, &path, "a.DNG").unwrap();
        assert_eq!(
            plan.steps,
            vec![(Op::Move, dir.join("a.ARW"), dir.join("a.DNG"))]
        );
    }

    #[test]
    fn a_failing_step_after_a_copy_deletes_the_copy() {
        let dir = temp_dir("file-shared-rollback");
        for name in ["a.ARW", "a.NEF", "a.xmp", "a.ARW.dop"] {
            write(&dir.join(name));
        }
        let path = dir.join("a.ARW").to_string_lossy().into_owned();
        let plan = file_plan(&dir, &path, "b.ARW").unwrap();
        let mut removed = Vec::new();
        let result = file_run(
            plan,
            |op, from, to| {
                if from.extension().is_some_and(|e| e == "dop") {
                    return Err("refused".to_string());
                }
                fs_apply(op, from, to)
            },
            |path| {
                removed.push(path.to_path_buf());
                fs_remove(path)
            },
        );
        assert_eq!(result, Err("refused".to_string()));
        assert_eq!(removed, [dir.join("b.xmp")]);
        assert_eq!(names(&dir), ["a.ARW", "a.ARW.dop", "a.NEF", "a.xmp"]);
    }

    #[test]
    fn a_copy_onto_an_existing_file_is_refused() {
        let dir = temp_dir("file-shared-collision");
        for name in ["a.ARW", "a.NEF", "a.xmp", "b.xmp"] {
            write(&dir.join(name));
        }
        let path = dir.join("a.ARW").to_string_lossy().into_owned();
        let before = names(&dir);
        assert_eq!(
            file_plan(&dir, &path, "b.ARW"),
            Err("b.xmp already exists".to_string())
        );
        assert!(apply(Op::Copy, &dir.join("a.xmp"), &dir.join("b.xmp")).is_err());
        assert_eq!(names(&dir), before);
    }

    #[test]
    fn a_folder_rename_carries_its_count_rows() {
        let root = temp_dir("folder-counts");
        let photos = root.join("photos");
        let sub = photos.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let s = |p: &Path| p.to_string_lossy().into_owned();
        let sibling = root.join("photos2");
        let index = Mutex::new(Index::open(&root.join("index.sqlite")).unwrap());
        index::lock(&index)
            .set_folder_counts(&[(s(&photos), 3), (s(&sub), 2), (s(&sibling), 1)])
            .unwrap();
        let watch = crate::watch::Watch::spawn(|_| {});
        let trees = crate::treewatch::TreeWatch::spawn(|_| {});
        let runs = crate::trash::Runs::default();
        let cx = Context {
            watch: &watch,
            trees: &trees,
            index: Some(&index),
            runs: &runs,
        };

        let renamed = rename_folder(&cx, &s(&photos), "shoot").unwrap();

        let shoot = root.join("shoot");
        assert_eq!(renamed.path, s(&shoot));
        assert_eq!(renamed.warning, None);
        let mut expected = vec![(s(&shoot), 3), (s(&shoot.join("sub")), 2), (s(&sibling), 1)];
        expected.sort();
        assert_eq!(index::lock(&index).folder_counts().unwrap(), expected);
    }
}
