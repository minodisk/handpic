use std::path::{Path, PathBuf};

/// The product's identifier after the Handpic rename.
pub const IDENTIFIER: &str = "dev.minodisk.handpic";

/// The identifiers the product had before `IDENTIFIER`, newest first.
pub const LEGACY_IDENTIFIERS: &[&str] = &["com.minodisk.handpic", "com.minodisk.riffle"];

const INDEX_FILE: &str = "index.sqlite";

/// The index file: `flag` when given, else `index.sqlite` in the
/// `<IDENTIFIER>` folder under `cache`; `None` when neither is known.
pub fn index_path(flag: Option<PathBuf>, cache: Option<&Path>) -> Option<PathBuf> {
    flag.or_else(|| cache.map(|cache| cache.join(IDENTIFIER).join(INDEX_FILE)))
}

/// What `move_native_index` did.
#[derive(Debug, PartialEq)]
pub enum IndexMove {
    Nothing,
    Moved,
    /// Moved, the index already in the plain folder renamed aside to this file.
    Aside(PathBuf),
}

/// `file` and its SQLite `-wal` and `-shm` in `dir`.
fn with_companions(dir: &Path, file: &str) -> [PathBuf; 3] {
    ["", "-wal", "-shm"].map(|suffix| dir.join(format!("{file}{suffix}")))
}

fn rename_present(from: &[PathBuf; 3], to: &[PathBuf; 3]) -> std::io::Result<()> {
    for (from, to) in from.iter().zip(to) {
        if from.exists() {
            std::fs::rename(from, to)?;
        }
    }
    Ok(())
}

/// Moves the index earlier builds kept in `<IDENTIFIER>.native` under `cache`
/// into the `<IDENTIFIER>` folder `index_path` names, with its `-wal` and
/// `-shm`. An index already there (a Tauri build's) is renamed aside to the
/// first free `index-aside-<n>.sqlite`, never deleted. The emptied `.native`
/// folder is removed when empty (on Windows it still holds the old log).
pub fn move_native_index(cache: &Path) -> std::io::Result<IndexMove> {
    let from = cache.join(format!("{IDENTIFIER}.native"));
    if !from.join(INDEX_FILE).is_file() {
        return Ok(IndexMove::Nothing);
    }
    let to = cache.join(IDENTIFIER);
    std::fs::create_dir_all(&to)?;
    let present = with_companions(&to, INDEX_FILE);
    let aside = if present.iter().any(|p| p.exists()) {
        let file = (1..)
            .map(|n| format!("index-aside-{n}.sqlite"))
            .find(|file| with_companions(&to, file).iter().all(|p| !p.exists()))
            .unwrap();
        rename_present(&present, &with_companions(&to, &file))?;
        Some(to.join(file))
    } else {
        None
    };
    rename_present(&with_companions(&from, INDEX_FILE), &present)?;
    let _ = std::fs::remove_dir(&from);
    Ok(aside.map_or(IndexMove::Moved, IndexMove::Aside))
}

/// The settings store: `flag` when given, else `settings.sqlite` in the
/// `<IDENTIFIER>` folder under `config`, next to where the Tauri app kept its
/// `settings.json`; `None` when neither is known.
pub fn settings_path(flag: Option<PathBuf>, config: Option<&Path>) -> Option<PathBuf> {
    flag.or_else(|| config.map(|config| config.join(IDENTIFIER).join("settings.sqlite")))
}

#[cfg(test)]
mod tests {
    use super::{IDENTIFIER, IndexMove, index_path, move_native_index, settings_path};
    use std::path::{Path, PathBuf};

    #[test]
    fn the_settings_flag_beats_the_default_under_the_config_dir() {
        assert_eq!(
            settings_path(
                Some(PathBuf::from("D:/t/s.sqlite")),
                Some(Path::new("C:/config"))
            ),
            Some(PathBuf::from("D:/t/s.sqlite"))
        );
        assert_eq!(
            settings_path(None, Some(Path::new("C:/config"))),
            Some(
                Path::new("C:/config")
                    .join(IDENTIFIER)
                    .join("settings.sqlite")
            )
        );
        assert_eq!(settings_path(None, None), None);
    }

    #[test]
    fn the_flag_beats_the_default() {
        assert_eq!(
            index_path(
                Some(PathBuf::from("D:/t/i.sqlite")),
                Some(Path::new("C:/cache"))
            ),
            Some(PathBuf::from("D:/t/i.sqlite"))
        );
    }

