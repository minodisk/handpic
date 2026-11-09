//! Watching the folder tree's expanded folders so a subfolder created, deleted
//! or renamed under one shows up without collapsing and re-expanding it.
//!
//! The frontend sends the whole set of folders to watch (`set_tree_watches`),
//! keyed by its own spelling of each path (the tree's node key). One watcher
//! holds them all, one non-recursive watch per folder; the events are only a
//! trigger, debounced per folder into one `tree-changed` event that makes the
//! frontend re-list that node. `watch::triggers` drops the app's own sidecar
//! writes, so culling in an expanded folder does not re-list it.
//!
//! On Windows a watch pins its folder's ancestors: renaming a folder with a
//! watched descendant is refused, while a watch on the parent does not block
//! renaming a child. `rename_folder` therefore renames through
//! `with_released`, which releases the watches on the renamed folder and
//! under it, runs the rename while still holding the `TreeWatch` lock (so a
//! concurrent `set_tree_watches` cannot re-watch a path being renamed), and
//! restores them if the rename fails.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use riffle_backend::watch::settle;
use tauri::{Emitter, Manager};

/// How long after the last event under a folder its `tree-changed` is
/// emitted, as `watch::DEBOUNCE`.
const DEBOUNCE: Duration = Duration::from_millis(500);

/// The tree's watched folders and the watcher holding them. Managed state, one
/// per app.
pub struct TreeWatch(Mutex<State>);

struct State {
    /// `None` when the watcher could not be created; every watch is then
    /// skipped and the tree falls back to re-listing on expand.
    watcher: Option<RecommendedWatcher>,
    /// Each watched tree path and its canonical path.
    watched: HashMap<String, PathBuf>,
    /// The tree paths of each watched canonical path, shared with the
    /// watcher's event handler. Two tree paths can resolve to one folder (a
    /// symlink), which then holds one OS watch for both.
    keys: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
}

impl TreeWatch {
    /// Start the watcher and the debounce thread. The thread ends when the
    /// app drops the state and with it the sender.
    pub fn spawn(app: tauri::AppHandle) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            run(rx, move |dir| {
                let _ = app.emit(
                    "tree-changed",
                    Changed {
                        dir: dir.to_string(),
                    },
                );
            });
        });
        Self(Mutex::new(State::new(tx)))
    }
}

/// The payload of `tree-changed`: the tree path of the folder to re-list.
#[derive(serde::Serialize, Clone)]
struct Changed {
    dir: String,
}

impl State {
    fn new(tx: Sender<String>) -> Self {
        let keys: Arc<Mutex<HashMap<PathBuf, Vec<String>>>> = Arc::default();
        let shared = keys.clone();
        let handler = move |event: notify::Result<notify::Event>| {
            let Ok(event) = event else { return };
            if !crate::watch::triggers(&event.paths) {
                return;
            }
            for dir in tree_paths(&crate::index::lock(&shared), &event.paths) {
                let _ = tx.send(dir);
            }
        };
        let watcher = notify::recommended_watcher(handler)
            .inspect_err(|e| log::warn!("failed to start the folder tree watcher: {e}"))
            .ok();
        Self {
            watcher,
            watched: HashMap::new(),
            keys,
        }
    }

    /// Make the watched set equal to `dirs`.
    fn apply(&mut self, dirs: &[String]) {
        let stale: Vec<String> = self
            .watched
            .keys()
            .filter(|dir| !dirs.contains(dir))
            .cloned()
            .collect();
        for dir in stale {
            self.remove(&dir);
        }
        for dir in dirs {
            self.add(dir);
        }
    }

    /// Watch the tree path `dir`, unless it already is. A folder that cannot
    /// be watched is logged and skipped.
    fn add(&mut self, dir: &str) {
        if self.watched.contains_key(dir) {
            return;
        }
        let Some(watcher) = &mut self.watcher else {
            return;
        };
        let canonical = match std::fs::canonicalize(dir) {
            Ok(canonical) => canonical,
            Err(e) => {
                log::warn!("failed to watch {dir}: {e}");
                return;
            }
        };
        let shared = crate::index::lock(&self.keys).contains_key(&canonical);
        if !shared {
            if let Err(e) = watcher.watch(&canonical, RecursiveMode::NonRecursive) {
                log::warn!("failed to watch {dir}: {e}");
                return;
            }
        }
        crate::index::lock(&self.keys)
            .entry(canonical.clone())
            .or_default()
            .push(dir.to_string());
        self.watched.insert(dir.to_string(), canonical);
    }

