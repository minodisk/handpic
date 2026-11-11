use std::path::{Path, PathBuf};

/// The product's identifier after the Handpic rename.
pub const IDENTIFIER: &str = "com.minodisk.handpic";

/// The identifiers the product had before `IDENTIFIER`, newest first.
pub const LEGACY_IDENTIFIERS: &[&str] = &["com.minodisk.riffle"];

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
                    .join("com.minodisk.handpic.native")
                    .join("index.sqlite")
            )
        );
        assert_eq!(index_path(None, None), None);
    }
}
