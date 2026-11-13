//! The strip's `Move to Trash…` and `Delete Permanently…`, kept pure: the
//! confirmation dialog's text and flow, which keys and clicks it takes, and
//! what the pane says during the run and once it comes back. A port of the
//! Tauri app's `trash.ts`; the window draws the dialog and runs the backend's
//! `trash::trash_listed` or `trash::delete_listed`.

use riffle_backend::trash::Summary;

/// What the dialog asks, and what a progress message reports on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Trash,
    Delete,
}

impl Mode {
    /// The idle gate's label of a held run (`idle::cancels_scan`).
    pub fn label(self) -> &'static str {
        match self {
            Mode::Trash => "Move to Trash",
            Mode::Delete => "Delete Permanently",
        }
    }

    /// What the pane's notes say of a file that failed.
    pub fn failure(self) -> &'static str {
        match self {
            Mode::Trash => "could not move to the Trash",
            Mode::Delete => "could not delete",
        }
    }
}

fn files(count: usize) -> &'static str {
    if count == 1 { "file" } else { "files" }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Text {
    pub title: &'static str,
    pub line: String,
    pub run: &'static str,
}

/// The dialog's title, line and run button: a move asks with the count
/// alone, a delete in the plainest words, since it has no way back.
pub fn dialog_text(mode: Mode, count: usize) -> Text {
    match mode {
        Mode::Trash => Text {
            title: "Move to Trash",
            line: format!("Move {count} {} to the Trash?", files(count)),
            run: "Move to Trash",
        },
        Mode::Delete => Text {
            title: "Delete Permanently",
            line: if count == 1 {
                "Permanently delete 1 file and its sidecars (a .xmp shared with a file you keep stays)? It is not moved to the Trash and this cannot be undone.".to_owned()
            } else {
                format!(
                    "Permanently delete {count} files and their sidecars (a .xmp shared with a file you keep stays)? They are not moved to the Trash and this cannot be undone."
                )
            },
            run: "Delete Permanently",
        },
    }
}

fn presses(key: &str) -> bool {
    let base = key.rsplit_once('+').map_or(key, |(_, base)| base);
    // A lone `+` names the plus key, not a modifier's separator.
    key != "+" && (base == "enter" || base == "space")
}

/// The keys the delete dialog swallows, so only a mouse click confirms:
/// Enter and Space, with or without a modifier, and any key `held` from
/// before the dialog opened. `key` is `keys::key_name`'s name.
pub fn swallowed_by_delete(key: &str, held: bool) -> bool {
    held || presses(key)
}

/// Whether a click on the run button confirms a delete: only a single mouse
/// click. 0 is a key, and 2 or more the rest of a double-click that opened
/// the dialog from the menu item under the pointer.
pub fn confirms_delete(click_count: usize) -> bool {
    click_count == 1
}

/// The pane's note while a run is under way.
pub fn progress_status(mode: Mode, done: usize, total: usize) -> String {
    let label = match mode {
        Mode::Trash => "moving to Trash",
        Mode::Delete => "deleting",
    };
    format!("{label} {done} / {total}")
}

/// What the pane says once a run came back.
pub fn trashed_status(mode: Mode, summary: &Summary) -> String {
    let count = summary.moved.len();
    let went = match mode {
        Mode::Trash => format!("Moved {count} {} to the Trash", files(count)),
        Mode::Delete => format!("Deleted {count} {}", files(count)),
    };
    if summary.failed.is_empty() {
        went
    } else {
        format!("{went}, {} failed", summary.failed.len())
    }
}

/// The files of the folder `dir` the dialog asks about, by their listed
/// paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub dir: String,
    pub paths: Vec<String>,
    pub mode: Mode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Run,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Asking,
    Running,
}

/// What a key does while the dialog is open. Every key stops there.
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    /// Escape, or Cancel pressed: the question closes.
    Dismiss,
    /// The run button pressed.
    Run,
    /// Nothing, or only the focus moved.
    Swallow,
}

/// One trashing or delete, from the menu item through the dialog to the
/// run's end, and which of the two buttons has the focus. A second start
/// while one is under way does nothing; the dialog is open while asking and
/// while running, and a running move cannot be canceled.
pub struct TrashFlow {
    phase: Phase,
    target: Option<Target>,
    focused: Button,
}

impl Default for TrashFlow {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            target: None,
            focused: Button::Run,
        }
    }
}

