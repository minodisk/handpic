use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
};

/// The distance from one cell's left edge to the next's, the Tauri strip's
/// `--cell-width`.
pub const CELL_PITCH: f32 = 154.;
/// A cell: the image box plus a 1 px border each side; the rest of the pitch
/// is the gap.
pub const CELL_WIDTH: f32 = BOX_WIDTH + 2.;
pub const BOX_WIDTH: f32 = 144.;
pub const BOX_HEIGHT: f32 = 96.;
pub const NAME_HEIGHT: f32 = 16.;
pub const CELL_HEIGHT: f32 = BOX_HEIGHT + NAME_HEIGHT + 2.;
/// Above and below the cells.
pub const STRIP_PADDING: f32 = 4.;
pub const STRIP_HEIGHT: f32 = CELL_HEIGHT + 2. * STRIP_PADDING;
/// Cells kept beyond the visible range, so a short scroll shows a thumbnail
/// that is already decoded.
pub const RANGE_MARGIN: usize = 4;

/// What the index answered for a cell's thumbnail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Thumbnail,
    /// The index has no row for the file yet: the scan has not reached it.
    NotScanned,
    /// The scan processed the file and it failed (`no cached thumbnail`).
    Failed,
}

/// The thumbnail strip's scroll position and which cells to ask the index
/// for, as `strip.ts` keeps them. Everything is keyed by the index in the
/// shown list; `width` is the strip's on-screen width.
#[derive(Default)]
pub struct Strip {
    len: usize,
    index_of: HashMap<String, usize>,
    offset: f32,
    /// Asked for and either still in flight or answered with a thumbnail, so
    /// a cell scrolling back into view does not ask again.
    requested: HashSet<usize>,
    /// Asked for and not answered yet; unlike `requested`, leaving the range
    /// does not clear it, so a cell coming back does not ask twice.
    in_flight: HashSet<usize>,
    /// Not scanned yet; asked again once the scan reports the path ready or
    /// ends.
    missing: HashSet<usize>,
    /// Reported committed by the scan. A request in flight at that moment may
    /// come back from a query that ran before the commit, and must not be
    /// left `missing` until the scan ends.
    ready: HashSet<usize>,
    /// Processed and failed: shown once, never retried.
    failed: HashSet<usize>,
}

impl Strip {
    pub fn new(paths: &[String]) -> Self {
        Self {
            len: paths.len(),
            index_of: paths
                .iter()
                .enumerate()
                .map(|(index, path)| (path.clone(), index))
                .collect(),
            ..Self::default()
        }
    }

    pub fn offset(&self) -> f32 {
        self.offset
    }

