//! The culling keymap: each action's default keys and the user's overrides
//! from the `shortcuts` settings key, merged so no key is bound twice: an
//! override loses only the keys another action already holds.

use serde::Serialize;
use serde_json::{Map, Value};

const MACOS: bool = cfg!(target_os = "macos");

/// The default key of `open`, the accelerator of `File > Open Folder…`.
const OPEN_DEFAULT: &str = if MACOS { "meta+o" } else { "ctrl+o" };

/// The default key of `undo`, the accelerator of `Edit > Undo`.
const UNDO_DEFAULT: &str = if MACOS { "meta+z" } else { "ctrl+z" };

/// The default key of `selectAll`, the accelerator of `Edit > Select All`.
const SELECT_ALL_DEFAULT: &str = if MACOS { "meta+a" } else { "ctrl+a" };

/// The default key of `redo`, the accelerator of `Edit > Redo`.
const REDO_DEFAULT: &str = if MACOS {
    "shift+meta+z"
} else {
    "ctrl+shift+z"
};

/// The default key of `toggleLeft`, the accelerator of `View > Folders`.
const TOGGLE_LEFT_DEFAULT: &str = if MACOS {
    "alt+meta+arrowleft"
} else {
    "ctrl+alt+arrowleft"
};

/// The default key of `toggleRight`, the accelerator of `View > Metadata`.
const TOGGLE_RIGHT_DEFAULT: &str = if MACOS {
    "alt+meta+arrowright"
} else {
    "ctrl+alt+arrowright"
};

/// The default key of `toggleStrip`, the accelerator of `View > Filmstrip`.
const TOGGLE_STRIP_DEFAULT: &str = if MACOS {
    "alt+meta+arrowdown"
} else {
    "ctrl+alt+arrowdown"
};

/// Every action in the order the shortcuts panel shows them, with its
/// default keys. A plain key is `event.key` lower-cased, with `" "` as
/// `"space"`. With a modifier held, the name is `ctrl+alt+shift+meta+` (only
/// the modifiers held, in that order) and the key named from `event.code`
/// (`Digit1` -> `1`, `Comma` -> `,`), as Option and Shift change
/// `event.key`.
const DEFAULTS: &[(&str, &[&str])] = &[
    ("previous", &["arrowleft"]),
    ("next", &["arrowright"]),
    ("burstPrevious", &["arrowup"]),
    ("burstNext", &["arrowdown"]),
    ("burstFramePrevious", &["alt+arrowleft"]),
    ("burstFrameNext", &["alt+arrowright"]),
    ("extendPrevious", &["shift+arrowleft"]),
    ("extendNext", &["shift+arrowright"]),
    ("selectAll", &[SELECT_ALL_DEFAULT]),
    ("open", &[OPEN_DEFAULT]),
    ("undo", &[UNDO_DEFAULT]),
    ("redo", &[REDO_DEFAULT]),
    ("toggleLeft", &[TOGGLE_LEFT_DEFAULT]),
    ("toggleRight", &[TOGGLE_RIGHT_DEFAULT]),
    ("toggleStrip", &[TOGGLE_STRIP_DEFAULT]),
    ("toggleSides", &["tab"]),
    ("focus", &["f"]),
    ("zoom", &["z"]),
    ("grayscale", &["g"]),
    ("compare", &["v"]),
    ("performance", &["d"]),
    ("peaking", &["e"]),
    ("rate1", &["1"]),
    ("rate2", &["2"]),
    ("rate3", &["3"]),
    ("rate4", &["4"]),
    ("rate5", &["5"]),
    ("reject", &["x"]),
    ("rejectRest", &["shift+x"]),
    ("pick", &["p"]),
    ("unflag", &["u"]),
    ("clear", &["0"]),
    ("red", &["ctrl+alt+1"]),
    ("orange", &["ctrl+alt+2"]),
    ("yellow", &["ctrl+alt+3"]),
    ("green", &["ctrl+alt+4"]),
    ("blue", &["ctrl+alt+5"]),
    ("pink", &["ctrl+alt+6"]),
    ("purple", &["ctrl+alt+7"]),
    ("clearlabel", &["ctrl+alt+0"]),
    ("clearall", &["c"]),
];

/// macOS combinations owned by the app's menu (`app_menu::build` on top of
/// `Menu::default`). The `Open Folder…`, `Undo`, `Redo` and `Select All`
/// accelerators are deliberately absent: each is derived from its own
/// action's keys, so it can never collide with another action, and once the
/// action moves off a combination that combination is free again.
const MACOS_MENU: &[&str] = &[
    "meta+,",
    "meta+q",
    "meta+h",
    "alt+meta+h",
    "meta+w",
    "meta+m",
    "ctrl+meta+f",
    "meta+x",
    "meta+c",
    "meta+v",
];

/// macOS combinations owned by the OS: app switcher, Spotlight, input
/// sources, window cycling, Force Quit, lock and log out, screenshots, Dock
/// and Mission Control.
const MACOS_SYSTEM: &[&str] = &[
    "meta+tab",
    "shift+meta+tab",
    "meta+space",
    "alt+meta+space",
    "ctrl+space",
    "meta+`",
    "shift+meta+`",
    "alt+meta+escape",
    "ctrl+meta+q",
    "shift+meta+q",
    "shift+meta+3",
    "shift+meta+4",
    "shift+meta+5",
    "alt+meta+d",
    "ctrl+meta+space",
    "ctrl+arrowleft",
    "ctrl+arrowright",
    "ctrl+arrowup",
    "ctrl+arrowdown",
];

/// Windows / Linux combinations owned by the app's menu; the keymap-derived
/// accelerators are absent, as in `MACOS_MENU`.
const OTHER_MENU: &[&str] = &["ctrl+,", "ctrl+x", "ctrl+c", "ctrl+v", "ctrl+m", "alt+f4"];

