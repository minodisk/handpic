mod args;
mod datadir;
mod eyes;
mod facemesh;
mod faces;
mod focus;
mod logger;
mod mark;
mod orient;
mod overlay;
mod pager;
mod resync;
mod scan;
mod strip;
mod thumbs;
mod timing;
mod viewed;

use std::{
    cell::RefCell,
    collections::HashSet,
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
use futures::{
    StreamExt,
    channel::mpsc::{self, UnboundedSender},
};
use gpui::{
    App, Bounds, ContentMask, Context, Corners, FocusHandle, KeyDownEvent, MouseButton,
    MouseDownEvent, PathPromptOptions, Pixels, RenderImage, ScrollEvent, SharedString, Task,
    TextAlign, TitlebarOptions, Window, WindowBounds, WindowOptions, canvas, div, fill, outline,
    point, prelude::*, px, rgb, size,
};
use gpui_platform::application;
use image::{Frame, RgbaImage};
use overlay::Marks;
use pager::{Move, Pager};
use resync::Resync;
use riffle_backend::{
    exif::{Labeled, exif},
    index::{self, FileStat, Index},
    listing::canonicalize,
    watch::{self, Watch},
};
use riffle_core::{
    reader,
    scan::{ReadSlots, ScanFocus},
};
use scan::{Event, Listed, Scan};
use strip::{Answer, Strip};
use thumbs::{MAX_IN_FLIGHT, Read, Thumbs};
use timing::{Page, Summary};
use viewed::{Batch, Batches, Dwell, Latest, VIEW_DWELL_MS};

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
    settings: String,
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

fn label(value: Option<Labeled>) -> String {
    value.map_or_else(|| "-".into(), |v| v.label)
}

fn load(path: &Path, slots: &ReadSlots) -> Result<Loaded, String> {
    let started = Instant::now();
    let paused = slots.pause();
    let preview = reader::read_preview(path);
    drop(paused);
    let (arw, jpeg) = preview.map_err(|e| e.to_string())?;
    let read = started.elapsed();
    let settings = exif(&arw.shot);
    let settings = format!(
        "{}  {}  {}",
        label(settings.shutter),
        label(settings.aperture),
        label(settings.iso),
    );
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
        settings,
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
fn shown(files: &[String], wanted: Option<(u32, u32)>, max: Option<usize>) -> Vec<PathBuf> {
    scan::natural_order(files)
        .into_iter()
        .filter(|p| wanted.is_none_or(|size| guarded(|| preview_size(p)) == Ok(size)))
        .take(max.unwrap_or(usize::MAX))
        .collect()
}

struct Opened {
    listed: Listed,
    shown: Vec<PathBuf>,
    /// `None` without an index.
    todo: Option<Result<Vec<FileStat>, String>>,
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

struct Riffle {
    focus: FocusHandle,
    files: Vec<PathBuf>,
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
    scan: Option<Scan>,
    /// The pending `ScanFocus` update, replaced on every change.
    scan_focus: Option<Task<()>>,
    next_scan: u64,
    /// The open folder's canonical path, the key of its index rows.
    dir: Option<String>,
    /// `--size` and `--max`, which a re-listing applies again.
    wanted: Option<(u32, u32)>,
    max: Option<usize>,
    /// Bumped by every listing, so a preview read for the previous one is
    /// dropped.
    listing: u64,
    watch: Arc<Watch>,
    resync: Resync,
    last_viewed: Latest<(String, String)>,
    viewed: Batches,
    dwell: Dwell,
    /// The running dwell clock; dropping it stops it.
    dwell_timer: Option<Task<()>>,
    events: UnboundedSender<Event>,
    /// `(done, total)` while pass 1 runs.
    progress: Option<(usize, usize)>,
    /// `(done, total)` while pass 2 runs.
    analyzing: Option<(usize, usize)>,
    failed: usize,
    image: Option<Arc<RenderImage>>,
    /// The `f` overlay's toggle and what it draws.
    marks: Marks,
    /// Shown images that the next presented frame no longer uses.
    retired: Rc<RefCell<Vec<Arc<RenderImage>>>>,
    /// The keyed page the next presented frame completes.
    pending: Rc<RefCell<Option<Page>>>,
    summary: Rc<RefCell<Summary>>,
    key: Option<Instant>,
    sent: Instant,
}

impl Riffle {
    fn new(
        focus: FocusHandle,
        index: Result<Indexes, String>,
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
        Self {
            focus,
            files: Vec::new(),
            pager: Pager::new(0),
            status: "no folder".into(),
            index,
            reader,
            strip: Strip::default(),
            thumbs: Thumbs::default(),
            strip_width: f32::from(window.viewport_size().width),
            eviction,
            slots: ReadSlots::new(scan::READ_SLOTS),
            scan: None,
            scan_focus: None,
            next_scan: 0,
            dir: None,
            wanted: None,
            max: None,
            listing: 0,
            watch: Arc::new(watch),
            resync: Resync::default(),
            last_viewed: Latest::default(),
            viewed: Batches::default(),
            dwell: Dwell::default(),
            dwell_timer: None,
            events,
            progress: None,
            analyzing: None,
            failed: 0,
            image: None,
            marks: Marks::default(),
            retired: Rc::default(),
            pending: Rc::default(),
            summary: Rc::default(),
            key: None,
            sent: Instant::now(),
        }
    }

    fn start(args: Args, cx: &mut Context<Self>) {
        let Args {
            folder,
            auto,
            size,
            max,
            index: _,
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
        let eviction = self.eviction.take();
        let watching = self.watch.clone();
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
                    let listed = scan::list(Path::new(&dir))?;
                    let todo = index.map(|index| Scan::prepare(&index, &listed));
                    let (last_viewed, viewed) = match reader {
                        Some(reader) if !carry => remembered(&reader, &listed.dir),
                        _ => Default::default(),
                    };
                    let shown = shown(&listed.files, wanted, max);
                    Ok::<_, String>(Opened {
                        listed,
                        shown,
                        todo,
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
            todo,
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
        self.marks.clear();
        let paths = path_strings(&shown);
        if carry {
            self.carry(&paths);
        } else {
            self.pager = Pager::new(paths.len());
            if let Some(at) = last_viewed.and_then(|last| paths.iter().position(|p| *p == last)) {
                self.pager.go(Move::To(at));
            }
            self.dwell.reset(viewed);
            self.dwell_timer = None;
            self.strip = Strip::new(&paths);
            self.strip
                .scroll_into_view(self.pager.current(), self.strip_width);
            let old = self.thumbs.reset();
            self.retire(old);
        }
        self.files = shown;
        self.dir = Some(listed.dir.clone());
        if carry {
            self.remember(cx);
        }
        self.strip_moved(cx);
        if self.files.is_empty() {
            self.status = format!("no RAW files in {}", listed.dir).into();
            let old = self.image.take();
            self.retire(old);
            self.marks.show(None);
        } else {
            self.request(Instant::now(), cx);
            self.read_focus(cx);
            self.analyze_current(cx);
        }
        match (todo, &self.index) {
            (Some(Ok(todo)), Ok(index)) => {
                let id = self.next_scan;
                self.next_scan += 1;
                self.progress = Some((0, todo.len()));
                self.analyzing = None;
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
                self.scan = Some(scan);
            }
            (Some(Err(e)), _) => {
                log::error!("failed to prepare the scan of {}: {e}", listed.dir);
                self.finished(cx);
            }
            _ => self.finished(cx),
        }
        cx.notify();
    }

    /// Move the pager, the strip and the loaded thumbnails from the shown
    /// files onto their re-listing `paths`: the current file stays current
    /// (or gives way to its nearest listed neighbour) and the strip keeps the
    /// same files in view.
    fn carry(&mut self, paths: &[String]) {
        let previous = path_strings(&self.files);
        let was = self.pager.current();
        let current = resync::carried_current(&previous, paths, was);
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
        if previous.get(was) != paths.get(current) {
            self.dwell.cancel();
            self.dwell_timer = None;
            self.strip.scroll_into_view(current, self.strip_width);
        }
    }

    fn scanned(&mut self, event: Event, cx: &mut Context<Self>) {
        let scan = match &event {
            Event::Progress { scan, .. }
            | Event::Done { scan, .. }
            | Event::FacesProgress { scan, .. }
            | Event::FacesDone { scan, .. } => *scan,
            Event::Changed(dir) => return self.changed(dir, cx),
        };
        if self.scan.as_ref().is_none_or(|s| s.id != scan) {
            return;
        }
        match event {
            Event::Progress {
                done, total, ready, ..
            } => {
                self.progress = Some((done, total));
                self.strip.mark_ready(&ready);
                if self.current_path().is_some_and(|p| ready.contains(&p)) {
                    self.read_focus(cx);
                }
            }
            Event::Done { summary, .. } => {
                self.progress = None;
                self.analyzing = Some((0, 0));
                self.failed = summary.errors;
                self.strip.refresh();
            }
            Event::FacesProgress {
                done, total, ready, ..
            } => {
                self.analyzing = Some((done, total));
                if let Some(current) = self.current_path()
                    && ready.iter().any(|r| r.path == current)
                {
                    self.read_focus(cx);
                }
            }
            Event::FacesDone { .. } => {
                self.analyzing = None;
                self.finished(cx);
            }
            Event::Changed(_) => {}
        }
        self.pump(cx);
        cx.notify();
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
        self.progress.is_some() || self.analyzing.is_some()
    }

    /// Hand the running scan the current file and the strip's range.
    fn focus_scan(&self) {
        if let Some(scan) = &self.scan
            && self.scanning()
        {
            scan.focus(self.scan_focus_paths());
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
            self.dwell.cancel();
            self.dwell_timer = None;
            self.remember(cx);
            self.strip
                .scroll_into_view(self.pager.current(), self.strip_width);
            self.strip_moved(cx);
            self.request(key, cx);
            self.read_focus(cx);
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
        let listing = self.listing;
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_spawn(async move { guarded(|| load(&path, &slots)) })
                .await;
            let received = Instant::now();
            this.update(cx, |this, cx| {
                if listing != this.listing {
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
                self.status = format!(
                    "{}/{}  {name}  {}x{}  {}  {}",
                    index + 1,
                    self.files.len(),
                    loaded.px.0,
                    loaded.px.1,
                    loaded.orientation,
                    loaded.settings,
                )
                .into();
                let old = self.image.replace(loaded.image);
                self.retire(old);
                let path = self.files[index].to_string_lossy().into_owned();
                self.marks.show(Some((path, loaded.orientation)));
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
        if let Err(e) = &self.index {
            status.push_str(&format!("  no index: {e}"));
        }
        if let Some((done, total)) = self.progress {
            status.push_str(&format!("  scanning {done}/{total}"));
        } else if let Some((done, total)) = self.analyzing {
            status.push_str(&format!("  analyzing {done}/{total}"));
        } else if self.failed > 0 {
            status.push_str(&format!("  {} failed", self.failed));
        }
        status.into()
    }

    fn retire(&self, images: impl IntoIterator<Item = Arc<RenderImage>>) {
        self.retired.borrow_mut().extend(images);
    }

    fn quit(&mut self, cx: &mut Context<Self>) {
        println!("{}", self.summary.borrow().table());
        cx.quit();
    }
}

impl Render for Riffle {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let image = self.image.clone();
        let retired = self.retired.clone();
        let pending = self.pending.clone();
        let summary = self.summary.clone();
        let cells = self.strip_cells();
        let mark = self.marks.mark(self.current_path().as_deref());
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x000000))
            .text_color(rgb(0xffffff))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(
                |this, e: &KeyDownEvent, _, cx| match e.keystroke.key.as_str() {
                    "right" => this.go(Move::Next, cx),
                    "left" => this.go(Move::Previous, cx),
                    "home" => this.go(Move::First, cx),
                    "end" => this.go(Move::Last, cx),
                    "f" => this.toggle_marks(cx),
                    "q" => this.quit(cx),
                    _ => {}
                },
            ))
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds: Bounds<Pixels>, _, window, cx| {
                        let page = pending.borrow_mut().take();
                        let paint_started = Instant::now();
                        if let Some(image) = image {
                            let fit = fit_centered(&image, bounds);
                            if let Err(e) =
                                window.paint_image(fit, fit, Corners::default(), image, 0, false)
                            {
                                println!("error paint: {e}");
                            }
                            overlay::paint(mark.as_ref(), fit, window, cx);
                        }
                        let paint_finished = Instant::now();
                        let old = retired.take();
                        if page.is_none() && old.is_empty() {
                            return;
                        }
                        // `defer` runs when the update holding the frame's
                        // draw and present ends.
                        cx.defer(move |cx| {
                            let presented = Instant::now();
                            if let Some(page) = page {
                                let stages = page.stages(paint_started, paint_finished, presented);
                                println!("{}", page.line(&stages));
                                summary.borrow_mut().record(stages);
                            }
                            for image in old {
                                cx.drop_image(image, None);
                            }
                        });
                    },
                )
                .flex_1()
                .w_full(),
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
            .child(div().px_2().child(self.status()))
    }
}

struct StripCell {
    /// From the strip's left edge.
    x: f32,
    image: Option<Arc<RenderImage>>,
    name: SharedString,
    /// `Some` on a cell the scan failed on, with why.
    failure: Option<SharedString>,
    current: bool,
}

const PLACEHOLDER: u32 = 0x262626;
const CURRENT_OUTLINE: u32 = 0xe5e5e5;
const NAME_COLOR: u32 = 0xd4d4d4;
const FAILURE_COLOR: u32 = 0xa3a3a3;

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
                let mut run = window.text_style().to_run(cell.name.len());
                run.color = rgb(NAME_COLOR).into();
                let name =
                    window
                        .text_system()
                        .shape_line(cell.name.clone(), px(11.), &[run], None);
                let at = point(origin.x, image_box.bottom());
                let _ = name.paint(
                    at,
                    px(NAME_HEIGHT),
                    TextAlign::Center,
                    Some(px(CELL_WIDTH)),
                    window,
                    cx,
                );
            },
        );
    }
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
    application().run(move |cx: &mut App| {
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        let bounds = Bounds::centered(None, size(px(1600.), px(1000.)), cx);
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Riffle".into()),
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
                    Riffle::new(focus, index, window, cx)
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
