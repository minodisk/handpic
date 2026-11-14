//! The folder tree's inline rename, kept pure: which typed name is worth
//! sending, the edit deciding once, the rename held until the scan ends and
//! the name its row shows meanwhile, the slow second click that starts an
//! edit, and the folders whose rename is in flight. A port of the folder
//! parts of the Tauri app's `rename.ts`; the window draws the editor with
//! `gpui-component`'s `Input` and runs the rename from `folderops.rs`.

use std::time::{Duration, Instant};

use crate::tree;

/// What ends a live edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Confirm,
    Cancel,
}

/// The decision a key makes on a live edit: Enter confirms, Escape cancels,
/// and every other key (`None`) is the input's own. The native editor takes
/// Enter (`PressEnter`, or the lost focus) and Escape (a key listener over
/// the input) in `folderops.rs`, so this stays a model of that rule.
#[cfg(test)]
pub fn edit_key(key: Option<&str>) -> Option<Decision> {
    match key {
        Some("enter") => Some(Decision::Confirm),
        Some("escape") => Some(Decision::Cancel),
        _ => None,
    }
}

/// The name to rename to, or `None` when there is nothing to rename: an
/// empty name or the current one just ends the edit.
pub fn confirm_name(original: &str, value: &str) -> Option<String> {
    let name = value.trim();
    (!name.is_empty() && name != original).then(|| name.to_owned())
}

/// A live inline rename: the folder, the name it started from (the pending
/// one while a rename of it is held) and whether it has decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineRename {
    pub path: String,
    pub original: String,
    committed: bool,
}

impl InlineRename {
    pub fn new(path: String, original: String) -> Self {
        Self {
            path,
            original,
            committed: false,
        }
    }
}

/// The edit's decision, once: the lost focus that follows an Enter or an
/// Escape (the input is removed, or the focus moves on) must not decide a
/// second time.
pub fn commit(rename: &mut InlineRename, decision: Decision) -> Option<Decision> {
    if rename.committed {
        return None;
    }
    rename.committed = true;
    Some(decision)
}

/// A confirmed rename the gate holds until the scan ends: its row shows
/// `name` in the pending style meanwhile. `id` tells two marks of the same
/// path and name apart, as the Tauri mark's object identity does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    id: u64,
    pub path: String,
    pub name: String,
}

/// A confirmed rename through the gate: the folder in the tree's spelling,
/// the new name, and the mark its row shows while the rename is held (none
/// when it ran at once).
pub struct Held {
    pub path: String,
    pub name: String,
    pub mark: Option<Pending>,
}

pub const PENDING_TITLE: &str = "Renames when the scan finishes";

/// The one held rename the tree shows. Only the mark that set it clears it,
/// so a rename that settles late cannot clear the mark a later rename of the
/// same path set.
#[derive(Default)]
pub struct PendingRename {
    current: Option<Pending>,
    marks: u64,
}

impl PendingRename {
    pub fn mark(&mut self, path: String, name: String) -> Pending {
        self.marks += 1;
        let mark = Pending {
            id: self.marks,
            path,
            name,
        };
        self.current = Some(mark.clone());
        mark
    }

    pub fn clear(&mut self, mark: &Pending) -> bool {
        if self
            .current
            .as_ref()
            .is_none_or(|current| current.id != mark.id)
        {
            return false;
        }
        self.current = None;
        true
    }

    pub fn current(&self) -> Option<&Pending> {
        self.current.as_ref()
    }
}

/// The name a row shows: the pending one while a rename of `path` is held.
pub fn display_name<'a>(pending: Option<&'a Pending>, path: &str, real: &'a str) -> &'a str {
    match pending {
        Some(pending) if pending.path == path => &pending.name,
        _ => real,
    }
}

/// What a confirmed edit does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Keep,
    Cancel,
    Rename(String),
}

/// A confirmed edit on a row whose rename is held: an empty or unchanged
/// (pending) name keeps it, the original name cancels it, any other
/// replaces it.
pub fn pending_outcome(real: &str, pending_name: &str, typed: &str) -> Outcome {
    let name = typed.trim();
    if name.is_empty() || name == pending_name {
        Outcome::Keep
    } else if name == real {
        Outcome::Cancel
    } else {
        Outcome::Rename(name.to_owned())
    }
}

