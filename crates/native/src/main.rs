mod args;
mod burst;
mod companion;
mod context;
mod datadir;
mod datefolder;
mod dialog;
mod dropped;
mod empty;
mod exif;
mod eyes;
mod facemesh;
mod faces;
mod filter;
mod focus;
mod folderops;
mod history;
mod hold;
mod icons;
mod idle;
mod import;
mod judgment;
mod keys;
mod logger;
mod mark;
mod mcp;
mod menu;
mod meta;
mod modal;
mod orient;
mod overlay;
mod pager;
mod panels;
mod peaking;
mod perf;
mod perfgraph;
mod prefetch;
mod progress;
mod rename;
mod resync;
mod scan;
mod selection;
mod sequence;
mod session;
mod settings;
mod settingsmodal;
mod settingsview;
mod sidecars;
mod sort;
mod strip;
mod theme;
mod thumbs;
mod timing;
mod trash;
mod tree;
mod treekeys;
mod viewed;
mod viewonly;
mod zoom;

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    env,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    process,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use args::Args;
use burst::{BURST_GAP_MS, BurstMark, BurstMember};
use filter::{Facts, FilterState};
use futures::{
    StreamExt,
    channel::mpsc::{self, UnboundedSender},
};
use gpui::{
    Anchor, AnyElement, App, Bounds, ClickEvent, ClipboardItem, ContentMask, Context, Corners,
    Entity, ExternalPaths, FocusHandle, FontWeight, KeyBinding, KeyDownEvent, Keystroke,
    MouseButton, MouseDownEvent, NoAction, PathPromptOptions, Pixels, Point, RenderImage,
    ScrollEvent, ScrollHandle, ScrollStrategy, SharedString, Task, TextAlign, TitlebarOptions,
    TransformationMatrix, UniformListScrollHandle, WeakEntity, Window, WindowBounds, WindowOptions,
    anchored, canvas, deferred, div, fill, outline, point, prelude::*, px, relative, rgb, rgba,
    size, svg, uniform_list,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Root, Sizable,
    button::{Button, ButtonVariants},
    dialog::{Dialog, DialogFooter},
    scroll::ScrollableElement,
    tooltip::Tooltip,
};
use gpui_platform::application;
use history::{Judged, TrashEntry, Undo};
use hold::PaintHold;
use image::{Frame, RgbaImage};
use judgment::{Command, State};
use keys::Bindings;
use menu::{Lead, Menus, Row, Which};
use meta::Shown;
use pager::{Finished, Move, Pager};
use panels::{Panel, Panels};
use peaking::{Peaking, View};
use prefetch::{Heading, Ring};
use resync::Resync;
use riffle_backend::{
    exif::Metadata,
    folders::{self, Folder},
    index::{self, Index},
    listing::canonicalize,
    sidecar::{self, SidecarFormat, Writer},
    treewatch::TreeWatch,
    watch::{self, Watch},
};
use riffle_core::{
    Flag,
    arw::FocusLocation,
    reader,
    scan::{Paused, ReadSlots, ScanFocus},
    xmp::LabelNames,
};
use scan::{Event, Listed, Outputs, Prepared, Running, Scan};
use selection::Selection;
use settings::Settings;
use settingsview::{Change, SettingsView};
use sort::{SortFacts, SortKey};
use strip::{Answer, Marks, Strip, Tint, Tip, TipRegion};
use thumbs::{MAX_IN_FLIGHT, Read, Thumbs};
use timing::{Page, Source, Summary};
use tree::{Child, IGNORE_CASE, Relation, StepKey, Tree, TreeCommand, TreeKey, Typed};
use treekeys::Gate;
use viewed::{Batch, Batches, Dwell, Latest, VIEW_DWELL_MS};
use zoom::{Arrived, Zoom};

const AUTO_SETTLE: Duration = Duration::from_secs(2);
/// Out of phase with 60 Hz vsync.
const AUTO_INTERVAL: Duration = Duration::from_millis(1003);

/// A decoded preview and what showing it needs, on screen or in the
/// prefetch ring.
struct Held {
    image: Arc<RenderImage>,
    jpeg_kb: usize,
    px: (u32, u32),
    orientation: u16,
    meta: Metadata,
    focus: Option<FocusLocation>,
}

impl prefetch::Pixels for Held {
    fn pixels(&self) -> u64 {
        u64::from(self.px.0) * u64::from(self.px.1)
    }
}

/// The image on screen's facts beside the image and its `Metadata` (in
/// `Riffle::shown`), to hand it back to the prefetch ring.
struct OnScreen {
    path: String,
    jpeg_kb: usize,
    px: (u32, u32),
    orientation: u16,
    focus: Option<FocusLocation>,
    /// Off once a re-listing may have changed the file's bytes.
    keepable: bool,
}

struct Loaded {
    held: Held,
    read: Duration,
    decode: Duration,
    orient: Duration,
    finished: Instant,
    /// The scan's pause, held until the image is painted (`hold.rs`).
    paused: Paused,
}

/// mozjpeg reports a corrupt JPEG by unwinding, which would otherwise lose
/// the in-flight request and stop the paging.
fn guarded<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|e| {
        Err(e
            .downcast::<String>()
            .map_or_else(|_| "panic while reading the preview".into(), |msg| *msg))
    })
}

fn load(path: &Path, slots: &ReadSlots) -> Result<Loaded, String> {
    let started = Instant::now();
    let paused = slots.pause();
    let preview = reader::read_preview(path);
    let (arw, jpeg) = preview.map_err(|e| e.to_string())?;
    let read = started.elapsed();
    let meta = riffle_backend::exif::metadata(file_name(path), &arw.shot);
    let started = Instant::now();
    let mut d = mozjpeg::Decompress::new_mem(&jpeg)
        .map_err(|e| e.to_string())?
        .to_colorspace(mozjpeg::ColorSpace::JCS_EXT_BGRA)
        .map_err(|e| e.to_string())?;
    let (w, h) = (d.width() as u32, d.height() as u32);
    let bgra = d.read_scanlines::<u8>().map_err(|e| e.to_string())?;
    d.finish().map_err(|e| e.to_string())?;
    let decode = started.elapsed();
    let started = Instant::now();
    let (bgra, upright_w, upright_h) = orient::rotate_bgra(bgra, w, h, arw.orientation);
    let orient = started.elapsed();
    let buffer =
        RgbaImage::from_raw(upright_w, upright_h, bgra).ok_or("the decode came up short")?;
    let image = Arc::new(RenderImage::new(vec![Frame::new(buffer)]));
    Ok(Loaded {
        held: Held {
            image,
            jpeg_kb: jpeg.len() / 1024,
            px: (w, h),
            orientation: arw.orientation,
            meta,
            focus: arw.shot.focus,
        },
        read,
        decode,
        orient,
        finished: Instant::now(),
        paused,
    })
}

