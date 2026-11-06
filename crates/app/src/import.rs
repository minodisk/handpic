//! Import from a memory card: the destination root and the folder pattern
//! the photos are copied into, persisted under `importRoot` and
//! `importPattern`, the pattern's validation, and the plan of an import:
//! every RAW and JPEG under the right-clicked folder, a RAW's sidecars with
//! it, grouped into destination folders by capture date, each file copied,
//! skipped as already imported or refused as a collision, and the one run
//! that copies a previewed plan through `riffle_core::copy`, without touching
//! the index.
//!
//! The pattern is chrono strftime syntax over the capture date, `/`
//! separating folders. Only date specifiers are allowed, and every folder
//! name it makes has to be valid on Windows and macOS.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::format::{Item, StrftimeItems};
use chrono::NaiveDate;
use riffle_core::copy;
use riffle_core::scan::{is_jpeg_file, is_raw_file};
use serde::Serialize;
use serde_json::Value;
use tauri::{Emitter, Manager};

use crate::commands::canonicalize;
use crate::sidecar::SidecarFormat;
use crate::{datefolder, folders, index};

/// The pattern used when none is stored: `2026/2026-10-24`.
pub const DEFAULT_PATTERN: &str = "%Y/%F";

/// The stored import settings. An empty `root` is not set yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportSettings {
    pub root: String,
    pub pattern: String,
}

/// The import settings in use, loaded at launch and replaced by the setters.
pub struct AppImportSettings(pub Mutex<ImportSettings>);

/// `Ok` when `pattern` formats every date into a valid relative folder path.
pub fn validate_pattern(pattern: &str) -> Result<(), String> {
    let date = NaiveDate::from_ymd_opt(2026, 1, 2).expect("a valid date");
    format_date(pattern, date).map(|_| ())
}

/// `date` formatted by `pattern`, the relative folder path with `/` as the
/// separator, or why the pattern is refused.
pub fn format_date(pattern: &str, date: NaiveDate) -> Result<String, String> {
    if StrftimeItems::new(pattern).any(|item| matches!(item, Item::Error)) {
        return Err(format!(
            "`{pattern}` has an unknown specifier or a lone `%`"
        ));
    }
    // `to_string` panics on the formatting error a time specifier raises
    // without a time; `write!` returns it.
    let mut path = String::new();
    if write!(path, "{}", date.format(pattern)).is_err() {
        return Err(format!(
            "`{pattern}` uses a time, zone or timestamp specifier; only the date is known"
        ));
    }
    for segment in path.split('/') {
        check_segment(segment)?;
    }
    Ok(path)
}

/// Why `segment` cannot be a folder name on Windows or macOS, if it cannot.
fn check_segment(segment: &str) -> Result<(), String> {
    if segment.is_empty() {
        return Err("the pattern makes an empty folder name".to_string());
    }
    if segment == "." || segment == ".." {
        return Err(format!("`{segment}` cannot be a folder name"));
    }
    if let Some(c) = segment
        .chars()
        .find(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\') || c.is_control())
    {
        return Err(format!(
            "the folder name `{segment}` has {c:?}, which a folder name cannot have"
        ));
    }
    if segment.ends_with(' ') || segment.ends_with('.') {
        return Err(format!(
            "the folder name `{segment}` ends in a space or a dot"
        ));
    }
    Ok(())
}

/// The stored `importRoot` value; missing or non-string means not set.
fn root_setting(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The stored `importPattern` value; missing, non-string or invalid means
/// `DEFAULT_PATTERN`.
fn pattern_setting(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .filter(|pattern| validate_pattern(pattern).is_ok())
        .unwrap_or(DEFAULT_PATTERN)
        .to_string()
}

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

/// `pattern` formatted for `date` and placed under `root` with the OS
/// separator; the relative path alone while `root` is not set.
fn example(root: &str, pattern: &str, date: NaiveDate) -> Result<String, String> {
    let relative = format_date(pattern, date)?;
    let mut path = PathBuf::from(root);
    path.extend(relative.split('/'));
    Ok(path.to_string_lossy().into_owned())
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

/// A file found under the imported folder and its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub path: PathBuf,
    pub size: u64,
}

/// A RAW or JPEG found under the imported folder, with a RAW's existing
/// `.xmp` / `.dop` sidecars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Photo {
    pub file: Found,
    pub sidecars: Vec<Found>,
}

/// Every RAW and JPEG under `dir`, recursively, sorted by path, each RAW
/// with the sidecars next to it. Symlinks are followed as `folders::kind`
/// follows them (a folder reached twice is walked once), and hidden folders
/// are skipped as the folder tree skips them.
pub fn walk(dir: &Path) -> Result<Vec<Photo>, String> {
    let mut photos = Vec::new();
    let mut visited = HashSet::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        if !visited.insert(fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone())) {
            continue;
        }
        let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut files = Vec::new();
        for entry in entries.flatten() {
            let Some((is_dir, is_file)) = folders::kind(&entry) else {
                continue;
            };
            let path = entry.path();
            if is_dir {
                if !entry.file_name().to_string_lossy().starts_with('.')
                    && !folders::is_hidden(&entry)
                {
                    dirs.push(path);
                }
            } else if is_file {
                let size = entry
                    .metadata()
                    .ok()
                    .filter(fs::Metadata::is_file)
                    .or_else(|| fs::metadata(&path).ok())
                    .map_or(0, |m| m.len());
                files.push(Found { path, size });
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let (mut sidecars, files): (Vec<Found>, Vec<Found>) = files
            .into_iter()
            .partition(|f| SidecarFormat::Both.matches(&name_of(&f.path)));
        for file in files {
            let raw = is_raw_file(&file.path);
            if !raw && !is_jpeg_file(&file.path) {
                continue;
            }
            let mut found = Vec::new();
            if raw {
                for format in SidecarFormat::Both.kinds() {
                    let wanted = name_of(&format.sidecar_path(&file.path));
                    if let Some(i) = sidecars
                        .iter()
                        .position(|s| name_of(&s.path).eq_ignore_ascii_case(&wanted))
                    {
                        found.push(sidecars.remove(i));
                    }
                }
            }
            photos.push(Photo {
                file,
                sidecars: found,
            });
        }
    }
    photos.sort_by(|a, b| a.file.path.cmp(&b.file.path));
    Ok(photos)
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// What the run does with one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Missing at the destination: copied.
    Copy,
    /// The same name and size are already there: imported before.
    Skip,
    /// The same name with another size is there, or another file of this
    /// import takes the name: never overwritten.
    Collision,
}

/// One file and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub size: u64,
    pub decision: Decision,
}

