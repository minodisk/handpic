//! The folder tree's `Import…`, kept pure: the dialog's text and the flow
//! from the menu item through the preview to the run's end. A port of the
//! Tauri app's `import.ts`; the window runs the backend's `import`.

use riffle_backend::import::{Preview, Ran};

pub const RUNNING_NOTE: &str = "Cancel stops at once and drops the file in flight; the files copied so far stay, and the next import skips them.";

pub const COLLISIONS_HEADING: &str = "Already at the destination with another size, not copied:";

pub const UNDATED_HEADING: &str = "Without a capture date, not copied:";

pub const NOTHING_TO_COPY: &str = "Nothing left to copy: every dated file is already there.";
pub const NO_PHOTOS: &str = "No photos were found under the folder.";
pub const NOTHING_NEW: &str =
    "Nothing to copy: the dated files are not copied (see the lists above) or there are none.";

fn files(count: usize) -> String {
    format!("{count} {}", if count == 1 { "file" } else { "files" })
}

fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Decimal units, as the `import:` log line's MB/s.
pub fn size_text(bytes: u64) -> String {
    let bytes = bytes as f64;
    if bytes >= 1e9 {
        format!("{:.2} GB", bytes / 1e9)
    } else {
        format!("{:.1} MB", bytes / 1e6)
    }
}

pub fn total_line(preview: &Preview) -> String {
    let totals = &preview.totals;
    let into = preview
        .folders
        .iter()
        .filter(|folder| folder.copy_count > 0)
        .count();
    let head = format!(
        "{} ({}) to copy into {into} {}",
        files(totals.copy_count),
        size_text(totals.copy_bytes),
        if into == 1 { "folder" } else { "folders" }
    );
    if totals.skipped == 0 {
        head
    } else {
        format!("{head}, {} already there", totals.skipped)
    }
}

/// One row per destination folder.
pub fn folder_rows(preview: &Preview) -> Vec<String> {
    preview
        .folders
        .iter()
        .map(|folder| {
            let mut parts = vec![files(folder.copy_count), size_text(folder.copy_bytes)];
            if folder.skipped > 0 {
                parts.push(format!("{} skipped", folder.skipped));
            }
            format!("{}  {}", folder.path, parts.join(", "))
        })
        .collect()
}

/// Each colliding file with the folder it would have gone to.
pub fn collision_items(preview: &Preview) -> Vec<String> {
    preview
        .folders
        .iter()
        .flat_map(|folder| {
            folder
                .collisions
                .iter()
                .map(|name| format!("{name}  in {}", base_name(&folder.path)))
        })
        .collect()
}

/// The note under the lists: why the import is refused, or that nothing is
/// left to copy; `None` when there is something to run.
pub fn import_note(preview: &Preview) -> Option<String> {
    if let Some(refusal) = &preview.refusal {
        return Some(refusal.clone());
    }
    let totals = &preview.totals;
    if totals.copy_count > 0 {
        return None;
    }
    if preview.folders.is_empty() && preview.undated.is_empty() {
        return Some(NO_PHOTOS.to_owned());
    }
    let note = if totals.skipped > 0 && totals.collisions == 0 {
        NOTHING_TO_COPY
    } else {
        NOTHING_NEW
    };
    Some(note.to_owned())
}

/// A collision or an undated file is informational: the run is enabled
/// whenever a file is to be copied.
pub fn can_run(preview: &Preview) -> bool {
    preview.id.is_some() && preview.totals.copy_count > 0
}

pub fn progress_status(done: usize, total: usize) -> String {
    format!("importing {done} / {total}")
}

/// The pane's notes word a run's failure this way through `write_failed`,
/// which takes the path and the message apart, so only the tests use it.
#[cfg(test)]
pub fn failure_text(failure: &riffle_backend::import::Failure) -> String {
    format!("{}: {}", base_name(&failure.path), failure.message)
}

