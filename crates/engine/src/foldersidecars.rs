//! The folder tree's `Rewrite Sidecars from Index…` and `Delete Sidecars…`:
//! counting and then rewriting the sidecars of one folder from its `ratings`
//! rows, or moving them to the Trash. The scan check and the sidecar
//! writer's drain are the caller's.
//!
//! Every write goes through the sidecar writer, which patches only the
//! judgment fields of an existing sidecar (so Lightroom's develop settings
//! and PhotoLab's corrections survive) and mints one only for a row holding
//! a judgment. A file with an existing sidecar of the format's kinds larger
//! than `MAX_SIDECAR_BYTES` is skipped, as a folder open never reads one. A delete is recorded in `trash::Runs` like a `Move to Trash`
//! run, so an undo takes it back and a redo moves it again. Only the current
//! `SidecarFormat`'s kinds are touched.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use handpic_core::xmp::LabelNames;
use handpic_core::Flag;
use serde::Serialize;

use crate::index::{self, DirtyRow, Index};
use crate::listing::{read_listing, stat_sidecars};
use crate::sidecar::{SidecarFormat, Writer, MAX_SIDECAR_BYTES};
use crate::trash::{self, Failure, Group, Runs, Trashed};

/// Longest a rewrite waits for the writer to drain. `DRAIN_TIMEOUT` is for
/// quit; a folder of thousands of files on Windows takes minutes (each write
/// is a read, a temp write, `sync_all` and a rename with retries).
pub const REWRITE_DRAIN: Duration = Duration::from_secs(10 * 60);

/// How many rows a rewrite hands to the writer before it waits for them and
/// reports its progress.
pub const REWRITE_BATCH: usize = 32;

/// What the rewrite's confirmation dialog counts: the
/// rows with a judgment, the rows without one (whose existing sidecar is
/// patched to "no judgment"), the skipped listed RAWs (those with no
/// `ratings` row, no judgment on record even when the scan indexed the file,
/// and those with an oversize sidecar) and the format written (`SidecarFormat::setting`).
#[derive(Debug, PartialEq, Serialize)]
pub struct Preview {
    pub judged: usize,
    pub unjudged: usize,
    pub skipped: usize,
    pub format: &'static str,
}

/// What a rewrite reports: the files handed to the writer whose
/// rows came back clean, those still dirty after the drain, the files
/// skipped as `Preview` counts them, and whether the
/// drain finished within `REWRITE_DRAIN`. A row still dirty is retried on the
/// next open of the folder; one a judgment re-dirtied during the drain is
/// counted as failed too.
#[derive(Debug, PartialEq, Serialize)]
pub struct Summary {
    pub written: usize,
    pub failed: usize,
    pub skipped: usize,
    pub flushed: bool,
}

fn folder_name(dir: &str) -> String {
    Path::new(dir)
        .file_name()
        .map_or_else(|| dir.to_string(), |n| n.to_string_lossy().into_owned())
}

fn judged((_, rating, flag, label, _): &DirtyRow) -> bool {
    rating.is_some_and(|r| r != 0) || *flag != Flag::None || label.is_some()
}

/// The `ratings` rows of `dir` to rewrite, those of the RAWs the folder lists,
/// and the number of listed RAWs skipped. A RAW without a row is skipped. Such
/// a row exists once a file is judged or its sidecar is read at a folder open,
/// so a scanned file never judged and with no sidecar is counted here. A row
/// whose RAW has an existing sidecar of `format`'s kinds larger than
/// `MAX_SIDECAR_BYTES` is skipped too: a folder open never read it, so
/// patching it would overwrite an edit unread. A row whose RAW is no longer
/// listed is left out, so no sidecar is minted next to a file that is gone. A
/// folder listing no RAW (a JPEG folder is view-only) and one with no row for
/// any listed RAW are refused.
pub fn targets(
    dir: &str,
    index: &Arc<Mutex<Index>>,
    format: SidecarFormat,
) -> Result<(Vec<DirtyRow>, usize), String> {
    let name = folder_name(dir);
    let listing = read_listing(Path::new(dir), Some(format))?;
    let raws: HashSet<&str> = listing
        .files
        .iter()
        .filter(|path| handpic_core::scan::is_raw_file(Path::new(path)))
        .map(String::as_str)
        .collect();
    if raws.is_empty() {
        return Err(format!(
            "{name}: rewriting sidecars applies to RAW files only"
        ));
    }
    let rows: Vec<DirtyRow> = index::lock(index)
        .rows_of(dir)?
        .into_iter()
        .filter(|(path, _, _, _, _)| raws.contains(path.as_str()))
        .collect();
    if rows.is_empty() {
        return Err(format!("No judgments indexed for {name}"));
    }
    let sidecars = stat_sidecars(&listing.sidecars);
    let rows: Vec<DirtyRow> = rows
        .into_iter()
        .filter(|(path, ..)| {
            trash::sidecars_of(format, path, &sidecars)
                .iter()
                .all(|(_, size, _)| *size <= MAX_SIDECAR_BYTES)
        })
        .collect();
    let skipped = raws.len() - rows.len();
    Ok((rows, skipped))
}

/// The counts of `targets` for `dir`, which is canonical.
pub fn preview(
    dir: &str,
    index: &Arc<Mutex<Index>>,
    format: SidecarFormat,
) -> Result<Preview, String> {
    let (rows, skipped) = targets(dir, index, format)?;
    let judged = rows.iter().filter(|row| judged(row)).count();
    Ok(Preview {
        judged,
        unjudged: rows.len() - judged,
        skipped,
        format: format.setting(),
    })
}

