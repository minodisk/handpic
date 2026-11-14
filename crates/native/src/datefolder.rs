//! The folder tree's `Rename by Capture Date…`, kept pure: the dialog's text
//! and the flow from the menu item through the dialog to the run's end. A
//! port of the Tauri app's `datefolder.ts`; the window runs the backend's
//! `datefolder` and `rename`.

use riffle_backend::datefolder::{Preview, Split};

pub const RUNNING_NOTE: &str = "Moving the files; this closes once every file is moved.";

fn files(count: usize) -> String {
    format!("{count} {}", if count == 1 { "file" } else { "files" })
}

pub fn date_title_line(preview: &Preview) -> String {
    let name = &preview.name;
    match preview.kind {
        "rename" => format!(
            "Rename {name} to {}?",
            preview.targets.first().map_or("", |t| t.name.as_str())
        ),
        "split" => format!(
            "Split {name} into {} folders by capture date?",
            preview.targets.len()
        ),
        _ => format!("{name} is already named its capture date."),
    }
}

/// One row per target folder with its file count.
pub fn date_rows(preview: &Preview) -> Vec<String> {
    preview
        .targets
        .iter()
        .map(|t| format!("{}  {}", t.name, files(t.count)))
        .collect()
}

/// The undated files: they come along with a rename and stay in a split.
/// `None` when there are none.
pub fn undated_line(preview: &Preview) -> Option<String> {
    let undated = &preview.undated;
    if undated.is_empty() {
        return None;
    }
    let one = undated.len() == 1;
    let place = if preview.kind == "split" {
        format!("{} in {}", if one { "stays" } else { "stay" }, preview.name)
    } else {
        (if one { "comes along" } else { "come along" }).to_owned()
    };
    Some(format!(
        "Without a capture date, {} {place}: {}",
        files(undated.len()),
        undated.join(", ")
    ))
}

pub fn removal_note(preview: &Preview) -> Option<String> {
    (preview.kind == "split").then(|| format!("{} is removed if it ends up empty.", preview.name))
}

/// Why the run is disabled, or `None` when it can run.
pub fn disabled_reason(preview: &Preview) -> Option<String> {
    let collisions = &preview.collisions;
    if !collisions.is_empty() {
        let verb = if collisions.len() == 1 {
            "exists"
        } else {
            "exist"
        };
        return Some(format!(
            "{} already {verb} next to {}; nothing is merged.",
            collisions.join(", "),
            preview.name
        ));
    }
    (preview.kind == "named").then(|| "Nothing to rename.".to_owned())
}

/// The status line after a split: the folders the files went to, and what
/// failed or was left behind.
pub fn split_status(ran: &Split, name: &str) -> String {
    let mut parts = vec![if ran.moved.is_empty() {
        format!("Moved nothing out of {name}")
    } else {
        let into: Vec<String> = ran
            .moved
            .iter()
            .map(|m| format!("{} ({})", m.name, m.count))
            .collect();
        format!("Split {name} into {}", into.join(", "))
    }];
    if !ran.failed.is_empty() {
        parts.push(format!("{} failed", files(ran.failed.len())));
    }
    if !ran.removed {
        parts.push(format!("{name} kept ({} left)", ran.left.len()));
    }
    if let Some(warning) = &ran.warning {
        parts.push(warning.clone());
    }
    parts.join("; ")
}

/// The folder `path` is in, as the tree keys it (a root keeps its
/// separator).
pub fn parent_of(path: &str) -> String {
    let Some(cut) = path.rfind(['/', '\\']) else {
        return path.to_owned();
    };
    if cut == 0 {
        return path[..1].to_owned();
    }
    let parent = &path[..cut];
    let b = parent.as_bytes();
    if b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return path[..=cut].to_owned();
    }
    parent.to_owned()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Previewing,
    Previewed,
    Running,
}

