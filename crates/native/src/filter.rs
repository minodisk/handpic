//! The strip's filter, a port of `filter.ts`: the checked items of one section
//! are OR-ed, the sections AND-ed, and a section with nothing checked lets
//! everything through. `0` stars is unrated; a reject keeps its stars. A label
//! is keyed lower-cased, or `none` without one; a label outside the menu's
//! colors matches no item. `Face focus` partitions the files like the focus
//! mark's states (`mark::af_eye_state`), a file the pass has not reached being
//! `Neutral`; `Eyes` reads the AF face's stored eye state, a file without one
//! being `Unknown`. A file whose export state is not known passes neither
//! `Output` item. An EXIF group's checked labels match the file's label in
//! that group (`exif::exif_key`), a file without one failing it; the EXIF
//! checks are per session and dropped when their label leaves the folder.
//!
//! The `Genre` section is port Step 9's (the ranking) and is not here yet.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use riffle_backend::{
    exif::Exif,
    index::{FaceReady, Focus, IndexedFile},
};
use riffle_core::{Flag, eyes::EyeState};
use serde_json::Value;

use crate::{
    exif::{self, Group, Groups},
    judgment::State,
    mark::MarkState,
};

/// The status line and the pane while the filter hides every listed file.
pub const FILTERED_TEXT: &str = "No files match the current filter.";

/// What the index has of a listed file that the strip's order and the filter
/// read.
pub struct Facts {
    pub orientation: u16,
    pub capture_time: Option<String>,
    pub subsec: Option<String>,
    pub focus: Option<Focus>,
    pub exif: Option<Exif>,
}

impl From<&IndexedFile> for Facts {
    fn from(file: &IndexedFile) -> Self {
        Self {
            orientation: file.orientation,
            capture_time: file.capture_time.clone(),
            subsec: file.subsec.clone(),
            focus: file.focus,
            exif: file.exif.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterFlag {
    Picked,
    Untagged,
    Rejected,
}

/// The displayed shape, decided by the Orientation tag alone: every sensor
/// read is landscape, so a quarter turn (6 or 8) is portrait.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    Portrait,
    Landscape,
}

pub fn orientation_of(tag: u16) -> Orientation {
    match tag {
        6 | 8 => Orientation::Portrait,
        _ => Orientation::Landscape,
    }
}

/// One item of the `Face focus` section.
pub type AfEye = MarkState;

/// One item of the `Eyes` section: the stored eye state, `Unknown` without
/// one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EyesItem {
    Open,
    Closed,
    Unknown,
}

impl EyesItem {
    pub fn of(eyes: Option<EyeState>) -> Self {
        match eyes {
            Some(EyeState::Open) => Self::Open,
            Some(EyeState::Closed) => Self::Closed,
            None => Self::Unknown,
        }
    }
}

/// One item of the `Output` section: whether a file has an export.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutputState {
    Yes,
    No,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilterState {
    pub flags: HashSet<FilterFlag>,
    pub stars: HashSet<u8>,
    pub labels: HashSet<String>,
    pub orientations: HashSet<Orientation>,
    pub candidates: HashSet<AfEye>,
    pub eyes: HashSet<EyesItem>,
    pub outputs: HashSet<OutputState>,
    /// The checked labels of each EXIF group; a group with none has no entry.
    pub exif: BTreeMap<Group, BTreeSet<String>>,
}

impl FilterState {
    /// Any section has an item checked.
    pub fn is_active(&self) -> bool {
        !(self.flags.is_empty()
            && self.stars.is_empty()
            && self.labels.is_empty()
            && self.orientations.is_empty()
            && self.candidates.is_empty()
            && self.eyes.is_empty()
            && self.outputs.is_empty()
            && self.exif.is_empty())
    }

    /// Replace the remembered sections' checks with `stored`, leaving
    /// `outputs` and `exif`, which are per session.
    pub fn apply_stored(&mut self, stored: &Stored) {
        self.flags = stored.flags.iter().copied().collect();
        self.stars = stored.stars.iter().copied().collect();
        self.labels = stored.labels.iter().cloned().collect();
        self.orientations = stored.orientations.iter().copied().collect();
        self.candidates = stored.candidates.iter().copied().collect();
        self.eyes = stored.eyes.iter().copied().collect();
    }

    /// Whether the open folder gave an export answer: without one the
    /// `Output` section is hidden and its checks cleared.
    pub fn set_output_known(&mut self, known: bool) {
        if !known {
            self.outputs.clear();
        }
    }

    /// Check `label` of `group` if unchecked, else uncheck it.
    pub fn toggle_exif(&mut self, group: Group, label: &str) {
        let set = self.exif.entry(group).or_default();
        if !set.remove(label) {
            set.insert(label.to_owned());
        }
        if set.is_empty() {
            self.exif.remove(&group);
        }
    }

    /// Another folder's open: no EXIF item is checked.
    pub fn clear_exif(&mut self) {
        self.exif.clear();
    }

    /// Drop each checked EXIF label the folder's `groups` no longer have, so
    /// a stale check cannot hide every file.
    pub fn prune_exif(&mut self, groups: &Groups) {
        self.exif.retain(|group, set| {
            let labels = groups.labels(*group);
            set.retain(|label| labels.contains(label));
            !set.is_empty()
        });
    }
}

/// The remembered filter, the Tauri `settings.json`'s `filter`: every
/// section but the outputs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stored {
    pub flags: Vec<FilterFlag>,
    pub stars: Vec<u8>,
    pub labels: Vec<String>,
    pub orientations: Vec<Orientation>,
    pub candidates: Vec<AfEye>,
    pub eyes: Vec<EyesItem>,
}

