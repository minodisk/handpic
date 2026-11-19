//! The ring of decoded neighbour previews, a port of `prefetch.ts`: which
//! files around the current one to hold, which to read next, and what to give
//! up on a list change. It names no GPUI type; every entry it gives up is
//! handed back for the caller to retire.
//!
//! An entry is owned by exactly one place at a time, so an eviction cannot
//! release an image on screen: `take` removes it from the ring for the view,
//! and the view hands it back through `keep` only once another image has
//! replaced it.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

/// The ring's total, excluding the image on screen. At 4 bytes a pixel a
/// 1616x1080 preview is 7 MB and a 4416x2944 RAF one 52 MB, so 32 MP (128 MB)
/// holds both neighbours of the largest RAW previews, while the whole JPEGs
/// of a view-only folder (24 MP and up) may not fit two at once; a preview too
/// large to share the cap with its neighbour is not kept at all.
pub const PREFETCH_PIXEL_CAP: u64 = 32_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    Back,
    #[default]
    Forward,
}

/// What the ring needs of an entry: its upright image's pixel count.
pub trait Pixels {
    fn pixels(&self) -> u64;
}

/// The paging direction, from the index last shown to the one shown now
/// (`pageDirection` in `main.ts`).
#[derive(Default)]
pub struct Heading {
    last: usize,
    direction: Direction,
}

impl Heading {
    pub fn to(&mut self, index: usize) {
        if index != self.last {
            self.direction = if index < self.last {
                Direction::Back
            } else {
                Direction::Forward
            };
            self.last = index;
        }
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }
}

fn key(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
}

/// The files to hold around `index`, the next one in the paging direction
/// first.
pub fn neighbours(files: &[impl AsRef<Path>], index: usize, direction: Direction) -> Vec<String> {
    let (ahead, behind) = match direction {
        Direction::Forward => (index.checked_add(1), index.checked_sub(1)),
        Direction::Back => (index.checked_sub(1), index.checked_add(1)),
    };
    [ahead, behind]
        .into_iter()
        .flatten()
        .filter_map(|at| files.get(at).map(key))
        .collect()
}

struct Pending {
    path: String,
    stale: bool,
}

pub struct Ring<T> {
    held: HashMap<String, T>,
    wanted: Vec<String>,
    /// Files that failed to load, not asked for again until they are
    /// forgotten.
    refused: HashSet<String>,
    /// Files evicted for space, not asked for again while they stay wanted, so
    /// a neighbour that cannot share the cap is not re-read on every page
    /// turn. They are dropped by `plan` once the page has moved past them.
    crowded: HashSet<String>,
    /// The one prefetch in flight. It stays here until its reply settles,
    /// even when it no longer matters, so stale reads never pile up.
    pending: Option<Pending>,
}

impl<T> Default for Ring<T> {
    fn default() -> Self {
        Self {
            held: HashMap::new(),
            wanted: Vec::new(),
            refused: HashSet::new(),
            crowded: HashSet::new(),
            pending: None,
        }
    }
}

impl<T: Pixels> Ring<T> {
    /// Points the ring at `index`, returning the entries no longer wanted.
    /// The caller takes the file it is about to show first, or its entry goes
    /// too.
    pub fn plan(
        &mut self,
        files: &[impl AsRef<Path>],
        index: usize,
        direction: Direction,
    ) -> Vec<T> {
        self.wanted = neighbours(files, index, direction);
        let wanted = self.wanted.clone();
        let gone = |path: &str| !wanted.iter().any(|w| w == path);
        self.crowded.retain(|path| !gone(path));
        self.drop_where(gone)
    }

    /// The file to read next, marked in flight, or `None` when there is none
    /// or one is already in flight.
    pub fn next(&mut self) -> Option<String> {
        if self.pending.is_some() {
            return None;
        }
        let path = self
            .wanted
            .iter()
            .find(|at| {
                !self.held.contains_key(*at)
                    && !self.refused.contains(*at)
                    && !self.crowded.contains(*at)
            })?
            .clone();
        self.pending = Some(Pending {
            path: path.clone(),
            stale: false,
        });
        Some(path)
    }

