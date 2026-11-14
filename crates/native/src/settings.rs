//! The native app's settings, kept in its own store
//! (`riffle_backend::store`) and read with the backend's rules, so a native
//! judgment writes the same sidecars with the same label names and keys as
//! the Tauri app. A new store is seeded once from the Tauri app's
//! `settings.json`, which is only ever read; after that the two apps'
//! settings diverge.

use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

use riffle_backend::{
    import, mcp,
    shortcuts::Keymap,
    sidecar::{self, SidecarFormat},
    store::Store,
};
use riffle_core::xmp::LabelNames;
use serde_json::{Map, Value};

use crate::{
    datadir::{self, IDENTIFIER, LEGACY_IDENTIFIERS},
    filter::Stored,
    panels::Panels,
    session::Geometry,
    sort::SortKey,
};

pub struct Settings {
    pub format: SidecarFormat,
    pub names: LabelNames,
    pub keymap: Keymap,
    pub auto_advance: bool,
    /// The strip's order at startup; the sort menu writes it back.
    pub sort: SortKey,
    /// The strip's filter at startup, its remembered sections; the filter
    /// menu writes them back.
    pub filter: Stored,
    /// The MCP companion's server is on (`mcpEnabled`, by the backend's
    /// rule).
    pub mcp_enabled: bool,
    /// The Import tab's destination root, empty while not set, and folder
    /// pattern, an invalid stored one read as the default.
    pub import_root: String,
    pub import_pattern: String,
    /// The folder of the previous session (`lastFolder`, the Tauri app's
    /// key), written on every open.
    pub last_folder: Option<String>,
    /// The window's geometry at the last quit (`windowBounds`).
    pub window: Option<Geometry>,
    /// The panes shown at startup; a pane toggle writes them back.
    pub panels: Panels,
    /// The writes to the store the values came from; a no-op when it did not
    /// open.
    pub writes: Writes,
}

/// How long the quit handler waits for the settings writes.
pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

enum Write {
    Set(String, Value),
    Delete(String),
    Barrier(mpsc::Sender<()>),
}

/// The settings writes, applied to the store by one thread in the order they
/// were given, so two quick changes of one key cannot land reversed. A failed
/// write is logged and the in-memory value stands, as in the Tauri app.
#[derive(Clone)]
pub struct Writes {
    jobs: Option<mpsc::Sender<Write>>,
}

impl Writes {
    /// The writer of `store`; without one every write is a no-op.
    fn spawn(store: Option<Store>) -> Self {
        let jobs = store.map(|store| {
            let (jobs, queued) = mpsc::channel::<Write>();
            thread::Builder::new()
                .name("settings-store".into())
                .spawn(move || {
                    for job in queued {
                        let (key, written) = match job {
                            Write::Set(key, value) => {
                                let written = store.set(&key, &value);
                                (key, written)
                            }
                            Write::Delete(key) => {
                                let written = store.delete(&key);
                                (key, written)
                            }
                            Write::Barrier(done) => {
                                done.send(()).ok();
                                continue;
                            }
                        };
                        if let Err(e) = written {
                            log::warn!("settings store: failed to write {key}: {e}");
                        }
                    }
                })
                .expect("spawn the settings store thread");
            jobs
        });
        Self { jobs }
    }

    /// Whether the store opened, so the writes land.
    pub fn is_open(&self) -> bool {
        self.jobs.is_some()
    }

    pub fn set(&self, key: &str, value: Value) {
        self.send(Write::Set(key.to_owned(), value));
    }

    pub fn delete(&self, key: &str) {
        self.send(Write::Delete(key.to_owned()));
    }

    /// `keymap`'s overrides under `shortcuts`, the key deleted when there are
    /// none, as the Tauri `update_keymap` saves them.
    pub fn shortcuts(&self, keymap: &Keymap) {
        let overrides = keymap.overrides();
        if overrides.as_object().is_some_and(|o| o.is_empty()) {
            self.delete("shortcuts");
        } else {
            self.set("shortcuts", overrides);
        }
    }

    fn send(&self, write: Write) {
        if let Some(jobs) = &self.jobs {
            jobs.send(write).ok();
        }
    }