fn preview_size(path: &Path) -> Result<(u32, u32), String> {
    let (_, jpeg) = reader::read_preview(path).map_err(|e| e.to_string())?;
    let d = mozjpeg::Decompress::new_mem(&jpeg).map_err(|e| e.to_string())?;
    Ok((d.width() as u32, d.height() as u32))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// The files shown: the listing in natural order, narrowed by `--size` and
/// `--max`. The index always reconciles the whole listing, so a measurement
/// run never drops rows.
fn shown(files: &[String], wanted: Option<(u32, u32)>, max: Option<usize>) -> Vec<String> {
    scan::natural_order(files)
        .into_iter()
        .filter(|p| wanted.is_none_or(|size| guarded(|| preview_size(p)) == Ok(size)))
        .take(max.unwrap_or(usize::MAX))
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

/// What the log calls a held operation dropped by an open.
fn held_label(held: &HeldOp) -> String {
    match held {
        HeldOp::Run(target) => format!("held run in {}", target.dir),
        HeldOp::Undo(entry) => format!("held undo of run {}", entry.run_id),
        HeldOp::Redo(entry) => format!("held redo of run {}", entry.run_id),
        HeldOp::Sidecars(target) => format!("held {} of {}", target.kind.label(), target.dir),
        HeldOp::Rename(held) => format!("held rename of {} to {}", held.path, held.name),
    }
}

/// The listed files' judgments and facts by path, and the folder's EXIF
/// groups.
type Entries = (HashMap<String, State>, HashMap<String, Facts>, exif::Groups);

/// What the gate holds for the listing or scan under way: a confirmed run,
/// or the undo or redo of one, or a folder's sidecar rewrite or delete, or
/// a folder rename.
enum HeldOp {
    Run(trash::Target),
    Undo(TrashEntry),
    Redo(TrashEntry),
    Sidecars(sidecars::Target),
    Rename(rename::Held),
}

struct Opened {
    listed: Listed,
    shown: Vec<String>,
    outputs: Outputs,
    /// `None` without an index.
    todo: Option<Result<Prepared, String>>,
    /// `None` without an index or when the read failed.
    entries: Option<Entries>,
    /// The file the folder was left at, to resume at.
    last_viewed: Option<String>,
    /// The folder's files the index has as viewed.
    viewed: HashSet<String>,
}

/// The folder's resume point and viewed files from the index; nothing when
/// a read fails, which only costs resuming at the first file.
fn remembered(reader: &Mutex<Index>, dir: &str) -> (Option<String>, HashSet<String>) {
    let index = index::lock(reader);
    let last_viewed = index.last_viewed(dir).unwrap_or_else(|e| {
        log::warn!("failed to read the last viewed file: {e}");
        None
    });
    let viewed = index.viewed_files(dir).unwrap_or_else(|e| {
        log::warn!("failed to read the viewed files: {e}");
        Vec::new()
    });
    (last_viewed, viewed.into_iter().collect())
}

/// The judgment and the facts of each of the folder's files the index has,
/// after the sidecar pass, from one `entries` read; `None` when it fails.
fn entries_of(reader: &Mutex<Index>, dir: &str) -> Option<Entries> {
    let entries = index::lock(reader)
        .entries(dir)
        .inspect_err(|e| log::warn!("failed to read the entries of {dir}: {e}"))
        .ok()?;
    let judgments = entries
        .iter()
        .map(|file| (file.path.clone(), State::from(file)))
        .collect();
    let facts = entries
        .iter()
        .map(|file| (file.path.clone(), Facts::from(file)))
        .collect();
    let groups = exif::groups_of(entries.iter().map(|file| file.exif.as_ref()));
    Some((judgments, facts, groups))
}

struct Riffle {
    focus: FocusHandle,
    /// The listing in natural order, narrowed by `--size` / `--max`: the
    /// Tauri `allFiles`.
    all: Vec<String>,
    /// `all` in the order in force, narrowed by the filter: what the pager,
    /// the strip and the pane index into; only the open and `refilter`
    /// rebuild it.
    files: Vec<PathBuf>,
    /// The listed files' facts by path, from the index.
    facts: HashMap<String, Facts>,
    /// The open folder's bursts over `all`, regrouped whenever the entries
    /// are set.
    bursts: HashMap<String, BurstMember>,
    /// Pass 2 files reported since the last read of the index, laid over a
    /// snapshot read before they were written.
    pending_faces: Vec<riffle_backend::index::FaceReady>,
    /// The sort key the user has; a view-only folder is in capture order
    /// whatever it is (`viewonly::sort_for`).
    sort: SortKey,
    /// The strip's filter; its remembered sections are written back on a menu
    /// change, `Output` and the EXIF checks live for the session only.
    filter: FilterState,
    /// The open folder's EXIF groups, the filter menu's last sections.
    exif_groups: exif::Groups,
    /// The anchor a menu change lands on while the filter hides every file.
    kept_anchor: filter::KeptAnchor,
    /// The status row's sort or filter menu, while one is open.
    menus: Menus,
    /// The open sort or filter menu's scroll, new on every press of a menu's
    /// button so a menu opens at its top.
    menu_scroll: ScrollHandle,
    /// The listed files with an export, `None` while the folder gives no
    /// answer, and the listing the answer is about (`filter::output_of`).
    outputs: Option<HashSet<String>>,
    outputs_listing: HashSet<String>,
    /// Why the last export read failed, in the pane's notes until one
    /// succeeds.
    outputs_failure: Option<String>,
    pager: Pager,
    status: SharedString,
    index: Result<Arc<Mutex<Index>>, String>,
    /// The index's read-only connection the strip's thumbnails come through,
    /// so they never wait behind a scan's write transaction.
    reader: Option<Arc<Mutex<Index>>>,
    strip: Strip,
    thumbs: Thumbs,
    /// The strip spans the window, so this is the window's width.
    strip_width: f32,
    /// The launch's eviction, which the first scan waits for.
    eviction: Option<JoinHandle<()>>,
    /// Resolves once the latest listing has stopped the previous scan and
    /// joined the eviction, which the next listing waits for first.
    settled: Option<futures::channel::oneshot::Receiver<()>>,
    slots: ReadSlots,
    scan: Running,
    /// The pending `ScanFocus` update, replaced on every change.
    scan_focus: Option<Task<()>>,
    next_scan: u64,
    /// The open folder's canonical path, the key of its index rows.
    dir: Option<String>,
    /// Whether the folder and the window geometry are remembered for the
    /// next launch; off under `--auto`.
    remember: bool,
    /// Bumped by every open, without a lock: a listing or a reveal of an
    /// earlier open is dropped.
    opens: Arc<AtomicU64>,
    /// Bumped by every drop, a port of `dropCounter`: a drop whose path
    /// resolves after a later drop opens nothing.
    drops: u64,
    /// Taken by a listing's background task around "the counter still equals
    /// my token, then set the watcher", never by the UI thread.
    watching_lock: Arc<Mutex<()>>,
    /// The left-hand folder tree and its drawn rows, rebuilt when it changes.
    tree: Tree,
    tree_rows: Vec<tree::Row>,
    /// The open folder as the tree spells it, and its row while drawn.
    tree_open: Option<String>,
    tree_open_row: Option<usize>,
    tree_scroll: UniformListScrollHandle,
    /// The tree has the keyboard: a click in its pane gave it, `escape` or a
    /// press elsewhere hands it back to the photo.
    tree_keys: bool,
    /// The tree's keyboard cursor, drawn while it has the keyboard, its row
    /// while drawn, and the type-ahead typed so far.
    tree_cursor: Option<String>,
    tree_cursor_row: Option<usize>,
    typed: Typed,
    /// The roots have been read; an open before that waits in
    /// `reveal_wanted`.
    roots_loaded: bool,
    reveal_wanted: Option<String>,
    /// The last folder listing of the tree that failed, until another open.
    tree_note: Option<String>,
    /// The watcher on the tree's expanded, drawn folders, the set last handed
    /// to it (`None` to hand the next one over whatever it is), and the
    /// latest hand-over, which awaits the one before it.
    tree_watch: Arc<TreeWatch>,
    tree_watched: Option<Vec<String>>,
    tree_syncing: Option<Task<()>>,
    /// The right-click menu, while one is open.
    context: context::Menu,
    /// The open right-click menu's scroll, new on every open.
    context_scroll: ScrollHandle,
    /// The strip's `Move to Trash…` / `Delete Permanently…` flow.
    trash: trash::TrashFlow,
    /// Its dialog in `Root`'s layer, open while the flow asks or runs.
    trash_modal: modal::Modal,
    /// The focus of the dialog's run and Cancel buttons.
    trash_buttons: [FocusHandle; 2],
    /// The folder menu's operations and their dialog.
    folderops: folderops::FolderOps,
    /// That dialog in `Root`'s layer, open while a flow shows it.
    folder_modal: modal::Modal,
    /// The focus of its run and Cancel buttons.
    folder_buttons: [FocusHandle; 2],
    /// A confirmed run, or a run's undo or redo, held for the listing or
    /// scan under way; in flight while one runs, which no listing or scan
    /// starts under.
    gate: idle::IdleGate<HeldOp>,
    /// The recorded runs to the Trash, for undo.
    runs: Arc<riffle_backend::trash::Runs>,
    /// The undo entry whose restore or redo is out, so a second undo or
    /// redo does nothing until it comes back.
    trash_step: Option<u64>,
    /// The judgments and `seen` membership of the files a Move to Trash
    /// took, kept for an undo landing before the re-listing reads them back.
    trashed_state: HashMap<String, (Option<State>, bool)>,
    /// How far the run in flight got, `(mode, done, total)`, in the pane's
    /// notes and the progress bar.
    trashing: Option<(trash::Mode, usize, usize)>,
    /// A folder the tree opened while a run was in flight, opened whole at
    /// its end (only the launch's open has `--size` / `--max`).
    deferred_open: Option<PathBuf>,
    /// The panes shown (`toggleLeft`, `toggleRight`, `toggleStrip`,
    /// `toggleSides`).
    panels: Panels,
    /// A pane change is written to `panels`; off under `--auto`.
    remember_panels: bool,
    /// `--size` and `--max`, which a re-listing applies again.
    wanted: Option<(u32, u32)>,
    max: Option<usize>,
    /// Bumped by every listing, so a read of the index for the previous one
    /// is dropped.
    listing: u64,
    /// Bumped by every listing and every rebuild of `files`, so a preview
    /// read for an index into an older `files` is dropped.
    order: u64,
    watch: Arc<Watch>,
    resync: Resync,
    last_viewed: Latest<(String, String)>,
    viewed: Batches,
    dwell: Dwell,
    /// The running dwell clock; dropping it stops it.
    dwell_timer: Option<Task<()>>,
    events: UnboundedSender<Event>,
    scan_status: scan::Status,
    failed: usize,
    image: Option<Arc<RenderImage>>,
    /// The meta pane's file, the one whose preview is on screen, with its
    /// index, `None` once a refilter hid it.
    shown: Option<(Option<usize>, Shown)>,
    /// The open folder holds only JPEGs.
    view_only: bool,
    settings: Settings,
    bindings: Bindings,
    /// The settings modal, laid out only while open.
    settings_view: Entity<SettingsView>,
    /// The open folder's judgments by listed path.
    judgments: HashMap<String, State>,
    /// The paths the index last returned an entry for, whose label is known.
    seen: HashSet<String>,
    /// The paths judged since the folder opened, whose state in the view a
    /// later read of the index does not replace.
    touched: HashSet<String>,
    /// `None` without an index.
    writer: Option<Arc<Writer>>,
    /// The judgment stores, run in the order of the keys.
    stores: judgment::Queue,
    /// Held by a sidecar format switch off the main thread, one at a time.
    format_switch: Arc<Mutex<()>>,
    /// The format last asked for, written on the main thread; a switch task that
    /// finds it differs from its own is superseded and does nothing.
    format_latest: Arc<Mutex<SidecarFormat>>,
    /// The open folder's undo and redo histories.
    undo: Undo,
    /// The strip's multi-selection; the current file is always a member.
    selection: Selection,
    /// `Undid <name>` / `Redid <name>`, or a run's `trash::trashed_status`,
    /// until the view moves on.
    step_note: Option<String>,
    /// The sidecars the open could not read, as `<name>: <message>`.
    notes: Vec<String>,
    /// The judgments that failed to be stored or written, one per path, the
    /// newest message replacing the older.
    write_errors: Vec<(String, String)>,
    /// The `f` overlay's toggle and what it draws.
    marks: overlay::Marks,
    /// Shown images that the next presented frame no longer uses.
    retired: Rc<RefCell<Vec<Arc<RenderImage>>>>,
    /// The page the next presented frame completes, and whether a key or a
    /// strip click turned it (an unkeyed one only feeds the `d` graph).
    pending: Rc<RefCell<Option<(Page, bool)>>>,
    /// The shown image's scan pause, released once it is painted.
    hold: PaintHold,
    summary: Rc<RefCell<Summary>>,
    key: Option<Instant>,
    sent: Instant,
    /// When the current file last changed, where an unkeyed page's latency
    /// starts.
    switched: Instant,
    zoom: Zoom<zoom::Crop>,
    /// The pending re-cut after a resize; dropping it cancels it.
    zoom_timer: Option<Task<()>>,
    /// The preview canvas's size in device pixels, as last painted.
    viewport: Rc<Cell<(u32, u32)>>,
    /// The keyed crop the next presented frame shows.
    zoom_pending: Rc<RefCell<Option<zoom::Timed>>>,
    peaking: Peaking<Arc<RenderImage>, Arc<RenderImage>>,
    /// The performance overlay on `d`.
    perf: perfgraph::Overlay,
    /// The MCP companion's server, `None` while `mcpEnabled` is off.
    mcp: Option<mcp::Server>,
    /// The decoded neighbours of the current file, `None` under
    /// `--no-prefetch`.
    prefetch: Option<Ring<Held>>,
    heading: Heading,
    /// `None` while `image` is.
    screen: Option<OnScreen>,
}

impl Riffle {
    #[allow(clippy::too_many_arguments)]
    fn new(
        focus: FocusHandle,
        index: Result<Indexes, String>,
        settings: Settings,
        mcp_port: Option<u16>,
        prefetch: bool,
        remember: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (index, reader) = match index {
            Ok((writer, reader)) => (Ok(writer), Some(reader)),
            Err(e) => (Err(e), None),
        };
        // Held by the quit handler, not the view, as the writer's drain is.
        let geometry: Arc<Mutex<Option<session::Geometry>>> = Arc::default();
        let observed = geometry.clone();
        cx.observe_window_bounds(window, move |this, window, cx| {
            let geometry = window_geometry(window, cx);
            if remember && geometry.is_usable() {
                *observed.lock().unwrap() = Some(geometry);
            }
            this.strip_width = f32::from(window.viewport_size().width);
            this.strip.clamp(this.strip_width);
            this.strip_moved(cx);
            this.zoom_resized(cx);
        })
        .detach();
        let eviction = index.as_ref().ok().map(|index| {
            let index = index.clone();
            thread::spawn(move || {
                match index::lock(&index).evict(index::now_secs(), index::EVICT_POLICY) {
                    Ok(summary) => log::info!("index eviction: {summary:?}"),
                    Err(e) => log::error!("failed to evict stale folders from the index: {e}"),
                }
            })
        });
        // The scan's callbacks run on its own threads, which cannot touch the
        // entity; they only send, and this task applies on the main thread.
        let (events, mut received) = mpsc::unbounded();
        cx.spawn(async move |this, cx| {
            while let Some(event) = received.next().await {
                if this.update(cx, |this, cx| this.scanned(event, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let watch = {
            let events = events.clone();
            Watch::spawn(move |dir| {
                let _ = events.unbounded_send(Event::Changed(dir));
            })
        };
        let writer = index.as_ref().ok().map(|index| {
            let events = events.clone();
            Arc::new(Writer::spawn(index.clone(), move |path, message| {
                log::warn!("sidecar write failed: path={} {message}", path.display());
                let _ = events.unbounded_send(Event::SidecarError {
                    path: path.to_string_lossy().into_owned(),
                    message: message.to_string(),
                });
            }))
        });
        // Held by the app, not the view: closing the window may release the
        // view before the quit handlers run. The drain runs as the handler is
        // called, so GPUI's 200 ms bound on the returned future does not cut it.
        let stores = judgment::Queue::new();
        let running = Running::default();
        if let Some(writer) = writer.clone() {
            let (stores, running) = (stores.clone(), running.clone());
            App::on_app_quit(cx, move |_| {
                let started = Instant::now();
                let stopped = running.stop_within(scan::STOP_TIMEOUT);
                log::info!(
                    "quit: scan stopped={stopped} in {}ms",
                    started.elapsed().as_millis()
                );
                let stored = stores.drain(sidecar::DRAIN_TIMEOUT);
                let drained = writer.flush(sidecar::DRAIN_TIMEOUT);
                log::info!(
                    "quit: judgment stores drained={stored} sidecar writer drained={drained}"
                );
                async {}
            })
            .detach();
        }
        Self::read_roots(cx);
        // A volume mounted since shows up when the window becomes active.
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() && this.roots_loaded {
                Self::read_roots(cx);
            }
        })
        .detach();
        let tree_watch = {
            let events = events.clone();
            TreeWatch::spawn(move |dir| {
                let _ = events.unbounded_send(Event::TreeChanged(dir));
            })
        };
        let writes = settings.writes.clone();
        let loaded = settings.window.clone();
        App::on_app_quit(cx, move |_| {
            if let Some(geometry) = geometry.lock().unwrap().take()
                && Some(&geometry) != loaded.as_ref()
            {
                writes.set("windowBounds", geometry.setting());
            }
            let drained = writes.drain(settings::DRAIN_TIMEOUT);
            log::info!("quit: settings writes drained={drained}");
            async {}
        })
        .detach();
        let settings_view = cx.new(|cx| SettingsView::new(window, cx));
        cx.subscribe_in(
            &settings_view,
            window,
            |this, _, change, window, cx| match *change {
                Change::AutoAdvance(on) => {
                    this.settings.auto_advance = on;
                    this.settings.writes.set("autoAdvance", on.into());
                }
                Change::SidecarFormat(format) => this.switch_format(format, cx),
                Change::LabelNames(ref names) => this.switch_names(names.clone(), cx),
                Change::Keymap(ref keymap) => {
                    this.bindings = Bindings::new(keymap);
                    this.settings.writes.shortcuts(keymap);
                    this.settings.keymap = keymap.clone();
                }
                Change::ImportRoot(ref root) => {
                    this.settings.writes.set("importRoot", root.as_str().into());
                    this.settings.import_root = root.clone();
                }
                Change::ImportPattern(ref pattern) => {
                    this.settings
                        .writes
                        .set("importPattern", pattern.as_str().into());
                    this.settings.import_pattern = pattern.clone();
                }
                Change::Closed => window.focus(&this.focus, cx),
            },
        )
        .detach();
        Self {
            mcp: mcp::start(
                settings.mcp_enabled,
                mcp_port,
                events.clone(),
                reader.clone(),
                cx,
            ),
            focus,
            all: Vec::new(),
            files: Vec::new(),
            facts: HashMap::new(),
            bursts: HashMap::new(),
            pending_faces: Vec::new(),
            sort: settings.sort,
            filter: {
                let mut filter = FilterState::default();
                filter.apply_stored(&settings.filter);
                filter
            },
            exif_groups: exif::Groups::default(),
            kept_anchor: filter::KeptAnchor::default(),
            menus: Menus::default(),
            menu_scroll: ScrollHandle::new(),
            outputs: None,
            outputs_listing: HashSet::new(),
            outputs_failure: None,
            pager: Pager::new(0),
            status: "no folder".into(),
            index,
            reader,
            strip: Strip::default(),
            thumbs: Thumbs::default(),
            strip_width: f32::from(window.viewport_size().width),
            eviction,
            settled: None,
            slots: ReadSlots::new(scan::READ_SLOTS),
            scan: running,
            scan_focus: None,
            next_scan: 0,
            dir: None,
            remember,
            opens: Arc::default(),
            drops: 0,
            watching_lock: Arc::default(),
            tree: Tree::default(),
            tree_rows: Vec::new(),
            tree_open: None,
            tree_open_row: None,
            tree_scroll: UniformListScrollHandle::new(),
            tree_keys: false,
            tree_cursor: None,
            tree_cursor_row: None,
            typed: Typed::default(),
            roots_loaded: false,
            reveal_wanted: None,
            tree_note: None,
            tree_watch: Arc::new(tree_watch),
            tree_watched: None,
            tree_syncing: None,
            context: context::Menu::default(),
            context_scroll: ScrollHandle::new(),
            trash: trash::TrashFlow::default(),
            trash_modal: modal::Modal::new(window.window_handle()),
            trash_buttons: [cx.focus_handle(), cx.focus_handle()],
            folderops: folderops::FolderOps::default(),
            folder_modal: modal::Modal::new(window.window_handle()),
            folder_buttons: [cx.focus_handle(), cx.focus_handle()],
            gate: idle::IdleGate::default(),
            runs: Arc::default(),
            trash_step: None,
            trashed_state: HashMap::new(),
            trashing: None,
            deferred_open: None,
            // `--auto` starts with every pane shown and leaves the stored
            // panes alone.
            panels: if remember {
                settings.panels
            } else {
                Panels::default()
            },
            remember_panels: remember,
            wanted: None,
            max: None,
            listing: 0,
            order: 0,
            watch: Arc::new(watch),
            resync: Resync::default(),
            last_viewed: Latest::default(),
            viewed: Batches::default(),
            dwell: Dwell::default(),
            dwell_timer: None,
            events,
            scan_status: scan::Status::default(),
            failed: 0,
            image: None,
            shown: None,
            view_only: false,
            bindings: Bindings::new(&settings.keymap),
            format_latest: Arc::new(Mutex::new(settings.format)),
            settings,
            settings_view,
            judgments: HashMap::new(),
            seen: HashSet::new(),
            touched: HashSet::new(),
            writer,
            stores,
            format_switch: Arc::default(),
            undo: Undo::new(),
            selection: Selection::default(),
            step_note: None,
            notes: Vec::new(),
            write_errors: Vec::new(),
            marks: overlay::Marks::default(),
            retired: Rc::default(),
            pending: Rc::default(),
            hold: PaintHold::default(),
            summary: Rc::default(),
            key: None,
            sent: Instant::now(),
            switched: Instant::now(),
            zoom: Zoom::default(),
            zoom_timer: None,
            viewport: Rc::default(),
            zoom_pending: Rc::default(),
            peaking: Peaking::default(),
            prefetch: prefetch.then(Ring::default),
            heading: Heading::default(),
            screen: None,
            perf: perfgraph::Overlay::default(),
        }
    }

    fn start(args: Args, cx: &mut Context<Self>) {
        let Args {
            folder,
            auto,
            size,
            max,
            index: _,
            settings: _,
            import_settings: _,
            log: _,
            mcp_port: _,
            no_prefetch: _,
        } = args;
        cx.spawn(async move |this, cx| {
            if let Some(folder) = folder
                && let Ok(opening) = this.update(cx, |this, cx| this.open(folder, size, max, cx))
            {
                opening.await;
            }
            let Some(presses) = auto else { return };
            cx.background_executor().timer(AUTO_SETTLE).await;
            for _ in 0..presses {
                this.update(cx, |this, cx| this.go(Move::Next, cx)).ok();
                cx.background_executor().timer(AUTO_INTERVAL).await;
            }
            this.update(cx, |this, cx| this.quit(cx)).ok();
        })
        .detach();
    }

    fn open(
        &mut self,
        folder: PathBuf,
        wanted: Option<(u32, u32)>,
        max: Option<usize>,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        // No listing starts under a run; its end opens the folder.
        if self.gate.in_flight() {
            self.deferred_open = Some(folder);
            return Task::ready(());
        }
        let dropped = self.gate.discard().and_then(|held| {
            log::info!("trash: {} dropped by an open", held_label(&held));
            self.held_dropped(held, None)
        });
        self.opens.fetch_add(1, Ordering::SeqCst);
        self.tree_note = None;
        let shown = std::path::absolute(&folder).unwrap_or_else(|_| folder.clone());
        self.reveal(shown.to_string_lossy().into_owned(), cx);
        self.wanted = wanted;
        self.max = max;
        self.resync.open();
        let listing = self.list(folder, false, cx);
        match dropped {
            Some(note) => self.note_after(listing, note, cx),
            None => listing,
        }
    }

    fn opens(&self) -> u64 {
        self.opens.load(Ordering::SeqCst)
    }

    /// A click on a folder's name in the tree, or an import's first folder:
    /// open it whole, without the command line's `--size` / `--max`.
    fn open_from_tree(&mut self, path: String, cx: &mut Context<Self>) -> Task<()> {
        if self.refuse_renaming(&path, cx) {
            return Task::ready(());
        }
        self.open(PathBuf::from(path), None, None, cx)
    }

    /// `open`: the OS folder picker, its folder opened as a click in the
    /// tree opens it, unless another open came first while it was up.
    fn pick_folder(&mut self, cx: &mut Context<Self>) {
        let opens = self.opens();
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => None,
                Ok(Err(e)) => {
                    this.update(cx, |this, cx| {
                        this.fail(&format!("open folder: {e}"));
                        cx.notify();
                    })
                    .ok();
                    None
                }
            };
            let Some(folder) = chosen else { return };
            this.update(cx, |this, cx| {
                if this.opens() == opens {
                    this.open(folder, None, None, cx).detach();
                }
            })
            .ok();
        })
        .detach();
    }

    /// A drop onto the window: one folder opens as a click in the tree
    /// opens it, one file opens its parent, anything else is refused.
    fn drop_paths(&mut self, paths: &ExternalPaths, cx: &mut Context<Self>) {
        let Some(path) = dropped::single(paths.paths()).map(Path::to_path_buf) else {
            self.status = dropped::DROP_TEXT.into();
            cx.notify();
            return;
        };
        self.drops += 1;
        let drop = self.drops;
        let resolve = cx
            .background_executor()
            .spawn(async move { dropped::dropped_dir(&path) });
        cx.spawn(async move |this, cx| {
            let folder = resolve.await;
            this.update(cx, |this, cx| {
                if this.drops != drop {
                    return;
                }
                match folder {
                    Some(folder) => this.open(folder, None, None, cx).detach(),
                    None => {
                        this.status = dropped::DROP_TEXT.into();
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// Show the panes `panels`: a hidden left pane gives up the keyboard, a
    /// 1:1 crop is re-cut for the new canvas, and the panes are remembered.
    fn change_panels(&mut self, panels: Panels, cx: &mut Context<Self>) {
        self.panels = panels;
        if !panels.left {
            self.set_tree_keys(false, cx);
        }
        self.zoom_resized(cx);
        if self.remember_panels {
            self.settings.writes.set("panels", panels.setting());
        }
        cx.notify();
    }

    /// Read the roots on a background task; `add_roots` skips the ones
    /// already shown, so this also runs again on the window's activation,
    /// where a failure is not reported.
    fn read_roots(cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let roots = cx
                .background_spawn(async { folders::roots(dirs::home_dir()) })
                .await;
            this.update(cx, |this, cx| this.roots_read(roots, cx)).ok();
        })
        .detach();
    }

    fn roots_read(&mut self, roots: Vec<folders::FolderNode>, cx: &mut Context<Self>) {
        self.tree.add_roots(roots.into_iter().map(Child::from));
        self.roots_loaded = true;
        self.tree_changed(cx);
        if let Some(path) = self.reveal_wanted.take() {
            self.reveal(path, cx);
        }
    }

    /// Rebuild the tree's rows, find the open folder's among them and hand a
    /// changed watched set to the tree's watcher.
    fn tree_changed(&mut self, cx: &mut Context<Self>) {
        self.tree_rows = self.tree.rows();
        self.tree_open_row = self.tree_open.as_ref().and_then(|open| {
            self.tree_rows.iter().position(|row| {
                tree::relation(&row.path, open, IGNORE_CASE) == Some(Relation::Same)
            })
        });
        self.tree_cursor_row = self
            .tree_cursor
            .as_ref()
            .and_then(|cursor| self.tree_rows.iter().position(|row| &row.path == cursor));
        self.sync_tree_watches(cx);
        cx.notify();
    }

    /// Hand the watched set to the tree's watcher off the main thread, after
    /// the previous hand-over, so they apply in order; an unchanged set is
    /// not sent. A collapse releases watches, which waits on the watcher.
    fn sync_tree_watches(&mut self, cx: &mut Context<Self>) {
        let dirs = self.tree.watched_folders();
        if self.tree_watched.as_ref() == Some(&dirs) {
            return;
        }
        self.tree_watched = Some(dirs.clone());
        let previous = self.tree_syncing.take();
        let watch = self.tree_watch.clone();
        self.tree_syncing = Some(cx.background_spawn(async move {
            if let Some(previous) = previous {
                previous.await;
            }
            watch.set(&dirs);
        }));
    }

    /// The tree's watcher saw `dir` change: re-list it while it is still
    /// expanded and drawn. A failed re-list is a background refresh's and
    /// leaves the folder as it was: deleting a folder often fails its own
    /// re-list first, and its parent's re-list removes the row.
    fn tree_folder_changed(&mut self, dir: String, cx: &mut Context<Self>) {
        if !self
            .tree_rows
            .iter()
            .any(|row| row.path == dir && row.expanded)
        {
            return;
        }
        cx.spawn(async move |this, cx| {
            let listing = dir.clone();
            let listed = cx
                .background_spawn(async move { folders::list(Path::new(&listing)) })
                .await;
            match listed {
                Ok(folder) => {
                    this.update(cx, |this, cx| {
                        this.tree_listed(&dir, Ok(folder), false);
                        this.tree_changed(cx);
                    })
                    .ok();
                }
                Err(e) => log::warn!("failed to re-list the subfolders of {dir}: {e}"),
            }
        })
        .detach();
    }

    /// The tree menu's `Refresh`: re-list the folder, expanded or not, and
    /// hand the watched set over again, so a watch that failed is retried.
    /// The open folder itself is re-listed as a watcher change would.
    fn tree_refresh(&mut self, dir: String, cx: &mut Context<Self>) {
        let open = self
            .tree_open
            .as_ref()
            .is_some_and(|open| tree::relation(open, &dir, IGNORE_CASE) == Some(Relation::Same));
        if open && self.dir.is_some() && self.resync.changed() {
            self.relist(cx);
        }
        cx.spawn(async move |this, cx| {
            let listing = dir.clone();
            let listed = cx
                .background_spawn(async move { folders::list(Path::new(&listing)) })
                .await;
            this.update(cx, |this, cx| {
                this.tree_listed(&dir, listed, true);
                this.tree_watched = None;
                this.tree_changed(cx);
            })
            .ok();
        })
        .detach();
    }

    /// A right-click on a tree row: its menu at the pointer, the status row's
    /// menus closed.
    fn tree_context(
        &mut self,
        at: Point<Pixels>,
        path: String,
        name: String,
        cx: &mut Context<Self>,
    ) {
        self.menus.close();
        let root = self
            .tree_rows
            .iter()
            .any(|row| row.path == path && row.depth == 0);
        self.context_scroll = ScrollHandle::new();
        self.context.open_folder(
            (f32::from(at.x), f32::from(at.y)),
            path,
            name,
            folders::REVEAL_LABEL,
            root,
        );
        cx.notify();
    }

    /// A right-click on the strip at `at`, `x` from its left edge, as the
    /// Tauri strip's `contextmenu` handler: a cell outside the selection
    /// becomes the selection alone, one inside keeps it, the focus moves to
    /// the cell, and the menu opens on the selection's targets. The strip's
    /// empty area opens nothing.
    fn strip_context(&mut self, at: Point<Pixels>, x: f32, cx: &mut Context<Self>) {
        let Some(index) = self.strip.hit_test(x) else {
            return;
        };
        let files = path_strings(&self.files);
        if !self.selection.selected.contains(&files[index]) {
            self.set_selection(selection::single(Some(&files[index])));
        }
        self.focus_on(Move::To(index), cx);
        self.menus.close();
        let focused = files[self.pager.current()].clone();
        let paths = selection::targets(&self.selection, &files, self.pager.current());
        let state = self.judgments.get(&focused).cloned().unwrap_or_default();
        let groups = context::strip_groups(
            &self.settings.keymap.bindings(),
            &state,
            folders::REVEAL_LABEL,
            self.view_only,
            paths.len(),
            self.trashing.is_some(),
        );
        self.context_scroll = ScrollHandle::new();
        self.context
            .open_files((f32::from(at.x), f32::from(at.y)), focused, paths, groups);
        cx.notify();
    }

    /// A click on an item of the right-click menu.
    fn context_action(
        &mut self,
        action: context::Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.context.close() {
            Some(context::Target::Folder { path, .. })
                if action == context::Action::RenameFolder =>
            {
                self.start_rename(path, window, cx);
            }
            Some(context::Target::Folder { path, name }) => {
                self.folder_action(action, path, name, cx);
            }
            Some(context::Target::Files { focused, paths }) => {
                self.files_action(action, &focused, &paths, cx);
            }
            None => {}
        }
        cx.notify();
    }

    /// An item of the strip's menu: the reveal acts on the focused file, the
    /// copy items on the targets, one per line, as listed; a judgment runs
    /// as its key does.
    fn files_action(
        &mut self,
        action: context::Action,
        focused: &str,
        paths: &[String],
        cx: &mut Context<Self>,
    ) {
        match action {
            context::Action::SelectAll => self.select_all(cx),
            context::Action::RevealFile => cx.reveal_path(Path::new(&context::shown_path(focused))),
            context::Action::CopyPath => {
                cx.write_to_clipboard(ClipboardItem::new_string(paths.join("\n")));
            }
            context::Action::CopyFileName => {
                let names: Vec<String> = paths.iter().map(|p| file_name(Path::new(p))).collect();
                cx.write_to_clipboard(ClipboardItem::new_string(names.join("\n")));
            }
            context::Action::Judge(action) => {
                if let Some(cmd) = Command::from_action(action) {
                    self.judge(cmd, cx);
                }
            }
            context::Action::TrashFiles => self.confirm_trash(paths, trash::Mode::Trash, cx),
            context::Action::DeleteFiles => self.confirm_trash(paths, trash::Mode::Delete, cx),
            _ => {}
        }
    }

    /// Ask whether to move `paths`, files of the open folder taken when the
    /// menu opened, to the Trash, or to delete them. Refused while a run is
    /// in flight or the dialog is open.
    fn confirm_trash(&mut self, paths: &[String], mode: trash::Mode, cx: &mut Context<Self>) {
        let Some(dir) = self.dir.clone() else { return };
        if self.trashing.is_some()
            || paths.is_empty()
            || self.settings_view.read(cx).is_open()
            || self.folderops.busy()
            || !self.trash.start(trash::Target {
                dir,
                paths: paths.to_vec(),
                mode,
            })
        {
            return;
        }
        self.menus.close();
        self.follow_trash(cx);
        cx.notify();
    }

    /// The dialog's run button, by a click with `click_count` (0 for a key):
    /// a delete takes a single mouse click only. Through the gate: at once
    /// with no listing or scan under way, the dialog staying open until the
    /// run comes back; else held for the scan's end, the dialog closed and
    /// the scan canceled, as its files are about to go.
    fn confirm_run(&mut self, click_count: usize, cx: &mut Context<Self>) {
        if self
            .trash
            .target()
            .is_some_and(|t| t.mode == trash::Mode::Delete)
            && !trash::confirms_delete(click_count)
        {
            return;
        }
        let Some(target) = self.trash.run() else {
            return;
        };
        let busy = self.resync.busy() || self.gate.in_flight();
        let label = target.mode.label();
        match self.gate_request(label, HeldOp::Run(target), busy) {
            Some(HeldOp::Run(target)) => self.run_trash(target, true, cx),
            Some(_) => {}
            None => {
                log::info!("trash: {label} held for the scan's end");
                self.trash.end();
                self.follow_trash(cx);
                self.scan.cancel();
                // The canceled scan leaves files unscanned, whatever becomes
                // of the held run.
                if self.resync.changed() {
                    self.relist(cx);
                }
            }
        }
        cx.notify();
    }

    fn dismiss_trash(&mut self, cx: &mut Context<Self>) {
        if self.trash.dismiss() {
            self.follow_trash(cx);
            cx.notify();
        }
    }

    /// A key pressed in the dialog; true when it goes on to the focused
    /// button, which presses itself on Enter or Space. Every other key
    /// stops in the dialog.
    fn trash_key_down(&mut self, name: Option<&str>, held: bool, cx: &mut Context<Self>) -> bool {
        match self.trash.key(name, held) {
            trash::Key::Dismiss => {
                self.dismiss_trash(cx);
                false
            }
            trash::Key::Press => true,
            trash::Key::Swallow => false,
        }
    }

    /// Open the dialog when the flow starts asking and close it when the
    /// flow goes idle, the only places it opens or closes.
    fn follow_trash(&mut self, cx: &mut Context<Self>) {
        if !self.trash.busy() {
            self.trash_modal.close(cx);
            return;
        }
        let Some(initial) = self.trash.initial_focus() else {
            return;
        };
        let this = cx.weak_entity();
        let buttons = self.trash_buttons.clone();
        let first = Cell::new(true);
        self.trash_modal.open(cx, move |dialog, window, cx| {
            if first.replace(false) {
                let at = match initial {
                    trash::Button::Run => 0,
                    trash::Button::Cancel => 1,
                };
                buttons[at].focus(window, cx);
            }
            trash_dialog(dialog, &this, &buttons, cx)
        });
    }

    /// Move `target`'s files to the Trash, or delete them, on a background
    /// task: the judgment stores still queued, then the sidecar writer, are
    /// flushed first, so no write lands on a file being moved. No listing
    /// or scan starts until it comes back (`gate`, `resync`). `own` when the
    /// dialog is still open for it, to close at the end.
    fn run_trash(&mut self, target: trash::Target, own: bool, cx: &mut Context<Self>) {
        let trash::Target { dir, paths, mode } = target;
        self.gate.enter_in_flight();
        self.resync.hold();
        self.trashing = Some((mode, 0, paths.len()));
        let (stores, writer, runs, events) = (
            self.stores.clone(),
            self.writer.clone(),
            self.runs.clone(),
            self.events.clone(),
        );
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let ran = dir.clone();
            let result = cx
                .background_spawn(async move {
                    stores.drain(sidecar::DRAIN_TIMEOUT);
                    if let Some(writer) = &writer {
                        writer.flush(sidecar::DRAIN_TIMEOUT);
                    }
                    let progress = |done, total| {
                        let _ = events.unbounded_send(Event::TrashProgress { mode, done, total });
                    };
                    match mode {
                        trash::Mode::Trash => {
                            riffle_backend::trash::trash_listed(&ran, &paths, &runs, progress)
                        }
                        trash::Mode::Delete => {
                            riffle_backend::trash::delete_listed(&ran, &paths, progress)
                        }
                    }
                })
                .await;
            log::info!(
                "trash: {} in {dir} in {}ms",
                mode.label(),
                started.elapsed().as_millis()
            );
            this.update(cx, |this, cx| {
                this.trashed(vec![dir], mode, trash::What::Files, own, false, result, cx);
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// A run, or a redo (`redo`), came back: report it, push a run that
    /// moved something on the undo history, drop what a delete took from
    /// the histories, and re-list the open folder, which also rescans what
    /// a canceled scan left, or open the folder asked for meanwhile.
    #[expect(
        clippy::too_many_arguments,
        reason = "the Tauri `trashed`'s parameters"
    )]
    fn trashed(
        &mut self,
        dirs: Vec<String>,
        mode: trash::Mode,
        what: trash::What,
        own: bool,
        redo: bool,
        result: Result<riffle_backend::trash::Summary, String>,
        cx: &mut Context<Self>,
    ) {
        self.gate.leave_in_flight();
        self.trashing = None;
        if own {
            self.trash.end();
            self.follow_trash(cx);
        }
        match result {
            Ok(summary) => {
                log::info!(
                    "trash: moved={} failed={} run_id={:?}",
                    summary.moved.len(),
                    summary.failed.len(),
                    summary.run_id
                );
                for path in &summary.moved {
                    self.touched.remove(path);
                    if mode == trash::Mode::Trash {
                        let state = self.judgments.get(path).cloned();
                        let seen = self.seen.contains(path);
                        if state.is_some() || seen {
                            self.trashed_state.insert(path.clone(), (state, seen));
                        }
                    }
                }
                if mode == trash::Mode::Delete && !summary.moved.is_empty() {
                    self.undo
                        .without_paths(&summary.moved.iter().cloned().collect());
                }
                if what == trash::What::Sidecars {
                    self.sidecars_trashed(&dirs, &summary);
                }
                if let Some(run_id) = summary.run_id {
                    let count = summary.moved.len();
                    if redo {
                        self.undo.redone_trash(run_id, count, what, dirs);
                    } else {
                        self.undo.record_trash(run_id, count, what, dirs);
                    }
                }
                for failure in &summary.failed {
                    self.write_failed(
                        failure.path.clone(),
                        format!("{}: {}", mode.failure(), failure.message),
                    );
                }
                self.step_note = Some(trash::trashed_status(mode, what, &summary));
            }
            Err(e) => {
                log::error!("trash: {} in {dirs:?} failed: {e}", mode.label());
                self.step_note = Some(e);
            }
        }
        if let Some(folder) = self.deferred_open.take() {
            self.open(folder, None, None, cx).detach();
        } else if self.dir.is_some() {
            // The re-listing covers any change seen during the run.
            self.resync.open();
            self.relist(cx);
        }
        cx.notify();
    }

    /// An undo or a redo with the run `entry` on top: through the gate,
    /// waiting for a listing or scan under way without canceling it.
    fn step_trash(&mut self, entry: TrashEntry, redo: bool, cx: &mut Context<Self>) {
        let (undo, redo_label) = entry.what.step_labels();
        let (label, held) = if redo {
            (redo_label, HeldOp::Redo(entry))
        } else {
            (undo, HeldOp::Undo(entry))
        };
        let busy = self.resync.busy() || self.gate.in_flight();
        match self.gate_request(label, held, busy) {
            Some(HeldOp::Undo(entry)) => {
                self.undo_trash(entry, cx);
            }
            Some(HeldOp::Redo(entry)) => {
                self.redo_trash(entry, cx);
            }
            Some(_) => {}
            None => log::info!("trash: {label} held for the scan's end"),
        }
        cx.notify();
    }

    /// Restore the run `entry` from the Trash on a background task; it stays
    /// on the history until the backend answers. False when another step is
    /// out or the entry is no longer on top, as nothing then runs.
    fn undo_trash(&mut self, entry: TrashEntry, cx: &mut Context<Self>) -> bool {
        if self.trash_step.is_some() || !self.undo.on_top(entry.id, false) {
            return false;
        }
        self.trash_step = Some(entry.id);
        self.gate.enter_in_flight();
        self.resync.hold();
        let runs = self.runs.clone();
        let run_id = entry.run_id;
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { riffle_backend::trash::undo_run(&runs, run_id) })
                .await;
            log::info!(
                "trash: undo of run {run_id} in {}ms: {:?}",
                started.elapsed().as_millis(),
                result.as_ref().map(|r| (r.restored.len(), r.failed.len()))
            );
            this.update(cx, |this, cx| this.restored(entry, result, cx))
                .ok();
        })
        .detach();
        true
    }

    /// An undo came back: the entry moves to the redo history when anything
    /// came back, a refusal other than the run being gone leaves it
    /// undoable, and when the open folder was among the run's folders the
    /// restored files join `all` at once, the current file and the
    /// selection kept, before the re-listing brings their judgments.
    fn restored(
        &mut self,
        entry: TrashEntry,
        result: Result<riffle_backend::trash::Restored, String>,
        cx: &mut Context<Self>,
    ) {
        self.trash_step = None;
        self.gate.leave_in_flight();
        let mut refresh = false;
        match result {
            Ok(restored) => {
                self.undo.undone(&entry, &restored);
                for failure in &restored.failed {
                    self.write_failed(
                        failure.path.clone(),
                        format!("could not restore from the Trash: {}", failure.message),
                    );
                }
                self.step_note = Some(trash::restored_status(&restored, entry.what, entry.count));
                let opens = self
                    .dir
                    .as_deref()
                    .is_some_and(|dir| trash::opens_target(dir, &entry.dirs, IGNORE_CASE));
                if !opens {
                    for path in &restored.restored {
                        self.trashed_state.remove(path);
                    }
                }
                if let Some(dir) = self.dir.clone()
                    && trash::opens_target(&dir, &entry.dirs, IGNORE_CASE)
                {
                    refresh = true;
                    let back =
                        trash::restored_into(&dir, &restored.restored, &self.all, IGNORE_CASE);
                    if !back.is_empty() {
                        self.all.extend(back);
                        trash::carry_stash(
                            &mut self.trashed_state,
                            &self.all,
                            &mut self.judgments,
                            &mut self.seen,
                        );
                        self.view_only = viewonly::is_view_only(&self.all);
                        self.refilter(self.current_path(), true, None, cx);
                        self.mark_strip();
                    }
                }
            }
            Err(e) => {
                log::error!("trash: undo of run {} failed: {e}", entry.run_id);
                if !trash::still_held(&e) {
                    self.undo.remove_trash(entry.id);
                }
                self.step_note = Some(e);
            }
        }
        if let Some(folder) = self.deferred_open.take() {
            self.open(folder, None, None, cx).detach();
        } else if refresh {
            self.resync.open();
            self.relist(cx);
        } else if self.resync.finished() {
            // A change seen in the open folder during the restore.
            self.resync.open();
            self.relist(cx);
        }
        cx.notify();
    }

    /// Move what the undo of `entry` brought back to the Trash again, with
    /// no dialog, as a new run; it stays on the redo history until the
    /// backend answers. False when another step or a run is out or the entry
    /// is no longer on top, as nothing then runs.
    fn redo_trash(&mut self, entry: TrashEntry, cx: &mut Context<Self>) -> bool {
        if self.trash_step.is_some() || self.trashing.is_some() || !self.undo.on_top(entry.id, true)
        {
            return false;
        }
        self.trash_step = Some(entry.id);
        self.gate.enter_in_flight();
        self.resync.hold();
        self.trashing = Some((trash::Mode::Trash, 0, entry.count));
        let (stores, writer, runs, events) = (
            self.stores.clone(),
            self.writer.clone(),
            self.runs.clone(),
            self.events.clone(),
        );
        let run_id = entry.run_id;
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    stores.drain(sidecar::DRAIN_TIMEOUT);
                    if let Some(writer) = &writer {
                        writer.flush(sidecar::DRAIN_TIMEOUT);
                    }
                    riffle_backend::trash::redo_run(&runs, run_id, |done, total| {
                        let _ = events.unbounded_send(Event::TrashProgress {
                            mode: trash::Mode::Trash,
                            done,
                            total,
                        });
                    })
                })
                .await;
            log::info!(
                "trash: redo of run {run_id} in {}ms",
                started.elapsed().as_millis()
            );
            this.update(cx, |this, cx| {
                this.trash_step = None;
                match &result {
                    Ok(_) => this.undo.remove_trash(entry.id),
                    Err(e) if !trash::still_held(e) => this.undo.remove_trash(entry.id),
                    Err(_) => {}
                }
                this.trashed(
                    entry.dirs,
                    trash::Mode::Trash,
                    entry.what,
                    false,
                    true,
                    result,
                    cx,
                );
            })
            .ok();
        })
        .detach();
        cx.notify();
        true
    }

    /// An item of the folder tree's menu.
    fn folder_action(
        &mut self,
        action: context::Action,
        path: String,
        name: String,
        cx: &mut Context<Self>,
    ) {
        match action {
            context::Action::RevealFolder => {
                let path = Path::new(&path);
                // A root has no parent folder to select it in, so it opens.
                if path.parent().is_some() {
                    cx.reveal_path(path);
                } else {
                    cx.open_with_system(path);
                }
            }
            context::Action::CopyPath => cx.write_to_clipboard(ClipboardItem::new_string(path)),
            context::Action::CopyFolderName => {
                cx.write_to_clipboard(ClipboardItem::new_string(name));
            }
            context::Action::RefreshFolder => self.tree_refresh(path, cx),
            context::Action::ExpandAll => self.tree_expand_all(path, cx),
            context::Action::CollapseAll => {
                self.tree.collapse_all(&path);
                self.tree_changed(cx);
            }
            action => self.folder_op(action, path, cx),
        }
    }

    /// The menu's `Expand All`: expand `path` and every subfolder under it,
    /// listing each as an expander click does, one listing at a time, as
    /// `folders.ts`'s `expandAll`. A branch stops once a folder on its chain
    /// was collapsed or dropped meanwhile; a folder that fails to list is
    /// marked and reported and the rest go on; a link is not descended into,
    /// since it can loop back on an ancestor.
    fn tree_expand_all(&mut self, path: String, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let mut chains = vec![vec![path]];
            while let Some(chain) = chains.pop() {
                let Some(dir) = chain.last().cloned() else {
                    continue;
                };
                let walking = this.update(cx, |this, cx| {
                    let parents = &chain[..chain.len() - 1];
                    if !parents.iter().all(|p| this.tree.is_expanded(p)) || !this.tree.has(&dir) {
                        return false;
                    }
                    this.tree.expand(&dir);
                    this.tree_changed(cx);
                    true
                });
                match walking {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(_) => return,
                }
                let listing = dir.clone();
                let listed = cx
                    .background_spawn(async move { folders::list(Path::new(&listing)) })
                    .await;
                let Ok(children) = this.update(cx, |this, cx| {
                    let listed = this.tree_listed(&dir, listed, true).is_some();
                    this.tree_changed(cx);
                    if listed {
                        this.tree.drawn_children_of(&dir)
                    } else {
                        Vec::new()
                    }
                }) else {
                    return;
                };
                for child in children.into_iter().rev().filter(|child| !child.is_link) {
                    let mut next = chain.clone();
                    next.push(child.path);
                    chains.push(next);
                }
            }
        })
        .detach();
    }

    /// Give the tree the keyboard, or with `false` hand it back to the photo;
    /// the type-ahead starts over either way.
    fn set_tree_keys(&mut self, keys: bool, cx: &mut Context<Self>) {
        if self.tree_keys != keys {
            self.tree_keys = keys;
            self.typed = Typed::default();
            cx.notify();
        }
    }

    fn move_tree_cursor(&mut self, to: String, cx: &mut Context<Self>) {
        self.tree_cursor_row = self.tree_rows.iter().position(|row| row.path == to);
        self.tree_cursor = Some(to);
        if let Some(row) = self.tree_cursor_row {
            self.tree_scroll
                .scroll_to_item(row, ScrollStrategy::Nearest);
        }
        cx.notify();
    }

    /// A key while the tree has the keyboard, as `folders.ts`'s `keydown`:
    /// `escape` hands the keys back, the arrows and `home` / `end` move the
    /// cursor, left / right / `enter` act on its row, and a printable key
    /// without Ctrl / Alt / Meta is type-ahead. False when the tree did not
    /// consume the key.
    fn tree_key_down(&mut self, keystroke: &Keystroke, name: &str, cx: &mut Context<Self>) -> bool {
        let step = match name {
            "arrowup" => Some(StepKey::Up),
            "arrowdown" => Some(StepKey::Down),
            "home" => Some(StepKey::Home),
            "end" => Some(StepKey::End),
            _ => None,
        };
        let key = match name {
            "arrowleft" => Some(TreeKey::Left),
            "arrowright" => Some(TreeKey::Right),
            "enter" => Some(TreeKey::Enter),
            _ => None,
        };
        if name == "escape" {
            self.set_tree_keys(false, cx);
        } else if let Some(step) = step {
            if let Some(to) = tree::step(&self.tree_rows, self.tree_cursor.as_deref(), step) {
                self.move_tree_cursor(to, cx);
            }
        } else if let Some(key) = key {
            match tree::tree_key(&self.tree_rows, self.tree_cursor.as_deref(), key) {
                Some(TreeCommand::Focus(path)) => self.move_tree_cursor(path, cx),
                Some(TreeCommand::Expand(path) | TreeCommand::Collapse(path)) => {
                    self.tree_toggle(path, cx);
                }
                Some(TreeCommand::Open(path)) => self.open_from_tree(path, cx).detach(),
                None => {}
            }
        } else if let Some(typed) = keys::typed_char(keystroke) {
            self.typed = tree::append_typed(
                &self.typed,
                &typed,
                Instant::now(),
                tree::TYPE_AHEAD_TIMEOUT,
            );
            if let Some(to) = tree::type_ahead(
                &self.tree_rows,
                self.tree_cursor.as_deref(),
                &self.typed.text,
            ) {
                self.move_tree_cursor(to, cx);
            }
        } else {
            return false;
        }
        true
    }

    /// A listing of the tree's `dir` answered: its children, or `None` with
    /// the folder collapsed and marked, and the error in the notes when
    /// `report`.
    fn tree_listed(
        &mut self,
        dir: &str,
        listed: Result<Folder, String>,
        report: bool,
    ) -> Option<Vec<Child>> {
        match listed {
            Ok(folder) => {
                let children: Vec<Child> = folder.children.into_iter().map(Child::from).collect();
                self.tree
                    .set_children(dir, folder.raw_count, children.clone());
                Some(children)
            }
            Err(e) => {
                log::warn!("failed to list the subfolders of {dir}: {e}");
                self.tree.collapse(dir);
                self.tree.mark_failed(dir, e.clone());
                if report {
                    self.tree_note = Some(e);
                }
                None
            }
        }
    }

    /// An expander click: collapse, or expand and list the folder again, its
    /// cached children drawn meanwhile.
    fn tree_toggle(&mut self, dir: String, cx: &mut Context<Self>) {
        if self
            .tree_rows
            .iter()
            .any(|row| row.path == dir && row.expanded)
        {
            self.tree.collapse(&dir);
            self.tree_changed(cx);
            return;
        }
        self.tree.expand(&dir);
        self.tree_changed(cx);
        cx.spawn(async move |this, cx| {
            let listing = dir.clone();
            let listed = cx
                .background_spawn(async move { folders::list(Path::new(&listing)) })
                .await;
            this.update(cx, |this, cx| {
                this.tree_listed(&dir, listed, true);
                this.tree_changed(cx);
            })
            .ok();
        })
        .detach();
    }

    /// Expand and list the chain from the root holding `path` (a root added
    /// for a folder none holds) down to it, respelling the chain from the
    /// listings, then highlight it and scroll it into view, as `folders.ts`'s
    /// `reveal`. A later open owns the tree: this one stops at its next step.
    fn reveal(&mut self, path: String, cx: &mut Context<Self>) {
        self.tree_open = Some(path.clone());
        self.tree_changed(cx);
        if !self.roots_loaded {
            self.reveal_wanted = Some(path);
            return;
        }
        let chain = match tree::ancestors_within(self.tree.roots(), &path, IGNORE_CASE) {
            Some(chain) => Some(chain),
            None => {
                let root = tree::root_of(&path);
                self.tree.add_roots([Child {
                    name: root.clone(),
                    path: root.clone(),
                    is_link: false,
                }]);
                self.tree_changed(cx);
                tree::ancestors_within([root.as_str()], &path, IGNORE_CASE)
            }
        };
        let Some(mut chain) = chain else { return };
        let token = self.opens();
        cx.spawn(async move |this, cx| {
            for at in 0..chain.len() {
                let dir = chain[at].clone();
                let walking = this.update(cx, |this, cx| {
                    if this.opens() != token || !this.tree.has(&dir) {
                        return false;
                    }
                    this.tree.expand(&dir);
                    this.tree_changed(cx);
                    true
                });
                if !walking.unwrap_or(false) {
                    break;
                }
                let listing = dir.clone();
                let listed = cx
                    .background_spawn(async move { folders::list(Path::new(&listing)) })
                    .await;
                let children = this.update(cx, |this, cx| {
                    let current = this.opens() == token;
                    if !current && listed.is_ok() {
                        return None;
                    }
                    let children = this.tree_listed(&dir, listed, current);
                    this.tree_changed(cx);
                    children.filter(|_| current)
                });
                let Ok(Some(children)) = children else { break };
                chain = tree::respell(chain, at + 1, &children, IGNORE_CASE);
            }
            this.update(cx, |this, cx| {
                if this.opens() != token {
                    return;
                }
                // The chain's end is spelled the way the tree keys its nodes,
                // which the caller's path may not be.
                if let Some(end) = chain.last() {
                    this.tree_open = Some(end.clone());
                }
                this.tree_cursor = this.tree_open.clone();
                this.tree_changed(cx);
                if let Some(row) = this.tree_open_row {
                    this.tree_scroll
                        .scroll_to_item(row, ScrollStrategy::Nearest);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The watcher saw the open folder change: re-list it now, or once the
    /// listing or scan under way is over.
    fn changed(&mut self, dir: &str, cx: &mut Context<Self>) {
        if self.dir.as_ref().is_some_and(|open| open != dir) {
            return;
        }
        if self.resync.changed() {
            self.relist(cx);
        } else {
            log::info!("resync: dir={dir} deferred");
        }
    }

    /// The listing and its scan are over: run the re-listing that waited.
    fn finished(&mut self, cx: &mut Context<Self>) {
        // The run held for this scan runs now, the re-listing at its end
        // taking the change that waited; a dialog opened meanwhile drops it.
        match self.gate.drain() {
            Some(HeldOp::Run(target)) if !self.trash.busy() => {
                return self.run_trash(target, false, cx);
            }
            Some(HeldOp::Run(target)) => log::info!(
                "trash: held run in {} dropped by an open dialog",
                target.dir
            ),
            Some(HeldOp::Undo(entry)) => {
                if self.undo_trash(entry, cx) {
                    return;
                }
            }
            Some(HeldOp::Redo(entry)) => {
                if self.redo_trash(entry, cx) {
                    return;
                }
            }
            Some(HeldOp::Sidecars(target)) => self.start_sidecars(target, cx),
            Some(HeldOp::Rename(held)) => {
                if self.run_rename(held, cx) {
                    return;
                }
            }
            None => {}
        }
        if self.resync.finished() {
            self.relist(cx);
        }
    }

    fn relist(&mut self, cx: &mut Context<Self>) {
        let Some(dir) = self.dir.clone() else { return };
        // A run's end re-lists.
        if self.gate.in_flight() {
            return;
        }
        log::info!("resync: dir={dir}");
        self.list(PathBuf::from(dir), true, cx).detach();
    }

    /// The settings' sidecar format switch: the next judgment is queued in
    /// `format` at once; off the main thread, one switch at a time, the
    /// judgments queued before it reach the writer and `sidecar::switch_to`
    /// drains, persists and resets the index; then the open folder is listed
    /// again, its histories cleared as the Tauri reopen clears them.
    fn switch_format(&mut self, format: SidecarFormat, cx: &mut Context<Self>) {
        if format == self.settings.format {
            return;
        }
        log::info!(
            "sidecar format: {} -> {}",
            self.settings.format.setting(),
            format.setting()
        );
        self.settings.format = format;
        *index::lock(&self.format_latest) = format;
        let (index, writer) = (self.index.clone().ok(), self.writer.clone());
        let (stores, writes) = (self.stores.clone(), self.settings.writes.clone());
        let latest = self.format_latest.clone();
        self.reset_sidecars(
            move || {
                if *index::lock(&latest) != format {
                    // A later click superseded this one; its task
                    // switches to the format the user ended on.
                    return Ok(false);
                }
                stores.drain(sidecar::DRAIN_TIMEOUT);
                sidecar::switch_to(format, writer.as_deref(), index.as_ref(), |format| {
                    writes.set("sidecarFormat", format.setting().into());
                    Ok(())
                })
                .map(|()| true)
            },
            cx,
        );
    }

    /// The settings' label names change: written and in force at once, so a
    /// judgment queued before it keeps the names it was made under and the
    /// next one takes the new ones; then the rest of a format switch, with
    /// nothing left to persist (the writes are ordered on this thread, so
    /// overlapping changes end stored as the fields show).
    fn switch_names(&mut self, names: LabelNames, cx: &mut Context<Self>) {
        if names == self.settings.names {
            return;
        }
        log::info!("label names: {:?} -> {names:?}", self.settings.names);
        self.settings
            .writes
            .set("labelNames", sidecar::label_names_value(&names));
        self.settings.names = names;
        let (index, writer) = (self.index.clone().ok(), self.writer.clone());
        let (stores, format) = (self.stores.clone(), self.settings.format);
        self.reset_sidecars(
            move || {
                stores.drain(sidecar::DRAIN_TIMEOUT);
                sidecar::switch_to(format, writer.as_deref(), index.as_ref(), |_| Ok(()))
                    .map(|()| true)
            },
            cx,
        );
    }

    /// Runs `job` off the main thread, one at a time with the other sidecar
    /// switches; when it did its switch (`Ok(true)`) the open folder is
    /// listed again, its histories cleared as the Tauri reopen clears them.
    /// A failure goes to the settings' status line, with no re-listing.
    fn reset_sidecars(
        &mut self,
        job: impl FnOnce() -> Result<bool, String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let lock = self.format_switch.clone();
        cx.spawn(async move |this, cx| {
            let switched = cx
                .background_spawn(async move {
                    let _guard = index::lock(&lock);
                    job()
                })
                .await;
            this.update(cx, |this, cx| {
                let switched = match switched {
                    Ok(true) => Ok(()),
                    Ok(false) => return,
                    Err(e) => Err(e),
                };
                if let Err(e) = switched {
                    log::error!("failed to switch the sidecars: {e}");
                    this.settings_view.update(cx, |view, cx| view.failed(e, cx));
                    return;
                }
                this.touched.clear();
                this.undo.drop_judgments();
                this.step_note = None;
                // Mark the listing busy so a trash or delete waits for it.
                this.resync.open();
                this.relist(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Watch and list the folder and, with an index, reconcile it once the
    /// previous scan and the launch's eviction are over, then start pass 1.
    /// `carry` re-lists the open folder, keeping the view on the same files.
    fn list(&mut self, folder: PathBuf, carry: bool, cx: &mut Context<Self>) -> Task<()> {
        let (wanted, max) = (self.wanted, self.max);
        let index = self.index.clone().ok();
        let reader = self.reader.clone();
        let previous = self.scan.take();
        self.scan_status.clear();
        let eviction = self.eviction.take();
        let earlier = self.settled.take();
        let (settle, settled) = futures::channel::oneshot::channel::<()>();
        self.settled = Some(settled);
        let watching = self.watch.clone();
        let writer = self.writer.clone();
        let (format, names) = (self.settings.format, self.settings.names.clone());
        let (opens, watching_lock, token) =
            (self.opens.clone(), self.watching_lock.clone(), self.opens());
        cx.spawn(async move |this, cx| {
            let opened = cx
                .background_spawn(async move {
                    // An earlier listing may still be stopping the previous
                    // scan or joining the eviction; this one reconciles only
                    // after it (a dropped sender resolves it too).
                    if let Some(earlier) = earlier {
                        let _ = earlier.await;
                    }
                    if let Some(previous) = previous {
                        previous.stop();
                    }
                    if let Some(eviction) = eviction {
                        let _ = eviction.join();
                    }
                    let _ = settle.send(());
                    // Watched before the listing, so a change during it is
                    // not missed; under `watching_lock`, so a later open's
                    // watcher is never replaced by this one's.
                    let dir = canonicalize(&folder.to_string_lossy());
                    {
                        let _guard = index::lock(&watching_lock);
                        if opens.load(Ordering::SeqCst) != token {
                            return Ok(None);
                        }
                        watch::set(&watching, &dir, &dir);
                    }
                    let listed = scan::list(Path::new(&dir), format)?;
                    let outputs = scan::outputs(&listed);
                    let todo = index.map(|index| {
                        Scan::prepare(&index, writer.as_deref(), &listed, format, &names)
                    });
                    let entries = reader
                        .as_ref()
                        .and_then(|reader| entries_of(reader, &listed.dir));
                    let (last_viewed, viewed) = match reader {
                        Some(reader) if !carry => remembered(&reader, &listed.dir),
                        _ => Default::default(),
                    };
                    let shown = shown(&listed.files, wanted, max);
                    Ok::<_, String>(Some(Opened {
                        listed,
                        shown,
                        outputs,
                        todo,
                        entries,
                        last_viewed,
                        viewed,
                    }))
                })
                .await;
            this.update(cx, |this, cx| {
                // A later open owns the view.
                if this.opens() != token {
                    return;
                }
                match opened {
                    Ok(Some(opened)) => this.opened(Ok(opened), carry, cx),
                    Ok(None) => {}
                    Err(e) => this.opened(Err(e), carry, cx),
                }
            })
            .ok();
        })
    }

    fn opened(&mut self, opened: Result<Opened, String>, carry: bool, cx: &mut Context<Self>) {
        let Opened {
            listed,
            shown,
            outputs,
            todo,
            entries,
            last_viewed,
            viewed,
        } = match opened {
            Ok(opened) => opened,
            Err(e) => {
                self.fail(&e);
                self.finished(cx);
                cx.notify();
                return;
            }
        };
        self.listing += 1;
        self.pending_faces.clear();
        self.order += 1;
        self.view_only = viewonly::is_view_only(&listed.files);
        if self.view_only && self.menus.open() == Some(Which::Sort) {
            self.menus.close();
        }
        self.marks.clear();
        self.clear_ring(cx);
        self.dir = Some(listed.dir.clone());
        let folder = context::shown_path(&listed.dir);
        if self.remember && self.settings.last_folder.as_ref() != Some(&folder) {
            self.settings
                .writes
                .set("lastFolder", folder.as_str().into());
            self.settings.last_folder = Some(folder);
        }
        self.filter.set_output_known(outputs.found.is_some());
        self.outputs = outputs.found;
        self.outputs_listing = outputs.listing;
        self.outputs_failure = outputs.failure;
        let entries = entries.unwrap_or_default();
        // A re-listing that leaves the same file current keeps its crop.
        let mut same_file = false;
        if carry {
            let was = self.current_path();
            let previous = path_strings(&self.files);
            self.all = shown;
            self.set_entries(entries);
            self.refilter(was.clone(), true, Some(&previous), cx);
            same_file = was.is_some() && was == self.current_path();
        } else {
            self.all = shown;
            self.failed = 0;
            self.key = None;
            self.heading = Heading::default();
            self.dwell.reset(viewed);
            self.dwell_timer = None;
            self.touched.clear();
            self.undo.drop_judgments();
            self.step_note = None;
            self.write_errors.clear();
            self.folderops.clear_imported();
            self.filter.clear_exif();
            self.kept_anchor.clear();
            self.set_entries(entries);
            let order = self.ordered();
            let paths = self.passing(&order);
            self.pager = Pager::new(paths.len());
            let resume = filter::anchor_after_filter(
                &order,
                |p| self.passes(p),
                last_viewed.as_deref(),
                None,
            );
            if let Some(at) = resume.and_then(|r| paths.iter().position(|p| *p == r)) {
                self.pager.go(Move::To(at));
            }
            self.strip = Strip::new(&paths);
            self.strip
                .scroll_into_view(self.pager.current(), self.strip_width);
            let old = self.thumbs.reset();
            self.retire(old);
            self.files = paths.iter().map(PathBuf::from).collect();
            self.mark_strip();
            let landed = selection::settle(&self.selection, &paths, self.pager.current(), true);
            self.set_selection(landed);
        }
        if carry {
            self.remember(cx);
        }
        self.strip_moved(cx);
        if self.files.is_empty() {
            self.status = if self.all.is_empty() {
                format!("no RAW files in {}", listed.dir).into()
            } else {
                filter::FILTERED_TEXT.into()
            };
            self.clear_image();
            self.shown = None;
            self.marks.show(None);
        } else {
            self.request(Instant::now(), cx);
            self.read_focus(cx);
            self.analyze_current(cx);
        }
        self.notes.clear();
        if !same_file {
            self.zoom_page(cx);
        }
        match (todo, &self.index) {
            (
                Some(Ok(Prepared {
                    todo,
                    sidecar_errors,
                })),
                Ok(index),
            ) => {
                self.notes = sidecar_errors
                    .iter()
                    .map(|e| format!("{}: {}", file_name(Path::new(&e.path)), e.message))
                    .collect();
                let id = self.next_scan;
                self.next_scan += 1;
                self.scan_status.started(todo.len());
                self.failed = 0;
                // Set before the workers start, so their first files are
                // already the ones on screen.
                let focus = ScanFocus::with_slots(self.slots.clone());
                focus.set(self.scan_focus_paths());
                let scan = Scan::start(
                    id,
                    index.clone(),
                    listed.dir,
                    todo,
                    focus,
                    self.events.clone(),
                );
                self.scan.put(scan);
                // A run held while the listing ran had no scan to cancel yet.
                if idle::cancels_scan(self.gate.waiting()) {
                    self.scan.cancel();
                }
            }
            (Some(Err(e)), _) => {
                log::error!("failed to prepare the scan of {}: {e}", listed.dir);
                self.finished(cx);
            }
            _ => self.finished(cx),
        }
        cx.notify();
    }

    /// `all` in the sort in force.
    fn ordered(&self) -> Vec<String> {
        let key = viewonly::sort_for(self.view_only, self.sort);
        sort::order_files(key, &self.all, |path| {
            let facts = self.facts.get(path);
            SortFacts {
                capture_time: facts.and_then(|f| f.capture_time.clone()),
                subsec: facts.and_then(|f| f.subsec.clone()),
                rating: self.judgments.get(path).and_then(|s| match s.flag {
                    Flag::Reject => Some(-1),
                    _ => s.rating.map(|r| r as i8),
                }),
            }
        })
    }

    /// Whether the filter lets the listed `path` through, with what the view
    /// has of it.
    fn passes(&self, path: &str) -> bool {
        let facts = self.facts.get(path);
        let focus = facts.and_then(|f| f.focus.as_ref());
        let unjudged = State::default();
        filter::passes(
            &self.filter,
            self.judgments.get(path).unwrap_or(&unjudged),
            facts,
            mark::af_eye_state(focus),
            focus.and_then(|f| f.eyes.eyes),
            filter::output_of(self.outputs.as_ref(), &self.outputs_listing, path),
        )
    }

    /// The paths of `order` the filter lets through.
    fn passing(&self, order: &[String]) -> Vec<String> {
        order.iter().filter(|p| self.passes(p)).cloned().collect()
    }

    /// Rebuild `files` from `all` in the order in force, narrowed by the
    /// filter, as the Tauri `refilter`, landing on `anchor_after_filter`'s
    /// file. `keep_scroll` keeps the same files in view; otherwise the
    /// current file is scrolled into view. `previous`, the list shown before
    /// a re-listing, rebuilds even an unchanged list, and its caller reads
    /// the current preview again itself. Whether the current file changed.
    fn refilter(
        &mut self,
        anchor: Option<String>,
        keep_scroll: bool,
        previous: Option<&[String]>,
        cx: &mut Context<Self>,
    ) -> bool {
        let order = self.ordered();
        let next = self.passing(&order);
        let old = path_strings(&self.files);
        if !filter::filter_list_changed(&old, &next, previous.is_some()) {
            return false;
        }
        let current =
            filter::anchor_after_filter(&order, |p| self.passes(p), anchor.as_deref(), previous)
                .and_then(|a| next.iter().position(|p| *p == a))
                .unwrap_or(0);
        let shown = self
            .shown
            .as_ref()
            .and_then(|(at, _)| old.get((*at)?).cloned());
        let moved = self.carry(&next, current, keep_scroll);
        self.order += 1;
        self.kept_anchor
            .refiltered(anchor.as_deref(), next.is_empty());
        if let Some((at, _)) = &mut self.shown {
            *at = filter::shown_after(&old, &next, *at);
        }
        if let Some((Some(to), _)) = self.shown {
            // The status line's `n/total` follows the shown file's new place.
            if let Some((head, rest)) = self.status.split_once("  ")
                && let Some((a, b)) = head.split_once('/')
                && a.parse::<usize>().is_ok()
                && b.parse::<usize>().is_ok()
            {
                self.status = format!("{}/{}  {rest}", to + 1, next.len()).into();
            }
        }
        self.files = next.iter().map(PathBuf::from).collect();
        if let Some(ring) = &mut self.prefetch {
            let given = ring.retain(&self.files);
            self.give_up(given, cx);
        }
        self.mark_strip();
        let settled = selection::settle(&self.selection, &next, current, false);
        self.set_selection(settled);
        self.strip_moved(cx);
        if next.is_empty() {
            self.status = filter::FILTERED_TEXT.into();
            self.clear_image();
            self.shown = None;
            self.marks.show(None);
        }
        if previous.is_none() {
            if moved {
                self.remember(cx);
            }
            if moved {
                // Before `request`: a ring hit stamps `zoom.shown` with this
                // page's `seq`.
                self.zoom_page(cx);
            }
            // The order's bump dropped a preview read still out.
            if moved || shown != self.current_path() {
                self.request(Instant::now(), cx);
            } else {
                // The current file may have new neighbours.
                self.request_prefetch(cx);
            }
            if moved {
                self.read_focus(cx);
            }
        }
        moved
    }

    /// Move the pager, the strip and the loaded thumbnails from `files` onto
    /// `paths` with `current` the current index: the strip keeps the same
    /// files in view with `keep_scroll`, else scrolls the current file into
    /// view, as it does when the current file changed. Whether it changed.
    fn carry(&mut self, paths: &[String], current: usize, keep_scroll: bool) -> bool {
        let previous = path_strings(&self.files);
        let was = self.pager.current();
        let offset =
            strip::carried_offset(&previous, paths, self.strip.offset(), self.strip_width, was);
        let loaded: Vec<usize> = self.thumbs.loaded().collect();
        let moves = strip::carried_indices(&previous, paths, loaded);
        let old = self.thumbs.carry(&moves);
        self.retire(old);
        self.pager = Pager::new(paths.len());
        self.pager.go(Move::To(current));
        self.strip = Strip::new(paths);
        self.strip.scroll_to(offset, self.strip_width);
        self.strip.keep(moves.into_values());
        let moved = previous.get(was) != paths.get(current);
        if moved {
            self.dwell.cancel();
            self.dwell_timer = None;
        }
        if moved || !keep_scroll {
            self.strip.scroll_into_view(current, self.strip_width);
        }
        moved
    }

    fn scanned(&mut self, event: Event, cx: &mut Context<Self>) {
        let scan = match &event {
            Event::Progress { scan, .. }
            | Event::Done { scan, .. }
            | Event::FacesProgress { scan, .. }
            | Event::FacesDone { scan, .. } => *scan,
            Event::Changed(dir) => return self.changed(dir, cx),
            Event::TreeChanged(dir) => return self.tree_folder_changed(dir.clone(), cx),
            Event::Mcp(request) => return self.mcp_request(request.clone(), cx),
            Event::FolderOp(message) => return self.folder_op_message(*message, cx),
            Event::TrashProgress { mode, done, total } => {
                // The last message can land after the run came back.
                if self.trashing.is_some() {
                    self.trashing = Some((*mode, *done, *total));
                    cx.notify();
                }
                return;
            }
            Event::Perf(sample) => {
                self.perf.add_sample(*sample);
                cx.notify();
                return;
            }
            Event::SidecarError { path, message } => {
                self.write_failed(path.clone(), message.clone());
                cx.notify();
                return;
            }
        };
        if !self.scan.is(scan) {
            return;
        }
        match event {
            Event::Progress {
                done, total, ready, ..
            } => {
                self.scan_status.extracting(done, total);
                self.strip.mark_ready(&ready);
                if self.current_path().is_some_and(|p| ready.contains(&p)) {
                    self.read_focus(cx);
                }
            }
            Event::Done { summary, .. } => {
                self.scan_status.extracted();
                self.failed = summary.errors;
                self.strip.refresh();
                self.read_entries(cx);
            }
            Event::FacesProgress {
                done, total, ready, ..
            } => {
                self.scan_status.analyzing(done, total);
                filter::apply_face_ready(&mut self.facts, &ready);
                self.pending_faces.extend(ready.iter().cloned());
                // The `Face focus` and `Eyes` sections fill in as the pass
                // runs, the strip keeping its scroll offset.
                if !(self.filter.candidates.is_empty() && self.filter.eyes.is_empty()) {
                    self.refilter(self.current_path(), true, None, cx);
                }
                if let Some(current) = self.current_path()
                    && ready.iter().any(|r| r.path == current)
                {
                    self.read_focus(cx);
                }
            }
            Event::FacesDone { .. } => {
                self.scan_status.clear();
                self.finished(cx);
                self.request_prefetch(cx);
            }
            Event::Changed(_)
            | Event::TreeChanged(_)
            | Event::SidecarError { .. }
            | Event::Mcp(_)
            | Event::FolderOp(_)
            | Event::TrashProgress { .. }
            | Event::Perf(_) => {}
        }
        self.pump(cx);
        cx.notify();
    }

    /// Keep the folder's judgments, the view's own for a file judged since
    /// the open, and its facts, and regroup the bursts over `all`.
    fn set_entries(&mut self, (mut judgments, facts, groups): Entries) {
        self.seen = judgments.keys().cloned().collect();
        for path in &self.touched {
            match self.judgments.get(path) {
                Some(state) => judgments.insert(path.clone(), state.clone()),
                None => judgments.remove(path),
            };
        }
        self.judgments = judgments;
        trash::carry_stash(
            &mut self.trashed_state,
            &self.all,
            &mut self.judgments,
            &mut self.seen,
        );
        self.facts = facts;
        self.bursts = burst::group_bursts(
            &self.all,
            |path| {
                let facts = self.facts.get(path);
                SortFacts {
                    capture_time: facts.and_then(|f| f.capture_time.clone()),
                    subsec: facts.and_then(|f| f.subsec.clone()),
                    rating: None,
                }
            },
            BURST_GAP_MS,
        );
        self.filter.prune_exif(&groups);
        self.exif_groups = groups;
    }

    /// Hand the shown files' judgments and burst marks to the strip.
    fn mark_strip(&mut self) {
        for (index, path) in self.files.iter().enumerate() {
            let state = self.judgments.get(path.to_string_lossy().as_ref()).cloned();
            self.strip.set_judgment(index, state);
        }
        let marks = burst::burst_marks(&path_strings(&self.files), &self.bursts);
        self.strip.set_bursts(marks);
    }

    /// Read the judgments and the facts again once pass 1 has written the
    /// files' rows, and reorder: the index returns a file only once pass 1
    /// has extracted it, so on a fresh index the open's read has nothing.
    fn read_entries(&mut self, cx: &mut Context<Self>) {
        let (Some(reader), Some(dir)) = (self.reader.clone(), self.dir.clone()) else {
            return;
        };
        let listing = self.listing;
        self.pending_faces.clear();
        cx.spawn(async move |this, cx| {
            let entries = cx
                .background_spawn(async move { entries_of(&reader, &dir) })
                .await;
            this.update(cx, |this, cx| {
                if let Some(entries) = entries
                    && listing == this.listing
                {
                    this.set_entries(entries);
                    // Pass 2 files written after the snapshot was read.
                    filter::apply_face_ready(&mut this.facts, &this.pending_faces);
                    this.refilter(this.current_path(), true, None, cx);
                    this.mark_strip();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Resolve a keystroke through the keymap. `home`, `end` and `q` are the
    /// native app's own keys, `ctrl+,` (`meta+,` on macOS) opens the settings,
    /// and `escape` while a menu is open; an action not handled yet is only
    /// logged. Nothing gets through while the settings are open. While the
    /// tree has the keyboard it takes the key first, and of the rest only
    /// `tree_gate`'s actions run.
    fn key_down(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings_view.read(cx).is_open() {
            return;
        }
        // The dialog in `Root`'s layer has the keys while one is open.
        if self.trash.busy() || self.folderops.dialog_open() {
            return;
        }
        let name = keys::key_name(keystroke);
        if self.rename_owns_keys() {
            return;
        }
        let Some(name) = name else {
            return;
        };
        let action = self.bindings.action_for(&name);
        log::debug!("key: {name} action={action:?}");
        // Any key closes the right-click menu; only `escape` stops there, as
        // in the Tauri app's keydown handler.
        if self.context.is_open() {
            self.context.close();
            cx.notify();
            if name == "escape" {
                return;
            }
        }
        if self.tree_keys
            && (self.tree_key_down(keystroke, &name, cx)
                || treekeys::tree_gate(action) == Gate::Swallow)
        {
            return;
        }
        match (name.as_str(), action) {
            ("home", _) => self.go(Move::First, cx),
            ("end", _) => self.go(Move::Last, cx),
            ("q", _) => self.quit(cx),
            // Not while a folder flow is under way, as in the Tauri app: the
            // dialog would open over the settings and the run would read a
            // format changed meanwhile.
            (name, _) if name == settingsmodal::OPEN_KEY && self.folderops.busy() => {}
            (name, _) if name == settingsmodal::OPEN_KEY => {
                self.menus.close();
                let settings = &self.settings;
                self.settings_view.update(cx, |view, cx| {
                    view.open(settings, window, cx);
                });
                cx.notify();
            }
            ("escape", _) if self.menus.is_open() => {
                self.menus.close();
                cx.notify();
            }
            (_, Some("previous")) => self.go(Move::Previous, cx),
            (_, Some("next")) => self.go(Move::Next, cx),
            (_, Some("burstPrevious")) => self.move_burst(burst::burst_step, -1, cx),
            (_, Some("burstNext")) => self.move_burst(burst::burst_step, 1, cx),
            (_, Some("burstFramePrevious")) => self.move_burst(burst::burst_frame_step, -1, cx),
            (_, Some("burstFrameNext")) => self.move_burst(burst::burst_frame_step, 1, cx),
            (_, Some("focus")) => self.toggle_marks(cx),
            (_, Some("zoom")) => self.toggle_zoom(cx),
            (_, Some("peaking")) => self.toggle_peaking(cx),
            (_, Some("performance")) => {
                let events = self.events.clone();
                self.perf
                    .toggle(move |sample| events.unbounded_send(Event::Perf(sample)), cx);
            }
            (_, Some("open")) => self.pick_folder(cx),
            (_, Some("toggleLeft")) => self.change_panels(self.panels.toggle(Panel::Left), cx),
            (_, Some("toggleRight")) => self.change_panels(self.panels.toggle(Panel::Right), cx),
            (_, Some("toggleStrip")) => self.change_panels(self.panels.toggle(Panel::Strip), cx),
            (_, Some("toggleSides")) => self.change_panels(self.panels.toggle_sides(), cx),
            (_, Some("extendPrevious")) => self.extend(-1, cx),
            (_, Some("extendNext")) => self.extend(1, cx),
            (_, Some("selectAll")) => self.select_all(cx),
            (_, Some("undo")) => self.step(false, cx),
            (_, Some("redo")) => self.step(true, cx),
            (_, Some("rejectRest")) => self.reject_rest(cx),
            (_, Some(action)) => match Command::from_action(action) {
                Some(cmd) => self.judge(cmd, cx),
                None => log::debug!("key: {action} is not handled yet"),
            },
            (_, None) => {}
        }
    }

    /// A judgment key on the selection's targets, its value decided from the
    /// current file: the view and the strip first, then the row and the
    /// writer per changed file in the background, all as one undo entry.
    /// Auto-advance moves on after the stars and the flags on a single
    /// target, also when the file already held the value.
    fn judge(&mut self, cmd: Command, cx: &mut Context<Self>) {
        if self.view_only {
            return;
        }
        let Some(path) = self.current_path() else {
            return;
        };
        self.step_note = None;
        let files = path_strings(&self.files);
        let targets = selection::targets(&self.selection, &files, self.pager.current());
        let changes = selection::judgments(
            &targets,
            &path,
            |p| self.judgments.get(p).cloned().unwrap_or_default(),
            cmd,
            |p| self.seen.contains(p),
        );
        if !changes.is_empty() {
            let entry = self.undo.record(
                changes
                    .iter()
                    .map(|change| Judged {
                        path: change.path.clone(),
                        state: change.before.clone(),
                    })
                    .collect(),
            );
            for change in &changes {
                self.touched.insert(change.path.clone());
                self.set_state(&change.path, change.after.clone());
            }
            self.refilter(Some(path.clone()), false, None, cx);
            for change in changes {
                let seen = self.seen.contains(&change.path);
                let label_known = judgment::label_known(seen, &change.before, &change.after, cmd);
                self.store(
                    change.path,
                    change.before,
                    change.after,
                    label_known,
                    entry,
                    cx,
                );
            }
        }
        let still_current = self.current_path() == Some(path);
        if judgment::should_advance(
            cmd,
            targets.len(),
            self.settings.auto_advance,
            still_current,
        ) {
            self.go(Move::NextClamped, cx);
        }
        cx.notify();
    }

    /// The MCP companion's `set_judgment`: `changes` as one undo entry,
    /// through the same view-row-writer path as a key, the current file
    /// staying current and no auto-advance.
    fn judge_paths(&mut self, changes: Vec<companion::Change>, cx: &mut Context<Self>) {
        if changes.is_empty() {
            return;
        }
        self.step_note = None;
        let entry = self.undo.record(
            changes
                .iter()
                .map(|change| Judged {
                    path: change.path.clone(),
                    state: change.before.clone(),
                })
                .collect(),
        );
        for change in &changes {
            self.touched.insert(change.path.clone());
            self.set_state(&change.path, change.after.clone());
        }
        self.refilter(self.current_path(), false, None, cx);
        for change in changes {
            self.store(
                change.path,
                change.before,
                change.after,
                change.label_known,
                entry,
                cx,
            );
        }
    }

    /// `rejectRest`: reject the other members of the current file's burst,
    /// over the whole listing, as one undo entry through `judge_paths`.
    fn reject_rest(&mut self, cx: &mut Context<Self>) {
        if self.view_only {
            return;
        }
        let Some(current) = self.current_path() else {
            return;
        };
        let changes = burst::reject_rest(
            &self.all,
            &current,
            &self.bursts,
            |p| self.judgments.get(p).cloned().unwrap_or_default(),
            &self.seen,
        );
        self.judge_paths(changes, cx);
        cx.notify();
    }

    /// Undo (or with `redo`, redo) the newest judgment through the same
    /// view-row-writer path as a key, moving to its file.
    /// A run to the Trash on top is restored or moved again instead
    /// (`step_trash`), and nothing steps while one is out.
    fn step(&mut self, redo: bool, cx: &mut Context<Self>) {
        if self.trash_step.is_some() {
            return;
        }
        if let Some(entry) = self.undo.top_trash(redo) {
            return self.step_trash(entry, redo, cx);
        }
        if self.view_only {
            return;
        }
        let (all, judgments) = (&self.all, &self.judgments);
        let listed = |p: &str| all.iter().any(|f| f == p);
        let current = |p: &str| judgments.get(p).cloned().unwrap_or_default();
        let stepped = if redo {
            self.undo.redo(listed, current)
        } else {
            self.undo.undo(listed, current)
        };
        let Some(stepped) = stepped else { return };
        let count = stepped.changes.len();
        let shown = self.current_path();
        for change in stepped.changes {
            let label_known =
                self.seen.contains(&change.path) || change.before.label != change.after.label;
            self.touched.insert(change.path.clone());
            self.set_state(&change.path, change.after.clone());
            self.store(
                change.path,
                change.before,
                change.after,
                label_known,
                stepped.id,
                cx,
            );
        }
        // A file the filter now hides leaves the current file where it is.
        let anchor = match &stepped.move_to {
            Some(path) if self.passes(path) => Some(path.clone()),
            _ => shown.clone(),
        };
        self.refilter(anchor, false, None, cx);
        let verb = if redo { "Redid" } else { "Undid" };
        let note = match stepped.move_to {
            Some(path) => {
                let restored = selection::restore(
                    &self.selection,
                    &path_strings(&self.files),
                    shown.as_deref(),
                    &path,
                );
                self.set_selection(restored);
                let name = file_name(Path::new(&path));
                match self.files.iter().position(|f| f.to_string_lossy() == path) {
                    Some(at) => {
                        self.go(Move::To(at), cx);
                        format!("{verb} {name}")
                    }
                    None => format!("{verb} {name} (hidden by the filter)"),
                }
            }
            None => format!("{verb} {count} files"),
        };
        self.step_note = Some(note);
        cx.notify();
    }

    fn set_state(&mut self, path: &str, state: State) {
        if let Some(index) = self.files.iter().position(|p| p.to_string_lossy() == path) {
            self.strip.set_judgment(index, Some(state.clone()));
        }
        self.judgments.insert(path.to_owned(), state);
    }

    /// `judgment::store` in the background; a failure puts `before` back
    /// and drops the file from undo entry `entry` while the folder is still
    /// open.
    fn store(
        &self,
        path: String,
        before: State,
        after: State,
        label_known: bool,
        entry: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(dir) = self.dir.clone() else { return };
        let submitted = match (self.index.clone(), self.writer.clone()) {
            (Ok(index), Some(writer)) => Ok(self.stores.submit(
                index,
                writer,
                dir.clone(),
                path.clone(),
                after,
                label_known,
                self.settings.format,
                self.settings.names.clone(),
            )),
            (Err(_), _) => Err("no index cache available".to_string()),
            (_, None) => Err("the sidecar writer is not running".to_string()),
        };
        cx.spawn(async move |this, cx| {
            let stored = match submitted {
                Ok(outcome) => outcome
                    .await
                    .unwrap_or_else(|_| Err("the judgment store stopped".to_string())),
                Err(e) => Err(e),
            };
            let Err(e) = stored else { return };
            log::error!("failed to store the judgment of {path}: {e}");
            this.update(cx, |this, cx| {
                if this.dir.as_deref() == Some(dir.as_str()) {
                    this.touched.remove(&path);
                    this.undo.forget(entry, &path);
                    this.set_state(&path, before);
                    this.refilter(this.current_path(), false, None, cx);
                    this.write_failed(path, e);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn write_failed(&mut self, path: String, message: String) {
        match self.write_errors.iter_mut().find(|(p, _)| *p == path) {
            Some(entry) => entry.1 = message,
            None => self.write_errors.push((path, message)),
        }
    }

    fn current_path(&self) -> Option<String> {
        self.files
            .get(self.pager.current())
            .map(|p| p.to_string_lossy().into_owned())
    }

    /// The current file and the strip's range, nearest first.
    fn scan_focus_paths(&self) -> Vec<String> {
        let range = self.strip.visible_range(self.strip_width);
        focus::scan_focus_paths(
            &self.files,
            self.pager.current(),
            *range.start(),
            *range.end(),
        )
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
    }

    /// Pass 1 or pass 2 is running.
    fn scanning(&self) -> bool {
        self.scan_status.scanning()
    }

    /// Hand the running scan the current file and the strip's range.
    fn focus_scan(&self) {
        if self.scanning() {
            self.scan.focus(self.scan_focus_paths());
        }
    }

    /// `focus_scan` once the current file and the strip's range have stayed
    /// put for `SCAN_FOCUS_DELAY`.
    fn focus_scan_soon(&mut self, cx: &mut Context<Self>) {
        if !self.scanning() {
            return;
        }
        self.scan_focus = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(focus::SCAN_FOCUS_DELAY)
                .await;
            this.update(cx, |this, _| this.focus_scan()).ok();
        }));
    }

    /// Move to `to` and, when the current file changed, select it alone, as
    /// a paging key leaves the selection.
    fn go(&mut self, to: Move, cx: &mut Context<Self>) {
        if self.focus_on(to, cx) {
            self.select_current();
        }
    }

    /// The Tauri `moveBurst` / `moveBurstFrame`: `step` over the shown
    /// files' burst ids, a file not grouped yet its own id.
    fn move_burst(
        &mut self,
        step: fn(&[i64], usize, isize) -> usize,
        direction: isize,
        cx: &mut Context<Self>,
    ) {
        if self.files.is_empty() {
            return;
        }
        let ids: Vec<i64> = self
            .files
            .iter()
            .enumerate()
            .map(|(at, path)| {
                self.bursts
                    .get(path.to_string_lossy().as_ref())
                    .map_or(-1 - at as i64, |member| member.burst as i64)
            })
            .collect();
        let at = step(&ids, self.pager.current(), direction);
        self.go(Move::To(at), cx);
    }

    fn select_current(&mut self) {
        let current = self.current_path();
        self.set_selection(selection::single(current.as_deref()));
    }

    fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
        self.strip.set_selected(&self.selection.selected);
    }

    /// `extendPrevious` / `extendNext`: move the focus by `delta` and select
    /// the range from the anchor to it.
    fn extend(&mut self, delta: isize, cx: &mut Context<Self>) {
        let files = path_strings(&self.files);
        let (extended, at) =
            selection::extend(&self.selection, &files, self.pager.current(), delta);
        self.set_selection(extended);
        self.focus_on(Move::To(at), cx);
        cx.notify();
    }

    /// `selectAll`: every shown file, the focus kept.
    fn select_all(&mut self, cx: &mut Context<Self>) {
        let all = selection::all(&path_strings(&self.files), self.pager.current());
        self.set_selection(all);
        cx.notify();
    }

    /// Move to `to` without touching the selection. Whether the current file
    /// changed.
    fn focus_on(&mut self, to: Move, cx: &mut Context<Self>) -> bool {
        let key = Instant::now();
        let moved = self.pager.go(to);
        if moved {
            self.key = Some(key);
            self.step_note = None;
            self.dwell.cancel();
            self.dwell_timer = None;
            self.remember(cx);
            self.strip
                .scroll_into_view(self.pager.current(), self.strip_width);
            self.strip_moved(cx);
            // Before `request`: a ring hit shows the image at once and
            // `zoom.shown` stamps the facts with this page's `seq`.
            self.zoom_page(cx);
            self.request(key, cx);
            self.read_focus(cx);
        }
        moved
    }

    fn toggle_marks(&mut self, cx: &mut Context<Self>) {
        if self.marks.toggle() {
            self.read_focus(cx);
            self.analyze_current(cx);
        }
        cx.notify();
    }

    /// Read the current file's stored focus for the `f` overlay, while it is
    /// on; an answer for a file no longer current is dropped.
    fn read_focus(&mut self, cx: &mut Context<Self>) {
        let (Some(reader), Some(path)) = (self.reader.clone(), self.current_path()) else {
            return;
        };
        if !self.marks.is_on() {
            return;
        }
        let ticket = self.marks.reading();
        overlay::read(reader, path, ticket, cx);
    }

    /// Detect the faces and judge the eyes of the current file for the `f`
    /// overlay, once its image is shown and unless they are known or being
    /// worked on.
    fn analyze_current(&mut self, cx: &mut Context<Self>) {
        let current = self.current_path();
        if let Some((path, token)) = self.marks.wanted_faces(current.as_deref()) {
            overlay::detect(path, token, cx);
        }
        if let Some((path, ticket, still)) = self.marks.wanted_eyes(current.as_deref()) {
            overlay::judge(path, ticket, still, cx);
        }
    }

    /// After the strip scrolled, resized or changed its current cell: drop
    /// the thumbnails that left its range and ask for the ones that entered.
    fn strip_moved(&mut self, cx: &mut Context<Self>) {
        let released = self.strip.release_outside(self.strip_width);
        let old = self.thumbs.release(&released);
        self.retire(old);
        self.pump(cx);
        self.focus_scan_soon(cx);
        cx.notify();
    }

    /// Record the current file as the one to resume the folder at.
    fn remember(&mut self, cx: &mut Context<Self>) {
        let (Some(dir), Some(path)) = (self.dir.clone(), self.current_path()) else {
            return;
        };
        if let Some(value) = self.last_viewed.push((dir, path)) {
            self.write_last_viewed(value, cx);
        }
    }

    fn write_last_viewed(&self, (dir, path): (String, String), cx: &mut Context<Self>) {
        let Ok(index) = self.index.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let written = cx
                .background_spawn(async move { index::lock(&index).set_last_viewed(&dir, &path) })
                .await;
            if let Err(e) = written {
                log::warn!("failed to remember the last viewed file: {e}");
            }
            this.update(cx, |this, cx| {
                if let Some(value) = this.last_viewed.settled() {
                    this.write_last_viewed(value, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The current file's preview is on screen: start its dwell.
    fn start_dwell(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.current_path() else {
            return;
        };
        let Some(ticket) = self.dwell.shown(&path) else {
            self.dwell_timer = None;
            return;
        };
        self.dwell_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(VIEW_DWELL_MS))
                .await;
            this.update(cx, |this, cx| this.dwelt(ticket, cx)).ok();
        }));
    }

    fn dwelt(&mut self, ticket: u64, cx: &mut Context<Self>) {
        let (Some(path), Some(dir)) = (self.dwell.elapsed(ticket), &self.dir) else {
            return;
        };
        if let Some(batch) = self.viewed.push(dir, &path) {
            self.write_viewed(batch, cx);
        }
    }

    fn write_viewed(&self, Batch { dir, paths }: Batch, cx: &mut Context<Self>) {
        let Ok(index) = self.index.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let written = cx
                .background_spawn(async move { index::lock(&index).mark_viewed(&dir, &paths) })
                .await;
            if let Err(e) = written {
                log::warn!("failed to mark files viewed: {e}");
            }
            this.update(cx, |this, cx| {
                if let Some(batch) = this.viewed.settled() {
                    this.write_viewed(batch, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    fn pump(&mut self, cx: &mut Context<Self>) {
        let Some(reader) = &self.reader else { return };
        while self.strip.in_flight() < MAX_IN_FLIGHT {
            let Some(index) = self.strip.next_to_request(self.strip_width) else {
                return;
            };
            self.strip.request(index);
            let path = self.files[index].to_string_lossy().into_owned();
            self.thumbs.request(reader.clone(), index, path, cx);
        }
    }

    fn thumbnail(&mut self, generation: u64, index: usize, read: Read, cx: &mut Context<Self>) {
        if !self.thumbs.is_current(generation) {
            if let Read::Thumbnail(image) = read {
                self.retire(Some(image));
            }
            return;
        }
        match read {
            Read::Thumbnail(image) => {
                if self.strip.answer(index, Answer::Thumbnail) {
                    let old = self.thumbs.insert(index, image);
                    self.retire(old);
                } else {
                    self.retire(Some(image));
                }
            }
            Read::NotScanned => {
                self.strip.answer(index, Answer::NotScanned);
            }
            Read::Failed(reason) => {
                self.strip.answer(index, Answer::Failed);
                self.thumbs.fail(index, reason);
            }
        }
        self.pump(cx);
        cx.notify();
    }

    /// A click on a cell, as `main.ts`'s strip click handler: a toggle only
    /// changes the selection; a range or a plain click also moves the focus
    /// to the clicked file.
    fn strip_clicked(&mut self, x: f32, modifiers: selection::Modifiers, cx: &mut Context<Self>) {
        let Some(at) = self.strip.hit_test(x) else {
            return;
        };
        let files = path_strings(&self.files);
        let focused = self.pager.current();
        let clicked = selection::click(&self.selection, &files, focused, at, modifiers);
        self.set_selection(clicked);
        self.focus_on(Move::To(if modifiers.toggle { focused } else { at }), cx);
        cx.notify();
    }

    /// A vertical wheel scrolls the strip sideways too, as the Tauri strip's.
    fn strip_scrolled(&mut self, e: &ScrollEvent, window: &Window, cx: &mut Context<Self>) {
        let delta = e.delta.pixel_delta(window.line_height());
        let delta = if delta.x != px(0.) { delta.x } else { delta.y };
        if self.strip.scroll_by(-f32::from(delta), self.strip_width) {
            self.strip_moved(cx);
        }
    }

    /// What the strip's canvas paints: the cells on screen.
    fn strip_cells(&self) -> Vec<StripCell> {
        let current = self.pager.current();
        self.strip
            .on_screen(self.strip_width)
            .map(|index| {
                let path = &self.files[index];
                let key = path.to_string_lossy();
                let failure: Option<SharedString> = self.strip.is_failed(index).then(|| {
                    self.thumbs
                        .failure(index)
                        .unwrap_or_default()
                        .to_string()
                        .into()
                });
                let label = self
                    .judgments
                    .get(key.as_ref())
                    .and_then(|state| state.label.as_deref());
                StripCell {
                    file_key: {
                        use std::hash::{Hash, Hasher};
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        key.hash(&mut hasher);
                        hasher.finish() as usize
                    },
                    x: index as f32 * strip::CELL_PITCH - self.strip.offset(),
                    image: self.thumbs.image(index).cloned(),
                    name: file_name(path).into(),
                    tips: strip::tip_regions(failure.as_deref(), label),
                    failure,
                    current: index == current,
                    selected: index != current && self.strip.is_selected(index),
                    marks: self.strip.marks(index),
                    face: mark::af_eye_state(
                        self.facts.get(key.as_ref()).and_then(|f| f.focus.as_ref()),
                    ),
                    burst: self.strip.burst(index),
                }
            })
            .collect()
    }

    /// Show the current file: from the prefetch ring when it holds it, else
    /// through a read.
    fn request(&mut self, at: Instant, cx: &mut Context<Self>) {
        self.switched = at;
        if let Some(held) = self.take_held(cx) {
            self.show_held(held, cx);
            return;
        }
        if let Some(index) = self.pager.request() {
            self.send(index, at, cx);
        }
    }

    /// Take the current file from the ring, then point the ring at it, as
    /// `show()` in `main.ts`.
    fn take_held(&mut self, cx: &mut Context<Self>) -> Option<Held> {
        let path = self.current_path()?;
        let index = self.pager.current();
        self.heading.to(index);
        let ring = self.prefetch.as_mut()?;
        let held = ring.take(&path);
        let given = ring.plan(&self.files, index, self.heading.direction());
        self.give_up(given, cx);
        held
    }

    /// A page turn onto a held neighbour: everything a current result does
    /// but the read and the paint hold.
    fn show_held(&mut self, held: Held, cx: &mut Context<Self>) {
        let index = self.pager.current();
        self.pager.hit();
        let now = Instant::now();
        let key = self.key.take();
        let keyed = key.is_some();
        let page = (keyed || self.perf.on()).then(|| Page {
            key: perf::latency_start(key, self.switched),
            sent: now,
            finished: now,
            received: now,
            notified: now,
            read: Duration::ZERO,
            decode: Duration::ZERO,
            orient: Duration::ZERO,
            jpeg_kb: held.jpeg_kb,
            px: held.px,
            orientation: held.orientation,
            source: Source::Prefetch,
            file: file_name(&self.files[index]),
        });
        self.show_current(index, held, cx);
        self.pend_page(page, keyed);
        self.request_prefetch(cx);
    }

    /// Read the next neighbour once the current file's preview is on screen
    /// and no page read is out. Skipped while a scan pass runs, so the scan
    /// is not slowed by reads the user may never look at.
    fn request_prefetch(&mut self, cx: &mut Context<Self>) {
        let on_screen = self.screen.as_ref().map(|s| s.path.as_str());
        if self.pager.in_flight()
            || self.scanning()
            || on_screen.is_none()
            || on_screen != self.current_path().as_deref()
        {
            return;
        }
        let index = self.pager.current();
        let Some(ring) = &mut self.prefetch else {
            return;
        };
        let given = ring.plan(&self.files, index, self.heading.direction());
        let next = ring.next();
        self.give_up(given, cx);
        let Some(path) = next else { return };
        let slots = self.slots.clone();
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_spawn(async move { guarded(|| load(Path::new(&path), &slots)) })
                .await;
            this.update(cx, |this, cx| this.prefetched(loaded, cx)).ok();
        })
        .detach();
    }

    /// The prefetch in flight ended; its scan pause ends with it.
    fn prefetched(&mut self, loaded: Result<Loaded, String>, cx: &mut Context<Self>) {
        let held = loaded.ok().map(|loaded| loaded.held);
        let given = match &mut self.prefetch {
            Some(ring) => ring.settle(held),
            None => held.into_iter().collect(),
        };
        self.give_up(given, cx);
        self.request_prefetch(cx);
    }

    /// Retire what the ring gave up. `retired` drains only on a paint, so ask
    /// for one rather than hold the images past the cap until the next.
    fn give_up(&self, given: Vec<Held>, cx: &mut Context<Self>) {
        if !given.is_empty() {
            self.retire(given.into_iter().map(|held| held.image));
            cx.notify();
        }
    }

    /// A folder open or a re-listing, which may have changed any file's
    /// bytes: the ring gives everything up and the image on screen does not
    /// go back to it.
    fn clear_ring(&mut self, cx: &mut Context<Self>) {
        if let Some(screen) = &mut self.screen {
            screen.keepable = false;
        }
        if let Some(ring) = &mut self.prefetch {
            let given = ring.clear();
            self.give_up(given, cx);
        }
    }

    /// Take the image off screen and retire it.
    fn clear_image(&mut self) {
        let old = self.image.take();
        self.retire(old);
        self.screen = None;
    }

    fn send(&mut self, index: usize, at: Instant, cx: &mut Context<Self>) {
        self.sent = at;
        let path = self.files[index].clone();
        let slots = self.slots.clone();
        let order = self.order;
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_spawn(async move { guarded(|| load(&path, &slots)) })
                .await;
            let received = Instant::now();
            this.update(cx, |this, cx| {
                if order != this.order {
                    if let Ok(loaded) = loaded {
                        this.retire(Some(loaded.held.image));
                    }
                    return;
                }
                this.loaded(index, loaded, received, cx);
            })
            .ok();
        })
        .detach();
    }

    fn loaded(
        &mut self,
        index: usize,
        result: Result<Loaded, String>,
        received: Instant,
        cx: &mut Context<Self>,
    ) {
        let shown = match &self.shown {
            Some((at, Shown::Read(_))) => *at,
            _ => None,
        };
        match self.pager.finish(index, result.is_ok(), shown) {
            Finished::Current => {}
            Finished::Stale { draw, next } => {
                match result {
                    Ok(loaded) if draw => self.stale(index, loaded, cx),
                    Ok(loaded) => self.retire(Some(loaded.held.image)),
                    Err(_) => {}
                }
                self.send(next, received, cx);
                return;
            }
            Finished::Dropped => {
                if let Ok(loaded) = result {
                    self.retire(Some(loaded.held.image));
                }
                self.request_prefetch(cx);
                return;
            }
        }
        let key = self.key.take();
        let name = file_name(&self.files[index]);
        match result {
            Ok(Loaded {
                held,
                read,
                decode,
                orient,
                finished,
                paused,
            }) => {
                let keyed = key.is_some();
                let page = (keyed || self.perf.on()).then(|| Page {
                    key: perf::latency_start(key, self.switched),
                    sent: self.sent,
                    finished,
                    received,
                    notified: received,
                    read,
                    decode,
                    orient,
                    jpeg_kb: held.jpeg_kb,
                    px: held.px,
                    orientation: held.orientation,
                    source: Source::Read,
                    file: name,
                });
                self.show_current(index, held, cx);
                self.hold.hold(paused);
                self.pend_page(page, keyed);
                self.request_prefetch(cx);
            }
            Err(e) => {
                self.fail(&format!("{name}: {e}"));
                self.shown = Some((Some(index), Shown::Failed { name, error: e }));
                self.dwell.cancel();
                self.dwell_timer = None;
                self.clear_image();
                self.marks.show(None);
                cx.notify();
            }
        }
    }

    /// Show the current file's preview with everything that is the current
    /// file's: the overlay's analysis, the 1:1 facts and the dwell.
    fn show_current(&mut self, index: usize, held: Held, cx: &mut Context<Self>) {
        let (focus, orientation) = (held.focus, held.orientation);
        self.present(index, held);
        self.zoom.shown(focus, orientation);
        self.analyze_current(cx);
        self.start_dwell(cx);
        cx.notify();
    }

    /// The keyed page the next presented frame completes, notified now.
    fn pend_page(&self, page: Option<Page>, keyed: bool) {
        if let Some(mut page) = page {
            page.notified = Instant::now();
            *self.pending.borrow_mut() = Some((page, keyed));
        }
    }

    /// Draw another file's result on the way to the current one: the frame,
    /// its status line and meta pane, and the overlay's file, which keeps the
    /// current file's marks off it. Nothing that is the current file's.
    fn stale(&mut self, index: usize, loaded: Loaded, cx: &mut Context<Self>) {
        self.present(index, loaded.held);
        self.hold.hold(loaded.paused);
        cx.notify();
    }

    /// Put `held` on screen as the file at `index`: the image, the status
    /// line, the meta pane and the overlay's file. The image it replaces goes
    /// back to the ring when its file is wanted there, else is retired.
    fn present(&mut self, index: usize, held: Held) {
        let Held {
            image,
            jpeg_kb,
            px,
            orientation,
            meta,
            focus,
        } = held;
        self.status = self.status_line(index, px, orientation, &meta);
        let path = self.files[index].to_string_lossy().into_owned();
        let shown = self
            .shown
            .replace((Some(index), Shown::Read(Box::new(meta))));
        let old = self.image.replace(image);
        let screen = self.screen.replace(OnScreen {
            path: path.clone(),
            jpeg_kb,
            px,
            orientation,
            focus,
            keepable: true,
        });
        self.marks.show(Some((path, orientation)));
        let Some(old) = old else { return };
        match (screen, shown, &mut self.prefetch) {
            (Some(screen), Some((_, Shown::Read(meta))), Some(ring)) if screen.keepable => {
                let given = ring.keep(
                    screen.path,
                    Held {
                        image: old,
                        jpeg_kb: screen.jpeg_kb,
                        px: screen.px,
                        orientation: screen.orientation,
                        meta: *meta,
                        focus: screen.focus,
                    },
                );
                self.retire(given.into_iter().map(|held| held.image));
            }
            _ => self.retire(Some(old)),
        }
    }

    fn status_line(
        &self,
        index: usize,
        px: (u32, u32),
        orientation: u16,
        m: &Metadata,
    ) -> SharedString {
        format!(
            "{}/{}  {}  {}x{}  {}  {}  {}  {}",
            index + 1,
            self.files.len(),
            file_name(&self.files[index]),
            px.0,
            px.1,
            orientation,
            dash(&m.shutter),
            dash(&m.aperture),
            dash(&m.iso),
        )
        .into()
    }

    fn fail(&mut self, message: &str) {
        println!("error {message}");
        self.status = format!("error: {message}").into();
    }

    fn status(&self) -> SharedString {
        let mut status = self.status.to_string();
        if self.zoom.is_on() {
            status.push_str("  1:1");
        }
        if let Err(e) = &self.index {
            status.push_str(&format!("  no index: {e}"));
        }
        if let Some(scan) = self.scan_status.suffix() {
            status.push_str(&scan);
        } else if self.failed > 0 {
            status.push_str(&format!("  {} failed", self.failed));
        }
        status.into()
    }

    fn toggle_zoom(&mut self, cx: &mut Context<Self>) {
        let request = self
            .zoom
            .toggle(!self.files.is_empty(), self.viewport.get(), Instant::now());
        self.send_crop(request, cx);
        cx.notify();
    }

    /// Another file or listing is current: zoomed, read the new crop while the
    /// held one stays on screen; otherwise drop the held crop.
    fn zoom_page(&mut self, cx: &mut Context<Self>) {
        let (old, request) = self.zoom.page(!self.files.is_empty(), self.viewport.get());
        self.retire(old.map(|crop| crop.image));
        self.zoom_timer = None;
        self.send_crop(request, cx);
    }

    fn send_crop(&self, request: Option<zoom::Request>, cx: &mut Context<Self>) {
        if let Some(request) = request {
            zoom::send(request, self.files[self.pager.current()].clone(), cx);
        }
    }

    fn crop_arrived(
        &mut self,
        seq: u64,
        crop: Result<(zoom::Crop, (u32, u32)), String>,
        keypress: Option<Instant>,
        cx: &mut Context<Self>,
    ) {
        let timed = crop
            .as_ref()
            .ok()
            .zip(keypress)
            .map(|((c, _), key)| zoom::Timed {
                key,
                read: c.read,
                decode: c.decode,
                orient: c.orient,
            });
        match self.zoom.arrived(seq, crop, self.viewport.get()) {
            Arrived::Shown { release, resized } => {
                self.retire(release.map(|crop| crop.image));
                *self.zoom_pending.borrow_mut() = timed;
                if resized {
                    self.zoom_resized(cx);
                }
            }
            Arrived::Stale { release, request } => {
                self.retire(release.map(|crop| crop.image));
                self.send_crop(request, cx);
            }
            Arrived::Failed { error, release } => {
                self.retire(release.map(|crop| crop.image));
                let name = file_name(&self.files[self.pager.current()]);
                self.fail(&format!("{name}: {error}"));
            }
        }
        cx.notify();
    }

    /// Re-cut the crop once the viewport has settled for
    /// `CROP_RESIZE_DEBOUNCE`.
    fn zoom_resized(&mut self, cx: &mut Context<Self>) {
        let Some(seq) = self.zoom.resized() else {
            return;
        };
        self.zoom_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(zoom::CROP_RESIZE_DEBOUNCE)
                .await;
            this.update(cx, |this, cx| {
                let request = this.zoom.settled(seq, this.viewport.get());
                this.send_crop(request, cx);
            })
            .ok();
        }));
    }

    /// What the preview canvas paints while zoomed: the held crop, over the
    /// scaled preview when the file has a focus point and the crop, if any,
    /// is the current page's.
    fn zoom_paint(&self) -> Option<zoom::Paint> {
        if !self.zoom.is_on() {
            return None;
        }
        let crop = self.zoom.crop();
        let preview = self.zoom.preview().filter(|_| self.zoom.placeholder_fits());
        let placeholder =
            self.image
                .clone()
                .zip(preview)
                .and_then(|(image, (focus, orientation))| {
                    let s = image.size(0);
                    let preview = (s.width.0 as u32, s.height.0 as u32);
                    let rect =
                        zoom::placeholder(preview, orientation, focus?, crop.map(|c| c.full))?;
                    Some((image, rect))
                });
        Some(zoom::Paint {
            placeholder,
            crop: crop.map(|c| (c.image.clone(), c.point)),
            peaking: self.peaking.paint(View::Crop),
        })
    }

    fn toggle_peaking(&mut self, cx: &mut Context<Self>) {
        let old = self.peaking.toggle();
        self.retire(old);
        cx.notify();
    }

    /// The preview canvas draws `source` in `view` at `size` device pixels
    /// without its overlay.
    fn want_peaking(
        &mut self,
        view: View,
        source: Arc<RenderImage>,
        size: (u32, u32),
        cx: &mut Context<Self>,
    ) {
        let (old, job) = self.peaking.want(view, source, size);
        self.send_peaking(job, cx);
        if old.is_some() {
            self.retire(old);
            cx.notify();
        }
    }

    fn send_peaking(&self, job: Option<peaking::Job<Arc<RenderImage>>>, cx: &mut Context<Self>) {
        if let Some(job) = job {
            peaking::send(job, cx);
        }
    }

    fn peaking_arrived(&mut self, tag: u64, overlay: peaking::Overlay, cx: &mut Context<Self>) {
        println!("{}", overlay.line());
        let (old, job) = self.peaking.arrived(tag, overlay.image);
        self.retire(old);
        self.send_peaking(job, cx);
        cx.notify();
    }

    fn retire(&self, images: impl IntoIterator<Item = Arc<RenderImage>>) {
        self.retired.borrow_mut().extend(images);
    }

    /// Why the preview area shows no photo, centered over it. Only the
    /// no-folder message takes a click, which opens the folder picker.
    fn empty_message(
        &self,
        state: empty::State,
        text: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let message = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .text_center()
            .text_size(px(14.4))
            .text_color(rgb(MUTED_COLOR));
        if state != empty::State::NoFolder {
            return message.child(text).into_any_element();
        }
        message
            .id("empty")
            .cursor_pointer()
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.pick_folder(cx)))
            .child(text)
            .into_any_element()
    }

    /// The right-hand pane: the file on screen with its metadata, or why its
    /// preview failed, and the folder's notes at the bottom.
    fn meta_pane(&self) -> impl IntoElement {
        let mut body = div()
            .id("meta")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3();
        if let Some((_, shown)) = &self.shown {
            let (name, meta, error) = match shown {
                Shown::Read(meta) => (&meta.name, Some(meta.as_ref()), None),
                Shown::Failed { name, error } => (name, None, Some(error)),
            };
            body = body.child(
                div()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(name.clone()))
                    .child(
                        div()
                            .text_color(rgb(MUTED_COLOR))
                            .child(meta::position(self.pager.current(), self.files.len())),
                    ),
            );
            if let Some(error) = error {
                body = body.child(div().text_color(rgb(ERROR_COLOR)).child(error.clone()));
            }
            for group in meta::groups(meta, &[]) {
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(MUTED_COLOR))
                                .child(group.heading),
                        )
                        .children(group.rows.into_iter().map(|row| {
                            div()
                                .flex()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_none()
                                        .text_color(rgb(MUTED_COLOR))
                                        .child(row.label.to_owned()),
                                )
                                .child(div().min_w_0().text_right().child(row.value.to_owned()))
                        })),
                );
            }
        } else if self.files.is_empty() && !self.all.is_empty() {
            body = body.child(
                div()
                    .text_color(rgb(MUTED_COLOR))
                    .child(filter::FILTERED_TEXT),
            );
        }
        div()
            .w(px(PANE_WIDTH))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .bg(rgb(PANE_COLOR))
            .text_size(px(13.))
            .child(body)
            .children(self.view_only.then(|| {
                div()
                    .text_color(rgb(MUTED_COLOR))
                    .child(viewonly::VIEW_ONLY_NOTE)
            }))
            .children(
                self.step_note
                    .clone()
                    .map(|note| div().text_color(rgb(MUTED_COLOR)).child(note)),
            )
            .children(
                self.trashing
                    .map(|(mode, done, total)| trash::progress_status(mode, done, total))
                    .or_else(|| self.folderops.note())
                    .or_else(|| {
                        self.gate
                            .waiting()
                            .map(|label| format!("{label}: waiting for the scan to finish"))
                    })
                    .map(|note| div().text_color(rgb(MUTED_COLOR)).child(note)),
            )
            .children(
                self.folderops
                    .import_note()
                    .map(|note| div().text_color(rgb(MUTED_COLOR)).child(note)),
            )
            .children(
                self.notes
                    .iter()
                    .cloned()
                    .chain(self.outputs_failure.clone())
                    .chain(self.tree_note.clone())
                    .chain(self.write_errors.iter().map(|(path, message)| {
                        format!("{}: {message}", file_name(Path::new(path)))
                    }))
                    .map(|note| div().text_color(rgb(ERROR_COLOR)).child(note)),
            )
    }

    /// The left-hand folder tree: a `uniform_list`, so only the rows on
    /// screen are laid out. A left press in it gives it the keyboard, a press
    /// of any button outside it hands the keyboard back, and a right press in
    /// it does neither.
    fn tree_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.disarm_slow_click();
                    this.set_tree_keys(true, cx);
                }),
            )
            .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.disarm_slow_click();
                this.set_tree_keys(false, cx);
            }))
            .w(px(PANE_WIDTH))
            .flex_none()
            .h_full()
            .py_1()
            .bg(rgb(PANE_COLOR))
            .text_size(px(TREE_TEXT))
            .child(
                uniform_list(
                    "tree",
                    self.tree_rows.len(),
                    cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                        range.map(|at| this.tree_row(at, cx)).collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.tree_scroll)
                .size_full(),
            )
    }

    /// One row of the tree: the expander, the name and, expanded, the RAW
    /// count; the open folder highlighted, a failed one's name in red.
    fn tree_row(&self, at: usize, cx: &mut Context<Self>) -> AnyElement {
        let row = &self.tree_rows[at];
        let mut expander = div()
            .id(("tree-expander", at))
            .w(px(14.))
            .flex_none()
            .text_center()
            .text_size(px(11.))
            .text_color(rgb(MUTED_COLOR));
        if row.expandable {
            let path = row.path.clone();
            expander = expander
                .cursor_pointer()
                .child(if row.expanded { "\u{25be}" } else { "\u{25b8}" })
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.tree_toggle(path.clone(), cx);
                }));
        }
        let count = row
            .raw_count
            .filter(|count| row.expanded && *count > 0)
            .map(|count| {
                div()
                    .flex_none()
                    .px(px(5.))
                    .rounded_full()
                    .bg(rgb(ACCENT_COLOR))
                    .text_size(px(11.))
                    .child(count.to_string())
            });
        let path = row.path.clone();
        let (menu_path, menu_name) = (row.path.clone(), row.name.clone());
        div()
            .id(("tree-row", at))
            .h(px(TREE_ROW_HEIGHT))
            .mx_1()
            .flex()
            .items_center()
            .gap(px(2.))
            .pl(px(4. + row.depth as f32 * 12.))
            .pr_2()
            .rounded_sm()
            .border_1()
            .border_color(if self.tree_keys && self.tree_cursor_row == Some(at) {
                rgba(RING_COLOR)
            } else {
                rgba(0x00000000)
            })
            .cursor_pointer()
            .whitespace_nowrap()
            .hover(|s| s.bg(rgb(ACCENT_COLOR)))
            .when(self.tree_open_row == Some(at), |s| {
                s.bg(rgb(ACCENT_COLOR)).font_weight(FontWeight::SEMIBOLD)
            })
            .child(expander)
            .child(self.tree_name(at, cx))
            .children(count)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.open_from_tree(path.clone(), cx).detach();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    this.tree_context(e.position, menu_path.clone(), menu_name.clone(), cx);
                }),
            )
            .into_any_element()
    }

    /// A press on a menu's button.
    fn menu_pressed(&mut self, which: Which, at: Point<Pixels>, cx: &mut Context<Self>) {
        self.menu_scroll = ScrollHandle::new();
        self.menus.toggle(which, (f32::from(at.x), f32::from(at.y)));
        cx.notify();
    }

    /// A press outside the open menu's popover.
    /// Only a left press is recorded: no toggle listens to another button, so
    /// a record of it would swallow the next left press on the button.
    fn menu_dismissed(&mut self, e: &MouseDownEvent, cx: &mut Context<Self>) {
        if e.button == MouseButton::Left {
            self.menus
                .dismiss((f32::from(e.position.x), f32::from(e.position.y)));
        } else {
            self.menus.close();
        }
        cx.notify();
    }

    /// A click on a menu item: the sort or the filter changes, the changed
    /// `sortOrder` or remembered `filter` sections are written to the settings
    /// store, and the strip is rebuilt on the current file.
    fn menu_action(&mut self, action: menu::Action, cx: &mut Context<Self>) {
        self.menus.acted(&action);
        let (sort, stored) = (self.sort, self.filter.stored());
        menu::apply(&mut self.filter, &mut self.sort, action);
        if self.sort != sort {
            self.settings
                .writes
                .set("sortOrder", self.sort.setting().into());
        }
        let after = self.filter.stored();
        if after != stored {
            self.settings.writes.set("filter", after.setting());
        }
        let anchor = self.kept_anchor.anchor(self.current_path());
        self.refilter(anchor, false, None, cx);
        cx.notify();
    }

    /// The status line with the sort and filter buttons at its right end.
    fn status_row(&self, max_menu_height: Pixels, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .children(self.progress_bar())
            .child(div().flex_1().min_w_0().child(self.status()))
            .child(self.menu_button(
                Which::Sort,
                "Sort \u{25be}",
                false,
                !self.view_only,
                max_menu_height,
                cx,
            ))
            .child(self.menu_button(
                Which::Filter,
                "Filter \u{25be}",
                self.filter.is_active(),
                true,
                max_menu_height,
                cx,
            ))
    }

    /// The bar along the status row's top edge while an operation with a
    /// count runs, out of the row's flow so neither it nor the canvas above
    /// changes size.
    fn progress_bar(&self) -> Option<impl IntoElement> {
        let (done, total) = progress::shown(
            self.trashing.map(|(_, done, total)| (done, total)),
            None,
            self.scan_status.running(),
        )?;
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .w_full()
                .h(px(2.))
                .bg(rgb(MUTED_FILL))
                .child(
                    div()
                        .h_full()
                        .w(relative(progress::fraction(done, total)))
                        .bg(rgb(PRIMARY_COLOR)),
                ),
        )
    }

    /// A menu's button, lit while `lit`, with its popover above it while
    /// open; laid out only then.
    fn menu_button(
        &self,
        which: Which,
        text: &'static str,
        lit: bool,
        enabled: bool,
        max_menu_height: Pixels,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = match which {
            Which::Sort => "sort-toggle",
            Which::Filter => "filter-toggle",
        };
        let mut button = div()
            .id(id)
            .px_1p5()
            .rounded_sm()
            .border_1()
            .border_color(rgba(if lit {
                PRIMARY_COLOR << 8 | 0xff
            } else {
                INPUT_BORDER
            }))
            .bg(rgba(BUTTON_FILL))
            .text_size(px(12.))
            .text_color(rgb(if lit { PRIMARY_COLOR } else { FOREGROUND_COLOR }))
            .child(text);
        if enabled {
            button = button
                .cursor_pointer()
                .hover(|s| s.bg(rgb(ACCENT_COLOR)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                        this.menu_pressed(which, e.position, cx);
                    }),
                );
        } else {
            button = button.opacity(0.5);
        }
        let popover = (self.menus.open() == Some(which)).then(|| {
            let rows = match which {
                Which::Sort => menu::sort_rows(viewonly::sort_for(self.view_only, self.sort)),
                Which::Filter => {
                    menu::filter_rows(&self.filter, self.outputs.is_some(), &self.exif_groups)
                }
            };
            // Drawn after the rest of the window and over it, its right edge
            // on the button's, opening upward.
            div().absolute().top_0().right_0().child(deferred(
                anchored()
                    .anchor(Anchor::BottomRight)
                    .offset(point(px(0.), px(-4.)))
                    .snap_to_window()
                    .child(self.popover(rows, max_menu_height, cx)),
            ))
        });
        div().relative().child(button).children(popover)
    }

    fn popover(&self, rows: Vec<Row>, max_height: Pixels, cx: &mut Context<Self>) -> AnyElement {
        let rows: Vec<AnyElement> = rows
            .into_iter()
            .enumerate()
            .map(|(at, row)| menu_row(at, row, cx))
            .collect();
        menu_surface(rows, max_height, &self.menu_scroll, cx)
            .id("menu")
            .on_mouse_down_out(cx.listener(|this, e: &MouseDownEvent, _, cx| {
                this.menu_dismissed(e, cx);
            }))
            .min_w(px(180.))
            .into_any_element()
    }

    /// The open right-click menu at the pointer, drawn after the rest of the
    /// window and over it, flipped and kept inside the window; laid out only
    /// while open.
    fn context_popover(&self, max_height: Pixels, cx: &mut Context<Self>) -> Option<AnyElement> {
        let open = self.context.open()?;
        let (x, y) = open.at;
        let mut rows: Vec<AnyElement> = Vec::new();
        for (group, items) in open.groups.iter().enumerate() {
            if group > 0 {
                rows.push(menu_separator(cx));
            }
            for item in items {
                rows.push(context_row(rows.len(), item, cx));
            }
        }
        let popover = menu_surface(rows, max_height, &self.context_scroll, cx)
            .id("context-menu")
            .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.context.close();
                cx.notify();
            }))
            // A press on the menu is one outside the tree, also where the
            // menu is drawn over the pane, as a Tauri menu button takes the
            // focus. In the capture phase, since the scrollbar stops a press
            // on it from bubbling.
            .capture_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.set_tree_keys(false, cx);
            }))
            .min_w(px(180.));
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                // `anchored`'s default fit flips the menu to the pointer's
                // other side near an edge, as the Tauri `menuPosition`.
                .child(deferred(
                    anchored().position(point(px(x), px(y))).child(popover),
                ))
                .into_any_element(),
        )
    }

    /// Answer an MCP companion tool call from the view, after moving it as
    /// the call asks.
    fn mcp_request(&mut self, request: riffle_backend::mcp::Request, cx: &mut Context<Self>) {
        fn view(this: &Riffle) -> companion::View<'_> {
            let current = this.current_path();
            companion::View {
                folder: this.dir.as_deref(),
                files: &this.files,
                index: this.pager.current(),
                zoomed: this.zoom.is_on(),
                sort: this.sort.setting(),
                filtered: this.filter.is_active(),
                eyes: current.and_then(|path| this.marks.eyes(&path)),
                view_only: this.view_only,
                judgments: &this.judgments,
                seen: &this.seen,
                bursts: &this.bursts,
                facts: &this.facts,
            }
        }
        let action = companion::answer(request.kind, &request.args, &view(self));
        let reply = match action {
            Ok(Some(companion::Action::Judge { changes, judged })) => {
                self.judge_paths(changes, cx);
                Ok(judged)
            }
            Ok(action) => {
                match action {
                    Some(companion::Action::Show(at)) => {
                        self.focus_on(Move::To(at), cx);
                        self.select_current();
                    }
                    Some(companion::Action::ToggleZoom) => self.toggle_zoom(cx),
                    _ => {}
                }
                Ok(companion::get_view(&view(self)))
            }
            Err(e) => Err(e),
        };
        if let Some(server) = &self.mcp {
            server.reply(request.id, reply);
        }
        cx.notify();
    }

    fn quit(&mut self, cx: &mut Context<Self>) {
        println!("{}", self.summary.borrow().table());
        cx.quit();
    }
}