/// Mark the rows of `targets` dirty, hand them to `writer` with no debounce,
/// `REWRITE_BATCH` at a time, and wait for each batch to drain, at most
/// `bound` in all; once the bound is reached the rest are handed over
/// without a wait. `progress` is called after each batch drained with the
/// rows written and their total. `dir` is canonical.
pub fn rewrite(
    dir: &str,
    index: &Arc<Mutex<Index>>,
    writer: &Writer,
    format: SidecarFormat,
    names: &LabelNames,
    bound: Duration,
    mut progress: impl FnMut(usize, usize),
) -> Result<Summary, String> {
    let (rows, skipped) = targets(dir, index, format)?;
    let paths: Vec<String> = rows.into_iter().map(|(path, ..)| path).collect();
    let dirty = index::lock(index).mark_dirty(dir, &paths)?;
    let total = dirty.len();
    let deadline = Instant::now() + bound;
    let mut flushed = true;
    let mut done = 0;
    for batch in dirty.chunks(REWRITE_BATCH) {
        for (path, rating, flag, label, label_known) in batch.iter().cloned() {
            writer.set_now(
                PathBuf::from(path),
                rating,
                flag,
                label,
                label_known,
                format,
                names.clone(),
            )?;
        }
        if flushed {
            flushed = writer.flush(deadline.saturating_duration_since(Instant::now()));
            if flushed {
                done += batch.len();
                progress(done, total);
            }
        }
    }
    let wanted: HashSet<&str> = paths.iter().map(String::as_str).collect();
    let failed = index::lock(index)
        .dirty_rows(dir)?
        .iter()
        .filter(|(path, _, _, _, _)| wanted.contains(path.as_str()))
        .count();
    Ok(Summary {
        written: total.saturating_sub(failed),
        failed,
        skipped,
        flushed,
    })
}

/// One sidecar `Delete Sidecars…` moves: the listed RAW it belongs to, its
/// kind, its path on disk and its size.
#[derive(Debug, PartialEq)]
pub struct Doomed {
    raw: String,
    kind: SidecarFormat,
    path: PathBuf,
    bytes: u64,
}

/// One kind's line of the delete dialog: how many sidecars of it there are
/// and their size, formatted as the platform's file manager would.
#[derive(Debug, PartialEq, Serialize)]
pub struct KindCount {
    pub kind: &'static str,
    pub count: usize,
    pub size_text: String,
}

/// What the delete's confirmation dialog counts: the
/// sidecars per kind of the current format (a kind with none left out) and
/// the totals.
#[derive(Debug, PartialEq, Serialize)]
pub struct DeletePreview {
    pub kinds: Vec<KindCount>,
    pub count: usize,
    pub size_text: String,
}

/// How the refusals name the current format's sidecars.
fn kinds_name(format: SidecarFormat) -> &'static str {
    match format {
        SidecarFormat::Xmp => "XMP",
        SidecarFormat::Dop => ".dop",
        SidecarFormat::Both => "XMP or .dop",
    }
}

/// The sidecars of the current format's kinds that exist for the RAWs `dir`
/// lists, each RAW's in `kinds()` order. A sidecar of a RAW that is not
/// listed is left alone, as is any file of the other format. A folder listing
/// no RAW (a JPEG folder is view-only) and one with no such sidecar are
/// refused.
pub fn doomed(dir: &str, format: SidecarFormat) -> Result<Vec<Doomed>, String> {
    let name = folder_name(dir);
    let listing = read_listing(Path::new(dir), Some(format))?;
    let raws: Vec<&String> = listing
        .files
        .iter()
        .filter(|path| handpic_core::scan::is_raw_file(Path::new(path)))
        .collect();
    if raws.is_empty() {
        return Err(format!(
            "{name}: deleting sidecars applies to RAW files only"
        ));
    }
    let sidecars = stat_sidecars(&listing.sidecars);
    let mut found = Vec::new();
    for raw in raws {
        for kind in format.kinds() {
            for (path, size, _) in trash::sidecars_of(*kind, raw, &sidecars) {
                found.push(Doomed {
                    raw: raw.clone(),
                    kind: *kind,
                    path: path.clone(),
                    bytes: u64::try_from(*size).unwrap_or(0),
                });
            }
        }
    }
    if found.is_empty() {
        return Err(format!("No {} sidecars in {name}", kinds_name(format)));
    }
    Ok(found)
}

/// The counts of `doomed` for the dialog, `size_text` formatting bytes.
pub fn delete_preview(
    doomed: &[Doomed],
    format: SidecarFormat,
    size_text: impl Fn(u64) -> String,
) -> DeletePreview {
    let kinds = format
        .kinds()
        .iter()
        .filter_map(|kind| {
            let of_kind: Vec<&Doomed> = doomed.iter().filter(|d| d.kind == *kind).collect();
            (!of_kind.is_empty()).then(|| KindCount {
                kind: kind.setting(),
                count: of_kind.len(),
                size_text: size_text(of_kind.iter().map(|d| d.bytes).sum()),
            })
        })
        .collect();
    DeletePreview {
        kinds,
        count: doomed.len(),
        size_text: size_text(doomed.iter().map(|d| d.bytes).sum()),
    }
}

