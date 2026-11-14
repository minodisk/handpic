//! The folder tree's `Rewrite Sidecars from Index…` and `Delete Sidecars…`,
//! kept pure: the confirmation dialog's text and the flow from the menu item
//! through the dialog to the run's end. A port of the Tauri app's
//! `sidecars.ts`; the window runs the backend's `foldersidecars`.

use riffle_backend::foldersidecars::{DeletePreview, Preview, Summary};

pub const REWRITE_RUNNING_NOTE: &str =
    "Writing the sidecars; this closes once every file is written.";

pub const DELETE_RUNNING_NOTE: &str = "Moving the sidecars to the Trash.";

fn files(count: usize) -> String {
    format!("{count} {}", if count == 1 { "file" } else { "files" })
}

fn sidecars(count: usize) -> &'static str {
    if count == 1 { "sidecar" } else { "sidecars" }
}

/// A sidecar format or kind as the backend's setting names it, as the
/// dialog names it.
pub fn format_name(format: &str) -> &'static str {
    match format {
        "xmp" => "XMP",
        "dop" => ".dop",
        _ => "XMP and .dop",
    }
}

/// The rewrite dialog's rows, leaving out a count of zero.
pub fn rewrite_rows(preview: &Preview) -> Vec<String> {
    let mut rows = Vec::new();
    if preview.judged > 0 {
        rows.push(format!("{} with a judgment", files(preview.judged)));
    }
    if preview.unjudged > 0 {
        rows.push(format!(
            "{} without one (any existing sidecar is cleared)",
            files(preview.unjudged)
        ));
    }
    if preview.skipped > 0 {
        rows.push(format!(
            "{} not in the index (skipped)",
            files(preview.skipped)
        ));
    }
    rows
}

pub fn rewrite_total_line(preview: &Preview, name: &str) -> String {
    format!(
        "Rewrite the {} sidecars of {} in {name}?",
        format_name(preview.format),
        files(preview.judged + preview.unjudged)
    )
}

pub fn rewritten_status(summary: &Summary, name: &str) -> String {
    let head = format!(
        "Rewrote the sidecars of {} in {name}",
        files(summary.written)
    );
    if summary.failed == 0 {
        head
    } else {
        format!("{head}, {} failed", summary.failed)
    }
}

/// The delete dialog's rows, one per kind with its size.
pub fn delete_rows(preview: &DeletePreview) -> Vec<String> {
    preview
        .kinds
        .iter()
        .map(|kind| {
            format!(
                "{} {} {} ({})",
                kind.count,
                format_name(kind.kind),
                sidecars(kind.count),
                kind.size_text
            )
        })
        .collect()
}

pub fn delete_total_line(preview: &DeletePreview, name: &str) -> String {
    format!(
        "Move {} {} ({}) of {name} to the Trash?",
        preview.count,
        sidecars(preview.count),
        preview.size_text
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rewrite,
    Delete,
}

impl Kind {
    /// The idle gate's label of a request held for the scan's end.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Rewrite => "Rewrite Sidecars from Index",
            Kind::Delete => "Delete Sidecars",
        }
    }
}

/// The folder whose sidecars are rewritten or deleted, as the tree spells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub kind: Kind,
    pub dir: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Previewing,
    Previewed,
    Running,
}

/// The flow of one rewrite (or delete) of a folder's sidecars, from the menu
/// item through the dialog to the run's end. A second start while one is
/// under way does nothing. The dialog is open while previewed and while
/// running; a running run cannot be canceled.
pub struct SidecarFlow {
    phase: Phase,
    target: Option<Target>,
}

impl Default for SidecarFlow {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            target: None,
        }
    }
}

impl SidecarFlow {
    pub fn is_open(&self) -> bool {
        matches!(self.phase, Phase::Previewed | Phase::Running)
    }

    pub fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }

    pub fn start(&mut self, target: Target) -> bool {
        if self.busy() {
            return false;
        }
        self.phase = Phase::Previewing;
        self.target = Some(target);
        true
    }

    /// The preview came back: whether the dialog opens on it.
    pub fn previewed(&mut self) -> bool {
        if self.phase != Phase::Previewing {
            return false;
        }
        self.phase = Phase::Previewed;
        true
    }

    /// The target to run on, or `None` when there is no preview to run.
    pub fn run(&mut self) -> Option<Target> {
        if self.phase != Phase::Previewed {
            return None;
        }
        self.phase = Phase::Running;
        self.target.clone()
    }

    /// True when the dialog should close: Cancel or Escape on a preview.
    pub fn dismiss(&mut self) -> bool {
        if self.phase != Phase::Previewed {
            return false;
        }
        self.end();
        true
    }

    /// The preview failed or the run came back, either way.
    pub fn end(&mut self) {
        self.phase = Phase::Idle;
        self.target = None;
    }
}

#[cfg(test)]
mod tests {
    use riffle_backend::foldersidecars::KindCount;

    use super::*;

    fn preview(judged: usize, unjudged: usize, skipped: usize) -> Preview {
        Preview {
            judged,
            unjudged,
            skipped,
            format: "xmp",
        }
    }

    #[test]
    fn rewrite_rows_list_the_judged_the_unjudged_and_the_skipped_files() {
        assert_eq!(
            rewrite_rows(&preview(12, 3, 5)),
            vec![
                "12 files with a judgment",
                "3 files without one (any existing sidecar is cleared)",
                "5 files not in the index (skipped)",
            ]
        );
    }

