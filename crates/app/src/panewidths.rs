//! The widths of the left pane (the folder tree) and the right pane (the meta
//! pane), set by dragging their boundary with the preview and stored under
//! `panelWidths`, in logical px.

use serde_json::{Value, json};

use crate::panels::{Panel, Panels};

/// The Tauri `#side` and `#info` width, the width before any drag.
pub const DEFAULT: f32 = 220.;
pub const MIN: f32 = 160.;
pub const MAX: f32 = 480.;
/// What a drag leaves the preview, and what a narrow window takes from the
/// panes before the preview.
pub const PREVIEW_MIN: f32 = 240.;

/// The user's widths, kept as set while a narrow window draws the panes
/// narrower (`drawn`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneWidths {
    pub left: f32,
    pub right: f32,
}

impl Default for PaneWidths {
    fn default() -> Self {
        Self {
            left: DEFAULT,
            right: DEFAULT,
        }
    }
}

impl PaneWidths {
    /// The stored `panelWidths` value; a value that is not an object, or a
    /// missing, non-number or non-finite entry, means `DEFAULT`, and a number
    /// out of range is clamped.
    pub fn from_setting(value: Option<&Value>) -> Self {
        let width = |name: &str| {
            value
                .and_then(|v| v.get(name))
                .and_then(Value::as_f64)
                .filter(|w| w.is_finite())
                .map_or(DEFAULT, |w| w.clamp(MIN.into(), MAX.into()) as f32)
        };
        Self {
            left: width("left"),
            right: width("right"),
        }
    }

    pub fn setting(self) -> Value {
        json!({"left": self.left, "right": self.right})
    }

    /// The widths after `pane`'s boundary is dragged to the window `x` in a
    /// window `window` wide: the left pane ends at `x` and the right one
    /// starts there, rounded to whole px, within `MIN` and `MAX` and leaving
    /// the preview `PREVIEW_MIN` beside the other pane when it is shown.
    pub fn dragged(self, pane: Panel, x: f32, window: f32, panels: Panels) -> Self {
        let room = |other: f32, shown: bool| window - PREVIEW_MIN - if shown { other } else { 0. };
        let width = |wanted: f32, room: f32| wanted.round().min(room).clamp(MIN, MAX);
        match pane {
            Panel::Left => Self {
                left: width(x, room(self.right, panels.right)),
                ..self
            },
            Panel::Right => Self {
                right: width(window - x, room(self.left, panels.left)),
                ..self
            },
            Panel::Strip => self,
        }
    }

