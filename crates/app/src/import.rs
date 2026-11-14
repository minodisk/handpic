//! Import from a memory card: the import settings in use, persisted under
//! `importRoot` and `importPattern`, their commands, the previewed plan kept
//! for the run, and the one run that copies it, over the backend's `import`.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use riffle_backend::import::{
    example, log_line, name_of, pattern_setting, plan_dir, prepare, refusal, root_setting,
    run_plan, summarize, validate_pattern, ImportSettings, Plan, Preview, Ran,
};
use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::commands::canonicalize;
use crate::index;

/// The import settings in use, loaded at launch and replaced by the setters.
pub struct AppImportSettings(pub Mutex<ImportSettings>);

/// The import settings from the store at launch; a store that cannot be read
/// is logged and gives the defaults.
pub fn load_settings(app: &tauri::AppHandle) -> ImportSettings {
    let store = crate::commands::settings(app)
        .inspect_err(|e| log::warn!("failed to open the settings: {e}"))
        .ok();
    let get = |key: &str| store.as_ref().and_then(|store| store.get(key));
    ImportSettings {
        root: root_setting(get("importRoot").as_ref()),
        pattern: pattern_setting(get("importPattern").as_ref()),
    }
}

/// Persist `value` under `key`; a failure is logged and the in-memory change
/// stands.
fn save(app: &tauri::AppHandle, key: &str, value: &str) {
    let saved = crate::commands::settings(app).and_then(|store| {
        store.set(key, value);
        store.save().map_err(|e| e.to_string())
    });
    if let Err(e) = saved {
        log::warn!("failed to save {key}: {e}");
    }
}

/// The destination root and the folder pattern.
#[tauri::command]
pub fn import_settings(app: tauri::AppHandle) -> ImportSettings {
    index::lock(&app.state::<AppImportSettings>().0).clone()
}

/// Set the destination root and persist it under `importRoot`.
#[tauri::command]
pub fn set_import_root(app: tauri::AppHandle, path: String) -> ImportSettings {
    let settings = {
        let state = app.state::<AppImportSettings>();
        let mut current = index::lock(&state.0);
        current.root = path;
        current.clone()
    };
    save(&app, "importRoot", &settings.root);
    settings
}

/// Set the folder pattern and persist it under `importPattern`; an invalid
/// pattern is refused with the reason and the stored one kept.
#[tauri::command]
pub fn set_import_pattern(
    app: tauri::AppHandle,
    pattern: String,
) -> Result<ImportSettings, String> {
    validate_pattern(&pattern)?;
    let settings = {
        let state = app.state::<AppImportSettings>();
        let mut current = index::lock(&state.0);
        current.pattern = pattern;
        current.clone()
    };
    save(&app, "importPattern", &settings.pattern);
    Ok(settings)
}

/// Where today's photos would go under `pattern` and the stored root, or why
/// the pattern is refused.
#[tauri::command]
pub fn import_pattern_example(app: tauri::AppHandle, pattern: String) -> Result<String, String> {
    let root = index::lock(&app.state::<AppImportSettings>().0)
        .root
        .clone();
    example(&root, &pattern, chrono::Local::now().date_naive())
}

/// The previewed plan the run takes by id, replaced by the next preview.
#[derive(Default)]
pub struct ImportPreviews {
    next_id: AtomicU64,
    plan: Mutex<Option<(u64, Previewed)>>,
}

/// A stored plan with the source folder and the destination root it was
/// made for, which the run's log line names.
struct Previewed {
    source: String,
    root: String,
    plan: Plan,
}

