//! Undo and redo of the judgments and the moves to the Trash, a port of
//! `undo.ts`'s `History` and the Tauri `step`: an entry is a batch of files
//! with their state before the change, undone as one, or a run to the Trash,
//! undone by restoring it.

use std::collections::{HashSet, VecDeque};

use riffle_backend::trash::Restored;

use crate::judgment::State;
use crate::trash::{self, What};

/// The bound `main.ts` gives both of its histories.
pub const LIMIT: usize = 100;

/// A bounded stack, used for both undo and redo: pushing past the limit
/// drops the oldest entry.
pub struct History<T> {
    entries: VecDeque<T>,
    limit: usize,
}

impl<T> History<T> {
    pub fn new(limit: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            limit,
        }
    }

    pub fn push(&mut self, entry: T) {
        self.entries.push_back(entry);
        if self.entries.len() > self.limit {
            self.entries.pop_front();
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.entries.pop_back()
    }

    pub fn peek(&self) -> Option<&T> {
        self.entries.back()
    }

    /// Drop every entry `matches` accepts.
    pub fn remove_where(&mut self, mut matches: impl FnMut(&T) -> bool) {
        self.entries.retain(|entry| !matches(entry));
    }

    pub fn update(&mut self, f: impl FnMut(&mut T)) {
        self.entries.iter_mut().for_each(f);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judged {
    pub path: String,
    pub state: State,
}

/// A judgment undo entry: the files it changed, with their states before.
/// `id` names it for `forget`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batch {
    pub id: u64,
    pub judged: Vec<Judged>,
}

/// One run to the Trash, undone by restoring it: the backend's `run_id`,
/// the RAWs (or JPEGs, or the sidecars of a `Delete Sidecars…` run) it moved,
/// what they were, and the folders it ran in. `id` names the entry, as a
/// batch's does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashEntry {
    pub id: u64,
    pub run_id: u64,
    pub count: usize,
    pub what: What,
    pub dirs: Vec<String>,
}

/// An undo entry: a batch of judgments, or a run to the Trash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Judgments(Batch),
    Trash(TrashEntry),
}

