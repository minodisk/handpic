//! A folder that holds only JPEGs opens view-only, a port of `viewonly.ts`:
//! the backend lists JPEGs only when there is no RAW, so the listed paths
//! alone decide the mode.

use std::path::Path;

pub const VIEW_ONLY_NOTE: &str = "JPEG folder: view only";

pub fn is_view_only(paths: &[String]) -> bool {
    !paths.is_empty()
        && paths.iter().all(|p| {
            Path::new(p)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg"))
        })
}

#[cfg(test)]
mod tests {
    use super::is_view_only;

    fn paths(ps: &[&str]) -> Vec<String> {
        ps.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn every_jpeg_whatever_the_case() {
        assert!(is_view_only(&paths(&[
            "/d/a.jpg",
            "/d/b.JPG",
            "/d/c.jpeg",
            "/d/d.JPEG"
        ])));
        assert!(is_view_only(&paths(&[r"\\?\D:\d\a.jpg"])));
    }

    #[test]
    fn not_for_an_empty_list() {
        assert!(!is_view_only(&[]));
    }

    #[test]
    fn not_when_any_path_is_a_raw() {
        assert!(!is_view_only(&paths(&["/d/a.jpg", "/d/b.ARW"])));
        assert!(!is_view_only(&paths(&["/d/a.ARW", "/d/b.dng"])));
    }

    #[test]
    fn looks_at_the_extension_only() {
        assert!(!is_view_only(&paths(&["/d/a.jpg.ARW"])));
        assert!(!is_view_only(&paths(&["/d.jpg/a.ARW"])));
        assert!(!is_view_only(&paths(&["/d/ajpg"])));
    }
}
