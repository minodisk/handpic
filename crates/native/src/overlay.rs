//! The `f` overlay over the preview: what it draws for the current file and
//! the painting of the AF focus mark, the detected faces and the face mesh
//! of the eye judgment.

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use gpui::{
    App, AppContext, Bounds, Context, Hsla, PaintQuad, PathBuilder, Pixels, Point, TextAlign,
    Window, fill, hsla, point, px, rgb, size,
};
use riffle_backend::{
    index::{self, Focus, Index},
    ondemand::{self, EyesJudgment, FacesResponse},
};

use crate::{
    Riffle,
    eyes::{EyesCache, EyesTicket},
    facemesh,
    faces::{self, FaceCache},
    file_name, guarded,
    mark::{self, FaceMark, FocusMark},
};

/// The crosshair's arm length and the gap left around the point, as the
/// Tauri app's `FOCUS_MARK_ARM` / `FOCUS_MARK_GAP`.
const ARM: f32 = 8.;
const GAP: f32 = 4.;
const OUTLINE_WIDTH: f32 = 4.;
const LINE_WIDTH: f32 = 2.;
/// The face boxes' color and their landmark dots' radius, as the Tauri app's
/// `FACE_MARK_COLOR` / `FACE_MARK_EYE_RADIUS`.
const FACE_COLOR: u32 = 0x33ffff;
const DOT_RADIUS: f32 = 2.5;
/// The face mesh's color and line widths, as the Tauri app's
/// `FACE_MESH_COLOR` / `FACE_MESH_OUTLINE_WIDTH` / `FACE_MESH_LINE_WIDTH`.
const MESH_COLOR: u32 = 0xff33ff;
const MESH_OUTLINE_WIDTH: f32 = 3.;
const MESH_LINE_WIDTH: f32 = 1.5;
const LABEL_SIZE: f32 = 11.;
/// The offsets of the label's dark copies under it, standing in for the
/// Tauri label's 3 px stroke, which GPUI cannot draw.
const HALO: [(f32, f32); 8] = [
    (-1., -1.),
    (0., -1.),
    (1., -1.),
    (-1., 0.),
    (1., 0.),
    (-1., 1.),
    (0., 1.),
    (1., 1.),
];

fn outline_color() -> Hsla {
    hsla(0., 0., 0., 0.8)
}

/// What the overlay draws on the current file, once its image is shown.
#[derive(Debug, Clone)]
pub struct Mark {
    /// `None` until read, or for a file with no stored focus.
    pub focus: Option<Focus>,
    pub orientation: u16,
    /// `None` until detected, or on a JPEG.
    pub faces: Option<Arc<FacesResponse>>,
    /// `None` until judged, for unknown eyes, or on a JPEG.
    pub eyes: Option<Arc<EyesJudgment>>,
}

/// The toggle, the current file's stored `Focus`, the file the shown image
/// belongs to and the faces detected and eyes judged so far.
#[derive(Default)]
pub struct Marks {
    on: bool,
    /// Bumped by every read, so only the latest read's answer is kept.
    read: u64,
    /// The file a read answered for and its focus, `None` for a file with no
    /// focus (a JPEG, no row).
    focus: Option<(String, Option<Focus>)>,
    /// The file the shown image belongs to and its orientation.
    shown: Option<(String, u16)>,
    faces: FaceCache,
    eyes: EyesCache,
}

