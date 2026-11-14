//! Why the preview area shows no photo, a port of `empty.ts`: no folder is
//! open, the folder has no RAW or JPEG files, the filter hides every file, or
//! the current file's preview could not be shown.

use riffle_backend::shortcuts::Binding;

use crate::{filter::FILTERED_TEXT, settingsmodal::display_key};

pub const NO_FILES_TEXT: &str = "This folder has no RAW or JPEG files.";
pub const PREVIEW_FAILED_TEXT: &str = "The preview of this file could not be shown.";

const OPEN_TEXT: &str = "Drop a folder onto the window, or click here to choose one.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    None,
    NoFolder,
    NoFiles,
    Filtered,
    PreviewFailed,
}

/// The state from the open folder, the listed and shown file counts and
/// whether the current file's preview failed, the earlier cases winning.
pub fn state(open: bool, total: usize, shown: usize, preview_failed: bool) -> State {
    if !open {
        State::NoFolder
    } else if total == 0 {
        State::NoFiles
    } else if shown == 0 {
        State::Filtered
    } else if preview_failed {
        State::PreviewFailed
    } else {
        State::None
    }
}

/// The no-folder message, naming the `open` action's keys when it has any.
pub fn open_hint(bindings: &[Binding]) -> String {
    let keys = bindings
        .iter()
        .find(|b| b.action == "open")
        .map(|b| b.keys.as_slice())
        .unwrap_or_default();
    if keys.is_empty() {
        return OPEN_TEXT.to_string();
    }
    let keys: Vec<&str> = keys.iter().map(|k| display_key(k)).collect();
    format!("{OPEN_TEXT} Or press {}.", keys.join(" or "))
}

/// The state's message, `None` for no message.
pub fn text(state: State, bindings: &[Binding]) -> Option<String> {
    match state {
        State::None => None,
        State::NoFolder => Some(open_hint(bindings)),
        State::NoFiles => Some(NO_FILES_TEXT.to_string()),
        State::Filtered => Some(FILTERED_TEXT.to_string()),
        State::PreviewFailed => Some(PREVIEW_FAILED_TEXT.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{State, open_hint, state, text};
    use crate::filter::FILTERED_TEXT;
    use riffle_backend::shortcuts::Binding;

    fn open(keys: &[&str]) -> Vec<Binding> {
        vec![Binding {
            action: "open",
            keys: keys.iter().map(|k| k.to_string()).collect(),
        }]
    }

    #[test]
    fn no_folder_open() {
        assert_eq!(state(false, 0, 0, false), State::NoFolder);
    }

    #[test]
    fn a_folder_without_raw_or_jpeg_files() {
        assert_eq!(state(true, 0, 0, false), State::NoFiles);
    }

    #[test]
    fn every_file_filtered_out() {
        assert_eq!(state(true, 12, 0, false), State::Filtered);
    }

    #[test]
    fn files_to_show() {
        assert_eq!(state(true, 12, 3, false), State::None);
    }

    #[test]
    fn the_current_files_preview_failed() {
        assert_eq!(state(true, 12, 3, true), State::PreviewFailed);
    }

    #[test]
    fn the_other_states_win_over_a_failed_preview() {
        assert_eq!(state(false, 0, 0, true), State::NoFolder);
        assert_eq!(state(true, 0, 0, true), State::NoFiles);
        assert_eq!(state(true, 12, 0, true), State::Filtered);
    }

    #[test]
    fn the_default_key() {
        assert_eq!(
            open_hint(&open(&["o"])),
            "Drop a folder onto the window, or click here to choose one. Or press o."
        );
    }

    #[test]
    fn several_keys() {
        assert_eq!(
            open_hint(&open(&["o", "ctrl+o"])),
            "Drop a folder onto the window, or click here to choose one. Or press o or ctrl+o."
        );
    }

    #[test]
    fn a_rebound_key() {
        assert_eq!(
            open_hint(&open(&["space"])),
            "Drop a folder onto the window, or click here to choose one. Or press Space."
        );
    }

    #[test]
    fn no_key_bound() {
        assert_eq!(
            open_hint(&open(&[])),
            "Drop a folder onto the window, or click here to choose one."
        );
    }

    #[test]
    fn no_open_binding() {
        assert_eq!(
            open_hint(&[]),
            "Drop a folder onto the window, or click here to choose one."
        );
    }

    #[test]
    fn each_states_text() {
        let bindings = open(&["o"]);
        assert_eq!(text(State::None, &bindings), None);
        assert_eq!(
            text(State::NoFolder, &bindings).as_deref(),
            Some("Drop a folder onto the window, or click here to choose one. Or press o.")
        );
        assert_eq!(
            text(State::NoFiles, &bindings).as_deref(),
            Some("This folder has no RAW or JPEG files.")
        );
        assert_eq!(
            text(State::Filtered, &bindings).as_deref(),
            Some(FILTERED_TEXT)
        );
        assert_eq!(
            text(State::PreviewFailed, &bindings).as_deref(),
            Some("The preview of this file could not be shown.")
        );
    }
}
