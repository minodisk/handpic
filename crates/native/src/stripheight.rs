//! The thumbnail strip's height, set by dragging its top edge and stored
//! under `stripHeight`, in logical px. The cells are sized from it
//! (`strip::Geometry`).

use serde_json::{Value, json};

/// The Tauri strip's height, the height before any drag: a 144 x 96 image
/// box.
pub const DEFAULT: f32 = 128.;
/// A 96 x 64 image box.
pub const MIN: f32 = 96.;
/// A 405 x 270 image box, the stored thumbnail's height.
pub const MAX: f32 = 302.;
/// What a drag leaves the preview.
pub const PREVIEW_MIN: f32 = 240.;

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

/// The height after the strip's top edge is dragged to make it `wanted` px
/// high, the preview and the strip sharing `shared` px: rounded to whole px,
/// within `MIN` and `MAX` and leaving the preview `PREVIEW_MIN`, never under
/// `MIN`.
pub fn dragged(wanted: f32, shared: f32) -> f32 {
    wanted.round().min(shared - PREVIEW_MIN).clamp(MIN, MAX)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT, MAX, MIN, PREVIEW_MIN, dragged, from_setting, setting};
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
    fn a_drag_follows_the_pointer_within_the_range() {
        assert_eq!(dragged(200.4, 900.), 200.);
        assert_eq!(dragged(200.6, 900.), 201.);
        assert_eq!(dragged(20., 900.), MIN);
        assert_eq!(dragged(-50., 900.), MIN);
        assert_eq!(dragged(600., 900.), MAX);
    }

    #[test]
    fn a_drag_leaves_the_preview_its_minimum() {
        // 500 - 240 leaves 260 for the strip.
        assert_eq!(dragged(300., 500.), 260.);
        assert_eq!(dragged(300., 260. + PREVIEW_MIN), 260.);
        // Never under the minimum, whatever the room.
        assert_eq!(dragged(300., 200.), MIN);
    }
}
