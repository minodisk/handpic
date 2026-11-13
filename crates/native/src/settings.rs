//! The Tauri app's settings the culling needs, read from its `settings.json`
//! read-only with the backend's rules, so a native judgment writes the same
//! sidecars with the same label names and keys. The native app never writes
//! the file; its settings UI is port Step 9.

use std::path::{Path, PathBuf};

use riffle_backend::{
    shortcuts::Keymap,
    sidecar::{self, SidecarFormat},
};
use riffle_core::xmp::LabelNames;
use serde_json::{Map, Value};

use crate::{
    datadir::{IDENTIFIER, LEGACY_IDENTIFIERS},
    filter::Stored,
    sort::SortKey,
};

pub struct Settings {
    pub format: SidecarFormat,
    pub names: LabelNames,
    pub keymap: Keymap,
    pub auto_advance: bool,
    /// The strip's order at startup; the native menu's changes are not
    /// written back.
    pub sort: SortKey,
    /// The strip's filter at startup, its remembered sections; likewise not
    /// written back.
    pub filter: Stored,
}

/// The settings in `path` (`--settings`), else in the first `settings.json`
/// that exists under the config dir, the current identifier's folder before
/// the legacy ones. A missing or unreadable file gives the defaults.
pub fn load(path: Option<&Path>) -> Settings {
    load_from(path, dirs::config_dir().as_deref())
}

fn load_from(path: Option<&Path>, config: Option<&Path>) -> Settings {
    let tried = candidates(path, config);
    let Some(found) = tried.iter().find(|p| p.exists()) else {
        let tried: Vec<String> = tried.iter().map(|p| p.display().to_string()).collect();
        log::warn!(
            "no settings file, using the defaults; tried: {}",
            tried.join(", ")
        );
        return from_values(&Map::new());
    };
    match read(found) {
        Ok(values) => {
            log::info!("settings: {}", found.display());
            from_values(&values)
        }
        Err(e) => {
            log::warn!(
                "failed to read the settings at {}, using the defaults: {e}",
                found.display()
            );
            from_values(&Map::new())
        }
    }
}

fn candidates(path: Option<&Path>, config: Option<&Path>) -> Vec<PathBuf> {
    if let Some(path) = path {
        return vec![path.to_path_buf()];
    }
    let Some(config) = config else {
        return Vec::new();
    };
    std::iter::once(IDENTIFIER)
        .chain(LEGACY_IDENTIFIERS.iter().copied())
        .map(|id| config.join(id).join("settings.json"))
        .collect()
}

fn read(path: &Path) -> Result<Map<String, Value>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    match serde_json::from_str(&text).map_err(|e| e.to_string())? {
        Value::Object(values) => Ok(values),
        _ => Err("not a JSON object".into()),
    }
}

fn from_values(values: &Map<String, Value>) -> Settings {
    Settings {
        format: SidecarFormat::from_setting(values.get("sidecarFormat").and_then(Value::as_str)),
        names: sidecar::label_names_setting(values.get("labelNames")),
        keymap: Keymap::from_overrides(values.get("shortcuts")),
        auto_advance: values
            .get("autoAdvance")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        sort: SortKey::from_setting(values.get("sortOrder").and_then(Value::as_str)),
        filter: Stored::from_setting(values.get("filter")),
    }
}

#[cfg(test)]
mod tests {
    use super::load_from;
    use crate::{
        filter::{FilterFlag, Stored},
        sort::SortKey,
    };
    use riffle_backend::{shortcuts::Keymap, sidecar::SidecarFormat};
    use riffle_core::xmp::LabelNames;
    use std::{fs, path::Path};

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn reads_the_stored_values() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("s.json");
        write(
            &file,
            r#"{"sidecarFormat":"both","labelNames":{"red":"Rot"},"autoAdvance":false,
                "shortcuts":{"pick":["k"]},"sortOrder":"rating",
                "filter":{"flags":["untagged"],"stars":[0,9]}}"#,
        );
        let s = load_from(Some(&file), None);
        assert_eq!(s.format, SidecarFormat::Both);
        assert_eq!(s.names.red, "Rot");
        assert_eq!(s.names.blue, LabelNames::default().blue);
        assert!(!s.auto_advance);
        assert_eq!(s.sort, SortKey::Rating);
        assert_eq!(
            s.filter,
            Stored {
                flags: vec![FilterFlag::Untagged],
                stars: vec![0],
                ..Stored::default()
            }
        );
        let pick = s.keymap.bindings().into_iter().find(|b| b.action == "pick");
        assert_eq!(pick.unwrap().keys, ["k"]);
    }

    #[test]
    fn a_malformed_or_missing_file_gives_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("s.json");
        write(&file, "{\"sidecarFormat\": ");
        for path in [file, dir.path().join("none.json")] {
            let s = load_from(Some(&path), None);
            assert_eq!(s.format, SidecarFormat::Xmp);
            assert_eq!(s.names, LabelNames::default());
            assert_eq!(s.keymap, Keymap::defaults());
            assert!(s.auto_advance);
            assert_eq!(s.sort, SortKey::Name);
            assert_eq!(s.filter, Stored::default());
        }
    }

    #[test]
    fn a_non_boolean_auto_advance_is_on() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("s.json");
        write(&file, r#"{"autoAdvance":"no"}"#);
        assert!(load_from(Some(&file), None).auto_advance);
    }

    #[test]
    fn a_stored_recommended_or_unknown_sort_order_is_mapped() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("s.json");
        for (stored, key) in [
            (r#"{"sortOrder":"capture"}"#, SortKey::Capture),
            (r#"{"sortOrder":"recommended"}"#, SortKey::Capture),
            (r#"{"sortOrder":"size"}"#, SortKey::Name),
            (r#"{"sortOrder":3}"#, SortKey::Name),
        ] {
            write(&file, stored);
            assert_eq!(load_from(Some(&file), None).sort, key);
        }
    }

    #[test]
    fn the_override_path_wins_over_the_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        write(
            &config.join("com.minodisk.handpic").join("settings.json"),
            r#"{"sidecarFormat":"dop"}"#,
        );
        let file = dir.path().join("s.json");
        write(&file, r#"{"sidecarFormat":"both"}"#);
        assert_eq!(
            load_from(Some(&file), Some(&config)).format,
            SidecarFormat::Both
        );
    }

    #[test]
    fn the_current_identifier_wins_over_the_legacy_one() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path()
                .join("com.minodisk.handpic")
                .join("settings.json"),
            r#"{"sidecarFormat":"dop"}"#,
        );
        write(
            &dir.path().join("com.minodisk.riffle").join("settings.json"),
            r#"{"sidecarFormat":"both"}"#,
        );
        assert_eq!(load_from(None, Some(dir.path())).format, SidecarFormat::Dop);
    }

    #[test]
    fn the_legacy_file_is_used_when_only_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("com.minodisk.riffle").join("settings.json"),
            r#"{"sidecarFormat":"both"}"#,
        );
        assert_eq!(
            load_from(None, Some(dir.path())).format,
            SidecarFormat::Both
        );
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(
            load_from(None, Some(empty.path())).format,
            SidecarFormat::Xmp
        );
    }
}