    /// Waits until every write given so far has run, at most `timeout`.
    /// `false` when the wait timed out or the thread is gone; `true` without
    /// a store.
    pub fn drain(&self, timeout: Duration) -> bool {
        let Some(jobs) = &self.jobs else {
            return true;
        };
        let (done, reached) = mpsc::channel();
        if jobs.send(Write::Barrier(done)).is_err() {
            return false;
        }
        reached.recv_timeout(timeout).is_ok()
    }
}

/// The settings in the store at `path` (`--settings`), else in
/// `settings.sqlite` under the config dir's `IDENTIFIER` folder. A new store
/// is seeded from `import` (`--import-settings`) when given; else, only
/// without `path`, from the first Tauri `settings.json` that exists under the
/// config dir, the `IDENTIFIER` folder's before the legacy ones. A store that
/// cannot be opened gives the defaults.
pub fn load(path: Option<PathBuf>, import: Option<&Path>) -> Settings {
    load_from(path, import, dirs::config_dir().as_deref())
}

fn load_from(path: Option<PathBuf>, import: Option<&Path>, config: Option<&Path>) -> Settings {
    let sources = seed_sources(path.is_some(), import, config);
    let Some(path) = datadir::settings_path(path, config) else {
        log::warn!("no config dir to keep the settings store in, using the defaults");
        return from_values(&Map::new());
    };
    let mut seeded = None;
    let opened = Store::open(&path, || {
        let (note, values) = seed(&sources);
        seeded = Some(note);
        values
    })
    .and_then(|store| store.values().map(|values| (store, values)));
    match opened {
        Ok((store, values)) => {
            log::info!("settings store: {}", path.display());
            if let Some(note) = seeded {
                log::info!("settings store: new, {note}");
            }
            Settings {
                writes: Writes::spawn(Some(store)),
                ..from_values(&values)
            }
        }
        Err(e) => {
            log::warn!(
                "failed to open the settings store at {}, using the defaults: {e}",
                path.display()
            );
            from_values(&Map::new())
        }
    }
}

fn seed_sources(has_path: bool, import: Option<&Path>, config: Option<&Path>) -> Vec<PathBuf> {
    if let Some(import) = import {
        return vec![import.to_path_buf()];
    }
    let (false, Some(config)) = (has_path, config) else {
        return Vec::new();
    };
    std::iter::once(IDENTIFIER)
        .chain(LEGACY_IDENTIFIERS.iter().copied())
        .map(|id| config.join(id).join("settings.json"))
        .collect()
}

/// What a new store is seeded with, and the log note saying from where.
fn seed(sources: &[PathBuf]) -> (String, Map<String, Value>) {
    let Some(found) = sources.iter().find(|p| p.exists()) else {
        let tried: Vec<String> = sources.iter().map(|p| p.display().to_string()).collect();
        return (
            format!("nothing to seed; tried: {}", tried.join(", ")),
            Map::new(),
        );
    };
    match read(found) {
        Ok(values) => (
            format!("seeded {} keys from {}", values.len(), found.display()),
            values,
        ),
        Err(e) => (
            format!("nothing seeded, failed to read {}: {e}", found.display()),
            Map::new(),
        ),
    }
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
        mcp_enabled: mcp::mcp_enabled_setting(values.get("mcpEnabled")),
        import_root: import::root_setting(values.get("importRoot")),
        import_pattern: import::pattern_setting(values.get("importPattern")),
        last_folder: values
            .get("lastFolder")
            .and_then(Value::as_str)
            .map(str::to_owned),
        window: Geometry::from_setting(values.get("windowBounds")),
        panels: Panels::from_setting(values.get("panels")),
        writes: Writes::spawn(None),
    }
}

#[cfg(test)]
mod tests {
    use super::{Settings, Writes, load_from};
    use crate::{
        datadir::{IDENTIFIER, LEGACY_IDENTIFIERS},
        filter::{FilterFlag, Stored},
        panels::Panels,
        session::{Geometry, Rect},
        sort::SortKey,
    };
    use riffle_backend::{shortcuts::Keymap, sidecar::SidecarFormat, store::Store};
    use riffle_core::xmp::LabelNames;
    use serde_json::{Map, Value, json};
    use std::{
        fs,
        path::{Path, PathBuf},
        time::Duration,
    };

