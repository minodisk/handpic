//! The strip's multi-selection, a port of `selection.ts`: which files a
//! judgment applies to. Keyed by listed path, because `refilter` rebuilds
//! `files`; the focused file (`files[focused]`) is always a member.

use std::collections::HashSet;

use crate::{
    history::Change,
    judgment::{self, Command, State},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub selected: HashSet<String>,
    pub anchor: Option<String>,
}

/// A click's modifiers: Ctrl (Cmd on macOS) toggles, Shift selects a range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub toggle: bool,
    pub range: bool,
}

pub fn single(path: Option<&str>) -> Selection {
    Selection {
        selected: path.into_iter().map(str::to_owned).collect(),
        anchor: path.map(str::to_owned),
    }
}

/// `paths` selected, the first one the anchor.
#[allow(dead_code, reason = "the MCP companion's select_photos")]
pub fn selection_of(paths: &[String]) -> Selection {
    Selection {
        selected: paths.iter().cloned().collect(),
        anchor: paths.first().cloned(),
    }
}

fn between(files: &[String], from: usize, to: usize) -> HashSet<String> {
    files[from.min(to)..=from.max(to)].iter().cloned().collect()
}

fn position(files: &[String], path: &str) -> Option<usize> {
    files.iter().position(|f| f == path)
}

/// A click on `files[at]` while `files[focused]` is focused.
pub fn click(
    selection: &Selection,
    files: &[String],
    focused: usize,
    at: usize,
    Modifiers { toggle, range }: Modifiers,
) -> Selection {
    let Some(path) = files.get(at) else {
        return selection.clone();
    };
    if range {
        let from = selection
            .anchor
            .as_deref()
            .and_then(|anchor| position(files, anchor));
        return match from {
            Some(from) => Selection {
                selected: between(files, from, at),
                anchor: selection.anchor.clone(),
            },
            None => single(Some(path)),
        };
    }
    if toggle {
        if at == focused {
            return selection.clone();
        }
        let mut selected = selection.selected.clone();
        if !selected.remove(path) {
            selected.insert(path.clone());
        }
        return Selection {
            selected,
            anchor: Some(path.clone()),
        };
    }
    single(Some(path))
}

/// Moves the focus by `delta` and selects the range from the anchor to it;
/// the new selection and the new focused index.
pub fn extend(
    selection: &Selection,
    files: &[String],
    focused: usize,
    delta: isize,
) -> (Selection, usize) {
    let Some(last) = files.len().checked_sub(1) else {
        return (selection.clone(), focused);
    };
    let index = focused.saturating_add_signed(delta).min(last);
    let anchor = selection
        .anchor
        .as_deref()
        .filter(|anchor| files.iter().any(|f| f == anchor))
        .or(files.get(focused).map(String::as_str));
    let Some(anchor) = anchor else {
        return (selection.clone(), focused);
    };
    let from = position(files, anchor).unwrap_or(index);
    (
        Selection {
            selected: between(files, from, index),
            anchor: Some(anchor.to_owned()),
        },
        index,
    )
}

/// Every file selected, anchored on the focused one.
pub fn all(files: &[String], focused: usize) -> Selection {
    if files.is_empty() {
        return single(None);
    }
    Selection {
        selected: files.iter().cloned().collect(),
        anchor: files.get(focused).cloned(),
    }
}

/// Drops the paths no longer in `files`; the anchor falls back to the
/// focused file when it is dropped.
pub fn prune(selection: &Selection, files: &[String], focused: usize) -> Selection {
    let visible: HashSet<&str> = files.iter().map(String::as_str).collect();
    let mut selected: HashSet<String> = selection
        .selected
        .iter()
        .filter(|path| visible.contains(path.as_str()))
        .cloned()
        .collect();
    let path = files.get(focused);
    if let Some(path) = path {
        selected.insert(path.clone());
    }
    let anchor = match &selection.anchor {
        Some(anchor) if visible.contains(anchor.as_str()) => Some(anchor.clone()),
        _ => path.cloned(),
    };
    Selection { selected, anchor }
}

