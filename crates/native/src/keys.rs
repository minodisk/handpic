//! A keystroke's name in the keymap's terms, a port of `keys.ts`'s `keyName`
//! onto GPUI, and the action the keymap binds it to.

use std::collections::HashMap;

use gpui::Keystroke;
use riffle_backend::shortcuts::Keymap;

/// The modifiers held in `ctrl+alt+shift+meta+` order, then the key
/// lower-cased, GPUI's arrow names given the keymap's; `None` for a lone
/// modifier. GPUI on Windows already reports a shifted digit or punctuation
/// key as the character it types with `shift` cleared (`!`, not `shift+1`),
/// which `keys.ts` names from `event.code` instead; no default key is one.
pub fn key_name(keystroke: &Keystroke) -> Option<String> {
    if keystroke.is_modifier_key() {
        return None;
    }
    let m = &keystroke.modifiers;
    let key = keystroke.key.to_lowercase();
    let key = match key.as_str() {
        "left" => "arrowleft",
        "right" => "arrowright",
        "up" => "arrowup",
        "down" => "arrowdown",
        other => other,
    };
    let mut name = String::new();
    for (held, modifier) in [
        (m.control, "ctrl+"),
        (m.alt, "alt+"),
        (m.shift, "shift+"),
        (m.platform, "meta+"),
    ] {
        if held {
            name.push_str(modifier);
        }
    }
    name.push_str(key);
    Some(name)
}

/// The character a printable key types, Shift or not, for the tree's
/// type-ahead (`folders.ts`'s `typeKey`), or `None` with Ctrl, Alt or Meta
/// held. `key_name` lower-cases a shifted letter and names the space bar
/// `space`, so the keystroke's own `key_char` comes first: `A` for Shift+A,
/// ` ` for the space bar. Without one, a one-character key is the character.
pub fn typed_char(keystroke: &Keystroke) -> Option<String> {
    let m = &keystroke.modifiers;
    if m.control || m.alt || m.platform || keystroke.is_modifier_key() {
        return None;
    }
    let one = |s: &str| {
        let mut chars = s.chars();
        matches!((chars.next(), chars.next()), (Some(c), None) if !c.is_control())
    };
    match &keystroke.key_char {
        Some(typed) if one(typed) => Some(typed.clone()),
        _ if keystroke.key == "space" => Some(" ".to_owned()),
        _ if one(&keystroke.key) => Some(keystroke.key.clone()),
        _ => None,
    }
}

/// Each bound key's action, built from the keymap, and built again whenever
/// the Keyboard Shortcuts tab edits it.
pub struct Bindings(HashMap<String, &'static str>);

impl Bindings {
    pub fn new(keymap: &Keymap) -> Self {
        Self(
            keymap
                .bindings()
                .into_iter()
                .flat_map(|b| b.keys.into_iter().map(move |key| (key, b.action)))
                .collect(),
        )
    }

    pub fn action_for(&self, key: &str) -> Option<&'static str> {
        self.0.get(key).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{Bindings, key_name, typed_char};
    use gpui::{Keystroke, Modifiers};
    use riffle_backend::shortcuts::Keymap;

    fn stroke(key: &str, control: bool, alt: bool, shift: bool) -> Keystroke {
        Keystroke {
            modifiers: Modifiers {
                control,
                alt,
                shift,
                ..Modifiers::default()
            },
            key: key.into(),
            key_char: None,
        }
    }

    #[test]
    fn names_follow_the_keymap() {
        let cases = [
            (stroke("1", false, false, false), "1"),
            (stroke("x", false, false, true), "shift+x"),
            (stroke("X", false, false, true), "shift+x"),
            (stroke("1", true, true, false), "ctrl+alt+1"),
            (stroke("z", true, false, false), "ctrl+z"),
            (stroke("z", true, false, true), "ctrl+shift+z"),
            (stroke("left", false, false, false), "arrowleft"),
            (stroke("right", false, true, false), "alt+arrowright"),
            (stroke("down", true, true, false), "ctrl+alt+arrowdown"),
            (stroke("space", false, false, false), "space"),
            (stroke("home", false, false, false), "home"),
        ];
        for (keystroke, name) in cases {
            assert_eq!(key_name(&keystroke).as_deref(), Some(name));
        }
    }

    #[test]
    fn a_printable_key_types_its_own_character() {
        let typed = |key: &str, key_char: Option<&str>, shift: bool| {
            let mut keystroke = stroke(key, false, false, shift);
            keystroke.key_char = key_char.map(str::to_owned);
            typed_char(&keystroke)
        };
        assert_eq!(typed("a", Some("A"), true).as_deref(), Some("A"));
        assert_eq!(typed("2", Some("2"), false).as_deref(), Some("2"));
        assert_eq!(typed("space", Some(" "), false).as_deref(), Some(" "));
        assert_eq!(typed("space", None, false).as_deref(), Some(" "));
        assert_eq!(typed("p", None, false).as_deref(), Some("p"));
        assert_eq!(typed("f1", None, false), None);
        assert_eq!(typed("enter", None, false), None);
        assert_eq!(typed("shift", None, true), None);
    }

    #[test]
    fn a_key_with_ctrl_alt_or_meta_types_nothing() {
        let mut keystroke = stroke("a", true, false, false);
        keystroke.key_char = Some("a".into());
        assert_eq!(typed_char(&keystroke), None);
        assert_eq!(typed_char(&stroke("a", false, true, false)), None);
        let mut meta = stroke("a", false, false, false);
        meta.modifiers.platform = true;
        assert_eq!(typed_char(&meta), None);
    }

    #[test]
    fn meta_comes_last() {
        let mut keystroke = stroke("a", true, false, true);
        keystroke.modifiers.platform = true;
        assert_eq!(key_name(&keystroke).as_deref(), Some("ctrl+shift+meta+a"));
    }

    #[test]
    fn a_lone_modifier_has_no_name() {
        for key in ["shift", "control", "alt", "platform", "function"] {
            assert_eq!(key_name(&stroke(key, false, false, true)), None);
        }
    }

    #[test]
    fn the_default_keymap_binds_the_named_keys() {
        let bindings = Bindings::new(&Keymap::defaults());
        let cases = [
            ("1", "rate1"),
            ("shift+x", "rejectRest"),
            ("x", "reject"),
            ("ctrl+alt+1", "red"),
            ("ctrl+alt+0", "clearlabel"),
            ("arrowleft", "previous"),
            ("arrowright", "next"),
            ("f", "focus"),
        ];
        for (key, action) in cases {
            assert_eq!(bindings.action_for(key), Some(action), "{key}");
        }
        assert_eq!(bindings.action_for("q"), None);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn undo_and_redo_are_ctrl_z_and_ctrl_shift_z() {
        let bindings = Bindings::new(&Keymap::defaults());
        assert_eq!(bindings.action_for("ctrl+z"), Some("undo"));
        assert_eq!(bindings.action_for("ctrl+shift+z"), Some("redo"));
    }
}
