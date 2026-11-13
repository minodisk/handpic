//! The folder tree's commands over `riffle_backend::folders`, and revealing
//! a folder in the OS file manager.

use riffle_backend::folders::{list, roots, Folder, FolderNode, REVEAL_LABEL};
use std::path::Path;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn folder_roots(app: AppHandle) -> Result<Vec<FolderNode>, String> {
    let home = app.path().home_dir().ok();
    tauri::async_runtime::spawn_blocking(move || roots(home))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_subfolders(dir: String) -> Result<Folder, String> {
    tauri::async_runtime::spawn_blocking(move || list(Path::new(&dir)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn reveal_label() -> &'static str {
    REVEAL_LABEL
}

/// Show `path`, a folder or a file, selected in its parent folder in the OS
/// file manager.
#[tauri::command]
pub async fn reveal_folder(app: AppHandle, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.opener()
            .reveal_item_in_dir(&path)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