/// One destination folder and its files: the dated photos by date then by
/// file name, each RAW followed by its sidecars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub path: PathBuf,
    pub items: Vec<Planned>,
}

/// The import plan: the destination folders in the order of their earliest
/// date, and the photos without a capture date, which are not copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub destinations: Vec<Destination>,
    pub undated: Vec<PathBuf>,
}

/// Plan the import of `photos`, each with its Exif capture time, into
/// `root` under `pattern`. `probe` gives the size of what exists at a
/// destination path. A sidecar follows its RAW: it collides with a
/// colliding RAW, and next to a skipped RAW it is copied only when missing.
pub fn plan(
    photos: &[(Photo, Option<String>)],
    root: &Path,
    pattern: &str,
    probe: impl Fn(&Path) -> Option<u64>,
) -> Result<Plan, String> {
    let mut by_date: BTreeMap<NaiveDate, Vec<&Photo>> = BTreeMap::new();
    let mut undated = Vec::new();
    for (photo, time) in photos {
        let date = time
            .as_deref()
            .and_then(datefolder::date_of)
            .and_then(|date| NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok());
        match date {
            Some(date) => by_date.entry(date).or_default().push(photo),
            None => undated.push(photo.file.path.clone()),
        }
    }
    let mut destinations: Vec<Destination> = Vec::new();
    let mut planned: Vec<HashSet<String>> = Vec::new();
    for (date, mut photos) in by_date {
        let mut path = root.to_path_buf();
        path.extend(format_date(pattern, date)?.split('/'));
        let i = match destinations.iter().position(|d| d.path == path) {
            Some(i) => i,
            None => {
                destinations.push(Destination {
                    path: path.clone(),
                    items: Vec::new(),
                });
                planned.push(HashSet::new());
                destinations.len() - 1
            }
        };
        photos.sort_by(|a, b| {
            (a.file.path.file_name(), &a.file.path).cmp(&(b.file.path.file_name(), &b.file.path))
        });
        let (items, names) = (&mut destinations[i].items, &mut planned[i]);
        let mut decide = |found: &Found| {
            let name = name_of(&found.path);
            let dst = path.join(&name);
            let decision = if !names.insert(name.to_lowercase()) {
                Decision::Collision
            } else {
                match probe(&dst) {
                    None => Decision::Copy,
                    Some(size) if size == found.size => Decision::Skip,
                    Some(_) => Decision::Collision,
                }
            };
            (dst, decision)
        };
        for photo in photos {
            let (dst, decision) = decide(&photo.file);
            items.push(Planned {
                src: photo.file.path.clone(),
                dst,
                size: photo.file.size,
                decision,
            });
            for sidecar in &photo.sidecars {
                let (dst, own) = decide(sidecar);
                let decision = match (decision, own) {
                    (Decision::Collision, _) => Decision::Collision,
                    (Decision::Skip, Decision::Collision) if probe(&dst).is_some() => {
                        Decision::Skip
                    }
                    (_, own) => own,
                };
                items.push(Planned {
                    src: sidecar.path.clone(),
                    dst,
                    size: sidecar.size,
                    decision,
                });
            }
        }
    }
    undated.sort_by(|a, b| (a.file_name(), a).cmp(&(b.file_name(), b)));
    Ok(Plan {
        destinations,
        undated,
    })
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

/// One destination folder in the dialog.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSummary {
    path: String,
    copy_count: usize,
    copy_bytes: u64,
    skipped: usize,
    collisions: Vec<String>,
}

