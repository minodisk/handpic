//! The settings modal's pure model: its open state and the shortcut row
//! waiting for a key, the key rule of `modal.ts`'s `SettingsModal`, the
//! pages' order and titles, and the Keyboard Shortcuts page's edits of the
//! keymap.

use gpui_kit::component::ThemeMode;
use handpic_core::{i18n, xmp::LabelNames};
use handpic_engine::{
    import,
    mcp::McpState,
    shortcuts::{Binding, Keymap},
    sidecar::{SidecarFormat, label_names_setting},
};

use crate::{
    context,
    theme::{self, Mode},
};

/// The key that opens the modal, the Tauri app menu's `Settings…`
/// accelerator, which the keymap refuses to bind.
pub const OPEN_KEY: &str = if cfg!(target_os = "macos") {
    "meta+,"
} else {
    "ctrl+,"
};

/// The title bar button's tooltip: the action and the key that opens it.
pub const OPEN_TIP: (&str, &str) = ("Settings", OPEN_KEY);

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

/// The label names block is shown while the format writes XMP.
pub fn writes_xmp(format: SidecarFormat) -> bool {
    format.kinds().contains(&SidecarFormat::Xmp)
}

/// The Tauri paragraph over the label name fields.
pub const LABEL_NAMES_NOTE: &str = "The xmp:Label written for each color. These must match the names \
     in Lightroom's color label set. Either edit them here to match your set (Reset fills in \
     Lightroom's default set for the language chosen below), or change your set to these names in \
     Lightroom Classic under Metadata > Color Label Set > Edit.";

/// The five name fields' colors, in the Tauri order.
pub const LABEL_COLORS: [&str; 5] = ["Red", "Yellow", "Green", "Blue", "Purple"];

/// `names` in `LABEL_COLORS` order, as the fields show them.
pub fn label_texts(names: &LabelNames) -> [&str; 5] {
    [
        &names.red,
        &names.yellow,
        &names.green,
        &names.blue,
        &names.purple,
    ]
}

/// The five fields' texts normalized as the engine stores them: trimmed, a
/// blank one the English default (`sidecar::label_names_setting`).
pub fn label_names_of(texts: [&str; 5]) -> LabelNames {
    let value: serde_json::Map<String, serde_json::Value> = LABEL_COLORS
        .iter()
        .zip(texts)
        .map(|(color, text)| (color.to_lowercase(), text.into()))
        .collect();
    label_names_setting(Some(&value.into()))
}

/// The change a commit asks for, none when the names are those in force.
pub fn label_change(current: &LabelNames, committed: LabelNames) -> Option<LabelNames> {
    (committed != *current).then_some(committed)
}

/// The index into `i18n::presets()` of the preset whose names are `names`,
/// none when no preset's are.
pub fn preset_of(names: &LabelNames) -> Option<usize> {
    i18n::presets()
        .iter()
        .position(|preset| preset.names == *names)
}

/// The Appearance tab's mode choices.
pub const MODES: [(Mode, &str); 3] = [
    (Mode::System, "System"),
    (Mode::Light, "Light"),
    (Mode::Dark, "Dark"),
];

/// The setting a click on `chosen` asks for, none when it is the mode in
/// force.
pub fn mode_change(current: theme::Setting, chosen: Mode) -> Option<theme::Setting> {
    (chosen != current.mode).then_some(theme::Setting {
        mode: chosen,
        ..current
    })
}

/// The setting a choice of `chosen` in the `mode` list asks for, none when
/// it is the theme in force or no theme of that mode.
pub fn theme_change(
    current: theme::Setting,
    mode: ThemeMode,
    chosen: &str,
) -> Option<theme::Setting> {
    let name = theme::names(mode).find(|&name| name == chosen)?;
    (name != current.name(mode)).then_some(if mode.is_dark() {
        theme::Setting {
            dark: name,
            ..current
        }
    } else {
        theme::Setting {
            light: name,
            ..current
        }
    })
}

/// The Tauri paragraph at the top of the Import tab.
pub const IMPORT_NOTE: &str = "Import\u{2026} in the folder tree copies the photos of a folder, \
     such as a memory card, into one folder per capture date under the destination root.";

