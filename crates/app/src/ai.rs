//! The `aiEnabled` setting over the backend's `ai`: the model file's place
//! in the app's local data dir and the `ai-state` event.

use std::path::PathBuf;

use riffle_backend::ai::Ai;
pub use riffle_backend::ai::AiState;
use tauri::{AppHandle, Emitter, Manager};

/// The backend's switch and model download, reporting as `ai-state`.
pub struct AppAi(Ai);

impl AppAi {
    pub fn new(app: &AppHandle, enabled: bool) -> Self {
        let emitter = app.clone();
        AppAi(Ai::new(enabled, model_path(app), move |state| {
            let _ = emitter.emit("ai-state", state);
        }))
    }

    pub fn enabled(&self) -> bool {
        self.0.enabled()
    }

    /// The model file the third pass embeds with: `None` while `aiEnabled`
    /// is off or the file is not in place yet.
    pub fn model_if_enabled(&self) -> Option<PathBuf> {
        self.0.model_if_enabled()
    }
}

/// The model file's path under the app's local data dir.
pub fn model_path(app: &AppHandle) -> Option<PathBuf> {
    match app.path().app_local_data_dir() {
        Ok(dir) => Some(dir.join(riffle_core::embed::MODEL_FILE)),
        Err(e) => {
            log::error!("no local data dir for the embedding model: {e}");
            None
        }
    }
}

pub fn state(app: &AppHandle) -> AiState {
    app.state::<AppAi>().0.state()
}

/// Turn AI analysis on or off and emit `ai-state` (see the backend's
/// `Ai::switch`).
pub fn switch(app: &AppHandle, enabled: bool) -> AiState {
    app.state::<AppAi>().0.switch(enabled)
}
