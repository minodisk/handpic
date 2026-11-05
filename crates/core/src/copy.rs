//! Copy files with one sequential reader feeding one writer, for importing a
//! memory card: the reader walks the jobs in order in large chunks, so a card
//! is read front to back without seeking between files, while the writer
//! overlaps the destination writes through a bounded channel of buffers.
//!
//! Each destination is written to `<name>`[`TEMP_SUFFIX`] in its folder and
//! renamed into place once complete, so a watcher or a scan never sees a
//! partial file, and carries the source's modified time. A destination that
//! already exists is never overwritten.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::time::{Duration, Instant, SystemTime};

/// Suffix of the temp file a copy, or the app's sidecar writer, writes before
/// renaming it into place. Not a RAW's extension, so no listing picks it up.
pub const TEMP_SUFFIX: &str = ".riffle-tmp";

/// One file to copy: `dst` is the final path, its folder already existing.
#[derive(Debug, Clone)]
pub struct Job {
    pub src: PathBuf,
    pub dst: PathBuf,
}

/// The size of each read and how many full buffers may wait for the writer.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub chunk: usize,
    pub buffers: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            chunk: 8 << 20,
            buffers: 4,
        }
    }
}

/// Where a run stands. `job` is the index of the job copied or just ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub job: usize,
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// What became of one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Copied, with the bytes written.
    Copied(u64),
    Failed(String),
    /// Not reached, or abandoned mid-file, because the run was canceled.
    NotRun,
}

/// Results of a [`copy_files`] run, `results` in the order of the jobs.
#[derive(Debug)]
pub struct Summary {
    pub results: Vec<Outcome>,
    /// Bytes of the files copied.
    pub bytes: u64,
    pub elapsed: Duration,
    pub canceled: bool,
}

enum Msg {
    Begin {
        job: usize,
        modified: Option<SystemTime>,
    },
    Chunk(Vec<u8>, usize),
    End,
    Fail(usize, String),
    Canceled,
}

/// [`copy_files_with`] the default [`Options`].
pub fn copy_files(jobs: &[Job], cancel: &AtomicBool, progress: impl FnMut(Progress)) -> Summary {
    copy_files_with(jobs, Options::default(), cancel, progress)
}

/// Copy `jobs` in order. `cancel` is checked before each file and between
/// chunks; once set, the file in flight is abandoned (its temp file removed)
/// and no further file is started. `progress` is called on the calling thread
/// after every chunk written and every job ended; a failed job counts its
/// source size as done, so the bytes reach the total.
pub fn copy_files_with(
    jobs: &[Job],
    options: Options,
    cancel: &AtomicBool,
    mut progress: impl FnMut(Progress),
) -> Summary {
    let start = Instant::now();
    let sizes: Vec<u64> = jobs
        .iter()
        .map(|j| fs::metadata(&j.src).map(|m| m.len()).unwrap_or(0))
        .collect();
    let bytes_total = sizes.iter().sum();
    let chunk = options.chunk.max(1);
    let (tx, rx) = mpsc::sync_channel(options.buffers.max(1));
    let (free_tx, free_rx) = mpsc::channel();
    let abandoned = AtomicUsize::new(0);

    let mut writer = Writer {
        jobs,
        results: vec![Outcome::NotRun; jobs.len()],
        bytes: 0,
        canceled: false,
        current: None,
        base: 0,
        files_done: 0,
        sizes: &sizes,
        bytes_total,
        abandoned: &abandoned,
    };
    std::thread::scope(|s| {
        s.spawn(|| read_all(jobs, chunk, cancel, &abandoned, tx, free_rx));
        writer.run(rx, free_tx, &mut progress);
    });

    Summary {
        results: writer.results,
        bytes: writer.bytes,
        elapsed: start.elapsed(),
        canceled: writer.canceled,
    }
}

fn read_all(
    jobs: &[Job],
    chunk: usize,
    cancel: &AtomicBool,
    abandoned: &AtomicUsize,
    tx: SyncSender<Msg>,
    free: Receiver<Vec<u8>>,
) {
    for (job, j) in jobs.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            let _ = tx.send(Msg::Canceled);
            return;
        }
        if j.dst.exists() {
            let _ = tx.send(Msg::Fail(job, format!("exists: {}", j.dst.display())));
            continue;
        }
        let mut file = match File::open(&j.src) {
            Ok(f) => f,
            Err(e) => {
                let _ = tx.send(Msg::Fail(job, format!("open: {}: {e}", j.src.display())));
                continue;
            }
        };
        let modified = file.metadata().and_then(|m| m.modified()).ok();
        if tx.send(Msg::Begin { job, modified }).is_err() {
            return;
        }
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(Msg::Canceled);
                return;
            }
            if abandoned.load(Ordering::Relaxed) == job + 1 {
                break;
            }
            let mut buf = free.try_recv().unwrap_or_default();
            buf.resize(chunk, 0);
            match fill(&mut file, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(Msg::Chunk(buf, n)).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Msg::Fail(job, format!("read: {}: {e}", j.src.display())));
                    break;
                }
            }
        }
        if tx.send(Msg::End).is_err() {
            return;
        }
    }
}