impl Stored {
    /// The stored `filter` by the Tauri `filter_setting` rule: each section
    /// keeps only its known members, deduplicated in stored order; a
    /// non-object value or a non-array section is empty, other keys dropped.
    pub fn from_setting(value: Option<&Value>) -> Self {
        Self {
            flags: members(value, "flags", |s| match s.as_str()? {
                "picked" => Some(FilterFlag::Picked),
                "untagged" => Some(FilterFlag::Untagged),
                "rejected" => Some(FilterFlag::Rejected),
                _ => None,
            }),
            stars: members(value, "stars", |s| {
                s.as_u64().filter(|n| *n <= 5).map(|n| n as u8)
            }),
            labels: members(value, "labels", |s| {
                let s = s.as_str()?;
                LABELS.contains(&s).then(|| s.to_owned())
            }),
            orientations: members(value, "orientations", |s| match s.as_str()? {
                "portrait" => Some(Orientation::Portrait),
                "landscape" => Some(Orientation::Landscape),
                _ => None,
            }),
            candidates: members(value, "candidates", |s| match s.as_str()? {
                "good" => Some(MarkState::Good),
                "neutral" => Some(MarkState::Neutral),
                "low" => Some(MarkState::Low),
                _ => None,
            }),
            eyes: members(value, "eyes", |s| match s.as_str()? {
                "open" => Some(EyesItem::Open),
                "closed" => Some(EyesItem::Closed),
                "unknown" => Some(EyesItem::Unknown),
                _ => None,
            }),
        }
    }
}

const LABELS: &[&str] = &[
    "red", "orange", "yellow", "green", "blue", "pink", "purple", "none",
];

fn members<T: PartialEq>(
    value: Option<&Value>,
    name: &str,
    parse: impl Fn(&Value) -> Option<T>,
) -> Vec<T> {
    let mut kept = Vec::new();
    let items = value
        .and_then(|v| v.get(name))
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for item in items.filter_map(parse) {
        if !kept.contains(&item) {
            kept.push(item);
        }
    }
    kept
}

pub fn passes(
    state: &FilterState,
    judgment: &State,
    facts: Option<&Facts>,
    af_eye: AfEye,
    eyes: Option<EyeState>,
    output: Option<OutputState>,
) -> bool {
    let flag = match judgment.flag {
        Flag::Pick => FilterFlag::Picked,
        Flag::Reject => FilterFlag::Rejected,
        Flag::None => FilterFlag::Untagged,
    };
    let stars = judgment.rating.unwrap_or(0);
    let label_matches = || match &judgment.label {
        None => state.labels.contains("none"),
        Some(label) => state.labels.contains(&label.to_lowercase()),
    };
    (state.flags.is_empty() || state.flags.contains(&flag))
        && (state.stars.is_empty() || state.stars.contains(&stars))
        && (state.labels.is_empty() || label_matches())
        && (state.orientations.is_empty()
            || facts.is_some_and(|f| state.orientations.contains(&orientation_of(f.orientation))))
        && (state.candidates.is_empty() || state.candidates.contains(&af_eye))
        && (state.eyes.is_empty() || state.eyes.contains(&EyesItem::of(eyes)))
        && (state.outputs.is_empty() || output.is_some_and(|o| state.outputs.contains(&o)))
        && state.exif.iter().all(|(group, labels)| {
            exif::exif_key(facts.and_then(|f| f.exif.as_ref()), *group)
                .is_some_and(|key| labels.contains(&key.label))
        })
}

/// The export state of `path`, from the `found` subset of `listing` the
/// export folder gave, or `None` without an answer. A file outside `listing`
/// is unknown rather than `No`.
pub fn output_of(
    found: Option<&HashSet<String>>,
    listing: &HashSet<String>,
    path: &str,
) -> Option<OutputState> {
    let found = found?;
    if !listing.contains(path) {
        return None;
    }
    Some(if found.contains(path) {
        OutputState::Yes
    } else {
        OutputState::No
    })
}

/// Patch each of pass 2's `ready` files into its facts' focus, as the Tauri
/// `applyFaceReady`: a file without facts or without a focus (a JPEG) is left
/// alone.
pub fn apply_face_ready(facts: &mut HashMap<String, Facts>, ready: &[FaceReady]) {
    for item in ready {
        if let Some(focus) = facts.get_mut(&item.path).and_then(|f| f.focus.as_mut()) {
            focus.eye_focus = item.eye_focus;
            focus.candidate = item.candidate;
            focus.eyes = item.eyes;
        }
    }
}

/// The path that becomes current after a refilter: `anchor` if it still
/// passes; otherwise the next passing path after it in `all`, else the last
/// one before it.
///
/// `previous` is the list shown before `all` was re-listed: when given and
/// `anchor` is gone from `all`, the anchor's next surviving passing neighbour
/// in `previous` wins, else its previous one, whether or not the anchor
/// itself still passes.
pub fn anchor_after_filter(
    all: &[String],
    pass: impl Fn(&str) -> bool,
    anchor: Option<&str>,
    previous: Option<&[String]>,
) -> Option<String> {
    let anchor = anchor?;
    if let Some(previous) = previous
        && !all.iter().any(|p| p == anchor)
    {
        let present: HashSet<&str> = all.iter().map(String::as_str).collect();
        let survives = |p: &&String| present.contains(p.as_str()) && pass(p);
        let (after, before) = match previous.iter().position(|p| p == anchor) {
            Some(from) => (&previous[from + 1..], &previous[..from]),
            None => (previous, &previous[..0]),
        };
        return after
            .iter()
            .find(survives)
            .or_else(|| before.iter().rev().find(survives))
            .cloned();
    }
    if pass(anchor) {
        return Some(anchor.to_owned());
    }
    let (after, before) = match all.iter().position(|p| p == anchor) {
        Some(from) => (&all[from + 1..], &all[..from]),
        None => (all, &all[..0]),
    };
    after
        .iter()
        .find(|p| pass(p))
        .or_else(|| before.iter().rev().find(|p| pass(p)))
        .cloned()
}

/// Whether `refilter` must rebuild `files`: the filtered `next` differs from
/// `files`, or `force` says to rebuild regardless.
pub fn filter_list_changed(files: &[String], next: &[String], force: bool) -> bool {
    force || files != next
}

/// Where the shown file, at `at` in `old`, sits in `next`: `None` once it is
/// not listed there, so a later refilter does not take the file then at that
/// index for the one on screen.
pub fn shown_after(old: &[String], next: &[String], at: Option<usize>) -> Option<usize> {
    let path = old.get(at?)?;
    next.iter().position(|p| p == path)
}

/// The anchor of the refilter that emptied the shown list, given back to a
/// menu change while no file is current, so it lands on the file shown before
/// the list emptied.
#[derive(Debug, Default)]
pub struct KeptAnchor(Option<String>);