/// The flow of one date rename or split, from the menu item through the
/// dialog to the run's end. A second start while one is under way does
/// nothing. The dialog is open while previewed and while running; a running
/// move cannot be canceled.
pub struct DateFolderFlow {
    phase: Phase,
    dir: Option<String>,
}

impl Default for DateFolderFlow {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            dir: None,
        }
    }
}

impl DateFolderFlow {
    pub fn is_open(&self) -> bool {
        matches!(self.phase, Phase::Previewed | Phase::Running)
    }

    pub fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }

    pub fn start(&mut self, dir: String) -> bool {
        if self.busy() {
            return false;
        }
        self.phase = Phase::Previewing;
        self.dir = Some(dir);
        true
    }

    pub fn previewed(&mut self) -> bool {
        if self.phase != Phase::Previewing {
            return false;
        }
        self.phase = Phase::Previewed;
        true
    }

    /// The folder to run on, or `None` when there is no preview to run.
    pub fn run(&mut self) -> Option<String> {
        if self.phase != Phase::Previewed {
            return None;
        }
        self.phase = Phase::Running;
        self.dir.clone()
    }

    /// Whether the dialog should close: Cancel or Escape on a preview.
    pub fn dismiss(&mut self) -> bool {
        if self.phase != Phase::Previewed {
            return false;
        }
        self.end();
        true
    }

    pub fn end(&mut self) {
        self.phase = Phase::Idle;
        self.dir = None;
    }
}

/// Ported from `datefolder.test.ts`.
#[cfg(test)]
mod tests {
    use super::*;
    use riffle_backend::datefolder::{Moved, Target};
    use riffle_backend::trash::Failure;

