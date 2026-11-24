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
use handpic_core::{
    scan::{Priority, ScanFocus},
    xmp::LabelNames,
};
use handpic_engine::{
    diagnostics::invariant,
    index::{self, AllFacesReady, FaceReady, FileStat, Index, ScanSummary},
    listing::{canonicalize, read_listing, stat_sidecars},
    output,
    sidecar::{self, SidecarError, SidecarFormat, Writer},
};

use crate::viewonly;

/// Scan workers inside a file read at once, as the Tauri app's `READ_SLOTS`.
pub const READ_SLOTS: usize = 2;

/// How long a quit waits for the running scan to stop. Workers finish the
/// stage of the file they are on, which a slow or network volume can hold
/// longer; past this the process exits with the scan still running.
pub const STOP_TIMEOUT: Duration = Duration::from_secs(2);

/// The OS priority of each pass's workers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Priorities {
    pub extract: Priority,
    pub faces: Priority,
    pub allfaces: Priority,
}

impl Priorities {
    /// The app's: pass 1 below normal, pass 2 lower still, so the viewer's
    /// preview decode and the UI win the cores they contend for. Pass 3 runs
    /// at pass 2's level, the lowest `Priority` names, after it by order.
    pub const APP: Self = Self {
        extract: Priority::BelowNormal,
        faces: Priority::Lowest,
        allfaces: Priority::Lowest,
    };
}

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

/// What the scan and the folder watchers tell the view, a scan's messages
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
    /// not list its files. The scan is over unless pass 3 has files.
    FacesDone {
        scan: u64,
        #[expect(dead_code, reason = "the engine's `scan faces:` line reports it")]
        summary: ScanSummary,
        /// The files pass 3 runs on next; with none it does not run.
        checking: usize,
    },
    /// Pass 3's progress.
    AllFacesProgress {
        scan: u64,
        done: usize,
        total: usize,
        /// The files written since the previous message.
        ready: Vec<AllFacesReady>,
    },
    /// Pass 3 ended, also when it was canceled: the scan is over.
    AllFacesDone {
        scan: u64,
        #[expect(dead_code, reason = "the engine's `scan allfaces:` line reports it")]
        summary: ScanSummary,
    },
    /// The folder watcher saw the folder change; the canonical path.
    Changed(String),
    /// The folder tree's watcher saw an expanded folder change; the tree's
    /// spelling of its path.
    TreeChanged(String),
    /// The sidecar writer failed to write `path`'s sidecar.
    SidecarError {
        path: String,
        message: String,
    },
    /// A tool call of the MCP companion for the view to answer.
    Mcp(handpic_engine::mcp::Request),
    /// The MCP companion's server after a switch.
    McpState(handpic_engine::mcp::McpState),
    /// How far the move to the Trash or the delete in flight got.
    TrashProgress {
        mode: crate::trash::Mode,
        done: usize,
        total: usize,
    },
    /// How far the undo of a trash run in flight got.
    UndoProgress {
        done: usize,
        total: usize,
    },
    /// A message of a folder operation's run.
    FolderOp(crate::folderops::Message),
    /// A performance overlay sample from the sampler's thread.
    Perf(handpic_engine::perf::Sample),
    /// An answer of the update check's or install's thread.
    Update(crate::update::Message),
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

/// The status line's scan: pass 1's, pass 2's or pass 3's `(done, total)`
/// while it runs.
#[derive(Default)]
pub struct Status {
    progress: Option<(usize, usize)>,
    analyzing: Option<(usize, usize)>,
    checking: Option<(usize, usize)>,
}