    /// Stop watching the tree path `dir`, releasing the OS watch when no
    /// other tree path shares it. The shared map is updated before the
    /// `unwatch`, so the handler never waits on a lock held across it.
    fn remove(&mut self, dir: &str) {
        let Some(canonical) = self.watched.remove(dir) else {
            return;
        };
        let last = {
            let mut keys = crate::index::lock(&self.keys);
            let Some(dirs) = keys.get_mut(&canonical) else {
                return;
            };
            dirs.retain(|d| d != dir);
            let last = dirs.is_empty();
            if last {
                keys.remove(&canonical);
            }
            last
        };
        if let (true, Some(watcher)) = (last, &mut self.watcher) {
            if let Err(e) = watcher.unwatch(&canonical) {
                log::warn!("failed to unwatch {dir}: {e}");
            }
            settle(watcher);
        }
    }

    /// Stop watching every folder whose canonical path is `dir` or under it,
    /// returning their tree paths, sorted.
    fn release_under(&mut self, dir: &str) -> Vec<String> {
        let mut released: Vec<String> = self
            .watched
            .iter()
            .filter(|(_, canonical)| canonical.starts_with(dir))
            .map(|(tree, _)| tree.clone())
            .collect();
        released.sort();
        for tree in &released {
            self.remove(tree);
        }
        released
    }
}

/// The tree paths an event over `paths` belongs to: those of each path's
/// parent (an entry inside a watched folder), or of the path itself (the
/// watched folder proper).
fn tree_paths(keys: &HashMap<PathBuf, Vec<String>>, paths: &[PathBuf]) -> Vec<String> {
    let mut dirs: Vec<String> = Vec::new();
    for path in paths {
        let owner = path
            .parent()
            .and_then(|parent| keys.get(parent))
            .or_else(|| keys.get(path));
        for dir in owner.into_iter().flatten() {
            if !dirs.contains(dir) {
                dirs.push(dir.clone());
            }
        }
    }
    dirs
}

/// Make the tree's watched set equal to `dirs`, the frontend's paths of its
/// expanded, drawn folders. Folders no longer in `dirs` are released; a
/// folder that cannot be watched is logged and skipped, never an error.
#[tauri::command]
pub async fn set_tree_watches(app: tauri::AppHandle, dirs: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::index::lock(&app.state::<TreeWatch>().0).apply(&dirs);
    })
    .await
    .map_err(|e| e.to_string())
}

/// Release the tree watches on the canonical `dir` and under it, run `f`
/// while still holding the `TreeWatch` lock, and put the released watches
/// back if `f` fails. Holding the lock across `f` keeps `set_tree_watches`
/// from re-watching a path `f` (a rename) is about to touch: without it, a
/// tree re-render landing in that window could send a set that still
/// contains the released paths, which `apply` would watch again and make the
/// rename fail with `PermissionDenied` on Windows. This is safe from
/// deadlock: the notify handler only takes the separate `keys` lock, never
/// this one.
pub fn with_released<T>(
    app: &tauri::AppHandle,
    dir: &str,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let tree_watch = app.state::<TreeWatch>();
    let mut state = crate::index::lock(&tree_watch.0);
    let released = state.release_under(dir);
    let result = f();
    if result.is_err() {
        for dir in &released {
            state.add(dir);
        }
    }
    result
}

