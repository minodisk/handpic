//! Which of the left pane, the thumbnail strip and the right pane are shown,
//! a port of `panels.ts` and of the Tauri `panels_setting`, the value stored
//! under `panels`.

use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panels {
    pub left: bool,
    pub strip: bool,
    pub right: bool,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            left: true,
            strip: true,
            right: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Left,
    Strip,
    Right,
}

impl Panels {
    pub fn toggle(self, which: Panel) -> Self {
        match which {
            Panel::Left => Self {
                left: !self.left,
                ..self
            },
            Panel::Strip => Self {
                strip: !self.strip,
                ..self
            },
            Panel::Right => Self {
                right: !self.right,
                ..self
            },
        }
    }

    /// Lightroom's Tab: hide both side panes when either is shown, else show
    /// both.
    pub fn toggle_sides(self) -> Self {
        let shown = !(self.left || self.right);
        Self {
            left: shown,
            right: shown,
            ..self
        }
    }

    /// The stored `panels` value; a missing or non-boolean entry means shown.
    pub fn from_setting(value: Option<&Value>) -> Self {
        let shown = |name: &str| {
            value
                .and_then(|v| v.get(name))
                .and_then(Value::as_bool)
                .unwrap_or(true)
        };
        Self {
            left: shown("left"),
            strip: shown("strip"),
            right: shown("right"),
        }
    }

    pub fn setting(self) -> Value {
        json!({"left": self.left, "strip": self.strip, "right": self.right})
    }
}

#[cfg(test)]
mod tests {
    use super::{Panel, Panels};
    use serde_json::json;

    const ALL: Panels = Panels {
        left: true,
        strip: true,
        right: true,
    };

    fn panels(left: bool, strip: bool, right: bool) -> Panels {
        Panels { left, strip, right }
    }

    #[test]
    fn toggle_flips_only_the_named_panel() {
        assert_eq!(ALL.toggle(Panel::Left), panels(false, true, true));
        assert_eq!(ALL.toggle(Panel::Strip), panels(true, false, true));
        assert_eq!(ALL.toggle(Panel::Right), panels(true, true, false));
        assert_eq!(ALL.toggle(Panel::Right).toggle(Panel::Right), ALL);
    }

    #[test]
    fn toggle_sides_hides_both_when_both_are_shown_and_twice_restores_them() {
        let hidden = ALL.toggle_sides();
        assert_eq!(hidden, panels(false, true, false));
        assert_eq!(hidden.toggle_sides(), ALL);
    }

    #[test]
    fn toggle_sides_hides_both_when_only_one_is_shown() {
        assert_eq!(
            panels(false, false, true).toggle_sides(),
            panels(false, false, false)
        );
        assert_eq!(
            panels(true, true, false).toggle_sides(),
            panels(false, true, false)
        );
    }

    #[test]
    fn the_stored_value_falls_back_to_shown() {
        assert_eq!(Panels::from_setting(None), ALL);
        assert_eq!(Panels::from_setting(Some(&json!("hidden"))), ALL);
        assert_eq!(
            Panels::from_setting(Some(&json!({"left": "no", "strip": null}))),
            ALL
        );
        assert_eq!(
            Panels::from_setting(Some(&json!({"left": false, "right": false, "extra": 1}))),
            panels(false, true, false)
        );
    }

    #[test]
    fn the_stored_value_round_trips() {
        for left in [false, true] {
            for strip in [false, true] {
                for right in [false, true] {
                    let p = panels(left, strip, right);
                    assert_eq!(Panels::from_setting(Some(&p.setting())), p);
                }
            }
        }
        assert_eq!(
            panels(false, true, false).setting(),
            json!({"left": false, "strip": true, "right": false})
        );
    }
}