/// Windows / Linux combinations owned by the OS; every `meta+` name is too,
/// as the shell owns the Windows / Super key.
const OTHER_SYSTEM: &[&str] = &[
    "alt+tab",
    "shift+alt+tab",
    "ctrl+escape",
    "ctrl+shift+escape",
];

/// Lone modifiers, which never name a key. An earlier bug let them into the
/// stored `shortcuts`, so they are dropped when it is read. Mirrors
/// `MODIFIER_KEYS` in `crates/app/ui/src/keys.ts`; the two must stay in sync.
const MODIFIER_ONLY: [&str; 4] = ["control", "alt", "shift", "meta"];

/// Why `key` cannot be bound on macOS or elsewhere, if it cannot.
fn forbidden(key: &str, macos: bool) -> Option<&'static str> {
    let (menu, system) = if macos {
        (MACOS_MENU, MACOS_SYSTEM)
    } else {
        (OTHER_MENU, OTHER_SYSTEM)
    };
    if menu.contains(&key) {
        Some("is a menu accelerator")
    } else if system.contains(&key) || (!macos && key.contains("meta+")) {
        Some("is reserved by the system")
    } else {
        None
    }
}

/// The muda accelerator string for a Handpic key name, or `None` when muda
/// has no accelerator for it. A key held with no `ctrl`, `alt` or `meta` is
/// `None` too: a modifier-less key equivalent would fire on every such key
/// typed anywhere, text fields included, and double with the webview.
pub fn accelerator(key: &str) -> Option<String> {
    let mut parts = key.split('+').peekable();
    let mut modifiers = Vec::new();
    let mut plain = true;
    for name in ["ctrl", "alt", "shift", "meta"] {
        if parts.peek() == Some(&name) {
            parts.next();
            modifiers.push(match name {
                "ctrl" => "Ctrl",
                "alt" => "Alt",
                "shift" => "Shift",
                _ => "Cmd",
            });
            plain &= name == "shift";
        }
    }
    let key = parts.next()?;
    if parts.next().is_some() || plain {
        return None;
    }
    let key = match key {
        "space" => "Space".to_string(),
        "escape" => "Escape".to_string(),
        "enter" => "Enter".to_string(),
        "tab" => "Tab".to_string(),
        "backspace" => "Backspace".to_string(),
        "delete" => "Delete".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "pageup" => "PageUp".to_string(),
        "pagedown" => "PageDown".to_string(),
        "insert" => "Insert".to_string(),
        "arrowup" => "ArrowUp".to_string(),
        "arrowdown" => "ArrowDown".to_string(),
        "arrowleft" => "ArrowLeft".to_string(),
        "arrowright" => "ArrowRight".to_string(),
        "-" | "=" | "," | "." | "/" | ";" | "'" | "[" | "]" | "\\" | "`" => key.to_string(),
        _ => {
            let mut chars = key.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                match key.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
                    Some(n) if (1..=24).contains(&n) => format!("F{n}"),
                    _ => return None,
                }
            } else if c.is_ascii_alphabetic() {
                c.to_ascii_uppercase().to_string()
            } else if c.is_ascii_digit() {
                c.to_string()
            } else {
                return None;
            }
        }
    };
    modifiers.push(&key);
    Some(modifiers.join("+"))
}

/// One action and the keys bound to it, as the `shortcuts` command returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Binding {
    pub action: &'static str,
    pub keys: Vec<String>,
}

/// The resolved keymap, one entry per action in `DEFAULTS` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    bindings: Vec<Binding>,
    /// Stored overrides skipped for a key conflict, kept so saving does not
    /// drop them.
    inactive: Map<String, Value>,
}

impl Keymap {
    /// The default keys.
    pub fn defaults() -> Keymap {
        Keymap {
            bindings: DEFAULTS
                .iter()
                .map(|&(action, keys)| Binding {
                    action,
                    keys: keys.iter().map(|k| k.to_string()).collect(),
                })
                .collect(),
            inactive: Map::new(),
        }
    }

    /// Merge the stored `shortcuts` value over the defaults. Every entry that
    /// cannot be used is logged and the action keeps its default. A key
    /// already bound to another action is dropped from the override (logged)
    /// and the rest apply, so the earlier action keeps that key and
    /// `overrides` stores only the rest. An override whose every key is bound
    /// elsewhere is skipped and kept in the stored value (`overrides`), minus
    /// any modifier-only keys.
    pub fn from_overrides(overrides: Option<&Value>) -> Keymap {
        let mut keymap = Keymap::defaults();
        let Some(overrides) = overrides else {
            return keymap;
        };
        let Some(overrides) = overrides.as_object() else {
            log::warn!("ignoring the shortcuts setting: not an object");
            return keymap;
        };
        for name in overrides.keys() {
            if !keymap.bindings.iter().any(|b| b.action == name) {
                log::warn!("ignoring the shortcut for an unknown action {name:?}");
            }
        }
        let mut pending = Vec::new();
        for i in 0..keymap.bindings.len() {
            let action = keymap.bindings[i].action;
            let Some(value) = overrides.get(action) else {
                continue;
            };
            let Some(keys) = parse_keys(value) else {
                log::warn!("ignoring the shortcut for {action}: not a non-empty list of keys");
                continue;
            };
            if let Some((key, reason)) = keys
                .iter()
                .find_map(|k| forbidden(k, MACOS).map(|r| (k, r)))
            {
                log::warn!("ignoring the shortcut for {action}: {key:?} {reason}");
                continue;
            }
            pending.push((i, keys));
        }
        // An override may need a key that a later override releases (`p` on
        // reject once pick moves off it), so retry until nothing applies.
        // Only then does the first override with some free keys apply in
        // part, dropping the keys another action holds. That replaces the
        // action's current keys and so may free one, hence the outer retry.
        // A partly applied action's requested keys are kept in `partial` so
        // that, once the outer loop settles, any key freed by a later
        // partial application on another action can still be recovered.
        let mut partial: Vec<(usize, Vec<String>)> = Vec::new();
        loop {
            loop {
                let before = pending.len();
                pending.retain(|(i, keys)| {
                    let free = keys.iter().all(|key| keymap.holder(*i, key).is_none());
                    if free {
                        keymap.bindings[*i].keys = keys.clone();
                    }
                    !free
                });
                if pending.len() == before {
                    break;
                }
            }
            let Some(p) = pending
                .iter()
                .position(|(i, keys)| keys.iter().any(|key| keymap.holder(*i, key).is_none()))
            else {
                break;
            };
            let (i, keys) = pending.remove(p);
            let action = keymap.bindings[i].action;
            let mut rest = Vec::new();
            for key in &keys {
                match keymap.holder(i, key) {
                    Some(other) => log::warn!(
                        "dropping {key:?} from the shortcut for {action}: it is bound to {other}"
                    ),
                    None => rest.push(key.clone()),
                }
            }
            keymap.bindings[i].keys = rest;
            partial.push((i, keys));
        }
        // Recompute each partly applied action's keys as its requested keys
        // filtered by what is free now that the outer loop has settled. This
        // only ever adds keys that are free at this point, so it cannot
        // create a conflict, and it recovers a key freed by a later partial
        // application (e.g. `arrowup` freed by `burstPrevious`'s partial
        // application after `previous`'s own partial application dropped it).
        for (i, keys) in &partial {
            keymap.bindings[*i].keys = keys
                .iter()
                .filter(|key| keymap.holder(*i, key).is_none())
                .cloned()
                .collect();
        }
        for (i, keys) in pending {
            let action = keymap.bindings[i].action;
            let conflict = keys
                .iter()
                .find_map(|key| keymap.holder(i, key).map(|other| (key, other)));
            if let Some((key, other)) = conflict {
                log::warn!("ignoring the shortcut for {action}: {key:?} is bound to {other}");
                keymap
                    .inactive
                    .insert(action.to_string(), Value::from(keys.clone()));
            }
        }
        keymap
    }

