//! The window's one progress bar: how full it is, a port of `progress.ts`'s
//! `progressWidth`, and which running operation it shows.

/// The indicator's share of the track: 0 while nothing is counted, never
/// above 1.
pub fn fraction(done: usize, total: usize) -> f32 {
    if total == 0 {
        return 0.;
    }
    (done as f32 / total as f32).min(1.)
}

/// The `(done, total)` the bar shows, `None` hiding it: a Move to Trash or
/// Delete Permanently run first, then a folder operation's run, then the
/// scan's running pass.
pub fn shown(
    trash: Option<(usize, usize)>,
    folder_op: Option<(usize, usize)>,
    scan: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    trash.or(folder_op).or(scan)
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
        assert_eq!(shown(None, None, None), None);
    }

    #[test]
    fn each_operation_alone_is_shown() {
        assert_eq!(shown(Some((1, 2)), None, None), Some((1, 2)));
        assert_eq!(shown(None, Some((3, 4)), None), Some((3, 4)));
        assert_eq!(shown(None, None, Some((5, 6))), Some((5, 6)));
    }

    #[test]
    fn the_trash_run_comes_first_then_a_folder_operation_then_the_scan() {
        let (trash, folder_op, scan) = (Some((1, 2)), Some((3, 4)), Some((5, 6)));
        assert_eq!(shown(trash, folder_op, scan), trash);
        assert_eq!(shown(trash, None, scan), trash);
        assert_eq!(shown(trash, folder_op, None), trash);
        assert_eq!(shown(None, folder_op, scan), folder_op);
    }
}
