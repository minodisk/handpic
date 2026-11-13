//! A key / JSON-value settings store over one SQLite file.
//!
//! Unlike the index, which is a cache, the store is never deleted or
//! recreated: a file that is not a database, or one written by a newer schema
//! version, is an error and is left as it was.

use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection, TransactionBehavior};
use serde_json::{Map, Value};

/// Stored in `PRAGMA user_version`; `0` is a store with no schema yet.
const SCHEMA_VERSION: i64 = 1;

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open the store at `path`, creating it and its parent folder when
    /// missing. Only a new store (no schema yet, which covers a missing file
    /// and a zero-byte one) calls `seed`, and its values are committed with
    /// the schema in one transaction, each key verbatim.
    pub fn open(path: &Path, seed: impl FnOnce() -> Map<String, Value>) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let mut conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.busy_timeout(Duration::from_secs(1))
            .map_err(|e| e.to_string())?;
        // Checked before the journal mode is set, so a newer store's header is
        // not touched.
        check_version(user_version(&conn)?)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| e.to_string())?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if check_version(user_version(&tx)?)? == 0 {
            tx.execute_batch("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
                .map_err(|e| e.to_string())?;
            for (key, value) in seed() {
                tx.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                    params![key, value.to_string()],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }

    /// Every stored value; a value that is not valid JSON is logged and left
    /// out.
    pub fn values(&self) -> Result<Map<String, Value>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut values = Map::new();
        for row in rows {
            let (key, text) = row.map_err(|e| e.to_string())?;
            match serde_json::from_str(&text) {
                Ok(value) => {
                    values.insert(key, value);
                }
                Err(e) => log::warn!("settings store: {key} is not JSON, ignored: {e}"),
            }
        }
        Ok(values)
    }

    pub fn set(&self, key: &str, value: &Value) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value.to_string()],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn delete(&self, key: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", params![key])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

fn user_version(conn: &Connection) -> Result<i64, String> {
    conn.pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())
}

fn check_version(version: i64) -> Result<i64, String> {
    if version > SCHEMA_VERSION {
        return Err(format!(
            "settings store schema version {version} is newer than {SCHEMA_VERSION}"
        ));
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::{Store, SCHEMA_VERSION};
    use rusqlite::Connection;
    use serde_json::{json, Map, Value};
    use std::path::{Path, PathBuf};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn seed(pairs: &[(&str, Value)]) -> Map<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn open_unseeded(path: &Path) -> Result<Store, String> {
        Store::open(path, || panic!("seeded an existing store"))
    }

    #[test]
    fn set_delete_and_read_round_trip() {
        let dir = temp_dir("round-trip");
        let path = dir.join("sub").join("settings.sqlite");
        let store = Store::open(&path, Map::new).unwrap();
        store.set("sidecarFormat", &json!("both")).unwrap();
        store.set("filter", &json!({"stars": [0, 3]})).unwrap();
        store.set("sidecarFormat", &json!("dop")).unwrap();
        store.set("autoAdvance", &json!(false)).unwrap();
        store.delete("autoAdvance").unwrap();
        store.delete("missing").unwrap();
        drop(store);
        let values = open_unseeded(&path).unwrap().values().unwrap();
        assert_eq!(
            values,
            seed(&[
                ("sidecarFormat", json!("dop")),
                ("filter", json!({"stars": [0, 3]})),
            ])
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_store_is_seeded_once_with_every_key_verbatim() {
        let dir = temp_dir("seed");
        let path = dir.join("settings.sqlite");
        let values = seed(&[
            ("sidecarFormat", json!("both")),
            ("unknownKey", json!({"a": [1, null, "x"]})),
        ]);
        let mut calls = 0;
        let store = Store::open(&path, || {
            calls += 1;
            values.clone()
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(store.values().unwrap(), values);
        drop(store);
        assert_eq!(open_unseeded(&path).unwrap().values().unwrap(), values);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_zero_byte_file_counts_as_new() {
        let dir = temp_dir("zero");
        let path = dir.join("settings.sqlite");
        std::fs::write(&path, b"").unwrap();
        let mut called = false;
        let store = Store::open(&path, || {
            called = true;
            seed(&[("autoAdvance", json!(false))])
        })
        .unwrap();
        assert!(called);
        assert_eq!(
            store.values().unwrap(),
            seed(&[("autoAdvance", json!(false))])
        );
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_json_file_is_an_error_and_left_unchanged() {
        let dir = temp_dir("json");
        let path = dir.join("settings.json");
        let bytes = br#"{"sidecarFormat":"both","sortOrder":"rating"}"#;
        std::fs::write(&path, bytes).unwrap();
        assert!(Store::open(&path, Map::new).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_newer_schema_version_is_refused_and_left_unchanged() {
        let dir = temp_dir("newer");
        let path = dir.join("settings.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        drop(conn);
        let bytes = std::fs::read(&path).unwrap();
        assert!(Store::open(&path, Map::new).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
