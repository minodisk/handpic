//! The right-click menus, kept pure: their items and which menu is open
//! where. A port of the Tauri app's `context.ts`, so far the folder tree's
//! menu; the window draws the open menu as a popover at the pointer.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    RevealFolder,
    CopyPath,
    CopyFolderName,
    RefreshFolder,
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
/// copying the folder's path or name, then re-listing it.
pub fn folder_groups(reveal_label: &str) -> Vec<Vec<Item>> {
    vec![
        vec![command(Action::RevealFolder, reveal_label)],
        vec![
            command(Action::CopyPath, "Copy Path"),
            command(Action::CopyFolderName, "Copy Folder Name"),
        ],
        vec![command(Action::RefreshFolder, "Refresh")],
    ]
}

/// What an open menu acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A tree folder, by the tree's spelling of its path, and its name.
    Folder { path: String, name: String },
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

    /// A right-click on a tree folder at `at`, replacing any menu open.
    pub fn open_folder(&mut self, at: (f32, f32), path: String, name: String, reveal_label: &str) {
        self.open = Some(Open {
            at,
            target: Target::Folder { path, name },
            groups: folder_groups(reveal_label),
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

    /// Ported from `context.test.ts`'s `folderMenuGroups` cases, less the
    /// items part 2 and the tree's keyboard step add (the renames, `Expand
    /// All` / `Collapse All`, the sidecars, the sequencing and the import).
    #[test]
    fn folder_groups_hold_the_reveal_then_the_copy_then_the_refresh_item() {
        assert_eq!(
            folder_groups(REVEAL),
            vec![
                vec![command(Action::RevealFolder, REVEAL)],
                vec![
                    command(Action::CopyPath, "Copy Path"),
                    command(Action::CopyFolderName, "Copy Folder Name"),
                ],
                vec![command(Action::RefreshFolder, "Refresh")],
            ]
        );
    }

    #[test]
    fn folder_items_have_no_shortcut_no_checked_state_and_are_enabled() {
        for item in folder_groups(REVEAL).into_iter().flatten() {
            assert_eq!(item.shortcut, "");
            assert_eq!(item.checked, None);
            assert!(!item.disabled);
        }
    }

    #[test]
    fn a_menu_opens_at_the_pointer_on_its_folder_and_closes_once() {
        let mut menu = Menu::default();
        assert!(!menu.is_open());
        assert_eq!(menu.close(), None);

        menu.open_folder((10., 20.), r"D:\a".into(), "a".into(), REVEAL);
        let open = menu.open().unwrap();
        assert_eq!(open.at, (10., 20.));
        assert_eq!(
            open.target,
            Target::Folder {
                path: r"D:\a".into(),
                name: "a".into()
            }
        );
        assert_eq!(open.groups, folder_groups(REVEAL));

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

    #[test]
    fn a_second_right_click_replaces_the_open_menu() {
        let mut menu = Menu::default();
        menu.open_folder((10., 20.), r"D:\a".into(), "a".into(), REVEAL);
        menu.open_folder((30., 40.), r"D:\b".into(), "b".into(), REVEAL);
        let open = menu.open().unwrap();
        assert_eq!(open.at, (30., 40.));
        assert_eq!(
            open.target,
            Target::Folder {
                path: r"D:\b".into(),
                name: "b".into()
            }
        );
    }
}
