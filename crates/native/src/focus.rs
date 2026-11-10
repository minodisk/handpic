use std::time::Duration;

/// How long the current file and the strip's range must settle before the
/// scan is told, so a held arrow key or a wheel scroll sends one list per
/// stop rather than one per step.
pub const SCAN_FOCUS_DELAY: Duration = Duration::from_millis(100);

/// The paths the running scan takes first, so it produces the rows of the
/// files the user is looking at before the rest of the folder: the current
/// file first, then the files of the strip's range `[first, last]` nearest to
/// it first, the earlier side before the later at the same distance. The
/// range caps the list; the current file leads it even when it is outside. A
/// port of the Tauri frontend's `scanFocusPaths`.
pub fn scan_focus_paths<T: Clone>(
    files: &[T],
    current: usize,
    first: usize,
    last: usize,
) -> Vec<T> {
    let Some(head) = files.get(current) else {
        return Vec::new();
    };
    let to = last.min(files.len() - 1);
    let mut paths = vec![head.clone()];
    let reach = current
        .saturating_sub(first)
        .max(to.saturating_sub(current));
    for distance in 1..=reach {
        let earlier = current.checked_sub(distance);
        let later = Some(current + distance);
        for at in [earlier, later].into_iter().flatten() {
            if at >= first && at <= to {
                paths.push(files[at].clone());
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::scan_focus_paths;
    use std::collections::HashSet;

    const FILES: [&str; 7] = ["a", "b", "c", "d", "e", "f", "g"];

    #[test]
    fn puts_the_current_file_first_then_the_range_outward_earlier_side_first() {
        assert_eq!(scan_focus_paths(&FILES, 3, 1, 5), ["d", "c", "e", "b", "f"]);
    }

    #[test]
    fn keeps_going_on_the_one_side_left_once_the_other_runs_out() {
        assert_eq!(
            scan_focus_paths(&FILES, 1, 0, 5),
            ["b", "a", "c", "d", "e", "f"]
        );
    }

    #[test]
    fn is_capped_at_the_range() {
        assert_eq!(scan_focus_paths(&FILES, 3, 2, 4), ["d", "c", "e"]);
    }

    #[test]
    fn clamps_a_range_that_runs_past_the_end_of_the_strip() {
        assert_eq!(scan_focus_paths(&FILES, 0, 0, 2), ["a", "b", "c"]);
        assert_eq!(scan_focus_paths(&FILES, 6, 4, 10), ["g", "f", "e"]);
    }

    #[test]
    fn leads_with_a_current_file_outside_the_range_then_the_range_nearest_to_it() {
        assert_eq!(scan_focus_paths(&FILES, 0, 3, 5), ["a", "d", "e", "f"]);
        assert_eq!(scan_focus_paths(&FILES, 6, 1, 2), ["g", "c", "b"]);
    }

    #[test]
    fn has_no_duplicates() {
        let paths = scan_focus_paths(&FILES, 2, 0, 6);
        assert_eq!(paths.iter().collect::<HashSet<_>>().len(), paths.len());
        assert_eq!(paths.len(), FILES.len());
    }

    #[test]
    fn is_empty_when_there_is_no_current_file() {
        assert!(scan_focus_paths::<&str>(&[], 0, 0, 4).is_empty());
        assert!(scan_focus_paths(&FILES, 7, 0, 6).is_empty());
    }
}