    /// What the existing store at `path` holds.
    fn values(path: &Path) -> Map<String, Value> {
        Store::open(path, || panic!("seeded an existing store"))
            .unwrap()
            .values()
            .unwrap()
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn tauri_json(config: &Path, id: &str) -> PathBuf {
        config.join(id).join("settings.json")
    }

    /// The settings of a fresh store `<name>.sqlite` in `dir` seeded from
    /// `<name>.json` holding `text`.
    fn seeded(dir: &Path, name: &str, text: &str) -> Settings {
        let json = dir.join(format!("{name}.json"));
        write(&json, text);
        load_from(Some(dir.join(format!("{name}.sqlite"))), Some(&json), None)
    }

    #[test]
    fn reads_the_stored_values() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"sidecarFormat":"both","labelNames":{"red":"Rot"},"autoAdvance":false,
                "shortcuts":{"pick":["k"]},"sortOrder":"rating",
                "filter":{"flags":["untagged"],"stars":[0,9]}}"#;
        drop(seeded(dir.path(), "s", text));
        fs::remove_file(dir.path().join("s.json")).unwrap();
        let s = load_from(Some(dir.path().join("s.sqlite")), None, None);
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
        assert!(s.writes.is_open());
    }

    #[test]
    fn a_malformed_or_missing_seed_gives_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let malformed = seeded(dir.path(), "malformed", "{\"sidecarFormat\": ");
        let missing = load_from(
            Some(dir.path().join("missing.sqlite")),
            Some(&dir.path().join("none.json")),
            None,
        );
        for (s, name) in [(malformed, "malformed"), (missing, "missing")] {
            assert_eq!(s.format, SidecarFormat::Xmp);
            assert_eq!(s.names, LabelNames::default());
            assert_eq!(s.keymap, Keymap::defaults());
            assert!(s.auto_advance);
            assert_eq!(s.sort, SortKey::Name);
            assert_eq!(s.filter, Stored::default());
            assert!(values(&dir.path().join(format!("{name}.sqlite"))).is_empty());
        }
    }

    #[test]
    fn mcp_is_on_only_for_a_stored_true() {
        let dir = tempfile::tempdir().unwrap();
        for (i, (stored, on)) in [
            (r#"{"mcpEnabled":true}"#, true),
            (r#"{"mcpEnabled":false}"#, false),
            (r#"{"mcpEnabled":"true"}"#, false),
            (r#"{}"#, false),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(seeded(dir.path(), &i.to_string(), stored).mcp_enabled, on);
        }
    }

    #[test]
    fn the_import_settings_are_read_with_the_backend_rules() {
        let dir = tempfile::tempdir().unwrap();
        let s = seeded(
            dir.path(),
            "valid",
            r#"{"importRoot":"D:\\Photos","importPattern":"%Y/%m"}"#,
        );
        assert_eq!(
            (s.import_root.as_str(), s.import_pattern.as_str()),
            ("D:\\Photos", "%Y/%m")
        );
        let s = seeded(
            dir.path(),
            "invalid",
            r#"{"importRoot":3,"importPattern":"%H"}"#,
        );
        assert_eq!(
            (s.import_root.as_str(), s.import_pattern.as_str()),
            ("", "%Y/%F")
        );
    }

    #[test]
    fn the_session_keys_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let s = seeded(
            dir.path(),
            "valid",
            r#"{"lastFolder":"D:\\Photos","windowBounds":{"x":10,"y":20,"width":800,"height":600,"maximized":true,"display":"u"}}"#,
        );
        assert_eq!(s.last_folder.as_deref(), Some("D:\\Photos"));
        assert_eq!(
            s.window,
            Some(Geometry {
                bounds: Rect {
                    x: 10.,
                    y: 20.,
                    width: 800.,
                    height: 600.
                },
                maximized: true,
                display: Some("u".into()),
            })
        );
        let s = seeded(
            dir.path(),
            "invalid",
            r#"{"lastFolder":3,"windowBounds":{"x":10}}"#,
        );
        assert_eq!((s.last_folder, s.window), (None, None));
    }

    #[test]
    fn the_panels_are_read_with_their_rule() {
        let dir = tempfile::tempdir().unwrap();
        let s = seeded(dir.path(), "left", r#"{"panels":{"left":false}}"#);
        assert_eq!(
            s.panels,
            Panels {
                left: false,
                strip: true,
                right: true,
            }
        );
        let s = seeded(
            dir.path(),
            "odd",
            r#"{"panels":{"left":"no","strip":false,"right":null}}"#,
        );
        assert_eq!(
            s.panels,
            Panels {
                left: true,
                strip: false,
                right: true,
            }
        );
        assert_eq!(seeded(dir.path(), "none", "{}").panels, Panels::default());
    }

    #[test]
    fn a_non_boolean_auto_advance_is_on() {
        let dir = tempfile::tempdir().unwrap();
        assert!(seeded(dir.path(), "s", r#"{"autoAdvance":"no"}"#).auto_advance);
    }

    #[test]
    fn a_stored_recommended_or_unknown_sort_order_is_mapped() {
        let dir = tempfile::tempdir().unwrap();
        for (i, (stored, key)) in [
            (r#"{"sortOrder":"capture"}"#, SortKey::Capture),
            (r#"{"sortOrder":"recommended"}"#, SortKey::Capture),
            (r#"{"sortOrder":"size"}"#, SortKey::Name),
            (r#"{"sortOrder":3}"#, SortKey::Name),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(seeded(dir.path(), &i.to_string(), stored).sort, key);
        }
    }

    #[test]
    fn a_stored_recommended_is_not_rewritten_by_the_load() {
        let dir = tempfile::tempdir().unwrap();
        drop(seeded(dir.path(), "s", r#"{"sortOrder":"recommended"}"#));
        let stored = values(&dir.path().join("s.sqlite"));
        assert_eq!(stored.get("sortOrder"), Some(&json!("recommended")));
    }

    #[test]
    fn the_import_flag_wins_over_the_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        write(
            &tauri_json(&config, IDENTIFIER),
            r#"{"sidecarFormat":"dop"}"#,
        );
        let json = dir.path().join("s.json");
        write(&json, r#"{"sidecarFormat":"both"}"#);
        assert_eq!(
            load_from(None, Some(&json), Some(&config)).format,
            SidecarFormat::Both
        );
        assert!(config.join(IDENTIFIER).join("settings.sqlite").exists());
    }

    #[test]
    fn a_settings_flag_without_an_import_never_looks_under_the_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        write(
            &tauri_json(&config, IDENTIFIER),
            r#"{"sidecarFormat":"dop"}"#,
        );
        let s = load_from(Some(dir.path().join("s.sqlite")), None, Some(&config));
        assert_eq!(s.format, SidecarFormat::Xmp);
        assert!(values(&dir.path().join("s.sqlite")).is_empty());
        assert!(!config.join(IDENTIFIER).join("settings.sqlite").exists());
    }

    #[test]
    fn the_current_identifier_wins_over_the_legacy_ones() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &tauri_json(dir.path(), IDENTIFIER),
            r#"{"sidecarFormat":"dop"}"#,
        );
        write(
            &tauri_json(dir.path(), LEGACY_IDENTIFIERS[0]),
            r#"{"sidecarFormat":"xmp"}"#,
        );
        write(
            &tauri_json(dir.path(), LEGACY_IDENTIFIERS[1]),
            r#"{"sidecarFormat":"both"}"#,
        );
        assert_eq!(
            load_from(None, None, Some(dir.path())).format,
            SidecarFormat::Dop
        );
    }

    #[test]
    fn the_newer_legacy_file_wins_when_the_current_one_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &tauri_json(dir.path(), LEGACY_IDENTIFIERS[0]),
            r#"{"sidecarFormat":"dop"}"#,
        );
        write(
            &tauri_json(dir.path(), LEGACY_IDENTIFIERS[1]),
            r#"{"sidecarFormat":"both"}"#,
        );
        assert_eq!(
            load_from(None, None, Some(dir.path())).format,
            SidecarFormat::Dop
        );
    }

    #[test]
    fn the_riffle_file_is_used_when_only_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &tauri_json(dir.path(), LEGACY_IDENTIFIERS[1]),
            r#"{"sidecarFormat":"both"}"#,
        );
        assert_eq!(
            load_from(None, None, Some(dir.path())).format,
            SidecarFormat::Both
        );
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(
            load_from(None, None, Some(empty.path())).format,
            SidecarFormat::Xmp
        );
    }

    #[test]
    fn a_second_load_does_not_reread_a_changed_seed() {
        let dir = tempfile::tempdir().unwrap();
        let json = dir.path().join("s.json");
        let store = dir.path().join("s.sqlite");
        write(&json, r#"{"sidecarFormat":"both"}"#);
        drop(load_from(Some(store.clone()), Some(&json), None));
        write(&json, r#"{"sidecarFormat":"dop"}"#);
        assert_eq!(
            load_from(Some(store), Some(&json), None).format,
            SidecarFormat::Both
        );
    }

    #[test]
    fn the_seed_file_is_left_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let json = dir.path().join("s.json");
        let text = r#"{"sidecarFormat":"both","sortOrder":"rating"}"#;
        write(&json, text);
        drop(load_from(
            Some(dir.path().join("s.sqlite")),
            Some(&json),
            None,
        ));
        assert_eq!(fs::read_to_string(&json).unwrap(), text);
    }

    #[test]
    fn a_store_that_does_not_open_gives_the_defaults_and_no_handle() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.json");
        let text = r#"{"sidecarFormat":"both"}"#;
        write(&path, text);
        let s = load_from(Some(path.clone()), None, None);
        assert_eq!(s.format, SidecarFormat::Xmp);
        assert!(!s.writes.is_open());
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }

    #[test]
    fn the_writes_land_in_order_and_the_drain_waits_for_them() {
        let dir = tempfile::tempdir().unwrap();
        let s = seeded(dir.path(), "s", r#"{"autoAdvance":false,"filter":{}}"#);
        for i in 0..50 {
            s.writes.set("sortOrder", json!(i));
        }
        s.writes.set("sortOrder", json!("rating"));
        s.writes.set("labelNames", json!({"red": "Rot"}));
        s.writes.delete("autoAdvance");
        s.writes.delete("missing");
        assert!(s.writes.drain(Duration::from_secs(5)));
        let stored = values(&dir.path().join("s.sqlite"));
        assert_eq!(stored.get("sortOrder"), Some(&json!("rating")));
        assert_eq!(stored.get("labelNames"), Some(&json!({"red": "Rot"})));
        assert_eq!(stored.get("filter"), Some(&json!({})));
        assert!(!stored.contains_key("autoAdvance"));
    }

    #[test]
    fn the_shortcuts_are_stored_as_overrides_and_deleted_without_any() {
        let dir = tempfile::tempdir().unwrap();
        let s = seeded(dir.path(), "s", r#"{"shortcuts":{"pick":["k"]}}"#);
        let path = dir.path().join("s.sqlite");
        let mut keymap = Keymap::defaults();
        keymap.add("reject", "j").unwrap();
        s.writes.shortcuts(&keymap);
        assert!(s.writes.drain(Duration::from_secs(5)));
        assert_eq!(
            values(&path).get("shortcuts"),
            Some(&json!({"reject": ["x", "j"]}))
        );
        keymap.reset_all();
        s.writes.shortcuts(&keymap);
        assert!(s.writes.drain(Duration::from_secs(5)));
        assert!(!values(&path).contains_key("shortcuts"));
        let reloaded = load_from(Some(path), None, None);
        assert_eq!(reloaded.keymap, Keymap::defaults());
    }

    #[test]
    fn without_a_store_every_write_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.json");
        let text = r#"{"sidecarFormat":"both"}"#;
        write(&path, text);
        let s = load_from(Some(path.clone()), None, None);
        s.writes.set("sortOrder", json!("rating"));
        s.writes.delete("sidecarFormat");
        assert!(s.writes.drain(Duration::from_secs(5)));
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
        let none = Writes::spawn(None);
        none.set("sortOrder", json!("rating"));
        assert!(none.drain(Duration::ZERO));
    }
}
