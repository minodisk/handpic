mod args;
mod orient;
mod pager;
mod timing;

use std::{
    cell::RefCell,
    env, fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    process,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use args::Args;
use gpui::{
    App, Bounds, Context, Corners, FocusHandle, KeyDownEvent, PathPromptOptions, Pixels,
    RenderImage, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions, canvas, div,
    point, prelude::*, px, rgb, size,
};
use gpui_platform::application;
use image::{Frame, RgbaImage};
use pager::{Move, Pager};
use riffle_core::{reader, scan::is_raw_file};
use timing::{Page, Summary};

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

fn load(path: &Path) -> Result<Loaded, String> {
    let started = Instant::now();
    let (arw, jpeg) = reader::read_preview(path).map_err(|e| e.to_string())?;
    let read = started.elapsed();
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

fn list(
    folder: &Path,
    wanted: Option<(u32, u32)>,
    max: Option<usize>,
) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = fs::read_dir(folder)
        .map_err(|e| format!("{}: {e}", folder.display()))?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| is_raw_file(p))
        .collect();
    files.sort_by(|a, b| natord::compare(&file_name(a), &file_name(b)));
    Ok(files
        .into_iter()
        .filter(|p| wanted.is_none_or(|size| guarded(|| preview_size(p)) == Ok(size)))
        .take(max.unwrap_or(usize::MAX))
        .collect())
}

struct Riffle {
    focus: FocusHandle,
    files: Vec<PathBuf>,
    pager: Pager,
    status: SharedString,
    image: Option<Arc<RenderImage>>,
    /// Shown images that the next presented frame no longer uses.
    retired: Rc<RefCell<Vec<Arc<RenderImage>>>>,
    /// The keyed page the next presented frame completes.
    pending: Rc<RefCell<Option<Page>>>,
    summary: Rc<RefCell<Summary>>,
    key: Option<Instant>,
    sent: Instant,
}

impl Riffle {
    fn new(focus: FocusHandle) -> Self {
        Self {
            focus,
            files: Vec::new(),
            pager: Pager::new(0),
            status: "no folder".into(),
            image: None,
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
            if let Some(folder) = folder {
                let listed = folder.clone();
                let listing = cx
                    .background_spawn(async move { list(&listed, size, max) })
                    .await;
                this.update(cx, |this, cx| this.opened(&folder, listing, cx))
                    .ok();
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

    fn opened(
        &mut self,
        folder: &Path,
        listing: Result<Vec<PathBuf>, String>,
        cx: &mut Context<Self>,
    ) {
        match listing {
            Ok(files) if files.is_empty() => {
                self.status = format!("no RAW files in {}", folder.display()).into();
            }
            Ok(files) => {
                self.pager = Pager::new(files.len());
                self.files = files;
                self.request(Instant::now(), cx);
            }
            Err(e) => self.fail(&e),
        }
        cx.notify();
    }

    fn go(&mut self, to: Move, cx: &mut Context<Self>) {
        let key = Instant::now();
        if self.pager.go(to) {
            self.key = Some(key);
            self.request(key, cx);
        }
    }

    fn request(&mut self, at: Instant, cx: &mut Context<Self>) {
        if let Some(index) = self.pager.request() {
            self.send(index, at, cx);
        }
    }

    fn send(&mut self, index: usize, at: Instant, cx: &mut Context<Self>) {
        self.sent = at;
        let path = self.files[index].clone();
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_spawn(async move { guarded(|| load(&path)) })
                .await;
            let received = Instant::now();
            this.update(cx, |this, cx| this.loaded(index, loaded, received, cx))
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
                    "{}/{}  {name}  {}x{}  {}",
                    index + 1,
                    self.files.len(),
                    loaded.px.0,
                    loaded.px.1,
                    loaded.orientation,
                )
                .into();
                let old = self.image.replace(loaded.image);
                self.retire(old);
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
                let old = self.image.take();
                self.retire(old);
                cx.notify();
            }
        }
    }

    fn fail(&mut self, message: &str) {
        println!("error {message}");
        self.status = format!("error: {message}").into();
    }

    fn retire(&self, image: Option<Arc<RenderImage>>) {
        self.retired.borrow_mut().extend(image);
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
            .child(div().px_2().child(self.status.clone()))
    }
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

fn main() {
    let args = match args::parse(env::args_os().skip(1)) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}\n{}", args::USAGE);
            process::exit(2);
        }
    };
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
                    Riffle::new(focus)
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
