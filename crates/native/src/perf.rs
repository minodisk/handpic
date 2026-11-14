//! The performance overlay's data, a port of `perf.ts`: the CPU and disk
//! samples and the per-switch preview latencies of the last `WINDOW`, each
//! stamped on the main thread's clock. No GPUI types, so `perfgraph.rs` draws
//! `view()`'s output.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use riffle_backend::perf::Sample;

use crate::timing::Stages;

pub const WINDOW: Duration = Duration::from_secs(60);
/// The latency axis never shrinks below this, so a quiet graph does not
/// stretch 20 ms bars to full height.
pub const LATENCY_FLOOR: Duration = Duration::from_millis(500);
/// The read axis never shrinks below this (10 MB/s), so the idle baseline of
/// small reads stays near the bottom instead of filling the plot.
pub const READ_FLOOR_BPS: f64 = 10_000_000.0;

/// A sample as it arrived on the main thread.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stamped {
    pub at: Instant,
    pub sample: Sample,
}

/// One preview switch, presented at `at`; its value is
/// `stages.keypress_to_pixels`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Latency {
    pub at: Instant,
    pub stages: Stages,
}

pub struct View<'a> {
    pub samples: &'a VecDeque<Stamped>,
    pub latency: &'a VecDeque<Latency>,
    pub latency_max: Duration,
    pub read_max: f64,
}

/// A switch's latency runs from the key that turned the page, else from the
/// selection change.
pub fn latency_start(key: Option<Instant>, switched: Instant) -> Instant {
    key.unwrap_or(switched)
}

/// Records only between `start` and `stop`, so nothing accumulates while the
/// overlay is off; `stop` drops what was recorded.
#[derive(Default)]
pub struct Perf {
    recording: bool,
    samples: VecDeque<Stamped>,
    latency: VecDeque<Latency>,
}

impl Perf {
    pub fn on(&self) -> bool {
        self.recording
    }

    pub fn start(&mut self) {
        self.recording = true;
    }

    pub fn stop(&mut self) {
        self.recording = false;
        self.samples.clear();
        self.latency.clear();
    }

    pub fn add_sample(&mut self, sample: Stamped) {
        if !self.recording {
            return;
        }
        self.samples.push_back(sample);
        self.prune(sample.at);
    }

    pub fn add_latency(&mut self, point: Latency) {
        if !self.recording {
            return;
        }
        self.latency.push_back(point);
        self.prune(point.at);
    }