/// Read until `buf` is full or the file ends; the bytes read.
fn fill(file: &mut File, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match file.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

struct InFlight {
    job: usize,
    file: File,
    temp: PathBuf,
    modified: Option<SystemTime>,
    written: u64,
}

struct Writer<'a> {
    jobs: &'a [Job],
    results: Vec<Outcome>,
    bytes: u64,
    canceled: bool,
    current: Option<InFlight>,
    base: u64,
    files_done: usize,
    sizes: &'a [u64],
    bytes_total: u64,
    abandoned: &'a AtomicUsize,
}

impl Writer<'_> {
    fn run(
        &mut self,
        rx: Receiver<Msg>,
        free: Sender<Vec<u8>>,
        progress: &mut impl FnMut(Progress),
    ) {
        for msg in rx {
            match msg {
                Msg::Begin { job, modified } => {
                    let temp = temp_path(&self.jobs[job].dst);
                    match File::create(&temp) {
                        Ok(file) => {
                            self.current = Some(InFlight {
                                job,
                                file,
                                temp,
                                modified,
                                written: 0,
                            })
                        }
                        Err(e) => {
                            self.fail(job, format!("create: {}: {e}", temp.display()), progress)
                        }
                    }
                }
                Msg::Chunk(buf, n) => {
                    if let Some(f) = &mut self.current {
                        let job = f.job;
                        match f.file.write_all(&buf[..n]) {
                            Ok(()) => {
                                f.written += n as u64;
                                let written = f.written;
                                progress(self.progress(job, written));
                            }
                            Err(e) => {
                                let temp = f.temp.display().to_string();
                                self.fail(job, format!("write: {temp}: {e}"), progress)
                            }
                        }
                    }
                    let _ = free.send(buf);
                }
                Msg::End => {
                    if let Some(f) = self.current.take() {
                        let job = f.job;
                        match finish(f, &self.jobs[job].dst) {
                            Ok(written) => {
                                self.bytes += written;
                                self.results[job] = Outcome::Copied(written);
                                self.end(job, progress);
                            }
                            Err(e) => self.fail(job, e, progress),
                        }
                    }
                }
                Msg::Fail(job, e) => self.fail(job, e, progress),
                Msg::Canceled => {
                    if let Some(f) = self.current.take() {
                        drop(f.file);
                        let _ = fs::remove_file(&f.temp);
                    }
                    self.canceled = true;
                }
            }
        }
    }

    /// Record `job` as failed, dropping its temp file if it is in flight and
    /// telling the reader to stop reading it.
    fn fail(&mut self, job: usize, error: String, progress: &mut impl FnMut(Progress)) {
        if let Some(f) = self.current.take_if(|f| f.job == job) {
            drop(f.file);
            let _ = fs::remove_file(&f.temp);
        }
        self.abandoned.store(job + 1, Ordering::Relaxed);
        self.results[job] = Outcome::Failed(error);
        self.end(job, progress);
    }

    fn end(&mut self, job: usize, progress: &mut impl FnMut(Progress)) {
        self.base += self.sizes[job];
        self.files_done += 1;
        progress(self.progress(job, 0));
    }

    fn progress(&self, job: usize, written: u64) -> Progress {
        Progress {
            job,
            files_done: self.files_done,
            files_total: self.jobs.len(),
            bytes_done: (self.base + written).min(self.bytes_total),
            bytes_total: self.bytes_total,
        }
    }
}

