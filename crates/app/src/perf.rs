//! The performance overlay's `set_perf_sampling` command over the backend's
//! sampler, emitting each sample as `perf-sample`.

use riffle_backend::perf::Sampler;
use tauri::{AppHandle, Emitter};

/// The backend's sampler, started and stopped by the overlay.
#[derive(Default)]
pub struct AppPerf(Sampler);

/// Start or stop the sampler. Starting while it runs does nothing; stopping
/// only signals the thread, which exits within one period, so the main
/// thread never waits on it.
#[tauri::command]
pub fn set_perf_sampling(app: AppHandle, state: tauri::State<AppPerf>, enabled: bool) {
    if enabled {
        state.0.start(move |sample| app.emit("perf-sample", sample));
    } else {
        state.0.stop();
    }
}
