//! The performance overlay on `d`, a port of `drawPerf` in the Tauri
//! `main.ts`: the sampler and the redraw it runs while shown (`Overlay`), the
//! graph's pure layout, and `paint`, which draws `perf.rs`'s window as a
//! band along the bottom of the preview canvas.

use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    App, Bounds, ContentMask, Context, Hsla, PathBuilder, Pixels, Point, Task, TextAlign, Window,
    fill, point, px, rgb, rgba, size,
};
use riffle_backend::perf::{Sample, Sampler};

use crate::perf::{Latency, Perf, Stamped, WINDOW};
use crate::timing::Stages;

/// The scrolling time axis redraws on this timer too, so the graph moves
/// between samples and latency points.
const REDRAW: Duration = Duration::from_millis(500);
pub const HEIGHT: f32 = 120.;
const HEADER: f32 = 16.;
const SWATCH: f32 = 14.;
const TEXT_SIZE: f32 = 11.;
/// `style.css`'s `--card` at 0.85 alpha, `--border`, `--muted-foreground`
/// and the `--chart-*` tokens.
const BACKGROUND: u32 = 0x171717d9;
const BORDER: u32 = 0xffffff1a;
const MUTED: u32 = 0xa1a1a1;
const CPU: u32 = 0x60a5fa;
const DISK: u32 = 0x4ade80;
const LATENCY: u32 = 0xf87171;
const DASH: [f32; 2] = [4., 3.];

/// One series: CPU lines are blue and disk lines green; within each pair
/// Handpic's is thick and solid, the system's thin and dashed. `value` is its
/// height as a fraction of the plot, `None` where it has no reading.
pub struct Line {
    pub label: &'static str,
    color: u32,
    width: f32,
    dashed: bool,
    value: fn(&Sample, f64) -> Option<f32>,
}

pub const LINES: [Line; 4] = [
    Line {
        label: "Handpic CPU",
        color: CPU,
        width: 2.,
        dashed: false,
        value: |s, _| Some(s.process / 100.),
    },
    Line {
        label: "System CPU",
        color: CPU,
        width: 1.,
        dashed: true,
        value: |s, _| Some(s.system / 100.),
    },
    Line {
        label: "Handpic disk",
        color: DISK,
        width: 2.,
        dashed: false,
        value: |s, read_max| Some((s.read.min(read_max) / read_max) as f32),
    },
    Line {
        label: "System disk",
        color: DISK,
        width: 1.,
        dashed: true,
        value: |s, _| s.disk.map(|disk| disk / 100.),
    },
];

type Stage = (&'static str, fn(&Stages) -> Duration);

/// The native `page` line's stages, stacked from the bottom in this order.
pub const STAGES: [Stage; 8] = [
    ("wait", |s| s.wait),
    ("read", |s| s.read),
    ("decode", |s| s.decode),
    ("orient", |s| s.orient),
    ("hop", |s| s.hop),
    ("frameWait", |s| s.frame_wait),
    ("paint", |s| s.paint),
    ("present", |s| s.present),
];

/// The x of a point at `at` in a band `width` wide, the newest at the right.
pub fn x_of(at: Instant, now: Instant, width: f32) -> f32 {
    width - now.saturating_duration_since(at).as_secs_f32() / WINDOW.as_secs_f32() * width
}

/// The y of `fraction` of the plot, from the band's top.
pub fn y_of(fraction: f32, bottom: f32, plot: f32) -> f32 {
    bottom - fraction * plot
}

/// One latency stage's step of the latency color, fading up the bar.
pub fn stage_alpha(n: usize) -> f32 {
    1. - n as f32 * 0.11
}

/// Each stage's top and height, stacked up from `bottom` on a
/// `latency_max` scale.
pub fn stack(stages: &Stages, latency_max: Duration, bottom: f32, plot: f32) -> [(f32, f32); 8] {
    let mut top = bottom;
    STAGES.map(|(_, stage)| {
        let height = stage(stages).as_secs_f32() / latency_max.as_secs_f32() * plot;
        top -= height;
        (top, height)
    })
}

/// The series drawn: all but `System disk` when no sample in the window has a
/// disk reading.
pub fn shown<'a>(samples: impl IntoIterator<Item = &'a Stamped>) -> Vec<&'static Line> {
    let disk = samples.into_iter().any(|s| s.sample.disk.is_some());
    LINES
        .iter()
        .filter(|line| line.label != "System disk" || disk)
        .collect()
}

/// The sampler and the redraw, running only while the overlay is shown.
#[derive(Default)]
pub struct Overlay {
    model: Rc<RefCell<Perf>>,
    sampler: Sampler,
    redraw: Option<Task<()>>,
}

impl Overlay {
    pub fn on(&self) -> bool {
        self.model.borrow().on()
    }

    /// The model the paint closure draws and completes latency points into.
    pub fn model(&self) -> Rc<RefCell<Perf>> {
        self.model.clone()
    }

