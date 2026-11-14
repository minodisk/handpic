//! The folder tree's `Sequence JPEG Timestamps…`, kept pure: the dialog's
//! text and the flow from the menu item through the dialog to the run's end.
//! A port of the Tauri app's `sequence.ts`; the window runs the backend's
//! `sequence`.

use riffle_backend::sequence::{Failure, Preview, Ran, Row};

pub const RUNNING_NOTE: &str = "Cancel stops before the next file; the files written so far stay in the output folder, and the next run rebuilds it.";

fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// A row as the dialog shows it: `same + diff` is the new time.
#[derive(Debug, PartialEq, Eq)]
pub struct RowParts {
    pub head: String,
    pub same: String,
    pub diff: String,
}

/// `diff` starts at the first field (split by `:` or a space) that differs
/// from the old time, and is empty when none does.
pub fn row_parts(row: &Row) -> RowParts {
    let old: Vec<char> = row.old.chars().collect();
    let new: Vec<char> = row.new.chars().collect();
    let mut start = 0;
    while start < new.len() && old.get(start) == Some(&new[start]) {
        start += 1;
    }
    if start < new.len() {
        while start > 0 && new[start - 1] != ':' && new[start - 1] != ' ' {
            start -= 1;
        }
    }
    RowParts {
        head: format!("{}  {} -> ", base_name(&row.path), row.old),
        same: new[..start].iter().collect(),
        diff: new[start..].iter().collect(),
    }
}

/// `None` when the output folder does not exist yet.
pub fn rebuild_notice(preview: &Preview) -> Option<&'static str> {
    preview
        .output_exists
        .then_some("will be rebuilt: its JPEG files are replaced")
}

/// Every readable file is written, the unchanged ones too; a failed file is
/// not a row.
pub fn written_line(rows: &[Row], output_dir: &str) -> String {
    let files = if rows.len() == 1 { "file" } else { "files" };
    format!(
        "{} {files} will be written to {}",
        rows.len(),
        base_name(output_dir)
    )
}

/// Counts the rows whose `DateTimeOriginal` changes, the green ones, not
/// `changed`, which an exported JPEG's `DateTime` alone turns true.
pub fn capture_time_line(rows: &[Row]) -> String {
    let count = rows.iter().filter(|row| row.old != row.new).count();
    let gets = if count == 1 { "gets" } else { "get" };
    format!("{count} of them {gets} a new capture time")
}

pub fn failure_text(failure: &Failure) -> String {
    format!("{}: {}", base_name(&failure.path), failure.message)
}

pub fn progress_status(done: usize, total: usize) -> String {
    format!("sequencing {done} / {total}")
}

/// The failure of a run that ended on a folder-level error, or `None` for
/// any other run.
pub fn folder_error(ran: &Ran) -> Option<&Failure> {
    if ran.total == 0 && !ran.canceled {
        ran.failed.first()
    } else {
        None
    }
}

/// `None` for a folder-level error, which shows in the notes instead.
pub fn done_status(ran: &Ran) -> Option<String> {
    if folder_error(ran).is_some() {
        return None;
    }
    let head = if ran.canceled {
        format!("canceled, {} of {} written", ran.written, ran.total)
    } else {
        let files = if ran.total == 1 { "file" } else { "files" };
        format!(
            "Wrote {} of {} {files} to {}",
            ran.written, ran.total, ran.output_dir
        )
    };
    Some(if ran.failed.is_empty() {
        head
    } else {
        format!("{head}, {} failed", ran.failed.len())
    })
}

/// The folder to reveal in the file manager once a run ends, or `None` when
/// the run was canceled or wrote nothing.
pub fn reveal_after(ran: &Ran) -> Option<&str> {
    (!ran.canceled && ran.written > 0).then_some(ran.output_dir.as_str())
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
    /// Cancel the run under way; its end closes the dialog.
    Cancel,
    None,
}

/// The flow of one sequencing, from the tree's menu item to the run's end.
/// The dialog is open while previewed and while running. The run's id is
/// handed out by `run`, before the run starts, so every message of the run
/// carries a known id.
pub struct SequenceFlow {
    phase: Phase,
    dir: Option<String>,
    run_id: Option<u64>,
    next_id: u64,
}

impl Default for SequenceFlow {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            dir: None,
            run_id: None,
            next_id: 0,
        }
    }
}

