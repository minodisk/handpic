//! The MCP companion over the backend's `mcp`: the bridge to the main window
//! (`mcp-request`, answered through `mcp_reply`) and the `mcp-state` event.

use std::sync::{Arc, Mutex};

pub use riffle_backend::mcp::McpState;
use riffle_backend::mcp::{Bridge, Mcp};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::index::Index;

const MAIN_WINDOW: &str = "main";

/// The backend's server, asking the main window over `mcp-request`.
pub struct AppMcp(Mcp);

impl AppMcp {
    pub fn new(app: &AppHandle, index: Option<Arc<Mutex<Index>>>) -> Self {
        let app = app.clone();
        AppMcp(Mcp::new(
            Bridge::new(move |request| {
                if app.get_webview_window(MAIN_WINDOW).is_none() {
                    return Err("Handpic's main window is not open".to_string());
                }
                app.emit_to(MAIN_WINDOW, "mcp-request", request)
                    .map_err(|e| e.to_string())
            }),
            index,
        ))
    }

    /// Complete a bridge call with the main window's answer.
    pub fn reply(&self, id: u64, ok: bool, value: Value) {
        self.0.reply(id, ok, value);
    }
}

/// Stop the server if it is running, waiting briefly for open connections.
pub async fn stop(mcp: &AppMcp) {
    mcp.0.stop().await;
}

/// The current state, as the settings modal shows it.
pub async fn state(app: &AppHandle) -> McpState {
    app.state::<AppMcp>().0.state().await
}

/// Turn the server on or off and emit `mcp-state` (see the backend's
/// `Mcp::switch`).
pub async fn switch(app: &AppHandle, enabled: bool) -> McpState {
    let state = app.state::<AppMcp>().0.switch(enabled).await;
    let _ = app.emit("mcp-state", state.clone());
    state
}