impl Render for Riffle {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let image = self.image.clone();
        let retired = self.retired.clone();
        let pending = self.pending.clone();
        let hold = self.hold.clone();
        let summary = self.summary.clone();
        let perf = self.perf.model();
        let cells = self.strip_cells();
        let strip_tips: Vec<_> = cells.iter().flat_map(strip_tips).collect();
        let mark = self.marks.mark(self.current_path().as_deref());
        let zoom_paint = self.zoom_paint();
        let viewport = self.viewport.clone();
        let zoom_pending = self.zoom_pending.clone();
        let fit_peaking = self.peaking.paint(View::Fit);
        let this = cx.entity().downgrade();
        let preview_failed = matches!(
            &self.shown,
            Some((Some(at), Shown::Failed { .. })) if *at == self.pager.current()
        ) && !self.zoom.is_on();
        let empty = empty::state(
            self.dir.is_some(),
            self.all.len(),
            self.files.len(),
            preview_failed,
        );
        let empty_text = empty::text(empty, &self.settings.keymap.bindings());
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x000000))
            .text_color(rgb(0xffffff))
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus)
            .group(DROP_GROUP)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.drop_paths(paths, cx);
            }))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.key_down(&e.keystroke, window, cx);
            }))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .children(self.panels.left.then(|| self.tree_pane(cx)))
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .child(
                                canvas(
                                    |_, _, _| {},
                                    move |bounds: Bounds<Pixels>, _, window, cx| {
                                        let page = pending.borrow_mut().take();
                                        let zoomed = zoom_pending.borrow_mut().take();
                                        let paint_started = Instant::now();
                                        let scale = window.scale_factor();
                                        viewport.set((
                                            (f32::from(bounds.size.width) * scale).round() as u32,
                                            (f32::from(bounds.size.height) * scale).round() as u32,
                                        ));
                                        let mut want = None;
                                        if let Some(zoom_paint) = zoom_paint {
                                            want = zoom_paint
                                                .paint(bounds, window)
                                                .map(|(image, size)| (View::Crop, image, size));
                                        } else if let Some(image) = image {
                                            let fit = fit_centered(&image, bounds);
                                            if let Err(e) = window.paint_image(
                                                fit,
                                                fit,
                                                Corners::default(),
                                                image.clone(),
                                                0,
                                                false,
                                            ) {
                                                println!("error paint: {e}");
                                            }
                                            want = fit_peaking
                                                .paint(&image, fit, fit, window)
                                                .map(|size| (View::Fit, image, size));
                                            overlay::paint(mark.as_ref(), fit, window, cx);
                                        }
                                        perfgraph::paint(&perf, bounds, window, cx);
                                        hold.release();
                                        let paint_finished = Instant::now();
                                        let old = retired.take();
                                        if page.is_none()
                                            && zoomed.is_none()
                                            && old.is_empty()
                                            && want.is_none()
                                        {
                                            return;
                                        }
                                        // `defer` runs when the update holding the frame's
                                        // draw and present ends.
                                        cx.defer(move |cx| {
                                            let presented = Instant::now();
                                            if let Some((page, keyed)) = page {
                                                let stages = page.stages(
                                                    paint_started,
                                                    paint_finished,
                                                    presented,
                                                );
                                                if keyed {
                                                    println!("{}", page.line(&stages));
                                                    summary.borrow_mut().record(stages);
                                                }
                                                perfgraph::add_latency(&perf, presented, stages);
                                            }
                                            if let Some(zoomed) = zoomed {
                                                println!("{}", zoomed.line(presented));
                                            }
                                            for image in old {
                                                cx.drop_image(image, None);
                                            }
                                            if let Some((view, image, size)) = want {
                                                this.update(cx, |this, cx| {
                                                    this.want_peaking(view, image, size, cx);
                                                })
                                                .ok();
                                            }
                                        });
                                    },
                                )
                                .size_full(),
                            )
                            .children(empty_text.map(|text| self.empty_message(empty, text, cx))),
                    )
                    .children(self.panels.right.then(|| self.meta_pane()))
                    .child(drop_outline()),
            )
            .children(self.panels.strip.then(|| {
                div()
                    .relative()
                    .w_full()
                    .h(px(strip::STRIP_HEIGHT))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, e: &MouseDownEvent, _, cx| {
                            // The strip spans the window from its left edge.
                            this.strip_context(e.position, f32::from(e.position.x), cx);
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, _, cx| {
                            let modifiers = selection::Modifiers {
                                toggle: e.modifiers.control || e.modifiers.platform,
                                range: e.modifiers.shift,
                            };
                            // The strip spans the window from its left edge.
                            this.strip_clicked(f32::from(e.position.x), modifiers, cx);
                        }),
                    )
                    .on_scroll(cx.listener(|this, e: &ScrollEvent, window, cx| {
                        this.strip_scrolled(e, window, cx);
                    }))
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, _, window, cx| paint_strip(&cells, bounds, window, cx),
                        )
                        .size_full(),
                    )
                    .children(strip_tips)
            }))
            .child(self.status_row(window.viewport_size().height - px(40.), cx))
            .child(self.settings_view.clone())
            .children(self.context_popover(window.viewport_size().height - px(40.), cx))
            .children(
                (self.trash.busy() || self.folderops.dialog_open()).then(|| modal::backdrop(cx)),
            )
    }
}

