//! The window's one progress indicator, the status row's circle: how full
//! it is, a port of `progress.ts`'s `progressWidth`, and which running
//! operation it shows.

/// The indicator's share of the track: 0 while nothing is counted, never
/// above 1.
pub fn fraction(done: usize, total: usize) -> f32 {
    if total == 0 {
        return 0.;
    }
    (done as f32 / total as f32).min(1.)
}

/// The `(done, total)` the indicator shows, `None` hiding it: a Move to Trash or
/// Delete Permanently run (or its redo) first, then the undo of one, then a
/// folder operation's run, then the scan's running pass.
pub fn shown(
    trash: Option<(usize, usize)>,
    undo: Option<(usize, usize)>,
    folder_op: Option<(usize, usize)>,
    scan: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    trash.or(undo).or(folder_op).or(scan)
}

#[cfg(test)]
mod tests {
    use super::{fraction, shown};

    #[test]
    fn a_zero_total_is_an_empty_bar() {
        assert_eq!(fraction(0, 0), 0.);
        assert_eq!(fraction(3, 0), 0.);
    }

    #[test]
    fn the_fraction_is_done_over_total_capped_at_one() {
        assert_eq!(fraction(0, 4), 0.);
        assert_eq!(fraction(1, 4), 0.25);
        assert_eq!(fraction(4, 4), 1.);
        assert_eq!(fraction(5, 4), 1.);
    }

    #[test]
    fn nothing_running_hides_the_bar() {
        assert_eq!(shown(None, None, None, None), None);
    }

    #[test]
    fn each_operation_alone_is_shown() {
        assert_eq!(shown(Some((1, 2)), None, None, None), Some((1, 2)));
        assert_eq!(shown(None, Some((7, 8)), None, None), Some((7, 8)));
        assert_eq!(shown(None, None, Some((3, 4)), None), Some((3, 4)));
        assert_eq!(shown(None, None, None, Some((5, 6))), Some((5, 6)));
    }

    #[test]
    fn the_trash_run_comes_first_then_a_folder_operation_then_the_scan() {
        let (trash, folder_op, scan) = (Some((1, 2)), Some((3, 4)), Some((5, 6)));
        assert_eq!(shown(trash, None, folder_op, scan), trash);
        assert_eq!(shown(trash, None, None, scan), trash);
        assert_eq!(shown(trash, None, folder_op, None), trash);
        assert_eq!(shown(None, None, folder_op, scan), folder_op);
    }

    #[test]
    fn an_undo_comes_after_a_trash_run_and_before_a_folder_operation_and_the_scan() {
        let (trash, undo, folder_op, scan) =
            (Some((1, 2)), Some((7, 8)), Some((3, 4)), Some((5, 6)));
        assert_eq!(shown(trash, undo, folder_op, scan), trash);
        assert_eq!(shown(None, undo, folder_op, scan), undo);
        assert_eq!(shown(None, undo, None, scan), undo);
    }
}
