//! Which of a folder's files have an export: a file in the `Output` or
//! `Output-sequenced` subfolder that PhotoLab and the JPEG sequencer write.
//! The strip's filter offers the files without one to the trash, so a folder
//! that was not read whole gives no answer rather than a partial one.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

const EXPORT_FOLDERS: [&str; 2] = ["output", "output-sequenced"];
const DEEP_PRIME_MARK: &str = "-dxo";

fn read_error(path: &Path) -> impl Fn(io::Error) -> String + '_ {
    move |e| format!("{}: {e}", path.display())
}

fn stem_of(name: &str) -> Option<String> {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
}

/// The lower-cased file stems an export with lower-cased `stem` counts for:
/// its own, the one it is a `_<digits>` or `-<digits>` PhotoLab virtual copy
/// of, and whatever precedes a `-DxO`, the original of a DeepPRIME DNG. An
/// export can name several, since a suffix is only ever read off the export:
/// a `BF_00634_1.jpg` is both `BF_00634_1.DNG`'s and `BF_00634.DNG`'s copy.
fn matched_stems(stem: &str) -> impl Iterator<Item = &str> + '_ {
    let unsuffixed = stem
        .strip_suffix(|c: char| c.is_ascii_digit())
        .map(|rest| rest.trim_end_matches(|c: char| c.is_ascii_digit()))
        .and_then(|rest| rest.strip_suffix(['_', '-']));
    let originals = stem
        .match_indices(DEEP_PRIME_MARK)
        .map(|(start, _)| &stem[..start]);
    std::iter::once(stem).chain(unsuffixed).chain(originals)
}

/// The `files` that have an export among the file names `exports`. `files`
/// may be paths; they come back as they were given.
fn matching(files: &[String], exports: &[String]) -> Vec<String> {
    let export_stems: Vec<String> = exports.iter().filter_map(|name| stem_of(name)).collect();
    let matched: HashSet<&str> = export_stems
        .iter()
        .flat_map(|stem| matched_stems(stem))
        .collect();
    files
        .iter()
        .filter(|file| stem_of(file).is_some_and(|stem| matched.contains(stem.as_str())))
        .cloned()
        .collect()
}

/// The export folders of `dir`, one `read_dir` so that a name in any case is
/// found on a case-sensitive file system too, where several can coexist. An
/// entry that fails to read could be one of them, so it fails the listing.
fn export_folders(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = match std::fs::read_dir(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        entries => entries.map_err(read_error(dir))?,
    };
    let mut folders = Vec::new();
    for entry in entries {
        let entry = entry.map_err(read_error(dir))?;
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !EXPORT_FOLDERS.contains(&name.as_str()) {
            continue;
        }
        let path = entry.path();
        if std::fs::metadata(&path)
            .map_err(read_error(&path))?
            .is_dir()
        {
            folders.push(path);
        }
    }
    Ok(folders)
}

