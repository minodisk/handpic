//! The settings' Cache page, kept pure: its texts, the index's size on disk
//! and the `Clear Cache` flow from the press through the wait for a running
//! scan and the confirmation to the clear's end. A port of the Tauri
//! `index_size` / `clear_index` commands and of `settings.ts`'s `clearGate`;
//! the window runs `Index::clear`.

use std::path::Path;

use riffle_backend::index;

use crate::dialog::{Content, Dialog, Style};

/// The Tauri paragraph under the size line.
pub const NOTE: &str = "Clearing drops every cached thumbnail, so the next open of a folder \
     scans it again; your judgments are kept, because they live in the sidecars.";

/// Shown while a press waits for the running scan.
pub const WAITING_NOTE: &str = "The cache is cleared as soon as the running scan finishes.";

pub const CLEAR_LABEL: &str = "Clear Cache";

/// The size shown before the first read of it answers.
pub const UNKNOWN_SIZE: &str = "\u{2026}";

pub fn size_line(size: &str) -> String {
    format!("Index cache: {size}")
}

/// The confirmation, the Tauri native dialog's text and buttons.
pub fn dialog() -> Dialog {
    Dialog::new(
        Content {
            title: "Clear the index cache?".to_owned(),
            lines: vec![
                "Every cached thumbnail and the cached metadata of every folder are removed. \
                 Judgments are kept. The next open of a folder scans it again."
                    .to_owned(),
            ],
            ..Content::default()
        },
        "Clear",
        Style::Destructive,
        true,
    )
}

/// The index's footprint on disk: the database file plus the `-wal` and
/// `-shm` files SQLite keeps beside it. A missing file counts as zero.
pub fn on_disk_bytes(path: &Path) -> u64 {
    [
        path.to_path_buf(),
        index::with_suffix(path, "-wal"),
        index::with_suffix(path, "-shm"),
    ]
    .iter()
    .map(|p| std::fs::metadata(p).map_or(0, |m| m.len()))
    .sum()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum State {
    #[default]
    Idle,
    /// Held for the scan's end, `confirmed` once the dialog was answered.
    Waiting {
        confirmed: bool,
    },
    Asking,
    Running,
}

/// What the window does next.
#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    /// Open the confirmation.
    Ask,
    /// Run the clear.
    Run,
    /// Nothing now: the request waits for the scan, or there is none.
    Wait,
}

/// One clear at a time, held in a slot of its own (not the view's idle
/// gate, whose request would replace a held trash or rename). The scan is
/// waited for first, then the user confirms, then the busy rule is checked
/// again, as a re-listing may have started while the dialog was up.
#[derive(Default)]
pub struct ClearFlow {
    state: State,
}

impl ClearFlow {
    /// `Clear Cache` pressed; ignored while a clear is asked or runs. A held
    /// one the view has since left idle for goes on at the press, for no
    /// listing's end may be left to drain it.
    pub fn press(&mut self, busy: bool) -> Next {
        if self.state == (State::Waiting { confirmed: false }) && !busy {
            self.state = State::Asking;
            return Next::Ask;
        }
        if self.state != State::Idle {
            return Next::Wait;
        }
        if busy {
            self.state = State::Waiting { confirmed: false };
            Next::Wait
        } else {
            self.state = State::Asking;
            Next::Ask
        }
    }

    /// The view became idle: a held clear goes on.
    pub fn drain(&mut self) -> Next {
        match self.state {
            State::Waiting { confirmed: false } => {
                self.state = State::Asking;
                Next::Ask
            }
            State::Waiting { confirmed: true } => {
                self.state = State::Running;
                Next::Run
            }
            _ => Next::Wait,
        }
    }

    /// The dialog's `Clear`: run now, or wait when the view became busy
    /// while it was up.
    pub fn confirm(&mut self, busy: bool) -> Next {
        if self.state != State::Asking {
            return Next::Wait;
        }
        if busy {
            self.state = State::Waiting { confirmed: true };
            Next::Wait
        } else {
            self.state = State::Running;
            Next::Run
        }
    }

    /// The dialog's Cancel or Escape.
    pub fn cancel(&mut self) {
        if self.state == State::Asking {
            self.state = State::Idle;
        }
    }

    /// The clear came back.
    pub fn done(&mut self) {
        if self.state == State::Running {
            self.state = State::Idle;
        }
    }