    /// The action other than the `i`th that holds `key`, if any.
    fn holder(&self, i: usize, key: &str) -> Option<&'static str> {
        let action = self.bindings[i].action;
        self.bindings
            .iter()
            .find(|b| b.action != action && b.keys.iter().any(|k| k == key))
            .map(|b| b.action)
    }

    /// The accelerator of `action`'s menu item: the first of its keys, in
    /// stored order, that converts to one. `None` when no key converts, or
    /// for an unknown action.
    pub fn accelerator_for(&self, action: &str) -> Option<String> {
        self.bindings
            .iter()
            .find(|b| b.action == action)?
            .keys
            .iter()
            .find_map(|k| accelerator(k))
    }

    /// The bindings in the order the shortcuts panel shows them.
    pub fn bindings(&self) -> Vec<Binding> {
        self.bindings.clone()
    }

    /// Add `key` to `action`'s keys, or explain why it cannot be: a key
    /// bound to another action is refused rather than moved, and a key the action holds is an error.
    pub fn add(&mut self, action: &str, key: &str) -> Result<(), String> {
        let i = self.check_bindable(action, key)?;
        if self.bindings[i].keys.iter().any(|k| k == key) {
            return Err(format!("{key:?} is already bound to {action}"));
        }
        self.bindings[i].keys.push(key.to_string());
        self.inactive.remove(action);
        Ok(())
    }

    /// Remove `key` from `action`'s keys. The last key cannot be removed, as an empty list does not load back.
    pub fn remove(&mut self, action: &str, key: &str) -> Result<(), String> {
        let i = self.index(action)?;
        let keys = &mut self.bindings[i].keys;
        let Some(at) = keys.iter().position(|k| k == key) else {
            return Err(format!("{key:?} is not bound to {action}"));
        };
        if keys.len() == 1 {
            return Err(format!("{key:?} is the only key for {action}; use Reset"));
        }
        keys.remove(at);
        self.inactive.remove(action);
        Ok(())
    }

    fn check_bindable(&self, action: &str, key: &str) -> Result<usize, String> {
        let i = self.index(action)?;
        if let Some(reason) = forbidden(key, MACOS) {
            return Err(format!("{key:?} {reason}"));
        }
        if let Some(other) = self
            .bindings
            .iter()
            .find(|b| b.action != action && b.keys.iter().any(|k| k == key))
        {
            return Err(format!("{key:?} is bound to {}", other.action));
        }
        Ok(i)
    }

    /// Restore `action`'s default keys, or fail if one of them has since been
    /// bound to another action.
    pub fn reset(&mut self, action: &str) -> Result<(), String> {
        let i = self.index(action)?;
        let defaults = Keymap::defaults().bindings.swap_remove(i).keys;
        if let Some((key, other)) = defaults.iter().find_map(|key| {
            self.bindings
                .iter()
                .find(|b| b.action != action && b.keys.contains(key))
                .map(|b| (key, b.action))
        }) {
            return Err(format!("{key:?} is bound to {other}"));
        }
        self.bindings[i].keys = defaults;
        self.inactive.remove(action);
        Ok(())
    }

    /// Restore every action's default keys.
    pub fn reset_all(&mut self) {
        *self = Keymap::defaults();
    }

    /// The actions whose keys differ from the default, as stored under the
    /// `shortcuts` settings key. A stored override skipped for a key
    /// conflict is kept as it was, minus any modifier-only keys.
    pub fn overrides(&self) -> Value {
        let defaults = Keymap::defaults();
        let mut overrides = self.inactive.clone();
        overrides.extend(
            self.bindings
                .iter()
                .zip(&defaults.bindings)
                .filter(|(b, d)| b.keys != d.keys)
                .map(|(b, _)| (b.action.to_string(), Value::from(b.keys.clone()))),
        );
        Value::Object(overrides)
    }

    fn index(&self, action: &str) -> Result<usize, String> {
        self.bindings
            .iter()
            .position(|b| b.action == action)
            .ok_or_else(|| format!("unknown action {action:?}"))
    }
}

