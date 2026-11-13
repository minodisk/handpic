//! The folder tree's `Rewrite Sidecars from Index…` and `Delete Sidecars…`
//! commands: wrappers over `riffle_backend::foldersidecars` that drain the
//! sidecar writer and refuse a running scan, the runs under the `Scans` lock.

use riffle_backend::foldersidecars::{
    delete_preview, delete_run, doomed, preview, rewrite, DeletePreview, Preview, Summary,
    REWRITE_DRAIN,
};
use riffle_backend::text::{format_bytes, SCAN_RUNNING, SIZE_BASE};
use tauri::Manager;

use crate::commands::{canonicalize, AppIndex, AppLabelNames, AppSidecarFormat, AppWriter, Scans};
use crate::index;
use crate::sidecar::DRAIN_TIMEOUT;
use crate::trash;

/// Count what `rewrite_sidecars_run` would write in `dir`, for the
/// confirmation dialog. A running scan is refused and the sidecar writer is
/// drained first, so the rows read are the flushed ones. The `Scans` lock is
/// not held: the dialog stays open for as long as the user wants.
#[tauri::command]
pub async fn rewrite_sidecars_preview(
    app: tauri::AppHandle,
    dir: String,
) -> Result<Preview, String> {
    {
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<Preview, String> {
        if let Some(writer) = &app.state::<AppWriter>().0 {
            writer.flush(DRAIN_TIMEOUT);
        }
        let Some(index) = app.state::<AppIndex>().0.clone() else {
            return Err("no index cache available".to_string());
        };
        let format = *index::lock(&app.state::<AppSidecarFormat>().0);
        preview(&canonicalize(&dir), &index, format)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Rewrite the sidecars of `dir` from its `ratings` rows once the dialog
/// `rewrite_sidecars_preview` fed was confirmed. The writer is drained first,
/// as in `trash_files`, and a running scan is refused under the
/// `Scans` lock, which is held across the index write and the drain.
#[tauri::command]
pub async fn rewrite_sidecars_run(app: tauri::AppHandle, dir: String) -> Result<Summary, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Summary, String> {
        let Some(writer) = &app.state::<AppWriter>().0 else {
            return Err("the sidecar writer is not running".to_string());
        };
        writer.flush(DRAIN_TIMEOUT);
        let Some(index) = app.state::<AppIndex>().0.clone() else {
            return Err("no index cache available".to_string());
        };
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
        let format = *index::lock(&app.state::<AppSidecarFormat>().0);
        let names = index::lock(&app.state::<AppLabelNames>().0).clone();
        let dir = canonicalize(&dir);
        let summary = rewrite(&dir, &index, writer, format, &names, REWRITE_DRAIN);
        log::info!("rewrote the sidecars of {dir}: {summary:?}");
        drop(state);
        summary
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Count the sidecars `delete_sidecars_run` would move from `dir`, for the
/// confirmation dialog. A running scan is refused and the sidecar writer is
/// drained first, so no pending write mints a sidecar the count misses. The
/// `Scans` lock is not held: the dialog stays open for as long as the user
/// wants, and the run collects again.
#[tauri::command]
pub async fn delete_sidecars_preview(
    app: tauri::AppHandle,
    dir: String,
) -> Result<DeletePreview, String> {
    {
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<DeletePreview, String> {
        if let Some(writer) = &app.state::<AppWriter>().0 {
            writer.flush(DRAIN_TIMEOUT);
        }
        let format = *index::lock(&app.state::<AppSidecarFormat>().0);
        let doomed = doomed(&canonicalize(&dir), format)?;
        Ok(delete_preview(&doomed, format, |bytes| {
            format_bytes(bytes, SIZE_BASE)
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Move the sidecars of `dir` to the OS Trash once the dialog
/// `delete_sidecars_preview` fed was confirmed, record the run for
/// `trash_rejected_undo`, and clear the judgments of the RAWs that lost a
/// sidecar in the index, so a closed folder does not keep them until its next
/// open. The writer is drained first: a judgment still in its debounce window
/// would mint a sidecar right after the delete. A running scan is refused
/// under the `Scans` lock, which is held across the moves and the index
/// write.
#[tauri::command]
pub async fn delete_sidecars_run(
    app: tauri::AppHandle,
    dir: String,
) -> Result<trash::Summary, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<trash::Summary, String> {
        if let Some(writer) = &app.state::<AppWriter>().0 {
            writer.flush(DRAIN_TIMEOUT);
        }
        let scans = app.state::<Scans>();
        let state = index::lock(&scans.0);
        if state.scanning() {
            return Err(SCAN_RUNNING.to_string());
        }
        let format = *index::lock(&app.state::<AppSidecarFormat>().0);
        let dir = canonicalize(&dir);
        let summary = delete_run(
            &dir,
            format,
            app.state::<AppIndex>().0.as_deref(),
            &app.state::<trash::Runs>(),
        );
        drop(state);
        summary
    })
    .await
    .map_err(|e| e.to_string())?
}
