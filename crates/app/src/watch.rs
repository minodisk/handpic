//! The Tauri side of `riffle_backend::watch`: the `folder-changed` event and
//! the managed `Watch`.

use riffle_backend::watch::Watch;
use tauri::{Emitter, Manager};

/// The payload of `folder-changed`: which folder to rescan, so a listener that
/// outlives the open folder can drop an event for another one.
#[derive(serde::Serialize, Clone)]
struct Changed {
    dir: String,
}

pub fn spawn(app: tauri::AppHandle) -> Watch {
    Watch::spawn(move |dir| {
        let _ = app.emit("folder-changed", Changed { dir });
    })
}

pub fn set(app: &tauri::AppHandle, dir: &str, owner: &str) {
    riffle_backend::watch::set(&app.state::<Watch>(), dir, owner);
}