/// A confirmed edit's outcome, held rename or not; `Keep` leaves things as
/// they are.
pub fn edit_outcome(pending: Option<&Pending>, path: &str, real: &str, typed: &str) -> Outcome {
    match pending {
        Some(pending) if pending.path == path => pending_outcome(real, &pending.name, typed),
        _ => confirm_name(real, typed).map_or(Outcome::Keep, Outcome::Rename),
    }
}

pub const SLOW_CLICK_DELAY: Duration = Duration::from_millis(500);

/// A click on the name of the folder that is already open arms a rename,
/// which starts `SLOW_CLICK_DELAY` later unless something disarms it first
/// (a double-click, another press, a key).
#[derive(Default)]
pub struct SlowClick {
    armed: Option<(String, Instant)>,
}

impl SlowClick {
    pub fn click(&mut self, path: &str, already_current: bool, now: Instant) -> bool {
        self.armed = already_current.then(|| (path.to_owned(), now));
        already_current
    }

    pub fn cancel(&mut self) {
        self.armed = None;
    }

    pub fn due(&mut self, path: &str, now: Instant) -> bool {
        match &self.armed {
            Some((armed, at)) if armed == path && now.duration_since(*at) >= SLOW_CLICK_DELAY => {
                self.armed = None;
                true
            }
            _ => false,
        }
    }
}

/// The folders whose rename is in flight: until it settles, the tree still
/// draws them, and everything under them, at paths about to move, so opening
/// or renaming one of those is refused.
#[derive(Default)]
pub struct RenamesInFlight {
    paths: Vec<String>,
}

impl RenamesInFlight {
    pub fn start(&mut self, path: String) {
        self.paths.push(path);
    }

    pub fn settle(&mut self, path: &str) {
        if let Some(at) = self.paths.iter().position(|p| p == path) {
            self.paths.remove(at);
        }
    }

    pub fn blocks(&self, path: &str, ignore_case: bool) -> bool {
        self.paths
            .iter()
            .any(|dir| tree::relation(path, dir, ignore_case).is_some())
    }
}

pub const RENAMING: &str = "A folder is being renamed; try again in a moment.";
pub const RENAME_CANCELED: &str = "Rename\u{2026} was canceled";
pub const RENAME_REFUSED: &str = "Rename\u{2026} is unavailable while an import runs";
/// The gate's label of a held rename.
pub const LABEL: &str = "Rename\u{2026}";

/// What the status says when the held rename of `held` is dropped, as
/// `main.ts`'s `holdRename` cancel: nothing when a rename of the same folder
/// replaced it (a re-edit of the pending name), else that it was canceled.
pub fn dropped_status(held: &str, by: Option<&str>) -> Option<&'static str> {
    (by != Some(held)).then_some(RENAME_CANCELED)
}

