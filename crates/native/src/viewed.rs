use std::collections::HashSet;

/// How long a file's decoded preview has to stay on screen before it counts
/// as viewed, so frames flown past while holding an arrow key do not.
pub const VIEW_DWELL_MS: u64 = 500;

/// Serializes the `set_last_viewed` writes: one in flight at a time, and only
/// the latest value pushed meanwhile is written once it settles, so two
/// writes cannot land out of order and a held-down arrow key collapses into a
/// few. The Tauri frontend's `lastViewedWriter`.
pub struct Latest<T> {
    pending: Option<T>,
    in_flight: bool,
}

impl<T> Default for Latest<T> {
    fn default() -> Self {
        Self {
            pending: None,
            in_flight: false,
        }
    }
}

impl<T> Latest<T> {
    /// The value to write now, `None` while a write is out.
    pub fn push(&mut self, value: T) -> Option<T> {
        self.pending = Some(value);
        if self.in_flight { None } else { self.next() }
    }

    /// The write out has ended, failed or not. The value to write next.
    pub fn settled(&mut self) -> Option<T> {
        self.next()
    }

    fn next(&mut self) -> Option<T> {
        let value = self.pending.take();
        self.in_flight = value.is_some();
        value
    }
}

/// One `mark_viewed` write: a folder and its newly viewed files.
#[derive(Debug, PartialEq, Eq)]
pub struct Batch {
    pub dir: String,
    pub paths: Vec<String>,
}

/// Serializes the `mark_viewed` writes: one in flight at a time, and the
/// paths pushed meanwhile are collected, per folder, into the next write, so
/// paging through a folder writes a few batches rather than one per file. The
/// Tauri frontend's `viewedWriter`.
#[derive(Default)]
pub struct Batches {
    /// In the order the folders were first pushed, each path once.
    pending: Vec<Batch>,
    in_flight: bool,
}

impl Batches {
    /// The batch to write now, `None` while a write is out.
    pub fn push(&mut self, dir: &str, path: &str) -> Option<Batch> {
        match self.pending.iter_mut().find(|b| b.dir == dir) {
            Some(batch) => {
                if !batch.paths.iter().any(|p| p == path) {
                    batch.paths.push(path.to_owned());
                }
            }
            None => self.pending.push(Batch {
                dir: dir.to_owned(),
                paths: vec![path.to_owned()],
            }),
        }
        if self.in_flight { None } else { self.next() }
    }

    /// The write out has ended, failed or not. The batch to write next.
    pub fn settled(&mut self) -> Option<Batch> {
        self.next()
    }

    fn next(&mut self) -> Option<Batch> {
        let batch = (!self.pending.is_empty()).then(|| self.pending.remove(0));
        self.in_flight = batch.is_some();
        batch
    }
}

/// The dwell clock of the file on screen, the Tauri frontend's `dwellTimer`
/// without the timer: `shown` hands out a ticket the caller's timer brings
/// back to `elapsed` after `VIEW_DWELL_MS`, and any later `shown` or `cancel`
/// voids it.
#[derive(Default)]
pub struct Dwell {
    /// The open folder's files shown at least once, across sessions.
    viewed: HashSet<String>,
    ticket: u64,
    timing: Option<(u64, String)>,
}

impl Dwell {
    /// A new folder, with the files the index has as viewed.
    pub fn reset(&mut self, viewed: HashSet<String>) {
        self.viewed = viewed;
        self.cancel();
    }

    /// `path`'s preview is on screen: (re)start the clock. The ticket to
    /// bring back, `None` for a path already viewed.
    pub fn shown(&mut self, path: &str) -> Option<u64> {
        self.cancel();
        if self.viewed.contains(path) {
            return None;
        }
        self.timing = Some((self.ticket, path.to_owned()));
        Some(self.ticket)
    }

    pub fn cancel(&mut self) {
        self.ticket += 1;
        self.timing = None;
    }