/// The window's group, so a drag anywhere over it lights the outline.
const DROP_GROUP: &str = "drop";

/// The key context of the app's view, inside `Root`'s.
const KEY_CONTEXT: &str = "Handpic";

/// `Root` binds `tab` / `shift-tab` to its focus moves. These mask them where
/// the focus is inside the app's view, so the keys reach `key_down` and the
/// keymap, while a component dialog, drawn in `Root`'s layer outside this
/// context, keeps them for its focus trap.
fn keymap_keys() -> [KeyBinding; 2] {
    [
        KeyBinding::new("tab", NoAction, Some(KEY_CONTEXT)),
        KeyBinding::new("shift-tab", NoAction, Some(KEY_CONTEXT)),
    ]
}

/// The `Move to Trash…` / `Delete Permanently…` dialog, inside `modal`'s
/// shell, rebuilt from the flow on every rebuild of the dialog layer: the
/// title, the line, and the run and Cancel buttons on the focus handles
/// `buttons` holds, both disabled while the run is under way. Its keys reach
/// `trash_key_down` before the focused button sees them.
fn trash_dialog(
    dialog: Dialog,
    this: &WeakEntity<Riffle>,
    buttons: &[FocusHandle; 2],
    cx: &mut App,
) -> Dialog {
    let Some((mode, count, running)) = this.upgrade().and_then(|view| {
        let flow = &view.read(cx).trash;
        flow.target()
            .map(|target| (target.mode, target.paths.len(), flow.running()))
    }) else {
        return dialog;
    };
    let text = trash::dialog_text(mode, count);
    let line = SharedString::from(text.line);
    let button = |id: &'static str, label: &'static str, which: trash::Button| {
        let this = this.clone();
        let at = match which {
            trash::Button::Run => 0,
            trash::Button::Cancel => 1,
        };
        Button::new(id)
            .track_focus(&buttons[at])
            .px(px(12.))
            .disabled(running)
            .child(
                div()
                    .text_size(px(13.6))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            )
            .on_click(move |e: &ClickEvent, _, cx| {
                this.update(cx, |this, cx| match which {
                    trash::Button::Run => {
                        let count = if e.is_keyboard() { 0 } else { e.click_count() };
                        this.confirm_run(count, cx);
                    }
                    trash::Button::Cancel => this.dismiss_trash(cx),
                })
                .ok();
            })
    };
    let view = this.clone();
    dialog
        .title(text.title)
        .content(move |content, _, _| content.child(line.clone()))
        .footer(
            div()
                .capture_key_down(move |e: &KeyDownEvent, window, cx| {
                    let name = keys::key_name(&e.keystroke);
                    let held = e.is_held;
                    let press = view
                        .update(cx, |this, cx| {
                            this.trash_key_down(name.as_deref(), held, cx)
                        })
                        .unwrap_or(false);
                    if !press {
                        window.prevent_default();
                        cx.stop_propagation();
                    }
                })
                .child(
                    DialogFooter::new()
                        .child(button("trash-run", text.run, trash::Button::Run).danger())
                        .child(button("trash-cancel", "Cancel", trash::Button::Cancel)),
                ),
        )
}