/// The names of the regular files directly in `folders`. Unlike
/// `listing::is_file`, a file that cannot be told is an error, not a skip.
fn export_names(folders: &[PathBuf]) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for folder in folders {
        for entry in std::fs::read_dir(folder).map_err(read_error(folder))? {
            let entry = entry.map_err(read_error(folder))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(read_error(&path))?;
            let is_file = if file_type.is_symlink() {
                std::fs::metadata(&path)
                    .map_err(read_error(&path))?
                    .is_file()
            } else {
                file_type.is_file()
            };
            if is_file {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    Ok(names)
}

pub fn outputs_in(dir: &Path, files: &[String]) -> Result<Option<Vec<String>>, String> {
    let folders = export_folders(dir)?;
    if folders.is_empty() {
        return Ok(None);
    }
    Ok(Some(matching(files, &export_names(&folders)?)))
}

#[cfg(test)]
mod tests {
    use super::{export_names, matching, outputs_in};
    use std::path::{Path, PathBuf};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-output-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    fn matched(files: &[&str], exports: &[&str]) -> Vec<String> {
        matching(&strings(files), &strings(exports))
    }

    fn write_exports(dir: &Path, folder: &str, names: &[&str]) {
        let folder = dir.join(folder);
        std::fs::create_dir(&folder).unwrap();
        for name in names {
            std::fs::write(folder.join(name), b"x").unwrap();
        }
    }

    #[test]
    fn an_exact_stem_matches_under_any_extension() {
        assert_eq!(matched(&["A.ARW"], &["A.jpg"]), ["A.ARW"]);
        assert_eq!(matched(&["A.ARW"], &["A.tiff"]), ["A.ARW"]);
    }

    #[test]
    fn the_stems_compare_case_insensitively() {
        assert_eq!(matched(&["DSC0001.ARW"], &["dsc0001.JPG"]), ["DSC0001.ARW"]);
        assert_eq!(matched(&["a.arw"], &["A_1.JPG"]), ["a.arw"]);
        assert_eq!(matched(&["a.arw"], &["A-DXO_DeepPRIME.jpg"]), ["a.arw"]);
    }

    #[test]
    fn a_virtual_copy_matches_by_its_digits() {
        assert_eq!(matched(&["A.ARW"], &["A_1.jpg"]), ["A.ARW"]);
        assert_eq!(matched(&["A.ARW"], &["A-2.jpg"]), ["A.ARW"]);
        assert_eq!(matched(&["A.ARW"], &["A_123.jpg"]), ["A.ARW"]);
    }

    #[test]
    fn a_suffix_without_only_digits_is_no_copy() {
        for export in [
            "A_.jpg",
            "A-.jpg",
            "A_1a.jpg",
            "A_a1.jpg",
            "A 1.jpg",
            "A1.jpg",
            "A_1_2.jpg",
            "A_\u{0661}.jpg",
        ] {
            assert!(matched(&["A.ARW"], &[export]).is_empty(), "{export}");
        }
    }

    #[test]
    fn a_deep_prime_dng_matches_by_its_own_stem_and_its_copy() {
        let dng = "L1002140-DxO_DeepPRIME 3.dng";
        assert_eq!(matched(&[dng], &["L1002140-DxO_DeepPRIME 3.jpg"]), [dng]);
        assert_eq!(matched(&[dng], &["L1002140-DxO_DeepPRIME 3-2.jpg"]), [dng]);
    }

    #[test]
    fn the_original_matches_through_its_deep_prime_export() {
        let files = ["L1002140.DNG", "L1002140-DxO_DeepPRIME 3.dng"];
        assert_eq!(matched(&files, &["L1002140-DxO_DeepPRIME 3.jpg"]), files);
        assert_eq!(matched(&files, &["L1002140-DxO_DeepPRIME 3-2.jpg"]), files);
    }

    #[test]
    fn every_deep_prime_mark_of_a_stem_counts() {
        assert_eq!(
            matched(&["A-DxO_1.dng"], &["A-DxO_1-DxO_DeepPRIME 3.jpg"]),
            ["A-DxO_1.dng"]
        );
    }

    #[test]
    fn a_different_stem_does_not_match() {
        assert!(matched(&["DSC0001.ARW"], &["DSC00011.jpg"]).is_empty());
        assert!(matched(&["DSC00011.ARW"], &["DSC0001.jpg"]).is_empty());
        assert!(matched(&["A.ARW"], &["BA.jpg", "B-A.jpg", "B_A_1.jpg"]).is_empty());
    }

    #[test]
    fn the_suffix_rules_read_the_export_never_the_raw() {
        let raw = "BF_00634.DNG";
        assert_eq!(matched(&[raw], &["BF_00634.jpg"]), [raw]);
        assert_eq!(matched(&[raw], &["BF_00634_2.jpg"]), [raw]);
        assert!(matched(&[raw], &["BF.jpg"]).is_empty());
        let copy = "BF_00634_1.DNG";
        assert_eq!(matched(&[copy], &["BF_00634_1.jpg"]), [copy]);
        assert!(matched(&[copy], &["BF_00634.jpg"]).is_empty());
    }

    #[test]
    fn the_files_come_back_as_given_and_in_order() {
        let dir = temp_dir("order");
        let (b, a) = (dir.join("B.ARW"), dir.join("A.ARW"));
        let files = [b.to_str().unwrap(), "C.ARW", a.to_str().unwrap()];
        assert_eq!(matched(&files, &["A.jpg", "B.jpg"]), [files[0], files[2]]);
    }

    #[test]
    fn no_export_folder_is_no_answer() {
        let dir = temp_dir("none");
        std::fs::create_dir(dir.join("Other")).unwrap();
        assert_eq!(outputs_in(&dir, &strings(&["A.ARW"])), Ok(None));
    }

    #[test]
    fn a_missing_dir_is_no_answer() {
        let dir = temp_dir("missing").join("gone");
        assert_eq!(outputs_in(&dir, &strings(&["A.ARW"])), Ok(None));
    }

    #[test]
    fn a_file_named_like_an_export_folder_is_no_folder() {
        let dir = temp_dir("file");
        std::fs::write(dir.join("Output"), b"x").unwrap();
        std::fs::write(dir.join("Output-sequenced"), b"x").unwrap();
        assert_eq!(outputs_in(&dir, &strings(&["A.ARW"])), Ok(None));
    }

    #[test]
    fn an_empty_export_folder_is_an_answer() {
        let dir = temp_dir("empty");
        std::fs::create_dir(dir.join("Output")).unwrap();
        assert_eq!(outputs_in(&dir, &strings(&["A.ARW"])), Ok(Some(Vec::new())));
    }

    #[test]
    fn a_file_in_output_sequenced_only_counts() {
        let dir = temp_dir("sequenced");
        write_exports(&dir, "Output-sequenced", &["A.jpg"]);
        assert_eq!(
            outputs_in(&dir, &strings(&["A.ARW", "B.ARW"])),
            Ok(Some(strings(&["A.ARW"])))
        );
    }

    #[test]
    fn both_folders_count() {
        let dir = temp_dir("both");
        write_exports(&dir, "Output", &["A.jpg"]);
        write_exports(&dir, "Output-sequenced", &["B.jpg"]);
        assert_eq!(
            outputs_in(&dir, &strings(&["A.ARW", "B.ARW", "C.ARW"])),
            Ok(Some(strings(&["A.ARW", "B.ARW"])))
        );
    }

    #[test]
    fn the_folder_names_compare_case_insensitively() {
        let dir = temp_dir("case");
        write_exports(&dir, "output", &["A.jpg"]);
        write_exports(&dir, "OUTPUT-SEQUENCED", &["B.jpg"]);
        assert_eq!(
            outputs_in(&dir, &strings(&["A.ARW", "B.ARW"])),
            Ok(Some(strings(&["A.ARW", "B.ARW"])))
        );
    }

    #[test]
    fn a_nested_entry_does_not_count() {
        let dir = temp_dir("nested");
        write_exports(&dir, "Output", &[]);
        std::fs::create_dir(dir.join("Output").join("A.jpg")).unwrap();
        std::fs::create_dir(dir.join("Output").join("sub")).unwrap();
        std::fs::write(dir.join("Output").join("sub").join("B.jpg"), b"x").unwrap();
        assert_eq!(
            outputs_in(&dir, &strings(&["A.ARW", "B.ARW"])),
            Ok(Some(Vec::new()))
        );
    }

    #[test]
    fn an_unreadable_folder_fails_the_whole_listing() {
        let dir = temp_dir("unreadable");
        write_exports(&dir, "Output", &["A.jpg"]);
        let unreadable = dir.join("not-a-folder");
        std::fs::write(&unreadable, b"x").unwrap();
        assert!(export_names(&[dir.join("Output"), unreadable]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_export_folder_link_is_an_error() {
        let dir = temp_dir("dangling-folder");
        write_exports(&dir, "Output-sequenced", &["A.jpg"]);
        std::os::unix::fs::symlink(dir.join("gone"), dir.join("Output")).unwrap();
        assert!(outputs_in(&dir, &strings(&["A.ARW"])).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_export_link_is_an_error() {
        let dir = temp_dir("dangling-file");
        write_exports(&dir, "Output", &["A.jpg"]);
        std::os::unix::fs::symlink(dir.join("gone"), dir.join("Output").join("B.jpg")).unwrap();
        assert!(outputs_in(&dir, &strings(&["A.ARW"])).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_folder_and_file_count() {
        let dir = temp_dir("linked");
        write_exports(&dir, "elsewhere", &["A.jpg"]);
        std::os::unix::fs::symlink(dir.join("elsewhere"), dir.join("Output")).unwrap();
        write_exports(&dir, "Output-sequenced", &[]);
        std::os::unix::fs::symlink(
            dir.join("elsewhere").join("A.jpg"),
            dir.join("Output-sequenced").join("B.jpg"),
        )
        .unwrap();
        assert_eq!(
            outputs_in(&dir, &strings(&["A.ARW", "B.ARW"])),
            Ok(Some(strings(&["A.ARW", "B.ARW"])))
        );
    }
}
