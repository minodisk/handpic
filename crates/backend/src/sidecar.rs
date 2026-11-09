//! The single writer thread that turns judgments into sidecars, in the
//! format selected by the `sidecarFormat` setting (XMP, DxO PhotoLab `.dop`,
//! or both).
//!
//! A keypress must never wait on disk, so `set_rating` only writes the
//! `ratings` row and hands the judgment to this thread. Entries are
//! coalesced per path with a trailing debounce — mashing `1`, `2`, `3` on one
//! file writes the sidecar once, with `3` — and each write goes to a temp file
//! that is fsynced and renamed over the sidecar, so a crash mid-write leaves
//! the previous sidecar intact rather than a truncated one.
//!
//! Nothing here is async: the thread blocks on `recv_timeout` against the
//! earliest deadline, which makes the drain on quit a plain synchronous
//! handshake.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use riffle_core::copy::TEMP_SUFFIX;
use riffle_core::xmp::LabelNames;
use riffle_core::{dop, xmp, Flag};

use crate::index::{self, lock, Index};
use crate::photolab;

/// Which sidecar Riffle reads and writes. `Xmp` and `Dop` are one format
/// each, whose other format's files are neither read nor written; `Both`
/// writes every judgment to both and reads back the one modified last.
///
/// `Both` names no single file: the per-file methods (`sidecar_path`, the
/// readers and the writers) panic on it, so call `kinds()` first and use them
/// on each kind it yields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SidecarFormat {
    #[default]
    Xmp,
    Dop,
    Both,
}

impl SidecarFormat {
    /// The format a stored `sidecarFormat` value names; anything but `"dop"`
    /// or `"both"` (missing, unknown) is `Xmp`.
    pub fn from_setting(value: Option<&str>) -> Self {
        match value {
            Some("dop") => Self::Dop,
            Some("both") => Self::Both,
            _ => Self::Xmp,
        }
    }

    /// The value stored under `sidecarFormat` for this format.
    pub fn setting(self) -> &'static str {
        match self {
            Self::Xmp => "xmp",
            Self::Dop => "dop",
            Self::Both => "both",
        }
    }

    /// The single-file formats this setting covers, XMP first; the order is
    /// the tie-break of `newest`.
    pub fn kinds(self) -> &'static [SidecarFormat] {
        match self {
            Self::Xmp => &[Self::Xmp],
            Self::Dop => &[Self::Dop],
            Self::Both => &[Self::Xmp, Self::Dop],
        }
    }

    pub fn sidecar_path(self, arw: &Path) -> PathBuf {
        match self {
            Self::Xmp => xmp::sidecar_path(arw),
            Self::Dop => dop::sidecar_path(arw),
            Self::Both => unreachable!("call kinds() first"),
        }
    }

    /// The `0`-`5` stars the sidecar holds, `None` when it has none.
    pub fn read_rating(self, bytes: &[u8]) -> Result<Option<i8>, String> {
        match self {
            Self::Xmp => xmp::read_rating(bytes),
            Self::Dop => dop::read_rating(bytes),
            Self::Both => unreachable!("call kinds() first"),
        }
    }

    /// The pick / reject the sidecar holds.
    pub fn read_flag(self, bytes: &[u8]) -> Result<Flag, String> {
        match self {
            Self::Xmp => xmp::read_flag(bytes),
            Self::Dop => dop::read_flag(bytes),
            Self::Both => unreachable!("call kinds() first"),
        }
    }

    /// The raw color label the sidecar holds, `None` when it has none.
    /// `names` are the `xmp:Label` names an XMP label is matched against.
    pub fn read_label(self, bytes: &[u8], names: &LabelNames) -> Result<Option<String>, String> {
        match self {
            Self::Xmp => xmp::read_label(bytes, names),
            Self::Dop => dop::read_label(bytes),
            Self::Both => unreachable!("call kinds() first"),
        }
    }

    /// `existing` (or a fresh sidecar of `arw`) with its color label set to
    /// `label`, or removed when `None`. An XMP label is written under its
    /// name in `names`; `orientation`, the EXIF Orientation of `arw`, and
    /// `uuids`, the Uuids of a fresh sidecar, only go into a `.dop` (see
    /// `dop::write_rating`).
    pub fn write_label(
        self,
        arw: &Path,
        existing: Option<&[u8]>,
        label: Option<&str>,
        names: &LabelNames,
        orientation: Option<u16>,
        uuids: Option<dop::Uuids>,
    ) -> Result<Vec<u8>, String> {
        match self {
            Self::Xmp => xmp::write_label(existing, label, names),
            Self::Dop => dop::write_label(
                existing,
                label,
                &raw_name(arw),
                orientation,
                &dop::timestamp(SystemTime::now()),
                uuids,
            ),
            Self::Both => unreachable!("call kinds() first"),
        }
    }

    /// The sidecar bytes of `arw` carrying `rating` and `flag`, patched from
    /// `existing` or freshly minted; `orientation` and `uuids` as for
    /// `write_label`.
    pub fn write_rating(
        self,
        arw: &Path,
        existing: Option<&[u8]>,
        rating: Option<i8>,
        flag: Flag,
        orientation: Option<u16>,
        uuids: Option<dop::Uuids>,
    ) -> Result<Vec<u8>, String> {
        match self {
            Self::Xmp => xmp::write_rating(existing, rating, flag),
            Self::Dop => dop::write_rating(
                existing,
                rating,
                flag,
                &raw_name(arw),
                orientation,
                &dop::timestamp(SystemTime::now()),
                uuids,
            ),
            Self::Both => unreachable!("call kinds() first"),
        }
    }

    /// Whether a directory entry named `name` is a sidecar of this format:
    /// `*.xmp`, or a RAW file name plus `.dop` (`*.arw.dop`, `*.dng.dop`),
    /// all case-insensitive; `Both` matches either.
    pub fn matches(self, name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        match self {
            Self::Xmp => Path::new(&name).extension().is_some_and(|x| x == "xmp"),
            Self::Dop => name
                .strip_suffix(".dop")
                .is_some_and(|raw| riffle_core::scan::is_raw_file(Path::new(raw))),
            Self::Both => Self::Xmp.matches(&name) || Self::Dop.matches(&name),
        }
    }
}

/// The effective one of a file's existing sidecars, given as
/// `(sidecar, mtime_ns)` in `kinds()` order: the one modified last, and on a
/// tie (an exFAT card keeps mtimes to 2 s) the one listed first. Both the
/// writer, picking the stat it stores, and the folder open, picking the file
/// it compares against that stat, go through this, so the two never disagree.
pub fn newest<T>(sidecars: impl IntoIterator<Item = (T, i64)>) -> Option<T> {
    sidecars
        .into_iter()
        .fold(
            None,
            |best: Option<(T, i64)>, (sidecar, mtime_ns)| match best {
                Some((_, best_ns)) if best_ns >= mtime_ns => best,
                _ => Some((sidecar, mtime_ns)),
            },
        )
        .map(|(sidecar, _)| sidecar)
}

