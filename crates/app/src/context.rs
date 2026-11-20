//! The right-click menus, kept pure: their items and what an open menu acts
//! on. A port of the Tauri app's `context.ts`, the folder tree's menu and
//! the strip's; the window shows the open menu as gpui-component's
//! `PopupMenu` at the pointer.
//! The strip menu's judgment groups differ from `context.ts` on purpose: they
//! follow the filter menu's (`menu::filter_rows`) order, the flags Pick,
//! Unflag, Reject and the rating from 5 stars down to the unrated, and the
//! rating rows draw its stars.

use handpic_backend::shortcuts::Binding;
use handpic_core::Flag;

use crate::strip::Tint;
use crate::{judgment::State, menu::Label};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    RevealFolder,
    CopyPath,
    CopyFolderName,
    RefreshFolder,
    ExpandAll,
    CollapseAll,
    RenameFolder,
    RenameByDate,
    RewriteSidecars,
    DeleteSidecars,
    SequenceTimestamps,
    ImportFolder,
    SelectAll,
    RevealFile,
    CopyFileName,
    /// A judgment, by the keymap's action name.
    Judge(&'static str),
    TrashFiles,
    DeleteFiles,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub action: Action,
    pub label: Label,
    /// The keymap's name of the key bound to the action, drawn by `Kbd` at
    /// the menu item's right end, or empty.
    pub shortcut: String,
    /// `None` for a plain command, which has no checked state.
    pub checked: Option<bool>,
    pub disabled: bool,
    /// `Some` on a color label item, which is drawn as a Tag: the label's
    /// tint, `None` for `No label`.
    pub tag: Option<Option<Tint>>,
}

fn command(action: Action, label: &str) -> Item {
    Item {
        action,
        label: Label::Owned(label.to_owned()),
        shortcut: String::new(),
        checked: None,
        disabled: false,
        tag: None,
    }
}

/// The tag of a judgment item, by its keymap action name: a color label's
/// tint, `No label`'s none, and not a tag for any other judgment.
fn label_tag(action: &str) -> Option<Option<Tint>> {
    match (action, Tint::of(action)) {
        ("clearlabel", _) => Some(None),
        (_, Tint::Other) => None,
        (_, tint) => Some(Some(tint)),
    }
}

/// The folder tree's menu: the reveal item, labeled per platform, then
/// copying the folder's path or name, then re-listing it, then, not on a
/// root (home or a volume), renaming it and expanding or collapsing every
/// subfolder under it, then rewriting or deleting its sidecars, sequencing
/// its JPEG timestamps and importing it.
pub fn folder_groups(reveal_label: &str, root: bool) -> Vec<Vec<Item>> {
    let mut groups = vec![
        vec![command(Action::RevealFolder, reveal_label)],
        vec![
            command(Action::CopyPath, "Copy Path"),
            command(Action::CopyFolderName, "Copy Folder Name"),
        ],
        vec![command(Action::RefreshFolder, "Refresh")],
    ];
    if !root {
        groups.push(vec![
            command(Action::RenameFolder, "Rename…"),
            command(Action::RenameByDate, "Rename by Capture Date…"),
        ]);
        groups.push(vec![
            command(Action::ExpandAll, "Expand All"),
            command(Action::CollapseAll, "Collapse All"),
        ]);
    }
    groups.push(vec![
        command(Action::RewriteSidecars, "Rewrite Sidecars from Index…"),
        command(Action::DeleteSidecars, "Delete Sidecars…"),
    ]);
    groups.push(vec![command(
        Action::SequenceTimestamps,
        "Sequence JPEG Timestamps…",
    )]);
    groups.push(vec![command(Action::ImportFolder, "Import…")]);
    groups
}

type Checked = fn(&State) -> bool;