/// The Tauri `body.dragging #main` outline, laid over the main area while an
/// OS file drag is over the window. Its own `drag_over` gives it the hitbox
/// the window group's style is applied through.
fn drop_outline() -> impl IntoElement {
    let lit = |style: gpui::StyleRefinement| style.border_color(rgba(DROP_RING_COLOR));
    div()
        .absolute()
        .inset_0()
        .border_3()
        .border_color(rgba(0))
        .drag_over::<ExternalPaths>(move |style, _, _, _| lit(style))
        .group_drag_over::<ExternalPaths>(DROP_GROUP, lit)
}

/// An open menu's surface, `occlude()`d, the theme's popover with its border
/// and radius, holding `rows` in a list that scrolls past `max_height` under
/// the component's scrollbar, as `PopupMenu` draws its own.
fn menu_surface(
    rows: Vec<AnyElement>,
    max_height: Pixels,
    scroll: &ScrollHandle,
    cx: &Context<Riffle>,
) -> gpui::Div {
    let theme = cx.theme();
    div()
        .occlude()
        .relative()
        .flex()
        .flex_col()
        .bg(theme.popover)
        .text_color(theme.popover_foreground)
        .border_1()
        .border_color(theme.border)
        .rounded(theme.radius)
        .text_size(px(13.))
        .child(
            div()
                .id("items")
                .flex()
                .flex_col()
                .gap(px(2.))
                .p_1()
                // Less the surface's border, so the whole menu stays within
                // `max_height`.
                .max_h(max_height - px(2.))
                .overflow_y_scroll()
                .track_scroll(scroll)
                .children(rows),
        )
        .vertical_scrollbar(scroll)
}

