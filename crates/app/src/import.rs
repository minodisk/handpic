//! Import from a memory card: the destination root and the folder pattern
//! the photos are copied into, persisted under `importRoot` and
//! `importPattern`, and the pattern's validation.
//!
//! The pattern is chrono strftime syntax over the capture date, `/`
//! separating folders. Only date specifiers are allowed, and every folder
//! name it makes has to be valid on Windows and macOS.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::format::{Item, StrftimeItems};
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::Value;
use tauri::Manager;

use crate::index;

/// The pattern used when none is stored: `2026/2026-10-24`.
pub const DEFAULT_PATTERN: &str = "%Y/%F";

/// The stored import settings. An empty `root` is not set yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportSettings {
    pub root: String,
    pub pattern: String,
}

/// The import settings in use, loaded at launch and replaced by the setters.
pub struct AppImportSettings(pub Mutex<ImportSettings>);

/// `Ok` when `pattern` formats every date into a valid relative folder path.
pub fn validate_pattern(pattern: &str) -> Result<(), String> {
    let date = NaiveDate::from_ymd_opt(2026, 1, 2).expect("a valid date");
    format_date(pattern, date).map(|_| ())
}

/// `date` formatted by `pattern`, the relative folder path with `/` as the
/// separator, or why the pattern is refused.
pub fn format_date(pattern: &str, date: NaiveDate) -> Result<String, String> {
    if StrftimeItems::new(pattern).any(|item| matches!(item, Item::Error)) {
        return Err(format!(
            "`{pattern}` has an unknown specifier or a lone `%`"
        ));
    }
    // `to_string` panics on the formatting error a time specifier raises
    // without a time; `write!` returns it.
    let mut path = String::new();
    if write!(path, "{}", date.format(pattern)).is_err() {
        return Err(format!(
            "`{pattern}` uses a time, zone or timestamp specifier; only the date is known"
        ));
    }
    for segment in path.split('/') {
        check_segment(segment)?;
    }
    Ok(path)
}

