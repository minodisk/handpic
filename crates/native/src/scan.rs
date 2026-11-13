use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use futures::channel::mpsc::UnboundedSender;
use riffle_backend::{
    index::{self, FaceReady, FileStat, Index, ScanSummary},
    listing::{canonicalize, read_listing, stat_sidecars},
    output,
    sidecar::{self, SidecarError, SidecarFormat, Writer},
};
use riffle_core::{scan::ScanFocus, xmp::LabelNames};

use crate::viewonly;

/// Scan workers inside a file read at once, as the Tauri app's `READ_SLOTS`.
pub const READ_SLOTS: usize = 2;

/// How long a quit waits for the running scan to stop. Workers finish the
/// stage of the file they are on, which a slow or network volume can hold
/// longer; past this the process exits with the scan still running.
pub const STOP_TIMEOUT: Duration = Duration::from_secs(2);

/// Two fewer than the cores, at least one, as the Tauri app's `scan_threads`:
/// two cores stay with the paging path.
pub fn scan_threads() -> usize {
    threads_for(thread::available_parallelism().map_or(4, |n| n.get()))
}

fn threads_for(cores: usize) -> usize {
    cores.saturating_sub(2).max(1)
}

/// Pass 2's workers: an eighth of the cores, at least one, as the Tauri app's
/// `faces_threads`. Paging during pass 2 took +9.4 ms (ARW) and +89 ms (RAF)
/// at p50 on two fewer than the cores and still +13 ms on RAF at a quarter,
/// `decode` and `paint` starved.
pub fn faces_threads() -> usize {
    faces_threads_for(thread::available_parallelism().map_or(4, |n| n.get()))
}

fn faces_threads_for(cores: usize) -> usize {
    (cores / 8).max(1)
}

/// What the scan and the folder watcher tell the view, a scan's messages
/// tagged with the scan they came from so a stopped scan's late messages can
/// be told apart.
pub enum Event {
    Progress {
        scan: u64,
        done: usize,
        total: usize,
        /// The files committed since the previous message.
        ready: Vec<String>,
    },
    Done {
        scan: u64,
        summary: ScanSummary,
    },
    /// Pass 2's progress.
    FacesProgress {
        scan: u64,
        done: usize,
        total: usize,
        /// The files written since the previous message.
        ready: Vec<FaceReady>,
    },
    /// Pass 2 ended, also when it was canceled, had nothing to do or could
    /// not list its files: the scan is over.
    FacesDone {
        scan: u64,
        #[expect(dead_code, reason = "the backend's `scan faces:` line reports it")]
        summary: ScanSummary,
    },
    /// The folder watcher saw the folder change; the canonical path.
    Changed(String),
    /// The sidecar writer failed to write `path`'s sidecar.
    SidecarError {
        path: String,
        message: String,
    },
}

/// One folder's listing: its canonical path, the key of every index row, its
/// RAW files (or JPEGs when it holds no RAW) under that path, and the
/// sidecars of the format listed for, as `read_listing` gives them.
pub struct Listed {
    pub dir: String,
    pub files: Vec<String>,
    pub sidecars: Vec<(String, PathBuf)>,
}

pub fn list(folder: &Path, format: SidecarFormat) -> Result<Listed, String> {
    let dir = canonicalize(&folder.to_string_lossy());
    let started = Instant::now();
    let listing = read_listing(Path::new(&dir), Some(format))?;
    log::info!(
        "scan list: dir={dir} files={} in {}ms",
        listing.files.len(),
        started.elapsed().as_millis()
    );
    Ok(Listed {
        dir,
        files: listing.files,
        sidecars: listing.sidecars,
    })
}

/// What the folder's export folders say of its listing, for the `Output`
/// filter section.
pub struct Outputs {
    /// The listed files with an export; `None` without an export folder or
    /// when one could not be read whole.
    pub found: Option<HashSet<String>>,
    /// The files the answer is about.
    pub listing: HashSet<String>,
    /// Why the export folder could not be read, for the pane's notes.
    pub failure: Option<String>,
}

/// `output::outputs_in` over the listing, as the Tauri `outputs_of`: at most
/// two `read_dir`s, so it runs with the listing on the open's background
/// task.
pub fn outputs(listed: &Listed) -> Outputs {
    let started = Instant::now();
    let (found, failure) = match output::outputs_in(Path::new(&listed.dir), &listed.files) {
        Ok(found) => (found.map(|f| f.into_iter().collect::<HashSet<_>>()), None),
        Err(e) => {
            log::warn!("output filter: {e}");
            (None, Some(format!("Output filter: {e}")))
        }
    };
    log::info!(
        "output: dir={} found={} in {}ms",
        listed.dir,
        found
            .as_ref()
            .map_or_else(|| "none".to_string(), |f| f.len().to_string()),
        started.elapsed().as_millis()
    );
    Outputs {
        found,
        listing: listed.files.iter().cloned().collect(),
        failure,
    }
}

