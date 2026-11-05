#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod commands;
mod datefolder;
mod diagnostics;
mod exif;
mod folders;
mod foldersidecars;
mod import;
mod index;
mod mcp;
mod perf;
mod photolab;
mod rank;
mod rename;
mod sequence;
mod shortcuts;
mod sidecar;
mod trash;
mod treewatch;
mod update;
mod watch;

mod app_menu {
    use tauri::image::Image;
    #[cfg(target_os = "macos")]
    use tauri::menu::IconMenuItem;
    use tauri::menu::{AboutMetadata, Menu, MenuEvent, MenuItemKind, PredefinedMenuItem, Submenu};
    use tauri::menu::{CheckMenuItem, MenuItem};
    use tauri::{AppHandle, Emitter, Manager, Wry};
    use tauri_plugin_opener::OpenerExt;

    const OPEN_FOLDER_ID: &str = "open-folder";
    const RELOAD_FOLDER_ID: &str = "reload-folder";
    const OPEN_LOG_FOLDER_ID: &str = "open-log-folder";
    const SETTINGS_ID: &str = "open-settings";
    const UNDO_ID: &str = "undo";
    const REDO_ID: &str = "redo";
    const SELECT_ALL_ID: &str = "select-all";
    const CHECK_UPDATES_ID: &str = "check-for-updates";

    /// The `View` check items: `(menu id, keymap action, label)`. A click
    /// emits `menu-action` with the action, which the frontend runs as its
    /// key; the check follows the stored panels (`apply_panels`).
    const VIEW_ITEMS: &[(&str, &str, &str)] = &[
        ("toggle-left", "toggleLeft", "Folders"),
        ("toggle-right", "toggleRight", "Metadata"),
        ("toggle-strip", "toggleStrip", "Filmstrip"),
    ];

    /// Whether the `View` item of `action` is checked for the stored
    /// `panels`, or `None` for an action that is not a pane toggle. A missing
    /// or non-boolean entry means shown, as `panels_setting` reads it.
    fn panel_checked(panels: &serde_json::Value, action: &str) -> Option<bool> {
        let name = match action {
            "toggleLeft" => "left",
            "toggleRight" => "right",
            "toggleStrip" => "strip",
            _ => return None,
        };
        Some(
            panels
                .get(name)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
        )
    }

    /// Set the `View` checks from `panels`.
    pub fn apply_panels(app: &AppHandle, panels: &serde_json::Value) {
        let Some(menu) = app.menu() else {
            return;
        };
        let Ok(items) = menu.items() else {
            return;
        };
        for &(id, action, _) in VIEW_ITEMS {
            let item = items
                .iter()
                .filter_map(|kind| kind.as_submenu()?.get(id))
                .find_map(|kind| kind.as_check_menuitem().cloned());
            if let (Some(item), Some(checked)) = (item, panel_checked(panels, action)) {
                if let Err(e) = item.set_checked(checked) {
                    log::warn!("failed to set the {id} check: {e}");
                }
            }
        }
    }

    /// Set the `View` checks from the panels in `AppPanels`.
    fn apply_stored_panels(app: &AppHandle) {
        let Some(state) = app.try_state::<crate::commands::AppPanels>() else {
            return;
        };
        let panels = crate::index::lock(&state.0).clone();
        apply_panels(app, &panels);
    }

    /// The menu ids and actions of the items whose accelerator mirrors the
    /// action's keys.
    fn keyed_items() -> impl Iterator<Item = (&'static str, &'static str)> {
        [
            (OPEN_FOLDER_ID, "open"),
            (UNDO_ID, "undo"),
            (REDO_ID, "redo"),
            (SELECT_ALL_ID, "selectAll"),
        ]
        .into_iter()
        .chain(VIEW_ITEMS.iter().map(|&(id, action, _)| (id, action)))
    }

