use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{
    AppContext, Bounds, Context, Corners, Pixels, RenderImage, Window, point, px, size,
};
use handpic_core::{arw::FocusLocation, partial, reader};
use image::{Frame, RgbaImage};

use crate::{Handpic, guarded, orient, peaking};

/// The largest crop per axis, in JPEG pixels, as the Tauri app's.
pub const CROP_MAX: usize = 1024;

/// A resize streams while the window edge is dragged; the crop is cut again
/// once it has settled this long, as the Tauri app's.
pub const CROP_RESIZE_DEBOUNCE: Duration = Duration::from_millis(150);

/// The crop size in unrotated JPEG pixels for a viewport of `width` x
/// `height` device pixels: each axis is capped, and the two are swapped for
/// the quarter turns, where the viewport's width runs along the JPEG's height.
pub fn crop_size(orientation: u16, width: usize, height: usize) -> (usize, usize) {
    let w = width.clamp(1, CROP_MAX);
    let h = height.clamp(1, CROP_MAX);
    if matches!(orientation, 6 | 8) {
        (h, w)
    } else {
        (w, h)
    }
}

/// A crop to read: for which page, at which viewport in device pixels, the
/// `z` press it answers, if any, and the neighbour it reads ahead, if it is
/// not the current page's.
#[derive(Debug, PartialEq)]
pub struct Request {
    pub seq: u64,
    pub viewport: (u32, u32),
    pub keypress: Option<Instant>,
    pub ahead: Option<String>,
}

/// What a crop that came back asks of the view.
#[derive(Debug, PartialEq)]
pub enum Arrived<C> {
    /// It is the current page's and is now held; `release` is the one it
    /// replaced, `resized` says the viewport changed while it was read, and
    /// `keypress` is a `z` pressed while it was in flight.
    Shown {
        release: Option<C>,
        resized: bool,
        keypress: Option<Instant>,
    },
    /// It is another page's or a neighbour's: held in place of the one in
    /// `release` while zoomed (released itself otherwise, and nothing held
    /// for a failed read), and the current page's read, if zoomed.
    Stale {
        release: Option<C>,
        request: Option<Request>,
    },
    /// The current page's read failed; `release` is a held crop of another
    /// page.
    Failed { error: String, release: Option<C> },
    /// The current page's read failed while not zoomed: nothing to show, and
    /// the next `z` reads it again.
    Missed,
}

/// What `z` asks of the view.
#[derive(Debug, PartialEq)]
pub enum Toggled {
    /// Nothing to read: turned off, no files, or the press waits for the read
    /// in flight.
    Nothing,
    /// The crop held for this page and viewport is shown; the press, for its
    /// `zoom` line.
    Held(Instant),
    Read(Request),
}

#[cfg(test)]
impl Toggled {
    pub fn read(self) -> Option<Request> {
        match self {
            Self::Read(request) => Some(request),
            _ => None,
        }
    }
}

struct Held<C> {
    crop: C,
    /// The page it was read for.
    seq: u64,
    viewport: (u32, u32),
}

/// A neighbour's crop read ahead while zoomed.
struct Ahead<C> {
    crop: C,
    path: String,
    viewport: (u32, u32),
}

/// The read in flight.
struct Flight {
    /// The page it was asked on.
    seq: u64,
    /// Whether it reads a neighbour ahead.
    ahead: bool,
    /// Whether the view was zoomed when it was asked for.
    zoomed: bool,
    /// A re-listing came since: its crop may be of bytes no longer there.
    void: bool,
}

/// The 1:1 view's state, a port of the Tauri frontend's `toggleZoom` /
/// `requestCrop`: one read in flight, the last crop read (the current page's,
/// read ahead once its preview is shown, or, during a held key while zoomed,
/// an earlier page's until the current one arrives), the next neighbour's
/// crop read ahead while zoomed, and the current page's preview facts the
/// placeholder needs.
pub struct Zoom<C> {
    on: bool,
    /// Bumped by every page and listing; a crop carries the one it was read
    /// for.
    seq: u64,
    /// The current page's file, which a neighbour's crop is matched against.
    current: Option<String>,
    in_flight: Option<Flight>,
    held: Option<Held<C>>,
    ahead: Option<Ahead<C>>,
    shown: Option<(u64, Option<FocusLocation>, u16)>,
    /// The page whose last read failed.
    failed: Option<u64>,
    /// The neighbour whose read ahead failed.
    ahead_failed: Option<String>,
    /// A `z` pressed while a read was in flight, with its page.
    waiting: Option<(u64, Instant)>,
}

impl<C> Default for Zoom<C> {
    fn default() -> Self {
        Self {
            on: false,
            seq: 0,
            current: None,
            in_flight: None,
            held: None,
            ahead: None,
            shown: None,
            failed: None,
            ahead_failed: None,
            waiting: None,
        }
    }
}

impl<C> Zoom<C> {
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The held crop, the current page's or another's.
    pub fn crop(&self) -> Option<&C> {
        self.held.as_ref().map(|held| &held.crop)
    }

    /// Whether the current page's placeholder may be drawn: no crop is held,
    /// or the held one is the current page's, so the canvas never mixes two
    /// files.
    pub fn placeholder_fits(&self) -> bool {
        self.held.as_ref().is_none_or(|held| held.seq == self.seq)
    }

    /// Whether the 1:1 view has nothing of the current page to show because
    /// its read failed: zoomed, its last read failed and no crop of it is
    /// held.
    pub fn read_failed(&self) -> bool {
        self.on
            && self.failed == Some(self.seq)
            && self.held.as_ref().is_none_or(|held| held.seq != self.seq)
    }

    fn holds(&self, viewport: (u32, u32)) -> bool {
        self.held
            .as_ref()
            .is_some_and(|held| held.seq == self.seq && held.viewport == viewport)
    }

    /// The current page's preview is on screen, with its focus point and
    /// orientation.
    pub fn shown(&mut self, focus: Option<FocusLocation>, orientation: u16) {
        self.shown = Some((self.seq, focus, orientation));
    }