/// `PopupMenu`'s separator: the theme's border, across the list's padding.
fn menu_separator(cx: &Context<Riffle>) -> AnyElement {
    div()
        .my(px(2.))
        .mx(px(-4.))
        .border_b(px(2.))
        .border_color(cx.theme().border)
        .into_any_element()
}

/// A menu row as `PopupMenu` lays one out: its height and padding, the check
/// mark's column, the theme's radius, and the accent on hover, or the muted
/// text while disabled.
fn menu_item(
    id: impl Into<gpui::ElementId>,
    checked: bool,
    disabled: bool,
    cx: &Context<Riffle>,
) -> gpui::Stateful<gpui::Div> {
    let theme = cx.theme();
    let (accent, accent_foreground) = (theme.accent, theme.accent_foreground);
    let row = div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap_1()
        .h(px(MENU_ROW))
        .px(px(MENU_PAD))
        .rounded(theme.radius)
        .child(
            if checked {
                Icon::new(IconName::Check)
            } else {
                Icon::empty()
            }
            .xsmall(),
        );
    if disabled {
        row.text_color(theme.muted_foreground)
    } else {
        row.cursor_pointer()
            .hover(move |s| s.bg(accent).text_color(accent_foreground))
    }
}

/// One row of an open menu: an item's check mark, swatch and text, or a
/// section's separator or heading.
fn menu_row(at: usize, row: Row, cx: &mut Context<Riffle>) -> AnyElement {
    let item = match row {
        Row::Separator => return menu_separator(cx),
        Row::Heading(text) => {
            // Indented to the item labels, past the check mark's column.
            return div()
                .pl(px(MENU_PAD + MENU_ICON + 4.))
                .pr(px(MENU_PAD))
                .py(px(3.))
                .text_size(px(11.))
                .text_color(cx.theme().muted_foreground)
                .child(text)
                .into_any_element();
        }
        Row::Item(item) => item,
    };
    let dot = |color: u32| div().size(px(7.)).flex_none().rounded_full().bg(rgb(color));
    let icon = |path: &'static str| svg().path(path).size(px(MENU_ICON)).flex_none();
    let lead = match item.lead {
        Lead::None => None,
        Lead::Flag(flag) => Some(
            dot(match flag {
                Some(filter::FilterFlag::Picked) => PICK_COLOR,
                Some(filter::FilterFlag::Rejected) => REJECT_COLOR,
                _ => MUTED_COLOR,
            })
            .into_any_element(),
        ),
        Lead::Label(Some(tint)) => Some(
            icon(icons::SQUARE_FILLED)
                .text_color(rgb(tint_color(tint)))
                .into_any_element(),
        ),
        Lead::Label(None) => Some(
            icon(icons::SQUARE)
                .text_color(rgb(MUTED_COLOR))
                .into_any_element(),
        ),
        Lead::Orientation(orientation) => Some(
            icon(match orientation {
                filter::Orientation::Portrait => icons::RECTANGLE_VERTICAL,
                filter::Orientation::Landscape => icons::RECTANGLE_HORIZONTAL,
            })
            .text_color(rgb(MUTED_COLOR))
            .into_any_element(),
        ),
        Lead::Face(Some(state)) => Some(
            icon(if state == mark::MarkState::Low {
                icons::SCAN_FACE_FROWN
            } else {
                icons::SCAN_FACE
            })
            .text_color(rgb(state.color()))
            .into_any_element(),
        ),
        Lead::Face(None) => Some(div().size(px(MENU_ICON)).flex_none().into_any_element()),
    };
    let label = match item.label {
        menu::Label::Text(text) => div().child(text),
        menu::Label::Owned(text) => div().child(text),
        menu::Label::Stars(on) => div()
            .flex()
            .items_center()
            .gap(px(2.))
            .children((0..5u8).map(|at| {
                if at < on {
                    icon(icons::STAR_FILLED).text_color(rgb(STARS_COLOR))
                } else {
                    icon(icons::STAR).text_color(rgba(MUTED_COLOR << 8 | 0x66))
                }
            })),
    };
    let action = item.action;
    menu_item(("menu-item", at), item.checked, false, cx)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .children(lead)
                .child(label),
        )
        .on_click(cx.listener(move |this, _, _, cx| this.menu_action(action.clone(), cx)))
        .into_any_element()
}