    /// The actions a menu item mirrors the accelerator of.
    pub fn keyed_actions() -> impl Iterator<Item = &'static str> {
        keyed_items().map(|(_, action)| action)
    }

    /// The default menu's submenu titled `title`, if the platform has one.
    fn submenu(menu: &Menu<Wry>, title: &str) -> tauri::Result<Option<Submenu<Wry>>> {
        for item in menu.items()? {
            if let MenuItemKind::Submenu(submenu) = item {
                if submenu.text()? == title {
                    return Ok(Some(submenu));
                }
            }
        }
        Ok(None)
    }

    /// The app menu, with the `Open Folder…`, `Undo`, `Redo`, `Select All`
    /// and `View` accelerators the keymap currently gives those actions.
    pub fn build(
        handle: &AppHandle,
        keymap: &crate::shortcuts::Keymap,
    ) -> tauri::Result<Menu<Wry>> {
        let open = keymap.accelerator_for("open");
        let open = open.as_deref();
        let undo_key = keymap.accelerator_for("undo");
        let undo_key = undo_key.as_deref();
        let redo_key = keymap.accelerator_for("redo");
        let redo_key = redo_key.as_deref();
        let select_all_key = keymap.accelerator_for("selectAll");
        let select_all_key = select_all_key.as_deref();
        // The default menu carries the platform's standard items (Quit, Copy,
        // ...), which setting a menu at all would otherwise replace.
        let menu = Menu::default(handle)?;
        // The default About's metadata carries no icon, so macOS's About
        // panel and GTK's About dialog show none; it is rebuilt with the same
        // metadata plus the app icon (Windows' MessageBox ignores it).
        let about = PredefinedMenuItem::about(
            handle,
            None,
            Some(AboutMetadata {
                name: Some(handle.package_info().name.clone()),
                version: Some(handle.package_info().version.to_string()),
                copyright: handle.config().bundle.copyright.clone(),
                authors: handle.config().bundle.publisher.clone().map(|p| vec![p]),
                icon: Some(Image::from_bytes(include_bytes!("../icons/128x128.png"))?),
                ..Default::default()
            }),
        )?;
        #[cfg(target_os = "macos")]
        let about_menu = match menu.items()?.into_iter().next() {
            Some(MenuItemKind::Submenu(app)) => Some(app),
            _ => None,
        };
        #[cfg(not(target_os = "macos"))]
        let about_menu = submenu(&menu, "Help")?;
        if let Some(about_menu) = about_menu {
            if let Some(MenuItemKind::Predefined(_)) = about_menu.items()?.into_iter().next() {
                about_menu.remove_at(0)?;
                about_menu.insert(&about, 0)?;
            }
        }
        // `folder.png` is deliberately shared with Help > Open Log Folder:
        // the two menus are never open at the same time.
        #[cfg(target_os = "macos")]
        let open_folder = IconMenuItem::with_id(
            handle,
            OPEN_FOLDER_ID,
            "Open Folder…",
            true,
            Some(Image::from_bytes(include_bytes!(
                "../icons/menu/folder.png"
            ))?),
            open,
        )?;
        #[cfg(not(target_os = "macos"))]
        let open_folder = MenuItem::with_id(handle, OPEN_FOLDER_ID, "Open Folder…", true, open)?;
        // A fixed accelerator, like Settings: reloading is not a
        // culling action, so it is not part of the rebindable keymap.
        #[cfg(target_os = "macos")]
        let reload_folder = IconMenuItem::with_id(
            handle,
            RELOAD_FOLDER_ID,
            "Reload Folder",
            true,
            Some(Image::from_bytes(include_bytes!(
                "../icons/menu/arrow.clockwise.png"
            ))?),
            Some("CmdOrCtrl+R"),
        )?;
        #[cfg(not(target_os = "macos"))]
        let reload_folder = MenuItem::with_id(
            handle,
            RELOAD_FOLDER_ID,
            "Reload Folder",
            true,
            Some("CmdOrCtrl+R"),
        )?;
        #[cfg(target_os = "macos")]
        let settings = IconMenuItem::with_id(
            handle,
            SETTINGS_ID,
            "Settings...",
            true,
            Some(Image::from_bytes(include_bytes!("../icons/menu/gear.png"))?),
            Some("CmdOrCtrl+,"),
        )?;
        #[cfg(not(target_os = "macos"))]
        let settings = MenuItem::with_id(
            handle,
            SETTINGS_ID,
            "Settings...",
            true,
            Some("CmdOrCtrl+,"),
        )?;
        #[cfg(target_os = "macos")]
        let check_updates = IconMenuItem::with_id(
            handle,
            CHECK_UPDATES_ID,
            "Check for Updates…",
            true,
            Some(Image::from_bytes(include_bytes!(
                "../icons/menu/square.and.arrow.down.png"
            ))?),
            None::<&str>,
        )?;
        #[cfg(not(target_os = "macos"))]
        let check_updates = MenuItem::with_id(
            handle,
            CHECK_UPDATES_ID,
            "Check for Updates…",
            true,
            None::<&str>,
        )?;
        // Linux's default menu has no File submenu, so one is added there.
        let file = match submenu(&menu, "File")? {
            Some(file) => file,
            None => {
                let file = Submenu::new(handle, "File", true)?;
                menu.prepend(&file)?;
                file
            }
        };
        let separator = PredefinedMenuItem::separator(handle)?;
        let own: [&dyn tauri::menu::IsMenuItem<Wry>; 3] =
            [&open_folder, &reload_folder, &separator];
        file.prepend_items(&own)?;
        // On macOS both go in the app menu: Check for Updates joins About above
        // the default menu's first separator, since both are app-identity
        // items, and Settings gets a section of its own below it, as the
        // platform convention has it. Elsewhere they go at the end of File's
        // own items, above Close Window and Quit.
        #[cfg(target_os = "macos")]
        if let Some(MenuItemKind::Submenu(app)) = menu.items()?.into_iter().next() {
            app.insert_items(&[&check_updates], 1)?;
            app.insert_items(&[&settings, &PredefinedMenuItem::separator(handle)?], 3)?;
        }
        #[cfg(not(target_os = "macos"))]
        file.insert_items(
            &[
                &settings,
                &check_updates,
                &PredefinedMenuItem::separator(handle)?,
            ],
            own.len(),
        )?;
        // `Help` may be missing from the default menu (Linux), in which case
        // it's created here; empty on macOS; holding About elsewhere, where
        // the separator keeps the two apart.
        let help = match submenu(&menu, "Help")? {
            Some(help) => help,
            None => {
                let help = Submenu::new(handle, "Help", true)?;
                menu.append(&help)?;
                help
            }
        };
        #[cfg(target_os = "macos")]
        let open_log_folder = IconMenuItem::with_id(
            handle,
            OPEN_LOG_FOLDER_ID,
            "Open Log Folder",
            true,
            Some(Image::from_bytes(include_bytes!(
                "../icons/menu/folder.png"
            ))?),
            None::<&str>,
        )?;
        #[cfg(not(target_os = "macos"))]
        let open_log_folder = MenuItem::with_id(
            handle,
            OPEN_LOG_FOLDER_ID,
            "Open Log Folder",
            true,
            None::<&str>,
        )?;
        if help.items()?.is_empty() {
            help.prepend(&open_log_folder)?;
        } else {
            help.prepend_items(&[&open_log_folder, &PredefinedMenuItem::separator(handle)?])?;
        }
        // `Edit` opens with the predefined Undo and Redo, which only act on
        // editable content (neither window has any) and would own Cmd+Z, and
        // ends with the predefined Select All, which would own Cmd+A.
        if let Some(edit) = submenu(&menu, "Edit")? {
            for _ in 0..2 {
                if let Some(MenuItemKind::Predefined(_)) = edit.items()?.into_iter().next() {
                    edit.remove_at(0)?;
                }
            }
            #[cfg(target_os = "macos")]
            let undo = IconMenuItem::with_id(
                handle,
                UNDO_ID,
                "Undo",
                true,
                Some(Image::from_bytes(include_bytes!(
                    "../icons/menu/arrow.uturn.backward.png"
                ))?),
                undo_key,
            )?;
            #[cfg(not(target_os = "macos"))]
            let undo = MenuItem::with_id(handle, UNDO_ID, "Undo", true, undo_key)?;
            edit.insert(&undo, 0)?;
            #[cfg(target_os = "macos")]
            let redo = IconMenuItem::with_id(
                handle,
                REDO_ID,
                "Redo",
                true,
                Some(Image::from_bytes(include_bytes!(
                    "../icons/menu/arrow.uturn.forward.png"
                ))?),
                redo_key,
            )?;
            #[cfg(not(target_os = "macos"))]
            let redo = MenuItem::with_id(handle, REDO_ID, "Redo", true, redo_key)?;
            edit.insert(&redo, 1)?;
            let items = edit.items()?;
            if let Some(MenuItemKind::Predefined(_)) = items.last() {
                edit.remove_at(items.len() - 1)?;
            }
            let select_all =
                MenuItem::with_id(handle, SELECT_ALL_ID, "Select All", true, select_all_key)?;
            edit.append(&select_all)?;
        }
        // A check item has no icon variant, so there is no macOS twin.
        let view_items = VIEW_ITEMS
            .iter()
            .map(|&(id, action, label)| {
                let key = keymap.accelerator_for(action);
                CheckMenuItem::with_id(handle, id, label, true, true, key.as_deref())
            })
            .collect::<tauri::Result<Vec<_>>>()?;
        let view_kinds: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = view_items
            .iter()
            .map(|item| item as &dyn tauri::menu::IsMenuItem<Wry>)
            .collect();
        // macOS's default `View` holds Enter Full Screen, kept below a
        // separator; elsewhere there is no `View`, so one goes after `Edit`.
        #[cfg(target_os = "macos")]
        if let Some(view) = submenu(&menu, "View")? {
            view.prepend_items(&view_kinds)?;
            view.insert(&PredefinedMenuItem::separator(handle)?, view_kinds.len())?;
        }
        #[cfg(not(target_os = "macos"))]
        {
            let view = Submenu::new(handle, "View", true)?;
            view.append_items(&view_kinds)?;
            let mut after_edit = None;
            for (i, item) in menu.items()?.into_iter().enumerate() {
                if let MenuItemKind::Submenu(submenu) = item {
                    if submenu.text()? == "Edit" {
                        after_edit = Some(i + 1);
                    }
                }
            }
            match after_edit {
                Some(at) => menu.insert(&view, at)?,
                None => menu.append(&view)?,
            }
        }
        Ok(menu)
    }

    /// Make the keymap-derived accelerators match the keymap, then re-apply
    /// the `View` checks. muda's macOS `set_accelerator(None)` does not clear
    /// a key equivalent, so macOS replaces the whole menu. Elsewhere a
    /// runtime `set_menu` turns the Windows dark menu bar white, so an
    /// existing menu is patched in place.
    pub fn refresh(app: &AppHandle, keymap: &crate::shortcuts::Keymap) -> tauri::Result<()> {
        #[cfg(not(target_os = "macos"))]
        if let Some(menu) = app.menu() {
            for (id, action) in keyed_items() {
                let key = keymap.accelerator_for(action);
                let key = key.as_deref();
                let item = menu
                    .items()?
                    .iter()
                    .find_map(|kind| kind.as_submenu()?.get(id));
                match item {
                    Some(MenuItemKind::MenuItem(item)) => item.set_accelerator(key)?,
                    Some(MenuItemKind::Check(item)) => item.set_accelerator(key)?,
                    _ => {
                        log::warn!("menu item {id} not found; accelerator not updated");
                        return Ok(());
                    }
                }
            }
            apply_stored_panels(app);
            return Ok(());
        }
        let menu = build(app, keymap)?;
        app.set_menu(menu)?;
        apply_stored_panels(app);
        Ok(())
    }

    pub fn on_event(app: &AppHandle, event: MenuEvent) {
        // The frontend owns which folder is open, so it does the invoking.
        if event.id() == OPEN_FOLDER_ID {
            let _ = app.emit("open-folder", ());
        }
        if event.id() == RELOAD_FOLDER_ID {
            let _ = app.emit("reload-folder", ());
        }
        if event.id() == UNDO_ID {
            let _ = app.emit("undo", ());
        }
        if event.id() == REDO_ID {
            let _ = app.emit("redo", ());
        }
        if event.id() == SELECT_ALL_ID {
            let _ = app.emit("select-all", ());
        }
        if event.id() == CHECK_UPDATES_ID {
            crate::update::spawn(app.clone(), true);
        }
        if event.id() == OPEN_LOG_FOLDER_ID {
            if let Err(e) = open_log_folder(app) {
                log::error!("failed to open the log folder: {e}");
            }
        }
        if event.id() == SETTINGS_ID {
            let _ = app.emit("open-settings", ());
        }
        if let Some(&(_, action, _)) = VIEW_ITEMS.iter().find(|&&(id, _, _)| event.id() == id) {
            // muda toggles a check item on click; that is undone from the
            // stored panels, so a click the frontend gates leaves the check
            // as it was and an effective one is set by `set_panels`.
            apply_stored_panels(app);
            let _ = app.emit("menu-action", action);
        }
    }

    /// Show the folder holding `Riffle.log` in the platform's file manager.
    fn open_log_folder(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let dir = app.path().app_log_dir()?;
        // The plugin creates the folder on its first write, which has happened
        // by the time a menu can be clicked; this only covers a log-less run.
        std::fs::create_dir_all(&dir)?;
        app.opener()
            .open_path(dir.to_string_lossy(), None::<&str>)?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Actions with no menu item: `previous`, `next` and the burst moves
        /// are cursor moves the strip offers by click and wheel, on arrow keys
        /// every keyboard has; `extendPrevious` / `extendNext` are the
        /// keyboard's `Shift+click`; `grayscale` is held down, which a menu
        /// click cannot be; the judgments are in the strip's right-click menu
        /// and on plain keys; `toggleSides`, `focus`, `zoom`, `compare` and
        /// `performance` are main-view / strip keys, not menu items (`View`
        /// holds only the pane check items).
        const MENU_LESS: &[&str] = &[
            "previous",
            "next",
            "burstPrevious",
            "burstNext",
            "burstFramePrevious",
            "burstFrameNext",
            "extendPrevious",
            "extendNext",
            "grayscale",
            "toggleSides",
            "focus",
            "zoom",
            "compare",
            "performance",
            "rate1",
            "rate2",
            "rate3",
            "rate4",
            "rate5",
            "reject",
            "rejectRest",
            "pick",
            "unflag",
            "clear",
            "red",
            "orange",
            "yellow",
            "green",
            "blue",
            "pink",
            "purple",
            "clearlabel",
            "clearall",
        ];

        #[test]
        fn panel_checked_maps_the_pane_toggles() {
            let panels = serde_json::json!({"left": false, "strip": true, "right": false});
            assert_eq!(panel_checked(&panels, "toggleLeft"), Some(false));
            assert_eq!(panel_checked(&panels, "toggleRight"), Some(false));
            assert_eq!(panel_checked(&panels, "toggleStrip"), Some(true));
            assert_eq!(panel_checked(&panels, "toggleSides"), None);
            assert_eq!(panel_checked(&panels, "open"), None);
            let odd = serde_json::json!({"left": "no", "strip": null});
            assert_eq!(panel_checked(&odd, "toggleLeft"), Some(true));
            assert_eq!(panel_checked(&odd, "toggleRight"), Some(true));
            assert_eq!(panel_checked(&odd, "toggleStrip"), Some(true));
            assert_eq!(
                panel_checked(&serde_json::Value::Null, "toggleLeft"),
                Some(true)
            );
        }

        #[test]
        fn every_view_item_has_a_check() {
            for &(id, action, _) in VIEW_ITEMS {
                assert!(
                    panel_checked(&serde_json::json!({}), action).is_some(),
                    "{id}"
                );
            }
        }

        #[test]
        fn menu_covers_every_action() {
            let mut placed: Vec<&str> = keyed_actions().chain(MENU_LESS.iter().copied()).collect();
            let count = placed.len();
            placed.sort_unstable();
            placed.dedup();
            assert_eq!(placed.len(), count, "an action is placed twice");
            let mut actions: Vec<&str> = crate::shortcuts::Keymap::defaults()
                .bindings()
                .into_iter()
                .map(|b| b.action)
                .collect();
            actions.sort_unstable();
            assert_eq!(placed, actions);
        }
    }
}