pub fn done_status(ran: &Ran) -> String {
    let head = if ran.canceled {
        format!("Import canceled, {} copied", files(ran.copied))
    } else {
        format!("Imported {} ({})", files(ran.copied), size_text(ran.bytes))
    };
    let mut parts = vec![head];
    if ran.skipped > 0 {
        parts.push(format!("{} already there", ran.skipped));
    }
    if !ran.failed.is_empty() {
        parts.push(format!("{} failed", ran.failed.len()));
    }
    parts.join(", ")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Previewing,
    Previewed,
    Running,
    Done,
}

/// What the dialog's Cancel or Escape does.
#[derive(Debug, PartialEq, Eq)]
pub enum Dismiss {
    /// Close the preview.
    Close,
    /// Hide the dialog of the run under way, which goes on.
    Aside,
    /// Cancel the run under way; its end closes the dialog.
    Cancel,
    None,
}

/// The flow of one import, from the tree's menu item to the run's end. The
/// dialog is open while previewed and until the run has created its
/// destination folders (`started`); it then steps aside so the strip can be
/// culled while the copy runs, and the tree's `Import…` brings it back
/// (`reopen`) to cancel the run. The run's id is handed out by `run`, before
/// the run starts, so every message of the run carries a known id.
pub struct ImportFlow {
    phase: Phase,
    preview_id: Option<u64>,
    run_id: Option<u64>,
    next_id: u64,
    /// A cancel was asked for before `started`: the dialog stays up.
    cancel_asked: bool,
    aside: bool,
    reopened: bool,
}

impl Default for ImportFlow {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            preview_id: None,
            run_id: None,
            next_id: 0,
            cancel_asked: false,
            aside: false,
            reopened: false,
        }
    }
}

impl ImportFlow {
    pub fn is_open(&self) -> bool {
        self.phase == Phase::Previewed || (self.phase == Phase::Running && !self.aside)
    }

    /// From `start` through the run's end or a failure: the flow is under
    /// way even while its dialog is not shown (previewing, or aside).
    pub fn busy(&self) -> bool {
        !matches!(self.phase, Phase::Idle | Phase::Done)
    }

    /// False while an import is already under way; else the folder is to
    /// be previewed.
    pub fn start(&mut self) -> bool {
        if self.busy() {
            return false;
        }
        self.phase = Phase::Previewing;
        self.preview_id = None;
        self.run_id = None;
        self.cancel_asked = false;
        self.aside = false;
        self.reopened = false;
        true
    }

    /// While a run is under way with its dialog aside: show the dialog again.
    pub fn reopen(&mut self) -> bool {
        if self.phase != Phase::Running || !self.aside {
            return false;
        }
        self.aside = false;
        self.reopened = true;
        true
    }

    /// `preview_id` is the preview's id, `None` for a refused one.
    pub fn previewed(&mut self, preview_id: Option<u64>) -> bool {
        if self.phase != Phase::Previewing {
            return false;
        }
        self.preview_id = preview_id;
        self.phase = Phase::Previewed;
        true
    }

    /// The preview failed: the flow ends.
    pub fn fail(&mut self) {
        self.phase = Phase::Idle;
    }

    /// The preview id to run and the run's id, or `None` when there is no
    /// preview to run.
    pub fn run(&mut self) -> Option<(u64, u64)> {
        let preview_id = self.preview_id.filter(|_| self.phase == Phase::Previewed)?;
        self.phase = Phase::Running;
        self.next_id += 1;
        self.run_id = Some(self.next_id);
        Some((preview_id, self.next_id))
    }

    /// The run `run_id` created its destination folders: whether its dialog
    /// steps aside, which it does not when a cancel was asked for meanwhile.
    pub fn started(&mut self, run_id: u64) -> bool {
        if !self.accepts(run_id) {
            return false;
        }
        self.aside = !self.cancel_asked;
        self.aside
    }

    pub fn accepts(&self, run_id: u64) -> bool {
        self.phase == Phase::Running && self.run_id == Some(run_id)
    }