/// One item of the right-click menu: its check mark, its label and its
/// shortcut at the right end, in the muted color and inert while disabled.
fn context_row(at: usize, item: &context::Item, cx: &mut Context<Riffle>) -> AnyElement {
    let action = item.action;
    let muted = cx.theme().muted_foreground;
    let row = menu_item(
        ("context-item", at),
        item.checked == Some(true),
        item.disabled,
        cx,
    )
    .child(
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(item.label.clone())
            .child(
                div()
                    .flex_none()
                    .text_xs()
                    .text_color(muted)
                    .child(item.shortcut.clone()),
            ),
    );
    if item.disabled {
        return row.into_any_element();
    }
    row.on_click(cx.listener(move |this, _, window, cx| {
        this.context_action(action, window, cx);
    }))
    .into_any_element()
}

struct StripCell {
    /// A hash of the file's path, which the tooltip regions' ids carry.
    file_key: usize,
    /// From the strip's left edge.
    x: f32,
    image: Option<Arc<RenderImage>>,
    name: SharedString,
    /// `Some` on a cell the scan failed on, with why.
    failure: Option<SharedString>,
    current: bool,
    /// Selected and not current.
    selected: bool,
    marks: Marks,
    /// The face focus icon's state, `Neutral` drawing none.
    face: mark::MarkState,
    /// The hover texts' regions.
    tips: Vec<TipRegion>,
    burst: Option<BurstMark>,
}