    /// The widths to draw the shown panes at in a window `window` wide. When
    /// the window cannot hold them and `PREVIEW_MIN`, each shown pane gives up
    /// the excess in proportion to its width over `MIN`; no pane is drawn
    /// under `MIN`, so a window too narrow for both minimums and `PREVIEW_MIN`
    /// takes the rest from the preview. A hidden pane's width is 0.
    pub fn drawn(self, window: f32, panels: Panels) -> Self {
        let left = if panels.left { self.left } else { 0. };
        let right = if panels.right { self.right } else { 0. };
        let excess = left + right - (window - PREVIEW_MIN);
        if excess <= 0. {
            return Self { left, right };
        }
        let slack = |w: f32| if w > 0. { (w - MIN).max(0.) } else { 0. };
        let total = slack(left) + slack(right);
        let shrink = |w: f32| {
            if w == 0. {
                0.
            } else if excess >= total {
                MIN
            } else {
                w - slack(w) * excess / total
            }
        };
        Self {
            left: shrink(left),
            right: shrink(right),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT, MAX, MIN, PREVIEW_MIN, PaneWidths};
    use crate::panels::{Panel, Panels};
    use serde_json::json;

    const BOTH: Panels = Panels {
        left: true,
        strip: true,
        right: true,
    };

    fn widths(left: f32, right: f32) -> PaneWidths {
        PaneWidths { left, right }
    }

    #[test]
    fn the_stored_value_falls_back_to_the_default() {
        let default = widths(DEFAULT, DEFAULT);
        assert_eq!(PaneWidths::from_setting(None), default);
        assert_eq!(PaneWidths::from_setting(Some(&json!("wide"))), default);
        assert_eq!(PaneWidths::from_setting(Some(&json!([300, 300]))), default);
        assert_eq!(
            PaneWidths::from_setting(Some(&json!({"left": "300", "right": null}))),
            default
        );
        assert_eq!(
            PaneWidths::from_setting(Some(&json!({"right": 300.5}))),
            widths(DEFAULT, 300.5)
        );
    }

    #[test]
    fn the_stored_value_is_clamped() {
        assert_eq!(
            PaneWidths::from_setting(Some(&json!({"left": 10, "right": 1e300}))),
            widths(MIN, MAX)
        );
        assert_eq!(
            PaneWidths::from_setting(Some(&json!({"left": -1e300, "right": 0}))),
            widths(MIN, MIN)
        );
    }

    #[test]
    fn the_stored_value_round_trips() {
        let w = widths(173., 412.);
        assert_eq!(w.setting(), json!({"left": 173.0, "right": 412.0}));
        assert_eq!(PaneWidths::from_setting(Some(&w.setting())), w);
        let d = PaneWidths::default();
        assert_eq!(PaneWidths::from_setting(Some(&d.setting())), d);
    }

    #[test]
    fn a_drag_follows_the_pointer_within_the_pane_range() {
        let w = PaneWidths::default();
        assert_eq!(
            w.dragged(Panel::Left, 300.4, 1600., BOTH),
            widths(300., DEFAULT)
        );
        assert_eq!(
            w.dragged(Panel::Right, 1250., 1600., BOTH),
            widths(DEFAULT, 350.)
        );
        assert_eq!(
            w.dragged(Panel::Left, 20., 1600., BOTH),
            widths(MIN, DEFAULT)
        );
        assert_eq!(
            w.dragged(Panel::Left, 900., 1600., BOTH),
            widths(MAX, DEFAULT)
        );
        assert_eq!(
            w.dragged(Panel::Right, 1590., 1600., BOTH),
            widths(DEFAULT, MIN)
        );
        assert_eq!(
            w.dragged(Panel::Right, 0., 1600., BOTH),
            widths(DEFAULT, MAX)
        );
        assert_eq!(w.dragged(Panel::Strip, 300., 1600., BOTH), w);
    }

    #[test]
    fn a_drag_leaves_the_preview_its_minimum_beside_the_other_pane() {
        let w = widths(DEFAULT, 400.);
        // 900 - 240 - 400 leaves 260 for the left pane.
        assert_eq!(w.dragged(Panel::Left, 470., 900., BOTH), widths(260., 400.));
        // A hidden right pane leaves the left pane its maximum.
        let left_only = Panels {
            right: false,
            ..BOTH
        };
        assert_eq!(
            w.dragged(Panel::Left, 470., 900., left_only),
            widths(470., 400.)
        );
        // 900 - 240 - 220 leaves 440 for the right pane.
        assert_eq!(
            w.dragged(Panel::Right, 0., 900., BOTH),
            widths(DEFAULT, 440.)
        );
        // Never under the minimum, whatever the room.
        assert_eq!(w.dragged(Panel::Left, 470., 500., BOTH), widths(MIN, 400.));
    }

    #[test]
    fn a_window_that_holds_the_panes_draws_them_as_set() {
        let w = widths(300., 400.);
        assert_eq!(w.drawn(1600., BOTH), w);
        assert_eq!(w.drawn(300. + 400. + PREVIEW_MIN, BOTH), w);
        let none = Panels {
            left: false,
            right: false,
            ..BOTH
        };
        assert_eq!(w.drawn(100., none), widths(0., 0.));
    }

    #[test]
    fn a_narrow_window_narrows_the_panes_by_their_width_over_the_minimum() {
        // 140 + 240 px over the minimums; 100 px too many go 140 : 240.
        let w = widths(300., 400.);
        let drawn = w.drawn(300. + 400. + PREVIEW_MIN - 100., BOTH);
        assert!((drawn.left - (300. - 100. * 140. / 380.)).abs() < 1e-3);
        assert!((drawn.right - (400. - 100. * 240. / 380.)).abs() < 1e-3);
        assert!((drawn.left + drawn.right - 600.).abs() < 1e-3);
        // A pane at its minimum gives nothing.
        let drawn = widths(MIN, 400.).drawn(MIN + 400. + PREVIEW_MIN - 100., BOTH);
        assert_eq!(drawn, widths(MIN, 300.));
        // A hidden pane takes nothing.
        let right_only = Panels {
            left: false,
            ..BOTH
        };
        assert_eq!(
            w.drawn(400. + PREVIEW_MIN - 50., right_only),
            widths(0., 350.)
        );
    }

    #[test]
    fn a_window_too_narrow_for_the_minimums_takes_the_rest_from_the_preview() {
        let w = widths(300., 400.);
        assert_eq!(w.drawn(MIN + MIN + PREVIEW_MIN, BOTH), widths(MIN, MIN));
        assert_eq!(w.drawn(400., BOTH), widths(MIN, MIN));
        assert_eq!(w.drawn(0., BOTH), widths(MIN, MIN));
    }
}
