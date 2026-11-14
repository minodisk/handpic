//! The confirmation dialog the folder operations share, kept pure: what it
//! shows, its run button and Cancel, which of them are enabled, which one the
//! focus goes to, and what a key does while it is open. The focus itself and
//! the Tab walk between the buttons are GPUI's, in the component `Dialog`
//! `folderops.rs` draws for all of them. The Tauri app gives each operation
//! its own HTML dialog with the settings modal's key decisions.

/// The run button's look: `primary`, or `destructive` for a run that moves
/// files away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Primary,
    Destructive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Run,
    Cancel,
}

/// One row of the dialog's list: its text, then a part set apart in the
/// accent color (empty for none), the whole row muted when `muted`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub text: String,
    pub accent: String,
    pub muted: bool,
}

impl From<String> for Row {
    fn from(text: String) -> Self {
        Self {
            text,
            ..Self::default()
        }
    }
}

/// What the dialog shows, top to bottom: the title, its text lines, a list
/// of rows and then each further list under its heading (scrolling together
/// when long), the notes, the errors, the reason the run is disabled, and,
/// while the run is under way, the running note.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Content {
    pub title: String,
    pub lines: Vec<String>,
    pub rows: Vec<Row>,
    /// Lists after the rows, each a heading and its items; an empty one is
    /// not shown.
    pub lists: Vec<(String, Vec<String>)>,
    pub notes: Vec<String>,
    pub errors: Vec<String>,
    pub reason: Option<String>,
    pub running_note: String,
}

/// What a key does while the dialog is open. Tab and Shift+Tab never get
/// here: `Root`'s focus trap walks the enabled buttons.
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    /// Escape: the operation decides whether the dialog closes, or what a
    /// run under way does.
    Dismiss,
    /// Enter or Space, left to the focused button, which presses itself.
    Press,
    /// Nothing; the key stops there.
    Swallow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dialog {
    pub content: Content,
    pub run_label: String,
    pub style: Style,
    run_enabled: bool,
    running: bool,
    /// Cancel stays enabled while the run is under way.
    cancelable: bool,
}

impl Dialog {
    /// A dialog asking with `content`.
    pub fn new(content: Content, run_label: &str, style: Style, run_enabled: bool) -> Self {
        Self {
            content,
            run_label: run_label.to_owned(),
            style,
            run_enabled,
            running: false,
            cancelable: false,
        }
    }

    /// The run started: both buttons disabled and the running note shown.
    pub fn set_running(&mut self) {
        self.running = true;
    }

    /// The run started and Cancel cancels it: the run button disabled, the
    /// focus moved to Cancel and the running note shown.
    pub fn set_running_cancelable(&mut self) {
        self.running = true;
        self.cancelable = true;
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn enabled(&self, button: Button) -> bool {
        match button {
            Button::Run => !self.running && self.run_enabled,
            Button::Cancel => !self.running || self.cancelable,
        }
    }

    /// The button the focus goes to, on the dialog's opening and again
    /// whenever this changes: the run button, or Cancel when the run is
    /// disabled or a cancelable run is under way; `None` while no button is
    /// enabled, the focus staying on the disabled button that has it.
    pub fn focus(&self) -> Option<Button> {
        [Button::Run, Button::Cancel]
            .into_iter()
            .find(|&button| self.enabled(button))
    }

    /// A key, `key` as `keys::key_name` names it (`None` for a lone
    /// modifier): Escape asks to dismiss, Enter (or Space, as on a focused
    /// HTML button) goes on to the focused button while one is enabled, and
    /// every other key is swallowed.
    pub fn key(&self, key: Option<&str>) -> Key {
        match key {
            Some("escape") => Key::Dismiss,
            Some("enter" | "space") if self.focus().is_some() => Key::Press,
            _ => Key::Swallow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dialog(run_enabled: bool) -> Dialog {
        Dialog::new(
            Content {
                title: "Delete Sidecars".to_owned(),
                lines: vec!["Move 2 sidecars (1 KB) of a to the Trash?".to_owned()],
                rows: vec!["2 XMP sidecars (1 KB)".to_owned().into()],
                running_note: "Moving the sidecars to the Trash.".to_owned(),
                ..Content::default()
            },
            "Move to Trash",
            Style::Destructive,
            run_enabled,
        )
    }

    #[test]
    fn the_focus_starts_on_the_run_button() {
        let d = dialog(true);
        assert_eq!(d.focus(), Some(Button::Run));
        assert!(d.enabled(Button::Run));
        assert!(d.enabled(Button::Cancel));
        assert!(!d.running());
    }

    #[test]
    fn the_focus_starts_on_cancel_when_the_run_is_disabled() {
        let d = dialog(false);
        assert_eq!(d.focus(), Some(Button::Cancel));
        assert!(!d.enabled(Button::Run));
        assert!(d.enabled(Button::Cancel));
    }

    #[test]
    fn enter_and_space_go_on_to_the_focused_button() {
        for run_enabled in [true, false] {
            let d = dialog(run_enabled);
            assert_eq!(d.key(Some("enter")), Key::Press);
            assert_eq!(d.key(Some("space")), Key::Press);
        }
    }

    #[test]
    fn escape_asks_to_dismiss_and_every_other_key_is_swallowed() {
        let d = dialog(true);
        assert_eq!(d.key(Some("escape")), Key::Dismiss);
        for key in [
            Some("p"),
            Some("ctrl+z"),
            Some("arrowleft"),
            Some("shift+enter"),
            Some("ctrl+space"),
            None,
        ] {
            assert_eq!(d.key(key), Key::Swallow, "{key:?}");
        }
    }

    #[test]
    fn a_running_dialog_disables_both_buttons_and_presses_nothing() {
        let mut d = dialog(true);
        d.set_running();
        assert!(d.running());
        assert!(!d.enabled(Button::Run));
        assert!(!d.enabled(Button::Cancel));
        assert_eq!(d.focus(), None);
        assert_eq!(d.key(Some("enter")), Key::Swallow);
        assert_eq!(d.key(Some("space")), Key::Swallow);
        assert_eq!(d.key(Some("escape")), Key::Dismiss);
    }

    #[test]
    fn a_cancelable_run_keeps_cancel_enabled_and_moves_the_focus_there() {
        let mut d = dialog(true);
        d.set_running_cancelable();
        assert!(d.running());
        assert!(!d.enabled(Button::Run));
        assert!(d.enabled(Button::Cancel));
        assert_eq!(d.focus(), Some(Button::Cancel));
        assert_eq!(d.key(Some("enter")), Key::Press);
        assert_eq!(d.key(Some("escape")), Key::Dismiss);
    }
}
