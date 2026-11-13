//! The settings modal's pure model: its open state, the selected tab and the
//! shortcut row waiting for a key, the key rule of `modal.ts`'s
//! `SettingsModal`, the vertical tablist of `tabs.ts`'s `nextTab`, and the AI
//! status line of `ai.ts`'s `aiStatus`.

use riffle_backend::ai::AiState;

/// The key that opens the modal, the Tauri app menu's `Settings…`
/// accelerator, which the keymap refuses to bind.
pub const OPEN_KEY: &str = if cfg!(target_os = "macos") {
    "meta+,"
} else {
    "ctrl+,"
};

/// Under the AI toggle: the Tauri paragraph, less the background run this app
/// does not have yet.
pub const AI_NOTE: &str = "Turning it on first downloads the model from Hugging Face. This app \
     does not run the analysis yet; the setting and the model are kept for when it does.";

/// The modal's status line while the settings store did not open.
pub const NOT_SAVED: &str = "The settings store did not open, so changes last until the app quits.";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Sidecar,
    Culling,
    Shortcuts,
    Import,
}

/// The tabs in the Tauri order, less the Cache, MCP and Debug ones.
pub const TABS: [Tab; 4] = [Tab::Sidecar, Tab::Culling, Tab::Shortcuts, Tab::Import];

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Tab::Sidecar => "Sidecar",
            Tab::Culling => "Culling",
            Tab::Shortcuts => "Keyboard Shortcuts",
            Tab::Import => "Import",
        }
    }

    pub fn at(self) -> usize {
        TABS.iter().position(|&t| t == self).unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Next,
    Previous,
}

/// What a key does while the modal is open. None of them reaches the
/// culling keymap.
#[derive(Debug, PartialEq, Eq)]
pub enum ModalKey {
    Add(String),
    Cancel,
    Close,
    Focus(Step),
    /// Left to the focused control.
    Native,
}

#[derive(Default)]
pub struct SettingsModal {
    opened: bool,
    pub tab: Tab,
    /// The action whose shortcut row waits for a key.
    pub capturing: Option<&'static str>,
}

impl SettingsModal {
    pub fn is_open(&self) -> bool {
        self.opened
    }

    /// False when it is already open, so a second open does nothing.
    pub fn open(&mut self) -> bool {
        if self.opened {
            return false;
        }
        self.opened = true;
        true
    }

    pub fn close(&mut self) {
        self.opened = false;
        self.capturing = None;
    }

    /// `key` is `keys::key_name`'s name, `None` for a lone modifier. While a
    /// row captures, Escape cancels the capture and any other key is added;
    /// otherwise Escape closes and Tab moves the focus within the modal.
    pub fn key(&self, key: Option<&str>) -> ModalKey {
        if self.capturing.is_some() {
            return match key {
                None => ModalKey::Native,
                Some("escape") => ModalKey::Cancel,
                Some(key) => ModalKey::Add(key.to_owned()),
            };
        }
        match key {
            Some("escape") => ModalKey::Close,
            Some("tab") => ModalKey::Focus(Step::Next),
            Some("shift+tab") => ModalKey::Focus(Step::Previous),
            _ => ModalKey::Native,
        }
    }
}

/// The tab a vertical tablist key moves to, wrapping around, or `current`
/// for any other key.
pub fn next_tab(current: Tab, key: &str) -> Tab {
    let at = current.at();
    match key {
        "arrowup" => TABS[(at + TABS.len() - 1) % TABS.len()],
        "arrowdown" => TABS[(at + 1) % TABS.len()],
        "home" => TABS[0],
        "end" => TABS[TABS.len() - 1],
        _ => current,
    }
}

const MB: u64 = 1_000_000;

