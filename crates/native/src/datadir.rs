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

/// The embedding model file `riffle_core::embed::MODEL_FILE` under `flag`
/// (`--data-dir`) when given, else under the native app's own
/// `<IDENTIFIER>.native` folder in `local`, apart from the Tauri app's model;
/// `None` when neither is known.
pub fn model_path(flag: Option<PathBuf>, local: Option<&Path>) -> Option<PathBuf> {
    flag.or_else(|| local.map(|local| local.join(format!("{IDENTIFIER}.native"))))
        .map(|dir| dir.join(riffle_core::embed::MODEL_FILE))
}

#[cfg(test)]
mod tests {
    use super::{IDENTIFIER, index_path, model_path, settings_path};
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
    fn the_model_is_under_the_data_dir_flag_else_the_native_folder() {
        let file = riffle_core::embed::MODEL_FILE;
        assert_eq!(
            model_path(
                Some(PathBuf::from("D:/t/data")),
                Some(Path::new("C:/local"))
            ),
            Some(PathBuf::from("D:/t/data").join(file))
        );
        assert_eq!(
            model_path(None, Some(Path::new("C:/local"))),
            Some(
                Path::new("C:/local")
                    .join("dev.minodisk.handpic.native")
                    .join(file)
            )
        );
        assert_eq!(model_path(None, None), None);
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
}