/// Give the temp file the source's modified time, flush it, and rename it to
/// `dst` unless `dst` appeared meanwhile; the bytes written.
fn finish(f: InFlight, dst: &Path) -> Result<u64, String> {
    let result = (|| {
        if let Some(t) = f.modified {
            f.file
                .set_modified(t)
                .map_err(|e| format!("set modified: {}: {e}", f.temp.display()))?;
        }
        f.file
            .sync_all()
            .map_err(|e| format!("flush: {}: {e}", f.temp.display()))?;
        drop(f.file);
        if dst.exists() {
            return Err(format!("exists: {}", dst.display()));
        }
        fs::rename(&f.temp, dst).map_err(|e| format!("rename: {}: {e}", dst.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&f.temp);
    }
    result.map(|()| f.written)
}

fn temp_path(dst: &Path) -> PathBuf {
    let mut temp = dst.as_os_str().to_owned();
    temp.push(TEMP_SUFFIX);
    PathBuf::from(temp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, bytes: &[u8], mtime: SystemTime) {
        fs::write(path, bytes).unwrap();
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    fn small() -> Options {
        Options {
            chunk: 7,
            buffers: 2,
        }
    }

    #[test]
    fn copies_content_and_mtime_without_temp_files() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let big: Vec<u8> = (0..100u8).collect();
        write(&src.path().join("a.ARW"), &big, mtime);
        write(&src.path().join("b.JPG"), b"", mtime);
        let jobs: Vec<Job> = ["a.ARW", "b.JPG"]
            .iter()
            .map(|n| Job {
                src: src.path().join(n),
                dst: dst.path().join(n),
            })
            .collect();

        let mut last = None;
        let summary = copy_files_with(&jobs, small(), &AtomicBool::new(false), |p| last = Some(p));

        assert_eq!(
            summary.results,
            vec![Outcome::Copied(100), Outcome::Copied(0)]
        );
        assert_eq!(summary.bytes, 100);
        assert!(!summary.canceled);
        assert_eq!(fs::read(dst.path().join("a.ARW")).unwrap(), big);
        for n in ["a.ARW", "b.JPG"] {
            let m = fs::metadata(dst.path().join(n))
                .unwrap()
                .modified()
                .unwrap();
            assert_eq!(m, mtime);
        }
        assert_eq!(names(dst.path()), vec!["a.ARW", "b.JPG"]);
        assert_eq!(
            last,
            Some(Progress {
                job: 1,
                files_done: 2,
                files_total: 2,
                bytes_done: 100,
                bytes_total: 100,
            })
        );
    }

    #[test]
    fn never_overwrites_an_existing_destination() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        write(&src.path().join("a.ARW"), b"new", now);
        write(&src.path().join("b.ARW"), b"bee", now);
        fs::write(dst.path().join("a.ARW"), b"old").unwrap();
        let jobs: Vec<Job> = ["a.ARW", "b.ARW"]
            .iter()
            .map(|n| Job {
                src: src.path().join(n),
                dst: dst.path().join(n),
            })
            .collect();

        let mut last = None;
        let summary = copy_files_with(&jobs, small(), &AtomicBool::new(false), |p| last = Some(p));

        assert!(matches!(&summary.results[0], Outcome::Failed(e) if e.starts_with("exists:")));
        assert_eq!(summary.results[1], Outcome::Copied(3));
        assert_eq!(fs::read(dst.path().join("a.ARW")).unwrap(), b"old");
        assert_eq!(names(dst.path()), vec!["a.ARW", "b.ARW"]);
        let last = last.unwrap();
        assert_eq!((last.files_done, last.bytes_done), (2, 6));
    }

    #[test]
    fn a_missing_source_fails_alone() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        write(&src.path().join("b.ARW"), b"bee", SystemTime::now());
        let jobs: Vec<Job> = ["a.ARW", "b.ARW"]
            .iter()
            .map(|n| Job {
                src: src.path().join(n),
                dst: dst.path().join(n),
            })
            .collect();

        let summary = copy_files(&jobs, &AtomicBool::new(false), |_| {});

        assert!(matches!(&summary.results[0], Outcome::Failed(e) if e.starts_with("open:")));
        assert_eq!(summary.results[1], Outcome::Copied(3));
        assert_eq!(names(dst.path()), vec!["b.ARW"]);
    }

    #[test]
    fn cancel_mid_file_leaves_no_partial_file() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        write(&src.path().join("a.ARW"), &[1; 10], now);
        write(&src.path().join("b.ARW"), &[2; 1000], now);
        write(&src.path().join("c.ARW"), &[3; 10], now);
        let jobs: Vec<Job> = ["a.ARW", "b.ARW", "c.ARW"]
            .iter()
            .map(|n| Job {
                src: src.path().join(n),
                dst: dst.path().join(n),
            })
            .collect();

        let cancel = AtomicBool::new(false);
        let summary = copy_files_with(&jobs, small(), &cancel, |p| {
            if p.job == 1 && p.bytes_done > 10 {
                cancel.store(true, Ordering::Relaxed);
            }
        });

        assert!(summary.canceled);
        assert_eq!(summary.results[0], Outcome::Copied(10));
        assert_eq!(summary.results[1], Outcome::NotRun);
        assert_eq!(summary.results[2], Outcome::NotRun);
        assert_eq!(names(dst.path()), vec!["a.ARW"]);
    }

    #[test]
    fn cancel_before_start_copies_nothing() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        write(&src.path().join("a.ARW"), b"a", SystemTime::now());
        let jobs = [Job {
            src: src.path().join("a.ARW"),
            dst: dst.path().join("a.ARW"),
        }];

        let summary = copy_files(&jobs, &AtomicBool::new(true), |_| {});

        assert!(summary.canceled);
        assert_eq!(summary.results, vec![Outcome::NotRun]);
        assert!(names(dst.path()).is_empty());
    }
}