/// The counts over every destination folder.
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    copy_count: usize,
    copy_bytes: u64,
    skipped: usize,
    collisions: usize,
    undated: usize,
}

/// What `import_preview` gives the dialog: the preview id the run takes
/// (`None` when refused), the source folder, the destination folders in
/// date order, the undated file names, the totals, and why the import is
/// refused, if it is.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    id: Option<u64>,
    source: String,
    folders: Vec<FolderSummary>,
    undated: Vec<String>,
    totals: Totals,
    refusal: Option<String>,
}

/// Why `settings` cannot take an import, if they cannot.
fn refusal(settings: &ImportSettings) -> Option<String> {
    if settings.root.trim().is_empty() {
        return Some("the destination folder is not set (Settings > Import)".to_string());
    }
    validate_pattern(&settings.pattern).err()
}

fn summarize(id: Option<u64>, source: String, plan: &Plan) -> Preview {
    let mut totals = Totals {
        undated: plan.undated.len(),
        ..Totals::default()
    };
    let folders = plan
        .destinations
        .iter()
        .map(|destination| {
            let mut folder = FolderSummary {
                path: destination.path.to_string_lossy().into_owned(),
                copy_count: 0,
                copy_bytes: 0,
                skipped: 0,
                collisions: Vec::new(),
            };
            for item in &destination.items {
                match item.decision {
                    Decision::Copy => {
                        folder.copy_count += 1;
                        folder.copy_bytes += item.size;
                    }
                    Decision::Skip => folder.skipped += 1,
                    Decision::Collision => folder.collisions.push(name_of(&item.src)),
                }
            }
            totals.copy_count += folder.copy_count;
            totals.copy_bytes += folder.copy_bytes;
            totals.skipped += folder.skipped;
            totals.collisions += folder.collisions.len();
            folder
        })
        .collect();
    Preview {
        id,
        source,
        folders,
        undated: plan.undated.iter().map(|path| name_of(path)).collect(),
        totals,
        refusal: None,
    }
}

/// The plan of importing every photo under `source` into `settings`' root
/// and pattern, each file's capture time read from its head.
fn plan_dir(source: &Path, settings: &ImportSettings) -> Result<Plan, String> {
    let dated: Vec<(Photo, Option<String>)> = walk(source)?
        .into_iter()
        .map(|photo| {
            let time = riffle_core::reader::read_metadata(&photo.file.path)
                .ok()
                .and_then(|arw| arw.shot.capture_time);
            (photo, time)
        })
        .collect();
    plan(
        &dated,
        Path::new(&settings.root),
        &settings.pattern,
        |path| fs::metadata(path).ok().map(|m| m.len()),
    )
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

/// One file that could not be copied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Failure {
    path: String,
    message: String,
}

/// What a run did: the files copied, the files skipped (by the plan, or
/// found at the destination with their size when their turn came), the
/// failures, whether it was canceled, the bytes copied, the time it took, and
/// the destination folders that got files to copy, in date order.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ran {
    copied: usize,
    skipped: usize,
    failed: Vec<Failure>,
    canceled: bool,
    bytes: u64,
    elapsed_ms: u64,
    folders: Vec<String>,
}

/// A plan made ready to copy: its destination folders created, the jobs to
/// run with their planned sizes, and what is already known before copying.
struct Prepared {
    jobs: Vec<copy::Job>,
    sizes: Vec<u64>,
    skipped: usize,
    failed: Vec<Failure>,
    folders: Vec<String>,
}

/// Create the destination folders of `plan` that get a file, before any copy
/// starts, so the first one can be opened while it is still empty, and line
/// up its `Copy` items in plan order (date, then name, each RAW followed by
/// its sidecars). A folder that cannot be created fails its files.
fn prepare(plan: &Plan) -> Prepared {
    let mut prepared = Prepared {
        jobs: Vec::new(),
        sizes: Vec::new(),
        skipped: 0,
        failed: Vec::new(),
        folders: Vec::new(),
    };
    for destination in &plan.destinations {
        prepared.skipped += destination
            .items
            .iter()
            .filter(|item| item.decision == Decision::Skip)
            .count();
        let copies: Vec<&Planned> = destination
            .items
            .iter()
            .filter(|item| item.decision == Decision::Copy)
            .collect();
        if copies.is_empty() {
            continue;
        }
        if let Err(e) = fs::create_dir_all(&destination.path) {
            let message = format!("create {}: {e}", destination.path.display());
            prepared.failed.extend(copies.iter().map(|item| Failure {
                path: item.src.to_string_lossy().into_owned(),
                message: message.clone(),
            }));
            continue;
        }
        prepared
            .folders
            .push(destination.path.to_string_lossy().into_owned());
        for item in copies {
            prepared.jobs.push(copy::Job {
                src: item.src.clone(),
                dst: item.dst.clone(),
            });
            prepared.sizes.push(item.size);
        }
    }
    prepared
}

