//! The right-click menus, kept pure: their items and which menu is open
//! where. A port of the Tauri app's `context.ts`, the folder tree's menu and
//! the strip's; the window draws the open menu as a popover at the pointer.

use riffle_backend::shortcuts::Binding;
use riffle_core::Flag;

use crate::judgment::State;

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
    pub label: String,
    /// The key bound to the action, shown at the item's right end, or empty.
    pub shortcut: String,
    /// `None` for a plain command, which has no checked state.
    pub checked: Option<bool>,
    pub disabled: bool,
}

fn command(action: Action, label: &str) -> Item {
    Item {
        action,
        label: label.to_owned(),
        shortcut: String::new(),
        checked: None,
        disabled: false,
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
/// the keymap's, checked from the focused file's state.
const JUDGMENT_GROUPS: [&[(&str, &str, Checked)]; 3] = [
    &[
        ("pick", "Pick", |s| s.flag == Flag::Pick),
        ("reject", "Reject", |s| s.flag == Flag::Reject),
        ("unflag", "Unflag", |s| s.flag == Flag::None),
    ],
    &[
        ("rate1", "1 star", |s| s.rating == Some(1)),
        ("rate2", "2 stars", |s| s.rating == Some(2)),
        ("rate3", "3 stars", |s| s.rating == Some(3)),
        ("rate4", "4 stars", |s| s.rating == Some(4)),
        ("rate5", "5 stars", |s| s.rating == Some(5)),
        ("clear", "No stars", |s| s.rating.is_none()),
    ],
    &[
        ("red", "Red", |s| s.label.as_deref() == Some("Red")),
        ("orange", "Orange", |s| s.label.as_deref() == Some("Orange")),
        ("yellow", "Yellow", |s| s.label.as_deref() == Some("Yellow")),
        ("green", "Green", |s| s.label.as_deref() == Some("Green")),
        ("blue", "Blue", |s| s.label.as_deref() == Some("Blue")),
        ("pink", "Pink", |s| s.label.as_deref() == Some("Pink")),
        ("purple", "Purple", |s| s.label.as_deref() == Some("Purple")),
        ("clearlabel", "No label", |s| s.label.is_none()),
    ],
];

/// `keys.ts`'s `displayKey`: a key as the menu shows it.
fn display_key(key: &str) -> String {
    if key == "space" {
        "Space".to_owned()
    } else {
        key.to_owned()
    }
}

/// The first key bound to `action`, as shown, or empty.
fn shortcut_of(bindings: &[Binding], action: &str) -> String {
    bindings
        .iter()
        .find(|binding| binding.action == action)
        .and_then(|binding| binding.keys.first())
        .map_or_else(String::new, |key| display_key(key))
}

/// The strip's menu, a port of `contextMenuGroups` less `Rename…` (part 2):
/// `Select All`, the reveal item, labeled per platform, copying the targets'
/// paths or names, the judgment groups with their checked state from the
/// focused file's `state`, left out in a view-only folder, then `Move to
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
                .map(|&(action, label, checked)| Item {
                    action: Action::Judge(action),
                    label: label.to_owned(),
                    shortcut: shortcut_of(bindings, action),
                    checked: Some(checked(state)),
                    disabled: false,
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

/// What an open menu acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A tree folder, by the tree's spelling of its path, and its name.
    Folder { path: String, name: String },
    /// The strip's focused file and the files the menu's items act on, by
    /// their listed paths, in `files` order.
    Files { focused: String, paths: Vec<String> },
}

#[derive(Clone, Debug)]
pub struct Open {
    /// The pointer's position in the window when it was opened.
    pub at: (f32, f32),
    pub target: Target,
    pub groups: Vec<Vec<Item>>,
}

/// The right-click menu, while one is open.
#[derive(Debug, Default)]
pub struct Menu {
    open: Option<Open>,
}

impl Menu {
    pub fn open(&self) -> Option<&Open> {
        self.open.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// A right-click on a tree folder at `at`, replacing any menu open;
    /// `root` when the folder is drawn as a root.
    pub fn open_folder(
        &mut self,
        at: (f32, f32),
        path: String,
        name: String,
        reveal_label: &str,
        root: bool,
    ) {
        self.open = Some(Open {
            at,
            target: Target::Folder { path, name },
            groups: folder_groups(reveal_label, root),
        });
    }

    /// A right-click on a strip cell at `at`, replacing any menu open.
    pub fn open_files(
        &mut self,
        at: (f32, f32),
        focused: String,
        paths: Vec<String>,
        groups: Vec<Vec<Item>>,
    ) {
        self.open = Some(Open {
            at,
            target: Target::Files { focused, paths },
            groups,
        });
    }

    /// Escape, a press outside, or an item picked: the target the menu had.
    pub fn close(&mut self) -> Option<Target> {
        self.open.take().map(|open| open.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_menu_opens_at_the_pointer_on_its_folder_and_closes_once() {
        let mut menu = Menu::default();
        assert!(!menu.is_open());
        assert_eq!(menu.close(), None);

        menu.open_folder((10., 20.), r"D:\a".into(), "a".into(), REVEAL, false);
        let open = menu.open().unwrap();
        assert_eq!(open.at, (10., 20.));
        assert_eq!(
            open.target,
            Target::Folder {
                path: r"D:\a".into(),
                name: "a".into()
            }
        );
        assert_eq!(open.groups, folder_groups(REVEAL, false));

        assert_eq!(
            menu.close(),
            Some(Target::Folder {
                path: r"D:\a".into(),
                name: "a".into()
            })
        );
        assert!(!menu.is_open());
        assert_eq!(menu.close(), None);
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

    /// `context.test.ts`'s `contextMenuGroups` cases, less `Rename…`.
    #[test]
    fn strip_groups_list_select_all_reveal_copy_the_judgments_then_trash_and_delete() {
        let groups = strip(&defaults(), &State::default());
        let judge = |action: &'static str, label: &str| (Action::Judge(action), label.to_owned());
        assert_eq!(
            map(&groups, |item| (item.action, item.label.clone())),
            vec![
                vec![(Action::SelectAll, "Select All".to_owned())],
                vec![(Action::RevealFile, FILE_REVEAL.to_owned())],
                vec![
                    (Action::CopyPath, "Copy Path".to_owned()),
                    (Action::CopyFileName, "Copy File Name".to_owned()),
                ],
                vec![
                    judge("pick", "Pick"),
                    judge("reject", "Reject"),
                    judge("unflag", "Unflag"),
                ],
                vec![
                    judge("rate1", "1 star"),
                    judge("rate2", "2 stars"),
                    judge("rate3", "3 stars"),
                    judge("rate4", "4 stars"),
                    judge("rate5", "5 stars"),
                    judge("clear", "No stars"),
                ],
                vec![
                    judge("red", "Red"),
                    judge("orange", "Orange"),
                    judge("yellow", "Yellow"),
                    judge("green", "Green"),
                    judge("blue", "Blue"),
                    judge("pink", "Pink"),
                    judge("purple", "Purple"),
                    judge("clearlabel", "No label"),
                ],
                vec![
                    (Action::TrashFiles, "Move to Trash…".to_owned()),
                    (Action::DeleteFiles, "Delete Permanently…".to_owned()),
                ],
            ]
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
                vec!["p".to_owned(), "x".to_owned(), "u".to_owned()],
                ["1", "2", "3", "4", "5", "0"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                ctrl_alt,
                vec![String::new(), String::new()],
            ]
        );
    }

    #[test]
    fn strip_groups_show_an_overridden_key_as_displayed() {
        let groups = strip(&rebound("rate3", &["space"]), &State::default());
        assert_eq!(groups[4][2].shortcut, "Space");
        let groups = strip(&rebound("selectAll", &["ctrl+alt+a"]), &State::default());
        assert_eq!(groups[0][0].shortcut, "ctrl+alt+a");
    }

    #[test]
    fn strip_groups_show_the_first_key_when_an_action_has_several() {
        let bindings = vec![Binding {
            action: "reject",
            keys: vec!["d".into(), "x".into()],
        }];
        assert_eq!(strip(&bindings, &State::default())[3][1].shortcut, "d");
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
                vec![FILE_REVEAL.to_owned()],
                vec!["Copy Path".to_owned(), "Copy File Name".to_owned()],
            ]
        );
        let view_only = strip_groups(&defaults(), &State::default(), FILE_REVEAL, true, 2, false);
        assert_eq!(
            map(&view_only[3..], |item| item.label.clone()),
            vec![vec![
                "Move 2 Files to Trash…".to_owned(),
                "Delete 2 Files Permanently…".to_owned(),
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
    fn a_strip_menu_opens_on_its_files_and_closes_with_them() {
        let mut menu = Menu::default();
        menu.open_folder((10., 20.), r"D:\a".into(), "a".into(), REVEAL, false);
        let groups = strip(&defaults(), &State::default());
        menu.open_files(
            (5., 6.),
            r"D:\a\2.ARW".into(),
            vec![r"D:\a\1.ARW".into(), r"D:\a\2.ARW".into()],
            groups.clone(),
        );
        let open = menu.open().unwrap();
        assert_eq!(open.at, (5., 6.));
        assert_eq!(open.groups, groups);
        assert_eq!(
            menu.close(),
            Some(Target::Files {
                focused: r"D:\a\2.ARW".into(),
                paths: vec![r"D:\a\1.ARW".into(), r"D:\a\2.ARW".into()],
            })
        );
        assert!(!menu.is_open());
    }

    #[test]
    fn a_second_right_click_replaces_the_open_menu() {
        let mut menu = Menu::default();
        menu.open_folder((10., 20.), r"D:\a".into(), "a".into(), REVEAL, false);
        menu.open_folder((30., 40.), r"D:\b".into(), "b".into(), REVEAL, true);
        let open = menu.open().unwrap();
        assert_eq!(open.at, (30., 40.));
        assert_eq!(
            open.target,
            Target::Folder {
                path: r"D:\b".into(),
                name: "b".into()
            }
        );
        assert_eq!(open.groups, folder_groups(REVEAL, true));
    }
}