use std::sync::{Arc, Mutex};

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, RunEvent};

use sidecar::SidecarFormat;

/// Whether timing logs are on; the settings modal toggles it, and it is kept
/// here so a reloaded main window picks it up again.
struct TimingLogs(AtomicBool);

#[tauri::command]
fn timing_logs(state: tauri::State<TimingLogs>) -> bool {
    state.0.load(Ordering::Relaxed)
}

#[tauri::command]
fn set_timing_logs(state: tauri::State<TimingLogs>, enabled: bool) {
    state.0.store(enabled, Ordering::Relaxed);
}

/// Write a frontend timing line to `Riffle.log`, so it can be read without
/// DevTools. It is `async` because a sync command would write the line on the
/// main thread, and the frontend calls it on hot paths.
#[tauri::command]
async fn log_timing(line: String) {
    log::info!("{line}");
}

/// Switch the sidecar format from the settings modal. The backend then emits
/// `sidecar-format` so the frontend reopens the folder in the new format.
#[tauri::command]
async fn set_sidecar_format(app: AppHandle, format: String) -> Result<(), String> {
    let format = SidecarFormat::from_setting(Some(&format));
    let unchanged = *index::lock(&app.state::<commands::AppSidecarFormat>().0) == format;
    // The switch drains the writer and touches SQLite and the settings file,
    // none of which may block the main thread.
    let switched = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        commands::switch_sidecar_format(&switched, format)
    })
    .await
    .map_err(|e| e.to_string())??;
    if !unchanged {
        let _ = app.emit("sidecar-format", format.setting());
    }
    Ok(())
}