impl Status {
    /// A scan started with `total` files for pass 1.
    pub fn started(&mut self, total: usize) {
        self.progress = Some((0, total));
        self.analyzing = None;
        self.checking = None;
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

    /// Pass 2 ended and pass 3 runs on `total` files.
    pub fn analyzed(&mut self, total: usize) {
        self.analyzing = None;
        self.checking = Some((0, total));
    }

    pub fn checking(&mut self, done: usize, total: usize) {
        self.checking = Some((done, total));
    }

    /// The last pass ended, or the scan was stopped: no scan runs.
    pub fn clear(&mut self) {
        self.progress = None;
        self.analyzing = None;
        self.checking = None;
    }

    /// Pass 1, pass 2 or pass 3 is running.
    pub fn scanning(&self) -> bool {
        self.pass().is_some()
    }

    /// The running pass's `(done, total)`, pass 1's before pass 2's before
    /// pass 3's.
    pub fn running(&self) -> Option<(usize, usize)> {
        self.pass().map(|(_, done, total)| (done, total))
    }

    /// The running pass as the status line names it, with its
    /// `(done, total)`.
    pub fn pass(&self) -> Option<(&'static str, usize, usize)> {
        let named = |name, pass: Option<(usize, usize)>| pass.map(|(d, t)| (name, d, t));
        named("scanning", self.progress)
            .or(named("analyzing", self.analyzing))
            .or(named("checking faces", self.checking))
    }

    /// What the status line says of the running pass.
    pub fn suffix(&self) -> Option<String> {
        self.pass()
            .map(|(name, done, total)| format!("  {name} {done}/{total}"))
    }
}

/// The launch's eviction, which every listing waits for before it reconciles:
/// the first waiter joins the thread under the lock, so a later one blocks
/// until the eviction is over, whether or not an earlier listing's task got
/// that far.
#[derive(Clone, Default)]
pub struct Eviction(Arc<Mutex<Option<JoinHandle<()>>>>);

impl Eviction {
    pub fn new(thread: Option<JoinHandle<()>>) -> Self {
        Self(Arc::new(Mutex::new(thread)))
    }

    /// Block until the eviction has ended.
    pub fn wait(&self) {
        let mut thread = index::lock(&self.0);
        if let Some(thread) = thread.take() {
            let _ = thread.join();
        }
    }

    #[cfg(test)]
    pub fn is_finished(&self) -> bool {
        index::lock(&self.0)
            .as_ref()
            .is_none_or(JoinHandle::is_finished)
    }
}

/// The view's scan, shared with the app's quit handler, which can outlive the
/// view, and the scans a re-listing took from it that are still stopping.
#[derive(Clone, Default)]
pub struct Running(Arc<Mutex<Held>>);

#[derive(Default)]
struct Held {
    scan: Option<Scan>,
    retired: Vec<Retired>,
}

/// What the holder keeps of a scan a re-listing is stopping: enough for the
/// quit to cancel it and wait for its thread to end.
struct Retired {
    id: u64,
    cancel: Arc<AtomicBool>,
    ended: Receiver<()>,
}

/// What a quit's bounded stop came to: the held scan stopped (true with none
/// held), and the retired one, `None` with none still stopping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stopped {
    pub held: bool,
    pub retired: Option<bool>,
}

impl Running {
    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn put(&self, scan: Scan) {
        self.held().scan = Some(scan);
    }

    /// Take the held scan for a re-listing to stop, keeping its cancel flag
    /// and end signal here until `Stopping::stop` returns.
    pub fn retire(&self) -> Option<Stopping> {
        let mut held = self.held();
        let scan = held.scan.take()?;
        held.retired.push(Retired {
            id: scan.id,
            cancel: scan.cancel.clone(),
            ended: scan.ended,
        });
        Some(Stopping {
            id: scan.id,
            cancel: scan.cancel,
            thread: scan.thread,
            running: self.clone(),
        })
    }

    /// The held scan is `id`.
    pub fn is(&self, id: u64) -> bool {
        self.held().scan.as_ref().is_some_and(|s| s.id == id)
    }

    /// The files the held scan takes first.
    pub fn focus(&self, paths: Vec<String>) {
        if let Some(scan) = self.held().scan.as_ref() {
            scan.focus(paths);
        }
    }

    /// `Scan::cancel` on the held scan, which stays held: it still ends
    /// through `Done` and `FacesDone`, then `AllFacesDone` once pass 3 runs.
    pub fn cancel(&self) {
        if let Some(scan) = self.held().scan.as_ref() {
            scan.cancel();
        }
    }

    /// Cancel the held scan and the retired ones and wait for them, at most
    /// `timeout` in all.
    pub fn stop_within(&self, timeout: Duration) -> Stopped {
        let deadline = Instant::now() + timeout;
        let (scan, retired) = {
            let mut held = self.held();
            (held.scan.take(), std::mem::take(&mut held.retired))
        };
        for retired in &retired {
            retired.cancel.store(true, Ordering::Relaxed);
        }
        let held = scan.is_none_or(|scan| scan.stop_within(timeout));
        let retired = (!retired.is_empty()).then(|| {
            retired.iter().fold(true, |stopped, retired| {
                let left = deadline.saturating_duration_since(Instant::now());
                match retired.ended.recv_timeout(left) {
                    Err(RecvTimeoutError::Timeout) => {
                        log::warn!(
                            "retired scan {} did not stop within {}ms",
                            retired.id,
                            timeout.as_millis()
                        );
                        false
                    }
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => stopped,
                }
            })
        });
        Stopped { held, retired }
    }
}