/// Collapse bursts on `rx` into one `emit` per tree path, `DEBOUNCE` after
/// that path's last event. Unlike `watch::run`, every path keeps its own
/// deadline, so two folders changing at once each get their event.
fn run<F>(rx: Receiver<String>, emit: F)
where
    F: Fn(&str),
{
    let mut pending: HashMap<String, Instant> = HashMap::new();
    loop {
        let message = match pending.values().min() {
            Some(deadline) => rx.recv_timeout(deadline.saturating_duration_since(Instant::now())),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match message {
            Ok(dir) => {
                pending.insert(dir, Instant::now() + DEBOUNCE);
            }
            Err(RecvTimeoutError::Timeout) => {
                let now = Instant::now();
                let due: Vec<String> = pending
                    .iter()
                    .filter(|(_, deadline)| **deadline <= now)
                    .map(|(dir, _)| dir.clone())
                    .collect();
                for dir in due {
                    pending.remove(&dir);
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
    use std::path::Path;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("riffle-treewatch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn s(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    fn keys(state: &State) -> Vec<String> {
        let mut keys: Vec<String> = state.watched.keys().cloned().collect();
        keys.sort();
        keys
    }

    #[test]
    fn applying_a_set_adds_and_removes_watches() {
        let root = temp_dir("apply");
        for dir in ["pa/a", "pb/b", "pc/c"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let (a, b, c) = (
            s(&root.join("pa").join("a")),
            s(&root.join("pb").join("b")),
            s(&root.join("pc").join("c")),
        );
        let missing = s(&root.join("missing"));
        let mut state = State::new(std::sync::mpsc::channel().0);

        state.apply(&[a.clone(), b.clone(), missing]);
        assert_eq!(keys(&state), [a.clone(), b.clone()]);

        state.apply(&[b.clone(), c.clone()]);
        assert_eq!(keys(&state), [b.clone(), c.clone()]);
        let gone = std::fs::canonicalize(&a).unwrap();
        assert!(!crate::index::lock(&state.keys).contains_key(&gone));
        // A folder with a watched descendant cannot be renamed on Windows, so
        // this fails unless the removed watch released its handle.
        std::fs::rename(root.join("pa"), root.join("pa-renamed")).unwrap();

        state.apply(&[]);
        assert!(state.watched.is_empty());
        assert!(crate::index::lock(&state.keys).is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn release_under_releases_the_folder_and_below_and_restore_puts_them_back() {
        let root = temp_dir("release");
        std::fs::create_dir_all(root.join("photos").join("sub")).unwrap();
        std::fs::create_dir_all(root.join("photos2")).unwrap();
        let photos = s(&root.join("photos"));
        let sub = s(&root.join("photos").join("sub"));
        let photos2 = s(&root.join("photos2"));
        let canonical = s(&std::fs::canonicalize(&photos).unwrap());
        let mut state = State::new(std::sync::mpsc::channel().0);
        state.apply(&[photos.clone(), sub.clone(), photos2.clone()]);

        let released = state.release_under(&canonical);
        assert_eq!(released, [photos.clone(), sub.clone()]);
        assert_eq!(keys(&state), std::slice::from_ref(&photos2));

        for dir in &released {
            state.add(dir);
        }
        let mut all = vec![photos.clone(), sub.clone(), photos2.clone()];
        all.sort();
        assert_eq!(keys(&state), all);

        state.release_under(&canonical);
        std::fs::rename(&photos, root.join("renamed")).unwrap();
        assert_eq!(keys(&state), [photos2]);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_event_is_sent_under_the_tree_path() {
        let root = temp_dir("event");
        let dir = s(&root);
        let (tx, rx) = std::sync::mpsc::channel();
        let mut state = State::new(tx);
        state.apply(std::slice::from_ref(&dir));

        std::fs::create_dir(root.join("new")).unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), dir);

        drop(state);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bursts_are_collapsed_per_path() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (out, emitted) = std::sync::mpsc::channel();
        let thread =
            std::thread::spawn(move || run(rx, move |dir| out.send(dir.to_string()).unwrap()));

        for _ in 0..10 {
            tx.send("/photos".to_string()).unwrap();
            tx.send("/other".to_string()).unwrap();
        }
        let mut got = vec![
            emitted.recv_timeout(DEBOUNCE * 10).unwrap(),
            emitted.recv_timeout(DEBOUNCE * 10).unwrap(),
        ];
        got.sort();
        assert_eq!(got, ["/other", "/photos"]);
        assert!(emitted.try_recv().is_err());

        tx.send("/photos".to_string()).unwrap();
        assert_eq!(emitted.recv_timeout(DEBOUNCE * 10).unwrap(), "/photos");
        assert!(emitted.try_recv().is_err());

        drop(tx);
        thread.join().unwrap();
    }
}