    /// The current page's focus point and orientation, once its preview is
    /// on screen.
    pub fn preview(&self) -> Option<(Option<FocusLocation>, u16)> {
        self.shown
            .filter(|(seq, _, _)| *seq == self.seq)
            .map(|(_, focus, orientation)| (focus, orientation))
    }

    fn request(
        &mut self,
        viewport: (u32, u32),
        keypress: Option<Instant>,
        ahead: Option<String>,
    ) -> Option<Request> {
        if self.in_flight.is_some() {
            return None;
        }
        self.in_flight = Some(Flight {
            seq: self.seq,
            ahead: ahead.is_some(),
            zoomed: self.on,
            void: false,
        });
        Some(Request {
            seq: self.seq,
            viewport,
            keypress,
            ahead,
        })
    }

    /// The `z` that waited for the read in flight, if it was pressed on the
    /// current page.
    fn take_waiting(&mut self) -> Option<Instant> {
        self.waiting
            .take()
            .filter(|(seq, _)| *seq == self.seq)
            .map(|(_, key)| key)
    }

    /// The current page's read while zoomed without its crop, answering a
    /// waiting `z`.
    fn wanted(&mut self, viewport: (u32, u32)) -> Option<Request> {
        if !self.on || self.holds(viewport) {
            return None;
        }
        let keypress = self.take_waiting();
        self.request(viewport, keypress, None)
    }

    /// `z`. Turning on shows the crop held for this page and viewport, else
    /// waits for the read in flight, else asks for one.
    pub fn toggle(&mut self, has_files: bool, viewport: (u32, u32), now: Instant) -> Toggled {
        if !has_files {
            return Toggled::Nothing;
        }
        self.on = !self.on;
        self.failed = None;
        self.waiting = None;
        if !self.on {
            return Toggled::Nothing;
        }
        if self.holds(viewport) {
            return Toggled::Held(now);
        }
        if self.in_flight.is_some() {
            self.waiting = Some((self.seq, now));
            return Toggled::Nothing;
        }
        self.request(viewport, Some(now), None)
            .map_or(Toggled::Nothing, Toggled::Read)
    }

    /// The read to make ahead of a `z` once the current page's preview is on
    /// screen: not zoomed, the current page's crop; zoomed with it held, the
    /// crop of `next`, the neighbour in the paging direction. A failed read
    /// is not retried until a `z`, a page or a listing.
    pub fn prefetch(&mut self, viewport: (u32, u32), next: Option<&str>) -> Option<Request> {
        if self.in_flight.is_some() || self.preview().is_none() {
            return None;
        }
        if !self.holds(viewport) {
            if self.on || self.failed == Some(self.seq) {
                return None;
            }
            return self.request(viewport, None, None);
        }
        let next = next.filter(|_| self.on)?;
        if self.ahead_failed.as_deref() == Some(next)
            || self
                .ahead
                .as_ref()
                .is_some_and(|ahead| ahead.path == next && ahead.viewport == viewport)
        {
            return None;
        }
        self.request(viewport, None, Some(next.to_owned()))
    }

    /// Another page or listing is current, `current` its file (`None` for an
    /// empty listing, which leaves the view). A neighbour's crop read for it
    /// at this viewport becomes the held one; any other is released. Zoomed,
    /// the held crop otherwise stays on screen until the new page's arrives,
    /// which is asked for; not zoomed, it is released.
    pub fn page(
        &mut self,
        current: Option<&str>,
        viewport: (u32, u32),
    ) -> (Vec<C>, Option<Request>) {
        self.seq += 1;
        self.waiting = None;
        self.current = current.map(str::to_owned);
        if current.is_none() {
            self.on = false;
        }
        let mut release = Vec::new();
        if let Some(ahead) = self.ahead.take() {
            if current == Some(ahead.path.as_str()) && ahead.viewport == viewport {
                let held = Held {
                    crop: ahead.crop,
                    seq: self.seq,
                    viewport,
                };
                release.extend(self.held.replace(held).map(|held| held.crop));
                return (release, None);
            }
            release.push(ahead.crop);
        }
        if !self.on {
            release.extend(self.held.take().map(|held| held.crop));
            return (release, None);
        }
        (release, self.request(viewport, None, None))
    }

    /// A re-listing, which may have changed any file's bytes: the
    /// neighbour's crop goes, and not zoomed, the held crop too; a read in
    /// flight for either is voided.
    pub fn relisted(&mut self) -> Vec<C> {
        let mut release: Vec<C> = self
            .ahead
            .take()
            .map(|ahead| ahead.crop)
            .into_iter()
            .collect();
        self.ahead_failed = None;
        if !self.on {
            release.extend(self.held.take().map(|held| held.crop));
            self.failed = None;
        }
        if let Some(flight) = &mut self.in_flight
            && (flight.ahead || !flight.zoomed || !self.on)
        {
            flight.void = true;
        }
        release
    }

    /// The viewport settled at `viewport`: the neighbour's crop and, not
    /// zoomed, the held crop go when cut for another.
    pub fn fit(&mut self, viewport: (u32, u32)) -> Vec<C> {
        let mut release: Vec<C> = self
            .ahead
            .take_if(|ahead| ahead.viewport != viewport)
            .map(|ahead| ahead.crop)
            .into_iter()
            .collect();
        if !self.on {
            release.extend(
                self.held
                    .take_if(|held| held.viewport != viewport)
                    .map(|held| held.crop),
            );
        }
        release
    }