    /// Whether the prefetch in flight still has a place in the ring.
    pub fn still_wanted(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|p| !p.stale && self.wanted.contains(&p.path))
    }

    /// Ends the prefetch in flight with its decoded preview, or `None` when it
    /// could not be loaded, returning the entries to retire.
    pub fn settle(&mut self, held: Option<T>) -> Vec<T> {
        let wanted = self.still_wanted();
        let path = self.pending.take().map(|p| p.path);
        let Some(path) = path.filter(|_| wanted) else {
            return held.into_iter().collect();
        };
        match held {
            None => {
                self.refused.insert(path);
                Vec::new()
            }
            Some(held) => self.insert(path, held),
        }
    }

    /// Removes `path`'s entry for the view, which now owns it.
    pub fn take(&mut self, path: &str) -> Option<T> {
        self.held.remove(path)
    }

    /// Takes back the entry the view has stopped drawing, returning the
    /// entries to retire (the given one when `path` is not wanted).
    pub fn keep(&mut self, path: String, held: T) -> Vec<T> {
        if self.wanted.contains(&path) {
            self.insert(path, held)
        } else {
            vec![held]
        }
    }

    /// A folder switch, or a re-listing that may have changed any file's
    /// bytes.
    pub fn clear(&mut self) -> Vec<T> {
        self.mark_stale(|_| true);
        self.refused.clear();
        self.crowded.clear();
        self.drop_where(|_| true)
    }

    /// A file list change: only the files that left the list go.
    pub fn retain(&mut self, files: &[impl AsRef<Path>]) -> Vec<T> {
        let kept: HashSet<String> = files.iter().map(key).collect();
        let gone = |path: &str| !kept.contains(path);
        self.mark_stale(gone);
        self.refused.retain(|path| !gone(path));
        self.crowded.retain(|path| !gone(path));
        self.drop_where(gone)
    }

    /// A rename or a delete: `paths` no longer name what the ring read.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the native app has no rename, trash or delete yet; the plans adding them call it"
        )
    )]
    pub fn forget(&mut self, paths: &[String]) -> Vec<T> {
        let gone = |path: &str| paths.iter().any(|p| p == path);
        self.mark_stale(gone);
        for path in paths {
            self.refused.remove(path);
            self.crowded.remove(path);
        }
        self.drop_where(gone)
    }

    fn mark_stale(&mut self, gone: impl Fn(&str) -> bool) {
        if let Some(pending) = &mut self.pending
            && gone(&pending.path)
        {
            pending.stale = true;
        }
    }

    fn drop_where(&mut self, gone: impl Fn(&str) -> bool) -> Vec<T> {
        let paths: Vec<String> = self.held.keys().filter(|p| gone(p)).cloned().collect();
        paths
            .iter()
            .filter_map(|path| self.held.remove(path))
            .collect()
    }

    /// Adds an entry, then evicts the lowest-priority entries (the last wanted
    /// first) until the ring is under the cap.
    fn insert(&mut self, path: String, held: T) -> Vec<T> {
        let mut given = Vec::new();
        given.extend(self.held.insert(path, held));
        for at in (0..self.wanted.len()).rev() {
            if self.pixels() <= PREFETCH_PIXEL_CAP {
                break;
            }
            let victim = &self.wanted[at];
            if let Some(entry) = self.held.remove(victim) {
                given.push(entry);
                self.crowded.insert(victim.clone());
            }
        }
        given
    }

    fn pixels(&self) -> u64 {
        self.held.values().map(Pixels::pixels).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Heading, PREFETCH_PIXEL_CAP, Pixels, Ring, neighbours};

    #[derive(Debug)]
    struct Fake {
        name: String,
        pixels: u64,
    }

    impl Pixels for Fake {
        fn pixels(&self) -> u64 {
            self.pixels
        }
    }

    fn held(name: &str) -> Fake {
        sized(name, 1)
    }

    fn sized(name: &str, pixels: u64) -> Fake {
        Fake {
            name: name.into(),
            pixels,
        }
    }

    const FILES: [&str; 4] = ["a", "b", "c", "d"];
    const FORWARD: Direction = Direction::Forward;

    fn names(given: Vec<Fake>) -> Vec<String> {
        let mut names: Vec<String> = given.into_iter().map(|f| f.name).collect();
        names.sort();
        names
    }

    /// Reads and stores one file.
    fn fill(ring: &mut Ring<Fake>) {
        if let Some(path) = ring.next() {
            ring.settle(Some(held(&path)));
        }
    }

    #[test]
    fn puts_the_next_file_in_the_paging_direction_first() {
        assert_eq!(neighbours(&FILES, 1, FORWARD), ["c", "a"]);
        assert_eq!(neighbours(&FILES, 1, Direction::Back), ["a", "c"]);
    }

    #[test]
    fn stops_at_the_ends_of_the_list() {
        assert_eq!(neighbours(&FILES, 0, FORWARD), ["b"]);
        assert_eq!(neighbours(&FILES, 3, FORWARD), ["c"]);
        assert_eq!(neighbours(&FILES, 0, Direction::Back), ["b"]);
    }

    #[test]
    fn the_heading_follows_the_last_move_and_holds_on_a_reshow() {
        let mut heading = Heading::default();
        assert_eq!(heading.direction(), FORWARD);
        heading.to(3);
        heading.to(2);
        assert_eq!(heading.direction(), Direction::Back);
        heading.to(2);
        assert_eq!(heading.direction(), Direction::Back);
        heading.to(5);
        assert_eq!(heading.direction(), FORWARD);
    }

    #[test]
    fn reads_the_wanted_files_in_order_one_at_a_time() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        assert_eq!(ring.next().as_deref(), Some("c"));
        assert_eq!(ring.next(), None);
        ring.settle(Some(held("c")));
        assert_eq!(ring.next().as_deref(), Some("a"));
        ring.settle(Some(held("a")));
        assert_eq!(ring.next(), None);
    }

    #[test]
    fn take_hands_the_entry_over_once() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        fill(&mut ring);
        assert_eq!(ring.take("c").map(|f| f.name).as_deref(), Some("c"));
        assert!(ring.take("c").is_none());
    }

    #[test]
    fn a_page_turn_gives_up_what_is_no_longer_a_neighbour() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        fill(&mut ring);
        fill(&mut ring);
        ring.take("c");
        assert_eq!(names(ring.plan(&FILES, 2, FORWARD)), ["a"]);
    }

    #[test]
    fn keep_takes_back_a_neighbour_and_gives_up_anything_else() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 2, FORWARD);
        assert!(ring.keep("b".into(), held("b")).is_empty());
        assert_eq!(names(ring.keep("a".into(), held("a"))), ["a"]);
        assert_eq!(ring.take("b").map(|f| f.name).as_deref(), Some("b"));
    }

    #[test]
    fn the_file_on_screen_is_never_read_or_kept() {
        let mut ring = Ring::default();
        ring.plan(&["a"], 0, FORWARD);
        assert_eq!(ring.next(), None);
        assert_eq!(names(ring.keep("a".into(), held("a"))), ["a"]);
    }

    #[test]
    fn over_the_cap_the_lowest_priority_entry_goes_first() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        assert!(
            ring.keep("a".into(), sized("a", PREFETCH_PIXEL_CAP))
                .is_empty()
        );
        assert_eq!(
            names(ring.keep("c".into(), sized("c", PREFETCH_PIXEL_CAP))),
            ["a"]
        );
        assert_eq!(ring.take("c").map(|f| f.name).as_deref(), Some("c"));
        assert!(ring.take("a").is_none());
    }

    #[test]
    fn a_preview_over_the_cap_is_not_kept_and_not_read_again() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 2, FORWARD);
        assert_eq!(ring.next().as_deref(), Some("d"));
        assert_eq!(
            names(ring.settle(Some(sized("d", PREFETCH_PIXEL_CAP + 1)))),
            ["d"]
        );
        assert_eq!(ring.next().as_deref(), Some("b"));
        ring.settle(Some(held("b")));
        ring.plan(&FILES, 2, FORWARD);
        assert_eq!(ring.next(), None);
    }

    #[test]
    fn a_neighbour_evicted_for_space_is_not_re_read_while_it_stays_wanted() {
        let big = PREFETCH_PIXEL_CAP * 3 / 4;
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        ring.keep("a".into(), sized("a", big));
        assert_eq!(ring.next().as_deref(), Some("c"));
        assert_eq!(names(ring.settle(Some(sized("c", big)))), ["a"]);
        assert_eq!(ring.next(), None);
        // Turning on to "c" makes "a" unwanted, which ends its exclusion.
        ring.plan(&FILES, 2, FORWARD);
        ring.plan(&FILES, 1, FORWARD);
        assert_eq!(ring.next().as_deref(), Some("c"));
        ring.settle(Some(held("c")));
        assert_eq!(ring.next().as_deref(), Some("a"));
    }

    #[test]
    fn a_failed_read_is_not_retried_until_the_file_is_forgotten() {
        let mut ring = Ring::<Fake>::default();
        ring.plan(&["a", "b"], 0, FORWARD);
        assert_eq!(ring.next().as_deref(), Some("b"));
        assert!(ring.settle(None).is_empty());
        ring.plan(&["a", "b"], 0, FORWARD);
        assert_eq!(ring.next(), None);
        ring.forget(&["b".into()]);
        assert_eq!(ring.next().as_deref(), Some("b"));
    }

    #[test]
    fn a_reply_for_a_file_the_page_has_moved_past_is_given_up() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        assert_eq!(ring.next().as_deref(), Some("c"));
        ring.plan(&FILES, 0, FORWARD);
        assert!(!ring.still_wanted());
        assert_eq!(names(ring.settle(Some(held("c")))), ["c"]);
    }

    #[test]
    fn one_prefetch_stays_in_flight_until_its_reply_even_when_stale() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        ring.next();
        ring.clear();
        ring.plan(&FILES, 1, FORWARD);
        assert_eq!(ring.next(), None);
        ring.settle(Some(held("c")));
        assert_eq!(ring.next().as_deref(), Some("c"));
    }

    #[test]
    fn a_folder_switch_gives_up_everything_and_voids_the_prefetch_in_flight() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        fill(&mut ring);
        ring.next();
        assert_eq!(names(ring.clear()), ["c"]);
        ring.plan(&FILES, 1, FORWARD);
        assert_eq!(names(ring.settle(Some(held("a")))), ["a"]);
        assert!(ring.take("a").is_none());
    }

    #[test]
    fn a_file_list_change_drops_only_the_files_that_left() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        fill(&mut ring);
        fill(&mut ring);
        assert_eq!(names(ring.retain(&["a", "b", "d"])), ["c"]);
        assert_eq!(ring.take("a").map(|f| f.name).as_deref(), Some("a"));
    }

    #[test]
    fn a_rename_or_delete_forgets_the_paths_and_voids_a_read_of_them() {
        let mut ring = Ring::default();
        ring.plan(&FILES, 1, FORWARD);
        fill(&mut ring);
        ring.next();
        assert_eq!(names(ring.forget(&["c".into(), "x".into()])), ["c"]);
        assert!(ring.take("c").is_none());
        assert!(ring.still_wanted());
        assert!(ring.forget(&["a".into()]).is_empty());
        assert!(!ring.still_wanted());
        assert_eq!(names(ring.settle(Some(held("a")))), ["a"]);
    }

    #[test]
    fn the_held_total_never_exceeds_the_cap_after_a_settle_or_a_keep() {
        let sizes = [
            1,
            PREFETCH_PIXEL_CAP / 3,
            PREFETCH_PIXEL_CAP / 2,
            PREFETCH_PIXEL_CAP * 3 / 4,
            PREFETCH_PIXEL_CAP,
            PREFETCH_PIXEL_CAP + 1,
        ];
        let files: Vec<String> = (0..8).map(|n| format!("f{n}")).collect();
        let mut ring = Ring::default();
        let mut step = 0usize;
        for round in 0..200usize {
            let index = (round * 7) % files.len();
            let direction = if round % 3 == 0 {
                Direction::Back
            } else {
                FORWARD
            };
            ring.plan(&files, index, direction);
            step += 1;
            let size = sizes[step % sizes.len()];
            if let Some(path) = ring.next() {
                ring.settle(Some(sized(&path, size)));
                assert!(ring.pixels() <= PREFETCH_PIXEL_CAP);
            }
            step += 1;
            let size = sizes[step % sizes.len()];
            let path = files[(index + round) % files.len()].clone();
            ring.keep(path.clone(), sized(&path, size));
            assert!(ring.pixels() <= PREFETCH_PIXEL_CAP);
        }
    }
}