/// What a folder open prepared: the files pass 1 extracts and the sidecars
/// the open could not read.
pub struct Prepared {
    pub todo: Vec<FileStat>,
    pub sidecar_errors: Vec<SidecarError>,
}

/// The files in natural file-name order, the order the pager shows them in.
pub fn natural_order(files: &[String]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    files.sort_by(|a, b| natord::compare(&file_name(a), &file_name(b)));
    files
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// The status line's scan: pass 1's or pass 2's `(done, total)` while it
/// runs.
#[derive(Default)]
pub struct Status {
    progress: Option<(usize, usize)>,
    analyzing: Option<(usize, usize)>,
}

impl Status {
    /// A scan started with `total` files for pass 1.
    pub fn started(&mut self, total: usize) {
        self.progress = Some((0, total));
        self.analyzing = None;
    }

    pub fn extracting(&mut self, done: usize, total: usize) {
        self.progress = Some((done, total));
    }

    /// Pass 1 ended; pass 2 has not yet counted its files.
    pub fn extracted(&mut self) {
        self.progress = None;
        self.analyzing = Some((0, 0));
    }

    pub fn analyzing(&mut self, done: usize, total: usize) {
        self.analyzing = Some((done, total));
    }

    /// Pass 2 ended, or the scan was stopped: no scan runs.
    pub fn clear(&mut self) {
        self.progress = None;
        self.analyzing = None;
    }

    /// Pass 1 or pass 2 is running.
    pub fn scanning(&self) -> bool {
        self.progress.is_some() || self.analyzing.is_some()
    }

    /// What the status line says of the running pass.
    pub fn suffix(&self) -> Option<String> {
        if let Some((done, total)) = self.progress {
            Some(format!("  scanning {done}/{total}"))
        } else {
            self.analyzing
                .map(|(done, total)| format!("  analyzing {done}/{total}"))
        }
    }
}

/// The view's scan, shared with the app's quit handler, which can outlive the
/// view.
#[derive(Clone, Default)]
pub struct Running(Arc<Mutex<Option<Scan>>>);

impl Running {
    fn held(&self) -> std::sync::MutexGuard<'_, Option<Scan>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn put(&self, scan: Scan) {
        *self.held() = Some(scan);
    }

    pub fn take(&self) -> Option<Scan> {
        self.held().take()
    }

    /// The held scan is `id`.
    pub fn is(&self, id: u64) -> bool {
        self.held().as_ref().is_some_and(|s| s.id == id)
    }

    /// The files the held scan takes first.
    pub fn focus(&self, paths: Vec<String>) {
        if let Some(scan) = self.held().as_ref() {
            scan.focus(paths);
        }
    }

    /// `Scan::stop_within` on the held scan; true with none held.
    pub fn stop_within(&self, timeout: Duration) -> bool {
        self.take().is_none_or(|scan| scan.stop_within(timeout))
    }
}

/// A running scan over one folder: pass 1, then pass 2 on the same thread.
pub struct Scan {
    pub id: u64,
    focus: ScanFocus,
    cancel: Arc<AtomicBool>,
    thread: JoinHandle<()>,
    /// Disconnected once the thread ends, also on a panic: `JoinHandle` has no
    /// timed join.
    ended: Receiver<()>,
}

impl Scan {
    /// Stat the whole listing and reconcile the index with it, which drops the
    /// rows of files not listed, then read back the sidecars changed since the
    /// index last saw them, as the Tauri app's `scan_folder` orders the two,
    /// handing the rows still dirty to `writer`. A view-only folder has no
    /// sidecar pass.
    pub fn prepare(
        index: &Arc<Mutex<Index>>,
        writer: Option<&Writer>,
        listed: &Listed,
        format: SidecarFormat,
        names: &LabelNames,
    ) -> Result<Prepared, String> {
        let started = Instant::now();
        let files: Vec<FileStat> = listed
            .files
            .iter()
            .filter_map(|p| index::stat(Path::new(p)).ok())
            .collect();
        let (todo, removed) = index::lock(index).reconcile(&listed.dir, &files)?;
        log::info!(
            "scan reconcile: dir={} todo={} removed={removed} in {}ms",
            listed.dir,
            todo.len(),
            started.elapsed().as_millis()
        );
        let sidecar_errors = if viewonly::is_view_only(&listed.files) {
            Vec::new()
        } else {
            reconcile_sidecars(index, writer, listed, format, names)
        };
        Ok(Prepared {
            todo,
            sidecar_errors,
        })
    }