/// The redo entry of the undone run `entry`: the backend keeps only the
/// files that came back, so it counts those alone (the RAWs, or the sidecars
/// of a sidecar run), and there is nothing to redo when none came back.
pub fn undone_trash(entry: &TrashEntry, restored: &Restored) -> Option<TrashEntry> {
    match trash::restored_count(restored, entry.what, entry.count) {
        0 => None,
        count => Some(TrashEntry {
            count,
            ..entry.clone()
        }),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub path: String,
    pub before: State,
    pub after: State,
}

/// What an undo or a redo does: the states to apply, the entry it pushed
/// onto the other history, and the file to move to.
#[derive(Debug, PartialEq, Eq)]
pub struct Stepped {
    pub id: u64,
    pub changes: Vec<Change>,
    /// The single file of a one-file batch; a larger batch leaves the
    /// current file.
    pub move_to: Option<String>,
}

/// Pop the last batch off `from`, keep its paths still `listed`, push their
/// `current` states onto `to` as entry `id`, and give back the states to
/// restore. `None` when `from` is empty, a trash run is on top (left there
/// for the caller, as the Tauri `step` handles it first) or none of the
/// batch's files is listed any more; the popped batch is gone either way.
pub fn step(
    from: &mut History<Entry>,
    to: &mut History<Entry>,
    id: u64,
    listed: impl Fn(&str) -> bool,
    current: impl Fn(&str) -> State,
) -> Option<Stepped> {
    if !matches!(from.peek(), Some(Entry::Judgments(_))) {
        return None;
    }
    let Some(Entry::Judgments(batch)) = from.pop() else {
        return None;
    };
    let changes: Vec<Change> = batch
        .judged
        .into_iter()
        .filter(|j| listed(&j.path))
        .map(|j| Change {
            before: current(&j.path),
            path: j.path,
            after: j.state,
        })
        .collect();
    if changes.is_empty() {
        return None;
    }
    to.push(Entry::Judgments(Batch {
        id,
        judged: changes
            .iter()
            .map(|c| Judged {
                path: c.path.clone(),
                state: c.before.clone(),
            })
            .collect(),
    }));
    let move_to = match changes.as_slice() {
        [only] => Some(only.path.clone()),
        _ => None,
    };
    Some(Stepped {
        id,
        changes,
        move_to,
    })
}

/// The undo and redo histories: the open folder's judgments and the runs to
/// the Trash, which outlive a folder switch.
pub struct Undo {
    undo: History<Entry>,
    redo: History<Entry>,
    next: u64,
}

impl Undo {
    pub fn new() -> Self {
        Self {
            undo: History::new(LIMIT),
            redo: History::new(LIMIT),
            next: 0,
        }
    }

    fn id(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// A new judgment: its files' states before it become the newest undo
    /// entry, and nothing is redoable any more. Its entry's id.
    pub fn record(&mut self, judged: Vec<Judged>) -> u64 {
        let id = self.id();
        self.undo.push(Entry::Judgments(Batch { id, judged }));
        self.redo.clear();
        id
    }

    /// A run to the Trash that moved something: the newest undo entry, and
    /// nothing is redoable any more.
    pub fn record_trash(&mut self, run_id: u64, count: usize, what: What, dirs: Vec<String>) {
        self.push_trash(run_id, count, what, dirs);
        self.redo.clear();
    }

    /// The redo of a run, a new run: the newest undo entry, the rest of the
    /// redo history kept, as a redone batch keeps it.
    pub fn redone_trash(&mut self, run_id: u64, count: usize, what: What, dirs: Vec<String>) {
        self.push_trash(run_id, count, what, dirs);
    }

    fn push_trash(&mut self, run_id: u64, count: usize, what: What, dirs: Vec<String>) {
        let id = self.id();
        self.undo.push(Entry::Trash(TrashEntry {
            id,
            run_id,
            count,
            what,
            dirs,
        }));
    }

    /// The run on top of the redo history (`redo`) or the undo one, which
    /// an undo or a redo restores or moves again instead of a `step`.
    pub fn top_trash(&self, redo: bool) -> Option<TrashEntry> {
        let history = if redo { &self.redo } else { &self.undo };
        match history.peek() {
            Some(Entry::Trash(entry)) => Some(entry.clone()),
            _ => None,
        }
    }

    /// Whether run `id` is still on top of the redo history (`redo`) or the
    /// undo one.
    pub fn on_top(&self, id: u64, redo: bool) -> bool {
        self.top_trash(redo).is_some_and(|entry| entry.id == id)
    }

    /// Drop the run `id` wherever it sits; later entries may have been
    /// pushed since.
    pub fn remove_trash(&mut self, id: u64) {
        for history in [&mut self.undo, &mut self.redo] {
            history.remove_where(|entry| matches!(entry, Entry::Trash(t) if t.id == id));
        }
    }

    /// The undo of `entry` came back with `restored`: it leaves the undo
    /// history, and what came back becomes redoable.
    pub fn undone(&mut self, entry: &TrashEntry, restored: &Restored) {
        self.remove_trash(entry.id);
        if let Some(redo) = undone_trash(entry, restored) {
            let id = self.id();
            self.redo.push(Entry::Trash(TrashEntry { id, ..redo }));
        }
    }

    pub fn undo(
        &mut self,
        listed: impl Fn(&str) -> bool,
        current: impl Fn(&str) -> State,
    ) -> Option<Stepped> {
        let id = self.id();
        step(&mut self.undo, &mut self.redo, id, listed, current)
    }

    pub fn redo(
        &mut self,
        listed: impl Fn(&str) -> bool,
        current: impl Fn(&str) -> State,
    ) -> Option<Stepped> {
        let id = self.id();
        step(&mut self.redo, &mut self.undo, id, listed, current)
    }

    /// The store of `path` from entry `id` failed: drop the file from the
    /// entry, and the entry once it is empty, as `forgetOnFail` does.
    pub fn forget(&mut self, id: u64, path: &str) {
        for history in [&mut self.undo, &mut self.redo] {
            history.update(|entry| {
                if let Entry::Judgments(batch) = entry
                    && batch.id == id
                {
                    batch.judged.retain(|j| j.path != path);
                }
            });
            history.remove_where(is_empty_batch);
        }
    }

    /// The files in `gone` were deleted for good: drop their judgments
    /// from both histories, and every batch left empty, as `withoutPaths`
    /// and `isEmptyBatch` do. Undoing one would judge a path that no
    /// longer exists.
    pub fn without_paths(&mut self, gone: &HashSet<String>) {
        for history in [&mut self.undo, &mut self.redo] {
            history.update(|entry| {
                if let Entry::Judgments(batch) = entry {
                    batch.judged.retain(|j| !gone.contains(&j.path));
                }
            });
            history.remove_where(is_empty_batch);
        }
    }

    /// Another folder opened: its judgments go, and the runs to the Trash
    /// stay, to bring their files back into the folder they left.
    pub fn drop_judgments(&mut self) {
        for history in [&mut self.undo, &mut self.redo] {
            history.remove_where(|entry| matches!(entry, Entry::Judgments(_)));
        }
    }
}

fn is_empty_batch(entry: &Entry) -> bool {
    matches!(entry, Entry::Judgments(batch) if batch.judged.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{Batch, Change, Entry, History, Judged, TrashEntry, Undo, step, undone_trash};
    use crate::judgment::State;
    use crate::trash::What;
    use riffle_backend::trash::{Failure, Restored};
    use riffle_core::Flag;
    use std::collections::{HashMap, HashSet};

    fn batch(id: u64, judged: Vec<Judged>) -> Entry {
        Entry::Judgments(Batch { id, judged })
    }

    fn stars(n: u8) -> State {
        State {
            rating: Some(n),
            ..State::default()
        }
    }

    fn judged(path: &str, state: State) -> Judged {
        Judged {
            path: path.into(),
            state,
        }
    }

    #[test]
    fn pops_in_reverse_push_order_and_is_empty_afterwards() {
        let mut history = History::new(10);
        history.push(1);
        history.push(2);
        assert_eq!(history.pop(), Some(2));
        assert_eq!(history.pop(), Some(1));
        assert_eq!(history.pop(), None);
    }

    #[test]
    fn drops_the_oldest_entry_past_the_limit() {
        let mut history = History::new(2);
        history.push(1);
        history.push(2);
        history.push(3);
        assert_eq!(history.pop(), Some(3));
        assert_eq!(history.pop(), Some(2));
        assert_eq!(history.pop(), None);
    }

    #[test]
    fn remove_where_drops_every_matching_entry() {
        let mut history = History::new(10);
        history.push("a");
        history.push("b");
        history.push("a");
        history.remove_where(|entry| *entry == "a");
        assert_eq!(history.pop(), Some("b"));
        assert_eq!(history.pop(), None);
    }

    #[test]
    fn a_batch_pushes_and_pops_as_one_entry() {
        let mut history = History::new(10);
        let single = vec!["a"];
        let batch = vec!["b", "c"];
        history.push(single.clone());
        history.push(batch.clone());
        assert_eq!(history.pop(), Some(batch));
        assert_eq!(history.pop(), Some(single));
        assert_eq!(history.pop(), None);
    }

    #[test]
    fn clear_empties_it() {
        let mut history = History::new(10);
        history.push(1);
        history.clear();
        assert_eq!(history.pop(), None);
    }

    #[test]
    fn peek_shows_the_top_without_popping_it() {
        let mut history = History::new(10);
        assert_eq!(history.peek(), None);
        history.push(1);
        assert_eq!(history.peek(), Some(&1));
        assert_eq!(history.pop(), Some(1));
    }

    #[test]
    fn a_step_restores_the_batch_and_pushes_the_current_states() {
        let mut from = History::new(10);
        let mut to = History::new(10);
        from.push(batch(1, vec![judged("a", State::default())]));
        let stepped = step(&mut from, &mut to, 2, |_| true, |_| stars(3)).unwrap();
        assert_eq!(
            stepped.changes,
            vec![Change {
                path: "a".into(),
                before: stars(3),
                after: State::default(),
            }]
        );
        assert_eq!(stepped.move_to.as_deref(), Some("a"));
        assert_eq!(to.pop(), Some(batch(2, vec![judged("a", stars(3))])));
        assert_eq!(from.pop(), None);
    }

    #[test]
    fn a_batch_of_more_than_one_file_leaves_the_current_file() {
        let mut from = History::new(10);
        let mut to = History::new(10);
        from.push(batch(1, vec![judged("a", stars(1)), judged("b", stars(2))]));
        let stepped = step(&mut from, &mut to, 2, |_| true, |_| State::default()).unwrap();
        assert_eq!(stepped.changes.len(), 2);
        assert_eq!(stepped.move_to, None);
    }

    #[test]
    fn drops_the_files_no_longer_listed() {
        let mut from = History::new(10);
        let mut to = History::new(10);
        from.push(batch(
            1,
            vec![judged("a", stars(1)), judged("gone", stars(2))],
        ));
        let stepped = step(&mut from, &mut to, 2, |p| p != "gone", |_| State::default()).unwrap();
        assert_eq!(stepped.changes.len(), 1);
        assert_eq!(stepped.move_to.as_deref(), Some("a"));
    }

    #[test]
    fn a_popped_batch_whose_only_file_is_gone_is_skipped() {
        let mut from = History::new(10);
        let mut to = History::new(10);
        from.push(batch(1, vec![judged("a", stars(1))]));
        from.push(batch(2, vec![judged("gone", stars(2))]));
        assert_eq!(
            step(&mut from, &mut to, 3, |p| p != "gone", |_| State::default()),
            None
        );
        assert_eq!(to.pop(), None);
        assert_eq!(from.pop(), Some(batch(1, vec![judged("a", stars(1))])));
    }

    /// The files' states as the view would keep them, with the undo and redo
    /// keys run through `Undo` and applied back.
    struct View {
        states: HashMap<String, State>,
        undo: Undo,
    }

    impl View {
        fn new() -> Self {
            Self {
                states: HashMap::new(),
                undo: Undo::new(),
            }
        }

        fn state(&self, path: &str) -> State {
            self.states.get(path).cloned().unwrap_or_default()
        }

        fn judge(&mut self, path: &str, after: State) -> u64 {
            let id = self.undo.record(vec![judged(path, self.state(path))]);
            self.states.insert(path.into(), after);
            id
        }

        fn apply(&mut self, stepped: Option<super::Stepped>) -> Option<String> {
            let stepped = stepped?;
            for change in stepped.changes {
                self.states.insert(change.path, change.after);
            }
            stepped.move_to
        }

        fn undo(&mut self) -> Option<String> {
            let states = self.states.clone();
            let stepped = self
                .undo
                .undo(|_| true, |p| states.get(p).cloned().unwrap_or_default());
            self.apply(stepped)
        }

        fn redo(&mut self) -> Option<String> {
            let states = self.states.clone();
            let stepped = self
                .undo
                .redo(|_| true, |p| states.get(p).cloned().unwrap_or_default());
            self.apply(stepped)
        }
    }

    #[test]
    fn undo_twice_then_redo_follows_the_manual_check() {
        let mut view = View::new();
        view.judge("1", stars(3));
        let pick = State {
            flag: Flag::Pick,
            ..State::default()
        };
        view.judge("2", pick.clone());
        assert_eq!(view.undo().as_deref(), Some("2"));
        assert_eq!(view.state("2"), State::default());
        assert_eq!(view.undo().as_deref(), Some("1"));
        assert_eq!(view.state("1"), State::default());
        assert_eq!(view.redo().as_deref(), Some("1"));
        assert_eq!(view.state("1"), stars(3));
        assert_eq!(view.redo().as_deref(), Some("2"));
        assert_eq!(view.state("2"), pick);
        assert_eq!(view.redo(), None);
    }

    #[test]
    fn redo_is_cleared_by_a_new_judgment() {
        let mut view = View::new();
        view.judge("1", stars(3));
        view.undo();
        view.judge("2", stars(1));
        assert_eq!(view.redo(), None);
        assert_eq!(view.state("1"), State::default());
    }

    #[test]
    fn undo_after_a_folder_switch_is_empty() {
        let mut view = View::new();
        view.judge("1", stars(3));
        view.undo();
        view.judge("2", stars(1));
        view.undo.drop_judgments();
        assert_eq!(view.undo(), None);
        assert_eq!(view.redo(), None);
        assert_eq!(view.state("2"), stars(1));
    }

    #[test]
    fn a_failed_store_drops_its_file_and_the_emptied_entry() {
        let mut view = View::new();
        view.judge("1", stars(3));
        let failed = view.judge("2", stars(4));
        view.undo.forget(failed, "2");
        assert_eq!(view.undo().as_deref(), Some("1"));
        assert_eq!(view.undo(), None);
    }

    #[test]
    fn a_failed_undo_drops_its_file_from_the_redo_entry() {
        let mut view = View::new();
        view.judge("1", stars(3));
        let states = view.states.clone();
        let stepped = view
            .undo
            .undo(|_| true, |p| states.get(p).cloned().unwrap_or_default())
            .unwrap();
        view.undo.forget(stepped.id, "1");
        assert_eq!(view.redo(), None);
    }

    #[test]
    fn dropping_deleted_paths_empties_a_batch_of_them_in_both_histories() {
        let mut undo = Undo::new();
        undo.record(vec![judged("1", stars(1)), judged("2", stars(2))]);
        undo.record(vec![judged("1", stars(3))]);
        undo.record(vec![judged("1", stars(4))]);
        let current = |_: &str| State::default();
        assert!(undo.undo(|_| true, current).is_some());
        undo.without_paths(&HashSet::from(["1".to_owned()]));
        assert_eq!(undo.redo(|_| true, current), None);
        let left = undo.undo(|_| true, current).unwrap();
        assert_eq!(
            left.changes
                .iter()
                .map(|c| c.path.as_str())
                .collect::<Vec<_>>(),
            vec!["2"]
        );
        assert_eq!(undo.undo(|_| true, current), None);
    }

    fn restored(restored: &[&str], failed: usize) -> Restored {
        Restored {
            restored: restored.iter().map(|p| (*p).to_owned()).collect(),
            failed: (0..failed)
                .map(|i| Failure {
                    path: format!("/a/{i}.xmp"),
                    message: "gone".to_owned(),
                })
                .collect(),
            back: Vec::new(),
        }
    }

    fn run() -> TrashEntry {
        TrashEntry {
            id: 9,
            run_id: 1,
            count: 2,
            what: What::Files,
            dirs: vec!["/a".to_owned()],
        }
    }

    #[test]
    fn an_undone_run_is_redone_over_the_raws_that_came_back_only() {
        assert_eq!(
            undone_trash(&run(), &restored(&["/a/1.ARW"], 1)),
            Some(TrashEntry { count: 1, ..run() })
        );
        assert_eq!(undone_trash(&run(), &restored(&[], 0)), None);
    }

    /// `undo.test.ts`'s sidecar run case.
    #[test]
    fn an_undone_sidecar_run_is_redone_over_the_sidecars_that_did_not_fail() {
        let sidecars = TrashEntry {
            count: 3,
            what: What::Sidecars,
            ..run()
        };
        assert_eq!(
            undone_trash(&sidecars, &restored(&[], 1)),
            Some(TrashEntry {
                count: 2,
                ..sidecars.clone()
            })
        );
        assert_eq!(
            undone_trash(&sidecars, &restored(&[], 0)),
            Some(sidecars.clone())
        );
        assert_eq!(undone_trash(&sidecars, &restored(&[], 3)), None);
    }

    #[test]
    fn a_step_leaves_a_trash_run_on_top_for_the_caller() {
        let mut from = History::new(10);
        let mut to = History::new(10);
        from.push(batch(1, vec![judged("a", stars(1))]));
        from.push(Entry::Trash(run()));
        assert_eq!(
            step(&mut from, &mut to, 2, |_| true, |_| State::default()),
            None
        );
        assert_eq!(from.peek(), Some(&Entry::Trash(run())));
        assert_eq!(to.pop(), None);
    }

    #[test]
    fn another_folder_drops_the_judgments_and_keeps_the_trash_runs() {
        let mut undo = Undo::new();
        undo.record(vec![judged("/a/1.ARW", stars(1))]);
        undo.record_trash(1, 2, What::Files, vec!["/a".to_owned()]);
        undo.record(vec![judged("/a/2.ARW", stars(2))]);
        undo.drop_judgments();
        let top = undo.top_trash(false).unwrap();
        assert_eq!((top.run_id, top.count), (1, 2));
        undo.remove_trash(top.id);
        assert_eq!(undo.top_trash(false), None);
        assert_eq!(undo.undo(|_| true, |_| State::default()), None);
    }

    #[test]
    fn a_trash_run_clears_the_redo_history_and_a_redone_one_keeps_it() {
        let mut undo = Undo::new();
        undo.record(vec![judged("1", stars(1))]);
        undo.undo(|_| true, |_| State::default());
        undo.record_trash(1, 1, What::Files, vec!["/a".to_owned()]);
        assert_eq!(undo.redo(|_| true, |_| State::default()), None);
        let entry = undo.top_trash(false).unwrap();
        undo.record(vec![judged("2", stars(2))]);
        undo.undo(|_| true, |_| State::default());
        undo.redone_trash(2, 1, What::Files, vec!["/a".to_owned()]);
        assert_ne!(undo.top_trash(false).unwrap().id, entry.id);
        assert!(undo.redo(|_| true, |_| State::default()).is_some());
    }

    #[test]
    fn an_undone_run_moves_to_the_redo_history_when_anything_came_back() {
        let mut undo = Undo::new();
        undo.record_trash(1, 2, What::Files, vec!["/a".to_owned()]);
        let entry = undo.top_trash(false).unwrap();
        assert!(undo.on_top(entry.id, false));
        undo.record(vec![judged("/a/3.ARW", stars(3))]);
        assert!(!undo.on_top(entry.id, false));
        undo.undone(&entry, &restored(&["/a/1.ARW"], 0));
        let redo = undo.top_trash(true).unwrap();
        assert_eq!((redo.run_id, redo.count), (1, 1));
        assert!(undo.on_top(redo.id, true));
        assert!(undo.undo(|_| true, |_| State::default()).is_some());
        assert_eq!(undo.top_trash(false), None);
    }

    #[test]
    fn an_undone_run_with_nothing_back_leaves_nothing_to_redo() {
        let mut undo = Undo::new();
        undo.record_trash(1, 2, What::Files, vec!["/a".to_owned()]);
        let entry = undo.top_trash(false).unwrap();
        undo.undone(&entry, &restored(&[], 2));
        assert_eq!(undo.top_trash(false), None);
        assert_eq!(undo.top_trash(true), None);
    }

    #[test]
    fn dropping_deleted_paths_leaves_a_trash_run_alone() {
        let mut undo = Undo::new();
        undo.record(vec![
            judged("/a/1.ARW", stars(1)),
            judged("/a/2.ARW", stars(2)),
        ]);
        undo.record(vec![judged("/a/1.ARW", stars(3))]);
        undo.record_trash(1, 2, What::Files, vec!["/a".to_owned()]);
        undo.without_paths(&HashSet::from(["/a/1.ARW".to_owned()]));
        let top = undo.top_trash(false).unwrap();
        undo.remove_trash(top.id);
        let left = undo.undo(|_| true, |_| State::default()).unwrap();
        assert_eq!(left.changes.len(), 1);
        assert_eq!(left.changes[0].path, "/a/2.ARW");
        assert_eq!(undo.undo(|_| true, |_| State::default()), None);
    }
}
