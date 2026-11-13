//! The settings modal's pure model: its open state, the selected tab and the
//! shortcut row waiting for a key, the key rule of `modal.ts`'s
//! `SettingsModal` and the vertical tablist of `tabs.ts`'s `nextTab`, and the
//! Keyboard Shortcuts tab's edits of the keymap.

use riffle_backend::{shortcuts::Keymap, sidecar::SidecarFormat};

/// The key that opens the modal, the Tauri app menu's `Settings…`
/// accelerator, which the keymap refuses to bind.
pub const OPEN_KEY: &str = if cfg!(target_os = "macos") {
    "meta+,"
} else {
    "ctrl+,"
};

/// The Sidecar tab's choices, in the Tauri order.
pub const FORMATS: [(SidecarFormat, &str); 3] = [
    (SidecarFormat::Xmp, "Lightroom (.xmp)"),
    (SidecarFormat::Dop, "PhotoLab (.dop)"),
    (SidecarFormat::Both, "Both (.xmp and .dop)"),
];

/// The switch a click on `chosen` asks for, none when it is already the
/// format in force.
pub fn format_change(current: SidecarFormat, chosen: SidecarFormat) -> Option<SidecarFormat> {
    (chosen != current).then_some(chosen)
}

/// The modal's status line while the settings store did not open.
pub const NOT_SAVED: &str = "The settings store did not open, so changes last until the app quits.";

/// The native app's own keys, which `key_down` takes before the keymap, so a
/// binding to one would never fire. `escape` cannot be captured (it cancels)
/// and `ctrl+,` / `meta+,` the keymap refuses as a menu accelerator.
pub const APP_KEYS: [&str; 3] = ["home", "end", "q"];