impl KeptAnchor {
    /// After a refilter from `anchor` that left the list `empty` or not.
    pub fn refiltered(&mut self, anchor: Option<&str>, empty: bool) {
        if !empty {
            self.0 = None;
        } else if let Some(anchor) = anchor {
            self.0 = Some(anchor.to_owned());
        }
    }

    /// `current`, else the kept anchor.
    pub fn anchor(&self, current: Option<String>) -> Option<String> {
        current.or_else(|| self.0.clone())
    }

    /// Another folder's open.
    pub fn clear(&mut self) {
        self.0 = None;
    }
}

#[cfg(test)]
mod tests {
    //! `filter.test.ts`, less the `Genre` section's six cases (port Step 9).
    //! `toStored` is not ported (nothing is written), so the round trip
    //! checks `apply_stored` alone. A `Face focus` state the Tauri
    //! cases leave `undefined` is `mark::af_eye_state(None)`, `Neutral`.

    use super::*;
    use crate::mark::af_eye_state;
    use riffle_backend::index::{FocusPoint, StoredEyes};
    use riffle_core::candidate::FocusCandidate;
    use serde_json::json;

    const NEUTRAL: AfEye = MarkState::Neutral;

    #[allow(clippy::too_many_arguments)]
    fn state(
        flags: &[FilterFlag],
        stars: &[u8],
        labels: &[&str],
        orientations: &[Orientation],
        candidates: &[AfEye],
        eyes: &[EyesItem],
        outputs: &[OutputState],
    ) -> FilterState {
        FilterState {
            flags: flags.iter().copied().collect(),
            stars: stars.iter().copied().collect(),
            labels: labels.iter().map(|l| l.to_string()).collect(),
            orientations: orientations.iter().copied().collect(),
            candidates: candidates.iter().copied().collect(),
            eyes: eyes.iter().copied().collect(),
            outputs: outputs.iter().copied().collect(),
            exif: BTreeMap::new(),
        }
    }

    fn none() -> FilterState {
        FilterState::default()
    }

    fn flags(flags: &[FilterFlag]) -> FilterState {
        state(flags, &[], &[], &[], &[], &[], &[])
    }

    fn judged(rating: Option<u8>, flag: Flag, label: Option<&str>) -> State {
        State {
            rating,
            flag,
            label: label.map(str::to_owned),
        }
    }

    fn unjudged() -> State {
        judged(None, Flag::None, None)
    }
    fn rejected() -> State {
        judged(None, Flag::Reject, None)
    }
    fn three_stars() -> State {
        judged(Some(3), Flag::None, None)
    }
    fn picked_two() -> State {
        judged(Some(2), Flag::Pick, None)
    }

    fn facts(orientation: u16) -> Facts {
        Facts {
            orientation,
            capture_time: None,
            subsec: None,
            focus: None,
            exif: None,
        }
    }

    /// `passes` with the judgment alone.
    fn judgment_passes(s: &FilterState, j: &State) -> bool {
        passes(s, j, None, NEUTRAL, None, None)
    }

    use FilterFlag::{Picked, Rejected, Untagged};

    #[test]
    fn empty_groups_pass_everything() {
        for j in [unjudged(), rejected(), three_stars(), picked_two()] {
            assert!(judgment_passes(&none(), &j));
        }
        assert!(!none().is_active());
    }

    #[test]
    fn ors_the_checked_flags() {
        let s = flags(&[Picked, Rejected]);
        assert!(s.is_active());
        assert!(judgment_passes(&s, &picked_two()));
        assert!(judgment_passes(&s, &rejected()));
        assert!(!judgment_passes(&s, &unjudged()));
    }

    #[test]
    fn ors_the_checked_stars() {
        let s = state(&[], &[0, 3], &[], &[], &[], &[], &[]);
        assert!(judgment_passes(&s, &unjudged()));
        assert!(judgment_passes(&s, &three_stars()));
        assert!(!judgment_passes(&s, &picked_two()));
    }

    #[test]
    fn ands_the_groups() {
        let s = state(&[Untagged], &[3], &[], &[], &[], &[], &[]);
        assert!(judgment_passes(&s, &three_stars()));
        assert!(!judgment_passes(&s, &unjudged()));
        assert!(!judgment_passes(&s, &picked_two()));
    }

    #[test]
    fn a_reject_counts_as_rejected_and_0_stars() {
        let s = state(&[Rejected], &[0], &[], &[], &[], &[], &[]);
        assert!(judgment_passes(&s, &rejected()));
        assert!(!judgment_passes(&flags(&[Untagged]), &rejected()));
    }

    #[test]
    fn a_reject_with_stars_counts_as_rejected_and_its_stars() {
        let rejected_four = judged(Some(4), Flag::Reject, None);
        let s = state(&[Rejected], &[4], &[], &[], &[], &[], &[]);
        assert!(judgment_passes(&s, &rejected_four));
        let unrated = state(&[], &[0], &[], &[], &[], &[], &[]);
        assert!(!judgment_passes(&unrated, &rejected_four));
    }

    #[test]
    fn a_pick_with_stars_counts_as_picked_and_its_stars() {
        let s = state(&[Picked], &[2], &[], &[], &[], &[], &[]);
        assert!(judgment_passes(&s, &picked_two()));
        assert!(!judgment_passes(&flags(&[Untagged]), &picked_two()));
        let unrated = state(&[], &[0], &[], &[], &[], &[], &[]);
        assert!(!judgment_passes(&unrated, &picked_two()));
    }

    fn labels(labels: &[&str]) -> FilterState {
        state(&[], &[], labels, &[], &[], &[], &[])
    }

    fn labeled(label: &str) -> State {
        judged(None, Flag::None, Some(label))
    }

    #[test]
    fn a_labeled_file_fails_none() {
        assert!(!judgment_passes(&labels(&["none"]), &labeled("Red")));
        assert!(judgment_passes(&labels(&["none"]), &unjudged()));
    }

    #[test]
    fn a_red_file_passes_red_and_fails_blue() {
        assert!(judgment_passes(&labels(&["red"]), &labeled("Red")));
        assert!(!judgment_passes(&labels(&["blue"]), &labeled("Red")));
    }

