mod args;
mod datadir;
mod eyes;
mod facemesh;
mod faces;
mod filter;
mod focus;
mod history;
mod hold;
mod judgment;
mod keys;
mod logger;
mod mark;
mod menu;
mod meta;
mod orient;
mod overlay;
mod pager;
mod peaking;
mod resync;
mod scan;
mod settings;
mod sort;
mod strip;
mod thumbs;
mod timing;
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
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use args::Args;
use filter::{Facts, FilterState};
use futures::{
    StreamExt,
    channel::mpsc::{self, UnboundedSender},
};
use gpui::{
    Anchor, AnyElement, App, Bounds, ContentMask, Context, Corners, FocusHandle, FontWeight,
    KeyDownEvent, Keystroke, MouseButton, MouseDownEvent, PathPromptOptions, Pixels, Point,
    RenderImage, ScrollEvent, SharedString, Task, TextAlign, TitlebarOptions, Window, WindowBounds,
    WindowOptions, anchored, canvas, deferred, div, fill, outline, point, prelude::*, px, rgb,
    rgba, size,
};
use gpui_platform::application;
use history::{Judged, Undo};
use hold::PaintHold;
use image::{Frame, RgbaImage};
use judgment::{Command, State};
use keys::Bindings;
use menu::{Lead, Menus, Row, Which};
use meta::Shown;
use pager::{Move, Pager};
use peaking::{Peaking, View};
use resync::Resync;
use riffle_backend::{
    exif::{self, Metadata},
    index::{self, Index},
    listing::canonicalize,
    sidecar::{self, Writer},
    watch::{self, Watch},
};
use riffle_core::{
    Flag,
    arw::FocusLocation,
    reader,
    scan::{Paused, ReadSlots, ScanFocus},
};
use scan::{Event, Listed, Outputs, Prepared, Running, Scan};
use settings::Settings;
use sort::{SortFacts, SortKey};
use strip::{Answer, Marks, Strip, Tint};
use thumbs::{MAX_IN_FLIGHT, Read, Thumbs};
use timing::{Page, Summary};
use viewed::{Batch, Batches, Dwell, Latest, VIEW_DWELL_MS};
use zoom::{Arrived, Zoom};

const AUTO_SETTLE: Duration = Duration::from_secs(2);
/// Out of phase with 60 Hz vsync.
const AUTO_INTERVAL: Duration = Duration::from_millis(1003);

struct Loaded {
    image: Arc<RenderImage>,
    read: Duration,
    decode: Duration,
    orient: Duration,
    finished: Instant,
    jpeg_kb: usize,
    px: (u32, u32),
    orientation: u16,
    meta: Metadata,
    focus: Option<FocusLocation>,
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
    let meta = exif::metadata(file_name(path), &arw.shot);
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
        image,
        read,
        decode,
        orient,
        finished: Instant::now(),
        jpeg_kb: jpeg.len() / 1024,
        px: (w, h),
        orientation: arw.orientation,
        meta,
        focus: arw.shot.focus,
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

/// The listed files' judgments and facts by path.
type Entries = (HashMap<String, State>, HashMap<String, Facts>);

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
    Some((judgments, facts))
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
    /// Pass 2 files reported since the last read of the index, laid over a
    /// snapshot read before they were written.
    pending_faces: Vec<riffle_backend::index::FaceReady>,
    /// The sort key the user has; a view-only folder is in capture order
    /// whatever it is (`viewonly::sort_for`).
    sort: SortKey,
    /// The strip's filter; a change lives for the session only.
    filter: FilterState,
    /// The status row's sort or filter menu, while one is open.
    menus: Menus,
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
    slots: ReadSlots,
    scan: Running,
    /// The pending `ScanFocus` update, replaced on every change.
    scan_focus: Option<Task<()>>,
    next_scan: u64,
    /// The open folder's canonical path, the key of its index rows.
    dir: Option<String>,
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
    /// index.
    shown: Option<(usize, Shown)>,
    /// The open folder holds only JPEGs.
    view_only: bool,
    settings: Settings,
    bindings: Bindings,
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
    /// The open folder's undo and redo histories.
    undo: Undo,
    /// `Undid <name>` / `Redid <name>` until the view moves on.
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
    /// The keyed page the next presented frame completes.
    pending: Rc<RefCell<Option<Page>>>,
    /// The shown image's scan pause, released once it is painted.
    hold: PaintHold,
    summary: Rc<RefCell<Summary>>,
    key: Option<Instant>,
    sent: Instant,
    zoom: Zoom<zoom::Crop>,
    /// The pending re-cut after a resize; dropping it cancels it.
    zoom_timer: Option<Task<()>>,
    /// The preview canvas's size in device pixels, as last painted.
    viewport: Rc<Cell<(u32, u32)>>,
    /// The keyed crop the next presented frame shows.
    zoom_pending: Rc<RefCell<Option<zoom::Timed>>>,
    peaking: Peaking<Arc<RenderImage>, Arc<RenderImage>>,
}

