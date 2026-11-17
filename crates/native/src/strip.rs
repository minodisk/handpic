use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
};

use handpic_core::Flag;

use crate::{burst::BurstMark, judgment::State, stripheight};

pub const NAME_HEIGHT: f32 = 16.;
/// Above and below the cells, room for the burst band.
pub const STRIP_PADDING: f32 = 7.;
/// Between two cells.
const CELL_GAP: f32 = 8.;

/// The strip's cells as its height sizes them: the image box at 3:2 takes
/// what the padding, the name band and the box's 1 px border each side leave
/// of the height. At `stripheight::DEFAULT` they are the Tauri strip's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub height: f32,
    pub box_width: f32,
    pub box_height: f32,
    /// The image box plus a 1 px border each side.
    pub cell_width: f32,
    pub cell_height: f32,
    /// The distance from one cell's left edge to the next's, the Tauri
    /// strip's `--cell-width`; the rest of it after the cell is the gap.
    pub pitch: f32,
}

impl Geometry {
    pub fn of(height: f32) -> Self {
        let box_height = height - 2. * STRIP_PADDING - NAME_HEIGHT - 2.;
        let box_width = (box_height * 1.5).round();
        let cell_width = box_width + 2.;
        Self {
            height,
            box_width,
            box_height,
            cell_width,
            cell_height: box_height + NAME_HEIGHT + 2.,
            pitch: cell_width + CELL_GAP,
        }
    }
}

impl Default for Geometry {
    fn default() -> Self {
        Self::of(stripheight::DEFAULT)
    }
}

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
    geometry: Geometry,
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
    /// The judged cells' stars, flag and label.
    judgments: HashMap<usize, State>,
    /// The selected cells, as `strip.ts`'s `selected`; painted unless current.
    selected: HashSet<usize>,
    /// Each cell's part of a burst's band and its badge, `None` outside a
    /// burst of two or more.
    bursts: Vec<Option<BurstMark>>,
}

/// The color a label tints the cell's name band with: one of the seven
/// Lightroom colors, any other name a neutral gray, as `strip.ts`'s
/// `paintRating`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Pink,
    Purple,
    Other,
}

impl Tint {
    pub fn of(label: &str) -> Self {
        match label.to_lowercase().as_str() {
            "red" => Self::Red,
            "orange" => Self::Orange,
            "yellow" => Self::Yellow,
            "green" => Self::Green,
            "blue" => Self::Blue,
            "pink" => Self::Pink,
            "purple" => Self::Purple,
            _ => Self::Other,
        }
    }
}

/// What a cell draws of its judgment: the stars at the box's top-right, the
/// pick or reject dot, the rejected cell dimmed, the label's name band.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Marks {
    pub stars: u8,
    pub flag: Flag,
    pub label: Option<Tint>,
}

impl Marks {
    pub fn dimmed(&self) -> bool {
        self.flag == Flag::Reject
    }
}