impl TrashFlow {
    pub fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }

    pub fn running(&self) -> bool {
        self.phase == Phase::Running
    }

    /// The files asked about or being moved.
    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    pub fn focused(&self) -> Button {
        self.focused
    }

    /// Ask about `target`; a delete opens on Cancel. False while one is
    /// under way.
    pub fn start(&mut self, target: Target) -> bool {
        if self.busy() {
            return false;
        }
        self.focused = match target.mode {
            Mode::Trash => Button::Run,
            Mode::Delete => Button::Cancel,
        };
        self.phase = Phase::Asking;
        self.target = Some(target);
        true
    }

    /// The files to move, or `None` when nothing is being asked.
    pub fn run(&mut self) -> Option<Target> {
        if self.phase != Phase::Asking {
            return None;
        }
        self.phase = Phase::Running;
        self.target.clone()
    }

    /// True when the dialog should close: Cancel or Escape on the question.
    pub fn dismiss(&mut self) -> bool {
        if self.phase != Phase::Asking {
            return false;
        }
        self.end();
        true
    }

    /// The run came back, or was held for a scan's end.
    pub fn end(&mut self) {
        self.phase = Phase::Idle;
        self.target = None;
    }

    /// A key while the dialog is open, `key` as `keys::key_name` names it
    /// (`None` for a lone modifier) and `held` a repeat: Escape closes the
    /// question, Tab and Shift+Tab move between the buttons, and Enter or
    /// Space press the focused one, except in the delete dialog, which
    /// swallows them and any repeat. While running, the buttons are
    /// disabled and nothing happens.
    pub fn key(&mut self, key: Option<&str>, held: bool) -> Key {
        let Some(key) = key else { return Key::Swallow };
        if self.phase != Phase::Asking {
            return Key::Swallow;
        }
        let delete = self.target.as_ref().is_some_and(|t| t.mode == Mode::Delete);
        if delete && swallowed_by_delete(key, held) {
            return Key::Swallow;
        }
        match key {
            "escape" => Key::Dismiss,
            "tab" | "shift+tab" => {
                self.focused = match self.focused {
                    Button::Run => Button::Cancel,
                    Button::Cancel => Button::Run,
                };
                Key::Swallow
            }
            "enter" | "space" => match self.focused {
                Button::Run => Key::Run,
                Button::Cancel => Key::Dismiss,
            },
            _ => Key::Swallow,
        }
    }
}

#[cfg(test)]
mod tests {
    use riffle_backend::trash::{Failure, Summary};

    use super::*;

    fn summary(moved: &[&str], failed: usize) -> Summary {
        Summary {
            moved: moved.iter().map(|p| (*p).to_owned()).collect(),
            failed: (0..failed)
                .map(|i| Failure {
                    path: format!("/f{i}.ARW"),
                    message: "denied".to_owned(),
                })
                .collect(),
            run_id: None,
        }
    }

    #[test]
    fn progress_status_counts_the_move_and_the_delete() {
        assert_eq!(
            progress_status(Mode::Trash, 0, 2677),
            "moving to Trash 0 / 2677"
        );
        assert_eq!(
            progress_status(Mode::Trash, 12, 2677),
            "moving to Trash 12 / 2677"
        );
        assert_eq!(
            progress_status(Mode::Delete, 12, 2677),
            "deleting 12 / 2677"
        );
    }

    #[test]
    fn trashed_status_counts_the_files_it_moved_and_appends_the_failures() {
        assert_eq!(
            trashed_status(Mode::Trash, &summary(&["/a.ARW"], 0)),
            "Moved 1 file to the Trash"
        );
        assert_eq!(
            trashed_status(Mode::Trash, &summary(&["/a.ARW", "/b.ARW", "/c.ARW"], 0)),
            "Moved 3 files to the Trash"
        );
        assert_eq!(
            trashed_status(Mode::Trash, &summary(&["/b.ARW", "/c.ARW"], 1)),
            "Moved 2 files to the Trash, 1 failed"
        );
    }

    #[test]
    fn trashed_status_of_a_delete_says_the_files_were_deleted() {
        assert_eq!(
            trashed_status(Mode::Delete, &summary(&["/a.ARW", "/b.ARW"], 0)),
            "Deleted 2 files"
        );
        assert_eq!(
            trashed_status(Mode::Delete, &summary(&["/a.ARW"], 0)),
            "Deleted 1 file"
        );
        assert_eq!(
            trashed_status(Mode::Delete, &summary(&["/a.ARW", "/b.ARW"], 1)),
            "Deleted 2 files, 1 failed"
        );
    }

    #[test]
    fn dialog_text_asks_a_move_with_the_count_alone() {
        assert_eq!(
            dialog_text(Mode::Trash, 3),
            Text {
                title: "Move to Trash",
                line: "Move 3 files to the Trash?".to_owned(),
                run: "Move to Trash",
            }
        );
        assert_eq!(
            dialog_text(Mode::Trash, 1).line,
            "Move 1 file to the Trash?"
        );
        assert_eq!(
            dialog_text(Mode::Trash, 12).line,
            "Move 12 files to the Trash?"
        );
    }