/// The judgment groups: the flags, the stars, the labels, each item's action
/// the keymap's, checked from the focused file's state, in the filter menu's
/// order.
const JUDGMENT_GROUPS: [&[(&str, Label, Checked)]; 3] = [
    &[
        ("pick", Label::Text("Pick"), |s| s.flag == Flag::Pick),
        ("unflag", Label::Text("Unflag"), |s| s.flag == Flag::None),
        ("reject", Label::Text("Reject"), |s| s.flag == Flag::Reject),
    ],
    &[
        ("rate5", Label::Stars(5), |s| s.rating == Some(5)),
        ("rate4", Label::Stars(4), |s| s.rating == Some(4)),
        ("rate3", Label::Stars(3), |s| s.rating == Some(3)),
        ("rate2", Label::Stars(2), |s| s.rating == Some(2)),
        ("rate1", Label::Stars(1), |s| s.rating == Some(1)),
        ("clear", Label::Stars(0), |s| s.rating.is_none()),
    ],
    &[
        ("red", Label::Text("Red"), |s| {
            s.label.as_deref() == Some("Red")
        }),
        ("orange", Label::Text("Orange"), |s| {
            s.label.as_deref() == Some("Orange")
        }),
        ("yellow", Label::Text("Yellow"), |s| {
            s.label.as_deref() == Some("Yellow")
        }),
        ("green", Label::Text("Green"), |s| {
            s.label.as_deref() == Some("Green")
        }),
        ("blue", Label::Text("Blue"), |s| {
            s.label.as_deref() == Some("Blue")
        }),
        ("pink", Label::Text("Pink"), |s| {
            s.label.as_deref() == Some("Pink")
        }),
        ("purple", Label::Text("Purple"), |s| {
            s.label.as_deref() == Some("Purple")
        }),
        ("clearlabel", Label::Text("No label"), |s| s.label.is_none()),
    ],
];

/// The actions of the judgment groups in the menu's order.
pub fn judgment_actions() -> impl Iterator<Item = &'static str> {
    JUDGMENT_GROUPS
        .into_iter()
        .flat_map(|group| group.iter().map(|(action, _, _)| *action))
}

/// The first key bound to `action`, by its keymap name, or empty.
fn shortcut_of(bindings: &[Binding], action: &str) -> String {
    bindings
        .iter()
        .find(|binding| binding.action == action)
        .and_then(|binding| binding.keys.first())
        .cloned()
        .unwrap_or_default()
}

/// The strip's menu, a port of `contextMenuGroups` less `Rename…` (part 2):
/// `Select All`, the reveal item, labeled per platform, copying the targets'
/// paths or names, the judgment groups with their checked state from the
/// focused file's `state`, left out in a view-only folder, in the filter
/// menu's order rather than `contextMenuGroups`', then `Move to
/// Trash…` and `Delete Permanently…`, naming the `count` targets when there
/// are several and disabled while `trashing`. Each item shows its action's
/// first key.
pub fn strip_groups(
    bindings: &[Binding],
    state: &State,
    reveal_label: &str,
    view_only: bool,
    count: usize,
    trashing: bool,
) -> Vec<Vec<Item>> {
    let mut groups = vec![
        vec![Item {
            shortcut: shortcut_of(bindings, "selectAll"),
            ..command(Action::SelectAll, "Select All")
        }],
        vec![command(Action::RevealFile, reveal_label)],
        vec![
            command(Action::CopyPath, "Copy Path"),
            command(Action::CopyFileName, "Copy File Name"),
        ],
    ];
    if !view_only {
        groups.extend(JUDGMENT_GROUPS.iter().map(|group| {
            group
                .iter()
                .map(|(action, label, checked)| Item {
                    action: Action::Judge(action),
                    label: label.clone(),
                    shortcut: shortcut_of(bindings, action),
                    checked: Some(checked(state)),
                    disabled: false,
                    tag: label_tag(action),
                })
                .collect()
        }));
    }
    let (trash, delete) = if count > 1 {
        (
            format!("Move {count} Files to Trash…"),
            format!("Delete {count} Files Permanently…"),
        )
    } else {
        (
            "Move to Trash…".to_owned(),
            "Delete Permanently…".to_owned(),
        )
    };
    groups.push(
        [(Action::TrashFiles, trash), (Action::DeleteFiles, delete)]
            .into_iter()
            .map(|(action, label)| Item {
                disabled: trashing,
                ..command(action, &label)
            })
            .collect(),
    );
    groups
}

