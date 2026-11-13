//! Undo and redo of the judgments, a port of `undo.ts`'s `History` and the
//! Tauri `step`: each entry is a batch of files with their state before the
//! change, undone as one.

use std::collections::{HashSet, VecDeque};

use crate::judgment::State;

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

    /// The trash entries of port Step 8 are peeked before they are popped.
    #[allow(dead_code)]
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

/// One undo entry: one file here, several once the selection exists.
/// `id` names it for `forget`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batch {
    pub id: u64,
    pub judged: Vec<Judged>,
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
/// restore. `None` when `from` is empty or none of the batch's files is
/// listed any more; the popped batch is gone either way.
pub fn step(
    from: &mut History<Batch>,
    to: &mut History<Batch>,
    id: u64,
    listed: impl Fn(&str) -> bool,
    current: impl Fn(&str) -> State,
) -> Option<Stepped> {
    let batch = from.pop()?;
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
    to.push(Batch {
        id,
        judged: changes
            .iter()
            .map(|c| Judged {
                path: c.path.clone(),
                state: c.before.clone(),
            })
            .collect(),
    });
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

/// The open folder's undo and redo histories.
pub struct Undo {
    undo: History<Batch>,
    redo: History<Batch>,
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
        self.undo.push(Batch { id, judged });
        self.redo.clear();
        id
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
            history.update(|batch| {
                if batch.id == id {
                    batch.judged.retain(|j| j.path != path);
                }
            });
            history.remove_where(|batch| batch.judged.is_empty());
        }
    }

    /// The files in `gone` were deleted for good: drop their judgments
    /// from both histories, and every batch left empty, as `withoutPaths`
    /// and `isEmptyBatch` do. Undoing one would judge a path that no
    /// longer exists.
    pub fn without_paths(&mut self, gone: &HashSet<String>) {
        for history in [&mut self.undo, &mut self.redo] {
            history.update(|batch| batch.judged.retain(|j| !gone.contains(&j.path)));
            history.remove_where(|batch| batch.judged.is_empty());
        }
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{Batch, Change, History, Judged, Undo, step};
    use crate::judgment::State;
    use riffle_core::Flag;
    use std::collections::{HashMap, HashSet};

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
        from.push(Batch {
            id: 1,
            judged: vec![judged("a", State::default())],
        });
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
        assert_eq!(
            to.pop(),
            Some(Batch {
                id: 2,
                judged: vec![judged("a", stars(3))],
            })
        );
        assert_eq!(from.pop(), None);
    }

    #[test]
    fn a_batch_of_more_than_one_file_leaves_the_current_file() {
        let mut from = History::new(10);
        let mut to = History::new(10);
        from.push(Batch {
            id: 1,
            judged: vec![judged("a", stars(1)), judged("b", stars(2))],
        });
        let stepped = step(&mut from, &mut to, 2, |_| true, |_| State::default()).unwrap();
        assert_eq!(stepped.changes.len(), 2);
        assert_eq!(stepped.move_to, None);
    }

    #[test]
    fn drops_the_files_no_longer_listed() {
        let mut from = History::new(10);
        let mut to = History::new(10);
        from.push(Batch {
            id: 1,
            judged: vec![judged("a", stars(1)), judged("gone", stars(2))],
        });
        let stepped = step(&mut from, &mut to, 2, |p| p != "gone", |_| State::default()).unwrap();
        assert_eq!(stepped.changes.len(), 1);
        assert_eq!(stepped.move_to.as_deref(), Some("a"));
    }

    #[test]
    fn a_popped_batch_whose_only_file_is_gone_is_skipped() {
        let mut from = History::new(10);
        let mut to = History::new(10);
        from.push(Batch {
            id: 1,
            judged: vec![judged("a", stars(1))],
        });
        from.push(Batch {
            id: 2,
            judged: vec![judged("gone", stars(2))],
        });
        assert_eq!(
            step(&mut from, &mut to, 3, |p| p != "gone", |_| State::default()),
            None
        );
        assert_eq!(to.pop(), None);
        assert_eq!(from.pop().map(|b| b.id), Some(1));
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
        view.undo.clear();
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
}
