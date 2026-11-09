//! One `read_dir` of a folder: the RAW and JPEG files it lists, the sidecars
//! next to them, and the canonical form of a folder path.

use std::collections::HashMap;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

use crate::index::{self, SidecarStat};
use crate::sidecar::SidecarFormat;

/// A folder's files as one `read_dir` finds them: its RAW files, and its
/// JPEGs, which it lists only when it holds no RAW at all, so a RAW+JPEG
/// folder never doubles its strip. The strip's listing and the tree's count
/// both go through it, so the two never disagree.
pub struct Media<T> {
    raws: Vec<T>,
    jpegs: Vec<T>,
}

impl<T> Default for Media<T> {
    fn default() -> Self {
        Self {
            raws: Vec::new(),
            jpegs: Vec::new(),
        }
    }
}

impl<T> Media<T> {
    /// Keep `item` if `path` names a RAW or a JPEG file; whether it did.
    pub fn add(&mut self, path: &Path, item: T) -> bool {
        if riffle_core::scan::is_raw_file(path) {
            self.raws.push(item);
        } else if riffle_core::scan::is_jpeg_file(path) {
            self.jpegs.push(item);
        } else {
            return false;
        }
        true
    }

    /// The files the folder lists, and whether they are its JPEGs.
    pub fn listed(self) -> (Vec<T>, bool) {
        if self.raws.is_empty() && !self.jpegs.is_empty() {
            (self.jpegs, true)
        } else {
            (self.raws, false)
        }
    }
}

/// Whether `entry` is a directory and whether it is a file, following a
/// symlink; `None` if that cannot be told (a broken link, say).
/// `file_type()` does not follow symlinks, so a symlinked folder or RAW file
/// is reported as neither a dir nor a file. Fall back to one `metadata()`
/// call (which does follow the link) only in that rare case, keeping the
/// common path free of a per-entry stat: `file_type()` comes with the
/// directory listing on Windows.
pub fn kind(entry: &DirEntry) -> Option<(bool, bool)> {
    let file_type = entry.file_type().ok()?;
    if file_type.is_symlink() {
        let m = std::fs::metadata(entry.path()).ok()?;
        Some((m.is_dir(), m.is_file()))
    } else {
        Some((file_type.is_dir(), file_type.is_file()))
    }
}

/// Whether `entry` is a file, following a symlink, as `kind` tells it.
pub fn is_file(entry: &DirEntry) -> bool {
    kind(entry).is_some_and(|(_, is_file)| is_file)
}

/// One `read_dir` of a folder: its RAW files (or its JPEGs, see
/// `Media`) sorted by file name, and the sidecars of the format it
/// was listed for as `(lower-cased name, path)`, not yet statted (see
/// `stat_sidecars`), none for a JPEG listing.
#[derive(Debug, PartialEq)]
pub struct Listing {
    pub files: Vec<String>,
    pub sidecars: Vec<(String, PathBuf)>,
}

/// Tells a RAW or JPEG file from the entry's `file_type()`, which comes with
/// the directory listing, rather than a `stat` per entry, which under disk
/// contention (a scan reading the same drive) costs seconds for a few hundred
/// files. A symlinked RAW or JPEG is still listed: only a symlink pays one
/// extra `stat` to follow it.
pub fn read_listing(dir: &Path, format: Option<SidecarFormat>) -> Result<Listing, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut media = Media::default();
    let mut sidecars = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if is_file(&entry) && media.add(&path, path.clone()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if format.is_some_and(|f| f.matches(&name)) {
            sidecars.push((name.to_lowercase(), path));
        }
    }
    let (mut files, jpegs) = media.listed();
    // A JPEG folder is view-only: nothing next to a JPEG is read or written.
    if jpegs {
        sidecars.clear();
    }
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    let files = files
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(Listing { files, sidecars })
}

/// The stats `reconcile_sidecars_of` compares, keyed by lower-cased file
/// name. A sidecar gone since the listing is left out.
pub fn stat_sidecars(sidecars: &[(String, PathBuf)]) -> HashMap<String, SidecarStat> {
    sidecars
        .iter()
        .filter_map(|(name, path)| {
            let stat = index::stat(path).ok()?;
            Some((name.clone(), (stat.path, stat.size, stat.mtime_ns)))
        })
        .collect()
}

/// Resolve symlinks and normalize a folder path so the same folder reached
/// through different spellings (a trailing separator, a symlinked parent,
/// `/tmp` vs `/private/tmp` on macOS) shares one row set in the index. Falls
/// back to the original string when canonicalization fails (e.g. the folder
/// was removed between picking and scanning).
pub fn canonicalize(dir: &str) -> String {
    std::fs::canonicalize(dir)
        .map_or_else(|_| dir.to_string(), |p| p.to_string_lossy().into_owned())
}