    /// The settings closed: a held or asked clear is dropped, one that runs
    /// goes on.
    pub fn discard(&mut self) {
        if self.state != State::Running {
            self.state = State::Idle;
        }
    }

    pub fn waiting(&self) -> bool {
        matches!(self.state, State::Waiting { .. })
    }

    pub fn asking(&self) -> bool {
        self.state == State::Asking
    }

    pub fn running(&self) -> bool {
        self.state == State::Running
    }
}

#[cfg(test)]
mod tests {
    use super::{ClearFlow, Next, on_disk_bytes, size_line};
    use crate::dialog::{Button, Style};

    #[test]
    fn a_press_when_idle_asks_and_a_confirm_runs() {
        let mut flow = ClearFlow::default();
        assert_eq!(flow.press(false), Next::Ask);
        assert!(flow.asking());
        assert_eq!(flow.confirm(false), Next::Run);
        assert!(flow.running());
        flow.done();
        assert!(!flow.running() && !flow.asking() && !flow.waiting());
    }

    #[test]
    fn a_press_while_busy_waits_then_asks_when_idle() {
        let mut flow = ClearFlow::default();
        assert_eq!(flow.press(true), Next::Wait);
        assert!(flow.waiting());
        assert_eq!(flow.drain(), Next::Ask);
        assert!(flow.asking());
        assert_eq!(flow.drain(), Next::Wait);
    }

    #[test]
    fn a_confirm_while_busy_waits_then_runs_without_asking_again() {
        let mut flow = ClearFlow::default();
        flow.press(false);
        assert_eq!(flow.confirm(true), Next::Wait);
        assert!(flow.waiting());
        assert_eq!(flow.drain(), Next::Run);
        assert!(flow.running());
    }

    #[test]
    fn a_cancel_changes_nothing() {
        let mut flow = ClearFlow::default();
        flow.press(false);
        flow.cancel();
        assert!(!flow.asking() && !flow.running() && !flow.waiting());
        assert_eq!(flow.drain(), Next::Wait);
        assert_eq!(flow.confirm(false), Next::Wait);
    }

    #[test]
    fn a_press_while_held_and_idle_asks() {
        let mut flow = ClearFlow::default();
        flow.press(true);
        assert_eq!(flow.press(false), Next::Ask);
        assert!(flow.asking());
    }

    #[test]
    fn a_second_press_is_ignored() {
        let mut flow = ClearFlow::default();
        flow.press(true);
        assert_eq!(flow.press(true), Next::Wait);
        assert!(flow.waiting());
        let mut flow = ClearFlow::default();
        flow.press(false);
        flow.confirm(false);
        assert_eq!(flow.press(false), Next::Wait);
        assert!(flow.running());
    }

    #[test]
    fn closing_the_settings_drops_a_held_or_asked_clear_but_not_a_running_one() {
        let mut flow = ClearFlow::default();
        flow.press(true);
        flow.discard();
        assert!(!flow.waiting());
        assert_eq!(flow.drain(), Next::Wait);
        flow.press(false);
        flow.discard();
        assert!(!flow.asking());
        flow.press(false);
        flow.confirm(true);
        flow.discard();
        assert_eq!(flow.drain(), Next::Wait);
        flow.press(false);
        flow.confirm(false);
        flow.discard();
        assert!(flow.running());
    }

    #[test]
    fn the_texts_are_the_tauri_ones() {
        assert_eq!(size_line("1.2 MB"), "Index cache: 1.2 MB");
        let d = super::dialog();
        assert_eq!(d.content.title, "Clear the index cache?");
        assert_eq!(
            d.content.lines,
            [
                "Every cached thumbnail and the cached metadata of every folder are removed. \
              Judgments are kept. The next open of a folder scans it again."
            ]
        );
        assert_eq!(d.run_label, "Clear");
        assert_eq!(d.style, Style::Destructive);
        assert_eq!(d.focus(), Some(Button::Run));
    }

    #[test]
    fn the_size_on_disk_sums_the_three_files_a_missing_one_counting_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.sqlite");
        assert_eq!(on_disk_bytes(&path), 0);
        std::fs::write(&path, [0; 100]).unwrap();
        std::fs::write(dir.path().join("index.sqlite-wal"), [0; 20]).unwrap();
        assert_eq!(on_disk_bytes(&path), 120);
        std::fs::write(dir.path().join("index.sqlite-shm"), [0; 3]).unwrap();
        assert_eq!(on_disk_bytes(&path), 123);
    }
}