impl Marks {
    /// Flip the overlay; whether it is now on. Turning it off drops the read
    /// focus and any read in flight, which nothing refreshes while off, so
    /// turning it on draws nothing until a fresh read answers.
    pub fn toggle(&mut self) -> bool {
        self.on = !self.on;
        if !self.on {
            self.focus = None;
            self.read += 1;
        }
        self.on
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// A read of the current file's focus starts: its ticket.
    pub fn reading(&mut self) -> u64 {
        self.read += 1;
        self.read
    }

    /// A read came back; dropped unless it is the latest.
    pub fn answer(&mut self, ticket: u64, path: String, focus: Option<Focus>) -> bool {
        if ticket != self.read {
            return false;
        }
        self.focus = Some((path, focus));
        true
    }

    /// The image of `path`, turned upright from `orientation`, is shown, or
    /// none is.
    pub fn show(&mut self, shown: Option<(String, u16)>) {
        self.shown = shown;
    }

    /// What to draw while the overlay is on and the shown image belongs to
    /// `current`: the focus once read for `current`, the faces once detected,
    /// the eyes once judged.
    pub fn mark(&self, current: Option<&str>) -> Option<Mark> {
        let current = current.filter(|_| self.on)?;
        let (shown, orientation) = self.shown.as_ref()?;
        (shown == current).then_some(())?;
        let focus = self
            .focus
            .as_ref()
            .filter(|(read, _)| read == current)
            .and_then(|(_, focus)| *focus);
        let raw = is_raw(current);
        let faces = self.faces.get(current).filter(|_| raw).cloned();
        let eyes = self
            .eyes
            .get(current)
            .filter(|_| raw)
            .and_then(|eyes| eyes.clone());
        Some(Mark {
            focus,
            orientation: *orientation,
            faces,
            eyes,
        })
    }

    /// `path`'s eyes as judged so far: `None` not judged yet, `Some(None)`
    /// judged with the eyes unknown.
    pub fn eyes(&self, path: &str) -> Option<Option<Arc<EyesJudgment>>> {
        self.eyes.get(path).cloned()
    }

    /// The file whose faces to detect and the token to settle them with:
    /// `current`, while the overlay is on, its image is shown, it is a RAW
    /// file and it is neither detected nor being detected.
    pub fn wanted_faces(&mut self, current: Option<&str>) -> Option<(String, u64)> {
        let current = current.filter(|_| self.on)?;
        let (shown, _) = self.shown.as_ref()?;
        (shown == current && is_raw(current)).then_some(())?;
        let token = self.faces.request(current)?;
        Some((current.to_owned(), token))
    }

    /// A detection came back; whether it was kept.
    pub fn found_faces(&mut self, path: String, token: u64, faces: FacesResponse) -> bool {
        self.faces.settle(path, token, faces)
    }

    /// The file whose eyes to judge, its ticket and the `current` check that
    /// supersedes it: `current`, under the same gates as `wanted_faces`,
    /// unless it is judged or any judgment is in flight.
    pub fn wanted_eyes(
        &mut self,
        current: Option<&str>,
    ) -> Option<(String, EyesTicket, impl Fn() -> bool + Send + 'static)> {
        let current = current.filter(|_| self.on)?;
        let (shown, _) = self.shown.as_ref()?;
        (shown == current && is_raw(current)).then_some(())?;
        let ticket = self.eyes.request(current)?;
        Some((current.to_owned(), ticket, self.eyes.current(ticket)))
    }

    /// A judgment came back, `None` when superseded; whether it was kept.
    pub fn judged_eyes(
        &mut self,
        path: String,
        ticket: EyesTicket,
        eyes: Option<Option<EyesJudgment>>,
    ) -> bool {
        self.eyes.settle(path, ticket, eyes)
    }

    /// Forget the detected faces and the judged eyes, stopping a judgment in
    /// flight: another folder opened, or the open one was re-listed and its
    /// files may have changed.
    pub fn clear(&mut self) {
        self.faces.clear();
        self.eyes.clear();
    }
}

/// A JPEG folder is view-only: its faces are not detected. A folder lists
/// either its RAW files or its JPEGs, so the file tells the folder.
fn is_raw(path: &str) -> bool {
    riffle_core::scan::is_raw_file(Path::new(path))
}

/// Read `path`'s stored focus through the index's read-only connection on
/// the background executor and hand it to the view, unless `path` is no
/// longer current by then.
pub fn read(reader: Arc<Mutex<Index>>, path: String, ticket: u64, cx: &mut Context<Riffle>) {
    cx.spawn(async move |this, cx| {
        let entry = {
            let path = path.clone();
            cx.background_spawn(async move { index::lock(&reader).entry(&path) })
                .await
        };
        let focus = entry
            .unwrap_or_else(|e| {
                log::warn!("failed to read the focus of {path}: {e}");
                None
            })
            .and_then(|e| e.focus);
        this.update(cx, |this, cx| {
            if this.current_path().as_deref() == Some(path.as_str())
                && this.marks.answer(ticket, path, focus)
            {
                cx.notify();
            }
        })
        .ok();
    })
    .detach();
}

/// Detect `path`'s faces through the backend on the background executor and
/// keep them by path; a failure is logged and kept as no faces. The view
/// repaints only when `path` is still current.
pub fn detect(path: String, token: u64, cx: &mut Context<Riffle>) {
    cx.spawn(async move |this, cx| {
        let found = {
            let path = path.clone();
            cx.background_spawn(async move { guarded(|| ondemand::read_faces(Path::new(&path))) })
                .await
        };
        let found = found.unwrap_or_else(|e| {
            log::warn!("failed to detect the faces of {path}: {e}");
            faces::no_faces()
        });
        this.update(cx, |this, cx| {
            let current = this.current_path().as_deref() == Some(path.as_str());
            if this.marks.found_faces(path, token, found) && current {
                cx.notify();
            }
        })
        .ok();
    })
    .detach();
}

/// Judge `path`'s eyes through the backend on the background executor and
/// log one line of stage timings; a failure is logged and kept as unknown
/// eyes. Once it settles the file current by then is asked for, so a held
/// key never queues a judgment per file passed.
pub fn judge(
    path: String,
    ticket: EyesTicket,
    current: impl Fn() -> bool + Send + 'static,
    cx: &mut Context<Riffle>,
) {
    cx.spawn(async move |this, cx| {
        let started = Instant::now();
        let judged = {
            let path = path.clone();
            cx.background_spawn(async move {
                guarded(|| ondemand::read_eyes(Path::new(&path), current))
            })
            .await
        };
        let eyes = match judged {
            Ok(found) => {
                let ms = |us: u32| f64::from(us) / 1000.;
                log::info!(
                    "eyes {} total={:.1}ms read={:.1}ms decode={:.1}ms detect={:.1}ms model={:.1}ms{}",
                    file_name(Path::new(&path)),
                    started.elapsed().as_secs_f64() * 1000.,
                    ms(found.read_us),
                    ms(found.decode_us),
                    ms(found.detect_us),
                    ms(found.model_us),
                    if found.superseded { " superseded" } else { "" },
                );
                (!found.superseded).then_some(found.eyes)
            }
            Err(e) => {
                log::warn!("failed to judge the eyes of {path}: {e}");
                Some(None)
            }
        };
        this.update(cx, |this, cx| {
            let current = this.current_path().as_deref() == Some(path.as_str());
            if this.marks.judged_eyes(path, ticket, eyes) && current {
                cx.notify();
            }
            this.analyze_current(cx);
        })
        .ok();
    })
    .detach();
}

/// Paint `mark` over the image drawn upright into `image`: the AF mark, the
/// face boxes with their landmark dots, the face mesh of the eye judgment,
/// then the face labels upright.
pub fn paint(mark: Option<&Mark>, image: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let Some(mark) = mark else {
        return;
    };
    let orientation = mark.orientation;
    let (width, height) = (f32::from(image.size.width), f32::from(image.size.height));
    let (draw_width, draw_height) = if matches!(orientation, 6 | 8) {
        (height, width)
    } else {
        (width, height)
    };
    let center = image.center();
    if let Some(focus_mark) = mark::focus_mark(mark.focus.as_ref(), draw_width, draw_height) {
        paint_focus(&focus_mark, orientation, center, window);
    }
    let faces = match mark.faces.as_ref() {
        Some(found) => mark::face_marks(
            &found.faces,
            found.width as f32,
            found.height as f32,
            draw_width,
            draw_height,
            orientation,
        ),
        None => Vec::new(),
    };
    if !faces.is_empty() {
        paint_faces(&faces, orientation, center, window);
    }
    if let Some(eyes) = &mark.eyes {
        paint_mesh(eyes, (draw_width, draw_height), orientation, center, window);
    }
    for face in &faces {
        paint_label(
            face,
            orientation,
            (draw_width, draw_height),
            center,
            window,
            cx,
        );
    }
}

/// The crosshair and the AF frame, stroked first in a dark outline and then
/// in the state color.
fn paint_focus(
    focus_mark: &FocusMark,
    orientation: u16,
    center: Point<Pixels>,
    window: &mut Window,
) {
    let color: Hsla = rgb(focus_mark.state.color()).into();
    for (width, color) in [(OUTLINE_WIDTH, outline_color()), (LINE_WIDTH, color)] {
        let mut path = PathBuilder::stroke(px(width));
        for (from, to) in segments(focus_mark, width) {
            path.move_to(at(from, orientation, center));
            path.line_to(at(to, orientation, center));
        }
        if let Some(rect) = focus_mark.rect {
            path.add_polygon(&corners(rect, orientation, center), true);
        }
        match path.build() {
            Ok(path) => window.paint_path(path, color),
            Err(e) => log::warn!("failed to build the focus mark: {e}"),
        }
    }
}

/// The face boxes, stroked like the AF frame, and a dot on each landmark: a
/// dark ring as wide as the boxes' line around a filled dot.
fn paint_faces(faces: &[FaceMark], orientation: u16, center: Point<Pixels>, window: &mut Window) {
    let color: Hsla = rgb(FACE_COLOR).into();
    for (width, color) in [(OUTLINE_WIDTH, outline_color()), (LINE_WIDTH, color)] {
        let mut path = PathBuilder::stroke(px(width));
        for face in faces {
            path.add_polygon(&corners(face.rect, orientation, center), true);
        }
        match path.build() {
            Ok(path) => window.paint_path(path, color),
            Err(e) => log::warn!("failed to build the face boxes: {e}"),
        }
    }
    let ring = DOT_RADIUS + LINE_WIDTH / 2.;
    for (radius, color) in [(ring, outline_color()), (DOT_RADIUS, color)] {
        for face in faces {
            for p in face.points {
                window.paint_quad(dot(at(p, orientation, center), radius, color));
            }
        }
    }
}

/// The judged face's mesh, its edges stroked like the face boxes but
/// thinner, and for open eyes the iris centers as dots with a dark ring as
/// wide as the lines.
fn paint_mesh(
    eyes: &EyesJudgment,
    (draw_width, draw_height): (f32, f32),
    orientation: u16,
    center: Point<Pixels>,
    window: &mut Window,
) {
    let mesh = &eyes.mesh;
    let points = facemesh::mesh_points(
        &mesh.points,
        mesh.width as f32,
        mesh.height as f32,
        draw_width,
        draw_height,
    );
    let point_at = |i: usize| points.get(i).map(|&p| at(p, orientation, center));
    let (edges, dots) = facemesh::mesh_edges(eyes.state);
    let color: Hsla = rgb(MESH_COLOR).into();
    for (width, color) in [
        (MESH_OUTLINE_WIDTH, outline_color()),
        (MESH_LINE_WIDTH, color),
    ] {
        let mut path = PathBuilder::stroke(px(width));
        for &[a, b] in &edges {
            if let (Some(a), Some(b)) = (point_at(a), point_at(b)) {
                path.move_to(a);
                path.line_to(b);
            }
        }
        match path.build() {
            Ok(path) => window.paint_path(path, color),
            Err(e) => log::warn!("failed to build the face mesh: {e}"),
        }
    }
    let ring = DOT_RADIUS + MESH_LINE_WIDTH / 2.;
    for (radius, color) in [(ring, outline_color()), (DOT_RADIUS, color)] {
        for p in dots.iter().filter_map(|&i| point_at(i)) {
            window.paint_quad(dot(p, radius, color));
        }
    }
}

fn dot(center: Point<Pixels>, radius: f32, color: Hsla) -> PaintQuad {
    let r = px(radius);
    fill(
        Bounds::new(center - point(r, r), size(r * 2., r * 2.)),
        color,
    )
    .corner_radii(r)
}

/// A face's label in 11 px text, upright at `face_label_at`, over dark
/// copies of itself offset by 1 px.
fn paint_label(
    face: &FaceMark,
    orientation: u16,
    (draw_width, draw_height): (f32, f32),
    center: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let shape = |color: Hsla, window: &mut Window| {
        let mut run = window.text_style().to_run(face.label.len());
        run.color = color;
        window
            .text_system()
            .shape_line(face.label.clone().into(), px(LABEL_SIZE), &[run], None)
    };
    let text = shape(rgb(FACE_COLOR).into(), window);
    let halo = shape(outline_color(), window);
    let (x, y) = mark::face_label_at(
        face.rect,
        orientation,
        draw_width,
        draw_height,
        f32::from(text.width),
    );
    let origin = center + point(px(x), px(y));
    let line_height = text.ascent + text.descent;
    for (dx, dy) in HALO {
        let at = origin + point(px(dx), px(dy));
        let _ = halo.paint(at, line_height, TextAlign::Left, None, window, cx);
    }
    let _ = text.paint(origin, line_height, TextAlign::Left, None, window, cx);
}

fn corners(rect: mark::Rect, orientation: u16, center: Point<Pixels>) -> [Point<Pixels>; 4] {
    [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x + rect.width, rect.y + rect.height),
        (rect.x, rect.y + rect.height),
    ]
    .map(|c| at(c, orientation, center))
}

