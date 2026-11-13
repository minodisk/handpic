//! The Tauri side of `riffle_backend::treewatch`: the `tree-changed` event,
//! the managed `TreeWatch` and the `set_tree_watches` command.

use riffle_backend::treewatch::TreeWatch;
use tauri::{Emitter, Manager};

/// The payload of `tree-changed`: the tree path of the folder to re-list.
#[derive(serde::Serialize, Clone)]
struct Changed {
    dir: String,
}

pub fn spawn(app: tauri::AppHandle) -> TreeWatch {
    TreeWatch::spawn(move |dir| {
        let _ = app.emit("tree-changed", Changed { dir });
    })
}

/// Make the tree's watched set equal to `dirs`, the frontend's paths of its
/// expanded, drawn folders (see `TreeWatch::set`).
#[tauri::command]
pub async fn set_tree_watches(app: tauri::AppHandle, dirs: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<TreeWatch>().set(&dirs);
    })
    .await
    .map_err(|e| e.to_string())
}

/// `riffle_backend::treewatch::with_released` over the managed `TreeWatch`.
pub fn with_released<T>(
    app: &tauri::AppHandle,
    dir: &str,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    riffle_backend::treewatch::with_released(&app.state::<TreeWatch>(), dir, f)
}