/// A scan a re-listing took from the holder, stopped off the UI thread.
pub struct Stopping {
    id: u64,
    cancel: Arc<AtomicBool>,
    thread: JoinHandle<()>,
    running: Running,
}

impl Stopping {
    /// Cancel and wait for the thread to end, then drop it from the holder.
    pub fn stop(self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.thread.join();
        self.running.held().retired.retain(|r| r.id != self.id);
    }
}

/// A running scan over one folder: pass 1, then pass 2, then pass 3 on the
/// same thread.
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

    /// Run `index::run_scan`, `index::run_faces_scan` and then, on the files
    /// at the gate, `index::run_allfaces_scan` on a thread of their own: they
    /// block for as long as the folder takes, which would hold one of GPUI's
    /// background threads. `group_min_faces` is the group threshold in force
    /// when the scan starts; a later change waits for the next listing.
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        id: u64,
        index: Arc<Mutex<Index>>,
        dir: String,
        todo: Vec<FileStat>,
        focus: ScanFocus,
        priorities: Priorities,
        group_min_faces: u32,
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
                    priorities.extract,
                    &focus,
                    &cancel,
                    index::PROGRESS_INTERVAL,
                    |done, total, ready| {
                        invariant!(
                            done <= total,
                            "scan progress within total",
                            "dir={dir} scan_id={id} done={done} total={total}"
                        );
                        let _ = events.unbounded_send(Event::Progress {
                            scan: id,
                            done,
                            total,
                            ready,
                        });
                    },
                );
                let _ = events.unbounded_send(Event::Done { scan: id, summary });
                let summary = run_faces_pass(
                    &index,
                    &dir,
                    priorities.faces,
                    &focus,
                    &cancel,
                    |done, total, ready| {
                        invariant!(
                            done <= total,
                            "scan progress within total",
                            "dir={dir} scan_id={id} done={done} total={total}"
                        );
                        let _ = events.unbounded_send(Event::FacesProgress {
                            scan: id,
                            done,
                            total,
                            ready,
                        });
                    },
                );
                let todo = allfaces_todo(&index, &dir, group_min_faces, &cancel);
                let _ = events.unbounded_send(Event::FacesDone {
                    scan: id,
                    summary,
                    checking: todo.len(),
                });
                if todo.is_empty() {
                    return;
                }
                let summary = index::run_allfaces_scan(
                    &index,
                    &dir,
                    &todo,
                    faces_threads(),
                    priorities.allfaces,
                    &focus,
                    &cancel,
                    index::PROGRESS_INTERVAL,
                    |done, total, ready| {
                        invariant!(
                            done <= total,
                            "scan progress within total",
                            "dir={dir} scan_id={id} done={done} total={total}"
                        );
                        let _ = events.unbounded_send(Event::AllFacesProgress {
                            scan: id,
                            done,
                            total,
                            ready,
                        });
                    },
                );
                let _ = events.unbounded_send(Event::AllFacesDone { scan: id, summary });
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

    /// Cancel without waiting: the workers finish the stage they are on and
    /// the thread still sends `Done` and `FacesDone`, then `AllFacesDone`
    /// once pass 3 runs.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
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
    priority: Priority,
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
        priority,
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

/// The files pass 3 runs on, `Index::allfaces_todo` at `min_faces`; none
/// once canceled or when they could not be listed.
fn allfaces_todo(
    index: &Mutex<Index>,
    dir: &str,
    min_faces: u32,
    cancel: &AtomicBool,
) -> Vec<String> {
    if cancel.load(Ordering::Relaxed) {
        return Vec::new();
    }
    index::lock(index)
        .allfaces_todo(dir, min_faces)
        .unwrap_or_else(|e| {
            log::error!("failed to list the all-faces pass of {dir}: {e}");
            Vec::new()
        })
}

