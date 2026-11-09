use std::time::{Duration, Instant};

/// What one keyed page knows by the time its result is handed to the window.
pub struct Page {
    pub key: Instant,
    pub sent: Instant,
    pub finished: Instant,
    pub received: Instant,
    pub notified: Instant,
    pub read: Duration,
    pub decode: Duration,
    pub orient: Duration,
    pub jpeg_kb: usize,
    pub px: (u32, u32),
    pub orientation: u16,
    pub file: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stages {
    pub wait: Duration,
    pub read: Duration,
    pub decode: Duration,
    pub orient: Duration,
    pub hop: Duration,
    pub frame_wait: Duration,
    pub paint: Duration,
    pub present: Duration,
    pub keypress_to_pixels: Duration,
}

impl Page {
    pub fn stages(
        &self,
        paint_started: Instant,
        paint_finished: Instant,
        presented: Instant,
    ) -> Stages {
        Stages {
            wait: self.sent.saturating_duration_since(self.key),
            read: self.read,
            decode: self.decode,
            orient: self.orient,
            hop: self.received.saturating_duration_since(self.finished),
            frame_wait: paint_started.saturating_duration_since(self.notified),
            paint: paint_finished.saturating_duration_since(paint_started),
            present: presented.saturating_duration_since(paint_finished),
            keypress_to_pixels: presented.saturating_duration_since(self.key),
        }
    }

    pub fn line(&self, s: &Stages) -> String {
        format!(
            "page wait={} read={} decode={} orient={} hop={} frameWait={} paint={} present={} \
             keypressToPixels={} jpegKB={} px={}x{} orientation={} file={}",
            ms(s.wait),
            ms(s.read),
            ms(s.decode),
            ms(s.orient),
            ms(s.hop),
            ms(s.frame_wait),
            ms(s.paint),
            ms(s.present),
            ms(s.keypress_to_pixels),
            self.jpeg_kb,
            self.px.0,
            self.px.1,
            self.orientation,
            self.file,
        )
    }
}

fn ms(d: Duration) -> String {
    format!("{:.2}ms", d.as_secs_f64() * 1000.0)
}

type Column = (&'static str, fn(&Stages) -> Duration);

#[derive(Default)]
pub struct Summary {
    pages: Vec<Stages>,
}

impl Summary {
    pub fn record(&mut self, stages: Stages) {
        self.pages.push(stages);
    }

    pub fn table(&self) -> String {
        let mut table = format!("summary n={}", self.pages.len());
        if self.pages.is_empty() {
            return table;
        }
        let columns: [Column; 9] = [
            ("wait", |s| s.wait),
            ("read", |s| s.read),
            ("decode", |s| s.decode),
            ("orient", |s| s.orient),
            ("hop", |s| s.hop),
            ("frameWait", |s| s.frame_wait),
            ("paint", |s| s.paint),
            ("present", |s| s.present),
            ("keypressToPixels", |s| s.keypress_to_pixels),
        ];
        for (name, stage) in columns {
            let mut values: Vec<Duration> = self.pages.iter().map(stage).collect();
            values.sort();
            let n = values.len();
            let p50 = values[n / 2];
            let p95 = values[((n as f64 * 0.95) as usize).min(n - 1)];
            table.push_str(&format!("\n  {name:<17} p50={} p95={}", ms(p50), ms(p95)));
        }
        table
    }
}

#[cfg(test)]
mod tests {
    use super::{Page, Summary};
    use std::time::{Duration, Instant};

    const fn millis(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn page(key: Instant, sent: Instant) -> Page {
        Page {
            key,
            sent,
            finished: sent + millis(10),
            received: sent + millis(11),
            notified: sent + millis(12),
            read: millis(3),
            decode: millis(6),
            orient: millis(1),
            jpeg_kb: 1234,
            px: (1616, 1080),
            orientation: 6,
            file: "DSC1.ARW".into(),
        }
    }

    #[test]
    fn keypress_to_pixels_is_the_end_to_end_difference_not_the_sum() {
        let key = Instant::now();
        let page = page(key, key + millis(2));
        let s = page.stages(key + millis(20), key + millis(21), key + millis(40));
        assert_eq!(s.wait, millis(2));
        assert_eq!(s.hop, millis(1));
        assert_eq!(s.frame_wait, millis(6));
        assert_eq!(s.paint, millis(1));
        assert_eq!(s.present, millis(19));
        assert_eq!(s.keypress_to_pixels, millis(40));
        let sum =
            s.wait + s.read + s.decode + s.orient + s.hop + s.frame_wait + s.paint + s.present;
        assert_ne!(s.keypress_to_pixels, sum);
    }

    #[test]
    fn wait_is_zero_for_a_lone_press() {
        let key = Instant::now();
        let s = page(key, key).stages(key + millis(20), key + millis(21), key + millis(22));
        assert_eq!(s.wait, Duration::ZERO);
    }

    #[test]
    fn a_press_after_the_request_went_out_does_not_underflow_wait() {
        let sent = Instant::now();
        let s = page(sent + millis(5), sent).stages(
            sent + millis(20),
            sent + millis(21),
            sent + millis(22),
        );
        assert_eq!(s.wait, Duration::ZERO);
    }

    #[test]
    fn the_line_names_every_stage_in_the_documented_order() {
        let key = Instant::now();
        let page = page(key, key);
        let s = page.stages(key + millis(20), key + millis(21), key + millis(22));
        assert_eq!(
            page.line(&s),
            "page wait=0.00ms read=3.00ms decode=6.00ms orient=1.00ms hop=1.00ms \
             frameWait=8.00ms paint=1.00ms present=1.00ms keypressToPixels=22.00ms \
             jpegKB=1234 px=1616x1080 orientation=6 file=DSC1.ARW"
        );
    }

    #[test]
    fn the_summary_takes_p50_and_p95_and_survives_no_pages() {
        let mut summary = Summary::default();
        assert_eq!(summary.table(), "summary n=0");
        let key = Instant::now();
        for n in 1..=20 {
            summary.record(page(key, key).stages(key, key, key + millis(n)));
        }
        let table = summary.table();
        assert!(table.starts_with("summary n=20\n"));
        assert!(table.contains("keypressToPixels  p50=11.00ms p95=20.00ms"));
    }
}
