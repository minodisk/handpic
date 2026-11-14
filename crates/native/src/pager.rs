#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Next,
    Previous,
    /// The next file, staying on the last one: Auto-advance.
    NextClamped,
    First,
    Last,
    /// A file picked by index, clamped to the list.
    To(usize),
}

/// Which file is current and whether a preview request is out. One request is
/// in flight at a time, so holding a key never queues stale decodes; a stale
/// result is drawn before the current file is requested, so a held key flips
/// through frames one load behind the cursor.
pub struct Pager {
    len: usize,
    index: usize,
    in_flight: bool,
    /// The current file's preview came from the prefetch ring.
    hit: bool,
}

impl Pager {
    pub fn new(len: usize) -> Self {
        Self {
            len,
            index: 0,
            in_flight: false,
            hit: false,
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
            Move::NextClamped => (self.index + 1).min(self.len - 1),
            Move::First => 0,
            Move::Last => self.len - 1,
            Move::To(index) => index.min(self.len - 1),
        };
        if self.index != before {
            self.hit = false;
        }
        self.index != before
    }

    /// Whether a preview request is out.
    pub fn in_flight(&self) -> bool {
        self.in_flight
    }

    /// The current file is on screen from the prefetch ring, with everything
    /// its result brings: a result still out for another file is dropped
    /// rather than followed by a request, until the current file changes.
    pub fn hit(&mut self) {
        self.hit = true;
    }

    /// The index to request now, or `None` while another request is out.
    pub fn request(&mut self) -> Option<usize> {
        if self.in_flight || self.len == 0 {
            return None;
        }
        self.in_flight = true;
        Some(self.index)
    }

    /// A result for `index` arrived, `ok` when its read succeeded, while the
    /// image of `shown` is on screen (`None` for no image or a failure).
    pub fn finish(&mut self, index: usize, ok: bool, shown: Option<usize>) -> Finished {
        if self.hit {
            self.in_flight = false;
            Finished::Dropped
        } else if index == self.index {
            self.in_flight = false;
            Finished::Current
        } else {
            Finished::Stale {
                draw: ok && shown != Some(self.index),
                next: self.index,
            }
        }
    }
}

/// What to do with a preview result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finished {
    /// The current file's: show it or its failure. Nothing is in flight.
    Current,
    /// Another file's: draw its frame when `draw` (a read that succeeded,
    /// with the current file's image not already on screen), then request
    /// `next`, which stays in flight. A failure is never shown.
    Stale { draw: bool, next: usize },
    /// Another file's, with the current file already on screen from the
    /// prefetch ring: drop it. Nothing is in flight.
    Dropped,
}

#[cfg(test)]
mod tests {
    use super::{Finished, Move, Pager};

    #[test]
    fn wraps_around_both_ways() {
        let mut p = Pager::new(3);
        assert!(p.go(Move::Previous));
        assert_eq!(p.index, 2);
        assert!(p.go(Move::Next));
        assert_eq!(p.index, 0);
    }

    #[test]
    fn the_clamped_next_stays_on_the_last_file() {
        let mut p = Pager::new(2);
        assert!(p.go(Move::NextClamped));
        assert_eq!(p.index, 1);
        assert!(!p.go(Move::NextClamped));
        assert_eq!(p.index, 1);
        assert!(!Pager::new(0).go(Move::NextClamped));
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
    fn a_held_key_sends_one_request_per_stale_result_and_never_a_second_one_out() {
        let mut p = Pager::new(9);
        assert_eq!(p.request(), Some(0));
        let mut shown = None;
        for sent in 0..4 {
            p.go(Move::Next);
            p.go(Move::Next);
            assert_eq!(p.request(), None);
            let current = p.current();
            assert_eq!(
                p.finish(sent * 2, true, shown),
                Finished::Stale {
                    draw: true,
                    next: current
                }
            );
            shown = Some(sent * 2);
            assert_eq!(p.request(), None);
        }
    }

    #[test]
    fn a_stale_success_draws_and_a_stale_failure_does_not_but_both_send() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        p.go(Move::Next);
        assert_eq!(
            p.finish(0, true, None),
            Finished::Stale {
                draw: true,
                next: 1
            }
        );
        p.go(Move::Next);
        assert_eq!(
            p.finish(1, false, Some(0)),
            Finished::Stale {
                draw: false,
                next: 2
            }
        );
        assert_eq!(p.request(), None);
    }

    #[test]
    fn a_stale_success_does_not_draw_over_the_current_files_image() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        assert_eq!(p.finish(0, true, None), Finished::Current);
        p.go(Move::Next);
        assert_eq!(p.request(), Some(1));
        p.go(Move::Previous);
        assert_eq!(
            p.finish(1, true, Some(0)),
            Finished::Stale {
                draw: false,
                next: 0
            }
        );
    }

    #[test]
    fn a_current_result_clears_the_in_flight_marker_success_or_failure() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        assert_eq!(p.finish(0, true, None), Finished::Current);
        p.go(Move::Next);
        assert_eq!(p.request(), Some(1));
        assert_eq!(p.finish(1, false, Some(0)), Finished::Current);
        p.go(Move::Next);
        assert_eq!(p.request(), Some(2));
    }

    #[test]
    fn a_ring_hit_drops_the_older_result_and_leaves_nothing_in_flight() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        p.go(Move::Next);
        p.hit();
        assert!(p.in_flight());
        assert_eq!(p.finish(0, true, Some(1)), Finished::Dropped);
        assert!(!p.in_flight());
        p.go(Move::Next);
        assert_eq!(p.request(), Some(2));
    }

    #[test]
    fn a_hit_on_the_file_whose_own_read_is_out_drops_that_read() {
        let mut p = Pager::new(5);
        p.go(Move::Next);
        assert_eq!(p.request(), Some(1));
        p.go(Move::Next);
        p.go(Move::Previous);
        p.hit();
        assert_eq!(p.finish(1, true, Some(1)), Finished::Dropped);
        assert!(!p.in_flight());
    }

    #[test]
    fn a_move_after_a_hit_follows_the_older_result_with_the_current_file() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        p.go(Move::Next);
        p.hit();
        p.go(Move::Next);
        assert_eq!(p.request(), None);
        assert_eq!(
            p.finish(0, true, Some(1)),
            Finished::Stale {
                draw: true,
                next: 2
            }
        );
        assert!(p.in_flight());
    }

    #[test]
    fn a_stale_frame_of_the_current_file_is_not_a_hit_and_still_reloads() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        assert_eq!(p.finish(0, true, None), Finished::Current);
        p.go(Move::Next);
        assert_eq!(p.request(), Some(1));
        p.go(Move::Next);
        assert_eq!(
            p.finish(1, true, Some(0)),
            Finished::Stale {
                draw: true,
                next: 2
            }
        );
        p.go(Move::Previous);
        assert_eq!(
            p.finish(2, true, Some(1)),
            Finished::Stale {
                draw: false,
                next: 1
            }
        );
    }

    #[test]
    fn after_a_hold_ends_the_next_request_goes_out() {
        let mut p = Pager::new(5);
        assert_eq!(p.request(), Some(0));
        p.go(Move::Next);
        assert_eq!(p.request(), None);
        p.go(Move::Next);
        assert_eq!(
            p.finish(0, true, None),
            Finished::Stale {
                draw: true,
                next: 2
            }
        );
        assert_eq!(p.request(), None);
        assert_eq!(p.finish(2, true, Some(0)), Finished::Current);
        p.go(Move::Next);
        assert_eq!(p.request(), Some(3));
    }
}
