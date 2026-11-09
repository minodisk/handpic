//! The DxO PhotoLab database lookup behind a fresh `.dop`. PhotoLab matches
//! sidecar items to its database by Uuid, so a fresh sidecar with random
//! Uuids for an image PhotoLab has already registered is imported as a
//! virtual copy. Reusing the registered Source Uuid and master Item Uuid makes
//! it apply to the master instead. Windows and macOS; on any other platform
//! every fresh `.dop` gets random Uuids.

use riffle_core::dop::Uuids;
use std::path::Path;

/// The Uuids PhotoLab registered for `arw`, `None` when PhotoLab's database
/// cannot be found or does not know the file.
#[cfg(any(windows, target_os = "macos"))]
pub fn registered_uuids(arw: &Path) -> Option<Uuids> {
    lookup(&database_path()?, arw)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn registered_uuids(_arw: &Path) -> Option<Uuids> {
    None
}

/// The newest `%APPDATA%\DxO\DxO PhotoLab N\Database\PhotoLab.db` that exists.
#[cfg(windows)]
fn database_path() -> Option<std::path::PathBuf> {
    let dxo = Path::new(&std::env::var_os("APPDATA")?).join("DxO");
    std::fs::read_dir(&dxo)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let version = version(entry.file_name().to_str()?)?;
            let db = entry.path().join("Database").join("PhotoLab.db");
            db.is_file().then_some((version, db))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, db)| db)
}

/// `N` of a `DxO PhotoLab N` directory name.
#[cfg(windows)]
fn version(name: &str) -> Option<u32> {
    let n = name.strip_prefix("DxO PhotoLab ")?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    n.parse().ok()
}

/// The newest `~/Library/DxO PhotoLab vN/DOPDatabaseVN.dopdata` that exists.
#[cfg(target_os = "macos")]
fn database_path() -> Option<std::path::PathBuf> {
    newest_database(&Path::new(&std::env::var_os("HOME")?).join("Library"))
}

/// The newest `DxO PhotoLab vN/DOPDatabaseVN.dopdata` under `library`.
#[cfg(target_os = "macos")]
fn newest_database(library: &Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(library)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let version = version(entry.file_name().to_str()?)?;
            let db = entry.path().join(format!("DOPDatabaseV{version}.dopdata"));
            db.is_file().then_some((version, db))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, db)| db)
}

/// `N` of a `DxO PhotoLab vN` directory name.
#[cfg(target_os = "macos")]
fn version(name: &str) -> Option<u32> {
    let n = name.strip_prefix("DxO PhotoLab v")?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    n.parse().ok()
}

/// How long one lookup attempt waits on a busy database.
#[cfg(any(windows, target_os = "macos"))]
const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);

/// How many attempts a lookup makes while the database stays busy or locked.
#[cfg(any(windows, target_os = "macos"))]
const BUSY_ATTEMPTS: u32 = 6;

/// The Source Uuid and master Item Uuid `db` holds for `arw`, `None` on any
/// failure or when the file is not registered. A busy or locked database is
/// retried up to `BUSY_ATTEMPTS` times; a final busy miss is logged at warn
/// level, any other failure (rather than a plain miss) at debug level.
#[cfg(any(windows, target_os = "macos"))]
pub fn lookup(db: &Path, arw: &Path) -> Option<Uuids> {
    let mut attempt = 1;
    loop {
        match query(db, arw) {
            Ok(uuids) => return uuids,
            Err(e) if is_busy(&e) && attempt < BUSY_ATTEMPTS => attempt += 1,
            Err(e) if is_busy(&e) => {
                log::warn!(
                    "PhotoLab lookup of {} in {} failed after {attempt} attempts: {e}",
                    arw.display(),
                    db.display()
                );
                return None;
            }
            Err(e) => {
                log::debug!(
                    "PhotoLab lookup of {} in {} failed: {e}",
                    arw.display(),
                    db.display()
                );
                return None;
            }
        }
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn is_busy(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked,
                ..
            },
            _
        )
    )
}

