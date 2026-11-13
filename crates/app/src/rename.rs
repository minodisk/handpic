//! The Tauri commands that rename a folder from the folder tree and a RAW
//! file from the strip: wrappers over `riffle_backend::rename` that drain the
//! sidecar writer, refuse a running scan under the `Scans` lock and stop the
//! embedding pass.

use std::path::Path;

use riffle_backend::rename::{file_plan, file_run, folder_target, Context, Renamed};
use riffle_backend::text::SCAN_RUNNING;
use riffle_backend::treewatch::TreeWatch;
use riffle_backend::watch::Watch;
use tauri::Manager;

use crate::commands::{AppIndex, AppWriter, Scans};
use crate::index;
use crate::trash::Runs;

/// Rename the folder `dir` to `name` (`riffle_backend::rename::rename_folder`),
/// refusing while a scan runs. The sidecar writer is drained first and the
/// `Scans` lock held across the rename and the index write, as in
/// `trash_files`.
#[tauri::command]
pub async fn rename_folder(
    app: tauri::AppHandle,
    dir: String,
    name: String,
) -> Result<Renamed, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || rename_folder_blocking(&app, &dir, &name))
        .await
        .map_err(|e| e.to_string())?
}

/// The body of `rename_folder`, run on a blocking thread after its first
/// scan check, shared with the one-date branch of `date_folder_run`. The
/// name is checked before the writer is drained, so a bad name is refused
/// first.
pub(crate) fn rename_folder_blocking(
    app: &tauri::AppHandle,
    dir: &str,
    name: &str,
) -> Result<Renamed, String> {
    folder_target(Path::new(dir), name)?;
    if let Some(writer) = &app.state::<AppWriter>().0 {
        writer.flush(crate::sidecar::DRAIN_TIMEOUT);
    }
    let scans = app.state::<Scans>();
    let state = index::lock(&scans.0);
    if state.scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    state.stop_embedding();
    let index = app.state::<AppIndex>();
    let cx = Context {
        watch: &app.state::<Watch>(),
        trees: &app.state::<TreeWatch>(),
        index: index.0.as_deref(),
        runs: &app.state::<Runs>(),
    };
    let renamed = riffle_backend::rename::rename_folder(&cx, dir, name);
    drop(state);
    renamed
}

/// Rename the RAW `path` in the folder `dir` to `name`, with its sidecars,
/// refusing while a scan runs. The guard order is `rename_folder`'s, without
/// the watcher release: the watcher holds the directory, not its files. The
/// plan is made after the writer is drained, so a sidecar a pending judgment
/// has just written moves too. A failed index write after a successful
/// rename still returns `Ok` with a warning, as in `rename_folder`.
/// `Renamed.path` is the canonical folder joined with `name`, the spelling
/// `list_arw` lists.
#[tauri::command]
pub async fn rename_file(
    app: tauri::AppHandle,
    dir: String,
    path: String,
    name: String,
) -> Result<Renamed, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(writer) = &app.state::<AppWriter>().0 {
            writer.flush(crate::sidecar::DRAIN_TIMEOUT);
        }
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
        state.stop_embedding();
        let dir = std::fs::canonicalize(&dir).map_err(|e| format!("{dir}: {e}"))?;
        let plan = file_plan(&dir, &path, &name)?;
        file_run(plan, |from, to| {
            std::fs::rename(from, to).map_err(|e| {
                let from = from.file_name().unwrap_or_default().to_string_lossy();
                format!("{from}: {e}")
            })
        })?;
        let new = dir.join(&name).to_string_lossy().into_owned();
        let mut warning = None;
        if let Some(index) = app.state::<AppIndex>().0.clone() {
            match index::lock(&index).rename_file(&path, &new) {
                Ok(()) => log::info!("renamed file: {path} -> {new}"),
                Err(e) => {
                    log::warn!("renamed file {path} -> {new} but not its index rows: {e}");
                    warning = Some(format!(
                        "the file was renamed but its cache was not ({e}); \
                         it is rebuilt on the next open"
                    ));
                }
            }
        }
        drop(state);
        Ok(Renamed { path: new, warning })
    })
    .await
    .map_err(|e| e.to_string())?
}