fn at(p: (f32, f32), orientation: u16, center: Point<Pixels>) -> Point<Pixels> {
    let (x, y) = mark::upright_point(p, orientation);
    center + point(px(x), px(y))
}

/// The four crosshair arms on the unrotated drawing. GPUI's stroke has butt
/// caps, so each arm is lengthened by half the stroke `width` at both ends,
/// where the Tauri mark's square caps reach.
fn segments(m: &FocusMark, width: f32) -> [((f32, f32), (f32, f32)); 4] {
    let (x, y, cap) = (m.x, m.y, width / 2.);
    let (near, far) = (GAP - cap, GAP + ARM + cap);
    [
        ((x - far, y), (x - near, y)),
        ((x + near, y), (x + far, y)),
        ((x, y - far), (x, y - near)),
        ((x, y + near), (x, y + far)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mark::MarkState;
    use riffle_backend::index::{FocusPoint, StoredEyes};
    use riffle_core::candidate::FocusCandidate;

    fn focus() -> Focus {
        Focus {
            point: Some(FocusPoint {
                sensor_w: 100,
                sensor_h: 100,
                x: 50,
                y: 50,
                frame: None,
            }),
            manual_focus: false,
            eye_focus: None,
            candidate: FocusCandidate::Unknown,
            eyes: StoredEyes::new(None, None, None, None, None),
        }
    }

    fn shown(path: &str) -> Marks {
        let mut marks = Marks::default();
        marks.toggle();
        marks.show(Some((path.into(), 6)));
        let ticket = marks.reading();
        marks.answer(ticket, path.into(), Some(focus()));
        marks
    }

    #[test]
    fn marks_the_current_file_once_its_image_and_focus_are_in() {
        let marks = shown("a");
        let mark = marks.mark(Some("a"));
        assert_eq!(mark.as_ref().map(|m| m.orientation), Some(6));
        assert!(mark.is_some_and(|m| m.focus.is_some()));
    }

    #[test]
    fn no_mark_while_off() {
        let mut marks = shown("a");
        assert!(!marks.toggle());
        assert!(marks.mark(Some("a")).is_none());
    }

    fn focus_of(marks: &Marks, current: &str) -> Option<Focus> {
        marks.mark(Some(current)).and_then(|m| m.focus)
    }

    #[test]
    fn no_stale_mark_after_toggling_off_and_on() {
        let mut marks = shown("a");
        marks.toggle();
        assert!(marks.toggle());
        assert!(focus_of(&marks, "a").is_none());
        let ticket = marks.reading();
        marks.answer(ticket, "a".into(), Some(focus()));
        assert!(focus_of(&marks, "a").is_some());
    }

    #[test]
    fn no_stale_mark_from_a_read_in_flight_when_toggled_off() {
        let mut marks = shown("a");
        let ticket = marks.reading();
        assert!(!marks.toggle());
        assert!(!marks.answer(ticket, "a".into(), Some(focus())));
        assert!(marks.toggle());
        assert!(focus_of(&marks, "a").is_none());
    }

    #[test]
    fn no_mark_on_the_previous_files_image() {
        let mut marks = shown("a");
        let ticket = marks.reading();
        marks.answer(ticket, "b".into(), Some(focus()));
        assert!(marks.mark(Some("b")).is_none());
        marks.show(Some(("b".into(), 1)));
        assert!(focus_of(&marks, "b").is_some());
        marks.show(None);
        assert!(marks.mark(Some("b")).is_none());
    }

    #[test]
    fn no_mark_before_the_current_files_focus_is_read_or_without_one() {
        let mut marks = shown("a");
        marks.show(Some(("b".into(), 1)));
        assert!(focus_of(&marks, "b").is_none());
        let ticket = marks.reading();
        marks.answer(ticket, "b".into(), None);
        assert!(focus_of(&marks, "b").is_none());
    }

    fn one_face() -> FacesResponse {
        let p = |x: f32, y: f32| ondemand::FacePoint { x, y };
        FacesResponse {
            width: 100,
            height: 100,
            faces: vec![ondemand::FaceBox {
                x: 10.,
                y: 10.,
                width: 20.,
                height: 20.,
                score: 0.9,
                left_eye: p(15., 15.),
                right_eye: p(25., 15.),
                nose: p(20., 20.),
                mouth_left: p(15., 25.),
                mouth_right: p(25., 25.),
            }],
        }
    }

    fn faces_on(marks: &Marks, current: &str) -> Option<usize> {
        marks
            .mark(Some(current))
            .and_then(|m| m.faces)
            .map(|f| f.faces.len())
    }

    #[test]
    fn faces_are_detected_once_for_the_shown_current_raw_file() {
        let mut marks = shown("a.arw");
        assert!(marks.wanted_faces(Some("b.arw")).is_none());
        let (path, token) = marks.wanted_faces(Some("a.arw")).unwrap();
        assert!(marks.wanted_faces(Some("a.arw")).is_none());
        assert!(marks.found_faces(path, token, one_face()));
        assert_eq!(faces_on(&marks, "a.arw"), Some(1));
        assert!(marks.wanted_faces(Some("a.arw")).is_none());
    }

    #[test]
    fn no_faces_are_detected_or_drawn_while_off_or_on_a_jpeg() {
        let mut marks = shown("a.jpg");
        assert!(marks.wanted_faces(Some("a.jpg")).is_none());
        let mut marks = shown("a.arw");
        marks.toggle();
        assert!(marks.wanted_faces(Some("a.arw")).is_none());
        marks.toggle();
        let (path, token) = marks.wanted_faces(Some("a.arw")).unwrap();
        marks.found_faces(path, token, one_face());
        marks.toggle();
        assert!(marks.mark(Some("a.arw")).is_none());
    }

    #[test]
    fn faces_kept_by_path_are_drawn_again_when_the_file_is_shown_again() {
        let mut marks = shown("a.arw");
        let (path, token) = marks.wanted_faces(Some("a.arw")).unwrap();
        marks.show(Some(("b.arw".into(), 1)));
        assert!(marks.found_faces(path, token, one_face()));
        assert_eq!(faces_on(&marks, "b.arw"), None);
        marks.show(Some(("a.arw".into(), 6)));
        assert_eq!(faces_on(&marks, "a.arw"), Some(1));
    }

    #[test]
    fn a_detection_from_before_a_clear_is_dropped_and_the_file_is_detected_again() {
        let mut marks = shown("a.arw");
        let (path, stale) = marks.wanted_faces(Some("a.arw")).unwrap();
        marks.clear();
        assert!(!marks.found_faces(path, stale, one_face()));
        assert_eq!(faces_on(&marks, "a.arw"), None);
        assert!(marks.wanted_faces(Some("a.arw")).is_some());
    }

    fn judgment() -> EyesJudgment {
        EyesJudgment {
            state: "open",
            probability: 0.07,
            ear: 0.3,
            pose: None,
            mesh: ondemand::EyesMesh {
                width: 100,
                height: 100,
                points: vec![[50., 50.]],
            },
        }
    }

    fn eyes_on(marks: &Marks, current: &str) -> Option<&'static str> {
        marks
            .mark(Some(current))
            .and_then(|m| m.eyes)
            .map(|e| e.state)
    }

    #[test]
    fn the_eyes_are_judged_one_file_at_a_time_and_then_the_current_one() {
        let mut marks = shown("a.arw");
        let (path, ticket, _) = marks.wanted_eyes(Some("a.arw")).unwrap();
        marks.show(Some(("b.arw".into(), 1)));
        assert!(marks.wanted_eyes(Some("b.arw")).is_none());
        assert!(marks.judged_eyes(path, ticket, Some(Some(judgment()))));
        assert_eq!(eyes_on(&marks, "b.arw"), None);
        let (path, _, _) = marks.wanted_eyes(Some("b.arw")).unwrap();
        assert_eq!(path, "b.arw");
        marks.show(Some(("a.arw".into(), 6)));
        assert_eq!(eyes_on(&marks, "a.arw"), Some("open"));
    }

    #[test]
    fn no_eyes_are_judged_or_drawn_while_off_or_on_a_jpeg() {
        let mut marks = shown("a.jpg");
        assert!(marks.wanted_eyes(Some("a.jpg")).is_none());
        let mut marks = shown("a.arw");
        marks.toggle();
        assert!(marks.wanted_eyes(Some("a.arw")).is_none());
        marks.toggle();
        let (path, ticket, _) = marks.wanted_eyes(Some("a.arw")).unwrap();
        marks.judged_eyes(path, ticket, Some(Some(judgment())));
        marks.toggle();
        assert!(marks.mark(Some("a.arw")).is_none());
    }

    #[test]
    fn a_clear_stops_the_judgment_in_flight_and_drops_its_answer() {
        let mut marks = shown("a.arw");
        let (path, ticket, still) = marks.wanted_eyes(Some("a.arw")).unwrap();
        marks.clear();
        assert!(!still());
        assert!(!marks.judged_eyes(path, ticket, Some(Some(judgment()))));
        assert_eq!(eyes_on(&marks, "a.arw"), None);
        assert!(marks.wanted_eyes(Some("a.arw")).is_some());
    }

    #[test]
    fn only_the_latest_read_is_kept() {
        let mut marks = shown("a");
        let older = marks.reading();
        let newer = marks.reading();
        let pass2 = Focus {
            candidate: FocusCandidate::NotCandidate,
            ..focus()
        };
        assert!(marks.answer(newer, "a".into(), Some(pass2)));
        assert!(!marks.answer(older, "a".into(), Some(focus())));
        let state = focus_of(&marks, "a").map(|f| mark::af_eye_state(Some(&f)));
        assert_eq!(state, Some(MarkState::Low));
    }

    #[test]
    fn arms_reach_where_square_caps_would() {
        let m = FocusMark {
            x: 0.,
            y: 0.,
            rect: None,
            state: MarkState::Neutral,
        };
        assert_eq!(
            segments(&m, 4.),
            [
                ((-14., 0.), (-2., 0.)),
                ((2., 0.), (14., 0.)),
                ((0., -14.), (0., -2.)),
                ((0., 2.), (0., 14.)),
            ]
        );
    }

    #[test]
    fn a_point_lands_on_the_upright_image_around_its_center() {
        let center = point(px(500.), px(300.));
        assert_eq!(at((10., 20.), 1, center), point(px(510.), px(320.)));
        assert_eq!(at((10., 20.), 6, center), point(px(480.), px(310.)));
    }
}