/// Why `segment` cannot be a folder name on Windows or macOS, if it cannot.
fn check_segment(segment: &str) -> Result<(), String> {
    if segment.is_empty() {
        return Err("the pattern makes an empty folder name".to_string());
    }
    if segment == "." || segment == ".." {
        return Err(format!("`{segment}` cannot be a folder name"));
    }
    if let Some(c) = segment
        .chars()
        .find(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\') || c.is_control())
    {
        return Err(format!(
            "the folder name `{segment}` has {c:?}, which a folder name cannot have"
        ));
    }
    if segment.ends_with(' ') || segment.ends_with('.') {
        return Err(format!(
            "the folder name `{segment}` ends in a space or a dot"
        ));
    }
    Ok(())
}

/// The stored `importRoot` value; missing or non-string means not set.
fn root_setting(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The stored `importPattern` value; missing, non-string or invalid means
/// `DEFAULT_PATTERN`.
fn pattern_setting(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .filter(|pattern| validate_pattern(pattern).is_ok())
        .unwrap_or(DEFAULT_PATTERN)
        .to_string()
}

/// The import settings from the store at launch; a store that cannot be read
/// is logged and gives the defaults.
pub fn load_settings(app: &tauri::AppHandle) -> ImportSettings {
    let store = crate::commands::settings(app)
        .inspect_err(|e| log::warn!("failed to open the settings: {e}"))
        .ok();
    let get = |key: &str| store.as_ref().and_then(|store| store.get(key));
    ImportSettings {
        root: root_setting(get("importRoot").as_ref()),
        pattern: pattern_setting(get("importPattern").as_ref()),
    }
}

/// Persist `value` under `key`; a failure is logged and the in-memory change
/// stands.
fn save(app: &tauri::AppHandle, key: &str, value: &str) {
    let saved = crate::commands::settings(app).and_then(|store| {
        store.set(key, value);
        store.save().map_err(|e| e.to_string())
    });
    if let Err(e) = saved {
        log::warn!("failed to save {key}: {e}");
    }
}

/// `pattern` formatted for `date` and placed under `root` with the OS
/// separator; the relative path alone while `root` is not set.
fn example(root: &str, pattern: &str, date: NaiveDate) -> Result<String, String> {
    let relative = format_date(pattern, date)?;
    let mut path = PathBuf::from(root);
    path.extend(relative.split('/'));
    Ok(path.to_string_lossy().into_owned())
}

/// The destination root and the folder pattern.
#[tauri::command]
pub fn import_settings(app: tauri::AppHandle) -> ImportSettings {
    index::lock(&app.state::<AppImportSettings>().0).clone()
}

/// Set the destination root and persist it under `importRoot`.
#[tauri::command]
pub fn set_import_root(app: tauri::AppHandle, path: String) -> ImportSettings {
    let settings = {
        let state = app.state::<AppImportSettings>();
        let mut current = index::lock(&state.0);
        current.root = path;
        current.clone()
    };
    save(&app, "importRoot", &settings.root);
    settings
}

/// Set the folder pattern and persist it under `importPattern`; an invalid
/// pattern is refused with the reason and the stored one kept.
#[tauri::command]
pub fn set_import_pattern(
    app: tauri::AppHandle,
    pattern: String,
) -> Result<ImportSettings, String> {
    validate_pattern(&pattern)?;
    let settings = {
        let state = app.state::<AppImportSettings>();
        let mut current = index::lock(&state.0);
        current.pattern = pattern;
        current.clone()
    };
    save(&app, "importPattern", &settings.pattern);
    Ok(settings)
}

/// Where today's photos would go under `pattern` and the stored root, or why
/// the pattern is refused.
#[tauri::command]
pub fn import_pattern_example(app: tauri::AppHandle, pattern: String) -> Result<String, String> {
    let root = index::lock(&app.state::<AppImportSettings>().0)
        .root
        .clone();
    example(&root, &pattern, chrono::Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
    }

    #[test]
    fn formats_every_accepted_specifier() {
        for (pattern, expected) in [
            ("%Y", "2026"),
            ("%y", "26"),
            ("%m", "10"),
            ("%d", "24"),
            ("%b", "Oct"),
            ("%B", "October"),
            ("%a", "Sat"),
            ("%A", "Saturday"),
            ("%j", "297"),
            ("%F", "2026-10-24"),
            ("%Y/%F", "2026/2026-10-24"),
            ("%Y/%m/%d", "2026/10/24"),
            ("Photos %Y/%d %B trip", "Photos 2026/24 October trip"),
        ] {
            assert_eq!(
                format_date(pattern, date()).as_deref(),
                Ok(expected),
                "{pattern}"
            );
            assert_eq!(validate_pattern(pattern), Ok(()), "{pattern}");
        }
    }

    #[test]
    fn refuses_an_unknown_specifier_or_a_lone_percent() {
        for pattern in ["%Q", "%Y/%", "%Y/%L"] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(error.contains("unknown specifier"), "{pattern}: {error}");
        }
    }

    #[test]
    fn refuses_time_zone_and_timestamp_specifiers() {
        for pattern in [
            "%H", "%M", "%S", "%p", "%z", "%Z", "%s", "%T", "%R", "%c", "%+", "%I", "%f",
            "%Y/%F %H",
        ] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(
                error.contains("time, zone or timestamp"),
                "{pattern}: {error}"
            );
        }
    }

    #[test]
    fn refuses_an_empty_folder_name() {
        for pattern in ["", "/%Y", "%Y/", "%Y//%F"] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(error.contains("empty folder name"), "{pattern:?}: {error}");
        }
    }

    #[test]
    fn refuses_a_dot_or_dot_dot_folder() {
        for pattern in [".", "../%Y", "%Y/./%F"] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(
                error.contains("cannot be a folder name"),
                "{pattern}: {error}"
            );
        }
    }

    #[test]
    fn refuses_a_character_windows_or_macos_refuses() {
        for pattern in [
            "%Y<", "%Y>", "%Y:%m", "%Y\"", "%Y|", "%Y?", "%Y*", "%Y\\%F", "%Y%n", "%Y%t", "a\u{1}b",
        ] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(
                error.contains("which a folder name cannot have"),
                "{pattern:?}: {error}"
            );
        }
    }

    #[test]
    fn refuses_a_folder_ending_in_a_space_or_a_dot() {
        for pattern in ["%Y /%F", "%Y./%F", "%F."] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(
                error.contains("ends in a space or a dot"),
                "{pattern}: {error}"
            );
        }
    }

    #[test]
    fn root_setting_is_empty_unless_a_string() {
        assert_eq!(root_setting(None), "");
        assert_eq!(root_setting(Some(&json!(3))), "");
        assert_eq!(root_setting(Some(&json!("D:\\Photos"))), "D:\\Photos");
    }

    #[test]
    fn pattern_setting_falls_back_to_the_default() {
        assert_eq!(pattern_setting(None), DEFAULT_PATTERN);
        assert_eq!(pattern_setting(Some(&json!(true))), DEFAULT_PATTERN);
        assert_eq!(pattern_setting(Some(&json!("%H"))), DEFAULT_PATTERN);
        assert_eq!(pattern_setting(Some(&json!("%Y/%m"))), "%Y/%m");
    }

    #[test]
    fn example_joins_the_root_with_the_os_separator() {
        let root = std::env::temp_dir().join("Photos");
        let expected = root.join("2026").join("2026-10-24");
        assert_eq!(
            example(&root.to_string_lossy(), DEFAULT_PATTERN, date()),
            Ok(expected.to_string_lossy().into_owned())
        );
        let relative = PathBuf::from("2026").join("10");
        assert_eq!(
            example("", "%Y/%m", date()),
            Ok(relative.to_string_lossy().into_owned())
        );
        assert!(example("", "%H", date()).is_err());
    }
}
