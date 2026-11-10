#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Next,
    Previous,
    First,
    Last,
    /// A file picked by index, clamped to the list.
    To(usize),
}

/// Which file is current and whether a preview request is out. One request is
/// in flight at a time, so holding a key never queues stale decodes.
pub struct Pager {
    len: usize,
    index: usize,
    in_flight: bool,
}

impl Pager {
    pub fn new(len: usize) -> Self {
        Self {
            len,
            index: 0,
            in_flight: false,
        }
    }

    pub fn current(&self) -> usize {
        self.index
    }

    /// Move the current file, wrapping around. Whether it changed.
    pub fn go(&mut self, to: Move) -> bool {
        if self.len == 0 {
            return false;
        }
        let before = self.index;
        self.index = match to {
            Move::Next => (self.index + 1) % self.len,
            Move::Previous => (self.index + self.len - 1) % self.len,
            Move::First => 0,
            Move::Last => self.len - 1,
            Move::To(index) => index.min(self.len - 1),
        };
        self.index != before
    }

    /// The index to request now, or `None` while another request is out.
    pub fn request(&mut self) -> Option<usize> {
        if self.in_flight || self.len == 0 {
            return None;
        }
        self.in_flight = true;
        Some(self.index)
    }

    /// A result for `index` arrived. `None` when it is the current file's, so
    /// nothing is in flight any more; otherwise the result is stale and this
    /// is the index to request instead, which stays in flight.
    pub fn finish(&mut self, index: usize) -> Option<usize> {
        if index == self.index {
            self.in_flight = false;
            None
        } else {
            Some(self.index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Move, Pager};

    #[test]
    fn wraps_around_both_ways() {
        let mut p = Pager::new(3);
        assert!(p.go(Move::Previous));
        assert_eq!(p.index, 2);
        assert!(p.go(Move::Next));
        assert_eq!(p.index, 0);
    }

    #[test]
    fn home_and_end_jump_and_report_no_move_when_already_there() {
        let mut p = Pager::new(3);
        assert!(!p.go(Move::First));
        assert!(p.go(Move::Last));
        assert_eq!(p.index, 2);
        assert!(!p.go(Move::Last));
    }

    #[test]
    fn a_picked_file_becomes_current_clamped_to_the_list() {
        let mut p = Pager::new(3);
        assert!(p.go(Move::To(1)));
        assert!(!p.go(Move::To(1)));
        assert!(p.go(Move::To(9)));
        assert_eq!(p.index, 2);
    }

    #[test]
    fn a_single_file_and_an_empty_list_never_move() {
        assert!(!Pager::new(1).go(Move::Next));
        let mut none = Pager::new(0);
        assert!(!none.go(Move::Next));
        assert_eq!(none.request(), None);
    }

    #[test]
    fn a_result_for_a_stale_index_re_requests_the_current_one() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        p.go(Move::Next);
        assert_eq!(p.finish(0), Some(1));
        assert_eq!(p.request(), None);
    }

    #[test]
    fn a_result_for_the_current_index_clears_the_in_flight_marker() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        assert_eq!(p.finish(0), None);
        p.go(Move::Next);
        assert_eq!(p.request(), Some(1));
    }

    #[test]
    fn two_presses_while_in_flight_produce_exactly_one_further_request() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        p.go(Move::Next);
        assert_eq!(p.request(), None);
        p.go(Move::Next);
        assert_eq!(p.request(), None);
        assert_eq!(p.finish(0), Some(2));
        assert_eq!(p.request(), None);
        assert_eq!(p.finish(2), None);
    }
}