    /// Run `index::run_scan` and then `index::run_faces_scan` on a thread of
    /// their own: they block for as long as the folder takes, which would hold
    /// one of GPUI's background threads.
    pub fn start(
        id: u64,
        index: Arc<Mutex<Index>>,
        dir: String,
        todo: Vec<FileStat>,
        focus: ScanFocus,
        events: UnboundedSender<Event>,
    ) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let (alive, ended) = mpsc::channel::<()>();
        let thread = {
            let (focus, cancel) = (focus.clone(), cancel.clone());
            thread::spawn(move || {
                let _alive = alive;
                let summary = index::run_scan(
                    &index,
                    &dir,
                    &todo,
                    scan_threads(),
                    &focus,
                    &cancel,
                    index::PROGRESS_INTERVAL,
                    |done, total, ready| {
                        let _ = events.unbounded_send(Event::Progress {
                            scan: id,
                            done,
                            total,
                            ready,
                        });
                    },
                );
                let _ = events.unbounded_send(Event::Done { scan: id, summary });
                let summary =
                    run_faces_pass(&index, &dir, &focus, &cancel, |done, total, ready| {
                        let _ = events.unbounded_send(Event::FacesProgress {
                            scan: id,
                            done,
                            total,
                            ready,
                        });
                    });
                let _ = events.unbounded_send(Event::FacesDone { scan: id, summary });
            })
        };
        Self {
            id,
            focus,
            cancel,
            thread,
            ended,
        }
    }

    /// The files the scan takes first.
    pub fn focus(&self, paths: Vec<String>) {
        self.focus.set(paths);
    }

    /// Cancel and wait for the thread to end.
    pub fn stop(self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.thread.join();
    }

    /// Cancel and wait at most `timeout` for the thread to end: false when it
    /// did not, the thread left to die with the process.
    pub fn stop_within(self, timeout: Duration) -> bool {
        self.cancel.store(true, Ordering::Relaxed);
        match self.ended.recv_timeout(timeout) {
            Err(RecvTimeoutError::Timeout) => {
                log::warn!(
                    "scan {} did not stop within {}ms",
                    self.id,
                    timeout.as_millis()
                );
                false
            }
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                let _ = self.thread.join();
                true
            }
        }
    }
}

/// `sidecar::reconcile_of` over the listing, its dirty rows queued with no
/// debounce, as the Tauri `scan_folder` does: a judgment an earlier session
/// stored but did not write (a crash inside the debounce) is written now. Each
/// row carries its own `label_known`, so a row made before its label was
/// known does not clear the sidecar's. A failure is logged and leaves the open
/// going, as in the Tauri app.
fn reconcile_sidecars(
    index: &Arc<Mutex<Index>>,
    writer: Option<&Writer>,
    listed: &Listed,
    format: SidecarFormat,
    names: &LabelNames,
) -> Vec<SidecarError> {
    let started = Instant::now();
    let dir = &listed.dir;
    let sidecars = stat_sidecars(&listed.sidecars);
    let (dirty, errors, changed) =
        match sidecar::reconcile_of(dir, &listed.files, &sidecars, index, format, names) {
            Ok((dirty, errors, changed)) => (dirty, errors, changed),
            Err(e) => {
                log::error!("failed to reconcile the sidecars of {dir}: {e}");
                (Vec::new(), Vec::new(), 0)
            }
        };
    log::info!(
        "scan sidecars: dir={dir} dirty={} changed={changed} in {}ms",
        dirty.len(),
        started.elapsed().as_millis()
    );
    if let Some(writer) = writer {
        for (path, rating, flag, label, label_known) in dirty {
            if let Err(e) = writer.set_now(
                PathBuf::from(path),
                rating,
                flag,
                label,
                label_known,
                format,
                names.clone(),
            ) {
                log::error!("failed to queue a pending sidecar: {e}");
            }
        }
    }
    errors
}