fn raw_name(arw: &Path) -> String {
    arw.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// The EXIF Orientation of `arw` for its `.dop`: the index row's, else read
/// from the file's head when it has no row yet or the row's own Orientation
/// is unset (an error row), `None` when neither yields one. The index lock
/// is released before the file is read.
fn raw_orientation(index: &Arc<Mutex<Index>>, arw: &Path) -> Option<u16> {
    match lock(index).orientation(&arw.to_string_lossy()) {
        Ok(Some(Some(orientation))) => Some(orientation),
        Ok(Some(None)) | Ok(None) | Err(_) => riffle_core::reader::read_metadata(arw)
            .ok()
            .map(|m| m.orientation),
    }
}

/// How long a path's last update is held before its sidecar is written.
pub const DEBOUNCE: Duration = Duration::from_millis(300);

/// Delay before the first retry of a failed write; each further retry doubles
/// it.
const RETRY_BASE: Duration = Duration::from_secs(1);

/// Retries of a failed write before it is given up on. Five retries (1 s, 2 s,
/// 4 s, 8 s, 16 s, ~31 s in all) ride out a card reconnect or a transient
/// lock, while a genuinely read-only folder stops reporting within a minute;
/// the dirty row is then written on the next folder open.
const RETRY_LIMIT: u32 = 5;

/// The delay before retry number `attempts` (1-based) of a failed write, or
/// `None` once `RETRY_LIMIT` retries have been made.
fn retry_delay(attempts: u32) -> Option<Duration> {
    (1..=RETRY_LIMIT)
        .contains(&attempts)
        .then(|| RETRY_BASE * 2u32.pow(attempts - 1))
}

/// Longest the quit path waits for the writer to drain.
pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

/// One file's rating, flag and color label, bundled so the writer's queue
/// and channel do not need one argument per field.
///
/// `label_known` is false when the caller has not learned this path's label
/// yet (e.g. a judgment made before the first `folder_entries` refresh, or
/// before a sidecar parse has landed); `label` is then ignored and the
/// sidecar's own current label, read from disk, is kept instead of being
/// cleared.
#[derive(Clone)]
struct Judgment {
    rating: Option<i8>,
    flag: Flag,
    label: Option<String>,
    label_known: bool,
}

enum Message {
    /// Queue a judgment, to be written once `deadline` has passed.
    Set {
        path: PathBuf,
        judgment: Judgment,
        format: SidecarFormat,
        names: LabelNames,
        deadline: Instant,
    },
    /// Write everything pending now and answer on the channel.
    Flush(Sender<()>),
}

/// A handle on the writer thread. Dropping it ends the thread, but the quit
/// path does not rely on that: it flushes explicitly (see `flush`), because a
/// `Drop` during process teardown is not guaranteed to run.
pub struct Writer {
    tx: Mutex<Sender<Message>>,
}

impl Writer {
    /// Start the thread. `on_error` reports every failed attempt at writing a
    /// sidecar; the write is retried with a growing delay (see `retry_delay`),
    /// and once the retries run out the row stays dirty and is written on the
    /// next folder open.
    pub fn spawn<F>(index: Arc<Mutex<Index>>, on_error: F) -> Self
    where
        F: Fn(&Path, &str) + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || run(rx, index, on_error));
        Self { tx: Mutex::new(tx) }
    }

    /// Queue one judgment, to be written `DEBOUNCE` after the last update of
    /// that path. Fails only once the thread is gone.
    #[allow(clippy::too_many_arguments)]
    pub fn set(
        &self,
        path: PathBuf,
        rating: Option<i8>,
        flag: Flag,
        label: Option<String>,
        label_known: bool,
        format: SidecarFormat,
        names: LabelNames,
    ) -> Result<(), String> {
        self.send(
            path,
            Judgment {
                rating,
                flag,
                label,
                label_known,
            },
            format,
            names,
            Instant::now() + DEBOUNCE,
        )
    }

    /// Queue one judgment with no debounce, for the dirty rows a folder open
    /// finds: they were queued in an earlier session and have waited long
    /// enough already.
    #[allow(clippy::too_many_arguments)]
    pub fn set_now(
        &self,
        path: PathBuf,
        rating: Option<i8>,
        flag: Flag,
        label: Option<String>,
        label_known: bool,
        format: SidecarFormat,
        names: LabelNames,
    ) -> Result<(), String> {
        self.send(
            path,
            Judgment {
                rating,
                flag,
                label,
                label_known,
            },
            format,
            names,
            Instant::now(),
        )
    }

    fn send(
        &self,
        path: PathBuf,
        judgment: Judgment,
        format: SidecarFormat,
        names: LabelNames,
        deadline: Instant,
    ) -> Result<(), String> {
        lock(&self.tx)
            .send(Message::Set {
                path,
                judgment,
                format,
                names,
                deadline,
            })
            .map_err(|_| "the sidecar writer is not running".to_string())
    }

    /// Write everything pending and wait for it, at most `timeout`. Used on
    /// quit so a normal Cmd+Q loses nothing. `false` when the wait timed out
    /// or the thread is gone.
    pub fn flush(&self, timeout: Duration) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();
        if lock(&self.tx).send(Message::Flush(tx)).is_err() {
            return false;
        }
        rx.recv_timeout(timeout).is_ok()
    }
}

fn run<F>(rx: Receiver<Message>, index: Arc<Mutex<Index>>, on_error: F)
where
    F: Fn(&Path, &str),
{
    let mut pending: Pending = std::collections::HashMap::new();
    loop {
        let wait = pending
            .values()
            .map(|entry| entry.deadline.saturating_duration_since(Instant::now()))
            .min();
        let message = match wait {
            Some(wait) => rx.recv_timeout(wait),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match message {
            Ok(Message::Set {
                path,
                judgment,
                format,
                names,
                deadline,
            }) => {
                pending.insert(
                    path,
                    Entry {
                        judgment,
                        format,
                        names,
                        deadline,
                        attempts: 0,
                    },
                );
            }
            Ok(Message::Flush(reply)) => {
                flush(&mut pending, None, &index, &on_error);
                let _ = reply.send(());
            }
            Err(RecvTimeoutError::Timeout) => {
                flush(&mut pending, Some(Instant::now()), &index, &on_error);
            }
            Err(RecvTimeoutError::Disconnected) => {
                flush(&mut pending, None, &index, &on_error);
                return;
            }
        }
    }
}

/// A queued judgment, with the format and label names selected when it was
/// made, the instant its sidecar is due and how many retries of it have
/// already failed.
struct Entry {
    judgment: Judgment,
    format: SidecarFormat,
    names: LabelNames,
    deadline: Instant,
    attempts: u32,
}

/// The entry queued per path.
type Pending = std::collections::HashMap<PathBuf, Entry>;

/// Write the entries whose deadline has passed by `now`, or all of them when
/// `now` is `None`. A failed write is requeued for a retry, except in a drain
/// (`now` is `None`), which makes one attempt so quitting never waits out a
/// backoff.
fn flush<F>(pending: &mut Pending, now: Option<Instant>, index: &Arc<Mutex<Index>>, on_error: &F)
where
    F: Fn(&Path, &str),
{
    let due: Vec<PathBuf> = pending
        .iter()
        .filter(|(_, entry)| now.is_none_or(|now| now >= entry.deadline))
        .map(|(path, _)| path.clone())
        .collect();
    for path in due {
        let entry = pending.remove(&path).expect("just listed");
        let judgment = &entry.judgment;
        let orientation = (entry.format != SidecarFormat::Xmp)
            .then(|| raw_orientation(index, &path))
            .flatten();
        match write(&path, judgment, entry.format, &entry.names, orientation) {
            Ok((stat, resolved_label)) => {
                if let Err(e) = lock(index).mark_written(
                    &path.to_string_lossy(),
                    judgment.rating,
                    judgment.flag,
                    resolved_label.as_deref(),
                    judgment.label_known,
                    stat,
                ) {
                    on_error(&path, &e);
                }
            }
            Err(e) => {
                let attempts = entry.attempts + 1;
                if let Some(stat) = e.partial {
                    if let Err(e2) = lock(index).mark_partial_write(
                        &path.to_string_lossy(),
                        judgment.rating,
                        judgment.flag,
                        stat,
                    ) {
                        on_error(&path, &e2);
                    }
                }
                match retry_delay(attempts).filter(|_| now.is_some()) {
                    Some(delay) => {
                        on_error(&path, &format!("{e} (retrying in {}s)", delay.as_secs()));
                        pending.insert(
                            path,
                            Entry {
                                deadline: Instant::now() + delay,
                                attempts,
                                ..entry
                            },
                        );
                    }
                    None => {
                        on_error(&path, &e.message);
                    }
                }
            }
        }
    }
}

/// The sidecar of `arw` as it exists on disk, preferring a name that differs
/// only in case (`FOO.XMP`) over the one `sidecar_path` would mint.
pub fn existing_sidecar(arw: &Path, format: SidecarFormat) -> Option<PathBuf> {
    let wanted = format.sidecar_path(arw);
    if wanted.exists() {
        return Some(wanted);
    }
    let name = wanted.file_name()?.to_string_lossy().into_owned();
    let dir = wanted.parent()?;
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(&name))
        })
}

