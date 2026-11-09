//! The Tauri command over `riffle_backend::output`.

use std::path::Path;

use riffle_backend::listing::canonicalize;
use riffle_backend::output::outputs_in;

/// The `files` of `dir` that have an export. `None` when `dir` has no export
/// folder, so that no file is taken for not exported; an error when one could
/// not be read whole.
#[tauri::command]
pub async fn outputs_of(dir: String, files: Vec<String>) -> Result<Option<Vec<String>>, String> {
    tauri::async_runtime::spawn_blocking(move || outputs_in(Path::new(&canonicalize(&dir)), &files))
        .await
        .map_err(|e| e.to_string())?
}