fn parse_keys(value: &Value) -> Option<Vec<String>> {
    let keys = value.as_array()?;
    if keys.is_empty() {
        return None;
    }
    let mut keys: Vec<String> = keys
        .iter()
        .map(|k| k.as_str().filter(|k| !k.is_empty()).map(str::to_string))
        .collect::<Option<Vec<String>>>()?;
    keys.retain(|k| {
        let modifier_only = MODIFIER_ONLY.contains(&k.as_str());
        if modifier_only {
            log::warn!("ignoring the modifier-only key {k:?} in a shortcut");
        }
        !modifier_only
    });
    if keys.is_empty() {
        return None;
    }
    Some(keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys_of(keymap: &Keymap, action: &str) -> Vec<String> {
        keymap
            .bindings()
            .into_iter()
            .find(|b| b.action == action)
            .unwrap()
            .keys
    }

    #[test]
    fn no_overrides_equals_the_defaults() {
        assert_eq!(Keymap::from_overrides(None), Keymap::defaults());
        assert_eq!(Keymap::from_overrides(Some(&json!({}))), Keymap::defaults());
    }

    #[test]
    fn a_valid_override_replaces_only_that_action() {
        let keymap = Keymap::from_overrides(Some(&json!({"reject": ["r"]})));
        assert_eq!(keys_of(&keymap, "reject"), vec!["r"]);
        let mut expected = Keymap::defaults();
        expected
            .bindings
            .iter_mut()
            .find(|b| b.action == "reject")
            .unwrap()
            .keys = vec!["r".into()];
        assert_eq!(keymap, expected);
    }

    #[test]
    fn the_extend_actions_can_be_rebound() {
        let keymap = Keymap::from_overrides(Some(&json!({
            "extendPrevious": ["shift+k"],
            "extendNext": ["shift+j"],
        })));
        assert_eq!(keys_of(&keymap, "extendPrevious"), vec!["shift+k"]);
        assert_eq!(keys_of(&keymap, "extendNext"), vec!["shift+j"]);
        assert_eq!(
            keymap.overrides(),
            json!({"extendPrevious": ["shift+k"], "extendNext": ["shift+j"]})
        );
    }

    #[test]
    fn invalid_shapes_keep_the_defaults() {
        for value in [
            json!(["r"]),
            json!("r"),
            json!({"nope": ["r"]}),
            json!({"reject": "r"}),
            json!({"reject": []}),
            json!({"reject": [""]}),
            json!({"reject": ["r", 1]}),
            json!({"reject": null}),
        ] {
            assert_eq!(
                Keymap::from_overrides(Some(&value)),
                Keymap::defaults(),
                "{value}"
            );
        }
    }

    #[test]
    fn an_unknown_action_does_not_block_the_others() {
        let keymap = Keymap::from_overrides(Some(&json!({"nope": ["q"], "reject": ["r"]})));
        assert_eq!(keys_of(&keymap, "reject"), vec!["r"]);
    }

    #[test]
    fn p_on_another_action_is_skipped_while_pick_holds_it() {
        let keymap = Keymap::from_overrides(Some(&json!({"reject": ["p"]})));
        assert_eq!(keys_of(&keymap, "reject"), vec!["x"]);
        assert_eq!(keymap.overrides(), json!({"reject": ["p"]}));
    }

    #[test]
    fn a_pick_override_applies_and_releases_p() {
        let keymap = Keymap::from_overrides(Some(&json!({"pick": ["q"]})));
        assert_eq!(keys_of(&keymap, "pick"), vec!["q"]);
        let keymap = Keymap::from_overrides(Some(&json!({"pick": ["q"], "reject": ["p"]})));
        assert_eq!(keys_of(&keymap, "pick"), vec!["q"]);
        assert_eq!(keys_of(&keymap, "reject"), vec!["p"]);
    }

    #[test]
    fn a_collision_with_another_default_is_skipped() {
        let keymap = Keymap::from_overrides(Some(&json!({"reject": ["arrowdown"]})));
        assert_eq!(keymap.bindings(), Keymap::defaults().bindings());
    }

    #[test]
    fn colliding_overrides_keep_the_first_in_action_order() {
        let keymap = Keymap::from_overrides(Some(&json!({"clear": ["r"], "reject": ["r"]})));
        assert_eq!(keys_of(&keymap, "reject"), vec!["r"]);
        assert_eq!(keys_of(&keymap, "clear"), vec!["0"]);
    }

    #[test]
    fn the_defaults_bind_no_key_twice() {
        let mut keys: Vec<String> = Keymap::defaults()
            .bindings()
            .into_iter()
            .flat_map(|b| b.keys)
            .collect();
        let total = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), total);
    }

    #[test]
    fn the_defaults_are_the_full_table() {
        let bindings: Vec<(&str, Vec<String>)> = Keymap::defaults()
            .bindings()
            .into_iter()
            .map(|b| (b.action, b.keys))
            .collect();
        let expected: Vec<(&str, Vec<String>)> = [
            ("previous", "arrowleft"),
            ("next", "arrowright"),
            ("burstPrevious", "arrowup"),
            ("burstNext", "arrowdown"),
            ("burstFramePrevious", "alt+arrowleft"),
            ("burstFrameNext", "alt+arrowright"),
            ("extendPrevious", "shift+arrowleft"),
            ("extendNext", "shift+arrowright"),
            ("selectAll", SELECT_ALL_DEFAULT),
            ("open", OPEN_DEFAULT),
            ("undo", UNDO_DEFAULT),
            ("redo", REDO_DEFAULT),
            ("toggleLeft", TOGGLE_LEFT_DEFAULT),
            ("toggleRight", TOGGLE_RIGHT_DEFAULT),
            ("toggleStrip", TOGGLE_STRIP_DEFAULT),
            ("toggleSides", "tab"),
            ("focus", "f"),
            ("zoom", "z"),
            ("grayscale", "g"),
            ("compare", "v"),
            ("performance", "d"),
            ("peaking", "e"),
            ("rate1", "1"),
            ("rate2", "2"),
            ("rate3", "3"),
            ("rate4", "4"),
            ("rate5", "5"),
            ("reject", "x"),
            ("rejectRest", "shift+x"),
            ("pick", "p"),
            ("unflag", "u"),
            ("clear", "0"),
            ("red", "ctrl+alt+1"),
            ("orange", "ctrl+alt+2"),
            ("yellow", "ctrl+alt+3"),
            ("green", "ctrl+alt+4"),
            ("blue", "ctrl+alt+5"),
            ("pink", "ctrl+alt+6"),
            ("purple", "ctrl+alt+7"),
            ("clearlabel", "ctrl+alt+0"),
            ("clearall", "c"),
        ]
        .into_iter()
        .map(|(action, key)| (action, vec![key.to_string()]))
        .collect();
        assert_eq!(bindings, expected);
    }

    #[test]
    fn the_menu_defaults_follow_the_platform() {
        let keymap = Keymap::defaults();
        if MACOS {
            assert_eq!(keys_of(&keymap, "open"), vec!["meta+o"]);
            assert_eq!(keys_of(&keymap, "undo"), vec!["meta+z"]);
            assert_eq!(keys_of(&keymap, "redo"), vec!["shift+meta+z"]);
            assert_eq!(keys_of(&keymap, "selectAll"), vec!["meta+a"]);
        } else {
            assert_eq!(keys_of(&keymap, "open"), vec!["ctrl+o"]);
            assert_eq!(keys_of(&keymap, "undo"), vec!["ctrl+z"]);
            assert_eq!(keys_of(&keymap, "redo"), vec!["ctrl+shift+z"]);
            assert_eq!(keys_of(&keymap, "selectAll"), vec!["ctrl+a"]);
        }
    }

    #[test]
    fn accelerator_converts_the_key_names() {
        for (key, expected) in [
            ("ctrl+o", "Ctrl+O"),
            ("meta+o", "Cmd+O"),
            ("shift+meta+o", "Shift+Cmd+O"),
            ("ctrl+shift+o", "Ctrl+Shift+O"),
            ("ctrl+alt+shift+meta+k", "Ctrl+Alt+Shift+Cmd+K"),
            ("ctrl+alt+1", "Ctrl+Alt+1"),
            ("ctrl+,", "Ctrl+,"),
            ("meta+`", "Cmd+`"),
            ("alt+\\", "Alt+\\"),
            ("ctrl+space", "Ctrl+Space"),
            ("meta+arrowleft", "Cmd+ArrowLeft"),
            ("ctrl+pagedown", "Ctrl+PageDown"),
            ("ctrl+f12", "Ctrl+F12"),
        ] {
            assert_eq!(accelerator(key).as_deref(), Some(expected), "{key}");
        }
        for key in [
            "o",
            "shift+j",
            "space",
            "arrowup",
            "shift+arrowup",
            "shift+arrowdown",
            "ctrl+f25",
            "ctrl+oo",
            "ctrl+",
            "meta",
        ] {
            assert_eq!(accelerator(key), None, "{key}");
        }
    }

    #[test]
    fn the_panel_toggles_have_modifier_defaults() {
        let keymap = Keymap::defaults();
        let (modifiers, shown) = if MACOS {
            ("alt+meta+", "Alt+Cmd+")
        } else {
            ("ctrl+alt+", "Ctrl+Alt+")
        };
        for (action, arrow, name) in [
            ("toggleLeft", "arrowleft", "ArrowLeft"),
            ("toggleRight", "arrowright", "ArrowRight"),
            ("toggleStrip", "arrowdown", "ArrowDown"),
        ] {
            assert_eq!(
                keys_of(&keymap, action),
                vec![format!("{modifiers}{arrow}")]
            );
            assert_eq!(
                keymap.accelerator_for(action),
                Some(format!("{shown}{name}")),
                "{action}"
            );
        }
        assert_eq!(
            accelerator("ctrl+alt+arrowleft").as_deref(),
            Some("Ctrl+Alt+ArrowLeft")
        );
        assert_eq!(
            accelerator("alt+meta+arrowleft").as_deref(),
            Some("Alt+Cmd+ArrowLeft")
        );
        assert_eq!(keys_of(&keymap, "toggleSides"), vec!["tab"]);
        assert_eq!(accelerator("tab"), None);
        for key in ["f6", "f7", "f8"] {
            assert!(
                keymap
                    .bindings()
                    .iter()
                    .all(|b| !b.keys.iter().any(|k| k == key)),
                "{key}"
            );
        }
    }

    #[test]
    fn a_stored_f7_override_still_applies() {
        let keymap = Keymap::from_overrides(Some(&json!({"toggleLeft": ["f7"]})));
        assert_eq!(keys_of(&keymap, "toggleLeft"), vec!["f7"]);
    }

    #[test]
    fn accelerator_for_takes_the_first_convertible_key() {
        let mut keymap = Keymap::defaults();
        assert_eq!(
            keymap.accelerator_for("open").as_deref(),
            Some(if MACOS { "Cmd+O" } else { "Ctrl+O" })
        );
        assert_eq!(
            keymap.accelerator_for("undo").as_deref(),
            Some(if MACOS { "Cmd+Z" } else { "Ctrl+Z" })
        );
        assert_eq!(
            keymap.accelerator_for("redo").as_deref(),
            Some(if MACOS { "Shift+Cmd+Z" } else { "Ctrl+Shift+Z" })
        );
        assert_eq!(
            keymap.accelerator_for("selectAll").as_deref(),
            Some(if MACOS { "Cmd+A" } else { "Ctrl+A" })
        );
        assert_eq!(keymap.accelerator_for("nope"), None);
        assert_eq!(keymap.accelerator_for("extendPrevious"), None);
        assert_eq!(keymap.accelerator_for("extendNext"), None);
        keymap.add("open", "j").unwrap();
        keymap.remove("open", OPEN_DEFAULT).unwrap();
        assert_eq!(keymap.accelerator_for("open"), None);
        keymap.add("open", "ctrl+alt+j").unwrap();
        assert_eq!(
            keymap.accelerator_for("open").as_deref(),
            Some("Ctrl+Alt+J")
        );
    }

    #[test]
    fn the_menu_defaults_are_not_forbidden() {
        for macos in [true, false] {
            let open = if macos { "meta+o" } else { "ctrl+o" };
            assert_eq!(forbidden(open, macos), None, "{open}");
            let undo = if macos { "meta+z" } else { "ctrl+z" };
            let redo = if macos {
                "shift+meta+z"
            } else {
                "ctrl+shift+z"
            };
            assert_eq!(forbidden(undo, macos), None, "{undo}");
            assert_eq!(forbidden(redo, macos), None, "{redo}");
            let select_all = if macos { "meta+a" } else { "ctrl+a" };
            assert_eq!(forbidden(select_all, macos), None, "{select_all}");
            for arrow in ["arrowleft", "arrowright", "arrowdown"] {
                let toggle = if macos {
                    format!("alt+meta+{arrow}")
                } else {
                    format!("ctrl+alt+{arrow}")
                };
                assert_eq!(forbidden(&toggle, macos), None, "{toggle}");
            }
        }
    }

    #[test]
    fn a_store_from_before_undo_redo_still_loads() {
        let stored = json!({"pick": ["q"], "burstNext": ["n"]});
        let keymap = Keymap::from_overrides(Some(&stored));
        assert_eq!(keys_of(&keymap, "pick"), vec!["q"]);
        assert_eq!(keys_of(&keymap, "burstNext"), vec!["n"]);
        assert_eq!(keys_of(&keymap, "undo"), vec![UNDO_DEFAULT]);
        assert_eq!(keys_of(&keymap, "redo"), vec![REDO_DEFAULT]);
        assert_eq!(keymap.overrides(), stored);
    }

    #[test]
    fn a_store_with_the_retired_photolab_action_still_loads() {
        let stored = json!({"photolab": ["ctrl+shift+o"], "pick": ["q"]});
        let keymap = Keymap::from_overrides(Some(&stored));
        assert_eq!(keys_of(&keymap, "pick"), vec!["q"]);
        let others = |keymap: &Keymap| {
            keymap
                .bindings()
                .into_iter()
                .filter(|b| b.action != "pick")
                .collect::<Vec<_>>()
        };
        assert_eq!(others(&keymap), others(&Keymap::defaults()));
        assert_eq!(keymap.overrides(), json!({"pick": ["q"]}));
    }

    #[test]
    fn a_default_freed_by_rebinding_can_go_to_another_action() {
        let mut keymap = Keymap::defaults();
        keymap.add("open", "ctrl+alt+j").unwrap();
        keymap.remove("open", OPEN_DEFAULT).unwrap();
        keymap.add("focus", OPEN_DEFAULT).unwrap();
        assert_eq!(keys_of(&keymap, "focus"), vec!["f", OPEN_DEFAULT]);
    }

    #[test]
    fn a_label_override_applies() {
        let keymap = Keymap::from_overrides(Some(&json!({"red": ["r"]})));
        assert_eq!(keys_of(&keymap, "red"), vec!["r"]);
    }

    #[test]
    fn ctrl_alt_keys_are_bindable() {
        let keymap = Keymap::from_overrides(Some(&json!({"reject": ["ctrl+alt+r"]})));
        assert_eq!(keys_of(&keymap, "reject"), vec!["ctrl+alt+r"]);
        let keymap = Keymap::from_overrides(Some(&json!({"reject": ["ctrl+alt+1"]})));
        assert_eq!(keymap.bindings(), Keymap::defaults().bindings());
    }

    #[test]
    fn overrides_and_reset_use_the_defaults() {
        let mut keymap = Keymap::defaults();
        keymap.add("red", "r").unwrap();
        assert_eq!(keymap.overrides(), json!({"red": ["ctrl+alt+1", "r"]}));
        keymap.reset("red").unwrap();
        assert_eq!(keys_of(&keymap, "red"), vec!["ctrl+alt+1"]);
        keymap.add("red", "r").unwrap();
        keymap.remove("red", "ctrl+alt+1").unwrap();
        keymap.reset_all();
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn add_keeps_the_existing_keys() {
        let mut keymap = Keymap::defaults();
        keymap.add("reject", "r").unwrap();
        assert_eq!(keys_of(&keymap, "reject"), vec!["x", "r"]);
        assert_eq!(keymap.overrides(), json!({"reject": ["x", "r"]}));
    }

    #[test]
    fn add_rejects_a_key_the_action_holds() {
        let mut keymap = Keymap::defaults();
        assert_eq!(
            keymap.add("reject", "x"),
            Err("\"x\" is already bound to reject".to_string())
        );
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn add_rejects_a_key_bound_to_another_action() {
        let mut keymap = Keymap::defaults();
        assert_eq!(
            keymap.add("reject", "arrowright"),
            Err("\"arrowright\" is bound to next".to_string())
        );
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn add_rejects_an_unknown_action() {
        let mut keymap = Keymap::defaults();
        assert!(keymap.add("nope", "q").is_err());
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn pick_is_editable() {
        let mut keymap = Keymap::defaults();
        keymap.add("pick", "q").unwrap();
        assert_eq!(keys_of(&keymap, "pick"), vec!["p", "q"]);
        assert_eq!(keymap.overrides(), json!({"pick": ["p", "q"]}));
        assert_eq!(
            keymap.add("reject", "p"),
            Err("\"p\" is bound to pick".to_string())
        );
        keymap.remove("pick", "p").unwrap();
        keymap.add("reject", "p").unwrap();
        assert_eq!(keys_of(&keymap, "reject"), vec!["x", "p"]);
        keymap.remove("reject", "p").unwrap();
        keymap.reset("pick").unwrap();
        assert_eq!(keys_of(&keymap, "pick"), vec!["p"]);
    }

    #[test]
    fn remove_drops_one_key() {
        let mut keymap = Keymap::defaults();
        keymap.add("previous", "k").unwrap();
        keymap.remove("previous", "arrowleft").unwrap();
        assert_eq!(keys_of(&keymap, "previous"), vec!["k"]);
    }

    #[test]
    fn remove_refuses_the_last_key() {
        let mut keymap = Keymap::defaults();
        assert_eq!(
            keymap.remove("reject", "x"),
            Err("\"x\" is the only key for reject; use Reset".to_string())
        );
        assert_eq!(
            keymap.remove("pick", "p"),
            Err("\"p\" is the only key for pick; use Reset".to_string())
        );
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn remove_rejects_a_key_not_on_the_action() {
        let mut keymap = Keymap::defaults();
        assert_eq!(
            keymap.remove("reject", "arrowdown"),
            Err("\"arrowdown\" is not bound to reject".to_string())
        );
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn adding_then_removing_returns_to_the_default() {
        let mut keymap = Keymap::defaults();
        keymap.add("reject", "r").unwrap();
        keymap.remove("reject", "r").unwrap();
        assert_eq!(keymap.overrides(), json!({}));
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn reset_restores_the_defaults() {
        let mut keymap = Keymap::defaults();
        keymap.add("reject", "r").unwrap();
        keymap.add("clear", "q").unwrap();
        keymap.add("previous", "k").unwrap();
        keymap.remove("previous", "arrowleft").unwrap();
        keymap.reset("reject").unwrap();
        assert_eq!(
            keymap.overrides(),
            json!({"clear": ["0", "q"], "previous": ["k"]})
        );
        keymap.reset_all();
        assert_eq!(keymap.overrides(), json!({}));
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn reset_refuses_a_default_key_now_bound_elsewhere() {
        let mut keymap = Keymap::defaults();
        keymap.add("reject", "r").unwrap();
        keymap.remove("reject", "x").unwrap();
        keymap.add("clear", "x").unwrap();
        assert_eq!(
            keymap.reset("reject"),
            Err("\"x\" is bound to clear".to_string())
        );
    }

    #[test]
    fn overrides_round_trip() {
        let mut keymap = Keymap::defaults();
        keymap.add("previous", "q").unwrap();
        keymap.remove("previous", "arrowleft").unwrap();
        keymap.add("reject", "r").unwrap();
        keymap.remove("reject", "x").unwrap();
        keymap.add("zoom", "space").unwrap();
        keymap.add("pick", "ctrl+alt+p").unwrap();
        keymap.remove("pick", "p").unwrap();
        assert_eq!(Keymap::from_overrides(Some(&keymap.overrides())), keymap);
    }

    #[test]
    fn the_burst_frame_keys_are_not_forbidden() {
        for macos in [true, false] {
            assert_eq!(forbidden("alt+arrowleft", macos), None);
            assert_eq!(forbidden("alt+arrowright", macos), None);
        }
    }

    #[test]
    fn forbidden_follows_the_platform() {
        assert_eq!(forbidden("meta+q", true), Some("is a menu accelerator"));
        assert_eq!(forbidden("meta+,", true), Some("is a menu accelerator"));
        assert_eq!(
            forbidden("shift+meta+4", true),
            Some("is reserved by the system")
        );
        assert_eq!(forbidden("meta+k", true), None);
        assert_eq!(forbidden("meta+arrowleft", true), None);
        assert_eq!(forbidden("meta+c", true), Some("is a menu accelerator"));
        assert_eq!(forbidden("ctrl+c", true), None);
        assert_eq!(forbidden("ctrl+c", false), Some("is a menu accelerator"));
        assert_eq!(forbidden("ctrl+,", false), Some("is a menu accelerator"));
        assert_eq!(forbidden("alt+f4", false), Some("is a menu accelerator"));
        assert_eq!(
            forbidden("alt+tab", false),
            Some("is reserved by the system")
        );
        assert_eq!(
            forbidden("meta+k", false),
            Some("is reserved by the system")
        );
        assert_eq!(
            forbidden("ctrl+alt+shift+meta+k", false),
            Some("is reserved by the system")
        );
        for key in ["shift+j", "ctrl+j", "alt+j", "ctrl+alt+1"] {
            assert_eq!(forbidden(key, true), None, "{key}");
            assert_eq!(forbidden(key, false), None, "{key}");
        }
    }

    #[test]
    fn add_refuses_a_forbidden_key() {
        let key = if MACOS { "meta+," } else { "ctrl+," };
        let mut keymap = Keymap::defaults();
        assert_eq!(
            keymap.add("reject", key),
            Err(format!("{key:?} is a menu accelerator"))
        );
        let key = if MACOS { "meta+tab" } else { "alt+tab" };
        assert_eq!(
            keymap.add("reject", key),
            Err(format!("{key:?} is reserved by the system"))
        );
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn a_forbidden_override_is_skipped() {
        let key = if MACOS { "meta+q" } else { "ctrl+c" };
        let keymap = Keymap::from_overrides(Some(&json!({"reject": [key]})));
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn modified_keys_are_bindable() {
        let mut keymap = Keymap::defaults();
        let mut keys = vec!["shift+j", "ctrl+j", "alt+j", "ctrl+alt+r"];
        if MACOS {
            keys.push("meta+k");
        }
        for key in &keys {
            keymap.add("reject", key).unwrap();
        }
        let expected: Vec<String> = std::iter::once("x").chain(keys).map(String::from).collect();
        assert_eq!(keys_of(&keymap, "reject"), expected);
    }

    #[test]
    fn an_inactive_override_survives_unrelated_edits() {
        let stored = json!({"reject": ["arrowup"]});
        let mut keymap = Keymap::from_overrides(Some(&stored));
        assert_eq!(keymap.overrides(), stored);
        keymap.add("zoom", "space").unwrap();
        assert_eq!(
            keymap.overrides(),
            json!({"reject": ["arrowup"], "zoom": ["z", "space"]})
        );
        keymap.remove("zoom", "space").unwrap();
        keymap.reset("zoom").unwrap();
        assert_eq!(keymap.overrides(), stored);
    }

    #[test]
    fn editing_or_resetting_an_action_drops_its_inactive_override() {
        let stored = json!({"reject": ["arrowup"], "clear": ["z"]});
        let mut keymap = Keymap::from_overrides(Some(&stored));
        keymap.add("reject", "r").unwrap();
        assert_eq!(
            keymap.overrides(),
            json!({"reject": ["x", "r"], "clear": ["z"]})
        );
        let mut keymap = Keymap::from_overrides(Some(&stored));
        keymap.add("reject", "r").unwrap();
        keymap.remove("reject", "r").unwrap();
        assert_eq!(keymap.overrides(), json!({"clear": ["z"]}));
        let mut keymap = Keymap::from_overrides(Some(&stored));
        keymap.reset("reject").unwrap();
        assert_eq!(keymap.overrides(), json!({"clear": ["z"]}));
        keymap.reset_all();
        assert_eq!(keymap.overrides(), json!({}));
        assert_eq!(keymap, Keymap::defaults());
    }

    #[test]
    fn inactive_overrides_round_trip() {
        let stored = json!({"reject": ["arrowup"]});
        let keymap = Keymap::from_overrides(Some(&stored));
        assert_eq!(keys_of(&keymap, "reject"), vec!["x"]);
        assert_eq!(keymap.overrides(), stored);
        assert_eq!(Keymap::from_overrides(Some(&keymap.overrides())), keymap);
    }

    #[test]
    fn a_store_from_the_old_defaults_still_loads() {
        let stored = json!({
            "zoom": ["space", "z"],
            "open": ["o"],
            "red": ["6"],
            "previous": ["arrowleft", "w", "a", "h", "k"],
        });
        let keymap = Keymap::from_overrides(Some(&stored));
        assert_eq!(keys_of(&keymap, "zoom"), vec!["space", "z"]);
        assert_eq!(keys_of(&keymap, "open"), vec!["o"]);
        assert_eq!(keymap.accelerator_for("open"), None);
        assert_eq!(keys_of(&keymap, "red"), vec!["6"]);
        assert_eq!(
            keys_of(&keymap, "previous"),
            vec!["arrowleft", "w", "a", "h", "k"]
        );
        assert_eq!(keymap.overrides(), stored);
        assert_eq!(Keymap::from_overrides(Some(&keymap.overrides())), keymap);
    }

    #[test]
    fn a_stale_default_in_an_old_store_drops_only_that_key() {
        let keymap = Keymap::from_overrides(Some(&json!({
            "previous": ["arrowup", "w", "a", "h", "k"],
            "next": ["arrowdown", "s", "n", "l", "j"],
        })));
        assert_eq!(keys_of(&keymap, "previous"), vec!["w", "a", "h", "k"]);
        assert_eq!(keys_of(&keymap, "next"), vec!["s", "n", "l", "j"]);
        assert_eq!(keys_of(&keymap, "burstPrevious"), vec!["arrowup"]);
        assert_eq!(keys_of(&keymap, "burstNext"), vec!["arrowdown"]);
        assert_eq!(
            keymap.overrides(),
            json!({"previous": ["w", "a", "h", "k"], "next": ["s", "n", "l", "j"]})
        );
        assert_eq!(Keymap::from_overrides(Some(&keymap.overrides())), keymap);
    }

    #[test]
    fn a_partly_applied_override_frees_its_default_for_another() {
        let keymap = Keymap::from_overrides(Some(&json!({
            "previous": ["arrowup", "w"],
            "burstFramePrevious": ["arrowleft"],
        })));
        assert_eq!(keys_of(&keymap, "previous"), vec!["w"]);
        assert_eq!(keys_of(&keymap, "burstPrevious"), vec!["arrowup"]);
        assert_eq!(keys_of(&keymap, "burstFramePrevious"), vec!["arrowleft"]);
        assert_eq!(Keymap::from_overrides(Some(&keymap.overrides())), keymap);
    }

    #[test]
    fn a_key_freed_by_a_later_partial_application_is_recovered() {
        let stored = json!({"previous": ["w", "arrowup"], "burstPrevious": ["arrowleft"]});
        let keymap = Keymap::from_overrides(Some(&stored));
        assert_eq!(keys_of(&keymap, "previous"), vec!["w", "arrowup"]);
        assert_eq!(keys_of(&keymap, "burstPrevious"), vec!["arrowleft"]);
        assert_eq!(keymap.overrides(), stored);
        assert_eq!(Keymap::from_overrides(Some(&keymap.overrides())), keymap);
    }

    #[test]
    fn a_modifier_only_override_keeps_the_default() {
        let keymap = Keymap::from_overrides(Some(&json!({"zoom": ["control"]})));
        assert_eq!(keys_of(&keymap, "zoom"), vec!["z"]);
        assert_eq!(keymap.overrides(), json!({}));
    }

    #[test]
    fn a_modifier_only_key_is_dropped_from_a_mixed_override() {
        let keymap = Keymap::from_overrides(Some(&json!({"zoom": ["shift", "m"]})));
        assert_eq!(keys_of(&keymap, "zoom"), vec!["m"]);
        assert_eq!(keymap.overrides(), json!({"zoom": ["m"]}));
        assert_eq!(Keymap::from_overrides(Some(&keymap.overrides())), keymap);
    }

    #[test]
    fn a_conflicting_override_keeps_only_its_filtered_keys() {
        let stored = json!({"reject": ["meta", "arrowup"]});
        let keymap = Keymap::from_overrides(Some(&stored));
        assert_eq!(keys_of(&keymap, "reject"), vec!["x"]);
        assert_eq!(keymap.overrides(), json!({"reject": ["arrowup"]}));
        assert_eq!(Keymap::from_overrides(Some(&keymap.overrides())), keymap);
    }
}