    /// A read of page `seq` came back; `viewport` is the one now.
    pub fn arrived(
        &mut self,
        seq: u64,
        crop: Result<(C, (u32, u32)), String>,
        viewport: (u32, u32),
    ) -> Arrived<C> {
        let flight = self.in_flight.take();
        let void = flight.as_ref().is_some_and(|flight| flight.void);
        let zoomed = flight.is_some_and(|flight| flight.zoomed);
        if void || seq != self.seq {
            if void || !self.on || !zoomed {
                return Arrived::Stale {
                    release: crop.ok().map(|(crop, _)| crop),
                    request: self.wanted(viewport),
                };
            }
            let release = crop.ok().and_then(|(crop, requested)| {
                self.held
                    .replace(Held {
                        crop,
                        seq,
                        viewport: requested,
                    })
                    .map(|held| held.crop)
            });
            return Arrived::Stale {
                release,
                request: self.wanted(viewport),
            };
        }
        match crop {
            Ok((crop, requested)) => self.hold(crop, requested, viewport),
            Err(error) => {
                self.failed = Some(seq);
                self.waiting = None;
                if !self.on {
                    return Arrived::Missed;
                }
                Arrived::Failed {
                    error,
                    release: if self.placeholder_fits() {
                        None
                    } else {
                        self.held.take().map(|held| held.crop)
                    },
                }
            }
        }
    }

    /// Hold `crop`, cut at `requested`, as the current page's.
    fn hold(&mut self, crop: C, requested: (u32, u32), viewport: (u32, u32)) -> Arrived<C> {
        self.failed = None;
        let release = self
            .held
            .replace(Held {
                crop,
                seq: self.seq,
                viewport: requested,
            })
            .map(|held| held.crop);
        Arrived::Shown {
            release,
            resized: self.on && requested != viewport,
            keypress: self.take_waiting(),
        }
    }

    /// A read ahead of the neighbour `path` came back; `viewport` is the one
    /// now. Paged onto meanwhile, it is the current page's crop; zoomed, it
    /// is held for the next page. A failure shows nothing and is not read
    /// ahead again.
    pub fn ahead_arrived(
        &mut self,
        path: &str,
        crop: Result<(C, (u32, u32)), String>,
        viewport: (u32, u32),
    ) -> Arrived<C> {
        let flight = self.in_flight.take();
        let void = flight.as_ref().is_some_and(|flight| flight.void);
        let asked = flight
            .map(|flight| flight.seq)
            .filter(|seq| *seq != self.seq);
        match crop {
            Ok((crop, requested)) if !void && self.current.as_deref() == Some(path) => {
                self.hold(crop, requested, viewport)
            }
            Ok((crop, requested)) if !void && self.on && asked.is_some() => {
                // The page passed this neighbour while it was read: draw it
                // as a stale crop, as `arrived` does, rather than park it.
                let release = self
                    .held
                    .replace(Held {
                        crop,
                        seq: asked.unwrap_or_default(),
                        viewport: requested,
                    })
                    .map(|held| held.crop);
                Arrived::Stale {
                    release,
                    request: self.wanted(viewport),
                }
            }
            Ok((crop, requested)) if !void && self.on => {
                let release = self
                    .ahead
                    .replace(Ahead {
                        crop,
                        path: path.to_owned(),
                        viewport: requested,
                    })
                    .map(|ahead| ahead.crop);
                Arrived::Stale {
                    release,
                    request: self.wanted(viewport),
                }
            }
            Ok((crop, _)) => Arrived::Stale {
                release: Some(crop),
                request: self.wanted(viewport),
            },
            Err(_) => {
                if !void {
                    self.ahead_failed = Some(path.to_owned());
                }
                Arrived::Stale {
                    release: None,
                    request: self.wanted(viewport),
                }
            }
        }
    }

    /// The viewport changed: the page to re-cut the crop for once it settles,
    /// while zoomed.
    pub fn resized(&self) -> Option<u64> {
        self.on.then_some(self.seq)
    }

    /// The viewport settled after a resize seen on page `seq`.
    pub fn settled(&mut self, seq: u64, viewport: (u32, u32)) -> Option<Request> {
        if !self.on || seq != self.seq || self.holds(viewport) {
            return None;
        }
        self.request(viewport, None, None)
    }
}

/// The point of interest `(x, y)` of a `width` x `height` crop in unrotated
/// JPEG pixels, in the crop `orient::rotate_bgra` turns upright.
pub fn upright_point(orientation: u16, width: u32, height: u32, (x, y): (u32, u32)) -> (u32, u32) {
    match orientation {
        3 => (width - 1 - x, height - 1 - y),
        6 => (height - 1 - y, x),
        8 => (y, width - 1 - x),
        _ => (x, y),
    }
}

/// The device pixel the point of interest lands on: the center of the canvas
/// at `origin` with `size` (logical pixels), floored to a whole device pixel.
pub fn canvas_center(origin: (f32, f32), size: (f32, f32), scale: f32) -> (f32, f32) {
    (
        ((origin.0 + size.0 / 2.) * scale).floor(),
        ((origin.1 + size.1 / 2.) * scale).floor(),
    )
}

/// Where the upright crop's top-left goes, in device pixels, so its point of
/// interest sits on `center`.
pub fn crop_origin(center: (f32, f32), point: (u32, u32)) -> (f32, f32) {
    (center.0 - point.0 as f32, center.1 - point.1 as f32)
}

/// A rectangle around the point of interest, in device pixels: the point is
/// `x` from its left edge and `y` from its top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Where the upright preview stands in for the crop, at the full JPEG's
/// scale, a port of the Tauri `placeholderRect`. `full` is the full JPEG's
/// unrotated size from a crop of this page; without one the full JPEG is
/// taken to be the sensor size. `None` for a focus point without a sensor
/// size.
pub fn placeholder(
    preview: (u32, u32),
    orientation: u16,
    focus: FocusLocation,
    full: Option<(u32, u32)>,
) -> Option<Rect> {
    if focus.sensor_w == 0 || focus.sensor_h == 0 {
        return None;
    }
    let (pw, ph) = if matches!(orientation, 6 | 8) {
        (preview.1, preview.0)
    } else {
        preview
    };
    let (sensor_w, sensor_h) = (f32::from(focus.sensor_w), f32::from(focus.sensor_h));
    let (w, h) = match full {
        Some((w, h)) => (w as f32, h as f32),
        None => (sensor_w, ph as f32 * sensor_w / pw as f32),
    };
    let x = f32::from(focus.x) * w / sensor_w;
    let y = f32::from(focus.y) * h / sensor_h;
    Some(match orientation {
        3 => Rect {
            x: w - x,
            y: h - y,
            width: w,
            height: h,
        },
        6 => Rect {
            x: h - y,
            y: x,
            width: h,
            height: w,
        },
        8 => Rect {
            x: y,
            y: w - x,
            width: h,
            height: w,
        },
        _ => Rect {
            x,
            y,
            width: w,
            height: h,
        },
    })
}