impl SequenceFlow {
    pub fn is_open(&self) -> bool {
        matches!(self.phase, Phase::Previewed | Phase::Running)
    }

    /// From `start` through the run's end or a failure.
    pub fn busy(&self) -> bool {
        !matches!(self.phase, Phase::Idle | Phase::Done)
    }

    /// False while a sequencing is already under way; else `dir` is to be
    /// previewed.
    pub fn start(&mut self, dir: String) -> bool {
        if self.busy() {
            return false;
        }
        self.phase = Phase::Previewing;
        self.dir = Some(dir);
        self.run_id = None;
        true
    }

    pub fn previewed(&mut self) -> bool {
        if self.phase != Phase::Previewing {
            return false;
        }
        self.phase = Phase::Previewed;
        true
    }

    /// The preview failed: the flow ends.
    pub fn fail(&mut self) {
        self.phase = Phase::Idle;
    }

    /// The folder to run on and the run's id, or `None` when there is no
    /// preview to run.
    pub fn run(&mut self) -> Option<(String, u64)> {
        if self.phase != Phase::Previewed {
            return None;
        }
        self.phase = Phase::Running;
        self.next_id += 1;
        self.run_id = Some(self.next_id);
        Some((self.dir.clone()?, self.next_id))
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

    pub fn dismiss(&mut self) -> Dismiss {
        match self.phase {
            Phase::Previewed => {
                self.phase = Phase::Idle;
                Dismiss::Close
            }
            Phase::Running => Dismiss::Cancel,
            _ => Dismiss::None,
        }
    }

    #[cfg(test)]
    fn phase(&self) -> Phase {
        self.phase
    }
}

/// Ported from `sequence.test.ts`. Its `picking` phase, `started`, the
/// `done` kept until `started` and the cancel passed on by `started` are left
/// out: the Tauri run id comes back after the run's events can, while here
/// `run` hands the id out before the run starts, so none of those cases can
/// occur.
#[cfg(test)]
mod tests {
    use super::*;

    fn row(path: &str, changed: bool) -> Row {
        Row {
            path: path.to_owned(),
            old: "2024:01:02 03:04:58".to_owned(),
            new: if changed {
                "2024:01:02 03:04:59"
            } else {
                "2024:01:02 03:04:58"
            }
            .to_owned(),
            changed,
        }
    }

    fn failure(path: &str, message: &str) -> Failure {
        Failure {
            path: path.to_owned(),
            message: message.to_owned(),
        }
    }

    fn ran() -> Ran {
        Ran {
            output_dir: "/x/export-sequenced".to_owned(),
            written: 3,
            total: 3,
            failed: Vec::new(),
            canceled: false,
        }
    }

    fn preview(output_exists: bool) -> Preview {
        Preview {
            output_dir: "/x/export-sequenced".to_owned(),
            output_exists,
            rows: Vec::new(),
            failed: Vec::new(),
        }
    }

    #[test]
    fn a_row_names_the_file_and_splits_off_the_changed_field_of_the_new_time() {
        assert_eq!(
            row_parts(&row("/x/export/DSC00001.jpg", true)),
            RowParts {
                head: "DSC00001.jpg  2024:01:02 03:04:58 -> ".to_owned(),
                same: "2024:01:02 03:04:".to_owned(),
                diff: "59".to_owned(),
            }
        );
        assert_eq!(
            row_parts(&row(r"C:\x\L1000001.JPG", false)),
            RowParts {
                head: "L1000001.JPG  2024:01:02 03:04:58 -> ".to_owned(),
                same: "2024:01:02 03:04:58".to_owned(),
                diff: String::new(),
            }
        );
    }

    #[test]
    fn a_rollover_highlights_from_the_first_changed_field() {
        let at = |old: &str, new: &str| {
            let parts = row_parts(&Row {
                path: "/a.jpg".to_owned(),
                old: old.to_owned(),
                new: new.to_owned(),
                changed: true,
            });
            (parts.same, parts.diff)
        };
        assert_eq!(
            at("2024:01:02 03:04:59", "2024:01:02 03:05:00"),
            ("2024:01:02 03:".to_owned(), "05:00".to_owned())
        );
        assert_eq!(
            at("2024:01:02 03:59:59", "2024:01:02 04:00:00"),
            ("2024:01:02 ".to_owned(), "04:00:00".to_owned())
        );
    }

