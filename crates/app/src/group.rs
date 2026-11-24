//! The group threshold: how many confident faces of the judged face's size
//! (the stored gate count, `StoredEyes::faces_group`) a frame needs for
//! pass 3 to check all its faces, stored under `groupMinFaces`, and the rule
//! that gives a frame's all-faces counts as the app shows them.

use handpic_engine::index::StoredEyes;
use serde_json::Value;

/// The threshold before any change.
pub const DEFAULT: u32 = 5;
pub const MIN: u32 = 2;
pub const MAX: u32 = 20;

/// Why a value is refused by `set_settings`, and in the settings modal.
pub const RANGE_REFUSAL: &str = "group_min_faces must be a whole number from 2 to 20";
pub const FIELD_REFUSAL: &str = "Faces in a group photo must be a whole number from 2 to 20.";

/// `n` when it is within `MIN` and `MAX`.
pub fn valid(n: u64) -> Option<u32> {
    u32::try_from(n).ok().filter(|n| (MIN..=MAX).contains(n))
}

/// The stored `groupMinFaces` value; a missing, non-integer or out of range
/// value means `DEFAULT`.
pub fn from_setting(value: Option<&Value>) -> u32 {
    value
        .and_then(Value::as_u64)
        .and_then(valid)
        .unwrap_or(DEFAULT)
}

/// A value typed in the settings modal's field, surrounding blanks ignored.
pub fn parse(text: &str) -> Result<u32, String> {
    text.trim()
        .parse::<u64>()
        .ok()
        .and_then(valid)
        .ok_or_else(|| FIELD_REFUSAL.to_owned())
}

/// A frame's all-faces counts as the app shows them.
// The meta pane, the strip and the filter are to read the counts through
// `shown`; until then only the tests do.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub judged: Option<u32>,
    pub closed: Option<u32>,
    pub soft: Option<u32>,
}

/// The stored counts when the stored gate count is at or above `min_faces`,
/// else unknown: pass 3 may have run under a lower threshold, and its counts
/// are not shown for a frame the threshold in force leaves out.
#[cfg_attr(not(test), allow(dead_code))]
pub fn shown(eyes: &StoredEyes, min_faces: u32) -> Counts {
    if eyes.faces_group.is_none_or(|group| group < min_faces) {
        return Counts::default();
    }
    Counts {
        judged: eyes.faces_judged,
        closed: eyes.faces_closed,
        soft: eyes.faces_soft,
    }
}

#[cfg(test)]
mod tests {
    use super::{Counts, DEFAULT, from_setting, parse, shown, valid};
    use handpic_engine::index::StoredEyes;
    use serde_json::json;

    #[test]
    fn the_stored_value_is_read_with_its_rule() {
        assert_eq!(from_setting(Some(&json!(3))), 3);
        assert_eq!(from_setting(Some(&json!(2))), 2);
        assert_eq!(from_setting(Some(&json!(20))), 20);
        for odd in [
            json!(1),
            json!(21),
            json!(0),
            json!(-4),
            json!(4.5),
            json!("6"),
            json!(null),
        ] {
            assert_eq!(from_setting(Some(&odd)), DEFAULT, "{odd}");
        }
        assert_eq!(from_setting(None), DEFAULT);
        assert_eq!(DEFAULT, 5);
    }

    #[test]
    fn a_typed_value_is_a_whole_number_from_2_to_20() {
        assert_eq!(parse("2"), Ok(2));
        assert_eq!(parse(" 20 "), Ok(20));
        for odd in ["1", "21", "", "5.5", "-3", "five", "99999999999"] {
            assert!(parse(odd).is_err(), "{odd}");
        }
        assert_eq!(valid(u64::MAX), None);
    }

    fn eyes(group: Option<u32>) -> StoredEyes {
        StoredEyes {
            faces_group: group,
            faces_judged: Some(6),
            faces_closed: Some(1),
            faces_soft: None,
            ..StoredEyes::new(None, None, None, None, None)
        }
    }

    #[test]
    fn the_counts_show_only_at_or_above_the_threshold() {
        let counts = Counts {
            judged: Some(6),
            closed: Some(1),
            soft: None,
        };
        assert_eq!(shown(&eyes(Some(5)), 5), counts);
        assert_eq!(shown(&eyes(Some(6)), 5), counts);
        assert_eq!(shown(&eyes(Some(4)), 5), Counts::default());
        assert_eq!(shown(&eyes(Some(6)), 7), Counts::default());
        assert_eq!(shown(&eyes(None), 2), Counts::default());
    }
}
