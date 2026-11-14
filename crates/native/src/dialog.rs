//! The confirmation dialog the folder operations share, kept pure: what it
//! shows, its run button and Cancel, which of the enabled buttons has the
//! focus, and what a key does while it is open. The Tauri app gives each
//! operation its own HTML dialog with the settings modal's key decisions;
//! `folderops.rs` draws this one for all of them.

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

/// What a key does while the dialog is open. Every key stops there.
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    /// Escape: the operation decides whether the dialog closes, or what a
    /// run under way does.
    Dismiss,
    /// The focused button pressed.
    Press(Button),
    /// Nothing, or only the focus moved.
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
    focused: Button,
}

impl Dialog {
    /// A dialog asking with `content`, its focus on the run button, or on
    /// Cancel when the run is disabled.
    pub fn new(content: Content, run_label: &str, style: Style, run_enabled: bool) -> Self {
        Self {
            content,
            run_label: run_label.to_owned(),
            style,
            run_enabled,
            running: false,
            cancelable: false,
            focused: if run_enabled {
                Button::Run
            } else {
                Button::Cancel
            },
        }
    }

    /// The run started: both buttons disabled and the running note shown.
    pub fn set_running(&mut self) {
        self.running = true;
    }

    /// The run started and Cancel cancels it: the run button disabled, the
    /// focus on Cancel and the running note shown.
    pub fn set_running_cancelable(&mut self) {
        self.running = true;
        self.cancelable = true;
        self.focused = Button::Cancel;
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

    /// The focused button, `None` while no button is enabled.
    pub fn focused(&self) -> Option<Button> {
        self.enabled(self.focused).then_some(self.focused)
    }

    /// A key, `key` as `keys::key_name` names it (`None` for a lone
    /// modifier): Escape asks to dismiss, Tab and Shift+Tab cycle the
    /// enabled buttons, Enter (or Space, as on a focused HTML button) presses
    /// the focused one, and every other key is swallowed.
    pub fn key(&mut self, key: Option<&str>) -> Key {
        match key {
            Some("escape") => Key::Dismiss,
            Some("tab" | "shift+tab") => {
                let other = match self.focused {
                    Button::Run => Button::Cancel,
                    Button::Cancel => Button::Run,
                };
                if self.enabled(other) {
                    self.focused = other;
                }
                Key::Swallow
            }
            Some("enter" | "space") => self.focused().map_or(Key::Swallow, Key::Press),
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
        assert_eq!(d.focused(), Some(Button::Run));
        assert!(d.enabled(Button::Run));
        assert!(d.enabled(Button::Cancel));
        assert!(!d.running());
    }

    #[test]
    fn the_focus_starts_on_cancel_when_the_run_is_disabled() {
        let d = dialog(false);
        assert_eq!(d.focused(), Some(Button::Cancel));
        assert!(!d.enabled(Button::Run));
    }

    #[test]
    fn tab_and_shift_tab_cycle_the_enabled_buttons() {
        let mut d = dialog(true);
        assert_eq!(d.key(Some("tab")), Key::Swallow);
        assert_eq!(d.focused(), Some(Button::Cancel));
        assert_eq!(d.key(Some("tab")), Key::Swallow);
        assert_eq!(d.focused(), Some(Button::Run));
        assert_eq!(d.key(Some("shift+tab")), Key::Swallow);
        assert_eq!(d.focused(), Some(Button::Cancel));
    }

    #[test]
    fn tab_stays_on_cancel_when_the_run_is_disabled() {
        let mut d = dialog(false);
        d.key(Some("tab"));
        assert_eq!(d.focused(), Some(Button::Cancel));
        d.key(Some("shift+tab"));
        assert_eq!(d.focused(), Some(Button::Cancel));
    }

    #[test]
    fn enter_and_space_press_the_focused_button() {
        let mut d = dialog(true);
        assert_eq!(d.key(Some("enter")), Key::Press(Button::Run));
        assert_eq!(d.key(Some("space")), Key::Press(Button::Run));
        d.key(Some("tab"));
        assert_eq!(d.key(Some("enter")), Key::Press(Button::Cancel));
        assert_eq!(dialog(false).key(Some("enter")), Key::Press(Button::Cancel));
    }

    #[test]
    fn escape_asks_to_dismiss_and_every_other_key_is_swallowed() {
        let mut d = dialog(true);
        assert_eq!(d.key(Some("escape")), Key::Dismiss);
        for key in [Some("p"), Some("ctrl+z"), Some("arrowleft"), None] {
            assert_eq!(d.key(key), Key::Swallow, "{key:?}");
        }
        assert_eq!(d.focused(), Some(Button::Run));
    }

    #[test]
    fn a_running_dialog_disables_both_buttons_and_presses_nothing() {
        let mut d = dialog(true);
        d.set_running();
        assert!(d.running());
        assert!(!d.enabled(Button::Run));
        assert!(!d.enabled(Button::Cancel));
        assert_eq!(d.focused(), None);
        assert_eq!(d.key(Some("enter")), Key::Swallow);
        assert_eq!(d.key(Some("tab")), Key::Swallow);
        assert_eq!(d.focused(), None);
        assert_eq!(d.key(Some("escape")), Key::Dismiss);
    }

    #[test]
    fn a_cancelable_run_keeps_cancel_enabled_and_focused() {
        let mut d = dialog(true);
        d.set_running_cancelable();
        assert!(d.running());
        assert!(!d.enabled(Button::Run));
        assert!(d.enabled(Button::Cancel));
        assert_eq!(d.focused(), Some(Button::Cancel));
        assert_eq!(d.key(Some("tab")), Key::Swallow);
        assert_eq!(d.focused(), Some(Button::Cancel));
        assert_eq!(d.key(Some("enter")), Key::Press(Button::Cancel));
        assert_eq!(d.key(Some("escape")), Key::Dismiss);
    }
}
