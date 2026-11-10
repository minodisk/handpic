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
    index::{self, FileStat, Index, ScanSummary},
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

/// A running pass 1 over one folder.
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

    /// Run `index::run_scan` on a thread of its own: it blocks for as long as
    /// the folder takes, which would hold one of GPUI's background threads.
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