/// `db` opened read-only, waiting at most `BUSY_TIMEOUT` on a busy database.
#[cfg(any(windows, target_os = "macos"))]
fn open(db: &Path) -> rusqlite::Result<rusqlite::Connection> {
    use rusqlite::{Connection, OpenFlags};

    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}

/// The folder is walked from its drive root one `Folders` row per path
/// component; names compare through the columns' `COLLATE NOCASE`. A path
/// that maps to more than one source (two removable volumes registered under
/// the same drive letter) is treated as unregistered.
#[cfg(windows)]
fn query(db: &Path, arw: &Path) -> rusqlite::Result<Option<Uuids>> {
    use rusqlite::OptionalExtension;

    let Some(parts) = folder_names(arw) else {
        return Ok(None);
    };
    let Some(file) = arw.file_name().and_then(|n| n.to_str()) else {
        return Ok(None);
    };
    let conn = open(db)?;

    let mut roots =
        conn.prepare("SELECT Id FROM Folders WHERE ParentFolderId IS NULL AND Name = ?")?;
    let mut children =
        conn.prepare("SELECT Id FROM Folders WHERE ParentFolderId = ? AND Name = ?")?;
    let mut sources =
        conn.prepare("SELECT Id, Uuid FROM Sources WHERE FolderId = ? AND Name = ?")?;
    let mut master =
        conn.prepare("SELECT Uuid FROM Items WHERE SourceId = ? ORDER BY Id LIMIT 1")?;

    let (root, rest) = parts.split_first().expect("folder_names yields the root");
    let mut folders = roots
        .query_map([root], |row| row.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for name in rest {
        let mut next = Vec::new();
        for parent in &folders {
            for id in children.query_map(rusqlite::params![parent, name], |row| row.get(0))? {
                next.push(id?);
            }
        }
        folders = next;
    }
    let mut found = Vec::new();
    for folder in &folders {
        for source in sources.query_map(rusqlite::params![folder, file], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })? {
            found.push(source?);
        }
    }
    let [(source_id, source)] = found.as_slice() else {
        return Ok(None);
    };
    let Some(item) = master
        .query_row([source_id], |row| row.get::<_, String>(0))
        .optional()?
    else {
        return Ok(None);
    };
    Ok((is_uuid(&item) && is_uuid(source)).then(|| Uuids {
        item,
        source: source.clone(),
    }))
}

/// The `Folders` names of `arw`'s parent: the drive (`D:`) then each
/// directory. `None` for a path not under a plain drive letter.
#[cfg(windows)]
fn folder_names(arw: &Path) -> Option<Vec<String>> {
    use std::path::{Component, Prefix};

    let mut names = Vec::new();
    for component in arw.parent()?.components() {
        match component {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                    names.push(format!("{}:", letter as char));
                }
                _ => return None,
            },
            Component::RootDir => {}
            Component::Normal(name) => names.push(name.to_str()?.to_owned()),
            Component::CurDir | Component::ParentDir => return None,
        }
    }
    (!names.is_empty() && names[0].ends_with(':')).then_some(names)
}