    /// The dwell of `ticket` is over: the path to record as viewed, `None`
    /// when the ticket was voided.
    pub fn elapsed(&mut self, ticket: u64) -> Option<String> {
        match self.timing.take() {
            Some((at, path)) if at == ticket => {
                self.viewed.insert(path.clone());
                Some(path)
            }
            timing => {
                self.timing = timing;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Batch, Batches, Dwell, Latest};
    use std::collections::HashSet;

    fn batch(dir: &str, paths: &[&str]) -> Batch {
        Batch {
            dir: dir.into(),
            paths: paths.iter().map(|p| p.to_string()).collect(),
        }
    }

    #[test]
    fn latest_writes_at_once_when_idle() {
        let mut latest = Latest::default();
        assert_eq!(latest.push("/d/a.ARW"), Some("/d/a.ARW"));
    }

    #[test]
    fn latest_keeps_only_the_latest_value_while_one_is_in_flight() {
        let mut latest = Latest::default();
        assert_eq!(latest.push("/d/a.ARW"), Some("/d/a.ARW"));
        assert_eq!(latest.push("/d/b.ARW"), None);
        assert_eq!(latest.push("/d/c.ARW"), None);
        assert_eq!(latest.settled(), Some("/d/c.ARW"));
        assert_eq!(latest.settled(), None);
        assert_eq!(latest.push("/e/a.ARW"), Some("/e/a.ARW"));
    }

    #[test]
    fn batches_write_at_once_when_idle() {
        let mut batches = Batches::default();
        assert_eq!(
            batches.push("/d", "/d/a.ARW"),
            Some(batch("/d", &["/d/a.ARW"]))
        );
    }

    #[test]
    fn batches_collect_the_paths_pushed_while_one_is_in_flight() {
        let mut batches = Batches::default();
        batches.push("/d", "/d/a.ARW");
        assert_eq!(batches.push("/d", "/d/b.ARW"), None);
        assert_eq!(batches.push("/d", "/d/c.ARW"), None);
        assert_eq!(batches.push("/d", "/d/b.ARW"), None);
        assert_eq!(
            batches.settled(),
            Some(batch("/d", &["/d/b.ARW", "/d/c.ARW"]))
        );
        assert_eq!(batches.settled(), None);
    }

    #[test]
    fn batches_keep_folders_apart() {
        let mut batches = Batches::default();
        batches.push("/d", "/d/a.ARW");
        batches.push("/d", "/d/b.ARW");
        batches.push("/e", "/e/a.ARW");
        assert_eq!(batches.settled(), Some(batch("/d", &["/d/b.ARW"])));
        assert_eq!(batches.settled(), Some(batch("/e", &["/e/a.ARW"])));
        assert_eq!(batches.settled(), None);
        assert_eq!(
            batches.push("/d", "/d/c.ARW"),
            Some(batch("/d", &["/d/c.ARW"]))
        );
    }

    #[test]
    fn a_display_cancelled_before_the_dwell_records_nothing() {
        let mut dwell = Dwell::default();
        let ticket = dwell.shown("/d/a.ARW").unwrap();
        dwell.cancel();
        assert_eq!(dwell.elapsed(ticket), None);
    }

    #[test]
    fn a_display_replaced_before_the_dwell_records_only_the_one_that_stays() {
        let mut dwell = Dwell::default();
        let a = dwell.shown("/d/a.ARW").unwrap();
        let b = dwell.shown("/d/b.ARW").unwrap();
        let c = dwell.shown("/d/c.ARW").unwrap();
        assert_eq!(dwell.elapsed(a), None);
        assert_eq!(dwell.elapsed(b), None);
        assert_eq!(dwell.elapsed(c).as_deref(), Some("/d/c.ARW"));
    }

    #[test]
    fn a_display_past_the_dwell_records_once() {
        let mut dwell = Dwell::default();
        let ticket = dwell.shown("/d/a.ARW").unwrap();
        assert_eq!(dwell.elapsed(ticket).as_deref(), Some("/d/a.ARW"));
        assert_eq!(dwell.elapsed(ticket), None);
    }

    #[test]
    fn showing_a_recorded_path_again_does_not_record_it_again() {
        let mut dwell = Dwell::default();
        let ticket = dwell.shown("/d/a.ARW").unwrap();
        dwell.elapsed(ticket);
        assert_eq!(dwell.shown("/d/a.ARW"), None);
    }

    #[test]
    fn a_path_the_index_has_as_viewed_is_not_recorded_again() {
        let mut dwell = Dwell::default();
        let stale = dwell.shown("/d/b.ARW").unwrap();
        dwell.reset(HashSet::from(["/d/a.ARW".to_string()]));
        assert_eq!(dwell.elapsed(stale), None);
        assert_eq!(dwell.shown("/d/a.ARW"), None);
        assert!(dwell.shown("/d/b.ARW").is_some());
    }
}
