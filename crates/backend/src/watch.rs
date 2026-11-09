//! Watching the open folder so files copied in or deleted outside the app
//! show up without waiting for the window to regain focus.
//!
//! The events are only a trigger: what changed is always re-derived by the
//! frontend's rescan (a fresh listing plus `scan_folder`), never read out of
//! the event. A burst is collapsed into one `on_changed` call, made
//! `DEBOUNCE` after the last event of the burst.
//!
//! The app's own sidecar writes cannot loop back in: `triggers` drops an event
//! whose paths are all sidecars or write temporaries. Even unfiltered they
//! would only cost a no-op rescan, since `reconcile_sidecars` reparses a
//! sidecar only when its stat differs from the one `mark_written` stored.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use riffle_core::copy::TEMP_SUFFIX;

use crate::sidecar::SidecarFormat;

/// How long after the last event of a burst the rescan is triggered. Long
/// enough that copying a batch of files in one go rescans once, short enough
/// that a single new file shows up in about a second.
const DEBOUNCE: Duration = Duration::from_millis(500);

/// The folder currently watched and the watcher holding it. Managed state, one
/// per app.
pub struct Watch(Mutex<State>);

struct State {
    dir: Option<String>,
    watcher: Option<RecommendedWatcher>,
    tx: Sender<String>,
    /// The caller's string for the watched folder, shared with the running
    /// watcher's event handler so that updating it (the "same folder,
    /// different caller string" case in `set`) does not require dropping and
    /// re-creating the watcher.
    owner: Arc<Mutex<String>>,
}

impl Watch {
    /// Start the debounce thread. It ends when the app drops the state and
    /// with it the sender; nothing is pending on disk, so quitting needs no
    /// drain of its own.
    pub fn spawn(on_changed: impl Fn(String) + Send + 'static) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            run(rx, move |dir| on_changed(dir.to_string()));
        });
        Self(Mutex::new(State {
            dir: None,
            watcher: None,
            tx,
            owner: Arc::new(Mutex::new(String::new())),
        }))
    }
}

/// Watch `dir` (the canonicalized path `notify` needs), replacing a watch on
/// another folder and leaving one on the same folder alone. `owner` is
/// what `on_changed` gets instead of `dir`: it is the caller's own
/// string for the folder (what the frontend holds as `openDir`), which can
/// differ from the canonicalized `dir` (a verbatim `\\?\` path on Windows, a
/// resolved symlink on macOS) and would otherwise never match the frontend's
/// comparison. Failing to watch (a network volume, say) is logged and
/// ignored: the rescan on focus and `File > Reload Folder` is the fallback,
/// and the folder must still open.
///
/// `owner` is refreshed even on the "same folder" early return: the same
/// canonical `dir` can be reopened under a different caller string (a
/// trailing slash, `/tmp/x` vs `/private/tmp/x`, a stale recent-folders
/// entry), and the watcher must keep passing on the *current* string rather
/// than the one it started with. The refresh goes through a shared cell so
/// it does not require dropping and re-creating the watcher (and, on
/// Windows, the directory handle that would cost).
pub fn set(watch: &Watch, dir: &str, owner: &str) {
    let mut state = crate::index::lock(&watch.0);
    *crate::index::lock(&state.owner) = owner.to_string();
    if state.dir.as_deref() == Some(dir) {
        return;
    }
    // Dropping the previous watcher first releases the directory handle
    // `ReadDirectoryChangesW` holds on Windows.
    state.watcher = None;
    state.dir = None;
    let (tx, owner) = (state.tx.clone(), state.owner.clone());
    let handler = move |event: notify::Result<notify::Event>| {
        if event.is_ok_and(|event| triggers(&event.paths)) {
            let _ = tx.send(crate::index::lock(&owner).clone());
        }
    };
    match notify::recommended_watcher(handler).and_then(|mut watcher| {
        watcher.watch(Path::new(dir), RecursiveMode::NonRecursive)?;
        Ok(watcher)
    }) {
        Ok(watcher) => {
            state.watcher = Some(watcher);
            state.dir = Some(dir.to_string());
        }
        Err(e) => log::warn!("failed to watch {dir}: {e}"),
    }
}