    #[test]
    fn the_rebuild_notice_shows_only_for_an_existing_output_folder() {
        assert_eq!(
            rebuild_notice(&preview(true)),
            Some("will be rebuilt: its JPEG files are replaced")
        );
        assert_eq!(rebuild_notice(&preview(false)), None);
    }

    #[test]
    fn the_written_line_counts_every_row_and_names_the_output_folder() {
        assert_eq!(
            written_line(
                &[row("/a.jpg", false), row("/b.jpg", true)],
                "/x/export-sequenced"
            ),
            "2 files will be written to export-sequenced"
        );
        assert_eq!(
            written_line(&[row(r"C:\x\a.jpg", false)], r"C:\x\export-sequenced"),
            "1 file will be written to export-sequenced"
        );
    }

    #[test]
    fn the_capture_time_line_counts_the_rows_whose_time_changes() {
        assert_eq!(
            capture_time_line(&[
                row("/a.jpg", false),
                row("/b.jpg", true),
                row("/c.jpg", true)
            ]),
            "2 of them get a new capture time"
        );
        assert_eq!(
            capture_time_line(&[row("/a.jpg", false), row("/b.jpg", true)]),
            "1 of them gets a new capture time"
        );
        assert_eq!(
            capture_time_line(&[row("/a.jpg", false)]),
            "0 of them get a new capture time"
        );
    }

    #[test]
    fn a_row_changed_only_by_its_date_time_does_not_get_a_new_capture_time() {
        let only_date_time = Row {
            changed: true,
            ..row("/a.jpg", false)
        };
        assert_eq!(
            capture_time_line(&[only_date_time]),
            "0 of them get a new capture time"
        );
    }

    #[test]
    fn a_failure_names_the_file() {
        assert_eq!(
            failure_text(&failure("/x/export/broken.jpg", "no EXIF")),
            "broken.jpg: no EXIF"
        );
    }

    #[test]
    fn progress() {
        assert_eq!(progress_status(2, 5), "sequencing 2 / 5");
    }

    #[test]
    fn the_done_status() {
        assert_eq!(
            done_status(&ran()).as_deref(),
            Some("Wrote 3 of 3 files to /x/export-sequenced")
        );
        assert_eq!(
            done_status(&Ran {
                written: 2,
                failed: vec![failure("/x/export/b.jpg", "denied")],
                ..ran()
            })
            .as_deref(),
            Some("Wrote 2 of 3 files to /x/export-sequenced, 1 failed")
        );
        assert_eq!(
            done_status(&Ran {
                written: 1,
                canceled: true,
                ..ran()
            })
            .as_deref(),
            Some("canceled, 1 of 3 written")
        );
        assert_eq!(
            done_status(&Ran {
                written: 0,
                total: 0,
                failed: vec![failure("/x/export", "no JPEG files")],
                ..ran()
            }),
            None
        );
    }

    #[test]
    fn the_folder_level_error() {
        let cases = [
            ("a run that wrote every file", ran(), false),
            (
                "a run with a per-file failure",
                Ran {
                    written: 2,
                    failed: vec![failure("/x/export/b.jpg", "denied")],
                    ..ran()
                },
                false,
            ),
            (
                "a run whose every file failed",
                Ran {
                    written: 0,
                    total: 1,
                    failed: vec![failure("/x/export/a.jpg", "denied")],
                    ..ran()
                },
                false,
            ),
            (
                "a canceled run",
                Ran {
                    written: 1,
                    canceled: true,
                    ..ran()
                },
                false,
            ),
            (
                "a canceled run before any file",
                Ran {
                    written: 0,
                    total: 0,
                    canceled: true,
                    ..ran()
                },
                false,
            ),
            (
                "a run that wrote nothing",
                Ran {
                    written: 0,
                    total: 3,
                    ..ran()
                },
                false,
            ),
            (
                "a folder-level error",
                Ran {
                    written: 0,
                    total: 0,
                    failed: vec![failure("/x/export", "no JPEG files")],
                    ..ran()
                },
                true,
            ),
        ];
        for (name, ran, expected) in cases {
            let error = folder_error(&ran).map(|f| (f.path.as_str(), f.message.as_str()));
            assert_eq!(
                error,
                expected.then_some(("/x/export", "no JPEG files")),
                "{name}"
            );
        }
    }