/// One read crop, upright.
pub struct Crop {
    pub image: Arc<RenderImage>,
    /// The point of interest in the upright crop.
    pub point: (u32, u32),
    /// The full JPEG's unrotated size.
    pub full: (u32, u32),
    pub read: Duration,
    pub decode: Duration,
    pub orient: Duration,
}

/// The crop of `path` for a viewport of `viewport` device pixels around its
/// focus point (the frame center without one), turned upright.
pub fn read(path: &Path, viewport: (u32, u32)) -> Result<Crop, String> {
    let started = Instant::now();
    let (arw, jpeg) = reader::read_full(path).map_err(|e| e.to_string())?;
    let read = started.elapsed();
    let (w, h) = crop_size(arw.orientation, viewport.0 as usize, viewport.1 as usize);
    let started = Instant::now();
    let focus =
        partial::decode_focus_crop(&jpeg, arw.shot.focus, w, h).map_err(|e| e.to_string())?;
    let mut pixels = focus.crop.pixels;
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let decode = started.elapsed();
    let started = Instant::now();
    let (cw, ch) = (focus.crop.width as u32, focus.crop.height as u32);
    let point = upright_point(
        arw.orientation,
        cw,
        ch,
        (focus.point_x as u32, focus.point_y as u32),
    );
    let (bgra, uw, uh) = orient::rotate_bgra(pixels, cw, ch, arw.orientation);
    let orient = started.elapsed();
    let buffer = RgbaImage::from_raw(uw, uh, bgra).ok_or("the crop came up short")?;
    Ok(Crop {
        image: Arc::new(RenderImage::new(vec![Frame::new(buffer)])),
        point,
        full: (
            focus.crop.image_width as u32,
            focus.crop.image_height as u32,
        ),
        read,
        decode,
        orient,
    })
}

/// Read the crop `request` asks for on the background executor, then hand
/// it to the view: the neighbour's it reads ahead, else `current`'s.
pub fn send(request: Request, current: PathBuf, cx: &mut Context<Handpic>) {
    let Request {
        seq,
        viewport,
        keypress,
        ahead,
    } = request;
    let path = ahead.as_ref().map_or(current, PathBuf::from);
    cx.spawn(async move |this, cx| {
        let crop = cx
            .background_spawn(async move { guarded(|| read(&path, viewport)) })
            .await;
        this.update(cx, |this, cx| {
            this.crop_arrived(seq, ahead, crop.map(|c| (c, viewport)), keypress, cx);
        })
        .ok();
    })
    .detach();
}

/// The stages of a crop a `z` press shows, printed once it is on screen. A
/// crop read ahead of the press carries the stages of that read.
pub struct Timed {
    pub key: Instant,
    pub read: Duration,
    pub decode: Duration,
    pub orient: Duration,
}

impl Timed {
    pub fn of(key: Instant, crop: &Crop) -> Self {
        Self {
            key,
            read: crop.read,
            decode: crop.decode,
            orient: crop.orient,
        }
    }

    pub fn line(&self, presented: Instant) -> String {
        format!(
            "zoom read={} decode={} orient={} keypressToPixels={}",
            ms(self.read),
            ms(self.decode),
            ms(self.orient),
            ms(presented.saturating_duration_since(self.key)),
        )
    }
}

fn ms(d: Duration) -> String {
    format!("{:.2}ms", d.as_secs_f64() * 1000.0)
}

/// What the preview canvas paints while zoomed.
pub struct Paint {
    pub placeholder: Option<(Arc<RenderImage>, Rect)>,
    pub crop: Option<(Arc<RenderImage>, (u32, u32))>,
    pub peaking: peaking::Paint,
    /// Whether the placeholder and the crop are drawn in grayscale.
    pub grayscale: bool,
}