/// Move every sidecar of `doomed` with `mover`, given one sidecar-only
/// `Group` per sidecar (`trash::run`, or `trash::run_chunked` on Windows),
/// which never stops at a failure. A failure is reported under the RAW's
/// path (the meta pane keys its errors by the file shown) and leaves the
/// sidecar where it is. Returns the summary (`moved` lists the sidecars),
/// every file moved in order with the RAW it belongs to (`Trashed::owner`,
/// which a redo clears again), for `Runs`, and the RAWs that lost a sidecar,
/// whose rows are cleared.
pub fn delete(
    doomed: Vec<Doomed>,
    mover: impl FnOnce(Vec<Group>) -> (trash::Summary, Vec<Trashed>),
) -> (trash::Summary, Vec<Trashed>, Vec<String>) {
    let raw_of: Vec<(String, String)> = doomed
        .iter()
        .map(|d| (d.path.to_string_lossy().into_owned(), d.raw.clone()))
        .collect();
    let raw = |sidecar: &str| {
        raw_of
            .iter()
            .find(|(path, _)| path == sidecar)
            .map_or_else(|| sidecar.to_string(), |(_, raw)| raw.clone())
    };
    let groups = doomed
        .into_iter()
        .map(|d| Group {
            raw: d.path,
            sidecars: Vec::new(),
        })
        .collect();
    let (mut summary, mut moved) = mover(groups);
    for trashed in &mut moved {
        let path = trashed.path.to_string_lossy();
        trashed.owner = raw_of
            .iter()
            .find(|(sidecar, _)| *sidecar == path)
            .map(|(_, raw)| PathBuf::from(raw));
    }
    summary.failed = summary
        .failed
        .into_iter()
        .map(|failure| {
            let name = Path::new(&failure.path).file_name().map_or_else(
                || failure.path.clone(),
                |n| n.to_string_lossy().into_owned(),
            );
            Failure {
                path: raw(&failure.path),
                message: format!("{name}: {}", failure.message),
            }
        })
        .collect();
    let mut cleared: Vec<String> = Vec::new();
    for sidecar in &summary.moved {
        let raw = raw(sidecar);
        if !cleared.contains(&raw) {
            cleared.push(raw);
        }
    }
    (summary, moved, cleared)
}