    #[test]
    fn rewrite_rows_leave_a_count_of_zero_out_and_speak_of_one_file_in_the_singular() {
        assert_eq!(
            rewrite_rows(&preview(1, 1, 0)),
            vec![
                "1 file with a judgment",
                "1 file without one (any existing sidecar is cleared)",
            ]
        );
        assert_eq!(
            rewrite_rows(&preview(0, 2, 1)),
            vec![
                "2 files without one (any existing sidecar is cleared)",
                "1 file not in the index (skipped)",
            ]
        );
    }

    #[test]
    fn rewrite_total_line_names_the_format_and_counts_the_files_written_not_the_skipped_ones() {
        assert_eq!(
            rewrite_total_line(&preview(12, 3, 5), "2026-10-05"),
            "Rewrite the XMP sidecars of 15 files in 2026-10-05?"
        );
        assert_eq!(
            rewrite_total_line(
                &Preview {
                    format: "dop",
                    ..preview(1, 0, 0)
                },
                "a"
            ),
            "Rewrite the .dop sidecars of 1 file in a?"
        );
        assert_eq!(
            rewrite_total_line(
                &Preview {
                    format: "both",
                    ..preview(2, 0, 0)
                },
                "a"
            ),
            "Rewrite the XMP and .dop sidecars of 2 files in a?"
        );
    }

    #[test]
    fn format_name_names_each_format() {
        assert_eq!(format_name("xmp"), "XMP");
        assert_eq!(format_name("dop"), ".dop");
        assert_eq!(format_name("both"), "XMP and .dop");
    }

    #[test]
    fn rewritten_status_counts_the_files_written_and_adds_the_failures() {
        let summary = |written, failed, flushed| Summary {
            written,
            failed,
            flushed,
        };
        assert_eq!(
            rewritten_status(&summary(15, 0, true), "a"),
            "Rewrote the sidecars of 15 files in a"
        );
        assert_eq!(
            rewritten_status(&summary(1, 0, true), "a"),
            "Rewrote the sidecars of 1 file in a"
        );
        assert_eq!(
            rewritten_status(&summary(13, 2, true), "a"),
            "Rewrote the sidecars of 13 files in a, 2 failed"
        );
        assert_eq!(
            rewritten_status(&summary(10, 5, false), "a"),
            "Rewrote the sidecars of 10 files in a, 5 failed"
        );
    }

    fn kind(kind: &'static str, count: usize, size_text: &str) -> KindCount {
        KindCount {
            kind,
            count,
            size_text: size_text.to_owned(),
        }
    }

    fn both() -> DeletePreview {
        DeletePreview {
            kinds: vec![kind("xmp", 12, "1.2 MB"), kind("dop", 12, "340.0 KB")],
            count: 24,
            size_text: "1.5 MB".to_owned(),
        }
    }

    #[test]
    fn delete_rows_list_each_kind_with_its_size() {
        assert_eq!(
            delete_rows(&both()),
            vec!["12 XMP sidecars (1.2 MB)", "12 .dop sidecars (340.0 KB)"]
        );
        assert_eq!(
            delete_rows(&DeletePreview {
                kinds: vec![kind("dop", 1, "2 KB")],
                count: 1,
                size_text: "2 KB".to_owned(),
            }),
            vec!["1 .dop sidecar (2 KB)"]
        );
    }

    #[test]
    fn delete_total_line_counts_the_sidecars_and_names_the_folder() {
        assert_eq!(
            delete_total_line(&both(), "2026-10-05"),
            "Move 24 sidecars (1.5 MB) of 2026-10-05 to the Trash?"
        );
        assert_eq!(
            delete_total_line(
                &DeletePreview {
                    count: 1,
                    size_text: "800 B".to_owned(),
                    ..both()
                },
                "a"
            ),
            "Move 1 sidecar (800 B) of a to the Trash?"
        );
    }

    fn target() -> Target {
        Target {
            kind: Kind::Rewrite,
            dir: "/photos".to_owned(),
        }
    }

    #[test]
    fn the_flow_refuses_a_second_start_while_one_is_under_way() {
        let mut flow = SidecarFlow::default();
        assert!(flow.start(target()));
        assert!(flow.busy());
        assert!(!flow.is_open());
        assert!(!flow.start(Target {
            kind: Kind::Delete,
            dir: "/other".to_owned(),
        }));
        assert!(flow.previewed());
        assert!(flow.is_open());
        assert_eq!(flow.run(), Some(target()));
        assert!(!flow.start(target()));
        flow.end();
        assert!(!flow.busy());
        assert!(flow.start(target()));
    }

    #[test]
    fn the_flow_closes_a_preview_on_dismiss_but_not_a_running_rewrite() {
        let mut flow = SidecarFlow::default();
        flow.start(target());
        flow.previewed();
        assert!(flow.dismiss());
        assert!(!flow.busy());
        assert_eq!(flow.run(), None);
        flow.start(target());
        flow.previewed();
        flow.run();
        assert!(!flow.dismiss());
        assert!(flow.is_open());
    }

    #[test]
    fn a_failed_preview_ends_the_flow_before_any_dialog() {
        let mut flow = SidecarFlow::default();
        flow.start(target());
        flow.end();
        assert!(!flow.previewed());
        assert!(!flow.is_open());
    }

    #[test]
    fn the_gate_labels_name_each_operation() {
        assert_eq!(Kind::Rewrite.label(), "Rewrite Sidecars from Index");
        assert_eq!(Kind::Delete.label(), "Delete Sidecars");
    }
}
