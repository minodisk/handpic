//! The folder tree's `Rename by Capture Date…` commands: `date_folder_preview`
//! serializes `riffle_backend::datefolder`'s plan for the confirmation dialog
//! and `date_folder_run` carries it out, both refusing a running scan, the
//! run draining the sidecar writer and holding the `Scans` lock across the
//! backend's split.

use std::path::Path;

use riffle_backend::datefolder::{
    file_name, plan_of, preview, split_folder, Kind, Plan, Preview, Ran, Split,
};
use riffle_backend::rename::Context;
use riffle_backend::text::SCAN_RUNNING;
use riffle_backend::treewatch::TreeWatch;
use riffle_backend::watch::Watch;
use tauri::Manager;

use crate::commands::{canonicalize, AppIndex, AppWriter, Scans};
use crate::index;
use crate::rename::rename_folder_blocking;
use crate::trash::Runs;

/// Plan the date rename or split of `dir` for the confirmation dialog,
/// refusing while a scan runs. Nothing on disk changes; the run plans again.
#[tauri::command]
pub async fn date_folder_preview(app: tauri::AppHandle, dir: String) -> Result<Preview, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<Preview, String> {
        let dir = canonicalize(&dir);
        let plan = plan_of(app.state::<AppIndex>().0.as_deref(), &dir)?;
        Ok(preview(Path::new(&dir), plan))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Split the canonical `dir` (spelled `caller` by the frontend) along
/// `plan` (`riffle_backend::datefolder::split_folder`), on a blocking
/// thread. The guards are `rename_folder`'s: the sidecar writer is drained
/// first, so a sidecar a pending judgment has just written moves too, and the
/// `Scans` lock is held across the moves and the index writes.
fn split_blocking(
    app: &tauri::AppHandle,
    dir: &str,
    caller: &str,
    plan: &Plan,
) -> Result<Split, String> {
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
    let split = split_folder(&cx, dir, caller, plan);
    drop(state);
    split
}

/// Carry out the date rename or split of `dir`, refusing while a scan runs.
/// The folder is planned again (the preview is not trusted across the
/// dialog) and the run refused on any collision or when the folder is
/// already named its date. One date renames the folder as `rename_folder`
/// does; several dates split it (`split_blocking`).
#[tauri::command]
pub async fn date_folder_run(app: tauri::AppHandle, dir: String) -> Result<Ran, String> {
    if index::lock(&app.state::<Scans>().0).scanning() {
        return Err(SCAN_RUNNING.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<Ran, String> {
        let canonical = canonicalize(&dir);
        let plan = plan_of(app.state::<AppIndex>().0.as_deref(), &canonical)?;
        if !plan.collisions.is_empty() {
            return Err(format!("{} already exists", plan.collisions.join(", ")));
        }
        match plan.kind {
            Kind::Named => Err(format!(
                "{} is already named its capture date",
                file_name(Path::new(&canonical))
            )),
            Kind::Rename => {
                rename_folder_blocking(&app, &dir, &plan.groups[0].name).map(Ran::Renamed)
            }
            Kind::Split => split_blocking(&app, &canonical, &dir, &plan).map(Ran::Split),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