    /// True when the run `run_id` ends this flow; the dialog then closes.
    pub fn done(&mut self, run_id: u64) -> bool {
        if !self.accepts(run_id) {
            return false;
        }
        self.phase = Phase::Done;
        true
    }

    /// The dialog's Cancel or Escape (`key`): close a preview, or cancel the
    /// run, whose end closes the dialog. Escape on a dialog brought back by
    /// `reopen` only steps aside again: the Cancel button is the one thing
    /// that cancels then.
    pub fn dismiss(&mut self, key: bool) -> Dismiss {
        match self.phase {
            Phase::Previewed => {
                self.phase = Phase::Idle;
                Dismiss::Close
            }
            Phase::Running if key && self.reopened => {
                self.aside = true;
                self.reopened = false;
                Dismiss::Aside
            }
            Phase::Running => {
                self.cancel_asked = true;
                Dismiss::Cancel
            }
            _ => Dismiss::None,
        }
    }

    #[cfg(test)]
    fn phase(&self) -> Phase {
        self.phase
    }
}

/// Ported from `import.test.ts`. The `done` kept until `started` is left
/// out: the run's end is awaited after its start on the same task, so it
/// cannot come first. A cancel before `started` needs no passing on either:
/// the run's id is known from `run`, so the cancel sets its flag at once, and
/// `started` only keeps the dialog up. The flow keeps no `dir`: the view
/// keys a preview failure by the folder it started with.
#[cfg(test)]
mod tests {
    use super::*;
    use riffle_backend::import::{Failure, FolderSummary, Totals};

    fn folder(
        path: &str,
        copy_count: usize,
        copy_bytes: u64,
        skipped: usize,
        collisions: &[&str],
    ) -> FolderSummary {
        FolderSummary {
            path: path.to_owned(),
            copy_count,
            copy_bytes,
            skipped,
            collisions: collisions.iter().map(|c| (*c).to_owned()).collect(),
        }
    }

    fn totals(copy_count: usize, copy_bytes: u64, skipped: usize, collisions: usize) -> Totals {
        Totals {
            copy_count,
            copy_bytes,
            skipped,
            collisions,
            undated: 0,
        }
    }

    fn preview() -> Preview {
        Preview {
            id: Some(1),
            source: r"E:\".to_owned(),
            folders: vec![
                folder(r"D:\Photos\2026\2026-10-24", 2, 88_000_000, 0, &[]),
                folder(
                    r"D:\Photos\2026\2026-10-25",
                    1,
                    44_000_000,
                    3,
                    &["DSC00009.ARW"],
                ),
            ],
            undated: Vec::new(),
            totals: totals(3, 132_000_000, 3, 1),
            refusal: None,
        }
    }

    fn nothing() -> Preview {
        Preview {
            folders: vec![folder(r"D:\Photos\2026\2026-10-24", 0, 0, 2, &[])],
            totals: totals(0, 0, 2, 0),
            ..preview()
        }
    }

    fn refused() -> Preview {
        Preview {
            id: None,
            refusal: Some("not set".to_owned()),
            ..preview()
        }
    }

    fn ran() -> Ran {
        Ran {
            copied: 3,
            skipped: 0,
            failed: Vec::new(),
            canceled: false,
            bytes: 132_000_000,
            elapsed_ms: 5000,
            folders: vec![r"D:\Photos\2026\2026-10-24".to_owned()],
        }
    }

    #[test]
    fn sizes_in_decimal_units() {
        assert_eq!(size_text(0), "0.0 MB");
        assert_eq!(size_text(44_000_000), "44.0 MB");
        assert_eq!(size_text(2_500_000_000), "2.50 GB");
    }

    #[test]
    fn the_total_line_counts_the_folders_that_get_files() {
        assert_eq!(
            total_line(&preview()),
            "3 files (132.0 MB) to copy into 2 folders, 3 already there"
        );
        assert_eq!(
            total_line(&nothing()),
            "0 files (0.0 MB) to copy into 0 folders, 2 already there"
        );
    }

