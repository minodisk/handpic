//! `Sequence JPEG Timestamps…`: the preview and the run of
//! `riffle_core::sequence` on a folder of exported JPEGs.
//!
//! JPEGs are not indexed, so nothing here touches the index. The output is
//! written to a sibling of the folder.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use serde::Serialize;

/// One file that could not be read or written.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub path: String,
    pub message: String,
}

/// One file of a preview, in the computed order.
#[derive(Debug, Serialize)]
pub struct Row {
    pub path: String,
    pub old: String,
    pub new: String,
    pub changed: bool,
}

/// The preview of a run: the output folder, whether it exists, the files
/// in the computed order and the files that could not be read.
#[derive(Debug, Serialize)]
pub struct Preview {
    pub output_dir: String,
    pub output_exists: bool,
    pub rows: Vec<Row>,
    pub failed: Vec<Failure>,
}

fn failure(path: &Path, message: &str) -> Failure {
    Failure {
        path: path.to_string_lossy().into_owned(),
        message: message.to_string(),
    }
}

/// The times a run on `dir` would write, without touching the disk.
pub fn preview(dir: &Path) -> Result<Preview, String> {
    let out = riffle_core::sequence::output_dir(dir)?;
    let summary = riffle_core::sequence::run(dir, true, &AtomicBool::new(false), |_, _, _, _| {})?;
    let mut rows = Vec::new();
    let mut failed = Vec::new();
    for (path, result) in &summary.results {
        match result {
            Ok(outcome) => rows.push(Row {
                path: path.to_string_lossy().into_owned(),
                old: outcome.old.to_string(),
                new: outcome.new.to_string(),
                changed: outcome.changed,
            }),
            Err(message) => failed.push(failure(path, message)),
        }
    }
    Ok(Preview {
        output_dir: out.to_string_lossy().into_owned(),
        output_exists: out.exists(),
        rows,
        failed,
    })
}

/// What a run did: the output folder, the files written, the files it
/// found, the failures and whether it was canceled.
#[derive(Debug)]
pub struct Ran {
    pub output_dir: String,
    pub written: usize,
    pub total: usize,
    pub failed: Vec<Failure>,
    pub canceled: bool,
}

/// Write the sequenced copy of `dir`, stopping before the next file once
/// `cancel` is set. `progress` gets `(done, total)` after each file and is
/// called for the last time before this returns. A folder-level error (no
/// JPEGs, an output folder that cannot be rebuilt) is the one failure, keyed
/// by `dir`.
pub fn run(dir: &Path, cancel: &AtomicBool, progress: impl Fn(usize, usize) + Sync) -> Ran {
    let output_dir = riffle_core::sequence::output_dir(dir)
        .map(|out| out.to_string_lossy().into_owned())
        .unwrap_or_default();
    match riffle_core::sequence::run(dir, false, cancel, |done, total, _, _| {
        progress(done, total)
    }) {
        Ok(summary) => {
            let failed: Vec<Failure> = summary
                .results
                .iter()
                .filter_map(|(path, result)| {
                    result.as_ref().err().map(|message| failure(path, message))
                })
                .collect();
            Ran {
                output_dir,
                written: summary.results.len() - failed.len(),
                total: summary.total,
                failed,
                canceled: summary.canceled,
            }
        }
        Err(message) => Ran {
            output_dir,
            written: 0,
            total: 0,
            failed: vec![failure(dir, &message)],
            canceled: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::preview;
    use std::path::{Path, PathBuf};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("riffle-sequence-{name}-{}", std::process::id()))
            .join("export");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A JPEG header holding only an Exif APP1 with `DateTimeOriginal`, which
    /// is all a dry run reads.
    fn jpeg(datetime: &str) -> Vec<u8> {
        let mut tiff = b"II".to_vec();
        tiff.extend_from_slice(&42u16.to_le_bytes());
        tiff.extend_from_slice(&8u32.to_le_bytes());
        // IFD0 at 8: the Exif IFD pointer, pointing at 26.
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x8769u16.to_le_bytes());
        tiff.extend_from_slice(&4u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&26u32.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        // Exif IFD at 26: DateTimeOriginal, its value at 44.
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x9003u16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&20u32.to_le_bytes());
        tiff.extend_from_slice(&44u32.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff.extend_from_slice(datetime.as_bytes());
        tiff.push(0);
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(&tiff);
        let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(&payload);
        out.extend_from_slice(&[0xFF, 0xD9]);
        out
    }

    fn name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn a_preview_lists_the_files_in_time_order() {
        let dir = temp_dir("preview");
        std::fs::write(dir.join("DSC00002.JPG"), jpeg("2024:01:02 03:04:59")).unwrap();
        std::fs::write(dir.join("L1000001.jpg"), jpeg("2024:01:02 03:04:58")).unwrap();
        std::fs::write(dir.join("DSC00001.jpg"), jpeg("2024:01:02 03:04:58")).unwrap();
        std::fs::write(dir.join("broken.jpg"), b"not a jpeg").unwrap();

        let preview = preview(&dir).unwrap();

        let rows: Vec<(String, &str, &str, bool)> = preview
            .rows
            .iter()
            .map(|r| (name(&r.path), r.old.as_str(), r.new.as_str(), r.changed))
            .collect();
        assert_eq!(
            rows,
            [
                (
                    "DSC00001.jpg".to_string(),
                    "2024:01:02 03:04:58",
                    "2024:01:02 03:04:58",
                    false
                ),
                (
                    "L1000001.jpg".to_string(),
                    "2024:01:02 03:04:58",
                    "2024:01:02 03:04:59",
                    true
                ),
                (
                    "DSC00002.JPG".to_string(),
                    "2024:01:02 03:04:59",
                    "2024:01:02 03:05:00",
                    true
                ),
            ]
        );
        assert_eq!(preview.failed.len(), 1);
        assert_eq!(name(&preview.failed[0].path), "broken.jpg");
        let out = dir.with_file_name("export-sequenced");
        assert_eq!(Path::new(&preview.output_dir), out);
        assert!(!preview.output_exists);
        assert!(!out.exists());

        std::fs::create_dir(&out).unwrap();
        assert!(super::preview(&dir).unwrap().output_exists);
    }

    #[test]
    fn a_preview_of_a_folder_without_jpegs_is_an_error() {
        let dir = temp_dir("empty");
        assert!(preview(&dir).is_err());
    }
}