    fn target(name: &str, count: usize) -> Target {
        Target {
            name: name.to_owned(),
            count,
        }
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn rename() -> Preview {
        Preview {
            kind: "rename",
            name: "100MSDCF".to_owned(),
            targets: vec![target("2026-10-31", 12)],
            undated: Vec::new(),
            collisions: Vec::new(),
        }
    }

    fn split() -> Preview {
        Preview {
            kind: "split",
            name: "100MSDCF".to_owned(),
            targets: vec![target("2026-10-31", 1), target("2026-11-01", 4)],
            undated: strings(&["DSC00001.ARW", "DSC00009.ARW"]),
            collisions: Vec::new(),
        }
    }

    #[test]
    fn the_title_names_the_one_date_of_a_rename() {
        assert_eq!(date_title_line(&rename()), "Rename 100MSDCF to 2026-10-31?");
    }

    #[test]
    fn the_title_counts_the_folders_of_a_split() {
        assert_eq!(
            date_title_line(&split()),
            "Split 100MSDCF into 2 folders by capture date?"
        );
    }

    #[test]
    fn the_title_says_a_folder_is_already_named_its_date() {
        let named = Preview {
            kind: "named",
            name: "2026-10-31".to_owned(),
            ..rename()
        };
        assert_eq!(
            date_title_line(&named),
            "2026-10-31 is already named its capture date."
        );
    }

    #[test]
    fn date_rows_give_one_row_per_target_with_its_file_count() {
        assert_eq!(date_rows(&rename()), ["2026-10-31  12 files"]);
        assert_eq!(
            date_rows(&split()),
            ["2026-10-31  1 file", "2026-11-01  4 files"]
        );
    }

    #[test]
    fn undated_line_lists_the_undated_files_that_stay_in_a_split() {
        assert_eq!(
            undated_line(&split()).as_deref(),
            Some("Without a capture date, 2 files stay in 100MSDCF: DSC00001.ARW, DSC00009.ARW")
        );
    }

    #[test]
    fn undated_line_says_the_undated_files_come_along_with_a_rename() {
        let preview = Preview {
            undated: strings(&["a.ARW"]),
            ..rename()
        };
        assert_eq!(
            undated_line(&preview).as_deref(),
            Some("Without a capture date, 1 file comes along: a.ARW")
        );
    }

    #[test]
    fn undated_line_is_none_when_every_file_is_dated() {
        assert_eq!(undated_line(&rename()), None);
    }

    #[test]
    fn removal_note_says_a_splits_original_folder_is_removed_once_empty() {
        assert_eq!(
            removal_note(&split()).as_deref(),
            Some("100MSDCF is removed if it ends up empty.")
        );
        assert_eq!(removal_note(&rename()), None);
    }

    #[test]
    fn disabled_reason_names_the_colliding_targets() {
        let one = Preview {
            collisions: strings(&["2026-10-31"]),
            ..rename()
        };
        assert_eq!(
            disabled_reason(&one).as_deref(),
            Some("2026-10-31 already exists next to 100MSDCF; nothing is merged.")
        );
        let two = Preview {
            collisions: strings(&["2026-10-31", "2026-11-01"]),
            ..split()
        };
        assert_eq!(
            disabled_reason(&two).as_deref(),
            Some("2026-10-31, 2026-11-01 already exist next to 100MSDCF; nothing is merged.")
        );
    }

    #[test]
    fn disabled_reason_disables_an_already_named_folder() {
        let named = Preview {
            kind: "named",
            ..rename()
        };
        assert_eq!(
            disabled_reason(&named).as_deref(),
            Some("Nothing to rename.")
        );
    }

    #[test]
    fn disabled_reason_is_none_when_the_run_can_go() {
        assert_eq!(disabled_reason(&rename()), None);
        assert_eq!(disabled_reason(&split()), None);
    }

    fn moved() -> Vec<Moved> {
        vec![
            Moved {
                name: "2026-10-31".to_owned(),
                path: "/p/2026-10-31".to_owned(),
                count: 1,
            },
            Moved {
                name: "2026-11-01".to_owned(),
                path: "/p/2026-11-01".to_owned(),
                count: 4,
            },
        ]
    }

    fn failure(path: &str) -> Failure {
        Failure {
            path: path.to_owned(),
            message: "denied".to_owned(),
        }
    }

    #[test]
    fn split_status_names_the_date_folders_of_a_clean_split() {
        let ran = Split {
            moved: moved(),
            failed: Vec::new(),
            left: Vec::new(),
            removed: true,
            warning: None,
        };
        assert_eq!(
            split_status(&ran, "100MSDCF"),
            "Split 100MSDCF into 2026-10-31 (1), 2026-11-01 (4)"
        );
    }

    #[test]
    fn split_status_adds_the_failures_the_folder_kept_and_the_warning() {
        let ran = Split {
            moved: moved(),
            failed: vec![failure("/p/100MSDCF/a.ARW")],
            left: strings(&["a.ARW", "b.ARW"]),
            removed: false,
            warning: Some("cache not moved".to_owned()),
        };
        assert_eq!(
            split_status(&ran, "100MSDCF"),
            "Split 100MSDCF into 2026-10-31 (1), 2026-11-01 (4); 1 file failed; 100MSDCF kept (2 left); cache not moved"
        );
    }

    #[test]
    fn split_status_says_when_nothing_moved() {
        let ran = Split {
            moved: Vec::new(),
            failed: vec![failure("/p/a/x.ARW")],
            left: strings(&["x.ARW"]),
            removed: false,
            warning: None,
        };
        assert_eq!(
            split_status(&ran, "a"),
            "Moved nothing out of a; 1 file failed; a kept (1 left)"
        );
    }

    #[test]
    fn parent_of_drops_the_last_component() {
        assert_eq!(parent_of("/home/me/100MSDCF"), "/home/me");
        assert_eq!(parent_of("D:\\photos\\100MSDCF"), "D:\\photos");
    }

    #[test]
    fn parent_of_keeps_a_roots_separator() {
        assert_eq!(parent_of("/100MSDCF"), "/");
        assert_eq!(parent_of("E:\\DCIM"), "E:\\");
    }

    #[test]
    fn the_flow_goes_from_start_through_the_preview_to_the_run() {
        let mut flow = DateFolderFlow::default();
        assert!(flow.start("/p/a".to_owned()));
        assert!(flow.busy());
        assert!(!flow.is_open());
        assert!(!flow.start("/p/b".to_owned()));
        assert_eq!(flow.run(), None);
        assert!(flow.previewed());
        assert!(flow.is_open());
        assert_eq!(flow.run().as_deref(), Some("/p/a"));
        assert!(!flow.dismiss());
        flow.end();
        assert!(!flow.busy());
    }

    #[test]
    fn the_flow_dismisses_a_preview_but_not_a_running_move() {
        let mut flow = DateFolderFlow::default();
        flow.start("/p/a".to_owned());
        flow.previewed();
        assert!(flow.dismiss());
        assert!(!flow.busy());
    }
}

/// The folder rename and the split on this crate's `notify` (8.2.2 in
/// `Cargo.lock`, 8.2.0 in the root lock): the backend's own tests run on the
/// root lock and cannot run from this workspace, so the release-and-settle
/// path the rename relies on is checked here, on a temp dir.
#[cfg(test)]
mod watch_release {
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;
    use std::time::Duration;