/// Move the sidecars of the canonical `dir` to the OS Trash, record the run
/// in `runs` (its id in the summary) and clear the judgments of the RAWs
/// that lost a sidecar in `index`, so a closed folder does not keep them
/// until its next open. Does not check for a scan: the caller refuses a
/// running scan and holds that refusal across the call (the app's
/// idle gate), and drains the sidecar writer first, since a judgment still in
/// its debounce window would mint a sidecar right after the delete.
/// The sidecars move one at a time, or up to `trash::CHUNK` per shell
/// operation on Windows (`trash::move_groups`). `progress` is called after
/// each sidecar, or each chunk on Windows, with the sidecars done and their
/// total.
pub fn delete_run(
    dir: &str,
    format: SidecarFormat,
    index: Option<&Mutex<Index>>,
    runs: &Runs,
    progress: impl FnMut(usize, usize),
) -> Result<trash::Summary, String> {
    let doomed = doomed(dir, format)?;
    let (mut summary, moved, cleared) =
        delete(doomed, |groups| trash::move_groups(groups, progress));
    summary.run_id = runs.record(moved);
    if let Some(index) = index {
        index::lock(index).clear_judgments(dir, &cleared)?;
    }
    log::info!("moved the sidecars of {dir} to the trash: {summary:?}");
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use handpic_core::{dop, xmp};

    const PHOTOLAB: &[u8] = include_bytes!("../../core/src/fixtures/dop/_DSC0003.ARW.dop");

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "handpic-foldersidecars-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct Fixture {
        dir: PathBuf,
        key: String,
        index: Arc<Mutex<Index>>,
        writer: Writer,
        errors: Arc<Mutex<Vec<String>>>,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let dir = temp_dir(name);
            let index = Arc::new(Mutex::new(Index::open(&dir.join("index.sqlite")).unwrap()));
            let errors = Arc::new(Mutex::new(Vec::new()));
            let sink = errors.clone();
            let writer = Writer::spawn(index.clone(), move |path, message| {
                index::lock(&sink).push(format!("{}: {message}", path.display()));
            });
            Self {
                key: dir.to_string_lossy().into_owned(),
                dir,
                index,
                writer,
                errors,
            }
        }

        fn raw(&self, name: &str) -> PathBuf {
            let path = self.dir.join(name);
            std::fs::write(&path, b"not really an ARW").unwrap();
            path
        }

        /// A clean row, as a folder open stores what it parsed.
        fn row(&self, path: &Path, rating: Option<i8>, flag: Flag, label: Option<&str>) {
            let path = path.to_string_lossy();
            let mut index = index::lock(&self.index);
            index
                .set_rating(&self.key, &path, rating, flag, label, true)
                .unwrap();
            index
                .mark_written(&path, rating, flag, label, true, None)
                .unwrap();
        }

        fn rewrite(&self, format: SidecarFormat) -> Summary {
            self.rewrite_counting(format, |_, _| {})
        }

        fn rewrite_counting(
            &self,
            format: SidecarFormat,
            progress: impl FnMut(usize, usize),
        ) -> Summary {
            let summary = rewrite(
                &self.key,
                &self.index,
                &self.writer,
                format,
                &LabelNames::default(),
                Duration::from_secs(30),
                progress,
            )
            .unwrap();
            let errors = index::lock(&self.errors);
            assert!(errors.is_empty(), "writer errors: {errors:?}");
            summary
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn lightroom_xmp() -> String {
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
   crs:Exposure2012="+0.35"
   crs:Contrast2012="+12">
   <xmp:Rating>1</xmp:Rating>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
            .to_string()
    }

    #[test]
    fn a_rewrite_reports_progress_per_batch() {
        let f = Fixture::new("rewrite-progress");
        for at in 0..REWRITE_BATCH + 1 {
            let raw = f.raw(&format!("{at:03}.ARW"));
            f.row(&raw, Some(3), Flag::None, None);
        }
        let mut progress = Vec::new();

        let summary = f.rewrite_counting(SidecarFormat::Xmp, |done, total| {
            progress.push((done, total));
        });

        assert_eq!(summary.written, REWRITE_BATCH + 1);
        assert!(summary.flushed);
        assert_eq!(
            progress,
            [
                (REWRITE_BATCH, REWRITE_BATCH + 1),
                (REWRITE_BATCH + 1, REWRITE_BATCH + 1)
            ]
        );
    }

    #[test]
    fn a_judged_row_with_no_sidecar_mints_one() {
        let f = Fixture::new("mint");
        let a = f.raw("a.ARW");
        f.row(&a, Some(4), Flag::None, None);

        let summary = f.rewrite(SidecarFormat::Xmp);

        assert_eq!(
            summary,
            Summary {
                written: 1,
                failed: 0,
                skipped: 0,
                flushed: true
            }
        );
        let bytes = std::fs::read(xmp::sidecar_path(&a)).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(4));
        assert!(!dop::sidecar_path(&a).exists(), "only the current format");
    }

    #[test]
    fn existing_sidecars_are_patched_with_their_foreign_content_intact() {
        let f = Fixture::new("patch");
        let a = f.raw("_DSC0003.ARW");
        std::fs::write(xmp::sidecar_path(&a), lightroom_xmp()).unwrap();
        std::fs::write(dop::sidecar_path(&a), PHOTOLAB).unwrap();
        f.row(&a, Some(5), Flag::Reject, None);

        f.rewrite(SidecarFormat::Both);

        let text = std::fs::read_to_string(xmp::sidecar_path(&a)).unwrap();
        assert_eq!(xmp::read_rating(text.as_bytes()).unwrap(), Some(5));
        assert_eq!(xmp::read_flag(text.as_bytes()).unwrap(), Flag::Reject);
        assert!(text.contains(r#"crs:Exposure2012="+0.35""#), "{text}");
        assert!(text.contains(r#"crs:Contrast2012="+12""#), "{text}");

        let original = String::from_utf8(PHOTOLAB.to_vec()).unwrap();
        let start = original.find("Settings = {").unwrap();
        let end = original.find("ShotDate").unwrap();
        let text = std::fs::read_to_string(dop::sidecar_path(&a)).unwrap();
        assert_eq!(dop::read_rating(text.as_bytes()).unwrap(), Some(5));
        assert_eq!(dop::read_flag(text.as_bytes()).unwrap(), Flag::Reject);
        assert!(text.contains(&original[start..end]), "{text}");
    }

    #[test]
    fn an_unjudged_row_clears_an_existing_sidecar_and_mints_nothing_without_one() {
        let f = Fixture::new("clear");
        let a = f.raw("a.ARW");
        let b = f.raw("b.ARW");
        std::fs::write(xmp::sidecar_path(&a), lightroom_xmp()).unwrap();
        f.row(&a, None, Flag::None, None);
        f.row(&b, None, Flag::None, None);

        let summary = f.rewrite(SidecarFormat::Xmp);

        assert_eq!(summary.written, 2);
        let bytes = std::fs::read(xmp::sidecar_path(&a)).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(0));
        assert_eq!(xmp::read_flag(&bytes).unwrap(), Flag::None);
        assert!(!xmp::sidecar_path(&b).exists());
    }

    #[test]
    fn a_listed_raw_without_a_row_is_skipped_and_an_unlisted_row_is_not_written() {
        let f = Fixture::new("skip");
        let a = f.raw("a.ARW");
        let b = f.raw("b.ARW");
        let gone = f.dir.join("gone.ARW");
        f.row(&a, Some(2), Flag::None, None);
        f.row(&gone, Some(3), Flag::Pick, None);

        assert_eq!(
            preview(&f.key, &f.index, SidecarFormat::Xmp).unwrap(),
            Preview {
                judged: 1,
                unjudged: 0,
                skipped: 1,
                format: "xmp"
            }
        );
        let summary = f.rewrite(SidecarFormat::Xmp);

        assert_eq!(summary.written, 1);
        assert!(xmp::sidecar_path(&a).exists());
        assert!(!xmp::sidecar_path(&b).exists());
        assert!(!xmp::sidecar_path(&gone).exists());
    }

    #[test]
    fn a_raw_the_scan_indexed_with_no_ratings_row_is_skipped_not_unjudged() {
        let f = Fixture::new("scanned");
        let a = f.raw("a.ARW");
        let b = f.raw("b.ARW");
        f.row(&a, Some(2), Flag::None, None);
        index::lock(&f.index)
            .write_batch(
                &f.key,
                &[(
                    index::stat(&b).unwrap(),
                    Err(handpic_core::scan::Failure::from("not really an ARW")),
                )],
            )
            .unwrap();
        assert!(index::lock(&f.index)
            .entry(&b.to_string_lossy())
            .unwrap()
            .is_some());

        assert_eq!(
            preview(&f.key, &f.index, SidecarFormat::Xmp).unwrap(),
            Preview {
                judged: 1,
                unjudged: 0,
                skipped: 1,
                format: "xmp"
            }
        );
    }

    #[test]
    fn under_both_both_kinds_are_written() {
        let f = Fixture::new("both");
        let a = f.raw("a.ARW");
        f.row(&a, None, Flag::None, Some("Red"));

        assert_eq!(
            preview(&f.key, &f.index, SidecarFormat::Both)
                .unwrap()
                .format,
            "both"
        );
        f.rewrite(SidecarFormat::Both);

        let names = LabelNames::default();
        let bytes = std::fs::read(xmp::sidecar_path(&a)).unwrap();
        assert_eq!(
            xmp::read_label(&bytes, &names).unwrap().as_deref(),
            Some("Red")
        );
        let bytes = std::fs::read(dop::sidecar_path(&a)).unwrap();
        assert_eq!(dop::read_label(&bytes).unwrap().as_deref(), Some("Red"));
    }

    #[test]
    fn a_folder_with_no_row_or_no_raw_is_refused() {
        let f = Fixture::new("refuse");
        f.raw("a.ARW");
        let err = preview(&f.key, &f.index, SidecarFormat::Xmp).unwrap_err();
        assert_eq!(
            err,
            format!(
                "No judgments indexed for handpic-foldersidecars-refuse-{}",
                std::process::id()
            )
        );

        let jpegs = Fixture::new("refuse-jpeg");
        std::fs::write(jpegs.dir.join("a.jpg"), b"not really a JPEG").unwrap();
        let err = preview(&jpegs.key, &jpegs.index, SidecarFormat::Xmp).unwrap_err();
        assert!(err.ends_with("applies to RAW files only"), "{err}");
    }

    fn oversize(path: &Path, extra: usize) -> Vec<u8> {
        let bytes = vec![b'x'; MAX_SIDECAR_BYTES as usize + extra];
        std::fs::write(path, &bytes).unwrap();
        bytes
    }

    /// The preview and then the summary of a rewrite of `f`'s folder, whose
    /// skipped figures must agree.
    fn preview_and_rewrite(f: &Fixture, format: SidecarFormat) -> (Preview, Summary) {
        let preview = preview(&f.key, &f.index, format).unwrap();
        let summary = f.rewrite(format);
        assert_eq!(preview.skipped, summary.skipped);
        (preview, summary)
    }

    fn assert_clean(f: &Fixture, path: &Path) {
        let key = path.to_string_lossy().into_owned();
        assert!(!index::lock(&f.index).row_flags(&f.key).unwrap()[&key].dirty);
    }

    #[test]
    fn an_oversize_xmp_under_xmp_is_skipped_and_left_unchanged() {
        let f = Fixture::new("oversize-xmp");
        let a = f.raw("a.ARW");
        let b = f.raw("b.ARW");
        let big = oversize(&xmp::sidecar_path(&a), 1);
        f.row(&a, Some(4), Flag::Pick, None);
        f.row(&b, Some(2), Flag::None, None);

        let (preview, summary) = preview_and_rewrite(&f, SidecarFormat::Xmp);

        assert_eq!(
            preview,
            Preview {
                judged: 1,
                unjudged: 0,
                skipped: 1,
                format: "xmp"
            }
        );
        assert_eq!(
            summary,
            Summary {
                written: 1,
                failed: 0,
                skipped: 1,
                flushed: true
            }
        );
        assert_eq!(std::fs::read(xmp::sidecar_path(&a)).unwrap(), big);
        assert_clean(&f, &a);
        assert!(xmp::sidecar_path(&b).exists());
    }

    #[test]
    fn an_oversize_dop_under_dop_is_skipped_and_left_unchanged() {
        let f = Fixture::new("oversize-dop");
        let a = f.raw("a.ARW");
        let big = oversize(&dop::sidecar_path(&a), 1);
        f.row(&a, None, Flag::None, None);

        let (preview, summary) = preview_and_rewrite(&f, SidecarFormat::Dop);

        assert_eq!(
            (preview.judged, preview.unjudged, preview.skipped),
            (0, 0, 1)
        );
        assert_eq!((summary.written, summary.failed), (0, 0));
        assert_eq!(std::fs::read(dop::sidecar_path(&a)).unwrap(), big);
        assert_clean(&f, &a);
    }

    #[test]
    fn under_both_one_oversize_kind_skips_the_file_and_neither_sidecar_is_written() {
        let f = Fixture::new("oversize-both");
        let a = f.raw("_DSC0003.ARW");
        std::fs::write(xmp::sidecar_path(&a), lightroom_xmp()).unwrap();
        let big = oversize(&dop::sidecar_path(&a), 1);
        f.row(&a, Some(5), Flag::Reject, None);

        let (preview, summary) = preview_and_rewrite(&f, SidecarFormat::Both);

        assert_eq!((preview.judged, preview.skipped), (0, 1));
        assert_eq!((summary.written, summary.skipped), (0, 1));
        assert_eq!(
            std::fs::read_to_string(xmp::sidecar_path(&a)).unwrap(),
            lightroom_xmp()
        );
        assert_eq!(std::fs::read(dop::sidecar_path(&a)).unwrap(), big);
        assert_clean(&f, &a);
    }

    #[test]
    fn an_oversize_sidecar_of_the_other_format_does_not_skip_the_file() {
        let f = Fixture::new("oversize-other");
        let a = f.raw("a.ARW");
        let b = f.raw("b.ARW");
        let big_dop = oversize(&dop::sidecar_path(&a), 1);
        let big_xmp = oversize(&xmp::sidecar_path(&b), 1);
        f.row(&a, Some(3), Flag::None, None);
        f.row(&b, Some(2), Flag::None, None);

        let (preview, summary) = preview_and_rewrite(&f, SidecarFormat::Xmp);
        assert_eq!((preview.judged, preview.skipped), (1, 1));
        assert_eq!((summary.written, summary.skipped), (1, 1));
        let bytes = std::fs::read(xmp::sidecar_path(&a)).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(3));
        assert_eq!(std::fs::read(dop::sidecar_path(&a)).unwrap(), big_dop);
        assert_eq!(std::fs::read(xmp::sidecar_path(&b)).unwrap(), big_xmp);

        std::fs::remove_file(xmp::sidecar_path(&b)).unwrap();
        std::fs::remove_file(dop::sidecar_path(&a)).unwrap();
        let big_xmp = oversize(&xmp::sidecar_path(&a), 1);
        let (preview, summary) = preview_and_rewrite(&f, SidecarFormat::Dop);
        assert_eq!((preview.judged, preview.skipped), (2, 0));
        assert_eq!((summary.written, summary.skipped), (2, 0));
        let bytes = std::fs::read(dop::sidecar_path(&a)).unwrap();
        assert_eq!(dop::read_rating(&bytes).unwrap(), Some(3));
        assert_eq!(std::fs::read(xmp::sidecar_path(&a)).unwrap(), big_xmp);
    }

    #[test]
    fn a_sidecar_of_exactly_the_limit_is_not_skipped() {
        let f = Fixture::new("oversize-limit");
        let a = f.raw("a.ARW");
        let text = lightroom_xmp();
        let end = text.find("<?xpacket end").unwrap();
        let padding = " ".repeat(MAX_SIDECAR_BYTES as usize - text.len());
        let text = format!("{}{padding}{}", &text[..end], &text[end..]);
        assert_eq!(text.len() as i64, MAX_SIDECAR_BYTES);
        std::fs::write(xmp::sidecar_path(&a), text).unwrap();
        f.row(&a, Some(4), Flag::None, None);

        assert_eq!(
            preview(&f.key, &f.index, SidecarFormat::Xmp)
                .unwrap()
                .skipped,
            0
        );
        let summary = f.rewrite(SidecarFormat::Xmp);

        assert_eq!((summary.written, summary.skipped), (1, 0));
        let bytes = std::fs::read(xmp::sidecar_path(&a)).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(4));
    }

    #[test]
    fn a_folder_whose_every_row_is_oversize_offers_a_rewrite_of_none() {
        let f = Fixture::new("oversize-all");
        let a = f.raw("a.ARW");
        let b = f.raw("b.ARW");
        let big = oversize(&xmp::sidecar_path(&a), 1);
        f.row(&a, Some(1), Flag::None, None);

        let (preview, summary) = preview_and_rewrite(&f, SidecarFormat::Xmp);

        assert_eq!(
            preview,
            Preview {
                judged: 0,
                unjudged: 0,
                skipped: 2,
                format: "xmp"
            }
        );
        assert_eq!(
            summary,
            Summary {
                written: 0,
                failed: 0,
                skipped: 2,
                flushed: true
            }
        );
        assert_eq!(std::fs::read(xmp::sidecar_path(&a)).unwrap(), big);
        assert!(!xmp::sidecar_path(&b).exists());
    }

    fn put(dir: &Path, name: &str, bytes: usize) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, vec![b'x'; bytes]).unwrap();
        path
    }

    /// The file names of `doomed`, with the name of the RAW each belongs to.
    fn names(doomed: &[Doomed]) -> Vec<(String, String)> {
        let name = |path: &Path| path.file_name().unwrap().to_string_lossy().into_owned();
        doomed
            .iter()
            .map(|d| (name(Path::new(&d.raw)), name(&d.path)))
            .collect()
    }

    #[test]
    fn the_collection_takes_the_current_formats_sidecars_of_listed_raws() {
        let f = Fixture::new("collect");
        f.raw("a.ARW");
        f.raw("b.ARW");
        put(&f.dir, "a.xmp", 10);
        put(&f.dir, "a.ARW.dop", 20);
        put(&f.dir, "B.XMP", 30);
        put(&f.dir, "gone.xmp", 40);
        put(&f.dir, "gone.ARW.dop", 50);

        let pair = |raw: &str, sidecar: &str| (raw.to_string(), sidecar.to_string());
        assert_eq!(
            names(&doomed(&f.key, SidecarFormat::Xmp).unwrap()),
            [pair("a.ARW", "a.xmp"), pair("b.ARW", "B.XMP")]
        );
        assert_eq!(
            names(&doomed(&f.key, SidecarFormat::Dop).unwrap()),
            [pair("a.ARW", "a.ARW.dop")]
        );
        let both = doomed(&f.key, SidecarFormat::Both).unwrap();
        assert_eq!(
            names(&both),
            [
                pair("a.ARW", "a.xmp"),
                pair("a.ARW", "a.ARW.dop"),
                pair("b.ARW", "B.XMP")
            ]
        );
        assert_eq!(
            delete_preview(&both, SidecarFormat::Both, |bytes| format!("{bytes} B")),
            DeletePreview {
                kinds: vec![
                    KindCount {
                        kind: "xmp",
                        count: 2,
                        size_text: "40 B".to_string()
                    },
                    KindCount {
                        kind: "dop",
                        count: 1,
                        size_text: "20 B".to_string()
                    },
                ],
                count: 3,
                size_text: "60 B".to_string(),
            }
        );
    }

    #[test]
    fn a_folder_with_no_sidecar_of_the_format_or_no_raw_is_refused() {
        let f = Fixture::new("delete-refuse");
        f.raw("a.ARW");
        put(&f.dir, "a.ARW.dop", 1);
        let err = doomed(&f.key, SidecarFormat::Xmp).unwrap_err();
        assert!(err.starts_with("No XMP sidecars in "), "{err}");

        let jpegs = Fixture::new("delete-refuse-jpeg");
        put(&jpegs.dir, "a.jpg", 1);
        put(&jpegs.dir, "a.xmp", 1);
        let err = doomed(&jpegs.key, SidecarFormat::Both).unwrap_err();
        assert!(err.ends_with("applies to RAW files only"), "{err}");
    }

    #[test]
    fn the_run_moves_in_order_and_reports_a_failure_under_its_raw() {
        let f = Fixture::new("delete-run");
        let trash = f.dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        let a = f.raw("a.ARW");
        let fail = f.raw("fail.ARW");
        let a_xmp = put(&f.dir, "a.xmp", 1);
        let a_dop = put(&f.dir, "a.ARW.dop", 1);
        let fail_xmp = put(&f.dir, "fail.xmp", 1);
        let doomed = doomed(&f.key, SidecarFormat::Both).unwrap();
        let mut progress = Vec::new();

        let (summary, moved, cleared) = delete(doomed, |groups| {
            trash::run(
                groups,
                |path| {
                    let name = path.file_name().unwrap();
                    if name.to_string_lossy().contains("fail") {
                        return Err("refused".to_string());
                    }
                    let to = trash.join(name);
                    std::fs::rename(path, &to).map_err(|e| e.to_string())?;
                    Ok(Some(to))
                },
                |done, total| progress.push((done, total)),
            )
        });

        let key = |path: &Path| path.to_string_lossy().into_owned();
        assert_eq!(
            moved.iter().map(|t| t.path.clone()).collect::<Vec<_>>(),
            [a_xmp.clone(), a_dop.clone()]
        );
        assert_eq!(summary.moved, [key(&a_xmp), key(&a_dop)]);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].path, key(&fail));
        assert_eq!(summary.failed[0].message, "fail.xmp: refused");
        assert_eq!(cleared, [key(&a)]);
        assert_eq!(progress, [(1, 3), (2, 3), (3, 3)]);
        assert!(fail_xmp.exists());
        assert!(!a_xmp.exists());
        assert!(trash.join("a.ARW.dop").exists());
    }

    #[test]
    fn a_redo_with_the_folder_closed_clears_the_rows_the_undo_left_judged() {
        let f = Fixture::new("delete-redo");
        let trash = f.dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        let a = f.raw("a.ARW");
        let b = f.raw("b.ARW");
        put(&f.dir, "a.xmp", 1);
        put(&f.dir, "b.xmp", 1);
        let judge = || {
            f.row(&a, Some(3), Flag::Pick, None);
            f.row(&b, Some(2), Flag::Reject, None);
        };
        judge();
        let to_trash = |path: &Path| -> Result<Option<PathBuf>, String> {
            let to = trash.join(path.file_name().unwrap());
            std::fs::rename(path, &to).map_err(|e| e.to_string())?;
            Ok(Some(to))
        };
        let runs = Runs::default();
        let (_, moved, cleared) = delete(doomed(&f.key, SidecarFormat::Xmp).unwrap(), |groups| {
            trash::run(groups, to_trash, |_, _| {})
        });
        let id = runs.record(moved).unwrap();
        index::lock(&f.index)
            .clear_judgments(&f.key, &cleared)
            .unwrap();

        // The undo, as `trash::undo_run` does it, from the fake Trash; a
        // reopen's reconcile then reads the judgments back from the sidecars.
        let run = runs.take(id).unwrap();
        let restored = trash::restore(
            &run,
            |t| Some(trash.join(t.path.file_name().unwrap())),
            Path::exists,
            |t, at| std::fs::rename(at, t.destination()).map_err(|e| e.to_string()),
            |_, _| {},
        );
        assert!(restored.failed.is_empty());
        runs.undone(id, restored.back);
        judge();

        let summary = trash::redo_back(
            &runs,
            Some(&f.index),
            runs.take_undone(id).unwrap(),
            |paths| {
                trash::redo(
                    paths,
                    Path::exists,
                    |path| {
                        if path.file_name().unwrap() == "b.xmp" {
                            return Err("refused".to_string());
                        }
                        to_trash(path)
                    },
                    |_, _| {},
                )
            },
        );

        let key = |path: &Path| path.to_string_lossy().into_owned();
        assert_eq!(summary.moved, [key(&f.dir.join("a.xmp"))]);
        assert_eq!(summary.failed.len(), 1);
        let rows = index::lock(&f.index).rows_of(&f.key).unwrap();
        let row = |path: &Path| {
            rows.iter()
                .find(|(of, ..)| *of == key(path))
                .unwrap()
                .clone()
        };
        assert_eq!(row(&a), (key(&a), None, Flag::None, None, true));
        assert_eq!(row(&b), (key(&b), Some(2), Flag::Reject, None, true));
        let redone = runs.take(summary.run_id.unwrap()).unwrap();
        assert_eq!(redone.moved.len(), 1);
        assert_eq!(redone.moved[0].owner, Some(a.clone()));
    }

    fn file_name(path: &Path) -> String {
        path.file_name().unwrap().to_string_lossy().into_owned()
    }

    /// A mover that renames into `trash`, telling no destination as the
    /// Windows `trash_one` does, and refuses any file whose name holds `fail`.
    fn renamer(trash: &Path) -> impl FnMut(&Path) -> Result<Option<PathBuf>, String> + '_ {
        move |path: &Path| {
            let name = path.file_name().unwrap();
            if name.to_string_lossy().contains("fail") {
                return Err("refused".to_string());
            }
            std::fs::rename(path, trash.join(name)).map_err(|e| e.to_string())?;
            Ok(None)
        }
    }

    /// A `move_many` like the Windows shell: it renames into `trash` all it
    /// can, going on past a `fail` file or one it cannot rename, then errs.
    /// Each call's file names are kept in `calls`.
    fn shell<'a>(
        trash: &'a Path,
        calls: &'a std::cell::RefCell<Vec<Vec<String>>>,
    ) -> impl FnMut(&[PathBuf]) -> Result<(), String> + 'a {
        move |paths: &[PathBuf]| {
            calls
                .borrow_mut()
                .push(paths.iter().map(|path| file_name(path)).collect());
            let mut result = Ok(());
            for path in paths {
                let name = path.file_name().unwrap();
                if name.to_string_lossy().contains("fail")
                    || std::fs::rename(path, trash.join(name)).is_err()
                {
                    result = Err("Some operations were aborted".to_string());
                }
            }
            result
        }
    }

    /// What a delete gave, by file name: the summary's moved sidecars and
    /// failures, the cleared RAWs, the run `Runs` recorded (each file with
    /// where it went and its owner) and the last progress call.
    #[derive(Debug, PartialEq)]
    struct Outcome {
        moved: Vec<String>,
        failed: Vec<(String, String)>,
        cleared: Vec<String>,
        recorded: Vec<(String, Option<PathBuf>, Option<String>)>,
        last_progress: Option<(usize, usize)>,
    }

    /// Delete the `format` sidecars of a folder holding `raws` and
    /// `sidecars`, through `trash::run_chunked` when `chunked`, else
    /// `trash::run`, and return the outcome, the progress calls and the
    /// chunk calls.
    fn delete_outcome(
        name: &str,
        raws: &[&str],
        sidecars: &[&str],
        format: SidecarFormat,
        chunked: bool,
    ) -> (Outcome, Vec<(usize, usize)>, Vec<Vec<String>>) {
        let f = Fixture::new(name);
        let trash = f.dir.join("trash");
        std::fs::create_dir(&trash).unwrap();
        for raw in raws {
            f.raw(raw);
        }
        for sidecar in sidecars {
            put(&f.dir, sidecar, 1);
        }
        let calls = std::cell::RefCell::new(Vec::new());
        let mut progress = Vec::new();
        let (summary, moved, cleared) = delete(doomed(&f.key, format).unwrap(), |groups| {
            let report = |done: usize, total: usize| progress.push((done, total));
            if chunked {
                trash::run_chunked(
                    groups,
                    Path::exists,
                    shell(&trash, &calls),
                    renamer(&trash),
                    report,
                )
            } else {
                trash::run(groups, renamer(&trash), report)
            }
        });
        let runs = Runs::default();
        let run = runs.take(runs.record(moved).unwrap()).unwrap();
        let outcome = Outcome {
            moved: summary
                .moved
                .iter()
                .map(|path| file_name(Path::new(path)))
                .collect(),
            failed: summary
                .failed
                .iter()
                .map(|failure| (file_name(Path::new(&failure.path)), failure.message.clone()))
                .collect(),
            cleared: cleared
                .iter()
                .map(|path| file_name(Path::new(path)))
                .collect(),
            recorded: run
                .moved
                .iter()
                .map(|trashed| {
                    (
                        file_name(&trashed.path),
                        trashed.trashed_at.clone(),
                        trashed.owner.as_deref().map(file_name),
                    )
                })
                .collect(),
            last_progress: progress.last().copied(),
        };
        (outcome, progress, calls.into_inner())
    }

    #[test]
    fn the_chunked_delete_agrees_with_the_per_file_one() {
        let raws = ["a.ARW", "b.ARW", "c.ARW", "fail.ARW"];
        let sidecars = ["a.xmp", "a.ARW.dop", "b.xmp", "fail.xmp"];
        let (per_file, per_file_progress, _) = delete_outcome(
            "delete-per-file",
            &raws,
            &sidecars,
            SidecarFormat::Both,
            false,
        );
        let (chunked, chunked_progress, calls) = delete_outcome(
            "delete-chunked",
            &raws,
            &sidecars,
            SidecarFormat::Both,
            true,
        );

        assert_eq!(chunked, per_file);
        let owned = |sidecar: &str, raw: &str| (sidecar.to_string(), None, Some(raw.to_string()));
        assert_eq!(
            per_file,
            Outcome {
                moved: ["a.xmp", "a.ARW.dop", "b.xmp"].map(String::from).to_vec(),
                failed: vec![("fail.ARW".to_string(), "fail.xmp: refused".to_string())],
                cleared: ["a.ARW", "b.ARW"].map(String::from).to_vec(),
                recorded: vec![
                    owned("a.xmp", "a.ARW"),
                    owned("a.ARW.dop", "a.ARW"),
                    owned("b.xmp", "b.ARW"),
                ],
                last_progress: Some((4, 4)),
            }
        );
        assert_eq!(per_file_progress, [(1, 4), (2, 4), (3, 4), (4, 4)]);
        assert_eq!(chunked_progress, [(4, 4)]);
        assert_eq!(calls, [vec!["a.xmp", "a.ARW.dop", "b.xmp", "fail.xmp"]]);
    }

    #[test]
    fn a_chunked_delete_moves_an_xmp_two_raws_share_once_and_records_it_twice() {
        let (outcome, _, calls) = delete_outcome(
            "delete-shared",
            &["a.ARW", "a.NEF"],
            &["a.xmp"],
            SidecarFormat::Xmp,
            true,
        );

        assert_eq!(calls, [vec!["a.xmp", "a.xmp"]]);
        let owned = ("a.xmp".to_string(), None, Some("a.ARW".to_string()));
        assert_eq!(
            outcome,
            Outcome {
                moved: ["a.xmp", "a.xmp"].map(String::from).to_vec(),
                failed: Vec::new(),
                cleared: vec!["a.ARW".to_string()],
                recorded: vec![owned.clone(), owned],
                last_progress: Some((2, 2)),
            }
        );
    }

    #[test]
    fn clearing_judgments_leaves_the_rows_as_a_gone_sidecar_does() {
        let f = Fixture::new("delete-clear");
        let a = f.raw("a.ARW");
        let b = f.raw("b.ARW");
        f.row(&b, Some(2), Flag::Reject, None);
        let a_key = a.to_string_lossy().into_owned();
        let b_key = b.to_string_lossy().into_owned();
        {
            let mut index = index::lock(&f.index);
            index
                .set_rating(&f.key, &a_key, Some(4), Flag::Pick, Some("Red"), true)
                .unwrap();
            index
                .mark_written(&a_key, Some(4), Flag::Pick, Some("Red"), true, Some((9, 9)))
                .unwrap();
            index
                .set_rating(&f.key, &a_key, Some(5), Flag::Pick, None, true)
                .unwrap();
            assert_eq!(
                index.row_flags(&f.key).unwrap()[&a_key].stat,
                (Some(9), Some(9))
            );
        }

        index::lock(&f.index)
            .clear_judgments(&f.key, std::slice::from_ref(&a_key))
            .unwrap();

        let index = index::lock(&f.index);
        let rows = index.rows_of(&f.key).unwrap();
        let row = |key: &str| rows.iter().find(|(path, ..)| path == key).unwrap().clone();
        assert_eq!(row(&a_key), (a_key.clone(), None, Flag::None, None, true));
        assert_eq!(
            row(&b_key),
            (b_key.clone(), Some(2), Flag::Reject, None, true)
        );
        let flags = index.row_flags(&f.key).unwrap();
        assert_eq!(flags[&a_key].stat, (None, None));
        assert!(!flags[&a_key].dirty);
    }
}