    pub fn is_failed(&self, index: usize) -> bool {
        self.failed.contains(&index)
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    fn max_offset(&self, width: f32) -> f32 {
        (self.len as f32 * CELL_PITCH - width).max(0.)
    }

    /// Scroll by `delta` px, clamped to the content. Whether it moved.
    pub fn scroll_by(&mut self, delta: f32, width: f32) -> bool {
        self.scroll_to(self.offset + delta, width)
    }

    /// Re-clamp the offset after the strip's width changed. Whether it moved.
    pub fn clamp(&mut self, width: f32) -> bool {
        self.scroll_to(self.offset, width)
    }

    pub fn scroll_to(&mut self, offset: f32, width: f32) -> bool {
        let offset = offset.clamp(0., self.max_offset(width));
        let moved = offset != self.offset;
        self.offset = offset;
        moved
    }

    /// Scroll the least that shows the whole cell, the `block: nearest` way.
    /// Whether it moved.
    pub fn scroll_into_view(&mut self, index: usize, width: f32) -> bool {
        let left = index as f32 * CELL_PITCH;
        let right = left + CELL_PITCH;
        if left < self.offset {
            self.scroll_to(left, width)
        } else if right > self.offset + width {
            self.scroll_to(right - width, width)
        } else {
            self.scroll_to(self.offset, width)
        }
    }

    /// The cells at least partly on screen.
    pub fn on_screen(&self, width: f32) -> RangeInclusive<usize> {
        self.range(width, 0)
    }

    /// The cells the strip keeps: the visible ones plus `RANGE_MARGIN` each
    /// side. Empty without files.
    pub fn visible_range(&self, width: f32) -> RangeInclusive<usize> {
        self.range(width, RANGE_MARGIN)
    }

    fn range(&self, width: f32, margin: usize) -> RangeInclusive<usize> {
        if self.len == 0 {
            #[allow(clippy::reversed_empty_ranges)]
            return 1..=0;
        }
        let first = (self.offset / CELL_PITCH).floor() as usize;
        let last = ((self.offset + width) / CELL_PITCH).ceil() as usize;
        first.saturating_sub(margin)..=(last + margin).min(self.len - 1)
    }

    /// The cell under `x`, measured from the strip's left edge; `None` in a
    /// gap or past the last cell.
    pub fn hit_test(&self, x: f32) -> Option<usize> {
        let at = self.offset + x;
        if at < 0. {
            return None;
        }
        let index = (at / CELL_PITCH).floor() as usize;
        (index < self.len && at - index as f32 * CELL_PITCH < CELL_WIDTH).then_some(index)
    }

    /// The cell in the range to ask for next, nearest the viewport's center
    /// first, so a fast scroll fills what is looked at rather than what it
    /// flew past.
    pub fn next_to_request(&self, width: f32) -> Option<usize> {
        let center = (self.offset + width / 2.) / CELL_PITCH;
        self.visible_range(width)
            .filter(|index| {
                !self.requested.contains(index)
                    && !self.in_flight.contains(index)
                    && !self.missing.contains(index)
                    && !self.failed.contains(index)
            })
            .min_by(|a, b| {
                (*a as f32 - center)
                    .abs()
                    .total_cmp(&(*b as f32 - center).abs())
            })
    }

    /// Cells whose thumbnails were carried over from the previous listing,
    /// so they are not asked for again.
    pub fn keep(&mut self, indices: impl IntoIterator<Item = usize>) {
        self.requested.extend(indices);
    }

    pub fn request(&mut self, index: usize) {
        self.requested.insert(index);
        self.in_flight.insert(index);
    }

    /// Record the index's answer for `index`. Whether to keep its thumbnail:
    /// false when the cell left the range while the read was out.
    pub fn answer(&mut self, index: usize, answer: Answer) -> bool {
        self.in_flight.remove(&index);
        match answer {
            Answer::Thumbnail => return self.requested.contains(&index),
            Answer::NotScanned => {
                self.requested.remove(&index);
                if !self.ready.remove(&index) {
                    self.missing.insert(index);
                }
            }
            Answer::Failed => {
                self.requested.remove(&index);
                self.failed.insert(index);
            }
        }
        false
    }

    /// Forget the cells outside the range, whose thumbnails are dropped and
    /// asked for again when they return. The cells forgotten.
    pub fn release_outside(&mut self, width: f32) -> Vec<usize> {
        let range = self.visible_range(width);
        let released: Vec<usize> = self
            .requested
            .iter()
            .copied()
            .filter(|index| !range.contains(index))
            .collect();
        for index in &released {
            self.requested.remove(index);
        }
        released
    }

    /// The scan has committed these paths, so the index can answer for them.
    /// Paths not shown are ignored.
    pub fn mark_ready(&mut self, paths: &[String]) {
        for path in paths {
            if let Some(&index) = self.index_of.get(path) {
                self.ready.insert(index);
                self.missing.remove(&index);
            }
        }
    }

    /// The scan is over: ask again for the cells it had nothing for.
    pub fn refresh(&mut self) {
        self.missing.clear();
    }
}

/// The strip's offset after `previous` is re-listed as `next`, the Tauri
/// strip's `carriedOffset`: the anchor is the current cell when it overlaps
/// the viewport, else the first cell that does; the view moves by the
/// anchor's index delta, or stays put when the anchor is gone, clamped to the
/// new content either way.
pub fn carried_offset(
    previous: &[String],
    next: &[String],
    offset: f32,
    width: f32,
    current: usize,
) -> f32 {
    let overlaps = |at: usize| {
        (at as f32 * CELL_PITCH) < offset + width && (at + 1) as f32 * CELL_PITCH > offset
    };
    let first = (offset / CELL_PITCH).floor() as usize;
    let anchor = if current < previous.len() && overlaps(current) {
        current
    } else {
        first
    };
    let mut carried = offset;
    if anchor < previous.len()
        && overlaps(anchor)
        && let Some(at) = next.iter().position(|p| *p == previous[anchor])
    {
        carried = offset + (at as f32 - anchor as f32) * CELL_PITCH;
    }
    carried.min(next.len() as f32 * CELL_PITCH - width).max(0.)
}

/// Old index to new index of each of `indices` whose path is still in
/// `next`, the Tauri strip's `carriedIndices`.
pub fn carried_indices(
    previous: &[String],
    next: &[String],
    indices: impl IntoIterator<Item = usize>,
) -> HashMap<usize, usize> {
    let index_of: HashMap<&String, usize> =
        next.iter().enumerate().map(|(at, p)| (p, at)).collect();
    indices
        .into_iter()
        .filter_map(|at| Some((at, *index_of.get(previous.get(at)?)?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Answer, CELL_PITCH, Strip, carried_indices, carried_offset};

    fn strip(len: usize) -> Strip {
        Strip::new(
            &(0..len)
                .map(|i| format!("D:/a/{i}.ARW"))
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn the_offset_stays_within_the_content() {
        let mut s = strip(10);
        assert!(!s.scroll_by(-50., 500.));
        assert!(s.scroll_by(5000., 500.));
        assert_eq!(s.offset(), 10. * CELL_PITCH - 500.);
        let mut short = strip(2);
        assert!(!short.scroll_by(100., 500.));
        assert_eq!(short.offset(), 0.);
    }

    #[test]
    fn a_wider_strip_pulls_the_offset_back() {
        let mut s = strip(10);
        s.scroll_by(1e6, 500.);
        assert!(s.clamp(800.));
        assert_eq!(s.offset(), 10. * CELL_PITCH - 800.);
        assert!(!s.clamp(800.));
        assert!(s.clamp(5000.));
        assert_eq!(s.offset(), 0.);
    }

    #[test]
    fn the_range_spans_the_visible_cells_plus_the_margin() {
        let mut s = strip(100);
        assert_eq!(s.visible_range(500.), 0..=8);
        assert_eq!(s.on_screen(500.), 0..=4);
        s.scroll_by(20. * CELL_PITCH + 10., 500.);
        assert_eq!(s.visible_range(500.), 16..=28);
        s.scroll_by(1e6, 500.);
        assert_eq!(*s.visible_range(500.).end(), 99);
        assert!(strip(0).visible_range(500.).is_empty());
    }

    #[test]
    fn scrolling_into_view_moves_the_least() {
        let mut s = strip(100);
        assert!(!s.scroll_into_view(2, 500.));
        assert!(s.scroll_into_view(10, 500.));
        assert_eq!(s.offset(), 11. * CELL_PITCH - 500.);
        assert!(!s.scroll_into_view(9, 500.));
        assert!(s.scroll_into_view(3, 500.));
        assert_eq!(s.offset(), 3. * CELL_PITCH);
    }

    #[test]
    fn a_hit_names_the_cell_and_misses_the_gap() {
        let mut s = strip(3);
        assert_eq!(s.hit_test(10.), Some(0));
        assert_eq!(s.hit_test(CELL_PITCH - 2.), None);
        assert_eq!(s.hit_test(CELL_PITCH + 1.), Some(1));
        assert_eq!(s.hit_test(3. * CELL_PITCH + 1.), None);
        s.scroll_by(CELL_PITCH, 100.);
        assert_eq!(s.hit_test(10.), Some(1));
    }

    #[test]
    fn the_cell_nearest_the_center_is_asked_for_first() {
        let mut s = strip(100);
        s.scroll_by(20. * CELL_PITCH, 3. * CELL_PITCH);
        assert_eq!(s.next_to_request(3. * CELL_PITCH), Some(21));
        s.request(21);
        assert_eq!(s.next_to_request(3. * CELL_PITCH), Some(22));
    }

    #[test]
    fn requested_in_flight_missing_and_failed_cells_are_skipped() {
        let mut s = strip(3);
        for (index, answer) in [(0, Some(Answer::NotScanned)), (1, Some(Answer::Failed))] {
            s.request(index);
            if let Some(answer) = answer {
                s.answer(index, answer);
            }
        }
        assert_eq!(s.next_to_request(1000.), Some(2));
        s.request(2);
        assert_eq!(s.next_to_request(1000.), None);
        assert_eq!(s.in_flight(), 1);
        assert!(s.is_failed(1));
    }

    #[test]
    fn a_ready_path_clears_missing() {
        let mut s = strip(3);
        s.request(0);
        s.answer(0, Answer::NotScanned);
        s.request(1);
        s.request(2);
        assert_eq!(s.next_to_request(1000.), None);
        s.mark_ready(&["D:/a/0.ARW".into(), "D:/b/9.ARW".into()]);
        assert_eq!(s.next_to_request(1000.), Some(0));
    }

    #[test]
    fn a_not_scanned_answer_after_ready_is_asked_again() {
        let mut s = strip(1);
        s.request(0);
        s.mark_ready(&["D:/a/0.ARW".into()]);
        s.answer(0, Answer::NotScanned);
        assert_eq!(s.next_to_request(1000.), Some(0));
    }

    #[test]
    fn the_scan_end_asks_again_for_missing_but_not_failed() {
        let mut s = strip(2);
        s.request(0);
        s.answer(0, Answer::NotScanned);
        s.request(1);
        s.answer(1, Answer::Failed);
        assert_eq!(s.next_to_request(1000.), None);
        s.refresh();
        assert_eq!(s.next_to_request(1000.), Some(0));
        s.request(0);
        assert_eq!(s.next_to_request(1000.), None);
    }

    #[test]
    fn released_cells_are_asked_again_and_late_answers_dropped() {
        let mut s = strip(100);
        let width = 3. * CELL_PITCH;
        s.request(0);
        assert!(s.answer(0, Answer::Thumbnail));
        s.request(1);
        s.scroll_by(50. * CELL_PITCH, width);
        let mut released = s.release_outside(width);
        released.sort_unstable();
        assert_eq!(released, [0, 1]);
        assert!(!s.answer(1, Answer::Thumbnail));
        s.scroll_by(-50. * CELL_PITCH, width);
        assert_eq!(s.next_to_request(width), Some(1));
        s.request(1);
        s.request(2);
        assert_eq!(s.next_to_request(width), Some(0));
        assert!(s.release_outside(width).is_empty());
    }

    #[test]
    fn kept_cells_are_not_asked_again() {
        let mut s = strip(2);
        s.keep([0]);
        assert_eq!(s.next_to_request(1000.), Some(1));
    }

    fn names(prefix: &str, count: usize) -> Vec<String> {
        (0..count).map(|at| format!("/{prefix}{at}.ARW")).collect()
    }

    const P: f32 = CELL_PITCH;

    #[test]
    fn files_inserted_before_a_visible_current_cell_keep_it_in_place() {
        let previous = names("a", 20);
        let next = [names("n", 3), previous.clone()].concat();
        assert_eq!(carried_offset(&previous, &next, 2. * P, 5. * P, 4), 5. * P);
    }

    #[test]
    fn with_the_current_cell_off_screen_the_first_visible_one_anchors() {
        let previous = names("a", 25);
        let next = [names("n", 100), previous.clone()].concat();
        assert_eq!(
            carried_offset(&previous, &next, 20. * P, 5. * P, 0),
            120. * P
        );
    }

    #[test]
    fn a_deleted_anchor_keeps_the_offset() {
        let previous = names("a", 20);
        let mut next = previous.clone();
        next.remove(5);
        assert_eq!(carried_offset(&previous, &next, 5. * P, 5. * P, 0), 5. * P);
    }

    #[test]
    fn the_carried_offset_is_clamped_to_the_new_content() {
        let previous = names("a", 20);
        let next = previous[..8].to_vec();
        assert_eq!(carried_offset(&previous, &next, 15. * P, 5. * P, 0), 3. * P);
        assert_eq!(carried_offset(&previous, &[], 15. * P, 5. * P, 0), 0.);
    }

    #[test]
    fn carried_indices_follow_their_files() {
        let list = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let (a, b, c) = (
            list(&["/a", "/b", "/c"]),
            list(&["/c", "/a", "/b"]),
            list(&["/a", "/c"]),
        );
        assert_eq!(
            carried_indices(&a, &b, [0, 1, 2]),
            HashMap::from([(0, 1), (1, 2), (2, 0)])
        );
        assert_eq!(
            carried_indices(&a, &c, [0, 1, 2]),
            HashMap::from([(0, 0), (2, 1)])
        );
        assert_eq!(
            carried_indices(&a, &list(&["/x", "/a", "/b"]), [0, 1]),
            HashMap::from([(0, 1), (1, 2)])
        );
        assert_eq!(carried_indices(&a, &a, [1]), HashMap::from([(1, 1)]));
    }
}