/// The status line under the AI toggle.
pub fn ai_status(state: &AiState) -> String {
    if let Some(error) = &state.error {
        return error.clone();
    }
    let total = (state.total + MB / 2) / MB;
    if let Some(downloaded) = state.downloaded {
        return format!("Downloading the model: {} / {total} MB", downloaded / MB);
    }
    match (state.enabled, state.ready) {
        (false, true) => "Off".into(),
        (false, false) => format!("Off; turning it on downloads the model ({total} MB)"),
        (true, true) => "On".into(),
        (true, false) => "The model is not downloaded".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ModalKey, SettingsModal, Step, TABS, Tab, ai_status, next_tab};
    use riffle_backend::ai::AiState;

    fn open() -> SettingsModal {
        let mut modal = SettingsModal::default();
        modal.open();
        modal
    }

    #[test]
    fn starts_closed_on_the_first_tab() {
        let modal = SettingsModal::default();
        assert!(!modal.is_open());
        assert_eq!(modal.tab, Tab::Sidecar);
    }

    #[test]
    fn opens_once_and_opening_again_while_open_is_a_no_op() {
        let mut modal = SettingsModal::default();
        assert!(modal.open());
        assert!(modal.is_open());
        assert!(!modal.open());
        assert!(modal.is_open());
    }

    #[test]
    fn closes_and_can_open_again_on_the_same_tab() {
        let mut modal = open();
        modal.tab = Tab::Culling;
        modal.close();
        assert!(!modal.is_open());
        assert!(modal.open());
        assert_eq!(modal.tab, Tab::Culling);
    }

    #[test]
    fn closing_cancels_a_capture() {
        let mut modal = open();
        modal.capturing = Some("next");
        modal.close();
        assert_eq!(modal.capturing, None);
    }

    #[test]
    fn escape_closes_and_tab_and_shift_tab_move_the_focus() {
        let modal = open();
        assert_eq!(modal.key(Some("escape")), ModalKey::Close);
        assert_eq!(modal.key(Some("tab")), ModalKey::Focus(Step::Next));
        assert_eq!(
            modal.key(Some("shift+tab")),
            ModalKey::Focus(Step::Previous)
        );
    }

    #[test]
    fn culling_keys_are_left_to_the_focused_control_never_mapped_to_an_action() {
        let modal = open();
        for key in [
            "arrowright",
            "p",
            "x",
            "1",
            "ctrl+alt+1",
            "meta+z",
            "space",
            "enter",
        ] {
            assert_eq!(modal.key(Some(key)), ModalKey::Native, "{key}");
        }
        assert_eq!(modal.key(None), ModalKey::Native);
    }

    #[test]
    fn while_capturing_a_key_is_added_and_escape_cancels_instead_of_closing() {
        let mut modal = open();
        modal.capturing = Some("next");
        assert_eq!(
            modal.key(Some("arrowright")),
            ModalKey::Add("arrowright".into())
        );
        assert_eq!(modal.key(Some("tab")), ModalKey::Add("tab".into()));
        assert_eq!(modal.key(Some("escape")), ModalKey::Cancel);
        modal.capturing = None;
        assert_eq!(modal.key(Some("escape")), ModalKey::Close);
    }

    #[test]
    fn while_capturing_a_lone_modifier_waits_for_the_rest_of_the_combination() {
        let mut modal = open();
        modal.capturing = Some("next");
        assert_eq!(modal.key(None), ModalKey::Native);
    }

    #[test]
    fn the_tabs_are_in_the_tauri_order() {
        let labels: Vec<_> = TABS.iter().map(|t| t.label()).collect();
        assert_eq!(
            labels,
            ["Sidecar", "Culling", "Keyboard Shortcuts", "Import"]
        );
    }

    #[test]
    fn arrow_down_moves_to_the_next_tab_and_wraps() {
        assert_eq!(next_tab(Tab::Sidecar, "arrowdown"), Tab::Culling);
        assert_eq!(next_tab(Tab::Shortcuts, "arrowdown"), Tab::Import);
        assert_eq!(next_tab(Tab::Import, "arrowdown"), Tab::Sidecar);
    }

    #[test]
    fn arrow_up_moves_to_the_previous_tab_and_wraps() {
        assert_eq!(next_tab(Tab::Culling, "arrowup"), Tab::Sidecar);
        assert_eq!(next_tab(Tab::Sidecar, "arrowup"), Tab::Import);
    }

    #[test]
    fn home_and_end_move_to_the_first_and_last_tab() {
        assert_eq!(next_tab(Tab::Culling, "home"), Tab::Sidecar);
        assert_eq!(next_tab(Tab::Culling, "end"), Tab::Import);
    }

    #[test]
    fn other_keys_keep_the_current_tab() {
        for key in ["arrowleft", "arrowright", "enter", "tab"] {
            assert_eq!(next_tab(Tab::Culling, key), Tab::Culling, "{key}");
        }
    }

    const BASE: AiState = AiState {
        enabled: false,
        ready: false,
        downloaded: None,
        total: 378_478_689,
        error: None,
    };

    #[test]
    fn the_ai_status_names_the_download_size_while_off_with_no_model() {
        assert_eq!(
            ai_status(&BASE),
            "Off; turning it on downloads the model (378 MB)"
        );
    }

    #[test]
    fn the_ai_status_is_off_once_the_model_is_in_place() {
        let state = AiState {
            ready: true,
            ..BASE
        };
        assert_eq!(ai_status(&state), "Off");
    }

    #[test]
    fn the_ai_status_shows_the_download_progress() {
        let state = AiState {
            enabled: true,
            downloaded: Some(120_500_000),
            ..BASE
        };
        assert_eq!(ai_status(&state), "Downloading the model: 120 / 378 MB");
    }

    #[test]
    fn the_ai_status_is_on_with_the_model_in_place() {
        let state = AiState {
            enabled: true,
            ready: true,
            ..BASE
        };
        assert_eq!(ai_status(&state), "On");
    }

    #[test]
    fn the_ai_status_shows_a_failed_download() {
        let error = "The model download failed: SHA-256 00, expected 0d";
        let state = AiState {
            enabled: true,
            error: Some(error.into()),
            ..BASE
        };
        assert_eq!(ai_status(&state), error);
    }

    #[test]
    fn the_ai_status_says_when_the_model_is_missing_while_on() {
        let state = AiState {
            enabled: true,
            ..BASE
        };
        assert_eq!(ai_status(&state), "The model is not downloaded");
    }
}