/// The folder pattern's specifiers, `importsettings.ts`'s
/// `IMPORT_SPECIFIERS`: the specifier, its meaning and its text for 24
/// October 2026.
pub const IMPORT_SPECIFIERS: [(&str, &str, &str); 11] = [
    ("%Y", "Year", "2026"),
    ("%y", "Year, two digits", "26"),
    ("%m", "Month, two digits", "10"),
    ("%d", "Day, two digits", "24"),
    ("%b", "Month, short name", "Oct"),
    ("%B", "Month, full name", "October"),
    ("%a", "Weekday, short name", "Sat"),
    ("%A", "Weekday, full name", "Saturday"),
    ("%j", "Day of the year", "297"),
    ("%F", "Same as %Y-%m-%d", "2026-10-24"),
    ("/", "Folder separator", ""),
];

/// The patterns offered as buttons, the default first.
pub const IMPORT_PATTERN_EXAMPLES: [&str; 4] = ["%Y/%F", "%F", "%Y/%m/%d", "%Y/%m"];

/// Shown under the root while no destination root is set,
/// `importsettings.ts`'s `rootNote`.
pub fn root_note(root: &str) -> Option<&'static str> {
    root.is_empty()
        .then_some("Choose the folder imports are copied into.")
}

/// The write a committed pattern asks for: none when it is the stored one,
/// the refusal's reason when it is invalid (the stored one is kept).
pub fn pattern_change(stored: &str, committed: &str) -> Result<Option<String>, String> {
    import::validate_pattern(committed)?;
    Ok((committed != stored).then(|| committed.to_owned()))
}

/// Whether the box's handler stops a key. With a text field focused, a key
/// the modal does not take goes on, so the platform's text input types its
/// character (a stopped key down is never translated into one on Windows).
pub fn stops(key: &ModalKey, editing: bool) -> bool {
    !(editing && *key == ModalKey::Native)
}

/// The modal's status line while the settings store did not open.
pub const NOT_SAVED: &str = "The settings store did not open, so changes last until the app quits.";

/// The MCP page's server switch, its note and the address's label.
pub const MCP_ENABLED_LABEL: &str = "Let MCP clients connect";
pub const MCP_ENABLED_NOTE: &str = "When on, Handpic serves MCP clients on this computer at the address below, so an AI assistant can follow and assist your culling session. MCP clients cannot change this setting.";
pub const MCP_URL_LABEL: &str = "Server address";

/// The endpoint's URL on `port`.
pub fn mcp_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

/// The port the page shows: the running server's, else `configured`.
pub fn mcp_port(configured: u16, state: Option<&McpState>) -> u16 {
    state
        .filter(|state| state.enabled && state.error.is_none())
        .map_or(configured, |state| state.port)
}

/// The server's state under the switch and its last reported state: off is
/// stopped, on is running, the bind error, or starting until it is known.
pub fn mcp_status(enabled: bool, state: Option<&McpState>) -> String {
    match state {
        _ if !enabled => "The server is stopped.".into(),
        Some(McpState {
            error: Some(error), ..
        }) => format!("The server could not start: {error}"),
        Some(_) => "The server is running.".into(),
        None => "The server is starting\u{2026}".into(),
    }
}

/// The MCP page's switch and its note.
pub const ALLOW_DESTRUCTIVE_LABEL: &str =
    "Let MCP clients run destructive operations without asking";
pub const ALLOW_DESTRUCTIVE_NOTE: &str = "When on, an MCP client can move photos to the Trash, redo a move to the Trash and run the folder operations (rewriting or deleting sidecars, sequencing, importing, renaming) once it has shown you the preview, without the app's dialog. When off, those calls open the app's dialog for you to confirm. Delete Permanently always asks you. MCP clients cannot change this setting.";

/// The app's own keys, which `key_down` takes before the keymap, so a
/// binding to one would never fire. `escape` cannot be captured (it cancels)
/// and `ctrl+,` / `meta+,` the keymap refuses as a menu accelerator.
pub const APP_KEYS: [&str; 3] = ["home", "end", "q"];