/// Plan the import of every photo under `dir` into the destination root and
/// pattern in use, reading each file's capture time from its head, and keep
/// the plan for the run. Nothing on disk changes.
#[tauri::command]
pub async fn import_preview(app: tauri::AppHandle, dir: String) -> Result<Preview, String> {
    let settings = index::lock(&app.state::<AppImportSettings>().0).clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Preview, String> {
        let source = canonicalize(&dir);
        if let Some(reason) = refusal(&settings) {
            let mut preview = summarize(
                None,
                source,
                &Plan {
                    destinations: Vec::new(),
                    undated: Vec::new(),
                },
            );
            preview.refusal = Some(reason);
            return Ok(preview);
        }
        let plan = plan_dir(Path::new(&source), &settings)?;
        let previews = app.state::<ImportPreviews>();
        let id = previews.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let preview = summarize(Some(id), source.clone(), &plan);
        *index::lock(&previews.plan) = Some((
            id,
            Previewed {
                source,
                root: settings.root,
                plan,
            },
        ));
        Ok(preview)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The id of the last run handed out and the cancel flag and join handle of
/// the one import that may be in progress, behind one lock so that the
/// "already running" check and the store are never interleaved between two
/// `import_run` calls (`sequence::Sequences`' shape).
#[derive(Default)]
pub struct Imports(Mutex<RunState>);

#[derive(Default)]
struct RunState {
    next_id: u64,
    /// The run in progress, cleared by its own task when it ends.
    running: Option<(u64, Arc<AtomicBool>, tauri::async_runtime::JoinHandle<()>)>,
}

impl RunState {
    fn finish(&mut self, run_id: u64) {
        if self
            .running
            .as_ref()
            .is_some_and(|(id, _, _)| *id == run_id)
        {
            self.running = None;
        }
    }
}

const RUN_IN_PROGRESS: &str = "an import is already in progress";

/// Mint a run id and store the task `spawn` starts for it, or refuse while
/// another run is in progress. The lock is held across `spawn`, so the task
/// cannot clear its entry before it is stored.
fn begin(
    imports: &Imports,
    spawn: impl FnOnce(u64, Arc<AtomicBool>) -> tauri::async_runtime::JoinHandle<()>,
) -> Result<u64, String> {
    let mut state = index::lock(&imports.0);
    if state.running.is_some() {
        return Err(RUN_IN_PROGRESS.to_string());
    }
    state.next_id += 1;
    let run_id = state.next_id;
    let cancel = Arc::new(AtomicBool::new(false));
    let handle = spawn(run_id, cancel.clone());
    state.running = Some((run_id, cancel, handle));
    Ok(run_id)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RunProgress {
    run_id: u64,
    files_done: usize,
    files_total: usize,
    bytes_done: u64,
    bytes_total: u64,
    file: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Done {
    run_id: u64,
    #[serde(flatten)]
    ran: Ran,
}

/// What `import_run` returns: the run's id and the destination folders it
/// created, in date order, which exist (still empty or not) by then.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    run_id: u64,
    folders: Vec<String>,
}

/// Create the destination folders of the previewed plan `preview_id`, copy
/// it in the background and return the run's id and those folders. Streams
/// `import-progress` and ends with one `import-done`, which follows every
/// progress event of the run because the copy only returns after its last
/// progress callback. The index is not touched.
#[tauri::command]
pub async fn import_run(app: tauri::AppHandle, preview_id: u64) -> Result<Started, String> {
    let (source, root, plan) = match &*index::lock(&app.state::<ImportPreviews>().plan) {
        Some((id, previewed)) if *id == preview_id => (
            previewed.source.clone(),
            previewed.root.clone(),
            previewed.plan.clone(),
        ),
        _ => return Err("the import preview is out of date; choose Import… again".to_string()),
    };
    let imports = app.state::<Imports>();
    // `prepare` creates the folders on the blocking task, off the runtime and
    // with no lock held; it hands them back here once they exist.
    let (folders_tx, mut folders_rx) = tauri::async_runtime::channel::<Vec<String>>(1);
    let run_id = begin(&imports, |run_id, cancel| {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let prepared = prepare(&plan);
            let _ = folders_tx.blocking_send(prepared.folders.clone());
            let ran = run_plan(prepared, &cancel, |p, dst| {
                let _ = app.emit(
                    "import-progress",
                    RunProgress {
                        run_id,
                        files_done: p.files_done,
                        files_total: p.files_total,
                        bytes_done: p.bytes_done,
                        bytes_total: p.bytes_total,
                        file: name_of(dst),
                    },
                );
            });
            log::info!("{}", log_line(&source, &root, &ran));
            // Cleared before `import-done`, so a run started in answer to it
            // is not refused.
            index::lock(&app.state::<Imports>().0).finish(run_id);
            let _ = app.emit("import-done", Done { run_id, ran });
        })
    })?;
    let folders = folders_rx
        .recv()
        .await
        .ok_or_else(|| "the import stopped before it began".to_string())?;
    Ok(Started { run_id, folders })
}

/// Ask the run `run_id` to stop at once, dropping the file in flight; a finished or unknown
/// run is ignored.
#[tauri::command]
pub async fn import_cancel(app: tauri::AppHandle, run_id: u64) {
    if let Some((id, cancel, _)) = &index::lock(&app.state::<Imports>().0).running {
        if *id == run_id {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_import_is_refused_while_one_runs() {
        let imports = Imports::default();
        let first = begin(&imports, |_, _| tauri::async_runtime::spawn_blocking(|| {})).unwrap();
        assert_eq!(
            begin(&imports, |_, _| unreachable!()),
            Err(RUN_IN_PROGRESS.to_string())
        );
        index::lock(&imports.0).finish(first);
        let second = begin(&imports, |_, _| tauri::async_runtime::spawn_blocking(|| {}));
        assert_eq!(second, Ok(first + 1));
    }
}