/// The widest a strip tooltip gets before its text wraps.
const TIP_MAX_WIDTH: f32 = 360.;
/// `.cell span.marks svg`'s size.
const FACE_ICON: f32 = 12.;

/// The cell's tooltip regions, transparent `div`s over the strip's canvas
/// that take no click or wheel from the strip's own `div`. Their ids carry a
/// hash of the cell's file path, not its index, so a different file landing
/// at the same index drops the old file's tooltip state.
fn strip_tips(cell: &StripCell) -> impl Iterator<Item = AnyElement> + '_ {
    cell.tips.iter().map(|tip| {
        let text: SharedString = tip.text.clone().into();
        let id = match tip.tip {
            Tip::Failure => "strip-tip-failure",
            Tip::Label => "strip-tip-label",
        };
        div()
            .id((id, cell.file_key))
            .absolute()
            .left(px(cell.x + tip.x))
            .top(px(strip::STRIP_PADDING + tip.y))
            .w(px(tip.width))
            .h(px(tip.height))
            .tooltip(move |window, cx| {
                let text = text.clone();
                Tooltip::element(move |_, _| div().max_w(px(TIP_MAX_WIDTH)).child(text.clone()))
                    .build(window, cx)
            })
            .into_any_element()
    })
}

/// In the Tauri info pane's range (`#info`, 220 px), and the folder tree's
/// (`#side`).
const PANE_WIDTH: f32 = 220.;
/// `#folders`' 0.8 rem text in a 1.4 line, and `.folder`'s 2 px padding.
const TREE_TEXT: f32 = 12.8;
const TREE_ROW_HEIGHT: f32 = 22.;
const PANE_COLOR: u32 = 0x171717;
const MUTED_COLOR: u32 = 0xa1a1a1;
const ERROR_COLOR: u32 = 0xff6467;
const PLACEHOLDER: u32 = 0x262626;
const CURRENT_OUTLINE: u32 = 0xe5e5e5;
/// `style.css`'s `--muted`, a selected cell's fill.
const MUTED_FILL: u32 = 0x262626;
const NAME_COLOR: u32 = 0xd4d4d4;
const FAILURE_COLOR: u32 = 0xa3a3a3;
/// `style.css`'s `--stars-color`, `--pick-color`, `--reject-color` and
/// `--foreground`, and its `--label-*` tokens in `tint_color`.
const STARS_COLOR: u32 = 0xffd050;
const PICK_COLOR: u32 = 0x6bdc8a;
const REJECT_COLOR: u32 = 0xff6b6b;
const LABELED_NAME_COLOR: u32 = 0xfafafa;
/// `.cell.rejected`'s `opacity: 0.45` over the strip's black.
const REJECTED_DIM: u32 = 0x0000008c;
const MARK_SIZE: f32 = 11.;
/// `.cell.burst::before`'s `rgba(255, 255, 255, 0.08)` and its end radius.
const BAND_COLOR: u32 = 0xffffff14;
/// `BAND_COLOR` at the rejected cell's 0.45 opacity: 0.08 x 0.45.
const BAND_COLOR_DIMMED: u32 = 0xffffff09;
const BAND_RADIUS: f32 = 6.;
const FLAG_DOT: f32 = 7.;
/// `style.css`'s `--primary`, `--foreground`, `--accent`, `--border` and
/// `--input`, and `.button.outline`'s fill.
const PRIMARY_COLOR: u32 = 0xe5e5e5;
const FOREGROUND_COLOR: u32 = 0xfafafa;
const ACCENT_COLOR: u32 = 0x262626;
/// `style.css`'s `--sidebar-ring`, the tree cursor's outline.
const RING_COLOR: u32 = 0x737373ff;
/// `style.css`'s `--ring`, the drop outline.
const DROP_RING_COLOR: u32 = 0x737373ff;
const BORDER_COLOR: u32 = 0xffffff1a;
/// The backdrop behind the app's dialogs and the settings modal.
const OVERLAY_COLOR: u32 = 0x00000099;
const INPUT_BORDER: u32 = 0xffffff26;
const BUTTON_FILL: u32 = 0xffffff0b;
/// `PopupMenu`'s row height at its default size (1.625 rem at the window's
/// 16 px rem) and its horizontal padding (0.5 rem).
const MENU_ROW: f32 = 26.;
const MENU_PAD: f32 = 8.;
/// The filter menu's icons, as the Tauri `.face`.
const MENU_ICON: f32 = 12.;

fn tint_color(tint: Tint) -> u32 {
    match tint {
        Tint::Red => 0xd9453f,
        Tint::Orange => 0xe8892c,
        Tint::Yellow => 0xd8c032,
        Tint::Green => 0x4fa84f,
        Tint::Blue => 0x3f7fd9,
        Tint::Pink => 0xd95fa8,
        Tint::Purple => 0x8a5fd0,
        Tint::Other => 0x808080,
    }
}

fn paint_strip(cells: &[StripCell], bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    use strip::{BOX_HEIGHT, BOX_WIDTH, CELL_HEIGHT, CELL_WIDTH, NAME_HEIGHT, STRIP_PADDING};
    // Every band first, so one reaching across the gap stays behind the
    // neighbour's outline.
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        for cell in cells {
            if let Some(mark) = &cell.burst {
                let origin = bounds.origin + point(px(cell.x), px(STRIP_PADDING));
                paint_burst_band(mark, cell.marks.dimmed(), origin, window);
            }
        }
    });
    for cell in cells {
        let origin = bounds.origin + point(px(cell.x), px(STRIP_PADDING));
        let cell_bounds = Bounds::new(origin, size(px(CELL_WIDTH), px(CELL_HEIGHT)));
        let image_box = Bounds::new(
            origin + point(px(1.), px(1.)),
            size(px(BOX_WIDTH), px(BOX_HEIGHT)),
        );
        if cell.current {
            window.paint_quad(outline(
                cell_bounds,
                rgb(CURRENT_OUTLINE),
                Default::default(),
            ));
        } else if cell.selected {
            // `.cell.selected`: `--muted` behind, a `--ring` border.
            window.paint_quad(fill(cell_bounds, rgb(MUTED_FILL)));
            window.paint_quad(outline(cell_bounds, rgba(RING_COLOR), Default::default()));
        }
        window.with_content_mask(
            Some(ContentMask {
                bounds: cell_bounds,
            }),
            |window| {
                match &cell.image {
                    Some(image) => {
                        let fit = fit_centered(image, image_box);
                        if let Err(e) = window.paint_image(
                            fit,
                            fit,
                            Corners::default(),
                            image.clone(),
                            0,
                            false,
                        ) {
                            println!("error paint: {e}");
                        }
                    }
                    None => window.paint_quad(fill(image_box, rgb(PLACEHOLDER))),
                }
                if let Some(failure) = &cell.failure {
                    paint_failure(failure, image_box, window, cx);
                }
                paint_marks(&cell.marks, image_box, window, cx);
                if let Some(mark) = &cell.burst {
                    paint_burst_badge(mark, image_box, window, cx);
                }
                if let Some(icon) = cell.face.icon() {
                    paint_face_icon(icon, cell.face.color(), image_box, window, cx);
                }
                let at = point(origin.x, image_box.bottom());
                let name_color = match cell.marks.label {
                    Some(tint) => {
                        let band = Bounds::new(at, size(px(CELL_WIDTH), px(NAME_HEIGHT)));
                        window.paint_quad(fill(band, rgb(tint_color(tint))));
                        LABELED_NAME_COLOR
                    }
                    None => NAME_COLOR,
                };
                let mut run = window.text_style().to_run(cell.name.len());
                run.color = rgb(name_color).into();
                let name =
                    window
                        .text_system()
                        .shape_line(cell.name.clone(), px(11.), &[run], None);
                let _ = name.paint(
                    at,
                    px(NAME_HEIGHT),
                    TextAlign::Center,
                    Some(px(CELL_WIDTH)),
                    window,
                    cx,
                );
                if cell.marks.dimmed() {
                    window.paint_quad(fill(cell_bounds, rgba(REJECTED_DIM)));
                }
            },
        );
    }
}

/// `.cell.burst::before`: a tint around the cell at `origin`, measured from
/// its padding box 5 px above and below and 1 px right, on a member after
/// another left across the gap, 2 px out with rounded corners at the band's
/// ends. A rejected member's band is dimmed with its cell, as `.cell.rejected`'s
/// opacity covers the `::before`.
fn paint_burst_band(mark: &BurstMark, dimmed: bool, origin: Point<Pixels>, window: &mut Window) {
    use strip::{CELL_HEIGHT, CELL_PITCH, CELL_WIDTH};
    // The padding box sits inside the cell's 1 px border.
    let (inner_left, inner_right) = (1., CELL_WIDTH - 1.);
    let (top, bottom) = (1. - 5., CELL_HEIGHT - 1. + 5.);
    let left = if mark.first {
        inner_left - 2.
    } else {
        inner_left - 1. - (CELL_PITCH - CELL_WIDTH)
    };
    let right = if mark.last {
        inner_right + 2.
    } else {
        inner_right + 1.
    };
    let band = Bounds::new(
        origin + point(px(left), px(top)),
        size(px(right - left), px(bottom - top)),
    );
    let (start, end) = (
        if mark.first { px(BAND_RADIUS) } else { px(0.) },
        if mark.last { px(BAND_RADIUS) } else { px(0.) },
    );
    let color = if dimmed {
        BAND_COLOR_DIMMED
    } else {
        BAND_COLOR
    };
    window.paint_quad(fill(band, rgba(color)).corner_radii(Corners {
        top_left: start,
        bottom_left: start,
        top_right: end,
        bottom_right: end,
    }));
}

/// `.cell span.marks`: the face focus icon 2 px inside the image box's
/// bottom-left, over dark copies for the `drop-shadow` halo.
fn paint_face_icon(
    icon: &'static str,
    color: u32,
    image_box: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let inset = px(2.);
    let at = point(
        image_box.origin.x + inset,
        image_box.bottom() - inset - px(FACE_ICON),
    );
    let paint = |at: Point<Pixels>, color: u32, window: &mut Window| {
        let bounds = Bounds::new(at, size(px(FACE_ICON), px(FACE_ICON)));
        if let Err(e) = window.paint_svg(
            bounds,
            icon.into(),
            None,
            TransformationMatrix::unit(),
            rgb(color).into(),
            cx,
        ) {
            log::warn!("failed to paint {icon}: {e}");
        }
    };
    for (dx, dy) in overlay::HALO {
        paint(at + point(px(dx), px(dy)), 0x000000, window);
    }
    paint(at, color, window);
}

/// `.cell span.count`: the `position/size` badge at the image box's
/// bottom-right, over dark copies for the halo.
fn paint_burst_badge(
    mark: &BurstMark,
    image_box: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let text: SharedString = burst::burst_badge(Some(mark)).into();
    let shape = |color: u32, window: &mut Window| {
        let mut run = window.text_style().to_run(text.len());
        run.color = rgb(color).into();
        window
            .text_system()
            .shape_line(text.clone(), px(MARK_SIZE), &[run], None)
    };
    let inset = px(2.);
    let line_height = px(MARK_SIZE + 3.);
    let width = Some(image_box.size.width - inset * 2.);
    let at = point(
        image_box.origin.x + inset,
        image_box.bottom() - inset - line_height,
    );
    let halo = shape(0x000000, window);
    for (dx, dy) in overlay::HALO {
        let _ = halo.paint(
            at + point(px(dx), px(dy)),
            line_height,
            TextAlign::Right,
            width,
            window,
            cx,
        );
    }
    let _ =
        shape(FOREGROUND_COLOR, window).paint(at, line_height, TextAlign::Right, width, window, cx);
}

/// The stars at the image box's top-right over a 1 px dark copy, and the pick
/// or reject dot at its top-left, as the Tauri cell's badges.
fn paint_marks(marks: &Marks, image_box: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let inset = px(2.);
    if marks.stars > 0 {
        let stars: SharedString = "\u{2605}".repeat(marks.stars.into()).into();
        let shape = |color: u32, window: &mut Window| {
            let mut run = window.text_style().to_run(stars.len());
            run.color = rgb(color).into();
            window
                .text_system()
                .shape_line(stars.clone(), px(MARK_SIZE), &[run], None)
        };
        let width = Some(image_box.size.width - inset * 2.);
        let at = image_box.origin + point(inset, inset);
        let line_height = px(MARK_SIZE + 3.);
        let _ = shape(0x000000, window).paint(
            at + point(px(1.), px(1.)),
            line_height,
            TextAlign::Right,
            width,
            window,
            cx,
        );
        let _ =
            shape(STARS_COLOR, window).paint(at, line_height, TextAlign::Right, width, window, cx);
    }
    let color = match marks.flag {
        Flag::Pick => PICK_COLOR,
        Flag::Reject => REJECT_COLOR,
        Flag::None => return,
    };
    let r = px(FLAG_DOT / 2.);
    let center = image_box.origin + point(inset + px(3.) + r, inset + px(3.) + r);
    window.paint_quad(
        fill(
            Bounds::new(center - point(r, r), size(r * 2., r * 2.)),
            rgb(color),
        )
        .corner_radii(r),
    );
}

/// Why the scan failed, wrapped inside the image box.
fn paint_failure(
    text: &SharedString,
    image_box: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let inner = Bounds::new(
        image_box.origin + point(px(6.), px(6.)),
        image_box.size - size(px(12.), px(12.)),
    );
    let mut run = window.text_style().to_run(text.len());
    run.color = rgb(FAILURE_COLOR).into();
    let line_height = px(12.);
    let Ok(lines) = window.text_system().shape_text(
        text.clone(),
        px(10.),
        &[run],
        Some(inner.size.width),
        None,
    ) else {
        return;
    };
    window.with_content_mask(Some(ContentMask { bounds: inner }), |window| {
        let mut at = inner.origin;
        for line in lines {
            let _ = line.paint(at, line_height, TextAlign::Left, Some(inner), window, cx);
            at.y += line.size(line_height).height;
        }
    });
}

fn dash(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("-")
}

fn path_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

fn fit_centered(image: &RenderImage, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
    let image_size = image.size(0);
    let (iw, ih) = (image_size.width.0 as f32, image_size.height.0 as f32);
    let scale = (f32::from(bounds.size.width) / iw).min(f32::from(bounds.size.height) / ih);
    let fit = size(px(iw * scale), px(ih * scale));
    let origin = bounds.origin
        + point(
            (bounds.size.width - fit.width) / 2.,
            (bounds.size.height - fit.height) / 2.,
        );
    Bounds::new(origin, fit)
}

/// The writer and the read-only connection; the writer stands in for a
/// reader that does not open, as in the Tauri app.
type Indexes = (Arc<Mutex<Index>>, Arc<Mutex<Index>>);

fn open_index(flag: Option<PathBuf>) -> Result<Indexes, String> {
    let path = datadir::index_path(flag, dirs::cache_dir().as_deref())
        .ok_or("no cache dir to keep the index in")?;
    let index = Index::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    log::info!("index: {}", path.display());
    let index = Arc::new(Mutex::new(index));
    let reader = match Index::open_reader(&path) {
        Ok(reader) => Arc::new(Mutex::new(reader)),
        Err(e) => {
            log::error!("failed to open the index reader at {}: {e}", path.display());
            index.clone()
        }
    };
    Ok((index, reader))
}

fn rect_of(bounds: Bounds<Pixels>) -> session::Rect {
    session::Rect {
        x: f32::from(bounds.origin.x),
        y: f32::from(bounds.origin.y),
        width: f32::from(bounds.size.width),
        height: f32::from(bounds.size.height),
    }
}

/// The window's restore bounds and display; a fullscreen window is kept by
/// its restore bounds, windowed.
fn window_geometry(window: &Window, cx: &App) -> session::Geometry {
    let (bounds, maximized) = match window.window_bounds() {
        WindowBounds::Windowed(bounds) | WindowBounds::Fullscreen(bounds) => (bounds, false),
        WindowBounds::Maximized(bounds) => (bounds, true),
    };
    session::Geometry {
        bounds: rect_of(bounds),
        maximized,
        display: window
            .display(cx)
            .and_then(|display| display.uuid().ok())
            .map(|uuid| uuid.to_string()),
    }
}

/// Where the window opens: the saved geometry placed on a display, else
/// centered at 1600x1000.
fn window_placement(
    saved: Option<&session::Geometry>,
    cx: &App,
) -> (Option<gpui::DisplayId>, WindowBounds) {
    let displays = cx.displays();
    let screens: Vec<session::Screen> = displays
        .iter()
        .map(|display| session::Screen {
            uuid: display.uuid().ok().map(|uuid| uuid.to_string()),
            bounds: rect_of(display.bounds()),
            visible: rect_of(display.visible_bounds()),
        })
        .collect();
    let Some((saved, (index, rect))) =
        saved.and_then(|saved| Some((saved, session::place(saved, &screens)?)))
    else {
        let bounds = Bounds::centered(None, size(px(1600.), px(1000.)), cx);
        return (None, WindowBounds::Windowed(bounds));
    };
    let bounds = Bounds::new(
        point(px(rect.x), px(rect.y)),
        size(px(rect.width), px(rect.height)),
    );
    log::info!(
        "window: restored on display {index} at {rect:?} maximized={}",
        saved.maximized
    );
    let display_id = Some(displays[index].id());
    if saved.maximized {
        (display_id, WindowBounds::Maximized(bounds))
    } else {
        (display_id, WindowBounds::Windowed(bounds))
    }
}

fn main() {
    let mut args = match args::parse(env::args_os().skip(1)) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}\n{}", args::USAGE);
            process::exit(2);
        }
    };
    logger::install(args.log.take());
    riffle_backend::diagnostics::install_panic_hook();
    let index = open_index(args.index.take()).inspect_err(|e| log::warn!("no index: {e}"));
    let settings = settings::load(args.settings.take(), args.import_settings.take().as_deref());
    log::info!(
        "settings: sidecarFormat={} autoAdvance={} sortOrder={} filter={:?} store={}",
        settings.format.setting(),
        settings.auto_advance,
        settings.sort.setting(),
        settings.filter,
        if settings.writes.is_open() {
            "open"
        } else {
            "none"
        }
    );
    let mcp_port = args.mcp_port;
    let prefetch = !args.no_prefetch;
    let remember = args.auto.is_none();
    args.folder = session::startup_folder(
        args.folder.take(),
        settings.last_folder.as_deref(),
        !remember,
        Path::is_dir,
    );
    let saved = settings.window.clone().filter(|_| remember);
    application()
        .with_assets(icons::Icons)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            cx.bind_keys(keymap_keys());
            theme::apply(cx);
            cx.on_window_closed(|cx, _| cx.quit()).detach();
            let (display_id, window_bounds) = window_placement(saved.as_ref(), cx);
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("Handpic".into()),
                        ..Default::default()
                    }),
                    window_bounds: Some(window_bounds),
                    display_id,
                    app_id: Some(datadir::IDENTIFIER.into()),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| {
                        let focus = cx.focus_handle();
                        window.focus(&focus, cx);
                        Riffle::start(args, cx);
                        Riffle::new(
                            focus, index, settings, mcp_port, prefetch, remember, window, cx,
                        )
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{KeyContext, Keymap, actions};

    actions!(test, [Focus]);

    fn bound(keymap: &Keymap, key: &'static str, contexts: &[&str]) -> usize {
        let stack: Vec<KeyContext> = contexts
            .iter()
            .map(|context| KeyContext::parse(context).unwrap())
            .collect();
        let (bindings, pending) =
            keymap.bindings_for_input(&[Keystroke::parse(key).unwrap()], &stack);
        assert!(!pending);
        bindings.len()
    }

    #[test]
    fn tab_and_shift_tab_reach_the_keymap_only_inside_the_view() {
        let mut keymap = Keymap::new(vec![
            KeyBinding::new("tab", Focus, Some("Root")),
            KeyBinding::new("shift-tab", Focus, Some("Root")),
        ]);
        keymap.add_bindings(keymap_keys());
        for key in ["tab", "shift-tab"] {
            assert_eq!(bound(&keymap, key, &["Root", KEY_CONTEXT]), 0);
            assert_eq!(bound(&keymap, key, &["Root"]), 1);
        }
    }
}