/// A Keyboard Shortcuts tab edit of the keymap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    Add(&'static str, String),
    Remove(&'static str, String),
    Reset(&'static str),
    ResetAll,
}

/// Applies `edit` to `keymap`, or gives the message of its refusal, leaving
/// `keymap` as it was: the backend's, or one of `APP_KEYS` added.
pub fn edit(keymap: &mut Keymap, edit: Edit) -> Result<(), String> {
    match edit {
        Edit::Add(_, key) if APP_KEYS.contains(&key.as_str()) => {
            Err(format!("{key:?} is taken by this app"))
        }
        Edit::Add(action, key) => keymap.add(action, &key),
        Edit::Remove(action, key) => keymap.remove(action, &key),
        Edit::Reset(action) => keymap.reset(action),
        Edit::ResetAll => {
            keymap.reset_all();
            Ok(())
        }
    }
}

/// A shortcut row's label, `settings.ts`'s `shortcutLabels`, else the
/// action's own name.
pub fn shortcut_label(action: &'static str) -> &'static str {
    match action {
        "previous" => "Previous",
        "next" => "Next",
        "burstPrevious" => "Previous burst",
        "burstNext" => "Next burst",
        "burstFramePrevious" => "Previous frame in burst",
        "burstFrameNext" => "Next frame in burst",
        "extendPrevious" => "Extend selection left",
        "extendNext" => "Extend selection right",
        "selectAll" => "Select all",
        "open" => "Open folder",
        "undo" => "Undo",
        "redo" => "Redo",
        "toggleLeft" => "Show / hide the folder tree",
        "toggleRight" => "Show / hide the metadata pane",
        "toggleStrip" => "Show / hide the filmstrip",
        "toggleSides" => "Show / hide both side panes",
        "focus" => "Focus mark",
        "zoom" => "1:1 zoom",
        "grayscale" => "Grayscale",
        "compare" => "Compare frames",
        "performance" => "Performance graph",
        "peaking" => "Focus peaking",
        "rate1" => "1 star",
        "rate2" => "2 stars",
        "rate3" => "3 stars",
        "rate4" => "4 stars",
        "rate5" => "5 stars",
        "reject" => "Reject",
        "rejectRest" => "Reject the rest of the burst",
        "pick" => "Pick",
        "unflag" => "Un-reject / un-pick",
        "clear" => "0 star",
        "red" => "Red label",
        "orange" => "Orange label",
        "yellow" => "Yellow label",
        "green" => "Green label",
        "blue" => "Blue label",
        "pink" => "Pink label",
        "purple" => "Purple label",
        "clearlabel" => "Clear label",
        "clearall" => "Clear",
        other => other,
    }
}

/// A key as its chip shows it, `keys.ts`'s `displayKey`.
pub fn display_key(key: &str) -> &str {
    if key == "space" { "Space" } else { key }
}

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

    /// Switching tabs cancels a capture, as `selectTab` does.
    pub fn select(&mut self, tab: Tab) {
        self.tab = tab;
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

#[cfg(test)]
mod tests {
    use riffle_backend::{shortcuts::Keymap, sidecar::SidecarFormat};

    use super::{
        APP_KEYS, Edit, FORMATS, ModalKey, SettingsModal, Step, TABS, Tab, display_key, edit,
        format_change, next_tab, shortcut_label,
    };

    fn keys_of(keymap: &Keymap, action: &str) -> Vec<String> {
        keymap
            .bindings()
            .into_iter()
            .find(|b| b.action == action)
            .unwrap()
            .keys
    }

    #[test]
    fn switching_tabs_cancels_a_capture() {
        let mut modal = open();
        modal.capturing = Some("pick");
        modal.select(Tab::Culling);
        assert_eq!(modal.tab, Tab::Culling);
        assert_eq!(modal.capturing, None);
    }

    #[test]
    fn a_captured_key_is_added_to_the_capturing_action() {
        let mut modal = open();
        modal.capturing = Some("pick");
        let ModalKey::Add(key) = modal.key(Some("k")) else {
            panic!("not added");
        };
        let action = modal.capturing.take().unwrap();
        let mut keymap = Keymap::defaults();
        assert_eq!(edit(&mut keymap, Edit::Add(action, key)), Ok(()));
        assert_eq!(keys_of(&keymap, "pick"), ["p", "k"]);
        assert_eq!(modal.key(Some("escape")), ModalKey::Close);
    }

    #[test]
    fn a_key_bound_to_another_action_is_refused_with_the_backend_message() {
        let mut keymap = Keymap::defaults();
        assert_eq!(
            edit(&mut keymap, Edit::Add("pick", "x".into())),
            Err("\"x\" is bound to reject".to_owned())
        );
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn the_app_s_own_keys_are_refused() {
        let mut keymap = Keymap::defaults();
        for key in APP_KEYS {
            assert_eq!(
                edit(&mut keymap, Edit::Add("pick", key.into())),
                Err(format!("{key:?} is taken by this app")),
                "{key}"
            );
        }
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn a_menu_accelerator_and_the_last_key_are_refused() {
        let mut keymap = Keymap::defaults();
        let open_key = if cfg!(target_os = "macos") {
            "meta+,"
        } else {
            "ctrl+,"
        };
        assert_eq!(
            edit(&mut keymap, Edit::Add("pick", open_key.into())),
            Err(format!("{open_key:?} is a menu accelerator"))
        );
        assert_eq!(
            edit(&mut keymap, Edit::Remove("pick", "p".into())),
            Err("\"p\" is the only key for pick; use Reset".to_owned())
        );
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn remove_reset_and_reset_all_edit_the_keymap() {
        let mut keymap = Keymap::defaults();
        edit(&mut keymap, Edit::Add("pick", "k".into())).unwrap();
        edit(&mut keymap, Edit::Add("reject", "j".into())).unwrap();
        edit(&mut keymap, Edit::Remove("pick", "p".into())).unwrap();
        assert_eq!(keys_of(&keymap, "pick"), ["k"]);
        edit(&mut keymap, Edit::Reset("pick")).unwrap();
        assert_eq!(keys_of(&keymap, "pick"), ["p"]);
        assert_eq!(keys_of(&keymap, "reject"), ["x", "j"]);
        edit(&mut keymap, Edit::ResetAll).unwrap();
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn every_action_has_its_tauri_label() {
        for binding in Keymap::defaults().bindings() {
            assert_ne!(
                shortcut_label(binding.action),
                binding.action,
                "{}",
                binding.action
            );
        }
        assert_eq!(shortcut_label("pick"), "Pick");
        assert_eq!(shortcut_label("clear"), "0 star");
        assert_eq!(shortcut_label("somethingNew"), "somethingNew");
    }

    #[test]
    fn the_space_bar_is_shown_as_space() {
        assert_eq!(display_key("space"), "Space");
        assert_eq!(display_key("shift+x"), "shift+x");
    }

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

    #[test]
    fn the_formats_are_the_tauri_radios_in_order() {
        let labels: Vec<_> = FORMATS.iter().map(|(_, label)| *label).collect();
        assert_eq!(
            labels,
            [
                "Lightroom (.xmp)",
                "PhotoLab (.dop)",
                "Both (.xmp and .dop)"
            ]
        );
    }

    #[test]
    fn choosing_the_format_in_force_switches_nothing() {
        for (format, _) in FORMATS {
            assert_eq!(format_change(format, format), None);
        }
    }

    #[test]
    fn choosing_another_format_switches_to_it() {
        assert_eq!(
            format_change(SidecarFormat::Xmp, SidecarFormat::Dop),
            Some(SidecarFormat::Dop)
        );
        assert_eq!(
            format_change(SidecarFormat::Both, SidecarFormat::Xmp),
            Some(SidecarFormat::Xmp)
        );
    }
}
