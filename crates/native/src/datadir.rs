use std::path::{Path, PathBuf};

/// The native app's own folder under the OS cache dir, apart from the Tauri
/// app's `com.minodisk.riffle`, so the two never share one SQLite file.
const APP_DIR: &str = "com.minodisk.riffle.native";

/// The index file: `flag` when given, else `index.sqlite` in the app's folder
/// under `cache`; `None` when neither is known.
pub fn index_path(flag: Option<PathBuf>, cache: Option<&Path>) -> Option<PathBuf> {
    flag.or_else(|| cache.map(|cache| cache.join(APP_DIR).join("index.sqlite")))
}

#[cfg(test)]
mod tests {
    use super::index_path;
    use std::path::{Path, PathBuf};

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
                    .join("com.minodisk.riffle.native")
                    .join("index.sqlite")
            )
        );
        assert_eq!(index_path(None, None), None);
    }
}