/// The selection `refilter` ends with: a landing on a resumed file selects
/// it alone, as an arrow key would leave it; otherwise `prune`.
pub fn settle(selection: &Selection, files: &[String], focused: usize, resumed: bool) -> Selection {
    if resumed {
        single(files.get(focused).map(String::as_str))
    } else {
        prune(selection, files, focused)
    }
}

/// A one-file undo / redo that restores `path` while `shown` was focused:
/// when it moves the focus onto a listed `path`, that file alone is
/// selected; otherwise the selection stays.
pub fn restore(
    selection: &Selection,
    files: &[String],
    shown: Option<&str>,
    path: &str,
) -> Selection {
    if shown != Some(path) && files.iter().any(|f| f == path) {
        single(Some(path))
    } else {
        selection.clone()
    }
}

/// The files a judgment applies to, in `files` order: the focused file alone
/// when one or none is selected.
pub fn targets(selection: &Selection, files: &[String], focused: usize) -> Vec<String> {
    let Some(path) = files.get(focused) else {
        return Vec::new();
    };
    if selection.selected.len() <= 1 {
        return vec![path.clone()];
    }
    files
        .iter()
        .filter(|f| selection.selected.contains(*f))
        .cloned()
        .collect()
}

/// The changes `cmd` makes to `paths`, focused file first: its value is
/// decided once from the focused file and applied to each file's own state.
/// A file is left out unless `judgment::writes` holds for it, so one already
/// at its new state is skipped, unless `seen` says the view has not been
/// told its entry and the command is `ClearAll`.
pub fn judgments(
    paths: &[String],
    focused: &str,
    lookup: impl Fn(&str) -> State,
    cmd: Command,
    seen: impl Fn(&str) -> bool,
) -> Vec<Change> {
    let value = lookup(focused);
    std::iter::once(focused)
        .chain(paths.iter().map(String::as_str).filter(|p| *p != focused))
        .filter_map(|path| {
            let before = lookup(path);
            let after = judgment::apply(&value, &before, cmd);
            judgment::writes(seen(path), &before, &after, cmd).then(|| Change {
                path: path.to_owned(),
                before,
                after,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        Modifiers, Selection, all, click, extend, judgments, prune, restore, selection_of, settle,
        single, targets,
    };
    use crate::judgment::{Command, State};
    use riffle_core::Flag;
    use std::collections::HashMap;

    fn files() -> Vec<String> {
        ["/a", "/b", "/c", "/d", "/e"].map(String::from).to_vec()
    }

    fn sel(selected: &[&str], anchor: Option<&str>) -> Selection {
        Selection {
            selected: selected.iter().map(|p| p.to_string()).collect(),
            anchor: anchor.map(String::from),
        }
    }

    const PLAIN: Modifiers = Modifiers {
        toggle: false,
        range: false,
    };
    const TOGGLE: Modifiers = Modifiers {
        toggle: true,
        range: false,
    };
    const RANGE: Modifiers = Modifiers {
        toggle: false,
        range: true,
    };

    #[test]
    fn selection_of_selects_the_paths_and_anchors_on_the_first() {
        assert_eq!(
            selection_of(&["/c".into(), "/a".into()]),
            sel(&["/c", "/a"], Some("/c"))
        );
    }

    #[test]
    fn all_selects_every_file_and_anchors_on_the_focused_one() {
        assert_eq!(
            all(&files(), 2),
            sel(&["/a", "/b", "/c", "/d", "/e"], Some("/c"))
        );
    }

    #[test]
    fn all_of_an_empty_list_is_an_empty_selection() {
        assert_eq!(all(&[], 0), sel(&[], None));
    }

    #[test]
    fn a_plain_click_collapses_to_the_file() {
        let multi = sel(&["/a", "/b", "/c"], Some("/a"));
        assert_eq!(click(&multi, &files(), 0, 3, PLAIN), single(Some("/d")));
    }

    #[test]
    fn toggle_adds_and_removes() {
        let added = click(&single(Some("/a")), &files(), 0, 2, TOGGLE);
        assert_eq!(added, sel(&["/a", "/c"], Some("/c")));
        assert_eq!(
            click(&added, &files(), 0, 2, TOGGLE),
            sel(&["/a"], Some("/c"))
        );
    }

    #[test]
    fn toggle_cannot_remove_the_focused_file() {
        let multi = sel(&["/a", "/c"], Some("/c"));
        assert_eq!(click(&multi, &files(), 0, 0, TOGGLE), multi);
    }

    #[test]
    fn a_range_after_a_toggle_uses_the_toggled_file_as_anchor() {
        let toggled = click(&single(Some("/a")), &files(), 0, 2, TOGGLE);
        assert_eq!(
            click(&toggled, &files(), 0, 4, RANGE),
            sel(&["/c", "/d", "/e"], Some("/c"))
        );
    }

    #[test]
    fn a_range_replaces_a_previous_range_from_the_same_anchor() {
        let wide = click(&single(Some("/b")), &files(), 1, 4, RANGE);
        assert_eq!(
            click(&wide, &files(), 4, 2, RANGE),
            sel(&["/b", "/c"], Some("/b"))
        );
    }

    #[test]
    fn a_range_without_an_anchor_is_a_plain_click() {
        assert_eq!(
            click(&single(None), &files(), 0, 2, RANGE),
            single(Some("/c"))
        );
    }

    #[test]
    fn extend_grows_and_shrinks_through_the_anchor() {
        let files = files();
        let (selection, index) = extend(&single(Some("/c")), &files, 2, 1);
        assert_eq!((&selection, index), (&sel(&["/c", "/d"], Some("/c")), 3));
        let (selection, index) = extend(&selection, &files, index, -1);
        let (selection, index) = extend(&selection, &files, index, -1);
        assert_eq!((selection, index), (sel(&["/b", "/c"], Some("/c")), 1));
    }

    #[test]
    fn extend_stops_at_the_ends() {
        assert_eq!(extend(&single(Some("/a")), &files(), 0, -1).1, 0);
        assert_eq!(extend(&single(Some("/e")), &files(), 4, 1).1, 4);
    }

    #[test]
    fn extend_on_an_empty_list_keeps_everything() {
        assert_eq!(extend(&single(None), &[], 0, 1), (single(None), 0));
    }

    #[test]
    fn prune_drops_hidden_paths_and_resets_a_hidden_anchor_to_the_focused_file() {
        let multi = sel(&["/a", "/b", "/d"], Some("/a"));
        let shown = ["/b", "/c", "/d"].map(String::from);
        assert_eq!(prune(&multi, &shown, 0), sel(&["/b", "/d"], Some("/b")));
    }

    #[test]
    fn prune_keeps_a_visible_anchor() {
        let multi = sel(&["/a", "/b"], Some("/a"));
        assert_eq!(prune(&multi, &files(), 1).anchor.as_deref(), Some("/a"));
    }

    #[test]
    fn prune_keeps_the_old_focused_file_next_to_a_new_one() {
        assert_eq!(
            prune(&single(Some("/a")), &files(), 1).selected,
            sel(&["/a", "/b"], None).selected
        );
    }

    #[test]
    fn a_resume_landing_selects_the_landed_file_alone() {
        assert_eq!(
            settle(&single(Some("/a")), &files(), 1, true),
            single(Some("/b"))
        );
    }

    #[test]
    fn a_resume_whose_file_is_hidden_selects_the_neighbor_it_landed_on() {
        let shown = ["/a", "/c", "/d"].map(String::from);
        assert_eq!(
            settle(&single(Some("/a")), &shown, 1, true),
            single(Some("/c"))
        );
    }

    #[test]
    fn a_plain_refresh_prunes_the_selection() {
        let multi = sel(&["/a", "/b", "/z"], Some("/a"));
        assert_eq!(
            settle(&multi, &files(), 2, false),
            sel(&["/a", "/b", "/c"], Some("/a"))
        );
    }

    #[test]
    fn a_focus_moved_onto_a_visible_file_selects_that_file_alone() {
        assert_eq!(
            restore(&single(Some("/b")), &files(), Some("/b"), "/a"),
            single(Some("/a"))
        );
    }

    #[test]
    fn a_changed_lists_pruned_selection_collapses_too() {
        let pruned = sel(&["/b", "/a"], Some("/b"));
        assert_eq!(
            restore(&pruned, &files(), Some("/b"), "/a"),
            single(Some("/a"))
        );
    }

    #[test]
    fn the_shown_file_keeps_a_multi_selection() {
        let multi = sel(&["/a", "/b", "/c"], Some("/a"));
        assert_eq!(restore(&multi, &files(), Some("/a"), "/a"), multi);
    }

    #[test]
    fn a_file_the_filter_hides_keeps_the_selection() {
        let selection = single(Some("/b"));
        assert_eq!(restore(&selection, &files(), Some("/b"), "/z"), selection);
    }

    #[test]
    fn targets_lists_the_selection_in_files_order() {
        let multi = sel(&["/d", "/a", "/c"], Some("/d"));
        assert_eq!(targets(&multi, &files(), 2), ["/a", "/c", "/d"]);
    }

    #[test]
    fn targets_fall_back_to_the_focused_file() {
        assert_eq!(targets(&single(Some("/a")), &files(), 3), ["/d"]);
        assert!(targets(&single(None), &[], 0).is_empty());
    }

    fn states() -> HashMap<String, State> {
        [
            ("/a", Some(2), Flag::None, Some("Red")),
            ("/b", Some(5), Flag::Pick, None),
            ("/c", Some(1), Flag::Reject, Some("Blue")),
        ]
        .into_iter()
        .map(|(path, rating, flag, label)| {
            (
                path.to_string(),
                State {
                    rating,
                    flag,
                    label: label.map(String::from),
                },
            )
        })
        .collect()
    }

    fn judge(paths: &[&str], focused: &str, cmd: Command) -> Vec<super::Change> {
        let states = states();
        let paths: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
        judgments(
            &paths,
            focused,
            |p| states.get(p).cloned().unwrap_or_default(),
            cmd,
            |_| true,
        )
    }

    #[test]
    fn a_label_toggle_on_a_mixed_selection_follows_the_focused_file() {
        let red = Command::Label("Red");
        let labels: Vec<_> = judge(&["/a", "/b", "/c"], "/b", red)
            .into_iter()
            .map(|c| c.after.label)
            .collect();
        assert_eq!(labels, [Some("Red".into()), Some("Red".into())]);
        let paths: Vec<_> = judge(&["/a", "/b", "/c"], "/a", red)
            .into_iter()
            .map(|c| c.path)
            .collect();
        assert_eq!(paths, ["/a", "/c"]);
    }

    #[test]
    fn a_star_command_keeps_each_files_label_and_flag() {
        let after: Vec<_> = judge(&["/a", "/b", "/c"], "/a", Command::Rate(3))
            .into_iter()
            .map(|c| c.after)
            .collect();
        let states = states();
        let three = |path: &str| State {
            rating: Some(3),
            ..states[path].clone()
        };
        assert_eq!(after, [three("/a"), three("/b"), three("/c")]);
    }

    #[test]
    fn files_already_at_the_value_are_skipped() {
        let changes = judge(&["/a", "/b"], "/b", Command::Rate(2));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "/b");
        assert_eq!(changes[0].before, states()["/b"]);
    }

    #[test]
    fn a_batch_with_no_change_is_empty_unless_clear_all_on_an_unseen_file() {
        let blank = |_: &str| State::default();
        let paths = ["/a".to_string(), "/b".to_string()];
        assert!(judgments(&paths, "/a", blank, Command::ClearAll, |_| true).is_empty());
        let unseen = judgments(&paths, "/a", blank, Command::ClearAll, |p| p != "/b");
        assert_eq!(unseen.len(), 1);
        assert_eq!(unseen[0].path, "/b");
        assert!(judgments(&paths, "/a", blank, Command::ClearRating, |_| false).is_empty());
    }

    #[test]
    fn the_focused_file_comes_first() {
        let paths: Vec<_> = judge(&["/a", "/b", "/c"], "/c", Command::ClearRating)
            .into_iter()
            .map(|c| c.path)
            .collect();
        assert_eq!(paths, ["/c", "/a", "/b"]);
    }
}