/// A Keyboard Shortcuts page edit of the keymap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    Add(&'static str, String),
    Remove(&'static str, String),
    Reset(&'static str),
    ResetAll,
}

/// Applies `edit` to `keymap`, or gives the message of its refusal, leaving
/// `keymap` as it was: the engine's, or one of `APP_KEYS` added.
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

/// The area a shortcut acts on, the Keyboard Shortcuts page's groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutGroup {
    Filmstrip,
    Judgment,
    Preview,
    Panes,
    General,
}

/// The groups in the page's order.
pub const SHORTCUT_GROUPS: [ShortcutGroup; 5] = [
    ShortcutGroup::Filmstrip,
    ShortcutGroup::Judgment,
    ShortcutGroup::Preview,
    ShortcutGroup::Panes,
    ShortcutGroup::General,
];

impl ShortcutGroup {
    pub fn title(self) -> &'static str {
        match self {
            ShortcutGroup::Filmstrip => "Filmstrip",
            ShortcutGroup::Judgment => "Judgment",
            ShortcutGroup::Preview => "Preview",
            ShortcutGroup::Panes => "Panes",
            ShortcutGroup::General => "General",
        }
    }
}

/// The group of an action's row, `General` for an action it does not know
/// so that no row leaves the page.
pub fn shortcut_group(action: &str) -> ShortcutGroup {
    known_group(action).unwrap_or(ShortcutGroup::General)
}

/// The rows of `group` in the keymap's order, the Judgment group's in the
/// strip menu's order instead, its actions the menu does not hold after them.
pub fn shortcut_rows(bindings: &[Binding], group: ShortcutGroup) -> Vec<&Binding> {
    let mut rows: Vec<&Binding> = bindings
        .iter()
        .filter(|b| shortcut_group(b.action) == group)
        .collect();
    if group == ShortcutGroup::Judgment {
        rows.sort_by_key(|b| {
            context::judgment_actions()
                .position(|action| action == b.action)
                .unwrap_or(usize::MAX)
        });
    }
    rows
}

fn known_group(action: &str) -> Option<ShortcutGroup> {
    Some(match action {
        "previous" | "next" | "burstPrevious" | "burstNext" | "burstFramePrevious"
        | "burstFrameNext" | "extendPrevious" | "extendNext" | "selectAll" => {
            ShortcutGroup::Filmstrip
        }
        "rate1" | "rate2" | "rate3" | "rate4" | "rate5" | "reject" | "rejectRest" | "pick"
        | "unflag" | "clear" | "red" | "orange" | "yellow" | "green" | "blue" | "pink"
        | "purple" | "clearlabel" | "clearall" => ShortcutGroup::Judgment,
        "focus" | "zoom" | "grayscale" | "compare" | "performance" | "peaking" => {
            ShortcutGroup::Preview
        }
        "toggleLeft" | "toggleRight" | "toggleStrip" | "toggleSides" => ShortcutGroup::Panes,
        "open" | "undo" | "redo" => ShortcutGroup::General,
        _ => return None,
    })
}

/// A key as the keymap spells it in running text, such as the empty state's
/// "Or press …" sentence, `keys.ts`'s `displayKey`. The Keyboard Shortcuts
/// chips use `Kbd::format` instead.
pub fn display_key(key: &str) -> &str {
    if key == "space" { "Space" } else { key }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Sidecar,
    Culling,
    Shortcuts,
    Cache,
    Import,
    Mcp,
    Appearance,
    Updates,
}

/// The pages in the Tauri order, less the Debug one, then the app's
/// own Appearance and Updates.
pub const TABS: [Tab; 8] = [
    Tab::Sidecar,
    Tab::Culling,
    Tab::Shortcuts,
    Tab::Cache,
    Tab::Import,
    Tab::Mcp,
    Tab::Appearance,
    Tab::Updates,
];

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Tab::Sidecar => "Sidecar",
            Tab::Culling => "Culling",
            Tab::Shortcuts => "Keyboard Shortcuts",
            Tab::Cache => "Cache",
            Tab::Import => "Import",
            Tab::Mcp => "MCP",
            Tab::Appearance => "Appearance",
            Tab::Updates => "Updates",
        }
    }
}