/// Drop the watcher when the watched folder is the canonical `dir` or under
/// it, releasing the directory handle that on Windows blocks renaming `dir`
/// when the watched folder is under it; a watch on any other folder is left
/// alone. Returns the watched dir and its owner string when one was dropped,
/// so the caller can restore it with `set` if whatever it released the watch
/// for fails. The next `set` (with the same or a different dir) watches again.
pub fn release_under(watch: &Watch, dir: &str) -> Option<(String, String)> {
    let mut state = crate::index::lock(&watch.0);
    release(&mut state, dir)
}

fn release(state: &mut State, dir: &str) -> Option<(String, String)> {
    let watched = state.dir.as_deref()?;
    if !Path::new(watched).starts_with(dir) {
        return None;
    }
    let watched = watched.to_string();
    let owner = crate::index::lock(&state.owner).clone();
    if let Some(mut watcher) = state.watcher.take() {
        let _ = watcher.unwatch(Path::new(&watched));
        settle(&mut watcher);
    }
    state.dir = None;
    Some((watched, owner))
}

/// Return only once `watcher` has carried out every `unwatch` sent to it
/// before. On Windows `unwatch` (and dropping the watcher) only queues the
/// request to notify's server thread, which closes the directory handle later,
/// so an ancestor rename issued at once can still fail with
/// `PermissionDenied`. notify 8.2's server handles its queued actions in
/// order and `configure` blocks on the server's reply, so it returns after
/// the handles of the earlier `unwatch`es are closed. Elsewhere the release is
/// already synchronous and this is a no-op round trip. The result is ignored:
/// it is an error only when the server is gone, and so is every handle.
pub fn settle(watcher: &mut RecommendedWatcher) {
    let _ = watcher.configure(notify::Config::default());
}

/// Whether an event over `paths` is worth a rescan: no, only when every path
/// is a sidecar of either format or a sidecar write temporary, which is what
/// the app's own writes produce. An event without paths says nothing, so it
/// triggers.
pub fn triggers(paths: &[PathBuf]) -> bool {
    paths.is_empty()
        || !paths.iter().all(|path| {
            let Some(name) = path.file_name().map(|n| n.to_string_lossy()) else {
                return false;
            };
            name.ends_with(TEMP_SUFFIX)
                || SidecarFormat::Xmp.matches(&name)
                || SidecarFormat::Dop.matches(&name)
        })
}