    #[test]
    fn a_row_per_destination_folder() {
        assert_eq!(
            folder_rows(&preview()),
            vec![
                r"D:\Photos\2026\2026-10-24  2 files, 88.0 MB".to_owned(),
                r"D:\Photos\2026\2026-10-25  1 file, 44.0 MB, 3 skipped".to_owned(),
            ]
        );
    }

    #[test]
    fn each_collision_names_its_folder() {
        assert_eq!(
            collision_items(&preview()),
            vec!["DSC00009.ARW  in 2026-10-25".to_owned()]
        );
    }

    #[test]
    fn the_note_a_refusal_nothing_to_copy_or_none() {
        assert_eq!(import_note(&preview()), None);
        assert_eq!(import_note(&nothing()).as_deref(), Some(NOTHING_TO_COPY));
        assert_eq!(import_note(&refused()).as_deref(), Some("not set"));
    }

    #[test]
    fn the_note_no_photos_under_the_folder() {
        let empty = Preview {
            folders: Vec::new(),
            totals: totals(0, 0, 0, 0),
            ..preview()
        };
        assert_eq!(import_note(&empty).as_deref(), Some(NO_PHOTOS));
    }

    #[test]
    fn the_note_collisions_or_undated_files_are_not_already_there() {
        let collided = Preview {
            folders: vec![folder(
                r"D:\Photos\2026\2026-10-24",
                0,
                0,
                0,
                &["DSC00001.ARW"],
            )],
            totals: totals(0, 0, 0, 1),
            ..preview()
        };
        assert_eq!(import_note(&collided).as_deref(), Some(NOTHING_NEW));
        let undated = Preview {
            folders: Vec::new(),
            undated: vec!["a.ARW".to_owned()],
            totals: Totals {
                undated: 1,
                ..totals(0, 0, 0, 0)
            },
            ..preview()
        };
        assert_eq!(import_note(&undated).as_deref(), Some(NOTHING_NEW));
    }

    #[test]
    fn runs_whenever_a_file_is_to_be_copied() {
        assert!(can_run(&preview()));
        assert!(can_run(&Preview {
            undated: vec!["a.ARW".to_owned()],
            ..preview()
        }));
        assert!(!can_run(&nothing()));
        assert!(!can_run(&refused()));
    }

    #[test]
    fn progress() {
        assert_eq!(progress_status(2, 5), "importing 2 / 5");
    }

    #[test]
    fn a_failure_names_the_file() {
        assert_eq!(
            failure_text(&Failure {
                path: r"E:\DCIM\DSC00001.ARW".to_owned(),
                message: "denied".to_owned(),
            }),
            "DSC00001.ARW: denied"
        );
    }

    #[test]
    fn the_done_status() {
        assert_eq!(done_status(&ran()), "Imported 3 files (132.0 MB)");
        assert_eq!(
            done_status(&Ran {
                copied: 1,
                skipped: 2,
                failed: vec![Failure {
                    path: "/a.ARW".to_owned(),
                    message: "x".to_owned(),
                }],
                bytes: 1_000_000,
                ..ran()
            }),
            "Imported 1 file (1.0 MB), 2 already there, 1 failed"
        );
        assert_eq!(
            done_status(&Ran {
                copied: 1,
                canceled: true,
                ..ran()
            }),
            "Import canceled, 1 file copied"
        );
    }

    fn running() -> (ImportFlow, u64) {
        let mut flow = ImportFlow::default();
        flow.start();
        flow.previewed(Some(7));
        let (_, run) = flow.run().unwrap();
        (flow, run)
    }