/// Save the format picked in the main window's first-launch dialog. Unlike
/// `set_sidecar_format` it saves an unchanged format too, so the dialog does
/// not come back on the next launch, and a failed save is returned so the
/// dialog stays up.
#[tauri::command]
async fn choose_sidecar_format(app: AppHandle, format: String) -> Result<(), String> {
    let format = SidecarFormat::from_setting(Some(&format));
    let unchanged = *index::lock(&app.state::<commands::AppSidecarFormat>().0) == format;
    let chosen = app.clone();
    tauri::async_runtime::spawn_blocking(move || commands::choose_sidecar_format(&chosen, format))
        .await
        .map_err(|e| e.to_string())??;
    if !unchanged {
        let _ = app.emit("sidecar-format", format.setting());
    }
    Ok(())
}

/// Whether this is a development build, which shows the settings modal's
/// Debug section; a distributable build leaves the `devtools` feature off.
#[tauri::command]
fn debug_build() -> bool {
    cfg!(any(feature = "devtools", debug_assertions))
}

/// The payload of the `sidecar-error` event: a sidecar that could not be
/// written. The writer retries it a few times with a growing delay, and the
/// judgment stays in the index to be written on the next open if all fail.
#[derive(Clone, serde::Serialize)]
struct SidecarError {
    path: String,
    message: String,
}