    #[test]
    fn reveals_the_output_folder() {
        let cases = [
            ("a run that wrote every file", ran(), true),
            (
                "a run with a per-file failure",
                Ran {
                    written: 2,
                    failed: vec![failure("/x/export/b.jpg", "denied")],
                    ..ran()
                },
                true,
            ),
            (
                "a canceled run",
                Ran {
                    written: 1,
                    canceled: true,
                    ..ran()
                },
                false,
            ),
            (
                "a run that wrote nothing",
                Ran {
                    written: 0,
                    ..ran()
                },
                false,
            ),
            (
                "a folder-level error",
                Ran {
                    written: 0,
                    total: 0,
                    failed: vec![failure("/x/export", "no JPEG files")],
                    ..ran()
                },
                false,
            ),
        ];
        for (name, ran, expected) in cases {
            assert_eq!(
                reveal_after(&ran),
                expected.then_some("/x/export-sequenced"),
                "{name}"
            );
        }
    }

    fn running() -> (SequenceFlow, u64) {
        let mut flow = SequenceFlow::default();
        flow.start("/x/export".to_owned());
        flow.previewed();
        let (_, id) = flow.run().unwrap();
        (flow, id)
    }

    #[test]
    fn walks_previewing_previewed_running_and_done() {
        let mut flow = SequenceFlow::default();
        assert!(flow.start("/x/export".to_owned()));
        assert_eq!(flow.phase(), Phase::Previewing);
        assert!(!flow.is_open());
        assert!(flow.previewed());
        assert_eq!(flow.phase(), Phase::Previewed);
        assert!(flow.is_open());
        assert_eq!(flow.run(), Some(("/x/export".to_owned(), 1)));
        assert_eq!(flow.phase(), Phase::Running);
        assert!(flow.accepts(1));
        assert!(flow.done(1));
        assert_eq!(flow.phase(), Phase::Done);
        assert!(!flow.is_open());
        assert!(flow.start("/x/other".to_owned()));
    }

    #[test]
    fn busy_is_true_from_previewing_unlike_is_open() {
        let mut flow = SequenceFlow::default();
        assert!(!flow.busy());
        flow.start("/x/export".to_owned());
        assert!(!flow.is_open());
        assert!(flow.busy());
        flow.previewed();
        assert!(flow.busy());
        let (_, id) = flow.run().unwrap();
        flow.done(id);
        assert_eq!(flow.phase(), Phase::Done);
        assert!(!flow.busy());
    }

    #[test]
    fn does_not_start_again_while_under_way() {
        let mut flow = SequenceFlow::default();
        flow.start("/x/export".to_owned());
        assert!(!flow.start("/x/other".to_owned()));
        assert!(!running().0.start("/x/other".to_owned()));
    }

    #[test]
    fn a_failed_preview_ends_the_flow() {
        let mut flow = SequenceFlow::default();
        flow.start("/x/export".to_owned());
        flow.fail();
        assert_eq!(flow.phase(), Phase::Idle);
        assert!(!flow.previewed());
    }

    #[test]
    fn dismissing_a_preview_closes_it_and_there_is_nothing_to_run() {
        let mut flow = SequenceFlow::default();
        flow.start("/x/export".to_owned());
        flow.previewed();
        assert_eq!(flow.dismiss(), Dismiss::Close);
        assert!(!flow.is_open());
        assert_eq!(flow.run(), None);
    }

    #[test]
    fn dismissing_a_run_cancels_it_and_keeps_the_dialog_open() {
        let (mut flow, _) = running();
        assert_eq!(flow.dismiss(), Dismiss::Cancel);
        assert!(flow.is_open());
    }

    #[test]
    fn ignores_the_messages_of_another_run() {
        let (mut flow, id) = running();
        assert!(!flow.accepts(id + 1));
        assert!(!flow.done(id + 1));
        assert_eq!(flow.phase(), Phase::Running);
    }

    #[test]
    fn each_run_gets_a_new_id_and_a_late_message_of_an_ended_run_is_dropped() {
        let (mut flow, first) = running();
        assert!(flow.done(first));
        assert!(!flow.accepts(first));
        flow.start("/x/export".to_owned());
        flow.previewed();
        let (_, second) = flow.run().unwrap();
        assert_ne!(first, second);
        assert!(!flow.accepts(first));
        assert!(flow.accepts(second));
    }
}
