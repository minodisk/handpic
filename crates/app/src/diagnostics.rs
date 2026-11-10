//! The Tauri command that writes the frontend's failures to `Handpic.log`, and
//! the backend's diagnostics under the path the app calls them by.

pub use riffle_backend::diagnostics::{install_panic_hook, invariant, invariant_line};

/// Write a frontend failure line to `Handpic.log`. It is `async` with owned
/// arguments so it does not run on the main thread.
#[tauri::command]
pub async fn log_frontend(kind: String, line: String) {
    match kind.as_str() {
        "uncaught-js" => log::error!("uncaught-js: {line}"),
        "invariant" => log::warn!("invariant: {line}"),
        _ => log::warn!("{kind}: {line}"),
    }
}