fn main() {
    let builder = tauri::Builder::default()
        // Registered first, as the plugin requires: a second launch would
        // contend with this one for the SQLite index and the sidecar writer,
        // so it focuses the existing window and exits instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                // The plugin's default cap is 40 KB, which a single ~30s
                // scan can fill on its own: `folder_entries` is re-read
                // while a `scan-progress` stream leaves the focused row
                // missing, so its `open entries` line alone runs to ~300
                // lines of ~120 bytes. The timing lines are the whole point
                // of the file, so the cap is raised rather than the lines
                // thinned out.
                .max_file_size(1_000_000)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::new().build());
    builder
        .on_menu_event(app_menu::on_event)
        .setup(|app| {
            diagnostics::install_panic_hook();
            // The window is created hidden so the window-state plugin can
            // restore its geometry first. The plugin shows it afterwards, but
            // not when the saved state has `visible: false` (a quit while
            // minimized on macOS) or when `set_position` / `set_size` /
            // `set_fullscreen` fails and aborts `restore_state` before its
            // `show()`, so show it here unconditionally.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            let path = app.path().app_cache_dir()?.join("index.sqlite");
            // The index is a thumbnail/metadata cache, not required data: an
            // unwritable cache dir degrades to "no thumbnails" rather than
            // stopping the app from launching.
            let index = match index::Index::open(&path) {
                Ok(index) => Some(Arc::new(Mutex::new(index))),
                Err(e) => {
                    log::error!("failed to open the index cache at {}: {e}", path.display());
                    None
                }
            };
            let reader = index
                .as_ref()
                .map(|index| match index::Index::open_reader(&path) {
                    Ok(reader) => Arc::new(Mutex::new(reader)),
                    Err(e) => {
                        log::error!("failed to open the index reader at {}: {e}", path.display());
                        index.clone()
                    }
                });
            let writer = index.as_ref().map(|index| {
                let handle = app.handle().clone();
                // `Emitter` is safe from any thread, as `run_scan` already
                // relies on, so the writer thread reports its own failures.
                sidecar::Writer::spawn(index.clone(), move |path, message| {
                    log::warn!("sidecar write failed: path={} {message}", path.display());
                    let _ = handle.emit(
                        "sidecar-error",
                        SidecarError {
                            path: path.to_string_lossy().into_owned(),
                            message: message.to_string(),
                        },
                    );
                })
            });
            let (format, keymap, auto_advance, label_names, mcp_enabled, ai_enabled) =
                commands::load_settings(app.handle());
            app.manage(TimingLogs(AtomicBool::new(false)));
            app.manage(perf::AppPerf::default());
            app.manage(commands::AppSidecarFormat(Mutex::new(format)));
            app.manage(commands::AppLabelNames(Mutex::new(label_names)));
            app.manage(import::AppImportSettings(Mutex::new(
                import::load_settings(app.handle()),
            )));
            app.manage(commands::AppAutoAdvance(AtomicBool::new(auto_advance)));
            app.manage(commands::AppPanels(Mutex::new(commands::stored_panels(
                app.handle(),
            ))));
            if let Err(e) = app_menu::refresh(app.handle(), &keymap) {
                log::error!("failed to set the app menu: {e}");
            }
            app.manage(commands::AppKeymap(Mutex::new(keymap)));
            app.manage(commands::AppSwitchLock(Mutex::new(())));
            app.manage(commands::AppWriter(writer));
            app.manage(commands::AppIndex(index));
            app.manage(commands::AppIndexReader(reader));
            app.manage(commands::Scans::default());
            app.manage(ai::AppAi::new(ai_enabled, ai::model_path(app.handle())));
            app.manage(rank::AppRank::default());
            rank::refit_in_background(app.handle());
            // A model still missing at launch (a download cut short by a
            // quit or a failure) is fetched again, as a later enable would.
            if ai_enabled {
                ai::switch(app.handle(), true);
            }
            app.manage(commands::EyesRequests::default());
            app.manage(sequence::Sequences::default());
            app.manage(import::ImportPreviews::default());
            app.manage(import::Imports::default());
            app.manage(trash::Runs::default());
            app.manage(commands::AppListing::default());
            app.manage(watch::Watch::spawn(app.handle().clone()));
            app.manage(treewatch::TreeWatch::spawn(app.handle().clone()));
            commands::spawn_eviction(app.handle().clone());
            app.manage(update::UpdateRun::default());
            update::spawn(app.handle().clone(), false);
            app.manage(mcp::AppMcp::new(
                mcp::Bridge::to_main_window(app.handle().clone()),
                app.state::<commands::AppIndexReader>().0.clone(),
            ));
            // A failed bind (the port in use) is reported in the settings
            // window rather than stopping the app from launching.
            if mcp_enabled {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    mcp::switch(&handle, true).await;
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_folder,
            commands::list_arw,
            commands::remember_folder,
            commands::last_folder,
            commands::sort_order,
            commands::set_sort_order,
            commands::filter,
            commands::set_filter,
            commands::panels,
            commands::set_panels,
            commands::preview,
            commands::preview_pixel_limit,
            commands::dropped_folder,
            folders::folder_roots,
            folders::list_subfolders,
            folders::reveal_label,
            folders::reveal_folder,
            treewatch::set_tree_watches,
            commands::scan_folder,
            commands::start_scan,
            commands::set_scan_focus,
            commands::folder_entries,
            commands::genres_of,
            rank::ranking_of,
            rank::ranking_model,
            rank::refit_ranking,
            rank::refit_ranking_in_background,
            commands::last_viewed,
            commands::set_last_viewed,
            commands::viewed_files,
            commands::mark_viewed,
            commands::thumbnail,
            commands::metadata,
            commands::focus_crop,
            commands::faces_of,
            commands::eyes_of,
            commands::set_rating,
            commands::sidecar_format,
            commands::sidecar_format_saved,
            commands::shortcuts,
            commands::add_shortcut_key,
            commands::remove_shortcut_key,
            commands::reset_shortcut,
            commands::reset_shortcuts,
            commands::auto_advance,
            commands::set_auto_advance,
            commands::mcp_enabled,
            commands::set_mcp_enabled,
            commands::ai_enabled,
            commands::set_ai_enabled,
            commands::mcp_reply,
            commands::label_names,
            commands::set_label_names,
            commands::index_size,
            commands::clear_index,
            commands::trash_files,
            commands::trash_rejected_undo,
            commands::trash_rejected_redo,
            datefolder::date_folder_preview,
            datefolder::date_folder_run,
            foldersidecars::rewrite_sidecars_preview,
            foldersidecars::rewrite_sidecars_run,
            foldersidecars::delete_sidecars_preview,
            foldersidecars::delete_sidecars_run,
            rename::rename_folder,
            rename::rename_file,
            import::import_settings,
            import::set_import_root,
            import::set_import_pattern,
            import::import_pattern_example,
            import::import_preview,
            import::import_run,
            import::import_cancel,
            sequence::sequence_preview,
            sequence::sequence_run,
            sequence::sequence_cancel,
            set_sidecar_format,
            choose_sidecar_format,
            debug_build,
            timing_logs,
            set_timing_logs,
            log_timing,
            perf::set_perf_sampling,
            diagnostics::log_frontend
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // A judgment may still be inside the writer's debounce window
            // when the user quits, so the exit waits on an explicit drain
            // rather than on the writer's `Drop`, which is not guaranteed to
            // run during teardown.
            // The folder watcher needs nothing here: it holds nothing
            // pending on disk, and its thread ends with the app.
            if let RunEvent::ExitRequested { .. } = event {
                if let Some(writer) = &app.state::<commands::AppWriter>().0 {
                    writer.flush(sidecar::DRAIN_TIMEOUT);
                }
                update::install_pending(app);
                tauri::async_runtime::block_on(mcp::stop(&app.state::<mcp::AppMcp>()));
            }
        });
}
