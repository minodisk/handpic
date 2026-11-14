/// When a change the folder watcher reported is acted on, as the Tauri
/// frontend's `resync` / `resyncPending`: never while the open folder is being
/// listed or scanned (a re-listing would cancel and restart that scan), but
/// once it is over, and a burst of changes meanwhile collapses into one. A
/// move to the Trash or a delete in flight holds it busy the same way, and
/// its end re-lists through `open`.
#[derive(Default)]
pub struct Resync {
    /// A listing or a scan of the open folder is under way.
    busy: bool,
    /// A change arrived while busy.
    pending: bool,
}

impl Resync {
    /// A folder is being opened: busy, and a change seen in the folder before
    /// is moot since the open lists it afresh.
    pub fn open(&mut self) {
        self.busy = true;
        self.pending = false;
    }

    /// A listing or a scan of the open folder, or a run holding it, is
    /// under way.
    pub fn busy(&self) -> bool {
        self.busy
    }

    /// A run on the open folder's files starts: busy until its re-listing
    /// ends, a change meanwhile kept for then.
    pub fn hold(&mut self) {
        self.busy = true;
    }

    /// The watcher reported a change. Whether to re-list now; otherwise it
    /// waits for `finished`.
    pub fn changed(&mut self) -> bool {
        if self.busy {
            self.pending = true;
            return false;
        }
        self.busy = true;
        true
    }

    /// The listing and its scan are over (or there was no scan to run).
    /// Whether to re-list now for a change that waited.
    pub fn finished(&mut self) -> bool {
        if self.pending {
            self.pending = false;
            return true;
        }
        self.busy = false;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::Resync;

    #[test]
    fn a_change_when_idle_relists_at_once() {
        let mut r = Resync::default();
        assert!(r.changed());
        assert!(!r.changed());
        assert!(r.finished());
        assert!(!r.finished());
        assert!(r.changed());
    }

    #[test]
    fn changes_during_an_open_wait_for_its_scan_and_collapse() {
        let mut r = Resync::default();
        r.open();
        assert!(!r.changed());
        assert!(!r.changed());
        assert!(r.finished());
        assert!(!r.finished());
        assert!(r.changed());
    }

    #[test]
    fn an_open_drops_the_pending_change() {
        let mut r = Resync::default();
        r.open();
        assert!(!r.changed());
        r.open();
        assert!(!r.finished());
    }

    #[test]
    fn a_held_run_defers_a_change_and_its_relisting_drops_it() {
        let mut r = Resync::default();
        assert!(!r.busy());
        r.hold();
        assert!(r.busy());
        assert!(!r.changed());
        r.open();
        assert!(!r.finished());
        assert!(!r.busy());
    }
}
