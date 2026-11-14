use std::path::{Path, PathBuf};

/// The product's identifier after the Handpic rename.
pub const IDENTIFIER: &str = "dev.minodisk.handpic";

/// The identifiers the product had before `IDENTIFIER`, newest first.
pub const LEGACY_IDENTIFIERS: &[&str] = &["com.minodisk.handpic", "com.minodisk.riffle"];

/// The index file: `flag` when given, else `index.sqlite` in the native app's
/// own `<IDENTIFIER>.native` folder under `cache`, apart from the Tauri app's
/// `<IDENTIFIER>` so the two never share one SQLite file; `None` when neither
/// is known.
pub fn index_path(flag: Option<PathBuf>, cache: Option<&Path>) -> Option<PathBuf> {
    flag.or_else(|| {
        cache.map(|cache| {
            cache
                .join(format!("{IDENTIFIER}.native"))
                .join("index.sqlite")
        })
    })
}

/// The settings store: `flag` when given, else `settings.sqlite` in the
/// `<IDENTIFIER>` folder under `config`, next to the Tauri app's
/// `settings.json`; `None` when neither is known.
pub fn settings_path(flag: Option<PathBuf>, config: Option<&Path>) -> Option<PathBuf> {
    flag.or_else(|| config.map(|config| config.join(IDENTIFIER).join("settings.sqlite")))
}

#[cfg(test)]
mod tests {
    use super::{IDENTIFIER, index_path, settings_path};
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
    fn the_default_is_the_native_folder_under_the_cache_dir() {
        assert_eq!(
            index_path(None, Some(Path::new("C:/cache"))),
            Some(
                Path::new("C:/cache")
                    .join("dev.minodisk.handpic.native")
                    .join("index.sqlite")
            )
        );
        assert_eq!(index_path(None, None), None);
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