    pub fn view(&mut self, now: Instant) -> View<'_> {
        self.prune(now);
        let latency_max = self
            .latency
            .iter()
            .map(|point| point.stages.keypress_to_pixels)
            .fold(LATENCY_FLOOR, Duration::max);
        let read_max = self
            .samples
            .iter()
            .map(|s| s.sample.read)
            .fold(READ_FLOOR_BPS, f64::max);
        View {
            samples: &self.samples,
            latency: &self.latency,
            latency_max,
            read_max,
        }
    }

    fn prune(&mut self, now: Instant) {
        let Some(from) = now.checked_sub(WINDOW) else {
            return;
        };
        while self.samples.front().is_some_and(|s| s.at < from) {
            self.samples.pop_front();
        }
        while self.latency.front().is_some_and(|p| p.at < from) {
            self.latency.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LATENCY_FLOOR, Latency, Perf, READ_FLOOR_BPS, Stamped, WINDOW, latency_start};
    use crate::timing::Stages;
    use riffle_backend::perf::Sample;
    use std::time::{Duration, Instant};

    const fn millis(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn recording() -> Perf {
        let mut perf = Perf::default();
        perf.start();
        perf
    }

    fn sample(at: Instant, read: f64, disk: Option<f32>) -> Stamped {
        Stamped {
            at,
            sample: Sample {
                process: 1.,
                system: 2.,
                read,
                disk,
            },
        }
    }

    fn stages(total: Duration) -> Stages {
        Stages {
            wait: millis(1),
            read: millis(2),
            decode: millis(3),
            orient: millis(4),
            hop: millis(5),
            frame_wait: millis(6),
            paint: millis(7),
            present: millis(8),
            keypress_to_pixels: total,
        }
    }

    fn latency(at: Instant, total: Duration) -> Latency {
        Latency {
            at,
            stages: stages(total),
        }
    }

    fn sample_ats(perf: &mut Perf, now: Instant) -> Vec<Instant> {
        perf.view(now).samples.iter().map(|s| s.at).collect()
    }

    fn latency_ats(perf: &mut Perf, now: Instant) -> Vec<Instant> {
        perf.view(now).latency.iter().map(|p| p.at).collect()
    }

    fn assert_empty(perf: &mut Perf, now: Instant) {
        let view = perf.view(now);
        assert!(view.samples.is_empty());
        assert!(view.latency.is_empty());
        assert_eq!(view.latency_max, LATENCY_FLOOR);
        assert_eq!(view.read_max, READ_FLOOR_BPS);
    }

    #[test]
    fn records_nothing_until_started() {
        let t = Instant::now();
        let mut perf = Perf::default();
        perf.add_sample(sample(t, 0., None));
        perf.add_latency(latency(t, millis(30)));
        assert!(!perf.on());
        assert_empty(&mut perf, t);
    }

    #[test]
    fn stop_drops_what_was_recorded_and_stops_recording() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_sample(sample(t, 0., None));
        perf.add_latency(latency(t, millis(30)));
        perf.stop();
        perf.add_latency(latency(t + millis(1), millis(40)));
        perf.add_sample(sample(t + millis(1), 0., None));
        assert!(!perf.on());
        assert_empty(&mut perf, t + millis(1));
    }

    #[test]
    fn keeps_points_in_insertion_order() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_latency(latency(t + millis(100), millis(30)));
        perf.add_latency(latency(t + millis(200), millis(10)));
        perf.add_sample(sample(t + millis(150), 0., None));
        perf.add_sample(sample(t + millis(650), 0., None));
        let now = t + millis(700);
        assert_eq!(
            latency_ats(&mut perf, now),
            [t + millis(100), t + millis(200)]
        );
        assert_eq!(
            sample_ats(&mut perf, now),
            [t + millis(150), t + millis(650)]
        );
    }

    #[test]
    fn prunes_points_older_than_the_window_on_insertion() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_sample(sample(t, 0., None));
        perf.add_latency(latency(t + millis(10), millis(900)));
        let late = t + WINDOW + millis(5);
        perf.add_sample(sample(late, 0., None));
        // The view prunes again at the same instant, so look at the deque the
        // insertion left first.
        assert_eq!(
            perf.samples.iter().map(|s| s.at).collect::<Vec<_>>(),
            [late]
        );
        assert_eq!(
            perf.latency.iter().map(|p| p.at).collect::<Vec<_>>(),
            [t + millis(10)]
        );
        assert_eq!(sample_ats(&mut perf, late), [late]);
    }

    #[test]
    fn prunes_on_a_view_with_no_insertion_and_keeps_the_windows_edge() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_latency(latency(t + millis(1_000), millis(30)));
        perf.add_latency(latency(t + millis(2_000), millis(40)));
        assert_eq!(
            latency_ats(&mut perf, t + WINDOW + millis(1_000)),
            [t + millis(1_000), t + millis(2_000)]
        );
        assert_eq!(
            latency_ats(&mut perf, t + WINDOW + millis(1_001)),
            [t + millis(2_000)]
        );
        assert!(latency_ats(&mut perf, t + WINDOW + millis(2_001)).is_empty());
    }

    #[test]
    fn the_latency_axis_maximum_has_a_floor() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_latency(latency(t, millis(20)));
        assert_eq!(perf.view(t).latency_max, LATENCY_FLOOR);
    }

    #[test]
    fn the_latency_axis_maximum_follows_a_point_above_the_floor_until_it_leaves_the_window() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_latency(latency(t, millis(1_200)));
        perf.add_latency(latency(t + millis(10), millis(30)));
        assert_eq!(perf.view(t + millis(10)).latency_max, millis(1_200));
        assert_eq!(perf.view(t + WINDOW + millis(1)).latency_max, LATENCY_FLOOR);
    }

    #[test]
    fn the_latency_axis_maximum_uses_the_total_not_a_stage() {
        let t = Instant::now();
        let mut perf = recording();
        let mut point = latency(t, millis(900));
        point.stages.read = millis(2_000);
        perf.add_latency(point);
        let view = perf.view(t);
        assert_eq!(view.latency_max, millis(900));
        assert_eq!(view.latency[0], point);
    }

    #[test]
    fn the_read_axis_maximum_has_a_floor() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_sample(sample(t, 200_000., None));
        assert_eq!(perf.view(t).read_max, READ_FLOOR_BPS);
    }

    #[test]
    fn the_read_axis_maximum_follows_a_sample_above_the_floor_until_it_leaves_the_window() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_sample(sample(t, 80_000_000., None));
        perf.add_sample(sample(t + millis(10), 1_000., None));
        assert_eq!(perf.view(t + millis(10)).read_max, 80_000_000.);
        assert_eq!(perf.view(t + WINDOW + millis(1)).read_max, READ_FLOOR_BPS);
    }

    #[test]
    fn keeps_a_sample_without_a_disk_busy_reading() {
        let t = Instant::now();
        let mut perf = recording();
        perf.add_sample(sample(t, 0., None));
        perf.add_sample(sample(t + millis(500), 0., Some(42.)));
        let disks: Vec<_> = perf
            .view(t + millis(500))
            .samples
            .iter()
            .map(|s| s.sample.disk)
            .collect();
        assert_eq!(disks, [None, Some(42.)]);
    }

    #[test]
    fn a_latency_starts_at_the_keypress_when_a_key_turned_the_page() {
        let t = Instant::now();
        assert_eq!(latency_start(Some(t), t + millis(5)), t);
    }

    #[test]
    fn a_latency_starts_at_the_selection_change_otherwise() {
        let t = Instant::now();
        assert_eq!(latency_start(None, t + millis(5)), t + millis(5));
    }
}