/// What a key does while the modal is open. None of them reaches the
/// culling keymap. Tab and Shift+Tab are not among them: `Root` walks the
/// dialog's focus trap with them before the box sees a key, unless a row
/// captures.
#[derive(Debug, PartialEq, Eq)]
pub enum ModalKey {
    Add(String),
    Cancel,
    Close,
    /// Left to the focused control.
    Native,
}

#[derive(Default)]
pub struct SettingsModal {
    opened: bool,
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
    /// otherwise Escape closes.
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
            _ => ModalKey::Native,
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::ThemeMode;
    use handpic_core::xmp::LabelNames;
    use handpic_engine::{mcp::McpState, shortcuts::Keymap, sidecar::SidecarFormat};

    use super::{
        APP_KEYS, Edit, FORMATS, IMPORT_PATTERN_EXAMPLES, IMPORT_SPECIFIERS, MODES, ModalKey,
        OPEN_TIP, SHORTCUT_GROUPS, SettingsModal, ShortcutGroup, TABS, display_key, edit,
        format_change, known_group, label_change, label_names_of, label_texts, mcp_port,
        mcp_status, mcp_url, mode_change, pattern_change, preset_of, root_note, shortcut_group,
        shortcut_label, shortcut_rows, stops, theme_change, writes_xmp,
    };
    use crate::theme::{Mode, NEUTRAL, Setting};

    #[test]
    fn the_open_tip_names_the_settings_and_their_key() {
        let key = if cfg!(target_os = "macos") {
            "meta+,"
        } else {
            "ctrl+,"
        };
        assert_eq!(OPEN_TIP, ("Settings", key));
    }

    #[test]
    fn the_specifiers_are_the_date_ones_and_the_folder_separator() {
        let specs: Vec<&str> = IMPORT_SPECIFIERS.iter().map(|&(spec, _, _)| spec).collect();
        assert_eq!(
            specs,
            [
                "%Y", "%y", "%m", "%d", "%b", "%B", "%a", "%A", "%j", "%F", "/"
            ]
        );
    }

    #[test]
    fn the_example_patterns_start_with_the_default_and_are_valid() {
        assert_eq!(
            IMPORT_PATTERN_EXAMPLES,
            ["%Y/%F", "%F", "%Y/%m/%d", "%Y/%m"]
        );
        assert_eq!(
            IMPORT_PATTERN_EXAMPLES[0],
            handpic_engine::import::DEFAULT_PATTERN
        );
        for pattern in IMPORT_PATTERN_EXAMPLES {
            assert!(handpic_engine::import::validate_pattern(pattern).is_ok());
        }
    }

    #[test]
    fn the_root_note_asks_for_a_root_only_while_none_is_set() {
        assert_eq!(
            root_note(""),
            Some("Choose the folder imports are copied into.")
        );
        assert_eq!(root_note("D:\\Photos"), None);
    }

    #[test]
    fn a_committed_pattern_is_written_when_valid_and_new() {
        assert_eq!(pattern_change("%Y/%F", "%Y/%m"), Ok(Some("%Y/%m".into())));
        assert_eq!(pattern_change("%Y/%F", "%Y/%F"), Ok(None));
        assert!(pattern_change("%Y/%F", "%H").is_err());
        assert!(pattern_change("%Y/%F", "%Y//%d").is_err());
    }

    #[test]
    fn the_label_names_show_while_the_format_writes_xmp() {
        assert!(writes_xmp(SidecarFormat::Xmp));
        assert!(writes_xmp(SidecarFormat::Both));
        assert!(!writes_xmp(SidecarFormat::Dop));
    }

    #[test]
    fn committed_names_are_trimmed_and_a_blank_one_is_the_english_default() {
        let names = label_names_of([" Rot ", "", "Grün", "   ", "Lila"]);
        assert_eq!(
            label_texts(&names),
            ["Rot", "Yellow", "Grün", "Blue", "Lila"]
        );
    }

    #[test]
    fn a_commit_of_the_names_in_force_changes_nothing() {
        let current = LabelNames::default();
        assert_eq!(
            label_change(
                &current,
                label_names_of([" Red", "Yellow", "Green", "", "Purple"])
            ),
            None
        );
        let rot = label_names_of(["Rot", "Yellow", "Green", "Blue", "Purple"]);
        assert_eq!(label_change(&current, rot.clone()), Some(rot));
    }