    #[test]
    fn walks_previewing_previewed_running_and_done() {
        let mut flow = ImportFlow::default();
        assert!(flow.start());
        assert_eq!(flow.phase(), Phase::Previewing);
        assert!(flow.busy());
        assert!(!flow.is_open());
        assert!(flow.previewed(Some(7)));
        assert!(flow.is_open());
        assert_eq!(flow.run(), Some((7, 1)));
        assert_eq!(flow.phase(), Phase::Running);
        assert!(flow.is_open());
        assert!(flow.started(1));
        assert!(!flow.is_open());
        assert!(flow.busy());
        assert!(flow.accepts(1));
        assert!(flow.done(1));
        assert_eq!(flow.phase(), Phase::Done);
        assert!(!flow.is_open());
        assert!(!flow.busy());
        assert!(flow.start());
    }

    #[test]
    fn does_not_start_again_while_under_way() {
        let mut flow = ImportFlow::default();
        flow.start();
        assert!(!flow.start());
        assert!(!running().0.start());
    }

    #[test]
    fn a_refused_preview_has_nothing_to_run() {
        let mut flow = ImportFlow::default();
        flow.start();
        flow.previewed(None);
        assert!(flow.is_open());
        assert_eq!(flow.run(), None);
        assert_eq!(flow.dismiss(false), Dismiss::Close);
        assert!(!flow.busy());
    }

    #[test]
    fn a_failed_preview_ends_the_flow() {
        let mut flow = ImportFlow::default();
        flow.start();
        flow.fail();
        assert_eq!(flow.phase(), Phase::Idle);
        assert!(!flow.previewed(Some(1)));
    }

    #[test]
    fn a_started_runs_dialog_steps_aside_and_comes_back_to_cancel() {
        let (mut flow, run) = running();
        flow.started(run);
        assert!(!flow.is_open());
        assert!(flow.reopen());
        assert!(flow.is_open());
        assert!(!flow.reopen());
        assert_eq!(flow.dismiss(false), Dismiss::Cancel);
        assert!(flow.is_open());
        assert!(flow.done(run));
        assert!(!flow.is_open());
        assert!(!flow.reopen());
    }

    #[test]
    fn only_a_run_under_way_with_its_dialog_aside_reopens() {
        let mut flow = ImportFlow::default();
        assert!(!flow.reopen());
        flow.start();
        assert!(!flow.reopen());
        flow.previewed(Some(7));
        assert!(!flow.reopen());
        let (_, run) = flow.run().unwrap();
        assert!(!flow.reopen());
        flow.started(run);
        assert!(flow.reopen());
    }

    #[test]
    fn ignores_the_messages_of_another_run() {
        let (mut flow, run) = running();
        flow.started(run);
        assert!(!flow.accepts(run + 1));
        assert!(!flow.done(run + 1));
        assert_eq!(flow.phase(), Phase::Running);
    }

    #[test]
    fn a_cancel_asked_for_before_started_keeps_the_dialog_up() {
        let (mut flow, run) = running();
        assert_eq!(flow.dismiss(false), Dismiss::Cancel);
        assert!(!flow.started(run));
        assert!(flow.is_open());
    }

    #[test]
    fn escape_on_a_reopened_dialog_steps_aside_again_cancel_still_cancels() {
        let (mut flow, run) = running();
        flow.started(run);
        assert!(!flow.is_open());
        assert!(flow.reopen());
        assert!(flow.is_open());
        assert_eq!(flow.dismiss(true), Dismiss::Aside);
        assert!(!flow.is_open());
        assert!(flow.reopen());
        assert_eq!(flow.dismiss(false), Dismiss::Cancel);
    }

    #[test]
    fn escape_before_started_cancels() {
        let (mut flow, _) = running();
        assert_eq!(flow.dismiss(true), Dismiss::Cancel);
    }

    #[test]
    fn each_run_gets_a_new_id() {
        let (mut flow, first) = running();
        flow.started(first);
        assert!(flow.done(first));
        flow.start();
        flow.previewed(Some(8));
        let (preview, second) = flow.run().unwrap();
        assert_eq!(preview, 8);
        assert_ne!(first, second);
        assert!(!flow.accepts(first));
    }
}