/// Copy the jobs of `prepared`. The copy re-checks each destination just
/// before writing; a file that appeared there since the preview counts as
/// skipped when it has the source's size and as a failure otherwise.
/// `progress` gets each copy progress with the destination of the file in
/// flight.
fn run_plan(
    prepared: Prepared,
    cancel: &AtomicBool,
    mut progress: impl FnMut(&copy::Progress, &Path),
) -> Ran {
    let Prepared {
        jobs,
        sizes,
        mut skipped,
        mut failed,
        folders,
    } = prepared;
    let summary = copy::copy_files(&jobs, cancel, |p| progress(&p, &jobs[p.job].dst));
    let mut copied = 0;
    for ((job, size), outcome) in jobs.iter().zip(&sizes).zip(&summary.results) {
        match outcome {
            copy::Outcome::Copied(_) => copied += 1,
            copy::Outcome::Failed(_) if fs::metadata(&job.dst).is_ok_and(|m| m.len() == *size) => {
                skipped += 1
            }
            copy::Outcome::Failed(message) => failed.push(Failure {
                path: job.src.to_string_lossy().into_owned(),
                message: message.clone(),
            }),
            copy::Outcome::NotRun => {}
        }
    }
    Ran {
        copied,
        skipped,
        failed,
        canceled: summary.canceled,
        bytes: summary.bytes,
        elapsed_ms: u64::try_from(summary.elapsed.as_millis()).unwrap_or(u64::MAX),
        folders,
    }
}