/// What `write` produces: the effective sidecar's `(size, mtime_ns)` (the
/// newest one under `Both`, see `newest`; `None` when there is nothing to
/// write: clearing a rating on a file that has no sidecar must not litter the
/// folder with an empty one), alongside the label actually written.
type WriteResult = Result<(Option<(i64, i64)>, Option<String>), WriteError>;

/// A failed `write`, carrying the stat of whichever sidecar it did manage to
/// write under `Both` before the other one failed (`None` for a single-file
/// format, or when neither write got that far), so a retry-exhausted `flush`
/// can record it and stop the next folder open from mistaking that write for
/// an external edit (see `flush`).
struct WriteError {
    message: String,
    partial: Option<(i64, i64)>,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Write one file's sidecar, or under `Both` each of its two sidecars.
///
/// `judgment.label_known` is false when `judgment.label` was never learned
/// by the caller (see `Writer::set`); the label is then ignored and the
/// current label of the newest existing sidecar, read from disk, is kept and
/// returned instead (and written to every sidecar), so an unknown label can
/// never clear one PhotoLab or Lightroom already wrote.
///
/// Each sidecar is replaced atomically, but under `Both` the two are not
/// replaced together: a failure (or a crash) between them leaves them
/// disagreeing, and the row dirty, so an in-session retry writes both again.
/// After every failed attempt, including one still queued for a retry,
/// `flush` records the stat of whichever sidecar this call did write
/// (`WriteError::partial`); the row stays dirty, so a rescan during the
/// backoff (or once retries are exhausted) sees an unchanged stat for that
/// sidecar and keeps the judgment queued to replay into both instead of
/// mistaking the earlier write for an external edit. A crash between the two
/// renames, with no retry to follow, still leaves the two disagreeing until
/// the file is judged again.
fn write(
    arw: &Path,
    judgment: &Judgment,
    format: SidecarFormat,
    names: &LabelNames,
    orientation: Option<u16>,
) -> WriteResult {
    let no_partial = |message: String| WriteError {
        message,
        partial: None,
    };
    let mut existing = Vec::new();
    for &kind in format.kinds() {
        if let Some(target) = existing_sidecar(arw, kind) {
            let bytes = std::fs::read(&target)
                .map_err(|e| no_partial(format!("{}: {e}", target.display())))?;
            let mtime_ns = index::stat(&target).map_err(no_partial)?.mtime_ns;
            existing.push(((kind, target, bytes), mtime_ns));
        }
    }
    let resolved_label = if judgment.label_known {
        judgment.label.clone()
    } else {
        newest(
            existing
                .iter()
                .map(|(sidecar, mtime_ns)| (sidecar, *mtime_ns)),
        )
        .map(|(kind, _, bytes)| kind.read_label(bytes, names))
        .transpose()
        .map_err(no_partial)?
        .flatten()
    };
    let mut written = Vec::new();
    let mut first_err = None;
    for &kind in format.kinds() {
        let current = existing
            .iter()
            .find(|((k, _, _), _)| *k == kind)
            .map(|((_, target, bytes), _)| (target.as_path(), bytes.as_slice()));
        match write_kind(
            arw,
            kind,
            current,
            judgment.rating,
            judgment.flag,
            resolved_label.as_deref(),
            names,
            orientation,
        ) {
            Ok(Some(stat)) => written.push((stat, stat.1)),
            Ok(None) => {}
            Err(e) => {
                first_err.get_or_insert(e);
            }
        };
    }
    if let Some(message) = first_err {
        return Err(WriteError {
            message,
            partial: newest(written),
        });
    }
    Ok((newest(written), resolved_label))
}

/// Write the sidecar of `arw` in the single-file format `kind`, patching
/// `current` (its path and bytes) when it exists. Returns its new
/// `(size, mtime_ns)`, or `None` when there was none and nothing to write.
#[allow(clippy::too_many_arguments)]
fn write_kind(
    arw: &Path,
    kind: SidecarFormat,
    current: Option<(&Path, &[u8])>,
    rating: Option<i8>,
    flag: Flag,
    label: Option<&str>,
    names: &LabelNames,
    orientation: Option<u16>,
) -> Result<Option<(i64, i64)>, String> {
    let (target, bytes) = match current {
        None => {
            if rating.is_none() && flag == Flag::None && label.is_none() {
                return Ok(None);
            }
            let uuids = (kind == SidecarFormat::Dop)
                .then(|| photolab::registered_uuids(arw))
                .flatten();
            let bytes = if rating.is_none() && flag == Flag::None {
                kind.write_label(arw, None, label, names, orientation, uuids)?
            } else {
                let rated = kind.write_rating(arw, None, rating, flag, orientation, uuids)?;
                kind.write_label(arw, Some(&rated), label, names, orientation, None)?
            };
            (kind.sidecar_path(arw), bytes)
        }
        Some((target, current)) => {
            let rated = kind.write_rating(arw, Some(current), rating, flag, orientation, None)?;
            // Clearing a label that is not there would still bump the `.dop`
            // timestamps, so it is skipped.
            let bytes = if label.is_none() && kind.read_label(&rated, names)?.is_none() {
                rated
            } else {
                kind.write_label(arw, Some(&rated), label, names, orientation, None)?
            };
            (target.to_path_buf(), bytes)
        }
    };

    let mut temp = target.clone().into_os_string();
    temp.push(TEMP_SUFFIX);
    let temp = PathBuf::from(temp);
    let write_temp = || -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()
    };
    if let Err(e) = write_temp() {
        let _ = std::fs::remove_file(&temp);
        return Err(format!("{}: {e}", temp.display()));
    }
    // On Windows, replacing an existing file this way can transiently fail
    // with a sharing violation while an indexer or antivirus has it briefly
    // open for scanning; a couple of short retries ride that out. POSIX
    // renames succeed on the first try, so this is a no-op there.
    let mut attempt = 0;
    let result = loop {
        match std::fs::rename(&temp, &target) {
            Ok(()) => break Ok(()),
            Err(e) if attempt < 5 => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(20 * attempt));
                let _ = e;
            }
            Err(e) => break Err(e),
        }
    };
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(format!("{}: {e}", target.display()));
    }

    let stat = index::stat(&target)?;
    Ok(Some((stat.size, stat.mtime_ns)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// The names of Lightroom's Japanese default color label set.
    fn japanese() -> LabelNames {
        riffle_core::i18n::preset("ja").unwrap().names.clone()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("riffle-sidecar-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // Best effort, like the removal in temp_dir above: on Windows a directory
    // holding an open SQLite database cannot be removed, and several of these
    // tests still hold the Index when they finish. The next run's temp_dir
    // clears whatever is left over.
    fn remove_temp_dir(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn index(dir: &Path) -> Arc<Mutex<Index>> {
        Arc::new(Mutex::new(Index::open(&dir.join("index.sqlite")).unwrap()))
    }

    fn writer(index: Arc<Mutex<Index>>) -> Writer {
        Writer::spawn(index, |path, message| {
            eprintln!("{}: {message}", path.display())
        })
    }

    /// Wait for `check`, so the tests do not depend on how long the writer
    /// thread takes to be scheduled. The budget is far longer than the work
    /// needs because it only costs time when the assertion is failing anyway,
    /// and a loaded CI runner is much slower than a developer's machine.
    fn eventually(check: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if check() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        check()
    }

    fn arw(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"not really an ARW").unwrap();
        path
    }

    /// A Lightroom-shaped sidecar: the `Rating` as an element, plus a `crs:`
    /// block that must survive the patch byte for byte.
    fn lightroom_sidecar(rating: i32) -> String {
        format!(
            r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
   crs:Exposure2012="+0.35"
   crs:Contrast2012="+12">
   <xmp:Rating>{rating}</xmp:Rating>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
        )
    }

    #[test]
    fn a_burst_within_the_debounce_window_writes_one_sidecar_with_the_last_value() {
        let dir = temp_dir("burst");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "a.ARW");

        for rating in [1i8, 2, 3] {
            lock(&index)
                .set_rating(
                    "d",
                    &path.to_string_lossy(),
                    Some(rating),
                    Flag::None,
                    None,
                    true,
                )
                .unwrap();
            writer
                .set(
                    path.clone(),
                    Some(rating),
                    Flag::None,
                    None,
                    true,
                    SidecarFormat::Xmp,
                    LabelNames::default(),
                )
                .unwrap();
        }

        let sidecar = xmp::sidecar_path(&path);
        assert!(eventually(|| sidecar.exists()));
        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(3));
        assert!(
            eventually(|| lock(&index).dirty_rows("d").unwrap().is_empty()),
            "the write clears dirty"
        );
        // Nothing is left behind by a successful write.
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            !names.iter().any(|n| n.ends_with(TEMP_SUFFIX)),
            "no temp file is left: {names:?}"
        );

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn an_existing_sidecar_is_patched_rather_than_replaced() {
        let dir = temp_dir("patch");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "b.ARW");
        let sidecar = xmp::sidecar_path(&path);
        std::fs::write(&sidecar, lightroom_sidecar(2)).unwrap();

        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(5),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(5),
                Flag::None,
                None,
                true,
                SidecarFormat::Xmp,
                LabelNames::default(),
            )
            .unwrap();

        assert!(eventually(
            || std::fs::read_to_string(&sidecar).unwrap() == lightroom_sidecar(5)
        ));

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_flush_writes_before_the_debounce_elapses() {
        let dir = temp_dir("flush");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "c.ARW");

        lock(&index)
            .set_rating("d", &path.to_string_lossy(), None, Flag::Reject, None, true)
            .unwrap();
        // A deadline far enough out that a slow machine cannot blur the
        // difference between flushing now and waiting for it. Measuring
        // against DEBOUNCE itself made this fail on CI, where the write alone
        // can take longer than 300ms.
        let deadline = Instant::now() + Duration::from_secs(30);
        writer
            .send(
                path.clone(),
                Judgment {
                    rating: None,
                    flag: Flag::Reject,
                    label: None,
                    label_known: true,
                },
                SidecarFormat::Xmp,
                LabelNames::default(),
                deadline,
            )
            .unwrap();
        let started = Instant::now();
        writer.flush(DRAIN_TIMEOUT);

        assert!(
            started.elapsed() < Duration::from_secs(15),
            "the flush did not wait it out"
        );
        let bytes = std::fs::read(xmp::sidecar_path(&path)).unwrap();
        assert_eq!(SidecarFormat::Xmp.read_flag(&bytes).unwrap(), Flag::Reject);

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_a_rating_writes_no_sidecar_when_there_is_none() {
        let dir = temp_dir("clear");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "d.ARW");

        lock(&index)
            .set_rating("d", &path.to_string_lossy(), None, Flag::None, None, true)
            .unwrap();
        writer
            .set(
                path.clone(),
                None,
                Flag::None,
                None,
                true,
                SidecarFormat::Xmp,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        assert!(!xmp::sidecar_path(&path).exists());
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn a_failed_write_leaves_the_row_dirty() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("readonly");
        let dir = root.join("locked");
        std::fs::create_dir_all(&dir).unwrap();
        let path = arw(&dir, "e.ARW");
        let index = index(&root);
        let writer = writer(index.clone());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();

        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(4),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(4),
                Flag::None,
                None,
                true,
                SidecarFormat::Xmp,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        let dirty = lock(&index).dirty_rows("d").unwrap();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].1, Some(4));

        drop(writer);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        remove_temp_dir(&root);
    }

    #[test]
    fn the_retry_delay_grows_and_stops_at_the_limit() {
        let delays: Vec<_> = (1..=RETRY_LIMIT).map(|n| retry_delay(n).unwrap()).collect();
        assert_eq!(delays[0], RETRY_BASE);
        assert!(delays.windows(2).all(|w| w[1] > w[0]));
        assert_eq!(retry_delay(RETRY_LIMIT + 1), None);
    }

    #[cfg(unix)]
    fn counting_writer(index: Arc<Mutex<Index>>) -> (Writer, Arc<AtomicUsize>) {
        let errors = Arc::new(AtomicUsize::new(0));
        let count = errors.clone();
        let writer = Writer::spawn(index, move |path, message| {
            eprintln!("{}: {message}", path.display());
            count.fetch_add(1, Ordering::SeqCst);
        });
        (writer, errors)
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_write_is_retried_until_it_succeeds() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("retry");
        let dir = root.join("locked");
        std::fs::create_dir_all(&dir).unwrap();
        let path = arw(&dir, "r.ARW");
        let index = index(&root);
        let (writer, errors) = counting_writer(index.clone());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();

        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(3),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(3),
                Flag::None,
                None,
                true,
                SidecarFormat::Xmp,
                LabelNames::default(),
            )
            .unwrap();
        assert!(eventually(|| errors.load(Ordering::SeqCst) >= 1));
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();

        let sidecar = xmp::sidecar_path(&path);
        assert!(eventually(|| std::fs::read(&sidecar)
            .is_ok_and(|b| xmp::read_rating(&b) == Ok(Some(3)))
            && lock(&index).dirty_rows("d").unwrap().is_empty()));

        drop(writer);
        remove_temp_dir(&root);
    }

    #[cfg(unix)]
    #[test]
    fn a_newer_judgment_replaces_one_waiting_for_a_retry() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("retry-latest");
        let dir = root.join("locked");
        std::fs::create_dir_all(&dir).unwrap();
        let path = arw(&dir, "l.ARW");
        let index = index(&root);
        let (writer, errors) = counting_writer(index.clone());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();

        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(2),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(2),
                Flag::None,
                None,
                true,
                SidecarFormat::Xmp,
                LabelNames::default(),
            )
            .unwrap();
        assert!(eventually(|| errors.load(Ordering::SeqCst) >= 1));
        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(5),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(5),
                Flag::None,
                None,
                true,
                SidecarFormat::Xmp,
                LabelNames::default(),
            )
            .unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();

        let sidecar = xmp::sidecar_path(&path);
        assert!(eventually(|| std::fs::read(&sidecar)
            .is_ok_and(|b| xmp::read_rating(&b) == Ok(Some(5)))
            && lock(&index).dirty_rows("d").unwrap().is_empty()));

        drop(writer);
        remove_temp_dir(&root);
    }

    const PHOTOLAB_0003: &[u8] = include_bytes!("../../core/src/fixtures/dop/_DSC0003.ARW.dop");

    #[test]
    fn the_format_setting_defaults_to_xmp_and_matches_names_case_insensitively() {
        assert_eq!(SidecarFormat::from_setting(None), SidecarFormat::Xmp);
        assert_eq!(
            SidecarFormat::from_setting(Some("tiff")),
            SidecarFormat::Xmp
        );
        assert_eq!(SidecarFormat::from_setting(Some("dop")), SidecarFormat::Dop);
        assert!(SidecarFormat::Xmp.matches("A.XMP"));
        assert!(!SidecarFormat::Xmp.matches("a.ARW.dop"));
        assert!(SidecarFormat::Dop.matches("A.ARW.DOP"));
        assert!(SidecarFormat::Dop.matches("L1000001.DNG.dop"));
        assert!(!SidecarFormat::Dop.matches("a.jpg.dop"));
        assert_eq!(
            SidecarFormat::from_setting(Some(SidecarFormat::Dop.setting())),
            SidecarFormat::Dop
        );
        assert_eq!(
            SidecarFormat::from_setting(Some(SidecarFormat::Xmp.setting())),
            SidecarFormat::Xmp
        );
        assert!(!SidecarFormat::Dop.matches("a.dop"));
        assert!(!SidecarFormat::Dop.matches("a.xmp"));
        assert_eq!(
            SidecarFormat::from_setting(Some("both")),
            SidecarFormat::Both
        );
        assert_eq!(SidecarFormat::Both.setting(), "both");
        assert_eq!(
            SidecarFormat::from_setting(Some(SidecarFormat::Both.setting())),
            SidecarFormat::Both
        );
        assert!(SidecarFormat::Both.matches("A.XMP"));
        assert!(SidecarFormat::Both.matches("a.ARW.dop"));
        assert!(!SidecarFormat::Both.matches("a.jpg.dop"));
    }

    #[test]
    fn the_newest_sidecar_wins_and_a_tie_goes_to_the_first_listed() {
        assert_eq!(newest([("xmp", 1), ("dop", 2)]), Some("dop"));
        assert_eq!(newest([("xmp", 2), ("dop", 1)]), Some("xmp"));
        assert_eq!(newest([("xmp", 2), ("dop", 2)]), Some("xmp"));
        assert_eq!(newest([("dop", 2)]), Some("dop"));
        assert_eq!(newest::<&str>([]), None);
    }

    fn set_mtime(path: &Path, secs: u64) {
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn a_judgment_under_both_lands_in_the_xmp_and_the_dop() {
        let dir = temp_dir("both-write");
        let index = index(&dir);
        let writer = writer(index.clone());

        let fresh = arw(&dir, "a.ARW");
        judge(
            &index,
            &writer,
            &fresh,
            Some(3),
            Flag::Pick,
            Some("Green"),
            SidecarFormat::Both,
        );
        let bytes = std::fs::read(xmp::sidecar_path(&fresh)).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(3));
        assert_eq!(xmp::read_flag(&bytes).unwrap(), Flag::Pick);
        assert_eq!(
            xmp::read_label(&bytes, &LabelNames::default())
                .unwrap()
                .as_deref(),
            Some("Green")
        );
        let bytes = std::fs::read(dop::sidecar_path(&fresh)).unwrap();
        assert_eq!(dop::read_rating(&bytes).unwrap(), Some(3));
        assert_eq!(dop::read_flag(&bytes).unwrap(), Flag::Pick);
        assert_eq!(dop::read_label(&bytes).unwrap().as_deref(), Some("Green"));

        // A file with only a PhotoLab sidecar gets it patched and an XMP
        // minted next to it.
        let photolab = arw(&dir, "_DSC0003.ARW");
        std::fs::write(dop::sidecar_path(&photolab), PHOTOLAB_0003).unwrap();
        judge(
            &index,
            &writer,
            &photolab,
            Some(5),
            Flag::None,
            None,
            SidecarFormat::Both,
        );
        let text = std::fs::read_to_string(dop::sidecar_path(&photolab)).unwrap();
        assert!(text.contains("\nRating = 5,"), "the .dop is patched");
        assert!(text.ends_with("}\n\r\n"), "the trailing CRLF survives");
        let bytes = std::fs::read(xmp::sidecar_path(&photolab)).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(5));
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_partial_both_write_records_the_xmp_stat_so_a_reopen_still_replays_the_dop() {
        let dir = temp_dir("both-partial");
        let index = index(&dir);
        let writer = writer(index.clone());

        let path = arw(&dir, "a.ARW");
        // Block only the .dop write: `write_kind` writes through a
        // `TEMP_SUFFIX` temp file before renaming it onto the target, so
        // pre-creating that temp path as a directory makes `File::create`
        // fail for the .dop alone, while the XMP (which has no such
        // obstacle) still gets written.
        let dop_temp = {
            let mut p = dop::sidecar_path(&path).into_os_string();
            p.push(TEMP_SUFFIX);
            PathBuf::from(p)
        };
        std::fs::create_dir_all(&dop_temp).unwrap();

        judge(
            &index,
            &writer,
            &path,
            Some(4),
            Flag::None,
            None,
            SidecarFormat::Both,
        );

        assert!(!dop::sidecar_path(&path).exists());
        let bytes = std::fs::read(xmp::sidecar_path(&path)).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(4));
        let dirty = lock(&index).dirty_rows("d").unwrap();
        assert_eq!(dirty.len(), 1, "the row stays dirty for the .dop retry");

        // Simulate the next folder open: reconciling against the XMP's
        // actual stat must not treat this write as an external edit (which
        // would clear `dirty` and strip the .dop's own record), because
        // `flush` recorded that stat via `mark_partial_write` when the .dop
        // write exhausted its retries.
        let xmp_stat = index::stat(&xmp::sidecar_path(&path)).unwrap();
        let (to_parse, _) = lock(&index)
            .reconcile_sidecars(
                "d",
                &[(
                    path.to_string_lossy().into_owned(),
                    Some((xmp::sidecar_path(&path), xmp_stat.size, xmp_stat.mtime_ns)),
                )],
            )
            .unwrap();
        assert!(
            to_parse.is_empty(),
            "the XMP's own write must not be mistaken for an external edit"
        );
        assert_eq!(
            lock(&index).dirty_rows("d").unwrap().len(),
            1,
            "the row is still dirty, so the writer will replay it into the .dop"
        );

        drop(writer);
        std::fs::remove_dir_all(&dop_temp).unwrap();
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_both_write_queued_for_retry_still_records_the_xmp_stat_so_a_rescan_keeps_the_row_dirty() {
        let dir = temp_dir("both-partial-retry");
        let index = index(&dir);

        let path = arw(&dir, "a.ARW");
        // As in the drain case above, block only the .dop write.
        let dop_temp = {
            let mut p = dop::sidecar_path(&path).into_os_string();
            p.push(TEMP_SUFFIX);
            PathBuf::from(p)
        };
        std::fs::create_dir_all(&dop_temp).unwrap();

        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(4),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        let mut pending = Pending::new();
        pending.insert(
            path.clone(),
            Entry {
                judgment: Judgment {
                    rating: Some(4),
                    flag: Flag::None,
                    label: None,
                    label_known: true,
                },
                format: SidecarFormat::Both,
                names: LabelNames::default(),
                deadline: Instant::now(),
                attempts: 0,
            },
        );

        // `now` is `Some`, so a failed write is queued for a retry instead
        // of being treated as exhausted.
        flush(&mut pending, Some(Instant::now()), &index, &|_, _| {});
        assert!(
            pending.contains_key(&path),
            "the entry is requeued for a retry"
        );

        assert!(!dop::sidecar_path(&path).exists());
        let bytes = std::fs::read(xmp::sidecar_path(&path)).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(4));

        // Simulate a rescan while the retry is still pending in the backoff
        // window (e.g. the main window regaining focus): reconciling
        // against the XMP's actual stat must not treat this write as an
        // external edit, because `flush` already recorded that stat via
        // `mark_partial_write` even though the retry has not run out yet.
        let xmp_stat = index::stat(&xmp::sidecar_path(&path)).unwrap();
        let (to_parse, _) = lock(&index)
            .reconcile_sidecars(
                "d",
                &[(
                    path.to_string_lossy().into_owned(),
                    Some((xmp::sidecar_path(&path), xmp_stat.size, xmp_stat.mtime_ns)),
                )],
            )
            .unwrap();
        assert!(
            to_parse.is_empty(),
            "the XMP's own write must not be mistaken for an external edit"
        );
        assert_eq!(
            lock(&index).dirty_rows("d").unwrap().len(),
            1,
            "the row is still dirty, so the pending retry still writes the .dop"
        );

        std::fs::remove_dir_all(&dop_temp).unwrap();
        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_under_both_creates_no_sidecar_and_patches_only_the_existing_one() {
        let dir = temp_dir("both-clear");
        let index = index(&dir);
        let writer = writer(index.clone());

        let none = arw(&dir, "a.ARW");
        judge(
            &index,
            &writer,
            &none,
            None,
            Flag::None,
            None,
            SidecarFormat::Both,
        );
        assert!(!xmp::sidecar_path(&none).exists());
        assert!(!dop::sidecar_path(&none).exists());

        let lightroom = arw(&dir, "b.ARW");
        std::fs::write(xmp::sidecar_path(&lightroom), lightroom_sidecar(2)).unwrap();
        judge(
            &index,
            &writer,
            &lightroom,
            None,
            Flag::None,
            None,
            SidecarFormat::Both,
        );
        assert_eq!(
            std::fs::read_to_string(xmp::sidecar_path(&lightroom)).unwrap(),
            lightroom_sidecar(0)
        );
        assert!(!dop::sidecar_path(&lightroom).exists());
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn an_unknown_label_under_both_keeps_the_newest_sidecars_label_in_both_files() {
        let dir = temp_dir("both-label-unknown");
        let index = index(&dir);
        let writer = writer(index.clone());

        for (xmp_secs, dop_secs, kept) in [(2_000, 1_000, "Red"), (1_000, 2_000, "Blue")] {
            let path = arw(&dir, &format!("{kept}.ARW"));
            let (xmp_path, dop_path) = (xmp::sidecar_path(&path), dop::sidecar_path(&path));
            std::fs::write(
                &xmp_path,
                xmp::write_label(None, Some("Red"), &LabelNames::default()).unwrap(),
            )
            .unwrap();
            std::fs::write(
                &dop_path,
                dop::write_label(
                    Some(PHOTOLAB_0003),
                    Some("Blue"),
                    &raw_name(&path),
                    None,
                    &dop::timestamp(SystemTime::now()),
                    None,
                )
                .unwrap(),
            )
            .unwrap();
            set_mtime(&xmp_path, xmp_secs);
            set_mtime(&dop_path, dop_secs);

            lock(&index)
                .set_rating(
                    "d",
                    &path.to_string_lossy(),
                    Some(4),
                    Flag::None,
                    None,
                    false,
                )
                .unwrap();
            writer
                .set(
                    path.clone(),
                    Some(4),
                    Flag::None,
                    None,
                    false,
                    SidecarFormat::Both,
                    LabelNames::default(),
                )
                .unwrap();
            writer.flush(DRAIN_TIMEOUT);

            let bytes = std::fs::read(&xmp_path).unwrap();
            assert_eq!(
                xmp::read_label(&bytes, &LabelNames::default())
                    .unwrap()
                    .as_deref(),
                Some(kept)
            );
            assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(4));
            let bytes = std::fs::read(&dop_path).unwrap();
            assert_eq!(dop::read_label(&bytes).unwrap().as_deref(), Some(kept));
            assert_eq!(dop::read_rating(&bytes).unwrap(), Some(4));
            assert!(lock(&index).dirty_rows("d").unwrap().is_empty());
        }

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_photolab_sidecar_is_patched_in_place() {
        let dir = temp_dir("dop-patch");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "_DSC0003.ARW");
        let sidecar = dop::sidecar_path(&path);
        std::fs::write(&sidecar, PHOTOLAB_0003).unwrap();

        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(3),
                Flag::Reject,
                None,
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(3),
                Flag::Reject,
                None,
                true,
                SidecarFormat::Dop,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(SidecarFormat::Dop.read_flag(&bytes).unwrap(), Flag::Reject);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\nRating = 3,"), "the stars are kept");
        assert!(text.ends_with("}\n\r\n"), "the trailing CRLF survives");
        assert!(!xmp::sidecar_path(&path).exists(), "no XMP is written");

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_file_with_no_dop_gets_the_minimal_template() {
        let dir = temp_dir("dop-fresh");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "f.ARW");

        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(2),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(2),
                Flag::None,
                None,
                true,
                SidecarFormat::Dop,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        let bytes = std::fs::read(dop::sidecar_path(&path)).unwrap();
        assert_eq!(dop::read_rating(&bytes).unwrap(), Some(2));
        assert!(String::from_utf8(bytes)
            .unwrap()
            .contains("Name = \"f.ARW\""));
        assert!(!xmp::sidecar_path(&path).exists());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_a_rating_writes_no_dop_when_there_is_none() {
        let dir = temp_dir("dop-clear");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "g.ARW");

        lock(&index)
            .set_rating("d", &path.to_string_lossy(), None, Flag::None, None, true)
            .unwrap();
        writer
            .set(
                path.clone(),
                None,
                Flag::None,
                None,
                true,
                SidecarFormat::Dop,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        assert!(!dop::sidecar_path(&path).exists());
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_pick_reaches_the_dop_and_survives_a_later_rating() {
        let dir = temp_dir("dop-pick");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "p.ARW");
        let key = path.to_string_lossy().into_owned();

        lock(&index)
            .set_rating("d", &key, None, Flag::Pick, None, true)
            .unwrap();
        writer
            .set(
                path.clone(),
                None,
                Flag::Pick,
                None,
                true,
                SidecarFormat::Dop,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);
        let bytes = std::fs::read(dop::sidecar_path(&path)).unwrap();
        assert_eq!(
            SidecarFormat::Dop.read_flag(&bytes).unwrap(),
            Flag::Pick,
            "a pick alone mints a .dop"
        );

        lock(&index)
            .set_rating("d", &key, Some(4), Flag::Pick, None, true)
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(4),
                Flag::Pick,
                None,
                true,
                SidecarFormat::Dop,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);
        let bytes = std::fs::read(dop::sidecar_path(&path)).unwrap();
        assert_eq!(SidecarFormat::Dop.read_flag(&bytes).unwrap(), Flag::Pick);
        assert_eq!(dop::read_rating(&bytes).unwrap(), Some(4));
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_pick_reaches_the_xmp() {
        let dir = temp_dir("xmp-pick");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "q.ARW");

        lock(&index)
            .set_rating("d", &path.to_string_lossy(), None, Flag::Pick, None, true)
            .unwrap();
        writer
            .set(
                path.clone(),
                None,
                Flag::Pick,
                None,
                true,
                SidecarFormat::Xmp,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        let bytes = std::fs::read(xmp::sidecar_path(&path)).unwrap();
        assert_eq!(
            SidecarFormat::Xmp.read_flag(&bytes).unwrap(),
            Flag::Pick,
            "a pick alone mints an XMP"
        );
        assert!(!dop::sidecar_path(&path).exists());
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_dop_differing_only_in_case_is_the_one_patched() {
        let dir = temp_dir("dop-case");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "h.ARW");
        let sidecar = dir.join("H.ARW.DOP");
        std::fs::write(&sidecar, PHOTOLAB_0003).unwrap();

        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(5),
                Flag::None,
                None,
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(5),
                Flag::None,
                None,
                true,
                SidecarFormat::Dop,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        // On a case-insensitive file system the rename may take either
        // spelling; what matters is that there is still one sidecar.
        let dops: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| SidecarFormat::Dop.matches(&p.file_name().unwrap().to_string_lossy()))
            .collect();
        assert_eq!(dops.len(), 1, "no second sidecar is minted: {dops:?}");
        let bytes = std::fs::read(&dops[0]).unwrap();
        assert_eq!(dop::read_rating(&bytes).unwrap(), Some(5));

        drop(writer);
        remove_temp_dir(&dir);
    }
    /// Record and queue one full judgment, then wait for the write.
    fn judge(
        index: &Arc<Mutex<Index>>,
        writer: &Writer,
        path: &Path,
        rating: Option<i8>,
        flag: Flag,
        label: Option<&str>,
        format: SidecarFormat,
    ) {
        lock(index)
            .set_rating("d", &path.to_string_lossy(), rating, flag, label, true)
            .unwrap();
        writer
            .set(
                path.to_path_buf(),
                rating,
                flag,
                label.map(str::to_string),
                true,
                format,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);
    }

    /// A TIFF shell whose IFD0 carries an Orientation and a preview of `body`,
    /// as `commands`' tests build one.
    fn arw_with_preview(orientation: u16, body: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"II\x2a\x00");
        buf.extend_from_slice(&8u32.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes());
        let at = 8 + 2 + 3 * 12 + 4;
        for (tag, ty, value) in [
            (0x0112u16, 3u16, orientation as u32),
            (0x0201, 4, at as u32),
            (0x0202, 4, body.len() as u32),
        ] {
            buf.extend_from_slice(&tag.to_le_bytes());
            buf.extend_from_slice(&ty.to_le_bytes());
            buf.extend_from_slice(&1u32.to_le_bytes());
            buf.extend_from_slice(&value.to_le_bytes());
        }
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(body);
        buf
    }

    #[test]
    fn a_fresh_dop_carries_the_raws_own_orientation() {
        let dir = temp_dir("dop-orientation");
        let index = index(&dir);
        let writer = writer(index.clone());

        let path = dir.join("a.ARW");
        std::fs::write(&path, arw_with_preview(8, b"not a jpeg")).unwrap();
        judge(
            &index,
            &writer,
            &path,
            Some(2),
            Flag::None,
            None,
            SidecarFormat::Dop,
        );
        let text = std::fs::read_to_string(dop::sidecar_path(&path)).unwrap();
        assert!(
            text.contains("Name = \"a.ARW\",\nOrientation = 8,\nRating = 2,\n"),
            "{text}"
        );

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn an_error_row_still_falls_back_to_the_raws_own_orientation() {
        // An extraction failure (e.g. the preview does not decode) writes
        // a row with `orientation = NULL`, even when IFD0 parsed fine and
        // carries an Orientation. `raw_orientation` must not read that NULL
        // as the file's Orientation; it must fall back to `read_metadata`.
        let dir = temp_dir("dop-orientation-error-row");
        let index = index(&dir);
        let writer = writer(index.clone());

        let path = dir.join("a.ARW");
        std::fs::write(&path, arw_with_preview(8, b"not a jpeg")).unwrap();
        {
            let mut guard = lock(&index);
            guard
                .write_batch(
                    &dir.to_string_lossy(),
                    &[(
                        index::FileStat {
                            path: path.clone(),
                            size: std::fs::metadata(&path).unwrap().len() as i64,
                            mtime_ns: 0,
                        },
                        Err("preview did not decode".into()),
                    )],
                )
                .unwrap();
        }

        judge(
            &index,
            &writer,
            &path,
            Some(2),
            Flag::None,
            None,
            SidecarFormat::Dop,
        );
        let text = std::fs::read_to_string(dop::sidecar_path(&path)).unwrap();
        assert!(
            text.contains("Name = \"a.ARW\",\nOrientation = 8,\nRating = 2,\n"),
            "{text}"
        );

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_label_alone_mints_an_xmp_and_a_dop() {
        let dir = temp_dir("label-mint");
        let index = index(&dir);
        let writer = writer(index.clone());

        let a = arw(&dir, "a.ARW");
        judge(
            &index,
            &writer,
            &a,
            None,
            Flag::None,
            Some("Green"),
            SidecarFormat::Xmp,
        );
        let bytes = std::fs::read(xmp::sidecar_path(&a)).unwrap();
        assert_eq!(
            xmp::read_label(&bytes, &xmp::LabelNames::default())
                .unwrap()
                .as_deref(),
            Some("Green")
        );

        let b = arw(&dir, "b.ARW");
        judge(
            &index,
            &writer,
            &b,
            None,
            Flag::None,
            Some("Orange"),
            SidecarFormat::Dop,
        );
        let bytes = std::fs::read(dop::sidecar_path(&b)).unwrap();
        assert_eq!(dop::read_label(&bytes).unwrap().as_deref(), Some("Orange"));
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn an_xmp_label_is_written_under_the_configured_name() {
        let dir = temp_dir("label-names");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "a.ARW");
        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                None,
                Flag::None,
                Some("Red"),
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                None,
                Flag::None,
                Some("Red".to_string()),
                true,
                SidecarFormat::Xmp,
                japanese(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        let text = std::fs::read_to_string(xmp::sidecar_path(&path)).unwrap();
        assert!(text.contains(r#"xmp:Label="レッド""#), "{text}");
        assert!(text.contains(r#"photoshop:LabelColor="red""#), "{text}");

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_label_patches_a_lightroom_sidecar_and_survives_a_later_rating() {
        let dir = temp_dir("label-lightroom");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "a.ARW");
        let sidecar = xmp::sidecar_path(&path);
        std::fs::write(&sidecar, lightroom_sidecar(2)).unwrap();

        judge(
            &index,
            &writer,
            &path,
            Some(2),
            Flag::None,
            Some("Red"),
            SidecarFormat::Xmp,
        );
        let text = std::fs::read_to_string(&sidecar).unwrap();
        assert_eq!(
            text.replace(
                r#" xmp:Label="Red" xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/" photoshop:LabelColor="red""#,
                ""
            ),
            lightroom_sidecar(2),
            "only the label and its color are added"
        );

        judge(
            &index,
            &writer,
            &path,
            Some(4),
            Flag::None,
            Some("Red"),
            SidecarFormat::Xmp,
        );
        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(xmp::read_rating(&bytes).unwrap(), Some(4));
        assert_eq!(
            xmp::read_label(&bytes, &xmp::LabelNames::default())
                .unwrap()
                .as_deref(),
            Some("Red")
        );

        judge(
            &index,
            &writer,
            &path,
            Some(4),
            Flag::None,
            None,
            SidecarFormat::Xmp,
        );
        let text = std::fs::read_to_string(&sidecar).unwrap();
        assert_eq!(
            text.replace(
                r#" xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/""#,
                ""
            ),
            lightroom_sidecar(4),
            "clearing removes both properties"
        );

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_a_label_on_a_dop_keeps_the_rating() {
        let dir = temp_dir("label-dop-clear");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "_DSC0003.ARW");
        let sidecar = dop::sidecar_path(&path);
        std::fs::write(&sidecar, PHOTOLAB_0003).unwrap();

        judge(
            &index,
            &writer,
            &path,
            Some(3),
            Flag::None,
            Some("Blue"),
            SidecarFormat::Dop,
        );
        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(dop::read_label(&bytes).unwrap().as_deref(), Some("Blue"));

        judge(
            &index,
            &writer,
            &path,
            Some(3),
            Flag::None,
            None,
            SidecarFormat::Dop,
        );
        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(dop::read_label(&bytes).unwrap(), None);
        assert_eq!(dop::read_rating(&bytes).unwrap(), Some(3));

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn a_judgment_before_the_label_is_known_keeps_the_sidecars_existing_label() {
        // Reproduces "judge before the row or a sidecar parse is known":
        // the writer must not treat an unknown label as "no label" and strip
        // the one PhotoLab already wrote.
        let dir = temp_dir("label-unknown-keeps");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "_DSC0003.ARW");
        let sidecar = dop::sidecar_path(&path);
        std::fs::write(&sidecar, PHOTOLAB_0003).unwrap();

        // PhotoLab already labeled the file, as if a prior session (or a
        // parse this session has not caught up with yet) put it there.
        judge(
            &index,
            &writer,
            &path,
            Some(3),
            Flag::None,
            Some("Red"),
            SidecarFormat::Dop,
        );
        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(dop::read_label(&bytes).unwrap().as_deref(), Some("Red"));

        // A judgment lands with the label unknown, mirroring `set_rating`
        // called before `folder_entries` (or a sidecar parse) has learned it.
        // There is no row for this path in a fresh index, but the guard must
        // hold even when one already exists.
        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(4),
                Flag::None,
                None,
                false,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(4),
                Flag::None,
                None,
                false,
                SidecarFormat::Dop,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);

        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(
            dop::read_label(&bytes).unwrap().as_deref(),
            Some("Red"),
            "the existing label survives a judgment with an unknown label"
        );
        assert_eq!(dop::read_rating(&bytes).unwrap(), Some(4));
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        // The resolved label the write actually kept ("Red") is now stored
        // and marked known in `ratings`, not left `NULL`: a following-up
        // judgment that keeps the same label, this time with `label_known:
        // true`, must still keep it, and a folder-open replay of a dirty row
        // (see `dirty_rows`) must never again send "no label" for this path.
        let stored_label: Option<String> = lock(&index)
            .conn
            .query_row(
                "SELECT label FROM ratings WHERE path = ?1",
                [path.to_string_lossy()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            stored_label.as_deref(),
            Some("Red"),
            "mark_written must persist the resolved label, not leave it NULL"
        );
        lock(&index)
            .set_rating(
                "d",
                &path.to_string_lossy(),
                Some(5),
                Flag::None,
                Some("Red"),
                true,
            )
            .unwrap();
        writer
            .set(
                path.clone(),
                Some(5),
                Flag::None,
                Some("Red".to_string()),
                true,
                SidecarFormat::Dop,
                LabelNames::default(),
            )
            .unwrap();
        writer.flush(DRAIN_TIMEOUT);
        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(
            dop::read_label(&bytes).unwrap().as_deref(),
            Some("Red"),
            "a follow-up known judgment using the now-stored label keeps it"
        );
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_a_label_writes_no_sidecar_when_there_is_none() {
        let dir = temp_dir("label-none");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "a.ARW");

        judge(
            &index,
            &writer,
            &path,
            None,
            Flag::None,
            None,
            SidecarFormat::Xmp,
        );
        judge(
            &index,
            &writer,
            &path,
            None,
            Flag::None,
            None,
            SidecarFormat::Dop,
        );

        assert!(!xmp::sidecar_path(&path).exists());
        assert!(!dop::sidecar_path(&path).exists());
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_everything_strips_stars_and_label_from_an_xmp() {
        let dir = temp_dir("clear-all-xmp");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "a.ARW");
        let sidecar = xmp::sidecar_path(&path);
        std::fs::write(&sidecar, lightroom_sidecar(2)).unwrap();
        judge(
            &index,
            &writer,
            &path,
            Some(4),
            Flag::None,
            Some("Red"),
            SidecarFormat::Xmp,
        );

        judge(
            &index,
            &writer,
            &path,
            None,
            Flag::None,
            None,
            SidecarFormat::Xmp,
        );

        let bytes = std::fs::read(&sidecar).unwrap();
        assert_eq!(
            xmp::read_rating(&bytes).unwrap(),
            Some(0),
            "cleared stars are Rating 0"
        );
        assert_eq!(SidecarFormat::Xmp.read_flag(&bytes).unwrap(), Flag::None);
        assert_eq!(
            xmp::read_label(&bytes, &xmp::LabelNames::default()).unwrap(),
            None
        );
        assert_eq!(
            std::fs::read_to_string(&sidecar).unwrap().replace(
                r#" xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/""#,
                ""
            ),
            lightroom_sidecar(0)
        );
        assert!(lock(&index).dirty_rows("d").unwrap().is_empty());

        drop(writer);
        remove_temp_dir(&dir);
    }

    #[test]
    fn clearing_everything_strips_every_judgment_from_a_dop() {
        let dir = temp_dir("clear-all-dop");
        let index = index(&dir);
        let writer = writer(index.clone());
        let path = arw(&dir, "_DSC0003.ARW");
        let sidecar = dop::sidecar_path(&path);

        for (rating, flag, label) in [
            (Some(4), Flag::None, "Red"),
            (None, Flag::Pick, "Blue"),
            (Some(2), Flag::Reject, "Green"),
        ] {
            std::fs::write(&sidecar, PHOTOLAB_0003).unwrap();
            judge(
                &index,
                &writer,
                &path,
                rating,
                flag,
                Some(label),
                SidecarFormat::Dop,
            );
            let bytes = std::fs::read(&sidecar).unwrap();
            assert_eq!(
                SidecarFormat::Dop.read_rating(&bytes).unwrap(),
                rating.or(Some(0))
            );
            assert_eq!(SidecarFormat::Dop.read_flag(&bytes).unwrap(), flag);
            assert_eq!(dop::read_label(&bytes).unwrap().as_deref(), Some(label));

            judge(
                &index,
                &writer,
                &path,
                None,
                Flag::None,
                None,
                SidecarFormat::Dop,
            );

            let bytes = std::fs::read(&sidecar).unwrap();
            assert_eq!(
                dop::read_rating(&bytes).unwrap(),
                Some(0),
                "{label}: unrated (Rating = 0) and not rejected"
            );
            assert_eq!(
                SidecarFormat::Dop.read_flag(&bytes).unwrap(),
                Flag::None,
                "{label}: not flagged"
            );
            assert_eq!(dop::read_label(&bytes).unwrap(), None, "{label}: no label");
            assert!(lock(&index).dirty_rows("d").unwrap().is_empty());
        }

        drop(writer);
        remove_temp_dir(&dir);
    }
}