    use riffle_backend::datefolder::{plan, split_folder};
    use riffle_backend::rename::{Context, rename_folder};
    use riffle_backend::trash::Runs;
    use riffle_backend::treewatch::TreeWatch;
    use riffle_backend::watch::{self, Watch};

    fn s(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    fn canonical(path: &Path) -> String {
        s(&std::fs::canonicalize(path).unwrap())
    }

    #[test]
    fn released_watches_let_a_watched_folder_be_renamed_and_split_away() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        let a = root.join("a");
        std::fs::create_dir_all(a.join("x")).unwrap();
        let (sent, changed) = mpsc::channel();
        let watch = Watch::spawn(move |dir| {
            let _ = sent.send(dir);
        });
        let trees = TreeWatch::spawn(|_| {});
        let runs = Runs::default();
        let cx = Context {
            watch: &watch,
            trees: &trees,
            index: None,
            runs: &runs,
        };

        watch::set(&watch, &canonical(&a), &s(&a));
        trees.set(&[s(&root), s(&a), s(&a.join("x"))]);
        if cfg!(windows) {
            assert!(std::fs::rename(&a, root.join("pinned")).is_err());
        }
        let renamed = rename_folder(&cx, &s(&a), "b").unwrap();
        let b = root.join("b");
        assert_eq!(renamed.path, s(&b));
        assert!(b.join("x").is_dir() && !a.exists());

        watch::set(&watch, &canonical(&b), &s(&b));
        trees.set(&[s(&root), s(&b), s(&b.join("x"))]);
        std::fs::write(b.join("new.ARW"), b"x").unwrap();
        assert_eq!(
            changed.recv_timeout(Duration::from_secs(10)).unwrap(),
            s(&b)
        );

        let files: Vec<(PathBuf, Option<String>)> =
            [("1.jpg", "2026-10-31"), ("2.jpg", "2026-11-01")]
                .into_iter()
                .map(|(name, date)| {
                    std::fs::write(b.join(name), b"x").unwrap();
                    (Path::new(&canonical(&b)).join(name), Some(date.to_owned()))
                })
                .collect();
        std::fs::remove_file(b.join("new.ARW")).unwrap();
        std::fs::remove_dir(b.join("x")).unwrap();
        trees.set(&[s(&root), s(&b)]);
        let plan = plan(Path::new(&canonical(&b)), &files).unwrap();
        let split = split_folder(&cx, &canonical(&b), &s(&b), &plan).unwrap();
        assert!(split.removed && split.failed.is_empty());
        assert!(!b.exists());
        assert!(root.join("2026-10-31").join("1.jpg").is_file());
        assert!(root.join("2026-11-01").join("2.jpg").is_file());
    }
}