/// A path as the user knows it, `trash.ts`'s `shownPath`: without the
/// verbatim `\\?\` prefix a canonical path carries on Windows, which the
/// shell's `SHParseDisplayName` refuses (`E_INVALIDARG`), so a reveal needs
/// it gone.
pub fn shown_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    }
}

/// The title bar's name of the open folder: its last component, else (a
/// drive root such as `C:\` or `/`) the shown path.
pub fn folder_title(dir: &str) -> String {
    match std::path::Path::new(dir).file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => shown_path(dir),
    }
}

/// What an open menu acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A tree folder, by the tree's spelling of its path, and its name.
    Folder { path: String, name: String },
    /// The strip's focused file and the files the menu's items act on, by
    /// their listed paths, in `files` order.
    Files { focused: String, paths: Vec<String> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        exif::Groups,
        filter::{FilterFlag, FilterState},
        menu::{Action as MenuAction, Item as MenuItem, Row, Toggle, filter_rows},
    };

    const REVEAL: &str = "Reveal in Finder";

    /// Ported from `context.test.ts`'s `folderMenuGroups` cases.
    #[test]
    fn folder_groups_hold_the_reveal_copy_refresh_rename_expand_sidecar_sequence_and_import_items()
    {
        assert_eq!(
            folder_groups(REVEAL, false),
            vec![
                vec![command(Action::RevealFolder, REVEAL)],
                vec![
                    command(Action::CopyPath, "Copy Path"),
                    command(Action::CopyFolderName, "Copy Folder Name"),
                ],
                vec![command(Action::RefreshFolder, "Refresh")],
                vec![
                    command(Action::RenameFolder, "Rename…"),
                    command(Action::RenameByDate, "Rename by Capture Date…"),
                ],
                vec![
                    command(Action::ExpandAll, "Expand All"),
                    command(Action::CollapseAll, "Collapse All"),
                ],
                vec![
                    command(Action::RewriteSidecars, "Rewrite Sidecars from Index…"),
                    command(Action::DeleteSidecars, "Delete Sidecars…"),
                ],
                vec![command(
                    Action::SequenceTimestamps,
                    "Sequence JPEG Timestamps…"
                )],
                vec![command(Action::ImportFolder, "Import…")],
            ]
        );
    }

    #[test]
    fn a_root_has_no_rename_or_expand_items_and_keeps_import() {
        let actions: Vec<Vec<Action>> = folder_groups(REVEAL, true)
            .into_iter()
            .map(|group| group.into_iter().map(|item| item.action).collect())
            .collect();
        assert_eq!(
            actions,
            vec![
                vec![Action::RevealFolder],
                vec![Action::CopyPath, Action::CopyFolderName],
                vec![Action::RefreshFolder],
                vec![Action::RewriteSidecars, Action::DeleteSidecars],
                vec![Action::SequenceTimestamps],
                vec![Action::ImportFolder],
            ]
        );
    }

    #[test]
    fn folder_items_have_no_shortcut_or_checked_state() {
        for item in folder_groups(REVEAL, false).into_iter().flatten() {
            assert_eq!(item.shortcut, "");
            assert_eq!(item.checked, None);
        }
    }

    #[test]
    fn no_folder_item_is_disabled() {
        let disabled: Vec<Action> = folder_groups(REVEAL, false)
            .into_iter()
            .flatten()
            .filter(|item| item.disabled)
            .map(|item| item.action)
            .collect();
        assert_eq!(disabled, vec![]);
    }

    const FILE_REVEAL: &str = "Reveal in File Explorer";

    fn defaults() -> Vec<Binding> {
        [
            ("selectAll", "meta+a"),
            ("pick", "p"),
            ("reject", "x"),
            ("unflag", "u"),
            ("rate1", "1"),
            ("rate2", "2"),
            ("rate3", "3"),
            ("rate4", "4"),
            ("rate5", "5"),
            ("clear", "0"),
            ("red", "ctrl+alt+1"),
            ("orange", "ctrl+alt+2"),
            ("yellow", "ctrl+alt+3"),
            ("green", "ctrl+alt+4"),
            ("blue", "ctrl+alt+5"),
            ("pink", "ctrl+alt+6"),
            ("purple", "ctrl+alt+7"),
            ("clearlabel", "ctrl+alt+0"),
        ]
        .into_iter()
        .map(|(action, key)| Binding {
            action,
            keys: vec![key.to_owned()],
        })
        .collect()
    }

    fn rebound(action: &'static str, keys: &[&str]) -> Vec<Binding> {
        defaults()
            .into_iter()
            .map(|binding| Binding {
                keys: if binding.action == action {
                    keys.iter().map(|&key| key.to_owned()).collect()
                } else {
                    binding.keys
                },
                ..binding
            })
            .collect()
    }

    fn strip(bindings: &[Binding], state: &State) -> Vec<Vec<Item>> {
        strip_groups(bindings, state, FILE_REVEAL, false, 1, false)
    }

    fn map<T>(groups: &[Vec<Item>], f: impl Fn(&Item) -> T) -> Vec<Vec<T>> {
        groups
            .iter()
            .map(|group| group.iter().map(&f).collect())
            .collect()
    }

    fn checked_actions(groups: &[Vec<Item>]) -> Vec<Action> {
        groups
            .iter()
            .flatten()
            .filter(|item| item.checked == Some(true))
            .map(|item| item.action)
            .collect()
    }

    /// `context.test.ts`'s `contextMenuGroups` cases, less `Rename…`, with
    /// the judgment groups in the filter menu's order and the rating rows as
    /// its stars.
    #[test]
    fn strip_groups_list_select_all_reveal_copy_the_judgments_then_trash_and_delete() {
        let groups = strip(&defaults(), &State::default());
        let owned = |action: Action, text: &str| (action, Label::Owned(text.to_owned()));
        let judge = |action: &'static str, label: Label| (Action::Judge(action), label);
        assert_eq!(
            map(&groups, |item| (item.action, item.label.clone())),
            vec![
                vec![owned(Action::SelectAll, "Select All")],
                vec![owned(Action::RevealFile, FILE_REVEAL)],
                vec![
                    owned(Action::CopyPath, "Copy Path"),
                    owned(Action::CopyFileName, "Copy File Name"),
                ],
                vec![
                    judge("pick", Label::Text("Pick")),
                    judge("unflag", Label::Text("Unflag")),
                    judge("reject", Label::Text("Reject")),
                ],
                vec![
                    judge("rate5", Label::Stars(5)),
                    judge("rate4", Label::Stars(4)),
                    judge("rate3", Label::Stars(3)),
                    judge("rate2", Label::Stars(2)),
                    judge("rate1", Label::Stars(1)),
                    judge("clear", Label::Stars(0)),
                ],
                vec![
                    judge("red", Label::Text("Red")),
                    judge("orange", Label::Text("Orange")),
                    judge("yellow", Label::Text("Yellow")),
                    judge("green", Label::Text("Green")),
                    judge("blue", Label::Text("Blue")),
                    judge("pink", Label::Text("Pink")),
                    judge("purple", Label::Text("Purple")),
                    judge("clearlabel", Label::Text("No label")),
                ],
                vec![
                    owned(Action::TrashFiles, "Move to Trash…"),
                    owned(Action::DeleteFiles, "Delete Permanently…"),
                ],
            ]
        );
    }

    /// The filter menu's item each judgment action stands for.
    fn toggle_of(action: &'static str) -> Toggle {
        match action {
            "pick" => Toggle::Flag(FilterFlag::Picked),
            "unflag" => Toggle::Flag(FilterFlag::Untagged),
            "reject" => Toggle::Flag(FilterFlag::Rejected),
            "clear" => Toggle::Stars(0),
            "clearlabel" => Toggle::Label("none"),
            rate if rate.starts_with("rate") => Toggle::Stars(rate[4..].parse().unwrap()),
            color => Toggle::Label(color),
        }
    }

    /// The flag, rating and label items of the strip menu, as filter items,
    /// run in `filter_rows`' order, and the rating rows draw its stars.
    #[test]
    fn strip_judgment_groups_follow_the_filter_menus_order_and_stars() {
        let filter: Vec<(Toggle, Option<Label>)> =
            filter_rows(&FilterState::default(), false, &Groups::default())
                .into_iter()
                .filter_map(|row| match row {
                    Row::Item(MenuItem {
                        label,
                        action: MenuAction::Toggle(toggle),
                        ..
                    }) => match toggle {
                        Toggle::Flag(_) | Toggle::Label(_) => Some((toggle, None)),
                        Toggle::Stars(_) => Some((toggle, Some(label))),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
        let judgments: Vec<(Toggle, Option<Label>)> = strip(&defaults(), &State::default())[3..6]
            .iter()
            .flatten()
            .map(|item| {
                let Action::Judge(action) = item.action else {
                    panic!("{:?}", item.action);
                };
                let toggle = toggle_of(action);
                let label = matches!(toggle, Toggle::Stars(_)).then(|| item.label.clone());
                (toggle, label)
            })
            .collect();
        assert_eq!(judgments, filter);
    }

    #[test]
    fn only_the_label_items_carry_a_tag() {
        let tagged: Vec<(Action, Option<Tint>)> = strip(&defaults(), &State::default())
            .iter()
            .flatten()
            .filter_map(|item| item.tag.map(|tint| (item.action, tint)))
            .collect();
        assert_eq!(
            tagged,
            vec![
                (Action::Judge("red"), Some(Tint::Red)),
                (Action::Judge("orange"), Some(Tint::Orange)),
                (Action::Judge("yellow"), Some(Tint::Yellow)),
                (Action::Judge("green"), Some(Tint::Green)),
                (Action::Judge("blue"), Some(Tint::Blue)),
                (Action::Judge("pink"), Some(Tint::Pink)),
                (Action::Judge("purple"), Some(Tint::Purple)),
                (Action::Judge("clearlabel"), None),
            ]
        );
        assert!(
            folder_groups(REVEAL, false)
                .iter()
                .flatten()
                .all(|item| item.tag.is_none())
        );
    }

    #[test]
    fn strip_groups_disable_trash_and_delete_alone_while_either_is_in_flight() {
        let disabled = |trashing: bool| -> Vec<Action> {
            strip_groups(
                &defaults(),
                &State::default(),
                FILE_REVEAL,
                false,
                1,
                trashing,
            )
            .into_iter()
            .flatten()
            .filter(|item| item.disabled)
            .map(|item| item.action)
            .collect()
        };
        assert_eq!(
            disabled(true),
            vec![Action::TrashFiles, Action::DeleteFiles]
        );
        assert_eq!(disabled(false), vec![]);
    }

    #[test]
    fn strip_groups_show_the_default_keys() {
        let ctrl_alt: Vec<String> = ["1", "2", "3", "4", "5", "6", "7", "0"]
            .into_iter()
            .map(|key| format!("ctrl+alt+{key}"))
            .collect();
        assert_eq!(
            map(&strip(&defaults(), &State::default()), |item| item
                .shortcut
                .clone()),
            vec![
                vec!["meta+a".to_owned()],
                vec![String::new()],
                vec![String::new(), String::new()],
                vec!["p".to_owned(), "u".to_owned(), "x".to_owned()],
                ["5", "4", "3", "2", "1", "0"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                ctrl_alt,
                vec![String::new(), String::new()],
            ]
        );
    }

    #[test]
    fn strip_groups_show_an_overridden_key_by_its_keymap_name() {
        let groups = strip(&rebound("rate3", &["space"]), &State::default());
        assert_eq!(groups[4][2].shortcut, "space");
        let groups = strip(&rebound("selectAll", &["ctrl+alt+a"]), &State::default());
        assert_eq!(groups[0][0].shortcut, "ctrl+alt+a");
    }

    #[test]
    fn strip_groups_show_the_first_key_when_an_action_has_several() {
        let bindings = vec![Binding {
            action: "reject",
            keys: vec!["d".into(), "x".into()],
        }];
        assert_eq!(strip(&bindings, &State::default())[3][2].shortcut, "d");
    }

    #[test]
    fn strip_groups_give_an_unbound_action_no_shortcut() {
        let shortcuts: Vec<String> = strip(&[], &State::default())
            .into_iter()
            .flatten()
            .map(|item| item.shortcut)
            .collect();
        assert_eq!(shortcuts.len(), 23);
        assert!(shortcuts.iter().all(String::is_empty));
    }

    #[test]
    fn strip_groups_give_select_all_reveal_copy_trash_and_delete_no_checked_state() {
        let groups = strip(&defaults(), &State::default());
        for item in groups[..3].iter().chain(groups.last()).flatten() {
            assert_eq!(item.checked, None, "{:?}", item.action);
        }
    }

    #[test]
    fn strip_groups_mark_the_unset_items_when_nothing_is_set() {
        assert_eq!(
            checked_actions(&strip(&defaults(), &State::default())),
            vec![
                Action::Judge("unflag"),
                Action::Judge("clear"),
                Action::Judge("clearlabel"),
            ]
        );
    }

    #[test]
    fn strip_groups_mark_the_current_flag_rating_and_label() {
        let state = State {
            rating: Some(3),
            flag: Flag::Pick,
            label: Some("Orange".into()),
        };
        assert_eq!(
            checked_actions(&strip(&defaults(), &state)),
            vec![
                Action::Judge("pick"),
                Action::Judge("rate3"),
                Action::Judge("orange"),
            ]
        );
        let rejected = State {
            flag: Flag::Reject,
            ..State::default()
        };
        assert_eq!(
            checked_actions(&strip(&defaults(), &rejected))[0],
            Action::Judge("reject")
        );
    }

    #[test]
    fn a_view_only_folder_keeps_select_all_reveal_copy_trash_and_delete() {
        let groups = strip_groups(&defaults(), &State::default(), FILE_REVEAL, true, 1, false);
        assert_eq!(
            groups,
            vec![
                vec![Item {
                    shortcut: "meta+a".into(),
                    ..command(Action::SelectAll, "Select All")
                }],
                vec![command(Action::RevealFile, FILE_REVEAL)],
                vec![
                    command(Action::CopyPath, "Copy Path"),
                    command(Action::CopyFileName, "Copy File Name"),
                ],
                vec![
                    command(Action::TrashFiles, "Move to Trash…"),
                    command(Action::DeleteFiles, "Delete Permanently…"),
                ],
            ]
        );
    }

    #[test]
    fn strip_groups_name_the_count_of_several_targets_in_trash_and_delete_alone() {
        let groups = strip_groups(&defaults(), &State::default(), FILE_REVEAL, false, 3, false);
        assert_eq!(
            groups.last().unwrap(),
            &vec![
                command(Action::TrashFiles, "Move 3 Files to Trash…"),
                command(Action::DeleteFiles, "Delete 3 Files Permanently…"),
            ]
        );
        assert_eq!(
            map(&groups[1..3], |item| item.label.clone()),
            vec![
                vec![Label::Owned(FILE_REVEAL.to_owned())],
                vec![
                    Label::Owned("Copy Path".to_owned()),
                    Label::Owned("Copy File Name".to_owned()),
                ],
            ]
        );
        let view_only = strip_groups(&defaults(), &State::default(), FILE_REVEAL, true, 2, false);
        assert_eq!(
            map(&view_only[3..], |item| item.label.clone()),
            vec![vec![
                Label::Owned("Move 2 Files to Trash…".to_owned()),
                Label::Owned("Delete 2 Files Permanently…".to_owned()),
            ]]
        );
    }

    /// `trash.test.ts`'s `shownPath` cases.
    #[test]
    fn a_shown_path_drops_the_verbatim_prefix() {
        assert_eq!(shown_path(r"\\?\D:\Photos\a.ARW"), r"D:\Photos\a.ARW");
        assert_eq!(
            shown_path(r"\\?\UNC\server\share\a.ARW"),
            r"\\server\share\a.ARW"
        );
        assert_eq!(shown_path(r"D:\Photos\a.ARW"), r"D:\Photos\a.ARW");
        assert_eq!(shown_path("/Users/a/b.ARW"), "/Users/a/b.ARW");
    }

    #[test]
    fn a_folder_title_is_the_last_component_else_the_shown_path() {
        assert_eq!(folder_title("/Users/a/Photos"), "Photos");
        assert_eq!(folder_title("/"), "/");
        #[cfg(windows)]
        {
            assert_eq!(folder_title(r"\\?\D:\Photos\2026"), "2026");
            assert_eq!(folder_title(r"\\?\D:\"), r"D:\");
        }
    }
}
