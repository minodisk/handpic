//! The thumbnail strip's height, set by dragging its top edge and stored
//! under `stripHeight`, in logical px. The cells are sized from it
//! (`strip::Geometry`); the drawn strip pane is `HEADER` taller, the row of
//! `Sort ▾` / `Filter ▾` over the cells, and `GUTTER` taller again, the room
//! under the cells for the horizontal scrollbar.

use serde_json::{Value, json};

use crate::strip::STRIP_PADDING;
use crate::theme::SCROLLBAR_TRACK;

/// The Tauri strip's height, the height before any drag: a 144 x 96 image
/// box.
pub const DEFAULT: f32 = 128.;
/// A 96 x 64 image box.
pub const MIN: f32 = 96.;
/// A 405 x 270 image box, the stored thumbnail's height.
pub const MAX: f32 = 302.;
/// What a drag leaves the preview.
pub const PREVIEW_MIN: f32 = 240.;
/// The header row over the cells: the resize handle's 5 px band, a 1 px gap,
/// the 21 px buttons and 5 px under them.
pub const HEADER: f32 = 32.;
/// The band under the cells, with their bottom `STRIP_PADDING`, the
/// scrollbar's track: the track starts at the cells' bottom edge, so no
/// cell, name or burst band lies under the thumb. Reserved whether or not the
/// strip overflows.
pub const GUTTER: f32 = SCROLLBAR_TRACK - STRIP_PADDING;

/// The stored `stripHeight` value; a missing, non-number or non-finite value
/// means `DEFAULT`, and a number out of range is clamped.
pub fn from_setting(value: Option<&Value>) -> f32 {
    value
        .and_then(Value::as_f64)
        .filter(|h| h.is_finite())
        .map_or(DEFAULT, |h| h.clamp(MIN.into(), MAX.into()) as f32)
}

pub fn setting(height: f32) -> Value {
    json!(height)
}

/// The cells' height after the strip pane's top edge is dragged to make the
/// pane, header and gutter included, `wanted` px high, the preview and the pane sharing
/// `shared` px: rounded to whole px, within `MIN` and `MAX` and leaving the
/// preview `PREVIEW_MIN`, never under `MIN`.
pub fn dragged(wanted: f32, shared: f32) -> f32 {
    (wanted - HEADER - GUTTER)
        .round()
        .min(shared - PREVIEW_MIN - HEADER - GUTTER)
        .clamp(MIN, MAX)
}

/// The strip pane's height, header and gutter included, its top edge pressed
/// at the window `press` with the pane `start` high, the pointer at `y`:
/// `start` moved by the pointer's travel from the press, the pane growing as
/// the pointer goes up.
pub fn wanted(start: f32, press: f32, y: f32) -> f32 {
    start - (y - press)
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT, GUTTER, HEADER, MAX, MIN, PREVIEW_MIN, SCROLLBAR_TRACK, STRIP_PADDING, dragged,
        from_setting, setting, wanted,
    };
    use serde_json::json;

    #[test]
    fn the_stored_value_falls_back_to_the_default() {
        assert_eq!(from_setting(None), DEFAULT);
        assert_eq!(from_setting(Some(&json!("tall"))), DEFAULT);
        assert_eq!(from_setting(Some(&json!(null))), DEFAULT);
        assert_eq!(from_setting(Some(&json!({"height": 200}))), DEFAULT);
        assert_eq!(from_setting(Some(&json!(200.5))), 200.5);
    }

    #[test]
    fn the_stored_value_is_clamped() {
        assert_eq!(from_setting(Some(&json!(10))), MIN);
        assert_eq!(from_setting(Some(&json!(-1e300))), MIN);
        assert_eq!(from_setting(Some(&json!(1e300))), MAX);
    }

    #[test]
    fn the_stored_value_round_trips() {
        assert_eq!(setting(173.), json!(173.0));
        assert_eq!(from_setting(Some(&setting(173.))), 173.);
        assert_eq!(from_setting(Some(&setting(DEFAULT))), DEFAULT);
    }

    #[test]
    fn the_gutter_holds_the_track_under_the_cells() {
        assert_eq!(GUTTER, 9.);
        assert_eq!(STRIP_PADDING + GUTTER, SCROLLBAR_TRACK);
    }

    #[test]
    fn a_drag_follows_the_pointer_within_the_range() {
        let extra = HEADER + GUTTER;
        assert_eq!(dragged(extra + 200.4, 900.), 200.);
        assert_eq!(dragged(extra + 200.6, 900.), 201.);
        assert_eq!(dragged(extra + MIN, 900.), MIN);
        assert_eq!(dragged(extra + MAX, 900.), MAX);
        // The header and gutter alone, or less, still leave the cells their minimum.
        assert_eq!(dragged(extra + 20., 900.), MIN);
        assert_eq!(dragged(extra, 900.), MIN);
        assert_eq!(dragged(HEADER, 900.), MIN);
        assert_eq!(dragged(-50., 900.), MIN);
        assert_eq!(dragged(600., 900.), MAX);
    }

    #[test]
    fn a_drag_leaves_the_preview_its_minimum() {
        let extra = HEADER + GUTTER;
        // 500 - 240 - 32 - 9 leaves 219 for the cells.
        assert_eq!(dragged(extra + 300., 500.), 219.);
        assert_eq!(dragged(extra + 300., extra + 260. + PREVIEW_MIN), 260.);
        // The preview keeps its minimum: the pane ends at `shared - PREVIEW_MIN`.
        let shared = 500.;
        assert_eq!(extra + dragged(shared, shared), shared - PREVIEW_MIN);
        // Never under the minimum, whatever the room.
        assert_eq!(dragged(extra + 300., 200.), MIN);
        assert_eq!(dragged(extra + 300., extra + PREVIEW_MIN + 50.), MIN);
    }

    #[test]
    fn a_drag_moves_the_edge_by_the_pointer_travel_from_the_press() {
        let start = HEADER + DEFAULT + GUTTER;
        // 50 px above the press, a pane 50 px higher.
        assert_eq!(wanted(start, 700., 650.), start + 50.);
        assert_eq!(dragged(wanted(start, 700., 650.), 900.), DEFAULT + 50.);
        // Back at the press, the starting height exactly.
        assert_eq!(wanted(start, 700., 700.), start);
        assert_eq!(dragged(wanted(start, 700., 700.), 900.), DEFAULT);
    }

    #[test]
    fn a_drag_past_a_clamp_and_back_does_not_drift() {
        let start = HEADER + DEFAULT + GUTTER;
        let at = |y: f32| dragged(wanted(start, 700., y), 900.);
        assert_eq!(at(0.), MAX);
        assert_eq!(at(900.), MIN);
        assert_eq!(at(680.), DEFAULT + 20.);
    }
}
