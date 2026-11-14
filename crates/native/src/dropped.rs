//! A folder dropped onto the window, a port of the Tauri `tauri://drag-drop`
//! listener and `dropped_dir`: one path is taken, a directory as is and a
//! file by its parent.

use std::path::{Path, PathBuf};

pub const DROP_TEXT: &str = "Drop a single folder or RAW file.";

/// The one path a drop names, `None` for none or several.
pub fn single(paths: &[PathBuf]) -> Option<&Path> {
    match paths {
        [path] => Some(path),
        _ => None,
    }
}

/// The folder a dropped path opens: a directory as is, a file's parent
/// (the root for a file directly under it), else `None`. A `stat`, so off
/// the UI thread.
pub fn dropped_dir(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return Some(path.to_path_buf());
    }
    if path.is_file() {
        return path.parent().map(Path::to_path_buf);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{dropped_dir, single};

    #[test]
    fn one_path_is_taken_and_none_or_several_are_not() {
        let a = PathBuf::from("/d/a");
        let b = PathBuf::from("/d/b");
        assert_eq!(single(std::slice::from_ref(&a)), Some(a.as_path()));
        assert_eq!(single(&[]), None);
        assert_eq!(single(&[a, b]), None);
    }

    #[test]
    fn dropped_directory_is_taken_as_is_and_a_file_by_its_parent() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().to_path_buf();
        let file = dir.join("a.jpg");
        std::fs::write(&file, b"x").unwrap();

        assert_eq!(dropped_dir(&dir), Some(dir.clone()));
        assert_eq!(dropped_dir(&file), Some(dir.clone()));
        assert_eq!(dropped_dir(&dir.join("gone.arw")), None);
    }

    #[test]
    fn a_files_parent_at_the_filesystem_root_is_the_root_not_none() {
        assert_eq!(Path::new("/a.arw").parent(), Some(Path::new("/")));
    }
}
