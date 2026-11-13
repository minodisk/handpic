//! An operation pressed while the open folder is listed or scanned waits for
//! that scan's end instead of being refused. A port of the Tauri app's
//! `idle.ts`: one slot, a later request replacing (and so canceling) the held
//! operation, `drain` at the scan's end, and an in-flight count while one
//! runs.

pub struct IdleGate<T> {
    held: Option<(&'static str, T)>,
    in_flight: usize,
}

impl<T> Default for IdleGate<T> {
    fn default() -> Self {
        Self {
            held: None,
            in_flight: 0,
        }
    }
}

impl<T> IdleGate<T> {
    /// `op` to run now, or `None` when `busy` held it under `label`. A held
    /// operation it replaces is dropped, as `discard` drops it.
    pub fn request(&mut self, label: &'static str, op: T, busy: bool) -> Option<T> {
        self.discard();
        if busy {
            self.held = Some((label, op));
            None
        } else {
            Some(op)
        }
    }

    /// The scan ended: the held operation, to run now, at most once.
    pub fn drain(&mut self) -> Option<T> {
        self.held.take().map(|(_, op)| op)
    }

    /// Drop the held operation, as a folder switch does.
    pub fn discard(&mut self) -> Option<T> {
        self.held.take().map(|(_, op)| op)
    }

    /// The held operation's label.
    pub fn waiting(&self) -> Option<&'static str> {
        self.held.as_ref().map(|(label, _)| *label)
    }

    /// A count rather than a flag, as in `idle.ts`: two operations can
    /// overlap, and the first to settle must not clear the other's.
    pub fn enter_in_flight(&mut self) {
        self.in_flight += 1;
    }

    pub fn leave_in_flight(&mut self) {
        self.in_flight = self.in_flight.saturating_sub(1);
    }

    pub fn in_flight(&self) -> bool {
        self.in_flight > 0
    }
}

/// Whether the operation held under `label` stops the running scan rather
/// than waiting it out: its files are about to go, so scanning them is
/// wasted.
pub fn cancels_scan(label: Option<&str>) -> bool {
    matches!(label, Some("Move to Trash" | "Delete Permanently"))
}

#[cfg(test)]
mod tests {
    use super::{IdleGate, cancels_scan};

    #[test]
    fn runs_at_once_when_idle() {
        let mut gate = IdleGate::default();
        assert_eq!(gate.request("a", 'a', false), Some('a'));
        assert_eq!(gate.waiting(), None);
    }

    #[test]
    fn holds_while_busy_and_reports_the_label() {
        let mut gate = IdleGate::default();
        assert_eq!(gate.request("a", 'a', true), None);
        assert_eq!(gate.waiting(), Some("a"));
    }

    #[test]
    fn drain_gives_the_held_operation_exactly_once_and_clears_it() {
        let mut gate = IdleGate::default();
        gate.request("a", 'a', true);
        assert_eq!(gate.drain(), Some('a'));
        assert_eq!(gate.drain(), None);
        assert_eq!(gate.waiting(), None);
    }

    #[test]
    fn a_second_request_while_busy_replaces_the_first() {
        let mut gate = IdleGate::default();
        gate.request("a", 'a', true);
        gate.request("b", 'b', true);
        assert_eq!(gate.waiting(), Some("b"));
        assert_eq!(gate.drain(), Some('b'));
    }

    #[test]
    fn discard_drops_the_held_operation() {
        let mut gate = IdleGate::default();
        gate.request("a", 'a', true);
        assert_eq!(gate.discard(), Some('a'));
        assert_eq!(gate.discard(), None);
        assert_eq!(gate.waiting(), None);
        assert_eq!(gate.drain(), None);
    }

    #[test]
    fn drain_with_nothing_held_is_a_no_op() {
        let mut gate = IdleGate::<char>::default();
        assert_eq!(gate.drain(), None);
        assert_eq!(gate.waiting(), None);
    }

    #[test]
    fn a_request_that_runs_at_once_holds_nothing() {
        let mut gate = IdleGate::default();
        assert_eq!(gate.request("a", 'a', false), Some('a'));
        assert_eq!(gate.discard(), None);
        assert_eq!(gate.waiting(), None);
    }

    #[test]
    fn the_in_flight_count_clears_once_every_operation_settled() {
        let mut gate = IdleGate::<char>::default();
        assert!(!gate.in_flight());
        gate.enter_in_flight();
        gate.enter_in_flight();
        gate.leave_in_flight();
        assert!(gate.in_flight());
        gate.leave_in_flight();
        assert!(!gate.in_flight());
    }

    #[test]
    fn cancels_scan_for_move_to_trash_and_delete_permanently() {
        assert!(cancels_scan(Some("Move to Trash")));
        assert!(cancels_scan(Some("Delete Permanently")));
    }

    #[test]
    fn cancels_scan_not_for_the_operations_that_wait_out_the_scan_nor_for_none() {
        assert!(!cancels_scan(Some("Rename…")));
        assert!(!cancels_scan(Some("Undo Move to Trash")));
        assert!(!cancels_scan(Some("Delete Sidecars")));
        assert!(!cancels_scan(None));
    }
}