#[cfg(test)]
mod tests {
    use super::{
        Event, Eviction, Priorities, Running, STOP_TIMEOUT, Scan, Status, Stopped,
        faces_threads_for, list, natural_order, threads_for,
    };
    use crate::group;
    use handpic_core::{
        Flag,
        arw::Shot,
        scan::{Entry, ScanFocus},
    };
    use handpic_engine::{
        index::{self, FacesRow, Index},
        sidecar::{DRAIN_TIMEOUT, SidecarFormat, Writer, label_names_setting},
    };
    use std::{
        fs,
        path::PathBuf,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
            mpsc::{self, RecvTimeoutError},
        },
        thread,
        time::Duration,
    };

    #[test]
    fn every_waiter_waits_for_the_eviction() {
        let (release, held) = mpsc::channel::<()>();
        let done = Arc::new(AtomicBool::new(false));
        let eviction = Eviction::new(Some(thread::spawn({
            let done = done.clone();
            move || {
                let _ = held.recv();
                done.store(true, Ordering::SeqCst);
            }
        })));
        let (ended, waited) = mpsc::channel();
        for _ in 0..2 {
            let (eviction, done, ended) = (eviction.clone(), done.clone(), ended.clone());
            thread::spawn(move || {
                eviction.wait();
                ended.send(done.load(Ordering::SeqCst)).unwrap();
            });
        }
        assert_eq!(
            waited.recv_timeout(Duration::from_millis(100)),
            Err(RecvTimeoutError::Timeout)
        );
        assert!(!done.load(Ordering::SeqCst));
        release.send(()).unwrap();
        assert_eq!(waited.recv_timeout(STOP_TIMEOUT), Ok(true));
        assert_eq!(waited.recv_timeout(STOP_TIMEOUT), Ok(true));
        assert!(eviction.is_finished());
        eviction.wait();
    }

    #[test]
    fn no_eviction_is_no_wait() {
        let eviction = Eviction::default();
        assert!(eviction.is_finished());
        eviction.wait();
    }

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
        status.started(10);
        status.extracted();
        status.analyzed(3);
        assert!(status.scanning());
        status.checking(1, 3);
        assert_eq!(status.suffix().as_deref(), Some("  checking faces 1/3"));
        status.clear();
        assert!(!status.scanning());
        assert_eq!(status.suffix(), None);
    }

    #[test]
    fn a_new_scan_drops_the_last_scans_third_pass() {
        let mut status = Status::default();
        status.analyzed(3);
        status.started(10);
        assert_eq!(status.pass(), Some(("scanning", 0, 10)));
        status.extracted();
        status.analyzing(2, 2);
        assert_eq!(status.pass(), Some(("analyzing", 2, 2)));
    }

    #[test]
    fn the_running_pass_gives_the_bar_its_counts() {
        let mut status = Status::default();
        assert_eq!(status.running(), None);
        status.started(10);
        assert_eq!(status.running(), Some((0, 10)));
        status.extracting(3, 10);
        assert_eq!(status.running(), Some((3, 10)));
        status.extracted();
        assert_eq!(status.running(), Some((0, 0)));
        status.analyzing(4, 9);
        assert_eq!(status.running(), Some((4, 9)));
        status.analyzed(3);
        assert_eq!(status.running(), Some((0, 3)));
        status.checking(2, 3);
        assert_eq!(status.running(), Some((2, 3)));
        status.clear();
        assert_eq!(status.running(), None);
    }

    #[test]
    fn the_running_pass_is_named_as_the_status_line_names_it() {
        let mut status = Status::default();
        assert_eq!(status.pass(), None);
        status.started(10);
        status.extracting(3, 10);
        assert_eq!(status.pass(), Some(("scanning", 3, 10)));
        status.extracted();
        status.analyzing(4, 9);
        assert_eq!(status.pass(), Some(("analyzing", 4, 9)));
        status.analyzed(3);
        status.checking(1, 3);
        assert_eq!(status.pass(), Some(("checking faces", 1, 3)));
        status.clear();
        assert_eq!(status.pass(), None);
    }

    /// A row through pass 2 with `group` as its gate count.
    fn gated(index: &Mutex<Index>, dir: &str, name: &str, group: u32) -> String {
        let path = PathBuf::from(dir).join(name);
        fs::write(&path, b"not a raw file").unwrap();
        let stat = index::stat(&path).unwrap();
        let entry = Entry {
            orientation: 1,
            shot: Shot::default(),
            thumbnail: vec![0xff, 0xd8, 0xff, 0xd9],
        };
        let mut index = index.lock().unwrap();
        index.write_batch(dir, &[(stat, Ok(entry))]).unwrap();
        let path = path.to_string_lossy().into_owned();
        index
            .write_faces(&[FacesRow {
                path: path.clone(),
                sharpness: None,
                eyes_ear: None,
                pose: None,
                eye_offset: None,
                edge_gap: None,
                full_eye_sharp: None,
                faces_group: Some(group),
            }])
            .unwrap();
        path
    }

    /// What a scan of `dir` with nothing for pass 1 sends at the group
    /// threshold `min_faces`, in order, each message by its kind and, for
    /// pass 2's and pass 3's rows, the files it names.
    fn scan_events_at(
        index: Arc<Mutex<Index>>,
        dir: &str,
        min_faces: u32,
    ) -> Vec<(&'static str, Vec<String>)> {
        let (events, received) = futures::channel::mpsc::unbounded();
        let scan = Scan::start(
            9,
            index,
            dir.to_owned(),
            Vec::new(),
            ScanFocus::default(),
            Priorities::APP,
            min_faces,
            events,
        );
        let sent = futures::executor::block_on_stream(received)
            .filter_map(|event| match event {
                Event::Done { .. } => Some(("done", Vec::new())),
                Event::FacesProgress { ready, .. } if !ready.is_empty() => {
                    Some(("faces-ready", ready.into_iter().map(|r| r.path).collect()))
                }
                Event::FacesDone { checking, .. } => {
                    Some(("faces-done", vec![checking.to_string()]))
                }
                Event::AllFacesProgress { ready, .. } if !ready.is_empty() => {
                    assert!(ready.iter().all(|r| r.faces_judged.is_none()));
                    Some((
                        "allfaces-ready",
                        ready.into_iter().map(|r| r.path).collect(),
                    ))
                }
                Event::AllFacesDone { .. } => Some(("allfaces-done", Vec::new())),
                _ => None,
            })
            .collect();
        assert!(scan.stop_within(STOP_TIMEOUT));
        sent
    }

    fn scan_events(index: Arc<Mutex<Index>>, dir: &str) -> Vec<(&'static str, Vec<String>)> {
        scan_events_at(index, dir, group::DEFAULT)
    }

    #[test]
    fn a_lowered_threshold_runs_pass_3_on_the_rows_that_now_meet_it_alone() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().to_string_lossy().into_owned();
        let index = Arc::new(Mutex::new(
            Index::open(&temp.path().join("index.sqlite")).unwrap(),
        ));
        let under = gated(&index, &dir, "A.ARW", 2);
        let three = gated(&index, &dir, "B.ARW", 3);
        let four = gated(&index, &dir, "C.ARW", 4);
        let at = gated(&index, &dir, "D.ARW", 5);
        assert_eq!(
            scan_events_at(index.clone(), &dir, 5),
            [
                ("done", Vec::new()),
                ("faces-done", vec!["1".to_owned()]),
                ("allfaces-ready", vec![at]),
                ("allfaces-done", Vec::new()),
            ]
        );
        let sent = scan_events_at(index.clone(), &dir, 3);
        assert!(
            sent.iter().all(|(kind, _)| *kind != "faces-ready"),
            "pass 2 runs on nothing: {sent:?}"
        );
        assert_eq!(sent[1], ("faces-done", vec!["2".to_owned()]));
        let mut checked: Vec<String> = sent
            .iter()
            .filter(|(kind, _)| *kind == "allfaces-ready")
            .flat_map(|(_, paths)| paths.clone())
            .collect();
        checked.sort();
        assert_eq!(checked, [three, four]);
        assert_eq!(sent.last().unwrap().0, "allfaces-done");
        assert_eq!(
            index.lock().unwrap().allfaces_todo(&dir, 2).unwrap(),
            [under],
            "the row under the lowered threshold is left for a lower one"
        );
    }

    #[test]
    fn pass_3_runs_after_pass_2_on_the_rows_at_the_gate() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().to_string_lossy().into_owned();
        let index = Arc::new(Mutex::new(
            Index::open(&temp.path().join("index.sqlite")).unwrap(),
        ));
        gated(&index, &dir, "A.ARW", group::DEFAULT - 1);
        let at = gated(&index, &dir, "B.ARW", group::DEFAULT);
        assert_eq!(
            scan_events(index.clone(), &dir),
            [
                ("done", Vec::new()),
                ("faces-done", vec!["1".to_owned()]),
                ("allfaces-ready", vec![at]),
                ("allfaces-done", Vec::new()),
            ]
        );
        assert!(
            index
                .lock()
                .unwrap()
                .allfaces_todo(&dir, group::DEFAULT)
                .unwrap()
                .is_empty(),
            "the unreadable file is stored and not retried"
        );
    }

    #[test]
    fn a_folder_with_no_row_at_the_gate_runs_no_pass_3() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().to_string_lossy().into_owned();
        let index = Arc::new(Mutex::new(
            Index::open(&temp.path().join("index.sqlite")).unwrap(),
        ));
        gated(&index, &dir, "A.ARW", group::DEFAULT - 1);
        assert_eq!(
            scan_events(index, &dir),
            [("done", Vec::new()), ("faces-done", vec!["0".to_owned()])]
        );
    }

    #[test]
    fn a_bounded_stop_reports_an_ended_scan_stopped() {
        let temp = tempfile::tempdir().unwrap();
        let index = Arc::new(Mutex::new(
            Index::open(&temp.path().join("index.sqlite")).unwrap(),
        ));
        let (events, _received) = futures::channel::mpsc::unbounded();
        let running = Running::default();
        let none = Stopped {
            held: true,
            retired: None,
        };
        assert_eq!(running.stop_within(STOP_TIMEOUT), none);
        let scan = Scan::start(
            7,
            index,
            temp.path().to_string_lossy().into_owned(),
            Vec::new(),
            ScanFocus::default(),
            Priorities::APP,
            group::DEFAULT,
            events,
        );
        assert_eq!(
            scan.ended.recv_timeout(Duration::from_secs(60)),
            Err(RecvTimeoutError::Disconnected),
            "the empty scan ends"
        );
        running.put(scan);
        assert!(running.is(7));
        assert_eq!(running.stop_within(STOP_TIMEOUT), none);
        assert!(!running.is(7));
        assert!(running.held().scan.is_none());
    }

    #[test]
    fn a_retired_scan_is_waited_for_until_its_stop_returns() {
        let (release, released) = mpsc::channel::<()>();
        let (alive, ended) = mpsc::channel::<()>();
        let cancel = Arc::new(AtomicBool::new(false));
        let running = Running::default();
        running.put(Scan {
            id: 3,
            focus: ScanFocus::default(),
            cancel: cancel.clone(),
            thread: thread::spawn(move || {
                let _alive = alive;
                let _ = released.recv();
            }),
            ended,
        });
        let stopping = running.retire().unwrap();
        assert!(!running.is(3));
        assert_eq!(running.held().retired.len(), 1);
        let stopper = thread::spawn(move || stopping.stop());
        let (quit, stopped) = mpsc::channel();
        let waiter = {
            let running = running.clone();
            thread::spawn(move || {
                let _ = quit.send(running.stop_within(Duration::from_secs(600)));
            })
        };
        // The quit's stop has the retired scan before its thread may end.
        while !running.held().retired.is_empty() {
            thread::sleep(Duration::from_millis(1));
        }
        assert!(stopped.recv_timeout(Duration::from_millis(100)).is_err());
        release.send(()).unwrap();
        stopper.join().unwrap();
        waiter.join().unwrap();
        assert_eq!(
            stopped.recv().unwrap(),
            Stopped {
                held: true,
                retired: Some(true),
            }
        );
        assert!(cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn a_retired_scan_leaves_the_holder_once_its_stop_returns() {
        let temp = tempfile::tempdir().unwrap();
        let index = Arc::new(Mutex::new(
            Index::open(&temp.path().join("index.sqlite")).unwrap(),
        ));
        let (events, _received) = futures::channel::mpsc::unbounded();
        let running = Running::default();
        running.put(Scan::start(
            8,
            index,
            temp.path().to_string_lossy().into_owned(),
            Vec::new(),
            ScanFocus::default(),
            Priorities::APP,
            group::DEFAULT,
            events,
        ));
        let stopping = running.retire().unwrap();
        assert_eq!(running.held().retired.len(), 1);
        stopping.stop();
        assert!(running.held().retired.is_empty());
        assert!(running.retire().is_none());
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
