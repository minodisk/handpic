//! The status row's task indicators: every running task in a fixed kind
//! order (`tasks`), the slot each kind's indicator takes (`slots`), and how
//! full a counted one's circle is, a port of `progress.ts`'s
//! `progressWidth`.

use crate::{icons, import, sequence, trash};

/// A task's kind, one status row slot each, in the slots' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Scan,
    Trash,
    Folder,
    Update,
}

pub const KINDS: [Kind; 4] = [Kind::Scan, Kind::Trash, Kind::Folder, Kind::Update];

impl Kind {
    /// The Lucide icon drawn beside the kind's circle.
    pub fn icon(self) -> &'static str {
        match self {
            Kind::Scan => icons::SCAN_FACE,
            Kind::Trash => icons::TRASH,
            Kind::Folder => icons::FOLDER,
            Kind::Update => icons::DOWNLOAD,
        }
    }
}

/// How far a task got: counted, or not known (a held operation, a download
/// of unknown size, the update's verify), drawn dim and indeterminate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Amount {
    Counted { done: usize, total: usize },
    Unknown,
}

/// One running task: its kind, its name, how far it got and its tooltip's
/// one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub kind: Kind,
    pub label: String,
    pub amount: Amount,
    pub tip: String,
}

/// A folder operation's run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderRun {
    Sequence,
    Import,
    /// A sidecar rewrite or delete, under its label.
    Sidecars(&'static str),
}

impl FolderRun {
    fn label(self) -> &'static str {
        match self {
            FolderRun::Sequence => sequence::LABEL,
            FolderRun::Import => import::LABEL,
            FolderRun::Sidecars(label) => label,
        }
    }
}