    /// Show or hide the overlay. Showing starts recording, the sampler,
    /// which hands each sample to `send` on its own thread, and the redraw;
    /// hiding stops all three and drops what was recorded.
    pub fn toggle<T: 'static, E: Display>(
        &mut self,
        send: impl FnMut(Sample) -> Result<(), E> + Send + 'static,
        cx: &mut Context<T>,
    ) {
        if self.model.borrow().on() {
            self.model.borrow_mut().stop();
            self.sampler.stop();
            self.redraw = None;
        } else {
            self.model.borrow_mut().start();
            self.sampler.start(send);
            self.redraw = Some(cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(REDRAW).await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }));
        }
        cx.notify();
    }

    /// Record a sample as it reaches the main thread; one arriving after a
    /// close is dropped by the model.
    pub fn add_sample(&self, sample: Sample) {
        self.model.borrow_mut().add_sample(Stamped {
            at: Instant::now(),
            sample,
        });
    }
}

/// Complete a switch's latency point at the presented frame.
pub fn add_latency(model: &RefCell<Perf>, presented: Instant, stages: Stages) {
    model.borrow_mut().add_latency(Latency {
        at: presented,
        stages,
    });
}

/// Draw the graph over the bottom `HEIGHT` px of `bounds`, if the overlay
/// is on: the CPU and disk series on a 0-100 % scale, Handpic's reads on a
/// 0-`read_max` scale and the latency bars on a 0-`latency_max` scale under
/// a one-line legend.
pub fn paint(model: &RefCell<Perf>, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let mut model = model.borrow_mut();
    if !model.on() {
        return;
    }
    let now = Instant::now();
    let view = model.view(now);
    let width = f32::from(bounds.size.width);
    let height = HEIGHT.min(f32::from(bounds.size.height));
    let band = Bounds::new(
        bounds.origin + point(px(0.), bounds.size.height - px(height)),
        size(bounds.size.width, px(height)),
    );
    let at = |x: f32, y: f32| band.origin + point(px(x), px(y));
    let bottom = height - 4.;
    let plot = bottom - HEADER - 4.;
    window.with_content_mask(Some(ContentMask { bounds: band }), |window| {
        window.paint_quad(fill(band, rgba(BACKGROUND)));
        for fraction in [0., 0.5, 1.] {
            let y = y_of(fraction, bottom, plot).round();
            window.paint_quad(fill(
                Bounds::new(at(0., y), size(px(width), px(1.))),
                rgba(BORDER),
            ));
        }
        for point in view.latency {
            let x = x_of(point.at, now, width) - 1.;
            for (n, (top, height)) in stack(&point.stages, view.latency_max, bottom, plot)
                .into_iter()
                .enumerate()
            {
                window.paint_quad(fill(
                    Bounds::new(at(x, top), size(px(2.), px(height))),
                    alpha(LATENCY, stage_alpha(n)),
                ));
            }
        }
        let lines = shown(view.samples.iter());
        for line in &lines {
            let mut path = builder(line);
            let mut segments = 0;
            let mut drawing = false;
            for s in view.samples {
                let Some(value) = (line.value)(&s.sample, view.read_max) else {
                    drawing = false;
                    continue;
                };
                let p = at(x_of(s.at, now, width), y_of(value, bottom, plot));
                if drawing {
                    path.line_to(p);
                    segments += 1;
                } else {
                    path.move_to(p);
                }
                drawing = true;
            }
            if segments > 0 {
                stroke(path, line.color, window);
            }
        }
        let mut left = 6.;
        let entries = [("100%  60 s".to_string(), MUTED, None, 1.)]
            .into_iter()
            .chain(
                lines
                    .iter()
                    .map(|line| (line.label.to_string(), line.color, Some(*line), 1.)),
            )
            .chain([("Latency".to_string(), LATENCY, None, 1.)])
            .chain(
                STAGES
                    .iter()
                    .enumerate()
                    .map(|(n, (name, _))| (name.to_string(), LATENCY, None, stage_alpha(n))),
            );
        for (text, color, line, opacity) in entries {
            if let Some(line) = line {
                let mut path = builder(line);
                path.move_to(at(left, 9.5));
                path.line_to(at(left + SWATCH, 9.5));
                stroke(path, line.color, window);
                left += SWATCH + 4.;
            }
            left += paint_text(&text, alpha(color, opacity), at(left, 3.), window, cx) + 12.;
        }
        let max = format!(
            "{} MB/s  {} ms",
            (view.read_max / 1_000_000.).round(),
            (view.latency_max.as_secs_f64() * 1000.).round()
        );
        let text = shape(&max, rgb(MUTED).into(), window);
        let line_height = text.ascent + text.descent;
        let x = width - 6. - f32::from(text.width);
        let _ = text.paint(at(x, 3.), line_height, TextAlign::Left, None, window, cx);
    });
}