    #[test]
    fn ors_the_checked_colors() {
        let s = labels(&["red", "blue"]);
        assert!(judgment_passes(&s, &labeled("Red")));
        assert!(judgment_passes(&s, &labeled("Blue")));
        assert!(!judgment_passes(&s, &unjudged()));
    }

    #[test]
    fn matches_case_insensitively() {
        assert!(judgment_passes(&labels(&["red"]), &labeled("RED")));
    }

    #[test]
    fn a_foreign_label_fails_every_color_and_none() {
        let foreign = labeled("Approved");
        for key in LABELS {
            assert!(!judgment_passes(&labels(&[key]), &foreign), "{key}");
        }
        assert!(judgment_passes(&none(), &foreign));
    }

    #[test]
    fn untagged_0_and_none_selects_exactly_the_unjudged_files() {
        let s = state(&[Untagged], &[0], &["none"], &[], &[], &[], &[]);
        assert!(judgment_passes(&s, &unjudged()));
        assert!(!judgment_passes(&s, &labeled("Red")));
        assert!(!judgment_passes(&s, &three_stars()));
        assert!(!judgment_passes(&s, &judged(None, Flag::Pick, None)));
        assert!(!judgment_passes(&s, &rejected()));
    }

    fn paths(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn anchor(
        all: &[&str],
        pass: impl Fn(&str) -> bool,
        at: Option<&str>,
        previous: Option<&[&str]>,
    ) -> Option<String> {
        let previous = previous.map(paths);
        anchor_after_filter(&paths(all), pass, at, previous.as_deref())
    }

    const ALL: &[&str] = &["a", "b", "c", "d"];

    #[test]
    fn stays_on_the_anchor_if_it_passes() {
        assert_eq!(anchor(ALL, |_| true, Some("b"), None).as_deref(), Some("b"));
    }

    #[test]
    fn moves_to_the_next_passing_file_after_it() {
        assert_eq!(
            anchor(ALL, |p| p != "b", Some("b"), None).as_deref(),
            Some("c")
        );
        assert_eq!(
            anchor(ALL, |p| p == "a" || p == "d", Some("b"), None).as_deref(),
            Some("d")
        );
    }

    #[test]
    fn falls_back_to_the_last_passing_file_before_it() {
        assert_eq!(
            anchor(ALL, |p| p == "a" || p == "b", Some("c"), None).as_deref(),
            Some("b")
        );
    }

    #[test]
    fn returns_none_when_nothing_passes() {
        assert_eq!(anchor(ALL, |_| false, Some("b"), None), None);
        assert_eq!(anchor(ALL, |_| true, None, None), None);
    }

    #[test]
    fn a_deleted_anchor_moves_to_the_next_file_in_the_previous_list() {
        let got = anchor(&["a", "c", "d"], |_| true, Some("b"), Some(ALL));
        assert_eq!(got.as_deref(), Some("c"));
    }

    #[test]
    fn a_deleted_last_anchor_moves_to_the_previous_file() {
        let got = anchor(&["a", "b", "c"], |_| true, Some("d"), Some(ALL));
        assert_eq!(got.as_deref(), Some("c"));
    }

    #[test]
    fn a_deleted_anchor_skips_a_neighbour_that_does_not_pass() {
        let got = anchor(&["a", "c", "d"], |p| p != "c", Some("b"), Some(ALL));
        assert_eq!(got.as_deref(), Some("d"));
    }

    #[test]
    fn a_deleted_anchor_skips_a_run_of_consecutive_deletions() {
        let got = anchor(&["a", "d"], |_| true, Some("b"), Some(ALL));
        assert_eq!(got.as_deref(), Some("d"));
        let got = anchor(&["a"], |_| true, Some("b"), Some(ALL));
        assert_eq!(got.as_deref(), Some("a"));
    }

    #[test]
    fn a_deleted_anchor_with_nothing_surviving_is_none() {
        assert_eq!(anchor(&[], |_| true, Some("b"), Some(ALL)), None);
    }

    #[test]
    fn a_still_listed_anchor_is_kept() {
        let got = anchor(ALL, |_| true, Some("b"), Some(ALL));
        assert_eq!(got.as_deref(), Some("b"));
    }

    #[test]
    fn an_unchanged_list_without_force_does_not_rebuild() {
        let all = paths(&["a", "b", "c"]);
        assert!(!filter_list_changed(&all, &paths(&["a", "b", "c"]), false));
    }

    #[test]
    fn a_changed_list_rebuilds_even_without_force() {
        let all = paths(&["a", "b", "c"]);
        assert!(filter_list_changed(&all, &paths(&["a", "c"]), false));
    }

    #[test]
    fn an_unchanged_list_with_force_rebuilds_anyway() {
        let all = paths(&["a", "b", "c"]);
        assert!(filter_list_changed(&all, &paths(&["a", "b", "c"]), true));
    }

    /// The Tauri `a judgment under untagged + 0 + none` cases: each of the
    /// four judgments moves on to the next unjudged file, falls back to the
    /// last one before it, and empties the view on the only file.
    #[test]
    fn a_judgment_under_untagged_0_and_none() {
        let s = state(&[Untagged], &[0], &["none"], &[], &[], &[], &[]);
        let judgments = [
            ("rating", three_stars()),
            ("reject", rejected()),
            ("pick", judged(None, Flag::Pick, None)),
            ("label", labeled("Red")),
        ];
        for (name, judgment) in judgments {
            let after = |all: &[&str], at: &str| {
                let pass = |p: &str| {
                    let j = if p == at {
                        judgment.clone()
                    } else {
                        unjudged()
                    };
                    judgment_passes(&s, &j)
                };
                anchor(all, pass, Some(at), None)
            };
            assert_eq!(after(&["a", "b", "c"], "b").as_deref(), Some("c"), "{name}");
            assert_eq!(after(&["a", "b", "c"], "c").as_deref(), Some("b"), "{name}");
            assert_eq!(after(&["a"], "a"), None, "{name}");
        }
    }

    #[test]
    fn a_quarter_turn_is_portrait() {
        assert_eq!(orientation_of(6), Orientation::Portrait);
        assert_eq!(orientation_of(8), Orientation::Portrait);
    }

    #[test]
    fn everything_else_is_landscape() {
        for tag in [1, 3, 99] {
            assert_eq!(orientation_of(tag), Orientation::Landscape);
        }
    }

    fn shaped(s: &FilterState, tag: Option<u16>) -> bool {
        let f = tag.map(facts);
        passes(s, &unjudged(), f.as_ref(), NEUTRAL, None, None)
    }

    fn shapes(o: &[Orientation]) -> FilterState {
        state(&[], &[], &[], o, &[], &[], &[])
    }

    use Orientation::{Landscape, Portrait};

    #[test]
    fn an_empty_orientation_group_passes_both_shapes() {
        assert!(shaped(&none(), Some(1)));
        assert!(shaped(&none(), Some(6)));
    }

    #[test]
    fn portrait_passes_a_quarter_turn_only() {
        let s = shapes(&[Portrait]);
        assert!(shaped(&s, Some(6)));
        assert!(shaped(&s, Some(8)));
        assert!(!shaped(&s, Some(1)));
        assert!(!shaped(&s, Some(3)));
    }

    #[test]
    fn landscape_passes_the_unrotated_tags_only() {
        let s = shapes(&[Landscape]);
        assert!(shaped(&s, Some(1)));
        assert!(shaped(&s, Some(3)));
        assert!(!shaped(&s, Some(6)));
        assert!(!shaped(&s, Some(8)));
    }

    #[test]
    fn both_shapes_checked_pass_both() {
        let s = shapes(&[Portrait, Landscape]);
        assert!(shaped(&s, Some(1)));
        assert!(shaped(&s, Some(6)));
    }

    #[test]
    fn an_unknown_orientation_fails_a_non_empty_selection() {
        assert!(!shaped(&shapes(&[Landscape]), None));
    }

    #[test]
    fn orientation_ands_with_the_other_groups() {
        let s = state(&[Untagged], &[], &[], &[Portrait], &[], &[], &[]);
        assert!(shaped(&s, Some(6)));
        assert!(!shaped(&s, Some(1)));
        let f = facts(6);
        assert!(!passes(&s, &picked_two(), Some(&f), NEUTRAL, None, None));
    }

    fn items(c: &[AfEye]) -> FilterState {
        state(&[], &[], &[], &[], c, &[], &[])
    }

    fn eyed(s: &FilterState, af_eye: AfEye) -> bool {
        passes(s, &unjudged(), Some(&facts(1)), af_eye, None, None)
    }

    fn candidate() -> Focus {
        Focus {
            point: Some(FocusPoint {
                sensor_w: 7008,
                sensor_h: 4672,
                x: 1752,
                y: 1168,
                frame: None,
            }),
            manual_focus: false,
            eye_focus: Some(1.),
            candidate: FocusCandidate::Candidate,
            eyes: StoredEyes {
                eyes_ear: Some(0.35),
                eyes: Some(EyeState::Open),
                eyes_closed: Some(0.001),
                pose: Some(riffle_backend::index::EyesPose {
                    yaw: 10.,
                    pitch: -5.,
                    roll: 3.,
                }),
                eye_offset: Some(0.03),
                edge_gap: Some(2.),
                full_eye_sharp: None,
            },
        }
    }

    fn with(edit: impl FnOnce(&mut Focus)) -> Focus {
        let mut focus = candidate();
        edit(&mut focus);
        focus
    }

    use MarkState::{Good, Low, Neutral};

    #[test]
    fn face_focus_off_passes_every_state() {
        for af_eye in [Good, Neutral, Low] {
            assert!(eyed(&none(), af_eye));
        }
    }

    #[test]
    fn good_passes_a_good_frame_only() {
        assert!(eyed(&items(&[Good]), Good));
        assert!(!eyed(&items(&[Good]), Neutral));
        assert!(!eyed(&items(&[Good]), Low));
        assert!(!eyed(&items(&[Good]), af_eye_state(None)));
    }

    #[test]
    fn low_passes_a_low_frame_only() {
        let low = items(&[Low]);
        assert!(eyed(&low, Low));
        assert!(!eyed(&low, Good));
        assert!(!eyed(&low, Neutral));
        assert!(!eyed(&low, af_eye_state(None)));
    }

    #[test]
    fn a_focus_candidate_that_is_not_good_passes_neutral_not_good_or_low() {
        let not_good = af_eye_state(Some(&with(|f| f.eyes.eyes_ear = Some(0.1))));
        assert!(eyed(&items(&[Neutral]), not_good));
        assert!(!eyed(&items(&[Good]), not_good));
        assert!(!eyed(&items(&[Low]), not_good));
        let good = af_eye_state(Some(&candidate()));
        assert!(eyed(&items(&[Good]), good));
        assert!(!eyed(&items(&[Neutral]), good));
    }

    #[test]
    fn neutral_passes_a_neutral_or_not_yet_computed_state() {
        let neutral = items(&[Neutral]);
        assert!(eyed(&neutral, Neutral));
        assert!(eyed(&neutral, af_eye_state(None)));
        assert!(!eyed(&neutral, Good));
        assert!(!eyed(&neutral, Low));
    }

    #[test]
    fn ors_the_checked_face_focus_states() {
        let either = items(&[Good, Neutral]);
        assert!(eyed(&either, Good));
        assert!(eyed(&either, Neutral));
        assert!(eyed(&either, af_eye_state(None)));
        assert!(!eyed(&either, Low));
    }

    #[test]
    fn good_and_low_checked_exclude_neutral() {
        let ends = items(&[Good, Low]);
        assert!(eyed(&ends, Good));
        assert!(eyed(&ends, Low));
        assert!(!eyed(&ends, Neutral));
        assert!(!eyed(&ends, af_eye_state(None)));
    }

    #[test]
    fn the_three_face_focus_items_partition_the_frames() {
        let not_candidate = |f: &mut Focus| {
            f.candidate = FocusCandidate::NotCandidate;
            f.eye_focus = Some(0.1);
        };
        let unknown = |f: &mut Focus| {
            f.candidate = FocusCandidate::Unknown;
            f.eye_focus = None;
        };
        let frames = [
            (Some(candidate()), Good),
            (Some(with(|f| f.eyes.eyes_ear = Some(0.1))), Neutral),
            (Some(with(not_candidate)), Low),
            (Some(with(unknown)), Neutral),
            (Some(with(|f| f.point = None)), Good),
            (
                Some(with(|f| {
                    f.point = None;
                    not_candidate(f);
                })),
                Low,
            ),
            (
                Some(with(|f| {
                    f.point = None;
                    unknown(f);
                })),
                Neutral,
            ),
            (None, Neutral),
        ];
        for item in [Good, Neutral, Low] {
            for (focus, owner) in &frames {
                let state = af_eye_state(focus.as_ref());
                assert_eq!(eyed(&items(&[item]), state), item == *owner, "{item:?}");
            }
        }
    }

    #[test]
    fn face_focus_ands_with_the_other_groups() {
        let flagged = state(&[Untagged], &[], &[], &[], &[Good], &[], &[]);
        assert!(eyed(&flagged, Good));
        assert!(!passes(&flagged, &picked_two(), None, Good, None, None));
    }

    fn eye_items(e: &[EyesItem]) -> FilterState {
        state(&[], &[], &[], &[], &[], e, &[])
    }

    fn eyes_pass(s: &FilterState, eyes: Option<EyeState>) -> bool {
        passes(s, &unjudged(), Some(&facts(1)), NEUTRAL, eyes, None)
    }

    const OPEN: Option<EyeState> = Some(EyeState::Open);
    const CLOSED: Option<EyeState> = Some(EyeState::Closed);

    #[test]
    fn nothing_checked_passes_every_eye_state() {
        for eyes in [OPEN, CLOSED, None] {
            assert!(eyes_pass(&none(), eyes));
        }
    }

    #[test]
    fn open_passes_open_eyes_only() {
        let s = eye_items(&[EyesItem::Open]);
        assert!(eyes_pass(&s, OPEN));
        assert!(!eyes_pass(&s, CLOSED));
        assert!(!eyes_pass(&s, None));
    }

    #[test]
    fn closed_passes_closed_eyes_only() {
        let s = eye_items(&[EyesItem::Closed]);
        assert!(eyes_pass(&s, CLOSED));
        assert!(!eyes_pass(&s, OPEN));
        assert!(!eyes_pass(&s, None));
    }

    #[test]
    fn unknown_passes_a_file_without_an_eye_state() {
        let s = eye_items(&[EyesItem::Unknown]);
        assert!(eyes_pass(&s, None));
        assert!(!eyes_pass(&s, OPEN));
        assert!(!eyes_pass(&s, CLOSED));
    }

    #[test]
    fn ors_the_checked_eye_states() {
        let either = eye_items(&[EyesItem::Open, EyesItem::Unknown]);
        assert!(eyes_pass(&either, OPEN));
        assert!(eyes_pass(&either, None));
        assert!(!eyes_pass(&either, CLOSED));
    }

    #[test]
    fn eyes_and_with_the_other_groups() {
        let both = state(
            &[Untagged],
            &[],
            &[],
            &[],
            &[Good],
            &[EyesItem::Closed],
            &[],
        );
        let f = facts(1);
        let check = |j: &State, af_eye, eyes| passes(&both, j, Some(&f), af_eye, eyes, None);
        assert!(check(&unjudged(), Good, CLOSED));
        assert!(!check(&unjudged(), Low, CLOSED));
        assert!(!check(&picked_two(), Good, CLOSED));
        assert!(!check(&unjudged(), Good, OPEN));
    }

    fn outputs(o: &[OutputState]) -> FilterState {
        state(&[], &[], &[], &[], &[], &[], o)
    }

    fn output_passes(s: &FilterState, output: Option<OutputState>) -> bool {
        passes(s, &unjudged(), Some(&facts(1)), NEUTRAL, None, output)
    }

    use OutputState::{No, Yes};

    #[test]
    fn no_output_item_lets_every_file_through_known_or_not() {
        for output in [Some(Yes), Some(No), None] {
            assert!(output_passes(&none(), output));
        }
    }

    #[test]
    fn output_passes_the_files_with_an_export() {
        assert!(output_passes(&outputs(&[Yes]), Some(Yes)));
        assert!(!output_passes(&outputs(&[Yes]), Some(No)));
    }

    #[test]
    fn no_output_passes_the_files_without_an_export() {
        assert!(output_passes(&outputs(&[No]), Some(No)));
        assert!(!output_passes(&outputs(&[No]), Some(Yes)));
    }

    #[test]
    fn both_output_items_pass_both_states() {
        assert!(output_passes(&outputs(&[Yes, No]), Some(Yes)));
        assert!(output_passes(&outputs(&[Yes, No]), Some(No)));
    }

    #[test]
    fn an_unknown_output_state_passes_no_checked_item() {
        for o in [&[No][..], &[Yes], &[Yes, No]] {
            assert!(!output_passes(&outputs(o), None));
        }
    }

    #[test]
    fn output_ands_with_the_other_groups() {
        let both = state(&[Untagged], &[], &[], &[], &[], &[], &[No]);
        let f = facts(1);
        let check = |j: &State| passes(&both, j, Some(&f), NEUTRAL, None, Some(No));
        assert!(check(&unjudged()));
        assert!(!check(&picked_two()));
    }

    #[test]
    fn outputs_are_not_remembered() {
        let mut s = outputs(&[No]);
        let stored = Stored {
            flags: vec![Picked],
            ..Stored::default()
        };
        s.apply_stored(&stored);
        assert_eq!(s.outputs, HashSet::from([No]));
        assert_eq!(s.flags, HashSet::from([Picked]));
    }

    fn set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn an_unknown_output_clears_the_output_checks() {
        let mut s = state(&[Picked], &[], &[], &[], &[], &[], &[Yes, No]);
        s.set_output_known(false);
        assert_eq!(s, state(&[Picked], &[], &[], &[], &[], &[], &[]));
    }

    #[test]
    fn a_known_output_keeps_the_output_checks() {
        let mut s = outputs(&[No]);
        s.set_output_known(true);
        assert_eq!(s.outputs, HashSet::from([No]));
    }

    fn ready(path: &str, focus: &Focus) -> FaceReady {
        FaceReady {
            path: path.into(),
            eye_focus: focus.eye_focus,
            candidate: focus.candidate,
            sharpness: Some(0.5),
            eyes: focus.eyes,
        }
    }

    #[test]
    fn a_ready_file_takes_pass_2_values_over_its_af_point() {
        let mut before = candidate();
        before.manual_focus = true;
        before.eye_focus = None;
        before.candidate = FocusCandidate::Unknown;
        before.eyes = StoredEyes::default();
        let mut facts = HashMap::from([(
            "A.ARW".to_string(),
            Facts {
                focus: Some(before),
                ..facts(6)
            },
        )]);
        let after = with(|f| f.eyes.eyes = Some(EyeState::Closed));
        apply_face_ready(&mut facts, &[ready("A.ARW", &after)]);
        let focus = facts["A.ARW"].focus.unwrap();
        assert_eq!(focus.eye_focus, after.eye_focus);
        assert_eq!(focus.candidate, after.candidate);
        assert_eq!(focus.eyes, after.eyes);
        assert_eq!(
            focus.point.map(|p| (p.x, p.y)),
            before.point.map(|p| (p.x, p.y))
        );
        assert!(focus.manual_focus);
        assert_eq!(facts["A.ARW"].orientation, 6);
        assert_eq!(af_eye_state(Some(&focus)), af_eye_state(Some(&after)));
    }

    #[test]
    fn a_ready_file_without_facts_or_focus_is_left_alone() {
        let mut facts = HashMap::from([("J.JPG".to_string(), facts(1))]);
        apply_face_ready(
            &mut facts,
            &[ready("J.JPG", &candidate()), ready("B.ARW", &candidate())],
        );
        assert!(facts["J.JPG"].focus.is_none());
        assert!(!facts.contains_key("B.ARW"));
    }

    #[test]
    fn output_of_reads_a_listed_file_from_the_answer() {
        let listing = set(&["A.ARW", "B.ARW"]);
        let found = set(&["A.ARW"]);
        assert_eq!(output_of(Some(&found), &listing, "A.ARW"), Some(Yes));
        assert_eq!(output_of(Some(&found), &listing, "B.ARW"), Some(No));
    }

    #[test]
    fn an_empty_answer_is_every_listed_file_without_an_export() {
        let listing = set(&["A.ARW", "B.ARW"]);
        assert_eq!(output_of(Some(&set(&[])), &listing, "A.ARW"), Some(No));
    }

    #[test]
    fn no_answer_leaves_every_file_unknown() {
        let listing = set(&["A.ARW", "B.ARW"]);
        assert_eq!(output_of(None, &listing, "A.ARW"), None);
    }

    #[test]
    fn a_file_the_answer_was_not_asked_about_is_unknown() {
        let listing = set(&["A.ARW", "B.ARW"]);
        assert_eq!(output_of(Some(&set(&["A.ARW"])), &listing, "C.ARW"), None);
    }

    #[test]
    fn apply_stored_replaces_the_remembered_sections() {
        let stored = Stored {
            flags: vec![Rejected, Picked],
            stars: vec![5, 0],
            labels: vec!["red".into(), "none".into()],
            orientations: vec![Portrait],
            candidates: vec![Neutral, Low],
            eyes: vec![EyesItem::Closed],
        };
        let mut s = state(&[Untagged], &[3], &[], &[], &[], &[], &[]);
        s.apply_stored(&stored);
        assert_eq!(
            s,
            state(
                &[Rejected, Picked],
                &[5, 0],
                &["red", "none"],
                &[Portrait],
                &[Neutral, Low],
                &[EyesItem::Closed],
                &[],
            )
        );
    }

    #[test]
    fn an_empty_stored_filter_restores_all_pass() {
        let mut s = state(
            &[Picked],
            &[1],
            &["blue"],
            &[Landscape],
            &[Good],
            &[EyesItem::Open],
            &[],
        );
        s.apply_stored(&Stored::default());
        assert_eq!(s, none());
        assert!(judgment_passes(&s, &rejected()));
    }

    // `commands.rs`'s three `filter_setting` tests.

    #[test]
    fn a_stored_filter_falls_back_to_empty() {
        let empty = Stored::default();
        assert_eq!(Stored::from_setting(None), empty);
        assert_eq!(Stored::from_setting(Some(&json!("x"))), empty);
        assert_eq!(Stored::from_setting(Some(&json!([]))), empty);
        assert_eq!(
            Stored::from_setting(Some(&json!({"flags": "picked", "stars": {"0": 1}}))),
            empty
        );
    }

    #[test]
    fn a_stored_filter_drops_unknown_members() {
        let stored = Stored::from_setting(Some(&json!({
            "flags": ["picked", "bogus", "picked"],
            "stars": [5, 7, "3", 2.5, 0, 6],
            "labels": ["Red", "red", "none"],
            "orientations": ["square", "landscape"],
            "candidates": ["ok", "not_candidate", "unknown", "good", "neutral"],
            "eyes": ["closed", 1],
        })));
        assert_eq!(
            stored,
            Stored {
                flags: vec![Picked],
                stars: vec![5, 0],
                labels: vec!["red".into(), "none".into()],
                orientations: vec![Landscape],
                candidates: vec![Good, Neutral],
                eyes: vec![EyesItem::Closed],
            }
        );
    }

    #[test]
    fn a_valid_stored_filter_is_kept_and_extra_keys_dropped() {
        let valid = Stored {
            flags: vec![Rejected, Picked],
            stars: vec![3, 1],
            labels: vec!["purple".into(), "green".into()],
            orientations: vec![Portrait],
            candidates: vec![Low],
            eyes: vec![EyesItem::Unknown, EyesItem::Open],
        };
        let value = json!({
            "flags": ["rejected", "picked"],
            "stars": [3, 1],
            "labels": ["purple", "green"],
            "orientations": ["portrait"],
            "candidates": ["low"],
            "eyes": ["unknown", "open"],
        });
        assert_eq!(Stored::from_setting(Some(&value)), valid);
        let mut extra = value.clone();
        extra["exif"] = json!({"ISO": ["100"]});
        assert_eq!(Stored::from_setting(Some(&extra)), valid);
    }

    fn exif_facts() -> Facts {
        Facts {
            exif: Some(Exif {
                camera: Some("ILCE-7RM5".into()),
                iso: Some(riffle_backend::exif::Labeled {
                    value: 100.,
                    label: "ISO 100".into(),
                }),
                ..Exif::default()
            }),
            ..facts(1)
        }
    }

    fn exif_state(checks: &[(Group, &[&str])]) -> FilterState {
        let mut s = none();
        for (group, labels) in checks {
            for label in *labels {
                s.toggle_exif(*group, label);
            }
        }
        s
    }

    fn exif_passes(s: &FilterState, facts: Option<&Facts>) -> bool {
        passes(s, &unjudged(), facts, NEUTRAL, None, None)
    }

    #[test]
    fn an_exif_selection_matches_by_label_and_fails_a_file_without_exif() {
        let f = exif_facts();
        let s = exif_state(&[(Group::Camera, &["ILCE-7RM5"])]);
        assert!(s.is_active());
        assert!(exif_passes(&s, Some(&f)));
        assert!(!exif_passes(
            &exif_state(&[(Group::Camera, &["Other"])]),
            Some(&f)
        ));
        assert!(!exif_passes(&s, Some(&facts(1))));
        assert!(!exif_passes(&s, None));
    }

    #[test]
    fn exif_groups_or_within_and_and_across() {
        let f = exif_facts();
        let either = exif_state(&[(Group::Camera, &["ILCE-7RM5", "ILCE-1"])]);
        assert!(exif_passes(&either, Some(&f)));
        let both = exif_state(&[(Group::Camera, &["ILCE-7RM5"]), (Group::Iso, &["ISO 6400"])]);
        assert!(!exif_passes(&both, Some(&f)));
        let missing = exif_state(&[(Group::Lens, &["FE 35mm F1.4 GM"])]);
        assert!(!exif_passes(&missing, Some(&f)));
    }

    #[test]
    fn apply_stored_leaves_the_exif_groups_alone() {
        let mut s = exif_state(&[(Group::Camera, &["ILCE-7RM5"])]);
        let stored = Stored {
            flags: vec![Picked],
            ..Stored::default()
        };
        s.apply_stored(&stored);
        assert_eq!(
            s.exif.get(&Group::Camera),
            Some(&BTreeSet::from(["ILCE-7RM5".to_owned()]))
        );
        assert_eq!(s.flags, HashSet::from([Picked]));
    }

    #[test]
    fn toggling_an_exif_label_twice_leaves_no_empty_group() {
        let mut s = none();
        s.toggle_exif(Group::Iso, "ISO 100");
        s.toggle_exif(Group::Iso, "ISO 100");
        assert_eq!(s, none());
        assert!(!s.is_active());
    }

    #[test]
    fn prune_drops_the_labels_the_folder_no_longer_has() {
        let mut s = exif_state(&[
            (Group::Camera, &["ILCE-7RM5", "ILCE-1"]),
            (Group::Iso, &["ISO 100"]),
            (Group::Lens, &["Gone"]),
        ]);
        s.flags.insert(Picked);
        let groups = Groups(vec![
            (Group::Camera, vec!["ILCE-7RM5".into()]),
            (Group::Iso, vec!["ISO 100".into(), "ISO 200".into()]),
        ]);
        s.prune_exif(&groups);
        let mut want = exif_state(&[(Group::Camera, &["ILCE-7RM5"]), (Group::Iso, &["ISO 100"])]);
        want.flags.insert(Picked);
        assert_eq!(s, want);
        s.prune_exif(&Groups::default());
        assert!(s.exif.is_empty());
        assert_eq!(s.flags, HashSet::from([Picked]));
    }

    #[test]
    fn clear_exif_unchecks_every_group_only() {
        let mut s = exif_state(&[(Group::Camera, &["ILCE-7RM5"])]);
        s.outputs.insert(No);
        s.clear_exif();
        assert!(s.exif.is_empty());
        assert_eq!(s.outputs, HashSet::from([No]));
    }

    /// `p` under `untagged` hides the shown `b`: the shown index must not
    /// carry over to `c`, which now sits where `b` was.
    #[test]
    fn a_hidden_shown_file_has_no_index_in_the_new_list() {
        let old = paths(&["a", "b", "c"]);
        let next = paths(&["a", "c"]);
        assert_eq!(shown_after(&old, &next, Some(1)), None);
        assert_eq!(shown_after(&old, &next, Some(2)), Some(1));
        assert_eq!(shown_after(&next, &paths(&["c"]), None), None);
    }

    #[test]
    fn a_menu_change_on_an_empty_list_lands_on_the_file_shown_before() {
        let mut kept = KeptAnchor::default();
        // `p` on `b` under `untagged` hides the last file.
        kept.refiltered(Some("b"), true);
        // A refilter on the still-empty list has no current file.
        kept.refiltered(None, true);
        let anchor = kept.anchor(None);
        assert_eq!(anchor.as_deref(), Some("b"));
        let landed = anchor_after_filter(&paths(ALL), |_| true, anchor.as_deref(), None);
        assert_eq!(landed.as_deref(), Some("b"));
        let landed = anchor_after_filter(&paths(ALL), |p| p != "b", anchor.as_deref(), None);
        assert_eq!(landed.as_deref(), Some("c"));
    }

    #[test]
    fn a_kept_anchor_no_longer_listed_falls_back_to_the_first_file() {
        let mut kept = KeptAnchor::default();
        kept.refiltered(Some("z"), true);
        let next = paths(&["a", "b"]);
        let landed = anchor_after_filter(&next, |_| true, kept.anchor(None).as_deref(), None);
        let at = landed
            .and_then(|a| next.iter().position(|p| *p == a))
            .unwrap_or(0);
        assert_eq!(at, 0);
    }

    #[test]
    fn the_current_file_wins_and_a_non_empty_list_forgets_the_anchor() {
        let mut kept = KeptAnchor::default();
        kept.refiltered(Some("b"), true);
        assert_eq!(kept.anchor(Some("c".into())).as_deref(), Some("c"));
        kept.refiltered(Some("c"), false);
        assert_eq!(kept.anchor(None), None);
        kept.refiltered(Some("b"), true);
        kept.clear();
        assert_eq!(kept.anchor(None), None);
    }
}
