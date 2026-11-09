//! The performance overlay's CPU and disk sampler: Handpic's process CPU and
//! the system's, each 0–100 %, Handpic's read throughput in bytes per second,
//! and the system's disk busy time, 0–100 %, emitted as `perf-sample` every
//! 500 ms while the overlay is on, and nothing at all while it is off.
//!
//! The read throughput is sysinfo's per-process read counter, which on
//! Windows counts every read the process issues (files, pipes, sockets), so
//! it carries a small non-disk baseline; that is fine for this overlay. The
//! disk busy time is the busiest disk's, `100 - \PhysicalDisk(*)\% Idle Time`
//! from PDH (minimum idle over the instances, `_Total` skipped) on Windows,
//! its `io_ticks` share of the interval on Linux, and `None` elsewhere.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};

const PERIOD: Duration = Duration::from_millis(500);

/// The running sampler's stop flag and thread, if any.
#[derive(Default)]
pub struct AppPerf(Mutex<Option<(Arc<AtomicBool>, JoinHandle<()>)>>);

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Sample {
    pub process: f32,
    pub system: f32,
    /// Handpic's reads over the interval, in bytes per second.
    pub read: f64,
    /// The system's disk busy time, 0–100, `None` where it cannot be read.
    pub disk: Option<f32>,
}

/// Scale a process CPU reading, up to 100 % per logical CPU, to 0–100.
fn normalized(raw: f32, cpus: usize) -> f32 {
    (raw / cpus.max(1) as f32).clamp(0.0, 100.0)
}

/// Bytes read over `elapsed`, per second; nothing elapsed reads as 0.
fn throughput(bytes: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs > 0.0 {
        bytes as f64 / secs
    } else {
        0.0
    }
}

/// The busy share of a `% Idle Time` reading, clamped to 0–100.
#[cfg(any(windows, test))]
fn busy_from_idle(idle: f64) -> f32 {
    (100.0 - idle).clamp(0.0, 100.0) as f32
}

/// The busiest disk's busy share from `(instance, % Idle Time)` pairs,
/// skipping the `_Total` average; `None` when no disk is listed.
#[cfg(any(windows, test))]
fn busiest(idles: &[(String, f64)]) -> Option<f32> {
    idles
        .iter()
        .filter(|(name, _)| name != "_Total")
        .map(|(_, idle)| busy_from_idle(*idle))
        .reduce(f32::max)
}

/// The share of `elapsed` a disk spent on I/O (`io_ticks`, in milliseconds),
/// clamped to 0–100; nothing elapsed reads as 0.
#[cfg(any(target_os = "linux", test))]
fn busy_from_ticks(ticks_ms: u64, elapsed: Duration) -> f32 {
    let ms = elapsed.as_secs_f64() * 1000.0;
    if ms > 0.0 {
        (ticks_ms as f64 / ms * 100.0).clamp(0.0, 100.0) as f32
    } else {
        0.0
    }
}

/// The `io_ticks` field (the 10th) of a `/sys/block/<dev>/stat` line.
#[cfg(any(target_os = "linux", test))]
fn io_ticks(stat: &str) -> Option<u64> {
    stat.split_whitespace().nth(9)?.parse().ok()
}