fn alpha(color: u32, opacity: f32) -> Hsla {
    let mut color: Hsla = rgb(color).into();
    color.a = opacity;
    color
}

fn builder(line: &Line) -> PathBuilder {
    let path = PathBuilder::stroke(px(line.width));
    if line.dashed {
        path.dash_array(&DASH.map(px))
    } else {
        path
    }
}

fn stroke(path: PathBuilder, color: u32, window: &mut Window) {
    match path.build() {
        Ok(path) => window.paint_path(path, rgb(color)),
        Err(e) => log::warn!("failed to build a performance line: {e}"),
    }
}

fn shape(text: &str, color: Hsla, window: &mut Window) -> gpui::ShapedLine {
    let mut run = window.text_style().to_run(text.len());
    run.color = color;
    window
        .text_system()
        .shape_line(text.to_string().into(), px(TEXT_SIZE), &[run], None)
}

/// Paint `text` with its top-left at `origin`; its width.
fn paint_text(
    text: &str,
    color: Hsla,
    origin: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let line = shape(text, color, window);
    let line_height = line.ascent + line.descent;
    let _ = line.paint(origin, line_height, TextAlign::Left, None, window, cx);
    f32::from(line.width)
}

#[cfg(test)]
mod tests {
    use super::{LINES, STAGES, shown, stack, stage_alpha, x_of, y_of};
    use crate::perf::{Stamped, WINDOW};
    use crate::timing::Stages;
    use riffle_backend::perf::Sample;
    use std::time::{Duration, Instant};

    const fn millis(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn stamped(at: Instant, disk: Option<f32>) -> Stamped {
        Stamped {
            at,
            sample: Sample {
                process: 50.,
                system: 25.,
                read: 0.,
                disk,
            },
        }
    }

    #[test]
    fn the_newest_point_is_at_the_right_edge_and_the_window_start_at_the_left() {
        let now = Instant::now() + WINDOW;
        assert_eq!(x_of(now, now, 600.), 600.);
        assert_eq!(x_of(now - millis(30_000), now, 600.), 300.);
        assert_eq!(x_of(now - WINDOW, now, 600.), 0.);
    }

    #[test]
    fn a_point_after_now_stays_at_the_right_edge() {
        let now = Instant::now();
        assert_eq!(x_of(now + millis(5), now, 600.), 600.);
    }

    #[test]
    fn y_runs_from_the_bottom_to_the_top_of_the_plot() {
        assert_eq!(y_of(0., 116., 96.), 116.);
        assert_eq!(y_of(0.5, 116., 96.), 68.);
        assert_eq!(y_of(1., 116., 96.), 20.);
    }

    #[test]
    fn the_stages_stack_from_the_bottom_in_page_line_order() {
        let stages = Stages {
            wait: millis(10),
            read: millis(20),
            decode: millis(30),
            orient: millis(40),
            hop: millis(50),
            frame_wait: millis(60),
            paint: millis(70),
            present: millis(80),
            keypress_to_pixels: millis(400),
        };
        let stacked = stack(&stages, millis(1_000), 116., 100.);
        assert_eq!(stacked[0], (115., 1.));
        assert_eq!(stacked[1], (113., 2.));
        let (top, height) = stacked[7];
        assert!((top - 80.).abs() < 1e-4);
        assert!((height - 8.).abs() < 1e-4);
        assert_eq!(
            STAGES.map(|(name, _)| name),
            [
                "wait",
                "read",
                "decode",
                "orient",
                "hop",
                "frameWait",
                "paint",
                "present"
            ]
        );
    }

    #[test]
    fn each_stage_fades_by_a_step() {
        assert_eq!(stage_alpha(0), 1.);
        assert!((stage_alpha(7) - 0.23).abs() < 1e-6);
    }

    #[test]
    fn the_system_disk_shows_only_with_a_reading_in_the_window() {
        let t = Instant::now();
        let without = [stamped(t, None), stamped(t + millis(500), None)];
        let labels = |lines: Vec<&super::Line>| lines.iter().map(|l| l.label).collect::<Vec<_>>();
        assert_eq!(
            labels(shown(without.iter())),
            ["Handpic CPU", "System CPU", "Handpic disk"]
        );
        let with = [stamped(t, None), stamped(t + millis(500), Some(3.))];
        assert_eq!(labels(shown(with.iter())), LINES.map(|l| l.label));
    }

    #[test]
    fn the_read_line_is_clamped_to_the_read_axis_and_a_missing_disk_breaks_its_line() {
        let mut sample = stamped(Instant::now(), None).sample;
        sample.read = 30_000_000.;
        assert_eq!((LINES[2].value)(&sample, 20_000_000.), Some(1.));
        assert_eq!((LINES[2].value)(&sample, 60_000_000.), Some(0.5));
        assert_eq!((LINES[0].value)(&sample, 1.), Some(0.5));
        assert_eq!((LINES[3].value)(&sample, 1.), None);
    }
}