/// The `import:` line a run leaves in `Riffle.log`, so its throughput can be
/// compared with Explorer's.
fn log_line(source: &str, root: &str, ran: &Ran) -> String {
    let seconds = ran.elapsed_ms as f64 / 1000.0;
    let mb_per_s = if seconds > 0.0 {
        ran.bytes as f64 / 1_000_000.0 / seconds
    } else {
        0.0
    };
    format!(
        "import: files={} bytes={} mb_per_s={mb_per_s:.1} skipped={} failed={} canceled={} source={source} root={root} in {}ms",
        ran.copied,
        ran.bytes,
        ran.skipped,
        ran.failed.len(),
        ran.canceled,
        ran.elapsed_ms
    )
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
    use serde_json::json;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
    }

    #[test]
    fn formats_every_accepted_specifier() {
        for (pattern, expected) in [
            ("%Y", "2026"),
            ("%y", "26"),
            ("%m", "10"),
            ("%d", "24"),
            ("%b", "Oct"),
            ("%B", "October"),
            ("%a", "Sat"),
            ("%A", "Saturday"),
            ("%j", "297"),
            ("%F", "2026-10-24"),
            ("%Y/%F", "2026/2026-10-24"),
            ("%Y/%m/%d", "2026/10/24"),
            ("Photos %Y/%d %B trip", "Photos 2026/24 October trip"),
        ] {
            assert_eq!(
                format_date(pattern, date()).as_deref(),
                Ok(expected),
                "{pattern}"
            );
            assert_eq!(validate_pattern(pattern), Ok(()), "{pattern}");
        }
    }

    #[test]
    fn refuses_an_unknown_specifier_or_a_lone_percent() {
        for pattern in ["%Q", "%Y/%", "%Y/%L"] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(error.contains("unknown specifier"), "{pattern}: {error}");
        }
    }

    #[test]
    fn refuses_time_zone_and_timestamp_specifiers() {
        for pattern in [
            "%H", "%M", "%S", "%p", "%z", "%Z", "%s", "%T", "%R", "%c", "%+", "%I", "%f",
            "%Y/%F %H",
        ] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(
                error.contains("time, zone or timestamp"),
                "{pattern}: {error}"
            );
        }
    }

    #[test]
    fn refuses_an_empty_folder_name() {
        for pattern in ["", "/%Y", "%Y/", "%Y//%F"] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(error.contains("empty folder name"), "{pattern:?}: {error}");
        }
    }

    #[test]
    fn refuses_a_dot_or_dot_dot_folder() {
        for pattern in [".", "../%Y", "%Y/./%F"] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(
                error.contains("cannot be a folder name"),
                "{pattern}: {error}"
            );
        }
    }

    #[test]
    fn refuses_a_character_windows_or_macos_refuses() {
        for pattern in [
            "%Y<", "%Y>", "%Y:%m", "%Y\"", "%Y|", "%Y?", "%Y*", "%Y\\%F", "%Y%n", "%Y%t", "a\u{1}b",
        ] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(
                error.contains("which a folder name cannot have"),
                "{pattern:?}: {error}"
            );
        }
    }

    #[test]
    fn refuses_a_folder_ending_in_a_space_or_a_dot() {
        for pattern in ["%Y /%F", "%Y./%F", "%F."] {
            let error = format_date(pattern, date()).unwrap_err();
            assert!(
                error.contains("ends in a space or a dot"),
                "{pattern}: {error}"
            );
        }
    }

    #[test]
    fn root_setting_is_empty_unless_a_string() {
        assert_eq!(root_setting(None), "");
        assert_eq!(root_setting(Some(&json!(3))), "");
        assert_eq!(root_setting(Some(&json!("D:\\Photos"))), "D:\\Photos");
    }

    #[test]
    fn pattern_setting_falls_back_to_the_default() {
        assert_eq!(pattern_setting(None), DEFAULT_PATTERN);
        assert_eq!(pattern_setting(Some(&json!(true))), DEFAULT_PATTERN);
        assert_eq!(pattern_setting(Some(&json!("%H"))), DEFAULT_PATTERN);
        assert_eq!(pattern_setting(Some(&json!("%Y/%m"))), "%Y/%m");
    }

    #[test]
    fn example_joins_the_root_with_the_os_separator() {
        let root = std::env::temp_dir().join("Photos");
        let expected = root.join("2026").join("2026-10-24");
        assert_eq!(
            example(&root.to_string_lossy(), DEFAULT_PATTERN, date()),
            Ok(expected.to_string_lossy().into_owned())
        );
        let relative = PathBuf::from("2026").join("10");
        assert_eq!(
            example("", "%Y/%m", date()),
            Ok(relative.to_string_lossy().into_owned())
        );
        assert!(example("", "%H", date()).is_err());
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-import-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![0u8; bytes]).unwrap();
    }

    fn found(path: &Path, size: u64) -> Found {
        Found {
            path: path.to_path_buf(),
            size,
        }
    }

    #[test]
    fn walk_finds_raws_and_jpegs_recursively_with_their_sidecars() {
        let card = temp_dir("walk");
        let first = card.join("DCIM").join("100MSDCF");
        let second = card.join("DCIM").join("101MSDCF");
        write(&first.join("DSC00001.ARW"), 10);
        write(&first.join("DSC00001.XMP"), 2);
        write(&first.join("DSC00001.ARW.dop"), 3);
        write(&first.join("DSC00002.JPG"), 4);
        write(&first.join("DSC00002.xmp"), 1);
        write(&first.join("C0001.MP4"), 5);
        write(&first.join("notes.txt"), 1);
        write(&second.join("IMG_0003.cr3"), 6);
        write(&second.join("DSC00004.nef"), 7);
        write(&card.join(".Trashes").join("DSC00009.ARW"), 8);

        let photos = walk(&card).unwrap();

        assert_eq!(
            photos,
            vec![
                Photo {
                    file: found(&first.join("DSC00001.ARW"), 10),
                    sidecars: vec![
                        found(&first.join("DSC00001.XMP"), 2),
                        found(&first.join("DSC00001.ARW.dop"), 3),
                    ],
                },
                Photo {
                    file: found(&first.join("DSC00002.JPG"), 4),
                    sidecars: Vec::new(),
                },
                Photo {
                    file: found(&second.join("DSC00004.nef"), 7),
                    sidecars: Vec::new(),
                },
                Photo {
                    file: found(&second.join("IMG_0003.cr3"), 6),
                    sidecars: Vec::new(),
                },
            ]
        );
        fs::remove_dir_all(&card).unwrap();
    }

    fn photo(path: &str, size: u64, sidecars: &[(&str, u64)]) -> Photo {
        Photo {
            file: found(Path::new(path), size),
            sidecars: sidecars
                .iter()
                .map(|(path, size)| found(Path::new(path), *size))
                .collect(),
        }
    }

    fn dated(photo: Photo, time: Option<&str>) -> (Photo, Option<String>) {
        (photo, time.map(str::to_string))
    }

    fn item(src: &str, dst: &Path, size: u64, decision: Decision) -> Planned {
        let src = PathBuf::from(src);
        Planned {
            dst: dst.join(src.file_name().unwrap()),
            src,
            size,
            decision,
        }
    }

    fn nothing(_: &Path) -> Option<u64> {
        None
    }

    fn existing(files: &[(PathBuf, u64)]) -> impl Fn(&Path) -> Option<u64> + '_ {
        |path| files.iter().find(|(p, _)| p == path).map(|(_, size)| *size)
    }

    #[test]
    fn plan_groups_by_date_in_date_order_under_the_pattern() {
        let root = PathBuf::from("Photos");
        let photos = [
            dated(
                photo("card/b/DSC00003.ARW", 30, &[]),
                Some("2026:10:25 08:00:00"),
            ),
            dated(
                photo("card/a/DSC00002.ARW", 20, &[]),
                Some("2026:10:24 23:59:59"),
            ),
            dated(
                photo("card/a/DSC00001.ARW", 10, &[]),
                Some("2026:10:24 07:00:00"),
            ),
            dated(
                photo("card/a/DSC00001.JPG", 5, &[]),
                Some("2026:10:24 07:00:00"),
            ),
        ];

        let plan = plan(&photos, &root, DEFAULT_PATTERN, nothing).unwrap();

        let first = root.join("2026").join("2026-10-24");
        let second = root.join("2026").join("2026-10-25");
        assert_eq!(
            plan,
            Plan {
                destinations: vec![
                    Destination {
                        path: first.clone(),
                        items: vec![
                            item("card/a/DSC00001.ARW", &first, 10, Decision::Copy),
                            item("card/a/DSC00001.JPG", &first, 5, Decision::Copy),
                            item("card/a/DSC00002.ARW", &first, 20, Decision::Copy),
                        ],
                    },
                    Destination {
                        path: second.clone(),
                        items: vec![item("card/b/DSC00003.ARW", &second, 30, Decision::Copy)],
                    },
                ],
                undated: Vec::new(),
            }
        );
    }

    #[test]
    fn plan_merges_dates_the_pattern_puts_in_one_folder() {
        let root = PathBuf::from("Photos");
        let photos = [
            dated(
                photo("card/DSC00002.ARW", 20, &[]),
                Some("2026:11:01 10:00:00"),
            ),
            dated(
                photo("card/DSC00001.ARW", 10, &[]),
                Some("2026:10:31 10:00:00"),
            ),
            dated(
                photo("card/DSC00003.ARW", 30, &[]),
                Some("2026:10:01 10:00:00"),
            ),
        ];

        let plan = plan(&photos, &root, "%Y/%m", nothing).unwrap();

        let october = root.join("2026").join("10");
        let november = root.join("2026").join("11");
        let paths: Vec<_> = plan.destinations.iter().map(|d| d.path.clone()).collect();
        assert_eq!(paths, vec![october.clone(), november]);
        assert_eq!(
            plan.destinations[0].items,
            vec![
                item("card/DSC00003.ARW", &october, 30, Decision::Copy),
                item("card/DSC00001.ARW", &october, 10, Decision::Copy),
            ]
        );
    }

    #[test]
    fn plan_skips_the_same_size_and_collides_on_another() {
        let root = PathBuf::from("Photos");
        let dst = root.join("2026").join("2026-10-24");
        let files = [
            (dst.join("DSC00001.ARW"), 10),
            (dst.join("DSC00002.ARW"), 99),
        ];
        let time = Some("2026:10:24 10:00:00");
        let photos = [
            dated(photo("card/DSC00001.ARW", 10, &[]), time),
            dated(photo("card/DSC00002.ARW", 20, &[]), time),
            dated(photo("card/DSC00003.ARW", 30, &[]), time),
        ];

        let plan = plan(&photos, &root, DEFAULT_PATTERN, existing(&files)).unwrap();

        assert_eq!(
            plan.destinations[0].items,
            vec![
                item("card/DSC00001.ARW", &dst, 10, Decision::Skip),
                item("card/DSC00002.ARW", &dst, 20, Decision::Collision),
                item("card/DSC00003.ARW", &dst, 30, Decision::Copy),
            ]
        );
    }

    #[test]
    fn plan_collides_two_sources_with_one_name_on_one_date() {
        let root = PathBuf::from("Photos");
        let dst = root.join("2026").join("2026-10-24");
        let time = Some("2026:10:24 10:00:00");
        let photos = [
            dated(photo("card/100MSDCF/DSC00001.ARW", 10, &[]), time),
            dated(photo("card/101MSDCF/DSC00001.ARW", 10, &[]), time),
        ];

        let plan = plan(&photos, &root, DEFAULT_PATTERN, nothing).unwrap();

        assert_eq!(
            plan.destinations[0].items,
            vec![
                item("card/100MSDCF/DSC00001.ARW", &dst, 10, Decision::Copy),
                item("card/101MSDCF/DSC00001.ARW", &dst, 10, Decision::Collision),
            ]
        );
    }

    #[test]
    fn plan_lists_the_undated_sorted_by_name() {
        let root = PathBuf::from("Photos");
        let photos = [
            dated(
                photo("card/b/DSC00002.ARW", 20, &[("card/b/DSC00002.xmp", 1)]),
                None,
            ),
            dated(
                photo("card/a/DSC00003.ARW", 30, &[]),
                Some("0000:00:00 00:00:00"),
            ),
            dated(
                photo("card/a/DSC00001.ARW", 10, &[]),
                Some("2026:02:31 10:00:00"),
            ),
        ];

        let plan = plan(&photos, &root, DEFAULT_PATTERN, nothing).unwrap();

        assert_eq!(
            plan,
            Plan {
                destinations: Vec::new(),
                undated: vec![
                    PathBuf::from("card/a/DSC00001.ARW"),
                    PathBuf::from("card/b/DSC00002.ARW"),
                    PathBuf::from("card/a/DSC00003.ARW"),
                ],
            }
        );
    }

    #[test]
    fn sidecars_follow_their_raw() {
        let root = PathBuf::from("Photos");
        let dst = root.join("2026").join("2026-10-24");
        let files = [
            (dst.join("A.ARW"), 10),
            (dst.join("A.xmp"), 5),
            (dst.join("B.ARW"), 10),
            (dst.join("C.ARW"), 99),
            (dst.join("D.xmp"), 9),
        ];
        let time = Some("2026:10:24 10:00:00");
        let photos = [
            dated(photo("card/A.ARW", 10, &[("card/A.xmp", 1)]), time),
            dated(photo("card/B.ARW", 10, &[("card/B.ARW.dop", 1)]), time),
            dated(photo("card/C.ARW", 10, &[("card/C.xmp", 1)]), time),
            dated(photo("card/D.ARW", 10, &[("card/D.xmp", 1)]), time),
        ];

        let plan = plan(&photos, &root, DEFAULT_PATTERN, existing(&files)).unwrap();

        assert_eq!(
            plan.destinations[0].items,
            vec![
                item("card/A.ARW", &dst, 10, Decision::Skip),
                item("card/A.xmp", &dst, 1, Decision::Skip),
                item("card/B.ARW", &dst, 10, Decision::Skip),
                item("card/B.ARW.dop", &dst, 1, Decision::Copy),
                item("card/C.ARW", &dst, 10, Decision::Collision),
                item("card/C.xmp", &dst, 1, Decision::Collision),
                item("card/D.ARW", &dst, 10, Decision::Copy),
                item("card/D.xmp", &dst, 1, Decision::Collision),
            ]
        );
    }

    #[test]
    fn plan_refuses_an_invalid_pattern() {
        let photos = [dated(
            photo("card/A.ARW", 10, &[]),
            Some("2026:10:24 10:00:00"),
        )];
        assert!(plan(&photos, Path::new("Photos"), "%H", nothing).is_err());
    }

    #[test]
    fn summarize_counts_each_folder_and_the_totals() {
        let root = PathBuf::from("Photos");
        let dst = root.join("2026").join("2026-10-24");
        let files = [(dst.join("A.ARW"), 10), (dst.join("D.ARW"), 1)];
        let time = Some("2026:10:24 10:00:00");
        let photos = [
            dated(photo("card/A.ARW", 10, &[]), time),
            dated(photo("card/B.ARW", 20, &[("card/B.xmp", 2)]), time),
            dated(photo("card/C.JPG", 3, &[]), None),
            dated(photo("card/D.ARW", 4, &[]), time),
        ];
        let plan = plan(&photos, &root, DEFAULT_PATTERN, existing(&files)).unwrap();

        let preview = summarize(Some(7), "card".to_string(), &plan);

        assert_eq!(
            preview,
            Preview {
                id: Some(7),
                source: "card".to_string(),
                folders: vec![FolderSummary {
                    path: dst.to_string_lossy().into_owned(),
                    copy_count: 2,
                    copy_bytes: 22,
                    skipped: 1,
                    collisions: vec!["D.ARW".to_string()],
                }],
                undated: vec!["C.JPG".to_string()],
                totals: Totals {
                    copy_count: 2,
                    copy_bytes: 22,
                    skipped: 1,
                    collisions: 1,
                    undated: 1,
                },
                refusal: None,
            }
        );
    }

    #[test]
    fn refuses_an_unset_root_or_an_invalid_pattern() {
        let settings = |root: &str, pattern: &str| ImportSettings {
            root: root.to_string(),
            pattern: pattern.to_string(),
        };
        assert!(refusal(&settings("", DEFAULT_PATTERN)).is_some());
        assert!(refusal(&settings("D:\\Photos", "%H")).is_some());
        assert_eq!(refusal(&settings("D:\\Photos", DEFAULT_PATTERN)), None);
    }

    /// A JPEG holding only an Exif APP1 with `DateTimeOriginal`, which is all
    /// the preview reads, padded to `size` bytes past its end marker.
    fn jpeg(datetime: &str, size: usize) -> Vec<u8> {
        let mut tiff = b"II".to_vec();
        tiff.extend_from_slice(&42u16.to_le_bytes());
        tiff.extend_from_slice(&8u32.to_le_bytes());
        // IFD0 at 8: the Exif IFD pointer, pointing at 26.
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x8769u16.to_le_bytes());
        tiff.extend_from_slice(&4u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&26u32.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        // Exif IFD at 26: DateTimeOriginal, its value at 44.
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x9003u16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&20u32.to_le_bytes());
        tiff.extend_from_slice(&44u32.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff.extend_from_slice(datetime.as_bytes());
        tiff.push(0);
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(&tiff);
        let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(&payload);
        out.extend_from_slice(&[0xFF, 0xD9]);
        out.resize(out.len().max(size), 0);
        out
    }

    fn copies(plan: &Plan) -> usize {
        plan.destinations
            .iter()
            .flat_map(|d| &d.items)
            .filter(|item| item.decision == Decision::Copy)
            .count()
    }

    fn import_case(name: &str) -> (PathBuf, PathBuf, ImportSettings) {
        let dir = temp_dir(name);
        let card = dir.join("card").join("DCIM").join("100MSDCF");
        fs::create_dir_all(&card).unwrap();
        fs::write(card.join("DSC00001.JPG"), jpeg("2026:10:24 07:00:00", 3000)).unwrap();
        fs::write(card.join("DSC00002.JPG"), jpeg("2026:10:25 08:00:00", 5000)).unwrap();
        fs::write(card.join("DSC00003.JPG"), b"no exif").unwrap();
        let root = dir.join("Photos");
        let settings = ImportSettings {
            root: root.to_string_lossy().into_owned(),
            pattern: DEFAULT_PATTERN.to_string(),
        };
        (dir.join("card"), root, settings)
    }

    #[test]
    fn a_run_copies_the_plan_and_a_second_import_copies_nothing() {
        let (card, root, settings) = import_case("run");

        let first = plan_dir(&card, &settings).unwrap();
        assert_eq!(copies(&first), 2);
        let mut last = None;
        let ran = run_plan(prepare(&first), &AtomicBool::new(false), |p, _| {
            last = Some(*p)
        });

        let day1 = root.join("2026").join("2026-10-24");
        let day2 = root.join("2026").join("2026-10-25");
        assert_eq!(
            ran,
            Ran {
                copied: 2,
                skipped: 0,
                failed: Vec::new(),
                canceled: false,
                bytes: 8000,
                elapsed_ms: ran.elapsed_ms,
                folders: vec![
                    day1.to_string_lossy().into_owned(),
                    day2.to_string_lossy().into_owned(),
                ],
            }
        );
        let last = last.unwrap();
        assert_eq!((last.files_done, last.bytes_done), (2, 8000));
        let src = card.join("DCIM").join("100MSDCF").join("DSC00001.JPG");
        let dst = day1.join("DSC00001.JPG");
        assert_eq!(fs::read(&dst).unwrap(), fs::read(&src).unwrap());
        assert_eq!(
            fs::metadata(&dst).unwrap().modified().unwrap(),
            fs::metadata(&src).unwrap().modified().unwrap()
        );
        assert!(day2.join("DSC00002.JPG").is_file());

        let second = plan_dir(&card, &settings).unwrap();
        assert_eq!(copies(&second), 0);
        assert_eq!(
            summarize(Some(1), String::new(), &second).totals.copy_count,
            0
        );
        let ran = run_plan(prepare(&second), &AtomicBool::new(false), |_, _| {});
        assert_eq!((ran.copied, ran.skipped, ran.bytes), (0, 2, 0));
        assert!(ran.failed.is_empty());
        assert!(ran.folders.is_empty());
        fs::remove_dir_all(card.parent().unwrap()).unwrap();
    }

    #[test]
    fn prepare_creates_every_destination_folder_before_any_copy() {
        let (card, root, settings) = import_case("prepare");
        let plan = plan_dir(&card, &settings).unwrap();

        let prepared = prepare(&plan);

        let day1 = root.join("2026").join("2026-10-24");
        let day2 = root.join("2026").join("2026-10-25");
        assert_eq!(
            prepared.folders,
            vec![
                day1.to_string_lossy().into_owned(),
                day2.to_string_lossy().into_owned(),
            ]
        );
        assert_eq!(fs::read_dir(&day1).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&day2).unwrap().count(), 0);
        assert_eq!(prepared.jobs.len(), 2);
        fs::remove_dir_all(card.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_file_that_appears_after_the_preview_is_skipped_or_failed_never_overwritten() {
        let (card, root, settings) = import_case("appeared");
        let plan = plan_dir(&card, &settings).unwrap();
        let day1 = root.join("2026").join("2026-10-24");
        let day2 = root.join("2026").join("2026-10-25");
        fs::create_dir_all(&day1).unwrap();
        fs::create_dir_all(&day2).unwrap();
        fs::write(day1.join("DSC00001.JPG"), vec![1u8; 3000]).unwrap();
        fs::write(day2.join("DSC00002.JPG"), b"other").unwrap();

        let ran = run_plan(prepare(&plan), &AtomicBool::new(false), |_, _| {});

        assert_eq!((ran.copied, ran.skipped), (0, 1));
        assert_eq!(ran.failed.len(), 1);
        assert!(ran.failed[0].path.ends_with("DSC00002.JPG"));
        assert_eq!(fs::read(day2.join("DSC00002.JPG")).unwrap(), b"other");
        fs::remove_dir_all(card.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_canceled_run_copies_nothing_more() {
        let (card, _, settings) = import_case("cancel");
        let plan = plan_dir(&card, &settings).unwrap();

        let ran = run_plan(prepare(&plan), &AtomicBool::new(true), |_, _| {});

        assert!(ran.canceled);
        assert_eq!((ran.copied, ran.bytes), (0, 0));
        assert!(ran.failed.is_empty());
        fs::remove_dir_all(card.parent().unwrap()).unwrap();
    }

    #[test]
    fn the_log_line_has_the_throughput_source_and_root() {
        let ran = Ran {
            copied: 3,
            skipped: 1,
            failed: Vec::new(),
            canceled: false,
            bytes: 50_000_000,
            elapsed_ms: 2000,
            folders: Vec::new(),
        };
        assert_eq!(
            log_line("E:\\DCIM", "D:\\Photos", &ran),
            "import: files=3 bytes=50000000 mb_per_s=25.0 skipped=1 failed=0 canceled=false source=E:\\DCIM root=D:\\Photos in 2000ms"
        );
    }

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