/// Collapse bursts on `rx` into one `emit` per folder, `DEBOUNCE` after that
/// folder's last event. Same shape as `sidecar::run`: wait on the nearest
/// deadline, and treat a timeout as "that one is due".
fn run<F>(rx: Receiver<String>, emit: F)
where
    F: Fn(&str),
{
    let mut pending: Option<(String, Instant)> = None;
    loop {
        let message = match &pending {
            Some((_, deadline)) => {
                rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            }
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match message {
            Ok(dir) => {
                // A folder change resets the window on the new folder; the old
                // one is no longer open, so its pending event is dropped.
                pending = Some((dir, Instant::now() + DEBOUNCE));
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Some((dir, _)) = pending.take() {
                    emit(&dir);
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names
            .iter()
            .map(|n| PathBuf::from("/photos").join(n))
            .collect()
    }

    #[test]
    fn sidecar_and_temp_paths_do_not_trigger() {
        assert!(!triggers(&paths(&["a.xmp"])));
        assert!(!triggers(&paths(&["A.XMP"])));
        assert!(!triggers(&paths(&["a.arw.dop"])));
        assert!(!triggers(&paths(&["A.ARW.dop"])));
        assert!(!triggers(&paths(&["a.xmp.riffle-tmp"])));
        assert!(!triggers(&paths(&["a.xmp", "b.arw.dop"])));
    }

    /// An import writes `<name>.riffle-tmp` and renames it into place: the
    /// writes do not trigger, the rename's event (which names both paths on
    /// some platforms) does.
    #[test]
    fn an_import_temp_file_triggers_only_once_renamed() {
        assert!(!triggers(&paths(&["DSC00001.ARW.riffle-tmp"])));
        assert!(!triggers(&paths(&["DSC00001.JPG.riffle-tmp"])));
        assert!(!triggers(&paths(&["DSC00001.ARW.dop.riffle-tmp"])));
        assert!(triggers(&paths(&["DSC00001.ARW"])));
        assert!(triggers(&paths(&[
            "DSC00001.ARW.riffle-tmp",
            "DSC00001.ARW"
        ])));
    }

    #[test]
    fn raw_paths_and_empty_events_trigger() {
        assert!(triggers(&paths(&["a.arw"])));
        assert!(triggers(&paths(&["a.dng"])));
        assert!(triggers(&paths(&["a.xmp", "b.arw"])));
        assert!(triggers(&[]));
    }

    /// A state watching `dir` for real, as `set` leaves it.
    fn watching(dir: &Path) -> State {
        let mut watcher = notify::recommended_watcher(|_| {}).unwrap();
        watcher.watch(dir, RecursiveMode::NonRecursive).unwrap();
        State {
            dir: Some(dir.to_string_lossy().into_owned()),
            watcher: Some(watcher),
            tx: std::sync::mpsc::channel().0,
            owner: Arc::new(Mutex::new(String::new())),
        }
    }

    #[test]
    fn release_drops_a_watch_on_the_folder_or_under_it_and_keeps_a_sibling() {
        let root =
            std::env::temp_dir().join(format!("riffle-watch-release-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("photos");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(root.join("photos2")).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let dir = root.join("photos");
        let s = |p: &Path| p.to_string_lossy().into_owned();

        let mut state = watching(&dir);
        assert!(release(&mut state, &s(&root.join("photos2"))).is_none());
        assert!(release(&mut state, &s(&root.join("pho"))).is_none());
        assert!(state.watcher.is_some());
        assert_eq!(state.dir, Some(s(&dir)));

        assert_eq!(
            release(&mut state, &s(&dir)),
            Some((s(&dir), String::new()))
        );
        assert!(state.watcher.is_none());
        assert_eq!(state.dir, None);
        assert!(release(&mut state, &s(&dir)).is_none());

        let mut state = watching(&dir);
        assert_eq!(
            release(&mut state, &s(&root)),
            Some((s(&dir), String::new()))
        );
        assert!(state.watcher.is_none());
        assert_eq!(state.dir, None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn release_returns_the_watched_dir_and_owner() {
        let root =
            std::env::temp_dir().join(format!("riffle-watch-release-owner-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("photos");
        std::fs::create_dir_all(&dir).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let dir = root.join("photos");
        let s = |p: &Path| p.to_string_lossy().into_owned();

        let mut state = watching(&dir);
        *crate::index::lock(&state.owner) = "/OpenDir/photos".to_string();
        assert_eq!(
            release(&mut state, &s(&dir)),
            Some((s(&dir), "/OpenDir/photos".to_string()))
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_burst_is_collapsed_into_one_event() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (out, emitted) = std::sync::mpsc::channel();
        let thread =
            std::thread::spawn(move || run(rx, move |dir| out.send(dir.to_string()).unwrap()));

        for _ in 0..10 {
            tx.send("/photos".to_string()).unwrap();
        }
        assert_eq!(emitted.recv_timeout(DEBOUNCE * 10).unwrap(), "/photos");
        assert!(emitted.try_recv().is_err());

        tx.send("/photos".to_string()).unwrap();
        assert_eq!(emitted.recv_timeout(DEBOUNCE * 10).unwrap(), "/photos");

        drop(tx);
        thread.join().unwrap();
    }
}