impl Paint {
    /// The placeholder and the crop over it, one JPEG pixel per device pixel,
    /// the point of interest at the canvas center, clipped to `bounds`, and
    /// the crop's peaking overlay. The crop and its size when the overlay is
    /// to be asked for.
    pub fn paint(
        self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
    ) -> Option<(Arc<RenderImage>, (u32, u32))> {
        let scale = window.scale_factor();
        let center = canvas_center(
            (f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
            (f32::from(bounds.size.width), f32::from(bounds.size.height)),
            scale,
        );
        let at = |x: f32, y: f32, w: f32, h: f32| {
            Bounds::new(
                point(px(x / scale), px(y / scale)),
                size(px(w / scale), px(h / scale)),
            )
        };
        let grayscale = self.grayscale;
        let mut draw = |image: Arc<RenderImage>, target: Bounds<Pixels>| {
            if let Err(e) =
                window.paint_image(bounds, target, Corners::default(), image, 0, grayscale)
            {
                println!("error paint: {e}");
            }
        };
        if let Some((image, rect)) = self.placeholder {
            draw(
                image,
                at(
                    center.0 - rect.x,
                    center.1 - rect.y,
                    rect.width,
                    rect.height,
                ),
            );
        }
        let (image, p) = self.crop?;
        let (x, y) = crop_origin(center, p);
        let s = image.size(0);
        let target = at(x, y, s.width.0 as f32, s.height.0 as f32);
        draw(image.clone(), target);
        let wanted = self.peaking.paint(&image, bounds, target, window)?;
        Some((image, wanted))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use handpic_core::arw::FocusLocation;

    use super::{
        Arrived, CROP_MAX, Rect, Request, Toggled, Zoom, canvas_center, crop_origin, crop_size,
        placeholder, upright_point,
    };
    use crate::orient::rotate_bgra;

    const VIEW: (u32, u32) = (800, 600);
    const A: Option<&str> = Some("a");

    #[test]
    fn a_quarter_turn_swaps_the_viewport_and_the_cap_bounds_each_axis() {
        assert_eq!(crop_size(1, 300, 200), (300, 200));
        assert_eq!(crop_size(6, 300, 200), (200, 300));
        assert_eq!(crop_size(8, 300, 200), (200, 300));
        assert_eq!(crop_size(8, 4000, 2000), (CROP_MAX, CROP_MAX));
        assert_eq!(crop_size(1, 0, 0), (1, 1));
    }

    fn on(z: &mut Zoom<u8>, now: Instant) -> Request {
        z.toggle(true, VIEW, now).read().unwrap()
    }

    #[test]
    fn toggling_on_without_a_crop_asks_for_one_with_the_keypress() {
        let mut z = Zoom::<u8>::default();
        let now = Instant::now();
        assert_eq!(
            z.toggle(true, VIEW, now),
            Toggled::Read(Request {
                seq: 0,
                viewport: VIEW,
                keypress: Some(now),
                ahead: None
            })
        );
        assert!(z.is_on());
    }

    /// A zoom on page `current` whose preview is on screen, not zoomed.
    fn shown_on(current: &str) -> Zoom<u8> {
        let mut z = Zoom::<u8>::default();
        z.page(Some(current), VIEW);
        z.shown(None, 1);
        z
    }

    #[test]
    fn the_current_pages_crop_is_read_ahead_once_its_preview_is_shown() {
        let mut z = Zoom::<u8>::default();
        z.page(A, VIEW);
        assert_eq!(z.prefetch(VIEW, Some("b")), None);
        z.shown(None, 1);
        assert_eq!(
            z.prefetch(VIEW, Some("b")),
            Some(Request {
                seq: 1,
                viewport: VIEW,
                keypress: None,
                ahead: None
            })
        );
        assert_eq!(z.prefetch(VIEW, Some("b")), None);
        assert_eq!(
            z.arrived(1, Ok((7, VIEW)), VIEW),
            Arrived::Shown {
                release: None,
                resized: false,
                keypress: None
            }
        );
        assert!(!z.is_on());
        assert_eq!(z.prefetch(VIEW, Some("b")), None);
    }

    #[test]
    fn z_on_a_crop_read_ahead_shows_it_with_its_press() {
        let mut z = shown_on("a");
        let r = z.prefetch(VIEW, None).unwrap();
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let now = Instant::now();
        assert_eq!(z.toggle(true, VIEW, now), Toggled::Held(now));
        assert!(z.is_on());
        assert_eq!(z.crop(), Some(&7));
    }

    #[test]
    fn z_while_the_read_ahead_is_in_flight_waits_for_it_and_takes_its_press() {
        let mut z = shown_on("a");
        let r = z.prefetch(VIEW, None).unwrap();
        let now = Instant::now();
        assert_eq!(z.toggle(true, VIEW, now), Toggled::Nothing);
        assert!(z.is_on());
        assert_eq!(
            z.arrived(r.seq, Ok((7, VIEW)), VIEW),
            Arrived::Shown {
                release: None,
                resized: false,
                keypress: Some(now)
            }
        );
    }

    #[test]
    fn z_after_a_page_while_an_earlier_read_ahead_is_in_flight_reads_with_its_press() {
        let mut z = shown_on("a");
        let r = z.prefetch(VIEW, None).unwrap();
        z.page(Some("b"), VIEW);
        let now = Instant::now();
        assert_eq!(z.toggle(true, VIEW, now), Toggled::Nothing);
        match z.arrived(r.seq, Ok((7, VIEW)), VIEW) {
            Arrived::Stale { release, request } => {
                assert_eq!(release, Some(7));
                assert_eq!(request.unwrap().keypress, Some(now));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(z.crop(), None);
    }

    #[test]
    fn a_crop_read_ahead_goes_with_a_page_a_listing_or_another_viewport() {
        let read = || {
            let mut z = shown_on("a");
            let r = z.prefetch(VIEW, None).unwrap();
            z.arrived(r.seq, Ok((7, VIEW)), VIEW);
            z
        };

        let mut z = read();
        assert_eq!(z.page(Some("b"), VIEW), (vec![7], None));

        let mut z = read();
        assert_eq!(z.relisted(), vec![7]);
        assert_eq!(z.crop(), None);

        let mut z = read();
        assert!(z.fit(VIEW).is_empty());
        assert_eq!(z.fit((900, 700)), vec![7]);
        assert_eq!(z.crop(), None);
    }

    #[test]
    fn a_read_ahead_in_flight_over_a_listing_is_dropped() {
        let mut z = shown_on("a");
        let r = z.prefetch(VIEW, None).unwrap();
        z.relisted();
        assert_eq!(
            z.arrived(r.seq, Ok((7, VIEW)), VIEW),
            Arrived::Stale {
                release: Some(7),
                request: None
            }
        );
        assert_eq!(z.crop(), None);
        assert!(z.prefetch(VIEW, None).is_some());
    }

    #[test]
    fn a_failed_read_ahead_shows_nothing_and_fails_again_on_z() {
        let mut z = shown_on("a");
        let r = z.prefetch(VIEW, None).unwrap();
        assert_eq!(z.arrived(r.seq, Err("bad".into()), VIEW), Arrived::Missed);
        assert!(!z.read_failed());
        assert_eq!(z.prefetch(VIEW, None), None);
        let r = z.toggle(true, VIEW, Instant::now()).read().unwrap();
        assert_eq!(
            z.arrived(r.seq, Err("bad".into()), VIEW),
            Arrived::Failed {
                error: "bad".into(),
                release: None
            }
        );
        assert!(z.read_failed());
    }

    /// Zoomed on `a` with its crop 7 held and its preview on screen.
    fn zoomed_on_a() -> Zoom<u8> {
        let mut z = shown_on("a");
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        z
    }

    #[test]
    fn zoomed_the_next_neighbours_crop_is_read_ahead_and_a_page_onto_it_shows_it() {
        let mut z = zoomed_on_a();
        let r = z.prefetch(VIEW, Some("b")).unwrap();
        assert_eq!(r.ahead.as_deref(), Some("b"));
        assert_eq!(
            z.ahead_arrived("b", Ok((8, VIEW)), VIEW),
            Arrived::Stale {
                release: None,
                request: None
            }
        );
        assert_eq!(z.prefetch(VIEW, Some("b")), None);
        assert_eq!(z.page(Some("b"), VIEW), (vec![7], None));
        assert_eq!(z.crop(), Some(&8));
        assert!(z.placeholder_fits());
        assert!(!z.read_failed());
    }

    #[test]
    fn zoomed_a_page_elsewhere_drops_the_neighbours_crop_and_reads() {
        let mut z = zoomed_on_a();
        let r = z.prefetch(VIEW, Some("b")).unwrap();
        z.ahead_arrived("b", Ok((8, VIEW)), VIEW);
        let (release, request) = z.page(Some("c"), VIEW);
        assert_eq!(release, vec![8]);
        assert_eq!(request.map(|q| q.seq), Some(r.seq + 1));
        assert_eq!(z.crop(), Some(&7));
    }

    #[test]
    fn zoomed_a_neighbours_crop_for_another_viewport_is_not_used() {
        let mut z = zoomed_on_a();
        z.prefetch(VIEW, Some("b")).unwrap();
        z.ahead_arrived("b", Ok((8, VIEW)), VIEW);
        let (release, request) = z.page(Some("b"), (900, 700));
        assert_eq!(release, vec![8]);
        assert!(request.is_some());
    }

    #[test]
    fn zoomed_a_page_onto_the_neighbour_in_flight_takes_its_crop_when_it_lands() {
        let mut z = zoomed_on_a();
        z.prefetch(VIEW, Some("b")).unwrap();
        assert_eq!(z.page(Some("b"), VIEW), (vec![], None));
        assert_eq!(
            z.ahead_arrived("b", Ok((8, VIEW)), VIEW),
            Arrived::Shown {
                release: Some(7),
                resized: false,
                keypress: None
            }
        );
        assert!(z.placeholder_fits());
    }

    #[test]
    fn zoomed_a_neighbours_crop_landing_after_a_page_elsewhere_asks_for_the_current() {
        let mut z = zoomed_on_a();
        z.prefetch(VIEW, Some("b")).unwrap();
        z.page(Some("c"), VIEW);
        match z.ahead_arrived("b", Ok((8, VIEW)), VIEW) {
            Arrived::Stale { release, request } => {
                assert_eq!(release, Some(7));
                assert_eq!(request.map(|r| r.ahead), Some(None));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(z.crop(), Some(&8));
        assert!(!z.placeholder_fits());
        assert_eq!(z.page(Some("d"), VIEW).0, Vec::<u8>::new());
    }

    #[test]
    fn zoomed_a_failed_neighbour_shows_nothing_and_is_not_read_ahead_again() {
        let mut z = zoomed_on_a();
        z.prefetch(VIEW, Some("b")).unwrap();
        assert_eq!(
            z.ahead_arrived("b", Err("bad".into()), VIEW),
            Arrived::Stale {
                release: None,
                request: None
            }
        );
        assert_eq!(z.prefetch(VIEW, Some("b")), None);
        assert!(!z.read_failed());
        assert!(z.page(Some("b"), VIEW).1.is_some());
    }

    #[test]
    fn zoomed_a_listing_drops_the_neighbours_crop_and_its_read() {
        let mut z = zoomed_on_a();
        z.prefetch(VIEW, Some("b")).unwrap();
        z.ahead_arrived("b", Ok((8, VIEW)), VIEW);
        assert_eq!(z.relisted(), vec![8]);
        assert_eq!(z.crop(), Some(&7));
        z.prefetch(VIEW, Some("b")).unwrap();
        z.relisted();
        assert_eq!(
            z.ahead_arrived("b", Ok((9, VIEW)), VIEW),
            Arrived::Stale {
                release: Some(9),
                request: None
            }
        );
        assert_eq!(z.page(Some("b"), VIEW).1.map(|r| r.ahead), Some(None));
    }

    #[test]
    fn not_zoomed_no_neighbour_is_read_ahead() {
        let mut z = shown_on("a");
        let r = z.prefetch(VIEW, Some("b")).unwrap();
        assert_eq!(r.ahead, None);
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        assert_eq!(z.prefetch(VIEW, Some("b")), None);
    }

    #[test]
    fn toggling_back_on_with_the_same_page_and_viewport_reuses_the_crop() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        assert_eq!(z.toggle(true, VIEW, Instant::now()), Toggled::Nothing);
        assert!(!z.is_on());
        let now = Instant::now();
        assert_eq!(z.toggle(true, VIEW, now), Toggled::Held(now));
        assert_eq!(z.crop(), Some(&7));
        assert_eq!(z.toggle(true, VIEW, Instant::now()), Toggled::Nothing);
        assert!(z.toggle(true, (900, 600), Instant::now()).read().is_some());
    }

    #[test]
    fn a_page_while_zoomed_keeps_the_held_crop_and_asks_for_the_new_page_once() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let (release, request) = z.page(A, VIEW);
        assert!(release.is_empty());
        assert_eq!(
            request,
            Some(Request {
                seq: 1,
                viewport: VIEW,
                keypress: None,
                ahead: None
            })
        );
        assert_eq!(z.crop(), Some(&7));
        assert!(!z.placeholder_fits());
        assert_eq!(z.page(A, VIEW), (vec![], None));
        assert!(z.is_on());
    }

    #[test]
    fn a_held_key_holds_each_stale_crop_and_asks_once_per_arrival() {
        let mut z = Zoom::<u8>::default();
        let mut r = on(&mut z, Instant::now());
        let mut held = None;
        for crop in 1..=4u8 {
            assert_eq!(z.page(A, VIEW), (vec![], None));
            assert_eq!(z.page(A, VIEW), (vec![], None));
            match z.arrived(r.seq, Ok((crop, VIEW)), VIEW) {
                Arrived::Stale { release, request } => {
                    assert_eq!(release, held);
                    r = request.unwrap();
                    assert_eq!(r.seq, u64::from(crop) * 2);
                }
                other => panic!("{other:?}"),
            }
            assert_eq!(z.crop(), Some(&crop));
            assert!(!z.placeholder_fits());
            held = Some(crop);
        }
        assert_eq!(
            z.arrived(r.seq, Ok((9, VIEW)), VIEW),
            Arrived::Shown {
                release: Some(4),
                resized: false,
                keypress: None
            }
        );
        assert!(z.placeholder_fits());
        assert_eq!(z.page(A, VIEW).1.map(|r| r.seq), Some(9));
    }

    #[test]
    fn a_held_key_draws_a_crop_read_ahead_that_the_page_passed() {
        let mut z = zoomed_on_a();
        z.prefetch(VIEW, Some("b")).unwrap();
        z.page(Some("b"), VIEW);
        z.page(Some("c"), VIEW);
        match z.ahead_arrived("b", Ok((8, VIEW)), VIEW) {
            Arrived::Stale { release, request } => {
                assert_eq!(release, Some(7));
                assert_eq!(request.map(|r| r.ahead), Some(None));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(z.crop(), Some(&8));
    }

    #[test]
    fn a_stale_crop_held_does_not_count_as_the_current_pages() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.page(A, VIEW);
        assert!(matches!(
            z.arrived(r.seq, Ok((7, VIEW)), VIEW),
            Arrived::Stale {
                request: Some(_),
                ..
            }
        ));
        z.in_flight = None;
        assert_eq!(z.settled(1, VIEW).map(|r| r.seq), Some(1));
        z.in_flight = None;
        z.toggle(true, VIEW, Instant::now()).read();
        assert_eq!(
            z.toggle(true, VIEW, Instant::now()).read().map(|r| r.seq),
            Some(1)
        );
        assert_eq!(z.crop(), Some(&7));
    }

    #[test]
    fn the_placeholder_fits_with_no_crop_or_the_current_pages() {
        let mut z = Zoom::<u8>::default();
        assert!(z.placeholder_fits());
        let r = on(&mut z, Instant::now());
        assert!(z.placeholder_fits());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        assert!(z.placeholder_fits());
        z.page(A, VIEW);
        assert!(!z.placeholder_fits());
    }

    #[test]
    fn a_stale_failure_holds_nothing_new_and_asks_for_the_current_page() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let r = z.page(A, VIEW).1.unwrap();
        z.page(A, VIEW);
        match z.arrived(r.seq, Err("bad".into()), VIEW) {
            Arrived::Stale { release, request } => {
                assert_eq!(release, None);
                assert_eq!(request.map(|r| r.seq), Some(2));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(z.crop(), Some(&7));
    }

    #[test]
    fn the_current_pages_failure_releases_a_stale_crop() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let r = z.page(A, VIEW).1.unwrap();
        assert_eq!(
            z.arrived(r.seq, Err("bad".into()), VIEW),
            Arrived::Failed {
                error: "bad".into(),
                release: Some(7)
            }
        );
        assert_eq!(z.crop(), None);
    }

    #[test]
    fn the_current_pages_failure_keeps_its_own_crop() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let r = z.settled(z.resized().unwrap(), (900, 700)).unwrap();
        assert_eq!(
            z.arrived(r.seq, Err("bad".into()), VIEW),
            Arrived::Failed {
                error: "bad".into(),
                release: None
            }
        );
        assert_eq!(z.crop(), Some(&7));
    }

    #[test]
    fn a_page_while_not_zoomed_drops_the_held_crop() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        z.toggle(true, VIEW, Instant::now()).read();
        assert_eq!(z.page(A, VIEW), (vec![7], None));
        assert_eq!(z.crop(), None);
        assert!(z.toggle(true, VIEW, Instant::now()).read().is_some());
    }

    #[test]
    fn one_request_is_in_flight_at_a_time() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        assert_eq!(z.page(A, VIEW).1, None);
        assert_eq!(z.page(A, VIEW).1, None);
        match z.arrived(r.seq, Ok((7, VIEW)), VIEW) {
            Arrived::Stale { release, request } => {
                assert_eq!(release, None);
                assert_eq!(request.map(|r| r.seq), Some(2));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(z.page(A, VIEW).1, None);
    }

    #[test]
    fn a_stale_result_while_not_zoomed_is_released_and_asks_for_nothing() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.toggle(true, VIEW, Instant::now()).read();
        z.page(A, VIEW);
        assert_eq!(
            z.arrived(r.seq, Ok((7, VIEW)), VIEW),
            Arrived::Stale {
                release: Some(7),
                request: None
            }
        );
        assert_eq!(z.crop(), None);
        let r = on(&mut z, Instant::now());
        z.page(A, VIEW);
        match z.arrived(r.seq, Err("bad".into()), VIEW) {
            Arrived::Stale { release, request } => {
                assert_eq!(release, None);
                assert_eq!(request.map(|r| r.seq), Some(2));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_current_pages_failure_is_reported() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        assert_eq!(
            z.arrived(r.seq, Err("bad".into()), VIEW),
            Arrived::Failed {
                error: "bad".into(),
                release: None
            }
        );
        assert!(z.page(A, VIEW).1.is_some());
    }

    #[test]
    fn the_current_pages_failure_with_no_crop_of_it_is_a_failed_read() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        assert!(!z.read_failed());
        z.arrived(r.seq, Err("bad".into()), VIEW);
        assert!(z.read_failed());

        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let r = z.page(A, VIEW).1.unwrap();
        z.arrived(r.seq, Err("bad".into()), VIEW);
        assert!(z.read_failed());
    }

    #[test]
    fn a_failed_re_cut_keeps_its_own_crop_and_is_no_failed_read() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let r = z.settled(z.resized().unwrap(), (900, 700)).unwrap();
        z.arrived(r.seq, Err("bad".into()), VIEW);
        assert!(!z.read_failed());
    }

    #[test]
    fn a_failed_read_ends_with_a_page_a_later_read_or_leaving_the_1_1_view() {
        let failed = || {
            let mut z = Zoom::<u8>::default();
            let r = on(&mut z, Instant::now());
            z.arrived(r.seq, Err("bad".into()), VIEW);
            z
        };

        let mut z = failed();
        let r = z.page(A, VIEW).1.unwrap();
        assert!(!z.read_failed());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        assert!(!z.read_failed());

        let mut z = failed();
        let r = z.settled(z.resized().unwrap(), (900, 700)).unwrap();
        assert!(z.read_failed());
        z.arrived(r.seq, Ok((7, (900, 700))), (900, 700));
        assert!(!z.read_failed());

        let mut z = failed();
        z.toggle(true, VIEW, Instant::now()).read();
        assert!(!z.read_failed());
        assert!(z.toggle(true, VIEW, Instant::now()).read().is_some());
        assert!(!z.read_failed());
    }

    #[test]
    fn a_resize_during_a_read_is_noticed_and_re_requested_once_settled() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        let now = (900, 700);
        assert_eq!(z.resized(), Some(0));
        assert_eq!(z.settled(0, now), None);
        assert_eq!(
            z.arrived(r.seq, Ok((7, VIEW)), now),
            Arrived::Shown {
                release: None,
                resized: true,
                keypress: None
            }
        );
        assert_eq!(z.settled(0, now).map(|r| r.viewport), Some(now));
        assert_eq!(
            z.arrived(0, Ok((8, now)), now),
            Arrived::Shown {
                release: Some(7),
                resized: false,
                keypress: None
            }
        );
        assert_eq!(z.settled(0, now), None);
    }

    #[test]
    fn a_settle_for_another_page_or_after_leaving_asks_for_nothing() {
        let mut z = Zoom::<u8>::default();
        assert_eq!(z.resized(), None);
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let seq = z.resized().unwrap();
        let r = z.page(A, VIEW).1.unwrap();
        z.arrived(r.seq, Ok((8, VIEW)), VIEW);
        assert_eq!(z.settled(seq, (900, 700)), None);
        z.toggle(true, VIEW, Instant::now()).read();
        assert_eq!(z.settled(r.seq, (900, 700)), None);
    }

    #[test]
    fn an_empty_listing_leaves_the_view() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        assert_eq!(z.page(None, VIEW), (vec![7], None));
        assert!(!z.is_on());
        assert_eq!(z.toggle(false, VIEW, Instant::now()).read(), None);
        assert!(!z.is_on());
    }

    #[test]
    fn the_preview_facts_belong_to_the_page_they_were_shown_on() {
        let mut z = Zoom::<u8>::default();
        let focus = FocusLocation {
            sensor_w: 6000,
            sensor_h: 4000,
            x: 10,
            y: 20,
        };
        z.shown(Some(focus), 6);
        assert_eq!(z.preview(), Some((Some(focus), 6)));
        z.page(A, VIEW);
        assert_eq!(z.preview(), None);
    }

    #[test]
    fn the_point_of_interest_follows_rotate_bgra() {
        let (w, h) = (5u32, 3u32);
        for orientation in [1u16, 3, 6, 8] {
            for (x, y) in [(0, 0), (1, 2), (4, 1), (3, 0)] {
                let mut buf = vec![0u8; (w * h * 4) as usize];
                buf[((y * w + x) * 4) as usize] = 255;
                let (out, uw, _) = rotate_bgra(buf, w, h, orientation);
                let lit = out.chunks(4).position(|p| p[0] == 255).unwrap() as u32;
                assert_eq!(
                    upright_point(orientation, w, h, (x, y)),
                    (lit % uw, lit / uw),
                    "orientation {orientation} at ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn the_crop_is_placed_with_its_point_on_the_canvas_center_device_pixel() {
        assert_eq!(canvas_center((0., 0.), (1600., 900.), 1.5), (1200., 675.));
        assert_eq!(canvas_center((0., 0.), (1601., 901.), 1.25), (1000., 563.));
        assert_eq!(crop_origin((1200., 675.), (512, 300)), (688., 375.));
        assert_eq!(crop_origin((10., 10.), (512, 300)), (-502., -290.));
    }

    const FOCUS: FocusLocation = FocusLocation {
        sensor_w: 6000,
        sensor_h: 4000,
        x: 1500,
        y: 1000,
    };

    #[test]
    fn the_placeholder_takes_the_sensor_size_until_a_crop_gives_the_full_size() {
        assert_eq!(
            placeholder((1600, 1000), 1, FOCUS, None),
            Some(Rect {
                x: 1500.,
                y: 937.5,
                width: 6000.,
                height: 3750.
            })
        );
        assert_eq!(
            placeholder((1600, 1000), 1, FOCUS, Some((3000, 2000))),
            Some(Rect {
                x: 750.,
                y: 500.,
                width: 3000.,
                height: 2000.
            })
        );
    }

    #[test]
    fn the_placeholder_turns_with_the_orientation() {
        let full = Some((3000, 2000));
        assert_eq!(
            placeholder((1500, 1000), 3, FOCUS, full),
            Some(Rect {
                x: 2250.,
                y: 1500.,
                width: 3000.,
                height: 2000.
            })
        );
        assert_eq!(
            placeholder((1000, 1500), 6, FOCUS, full),
            Some(Rect {
                x: 1500.,
                y: 750.,
                width: 2000.,
                height: 3000.
            })
        );
        assert_eq!(
            placeholder((1000, 1500), 8, FOCUS, full),
            Some(Rect {
                x: 500.,
                y: 2250.,
                width: 2000.,
                height: 3000.
            })
        );
        let no_sensor = FocusLocation {
            sensor_w: 0,
            ..FOCUS
        };
        assert_eq!(placeholder((1000, 1500), 1, no_sensor, None), None);
    }
}