/// Ported from `rename.test.ts`. Left out: `stemLength` and the `file` kind
/// of `inlineRename`, with the case of a file's rename blocking only that
/// file, since the strip's `Rename…` is not ported; `inlineRename`'s `value`,
/// which the text input holds.
#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        Decision, InlineRename, Outcome, Pending, PendingRename, RENAME_CANCELED, RenamesInFlight,
        SLOW_CLICK_DELAY, SlowClick, commit, confirm_name, display_name, dropped_status, edit_key,
        edit_outcome, pending_outcome,
    };

    #[test]
    fn a_held_rename_replaced_by_a_rename_of_the_same_folder_says_nothing() {
        assert_eq!(dropped_status("/a/b", Some("/a/b")), None);
    }

    #[test]
    fn a_held_rename_dropped_otherwise_says_it_was_canceled() {
        assert_eq!(dropped_status("/a/b", Some("/a/c")), Some(RENAME_CANCELED));
        assert_eq!(dropped_status("/a/b", None), Some(RENAME_CANCELED));
    }

    #[test]
    fn inline_rename_starts_from_the_original_name_not_committed() {
        let rename = InlineRename::new("/a/b".into(), "b".into());
        assert_eq!(rename.path, "/a/b");
        assert_eq!(rename.original, "b");
        assert!(!rename.committed);
    }

    #[test]
    fn enter_confirms_escape_cancels_anything_else_is_the_inputs() {
        assert_eq!(edit_key(Some("enter")), Some(Decision::Confirm));
        assert_eq!(edit_key(Some("escape")), Some(Decision::Cancel));
        assert_eq!(edit_key(Some("arrowleft")), None);
        assert_eq!(edit_key(Some("a")), None);
        assert_eq!(edit_key(Some("shift+enter")), None);
        assert_eq!(edit_key(None), None);
    }

    #[test]
    fn confirm_name_trims_the_typed_name() {
        assert_eq!(confirm_name("old", "  new "), Some("new".into()));
    }

    #[test]
    fn an_empty_or_unchanged_name_renames_nothing() {
        assert_eq!(confirm_name("old", ""), None);
        assert_eq!(confirm_name("old", "   "), None);
        assert_eq!(confirm_name("old", "old"), None);
        assert_eq!(confirm_name("old", " old "), None);
    }

    #[test]
    fn a_case_only_change_is_a_rename() {
        assert_eq!(confirm_name("Photos", "photos"), Some("photos".into()));
    }

    #[test]
    fn commit_answers_the_first_decision_only() {
        let mut rename = InlineRename::new("/a/b".into(), "b".into());
        assert_eq!(
            commit(&mut rename, Decision::Confirm),
            Some(Decision::Confirm)
        );
        assert_eq!(commit(&mut rename, Decision::Confirm), None);
    }

    #[test]
    fn the_lost_focus_after_an_escape_does_not_confirm() {
        let mut rename = InlineRename::new("/a/b".into(), "b".into());
        assert_eq!(
            commit(&mut rename, Decision::Cancel),
            Some(Decision::Cancel)
        );
        assert_eq!(commit(&mut rename, Decision::Confirm), None);
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_first_click_on_a_folder_that_is_not_open_does_not_arm() {
        let t = Instant::now();
        let mut slow = SlowClick::default();
        assert!(!slow.click("/a", false, t));
        assert!(!slow.due("/a", t + SLOW_CLICK_DELAY));
    }

    #[test]
    fn a_click_on_the_open_folder_arms_due_after_the_delay() {
        let t = Instant::now();
        let mut slow = SlowClick::default();
        assert!(slow.click("/a", true, t + ms(100)));
        assert!(slow.due("/a", t + ms(100) + SLOW_CLICK_DELAY));
    }

    #[test]
    fn not_due_before_the_delay() {
        let t = Instant::now();
        let mut slow = SlowClick::default();
        slow.click("/a", true, t + ms(100));
        assert!(!slow.due("/a", t + ms(100) + SLOW_CLICK_DELAY - ms(1)));
    }

    #[test]
    fn a_double_click_or_a_key_cancels_the_arm() {
        let t = Instant::now();
        let mut slow = SlowClick::default();
        slow.click("/a", true, t);
        slow.cancel();
        assert!(!slow.due("/a", t + SLOW_CLICK_DELAY));
        assert!(!slow.due("/a", t + SLOW_CLICK_DELAY * 2));
    }

    #[test]
    fn a_click_on_another_folder_replaces_the_arm() {
        let t = Instant::now();
        let mut slow = SlowClick::default();
        slow.click("/a", true, t);
        slow.click("/b", true, t + ms(10));
        assert!(!slow.due("/a", t + SLOW_CLICK_DELAY + ms(10)));
        assert!(slow.due("/b", t + SLOW_CLICK_DELAY + ms(10)));
    }

    #[test]
    fn due_answers_true_only_once() {
        let t = Instant::now();
        let mut slow = SlowClick::default();
        slow.click("/a", true, t);
        assert!(slow.due("/a", t + SLOW_CLICK_DELAY));
        assert!(!slow.due("/a", t + SLOW_CLICK_DELAY));
    }

    #[test]
    fn blocks_the_renamed_folder_and_everything_under_it() {
        let mut renames = RenamesInFlight::default();
        renames.start(r"D:\photos".into());
        assert!(renames.blocks(r"D:\photos", false));
        assert!(renames.blocks(r"D:\photos\2026", false));
        assert!(renames.blocks(r"D:\photos\2026\01", false));
    }

    #[test]
    fn leaves_a_sibling_sharing_the_names_prefix_and_an_unrelated_folder() {
        let mut renames = RenamesInFlight::default();
        renames.start(r"D:\photos".into());
        assert!(!renames.blocks(r"D:\photos2", false));
        assert!(!renames.blocks(r"D:\photos2\a", false));
        assert!(!renames.blocks(r"E:\other", false));
    }

    #[test]
    fn a_case_only_difference_blocks_only_ignoring_case() {
        let mut renames = RenamesInFlight::default();
        renames.start("/Users/me/Photos".into());
        assert!(renames.blocks("/Users/me/photos/a", true));
        assert!(!renames.blocks("/Users/me/photos/a", false));
    }

    #[test]
    fn settling_the_rename_clears_the_block() {
        let mut renames = RenamesInFlight::default();
        renames.start("/a/photos".into());
        renames.settle("/a/photos");
        assert!(!renames.blocks("/a/photos", false));
        assert!(!renames.blocks("/a/photos/b", false));
    }

    #[test]
    fn settling_one_of_two_renames_leaves_the_others_block() {
        let mut renames = RenamesInFlight::default();
        renames.start("/a/one".into());
        renames.start("/a/two".into());
        renames.settle("/a/one");
        assert!(!renames.blocks("/a/one/x", false));
        assert!(renames.blocks("/a/two/x", false));
    }

    #[test]
    fn settling_a_path_never_started_is_a_no_op() {
        let mut renames = RenamesInFlight::default();
        renames.start("/a/one".into());
        renames.settle("/a/other");
        assert!(renames.blocks("/a/one", false));
    }

    #[test]
    fn a_mark_is_cleared_only_by_itself_once() {
        let mut pending = PendingRename::default();
        let a = pending.mark("/a/b".into(), "c".into());
        let b = pending.mark("/a/b".into(), "d".into());
        assert!(!pending.clear(&a));
        assert_eq!(display_name(pending.current(), "/a/b", "b"), "d");
        assert!(pending.clear(&b));
        assert_eq!(pending.current(), None);
        assert_eq!(display_name(pending.current(), "/a/b", "b"), "b");
        assert!(!pending.clear(&b));
    }

    fn held(path: &str, name: &str) -> Pending {
        let mut pending = PendingRename::default();
        pending.mark(path.into(), name.into())
    }

    #[test]
    fn display_name_shows_the_pending_name_for_the_path_it_is_held_for() {
        let pending = held("/a/b", "c");
        assert_eq!(display_name(Some(&pending), "/a/b", "b"), "c");
    }

    #[test]
    fn display_name_shows_the_real_name_for_any_other_path_or_with_nothing_held() {
        let pending = held("/a/b", "c");
        assert_eq!(display_name(Some(&pending), "/a/x", "x"), "x");
        assert_eq!(display_name(None, "/a/b", "b"), "b");
    }

    #[test]
    fn an_empty_or_unchanged_pending_name_keeps_the_held_rename() {
        assert_eq!(pending_outcome("b", "c", ""), Outcome::Keep);
        assert_eq!(pending_outcome("b", "c", "  "), Outcome::Keep);
        assert_eq!(pending_outcome("b", "c", " c "), Outcome::Keep);
    }

    #[test]
    fn the_original_name_cancels_the_held_rename() {
        assert_eq!(pending_outcome("b", "c", "b"), Outcome::Cancel);
        assert_eq!(pending_outcome("b", "c", " b "), Outcome::Cancel);
    }

    #[test]
    fn any_other_name_replaces_it() {
        assert_eq!(
            pending_outcome("b", "c", " d "),
            Outcome::Rename("d".into())
        );
    }

    #[test]
    fn edit_outcome_follows_pending_outcome_on_the_row_whose_rename_is_held() {
        let pending = held("/a/b", "c");
        assert_eq!(
            edit_outcome(Some(&pending), "/a/b", "b", "c"),
            Outcome::Keep
        );
        assert_eq!(
            edit_outcome(Some(&pending), "/a/b", "b", "b"),
            Outcome::Cancel
        );
        assert_eq!(
            edit_outcome(Some(&pending), "/a/b", "b", "d"),
            Outcome::Rename("d".into())
        );
    }

    #[test]
    fn edit_outcome_follows_confirm_name_on_any_other_row() {
        let pending = held("/a/b", "c");
        assert_eq!(
            edit_outcome(Some(&pending), "/a/x", "x", "x"),
            Outcome::Keep
        );
        assert_eq!(edit_outcome(None, "/a/x", "x", ""), Outcome::Keep);
        assert_eq!(
            edit_outcome(None, "/a/x", "x", " y "),
            Outcome::Rename("y".into())
        );
    }
}
