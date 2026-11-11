//! One file's judgment as the view keeps it: the stars, the pick / reject
//! flag and the color label.

use riffle_backend::index::IndexedFile;
use riffle_core::Flag;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// `1..=5`; unrated and `0` are `None`.
    pub rating: Option<u8>,
    pub flag: Flag,
    pub label: Option<String>,
}

impl From<&IndexedFile> for State {
    fn from(file: &IndexedFile) -> Self {
        Self {
            rating: file.rating.filter(|r| (1..=5).contains(r)).map(|r| r as u8),
            flag: file.flag,
            label: file.label.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::State;
    use riffle_backend::index::IndexedFile;
    use riffle_core::Flag;

    fn file(rating: Option<i8>, flag: Flag, label: Option<&str>) -> IndexedFile {
        IndexedFile {
            path: "D:/a/DSC1.ARW".into(),
            orientation: 1,
            capture_time: None,
            subsec: None,
            focus: None,
            has_thumb: true,
            rating,
            flag,
            label: label.map(Into::into),
            has_sidecar: true,
            sharpness: None,
            exif: None,
            error: None,
        }
    }

    #[test]
    fn takes_the_stars_flag_and_label() {
        assert_eq!(
            State::from(&file(Some(3), Flag::Pick, Some("red"))),
            State {
                rating: Some(3),
                flag: Flag::Pick,
                label: Some("red".into()),
            }
        );
    }

    #[test]
    fn zero_and_out_of_range_stars_are_unrated() {
        for rating in [None, Some(0), Some(-1), Some(6)] {
            assert_eq!(State::from(&file(rating, Flag::None, None)).rating, None);
        }
    }
}