/// Pass 2 over the files `faces_todo` gives, unless canceled, as the Tauri
/// app's `run_faces_pass`.
fn run_faces_pass(
    index: &Mutex<Index>,
    dir: &str,
    focus: &ScanFocus,
    cancel: &AtomicBool,
    progress: impl Fn(usize, usize, Vec<FaceReady>) + Send + Sync,
) -> ScanSummary {
    let none = ScanSummary {
        total: 0,
        errors: 0,
    };
    if cancel.load(Ordering::Relaxed) {
        return none;
    }
    let todo = match index::lock(index).faces_todo(dir) {
        Ok(todo) => todo,
        Err(e) => {
            log::error!("failed to list the faces pass of {dir}: {e}");
            return none;
        }
    };
    let first = AtomicBool::new(true);
    index::run_faces_scan(
        index,
        dir,
        &todo,
        faces_threads(),
        focus,
        cancel,
        index::PROGRESS_INTERVAL,
        |done, total, ready| {
            if !ready.is_empty() && first.swap(false, Ordering::Relaxed) {
                let names: Vec<String> = ready
                    .iter()
                    .map(|r| file_name(Path::new(&r.path)))
                    .collect();
                log::info!("scan faces first: {}", names.join(" "));
            }
            progress(done, total, ready);
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{
        Running, STOP_TIMEOUT, Scan, Status, faces_threads_for, list, natural_order, threads_for,
    };
    use riffle_backend::{
        index::Index,
        sidecar::{DRAIN_TIMEOUT, SidecarFormat, Writer, label_names_setting},
    };
    use riffle_core::{Flag, scan::ScanFocus};
    use std::{
        fs,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    #[test]
    fn a_stop_leaves_no_scan_on_the_status_line() {
        let mut status = Status::default();
        assert!(!status.scanning());
        assert_eq!(status.suffix(), None);
        status.started(10);
        status.extracting(3, 10);
        assert_eq!(status.suffix().as_deref(), Some("  scanning 3/10"));
        // A re-listing stops the scan whatever comes of its prepare.
        status.clear();
        assert!(!status.scanning());
        assert_eq!(status.suffix(), None);
        status.started(10);
        status.extracted();
        assert!(status.scanning());
        status.analyzing(4, 9);
        assert_eq!(status.suffix().as_deref(), Some("  analyzing 4/9"));
        status.clear();
        assert!(!status.scanning());
        assert_eq!(status.suffix(), None);
    }

    #[test]
    fn a_bounded_stop_reports_an_ended_scan_stopped() {
        let temp = tempfile::tempdir().unwrap();
        let index = Arc::new(Mutex::new(
            Index::open(&temp.path().join("index.sqlite")).unwrap(),
        ));
        let (events, _received) = futures::channel::mpsc::unbounded();
        let running = Running::default();
        assert!(running.stop_within(STOP_TIMEOUT));
        running.put(Scan::start(
            7,
            index,
            temp.path().to_string_lossy().into_owned(),
            Vec::new(),
            ScanFocus::default(),
            events,
        ));
        assert!(running.is(7));
        assert!(running.stop_within(STOP_TIMEOUT));
        assert!(!running.is(7));
        assert!(running.take().is_none());
    }

    #[test]
    fn an_open_writes_the_judgment_a_crash_left_unwritten() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("photos");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("DSC1.ARW"), b"").unwrap();
        let index = Arc::new(Mutex::new(
            Index::open(&temp.path().join("index.sqlite")).unwrap(),
        ));
        let names = label_names_setting(None);
        let listed = list(&folder, SidecarFormat::Xmp).unwrap();
        let arw = &listed.files[0];
        // The row a key stored before the process died inside the debounce.
        index
            .lock()
            .unwrap()
            .set_rating(&listed.dir, arw, Some(4), Flag::None, None, false)
            .unwrap();
        let writer = Writer::spawn(index.clone(), |path, message| {
            panic!("sidecar write failed: {} {message}", path.display())
        });
        let prepared =
            Scan::prepare(&index, Some(&writer), &listed, SidecarFormat::Xmp, &names).unwrap();
        assert!(prepared.sidecar_errors.is_empty());
        assert!(writer.flush(DRAIN_TIMEOUT));
        let xmp = fs::read_to_string(folder.join("DSC1.xmp")).unwrap();
        assert!(xmp.contains(r#"xmp:Rating="4""#), "{xmp}");
        assert!(
            index
                .lock()
                .unwrap()
                .dirty_rows(&listed.dir)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn orders_by_natural_file_name() {
        let files = ["D:/a/DSC10.ARW", "D:/a/DSC9.ARW", "D:/a/DSC100.ARW"].map(String::from);
        assert_eq!(
            natural_order(&files),
            ["D:/a/DSC9.ARW", "D:/a/DSC10.ARW", "D:/a/DSC100.ARW"].map(PathBuf::from)
        );
    }

    #[test]
    fn leaves_two_cores_and_keeps_one_thread() {
        assert_eq!(threads_for(24), 22);
        assert_eq!(threads_for(3), 1);
        assert_eq!(threads_for(2), 1);
        assert_eq!(threads_for(1), 1);
    }

    #[test]
    fn pass_two_takes_an_eighth_of_the_cores_and_keeps_one_thread() {
        assert_eq!(faces_threads_for(24), 3);
        assert_eq!(faces_threads_for(16), 2);
        assert_eq!(faces_threads_for(7), 1);
        assert_eq!(faces_threads_for(1), 1);
    }
}