/// What runs, as the view holds it.
#[derive(Clone, Debug, Default)]
pub struct Sources {
    /// The scan's running pass, as the status line names it, with its
    /// `(done, total)`.
    pub scan: Option<(&'static str, usize, usize)>,
    /// A Move to Trash or Delete Permanently run, or its redo, under its
    /// label.
    pub trash: Option<(&'static str, trash::Mode, usize, usize)>,
    /// The undo of a run, under its label.
    pub undo: Option<(&'static str, usize, usize)>,
    pub folder: Option<(FolderRun, usize, usize)>,
    /// The operation the idle gate holds, its kind and its label.
    pub held: Option<(Kind, &'static str)>,
    /// The update's download, its percent when the size is known, `None` also
    /// while it is verified, with the update's phase text.
    pub update: Option<(Option<u8>, String)>,
}

/// Every running task, scan, trash, folder and update in that order, a
/// kind's running task before the one the gate holds.
pub fn tasks(sources: Sources) -> Vec<Task> {
    let counted = |done, total| Amount::Counted { done, total };
    let mut tasks = Vec::new();
    if let Some((name, done, total)) = sources.scan {
        tasks.push(Task {
            kind: Kind::Scan,
            label: "Scan".to_owned(),
            amount: counted(done, total),
            tip: format!("{name} {done}/{total}"),
        });
    }
    let held = |kind: Kind| {
        sources
            .held
            .filter(|(held, _)| *held == kind)
            .map(|(_, label)| Task {
                kind,
                label: label.to_owned(),
                amount: Amount::Unknown,
                tip: format!("{label}: waiting for the scan to finish"),
            })
    };
    if let Some((label, mode, done, total)) = sources.trash {
        tasks.push(Task {
            kind: Kind::Trash,
            label: label.to_owned(),
            amount: counted(done, total),
            tip: trash::progress_status(mode, done, total),
        });
    }
    if let Some((label, done, total)) = sources.undo {
        tasks.push(Task {
            kind: Kind::Trash,
            label: label.to_owned(),
            amount: counted(done, total),
            tip: format!("{label} {done} / {total}"),
        });
    }
    tasks.extend(held(Kind::Trash));
    if let Some((run, done, total)) = sources.folder {
        tasks.push(Task {
            kind: Kind::Folder,
            label: run.label().to_owned(),
            amount: counted(done, total),
            tip: match run {
                FolderRun::Sequence => sequence::progress_status(done, total),
                FolderRun::Import => import::progress_status(done, total),
                FolderRun::Sidecars(label) => format!("{label} {done} / {total}"),
            },
        });
    }
    tasks.extend(held(Kind::Folder));
    if let Some((percent, text)) = sources.update {
        tasks.push(Task {
            kind: Kind::Update,
            label: "Update".to_owned(),
            amount: match percent {
                Some(percent) => counted(percent.into(), 100),
                None => Amount::Unknown,
            },
            tip: text,
        });
    }
    tasks
}

/// Each kind's slot in `KINDS` order with the task it shows, its first, an
/// empty slot keeping its place.
pub fn slots(tasks: &[Task]) -> [(Kind, Option<&Task>); 4] {
    KINDS.map(|kind| (kind, tasks.iter().find(|task| task.kind == kind)))
}

/// The indicator's share of the track: 0 while nothing is counted, never
/// above 1.
pub fn fraction(done: usize, total: usize) -> f32 {
    if total == 0 {
        return 0.;
    }
    (done as f32 / total as f32).min(1.)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(tasks: &[Task]) -> Vec<Kind> {
        tasks.iter().map(|task| task.kind).collect()
    }

    #[test]
    fn a_zero_total_is_an_empty_bar() {
        assert_eq!(fraction(0, 0), 0.);
        assert_eq!(fraction(3, 0), 0.);
    }

    #[test]
    fn the_fraction_is_done_over_total_capped_at_one() {
        assert_eq!(fraction(0, 4), 0.);
        assert_eq!(fraction(1, 4), 0.25);
        assert_eq!(fraction(4, 4), 1.);
        assert_eq!(fraction(5, 4), 1.);
    }

    #[test]
    fn nothing_running_is_no_task_and_four_empty_slots() {
        let tasks = tasks(Sources::default());
        assert!(tasks.is_empty());
        assert_eq!(slots(&tasks), KINDS.map(|kind| (kind, None)));
    }

    #[test]
    fn the_scan_alone_is_its_pass_counted() {
        let tasks = tasks(Sources {
            scan: Some(("analyzing", 3, 10)),
            ..Sources::default()
        });
        assert_eq!(
            tasks,
            [Task {
                kind: Kind::Scan,
                label: "Scan".into(),
                amount: Amount::Counted { done: 3, total: 10 },
                tip: "analyzing 3/10".into(),
            }]
        );
    }

    #[test]
    fn a_trash_run_alone_tells_its_progress() {
        let tasks = tasks(Sources {
            trash: Some(("Delete Permanently", trash::Mode::Delete, 2, 5)),
            ..Sources::default()
        });
        assert_eq!(kinds(&tasks), [Kind::Trash]);
        assert_eq!(tasks[0].label, "Delete Permanently");
        assert_eq!(tasks[0].amount, Amount::Counted { done: 2, total: 5 });
        assert_eq!(
            tasks[0].tip,
            trash::progress_status(trash::Mode::Delete, 2, 5)
        );
    }

    #[test]
    fn an_undo_alone_is_a_trash_task_under_its_label() {
        let tasks = tasks(Sources {
            undo: Some(("Undo Move to Trash", 1, 4)),
            ..Sources::default()
        });
        assert_eq!(kinds(&tasks), [Kind::Trash]);
        assert_eq!(tasks[0].tip, "Undo Move to Trash 1 / 4");
    }

    #[test]
    fn each_folder_run_alone_tells_its_progress() {
        for (run, label, tip) in [
            (
                FolderRun::Sequence,
                sequence::LABEL,
                sequence::progress_status(1, 2),
            ),
            (
                FolderRun::Import,
                import::LABEL,
                import::progress_status(1, 2),
            ),
            (
                FolderRun::Sidecars("Delete Sidecars"),
                "Delete Sidecars",
                "Delete Sidecars 1 / 2".to_owned(),
            ),
        ] {
            let tasks = tasks(Sources {
                folder: Some((run, 1, 2)),
                ..Sources::default()
            });
            assert_eq!(kinds(&tasks), [Kind::Folder]);
            assert_eq!(tasks[0].label, label);
            assert_eq!(tasks[0].amount, Amount::Counted { done: 1, total: 2 });
            assert_eq!(tasks[0].tip, tip);
        }
    }

    #[test]
    fn a_held_operation_is_unknown_and_waits_for_the_scan() {
        let tasks = tasks(Sources {
            held: Some((Kind::Folder, "Rename")),
            ..Sources::default()
        });
        assert_eq!(kinds(&tasks), [Kind::Folder]);
        assert_eq!(tasks[0].amount, Amount::Unknown);
        assert_eq!(tasks[0].tip, "Rename: waiting for the scan to finish");
    }

    #[test]
    fn the_update_is_counted_by_percent_else_unknown() {
        let tasks = tasks(Sources {
            update: Some((Some(40), "Downloading Handpic 2.0.0\u{2026} 40%".into())),
            ..Sources::default()
        });
        assert_eq!(kinds(&tasks), [Kind::Update]);
        assert_eq!(
            tasks[0].amount,
            Amount::Counted {
                done: 40,
                total: 100
            }
        );
        assert_eq!(tasks[0].tip, "Downloading Handpic 2.0.0\u{2026} 40%");
        let tasks = super::tasks(Sources {
            update: Some((None, "Verifying Handpic 2.0.0\u{2026}".into())),
            ..Sources::default()
        });
        assert_eq!(tasks[0].amount, Amount::Unknown);
        assert_eq!(tasks[0].tip, "Verifying Handpic 2.0.0\u{2026}");
    }

    #[test]
    fn several_kinds_keep_the_fixed_order() {
        let tasks = tasks(Sources {
            scan: Some(("scanning", 1, 9)),
            trash: Some(("Move to Trash", trash::Mode::Trash, 0, 3)),
            folder: Some((FolderRun::Import, 4, 8)),
            update: Some((None, "Downloading Handpic 2.0.0\u{2026}".into())),
            ..Sources::default()
        });
        assert_eq!(
            kinds(&tasks),
            [Kind::Scan, Kind::Trash, Kind::Folder, Kind::Update]
        );
        let slots = slots(&tasks);
        assert_eq!(slots.map(|(kind, _)| kind), KINDS);
        assert!(slots.iter().all(|(_, task)| task.is_some()));
    }

    #[test]
    fn an_empty_kind_keeps_its_slot() {
        let tasks = tasks(Sources {
            scan: Some(("scanning", 1, 9)),
            update: Some((Some(5), String::new())),
            ..Sources::default()
        });
        let slots = slots(&tasks);
        assert_eq!(
            slots.map(|(_, task)| task.map(|t| t.kind)),
            [Some(Kind::Scan), None, None, Some(Kind::Update)]
        );
    }

    #[test]
    fn a_kinds_running_task_comes_before_the_held_one_and_takes_the_slot() {
        let tasks = tasks(Sources {
            trash: Some(("Move to Trash", trash::Mode::Trash, 1, 2)),
            held: Some((Kind::Trash, "Delete Permanently")),
            ..Sources::default()
        });
        assert_eq!(kinds(&tasks), [Kind::Trash, Kind::Trash]);
        assert_eq!(tasks[1].amount, Amount::Unknown);
        assert_eq!(slots(&tasks)[1].1, Some(&tasks[0]));
    }

    #[test]
    fn a_held_operation_of_one_kind_leaves_the_others_alone() {
        let tasks = tasks(Sources {
            scan: Some(("scanning", 0, 3)),
            held: Some((Kind::Trash, "Move to Trash")),
            ..Sources::default()
        });
        assert_eq!(kinds(&tasks), [Kind::Scan, Kind::Trash]);
    }

    #[test]
    fn each_kind_has_its_icon() {
        assert_eq!(
            KINDS.map(Kind::icon),
            [
                icons::SCAN_FACE,
                icons::TRASH,
                icons::FOLDER,
                icons::DOWNLOAD
            ]
        );
    }
}