impl Strip {
    pub fn new(paths: &[String], geometry: Geometry) -> Self {
        Self {
            geometry,
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

    pub fn geometry(&self) -> Geometry {
        self.geometry
    }

    /// Size the cells for another height, keeping the same cells in view:
    /// the `current` cell's center stays where it is on screen when that
    /// cell overlaps the viewport, else the cell at the viewport's left edge
    /// stays there, then the offset is clamped to the new content. Whether
    /// the offset moved.
    pub fn set_geometry(&mut self, geometry: Geometry, width: f32, current: usize) -> bool {
        let old = self.geometry;
        let left = current as f32 * old.pitch;
        let offset = if current < self.len
            && left < self.offset + width
            && left + old.cell_width > self.offset
        {
            let at = left + old.cell_width / 2. - self.offset;
            current as f32 * geometry.pitch + geometry.cell_width / 2. - at
        } else {
            self.offset / old.pitch * geometry.pitch
        };
        self.geometry = geometry;
        self.scroll_to(offset, width)
    }

    pub fn is_failed(&self, index: usize) -> bool {
        self.failed.contains(&index)
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// The width of all the cells side by side.
    pub fn content_width(&self) -> f32 {
        self.len as f32 * self.geometry.pitch
    }

    fn max_offset(&self, width: f32) -> f32 {
        (self.content_width() - width).max(0.)
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
        let left = index as f32 * self.geometry.pitch;
        let right = left + self.geometry.pitch;
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
        let pitch = self.geometry.pitch;
        let first = (self.offset / pitch).floor() as usize;
        let last = ((self.offset + width) / pitch).ceil() as usize;
        first.saturating_sub(margin)..=(last + margin).min(self.len - 1)
    }

    /// The cell under `x`, measured from the strip's left edge; `None` in a
    /// gap or past the last cell.
    pub fn hit_test(&self, x: f32) -> Option<usize> {
        let at = self.offset + x;
        if at < 0. {
            return None;
        }
        let Geometry {
            pitch, cell_width, ..
        } = self.geometry;
        let index = (at / pitch).floor() as usize;
        (index < self.len && at - index as f32 * pitch < cell_width).then_some(index)
    }

    /// The cell in the range to ask for next, nearest the viewport's center
    /// first, so a fast scroll fills what is looked at rather than what it
    /// flew past.
    pub fn next_to_request(&self, width: f32) -> Option<usize> {
        let center = (self.offset + width / 2.) / self.geometry.pitch;
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

    /// Record the judgment of a cell; `None` is none at all.
    pub fn set_judgment(&mut self, index: usize, state: Option<State>) {
        match state {
            Some(state) => self.judgments.insert(index, state),
            None => self.judgments.remove(&index),
        };
    }

    /// Mark the cells of the selected `paths`; a path not listed is ignored.
    pub fn set_selected<'a>(&mut self, paths: impl IntoIterator<Item = &'a String>) {
        self.selected = paths
            .into_iter()
            .filter_map(|path| self.index_of.get(path).copied())
            .collect();
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selected.contains(&index)
    }

    /// Record every cell's burst mark, in cell order.
    pub fn set_bursts(&mut self, marks: Vec<Option<BurstMark>>) {
        self.bursts = marks;
    }

    pub fn burst(&self, index: usize) -> Option<BurstMark> {
        self.bursts.get(index).copied().flatten()
    }

    pub fn marks(&self, index: usize) -> Marks {
        self.judgments
            .get(&index)
            .map_or_else(Marks::default, |state| Marks {
                stars: state.rating.unwrap_or(0),
                flag: state.flag,
                label: state.label.as_deref().map(Tint::of),
            })
    }
}

/// Which of a cell's hover texts a tooltip region shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tip {
    Failure,
    Label,
}

/// A part of a cell that shows a hover text, from the cell's origin.
#[derive(Clone, Debug, PartialEq)]
pub struct TipRegion {
    pub tip: Tip,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub text: String,
}

/// A cell's hover texts, `strip.ts`'s `cell.el.title` and `cell.name.title`:
/// the label's name over the name band, the scan's error over the rest of
/// the cell (all of it without a label). The regions never overlap, so no
/// two tooltips are asked for at once; an empty text gives none, as an
/// empty `title` shows nothing.
pub fn tip_regions(
    geometry: Geometry,
    failure: Option<&str>,
    label: Option<&str>,
) -> Vec<TipRegion> {
    let label = label.filter(|label| !label.is_empty());
    let band_top = 1. + geometry.box_height;
    let mut regions = Vec::new();
    if let Some(failure) = failure.filter(|failure| !failure.is_empty()) {
        regions.push(TipRegion {
            tip: Tip::Failure,
            x: 0.,
            y: 0.,
            width: geometry.cell_width,
            height: if label.is_some() {
                band_top
            } else {
                geometry.cell_height
            },
            text: failure.to_owned(),
        });
    }
    if let Some(label) = label {
        regions.push(TipRegion {
            tip: Tip::Label,
            x: 0.,
            y: band_top,
            width: geometry.cell_width,
            height: NAME_HEIGHT,
            text: label.to_owned(),
        });
    }
    regions
}

/// The strip's offset after `previous` is re-listed as `next`, the Tauri
/// strip's `carriedOffset`: the anchor is the current cell when it overlaps
/// the viewport, else the first cell that does; the view moves by the
/// anchor's index delta, or stays put when the anchor is gone, clamped to the
/// new content either way. `pitch` is the cells'.
pub fn carried_offset(
    previous: &[String],
    next: &[String],
    offset: f32,
    width: f32,
    current: usize,
    pitch: f32,
) -> f32 {
    let overlaps =
        |at: usize| (at as f32 * pitch) < offset + width && (at + 1) as f32 * pitch > offset;
    let first = (offset / pitch).floor() as usize;
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
        carried = offset + (at as f32 - anchor as f32) * pitch;
    }
    carried.min(next.len() as f32 * pitch - width).max(0.)
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

    use handpic_core::Flag;

    use super::{
        Answer, Geometry, Marks, NAME_HEIGHT, Strip, Tint, Tip, TipRegion, carried_indices,
        carried_offset, tip_regions,
    };
    use crate::{burst::BurstMark, judgment::State, stripheight};

    /// The default geometry's numbers, the Tauri strip's.
    const CELL_PITCH: f32 = 154.;
    const CELL_WIDTH: f32 = 146.;
    const CELL_HEIGHT: f32 = 114.;
    const BOX_HEIGHT: f32 = 96.;

    fn paths(len: usize) -> Vec<String> {
        (0..len).map(|i| format!("D:/a/{i}.ARW")).collect()
    }

    fn strip(len: usize) -> Strip {
        Strip::new(&paths(len), Geometry::default())
    }

    /// The geometry of a 176 px strip: a 216 x 144 image box.
    fn tall() -> Geometry {
        Geometry::of(176.)
    }

    #[test]
    fn the_default_geometry_is_the_tauri_strips() {
        assert_eq!(
            Geometry::default(),
            Geometry {
                height: 128.,
                box_width: 144.,
                box_height: BOX_HEIGHT,
                cell_width: CELL_WIDTH,
                cell_height: CELL_HEIGHT,
                pitch: CELL_PITCH,
            }
        );
        assert_eq!(Geometry::of(stripheight::DEFAULT), Geometry::default());
    }

    #[test]
    fn the_cells_are_sized_from_the_height() {
        assert_eq!(
            tall(),
            Geometry {
                height: 176.,
                box_width: 216.,
                box_height: 144.,
                cell_width: 218.,
                cell_height: 162.,
                pitch: 226.,
            }
        );
        let min = Geometry::of(stripheight::MIN);
        assert_eq!((min.box_width, min.box_height), (96., 64.));
        let max = Geometry::of(stripheight::MAX);
        assert_eq!((max.box_width, max.box_height), (405., 270.));
        // A box height whose 3:2 width is not whole is rounded.
        assert_eq!(Geometry::of(133.).box_width, 152.);
    }

    #[test]
    fn hit_test_range_and_content_follow_the_geometry() {
        let mut s = Strip::new(&paths(100), tall());
        assert_eq!(s.content_width(), 100. * 226.);
        assert_eq!(s.hit_test(217.), Some(0));
        assert_eq!(s.hit_test(219.), None);
        assert_eq!(s.hit_test(227.), Some(1));
        assert_eq!(s.on_screen(500.), 0..=3);
        assert_eq!(s.visible_range(500.), 0..=7);
        s.scroll_by(20. * 226. + 10., 500.);
        assert_eq!(s.on_screen(500.), 20..=23);
        assert_eq!(s.next_to_request(500.), Some(21));
        assert!(s.scroll_into_view(30, 500.));
        assert_eq!(s.offset(), 31. * 226. - 500.);
    }

    #[test]
    fn a_new_height_keeps_the_current_cell_centered_where_it_was() {
        let mut s = strip(100);
        s.scroll_to(10. * CELL_PITCH, 1000.);
        // Cell 12's center is 2 * 154 + 73 = 381 px into the viewport.
        assert!(s.set_geometry(tall(), 1000., 12));
        assert_eq!(s.offset(), 12. * 226. + 109. - 381.);
        assert_eq!(s.geometry(), tall());
        assert!(s.on_screen(1000.).contains(&12));
        // And back.
        s.set_geometry(Geometry::default(), 1000., 12);
        assert_eq!(s.offset(), 10. * CELL_PITCH);
    }

    #[test]
    fn a_new_height_keeps_the_left_edge_without_the_current_cell_on_screen() {
        let mut s = strip(100);
        s.scroll_to(10.5 * CELL_PITCH, 1000.);
        s.set_geometry(tall(), 1000., 0);
        assert_eq!(s.offset(), 10.5 * 226.);
        // A current index past the list is not on screen either.
        let mut s = strip(100);
        s.scroll_to(10. * CELL_PITCH, 1000.);
        s.set_geometry(tall(), 1000., 100);
        assert_eq!(s.offset(), 10. * 226.);
    }

    #[test]
    fn a_new_height_clamps_the_offset_to_the_new_content() {
        let mut s = strip(10);
        s.scroll_by(1e6, 1000.);
        assert_eq!(s.offset(), 10. * CELL_PITCH - 1000.);
        s.set_geometry(Geometry::of(stripheight::MIN), 1000., 9);
        // Ten 106 px cells overflow 1000 px by 60 px.
        assert_eq!(s.offset(), 60.);
        s.set_geometry(Geometry::of(stripheight::MIN), 2000., 9);
        assert_eq!(s.offset(), 0.);
        let mut empty = strip(0);
        assert!(!empty.set_geometry(tall(), 1000., 0));
        assert_eq!(empty.offset(), 0.);
    }

    #[test]
    fn marks_the_selected_paths_it_lists() {
        let mut s = strip(5);
        let paths = ["D:/a/1.ARW", "D:/a/3.ARW", "D:/b/9.ARW"].map(String::from);
        s.set_selected(&paths);
        let selected: Vec<_> = (0..5).filter(|&i| s.is_selected(i)).collect();
        assert_eq!(selected, [1, 3]);
        s.set_selected(&[]);
        assert!(!s.is_selected(1));
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
        assert_eq!(
            carried_offset(&previous, &next, 2. * P, 5. * P, 4, P),
            5. * P
        );
    }

    #[test]
    fn with_the_current_cell_off_screen_the_first_visible_one_anchors() {
        let previous = names("a", 25);
        let next = [names("n", 100), previous.clone()].concat();
        assert_eq!(
            carried_offset(&previous, &next, 20. * P, 5. * P, 0, P),
            120. * P
        );
    }

    #[test]
    fn a_deleted_anchor_keeps_the_offset() {
        let previous = names("a", 20);
        let mut next = previous.clone();
        next.remove(5);
        assert_eq!(
            carried_offset(&previous, &next, 5. * P, 5. * P, 0, P),
            5. * P
        );
    }

    #[test]
    fn the_carried_offset_is_clamped_to_the_new_content() {
        let previous = names("a", 20);
        let next = previous[..8].to_vec();
        assert_eq!(
            carried_offset(&previous, &next, 15. * P, 5. * P, 0, P),
            3. * P
        );
        assert_eq!(carried_offset(&previous, &[], 15. * P, 5. * P, 0, P), 0.);
    }

    #[test]
    fn the_carried_offset_follows_the_pitch() {
        let tall = tall().pitch;
        let previous = names("a", 20);
        let next = [names("n", 3), previous.clone()].concat();
        assert_eq!(
            carried_offset(&previous, &next, 2. * tall, 5. * tall, 4, tall),
            5. * tall
        );
        let next = previous[..8].to_vec();
        assert_eq!(
            carried_offset(&previous, &next, 15. * tall, 5. * tall, 0, tall),
            3. * tall
        );
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

    #[test]
    fn a_judged_cell_draws_its_stars_flag_and_label() {
        let mut s = strip(3);
        s.set_judgment(
            1,
            Some(State {
                rating: Some(3),
                flag: Flag::Reject,
                label: Some("Red".into()),
            }),
        );
        let marks = s.marks(1);
        assert_eq!(
            marks,
            Marks {
                stars: 3,
                flag: Flag::Reject,
                label: Some(Tint::Red),
            }
        );
        assert!(marks.dimmed());
        assert_eq!(s.marks(0), Marks::default());
        assert!(!s.marks(0).dimmed());
        s.set_judgment(1, None);
        assert_eq!(s.marks(1), Marks::default());
    }

    #[test]
    fn a_cell_carries_its_burst_mark() {
        let mut s = strip(3);
        let mark = BurstMark {
            first: true,
            last: false,
            position: 0,
            size: 2,
        };
        s.set_bursts(vec![Some(mark), None]);
        assert_eq!(s.burst(0), Some(mark));
        assert_eq!(s.burst(1), None);
        assert_eq!(s.burst(2), None);
    }

    #[test]
    fn a_label_outside_the_seven_colors_is_other() {
        assert_eq!(Tint::of("pink"), Tint::Pink);
        assert_eq!(Tint::of("ORANGE"), Tint::Orange);
        assert_eq!(Tint::of("Rot"), Tint::Other);
        assert_eq!(Tint::of(""), Tint::Other);
    }

    fn region(tip: Tip, y: f32, height: f32, text: &str) -> TipRegion {
        TipRegion {
            tip,
            x: 0.,
            y,
            width: CELL_WIDTH,
            height,
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_cell_neither_failed_nor_labeled_has_no_tooltip() {
        assert_eq!(tip_regions(Geometry::default(), None, None), []);
        assert_eq!(tip_regions(Geometry::default(), Some(""), None), []);
        assert_eq!(tip_regions(Geometry::default(), Some(""), Some("")), []);
    }

    #[test]
    fn a_failed_cell_shows_its_error_over_the_whole_cell() {
        assert_eq!(
            tip_regions(Geometry::default(), Some("truncated"), None),
            [region(Tip::Failure, 0., CELL_HEIGHT, "truncated")]
        );
    }

    #[test]
    fn a_labeled_cell_shows_its_label_name_over_the_name_band() {
        let band = region(Tip::Label, 1. + BOX_HEIGHT, NAME_HEIGHT, "Red");
        assert_eq!(
            tip_regions(Geometry::default(), None, Some("Red")),
            std::slice::from_ref(&band)
        );
        assert_eq!(
            tip_regions(Geometry::default(), Some(""), Some("Red")),
            [band]
        );
    }

    #[test]
    fn the_tooltip_regions_follow_the_geometry() {
        let tall = tall();
        assert_eq!(
            tip_regions(tall, Some("truncated"), Some("Red")),
            [
                TipRegion {
                    width: 218.,
                    ..region(Tip::Failure, 0., 145., "truncated")
                },
                TipRegion {
                    width: 218.,
                    ..region(Tip::Label, 145., NAME_HEIGHT, "Red")
                },
            ]
        );
        assert_eq!(
            tip_regions(tall, Some("truncated"), None)[0].height,
            tall.cell_height
        );
    }

    #[test]
    fn a_failed_labeled_cell_splits_at_the_name_band() {
        let regions = tip_regions(Geometry::default(), Some("truncated"), Some("Client pick"));
        assert_eq!(
            regions,
            [
                region(Tip::Failure, 0., 1. + BOX_HEIGHT, "truncated"),
                region(Tip::Label, 1. + BOX_HEIGHT, NAME_HEIGHT, "Client pick"),
            ]
        );
        assert_eq!(regions[0].y + regions[0].height, regions[1].y);
        assert_eq!(regions[1].y + regions[1].height, CELL_HEIGHT - 1.);
    }
}