    #[test]
    fn the_default_is_the_identifier_folder_under_the_cache_dir() {
        assert_eq!(
            index_path(None, Some(Path::new("C:/cache"))),
            Some(
                Path::new("C:/cache")
                    .join("dev.minodisk.handpic")
                    .join("index.sqlite")
            )
        );
        assert_eq!(index_path(None, None), None);
    }

    fn put(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn nothing_moves_without_a_native_index() {
        let cache = tempfile::tempdir().unwrap();
        let plain = cache.path().join(IDENTIFIER);
        put(&plain.join("index.sqlite"), "tauri");
        assert_eq!(move_native_index(cache.path()).unwrap(), IndexMove::Nothing);
        assert_eq!(read(&plain.join("index.sqlite")), "tauri");
        assert!(!plain.join("index-aside-1.sqlite").exists());
    }

    #[test]
    fn the_native_index_moves_with_its_companions_into_the_plain_folder() {
        let cache = tempfile::tempdir().unwrap();
        let native = cache.path().join(format!("{IDENTIFIER}.native"));
        put(&native.join("index.sqlite"), "native");
        put(&native.join("index.sqlite-wal"), "wal");
        put(&native.join("index.sqlite-shm"), "shm");
        assert_eq!(move_native_index(cache.path()).unwrap(), IndexMove::Moved);
        let plain = cache.path().join(IDENTIFIER);
        assert_eq!(read(&plain.join("index.sqlite")), "native");
        assert_eq!(read(&plain.join("index.sqlite-wal")), "wal");
        assert_eq!(read(&plain.join("index.sqlite-shm")), "shm");
        assert!(!native.exists());
        assert_eq!(move_native_index(cache.path()).unwrap(), IndexMove::Nothing);
    }

    #[test]
    fn an_index_already_in_the_plain_folder_is_renamed_aside() {
        let cache = tempfile::tempdir().unwrap();
        let native = cache.path().join(format!("{IDENTIFIER}.native"));
        let plain = cache.path().join(IDENTIFIER);
        put(&native.join("index.sqlite"), "native");
        put(&plain.join("index.sqlite"), "tauri");
        put(&plain.join("index.sqlite-wal"), "tauri wal");
        put(&plain.join("index-aside-1.sqlite"), "earlier");
        put(&plain.join("settings.sqlite"), "settings");
        assert_eq!(
            move_native_index(cache.path()).unwrap(),
            IndexMove::Aside(plain.join("index-aside-2.sqlite"))
        );
        assert_eq!(read(&plain.join("index.sqlite")), "native");
        assert!(!plain.join("index.sqlite-wal").exists());
        assert_eq!(read(&plain.join("index-aside-2.sqlite")), "tauri");
        assert_eq!(read(&plain.join("index-aside-2.sqlite-wal")), "tauri wal");
        assert_eq!(read(&plain.join("index-aside-1.sqlite")), "earlier");
        assert_eq!(read(&plain.join("settings.sqlite")), "settings");
        assert!(!native.exists());
    }

    fn packaging(file: &str) -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("packaging")
                .join(file),
        )
        .unwrap()
    }

    fn plist_string(plist: &str, key: &str) -> String {
        let after = plist
            .split(&format!("<key>{key}</key>"))
            .nth(1)
            .unwrap_or_else(|| panic!("no {key}"));
        let value = after.trim_start().strip_prefix("<string>").unwrap();
        value[..value.find("</string>").unwrap()].to_string()
    }

    fn desktop_value(desktop: &str, key: &str) -> String {
        desktop
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("no {key}"))
            .to_string()
    }

    #[test]
    fn the_macos_bundle_names_the_identifier_and_the_binary() {
        let plist = packaging("macos/Info.plist");
        assert_eq!(plist_string(&plist, "CFBundleIdentifier"), IDENTIFIER);
        assert_eq!(
            plist_string(&plist, "CFBundleExecutable"),
            env!("CARGO_BIN_NAME")
        );
    }

    #[test]
    fn the_linux_desktop_file_names_the_identifier_and_the_binary() {
        let desktop = packaging(&format!("linux/{IDENTIFIER}.desktop"));
        assert_eq!(desktop_value(&desktop, "StartupWMClass"), IDENTIFIER);
        assert_eq!(desktop_value(&desktop, "Icon"), IDENTIFIER);
        assert_eq!(desktop_value(&desktop, "Exec"), env!("CARGO_BIN_NAME"));
    }
}