#[cfg(windows)]
mod disk {
    use windows::core::{w, PCWSTR};
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
        PDH_MORE_DATA,
    };

    /// One PDH query on `\PhysicalDisk(*)\% Idle Time`, closed on drop.
    pub struct DiskBusy {
        query: Option<(PDH_HQUERY, PDH_HCOUNTER)>,
        primed: bool,
        warned: bool,
    }

    impl DiskBusy {
        pub fn new() -> Self {
            let mut busy = Self {
                query: None,
                primed: false,
                warned: false,
            };
            let mut query = PDH_HQUERY::default();
            // SAFETY: the out pointer is a live local.
            let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
            if status != 0 {
                busy.warn(status);
                return busy;
            }
            let mut counter = PDH_HCOUNTER::default();
            // SAFETY: `query` is open and the out pointer is a live local. The
            // English path works whatever the Windows display language.
            let status = unsafe {
                PdhAddEnglishCounterW(query, w!("\\PhysicalDisk(*)\\% Idle Time"), 0, &mut counter)
            };
            if status != 0 {
                // SAFETY: `query` was opened above and is not used again.
                unsafe { PdhCloseQuery(query) };
                busy.warn(status);
                return busy;
            }
            busy.query = Some((query, counter));
            busy
        }

        /// Collect and read the busy share of the interval since the last
        /// call; the first call only primes the rate.
        pub fn read(&mut self) -> Option<f32> {
            let (query, counter) = self.query?;
            // SAFETY: the handles stay valid until drop.
            let status = unsafe { PdhCollectQueryData(query) };
            if status != 0 {
                self.warn(status);
                return None;
            }
            if !std::mem::replace(&mut self.primed, true) {
                return None;
            }
            let mut size = 0u32;
            let mut count = 0u32;
            // SAFETY: a null buffer with a zero size only asks for the size.
            let status = unsafe {
                PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None)
            };
            if status != PDH_MORE_DATA {
                self.warn(status);
                return None;
            }
            // u64 elements keep the buffer aligned for the items.
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            // SAFETY: `buffer` holds at least `size` bytes, aligned for the items.
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    Some(items),
                )
            };
            if status != 0 {
                self.warn(status);
                return None;
            }
            // SAFETY: PDH filled `count` items, and their names point into
            // `buffer`; PDH_FMT_DOUBLE fills the union's `doubleValue`.
            let idles: Vec<(String, f64)> =
                unsafe { std::slice::from_raw_parts(items, count as usize) }
                    .iter()
                    .map(|item| unsafe {
                        (
                            item.szName.to_string().unwrap_or_default(),
                            item.FmtValue.Anonymous.doubleValue,
                        )
                    })
                    .collect();
            super::busiest(&idles)
        }

        fn warn(&mut self, status: u32) {
            if !std::mem::replace(&mut self.warned, true) {
                log::warn!("disk busy PDH call failed: {status:#x}");
            }
        }
    }

    impl Drop for DiskBusy {
        fn drop(&mut self) {
            if let Some((query, _)) = self.query.take() {
                // SAFETY: the query was opened in `new` and closes once.
                unsafe { PdhCloseQuery(query) };
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod disk {
    use std::collections::HashMap;
    use std::time::Instant;

    /// The previous `io_ticks` of each whole physical disk in `/sys/block`.
    pub struct DiskBusy {
        last: Option<(Instant, HashMap<String, u64>)>,
    }

    fn ticks() -> HashMap<String, u64> {
        let Ok(entries) = std::fs::read_dir("/sys/block") else {
            return HashMap::new();
        };
        entries
            .flatten()
            .filter(|entry| entry.path().join("device").exists())
            .filter_map(|entry| {
                let stat = std::fs::read_to_string(entry.path().join("stat")).ok()?;
                let ticks = super::io_ticks(&stat)?;
                Some((entry.file_name().to_string_lossy().into_owned(), ticks))
            })
            .collect()
    }

    impl DiskBusy {
        pub fn new() -> Self {
            Self { last: None }
        }

        /// The busiest disk's share of the interval since the last call; the
        /// first call only records the ticks.
        pub fn read(&mut self) -> Option<f32> {
            let now = Instant::now();
            let ticks = ticks();
            let busy = self.last.as_ref().map(|(at, prev)| {
                let elapsed = now.duration_since(*at);
                ticks
                    .iter()
                    .filter_map(|(name, ticks)| {
                        let delta = ticks.checked_sub(*prev.get(name)?)?;
                        Some(super::busy_from_ticks(delta, elapsed))
                    })
                    .fold(0.0, f32::max)
            });
            self.last = Some((now, ticks));
            busy
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod disk {
    /// No system disk busy reading on this platform.
    pub struct DiskBusy;

    impl DiskBusy {
        pub fn new() -> Self {
            Self
        }

        pub fn read(&mut self) -> Option<f32> {
            None
        }
    }
}

/// Turns raw readings into samples, dropping the first: sysinfo's CPU usage
/// and read bytes are diffs against the previous refresh, so the first has
/// nothing to diff.
#[derive(Default)]
struct Readings {
    primed: bool,
}

impl Readings {
    fn sample(
        &mut self,
        process: f32,
        system: f32,
        cpus: usize,
        read: f64,
        disk: Option<f32>,
    ) -> Option<Sample> {
        if !std::mem::replace(&mut self.primed, true) {
            return None;
        }
        Some(Sample {
            process: normalized(process, cpus),
            system: system.clamp(0.0, 100.0),
            read,
            disk,
        })
    }
}

/// Start or stop the sampler. Starting while it runs does nothing; stopping
/// only signals the thread, which exits within one period, so the main
/// thread never waits on it.
#[tauri::command]
pub fn set_perf_sampling(app: AppHandle, state: tauri::State<AppPerf>, enabled: bool) {
    let mut running = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if !enabled {
        if let Some((stop, _)) = running.take() {
            stop.store(true, Ordering::Relaxed);
        }
        return;
    }
    if running
        .as_ref()
        .is_some_and(|(_, handle)| !handle.is_finished())
    {
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    let handle = std::thread::spawn({
        let stop = stop.clone();
        move || run(&app, &stop)
    });
    *running = Some((stop, handle));
}

fn run(app: &AppHandle, stop: &AtomicBool) {
    let period = PERIOD.max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    let pid = sysinfo::get_current_pid().ok();
    let mut sys = System::new();
    let mut disk = disk::DiskBusy::new();
    let mut readings = Readings::default();
    let mut last = Instant::now();
    let mut warned = false;
    while !stop.load(Ordering::Relaxed) {
        sys.refresh_cpu_usage();
        let (process, read_bytes) = pid.map_or((0.0, 0), |pid| {
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                false,
                ProcessRefreshKind::nothing().with_cpu().with_disk_usage(),
            );
            sys.process(pid)
                .map_or((0.0, 0), |p| (p.cpu_usage(), p.disk_usage().read_bytes))
        });
        let now = Instant::now();
        let read = throughput(read_bytes, now.duration_since(last));
        last = now;
        let busy = disk.read();
        if let Some(sample) = readings.sample(
            process,
            sys.global_cpu_usage(),
            sys.cpus().len(),
            read,
            busy,
        ) {
            if let Err(e) = app.emit("perf-sample", sample) {
                if !std::mem::replace(&mut warned, true) {
                    log::warn!("perf-sample emit failed: {e}");
                }
            }
        }
        std::thread::sleep(period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_scales_by_cpu_count() {
        assert_eq!(normalized(0.0, 8), 0.0);
        assert_eq!(normalized(800.0, 8), 100.0);
        assert_eq!(normalized(200.0, 8), 25.0);
    }

    #[test]
    fn normalized_clamps_at_100() {
        assert_eq!(normalized(1000.0, 8), 100.0);
    }

    #[test]
    fn normalized_with_no_cpus_does_not_divide_by_zero() {
        assert_eq!(normalized(50.0, 0), 50.0);
        assert_eq!(normalized(500.0, 0), 100.0);
    }

    #[test]
    fn throughput_divides_bytes_by_the_elapsed_time() {
        assert_eq!(
            throughput(1_000_000, Duration::from_millis(500)),
            2_000_000.0
        );
        assert_eq!(throughput(0, Duration::from_millis(500)), 0.0);
    }

    #[test]
    fn throughput_with_nothing_elapsed_does_not_divide_by_zero() {
        assert_eq!(throughput(1_000, Duration::ZERO), 0.0);
    }

    #[test]
    fn busy_from_idle_inverts_and_clamps() {
        assert_eq!(busy_from_idle(100.0), 0.0);
        assert_eq!(busy_from_idle(25.0), 75.0);
        assert_eq!(busy_from_idle(120.0), 0.0);
        assert_eq!(busy_from_idle(-5.0), 100.0);
    }

    #[test]
    fn busiest_is_the_least_idle_disk_without_the_total() {
        let idles = [
            ("0 C:".to_string(), 99.0),
            ("1 D:".to_string(), 10.0),
            ("_Total".to_string(), 54.5),
        ];
        assert_eq!(busiest(&idles), Some(90.0));
        assert_eq!(busiest(&[("_Total".to_string(), 50.0)]), None);
        assert_eq!(busiest(&[]), None);
    }

    #[test]
    fn busy_from_ticks_is_the_share_of_the_interval() {
        assert_eq!(busy_from_ticks(250, Duration::from_millis(500)), 50.0);
        assert_eq!(busy_from_ticks(700, Duration::from_millis(500)), 100.0);
        assert_eq!(busy_from_ticks(10, Duration::ZERO), 0.0);
    }

    #[test]
    fn io_ticks_is_the_tenth_field() {
        let stat = "  4370   1203  301838   1934   2711   4221   54888   7349      0   4210   9283";
        assert_eq!(io_ticks(stat), Some(4210));
        assert_eq!(io_ticks("1 2 3"), None);
    }

    #[test]
    fn the_first_reading_is_dropped() {
        let mut readings = Readings::default();
        assert_eq!(readings.sample(400.0, 30.0, 4, 10.0, Some(5.0)), None);
        assert_eq!(
            readings.sample(400.0, 30.0, 4, 2_000.0, Some(40.0)),
            Some(Sample {
                process: 100.0,
                system: 30.0,
                read: 2_000.0,
                disk: Some(40.0),
            })
        );
        assert_eq!(
            readings.sample(100.0, 120.0, 4, 0.0, None),
            Some(Sample {
                process: 25.0,
                system: 100.0,
                read: 0.0,
                disk: None,
            })
        );
    }
}