    #[test]
    fn dialog_text_says_a_delete_is_permanent_singular_for_one_file() {
        assert_eq!(
            dialog_text(Mode::Delete, 3),
            Text {
                title: "Delete Permanently",
                line: "Permanently delete 3 files and their sidecars (a .xmp shared with a file you keep stays)? They are not moved to the Trash and this cannot be undone.".to_owned(),
                run: "Delete Permanently",
            }
        );
        assert_eq!(
            dialog_text(Mode::Delete, 1).line,
            "Permanently delete 1 file and its sidecars (a .xmp shared with a file you keep stays)? It is not moved to the Trash and this cannot be undone."
        );
    }

    #[test]
    fn swallowed_by_delete_takes_the_keys_that_click_a_button_and_any_repeat() {
        assert!(swallowed_by_delete("enter", false));
        assert!(swallowed_by_delete("space", false));
        assert!(swallowed_by_delete("shift+enter", false));
        assert!(swallowed_by_delete("ctrl+space", false));
        assert!(swallowed_by_delete("escape", true));
        assert!(swallowed_by_delete("tab", true));
    }

    #[test]
    fn swallowed_by_delete_leaves_escape_and_tab_to_the_dialog() {
        assert!(!swallowed_by_delete("escape", false));
        assert!(!swallowed_by_delete("tab", false));
        assert!(!swallowed_by_delete("+", false));
    }

    #[test]
    fn confirms_delete_takes_a_single_mouse_click_only() {
        assert!(confirms_delete(1));
        assert!(!confirms_delete(0));
        assert!(!confirms_delete(2));
        assert!(!confirms_delete(3));
    }

    fn target(mode: Mode) -> Target {
        Target {
            dir: "/photos".to_owned(),
            paths: vec!["/photos/a.ARW".to_owned()],
            mode,
        }
    }

    #[test]
    fn the_flow_refuses_a_second_start_while_one_is_under_way() {
        let mut flow = TrashFlow::default();
        assert!(flow.start(target(Mode::Trash)));
        assert!(!flow.start(target(Mode::Trash)));
        assert!(flow.busy());
        assert_eq!(flow.run(), Some(target(Mode::Trash)));
        assert!(!flow.start(target(Mode::Trash)));
        flow.end();
        assert!(!flow.busy());
        assert!(flow.start(target(Mode::Trash)));
    }

    #[test]
    fn the_flow_closes_the_question_on_dismiss_but_not_a_running_move() {
        let mut flow = TrashFlow::default();
        flow.start(target(Mode::Trash));
        assert!(flow.dismiss());
        assert_eq!(flow.run(), None);
        flow.start(target(Mode::Trash));
        flow.run();
        assert!(!flow.dismiss());
        assert!(flow.busy());
    }

    #[test]
    fn a_move_opens_on_its_run_button_and_enter_or_space_press_the_focused_one() {
        let mut flow = TrashFlow::default();
        flow.start(target(Mode::Trash));
        assert_eq!(flow.focused(), Button::Run);
        assert_eq!(flow.key(Some("enter"), false), Key::Run);
        assert_eq!(flow.key(Some("tab"), false), Key::Swallow);
        assert_eq!(flow.focused(), Button::Cancel);
        assert_eq!(flow.key(Some("space"), false), Key::Dismiss);
        assert_eq!(flow.key(Some("shift+tab"), false), Key::Swallow);
        assert_eq!(flow.focused(), Button::Run);
        assert_eq!(flow.key(Some("escape"), false), Key::Dismiss);
        assert_eq!(flow.key(Some("p"), false), Key::Swallow);
        assert_eq!(flow.key(None, false), Key::Swallow);
    }

    #[test]
    fn a_delete_opens_on_cancel_and_takes_no_key_as_a_yes() {
        let mut flow = TrashFlow::default();
        flow.start(target(Mode::Delete));
        assert_eq!(flow.focused(), Button::Cancel);
        assert_eq!(flow.key(Some("tab"), false), Key::Swallow);
        assert_eq!(flow.focused(), Button::Run);
        assert_eq!(flow.key(Some("enter"), false), Key::Swallow);
        assert_eq!(flow.key(Some("space"), false), Key::Swallow);
        assert_eq!(flow.key(Some("escape"), true), Key::Swallow);
        assert_eq!(flow.key(Some("escape"), false), Key::Dismiss);
    }

    #[test]
    fn a_running_flow_takes_no_key() {
        let mut flow = TrashFlow::default();
        flow.start(target(Mode::Trash));
        flow.run();
        assert_eq!(flow.key(Some("escape"), false), Key::Swallow);
        assert_eq!(flow.key(Some("enter"), false), Key::Swallow);
        assert!(flow.running());
    }
}