    #[test]
    fn a_preset_fills_the_fields_in_the_tauri_order() {
        let ja = handpic_core::i18n::preset("ja").unwrap();
        assert_eq!(
            label_texts(&ja.names),
            ["レッド", "イエロー", "グリーン", "ブルー", "パープル"]
        );
        assert_eq!(handpic_core::i18n::presets()[0].code, "en");
    }

    #[test]
    fn the_language_shown_is_the_preset_whose_names_are_in_force() {
        let presets = handpic_core::i18n::presets();
        for (at, preset) in presets.iter().enumerate() {
            assert_eq!(preset_of(&preset.names), Some(at));
        }
        assert_eq!(preset_of(&LabelNames::default()), Some(0));
        for preset in presets {
            let texts = label_texts(&preset.names);
            let edited = label_names_of([texts[0], texts[1], texts[2], texts[3], "Custom"]);
            assert_eq!(preset_of(&edited), None);
        }
    }

    #[test]
    fn a_focused_text_field_keeps_the_keys_the_modal_does_not_take() {
        assert!(!stops(&ModalKey::Native, true));
        assert!(stops(&ModalKey::Native, false));
        assert!(stops(&ModalKey::Add("tab".into()), true));
        assert!(stops(&ModalKey::Close, true));
    }

    fn keys_of(keymap: &Keymap, action: &str) -> Vec<String> {
        keymap
            .bindings()
            .into_iter()
            .find(|b| b.action == action)
            .unwrap()
            .keys
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
    fn every_action_has_its_own_group() {
        for binding in Keymap::defaults().bindings() {
            assert!(known_group(binding.action).is_some(), "{}", binding.action);
        }
        assert_eq!(shortcut_group("pick"), ShortcutGroup::Judgment);
        assert_eq!(shortcut_group("somethingNew"), ShortcutGroup::General);
    }

    #[test]
    fn the_judgment_rows_follow_the_strip_menu_then_the_keymap() {
        let bindings = Keymap::defaults().bindings();
        let rows: Vec<&str> = shortcut_rows(&bindings, ShortcutGroup::Judgment)
            .iter()
            .map(|b| b.action)
            .collect();
        let mut expected: Vec<&str> = crate::context::judgment_actions().collect();
        expected.extend(["rejectRest", "clearall"]);
        assert_eq!(rows, expected);
        assert_eq!(
            &rows[..9],
            [
                "pick", "unflag", "reject", "rate5", "rate4", "rate3", "rate2", "rate1", "clear"
            ]
        );
    }

    #[test]
    fn the_other_rows_keep_the_keymap_order() {
        let bindings = Keymap::defaults().bindings();
        for group in SHORTCUT_GROUPS {
            if group == ShortcutGroup::Judgment {
                continue;
            }
            let expected: Vec<&str> = bindings
                .iter()
                .filter(|b| shortcut_group(b.action) == group)
                .map(|b| b.action)
                .collect();
            let rows: Vec<&str> = shortcut_rows(&bindings, group)
                .iter()
                .map(|b| b.action)
                .collect();
            assert_eq!(rows, expected, "{group:?}");
        }
        let total: usize = SHORTCUT_GROUPS
            .iter()
            .map(|&group| shortcut_rows(&bindings, group).len())
            .sum();
        assert_eq!(total, bindings.len());
    }

    #[test]
    fn every_group_holds_an_action() {
        let bindings = Keymap::defaults().bindings();
        for group in SHORTCUT_GROUPS {
            assert!(
                bindings.iter().any(|b| shortcut_group(b.action) == group),
                "{group:?}"
            );
        }
    }

    #[test]
    fn the_groups_are_in_the_page_order() {
        let titles: Vec<_> = SHORTCUT_GROUPS.iter().map(|g| g.title()).collect();
        assert_eq!(
            titles,
            ["Filmstrip", "Judgment", "Preview", "Panes", "General"]
        );
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
    fn opens_once_and_opening_again_while_open_is_a_no_op() {
        let mut modal = SettingsModal::default();
        assert!(modal.open());
        assert!(modal.is_open());
        assert!(!modal.open());
        assert!(modal.is_open());
    }

    #[test]
    fn closing_cancels_a_capture() {
        let mut modal = open();
        modal.capturing = Some("next");
        modal.close();
        assert_eq!(modal.capturing, None);
    }

    #[test]
    fn escape_closes_and_tab_and_shift_tab_are_left_to_the_dialog() {
        let modal = open();
        assert_eq!(modal.key(Some("escape")), ModalKey::Close);
        assert_eq!(modal.key(Some("tab")), ModalKey::Native);
        assert_eq!(modal.key(Some("shift+tab")), ModalKey::Native);
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
        assert_eq!(
            modal.key(Some("shift+tab")),
            ModalKey::Add("shift+tab".into())
        );
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
            [
                "Sidecar",
                "Culling",
                "Keyboard Shortcuts",
                "Cache",
                "Import",
                "MCP",
                "Appearance",
                "Updates"
            ]
        );
    }

    #[test]
    fn the_modes_are_system_light_and_dark() {
        let labels: Vec<_> = MODES.iter().map(|(_, label)| *label).collect();
        assert_eq!(labels, ["System", "Light", "Dark"]);
    }

    #[test]
    fn choosing_the_mode_in_force_changes_nothing_and_another_is_set() {
        for (mode, _) in MODES {
            let current = Setting {
                mode,
                ..Setting::default()
            };
            assert_eq!(mode_change(current, mode), None);
        }
        let current = Setting {
            mode: Mode::Dark,
            light: "Ayu Light",
            dark: "Ayu Dark",
        };
        assert_eq!(
            mode_change(current, Mode::System),
            Some(Setting {
                mode: Mode::System,
                ..current
            })
        );
    }

    #[test]
    fn choosing_a_theme_sets_its_mode_only() {
        let current = Setting::default();
        assert_eq!(theme_change(current, ThemeMode::Light, NEUTRAL), None);
        assert_eq!(theme_change(current, ThemeMode::Dark, NEUTRAL), None);
        assert_eq!(
            theme_change(current, ThemeMode::Light, "Ayu Light"),
            Some(Setting {
                light: "Ayu Light",
                ..current
            })
        );
        let dark = theme_change(current, ThemeMode::Dark, "Tokyo Night").unwrap();
        assert_eq!(
            dark,
            Setting {
                dark: "Tokyo Night",
                ..current
            }
        );
        assert_eq!(theme_change(dark, ThemeMode::Dark, "Tokyo Night"), None);
        assert_eq!(theme_change(dark, ThemeMode::Dark, NEUTRAL), Some(current));
        assert_eq!(theme_change(current, ThemeMode::Dark, "Ayu Light"), None);
        assert_eq!(theme_change(current, ThemeMode::Light, "Sepia"), None);
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

    #[test]
    fn the_mcp_page_shows_the_running_port_and_the_switchs_state() {
        let running = McpState {
            enabled: true,
            port: 50123,
            error: None,
        };
        let failed = McpState {
            enabled: true,
            port: 41917,
            error: Some("could not listen on 127.0.0.1:41917: in use".into()),
        };
        let stopped = McpState {
            enabled: false,
            port: 41917,
            error: None,
        };
        assert_eq!(mcp_url(41917), "http://127.0.0.1:41917/mcp");
        assert_eq!(mcp_port(0, Some(&running)), 50123);
        assert_eq!(mcp_port(41917, Some(&failed)), 41917);
        assert_eq!(mcp_port(41999, Some(&stopped)), 41999);
        assert_eq!(mcp_port(41999, None), 41999);
        assert_eq!(mcp_status(false, None), "The server is stopped.");
        assert_eq!(mcp_status(false, Some(&running)), "The server is stopped.");
        assert_eq!(mcp_status(true, None), "The server is starting\u{2026}");
        assert_eq!(mcp_status(true, Some(&running)), "The server is running.");
        assert_eq!(
            mcp_status(true, Some(&failed)),
            "The server could not start: could not listen on 127.0.0.1:41917: in use"
        );
    }
}
