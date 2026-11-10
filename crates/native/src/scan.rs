use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use futures::channel::mpsc::UnboundedSender;
use riffle_backend::{
    index::{self, FaceReady, FileStat, Index, ScanSummary},
    listing::{canonicalize, read_listing},
};
use riffle_core::scan::ScanFocus;

/// Scan workers inside a file read at once, as the Tauri app's `READ_SLOTS`.
pub const READ_SLOTS: usize = 2;

/// Two fewer than the cores, at least one, as the Tauri app's `scan_threads`:
/// two cores stay with the paging path.
pub fn scan_threads() -> usize {
    threads_for(thread::available_parallelism().map_or(4, |n| n.get()))
}

fn threads_for(cores: usize) -> usize {
    cores.saturating_sub(2).max(1)
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
        #[expect(
            dead_code,
            reason = "the focus mark reads it (plan gpui-focus-marks Step 2)"
        )]
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
}

/// One folder's listing: its canonical path, the key of every index row, and
/// its RAW files (or JPEGs when it holds no RAW) under that path.
pub struct Listed {
    pub dir: String,
    pub files: Vec<String>,
}

pub fn list(folder: &Path) -> Result<Listed, String> {
    let dir = canonicalize(&folder.to_string_lossy());
    let started = Instant::now();
    let files = read_listing(Path::new(&dir), None)?.files;
    log::info!(
        "scan list: dir={dir} files={} in {}ms",
        files.len(),
        started.elapsed().as_millis()
    );
    Ok(Listed { dir, files })
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

/// A running scan over one folder: pass 1, then pass 2 on the same thread.
pub struct Scan {
    pub id: u64,
    focus: ScanFocus,
    cancel: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl Scan {
    /// Stat the whole listing and reconcile the index with it, which drops the
    /// rows of files not listed. The files to extract.
    pub fn prepare(index: &Mutex<Index>, listed: &Listed) -> Result<Vec<FileStat>, String> {
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
        Ok(todo)
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
        let thread = {
            let (focus, cancel) = (focus.clone(), cancel.clone());
            thread::spawn(move || {
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
        scan_threads(),
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
    use super::{natural_order, threads_for};
    use std::path::PathBuf;

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
}