/// The folder is walked from the boot volume (the `ZTYPE = 2` volume row) one
/// `ZDOPFOLDER` row per path component; names compare as stored (`COLLATE
/// BINARY`, so case-sensitively). A path that maps to more than one source
/// (two such volume rows) is treated as unregistered.
#[cfg(target_os = "macos")]
fn query(db: &Path, arw: &Path) -> rusqlite::Result<Option<Uuids>> {
    use rusqlite::OptionalExtension;

    let Some(parts) = folder_names(arw) else {
        return Ok(None);
    };
    let Some(file) = arw.file_name().and_then(|n| n.to_str()) else {
        return Ok(None);
    };
    let conn = open(db)?;

    let mut roots =
        conn.prepare("SELECT Z_PK FROM ZDOPFOLDER WHERE ZPARENT IS NULL AND ZTYPE = 2")?;
    let mut children =
        conn.prepare("SELECT Z_PK FROM ZDOPFOLDER WHERE ZPARENT = ? AND ZNAME = ?")?;
    let mut sources =
        conn.prepare("SELECT Z_PK, ZUUID FROM ZDOPSOURCE WHERE ZPARENT = ? AND ZNAME = ?")?;
    let mut master =
        conn.prepare("SELECT ZUUID FROM ZDOPINPUTITEM WHERE ZSOURCE = ? ORDER BY Z_PK LIMIT 1")?;

    let mut folders = roots
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for name in &parts {
        let mut next = Vec::new();
        for parent in &folders {
            for id in children.query_map(rusqlite::params![parent, name], |row| row.get(0))? {
                next.push(id?);
            }
        }
        folders = next;
    }
    let mut found = Vec::new();
    for folder in &folders {
        for source in sources.query_map(rusqlite::params![folder, file], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })? {
            found.push(source?);
        }
    }
    let [(source_id, source)] = found.as_slice() else {
        return Ok(None);
    };
    let Some(item) = master
        .query_row([source_id], |row| row.get::<_, String>(0))
        .optional()?
    else {
        return Ok(None);
    };
    Ok((is_uuid(&item) && is_uuid(source)).then(|| Uuids {
        item,
        source: source.clone(),
    }))
}