impl Riffle {
    fn new(
        focus: FocusHandle,
        index: Result<Indexes, String>,
        settings: Settings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (index, reader) = match index {
            Ok((writer, reader)) => (Ok(writer), Some(reader)),
            Err(e) => (Err(e), None),
        };
        cx.observe_window_bounds(window, |this, window, cx| {
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
        Self {
            focus,
            all: Vec::new(),
            files: Vec::new(),
            facts: HashMap::new(),
            pending_faces: Vec::new(),
            sort: settings.sort,
            filter: {
                let mut filter = FilterState::default();
                filter.apply_stored(&settings.filter);
                filter
            },
            menus: Menus::default(),
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
            slots: ReadSlots::new(scan::READ_SLOTS),
            scan: running,
            scan_focus: None,
            next_scan: 0,
            dir: None,
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
            settings,
            judgments: HashMap::new(),
            seen: HashSet::new(),
            touched: HashSet::new(),
            writer,
            stores,
            undo: Undo::new(),
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
            zoom: Zoom::default(),
            zoom_timer: None,
            viewport: Rc::default(),
            zoom_pending: Rc::default(),
            peaking: Peaking::default(),
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
        } = args;
        let prompt = folder.is_none().then(|| {
            cx.prompt_for_paths(PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: None,
            })
        });
        cx.spawn(async move |this, cx| {
            let folder = match (folder, prompt) {
                (Some(folder), _) => Some(folder),
                (None, Some(prompt)) => prompt
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .flatten()
                    .and_then(|mut paths| paths.pop()),
                (None, None) => None,
            };
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
        self.wanted = wanted;
        self.max = max;
        self.resync.open();
        self.list(folder, false, cx)
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
        if self.resync.finished() {
            self.relist(cx);
        }
    }

    fn relist(&mut self, cx: &mut Context<Self>) {
        let Some(dir) = self.dir.clone() else { return };
        log::info!("resync: dir={dir}");
        self.list(PathBuf::from(dir), true, cx).detach();
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
        let watching = self.watch.clone();
        let writer = self.writer.clone();
        let (format, names) = (self.settings.format, self.settings.names.clone());
        cx.spawn(async move |this, cx| {
            let opened = cx
                .background_spawn(async move {
                    if let Some(previous) = previous {
                        previous.stop();
                    }
                    if let Some(eviction) = eviction {
                        let _ = eviction.join();
                    }
                    // Watched before the listing, so a change during it is
                    // not missed.
                    let dir = canonicalize(&folder.to_string_lossy());
                    watch::set(&watching, &dir, &dir);
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
                    Ok::<_, String>(Opened {
                        listed,
                        shown,
                        outputs,
                        todo,
                        entries,
                        last_viewed,
                        viewed,
                    })
                })
                .await;
            this.update(cx, |this, cx| this.opened(opened, carry, cx))
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
        self.dir = Some(listed.dir.clone());
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
            self.dwell.reset(viewed);
            self.dwell_timer = None;
            self.touched.clear();
            self.undo.clear();
            self.step_note = None;
            self.write_errors.clear();
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
            let old = self.image.take();
            self.retire(old);
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
            .and_then(|(at, _)| old.get(*at).cloned());
        let moved = self.carry(&next, current, keep_scroll);
        self.order += 1;
        if let (Some((at, _)), Some(path)) = (&mut self.shown, &shown)
            && let Some(to) = next.iter().position(|p| p == path)
        {
            *at = to;
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
        self.mark_strip();
        self.strip_moved(cx);
        if next.is_empty() {
            self.status = filter::FILTERED_TEXT.into();
            let old = self.image.take();
            self.retire(old);
            self.shown = None;
            self.marks.show(None);
        }
        if previous.is_none() {
            if moved {
                self.remember(cx);
            }
            // The order's bump dropped a preview read still out.
            if moved || shown != self.current_path() {
                self.request(Instant::now(), cx);
            }
            if moved {
                self.read_focus(cx);
                self.zoom_page(cx);
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
            }
            Event::Changed(_) | Event::SidecarError { .. } => {}
        }
        self.pump(cx);
        cx.notify();
    }

    /// Keep the folder's judgments, the view's own for a file judged since
    /// the open, and its facts.
    fn set_entries(&mut self, (mut judgments, facts): Entries) {
        self.seen = judgments.keys().cloned().collect();
        for path in &self.touched {
            match self.judgments.get(path) {
                Some(state) => judgments.insert(path.clone(), state.clone()),
                None => judgments.remove(path),
            };
        }
        self.judgments = judgments;
        self.facts = facts;
    }

    /// Hand the shown files' judgments to the strip.
    fn mark_strip(&mut self) {
        for (index, path) in self.files.iter().enumerate() {
            let state = self.judgments.get(path.to_string_lossy().as_ref()).cloned();
            self.strip.set_judgment(index, state);
        }
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
    /// native app's own keys, and `escape` while a menu is open; an action
    /// not handled yet is only logged.
    fn key_down(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        let Some(name) = keys::key_name(keystroke) else {
            return;
        };
        let action = self.bindings.action_for(&name);
        log::debug!("key: {name} action={action:?}");
        match (name.as_str(), action) {
            ("home", _) => self.go(Move::First, cx),
            ("end", _) => self.go(Move::Last, cx),
            ("q", _) => self.quit(cx),
            ("escape", _) if self.menus.is_open() => {
                self.menus.close();
                cx.notify();
            }
            (_, Some("previous")) => self.go(Move::Previous, cx),
            (_, Some("next")) => self.go(Move::Next, cx),
            (_, Some("focus")) => self.toggle_marks(cx),
            (_, Some("zoom")) => self.toggle_zoom(cx),
            (_, Some("peaking")) => self.toggle_peaking(cx),
            (_, Some("undo")) => self.step(false, cx),
            (_, Some("redo")) => self.step(true, cx),
            (_, Some(action)) => match Command::from_action(action) {
                Some(cmd) => self.judge(cmd, cx),
                None => log::debug!("key: {action} is not handled yet"),
            },
            (_, None) => {}
        }
    }

    /// A judgment key on the current file: the view and the strip first, then
    /// the row and the writer in the background. Auto-advance moves on after
    /// the stars and the flags, also when the file already held the value.
    fn judge(&mut self, cmd: Command, cx: &mut Context<Self>) {
        if self.view_only {
            return;
        }
        let Some(path) = self.current_path() else {
            return;
        };
        self.step_note = None;
        let own = self.judgments.get(&path).cloned().unwrap_or_default();
        let after = judgment::apply(&own, &own, cmd);
        let seen = self.seen.contains(&path);
        if judgment::writes(seen, &own, &after, cmd) {
            let label_known = judgment::label_known(seen, &own, &after, cmd);
            let entry = self.undo.record(vec![Judged {
                path: path.clone(),
                state: own.clone(),
            }]);
            self.touched.insert(path.clone());
            self.set_state(&path, after.clone());
            self.refilter(Some(path.clone()), false, None, cx);
            self.store(path.clone(), own, after, label_known, entry, cx);
        }
        let still_current = self.current_path() == Some(path);
        if judgment::should_advance(cmd, self.settings.auto_advance, still_current) {
            self.go(Move::NextClamped, cx);
        }
        cx.notify();
    }

    /// Undo (or with `redo`, redo) the newest judgment through the same
    /// view-row-writer path as a key, moving to its file.
    fn step(&mut self, redo: bool, cx: &mut Context<Self>) {
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
            _ => shown,
        };
        self.refilter(anchor, false, None, cx);
        let verb = if redo { "Redid" } else { "Undid" };
        let note = match stepped.move_to {
            Some(path) => {
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

    fn go(&mut self, to: Move, cx: &mut Context<Self>) {
        let key = Instant::now();
        if self.pager.go(to) {
            self.key = Some(key);
            self.step_note = None;
            self.dwell.cancel();
            self.dwell_timer = None;
            self.remember(cx);
            self.strip
                .scroll_into_view(self.pager.current(), self.strip_width);
            self.strip_moved(cx);
            self.request(key, cx);
            self.read_focus(cx);
            self.zoom_page(cx);
        }
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

    fn strip_clicked(&mut self, x: f32, cx: &mut Context<Self>) {
        if let Some(index) = self.strip.hit_test(x) {
            self.go(Move::To(index), cx);
        }
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
            .map(|index| StripCell {
                x: index as f32 * strip::CELL_PITCH - self.strip.offset(),
                image: self.thumbs.image(index).cloned(),
                name: file_name(&self.files[index]).into(),
                failure: self.strip.is_failed(index).then(|| {
                    self.thumbs
                        .failure(index)
                        .unwrap_or_default()
                        .to_string()
                        .into()
                }),
                current: index == current,
                marks: self.strip.marks(index),
            })
            .collect()
    }

    fn request(&mut self, at: Instant, cx: &mut Context<Self>) {
        if let Some(index) = self.pager.request() {
            self.send(index, at, cx);
        }
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
                        this.retire(Some(loaded.image));
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
        if let Some(current) = self.pager.finish(index) {
            self.send(current, received, cx);
            return;
        }
        let key = self.key.take();
        let name = file_name(&self.files[index]);
        match result {
            Ok(loaded) => {
                let m = &loaded.meta;
                self.status = format!(
                    "{}/{}  {name}  {}x{}  {}  {}  {}  {}",
                    index + 1,
                    self.files.len(),
                    loaded.px.0,
                    loaded.px.1,
                    loaded.orientation,
                    dash(&m.shutter),
                    dash(&m.aperture),
                    dash(&m.iso),
                )
                .into();
                self.shown = Some((index, Shown::Read(Box::new(loaded.meta))));
                let old = self.image.replace(loaded.image);
                self.retire(old);
                self.hold.hold(loaded.paused);
                let path = self.files[index].to_string_lossy().into_owned();
                self.marks.show(Some((path, loaded.orientation)));
                self.zoom.shown(loaded.focus, loaded.orientation);
                self.analyze_current(cx);
                self.start_dwell(cx);
                cx.notify();
                if let Some(key) = key {
                    *self.pending.borrow_mut() = Some(Page {
                        key,
                        sent: self.sent,
                        finished: loaded.finished,
                        received,
                        notified: Instant::now(),
                        read: loaded.read,
                        decode: loaded.decode,
                        orient: loaded.orient,
                        jpeg_kb: loaded.jpeg_kb,
                        px: loaded.px,
                        orientation: loaded.orientation,
                        file: name,
                    });
                }
            }
            Err(e) => {
                self.fail(&format!("{name}: {e}"));
                self.shown = Some((index, Shown::Failed { name, error: e }));
                self.dwell.cancel();
                self.dwell_timer = None;
                let old = self.image.take();
                self.retire(old);
                self.marks.show(None);
                cx.notify();
            }
        }
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

    /// Another file or listing is current: drop the held crop and, zoomed,
    /// read the new one.
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
            Arrived::Failed(e) => {
                let name = file_name(&self.files[self.pager.current()]);
                self.fail(&format!("{name}: {e}"));
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

    /// What the preview canvas paints while zoomed: the crop, over the
    /// scaled preview when the file has a focus point.
    fn zoom_paint(&self) -> Option<zoom::Paint> {
        if !self.zoom.is_on() {
            return None;
        }
        let crop = self.zoom.crop();
        let placeholder = self.image.clone().zip(self.zoom.preview()).and_then(
            |(image, (focus, orientation))| {
                let s = image.size(0);
                let preview = (s.width.0 as u32, s.height.0 as u32);
                let rect = zoom::placeholder(preview, orientation, focus?, crop.map(|c| c.full))?;
                Some((image, rect))
            },
        );
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
        if let Some((at, shown)) = &self.shown {
            let (name, meta, error) = match shown {
                Shown::Read(meta) => (&meta.name, Some(meta.as_ref()), None),
                Shown::Failed { name, error } => (name, None, Some(error)),
            };
            body = body.child(
                div()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(name.clone()))
                    .child(div().text_color(rgb(MUTED_COLOR)).child(format!(
                        "{} / {}",
                        at + 1,
                        self.files.len()
                    ))),
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
                self.notes
                    .iter()
                    .cloned()
                    .chain(self.outputs_failure.clone())
                    .chain(self.write_errors.iter().map(|(path, message)| {
                        format!("{}: {message}", file_name(Path::new(path)))
                    }))
                    .map(|note| div().text_color(rgb(ERROR_COLOR)).child(note)),
            )
    }

    /// A press on a menu's button.
    fn menu_pressed(&mut self, which: Which, at: Point<Pixels>, cx: &mut Context<Self>) {
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

    /// A click on a menu item: the sort or the filter changes for the
    /// session only, and the strip is rebuilt on the current file.
    fn menu_action(&mut self, action: menu::Action, cx: &mut Context<Self>) {
        menu::apply(&mut self.filter, &mut self.sort, action);
        self.menus.acted(action);
        self.refilter(self.current_path(), false, None, cx);
        cx.notify();
    }

    /// The status line with the sort and filter buttons at its right end.
    fn status_row(&self, max_menu_height: Pixels, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
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
                Which::Filter => menu::filter_rows(&self.filter, self.outputs.is_some()),
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
        div()
            .id("menu")
            .occlude()
            .on_mouse_down_out(cx.listener(|this, e: &MouseDownEvent, _, cx| {
                this.menu_dismissed(e, cx);
            }))
            .flex()
            .flex_col()
            .min_w(px(180.))
            .max_h(max_height)
            .overflow_y_scroll()
            .p_1()
            .bg(rgb(PANE_COLOR))
            .border_1()
            .border_color(rgba(BORDER_COLOR))
            .rounded_md()
            .text_size(px(13.))
            .children(
                rows.into_iter()
                    .enumerate()
                    .map(|(at, row)| menu_row(at, row, cx)),
            )
            .into_any_element()
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
        let cells = self.strip_cells();
        let mark = self.marks.mark(self.current_path().as_deref());
        let zoom_paint = self.zoom_paint();
        let viewport = self.viewport.clone();
        let zoom_pending = self.zoom_pending.clone();
        let fit_peaking = self.peaking.paint(View::Fit);
        let this = cx.entity().downgrade();
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x000000))
            .text_color(rgb(0xffffff))
            .track_focus(&self.focus)
            .on_key_down(
                cx.listener(|this, e: &KeyDownEvent, _, cx| this.key_down(&e.keystroke, cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
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
                                    if let Some(page) = page {
                                        let stages =
                                            page.stages(paint_started, paint_finished, presented);
                                        println!("{}", page.line(&stages));
                                        summary.borrow_mut().record(stages);
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
                        .flex_1()
                        .h_full(),
                    )
                    .child(self.meta_pane()),
            )
            .child(
                div()
                    .w_full()
                    .h(px(strip::STRIP_HEIGHT))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, _, cx| {
                            // The strip spans the window from its left edge.
                            this.strip_clicked(f32::from(e.position.x), cx);
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
                    ),
            )
            .child(self.status_row(window.viewport_size().height - px(40.), cx))
    }
}

/// One row of an open menu: an item's check mark, swatch and text, or a
/// section's separator or heading.
fn menu_row(at: usize, row: Row, cx: &mut Context<Riffle>) -> AnyElement {
    let item = match row {
        Row::Separator => {
            return div()
                .h(px(1.))
                .my_1()
                .bg(rgba(BORDER_COLOR))
                .into_any_element();
        }
        Row::Heading(text) => {
            return div()
                .pl(px(MENU_INDENT))
                .py(px(3.))
                .text_size(px(11.))
                .text_color(rgb(MUTED_COLOR))
                .child(text)
                .into_any_element();
        }
        Row::Item(item) => item,
    };
    let dot = |color: u32| div().size(px(7.)).flex_none().rounded_full().bg(rgb(color));
    let lead = match item.lead {
        Lead::None => None,
        Lead::Flag(flag) => Some(dot(match flag {
            Some(filter::FilterFlag::Picked) => PICK_COLOR,
            Some(filter::FilterFlag::Rejected) => REJECT_COLOR,
            _ => MUTED_COLOR,
        })),
        Lead::Label(Some(tint)) => Some(dot(tint_color(tint))),
        Lead::Label(None) => Some(
            div()
                .size(px(7.))
                .flex_none()
                .rounded_full()
                .border_1()
                .border_color(rgba(BORDER_COLOR)),
        ),
        Lead::Face(state) => Some(
            div()
                .size(px(12.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .children(state.map(|state| dot(state.color()).size(px(9.)))),
        ),
    };
    let label = match item.label {
        menu::Label::Text(text) => div().child(text),
        menu::Label::Stars(on) => div()
            .flex()
            .child(
                div()
                    .text_color(rgb(STARS_COLOR))
                    .child("\u{2605}".repeat(on.into())),
            )
            .child(
                div()
                    .text_color(rgba(MUTED_COLOR << 8 | 0x66))
                    .child("\u{2605}".repeat((5 - on).into())),
            ),
    };
    let action = item.action;
    div()
        .id(("menu-item", at))
        .flex()
        .items_center()
        .gap_2()
        .pr_3()
        .py(px(3.))
        .rounded_sm()
        .cursor_pointer()
        .hover(|s| s.bg(rgb(ACCENT_COLOR)))
        .child(
            div()
                .w(px(MENU_INDENT))
                .flex_none()
                .text_center()
                .child(if item.checked { "\u{2713}" } else { "" }),
        )
        .children(lead)
        .child(label)
        .on_click(cx.listener(move |this, _, _, cx| this.menu_action(action, cx)))
        .into_any_element()
}

struct StripCell {
    /// From the strip's left edge.
    x: f32,
    image: Option<Arc<RenderImage>>,
    name: SharedString,
    /// `Some` on a cell the scan failed on, with why.
    failure: Option<SharedString>,
    current: bool,
    marks: Marks,
}

/// In the Tauri info pane's range (`#info`, 220 px).
const PANE_WIDTH: f32 = 220.;
const PANE_COLOR: u32 = 0x171717;
const MUTED_COLOR: u32 = 0xa1a1a1;
const ERROR_COLOR: u32 = 0xff6467;
const PLACEHOLDER: u32 = 0x262626;
const CURRENT_OUTLINE: u32 = 0xe5e5e5;
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
const FLAG_DOT: f32 = 7.;
/// `style.css`'s `--primary`, `--foreground`, `--accent`, `--border` and
/// `--input`, and `.button.outline`'s fill.
const PRIMARY_COLOR: u32 = 0xe5e5e5;
const FOREGROUND_COLOR: u32 = 0xfafafa;
const ACCENT_COLOR: u32 = 0x262626;
const BORDER_COLOR: u32 = 0xffffff1a;
const INPUT_BORDER: u32 = 0xffffff26;
const BUTTON_FILL: u32 = 0xffffff0b;
/// The leading column a checked item's check mark sits in.
const MENU_INDENT: f32 = 20.;

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

fn main() {
    logger::install();
    let mut args = match args::parse(env::args_os().skip(1)) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}\n{}", args::USAGE);
            process::exit(2);
        }
    };
    let index = open_index(args.index.take()).inspect_err(|e| log::warn!("no index: {e}"));
    let settings = settings::load(args.settings.take().as_deref());
    log::info!(
        "settings: sidecarFormat={} autoAdvance={} sortOrder={} filter={:?}",
        settings.format.setting(),
        settings.auto_advance,
        settings.sort.setting(),
        settings.filter
    );
    application().run(move |cx: &mut App| {
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        let bounds = Bounds::centered(None, size(px(1600.), px(1000.)), cx);
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Handpic".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Riffle::start(args, cx);
                    Riffle::new(focus, index, settings, window, cx)
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
