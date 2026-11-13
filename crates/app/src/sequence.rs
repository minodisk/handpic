//! The folder tree's `Sequence JPEG Timestamps…`: the commands that preview and run
//! the backend's `sequence` on the folder of exported JPEGs right-clicked in the
//! folder tree, and the state of the one run that may be in progress.
//!
//! JPEGs are not indexed, so nothing here touches the index, and a run is not
//! refused while a scan is in progress. The output is written to a sibling of
//! the right-clicked folder, so even when that folder is the open RAW folder
//! the watcher, which watches it non-recursively, sees no `folder-changed` for
//! it.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use riffle_backend::sequence::{self, Failure, Preview};
use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::index;

/// The id of the last run handed out and the cancel flag and join handle of
/// the one run that may be in progress, behind one lock so that the "already
/// running" check and the store are never interleaved between two
/// `sequence_run` calls.
#[derive(Default)]
pub struct Sequences(Mutex<SequenceState>);

#[derive(Default)]
struct SequenceState {
    next_id: u64,
    /// The run in progress, cleared by its own task when it ends.
    running: Option<(u64, Arc<AtomicBool>, tauri::async_runtime::JoinHandle<()>)>,
}

impl SequenceState {
    fn finish(&mut self, run_id: u64) {
        if self
            .running
            .as_ref()
            .is_some_and(|(id, _, _)| *id == run_id)
        {
            self.running = None;
        }
    }
}

const RUN_IN_PROGRESS: &str = "a sequence run is already in progress";

/// Mint a run id and store the task `spawn` starts for it, or refuse while
/// another run is in progress. The lock is held across `spawn`, so the task
/// cannot clear its entry before it is stored.
fn begin(
    sequences: &Sequences,
    spawn: impl FnOnce(u64, Arc<AtomicBool>) -> tauri::async_runtime::JoinHandle<()>,
) -> Result<u64, String> {
    let mut state = index::lock(&sequences.0);
    if state.running.is_some() {
        return Err(RUN_IN_PROGRESS.to_string());
    }
    state.next_id += 1;
    let run_id = state.next_id;
    let cancel = Arc::new(AtomicBool::new(false));
    let handle = spawn(run_id, cancel.clone());
    state.running = Some((run_id, cancel, handle));
    Ok(run_id)
}

/// The times a run on `dir` would write, without touching the disk.
#[tauri::command]
pub async fn sequence_preview(dir: String) -> Result<Preview, String> {
    tauri::async_runtime::spawn_blocking(move || sequence::preview(Path::new(&dir)))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize, Clone)]
struct Progress<'a> {
    run_id: u64,
    dir: &'a str,
    done: usize,
    total: usize,
}

#[derive(Serialize, Clone)]
struct Done<'a> {
    run_id: u64,
    dir: &'a str,
    output_dir: String,
    written: usize,
    total: usize,
    failed: Vec<Failure>,
    canceled: bool,
}

/// Write the sequenced copy of `dir` in the background and return the run's
/// id. Streams `sequence-progress` and ends with one `sequence-done`, which
/// follows every progress event of the run because `run` only returns after
/// its last progress callback. A folder-level error (no JPEGs, an output
/// folder that cannot be rebuilt) arrives as `sequence-done`'s one failure,
/// keyed by `dir`.
#[tauri::command]
pub async fn sequence_run(app: tauri::AppHandle, dir: String) -> Result<u64, String> {
    let sequences = app.state::<Sequences>();
    begin(&sequences, |run_id, cancel| {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let ran = sequence::run(Path::new(&dir), &cancel, |done, total| {
                let _ = app.emit(
                    "sequence-progress",
                    Progress {
                        run_id,
                        dir: &dir,
                        done,
                        total,
                    },
                );
            });
            let done = Done {
                run_id,
                dir: &dir,
                output_dir: ran.output_dir,
                written: ran.written,
                total: ran.total,
                failed: ran.failed,
                canceled: ran.canceled,
            };
            // Cleared before `sequence-done`, so a run started in answer to
            // it is not refused.
            index::lock(&app.state::<Sequences>().0).finish(run_id);
            let _ = app.emit("sequence-done", done);
        })
    })
}

/// Ask the run `run_id` to stop before its next file; a finished or unknown
/// run is ignored.
#[tauri::command]
pub async fn sequence_cancel(app: tauri::AppHandle, run_id: u64) {
    if let Some((id, cancel, _)) = &index::lock(&app.state::<Sequences>().0).running {
        if *id == run_id {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{begin, Sequences, RUN_IN_PROGRESS};
    use crate::index;
    use std::sync::{mpsc, Arc};

    #[test]
    fn a_second_run_is_refused_until_the_first_one_ends() {
        let sequences = Arc::new(Sequences::default());
        let (go, rx) = mpsc::channel::<()>();
        let first = begin(&sequences, |run_id, _| {
            let sequences = sequences.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let _ = rx.recv();
                index::lock(&sequences.0).finish(run_id);
            })
        })
        .unwrap();

        let refused = begin(&sequences, |_, _| unreachable!());
        assert_eq!(refused, Err(RUN_IN_PROGRESS.to_string()));

        let _ = go.send(());
        let waited = tauri::async_runtime::spawn_blocking({
            let sequences = sequences.clone();
            move || {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                while index::lock(&sequences.0).running.is_some()
                    && std::time::Instant::now() < deadline
                {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        });
        tauri::async_runtime::block_on(waited).unwrap();
        assert!(index::lock(&sequences.0).running.is_none());

        let second = begin(&sequences, |_, _| {
            tauri::async_runtime::spawn_blocking(|| {})
        });
        assert_eq!(second, Ok(first + 1));
    }
}