/// The `ZDOPFOLDER` names of `arw`'s parent below the boot volume, one per
/// directory. `None` for a relative path, one with `.` / `..`, or one under
/// `/Volumes` (other volumes are not mapped yet).
#[cfg(target_os = "macos")]
fn folder_names(arw: &Path) -> Option<Vec<String>> {
    use std::path::Component;

    let mut components = arw.parent()?.components();
    if components.next()? != Component::RootDir {
        return None;
    }
    let names = components
        .map(|component| match component {
            Component::Normal(name) => name.to_str().map(str::to_owned),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (names.first().map(String::as_str) != Some("Volumes")).then_some(names)
}

/// Whether `s` has the 8-4-4-4-12 hex shape of a UUID. The `.dop` template
/// writes the Uuids between quotes unescaped, so a value read from another
/// program's database must not be able to break the Lua literal.
#[cfg(any(windows, target_os = "macos"))]
fn is_uuid(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(g, len)| g.len() == len && g.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::path::PathBuf;

    const SOURCE: &str = "E584D7F8-131A-4EC5-B191-5E5C570A2F9B";
    const MASTER: &str = "04B2552F-D026-4597-9695-AA245AEB0132";
    const COPY: &str = "9FEF2691-2E05-4189-9C48-B0EB7C83D320";

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("riffle-app-photolab-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A database holding `D:\Photos\tests\shoot\_DSC0001.ARW` with a master
    /// and a virtual copy, and `D:\Photos\other\_DSC0001.ARW`'s folder with
    /// no sources.
    fn fixture(name: &str, source_uuid: &str) -> PathBuf {
        let db = temp_dir(name).join("PhotoLab.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE Folders (Id INTEGER PRIMARY KEY, Name TEXT COLLATE NOCASE, ParentFolderId INTEGER);
             CREATE TABLE Sources (Id INTEGER PRIMARY KEY, Name TEXT COLLATE NOCASE, Uuid TEXT, FolderId INTEGER);
             CREATE TABLE Items (Id INTEGER PRIMARY KEY, SourceId INTEGER, Uuid TEXT, CreationDate TEXT, ModificationDate TEXT, StorePickOrReject INTEGER);
             INSERT INTO Folders VALUES (1, 'C:', NULL), (7, 'D:', NULL), (41, 'Photos', 7), (146, 'tests', 41), (147, 'shoot', 146), (148, 'other', 41);
             INSERT INTO Sources VALUES (10, '_DSC0001.ARW', '{source_uuid}', 147);
             INSERT INTO Items VALUES
               (100, 10, '{MASTER}', '2026-09-27 14:37:23.6297741Z', '2026-09-27 14:37:23.7544221Z', 0),
               (101, 10, '{COPY}', '2026-09-27 14:39:56.4485575Z', '2026-09-27 14:39:56.4965436Z', 1);"
        ))
        .unwrap();
        db
    }

    fn uuids(item: &str, source: &str) -> Option<Uuids> {
        Some(Uuids {
            item: item.to_owned(),
            source: source.to_owned(),
        })
    }

    #[test]
    fn a_registered_file_yields_its_source_and_master_uuids() {
        let db = fixture("registered", SOURCE);
        assert_eq!(
            lookup(&db, Path::new(r"D:\Photos\tests\shoot\_DSC0001.ARW")),
            uuids(MASTER, SOURCE)
        );
    }

    #[test]
    fn a_file_at_a_drive_root_yields_its_source_and_master_uuids() {
        let db = fixture("drive_root", SOURCE);
        Connection::open(&db)
            .unwrap()
            .execute_batch(&format!(
                "INSERT INTO Sources VALUES (30, '_DSC0001.ARW', '{SOURCE}', 7);
                 INSERT INTO Items VALUES (300, 30, '{MASTER}', '2026-09-27 14:37:23Z', '2026-09-27 14:37:23Z', 0);"
            ))
            .unwrap();
        assert_eq!(
            lookup(&db, Path::new(r"D:\_DSC0001.ARW")),
            uuids(MASTER, SOURCE)
        );
    }

    #[test]
    fn names_match_case_insensitively() {
        let db = fixture("case", SOURCE);
        assert_eq!(
            lookup(&db, Path::new(r"d:\photos\TESTS\shoot\_dsc0001.arw")),
            uuids(MASTER, SOURCE)
        );
    }

    #[test]
    fn misses_and_failures_yield_none() {
        let db = fixture("misses", SOURCE);
        assert_eq!(
            lookup(&db, Path::new(r"D:\Photos\tests\shoot\_DSC0002.ARW")),
            None
        );
        assert_eq!(
            lookup(&db, Path::new(r"D:\Photos\other\_DSC0001.ARW")),
            None
        );
        assert_eq!(
            lookup(&db, Path::new(r"E:\Photos\tests\shoot\_DSC0001.ARW")),
            None
        );
        assert_eq!(
            lookup(&db, Path::new(r"\\server\share\shoot\_DSC0001.ARW")),
            None
        );
        let missing = db.with_file_name("missing.db");
        assert_eq!(
            lookup(&missing, Path::new(r"D:\Photos\tests\shoot\_DSC0001.ARW")),
            None
        );
        let empty = db.with_file_name("empty.db");
        Connection::open(&empty)
            .unwrap()
            .execute_batch("CREATE TABLE Other (Id INTEGER)")
            .unwrap();
        assert_eq!(
            lookup(&empty, Path::new(r"D:\Photos\tests\shoot\_DSC0001.ARW")),
            None
        );
    }

    #[test]
    fn a_path_under_two_same_lettered_volumes_yields_none() {
        let db = fixture("volumes", SOURCE);
        Connection::open(&db)
            .unwrap()
            .execute_batch(&format!(
                "INSERT INTO Folders VALUES (200, 'D:', NULL), (201, 'Photos', 200), (202, 'tests', 201), (203, 'shoot', 202);
                 INSERT INTO Sources VALUES (20, '_DSC0001.ARW', '{SOURCE}', 203);
                 INSERT INTO Items VALUES (200, 20, '{MASTER}', '2026-09-27 14:37:23Z', '2026-09-27 14:37:23Z', 0);"
            ))
            .unwrap();
        assert_eq!(
            lookup(&db, Path::new(r"D:\Photos\tests\shoot\_DSC0001.ARW")),
            None
        );
    }

    #[test]
    fn a_uuid_that_could_break_the_dop_literal_yields_none() {
        let db = fixture("literal", r#"E584D7F8-131A-4EC5-B191-5E5C570A2F9"",x=""#);
        assert_eq!(
            lookup(&db, Path::new(r"D:\Photos\tests\shoot\_DSC0001.ARW")),
            None
        );
    }

    #[test]
    fn a_lock_released_within_the_retries_yields_the_uuids() {
        let db = fixture("busy_released", SOURCE);
        let lock = Connection::open(&db).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let release = std::thread::spawn(move || {
            // One attempt takes about 0.8 s of wall time on Windows (winLock
            // sleeps on top of the busy handler), so 2 s is past two attempts
            // and well under the six-attempt total of about 4.9 s.
            std::thread::sleep(std::time::Duration::from_secs(2));
            lock.execute_batch("COMMIT").unwrap();
        });
        assert_eq!(
            lookup(&db, Path::new(r"D:\Photos\tests\shoot\_DSC0001.ARW")),
            uuids(MASTER, SOURCE)
        );
        release.join().unwrap();
    }

    #[test]
    fn a_lock_held_through_every_retry_yields_none() {
        let db = fixture("busy_held", SOURCE);
        let lock = Connection::open(&db).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();
        assert_eq!(
            lookup(&db, Path::new(r"D:\Photos\tests\shoot\_DSC0001.ARW")),
            None
        );
        lock.execute_batch("COMMIT").unwrap();
    }

    #[test]
    fn uuid_shape() {
        assert!(is_uuid(SOURCE));
        assert!(is_uuid(&SOURCE.to_lowercase()));
        assert!(!is_uuid("E584D7F8131A4EC5B1915E5C570A2F9B"));
        assert!(!is_uuid("E584D7F8-131A-4EC5-B191-5E5C570A2F9G"));
        assert!(!is_uuid("E584D7F8-131A-4EC5-B191-5E5C570A2F9B-"));
    }

    #[test]
    fn photolab_directory_versions() {
        assert_eq!(version("DxO PhotoLab 10"), Some(10));
        assert_eq!(version("DxO PhotoLab 9"), Some(9));
        assert_eq!(version("DxO PhotoLab"), None);
        assert_eq!(version("DxO PhotoLab 10 beta"), None);
        assert_eq!(version("DxO PureRAW 4"), None);
        assert_eq!(version("DxO PhotoLab +1"), None);
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;
    use rusqlite::Connection;
    use std::path::PathBuf;

    const SOURCE: &str = "E584D7F8-131A-4EC5-B191-5E5C570A2F9B";
    const MASTER: &str = "04B2552F-D026-4597-9695-AA245AEB0132";
    const COPY: &str = "9FEF2691-2E05-4189-9C48-B0EB7C83D320";
    const SHOT: &str = "/Users/mino/Pictures/shoot/_DSC0001.ARW";

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("riffle-app-photolab-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A database holding `/Users/mino/Pictures/shoot/_DSC0001.ARW` on the
    /// boot volume with a copy inserted before its master, and
    /// `/Users/mino/Pictures/other`'s folder with no sources.
    fn fixture(name: &str, source_uuid: &str) -> PathBuf {
        let db = temp_dir(name).join("DOPDatabaseV10.dopdata");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE ZDOPFOLDER (Z_PK INTEGER PRIMARY KEY, Z_ENT INTEGER, ZPARENT INTEGER, Z5_PARENT INTEGER, ZTYPE INTEGER, ZNAME VARCHAR, ZLOCALIZEDNAME VARCHAR, ZUNIQUEID VARCHAR);
             CREATE TABLE ZDOPSOURCE (Z_PK INTEGER PRIMARY KEY, Z_ENT INTEGER, ZPARENT INTEGER, ZNAME VARCHAR, ZUUID VARCHAR);
             CREATE TABLE ZDOPINPUTITEM (Z_PK INTEGER PRIMARY KEY, Z_ENT INTEGER, ZSOURCE INTEGER, ZUUID VARCHAR);
             INSERT INTO ZDOPFOLDER VALUES
               (1, 6, NULL, NULL, 1, 'DxO PhotoLab', NULL, NULL),
               (2, 6, NULL, NULL, 2, 'Macintosh HD', NULL, '6F1D2A4C-0B8E-4F5A-9C3D-2E7B8A1F0C64'),
               (3, 5, 2, 6, NULL, 'Users', NULL, NULL),
               (4, 5, 3, 5, NULL, 'mino', NULL, NULL),
               (5, 5, 4, 5, NULL, 'Pictures', NULL, NULL),
               (6, 5, 5, 5, NULL, 'shoot', NULL, NULL),
               (7, 5, 5, 5, NULL, 'other', NULL, NULL);
             INSERT INTO ZDOPSOURCE VALUES (10, 18, 6, '_DSC0001.ARW', '{source_uuid}');
             INSERT INTO ZDOPINPUTITEM VALUES (101, 8, 10, '{COPY}'), (100, 8, 10, '{MASTER}');"
        ))
        .unwrap();
        db
    }

    fn uuids(item: &str, source: &str) -> Option<Uuids> {
        Some(Uuids {
            item: item.to_owned(),
            source: source.to_owned(),
        })
    }

    #[test]
    fn a_registered_file_yields_its_source_and_master_uuids() {
        let db = fixture("mac_registered", SOURCE);
        assert_eq!(lookup(&db, Path::new(SHOT)), uuids(MASTER, SOURCE));
    }

    #[test]
    fn the_lookup_does_not_depend_on_the_core_data_entity_numbers() {
        let db = fixture("mac_entities", SOURCE);
        Connection::open(&db)
            .unwrap()
            .execute_batch("UPDATE ZDOPFOLDER SET Z_ENT = Z_ENT + 20;")
            .unwrap();
        assert_eq!(lookup(&db, Path::new(SHOT)), uuids(MASTER, SOURCE));
    }

    #[test]
    fn names_match_case_sensitively() {
        let db = fixture("mac_case", SOURCE);
        assert_eq!(
            lookup(&db, Path::new("/Users/mino/Pictures/shoot/_dsc0001.arw")),
            None
        );
        assert_eq!(
            lookup(&db, Path::new("/users/mino/Pictures/shoot/_DSC0001.ARW")),
            None
        );
    }

    #[test]
    fn a_folder_chain_under_the_other_volume_is_not_matched() {
        let db = fixture("mac_other_volume", SOURCE);
        Connection::open(&db)
            .unwrap()
            .execute_batch(&format!(
                "INSERT INTO ZDOPFOLDER VALUES
                   (20, 5, 1, 6, NULL, 'Users', NULL, NULL),
                   (21, 5, 20, 5, NULL, 'mino', NULL, NULL),
                   (22, 5, 21, 5, NULL, 'Pictures', NULL, NULL),
                   (23, 5, 22, 5, NULL, 'shoot', NULL, NULL);
                 INSERT INTO ZDOPSOURCE VALUES (20, 18, 23, '_DSC0002.ARW', '{SOURCE}');
                 INSERT INTO ZDOPINPUTITEM VALUES (200, 8, 20, '{MASTER}');"
            ))
            .unwrap();
        assert_eq!(
            lookup(&db, Path::new("/Users/mino/Pictures/shoot/_DSC0002.ARW")),
            None
        );
    }

    #[test]
    fn a_path_under_two_boot_volume_rows_yields_none() {
        let db = fixture("mac_volumes", SOURCE);
        Connection::open(&db)
            .unwrap()
            .execute_batch(&format!(
                "INSERT INTO ZDOPFOLDER VALUES
                   (30, 6, NULL, NULL, 2, 'Macintosh HD', NULL, NULL),
                   (31, 5, 30, 6, NULL, 'Users', NULL, NULL),
                   (32, 5, 31, 5, NULL, 'mino', NULL, NULL),
                   (33, 5, 32, 5, NULL, 'Pictures', NULL, NULL),
                   (34, 5, 33, 5, NULL, 'shoot', NULL, NULL);
                 INSERT INTO ZDOPSOURCE VALUES (30, 18, 34, '_DSC0001.ARW', '{SOURCE}');
                 INSERT INTO ZDOPINPUTITEM VALUES (300, 8, 30, '{MASTER}');"
            ))
            .unwrap();
        assert_eq!(lookup(&db, Path::new(SHOT)), None);
    }

    #[test]
    fn misses_and_failures_yield_none() {
        let db = fixture("mac_misses", SOURCE);
        for path in [
            "/Users/mino/Pictures/shoot/_DSC0002.ARW",
            "/Users/mino/Pictures/other/_DSC0001.ARW",
            "Users/mino/Pictures/shoot/_DSC0001.ARW",
            "./Users/mino/Pictures/shoot/_DSC0001.ARW",
            "/Users/mino/Pictures/x/../shoot/_DSC0001.ARW",
            "/Volumes/Macintosh HD/Users/mino/Pictures/shoot/_DSC0001.ARW",
        ] {
            assert_eq!(lookup(&db, Path::new(path)), None, "{path}");
        }
        let missing = db.with_file_name("missing.dopdata");
        assert_eq!(lookup(&missing, Path::new(SHOT)), None);
        let empty = db.with_file_name("empty.dopdata");
        Connection::open(&empty)
            .unwrap()
            .execute_batch("CREATE TABLE Other (Z_PK INTEGER)")
            .unwrap();
        assert_eq!(lookup(&empty, Path::new(SHOT)), None);
    }

    #[test]
    fn a_uuid_that_could_break_the_dop_literal_yields_none() {
        let db = fixture(
            "mac_literal",
            r#"E584D7F8-131A-4EC5-B191-5E5C570A2F9"",x=""#,
        );
        assert_eq!(lookup(&db, Path::new(SHOT)), None);
    }

    #[test]
    fn uuid_shape() {
        assert!(is_uuid(SOURCE));
        assert!(is_uuid(&SOURCE.to_lowercase()));
        assert!(!is_uuid("E584D7F8131A4EC5B1915E5C570A2F9B"));
        assert!(!is_uuid("E584D7F8-131A-4EC5-B191-5E5C570A2F9G"));
        assert!(!is_uuid("E584D7F8-131A-4EC5-B191-5E5C570A2F9B-"));
    }

    #[test]
    fn photolab_directory_versions() {
        assert_eq!(version("DxO PhotoLab v10"), Some(10));
        assert_eq!(version("DxO PhotoLab v9"), Some(9));
        assert_eq!(version("DxO PhotoLab v"), None);
        assert_eq!(version("DxO PhotoLab 10"), None);
        assert_eq!(version("DxO PhotoLab v10 beta"), None);
        assert_eq!(version("DxO PureRAW v4"), None);
        assert_eq!(version("DxO PhotoLab v+1"), None);
    }

    #[test]
    fn the_newest_version_with_a_database_file_wins() {
        let library = temp_dir("mac_library");
        for (dir, file) in [
            ("DxO PhotoLab v9", Some("DOPDatabaseV9.dopdata")),
            ("DxO PhotoLab v10", Some("DOPDatabaseV10.dopdata")),
            ("DxO PhotoLab v11", None),
            ("DxO PhotoLab v12", Some("DOPDatabaseV10.dopdata")),
        ] {
            std::fs::create_dir_all(library.join(dir)).unwrap();
            if let Some(file) = file {
                std::fs::write(library.join(dir).join(file), b"").unwrap();
            }
        }
        assert_eq!(
            newest_database(&library),
            Some(
                library
                    .join("DxO PhotoLab v10")
                    .join("DOPDatabaseV10.dopdata")
            )
        );
        assert_eq!(newest_database(&library.join("missing")), None);
    }
}
